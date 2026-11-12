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
       route = "L2", hazards = "x87-transcendental", note = "FSIN/FCOS")]
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
       route = "L2", hazards = "x87-transcendental,stack-residue",
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

/// One branch of `CQuaternion::Set(const RwMatrix&)`: `s = sqrt(t + 1)` (register), the component `big` = f32(0.5 * s),
/// `inv = 0.5 / s`; each `(o, a, b, add)` writes f32((m[a] ± m[b]) * inv) to `o`.
fn quaternion_from_matrix_branch(this: GPtr<u8>, m: GPtr<u8>, t: X87, big: isize, rest: [(isize, isize, isize, bool); 3]) {
    let s = t.fadd_m(1.0).fsqrt();
    wr::<f32>(this, big, x(0.5).fmul(s).to_f32());
    let inv = s.fdivr_m(0.5);
    for (o, a, b, add) in rest {
        let v = if add { x(rd::<f32>(m, a)).fadd_m(rd::<f32>(m, b)) } else { x(rd::<f32>(m, a)).fsub_m(rd::<f32>(m, b)) };
        wr::<f32>(this, o, v.fmul(inv).to_f32());
    }
}

/// `CQuaternion::Set(const RwMatrix& m)` (0x59C3E0, thiscall, `ret 4`). x87 (FSQRT). With the matrix rows right
/// (+0), up (+0x10), at (+0x20): trace `up.y + right.x + at.z` not below 0 (`test ah, 1`; NaN falls through) → w
/// branch: `w = 0.5 s`, `x = (up.z - at.y) inv`, `y = (at.x - right.z) inv`, `z = (right.y - up.x) inv`; else `right.x
/// - up.y - at.z` not below 0 → `x = 0.5 s`, `y = (up.x + right.y)`, `z = (at.x + right.z)`, `w = (up.z - at.y)`; else
/// `up.y - right.x - at.z` not below 0 → `y`, `w = (at.x - right.z)`, `x = (up.x + right.y)`, `z = (at.y + up.z)`; else
/// `at.z - (up.y + right.x)` → `z`, `w = (right.y - up.x)`, `x = (at.x + right.z)`, `y = (at.y + up.z)`; in each
/// branch `s = sqrt(t + 1)`, `inv = 0.5 / s` (register), every component stored as f32 in that order.
#[hook(addr = 0x59c3e0, conv = thiscall, name = "CQuaternion::Set", this = "CQuaternion", expect = "8B 54 24 04 D9 42 14",
       route = "L2", hazards = "x87-transcendental", note = "FSQRT; the RwMatrix overload")]
pub fn quaternion_set_from_matrix(this: GPtr<u8>, m: GPtr<u8>) {
    let f = |o: isize| rd::<f32>(m, o);
    let t = x(f(0x14)).fadd_m(f(0)).fadd_m(f(0x28));
    if !t.fcom(X87::zero()).c0() {
        quaternion_from_matrix_branch(this, m, t, 0xC, [(0, 0x18, 0x24, false), (4, 0x20, 8, false), (8, 4, 0x10, false)]);
        return;
    }
    let t = x(f(0)).fsub_m(f(0x14)).fsub_m(f(0x28));
    if !t.fcom(X87::zero()).c0() {
        quaternion_from_matrix_branch(this, m, t, 0, [(4, 0x10, 4, true), (8, 0x20, 8, true), (0xC, 0x18, 0x24, false)]);
        return;
    }
    let t = x(f(0x14)).fsub_m(f(0)).fsub_m(f(0x28));
    if !t.fcom(X87::zero()).c0() {
        quaternion_from_matrix_branch(this, m, t, 4, [(0xC, 0x20, 8, false), (0, 0x10, 4, true), (8, 0x24, 0x18, true)]);
        return;
    }
    let t = x(f(0x14)).fadd_m(f(0)).fsubr_m(f(0x28));
    quaternion_from_matrix_branch(this, m, t, 8, [(0xC, 4, 0x10, false), (0, 0x20, 8, true), (4, 0x24, 0x18, true)]);
}

pub const HOOKS: &[HookReg] = &[HOOK_QUATERNION_SET, HOOK_QUATERNION_SLERP, HOOK_QUATERNION_SET_FROM_MATRIX];
