//! WRITER-WORLD-2 (batch `world6-3`): interior furnishing dispatch and teardown, and
//! `InteriorManager_c`'s lookups of the interior containing a point, a ped or a furniture entity.
//! Ported from the 1.0 US bytes; gta-reversed has none of them reversed.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use super::common::{ftol32, g, rd, wr, x, xi};

use crate::bindings::furnish_bedroom;
use crate::bindings::furnish_kitchen;
use crate::bindings::furnish_lounge;
use crate::bindings::office_place_desk;
use crate::bindings::ped_get_held_entity as get_entity_held;
use crate::bindings::set_tiles_status;
use sa_core::bindings::find_player_ped;
use sa_core::bindings::list_add_item;
use sa_core::bindings::list_remove_item;
use sa_core::bindings::world_remove;
orig! {
    /// `Interior_c::FurnishShop(int32)` (0x59A790)
    fn furnish_shop(this: GPtr<u8>, a: i32) = thiscall 0x59A790 => unported;
    /// `Interior_c::FurnishOffice()` (0x599AF0)
    fn furnish_office(this: GPtr<u8>) = thiscall 0x599AF0 => furnish_office_port;
    /// `CEntity::~CEntity` deleting destructor (vtable +0)
    fn v_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CPickups::RemovePickUpsInArea(float, float, float, float, float, float)` (0x456D30)
    fn remove_pickups_in_area(x0: f32, x1: f32, y0: f32, y1: f32, z0: f32, z1: f32) = cdecl 0x456D30 => unported;
    /// `Interior_c::Unfurnish()` (0x5915D0)
    fn unfurnish_call(this: GPtr<u8>) = thiscall 0x5915D0 => unfurnish;
    /// `Interior_c::GetFurnitureEntity(CEntity*)` (0x5913B0)
    fn get_furniture_entity(it: GPtr<u8>, e: GPtr<u8>) -> GPtr<u8> = thiscall 0x5913B0 => super::misc2::interior_get_furniture_entity;
    /// `Interior_c::GetBoundingBox(FurnitureEntity_c*, CVector*)` (0x593DB0)
    fn interior_get_bounding_box(it: GPtr<u8>, fe: GPtr<u8>, out: GPtr<u8>) -> u8 = thiscall 0x593DB0 => unported;
    /// `Interior_c::IsPtInside(const CVector&, float, float, float)` (0x5913E0)
    fn is_pt_inside(it: GPtr<u8>, pt: GPtr<f32>, mx: f32, my: f32, mz: f32) -> u8 = thiscall 0x5913E0 => super::misc2::interior_is_pt_inside;
    /// `InteriorManager_c::GetPedsInteriorGroup(CPed*)` (0x598240)
    fn get_peds_interior_group(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x598240 => super::misc2::interior_manager_get_peds_interior_group;
    /// `FurnitureManager_c::GetRandomId(int32 group, int32 subGroup, uint8 wealth)` (0x591220) on `g_furnitureMan` (0xBAB380)
    fn get_random_id(man: GPtr<u8>, group: i32, sub: u32, wealth: u32) -> i32 = thiscall 0x591220 => super::misc2::furniture_manager_get_random_id;
    /// `Interior_c::Office_FurnishEdges()` (0x599770)
    fn office_furnish_edges(this: GPtr<u8>) = thiscall 0x599770 => super::interior_room_items::office_furnish_edges;
    /// `Interior_c::Office_FurnishCenter()` (0x599A30)
    fn office_furnish_center(this: GPtr<u8>) = thiscall 0x599A30 => office_furnish_center_port;
    /// `Interior_c::Office_PlaceDeskQuad(int32, int32 x, int32 y, int32 id)` (0x599960)
    fn office_place_desk_quad_call(this: GPtr<u8>, a0: i32, tx: i32, ty: i32, id: i32) -> i32 = thiscall 0x599960 => office_place_desk_quad;
    /// `Interior_c::Shop_FurnishCeiling()` (0x59A130)
    fn shop_furnish_ceiling(this: GPtr<u8>) = thiscall 0x59A130 => unported;
    /// `Interior_c::CheckTilesEmpty(int32 x, int32 y, int32 w, int32 d, bool)` (0x591680)
    fn check_tiles_empty(this: GPtr<u8>, tx: i32, ty: i32, w: i32, d: i32, b: u8) -> u8 = thiscall 0x591680 => super::misc2::interior_check_tiles_empty;
}

/// `ftol((rand() & 0xFFFF) * (1/32768) * n)` (`fimul`).
fn rand_times(site: u32, n: i32) -> i32 {
    let r = at_site(site, rand) & 0xFFFF;
    ftol32(xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul(xi(n)))
}

/// `Interior_c::Furnish()` (0x591590): by the interior info's type (+0): 0 `FurnishShop(0)`, 1 office,
/// 2 lounge, 3 bedroom, 4 kitchen (tail calls), anything else nothing.
#[hook(addr = 0x591590, conv = thiscall, name = "Interior_c::Furnish", this = "Interior_c",
       expect = "8B 41 14 0F B6 00",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn furnish(this: GPtr<u8>) {
    match rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 0) {
        0 => furnish_shop(this, 0),
        1 => furnish_office(this),
        2 => furnish_lounge(this),
        3 => furnish_bedroom(this),
        4 => furnish_kitchen(this),
        _ => {}
    }
}

/// `Interior_c::Unfurnish()` (0x5915D0): every furniture item of the interior (+0x5C list): the
/// object the player holds (a `CObject` with +0x140 bit 13) is kept and handed over (counter
/// 0xBB4A70 + 1, created-by 3 at +0x13C, removal time now + 99,999,999 at +0x150); any other entity
/// is removed from the world and deleted. The item is cleared, unlinked and returned to the free
/// list (0xBAD3EC).
#[hook(addr = 0x5915D0, conv = thiscall, name = "Interior_c::Unfurnish", this = "Interior_c",
       expect = "55 8D 69 5C 56",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn unfurnish(this: GPtr<u8>) {
    let list = this.byte_add(0x5C);
    let mut item = rd::<GPtr<u8>>(list, 0);
    while !item.is_null() {
        let next = rd::<GPtr<u8>>(item, 4);
        let ped = find_player_ped(-1);
        let held = if ped.is_null() { GPtr::NULL } else { get_entity_held(ped) };
        if !held.is_null()
            && held == rd::<GPtr<u8>>(item, 8)
            && rd::<u8>(held, 0x36) & 7 == 4
            && rd::<u32>(held, 0x140) & 0x2000 != 0
        {
            let c = g::<u16>(0xBB_4A70);
            c.write(c.read().wrapping_add(1));
            wr::<u8>(held, 0x13C, 3);
            wr::<u32>(held, 0x150, g::<u32>(0xB7_CB84).read().wrapping_add(0x5F5_E0FF));
        } else {
            world_remove(rd::<GPtr<u8>>(item, 8));
            let e = rd::<GPtr<u8>>(item, 8);
            if !e.is_null() {
                v_delete(e, 1);
            }
        }
        wr::<u32>(item, 8, 0);
        list_remove_item(list, item);
        list_add_item(GPtr::from_addr(0xBA_D3EC), item);
        item = next;
    }
}

/// `Interior_c::Exit()` (0x592230): removes the pickups within 50 of the interior's position
/// (+0x48) on every axis, then tail-calls `Unfurnish`.
#[hook(addr = 0x592230, conv = thiscall, name = "Interior_c::Exit", this = "Interior_c",
       expect = "83 EC 0C 56 8B F1",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn interior_exit(this: GPtr<u8>) {
    let (px, py, pz) = (rd::<f32>(this, 0x48), rd::<f32>(this, 0x4C), rd::<f32>(this, 0x50));
    let k = 50.0;
    remove_pickups_in_area(
        x(px).fsub_m(k).to_f32(),
        x(px).fadd_m(k).to_f32(),
        x(py).fsub_m(k).to_f32(),
        x(py).fadd_m(k).to_f32(),
        x(pz).fsub_m(k).to_f32(),
        x(pz).fadd_m(k).to_f32(),
    );
    unfurnish_call(this);
}

/// The interiors of every group of the manager (+0x428C list, next at +4; 8 slots at +0x10), in
/// order, with `f` deciding when to stop.
fn each_interior<T>(this: GPtr<u8>, mut f: impl FnMut(GPtr<u8>) -> Option<T>) -> Option<T> {
    let mut grp = rd::<GPtr<u8>>(this, 0x428C);
    while !grp.is_null() {
        for k in 0..8 {
            let it = rd::<GPtr<u8>>(grp, 0x10 + 4 * k);
            if let Some(v) = (!it.is_null()).then(|| f(it)).flatten() {
                return Some(v);
            }
        }
        grp = rd::<GPtr<u8>>(grp, 4);
    }
    None
}

/// `InteriorManager_c::GetBoundingBox(CEntity* furniture, CVector* out)` (0x598090): the first
/// interior that owns `furniture` (`GetFurnitureEntity`) answers with its `GetBoundingBox`; none:
/// false (`xor al, al`).
#[hook(addr = 0x598090, conv = thiscall, name = "InteriorManager_c::GetBoundingBox", this = "InteriorManager_c",
       expect = "53 8B 99 8C 42 00 00",
       unverified = "world6-3", route = "L2", hazards = "none",
       note = "AL result; the not-found path leaves EAX's upper bytes from the last lookup (or the caller's EAX)")]
pub fn manager_get_bounding_box(this: GPtr<u8>, e: GPtr<u8>, out: GPtr<u8>) -> u8 {
    each_interior(this, |it| {
        let fe = get_furniture_entity(it, e);
        if fe.is_null() { None } else { Some(interior_get_bounding_box(it, fe, out)) }
    })
    .unwrap_or(0)
}

/// `InteriorManager_c::GetVectorsInterior(const CVector* pt)` (0x5983D0): the first interior of any
/// group containing `pt` (`IsPtInside` with no margins), else null.
#[hook(addr = 0x5983D0, conv = thiscall, name = "InteriorManager_c::GetVectorsInterior", this = "InteriorManager_c",
       expect = "53 8B 99 8C 42 00 00",
       unverified = "world6-3", route = "L2", hazards = "none")]
pub fn get_vectors_interior(this: GPtr<u8>, pt: GPtr<f32>) -> GPtr<u8> {
    each_interior(this, |it| if is_pt_inside(it, pt, 0.0, 0.0, 0.0) != 0 { Some(it) } else { None })
        .unwrap_or(GPtr::NULL)
}

/// `InteriorManager_c::GetPedsInterior(CPed* ped)` (0x598620): the interior of the ped's group
/// (`GetPedsInteriorGroup`) containing the ped's position, else null.
#[hook(addr = 0x598620, conv = thiscall, name = "InteriorManager_c::GetPedsInterior", this = "InteriorManager_c",
       expect = "53 8B 5C 24 08",
       unverified = "world6-3", route = "L2", hazards = "none")]
pub fn get_peds_interior(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let grp = get_peds_interior_group(this, ped);
    if grp.is_null() {
        return GPtr::NULL;
    }
    for k in 0..8 {
        let it = rd::<GPtr<u8>>(grp, 0x10 + 4 * k);
        if it.is_null() {
            continue;
        }
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
        if is_pt_inside(it, pos.cast(), 0.0, 0.0, 0.0) != 0 {
            return it;
        }
    }
    GPtr::NULL
}

/// `Interior_c::FurnishOffice()` (0x599AF0): marks the four 2 x 2 corners status 2, picks the office
/// furniture ids (+0x791 = random id of group 1 / sub 0, +0x792 of group 1 / sub 1, by wealth),
/// furnishes the edges and the centre and tail-calls `Shop_FurnishCeiling`.
#[hook(addr = 0x599AF0, conv = thiscall, name = "Interior_c::FurnishOffice", this = "Interior_c",
       expect = "56 6A 00 6A 02",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn furnish_office_port(this: GPtr<u8>) {
    let info = || rd::<GPtr<u8>>(this, 0x14);
    set_tiles_status(this, 0, 0, 2, 2, 2, 0);
    set_tiles_status(this, 0, i32::from(rd::<u8>(info(), 3)) - 2, 2, 2, 2, 0);
    set_tiles_status(this, i32::from(rd::<u8>(info(), 2)) - 2, 0, 2, 2, 2, 0);
    let (w, d) = (i32::from(rd::<u8>(info(), 2)), i32::from(rd::<u8>(info(), 3)));
    set_tiles_status(this, w - 2, d - 2, 2, 2, 2, 0);
    let a = get_random_id(GPtr::from_addr(0xBA_B380), 1, 0, u32::from(rd::<u8>(info(), 0x1F)));
    wr::<u8>(this, 0x791, a as u8);
    let b = get_random_id(GPtr::from_addr(0xBA_B380), 1, 1, u32::from(rd::<u8>(info(), 0x1F)));
    wr::<u8>(this, 0x792, b as u8);
    office_furnish_edges(this);
    office_furnish_center(this);
    shop_furnish_ceiling(this);
}

/// `Interior_c::FindEmptyTiles(int32 w, int32 d, int32* outX, int32* outY)` (0x591C50): up to 100
/// random positions (`x = rand() * (W - w) / 32768`, then y likewise with D - d) until
/// `CheckTilesEmpty(x, y, w, d, true)`; found: the outputs and true, else false.
#[hook(addr = 0x591C50, conv = thiscall, name = "Interior_c::FindEmptyTiles", this = "Interior_c",
       expect = "83 EC 08 53 55",
       unverified = "world6-3", route = "L2", hazards = "none",
       note = "rand at_site 0x591C79, 0x591CA9 per try; false leaves EAX's upper bytes from CheckTilesEmpty")]
pub fn find_empty_tiles(this: GPtr<u8>, w: i32, d: i32, out_x: GPtr<i32>, out_y: GPtr<i32>) -> u8 {
    for _ in 0..100 {
        let info = rd::<GPtr<u8>>(this, 0x14);
        let tx = rand_times(0x59_1C79, i32::from(rd::<u8>(info, 2)) - w);
        let info = rd::<GPtr<u8>>(this, 0x14);
        let ty = rand_times(0x59_1CA9, i32::from(rd::<u8>(info, 3)) - d);
        if check_tiles_empty(this, tx, ty, w, d, 1) != 0 {
            out_x.write(tx);
            out_y.write(ty);
            return 1;
        }
    }
    0
}

/// `Interior_c::Office_PlaceDeskQuad(int32 unused, int32 x, int32 y, int32 id)` (0x599960): four
/// desks around (x, y) (at (x, y - 2) and (x - 2, y - 2) facing 2, at (x, y) and (x - 2, y) facing 0;
/// chance 70, furniture `id`), the walkway around them (rows y - 3 and y + 2 from x - 3, six wide;
/// columns x - 3 and x + 2 from y - 2, four deep; status 3). Returns 6.
#[hook(addr = 0x599960, conv = thiscall, name = "Interior_c::Office_PlaceDeskQuad", this = "Interior_c",
       expect = "8B 44 24 10 53 55",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn office_place_desk_quad(this: GPtr<u8>, _a0: i32, tx: i32, ty: i32, id: i32) -> i32 {
    office_place_desk(this, tx, ty - 2, 2, 0x46, 0, id);
    office_place_desk(this, tx, ty, 0, 0x46, 0, id);
    office_place_desk(this, tx - 2, ty, 0, 0x46, 0, id);
    office_place_desk(this, tx - 2, ty - 2, 2, 0x46, 0, id);
    set_tiles_status(this, tx - 3, ty - 3, 6, 1, 3, 0);
    set_tiles_status(this, tx - 3, ty + 2, 6, 1, 3, 0);
    set_tiles_status(this, tx - 3, ty - 2, 1, 4, 3, 0);
    set_tiles_status(this, tx + 2, ty - 2, 1, 4, 3, 0);
    6
}

/// `Interior_c::Office_FurnishCenter()` (0x599A30): the free centre (W - 6 by D - 6) holds a grid of
/// `(W - 6) / 6` by `(D - 6) / 6` desk quads, 6 tiles apart and centred (offset half the remainder,
/// C division, + 6), each with the office desk id (+0x791).
#[hook(addr = 0x599A30, conv = thiscall, name = "Interior_c::Office_FurnishCenter", this = "Interior_c",
       expect = "83 EC 0C 53 55",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn office_furnish_center_port(this: GPtr<u8>) {
    let info = rd::<GPtr<u8>>(this, 0x14);
    let w = i32::from(rd::<u8>(info, 2)) - 6;
    let d = i32::from(rd::<u8>(info, 3)) - 6;
    let nx = w / 6;
    let ny = d / 6;
    let ox = (w - nx * 6) / 2;
    let oy = (d - ny * 6) / 2;
    if w <= 0 || d <= 0 || nx <= 0 {
        return;
    }
    let mut px = ox + 6;
    for _ in 0..nx {
        let mut py = oy + 6;
        for _ in 0..ny.max(0) {
            office_place_desk_quad_call(this, -1, px, py, i32::from(rd::<i8>(this, 0x791)));
            py += 6;
        }
        px += 6;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_FURNISH,
    HOOK_UNFURNISH,
    HOOK_INTERIOR_EXIT,
    HOOK_MANAGER_GET_BOUNDING_BOX,
    HOOK_GET_VECTORS_INTERIOR,
    HOOK_GET_PEDS_INTERIOR,
    HOOK_FURNISH_OFFICE_PORT,
    HOOK_FIND_EMPTY_TILES,
    HOOK_OFFICE_PLACE_DESK_QUAD,
    HOOK_OFFICE_FURNISH_CENTER_PORT,
];
