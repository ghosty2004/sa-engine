//! RtQuat toolkit functions (batch `writero-rw-1`): `RtQuatConvertFromMatrix` with its three
//! `_rtQuatFromMatrix*` helpers, `RtQuatRotate`, `RtQuatTransformVectors` and `RtQuatSetupSlerpCache`, plus the phase A hook of
//! `RwMatrixOrthoNormalize`'s body 0x7F1930 (its entry 0x7F1920 is a `jmp` into it, registered as a
//! native in `matrix.rs`). Ported from the 1.0 US bytes (gta-reversed does not reverse RtQuat). Every
//! x87 value the original keeps on the FPU stack is a full-width [`X87`]; `fstp dword` is `to_f32`.
//! Verified by L2 (VERIFIER-10, docs/verify/verdicts-gen2b.tsv).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{RwError, g, rd, rw_error_set, rw_error1, rw_error2, wr, x};

/// 0.0f, 1.0f, 0.5f (`.rdata`); the RtQuatRotate angle scale (degrees to half radians) at 0x8631D4.
const C_ZERO: usize = 0x858B50;
const C_ONE: usize = 0x858624;
const C_HALF: usize = 0x858B8C;
const C_HALF_RAD: usize = 0x8631D4;
/// The RtQuat plugin id of its `RwError`s.
const QUAT_PLUGIN: i32 = 0x195;

orig! {
    /// `_rwSqrt(float)` (0x7EDB30): table sqrt, f32 in ST0
    fn rw_sqrt(v: f32) -> f64 = cdecl 0x7EDB30 => crate::vector::rw_sqrt_f64;
    /// `RwV3dNormalize(out, in)` (0x7ED9B0): ST0 length (discarded here)
    fn rw_v3d_normalize(out: GPtr<u8>, inp: GPtr<u8>) -> f64 = cdecl 0x7ED9B0 => normalize_f64;
    /// `_rtQuatFromMatrixX/Y/Z(RtQuat*, const RwMatrix*)` (called through a register in the original)
    fn from_matrix_x(q: GPtr<u8>, m: GPtr<u8>) = cdecl 0x7EB6A0 => rt_quat_from_matrix_x;
    fn from_matrix_y(q: GPtr<u8>, m: GPtr<u8>) = cdecl 0x7EB700 => rt_quat_from_matrix_y;
    fn from_matrix_z(q: GPtr<u8>, m: GPtr<u8>) = cdecl 0x7EB760 => rt_quat_from_matrix_z;
}

/// [`crate::vector::rw_v3d_normalize`] with the ST0 length as the f64 an `orig!` return carries.
pub fn normalize_f64(out: GPtr<u8>, inp: GPtr<u8>) -> f64 {
    crate::vector::rw_v3d_normalize(out.cast(), inp.cast()).to_f64()
}

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn f(p: GPtr<u8>, o: isize) -> f32 {
    rd::<f32>(p, o)
}

/// `RtQuatConvertFromMatrix(RtQuat* q, const RwMatrix* m)` (0x7EB5C0, cdecl): 0 for a null argument.
/// `t = (m.at.z + m.right.x) + m.up.y` (x87); `t > 0`: `r = _rwSqrt(f32(t + 1))`, w = `f32(0.5 * r)`,
/// `s = 0.5 / r` (full width), x = `(up.z - at.y) * s`, y = `(at.x - right.z) * s`, z = `(right.y -
/// up.x) * s`. Otherwise the largest diagonal term picks `_rtQuatFromMatrixX` (right.x > up.y and >
/// at.z), `Y` (up.y > at.z) or `Z` (ties and NaNs as the original's `and eax, 0x4100` tests). Returns 1.
#[hook(addr = 0x7EB5C0, conv = cdecl, name = "RtQuatConvertFromMatrix", expect = "53 56 57 8B 7C 24 10",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn rt_quat_convert_from_matrix(q: GPtr<u8>, m: GPtr<u8>) -> u32 {
    if q.is_null() || m.is_null() {
        return 0;
    }
    let zero = x(gf(C_ZERO));
    let t = x(f(m, 0x28)).fadd_m(f(m, 0)).fadd_m(f(m, 0x14));
    if !t.fcom(zero).test_any(0x41) {
        let r = X87::from_f64_exact(rw_sqrt(t.fadd_m(gf(C_ONE)).to_f32()));
        wr::<f32>(q, 0xC, x(gf(C_HALF)).fmul(r).to_f32());
        let s = r.fdivr_m(gf(C_HALF));
        wr::<f32>(q, 0, x(f(m, 0x18)).fsub_m(f(m, 0x24)).fmul(s).to_f32());
        wr::<f32>(q, 4, x(f(m, 0x20)).fsub_m(f(m, 8)).fmul(s).to_f32());
        wr::<f32>(q, 8, x(f(m, 4)).fsub_m(f(m, 0x10)).fmul(s).to_f32());
        return 1;
    }
    if !x(f(m, 0)).fcom(x(f(m, 0x14))).test_any(0x41) {
        if !x(f(m, 0)).fcom(x(f(m, 0x28))).test_any(0x41) {
            from_matrix_x(q, m);
        } else {
            from_matrix_z(q, m);
        }
    } else if !x(f(m, 0x14)).fcom(x(f(m, 0x28))).test_any(0x41) {
        from_matrix_y(q, m);
    } else {
        from_matrix_z(q, m);
    }
    1
}

/// The shared body of the three `_rtQuatFromMatrix*` helpers: `r = _rwSqrt(f32((d - (a + b)) + 1))`
/// (`fld a; fadd b; fsubr d; fadd 1.0`; the original stores that f32 into its own `m` argument slot,
/// which its only caller pops unread), the dominant component (`big`) = `f32(0.5 * r)`, `s = 0.5 / r`,
/// then each `(offset, p, q, minus)` = `f32((m[p] -/+ m[q]) * s)` in the original's store order.
fn from_matrix(q: GPtr<u8>, m: GPtr<u8>, a: isize, b: isize, d: isize, big: isize, rest: [(isize, isize, isize, bool); 3]) {
    let v = x(f(m, a)).fadd_m(f(m, b)).fsubr_m(f(m, d)).fadd_m(gf(C_ONE)).to_f32();
    let r = X87::from_f64_exact(rw_sqrt(v));
    wr::<f32>(q, big, x(gf(C_HALF)).fmul(r).to_f32());
    let s = r.fdivr_m(gf(C_HALF));
    for (o, p, n, minus) in rest {
        let t = if minus { x(f(m, p)).fsub_m(f(m, n)) } else { x(f(m, p)).fadd_m(f(m, n)) };
        wr::<f32>(q, o, t.fmul(s).to_f32());
    }
}

/// `_rtQuatFromMatrixX(RtQuat* q, const RwMatrix* m)` (0x7EB6A0, cdecl): `r = sqrt(f32((right.x -
/// (at.z + up.y)) + 1))`; x = `0.5 r`, w = `(up.z - at.y) s`, y = `(up.x + right.y) s`, z = `(at.x +
/// right.z) s` (`s = 0.5 / r`). See [`from_matrix`].
#[hook(addr = 0x7EB6A0, conv = cdecl, name = "_rtQuatFromMatrixX", expect = "56 8B 74 24 0C",
       route = "L2", hazards = "none", recs = "grove-walk",
       note = "stores an f32 into its own m argument slot (entry+8); the caller pops it unread")]
pub fn rt_quat_from_matrix_x(q: GPtr<u8>, m: GPtr<u8>) {
    from_matrix(q, m, 0x28, 0x14, 0, 0, [(0xC, 0x18, 0x24, true), (4, 0x10, 4, false), (8, 0x20, 8, false)]);
}

/// `_rtQuatFromMatrixY(RtQuat* q, const RwMatrix* m)` (0x7EB700, cdecl): `r = sqrt(f32((up.y - (at.z
/// + right.x)) + 1))`; y = `0.5 r`, w = `(at.x - right.z) s`, z = `(at.y + up.z) s`, x = `(up.x +
/// right.y) s`.
#[hook(addr = 0x7EB700, conv = cdecl, name = "_rtQuatFromMatrixY", expect = "56 8B 74 24 0C",
       route = "L2", hazards = "none", recs = "grove-walk",
       note = "stores an f32 into its own m argument slot (entry+8); the caller pops it unread")]
pub fn rt_quat_from_matrix_y(q: GPtr<u8>, m: GPtr<u8>) {
    from_matrix(q, m, 0x28, 0, 0x14, 4, [(0xC, 0x20, 8, true), (8, 0x24, 0x18, false), (0, 0x10, 4, false)]);
}

/// `_rtQuatFromMatrixZ(RtQuat* q, const RwMatrix* m)` (0x7EB760, cdecl): `r = sqrt(f32((at.z - (up.y
/// + right.x)) + 1))`; z = `0.5 r`, w = `(right.y - up.x) s`, x = `(at.x + right.z) s`, y = `(at.y +
/// up.z) s`.
#[hook(addr = 0x7EB760, conv = cdecl, name = "_rtQuatFromMatrixZ", expect = "56 8B 74 24 0C",
       route = "L2", hazards = "none", recs = "grove-walk",
       note = "stores an f32 into its own m argument slot (entry+8); the caller pops it unread")]
pub fn rt_quat_from_matrix_z(q: GPtr<u8>, m: GPtr<u8>) {
    from_matrix(q, m, 0x14, 0, 0x28, 8, [(0xC, 4, 0x10, true), (0, 0x20, 8, false), (4, 0x24, 0x18, false)]);
}

/// `RtQuatRotate(RtQuat* q, const RwV3d* axis, float angle, RwOpCombineType op)` (0x7EB7C0, cdecl):
/// null q or axis: `RwErrorSet({0x195, _rwerror(0x80000016)})`, 0. `h = angle * [0x8631D4]` (full
/// width), `s = f32(fsin h)` (kept in the original's own `angle` slot), `c = fcos h`.
/// - op 0 (replace): w = `f32(c)`; `RwV3dNormalize(q.xyz, axis)`; x/y/z = `f32(s * q.x)` etc.
/// - op 1 (preconcat) / 2 (postconcat): on a copy of q and `V = normalize(axis)`, `A/B/C = s * V`
///   (full width), w = `w c - (B y + A x + C z)` (preconcat: `c w - ((B y + A x) + C z)`); the vector
///   part is `c q.v + w (A, B, C) ± (A, B, C) x q.v` with the operand order of each original sum.
/// - other ops: `RwErrorSet({0x195, _rwerror(0x80000003, 0x8E21FC)})`, 0.
///
/// Returns q.
#[hook(addr = 0x7EB7C0, conv = cdecl, name = "RtQuatRotate", expect = "83 EC 20 56 8B 74 24 28",
       route = "L2", hazards = "x87-transcendental", recs = "grove-walk",
       note = "fsin/fcos of the full-width half angle; stores sin as f32 into its own angle argument slot (entry+0xC)")]
pub fn rt_quat_rotate(q: GPtr<u8>, axis: GPtr<u8>, angle: f32, op: u32) -> u32 {
    if q.is_null() || axis.is_null() {
        let code = rw_error1(0x8000_0016);
        let mut e = RwError { plugin_id: QUAT_PLUGIN, error_code: code as i32 };
        rw_error_set(GPtr::from_raw(&mut e));
        return 0;
    }
    if op > 2 {
        let code = rw_error2(0x8000_0003, 0x8E21FC);
        let mut e = RwError { plugin_id: QUAT_PLUGIN, error_code: code as i32 };
        rw_error_set(GPtr::from_raw(&mut e));
        return 0;
    }
    let h = x(angle).fmul_m(gf(C_HALF_RAD));
    let s = h.fsin().to_f32();
    if op == 0 {
        wr::<f32>(q, 0xC, h.fcos().to_f32());
        rw_v3d_normalize(q, axis);
        for o in [0, 4, 8] {
            wr::<f32>(q, o, x(s).fmul_m(f(q, o)).to_f32());
        }
        return q.addr() as u32;
    }
    let (qx, qy, qz, qw) = (f(q, 0), f(q, 4), f(q, 8), f(q, 0xC));
    let c = h.fcos().to_f32();
    let mut v = [0f32; 3];
    let vp = GPtr::from_raw(v.as_mut_ptr().cast::<u8>());
    rw_v3d_normalize(vp, axis);
    let a = x(s).fmul_m(v[0]);
    let b = x(v[1]).fmul_m(s);
    let cc = x(v[2]).fmul_m(s);
    if op == 2 {
        // postconcat (0x7EB822)
        let d = x(qw).fmul_m(c);
        let sum = x(qy).fmul(b).fadd(x(qx).fmul(a)).fadd(x(qz).fmul(cc));
        wr::<f32>(q, 0xC, d.fsub(sum).to_f32());
        let vx = x(qz).fmul(b).fsub(cc.fmul_m(qy)).fadd(a.fmul_m(qw)).fadd(x(qx).fmul_m(c));
        wr::<f32>(q, 0, vx.to_f32());
        let vy = x(qx).fmul(cc).fsub(x(qz).fmul(a)).fadd(x(qy).fmul_m(c)).fadd(b.fmul_m(qw));
        wr::<f32>(q, 4, vy.to_f32());
        let vz = x(qy).fmul(a).fsub(b.fmul_m(qx)).fadd(cc.fmul_m(qw)).fadd(x(qz).fmul_m(c));
        wr::<f32>(q, 8, vz.to_f32());
    } else {
        // preconcat (0x7EB911)
        let d = x(c).fmul_m(qw);
        let sum = b.fmul_m(qy).fadd(a.fmul_m(qx)).fadd(cc.fmul_m(qz));
        wr::<f32>(q, 0xC, d.fsub(sum).to_f32());
        let vx = x(qy).fmul(cc).fsub(x(qz).fmul(b)).fadd(x(qx).fmul_m(c)).fadd(a.fmul_m(qw));
        wr::<f32>(q, 0, vx.to_f32());
        let vy = x(qz).fmul(a).fsub(cc.fmul_m(qx)).fadd(x(qy).fmul_m(c)).fadd(b.fmul_m(qw));
        wr::<f32>(q, 4, vy.to_f32());
        let vz = b.fmul_m(qx).fsub(x(qy).fmul(a)).fadd(x(qz).fmul_m(c)).fadd(cc.fmul_m(qw));
        wr::<f32>(q, 8, vz.to_f32());
    }
    q.addr() as u32
}

/// `RtQuatTransformVectors(RwV3d* out, const RwV3d* in, int32 n, const RtQuat* q)` (0x7EBBB0, cdecl):
/// `k = f32(w*w - ((x*x + y*y) + z*z))` (stored into the original's own `q` argument slot), the f32
/// locals `2x, 2y, 2z` and `w*2x, w*2y, w*2z`. Per vector v: `d = (v.y*2y + 2x*v.x) + v.z*2z` (full
/// width); out = `(v.z*w2y - v.y*w2z, w2z*v.x - v.z*w2x, v.y*w2x - w2y*v.x)` (f32), then out += `q.v * d`,
/// then out += `v * k`, where v is re-read from `in` after each store (in-place calls see the new
/// values, as in the original). Returns out.
#[hook(addr = 0x7EBBB0, conv = cdecl, name = "RtQuatTransformVectors", expect = "83 EC 24 53 56",
       route = "L2", hazards = "none", recs = "grove-walk",
       note = "stores k as f32 into its own q argument slot (entry+0x10); callers pop it unread")]
pub fn rt_quat_transform_vectors(out: GPtr<u8>, inp: GPtr<u8>, n: i32, q: GPtr<u8>) -> GPtr<u8> {
    let (qx, qy, qz, qw) = (f(q, 0), f(q, 4), f(q, 8), f(q, 0xC));
    let (wx, wy, wz, ww) = (x(qx), x(qy), x(qz), x(qw));
    let k = ww.fmul(ww).fsub(wx.fmul(wx).fadd(wy.fmul(wy)).fadd(wz.fmul(wz))).to_f32();
    let tx = wx.fadd(wx).to_f32();
    let ty = wy.fadd(wy).to_f32();
    let tz = wz.fadd(wz).to_f32();
    let w2x = x(qw).fmul_m(tx).to_f32();
    let w2y = x(qw).fmul_m(ty).to_f32();
    let w2z = x(qw).fmul_m(tz).to_f32();
    for i in 0..n.max(0) as isize {
        let v = inp.byte_add(12 * i);
        let o = out.byte_add(12 * i);
        let d = x(f(v, 4)).fmul_m(ty).fadd(x(tx).fmul_m(f(v, 0))).fadd(x(f(v, 8)).fmul_m(tz));
        let c1 = x(f(v, 8)).fmul_m(w2y).fsub(x(f(v, 4)).fmul_m(w2z)).to_f32();
        let c2 = x(w2z).fmul_m(f(v, 0)).fsub(x(f(v, 8)).fmul_m(w2x)).to_f32();
        let c3 = x(f(v, 4)).fmul_m(w2x).fsub(x(w2y).fmul_m(f(v, 0))).to_f32();
        wr::<f32>(o, 0, c1);
        wr::<f32>(o, 4, c2);
        wr::<f32>(o, 8, c3);
        wr::<f32>(o, 0, d.fmul_m(qx).fadd_m(f(o, 0)).to_f32());
        wr::<f32>(o, 4, x(qy).fmul(d).fadd_m(f(o, 4)).to_f32());
        wr::<f32>(o, 8, x(qz).fmul(d).fadd_m(f(o, 8)).to_f32());
        wr::<f32>(o, 0, x(k).fmul_m(f(v, 0)).fadd_m(f(o, 0)).to_f32());
        wr::<f32>(o, 4, x(f(v, 4)).fmul_m(k).fadd_m(f(o, 4)).to_f32());
        wr::<f32>(o, 8, x(f(v, 8)).fmul_m(k).fadd_m(f(o, 8)).to_f32());
    }
    out
}

/// `RwMatrixOrthoNormalize`'s body (0x7F1930, cdecl `(dst, src)`, returns dst): the same function as
/// the native at 0x7F1920 ([`crate::matrix::matrix_ortho_normalize`]), hooked at the body so phase A
/// reaches the port (the 0x7F1920 entry is a `jmp` into game text and cannot be patched).
#[hook(addr = 0x7F1930, conv = cdecl, name = "FUN_007f1930", expect = "83 EC 3C 8B 44 24 44",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn matrix_ortho_normalize_body(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    crate::matrix::matrix_ortho_normalize(dst, src)
}


/// A `.rdata` float by address (the polynomial coefficients of the inlined `acosf` / `sinf`).
fn c(a: usize) -> f32 {
    gf(a)
}

/// The inlined `acosf` rational term: `p(z) / q(z)` with `p = (((((c90 z + c8C) z - c88) z + c84) z -
/// c80) z + 1/6) z` and `q = (((c7C z - c78) z + c74) z - c70) z + 1` (constants at 0x874E70..0x874E90,
/// 0x85F0A0), `z` either a full-width value (`zw`) or an f32 memory operand (`zf`), as the original.
fn acos_ratio(z: Z) -> X87 {
    let mul = |v: X87| match z {
        Z::Wide(w) => v.fmul(w),
        Z::Mem(m) => v.fmul_m(m),
    };
    let first = match z {
        Z::Wide(w) => x(c(0x874E90)).fmul(w),
        Z::Mem(m) => x(m).fmul_m(c(0x874E90)),
    };
    let p = mul(mul(mul(mul(mul(first.fadd_m(c(0x874E8C))).fsub_m(c(0x874E88))).fadd_m(c(0x874E84))).fsub_m(c(0x874E80))).fadd_m(c(0x85F0A0)));
    let qfirst = match z {
        Z::Wide(w) => w.fmul_m(c(0x874E7C)),
        Z::Mem(m) => x(m).fmul_m(c(0x874E7C)),
    };
    let q = mul(mul(mul(qfirst.fsub_m(c(0x874E78))).fadd_m(c(0x874E74))).fsub_m(c(0x874E70))).fadd_m(gf(C_ONE));
    p.fdiv(q)
}

#[derive(Clone, Copy)]
enum Z {
    Wide(X87),
    Mem(f32),
}

/// `RtQuatSetupSlerpCache(const RtQuat* from, const RtQuat* to, RtQuatSlerpCache* cache)` (0x7EC220,
/// cdecl). cache: +0 from, +0x10 to, +0x20 omega, +0x24 nearly-zero flag.
///
/// cache.from = *from; `dot = ((fy ty + fx tx) + fz tz) + fw tw` (full width). Full-width dot < 0:
/// cache.to = -*to and `d = -f32(dot)` (1.0 when `f32(dot) < -1`); else cache.to = *to and `d =
/// f32(dot)` (1.0 when above 1). `omega = acos(d)` by the inlined fdlibm `acosf` on the bits of d
/// (|d| ≥ 1: 0 for d > 0 as an int, else pi; |d| < 0.5: pi/2 below 2^-57, else `pio2_hi - (d - (pio2_lo -
/// d r))`; d < 0: `pi - 2 (s + (r s - pio2_lo))` with `s = _rwSqrt(f32((d + 1) / 2))`; else `2 (df + (r s
/// + (z - df²) / (df + s)))` with df = s's bits `& 0xFFFFF000`). The flag = `d >= [0x871258]`; without
/// it both quaternions are scaled by `1 / sin(omega)` (a 6-term polynomial). Returns the flag. The
/// original keeps d, z and df in its own argument slots (entry+0xC, +4, +8).
#[hook(addr = 0x7EC220, conv = cdecl, name = "RtQuatSetupSlerpCache", expect = "8B 44 24 04 56",
       route = "L2", hazards = "none", recs = "grove-walk",
       note = "stores f32 scratch into its own argument slots (entry+4, +8, +0xC); callers pop them unread")]
pub fn rt_quat_setup_slerp_cache(from: GPtr<u8>, to: GPtr<u8>, cache: GPtr<u8>) -> u32 {
    for o in [0, 4, 8, 0xC] {
        wr::<u32>(cache, o, rd::<u32>(from, o));
    }
    let dot = x(f(from, 4)).fmul_m(f(to, 4)).fadd(x(f(from, 0)).fmul_m(f(to, 0))).fadd(x(f(from, 8)).fmul_m(f(to, 8))).fadd(x(f(from, 0xC)).fmul_m(f(to, 0xC)));
    let dotf = dot.to_f32();
    let mut d = dotf;
    if !dot.fcom(x(gf(C_ZERO))).test_parity(5) {
        if x(dotf).fcom(x(c(0x858C1C))).test_parity(5) {
            d = x(dotf).fchs().to_f32();
        } else {
            d = f32::from_bits(0x3F80_0000);
        }
        for (o, s) in [(0x1C, 0xC), (0x10, 0), (0x14, 4), (0x18, 8)] {
            wr::<f32>(cache, o, x(f(to, s)).fchs().to_f32());
        }
    } else {
        if !x(dotf).fcom(x(gf(C_ONE))).test_any(0x41) {
            d = f32::from_bits(0x3F80_0000);
        }
        for o in [0, 4, 8, 0xC] {
            wr::<u32>(cache, 0x10 + o, rd::<u32>(to, o));
        }
    }
    let bits = d.to_bits() as i32;
    let ab = bits & 0x7FFF_FFFF;
    let omega: u32 = if ab >= 0x3F80_0000 {
        if bits > 0 { 0 } else { 0x4049_0FDB }
    } else if ab < 0x3F00_0000 {
        if ab <= 0x2300_0000 {
            0x3FC9_0FDB
        } else {
            let z = x(d).fmul_m(d);
            let r = acos_ratio(Z::Wide(z));
            r.fmul_m(d).fsubr_m(c(0x874E6C)).fsubr_m(d).fsubr_m(c(0x874E68)).to_f32().to_bits()
        }
    } else if bits < 0 {
        let zf = x(d).fadd_m(gf(C_ONE)).fmul_m(gf(C_HALF)).to_f32();
        let s = X87::from_f64_exact(rw_sqrt(zf));
        let r = acos_ratio(Z::Mem(zf));
        let w = r.fmul(s).fsub_m(c(0x874E6C)).fadd(s);
        w.fadd(w).fsubr_m(c(0x874E64)).to_f32().to_bits()
    } else {
        let zf = x(gf(C_ONE)).fsub_m(d).fmul_m(gf(C_HALF)).to_f32();
        let s = X87::from_f64_exact(rw_sqrt(zf));
        let df = f32::from_bits(s.to_f32().to_bits() & 0xFFFF_F000);
        let r = acos_ratio(Z::Mem(zf));
        let rs = r.fmul(s);
        let cc = x(df).fmul_m(df).fsubr_m(zf).fdiv(x(df).fadd(s));
        let w = rs.fadd(cc).fadd_m(df);
        w.fadd(w).to_f32().to_bits()
    };
    wr::<u32>(cache, 0x20, omega);
    let near = u32::from(!x(d).fcom(x(c(0x871258))).c0());
    wr::<u32>(cache, 0x24, near);
    if near != 0 {
        return near;
    }
    let om = x(f(cache, 0x20));
    let z2 = om.fmul(om);
    let p = x(c(0x86D2B0)).fmul(z2).fsub_m(c(0x86D2AC)).fmul(z2).fadd_m(c(0x86D2A8)).fmul(z2).fsub_m(c(0x86D2A4)).fmul(z2).fadd_m(c(0x864E0C)).fmul(z2).fsub_m(c(0x85F0A0));
    let sin = p.fmul(z2.fmul(om)).fadd(om);
    let inv = sin.fdivr_m(gf(C_ONE));
    wr::<f32>(cache, 0xC, inv.fmul_m(f(cache, 0xC)).to_f32());
    wr::<f32>(cache, 0, inv.fmul_m(f(cache, 0)).to_f32());
    for o in [4, 8] {
        wr::<f32>(cache, o, x(f(cache, o)).fmul(inv).to_f32());
    }
    wr::<f32>(cache, 0x1C, inv.fmul_m(f(cache, 0x1C)).to_f32());
    for o in [0x10, 0x14, 0x18] {
        wr::<f32>(cache, o, x(f(cache, o)).fmul(inv).to_f32());
    }
    0
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RT_QUAT_SETUP_SLERP_CACHE,
    HOOK_RT_QUAT_CONVERT_FROM_MATRIX,
    HOOK_RT_QUAT_FROM_MATRIX_X,
    HOOK_RT_QUAT_FROM_MATRIX_Y,
    HOOK_RT_QUAT_FROM_MATRIX_Z,
    HOOK_RT_QUAT_ROTATE,
    HOOK_RT_QUAT_TRANSFORM_VECTORS,
    HOOK_MATRIX_ORTHO_NORMALIZE_BODY,
];
