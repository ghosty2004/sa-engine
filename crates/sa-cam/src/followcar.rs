//! PORT-CAM-3 (helper B): `CCam::Process_FollowCar_SA` (0x5245B0), the in-vehicle third-person camera.
//! Ported from the 1.0 US bytes (0x5245B0-0x5245BD, the `.text` protection fragment 0x407E47 that loads
//! `EDI = this->m_pCamTargetEntity` and returns to 0x5245BD, and the body up to the `ret 0x14` at 0x525E48);
//! `docs/re/specs/CAM/CCam.md` as reading aid (gta-reversed has only a `NOTSA_UNREACHABLE()` stub).
//!
//! x87 values follow the original's stack: a value kept on the stack is an [`X87`], a value the bytes
//! store with `fstp dword` (the frame locals) is an `f32`. Memory and register operands keep their
//! forms (`fadd_m` / `fadd`) and their order. FSIN / FCOS / FPATAN use sa-math's f64 model; `_CIpow`,
//! `_CIasin`, `CPlaceable::GetHeading`, `CGeneral::GetATanOfXY` and the vehicle's `GetHeightAboveRoad`
//! come back from the original through an f64 (a 64-bit ST0 result is rounded to 53 bits first; the
//! documented approximation of the earlier CAM passes).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::{TIME_MS, TIME_STEP};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use sa_world::world::cworld::IGNORE_ENTITY;

use crate::common::{active_cam, f32_of, g, jne, jp, k, rd, the_camera, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// The per-vehicle-type camera settings (7 rows of 15 f32, `.data`; row `vt` at `+vt * 0x3C`)
    pub static CAR_CAM_SETTINGS @ 0x8CC600: [u32; 105] = [
        0x3FA66666, 0x3F800000, 0x3ECCCCCD, 0x41200000, 0x41700000, 0x3F000000, 0x3F800000, 0x3F800000,
        0x3F59999A, 0x3E4CCCCD, 0x3D99999A, 0x3D4CCCCD, 0x3F4CCCCD, 0x3F490FDB, 0x3FC6D3F2,
        0x3F8CCCCD, 0x3F800000, 0x3DCCCCCD, 0x41200000, 0x41300000, 0x3F000000, 0x3F800000, 0x3F800000,
        0x3F59999A, 0x3E4CCCCD, 0x3D99999A, 0x3D4CCCCD, 0x3F400000, 0x3F490FDB, 0x3FC6D3F2,
        0x3F8CCCCD, 0x3F800000, 0x3E4CCCCD, 0x41200000, 0x41700000, 0x3D4CCCCD, 0x3D4CCCCD, 0x00000000,
        0x3F666666, 0x3D4CCCCD, 0x3C23D70A, 0x3D4CCCCD, 0x3F800000, 0x3E32B8C3, 0x3F9C61AA,
        0x3F8CCCCD, 0x40600000, 0x3E4CCCCD, 0x41200000, 0x41C80000, 0x3F000000, 0x3F800000, 0x3F800000,
        0x3F400000, 0x3DCCCCCD, 0x3BA3D70A, 0x3E4CCCCD, 0x3F800000, 0x3FC6D3F2, 0x3FC6D3F2,
        0x3FA66666, 0x3F800000, 0x3ECCCCCD, 0x41200000, 0x41700000, 0x3F000000, 0x3F800000, 0x00000000,
        0x3F666666, 0x3D4CCCCD, 0x3BA3D70A, 0x3D4CCCCD, 0x3F800000, 0x3EB2B8C3, 0x3F9C61AA,
        0x3F8CCCCD, 0x3F800000, 0x3E4CCCCD, 0x41200000, 0x40A00000, 0x3F000000, 0x3F800000, 0x3F800000,
        0x3F400000, 0x3DCCCCCD, 0x3BA3D70A, 0x3E4CCCCD, 0x3F800000, 0x3F490FDB, 0x3FC6D3F2,
        0x3F8CCCCD, 0x3F800000, 0x3E4CCCCD, 0x41200000, 0x40A00000, 0x3F000000, 0x3F800000, 0x3F800000,
        0x3F400000, 0x3DCCCCCD, 0x3BA3D70A, 0x3E4CCCCD, 0x3F800000, 0x3EB2B8C3, 0x3F9C61AA,
    ];
    /// The look-height tables by `GetArrPosForVehicleType` slot: zoom 1 (0x8CC41C), zoom 2 / remote
    /// control (0x8CC430), zoom 3 (0x8CC444), 5 f32 each
    pub static CAR_ZOOM_HEIGHTS @ 0x8CC41C: [u32; 15] = [
        0x3DA3D70A, 0x3DA3D70A, 0x3E19999A, 0x3DA3D70A, 0x3DA3D70A,
        0x3D8F5C29, 0x3DA3D70A, 0x3E99999A, 0x3DA3D70A, 0x3DA3D70A,
        0x3D6147AE, 0x3D4CCCCD, 0x3E19999A, 0x3D75C28F, 0x3DA3D70A,
    ];
    /// The stick scale applied twice (0.007)
    pub static CAR_STICK_SCALE @ 0x8CC4A0: f32 = 0.007;
    /// heli up-vector offset (0.6), FOV speed threshold (0.4), FOV decay base (0.98)
    pub static CAR_HELI_FOV @ 0x8CC53C: [u32; 3] = [0x3F19999A, 0x3ECCCCCD, 0x3F7AE148];
    /// The function's `.data` block 0x8CCEA4..0x8CCEEC: 0.0001 (beta filter), `sLastBeta` (-9999),
    /// 0.0001, `sLastAlpha` (-9999), 0.0001, 0.075, 0.0001, 0.05, 0.0035, 0.1, 1.0, 1.2, 1.5, 1.2, 0.3,
    /// 0.4, 0.5, 0.02
    pub static FOLLOW_CAR_K @ 0x8CCEA4: [u32; 18] = [
        0x38D1B717, 0xC61C3C00, 0x38D1B717, 0xC61C3C00, 0x38D1B717, 0x3D99999A, 0x38D1B717, 0x3D4CCCCD,
        0x3B656042, 0x3DCCCCCD, 0x3F800000, 0x3F99999A, 0x3FC00000, 0x3F99999A, 0x3E99999A, 0x3ECCCCCD,
        0x3F000000, 0x3CA3D70A,
    ];
    /// `sPitchDone` (u8): the automatic pitch-back has finished
    pub static FOLLOW_CAR_PITCH_DONE @ 0xB70114: u8 = 0;
    /// `sMouseTimer`: frames left of mouse-look hold
    pub static FOLLOW_CAR_MOUSE_TIMER @ 0xB70118: f32 = 0.0;
    /// `sBlend`: the trailer / bike-passenger blend (0..1)
    pub static FOLLOW_CAR_BLEND @ 0xB7011C: f32 = 0.0;
    /// The target point of the last `Process_FollowCar_SA` (the CVector just before `TheCamera`)
    pub static FOLLOW_CAR_TARGET @ 0xB6F018: [u32; 3] = [0; 3];
}

/// `.data` addresses inside [`FOLLOW_CAR_K`] / [`CAR_HELI_FOV`].
const K_BETA_EPS: usize = 0x8CCEA4;
const LAST_BETA: usize = 0x8CCEA8;
const K_ALPHA_EPS: usize = 0x8CCEAC;
const LAST_ALPHA: usize = 0x8CCEB0;
const K_ASPEED_EPS: usize = 0x8CCEB4;
const K_PITCH_DOWN: usize = 0x8CCEB8;
const K_BSPEED_EPS: usize = 0x8CCEBC;
const K_AUTO_PITCH: usize = 0x8CCEC0;
const K_SOLICIT_RATE: usize = 0x8CCEC4;
const K_SOLICIT_MARGIN: usize = 0x8CCEC8;
const K_MOUSE_HOLD: usize = 0x8CCECC;
const K_SLOPE_DIST: usize = 0x8CCED0;
const K_SLOPE_BACK: usize = 0x8CCED4;
const K_SLOPE_SIDE: usize = 0x8CCED8;
const K_HEIGHT_ANGLE: usize = 0x8CCEDC;
const K_PASSENGER_HEIGHT: usize = 0x8CCEE0;
const K_TRAILER_LEN: usize = 0x8CCEE4;
const K_BLEND_RATE: usize = 0x8CCEE8;
const K_HELI_UP: usize = 0x8CC53C;
const K_FOV_SPEED: usize = 0x8CC540;
const K_FOV_DECAY: usize = 0x8CC544;
const K_STICK: usize = 0x8CC4A0;

/// `TheCamera` fields / CCamera statics used here.
const CAR_ZOOM: usize = 0xB6F0DC;
const CAR_ZOOM_SMOOTHED: usize = 0xB6F0E8;
const CAM_DIRECTLY_BEHIND: usize = 0xB6F042;
const CAM_DIRECTLY_IN_FRONT: usize = 0xB6F043;
const JUST_CAME_OUT_OF_GARAGE: usize = 0xB6F050;
const RESET_OLD_MATRIX: usize = 0xB6F999;
const EXTRA_ENTITIES_COUNT: usize = 0xB6FC70;
const TWEAK_DISTANCE: usize = 0xB6FD70;
const TWEAK_ALTITUDE: usize = 0xB6FD74;
const TWEAK_ANGLE: usize = 0xB6FD78;
const USE_MOUSE_3RD_PERSON: usize = 0xB6EC2E;
const MOUSE_ACCEL_HORZNTL: usize = 0xB6EC1C;
/// `CPad::NewMouseControllerState.m_AmountMoved` x / y
const MOUSE_MOVED_X: usize = 0xB73424;
const MOUSE_MOVED_Y: usize = 0xB73428;
/// `CVehicle::m_bEnableMouseSteering` / `m_bEnableMouseFlying`
const ENABLE_MOUSE_STEERING: usize = 0xC1CC02;
const ENABLE_MOUSE_FLYING: usize = 0xC1CC03;
/// `gCameraDirection`, `gCameraMode`
const CAMERA_DIRECTION: usize = 0x8CC384;
const CAMERA_MODE: usize = 0x8CC388;
/// `CPlane::HARRIER_NOZZLE_SWITCH_LIMIT` (i16)
const HARRIER_NOZZLE_SWITCH_LIMIT: usize = 0x8D33C8;
/// The byte `CameraVehicleModeSpecialCases`' callers set for vehicles with flag 0x429 & 4
const CAM_BYTE_9655E5: usize = 0x9655E5;

/// `.rdata` constants: 1.0 (0x858624), 0.0 (0x858B50), 0.5 (0x858B8C), PI (0x858CB8), 2PI (0x858CBC),
/// -PI (0x858CC0), PI/2 (0x858FE4), 70 (0x858CE0), 100 (0x858628), 0.65 (0x858F50), 0.02 (0x858B38),
/// -1.0 (0x858C1C), 0.04 (0x863244), 0.05 (0x858C28), 0.0125 (0x85A5C4), 1/14 (0x859B50), 3/70
/// (0x8631AC), 120 (0x858BB0), -2.0 (0x858B18), 50 (0x858B40).
const K_ONE: u32 = 0x3F80_0000;
const K_ZERO: u32 = 0;
const K_HALF: u32 = 0x3F00_0000;
const K_PI: u32 = 0x4049_0FDB;
const K_2PI: u32 = 0x40C9_0FDB;
const K_MPI: u32 = 0xC049_0FDB;
const K_HPI: u32 = 0x3FC9_0FDB;
const K_70: u32 = 0x428C_0000;
const K_100: u32 = 0x42C8_0000;
const K_0_65: u32 = 0x3F26_6666;
const K_0_02: u32 = 0x3CA3_D70A;
const K_M1: u32 = 0xBF80_0000;
const K_0_04: u32 = 0x3D23_D70B;
const K_0_05: u32 = 0x3D4C_CCCD;
const K_0_0125: u32 = 0x3C4C_CCCD;
const K_LR_SCALE: u32 = 0x3D92_4925;
const K_UD_SCALE: u32 = 0x3D2F_8AF9;
const K_120: u32 = 0x42F0_0000;
const K_M2: u32 = 0xC000_0000;
const K_50: u32 = 0x4248_0000;

/// `_CIpow` (st1 ^ st0) and `_CIasin` (ST0 -> ST0)
const CI_POW: usize = 0x822130;
const CI_ASIN: usize = 0x821E70;

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CCamera::ApplyVehicleCameraTweaks(CVehicle*)` (0x50A480)
    pub fn apply_vehicle_camera_tweaks(cam: GPtr<u8>, veh: GPtr<u8>) = thiscall 0x50A480 => crate::camfx::apply_vehicle_camera_tweaks;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    pub fn get_vehicle_appearance(veh: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CCamera::GetArrPosForVehicleType(int32 appearance, int32* out)` (0x50AF00)
    pub fn get_arr_pos_for_vehicle_type(cam: GPtr<u8>, kind: i32, out: GPtr<i32>) -> u8 = thiscall 0x50AF00 => crate::camctl::get_arr_pos_for_vehicle_type;
    /// `CEntity::GetColModel()` (0x535300)
    pub fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730), returns `out`
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `DotProduct(const CVector*, const CVector*)` (0x40FDB0, ST0)
    pub fn dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `CPlaceable::GetHeading()` (0x441DB0, the raw FPATAN in ST0)
    pub fn get_heading(e: GPtr<u8>) -> f64 = thiscall 0x441DB0 => unported;
    /// `CVehicle::GetHeightAboveRoad()` (vtable slot +0xD4, ST0)
    pub fn v_get_height_above_road(veh: GPtr<u8>) -> f64 = thiscall vslot 0xD4 => unported;
    /// `a > b ? a : b` (0x420800, cdecl floats as dwords, ST0)
    pub fn max_f(a: u32, b: u32) -> f64 = cdecl 0x420800 => unported;
    /// `CPad::AimWeaponLeftRight(CPed*)` (0x541040, AX)
    pub fn aim_weapon_left_right(pad: GPtr<u8>, ped: GPtr<u8>) -> i16 = thiscall 0x541040 => unported;
    /// `CPad::AimWeaponUpDown(CPed*)` (0x5410C0, AX)
    pub fn aim_weapon_up_down(pad: GPtr<u8>, ped: GPtr<u8>) -> i16 = thiscall 0x5410C0 => unported;
    /// `CPad::GetSteeringUpDown()` (0x53FBD0, AX)
    pub fn get_steering_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FBD0 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720)
    pub fn get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::GetTaskType()` (vtable slot +0x10)
    pub fn v_get_task_type(task: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CGeneral::GetATanOfXY(float, float)` (0x53CC70, ST0); the floats as dwords
    pub fn get_atan_of_xy(xb: u32, yb: u32) -> f64 = cdecl 0x53CC70 => unported;
    /// `CCam::ClipBeta()` (0x509C50)
    pub fn clip_beta(cam: GPtr<u8>) = thiscall 0x509C50 => crate::cam::clip_beta;
    /// `FUN_0050a0a0(float, int32)` (0x50A0A0, ST0): round to `n` decimals
    pub fn round_decimals(v: u32, n: i32) -> f64 = cdecl 0x50A0A0 => unported;
    /// `CCam::GetVectorsReadyForRW()` (0x509CE0)
    pub fn get_vectors_ready_for_rw(cam: GPtr<u8>) = thiscall 0x509CE0 => crate::cam::get_vectors_ready_for_rw;
    /// `CCamera::SetColVarsVehicle(int32 type, int32 zoom)` (0x50CCA0)
    pub fn set_col_vars_vehicle(ty: i32, zoom: i32) = cdecl 0x50CCA0 => crate::camfx::set_col_vars_vehicle;
    /// `CCamera::CameraVehicleModeSpecialCases(CVehicle*)` (0x50CDE0)
    pub fn camera_vehicle_mode_special_cases(cam: GPtr<u8>, veh: GPtr<u8>) = thiscall 0x50CDE0 => crate::camctl::camera_vehicle_mode_special_cases;
    /// `CCamera::CameraColDetAndReact(CVector* source, CVector* target)` (0x520190; ECX = TheCamera is
    /// loaded but never read)
    pub fn camera_col_det_and_react(source: GPtr<u8>, target: GPtr<u8>) -> u8 = stdcall 0x520190 => crate::camcol::camera_col_det_and_react;
    /// `CCamera::ImproveNearClip(CVehicle*, CPed*, CVector* source, CVector* target)` (0x516B20)
    pub fn improve_near_clip(cam: GPtr<u8>, veh: GPtr<u8>, ped: GPtr<u8>, source: GPtr<u8>, target: GPtr<u8>) = thiscall 0x516B20 => crate::camcol::improve_near_clip;
}

/// `fld dword` of a `.data` float.
fn d(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// Row `vt`, column `i` of [`CAR_CAM_SETTINGS`] (`[ebx + 0x8CC600 + 4i]`, `ebx = vt * 0x3C`).
fn c(vt: i32, i: usize) -> f32 {
    d(0x8CC600 + vt as usize * 0x3C + 4 * i)
}

/// `CTimer::ms_fTimeStep`, re-read at every use as the bytes do.
fn ts() -> f32 {
    TIME_STEP.get()
}

/// `_CIpow(base, exp)` (`fld base; fld exp; call _CIpow`).
fn pow(base: f32, exp: f32) -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_binary(
        CI_POW,
        x(base).to_f64(),
        x(exp).to_f64(),
    ))
}

/// `_CIasin(v)` with `v` on the stack.
fn asin(v: X87) -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_unary(CI_ASIN, v.to_f64()))
}

/// `CPed::m_pIntelligence->m_TaskMgr` (`[ped + 0x47C] + 4`).
fn task_manager(ped: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(ped, 0x47C).byte_add(4)
}

/// A frame-local CVector and its game pointer.
fn vp(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::<f32>::from_raw(v.as_mut_ptr()).cast::<u8>()
}

/// `v->x/y/z = FUN_0050a0a0(v->x/y/z, 4)` (each pushed as a dword, stored with `fstp dword`).
fn round_source(src: GPtr<u8>) {
    for o in [0, 4, 8] {
        let r = round_decimals(rd::<u32>(src, o), 4);
        wr::<f32>(src, o, f32_of(X87::from_f64_exact(r)));
    }
}

/// `CCam::Process_FollowCar_SA(const CVector& target, float orientation, float speedVar, float
/// speedVarWanted, bool keepAngles)` (0x5245B0, thiscall, `ret 0x14`; the three floats are never read).
/// The vehicle `V` is `this->m_pCamTargetEntity` (+0x21C); a non-vehicle returns at once. Steps
/// (`docs/re/specs/CAM/CCam.md` numbering):
/// 1. `T = *target`; the pad (`GetPad(1)` when the driver's ped type is 1); `ApplyVehicleCameraTweaks`.
/// 2. Camera type `vt` (models 0x1B9/0x1D0/0x234/0x252 -> 5, 0x1D1/0x1F5 -> 6, bikes / quads 1, helis
///    2, the Harrier with its nozzle at the switch limit 2, other planes 3 (0x21B: 0), boats 4, else 0);
///    the zoom distance `Lz = m_fCarZoomSmoothed + C1`; the look height `Lh` from the zoom tables.
/// 3. Length / height from the col model, the trailer (or bike passenger) blend `sBlend`, the tweak
///    distance / altitude / angle, the heli up-vector offset, `m_fCaMaxDistance = Lz`, min 3.5.
/// 4. FOV: speed bonus for cars and bikes, decay `pow(0.98, ts)` above 70, clamp to [70, 100].
/// 5. Reset (`m_bResetStatics` / directly behind / in front): angles from the vehicle heading, the
///    target history from the new front vector.
/// 6. Front from the history, the beta speed from the move direction, `asin(front.z)`, the slope
///    pitch on slow ground vehicles, the alpha speed, stick / mouse / solicit / auto-pitch inputs, the
///    `sLastAlpha` / `sLastBeta` jitter filter, the source, the history, `SetColVarsVehicle`, the
///    collision passes (gCameraDirection 3), and the source rounding to 4 decimals (three times).
#[hook(addr = 0x5245B0, conv = thiscall, name = "CCam::Process_FollowCar_SA", expect = "83 EC 68 56 8B F1", this = "CCam")]
pub fn process_follow_car_sa(
    this: GPtr<u8>,
    target: GPtr<u8>,
    _orientation: u32,
    _speed_var: u32,
    _speed_var_wanted: u32,
    keep_angles: u32,
) {
    let v = rd::<GPtr<u8>>(this, 0x21C);
    if rd::<u8>(v, 0x36) & 7 != 2 {
        return;
    }
    let zero = x(k(K_ZERO));
    let one = x(k(K_ONE));
    let alpha = || rd::<f32>(this, 0xAC);
    let beta = || rd::<f32>(this, 0xBC);
    let fov = || rd::<f32>(this, 0xB4);
    let mut t: [f32; 3] = [rd::<f32>(target, 0), rd::<f32>(target, 4), rd::<f32>(target, 8)];

    // 1. pad, tweaks
    let mut pad = get_pad(0);
    let drv = rd::<GPtr<u8>>(v, 0x460);
    if !drv.is_null() && rd::<i32>(drv, 0x598) == 1 {
        pad = get_pad(1);
    }
    apply_vehicle_camera_tweaks(the_camera(), v);

    // 2. camera type (L24), zoom (Lz, the target argument slot), look height (Lh, L1C)
    let model = rd::<u16>(v, 0x22);
    let vt: i32 = match model {
        0x1B9 | 0x1D0 | 0x234 | 0x252 => 5,
        0x1D1 | 0x1F5 => 6,
        _ if rd::<i32>(v, 0x590) == 9 => 1,
        _ => match rd::<i32>(v, 0x594) {
            2 => 1,
            3 => 2,
            4 if model == 0x208
                && i32::from(rd::<u16>(v, 0x86C))
                    >= i32::from(g::<i16>(HARRIER_NOZZLE_SWITCH_LIMIT).read()) =>
            {
                2
            }
            4 => (i32::from(model == 0x21B) - 1) & 3,
            5 => 4,
            _ => 0,
        },
    };
    let mut lz = f32_of(x(d(CAR_ZOOM_SMOOTHED)).fadd_m(c(vt, 1)));
    let mut arr: i32 = 0;
    let app = get_vehicle_appearance(v);
    get_arr_pos_for_vehicle_type(the_camera(), app, GPtr::from_raw(&mut arr));
    let zoom_height = |table: usize| d((table as u32).wrapping_add((arr as u32).wrapping_mul(4)) as usize);
    let mut lh: f32 = 0.0;
    if rd::<u8>(v, 0x36) & 0xF8 == 0x40 {
        lh = f32_of(x(zoom_height(0x8CC430)));
    } else {
        match g::<i32>(CAR_ZOOM).read() {
            1 => lh = f32_of(x(zoom_height(0x8CC41C))),
            2 => lh = f32_of(x(zoom_height(0x8CC430))),
            3 => lh = f32_of(x(zoom_height(0x8CC444))),
            _ => {}
        }
    }

    // 3. vehicle length (L38) and height (L18)
    let mut lmaxz = rd::<f32>(get_col_model(v), 0x14);
    let cm = get_col_model(v);
    let mut llen = f32_of(x(rd::<f32>(cm, 4)).fabs().fdouble());
    if !rd::<GPtr<u8>>(v, 0x4C8).is_null() {
        if !jp(x(FOLLOW_CAR_BLEND.get()), one, 0x05) {
            let t1 = x(ts()).fmul_m(d(K_BLEND_RATE)).fadd_m(FOLLOW_CAR_BLEND.get());
            if jp(one, t1, 0x05) {
                FOLLOW_CAR_BLEND.set(f32_of(t1));
            } else {
                FOLLOW_CAR_BLEND.set(k(K_ONE));
            }
        }
        let cv = get_col_model(v);
        let ct = get_col_model(rd::<GPtr<u8>>(v, 0x4C8));
        let dx = x(rd::<f32>(ct, 0xC)).fsub_m(rd::<f32>(cv, 0));
        let dy = x(rd::<f32>(ct, 0x10)).fsub_m(rd::<f32>(cv, 4));
        let dz = x(rd::<f32>(ct, 0x14)).fsub_m(rd::<f32>(cv, 8));
        let len = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
        llen = f32_of(
            len.fmul_m(d(K_TRAILER_LEN))
                .fmul_m(FOLLOW_CAR_BLEND.get())
                .fadd_m(llen),
        );
        let ctz = x(rd::<f32>(get_col_model(rd::<GPtr<u8>>(v, 0x4C8)), 0x14));
        let m = if jne(x(lmaxz), ctz, 0x41) { ctz } else { x(lmaxz) };
        lmaxz = f32_of(m.fsub_m(lmaxz).fmul_m(FOLLOW_CAR_BLEND.get()).fadd_m(lmaxz));
        let tr = rd::<GPtr<u8>>(v, 0x4C8);
        let tm = rd::<GPtr<u8>>(tr, 0x14);
        let p = if tm.is_null() {
            tr.byte_add(4)
        } else {
            tm.byte_add(0x30)
        };
        let kk = x(FOLLOW_CAR_BLEND.get()).fmul_m(k(K_HALF));
        let l60 = f32_of(kk.fmul_m(rd::<f32>(p, 0)));
        let l64 = f32_of(kk.fmul_m(rd::<f32>(p, 4)));
        let kpz = kk.fmul_m(rd::<f32>(p, 8));
        let w = one.fsub(kk);
        let txw = x(t[0]).fmul(w);
        let l70 = f32_of(w.fmul_m(t[1]));
        let l74 = f32_of(w.fmul_m(t[2]));
        t[0] = f32_of(txw.fadd_m(l60));
        t[1] = f32_of(x(l70).fadd_m(l64));
        t[2] = f32_of(x(l74).fadd(kpz));
    } else {
        let st = rd::<i32>(v, 0x594);
        if st != 9 && st != 2 {
            FOLLOW_CAR_BLEND.set(0.0);
        } else {
            let sb = x(FOLLOW_CAR_BLEND.get());
            if !rd::<GPtr<u8>>(v, 0x464).is_null() {
                if !jp(sb, one, 0x05) {
                    let t1 = x(ts()).fmul_m(d(K_BLEND_RATE)).fadd_m(FOLLOW_CAR_BLEND.get());
                    if jp(one, t1, 0x05) {
                        FOLLOW_CAR_BLEND.set(f32_of(t1));
                    } else {
                        FOLLOW_CAR_BLEND.set(k(K_ONE));
                    }
                }
            } else if !jne(sb, zero, 0x41) {
                let t1 = x(ts()).fmul_m(d(K_BLEND_RATE)).fsubr_m(FOLLOW_CAR_BLEND.get());
                if jne(zero, t1, 0x41) {
                    FOLLOW_CAR_BLEND.set(f32_of(t1));
                } else {
                    FOLLOW_CAR_BLEND.set(0.0);
                }
            }
            lmaxz = f32_of(
                x(d(K_PASSENGER_HEIGHT))
                    .fmul_m(FOLLOW_CAR_BLEND.get())
                    .fadd_m(lmaxz),
            );
        }
    }
    let a = x(d(TWEAK_DISTANCE)).fmul_m(llen);
    lz = f32_of(x(lz).fadd(a));
    let lmin2 = f32_of(a.fmul_m(c(vt, 3)));

    // height of the target point
    let tz: X87;
    if get_vehicle_appearance(v) == 3 && rd::<u8>(v, 0x36) & 0xF8 != 0x40 {
        let m = rd::<GPtr<u8>>(v, 0x14);
        let ux = x(d(K_HELI_UP)).fmul_m(rd::<f32>(m, 0x20));
        let uy = x(d(K_HELI_UP)).fmul_m(rd::<f32>(m, 0x24));
        let l68 = f32_of(x(d(K_HELI_UP)).fmul_m(rd::<f32>(m, 0x28)));
        let l6c = f32_of(ux.fmul_m(lmaxz));
        let uyl = uy.fmul_m(lmaxz);
        let l74 = f32_of(x(l68).fmul_m(lmaxz));
        t[0] = f32_of(x(l6c).fadd_m(t[0]));
        t[1] = f32_of(uyl.fadd_m(t[1]));
        tz = x(l74).fadd_m(t[2]);
    } else {
        let dd = x(lmaxz).fmul_m(c(vt, 0)).fsub_m(c(vt, 2));
        let l30 = f32_of(dd);
        if !jne(dd, zero, 0x41) {
            tz = x(t[2]).fadd_m(l30);
            lz = f32_of(x(l30).fadd_m(lz));
            lh = f32_of(x(d(K_HEIGHT_ANGLE)).fdiv_m(lz).fmul_m(l30).fadd_m(lh));
        } else {
            tz = x(t[2]);
        }
    }
    t[2] = f32_of(x(d(TWEAK_ALTITUDE)).fmul(tz));
    lh = f32_of(x(d(TWEAK_ANGLE)).fadd_m(lh));
    let mut lim = x(c(vt, 4));
    if g::<i32>(CAR_ZOOM).read() == 1 && (vt == 0 || vt == 1) {
        lim = lim.fmul_m(k(K_0_65));
    }
    let lmin = if jne(x(lz), lim, 0x41) { f32_of(lim) } else { lz };

    // 4. FOV
    let reset = rd::<u8>(this, 0xA);
    wr::<f32>(this, 0xD0, lz);
    wr::<u32>(this, 0xCC, 0x4060_0000);
    if reset != 0 {
        wr::<u32>(this, 0xB4, K_70);
    } else {
        let st = rd::<i32>(v, 0x594);
        if st == 0 || st == 9 {
            let m = rd::<GPtr<u8>>(v, 0x14);
            let dot = x(rd::<f32>(m, 0x18))
                .fmul_m(rd::<f32>(v, 0x4C))
                .fadd(x(rd::<f32>(m, 0x14)).fmul_m(rd::<f32>(v, 0x48)))
                .fadd(x(rd::<f32>(v, 0x44)).fmul_m(rd::<f32>(m, 0x10)));
            if !jne(dot, x(d(K_FOV_SPEED)), 0x41) {
                let dot2 = x(rd::<f32>(v, 0x4C))
                    .fmul_m(rd::<f32>(m, 0x18))
                    .fadd(x(rd::<f32>(v, 0x48)).fmul_m(rd::<f32>(m, 0x14)))
                    .fadd(x(rd::<f32>(v, 0x44)).fmul_m(rd::<f32>(m, 0x10)));
                let f = dot2.fsub_m(d(K_FOV_SPEED)).fmul_m(ts()).fadd_m(fov());
                wr::<f32>(this, 0xB4, f32_of(f));
            }
        }
        if !jne(x(fov()), x(k(K_70)), 0x41) {
            let pw = pow(d(K_FOV_DECAY), ts());
            let r = x(fov()).fsub_m(k(K_70));
            wr::<f32>(this, 0xB4, f32_of(pw.fmul(r).fadd_m(k(K_70))));
        }
        if !jne(x(fov()), x(k(K_100)), 0x41) {
            wr::<u32>(this, 0xB4, K_100);
        } else if !jp(x(fov()), x(k(K_70)), 0x05) {
            wr::<u32>(this, 0xB4, K_70);
        }
    }

    // 5. reset
    let front = this.byte_add(0x190);
    if reset != 0 || g::<u8>(CAM_DIRECTLY_BEHIND).read() != 0 || g::<u8>(CAM_DIRECTLY_IN_FRONT).read() != 0 {
        let keep = keep_angles as u8;
        wr::<u8>(this, 0xA, 0);
        wr::<u8>(this, 0xB, 0);
        wr::<u8>(this, 3, 1);
        let garage = g::<u8>(JUST_CAME_OUT_OF_GARAGE).read();
        g::<u8>(RESET_OLD_MATRIX).write(1);
        if garage == 0 && keep == 0 {
            wr::<u32>(this, 0xAC, 0);
            let m = rd::<GPtr<u8>>(v, 0x14);
            let h = if m.is_null() {
                x(rd::<f32>(v, 0x10))
            } else {
                x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)))
            };
            let b = h.fsub_m(k(K_HPI));
            wr::<f32>(this, 0xBC, f32_of(b));
            if g::<u8>(CAM_DIRECTLY_IN_FRONT).read() != 0 {
                wr::<f32>(this, 0xBC, f32_of(b.fadd_m(k(K_PI))));
            }
        }
        let ca = x(alpha()).fcos();
        wr::<u32>(this, 0xC0, 0);
        wr::<u32>(this, 0xB0, 0);
        wr::<u32>(this, 0xC4, 0x447A_0000);
        wr::<f32>(front, 0, f32_of(x(beta()).fcos().fmul(ca).fchs()));
        wr::<f32>(front, 4, f32_of(x(beta()).fsin().fmul(ca).fchs()));
        wr::<f32>(front, 8, f32_of(x(alpha()).fsin()));
        let hist = |l: f32, o: isize| {
            let px = x(l).fmul_m(rd::<f32>(front, 0));
            let py = x(l).fmul_m(rd::<f32>(front, 4));
            let l74 = f32_of(x(l).fmul_m(rd::<f32>(front, 8)));
            wr::<f32>(this, o, f32_of(x(t[0]).fsub(px)));
            wr::<f32>(this, o + 4, f32_of(x(t[1]).fsub(py)));
            wr::<f32>(this, o + 8, f32_of(x(t[2]).fsub_m(l74)));
        };
        hist(lmin, 0x1D8);
        wr::<u32>(this, 0x208, TIME_MS.get());
        hist(lz, 0x1E4);
        wr::<u32>(this, 0x218, 0);
        if g::<u8>(JUST_CAME_OUT_OF_GARAGE).read() == 0 && keep == 0 {
            wr::<f32>(this, 0xAC, f32_of(x(lh).fchs()));
        }
    }

    // 6. direction, beta speed (L44), asin of front.z (L14)
    wr::<f32>(front, 0, f32_of(x(t[0]).fsub_m(rd::<f32>(this, 0x1D8))));
    wr::<f32>(front, 4, f32_of(x(t[1]).fsub_m(rd::<f32>(this, 0x1DC))));
    wr::<f32>(front, 8, f32_of(x(t[2]).fsub_m(rd::<f32>(this, 0x1E0))));
    vector_normalise(front);
    let hx = x(t[0]).fsub_m(rd::<f32>(this, 0x1E4));
    let hy = x(t[1]).fsub_m(rd::<f32>(this, 0x1E8));
    let hz = x(t[2]).fsub_m(rd::<f32>(this, 0x1EC));
    let ldist = f32_of(hx.fmul(hx).fadd(hz.fmul(hz)).fadd(hy.fmul(hy)).fsqrt());
    let mut ang = x(rd::<f32>(front, 0))
        .fchs()
        .fpatan(x(rd::<f32>(this, 0x194)))
        .fsub_m(k(K_HPI));
    if !jp(ang, x(k(K_MPI)), 0x05) {
        ang = ang.fadd_m(k(K_2PI));
    }
    let (mvx, mvy) = (x(rd::<f32>(v, 0x44)), x(rd::<f32>(v, 0x48)));
    let sp = mvx.fmul(mvx).fadd(mvy.fmul(mvy)).fsqrt();
    let mut bm = if !jne(sp, x(k(K_0_02)), 0x41) {
        x(rd::<f32>(v, 0x44))
            .fchs()
            .fpatan(x(rd::<f32>(v, 0x48)))
            .fsub_m(k(K_HPI))
    } else {
        ang
    };
    if !jne(bm, ang.fadd_m(k(K_PI)), 0x41) {
        bm = bm.fsub_m(k(K_2PI));
    } else if !jp(bm, ang.fsub_m(k(K_PI)), 0x05) {
        bm = bm.fadd_m(k(K_2PI));
    }
    let rate = x(ts()).fmul_m(c(vt, 10));
    let lclamp = f32_of(x(ts()).fmul_m(c(vt, 11)));
    let dv = x(rd::<f32>(v, 0x4C))
        .fmul_m(rd::<f32>(front, 8))
        .fadd(x(rd::<f32>(v, 0x48)).fmul_m(rd::<f32>(front, 4)))
        .fadd(x(rd::<f32>(v, 0x44)).fmul_m(rd::<f32>(front, 0)));
    let pfx = dv.fmul_m(rd::<f32>(front, 0));
    let l70 = f32_of(dv.fmul_m(rd::<f32>(front, 4)));
    let l74 = f32_of(dv.fmul_m(rd::<f32>(front, 8)));
    let lx = pfx.fsubr_m(rd::<f32>(v, 0x44));
    let ly = x(rd::<f32>(v, 0x48)).fsub_m(l70);
    let lzz = x(rd::<f32>(v, 0x4C)).fsub_m(l74);
    let ll = f32_of(lx.fmul(lx).fadd(lzz.fmul(lzz)).fadd(ly.fmul(ly)).fsqrt());
    let mut f = rate.fmul_m(ll);
    if !jp(one, f, 0x05) {
        f = one;
    }
    let mut dd = f.fmul(bm.fsub(ang));
    if !jne(dd, x(lclamp), 0x41) {
        dd = x(lclamp);
    } else {
        let n = f32_of(x(lclamp).fchs());
        if !jp(dd, x(n), 0x05) {
            dd = x(n);
        }
    }
    let mut nb = ang.fadd(dd);
    if !jne(nb, x(beta()).fadd_m(k(K_PI)), 0x41) {
        nb = nb.fsub_m(k(K_2PI));
    } else if !jp(nb, x(beta()).fsub_m(k(K_PI)), 0x05) {
        nb = nb.fadd_m(k(K_2PI));
    }
    let dt = if !jne(one, x(ts()), 0x41) { one } else { x(ts()) };
    let lbeta_spd = f32_of(nb.fsub_m(beta()).fdiv(dt));
    let fz = rd::<f32>(this, 0x198);
    let in_range = |fz: f32| if jp(one, x(fz), 0x05) { fz } else { k(K_ONE) };
    let clamped = if jp(one, x(fz), 0x05) && !jne(x(k(K_M1)), x(fz), 0x41) {
        k(K_M1)
    } else {
        in_range(fz)
    };
    let mut lasin = f32_of(asin(x(clamped)));

    // distance (Lz) and the pitch limit (Lup, L28)
    if !jp(x(ldist), x(lz), 0x05) && !jne(x(lz), x(lmin2), 0x41) {
        lz = if !jne(x(lmin2), x(ldist), 0x41) {
            lmin2
        } else {
            ldist
        };
    }
    let mut lup = c(vt, 13);
    let mvz = x(rd::<f32>(v, 0x4C));
    let (mvx, mvy) = (x(rd::<f32>(v, 0x44)), x(rd::<f32>(v, 0x48)));
    let sq = mvx.fmul(mvx).fadd(mvy.fmul(mvy)).fadd(mvz.fmul(mvz));
    if !jp(sq, x(k(K_0_04)), 0x05) {
        let skip = (rd::<i32>(v, 0x590) == 9 && rd::<u8>(v, 0x804) < 4) || {
            let st = rd::<i32>(v, 0x594);
            st == 3 || (st == 4 && rd::<u8>(v, 0x960) == 0)
        };
        if !skip {
            slope_pitch(this, v, &t, &mut lup);
        }
    }

    // pitch target (L14) and the alpha step (Lap, L30)
    let s1 = x(lasin).fsub_m(lh);
    lasin = f32_of(s1);
    if !jne(s1, x(lup), 0x41) {
        lasin = lup;
    } else {
        let n = x(c(vt, 14)).fchs();
        if !jp(x(lasin), n, 0x05) {
            lasin = f32_of(n);
        }
    }
    let lclamp2 = f32_of(x(ts()).fmul_m(c(vt, 6)));
    let lap_x = pow(c(vt, 5), ts())
        .fsubr_m(k(K_ONE))
        .fmul(x(lasin).fsub_m(alpha()));
    let mut lap = f32_of(lap_x);
    if !jne(lap_x, x(lclamp2), 0x41) {
        lap = lclamp2;
    } else {
        let n = x(lclamp2).fchs();
        if !jp(x(lap), n, 0x05) {
            lap = f32_of(n);
        }
    }

    // stick input: horizontal (Lh2, L20), vertical (Lv, L10)
    let lr = i32::from(aim_weapon_left_right(pad, GPtr::NULL)).wrapping_neg();
    let mut lh2 = f32_of(xi(lr));
    let ud = i32::from(aim_weapon_up_down(pad, GPtr::NULL));
    let udx = if g::<u8>(USE_MOUSE_3RD_PERSON).read() != 0 {
        zero
    } else {
        xi(ud)
    };
    let fv = x(fov()).fmul_m(k(K_0_0125));
    lh2 = f32_of(
        x(lh2)
            .fabs()
            .fmul(fv.fmul_m(k(K_LR_SCALE)))
            .fmul_m(lh2)
            .fmul_m(d(K_STICK))
            .fmul_m(d(K_STICK)),
    );
    let mut lv = f32_of(
        udx.fabs()
            .fmul(fv.fmul_m(k(K_UD_SCALE)))
            .fmul(udx)
            .fmul_m(d(K_STICK))
            .fmul_m(d(K_STICK)),
    );
    let mut allow_auto = true;
    match model {
        0x1BB | 0x1E6 | 0x196 | 0x20C | 0x250 | 0x208 | 0x20D | 0x212 | 0x213 => lv = 0.0,
        0x234 => {
            lh2 = 0.0;
            lv = 0.0;
        }
        _ if rd::<i32>(v, 0x590) == 0 && rd::<u32>(v, 0x38C) & 0x20000 != 0 => {
            lh2 = 0.0;
            lv = 0.0;
        }
        _ => allow_auto = false,
    }
    if g::<i32>(CAMERA_DIRECTION).read() != 3 {
        lv = 0.0;
        lh2 = 0.0;
    }
    if vt == 0 {
        let s = i32::from(get_steering_up_down(pad));
        if !jne(xi(s).fabs(), x(k(K_120)), 0x41) {
            let drv = rd::<GPtr<u8>>(v, 0x460);
            if !drv.is_null() && !get_active_task(task_manager(drv)).is_null() {
                let task = get_active_task(task_manager(rd::<GPtr<u8>>(v, 0x460)));
                if v_get_task_type(task) != 0x2C0 {
                    let s2 = xi(i32::from(get_steering_up_down(pad)));
                    let f = x(fov()).fmul_m(k(K_0_0125)).fmul_m(k(K_UD_SCALE));
                    lv = f32_of(
                        s2.fabs()
                            .fmul(f)
                            .fmul(s2)
                            .fmul_m(d(K_STICK))
                            .fmul_m(d(K_STICK))
                            .fmul_m(k(K_HALF))
                            .fadd_m(lv),
                    );
                }
            }
        }
    }
    if !jne(x(lv), zero, 0x41) {
        lv = f32_of(x(lv).fmul_m(k(K_HALF)));
    }

    // mouse
    let mut mouse_used = false;
    if g::<u8>(USE_MOUSE_3RD_PERSON).read() != 0 && rd::<u16>(pad, 0x10E) == 0 {
        let my2 = f32_of(x(d(MOUSE_MOVED_Y)).fdouble());
        let mx2 = f32_of(x(d(MOUSE_MOVED_X)).fmul_m(k(K_M2)));
        let st = rd::<i32>(v, 0x594);
        let en = if st == 4 || st == 3 {
            g::<u8>(ENABLE_MOUSE_FLYING).read()
        } else {
            g::<u8>(ENABLE_MOUSE_STEERING).read()
        };
        let moved = jp(x(mx2), zero, 0x44) || jp(x(my2), zero, 0x44);
        if moved && (rd::<u16>(pad, 0x2C) != 0 || en == 0) {
            let gg = x(fov()).fmul_m(k(K_0_0125));
            lv = f32_of(x(my2).fmul(gg).fmul_m(d(MOUSE_ACCEL_HORZNTL)));
            let h = x(mx2).fmul(gg).fmul_m(d(MOUSE_ACCEL_HORZNTL));
            wr::<u32>(this, 0xC0, 0);
            wr::<u32>(this, 0xB0, 0);
            lh2 = f32_of(h);
            lasin = alpha();
            FOLLOW_CAR_MOUSE_TIMER.set(f32_of(x(d(K_MOUSE_HOLD)).fmul_m(k(K_50))));
            mouse_used = true;
        } else if !jne(x(FOLLOW_CAR_MOUSE_TIMER.get()), zero, 0x41) {
            let al = alpha();
            let tt = f32_of(x(FOLLOW_CAR_MOUSE_TIMER.get()).fsub_m(ts()));
            wr::<u32>(this, 0xC0, 0);
            wr::<u32>(this, 0xB0, 0);
            lv = 0.0;
            lh2 = 0.0;
            lasin = al;
            let r = max_f(0, tt.to_bits());
            FOLLOW_CAR_MOUSE_TIMER.set(f32_of(X87::from_f64_exact(r)));
            mouse_used = true;
        }
    }

    // passenger soliciting (TASK_COMPLEX_PROSTITUTE_SOLICIT)
    let ps = rd::<GPtr<u8>>(v, 0x464);
    if !ps.is_null()
        && !get_active_task(task_manager(ps)).is_null()
        && v_get_task_type(get_active_task(task_manager(rd::<GPtr<u8>>(v, 0x464)))) == 0x51D
        && rd::<u8>(get_active_task(task_manager(rd::<GPtr<u8>>(v, 0x464))), 0x2D) & 1 != 0
    {
        lv = if !jne(x(lup).fsub_m(d(K_SOLICIT_MARGIN)), x(alpha()), 0x41) {
            f32_of(x(ts()).fmul_m(d(K_SOLICIT_RATE)))
        } else {
            0.0
        };
    }

    // automatic pitch-back
    if allow_auto {
        let test = if g::<i32>(CAMERA_MODE).read() != 0x12 {
            FOLLOW_CAR_PITCH_DONE.set(0);
            true
        } else {
            FOLLOW_CAR_PITCH_DONE.get() == 0
        };
        let mut done = !test;
        if test {
            if !jne(x(lh).fadd_m(alpha()).fabs(), x(k(K_0_05)), 0x41) {
                lv = f32_of(x(lh).fchs().fsub_m(alpha()).fmul_m(d(K_AUTO_PITCH)));
            } else {
                done = true;
            }
        }
        if done {
            FOLLOW_CAR_PITCH_DONE.set(1);
        }
    }
    lv = f32_of(x(lv).fmul_m(c(vt, 12)));
    lh2 = f32_of(x(lh2).fmul_m(c(vt, 12)));

    // beta
    let pw = f32_of(pow(c(vt, 8), ts()));
    let c9 = c(vt, 9);
    let mut xx = x(lh2).fadd_m(lbeta_spd);
    if !jne(xx, x(c9), 0x41) {
        xx = x(c9);
    } else {
        let n = f32_of(x(c9).fchs());
        if !jp(xx, x(n), 0x05) {
            xx = x(n);
        }
    }
    let q = one.fsub_m(pw);
    let q32 = f32_of(q);
    let bs = xx.fmul(q).fadd(x(pw).fmul_m(rd::<f32>(this, 0xC0)));
    wr::<f32>(this, 0xC0, f32_of(bs));
    if !jp(bs.fabs(), x(d(K_BSPEED_EPS)), 0x05) {
        wr::<u32>(this, 0xC0, 0);
    }
    let inc = if mouse_used {
        x(lh2)
    } else {
        x(ts()).fmul_m(rd::<f32>(this, 0xC0))
    };
    wr::<f32>(this, 0xBC, f32_of(inc.fadd_m(beta())));
    if g::<u8>(JUST_CAME_OUT_OF_GARAGE).read() != 0 {
        let a = X87::from_f64_exact(get_atan_of_xy(rd::<u32>(front, 0), rd::<u32>(front, 4)));
        wr::<f32>(this, 0xBC, f32_of(a.fadd_m(k(K_PI))));
    }
    clip_beta(this);

    // extra pitch-down when the target is behind the history
    if vt <= 1 && !jp(x(lasin), x(alpha()), 0x05) && !jne(x(ldist), x(lz), 0x01) {
        let wheels = match rd::<i32>(v, 0x590) {
            0 => Some(rd::<u8>(v, 0x960)),
            9 => Some(rd::<u8>(v, 0x804)),
            _ => None,
        };
        if wheels.is_some_and(|n| n > 1) {
            lv = f32_of(x(lasin).fsub_m(alpha()).fmul_m(d(K_PITCH_DOWN)).fadd_m(lv));
        }
    }

    // alpha
    let asp = x(q32).fmul_m(lv).fadd(x(pw).fmul_m(rd::<f32>(this, 0xB0)));
    let as32 = f32_of(asp);
    wr::<f32>(this, 0xB0, as32);
    let mut lim = x(c9);
    if !jne(x(lv), zero, 0x41) {
        lim = lim.fmul_m(k(K_HALF));
    }
    if !jne(x(as32), lim, 0x41) {
        wr::<f32>(this, 0xB0, f32_of(lim));
    } else {
        let n = lim.fchs();
        if !jp(x(as32), n, 0x05) {
            wr::<f32>(this, 0xB0, f32_of(n));
        }
    }
    if !jp(x(rd::<f32>(this, 0xB0)).fabs(), x(d(K_ASPEED_EPS)), 0x05) {
        wr::<u32>(this, 0xB0, 0);
    }
    let add = if mouse_used {
        lasin = f32_of(x(lv).fadd_m(lasin));
        x(lv)
    } else {
        lasin = f32_of(x(ts()).fmul_m(rd::<f32>(this, 0xB0)).fadd_m(lasin));
        x(lap)
    };
    wr::<f32>(this, 0xAC, f32_of(add.fadd_m(alpha())));
    if !jne(x(alpha()), x(lup), 0x41) {
        wr::<f32>(this, 0xAC, lup);
        wr::<u32>(this, 0xB0, 0);
    } else {
        let n = x(c(vt, 14)).fchs();
        if !jne(n, x(alpha()), 0x41) {
            wr::<f32>(this, 0xAC, f32_of(n));
            wr::<u32>(this, 0xB0, 0);
        }
    }

    // jitter filter
    if !jp(x(d(LAST_ALPHA)).fsub_m(alpha()).fabs(), x(d(K_ALPHA_EPS)), 0x05) {
        wr::<u32>(this, 0xAC, g::<u32>(LAST_ALPHA).read());
    }
    let lb = x(d(LAST_BETA)).fsub_m(beta());
    g::<u32>(LAST_ALPHA).write(rd::<u32>(this, 0xAC));
    if !jp(lb.fabs(), x(d(K_BETA_EPS)), 0x05) {
        wr::<u32>(this, 0xBC, g::<u32>(LAST_BETA).read());
    }

    // front, source, history
    let ca = x(alpha()).fcos();
    g::<u32>(LAST_BETA).write(rd::<u32>(this, 0xBC));
    wr::<f32>(front, 0, f32_of(x(beta()).fcos().fmul(ca).fchs()));
    wr::<f32>(front, 4, f32_of(x(beta()).fsin().fmul(ca).fchs()));
    wr::<f32>(front, 8, f32_of(x(alpha()).fsin()));
    let source = this.byte_add(0x19C);
    round_source(source);
    get_vectors_ready_for_rw(this);
    g::<u8>(CAM_DIRECTLY_BEHIND).write(0);
    g::<u8>(CAM_DIRECTLY_IN_FRONT).write(0);
    let px = x(lz).fmul_m(rd::<f32>(front, 0));
    let py = x(lz).fmul_m(rd::<f32>(front, 4));
    let l74 = f32_of(x(lz).fmul_m(rd::<f32>(front, 8)));
    wr::<f32>(source, 0, f32_of(x(t[0]).fsub(px)));
    wr::<f32>(source, 4, f32_of(x(t[1]).fsub(py)));
    wr::<f32>(source, 8, f32_of(x(t[2]).fsub_m(l74)));
    lasin = f32_of(x(lasin).fadd_m(lh));
    for (i, o) in [0x148, 0x14C, 0x150].into_iter().enumerate() {
        wr::<f32>(this, o, t[i]);
    }
    let ca2 = f32_of(x(lasin).fcos());
    for o in [0, 4, 8] {
        wr::<u32>(this, 0x1F0 + o, rd::<u32>(this, 0x1D8 + o));
    }
    let fxp = f32_of(x(beta()).fcos().fmul_m(ca2).fchs());
    let fyp = x(beta()).fsin().fmul_m(ca2).fchs();
    let sz = x(lasin).fsin();
    let sz32 = f32_of(sz);
    wr::<f32>(this, 0x1D8, f32_of(x(t[0]).fsub_m(f32_of(x(fxp).fmul_m(lmin)))));
    wr::<f32>(this, 0x1DC, f32_of(x(t[1]).fsub_m(f32_of(fyp.fmul_m(lmin)))));
    wr::<f32>(this, 0x1E0, f32_of(x(t[2]).fsub(sz.fmul_m(lmin))));
    wr::<f32>(this, 0x1E4, f32_of(x(t[0]).fsub_m(f32_of(x(fxp).fmul_m(lz)))));
    wr::<f32>(this, 0x1E8, f32_of(x(t[1]).fsub_m(f32_of(fyp.fmul_m(lz)))));
    wr::<f32>(this, 0x1EC, f32_of(x(t[2]).fsub(x(sz32).fmul_m(lz))));

    // collision
    set_col_vars_vehicle(vt, g::<i32>(CAR_ZOOM).read());
    if g::<i32>(CAMERA_DIRECTION).read() == 3 {
        IGNORE_ENTITY.set(v);
        g::<u32>(EXTRA_ENTITIES_COUNT).write(0);
        camera_vehicle_mode_special_cases(the_camera(), v);
        if rd::<u8>(v, 0x429) & 4 != 0 {
            g::<u8>(CAM_BYTE_9655E5).write(1);
        }
        camera_col_det_and_react(source, vp(&mut t));
        improve_near_clip(the_camera(), v, GPtr::NULL, source, vp(&mut t));
        IGNORE_ENTITY.set(GPtr::NULL);
        round_source(source);
    }
    g::<u8>(CAM_DIRECTLY_BEHIND).write(0);
    g::<u8>(CAM_DIRECTLY_IN_FRONT).write(0);
    round_source(source);
    get_vectors_ready_for_rw(this);
    FOLLOW_CAR_TARGET.set(t.map(f32::to_bits));
}

/// The slope pitch of `Process_FollowCar_SA` (0x52512D-0x525364): with `R = normalise(fwd x (0,0,1))`
/// and `U = normalise(R x fwd)`, only when `U . front > 0`:
/// `Lrel = (T.z - pos.z) + GetHeightAboveRoad()`; `A = asin(|sin(beta - (heading - PI/2))|)`;
/// `Bc = atan2(cm.max.x, -cm.min.y)`; `Rr = A > Bc ? (1.2 + max.x) / cos(max(0, PI/2 - A))
/// : (1.5 - min.y) / cos(A)`; `Lup = cos(beta - (heading - PI/2)) * atan2(fwd.z, |fwd.xy|) +
/// atan2(Lrel, Rr * 1.2)`, plus the roll term for automobiles with more than one wheel on the ground
/// and `|turnSpeed . fwd| < 0.05`. The matrix pointer is used without a null check from 0x525263 on.
fn slope_pitch(this: GPtr<u8>, v: GPtr<u8>, t: &[f32; 3], lup: &mut f32) {
    let zero = x(k(K_ZERO));
    let beta = || rd::<f32>(this, 0xBC);
    let front = this.byte_add(0x190);
    let mut zup: [f32; 3] = [0.0, 0.0, 1.0];
    let mut rr: [f32; 3] = [0.0; 3];
    let mut uu: [f32; 3] = [0.0; 3];
    cross_product(vp(&mut rr), rd::<GPtr<u8>>(v, 0x14).byte_add(0x10), vp(&mut zup));
    vector_normalise(vp(&mut rr));
    cross_product(vp(&mut uu), vp(&mut rr), rd::<GPtr<u8>>(v, 0x14).byte_add(0x10));
    vector_normalise(vp(&mut uu));
    let ud = x(uu[1])
        .fmul_m(rd::<f32>(front, 4))
        .fadd(x(uu[2]).fmul_m(rd::<f32>(front, 8)))
        .fadd(x(uu[0]).fmul_m(rd::<f32>(front, 0)));
    if jne(ud, zero, 0x41) {
        return;
    }
    let cm = get_col_model(v);
    let h = X87::from_f64_exact(v_get_height_above_road(v));
    let m = rd::<GPtr<u8>>(v, 0x14);
    let p = if m.is_null() {
        v.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    let lrel = f32_of(x(t[2]).fsub_m(rd::<f32>(p, 8)).fadd(h));
    let s = X87::from_f64_exact(get_heading(v))
        .fsub_m(k(K_HPI))
        .fsubr_m(beta())
        .fsin()
        .fabs();
    let a = asin(s);
    let bc = x(rd::<f32>(cm, 0xC)).fpatan(x(rd::<f32>(cm, 4)).fchs());
    let rr_ = if !jne(a, bc, 0x41) {
        let num = f32_of(x(d(K_SLOPE_SIDE)).fadd_m(rd::<f32>(cm, 0xC)));
        let arg = f32_of(x(k(K_HPI)).fsub(a));
        X87::from_f64_exact(max_f(0, arg.to_bits())).fcos().fdivr_m(num)
    } else {
        let num = x(d(K_SLOPE_BACK)).fsub_m(rd::<f32>(cm, 4));
        num.fdiv(a.fcos())
    };
    let l30 = f32_of(rr_);
    let fy = rd::<f32>(m, 0x14);
    let fx = rd::<f32>(m, 0x10);
    let c1 = X87::from_f64_exact(get_heading(v))
        .fsub_m(k(K_HPI))
        .fsubr_m(beta())
        .fcos();
    let pitch = x(rd::<f32>(m, 0x18)).fpatan(x(fx).fmul_m(fx).fadd(x(fy).fmul_m(fy)).fsqrt());
    let at = x(lrel).fpatan(x(l30).fmul_m(d(K_SLOPE_DIST)));
    *lup = f32_of(c1.fmul(pitch).fadd(at));
    if rd::<i32>(v, 0x590) != 0 || rd::<u8>(v, 0x960) <= 1 {
        return;
    }
    let dp = X87::from_f64_exact(dot_product(v.byte_add(0x50), m.byte_add(0x10))).fabs();
    if jp(dp, x(k(K_0_05)), 0x05) {
        return;
    }
    let ry = rd::<f32>(m, 4);
    let rx = rd::<f32>(m, 0);
    let c2 = X87::from_f64_exact(get_heading(v))
        .fsub_m(k(K_HPI))
        .fsubr_m(beta())
        .fadd_m(k(K_HPI))
        .fcos();
    let roll = x(rd::<f32>(m, 8)).fpatan(x(rx).fmul_m(rx).fadd(x(ry).fmul_m(ry)).fsqrt());
    *lup = f32_of(c2.fmul(roll).fadd_m(*lup));
}

orig! {
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    pub fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    pub fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CPad::GetLookBehindForCar()` (0x53FE70, AL)
    pub fn get_look_behind_for_car(pad: GPtr<u8>) -> u8 = thiscall 0x53FE70 => unported;
    /// `CPad::GetLookBehindForPed()` (0x53FEC0, AL)
    pub fn get_look_behind_for_ped(pad: GPtr<u8>) -> u8 = thiscall 0x53FEC0 => unported;
    /// `CPad::GetLookLeft()` (0x53FDD0, AL)
    pub fn get_look_left(pad: GPtr<u8>) -> u8 = thiscall 0x53FDD0 => unported;
    /// `CPad::GetLookRight()` (0x53FE10, AL)
    pub fn get_look_right(pad: GPtr<u8>) -> u8 = thiscall 0x53FE10 => unported;
    /// `CCam::WellBufferMe(float target, float* value, float* speed, float topSpeed, float step, bool isAngle)`
    /// (0x509AE0)
    pub fn well_buffer_me(target: f32, value: GPtr<f32>, speed: GPtr<f32>, top_speed: f32, step: f32, is_angle: u8) = cdecl 0x509AE0 => crate::cam::well_buffer_me;
}

/// `TheCamera.m_bTransitionState` (+0x58) and `m_bUseTransitionBeta` (+0x37).
const TRANSITION_STATE: usize = 0xB6F080;
const USE_TRANSITION_BETA: usize = 0xB6F05F;

/// The inlined angle wrap of `RotCamIfInFrontCar`: while `d > pi` subtract 2 pi (only entered when
/// `d > pi`), then while `d < -pi` add 2 pi, on the x87 stack (`test ah,0x41` / `test ah,5`).
fn wrap_pi(mut d: X87) -> X87 {
    if !jne(d, x(k(K_PI)), 0x41) {
        loop {
            d = d.fsub_m(k(K_2PI));
            if jne(d, x(k(K_PI)), 0x41) {
                break;
            }
        }
    }
    if !jp(d, x(k(K_MPI)), 0x05) {
        loop {
            d = d.fadd_m(k(K_2PI));
            if jp(d, x(k(K_MPI)), 0x05) {
                break;
            }
        }
    }
    d
}

/// `this` is `TheCamera`'s active cam (`&m_aCams[m_nActiveCam]`, re-read at every test).
fn is_active(this: GPtr<u8>) -> bool {
    active_cam(the_camera()).addr() == this.addr()
}

/// `CCam::RotCamIfInFrontCar(const CVector& target, float orientation)` (0x50A4F0, thiscall, `ret 8`,
/// AL). Non-vehicles return 0. The vehicle's matrix is allocated when missing; `forward` =
/// `fwd . moveSpeed > 0.1` (z + y first); when `|moveSpeed.xy|^2 > 0.0036` the orientation argument
/// becomes `atan2(-v.x, v.y) - pi/2` (stored f32). `dist` = the flat source-target distance (f32);
/// `d = wrap(orientation - beta)`; `|d| > 0.349`, `forward` and no transition set `m_bFixingBeta` (+4).
/// No look button (behind for car / ped, left, right, each with its own `GetPad(0)`) and
/// `m_nDirectionWasLooking != 3` set `m_bCamDirectlyBehind`. Without fixing beta, transition beta,
/// directly behind and in front: return 0. Otherwise, when fixing beta or (one of the three flags `== 1`
/// and this is the active cam): `WellBufferMe(orientation, &beta, &betaSpeed, 0.1, 0.003, true)`, beta
/// from the flags (behind: the orientation; in front: + pi; transition: `m_fTransitionBeta` +0x9C),
/// `source.xy = target.xy - -(cos|sin(beta) * dist)`, and `|wrap(orientation - beta)| < 0.0349` clears
/// fixing beta. Then both direction flags are cleared and 1 returned.
#[hook(addr = 0x50A4F0, conv = thiscall, name = "CCam::RotCamIfInFrontCar", expect = "83 EC 10 55 56 8B F1", this = "CCam")]
pub fn rot_cam_if_in_front_car(this: GPtr<u8>, target: GPtr<u8>, orientation: u32) -> u8 {
    let v = rd::<GPtr<u8>>(this, 0x21C);
    if rd::<u8>(v, 0x36) & 7 != 2 {
        return 0;
    }
    if rd::<GPtr<u8>>(v, 0x14).is_null() {
        allocate_matrix(v);
        simple_transform_update_matrix(v.byte_add(4), rd::<GPtr<u8>>(v, 0x14));
    }
    let m = rd::<GPtr<u8>>(v, 0x14);
    let dot = x(rd::<f32>(m, 0x18))
        .fmul_m(rd::<f32>(v, 0x4C))
        .fadd(x(rd::<f32>(m, 0x14)).fmul_m(rd::<f32>(v, 0x48)))
        .fadd(x(rd::<f32>(m, 0x10)).fmul_m(rd::<f32>(v, 0x44)));
    let forward = !jne(dot, x(k(0x3DCC_CCCD)), 0x41);
    let (sx, sy) = (rd::<f32>(v, 0x44), rd::<f32>(v, 0x48));
    let mut ori = f32::from_bits(orientation);
    if !jne(x(sx).fmul_m(sx).fadd(x(sy).fmul_m(sy)), x(k(0x3B6B_EDFA)), 0x41) {
        ori = f32_of(x(sx).fchs().fpatan(x(sy)).fsub_m(k(K_HPI)));
    }
    let dx = x(rd::<f32>(this, 0x19C)).fsub_m(rd::<f32>(target, 0));
    let dy = x(rd::<f32>(this, 0x1A0)).fsub_m(rd::<f32>(target, 4));
    let dist = f32_of(dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt());
    let beta = || rd::<f32>(this, 0xBC);
    let d = wrap_pi(x(ori).fsub_m(beta()));
    if !jne(d.fabs(), x(k(0x3EB2_B8C0)), 0x41) && forward && g::<u8>(TRANSITION_STATE).read() == 0 {
        wr::<u8>(this, 4, 1);
    }
    let looking = get_look_behind_for_car(get_pad(0)) != 0
        || get_look_behind_for_ped(get_pad(0)) != 0
        || get_look_left(get_pad(0)) != 0
        || get_look_right(get_pad(0)) != 0;
    let behind = if !looking && rd::<i32>(this, 0x28) != 3 {
        g::<u8>(CAM_DIRECTLY_BEHIND).write(1);
        1
    } else {
        g::<u8>(CAM_DIRECTLY_BEHIND).read()
    };
    let fixing = rd::<u8>(this, 4);
    let in_front = g::<u8>(CAM_DIRECTLY_IN_FRONT).read();
    if fixing == 0 && g::<u8>(USE_TRANSITION_BETA).read() == 0 && behind == 0 && in_front == 0 {
        return 0;
    }
    let requested =
        (behind == 1 || in_front == 1 || g::<u8>(USE_TRANSITION_BETA).read() == 1) && is_active(this);
    if fixing != 0 || requested {
        let bp = this.byte_add(0xBC).cast::<f32>();
        well_buffer_me(
            ori,
            bp,
            this.byte_add(0xC0).cast::<f32>(),
            k(0x3DCC_CCCD),
            k(0x3B44_9BA6),
            1,
        );
        if g::<u8>(CAM_DIRECTLY_BEHIND).read() == 1 && is_active(this) {
            wr::<f32>(this, 0xBC, ori);
        }
        if g::<u8>(CAM_DIRECTLY_IN_FRONT).read() == 1 && is_active(this) {
            wr::<f32>(this, 0xBC, f32_of(x(ori).fadd_m(k(K_PI))));
        }
        if g::<u8>(USE_TRANSITION_BETA).read() != 0 && is_active(this) {
            wr::<u32>(this, 0xBC, rd::<u32>(this, 0x9C));
        }
        let a = x(beta()).fcos().fmul_m(dist).fchs();
        let b = x(beta()).fsin().fmul_m(dist).fchs();
        wr::<f32>(this, 0x19C, f32_of(x(rd::<f32>(target, 0)).fsub(a)));
        wr::<f32>(this, 0x1A0, f32_of(x(rd::<f32>(target, 4)).fsub(b)));
        let d2 = wrap_pi(x(ori).fsub_m(beta()));
        if !jp(d2.fabs(), x(k(0x3D0E_FA35)), 0x05) {
            wr::<u8>(this, 4, 0);
        }
    }
    g::<u8>(CAM_DIRECTLY_BEHIND).write(0);
    g::<u8>(CAM_DIRECTLY_IN_FRONT).write(0);
    1
}

orig! {
    /// `RwCameraSetNearClipPlane(RwCamera*, RwReal)` (0x7EE1D0)
    pub fn rw_camera_set_near_clip_plane(cam: GPtr<u8>, near: f32) -> GPtr<u8> = cdecl 0x7EE1D0 => unported;
    /// `CPedIntelligence::GetTaskSwim()` (0x601070)
    pub fn get_task_swim(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `CPedIntelligence::GetTaskHold(bool)` (0x600FF0)
    pub fn get_task_hold(intel: GPtr<u8>, ignore_check: u8) -> GPtr<u8> = thiscall 0x600FF0 => unported;
    /// `CCamera::HandleCameraMotionForDucking(CPed*, CVector* src, CVector* targ, bool instant)` (0x50CFA0)
    pub fn handle_camera_motion_for_ducking(cam: GPtr<u8>, ped: GPtr<u8>, src: GPtr<u8>, targ: GPtr<u8>, instant: u32) = thiscall 0x50CFA0 => crate::camfx::handle_camera_motion_for_ducking;
    /// `FUN_00509ca0(float* out)` (0x509CA0, AL): the boat look left/right/behind height
    pub fn boat_look_height(cam: GPtr<u8>, out: GPtr<u8>) -> u8 = thiscall 0x509CA0 => crate::camproc::get_boat_look_lr_behind_cam_height;
    /// `CCullZones::Cam1stPersonForPlayer()` (0x72DD40, AL)
    pub fn cam_1st_person_for_player() -> u8 = cdecl 0x72DD40 => unported;
    /// `CPed::SetPedPositionInCar()` (0x5DF910)
    pub fn ped_set_ped_position_in_car(p: GPtr<u8>) = thiscall 0x5DF910 => unported;
    /// `CEntity::UpdateRwMatrix()` (0x446F90)
    pub fn update_rw_matrix(e: GPtr<u8>) = thiscall 0x446F90 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    pub fn update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CEntity::UpdateRpHAnim()` (0x532B20)
    pub fn update_rp_hanim(e: GPtr<u8>) = thiscall 0x532B20 => unported;
    /// `CPed::GetBonePosition(RwV3d& out, uint32 bone, bool updateSkin)` (0x5E4280)
    pub fn ped_get_bone_position(p: GPtr<u8>, out: GPtr<u8>, bone: u32, update: u32) = thiscall 0x5E4280 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    pub fn get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `operator*(CVector* out, float s, const CVector& v)` (0x40FE90), returns `out`
    pub fn vec_s_mul(out: GPtr<u8>, s: u32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00)
    pub fn vector_add_assign(v: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> = thiscall 0x411A00 => unported;
}

/// `Scene.m_pRwCamera`
const SCENE_CAMERA: usize = 0xC1703C;
/// `TheCamera.m_nPedZoom` (i32), `m_fPedZoomSmoothed`
const PED_ZOOM: usize = 0xB6F0F0;
const PED_ZOOM_SMOOTHED: usize = 0xB6F0FC;
/// `TheCamera.m_pExtraEntity[]` (after the count 0xB6FC70)
const EXTRA_ENTITIES: usize = 0xB6FC74;

/// The entity position: `&matrix->pos` (+0x30) or `&placement.pos` (+4).
fn pos_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// The inlined `GetMatrix()`: allocates the matrix from the placement when missing.
fn ensure_matrix(e: GPtr<u8>) {
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
}

/// Dword copy of a 3-word vector.
fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for o in [0, 4, 8] {
        wr::<u32>(dst, o, rd::<u32>(src, o));
    }
}

/// `v[i] = f32(a[i] - b[i])` for x, y, z (each `fld; fsub; fstp` through a local).
fn sub3(v: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) {
    let r = [0isize, 4, 8].map(|o| f32_of(x(rd::<f32>(a, o)).fsub_m(rd::<f32>(b, o))));
    for (i, f) in r.into_iter().enumerate() {
        wr::<f32>(v, 4 * i as isize, f);
    }
}

/// `src += dir * s` as the bytes do it: x and y products kept on the stack, z stored f32 first.
fn add_scaled(src: GPtr<u8>, dir: GPtr<u8>, s: f32) {
    let px = x(rd::<f32>(dir, 0)).fmul_m(s);
    let py = x(rd::<f32>(dir, 4)).fmul_m(s);
    let pz = f32_of(x(rd::<f32>(dir, 8)).fmul_m(s));
    wr::<f32>(src, 0, f32_of(px.fadd_m(rd::<f32>(src, 0))));
    wr::<f32>(src, 4, f32_of(py.fadd_m(rd::<f32>(src, 4))));
    wr::<f32>(src, 8, f32_of(x(pz).fadd_m(rd::<f32>(src, 8))));
}

/// `src -= dir * s` (x and y `fsub st(i)`, z the f32-stored product).
fn sub_scaled(src: GPtr<u8>, dir: GPtr<u8>, s: f32) {
    let px = x(rd::<f32>(dir, 0)).fmul_m(s);
    let py = x(rd::<f32>(dir, 4)).fmul_m(s);
    let pz = f32_of(x(rd::<f32>(dir, 8)).fmul_m(s));
    wr::<f32>(src, 0, f32_of(x(rd::<f32>(src, 0)).fsub(px)));
    wr::<f32>(src, 4, f32_of(x(rd::<f32>(src, 4)).fsub(py)));
    wr::<f32>(src, 8, f32_of(x(rd::<f32>(src, 8)).fsub_m(pz)));
}

/// `v = -v` (each `fld; fchs; fstp`).
fn negate3(v: GPtr<u8>) {
    let r = [0isize, 4, 8].map(|o| f32_of(x(rd::<f32>(v, o)).fchs()));
    for (i, f) in r.into_iter().enumerate() {
        wr::<f32>(v, 4 * i as isize, f);
    }
}

/// `CCam::LookBehind()` (0x520690, thiscall, AL). Three cases by mode (+0xC, a word) and target type:
/// - car (modes 0x12 / 0x16 / 3, vehicle target): `m_bLookingBehind` (+7); the point is the last
///   `Process_FollowCar_SA` target 0xB6F018; source = point + (fwd + (0,0,0.2)) * dist (dist =
///   `m_fCaMaxDistance` in mode 0x12, else 15.5), with the collision passes (ignore entity, special
///   cases, `CameraColDetAndReact`, front = pos - source, `ImproveNearClip`).
/// - first person in a vehicle (mode 0x10): near clip 0.05; front = normalised fwd; boats lower the
///   source by 0.5; appearance 2 (bike): source += front * 2.3, front reversed, vectors for RW;
///   appearance 3: front = up * -1, up = fwd, source += front * 0.25; others: source += front * 0.25,
///   front reversed.
/// - ped: source direction `(-cos beta, -sin beta, 0)` lifted by the ped's +0x578 vector and 0.3,
///   normalised, scaled by `max(2.0 + m_fPedZoomSmoothed, 0.6)` from the ped position; swimming pulls it
///   back; the per-zoom heights; ducking, the held entity as an extra entity, the collision flags
///   0x8A5B14 / 15 / 17, `CameraColDetAndReact`, front, `ImproveNearClip`, near clip 0.05 at zoom 1.
///
/// Every case starts with `front = pos - source` and ends with `GetVectorsReadyForRW`; returns 1, or 0
/// (nothing changed) when no case applies.
#[hook(addr = 0x520690, conv = thiscall, name = "CCam::LookBehind", expect = "83 EC 3C 53 57 8B F9", this = "CCam")]
pub fn look_behind(this: GPtr<u8>) -> u8 {
    let tgt = || rd::<GPtr<u8>>(this, 0x21C);
    let mode = rd::<u16>(this, 0xC);
    let car = (mode == 0x12 || mode == 0x16 || mode == 3) && rd::<u8>(tgt(), 0x36) & 7 == 2;
    let e = tgt();
    let ty = rd::<u8>(e, 0x36) & 7;
    let ped = ty == 3;
    let first = mode == 0x10 && ty == 2;
    if !car && !first && !ped {
        return 0;
    }
    let pp = pos_of(e);
    let mut p: [f32; 3] = [rd::<f32>(pp, 0), rd::<f32>(pp, 4), rd::<f32>(pp, 8)];
    let source = this.byte_add(0x19C);
    let front = this.byte_add(0x190);
    sub3(front, pos_of(e), source);
    if car {
        p = FOLLOW_CAR_TARGET.get().map(f32::from_bits);
        wr::<u8>(this, 7, 1);
        let dist = if mode == 0x12 {
            rd::<f32>(this, 0xD0)
        } else {
            k(0x4178_0000)
        };
        ensure_matrix(e);
        copy3(source, rd::<GPtr<u8>>(e, 0x14).byte_add(0x10));
        wr::<f32>(source, 8, f32_of(x(rd::<f32>(source, 8)).fadd_m(k(0x3E4C_CCCD))));
        let ax = x(dist).fmul_m(rd::<f32>(source, 0));
        let ay = x(dist).fmul_m(rd::<f32>(source, 4));
        let l48 = f32_of(x(dist).fmul_m(rd::<f32>(source, 8)));
        wr::<f32>(source, 0, f32_of(x(p[0]).fadd(ax)));
        wr::<f32>(source, 4, f32_of(x(p[1]).fadd(ay)));
        let mut c = p;
        wr::<f32>(source, 8, f32_of(x(l48).fadd_m(p[2])));
        IGNORE_ENTITY.set(tgt());
        g::<u32>(EXTRA_ENTITIES_COUNT).write(0);
        camera_vehicle_mode_special_cases(the_camera(), tgt());
        camera_col_det_and_react(source, vp(&mut c));
        sub3(front, pos_of(tgt()), source);
        get_vectors_ready_for_rw(this);
        improve_near_clip(the_camera(), tgt(), GPtr::NULL, source, vp(&mut c));
        IGNORE_ENTITY.set(GPtr::NULL);
    }
    if first {
        wr::<u8>(this, 7, 1);
        rw_camera_set_near_clip_plane(g::<GPtr<u8>>(SCENE_CAMERA).read(), k(0x3D4C_CCCD));
        let e = tgt();
        ensure_matrix(e);
        copy3(front, rd::<GPtr<u8>>(e, 0x14).byte_add(0x10));
        vector_normalise(front);
        let e = tgt();
        if rd::<i32>(e, 0x590) == 5 {
            wr::<f32>(source, 8, f32_of(x(rd::<f32>(source, 8)).fsub_m(k(K_HALF))));
        }
        if get_vehicle_appearance(e) == 2 {
            add_scaled(source, front, k(0x4013_3333));
            negate3(front);
            get_vectors_ready_for_rw(this);
        } else if get_vehicle_appearance(tgt()) == 3 {
            let e = tgt();
            ensure_matrix(e);
            let up = rd::<GPtr<u8>>(e, 0x14).byte_add(0x20);
            let e = tgt();
            let r = [0isize, 4, 8].map(|o| f32_of(x(rd::<f32>(up, o)).fmul_m(k(K_M1))));
            for (i, f) in r.into_iter().enumerate() {
                wr::<f32>(front, 4 * i as isize, f);
            }
            ensure_matrix(e);
            copy3(this.byte_add(0x1B4), rd::<GPtr<u8>>(e, 0x14).byte_add(0x10));
            add_scaled(source, front, k(0x3E80_0000));
        } else {
            add_scaled(source, front, k(0x3E80_0000));
            negate3(front);
        }
    }
    if ped {
        wr::<f32>(source, 0, f32_of(x(rd::<f32>(this, 0xBC)).fcos().fchs()));
        wr::<f32>(source, 4, f32_of(x(rd::<f32>(this, 0xBC)).fsin().fchs()));
        wr::<u32>(source, 8, 0);
        let e = tgt();
        let dd = x(rd::<f32>(e, 0x580))
            .fmul_m(rd::<f32>(source, 8))
            .fadd(x(rd::<f32>(e, 0x57C)).fmul_m(rd::<f32>(source, 4)))
            .fadd(x(rd::<f32>(e, 0x578)).fmul_m(rd::<f32>(source, 0)));
        wr::<f32>(
            source,
            8,
            f32_of(dd.fsubr_m(rd::<f32>(source, 8)).fadd_m(k(0x3E99_999A))),
        );
        vector_normalise(source);
        let mut zz = x(d(0x8CCE4C)).fadd_m(d(PED_ZOOM_SMOOTHED));
        if !jne(x(k(0x3F19_999A)), zz, 0x41) {
            zz = x(k(0x3F19_999A));
        }
        let zx = zz.fmul_m(rd::<f32>(source, 0));
        let l38 = f32_of(zz.fmul_m(rd::<f32>(source, 4)));
        let l3c = f32_of(zz.fmul_m(rd::<f32>(source, 8)));
        wr::<f32>(source, 0, f32_of(zx.fadd_m(p[0])));
        wr::<f32>(source, 4, f32_of(x(l38).fadd_m(p[1])));
        wr::<f32>(source, 8, f32_of(x(l3c).fadd_m(p[2])));
        let idx = (g::<i32>(PED_ZOOM).read() as u32).wrapping_mul(4).wrapping_sub(4);
        let tab = |base: u32| base.wrapping_add(idx) as usize;
        let l14 = g::<u32>(tab(0x8CCE40)).read();
        let l18 = g::<u32>(tab(0x8CCE34)).read();
        if !get_task_swim(rd::<GPtr<u8>>(tgt(), 0x47C)).is_null() {
            let a = x(d(tab(0x8CCE28)));
            let bv = x(d(tab(0x8CCE1C)));
            let dx = x(p[0]).fsub_m(rd::<f32>(source, 0));
            let dy = x(p[1]).fsub_m(rd::<f32>(source, 4));
            let l48 = f32_of(x(p[2]).fsub_m(rd::<f32>(source, 8)));
            let l34 = f32_of(dx.fmul(a));
            let dya = dy.fmul(a);
            let dza = x(l48).fmul(a);
            wr::<f32>(source, 0, f32_of(x(l34).fadd_m(p[0])));
            wr::<f32>(source, 4, f32_of(dya.fadd_m(p[1])));
            wr::<f32>(source, 8, f32_of(dza.fadd_m(p[2])));
            wr::<f32>(source, 8, f32_of(bv.fadd_m(rd::<f32>(source, 8))));
        }
        wr::<f32>(
            source,
            8,
            f32_of(x(f32::from_bits(l14)).fadd_m(rd::<f32>(source, 8))),
        );
        p[2] = f32_of(x(p[2]).fadd_m(f32::from_bits(l18)));
        handle_camera_motion_for_ducking(the_camera(), tgt(), source, vp(&mut p), 0);
        let e = tgt();
        g::<u32>(EXTRA_ENTITIES_COUNT).write(0);
        if !e.is_null() {
            let h = get_task_hold(rd::<GPtr<u8>>(e, 0x47C), 0);
            if !h.is_null() {
                let held = rd::<u32>(h, 8);
                if held != 0 {
                    let n = g::<u32>(EXTRA_ENTITIES_COUNT).read();
                    g::<u32>((EXTRA_ENTITIES as u32).wrapping_add(n.wrapping_mul(4)) as usize).write(held);
                    g::<u32>(EXTRA_ENTITIES_COUNT).write(n.wrapping_add(1));
                }
            }
        }
        g::<u8>(0x8A5B14).write(1);
        g::<u8>(0x8A5B15).write(1);
        g::<u8>(0x8A5B17).write(1);
        camera_col_det_and_react(source, vp(&mut p));
        sub3(front, vp(&mut p), source);
        get_vectors_ready_for_rw(this);
        improve_near_clip(the_camera(), GPtr::NULL, tgt(), source, vp(&mut p));
        if g::<i32>(PED_ZOOM).read() == 1 {
            let cam = g::<GPtr<u8>>(SCENE_CAMERA).read();
            if !jne(x(rd::<f32>(cam, 0x80)), x(k(K_0_05)), 0x41) {
                rw_camera_set_near_clip_plane(cam, k(0x3D4C_CCCD));
            }
        }
    }
    get_vectors_ready_for_rw(this);
    1
}

/// `CCam::LookRight(bool right)` (0x520E40, thiscall, `ret 4`, AL; the mode test goes through the
/// `.text` fragment 0x4012B0 that loads the target). Sets `m_bLookingLeft` (+8) or `m_bLookingRight`
/// (+9) first, then:
/// - car (modes 0x12 / 0x16 / 3, vehicle target): dist = `m_fCaMaxDistance` (0x12) or 9.0; in mode
///   0x16 the boat look height lifts the source (unless `Cam1stPersonForPlayer`); source.xy =
///   pos.xy + (cos|sin)(atan(fwd.xy) + dir * pi/2) * dist (dir = +-1); the collision passes; the source
///   height is limited to `max(min(oldZ, max(colTop, fudge.z) + 0.1), current)`; front = pos - source,
///   front.z + 1.1 (+ 1.2 more in mode 0x16), vectors for RW.
/// - first person in a vehicle (mode 0x10): near clip [0x8CCE50]; boats with a driver take the driver's
///   bone 4 + right * (0.7 | 0.3) + up * 0.2 as the source, boats without one lower it by 0.5; non-bikes
///   move it back by right * 0.35; up = normalised up axis, front = normalised `fwd x up` (`up x fwd`
///   for left); appearance 2 moves the source back by front * 1.45.
/// - ped: nothing more. Returns 1, or 0 when no case applies.
#[hook(addr = 0x520E40, conv = thiscall, name = "CCam::LookRight", expect = "83 EC 34 53 55 56 8B F1", this = "CCam")]
pub fn look_right(this: GPtr<u8>, right: u32) -> u8 {
    let tgt = || rd::<GPtr<u8>>(this, 0x21C);
    let right = right as u8 != 0;
    let mode = rd::<u16>(this, 0xC);
    let car = (mode == 0x12 || mode == 0x16 || mode == 3) && rd::<u8>(tgt(), 0x36) & 7 == 2;
    let e = tgt();
    let ty = rd::<u8>(e, 0x36) & 7;
    let ped = ty == 3;
    let first = mode == 0x10 && ty == 2;
    let dir = if right {
        wr::<u8>(this, 9, 1);
        k(K_ONE)
    } else {
        wr::<u8>(this, 8, 1);
        k(K_M1)
    };
    let source = this.byte_add(0x19C);
    let front = this.byte_add(0x190);
    if car {
        let pp = pos_of(e);
        let p: [f32; 3] = [rd::<f32>(pp, 0), rd::<f32>(pp, 4), rd::<f32>(pp, 8)];
        let dist = if mode == 0x12 {
            rd::<f32>(this, 0xD0)
        } else {
            if mode == 0x16 {
                let mut h: f32 = 0.0;
                if boat_look_height(this, GPtr::<f32>::from_raw(&mut h).cast::<u8>()) != 0
                    && cam_1st_person_for_player() == 0
                {
                    wr::<f32>(source, 8, f32_of(x(p[2]).fadd_m(h)));
                }
            }
            k(0x4110_0000)
        };
        let e = tgt();
        ensure_matrix(e);
        let mut fw: [f32; 3] = [0.0; 3];
        copy3(vp(&mut fw), rd::<GPtr<u8>>(e, 0x14).byte_add(0x10));
        vector_normalise(vp(&mut fw));
        let at = X87::from_f64_exact(get_atan_of_xy(fw[0].to_bits(), fw[1].to_bits()));
        let a = at.fadd(x(dir).fmul_m(k(K_HPI)));
        wr::<f32>(source, 0, f32_of(a.fcos().fmul_m(dist).fadd_m(p[0])));
        wr::<f32>(source, 4, f32_of(a.fsin().fmul_m(dist).fadd_m(p[1])));
        let cm = get_col_model(tgt());
        let old_z = rd::<f32>(source, 8);
        let mut c = p;
        IGNORE_ENTITY.set(tgt());
        g::<u32>(EXTRA_ENTITIES_COUNT).write(0);
        camera_vehicle_mode_special_cases(the_camera(), tgt());
        camera_col_det_and_react(source, vp(&mut c));
        IGNORE_ENTITY.set(GPtr::NULL);
        let e = tgt();
        let pz = rd::<f32>(pos_of(e), 8);
        let side = if right {
            ensure_matrix(e);
            x(rd::<f32>(rd::<GPtr<u8>>(e, 0x14), 8)).fmul_m(rd::<f32>(cm, 0))
        } else {
            ensure_matrix(e);
            x(rd::<f32>(cm, 0xC)).fmul_m(rd::<f32>(rd::<GPtr<u8>>(e, 0x14), 8))
        };
        let e = tgt();
        let l28 = f32_of(side.fadd_m(pz));
        ensure_matrix(e);
        let h = x(rd::<f32>(cm, 0x14))
            .fmul_m(rd::<f32>(rd::<GPtr<u8>>(e, 0x14), 0x28))
            .fadd_m(l28);
        let fudge_z = || x(rd::<f32>(this, 0x150));
        let m = if jp(h, fudge_z(), 0x05) { h } else { fudge_z() };
        let mut v = if !jp(x(old_z), x(k(0x3DCC_CCCD)).fadd(m), 0x05) {
            x(old_z)
        } else {
            let mx = if jp(h, fudge_z(), 0x05) { h } else { fudge_z() };
            mx.fadd_m(k(0x3DCC_CCCD))
        };
        if jne(v, x(rd::<f32>(source, 8)), 0x41) {
            v = x(rd::<f32>(source, 8));
        }
        wr::<f32>(source, 8, f32_of(v));
        sub3(front, pos_of(tgt()), source);
        let fz = x(rd::<f32>(front, 8)).fadd_m(k(0x3F8C_CCCD));
        wr::<f32>(front, 8, f32_of(fz));
        if mode == 0x16 {
            wr::<f32>(front, 8, f32_of(fz.fadd_m(k(0x3F99_999A))));
        }
        get_vectors_ready_for_rw(this);
        return 1;
    }
    if first {
        if right {
            wr::<u8>(this, 9, 1);
        } else {
            wr::<u8>(this, 8, 1);
        }
        rw_camera_set_near_clip_plane(g::<GPtr<u8>>(SCENE_CAMERA).read(), d(0x8CCE50));
        let e = tgt();
        if rd::<i32>(e, 0x590) == 5 {
            let drv = rd::<GPtr<u8>>(e, 0x460);
            if !drv.is_null() {
                let mut bp: [f32; 3] = [0.0; 3];
                ped_set_ped_position_in_car(drv);
                update_rw_matrix(drv);
                update_rw_frame(drv);
                update_rp_hanim(drv);
                ped_get_bone_position(drv, vp(&mut bp), 4, 1);
                let m = get_matrix(tgt());
                let s = if right {
                    g::<u32>(0x8CC498).read()
                } else {
                    g::<u32>(0x8CC49C).read()
                };
                let mut tmp: [f32; 3] = [0.0; 3];
                let r = vec_s_mul(vp(&mut tmp), s, m);
                vector_add_assign(vp(&mut bp), r);
                let m2 = get_matrix(tgt());
                let ux = x(d(0x8CC494)).fmul_m(rd::<f32>(m2, 0x20));
                let uy = x(d(0x8CC494)).fmul_m(rd::<f32>(m2, 0x24));
                let l40 = f32_of(x(d(0x8CC494)).fmul_m(rd::<f32>(m2, 0x28)));
                bp[0] = f32_of(ux.fadd_m(bp[0]));
                wr::<f32>(source, 0, bp[0]);
                bp[1] = f32_of(uy.fadd_m(bp[1]));
                wr::<f32>(source, 4, bp[1]);
                bp[2] = f32_of(x(l40).fadd_m(bp[2]));
                wr::<f32>(source, 8, bp[2]);
            } else {
                wr::<f32>(source, 8, f32_of(x(rd::<f32>(source, 8)).fsub_m(k(K_HALF))));
            }
        }
        let e = tgt();
        if rd::<i32>(e, 0x590) != 9 {
            ensure_matrix(e);
            sub_scaled(source, rd::<GPtr<u8>>(e, 0x14), k(0x3EB3_3333));
        }
        let up = this.byte_add(0x1B4);
        let e = tgt();
        ensure_matrix(e);
        copy3(up, rd::<GPtr<u8>>(e, 0x14).byte_add(0x20));
        vector_normalise(up);
        let e = tgt();
        ensure_matrix(e);
        copy3(front, rd::<GPtr<u8>>(e, 0x14).byte_add(0x10));
        vector_normalise(front);
        let mut tmp: [f32; 3] = [0.0; 3];
        let r = if right {
            cross_product(vp(&mut tmp), front, up)
        } else {
            cross_product(vp(&mut tmp), up, front)
        };
        copy3(front, r);
        vector_normalise(front);
        if get_vehicle_appearance(tgt()) == 2 {
            sub_scaled(source, front, k(0x3FB9_999A));
        }
        return 1;
    }
    u8::from(ped)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_PROCESS_FOLLOW_CAR_SA,
    HOOK_ROT_CAM_IF_IN_FRONT_CAR,
    HOOK_LOOK_BEHIND,
    HOOK_LOOK_RIGHT,
];
