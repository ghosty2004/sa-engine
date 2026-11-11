//! VECTORMODULE (engine plugin 0x401): the 12-bit table square root / inverse square root, `RwV3d`
//! length / normalise / transforms and the module open (PORT-RW-1). Ported from the 1.0 US bytes;
//! `docs/re/specs/RW/RwMatrix.md` is the reading aid.
//!
//! Module `V = G + [0xC97934]`: +0 sqrt table (4096 u32), +4 inverse sqrt table, +8 transformPoint,
//! +0xC transformPoints, +0x10 transformVector, +0x14 transformVectors.
//!
//! Table lookup (bit-exact semantics of the originals): `b = bits(x)`; `b == 0` (only +0.0) gives +0.0
//! (the stored value itself). Otherwise `b' = b + 0x800`, `i = (b' >> 12) & 0xFFF`;
//! sqrt = `T[i] + ((b' >> 1) & 0x3FC00000)`, inverse = `U[i] + ((!b' >> 1) & 0x3FC00000)` (wrapping
//! u32 adds, no sign or NaN handling).

use sa_core::abi::{Fp, HookReg, stack_footprint};
use sa_core::light::RwV3d;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{error1_code, error2, f32_of, g, module, p8, rd, rw_malloc, wr, x};
use crate::layout::modules::VECTOR;

/// `0.0f` at 0x858B50 (compare constant of `RwV3dNormalize`).
const ZERO_F: f32 = 0.0;

/// Default transform functions (0x7ED730, 0x7ED670, 0x7ED880, 0x7ED7D0).
pub const DEFAULT_TRANSFORM_POINT: u32 = 0x7ED730;
pub const DEFAULT_TRANSFORM_POINTS: u32 = 0x7ED670;
pub const DEFAULT_TRANSFORM_VECTOR: u32 = 0x7ED880;
pub const DEFAULT_TRANSFORM_VECTORS: u32 = 0x7ED7D0;

/// VECTORMODULE instance counter.
const VECTOR_INSTANCES: usize = 0xC97938;

orig! {
    /// `V->transformPoint(out, in, m)` (stored pointer)
    pub fn call_transform_point(f: u32, out: GPtr<RwV3d>, inp: GPtr<RwV3d>, m: GPtr<u8>) = cdecl dyn => unported;
    /// `V->transformPoints(out, in, n, m)` (stored pointer)
    pub fn call_transform_points(f: u32, out: GPtr<RwV3d>, inp: GPtr<RwV3d>, n: i32, m: GPtr<u8>) = cdecl dyn => unported;
}

fn vmod() -> GPtr<u8> {
    module(VECTOR)
}

/// The sqrt table `V+0`.
pub fn sqrt_table() -> GPtr<u32> {
    p8(rd::<u32>(vmod(), 0)).cast()
}

/// The inverse sqrt table `V+4`.
pub fn inv_sqrt_table() -> GPtr<u32> {
    p8(rd::<u32>(vmod(), 4)).cast()
}

/// Sqrt lookup on the bits of an f32 with the table `t` (returns result bits).
pub fn sqrt_lookup_in(t: GPtr<u32>, b: u32) -> u32 {
    if b == 0 {
        return b;
    }
    let b = b.wrapping_add(0x800);
    t.wrapping_index(((b >> 12) & 0xFFF) as i32)
        .read()
        .wrapping_add((b >> 1) & 0x3FC0_0000)
}

/// Inverse sqrt lookup on the bits of an f32 with the table `u` (returns result bits).
pub fn inv_sqrt_lookup_in(u: GPtr<u32>, b: u32) -> u32 {
    if b == 0 {
        return b;
    }
    let b = b.wrapping_add(0x800);
    u.wrapping_index(((b >> 12) & 0xFFF) as i32)
        .read()
        .wrapping_add((!b >> 1) & 0x3FC0_0000)
}

/// `(x*x + y*y) + z*z` on the x87 stack (load order y, x, z), unstored.
pub fn len_sq(v: RwV3d) -> X87 {
    let (vx, vy, vz) = (x(v.x), x(v.y), x(v.z));
    vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz))
}

/// `_rwSqrt(x)` (0x7EDB30): table sqrt, f32 in ST0.
#[hook(addr = 0x7EDB30, conv = cdecl, name = "_rwSqrt", expect = "8B 44 24 04 89 44 24 04")]
pub fn rw_sqrt(v: f32) -> X87 {
    rw_sqrt_in(sqrt_table(), v)
}

/// [`rw_sqrt`] as the f64 the original's ST0 return becomes (the type `orig!` declarations use: an
/// `X87` is not an FFI return type).
pub fn rw_sqrt_f64(v: f32) -> f64 {
    rw_sqrt(v).to_f64()
}

pub fn rw_sqrt_in(t: GPtr<u32>, v: f32) -> X87 {
    x(f32::from_bits(sqrt_lookup_in(t, v.to_bits())))
}

/// `_rwInvSqrt(x)` (0x7EDB90): table inverse sqrt (+0.0 gives +0.0), f32 in ST0.
#[hook(addr = 0x7EDB90, conv = cdecl, name = "_rwInvSqrt", expect = "8B 44 24 04 89 44 24 04")]
pub fn rw_inv_sqrt(v: f32) -> X87 {
    rw_inv_sqrt_in(inv_sqrt_table(), v)
}

pub fn rw_inv_sqrt_in(u: GPtr<u32>, v: f32) -> X87 {
    x(f32::from_bits(inv_sqrt_lookup_in(u, v.to_bits())))
}

/// `RwV3dLength(v)` (0x7EDAC0): `sqrtLookup(f32((x*x + y*y) + z*z))`.
#[hook(addr = 0x7EDAC0, conv = cdecl, name = "RwV3dLength", expect = "8B 44 24 04 D9 40 04")]
pub fn rw_v3d_length(v: GPtr<RwV3d>) -> X87 {
    rw_v3d_length_in(sqrt_table(), v)
}

pub fn rw_v3d_length_in(t: GPtr<u32>, v: GPtr<RwV3d>) -> X87 {
    let s = f32_of(len_sq(v.read()));
    x(f32::from_bits(sqrt_lookup_in(t, s.to_bits())))
}

/// `RwV3dNormalize(out, in)` (0x7ED9B0): returns the table length; `out = in * invSqrt` (x written
/// before y/z are read); `RwErrorSet(E_RW_ZEROLENGTH)` when the length compares `<= 0` (a NaN length
/// does not set it: `test ah, 0x41; jp` skips on unordered).
///
/// Stack footprint (`footprint`, ADR-001 §5b): original code reads this function's stack residue
/// (CWeapon::FireInstantHit 0x73FB10 passes an uninitialised local to AddGunshell; c-bug frame 2494),
/// so the hook leaves what the original leaves, relative to the entry ESP S (from the disassembly):
/// `sub esp, 0xC; push esi` → [S-0x10] = esi; `fst [esp+4]` → [S-0xC] = f32(s), then (s != 0, after
/// `push edi` → [S-0x14] = edi) the sqrt table value; `fstp [esp+0x18]` → the `in` argument slot
/// [S+8] = f32(s), then (s != 0) the inverse-sqrt table value. Error path (`add esp` restored, ESI
/// popped): `push 0x19` → [S-0x10]; `mov [esp+8], 1` → [S-8]; `call _rwerror` (ret 0x7EDAA6 at
/// [S-0x14], its frame below); `mov [esp+0xC], eax` → [S-4] = the code; `push &err` → [S-0x14] = S-8;
/// `call RwErrorSet` → [S-0x18] = 0x7EDAB4, and RwErrorSet's `push esi` → [S-0x1C] = the caller's
/// ESI. Not reproduced: deeper residue of the error path's callees ([S-0x20] and below; RwErrorSet
/// pushes EDI there on one of its paths, _rwerror's frame is overwritten except below that). The
/// 0x800-byte window covers the Rust shim's depth: below S only the slots above remain changed.
#[hook(addr = 0x7ED9B0, conv = cdecl, name = "RwV3dNormalize", expect = "83 EC 0C 8B 44 24 14", footprint = 0x800)]
pub fn rw_v3d_normalize(out: GPtr<RwV3d>, inp: GPtr<RwV3d>) -> X87 {
    let s = f32_of(len_sq(inp.read())).to_bits();
    let (len, err) = rw_v3d_normalize_in(sqrt_table(), inv_sqrt_table(), out, inp);
    let (len_bits, r_bits) = if s == 0 {
        (0, 0)
    } else {
        (len.to_f32().to_bits(), inv_sqrt_lookup_in(inv_sqrt_table(), s))
    };
    let fp = [(-0x10, Fp::Esi), (-0xC, Fp::Val(len_bits)), (8, Fp::Val(r_bits))];
    if s == 0 {
        stack_footprint(&fp);
    } else {
        stack_footprint(&[(-0x14, Fp::Edi)]);
        stack_footprint(&fp);
    }
    if err {
        let code = error1_code(0x19);
        stack_footprint(&[
            (-0x10, Fp::Val(0x19)),
            (-8, Fp::Val(1)),
            (-4, Fp::Val(code)),
            (-0x14, Fp::Addr(-8)),
            (-0x18, Fp::Val(0x7EDAB4)),
            (-0x1C, Fp::Esi),
        ]);
    }
    len
}

/// [`rw_v3d_normalize`] with the tables given; returns (length, error).
pub fn rw_v3d_normalize_in(t: GPtr<u32>, u: GPtr<u32>, out: GPtr<RwV3d>, inp: GPtr<RwV3d>) -> (X87, bool) {
    let v = inp.read();
    let s = f32_of(len_sq(v)).to_bits();
    let len = f32::from_bits(sqrt_lookup_in(t, s));
    let r = f32::from_bits(inv_sqrt_lookup_in(u, s));
    let xo = f32_of(x(r).fmul_m(v.x));
    out.cast::<f32>().write(xo);
    let ip = inp.cast::<f32>();
    let yo = f32_of(x(ip.wrapping_index(1).read()).fmul_m(r));
    out.cast::<f32>().wrapping_index(1).write(yo);
    let zo = f32_of(x(ip.wrapping_index(2).read()).fmul_m(r));
    out.cast::<f32>().wrapping_index(2).write(zo);
    let err = !x(len).fcom(x(ZERO_F)).test_parity(0x41);
    (x(len), err)
}

/// `RwV3dTransformPoint(out, in, m)` (0x7EDD60): `V->transformPoint(out, in, m)`; returns out.
#[hook(addr = 0x7EDD60, conv = cdecl, name = "RwV3dTransformPoint", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn rw_v3d_transform_point(out: GPtr<RwV3d>, inp: GPtr<RwV3d>, m: GPtr<u8>) -> GPtr<RwV3d> {
    call_transform_point(rd::<u32>(vmod(), 8), out, inp, m);
    out
}

/// `RwV3dTransformPoints(out, in, n, m)` (0x7EDD90): `V->transformPoints`; returns out.
#[hook(addr = 0x7EDD90, conv = cdecl, name = "RwV3dTransformPoints", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn rw_v3d_transform_points(out: GPtr<RwV3d>, inp: GPtr<RwV3d>, n: i32, m: GPtr<u8>) -> GPtr<RwV3d> {
    call_transform_points(rd::<u32>(vmod(), 0xC), out, inp, n, m);
    out
}

/// `RwV3dTransformVector(out, in, m)` (0x7EDDC0): `V->transformVector`; returns out.
#[hook(addr = 0x7EDDC0, conv = cdecl, name = "RwV3dTransformVector", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn rw_v3d_transform_vector(out: GPtr<RwV3d>, inp: GPtr<RwV3d>, m: GPtr<u8>) -> GPtr<RwV3d> {
    call_transform_point(rd::<u32>(vmod(), 0x10), out, inp, m);
    out
}

/// `RwV3dTransformVectors(out, in, n, m)` (0x7EDDF0): `V->transformVectors`; returns out.
#[hook(addr = 0x7EDDF0, conv = cdecl, name = "RwV3dTransformVectors", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn rw_v3d_transform_vectors(out: GPtr<RwV3d>, inp: GPtr<RwV3d>, n: i32, m: GPtr<u8>) -> GPtr<RwV3d> {
    call_transform_points(rd::<u32>(vmod(), 0x14), out, inp, n, m);
    out
}

/// `_rwVectorSetMultFn(point, points, vector, vectors)` (0x7ED5F0): NULL arguments take the defaults;
/// stores V+8, +0xC, +0x10, +0x14; returns 1.
#[hook(addr = 0x7ED5F0, conv = cdecl, name = "_rwVectorSetMultFn", expect = "8B 44 24 04 85 C0")]
pub fn rw_vector_set_mult_fn(point: u32, points: u32, vector: u32, vectors: u32) -> u32 {
    let pick = |f: u32, d: u32| if f == 0 { d } else { f };
    wr::<u32>(vmod(), 8, pick(point, DEFAULT_TRANSFORM_POINT));
    wr::<u32>(vmod(), 0xC, pick(points, DEFAULT_TRANSFORM_POINTS));
    wr::<u32>(vmod(), 0x10, pick(vector, DEFAULT_TRANSFORM_VECTOR));
    wr::<u32>(vmod(), 0x14, pick(vectors, DEFAULT_TRANSFORM_VECTORS));
    1
}

/// Fills the sqrt table (4096 words) as VECTORMODULE Open does: entries 0x800.. from x = 1.0
/// (`bits(f32(fsqrt(x))) - 0x1FC00000`), then entries 0.. from x = 2.0 (`- 0x20000000`); x advances by
/// 0x1000 in its bit pattern. `fsqrt` stored to f32 is correctly rounded at any precision control.
pub fn fill_sqrt_table(t: GPtr<u32>) {
    let mut xb: u32 = 0x3F80_0000;
    for i in 0..0x800 {
        let s = f32_of(x(f32::from_bits(xb)).fsqrt()).to_bits();
        t.wrapping_index(0x800 + i).write(s.wrapping_sub(0x1FC0_0000));
        xb = xb.wrapping_add(0x1000);
    }
    for i in 0..0x800 {
        let s = f32_of(x(f32::from_bits(xb)).fsqrt()).to_bits();
        t.wrapping_index(i).write(s.wrapping_sub(0x2000_0000));
        xb = xb.wrapping_add(0x1000);
    }
}

/// Fills the inverse sqrt table: `bits(f32(1.0 / fsqrt(x))) - 0x20000000` for entries 0x800.. from
/// x = 1.0, then `- 0x1FC00000` for entries 0.. from x = 2.0. Computed under PC=24 (the [`X87`] model);
/// see docs/arch/rw-port.md for the precision-control question.
pub fn fill_inv_sqrt_table(u: GPtr<u32>) {
    let mut xb: u32 = 0x3F80_0000;
    for i in 0..0x800 {
        let s = f32_of(X87::one().fdiv(x(f32::from_bits(xb)).fsqrt())).to_bits();
        u.wrapping_index(0x800 + i).write(s.wrapping_sub(0x2000_0000));
        xb = xb.wrapping_add(0x1000);
    }
    for i in 0..0x800 {
        let s = f32_of(X87::one().fdiv(x(f32::from_bits(xb)).fsqrt())).to_bits();
        u.wrapping_index(i).write(s.wrapping_sub(0x1FC0_0000));
        xb = xb.wrapping_add(0x1000);
    }
}

/// `cb_VECTORMODULE_RwEngine_Open(instance, offset, size)` (0x7EDE90): default transforms, both tables
/// (`G->rwmalloc(0x4000, 0x40401)` each; failure: E_RW_NOMEM error, NULL), instance counter.
#[hook(addr = 0x7EDE90, conv = cdecl, name = "cb_VECTORMODULE_RwEngine_Open", expect = "8B 44 24 08 8B 0D 24 7B C9 00")]
pub fn vector_module_open(instance: u32, offset: u32, _size: u32) -> u32 {
    g::<u32>(VECTOR).write(offset);
    wr::<u32>(vmod(), 8, DEFAULT_TRANSFORM_POINT);
    wr::<u32>(vmod(), 0xC, DEFAULT_TRANSFORM_POINTS);
    wr::<u32>(vmod(), 0x10, DEFAULT_TRANSFORM_VECTOR);
    wr::<u32>(vmod(), 0x14, DEFAULT_TRANSFORM_VECTORS);
    let t = rw_malloc(0x4000, 0x40401);
    if t.is_null() {
        error2(0x8000_0013, 0x4000);
        return 0;
    }
    fill_sqrt_table(t.cast());
    wr::<u32>(vmod(), 0, t.addr() as u32);
    let u = rw_malloc(0x4000, 0x40401);
    if u.is_null() {
        error2(0x8000_0013, 0x4000);
        return 0;
    }
    fill_inv_sqrt_table(u.cast());
    wr::<u32>(vmod(), 4, u.addr() as u32);
    let c = g::<u32>(VECTOR_INSTANCES);
    c.write(c.read().wrapping_add(1));
    instance
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RW_SQRT,
    HOOK_RW_INV_SQRT,
    HOOK_RW_V3D_LENGTH,
    HOOK_RW_V3D_NORMALIZE,
    HOOK_RW_V3D_TRANSFORM_POINT,
    HOOK_RW_V3D_TRANSFORM_POINTS,
    HOOK_RW_V3D_TRANSFORM_VECTOR,
    HOOK_RW_V3D_TRANSFORM_VECTORS,
    HOOK_RW_VECTOR_SET_MULT_FN,
    HOOK_VECTOR_MODULE_OPEN,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
