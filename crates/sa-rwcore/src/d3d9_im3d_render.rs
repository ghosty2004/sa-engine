//! `_rwD3D9ImmRenderPrimitiveNode` (0x80E270, batch `std10-rw-1`): the D3D9 draw node of RwIm3D (the sky
//! gradient, clouds, markers, ropes, every `RwIm3DRender*Primitive`). Ported from the 1.0 US bytes. Unverified
//! (docs/verify/HOWTO.md).
//!
//! State (0xC9AB30, `im3d.rs`): +0 the pool (u16 vertex count, +4 the `RwIm3DVertex` array (0x24 each: position,
//! normal, colour at +0x18, u / v at +0x1C / +0x20), +0xC the stash: flags, +4 LTM, +0x24 primitive type, +0x28
//! indices, +0x2C index count), +4 base vertex, +8 dynamic vertex buffer, +0xC index offset, +0x10 index buffer,
//! +0x14 / +0x18 / +0x1C the vertex declarations (full vertex, position + colour + uv, position + colour).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, wr};
use crate::d3d9state::device;

const STATE: usize = 0xC9AB30;
const BASE_VERTEX: usize = 0xC9AB34;
const VB: usize = 0xC9AB38;
const IB_OFFSET: usize = 0xC9AB3C;
const IB: usize = 0xC9AB40;
const DECL_FULL: usize = 0xC9AB44;
const DECL_UV: usize = 0xC9AB48;
const DECL_COLOR: usize = 0xC9AB4C;
const LAST_FVF: usize = 0x8E2440;
const LAST_DECL: usize = 0x8E2444;
const LAST_VS: usize = 0x8E2448;
const LAST_PS: usize = 0x8E244C;
const LAST_IB: usize = 0x8E2450;
/// RW primitive type -> `D3DPRIMITIVETYPE` (dwords).
const PRIM_TYPES: usize = 0x88513C;
/// `rwIM3D_VERTEXUV`, `rwIM3D_ALLOPAQUE`, `rwIM3D_NOCLIP`.
const VERTEX_UV: u32 = 1;
const ALL_OPAQUE: u32 = 2;
const NO_CLIP: u32 = 4;

orig! {
    fn dev_set_vertex_declaration(dev: GPtr<u8>, decl: u32) -> i32 = stdcall vslot 0x15C => unported;
    fn dev_set_vertex_shader(dev: GPtr<u8>, vs: u32) -> i32 = stdcall vslot 0x170 => unported;
    fn dev_set_pixel_shader(dev: GPtr<u8>, ps: u32) -> i32 = stdcall vslot 0x1AC => unported;
    fn dev_set_indices(dev: GPtr<u8>, ib: u32) -> i32 = stdcall vslot 0x1A0 => unported;
    fn dev_draw_primitive(dev: GPtr<u8>, prim: u32, start: u32, count: u32) -> i32 = stdcall vslot 0x144 => unported;
    fn dev_draw_indexed_primitive(dev: GPtr<u8>, prim: u32, base: u32, min: u32, num: u32, start: u32, count: u32) -> i32 = stdcall vslot 0x148 => unported;
    fn dev_draw_primitive_up(dev: GPtr<u8>, prim: u32, count: u32, data: u32, stride: u32) -> i32 = stdcall vslot 0x14C => unported;
    fn dev_draw_indexed_primitive_up(dev: GPtr<u8>, prim: u32, min: u32, num: u32, count: u32, indices: u32, fmt: u32, data: u32, stride: u32) -> i32 = stdcall vslot 0x150 => unported;
    fn ib_lock(ib: GPtr<u8>, offset: u32, size: u32, data: GPtr<u32>, flags: u32) -> i32 = stdcall vslot 0x2C => unported;
    fn ib_unlock(ib: GPtr<u8>) -> i32 = stdcall vslot 0x30 => unported;

    /// `_rwD3D9RenderStateVertexAlphaEnable(enable)` (0x7FE0A0)
    fn vertex_alpha_enable(v: u32) -> u32 = cdecl 0x7FE0A0 => crate::d3d9state::rs_vertex_alpha_enable;
    /// `RwD3D9SetTransformWorld(matrix)` (0x7FA520)
    fn set_transform_world(m: GPtr<u8>) -> u32 = cdecl 0x7FA520 => crate::d3d9dev::set_transform_world;
    /// `RwD3D9SetRenderState(state, value)` (0x7FC2D0)
    fn set_render_state(state: u32, value: u32) -> u32 = cdecl 0x7FC2D0 => crate::d3d9state::set_render_state;
    /// `RwD3D9GetCurrentTexture(stage)`-like (0x7FDE50)
    fn get_current_texture(stage: u32) -> u32 = cdecl 0x7FDE50 => crate::d3d9state::get_current_texture;
    /// `RwD3D9SetTextureStageState(stage, type, value)` (0x7FC340)
    fn set_tss(stage: u32, ty: u32, value: u32) = cdecl 0x7FC340 => crate::d3d9state::set_texture_stage_state;
    /// `RwD3D9DynamicVertexBufferLock(stride, numVertices, &vb, &data, &baseIndex)` (0x7F5B10)
    fn dynamic_vertex_buffer_lock(stride: u32, n: u32, vb: GPtr<u32>, data: u32, base: GPtr<u32>) -> u32 = cdecl 0x7F5B10 => crate::d3d9vb::dynamic_vertex_buffer_lock;
    /// `RwD3D9DynamicVertexBufferUnlock(vb)` (0x7F5C90)
    fn dynamic_vertex_buffer_unlock(vb: GPtr<u8>) -> u32 = cdecl 0x7F5C90 => crate::d3d9vb::dynamic_vertex_buffer_unlock;
    /// `RwD3D9SetStreamSource(stream, vb, offset, stride)` (0x7FA030)
    fn set_stream_source(stream: u32, vb: u32, offset: u32, stride: u32) = cdecl 0x7FA030 => crate::d3d9dev::set_stream_source;
    /// `_rwD3D9RenderStateFlushCache()` (0x7FC200)
    fn flush_cache() = cdecl 0x7FC200 => crate::d3d9state::flush_cache;
}

fn gu(a: usize) -> u32 {
    g::<u32>(a).read()
}

fn su(a: usize, v: u32) {
    g::<u32>(a).write(v)
}

/// The primitive count of `n` vertices / indices for a D3D primitive type; other types keep `prev` (the original's
/// uninitialised local).
fn primitive_count(prim: u32, n: u32, prev: u32) -> u32 {
    match prim {
        2 => n >> 1,
        3 => n.wrapping_sub(1),
        4 => n / 3,
        5 | 6 => n.wrapping_sub(2),
        _ => prev,
    }
}

/// One Im3D vertex copied to the stream layout: position and colour, with `uv` also the texture coordinates.
fn copy_vertex(dst: GPtr<u8>, src: GPtr<u8>, uv: bool) {
    for (d, s) in [(0, 0), (4, 4), (8, 8), (0xC, 0x18)] {
        wr::<u32>(dst, d, rd::<u32>(src, s));
    }
    if uv {
        wr::<u32>(dst, 0x10, rd::<u32>(src, 0x1C));
        wr::<u32>(dst, 0x14, rd::<u32>(src, 0x20));
    }
}

/// `_rwD3D9ImmRenderPrimitiveNode(node, params)` (0x80E270, cdecl). Vertex layout by `rwIM3D_VERTEXUV` (0x18-byte
/// position / colour / uv, else 0x10-byte position / colour), vertex alpha off for `rwIM3D_ALLOPAQUE`, the stash's LTM
/// as world, LIGHTING and NORMALIZENORMALS off, CLIPPING unless `rwIM3D_NOCLIP`, stage 0 SELECTARG2 of DIFFUSE (no
/// texture) or MODULATE of TEXTURE and DIFFUSE, no shaders. Indexed: up to 8 indices (and no more than the vertices)
/// are expanded into the dynamic vertex buffer and drawn with `DrawPrimitive`; otherwise, with the shared index
/// buffer and at most 0xFFFF indices, the vertices go to the dynamic buffer, the indices into the index buffer
/// (DISCARD and back to offset 0 past 0xFFFF, else NOOVERWRITE at the running offset) and `DrawIndexedPrimitive`.
/// Unindexed: the vertices in the dynamic buffer and `DrawPrimitive`. Any failure falls back to the user-pointer draws
/// of the full 0x24-byte vertices. Returns 1.
#[hook(addr = 0x80E270, conv = cdecl, name = "_rwD3D9ImmRenderPrimitiveNode", expect = "83 EC 1C A1 30 AB C9 00 53 55",
       unverified = "std10-rw-1", route = "L2", hazards = "heap-writes, uninit-locals", recs = "grove-walk",
       note = "the primitive count of a type outside 2..6 is the original's uninitialised local (0 here)")]
pub fn imm_render_primitive_node(_node: GPtr<u8>, _params: GPtr<u8>) -> u32 {
    let pool = p8(gu(STATE));
    let stash = pool.byte_add(0xC);
    let verts = p8(rd::<u32>(pool, 4));
    let flags = rd::<u32>(stash, 0);
    let uv = flags & VERTEX_UV != 0;
    let (decl, stride) = if uv { (gu(DECL_UV), 0x18u32) } else { (gu(DECL_COLOR), 0x10u32) };
    vertex_alpha_enable(if flags & ALL_OPAQUE != 0 { 0 } else { 1 });
    set_transform_world(p8(rd::<u32>(stash, 4)));
    set_render_state(0x89, 0);
    set_render_state(0x8F, 0);
    set_render_state(0x88, if rd::<u8>(stash, 0) as u32 & NO_CLIP != 0 { 0 } else { 1 });
    if get_current_texture(0) == 0 {
        set_tss(0, 1, 3);
        set_tss(0, 3, 0);
        set_tss(0, 4, 3);
        set_tss(0, 6, 0);
    } else {
        set_tss(0, 1, 4);
        set_tss(0, 2, 2);
        set_tss(0, 3, 0);
        set_tss(0, 4, 4);
        set_tss(0, 5, 2);
        set_tss(0, 6, 0);
    }
    if gu(LAST_DECL) != decl {
        su(LAST_DECL, decl);
        su(LAST_FVF, 0xFFFF_FFFF);
        dev_set_vertex_declaration(device(), decl);
    }
    if gu(LAST_VS) != 0 {
        su(LAST_VS, 0);
        dev_set_vertex_shader(device(), 0);
    }
    if gu(LAST_PS) != 0 {
        su(LAST_PS, 0);
        dev_set_pixel_shader(device(), 0);
    }
    let prim = gu(PRIM_TYPES + 4 * rd::<u32>(stash, 0x24) as usize);
    let num_verts = u32::from(rd::<u16>(pool, 0));
    if num_verts == 0 {
        return 1;
    }
    let indices = rd::<u32>(stash, 0x28);
    let num_indices = rd::<u32>(stash, 0x2C);
    let mut count: u32 = 0;
    let mut data_cell: u32 = 0;
    let dp = GPtr::<u32>::from_raw(&raw mut data_cell);
    let data_p = || dp.addr() as u32;
    let lock = |n: u32, out: u32| dynamic_vertex_buffer_lock(stride, n, g::<u32>(VB), out, g::<u32>(BASE_VERTEX)) != 0;
    if indices != 0 && num_indices > 0 {
        count = primitive_count(prim, num_indices, count);
        let idx = p8(indices);
        let mut drawn = false;
        if num_indices <= num_verts && num_indices <= 8 {
            if lock(num_indices, data_p()) {
                for i in 0..num_indices as isize {
                    let v = verts.byte_add(0x24 * rd::<u16>(idx, 2 * i) as isize);
                    copy_vertex(p8(dp.read()).byte_add(stride as isize * i), v, uv);
                }
                dynamic_vertex_buffer_unlock(p8(gu(VB)));
                set_stream_source(0, gu(VB), 0, stride);
                flush_cache();
                drawn = dev_draw_primitive(device(), prim, gu(BASE_VERTEX), count) >= 0;
            }
        } else if gu(IB) != 0 && num_indices <= 0xFFFF && lock(num_verts, data_p()) {
            for i in 0..num_verts as isize {
                copy_vertex(p8(dp.read()).byte_add(stride as isize * i), verts.byte_add(0x24 * i), uv);
            }
            dynamic_vertex_buffer_unlock(p8(gu(VB)));
            let size = 2 * num_indices;
            let ib = p8(gu(IB));
            let mut ip: u32 = 0;
            let ok = if gu(IB_OFFSET).wrapping_add(num_indices) > 0xFFFF {
                su(IB_OFFSET, 0);
                ib_lock(ib, 0, size, GPtr::from_raw(&mut ip), 0x2800)
            } else {
                ib_lock(ib, 2 * gu(IB_OFFSET), size, GPtr::from_raw(&mut ip), 0x1800)
            } >= 0;
            if ok {
                for k in 0..size as isize {
                    wr::<u8>(p8(ip), k, rd::<u8>(idx, k));
                }
                ib_unlock(p8(gu(IB)));
                set_stream_source(0, gu(VB), 0, stride);
                if gu(LAST_IB) != gu(IB) {
                    su(LAST_IB, gu(IB));
                    dev_set_indices(device(), gu(IB));
                }
                flush_cache();
                let r = dev_draw_indexed_primitive(device(), prim, gu(BASE_VERTEX), 0, num_verts, gu(IB_OFFSET), count);
                su(IB_OFFSET, gu(IB_OFFSET).wrapping_add(num_indices));
                drawn = r >= 0;
            }
        }
        if !drawn {
            fallback_decl();
            if gu(LAST_IB) != 0 {
                su(LAST_IB, 0);
                dev_set_indices(device(), 0);
            }
            flush_cache();
            dev_draw_indexed_primitive_up(device(), prim, 0, num_verts, count, indices, 0x65, verts.addr() as u32, 0x24);
        }
        return 1;
    }
    count = primitive_count(prim, num_verts, count);
    if lock(num_verts, data_p()) {
        for i in 0..num_verts as isize {
            copy_vertex(p8(dp.read()).byte_add(stride as isize * i), verts.byte_add(0x24 * i), uv);
        }
        dynamic_vertex_buffer_unlock(p8(gu(VB)));
        set_stream_source(0, gu(VB), 0, stride);
        flush_cache();
        if dev_draw_primitive(device(), prim, gu(BASE_VERTEX), count) >= 0 {
            return 1;
        }
    }
    fallback_decl();
    flush_cache();
    dev_draw_primitive_up(device(), prim, count, verts.addr() as u32, 0x24);
    1
}

/// The full-vertex declaration and no stream 0 (the user-pointer fallback).
fn fallback_decl() {
    let d = gu(DECL_FULL);
    if gu(LAST_DECL) != d {
        su(LAST_DECL, d);
        su(LAST_FVF, 0xFFFF_FFFF);
        dev_set_vertex_declaration(device(), d);
    }
    set_stream_source(0, 0, 0, 0);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_IMM_RENDER_PRIMITIVE_NODE];
