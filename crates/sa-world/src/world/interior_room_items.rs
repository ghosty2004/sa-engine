//! WRITER-WORLD-2 (batch `world6-3`): bedroom / lounge furnishing helpers that place one object on
//! a tile or add a chair info against a wall, and the kitchen furnishing. Ported from the 1.0 US bytes; gta-reversed has them
//! unreversed.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use super::common::{ftol32, rd, wr, x, xi};

use crate::bindings::get_furniture;
use crate::bindings::set_tiles_status;
use crate::bindings::add_goto_pt;
use crate::bindings::add_interior_info;
use crate::bindings::get_random_id;
use crate::bindings::place_furniture;
orig! {
    /// `Interior_c::PlaceObject(bool, Furniture_c*, float x, float y, float z, float rot)` (0x5934E0)
    fn place_object(this: GPtr<u8>, a: u32, f: GPtr<u8>, px: f32, py: f32, pz: f32, rot: f32) -> GPtr<u8> = thiscall 0x5934E0 => super::interior_place_furniture::place_object;
    /// `Interior_c::Kitchen_FurnishEdges()` (0x596930)
    fn kitchen_furnish_edges(this: GPtr<u8>) = thiscall 0x596930 => unported;
    /// `Interior_c::Office_PlaceEdgeDesks(int32 id, int32 x, int32 y, int32 dir, int32)` (0x5995B0)
    fn office_place_edge_desks(this: GPtr<u8>, id: i32, tx: i32, ty: i32, dir: i32, a4: i32) -> i32 = thiscall 0x5995B0 => unported;
    /// `Interior_c::Office_PlaceEdgeFillers(int32 id, int32 x, int32 y, int32 dir, int32)` (0x599210)
    fn office_place_edge_fillers(this: GPtr<u8>, id: i32, tx: i32, ty: i32, dir: i32, a4: i32) -> i32 = thiscall 0x599210 => super::interior_wall_fillers::office_place_edge_fillers;
}

/// `0.5` (0x858B8C), `90` (0x85991C).
const F_0_5: f32 = 0.5;
const F_90: f32 = 90.0;

fn wealth(this: GPtr<u8>) -> u32 {
    u32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 0x1F))
}

/// The tile corner of (tx, ty) moved half a tile along the wall of `dir` (x for 0 / 2, y for 1 / 3).
fn wall_point(dir: i32, tx: i32, ty: i32) -> (f32, f32) {
    let (mut fx, mut fy) = (xi(tx).to_f32(), xi(ty).to_f32());
    match dir {
        0 | 2 => fx = x(fx).fadd_m(F_0_5).to_f32(),
        1 | 3 => fy = x(fy).fadd_m(F_0_5).to_f32(),
        _ => {}
    }
    (fx, fy)
}

/// `Interior_c::Bedroom_AddTableItem(int32 group, int32 subGroup, int32 dir, int32 x, int32 y,
/// int32 rot)` (0x593F10): a random furniture of (group, subGroup) placed at the tile point (half a
/// tile along the wall, then the tile centre: + 0.5 on both axes), height 0.5, rotation `rot * 90`.
#[hook(addr = 0x593F10, conv = thiscall, name = "Interior_c::Bedroom_AddTableItem", this = "Interior_c",
       expect = "51 DB 44 24 14",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
#[allow(clippy::too_many_arguments)]
pub fn bedroom_add_table_item(this: GPtr<u8>, group: i32, sub: i32, dir: i32, tx: i32, ty: i32, rot: i32) -> GPtr<u8> {
    let (fx, fy) = wall_point(dir, tx, ty);
    let f = get_furniture(GPtr::from_addr(0xBA_B380), group, sub, -1, wealth(this));
    let r = xi(rot).fmul_m(F_90).to_f32();
    place_object(this, 1, f, x(fx).fadd_m(F_0_5).to_f32(), x(fy).fadd_m(F_0_5).to_f32(), F_0_5, r)
}

/// `Interior_c::Lounge_AddHifi(int32 dir, int32 x, int32 y, int32)` (0x597430): a hifi (furniture
/// 2/8) at the tile point like `Bedroom_AddTableItem`, rotated `(dir & 3) * 90`.
#[hook(addr = 0x597430, conv = thiscall, name = "Interior_c::Lounge_AddHifi", this = "Interior_c",
       expect = "51 DB 44 24 0C",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn lounge_add_hifi(this: GPtr<u8>, dir: i32, tx: i32, ty: i32, _a3: i32) -> GPtr<u8> {
    let (fx, fy) = wall_point(dir, tx, ty);
    let f = get_furniture(GPtr::from_addr(0xBA_B380), 2, 8, -1, wealth(this));
    let r = xi(dir & 3).fmul_m(F_90).to_f32();
    place_object(this, 1, f, x(fx).fadd_m(F_0_5).to_f32(), x(fy).fadd_m(F_0_5).to_f32(), F_0_5, r)
}

/// `Interior_c::Lounge_AddChairInfo(int32 dir, int32 pos, int32 entity)` (0x5974E0): a chair info
/// (type 1, direction `(dir - 2) & 3`, the raw third argument as the entity) one tile in front of
/// the wall of `dir`: dir 0 at (pos + 0.5, D - 2), 2 at (pos + 0.5, 1), 1 at (1, pos + 0.5), 3 at
/// (W - 2, pos + 0.5); any other dir adds nothing.
#[hook(addr = 0x5974E0, conv = thiscall, name = "Interior_c::Lounge_AddChairInfo", this = "Interior_c",
       expect = "8B 54 24 04 8D 42 FE",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn lounge_add_chair_info(this: GPtr<u8>, dir: i32, pos: i32, entity: i32) {
    let d = (dir - 2) & 3;
    let along = || xi(pos).fadd_m(F_0_5).to_f32();
    let info = || rd::<GPtr<u8>>(this, 0x14);
    match dir {
        0 => {
            let y = xi(i32::from(rd::<u8>(info(), 3)) - 1).fsub_m(1.0).to_f32();
            add_interior_info(this, 1, along(), y, d as u32, entity as u32);
        }
        2 => { add_interior_info(this, 1, along(), 1.0, d as u32, entity as u32); },
        1 => { add_interior_info(this, 1, 1.0, along(), d as u32, entity as u32); },
        3 => {
            let a = along();
            let px = xi(i32::from(rd::<u8>(info(), 2)) - 1).fsub_m(1.0).to_f32();
            add_interior_info(this, 1, px, a, d as u32, entity as u32);
        }
        _ => {}
    }
}

/// `Interior_c::FurnishKitchen()` (0x5970B0): the door tiles (x = info +5 - 1, 2 x 1, status 7), a
/// status-3 ring one tile inside the walls (rows y = 0 and D - 2 for x = 1..W - 2, columns x = 1 and
/// W - 2 for y = 0..D - 2), goto points at the four inner corners, the kitchen id (+0x791 = random id
/// of group 4 / sub 0), the edge units, and the table (furniture 8/1) centred
/// (`ftol(W * 0.5 - w * 0.5)`, likewise y).
#[hook(addr = 0x5970B0, conv = thiscall, name = "Interior_c::FurnishKitchen", this = "Interior_c",
       expect = "83 EC 0C 53 55 56 57",
       unverified = "world6-3", route = "L2", hazards = "uninit-locals,heap-writes",
       note = "the table's size outputs are uninitialised stack words (unused)")]
pub fn furnish_kitchen(this: GPtr<u8>) {
    let info = || rd::<GPtr<u8>>(this, 0x14);
    set_tiles_status(this, i32::from(rd::<i8>(info(), 5)) - 1, 0, 2, 1, 7, 0);
    let w2 = i32::from(rd::<u8>(info(), 2)) - 2;
    let d2 = i32::from(rd::<u8>(info(), 3)) - 2;
    for xx in 1..=w2 {
        set_tiles_status(this, xx, d2, 1, 1, 3, 0);
        set_tiles_status(this, xx, 0, 1, 1, 3, 0);
    }
    for yy in 0..=d2 {
        set_tiles_status(this, 1, yy, 1, 1, 3, 0);
        set_tiles_status(this, w2, yy, 1, 1, 3, 0);
    }
    add_goto_pt(this, 1, 1, 0.0, 0.0);
    add_goto_pt(this, 1, d2, 0.0, 0.0);
    add_goto_pt(this, w2, 1, 0.0, 0.0);
    add_goto_pt(this, w2, d2, 0.0, 0.0);
    let id = get_random_id(GPtr::from_addr(0xBA_B380), 4, 0, wealth(this));
    wr::<u8>(this, 0x791, id as u8);
    kitchen_furnish_edges(this);
    let f = get_furniture(GPtr::from_addr(0xBA_B380), 8, 1, -1, wealth(this));
    let i = info();
    let ty = ftol32(xi(i32::from(rd::<u8>(i, 3))).fmul_m(F_0_5).fsub(xi(i32::from(rd::<u8>(f, 0xD))).fmul_m(F_0_5)));
    let tx = ftol32(xi(i32::from(rd::<u8>(i, 2))).fmul_m(F_0_5).fsub(xi(i32::from(rd::<u8>(f, 0xC))).fmul_m(F_0_5)));
    let (mut ow, mut od) = (0i32, 0i32);
    place_furniture(this, f, tx, ty, 0, 0, 0, GPtr::from_raw(&mut ow as *mut i32).cast(), GPtr::from_raw(&mut od as *mut i32).cast(), 0);
}

/// `Interior_c::PlaceFurnitureInCorner(int32 group, int32 subGroup, int32 id, float height, int32
/// mode, int32 corner, int32 offset, int32* outCorner, int32* outX, int32* outY, int32* outW, int32*
/// outD)` (0x593340): the furniture (`GetFurniture(group, subGroup, id, wealth)`; none: null) is tried
/// up to 20 times (once for a given corner; a -1 corner is drawn each try from `rand() * 4 / 32768`)
/// in the corner: 0 at (0, D - f.d - offset), 1 at (offset, 0), 2 at (W - f.c, offset), 3 at
/// (W - f.d - offset, D - f.c); any other corner keeps the previous try's tile (first try: (corner,
/// corner)). `PlaceFurniture(f, x, y, height, mode, corner)`; success fills the non-null outputs
/// and returns the object, else null.
#[hook(addr = 0x593340, conv = thiscall, name = "Interior_c::PlaceFurnitureInCorner", this = "Interior_c",
       expect = "83 EC 08 8B 54 24 14",
       unverified = "world6-3", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x5933C0")]
#[allow(clippy::too_many_arguments)]
pub fn place_furniture_in_corner(this: GPtr<u8>, group: i32, sub: i32, id: i32, height: u32, mode: i32, corner: i32,
                                 offset: i32, out_corner: GPtr<i32>, out_x: GPtr<i32>, out_y: GPtr<i32>,
                                 out_w: GPtr<i32>, out_d: GPtr<i32>) -> GPtr<u8> {
    let f = get_furniture(GPtr::from_addr(0xBA_B380), group, sub, id, wealth(this));
    let tries = if corner != -1 { 1 } else { 20 };
    if f.is_null() {
        return GPtr::NULL;
    }
    let info = || rd::<GPtr<u8>>(this, 0x14);
    let (mut tx, mut ty) = (corner, corner);
    let mut c = corner;
    for _ in 0..tries {
        if corner == -1 {
            let r = at_site(0x59_33C0, rand) & 0xFFFF;
            c = ftol32(xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(4.0));
        }
        let fc = i32::from(rd::<u8>(f, 0xC));
        let fd = i32::from(rd::<u8>(f, 0xD));
        match c {
            1 => {
                tx = offset;
                ty = 0;
            }
            3 => {
                tx = i32::from(rd::<u8>(info(), 2)) - fd - offset;
                ty = i32::from(rd::<u8>(info(), 3)) - fc;
            }
            2 => {
                tx = i32::from(rd::<u8>(info(), 2)) - fc;
                ty = offset;
            }
            0 => {
                ty = i32::from(rd::<u8>(info(), 3)) - fd - offset;
                tx = 0;
            }
            _ => {}
        }
        let (mut ow, mut od) = (0i32, 0i32);
        let o = place_furniture(this, f, tx, ty, height, mode, c, GPtr::from_raw(&mut ow as *mut i32).cast(),
                                GPtr::from_raw(&mut od as *mut i32).cast(), 0);
        if !o.is_null() {
            for (p, v) in [(out_corner, c), (out_x, tx), (out_y, ty), (out_w, ow), (out_d, od)] {
                if !p.is_null() {
                    p.write(v);
                }
            }
            return o;
        }
    }
    GPtr::NULL
}

/// `Interior_c::Office_FurnishEdges()` (0x599770): a status-3 walkway ring two tiles inside the walls
/// (rows y = 2 and D - 3, columns x = 2 and W - 3), goto points at its corners (offset half a tile
/// inwards), the door tiles (x = info +5 - 2, 4 x 2, status 7), then edge desks along the four walls
/// (y = 0 facing 2, y = D - 1 facing 0 for x = 1.. < W - 1; x = 0 facing 1, x = W - 1 facing 3 for
/// y = 1..= D - 2, each step the returned width), then edge fillers along the same walls.
#[hook(addr = 0x599770, conv = thiscall, name = "Interior_c::Office_FurnishEdges", this = "Interior_c",
       expect = "53 55 56 8B F1",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn office_furnish_edges(this: GPtr<u8>) {
    let info = || rd::<GPtr<u8>>(this, 0x14);
    let w3 = i32::from(rd::<u8>(info(), 2)) - 3;
    let d3 = i32::from(rd::<u8>(info(), 3)) - 3;
    for xx in 2..=w3 {
        set_tiles_status(this, xx, d3, 1, 1, 3, 0);
        set_tiles_status(this, xx, 2, 1, 1, 3, 0);
    }
    for yy in 2..=d3 {
        set_tiles_status(this, 2, yy, 1, 1, 3, 0);
        set_tiles_status(this, w3, yy, 1, 1, 3, 0);
    }
    add_goto_pt(this, 2, 2, 0.5, 0.5);
    add_goto_pt(this, 2, d3, 0.5, -0.5);
    add_goto_pt(this, w3, 2, -0.5, 0.5);
    add_goto_pt(this, w3, d3, -0.5, -0.5);
    let w1 = i32::from(rd::<u8>(info(), 2)) - 1;
    let d1 = i32::from(rd::<u8>(info(), 3)) - 1;
    set_tiles_status(this, i32::from(rd::<i8>(info(), 5)) - 2, 0, 4, 2, 7, 0);
    type Placer = fn(GPtr<u8>, i32, i32, i32, i32, i32) -> i32;
    for place in [office_place_edge_desks as Placer, office_place_edge_fillers as Placer] {
        let mut xx = 1;
        while xx < w1 {
            xx += place(this, -1, xx, 0, 2, 2);
        }
        let mut xx = 1;
        while xx < w1 {
            xx += place(this, -1, xx, d1, 0, 0);
        }
        let mut yy = 1;
        while yy < d1 {
            yy += place(this, -1, 0, yy, 1, 1);
        }
        let mut yy = 1;
        while yy < d1 {
            yy += place(this, -1, w1, yy, 3, 3);
        }
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_BEDROOM_ADD_TABLE_ITEM, HOOK_LOUNGE_ADD_HIFI, HOOK_LOUNGE_ADD_CHAIR_INFO, HOOK_FURNISH_KITCHEN, HOOK_PLACE_FURNITURE_IN_CORNER, HOOK_OFFICE_FURNISH_EDGES];
