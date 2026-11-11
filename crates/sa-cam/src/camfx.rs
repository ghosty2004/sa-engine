//! PORT-CAM: `CCamera` fades, shakes, FOV lerps, vector move/track, ducking, collision vars and vehicle
//! tweaks. Ported from the 1.0 US bytes; gta-reversed `Camera.cpp` and `docs/re/specs/CAM/CCamera.md`
//! as reading aids. `CCamera` fields are reached through `this` (thiscall, `this` = `TheCamera`).

use sa_core::abi::HookReg;
use sa_core::game::FADE_RGBV;
use sa_core::mem::GPtr;
use sa_core::platform::SCENE_CAMERA;
use sa_core::timer::{TIME_MS, TIME_STEP, fild_u32};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{active_cam, ftol32, jne, jp, k, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `gCurCamColVars` (index of the active row of `gCamColVars`)
    pub static CUR_CAM_COL_VARS @ 0x8CCB80: u8 = 5;
    /// `gCurDistForCam`
    pub static CUR_DIST_FOR_CAM @ 0x8CCB84: f32 = 1.0;
    /// `gpCamColVars` (`&gCamColVars[gCurCamColVars][0]`)
    pub static CAM_COL_VARS_PTR @ 0xB6FE88: GPtr<u8> = GPtr::NULL;
    /// unnamed f32 divisor of `SetNearClipBasedOnPedCollision` (.bss, written at run time)
    pub static NEAR_CLIP_DIST_DIVISOR @ 0xB6EC68: f32 = 0.0;
    /// unnamed f32 factor of `SetNearClipBasedOnPedCollision` (.data 0.25)
    pub static NEAR_CLIP_DIST_FACTOR @ 0x8CCC84: f32 = 0.25;
    /// unnamed f32 ducking offset base used when the ped is not moving (.data 0.3; result - 1.0)
    pub static DUCK_OFFSET_STILL @ 0x8CCB94: f32 = 0.3;
    /// unnamed f32 ducking offset base used when the ped is moving (.data 0.3; result - 0.5)
    pub static DUCK_OFFSET_MOVING @ 0x8CCB98: f32 = 0.3;
}

/// `gCamColVars` (float[28][6], .data); `gpCamColVars` points into it.
const CAM_COL_VARS: u32 = 0x8CC8E0;
/// `AudioEngine` (CAudioEngine).
const AUDIO_ENGINE: usize = 0xB6BC90;

orig! {
    /// `CCamera::SetCamCollisionVarDataSet(int)` (0x50CB60)
    pub fn set_cam_collision_var_data_set_call(idx: i32) = cdecl 0x50CB60 => set_cam_collision_var_data_set;
    /// `CCamera::InitCameraVehicleTweaks()` (0x50A3B0)
    pub fn init_camera_vehicle_tweaks_call(this: GPtr<u8>) = thiscall 0x50A3B0 => init_camera_vehicle_tweaks;
    /// `CCamera::ProcessVectorTrackLinear(float)` (0x50D350)
    pub fn process_vector_track_linear_call(this: GPtr<u8>, ratio: f32) = thiscall 0x50D350 => process_vector_track_linear_ratio;
    /// `CCamera::ProcessVectorMoveLinear(float)` (0x50D430)
    pub fn process_vector_move_linear_call(this: GPtr<u8>, ratio: f32) = thiscall 0x50D430 => process_vector_move_linear_ratio;
    /// `CCamera::ProcessFOVLerp(float)` (0x50D510)
    pub fn process_fov_lerp_call(this: GPtr<u8>, ratio: f32) = thiscall 0x50D510 => process_fov_lerp_ratio;
    /// `CCamera::ProcessShake(float)` (0x516560)
    pub fn process_shake_float(this: GPtr<u8>, ratio: f32) -> GPtr<u8> = thiscall 0x516560 => unported;
    /// `CPedIntelligence::GetTaskDuck(bool)` (0x6010A0)
    pub fn get_task_duck(intel: GPtr<u8>, ignore_check: u8) -> GPtr<u8> = thiscall 0x6010A0 => unported;
    /// `RwCameraSetNearClipPlane(RwCamera*, RwReal)` (0x7EE1D0)
    pub fn rw_camera_set_near_clip_plane(cam: GPtr<u8>, near: f32) -> GPtr<u8> = cdecl 0x7EE1D0 => unported;
    /// `CAudioEngine::IsLoadingTuneActive()` (0x506D90)
    pub fn audio_is_loading_tune_active(ae: GPtr<u8>) -> u8 = thiscall 0x506D90 => unported;
    /// `CAudioEngine::SetMusicFaderScalingFactor(float)` (0x506E40)
    pub fn audio_set_music_fader_scaling_factor(ae: GPtr<u8>, v: u32) = thiscall 0x506E40 => unported;
    /// `CAudioEngine::SetEffectsFaderScalingFactor(float)` (0x506E50)
    pub fn audio_set_effects_fader_scaling_factor(ae: GPtr<u8>, v: u32) = thiscall 0x506E50 => unported;
}

/// `fild CTimer::m_snTimeInMilliseconds` with the unsigned fix-up (+2^32 [0x858C54]).
fn time_now() -> X87 {
    fild_u32(TIME_MS.get())
}

/// Copies the 12 bytes of a `CVector` dword by dword (raw moves, no float conversion).
fn copy_vec(dst: GPtr<u8>, src: GPtr<u8>) {
    for i in 0..3 {
        wr::<u32>(dst, i * 4, rd::<u32>(src, i * 4));
    }
}

/// `CCamera::VectorTrackRunning()` (0x474870): `time <= m_fTrackLinearEndTime` (+0xC60); NaN gives false.
#[hook(addr = 0x474870, conv = thiscall, name = "CCamera::VectorTrackRunning", expect = "51 A1 84 CB B7 00", this = "CCamera")]
pub fn vector_track_running(this: GPtr<u8>) -> u8 {
    u8::from(!jp(time_now(), x(rd::<f32>(this, 0xC60)), 0x41))
}

/// `CCamera::VectorMoveRunning()` (0x4748A0): `time <= m_fMoveLinearEndTime` (+0xCC0).
#[hook(addr = 0x4748A0, conv = thiscall, name = "CCamera::VectorMoveRunning", expect = "51 A1 84 CB B7 00", this = "CCamera")]
pub fn vector_move_running(this: GPtr<u8>) -> u8 {
    u8::from(!jp(time_now(), x(rd::<f32>(this, 0xCC0)), 0x41))
}

/// `CCamera::SetFadeColour(uint8, uint8, uint8)` (0x50BF00): `m_bFadeTargetIsSplashScreen` (+0x55) =
/// `r == 2 && g == 2 && b == g`; `CDraw::FadeRed/Green/Blue`.
#[hook(addr = 0x50BF00, conv = thiscall, name = "CCamera::SetFadeColour", expect = "8A 44 24 04 3C 02", this = "CCamera")]
pub fn set_fade_colour(this: GPtr<u8>, r: u32, g: u32, b: u32) {
    let (r, g, b) = (r as u8, g as u8, b as u8);
    wr::<u8>(this, 0x55, 0);
    if r == 2 && g == 2 && b == g {
        wr::<u8>(this, 0x55, 1);
    }
    let fade = FADE_RGBV.ptr().cast::<u8>();
    wr::<u8>(fade, 1, g);
    wr::<u8>(fade, 0, r);
    wr::<u8>(fade, 2, b);
}

/// `CCamera::AddShakeSimple(float duration, int type, float intensity)` (0x50D240).
#[hook(addr = 0x50D240, conv = thiscall, name = "CCamera::AddShakeSimple", expect = "8B 44 24 0C 8B 54 24 08", this = "CCamera")]
pub fn add_shake_simple(this: GPtr<u8>, duration: f32, ty: u32, intensity: u32) {
    wr::<u32>(this, 0xC90, intensity);
    wr::<u32>(this, 0xCA0, ty);
    let t = time_now();
    wr::<f32>(this, 0xC94, t.to_f32());
    wr::<f32>(this, 0xC98, t.fadd_m(duration).to_f32());
}

/// Body of `SetCamCollisionVarDataSet`, also inlined into `SetColVarsPed`.
fn col_var_data_set(idx: i32) {
    if idx == i32::from(CUR_CAM_COL_VARS.get()) {
        return;
    }
    CUR_CAM_COL_VARS.set(idx as u8);
    let p = CAM_COL_VARS.wrapping_add((idx as u32).wrapping_mul(0x18));
    CUR_DIST_FOR_CAM.set(1.0);
    CAM_COL_VARS_PTR.set(GPtr::from_addr(p as usize));
}

/// `CCamera::SetCamCollisionVarDataSet(int)` (0x50CB60, static cdecl).
#[hook(addr = 0x50CB60, conv = cdecl, name = "CCamera::SetCamCollisionVarDataSet", expect = "0F B6 0D 80 CB 8C 00")]
pub fn set_cam_collision_var_data_set(idx: i32) {
    col_var_data_set(idx);
}

/// `CCamera::SetColVarsAimWeapon(int)` (0x50CBF0, static cdecl): modes 0..3 tail-jump to
/// `SetCamCollisionVarDataSet(mode)`; other values (unsigned compare) do nothing.
#[hook(addr = 0x50CBF0, conv = cdecl, name = "CCamera::SetColVarsAimWeapon", expect = "8B 44 24 04 83 F8 03")]
pub fn set_col_vars_aim_weapon(mode: i32) {
    if (mode as u32) <= 3 {
        set_cam_collision_var_data_set_call(mode);
    }
}

/// `CCamera::SetColVarsPed(int type, int zoom)` (0x50CC50, static cdecl): type 0 -> set `zoom + 3`,
/// type 1 -> `zoom + 6` (the data-set body is inlined, no call).
#[hook(addr = 0x50CC50, conv = cdecl, name = "CCamera::SetColVarsPed", expect = "8B 44 24 04 83 E8 00")]
pub fn set_col_vars_ped(ty: i32, zoom: i32) {
    let idx = match ty {
        0 => zoom.wrapping_add(3),
        1 => zoom.wrapping_add(6),
        _ => return,
    };
    col_var_data_set(idx);
}

/// `CCamera::SetColVarsVehicle(int type, int zoom)` (0x50CCA0, static cdecl): jump table 0x50CD14,
/// types 0..6 -> `zoom + {9, 12, 15, 18, 9, 21, 24}`; other values (unsigned) do nothing.
#[hook(addr = 0x50CCA0, conv = cdecl, name = "CCamera::SetColVarsVehicle", expect = "8B 44 24 04 83 F8 06")]
pub fn set_col_vars_vehicle(ty: i32, zoom: i32) {
    let add = match ty {
        0 | 4 => 9,
        1 => 0xC,
        2 => 0xF,
        3 => 0x12,
        5 => 0x15,
        6 => 0x18,
        _ => return,
    };
    set_cam_collision_var_data_set_call(zoom.wrapping_add(add));
}

/// `CCamera::ProcessShake()` (0x51A6F0): while `time <= m_fShakeEndTime` (+0xC98), calls
/// `ProcessShake((time - start) / (end - start))` (start +0xC94; ratio stored as f32).
#[hook(addr = 0x51A6F0, conv = thiscall, name = "CCamera::ProcessShake-void", expect = "51 A1 84 CB B7 00", this = "CCamera")]
pub fn process_shake(this: GPtr<u8>) {
    let t = time_now();
    let end = rd::<f32>(this, 0xC98);
    if jp(t, x(end), 0x41) {
        return;
    }
    let start = rd::<f32>(this, 0xC94);
    let ratio = t.fsub_m(start).fdiv(x(end).fsub_m(start)).to_f32();
    process_shake_float(this, ratio);
}

/// `CCamera::LerpFOV(float from, float to, float duration, bool ease)` (0x50D280).
#[hook(addr = 0x50D280, conv = thiscall, name = "CCamera::LerpFOV", expect = "51 A1 84 CB B7 00", this = "CCamera")]
pub fn lerp_fov(this: GPtr<u8>, from: u32, to: u32, duration: f32, ease: u32) {
    let t = time_now();
    wr::<f32>(this, 0xCA4, t.to_f32());
    wr::<u32>(this, 0xCAC, from);
    wr::<f32>(this, 0xCA8, t.fadd_m(duration).to_f32());
    wr::<u32>(this, 0xCB0, to);
    wr::<u8>(this, 0xCB4, ease as u8);
}

/// The duck test shared by the ducking functions: `GetTaskDuck(true)` non-null, ped flag
/// +0x46C bit 0x4000000 and task byte +0x19 clear.
fn ped_is_ducking(ped: GPtr<u8>) -> bool {
    let task = get_task_duck(rd(ped, 0x47C), 1);
    !task.is_null() && rd::<u32>(ped, 0x46C) & 0x0400_0000 != 0 && rd::<u8>(task, 0x19) == 0
}

/// `|m_vecMoveSpeed|^2` (+0x44) in the original order `(x*x + y*y) + z*z`.
fn move_speed_sq(ped: GPtr<u8>) -> X87 {
    let vx = x(rd::<f32>(ped, 0x44));
    let vy = x(rd::<f32>(ped, 0x48));
    let vz = x(rd::<f32>(ped, 0x4C));
    vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz))
}

/// Ducking goal height: moving (`|v|^2 > 1e-6` [0x8630EC]) `0x8CCB98 - 0.5`, else (NaN included)
/// `0x8CCB94 - 1.0`.
fn duck_goal(ped: GPtr<u8>) -> X87 {
    if jne(move_speed_sq(ped), x(k(0x358637BE)), 0x41) {
        x(DUCK_OFFSET_STILL.get()).fsub_m(k(0x3F800000)) // 1.0 [0x858624]
    } else {
        x(DUCK_OFFSET_MOVING.get()).fsub_m(k(0x3F000000)) // 0.5 [0x858B8C]
    }
}

/// `CCamera::ConsiderPedAsDucking(CPed*)` (0x50CEB0).
#[hook(addr = 0x50CEB0, conv = thiscall, name = "CCamera::ConsiderPedAsDucking", expect = "56 8B 74 24 08", this = "CCamera")]
pub fn consider_ped_as_ducking(_this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    u8::from(ped_is_ducking(ped))
}

/// `CCamera::SetNearClipBasedOnPedCollision(float distSq)` (0x50CB90): `A = gpCamColVars[4]`,
/// `n = (sqrt(d) / g_B6EC68 * g_8CCC84) * (0.3 - A) + A` (stored f32, compared unrounded),
/// near clip `n` unless `n < A`, then `A`.
#[hook(addr = 0x50CB90, conv = thiscall, name = "CCamera::SetNearClipBasedOnPedCollision", expect = "A1 88 FE B6 00", this = "CCamera")]
pub fn set_near_clip_based_on_ped_collision(_this: GPtr<u8>, dist_sq: f32) {
    let a = x(rd::<f32>(CAM_COL_VARS_PTR.get(), 0x10));
    let q = x(dist_sq)
        .fsqrt()
        .fdiv_m(NEAR_CLIP_DIST_DIVISOR.get())
        .fmul_m(NEAR_CLIP_DIST_FACTOR.get());
    let c = x(k(0x3E99999A)).fsub(a); // 0.3 [0x858C24]
    let n = q.fmul(c).fadd(a);
    let mut clip = n.to_f32();
    if !jp(n, a, 0x05) {
        clip = a.to_f32();
    }
    rw_camera_set_near_clip_plane(SCENE_CAMERA.get(), clip);
}

/// Shared body of `VectorMoveLinear` / `VectorTrackLinear`: start/end times, positions, ease flag.
fn vector_linear(this: GPtr<u8>, base: isize, end: GPtr<u8>, start: GPtr<u8>, duration: f32, ease: u32) {
    let t = time_now();
    wr::<f32>(this, base, t.to_f32());
    wr::<f32>(this, base + 4, t.fadd_m(duration).to_f32());
    copy_vec(this.byte_add(base + 8), start);
    copy_vec(this.byte_add(base + 0x14), end);
    wr::<u8>(this, base + 0x20, ease as u8);
}

/// `CCamera::VectorMoveLinear(CVector* end, CVector* start, float duration, bool ease)` (0x50D160):
/// +0xCBC start time, +0xCC0 end time, +0xCC4 = *start, +0xCD0 = *end, +0xCDC ease.
#[hook(addr = 0x50D160, conv = thiscall, name = "CCamera::VectorMoveLinear", expect = "51 A1 84 CB B7 00", this = "CCamera")]
pub fn vector_move_linear(this: GPtr<u8>, end: GPtr<u8>, start: GPtr<u8>, duration: f32, ease: u32) {
    vector_linear(this, 0xCBC, end, start, duration, ease);
}

/// `CCamera::VectorTrackLinear(CVector* end, CVector* start, float duration, bool ease)` (0x50D1D0):
/// +0xC5C start time, +0xC60 end time, +0xC64 = *start, +0xC70 = *end, +0xC7C ease.
#[hook(addr = 0x50D1D0, conv = thiscall, name = "CCamera::VectorTrackLinear", expect = "51 A1 84 CB B7 00", this = "CCamera")]
pub fn vector_track_linear(this: GPtr<u8>, end: GPtr<u8>, start: GPtr<u8>, duration: f32, ease: u32) {
    vector_linear(this, 0xC5C, end, start, duration, ease);
}

/// Shared body of the three `Process*` (no ratio) functions: while `time <= end`, the ratio
/// `(time - start) / (end - start)` (f32) goes to `call`; once over (or NaN), `done` is set when
/// `keep` is set.
fn process_wo_ratio(this: GPtr<u8>, start_off: isize, keep: isize, done: isize, call: fn(GPtr<u8>, f32)) {
    let t = time_now();
    let end = rd::<f32>(this, start_off + 4);
    if jp(t, x(end), 0x41) {
        if rd::<u8>(this, keep) != 0 {
            wr::<u8>(this, done, 1);
        }
        return;
    }
    let start = rd::<f32>(this, start_off);
    let ratio = t.fsub_m(start).fdiv(x(end).fsub_m(start)).to_f32();
    call(this, ratio);
}

/// `CCamera::ProcessVectorTrackLinear()` (0x516440): flag +0xCEF -> +0xC8C when over.
#[hook(addr = 0x516440, conv = thiscall, name = "CCamera::ProcessVectorTrackLinear-wo-ratio", expect = "51 A1 84 CB B7 00", this = "CCamera", preserve = "edx")]
pub fn process_vector_track_linear(this: GPtr<u8>) {
    process_wo_ratio(this, 0xC5C, 0xCEF, 0xC8C, process_vector_track_linear_call);
}

/// `CCamera::ProcessVectorMoveLinear()` (0x5164A0): flag +0xCEE -> +0xCEC when over.
#[hook(addr = 0x5164A0, conv = thiscall, name = "CCamera::ProcessVectorMoveLinear-wo-ratio", expect = "51 A1 84 CB B7 00", this = "CCamera", preserve = "edx")]
pub fn process_vector_move_linear(this: GPtr<u8>) {
    process_wo_ratio(this, 0xCBC, 0xCEE, 0xCEC, process_vector_move_linear_call);
}

/// `CCamera::ProcessFOVLerp()` (0x516500): flag +0xCED -> +0xCB5 when over.
#[hook(addr = 0x516500, conv = thiscall, name = "CCamera::ProcessFOVLerp-wo-ratio", expect = "51 A1 84 CB B7 00", this = "CCamera")]
pub fn process_fov_lerp(this: GPtr<u8>) {
    process_wo_ratio(this, 0xCA4, 0xCED, 0xCB5, process_fov_lerp_call);
}

/// The ease weight `(sin((270 - r*180) * 0.0174533) + 1) * 0.5`, kept on the x87 stack.
fn ease_weight(ratio: f32) -> X87 {
    x(ratio)
        .fmul_m(k(0x43340000)) // 180.0 [0x85A994]
        .fsubr_m(k(0x43870000)) // 270.0 [0x859070]
        .fmul_m(k(0x3C8EFA35)) // 0.0174533 [0x8595EC]
        .fsin()
        .fadd_m(k(0x3F800000)) // 1.0 [0x858624]
        .fmul_m(k(0x3F000000)) // 0.5 [0x858B8C]
}

/// Shared body of `ProcessVectorTrackLinear(float)` / `ProcessVectorMoveLinear(float)`:
/// `out = (end - start) * w + start` per component (`fmul st, st(1)`: difference times weight).
fn process_vec_ratio(this: GPtr<u8>, ratio: f32, start: isize, ease: isize, flag: isize) {
    let end = start + 0xC;
    let out = start + 0x1C;
    let e = rd::<u8>(this, ease);
    wr::<u8>(this, flag, 1);
    if e != 0 {
        let w = ease_weight(ratio);
        for i in 0..3 {
            let s = rd::<f32>(this, start + i * 4);
            let d = x(rd::<f32>(this, end + i * 4)).fsub_m(s);
            wr::<f32>(this, out + i * 4, d.fmul(w).fadd_m(s).to_f32());
        }
    } else {
        for i in 0..3 {
            let s = rd::<f32>(this, start + i * 4);
            let d = x(rd::<f32>(this, end + i * 4)).fsub_m(s);
            wr::<f32>(this, out + i * 4, d.fmul_m(ratio).fadd_m(s).to_f32());
        }
    }
}

/// `CCamera::ProcessVectorTrackLinear(float)` (0x50D350): +0xC8C = 1; +0xC80 = lerp(+0xC64, +0xC70).
#[hook(addr = 0x50D350, conv = thiscall, name = "CCamera::ProcessVectorTrackLinear-with-ratio", expect = "8A 81 7C 0C 00 00", this = "CCamera")]
pub fn process_vector_track_linear_ratio(this: GPtr<u8>, ratio: f32) {
    process_vec_ratio(this, ratio, 0xC64, 0xC7C, 0xC8C);
}

/// `CCamera::ProcessVectorMoveLinear(float)` (0x50D430): +0xCEC = 1; +0xCE0 = lerp(+0xCC4, +0xCD0).
#[hook(addr = 0x50D430, conv = thiscall, name = "CCamera::ProcessVectorMoveLinear-with-ratio", expect = "8A 81 DC 0C 00 00", this = "CCamera")]
pub fn process_vector_move_linear_ratio(this: GPtr<u8>, ratio: f32) {
    process_vec_ratio(this, ratio, 0xCC4, 0xCDC, 0xCEC);
}

/// `CCamera::ProcessFOVLerp(float)` (0x50D510): +0xCB5 = 1; +0xCB8 = lerp(+0xCAC, +0xCB0). The eased
/// form multiplies weight times difference (`fmulp st(1), st`), the reverse of the vector lerps.
#[hook(addr = 0x50D510, conv = thiscall, name = "CCamera::ProcessFOVLerp-with-ratio", expect = "8A 81 B4 0C 00 00", this = "CCamera")]
pub fn process_fov_lerp_ratio(this: GPtr<u8>, ratio: f32) {
    let e = rd::<u8>(this, 0xCB4);
    wr::<u8>(this, 0xCB5, 1);
    let s = rd::<f32>(this, 0xCAC);
    let v = if e != 0 {
        let w = ease_weight(ratio);
        let d = x(rd::<f32>(this, 0xCB0)).fsub_m(s);
        w.fmul(d).fadd_m(s)
    } else {
        x(rd::<f32>(this, 0xCB0)).fsub_m(s).fmul_m(ratio).fadd_m(s)
    };
    wr::<f32>(this, 0xCB8, v.to_f32());
}

/// `force - (time - m_nCamShakeStart) * 0.001` clamped: not `< 2.0` (NaN included) gives 2.0, else not
/// `> 0.0` gives 0.0. `scale` (f32) multiplies before the clamp when given.
fn current_shake(cam: GPtr<u8>, scale: Option<f32>) -> X87 {
    let e = TIME_MS.get().wrapping_sub(rd::<u32>(cam, 0x5C));
    let mut cur = fild_u32(e)
        .fmul_m(k(0x3A83126F)) // 0.001 [0x858CDC]
        .fsubr_m(rd::<f32>(cam, 0x12C));
    if let Some(s) = scale {
        cur = cur.fmul_m(s);
    }
    let two = k(0x40000000); // 2.0 [0x858CA0]
    let zero = k(0x00000000); // 0.0 [0x858B50]
    // test ah, 5; jnp: only C0 set (cur < 2.0)
    if jp(cur, x(two), 0x05) {
        x(two)
    } else if !jne(cur, x(zero), 0x41) {
        cur
    } else {
        x(zero)
    }
}

/// `CCamera::CamShakeNoPos(CCamera*, float strength)` (0x50A970, static cdecl): when `strength` exceeds
/// the current (decayed, clamped) shake, `m_fCamShakeForce` (+0x12C) = strength, `m_nCamShakeStart`
/// (+0x5C) = now.
#[hook(addr = 0x50A970, conv = cdecl, name = "CCamera::CamShakeNoPos", expect = "8B 4C 24 04 A1 84 CB B7 00")]
pub fn cam_shake_no_pos(cam: GPtr<u8>, strength: u32) {
    let cur = current_shake(cam, None);
    if jne(x(f32::from_bits(strength)), cur, 0x41) {
        return;
    }
    wr::<u32>(cam, 0x12C, strength);
    wr::<u32>(cam, 0x5C, TIME_MS.get());
}

/// `CCamera::CamShake(float strength, float x, float y, float z)` (0x50A9F0): distance from the active
/// cam's source (+0x19C) as `sqrt(dz_f32^2 + sqrt(dx^2 + dy^2)^2)` (dz stored f32, the 2D length kept
/// on the stack), clamped to [0, 100] (`> 100` first, NaN kept), `scale = (f32)(1.0 - d * 0.01)` (f64
/// constants), the current shake times scale clamped as in `CamShakeNoPos`, then
/// `v = scale * strength * 0.35` (f32 stored, compared unrounded).
#[hook(addr = 0x50A9F0, conv = thiscall, name = "CCamera::CamShake", expect = "83 EC 0C 0F B6 41 59", this = "CCamera")]
pub fn cam_shake(this: GPtr<u8>, strength: f32, px: f32, py: f32, pz: f32) {
    let cam = active_cam(this);
    let dx = x(rd::<f32>(cam, 0x19C)).fsub_m(px);
    let dy = x(rd::<f32>(cam, 0x1A0)).fsub_m(py);
    let dz = x(rd::<f32>(cam, 0x1A4)).fsub_m(pz).to_f32();
    let d2 = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
    let mut d = x(dz).fmul_m(dz).fadd(d2.fmul(d2)).fsqrt();
    let hundred = k(0x42C80000); // 100.0 [0x858628]
    let zero = k(0x00000000); // 0.0 [0x858B50]
    if !jne(d, x(hundred), 0x41) {
        d = x(hundred);
    } else if !jp(d, x(zero), 0x05) {
        d = x(zero);
    }
    let scale = d
        .fmul(X87::from_f64_exact(0.01)) // f64 [0x85A308]
        .fsubr(X87::from_f64_exact(1.0)) // f64 [0x85A310]
        .to_f32();
    let cur = current_shake(this, Some(scale));
    let v = x(scale).fmul_m(strength).fmul_m(k(0x3EB33333)); // 0.35 [0x858F9C]
    if jne(v, cur, 0x41) {
        return;
    }
    wr::<f32>(this, 0x12C, v.to_f32());
    wr::<u32>(this, 0x5C, TIME_MS.get());
}

/// `CCamera::InitialiseScriptableComponents()` (0x50D2D0, entry `jmp 0x4010F8`, which loads
/// EDX = -1.0f and jumps back).
#[hook(addr = 0x50D2D0, conv = thiscall, name = "CCamera::InitialiseScriptableComponents", expect = "E9 23 3E EF FF 33 C0", this = "CCamera")]
pub fn initialise_scriptable_components(this: GPtr<u8>) {
    let m1 = 0xBF80_0000u32;
    for off in [0xC5C, 0xC60, 0xC94, 0xC98, 0xCA8, 0xCA4] {
        wr::<u32>(this, off, m1);
    }
    wr::<u32>(this, 0xCAC, 0);
    wr::<u32>(this, 0xCB0, 0);
    wr::<u8>(this, 0xC7C, 1);
    wr::<u8>(this, 0xCB4, 1);
    wr::<u8>(this, 0xCDC, 1);
    wr::<u32>(this, 0xCBC, m1);
    wr::<u32>(this, 0xCC0, m1);
    for off in [0xCED, 0xCEE, 0xCEF, 0xC8C, 0xCEC, 0xCB5] {
        wr::<u8>(this, off, 0);
    }
}

/// `CCamera::ResetDuckingSystem(CPed*)` (0x50CEF0): both offsets (+0xC54, +0xC58) 0; for a ducking ped
/// `+0xC54 = duck goal` and `+0xC58 = -0.35f`.
#[hook(addr = 0x50CEF0, conv = thiscall, name = "CCamera::ResetDuckingSystem", expect = "56 8B 74 24 08", this = "CCamera")]
pub fn reset_ducking_system(this: GPtr<u8>, ped: GPtr<u8>) {
    wr::<u32>(this, 0xC54, 0);
    wr::<u32>(this, 0xC58, 0);
    if ped.is_null() || !ped_is_ducking(ped) {
        return;
    }
    wr::<f32>(this, 0xC54, duck_goal(ped).to_f32());
    wr::<u32>(this, 0xC58, 0xBEB3_3333);
}

/// Shared tail of the two ducking motion functions: unless `instant`, `off += (goal - off) *
/// (CTimer::ms_fTimeStep * rate)`; then `src.z` and `targ.z` (when non-null) `= off + z`.
fn duck_motion(
    this: GPtr<u8>,
    off: isize,
    goal: X87,
    rate: f32,
    src: GPtr<u8>,
    targ: GPtr<u8>,
    instant: u32,
) {
    if instant as u8 == 0 {
        let cur = rd::<f32>(this, off);
        let t = x(TIME_STEP.get()).fmul_m(rate);
        wr::<f32>(this, off, goal.fsub_m(cur).fmul(t).fadd_m(cur).to_f32());
    }
    for v in [src, targ] {
        if !v.is_null() {
            let z = x(rd::<f32>(this, off)).fadd_m(rd::<f32>(v, 8));
            wr::<f32>(v, 8, z.to_f32());
        }
    }
}

/// `CCamera::HandleCameraMotionForDucking(CPed*, CVector* src, CVector* targ, bool instant)` (0x50CFA0):
/// goal = duck goal for a ducking ped else 0.0 (the original zeroes the ped argument slot and loads it);
/// rate 0.1 [0x858B1C] on +0xC54. `GetTaskDuck(true)` is called twice (first result unused).
#[hook(addr = 0x50CFA0, conv = thiscall, name = "CCamera::HandleCameraMotionForDucking", expect = "56 8B 74 24 08", this = "CCamera")]
pub fn handle_camera_motion_for_ducking(
    this: GPtr<u8>,
    ped: GPtr<u8>,
    src: GPtr<u8>,
    targ: GPtr<u8>,
    instant: u32,
) {
    let _ = get_task_duck(rd(ped, 0x47C), 1);
    let goal = if ped_is_ducking(ped) {
        duck_goal(ped)
    } else {
        x(0.0)
    };
    duck_motion(this, 0xC54, goal, k(0x3DCCCCCD), src, targ, instant);
}

/// `CCamera::HandleCameraMotionForDuckingDuringAim(CPed*, CVector*, CVector*, bool)` (0x50D090): as
/// above with goal -0.35 [0x8630F0] (the speed is computed and compared but unused) and rate 0.13
/// [0x859020] on +0xC58.
#[hook(addr = 0x50D090, conv = thiscall, name = "CCamera::HandleCameraMotionForDuckingDuringAim", expect = "56 8B 74 24 08", this = "CCamera")]
pub fn handle_camera_motion_for_ducking_during_aim(
    this: GPtr<u8>,
    ped: GPtr<u8>,
    src: GPtr<u8>,
    targ: GPtr<u8>,
    instant: u32,
) {
    let _ = get_task_duck(rd(ped, 0x47C), 1);
    let goal = if ped_is_ducking(ped) {
        x(k(0xBEB33333))
    } else {
        x(0.0)
    };
    duck_motion(this, 0xC58, goal, k(0x3E051EB8), src, targ, instant);
}

/// `CCamera::InitCameraVehicleTweaks()` (0x50A3B0): current tweak (+0xD48..+0xD54) = {1, 1, 0, -1};
/// on the first call (+0xD44 clear) the 5-entry table +0xCF4 (stride 0x10: model, 3 floats) is reset and
/// entry 0 = {501, 1.0, 1.0, 0.179}.
#[hook(addr = 0x50A3B0, conv = thiscall, name = "CCamera::InitCameraVehicleTweaks", expect = "53 33 DB B8 00 00 80 3F", this = "CCamera", preserve = "ecx")]
pub fn init_camera_vehicle_tweaks(this: GPtr<u8>) {
    let one = 0x3F80_0000u32;
    wr::<u32>(this, 0xD48, one);
    wr::<u32>(this, 0xD4C, one);
    wr::<u32>(this, 0xD50, 0);
    wr::<i32>(this, 0xD54, -1);
    if rd::<u8>(this, 0xD44) != 0 {
        return;
    }
    for i in 0..5 {
        let e = 0xCF4 + i * 0x10;
        wr::<i32>(this, e, -1);
        wr::<u32>(this, e + 4, one);
        wr::<u32>(this, e + 8, one);
        wr::<u32>(this, e + 0xC, 0);
    }
    wr::<i32>(this, 0xCF4, 0x1F5);
    wr::<u32>(this, 0xCF8, one);
    wr::<u32>(this, 0xCFC, one);
    wr::<u32>(this, 0xD00, 0x3E37_4AFD);
    wr::<u8>(this, 0xD44, 1);
}

/// `CCamera::ApplyVehicleCameraTweaks(CVehicle*)` (0x50A480): when the model (+0x22) differs from
/// +0xD54, re-init, then copy the matching table entry (model >= 0) into the current tweak.
#[hook(addr = 0x50A480, conv = thiscall, name = "CCamera::ApplyVehicleCameraTweaks", expect = "8B 91 54 0D 00 00", this = "CCamera")]
pub fn apply_vehicle_camera_tweaks(this: GPtr<u8>, veh: GPtr<u8>) {
    if i32::from(rd::<i16>(veh, 0x22)) == rd::<i32>(this, 0xD54) {
        return;
    }
    init_camera_vehicle_tweaks_call(this);
    for i in 0..5 {
        let e = 0xCF4 + i * 0x10;
        let m = rd::<i32>(this, e);
        if m >= 0 && m == i32::from(rd::<i16>(veh, 0x22)) {
            wr::<u32>(this, 0xD48, rd::<u32>(this, e + 4));
            wr::<i32>(this, 0xD54, m);
            wr::<u32>(this, 0xD4C, rd::<u32>(this, e + 8));
            wr::<u32>(this, 0xD50, rd::<u32>(this, e + 0xC));
            return;
        }
    }
}

/// `CCamera::Fade(float time, int16 direction)` (0x50AC20, entry `jmp 0x40116D`, which loads
/// DX = direction). See the body for the x87 clamp sequence of the music fade.
/// Not hooked: the protection code reads the entry bytes (`sbb edx, [0x50AC20]` at 0x1566C27).
pub fn fade(this: GPtr<u8>, time_bits: u32, direction: u32) {
    let time = f32::from_bits(time_bits);
    let dir = direction as u16;
    let ignore_music = rd::<u8>(this, 0x25);
    wr::<u32>(this, 0xC04, time_bits);
    wr::<u8>(this, 0x51, 1);
    wr::<u16>(this, 0xC30, dir);
    wr::<u32>(this, 0xC40, TIME_MS.get());
    if ignore_music != 0 && dir != 1 {
        return;
    }
    wr::<u8>(this, 0x52, 1);
    let k03 = k(0x3E99999A); // 0.3 [0x858C24]
    let p = x(time).fmul_m(k03);
    wr::<u16>(this, 0xC3A, dir);
    // q = p unless p <= 0.3 or NaN (then 0.3)
    let q = if jne(p, x(k03), 0x41) { x(k03) } else { p };
    // fcomp [time]; test ah, 5; jp: q >= time or unordered -> time
    let r = if jp(q, x(time), 0x05) {
        x(time)
    } else if !jne(p, x(k03), 0x41) {
        p
    } else {
        x(k03)
    };
    wr::<f32>(this, 0xC08, r.to_f32());
    if dir != 0 {
        wr::<u32>(this, 0xC0C, 0);
        wr::<u32>(this, 0xC44, TIME_MS.get());
        return;
    }
    wr::<f32>(this, 0xC0C, x(time).fsub(r).to_f32());
    let zero = k(0x00000000); // 0.0 [0x858B50]
    let mut v = r.fsub_m(k(0x3DCCCCCD)); // 0.1 [0x858B1C]
    if jne(v, x(zero), 0x41) {
        v = x(zero);
    }
    wr::<f32>(this, 0xC08, v.to_f32());
    wr::<u32>(this, 0xC44, TIME_MS.get());
}

/// `TimeStep * 0.02 [0x858B38] / duration * scale` as the fade steps compute it.
fn fade_step(duration: f32, scale: Option<f32>) -> X87 {
    let s = x(TIME_STEP.get()).fmul_m(k(0x3CA3D70A)).fdiv_m(duration);
    match scale {
        Some(m) => s.fmul_m(m),
        None => s,
    }
}

/// `CCamera::ProcessFade()` (0x50B5D0; the `jmp 0x401A07` at 0x50B5DC loads AX = `m_nFadeInOutFlag`
/// +0xC30). While fading (+0x51 == 1): direction 1 (fade in) lowers `m_fFadeAlpha` (+0xBFC) by
/// `TimeStep * 0.02 / m_fTimeToFadeOut (+0xC04) * 255` (to 0 at once when the time is exactly 0) and
/// stops at <= 0; direction 0 raises it the same way and stops at >= 255. `CDraw::FadeValue =
/// _ftol(alpha)`.
#[hook(addr = 0x50B5D0, conv = thiscall, name = "CCamera::ProcessFade", expect = "80 79 51 01 0F 85 EE 00 00 00", this = "CCamera")]
pub fn process_fade(this: GPtr<u8>) {
    if rd::<u8>(this, 0x51) != 1 {
        return;
    }
    let dir = rd::<u16>(this, 0xC30);
    let zero = k(0x00000000); // 0.0 [0x858B50]
    let k255 = k(0x437F0000); // 255.0 [0x859AAC]
    let dur = rd::<f32>(this, 0xC04);
    // test ah, 0x44; jp not taken: exactly equal
    let instant = !jp(x(dur), x(zero), 0x44);
    if dir == 1 {
        if instant {
            wr::<u32>(this, 0xBFC, 0);
        } else {
            let a = fade_step(dur, Some(k255)).fsubr_m(rd::<f32>(this, 0xBFC));
            wr::<f32>(this, 0xBFC, a.to_f32());
        }
        if !jp(x(rd::<f32>(this, 0xBFC)), x(zero), 0x41) {
            wr::<u8>(this, 0x51, 0);
            wr::<u32>(this, 0xBFC, 0);
        }
    } else if dir == 0 {
        if !jne(x(rd::<f32>(this, 0xBFC)), x(k255), 0x01) {
            wr::<u8>(this, 0x51, 0);
        }
        if instant {
            wr::<f32>(this, 0xBFC, k255);
        } else {
            let a = fade_step(dur, Some(k255)).fadd_m(rd::<f32>(this, 0xBFC));
            wr::<f32>(this, 0xBFC, a.to_f32());
        }
        if !jne(x(rd::<f32>(this, 0xBFC)), x(k255), 0x01) {
            wr::<f32>(this, 0xBFC, k255);
        }
    }
    // `CDraw::FadeValue` (0xC3EFAB) = AL of `_ftol`
    wr::<u8>(
        FADE_RGBV.ptr().cast::<u8>(),
        3,
        ftol32(x(rd::<f32>(this, 0xBFC))) as u8,
    );
}

/// `CCamera::ProcessMusicFade()` (0x50B6D0): while +0x52 is set, waits out `m_fTimeToWaitToFadeMusic`
/// (+0xC0C), then moves `m_fEffectsFaderScalingFactor` (+0xC00) by `TimeStep * 0.02 / +0xC08` up
/// (+0xC3A == 1, stop at >= 1.0) or down (+0xC30 == 0, stop at <= 0.0); then, unless the loading tune
/// plays, both audio fader factors = +0xC00.
#[hook(addr = 0x50B6D0, conv = thiscall, name = "CCamera::ProcessMusicFade", expect = "56 8B F1 8A 46 52", this = "CCamera")]
pub fn process_music_fade(this: GPtr<u8>) {
    if rd::<u8>(this, 0x52) == 0 {
        return;
    }
    let zero = k(0x00000000); // 0.0 [0x858B50]
    let one = k(0x3F800000); // 1.0 [0x858624]
    let wait = rd::<f32>(this, 0xC0C);
    if !jne(x(wait), x(zero), 0x41) {
        let w = x(TIME_STEP.get()).fmul_m(k(0x3CA3D70A)).fsubr_m(wait);
        wr::<f32>(this, 0xC0C, w.to_f32());
    } else if rd::<u16>(this, 0xC3A) == 1 {
        let dur = rd::<f32>(this, 0xC08);
        if !jp(x(dur), x(zero), 0x41) {
            wr::<f32>(this, 0xC00, one);
        } else {
            let v = fade_step(dur, None).fadd_m(rd::<f32>(this, 0xC00));
            wr::<f32>(this, 0xC00, v.to_f32());
        }
        if !jne(x(rd::<f32>(this, 0xC00)), x(one), 0x01) {
            wr::<u8>(this, 0x53, 0);
            wr::<u8>(this, 0x52, 0);
            wr::<f32>(this, 0xC00, one);
        }
    } else if rd::<u16>(this, 0xC30) == 0 {
        if !jp(x(rd::<f32>(this, 0xC00)), x(zero), 0x41) {
            wr::<u8>(this, 0x53, 1);
            wr::<u8>(this, 0x52, 0);
            wr::<u32>(this, 0xC00, 0);
        }
        let dur = rd::<f32>(this, 0xC08);
        if !jp(x(dur), x(zero), 0x41) {
            wr::<u32>(this, 0xC00, 0);
        } else {
            let v = fade_step(dur, None).fsubr_m(rd::<f32>(this, 0xC00));
            wr::<f32>(this, 0xC00, v.to_f32());
        }
        if !jp(x(rd::<f32>(this, 0xC00)), x(zero), 0x41) {
            wr::<u32>(this, 0xC00, 0);
        }
    }
    let ae = GPtr::from_addr(AUDIO_ENGINE);
    if audio_is_loading_tune_active(ae) == 0 {
        audio_set_music_fader_scaling_factor(ae, rd::<u32>(this, 0xC00));
        audio_set_effects_fader_scaling_factor(ae, rd::<u32>(this, 0xC00));
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_VECTOR_TRACK_RUNNING,
    HOOK_VECTOR_MOVE_RUNNING,
    HOOK_SET_FADE_COLOUR,
    HOOK_ADD_SHAKE_SIMPLE,
    HOOK_SET_CAM_COLLISION_VAR_DATA_SET,
    HOOK_SET_COL_VARS_AIM_WEAPON,
    HOOK_SET_COL_VARS_PED,
    HOOK_SET_COL_VARS_VEHICLE,
    HOOK_PROCESS_SHAKE,
    HOOK_LERP_FOV,
    HOOK_CONSIDER_PED_AS_DUCKING,
    HOOK_SET_NEAR_CLIP_BASED_ON_PED_COLLISION,
    HOOK_VECTOR_MOVE_LINEAR,
    HOOK_VECTOR_TRACK_LINEAR,
    HOOK_PROCESS_VECTOR_TRACK_LINEAR,
    HOOK_PROCESS_VECTOR_MOVE_LINEAR,
    HOOK_PROCESS_FOV_LERP,
    HOOK_PROCESS_FOV_LERP_RATIO,
    HOOK_PROCESS_VECTOR_TRACK_LINEAR_RATIO,
    HOOK_PROCESS_VECTOR_MOVE_LINEAR_RATIO,
    HOOK_CAM_SHAKE_NO_POS,
    HOOK_CAM_SHAKE,
    HOOK_INITIALISE_SCRIPTABLE_COMPONENTS,
    HOOK_RESET_DUCKING_SYSTEM,
    HOOK_HANDLE_CAMERA_MOTION_FOR_DUCKING,
    HOOK_HANDLE_CAMERA_MOTION_FOR_DUCKING_DURING_AIM,
    HOOK_INIT_CAMERA_VEHICLE_TWEAKS,
    HOOK_APPLY_VEHICLE_CAMERA_TWEAKS,
    HOOK_PROCESS_FADE,
    HOOK_PROCESS_MUSIC_FADE,
];
