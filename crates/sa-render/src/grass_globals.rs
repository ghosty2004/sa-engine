//! `CGrassRenderer` global setters (batch `std8-render-1`), ported from the 1.0 US bytes. Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::common::g;

/// `CGrassRenderer::SetGlobalCameraPos(const CVector& pos)` (0x5DABC0, cdecl): the three dwords to
/// `ms_vecCameraPos` (0xC02DDC..0xC02DE4).
#[hook(addr = 0x5DABC0, conv = cdecl, name = "CGrassRenderer::SetGlobalCameraPos", expect = "8B 44 24 04 8B 08 89 0D DC 2D C0 00",
       unverified = "std8-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn grass_set_global_camera_pos(pos: GPtr<u8>) {
    for i in 0..3 {
        g::<u32>(0xC02DDC + i * 4).write(pos.byte_add(i as isize * 4).cast::<u32>().read());
    }
}

/// `CGrassRenderer::SetGlobalWindBending(float)` (0x5DAC00, cdecl): `ms_fWindBending` (0xC02DB8) = the argument
/// (raw dword).
#[hook(addr = 0x5DAC00, conv = cdecl, name = "CGrassRenderer::SetGlobalWindBending", expect = "8B 44 24 04 A3 B8 2D C0 00",
       unverified = "std8-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn grass_set_global_wind_bending(v: u32) {
    g::<u32>(0xC02DB8).write(v);
}

pub const HOOKS: &[HookReg] = &[HOOK_GRASS_SET_GLOBAL_CAMERA_POS, HOOK_GRASS_SET_GLOBAL_WIND_BENDING];
