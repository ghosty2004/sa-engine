//! PORT-RW-5 module `d3d9inst` (helper A): the D3D9 vertex-declaration instancing converters
//! (`_rpD3D9VertexDeclarationInst*` / `UnInst*`, min/max and format finders) and the default atomic /
//! world-sector instance callbacks (see docs/arch/rw-port.md, PORT-RW-5). Ported from the 1.0 US bytes.
//!
//! Conventions of the converters (all `__cdecl`, `type` = D3DDECLTYPE):
//! - every converter returns `[0x874EF8 + 4*type]` (the element size), also for `n <= 0` and for a type
//!   without a case (then nothing is written; the table read is unchecked, as in the original);
//! - per vertex, `dst` advances by `stride` and the source by its element size; `n` is signed (`jle`);
//! - integer encodings: `rnd(v*S)` = x87 product (PC=24) stored to an f32 temporary, reloaded and
//!   converted by `fistp dword` under the game's control word (round to nearest even; NaN / out of
//!   range give 0x80000000), then truncated to the destination width; `fist(v)` = the same without the
//!   scale. No clamping anywhere;
//! - the originals reuse their own argument slots as temporaries (`n`, `stride`, `dst`, `t`, ...); not
//!   reproduced (the callers pop the arguments and never read them back);
//! - the write order inside one vertex follows the bytes for `InstWeights`, `InstV3d`, `InstV2d`,
//!   `InstV3dMorph`; `InstV3dComp` uses the `InstV3d` order (its own order differs only in when the
//!   constant 4th component is written; the addresses are distinct, so only an aliasing `src`/`dst`
//!   could tell).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, p8, rd, rw_free, rw_malloc, wr, x};

/// Vertex-declaration element type sizes (u32 per D3DDECLTYPE).
const DECL_TYPE_SIZES: u32 = 0x874EF8;
/// `D3DCAPS9.DeclTypes` of the driver's caps copy (low byte tested).
const CAPS_DECL_TYPES: usize = 0xC9_BFEC;

const F_ONE: u32 = 0x3F80_0000; // [0x858624]
const F_ZERO: u32 = 0; // [0x858B50]
const F_MINUS_ONE: u32 = 0xBF80_0000; // [0x858C1C]
const F_255: u32 = 0x437F_0000; // [0x859AAC]
const F_32767: u32 = 0x46FF_FE00; // [0x86B2C0]
const F_65535: u32 = 0x477F_FF00; // [0x8729E8]
const F_511: u32 = 0x43FF_8000; // [0x874F3C]
/// `[0x874F40]` = 1/511 (DEC3N test / decode).
const F_INV_511: u32 = 0x3B00_4020;
/// `[0x858B88]` = 1/128 (RpVertexNormal decode).
const F_INV_128: u32 = 0x3C00_0000;
/// `[0x859A3C]` = 1/255.
const F_INV_255: u32 = 0x3B80_8081;
/// `[0x858C7C]` = 1/32767.
const F_INV_32767: u32 = 0x3800_0100;
/// `[0x874F48]` = 1/65535.
const F_INV_65535: u32 = 0x3780_0080;
/// `[0x874F44]` = 1e-12 (tangent determinant threshold).
const F_TANGENT_EPS: u32 = 0x2B8C_BCCC;

orig! {
    /// `IDirect3DIndexBuffer9::Lock(this, offset, size, &data, flags)` (vtbl +0x2C)
    pub fn ib_lock(ib: GPtr<u8>, offset: u32, size: u32, data: GPtr<u32>, flags: u32) -> i32 = stdcall vslot 0x2C => unported;
    /// `IDirect3DIndexBuffer9::Unlock(this)` (vtbl +0x30)
    pub fn ib_unlock(ib: GPtr<u8>) -> i32 = stdcall vslot 0x30 => unported;
    /// `_rwInvSqrt(x)` (0x7EDB90), result in ST0
    pub fn rw_inv_sqrt(v: f32) -> f64 = cdecl 0x7EDB90 => inv_sqrt_f64;
    /// `_rpD3D9VertexDeclarationInstV3d(type, dst, src, n, stride)` (0x752AD0)
    pub fn inst_v3d_call(ty: u32, dst: u32, src: u32, n: i32, stride: u32) -> u32 = cdecl 0x752AD0 => inst_v3d;
    /// `_rpD3D9GetMinMaxValuesV3d(v, n, min, max)` (0x755830)
    pub fn min_max_v3d_call(v: GPtr<u8>, n: i32, min: GPtr<u8>, max: GPtr<u8>) = cdecl 0x755830 => get_min_max_values_v3d;
    /// `_rpD3D9GetMinMaxValuesV2d(uv, n, min, max)` (0x7558F0)
    pub fn min_max_v2d_call(uv: GPtr<u8>, n: i32, min: GPtr<u8>, max: GPtr<u8>) = cdecl 0x7558F0 => get_min_max_values_v2d;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn inv_sqrt_f64(v: f32) -> f64 {
    crate::vector::rw_inv_sqrt(v).to_f64()
}

// ---- raw access on 32-bit game addresses -------------------------------------------------------

#[inline]
fn r8(a: u32) -> u8 {
    rd::<u8>(p8(a), 0)
}
#[inline]
fn r16(a: u32) -> u16 {
    rd::<u16>(p8(a), 0)
}
#[inline]
fn r32(a: u32) -> u32 {
    rd::<u32>(p8(a), 0)
}
#[inline]
fn rf(a: u32) -> f32 {
    f32::from_bits(r32(a))
}
#[inline]
fn w8(a: u32, v: u8) {
    wr::<u8>(p8(a), 0, v)
}
#[inline]
fn w16(a: u32, v: u16) {
    wr::<u16>(p8(a), 0, v)
}
#[inline]
fn w32(a: u32, v: u32) {
    wr::<u32>(p8(a), 0, v)
}
#[inline]
fn wf(a: u32, v: f32) {
    w32(a, v.to_bits())
}

/// The converters' return value: `[0x874EF8 + 4*type]` (unchecked index).
#[inline]
fn decl_size(ty: u32) -> u32 {
    g::<u32>(DECL_TYPE_SIZES.wrapping_add(ty.wrapping_mul(4)) as usize).read()
}

#[inline]
fn kf(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `fld dword` of an f32, then `fistp dword` (RC = nearest even, the game's control word): NaN, ±inf and
/// results outside the i32 range give the integer indefinite 0x80000000.
pub fn fist(v: f32) -> i32 {
    let b = v.to_bits();
    let neg = b >> 31 != 0;
    let e = ((b >> 23) & 0xFF) as i32;
    if e == 0xFF {
        return i32::MIN;
    }
    let m = (b & 0x7F_FFFF) | if e != 0 { 0x80_0000 } else { 0 };
    // value = m * 2^sh
    let sh = if e == 0 { 1 } else { e } - 150;
    let mag: u64 = if sh >= 0 {
        if sh > 31 {
            return i32::MIN;
        }
        u64::from(m) << sh
    } else {
        let s = (-sh) as u32;
        if s > 25 {
            0
        } else {
            let q = m >> s;
            let r = m & ((1u32 << s) - 1);
            let half = 1u32 << (s - 1);
            u64::from(if r > half || (r == half && q & 1 == 1) { q + 1 } else { q })
        }
    };
    if neg {
        if mag > 0x8000_0000 {
            i32::MIN
        } else {
            (mag as i64).wrapping_neg() as i32
        }
    } else if mag > 0x7FFF_FFFF {
        i32::MIN
    } else {
        mag as i32
    }
}

/// `fld v; fmul dword [S]; fstp dword tmp; fld tmp; fistp dword`.
#[inline]
fn rnd(v: f32, s: u32) -> i32 {
    fist(x(v).fmul_m(kf(s)).to_f32())
}

/// The scale `S` of a normalised D3DDECLTYPE (D3DCOLOR / UBYTE4N 255, SHORT*N 32767, USHORT*N 65535,
/// DEC3N 511); `None` for the plain integer types (converted with `fist`).
fn norm_scale(ty: u32) -> Option<u32> {
    match ty {
        4 | 8 => Some(F_255),
        9 | 0xA => Some(F_32767),
        0xB | 0xC => Some(F_65535),
        0xE => Some(F_511),
        _ => None,
    }
}

/// Integer conversion of one component for an integer D3DDECLTYPE (`rnd(v*S)` or `fist(v)`).
#[inline]
fn conv(ty: u32, v: f32) -> i32 {
    match norm_scale(ty) {
        Some(s) => rnd(v, s),
        None => fist(v),
    }
}

/// UDEC3 / DEC3N packing of three converted components: `((z&0x3FF) << 10 | (y&0x3FF)) << 10 | (x&0x3FF)`.
#[inline]
fn pack3(xi: i32, yi: i32, zi: i32) -> u32 {
    let (xi, yi, zi) = (xi as u32 & 0x3FF, yi as u32 & 0x3FF, zi as u32 & 0x3FF);
    (((zi << 10) | yi) << 10) | xi
}

/// The integer encodings of an RwV3d-like triple (types 4..=0xE) from already converted components, in
/// the store order of `InstV3d` / `InstV3dMorph`.
fn put_v3_int(ty: u32, d: u32, xi: i32, yi: i32, zi: i32) {
    match ty {
        // D3DCOLOR: bytes {z, y, x, 0xFF}
        4 => {
            w8(d + 2, xi as u8);
            w8(d + 1, yi as u8);
            w8(d + 3, 0xFF);
            w8(d, zi as u8);
        }
        5 | 8 => {
            w8(d, xi as u8);
            w8(d + 1, yi as u8);
            w8(d + 3, if ty == 5 { 1 } else { 0xFF });
            w8(d + 2, zi as u8);
        }
        6 | 9 | 0xB => {
            w16(d, xi as u16);
            w16(d + 2, yi as u16);
        }
        7 | 0xA | 0xC => {
            w16(d, xi as u16);
            w16(d + 2, yi as u16);
            w16(
                d + 6,
                match ty {
                    7 => 1,
                    0xA => 0x7FFF,
                    _ => 0xFFFF,
                },
            );
            w16(d + 4, zi as u16);
        }
        0xD | 0xE => w32(d, pack3(xi, yi, zi)),
        _ => {}
    }
}

/// One RwV3d-like triple (f32 bit patterns) in the encodings of `InstV3d` (types 0..=0xE; FLOAT3 per
/// vertex, the `stride == 12` block copy is the caller's).
fn put_v3(ty: u32, d: u32, c: [u32; 3]) {
    match ty {
        0 => w32(d, c[0]),
        1 => {
            w32(d, c[0]);
            w32(d + 4, c[1]);
        }
        2 => {
            w32(d, c[0]);
            w32(d + 4, c[1]);
            w32(d + 8, c[2]);
        }
        3 => {
            w32(d, c[0]);
            w32(d + 4, c[1]);
            w32(d + 8, c[2]);
            w32(d + 12, F_ONE);
        }
        _ => {
            let f = |i: usize| f32::from_bits(c[i]);
            put_v3_int(ty, d, conv(ty, f(0)), conv(ty, f(1)), conv(ty, f(2)));
        }
    }
}

/// `_rpD3D9VertexDeclarationInstWeights` (0x752320): writes `n` RwMatrixWeights (4 f32, stride 0x10) as
/// `type`. FLOAT1/2/4: raw dword copies; FLOAT3 copies FOUR dwords (writes 16 bytes into a 12-byte
/// element, original bug kept). D3DCOLOR / UBYTE4N: `w0 < 1.0` false (not NaN) gives {0,0,0xFF,0} /
/// {0xFF,0,0,0}; else the bytes of w1..w3 scaled by 255 and the remaining byte `0xFF - the others`
/// (8-bit wrap; D3DCOLOR puts w1 at byte 1, w2 at byte 0, w3 at byte 3, remainder at byte 2; UBYTE4N w1..w3
/// at bytes 1..3, remainder at byte 0; the remainder re-reads the stored bytes). UBYTE4 / SHORT2 / SHORT4:
/// `fist(w_i)`. SHORT2N / USHORT2N: `{a, 0x7FFF - a}` / `{a, 0xFFFF - a}` with `a = rnd(w0*S)`; SHORT4N /
/// USHORT4N: `{0x7FFF - c - b - a, a, b, c}` with w1..w3 (stored in the order [2], [1], [3], [0]). UDEC3:
/// `fist` of w0..w2 packed. DEC3N: `w2 < 1/511` (C0 alone; NaN / equal take the other path) packs
/// `a = rnd(w0*511)&0x3FF` as `(0x7FC00 - (a << 10)) | a`, else `b = rnd(w1*511)&0x3FF`,
/// `c = rnd(w2*511)&0x3FF`, `((c << 10 | b) << 10) | (0x1FF - c - b)` (low term not masked). Returns the
/// element size of `type`.
#[hook(addr = 0x752320, conv = cdecl, name = "_rpD3D9VertexDeclarationInstWeights", expect = "8B 44 24 04 83 EC 10",
       route = "L2", hazards = "heap-writes",
       note = "writes into a locked vertex buffer (skin instancing); reuses its argument slots as temporaries")]
pub fn inst_weights(ty: u32, dst: u32, src: u32, n: i32, stride: u32) -> u32 {
    if ty <= 0xE && n > 0 {
        let (mut d, mut s) = (dst, src);
        for _ in 0..n {
            let w = |i: u32| rf(s + 4 * i);
            match ty {
                0 => w32(d, r32(s)),
                1 => {
                    w32(d, r32(s));
                    w32(d + 4, r32(s + 4));
                }
                2 | 3 => {
                    for i in 0..4 {
                        w32(d + 4 * i, r32(s + 4 * i));
                    }
                }
                4 => {
                    if !x(w(0)).fcom(x(kf(F_ONE))).c0() {
                        w8(d + 2, 0xFF);
                        w8(d + 1, 0);
                        w8(d, 0);
                        w8(d + 3, 0);
                    } else {
                        w8(d + 1, rnd(w(1), F_255) as u8);
                        w8(d, rnd(w(2), F_255) as u8);
                        let a3 = rnd(w(3), F_255) as u8;
                        w8(d + 3, a3);
                        let rem = 0xFFu8.wrapping_sub(a3).wrapping_sub(r8(d + 1)).wrapping_sub(r8(d));
                        w8(d + 2, rem);
                    }
                }
                5 => {
                    for i in 0..4 {
                        w8(d + i, fist(w(i)) as u8);
                    }
                }
                6 => {
                    w16(d, fist(w(0)) as u16);
                    w16(d + 2, fist(w(1)) as u16);
                }
                7 => {
                    for i in 0..4 {
                        w16(d + 2 * i, fist(w(i)) as u16);
                    }
                }
                8 => {
                    if !x(w(0)).fcom(x(kf(F_ONE))).c0() {
                        w8(d, 0xFF);
                        w8(d + 1, 0);
                        w8(d + 2, 0);
                        w8(d + 3, 0);
                    } else {
                        w8(d + 1, rnd(w(1), F_255) as u8);
                        w8(d + 2, rnd(w(2), F_255) as u8);
                        let a3 = rnd(w(3), F_255) as u8;
                        let mut rem = 0xFFu8.wrapping_sub(r8(d + 2));
                        w8(d + 3, a3);
                        rem = rem.wrapping_sub(a3).wrapping_sub(r8(d + 1));
                        w8(d, rem);
                    }
                }
                9 | 0xB => {
                    let (sc, top) = if ty == 9 { (F_32767, 0x7FFFu32) } else { (F_65535, 0xFFFF) };
                    let a = rnd(w(0), sc) as u32;
                    w16(d, a as u16);
                    w16(d + 2, top.wrapping_sub(a) as u16);
                }
                0xA | 0xC => {
                    let (sc, top) = if ty == 0xA { (F_32767, 0x7FFFu32) } else { (F_65535, 0xFFFF) };
                    let a = rnd(w(1), sc) as u32;
                    let b = rnd(w(2), sc) as u32;
                    let c = rnd(w(3), sc) as u32;
                    let rem = top.wrapping_sub(c).wrapping_sub(b).wrapping_sub(a);
                    w16(d + 4, b as u16);
                    w16(d + 2, a as u16);
                    w16(d + 6, c as u16);
                    w16(d, rem as u16);
                }
                0xD => w32(d, pack3(fist(w(0)), fist(w(1)), fist(w(2)))),
                _ => {
                    // 0xE DEC3N: `fcomp [1/511]; test ah, 5; jp` -> the first path only for Less
                    let v = if !x(w(2)).fcom(x(kf(F_INV_511))).test_parity(0x05) {
                        let a = rnd(w(0), F_511) as u32 & 0x3FF;
                        0x7_FC00u32.wrapping_sub(a << 10) | a
                    } else {
                        let b = rnd(w(1), F_511) as u32 & 0x3FF;
                        let c = rnd(w(2), F_511) as u32 & 0x3FF;
                        let low = 0x1FFu32.wrapping_sub(c).wrapping_sub(b);
                        (((c << 10) | b) << 10) | low
                    };
                    w32(d, v);
                }
            }
            d = d.wrapping_add(stride);
            s = s.wrapping_add(0x10);
        }
    }
    decl_size(ty)
}

/// `_rpD3D9VertexDeclarationInstV3d` (0x752AD0): writes `n` RwV3d (stride 0xC) as `type`. FLOAT1/2/3:
/// raw dword copies; FLOAT3 with `stride == 12` is one `rep movsd` of `(n*12) >> 2` dwords WITHOUT the
/// `n <= 0` test (a negative `n` copies a wrapped, huge count, as the original); FLOAT4 adds 1.0f.
/// D3DCOLOR: bytes {rnd(z*255), rnd(y*255), rnd(x*255), 0xFF}; UBYTE4 {fist x, y, z, 1}; SHORT2 / SHORT4
/// `fist` (4th = 1); UBYTE4N {rnd(*255) x, y, z, 0xFF}; SHORT2N / SHORT4N `rnd(*32767)` (4th 0x7FFF);
/// USHORT2N / USHORT4N `rnd(*65535)` (4th 0xFFFF); UDEC3 `fist`, DEC3N `rnd(*511)`, packed
/// `(z << 20 | y << 10 | x)` of the low 10 bits. Returns the element size of `type` (also for 0xF/0x10).
#[hook(addr = 0x752AD0, conv = cdecl, name = "_rpD3D9VertexDeclarationInstV3d", expect = "51 53 55 8B 6C 24 10",
       route = "L2", hazards = "heap-writes",
       note = "writes into a locked vertex buffer; reuses its argument slots as temporaries")]
pub fn inst_v3d(ty: u32, dst: u32, src: u32, n: i32, stride: u32) -> u32 {
    if ty > 0xE {
        return decl_size(ty);
    }
    if ty == 2 && stride == 0xC {
        // rep movsd (n*12 >> 2 dwords), rep movsb (n*12 & 3 = 0 bytes); no n test
        let cnt = (n as u32).wrapping_mul(12) >> 2;
        let mut i = 0u32;
        while i < cnt {
            w32(dst.wrapping_add(4 * i), r32(src.wrapping_add(4 * i)));
            i += 1;
        }
        return decl_size(ty);
    }
    let (mut d, mut s) = (dst, src);
    for _ in 0..n.max(0) {
        put_v3(ty, d, [r32(s), r32(s + 4), r32(s + 8)]);
        d = d.wrapping_add(stride);
        s = s.wrapping_add(0xC);
    }
    decl_size(ty)
}

/// `_rpD3D9VertexDeclarationInstV3dComp` (0x7531B0): as [`inst_v3d`] for `n` RpVertexNormal (4 bytes,
/// three s8 components used): each component is `f32(fild(s8) * 1/128)` (exact), then encoded as by
/// `InstV3d` (no FLOAT3 block-copy case here). Returns the element size of `type`.
#[hook(addr = 0x7531B0, conv = cdecl, name = "_rpD3D9VertexDeclarationInstV3dComp", expect = "83 EC 0C 53 55 8B 6C 24 18",
       unverified = "rw5-inst-1", route = "L2", hazards = "heap-writes",
       note = "writes into a locked vertex buffer; reuses its argument slots as temporaries")]
pub fn inst_v3d_comp(ty: u32, dst: u32, src: u32, n: i32, stride: u32) -> u32 {
    if ty <= 0xE {
        let (mut d, mut s) = (dst, src);
        for _ in 0..n.max(0) {
            let c = |i: u32| {
                X87::from_i32(i32::from(r8(s + i) as i8))
                    .fmul_m(kf(F_INV_128))
                    .to_f32()
                    .to_bits()
            };
            put_v3(ty, d, [c(0), c(1), c(2)]);
            d = d.wrapping_add(stride);
            s = s.wrapping_add(4);
        }
    }
    decl_size(ty)
}

/// `_rpD3D9VertexDeclarationInstV3dMorph` (0x753B60): writes `n` interpolations of the RwV3d arrays `a`
/// and `b` at `t`. The float cases and UBYTE4 / SHORT2 / SHORT4 / UDEC3 use `v = f32((b - a)*t + a)`
/// (`fld b; fsub a; fmul t; fadd a`) then encode like `InstV3d` (`fist` for the integer ones). The
/// normalised cases (D3DCOLOR, UBYTE4N, SHORT*N, USHORT*N, DEC3N) first compute `k0 = f32((1.0 - t)*S)`
/// and `k1 = f32(t*S)` (k1 stored over the `t` argument slot, before the `n` test; not reproduced) and
/// convert `v = fist(f32(k0*a + k1*b))` (the bytes add in either order per case; x87 addition is
/// commutative, NaNs end as 0x80000000). Same element layouts as `InstV3d`. Returns the element size.
#[hook(addr = 0x753B60, conv = cdecl, name = "_rpD3D9VertexDeclarationInstV3dMorph", expect = "51 8B 44 24 08 53 55 56",
       unverified = "rw5-inst-1", route = "L2", hazards = "heap-writes",
       note = "writes into a locked vertex buffer; overwrites its t argument slot with t*S (not reproduced)")]
pub fn inst_v3d_morph(ty: u32, dst: u32, a: u32, b: u32, t: f32, n: i32, stride: u32) -> u32 {
    if ty > 0xE {
        return decl_size(ty);
    }
    let norm = norm_scale(ty);
    let (k0, k1) = match norm {
        Some(s) => (x(kf(F_ONE)).fsub_m(t).fmul_m(kf(s)).to_f32(), x(t).fmul_m(kf(s)).to_f32()),
        None => (0.0, 0.0),
    };
    let (mut d, mut pa, mut pb) = (dst, a, b);
    for _ in 0..n.max(0) {
        if norm.is_some() {
            let v = |o: u32| fist(x(k0).fmul_m(rf(pa + o)).fadd(x(k1).fmul_m(rf(pb + o))).to_f32());
            put_v3_int(ty, d, v(0), v(4), v(8));
        } else {
            let lerp =
                |o: u32| x(rf(pb + o)).fsub_m(rf(pa + o)).fmul_m(t).fadd_m(rf(pa + o)).to_f32().to_bits();
            put_v3(ty, d, [lerp(0), lerp(4), lerp(8)]);
        }
        d = d.wrapping_add(stride);
        pa = pa.wrapping_add(0xC);
        pb = pb.wrapping_add(0xC);
    }
    decl_size(ty)
}

/// `_rpD3D9VertexDeclarationInstV2d` (0x7544E0): writes `n` RwV2d (stride 8) as `type`. FLOAT1 u;
/// FLOAT2 with `stride == 8` is one `rep movsd` of `2n` dwords without the `n` test, else u, v; FLOAT3
/// u, v, 0; FLOAT4 u, v, 0, 1.0f. D3DCOLOR {0, rnd(v*255), rnd(u*255), 0xFF}; UBYTE4 {fist u, fist v, 0,
/// 1}; SHORT2 / SHORT4 {u, v[, 0, 1]}; UBYTE4N {rnd u, rnd v, 0, 0xFF}; SHORT*N / USHORT*N {u, v[, 0,
/// 0x7FFF / 0xFFFF]}; UDEC3 / DEC3N `(v&0x3FF) << 10 | (u&0x3FF)`. Returns the element size.
#[hook(addr = 0x7544E0, conv = cdecl, name = "_rpD3D9VertexDeclarationInstV2d", expect = "53 55 8B 6C 24 0C",
       route = "L2", hazards = "heap-writes",
       note = "writes into a locked vertex buffer; reuses its argument slots as temporaries")]
pub fn inst_v2d(ty: u32, dst: u32, src: u32, n: i32, stride: u32) -> u32 {
    if ty > 0xE {
        return decl_size(ty);
    }
    if ty == 1 && stride == 8 {
        let cnt = (n as u32).wrapping_mul(2);
        let mut i = 0u32;
        while i < cnt {
            w32(dst.wrapping_add(4 * i), r32(src.wrapping_add(4 * i)));
            i += 1;
        }
        return decl_size(ty);
    }
    let (mut d, mut s) = (dst, src);
    for _ in 0..n.max(0) {
        let (u, v) = (rf(s), rf(s + 4));
        match ty {
            0 => w32(d, r32(s)),
            1..=3 => {
                w32(d, r32(s));
                w32(d + 4, r32(s + 4));
                if ty >= 2 {
                    w32(d + 8, 0);
                }
                if ty == 3 {
                    w32(d + 12, F_ONE);
                }
            }
            4 => {
                w8(d + 2, rnd(u, F_255) as u8);
                let vi = rnd(v, F_255);
                w8(d, 0);
                w8(d + 1, vi as u8);
                w8(d + 3, 0xFF);
            }
            5 | 8 => {
                w8(d, conv(ty, u) as u8);
                let vi = conv(ty, v);
                w8(d + 2, 0);
                w8(d + 1, vi as u8);
                w8(d + 3, if ty == 5 { 1 } else { 0xFF });
            }
            6 | 9 | 0xB => {
                w16(d, conv(ty, u) as u16);
                w16(d + 2, conv(ty, v) as u16);
            }
            7 | 0xA | 0xC => {
                w16(d, conv(ty, u) as u16);
                let vi = conv(ty, v);
                w16(d + 4, 0);
                w16(d + 2, vi as u16);
                w16(
                    d + 6,
                    match ty {
                        7 => 1,
                        0xA => 0x7FFF,
                        _ => 0xFFFF,
                    },
                );
            }
            _ => {
                // 0xD UDEC3 / 0xE DEC3N
                let ui = conv(ty, u) as u32 & 0x3FF;
                let vi = conv(ty, v) as u32 & 0x3FF;
                w32(d, (vi << 10) | ui);
            }
        }
        d = d.wrapping_add(stride);
        s = s.wrapping_add(8);
    }
    decl_size(ty)
}

/// `_rpD3D9VertexDeclarationInstColor` (0x754AE0): writes `n` RwRGBA as D3DCOLOR dwords
/// (`a << 24 | r << 16 | g << 8 | b`) and ANDs the alphas; returns 1 when some alpha is below 0xFF (0 for
/// `n <= 0`). No type argument.
#[hook(addr = 0x754AE0, conv = cdecl, name = "_rpD3D9VertexDeclarationInstColor", expect = "56 8B 74 24 08 57",
       route = "L2", hazards = "heap-writes",
       note = "writes into a locked vertex buffer")]
pub fn inst_color(dst: u32, src: u32, n: i32, stride: u32) -> u32 {
    let mut alpha = 0xFFu8;
    let (mut d, mut s) = (dst, src);
    for _ in 0..n.max(0) {
        let v = (((((u32::from(r8(s + 3)) << 8) | u32::from(r8(s))) << 8) | u32::from(r8(s + 1))) << 8)
            | u32::from(r8(s + 2));
        w32(d, v);
        alpha &= r8(s + 3);
        d = d.wrapping_add(stride);
        s = s.wrapping_add(4);
    }
    u32::from(alpha != 0xFF)
}

/// `_rpD3D9VertexDeclarationInstIndices` (0x754B40): writes `n` packed bone-index dwords (4 bytes each)
/// as `type` (4..=7 only): D3DCOLOR {s2, s1, s0, s3}; UBYTE4 {s0..s3}; SHORT2 u16 {s0, s1}; SHORT4 u16
/// {s0..s3}. Returns the element size of `type`.
#[hook(addr = 0x754B40, conv = cdecl, name = "_rpD3D9VertexDeclarationInstIndices", expect = "8B 44 24 0C 55 8B 6C 24 08",
       unverified = "rw5-inst-1", route = "L2", hazards = "heap-writes",
       note = "writes into a locked vertex buffer")]
pub fn inst_indices(ty: u32, dst: u32, src: u32, n: i32, stride: u32) -> u32 {
    inst_indices_impl(ty, dst, src, n, stride, |v| v);
    decl_size(ty)
}

fn inst_indices_impl(ty: u32, dst: u32, src: u32, n: i32, stride: u32, map: impl Fn(u8) -> u8) {
    if ty.wrapping_sub(4) > 3 {
        return;
    }
    let (mut d, mut s) = (dst, src);
    for _ in 0..n.max(0) {
        let b = |i: u32| map(r8(s + i));
        match ty {
            4 => {
                w8(d, b(2));
                w8(d + 1, b(1));
                w8(d + 2, b(0));
                w8(d + 3, b(3));
            }
            5 => {
                for i in 0..4 {
                    w8(d + i, b(i));
                }
            }
            6 => {
                w16(d, u16::from(b(0)));
                w16(d + 2, u16::from(b(1)));
            }
            _ => {
                for i in 0..4 {
                    w16(d + 2 * i, u16::from(b(i)));
                }
            }
        }
        d = d.wrapping_add(stride);
        s = s.wrapping_add(4);
    }
}

/// `_rpD3D9VertexDeclarationInstIndicesRemap` (0x754C80): as [`inst_indices`] with every index byte
/// mapped through `remap[]` (`remap[s_i]`). Returns the element size of `type`.
#[hook(addr = 0x754C80, conv = cdecl, name = "_rpD3D9VertexDeclarationInstIndicesRemap", expect = "8B 54 24 04 8B 44 24 0C",
       route = "L2", hazards = "heap-writes",
       note = "writes into a locked vertex buffer")]
pub fn inst_indices_remap(ty: u32, dst: u32, src: u32, remap: u32, n: i32, stride: u32) -> u32 {
    inst_indices_impl(ty, dst, src, n, stride, |v| r8(remap.wrapping_add(u32::from(v))));
    decl_size(ty)
}

/// `_rpD3D9VertexDeclarationInstTangent` (0x754E20): per-vertex tangents from the positions `pos` and
/// the texture coordinates `uv`, written with `InstV3d(type, dst, T, hdr->totalNumVertex, stride)`.
/// 1. `T = rwmalloc(nv*12, 0x1050D)` (no NULL check), zero-filled.
/// 2. `hdr->indexBuffer->Lock(0, totalNumIndex*2, &idx, 0)` (result ignored; on failure the original
///    reads its uninitialised `idx` local, the port uses NULL).
/// 3. For each of `hdr->numMeshes` instances (do-while: 0 loops 2^32 times): `cnt = numIndex` (minus 2
///    when primType is 5); for `k < cnt` (signed): list (primType 4, re-read per triangle):
///    `A, B, C = idx[start+k..k+2] + minVert`, `k += 3`; else strip: even k `A, B, C = [k], [k+1],
///    [k+2]`, odd k `[k+2], [k+1], [k]` (+ minVert), `k += 1`, degenerate triangles skipped. The x87
///    sequence of the bytes: `e1x, dv1, e2x, dv2` stored as f32 (`du1, du2` stay in registers),
///    `det = du1*dv2 - du2*dv1f`; x: `sx = f32(e2x*dv1 - dv2*e1x)`, if `|det| > 1e-12` (C0|C3 clear):
///    `r = (1/det)*sx`, `T[A,B,C].x -= r`; y and z: `e1, e2` in registers, `detf = f32(det)`,
///    `s = e2*dv1 - dv2*e1`, if `|detf| > 1e-12`: `r = s*(1/detf)`, subtracted likewise.
/// 4. Unlock; normalise every tangent with `_rwInvSqrt(f32(y*y + x*x + z*z))` (no zero guard).
/// 5. `res = InstV3d(...)`, `rwfree(T)`, return `res`.
#[hook(addr = 0x754E20, conv = cdecl, name = "_rpD3D9VertexDeclarationInstTangent", expect = "83 EC 50 53 55 8B 6C 24 6C",
       unverified = "rw5-inst-1", route = "L2", hazards = "heap-writes,uninit-locals",
       note = "locks/unlocks the header's index buffer; rwmalloc/rwfree of the tangent array; a failed Lock reads an uninitialised pointer in the original (port: NULL)")]
pub fn inst_tangent(ty: u32, dst: u32, pos: u32, uv: u32, hdr: u32, stride: u32) -> u32 {
    let nv = r32(hdr + 0x3C);
    let size = nv.wrapping_mul(12);
    let t = rw_malloc(size, 0x1_050D).addr() as u32;
    for i in 0..size >> 2 {
        w32(t.wrapping_add(4 * i), 0);
    }
    for i in size & !3..size {
        w8(t.wrapping_add(i), 0);
    }
    let mut idx: u32 = 0;
    let ib = p8(r32(hdr + 8));
    ib_lock(ib, 0, r32(hdr + 0x38).wrapping_mul(2), GPtr::from_raw(&mut idx), 0);

    let mut meshes = r32(hdr + 4);
    let mut inst = hdr + 0x40;
    loop {
        let base = idx.wrapping_add(r32(inst + 0x1C).wrapping_mul(2));
        let mut cnt = r32(inst) as i32;
        if r32(hdr + 0xC) == 5 {
            cnt = cnt.wrapping_sub(2);
        }
        let mut k: i32 = 0;
        if cnt > 0 {
            let mut p1 = base.wrapping_add(2);
            let mut p2 = base.wrapping_add(4);
            loop {
                let mv = r32(inst + 4);
                let ix = |p: u32| u32::from(r16(p)).wrapping_add(mv);
                let pk = base.wrapping_add((k as u32).wrapping_mul(2));
                let (a, b, c);
                let mut skip = false;
                if r32(hdr + 0xC) == 4 {
                    a = ix(pk);
                    c = ix(p2);
                    b = ix(p1);
                    k = k.wrapping_add(3);
                    p1 = p1.wrapping_add(6);
                    p2 = p2.wrapping_add(6);
                } else {
                    if k & 1 != 0 {
                        b = ix(p1);
                        a = ix(p2);
                        c = ix(pk);
                    } else {
                        a = ix(pk);
                        b = ix(p1);
                        c = ix(p2);
                    }
                    p1 = p1.wrapping_add(2);
                    k = k.wrapping_add(1);
                    p2 = p2.wrapping_add(2);
                    skip = a == b || a == c || b == c;
                }
                if !skip {
                    tangent_triangle(t, pos, uv, a, b, c);
                }
                if k >= cnt {
                    break;
                }
            }
        }
        inst = inst.wrapping_add(0x24);
        meshes = meshes.wrapping_sub(1);
        if meshes == 0 {
            break;
        }
    }
    ib_unlock(p8(r32(hdr + 8)));

    let nvi = nv as i32;
    let mut e = t;
    for _ in 0..nvi.max(0) {
        let (vx, vy, vz) = (x(rf(e)), x(rf(e + 4)), x(rf(e + 8)));
        let len2 = vy.fmul(vy).fadd(vx.fmul(vx)).fadd(vz.fmul(vz)).to_f32();
        let inv = X87::from_f64_exact(rw_inv_sqrt(len2));
        wf(e, inv.fmul_m(rf(e)).to_f32());
        wf(e + 4, x(rf(e + 4)).fmul(inv).to_f32());
        wf(e + 8, x(rf(e + 8)).fmul(inv).to_f32());
        e = e.wrapping_add(0xC);
    }
    let res = inst_v3d_call(ty, dst, t, nvi, stride);
    rw_free(p8(t));
    res
}

/// One triangle of [`inst_tangent`] (vertex indices `a`, `b`, `c`), in the x87 order of 0x754F90..0x755121.
fn tangent_triangle(t: u32, pos: u32, uv: u32, a: u32, b: u32, c: u32) {
    let pa = pos.wrapping_add(a.wrapping_mul(12));
    let pb = pos.wrapping_add(b.wrapping_mul(12));
    let pc = pos.wrapping_add(c.wrapping_mul(12));
    let ua = uv.wrapping_add(a.wrapping_mul(8));
    let ub = uv.wrapping_add(b.wrapping_mul(8));
    let uc = uv.wrapping_add(c.wrapping_mul(8));
    let ta = t.wrapping_add(a.wrapping_mul(12));
    let tb = t.wrapping_add(b.wrapping_mul(12));
    let tc = t.wrapping_add(c.wrapping_mul(12));
    let eps = x(kf(F_TANGENT_EPS));

    let e1x = x(rf(pb)).fsub_m(rf(pa)).to_f32();
    let du1 = x(rf(ub)).fsub_m(rf(ua));
    let dv1 = x(rf(ub + 4)).fsub_m(rf(ua + 4)).to_f32();
    let e2x = x(rf(pc)).fsub_m(rf(pa)).to_f32();
    let du2 = x(rf(uc)).fsub_m(rf(ua));
    let dv2r = x(rf(uc + 4)).fsub_m(rf(ua + 4));
    let dv2 = dv2r.to_f32(); // fst (ST0 keeps the register value)
    let det = du1.fmul(dv2r).fsub(du2.fmul_m(dv1));
    let sx = x(e2x).fmul_m(dv1).fsub(x(dv2).fmul_m(e1x)).to_f32();
    let sub3 = |o: u32, r: X87| {
        wf(ta + o, x(rf(ta + o)).fsub(r).to_f32());
        wf(tb + o, x(rf(tb + o)).fsub(r).to_f32());
        wf(tc + o, x(rf(tc + o)).fsub(r).to_f32());
    };
    // `fabs; fcomp [1e-12]; and eax, 0x4100; jne skip`: only Greater goes on
    if !det.fabs().fcom(eps).test_any(0x41) {
        let r = det.fdivr_m(kf(F_ONE)).fmul_m(sx);
        sub3(0, r);
    }
    for o in [4u32, 8] {
        let e1 = x(rf(pb + o)).fsub_m(rf(pa + o));
        let e2 = x(rf(pc + o)).fsub_m(rf(pa + o));
        let detf = det.to_f32();
        let s = e2.fmul_m(dv1).fsub(x(dv2).fmul(e1));
        if !x(detf).fabs().fcom(eps).test_any(0x41) {
            let r = s.fmul(x(kf(F_ONE)).fdiv_m(detf));
            sub3(o, r);
        }
    }
}

/// `_rpD3D9VertexDeclarationUnInstV3d` (0x7551F0): decodes one element of `type` at `src` into the
/// RwV3d `dst`. FLOAT1 (x, 0, 0) (y and z stored first); FLOAT2 (x, y, 0); FLOAT3 / FLOAT4 (x, y, z)
/// (dword copies); D3DCOLOR (s[2], s[1], s[0]) * 1/255; UBYTE4 (s[0], s[1], s[2]); SHORT2 / SHORT4 i16
/// (z = 0 for SHORT2); UBYTE4N s[0..2] * 1/255; SHORT2N / SHORT4N i16 * 1/32767; USHORT2N / USHORT4N
/// SIGNED (`movsx`, kept) * 1/65535; UDEC3 the three 10-bit fields; DEC3N the fields sign-extended from
/// bit 9 * 1/511; other types (0, 0, 0). Every product is `fild; fmul dword; fstp dword`.
#[hook(addr = 0x7551F0, conv = cdecl, name = "_rpD3D9VertexDeclarationUnInstV3d", expect = "8B 44 24 04 83 EC 08",
       unverified = "rw5-inst-1", route = "L2", hazards = "heap-writes",
       note = "reads a locked vertex buffer, writes an RwV3d")]
pub fn uninst_v3d(ty: u32, dst: u32, src: u32) {
    let sc = |v: i32, k: u32| X87::from_i32(v).fmul_m(kf(k)).to_f32();
    let il = |v: i32| X87::from_i32(v).to_f32();
    let s16 = |o: u32| i32::from(r16(src + o) as i16);
    let b = |o: u32| i32::from(r8(src + o));
    match ty {
        0 => {
            let v = r32(src);
            w32(dst + 4, 0);
            w32(dst + 8, 0);
            w32(dst, v);
        }
        1 => {
            w32(dst, r32(src));
            w32(dst + 4, r32(src + 4));
            w32(dst + 8, 0);
        }
        2 | 3 => {
            w32(dst, r32(src));
            w32(dst + 4, r32(src + 4));
            w32(dst + 8, r32(src + 8));
        }
        4 => {
            wf(dst, sc(b(2), F_INV_255));
            wf(dst + 4, sc(b(1), F_INV_255));
            wf(dst + 8, sc(b(0), F_INV_255));
        }
        5 => {
            wf(dst, il(b(0)));
            wf(dst + 4, il(b(1)));
            wf(dst + 8, il(b(2)));
        }
        6 => {
            wf(dst, il(s16(0)));
            w32(dst + 8, 0);
            wf(dst + 4, il(s16(2)));
        }
        7 => {
            wf(dst, il(s16(0)));
            wf(dst + 4, il(s16(2)));
            wf(dst + 8, il(s16(4)));
        }
        8 => {
            wf(dst, sc(b(0), F_INV_255));
            wf(dst + 4, sc(b(1), F_INV_255));
            wf(dst + 8, sc(b(2), F_INV_255));
        }
        9 | 0xB => {
            let k = if ty == 9 { F_INV_32767 } else { F_INV_65535 };
            wf(dst, sc(s16(0), k));
            w32(dst + 8, 0);
            wf(dst + 4, sc(s16(2), k));
        }
        0xA | 0xC => {
            let k = if ty == 0xA { F_INV_32767 } else { F_INV_65535 };
            wf(dst, sc(s16(0), k));
            wf(dst + 4, sc(s16(2), k));
            wf(dst + 8, sc(s16(4), k));
        }
        0xD => {
            let v = r32(src);
            wf(dst, il((v & 0x3FF) as i32));
            wf(dst + 4, il(((v >> 10) & 0x3FF) as i32));
            wf(dst + 8, il(((v >> 20) & 0x3FF) as i32));
        }
        0xE => {
            let v = r32(src);
            wf(dst, sc(sext10(v), F_INV_511));
            wf(dst + 4, sc(sext10(v >> 10), F_INV_511));
            wf(dst + 8, sc(sext10(v >> 20), F_INV_511));
        }
        _ => {
            w32(dst, 0);
            w32(dst + 4, 0);
            w32(dst + 8, 0);
        }
    }
}

/// The low 10 bits of `v`, sign-extended from bit 9 (`and 0x3FF; test ch, 2; or 0xFFFFFC00`).
#[inline]
fn sext10(v: u32) -> i32 {
    let f = v & 0x3FF;
    (if f & 0x200 != 0 { f | 0xFFFF_FC00 } else { f }) as i32
}

/// `_rpD3D9VertexDeclarationUnInstV2d` (0x7555E0): decodes one element of `type` at `src` into the RwV2d
/// `dst`: FLOAT1 (x, 0); FLOAT2..4 (x, y); D3DCOLOR (s[2], s[1]) * 1/255; UBYTE4 (s[0], s[1]); SHORT2 /
/// SHORT4 i16; UBYTE4N (s[0], s[1]) * 1/255; SHORT*N i16 * 1/32767; USHORT*N signed (`movsx`, kept) *
/// 1/65535; UDEC3 the low two 10-bit fields; DEC3N sign-extended * 1/511; other types (0, 0).
#[hook(addr = 0x7555E0, conv = cdecl, name = "_rpD3D9VertexDeclarationUnInstV2d", expect = "8B 44 24 04 83 EC 08",
       unverified = "rw5-inst-1", route = "L2", hazards = "heap-writes",
       note = "reads a locked vertex buffer, writes an RwV2d")]
pub fn uninst_v2d(ty: u32, dst: u32, src: u32) {
    let sc = |v: i32, k: u32| X87::from_i32(v).fmul_m(kf(k)).to_f32();
    let il = |v: i32| X87::from_i32(v).to_f32();
    let s16 = |o: u32| i32::from(r16(src + o) as i16);
    let b = |o: u32| i32::from(r8(src + o));
    match ty {
        0 => {
            w32(dst, r32(src));
            w32(dst + 4, 0);
        }
        1..=3 => {
            w32(dst, r32(src));
            w32(dst + 4, r32(src + 4));
        }
        4 => {
            wf(dst, sc(b(2), F_INV_255));
            wf(dst + 4, sc(b(1), F_INV_255));
        }
        5 => {
            wf(dst, il(b(0)));
            wf(dst + 4, il(b(1)));
        }
        6 | 7 => {
            wf(dst, il(s16(0)));
            wf(dst + 4, il(s16(2)));
        }
        8 => {
            wf(dst, sc(b(0), F_INV_255));
            wf(dst + 4, sc(b(1), F_INV_255));
        }
        9..=0xC => {
            let k = if ty <= 0xA { F_INV_32767 } else { F_INV_65535 };
            wf(dst, sc(s16(0), k));
            wf(dst + 4, sc(s16(2), k));
        }
        0xD => {
            let v = r32(src);
            wf(dst, il((v & 0x3FF) as i32));
            wf(dst + 4, il(((v >> 10) & 0x3FF) as i32));
        }
        0xE => {
            let v = r32(src);
            wf(dst, sc(sext10(v), F_INV_511));
            wf(dst + 4, sc(sext10(v >> 10), F_INV_511));
        }
        _ => {
            w32(dst, 0);
            w32(dst + 4, 0);
        }
    }
}

/// One axis of the min/max scan: `if !(min <= v) min = v` (`and 0x4100` = C0|C3 clear: Greater only)
/// `else if (max < v) max = v` (`test ah, 5; jp`: Less only). Values are copied as dwords.
fn min_max_axis(min: GPtr<u8>, max: GPtr<u8>, v: GPtr<u8>) {
    let vb = rd::<u32>(v, 0);
    let vv = x(f32::from_bits(vb));
    if !x(f32::from_bits(rd::<u32>(min, 0))).fcom(vv).test_any(0x41) {
        wr::<u32>(min, 0, vb);
    } else if !x(f32::from_bits(rd::<u32>(max, 0))).fcom(vv).test_parity(0x05) {
        wr::<u32>(max, 0, vb);
    }
}

/// Shared body of the two min/max scans (`dims` components, element stride `4*dims`).
fn min_max(v: GPtr<u8>, n: i32, min: GPtr<u8>, max: GPtr<u8>, dims: isize) {
    for o in 0..dims {
        wr::<u32>(min, 4 * o, rd::<u32>(v, 4 * o));
    }
    for o in 0..dims {
        wr::<u32>(max, 4 * o, rd::<u32>(v, 4 * o));
    }
    if n > 1 {
        let mut p = v;
        for _ in 0..n - 1 {
            for o in 0..dims {
                min_max_axis(min.byte_add(4 * o), max.byte_add(4 * o), p.byte_add(4 * o));
            }
            p = p.byte_add(4 * dims);
        }
    }
}

/// `_rpD3D9GetMinMaxValuesV3d` (0x755830): `*min = *max = v[0]` (dword copies), then for `n > 1`,
/// `n - 1` iterations starting AT `v[0]` (so `v[0..n-2]` are examined and the last vector never is;
/// original bug kept, the spec's "i = 1..n-1" is wrong), per axis x, y, z: a value that lowers `min` is
/// not tested against `max`; NaNs never update.
#[hook(addr = 0x755830, conv = cdecl, name = "_rpD3D9GetMinMaxValuesV3d", expect = "8B 4C 24 04 8B 54 24 0C",
       unverified = "rw5-inst-1", route = "L2", hazards = "none")]
pub fn get_min_max_values_v3d(v: GPtr<u8>, n: i32, min: GPtr<u8>, max: GPtr<u8>) {
    min_max(v, n, min, max, 3);
}

/// `_rpD3D9GetMinMaxValuesV2d` (0x7558F0): as [`get_min_max_values_v3d`] for RwV2d (x, y; stride 8;
/// the last element is never examined).
#[hook(addr = 0x7558F0, conv = cdecl, name = "_rpD3D9GetMinMaxValuesV2d", expect = "8B 4C 24 04 8B 54 24 0C",
       route = "L2", hazards = "none")]
pub fn get_min_max_values_v2d(uv: GPtr<u8>, n: i32, min: GPtr<u8>, max: GPtr<u8>) {
    min_max(uv, n, min, max, 2);
}

/// `test ah, 0x41; jp`: Greater or Unordered.
#[inline]
fn gt_or_nan(v: u32, k: u32) -> bool {
    x(f32::from_bits(v)).fcom(x(kf(k))).test_parity(0x41)
}

/// `and eax, 0x100; jne`: Less or Unordered.
#[inline]
fn lt_or_nan(v: u32, k: u32) -> bool {
    x(f32::from_bits(v)).fcom(x(kf(k))).c0()
}

/// `_rpD3D9FindFormatV3d` (0x755980): the D3DDECLTYPE for `n` RwV3d from their range (min/max through
/// 0x755830 on stack locals): any `max.k > 1.0` or NaN -> 2 (FLOAT3); all `min.k >= 0` (not NaN):
/// DeclTypes (`[0xC9BFEC]` low byte) bit 7 (DEC3N) -> 0xE, else bit 5 (USHORT4N) -> 0xC, else 2; else all
/// `min.k >= -1` (not NaN): DEC3N -> 0xE, else bit 3 (SHORT4N) -> 0xA, else 2; else 2.
#[hook(addr = 0x755980, conv = cdecl, name = "_rpD3D9FindFormatV3d", expect = "8B 54 24 08 83 EC 18",
       unverified = "rw5-inst-1", route = "L2", hazards = "none")]
pub fn find_format_v3d(v: GPtr<u8>, n: i32) -> u32 {
    let mut min = [0u32; 3];
    let mut max = [0u32; 3];
    min_max_v3d_call(v, n, GPtr::<u32>::from_raw(&mut min[0]).cast(), GPtr::<u32>::from_raw(&mut max[0]).cast());
    if max.iter().any(|&m| gt_or_nan(m, F_ONE)) {
        return 2;
    }
    let caps = g::<u8>(CAPS_DECL_TYPES).read();
    if !min.iter().any(|&m| lt_or_nan(m, F_ZERO)) {
        return if caps & 0x80 != 0 {
            0xE
        } else if caps & 0x20 != 0 {
            0xC
        } else {
            2
        };
    }
    if min.iter().any(|&m| lt_or_nan(m, F_MINUS_ONE)) {
        return 2;
    }
    if caps & 0x80 != 0 {
        0xE
    } else if caps & 0x08 != 0 {
        0xA
    } else {
        2
    }
}

/// `_rpD3D9FindFormatV2d` (0x755AA0): as [`find_format_v3d`] for `n` RwV2d (default 1 FLOAT2): any
/// `max.k > 1.0` or NaN -> 1; all `min.k >= 0`: DeclTypes bit 4 (USHORT2N) -> 0xB, else bit 7 -> 0xE
/// else 4 (D3DCOLOR); else all `min.k >= -1`: bit 2 (SHORT2N) -> 9, else bit 7 -> 0xE, else 1; else 1.
#[hook(addr = 0x755AA0, conv = cdecl, name = "_rpD3D9FindFormatV2d", expect = "8B 54 24 08 83 EC 10",
       route = "L2", hazards = "none")]
pub fn find_format_v2d(uv: GPtr<u8>, n: i32) -> u32 {
    let mut min = [0u32; 2];
    let mut max = [0u32; 2];
    min_max_v2d_call(uv, n, GPtr::<u32>::from_raw(&mut min[0]).cast(), GPtr::<u32>::from_raw(&mut max[0]).cast());
    if max.iter().any(|&m| gt_or_nan(m, F_ONE)) {
        return 1;
    }
    let caps = g::<u8>(CAPS_DECL_TYPES).read();
    if !min.iter().any(|&m| lt_or_nan(m, F_ZERO)) {
        return if caps & 0x10 != 0 {
            0xB
        } else if caps & 0x80 != 0 {
            0xE
        } else {
            4
        };
    }
    if min.iter().any(|&m| lt_or_nan(m, F_MINUS_ONE)) {
        return 1;
    }
    if caps & 0x04 != 0 {
        9
    } else if caps & 0x80 != 0 {
        0xE
    } else {
        1
    }
}

orig! {
    /// `RpD3D9GeometryGetUsageFlags(geometry)` (0x7588D0)
    pub fn geometry_get_usage_flags_call(g: GPtr<u8>) -> u32 = cdecl 0x7588D0 => crate::d3d9misc::geometry_get_usage_flags;
    /// `RwD3D9DestroyVertexBuffer(stride, size, vb, offset)` (0x7F56A0)
    pub fn destroy_vertex_buffer_call(stride: u32, size: u32, vb: u32, offset: u32) = cdecl 0x7F56A0 => crate::d3d9vb::destroy_vertex_buffer;
    /// `RwD3D9DynamicVertexBufferDestroy(vb)` (0x7F5AE0)
    pub fn dynamic_vertex_buffer_destroy_call(vb: u32) = cdecl 0x7F5AE0 => crate::d3d9vb::dynamic_vertex_buffer_destroy;
    /// `RwD3D9CreateVertexBuffer(stride, size, &vb, &offset)` (0x7F5500)
    pub fn create_vertex_buffer_call(stride: u32, size: u32, vb: GPtr<u32>, offset: GPtr<u32>) -> u32 = cdecl 0x7F5500 => crate::d3d9vb::create_vertex_buffer;
    /// `RwD3D9DynamicVertexBufferCreate(size, &vb)` (0x7F5A00)
    pub fn dynamic_vertex_buffer_create_call(size: u32, vb: GPtr<u32>) -> u32 = cdecl 0x7F5A00 => crate::d3d9vb::dynamic_vertex_buffer_create;
    /// `_rpD3D9AddDynamicGeometry(geometry)` (0x74E970)
    pub fn add_dynamic_geometry_call(g: u32) = cdecl 0x74E970 => crate::d3d9misc::add_dynamic_geometry;
    /// CRT `qsort(base, n, size, cmp)` (0x8247E0)
    pub fn crt_qsort(base: GPtr<u8>, n: u32, size: u32, cmp: u32) = cdecl 0x8247E0 => unported;
    /// `RwD3D9CreateVertexDeclaration(elements, &decl)` (0x7FAA30)
    pub fn create_vertex_declaration_call(e: GPtr<u8>, out: GPtr<u32>) -> u32 = cdecl 0x7FAA30 => crate::d3d9dev::create_vertex_declaration;
    /// `IDirect3DVertexDeclaration9::GetDeclaration(this, elements, &n)` (vtbl +0x10)
    pub fn decl_get_declaration(d: GPtr<u8>, e: GPtr<u8>, n: GPtr<u32>) -> i32 = stdcall vslot 0x10 => unported;
    /// `IDirect3DVertexBuffer9::Lock(this, offset, size, &data, flags)` (vtbl +0x2C)
    pub fn vb_lock(vb: GPtr<u8>, offset: u32, size: u32, data: GPtr<u32>, flags: u32) -> i32 = stdcall vslot 0x2C => unported;
    /// `IDirect3DVertexBuffer9::Unlock(this)` (vtbl +0x30)
    pub fn vb_unlock(vb: GPtr<u8>) -> i32 = stdcall vslot 0x30 => unported;
    /// `_rpD3D9VertexDeclarationInstV3dMorph` (0x753B60)
    pub fn inst_v3d_morph_call(ty: u32, dst: u32, a: u32, b: u32, t: f32, n: i32, stride: u32) -> u32 = cdecl 0x753B60 => inst_v3d_morph;
    /// `_rpD3D9VertexDeclarationInstV2d` (0x7544E0)
    pub fn inst_v2d_call(ty: u32, dst: u32, src: u32, n: i32, stride: u32) -> u32 = cdecl 0x7544E0 => inst_v2d;
    /// `_rpD3D9VertexDeclarationInstColor` (0x754AE0)
    pub fn inst_color_call(dst: u32, src: u32, n: i32, stride: u32) -> u32 = cdecl 0x754AE0 => inst_color;
    /// `_rpD3D9VertexDeclarationInstTangent` (0x754E20)
    pub fn inst_tangent_call(ty: u32, dst: u32, pos: u32, uv: u32, hdr: u32, stride: u32) -> u32 = cdecl 0x754E20 => inst_tangent;
    /// `_rxD3D9DefaultInstanceCallback(object, header, reinstance)` (0x7578C0)
    pub fn default_instance_callback_call(object: u32, hdr: u32, reinstance: u32) -> u32 = cdecl 0x7578C0 => default_instance_callback;
    /// The pipeline's instance callback, through its stored pointer
    pub fn call_instance_cb(f: u32, object: u32, hdr: u32, reinstance: u32) -> u32 = cdecl dyn => unported;
}

/// `D3DCAPS9.DevCaps` (0x10000 = HWTRANSFORMANDLIGHT) and `DevCaps2` (1 = STREAMOFFSET) of the caps copy.
const CAPS_DEV_CAPS: usize = 0xC9_BF1C;
const CAPS_DEV_CAPS2: usize = 0xC9_BFD4;
/// `_rxD3D9IndexCompare` (qsort comparator on D3DVERTEXELEMENT9: Stream, then Offset).
const INDEX_COMPARE: u32 = 0x75_8240;

/// One D3DVERTEXELEMENT9 {Stream, Offset, Type, Method 0, Usage, UsageIndex} at element `i` of `decl`.
fn put_elem(decl: GPtr<u8>, i: u32, stream: u32, offset: u16, ty: u8, usage: u8, index: u8) {
    let e = decl.byte_add(8 * i as isize);
    wr::<u16>(e, 0, stream as u16);
    wr::<u16>(e, 2, offset);
    wr::<u8>(e, 4, ty);
    wr::<u8>(e, 5, 0);
    wr::<u8>(e, 6, usage);
    wr::<u8>(e, 7, index);
}

/// The first element with `Usage == usage` and `UsageIndex == index` (unbounded search, as the original).
fn find_elem(decl: GPtr<u8>, usage: u8, index: u32) -> GPtr<u8> {
    let mut e = decl;
    loop {
        if rd::<u8>(e, 6) == usage && u32::from(rd::<u8>(e, 7)) == index {
            return e;
        }
        e = e.byte_add(8);
    }
}

/// Runs `f(inst)` for every RxD3D9InstanceData of `h` in the original's do-while (`numMeshes` 0 loops
/// 2^32 times).
fn for_each_instance(h: u32, mut f: impl FnMut(u32)) {
    let mut cnt = r32(h + 4);
    let mut inst = h + 0x40;
    loop {
        f(inst);
        inst = inst.wrapping_add(0x24);
        cnt = cnt.wrapping_sub(1);
        if cnt == 0 {
            break;
        }
    }
}

/// `_rxD3D9DefaultInstanceCallback` (0x7578C0): (re)instances the geometry of `object` (an RpAtomic)
/// into the vertex streams of the resource-entry header `h` (`RxD3D9ResEntryHeader`, stream `s` at
/// `h + 0x10 + 0x10*s`: vb, offset, stride, u16 geometryFlags, u8 managed, u8 dynamicLock).
/// - First instance (`reinstance == 0`): `totalNumVertex = numVertices`; both streams' buffers released
///   (managed: `RwD3D9DestroyVertexBuffer(stride, stride*tnv, vb, offset)`; offset 0 and not
///   dynamic-locked: `RwD3D9DynamicVertexBufferDestroy`; otherwise only forgotten) and the stream fields
///   cleared. Dynamic = (numMorphTargets > 1 or `usage & 0xFFF`) and hardware T&L and the pipeline is
///   not plugin 0x12E: then (morph: `usage |= 6`) two streams and `_rpD3D9AddDynamicGeometry`. An
///   attribute goes to stream 0 when its usage bit is set, else to the last stream. Elements (Offset =
///   the stream's current stride): POSITION (FLOAT3, stride SET to 0xC, flags SET to 2), NORMAL
///   (geometry flag 0x10; DEC3N +4 if DeclTypes has DEC3N else FLOAT3 +0xC; flags |= 4), COLOR (flag 8;
///   D3DCOLOR +4; |= 8), TEXCOORD i (FLOAT2 +8; |= 0x10 << i), TANGENT (usage 0x10000 with texcoords;
///   stream from `usage & 0xFF2`; DEC3N/FLOAT3; flags untouched). Trailing empty streams dropped;
///   `qsort` by stream/offset; D3DDECL_END; `RwD3D9CreateVertexDeclaration` (result ignored); per stream
///   a managed VB (`RwD3D9CreateVertexBuffer`; also stream 1 of a dynamic pair with STREAMOFFSET) or a
///   dynamic one; a failure returns 0. Two streams: `useOffsets = 1` only with STREAMOFFSET (else
///   untouched), `baseIndex = minVert`; one stream: `useOffsets = 0`,
///   `baseIndex = offset / stride + minVert` (a zero stride faults in both).
/// - Reinstance: `GetDeclaration` of the stored declaration; numStreams = 1 + the largest Stream.
/// - Lock: first instance every stream (`0x800`) with lockMask 0xFFF; reinstance lockMask =
///   `lockedSinceLastInst`, a stream only when the mask is 0xFFF or meets its flags (managed 0x800;
///   dynamic 0x2800 and `lockMask |= flags`).
/// - Positions / normals: morph targets `start/end` (i16; both 0 if either is out of range),
///   `t = f32(position * recipTime)`; `start != end` and `t != 0` (NaN counts): `InstV3dMorph` for
///   POSITION and (flag 0x10) NORMAL whatever the mask; else `InstV3d` of morph target `start` when the
///   mask has 2 / 4.
/// - Prelight (flag 8): mask 8: per instance `vertexAlpha = InstColor(stream + minVert*stride + offset,
///   preLitLum + minVert, numVertices, stride)`; not prelit and first instance: every `vertexAlpha = 0`.
/// - Texcoords (mask & 0xFF0): set i when mask has `0x10 << i`: `InstV2d`.
/// - Tangents (usage 0x10000, texcoords, mask & 0xFF2): `InstTangent` with morph target `start` and the
///   LAST texcoord set.
/// - Unlock every locked stream; return 1.
///
/// Not reproduced: the original's `lockPtr[2]` local is followed by its other locals (a declaration with
/// a Stream >= 2 would overwrite them); the port's lock array has 16 slots. A failed `GetDeclaration`
/// leaves the original's element array uninitialised (port: zeros). The element searches are
/// unbounded, as in the original.
#[hook(addr = 0x7578C0, conv = cdecl, name = "_rxD3D9DefaultInstanceCallback", expect = "55 8B EC 83 E4 F8",
       route = "split", hazards = "heap-writes,uninit-locals",
       note = "first instance creates/destroys vertex buffers and a vertex declaration (D3D objects): not repeatable; reinstance locks and rewrites the streams")]
pub fn default_instance_callback(object: u32, h: u32, reinstance: u32) -> u32 {
    let geo = r32(object + 0x18);
    let gflags = r32(geo + 8);
    let mut usage = geometry_get_usage_flags_call(p8(geo));
    let ntex = r32(geo + 0x1C);
    let mut decl_buf = [0u32; 0x40];
    let decl = GPtr::<u32>::from_raw(&mut decl_buf[0]).cast::<u8>();
    let es = |s: u32| h.wrapping_add(0x10).wrapping_add(s.wrapping_mul(0x10));
    let mut nstreams: u32;
    if reinstance == 0 {
        w32(h + 0x3C, r32(geo + 0x14));
        for s in 0..2 {
            let e = es(s);
            let vb = r32(e);
            if vb != 0 {
                if r8(e + 0xE) != 0 {
                    let stride = r32(e + 8);
                    destroy_vertex_buffer_call(stride, r32(h + 0x3C).wrapping_mul(stride), vb, r32(e + 4));
                } else if r32(e + 4) == 0 && r8(e + 0xF) == 0 {
                    dynamic_vertex_buffer_destroy_call(vb);
                }
                w32(e, 0);
            }
            w32(e + 4, 0);
            w32(e + 8, 0);
            w16(e + 0xC, 0);
            w8(e + 0xE, 0);
            w8(e + 0xF, 0);
        }
        let nm = r32(geo + 0x18) as i32;
        let pipe = r32(object + 0x6C);
        let dynamic = (nm > 1 || usage & 0xFFF != 0)
            && g::<u32>(CAPS_DEV_CAPS).read() & 0x1_0000 != 0
            && !(pipe != 0 && r32(pipe + 0x2C) == 0x12E);
        if dynamic {
            if nm > 1 {
                usage |= 6;
            }
            nstreams = 2;
            add_dynamic_geometry_call(geo);
        } else {
            nstreams = 1;
        }
        let sel = |bit: u32| if usage & bit != 0 { 0 } else { nstreams - 1 };
        let caps = g::<u8>(CAPS_DECL_TYPES).read();
        // POSITION
        let s = sel(2);
        put_elem(decl, 0, s, r16(es(s) + 8), 2, 0, 0);
        w32(es(s) + 8, 0xC);
        w16(es(s) + 0xC, 2);
        let mut n = 1u32;
        if gflags & 0x10 != 0 {
            let s = sel(4);
            let st = r32(es(s) + 8);
            let (ty, add) = if caps & 0x80 != 0 { (0xE, 4) } else { (2, 0xC) };
            put_elem(decl, 1, s, st as u16, ty, 3, 0);
            w32(es(s) + 8, st.wrapping_add(add));
            w8(es(s) + 0xC, r8(es(s) + 0xC) | 4);
            n = 2;
        }
        if gflags & 8 != 0 {
            let s = sel(8);
            put_elem(decl, n, s, r16(es(s) + 8), 4, 0xA, 0);
            n += 1;
            w8(es(s) + 0xC, r8(es(s) + 0xC) | 8);
            w32(es(s) + 8, r32(es(s) + 8).wrapping_add(4));
        }
        let mut i = 0u32;
        while i < ntex {
            let s = sel(0x10u32.wrapping_shl(i));
            put_elem(decl, n, s, r16(es(s) + 8), 1, 5, i as u8);
            n += 1;
            w32(es(s) + 8, r32(es(s) + 8).wrapping_add(8));
            w16(es(s) + 0xC, r16(es(s) + 0xC) | 0x10u32.wrapping_shl(i) as u16);
            i += 1;
        }
        if usage & 0x1_0000 != 0 && ntex != 0 {
            let s = sel(0xFF2);
            let st = r32(es(s) + 8);
            let (ty, add) = if caps & 0x80 != 0 { (0xE, 4) } else { (2, 0xC) };
            put_elem(decl, n, s, st as u16, ty, 6, 0);
            w32(es(s) + 8, st.wrapping_add(add));
            n += 1;
        }
        while nstreams > 1 && r32(es(nstreams - 1) + 8) == 0 {
            nstreams -= 1;
        }
        crt_qsort(decl, n, 8, INDEX_COMPARE);
        let end = decl.byte_add(8 * n as isize);
        wr::<u16>(end, 0, 0xFF);
        wr::<u16>(end, 2, 0);
        wr::<u8>(end, 4, 0x11);
        wr::<u8>(end, 5, 0);
        wr::<u8>(end, 6, 0);
        wr::<u8>(end, 7, 0);
        create_vertex_declaration_call(decl, p8(h + 0x34).cast());
        for s in 0..nstreams {
            let e = es(s);
            let stride = r32(e + 8);
            let size = r32(h + 0x3C).wrapping_mul(stride);
            let ok = if !dynamic || (s != 0 && g::<u8>(CAPS_DEV_CAPS2).read() & 1 != 0) {
                w8(e + 0xE, 1);
                create_vertex_buffer_call(stride, size, p8(e).cast(), p8(e + 4).cast())
            } else {
                w8(e + 0xE, 0);
                dynamic_vertex_buffer_create_call(size, p8(e).cast())
            };
            if ok == 0 {
                return 0;
            }
        }
        if nstreams > 1 {
            if g::<u8>(CAPS_DEV_CAPS2).read() & 1 != 0 {
                w32(h + 0x30, 1);
            }
            for_each_instance(h, |inst| w32(inst + 0x14, r32(inst + 4)));
        } else {
            w32(h + 0x30, 0);
            for_each_instance(h, |inst| {
                w32(inst + 0x14, (r32(h + 0x14) / r32(h + 0x18)).wrapping_add(r32(inst + 4)))
            });
        }
    } else {
        let mut cnt = 0u32;
        decl_get_declaration(p8(r32(h + 0x34)), decl, GPtr::from_raw(&mut cnt));
        nstreams = 0;
        let mut e = decl;
        while rd::<u8>(e, 4) != 0x11 {
            let st = u32::from(rd::<u16>(e, 0));
            if st >= nstreams {
                nstreams = st + 1;
            }
            e = e.byte_add(8);
        }
    }

    // Lock
    let mut lock_buf = [0u32; 16];
    let lp = GPtr::<u32>::from_raw(&mut lock_buf[0]);
    let lock_slot = |s: u32| lp.byte_add(4 * s as isize);
    let tnv = r32(h + 0x3C);
    let mut mask: u32;
    if reinstance == 0 {
        mask = 0xFFF;
        for s in 0..nstreams {
            let e = es(s);
            vb_lock(p8(r32(e)), r32(e + 4), r32(e + 8).wrapping_mul(tnv), lock_slot(s), 0x800);
        }
    } else {
        mask = u32::from(r16(geo + 0xC));
        for s in 0..nstreams {
            let e = es(s);
            if mask == 0xFFF || mask & u32::from(r16(e + 0xC)) != 0 {
                let size = r32(e + 8).wrapping_mul(tnv);
                let flags = if r8(e + 0xE) != 0 {
                    0x800
                } else {
                    mask |= u32::from(r16(e + 0xC));
                    0x2800
                };
                vb_lock(p8(r32(e)), r32(e + 4), size, lock_slot(s), flags);
            }
        }
    }
    let stream_of = |e: GPtr<u8>| u32::from(rd::<u16>(e, 0));
    let dst = |e: GPtr<u8>| lock_slot(stream_of(e)).read().wrapping_add(u32::from(rd::<u16>(e, 2)));
    let strd = |e: GPtr<u8>| r32(es(stream_of(e)) + 8);
    let ty = |e: GPtr<u8>| u32::from(rd::<u8>(e, 4));

    // Positions and normals
    let nm = r32(geo + 0x18);
    let mt = r32(geo + 0x5C);
    let target = |i: i32, off: u32| r32(mt.wrapping_add((i as u32).wrapping_mul(0x1C)).wrapping_add(off));
    let mut start: i32 = 0;
    let mut morphed = false;
    if nm != 1 {
        let mut st = i32::from(r16(object + 0x50) as i16);
        let mut en = i32::from(r16(object + 0x52) as i16);
        if st >= nm as i32 || en >= nm as i32 {
            st = 0;
            en = 0;
        }
        start = st;
        let t = x(rf(object + 0x5C)).fmul_m(rf(object + 0x58)).to_f32();
        // `fcomp [0.0]; test ah, 0x44; jnp static`: static only for Equal
        if st != en && x(t).fcom(x(kf(F_ZERO))).test_parity(0x44) {
            morphed = true;
            let e = find_elem(decl, 0, 0);
            inst_v3d_morph_call(ty(e), dst(e), target(st, 0x14), target(en, 0x14), t, tnv as i32, strd(e));
            if gflags & 0x10 != 0 {
                let e = find_elem(decl, 3, 0);
                inst_v3d_morph_call(ty(e), dst(e), target(st, 0x18), target(en, 0x18), t, tnv as i32, strd(e));
            }
        }
    }
    if !morphed {
        if mask & 2 != 0 {
            let v = target(start, 0x14);
            let e = find_elem(decl, 0, 0);
            inst_v3d_call(ty(e), dst(e), v, tnv as i32, strd(e));
        }
        if gflags & 0x10 != 0 && mask & 4 != 0 {
            let v = target(start, 0x18);
            let e = find_elem(decl, 3, 0);
            inst_v3d_call(ty(e), dst(e), v, tnv as i32, strd(e));
        }
    }

    // Prelight
    if gflags & 8 != 0 {
        if mask & 8 != 0 {
            let e = find_elem(decl, 0xA, 0);
            let pre = r32(geo + 0x30);
            let stride = strd(e);
            for_each_instance(h, |inst| {
                let mv = r32(inst + 4);
                let n = r32(inst + 0x18);
                let d = lock_slot(stream_of(e))
                    .read()
                    .wrapping_add(mv.wrapping_mul(stride))
                    .wrapping_add(u32::from(rd::<u16>(e, 2)));
                let a = inst_color_call(d, pre.wrapping_add(mv.wrapping_mul(4)), n as i32, stride);
                w32(inst + 0xC, a);
            });
        }
    } else if reinstance == 0 {
        for_each_instance(h, |inst| w32(inst + 0xC, 0));
    }

    // Texture coordinates
    if ntex != 0 && mask & 0xFF0 != 0 {
        for i in 0..ntex {
            if mask & 0x10u32.wrapping_shl(i) != 0 {
                let tc = r32(geo.wrapping_add(0x34).wrapping_add(4 * i));
                let e = find_elem(decl, 5, i);
                inst_v2d_call(ty(e), dst(e), tc, tnv as i32, strd(e));
            }
        }
    }

    // Tangents
    if usage & 0x1_0000 != 0 && ntex != 0 && mask & 0xFF2 != 0 {
        let e = find_elem(decl, 6, 0);
        let uv = r32(geo.wrapping_add(0x30).wrapping_add(ntex.wrapping_mul(4)));
        let v = target(start, 0x14);
        inst_tangent_call(ty(e), dst(e), v, uv, h, strd(e));
    }

    // Unlock
    for s in 0..nstreams {
        if lock_slot(s).read() != 0 {
            vb_unlock(p8(r32(es(s))));
        }
    }
    1
}

/// `_rxD3D9DefaultReinstanceCallback` (0x758270): with several morph targets, only when the atomic's
/// interpolator flag 1 is set or `lockedSinceLastInst != 0`: `lockedSinceLastInst |= 6` (low byte),
/// `_rxD3D9DefaultInstanceCallback(object, res + 0x18, 1)` called DIRECTLY (the stored instance callback
/// and the result are ignored), the old value restored; returns 1. One morph target: with an instance
/// callback, `instanceCb(object, res + 0x18, 1) != 0`; without one 1.
#[hook(addr = 0x758270, conv = cdecl, name = "_rxD3D9DefaultReinstanceCallback", expect = "8B 4C 24 04 57",
       route = "split", hazards = "heap-writes,uninit-locals",
       note = "the nested instance callback locks and rewrites the atomic's vertex streams; it runs 0x7578C0 directly, so a double run (l2) re-instances: split only (VERIFIER-3)")]
pub fn default_reinstance_callback(object: u32, res: u32, cb: u32) -> u32 {
    let geo = r32(object + 0x18);
    if r32(geo + 0x18) != 1 {
        if r8(object + 0x4C) & 1 != 0 || r16(geo + 0xC) != 0 {
            let saved = r16(geo + 0xC);
            w16(geo + 0xC, saved | 6);
            default_instance_callback_call(object, res.wrapping_add(0x18), 1);
            w16(geo + 0xC, saved);
        }
        return 1;
    }
    if cb == 0 {
        return 1;
    }
    u32::from(call_instance_cb(cb, object, res.wrapping_add(0x18), 1) != 0)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_INST_WEIGHTS,
    HOOK_INST_V3D,
    HOOK_INST_V3D_COMP,
    HOOK_INST_V3D_MORPH,
    HOOK_INST_V2D,
    HOOK_INST_COLOR,
    HOOK_INST_INDICES,
    HOOK_INST_INDICES_REMAP,
    HOOK_INST_TANGENT,
    HOOK_UNINST_V3D,
    HOOK_UNINST_V2D,
    HOOK_GET_MIN_MAX_VALUES_V3D,
    HOOK_GET_MIN_MAX_VALUES_V2D,
    HOOK_FIND_FORMAT_V3D,
    HOOK_FIND_FORMAT_V2D,
    HOOK_DEFAULT_INSTANCE_CALLBACK,
    HOOK_DEFAULT_REINSTANCE_CALLBACK,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
