//! PORT-RW-4 module `skin`: RpSkin and its D3D9 pipeline (see docs/arch/rw-port.md, PORT-RW-4).
//! Ported from the 1.0 US bytes; the specs under `docs/re/specs/RW/` are reading aids.
//!
//! Plugin offsets: engine 0xC978A0, atomic 0xC978A4 (the `RpHAnimHierarchy*`), geometry 0xC978A8 (the
//! `RpSkin*`). Skin freelist `[0xC978B8]` (create parameters 0x8DECE0 / 0x8DECE4). Skin pipelines
//! 0xC978C4..0xC978D4 (only [0] is created on D3D9). D3D9 skin state: 0xC978E0 hardware T&L, 0xC978E4 vertex
//! shaders usable, 0xC978E8 max bones per mesh, 0xC978EC max vertex shader constants.
//!
//! `RpSkin` (0x40, freelist): +0 numBones, +4 numBoneIndices used, +8 bone index list (also the heap
//! block), +0xC inverse matrices (16-aligned), +0x10 max weights per vertex, +0x14 vertex bone indices
//! (u8[4] each), +0x18 vertex weights (f32[4] each), +0x24 split boneLimit, +0x28 numMeshes, +0x2C numRLE,
//! +0x30 split block (remap indices), +0x34 RLE counts, +0x38 RLE, +0x3C data block.
//!
//! Not ported (left to the original): `_rpD3D9SkinAllInOneNode` (0x7C7B90) (see the hand-back notes).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{f32_of, g, p8, rd, rw_free, rw_malloc, rw_memory_alloc, rw_memory_free, wr, x};

/// Engine plugin offset.
const SKIN_ENGINE_OFFSET: usize = 0xC978A0;
/// Atomic plugin offset (the HAnim hierarchy pointer).
const SKIN_ATOMIC_OFFSET: usize = 0xC978A4;
/// Geometry plugin offset (the `RpSkin*`).
const SKIN_GEOMETRY_OFFSET: usize = 0xC978A8;
/// Skin freelist.
const SKIN_FREELIST: usize = 0xC978B8;
/// Skin pipelines (4 slots).
const SKIN_PIPELINES: usize = 0xC978C4;
const SKIN_PIPELINES_END: usize = 0xC978D4;
/// Freelist create parameters.
const FL_BLOCK_SIZE: usize = 0x8DECE0;
const FL_PREALLOC: usize = 0x8DECE4;
/// D3D9 skin state.
const D3D9_SKIN_HWTL: usize = 0xC978E0;
const D3D9_SKIN_VS: usize = 0xC978E4;
const D3D9_SKIN_MAX_BONES: usize = 0xC978E8;
const D3D9_SKIN_MAX_VS_CONST: usize = 0xC978EC;
/// `RxNodeDefinition` of the D3D9 skin all-in-one node.
const D3D9_SKIN_NODE_DEF: u32 = 0x8DED08;
/// rwID_SKINPLUGIN
const SKIN_ID: u32 = 0x116;
/// Allocation hint.
const HINT: u32 = 0x30116;

orig! {
    /// `RwEngineRegisterPlugin(size, id, open, close)` (0x7F2BB0)
    fn rw_engine_register_plugin(size: i32, id: u32, open: u32, close: u32) -> i32 = cdecl 0x7F2BB0 => unported;
    /// `RwEngineGetPluginOffset(id)` (0x7F2BE0)
    fn rw_engine_get_plugin_offset(id: u32) -> i32 = cdecl 0x7F2BE0 => unported;
    /// `RpAtomicRegisterPlugin(size, id, ctor, dtor, copy)` (0x74BDA0)
    fn rp_atomic_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74BDA0 => unported;
    /// `RpAtomicRegisterPluginStream(id, read, write, getSize)` (0x74BE00)
    fn rp_atomic_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x74BE00 => unported;
    /// `RpAtomicSetStreamAlwaysCallBack(id, cb)` (0x74BE30)
    fn rp_atomic_set_stream_always_callback(id: u32, cb: u32) -> i32 = cdecl 0x74BE30 => unported;
    /// `RpAtomicSetStreamRightsCallBack(id, cb)` (0x74BE50)
    fn rp_atomic_set_stream_rights_callback(id: u32, cb: u32) -> i32 = cdecl 0x74BE50 => unported;
    /// `RpGeometryRegisterPlugin(size, id, ctor, dtor, copy)` (0x74CD70)
    fn rp_geometry_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74CD70 => unported;
    /// `RpGeometryRegisterPluginStream(id, read, write, getSize)` (0x74CDA0)
    fn rp_geometry_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x74CDA0 => unported;
    /// Bone-usage scan of `RpSkinCreate` (0x7C70C0): (skin, indices, weights, usedList, &usedCount, numVertices)
    fn skin_find_used_bones(skin: GPtr<u8>, idx: GPtr<u8>, w: GPtr<u8>, used: GPtr<u8>, count: GPtr<u32>, n: u32) = cdecl 0x7C70C0 => unported;
    /// CRT `qsort(base, n, size, cmp)` (0x8247E0)
    fn qsort(base: GPtr<u8>, n: u32, size: u32, cmp: u32) = cdecl 0x8247E0 => unported;
    /// `RwD3D9GetCaps()` (0x7FAD20)
    fn rw_d3d9_get_caps() -> GPtr<u8> = cdecl 0x7FAD20 => unported;
    /// `RxPipelineCreate()` (0x8057B0)
    fn rx_pipeline_create() -> GPtr<u8> = cdecl 0x8057B0 => unported;
    /// `RxPipelineLock(pipeline)` (0x806990)
    fn rx_pipeline_lock(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x806990 => unported;
    /// `RxLockedPipeAddFragment(pipeline, firstIndex, nodeDef, NULL)` (0x806BE0, varargs; as called here)
    fn rx_locked_pipe_add_fragment(p: GPtr<u8>, first: u32, node: GPtr<u8>, end: u32) -> GPtr<u8> = cdecl 0x806BE0 => unported;
    /// `RxLockedPipeUnlock(pipeline)` (0x805F40)
    fn rx_locked_pipe_unlock(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x805F40 => unported;
    /// `_rxPipelineDestroy(pipeline)` (0x805820)
    fn rx_pipeline_destroy(p: GPtr<u8>) = cdecl 0x805820 => unported;

    /// `_rpSkinDeinitialize(geometry)` (0x7C8820)
    fn skin_deinitialize_call(geom: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C8820 => rp_skin_deinitialize;
    /// `_rpSkinInitialize(geometry)` (0x7C8740)
    fn skin_initialize_call(geom: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C8740 => rp_skin_initialize;
    /// `_rpSkinSplitDataDestroy(skin)` (0x7C8B10)
    fn split_data_destroy_call(skin: GPtr<u8>) -> u32 = cdecl 0x7C8B10 => rp_skin_split_data_destroy;
    /// `_rpSkinAtomicSetPipeline(atomic, type)` (0x7C89B0)
    fn atomic_set_pipeline_call(atomic: GPtr<u8>, ty: u32) -> GPtr<u8> = cdecl 0x7C89B0 => rp_skin_atomic_set_pipeline;
    /// `RpSkinGetSkinToBoneMatrices(skin)` (0x7C7810)
    fn get_skin_to_bone_matrices_call(skin: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7810 => skin_to_bone_ptr;
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn rw_frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `RwMatrixMultiply(dst, a, b)` (0x7F18B0)
    fn rw_matrix_multiply(dst: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18B0 => crate::matrix::rw_matrix_multiply;
    /// `_rwD3D9VSGetInverseWorldMatrix(out)` (0x7647B0)
    fn vs_get_inverse_world_matrix(out: GPtr<u8>) = cdecl 0x7647B0 => crate::d3d9vsmat::vs_get_inverse_world_matrix;
    /// `_rpD3D9SkinPipelineCreate()` (0x7CB2B0)
    fn d3d9_skin_pipeline_create_call() -> GPtr<u8> = cdecl 0x7CB2B0 => rp_d3d9_skin_pipeline_create;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn skin_to_bone_ptr(skin: GPtr<u8>) -> GPtr<u8> {
    p8(rp_skin_get_skin_to_bone_matrices(skin))
}

/// Last atomic / frame / hierarchy of `_rpD3D9SkinVertexShaderMatrixUpdate`.
const VS_CACHE_HIER: usize = 0xC978D4;
const VS_CACHE_FRAME: usize = 0xC978D8;
const VS_CACHE_ATOMIC_FRAME: usize = 0xC978DC;

/// A 16-byte aligned local RwMatrix.
#[repr(C, align(16))]
struct Mat([u32; 16]);

impl Mat {
    fn p(&mut self) -> GPtr<u8> {
        GPtr::from_raw(self.0.as_mut_ptr()).cast()
    }
}

/// Writes the 3x4 transpose of `a` (right/up/at/pos columns) as 12 dwords at `out`.
fn store_3x4(out: GPtr<u8>, a: GPtr<u8>) {
    for r in 0..3isize {
        for c in 0..4isize {
            wr::<u32>(out, (r * 4 + c) * 4, rd::<u32>(a, (c * 4 + r) * 4));
        }
    }
}

fn freelist() -> GPtr<u8> {
    p8(g::<u32>(SKIN_FREELIST).read())
}

fn geom_skin_slot(geom: GPtr<u8>) -> GPtr<u8> {
    geom.byte_add(g::<i32>(SKIN_GEOMETRY_OFFSET).read() as isize)
}

/// Byte copy `n` bytes (`rep movs` dword part, then the byte tail; no overlap in the callers).
fn copy_bytes(dst: GPtr<u8>, src: GPtr<u8>, n: u32) {
    let dw = n >> 2;
    for k in 0..dw as isize {
        wr::<u32>(dst, k * 4, rd::<u32>(src, k * 4));
    }
    for k in (dw * 4) as isize..n as isize {
        wr::<u8>(dst, k, rd::<u8>(src, k));
    }
}

/// `RpSkinSetFreeListCreateParams(blockSize, numBlocksToPrealloc)` (0x7C67F0). EAX = blockSize.
#[hook(addr = 0x7C67F0, conv = cdecl, name = "RpSkinSetFreeListCreateParams", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn rp_skin_set_free_list_create_params(block: u32, prealloc: u32) -> u32 {
    g::<u32>(FL_BLOCK_SIZE).write(block);
    g::<u32>(FL_PREALLOC).write(prealloc);
    block
}

/// `_rpD3D9SkinVertexShaderMatrixUpdate(out, atomic, skin)` (0x7C78A0): skipped when hierarchy, render
/// frame (`~(u16)G+8`) and atomic frame equal the cached ones (else cached). Per node, out (0x30 each,
/// the 3x4 transpose) = NOMATRICES: (skinToBone[i] * LTM(node frame)) * inverseWorld; LOCALSPACE:
/// skinToBone[i] * matrices[i]; else (skinToBone[i] * matrices[i]) * inverseWorld.
#[hook(addr = 0x7C78A0, conv = cdecl, name = "_rpD3D9SkinVertexShaderMatrixUpdate", expect = "8B 44 24 08 8B 0D A4 78 C9 00")]
pub fn rp_d3d9_skin_vertex_shader_matrix_update(out: GPtr<u8>, atomic: GPtr<u8>, skin: GPtr<u8>) {
    let h = p8(rd::<u32>(atomic, g::<i32>(SKIN_ATOMIC_OFFSET).read() as isize));
    if h.is_null() {
        return;
    }
    let frame = !(rd::<u16>(crate::common::rw_globals(), 8) as u32);
    if g::<u32>(VS_CACHE_HIER).read() == h.addr() as u32
        && g::<u32>(VS_CACHE_FRAME).read() == frame
        && g::<u32>(VS_CACHE_ATOMIC_FRAME).read() == rd::<u32>(atomic, 4)
    {
        return;
    }
    g::<u32>(VS_CACHE_HIER).write(h.addr() as u32);
    g::<u32>(VS_CACHE_FRAME).write(frame);
    g::<u32>(VS_CACHE_ATOMIC_FRAME).write(rd::<u32>(atomic, 4));
    let mut bone = get_skin_to_bone_matrices_call(skin);
    let flags = rd::<u32>(h, 0);
    let (mut a, mut l, mut inv) = (Mat([0; 16]), Mat([0; 16]), Mat([0; 16]));
    let mut o = out;
    if flags & 2 != 0 {
        let mut info = p8(rd::<u32>(h, 0x10)).byte_add(0xC);
        vs_get_inverse_world_matrix(inv.p());
        let mut i = 0i32;
        while i < rd::<i32>(h, 4) {
            let ltm = rw_frame_get_ltm(p8(rd::<u32>(info, 0)));
            rw_matrix_multiply(l.p(), bone, ltm);
            rw_matrix_multiply(a.p(), l.p(), inv.p());
            store_3x4(o, a.p());
            i += 1;
            info = info.byte_add(0x10);
            bone = bone.byte_add(0x40);
            o = o.byte_add(0x30);
        }
        return;
    }
    let mut mats = p8(rd::<u32>(h, 8));
    if flags & 0x4000 != 0 {
        let mut i = 0i32;
        while i < rd::<i32>(h, 4) {
            rw_matrix_multiply(a.p(), bone, mats);
            store_3x4(o, a.p());
            i += 1;
            bone = bone.byte_add(0x40);
            mats = mats.byte_add(0x40);
            o = o.byte_add(0x30);
        }
        return;
    }
    vs_get_inverse_world_matrix(inv.p());
    let mut i = 0i32;
    while i < rd::<i32>(h, 4) {
        rw_matrix_multiply(l.p(), bone, mats);
        rw_matrix_multiply(a.p(), l.p(), inv.p());
        store_3x4(o, a.p());
        i += 1;
        bone = bone.byte_add(0x40);
        mats = mats.byte_add(0x40);
        o = o.byte_add(0x30);
    }
}

/// `RpSkinPluginAttach()` (0x7C6820): engine plugin (0, open 0x7C68E0, close 0x7C69A0), atomic plugin (4,
/// 0x7C6A80/0x7C6AA0/0x7C6AC0) + stream (0x7C72A0/0x7C7500/0x7C7510), always 0x7C6AE0, rights 0x7C6B70,
/// geometry plugin (4, 0x7C69F0/0x7C6A10/0x7C6A70) + stream (0x7C6D20/0x7C6C30/0x7C6BE0). No error checks;
/// returns 1.
#[hook(addr = 0x7C6820, conv = cdecl, name = "RpSkinPluginAttach", expect = "68 A0 69 7C 00")]
pub fn rp_skin_plugin_attach() -> u32 {
    let e = rw_engine_register_plugin(0, SKIN_ID, 0x7C68E0, 0x7C69A0);
    g::<i32>(SKIN_ENGINE_OFFSET).write(e);
    let a = rp_atomic_register_plugin(4, SKIN_ID, 0x7C6A80, 0x7C6AA0, 0x7C6AC0);
    g::<i32>(SKIN_ATOMIC_OFFSET).write(a);
    rp_atomic_register_plugin_stream(SKIN_ID, 0x7C72A0, 0x7C7500, 0x7C7510);
    rp_atomic_set_stream_always_callback(SKIN_ID, 0x7C6AE0);
    rp_atomic_set_stream_rights_callback(SKIN_ID, 0x7C6B70);
    let gm = rp_geometry_register_plugin(4, SKIN_ID, 0x7C69F0, 0x7C6A10, 0x7C6A70);
    g::<i32>(SKIN_GEOMETRY_OFFSET).write(gm);
    rp_geometry_register_plugin_stream(SKIN_ID, 0x7C6D20, 0x7C6C30, 0x7C6BE0);
    1
}

/// `RpSkinAtomicSetHAnimHierarchy(atomic, hierarchy)` (0x7C7520). EAX = atomic.
#[hook(addr = 0x7C7520, conv = cdecl, name = "RpSkinAtomicSetHAnimHierarchy", expect = "8B 44 24 04 8B 15 A4 78 C9 00")]
pub fn rp_skin_atomic_set_hanim_hierarchy(atomic: GPtr<u8>, h: u32) -> GPtr<u8> {
    wr::<u32>(atomic, g::<i32>(SKIN_ATOMIC_OFFSET).read() as isize, h);
    atomic
}

/// `RpSkinAtomicGetHAnimHierarchy(atomic)` (0x7C7540).
#[hook(addr = 0x7C7540, conv = cdecl, name = "RpSkinAtomicGetHAnimHierarchy", expect = "8B 44 24 04 8B 0D A4 78 C9 00")]
pub fn rp_skin_atomic_get_hanim_hierarchy(atomic: GPtr<u8>) -> u32 {
    rd::<u32>(atomic, g::<i32>(SKIN_ATOMIC_OFFSET).read() as isize)
}

/// `RpSkinGeometryGetSkin(geometry)` (0x7C7550).
#[hook(addr = 0x7C7550, conv = cdecl, name = "RpSkinGeometryGetSkin", expect = "8B 44 24 04 8B 0D A8 78 C9 00")]
pub fn rp_skin_geometry_get_skin(geom: GPtr<u8>) -> u32 {
    rd::<u32>(geom, g::<i32>(SKIN_GEOMETRY_OFFSET).read() as isize)
}

/// `RpSkinGeometrySetSkin(geometry, skin)` (0x7C7560): a different skin deinitialises the old one (if
/// any), is stored, and (non-NULL) initialised; `_rpSkinInitialize` failing returns NULL. Returns geometry.
#[hook(addr = 0x7C7560, conv = cdecl, name = "RpSkinGeometrySetSkin", expect = "A1 A8 78 C9 00")]
pub fn rp_skin_geometry_set_skin(geom: GPtr<u8>, skin: GPtr<u8>) -> GPtr<u8> {
    let old = rd::<u32>(geom_skin_slot(geom), 0);
    if skin.addr() as u32 == old {
        return geom;
    }
    if old != 0 {
        skin_deinitialize_call(geom);
    }
    wr::<u32>(geom_skin_slot(geom), 0, skin.addr() as u32);
    if !skin.is_null() && skin_initialize_call(geom).is_null() {
        return GPtr::NULL;
    }
    geom
}

/// `RpSkinCreate(numVertices, numBones, weights, indices, inverseMatrices)` (0x7C75B0): skin from the
/// freelist (no NULL check) zeroed, max weights = 1 + the leading non-zero weight dwords per vertex
/// (at most 4); the used-bone scan 0x7C70C0 into a 0x100-byte stack list; one zeroed heap block of
/// `numBones*64 + numVertices*20 + used + 15` bytes: used list, 16-aligned matrices, indices, weights;
/// matrices copied last-to-first, then indices and weights. Heap failure frees the skin, NULL.
#[hook(addr = 0x7C75B0, conv = cdecl, name = "RpSkinCreate", expect = "81 EC 04 01 00 00")]
pub fn rp_skin_create(nverts: u32, nbones: u32, weights: GPtr<u8>, indices: GPtr<u8>, inv: GPtr<u8>) -> GPtr<u8> {
    let mut used_count = 0u32;
    let mut used = [0u8; 0x100];
    let skin = rw_memory_alloc(freelist(), HINT);
    for k in 0..16 {
        wr::<u32>(skin, k * 4, 0);
    }
    wr::<u32>(skin, 0x10, 1);
    'verts: for v in 0..nverts {
        let mut k = rd::<u32>(skin, 0x10);
        if k >= 4 {
            continue;
        }
        while rd::<u32>(weights, ((k + v * 4) * 4) as isize) != 0 {
            let m = rd::<u32>(skin, 0x10).wrapping_add(1);
            wr::<u32>(skin, 0x10, m);
            if m == 4 {
                break 'verts;
            }
            k += 1;
            if k >= 4 {
                break;
            }
        }
    }
    let usedp: GPtr<u8> = GPtr::from_raw(used.as_mut_ptr());
    skin_find_used_bones(skin, indices, weights, usedp, GPtr::from_raw(&mut used_count), nverts);
    let ucount = used_count;
    let size = (nbones << 6).wrapping_add(nverts.wrapping_mul(20)).wrapping_add(ucount).wrapping_add(0xF);
    let data = rw_malloc(size, HINT);
    wr::<u32>(skin, 0x3C, data.addr() as u32);
    if data.is_null() {
        rw_memory_free(freelist(), skin);
        return GPtr::NULL;
    }
    for k in 0..size as isize {
        wr::<u8>(data, k, 0);
    }
    let data = p8(rd::<u32>(skin, 0x3C));
    wr::<u32>(skin, 0x0, nbones);
    wr::<u32>(skin, 0x4, ucount);
    let mats = (data.addr() as u32).wrapping_add(ucount).wrapping_add(0xF) & !0xF;
    wr::<u32>(skin, 0xC, mats);
    let idx = mats.wrapping_add(nbones << 6);
    wr::<u32>(skin, 0x14, idx);
    wr::<u32>(skin, 0x18, idx.wrapping_add(nverts.wrapping_mul(4)));
    wr::<u32>(skin, 0x8, data.addr() as u32);
    if ucount != 0 {
        copy_bytes(data, usedp, ucount);
    }
    if !inv.is_null() {
        let mut b = rd::<u32>(skin, 0);
        while b != 0 {
            let off = (b << 6).wrapping_sub(0x40);
            let dst = p8(rd::<u32>(skin, 0xC).wrapping_add(off));
            copy_bytes(dst, inv.byte_add(off as i32 as isize), 0x40);
            b -= 1;
        }
    }
    if !indices.is_null() {
        copy_bytes(p8(rd::<u32>(skin, 0x14)), indices, nverts.wrapping_mul(4));
    }
    if !weights.is_null() {
        copy_bytes(p8(rd::<u32>(skin, 0x18)), weights, nverts.wrapping_mul(16));
    }
    skin
}

/// `RpSkinDestroy(skin)` (0x7C77A0): frees the data block (if any), the split data, the skin. Returns 0.
#[hook(addr = 0x7C77A0, conv = cdecl, name = "RpSkinDestroy", expect = "56 8B 74 24 08")]
pub fn rp_skin_destroy(skin: GPtr<u8>) -> u32 {
    let d = rd::<u32>(skin, 0x3C);
    if d != 0 {
        rw_free(p8(d));
    }
    split_data_destroy_call(skin);
    rw_memory_free(freelist(), skin);
    0
}

/// `RpSkinGetNumBones(skin)` (0x7C77E0).
#[hook(addr = 0x7C77E0, conv = cdecl, name = "RpSkinGetNumBones", expect = "8B 44 24 04 8B 00")]
pub fn rp_skin_get_num_bones(skin: GPtr<u8>) -> u32 {
    rd::<u32>(skin, 0)
}

/// `RpSkinGetVertexBoneWeights(skin)` (0x7C77F0).
#[hook(addr = 0x7C77F0, conv = cdecl, name = "RpSkinGetVertexBoneWeights", expect = "8B 44 24 04 8B 40 18")]
pub fn rp_skin_get_vertex_bone_weights(skin: GPtr<u8>) -> u32 {
    rd::<u32>(skin, 0x18)
}

/// `RpSkinGetVertexBoneIndices(skin)` (0x7C7800).
#[hook(addr = 0x7C7800, conv = cdecl, name = "RpSkinGetVertexBoneIndices", expect = "8B 44 24 04 8B 40 14")]
pub fn rp_skin_get_vertex_bone_indices(skin: GPtr<u8>) -> u32 {
    rd::<u32>(skin, 0x14)
}

/// `RpSkinGetSkinToBoneMatrices(skin)` (0x7C7810).
#[hook(addr = 0x7C7810, conv = cdecl, name = "RpSkinGetSkinToBoneMatrices", expect = "8B 44 24 04 8B 40 0C")]
pub fn rp_skin_get_skin_to_bone_matrices(skin: GPtr<u8>) -> u32 {
    rd::<u32>(skin, 0xC)
}

/// `RpSkinIsSplit(skin)` (0x7C7820): `numMeshes != 0`.
#[hook(addr = 0x7C7820, conv = cdecl, name = "RpSkinIsSplit", expect = "8B 4C 24 04 33 C0")]
pub fn rp_skin_is_split(skin: GPtr<u8>) -> u32 {
    (rd::<u32>(skin, 0x28) != 0) as u32
}

/// `RpSkinAtomicSetType(atomic, type)` (0x7C7830): type 2 (MatFX) without plugin 0x120, or type 3 without
/// plugin 0x12E, falls back to 1; `_rpSkinAtomicSetPipeline(atomic, type)`.
#[hook(addr = 0x7C7830, conv = cdecl, name = "RpSkinAtomicSetType", expect = "56 8B 74 24 0C")]
pub fn rp_skin_atomic_set_type(atomic: GPtr<u8>, ty: u32) -> GPtr<u8> {
    let mut t = ty;
    let mut fallback = rw_engine_get_plugin_offset(0x120) == -1 && t == 2;
    if !fallback {
        fallback = rw_engine_get_plugin_offset(0x12E) == -1 && t == 3;
    }
    if fallback {
        t = 1;
    }
    atomic_set_pipeline_call(atomic, t)
}

/// `RpSkinAtomicGetType(atomic)` (0x7C7880): the pipeline's plugin data when its pluginId is the skin's.
#[hook(addr = 0x7C7880, conv = cdecl, name = "RpSkinAtomicGetType", expect = "8B 44 24 04 8B 40 6C")]
pub fn rp_skin_atomic_get_type(atomic: GPtr<u8>) -> u32 {
    let p = p8(rd::<u32>(atomic, 0x6C));
    if rd::<u32>(p, 0x2C) == SKIN_ID {
        rd::<u32>(p, 0x30)
    } else {
        0
    }
}

/// `_rpSkinPipelinesDestroy()` (0x7C86F0): every non-NULL slot of 0xC978C4..0xC978D4 destroyed and
/// cleared. Returns 1.
#[hook(addr = 0x7C86F0, conv = cdecl, name = "_rpSkinPipelinesDestroy", expect = "56 BE C4 78 C9 00")]
pub fn rp_skin_pipelines_destroy() -> u32 {
    let mut a = SKIN_PIPELINES;
    while a < SKIN_PIPELINES_END {
        let p = g::<u32>(a).read();
        if p != 0 {
            rx_pipeline_destroy(p8(p));
            g::<u32>(a).write(0);
        }
        a += 4;
    }
    1
}

/// `_rpSkinInitialize(geometry)` (0x7C8740): per vertex whose first weight is below 1.0 (signed integer
/// compare of the bits), a bubble pass sorting the 4 weights descending (signed integer compares; the
/// moved-down weight goes through `fld`/`fstp`, the moved-up one is copied raw) with the bone indices; a
/// swap of the last pair restarts the pass. Then `qsort(usedList, usedCount, 1, 0x7C8720)` when the list
/// exists. Returns geometry. Original bug kept: a geometry without skin dereferences NULL+8.
#[hook(addr = 0x7C8740, conv = cdecl, name = "_rpSkinInitialize", expect = "A1 A8 78 C9 00")]
pub fn rp_skin_initialize(geom: GPtr<u8>) -> GPtr<u8> {
    let skin = p8(rd::<u32>(geom_skin_slot(geom), 0));
    if !skin.is_null() && rd::<i32>(geom, 0x14) > 0 {
        let mut w = p8(rd::<u32>(skin, 0x18));
        let mut ix = p8(rd::<u32>(skin, 0x14));
        let mut v = 0i32;
        loop {
            if rd::<i32>(w, 0) < 0x3F80_0000 {
                loop {
                    let mut swapped = false;
                    let mut last = false;
                    for k in 0..3isize {
                        if rd::<i32>(w, k * 4) < rd::<i32>(w, k * 4 + 4) {
                            let (a, b) = (rd::<u8>(ix, k), rd::<u8>(ix, k + 1));
                            wr::<u8>(ix, k, b);
                            wr::<u8>(ix, k + 1, a);
                            let lo = f32_of(x(rd::<f32>(w, k * 4)));
                            let hi = rd::<u32>(w, k * 4 + 4);
                            wr::<f32>(w, k * 4 + 4, lo);
                            wr::<u32>(w, k * 4, hi);
                            swapped = true;
                            last = k == 2;
                        }
                    }
                    if !(last || swapped) {
                        break;
                    }
                }
            }
            w = w.byte_add(0x10);
            ix = ix.byte_add(4);
            v += 1;
            if v >= rd::<i32>(geom, 0x14) {
                break;
            }
        }
    }
    let list = rd::<u32>(skin, 0x8);
    if list != 0 {
        qsort(p8(list), rd::<u32>(skin, 0x4), 1, 0x7C8720);
    }
    geom
}

/// `_rpSkinDeinitialize(geometry)` (0x7C8820): returns geometry.
#[hook(addr = 0x7C8820, conv = cdecl, name = "_rpSkinDeinitialize", expect = "8B 44 24 04 C3")]
pub fn rp_skin_deinitialize(geom: GPtr<u8>) -> GPtr<u8> {
    geom
}

/// `_rpSkinPipelinesCreate(flags)` (0x7C8980): bit 0 creates the D3D9 skin pipeline into slot 0 (stored
/// even when NULL). Returns 0 when it failed, else 1.
#[hook(addr = 0x7C8980, conv = cdecl, name = "_rpSkinPipelinesCreate", expect = "8A 44 24 04 56")]
pub fn rp_skin_pipelines_create(flags: u32) -> u32 {
    let mut ok = 1;
    if flags & 1 != 0 {
        let p = d3d9_skin_pipeline_create_call();
        if p.is_null() {
            ok = 0;
        }
        g::<u32>(SKIN_PIPELINES).write(p.addr() as u32);
    }
    ok
}

/// `_rpSkinAtomicSetPipeline(atomic, type)` (0x7C89B0): atomic->pipeline = slot 0 (type ignored). EAX =
/// atomic.
#[hook(addr = 0x7C89B0, conv = cdecl, name = "_rpSkinAtomicSetPipeline", expect = "8B 44 24 04 8B 0D C4 78 C9 00")]
pub fn rp_skin_atomic_set_pipeline(atomic: GPtr<u8>, _ty: u32) -> GPtr<u8> {
    wr::<u32>(atomic, 0x6C, g::<u32>(SKIN_PIPELINES).read());
    atomic
}

/// `_rpSkinGetMeshBoneRemapIndices(skin)` (0x7C8A40).
#[hook(addr = 0x7C8A40, conv = cdecl, name = "_rpSkinGetMeshBoneRemapIndices", expect = "8B 44 24 04 8B 40 30")]
pub fn rp_skin_get_mesh_bone_remap_indices(skin: GPtr<u8>) -> u32 {
    rd::<u32>(skin, 0x30)
}

/// `_rpSkinGetMeshBoneRLECount(skin)` (0x7C8A50).
#[hook(addr = 0x7C8A50, conv = cdecl, name = "_rpSkinGetMeshBoneRLECount", expect = "8B 44 24 04 8B 40 34")]
pub fn rp_skin_get_mesh_bone_rle_count(skin: GPtr<u8>) -> u32 {
    rd::<u32>(skin, 0x34)
}

/// `_rpSkinGetMeshBoneRLE(skin)` (0x7C8A60).
#[hook(addr = 0x7C8A60, conv = cdecl, name = "_rpSkinGetMeshBoneRLE", expect = "8B 44 24 04 8B 40 38")]
pub fn rp_skin_get_mesh_bone_rle(skin: GPtr<u8>) -> u32 {
    rd::<u32>(skin, 0x38)
}

/// Clears the 6 split fields (+0x24..+0x38) after freeing the split block.
fn split_clear(skin: GPtr<u8>) {
    let b = rd::<u32>(skin, 0x30);
    if b != 0 {
        rw_free(p8(b));
    }
    for o in [0x24isize, 0x28, 0x2C, 0x30, 0x34, 0x38] {
        wr::<u32>(skin, o, 0);
    }
}

/// `_rpSkinSplitDataCreate(skin, boneLimit, numBones, numMeshes, numRLE)` (0x7C8A70): old split data
/// freed and cleared; one zeroed block of `numBones + 2 * (numMeshes + numRLE)` bytes: remap indices,
/// RLE counts (+numBones), RLE (+2 * numMeshes). Returns skin, or NULL (fields left cleared).
#[hook(addr = 0x7C8A70, conv = cdecl, name = "_rpSkinSplitDataCreate", expect = "53 55 56 8B 74 24 10")]
pub fn rp_skin_split_data_create(skin: GPtr<u8>, bone_limit: u32, nbones: u32, nmeshes: u32, nrle: u32) -> GPtr<u8> {
    split_clear(skin);
    let size = nbones.wrapping_add(nmeshes.wrapping_add(nrle).wrapping_mul(2));
    let p = rw_malloc(size, HINT);
    wr::<u32>(skin, 0x30, p.addr() as u32);
    if p.is_null() {
        return GPtr::NULL;
    }
    for k in 0..size as isize {
        wr::<u8>(p, k, 0);
    }
    let blk = rd::<u32>(skin, 0x30);
    wr::<u32>(skin, 0x24, bone_limit);
    wr::<u32>(skin, 0x2C, nrle);
    wr::<u32>(skin, 0x28, nmeshes);
    let rlec = nbones.wrapping_add(blk);
    wr::<u32>(skin, 0x34, rlec);
    wr::<u32>(skin, 0x38, rlec.wrapping_add(nmeshes.wrapping_mul(2)));
    skin
}

/// `_rpSkinSplitDataDestroy(skin)` (0x7C8B10): frees the split block, clears the fields. Returns 1.
#[hook(addr = 0x7C8B10, conv = cdecl, name = "_rpSkinSplitDataDestroy", expect = "56 8B 74 24 08")]
pub fn rp_skin_split_data_destroy(skin: GPtr<u8>) -> u32 {
    split_clear(skin);
    1
}

/// `_rpD3D9SkinPipelineInit(node)` (0x7CB240): the node's private data gets the instance (0x756DF0) and
/// lighting (0x757400) callbacks, and with vertex shaders the VS callbacks (0x7CB190, 0x761170, 0x761010,
/// 0x761030, 0x761000), else zeros. Returns 1.
#[hook(addr = 0x7CB240, conv = cdecl, name = "_rpD3D9SkinPipelineInit", expect = "8B 44 24 04 33 C9")]
pub fn rp_d3d9_skin_pipeline_init(node: GPtr<u8>) -> u32 {
    let d = p8(rd::<u32>(node, 0x14));
    wr::<u32>(d, 0x0, 0x756DF0);
    wr::<u32>(d, 0x4, 0x757400);
    let vs = g::<u32>(D3D9_SKIN_VS).read() != 0;
    let cbs: [u32; 5] = if vs { [0x7CB190, 0x761170, 0x761010, 0x761030, 0x761000] } else { [0; 5] };
    for (k, v) in cbs.iter().enumerate() {
        wr::<u32>(d, 8 + k as isize * 4, *v);
    }
    1
}

/// `RxNodeDefinitionGetD3D9SkinAtomicAllInOne()` (0x7CB2A0).
#[hook(addr = 0x7CB2A0, conv = cdecl, name = "RxNodeDefinitionGetD3D9SkinAtomicAllInOne", expect = "B8 08 ED 8D 00")]
pub fn rx_node_definition_get_d3d9_skin_atomic_all_in_one() -> u32 {
    D3D9_SKIN_NODE_DEF
}

/// `_rpD3D9SkinPipelineCreate()` (0x7CB2B0): skin caps from `RwD3D9GetCaps` (hardware T&L bit of byte
/// +0x1E; VS version (+0xC4) and PS version (+0xCC) low words >= 0x101 enable vertex shaders, max bones =
/// (MaxVertexShaderConst (+0xC8) - 12) / 3), then a pipeline (pluginId 0x116, data 1) with the all-in-one
/// node; a lock failure destroys it (NULL).
#[hook(addr = 0x7CB2B0, conv = cdecl, name = "_rpD3D9SkinPipelineCreate", expect = "E8 6B FA 02 00")]
pub fn rp_d3d9_skin_pipeline_create() -> GPtr<u8> {
    let caps = rw_d3d9_get_caps();
    let hw = (rd::<u8>(caps, 0x1E) & 1) as u32;
    g::<u32>(D3D9_SKIN_HWTL).write(hw);
    if hw != 0 && rd::<u32>(caps, 0xC4) & 0xFFFF >= 0x101 && rd::<u32>(caps, 0xCC) & 0xFFFF >= 0x101 {
        g::<u32>(D3D9_SKIN_VS).write(1);
        let m = rd::<u32>(caps, 0xC8);
        g::<u32>(D3D9_SKIN_MAX_VS_CONST).write(m);
        g::<u32>(D3D9_SKIN_MAX_BONES).write(m.wrapping_sub(0xC) / 3);
    } else {
        g::<u32>(D3D9_SKIN_VS).write(0);
        g::<u32>(D3D9_SKIN_MAX_BONES).write(0);
        g::<u32>(D3D9_SKIN_MAX_VS_CONST).write(0);
    }
    let p = rx_pipeline_create();
    if p.is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(p, 0x2C, SKIN_ID);
    wr::<u32>(p, 0x30, 1);
    let l = rx_pipeline_lock(p);
    if l.is_null() {
        rx_pipeline_destroy(p);
        return GPtr::NULL;
    }
    let f = rx_locked_pipe_add_fragment(l, 0, p8(D3D9_SKIN_NODE_DEF), 0);
    rx_locked_pipe_unlock(f);
    p
}

orig! {
    /// `RwFreeListCreateAndPreallocateSpace(size, perBlock, align, n, space, hint)` (0x801B70)
    fn freelist_create_prealloc(size: u32, per_block: u32, align: u32, n: u32, space: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x801B70 => crate::memory::rw_freelist_create_and_preallocate_space;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    fn freelist_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => crate::memory::rw_freelist_destroy;
    /// `_rpSkinPipelinesCreate(flags)` (0x7C8980)
    fn skin_pipelines_create_call(flags: u32) -> u32 = cdecl 0x7C8980 => rp_skin_pipelines_create;
    /// `_rpSkinPipelinesDestroy()` (0x7C86F0)
    fn skin_pipelines_destroy_call() -> u32 = cdecl 0x7C86F0 => rp_skin_pipelines_destroy;
}

/// Skin engine-module instance count.
const SKIN_OPEN_COUNT: usize = 0xC978C0;

/// `cb_SKINPLUGIN_RwEngine_Open(instance, offset, size)` (0x7C68E0). On the first instance: the
/// pipeline flags (1; 3 when the MatFX engine plugin 0x120 is attached; | 4 with plugin 0x12E),
/// `_rpSkinPipelinesCreate(flags)` (result ignored), the matrix freelist (0x40-byte entries,
/// `[0x8DECE0]` per block, `[0x8DECE4]` preallocated, space 0xC97078, hint 0x40116) -> 0xC978B8, a
/// zeroed 0x400F-byte block (`rwmalloc`, not NULL-checked) -> 0xC978B0 and its 16-aligned address
/// -> 0xC978AC. Counts the instance and returns it.
#[hook(addr = 0x7C68E0, conv = cdecl, name = "cb_SKINPLUGIN_RwEngine_Open", expect = "A1 C0 78 C9 00 85 C0",
       unverified = "std4-rxpipe-2", route = "L2", hazards = "heap-writes")]
pub fn skin_engine_open(instance: u32, _offset: u32, _size: u32) -> u32 {
    if g::<u32>(SKIN_OPEN_COUNT).read() == 0 {
        let mut flags = 1u32;
        if rw_engine_get_plugin_offset(0x120) != -1 {
            flags = 3;
        }
        if rw_engine_get_plugin_offset(0x12E) != -1 {
            flags |= 4;
        }
        skin_pipelines_create_call(flags);
        let fl = freelist_create_prealloc(
            0x40,
            g::<u32>(FL_BLOCK_SIZE).read(),
            4,
            g::<u32>(FL_PREALLOC).read(),
            p8(0xC97078),
            0x40116,
        );
        g::<u32>(SKIN_FREELIST).write(fl.addr() as u32);
        let buf = rw_malloc(0x400F, 0x40116);
        g::<u32>(0xC978B0).write(buf.addr() as u32);
        for k in 0..0x400F {
            buf.byte_add(k).write(0u8);
        }
        let b = g::<u32>(0xC978B0).read();
        g::<u32>(0xC978AC).write(b.wrapping_add(0xF) & !0xF);
    }
    g::<u32>(SKIN_OPEN_COUNT).write(g::<u32>(SKIN_OPEN_COUNT).read().wrapping_add(1));
    instance
}

/// `cb_SKINPLUGIN_RwEngine_Close(instance, offset, size)` (0x7C69A0): on the last instance the
/// pipelines, the matrix freelist (0xC978B8 = NULL) and the 0x400F-byte block (0xC978B0 = NULL) are
/// released. Returns the instance.
#[hook(addr = 0x7C69A0, conv = cdecl, name = "cb_SKINPLUGIN_RwEngine_Close", expect = "A1 C0 78 C9 00 48 A3",
       unverified = "std4-rxpipe-2", route = "L2", hazards = "heap-writes")]
pub fn skin_engine_close(instance: u32, _offset: u32, _size: u32) -> u32 {
    let n = g::<u32>(SKIN_OPEN_COUNT).read().wrapping_sub(1);
    g::<u32>(SKIN_OPEN_COUNT).write(n);
    if n == 0 {
        skin_pipelines_destroy_call();
        freelist_destroy(p8(g::<u32>(SKIN_FREELIST).read()));
        let b = g::<u32>(0xC978B0).read();
        g::<u32>(SKIN_FREELIST).write(0);
        rw_free(p8(b));
        g::<u32>(0xC978B0).write(0);
    }
    instance
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_SKIN_ENGINE_OPEN,
    HOOK_SKIN_ENGINE_CLOSE,
    HOOK_RP_SKIN_SET_FREE_LIST_CREATE_PARAMS,
    HOOK_RP_SKIN_PLUGIN_ATTACH,
    HOOK_RP_SKIN_ATOMIC_SET_HANIM_HIERARCHY,
    HOOK_RP_SKIN_ATOMIC_GET_HANIM_HIERARCHY,
    HOOK_RP_SKIN_GEOMETRY_GET_SKIN,
    HOOK_RP_SKIN_GEOMETRY_SET_SKIN,
    HOOK_RP_SKIN_CREATE,
    HOOK_RP_SKIN_DESTROY,
    HOOK_RP_SKIN_GET_NUM_BONES,
    HOOK_RP_SKIN_GET_VERTEX_BONE_WEIGHTS,
    HOOK_RP_SKIN_GET_VERTEX_BONE_INDICES,
    HOOK_RP_SKIN_GET_SKIN_TO_BONE_MATRICES,
    HOOK_RP_SKIN_IS_SPLIT,
    HOOK_RP_SKIN_ATOMIC_SET_TYPE,
    HOOK_RP_SKIN_ATOMIC_GET_TYPE,
    HOOK_RP_D3D9_SKIN_VERTEX_SHADER_MATRIX_UPDATE,
    HOOK_RP_SKIN_PIPELINES_DESTROY,
    HOOK_RP_SKIN_INITIALIZE,
    HOOK_RP_SKIN_DEINITIALIZE,
    HOOK_RP_SKIN_PIPELINES_CREATE,
    HOOK_RP_SKIN_ATOMIC_SET_PIPELINE,
    HOOK_RP_SKIN_GET_MESH_BONE_REMAP_INDICES,
    HOOK_RP_SKIN_GET_MESH_BONE_RLE_COUNT,
    HOOK_RP_SKIN_GET_MESH_BONE_RLE,
    HOOK_RP_SKIN_SPLIT_DATA_CREATE,
    HOOK_RP_SKIN_SPLIT_DATA_DESTROY,
    HOOK_RP_D3D9_SKIN_PIPELINE_INIT,
    HOOK_RX_NODE_DEFINITION_GET_D3D9_SKIN_ATOMIC_ALL_IN_ONE,
    HOOK_RP_D3D9_SKIN_PIPELINE_CREATE,
];

/// Globals owned by this module (skin state is accessed raw; other modules read the plugin offsets).
pub const GLOBALS: &[GlobalDesc] = &[];
