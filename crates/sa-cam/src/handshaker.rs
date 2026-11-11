//! PORT-CAM: `CHandShaker` (0x94 bytes; `gHandShaker[6]` at 0xB6ECA0). Ported from the 1.0 US bytes;
//! gta-reversed `HandShaker.cpp` and `docs/re/specs/CAM/CHandShaker.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{k, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `gHandShaker[6]` (CHandShaker, 0x94 bytes each)
    pub static G_HAND_SHAKER @ 0xB6ECA0: [u8; 6 * 0x94] = [0; 6 * 0x94];
}

/// `CHandShaker::~CHandShaker` (0x509810), the `eh vector` destructor argument of `__init_gHandShaker`.
pub const HAND_SHAKER_DTOR: u32 = 0x509810;
/// `CHandShaker::CHandShaker` (0x517740), the `eh vector` constructor argument.
pub const HAND_SHAKER_CTOR: u32 = 0x517740;

/// 1/32767 (0x858C7C), the `rand()` scale of `Reset`.
const RAND_SCALE: u32 = 0x3800_0100;

orig! {
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    pub fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CHandShaker::SetDefaults()` (0x517330)
    pub fn set_defaults_call(this: GPtr<u8>) = thiscall 0x517330 => hand_shaker_set_defaults;
    /// `CMatrix::SetRotate(float, float, float)` (0x59B120)
    pub fn matrix_set_rotate(m: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x59B120 => unported;
    /// `CHandShaker::Reset()` (0x50D860)
    pub fn reset_call(this: GPtr<u8>) = thiscall 0x50D860 => hand_shaker_reset;
}

/// `CHandShaker::~CHandShaker()` (0x509810): `add ecx, 0x3C; jmp CMatrix::~CMatrix` (the matrix member).
#[hook(addr = 0x509810, conv = thiscall, name = "CHandShaker::~CHandShaker", expect = "83 C1 3C E9 B8 14 09 00", this = "CHandShaker")]
pub fn hand_shaker_destructor(this: GPtr<u8>) {
    matrix_dtor(this.byte_add(0x3C));
}

/// `CHandShaker::CHandShaker()` (0x517740): the matrix member's attach pointer and owns flag (+0x7C,
/// +0x80) cleared, `SetDefaults()`, returns `this`.
#[hook(addr = 0x517740, conv = thiscall, name = "CHandShaker::CHandShaker", expect = "56 8B F1 33 C0", this = "CHandShaker")]
pub fn hand_shaker_constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x7C, 0);
    wr::<u32>(this, 0x80, 0);
    set_defaults_call(this);
    this
}

/// `CHandShaker::SetDefaults()` (0x517330): the tuning vectors +0xC (0.02, 0.02, 0.01), +0x18
/// (0.0002, 0.0002, 0.0001), +0x30 (1.3, 1.3, 1.4), +0x84 0.3, +0x88 1.0, +0x8C 20, +0x90 0.001, then
/// tail-jumps to `Reset()`.
#[hook(addr = 0x517330, conv = thiscall, name = "CHandShaker::SetDefaults", expect = "83 EC 0C C7 44 24 00 0A D7 A3 3C", this = "CHandShaker")]
pub fn hand_shaker_set_defaults(this: GPtr<u8>) {
    for (o, v) in [
        (0x0C, 0x3CA3_D70A),
        (0x10, 0x3CA3_D70A),
        (0x14, 0x3C23_D70A),
        (0x18, 0x3951_B717),
        (0x1C, 0x3951_B717),
        (0x20, 0x38D1_B717),
        (0x30, 0x3FA6_6666),
        (0x34, 0x3FA6_6666),
        (0x38, 0x3FB3_3333),
        (0x84, 0x3E99_999A),
        (0x88, 0x3F80_0000),
        (0x8C, 0x14),
        (0x90, 0x3A83_126F),
    ] {
        wr::<u32>(this, o, v);
    }
    reset_call(this);
}

/// `CHandShaker::Reset()` (0x50D860): +0 = (0, 0, 0); three `rand()` (return addresses 0x50D8A0,
/// 0x50D8BE, 0x50D8DC) scaled by 1/32767: `a` and `b` stored as f32, the third kept on the stack;
/// +0x24 = `r3 * k * [+0x18]`, +0x28 = `b * [+0x1C]`, +0x2C = `a * [+0x20]`.
#[hook(addr = 0x50D860, conv = thiscall, name = "CHandShaker::Reset", expect = "83 EC 24 56 8B F1", this = "CHandShaker")]
pub fn hand_shaker_reset(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    let s20: f32 = rd(this, 0x20);
    let r1 = sa_core::orig::at_site(0x50D8A0, sa_core::crt::rand);
    let a = xi(r1).fmul_m(k(RAND_SCALE)).to_f32();
    let s1c: f32 = rd(this, 0x1C);
    let r2 = sa_core::orig::at_site(0x50D8BE, sa_core::crt::rand);
    let b = xi(r2).fmul_m(k(RAND_SCALE)).to_f32();
    let s18: f32 = rd(this, 0x18);
    let r3 = sa_core::orig::at_site(0x50D8DC, sa_core::crt::rand);
    let c = xi(r3).fmul_m(k(RAND_SCALE)).fmul_m(s18).to_f32();
    let d = x(b).fmul_m(s1c).to_f32();
    let e = x(a).fmul_m(s20).to_f32();
    wr(this, 0x24, c);
    wr(this, 0x28, d);
    wr(this, 0x2C, e);
}

/// `CHandShaker::Process(float degree)` (0x50D930, `ret 4`), from the bytes (gta-reversed draws the random
/// numbers in another order and uses other constants: `docs/re/specs/CAM/CHandShaker.deviations.md`):
/// scales `fabs(ang/lim) * (max - min) + min` (stored f32); the slow factor when angle and velocity are both
/// strictly positive or both strictly negative; three `rand()` (z, y, x order) for the motion, negated where
/// the angle is positive, added to the velocity; `t = (float)(uint32)ftol(stepNonClipped * 0.02 * 1000) *
/// 0.03`; a twitch when `ftol((rand() & 0xFFFF) / 32768 * (ftol(freq * t) - 1)) == 1` (three more `rand()`,
/// z, y, x order); the angles advanced by `t * vel` and clamped to +-lim (y against its unstored sum); then
/// `m_resultMat.SetRotate(ang * degree)`. `rand()` goes through `at_site` with the original return addresses.
#[hook(addr = 0x50D930, conv = thiscall, name = "CHandShaker::Process", expect = "83 EC 34 56 8B F1", this = "CHandShaker")]
pub fn hand_shaker_process(this: GPtr<u8>, degree: f32) {
    let f = |o: isize| rd::<f32>(this, o);
    let rnd = |site: u32| sa_core::orig::at_site(site, sa_core::crt::rand);
    let zero = x(k(0));
    // 1. scales
    let d = x(f(0x88)).fsub_m(f(0x84));
    let mut sc = [0f32; 3];
    for (i, s) in sc.iter_mut().enumerate() {
        let o = 4 * i as isize;
        *s = x(f(o)).fdiv_m(f(0xC + o)).fabs().fmul(d).fadd_m(f(0x84)).to_f32();
    }
    // 2. slow factor
    for (i, s) in sc.iter_mut().enumerate() {
        let o = 4 * i as isize;
        let ang = x(f(o));
        let vel = || x(f(0x24 + o));
        let both = if !ang.fcom(zero).test_any(0x41) {
            !vel().fcom(zero).test_any(0x41)
        } else {
            !x(f(o)).fcom(zero).test_parity(0x05) && !vel().fcom(zero).test_parity(0x05)
        };
        if both {
            *s = x(*s).fmul_m(f(0x30 + o)).to_f32();
        }
    }
    // 3. random motion
    let c = k(RAND_SCALE);
    let mz = f(0x20);
    let r1 = xi(rnd(0x50DA6A)).fmul_m(c).to_f32();
    let my = f(0x1C);
    let r2 = xi(rnd(0x50DA88)).fmul_m(c).to_f32();
    let mx = f(0x18);
    let r3 = xi(rnd(0x50DAA6)).fmul_m(c);
    let mut vx = r3.fmul_m(mx).fmul_m(sc[0]);
    let mut vy = x(r2).fmul_m(my).fmul_m(sc[1]);
    let mut vz = x(r1).fmul_m(mz).fmul_m(sc[2]);
    let neg = k(0xBF80_0000); // -1.0 [0x858C1C]
    if !x(f(0)).fcom(zero).test_any(0x41) {
        vx = vx.fmul_m(neg);
    }
    if !x(f(4)).fcom(zero).test_any(0x41) {
        vy = vy.fmul_m(neg);
    }
    if !x(f(8)).fcom(zero).test_any(0x41) {
        vz = vz.fmul_m(neg);
    }
    wr::<f32>(this, 0x24, vx.fadd_m(f(0x24)).to_f32());
    wr::<f32>(this, 0x28, vy.fadd_m(f(0x28)).to_f32());
    wr::<f32>(this, 0x2C, vz.fadd_m(f(0x2C)).to_f32());
    // 4. time and twitch test
    let ms = x(sa_core::timer::TIME_STEP_NON_CLIPPED.get())
        .fmul_m(k(0x3CA3_D70A))
        .fmul_m(k(0x447A_0000));
    let msi = ms.ftol() as u32;
    let mut tf = xi(msi as i32);
    if (msi as i32) < 0 {
        tf = tf.fadd_m(k(0x4F80_0000));
    }
    let t = tf.fmul_m(k(0x3CF5_C28F)).to_f32(); // 0.03 [0x85A6EC]
    let n = xi(rd::<i32>(this, 0x8C)).fmul_m(t).ftol() as u32 as i32;
    let r = rnd(0x50DB7B) & 0xFFFF;
    let kk = xi(r).fmul_m(k(0x3800_0000)).fmul(xi(n.wrapping_sub(1))).ftol() as u32;
    if kk.wrapping_add(1) == 2 {
        // 5. twitch
        let tv = || f(0x90);
        let hi1 = tv();
        let lo1 = x(tv()).fchs().to_f32();
        let r1 = xi(rnd(0x50DBC2)).fmul_m(c).to_f32();
        let hi2 = tv();
        let lo2 = x(tv()).fchs().to_f32();
        let r2 = xi(rnd(0x50DBEF)).fmul_m(c).to_f32();
        let hi3 = tv();
        let lo3 = x(tv()).fchs().to_f32();
        let r3 = xi(rnd(0x50DC1C)).fmul_m(c);
        let c3 = x(hi3).fsub_m(lo3).fmul(r3).fadd_m(lo3);
        let c2 = x(hi2).fsub_m(lo2).fmul_m(r2).fadd_m(lo2);
        let c1 = x(hi1).fsub_m(lo1).fmul_m(r1).fadd_m(lo1).to_f32();
        wr::<f32>(this, 0x24, c3.fadd_m(f(0x24)).to_f32());
        wr::<f32>(this, 0x28, c2.fadd_m(f(0x28)).to_f32());
        wr::<f32>(this, 0x2C, x(c1).fadd_m(f(0x2C)).to_f32());
    }
    // 6. angles
    let dx = x(t).fmul_m(f(0x24));
    let dy = x(t).fmul_m(f(0x28));
    let dz = x(t).fmul_m(f(0x2C)).to_f32();
    let ax = dx.fadd_m(f(0)).to_f32();
    wr::<f32>(this, 0, ax);
    let ay = dy.fadd_m(f(4));
    let ay32 = ay.to_f32();
    wr::<f32>(this, 4, ay32);
    let az = x(dz).fadd_m(f(8)).to_f32();
    wr::<f32>(this, 8, az);
    // 7. clamps (x and z against the stored f32, y against the register)
    let clamp = |cur: X87, cur32: f32, o: isize| {
        if !cur.fcom(x(f(0xC + o))).test_any(0x41) {
            wr::<u32>(this, o, rd::<u32>(this, 0xC + o));
        } else {
            let lo = x(f(0xC + o)).fchs();
            if !x(cur32).fcom(lo).test_parity(0x05) {
                wr::<f32>(this, o, lo.to_f32());
            }
        }
    };
    clamp(x(ax), ax, 0);
    clamp(ay, ay32, 4);
    clamp(x(az), az, 8);
    // 8. the rotation
    let rz = x(degree).fmul_m(f(8)).to_f32();
    let ry = x(degree).fmul_m(f(4)).to_f32();
    let rx = x(degree).fmul_m(f(0)).to_f32();
    matrix_set_rotate(this.byte_add(0x3C), rx, ry, rz);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_HAND_SHAKER_DESTRUCTOR,
    HOOK_HAND_SHAKER_CONSTRUCTOR,
    HOOK_HAND_SHAKER_SET_DEFAULTS,
    HOOK_HAND_SHAKER_RESET,
    HOOK_HAND_SHAKER_PROCESS,
];
