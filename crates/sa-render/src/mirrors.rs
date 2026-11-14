//! PORT-RENDER: `CMirrors` (static class). Ported from the 1.0 US bytes; gta-reversed `Mirrors.cpp` and
//! `docs/re/specs/RENDER/CMirrors.deviations.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;

use crate::common::{g, rd};

sa_core::globals! {
    list GLOBALS;
    /// `CMirrors::pBuffer` (RwRaster*)
    pub static BUFFER @ 0xC7C71C: u32 = 0;
    /// `CMirrors::pZBuffer` (RwRaster*)
    pub static Z_BUFFER @ 0xC7C720: u32 = 0;
}

/// `Scene.m_pRwCamera` (0xC1703C).
const SCENE_CAMERA: usize = 0xC1703C;
/// `g_fx` (0xA9AE00).
const FX: usize = 0xA9AE00;

pub use sa_rwcore::bindings::rw_raster_create;
pub use sa_rwcore::bindings::rw_raster_destroy;
pub use crate::bindings::fx_get_fx_quality as get_fx_quality;

/// `CMirrors::CreateBuffer()` (0x7230A0): once (`pBuffer` null), with the frame buffer's depth
/// (`Scene.m_pRwCamera->frameBuffer->depth`): at FX quality >= 1 a 1024x512 camera raster (type 5) and
/// z raster (type 1); when either fails (the first destroyed) or at a lower quality, 512x256 ones
/// (stored without a check).
#[hook(addr = 0x7230A0, conv = cdecl, name = "CMirrors::CreateBuffer", expect = "A1 1C C7 C7 00")]
pub fn create_buffer() {
    if BUFFER.get() != 0 {
        return;
    }
    let cam: GPtr<u8> = g::<GPtr<u8>>(SCENE_CAMERA).read();
    let fb: GPtr<u8> = rd(cam, 0x60);
    let depth: i32 = rd(fb, 0x14);
    if get_fx_quality(g(FX)) as i32 >= 1 {
        let b = rw_raster_create(0x400, 0x200, (depth as u32) as i32, 5);
        BUFFER.set(b.addr() as u32);
        if !b.is_null() {
            let z = rw_raster_create(0x400, 0x200, (depth as u32) as i32, 1);
            Z_BUFFER.set(z.addr() as u32);
            if !z.is_null() {
                return;
            }
            rw_raster_destroy(GPtr::from_addr(BUFFER.get() as usize));
            BUFFER.set(0);
        }
    }
    let b = rw_raster_create(0x200, 0x100, (depth as u32) as i32, 5);
    BUFFER.set(b.addr() as u32);
    let z = rw_raster_create(0x200, 0x100, (depth as u32) as i32, 1);
    Z_BUFFER.set(z.addr() as u32);
}

pub const HOOKS: &[HookReg] = &[HOOK_CREATE_BUFFER];
