//! The CPU skinning of the D3D9 skin node without SSE: the cached bone matrices (0x7CA850) and the x87 vertex
//! blenders for up to four (0x7C9DA0) and two (0x7CA410) weights.
//!
//! Every output component is a row of a bone matrix times the input vector, the three products added in an order
//! that differs per component and per blender (x87 extended precision, so the order is kept: `ord` below); positions
//! add the row's translation. Weighted sums keep some partial results on the x87 stack and store others as f32,
//! as the original does.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, p8, rd, wr, x};
use crate::d3d9_skin_blend::{MATRIX_BUFFER, Mat, matrix_cache_hit, matrix_cache_store};

use crate::bindings::frame_get_ltm;
use crate::bindings::rp_skin_get_skin_to_bone_matrices as skin_to_bone;
use crate::bindings::rw_matrix_invert as matrix_invert;
use crate::bindings::rw_matrix_multiply as matrix_multiply;
orig! {
    /// The x87 3x4 matrix product `out = a * b` (0x7CA6C0)
    fn matrix_multiply_x87(out: u32, a: u32, b: u32) -> u32 = cdecl 0x7CA6C0 => crate::generated::generated_setters::g_missed_by_ghidra_cat7_call_7ca6c0;
}

/// The cached bone matrices of the x87 path (0x7CA850, cdecl; `_rpSkinPrepareMatrices`-like): unless the cache
/// matches (`matrix_cache_hit`), with a hierarchy, per node `i` (the count re-read each pass): NOMATRICES (flags bit
/// 1): `skinToBone[i] * LTM(node frame)` times `inverse(LTM(atomic frame))`; LOCALSPACE (0x4000): `skinToBone[i]`
/// times `matrices[i]`; else `skinToBone[i] * matrices[i]` times the inverse; the last product each time by the x87
/// 0x7CA6C0 into the matrix buffer. Then the cache is updated (also without a hierarchy). Returns the buffer.
#[hook(addr = 0x7CA850, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "A1 D4 78 C9 00 81 EC 84 00 00 00",
       unverified = "frontier2-c25", route = "L2", hazards = "heap-writes")]
pub fn skin_prepare_matrices_x87(atomic: GPtr<u8>, skin: GPtr<u8>, hierarchy: GPtr<u8>) -> GPtr<u8> {
    if !matrix_cache_hit(atomic, hierarchy) {
        if !hierarchy.is_null() {
            let mut out = p8(g::<u32>(MATRIX_BUFFER).read());
            let mut bone = p8(skin_to_bone(skin));
            let (mut inv, mut tmp) = (Mat([0; 16]), Mat([0; 16]));
            let flags = rd::<u32>(hierarchy, 0);
            let nodes = || rd::<i32>(hierarchy, 4);
            let a = |p: GPtr<u8>| p.addr() as u32;
            let local_space = flags & 2 == 0 && flags & 0x4000 != 0;
            if !local_space {
                matrix_invert(inv.p(), frame_get_ltm(p8(rd::<u32>(atomic, 4))));
            }
            let mut i = 0;
            while i < nodes() {
                if flags & 2 != 0 {
                    let info = p8(rd::<u32>(hierarchy, 0x10)).byte_add(0x10 * i as isize);
                    matrix_multiply(tmp.p(), bone, frame_get_ltm(p8(rd::<u32>(info, 0xC))));
                    matrix_multiply_x87(a(out), a(tmp.p()), a(inv.p()));
                } else if local_space {
                    matrix_multiply_x87(a(out), a(bone), rd::<u32>(hierarchy, 8).wrapping_add(0x40 * i as u32));
                } else {
                    matrix_multiply(tmp.p(), bone, p8(rd::<u32>(hierarchy, 8)).byte_add(0x40 * i as isize));
                    matrix_multiply_x87(a(out), a(tmp.p()), a(inv.p()));
                }
                i += 1;
                bone = bone.byte_add(0x40);
                out = out.byte_add(0x40);
            }
        }
        matrix_cache_store(atomic, hierarchy);
    }
    p8(g::<u32>(MATRIX_BUFFER).read())
}

/// Row `row` of matrix `m` times the vector at `v`, the products `m[row][k] * v[k]` added in the order `ord`.
fn row(m: GPtr<u8>, r: isize, v: GPtr<u8>, ord: [isize; 3]) -> X87 {
    let t = |k: isize| x(rd::<f32>(m, 0x10 * r + 4 * k)).fmul_m(rd::<f32>(v, 4 * k));
    t(ord[0]).fadd(t(ord[1])).fadd(t(ord[2]))
}

/// [`row`] plus the row's translation (`m[row][3]`).
fn point(m: GPtr<u8>, r: isize, v: GPtr<u8>, ord: [isize; 3]) -> X87 {
    row(m, r, v, ord).fadd_m(rd::<f32>(m, 0x10 * r + 0xC))
}

/// The bone matrix of index byte `b`.
fn bone(mats: GPtr<u8>, b: u32) -> GPtr<u8> {
    mats.byte_add(((b & 0xFF) << 6) as isize)
}

fn put3(p: GPtr<u8>, v: [f32; 3]) {
    for (k, c) in v.into_iter().enumerate() {
        wr::<f32>(p, 4 * k as isize, c);
    }
}

/// `_rpSkinBlendPositions4Weights`-like x87 blender (0x7C9DA0, cdecl `(count, weights, indices, matrices, outPos,
/// inPos, outNormals, inNormals, stride)`): per vertex the weights are 4 floats, the indices 4 bytes. A first
/// weight whose bits are at least those of 1.0 (signed) takes bone 0 alone; otherwise bones 0 and 1 by their
/// weights, then bone 2 when the third weight's bits are > 0 and, only then, bone 3 when the fourth's are. The
/// normals are computed when `inNormals` is set and stored when `outNormals` is set, so with only `outNormals` the
/// stored normals are the accumulator locals as the caller's stack left them (entry ESP - 0x18, `peek`). The
/// original keeps the output pointer and the index dword in the caller's argument slots.
#[hook(addr = 0x7C9DA0, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "83 EC 1C 8B 44 24 20",
       peek = 0x18, unverified = "frontier2-c25", route = "L2", hazards = "heap-writes,stack-residue",
       note = "keeps outPos and the vertex's index dword in the caller's argument slots")]
#[allow(clippy::too_many_arguments)]
pub fn skin_blend_x87_weights4(count: u32, weights: GPtr<u8>, idx: GPtr<u8>, mats: GPtr<u8>, out: GPtr<u8>, inp: GPtr<u8>,
    out_n: GPtr<u8>, in_n: GPtr<u8>, stride: u32) {
    const P: [[isize; 3]; 3] = [[0, 2, 1], [0, 2, 1], [1, 0, 2]];
    const N: [[isize; 3]; 3] = [[1, 0, 2], [2, 1, 0], [1, 0, 2]];
    let mut nacc = [0f32; 3];
    for (k, c) in nacc.iter_mut().enumerate() {
        *c = f32::from_bits(sa_core::abi::peek_stack(0x7C9DA0, -0x18 + 4 * k as i32).unwrap_or(0));
    }
    let (mut w, mut o, mut v, mut on, mut n) = (weights, out, inp, out_n, in_n);
    let mut i = 0;
    while i < count as i32 {
        let ix = rd::<u32>(idx, 4 * i as isize);
        let m0 = bone(mats, ix);
        let x0 = point(m0, 0, v, P[0]);
        if rd::<i32>(w, 0) >= 0x3F80_0000 {
            put3(o, [f32_of(x0), f32_of(point(m0, 1, v, P[1])), f32_of(point(m0, 2, v, P[2]))]);
            v = v.byte_add(0xC);
            o = o.byte_add(stride as isize);
            if !on.is_null() {
                put3(on, [0, 1, 2].map(|r| f32_of(row(m0, r, n, N[r as usize]))));
                on = on.byte_add(stride as isize);
                n = n.byte_add(0xC);
            }
        } else {
            let w0 = rd::<f32>(w, 0);
            let x0w = x0.fmul_m(w0);
            let y0w = point(m0, 1, v, P[1]).fmul_m(w0);
            let z0w = f32_of(point(m0, 2, v, P[2]).fmul_m(w0));
            if !n.is_null() {
                nacc = [0, 1, 2].map(|r| f32_of(row(m0, r, n, N[r as usize]).fmul_m(w0)));
            }
            let m1 = bone(mats, ix >> 8);
            let w1 = rd::<f32>(w, 4);
            let mut acc = [
                f32_of(point(m1, 0, v, P[0]).fmul_m(w1).fadd(x0w)),
                f32_of(point(m1, 1, v, P[1]).fmul_m(w1).fadd(y0w)),
                f32_of(point(m1, 2, v, P[2]).fmul_m(w1).fadd_m(z0w)),
            ];
            let mut add_bone = |m: GPtr<u8>, wk: f32, acc: &mut [f32; 3], positions: bool| {
                if positions {
                    for r in 0..3 {
                        acc[r] = f32_of(point(m, r as isize, v, P[r]).fmul_m(wk).fadd_m(acc[r]));
                    }
                }
                if !n.is_null() {
                    for r in 0..3 {
                        nacc[r] = f32_of(row(m, r as isize, n, N[r]).fmul_m(wk).fadd_m(nacc[r]));
                    }
                }
            };
            add_bone(m1, w1, &mut acc, false);
            if rd::<i32>(w, 8) > 0 {
                add_bone(bone(mats, ix >> 16), rd::<f32>(w, 8), &mut acc, true);
                if rd::<i32>(w, 0xC) > 0 {
                    add_bone(bone(mats, ix >> 24), rd::<f32>(w, 0xC), &mut acc, true);
                }
            }
            put3(o, acc);
            o = o.byte_add(stride as isize);
            v = v.byte_add(0xC);
            if !on.is_null() {
                put3(on, nacc);
                on = on.byte_add(stride as isize);
                n = n.byte_add(0xC);
            }
        }
        i += 1;
        w = w.byte_add(0x10);
    }
}

/// `_rpSkinBlendPositions2Weights`-like x87 blender (0x7CA410, cdecl, the arguments of 0x7C9DA0): a first weight
/// whose bits are at least those of 1.0 (signed) takes bone 0 alone, its normals when `outNormals` is set;
/// otherwise bones 0 and 1 by their weights (the weights still 4 floats per vertex), the normals when `inNormals` is
/// set. Bone 1's products are added in other orders than bone 0's. The original advances `outNormals` in the
/// caller's argument slot.
#[hook(addr = 0x7CA410, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "83 EC 1C 8B 4C 24 20",
       unverified = "frontier2-c25", route = "L2", hazards = "heap-writes,stack-residue",
       note = "advances outNormals in the caller's argument slot")]
#[allow(clippy::too_many_arguments)]
pub fn skin_blend_x87_weights2(count: u32, weights: GPtr<u8>, idx: GPtr<u8>, mats: GPtr<u8>, out: GPtr<u8>, inp: GPtr<u8>,
    out_n: GPtr<u8>, in_n: GPtr<u8>, stride: u32) {
    const P0: [[isize; 3]; 3] = [[0, 2, 1], [0, 2, 1], [1, 0, 2]];
    const N0: [[isize; 3]; 3] = [[2, 0, 1], [2, 0, 1], [2, 1, 0]];
    const P1: [[isize; 3]; 3] = [[1, 0, 2], [1, 2, 0], [0, 2, 1]];
    const N1: [[isize; 3]; 3] = [[1, 0, 2], [1, 0, 2], [2, 0, 1]];
    let (mut w, mut o, mut v, mut on, mut n) = (weights, out, inp, out_n, in_n);
    let mut i = 0;
    while i < count as i32 {
        let ix = rd::<u32>(idx, 4 * i as isize);
        let (m0, m1) = (bone(mats, ix), bone(mats, ix >> 8));
        let x0 = point(m0, 0, v, P0[0]);
        if rd::<i32>(w, 0) >= 0x3F80_0000 {
            put3(o, [f32_of(x0), f32_of(point(m0, 1, v, P0[1])), f32_of(point(m0, 2, v, P0[2]))]);
            v = v.byte_add(0xC);
            o = o.byte_add(stride as isize);
            if !on.is_null() {
                put3(on, [0, 1, 2].map(|r| f32_of(row(m0, r, n, N0[r as usize]))));
                on = on.byte_add(stride as isize);
                n = n.byte_add(0xC);
            }
        } else {
            let (w0, w1) = (rd::<f32>(w, 0), rd::<f32>(w, 4));
            let x0w = x0.fmul_m(w0);
            let y0w = point(m0, 1, v, P0[1]).fmul_m(w0);
            let z0w = f32_of(point(m0, 2, v, P0[2]).fmul_m(w0));
            let n0w = if n.is_null() { [0f32; 3] } else { [0, 1, 2].map(|r| f32_of(row(m0, r, n, N0[r as usize]).fmul_m(w0))) };
            put3(o, [
                f32_of(point(m1, 0, v, P1[0]).fmul_m(w1).fadd(x0w)),
                f32_of(point(m1, 1, v, P1[1]).fmul_m(w1).fadd(y0w)),
                f32_of(point(m1, 2, v, P1[2]).fmul_m(w1).fadd_m(z0w)),
            ]);
            v = v.byte_add(0xC);
            o = o.byte_add(stride as isize);
            if !n.is_null() {
                put3(on, [0, 1, 2].map(|r| f32_of(row(m1, r, n, N1[r as usize]).fmul_m(w1).fadd_m(n0w[r as usize]))));
                on = on.byte_add(stride as isize);
                n = n.byte_add(0xC);
            }
        }
        i += 1;
        w = w.byte_add(0x10);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_SKIN_PREPARE_MATRICES_X87, HOOK_SKIN_BLEND_X87_WEIGHTS4, HOOK_SKIN_BLEND_X87_WEIGHTS2];
