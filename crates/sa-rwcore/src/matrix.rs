//! MATRIXMODULE (engine plugin 0x402) and the `RwMatrix` / `RwBBox` functions, plus the SSE2 vector
//! transform kernels (PORT-RW-1). Ported from the 1.0 US bytes; `docs/re/specs/RW/RwMatrix.md` is the
//! reading aid.
//!
//! Module `M = G + [0xC979BC]`: +0 matrix freelist, +4 identity-flag mask (0x20000), +8 multiply
//! function `(dst, a, b)`, +0xC `RwMatrixTolerance` {normal, orthogonal, identity}.
//!
//! Matrices are accessed by byte offset (right +0, flags +0xC, up +0x10, at +0x20, pos +0x30) and every
//! read happens where the original reads, so aliasing arguments behave as in the original. Every x87
//! sequence follows the original's grouping (multiplication and addition are commutative bit for bit,
//! including NaN selection, so only the grouping and the stored-vs-unstored distinction matter).
//!
//! Uninitialised stack copied by the originals (pads of the local matrices of
//! `RwMatrixRotateOneMinusCosineSine` and of the concatenation temporaries of it and of
//! `RwMatrixTransform`, which the multiply kernel never writes) is written as 0 here.

use sa_core::abi::HookReg;
use sa_core::light::RwV3d;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{error2, f32_of, g, module, p8, rd, rw_memory_alloc, rw_memory_free, wr, x};
use crate::layout::modules::MATRIX;

/// `1.0f` at 0x858624.
const ONE_F: f32 = 1.0;
/// `0.017453292f` at 0x8595EC (degrees to radians).
const DEG_TO_RAD_BITS: u32 = 0x3C8E_FA35;
/// `_rwMatrixMultiplyDefault` (0x7F12F0).
pub const DEFAULT_MULTIPLY: u32 = 0x7F12F0;
/// "Invalid combination type" (0x8E21FC), the E_RW_BADPARAM argument.
const BAD_COMBINE_MSG: u32 = 0x8E21FC;
/// E_RW_BADPARAM
const E_RW_BADPARAM: u32 = 0x8000_0003;
/// MATRIXMODULE instance counter.
const MATRIX_INSTANCES: usize = 0xC979C0;

orig! {
    /// `M->multiply(dst, a, b)` (stored pointer)
    pub fn call_multiply(f: u32, dst: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = cdecl dyn => unported;
    /// `_rwMatrixNormalError(m)` (0x7F1500)
    pub fn normal_error_call(m: GPtr<u8>) -> f64 = cdecl 0x7F1500 => normal_error_f64;
    /// `_rwMatrixOrthogonalError(m)` (0x7F14A0)
    pub fn orthogonal_error_call(m: GPtr<u8>) -> f64 = cdecl 0x7F14A0 => orthogonal_error_f64;
    /// `_rwMatrixIdentityError(m)` (0x7F1590, not ported)
    pub fn identity_error_call(m: GPtr<u8>) -> f64 = cdecl 0x7F1590 => unported;
    /// `_rwInvSqrt(x)` (0x7EDB90)
    pub fn inv_sqrt_call(v: f32) -> f64 = cdecl 0x7EDB90 => inv_sqrt_f64;
    /// `RwMatrixRotateOneMinusCosineSine(m, axis, omc, sin, op)` (0x7F1D00)
    pub fn rotate_omcs_call(m: GPtr<u8>, axis: GPtr<RwV3d>, omc: f32, sin: f32, op: i32) -> GPtr<u8> = cdecl 0x7F1D00 => rw_matrix_rotate_one_minus_cosine_sine;
    /// `_rwMatrixInvertOrthoNormalizedOrGeneral(dst, src)` (0x7F2160)
    pub fn invert_general_call(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F2160 => rw_matrix_invert_general;
    /// `RwFreeListCreateAndPreallocateSpace(entrySize, entriesPerBlock, alignment, numBlocks, space, hint)` (0x801B70)
    pub fn freelist_create_prealloc(size: u32, per_block: u32, align: u32, n: u32, space: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x801B70 => unported;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn normal_error_f64(m: GPtr<u8>) -> f64 {
    rw_matrix_normal_error(m).to_f64()
}
#[cfg_attr(sa_phase_a, allow(dead_code))]
fn orthogonal_error_f64(m: GPtr<u8>) -> f64 {
    rw_matrix_orthogonal_error(m).to_f64()
}
#[cfg_attr(sa_phase_a, allow(dead_code))]
fn inv_sqrt_f64(v: f32) -> f64 {
    crate::vector::rw_inv_sqrt(v).to_f64()
}

fn mmod() -> GPtr<u8> {
    module(MATRIX)
}

/// `fld dword [m + off]`.
#[inline]
fn ld(m: GPtr<u8>, off: isize) -> X87 {
    x(rd::<f32>(m, off))
}

/// `fstp dword [m + off]`.
#[inline]
fn st(m: GPtr<u8>, off: isize, v: X87) {
    wr::<f32>(m, off, f32_of(v));
}

#[inline]
fn fm(m: GPtr<u8>, off: isize) -> f32 {
    rd::<f32>(m, off)
}

/// 16 dwords from `src` to `dst` (`rep movsd`).
fn copy16(dst: GPtr<u8>, src: GPtr<u8>) {
    for i in 0..16 {
        wr::<u32>(dst, i * 4, rd::<u32>(src, i * 4));
    }
}

/// `M+4 & 0x20000`.
fn identity_mask() -> u32 {
    identity_mask_in(mmod())
}
fn identity_mask_in(mm: GPtr<u8>) -> u32 {
    rd::<u32>(mm, 4) & 0x20000
}

/// `_rwMatrixSetMultFn(fn)` (0x7F12C0): `M+8 = fn ? fn : _rwMatrixMultiplyDefault`; returns 1.
#[hook(addr = 0x7F12C0, conv = cdecl, name = "_rwMatrixSetMultFn", expect = "8B 44 24 04 8B 15 BC 79 C9 00")]
pub fn rw_matrix_set_mult_fn(f: u32) -> u32 {
    wr::<u32>(mmod(), 8, if f == 0 { DEFAULT_MULTIPLY } else { f });
    1
}

/// `_rwMatrixDeterminant(m)` (0x7F1450), unstored x87 result.
#[hook(addr = 0x7F1450, conv = cdecl, name = "_rwMatrixDeterminant", expect = "8B 44 24 04 D9 40 28")]
pub fn rw_matrix_determinant(m: GPtr<u8>) -> X87 {
    let c0 = ld(m, 0x28)
        .fmul_m(fm(m, 0x14))
        .fsub(ld(m, 0x24).fmul_m(fm(m, 0x18)));
    let c1 = ld(m, 0x18)
        .fmul_m(fm(m, 0x20))
        .fsub(ld(m, 0x28).fmul_m(fm(m, 0x10)));
    let c2 = ld(m, 0x24)
        .fmul_m(fm(m, 0x10))
        .fsub(ld(m, 0x14).fmul_m(fm(m, 0x20)));
    let a = ld(m, 0x4).fmul(c1);
    let b = c0.fmul_m(fm(m, 0));
    let s = b.fadd(a);
    s.fadd(ld(m, 0x8).fmul(c2))
}

/// `(p*q + r*s) + t*u` with memory operands at the given offsets (the error sums).
fn dot3(m: GPtr<u8>, o: [(isize, isize); 3]) -> X87 {
    let a = ld(m, o[0].0).fmul_m(fm(m, o[0].1));
    let b = ld(m, o[1].0).fmul_m(fm(m, o[1].1));
    let c = ld(m, o[2].0).fmul_m(fm(m, o[2].1));
    a.fadd(b).fadd(c)
}

/// `_rwMatrixOrthogonalError(m)` (0x7F14A0), unstored x87 result.
#[hook(addr = 0x7F14A0, conv = cdecl, name = "_rwMatrixOrthogonalError", expect = "8B 44 24 04 D9 40 24")]
pub fn rw_matrix_orthogonal_error(m: GPtr<u8>) -> X87 {
    let dua = dot3(m, [(0x24, 0x14), (0x20, 0x10), (0x28, 0x18)]);
    let dar = dot3(m, [(0x24, 0x4), (0x20, 0x0), (0x28, 0x8)]);
    let dur = dot3(m, [(0x14, 0x4), (0x10, 0x0), (0x18, 0x8)]);
    dar.fmul(dar).fadd(dua.fmul(dua)).fadd(dur.fmul(dur))
}

/// `((a*a + b*b) + c*c) - 1` of a row loaded as (y, x, z).
fn row_norm_err(m: GPtr<u8>, row: isize) -> X87 {
    let (vy, vx, vz) = (ld(m, row + 4), ld(m, row), ld(m, row + 8));
    vy.fmul(vy).fadd(vx.fmul(vx)).fadd(vz.fmul(vz)).fsub_m(ONE_F)
}

/// `_rwMatrixNormalError(m)` (0x7F1500), unstored x87 result.
#[hook(addr = 0x7F1500, conv = cdecl, name = "_rwMatrixNormalError", expect = "8B 44 24 04 D9 40 04")]
pub fn rw_matrix_normal_error(m: GPtr<u8>) -> X87 {
    let er = row_norm_err(m, 0);
    let eu = row_norm_err(m, 0x10);
    let ea = row_norm_err(m, 0x20);
    eu.fmul(eu).fadd(er.fmul(er)).fadd(ea.fmul(ea))
}

/// `RwMatrixOptimize(m, tol)` (0x7F17E0): sets/clears bits 1 (normal), 2 (orthogonal), 0x20000
/// (identity, evaluated only when both hold) from the three error functions; `tol` NULL = `M+0xC`.
#[hook(addr = 0x7F17E0, conv = cdecl, name = "RwMatrixOptimize", expect = "53 55 56 57 8B 7C 24 18")]
pub fn rw_matrix_optimize(m: GPtr<u8>, tol: GPtr<f32>) -> GPtr<u8> {
    let tol: GPtr<u8> = if tol.is_null() {
        mmod().byte_add(0xC)
    } else {
        tol.cast()
    };
    let within = |e: f64, t: f32| !X87::from_f64_exact(e).fcom(x(t)).test_parity(0x41);
    let normal = within(normal_error_call(m), rd::<f32>(tol, 0));
    let orth = within(orthogonal_error_call(m), rd::<f32>(tol, 4));
    let ident = normal && orth && within(identity_error_call(m), rd::<f32>(tol, 8));
    let mut f = rd::<u32>(m, 0xC);
    f = if normal { f | 1 } else { f & !1 };
    f = if orth { f | 2 } else { f & !2 };
    f = if ident { f | 0x20000 } else { f & !0x20000 };
    wr::<u32>(m, 0xC, f);
    m
}

/// `RwMatrixUpdate(m)` (0x7F18A0): `flags &= 0xFFFDFFFC`.
#[hook(addr = 0x7F18A0, conv = cdecl, name = "RwMatrixUpdate", expect = "8B 44 24 04 81 60 0C FC FF FD FF")]
pub fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(m, 0xC, rd::<u32>(m, 0xC) & 0xFFFD_FFFC);
    m
}

/// `RwMatrixMultiply(dst, a, b)` (0x7F18B0): identity shortcuts on `a` then `b`, else `M+8(dst, a, b)`
/// and `dst.flags = a.flags & b.flags` (both read before the call).
#[hook(addr = 0x7F18B0, conv = cdecl, name = "RwMatrixMultiply", expect = "8B 0D BC 79 C9 00")]
pub fn rw_matrix_multiply(dst: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> {
    let mask = identity_mask();
    let (fa, fb) = (rd::<u32>(a, 0xC), rd::<u32>(b, 0xC));
    if fa & mask != 0 {
        copy16(dst, b);
    } else if fb & mask != 0 {
        copy16(dst, a);
    } else {
        call_multiply(rd::<u32>(mmod(), 8), dst, a, b);
        wr::<u32>(dst, 0xC, fb & fa);
    }
    dst
}

/// `RwMatrixRotateOneMinusCosineSine(m, axis, omc, sin, op)` (0x7F1D00).
#[hook(addr = 0x7F1D00, conv = cdecl, name = "RwMatrixRotateOneMinusCosineSine", expect = "81 EC A4 00 00 00")]
pub fn rw_matrix_rotate_one_minus_cosine_sine(
    m: GPtr<u8>,
    axis: GPtr<RwV3d>,
    omc: f32,
    sin: f32,
    op: i32,
) -> GPtr<u8> {
    let mut local = [0u32; 16];
    build_rotation(axis, omc, sin, &mut local);
    let lp: GPtr<u8> = GPtr::from_raw(local.as_mut_ptr().cast());
    match op {
        0 => {
            copy16(m, lp);
            m
        }
        1 | 2 => {
            let mask = identity_mask();
            let mf = rd::<u32>(m, 0xC);
            let mut tmp = [0u32; 16];
            let tp: GPtr<u8> = GPtr::from_raw(tmp.as_mut_ptr().cast());
            if mf & mask != 0 {
                copy16(tp, lp);
            } else {
                let f = rd::<u32>(mmod(), 8);
                if op == 1 {
                    call_multiply(f, tp, lp, m);
                } else {
                    call_multiply(f, tp, m, lp);
                }
                wr::<u32>(tp, 0xC, mf & 3);
            }
            copy16(m, tp);
            m
        }
        _ => {
            error2(E_RW_BADPARAM, BAD_COMBINE_MSG);
            GPtr::from_addr(0)
        }
    }
}

/// The rotation matrix of `RwMatrixRotateOneMinusCosineSine` (pads 0, see the module docs).
pub fn build_rotation(axis: GPtr<RwV3d>, omc: f32, sin: f32, out: &mut [u32; 16]) {
    let a: GPtr<u8> = axis.cast();
    let (ax, ay, az) = (ld(a, 0), ld(a, 4), ld(a, 8));
    let ca = ax.fmul(ax).fsubr_m(ONE_F);
    let cb = ay.fmul(ay).fsubr_m(ONE_F);
    let cc = f32_of(az.fmul(az).fsubr_m(ONE_F));
    let tx = f32_of(ca.fmul_m(omc));
    let ty = cb.fmul_m(omc);
    let tz = x(cc).fmul_m(omc);
    let ayz = ld(a, 4).fmul_m(fm(a, 8));
    let azx = ld(a, 8).fmul_m(fm(a, 0));
    let axy = f32_of(ld(a, 4).fmul_m(fm(a, 0)));
    let pyz = f32_of(ayz.fmul_m(omc));
    let pzx = azx.fmul_m(omc);
    let pxy = x(axy).fmul_m(omc);
    let sx = f32_of(x(sin).fmul_m(fm(a, 0)));
    let sy = f32_of(ld(a, 4).fmul_m(sin));
    let sz = ld(a, 8).fmul_m(sin);
    let w = |v: X87| f32_of(v).to_bits();
    out[0] = w(x(ONE_F).fsub_m(tx));
    out[1] = w(sz.fadd(pxy));
    out[2] = w(pzx.fsub_m(sy));
    out[3] = 3;
    out[4] = w(pxy.fsub(sz));
    out[5] = w(x(ONE_F).fsub(ty));
    out[6] = w(x(sx).fadd_m(pyz));
    out[8] = w(x(sy).fadd(pzx));
    out[9] = w(x(pyz).fsub_m(sx));
    out[10] = w(x(ONE_F).fsub(tz));
    out[12] = 0;
    out[13] = 0;
    out[14] = 0;
}

/// `RwMatrixRotate(m, axis, angle, op)` (0x7F1FD0): `rad = f32(angle * 0.017453292f)`, unit axis by
/// `_rwInvSqrt`, `RwMatrixRotateOneMinusCosineSine(m, &u, f32(1 - fcos(rad)), f32(fsin(rad)), op)`;
/// returns m. (The original stores `rad` over its angle argument slot: not reproduced.)
#[hook(addr = 0x7F1FD0, conv = cdecl, name = "RwMatrixRotate", expect = "83 EC 0C D9 44 24 18")]
pub fn rw_matrix_rotate(m: GPtr<u8>, axis: GPtr<RwV3d>, angle: f32, op: i32) -> GPtr<u8> {
    rotate_impl(|s| X87::from_f64_exact(inv_sqrt_call(s)), m, axis, angle, op)
}

/// [`rw_matrix_rotate`] with the inverse sqrt table given (host tests).
pub fn rw_matrix_rotate_with_tables(
    u: GPtr<u32>,
    m: GPtr<u8>,
    axis: GPtr<RwV3d>,
    angle: f32,
    op: i32,
) -> GPtr<u8> {
    rotate_impl(|s| crate::vector::rw_inv_sqrt_in(u, s), m, axis, angle, op)
}

fn rotate_impl(
    inv_sqrt: impl Fn(f32) -> X87,
    m: GPtr<u8>,
    axis: GPtr<RwV3d>,
    angle: f32,
    op: i32,
) -> GPtr<u8> {
    let rad = f32_of(x(angle).fmul_m(f32::from_bits(DEG_TO_RAD_BITS)));
    let a: GPtr<u8> = axis.cast();
    let (vy, vx, vz) = (ld(a, 4), ld(a, 0), ld(a, 8));
    let s = f32_of(vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)));
    let inv = inv_sqrt(s);
    let mut u = RwV3d {
        x: f32_of(inv.fmul_m(fm(a, 0))),
        y: 0.0,
        z: 0.0,
    };
    u.y = f32_of(ld(a, 4).fmul(inv));
    u.z = f32_of(ld(a, 8).fmul(inv));
    let sn = f32_of(x(rad).fsin());
    let omc = f32_of(x(rad).fcos().fsubr_m(ONE_F));
    rotate_omcs_call(m, GPtr::from_raw(&mut u), omc, sn, op);
    m
}

/// `RwMatrixInvert(dst, src)` (0x7F2070): identity copy, orthonormal transpose, or the general inverse.
#[hook(addr = 0x7F2070, conv = cdecl, name = "RwMatrixInvert", expect = "8B 0D 24 7B C9 00")]
pub fn rw_matrix_invert(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    let mm = rd::<u32>(mmod(), 4);
    rw_matrix_invert_with(mm, dst, src)
}

/// [`rw_matrix_invert`] with `M+4` given.
pub fn rw_matrix_invert_with(mask_word: u32, dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    let sf = rd::<u32>(src, 0xC);
    if mask_word & sf & 0x20000 != 0 {
        copy16(dst, src);
        return dst;
    }
    if sf & 3 != 3 {
        invert_general_call(dst, src);
        return dst;
    }
    let (s, d) = (src, dst);
    for (to, from) in [
        (0x0, 0x0),
        (0x4, 0x10),
        (0x8, 0x20),
        (0x10, 0x4),
        (0x14, 0x14),
        (0x18, 0x24),
        (0x20, 0x8),
        (0x24, 0x18),
        (0x28, 0x28),
    ] {
        wr::<u32>(d, to, rd::<u32>(s, from));
    }
    let px = ld(s, 0x34)
        .fmul_m(fm(s, 0x4))
        .fadd(ld(s, 0x30).fmul_m(fm(s, 0)))
        .fadd(ld(s, 0x8).fmul_m(fm(s, 0x38)));
    st(d, 0x30, px.fchs());
    let py = ld(s, 0x30)
        .fmul_m(fm(s, 0x10))
        .fadd(ld(s, 0x34).fmul_m(fm(s, 0x14)))
        .fadd(ld(s, 0x38).fmul_m(fm(s, 0x18)));
    st(d, 0x34, py.fchs());
    let pz = ld(s, 0x20)
        .fmul_m(fm(s, 0x30))
        .fadd(ld(s, 0x34).fmul_m(fm(s, 0x24)))
        .fadd(ld(s, 0x28).fmul_m(fm(s, 0x38)));
    wr::<u32>(d, 0xC, 3);
    st(d, 0x38, pz.fchs());
    dst
}

/// `_rwMatrixInvertOrthoNormalizedOrGeneral(dst, src)` (0x7F2160): the general 3x4 inverse by
/// cofactors; `det == +0` (bit pattern 0) keeps `inv = 1`. `dst.flags = 0`. Returns dst (EAX).
/// (The original stores `f32(det)` and later `ry` over its `src` argument slot: not reproduced.)
#[hook(addr = 0x7F2160, conv = cdecl, name = "FUN_007f2160", expect = "8B 4C 24 08 8B 44 24 04")]
pub fn rw_matrix_invert_general(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    let (s, d) = (src, dst);
    let cof =
        |a: isize, b: isize, c: isize, e: isize| ld(s, a).fmul_m(fm(s, b)).fsub(ld(s, c).fmul_m(fm(s, e)));
    st(d, 0, cof(0x28, 0x14, 0x24, 0x18));
    st(d, 4, cof(0x28, 0x4, 0x24, 0x8).fchs());
    let c = cof(0x18, 0x4, 0x14, 0x8);
    st(d, 8, c);
    let det = ld(s, 0x10)
        .fmul_m(fm(d, 4))
        .fadd(ld(s, 0x20).fmul(c))
        .fadd(ld(d, 0).fmul_m(fm(s, 0)));
    let detf = f32_of(det);
    let mut inv = x(ONE_F);
    if detf.to_bits() != 0 {
        inv = inv.fdiv_m(detf);
    }
    let rx = inv.fmul_m(fm(d, 0));
    st(d, 0, rx);
    let ry = f32_of(inv.fmul_m(fm(d, 4)));
    wr::<f32>(d, 4, ry);
    st(d, 8, inv.fmul_m(fm(d, 8)));
    st(d, 0x10, cof(0x28, 0x10, 0x20, 0x18).fmul(inv).fchs());
    st(d, 0x14, cof(0x28, 0x0, 0x20, 0x8).fmul(inv));
    st(d, 0x18, cof(0x18, 0x0, 0x10, 0x8).fmul(inv).fchs());
    st(d, 0x20, cof(0x24, 0x10, 0x20, 0x14).fmul(inv));
    st(d, 0x24, cof(0x24, 0x0, 0x20, 0x4).fmul(inv).fchs());
    st(d, 0x28, cof(0x14, 0x0, 0x10, 0x4).fmul(inv));
    let px = ld(d, 0x20)
        .fmul_m(fm(s, 0x38))
        .fadd(ld(s, 0x34).fmul_m(fm(d, 0x10)))
        .fadd(ld(s, 0x30).fmul(rx));
    st(d, 0x30, px.fchs());
    let py = ld(d, 0x24)
        .fmul_m(fm(s, 0x38))
        .fadd(ld(s, 0x34).fmul_m(fm(d, 0x14)))
        .fadd(ld(s, 0x30).fmul_m(ry));
    st(d, 0x34, py.fchs());
    let pz0 = ld(s, 0x34)
        .fmul_m(fm(d, 0x18))
        .fadd(ld(s, 0x30).fmul_m(fm(d, 0x8)));
    let pz1 = ld(d, 0x28).fmul_m(fm(s, 0x38));
    wr::<u32>(d, 0xC, 0);
    st(d, 0x38, pz0.fadd(pz1).fchs());
    dst
}

/// The identity with `flags |= 0x20003` as REPLACE of Scale/Translate writes it (store order of the
/// original: diagonal, flags, then the zeros).
fn set_identity_or_flags(m: GPtr<u8>) {
    let one = ONE_F.to_bits();
    wr::<u32>(m, 0x28, one);
    wr::<u32>(m, 0x14, one);
    wr::<u32>(m, 0, one);
    let f = rd::<u32>(m, 0xC) | 0x20003;
    wr::<u32>(m, 0x10, 0);
    wr::<u32>(m, 0xC, f);
    for o in [0x8, 0x4, 0x24, 0x20, 0x18, 0x38, 0x34, 0x30] {
        wr::<u32>(m, o, 0);
    }
}

/// The original's bad-combine tail: `xor eax, eax; mov ecx, [eax+0xC]` (access violation).
fn bad_combine_crash(clear: u32) -> GPtr<u8> {
    error2(E_RW_BADPARAM, BAD_COMBINE_MSG);
    let null = p8(0);
    wr::<u32>(null, 0xC, rd::<u32>(null, 0xC) & clear);
    null
}

/// `RwMatrixScale(m, s, op)` (0x7F22C0). A bad `op` crashes as the original (NULL dereference).
#[hook(addr = 0x7F22C0, conv = cdecl, name = "RwMatrixScale", expect = "8B 44 24 0C 83 EC 08")]
pub fn rw_matrix_scale(m: GPtr<u8>, s: GPtr<RwV3d>, op: i32) -> GPtr<u8> {
    let sv: GPtr<u8> = s.cast();
    match op {
        0 => {
            set_identity_or_flags(m);
            wr::<u32>(m, 0, rd::<u32>(sv, 0));
            wr::<u32>(m, 0x14, rd::<u32>(sv, 4));
            wr::<u32>(m, 0x28, rd::<u32>(sv, 8));
            wr::<u32>(m, 0xC, rd::<u32>(m, 0xC) & 0xFFFD_FFFC);
        }
        1 => {
            // row 0: `fld m; fmul s`; rows 1 and 2: `fld s; fmul m`
            for o in [0x0, 0x4, 0x8] {
                st(m, o, ld(m, o).fmul_m(fm(sv, 0)));
            }
            for (o, k) in [(0x10, 4), (0x14, 4), (0x18, 4), (0x20, 8), (0x24, 8), (0x28, 8)] {
                st(m, o, ld(sv, k).fmul_m(fm(m, o)));
            }
            wr::<u32>(m, 0xC, rd::<u32>(m, 0xC) & 0xFFFD_FFFC);
        }
        2 => {
            // `fld m; fmul s` except +0x14 and +0x34 (`fld s.y; fmul m`)
            for row in [0x0, 0x10, 0x20, 0x30] {
                for k in [0, 4, 8] {
                    let o = row + k;
                    let v = if o == 0x14 || o == 0x34 {
                        ld(sv, k).fmul_m(fm(m, o))
                    } else {
                        ld(m, o).fmul_m(fm(sv, k))
                    };
                    st(m, o, v);
                }
            }
            wr::<u32>(m, 0xC, rd::<u32>(m, 0xC) & 0xFFFD_FFFC);
        }
        _ => return bad_combine_crash(0xFFFD_FFFC),
    }
    m
}

/// `RwMatrixTranslate(m, t, op)` (0x7F2450). A bad `op` crashes as the original.
#[hook(addr = 0x7F2450, conv = cdecl, name = "RwMatrixTranslate", expect = "8B 44 24 0C 83 EC 08")]
pub fn rw_matrix_translate(m: GPtr<u8>, t: GPtr<RwV3d>, op: i32) -> GPtr<u8> {
    let tv: GPtr<u8> = t.cast();
    match op {
        0 => {
            set_identity_or_flags(m);
            wr::<u32>(m, 0x30, rd::<u32>(tv, 0));
            wr::<u32>(m, 0x34, rd::<u32>(tv, 4));
            wr::<u32>(m, 0x38, rd::<u32>(tv, 8));
        }
        1 => {
            for k in [0, 4, 8] {
                let v = ld(m, 0x20 + k)
                    .fmul_m(fm(tv, 8))
                    .fadd(ld(m, 0x10 + k).fmul_m(fm(tv, 4)))
                    .fadd(ld(m, k).fmul_m(fm(tv, 0)))
                    .fadd_m(fm(m, 0x30 + k));
                st(m, 0x30 + k, v);
            }
        }
        2 => {
            st(m, 0x30, ld(m, 0x30).fadd_m(fm(tv, 0)));
            st(m, 0x34, ld(tv, 4).fadd_m(fm(m, 0x34)));
            st(m, 0x38, ld(tv, 8).fadd_m(fm(m, 0x38)));
        }
        _ => return bad_combine_crash(0xFFFD_FFFF),
    }
    wr::<u32>(m, 0xC, rd::<u32>(m, 0xC) & 0xFFFD_FFFF);
    m
}

/// `RwMatrixTransform(m, t, op)` (0x7F25A0); NULL (with E_RW_BADPARAM) for a bad `op`.
#[hook(addr = 0x7F25A0, conv = cdecl, name = "RwMatrixTransform", expect = "8B 44 24 0C 83 EC 48")]
pub fn rw_matrix_transform(m: GPtr<u8>, t: GPtr<u8>, op: i32) -> GPtr<u8> {
    match op {
        0 => {
            copy16(m, t);
            return m;
        }
        1 | 2 => {}
        _ => {
            error2(E_RW_BADPARAM, BAD_COMBINE_MSG);
            return p8(0);
        }
    }
    let mask = identity_mask();
    let (mf, tf) = (rd::<u32>(m, 0xC), rd::<u32>(t, 0xC));
    let mut tmp = [0u32; 16];
    let tp: GPtr<u8> = GPtr::from_raw(tmp.as_mut_ptr().cast());
    let (first_ident, src_if_first, second_ident, src_if_second) = if op == 1 {
        (tf & mask != 0, m, mf & mask != 0, t)
    } else {
        (mf & mask != 0, t, tf & mask != 0, m)
    };
    if first_ident {
        copy16(tp, src_if_first);
    } else if second_ident {
        copy16(tp, src_if_second);
    } else {
        let f = rd::<u32>(mmod(), 8);
        if op == 1 {
            call_multiply(f, tp, t, m);
        } else {
            call_multiply(f, tp, m, t);
        }
        wr::<u32>(tp, 0xC, mf & tf);
    }
    copy16(m, tp);
    m
}

/// `RwMatrixDestroy(m)` (0x7F2A20): `G->memoryFree(M+0, m)`; returns 1.
#[hook(addr = 0x7F2A20, conv = cdecl, name = "RwMatrixDestroy", expect = "8B 44 24 04 8B 0D BC 79 C9 00")]
pub fn rw_matrix_destroy(m: GPtr<u8>) -> u32 {
    rw_memory_free(p8(rd::<u32>(mmod(), 0)), m);
    1
}

/// `RwMatrixCreate()` (0x7F2A50): `G->memoryAlloc(M+0, 0x3000D)`, identity (pads untouched),
/// flags 0x20003.
#[hook(addr = 0x7F2A50, conv = cdecl, name = "RwMatrixCreate", expect = "A1 24 7B C9 00")]
pub fn rw_matrix_create() -> GPtr<u8> {
    let m = rw_memory_alloc(p8(rd::<u32>(mmod(), 0)), 0x3000D);
    if m.is_null() {
        return m;
    }
    let one = ONE_F.to_bits();
    wr::<u32>(m, 0x10, 0);
    wr::<u32>(m, 0x28, one);
    wr::<u32>(m, 0x14, one);
    wr::<u32>(m, 0, one);
    for o in [0x8, 0x4, 0x24, 0x20, 0x18, 0x38, 0x34, 0x30] {
        wr::<u32>(m, o, 0);
    }
    wr::<u32>(m, 0xC, 0x20003);
    m
}

/// `cb_MATRIXMODULE_RwEngine_Open(instance, offset, size)` (0x7F16C0).
#[hook(addr = 0x7F16C0, conv = cdecl, name = "cb_MATRIXMODULE_RwEngine_Open", expect = "8B 0D 94 22 8E 00")]
pub fn matrix_module_open(instance: u32, offset: u32, _size: u32) -> u32 {
    let n = g::<u32>(0x8E2294).read();
    let per = g::<u32>(0x8E2290).read();
    g::<u32>(MATRIX).write(offset);
    let fl = freelist_create_prealloc(0x40, per, 4, n, p8(0xC97998), 0x4000D);
    wr::<u32>(mmod(), 0, fl.addr() as u32);
    if rd::<u32>(mmod(), 0) == 0 {
        return 0;
    }
    wr::<u32>(mmod(), 4, 0x20000);
    wr::<u32>(mmod(), 8, DEFAULT_MULTIPLY);
    for o in [0xC, 0x10, 0x14] {
        wr::<u32>(mmod(), o, 0x3C23_D70A);
    }
    let c = g::<u32>(MATRIX_INSTANCES);
    c.write(c.read().wrapping_add(1));
    instance
}

/// `RwBBoxCalculate(box, verts, n)` (0x808F60): `inf = sup = verts[0]` (dword copies, inf first); for
/// the remaining `n - 1` (u32, so n = 0 runs 2^32 - 1 times as the original) vertices `inf.k = v.k`
/// when `inf.k > v.k`, then `sup.k = v.k` when `sup.k < v.k` (NaN never replaces).
#[hook(addr = 0x808F60, conv = cdecl, name = "RwBBoxCalculate", expect = "8B 54 24 04 8B 4C 24 08")]
pub fn rw_bbox_calculate(bx: GPtr<u8>, verts: GPtr<u8>, n: i32) -> GPtr<u8> {
    for k in 0..3 {
        wr::<u32>(bx, 0xC + k * 4, rd::<u32>(verts, k * 4));
    }
    for k in 0..3 {
        wr::<u32>(bx, k * 4, rd::<u32>(verts, k * 4));
    }
    let mut left = (n as u32).wrapping_sub(1);
    let mut v = verts;
    while left != 0 {
        v = v.byte_add(0xC);
        for k in 0..3 {
            let c = ld(bx, 0xC + k * 4).fcom(ld(v, k * 4));
            if !c.test_any(0x41) {
                wr::<u32>(bx, 0xC + k * 4, rd::<u32>(v, k * 4));
            }
        }
        for k in 0..3 {
            let c = ld(bx, k * 4).fcom(ld(v, k * 4));
            if !c.test_parity(0x05) {
                wr::<u32>(bx, k * 4, rd::<u32>(v, k * 4));
            }
        }
        left -= 1;
    }
    bx
}

/// One SSE single-precision operation with the game's MXCSR (FTZ on, DAZ off, round to nearest): NaN
/// operands propagate the first operand's NaN (quieted), invalid operations give the default NaN
/// 0xFFC00000, and results that are subnormal after rounding are flushed to signed zero.
fn sse(a: f32, b: f32, op: fn(f32, f32) -> f32) -> f32 {
    if a.is_nan() {
        return f32::from_bits(a.to_bits() | 0x40_0000);
    }
    if b.is_nan() {
        return f32::from_bits(b.to_bits() | 0x40_0000);
    }
    let r = op(a, b);
    if r.is_nan() {
        return f32::from_bits(0xFFC0_0000);
    }
    if r.is_subnormal() {
        return f32::from_bits(r.to_bits() & 0x8000_0000);
    }
    r
}

fn mulps(a: f32, b: f32) -> f32 {
    sse(a, b, |p, q| p * q)
}
fn addps(a: f32, b: f32) -> f32 {
    sse(a, b, |p, q| p + q)
}

/// `_rwSSE2V3dTransformPoint` lane math: `(z*at + y*up) + (x*right + pos)`.
fn sse_point(v: [f32; 3], m: GPtr<u8>) -> [f32; 3] {
    let mut o = [0f32; 3];
    for (k, ok) in o.iter_mut().enumerate() {
        let k4 = k as isize * 4;
        let x0 = addps(mulps(v[0], fm(m, k4)), fm(m, 0x30 + k4));
        let x2 = addps(mulps(v[2], fm(m, 0x20 + k4)), mulps(v[1], fm(m, 0x10 + k4)));
        *ok = addps(x2, x0);
    }
    o
}

/// `_rwSSE2V3dTransformVector` lane math: `z*at + (y*up + x*right)`.
fn sse_vector(v: [f32; 3], m: GPtr<u8>) -> [f32; 3] {
    let mut o = [0f32; 3];
    for (k, ok) in o.iter_mut().enumerate() {
        let k4 = k as isize * 4;
        let x1 = addps(mulps(v[1], fm(m, 0x10 + k4)), mulps(v[0], fm(m, k4)));
        *ok = addps(mulps(v[2], fm(m, 0x20 + k4)), x1);
    }
    o
}

fn rd3(p: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

fn wr3(p: GPtr<u8>, v: [f32; 3]) {
    for (k, c) in v.iter().enumerate() {
        wr::<f32>(p, k as isize * 4, *c);
    }
}

/// `_rwSSE2V3dTransformPoints(out, in, n, m)` (0x80D440); n = 0 loops 2^32 times as the original.
#[hook(addr = 0x80D440, conv = cdecl, name = "_rwSSE2V3dTransformPoints", expect = "55 8B EC 56 57")]
pub fn rw_sse2_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> {
    let mut left = n as u32;
    let (mut i, mut o) = (inp, out);
    loop {
        wr3(o, sse_point(rd3(i), m));
        i = i.byte_add(0xC);
        o = o.byte_add(0xC);
        left = left.wrapping_sub(1);
        if left == 0 {
            break;
        }
    }
    out
}

/// `_rwSSE2V3dTransformVectors(out, in, n, m)` (0x80D4E0).
#[hook(addr = 0x80D4E0, conv = cdecl, name = "_rwSSE2V3dTransformVectors", expect = "55 8B EC 56 57")]
pub fn rw_sse2_v3d_transform_vectors(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> {
    let mut left = n as u32;
    let (mut i, mut o) = (inp, out);
    loop {
        wr3(o, sse_vector(rd3(i), m));
        i = i.byte_add(0xC);
        o = o.byte_add(0xC);
        left = left.wrapping_sub(1);
        if left == 0 {
            break;
        }
    }
    out
}

/// `_rwSSE2V3dTransformPoint(out, in, m)` (0x80D570).
#[hook(addr = 0x80D570, conv = cdecl, name = "_rwSSE2V3dTransformPoint", expect = "55 8B EC 8B 45 10")]
pub fn rw_sse2_v3d_transform_point(out: GPtr<u8>, inp: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> {
    wr3(out, sse_point(rd3(inp), m));
    out
}

/// `_rwSSE2V3dTransformVector(out, in, m)` (0x80D600).
#[hook(addr = 0x80D600, conv = cdecl, name = "_rwSSE2V3dTransformVector", expect = "55 8B EC 8B 45 10")]
pub fn rw_sse2_v3d_transform_vector(out: GPtr<u8>, inp: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> {
    wr3(out, sse_vector(rd3(inp), m));
    out
}

/// Every `#[hook]` of this module.
/// `RwMatrixOrthoNormalize(RwMatrix* dst, const RwMatrix* src)` (0x7F1920, a `jmp` to 0x7F1930;
/// returns `dst`) (STD-6). On a stack copy of src's right R, up U, at A: each is scaled by
/// `_rwInvSqrt` of its squared length (`(y*y + x*x) + z*z`). If an inverse length is not > 0 (R, then U,
/// then A checked) that axis is rebuilt from the other two; otherwise the axis pair with the smallest
/// `|dot|` (|A.U|, |A.R|, |U.R|, ties as the original's `test ah,5` compares) keeps its first member and
/// the third axis is rebuilt: with (b, s, d) = the chosen triple, `d = s x b` normalised, then
/// `s = b x d` normalised (`(x*x + y*y) + z*z` here). The matrix (position copied) goes to dst and its
/// flags +0xC get `& ~0x20000 | 3`.
#[sa_core::native(addr = 0x7F1920, conv = cdecl, name = "RwMatrixOrthoNormalize")]
pub fn matrix_ortho_normalize(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    let inv = |v: X87| crate::vector::rw_inv_sqrt(v.to_f32());
    let mut v = [[0f32; 3]; 3];
    for (i, a) in v.iter_mut().enumerate() {
        for (j, c) in a.iter_mut().enumerate() {
            *c = rd::<f32>(src, (i * 0x10 + j * 4) as isize);
        }
    }
    let pos: [u32; 3] = [rd(src, 0x30), rd(src, 0x34), rd(src, 0x38)];
    let len2 = |a: &[f32; 3]| x(a[1]).fmul_m(a[1]).fadd(x(a[0]).fmul_m(a[0])).fadd(x(a[2]).fmul_m(a[2]));
    let mut il = [X87::zero(); 3];
    for i in 0..3 {
        let k = inv(len2(&v[i]));
        il[i] = k;
        for c in v[i].iter_mut() {
            *c = k.fmul_m(*c).to_f32();
        }
    }
    let (ir, iu) = (il[0].to_f32(), il[1].to_f32());
    let z = X87::zero();
    let le = |a: X87| a.fcom(z).test_any(0x41);
    const R: usize = 0;
    const U: usize = 1;
    const A: usize = 2;
    let (b, s, d) = if le(x(ir)) {
        (U, A, R)
    } else if le(x(iu)) {
        (A, R, U)
    } else if le(il[2]) {
        (R, U, A)
    } else {
        let dot = |p: usize, q: usize| {
            x(v[p][1]).fmul_m(v[q][1]).fadd(x(v[p][0]).fmul_m(v[q][0])).fadd(x(v[p][2]).fmul_m(v[q][2]))
        };
        let abs32 = |t: X87| {
            let f = t.to_f32();
            if t.fcom(z).test_any(0x01) { -f } else { f }
        };
        let au = abs32(dot(A, U));
        let ar = abs32(dot(A, R));
        let ur = dot(U, R);
        let ur = if ur.fcom(z).test_any(0x01) { ur.fchs() } else { ur };
        if x(au).fcom(x(ar)).test_parity(0x05) {
            if x(ar).fcom(ur).test_parity(0x05) { (R, U, A) } else { (A, R, U) }
        } else if !x(au).fcom(ur).test_parity(0x05) {
            (U, A, R)
        } else {
            (R, U, A)
        }
    };
    let cross_norm = |v: &mut [[f32; 3]; 3], b: usize, s: usize, d: usize, b_first: bool| {
        // d = s x b, element by element as the original forms them
        let (bb, ss) = (v[b], v[s]);
        let dx = x(ss[2]).fmul_m(bb[1]).fsub(x(ss[1]).fmul_m(bb[2]));
        let dy = x(bb[2]).fmul_m(ss[0]).fsub(x(ss[2]).fmul_m(bb[0]));
        let dz = if b_first {
            x(ss[1]).fmul_m(bb[0]).fsub(x(bb[1]).fmul_m(ss[0]))
        } else {
            x(bb[1]).fmul_m(ss[0]).fsub(x(ss[1]).fmul_m(bb[0]))
        };
        v[d] = [dx.to_f32(), dy.to_f32(), dz.to_f32()];
        let (fx, fy) = (v[d][0], v[d][1]);
        let k = inv(x(fx).fmul_m(fx).fadd(x(fy).fmul_m(fy)).fadd(dz.fmul(dz)));
        v[d][0] = k.fmul_m(v[d][0]).to_f32();
        let ny = x(v[d][1]).fmul(k);
        v[d][1] = ny.to_f32();
        let nz = x(v[d][2]).fmul(k);
        v[d][2] = nz.to_f32();
        (ny, nz)
    };
    // d = s x b (0x7F1BAB)
    let (ny, nz) = cross_norm(&mut v, b, s, d, true);
    // s = b x d (0x7F1C13): x from the register copies of d.y / d.z, y / z from memory
    let bb = v[b];
    let sx = x(bb[2]).fmul(ny).fsub(x(bb[1]).fmul(nz));
    let sy = x(v[d][2]).fmul_m(bb[0]).fsub(x(bb[2]).fmul_m(v[d][0]));
    let sz = x(bb[1]).fmul_m(v[d][0]).fsub(x(v[d][1]).fmul_m(bb[0]));
    v[s] = [sx.to_f32(), sy.to_f32(), sz.to_f32()];
    let (fx, fy) = (v[s][0], v[s][1]);
    let k = inv(x(fx).fmul_m(fx).fadd(x(fy).fmul_m(fy)).fadd(sz.fmul(sz)));
    for c in v[s].iter_mut() {
        *c = k.fmul_m(*c).to_f32();
    }
    for (i, a) in v.iter().enumerate() {
        for (j, c) in a.iter().enumerate() {
            wr::<f32>(dst, (i * 0x10 + j * 4) as isize, *c);
        }
    }
    wr::<u32>(dst, 0x30, pos[0]);
    wr::<u32>(dst, 0x34, pos[1]);
    wr::<u32>(dst, 0x38, pos[2]);
    wr::<u32>(dst, 0xC, (rd::<u32>(dst, 0xC) & !0x20000) | 3);
    dst
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RW_MATRIX_SET_MULT_FN,
    HOOK_RW_MATRIX_DETERMINANT,
    HOOK_RW_MATRIX_ORTHOGONAL_ERROR,
    HOOK_RW_MATRIX_NORMAL_ERROR,
    HOOK_RW_MATRIX_OPTIMIZE,
    HOOK_RW_MATRIX_UPDATE,
    HOOK_RW_MATRIX_MULTIPLY,
    HOOK_RW_MATRIX_ROTATE_ONE_MINUS_COSINE_SINE,
    HOOK_RW_MATRIX_ROTATE,
    HOOK_RW_MATRIX_INVERT,
    HOOK_RW_MATRIX_INVERT_GENERAL,
    HOOK_RW_MATRIX_SCALE,
    HOOK_RW_MATRIX_TRANSLATE,
    HOOK_RW_MATRIX_TRANSFORM,
    HOOK_RW_MATRIX_DESTROY,
    HOOK_RW_MATRIX_CREATE,
    HOOK_MATRIX_MODULE_OPEN,
    HOOK_RW_BBOX_CALCULATE,
    HOOK_RW_SSE2_V3D_TRANSFORM_POINTS,
    HOOK_RW_SSE2_V3D_TRANSFORM_VECTORS,
    HOOK_RW_SSE2_V3D_TRANSFORM_POINT,
    HOOK_RW_SSE2_V3D_TRANSFORM_VECTOR,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
