//! STD-8: `CMatrix::Reorthogonalise` and `CMatrix::ForceUpVector`, ported from the 1.0 US bytes with the original's
//! x87 grouping (PC=24 register values; the results stored as f32). `CMatrix`: right +0, forward +0x10, up +0x20.
//! Unverified (batch `std8-math-1`).

use sa_math::x87::X87;

use crate::abi::HookReg;
use crate::hook;
use crate::mem::GPtr;

fn ldf(p: GPtr<u8>, o: isize) -> f32 {
    p.byte_add(o).cast::<f32>().read()
}
fn st(p: GPtr<u8>, o: isize, v: X87) {
    p.byte_add(o).cast::<f32>().write(v.to_f32())
}

/// `1.0f` (0x858624, the `fdivr` constant)
fn one() -> f32 {
    GPtr::<f32>::from_addr(0x858624).read()
}

/// `CMatrix::Reorthogonalise()` (0x59B6A0, thiscall). With right R (+0) and forward F (+0x10), in x87 registers:
/// `a = (R.y*F.z - F.y*R.z, F.x*R.z - R.x*F.z, F.y*R.x - F.x*R.y)`, `sy = f32(a.y)`, `sz = f32(a.z)`,
/// `inv = 1 / sqrt(a.z*sz + sy*sy + a.x*a.x)`, `k = f32(inv)`; the new up `A = (a.x*inv, k*sy, k*sz)`, `Az = f32(A.z)`;
/// `b = (A.z*F.y - A.y*F.z, A.x*F.z - Az*F.x, A.y*F.x - A.x*F.y)`, `ty = f32(b.y)`, `tz = f32(b.z)`,
/// `k2 = f32(1 / sqrt(b.z*tz + ty*ty + b.x*b.x))`; the new right `B = (b.x*k2, ty*k2, tz*k2)`, `Bz = f32(B.z)`; the new
/// forward `C = (B.z*A.y - B.y*Az, B.x*Az - Bz*A.x, B.y*A.x - B.x*A.y)`. Everything stored as f32 (the up z as `Az`).
#[hook(addr = 0x59B6A0, conv = thiscall, name = "CMatrix::Reorthogonalise", this = "CMatrix",
       expect = "83 EC 28 D9 41 04 D8 49 18", route = "L1-pure",
       hazards = "x87-transcendental", note = "FSQRT twice")]
pub fn matrix_reorthogonalise(m: GPtr<u8>) {
    let (r0, r1, r2) = (ldf(m, 0), ldf(m, 4), ldf(m, 8));
    let (f0, f1, f2) = (ldf(m, 0x10), ldf(m, 0x14), ldf(m, 0x18));
    let a0 = X87::from_f32(r1).fmul_m(f2).fsub(X87::from_f32(f1).fmul_m(r2));
    let sy = X87::from_f32(f0)
        .fmul_m(r2)
        .fsub(X87::from_f32(r0).fmul_m(f2))
        .to_f32();
    let a2 = X87::from_f32(f1).fmul_m(r0).fsub(X87::from_f32(f0).fmul_m(r1));
    let sz = a2.to_f32();
    let sum = a2.fmul_m(sz).fadd(X87::from_f32(sy).fmul_m(sy)).fadd(a0.fmul(a0));
    let inv = sum.fsqrt().fdivr_m(one());
    let k = inv.to_f32();
    let up0 = a0.fmul(inv);
    let up1 = X87::from_f32(k).fmul_m(sy);
    let up2 = X87::from_f32(k).fmul_m(sz);
    let up2s = up2.to_f32();
    let b0 = up2.fmul_m(f1).fsub(up1.fmul_m(f2));
    let ty = up0.fmul_m(f2).fsub(X87::from_f32(up2s).fmul_m(f0)).to_f32();
    let b2 = up1.fmul_m(f0).fsub(up0.fmul_m(f1));
    let tz = b2.to_f32();
    let sum2 = b2.fmul_m(tz).fadd(X87::from_f32(ty).fmul_m(ty)).fadd(b0.fmul(b0));
    let k2 = sum2.fsqrt().fdivr_m(one()).to_f32();
    let rt0 = b0.fmul_m(k2);
    let rt1 = X87::from_f32(ty).fmul_m(k2);
    let rt2 = X87::from_f32(tz).fmul_m(k2);
    let rt2s = rt2.to_f32();
    let c0 = rt2.fmul(up1).fsub(rt1.fmul_m(up2s));
    let c1 = rt0.fmul_m(up2s).fsub(X87::from_f32(rt2s).fmul(up0));
    let c2 = rt1.fmul(up0).fsub(rt0.fmul(up1));
    st(m, 0, rt0);
    st(m, 4, rt1);
    st(m, 8, rt2);
    st(m, 0x10, c0);
    st(m, 0x14, c1);
    st(m, 0x18, c2);
    st(m, 0x20, up0);
    st(m, 0x24, up1);
    st(m, 0x28, up2);
}

/// `CMatrix::ForceUpVector(CVector up)` (0x59B7E0, thiscall, `ret 0xC`). With forward F (+0x10), in x87 registers:
/// `a = (u.z*F.y - u.y*F.z, u.x*F.z - u.z*F.x, u.y*F.x - u.x*F.y)`, `sz = f32(a.z)`; right = `a`; forward =
/// `(a.z*u.y - a.y*u.z, a.x*u.z - sz*u.x, u.x*a.y - a.x*u.y)`; up = `u` (raw copies). Stored as f32.
#[hook(addr = 0x59B7E0, conv = thiscall, name = "CMatrix::ForceUpVector", this = "CMatrix",
       expect = "83 EC 18 D9 44 24 24 D8 49 14", route = "L1-pure", hazards = "none")]
pub fn matrix_force_up_vector(m: GPtr<u8>, ux: f32, uy: f32, uz: f32) {
    let (f0, f1, f2) = (ldf(m, 0x10), ldf(m, 0x14), ldf(m, 0x18));
    let a0 = X87::from_f32(uz).fmul_m(f1).fsub(X87::from_f32(uy).fmul_m(f2));
    let a1 = X87::from_f32(ux).fmul_m(f2).fsub(X87::from_f32(uz).fmul_m(f0));
    let a2 = X87::from_f32(uy).fmul_m(f0).fsub(X87::from_f32(ux).fmul_m(f1));
    let sz = a2.to_f32();
    let c0 = a2.fmul_m(uy).fsub(a1.fmul_m(uz));
    let c1 = a0.fmul_m(uz).fsub(X87::from_f32(sz).fmul_m(ux));
    let c2 = X87::from_f32(ux).fmul(a1).fsub(a0.fmul_m(uy));
    st(m, 0, a0);
    st(m, 4, a1);
    m.byte_add(8).cast::<f32>().write(sz);
    st(m, 0x10, c0);
    st(m, 0x14, c1);
    st(m, 0x18, c2);
    for (o, v) in [(0x20, ux), (0x24, uy), (0x28, uz)] {
        m.byte_add(o).cast::<u32>().write(v.to_bits());
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_MATRIX_REORTHOGONALISE, HOOK_MATRIX_FORCE_UP_VECTOR];
