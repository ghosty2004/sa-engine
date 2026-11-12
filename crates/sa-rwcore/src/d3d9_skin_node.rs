//! The D3D9 skin all-in-one node and its instancing (batch `std10-skin-1`): `_rpD3D9SkinAllInOneNode` (0x7C7B90),
//! the skin mesh instancing (0x7C8D60) with its index helpers (0x756710 index range, 0x756770 triangle sort), the
//! vertex declaration / vertex buffer set-up (0x7C9680), the re-instancing of the non-blended vertex data (0x7C90D0)
//! and the render step without vertex shaders (0x7C85B0). Ported from the 1.0 US bytes (RenderWare has no
//! gta-reversed source). Unverified (docs/verify/HOWTO.md).
//!
//! The resource entry and `RxD3D9ResEntryHeader` layout are those of `d3d9_mesh_instance.rs`; a vertex stream
//! (0x10, at header +0x10 and +0x20): +0 vertex buffer, +4 offset, +8 stride, +0xC u16 geometry flags, +0xE managed,
//! +0xF dynamic lock; header +0x30 use offsets, +0x34 vertex declaration, +0x38 total index count, +0x3C total vertex
//! count. `D3DVERTEXELEMENT9` (8 bytes): +0 stream, +2 offset, +4 type, +5 method, +6 usage, +7 usage index.
//!
//! With the caps sa-game reports (hardware T&L, no vertex shaders) the node blends the vertices on the CPU each frame
//! into a dynamic stream 0 (positions, normals) and keeps a managed stream 1 (prelit colours, texture coordinates):
//! `_rpD3D9SkinGeometryInstance`'s two-stream layout, the SSE blenders of `d3d9_skin_blend.rs`.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, rw_globals, wr, x};

/// Geometry plugin offset of the skin (`RpSkin*`).
const SKIN_GEOMETRY_OFFSET: usize = 0xC978A8;
/// Atomic plugin offset of the skin (`RpHAnimHierarchy*`).
const SKIN_ATOMIC_OFFSET: usize = 0xC978A4;
/// The skin's hardware T&L flag.
const D3D9_SKIN_HWTL: usize = 0xC978E0;
/// The skin's maximum bone count for vertex shaders.
const D3D9_SKIN_MAX_BONES: usize = 0xC978E8;
/// `_rwIntelSSEsupported`.
const CPU_SSE: usize = 0xC980A4;
/// RESOURCESMODULE offset in RwGlobals (the used-entries list pointer is at +0x24).
const RES_MODULE_OFFSET: usize = 0xC9BC58;
/// MESHMODULE offset global (`G + [0xC9B8C0] + 8`: the RW primitive type -> D3D type index bytes).
const MESH_MODULE: usize = 0xC9B8C0;
/// RW primitive type index -> `D3DPRIMITIVETYPE` (dwords).
const PRIM_TYPE_TABLE: usize = 0x882D74;
/// The resource entry's destroy notification.
const DESTROY_NOTIFY: u32 = 0x4C9990;
/// The triangle comparator handed to `qsort`.
const TRIANGLE_COMPARE: u32 = 0x7567A0;
/// The f32 1.0 the weight clamp compares against.
const ONE_ADDR: usize = 0x858624;
/// `D3DDECLTYPE_UNUSED`: the type of the `D3DDECL_END` element.
const DECL_END_TYPE: u8 = 0x11;

orig! {
    /// `IDirect3DVertexBuffer9::Lock` / `Unlock`
    fn vb_lock(vb: GPtr<u8>, offset: u32, size: u32, data: GPtr<u32>, flags: u32) -> i32 = stdcall vslot 0x2C => unported;
    fn vb_unlock(vb: GPtr<u8>) -> i32 = stdcall vslot 0x30 => unported;
    /// `IDirect3DIndexBuffer9::Lock` / `Unlock`
    fn ib_lock(ib: GPtr<u8>, offset: u32, size: u32, data: GPtr<u32>, flags: u32) -> i32 = stdcall vslot 0x2C => unported;
    fn ib_unlock(ib: GPtr<u8>) -> i32 = stdcall vslot 0x30 => unported;
    /// `IDirect3DVertexDeclaration9::GetDeclaration(elements, count)` (+0x10)
    fn decl_get_declaration(decl: GPtr<u8>, elements: GPtr<u8>, count: GPtr<u32>) -> i32 = stdcall vslot 0x10 => unported;

    /// `RwResourcesAllocateResEntry(owner, ownerRef, size, destroyNotify)` (0x807ED0)
    fn allocate_res_entry(owner: u32, owner_ref: GPtr<u32>, size: u32, notify: u32) -> GPtr<u8> = cdecl 0x807ED0 => crate::memory::rw_resources_allocate_res_entry;
    /// `RwResourcesFreeResEntry(entry)` (0x807DE0)
    fn free_res_entry(e: GPtr<u8>) -> u32 = cdecl 0x807DE0 => crate::memory::rw_resources_free_res_entry;
    /// `RwD3D9IndexBufferCreate(numIndices, IDirect3DIndexBuffer9** out)` (0x4C9970)
    fn index_buffer_create(num: u32, out: u32) -> u32 = cdecl 0x4C9970 => crate::d3d9vb::index_buffer_create;
    /// `_rpD3D9IndexRangeMinMax(indices, count, &numVertices, &minVertex)` (0x756710)
    fn index_range_call(indices: GPtr<u8>, n: u32, num_out: GPtr<u32>, min_out: GPtr<u32>) = cdecl 0x756710 => index_range;
    /// `_rpD3D9SortTriangles(indices, count)` (0x756770)
    fn sort_triangles_call(indices: GPtr<u8>, n: u32) = cdecl 0x756770 => sort_triangles;
    /// CRT `qsort(base, num, width, comp)` (0x8247E0)
    fn qsort(base: GPtr<u8>, num: u32, width: u32, comp: u32) = cdecl 0x8247E0 => unported;

    /// `RwD3D9GetCaps()` (0x7FAD20)
    fn get_caps() -> u32 = cdecl 0x7FAD20 => crate::d3d9dev::get_caps;
    /// `RpD3D9GeometryGetUsageFlags(geometry)` (0x7588D0)
    fn geometry_get_usage_flags(geo: GPtr<u8>) -> u32 = cdecl 0x7588D0 => crate::d3d9misc::geometry_get_usage_flags;
    /// `_rpD3D9AddDynamicGeometry(geometry)` (0x74E970)
    fn add_dynamic_geometry(geo: u32) = cdecl 0x74E970 => crate::d3d9misc::add_dynamic_geometry;
    /// `_rpD3D9VertexDeclarationGetSize(type)` (0x7522E0)
    fn decl_get_size(ty: u32) -> u32 = cdecl 0x7522E0 => crate::d3d9misc::vertex_declaration_get_size;
    /// `_rpD3D9FindFormatV2d(texCoords, numVertices)` (0x755AA0)
    fn find_format_v2d(uv: GPtr<u8>, n: i32) -> u32 = cdecl 0x755AA0 => crate::d3d9inst::find_format_v2d;
    /// `RwD3D9CreateVertexDeclaration(elements, &decl)` (0x7FAA30)
    fn create_vertex_declaration(elements: GPtr<u8>, out: GPtr<u32>) -> u32 = cdecl 0x7FAA30 => crate::d3d9dev::create_vertex_declaration;
    /// `RwD3D9CreateVertexBuffer(stride, size, &vb, &offset)` (0x7F5500)
    fn create_vertex_buffer(stride: u32, size: u32, vb: GPtr<u32>, offset: GPtr<u32>) -> u32 = cdecl 0x7F5500 => crate::d3d9vb::create_vertex_buffer;
    /// `RwD3D9DestroyVertexBuffer(stride, size, vb, offset)` (0x7F56A0)
    fn destroy_vertex_buffer(stride: u32, size: u32, vb: u32, offset: u32) = cdecl 0x7F56A0 => crate::d3d9vb::destroy_vertex_buffer;
    /// `RwD3D9DynamicVertexBufferCreate(size, &vb)` (0x7F5A00)
    fn dynamic_vertex_buffer_create(size: u32, out: GPtr<u32>) -> u32 = cdecl 0x7F5A00 => crate::d3d9vb::dynamic_vertex_buffer_create;
    /// `RwD3D9DynamicVertexBufferDestroy(vb)` (0x7F5AE0)
    fn dynamic_vertex_buffer_destroy(vb: u32) = cdecl 0x7F5AE0 => crate::d3d9vb::dynamic_vertex_buffer_destroy;
    /// `RwD3D9DynamicVertexBufferLock(stride, numVertices, &vb, &data, &baseIndex)` (0x7F5B10)
    fn dynamic_vertex_buffer_lock(stride: u32, n: u32, vb: GPtr<u32>, data: u32, base: GPtr<u32>) -> u32 = cdecl 0x7F5B10 => crate::d3d9vb::dynamic_vertex_buffer_lock;
    /// `_rpD3D9SkinUseSoftware(atomic, header)`-like predicate (0x7C89C0; 1 without hardware T&L)
    fn skin_use_software(atomic: u32) -> u32 = cdecl 0x7C89C0 => crate::generated::generated_leaf_logic::g_fun_007c89c0_7c89c0;
    /// `_rpD3D9SkinUseHardware(atomic)`-like predicate (0x7C8A00; vertex shaders with few enough bones)
    fn skin_use_hardware(atomic: u32) -> u32 = cdecl 0x7C8A00 => crate::generated::generated_leaf_logic::g_fun_007c8a00_7c8a00;

    /// The `_rpD3D9VertexDeclarationInst*` writers
    fn inst_v3d(ty: u32, dst: u32, src: u32, n: i32, stride: u32) -> u32 = cdecl 0x752AD0 => crate::d3d9inst::inst_v3d;
    fn inst_v2d(ty: u32, dst: u32, src: u32, n: i32, stride: u32) -> u32 = cdecl 0x7544E0 => crate::d3d9inst::inst_v2d;
    fn inst_color(dst: u32, src: u32, n: i32, stride: u32) -> u32 = cdecl 0x754AE0 => crate::d3d9inst::inst_color;
    fn inst_weights(ty: u32, dst: u32, src: u32, n: i32, stride: u32) -> u32 = cdecl 0x752320 => crate::d3d9inst::inst_weights;
    fn inst_indices_remap(ty: u32, dst: u32, src: u32, remap: u32, n: i32, stride: u32) -> u32 = cdecl 0x754C80 => crate::d3d9inst::inst_indices_remap;
    fn inst_tangent(ty: u32, dst: u32, pos: u32, uv: u32, hdr: u32, stride: u32) -> u32 = cdecl 0x754E20 => crate::d3d9inst::inst_tangent;

    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => crate::frame::get_ltm;
    /// `RwD3D9SetTransformWorld(matrix)` (0x7FA520)
    fn set_transform_world(m: GPtr<u8>) -> u32 = cdecl 0x7FA520 => crate::d3d9dev::set_transform_world;
    /// `RwD3D9SetRenderState(state, value)` (0x7FC2D0)
    fn set_render_state(state: u32, value: u32) -> u32 = cdecl 0x7FC2D0 => crate::d3d9state::set_render_state;
    /// `RwD3D9GetRenderState(state, out)` (0x7FC320)
    fn get_render_state(state: u32, out: GPtr<u32>) = cdecl 0x7FC320 => crate::d3d9state::get_render_state;

    /// The skin mesh instancing (0x7C8D60)
    fn instance_meshes_call(atomic: GPtr<u8>, owner: GPtr<u8>, entry_ref: GPtr<u32>, mesh: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C8D60 => skin_instance_meshes;
    /// The vertex declaration / buffer set-up (0x7C9680)
    fn geometry_instance_call(atomic: GPtr<u8>, header: GPtr<u8>, has_hierarchy: u32) -> u32 = cdecl 0x7C9680 => skin_geometry_instance;
    /// The re-instancing of the non-blended vertex data (0x7C90D0)
    fn reinstance_call(atomic: GPtr<u8>, header: GPtr<u8>, hierarchy: GPtr<u8>, flags: u32) = cdecl 0x7C90D0 => skin_reinstance;
    /// The vertex-shader render step (0x7C8060; reached only with vertex shaders)
    fn render_vs(cbs: GPtr<u8>, atomic: GPtr<u8>, geo: GPtr<u8>, skin: GPtr<u8>, hierarchy: GPtr<u8>) = cdecl 0x7C8060 => unported;
    /// The render step without vertex shaders (0x7C85B0)
    fn render_sw_call(cbs: GPtr<u8>, entry: GPtr<u8>, atomic: GPtr<u8>, flags: u32) = cdecl 0x7C85B0 => skin_render_software;

    /// The skin matrices (0x7CAA30 with SSE, 0x7CA850 without)
    fn prepare_matrices_sse(atomic: GPtr<u8>, skin: GPtr<u8>, hierarchy: GPtr<u8>) -> GPtr<u8> = cdecl 0x7CAA30 => crate::d3d9_skin_blend::skin_prepare_matrices_sse;
    fn prepare_matrices(atomic: GPtr<u8>, skin: GPtr<u8>, hierarchy: GPtr<u8>) -> GPtr<u8> = cdecl 0x7CA850 => unported;
    /// SSE blenders (4 / 2 weights, one bone, rigid per vertex)
    fn blend_sse4(n: u32, w: GPtr<u8>, i: GPtr<u8>, m: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>, dn: GPtr<u8>, sn: GPtr<u8>, stride: u32) = cdecl 0x7CAC60 => crate::d3d9_skin_blend::skin_blend_sse_weights4;
    fn blend_sse2(n: u32, w: GPtr<u8>, i: GPtr<u8>, m: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>, dn: GPtr<u8>, sn: GPtr<u8>, stride: u32) = cdecl 0x7CAF50 => crate::d3d9_skin_blend::skin_blend_sse_weights2;
    fn blend_sse_one(n: u32, m: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>, dn: GPtr<u8>, sn: GPtr<u8>, stride: u32) = cdecl 0x7CAAD0 => crate::d3d9_skin_blend::skin_blend_sse_one_matrix;
    fn blend_sse_rigid(n: u32, i: GPtr<u8>, m: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>, dn: GPtr<u8>, sn: GPtr<u8>, stride: u32) = cdecl 0x7CAB80 => crate::d3d9_skin_blend::skin_blend_sse_rigid;
    /// x87 blenders (4 / 2 weights, one bone, rigid per vertex)
    fn blend_x87_4(n: u32, w: GPtr<u8>, i: GPtr<u8>, m: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>, dn: GPtr<u8>, sn: GPtr<u8>, stride: u32) = cdecl 0x7C9DA0 => unported;
    fn blend_x87_2(n: u32, w: GPtr<u8>, i: GPtr<u8>, m: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>, dn: GPtr<u8>, sn: GPtr<u8>, stride: u32) = cdecl 0x7CA410 => unported;
    fn blend_x87_one(n: u32, m: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>, dn: GPtr<u8>, sn: GPtr<u8>, stride: u32) = cdecl 0x7CA330 => unported;
    fn blend_x87_rigid(n: u32, i: GPtr<u8>, m: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>, dn: GPtr<u8>, sn: GPtr<u8>, stride: u32) = cdecl 0x7CA240 => unported;

    /// The lighting callback `(object)` and the render callback `(entry, object, type, flags)`
    fn call_lighting_cb(f: u32, obj: u32) = cdecl dyn => unported;
    fn call_render_cb(f: u32, e: u32, obj: u32, ty: u32, flags: u32) = cdecl dyn => unported;
}

fn ptr_of<T>(v: &mut T) -> GPtr<u32> {
    GPtr::from_raw(v as *mut T).cast()
}

fn addr(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// `_rwResourcesUseResEntry` inlined: a linked entry moves to the head of the used-resources list.
fn use_res_entry(e: GPtr<u8>) {
    if rd::<u32>(e, 0) == 0 {
        return;
    }
    wr::<u32>(p8(rd::<u32>(e, 4)), 0, rd::<u32>(e, 0));
    wr::<u32>(p8(rd::<u32>(e, 0)), 4, rd::<u32>(e, 4));
    let head = || p8(rd::<u32>(rw_globals().byte_add(g::<u32>(RES_MODULE_OFFSET).read() as i32 as isize), 0x24));
    wr::<u32>(e, 0, rd::<u32>(head(), 0));
    wr::<u32>(e, 4, addr(head()));
    wr::<u32>(p8(rd::<u32>(head(), 0)), 4, addr(e));
    wr::<u32>(head(), 0, addr(e));
}

/// `_rpD3D9IndexRangeMinMax(indices, count, &numVertices, &minVertex)` (0x756710, cdecl): over `count` u16 indices
/// the minimum and `max - min + 1`; both 0 for an empty list.
#[hook(addr = 0x756710, conv = cdecl, name = "FUN_00756710", expect = "57 8B 7C 24 0C 85 FF 74 36 56",
       unverified = "std10-skin-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn index_range(indices: GPtr<u8>, n: u32, num_out: GPtr<u32>, min_out: GPtr<u32>) {
    if n == 0 {
        num_out.write(0);
        min_out.write(0);
        return;
    }
    let (mut lo, mut hi) = (u32::MAX, 0u32);
    for k in 0..n as isize {
        let v = u32::from(rd::<u16>(indices, 2 * k));
        if lo >= v {
            lo = v;
        }
        if hi <= v {
            hi = v;
        }
    }
    num_out.write(hi.wrapping_sub(lo).wrapping_add(1));
    min_out.write(lo);
}

/// `_rpD3D9SortTriangles(indices, count)` (0x756770, cdecl): `qsort` of the `count / 3` triangles (6 bytes each)
/// with the triangle comparator 0x7567A0.
#[hook(addr = 0x756770, conv = cdecl, name = "FUN_00756770", expect = "B8 AB AA AA AA 68 A0 67 75 00",
       unverified = "std10-skin-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn sort_triangles(indices: GPtr<u8>, n: u32) {
    qsort(indices, n / 3, 6, TRIANGLE_COMPARE);
}

/// The primitive count of `n` indices by `D3DPRIMITIVETYPE` (the 0x7C90BC jump table: 2 n/2, 3 n-1, 4 n/3,
/// 5 / 6 n-2, else 0).
fn primitive_count(prim: u32, n: u32) -> u32 {
    match prim {
        2 => n >> 1,
        3 => n.wrapping_sub(1),
        4 => n / 3,
        5 | 6 => n.wrapping_sub(2),
        _ => 0,
    }
}

/// The skin mesh instancing (0x7C8D60, cdecl; `_rpD3D9SkinInstance`-like): `RwResourcesAllocateResEntry(owner,
/// entryRef, 0x40 + 0x24 * meshes, 0x4C9990)`, zeroed; serial and mesh count from the mesh header; the total index
/// count, the index buffer (created and locked when any indices, the lock result unchecked); the primitive type from
/// the mesh module table; both streams cleared and no declaration. Per mesh: index range, primitive count by type,
/// the material, and with a locked buffer the index count, start index and the indices (rebased by the minimum vertex
/// or copied, then sorted for triangle lists); without, both 0. The buffer is unlocked. The skin's bone count for
/// vertex shaders (+0x1C) is its split bone limit (+0x24) when split (+0x34), else its used-bone count (+4); its
/// hardware flag (+0x20) `skin_use_hardware(atomic)` with a hierarchy, else 0. The header's vertex count is the
/// geometry's; `skin_geometry_instance(atomic, header, hierarchy != 0)`; then `skin_reinstance` with every flag
/// (0xFFF) for the hardware path, else positions (2), normals (4 more), prelit (8) and one bit per texture
/// coordinate set (0x10 << i), less the geometry flags already in a non-managed stream 0. Returns the entry (NULL when
/// the allocation fails: the original then dereferences it, not reproduced here: the caller never sees NULL).
#[hook(addr = 0x7C8D60, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "83 EC 10 53 55 56 8B 74 24 2C",
       unverified = "std10-skin-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "the original does not check the entry allocation; the port returns NULL like _rxD3D9Instance")]
pub fn skin_instance_meshes(atomic: GPtr<u8>, owner: GPtr<u8>, entry_ref: GPtr<u32>, mesh: GPtr<u8>) -> GPtr<u8> {
    let num_meshes = u32::from(rd::<u16>(mesh, 4));
    let size = num_meshes * 0x24 + 0x40;
    let e = allocate_res_entry(addr(owner), entry_ref, size, DESTROY_NOTIFY);
    if e.is_null() {
        return GPtr::NULL;
    }
    let h = e.byte_add(0x18);
    for k in 0..size as isize {
        wr::<u8>(h, k, 0);
    }
    wr::<u32>(h, 0, u32::from(rd::<u16>(mesh, 6)));
    wr::<u32>(h, 4, u32::from(rd::<u16>(mesh, 4)));
    let mut locked: u32 = 0;
    wr::<u32>(h, 8, 0);
    wr::<u32>(h, 0x38, 0);
    for k in 0..u32::from(rd::<u16>(mesh, 4)) as isize {
        wr::<u32>(h, 0x38, rd::<u32>(h, 0x38).wrapping_add(rd::<u32>(mesh, 0x14 + 0xC * k)));
    }
    let total = rd::<u32>(h, 0x38);
    if total != 0 && index_buffer_create(total, addr(h.byte_add(8))) != 0 {
        ib_lock(p8(rd::<u32>(h, 8)), 0, 0, ptr_of(&mut locked), 0);
    }
    let pidx = rd::<u8>(rw_globals(), (rd::<u32>(mesh, 0) & 0xFF).wrapping_add(g::<u32>(MESH_MODULE).read()) as isize + 8);
    wr::<u32>(h, 0xC, g::<u32>(PRIM_TYPE_TABLE + 4 * pidx as usize).read());
    for s in 0..2isize {
        let st = h.byte_add(0x10 + 0x10 * s);
        wr::<u32>(st, 0, 0);
        wr::<u32>(st, 4, 0);
        wr::<u32>(st, 8, 0);
        wr::<u8>(st, 0xE, 0);
        wr::<u16>(st, 0xC, 0);
    }
    wr::<u32>(h, 0x34, 0);
    let mut start: u32 = 0;
    let mut m = mesh.byte_add(0x10);
    let mut inst = h.byte_add(0x40);
    for _ in 0..u32::from(rd::<u16>(mesh, 4)) {
        let n = rd::<u32>(m, 4);
        let src = p8(rd::<u32>(m, 0));
        index_range_call(src, n, inst.byte_add(0x18).cast(), inst.byte_add(4).cast());
        wr::<u32>(inst, 0x20, primitive_count(rd::<u32>(h, 0xC), n));
        wr::<u32>(inst, 8, rd::<u32>(m, 8));
        wr::<u32>(inst, 0x10, 0);
        wr::<u32>(inst, 0x14, 0);
        wr::<u32>(inst, 0xC, 0);
        if locked != 0 {
            wr::<u32>(inst, 0, n);
            wr::<u32>(inst, 0x1C, start);
            let dst = p8(locked);
            let min = rd::<u32>(inst, 4);
            if min != 0 {
                for k in 0..n as isize {
                    wr::<u16>(dst, 2 * k, rd::<u16>(src, 2 * k).wrapping_sub(min as u16));
                }
            } else {
                for k in 0..(2 * n) as isize {
                    wr::<u8>(dst, k, rd::<u8>(src, k));
                }
            }
            if rd::<u32>(e, 0x18 + 0xC) == 4 {
                sort_triangles_call(dst, n);
            }
            locked = locked.wrapping_add(2 * n);
            start = start.wrapping_add(n);
        } else {
            wr::<u32>(inst, 0, 0);
            wr::<u32>(inst, 0x1C, 0);
        }
        inst = inst.byte_add(0x24);
        m = m.byte_add(0xC);
    }
    if locked != 0 {
        ib_unlock(p8(rd::<u32>(h, 8)));
    }
    let geo = p8(rd::<u32>(atomic, 0x18));
    let hier = rd::<u32>(atomic, g::<i32>(SKIN_ATOMIC_OFFSET).read() as isize);
    let skin = p8(rd::<u32>(geo, g::<i32>(SKIN_GEOMETRY_OFFSET).read() as isize));
    if !skin.is_null() {
        let bones = if rd::<u32>(skin, 0x34) != 0 { rd::<u32>(skin, 0x24) } else { rd::<u32>(skin, 4) };
        wr::<u32>(skin, 0x1C, bones);
        let hw = if hier != 0 { skin_use_hardware(addr(atomic)) } else { 0 };
        wr::<u32>(skin, 0x20, hw);
    }
    wr::<u32>(h, 0x3C, rd::<u32>(geo, 0x14));
    geometry_instance_call(atomic, h, (hier != 0) as u32);
    let flags = if rd::<u32>(skin, 0x20) != 0 {
        0xFFF
    } else {
        let gf = rd::<u32>(geo, 8);
        let mut f: u32 = if gf & 0x10 != 0 { 6 } else { 2 };
        if gf & 8 != 0 {
            f |= 8;
        }
        for i in 0..rd::<i32>(geo, 0x1C).max(0) as u32 {
            f |= 0x10 << i;
        }
        if rd::<u8>(h, 0x1E) == 0 {
            f &= !u32::from(rd::<u16>(h, 0x1C));
        }
        f
    };
    reinstance_call(atomic, h, p8(hier), flags);
    e
}

/// One `D3DVERTEXELEMENT9` at `els[i]`.
fn element(els: GPtr<u8>, i: u32, stream: u16, offset: u16, ty: u8, usage: u8, index: u8) {
    let el = els.byte_add(8 * i as isize);
    wr::<u16>(el, 0, stream);
    wr::<u16>(el, 2, offset);
    wr::<u8>(el, 4, ty);
    wr::<u8>(el, 5, 0);
    wr::<u8>(el, 6, usage);
    wr::<u8>(el, 7, index);
}

/// The stream (0 or the last) of a vertex component: 0 when the usage flags ask for it in the static stream.
fn stream_for(usage: u32, bit: u32, last: u32) -> u32 {
    if usage & bit != 0 { 0 } else { last }
}

/// The vertex declaration and vertex buffers of a skinned geometry (0x7C9680, cdecl;
/// `_rpD3D9SkinGeometryInstance`-like). Both streams are released (managed: `RwD3D9DestroyVertexBuffer`; dynamic
/// without offset or dynamic lock: `RwD3D9DynamicVertexBufferDestroy`) and cleared. Without usage flags (0xFFF) and
/// on the hardware path, or with `skin_use_software`, one managed stream; else two (stream 0 dynamic, stream 1
/// managed) and `_rpD3D9AddDynamicGeometry`. Hardware path: positions, normals (DEC3N or FLOAT3 by the caps'
/// DeclTypes +0xEC), tangents, colours, texture coordinates (their format from `_rpD3D9FindFormatV2d`), blend weights
/// (by weight count and DeclTypes) and indices; software path: FLOAT3 positions and normals, D3DCOLOR colours, FLOAT2
/// texture coordinates. `RwD3D9CreateVertexDeclaration`; trailing empty streams dropped; one managed stream:
/// `RwD3D9CreateVertexBuffer`, else per stream a managed buffer (with stream offsets, DevCaps2 +0xD4 bit 0), a
/// dynamic lock flag, or `RwD3D9DynamicVertexBufferCreate`. Base indices: with one stream `offset / stride + minVert`
/// and no offsets, else the minimum vertex. Returns 0 when a buffer cannot be created.
#[hook(addr = 0x7C9680, conv = cdecl, name = "FUN_007c9680", expect = "81 EC AC 00 00 00 53 55 56 57",
       unverified = "std10-skin-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn skin_geometry_instance(atomic: GPtr<u8>, header: GPtr<u8>, has_hierarchy: u32) -> u32 {
    let caps = p8(get_caps());
    let geo = p8(rd::<u32>(atomic, 0x18));
    let skin = p8(rd::<u32>(geo, g::<i32>(SKIN_GEOMETRY_OFFSET).read() as isize));
    let gflags = rd::<u32>(geo, 8);
    let usage = geometry_get_usage_flags(geo);
    let num_tex = rd::<u32>(geo, 0x1C);
    let h = header;
    for s in 0..2isize {
        let st = h.byte_add(0x10 + 0x10 * s);
        let vb = rd::<u32>(st, 0);
        if vb != 0 {
            if rd::<u8>(st, 0xE) != 0 {
                let stride = rd::<u32>(st, 8);
                destroy_vertex_buffer(stride, rd::<u32>(h, 0x3C).wrapping_mul(stride), vb, rd::<u32>(st, 4));
            } else if rd::<u32>(st, 4) == 0 && rd::<u8>(st, 0xF) == 0 {
                dynamic_vertex_buffer_destroy(vb);
            }
            wr::<u32>(st, 0, 0);
        }
        wr::<u32>(st, 4, 0);
        wr::<u32>(st, 8, 0);
        wr::<u16>(st, 0xC, 0);
        wr::<u8>(st, 0xE, 0);
        wr::<u8>(st, 0xF, 0);
    }
    let streams = h.byte_add(0x10);
    let st = |s: u32| streams.byte_add(0x10 * s as isize);
    let one = usage & 0xFFF == 0
        && (skin.is_null() || rd::<u32>(skin, 0x20) != 0 || has_hierarchy == 0 || skin_use_software(addr(atomic)) != 0);
    let num_streams: u32 = if one {
        wr::<u8>(streams, 0xE, 1);
        1
    } else {
        wr::<u8>(streams, 0xE, 0);
        wr::<u8>(streams, 0x1E, 1);
        add_dynamic_geometry(addr(geo));
        2
    };
    let last = num_streams.wrapping_sub(1);
    let mut els = [0u8; 0x90];
    let e = GPtr::from_raw(els.as_mut_ptr());
    let mut n: u32;
    let decl_types = || rd::<u32>(caps, 0xEC);
    let add_stride = |s: u32, v: u32| wr::<u32>(st(s), 8, rd::<u32>(st(s), 8).wrapping_add(v));
    let stride_of = |s: u32| rd::<u32>(st(s), 8) as u16;
    if !skin.is_null() && rd::<u32>(skin, 0x20) != 0 {
        let s = stream_for(usage, 2, last);
        element(e, 0, s as u16, stride_of(s), 2, 0, 0);
        wr::<u32>(st(s), 8, 0xC);
        wr::<u16>(st(s), 0xC, 2);
        n = 1;
        if gflags & 0x10 != 0 {
            let s = stream_for(usage, 4, last);
            if decl_types() & 0x80 != 0 {
                element(e, 1, s as u16, stride_of(s), 0xE, 3, 0);
                add_stride(s, 4);
            } else {
                element(e, 1, s as u16, stride_of(s), 2, 3, 0);
                add_stride(s, 0xC);
            }
            wr::<u16>(st(s), 0xC, rd::<u16>(st(s), 0xC) | 4);
            n = 2;
        }
        if usage & 0x1_0000 != 0 {
            let s = stream_for(usage, 0xFF2, last);
            if decl_types() & 0x80 != 0 {
                element(e, n, s as u16, stride_of(s), 0xE, 6, 0);
                add_stride(s, 4);
            } else {
                element(e, n, s as u16, stride_of(s), 2, 6, 0);
                add_stride(s, 0xC);
            }
            n += 1;
        }
        if gflags & 8 != 0 {
            let s = stream_for(usage, 8, last);
            element(e, n, s as u16, stride_of(s), 4, 0xA, 0);
            add_stride(s, 4);
            wr::<u8>(st(s), 0xC, rd::<u8>(st(s), 0xC) | 8);
            n += 1;
        }
        for i in 0..num_tex {
            let s = stream_for(usage, 0x10 << i, last);
            let uv = p8(rd::<u32>(geo, 0x34 + 4 * i as isize));
            let ty = find_format_v2d(uv, rd::<i32>(h, 0x3C)) as u8;
            element(e, n, s as u16, stride_of(s), ty, 5, i as u8);
            add_stride(s, decl_size(ty));
            wr::<u16>(st(s), 0xC, rd::<u16>(st(s), 0xC) | (0x10u32 << i) as u16);
            n += 1;
        }
        let max_w = rd::<u32>(skin, 0x10);
        if max_w > 1 {
            let dt = decl_types();
            let (ty, size): (u8, u32) = if max_w >= 4 || (max_w >= 3 && dt & 0x80 == 0) {
                if dt & 0x20 != 0 {
                    (0xC, 8)
                } else if dt & 8 != 0 {
                    (0xA, 8)
                } else if dt & 2 != 0 {
                    (8, 4)
                } else {
                    (4, 4)
                }
            } else if max_w >= 3 {
                (0xE, 4)
            } else if dt & 0x10 != 0 {
                (0xB, 4)
            } else if dt & 4 != 0 {
                (9, 4)
            } else if dt & 0x80 != 0 {
                (0xE, 4)
            } else if dt & 2 != 0 {
                (8, 4)
            } else {
                (4, 4)
            };
            element(e, n, last as u16, stride_of(last), ty, 1, 0);
            add_stride(last, size);
            n += 1;
        }
        let (ty, size): (u8, u32) = if rd::<u32>(skin, 0x10) >= 3 {
            if rd::<u8>(caps, 0xEC) & 1 != 0 { (5, 4) } else { (7, 8) }
        } else {
            (6, 4)
        };
        element(e, n, last as u16, stride_of(last), ty, 2, 0);
        add_stride(last, size);
        n += 1;
    } else {
        element(e, 0, 0, stride_of(0), 2, 0, 0);
        wr::<u32>(streams, 8, 0xC);
        wr::<u16>(streams, 0xC, 2);
        n = 1;
        if gflags & 0x10 != 0 {
            add_stride(0, 0xC);
            wr::<u16>(streams, 0xC, rd::<u16>(streams, 0xC) | 4);
            element(e, 1, 0, 0xC, 2, 3, 0);
            n = 2;
        }
        if gflags & 8 != 0 {
            let s = stream_for(usage, 8, last);
            element(e, n, s as u16, stride_of(s), 4, 0xA, 0);
            add_stride(s, 4);
            wr::<u8>(st(s), 0xC, rd::<u8>(st(s), 0xC) | 8);
            n += 1;
        }
        for i in 0..num_tex {
            let s = stream_for(usage, 0x10 << i, last);
            element(e, n, s as u16, stride_of(s), 1, 5, i as u8);
            add_stride(s, 8);
            wr::<u16>(st(s), 0xC, rd::<u16>(st(s), 0xC) | (0x10u32 << i) as u16);
            n += 1;
        }
    }
    element(e, n, 0xFF, 0, DECL_END_TYPE, 0, 0);
    create_vertex_declaration(e, h.byte_add(0x34).cast());
    let mut used = num_streams;
    while used > 1 && rd::<u32>(st(used - 1), 8) == 0 {
        used -= 1;
    }
    let num_verts = || rd::<u32>(h, 0x3C);
    let offsets_cap = || rd::<u8>(caps, 0xD4) & 1 != 0;
    let single_managed = used == 1 && rd::<u8>(streams, 0xE) != 0;
    if single_managed {
        let stride = rd::<u32>(streams, 8);
        if create_vertex_buffer(stride, num_verts().wrapping_mul(stride), streams.cast(), streams.byte_add(4).cast()) == 0 {
            return 0;
        }
    } else {
        for s in 0..used {
            let sp = st(s);
            let stride = rd::<u32>(sp, 8);
            let size = num_verts().wrapping_mul(stride);
            if rd::<u8>(sp, 0xE) != 0 && offsets_cap() {
                if create_vertex_buffer(stride, size, sp.cast(), sp.byte_add(4).cast()) == 0 {
                    return 0;
                }
                continue;
            }
            if rd::<u32>(skin, 0x20) == 0 && (offsets_cap() || used == 1) {
                wr::<u8>(sp, 0xF, 1);
            } else if dynamic_vertex_buffer_create(size, sp.cast()) == 0 {
                return 0;
            }
            wr::<u8>(sp, 0xE, 0);
            wr::<u32>(sp, 4, 0);
        }
        if used > 1 {
            if offsets_cap() {
                wr::<u32>(h, 0x30, 1);
            }
            let mut inst = h.byte_add(0x40);
            let mut k = rd::<u32>(h, 4);
            loop {
                wr::<u32>(inst, 0x14, rd::<u32>(inst, 4));
                inst = inst.byte_add(0x24);
                k = k.wrapping_sub(1);
                if k == 0 {
                    break;
                }
            }
            return 1;
        }
    }
    wr::<u32>(h, 0x30, 0);
    let mut inst = h.byte_add(0x40);
    let mut k = rd::<u32>(h, 4);
    loop {
        let base = rd::<u32>(h, 0x14) / rd::<u32>(h, 0x18);
        wr::<u32>(inst, 0x14, base.wrapping_add(rd::<u32>(inst, 4)));
        inst = inst.byte_add(0x24);
        k = k.wrapping_sub(1);
        if k == 0 {
            break;
        }
    }
    1
}

fn decl_size(ty: u8) -> u32 {
    decl_get_size(u32::from(ty))
}

/// The first element with `usage` and `index` (the original scans without a bound).
fn find_element(els: GPtr<u8>, usage: u8, index: u8) -> GPtr<u8> {
    let mut el = els;
    while !(rd::<u8>(el, 6) == usage && rd::<u8>(el, 7) == index) {
        el = el.byte_add(8);
    }
    el
}

/// The re-instancing of a skinned geometry's vertex data (0x7C90D0, cdecl; `_rpD3D9SkinGeometryReinstance`-like):
/// the declaration's elements (`GetDeclaration`) give the stream count; the streams with a requested component (every
/// one for flags 0xFFF) are locked (dynamic stream without offset: DISCARD | NOSYSLOCK over the whole buffer, the
/// stream's flags joining `flags`; else NOSYSLOCK over the geometry's range). Positions (2), on the hardware path with
/// every flag the blend weights (a first weight above 1.0 becomes exactly (1, 0, 0, 0) in the skin) and the bone
/// indices through a remap table (`index * 3`: the split remap, identity up to the vertex-shader bone limit, or the
/// used-bone list), normals (4), tangents (usage 0x10000 with 0xFF2), texture coordinates (0x10 << i) and prelit
/// colours (8, per mesh: the material's vertex alpha) are written; the locked streams are unlocked.
#[hook(addr = 0x7C90D0, conv = cdecl, name = "FUN_007c90d0", expect = "55 8B EC 83 E4 F8 81 EC BC 01",
       unverified = "std10-skin-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "the element buffer is larger than the original's 18 entries (GetDeclaration never writes more here)")]
pub fn skin_reinstance(atomic: GPtr<u8>, header: GPtr<u8>, hierarchy: GPtr<u8>, flags_in: u32) {
    let mut flags = flags_in;
    let geo = p8(rd::<u32>(atomic, 0x18));
    let usage = geometry_get_usage_flags(geo);
    let skin = p8(rd::<u32>(geo, g::<i32>(SKIN_GEOMETRY_OFFSET).read() as isize));
    let num_tex = rd::<u32>(geo, 0x1C);
    let h = header;
    let mut els = [0u8; 65 * 8];
    let e = GPtr::from_raw(els.as_mut_ptr());
    let mut count: u32 = 0;
    decl_get_declaration(p8(rd::<u32>(h, 0x34)), e, ptr_of(&mut count));
    let mut num_streams: u32 = 0;
    let mut el = e;
    while rd::<u8>(el, 4) != DECL_END_TYPE {
        let s = u32::from(rd::<u16>(el, 0));
        if s >= num_streams {
            num_streams = s + 1;
        }
        el = el.byte_add(8);
    }
    let mut locked = [0u32; 2];
    let st = |s: u32| h.byte_add(0x10 + 0x10 * s as isize);
    for i in 0..num_streams {
        let sp = st(i);
        if flags != 0xFFF && flags & u32::from(rd::<u16>(sp, 0xC)) == 0 {
            continue;
        }
        let size = rd::<u32>(sp, 8).wrapping_mul(rd::<u32>(h, 0x3C));
        let slot: GPtr<u32> = GPtr::from_raw(locked.as_mut_ptr()).wrapping_index(i as i32);
        if rd::<u8>(sp, 0xE) == 0 && rd::<u32>(sp, 4) == 0 {
            flags |= u32::from(rd::<u16>(sp, 0xC));
            vb_lock(p8(rd::<u32>(sp, 0)), 0, 0, slot, 0x2800);
        } else {
            vb_lock(p8(rd::<u32>(sp, 0)), rd::<u32>(sp, 4), size, slot, 0x800);
        }
    }
    let dst_of = |el: GPtr<u8>, locked: &[u32; 2]| {
        let s = u32::from(rd::<u16>(el, 0));
        (locked[s as usize].wrapping_add(u32::from(rd::<u16>(el, 2))), rd::<u32>(st(s), 8))
    };
    let mt = p8(rd::<u32>(geo, 0x5C));
    let nv = || rd::<i32>(h, 0x3C);
    if flags & 2 != 0 {
        let el = find_element(e, 0, 0);
        let (dst, stride) = dst_of(el, &locked);
        inst_v3d(u32::from(rd::<u8>(el, 4)), dst, rd::<u32>(mt, 0x14), nv(), stride);
    }
    if !skin.is_null() && rd::<u32>(skin, 0x20) != 0 && flags == 0xFFF {
        let used_bones = rd::<u32>(skin, 4);
        if rd::<u32>(skin, 0x10) > 1 {
            let el = find_element(e, 1, 0);
            let w = p8(rd::<u32>(skin, 0x18));
            for k in 0..rd::<u32>(h, 0x3C) as isize {
                let v = w.byte_add(0x10 * k);
                if rd::<f32>(v, 0) > g::<f32>(ONE_ADDR).read() {
                    wr::<u32>(v, 0, 0x3F80_0000);
                    wr::<u32>(v, 4, 0);
                    wr::<u32>(v, 8, 0);
                    wr::<u32>(v, 0xC, 0);
                }
            }
            let (dst, stride) = dst_of(el, &locked);
            inst_weights(u32::from(rd::<u8>(el, 4)), dst, addr(w), nv(), stride);
        }
        let el = find_element(e, 2, 0);
        let mut remap = [0u8; 0x100];
        if rd::<u32>(skin, 0x34) != 0 {
            let r = p8(rd::<u32>(skin, 0x30));
            if i32::from(rd::<u8>(r, 0)) >= rd::<i32>(hierarchy, 4) {
                wr::<u8>(r, 0, 0);
            }
            let nodes = rd::<u32>(hierarchy, 4);
            for k in 0..nodes {
                let b = rd::<u8>(r, k as isize);
                if b < 0xFF {
                    remap[k as usize & 0xFF] = b.wrapping_mul(3);
                }
            }
        } else {
            let nodes = rd::<u32>(hierarchy, 4);
            if nodes <= g::<u32>(D3D9_SKIN_MAX_BONES).read() {
                for k in 0..nodes {
                    remap[k as usize & 0xFF] = (k as u8).wrapping_mul(3);
                }
            } else {
                remap[0] = 0;
                let list = p8(rd::<u32>(skin, 8));
                for k in 0..used_bones {
                    remap[rd::<u8>(list, k as isize) as usize] = (k as u8).wrapping_mul(3);
                }
            }
        }
        let (dst, stride) = dst_of(el, &locked);
        inst_indices_remap(u32::from(rd::<u8>(el, 4)), dst, rd::<u32>(skin, 0x14), remap.as_ptr() as u32, nv(), stride);
    }
    if rd::<u8>(geo, 8) & 0x10 != 0 && flags & 4 != 0 {
        let el = find_element(e, 3, 0);
        let (dst, stride) = dst_of(el, &locked);
        inst_v3d(u32::from(rd::<u8>(el, 4)), dst, rd::<u32>(mt, 0x18), nv(), stride);
    }
    if usage & 0x1_0000 != 0 && flags & 0xFF2 != 0 {
        let el = find_element(e, 6, 0);
        let (dst, stride) = dst_of(el, &locked);
        let uv = rd::<u32>(geo, 0x30 + 4 * num_tex as isize);
        inst_tangent(u32::from(rd::<u8>(el, 4)), dst, rd::<u32>(mt, 0x14), uv, addr(h), stride);
    }
    if num_tex != 0 && flags & 0xFF0 != 0 {
        for i in 0..num_tex {
            if flags & (0x10 << i) == 0 {
                continue;
            }
            let el = find_element(e, 5, i as u8);
            let (dst, stride) = dst_of(el, &locked);
            inst_v2d(u32::from(rd::<u8>(el, 4)), dst, rd::<u32>(geo, 0x34 + 4 * i as isize), nv(), stride);
        }
    }
    if rd::<u8>(geo, 8) & 8 != 0 && flags & 8 != 0 {
        let el = find_element(e, 0xA, 0);
        let s = u32::from(rd::<u16>(el, 0));
        let off = u32::from(rd::<u16>(el, 2));
        let stride = rd::<u32>(st(s), 8);
        let mut inst = h.byte_add(0x40);
        let mut k = rd::<u32>(h, 4);
        loop {
            let min = rd::<u32>(inst, 4);
            let dst = min.wrapping_mul(stride).wrapping_add(locked[s as usize]).wrapping_add(off);
            let src = rd::<u32>(geo, 0x30).wrapping_add(4 * min);
            let a = inst_color(dst, src, rd::<i32>(inst, 0x18), stride);
            wr::<u32>(inst, 0xC, a);
            inst = inst.byte_add(0x24);
            k = k.wrapping_sub(1);
            if k == 0 {
                break;
            }
        }
    }
    for i in 0..num_streams {
        if locked[i as usize] != 0 {
            vb_unlock(p8(rd::<u32>(st(i), 0)));
        }
    }
}

/// The squared length `(x*x + y*y) + z*z` of a matrix row (x87, stored as f32) within [0.9, 1.1] by the signed
/// integer compare of its bits.
fn row_normalised(m: GPtr<u8>, off: isize) -> bool {
    let (vx, vy, vz) = (x(rd::<f32>(m, off)), x(rd::<f32>(m, off + 4)), x(rd::<f32>(m, off + 8)));
    let l = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).to_f32().to_bits() as i32;
    (0x3F66_6666..=0x3F8C_CCCD).contains(&l)
}

/// The skin render step without vertex shaders (0x7C85B0, cdecl): the lighting callback (`cbs` +4), the atomic's LTM
/// as world transform, NORMALIZENORMALS on when lit and (hardware T&L, or an LTM that is neither identity nor
/// orthonormal with a row not of unit length), then the render callback (`cbs` +0) with type 1. The original keeps the
/// LIGHTING value in its atomic argument slot (not reproduced).
#[hook(addr = 0x7C85B0, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "83 EC 08 53 8B 5C 24 10 56 57",
       unverified = "std10-skin-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn skin_render_software(cbs: GPtr<u8>, entry: GPtr<u8>, atomic: GPtr<u8>, flags: u32) {
    let light_cb = rd::<u32>(cbs, 4);
    if light_cb != 0 {
        call_lighting_cb(light_cb, addr(atomic));
    }
    let ltm = frame_get_ltm(p8(rd::<u32>(atomic, 4)));
    set_transform_world(ltm);
    let mut lighting: u32 = 0;
    get_render_state(0x89, ptr_of(&mut lighting));
    let normalize = lighting != 0
        && (g::<u32>(D3D9_SKIN_HWTL).read() != 0
            || (rd::<u32>(ltm, 0xC) & 0x2_0001 == 0
                && !(row_normalised(ltm, 0) && row_normalised(ltm, 0x10) && row_normalised(ltm, 0x20))));
    set_render_state(0x8F, normalize as u32);
    let render_cb = rd::<u32>(cbs, 0);
    if render_cb != 0 {
        call_render_cb(render_cb, addr(entry), addr(atomic), 1, flags);
    }
}

/// `_rpD3D9SkinAllInOneNode(node, params)` (0x7C7B90, cdecl). Returns 1 for an atomic without vertices or meshes.
/// The resource entry (the geometry's with one morph target, else the atomic's) is freed when its serial differs from
/// the mesh header's; a missing entry is instanced (`skin_instance_meshes`, NULL: returns 0) and the geometry's lock
/// flags cleared, a kept one moves to the head of the used list. With a skin on the software path (+0x20 clear) and a
/// hierarchy: stream 0 is re-instanced first when its managed state does not match `skin_use_software` (lock flags
/// 0xFFF), then locked (managed: its range, NOSYSLOCK; dynamic: DISCARD of a private buffer, or
/// `RwD3D9DynamicVertexBufferLock` with the base indices moved to the new offset unless offsets are used), blended
/// (SSE or x87 by `_rwIntelSSEsupported`: more than two weights, two, one used bone, else rigid) from the morph target's
/// positions and normals and unlocked; lock flags 2 and 4 are cleared. Remaining lock flags re-instance the other
/// data. The atomic's interpolator dirty bit (+0x4C bit 0) is cleared; render through the vertex-shader step (hardware
/// skin) or `skin_render_software`. Returns 1.
#[hook(addr = 0x7C7B90, conv = cdecl, name = "_rpD3D9SkinAllInOneNode", expect = "83 EC 20 8B 54 24 28 8B 44 24",
       unverified = "std10-skin-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "the original keeps the morph target's vertex / normal pointers in its argument slots (locals here)")]
pub fn skin_all_in_one_node(node: GPtr<u8>, params: GPtr<u8>) -> u32 {
    let cbs = p8(rd::<u32>(node, 0x14));
    let atomic = p8(rd::<u32>(params, 0));
    let geo = p8(rd::<u32>(atomic, 0x18));
    if rd::<i32>(geo, 0x14) <= 0 {
        return 1;
    }
    let mesh = p8(rd::<u32>(geo, 0x54));
    if rd::<u16>(mesh, 4) == 0 {
        return 1;
    }
    let single = rd::<u32>(geo, 0x18) == 1;
    let mut entry = if single { p8(rd::<u32>(geo, 0x58)) } else { p8(rd::<u32>(atomic, 0x14)) };
    if !entry.is_null() && rd::<u32>(entry, 0x18) != u32::from(rd::<u16>(mesh, 6)) {
        free_res_entry(entry);
        entry = GPtr::NULL;
    }
    let flags = rd::<u32>(geo, 8);
    if entry.is_null() {
        let (owner, slot) = if rd::<u32>(geo, 0x18) == 1 {
            (geo, geo.byte_add(0x58))
        } else {
            (atomic, atomic.byte_add(0x14))
        };
        entry = instance_meshes_call(atomic, owner, slot.cast(), p8(rd::<u32>(geo, 0x54)));
        if entry.is_null() {
            return 0;
        }
        wr::<u16>(geo, 0xC, 0);
    } else {
        use_res_entry(entry);
    }
    let skin = p8(rd::<u32>(geo, g::<i32>(SKIN_GEOMETRY_OFFSET).read() as isize));
    let hier = p8(rd::<u32>(atomic, g::<i32>(SKIN_ATOMIC_OFFSET).read() as isize));
    if !skin.is_null() && rd::<u32>(skin, 0x20) == 0 && !hier.is_null() {
        let h = entry.byte_add(0x18);
        let mt = p8(rd::<u32>(geo, 0x5C));
        let src_pos = p8(rd::<u32>(mt, 0x14));
        let src_nrm = if rd::<u8>(geo, 8) & 0x10 != 0 { p8(rd::<u32>(mt, 0x18)) } else { GPtr::NULL };
        let mut data: u32 = 0;
        let mut vb: u32;
        let stride: u32;
        if skin_use_software(addr(atomic)) != 0 {
            if rd::<u8>(h, 0x1E) == 0 {
                geometry_instance_call(atomic, h, 1);
                wr::<u16>(geo, 0xC, 0xFFF);
            }
            vb = rd::<u32>(h, 0x10);
            stride = rd::<u32>(h, 0x18);
            vb_lock(p8(vb), rd::<u32>(h, 0x14), rd::<u32>(geo, 0x14).wrapping_mul(stride), ptr_of(&mut data), 0x800);
        } else {
            if rd::<u8>(h, 0x1E) != 0 {
                geometry_instance_call(atomic, h, 1);
                wr::<u16>(geo, 0xC, 0xFFF);
            }
            vb = rd::<u32>(h, 0x10);
            stride = rd::<u32>(h, 0x18);
            if vb != 0 && rd::<u32>(h, 0x14) == 0 && rd::<u8>(h, 0x1F) == 0 {
                vb_lock(p8(vb), 0, 0, ptr_of(&mut data), 0x2800);
            } else {
                let mut base: u32 = 0;
                dynamic_vertex_buffer_lock(stride, rd::<u32>(geo, 0x14), ptr_of(&mut vb), ptr_of(&mut data).addr() as u32, ptr_of(&mut base));
                wr::<u32>(h, 0x10, vb);
                let offset = base.wrapping_mul(stride);
                wr::<u32>(h, 0x14, offset);
                if rd::<u32>(h, 0x30) == 0 {
                    let b = offset / stride;
                    let mut inst = h.byte_add(0x40);
                    let mut k = rd::<u32>(h, 4);
                    loop {
                        wr::<u32>(inst, 0x14, rd::<u32>(inst, 4).wrapping_add(b));
                        inst = inst.byte_add(0x24);
                        k = k.wrapping_sub(1);
                        if k == 0 {
                            break;
                        }
                    }
                }
            }
            wr::<u16>(geo, 0xC, rd::<u16>(geo, 0xC) | rd::<u16>(h, 0x1C));
        }
        if data != 0 {
            let dst = p8(data);
            let dst_nrm = if src_nrm.is_null() { GPtr::NULL } else { dst.byte_add(0xC) };
            let n = rd::<u32>(geo, 0x14);
            let w = p8(rd::<u32>(skin, 0x18));
            let idx = p8(rd::<u32>(skin, 0x14));
            let sse = g::<u32>(CPU_SSE).read() != 0;
            let m = if sse { prepare_matrices_sse(atomic, skin, hier) } else { prepare_matrices(atomic, skin, hier) };
            let max_w = rd::<u32>(skin, 0x10);
            if max_w > 2 {
                if sse {
                    blend_sse4(n, w, idx, m, dst, src_pos, dst_nrm, src_nrm, stride);
                } else {
                    blend_x87_4(n, w, idx, m, dst, src_pos, dst_nrm, src_nrm, stride);
                }
            } else if max_w > 1 {
                if sse {
                    blend_sse2(n, w, idx, m, dst, src_pos, dst_nrm, src_nrm, stride);
                } else {
                    blend_x87_2(n, w, idx, m, dst, src_pos, dst_nrm, src_nrm, stride);
                }
            } else if rd::<u32>(skin, 4) == 1 {
                let one = m.byte_add((u32::from(rd::<u8>(p8(rd::<u32>(skin, 8)), 0)) << 6) as isize);
                if sse {
                    blend_sse_one(n, one, dst, src_pos, dst_nrm, src_nrm, stride);
                } else {
                    blend_x87_one(n, one, dst, src_pos, dst_nrm, src_nrm, stride);
                }
            } else if sse {
                blend_sse_rigid(n, idx, m, dst, src_pos, dst_nrm, src_nrm, stride);
            } else {
                blend_x87_rigid(n, idx, m, dst, src_pos, dst_nrm, src_nrm, stride);
            }
            vb_unlock(p8(vb));
        }
        wr::<u8>(geo, 0xC, rd::<u8>(geo, 0xC) & 0xF9);
    }
    let lock = rd::<u16>(geo, 0xC);
    if lock != 0 {
        reinstance_call(atomic, entry.byte_add(0x18), hier, u32::from(lock));
        wr::<u16>(geo, 0xC, 0);
    }
    wr::<u32>(atomic, 0x4C, rd::<u32>(atomic, 0x4C) & !1);
    if !skin.is_null() && rd::<u32>(skin, 0x20) != 0 && !hier.is_null() {
        render_vs(cbs.byte_add(8), atomic, geo, skin, hier);
        return 1;
    }
    render_sw_call(cbs, entry, atomic, flags);
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_INDEX_RANGE,
    HOOK_SORT_TRIANGLES,
    HOOK_SKIN_INSTANCE_MESHES,
    HOOK_SKIN_GEOMETRY_INSTANCE,
    HOOK_SKIN_REINSTANCE,
    HOOK_SKIN_RENDER_SOFTWARE,
    HOOK_SKIN_ALL_IN_ONE_NODE,
];
