//! PORT-CAM-2: small CAM helpers found by the UNK classification (bone-tag map, decimal rounding,
//! member constructors, cull-zone centre), the camera destructors, and mode processors ported by the
//! PORT-CAM-2 agent itself. Ported from the 1.0 US bytes; `docs/re/specs/CAM/*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, k, rd, the_camera, wr, x};

/// `_CIpow` (0x822130: `st1 ^ st0`, both operands and the result on the x87 stack)
const CI_POW: usize = 0x822130;

orig! {
    /// CRT `modf(double, double*)` (0x8232F0), result (the fraction) in ST0
    pub fn crt_modf(v: f64, ip: GPtr<f64>) -> f64 = cdecl 0x8232F0 => unported;
    /// `round to n decimals` (0x50A0A0, cdecl, ST0); the float travels as its bit pattern
    pub fn round_decimals_call(v: u32, n: i32) -> f64 = cdecl 0x50A0A0 => unported;
    /// CRT `eh vector destructor iterator` (0x821E02, stdcall)
    pub fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    pub fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CCamera::~CCamera()` (0x50A870)
    pub fn camera_dtor_call(this: GPtr<u8>) = thiscall 0x50A870 => camera_dtor;
    /// `CPlaceable::~CPlaceable()` (0x54F490)
    pub fn placeable_dtor(p: GPtr<u8>) = thiscall 0x54F490 => unported;
    /// `operator delete` (0x8214BD, the CRT `free` the scalar deleting destructors call)
    pub fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730), returns `out`
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `CCam::GetVectorsReadyForRW()` (0x509CE0)
    pub fn get_vectors_ready_for_rw(cam: GPtr<u8>) = thiscall 0x509CE0 => crate::cam::get_vectors_ready_for_rw;
    /// `CWaterLevel::GetWaterLevel(float x, float y, float z, float* level, bool waves, CVector* normal)`
    /// (0x6EB690, AL); the coordinates are pushed as dwords
    pub fn get_water_level(xb: u32, yb: u32, zb: u32, level: GPtr<f32>, waves: u8, normal: GPtr<u8>) -> u8 = cdecl 0x6EB690 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `cHandlingDataMgr::GetBoatPointer(int32)` (0x6F5300, thiscall on `gHandlingDataMgr` 0xC2B9C8)
    pub fn get_boat_pointer(mgr: GPtr<u8>, id: i32) -> GPtr<u8> = thiscall 0x6F5300 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CTask::GetSubTask()` (vtable +0x8)
    fn v_get_sub_task(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x8 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPad::AimWeaponLeftRight(CPed*)` (0x541040, AX)
    pub fn aim_weapon_left_right(pad: GPtr<u8>, ped: GPtr<u8>) -> i16 = thiscall 0x541040 => unported;
    /// `CPad::AimWeaponUpDown(CPed*)` (0x5410C0, AX)
    pub fn aim_weapon_up_down(pad: GPtr<u8>, ped: GPtr<u8>) -> i16 = thiscall 0x5410C0 => unported;
    /// `CPad::GetAccelerate()` (0x5403F0, AX)
    pub fn get_accelerate(pad: GPtr<u8>) -> i16 = thiscall 0x5403F0 => unported;
    /// `CPad::GetBrake()` (0x540080, AX)
    pub fn get_brake(pad: GPtr<u8>) -> i16 = thiscall 0x540080 => unported;
    /// `CIdleCam::ProcessIdleCamTicker()` (0x50A200)
    pub fn idle_process_ticker(this: GPtr<u8>) = thiscall 0x50A200 => crate::idlecam::process_idle_cam_ticker;
    /// `CIdleCam::Reset(bool)` (0x50A160)
    pub fn idle_reset(this: GPtr<u8>, reset_controls: u8) = thiscall 0x50A160 => crate::idlecam::reset;
    /// `CIdleCam::SetTarget(CEntity*)` (0x50A280)
    pub fn idle_set_target(this: GPtr<u8>, e: GPtr<u8>) = thiscall 0x50A280 => idle_cam_set_target;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CIdleCam::Run()` (0x51D3E0)
    pub fn idle_run(this: GPtr<u8>) = thiscall 0x51D3E0 => idle_cam_run;
    /// `CIdleCam::ProcessTargetSelection()` (0x517870)
    pub fn idle_process_target_selection(this: GPtr<u8>) = thiscall 0x517870 => unported;
    /// `CIdleCam::ProcessSlerp(float& rotZ, float& rotY)` (0x5179E0, ST0)
    pub fn idle_process_slerp(this: GPtr<u8>, a: GPtr<f32>, b: GPtr<f32>) -> f64 = thiscall 0x5179E0 => unported;
    /// `CIdleCam::ProcessFOVZoom(float)` (0x517BF0); the float pushed as a dword
    pub fn idle_process_fov_zoom(this: GPtr<u8>, t: u32) = thiscall 0x517BF0 => unported;
    /// `CIdleCam::FinaliseIdleCamera(float, float, float)` (0x50E760); dwords
    pub fn idle_finalise(this: GPtr<u8>, a: u32, b: u32, c: u32) = thiscall 0x50E760 => unported;
    /// `CCamera::GetScreenRect(CRect*)` (0x50AB50)
    pub fn get_screen_rect(this: GPtr<u8>, r: GPtr<u8>) = thiscall 0x50AB50 => crate::camctl::get_screen_rect;
    /// `RwRenderStateSet(RwRenderState, void*)` (`RwEngineInstance` +0x20)
    fn rw_render_state_set(state: u32, v: u32) -> u32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CRGBA::CRGBA(r, g, b, a)` (0x7170C0)
    pub fn rgba_ctor(c: GPtr<u8>, r: u32, gr: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    pub fn sprite_draw_rect(r: GPtr<u8>, c: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    pub fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CCamera::ConeCastCollisionResolve(const CVector&, const CVector&, CVector&, float, float, float&)`
    /// (0x51A5D0, AL); radius and angle are pushed as immediates
    pub fn cone_cast_collision_resolve(this: GPtr<u8>, src: GPtr<u8>, tgt: GPtr<u8>, out: GPtr<u8>, radius: u32, angle: u32, dist: GPtr<f32>) -> u8 = thiscall 0x51A5D0 => unported;
}

/// `TheCamera.m_bUseSpecialFovTrain` (0xB6F05E) and `m_fFovForTrain` (0xB6F158).
const USE_SPECIAL_FOV_TRAIN: usize = 0xB6F05E;
const FOV_FOR_TRAIN: usize = 0xB6F158;
/// `CTimeCycle::m_CurrentColours` water red / green / blue (0xB7C508 / 0xB7C50C / 0xB7C510).
const WATER_R: usize = 0xB7C508;
const WATER_G: usize = 0xB7C50C;
const WATER_B: usize = 0xB7C510;
/// `TheCamera` under-water colour (+0x84 b, +0x88 g, +0x8C r), its alpha-ish word (+0xA0) and the
/// under-water flag (+0x90 = 2).
const UW_B: usize = 0xB6F0AC;
const UW_G: usize = 0xB6F0B0;
const UW_R: usize = 0xB6F0B4;
const UW_MODE: usize = 0xB6F0B8;
const UW_A: usize = 0xB6F0C8;
/// `.data` tweak values: the colour magnitude cap 0x8CC7A8 (10.0) and the alpha 0x8CC7A4 (20).
const UW_COLOUR_CAP: usize = 0x8CC7A8;
const UW_ALPHA: usize = 0x8CC7A4;
/// Byte enabling the fixed camera's collision push (0xB6EC2C).
const FIXED_CAM_COLLISION: usize = 0xB6EC2C;
/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8) and `gHandlingDataMgr` (0xC2B9C8)
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
const HANDLING_DATA_MGR: usize = 0xC2B9C8;
/// `CWorld::pIgnoreEntity` (0xB7CD68)
const IGNORE_ENTITY: usize = 0xB7CD68;

/// `ConvertPedNode2BoneTag(int32 node)` (0x4D58A0, cdecl): the jump table 0x4D5920 over `node - 1`
/// (0..17), -1 outside it.
#[hook(addr = 0x4D58A0, conv = cdecl, name = "ConvertPedNode2BoneTag", expect = "8B 4C 24 04 83 C8 FF")]
pub fn convert_ped_node_2_bone_tag(node: i32) -> i32 {
    const TAGS: [i32; 18] = [
        3, 5, 0x20, 0x16, 0x22, 0x18, 0x29, 0x33, 0x2B, 0x35, 0x34, 0x2A, 0x21, 0x17, 0x1F, 0x15, 4, 8,
    ];
    match TAGS.get(node.wrapping_sub(1) as u32 as usize) {
        Some(&t) => t,
        None => -1,
    }
}

/// `FUN_0050a0a0` (0x50A0A0, cdecl, ST0): rounds `v` to `n` decimals:
/// `p = _CIpow(10.0, n + 1); t = (v * p (+|-) 5.0) * 0.1` (`+` when `v >= 0` or unordered:
/// `fcomp 0.0; test ah, 5; jp`), `modf(t, &ip)`, result `ip / _CIpow(10.0, n)` (kept on the stack).
/// The two powers come back from `_CIpow` through `fstp qword` (exact for the integral powers of ten
/// up to 10^22; a negative `n + 1` would round the 64-bit result to 53 bits first). The writes to its
/// own argument slots (`fst [esp+0x18]`) are not observable.
#[hook(addr = 0x50A0A0, conv = cdecl, name = "FUN_0050a0a0", expect = "83 EC 08 8B 44 24 10")]
pub fn round_decimals(v: u32, n: i32) -> X87 {
    let vf = f32::from_bits(v);
    let p = sa_core::orig::x87_binary(CI_POW, 10.0, f64::from(n.wrapping_add(1)));
    let a = X87::from_f64_exact(p).fmul_m(vf);
    // fld v; fcomp [0.0]
    let t = if x(vf).fcom(x(k(0))).test_parity(0x5) {
        a.fadd_m(k(0x40A0_0000))
    } else {
        a.fsub_m(k(0x40A0_0000))
    }
    .fmul_m(k(0x3DCC_CCCD));
    let mut ip = [0f64; 1];
    // fstp qword [esp]: t holds 24 bits, exact in a double
    crt_modf(t.to_f64(), GPtr::from_raw(ip.as_mut_ptr()));
    let q = sa_core::orig::x87_binary(CI_POW, 10.0, f64::from(n));
    // fld qword ip; fld 10; fild n; call _CIpow; fdivp st(1), st
    X87::from_f64_exact(ip[0]).fdiv(X87::from_f64_exact(q))
}

/// `FUN_0050a120` (0x50A120, cdecl): `v->x/y/z = FUN_0050a0a0(v->x/y/z, n)` (each pushed as a dword,
/// stored with `fstp dword`).
#[hook(addr = 0x50A120, conv = cdecl, name = "FUN_0050a120", expect = "56 8B 74 24 08")]
pub fn round_vector_decimals(v: GPtr<u8>, n: i32) {
    for i in 0..3 {
        let r = round_decimals_call(rd::<u32>(v, 4 * i), n);
        wr::<f32>(v, 4 * i, r as f32);
    }
}

/// `FUN_0050e180` (0x50E180, thiscall): the DW heli-chase tuning block's defaults; EAX = 0 on return.
#[hook(addr = 0x50E180, conv = thiscall, name = "FUN_0050e180", expect = "B8 00 00 48 42", preserve = "ecx")]
pub fn heli_chase_settings_init(this: GPtr<u8>) -> u32 {
    for (o, v) in [
        (0x18, 0x4248_0000u32),
        (0x24, 0x4248_0000),
        (0x1C, 0x41F0_0000),
        (0x20, 0x425C_0000),
        (0x28, 0x3D4C_CCCD),
        (0x2C, 0x428C_0000),
        (0x30, 0x41B0_0000),
        (0x34, 0x3F80_0000),
        (0x38, 0),
        (0x3C, 0x4120_0000),
        (0x48, 0x1E),
        (0x58, 8),
        (0x60, 0x3C),
        (0x68, 0x42C8_0000),
        (0x6C, 0x42DC_0000),
        (0x70, 0x4120_0000),
        (0x74, 0x40A0_0000),
        (0x78, 0x4140_0000),
        (0x94, 0x3F40_0000),
        (0x98, 0xFA0),
    ] {
        wr::<u32>(this, o, v);
    }
    for o in [0x40, 0x5C, 0x80, 0x81] {
        wr::<u8>(this, o, 0);
    }
    wr::<u32>(this, 0x7C, 0x4240_0000);
    wr::<u32>(this, 0x64, 0x3C);
    wr::<u32>(this, 0x44, 0x1E);
    wr::<u8>(this, 0x88, 0);
    0
}

/// `(missed by ghidra: data-code-immediate)` 0x50E470 (thiscall, the 0x10-byte entries at
/// `CCamera+0xCF4`): `+0 = -1, +4 = +8 = 1.0, +0xC = 0`; returns `this`.
#[hook(addr = 0x50E470, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "8B C1 B9 00 00 80 3F")]
pub fn cam_entry_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<i32>(this, 0, -1);
    wr::<u32>(this, 4, 0x3F80_0000);
    wr::<u32>(this, 8, 0x3F80_0000);
    wr::<u32>(this, 0xC, 0);
    this
}

/// `(missed by ghidra: data-code-immediate)` 0x5097B0 (thiscall, the CCamPathSplines constructor):
/// `+0 = 0`; returns `this`.
#[hook(addr = 0x5097B0, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "8B C1 C7 00 00 00 00 00")]
pub fn cam_path_splines_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    this
}

/// `FUN_0072d6d0` (0x72D6D0, thiscall on a cull zone, `ret 4`): the zone centre from its int16 words
/// (`x = (w8 + w4) * 0.5 + w0`, `y = (wA + w6) * 0.5 + w2`, `z = (wC + wE) * 0.5`, the z sum in an
/// integer register, the others with `fiadd`); returns `out`. Every intermediate value is a small
/// integer or half-integer, exact in 24 bits.
#[hook(addr = 0x72D6D0, conv = thiscall, name = "FUN_0072d6d0", expect = "51 0F BF 51 0C")]
pub fn cull_zone_centre(this: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> {
    let w = |o: isize| i32::from(rd::<i16>(this, o));
    let half = k(0x3F00_0000);
    let z = X87::from_i32(w(0xC) + w(0xE)).fmul_m(half);
    let y = X87::from_i32(w(0xA))
        .fadd(X87::from_i32(w(0x6)))
        .fmul_m(half)
        .fadd(X87::from_i32(w(0x2)));
    let xx = X87::from_i32(w(0x8))
        .fadd(X87::from_i32(w(0x4)))
        .fmul_m(half)
        .fadd(X87::from_i32(w(0x0)));
    wr::<f32>(out, 0, f32_of(xx));
    wr::<f32>(out, 4, f32_of(y));
    wr::<f32>(out, 8, f32_of(z));
    out
}

/// `CCamera::~CCamera()` (0x50A870): vtable 0x8630E8, `+0x14 = 0`, the queued entries at +0xCF4
/// (5 x 0x10, dtor 0x509800), the six CMatrix members (+0xADC .. +0x974, reverse order), the
/// CCamPathSplines (+0x960, 4 x 4, dtor 0x5097C0), the three CCams (+0x174, 0x238, dtor 0x5097D0),
/// then the CPlaceable base destructor.
#[hook(addr = 0x50A870, conv = thiscall, name = "CCamera::~CCamera", expect = "6A FF 68 3F C7 83 00")]
pub fn camera_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x8630E8);
    wr::<u32>(this, 0x14, 0);
    eh_vector_dtor(this.byte_add(0xCF4), 0x10, 5, 0x509800);
    for o in [0xADC, 0xA94, 0xA4C, 0xA04, 0x9BC, 0x974] {
        matrix_dtor(this.byte_add(o));
    }
    eh_vector_dtor(this.byte_add(0x960), 4, 4, 0x5097C0);
    eh_vector_dtor(this.byte_add(0x174), 0x238, 3, 0x5097D0);
    placeable_dtor(this);
}

/// `CCamera`'s scalar deleting destructor (0x514010, thiscall, `ret 4`): `~CCamera()`, then
/// `operator delete(this)` when bit 0 of the flags is set; returns `this`.
#[hook(addr = 0x514010, conv = thiscall, name = "CCamera", expect = "56 8B F1 E8 58 68 FF FF")]
pub fn camera_scalar_deleting_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    camera_dtor_call(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `__dtor_gHandShaker` (0x8563B0): `eh vector destructor iterator(gHandShaker 0xB6ECA0, 0x94, 6,
/// CHandShaker::~CHandShaker 0x509810)`.
#[hook(addr = 0x8563B0, conv = cdecl, name = "__dtor_gHandShaker", expect = "68 10 98 50 00")]
pub fn dtor_g_hand_shaker() {
    eh_vector_dtor(GPtr::from_addr(0xB6ECA0), 0x94, 6, 0x509810);
}

/// `__dtor_preMirrorMat` (0x8563E0): `preMirrorMat.~CMatrix()` (tail jump).
#[hook(addr = 0x8563E0, conv = cdecl, name = "__dtor_preMirrorMat", expect = "B9 40 FE B6 00")]
pub fn dtor_pre_mirror_mat() {
    matrix_dtor(GPtr::from_addr(0xB6FE40));
}

/// `__dtor_g_b6fec0` (0x8563F0): `eh vector destructor iterator(0xB6FEC0, 0x9C, 1, 0x50E080)`.
#[hook(addr = 0x8563F0, conv = cdecl, name = "__dtor_g_b6fec0", expect = "68 80 E0 50 00")]
pub fn dtor_g_b6fec0() {
    eh_vector_dtor(GPtr::from_addr(0xB6FEC0), 0x9C, 1, 0x50E080);
}

/// `__dtor_TheCamera` (0x856410): `TheCamera.~CCamera()` (tail jump).
#[hook(addr = 0x856410, conv = cdecl, name = "__dtor_TheCamera", expect = "B9 28 F0 B6 00")]
pub fn dtor_the_camera() {
    camera_dtor_call(the_camera());
}

/// The three-dword CVector copy.
fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for i in 0..3 {
        wr::<u32>(dst, 4 * i, rd::<u32>(src, 4 * i));
    }
}

/// `CCam::Process_Fixed(const CVector& target, float, float, float)` (0x51D470, `ret 0x10`).
///
/// Mode +0x28 = 3; source (+0x19C) = the fixed position (+0x160); front (+0x190) = target - source
/// (f32 per axis) normalised; +0x148 = target; `GetVectorsReadyForRW()`; up (+0x1B4) = (0, 0, 1) plus
/// the fixed up offset (+0x16C, operand order of the bytes), normalised, then
/// `up = Normalise(front x up) x front`; FOV 70 (the train FOV when `TheCamera` +0x36 == 1). Under
/// water (`GetWaterLevel` found a level above the source): `TheCamera`'s under-water colour from the
/// timecycle water colour, scaled to the 0x8CC7A8 cap when its magnitude exceeds it (`_ftol` each),
/// alpha 0x8CC7A4, mode 2. With 0xB6EC2C set, the source is pushed out of the world by
/// `ConeCastCollisionResolve(source, target copy, out, 2.0, 0.1, dist = 1.0)` with
/// `pIgnoreEntity = FindPlayerVehicle(-1, false)` (restored afterwards).
#[hook(addr = 0x51D470, conv = thiscall, name = "CCam::Process_Fixed", expect = "83 EC 28 53 55 56 57", this = "CCam")]
pub fn process_fixed(this: GPtr<u8>, target: GPtr<u8>, _f1: u32, _f2: u32, _f3: u32) {
    if rd::<i32>(this, 0x28) != 3 {
        wr::<i32>(this, 0x28, 3);
    }
    let src = this.byte_add(0x19C);
    let front = this.byte_add(0x190);
    let up = this.byte_add(0x1B4);
    copy3(src, this.byte_add(0x160));
    for i in 0..3 {
        let d = f32_of(x(rd::<f32>(target, 4 * i)).fsub_m(rd::<f32>(src, 4 * i)));
        wr::<f32>(front, 4 * i, d);
    }
    vector_normalise(front);
    copy3(this.byte_add(0x148), target);
    get_vectors_ready_for_rw(this);
    wr::<u32>(up, 0, 0);
    wr::<u32>(up, 4, 0);
    wr::<u32>(up, 8, 0x3F80_0000);
    // fld up.x; fadd [+0x16C] / fld [+0x170]; fadd up.y / fld [+0x174]; fadd up.z
    wr::<f32>(up, 0, f32_of(x(rd::<f32>(up, 0)).fadd_m(rd::<f32>(this, 0x16C))));
    wr::<f32>(up, 4, f32_of(x(rd::<f32>(this, 0x170)).fadd_m(rd::<f32>(up, 4))));
    wr::<f32>(up, 8, f32_of(x(rd::<f32>(this, 0x174)).fadd_m(rd::<f32>(up, 8))));
    vector_normalise(up);
    // locals: [esp+0x10] dist, [esp+0x14] target copy, [esp+0x20] cross / cone-cast out, [esp+0x2c] right
    let mut right = [0u32; 3];
    let mut cross = [0u32; 3];
    let rp = GPtr::from_raw(right.as_mut_ptr()).cast::<u8>();
    let cp = GPtr::from_raw(cross.as_mut_ptr()).cast::<u8>();
    cross_product(rp, front, up);
    vector_normalise(rp);
    let r = cross_product(cp, rp, front);
    copy3(up, r);
    wr::<u32>(this, 0xB4, 0x428C_0000);
    if g::<u8>(USE_SPECIAL_FOV_TRAIN).read() == 1 {
        wr::<u32>(this, 0xB4, g::<u32>(FOV_FOR_TRAIN).read());
    }
    let mut level = [0f32; 1];
    let found = get_water_level(
        rd::<u32>(src, 0),
        rd::<u32>(src, 4),
        rd::<u32>(src, 8),
        GPtr::from_raw(level.as_mut_ptr()),
        1,
        GPtr::from_addr(0),
    );
    // fld src.z; fcomp level; test ah, 5; jp: below the surface only when src.z < level
    if found != 0 && !x(rd::<f32>(src, 8)).fcom(x(level[0])).test_parity(0x5) {
        let (wr_, wg, wb) = (
            g::<f32>(WATER_R).read(),
            g::<f32>(WATER_G).read(),
            g::<f32>(WATER_B).read(),
        );
        let mag = x(wb)
            .fmul_m(wb)
            .fadd(x(wg).fmul_m(wg))
            .fadd(x(wr_).fmul_m(wr_))
            .fsqrt();
        let cap = g::<f32>(UW_COLOUR_CAP).read();
        // fcom cap; test ah, 0x41; jne: scale only when mag > cap (ordered)
        if !mag.fcom(x(cap)).test_any(0x41) {
            let s = mag.fdivr_m(cap);
            g::<i32>(UW_R).write(ftol32(x(wr_).fmul(s)));
            g::<i32>(UW_G).write(ftol32(x(wg).fmul(s)));
            g::<i32>(UW_B).write(ftol32(x(wb).fmul(s)));
        } else {
            g::<i32>(UW_R).write(ftol32(x(wr_)));
            g::<i32>(UW_G).write(ftol32(x(wg)));
            g::<i32>(UW_B).write(ftol32(x(wb)));
        }
        g::<u32>(UW_A).write(g::<u32>(UW_ALPHA).read());
        g::<i32>(UW_MODE).write(2);
    }
    if g::<u8>(FIXED_CAM_COLLISION).read() != 0 {
        let saved = g::<u32>(IGNORE_ENTITY).read();
        let mut dist = [1.0f32; 1];
        let mut tgt = [0u32; 3];
        let tp = GPtr::from_raw(tgt.as_mut_ptr()).cast::<u8>();
        copy3(tp, target);
        let v = find_player_vehicle(-1, 0);
        g::<GPtr<u8>>(IGNORE_ENTITY).write(v);
        if cone_cast_collision_resolve(
            the_camera(),
            src,
            tp,
            cp,
            0x4000_0000,
            0x3DCC_CCCD,
            GPtr::from_raw(dist.as_mut_ptr()),
        ) != 0
        {
            copy3(src, cp);
        }
        g::<u32>(IGNORE_ENTITY).write(saved);
    }
}

/// `FUN_00509ca0` (0x509CA0, thiscall on a CCam, `ret 4`, AL): the boat "look left/right behind"
/// camera height of the target entity's handling (`*out = boat->+0x20`, a dword copy) and 1; 0 without
/// a target or boat handling. No vehicle-type check (any target's model info +0x4A is used).
#[hook(addr = 0x509CA0, conv = thiscall, name = "FUN_00509ca0", expect = "8B 81 1C 02 00 00", this = "CCam")]
pub fn get_boat_look_lr_behind_cam_height(this: GPtr<u8>, out: GPtr<u8>) -> u8 {
    let e = rd::<GPtr<u8>>(this, 0x21C);
    if e.is_null() {
        return 0;
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS)
        .wrapping_index(i32::from(rd::<i16>(e, 0x22)))
        .read();
    let b = get_boat_pointer(GPtr::from_addr(HANDLING_DATA_MGR), i32::from(rd::<i16>(mi, 0x4A)));
    if b.is_null() {
        return 0;
    }
    wr::<u32>(out, 0, rd::<u32>(b, 0x20));
    1
}

/// An `.rdata` f32 (constant tables and literals read by address, as the bytes do).
fn rc(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// `FUN_0063a380` (0x63A380, thiscall on a CTaskComplexEnterCarAsDriver, `ret 0x18`): the camera stick
/// modifier while the player gets into a car (only caller `CCam::Process_FollowPed_SA`).
/// Args: `ped`, `dist`, `alpha*`, `beta*`, `stickAlpha*`, `stickBeta*`; `this+0xC` the vehicle, `this+0x29`
/// the stick mode (3 aiming, 2 accelerate, 1 brake; kept between calls). See
/// `docs/re/specs/CAM/CCam.md` (0x63A380); the x87 sequence follows the bytes (FPATAN in sa-math's f64
/// model).
#[hook(addr = 0x63A380, conv = thiscall, name = "FUN_0063a380", expect = "83 EC 08 56 8B F1")]
pub fn enter_car_camera_stick_modifier(
    this: GPtr<u8>,
    ped: GPtr<u8>,
    dist: f32,
    alpha: GPtr<f32>,
    beta: GPtr<f32>,
    stick_alpha: GPtr<f32>,
    stick_beta: GPtr<f32>,
) {
    let v = rd::<GPtr<u8>>(this, 0xC);
    if v.is_null() || rd::<i32>(v, 0x594) == 3 {
        return;
    }
    let mut pad = get_pad(0);
    if rd::<i32>(ped, 0x598) == 1 {
        pad = get_pad(1);
    }
    if v_get_sub_task(this).is_null() {
        return;
    }
    let mut ok = false;
    for t in [0x322, 0x322, 0x324, 0x334, 0x327, 0x328] {
        if v_get_task_type(v_get_sub_task(this)) == t {
            ok = true;
            break;
        }
    }
    if !ok {
        return;
    }
    // fild; fabs; fcomp 20.0; test ah, 0x41; je -> aiming
    let big = |v: i16| {
        !X87::from_i32(i32::from(v).abs())
            .fcom(x(rc(0x858BA4)))
            .test_any(0x41)
    };
    if big(aim_weapon_left_right(pad, ped)) || big(aim_weapon_up_down(pad, ped)) {
        wr::<u8>(this, 0x29, 3);
    } else if get_accelerate(pad) != 0 {
        wr::<u8>(this, 0x29, 2);
    } else if get_brake(pad) != 0 {
        wr::<u8>(this, 0x29, 1);
    }
    if rd::<u8>(this, 0x29) == 3 {
        return;
    }
    let m = rd::<GPtr<u8>>(v, 0x14);
    let h = if !m.is_null() {
        // fld up.x; fchs; fld up.y; fpatan
        x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)))
    } else {
        x(rd::<f32>(v, 0x10))
    };
    let h0 = f32_of(h.fsub_m(rc(0x858FE4)));
    let vp = if m.is_null() {
        v.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    let pp = if pm.is_null() {
        ped.byte_add(4)
    } else {
        pm.byte_add(0x30)
    };
    let d = |i: isize| x(rd::<f32>(pp, 4 * i)).fsub_m(rd::<f32>(vp, 4 * i));
    let (dx, dy, dz) = (d(0), d(1), d(2));
    // (dz * right.z + dy * right.y) + dx * right.x, right read from the vehicle matrix (`edi`)
    let lat_st = dz
        .fmul_m(rd::<f32>(m, 8))
        .fadd(dy.fmul_m(rd::<f32>(m, 4)))
        .fadd(dx.fmul_m(rd::<f32>(m, 0)));
    let lat = f32_of(lat_st);
    // fld dist; fpatan: atan2(lat, dist) with the unrounded lat
    let a = f32_of(lat_st.fpatan(x(dist)).fmul_m(rc(0x86E690)));
    let cm = get_col_model(v);
    let mut t = x(a);
    // fld -bbox.min.y; fcomp dist; test ah, 0x41; jne
    if !x(rd::<f32>(cm, 4)).fchs().fcom(x(dist)).test_any(0x41) {
        // fcomp 0.0; fld 8 deg; ...; jne skips the fchs
        let k8 = x(rc(0x86E694));
        t = if x(a).fcom(x(rc(0x858B50))).test_any(0x41) {
            k8
        } else {
            k8.fchs()
        };
    }
    let mode = rd::<u8>(this, 0x29);
    t = if mode == 1 {
        // fld h0; fadd pi; faddp st(1), st
        t.fadd(x(h0).fadd_m(rc(0x858CB8)))
    } else {
        // fsubr h0
        t.fsubr_m(h0)
    };
    let b = beta.read();
    // fld b; fadd pi; fld st(1); fcompp; test ah, 0x41; jne
    if !t.fcom(x(b).fadd_m(rc(0x858CB8))).test_any(0x41) {
        t = t.fsub_m(rc(0x858CBC));
    } else if !t.fcom(x(b).fsub_m(rc(0x858CB8))).test_parity(0x5) {
        t = t.fadd_m(rc(0x858CBC));
    }
    t = t.fsub_m(beta.read());
    let lat_pos = !x(lat).fcom(x(rc(0x858B50))).test_any(0x41);
    let negate = if lat_pos == (mode == 1) {
        // 0x63A5DB: fcom -pi/2; test ah, 0x41; jp skips
        !t.fcom(x(rc(0x859998))).test_parity(0x41)
    } else {
        // 0x63A610: fcom pi/2; test ah, 1; jne skips
        !t.fcom(x(rc(0x858FE4))).test_any(0x1)
    };
    if negate {
        t = t.fmul_m(rc(0x858C1C));
    }
    let lim_at = 0x86E684 + 4 * usize::from(mode);
    let gain_at = 0x86E678 + 4 * usize::from(mode);
    if !t.fcom(x(rc(lim_at))).test_any(0x41) {
        t = x(rc(lim_at));
    } else {
        // fld lim; fchs; fstp [dist slot]; fcom it; test ah, 5; jp skips
        let nl = f32_of(x(rc(lim_at)).fchs());
        if !t.fcom(x(nl)).test_parity(0x5) {
            t = x(nl);
        }
    }
    let sb = stick_beta.read();
    stick_beta.write(f32_of(t.fmul_m(rc(gain_at)).fadd_m(sb)));
    if !x(alpha.read()).fcom(x(g::<f32>(0x8D2ECC).read())).test_any(0x41) {
        let gm = 0x86E678 + 4 * usize::from(rd::<u8>(this, 0x29));
        let lm = 0x86E684 + 4 * usize::from(rd::<u8>(this, 0x29));
        let sa = stick_alpha.read();
        stick_alpha.write(f32_of(x(rc(gm)).fmul_m(rc(lm)).fsubr_m(sa)));
    }
}

/// `(float)CTimer::m_snTimeInMilliseconds`: `fild` of the u32 as an int, `+ 2^32` (0x858C54) when
/// negative, stored as f32.
fn time_ms_f32() -> f32 {
    let t = sa_core::timer::TIME_MS.get() as i32;
    let v = X87::from_i32(t);
    f32_of(if t < 0 { v.fadd_m(k(0x4F80_0000)) } else { v })
}

/// `CIdleCam::Process()` (0x522C80, AL): `ProcessIdleCamTicker()`; nothing (0) until the ticker
/// (+0x94, int) exceeds the threshold (+0x28; `fild; fcomp; test ah, 0x41; jne`). Then +0x98 = the
/// active cam; when the previous frame (+0x30) is older than `FrameCounter - 1` (unsigned) the idle cam
/// starts: bit 0 of 0xC0B184 set, `Reset(false)`, start time +0x2C = now (f32), target = the player
/// ped, +0x78 = 1. +0x30 = `FrameCounter`, `Run()`, 0xB6EC40 = +0x30; 1.
#[hook(addr = 0x522C80, conv = thiscall, name = "CIdleCam::Process", expect = "51 56 8B F1 E8 77 75 FE FF", this = "CIdleCam")]
pub fn idle_cam_process(this: GPtr<u8>) -> u8 {
    idle_process_ticker(this);
    if X87::from_i32(rd::<i32>(this, 0x94))
        .fcom(x(rd::<f32>(this, 0x28)))
        .test_any(0x41)
    {
        return 0;
    }
    let now = time_ms_f32();
    let cam = 0xB6F19C + 0x238 * u32::from(g::<u8>(0xB6F081).read());
    wr::<u32>(this, 0x98, cam);
    let frame = sa_core::timer::FRAME_COUNTER.get();
    if rd::<u32>(this, 0x30) < frame.wrapping_sub(1) {
        let f = g::<u8>(0xC0B184);
        f.write(f.read() | 1);
        idle_reset(this, 0);
        wr::<f32>(this, 0x2C, now);
        let ped = find_player_ped(-1);
        idle_set_target(this, ped);
        wr::<u8>(this, 0x78, 1);
    }
    wr::<u32>(this, 0x30, sa_core::timer::FRAME_COUNTER.get());
    idle_run(this);
    g::<u32>(0xB6EC40).write(rd::<u32>(this, 0x30));
    1
}

/// `CIdleCam::Run()` (0x51D3E0): `ProcessTargetSelection()`; progress = `(now - start) / duration`
/// (+0x2C, +0x8C; 1.0 when `now - start >= duration` or unordered: `test ah, 5; jp`);
/// `r = ProcessSlerp(&b, &a)` stored to +0x24; `ProcessFOVZoom(r)`; `FinaliseIdleCamera(b, a, progress)`.
#[hook(addr = 0x51D3E0, conv = thiscall, name = "CIdleCam::Run", expect = "A1 84 CB B7 00", this = "CIdleCam")]
pub fn idle_cam_run(this: GPtr<u8>) {
    let now = time_ms_f32();
    idle_process_target_selection(this);
    let d = x(now).fsub_m(rd::<f32>(this, 0x2C));
    let dur = rd::<f32>(this, 0x8C);
    let progress = if d.fcom(x(dur)).test_parity(0x5) {
        1.0f32
    } else {
        f32_of(d.fdiv_m(dur))
    };
    // locals [esp+0xc] (second argument) and [esp+0x10] (first), both written by ProcessSlerp
    let mut a = [0f32; 1];
    let mut b = [0f32; 1];
    let r = idle_process_slerp(
        this,
        GPtr::from_raw(b.as_mut_ptr()),
        GPtr::from_raw(a.as_mut_ptr()),
    );
    let r = r as f32;
    wr::<f32>(this, 0x24, r);
    idle_process_fov_zoom(this, r.to_bits());
    idle_finalise(this, b[0].to_bits(), a[0].to_bits(), progress.to_bits());
}

/// `CCamera::DrawBordersForWideScreen()` (0x514860): `GetScreenRect(&r)` on a local whose +4 / +0xC
/// are pre-set to -1e6 / 1e6; letterbox modes 0 and 2 (+0x90) set +0xA4 = 80;
/// `RwRenderStateSet(1, 0)`; two black rectangles (`CRGBA(0, 0, 0, 255)`):
/// `(-5, -5, w + 5, r.top)` and `(-5, r.bottom, w + 5, h + 5)` (w / h = `RsGlobal` maximum width / height,
/// `fild`).
#[hook(addr = 0x514860, conv = thiscall, name = "CCamera::DrawBordersForWideScreen", expect = "83 EC 24 8D 44 24 14", this = "CCamera")]
pub fn draw_borders_for_wide_screen(this: GPtr<u8>) {
    let mut r = [0u32; 4];
    r[1] = 0xC974_2400;
    r[3] = 0x4974_2400;
    let rp = GPtr::from_raw(r.as_mut_ptr()).cast::<u8>();
    get_screen_rect(this, rp);
    let mode = rd::<i32>(this, 0x90);
    if mode == 0 || mode == 2 {
        wr::<i32>(this, 0xA4, 0x50);
    }
    rw_render_state_set(1, 0);
    let w5 = f32_of(X87::from_i32(g::<i32>(0xC17044).read().wrapping_add(5)));
    let mut col = [0u32; 1];
    let cp = GPtr::from_raw(col.as_mut_ptr()).cast::<u8>();
    let mut q = [0xC0A0_0000u32, 0xC0A0_0000, w5.to_bits(), r[1]];
    let qp = GPtr::from_raw(q.as_mut_ptr()).cast::<u8>();
    let c = rgba_ctor(cp, 0, 0, 0, 0xFF);
    sprite_draw_rect(qp, c);
    let h5 = f32_of(X87::from_i32(g::<i32>(0xC17048).read().wrapping_add(5)));
    let w5 = f32_of(X87::from_i32(g::<i32>(0xC17044).read().wrapping_add(5)));
    q = [0xC0A0_0000, r[3], w5.to_bits(), h5.to_bits()];
    let qp = GPtr::from_raw(q.as_mut_ptr()).cast::<u8>();
    let c = rgba_ctor(cp, 0, 0, 0, 0xFF);
    sprite_draw_rect(qp, c);
}

orig! {
    /// `CCam` mode processors taking `(const CVector& target, float orientation, float speedVar,
    /// float speedVarWanted)` (the floats pushed as dwords)
    pub fn cam_follow_ped_with_mouse(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32) = thiscall 0x50F970 => unported;
    pub fn cam_m16_1st_person(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32) = thiscall 0x5105C0 => unported;
    pub fn cam_aim_weapon(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32) = thiscall 0x521500 => unported;
    pub fn cam_wheel_cam(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32) = thiscall 0x512110 => unported;
    pub fn cam_fixed(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32) = thiscall 0x51D470 => unported;
    pub fn cam_special_fixed_for_syphon(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32) = thiscall 0x517500 => unported;
    pub fn cam_1st_person(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32) = thiscall 0x517EA0 => unported;
    pub fn cam_fly_by(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32) = thiscall 0x5B25F0 => unported;
    pub fn cam_editor(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32) = thiscall 0x50F3F0 => unported;
    pub fn cam_1rst_person_ped_on_pc(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32) = thiscall 0x50EB70 => unported;
    /// ... and the ones with a trailing bool
    pub fn cam_follow_car_sa(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32, b: u32) = thiscall 0x5245B0 => unported;
    pub fn cam_follow_ped_sa(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32, b: u32) = thiscall 0x522D40 => unported;
    pub fn cam_rocket(c: GPtr<u8>, t: GPtr<u8>, o: u32, sv: u32, svw: u32, b: u32) = thiscall 0x511B50 => unported;
    /// `CCam` mode processors without arguments
    pub fn cam_peds_dead_baby(c: GPtr<u8>) = thiscall 0x519250 => unported;
    pub fn cam_two_player(c: GPtr<u8>) = thiscall 0x525E50 => unported;
    pub fn cam_two_player_in_car_and_shooting(c: GPtr<u8>) = thiscall 0x519810 => unported;
    pub fn cam_two_player_separate_cars(c: GPtr<u8>) = thiscall 0x513510 => unported;
    pub fn cam_two_player_separate_cars_top_down(c: GPtr<u8>) = thiscall 0x513BE0 => unported;
    pub fn cam_arrest_cam_one(c: GPtr<u8>) -> u8 = thiscall 0x518500 => unported;
    pub fn cam_attached_cam(c: GPtr<u8>) = thiscall 0x512B10 => unported;
    /// `CCam` DW processors `(bool)`
    pub fn cam_dw_heli_chase(c: GPtr<u8>, b: u32) = thiscall 0x51A740 => unported;
    pub fn cam_dw_cam_man(c: GPtr<u8>, b: u32) = thiscall 0x51B120 => unported;
    pub fn cam_dw_birdy(c: GPtr<u8>, b: u32) = thiscall 0x51B850 => unported;
    pub fn cam_dw_plane_spotter(c: GPtr<u8>, b: u32) = thiscall 0x51C250 => unported;
    pub fn cam_dw_plane_cam1(c: GPtr<u8>, b: u32) = thiscall 0x51C760 => unported;
    pub fn cam_dw_plane_cam2(c: GPtr<u8>, b: u32) = thiscall 0x51CC30 => unported;
    pub fn cam_dw_plane_cam3(c: GPtr<u8>, b: u32) = thiscall 0x51D100 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    pub fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CWeaponEffects::ClearCrossHairImmediately(int32)` (0x742CA0)
    pub fn clear_cross_hair_immediately(n: i32) = cdecl 0x742CA0 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    pub fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, on the placement)
    pub fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CGeneral::GetATanOfXY(float, float)` (0x53CC70, ST0); dwords
    pub fn get_atan_of_xy_bits(xb: u32, yb: u32) -> f64 = cdecl 0x53CC70 => unported;
    /// `CPad::GetLookBehindForCar()` / `GetLookLeft()` / `GetLookRight()` / `GetLookBehindForPed()` (AL)
    pub fn pad_look_behind_for_car(p: GPtr<u8>) -> u8 = thiscall 0x53FE70 => unported;
    pub fn pad_look_left(p: GPtr<u8>) -> u8 = thiscall 0x53FDD0 => unported;
    pub fn pad_look_right(p: GPtr<u8>) -> u8 = thiscall 0x53FE10 => unported;
    pub fn pad_look_behind_for_ped(p: GPtr<u8>) -> u8 = thiscall 0x53FEC0 => unported;
    /// `CPedIntelligence::GetTaskClimb()` (0x601180) / `GetTaskFighting()` (0x600F30)
    pub fn get_task_climb(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601180 => unported;
    pub fn get_task_fighting(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F30 => unported;
    /// `CTaskSimpleClimb::GetCameraTargetPos(CPed*, CVector&)` (0x67A390)
    pub fn climb_get_camera_target_pos(t: GPtr<u8>, ped: GPtr<u8>, out: GPtr<u8>) = thiscall 0x67A390 => unported;
    /// `CCam::Using3rdPersonMouseCam()` (0x50A850, AL)
    pub fn using_3rd_person_mouse_cam(c: GPtr<u8>) -> u8 = thiscall 0x50A850 => crate::cam::using_3rd_person_mouse_cam;
    /// `out = s * v` (0x40FE90), `out = v * s` (0x40FEC0), `out = a + b` (0x40FE30),
    /// `out = v * (1 / s)` (0x4119D0); each returns `out`
    pub fn vec_s_mul(out: GPtr<u8>, s: u32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    pub fn vec_mul_s(out: GPtr<u8>, v: GPtr<u8>, s: u32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    pub fn vec_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    pub fn vec_div_s(out: GPtr<u8>, v: GPtr<u8>, s: u32) -> GPtr<u8> = cdecl 0x4119D0 => unported;
    /// `a > b ? a : b` (0x420800, cdecl floats, ST0); dwords
    pub fn max_f(a: u32, b: u32) -> f64 = cdecl 0x420800 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    pub fn get_vehicle_appearance(veh: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CCam::KeepTrackOfTheSpeed(...)` (0x509DF0)
    pub fn keep_track_of_the_speed(c: GPtr<u8>, s: GPtr<u8>, t: GPtr<u8>, u: GPtr<u8>, a: GPtr<f32>, b: GPtr<f32>, f: GPtr<f32>) = thiscall 0x509DF0 => crate::cam::keep_track_of_the_speed;
    /// `CCam::LookBehind()` (0x520690) / `CCam::LookRight(bool)` (0x520E40)
    pub fn cam_look_behind(c: GPtr<u8>) = thiscall 0x520690 => unported;
    pub fn cam_look_right(c: GPtr<u8>, right: u32) = thiscall 0x520E40 => unported;
}

/// `CPlaceable::GetMatrix()` inlined: allocate and fill the matrix from the placement when missing.
fn entity_matrix(e: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    rd::<GPtr<u8>>(e, 0x14)
}

/// The entity position (`matrix + 0x30`, else the placement `+4`).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `CCam::Process`' orientation of the target entity (`slot` = `&m_pCamTargetEntity`), re-reading the
/// entity and its matrix (inlined `GetMatrix`) at every use as the bytes do: 0 when the forward vector's
/// x and y are both exactly 0 (`fcomp 0.0; test ah, 0x44; jp`), else `GetATanOfXY(fwd.x, fwd.y)`.
fn target_orientation(slot: GPtr<u8>) -> u32 {
    let m = entity_matrix(slot.cast::<GPtr<u8>>().read());
    if !x(rd::<f32>(m, 0x10)).fcom(x(0.0)).test_parity(0x44) {
        let m = entity_matrix(slot.cast::<GPtr<u8>>().read());
        if !x(rd::<f32>(m, 0x14)).fcom(x(0.0)).test_parity(0x44) {
            return 0;
        }
    }
    let m1 = entity_matrix(slot.cast::<GPtr<u8>>().read());
    let m2 = entity_matrix(slot.cast::<GPtr<u8>>().read());
    (get_atan_of_xy_bits(rd::<u32>(m2, 0x10), rd::<u32>(m1, 0x14)) as f32).to_bits()
}

/// A local CVector (three dwords) and its pointer.
fn v3() -> [u32; 3] {
    [0; 3]
}
fn vp(v: &mut [u32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

/// `TheCamera` fields and other globals `CCam::Process` uses (see `docs/re/specs/CAM/CCam.md`).
const IDLE_COUNTER: usize = 0xB6FE34;
const IDLE_THRESHOLD: usize = 0xB6FDC8;
const ACTIVE_CAM_IDX: usize = 0xB6F081;
const CAMS_ABS: u32 = 0xB6F19C;
const CAMERA_TARGET: usize = 0xB6F980;
const CROSSHAIR_FLAG: usize = 0xC8A860;
const S_PED_POS: usize = 0x8CCC3C;
const S_PED_VEL: usize = 0xB6EC7C;
const G_CAMERA_DIRECTION: usize = 0x8CC384;
const G_CAMERA_MODE: usize = 0x8CC388;
const G_DW_CAM_MODE: usize = 0x8CC488;
const G_CUR_DIST_FOR_CAM: usize = 0x8CCB84;
const TRANSITION_STATE: usize = 0xB6F080;
const DOING_SPECIAL_INTERP: usize = 0xB6F077;
const WAIT_FOR_INTERPOL: usize = 0xB6F062;
const JUST_SWITCHED: usize = 0xB6F052;
const RESET_OLD_MATRIX: usize = 0xB6F999;
const CAM_DIRECTLY_BEHIND: usize = 0xB6F042;
const CAM_DIRECTLY_IN_FRONT: usize = 0xB6F043;
const PLAYER_IS_IN_GARAGE: usize = 0xB6F04E;
const JUST_CAME_OUT_OF_GARAGE: usize = 0xB6F050;
const USE_NEAR_CLIP_SCRIPT: usize = 0xB6F059;
const FOV_LERP_PROCESSED: usize = 0xB6FCDD;
const FOV_NEW: usize = 0xB6FCE0;
const VEC_MOVE_LINEAR_PROCESSED: usize = 0xB6FD14;
const VEC_MOVE_LINEAR: usize = 0xB6FD08;
const VEC_TRACK_LINEAR_PROCESSED: usize = 0xB6FCB4;
const VEC_TRACK_LINEAR: usize = 0xB6FCA8;
const USE_MOUSE_3RD_PERSON: usize = 0xB6EC2E;
const MOUSE_FOLLOW_PED_OFF: usize = 0x8CCF00;
const SPEED_FX_USER_FLAG: usize = 0x8D5109;
const FIRST_PERSON_RUN_THIS_FRAME: usize = 0xB6EC20;
/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
/// `.data` smoothing bases 0.6 / 0.8 / 0.9
const SMOOTH_POS: usize = 0x8CC394;
const SMOOTH_VEL: usize = 0x8CC398;
const SMOOTH_FIGHT: usize = 0x8CC39C;

/// `CCam::Process()` (0x526FC0): target point, orientation and speed variation for the mode processor,
/// the look-behind / left / right direction, the dispatch on `m_nMode` (+0xC), the true alpha / beta,
/// `KeepTrackOfTheSpeed`, the look processors and the script overrides. See
/// `docs/re/specs/CAM/CCam.md` (0x526FC0); the order of calls, re-reads and x87 operations follows the
/// bytes. `_CIpow` goes through `orig::x87_binary` and `GetATanOfXY` through its ST0 as an f64 (both
/// documented roundings, see the module docs of `camproc`).
///
/// **Not hooked** (decision for the main session): split runs on the grove walk give 1000/1000 cases
/// with 0 mismatches, but with this port on Rust the next `CCamera::CalculateDerivedValues` picks up
/// a different stale stack word as `m_mMatInverse.flags` (0xB6FA80): the original leaves the return
/// address 0x527DB4 of its `KeepTrackOfTheSpeed` call there (bit 0x20000, `rwMATRIXINTERNALIDENTITY`,
/// set), the Rust frame leaves other data. Ready attribute:
/// `#[hook(addr = 0x526FC0, conv = thiscall, name = "CCam::Process", expect = "DB 05 34 FE B6 00", this = "CCam")]`
pub fn cam_process(this: GPtr<u8>) {
    // 1. idle-camera bit and counter
    let flags = g::<u8>(0xC0B184);
    if X87::from_i32(g::<i32>(IDLE_COUNTER).read())
        .fcom(x(g::<f32>(IDLE_THRESHOLD).read()))
        .test_any(0x41)
    {
        flags.write(flags.read() & 0xFE);
    }
    let active = CAMS_ABS + 0x238 * u32::from(g::<u8>(ACTIVE_CAM_IDX).read());
    if g::<i16>(active as usize + 0xC).read() != 4 {
        flags.write(flags.read() & 0xFE);
        g::<u32>(IDLE_COUNTER).write(0);
    }
    let slot = this.byte_add(0x21C);
    let mut svw: u32 = 0; // speedVarWanted [B+0x10]
    let mut orientation: u32 = 0; // [B+0x14]
    if rd::<GPtr<u8>>(this, 0x21C).is_null() {
        let t = g::<GPtr<u8>>(CAMERA_TARGET).read();
        slot.cast::<GPtr<u8>>().write(t);
        register_reference(t, slot);
    }
    if g::<u8>(CROSSHAIR_FLAG).read() == 1 {
        let p = find_player_ped(-1);
        let keep = !p.is_null() && {
            let v = rd::<GPtr<u8>>(p, 0x58C);
            !v.is_null() && rd::<i16>(v, 0x22) == 0x208
        };
        if !keep {
            clear_cross_hair_immediately(0);
        }
    }
    // 2. collision-check frame counter
    let n = rd::<i32>(this, 0x1C).wrapping_add(1);
    wr::<i32>(this, 0x1C, n);
    if n > rd::<i32>(this, 0x18) {
        wr::<i32>(this, 0x1C, 1);
    }
    wr::<u8>(this, 3, u8::from(rd::<i32>(this, 0x1C) == rd::<i32>(this, 0x14)));
    // 3. target point
    let mut tgt = v3();
    if rd::<u8>(this, 2) != 0 {
        for (i, d) in tgt.iter_mut().enumerate() {
            *d = rd::<u32>(this, 0x154 + 4 * i as isize);
        }
    } else {
        let t = rd::<GPtr<u8>>(this, 0x21C);
        if rd::<u8>(t, 0x36) & 7 == 2 {
            process_vehicle_target(this, slot, &mut tgt, &mut orientation, &mut svw);
        } else {
            let tz = if t == find_player_ped(-1) {
                process_player_target(this, &mut tgt)
            } else {
                let p = entity_pos(t);
                tgt[0] = rd::<u32>(p, 0);
                tgt[1] = rd::<u32>(p, 4);
                rd::<u32>(p, 8)
            };
            tgt[2] = tz;
            orientation = target_orientation(slot);
            svw = 0;
            wr::<u32>(this, 0xD4, 0);
        }
    }
    // 4. look direction
    wr::<u32>(this, 0x28, g::<u32>(G_CAMERA_DIRECTION).read());
    let is_active = CAMS_ABS + 0x238 * u32::from(g::<u8>(ACTIVE_CAM_IDX).read()) == this.addr() as u32;
    g::<i32>(G_CAMERA_DIRECTION).write(3);
    if is_active {
        let mut ok = g::<u8>(TRANSITION_STATE).read() == 0;
        let mode = rd::<i16>(this, 0xC);
        let t = rd::<GPtr<u8>>(this, 0x21C);
        if matches!(mode, 0x12 | 0x10 | 0x16 | 3) && rd::<u8>(t, 0x36) & 7 == 2 {
            ok = !t.is_null() && {
                get_vehicle_appearance(t) == 3 || get_vehicle_appearance(rd::<GPtr<u8>>(this, 0x21C)) == 5
            };
            let clear3 = || {
                g::<u8>(TRANSITION_STATE).write(0);
                g::<u8>(DOING_SPECIAL_INTERP).write(0);
                g::<u8>(WAIT_FOR_INTERPOL).write(0);
            };
            if pad_look_behind_for_car(get_pad(0)) != 0 {
                clear3();
                if rd::<i32>(this, 0x28) != 0 {
                    g::<u8>(JUST_SWITCHED).write(1);
                }
                g::<i32>(G_CAMERA_DIRECTION).write(0);
            } else {
                let dir = if pad_look_left(get_pad(0)) != 0 && !ok {
                    clear3();
                    1
                } else if pad_look_right(get_pad(0)) != 0 && !ok {
                    clear3();
                    2
                } else {
                    3
                };
                if rd::<i32>(this, 0x28) != dir {
                    g::<u8>(JUST_SWITCHED).write(1);
                }
                g::<i32>(G_CAMERA_DIRECTION).write(dir);
            }
        } else {
            let ped_behind = if mode == 4 && rd::<u8>(t, 0x36) & 7 == 3 {
                if pad_look_behind_for_ped(get_pad(0)) != 0 {
                    if rd::<i32>(this, 0x28) != 0 && ok {
                        g::<u8>(JUST_SWITCHED).write(1);
                    }
                    g::<i32>(G_CAMERA_DIRECTION).write(0);
                    Some(true)
                } else {
                    Some(false)
                }
            } else if mode == 0x35 {
                Some(false)
            } else {
                None
            };
            if ped_behind == Some(false) {
                if rd::<i32>(this, 0x28) != 3 {
                    g::<u32>(G_CUR_DIST_FOR_CAM).write(0x3F80_0000);
                }
                g::<i32>(G_CAMERA_DIRECTION).write(3);
            }
        }
    }
    // 5. switch resets
    if g::<u8>(JUST_SWITCHED).read() != 0 {
        g::<u32>(G_CUR_DIST_FOR_CAM).write(0x3F80_0000);
        g::<u8>(RESET_OLD_MATRIX).write(1);
    }
    let mode = rd::<i16>(this, 0xC);
    if !matches!(mode, 3 | 0x12 | 0x16 | 0x10 | 0x31) {
        g::<u8>(SPEED_FX_USER_FLAG).write(0);
    }
    g::<u8>(FIRST_PERSON_RUN_THIS_FRAME).write(0);
    // 6. dispatch
    let tp = vp(&mut tgt);
    let sv = rd::<u32>(this, 0xD4);
    let (o, w) = (orientation, svw);
    match mode {
        3 | 18 | 22 => cam_follow_car_sa(this, tp, o, sv, w, 0),
        4 => {
            if g::<u8>(USE_MOUSE_3RD_PERSON).read() != 0 && g::<u8>(MOUSE_FOLLOW_PED_OFF).read() == 0 {
                cam_follow_ped_with_mouse(this, tp, o, sv, w);
            } else {
                cam_follow_ped_sa(this, tp, o, sv, w, 0);
            }
        }
        8 => cam_rocket(this, tp, o, sv, w, 0),
        51 => cam_rocket(this, tp, o, sv, w, 1),
        7 | 34 | 45 | 46 => cam_m16_1st_person(this, tp, o, sv, w),
        53 | 55 | 65 => cam_aim_weapon(this, tp, o, sv, w),
        14 => cam_wheel_cam(this, tp, o, sv, w),
        15 => cam_fixed(this, tp, o, sv, w),
        35 => cam_special_fixed_for_syphon(this, tp, o, sv, w),
        16 => cam_1st_person(this, tp, o, sv, w),
        17 => cam_fly_by(this, tp, o, sv, w),
        29 => {
            cam_peds_dead_baby(this);
            g::<u8>(PLAYER_IS_IN_GARAGE).write(0);
            g::<u8>(JUST_CAME_OUT_OF_GARAGE).write(0);
        }
        48 => cam_two_player(this),
        49 => cam_two_player_in_car_and_shooting(this),
        50 => cam_two_player_separate_cars(this),
        54 => cam_two_player_separate_cars_top_down(this),
        44 => cam_editor(this, tp, o, sv, w),
        39..=43 | 52 => cam_1rst_person_ped_on_pc(this, tp, o, sv, w),
        32 => {
            cam_arrest_cam_one(this);
        }
        47 => cam_attached_cam(this),
        56 => cam_dw_heli_chase(this, 0),
        57 => cam_dw_cam_man(this, 0),
        58 => cam_dw_birdy(this, 0),
        59 => cam_dw_plane_spotter(this, 0),
        60 | 61 => g::<u8>(USE_NEAR_CLIP_SCRIPT).write(0),
        62 => cam_dw_plane_cam1(this, 0),
        63 => cam_dw_plane_cam2(this, 0),
        64 => cam_dw_plane_cam3(this, 0),
        33 => {}
        _ => {
            for (o, v) in [
                (0x19C, [0u32, 0, 0]),
                (0x190, [0, 0x3F80_0000, 0]),
                (0x1B4, [0, 0, 0x3F80_0000]),
            ] {
                for (i, d) in v.iter().enumerate() {
                    wr::<u32>(this, o + 4 * i as isize, *d);
                }
            }
        }
    }
    // 7. true alpha / beta, speeds
    let mode = rd::<i16>(this, 0xC);
    if !(0x38..=0x40).contains(&mode) {
        g::<i32>(G_DW_CAM_MODE).write(-1);
    }
    g::<i32>(G_CAMERA_MODE).write(i32::from(mode));
    let src = this.byte_add(0x19C);
    let fudge = this.byte_add(0x148);
    let d = |i: isize| f32_of(x(rd::<f32>(src, 4 * i)).fsub_m(rd::<f32>(fudge, 4 * i)));
    let (dx, dy, dz) = (d(0), d(1), d(2));
    let beta = get_atan_of_xy_bits(dx.to_bits(), dy.to_bits()) as f32;
    wr::<f32>(this, 0xA0, beta);
    let h = f32_of(x(dy).fmul_m(dy).fadd(x(dx).fmul_m(dx)).fsqrt());
    let alpha = get_atan_of_xy_bits(h.to_bits(), dz.to_bits()) as f32;
    wr::<f32>(this, 0xA4, alpha);
    if g::<u8>(TRANSITION_STATE).read() == 0 {
        keep_track_of_the_speed(
            this,
            src,
            fudge,
            this.byte_add(0x1B4),
            this.byte_add(0xA4).cast(),
            this.byte_add(0xA0).cast(),
            this.byte_add(0xB4).cast(),
        );
    }
    // 8. look processors
    for i in 0..3 {
        wr::<u32>(this, 0x1A8 + 4 * i, rd::<u32>(src, 4 * i));
    }
    wr::<u8>(this, 9, 0);
    wr::<u8>(this, 8, 0);
    wr::<u8>(this, 7, 0);
    if CAMS_ABS + 0x238 * u32::from(g::<u8>(ACTIVE_CAM_IDX).read()) == this.addr() as u32 {
        match g::<i32>(G_CAMERA_DIRECTION).read() {
            0 => cam_look_behind(this),
            1 => cam_look_right(this, 0),
            2 => cam_look_right(this, 1),
            _ => {}
        }
        wr::<u32>(this, 0x28, g::<u32>(G_CAMERA_DIRECTION).read());
    }
    // 9. script overrides
    if g::<u8>(FOV_LERP_PROCESSED).read() != 0 {
        wr::<u32>(this, 0xB4, g::<u32>(FOV_NEW).read());
        g::<u8>(FOV_LERP_PROCESSED).write(0);
    }
    if g::<u8>(VEC_MOVE_LINEAR_PROCESSED).read() != 0 {
        for i in 0..3 {
            wr::<u32>(src, 4 * i as isize, g::<u32>(VEC_MOVE_LINEAR + 4 * i).read());
        }
        g::<u8>(VEC_MOVE_LINEAR_PROCESSED).write(0);
    }
    if g::<u8>(VEC_TRACK_LINEAR_PROCESSED).read() != 0 {
        let front = this.byte_add(0x190);
        for i in 0..3 {
            let v =
                f32_of(x(g::<f32>(VEC_TRACK_LINEAR + 4 * i).read()).fsub_m(rd::<f32>(src, 4 * i as isize)));
            wr::<f32>(front, 4 * i as isize, v);
        }
        vector_normalise(front);
        get_vectors_ready_for_rw(this);
        g::<u8>(VEC_TRACK_LINEAR_PROCESSED).write(0);
    }
}

/// `CCam::Process` step 3 for a vehicle target: target point, orientation, the 2D-normalised heading
/// against the move speed, speed variation, and the directly-behind flag.
fn process_vehicle_target(
    this: GPtr<u8>,
    slot: GPtr<u8>,
    tgt: &mut [u32; 3],
    orientation: &mut u32,
    svw: &mut u32,
) {
    let ent = || slot.cast::<GPtr<u8>>().read();
    let p = entity_pos(ent());
    for (i, d) in tgt.iter_mut().enumerate() {
        *d = rd::<u32>(p, 4 * i as isize);
    }
    *orientation = target_orientation(slot);
    let mut f = v3();
    f[0] = rd::<u32>(entity_matrix(ent()), 0x10);
    f[1] = rd::<u32>(entity_matrix(ent()), 0x14);
    vector_normalise(vp(&mut f));
    let (fx, fy) = (f32::from_bits(f[0]), f32::from_bits(f[1]));
    let l = x(fy).fmul_m(fy).fadd(x(fx).fmul_m(fx)).fsqrt();
    // fcom 0.0; test ah, 0x44; jnp: skip only when exactly 0
    if l.fcom(x(0.0)).test_parity(0x44) {
        let inv = l.fdivr_m(1.0);
        f[0] = f32_of(x(fx).fmul(inv)).to_bits();
        f[1] = f32_of(inv.fmul_m(fy)).to_bits();
    }
    let veh = ent();
    let a = x(f32::from_bits(f[0])).fmul_m(rd::<f32>(veh, 0x44));
    let b = x(f32::from_bits(f[1])).fmul_m(rd::<f32>(veh, 0x48));
    let positive = !b.fadd(a).fcom(x(0.0)).test_any(0x41);
    let sp = b.fmul(b).fadd(a.fmul(a)).fsqrt();
    *svw = if positive {
        let v = sp.fmul_m(g::<f32>(0x86325C).read());
        if x(1.0).fcom(v).test_parity(0x5) {
            f32_of(v).to_bits()
        } else {
            0x3F80_0000
        }
    } else {
        let v = sp.fmul_m(g::<f32>(0x863258).read());
        let h = g::<f32>(0x858B8C).read();
        let m = if x(h).fcom(v).test_parity(0x5) { v } else { x(h) };
        f32_of(m.fchs()).to_bits()
    };
    let sv = x(f32::from_bits(*svw))
        .fmul_m(g::<f32>(0x863254).read())
        .fadd(x(rd::<f32>(this, 0xD4)).fmul_m(g::<f32>(0x863250).read()));
    wr::<f32>(this, 0xD4, f32_of(sv));
    if rd::<i32>(this, 0x28) != 3 {
        let skip = pad_look_behind_for_car(get_pad(0)) != 0
            && pad_look_left(get_pad(0)) == 0
            && pad_look_right(get_pad(0)) == 0;
        if !skip {
            g::<u8>(CAM_DIRECTLY_BEHIND).write(1);
        }
    }
}

/// `CCam::Process` step 3 for the player ped: the (climb-adjusted) position, smoothed through the
/// statics `sPedPos` / `sPedVel` when it moved by at most 3 units, the time step is >= 0.2 and no
/// mouse / directly-behind / in-front camera is active. Writes `tgt.x/y` and returns `tgt.z`.
fn process_player_target(this: GPtr<u8>, tgt: &mut [u32; 3]) -> u32 {
    let mut pv = v3();
    let pos = entity_pos(find_player_ped(-1));
    for (i, d) in pv.iter_mut().enumerate() {
        *d = rd::<u32>(pos, 4 * i as isize);
    }
    let pp = vp(&mut pv);
    let intel = |p: GPtr<u8>| rd::<GPtr<u8>>(p, 0x47C);
    if !get_task_climb(intel(find_player_ped(-1))).is_null() {
        let i = intel(find_player_ped(-1));
        let ped = find_player_ped(-1);
        let task = get_task_climb(i);
        climb_get_camera_target_pos(task, ped, pp);
    }
    let spos = GPtr::<u8>::from_addr(S_PED_POS);
    let svel = GPtr::<u8>::from_addr(S_PED_VEL);
    let pf = |i: usize| f32::from_bits(pv[i]);
    let dx = x(rd::<f32>(spos, 0)).fsub_m(pf(0));
    let dy = x(rd::<f32>(spos, 4)).fsub_m(pf(1));
    let dz = x(rd::<f32>(spos, 8)).fsub_m(pf(2));
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    let ts_bits = g::<u32>(TIME_STEP).read();
    let ts = f32::from_bits(ts_bits);
    let smooth = d2.fcom(x(g::<f32>(0x85EED4).read())).test_any(0x41)
        && x(ts).fcom(x(g::<f32>(0x858CC4).read())).test_parity(0x5)
        && using_3rd_person_mouse_cam(this) == 0
        && g::<u8>(CAM_DIRECTLY_BEHIND).read() == 0
        && g::<u8>(CAM_DIRECTLY_IN_FRONT).read() == 0;
    let (nx, ny, nz, vel): (u32, u32, u32, [u32; 3]);
    if !smooth {
        (nx, ny, nz, vel) = (pv[0], pv[1], pv[2], [0, 0, 0]);
    } else if !get_task_fighting(intel(find_player_ped(-1))).is_null() && rd::<i16>(this, 0xC) == 0x35 {
        let p = f32_of(X87::from_f64_exact(sa_core::orig::x87_binary(
            CI_POW,
            f64::from(g::<f32>(SMOOTH_FIGHT).read()),
            f64::from(ts),
        )));
        let (mut t1, mut t2, mut t3) = (v3(), v3(), v3());
        let one_minus = f32_of(x(1.0).fsub_m(p));
        vec_s_mul(vp(&mut t1), one_minus.to_bits(), pp);
        vec_s_mul(vp(&mut t2), p.to_bits(), spos);
        let r = vec_add(vp(&mut t3), vp(&mut t2), vp(&mut t1));
        (nx, ny, nz, vel) = (rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8), [0, 0, 0]);
    } else {
        let pw = |base: usize| {
            X87::from_f64_exact(sa_core::orig::x87_binary(
                CI_POW,
                f64::from(g::<f32>(base).read()),
                f64::from(ts),
            ))
        };
        let a = f32_of(pw(SMOOTH_POS).fsubr_m(1.0));
        let b = f32_of(pw(SMOOTH_VEL).fsubr_m(1.0));
        let (mut t50, mut t44, mut t5c, mut t68, mut t74, mut t80) = (v3(), v3(), v3(), v3(), v3(), v3());
        vec_mul_s(vp(&mut t50), svel, ts_bits);
        let pred = vec_add(vp(&mut t44), spos, vp(&mut t50));
        let r = vec_s_mul(vp(&mut t5c), f32_of(x(1.0).fsub_m(a)).to_bits(), pred);
        let ap = vec_s_mul(vp(&mut t68), a.to_bits(), pp);
        let nn = vec_add(vp(&mut t74), ap, r);
        let (n0, n1) = (rd::<u32>(nn, 0), rd::<u32>(nn, 4));
        let mut dd = v3();
        dd[0] = f32_of(x(f32::from_bits(n0)).fsub_m(rd::<f32>(spos, 0))).to_bits();
        dd[1] = f32_of(x(f32::from_bits(n1)).fsub_m(rd::<f32>(spos, 4))).to_bits();
        dd[2] = f32_of(x(pf(2)).fsub_m(rd::<f32>(spos, 8))).to_bits();
        let vb = vec_s_mul(vp(&mut t74), f32_of(x(1.0).fsub_m(b)).to_bits(), svel);
        let mx = max_f(0x3F80_0000, ts_bits) as f32;
        let bd = vec_s_mul(vp(&mut t68), b.to_bits(), vp(&mut dd));
        let q = vec_div_s(vp(&mut t5c), bd, mx.to_bits());
        let v = vec_add(vp(&mut t80), q, vb);
        (nx, ny, nz, vel) = (n0, n1, pv[2], [rd::<u32>(v, 0), rd::<u32>(v, 4), 0]);
    }
    tgt[0] = nx;
    tgt[1] = ny;
    g::<[u32; 3]>(S_PED_POS).write([nx, ny, nz]);
    g::<[u32; 3]>(S_PED_VEL).write(vel);
    nz
}

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    pub fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CIdleCam::GetLookAtPositionOnTarget(CEntity*, CVector*)` (0x50EAE0)
    pub fn idle_get_look_at_position_on_target(this: GPtr<u8>, e: GPtr<u8>, out: GPtr<u8>) = thiscall 0x50EAE0 => crate::idlecam::get_look_at_position_on_target;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490, AL)
    pub fn get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bb: u32, v: u32, p: u32, o: u32, d: u32, s: u32, c: u32) -> u8 = cdecl 0x56A490 => unported;
}

/// `CIdleCam::SetTarget(CEntity*)` (0x50A280, `ret 4`). The entry `jmp 0x403961` is a protection
/// fragment that loads `eax = [rol([0x156077C], [0x5E4224] & 31)]` = `[0xB7CB84]`
/// (`CTimer::m_snTimeInMilliseconds`, for the `+ 2^32` sign test) and jumps back. The previous look-at
/// point (+4) becomes the old target's (+0x18) or `cam.source + cam.front` of the active cam (+0x98;
/// f32 per axis); the target reference (+0) moves to `e` (or is cleared); +0x10 and +0x34 = now (f32),
/// +0x54 = 0, +0x79 = 0.
///
/// Not hooked: the protection code reads its entry bytes as data (`sbb ebx, [0x50A281]` at 0x4039BB,
/// sa-hook `every_hook_installs_on_the_exe`); callers reach it through `orig!` (original body).
#[cfg_attr(sa_phase_a, allow(dead_code))]
pub fn idle_cam_set_target(this: GPtr<u8>, e: GPtr<u8>) {
    let now = time_ms_f32();
    let old = rd::<GPtr<u8>>(this, 0);
    if !old.is_null() {
        for i in 0..3 {
            wr::<u32>(this, 4 + 4 * i, rd::<u32>(this, 0x18 + 4 * i));
        }
    } else {
        let cam = rd::<GPtr<u8>>(this, 0x98);
        for i in 0..3 {
            let v = f32_of(x(rd::<f32>(cam, 0x19C + 4 * i)).fadd_m(rd::<f32>(cam, 0x190 + 4 * i)));
            wr::<f32>(this, 4 + 4 * i, v);
        }
    }
    if !e.is_null() {
        if !old.is_null() {
            clean_up_old_reference(old, this);
        }
        wr::<GPtr<u8>>(this, 0, e);
        register_reference(e, this);
    } else if !old.is_null() {
        clean_up_old_reference(old, this);
        wr::<u32>(this, 0, 0);
    }
    wr::<f32>(this, 0x10, now);
    wr::<f32>(this, 0x34, now);
    wr::<u32>(this, 0x54, 0);
    wr::<u8>(this, 0x79, 0);
}

/// `CIdleCam::IsTargetValid(CEntity*)` (0x517770, `ret 4`, AL): 0 without a target, 1 for the player
/// ped; otherwise the distance from the active cam's source to the look-at point must lie in
/// [+0x44, +0x4C] (`test ah, 5; jp` / `test ah, 0x41; je`), then 1 when +0x24 < 1.0 (or unordered) or
/// the line of sight is clear (`pIgnoreEntity` = the target), else the blocked counter +0x54 is
/// incremented and the old value compared with +0x50 (`jg` -> 0).
#[hook(addr = 0x517770, conv = thiscall, name = "CIdleCam::IsTargetValid", expect = "83 EC 0C 53 8B 5C 24 14", this = "CIdleCam")]
pub fn idle_cam_is_target_valid(this: GPtr<u8>, e: GPtr<u8>) -> u8 {
    if e.is_null() {
        return 0;
    }
    if e == find_player_ped(-1) {
        return 1;
    }
    let mut p = v3();
    idle_get_look_at_position_on_target(this, e, vp(&mut p));
    let src = rd::<GPtr<u8>>(this, 0x98).byte_add(0x19C);
    let d = |i: usize| x(rd::<f32>(src, 4 * i as isize)).fsub_m(f32::from_bits(p[i]));
    let (dx, dy, dz) = (d(0), d(1), d(2));
    let dist = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    if !dist.fcom(x(rd::<f32>(this, 0x44))).test_parity(0x5) {
        return 0;
    }
    if !dist.fcom(x(rd::<f32>(this, 0x4C))).test_any(0x41) {
        return 0;
    }
    if x(rd::<f32>(this, 0x24)).fcom(x(1.0)).test_any(0x1) {
        return 1;
    }
    let saved = g::<u32>(IGNORE_ENTITY).read();
    g::<GPtr<u8>>(IGNORE_ENTITY).write(e);
    let clear = get_is_line_of_sight_clear(src, vp(&mut p), 1, 0, 0, 1, 0, 0, 1);
    g::<u32>(IGNORE_ENTITY).write(saved);
    if clear != 0 {
        return 1;
    }
    let n = rd::<i32>(this, 0x54);
    wr::<i32>(this, 0x54, n.wrapping_add(1));
    if n > rd::<i32>(this, 0x50) { 0 } else { 1 }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_CONVERT_PED_NODE_2_BONE_TAG,
    HOOK_ROUND_DECIMALS,
    HOOK_ROUND_VECTOR_DECIMALS,
    HOOK_HELI_CHASE_SETTINGS_INIT,
    HOOK_CAM_ENTRY_CTOR,
    HOOK_CAM_PATH_SPLINES_CTOR,
    HOOK_CULL_ZONE_CENTRE,
    HOOK_CAMERA_DTOR,
    HOOK_CAMERA_SCALAR_DELETING_DTOR,
    HOOK_DTOR_G_HAND_SHAKER,
    HOOK_DTOR_PRE_MIRROR_MAT,
    HOOK_DTOR_G_B6FEC0,
    HOOK_DTOR_THE_CAMERA,
    HOOK_PROCESS_FIXED,
    HOOK_GET_BOAT_LOOK_LR_BEHIND_CAM_HEIGHT,
    HOOK_ENTER_CAR_CAMERA_STICK_MODIFIER,
    HOOK_IDLE_CAM_PROCESS,
    HOOK_IDLE_CAM_RUN,
    HOOK_DRAW_BORDERS_FOR_WIDE_SCREEN,
    HOOK_IDLE_CAM_IS_TARGET_VALID,
];

/// This module's globals (none).
pub const GLOBALS: &[GlobalDesc] = &[];
