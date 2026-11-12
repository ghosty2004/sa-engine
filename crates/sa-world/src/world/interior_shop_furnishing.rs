//! WRITER-WORLD-2 (batch `world6-2`, SA:MP-executed): the shop furnishing helpers that place
//! multi-piece shelf units and the counter. `Interior_c::Shop_Place3PieceUnit`'s body continues in
//! the no-CD protection chunk 0x408080, decoded by hand: `movsx edx, byte [esi+0x790]; jmp 0x599BD5`
//! (the furniture group load, moved out of the body). Ported from the 1.0 US bytes; gta-reversed
//! has none of them reversed.
//!
//! `PlaceFurniture`'s two size outputs go to argument slots of the caller in the original; when it
//! does not write them they keep those arguments' values, which is modelled by initialising them
//! the same way.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use super::common::{ftol32, g, rd, xi};

orig! {
    /// `FurnitureManager_c::GetFurniture(int32 group, int32 subGroup, int16 id, uint8 wealth)` (0x5911E0) on `g_furnitureMan` (0xBAB380)
    fn get_furniture(man: GPtr<u8>, group: i32, sub: i32, id: i32, wealth: u32) -> GPtr<u8> = thiscall 0x5911E0 => unported;
    /// `Interior_c::PlaceFurniture(Furniture_c*, int32 x, int32 y, float height, int32, int32 rot, int32* outW, int32* outD, uint8)` (0x592AA0)
    fn place_furniture(this: GPtr<u8>, f: GPtr<u8>, x: i32, y: i32, h: u32, a5: i32, rot: i32, ow: GPtr<u8>, od: GPtr<u8>, a9: u32) -> i32 = thiscall 0x592AA0 => unported;
    /// `Interior_c::GetNumEmptyTiles(int32 x, int32 y, int32 dir, int32 count)` (0x591920)
    fn get_num_empty_tiles(this: GPtr<u8>, x: i32, y: i32, dir: i32, n: i32) -> i32 = thiscall 0x591920 => unported;
    /// `Interior_c::SetTilesStatus(int32 x, int32 y, int32 w, int32 d, int32 status, bool)` (0x591700)
    fn set_tiles_status(this: GPtr<u8>, x: i32, y: i32, w: i32, d: i32, status: i32, force: u32) = thiscall 0x591700 => unported;
    /// `Interior_c::Shop_Place3PieceUnit(int32, int32, int32, int32, int32)` (0x599BB0)
    fn shop_place_3_piece_unit_call(this: GPtr<u8>, ty: i32, x: i32, y: i32, dir: i32, len: i32) -> u8 = thiscall 0x599BB0 => shop_place_3_piece_unit;
    /// `Interior_c::Shop_PlaceEdgeUnits(int32, int32, int32, int32)` (0x599DC0)
    fn shop_place_edge_units_call(this: GPtr<u8>, ty: i32, x: i32, y: i32, dir: i32) -> i32 = thiscall 0x599DC0 => shop_place_edge_units;
    /// `Interior_c::AddGotoPt(int32 x, int32 y, float dx, float dy)` (0x591D20)
    fn add_goto_pt(this: GPtr<u8>, tx: i32, ty: i32, dx: f32, dy: f32) = thiscall 0x591D20 => super::interior_info::interior_add_goto_pt;
    /// `Interior_c::Shop_AddShelfInfo(int32 x, int32 y, int32 dir)` (0x59A140)
    fn shop_add_shelf_info(this: GPtr<u8>, tx: i32, ty: i32, dir: u32) = thiscall 0x59A140 => super::interior_group_update::interior_shop_add_shelf_info;
    /// `Interior_c::GetTileStatus(int32 x, int32 y)` (0x5918E0)
    fn get_tile_status(this: GPtr<u8>, tx: i32, ty: i32) -> u32 = thiscall 0x5918E0 => super::misc2::interior_get_tile_status;
    /// `Interior_c::AddInteriorInfo(int32 type, float x, float y, int32 dir, CEntity*)` (0x591E40)
    fn add_interior_info(this: GPtr<u8>, ty: i32, x: f32, y: f32, dir: i32, e: u32) = thiscall 0x591E40 => unported;
}

/// The shelf-info spacing counter (0x8D0948) shared with `Shop_AddShelfInfo`.
const SHELF_COUNTER: usize = 0x8D_0948;

/// The shelf type roll of the furnishing passes: `ftol(rand() * 100 / 32768)` > 50: 0, > 25: 3, >
/// 10: 6, else 9.
fn shelf_type(site: u32) -> i32 {
    let r = rand_scaled(site, 100.0);
    if r > 50 {
        0
    } else if r > 25 {
        3
    } else if r > 10 {
        6
    } else {
        9
    }
}

/// `x` lies in the door span `info[a] ..= info[a + 1]` (a start of -1 is no door).
fn in_door_span(info: GPtr<u8>, a: isize, v: i32) -> bool {
    let s = rd::<i8>(info, a);
    s != -1 && v >= i32::from(s) && v <= i32::from(rd::<i8>(info, a + 1))
}

/// `g_furnitureMan` (0xBAB380).
const FURNITURE_MAN: usize = 0xBA_B380;
/// `1 / 32768` (0x858B14).
const INV_32768: f32 = f32::from_bits(0x3800_0000);
/// The shop's shelf-type roll (0xBB3DE4), set by the shop furnishing code.
const SHOP_ROLL: usize = 0xBB_3DE4;

/// A Rust local passed by address to an original callee.
fn p<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

fn wealth(this: GPtr<u8>) -> u32 {
    u32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 0x1F))
}

/// `ftol((rand() & 0xFFFF) * (1/32768) * k)`.
fn rand_scaled(site: u32, k: f32) -> i32 {
    let r = at_site(site, rand) & 0xFFFF;
    ftol32(xi(r).fmul_m(INV_32768).fmul_m(k))
}

/// `Interior_c::Shop_Place3PieceUnit(int32 type, int32 x, int32 y, int32 dir, int32 length)`
/// (0x599BB0): the unit's two end pieces (furniture group = the interior's +0x790, subgroups `type`
/// and `type + 1`, the second with the first's id +0xA; swapped for dir 2 / 3) and its middle
/// piece (subgroup `type + 2`, same id) are placed in a row from `(x, y)`: along y for dirs 1 / 3,
/// along x for dirs 0 / 2, each next tile advanced by the previous piece's size output (depth
/// along y, width along x); `length - 2` middle pieces. Returns true.
#[hook(addr = 0x599BB0, conv = thiscall, name = "Interior_c::Shop_Place3PieceUnit", this = "Interior_c",
       expect = "83 EC 08 53 8B 5C 24 1C",
       unverified = "world6-2", route = "L2", hazards = "heap-writes",
       note = "body continues in protection chunk 0x408080 (movsx edx, byte [esi+0x790]); the size outputs live in the type / dir argument slots; AL result over PlaceFurniture's EAX")]
pub fn shop_place_3_piece_unit(this: GPtr<u8>, ty: i32, px: i32, py: i32, dir: i32, len: i32) -> u8 {
    let group = || i32::from(rd::<i8>(this, 0x790));
    let (first, second);
    if dir == 2 || dir == 3 {
        first = get_furniture(g(FURNITURE_MAN), group(), ty + 1, -1, wealth(this));
        let id = i32::from(rd::<u16>(first, 0xA));
        second = get_furniture(g(FURNITURE_MAN), group(), ty, id, wealth(this));
    } else {
        first = get_furniture(g(FURNITURE_MAN), group(), ty, -1, wealth(this));
        let id = i32::from(rd::<u16>(first, 0xA));
        second = get_furniture(g(FURNITURE_MAN), group(), ty + 1, id, wealth(this));
    }
    let id = i32::from(rd::<u16>(first, 0xA));
    let middle = get_furniture(g(FURNITURE_MAN), group(), ty + 2, id, wealth(this));
    // [esp+0x28] (the dir slot) and [esp+0x1C] (the type slot)
    let (mut ow, mut od) = (dir, ty);
    if dir == 2 || dir == 0 {
        let mut x = px;
        place_furniture(this, first, x, py, 0, 1, dir, p(&mut ow), p(&mut od), 0);
        x += ow;
        let mut n = len - 2;
        while n > 0 {
            place_furniture(this, middle, x, py, 0, 1, dir, p(&mut ow), p(&mut od), 0);
            x += ow;
            n -= 1;
        }
        place_furniture(this, second, x, py, 0, 1, dir, p(&mut ow), p(&mut od), 0);
    } else {
        let mut y = py;
        place_furniture(this, first, px, y, 0, 1, dir, p(&mut ow), p(&mut od), 0);
        y += od;
        let mut n = len - 2;
        while n > 0 {
            place_furniture(this, middle, px, y, 0, 1, dir, p(&mut ow), p(&mut od), 0);
            y += od;
            n -= 1;
        }
        place_furniture(this, second, px, y, 0, 1, dir, p(&mut ow), p(&mut od), 0);
    }
    1
}

/// `Interior_c::Shop_PlaceEdgeUnits(int32 type, int32 x, int32 y, int32 dir)` (0x599DC0): with at most
/// one empty tile at `(x, y)` towards `dir == 0 || dir == 2 ? 1 : 2` returns 1. Otherwise the unit
/// length is `2 - ftol(rand() * -3 / 32768)` (2..4), 3 when exactly 3 tiles are free, at most the
/// free count, one less when that leaves a single tile; a -1 `type` picks 0 / 3 / 6 / 9 from the
/// shop roll (0xBB3DE4 > 50 / > 25 / > 10 / else). Places a 3-piece unit and returns its length.
#[hook(addr = 0x599DC0, conv = thiscall, name = "Interior_c::Shop_PlaceEdgeUnits", this = "Interior_c",
       expect = "8B 44 24 10 83 F8 02",
       unverified = "world6-2", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x599E0A")]
pub fn shop_place_edge_units(this: GPtr<u8>, ty: i32, px: i32, py: i32, dir: i32) -> i32 {
    let n = if dir == 2 || dir == 0 { 1 } else { 2 };
    let free = get_num_empty_tiles(this, px, py, n, 1);
    if free <= 1 {
        return 1;
    }
    let r = rand_scaled(0x59_9E0A, f32::from_bits(0xC040_0000));
    let mut len = 2 - r;
    if free == 3 {
        len = 3;
    } else {
        let d = free.wrapping_sub(len);
        if d < 0 {
            len = free;
        } else if d == 1 {
            len -= 1;
        }
    }
    let kind = if ty != -1 {
        ty
    } else {
        let roll = g::<i32>(SHOP_ROLL).read();
        if roll > 50 {
            0
        } else if roll > 25 {
            3
        } else if roll > 10 {
            6
        } else {
            9
        }
    };
    shop_place_3_piece_unit_call(this, kind, px, py, dir, len);
    len
}

/// `Interior_c::Shop_PlaceCounter(bool left)` (0x599EF0): the counter (furniture 0/12) and the till
/// (0/13) along the back wall: on the left at x = width +5 - 5 (tiles from x - 1, `w + 1` wide, two
/// deep, marked status 2) with the till at width +5 + 1, else at x = width +5 + 2 with the till at
/// width +5 - 2; the till's rotation is `ftol(rand() * 4 / 32768)`. Returns the counter's x + 2.
#[hook(addr = 0x599EF0, conv = thiscall, name = "Interior_c::Shop_PlaceCounter", this = "Interior_c",
       expect = "83 EC 0C 53 55 56",
       unverified = "world6-2", route = "L2", hazards = "uninit-locals,heap-writes",
       note = "rand at_site 0x599F34; the counter width output feeds SetTilesStatus and is an uninitialised stack word when PlaceFurniture does not write it (0 here)")]
pub fn shop_place_counter(this: GPtr<u8>, left: u8) -> i32 {
    let counter = get_furniture(g(FURNITURE_MAN), 0, 0xC, -1, wealth(this));
    let till = get_furniture(g(FURNITURE_MAN), 0, 0xD, -1, wealth(this));
    let rot = rand_scaled(0x59_9F34, 4.0);
    let (mut ow, mut od) = (0i32, 0i32);
    let w5 = || i32::from(rd::<i8>(rd::<GPtr<u8>>(this, 0x14), 5));
    let x;
    if left != 0 {
        x = w5() - 5;
        place_furniture(this, counter, x, 1, 0, 1, 0, p(&mut ow), p(&mut od), 0);
        set_tiles_status(this, x - 1, 0, ow + 1, 1, 2, 0);
        place_furniture(this, till, w5() + 1, 0, 0, 1, rot, p(&mut ow), p(&mut od), 1);
    } else {
        x = w5() + 2;
        place_furniture(this, counter, x, 1, 0, 1, 0, p(&mut ow), p(&mut od), 0);
        set_tiles_status(this, x, 0, ow + 1, 1, 2, 0);
        place_furniture(this, till, w5() - 2, 0, 0, 1, rot, p(&mut ow), p(&mut od), 1);
    }
    x + 2
}

/// `Interior_c::Shop_FurnishAisles()` (0x59A590): for a shop of width W and depth D with
/// `W - 6 > 0` aisles and `D - 7 > 0` rows: goto points at (2, 3) and (2, D - 3), then per aisle
/// column i (x = i + 3) a shelf type from `ftol(rand() * 100 / 32768)` (> 50: 0, > 25: 3, > 10: 6,
/// else 9) and, by `i % 4`: 0 (except the last column) D - 7 edge units facing 3 from y = 4 on, 1
/// the same facing 1, 2 a shelf info at every row (y = 4 + j, dir 3), 3 a walkway (tiles x, 4..,
/// 1 x (D - 7) status 3) with goto points at its two ends; finally goto points at (W - 3, 3) and
/// (W - 3, D - 3).
#[hook(addr = 0x59A590, conv = thiscall, name = "Interior_c::Shop_FurnishAisles", this = "Interior_c",
       expect = "83 EC 14 53 55 56",
       unverified = "world6-2", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x59A60B (once per column)")]
pub fn shop_furnish_aisles(this: GPtr<u8>) {
    let info = rd::<GPtr<u8>>(this, 0x14);
    let cols = i32::from(rd::<u8>(info, 2)) - 6;
    let rows = i32::from(rd::<u8>(info, 3)) - 7;
    if cols <= 0 || rows <= 0 {
        return;
    }
    let back = rows + 4; // D - 3
    add_goto_pt(this, 2, 3, -0.5, -0.5);
    add_goto_pt(this, 2, back, -0.5, 0.5);
    let mut i = 0;
    while i < cols {
        let ty = shelf_type(0x59_A60B);
        let col = i + 3;
        match i % 4 {
            0 => {
                if i != cols - 1 {
                    let mut y = 4;
                    for _ in 0..rows {
                        y += shop_place_edge_units_call(this, ty, col, y, 3);
                    }
                }
            }
            1 => {
                let mut y = 4;
                for _ in 0..rows {
                    y += shop_place_edge_units_call(this, ty, col, y, 1);
                }
            }
            2 => {
                for j in 0..rows {
                    shop_add_shelf_info(this, col, j + 4, 3);
                }
            }
            _ => {
                set_tiles_status(this, col, 4, 1, rows, 3, 0);
                add_goto_pt(this, col, 3, -0.5, -0.5);
                add_goto_pt(this, col, back, -0.5, 0.5);
            }
        }
        i += 1;
    }
    let col = i + 3;
    add_goto_pt(this, col, 3, 0.5, -0.5);
    add_goto_pt(this, col, back, 0.5, 0.5);
}

/// `Interior_c::Shop_FurnishEdges()` (0x59A1B0): edge shelf units along the back wall (y = D - 1,
/// facing 0, x from 1 while < W - 1), the left wall (x = 0, facing 1) and the right wall (x = W - 1,
/// facing 3, y from 1 while <= D - 2), each wall with its own shelf type roll. Then shelf infos
/// (type 8, 40% once the counter 0x8D0948 is above 1; the counter restarts at 1 after one) along
/// the row y = D - 2 outside door C (dir 2), and walkway tiles (status 2 where a tile is still 0) at
/// y = 1; along the columns x = 1 (dir 3) and x = W - 2 (dir 1) for y = 2..D - 3 outside doors B / D
/// (here the counter is kept in a register across the column and reloaded only after a failed
/// roll); walkway tiles at y = 2 (status 2 where free) and status-3 tiles at y = D - 3 for x =
/// 2..W - 3, at x = 2 and x = W - 3 for y = 3..D - 4, and a status-3 row (3, 3) of W - 6 tiles.
#[hook(addr = 0x59A1B0, conv = thiscall, name = "Interior_c::Shop_FurnishEdges", this = "Interior_c",
       expect = "83 EC 10 53 55 56",
       unverified = "world6-2", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x59A1D0, 0x59A249, 0x59A2C1, 0x59A368, 0x59A423, 0x59A494; the side-column pass keeps 0x8D0948 in EDX")]
pub fn shop_furnish_edges(this: GPtr<u8>) {
    let info = || rd::<GPtr<u8>>(this, 0x14);
    let mut w1 = i32::from(rd::<u8>(info(), 2)) - 1;
    let mut d1 = i32::from(rd::<u8>(info(), 3)) - 1;
    let t = shelf_type(0x59_A1D0);
    let mut i = 1;
    if w1 > 1 {
        loop {
            i += shop_place_edge_units_call(this, t, i, d1, 0);
            if i >= w1 {
                break;
            }
        }
    }
    d1 -= 1; // D - 2
    let t = shelf_type(0x59_A249);
    let mut i = 1;
    if d1 >= 1 {
        loop {
            i += shop_place_edge_units_call(this, t, 0, i, 1);
            if i > d1 {
                break;
            }
        }
    }
    let t = shelf_type(0x59_A2C1);
    let mut i = 1;
    if d1 >= 1 {
        loop {
            i += shop_place_edge_units_call(this, t, w1, i, 3);
            if i > d1 {
                break;
            }
        }
    }
    w1 -= 1; // W - 2
    let cnt = || g::<i32>(SHELF_COUNTER);
    let mut xx = 1;
    while xx <= w1 {
        if !in_door_span(info(), 0xA, xx) {
            if cnt().read() > 1 && rand_scaled(0x59_A368, 100.0) > 60 {
                add_interior_info(this, 8, xi(xx).to_f32(), xi(d1).to_f32(), 2, 0);
                cnt().write(0);
            }
            cnt().write(cnt().read().wrapping_add(1));
        }
        if get_tile_status(this, xx, 1) == 0 {
            set_tiles_status(this, xx, 1, 1, 1, 2, 0);
        }
        xx += 1;
    }
    d1 -= 1; // D - 3
    if d1 >= 2 {
        let mut edx = cnt().read();
        for y in 2..=d1 {
            if !in_door_span(info(), 6, y) {
                if edx > 1 {
                    if rand_scaled(0x59_A423, 100.0) > 60 {
                        add_interior_info(this, 8, 1.0, xi(y).to_f32(), 3, 0);
                        edx = 0;
                    } else {
                        edx = cnt().read();
                    }
                }
                edx = edx.wrapping_add(1);
                cnt().write(edx);
            }
            if !in_door_span(info(), 8, y) {
                if edx > 1 {
                    if rand_scaled(0x59_A494, 100.0) > 60 {
                        add_interior_info(this, 8, xi(w1).to_f32(), xi(y).to_f32(), 1, 0);
                        edx = 0;
                    } else {
                        edx = cnt().read();
                    }
                }
                edx = edx.wrapping_add(1);
                cnt().write(edx);
            }
        }
    }
    w1 -= 1; // W - 3
    for x in 2..=w1 {
        if get_tile_status(this, x, 2) == 0 {
            set_tiles_status(this, x, 2, 1, 1, 2, 0);
        }
        set_tiles_status(this, x, d1, 1, 1, 3, 0);
    }
    d1 -= 1; // D - 4
    for y in 3..=d1 {
        set_tiles_status(this, 2, y, 1, 1, 3, 0);
        set_tiles_status(this, w1, y, 1, 1, 3, 0);
    }
    set_tiles_status(this, 3, 3, w1 - 3, 1, 3, 0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_SHOP_PLACE_3_PIECE_UNIT,
    HOOK_SHOP_PLACE_EDGE_UNITS,
    HOOK_SHOP_PLACE_COUNTER,
    HOOK_SHOP_FURNISH_AISLES,
    HOOK_SHOP_FURNISH_EDGES,
];
