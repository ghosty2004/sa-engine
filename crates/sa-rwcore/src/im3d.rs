//! PORT-RW-1 module `im3d`: IMMEDIATEMODULE (RwIm3D) and the D3D9 Im3D open/close (see docs/arch/rw-port.md).
//! Ported from the 1.0 US bytes; `docs/re/specs/RW/RwIm3D.md` as a reading aid. D3D9 calls stay calls into
//! the original (`orig!`).
//!
//! Module `M = G + [0xC9C078]` (0x74): +0 current transform pipeline, +4/+8/+0xC/+0x10/+0x14/+0x18 current
//! render pipelines (TRILIST, TRIFAN, TRISTRIP, LINELIST, POLYLINE, POINTLIST), +0x1C default transform
//! pipeline, +0x20.. default render pipelines (same order), +0x38 pool (u16 count, +0x3C elements,
//! +0x40 stride), +0x44 stash (flags, +0x48 ltm, +0x64 pipeline, +0x68 primType, +0x6C indices,
//! +0x70 numIndices).
//!
//! The D3D9 draw node `_rwD3D9ImmRenderPrimitiveNode` (0x80E270) is in `d3d9_im3d_render.rs`.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{error1, error2, p8, rd, rw_globals, wr};

sa_core::globals! {
    list IM3D_GLOBALS;
    /// The IMMEDIATEMODULE base cached at open (`G + offset`)
    pub static IM3D_MODULE_BASE @ 0xC97968: u32 = 0;
    /// IMMEDIATEMODULE offset in `RwGlobals`
    pub static IM3D_MODULE_OFFSET @ 0xC9C078: u32 = 0;
    /// IMMEDIATEMODULE open-instance counter
    pub static IM3D_OPEN_COUNT @ 0xC9C07C: i32 = 0;
    /// D3D9 Im3D state: pool for the render node, base vertex, dynamic VB, index offset, index buffer,
    /// vertex declarations (full, pos+color+uv, pos+color)
    pub static D3D9_IM3D_STATE @ 0xC9AB30: [u32; 8] = [0; 8];
}

/// `_RwD3DDevice` (`IDirect3DDevice9*`).
const D3D_DEVICE: usize = 0xC97C28;
/// D3D device caps word (bit 0x10000 = D3DDEVCAPS_HWTRANSFORMANDLIGHT).
const D3D_DEV_CAPS: usize = 0xC9BF1C;
/// Slots of [`D3D9_IM3D_STATE`] (byte offsets from 0xC9AB30).
const IB: isize = 0x10;
const DECL_FULL: isize = 0x14;
const DECL_UV: isize = 0x18;
const DECL_COLOR: isize = 0x1C;
const IB_OFFSET: isize = 0xC;

orig! {
    /// `RxPipelineExecute(pipeline, data, heapReset)` (0x805710)
    pub fn rx_pipeline_execute(p: GPtr<u8>, data: GPtr<u8>, reset: i32) -> GPtr<u8> = cdecl 0x805710 => unported;
    /// `RxHeapGetGlobalHeap()` (0x8052F0)
    pub fn rx_heap_get_global_heap() -> GPtr<u8> = cdecl 0x8052F0 => unported;
    /// `RxPipelineCreate()` (0x8057B0)
    pub fn rx_pipeline_create() -> GPtr<u8> = cdecl 0x8057B0 => unported;
    /// `RxPipelineLock(pipeline)` (0x806990)
    pub fn rx_pipeline_lock(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x806990 => unported;
    /// `RxLockedPipeAddFragment(pipeline, firstIndex, nodeDef, NULL)` (0x806BE0, varargs; as called here)
    pub fn rx_locked_pipe_add_fragment(p: GPtr<u8>, first: u32, node: GPtr<u8>, end: u32) -> GPtr<u8> = cdecl 0x806BE0 => unported;
    /// `RxLockedPipeUnlock(pipeline)` (0x805F40)
    pub fn rx_locked_pipe_unlock(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x805F40 => unported;
    /// `_rxPipelineDestroy(pipeline)` (0x805820)
    pub fn rx_pipeline_destroy(p: GPtr<u8>) = cdecl 0x805820 => unported;
    /// `RxNodeDefinitionGetD3D9ImmInstance()` (0x80E990)
    pub fn node_def_d3d9_imm_instance() -> GPtr<u8> = cdecl 0x80E990 => unported;
    /// `RxNodeDefinitionGetD3D9SubmitNoLight()` (0x80E9A0)
    pub fn node_def_d3d9_submit_no_light() -> GPtr<u8> = cdecl 0x80E9A0 => unported;
    /// `RwD3D9CreateVertexDeclaration(elements, out)` (0x7FAA30)
    pub fn d3d9_create_vertex_declaration(elements: GPtr<u8>, out: GPtr<u32>) -> i32 = cdecl 0x7FAA30 => unported;
    /// `RwD3D9DeleteVertexDeclaration(decl)` (0x7FAC10)
    pub fn d3d9_delete_vertex_declaration(decl: u32) = cdecl 0x7FAC10 => unported;
    /// `IDirect3DDevice9::CreateIndexBuffer` (vtable +0x6C)
    pub fn device_create_index_buffer(dev: GPtr<u8>, len: u32, usage: u32, fmt: u32, pool: u32, out: GPtr<u32>, shared: u32) -> i32 = stdcall vslot 0x6C => unported;
    /// `IUnknown::Release` (vtable +8)
    pub fn com_release(obj: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;

    /// `RwIm3DSetTransformPipeline` (0x7EFAC0)
    pub fn set_transform_pipeline_call(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EFAC0 => set_transform_pipeline;
    /// `RwIm3DSetRenderPipeline` (0x7EFB10)
    pub fn set_render_pipeline_call(p: GPtr<u8>, prim: i32) -> GPtr<u8> = cdecl 0x7EFB10 => set_render_pipeline;
    /// `_rwIm3DCreateTransformPipeline` (0x80A0A0)
    pub fn create_transform_pipeline_call(out: GPtr<u8>) -> u32 = cdecl 0x80A0A0 => create_transform_pipeline;
    /// `_rwIm3DDestroyTransformPipeline` (0x80A110)
    pub fn destroy_transform_pipeline_call(out: GPtr<u8>) = cdecl 0x80A110 => destroy_transform_pipeline;
    /// `_rwIm3DCreateRenderPipelines` (0x80A1A0)
    pub fn create_render_pipelines_call(defaults: GPtr<u8>) -> u32 = cdecl 0x80A1A0 => create_render_pipelines;
    /// `_rwIm3DDestroyRenderPipelines` (0x80A140)
    pub fn destroy_render_pipelines_call(defaults: GPtr<u8>) = cdecl 0x80A140 => destroy_render_pipelines;
    /// `_rwD3D9Im3DRenderOpen` (0x80E020)
    pub fn d3d9_im3d_render_open_call() -> u32 = cdecl 0x80E020 => d3d9_im3d_render_open;
    /// `_rwD3D9Im3DRenderClose` (0x80DFB0)
    pub fn d3d9_im3d_render_close_call() = cdecl 0x80DFB0 => d3d9_im3d_render_close;
}

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// `M = G + [0xC9C078]`.
fn module() -> GPtr<u8> {
    rw_globals().byte_add(IM3D_MODULE_OFFSET.get() as i32 as isize)
}

fn state() -> GPtr<u8> {
    D3D9_IM3D_STATE.ptr().cast()
}

/// Current render-pipeline slot of an RW primitive type (1 LINELIST .. 6 POINTLIST), or None.
fn render_slot(prim: i32) -> Option<isize> {
    match prim {
        1 => Some(0x10),
        2 => Some(0x14),
        3 => Some(0x04),
        4 => Some(0x0C),
        5 => Some(0x08),
        6 => Some(0x18),
        _ => None,
    }
}

/// `RwIm3DTransform` (0x7EF450): more than 0x10000 vertices is error 0x32 (exactly 0x10000 is stored
/// as a u16 0). Flags get VERTEXXYZ | VERTEXRGBA. Returns verts or NULL.
#[hook(addr = 0x7EF450, conv = cdecl, name = "RwIm3DTransform", expect = "8B 44 24 08 83 EC 08")]
pub fn transform(verts: GPtr<u8>, num: u32, ltm: GPtr<u8>, flags: u32) -> GPtr<u8> {
    if num > 0x10000 {
        error1(0x32);
        return GPtr::NULL;
    }
    wr::<u16>(module(), 0x38, num as u16);
    wr::<u32>(module(), 0x3C, a32(verts));
    wr::<u32>(module(), 0x40, 0x24);
    wr::<u32>(module(), 0x48, a32(ltm));
    wr::<u32>(module(), 0x44, flags | 0x18);
    let m = module();
    if rx_pipeline_execute(p8(rd::<u32>(m, 0)), m.byte_add(0x38), 1).is_null() {
        return GPtr::NULL;
    }
    verts
}

/// `RwIm3DEnd` (0x7EF520): 0 without a transformed pool, else zeroes the pool and stash; 1.
#[hook(addr = 0x7EF520, conv = cdecl, name = "RwIm3DEnd", expect = "8B 15 78 C0 C9 00")]
pub fn end() -> u32 {
    let m = module();
    if rd::<u32>(m, 0x3C) == 0 {
        return 0;
    }
    for i in 0..15 {
        wr::<u32>(m, 0x38 + i * 4, 0);
    }
    1
}

/// `RwIm3DRenderIndexedPrimitive` (0x7EF550): LINELIST / TRILIST counts truncated to whole primitives
/// (signed remainder); POINTLIST and invalid types report 0x25 and execute a NULL pipeline.
#[hook(addr = 0x7EF550, conv = cdecl, name = "RwIm3DRenderIndexedPrimitive", expect = "A1 78 C0 C9 00")]
pub fn render_indexed_primitive(prim: i32, indices: GPtr<u8>, num: i32) -> u32 {
    let m = module();
    if rd::<u32>(m, 0x3C) == 0 {
        error1(0x23);
        return 0;
    }
    let st = m.byte_add(0x44);
    wr::<u32>(st, 0x28, a32(indices));
    wr::<u32>(st, 0x20, 0);
    wr::<i32>(st, 0x24, prim);
    wr::<i32>(st, 0x2C, num);
    match prim {
        1 => {
            wr::<u32>(st, 0x20, rd::<u32>(module(), 0x10));
            wr::<i32>(st, 0x2C, num.wrapping_sub(num.wrapping_rem(2)));
        }
        2 => wr::<u32>(st, 0x20, rd::<u32>(module(), 0x14)),
        3 => {
            wr::<u32>(st, 0x20, rd::<u32>(module(), 0x04));
            wr::<i32>(st, 0x2C, num.wrapping_sub(num.wrapping_rem(3)));
        }
        4 => wr::<u32>(st, 0x20, rd::<u32>(module(), 0x0C)),
        5 => wr::<u32>(st, 0x20, rd::<u32>(module(), 0x08)),
        _ => error2(0x25, prim as u32),
    }
    (!rx_pipeline_execute(p8(rd::<u32>(st, 0x20)), st, 0).is_null()) as u32
}

/// `RwIm3DRenderPrimitive` (0x7EF6B0): `RxHeapGetGlobalHeap` first (result unused); the vertex count as
/// the index count; invalid types report 0x25 and execute a NULL pipeline.
#[hook(addr = 0x7EF6B0, conv = cdecl, name = "RwIm3DRenderPrimitive", expect = "8B 0D 24 7B C9 00")]
pub fn render_primitive(prim: i32) -> u32 {
    let have = rd::<u32>(module(), 0x3C) != 0;
    rx_heap_get_global_heap();
    if !have {
        error1(0x23);
        return 0;
    }
    wr::<u32>(module(), 0x64, 0);
    let st = module().byte_add(0x44);
    wr::<i32>(st, 0x24, prim);
    wr::<u32>(st, 0x28, 0);
    wr::<u32>(st, 0x2C, rd::<u16>(module(), 0x38) as u32);
    match prim {
        1..=5 => {
            let slot = render_slot(prim).unwrap_or(0);
            wr::<u32>(st, 0x20, rd::<u32>(module(), slot));
        }
        _ => error2(0x25, prim as u32),
    }
    (!rx_pipeline_execute(p8(rd::<u32>(st, 0x20)), st, 0).is_null()) as u32
}

/// `RwIm3DSetTransformPipeline` (0x7EFAC0): NULL restores the default; returns the current one.
#[hook(addr = 0x7EFAC0, conv = cdecl, name = "RwIm3DSetTransformPipeline", expect = "8B 44 24 04 85 C0")]
pub fn set_transform_pipeline(p: GPtr<u8>) -> GPtr<u8> {
    if !p.is_null() {
        wr::<u32>(module(), 0, a32(p));
    } else {
        let d = rd::<u32>(module(), 0x1C);
        wr::<u32>(module(), 0, d);
    }
    p8(rd::<u32>(module(), 0))
}

/// `RwIm3DSetRenderPipeline` (0x7EFB10): stores p (returns p), or restores the slot's default (returns
/// it); invalid type: error 0x25, NULL.
#[hook(addr = 0x7EFB10, conv = cdecl, name = "RwIm3DSetRenderPipeline", expect = "8B 44 24 04 83 EC 08")]
pub fn set_render_pipeline(p: GPtr<u8>, prim: i32) -> GPtr<u8> {
    let Some(slot) = render_slot(prim) else {
        error2(0x25, prim as u32);
        return GPtr::NULL;
    };
    if !p.is_null() {
        wr::<u32>(module(), slot, a32(p));
        return p;
    }
    let d = rd::<u32>(module(), slot + 0x1C);
    wr::<u32>(module(), slot, d);
    p8(rd::<u32>(module(), slot))
}

/// `cb_IMMEDIATEMODULE_RwEngine_Open` (0x7EFE20): counts first, zeroes the module, creates the default
/// pipelines; on failure destroys both (render first) and uncounts.
#[hook(addr = 0x7EFE20, conv = cdecl, name = "cb_IMMEDIATEMODULE_RwEngine_Open", expect = "8B 44 24 08 8B 0D 24 7B C9 00")]
pub fn module_open(instance: GPtr<u8>, offset: i32, _size: i32) -> GPtr<u8> {
    IM3D_MODULE_OFFSET.set(offset as u32);
    let base = rw_globals().byte_add(offset as isize);
    IM3D_OPEN_COUNT.set(IM3D_OPEN_COUNT.get().wrapping_add(1));
    IM3D_MODULE_BASE.set(a32(base));
    for i in 0..0x1D {
        wr::<u32>(base, i * 4, 0);
    }
    if create_transform_pipeline_call(module().byte_add(0x1C)) != 0
        && create_render_pipelines_call(module().byte_add(0x20)) != 0
    {
        return instance;
    }
    destroy_render_pipelines_call(module().byte_add(0x20));
    destroy_transform_pipeline_call(module().byte_add(0x1C));
    IM3D_OPEN_COUNT.set(IM3D_OPEN_COUNT.get().wrapping_sub(1));
    GPtr::NULL
}

/// Shared body of the two pipeline builders: create, mark plugin 1, lock, one fragment, unlock. On an
/// unlock failure the original destroys the (NULL) unlock result, leaking the pipeline.
fn build_pipeline(node: fn() -> GPtr<u8>) -> GPtr<u8> {
    let p = rx_pipeline_create();
    if p.is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(p, 0x2C, 1);
    let lp = rx_pipeline_lock(p);
    if lp.is_null() {
        rx_pipeline_destroy(p);
        return GPtr::NULL;
    }
    let n = node();
    let p2 = rx_locked_pipe_unlock(rx_locked_pipe_add_fragment(lp, 0, n, 0));
    if p2.is_null() {
        rx_pipeline_destroy(p2);
        return GPtr::NULL;
    }
    p2
}

/// `_rwIm3DCreateTransformPipeline` (0x80A0A0).
#[hook(addr = 0x80A0A0, conv = cdecl, name = "_rwIm3DCreateTransformPipeline", expect = "56 57 E8 09 B7 FF FF")]
pub fn create_transform_pipeline(out: GPtr<u8>) -> u32 {
    let p = build_pipeline(node_def_d3d9_imm_instance);
    if p.is_null() {
        return 0;
    }
    wr::<u32>(out, 0, a32(p));
    set_transform_pipeline_call(p);
    1
}

/// `_rwIm3DDestroyTransformPipeline` (0x80A110).
#[hook(addr = 0x80A110, conv = cdecl, name = "_rwIm3DDestroyTransformPipeline", expect = "56 6A 00 E8 A8 59 FE FF")]
pub fn destroy_transform_pipeline(out: GPtr<u8>) {
    set_transform_pipeline_call(GPtr::NULL);
    let p = p8(rd::<u32>(out, 0));
    if !p.is_null() {
        rx_pipeline_destroy(p);
        wr::<u32>(out, 0, 0);
    }
}

/// The render pipelines are installed for TRILIST, TRIFAN, TRISTRIP, LINELIST, POLYLINE in this order.
const RENDER_TYPES: [i32; 5] = [3, 5, 4, 1, 2];

/// `_rwIm3DCreateRenderPipelines` (0x80A1A0): one pipeline in the five non-point defaults, installed, then
/// (tail call) `_rwD3D9Im3DRenderOpen`'s result.
#[hook(addr = 0x80A1A0, conv = cdecl, name = "_rwIm3DCreateRenderPipelines", expect = "56 57 E8 09 B6 FF FF")]
pub fn create_render_pipelines(defaults: GPtr<u8>) -> u32 {
    let p = build_pipeline(node_def_d3d9_submit_no_light);
    if p.is_null() {
        return 0;
    }
    for i in 0..5 {
        wr::<u32>(defaults, i * 4, a32(p));
    }
    for t in RENDER_TYPES {
        set_render_pipeline_call(p, t);
    }
    d3d9_im3d_render_open_call()
}

/// `_rwIm3DDestroyRenderPipelines` (0x80A140): D3D9 close, restore the five slots, destroy the shared
/// pipeline (`defaults[0]`), clear the five defaults.
#[hook(addr = 0x80A140, conv = cdecl, name = "_rwIm3DDestroyRenderPipelines", expect = "56 57 E8 69 3E 00 00")]
pub fn destroy_render_pipelines(defaults: GPtr<u8>) {
    d3d9_im3d_render_close_call();
    for t in RENDER_TYPES {
        set_render_pipeline_call(GPtr::NULL, t);
    }
    let p = p8(rd::<u32>(defaults, 0));
    if !p.is_null() {
        rx_pipeline_destroy(p);
    }
    for i in 0..5 {
        wr::<u32>(defaults, i * 4, 0);
    }
}

/// Release the index buffer, delete the declarations (pos+color, pos+color+uv, full).
fn close_body() {
    let ib = p8(rd::<u32>(state(), IB));
    if !ib.is_null() {
        com_release(ib);
        wr::<u32>(state(), IB, 0);
    }
    for slot in [DECL_COLOR, DECL_UV, DECL_FULL] {
        let d = rd::<u32>(state(), slot);
        if d != 0 {
            d3d9_delete_vertex_declaration(d);
            wr::<u32>(state(), slot, 0);
        }
    }
}

/// `_rwD3D9Im3DRenderClose` (0x80DFB0).
#[hook(addr = 0x80DFB0, conv = cdecl, name = "_rwD3D9Im3DRenderClose", expect = "A1 40 AB C9 00")]
pub fn d3d9_im3d_render_close() {
    close_body();
}

/// One `D3DVERTEXELEMENT9` {stream, offset, type, method, usage, usageIndex} at element `i` of `buf`.
fn element(buf: &mut [u8; 40], i: usize, offset: u16, ty: u8, usage: u8) {
    let o = i * 8;
    buf[o..o + 2].copy_from_slice(&0u16.to_le_bytes());
    buf[o + 2..o + 4].copy_from_slice(&offset.to_le_bytes());
    buf[o + 4] = ty;
    buf[o + 5] = 0;
    buf[o + 6] = usage;
    buf[o + 7] = 0;
}

/// `D3DDECL_END()` at element `i`.
fn element_end(buf: &mut [u8; 40], i: usize) {
    let o = i * 8;
    buf[o..o + 2].copy_from_slice(&0xFFu16.to_le_bytes());
    buf[o + 2..o + 4].copy_from_slice(&0u16.to_le_bytes());
    buf[o + 4] = 0x11;
    buf[o + 5] = 0;
    buf[o + 6] = 0;
    buf[o + 7] = 0;
}

/// `_rwD3D9Im3DRenderOpen` (0x80E020): the dynamic 16-bit index buffer (0x1FFFE bytes; default pool with
/// hardware T&L, else system memory) and three vertex declarations built in one reused stack buffer
/// (results ignored). A non-zero HRESULT runs the close sequence and returns 0.
#[hook(addr = 0x80E020, conv = cdecl, name = "_rwD3D9Im3DRenderOpen", expect = "A1 1C BF C9 00")]
pub fn d3d9_im3d_render_open() -> u32 {
    let caps = sa_core::mem::GPtr::<u32>::from_addr(D3D_DEV_CAPS).read();
    let dev = p8(sa_core::mem::GPtr::<u32>::from_addr(D3D_DEVICE).read());
    let pool = ((!caps) >> 15) & 2;
    wr::<u32>(state(), IB_OFFSET, 0);
    let ib_cell: GPtr<u32> = state().byte_add(IB).cast();
    if device_create_index_buffer(dev, 0x1FFFE, 0x208, 0x65, pool, ib_cell, 0) != 0 {
        close_body();
        return 0;
    }
    let mut buf = [0u8; 40];
    element(&mut buf, 0, 0, 2, 0);
    element(&mut buf, 1, 0xC, 2, 3);
    element(&mut buf, 2, 0x18, 4, 0xA);
    element(&mut buf, 3, 0x1C, 1, 5);
    element_end(&mut buf, 4);
    d3d9_create_vertex_declaration(GPtr::from_raw(buf.as_mut_ptr()), state().byte_add(DECL_FULL).cast());
    element(&mut buf, 0, 0, 2, 0);
    element(&mut buf, 1, 0xC, 4, 0xA);
    element(&mut buf, 2, 0x10, 1, 5);
    element_end(&mut buf, 3);
    d3d9_create_vertex_declaration(GPtr::from_raw(buf.as_mut_ptr()), state().byte_add(DECL_UV).cast());
    element(&mut buf, 0, 0, 2, 0);
    element(&mut buf, 1, 0xC, 4, 0xA);
    element_end(&mut buf, 2);
    d3d9_create_vertex_declaration(GPtr::from_raw(buf.as_mut_ptr()), state().byte_add(DECL_COLOR).cast());
    1
}

/// `_rwD3D9ImmInstanceNode` (0x80E970): records the pool pointer (the first dword of the params) for the
/// render node; returns 1.
#[hook(addr = 0x80E970, conv = cdecl, name = "_rwD3D9ImmInstanceNode", expect = "8B 44 24 08 8B 08")]
pub fn d3d9_imm_instance_node(_self: GPtr<u8>, params: GPtr<u8>) -> u32 {
    wr::<u32>(state(), 0, rd::<u32>(params, 0));
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_TRANSFORM,
    HOOK_END,
    HOOK_RENDER_INDEXED_PRIMITIVE,
    HOOK_RENDER_PRIMITIVE,
    HOOK_SET_TRANSFORM_PIPELINE,
    HOOK_SET_RENDER_PIPELINE,
    HOOK_MODULE_OPEN,
    HOOK_CREATE_TRANSFORM_PIPELINE,
    HOOK_DESTROY_TRANSFORM_PIPELINE,
    HOOK_CREATE_RENDER_PIPELINES,
    HOOK_DESTROY_RENDER_PIPELINES,
    HOOK_D3D9_IM3D_RENDER_CLOSE,
    HOOK_D3D9_IM3D_RENDER_OPEN,
    HOOK_D3D9_IMM_INSTANCE_NODE,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = IM3D_GLOBALS;
