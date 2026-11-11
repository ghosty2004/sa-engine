//! PORT-RW-4 module `im2d`: the D3D9 Im2D back end (open/close, line, triangle, primitive, indexed
//! primitive and the shared render-state setup), see docs/arch/rw-port.md (PORT-RW-4). Ported from the
//! 1.0 US bytes (0x7FB480..0x7FC1E2); `docs/re/specs/RW/RwD3D9Im2D.md` as a reading aid.
//!
//! State: 0xC980E0 "own Im2D vertex buffer" flag, 0xC980E4 vertex offset (in vertices), 0xC980E8 the
//! vertex buffer (own, or the dynamic one of the last `RwD3D9DynamicVertexBufferLock`), 0xC980EC index
//! offset, 0xC980F0 index buffer, 0xC980F4 vertex declaration. `RwIm2DVertex` = 0x1C bytes (x, y, z,
//! rhw, colour, u, v); x / y get the frame-buffer raster's `nOffsetX/Y` (+0x1C/+0x1E, i16) added.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, p8, rd, wr};
use crate::d3d9state::device;

const OWN_VB: usize = 0xC980E0;
const VOFF: usize = 0xC980E4;
const VB: usize = 0xC980E8;
const IOFF: usize = 0xC980EC;
const IB: usize = 0xC980F0;
const DECL: usize = 0xC980F4;
const DEV_CAPS: usize = 0xC9BF1C;
const CUR_CAMERA: usize = 0xC9BCC0;
/// RW primitive type -> D3DPRIMITIVETYPE.
const PRIM_TABLE: u32 = 0x884888;
/// Own vertex buffer capacity in vertices (0x40000 / 0x1C).
const VB_VERTS: u32 = 0x2492;
/// Index buffer capacity in indices.
const IB_INDICES: u32 = 0x2710;
const VSIZE: u32 = 0x1C;

orig! {
    /// `IDirect3DDevice9::GetAvailableTextureMem` (+0x10)
    pub fn dev_get_available_texture_mem(dev: GPtr<u8>) -> u32 = stdcall vslot 0x10 => unported;
    /// `IDirect3DDevice9::CreateVertexBuffer` (+0x68)
    pub fn dev_create_vertex_buffer(dev: GPtr<u8>, len: u32, usage: u32, fvf: u32, pool: u32, out: u32, shared: u32) -> i32 = stdcall vslot 0x68 => unported;
    /// `IDirect3DDevice9::CreateIndexBuffer` (+0x6C)
    pub fn dev_create_index_buffer(dev: GPtr<u8>, len: u32, usage: u32, fmt: u32, pool: u32, out: u32, shared: u32) -> i32 = stdcall vslot 0x6C => unported;
    /// `IDirect3DDevice9::DrawPrimitive` (+0x144)
    pub fn dev_draw_primitive(dev: GPtr<u8>, prim: u32, start: u32, count: u32) -> i32 = stdcall vslot 0x144 => unported;
    /// `IDirect3DDevice9::DrawIndexedPrimitive` (+0x148)
    pub fn dev_draw_indexed_primitive(dev: GPtr<u8>, prim: u32, base: u32, min: u32, num: u32, start: u32, count: u32) -> i32 = stdcall vslot 0x148 => unported;
    /// `IDirect3D{Vertex,Index}Buffer9::Lock` (+0x2C)
    pub fn buf_lock(buf: GPtr<u8>, offset: u32, size: u32, data: GPtr<u32>, flags: u32) -> i32 = stdcall vslot 0x2C => unported;
    /// `IDirect3D{Vertex,Index}Buffer9::Unlock` (+0x30)
    pub fn buf_unlock(buf: GPtr<u8>) -> i32 = stdcall vslot 0x30 => unported;
    /// `IUnknown::Release` (+8)
    pub fn com_release(obj: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;

    /// `RwD3D9CreateVertexDeclaration(elements, out)` (0x7FAA30)
    pub fn create_vertex_declaration(elements: GPtr<u8>, out: GPtr<u32>) -> u32 = cdecl 0x7FAA30 => crate::d3d9dev::create_vertex_declaration;
    /// `RwD3D9DeleteVertexDeclaration(decl)` (0x7FAC10)
    pub fn delete_vertex_declaration(decl: u32) = cdecl 0x7FAC10 => crate::d3d9dev::delete_vertex_declaration;
    /// `RwD3D9DynamicVertexBufferLock(vertexSize, num, vbOut, data, baseIndex)` (0x7F5B10)
    pub fn dyn_lock(vsize: u32, num: u32, vb: GPtr<u32>, data: u32, base: GPtr<u32>) -> u32 = cdecl 0x7F5B10 => crate::d3d9vb::dynamic_vertex_buffer_lock;
    /// `_rwD3D9SetVertexDeclaration(decl)` (0x7F9F70)
    pub fn set_vertex_declaration(decl: u32) = cdecl 0x7F9F70 => crate::d3d9dev::set_vertex_declaration;
    /// `_rwD3D9SetVertexShader(vs)` (0x7F9FB0)
    pub fn set_vertex_shader(vs: u32) = cdecl 0x7F9FB0 => crate::d3d9dev::set_vertex_shader;
    /// `_rwD3D9SetPixelShader(ps)` (0x7F9FF0)
    pub fn set_pixel_shader(ps: u32) = cdecl 0x7F9FF0 => crate::d3d9dev::set_pixel_shader;
    /// `RwD3D9SetStreamSource(stream, vb, offset, stride)` (0x7FA030)
    pub fn set_stream_source(stream: u32, vb: u32, offset: u32, stride: u32) = cdecl 0x7FA030 => crate::d3d9dev::set_stream_source;
    /// `_rwD3D9SetIndices(ib)` (0x7FA1C0)
    pub fn set_indices(ib: u32) = cdecl 0x7FA1C0 => crate::d3d9dev::set_indices;
    /// `RwD3D9SetRenderState(state, value)` (0x7FC2D0)
    pub fn set_render_state(state: u32, value: u32) -> u32 = cdecl 0x7FC2D0 => crate::d3d9state::set_render_state;
    /// `RwD3D9SetTextureStageState(stage, type, value)` (0x7FC340)
    pub fn set_tss(stage: u32, ty: u32, value: u32) = cdecl 0x7FC340 => crate::d3d9state::set_texture_stage_state;
    /// `_rwD3D9GetCurrentTexture(stage)` (0x7FDE50)
    pub fn get_current_texture(stage: u32) -> u32 = cdecl 0x7FDE50 => crate::d3d9state::get_current_texture;
    /// `_rwD3D9RenderStateFlushCache()` (0x7FC200)
    pub fn flush_cache() = cdecl 0x7FC200 => crate::d3d9state::flush_cache;
    /// `_rwD3D9Im2DSetRenderStates()` (0x7FB810)
    pub fn set_render_states_call() = cdecl 0x7FB810 => im2d_set_render_states;
    /// `RwRasterGetCurrentContext()` (0x7FAE60)
    pub fn raster_get_current_context() -> GPtr<u8> = cdecl 0x7FAE60 => unported;
}

#[inline]
fn gu(addr: usize) -> u32 {
    g::<u32>(addr).read()
}

#[inline]
fn su(addr: usize, v: u32) {
    g::<u32>(addr).write(v)
}

fn vb() -> GPtr<u8> {
    p8(gu(VB))
}

fn prim(ty: u32) -> u32 {
    g::<u32>(PRIM_TABLE.wrapping_add(ty.wrapping_mul(4)) as usize).read()
}

/// The vertices for `n` new Im2D vertices: the own buffer (wrapping to 0 with DISCARD when they do not
/// fit, else NOOVERWRITE at the current offset) or `RwD3D9DynamicVertexBufferLock`. The data pointer
/// (0 on failure).
fn lock_vertices(n: u32) -> u32 {
    let mut data: u32 = 0;
    if gu(OWN_VB) != 0 {
        let off = gu(VOFF);
        if off.wrapping_add(n) > VB_VERTS {
            su(VOFF, 0);
            buf_lock(vb(), 0, n.wrapping_mul(VSIZE), GPtr::from_raw(&mut data), 0x2800);
        } else {
            buf_lock(vb(), off.wrapping_mul(VSIZE), n.wrapping_mul(VSIZE), GPtr::from_raw(&mut data), 0x1800);
        }
    } else {
        let p: GPtr<u32> = GPtr::from_raw(&mut data);
        dyn_lock(VSIZE, n, g(VB), p.addr() as u32, g(VOFF));
    }
    data
}

/// One vertex with the raster offset added to x / y (`fld offset; fadd x; fstp`), the rest copied.
fn put_vertex(dst: GPtr<u8>, src: GPtr<u8>, ox: X87, oy: X87) {
    wr::<f32>(dst, 0, ox.fadd_m(rd::<f32>(src, 0)).to_f32());
    wr::<f32>(dst, 4, oy.fadd_m(rd::<f32>(src, 4)).to_f32());
    for o in (8..0x1C).step_by(4) {
        wr::<u32>(dst, o, rd::<u32>(src, o));
    }
}

fn copy_vertex(dst: GPtr<u8>, src: GPtr<u8>) {
    for o in (0..0x1C).step_by(4) {
        wr::<u32>(dst, o, rd::<u32>(src, o));
    }
}

/// The frame-buffer raster's offsets (`fild` of the i16 fields).
fn offsets(raster: GPtr<u8>) -> (X87, X87) {
    (
        X87::from_i32(rd::<i16>(raster, 0x1C) as i32),
        X87::from_i32(rd::<i16>(raster, 0x1E) as i32),
    )
}

/// `_rwD3D9Im2DRenderOpen` (0x7FB480): without hardware T&L and with more than 8 MB of texture memory an
/// own vertex buffer (0x40000 bytes, WRITEONLY | DONOTCLIP | DYNAMIC, DEFAULT pool); the index buffer
/// (0x4E20 bytes, INDEX16); the declaration POSITIONT float4 / COLOR / TEXCOORD float2. A failed index
/// buffer releases everything and returns 0.
#[hook(addr = 0x7FB480, conv = cdecl, name = "_rwD3D9Im2DRenderOpen", expect = "A1 1C BF C9 00 83 EC 20")]
pub fn im2d_render_open() -> u32 {
    su(OWN_VB, 0);
    if gu(DEV_CAPS) & 0x1_0000 == 0
        && dev_get_available_texture_mem(device()) > 0x80_0000
        && dev_create_vertex_buffer(device(), 0x4_0000, 0x228, 0, 0, VB as u32, 0) == 0
    {
        su(OWN_VB, 1);
        su(VOFF, 0);
    }
    su(IOFF, 0);
    if dev_create_index_buffer(device(), 0x4E20, 0x228, 0x65, 0, IB as u32, 0) != 0 {
        if gu(DECL) != 0 {
            delete_vertex_declaration(gu(DECL));
            su(DECL, 0);
        }
        if gu(OWN_VB) != 0 && gu(VB) != 0 {
            com_release(vb());
            su(VB, 0);
        }
        if gu(IB) != 0 {
            com_release(p8(gu(IB)));
            su(IB, 0);
        }
        return 0;
    }
    #[rustfmt::skip]
    let mut elements: [u8; 32] = [
        0, 0, 0, 0, 3, 0, 9, 0,
        0, 0, 0x10, 0, 4, 0, 0xA, 0,
        0, 0, 0x14, 0, 1, 0, 5, 0,
        0xFF, 0, 0, 0, 0x11, 0, 0, 0,
    ];
    create_vertex_declaration(GPtr::from_raw(elements.as_mut_ptr()), g(DECL));
    1
}

/// `_rwD3D9Im2DRenderClose` (0x7FB5F0): deletes the declaration, releases the own vertex buffer and the
/// index buffer.
#[hook(addr = 0x7FB5F0, conv = cdecl, name = "_rwD3D9Im2DRenderClose", expect = "A1 F4 80 C9 00 85 C0")]
pub fn im2d_render_close() {
    if gu(DECL) != 0 {
        delete_vertex_declaration(gu(DECL));
        su(DECL, 0);
    }
    if gu(OWN_VB) != 0 && gu(VB) != 0 {
        com_release(vb());
        su(VB, 0);
    }
    if gu(IB) != 0 {
        com_release(p8(gu(IB)));
        su(IB, 0);
    }
}

/// `FUN_007fb810` (`_rwD3D9Im2DSetRenderStates`): the Im2D declaration, no shaders, CLIPPING and LIGHTING
/// off, stage 0 = diffuse (no texture) or texture * diffuse, stream 0 = the Im2D buffer (stride 0x1C),
/// then (tail call) the render-state flush.
#[hook(addr = 0x7FB810, conv = cdecl, name = "FUN_007fb810", expect = "A1 F4 80 C9 00 50")]
pub fn im2d_set_render_states() {
    set_vertex_declaration(gu(DECL));
    set_vertex_shader(0);
    set_pixel_shader(0);
    set_render_state(0x88, 0);
    set_render_state(0x89, 0);
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
    set_stream_source(0, gu(VB), 0, VSIZE);
    flush_cache();
}

/// The camera's frame-buffer raster (`[[0xC9BCC0] + 0x60]`).
fn camera_raster() -> GPtr<u8> {
    p8(rd::<u32>(p8(gu(CUR_CAMERA)), 0x60))
}

/// Unlock, render states, `DrawPrimitive(prim, offset, count)`, offset += `n`; `hr >= 0`.
fn draw(prim_d3d: u32, count: u32, n: u32) -> u32 {
    let _ = buf_unlock(vb());
    set_render_states_call();
    let hr = dev_draw_primitive(device(), prim_d3d, gu(VOFF), count);
    su(VOFF, gu(VOFF).wrapping_add(n));
    (hr >= 0) as u32
}

/// `_rwD3D9Im2DRenderLine` (0x7FB650): vertices `i1` and `i2` as one LINELIST line; 0 when the lock
/// gives no data.
#[hook(addr = 0x7FB650, conv = cdecl, name = "_rwD3D9Im2DRenderLine", expect = "83 EC 08 A1 C0 BC C9 00")]
pub fn im2d_render_line(verts: GPtr<u8>, _num: i32, i1: i32, i2: i32) -> u32 {
    let raster = camera_raster();
    let data = p8(lock_vertices(2));
    if data.is_null() {
        return 0;
    }
    let (ox, oy) = offsets(raster);
    put_vertex(data, verts.byte_add(i1 as isize * 0x1C), ox, oy);
    put_vertex(data.byte_add(0x1C), verts.byte_add(i2 as isize * 0x1C), ox, oy);
    draw(2, 1, 2)
}

/// `_rwD3D9Im2DRenderTriangle` (0x7FB8E0): vertices `i1..i3` as one TRILIST triangle.
#[hook(addr = 0x7FB8E0, conv = cdecl, name = "_rwD3D9Im2DRenderTriangle", expect = "83 EC 08 A1 C0 BC C9 00")]
pub fn im2d_render_triangle(verts: GPtr<u8>, _num: i32, i1: i32, i2: i32, i3: i32) -> u32 {
    let raster = camera_raster();
    let data = p8(lock_vertices(3));
    if data.is_null() {
        return 0;
    }
    let (ox, oy) = offsets(raster);
    put_vertex(data, verts.byte_add(i1 as isize * 0x1C), ox, oy);
    put_vertex(data.byte_add(0x1C), verts.byte_add(i2 as isize * 0x1C), ox, oy);
    put_vertex(data.byte_add(0x38), verts.byte_add(i3 as isize * 0x1C), ox, oy);
    draw(4, 1, 3)
}

/// Primitive count of an RW primitive type for `n` vertices / indices (signed arithmetic as compiled).
fn prim_count(ty: u32, n: i32) -> u32 {
    (match ty {
        1 => n / 2,
        2 => n.wrapping_sub(1),
        3 => n / 3,
        4 | 5 => n.wrapping_sub(2),
        _ => 0,
    }) as u32
}

/// Copies `n` vertices (offsets applied when the raster has any; the `f32` offsets as stored locals).
fn copy_offset(raster: GPtr<u8>, data: GPtr<u8>, n: i32, src: impl Fn(i32) -> GPtr<u8>) {
    if rd::<i16>(raster, 0x1C) == 0 && rd::<i16>(raster, 0x1E) == 0 {
        for i in 0..n.max(0) {
            copy_vertex(data.byte_add(i as isize * 0x1C), src(i));
        }
        return;
    }
    let (ox, oy) = offsets(raster);
    let (ox, oy) = (X87::from_f32(ox.to_f32()), X87::from_f32(oy.to_f32()));
    for i in 0..n.max(0) {
        put_vertex(data.byte_add(i as isize * 0x1C), src(i), ox, oy);
    }
}

/// `_rwD3D9Im2DRenderPrimitive` (0x7FBB00): `num` vertices as one draw (the current raster context when
/// no camera is current).
#[hook(addr = 0x7FBB00, conv = cdecl, name = "_rwD3D9Im2DRenderPrimitive", expect = "83 EC 08 A1 C0 BC C9 00")]
pub fn im2d_render_primitive(ty: u32, verts: GPtr<u8>, num: i32) -> u32 {
    let raster = if gu(CUR_CAMERA) != 0 {
        camera_raster()
    } else {
        raster_get_current_context()
    };
    let count = prim_count(ty, num);
    let data = p8(lock_vertices(num as u32));
    if data.is_null() {
        return 0;
    }
    copy_offset(raster, data, num, |i| verts.byte_add(i as isize * 0x1C));
    draw(prim(ty), count, num as u32)
}

/// `_rwD3D9Im2DRenderIndexedPrimitive` (0x7FBD30): with more indices than vertices the vertices go to the
/// vertex buffer and the indices (at most 0x2710, else 0) to the index buffer for one
/// `DrawIndexedPrimitive`; otherwise the indexed vertices are expanded into the vertex buffer for one
/// `DrawPrimitive`.
#[hook(addr = 0x7FBD30, conv = cdecl, name = "_rwD3D9Im2DRenderIndexedPrimitive", expect = "83 EC 08 A1 C0 BC C9 00")]
pub fn im2d_render_indexed_primitive(ty: u32, verts: GPtr<u8>, num_verts: i32, indices: GPtr<u16>, num_indices: i32) -> u32 {
    let raster = camera_raster();
    let count = prim_count(ty, num_indices);
    if num_indices <= num_verts {
        let data = p8(lock_vertices(num_indices as u32));
        if data.is_null() {
            return 0;
        }
        copy_offset(raster, data, num_indices, |i| {
            let k = indices.wrapping_index(i).read() as isize;
            verts.byte_add(k * 0x1C)
        });
        return draw(prim(ty), count, num_indices as u32);
    }
    let data = p8(lock_vertices(num_verts as u32));
    if data.is_null() {
        return 0;
    }
    copy_offset(raster, data, num_verts, |i| verts.byte_add(i as isize * 0x1C));
    let _ = buf_unlock(vb());
    let ib = p8(gu(IB));
    if ib.is_null() || num_indices > IB_INDICES as i32 {
        return 0;
    }
    let bytes = (num_indices as u32).wrapping_mul(2);
    let mut idata: u32 = 0;
    let ioff = gu(IOFF);
    let hr = if ioff.wrapping_add(num_indices as u32) > IB_INDICES {
        su(IOFF, 0);
        buf_lock(ib, 0, bytes, GPtr::from_raw(&mut idata), 0x2800)
    } else {
        buf_lock(ib, ioff.wrapping_mul(2), bytes, GPtr::from_raw(&mut idata), 0x1800)
    };
    let hr = if hr >= 0 {
        let dst = p8(idata);
        let src: GPtr<u8> = indices.cast();
        for k in 0..bytes as isize {
            wr::<u8>(dst, k, rd::<u8>(src, k));
        }
        let _ = buf_unlock(p8(gu(IB)));
        set_render_states_call();
        set_indices(gu(IB));
        let hr = dev_draw_indexed_primitive(device(), prim(ty), gu(VOFF), 0, num_verts as u32, gu(IOFF), count);
        su(IOFF, gu(IOFF).wrapping_add(num_indices as u32));
        hr
    } else {
        hr
    };
    su(VOFF, gu(VOFF).wrapping_add(num_verts as u32));
    (hr >= 0) as u32
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_IM2D_RENDER_OPEN,
    HOOK_IM2D_RENDER_CLOSE,
    HOOK_IM2D_SET_RENDER_STATES,
    HOOK_IM2D_RENDER_LINE,
    HOOK_IM2D_RENDER_TRIANGLE,
    HOOK_IM2D_RENDER_PRIMITIVE,
    HOOK_IM2D_RENDER_INDEXED_PRIMITIVE,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
