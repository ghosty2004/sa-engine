//! PORT-CAM-3 (helper A): `CCam::Process_FollowPed_SA` (0x522D40), the third-person camera behind the
//! player on foot. Ported from the 1.0 US bytes (gta-reversed has only a `NOTSA_UNREACHABLE()` stub;
//! `docs/re/specs/CAM/CCam.md` 0x522D40 as reading aid; where it differs, the bytes win).
//!
//! The port is a literal transcription: the locals live in a frame array addressed by the original's
//! `B`-relative offsets (B = ESP after the four register pushes), so slot reuse is kept (the zoom row
//! `B+0x48` becomes `1 - p`, the speed `B+0x38` becomes a clamp temporary, `B+0x44` holds the
//! stick values and the player pointer, `B+0x50` the cosine of the output pitch), and the x87 stack runs
//! on the [`Fpu`] machine instruction by instruction (which values are stored as f32 and which stay on
//! the stack, operand order, compare masks). FSIN / FCOS / FPATAN use sa-math's 64-bit models; `_CIasin`
//! and `_CIpow` run the original CRT code (`orig::x87_unary` / `x87_binary`), as in `CCam::Process`.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::camcol::{get_task_jet_pack, get_task_swim, get_using_parachute, vector_magnitude, vector_sub};
use crate::camproc::{
    aim_weapon_left_right, aim_weapon_up_down, find_player_ped, get_pad, get_task_climb,
    get_vectors_ready_for_rw, max_f, round_decimals_call, vec_add, vec_mul_s, vec_s_mul, vector_normalise,
};
use crate::common::{Fpu, g, k, rd, the_camera, wr};

/// CRT `_CIasin` (0x821E70) and `_CIpow` (0x822130): operands and result on the x87 stack.
pub(crate) const CI_ASIN: usize = 0x821E70;
pub(crate) const CI_POW: usize = 0x822130;

orig! {
    /// `CPed::IsPlayer()` (0x5DF8F0, AL)
    pub fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00), returns `this`
    pub fn vector_add_assign(v: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> = thiscall 0x411A00 => unported;
    /// `CCamera::ResetDuckingSystem(CPed*)` (0x50CEF0)
    pub fn reset_ducking_system(cam: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x50CEF0 => crate::camfx::reset_ducking_system;
    /// `DotProduct(const CVector*, const CVector*)` (0x40FDB0, ST0)
    pub fn dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `CPad::ClearMouseHistory()` (0x541BD0)
    pub fn pad_clear_mouse_history(pad: GPtr<u8>) = thiscall 0x541BD0 => unported;
    /// `CPad::GetForceCameraBehindPlayer()` (0x540AE0, AL)
    pub fn pad_get_force_camera_behind_player(pad: GPtr<u8>) -> u8 = thiscall 0x540AE0 => unported;
    /// `CPedIntelligence::GetTaskUseGun()` (0x600F70)
    pub fn get_task_use_gun(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F70 => unported;
    /// `CPad::GetPedWalkLeftRight()` (0x53FC90, AX) / `GetPedWalkUpDown()` (0x53FD30, AX)
    pub fn pad_get_ped_walk_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FC90 => unported;
    pub fn pad_get_ped_walk_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FD30 => unported;
    /// `CPlaceable::GetTopDirection(CVector* out)` (0x41CCB0)
    pub fn placeable_get_top_direction(p: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x41CCB0 => unported;
    /// `IKChainManager_c::LookAt(const char*, CPed*, CEntity*, int32 time, ePedBones, CVector* pos,
    /// bool torso, float speed, int32 blend, int32 priority, bool force)` (0x618970); the float as a dword
    pub fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: GPtr<u8>, torso: u32, speed: u32, blend: i32, prio: i32, force: u32) -> u8 = thiscall 0x618970 => unported;
    /// `CTaskSimpleClimb::GetCameraStickModifier(CEntity*, float& alpha, float& beta, float& stickAlpha,
    /// float& stickBeta)` (0x67A5D0)
    pub fn climb_get_camera_stick_modifier(t: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, sa: GPtr<u8>, sb: GPtr<u8>) = thiscall 0x67A5D0 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720)
    pub fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `FUN_0063a380` (the enter-car stick modifier, `this` = CTaskComplexEnterCarAsDriver)
    pub fn enter_car_stick_modifier(t: GPtr<u8>, ped: GPtr<u8>, dist: f32, a: GPtr<f32>, b: GPtr<f32>, sa: GPtr<f32>, sb: GPtr<f32>) = thiscall 0x63A380 => crate::camproc::enter_car_camera_stick_modifier;
    /// `CCam::ClipBeta()` (0x509C50)
    pub fn clip_beta(c: GPtr<u8>) = thiscall 0x509C50 => crate::cam::clip_beta;
    /// `CCamera::GetFadingDirection()` (0x50ADF0)
    pub fn get_fading_direction(cam: GPtr<u8>) -> i32 = thiscall 0x50ADF0 => crate::camera::get_fading_direction;
    /// `CCamera::HandleCameraMotionForDucking(CPed*, CVector*, CVector*, bool)` (0x50CFA0)
    pub fn handle_camera_motion_for_ducking(cam: GPtr<u8>, ped: GPtr<u8>, src: GPtr<u8>, targ: GPtr<u8>, instant: u32) = thiscall 0x50CFA0 => crate::camfx::handle_camera_motion_for_ducking;
    /// `CCamera::SetColVarsPed(int32 type, int32 zoom)` (0x50CC50, cdecl)
    pub fn set_col_vars_ped(ty: i32, zoom: i32) = cdecl 0x50CC50 => crate::camfx::set_col_vars_ped;
    /// `CCamera::CameraGenericModeSpecialCases(CPed*)` (0x50CD30)
    pub fn camera_generic_mode_special_cases(cam: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x50CD30 => crate::camera::camera_generic_mode_special_cases;
    /// `CCamera::CameraColDetAndReact(CVector* source, const CVector* target)` (0x520190, stdcall, AL;
    /// ported in camcol but not hooked)
    pub fn camera_col_det_and_react(source: GPtr<u8>, target: GPtr<u8>) -> u8 = stdcall 0x520190 => crate::camcol::camera_col_det_and_react;
    /// `CCamera::ImproveNearClip(CVehicle*, CPed*, CVector*, CVector*)` (0x516B20)
    pub fn improve_near_clip(cam: GPtr<u8>, veh: GPtr<u8>, ped: GPtr<u8>, source: GPtr<u8>, target: GPtr<u8>) = thiscall 0x516B20 => crate::camcol::improve_near_clip;
    /// `FUN_0050a120(CVector*, int32)` (0x50A120): each component rounded to n decimals
    pub fn round_vector_decimals(v: GPtr<u8>, n: i32) = cdecl 0x50A120 => crate::camproc::round_vector_decimals;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390, AL)
    pub fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CVector::SquaredMagnitude()` (0x406DA0, ST0 = `(x*x + y*y) + z*z`)
    pub fn magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CIdleCam::Process()` (0x522C80, AL)
    pub fn idle_cam_process(ic: GPtr<u8>) -> u8 = thiscall 0x522C80 => crate::camproc::idle_cam_process;
}

/// `CGame::currArea` (0xB72914).
const CURR_AREA: usize = 0xB72914;
/// `TheCamera` fields: `m_nPedZoom` (+0xC8), `m_fPedZoomBase` (+0xCC), `m_fPedZoomTotal` (+0xD0),
/// `m_fPedZoomSmoothed` (+0xD4), `m_bTransitionState` (+0x58), `m_bCamDirectlyBehind` (+0x1A),
/// `m_bCamDirectlyInFront` (+0x1B), `m_bJustCameOutOfGarage` (+0x28), `m_bFading` (+0x51),
/// `m_nWhoIsInControlOfTheCamera` (+0xB0), `m_bFOVLerpProcessed` (+0xCB5),
/// `m_bVecMoveLinearProcessed` (+0xCEC), `m_bVecTrackLinearProcessed` (+0xC8C).
const PED_ZOOM: usize = 0xB6F0F0;
const PED_ZOOM_BASE: usize = 0xB6F0F4;
const PED_ZOOM_TOTAL: usize = 0xB6F0F8;
const PED_ZOOM_SMOOTHED: usize = 0xB6F0FC;
const TRANSITION_STATE: usize = 0xB6F080;
const CAM_DIRECTLY_BEHIND: usize = 0xB6F042;
const CAM_DIRECTLY_IN_FRONT: usize = 0xB6F043;
const JUST_CAME_OUT_OF_GARAGE: usize = 0xB6F050;
const FADING: usize = 0xB6F079;
const WHO_IS_IN_CONTROL: usize = 0xB6F0D8;
const FOV_LERP_PROCESSED: usize = 0xB6FCDD;
const VEC_MOVE_LINEAR_PROCESSED: usize = 0xB6FD14;
const VEC_TRACK_LINEAR_PROCESSED: usize = 0xB6FCB4;
/// `CDraw::FadeValue` (u8, 0xC3EFAB)
const FADE_VALUE: usize = 0xC3EFAB;
/// The previous distance (0xB6EC50) and the force-behind latch (u8 0xB6EC54).
const PREV_Z: usize = 0xB6EC50;
const FORCE_BEHIND: usize = 0xB6EC54;
/// `CCam::m_bUseMouse3rdPerson` (static, 0xB6EC2E), `CCamera::m_fMouseAccelHorzntl` (0xB6EC1C).
const USE_MOUSE_3RD_PERSON: usize = 0xB6EC2E;
const MOUSE_ACCEL_HORZNTL: usize = 0xB6EC1C;
/// `CPad::NewMouseControllerState` x / y (0xB73424 / 0xB73428).
const MOUSE_X: usize = 0xB73424;
const MOUSE_Y: usize = 0xB73428;
/// `CTimer::ms_fTimeStep`, `m_snTimeInMilliseconds`.
const TIME_STEP: usize = 0xB7CB5C;
const TIME_MS: usize = 0xB7CB84;
/// The ped position statics `CCam::Process` smooths (0x8CCC3C) and its velocity (0xB6EC7C).
const S_PED_POS: usize = 0x8CCC3C;
const S_PED_VEL: usize = 0xB6EC7C;
/// The previous alpha / beta of the jitter filter (0x8CCE74 / 0x8CCE6C) and their thresholds.
const LAST_ALPHA: usize = 0x8CCE74;
const LAST_BETA: usize = 0x8CCE6C;
/// `CCollision::bCamCollideWithVehicles` / `...Objects` / `...Peds` (0x8A5B14 / 0x8A5B15 / 0x8A5B17).
const CAM_COLLIDE_VEHICLES: usize = 0x8A5B14;
const CAM_COLLIDE_OBJECTS: usize = 0x8A5B15;
const CAM_COLLIDE_PEDS: usize = 0x8A5B17;
/// `gCameraDirection` (0x8CC384), the idle counter (0xB6FE34), `gIdleCam` (0xB6FDA0),
/// `g_ikChainMan` (0xC15448) and its "FollowPedSA" purpose string (0x863238).
const G_CAMERA_DIRECTION: usize = 0x8CC384;
const IDLE_COUNTER: usize = 0xB6FE34;
const G_IDLE_CAM: usize = 0xB6FDA0;
const IK_CHAIN_MAN: usize = 0xC15448;
const IK_PURPOSE: u32 = 0x863238;
/// The ped camera table `0x8CC548 + interior * 0x3C` (15 f32 per row, `.data`).
const PED_CAM_TABLE: usize = 0x8CC548;

/// `.rdata` constants (bits; the address in the comment).
const K_1: u32 = 0x3F80_0000; // 1.0 [0x858624], [0x862F1C]
const K_0: u32 = 0; // 0.0 [0x858B50]
const K_M1: u32 = 0xBF80_0000; // -1.0 [0x858C1C]
const K_0_1: u32 = 0x3DCC_CCCD; // 0.1 [0x858B1C]
const K_0_3: u32 = 0x3E99_999A; // 0.3 [0x858C24]
const K_0_05: u32 = 0x3D4C_CCCD; // 0.05 [0x858C28]
const K_0_01: u32 = 0x3C23_D70A; // 0.01 [0x858C58]
const K_5: u32 = 0x40A0_0000; // 5.0 [0x858C80]
const K_PI: u32 = 0x4049_0FDB; // [0x858CB8]
const K_2PI: u32 = 0x40C9_0FDB; // [0x858CBC]
const K_MPI: u32 = 0xC049_0FDB; // [0x858CC0]
const K_0_001: u32 = 0x3A83_126F; // [0x858CDC]
const K_70: u32 = 0x428C_0000; // [0x858CE0]
const K_1_2: u32 = 0x3F99_999A; // [0x858F08]
const K_0_002: u32 = 0x3B03_126F; // [0x858F44]
const K_2_5: u32 = 0x4020_0000; // [0x858FA0]
const K_0_0001: u32 = 0x38D1_B717; // [0x858FC4]
const K_HALF_PI: u32 = 0x3FC9_0FDB; // [0x858FE4]
const K_1_14: u32 = 0x3D92_4925; // 1/14 [0x859B50]
const K_0_0125: u32 = 0x3C4C_CCCD; // [0x85A5C4]
const K_0_5: u32 = 0x3F00_0000; // [0x862F34]
const K_SWIM_PITCH: u32 = 0xBE86_0A92; // -0.2617994 [0x862F38]
const K_3: u32 = 0x4040_0000; // [0x862F3C]
const K_JETPACK_PITCH: u32 = 0xBEB2_B8C3; // -0.34906587 [0x862F40]
const K_3_70: u32 = 0x3D2F_8AF9; // 3/70 [0x8631AC]
const K_M2_5: u32 = 0xC020_0000; // [0x8631B0]

/// A `.data` f32 read by address, as the bytes do (tweakable values).
pub(crate) fn d(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// `fsin` / `fcos` / `fpatan` on the machine (sa-math's f64 models).
pub(crate) fn fsin(f: &mut Fpu) {
    let v = f.pop().fsin();
    f.push(v);
}
pub(crate) fn fcos(f: &mut Fpu) {
    let v = f.pop().fcos();
    f.push(v);
}
/// `fpatan`: `st1 = atan2(st1, st0)`, pop.
pub(crate) fn fpatan(f: &mut Fpu) {
    let xv = f.pop();
    let yv = f.pop();
    f.push(yv.fpatan(xv));
}
/// `call _CIasin` (operand and result in ST0).
pub(crate) fn ci_asin(f: &mut Fpu) {
    let v = f.pop();
    f.push(X87::from_f64_exact(sa_core::orig::x87_unary(CI_ASIN, v.to_f64())));
}
/// `call _CIpow` (`st1 ^ st0`, both popped, result pushed).
pub(crate) fn ci_pow(f: &mut Fpu) {
    let e = f.pop();
    let base = f.pop();
    f.push(X87::from_f64_exact(sa_core::orig::x87_binary(
        CI_POW,
        base.to_f64(),
        e.to_f64(),
    )));
}
/// An ST0 result of a call.
pub(crate) fn push_st0(f: &mut Fpu, v: f64) {
    f.push(X87::from_f64_exact(v));
}

/// `fld [v+0x8]; fld [v+4]; fld [v]; ...; fcomp m` of `(x*x + y*y) + z*z` against `m` (the inlined
/// squared magnitude of the ped's move speed), then the three pops; returns AH's compare.
fn speed_sq_cmp(f: &mut Fpu, v: GPtr<u8>, m: f32) {
    f.fld(rd(v, 8));
    f.fld(rd(v, 4));
    f.fld(rd(v, 0));
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fcomp_m(m);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fstp_st(0);
}

/// The clamp of `DotProduct(&ped->+0x578, &matrix->forward)` to [-1, 1] through the local `B+slot`,
/// then `_CIasin` (ST0 left on the stack; the caller negates it).
pub(crate) fn clamped_asin_of_dot(f: &mut Fpu, b: GPtr<u8>, slot: isize, ped: GPtr<u8>) {
    let fwd = rd::<GPtr<u8>>(ped, 0x14).byte_add(0x10);
    push_st0(f, dot_product(ped.byte_add(0x578), fwd));
    f.fld(k(K_M1));
    f.fcomp_st(1);
    if !f.jne(0x41) {
        f.fstp_st(0);
        wr::<u32>(b, slot, K_M1);
    } else {
        f.fld(k(K_1));
        f.fcomp_st(1);
        if f.jp(0x5) {
            wr::<f32>(b, slot, f.fstp());
        } else {
            f.fstp_st(0);
            wr::<u32>(b, slot, K_1);
        }
    }
    f.fld(rd(b, slot));
    ci_asin(f);
}

/// `fcom m; test ah, 0x41; jne` / `fld lim; fchs; fstp tmp; fcom tmp; test ah, 5; jp` clamp of ST0 to
/// `[-lim, lim]` (the bytes' common form: `lim` at `B+lim_off`, the negated limit stored at `B+tmp_off`).
pub(crate) fn clamp_st0(f: &mut Fpu, b: GPtr<u8>, lim_off: isize, tmp_off: isize) {
    f.fcom_m(rd(b, lim_off));
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fld(rd(b, lim_off));
        return;
    }
    f.fld(rd(b, lim_off));
    f.fchs();
    wr::<f32>(b, tmp_off, f.fstp());
    f.fcom_m(rd(b, tmp_off));
    if !f.jp(0x5) {
        f.fstp_st(0);
        f.fld(rd(b, tmp_off));
    }
}

/// `fld [e+0x10]; fchs; fld [e+0x14]; fpatan` with the matrix, else `fld [e+0x10]` (the placement
/// heading): the inlined `CPlaceable::GetHeading`.
pub(crate) fn heading(f: &mut Fpu, e: GPtr<u8>) {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if !m.is_null() {
        f.fld(rd(m, 0x10));
        f.fchs();
        f.fld(rd(m, 0x14));
        fpatan(f);
    } else {
        f.fld(rd(e, 0x10));
    }
}

/// The entity position (`matrix + 0x30`, else the placement `+4`).
pub(crate) fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// Dword copy of 3 words.
pub(crate) fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for o in [0, 4, 8] {
        wr::<u32>(dst, o, rd::<u32>(src, o));
    }
}

/// `source.x/y/z = FUN_0050a0a0(source.x/y/z, 4)` (each pushed as a dword, stored with `fstp dword`).
fn round_source(src: GPtr<u8>) {
    for o in [0, 4, 8] {
        let r = round_decimals_call(rd::<u32>(src, o), 4);
        let mut f = Fpu::new();
        push_st0(&mut f, r);
        wr::<f32>(src, o, f.fstp());
    }
}

/// `CCam::Process_FollowPed_SA(const CVector& target, float, float, float, bool resetFromScript)`
/// (0x522D40, thiscall, `ret 0x14`). See `docs/re/specs/CAM/CCam.md` (0x522D40) for the block
/// structure; the order of calls, re-reads and x87 operations follows the bytes. Details the bytes
/// have and the spec simplifies:
/// - the entry `jmp 0x406AFC` (`.text` protection chunk) loads `ecx = this->m_pCamTargetEntity`;
/// - the IK look-at runs when the aim stick is **not** centred (`UD != 0 || LR != 0`) and both walk
///   axes are 0 (`test ah, 0x44; jp` on UD, then `jnp` on LR);
/// - the turn rate's speed `w` slot (`B+0x38`) and the stick slot `B+0x44` are reused as temporaries,
///   the zoom row `B+0x48` becomes `1 - pow(C8, ts)` before the alpha update reads it;
/// - `alphaSpeed = (1-p)*alphaSpeed + UD*p` (p / q swapped against beta);
/// - the mouse pitch uses `m_fMouseAccelHorzntl`, and the auto-pitch target is `-pitchBase`;
/// - the idle camera test is `(x*x + y*y) + z*z` of the move speed against 0.0001 with
///   `test ah, 0x41; jp` (NaN skips it).
///
/// PORT-CAM-3 left it unhooked: with sa-math's former 53-bit FPATAN, car-ai diverged at frame 4981 on the
/// beta at cam+0xBC / `0x8CCE6C` (orig 0xA6F72C00 = -1.7e-15, Rust 0xA6E00000): `atan2(-front.x,
/// front.y) - pi/2` cancels, so bits of FPATAN's 64-bit result reach the f32. X87-2 replaced the model
/// with the 64-bit FPATAN / FSIN / FCOS models (`sa_math::x87`).
#[hook(addr = 0x522D40, conv = thiscall, name = "CCam::Process_FollowPed_SA", expect = "83 EC 7C 57 8B F9", this = "CCam")]
pub fn process_follow_ped_sa(
    this: GPtr<u8>,
    target: GPtr<u8>,
    _orientation: u32,
    _speed_var: u32,
    _speed_var_wanted: u32,
    reset_from_script: u32,
) {
    let mut fr = [0u32; 0x24];
    let b = GPtr::<u32>::from_raw(fr.as_mut_ptr()).cast::<u8>();
    let mut f = Fpu::new();
    let ts = || d(TIME_STEP);
    let row = || rd::<u32>(b, 0x48) as usize;
    let tbl = |r: usize, i: usize| d(PED_CAM_TABLE + r + 4 * i);
    let arg5 = reset_from_script as u8;

    // 0x522D4C: target type and IsPlayer
    let t0 = rd::<GPtr<u8>>(this, 0x21C);
    if rd::<u8>(t0, 0x36) & 7 != 3 || ped_is_player(t0) == 0 {
        return;
    }
    let ped = rd::<GPtr<u8>>(this, 0x21C);
    let intel = || rd::<GPtr<u8>>(ped, 0x47C);
    wr(b, 0x2C, get_pad(0));
    if rd::<i32>(ped, 0x598) == 1 {
        wr(b, 0x2C, get_pad(1));
    }
    copy3(b.byte_add(0x5C), target);
    let interior = u32::from(g::<u32>(CURR_AREA).read() != 0);
    wr::<u32>(b, 0x54, interior);
    let r0 = (interior * 0x3C) as usize;
    wr::<u32>(b, 0x48, r0 as u32);
    wr::<f32>(b, 0x4C, tbl(r0, 1));
    if rd::<u8>(ped, 0x46C) & 1 == 0 && g::<i32>(PED_ZOOM).read() == 3 && get_using_parachute(intel()) != 0 {
        f.fld(rd(b, 0x4C));
        f.fadd_st(0);
        wr::<f32>(b, 0x4C, f.fstp());
    }
    // 0x522E10: distance z = smoothed zoom + C1
    let zoom = g::<i32>(PED_ZOOM).read();
    f.fld(d(PED_ZOOM_SMOOTHED));
    f.fadd_m(rd(b, 0x4C));
    wr::<f32>(b, 0x34, tbl(r0, 3));
    f.fcom_m(d(PREV_Z));
    wr::<f32>(b, 0x4C, tbl(r0, 13));
    wr::<f32>(b, 0x3C, tbl(r0, 4));
    wr::<f32>(b, 0x1C, f.fst());
    wr::<f32>(b, 0x50, tbl(r0, 14));
    if !f.jne(0x41) {
        wr::<f32>(b, 0x34, f.fst());
    }
    g::<f32>(PREV_Z).write(f.fstp());
    wr::<f32>(b, 0x28, tbl(r0, 2));
    let extra = match zoom {
        1 => Some(0xE8),
        2 => Some(if interior == 1 { 0xF0 } else { 0xEC }),
        3 => Some(0xF4),
        _ => None,
    };
    if let Some(o) = extra {
        f.fld(rd(b, 0x28));
        f.fadd_m(rd(this, o));
        wr::<f32>(b, 0x28, f.fstp());
    }
    // 0x522EA9: swim / jetpack weights w (B+0x38) and w2 (B+0x44)
    wr::<u32>(b, 0x44, 0);
    wr::<u32>(b, 0x38, 0);
    if !get_task_swim(intel()).is_null() {
        wr::<u32>(b, 0x38, K_1);
        if rd::<u16>(get_task_swim(intel()), 0xA) != 4 {
            wr::<u32>(b, 0x44, K_0_5);
        }
    } else if !get_task_jet_pack(intel()).is_null() {
        wr::<u32>(b, 0x38, 0x3F00_0000);
        if rd::<u8>(ped, 0x46C) & 1 == 0 {
            wr::<u32>(b, 0x44, K_3);
        }
    }
    // 0x522F12: FOV toward 70
    if g::<u8>(TRANSITION_STATE).read() == 0 {
        if rd::<u8>(this, 0xA) != 0 {
            wr::<u32>(this, 0xB4, K_70);
        } else {
            f.fld(ts());
            f.fmul_m(k(K_1));
            wr::<f32>(b, 0x40, f.fst());
            f.fadd_m(rd(this, 0xB4));
            f.fld(k(K_70));
            f.fcomp_st(1);
            if !f.jne(0x41) {
                wr::<f32>(this, 0xB4, f.fstp());
            } else {
                f.fstp_st(0);
                f.fld(rd(this, 0xB4));
                f.fsub_m(rd(b, 0x40));
                f.fld(k(K_70));
                f.fcomp_st(1);
                if !f.jp(0x5) {
                    wr::<f32>(this, 0xB4, f.fstp());
                } else {
                    f.fstp_st(0);
                    wr::<u32>(this, 0xB4, K_70);
                }
            }
        }
    }
    // 0x522F7E: T.z += C0; Lmin = max-ish(z, C4)
    f.fld(rd(b, 0x64));
    f.fadd_m(tbl(row(), 0));
    wr::<f32>(b, 0x64, f.fstp());
    f.fld(rd(b, 0x1C));
    f.fcomp_m(rd(b, 0x3C));
    if !f.jne(0x41) {
        wr::<u32>(b, 0x40, rd::<u32>(b, 0x1C));
    } else {
        wr::<u32>(b, 0x40, rd::<u32>(b, 0x3C));
    }
    let reset = rd::<u8>(this, 0xA) != 0
        || g::<u8>(CAM_DIRECTLY_BEHIND).read() != 0
        || g::<u8>(CAM_DIRECTLY_IN_FRONT).read() != 0
        || arg5 != 0;
    if !reset {
        train_follow(this, b, ped, &mut f);
    } else {
        reset_block(this, b, ped, arg5, &mut f);
    }
    main_block(this, b, ped, &mut f);
}

/// 0x522FE1: a ped standing on a train (sub-type 6), or on something attached to one, drags the target
/// history along: `k = max(0, |mv| - 0.01) / max(0.01, |mv|)`, `hist[i] += ts * (k * mv)`.
fn train_follow(this: GPtr<u8>, b: GPtr<u8>, ped: GPtr<u8>, f: &mut Fpu) {
    let e = rd::<GPtr<u8>>(ped, 0x568);
    if e.is_null() {
        return;
    }
    let is_train = |v: GPtr<u8>| rd::<u8>(v, 0x36) & 7 == 2 && rd::<i32>(v, 0x594) == 6;
    if !is_train(e) {
        let a = rd::<GPtr<u8>>(e, 0xFC);
        if a.is_null() || !is_train(a) {
            return;
        }
    }
    let mv = e.byte_add(0x44);
    push_st0(f, vector_magnitude(mv));
    wr::<f32>(b, 0x3C, f.fst());
    f.fsub_m(d(0x8CCEA0));
    let a = f.fstp().to_bits();
    push_st0(f, max_f(0, a));
    wr::<f32>(b, 0x20, f.fstp());
    push_st0(f, max_f(g::<u32>(0x8CCEA0).read(), rd::<u32>(b, 0x3C)));
    f.fdivr_m(rd(b, 0x20));
    let tsb = g::<u32>(TIME_STEP).read();
    wr::<f32>(b, 0x3C, f.fstp());
    let kk = rd::<u32>(b, 0x3C);
    let o = vec_s_mul(b.byte_add(0x74), kk, mv);
    let o = vec_mul_s(b.byte_add(0x68), o, tsb);
    vector_add_assign(this.byte_add(0x1D8), o);
    let tsb = g::<u32>(TIME_STEP).read();
    let mv = rd::<GPtr<u8>>(ped, 0x568).byte_add(0x44);
    let o = vec_s_mul(b.byte_add(0x74), kk, mv);
    let o = vec_mul_s(b.byte_add(0x80), o, tsb);
    vector_add_assign(this.byte_add(0x1E4), o);
}

/// 0x5230E4 .. 0x523421: the reset (statics reset, directly behind / in front, script reset).
fn reset_block(this: GPtr<u8>, b: GPtr<u8>, ped: GPtr<u8>, arg5: u8, f: &mut Fpu) {
    let intel = || rd::<GPtr<u8>>(ped, 0x47C);
    if arg5 != 0 {
        copy3(g::<u8>(S_PED_POS), entity_pos(ped));
        for o in [0x68, 0x6C, 0x70] {
            wr::<u32>(b, o, 0);
        }
        copy3(g::<u8>(S_PED_VEL), b.byte_add(0x68));
        copy3(b.byte_add(0x5C), entity_pos(ped));
        f.fld(rd(b, 0x64));
        f.fadd_m(d(PED_CAM_TABLE + rd::<u32>(b, 0x48) as usize));
        wr::<f32>(b, 0x64, f.fstp());
    }
    reset_ducking_system(the_camera(), ped);
    wr::<u8>(this, 0xB, 0);
    g::<u8>(FORCE_BEHIND).write(0);
    wr::<u8>(this, 0x3, 1);
    let keep_angles = || g::<u8>(JUST_CAME_OUT_OF_GARAGE).read() != 0 || arg5 != 0;
    if !keep_angles() {
        heading(f, ped);
        f.fsub_m(k(K_HALF_PI));
        wr::<f32>(this, 0xBC, f.fst());
        if g::<u8>(CAM_DIRECTLY_IN_FRONT).read() != 0 {
            f.fadd_m(k(K_PI));
            wr::<f32>(this, 0xBC, f.fstp());
        } else {
            f.fstp_st(0);
        }
    }
    // 0x5231ED
    f.fld(rd(b, 0x1C));
    wr::<u32>(this, 0xC0, 0);
    wr::<u32>(this, 0xB0, 0);
    wr::<u32>(this, 0xC4, 0x447A_0000);
    f.fcomp_m(d(PED_ZOOM_BASE));
    if !f.jp(0x44) {
        let v = g::<u32>(PED_ZOOM_TOTAL).read();
        g::<u32>(PED_ZOOM_SMOOTHED).write(v);
        wr::<u32>(b, 0x1C, v);
    }
    if !keep_angles() {
        wr::<u32>(this, 0xAC, 0);
        if rd::<u8>(ped, 0x46C) & 1 != 0 {
            clamped_asin_of_dot(f, b, 0x3C, ped);
            f.fchs();
            wr::<f32>(this, 0xAC, f.fstp());
        }
    }
    // 0x5232A3: front from alpha / beta, the two history points
    let front = this.byte_add(0x190);
    f.fld(rd(this, 0xAC));
    fcos(f);
    f.fld(rd(this, 0xBC));
    fcos(f);
    f.fmul_st(1);
    f.fchs();
    wr::<f32>(b, 0x68, f.fstp());
    wr::<u32>(front, 0, rd::<u32>(b, 0x68));
    f.fld(rd(this, 0xBC));
    fsin(f);
    f.fmul_st(1);
    f.fchs();
    wr::<f32>(b, 0x6C, f.fstp());
    wr::<u32>(front, 4, rd::<u32>(b, 0x6C));
    f.fstp_st(0);
    f.fld(rd(this, 0xAC));
    fsin(f);
    wr::<f32>(b, 0x70, f.fstp());
    f.fld(rd(b, 0x40));
    wr::<u32>(front, 8, rd::<u32>(b, 0x70));
    hist_point(this, b, front, 0x40, 0x1D8, f, true);
    wr::<u32>(this, 0x208, g::<u32>(TIME_MS).read());
    hist_point(this, b, front, 0x1C, 0x1E4, f, false);
    wr::<u32>(this, 0x218, 0);
    if !keep_angles() {
        f.fld(rd(b, 0x28));
        f.fchs();
        wr::<f32>(this, 0xAC, f.fstp());
    }
    let swim = !get_task_swim(intel()).is_null() && rd::<u16>(get_task_swim(intel()), 0xA) != 4;
    let add = if swim {
        Some(K_SWIM_PITCH)
    } else if !get_task_jet_pack(intel()).is_null() {
        Some(K_JETPACK_PITCH)
    } else {
        None
    };
    if let Some(c) = add {
        f.fld(k(c));
        f.fadd_m(rd(this, 0xAC));
        wr::<f32>(this, 0xAC, f.fstp());
    }
    pad_clear_mouse_history(get_pad(0));
}

/// The reset's history point `hist = T - front * L` (L at `B+l_off`; `first`: the `fld L` was already
/// issued by the caller). Products x / y stay on the stack, z goes through `B+0x7C`; the differences
/// through `B+0x68..0x70`.
fn hist_point(
    this: GPtr<u8>,
    b: GPtr<u8>,
    front: GPtr<u8>,
    l_off: isize,
    h_off: isize,
    f: &mut Fpu,
    first: bool,
) {
    if !first {
        f.fld(rd(b, l_off));
    }
    f.fmul_m(rd(front, 0));
    f.fld(rd(b, l_off));
    f.fmul_m(rd(front, 4));
    f.fld(rd(b, l_off));
    f.fmul_m(rd(front, 8));
    wr::<f32>(b, 0x7C, f.fstp());
    f.fld(rd(b, 0x5C));
    f.fsub_st(2);
    wr::<f32>(b, 0x68, f.fstp());
    f.fld(rd(b, 0x60));
    f.fsub_st(1);
    wr::<f32>(b, 0x6C, f.fstp());
    f.fstp_st(0);
    f.fstp_st(0);
    f.fld(rd(b, 0x64));
    f.fsub_m(rd(b, 0x7C));
    wr::<f32>(b, 0x70, f.fstp());
    copy3(this.byte_add(h_off), b.byte_add(0x68));
}

/// 0x523426 to the end.
fn main_block(this: GPtr<u8>, b: GPtr<u8>, ped: GPtr<u8>, f: &mut Fpu) {
    let ts = || d(TIME_STEP);
    let intel = || rd::<GPtr<u8>>(ped, 0x47C);
    let pad = || rd::<GPtr<u8>>(b, 0x2C);
    let front = this.byte_add(0x190);
    let alpha = this.byte_add(0xAC);
    let beta = this.byte_add(0xBC);
    // front = normalised T - hist[0]
    for (i, o) in [0x68, 0x6C, 0x70].into_iter().enumerate() {
        f.fld(rd(b, 0x5C + 4 * i as isize));
        f.fsub_m(rd(this, 0x1D8 + 4 * i as isize));
        wr::<f32>(b, o, f.fstp());
    }
    copy3(front, b.byte_add(0x68));
    vector_normalise(front);
    // |T - hist[1]| with the bytes' register order: (dx*dx + dz*dz) + dy*dy
    let row0 = rd::<u32>(b, 0x48) as usize;
    f.fld(rd(b, 0x5C));
    f.fsub_m(rd(this, 0x1E4));
    f.fld(rd(b, 0x60));
    f.fsub_m(rd(this, 0x1E8));
    f.fld(rd(b, 0x64));
    f.fsub_m(rd(this, 0x1EC));
    f.fld_st(2);
    f.fmulp(3);
    f.fld_st(0);
    f.fmul_st(1);
    f.faddp(3);
    f.fld_st(1);
    f.fmul_st(2);
    f.faddp(3);
    f.fxch(2);
    f.fsqrt();
    f.fstp_st(2);
    f.fstp_st(0);
    wr::<f32>(b, 0x3C, f.fst());
    f.fcomp_m(rd(b, 0x1C));
    if !f.jp(0x5) {
        f.fld(rd(b, 0x1C));
        f.fcomp_m(d(PED_CAM_TABLE + row0 + 0xC));
        if !f.jne(0x41) {
            f.fld(rd(b, 0x34));
            f.fcomp_m(rd(b, 0x3C));
            let v = if !f.jne(0x41) {
                rd::<u32>(b, 0x34)
            } else {
                rd::<u32>(b, 0x3C)
            };
            wr::<u32>(b, 0x1C, v);
        }
    }
    // 0x5234EC: a = atan2(-front.x, front.y) - pi/2 (+2pi below -pi)
    f.fld(rd(front, 0));
    wr::<u32>(b, 0x3C, 0);
    f.fchs();
    wr::<u32>(b, 0x14, 0);
    f.fld(rd(this, 0x194));
    fpatan(f);
    wr::<u32>(b, 0x10, 0);
    f.fsub_m(k(K_HALF_PI));
    wr::<f32>(b, 0x20, f.fst());
    f.fcomp_m(k(K_MPI));
    if !f.jp(0x5) {
        f.fld(rd(b, 0x20));
        f.fadd_m(k(K_2PI));
        wr::<f32>(b, 0x20, f.fstp());
    }
    // H = heading(ped) - pi/2, wrapped around a
    heading(f, ped);
    f.fsub_m(k(K_HALF_PI));
    wr::<f32>(b, 0x34, f.fst());
    f.fsub_m(rd(b, 0x20));
    f.fcom_m(k(K_PI));
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fld(rd(b, 0x34));
        f.fsub_m(k(K_2PI));
        wr::<f32>(b, 0x34, f.fstp());
    } else {
        f.fcomp_m(k(K_MPI));
        if !f.jp(0x5) {
            f.fld(rd(b, 0x34));
            f.fadd_m(k(K_2PI));
            wr::<f32>(b, 0x34, f.fstp());
        }
    }
    // 0x52358F: force-behind latch
    let pad0 = pad();
    if pad_get_force_camera_behind_player(pad0) != 0 {
        g::<u8>(FORCE_BEHIND).write(1);
    } else if g::<u8>(FORCE_BEHIND).read() != 0 {
        speed_sq_cmp(f, ped.byte_add(0x44), k(K_0_001));
        let clear = if !f.jne(0x41) {
            true
        } else {
            f.fld(rd(b, 0x34));
            f.fsub_m(rd(b, 0x20));
            f.fabs();
            f.fcomp_m(k(K_0_01));
            !f.jp(0x5) || aim_weapon_left_right(pad0, ped) != 0 || aim_weapon_up_down(pad0, ped) != 0
        };
        if clear {
            g::<u8>(FORCE_BEHIND).write(0);
        }
    }
    // 0x523614: the turn toward the ped's heading
    f.fld(rd(b, 0x34));
    f.fsub_m(rd(b, 0x20));
    wr::<f32>(b, 0x18, f.fst());
    f.fabs();
    f.fcomp_m(d(0x8CCE9C));
    let turn = !f.jp(0x5) || g::<u8>(FORCE_BEHIND).read() != 0 || {
        f.fld(rd(b, 0x38));
        f.fcomp_m(k(K_0));
        !f.jnp_eq()
    };
    if turn {
        let r = rd::<u32>(b, 0x48) as usize;
        f.fld(ts());
        let force = g::<u8>(FORCE_BEHIND).read();
        f.fmul_m(d(PED_CAM_TABLE + r + 0x28));
        wr::<f32>(b, 0x14, f.fstp());
        f.fld(ts());
        f.fmul_m(d(PED_CAM_TABLE + r + 0x2C));
        wr::<f32>(b, 0x10, f.fstp());
        let w_zero = force == 0 && {
            f.fld(rd(b, 0x38));
            f.fcomp_m(k(K_0));
            !f.jp(0x44)
        };
        if w_zero {
            let e = rd::<GPtr<u8>>(ped, 0x568);
            let mag = |out: isize| -> f64 {
                if e.is_null() {
                    vector_magnitude(ped.byte_add(0x44))
                } else {
                    vector_magnitude(vector_sub(b.byte_add(out), ped.byte_add(0x44), e.byte_add(0x44)))
                }
            };
            push_st0(f, mag(0x80));
            f.fmul_m(rd(b, 0x14));
            f.fcomp_m(k(K_1));
            if !f.jne(0x41) {
                wr::<u32>(b, 0x14, K_1);
            } else {
                push_st0(f, mag(0x74));
                f.fmul_m(rd(b, 0x14));
                wr::<f32>(b, 0x14, f.fstp());
            }
        } else {
            f.fld(rd(b, 0x14));
            f.fmul_m(d(0x8CCE98));
            f.fld(k(K_1));
            f.fcomp_st(1);
            if f.jp(0x5) {
                wr::<f32>(b, 0x14, f.fstp());
            } else {
                f.fstp_st(0);
                wr::<u32>(b, 0x14, K_1);
            }
            f.fld(d(0x8CCE94));
            f.fmul_m(rd(b, 0x10));
            wr::<f32>(b, 0x10, f.fstp());
            f.fld(rd(b, 0x38));
            f.fcomp_m(k(K_0));
            if !f.jnp_eq() {
                f.fld(rd(b, 0x14));
                f.fmul_m(rd(b, 0x38));
                wr::<f32>(b, 0x14, f.fstp());
                f.fld(rd(b, 0x10));
                f.fmul_m(rd(b, 0x38));
                wr::<f32>(b, 0x10, f.fstp());
            }
        }
        // 0x52378F: turn = clamp(d * r, -lim, lim)
        f.fld(rd(b, 0x18));
        f.fmul_m(rd(b, 0x14));
        clamp_st0(f, b, 0x10, 0x38);
    } else {
        f.fld(rd(b, 0x3C));
    }
    // 0x5237CB: a += turn; wrap a around beta; betaRate = (a - beta) / max(1, ts)
    f.fld(rd(b, 0x20));
    f.fadd_st(1);
    wr::<f32>(b, 0x20, f.fstp());
    f.fstp_st(0);
    f.fld(rd(beta, 0));
    f.fadd_m(k(K_PI));
    f.fcomp_m(rd(b, 0x20));
    if !f.jp(0x5) {
        f.fld(rd(b, 0x20));
        f.fsub_m(k(K_2PI));
        wr::<f32>(b, 0x20, f.fstp());
    } else {
        f.fld(rd(beta, 0));
        f.fsub_m(k(K_PI));
        f.fcomp_m(rd(b, 0x20));
        if !f.jne(0x41) {
            f.fld(rd(b, 0x20));
            f.fadd_m(k(K_2PI));
            wr::<f32>(b, 0x20, f.fstp());
        }
    }
    f.fld(k(K_1));
    f.fcomp_m(ts());
    if !f.jne(0x41) {
        f.fld(k(K_1));
    } else {
        f.fld(ts());
    }
    f.fld(rd(b, 0x20));
    f.fsub_m(rd(beta, 0));
    f.fdiv_st(1);
    wr::<f32>(b, 0x58, f.fstp());
    f.fstp_st(0);
    // 0x52384C: Lasin = asin(clamp(front.z))
    f.fld(k(K_1));
    f.fcomp_m(rd(this, 0x198));
    let lo = f.jp(0x5) && {
        f.fld(rd(this, 0x198));
        f.fld(k(K_M1));
        f.fcomp_st(1);
        f.fstp_st(0);
        !f.jne(0x41)
    };
    if lo {
        wr::<u32>(b, 0x3C, K_M1);
    } else {
        f.fld(k(K_1));
        f.fcomp_m(rd(this, 0x198));
        let v = if f.jp(0x5) { rd::<u32>(this, 0x198) } else { K_1 };
        wr::<u32>(b, 0x3C, v);
    }
    f.fld(rd(b, 0x3C));
    ci_asin(f);
    wr::<f32>(b, 0x18, f.fstp());
    // 0x5238B4: pitch limit when turning away at speed
    f.fld(rd(b, 0x34));
    f.fsub_m(rd(b, 0x20));
    f.fabs();
    wr::<f32>(b, 0x3C, f.fst());
    f.fcomp_m(d(0x8CCE90));
    if !f.jne(0x41) && {
        speed_sq_cmp(f, ped.byte_add(0x44), k(K_0_002));
        !f.jne(0x41)
    } {
        f.fld(rd(b, 0x3C));
        f.fsub_m(d(0x8CCE90));
        f.fmul_m(k(K_1_2));
        f.fld(k(K_PI));
        f.fsub_m(d(0x8CCE90));
        f.fdivp(1);
        f.fld(k(K_1));
        f.fcomp_st(1);
        if !f.jp(0x5) {
            f.fstp_st(0);
            f.fld(k(K_1));
        }
        f.fld(k(K_HALF_PI));
        f.fsub_m(d(0x8CCE8C));
        f.fmul_st(1);
        f.fsubr_m(k(K_HALF_PI));
        wr::<f32>(b, 0x3C, f.fstp());
        f.fstp_st(0);
        f.fld(d(0x8CCE88));
        f.fld(ts());
        ci_pow(f);
        f.fld(rd(b, 0x18));
        f.fcomp_m(rd(b, 0x3C));
        if !f.jne(0x41) {
            f.fld(k(K_1));
            f.fsub_st(1);
            f.fmul_m(rd(b, 0x3C));
            f.fxch(1);
            f.fmul_m(rd(b, 0x18));
            f.faddp(1);
            wr::<f32>(b, 0x18, f.fstp());
        } else {
            f.fld(rd(b, 0x3C));
            f.fchs();
            f.fcomp_m(rd(b, 0x18));
            if !f.jne(0x41) {
                f.fld(rd(b, 0x18));
                f.fmul_st(1);
                f.fld(k(K_1));
                f.fsub_st(2);
                f.fmul_m(rd(b, 0x3C));
                f.fsubp(1);
                wr::<f32>(b, 0x18, f.fstp());
            }
            f.fstp_st(0);
        }
    }
    // 0x5239B7: the pitch turn toward the swim / jetpack / slope target
    f.fld(k(K_0));
    f.fld(rd(b, 0x44));
    f.fcomp_m(k(K_0));
    let pitch_turn = f.jp(0x44) || (g::<u8>(FORCE_BEHIND).read() != 0 && rd::<u8>(ped, 0x46C) & 1 != 0);
    if pitch_turn {
        f.fstp_st(0);
        wr::<u32>(b, 0x3C, 0);
        if !get_task_jet_pack(intel()).is_null() {
            f.fld(k(K_JETPACK_PITCH));
        } else if !get_task_swim(intel()).is_null() {
            f.fld(k(K_SWIM_PITCH));
        } else if rd::<u8>(ped, 0x46C) & 1 != 0 {
            clamped_asin_of_dot(f, b, 0x3C, ped);
            f.fchs();
        } else {
            f.fld(rd(b, 0x3C));
        }
        f.fld(d(0x8CCE84));
        f.fmul_m(rd(b, 0x14));
        f.fld(k(K_1));
        f.fcomp_st(1);
        if !f.jp(0x5) {
            f.fstp_st(0);
            f.fld(k(K_1));
        }
        f.fld(d(0x8CCE80));
        f.fmul_m(rd(b, 0x10));
        wr::<f32>(b, 0x10, f.fstp());
        f.fld(rd(b, 0x44));
        f.fcomp_m(k(K_0));
        if !f.jnp_eq() {
            f.fmul_m(rd(b, 0x44));
            f.fld(rd(b, 0x10));
            f.fmul_m(rd(b, 0x44));
            wr::<f32>(b, 0x10, f.fstp());
        }
        f.fxch(1);
        f.fsub_m(rd(b, 0x18));
        f.fmulp(1);
        clamp_st0(f, b, 0x10, 0x38);
    }
    // 0x523B2C: Lasin = (Lasin + turnA) - pitchBase, clamped to [-C14, C13]
    f.fld(rd(b, 0x18));
    f.fadd_st(1);
    f.fsub_m(rd(b, 0x28));
    wr::<f32>(b, 0x18, f.fstp());
    f.fstp_st(0);
    f.fld(rd(b, 0x18));
    f.fcomp_m(rd(b, 0x4C));
    if !f.jne(0x41) {
        wr::<u32>(b, 0x18, rd::<u32>(b, 0x4C));
    } else {
        f.fld(rd(b, 0x50));
        f.fchs();
        f.fld(rd(b, 0x18));
        f.fcomp_st(1);
        if !f.jp(0x5) {
            wr::<f32>(b, 0x18, f.fstp());
        } else {
            f.fstp_st(0);
        }
    }
    // 0x523B70: Lap = clamp((Lasin - alpha) * (1 - pow(C5, ts)), -ts*C6, ts*C6)
    let r = rd::<u32>(b, 0x48) as usize;
    f.fld(ts());
    f.fmul_m(d(PED_CAM_TABLE + r + 0x18));
    wr::<f32>(b, 0x10, f.fstp());
    f.fld(d(PED_CAM_TABLE + r + 0x14));
    f.fld(ts());
    ci_pow(f);
    f.fsubr_m(k(K_1));
    f.fld(rd(b, 0x18));
    f.fsub_m(rd(alpha, 0));
    f.fmulp(1);
    wr::<f32>(b, 0x3C, f.fst());
    f.fcomp_m(rd(b, 0x10));
    if !f.jne(0x41) {
        wr::<u32>(b, 0x3C, rd::<u32>(b, 0x10));
    } else {
        f.fld(rd(b, 0x10));
        f.fchs();
        f.fld(rd(b, 0x3C));
        f.fcomp_st(1);
        if !f.jp(0x5) {
            wr::<f32>(b, 0x3C, f.fstp());
        } else {
            f.fstp_st(0);
        }
    }
    // 0x523BDD: stick input
    let lr = i32::from(aim_weapon_left_right(pad(), ped)).wrapping_neg();
    wr::<i32>(b, 0x44, lr);
    f.fild(rd(b, 0x44));
    wr::<f32>(b, 0x30, f.fstp());
    let ud = i32::from(aim_weapon_up_down(pad(), ped));
    wr::<i32>(b, 0x44, ud);
    f.fild(rd(b, 0x44));
    wr::<f32>(b, 0x24, f.fstp());
    if !get_task_use_gun(intel()).is_null() {
        let st = rd::<u8>(get_task_use_gun(intel()), 0x10);
        if (st == 2 || st == 3)
            && !rd::<GPtr<u8>>(get_task_use_gun(intel()), 0x30).is_null()
            && (rd::<u32>(rd::<GPtr<u8>>(get_task_use_gun(intel()), 0x30), 0x18) >> 1) & 1 == 0
        {
            let w = i32::from(pad_get_ped_walk_left_right(pad()));
            wr::<i32>(b, 0x44, w);
            f.fild(rd(b, 0x44));
            f.fabs();
            f.fld(rd(b, 0x30));
            f.fabs();
            f.fxch(1);
            f.fxch(1);
            f.fcompp();
            if !f.jp(0x5) {
                let w = i32::from(pad_get_ped_walk_left_right(pad())).wrapping_neg();
                wr::<i32>(b, 0x44, w);
                f.fild(rd(b, 0x44));
                wr::<f32>(b, 0x30, f.fstp());
            }
        }
    }
    // 0x523CA6: look where the camera looks while the aim stick moves and the ped stands
    f.fld(rd(b, 0x24));
    f.fcomp_m(k(K_0));
    let stick = f.jp(0x44) || {
        f.fld(rd(b, 0x30));
        f.fcomp_m(k(K_0));
        !f.jnp_eq()
    };
    if stick && pad_get_ped_walk_left_right(pad()) == 0 && pad_get_ped_walk_up_down(pad()) == 0 {
        look_at_camera_top(b, f);
    }
    // 0x523DD9: stick scaling
    f.fld(rd(this, 0xB4));
    f.fmul_m(k(K_0_0125));
    f.fld(rd(b, 0x30));
    f.fabs();
    f.fld_st(1);
    f.fmul_m(k(K_1_14));
    f.fmulp(1);
    f.fmul_m(d(0x8CC4A0));
    f.fmul_m(d(0x8CC4A0));
    f.fmul_m(rd(b, 0x30));
    wr::<f32>(b, 0x30, f.fstp());
    f.fld(rd(b, 0x24));
    f.fabs();
    f.fxch(1);
    f.fmul_m(k(K_3_70));
    f.fmulp(1);
    f.fmul_m(d(0x8CC4A0));
    f.fmul_m(d(0x8CC4A0));
    f.fmul_m(rd(b, 0x24));
    wr::<f32>(b, 0x24, f.fstp());
    // 0x523E2D: climb / enter-car stick modifiers
    if !get_task_climb(intel()).is_null() {
        let t = get_task_climb(intel());
        climb_get_camera_stick_modifier(t, ped, alpha, beta, b.byte_add(0x24), b.byte_add(0x30));
    } else if v_get_task_type(tm_get_active_task(intel().byte_add(4))) == 0x2BD {
        let z = rd::<f32>(b, 0x1C);
        let t = tm_get_active_task(intel().byte_add(4));
        enter_car_stick_modifier(
            t,
            ped,
            z,
            alpha.cast(),
            beta.cast(),
            b.byte_add(0x24).cast(),
            b.byte_add(0x30).cast(),
        );
    }
    // 0x523EA0: beta speed
    let r = rd::<u32>(b, 0x48) as usize;
    f.fld(d(PED_CAM_TABLE + r + 0x20));
    f.fld(ts());
    ci_pow(f);
    let lim = g::<u32>(PED_CAM_TABLE + rd::<u32>(b, 0x48) as usize + 0x24).read();
    wr::<f32>(b, 0x14, f.fstp());
    f.fld(rd(b, 0x30));
    wr::<u32>(b, 0x10, lim);
    f.fadd_m(rd(b, 0x58));
    clamp_st0(f, b, 0x10, 0x38);
    f.fld(k(K_1));
    f.fsub_m(rd(b, 0x14));
    wr::<f32>(b, 0x48, f.fstp());
    f.fld(rd(b, 0x14));
    f.fmul_m(rd(this, 0xC0));
    f.fld(rd(b, 0x48));
    f.fmul_st(2);
    f.faddp(1);
    f.fstp_st(1);
    wr::<f32>(this, 0xC0, f.fst());
    f.fabs();
    f.fcomp_m(d(0x8CCE7C));
    if !f.jp(0x5) {
        wr::<u32>(this, 0xC0, 0);
    }
    let mouse = || g::<u8>(USE_MOUSE_3RD_PERSON).read() != 0 && rd::<u16>(pad(), 0x10E) == 0;
    if mouse() {
        f.fld(d(MOUSE_X));
        f.fmul_m(k(K_M2_5));
        f.fld(rd(this, 0xB4));
        f.fmul_m(k(K_0_0125));
        f.fmulp(1);
        f.fmul_m(d(MOUSE_ACCEL_HORZNTL));
        wr::<u32>(this, 0xC0, 0);
        wr::<f32>(b, 0x30, f.fst());
    } else {
        f.fld(ts());
        f.fmul_m(rd(this, 0xC0));
    }
    f.fadd_m(rd(beta, 0));
    wr::<f32>(beta, 0, f.fstp());
    clip_beta(this);
    // 0x523F9C: alpha speed (p / q swapped against beta)
    f.fld(rd(b, 0x48));
    f.fmul_m(rd(this, 0xB0));
    f.fld(rd(b, 0x24));
    f.fmul_m(rd(b, 0x14));
    f.faddp(1);
    wr::<f32>(b, 0x58, f.fst());
    wr::<f32>(this, 0xB0, f.fst());
    f.fcomp_m(rd(b, 0x10));
    if !f.jne(0x41) {
        wr::<u32>(this, 0xB0, rd::<u32>(b, 0x10));
    } else {
        f.fld(rd(b, 0x10));
        f.fchs();
        f.fld(rd(b, 0x58));
        f.fcomp_st(1);
        if !f.jp(0x5) {
            wr::<f32>(this, 0xB0, f.fstp());
        } else {
            f.fstp_st(0);
        }
    }
    f.fld(rd(this, 0xB0));
    f.fabs();
    f.fcomp_m(d(0x8CCE78));
    if !f.jp(0x5) {
        wr::<u32>(this, 0xB0, 0);
    }
    f.fld(ts());
    f.fmul_m(rd(this, 0xB0));
    f.fadd_m(rd(b, 0x18));
    wr::<f32>(b, 0x18, f.fstp());
    if mouse() {
        f.fld(d(MOUSE_Y));
        let fading = g::<u8>(FADING).read();
        f.fmul_m(k(K_2_5));
        f.fld(rd(this, 0xB4));
        let fade = g::<u8>(FADE_VALUE).read();
        f.fmul_m(k(K_0_0125));
        f.fmulp(1);
        f.fmul_m(d(MOUSE_ACCEL_HORZNTL));
        wr::<f32>(b, 0x24, f.fstp());
        let auto = (fading != 0 && get_fading_direction(the_camera()) == 1 && fade > 0x2D) || fade > 0xC8;
        if auto {
            auto_pitch(this, b, f);
        } else {
            f.fld(rd(b, 0x24));
            wr::<u32>(this, 0xB0, 0);
        }
    } else {
        f.fld(rd(b, 0x3C));
    }
    f.fadd_m(rd(alpha, 0));
    wr::<f32>(alpha, 0, f.fstp());
    // 0x524151: alpha clamp
    f.fld(rd(alpha, 0));
    f.fcomp_m(rd(b, 0x4C));
    if !f.jne(0x41) {
        wr::<u32>(alpha, 0, rd::<u32>(b, 0x4C));
        wr::<u32>(this, 0xB0, 0);
    } else {
        f.fld(rd(b, 0x50));
        f.fchs();
        f.fcom_m(rd(alpha, 0));
        if !f.jne(0x41) {
            wr::<f32>(alpha, 0, f.fstp());
            wr::<u32>(this, 0xB0, 0);
        } else {
            f.fstp_st(0);
        }
    }
    // 0x524187: jitter filter
    f.fld(d(LAST_ALPHA));
    f.fsub_m(rd(alpha, 0));
    f.fabs();
    f.fcomp_m(d(0x8CCE70));
    if !f.jp(0x5) {
        wr::<u32>(alpha, 0, g::<u32>(LAST_ALPHA).read());
    }
    f.fld(d(LAST_BETA));
    let a = rd::<u32>(alpha, 0);
    f.fsub_m(rd(beta, 0));
    g::<u32>(LAST_ALPHA).write(a);
    f.fabs();
    f.fcomp_m(d(0x8CCE68));
    if !f.jp(0x5) {
        wr::<u32>(beta, 0, g::<u32>(LAST_BETA).read());
    }
    // 0x5241CC: front from alpha / beta
    f.fld(rd(alpha, 0));
    let bb = rd::<u32>(beta, 0);
    fcos(f);
    g::<u32>(LAST_BETA).write(bb);
    f.fld(rd(beta, 0));
    fcos(f);
    f.fmul_st(1);
    f.fchs();
    wr::<f32>(b, 0x74, f.fstp());
    wr::<u32>(front, 0, rd::<u32>(b, 0x74));
    f.fld(rd(beta, 0));
    fsin(f);
    f.fmul_st(1);
    f.fchs();
    wr::<f32>(b, 0x78, f.fstp());
    wr::<u32>(front, 4, rd::<u32>(b, 0x78));
    f.fstp_st(0);
    f.fld(rd(alpha, 0));
    fsin(f);
    wr::<f32>(b, 0x7C, f.fstp());
    wr::<u32>(front, 8, rd::<u32>(b, 0x7C));
    get_vectors_ready_for_rw(this);
    // 0x52421A: source = T - front * z (esp = B-4 from the pushed 4 on)
    let src = this.byte_add(0x19C);
    f.fld(rd(b, 0x1C));
    g::<u8>(CAM_DIRECTLY_BEHIND).write(0);
    g::<u8>(CAM_DIRECTLY_IN_FRONT).write(0);
    f.fmul_m(rd(front, 0));
    f.fld(rd(b, 0x1C));
    f.fmul_m(rd(front, 4));
    f.fld(rd(b, 0x1C));
    f.fmul_m(rd(front, 8));
    wr::<f32>(b, 0x70, f.fstp());
    f.fld(rd(b, 0x5C));
    f.fsub_st(2);
    wr::<f32>(b, 0x74, f.fstp());
    wr::<u32>(src, 0, rd::<u32>(b, 0x74));
    f.fld(rd(b, 0x60));
    f.fsub_st(1);
    wr::<f32>(b, 0x78, f.fstp());
    f.fstp_st(0);
    wr::<u32>(src, 4, rd::<u32>(b, 0x78));
    f.fstp_st(0);
    f.fld(rd(b, 0x64));
    f.fsub_m(rd(b, 0x70));
    wr::<f32>(b, 0x7C, f.fstp());
    wr::<u32>(src, 8, rd::<u32>(b, 0x7C));
    round_source(src);
    // 0x5242BB: the history points from the unclamped pitch Lasin + pitchBase
    f.fld(rd(b, 0x18));
    f.fadd_m(rd(b, 0x28));
    wr::<f32>(b, 0x18, f.fst());
    fcos(f);
    wr::<f32>(b, 0x50, f.fstp());
    f.fld(rd(beta, 0));
    fcos(f);
    f.fmul_m(rd(b, 0x50));
    f.fchs();
    wr::<f32>(b, 0x74, f.fst());
    f.fld(rd(beta, 0));
    fsin(f);
    f.fmul_m(rd(b, 0x50));
    f.fchs();
    f.fld_st(0);
    f.fld(rd(b, 0x18));
    fsin(f);
    wr::<f32>(b, 0x58, f.fst());
    f.fld(rd(b, 0x74));
    f.fmul_m(rd(b, 0x40));
    wr::<f32>(b, 0x68, f.fstp());
    f.fxch(1);
    f.fmul_m(rd(b, 0x40));
    wr::<f32>(b, 0x6C, f.fstp());
    f.fmul_m(rd(b, 0x40));
    hist_from_stack(this, b, 0x1D8, f);
    f.fxch(1);
    wr::<f32>(b, 0x74, f.fstp());
    f.fld(rd(b, 0x58));
    f.fld(rd(b, 0x74));
    f.fmul_m(rd(b, 0x1C));
    wr::<f32>(b, 0x68, f.fstp());
    f.fxch(1);
    f.fmul_m(rd(b, 0x1C));
    wr::<f32>(b, 0x6C, f.fstp());
    f.fmul_m(rd(b, 0x1C));
    hist_from_stack(this, b, 0x1E4, f);
    // 0x5243B7: aim rotation follows the camera while forced behind and the left/right aim moves
    let pad0 = pad();
    if pad_get_force_camera_behind_player(pad0) != 0 && aim_weapon_left_right(pad(), GPtr::NULL) != 0 {
        f.fld(rd(ped, 0x558));
        f.fsub_m(k(K_HALF_PI));
        f.fsubr_m(rd(beta, 0));
        f.fcom_m(k(K_PI));
        if !f.jne(0x41) {
            f.fsub_m(k(K_2PI));
        } else {
            f.fcom_m(k(K_MPI));
            if !f.jp(0x5) {
                f.fadd_m(k(K_2PI));
            }
        }
        f.fabs();
        f.fld(ts());
        f.fmul_m(k(K_0_1));
        f.fcompp();
        if !f.jne(0x41) {
            f.fld(rd(beta, 0));
            f.fadd_m(k(K_HALF_PI));
            wr::<f32>(ped, 0x55C, f.fstp());
        }
    }
    // 0x52442B: ducking, fudge point, collision
    let tp = b.byte_add(0x5C);
    handle_camera_motion_for_ducking(the_camera(), ped, src, tp, 0);
    copy3(this.byte_add(0x148), tp);
    round_source(src);
    let interior = rd::<i32>(b, 0x54);
    set_col_vars_ped(interior, g::<i32>(PED_ZOOM).read());
    if g::<i32>(G_CAMERA_DIRECTION).read() == 3 {
        camera_generic_mode_special_cases(the_camera(), ped);
        g::<u8>(CAM_COLLIDE_VEHICLES).write(1);
        g::<u8>(CAM_COLLIDE_OBJECTS).write(1);
        g::<u8>(CAM_COLLIDE_PEDS).write(1);
        camera_col_det_and_react(src, tp);
        improve_near_clip(the_camera(), GPtr::NULL, ped, src, tp);
        round_vector_decimals(src, 4);
    }
    g::<u8>(CAM_DIRECTLY_BEHIND).write(0);
    g::<u8>(CAM_DIRECTLY_IN_FRONT).write(0);
    round_source(src);
    get_vectors_ready_for_rw(this);
    // 0x52452D: the idle camera
    let idle = interior == 0
        && g::<i32>(WHO_IS_IN_CONTROL).read() != 1
        && rd::<u8>(ped, 0x46C) & 1 != 0
        && is_coop_game_going_on() == 0
        && g::<u8>(FOV_LERP_PROCESSED).read() == 0
        && g::<u8>(VEC_MOVE_LINEAR_PROCESSED).read() == 0
        && g::<u8>(VEC_TRACK_LINEAR_PROCESSED).read() == 0
        && {
            push_st0(f, magnitude_sqr(ped.byte_add(0x44)));
            f.fcomp_m(k(K_0_0001));
            !f.jp(0x41)
        };
    if idle {
        idle_cam_process(g::<u8>(G_IDLE_CAM));
    } else {
        g::<u32>(IDLE_COUNTER).write(0);
    }
    wr::<u8>(this, 0xA, 0);
}

/// 0x524313 / 0x52437D: `hist = T - (B+0x68, B+0x6C, st0)` through `B+0x74..0x7C`; the stack keeps the
/// two values below st0.
fn hist_from_stack(this: GPtr<u8>, b: GPtr<u8>, h_off: isize, f: &mut Fpu) {
    f.fld(rd(b, 0x5C));
    f.fsub_m(rd(b, 0x68));
    wr::<f32>(b, 0x74, f.fstp());
    wr::<u32>(this, h_off, rd::<u32>(b, 0x74));
    f.fld(rd(b, 0x60));
    f.fsub_m(rd(b, 0x6C));
    wr::<f32>(b, 0x78, f.fstp());
    f.fld(rd(b, 0x64));
    wr::<u32>(this, h_off + 4, rd::<u32>(b, 0x78));
    f.fsub_st(1);
    wr::<f32>(b, 0x7C, f.fstp());
    f.fstp_st(0);
    wr::<u32>(this, h_off + 8, rd::<u32>(b, 0x7C));
}

/// 0x523CF0: `g_ikChainMan.LookAt("FollowPedSA", player, nullptr, 1500, -1, &(pos + top * 5), false,
/// 0.25f, 500, 3, false)` when the camera's top direction points along the player's forward
/// (`(t.x*f.x + t.z*f.z) + t.y*f.y > 0.3`). The player pointer is kept in `B+0x44`, forward in
/// `B+0x74..0x7C`, top in `B+0x68..0x70`.
fn look_at_camera_top(b: GPtr<u8>, f: &mut Fpu) {
    let p0 = find_player_ped(0);
    wr(b, 0x44, p0);
    copy3(b.byte_add(0x74), rd::<GPtr<u8>>(p0, 0x14).byte_add(0x10));
    placeable_get_top_direction(the_camera(), b.byte_add(0x68));
    f.fld(rd(b, 0x68));
    f.fmul_m(rd(b, 0x74));
    f.fld(rd(b, 0x70));
    f.fmul_m(rd(b, 0x7C));
    f.faddp(1);
    f.fld(rd(b, 0x6C));
    f.fmul_m(rd(b, 0x78));
    f.faddp(1);
    f.fcomp_m(k(K_0_3));
    if f.jne(0x41) {
        return;
    }
    let p = rd::<GPtr<u8>>(b, 0x44);
    for (i, o) in [0x74, 0x78, 0x7C].into_iter().enumerate() {
        f.fld(rd(b, 0x68 + 4 * i as isize));
        f.fmul_m(k(K_5));
        wr::<f32>(b, o, f.fstp());
    }
    vec_add(b.byte_add(0x68), entity_pos(p), b.byte_add(0x74));
    let p = rd::<GPtr<u8>>(b, 0x44);
    ik_look_at(
        g::<u8>(IK_CHAIN_MAN),
        IK_PURPOSE,
        p,
        GPtr::NULL,
        0x5DC,
        -1,
        b.byte_add(0x68),
        0,
        0x3E80_0000,
        0x1F4,
        3,
        0,
    );
}

/// 0x524090: the automatic pitch toward `P = -pitchBase` (stored in `B+0x48`) after a fade: steps of at
/// most 0.05, leaves the step in ST0 and `B+0x24`; `alphaSpeed = 0`.
fn auto_pitch(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu) {
    let alpha = this.byte_add(0xAC);
    f.fld(rd(b, 0x28));
    f.fchs();
    wr::<f32>(b, 0x48, f.fst());
    f.fsub_m(k(K_0_05));
    f.fcomp_m(rd(alpha, 0));
    if !f.jne(0x41) {
        wr::<u32>(b, 0x24, K_0_05);
        f.fld(rd(b, 0x24));
        wr::<u32>(this, 0xB0, 0);
        return;
    }
    f.fld(rd(alpha, 0));
    f.fcomp_m(rd(b, 0x48));
    f.fld(rd(b, 0x48));
    if !f.jp(0x5) {
        f.fsub_m(rd(alpha, 0));
        wr::<u32>(this, 0xB0, 0);
        wr::<f32>(b, 0x24, f.fstp());
        f.fld(rd(b, 0x24));
        return;
    }
    f.fadd_m(k(K_0_05));
    f.fcomp_m(rd(alpha, 0));
    if !f.jp(0x5) {
        wr::<u32>(b, 0x24, 0xBD4C_CCCD);
        f.fld(rd(b, 0x24));
        wr::<u32>(this, 0xB0, 0);
        return;
    }
    f.fld(rd(alpha, 0));
    f.fcomp_m(rd(b, 0x48));
    if !f.jne(0x41) {
        f.fld(rd(b, 0x48));
        f.fsub_m(rd(alpha, 0));
        wr::<u32>(this, 0xB0, 0);
        wr::<f32>(b, 0x24, f.fstp());
        f.fld(rd(b, 0x24));
        return;
    }
    wr::<u32>(b, 0x24, 0);
    f.fld(rd(b, 0x24));
    wr::<u32>(this, 0xB0, 0);
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_FOLLOW_PED_SA];

/// Globals of this module (none: the statics are accessed by address).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];

/// `test ah, 0x44; jnp` taken (the compare was "equal": C3 alone).
trait JnpEq {
    fn jnp_eq(&self) -> bool;
}
impl JnpEq for Fpu {
    fn jnp_eq(&self) -> bool {
        !self.jp(0x44)
    }
}
