//! PORT-CAM: `CCam` methods (the per-mode camera, `this` = CCam 0x238 bytes). Ported from the 1.0 US
//! bytes; gta-reversed `Cam.cpp` and `docs/re/specs/CAM/CCam.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::{TIME_MS, TIME_STEP};
use sa_core::{hook, orig};

use crate::common::{Fpu, g, k, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `DWCineyCamLastPos` (Cam.cpp)
    pub static DW_CINEY_CAM_LAST_POS @ 0xB6FE8C: [u32; 3] = [0; 3];
    /// `DWCineyCamLastUp`
    pub static DW_CINEY_CAM_LAST_UP @ 0xB6FE98: [u32; 3] = [0; 3];
    /// `DWCineyCamLastRight`
    pub static DW_CINEY_CAM_LAST_RIGHT @ 0xB6FEA4: [u32; 3] = [0; 3];
    /// `DWCineyCamLastFwd`
    pub static DW_CINEY_CAM_LAST_FWD @ 0xB6FEB0: [u32; 3] = [0; 3];
    /// `DWCineyCamLastNearClip`
    pub static DW_CINEY_CAM_LAST_NEAR_CLIP @ 0xB6EC08: f32 = 0.0;
    /// `DWCineyCamLastFov`
    pub static DW_CINEY_CAM_LAST_FOV @ 0xB6EC0C: u32 = 0;
    /// `__init_g_b6fec0`'s object (0x9C bytes, constructed by 0x5173F0; class not proven, see
    /// `docs/re/specs/CAM/_static-init-B.md`)
    pub static G_B6FEC0 @ 0xB6FEC0: [u8; 0x9C] = [0; 0x9C];
    /// `CCam::KeepTrackOfTheSpeed`'s `s_PreviousFov`
    pub static KTS_PREV_FOV @ 0xB6FF5C: f32 = 0.0;
    /// `s_PreviousAlpha`
    pub static KTS_PREV_ALPHA @ 0xB6FF60: f32 = 0.0;
    /// `s_PreviousBeta`
    pub static KTS_PREV_BETA @ 0xB6FF64: f32 = 0.0;
    /// `s_PreviousUp`
    pub static KTS_PREV_UP @ 0xB6FF68: [u32; 3] = [0; 3];
    /// `s_PreviousTarget`
    pub static KTS_PREV_TARGET @ 0xB6FF74: [u32; 3] = [0; 3];
    /// `s_PreviousSource`
    pub static KTS_PREV_SOURCE @ 0xB6FF80: [u32; 3] = [0; 3];
    /// The function-local statics' guard bits (1 source, 2 target, 4 up, 8 beta, 0x10 alpha, 0x20 fov)
    pub static KTS_GUARD @ 0xB6FF8C: u32 = 0;
}

/// `CCamera::m_bUseMouse3rdPerson` (CCamera static, camera.rs's subsystem)
const USE_MOUSE_3RD_PERSON: usize = 0xB6EC2E;
/// `TheCamera.m_bJust_Switched`-area byte (TheCamera +0x2A) KeepTrackOfTheSpeed tests
const CAMERA_B6F052: usize = 0xB6F052;
/// Camera.cpp globals `CCam::Init` writes: `gPlayerPedVisible`, its neighbour byte, `gCameraDirection`,
/// `gCameraMode`, `gLastTime2PlayerCameraWasOK`, `gLastTime2PlayerCameraCollided`, byte 0xB6FD18.
const G_PLAYER_PED_VISIBLE: usize = 0x8CC380;
const G_8CC381: usize = 0x8CC381;
const G_CAMERA_DIRECTION: usize = 0x8CC384;
const G_CAMERA_MODE: usize = 0x8CC388;
const G_LAST_TIME_2P_OK: usize = 0xB6EC24;
const G_LAST_TIME_2P_COLLIDED: usize = 0xB6EC28;
const G_B6FD18: usize = 0xB6FD18;

/// pi (0x858CB8), 2 pi (0x858CBC), -pi (0x858CC0), 0.0 (0x858B50), 10.0 (0x85862C)
const PI: u32 = 0x4049_0FDB;
const TWO_PI: u32 = 0x40C9_0FDB;
const NEG_PI: u32 = 0xC049_0FDB;
const ZERO: u32 = 0x0000_0000;
const TEN: u32 = 0x4120_0000;

orig! {
    /// `CCam::WrapAngle(float&)` (0x509BE0)
    pub fn wrap_angle_call(a: GPtr<f32>) = cdecl 0x509BE0 => wrap_angle;
    /// `CCam::Init()` (0x50E490)
    pub fn cam_init_call(this: GPtr<u8>) = thiscall 0x50E490 => cam_init;
    /// `CIdleCam::Reset(bool)` (0x50A160)
    pub fn idle_cam_reset_call(this: GPtr<u8>, reset_controls: u8) = thiscall 0x50A160 => crate::idlecam::reset;
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730)
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
}

/// Dword copy of a 3-word vector (`mov` pairs, component by component).
fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for o in [0, 4, 8] {
        wr::<u32>(dst, o, rd::<u32>(src, o));
    }
}

fn gv(v: &sa_core::mem::Global<[u32; 3]>) -> GPtr<u8> {
    v.ptr().cast()
}

/// `CCam::WrapAngle(float& angle)` (0x509BE0, cdecl): while `!(a < pi)` subtract 2 pi, then while
/// `a < -pi` add 2 pi, each loop on the x87 stack (one `fstp` at its end). NaN leaves `a` untouched;
/// +-inf never leaves its loop (as the original).
#[hook(addr = 0x509BE0, conv = cdecl, name = "CCam::WrapAngle", expect = "8B 4C 24 04 D9 01", preserve = "edx")]
pub fn wrap_angle(a: GPtr<f32>) {
    let mut f = Fpu::new();
    f.fld(a.read());
    f.fcomp_m(k(PI));
    if !f.jne(0x01) {
        f.fld(a.read());
        loop {
            f.fsub_m(k(TWO_PI));
            f.fcom_m(k(PI));
            if f.jne(0x01) {
                break;
            }
        }
        a.write(f.fstp());
    }
    f.fld(a.read());
    f.fcomp_m(k(NEG_PI));
    if f.jp(0x05) {
        return;
    }
    f.fld(a.read());
    loop {
        f.fadd_m(k(TWO_PI));
        f.fcom_m(k(NEG_PI));
        if f.jp(0x05) {
            break;
        }
    }
    a.write(f.fstp());
}

/// `CCam::ClipBeta()` (0x509C50): `m_fVerticalAngle` (+0xBC) above pi loses 2 pi; below -pi gains
/// 2 pi (once each).
#[hook(addr = 0x509C50, conv = thiscall, name = "CCam::ClipBeta", expect = "D9 81 BC 00 00 00", this = "CCam")]
pub fn clip_beta(this: GPtr<u8>) {
    let mut f = Fpu::new();
    f.fld(rd(this, 0xBC));
    f.fcomp_m(k(PI));
    f.fld(rd(this, 0xBC));
    if !f.jne(0x41) {
        f.fsub_m(k(TWO_PI));
        wr(this, 0xBC, f.fstp());
        return;
    }
    f.fcomp_m(k(NEG_PI));
    if f.jp(0x05) {
        return;
    }
    f.fld(rd(this, 0xBC));
    f.fadd_m(k(TWO_PI));
    wr(this, 0xBC, f.fstp());
}

/// `CCam::Using3rdPersonMouseCam()` (0x50A850): `CCamera::m_bUseMouse3rdPerson && mode == 4`.
#[hook(addr = 0x50A850, conv = thiscall, name = "CCam::Using3rdPersonMouseCam", expect = "A0 2E EC B6 00", this = "CCam")]
pub fn using_3rd_person_mouse_cam(this: GPtr<u8>) -> u8 {
    u8::from(g::<u8>(USE_MOUSE_3RD_PERSON).read() != 0 && rd::<u16>(this, 0xC) == 4)
}

/// `CCam::GetWeaponFirstPersonOn()` (0x509DC0): for a ped target (+0x21C, type bits 3), byte +0x5B4
/// (`m_bFirstPersonWeaponModeSelected`?) of its active weapon slot (`m_nActiveWeaponSlot` +0x718,
/// sign-extended, stride 0x1C).
#[hook(addr = 0x509DC0, conv = thiscall, name = "CCam::GetWeaponFirstPersonOn", expect = "8B 81 1C 02 00 00", this = "CCam")]
pub fn get_weapon_first_person_on(this: GPtr<u8>) -> u8 {
    let e: GPtr<u8> = rd(this, 0x21C);
    if e.is_null() || rd::<u8>(e, 0x36) & 7 != 3 {
        return 0;
    }
    let slot = i32::from(rd::<i8>(e, 0x718)).wrapping_mul(0x1C);
    rd::<u8>(e.byte_add(slot as isize), 0x5B4)
}

/// `CCam::DoCamBump(float horizontal, float vertical)` (0x50CB30, `ret 8`): dword copies to +0x118 /
/// +0x11C (the arguments as raw bits) and the bump time +0x120 = `CTimer::m_snTimeInMilliseconds`.
#[hook(addr = 0x50CB30, conv = thiscall, name = "CCam::DoCamBump", expect = "8B 44 24 04 8B 54 24 08", this = "CCam")]
pub fn do_cam_bump(this: GPtr<u8>, horizontal: u32, vertical: u32) {
    wr(this, 0x118, horizontal);
    wr(this, 0x11C, vertical);
    wr(this, 0x120, TIME_MS.get());
}

/// `WellBufferMe(float target, float& value, float& speed, float topSpeed, float step, bool isAngle)`
/// (0x509AE0, cdecl). `d = target - value` (with `isAngle == 1`: the f32 `d` wrapped into [-pi, pi)
/// on the stack, else the unstored difference); `ts = d * topSpeed`; the speed moves towards `ts` by
/// `|ts - speed| * timeStep * step` (added when `ts - speed > 0`, else subtracted) and is clamped to `ts`
/// when it overshoots; `value += min(10, timeStep)` (10 unless `10 < timeStep` is false) `* speed`.
#[hook(addr = 0x509AE0, conv = cdecl, name = "CCam::WellBufferMe", expect = "D9 44 24 04 8B 54 24 08")]
pub fn well_buffer_me(
    target: f32,
    value: GPtr<f32>,
    speed: GPtr<f32>,
    top_speed: f32,
    step: f32,
    is_angle: u8,
) {
    let mut f = Fpu::new();
    f.fld(target);
    f.fsub_m(value.read());
    let d = f.fst();
    if is_angle == 1 {
        f.fstp_st(0);
        f.fld(d);
        f.fcom_m(k(PI));
        if !f.jne(0x01) {
            loop {
                f.fsub_m(k(TWO_PI));
                f.fcom_m(k(PI));
                if f.jne(0x01) {
                    break;
                }
            }
        }
        f.fcom_m(k(NEG_PI));
        if !f.jp(0x05) {
            loop {
                f.fadd_m(k(TWO_PI));
                f.fcom_m(k(NEG_PI));
                if f.jp(0x05) {
                    break;
                }
            }
        }
    }
    f.fmul_m(top_speed);
    f.fld_st(0);
    f.fsub_m(speed.read());
    f.fcom_m(k(ZERO));
    let up = !f.jne(0x41);
    f.fabs();
    f.fmul_m(TIME_STEP.get());
    f.fmul_m(step);
    if up {
        f.fadd_m(speed.read());
    } else {
        f.fsubr_m(speed.read());
    }
    speed.write(f.fstp());
    // ST0 = ts
    f.fcom_m(k(ZERO));
    let mut stored = false;
    if !f.jp(0x05) {
        f.fcom_m(speed.read());
        if !f.jne(0x41) {
            speed.write(f.fstp());
            stored = true;
        }
    }
    if !stored {
        f.fcom_m(k(ZERO));
        if !f.jne(0x41) {
            f.fcom_m(speed.read());
            if !f.jp(0x05) {
                speed.write(f.fstp());
                stored = true;
            }
        }
    }
    if !stored {
        f.fstp_st(0);
    }
    f.fld(k(TEN));
    f.fcomp_m(TIME_STEP.get());
    if f.jp(0x05) {
        f.fld(TIME_STEP.get());
    } else {
        f.fld(k(TEN));
    }
    f.fmul_m(speed.read());
    f.fadd_m(value.read());
    value.write(f.fstp());
}

/// `CCam::GetVectorsReadyForRW()` (0x509CE0): up = (0, 0, 1), `front.Normalise()`, a front with
/// `x == 0 && y == 0` (ordered) gets x = y = 0.0001; `right = Normalise(front x up)`,
/// `up = right x front`.
#[hook(addr = 0x509CE0, conv = thiscall, name = "CCam::GetVectorsReadyForRW", expect = "83 EC 18 53 8B D9", this = "CCam")]
pub fn get_vectors_ready_for_rw(this: GPtr<u8>) {
    let up = this.byte_add(0x1B4);
    let front = this.byte_add(0x190);
    wr::<u32>(up, 0, 0);
    wr::<u32>(up, 4, 0);
    wr::<u32>(up, 8, 0x3F80_0000);
    vector_normalise(front);
    let fx = x(rd(front, 0));
    let fy = x(rd(this, 0x194));
    if !crate::common::jp(fx, x(k(ZERO)), 0x44) && !crate::common::jp(fy, x(k(ZERO)), 0x44) {
        wr::<u32>(front, 0, 0x38D1_B717);
        wr::<u32>(this, 0x194, 0x38D1_B717);
    }
    // locals: [0..3) right, [3..6) CrossProduct output
    let mut loc = [0u32; 6];
    let lp = GPtr::<u32>::from_raw(loc.as_mut_ptr()).cast::<u8>();
    let r = cross_product(lp.byte_add(12), front, up);
    copy3(lp, r);
    vector_normalise(lp);
    let r = cross_product(lp.byte_add(12), lp, front);
    copy3(up, r);
}

/// `CCam::Constructor()` (0x517730): `Init()`, returns `this`.
#[hook(addr = 0x517730, conv = thiscall, name = "CCam::Constructor", expect = "56 8B F1 E8 58 6D FF FF", this = "CCam")]
pub fn cam_constructor(this: GPtr<u8>) -> GPtr<u8> {
    cam_init_call(this);
    this
}

/// `CCam::Init()` (0x50E490; its `mov ebx, 1` sits in a SecuROM fragment at 0x40808C):
/// `gIdleCam.Reset(true)`, every field the bytes set (in their order), and the Camera.cpp globals
/// `gPlayerPedVisible` = 1, 0x8CC381 = 2, `gCameraDirection` = 3, `gCameraMode` = -1,
/// `gLastTime2PlayerCamera{WasOK,Collided}` = 0, byte 0xB6FD18 = 0.
#[hook(addr = 0x50E490, conv = thiscall, name = "CCam::Init", expect = "83 EC 0C 53 55", this = "CCam")]
pub fn cam_init(this: GPtr<u8>) {
    idle_cam_reset_call(crate::idlecam::G_IDLE_CAM.ptr().cast(), 1);
    let w = |o: isize, v: u32| wr::<u32>(this, o, v);
    let b = |o: isize, v: u8| wr::<u8>(this, o, v);
    w(0x190, 0);
    w(0x194, 0);
    w(0x198, 0xBF80_0000);
    w(0x1B4, 0);
    w(0x1B8, 0);
    w(0x1BC, 0x3F80_0000);
    wr::<u16>(this, 0xC, 4);
    b(0xB, 0);
    w(0x14, 1);
    w(0x18, 9);
    w(0x1C, 0);
    b(0x3, 1);
    w(0x88, 0);
    w(0xC0, 0);
    w(0xB0, 0);
    w(0xD8, 0x3F40_0000);
    w(0x2C, 0x3D0E_FA35);
    w(0xC4, 0x41F0_0000);
    w(0xC8, 0);
    w(0x22C, 0);
    w(0x230, 0);
    b(0xA, 1);
    w(0xBC, 0);
    w(0x94, 0);
    w(0x98, 0);
    b(0x4, 0);
    w(0xCC, 0);
    w(0xD0, 0);
    b(0x7, 0);
    b(0x8, 0);
    b(0x9, 0);
    w(0x80, 0x3EB2_B8C3);
    w(0x38, 0x3F00_0000);
    w(0x84, 0x3FC0_0000);
    w(0x28, 3);
    b(0x6, 0);
    for o in [
        0x30, 0x34, 0x5C, 0x60, 0x64, 0x68, 0x7C, 0x8C, 0x48, 0x4C, 0x50, 0x6C, 0x104, 0x10C, 0x108, 0x110,
        0x114,
    ] {
        w(o, 0);
    }
    w(0x118, 0x3F80_0000);
    w(0x11C, 0);
    w(0x120, 0);
    for i in 0..4 {
        let v = 0x1D8 + 12 * i;
        w(v, 0);
        w(v + 4, 0);
        w(0x208 + 4 * i, 0);
        w(v + 8, 0);
    }
    w(0x218, 0);
    g::<u8>(G_PLAYER_PED_VISIBLE).write(1);
    g::<u8>(G_8CC381).write(2);
    g::<u32>(G_CAMERA_DIRECTION).write(3);
    g::<u32>(G_CAMERA_MODE).write(0xFFFF_FFFF);
    g::<u32>(G_LAST_TIME_2P_OK).write(0);
    g::<u32>(G_LAST_TIME_2P_COLLIDED).write(0);
    g::<u8>(G_B6FD18).write(0);
}

/// `CCam::KeepTrackOfTheSpeed(source, target, up, alpha, beta, fov)` (0x509DF0, `ret 0x18`, all by
/// reference). First call per static: the previous value is the current one (the beta and fov guards
/// keep the value on the x87 stack without storing the static). `TheCamera` byte +0x2A (0xB6F052)
/// resets the previous vectors. Speeds: +0x124 source, +0x130 target, +0x13C up (f32 differences),
/// +0x74 fov, +0x44 beta (wrapped), +0x40 alpha (wrapped); then every previous value is updated.
#[hook(addr = 0x509DF0, conv = thiscall, name = "CCam::KeepTrackOfTheSpeed", expect = "A1 8C FF B6 00", this = "CCam")]
pub fn keep_track_of_the_speed(
    this: GPtr<u8>,
    source: GPtr<u8>,
    target: GPtr<u8>,
    up: GPtr<u8>,
    alpha: GPtr<f32>,
    beta: GPtr<f32>,
    fov: GPtr<f32>,
) {
    let mut fl = KTS_GUARD.get();
    if fl & 1 == 0 {
        copy3(gv(&KTS_PREV_SOURCE), source);
        fl |= 1;
        KTS_GUARD.set(fl);
    }
    if fl & 2 == 0 {
        copy3(gv(&KTS_PREV_TARGET), target);
        fl |= 2;
        KTS_GUARD.set(fl);
    }
    if fl & 4 == 0 {
        copy3(gv(&KTS_PREV_UP), up);
        fl |= 4;
        KTS_GUARD.set(fl);
    }
    let mut f = Fpu::new();
    if fl & 8 == 0 {
        f.fld(beta.read());
        fl |= 8;
        KTS_GUARD.set(fl);
    } else {
        f.fld(KTS_PREV_BETA.get());
    }
    if fl & 0x10 == 0 {
        let a = alpha.cast::<u32>().read();
        fl |= 0x10;
        KTS_GUARD.set(fl);
        KTS_PREV_ALPHA.ptr().cast::<u32>().write(a);
    }
    if fl & 0x20 == 0 {
        f.fld(fov.read());
        fl |= 0x20;
        KTS_GUARD.set(fl);
    } else {
        f.fld(KTS_PREV_FOV.get());
    }
    if g::<u8>(CAMERA_B6F052).read() != 0 {
        copy3(gv(&KTS_PREV_SOURCE), source);
        copy3(gv(&KTS_PREV_TARGET), target);
        copy3(gv(&KTS_PREV_UP), up);
    }
    for (v, prev, off) in [
        (source, gv(&KTS_PREV_SOURCE), 0x124),
        (target, gv(&KTS_PREV_TARGET), 0x130),
        (up, gv(&KTS_PREV_UP), 0x13C),
    ] {
        let d: [f32; 3] = [0, 4, 8].map(|o| x(rd::<f32>(v, o)).fsub_m(rd::<f32>(prev, o)).to_f32());
        for (i, c) in d.iter().enumerate() {
            wr(this, off + 4 * i as isize, *c);
        }
    }
    // ST0 = previous fov, ST1 = previous beta
    f.fld(fov.read());
    f.fsub_st(1);
    wr(this, 0x74, f.fstp());
    f.fstp_st(0);
    f.fld(beta.read());
    f.fsub_st(1);
    wr(this, 0x44, f.fstp());
    f.fstp_st(0);
    wrap_angle_call(this.byte_add(0x44).cast());
    wr(this, 0x40, x(alpha.read()).fsub_m(KTS_PREV_ALPHA.get()).to_f32());
    wrap_angle_call(this.byte_add(0x40).cast());
    copy3(gv(&KTS_PREV_SOURCE), source);
    copy3(gv(&KTS_PREV_TARGET), target);
    copy3(gv(&KTS_PREV_UP), up);
    let (b, a, v) = (
        beta.cast::<u32>().read(),
        alpha.cast::<u32>().read(),
        fov.cast::<u32>().read(),
    );
    KTS_PREV_BETA.ptr().cast::<u32>().write(b);
    KTS_PREV_ALPHA.ptr().cast::<u32>().write(a);
    KTS_PREV_FOV.ptr().cast::<u32>().write(v);
}

/// `CCam::CacheLastSettingsDWCineyCam()` (0x50D7A0): `DWCineyCamLastUp` = up (+0x1B4),
/// `LastRight` = front x up, `LastFwd` = front (+0x190), `LastFov` = +0xB4 (dword), `LastNearClip` =
/// `Scene.camera->nearPlane` (+0x80, `fld`/`fstp`), `LastPos` = source (+0x19C).
#[hook(addr = 0x50D7A0, conv = thiscall, name = "CCam::CacheLastSettingsDWCineyCam", expect = "83 EC 0C 56 8B F1", this = "CCam")]
pub fn cache_last_settings_dw_ciney_cam(this: GPtr<u8>) {
    let up = this.byte_add(0x1B4);
    let front = this.byte_add(0x190);
    copy3(gv(&DW_CINEY_CAM_LAST_UP), up);
    let mut loc = [0u32; 3];
    let lp = GPtr::<u32>::from_raw(loc.as_mut_ptr()).cast::<u8>();
    cross_product(lp, front, up);
    copy3(gv(&DW_CINEY_CAM_LAST_RIGHT), lp);
    copy3(gv(&DW_CINEY_CAM_LAST_FWD), front);
    let cam = sa_core::platform::SCENE_CAMERA.get();
    DW_CINEY_CAM_LAST_FOV.set(rd(this, 0xB4));
    DW_CINEY_CAM_LAST_NEAR_CLIP.set(x(rd(cam, 0x80)).to_f32());
    copy3(gv(&DW_CINEY_CAM_LAST_POS), this.byte_add(0x19C));
}

pub const HOOKS: &[HookReg] = &[
    HOOK_WRAP_ANGLE,
    HOOK_CLIP_BETA,
    HOOK_USING_3RD_PERSON_MOUSE_CAM,
    HOOK_GET_WEAPON_FIRST_PERSON_ON,
    HOOK_DO_CAM_BUMP,
    HOOK_WELL_BUFFER_ME,
    HOOK_GET_VECTORS_READY_FOR_RW,
    HOOK_CAM_CONSTRUCTOR,
    HOOK_CAM_INIT,
    HOOK_KEEP_TRACK_OF_THE_SPEED,
    HOOK_CACHE_LAST_SETTINGS_DW_CINEY_CAM,
];
