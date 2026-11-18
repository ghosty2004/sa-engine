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

/// `CMirrors::BuildCamMatrix(CMatrix& m, CVector a, CVector b)` (0x723150, cdecl, both vectors by value): the
/// position `a` (`SetTranslateOnly`), forward = `b - a` normalised, right = `CrossProduct(up = (0, 0, 1), forward)`
/// normalised, up = `CrossProduct(forward, right)`. The original builds the (0, 0, 1) vector and the cross
/// products' results in its own argument slots (`a`'s), as here only through `CrossProduct`'s out pointer.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x723150, conv = cdecl, name = "CMirrors::BuildCamMatrix", expect = "83 EC 0C 8B 44 24 1C",
       unverified = "frontier2-c20", route = "L2", hazards = "stack-residue",
       note = "writes (0, 0, 1) and the cross products into its own argument slots")]
pub fn build_cam_matrix(m: GPtr<u8>, ax: f32, ay: f32, az: f32, bx: f32, by: f32, bz: f32) {
    matrix_set_translate_only(m, ax, ay, az);
    let fwd = m.byte_add(0x10);
    wr::<f32>(fwd, 0, x(bx).fsub_m(ax).to_f32());
    wr::<f32>(fwd, 4, x(by).fsub_m(ay).to_f32());
    wr::<f32>(fwd, 8, x(bz).fsub_m(az).to_f32());
    vector_normalise(fwd.cast());
    let up = m.byte_add(0x20);
    wr::<u32>(up, 0, 0);
    wr::<u32>(up, 4, 0);
    wr::<u32>(up, 8, 0x3F80_0000);
    let mut out = [0u32; 3];
    let r = cross_product(lp(&mut out), up, fwd);
    for i in 0..3 {
        wr::<u32>(m, i * 4, rd::<u32>(r, i * 4));
    }
    vector_normalise(m.cast());
    let r = cross_product(lp(&mut out), fwd, m);
    for i in 0..3 {
        wr::<u32>(up, i * 4, rd::<u32>(r, i * 4));
    }
}

/// `CMirrors::BuildCameraMatrixForScreens(CMatrix& m)` (0x7266B0): by `s = (ms / 1000) & 31` and `t = (ms % 1000)
/// * 0.001 + s'` (CTimer ms 0xB7CB84; the byte table 0x7268D0): 0..3 pans along x from `(t * 6 - 1249.3,
/// -224.5, 1064.2)` towards `(-1265.4, -207.5, 1053.2)`; 10..13 a fixed view; 22..25 (`s' = s - 22`) tilts towards
/// `(-1428, -256.7, t * 3 + 1057.3)`; 4..9, 14..21 and 26..31 the matrix turned about z by `(ms & 0x3FFF) *
/// 0.000383` and moved to (-1397, -219, 1054) (`SetRotateZOnly` / `SetTranslateOnly`). The vectors are
/// the literal bits of the original.
#[hook(addr = 0x7266B0, conv = cdecl, name = "CMirrors::BuildCameraMatrixForScreens", expect = "83 EC 20 8B 0D 84 CB B7 00",
       unverified = "frontier2-c20", route = "L2", hazards = "none")]
pub fn build_camera_matrix_for_screens(m: GPtr<u8>) {
    let ms = g::<u32>(0xB7_CB84).read();
    let s = ((ms / 1000) & 0x1F) as i32;
    let rem = (ms % 1000) as i32;
    let f = f32::from_bits;
    let t = |sec: i32| xi(rem).fmul_m(g::<f32>(0x85_8CDC).read()).fadd(xi(sec));
    match s {
        0..=3 => {
            let a = t(s).fmul_m(g::<f32>(0x85_8B44).read()).fsub_m(g::<f32>(0x87_2A00).read()).to_f32();
            build_cam_matrix(m, a, f(0xC360_8000), f(0x4485_0666), f(0xC49E_2CCD), f(0xC34F_8000), f(0x4483_A666));
        }
        22..=25 => {
            let z = t(s - 22).fmul_m(g::<f32>(0x85_8B3C).read()).fadd_m(g::<f32>(0x87_29FC).read()).to_f32();
            build_cam_matrix(m, f(0xC4B8_E000), f(0xC391_4000), f(0x4489_7148), f(0xC4B2_8000), f(0xC380_599A), z);
        }
        10..=13 => {
            build_cam_matrix(m, f(0xC4AF_CCCD), f(0xC307_4CCD), f(0x4482_B4CD), f(0xC4AF_5000), f(0xC312_CCCD), f(0x4482_6333));
        }
        _ => {
            let a = xi((ms & 0x3FFF) as i32).fmul_m(g::<f32>(0x87_29F8).read()).to_f32();
            matrix_set_rotate_z_only(m, a);
            matrix_set_translate_only(m, f(0xC4AE_A000), f(0xC35B_0000), f(0x4483_C000));
        }
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_CREATE_BUFFER, HOOK_BUILD_CAM_MATRIX, HOOK_BUILD_CAMERA_MATRIX_FOR_SCREENS];

use crate::common::{wr, x, xi};
use sa_core::bindings::{cross_product, matrix_set_rotate_z_only, matrix_set_translate_only, vector_normalise};
use sa_core::field::ptr_of as lp;
