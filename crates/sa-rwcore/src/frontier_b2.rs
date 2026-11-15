//! FRONTIER-2 turn 2, lane B (batch `frontier2-b8`): RenderWare functions on the phase B frontier (the skin native
//! stream read, the MatFX vertex-shader draw). Ported from the 1.0 US bytes (RenderWare is not reversed in
//! gta-reversed). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::bindings::{flush_cache, read_int32, rp_skin_geometry_set_skin};
use crate::common::{g, rd, rw_error_set, rw_error1, rw_memory_alloc, wr, RwError};
use sa_math::x87::X87;
use crate::d3d9dev::{dev_draw_indexed_primitive, dev_draw_primitive, dev_set_pixel_shader, dev_set_vertex_shader};
use crate::d3d9state::device;
use sa_core::bindings::rw_stream_find_chunk_call;

/// `_rpSkinGeometryNativeRead(RwStream*, RpGeometry*)` (0x7C88B0, cdecl; name from the RW symbols, low confidence;
/// called by the skin plugin's native-data read 0x7C6D20): the struct chunk (`RwStreamFindChunk(stream, 1, &len,
/// &version)`, the version kept in the caller's `stream` argument slot); a version outside 0x34000..=0x36003 sets the
/// error {0x116, `_rwerror(0x80000004)`} and returns null. A first int32 other than 9, or a failed read, returns null.
/// Else a skin from the skin free list (0xC978B8, `memoryAlloc` hint 0x30116) zeroed (0x40 bytes), its first int32
/// read, `RpSkinGeometrySetSkin(geometry, skin)`; returns the stream.
#[hook(addr = 0x7C88B0, conv = cdecl, name = "FUN_007c88b0", expect = "83 EC 10 8D 44 24 14",
       unverified = "frontier2-b8", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the chunk version is written into the caller's stream argument slot")]
pub fn skin_geometry_native_read(stream: GPtr<u8>, object: GPtr<u8>) -> GPtr<u8> {
    let mut l = [0u32, 0u32];
    let mut ver = [0u32];
    let lp = GPtr::from_raw(l.as_mut_ptr());
    let vp = GPtr::from_raw(ver.as_mut_ptr());
    if rw_stream_find_chunk_call(stream, 1, lp, vp) == 0 {
        return GPtr::NULL;
    }
    let v = vp.read();
    if !(0x34000..=0x36003).contains(&v) {
        let mut e = RwError { plugin_id: 0x116, error_code: 0 };
        e.error_code = rw_error1(0x8000_0004) as i32;
        rw_error_set(GPtr::from_raw(&mut e).cast());
        return GPtr::NULL;
    }
    let mut ty = [0u32];
    let tp = GPtr::from_raw(ty.as_mut_ptr()).cast::<u8>();
    if read_int32(stream, tp, 4).is_null() || rd::<u32>(tp, 0) != 9 {
        return GPtr::NULL;
    }
    let skin = rw_memory_alloc(g::<GPtr<u8>>(0xC9_78B8).read(), 0x30116);
    for k in 0..16isize {
        skin.byte_add(4 * k).cast::<u32>().write(0);
    }
    if read_int32(stream, skin, 4).is_null() {
        return GPtr::NULL;
    }
    rp_skin_geometry_set_skin(object, skin);
    stream
}

/// `_rpMatFXD3D9SetVertexShaderState`-like draw (0x812C90, cdecl; name from the RW symbols, low confidence; called by
/// the MatFX D3D9 render callback): the cached pixel shader (0x8E244C) cleared with `SetPixelShader(NULL)` when set;
/// the mesh's vertex shader (+0x10) set when it differs from the cached one (0x8E2448); the render-state cache
/// flushed; `DrawIndexedPrimitive(header +0xC, mesh +0x14, 0, +0x18, +0x1C, +0x20)` with an index buffer (header
/// +8), else `DrawPrimitive(header +0xC, mesh +0x14, +0x20)`. Returns the draw's result.
#[hook(addr = 0x812C90, conv = cdecl, name = "FUN_00812c90", expect = "A1 4C 24 8E 00 85 C0",
       unverified = "frontier2-b8", route = "L2", hazards = "none")]
pub fn matfx_d3d9_draw_mesh(header: GPtr<u8>, mesh: GPtr<u8>) -> i32 {
    if g::<u32>(0x8E_244C).read() != 0 {
        let dev = device();
        g::<u32>(0x8E_244C).write(0);
        dev_set_pixel_shader(dev, 0);
    }
    let vs = rd::<u32>(mesh, 0x10);
    if g::<u32>(0x8E_2448).read() != vs {
        g::<u32>(0x8E_2448).write(vs);
        dev_set_vertex_shader(device(), rd(mesh, 0x10));
    }
    flush_cache();
    if rd::<u32>(header, 8) != 0 {
        dev_draw_indexed_primitive(device(), rd(header, 0xC), rd(mesh, 0x14), 0, rd(mesh, 0x18), rd(mesh, 0x1C), rd(mesh, 0x20))
    } else {
        dev_draw_primitive(device(), rd(header, 0xC), rd(mesh, 0x14), rd(mesh, 0x20))
    }
}

/// `fld a; fmul b` (both f32 memory operands).
fn mulf(a: f32, b: f32) -> X87 {
    X87::from_f32(a).fmul_m(b)
}

/// `_rpSkinBlendPositionsRigid(count, const RwMatrix* m, RwV3d* outPos, const RwV3d* inPos, RwV3d* outNormals, const
/// RwV3d* inNormals, stride)` (0x7CA330, cdecl; name from the RW symbols, low confidence): every position transformed
/// by `m` (x87, the original's operand order: x = `(m0*vx + vz*m8) + vy*m4 + m12`, y = `(m16*vx + m24*vz) + vy*m20 +
/// m28`, z = `(m36*vy + m32*vx) + vz*m40 + m44`, byte offsets), the outputs `stride` bytes apart; with `outNormals` the
/// normals rotated (`(ny*m4 + m0*nx) + m8*nz`, `(m24*nz + ny*m20) + m16*nx`, `(m36*ny + m32*nx) + m40*nz`).
#[hook(addr = 0x7CA330, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "8B 44 24 08 8B 54 24 18",
       unverified = "frontier2-b8", route = "L2", hazards = "none")]
pub fn skin_blend_positions_rigid(count: u32, m: GPtr<u8>, out: GPtr<u8>, inp: GPtr<u8>, out_n: GPtr<u8>, in_n: GPtr<u8>, stride: u32) {
    let f = |p: GPtr<u8>, o: isize| rd::<f32>(p, o);
    let (mut o, mut v, mut on, mut n) = (out, inp, out_n, in_n);
    let mut k = count;
    loop {
        let x = mulf(f(m, 0), f(v, 0)).fadd(mulf(f(v, 8), f(m, 8))).fadd(mulf(f(v, 4), f(m, 4))).fadd_m(f(m, 0xC));
        wr::<f32>(o, 0, x.to_f32());
        let y = mulf(f(m, 0x10), f(v, 0)).fadd(mulf(f(m, 0x18), f(v, 8))).fadd(mulf(f(v, 4), f(m, 0x14))).fadd_m(f(m, 0x1C));
        wr::<f32>(o, 4, y.to_f32());
        let z = mulf(f(m, 0x24), f(v, 4)).fadd(mulf(f(m, 0x20), f(v, 0))).fadd(mulf(f(v, 8), f(m, 0x28))).fadd_m(f(m, 0x2C));
        wr::<f32>(o, 8, z.to_f32());
        v = v.byte_add(0xC);
        o = o.byte_add(stride as isize);
        if !on.is_null() {
            let x = mulf(f(n, 4), f(m, 4)).fadd(mulf(f(m, 0), f(n, 0))).fadd(mulf(f(m, 8), f(n, 8)));
            wr::<f32>(on, 0, x.to_f32());
            let y = mulf(f(m, 0x18), f(n, 8)).fadd(mulf(f(n, 4), f(m, 0x14))).fadd(mulf(f(m, 0x10), f(n, 0)));
            wr::<f32>(on, 4, y.to_f32());
            let z = mulf(f(m, 0x24), f(n, 4)).fadd(mulf(f(m, 0x20), f(n, 0))).fadd(mulf(f(m, 0x28), f(n, 8)));
            wr::<f32>(on, 8, z.to_f32());
            on = on.byte_add(stride as isize);
            n = n.byte_add(0xC);
        }
        k = k.wrapping_sub(1);
        if k == 0 {
            break;
        }
    }
}

/// `_rpSkinBlendPositions1Weight(count, const uint32* indices, const RwMatrix* matrices, RwV3d* outPos, const RwV3d*
/// inPos, RwV3d* outNormals, const RwV3d* inNormals, stride)` (0x7CA240, cdecl; name from the RW symbols, low
/// confidence): as [`skin_blend_positions_rigid`] with the matrix `matrices + 0x40 * (index & 0xFF)` per vertex and
/// the original's operand order of this copy (x = `(vz*m8 + vy*m4) + vx*m0 + m12`, y = `(vy*m20 + vx*m16) + vz*m24 +
/// m28`, z = `(vz*m40 + m36*vy) + vx*m32 + m44`; normals `(ny*m4 + m0*nx) + m8*nz`, `(ny*m20 + nx*m16) + nz*m24`,
/// `(m32*nx + m40*nz) + m36*ny`). The count is decremented in the caller's argument slot.
#[hook(addr = 0x7CA240, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "8B 4C 24 14 8B 54 24 10",
       unverified = "frontier2-b8", route = "L2", hazards = "stack-residue",
       note = "decrements its count in the caller's argument slot")]
#[allow(clippy::too_many_arguments)]
pub fn skin_blend_positions_1_weight(count: u32, idx: GPtr<u8>, mats: GPtr<u8>, out: GPtr<u8>, inp: GPtr<u8>, out_n: GPtr<u8>, in_n: GPtr<u8>, stride: u32) {
    let f = |p: GPtr<u8>, o: isize| rd::<f32>(p, o);
    let (mut ix, mut o, mut v, mut on, mut n) = (idx, out, inp, out_n, in_n);
    let mut k = count;
    loop {
        let m = mats.byte_add(((rd::<u32>(ix, 0) & 0xFF) << 6) as isize);
        ix = ix.byte_add(4);
        let x = mulf(f(v, 8), f(m, 8)).fadd(mulf(f(v, 4), f(m, 4))).fadd(mulf(f(v, 0), f(m, 0))).fadd_m(f(m, 0xC));
        wr::<f32>(o, 0, x.to_f32());
        let y = mulf(f(v, 4), f(m, 0x14)).fadd(mulf(f(v, 0), f(m, 0x10))).fadd(mulf(f(v, 8), f(m, 0x18))).fadd_m(f(m, 0x1C));
        wr::<f32>(o, 4, y.to_f32());
        let z = mulf(f(v, 8), f(m, 0x28)).fadd(mulf(f(m, 0x24), f(v, 4))).fadd(mulf(f(v, 0), f(m, 0x20))).fadd_m(f(m, 0x2C));
        wr::<f32>(o, 8, z.to_f32());
        v = v.byte_add(0xC);
        o = o.byte_add(stride as isize);
        if !on.is_null() {
            let x = mulf(f(n, 4), f(m, 4)).fadd(mulf(f(m, 0), f(n, 0))).fadd(mulf(f(m, 8), f(n, 8)));
            wr::<f32>(on, 0, x.to_f32());
            let y = mulf(f(n, 4), f(m, 0x14)).fadd(mulf(f(n, 0), f(m, 0x10))).fadd(mulf(f(n, 8), f(m, 0x18)));
            wr::<f32>(on, 4, y.to_f32());
            let z = mulf(f(m, 0x20), f(n, 0)).fadd(mulf(f(m, 0x28), f(n, 8))).fadd(mulf(f(m, 0x24), f(n, 4)));
            wr::<f32>(on, 8, z.to_f32());
            on = on.byte_add(stride as isize);
            n = n.byte_add(0xC);
        }
        k = k.wrapping_sub(1);
        if k == 0 {
            break;
        }
    }
}

/// `_rwImageApplyMaskTrueColor`-like depth copy (0x803840, cdecl `(RwImage* dst, const RwImage* src)`, AL-wide BOOL;
/// name from the RW symbols, low confidence): by `(src depth << 8) | dst depth` (+0xC): 8 -> 32 bits (0x820) and 4 ->
/// 32 (0x420) expand each source byte through the source palette (+0x18) into a dword per pixel; 4 -> 8 (0x408)
/// copies the rows (`width` bytes, `rep movsd` / `movsb`); 0x808, 0x404 and 0x2020 do nothing; rows step by each
/// image's stride (+0x10), the pixels at +0x14, the size +4 / +8 of `dst`. Anything else sets the error {1,
/// `_rwerror(0x80000009)`} and returns 0; else 1. The width is kept in the caller's `dst` argument slot.
#[hook(addr = 0x803840, conv = cdecl, name = "FUN_00803840", expect = "83 EC 08 8B 4C 24 10",
       unverified = "frontier2-b8", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the width is kept in the caller's dst argument slot")]
pub fn image_copy_depth(dst: GPtr<u8>, src: GPtr<u8>) -> u32 {
    let width = rd::<i32>(dst, 4);
    let mut s = rd::<GPtr<u8>>(src, 0x14);
    let pal = rd::<GPtr<u8>>(src, 0x18);
    let mut d = rd::<GPtr<u8>>(dst, 0x14);
    let mut h = rd::<i32>(dst, 8);
    let key = (rd::<u32>(src, 0xC) << 8) | rd::<u32>(dst, 0xC);
    match key {
        0x808 | 0x404 | 0x2020 => 1,
        0x820 | 0x420 => {
            if h > 0 {
                loop {
                    let mut x = 0i32;
                    while x < width {
                        let c = rd::<u8>(s, x as isize);
                        wr::<u32>(d, 4 * x as isize, rd::<u32>(pal, 4 * isize::from(c)));
                        x += 1;
                    }
                    s = s.byte_add(rd::<i32>(src, 0x10) as isize);
                    d = d.byte_add(rd::<i32>(dst, 0x10) as isize);
                    h -= 1;
                    if h == 0 {
                        break;
                    }
                }
            }
            1
        }
        0x408 => {
            if h > 0 {
                loop {
                    for k in 0..width as u32 as isize {
                        wr::<u8>(d, k, rd::<u8>(s, k));
                    }
                    s = s.byte_add(rd::<i32>(src, 0x10) as isize);
                    d = d.byte_add(rd::<i32>(dst, 0x10) as isize);
                    h -= 1;
                    if h == 0 {
                        break;
                    }
                }
            }
            1
        }
        _ => {
            let mut e = RwError { plugin_id: 1, error_code: 0 };
            e.error_code = rw_error1(0x8000_0009) as i32;
            rw_error_set(GPtr::from_raw(&mut e).cast());
            0
        }
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_IMAGE_COPY_DEPTH,
    HOOK_SKIN_GEOMETRY_NATIVE_READ,
    HOOK_MATFX_D3D9_DRAW_MESH,
    HOOK_SKIN_BLEND_POSITIONS_RIGID,
    HOOK_SKIN_BLEND_POSITIONS_1_WEIGHT,
];
