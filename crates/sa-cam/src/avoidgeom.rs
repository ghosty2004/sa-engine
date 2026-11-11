//! PORT-CAM-3 (helper A): `CCamera::AvoidTheGeometry` (0x514030), ported from the 1.0 US bytes
//! (gta-reversed: not reversed). Same technique as [`crate::followped`]: a frame array at the
//! original's `B`-relative offsets (B = ESP after the four register pushes; `B+0x60` is the CColPoint of
//! the line-of-sight tests) and the [`Fpu`] machine. FPTAN (0x5143D1) uses sa-math's 64-bit model;
//! FSIN / FCOS the f64 models; `GetATanOfXY` (raw FPATAN in ST0) comes back through `orig!` as an f64
//! (64 -> 53 -> 24 bit double rounding, the documented CAM approximation).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::platform::SCENE_CAMERA;
use sa_core::{hook, orig};

use crate::camcol::{set_near_clip_raw, vector_sub};
use crate::cammodes::test_sphere_against_world;
use crate::camproc::{cross_product, find_player_ped, get_atan_of_xy_bits, vector_normalise};
use crate::common::{Fpu, g, k, rd, wr};
use crate::followped::{ci_pow, copy3, d, fcos, fsin, push_st0};

orig! {
    /// `CWorld::ProcessLineOfSight(const CVector&, const CVector&, CColPoint&, CEntity*&, bool buildings,
    /// bool vehicles, bool peds, bool objects, bool dummies, bool seeThrough, bool camIgnore,
    /// bool shootThrough)` (0x56BA00, AL)
    pub fn process_line_of_sight(a: GPtr<u8>, b: GPtr<u8>, cp: GPtr<u8>, hit: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, cam: u32, sh: u32) -> u8 = cdecl 0x56BA00 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    pub fn placeable_get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `CCam::WellBufferMe(float target, float* value, float* speed, float topSpeed, float step, bool angle)`
    /// (0x509AE0, cdecl); the floats as dwords
    pub fn well_buffer_me(target: u32, value: GPtr<u8>, speed: GPtr<u8>, top: u32, step: u32, angle: u32) = cdecl 0x509AE0 => unported;
}

/// `CWorld::pIgnoreEntity` (0xB7CD68).
const IGNORE_ENTITY: usize = 0xB7CD68;
/// `gaTempSphereColPoints[0]` (0xB9B250: position, +0x10 normal) of `TestSphereAgainstWorld`.
const TEMP_SPHERE_COL_POINT: usize = 0xB9B250;
/// The second line-of-sight switch (u8 0xB6EC65) and a byte read but not used (0xBA6793).
const SECOND_LOS: usize = 0xB6EC65;
const UNUSED_BYTE: usize = 0xBA6793;
/// The avoidance blend (`value` 0xB6EC38, `speed` 0xB6EC3C) of `WellBufferMe`.
const AVOID_VALUE: u32 = 0xB6EC38;
const AVOID_SPEED: u32 = 0xB6EC3C;
const TIME_STEP: usize = 0xB7CB5C;
const ASPECT_RATIO: usize = 0xC3EFA4;
/// `.data` tweak values: near-clip margin 0.5 (0x8CC38C), minimum near clip 0.15 (0x8CC390), the
/// smoothing base 0.9 (0x8CC81C) and the sphere scale 1.15 (0x8CC820).
const NEAR_MARGIN: usize = 0x8CC38C;
const NEAR_MIN: usize = 0x8CC390;
const SMOOTH: usize = 0x8CC81C;
const SPHERE_SCALE: usize = 0x8CC820;

const K_0: u32 = 0; // [0x858B50]
const K_0_5: u32 = 0x3F00_0000; // [0x858B8C]
const K_0_9: u32 = 0x3F66_6666; // [0x858C20]
const K_DEG2RAD: u32 = 0x3C8E_FA35; // [0x8595EC]

/// The near plane of `Scene.m_pRwCamera` (+0x80).
fn near_clip() -> f32 {
    rd::<f32>(SCENE_CAMERA.get(), 0x80)
}

/// `fld [a]; fsub [b]` x3, `(dx*dx + dz*dz) + dy*dy` (register order of the bytes), `fsqrt`.
fn dist_xzy(f: &mut Fpu, a: GPtr<u8>, b: GPtr<u8>) {
    f.fld(rd(a, 0));
    f.fsub_m(rd(b, 0));
    f.fld(rd(a, 4));
    f.fsub_m(rd(b, 4));
    f.fld(rd(a, 8));
    f.fsub_m(rd(b, 8));
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
}

/// `CCamera::AvoidTheGeometry(const CVector& from, const CVector& target, CVector& out, float fov)`
/// (0x514030, thiscall, `ret 0x10`):
/// 1. `D = target - from`; `this+0x8FC = 0`; `dist = |D|`, `h = |D.xy|`; beta = `GetATanOfXY(D.x, D.y)`
///    (`GetATanOfXY(+0x984, +0x988)` when D.x and D.y are both 0), alpha = `GetATanOfXY(h, D.z)` (0.0 when
///    h and D.z are both 0); `out = target - dir * dist` with `dir = (cos b cos a, sin b cos a, sin a)`
///    (then normalised in its local).
/// 2. `ProcessLineOfSight(target, out, buildings + objects, camIgnore)` ignoring `m_pTargetEntity`: a hit
///    moves out to the hit point; with 0xB6EC65 set a second test `out -> target` (vehicles, peds,
///    objects) replaces it by the nearer point when that one is closer to `out` than the near clip
///    (else re-stores the first hit).
/// 3. With a player ped: near clip `= max(|target - out| - 0.5, 0.15)` when that is below the current one.
/// 4. A sphere `r = near * tan(fov/2) * aspect * 1.15` at `out + dir * near` against buildings and
///    objects: a hit sets the near clip from the contact (0.15 .. 0.9), pushes along the contact normal
///    (`this+0x8FC = n * ((r - |c|) * -(c . n))` with the normal flipped to face the centre), the ped
///    target's `+0x164` timer and `+0x168` side; then `+0x164 *= pow(0.9, ts)`,
///    `WellBufferMe(hit ? 1 : 0, &0xB6EC38, &0xB6EC3C, 0.2, 0.05, false)`, `this+0x8FC *= [0xB6EC38]`,
///    `this+0x2D = 1`.
#[hook(addr = 0x514030, conv = thiscall, name = "CCamera::AvoidTheGeometry", expect = "83 EC 7C 8B 84 24 80 00 00 00", this = "CCamera")]
pub fn avoid_the_geometry(this: GPtr<u8>, from: GPtr<u8>, target: GPtr<u8>, out: GPtr<u8>, fov: u32) {
    let mut fr = [0u32; 0x24];
    let b = GPtr::<u32>::from_raw(fr.as_mut_ptr()).cast::<u8>();
    let mut fpu = Fpu::new();
    let f = &mut fpu;
    let push_vec = this.byte_add(0x8FC);
    // 1. direction and distance
    f.fld(rd(target, 0));
    f.fsub_m(rd(from, 0));
    wr::<u32>(b, 0x3C, 0);
    wr::<u32>(b, 0x40, 0);
    wr::<f32>(b, 0x30, f.fst());
    f.fld(rd(target, 4));
    wr::<u32>(b, 0x44, 0);
    f.fsub_m(rd(from, 4));
    wr::<u32>(b, 0x20, 0);
    wr::<f32>(b, 0x34, f.fstp());
    f.fld(rd(target, 8));
    f.fsub_m(rd(from, 8));
    copy3(push_vec, b.byte_add(0x3C));
    wr::<f32>(b, 0x38, f.fstp());
    f.fld(rd(b, 0x34));
    f.fmul_m(rd(b, 0x34));
    f.fld_st(1);
    f.fmul_st(2);
    f.fld(rd(b, 0x38));
    f.fmul_m(rd(b, 0x38));
    f.fadd_st(1);
    f.fadd_st(2);
    f.fsqrt();
    wr::<f32>(b, 0x14, f.fstp());
    f.faddp(1);
    f.fsqrt();
    wr::<f32>(b, 0x18, f.fstp());
    f.fcomp_m(k(K_0));
    let use_xy = f.jp(0x44) || {
        f.fld(rd(b, 0x34));
        f.fcomp_m(k(K_0));
        f.jp(0x44)
    };
    let beta = if use_xy {
        get_atan_of_xy_bits(rd::<u32>(b, 0x30), rd::<u32>(b, 0x34))
    } else {
        get_atan_of_xy_bits(rd::<u32>(this, 0x984), rd::<u32>(this, 0x988))
    };
    push_st0(f, beta);
    wr::<f32>(b, 0x10, f.fstp());
    f.fld(rd(b, 0x18));
    f.fcomp_m(k(K_0));
    let use_hz = f.jp(0x44) || {
        f.fld(rd(b, 0x38));
        f.fcomp_m(k(K_0));
        f.jp(0x44)
    };
    if use_hz {
        let a = get_atan_of_xy_bits(rd::<u32>(b, 0x18), rd::<u32>(b, 0x38));
        push_st0(f, a);
    } else {
        f.fld(rd(b, 0x20));
    }
    // dir = (cos b cos a, sin b cos a, sin a) into B+0x3C; out = target - dir * dist
    f.fld_st(0);
    fcos(f);
    f.fld(rd(b, 0x10));
    fcos(f);
    f.fmul_st(1);
    wr::<f32>(b, 0x3C, f.fstp());
    f.fld(rd(b, 0x10));
    fsin(f);
    f.fmul_st(1);
    wr::<f32>(b, 0x40, f.fstp());
    f.fstp_st(0);
    fsin(f);
    wr::<f32>(b, 0x44, f.fstp());
    f.fld(rd(b, 0x3C));
    f.fmul_m(rd(b, 0x14));
    f.fld(rd(b, 0x40));
    f.fmul_m(rd(b, 0x14));
    f.fld(rd(b, 0x44));
    f.fmul_m(rd(b, 0x14));
    wr::<f32>(b, 0x5C, f.fstp());
    f.fld(rd(target, 0));
    f.fsub_st(2);
    wr::<f32>(b, 0x30, f.fstp());
    f.fld(rd(target, 4));
    f.fsub_st(1);
    wr::<f32>(b, 0x34, f.fstp());
    f.fstp_st(0);
    f.fstp_st(0);
    f.fld(rd(target, 8));
    wr::<u32>(out, 0, rd::<u32>(b, 0x30));
    f.fsub_m(rd(b, 0x5C));
    wr::<u32>(out, 4, rd::<u32>(b, 0x34));
    wr::<f32>(b, 0x38, f.fstp());
    wr::<u32>(out, 8, rd::<u32>(b, 0x38));
    vector_normalise(b.byte_add(0x3C));
    // 2. line of sight target -> out
    let cp = b.byte_add(0x60);
    let hit_ent = b.byte_add(0x1C);
    let te = rd::<GPtr<u8>>(this, 0x958);
    wr::<u32>(b, 0x1C, 0);
    g::<GPtr<u8>>(IGNORE_ENTITY).write(te);
    if process_line_of_sight(target, out, cp, hit_ent, 1, 0, 0, 1, 0, 0, 1, 0) != 0 {
        copy3(out, cp);
        copy3(b.byte_add(0x30), cp);
        if g::<u8>(SECOND_LOS).read() != 0
            && process_line_of_sight(out, target, cp, hit_ent, 0, 1, 1, 1, 0, 0, 1, 0) != 0
        {
            dist_xzy(f, out, cp);
            f.fcomp_m(near_clip());
            if !f.jp(0x5) {
                copy3(out, cp);
            } else {
                dist_xzy(f, out, b.byte_add(0x30));
                f.fcomp_m(near_clip());
                if !f.jp(0x5) {
                    copy3(out, b.byte_add(0x30));
                }
            }
        }
    }
    g::<u32>(IGNORE_ENTITY).write(0);
    // 3. near clip from the distance to the target (only with a player ped)
    for i in 0..3 {
        f.fld(rd(target, 4 * i));
        f.fsub_m(rd(out, 4 * i));
        wr::<f32>(b, 0x30 + 4 * i, f.fstp());
    }
    if !find_player_ped(-1).is_null() {
        let cam = SCENE_CAMERA.get();
        f.fld(rd(b, 0x30));
        f.fmul_m(rd(b, 0x30));
        f.fld(rd(b, 0x38));
        f.fmul_m(rd(b, 0x38));
        f.faddp(1);
        f.fld(rd(b, 0x34));
        f.fmul_m(rd(b, 0x34));
        f.faddp(1);
        f.fsqrt();
        f.fsub_m(d(NEAR_MARGIN));
        f.fld(rd(cam, 0x80));
        f.fld_st(1);
        f.fcompp();
        if f.jp(0x5) {
            f.fstp_st(0);
        } else {
            f.fcom_m(d(NEAR_MIN));
            if !f.jne(0x41) {
                wr::<f32>(b, 0x18, f.fstp());
                set_near_clip_raw(cam, rd::<u32>(b, 0x18));
            } else {
                let m = g::<u32>(NEAR_MIN).read();
                f.fstp_st(0);
                wr::<u32>(b, 0x18, m);
                set_near_clip_raw(cam, m);
            }
        }
    }
    // 4. the sphere in front of the camera (esp = B-0x1C after the pushes, B-0x2C at the call)
    let cam = SCENE_CAMERA.get();
    f.fld(f32::from_bits(fov));
    f.fmul_m(k(K_DEG2RAD));
    let _ = g::<u8>(UNUSED_BYTE).read();
    f.fmul_m(k(K_0_5));
    f.fptan();
    wr::<u32>(b, 0x10, 0);
    f.fstp_st(0);
    f.fmul_m(d(ASPECT_RATIO));
    f.fmul_m(d(SPHERE_SCALE));
    f.fld(rd(cam, 0x80));
    f.fld_st(0);
    f.fmul_st(2);
    wr::<f32>(b, 0x18, f.fstp());
    f.fld(rd(b, 0x3C));
    let radius = rd::<u32>(b, 0x18);
    f.fmul_st(1);
    wr::<f32>(b, 0x48, f.fstp());
    f.fld(rd(b, 0x40));
    f.fmul_st(1);
    wr::<f32>(b, 0x4C, f.fstp());
    f.fld(rd(b, 0x44));
    f.fmul_st(1);
    f.fstp_st(2);
    f.fstp_st(0);
    f.fld(rd(b, 0x48));
    f.fadd_m(rd(out, 0));
    wr::<f32>(b, 0x24, f.fstp());
    f.fld(rd(b, 0x4C));
    f.fadd_m(rd(out, 4));
    wr::<f32>(b, 0x28, f.fstp());
    f.fadd_m(rd(out, 8));
    wr::<f32>(b, 0x2C, f.fstp());
    let hit = test_sphere_against_world(
        rd::<u32>(b, 0x24),
        rd::<u32>(b, 0x28),
        rd::<u32>(b, 0x2C),
        radius,
        GPtr::NULL,
        1,
        0,
        0,
        1,
        0,
        1,
    );
    if !hit.is_null() {
        sphere_push(this, b, out, f);
    }
    // 0x5147B9: decay, blend, scaled push vector
    f.fld(d(SMOOTH));
    f.fld(d(TIME_STEP));
    ci_pow(f);
    let target_v = rd::<u32>(b, 0x10);
    f.fmul_m(rd(this, 0x164));
    wr::<f32>(this, 0x164, f.fstp());
    well_buffer_me(
        target_v,
        GPtr::from_addr(AVOID_VALUE as usize),
        GPtr::from_addr(AVOID_SPEED as usize),
        0x3E4C_CCCD,
        0x3D4C_CCCD,
        0,
    );
    let mut out3 = [0u32; 3];
    for (i, o) in out3.iter_mut().enumerate() {
        f.fld(d(AVOID_VALUE as usize));
        f.fmul_m(rd(push_vec, 4 * i as isize));
        *o = f.fstp().to_bits();
    }
    wr::<u32>(push_vec, 0, out3[0]);
    wr::<u32>(push_vec, 4, out3[1]);
    wr::<u8>(this, 0x2D, 1);
    wr::<u32>(push_vec, 8, out3[2]);
}

/// 0x514463: the sphere touched something: near clip from the contact, the push vector along the
/// contact normal, and the ped target's timer (+0x164) / side (+0x168).
fn sphere_push(this: GPtr<u8>, b: GPtr<u8>, out: GPtr<u8>, f: &mut Fpu) {
    let cpp = |i: isize| d(TEMP_SPHERE_COL_POINT + 4 * i as usize);
    // c = contact - centre (B+0x30); proj = (contact - out) . dir
    for i in 0..3 {
        f.fld(cpp(i));
        f.fsub_m(rd(b, 0x24 + 4 * i));
        wr::<f32>(b, 0x30 + 4 * i, f.fstp());
    }
    f.fld(cpp(0));
    f.fsub_m(rd(out, 0));
    f.fld(cpp(1));
    f.fsub_m(rd(out, 4));
    f.fld(cpp(2));
    f.fsub_m(rd(out, 8));
    f.fxch(2);
    f.fmul_m(rd(b, 0x3C));
    f.fxch(2);
    f.fmul_m(rd(b, 0x44));
    f.faddp(2);
    f.fmul_m(rd(b, 0x40));
    f.faddp(1);
    f.fcom_m(d(NEAR_MIN));
    wr::<f32>(b, 0x20, f.fst());
    let cam = SCENE_CAMERA.get();
    if !f.jne(0x41) && {
        f.fcom_m(k(K_0_9));
        !f.jp(0x5)
    } {
        f.fld(rd(cam, 0x80));
        f.fxch(1);
        f.fcompp();
        if !f.jp(0x5) {
            set_near_clip_raw(cam, rd::<u32>(b, 0x20));
        }
    } else {
        f.fcomp_m(d(NEAR_MIN));
        if !f.jp(0x5) {
            set_near_clip_raw(cam, g::<u32>(NEAR_MIN).read());
        }
    }
    // s = r - |c|; c normalised; n = contact normal, normalised
    f.fld(rd(b, 0x30));
    f.fmul_m(rd(b, 0x30));
    f.fld(rd(b, 0x38));
    f.fmul_m(rd(b, 0x38));
    f.faddp(1);
    f.fld(rd(b, 0x34));
    f.fmul_m(rd(b, 0x34));
    f.faddp(1);
    f.fsqrt();
    f.fsubr_m(rd(b, 0x18));
    wr::<f32>(b, 0x14, f.fstp());
    vector_normalise(b.byte_add(0x30));
    wr::<u32>(b, 0x2C, g::<u32>(TEMP_SPHERE_COL_POINT + 0x18).read());
    wr::<u32>(b, 0x24, g::<u32>(TEMP_SPHERE_COL_POINT + 0x10).read());
    wr::<u32>(b, 0x28, g::<u32>(TEMP_SPHERE_COL_POINT + 0x14).read());
    vector_normalise(b.byte_add(0x24));
    f.fld(rd(b, 0x30));
    f.fmul_m(rd(b, 0x14));
    f.fld(rd(b, 0x34));
    f.fmul_m(rd(b, 0x14));
    wr::<f32>(b, 0x58, f.fstp());
    f.fld(rd(b, 0x38));
    f.fmul_m(rd(b, 0x14));
    wr::<f32>(b, 0x5C, f.fstp());
    // flip n to face against c: (-c) . n < 0 keeps it, else n = -n
    f.fld(rd(b, 0x30));
    f.fchs();
    f.fld(rd(b, 0x34));
    f.fchs();
    f.fld(rd(b, 0x38));
    f.fchs();
    f.fxch(2);
    f.fmul_m(rd(b, 0x24));
    f.fld(rd(b, 0x2C));
    f.fmul_st(3);
    f.faddp(1);
    f.fld(rd(b, 0x28));
    f.fmul_st(2);
    f.faddp(1);
    f.fcomp_m(k(K_0));
    f.fstp_st(1);
    f.fstp_st(0);
    if !f.jp(0x5) {
        for (i, o) in [0x48, 0x4C, 0x50].into_iter().enumerate() {
            f.fld(rd(b, 0x24 + 4 * i as isize));
            f.fchs();
            wr::<f32>(b, o, f.fstp());
            wr::<u32>(b, 0x24 + 4 * i as isize, rd::<u32>(b, o));
        }
    }
    // q = (-s*c) . n; push = n * q
    let push_vec = this.byte_add(0x8FC);
    f.fchs();
    f.fld(rd(b, 0x58));
    wr::<u32>(b, 0x10, 0x3F80_0000);
    f.fchs();
    f.fld(rd(b, 0x5C));
    f.fchs();
    f.fxch(2);
    f.fmul_m(rd(b, 0x24));
    f.fld_st(2);
    f.fmul_m(rd(b, 0x2C));
    f.faddp(1);
    f.fxch(1);
    f.fmul_m(rd(b, 0x28));
    f.faddp(1);
    f.fxch(1);
    f.fstp_st(0);
    f.fld(rd(b, 0x24));
    f.fmul_st(1);
    wr::<f32>(b, 0x48, f.fstp());
    wr::<u32>(push_vec, 0, rd::<u32>(b, 0x48));
    f.fld(rd(b, 0x28));
    f.fmul_st(1);
    wr::<f32>(b, 0x4C, f.fstp());
    f.fld(rd(b, 0x2C));
    f.fmul_st(1);
    wr::<u32>(push_vec, 4, rd::<u32>(b, 0x4C));
    let te = rd::<GPtr<u8>>(this, 0x958);
    wr::<f32>(b, 0x50, f.fstp());
    f.fstp_st(0);
    wr::<u32>(push_vec, 8, rd::<u32>(b, 0x50));
    if te.is_null() || rd::<u8>(te, 0x36) & 7 != 3 {
        return;
    }
    // a ped target close to the camera
    f.fld(near_clip());
    f.fld(d(NEAR_MIN));
    f.fadd_st(0);
    f.fcompp();
    if f.jne(0x41) {
        return;
    }
    let dot_fwd = |f: &mut Fpu, m: GPtr<u8>| {
        f.fld(rd(b, 0x2C));
        f.fmul_m(rd(m, 0x18));
        let fw = m.byte_add(0x10);
        f.fld(rd(b, 0x28));
        f.fmul_m(rd(fw, 4));
        f.faddp(1);
        f.fld(rd(b, 0x24));
        f.fmul_m(rd(fw, 0));
        f.faddp(1);
    };
    dot_fwd(f, placeable_get_matrix(te));
    f.fcomp_m(k(K_0));
    let timer = this.byte_add(0x164);
    if !f.jp(0x5) {
        f.fld(rd(timer, 0));
        f.fcomp_m(k(K_0));
        if !f.jp(0x5) {
            wr::<u32>(timer, 0, 0);
        }
        f.fld(d(TIME_STEP));
        f.fadd_m(rd(timer, 0));
        wr::<f32>(timer, 0, f.fstp());
    } else {
        dot_fwd(f, placeable_get_matrix(rd::<GPtr<u8>>(this, 0x958)));
        f.fcomp_m(k(K_0_5));
        if !f.jne(0x41) {
            f.fld(rd(timer, 0));
            f.fcomp_m(k(K_0));
            if !f.jne(0x41) {
                wr::<u32>(timer, 0, 0);
            }
            f.fld(rd(timer, 0));
            f.fsub_m(d(TIME_STEP));
            wr::<f32>(timer, 0, f.fstp());
        }
    }
    if rd::<i16>(this, 0x168) != 0 {
        return;
    }
    let te = rd::<GPtr<u8>>(this, 0x958);
    let m = rd::<GPtr<u8>>(te, 0x14);
    let pos = if m.is_null() {
        te.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    let s = vector_sub(b.byte_add(0x54), pos, out);
    let c = cross_product(b.byte_add(0x48), s, b.byte_add(0x24));
    f.fld(rd(c, 8));
    f.fcomp_m(k(K_0));
    wr::<i16>(this, 0x168, if !f.jne(0x41) { -1 } else { 1 });
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_AVOID_THE_GEOMETRY];

/// Globals of this module (none: the statics are accessed by address).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
