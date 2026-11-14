//! WRITER-WORLD-2 (batch `world6-4`): `Interior_c::Office_PlaceEdgeDesks`. Ported from the 1.0 US
//! bytes; gta-reversed has it unreversed.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::hook;

use super::common::{ftol32, rd, xi};

use crate::bindings::get_num_empty_tiles;
use crate::bindings::office_place_desk;

/// `ftol((rand() & 0xFFFF) * (1/32768) * k)`.
fn rand_scaled(site: u32, k: f32) -> i32 {
    let r = at_site(site, rand) & 0xFFFF;
    ftol32(xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(k))
}

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
pub const HOOKS: &[HookReg] = &[HOOK_OFFICE_PLACE_EDGE_DESKS];
