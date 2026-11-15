//! FRONTIER-2 lane B (batch `frontier2-b1`): `CPostEffects` filters and script switches on the frontier (the manual
//! speed-FX value, the script darkness filter switch and the darkness filter, the night-vision and CCTV overlays, the
//! infrared heat-object lights). Ported from the 1.0 US bytes; gta-reversed `PostEffects.cpp` as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::bindings::{draw_quad, im_render_states_store, rs_restore, rs_set_immediate};
use crate::bindings::{rw_raster_pop_context, rw_raster_push_context, rw_raster_render_fast};
use crate::common::{ftol32, g, jp, x, xi};
use sa_core::bindings::{rs_camera_begin_update, rw_camera_end_update};

orig! {
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `Light::StoreAndSetLightsForInfraredVisionHeatObjects()` (0x735E70)
    fn store_and_set_lights_for_infrared_heat_objects() = cdecl 0x735E70 => sa_core::light::store_and_set_lights_for_infrared_vision_heat_objects;
}

/// `CPostEffects::m_fSpeedFXManualSpeedCurrentFrame` (0xC402C8).
const SPEED_FX_MANUAL: usize = 0xC4_02C8;
/// `CPostEffects::m_bDarknessFilter` (byte 0xC402C4), `m_DarknessFilterAlpha` (0x8D5204) and its default (0x8D50F4).
const DARKNESS_FILTER: usize = 0xC4_02C4;
const DARKNESS_ALPHA: usize = 0x8D_5204;
const DARKNESS_ALPHA_DEFAULT: usize = 0x8D_50F4;
/// `RsGlobal.maximumWidth` / `maximumHeight` (0xC17044 / 0xC17048).
const SCREEN_W: usize = 0xC1_7044;
const SCREEN_H: usize = 0xC1_7048;
/// `CTimer::ms_fTimeStep` (0xB7CB5C), `m_FrameCounter` (0xB7CB84).
const TIME_STEP: usize = 0xB7_CB5C;
const FRAME_COUNTER: usize = 0xB7_CB84;
/// The 0.0 / 1.0 constants (0x858B50 / 0x858624).
const K_ZERO: usize = 0x85_8B50;
const K_ONE: usize = 0x85_8624;

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn gu(a: usize) -> u32 {
    g::<u32>(a).read()
}

/// `fild dword [a]` stored as an f32 argument.
fn fild_f32(a: usize) -> u32 {
    xi(g::<i32>(a).read()).to_f32().to_bits()
}

/// `CPostEffects::SetSpeedFXManualSpeedCurrentFrame(float)` (0x700BE0): the value clamped to `[0, 1]` (`< 0` → 0,
/// `> 1` → 1; NaN is stored as given).
#[hook(addr = 0x700BE0, conv = cdecl, name = "CPostEffects::SetSpeedFXManualSpeedCurrentFrame",
       expect = "D9 44 24 04 D8 1D 50 8B 85 00", unverified = "frontier2-b1", route = "L2", hazards = "none")]
pub fn set_speed_fx_manual_speed_current_frame(v: f32) {
    let cell = g::<f32>(SPEED_FX_MANUAL);
    if !jp(x(v), x(gf(K_ZERO)), 5) {
        cell.write(0.0);
    } else if !x(v).fcom(x(gf(K_ONE))).test_any(0x41) {
        cell.write(1.0);
    } else {
        cell.write(v);
    }
}

/// `CPostEffects::ScriptDarknessFilterSwitch(bool on, int32 alpha)` (0x701170): the switch byte, then the alpha:
/// -1 → the default (0x8D50F4), else clamped to `[0, 255]`.
#[hook(addr = 0x701170, conv = cdecl, name = "CPostEffects::ScriptDarknessFilterSwitch",
       expect = "8A 44 24 04 A2 C4 02 C4 00", unverified = "frontier2-b1", route = "L2", hazards = "none")]
pub fn script_darkness_filter_switch(on: u32, alpha: i32) {
    g::<u8>(DARKNESS_FILTER).write(on as u8);
    let a = g::<i32>(DARKNESS_ALPHA);
    if alpha == -1 {
        a.write(g::<i32>(DARKNESS_ALPHA_DEFAULT).read());
    } else if alpha < 0 {
        a.write(0);
    } else {
        a.write(alpha.min(0xFF));
    }
}

/// `CPostEffects::DarknessFilter(int32 alpha)` (0x702F00): the immediate-mode states stored and set, a black
/// full-screen quad of `alpha` (no raster), the states restored (a tail jump).
#[hook(addr = 0x702F00, conv = cdecl, name = "CPostEffects::DarknessFilter", expect = "E8 BB DD FF FF",
       unverified = "frontier2-b1", route = "L2", hazards = "none")]
pub fn darkness_filter(alpha: u32) {
    im_render_states_store();
    rs_set_immediate();
    draw_quad(0, 0, fild_f32(SCREEN_W), fild_f32(SCREEN_H), 0, 0, 0, alpha, 0);
    rs_restore();
}

/// `CPostEffects::NightVision(RwRGBA)` (0x7011C0; the colour argument is never read). While the flash value
/// (0xC40300) is above 0 it falls by the time step (`<= 0` → 0, NaN kept) and `_ftol(value)` additive quads (states
/// 10 / 11 = 2, colour 8 8 8 255, size 0xC4015C / 0xC40160, the raster 0xC40158 read at entry) are drawn; then a
/// green modulating full-screen quad (10 = 1, 11 = 3, colour 0x20 0xFF 0x20 0xFF), the states restored (tail jump).
#[hook(addr = 0x7011C0, conv = cdecl, name = "CPostEffects::NightVision", expect = "D9 05 00 03 C4 00",
       unverified = "frontier2-b1", route = "L2", hazards = "none")]
pub fn night_vision(_col: u32) {
    let flash = g::<f32>(0xC4_0300);
    let raster = gu(0xC4_0158);
    if !x(flash.read()).fcom(x(gf(K_ZERO))).test_any(0x41) {
        let v = x(flash.read()).fsub_m(gf(TIME_STEP));
        flash.write(v.to_f32());
        if !v.fcom(x(gf(K_ZERO))).test_parity(0x41) {
            flash.write(0.0);
        }
        im_render_states_store();
        rs_set_immediate();
        rw_render_state_set(0xA, 2);
        rw_render_state_set(0xB, 2);
        let n = ftol32(x(flash.read()));
        for _ in 0..n.max(0) {
            draw_quad(0, 0, fild_f32(0xC4_015C), fild_f32(0xC4_0160), 8, 8, 8, 0xFF, raster);
        }
        rs_restore();
    }
    im_render_states_store();
    rs_set_immediate();
    rw_render_state_set(0xA, 1);
    rw_render_state_set(0xB, 3);
    draw_quad(0, 0, fild_f32(SCREEN_W), fild_f32(SCREEN_H), 0x20, 0xFF, 0x20, 0xFF, 0);
    rs_restore();
}

/// `CPostEffects::InfraredVisionStoreAndSetLightsForHeatObjects(CPed*)` (0x701320): with infrared vision on and post
/// effects not disabled, the four heat colour words 0x8D50BC..0x8D50C8 are saved; for a dying ped (state +0x530 ==
/// 0x37) `t = |frame counter - ped +0x750| * 0.0001` fades them (0x8D50BC -= t, below 0 → 0; 0x8D50C0 = 0; 0x8D50C4
/// += t, above 1 → 1); `Light::StoreAndSetLightsForInfraredVisionHeatObjects()`, then the saved words written back.
#[hook(addr = 0x701320, conv = cdecl, name = "CPostEffects::InfraredVisionStoreAndSetLightsForHeatObjects",
       expect = "51 A0 B9 02 C4 00", unverified = "frontier2-b1", route = "L2", hazards = "none")]
pub fn infrared_vision_store_and_set_lights_for_heat_objects(ped: GPtr<u8>) {
    if g::<u8>(0xC4_02B9).read() == 0 || g::<u8>(0xC4_02B7).read() != 0 {
        return;
    }
    let c4 = gu(0x8D_50C4);
    let c8 = gu(0x8D_50C8);
    let bc = gu(0x8D_50BC);
    let c0 = gu(0x8D_50C0);
    if sa_core::field::rd::<u32>(ped, 0x530) == 0x37 {
        let d = g::<i32>(FRAME_COUNTER).read().wrapping_sub(sa_core::field::rd::<i32>(ped, 0x750));
        let t = xi(d.wrapping_abs()).fmul_m(gf(0x85_8FC4));
        let v = x(gf(0x8D_50BC)).fsub(t);
        g::<f32>(0x8D_50BC).write(v.to_f32());
        if !v.fcom(x(gf(K_ZERO))).test_parity(5) {
            g::<u32>(0x8D_50BC).write(0);
        }
        g::<u32>(0x8D_50C0).write(0);
        g::<f32>(0x8D_50C4).write(x(gf(0x8D_50C4)).fadd(t).to_f32());
        if !x(gf(0x8D_50C4)).fcom(x(gf(K_ONE))).test_any(0x41) {
            g::<u32>(0x8D_50C4).write(0x3F80_0000);
        }
    }
    store_and_set_lights_for_infrared_heat_objects();
    g::<u32>(0x8D_50C0).write(c0);
    g::<u32>(0x8D_50BC).write(bc);
    g::<u32>(0x8D_50C8).write(c8);
    g::<u32>(0x8D_50C4).write(c4);
}

/// `CPostEffects::CCTV()` (0x702F40): state 2 = 3; the camera's frame buffer (`Scene.camera` 0xC1703C +0x60) is
/// copied into the front-buffer copy (0xC402D8) between `RwCameraEndUpdate` / `RsCameraBeginUpdate`; `n =
/// _ftol(H * 0.002232143 * 2)`; states 9 = 1, 14 = 0, 6 = 1, 8 = 0, 1 = the copy, 12 = 1, 10 = 5, 11 = 6; the
/// immediate-mode states stored and set; `H / (2n)` (`idiv`) bands of height n every 2n pixels (x 0, width W) in
/// the CCTV colour (0x8D50FC: r, g and the byte 0x8D50FE as b, alpha 255) with the copy as raster; the states
/// restored (tail jump).
#[hook(addr = 0x702F40, conv = cdecl, name = "CPostEffects::CCTV", expect = "83 EC 08 A1 24 7B C9 00",
       unverified = "frontier2-b1", route = "L2", hazards = "none")]
pub fn cctv() {
    rw_render_state_set(2, 3);
    let cam = g::<GPtr<u8>>(0xC1_703C);
    rw_camera_end_update(cam.read());
    rw_raster_push_context(g::<GPtr<u8>>(0xC4_02D8).read());
    rw_raster_render_fast(sa_core::field::rd(cam.read(), 0x60), 0, 0);
    rw_raster_pop_context();
    rs_camera_begin_update(cam.read());
    let mut y = 0i32;
    let n = ftol32({
        let h = xi(g::<i32>(SCREEN_H).read()).fmul_m(gf(0x85_9524));
        h.fadd(h)
    });
    let step = n.wrapping_add(n);
    let mut count = g::<i32>(SCREEN_H).read().wrapping_div(step);
    rw_render_state_set(9, 1);
    rw_render_state_set(0xE, 0);
    rw_render_state_set(6, 1);
    rw_render_state_set(8, 0);
    rw_render_state_set(1, gu(0xC4_02D8));
    rw_render_state_set(0xC, 1);
    rw_render_state_set(0xA, 5);
    rw_render_state_set(0xB, 6);
    im_render_states_store();
    rs_set_immediate();
    if count > 0 {
        let h = X87::from_i32(n).to_f32().to_bits();
        while count > 0 {
            let col = gu(0x8D_50FC);
            let b = u32::from(g::<u8>(0x8D_50FE).read());
            draw_quad(0, xi(y).to_f32().to_bits(), fild_f32(SCREEN_W), h, col, (col >> 8) & 0xFF, b, 0xFF, gu(0xC4_02D8));
            y = y.wrapping_add(step);
            count -= 1;
        }
    }
    rs_restore();
}

pub const HOOKS: &[HookReg] = &[
    HOOK_SET_SPEED_FX_MANUAL_SPEED_CURRENT_FRAME,
    HOOK_SCRIPT_DARKNESS_FILTER_SWITCH,
    HOOK_DARKNESS_FILTER,
    HOOK_NIGHT_VISION,
    HOOK_INFRARED_VISION_STORE_AND_SET_LIGHTS_FOR_HEAT_OBJECTS,
    HOOK_CCTV,
];
