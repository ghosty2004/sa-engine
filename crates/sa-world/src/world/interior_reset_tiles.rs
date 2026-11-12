//! WRITER-WORLD-2 (batch `world6-2`, SA:MP-executed): `Interior_c::ResetTiles`, the tile map reset
//! of an interior before it is furnished. Ported from the 1.0 US bytes (the catalogue splits the
//! body at 0x593A70, a loop head inside it); gta-reversed has it unreversed.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;

use super::common::{rd, wr};

/// Marks the free (0) tiles of the rectangle `x..x+w` by `y..y+d` of the 30 x 30 tile map (+0x68,
/// index `x * 30 + y`) with `status`, when the rectangle is non-empty, starts at or above 0 and ends
/// within the interior's width (info +2) and depth (info +3). This is the check every inlined copy
/// in `ResetTiles` performs (in different orders; all of them are side-effect free). The copies test
/// `!= 3` before `== 0`, so only free tiles change.
fn mark(this: GPtr<u8>, x: i32, y: i32, w: i32, d: i32, status: u8) {
    let info = rd::<GPtr<u8>>(this, 0x14);
    let (wd, dp) = (i32::from(rd::<u8>(info, 2)), i32::from(rd::<u8>(info, 3)));
    if x < 0 || y < 0 || w <= 0 || d <= 0 || x + w > wd || y + d > dp {
        return;
    }
    for i in 0..w {
        for j in 0..d {
            let o = 0x68 + ((x + i) * 30 + y + j) as isize;
            let t = rd::<u8>(this, o);
            if t == 0 {
                wr::<u8>(this, o, status);
            }
        }
    }
}

/// `Interior_c::ResetTiles()` (0x593910): clears the 900-byte tile map (+0x68), then marks the free
/// tiles in front of the doors of the interior info: the spans (start, end) at info +6/+7 and +0xC/+0xD
/// along the x = 0 wall, +8/+9 and +0xE/+0xF along x = width - 1, +0xA/+0xB and +0x10/+0x11 along
/// y = depth - 1 (status 8 for the first three, 9 for the others; a start of -1 skips the span), then
/// the three rectangles at info +0x12+k (x), +0x15+k (y), +0x18+k (w), +0x1B+k (d) with status 0xB
/// (an x or y of -1 skips it).
#[hook(addr = 0x593910, conv = thiscall, name = "Interior_c::ResetTiles", this = "Interior_c",
       expect = "83 EC 08 53 8B D9",
       unverified = "world6-2", route = "L2", hazards = "none")]
pub fn reset_tiles(this: GPtr<u8>) {
    for k in 0..0xE1 {
        wr::<u32>(this, 0x68 + 4 * k, 0);
    }
    let info = || rd::<GPtr<u8>>(this, 0x14);
    let wd = || i32::from(rd::<u8>(info(), 2));
    let dp = || i32::from(rd::<u8>(info(), 3));
    let span = |a: isize| {
        let s = rd::<i8>(info(), a);
        (s, i32::from(rd::<i8>(info(), a + 1)) - i32::from(s))
    };
    let walls: [(isize, u8, u8); 6] = [(6, 0, 8), (8, 1, 8), (0xA, 2, 8), (0xC, 0, 9), (0xE, 1, 9), (0x10, 2, 9)];
    for (a, wall, status) in walls {
        let (s, n) = span(a);
        if s == -1 {
            continue;
        }
        let s = i32::from(s);
        match wall {
            0 => mark(this, 0, s, 1, n, status),
            1 => mark(this, wd() - 1, s, 1, n, status),
            _ => mark(this, s, dp() - 1, n, 1, status),
        }
    }
    for k in 0..3 {
        let x = rd::<i8>(info(), 0x12 + k);
        let y = rd::<i8>(info(), 0x15 + k);
        if x == -1 || y == -1 {
            continue;
        }
        let w = i32::from(rd::<i8>(info(), 0x18 + k));
        let d = i32::from(rd::<i8>(info(), 0x1B + k));
        mark(this, i32::from(x), i32::from(y), w, d, 0xB);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_RESET_TILES];
