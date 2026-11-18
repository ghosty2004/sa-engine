//! WRITER-WORLD-2 (batch `world6-4`): `Interior_c::Office_PlaceEdgeDesks`. Ported from the 1.0 US
//! bytes; gta-reversed has it unreversed.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;

use super::common::{rd, x};
use sa_core::field::ptr_of as p;

use crate::bindings::get_num_empty_tiles;
use crate::bindings::office_place_desk;

use super::interior_shop_furnishing::rand_scaled;

/// `Interior_c::Office_PlaceEdgeDesks(int32 id, int32 x, int32 y, int32 dir, int32 wall)` (0x5995B0):
/// rolls `r1 = rand() * 100 / 32768` and `limit = 30 - rand() * -40 / 32768`; with at most one
/// empty tile at (x, y) towards `dir == 0 || dir == 2 ? 1 : 2` returns 1. The desk count is
/// `2 - rand() * -2 / 32768` (0..2 more) when below half the free tiles, else a fresh roll of the
/// same; `r1 > limit` returns 1. Otherwise the desks (chance 70, the office id +0x791, facing
/// `(dir - 2) & 3`) are placed along the wall from the start tile: wall 0 at (x + n, y - 1), 1 at (x,
/// y + n), 2 at (x + n, y), 3 at (x - 1, y + n), `n` advanced by each desk's width. Returns n + 1.
#[hook(addr = 0x5995B0, conv = thiscall, name = "Interior_c::Office_PlaceEdgeDesks", this = "Interior_c",
       expect = "83 EC 08 53 55 57",
       unverified = "world6-4", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x5995BD, 0x5995E4, 0x599654 and (when the first count is not below half) 0x599686; the id argument is unused")]
pub fn office_place_edge_desks(this: GPtr<u8>, _id: i32, tx: i32, ty: i32, dir: i32, wall: i32) -> i32 {
    let r1 = rand_scaled(0x59_95BD, 100.0);
    let limit = 30 - rand_scaled(0x59_95E4, f32::from_bits(0xC220_0000));
    let n = if dir == 2 || dir == 0 { 1 } else { 2 };
    let free = get_num_empty_tiles(this, tx, ty, n, 1);
    if free <= 1 {
        return 1;
    }
    let half = free / 2;
    let mut count = 2 - rand_scaled(0x59_9654, -2.0);
    if half < count {
        count = half;
    } else {
        count = 2 - rand_scaled(0x59_9686, -2.0);
    }
    if r1 > limit {
        return 1;
    }
    let mut placed = 0;
    let face = (dir - 2) & 3;
    let mut k = count;
    while k > 0 {
        let id = i32::from(rd::<i8>(this, 0x791));
        let w = match wall {
            0 => Some(office_place_desk(this, tx + placed, ty - 1, face, 0x46, 0, id)),
            3 => Some(office_place_desk(this, tx - 1, ty + placed, face, 0x46, 0, id)),
            1 => Some(office_place_desk(this, tx, ty + placed, face, 0x46, 0, id)),
            2 => Some(office_place_desk(this, tx + placed, ty, face, 0x46, 0, id)),
            _ => None,
        };
        if let Some(w) = w {
            placed += w;
        }
        k -= 1;
    }
    placed + 1
}

/// This module's registrations.
/// `Interior_c::Office_PlaceDesk(int32 x, int32 y, int32 dir, int32, int32, int32 id)` (0x5993E0, thiscall, `ret 0x18`;
/// the fourth and fifth arguments are not read). `rot = (dir - 2) & 3`. The desk (`GetFurniture(1, 0, id, wealth)`,
/// wealth = the group's +0x1F) is placed at the tile `(x + (dir == 1), y + (dir == 2))`; no desk placed returns 1.
/// Then the chair (`GetFurniture(1, 1, office id +0x792 (sign-extended into AX; EAX's high half is the previous
/// value: dir, or y + 1 for dir 1), wealth)`) at `(x, y)` moved by dir (2: x + 1, 0: y + 1, 3: both) with the last
/// argument 1; a sit-at-desk point (type 6) at the chair tile's float coordinates moved by 0.5 towards `rot` (2: +y,
/// 0: -y, 3: -x, 1: +x), facing `(rot - 2) & 3`, with the chair object; the tile `(x + (dir is 0 or 3), y + (dir is
/// 0, 1 or 3))` gets status 2 (1x1, forced). Returns 2.
#[hook(addr = 0x5993E0, conv = thiscall, name = "Interior_c::Office_PlaceDesk", this = "Interior_c",
       expect = "83 EC 0C 8B 44 24 18 53", unverified = "frontier2-a19", route = "L2", hazards = "heap-writes",
       note = "office interiors (Office_PlaceEdgeDesks / Office_FurnishCenter)")]
#[allow(clippy::too_many_arguments)]
pub fn office_place_desk_port(this: GPtr<u8>, tx: i32, ty: i32, dir: i32, _a3: i32, _a4: i32, id: i32) -> i32 {
    use crate::bindings::{add_interior_info, get_furniture, place_furniture, set_tiles_status};
    const FURNITURE_MANAGER: usize = 0xBA_B380;
    let man = GPtr::<u8>::from_addr(FURNITURE_MANAGER);
    let rot = dir.wrapping_sub(2) & 3;
    let wealth = || u32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 0x1F));
    let (dx, dy) = (tx + i32::from(dir == 1), ty + i32::from(dir == 2));
    let (mut ow, mut od) = (0u32, 0u32);
    let desk = get_furniture(man, 1, 0, id, wealth());
    if place_furniture(this, desk, dx, dy, 0, 1, rot, p(&mut ow), p(&mut od), 0).is_null() {
        return 1;
    }
    let (cx, cy, mut eax) = match dir {
        2 => (tx + 1, ty, 2u32),
        0 => (tx, ty + 1, 0),
        3 => (tx + 1, ty + 1, 3),
        1 => (tx, ty, ty.wrapping_add(1) as u32),
        d => (tx, ty, d as u32),
    };
    let (sx, sy) = match dir {
        0 => (tx + 1, ty + 1),
        3 => (tx + 1, ty + 1),
        1 => (tx, ty + 1),
        _ => (tx, ty),
    };
    eax = (eax & 0xFFFF_0000) | u32::from(rd::<i8>(this, 0x792) as i16 as u16);
    let chair = get_furniture(man, 1, 1, eax as i32, wealth());
    let obj = place_furniture(this, chair, cx, cy, 0, 1, rot, p(&mut ow), p(&mut od), 1);
    let (mut fx, mut fy) = (cx as f32, cy as f32);
    match rot {
        2 => fy = x(fy).fadd_m(0.5).to_f32(),
        0 => fy = x(fy).fsub_m(0.5).to_f32(),
        3 => fx = x(fx).fsub_m(0.5).to_f32(),
        _ => fx = x(fx).fadd_m(0.5).to_f32(),
    }
    add_interior_info(this, 6, fx, fy, (rot.wrapping_sub(2) & 3) as u32, obj.addr() as u32);
    set_tiles_status(this, sx, sy, 1, 1, 2, 1);
    2
}

pub const HOOKS: &[HookReg] = &[HOOK_OFFICE_PLACE_EDGE_DESKS, HOOK_OFFICE_PLACE_DESK_PORT];
