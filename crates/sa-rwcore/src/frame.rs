//! PORT-RW-1 module `frame`: `RwFrame`, the frame list stream reader, the dirty-frame list and LTM sync,
//! `RwObjectHasFrame` (see docs/arch/rw-port.md). Ported from the 1.0 US bytes; `docs/re/specs/RW/RwFrame.md`
//! as a reading aid.
//!
//! `RwFrame` (0xA4 base, registry size 0xC8 in GTA): +0 type, +1 subType, +2 flags, +3 privateFlags,
//! +4 parent, +8/+0xC inDirtyListLink, +0x10 modelling (`RwMatrix`), +0x50 ltm, +0x90 objectList,
//! +0x98 child, +0x9C next sibling, +0xA0 root. privateFlags: 1 HIERARCHYSYNCLTM, 2 HIERARCHYSYNCOBJ (root
//! in the dirty list), 4 SUBTREESYNCLTM, 8 SUBTREESYNCOBJ. `RwObjectHasFrame`: +4 parent frame, +8/+0xC
//! lFrame link, +0x10 sync callback. The dirty list head is `G + 0xBC` (next) / `G + 0xC0` (prev).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{
    error1, error2, p8, rd, rw_free, rw_globals, rw_malloc, rw_memory_alloc, rw_memory_free, wr,
};

/// `G + 0xBC`: the dirty frame list head.
const DIRTY_LIST: isize = 0xBC;

sa_core::globals! {
    list FRAME_GLOBALS;
    /// FRAMEMODULE offset in `RwGlobals` (written by the module Open callback)
    pub static FRAME_MODULE_OFFSET @ 0xC9796C: u32 = 0;
    /// FRAMEMODULE open-instance counter
    pub static FRAME_OPEN_COUNT @ 0xC97970: i32 = 0;
    /// The frame freelist's in-place `RwFreeList` (0x24 bytes)
    pub static FRAME_FREELIST_SPACE @ 0xC97974: [u32; 9] = [0; 9];
    /// Frame `RwPluginRegistry` (sizeOfStruct, origSizeOfStruct, maxSizeOfStruct, staticAlloc, first, last)
    pub static FRAME_REGISTRY @ 0x8E2270: [u32; 6] = [0xA4, 0xA4, 0, 0, 0, 0];
    /// Frame freelist entries per block
    pub static FRAME_FREELIST_BLOCK_SIZE @ 0x8E2288: i32 = 0x32;
    /// Frame freelist blocks to preallocate
    pub static FRAME_FREELIST_PREALLOC @ 0x8E228C: i32 = 1;
    /// `frameListAutoUpdate` (RwFrameListSetAutoUpdate)
    pub static FRAME_LIST_AUTO_UPDATE @ 0x8E26A8: i32 = 1;
}

/// `[0x858C58]` = 0.01f: matrix normal/orthogonal error tolerance of `_rwFrameListStreamRead`.
const MATRIX_ERROR_TOL: f32 = f32::from_bits(0x3C23_D70A);
/// `[0x862CD0]` = 0.99f: minimum determinant of `_rwFrameListStreamRead`.
const MATRIX_DET_MIN: f32 = f32::from_bits(0x3F7D_70A4);

orig! {
    /// `RwFreeListCreateAndPreallocateSpace(entrySize, entriesPerBlock, alignment, numBlocks, space, hint)` (0x801B70)
    pub fn free_list_create_and_preallocate_space(size: i32, per_block: i32, align: i32, blocks: i32, space: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x801B70 => unported;
    /// `RwFreeListDestroy(freelist)` (0x801B80)
    pub fn free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => unported;
    /// `RwMatrixMultiply(dst, a, b)` (0x7F18B0)
    pub fn matrix_multiply(dst: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18B0 => unported;
    /// `RwMatrixOrthoNormalize(dst, src)` (0x7F1920)
    pub fn matrix_ortho_normalize(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F1920 => unported;
    /// `RwMatrixRotate(m, axis, angle, op)` (0x7F1FD0; the angle is passed as its raw dword)
    pub fn matrix_rotate(m: GPtr<u8>, axis: GPtr<u8>, angle: u32, op: i32) -> GPtr<u8> = cdecl 0x7F1FD0 => unported;
    /// `RwMatrixScale(m, v, op)` (0x7F22C0)
    pub fn matrix_scale(m: GPtr<u8>, v: GPtr<u8>, op: i32) -> GPtr<u8> = cdecl 0x7F22C0 => unported;
    /// `RwMatrixTranslate(m, v, op)` (0x7F2450)
    pub fn matrix_translate(m: GPtr<u8>, v: GPtr<u8>, op: i32) -> GPtr<u8> = cdecl 0x7F2450 => unported;
    /// `RwMatrixTransform(m, t, op)` (0x7F25A0)
    pub fn matrix_transform(m: GPtr<u8>, t: GPtr<u8>, op: i32) -> GPtr<u8> = cdecl 0x7F25A0 => unported;
    /// `_rwMatrixNormalError(m)` (0x7F1500, ST0 result)
    pub fn matrix_normal_error(m: GPtr<u8>) -> f64 = cdecl 0x7F1500 => unported;
    /// `_rwMatrixOrthogonalError(m)` (0x7F14A0, ST0 result)
    pub fn matrix_orthogonal_error(m: GPtr<u8>) -> f64 = cdecl 0x7F14A0 => unported;
    /// `_rwMatrixDeterminant(m)` (0x7F1450, ST0 result)
    pub fn matrix_determinant(m: GPtr<u8>) -> f64 = cdecl 0x7F1450 => unported;
    /// `RwStreamFindChunk(stream, type, lengthOut, versionOut)` (0x7ED2D0)
    pub fn stream_find_chunk(s: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> i32 = cdecl 0x7ED2D0 => unported;
    /// `RwStreamRead(stream, buffer, length)` (0x7EC9D0)
    pub fn stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => unported;
    /// `RwObjectHasFrame` sync callback `obj->sync(obj)` (stored at obj + 0x10)
    pub fn call_object_sync(f: u32, obj: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
    /// `RwFrameForAllChildren` callback `cb(frame, data)`
    pub fn call_frame_cb(f: u32, frame: GPtr<u8>, data: u32) -> GPtr<u8> = cdecl dyn => unported;
    /// `RwFrameForAllObjects` callback `cb(object, data)`
    pub fn call_object_cb(f: u32, obj: GPtr<u8>, data: u32) -> GPtr<u8> = cdecl dyn => unported;

    /// `_rwPluginRegistryInitObject(reg, obj)` (0x8086E0)
    pub fn registry_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => crate::plugin::registry_init_object;
    /// `_rwPluginRegistryDeInitObject(reg, obj)` (0x808740)
    pub fn registry_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => crate::plugin::registry_deinit_object;
    /// `_rwPluginRegistryCopyObject(reg, dst, src)` (0x808770)
    pub fn registry_copy_object(reg: GPtr<u8>, dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x808770 => crate::plugin::registry_copy_object;
    /// `_rwPluginRegistryAddPlugin(reg, size, id, ctor, dtor, copy)` (0x8084A0)
    pub fn registry_add_plugin(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x8084A0 => crate::plugin::registry_add_plugin;
    /// `_rwPluginRegistryAddPluginStream(reg, id, read, write, getSize)` (0x8088E0)
    pub fn registry_add_plugin_stream(reg: GPtr<u8>, id: u32, read: u32, write: u32, get_size: u32) -> i32 = cdecl 0x8088E0 => crate::plugin::registry_add_plugin_stream;
    /// `_rwPluginRegistryReadDataChunks(reg, stream, obj)` (0x808980)
    pub fn registry_read_data_chunks(reg: GPtr<u8>, s: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808980 => crate::plugin::registry_read_data_chunks;

    /// `_rwFrameCloneRecurse` (0x7F0050)
    pub fn clone_recurse_call(old: GPtr<u8>, new_root: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0050 => clone_recurse;
    /// `_rwSetHierarchyRoot` (0x7F0210)
    pub fn set_hierarchy_root_call(f: GPtr<u8>, root: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0210 => set_hierarchy_root;
    /// `_rwFrameInternalInit` (0x7F0360)
    pub fn internal_init_call(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0360 => internal_init;
    /// `RwFrameCreate` (0x7F0410)
    pub fn create_call() -> GPtr<u8> = cdecl 0x7F0410 => create;
    /// `_rwFrameDestroyRecurse` (0x7F0830)
    pub fn destroy_recurse_call(f: GPtr<u8>) = cdecl 0x7F0830 => destroy_recurse;
    /// `RwFrameDestroyHierarchy` (0x7F08A0)
    pub fn destroy_hierarchy_call(f: GPtr<u8>) -> u32 = cdecl 0x7F08A0 => destroy_hierarchy;
    /// `RwFrameUpdateObjects` (0x7F0910)
    pub fn update_objects_call(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => update_objects;
    /// `RwFrameAddChildNoUpdate` (0x7F09C0)
    pub fn add_child_no_update_call(parent: GPtr<u8>, child: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F09C0 => add_child_no_update;
    /// `RwFrameAddChild` (0x7F0B00)
    pub fn add_child_call(parent: GPtr<u8>, child: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0B00 => add_child;
    /// `RwFrameCount` (0x7F0E00)
    pub fn count_call(f: GPtr<u8>) -> i32 = cdecl 0x7F0E00 => count;
    /// `_rwFrameSyncDirtyLTM` (0x809700)
    pub fn sync_dirty_ltm_call(f: GPtr<u8>, parent_flags: u32) = cdecl 0x809700 => sync_dirty_ltm;
    /// `_rwFrameSyncDirtyObjects` (0x809780)
    pub fn sync_dirty_objects_call(f: GPtr<u8>) = cdecl 0x809780 => sync_dirty_objects;
    /// `_rwFrameSyncHierarchyLTM` (0x8097D0)
    pub fn sync_hierarchy_ltm_call(root: GPtr<u8>) = cdecl 0x8097D0 => sync_hierarchy_ltm;
    /// `_rwFrameSyncHierarchyLTMRecurse` (0x809850)
    pub fn sync_hierarchy_ltm_recurse_call(f: GPtr<u8>, parent_flags: u32) = cdecl 0x809850 => sync_hierarchy_ltm_recurse;
}

// ---- field helpers ----

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}
fn flags(f: GPtr<u8>) -> u8 {
    rd::<u8>(f, 3)
}
fn set_flags(f: GPtr<u8>, v: u8) {
    wr::<u8>(f, 3, v)
}
fn parent(f: GPtr<u8>) -> GPtr<u8> {
    p8(rd::<u32>(f, 4))
}
fn child(f: GPtr<u8>) -> GPtr<u8> {
    p8(rd::<u32>(f, 0x98))
}
fn next(f: GPtr<u8>) -> GPtr<u8> {
    p8(rd::<u32>(f, 0x9C))
}
fn root(f: GPtr<u8>) -> GPtr<u8> {
    p8(rd::<u32>(f, 0xA0))
}

/// The frame registry (`0x8E2270`).
pub fn frame_registry() -> GPtr<u8> {
    FRAME_REGISTRY.ptr().cast()
}

/// The frame freelist: `[G + [0xC9796C]]`.
fn frame_freelist() -> GPtr<u8> {
    p8(rd::<u32>(rw_globals(), FRAME_MODULE_OFFSET.get() as i32 as isize))
}

/// Copies the 16 dwords of a matrix (`rep movsd`, ascending).
fn copy_matrix(dst: GPtr<u8>, src: GPtr<u8>) {
    for i in 0..16 {
        wr::<u32>(dst, i * 4, rd::<u32>(src, i * 4));
    }
}

/// MarkDirty(f): the inlined body of `RwFrameUpdateObjects` (0x7F0910..0x7F098C) in its store order. The
/// root's flags are read once; `f->root` and `G` are re-read for every store, as the original does.
fn mark_dirty(f: GPtr<u8>) {
    let rf = flags(root(f));
    if rf & 3 == 0 {
        wr::<u32>(root(f), 8, rd::<u32>(rw_globals(), DIRTY_LIST));
        wr::<u32>(root(f), 0xC, a32(rw_globals()).wrapping_add(DIRTY_LIST as u32));
        let head_next = p8(rd::<u32>(rw_globals(), DIRTY_LIST));
        wr::<u32>(head_next, 4, a32(root(f)).wrapping_add(8));
        wr::<u32>(rw_globals(), DIRTY_LIST, a32(root(f)).wrapping_add(8));
    }
    set_flags(root(f), rf | 3);
    set_flags(f, flags(f) | 0xC);
}

/// Unlink(f) from the dirty list (link fields kept).
fn unlink_dirty(f: GPtr<u8>) {
    wr::<u32>(p8(rd::<u32>(f, 0xC)), 0, rd::<u32>(f, 8));
    wr::<u32>(p8(rd::<u32>(f, 8)), 4, rd::<u32>(f, 0xC));
}

/// Unlink an `RwLLLink` at `l` (prev->next = next; next->prev = prev).
fn unlink_link(l: GPtr<u8>) {
    wr::<u32>(p8(rd::<u32>(l, 4)), 0, rd::<u32>(l, 0));
    wr::<u32>(p8(rd::<u32>(l, 0)), 4, rd::<u32>(l, 4));
}

/// Detach(child) from its (non-NULL) parent's child list, make it a root and re-root its subtree.
/// No termination guard in the sibling walk (child must be in the list).
fn detach(c: GPtr<u8>) {
    let p = parent(c);
    let first = child(p);
    if first == c {
        wr::<u32>(p, 0x98, rd::<u32>(c, 0x9C));
    } else {
        let mut s = first;
        loop {
            let n = next(s);
            if n == c {
                break;
            }
            s = n;
        }
        wr::<u32>(s, 0x9C, rd::<u32>(c, 0x9C));
    }
    let mut k = child(c);
    wr::<u32>(c, 4, 0);
    wr::<u32>(c, 0x9C, 0);
    wr::<u32>(c, 0xA0, a32(c));
    while !k.is_null() {
        set_hierarchy_root_call(k, c);
        k = next(k);
    }
}

/// SyncObjects(f): `obj->sync(obj)` for every attached object, next link read after the callback.
fn sync_objects(f: GPtr<u8>) {
    let head = f.byte_add(0x90);
    let mut l = p8(rd::<u32>(f, 0x90));
    while l != head {
        let obj = l.byte_add(-8);
        call_object_sync(rd::<u32>(obj, 0x10), obj);
        l = p8(rd::<u32>(l, 0));
    }
}

// ---- module ----

/// `FRAMEMODULE` engine Open (0x7EFEF0): records the module offset, creates the frame freelist in its
/// static space, empties the dirty list, counts the instance.
#[hook(addr = 0x7EFEF0, conv = cdecl, name = "cb_FRAMEMODULE_RwEngine_Open", expect = "8B 44 24 08 8B 0D 8C 22 8E 00")]
pub fn module_open(instance: GPtr<u8>, offset: i32, _size: i32) -> GPtr<u8> {
    let prealloc = FRAME_FREELIST_PREALLOC.get();
    let per_block = FRAME_FREELIST_BLOCK_SIZE.get();
    FRAME_MODULE_OFFSET.set(offset as u32);
    let size = rd::<i32>(frame_registry(), 0);
    let fl = free_list_create_and_preallocate_space(
        size,
        per_block,
        4,
        prealloc,
        FRAME_FREELIST_SPACE.ptr().cast(),
        0x4000E,
    );
    wr::<u32>(rw_globals(), FRAME_MODULE_OFFSET.get() as i32 as isize, a32(fl));
    if frame_freelist().is_null() {
        return GPtr::NULL;
    }
    let head = rw_globals().byte_add(DIRTY_LIST);
    wr::<u32>(head, 0, a32(head));
    let head = rw_globals().byte_add(DIRTY_LIST);
    wr::<u32>(rw_globals(), DIRTY_LIST + 4, a32(head));
    FRAME_OPEN_COUNT.set(FRAME_OPEN_COUNT.get().wrapping_add(1));
    instance
}

/// `FRAMEMODULE` engine Close (0x7EFF70): destroys the frame freelist, clears its slot, uncounts.
#[hook(addr = 0x7EFF70, conv = cdecl, name = "cb_FRAMEMODULE_RwEngine_Close", expect = "A1 24 7B C9 00")]
pub fn module_close(instance: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let fl = frame_freelist();
    if !fl.is_null() {
        free_list_destroy(fl);
        wr::<u32>(rw_globals(), FRAME_MODULE_OFFSET.get() as i32 as isize, 0);
    }
    FRAME_OPEN_COUNT.set(FRAME_OPEN_COUNT.get().wrapping_sub(1));
    instance
}

// ---- clone ----

/// Clear the clone's "in dirty list" bits and mark it dirty (shared by 0x7EFFB0 and 0x7F0250).
fn mark_clone_dirty(c: GPtr<u8>) {
    set_flags(c, flags(c) & 0xFC);
    mark_dirty(c);
}

/// Inline `_rwFramePurgeClone` body.
fn purge_clone_body(f: GPtr<u8>) {
    let p = parent(f);
    let r = if p.is_null() { f } else { root(p) };
    let mut c = child(f);
    wr::<u32>(f, 0xA0, a32(r));
    while !c.is_null() {
        set_hierarchy_root_call(c, r);
        c = next(c);
    }
}

/// `_rwFrameCloneAndLinkClones` (0x7EFFB0): clones the hierarchy; every original's `root` then points at
/// its clone until `_rwFramePurgeClone`.
#[hook(addr = 0x7EFFB0, conv = cdecl, name = "_rwFrameCloneAndLinkClones", expect = "8B 44 24 04 6A 00")]
pub fn clone_and_link_clones(root_frame: GPtr<u8>) -> GPtr<u8> {
    let c = clone_recurse_call(root_frame, GPtr::NULL);
    if c.is_null() {
        return c;
    }
    mark_clone_dirty(c);
    c
}

/// `_rwFrameCloneRecurse` (0x7F0050). The dirty link, the ltm and the plugin area are not initialised
/// before the plugin constructors run. The failure path (freelist exhaustion) keeps the original's
/// bugs: plugin destructors on an unconstructed frame, an unlink through the copied flags' stale link.
#[hook(addr = 0x7F0050, conv = cdecl, name = "_rwFrameCloneRecurse", expect = "A1 24 7B C9 00")]
pub fn clone_recurse(old: GPtr<u8>, new_root: GPtr<u8>) -> GPtr<u8> {
    let f = rw_memory_alloc(frame_freelist(), 0x3000E);
    if f.is_null() {
        return GPtr::NULL;
    }
    let new_root = if new_root.is_null() { f } else { new_root };
    for i in 0..4 {
        wr::<u8>(f, i, rd::<u8>(old, i));
    }
    let list = f.byte_add(0x90);
    wr::<u32>(f, 0x94, a32(list));
    wr::<u32>(list, 0, a32(list));
    wr::<u32>(f, 4, 0);
    copy_matrix(f.byte_add(0x10), old.byte_add(0x10));
    wr::<u32>(f, 0x98, 0);
    wr::<u32>(f, 0x9C, 0);
    wr::<u32>(f, 0xA0, a32(new_root));
    let mut c = child(old);
    wr::<u32>(old, 0xA0, a32(f));
    while !c.is_null() {
        let cc = clone_recurse_call(c, new_root);
        if cc.is_null() {
            let mut k = child(f);
            while !k.is_null() {
                let n = next(k);
                destroy_recurse_call(k);
                k = n;
            }
            registry_deinit_object(frame_registry(), f);
            if flags(f) & 3 != 0 {
                unlink_dirty(f);
            }
            rw_memory_free(frame_freelist(), f);
            return GPtr::NULL;
        }
        wr::<u32>(cc, 0x9C, rd::<u32>(f, 0x98));
        wr::<u32>(f, 0x98, a32(cc));
        wr::<u32>(cc, 4, a32(f));
        c = next(c);
    }
    registry_init_object(frame_registry(), f);
    registry_copy_object(frame_registry(), f, old);
    f
}

/// `_rwFramePurgeClone` (0x7F01A0): restores `root` of a hierarchy after `_rwFrameCloneAndLinkClones`.
#[hook(addr = 0x7F01A0, conv = cdecl, name = "_rwFramePurgeClone", expect = "53 56 57 8B 7C 24 10")]
pub fn purge_clone(f: GPtr<u8>) -> GPtr<u8> {
    purge_clone_body(f);
    f
}

/// `_rwSetHierarchyRoot` (0x7F0210): sets `root` of f and its whole subtree (not f's siblings). Returns
/// EAX = f. Not hooked: every frame function that calls it keeps ECX and EDX across the call (88 uses,
/// sa-hook `callers` test); a `preserve = "ecx,edx"` hook would need an L1 spec, and the function writes
/// game memory.
pub fn set_hierarchy_root(f: GPtr<u8>, r: GPtr<u8>) -> GPtr<u8> {
    let mut c = child(f);
    wr::<u32>(f, 0xA0, a32(r));
    while !c.is_null() {
        set_hierarchy_root_call(c, r);
        c = next(c);
    }
    f
}

/// `RwFrameCloneHierarchy` (0x7F0250): clone + mark dirty, then (also when cloning failed) the inline
/// `_rwFramePurgeClone` of the original hierarchy.
#[hook(addr = 0x7F0250, conv = cdecl, name = "RwFrameCloneHierarchy", expect = "56 8B 74 24 08")]
pub fn clone_hierarchy(f: GPtr<u8>) -> GPtr<u8> {
    let c = clone_recurse_call(f, GPtr::NULL);
    if !c.is_null() {
        mark_clone_dirty(c);
    }
    purge_clone_body(f);
    c
}

/// `RwFrameDirty` (0x7F0340): `f->root->privateFlags & 3` (EAX fully defined).
#[hook(addr = 0x7F0340, conv = cdecl, name = "RwFrameDirty", expect = "8B 44 24 04 8B 88 A0 00 00 00")]
pub fn dirty(f: GPtr<u8>) -> u32 {
    (flags(root(f)) & 3) as u32
}

// ---- create / destroy ----

/// `_rwFrameInternalInit` (0x7F0360): identity matrices, empty links, then the plugin constructors.
/// Matrix pads and the dirty link are not written.
#[hook(addr = 0x7F0360, conv = cdecl, name = "_rwFrameInternalInit", expect = "8B 44 24 04 33 C9")]
pub fn internal_init(f: GPtr<u8>) -> GPtr<u8> {
    let list = f.byte_add(0x90);
    wr::<u8>(f, 0, 0);
    wr::<u32>(f, 0x94, a32(list));
    wr::<u8>(f, 1, 0);
    wr::<u32>(list, 0, a32(list));
    wr::<u8>(f, 2, 0);
    wr::<u8>(f, 3, 0);
    wr::<u32>(f, 4, 0);
    const ONE: u32 = 0x3F80_0000;
    for (off, v) in [
        (0x38, ONE),
        (0x24, ONE),
        (0x10, ONE),
        (0x20, 0),
        (0x18, 0),
        (0x14, 0),
        (0x34, 0),
        (0x30, 0),
        (0x28, 0),
        (0x48, 0),
        (0x44, 0),
        (0x40, 0),
        (0x1C, 0x20003),
        (0x78, ONE),
        (0x64, ONE),
        (0x50, ONE),
        (0x60, 0),
        (0x58, 0),
        (0x54, 0),
        (0x74, 0),
        (0x70, 0),
        (0x68, 0),
        (0x88, 0),
        (0x84, 0),
        (0x80, 0),
        (0x5C, 0x20003),
        (0x98, 0),
        (0x9C, 0),
    ] {
        wr::<u32>(f, off, v);
    }
    wr::<u32>(f, 0xA0, a32(f));
    registry_init_object(frame_registry(), f)
}

/// `RwFrameCreate` (0x7F0410): no RwError on allocation failure; a plugin constructor failure still
/// returns the frame.
#[hook(addr = 0x7F0410, conv = cdecl, name = "RwFrameCreate", expect = "A1 24 7B C9 00")]
pub fn create() -> GPtr<u8> {
    let f = rw_memory_alloc(frame_freelist(), 0x3000E);
    if f.is_null() {
        return f;
    }
    internal_init_call(f);
    f
}

/// `_rwFrameInit` (0x7F0450): `_rwFrameInternalInit`, then privateFlags |= 0x10; returns 1.
#[hook(addr = 0x7F0450, conv = cdecl, name = "_rwFrameInit", expect = "56 8B 74 24 08")]
pub fn frame_init(f: GPtr<u8>) -> u32 {
    internal_init_call(f);
    set_flags(f, flags(f) | 0x10);
    1
}

/// The shared body of `RwFrameDestroy` and `_rwFrameDeInit`: plugin destructors first, detach + mark
/// dirty when parented, unlink from the dirty list, orphan the children (their `root` stays f).
fn destroy_body(f: GPtr<u8>) {
    registry_deinit_object(frame_registry(), f);
    if !parent(f).is_null() {
        detach(f);
        mark_dirty(f);
    }
    if flags(f) & 3 != 0 {
        unlink_dirty(f);
    }
    let mut c = child(f);
    while !c.is_null() {
        wr::<u32>(c, 4, 0);
        c = next(c);
    }
}

/// `_rwFrameDeInit` (0x7F06F0): `RwFrameDestroy` without the free; returns 1.
#[hook(addr = 0x7F06F0, conv = cdecl, name = "_rwFrameDeInit", expect = "56 8B 74 24 08")]
pub fn frame_deinit(f: GPtr<u8>) -> u32 {
    destroy_body(f);
    1
}

/// `RwFrameDestroy` (0x7F05A0): always 1. Attached objects are not detached.
#[hook(addr = 0x7F05A0, conv = cdecl, name = "RwFrameDestroy", expect = "56 8B 74 24 08")]
pub fn destroy(f: GPtr<u8>) -> u32 {
    destroy_body(f);
    rw_memory_free(frame_freelist(), f);
    1
}

fn destroy_recurse_body(f: GPtr<u8>) {
    let mut c = child(f);
    while !c.is_null() {
        let n = next(c);
        destroy_recurse_call(c);
        c = n;
    }
    registry_deinit_object(frame_registry(), f);
    if flags(f) & 3 != 0 {
        unlink_dirty(f);
    }
    rw_memory_free(frame_freelist(), f);
}

/// `_rwFrameDestroyRecurse` (0x7F0830): children first (next read before the call), then f; no detach.
#[hook(addr = 0x7F0830, conv = cdecl, name = "_rwFrameDestroyRecurse", expect = "57 8B 7C 24 08")]
pub fn destroy_recurse(f: GPtr<u8>) {
    if f.is_null() {
        return;
    }
    destroy_recurse_body(f);
}

/// `RwFrameDestroyHierarchy` (0x7F08A0): as `_rwFrameDestroyRecurse`; returns 1 (also for NULL).
#[hook(addr = 0x7F08A0, conv = cdecl, name = "RwFrameDestroyHierarchy", expect = "57 8B 7C 24 08")]
pub fn destroy_hierarchy(f: GPtr<u8>) -> u32 {
    if !f.is_null() {
        destroy_recurse_body(f);
    }
    1
}

// ---- hierarchy ----

/// `RwFrameUpdateObjects` (0x7F0910): MarkDirty(f); returns f.
#[hook(addr = 0x7F0910, conv = cdecl, name = "RwFrameUpdateObjects", expect = "8B 44 24 04 33 C9")]
pub fn update_objects(f: GPtr<u8>) -> GPtr<u8> {
    mark_dirty(f);
    f
}

/// `RwFrameGetLTM` (0x7F0990): syncs the hierarchy's LTMs when the root has HIERARCHYSYNCLTM.
#[hook(addr = 0x7F0990, conv = cdecl, name = "RwFrameGetLTM", expect = "56 8B 74 24 08")]
pub fn get_ltm(f: GPtr<u8>) -> GPtr<u8> {
    let r = root(f);
    if flags(r) & 1 != 0 {
        sync_hierarchy_ltm_call(r);
    }
    f.byte_add(0x50)
}

/// `RwFrameGetRoot` (0x7F09B0).
#[hook(addr = 0x7F09B0, conv = cdecl, name = "RwFrameGetRoot", expect = "8B 44 24 04 8B 80 A0 00 00 00")]
pub fn get_root(f: GPtr<u8>) -> GPtr<u8> {
    root(f)
}

/// Link `c` as the first child of `p` and give its subtree p's root (the root read once).
fn link_child(p: GPtr<u8>, c: GPtr<u8>) {
    wr::<u32>(c, 0x9C, rd::<u32>(p, 0x98));
    wr::<u32>(p, 0x98, a32(c));
    let mut k = child(c);
    wr::<u32>(c, 4, a32(p));
    let r = root(p);
    wr::<u32>(c, 0xA0, a32(r));
    while !k.is_null() {
        set_hierarchy_root_call(k, r);
        k = next(k);
    }
}

/// `RwFrameAddChildNoUpdate` (0x7F09C0): detach (+ mark dirty) when parented, prepend; no dirty
/// handling for the new position. Returns parent.
#[hook(addr = 0x7F09C0, conv = cdecl, name = "RwFrameAddChildNoUpdate", expect = "8B 4C 24 08 56")]
pub fn add_child_no_update(p: GPtr<u8>, c: GPtr<u8>) -> GPtr<u8> {
    if !parent(c).is_null() {
        detach(c);
        mark_dirty(c);
    }
    link_child(p, c);
    p
}

/// `RwFrameAddChild` (0x7F0B00): as NoUpdate, then the child leaves the dirty list (if it was a dirty
/// root) and the new root is marked dirty. Always prepends. Returns parent.
#[hook(addr = 0x7F0B00, conv = cdecl, name = "RwFrameAddChild", expect = "8B 4C 24 08 56")]
pub fn add_child(p: GPtr<u8>, c: GPtr<u8>) -> GPtr<u8> {
    if !parent(c).is_null() {
        detach(c);
        mark_dirty(c);
    }
    link_child(p, c);
    if flags(c) & 3 != 0 {
        unlink_dirty(c);
        set_flags(c, flags(c) & 0xFC);
    }
    mark_dirty(c);
    p
}

/// `RwFrameRemoveChild` (0x7F0CD0): no NULL check of the parent. Returns child.
#[hook(addr = 0x7F0CD0, conv = cdecl, name = "RwFrameRemoveChild", expect = "8B 4C 24 04 8B 41 04")]
pub fn remove_child(c: GPtr<u8>) -> GPtr<u8> {
    detach(c);
    mark_dirty(c);
    c
}

/// `RwFrameForAllChildren` (0x7F0DC0): direct children, next read before the callback, stops on NULL.
#[hook(addr = 0x7F0DC0, conv = cdecl, name = "RwFrameForAllChildren", expect = "53 55 8B 6C 24 0C")]
pub fn for_all_children(f: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    let mut c = child(f);
    while !c.is_null() {
        let n = next(c);
        if call_frame_cb(cb, c, data).is_null() {
            break;
        }
        c = n;
    }
    f
}

/// `RwFrameCount` (0x7F0E00): frames in the subtree including f.
#[hook(addr = 0x7F0E00, conv = cdecl, name = "RwFrameCount", expect = "8B 44 24 04 56")]
pub fn count(f: GPtr<u8>) -> i32 {
    let mut n: i32 = 1;
    let mut c = child(f);
    while !c.is_null() {
        let k = count_call(c);
        c = next(c);
        n = n.wrapping_add(k);
    }
    n
}

// ---- modelling matrix ----

/// `RwFrameTranslate` (0x7F0E30).
#[hook(addr = 0x7F0E30, conv = cdecl, name = "RwFrameTranslate", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn translate(f: GPtr<u8>, v: GPtr<u8>, op: i32) -> GPtr<u8> {
    matrix_translate(f.byte_add(0x10), v, op);
    mark_dirty(f);
    f
}

/// `RwFrameScale` (0x7F0ED0).
#[hook(addr = 0x7F0ED0, conv = cdecl, name = "RwFrameScale", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn scale(f: GPtr<u8>, v: GPtr<u8>, op: i32) -> GPtr<u8> {
    matrix_scale(f.byte_add(0x10), v, op);
    mark_dirty(f);
    f
}

/// `RwFrameTransform` (0x7F0F70).
#[hook(addr = 0x7F0F70, conv = cdecl, name = "RwFrameTransform", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn transform(f: GPtr<u8>, m: GPtr<u8>, op: i32) -> GPtr<u8> {
    matrix_transform(f.byte_add(0x10), m, op);
    mark_dirty(f);
    f
}

/// `RwFrameRotate` (0x7F1010): the angle is forwarded as its raw dword.
#[hook(addr = 0x7F1010, conv = cdecl, name = "RwFrameRotate", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn rotate(f: GPtr<u8>, axis: GPtr<u8>, angle: u32, op: i32) -> GPtr<u8> {
    matrix_rotate(f.byte_add(0x10), axis, angle, op);
    mark_dirty(f);
    f
}

/// `RwFrameSetIdentity` (0x7F10B0): identity rows; flags |= 0x20003 (other bits kept).
#[hook(addr = 0x7F10B0, conv = cdecl, name = "RwFrameSetIdentity", expect = "8B 44 24 04 B9 00 00 80 3F")]
pub fn set_identity(f: GPtr<u8>) -> GPtr<u8> {
    const ONE: u32 = 0x3F80_0000;
    for (off, v) in [
        (0x38, ONE),
        (0x24, ONE),
        (0x10, ONE),
        (0x20, 0),
        (0x18, 0),
        (0x14, 0),
        (0x34, 0),
        (0x30, 0),
        (0x28, 0),
        (0x48, 0),
        (0x44, 0),
        (0x40, 0),
    ] {
        wr::<u32>(f, off, v);
    }
    wr::<u32>(f, 0x1C, rd::<u32>(f, 0x1C) | 0x20003);
    mark_dirty(f);
    f
}

/// `RwFrameOrthoNormalize` (0x7F1170): in place.
#[hook(addr = 0x7F1170, conv = cdecl, name = "RwFrameOrthoNormalize", expect = "56 8B 74 24 08")]
pub fn ortho_normalize(f: GPtr<u8>) -> GPtr<u8> {
    let m = f.byte_add(0x10);
    matrix_ortho_normalize(m, m);
    mark_dirty(f);
    f
}

/// `RwFrameForAllObjects` (0x7F1200): `cb(link - 8, data)`, next link read before the callback.
#[hook(addr = 0x7F1200, conv = cdecl, name = "RwFrameForAllObjects", expect = "8B 44 24 04 53")]
pub fn for_all_objects(f: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    let head = f.byte_add(0x90);
    let mut l = p8(rd::<u32>(f, 0x90));
    while l != head {
        let n = p8(rd::<u32>(l, 0));
        if call_object_cb(cb, l.byte_add(-8), data).is_null() {
            break;
        }
        l = n;
    }
    f
}

// ---- plugins ----

/// `RwFrameRegisterPlugin` (0x7F1260).
#[hook(addr = 0x7F1260, conv = cdecl, name = "RwFrameRegisterPlugin", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 {
    registry_add_plugin(frame_registry(), size, id, ctor, dtor, copy)
}

/// `RwFrameRegisterPluginStream` (0x807580).
#[hook(addr = 0x807580, conv = cdecl, name = "RwFrameRegisterPluginStream", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn register_plugin_stream(id: u32, read: u32, write: u32, get_size: u32) -> i32 {
    registry_add_plugin_stream(frame_registry(), id, read, write, get_size)
}

// ---- RwObjectHasFrame ----

/// `_rwObjectHasFrameSetFrame` (0x804EF0): unlink from the old frame, link at the head of the new
/// frame's object list and mark it dirty. EAX on return is not meaningful in the original (void here).
#[hook(addr = 0x804EF0, conv = cdecl, name = "_rwObjectHasFrameSetFrame", expect = "8B 4C 24 04 8B 41 04")]
pub fn object_has_frame_set_frame(obj: GPtr<u8>, frame: GPtr<u8>) {
    if rd::<u32>(obj, 4) != 0 {
        unlink_link(obj.byte_add(8));
    }
    wr::<u32>(obj, 4, a32(frame));
    if frame.is_null() {
        return;
    }
    let list = frame.byte_add(0x90);
    let link = obj.byte_add(8);
    let old = rd::<u32>(list, 0);
    wr::<u32>(obj, 0xC, a32(list));
    wr::<u32>(link, 0, old);
    wr::<u32>(p8(rd::<u32>(list, 0)), 4, a32(link));
    wr::<u32>(list, 0, a32(link));
    update_objects_call(frame);
}

/// `_rwObjectHasFrameReleaseFrame` (0x804F40): unlink only; `parent` is kept. Returns EAX = obj.
#[hook(addr = 0x804F40, conv = cdecl, name = "_rwObjectHasFrameReleaseFrame", expect = "8B 44 24 04 8B 48 04")]
pub fn object_has_frame_release_frame(obj: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(obj, 4) != 0 {
        unlink_link(obj.byte_add(8));
    }
    obj
}

// ---- frame lists ----

/// `RwFrameListSetAutoUpdate` (0x807570). Returns EAX = the value.
#[hook(addr = 0x807570, conv = cdecl, name = "RwFrameListSetAutoUpdate", expect = "8B 44 24 04 A3 A8 26 8E 00")]
pub fn frame_list_set_auto_update(v: i32) -> i32 {
    FRAME_LIST_AUTO_UPDATE.set(v);
    v
}

/// `_rwFrameListDeinitialize` (0x807720): frees the frame array when `numFrames != 0`; frames stay.
#[hook(addr = 0x807720, conv = cdecl, name = "_rwFrameListDeinitialize", expect = "56 8B 74 24 08")]
pub fn frame_list_deinitialize(list: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(list, 4) != 0 {
        rw_free(p8(rd::<u32>(list, 0)));
    }
    list
}

/// `_rwFrameListStreamRead` (0x807970). `numFrames` is the local read from the stream (not re-read
/// from the list); a parent index is not bounds-checked; frames created before a read failure leak.
/// The three matrix tests compare the callees' ST0 results (received as f64; exact under the game's
/// 24-bit precision control) with 0.01f / 0.99f.
#[hook(addr = 0x807970, conv = cdecl, name = "_rwFrameListStreamRead", expect = "83 EC 4C 8D 44 24 04")]
pub fn frame_list_stream_read(s: GPtr<u8>, list: GPtr<u8>) -> GPtr<u8> {
    let mut version = [0u32; 1];
    let mut length = [0u32; 1];
    if stream_find_chunk(
        s,
        1,
        GPtr::from_raw(length.as_mut_ptr()),
        GPtr::from_raw(version.as_mut_ptr()),
    ) == 0
    {
        return GPtr::NULL;
    }
    let v = version[0];
    if !(0x34000..=0x36003).contains(&v) {
        error1(0x8000_0004);
        return GPtr::NULL;
    }
    let mut nf_buf = [0i32; 1];
    if stream_read(s, GPtr::from_raw(nf_buf.as_mut_ptr()).cast(), 4) != 4 {
        return GPtr::NULL;
    }
    let nf = nf_buf[0];
    wr::<i32>(list, 4, nf);
    let frames = rw_malloc((nf as u32).wrapping_shl(2), 0x3000E);
    wr::<u32>(list, 0, a32(frames));
    if frames.is_null() {
        error2(0x8000_0013, (nf as u32).wrapping_mul(4));
        return GPtr::NULL;
    }
    let mut buf = [0u32; 14];
    let mut i: i32 = 0;
    while i < nf {
        if stream_read(s, GPtr::from_raw(buf.as_mut_ptr()).cast(), 0x38) != 0x38 {
            rw_free(p8(rd::<u32>(list, 0)));
            return GPtr::NULL;
        }
        let fr = create_call();
        if fr.is_null() {
            rw_free(p8(rd::<u32>(list, 0)));
            return GPtr::NULL;
        }
        let m = fr.byte_add(0x10);
        for row in 0..4 {
            for k in 0..3 {
                wr::<u32>(m, row * 0x10 + k * 4, buf[(row * 3 + k) as usize]);
            }
        }
        let tol = MATRIX_ERROR_TOL as f64;
        let ok = matrix_normal_error(m) <= tol
            && matrix_orthogonal_error(m) <= tol
            && matrix_determinant(m) >= MATRIX_DET_MIN as f64;
        let mask = if ok { 0xFFFD_FFFF } else { 0xFFFD_FFFC };
        wr::<u32>(m, 0xC, rd::<u32>(m, 0xC) & mask);
        let arr = p8(rd::<u32>(list, 0));
        wr::<u32>(arr, (i as isize) * 4, a32(fr));
        let pi = buf[12] as i32;
        if pi >= 0 {
            let auto = FRAME_LIST_AUTO_UPDATE.get();
            let p = p8(rd::<u32>(p8(rd::<u32>(list, 0)), (pi as isize) * 4));
            if auto != 0 {
                add_child_call(p, fr);
            } else {
                add_child_no_update_call(p, fr);
            }
        }
        i += 1;
    }
    let mut i: i32 = 0;
    while i < nf {
        let fr = p8(rd::<u32>(p8(rd::<u32>(list, 0)), (i as isize) * 4));
        if registry_read_data_chunks(frame_registry(), s, fr).is_null() {
            destroy_hierarchy_call(p8(rd::<u32>(p8(rd::<u32>(list, 0)), 0)));
            rw_free(p8(rd::<u32>(list, 0)));
            return GPtr::NULL;
        }
        i += 1;
    }
    list
}

// ---- dirty list sync ----

/// `RwMatrixMultiply(&f->ltm, &f->modelling, &f->parent->ltm)`.
fn multiply_ltm(f: GPtr<u8>) {
    matrix_multiply(f.byte_add(0x50), f.byte_add(0x10), parent(f).byte_add(0x50));
}

/// `_rwFrameSyncDirty` (0x809550): syncs every root in the dirty list, then empties the list (frames are
/// not unlinked one by one). The root's flags are written from the snapshot taken before its sync.
#[hook(addr = 0x809550, conv = cdecl, name = "_rwFrameSyncDirty", expect = "83 EC 10 A1 24 7B C9 00")]
pub fn sync_dirty() -> u32 {
    let head = rw_globals().byte_add(DIRTY_LIST);
    let mut link = p8(rd::<u32>(head, 0));
    while link != head {
        let fr = link.byte_add(-8);
        let fl = flags(fr) as u32;
        if fl & 1 != 0 {
            let inh = fl & 4;
            if inh != 0 {
                copy_matrix(fr.byte_add(0x50), fr.byte_add(0x10));
            }
            sync_objects(fr);
            let mut c = child(fr);
            while !c.is_null() {
                let cf = flags(c) as u32 | inh;
                if cf & 4 != 0 {
                    multiply_ltm(c);
                }
                sync_objects(c);
                let k = child(c);
                set_flags(c, flags(c) & 0xF3);
                sync_dirty_ltm_call(k, cf);
                c = next(c);
            }
        } else {
            sync_objects(fr);
            let mut c = child(fr);
            while !c.is_null() {
                sync_objects(c);
                let k = child(c);
                set_flags(c, flags(c) & 0xF7);
                sync_dirty_objects_call(k);
                c = next(c);
            }
        }
        set_flags(fr, (fl & 0xF0) as u8);
        link = p8(rd::<u32>(link, 0));
    }
    let h = rw_globals().byte_add(DIRTY_LIST);
    wr::<u32>(h, 0, a32(h));
    let h = rw_globals().byte_add(DIRTY_LIST);
    wr::<u32>(rw_globals(), DIRTY_LIST + 4, a32(h));
    1
}

/// `_rwFrameSyncDirtyLTM` (0x809700): siblings loop with recursion into children.
#[hook(addr = 0x809700, conv = cdecl, name = "_rwFrameSyncDirtyLTM", expect = "56 8B 74 24 08")]
pub fn sync_dirty_ltm(f: GPtr<u8>, parent_flags: u32) {
    let mut f = f;
    while !f.is_null() {
        let cf = flags(f) as u32 | parent_flags;
        if cf & 4 != 0 {
            multiply_ltm(f);
        }
        sync_objects(f);
        set_flags(f, flags(f) & 0xF3);
        sync_dirty_ltm_call(child(f), cf);
        f = next(f);
    }
}

/// `_rwFrameSyncDirtyObjects` (0x809780).
#[hook(addr = 0x809780, conv = cdecl, name = "_rwFrameSyncDirtyObjects", expect = "55 8B 6C 24 08")]
pub fn sync_dirty_objects(f: GPtr<u8>) {
    let mut f = f;
    while !f.is_null() {
        sync_objects(f);
        set_flags(f, flags(f) & 0xF7);
        sync_dirty_objects_call(child(f));
        f = next(f);
    }
}

/// `_rwFrameSyncHierarchyLTM` (0x8097D0): LTMs only; the root keeps HIERARCHYSYNCOBJ (stays in the
/// dirty list). Its flags are written from the snapshot.
#[hook(addr = 0x8097D0, conv = cdecl, name = "_rwFrameSyncHierarchyLTM", expect = "55 8B 6C 24 08")]
pub fn sync_hierarchy_ltm(r: GPtr<u8>) {
    let fl = flags(r) as u32;
    if fl & 4 != 0 {
        copy_matrix(r.byte_add(0x50), r.byte_add(0x10));
    }
    let mut c = child(r);
    while !c.is_null() {
        let cf = flags(c) as u32 | fl;
        if cf & 4 != 0 {
            multiply_ltm(c);
            set_flags(c, flags(c) & 0xFB);
        }
        sync_hierarchy_ltm_recurse_call(child(c), cf);
        c = next(c);
    }
    set_flags(r, (fl & 0xFA) as u8);
}

/// `_rwFrameSyncHierarchyLTMRecurse` (0x809850).
#[hook(addr = 0x809850, conv = cdecl, name = "_rwFrameSyncHierarchyLTMRecurse", expect = "56 8B 74 24 08")]
pub fn sync_hierarchy_ltm_recurse(f: GPtr<u8>, parent_flags: u32) {
    let mut f = f;
    while !f.is_null() {
        let cf = flags(f) as u32 | parent_flags;
        if cf & 4 != 0 {
            multiply_ltm(f);
            set_flags(f, flags(f) & 0xFB);
        }
        sync_hierarchy_ltm_recurse_call(child(f), cf);
        f = next(f);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_MODULE_OPEN,
    HOOK_MODULE_CLOSE,
    HOOK_CLONE_AND_LINK_CLONES,
    HOOK_CLONE_RECURSE,
    HOOK_PURGE_CLONE,
    HOOK_CLONE_HIERARCHY,
    HOOK_DIRTY,
    HOOK_INTERNAL_INIT,
    HOOK_CREATE,
    HOOK_FRAME_INIT,
    HOOK_FRAME_DEINIT,
    HOOK_DESTROY,
    HOOK_DESTROY_RECURSE,
    HOOK_DESTROY_HIERARCHY,
    HOOK_UPDATE_OBJECTS,
    HOOK_GET_LTM,
    HOOK_GET_ROOT,
    HOOK_ADD_CHILD_NO_UPDATE,
    HOOK_ADD_CHILD,
    HOOK_REMOVE_CHILD,
    HOOK_FOR_ALL_CHILDREN,
    HOOK_COUNT,
    HOOK_TRANSLATE,
    HOOK_SCALE,
    HOOK_TRANSFORM,
    HOOK_ROTATE,
    HOOK_SET_IDENTITY,
    HOOK_ORTHO_NORMALIZE,
    HOOK_FOR_ALL_OBJECTS,
    HOOK_REGISTER_PLUGIN,
    HOOK_REGISTER_PLUGIN_STREAM,
    HOOK_OBJECT_HAS_FRAME_SET_FRAME,
    HOOK_OBJECT_HAS_FRAME_RELEASE_FRAME,
    HOOK_FRAME_LIST_SET_AUTO_UPDATE,
    HOOK_FRAME_LIST_DEINITIALIZE,
    HOOK_FRAME_LIST_STREAM_READ,
    HOOK_SYNC_DIRTY,
    HOOK_SYNC_DIRTY_LTM,
    HOOK_SYNC_DIRTY_OBJECTS,
    HOOK_SYNC_HIERARCHY_LTM,
    HOOK_SYNC_HIERARCHY_LTM_RECURSE,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = FRAME_GLOBALS;
