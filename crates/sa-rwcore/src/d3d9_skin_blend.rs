//! The CPU skinning of the D3D9 skin node (batch `std10-skin-2`): the bone matrices (0x7CB390, cached by 0x7CAA30)
//! and the SSE vertex blenders (0x7CAC60 up to four weights, 0x7CAF50 two weights, 0x7CAAD0 one matrix, 0x7CAB80
//! one bone per vertex). Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
//!
//! The SSE blenders work on whole matrix rows (`mulps` / `addps` on four lanes; lane 3 is never stored), so every
//! output component is an f32 expression of the same component of the rows: per lane
//! `((z * at + pos) + (y * up + x * right))` for positions and `(z * at + (y * up + x * right))` for normals, the
//! blended rows `((m1 * w1 + m0 * w0) + m2 * w2) + m3 * w3`. f32 arithmetic here is SSE arithmetic (i686 target, the
//! MXCSR flush-to-zero bit RW sets is in effect for both).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, rw_globals, wr};

/// The skin matrix buffer (`RwMatrix[]`, 16-aligned).
const MATRIX_BUFFER: usize = 0xC978AC;
/// Last hierarchy / render frame / atomic frame of the matrix cache (shared with
/// `_rpD3D9SkinVertexShaderMatrixUpdate`).
const CACHE_HIER: usize = 0xC978D4;
const CACHE_FRAME: usize = 0xC978D8;
const CACHE_ATOMIC_FRAME: usize = 0xC978DC;
/// The f32 1.0 whose bits the first weight is compared with (unsigned), and the f32 0.0 of the extra weights.
const ONE_BITS: usize = 0x882DE4;
const ZERO: usize = 0x882DE8;

orig! {
    /// `RpSkinGetSkinToBoneMatrices(skin)` (0x7C7810)
    fn skin_to_bone(skin: GPtr<u8>) -> u32 = cdecl 0x7C7810 => crate::skin::rp_skin_get_skin_to_bone_matrices;
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => crate::frame::get_ltm;
    /// `RwMatrixMultiply(dst, a, b)` (0x7F18B0)
    fn matrix_multiply(dst: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18B0 => crate::matrix::rw_matrix_multiply;
    /// `RwMatrixInvert(dst, src)` (0x7F2070)
    fn matrix_invert(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F2070 => crate::matrix::rw_matrix_invert;
    /// The bone matrices (0x7CB390)
    fn prepare_internal_call(atomic: GPtr<u8>, skin: GPtr<u8>, hierarchy: GPtr<u8>) -> GPtr<u8> = cdecl 0x7CB390 => skin_prepare_matrices_internal;
}

/// A 16-byte aligned local RwMatrix.
#[repr(C, align(16))]
struct Mat([u32; 16]);

impl Mat {
    fn p(&mut self) -> GPtr<u8> {
        GPtr::from_raw(self.0.as_mut_ptr()).cast()
    }
}

/// The bone matrices into the skin matrix buffer (0x7CB390, cdecl; `_rpSkinPrepareMatricesInternal`-like): NULL
/// without a hierarchy. Per node `i`: NOMATRICES (flags bit 1): `(skinToBone[i] * LTM(node frame)) *
/// inverse(LTM(atomic frame))`; LOCALSPACE (0x4000): `skinToBone[i] * matrices[i]`; else `(skinToBone[i] *
/// matrices[i]) * inverse(LTM(atomic frame))`. Returns the buffer.
#[hook(addr = 0x7CB390, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "81 EC 88 00 00 00 33 C0 53 55",
       unverified = "std10-skin-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn skin_prepare_matrices_internal(atomic: GPtr<u8>, skin: GPtr<u8>, hierarchy: GPtr<u8>) -> GPtr<u8> {
    if hierarchy.is_null() {
        return GPtr::NULL;
    }
    let buf = p8(g::<u32>(MATRIX_BUFFER).read());
    let mut bone = p8(skin_to_bone(skin));
    let mut out = buf;
    let (mut inv, mut tmp) = (Mat([0; 16]), Mat([0; 16]));
    let flags = rd::<u32>(hierarchy, 0);
    let nodes = || rd::<i32>(hierarchy, 4);
    if flags & 2 != 0 {
        let ltm = frame_get_ltm(p8(rd::<u32>(atomic, 4)));
        matrix_invert(inv.p(), ltm);
        let mut i = 0;
        while i < nodes() {
            let info = p8(rd::<u32>(hierarchy, 0x10)).byte_add(0x10 * i as isize);
            let node_ltm = frame_get_ltm(p8(rd::<u32>(info, 0xC)));
            matrix_multiply(tmp.p(), bone, node_ltm);
            matrix_multiply(out, tmp.p(), inv.p());
            i += 1;
            bone = bone.byte_add(0x40);
            out = out.byte_add(0x40);
        }
        return buf;
    }
    if flags & 0x4000 != 0 {
        let mut i = 0;
        while i < nodes() {
            let m = p8(rd::<u32>(hierarchy, 8)).byte_add(0x40 * i as isize);
            matrix_multiply(out, bone, m);
            i += 1;
            bone = bone.byte_add(0x40);
            out = out.byte_add(0x40);
        }
        return buf;
    }
    let ltm = frame_get_ltm(p8(rd::<u32>(atomic, 4)));
    matrix_invert(inv.p(), ltm);
    let mut i = 0;
    while i < nodes() {
        let m = p8(rd::<u32>(hierarchy, 8)).byte_add(0x40 * i as isize);
        matrix_multiply(tmp.p(), bone, m);
        matrix_multiply(out, tmp.p(), inv.p());
        i += 1;
        bone = bone.byte_add(0x40);
        out = out.byte_add(0x40);
    }
    buf
}

/// The cached bone matrices of the SSE path (0x7CAA30, cdecl; `_rpSkinPrepareMatricesSSE`-like): unless hierarchy,
/// render frame (`RwGlobals` +8, u16) and atomic frame equal the cached ones, `skin_prepare_matrices_internal` and the
/// fourth column (+0xC, +0x1C, +0x2C, +0x3C) of every node's matrix zeroed, then the cache updated. Returns the buffer.
#[hook(addr = 0x7CAA30, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "A1 D4 78 C9 00 56 8B 74 24 10",
       unverified = "std10-skin-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn skin_prepare_matrices_sse(atomic: GPtr<u8>, skin: GPtr<u8>, hierarchy: GPtr<u8>) -> GPtr<u8> {
    let frame = || u32::from(rd::<u16>(rw_globals(), 8));
    let hit = g::<u32>(CACHE_HIER).read() == hierarchy.addr() as u32
        && g::<u32>(CACHE_FRAME).read() == frame()
        && g::<u32>(CACHE_ATOMIC_FRAME).read() == rd::<u32>(atomic, 4);
    if !hit {
        prepare_internal_call(atomic, skin, hierarchy);
        let buf = p8(g::<u32>(MATRIX_BUFFER).read());
        let mut i = 0;
        while i < rd::<i32>(hierarchy, 4) {
            let m = buf.byte_add(0x40 * i as isize);
            for off in [0xC, 0x1C, 0x2C, 0x3C] {
                wr::<u32>(m, off, 0);
            }
            i += 1;
        }
        g::<u32>(CACHE_HIER).write(hierarchy.addr() as u32);
        g::<u32>(CACHE_FRAME).write(frame());
        g::<u32>(CACHE_ATOMIC_FRAME).write(rd::<u32>(atomic, 4));
    }
    p8(g::<u32>(MATRIX_BUFFER).read())
}

/// Three rows (right, up, at) and the position row of a matrix, xyz lanes.
#[derive(Clone, Copy)]
struct Rows([[f32; 3]; 4]);

impl Rows {
    fn load(m: GPtr<u8>) -> Rows {
        let mut r = [[0f32; 3]; 4];
        for (k, row) in r.iter_mut().enumerate() {
            for (l, v) in row.iter_mut().enumerate() {
                *v = rd::<f32>(m, (0x10 * k + 4 * l) as isize);
            }
        }
        Rows(r)
    }

    /// `m * w` per element (`mulps` by the broadcast weight).
    fn scaled(m: GPtr<u8>, w: f32) -> Rows {
        let mut r = Rows::load(m);
        for row in r.0.iter_mut() {
            for v in row.iter_mut() {
                *v *= w;
            }
        }
        r
    }

    /// `self + o` per element (`addps self, o`).
    fn add(self, o: Rows) -> Rows {
        let mut r = self;
        for k in 0..4 {
            for l in 0..3 {
                r.0[k][l] = self.0[k][l] + o.0[k][l];
            }
        }
        r
    }

    /// Transforms the point at `src` into `dst` (xyz).
    fn point(&self, src: GPtr<u8>, dst: GPtr<u8>) {
        let (px, py, pz) = (rd::<f32>(src, 0), rd::<f32>(src, 4), rd::<f32>(src, 8));
        for l in 0..3 {
            let a = py * self.0[1][l] + px * self.0[0][l];
            let b = pz * self.0[2][l] + self.0[3][l];
            wr::<f32>(dst, 4 * l as isize, b + a);
        }
    }

    /// Transforms the normal at `src` into `dst` (xyz, no translation).
    fn normal(&self, src: GPtr<u8>, dst: GPtr<u8>) {
        let (px, py, pz) = (rd::<f32>(src, 0), rd::<f32>(src, 4), rd::<f32>(src, 8));
        for l in 0..3 {
            let a = py * self.0[1][l] + px * self.0[0][l];
            wr::<f32>(dst, 4 * l as isize, pz * self.0[2][l] + a);
        }
    }
}

/// The weighted blenders' shared loop: `extra` weights beyond two (0 or 2).
#[allow(clippy::too_many_arguments)]
fn blend_weighted(
    n: u32,
    weights: GPtr<u8>,
    indices: GPtr<u8>,
    mats: GPtr<u8>,
    dst: GPtr<u8>,
    src: GPtr<u8>,
    dst_n: GPtr<u8>,
    src_n: GPtr<u8>,
    stride: u32,
    extra: bool,
) {
    let (mut w, mut ix, mut d, mut s, mut dn, mut sn) = (weights, indices, dst, src, dst_n, src_n);
    let mut k = n;
    let bone = |ix: GPtr<u8>, b: isize| mats.byte_add((u32::from(rd::<u8>(ix, b)) << 6) as isize);
    loop {
        let m = if rd::<u32>(w, 0) >= g::<u32>(ONE_BITS).read() {
            Rows::load(bone(ix, 0))
        } else {
            let mut m = Rows::scaled(bone(ix, 1), rd::<f32>(w, 4)).add(Rows::scaled(bone(ix, 0), rd::<f32>(w, 0)));
            let zero = g::<f32>(ZERO).read();
            if extra && rd::<f32>(w, 8) > zero {
                m = m.add(Rows::scaled(bone(ix, 2), rd::<f32>(w, 8)));
                if rd::<f32>(w, 0xC) > zero {
                    m = m.add(Rows::scaled(bone(ix, 3), rd::<f32>(w, 0xC)));
                }
            }
            m
        };
        m.point(s, d);
        s = s.byte_add(0xC);
        d = d.byte_add(stride as isize);
        if !sn.is_null() {
            m.normal(sn, dn);
            sn = sn.byte_add(0xC);
            dn = dn.byte_add(stride as isize);
        }
        w = w.byte_add(0x10);
        ix = ix.byte_add(4);
        k = k.wrapping_sub(1);
        if k == 0 {
            break;
        }
    }
}

/// The SSE blender for up to four weights (0x7CAC60, cdecl): per vertex a first weight of 1.0 or more (unsigned
/// compare of the bits) takes the first bone's matrix alone; else the first two bones blended, the third and fourth
/// added while their weight is above 0.0. Positions with translation, normals (when given) without.
#[hook(addr = 0x7CAC60, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "53 8B DC 51 83 E4 F0 83 C4 04",
       unverified = "std10-skin-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
#[allow(clippy::too_many_arguments)]
pub fn skin_blend_sse_weights4(
    n: u32,
    weights: GPtr<u8>,
    indices: GPtr<u8>,
    mats: GPtr<u8>,
    dst: GPtr<u8>,
    src: GPtr<u8>,
    dst_n: GPtr<u8>,
    src_n: GPtr<u8>,
    stride: u32,
) {
    blend_weighted(n, weights, indices, mats, dst, src, dst_n, src_n, stride, true);
}

/// The SSE blender for two weights (0x7CAF50, cdecl): as 0x7CAC60 without the third and fourth weights.
#[hook(addr = 0x7CAF50, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "53 8B DC 51 83 E4 F0 83 C4 04",
       unverified = "std10-skin-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
#[allow(clippy::too_many_arguments)]
pub fn skin_blend_sse_weights2(
    n: u32,
    weights: GPtr<u8>,
    indices: GPtr<u8>,
    mats: GPtr<u8>,
    dst: GPtr<u8>,
    src: GPtr<u8>,
    dst_n: GPtr<u8>,
    src_n: GPtr<u8>,
    stride: u32,
) {
    blend_weighted(n, weights, indices, mats, dst, src, dst_n, src_n, stride, false);
}

/// The SSE blender for one matrix (0x7CAAD0, cdecl): every vertex by `matrix`.
#[hook(addr = 0x7CAAD0, conv = cdecl, name = "FUN_007caad0", expect = "55 8B EC 53 56 57 8B 4D 08 8B",
       unverified = "std10-skin-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn skin_blend_sse_one_matrix(n: u32, matrix: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>, dst_n: GPtr<u8>, src_n: GPtr<u8>, stride: u32) {
    let m = Rows::load(matrix);
    let (mut d, mut s, mut dn, mut sn) = (dst, src, dst_n, src_n);
    let mut k = n;
    loop {
        m.point(s, d);
        s = s.byte_add(0xC);
        d = d.byte_add(stride as isize);
        if !sn.is_null() {
            m.normal(sn, dn);
            sn = sn.byte_add(0xC);
            dn = dn.byte_add(stride as isize);
        }
        k = k.wrapping_sub(1);
        if k == 0 {
            break;
        }
    }
}

/// The SSE blender for one bone per vertex (0x7CAB80, cdecl): every vertex by the matrix of its first bone index.
#[hook(addr = 0x7CAB80, conv = cdecl, name = "FUN_007cab80", expect = "55 8B EC 53 56 57 8B 4D 08 8B",
       unverified = "std10-skin-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
#[allow(clippy::too_many_arguments)]
pub fn skin_blend_sse_rigid(
    n: u32,
    indices: GPtr<u8>,
    mats: GPtr<u8>,
    dst: GPtr<u8>,
    src: GPtr<u8>,
    dst_n: GPtr<u8>,
    src_n: GPtr<u8>,
    stride: u32,
) {
    let (mut ix, mut d, mut s, mut dn, mut sn) = (indices, dst, src, dst_n, src_n);
    let mut k = n;
    loop {
        let m = Rows::load(mats.byte_add((u32::from(rd::<u8>(ix, 0)) << 6) as isize));
        ix = ix.byte_add(4);
        m.point(s, d);
        s = s.byte_add(0xC);
        d = d.byte_add(stride as isize);
        if !sn.is_null() {
            m.normal(sn, dn);
            sn = sn.byte_add(0xC);
            dn = dn.byte_add(stride as isize);
        }
        k = k.wrapping_sub(1);
        if k == 0 {
            break;
        }
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_SKIN_PREPARE_MATRICES_INTERNAL,
    HOOK_SKIN_PREPARE_MATRICES_SSE,
    HOOK_SKIN_BLEND_SSE_WEIGHTS4,
    HOOK_SKIN_BLEND_SSE_WEIGHTS2,
    HOOK_SKIN_BLEND_SSE_ONE_MATRIX,
    HOOK_SKIN_BLEND_SSE_RIGID,
];
