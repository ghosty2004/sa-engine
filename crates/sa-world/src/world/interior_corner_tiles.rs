//! WRITER-WORLD-2 (batch `world6-4`): `Interior_c::SetCornerTiles`. Ported from the 1.0 US bytes;
//! gta-reversed has it unreversed.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;

use super::common::rd;

use crate::bindings::set_tiles_status;

/// `Interior_c::SetCornerTiles(int32 corner, int32 size, int32 status, bool force)` (0x5917C0): the
/// two `size`-long strips along the walls meeting at the corner get `status`: corner 1 at (0, 0)
/// (row then column), 0 at (0, D - 1) / (0, D - size), 2 at (W - size, 0) / (W - 1, 0), 3 at
/// (W - size, D - 1) / (W - 1, D - size); any other corner nothing.
#[hook(addr = 0x5917C0, conv = thiscall, name = "Interior_c::SetCornerTiles", this = "Interior_c",
       expect = "8B 44 24 04 83 F8 01",
       unverified = "world6-4", route = "L2", hazards = "none")]
pub fn set_corner_tiles(this: GPtr<u8>, corner: i32, size: i32, status: i32, force: u32) {
    let info = || rd::<GPtr<u8>>(this, 0x14);
    let w = || i32::from(rd::<u8>(info(), 2));
    let d = || i32::from(rd::<u8>(info(), 3));
    match corner {
        1 => {
            set_tiles_status(this, 0, 0, size, 1, status, force);
            set_tiles_status(this, 0, 0, 1, size, status, force);
        }
        0 => {
            set_tiles_status(this, 0, d() - 1, size, 1, status, force);
            set_tiles_status(this, 0, d() - size, 1, size, status, force);
        }
        2 => {
            set_tiles_status(this, w() - size, 0, size, 1, status, force);
            set_tiles_status(this, w() - 1, 0, 1, size, status, force);
        }
        3 => {
            let (wd, dp) = (w(), d());
            set_tiles_status(this, wd - size, dp - 1, size, 1, status, force);
            let (wd, dp) = (w(), d());
            set_tiles_status(this, wd - 1, dp - size, 1, size, status, force);
        }
        _ => {}
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_SET_CORNER_TILES];
