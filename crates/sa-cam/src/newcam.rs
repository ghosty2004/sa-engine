//! FRONTIER-2 turn 8, lane B (batch `frontier2-b18`): the cinema camera sequencer's two decisions,
//! `CCamera::IsItTimeForNewcam` (0x51D770) and `CCamera::TryToStartNewCamMode` (0x51E560), called by the five
//! `ProcessObbeCinemaCamera*` selectors (obbe.rs). Ported from the 1.0 US bytes with
//! `docs/re/specs/CAM/CCamera.md` as reading aid (gta-reversed has no body); unverified (docs/verify/HOWTO.md).
//! Both entries are `nop; jmp` into a protection fragment that only reserves the stack frame.

use sa_core::abi::HookReg;
use sa_core::field::v3_ptr as vp;
use sa_core::mem::GPtr;
use sa_core::timer::{TIME_MS, fild_u32};
use sa_core::hook;
use sa_math::fpu::{gt, le_or_nan, lt};
use sa_math::x87::X87;

use crate::common::{active_cam, f32_of, g, rd, wr, x, xi};
use sa_core::bindings::{get_pad, rw_camera_set_near_clip_plane, vector_magnitude, world_get_is_line_of_sight_clear};
use sa_world::bindings::{
    entity_get_is_on_screen, find_player_coors, find_player_speed, find_player_vehicle, vector_sub,
};
use sa_world::world::common::entity_pos;

use crate::camproc::{
    cam_dw_birdy, cam_dw_cam_man, cam_dw_heli_chase, cam_dw_plane_cam1, cam_dw_plane_cam2, cam_dw_plane_cam3,
    cam_dw_plane_spotter,
};

/// .data: the longest shot (15000.0) and the absolute maximum (20000.0) in ms.
const MAX_SHOT_TIME: usize = 0x8C_CDF0;
const ABS_MAX_SHOT_TIME: usize = 0x8C_CDF8;
/// The right-stick "released" latch (u8) and the sequence step (i8, +1 / -1) the selectors add.
const STICK_LATCH: usize = 0x8C_CDF4;
const STEP: usize = 0x8C_C471;
/// `TheCamera.m_bUseNearClipScript`, written through its absolute address.
const THE_CAMERA_USE_NEAR_CLIP_SCRIPT: usize = 0xB6_F059;
/// `Scene.m_pRwCamera`
const SCENE_CAMERA: usize = 0xC1_703C;
/// `m_vecFixedModeSource` / `m_pTargetEntity` / `m_bUseNearClipScript` / `m_fNearClipScript`
const FIXED_SOURCE: isize = 0x86C;
const TARGET: isize = 0x958;
const USE_NEAR_CLIP: isize = 0x31;
const NEAR_CLIP: isize = 0x138;
/// The skimmer and the Rhino.
const MI_SKIMMER: i16 = 0x1CC;
const MI_RHINO: i16 = 0x1B0;
/// `CVehicle::m_nVehicleType` (+0x590): 5 = boat.
const VEHICLE_TYPE: isize = 0x590;

fn player_vehicle() -> GPtr<u8> {
    find_player_vehicle(-1, 0)
}

/// `GetIsLineOfSightClear(a, b, buildings only)`.
fn los(a: GPtr<u8>, b: GPtr<u8>) -> bool {
    world_get_is_line_of_sight_clear(a, b, 1, 0, 0, 0, 0, 0, 0) != 0
}

/// `LOS(FindPlayerCoors(-1), &m_vecFixedModeSource)`.
fn los_player_fixed(this: GPtr<u8>) -> bool {
    let mut p = [0f32; 3];
    let pp = find_player_coors(vp(&mut p).cast(), -1);
    los(pp.cast(), this.byte_add(FIXED_SOURCE))
}

/// `LOS(target position, active camera source)` (cases 0 and 8).
fn los_target_cam(this: GPtr<u8>) -> bool {
    let t = entity_pos(rd::<GPtr<u8>>(this, TARGET));
    los(t, active_cam(this).byte_add(0x19C))
}

/// `V = VectorSub(FindPlayerCoors(-1), m_vecFixedModeSource)`, with `V.z = 0` unless `keep_z`.
fn v_from_fixed(this: GPtr<u8>, keep_z: bool) -> [f32; 3] {
    let mut p = [0f32; 3];
    let mut out = [0f32; 3];
    let pp = find_player_coors(vp(&mut p).cast(), -1);
    let r = vector_sub(vp(&mut out), pp.cast(), this.byte_add(FIXED_SOURCE));
    let mut v = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
    if !keep_z {
        v[2] = 0.0;
    }
    v
}

/// `CVector::Magnitude` (0x4082C0) of a local, a fresh call at every use as the bytes do.
fn mag(v: &mut [f32; 3]) -> X87 {
    X87::from_f64_exact(vector_magnitude(vp(v).cast()))
}

/// `moving`: `V . FindPlayerSpeed(-1) > 0` with the speed's z term multiplied by 0.0. The cases add the three
/// products in different orders (`(V.y*s.y + s.z*0) + V.x*s.x` and others); with one term a signed zero (or a
/// NaN for an infinite z) every order gives the same register value.
fn moving(v: &[f32; 3]) -> bool {
    let s = find_player_speed(-1);
    let d = x(v[1]).fmul_m(rd::<f32>(s, 4)).fadd(x(rd::<f32>(s, 8)).fmul_m(0.0)).fadd(x(v[0]).fmul_m(rd::<f32>(s, 0)));
    gt(d, x(0.0))
}

/// `boatSkip`: the player is in a boat and the camera target is not the skimmer (FindPlayerVehicle called twice).
fn boat_skip(this: GPtr<u8>) -> bool {
    !player_vehicle().is_null()
        && rd::<i32>(player_vehicle(), VEHICLE_TYPE) == 5
        && rd::<i16>(rd::<GPtr<u8>>(this, TARGET), 0x22) != MI_SKIMMER
}

/// `RwCameraSetNearClipPlane(Scene.m_pRwCamera, 0.15f)`
fn near015() {
    rw_camera_set_near_clip_plane(g::<GPtr<u8>>(SCENE_CAMERA).read(), f32::from_bits(0x3E19_999A));
}

/// `now > start + ms` (unsigned 32-bit).
fn after(start: u32, ms: u32) -> bool {
    TIME_MS.get() > start.wrapping_add(ms)
}

/// `CCamera::IsItTimeForNewcam(int32 seq, uint32 startTimeMs)` (0x51D770, thiscall `ret 8`, AL = 1 to switch).
/// The elapsed time is `fild now (unsigned fix-up) - fisub start`, compared with 20000 in the register and stored
/// f32 for the 15000 tests. A right-stick push past 96 after a release (|x| <= 32 sets the latch) sets the step
/// to +1 / -1 and switches. Then `switch (seq)` (table 0x51E4E8; 9..14 and > 29 keep the camera).
#[hook(addr = 0x51D770, conv = thiscall, name = "CCamera::IsItTimeForNewcam", expect = "90 E9 EB C3 EE FF",
       this = "CCamera", unverified = "frontier2-b18", route = "L2", hazards = "none",
       note = "CVector::Magnitude's ST0 read as f64; the right-stick test of a NaN cannot happen (an int16)")]
pub fn is_it_time_for_newcam(this: GPtr<u8>, seq: i32, start: u32) -> u8 {
    u8::from(time_for_newcam(this, seq, start))
}

fn time_for_newcam(this: GPtr<u8>, seq: i32, start: u32) -> bool {
    if seq < 0 {
        return true;
    }
    let el = fild_u32(TIME_MS.get()).fsub(xi(start as i32));
    let el_f = f32_of(el);
    if gt(el, x(g::<f32>(ABS_MAX_SHOT_TIME).read())) {
        return true;
    }
    let s = f32::from(rd::<i16>(get_pad(0), 4));
    let a = x(s).fabs();
    if le_or_nan(a, x(32.0)) {
        g::<u8>(STICK_LATCH).write(1);
    } else if !le_or_nan(a, x(96.0)) && g::<u8>(STICK_LATCH).read() != 0 {
        g::<i8>(STEP).write(if gt(x(s), x(0.0)) { 1 } else { -1 });
        g::<u8>(STICK_LATCH).write(0);
        return true;
    }
    let long = || gt(x(el_f), x(g::<f32>(MAX_SHOT_TIME).read()));
    match seq {
        0 => {
            let v = player_vehicle();
            if !v.is_null() {
                if rd::<i32>(v, VEHICLE_TYPE) == 5 && rd::<i16>(rd::<GPtr<u8>>(this, TARGET), 0x22) != MI_SKIMMER {
                    return true;
                }
                if rd::<i16>(v, 0x22) == MI_RHINO || !los_target_cam(this) {
                    return true;
                }
            }
            if after(start, 5000) {
                return true;
            }
            near015();
            false
        }
        1 => {
            if long() || boat_skip(this) || !los_player_fixed(this) {
                return true;
            }
            let mut v = v_from_fixed(this, false);
            if gt(mag(&mut v), x(40.0)) && moving(&v) {
                return true;
            }
            lt(mag(&mut v), x(4.5))
        }
        2 => {
            if long() || boat_skip(this) || !los_player_fixed(this) {
                return true;
            }
            let mut v = v_from_fixed(this, false);
            if lt(mag(&mut v), x(2.0)) {
                let n = mag(&mut v).fmul_m(0.5);
                let n = if lt(n, x(0.05)) { x(0.05) } else { n };
                wr::<f32>(this, NEAR_CLIP, f32_of(n));
                wr::<u8>(this, USE_NEAR_CLIP, 1);
            }
            if gt(mag(&mut v), x(29.0)) && moving(&v) {
                return true;
            }
            if lt(mag(&mut v), x(2.0)) {
                return true;
            }
            near015();
            false
        }
        3 => {
            if long() || !los_player_fixed(this) {
                return true;
            }
            let mut v = v_from_fixed(this, false);
            if gt(mag(&mut v), x(48.0)) && moving(&v) {
                return true;
            }
            near015();
            false
        }
        4 | 6 => after(start, 3000),
        5 => {
            if long() || boat_skip(this) || !los_player_fixed(this) {
                return true;
            }
            let mut v = v_from_fixed(this, false);
            gt(mag(&mut v), x(38.0)) && moving(&v)
        }
        7 => after(start, 2000) && entity_get_is_on_screen(player_vehicle()) == 0,
        8 => {
            if long() || boat_skip(this) || !los_target_cam(this) || after(start, 1000) {
                return true;
            }
            wr::<u32>(this, NEAR_CLIP, 0x3F19_999A);
            wr::<u8>(this, USE_NEAR_CLIP, 1);
            false
        }
        15 | 16 | 17 => {
            if long() {
                return true;
            }
            if player_vehicle().is_null() {
                return false;
            }
            if !los_player_fixed(this) {
                return true;
            }
            let mut v = v_from_fixed(this, false);
            let (far, near) = match seq {
                15 => (44.0, 3.0),
                16 => (50.0, 3.0),
                _ => (50.0, 2.0),
            };
            if gt(mag(&mut v), x(far)) && (seq == 16 || moving(&v)) {
                return true;
            }
            lt(mag(&mut v), x(near))
        }
        18 | 19 => {
            if long() || !los_player_fixed(this) {
                return true;
            }
            let mut v = v_from_fixed(this, seq == 18);
            let (far, near) = if seq == 18 { (57.0, 1.0) } else { (36.0, 2.0) };
            if gt(mag(&mut v), x(far)) {
                return true;
            }
            lt(mag(&mut v), x(near))
        }
        20..=23 | 26..=28 => !dw_cam(this, seq),
        24 | 25 => {
            g::<u8>(THE_CAMERA_USE_NEAR_CLIP_SCRIPT).write(0);
            true
        }
        29 => after(start, 5000),
        _ => false,
    }
}

/// `m_aCams[m_nActiveCam].Process_DW_X(true)` for the DW sequence ids 20..23, 26..28.
fn dw_cam(this: GPtr<u8>, seq: i32) -> bool {
    let c = active_cam(this);
    let f: fn(GPtr<u8>, u32) -> u8 = match seq {
        20 => cam_dw_heli_chase,
        21 => cam_dw_cam_man,
        22 => cam_dw_birdy,
        23 => cam_dw_plane_spotter,
        26 => cam_dw_plane_cam1,
        27 => cam_dw_plane_cam2,
        _ => cam_dw_plane_cam3,
    };
    f(c, 1) != 0
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_IS_IT_TIME_FOR_NEWCAM];
