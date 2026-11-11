//! `CQuaternion::Set` and `CQuaternion::Slerp`, used by the animation blending on the in-world path
//! (written by WRITER-MVP-D, batch `mvpd-math-1`).
//!
//! Ported from the 1.0 US bytes; gta-reversed `Quaternion.cpp` as reading aid. x87 arithmetic follows
//! the original's instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::common::{jp, rd, wr, x};

/// `CQuaternion::Set(const RwV3d& axis, float angle)` (0x59C600, thiscall, `ret 8`): h = angle * 0.5;
/// xyz = `sin(h) * axis`, w = `cos(h)` (FSIN/FCOS of the unrounded h).
#[hook(addr = 0x59C600, conv = thiscall, name = "CQuaternion::Set", this = "CQuaternion", expect = "D9 44 24 08 8B 44 24 04",
       unverified = "mvpd-math-1", route = "L2", hazards = "x87-transcendental", note = "FSIN/FCOS")]
pub fn quaternion_set(this: GPtr<u8>, axis: GPtr<u8>, angle: f32) {
    let h = x(angle).fmul_m(f32::from_bits(0x3F00_0000));
    let s = h.fsin();
    wr::<f32>(this, 0, s.fmul_m(rd::<f32>(axis, 0)).to_f32());
    wr::<f32>(this, 4, s.fmul_m(rd::<f32>(axis, 4)).to_f32());
    wr::<f32>(this, 8, s.fmul_m(rd::<f32>(axis, 8)).to_f32());
    wr::<f32>(this, 0xC, h.fcos().to_f32());
}

/// `CQuaternion::Slerp(const CQuaternion& from, const CQuaternion& to, float halfTheta, float
/// sinThetaInv, float t)` (0x59C300, thiscall, `ret 0x14`): `to` when halfTheta == 0; otherwise with
/// halfTheta above π/2 it becomes `π - halfTheta` (stored back into its own argument slot) and the
/// second weight is negated: `a = sin((1 - t) * ht) * inv`, `b = ±sin(ht * t) * inv`; x = `a f.x + b
/// t.x`, y/z/w = `b t.c + a f.c`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x59C300, conv = thiscall, name = "CQuaternion::Slerp", this = "CQuaternion", expect = "D9 44 24 0C D8 1D 50 8B 85 00",
       unverified = "mvpd-math-1", route = "L2", hazards = "x87-transcendental,stack-residue",
       note = "FSIN; writes pi - halfTheta into its own argument slot")]
pub fn quaternion_slerp(this: GPtr<u8>, from: GPtr<u8>, to: GPtr<u8>, half_theta: f32, sin_inv: f32, t: f32) {
    if !jp(x(half_theta), X87::zero(), 0x44) {
        for o in [0isize, 4, 8, 0xC] {
            wr::<u32>(this, o, rd::<u32>(to, o));
        }
        return;
    }
    let (a, b) = if !x(half_theta).fcom(x(f32::from_bits(0x3FC9_0FDB))).test_any(0x41) {
        let ht = x(f32::from_bits(0x4049_0FDB)).fsub_m(half_theta).to_f32();
        let a = X87::one().fsub_m(t).fmul_m(ht).fsin().fmul_m(sin_inv);
        let b = x(ht).fmul_m(t).fsin().fmul_m(sin_inv).fchs();
        (a, b)
    } else {
        let a = X87::one().fsub_m(t).fmul_m(half_theta).fsin().fmul_m(sin_inv);
        let b = x(half_theta).fmul_m(t).fsin().fmul_m(sin_inv);
        (a, b)
    };
    wr::<f32>(this, 0, a.fmul_m(rd::<f32>(from, 0)).fadd(b.fmul_m(rd::<f32>(to, 0))).to_f32());
    for o in [4isize, 8, 0xC] {
        wr::<f32>(this, o, b.fmul_m(rd::<f32>(to, o)).fadd(a.fmul_m(rd::<f32>(from, o))).to_f32());
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_QUATERNION_SET, HOOK_QUATERNION_SLERP];
