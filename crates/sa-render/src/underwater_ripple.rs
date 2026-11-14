//! `CPostEffects::UnderWaterRipple` (0x7039C0, batch `samp2-water-1`): the wobbling screen under water.
//! Ported from the 1.0 US bytes (gta-reversed multiplies the closing strip's wave by `xoffset`; the original
//! does not). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, k, x, xi};

use crate::bindings::rs_restore;
use crate::bindings::rs_set_immediate;
use sa_core::bindings::rs_camera_begin_update;
use sa_core::bindings::rw_camera_end_update;
use sa_core::bindings::rw_im2d_render_primitive;
orig! {
    /// `CPostEffects::ImmediateModeRenderStatesStore()` (0x700CC0)
    fn rs_store() = cdecl 0x700CC0 => crate::mvpd_r3::immediate_mode_render_states_store;
    /// `RwRenderStateSet` (`RwEngineInstance` 0xC97B24, slot 0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
}

/// `Scene.m_pRwCamera` (0xC1703C), `CPostEffects::pRasterFrontBuffer` (0xC402D8), `TempBufferVertices`
/// (0xC5F958, `RwIm2DVertex` 0x1C each), `uiTempBufferVerticesStored` / `IndicesStored` (0xC4B950 / 0xC4B954).
const CAMERA: usize = 0xC1_703C;
const FRONT_BUFFER: usize = 0xC4_02D8;
const VERTS: usize = 0xC5_F958;
const STORED: usize = 0xC4_B950;

/// `fild` of `CTimer::m_snTimeInMilliseconds` as unsigned, times `speed`.
fn time_phase(speed: f32) -> X87 {
    let ms = sa_core::timer::TIME_MS.get() as i32;
    let t = xi(ms);
    let t = if ms < 0 { t.fadd_m(k(0x4F80_0000)) } else { t };
    t.fmul_m(speed)
}

/// One strip pair at vertex `i` (x87 as the original: `v = y / h` kept, u's divided by the integer width).
#[allow(clippy::too_many_arguments)]
fn emit(i: u32, wave: X87, y: X87, h: i32, w: i32, xoff: f32, xoff2: i32, colour: u32) {
    let near_z = g::<u32>(0xC9_7B24).read();
    let near_z = GPtr::<u32>::from_addr(near_z as usize + 0x18).read();
    let cam = g::<GPtr<u8>>(CAMERA).read();
    let v0 = VERTS + i as usize * 0x1C;
    let v1 = v0 + 0x1C;
    let vf = y.fdiv(xi(h));
    g::<f32>(v0 + 0x14).write(wave.fadd_m(xoff).fdiv(xi(w)).to_f32());
    g::<f32>(v0 + 0x18).write(vf.to_f32());
    g::<u32>(v0).write(0);
    g::<f32>(v0 + 4).write(y.to_f32());
    g::<u32>(v0 + 8).write(near_z);
    g::<f32>(v0 + 0xC).write(x(1.0).fdiv(x(cam.byte_add(0x80).cast::<f32>().read())).to_f32());
    let wx = xi(w);
    g::<f32>(v1 + 0x14).write(wx.fadd(wave).fsub_m(xoff).fdiv(wx).to_f32());
    g::<f32>(v1 + 0x18).write(vf.to_f32());
    g::<f32>(v1).write(xi(w.wrapping_add(xoff2)).to_f32());
    g::<f32>(v1 + 4).write(y.to_f32());
    let near_z2 = GPtr::<u32>::from_addr(g::<u32>(0xC9_7B24).read() as usize + 0x18).read();
    g::<u32>(v1 + 8).write(near_z2);
    g::<f32>(v1 + 0xC).write(x(1.0).fdiv(x(g::<GPtr<u8>>(CAMERA).read().byte_add(0x80).cast::<f32>().read())).to_f32());
    g::<u32>(v0 + 0x10).write(colour);
    g::<u32>(v1 + 0x10).write(colour);
}

/// `CPostEffects::UnderWaterRipple(CRGBA colour, float xoffset, float yoffset, float strength, float speed,
/// float freq)` (0x7039C0, cdecl; `strength` unused).
///
/// Ends the camera update and begins it again, clears the temporary buffer counts, stores and sets the
/// immediate-mode states, raster = the front buffer. A triangle strip down the screen: for y from 0 while
/// `y < height` (`y = ftol(y + yoffset)`): `wave = sin(freq * y + time * speed) * xoffset` (time as
/// unsigned, the product computed once), left vertex (0, y) with u = `(wave + xoffset) / width`, right
/// vertex `(width + ftol(2 * xoffset), y)` with u = `((width + wave) - xoffset) / width`, v = y / height,
/// rhw = `1 / nearPlane`, colour `0xFF` alpha with the RGB of `colour`. The closing pair at the last y uses
/// `wave = sin(time * speed + y * freq)` **without** the `xoffset` factor (original). Renders the strip
/// when more than 2 vertices, then restores the states.
#[hook(addr = 0x7039C0, conv = cdecl, name = "CPostEffects::UnderWaterRipple", expect = "83 EC 14 8B 44 24 18",
       unverified = "samp2-water-1", route = "L3-only", hazards = "x87-transcendental",
       recs = "swim", note = "renders; fsin")]
#[allow(clippy::too_many_arguments)]
pub fn underwater_ripple(colour: u32, xoff: f32, yoff: f32, _strength: f32, speed: f32, freq: f32) {
    let (r, gg, b) = (colour & 0xFF, (colour >> 8) & 0xFF, (colour >> 16) & 0xFF);
    rw_camera_end_update(g::<GPtr<u8>>(CAMERA).read());
    rs_camera_begin_update(g::<GPtr<u8>>(CAMERA).read());
    g::<u32>(STORED).write(0);
    g::<u32>(STORED + 4).write(0);
    rs_store();
    rs_set_immediate();
    rw_render_state_set(1, g::<u32>(FRONT_BUFFER).read());
    let xoff2 = ftol32(x(xoff).fadd(x(xoff)));
    let raster = g::<GPtr<u8>>(FRONT_BUFFER).read();
    let rgb = ((((r | 0xFFFF_FF00) << 8) | gg) << 8) | b;
    let mut y = 0i32;
    let mut h = raster.byte_add(0x10).cast::<i32>().read();
    let mut n = g::<u32>(STORED).read();
    if h > 0 {
        let t = time_phase(speed);
        loop {
            let yf = xi(y);
            let wave = x(freq).fmul(yf).fadd(t).fsin().fmul_m(xoff);
            let i = n;
            n += 2;
            emit(i, wave, yf, h, raster.byte_add(0xC).cast::<i32>().read(), xoff, xoff2, rgb);
            g::<u32>(STORED).write(n);
            y = ftol32(yf.fadd_m(yoff));
            h = raster.byte_add(0x10).cast::<i32>().read();
            if y >= h {
                break;
            }
        }
    }
    let yf = xi(y);
    let s = time_phase(speed).fadd(yf.fmul_m(freq)).fsin();
    let i = n;
    n += 2;
    emit(i, s, yf, raster.byte_add(0x10).cast::<i32>().read(), raster.byte_add(0xC).cast::<i32>().read(), xoff,
        xoff2, rgb);
    g::<u32>(STORED).write(n);
    if n > 2 {
        rw_im2d_render_primitive(4, g(VERTS), n as i32);
    }
    rs_restore();
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_UNDERWATER_RIPPLE];
