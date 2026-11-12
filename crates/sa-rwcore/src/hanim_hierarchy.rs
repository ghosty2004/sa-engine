//! RpHAnim hierarchy create / destroy / attach / detach / lookup (STD-6, batch `std6-rw-1`). Ported
//! from the 1.0 US bytes (0x7C4C30..0x7C5120, 0x7C51A0). Layouts as in `hanim_stream.rs`.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, rw_free, rw_malloc, rw_memory_alloc, rw_memory_free, wr};

orig! {
    /// `RtAnimInterpolatorCreate(numNodes, keyFrameSize)` (0x7CD520)
    fn interpolator_create(n: i32, kfs: i32) -> GPtr<u8> = cdecl 0x7CD520 => crate::hanim_stream::rt_anim_interpolator_create;
    /// `RtAnimInterpolatorDestroy(interp)` (0x7CD590)
    fn interpolator_destroy(i: GPtr<u8>) = cdecl 0x7CD590 => crate::hanim_stream::rt_anim_interpolator_destroy;
    /// `RpHAnimHierarchyCreate` (0x7C4C30)
    fn hierarchy_create_call(n: i32, flags_in: GPtr<u32>, ids: GPtr<i32>, flags: u32, kfs: i32) -> GPtr<u8> = cdecl 0x7C4C30 => rp_hanim_hierarchy_create;
    /// `RwFrameUpdateObjects(frame)` (0x7F0910)
    fn frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => crate::frame::update_objects;
    /// `RwFrameForAllChildren(frame, cb, data)` (0x7F0DC0)
    fn frame_for_all_children(f: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x7F0DC0 => crate::frame::for_all_children;
}

const HANIM_FREELIST: usize = 0xC9B8CC;
const HANIM_FRAME_OFFSET: usize = 0xC9B8C8;
const HINT: u32 = 0x3011E;

fn ext(frame: GPtr<u8>) -> GPtr<u8> {
    frame.byte_add(g::<i32>(HANIM_FRAME_OFFSET).read() as isize)
}

fn node(h: GPtr<u8>, i: i32) -> GPtr<u8> {
    p8(rd::<u32>(h, 0x10)).byte_add(i as isize * 0x10)
}

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// `RpHAnimHierarchyCreate(numNodes, nodeFlags, nodeIDs, flags, maxKeyFrameSize)` (0x7C4C30): from the
/// freelist (not zeroed, not checked); +0x20 a new interpolator, +0 flags, +4 numNodes, +0x14 0;
/// matrices (`numNodes * 64 + 15`, 16-aligned at +8, raw at +0xC) unless flag 2; node info
/// (`numNodes * 16`) at +0x10: frame 0, index i, id / flags from the arrays when given (left as
/// allocated otherwise); +0x18 = itself.
#[hook(addr = 0x7C4C30, conv = cdecl, name = "RpHAnimHierarchyCreate", expect = "A1 CC B8 C9 00 8B 0D 24 7B C9 00",
       route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn rp_hanim_hierarchy_create(n: i32, node_flags: GPtr<u32>, ids: GPtr<i32>, flags: u32, kfs: i32) -> GPtr<u8> {
    let h = rw_memory_alloc(p8(g::<u32>(HANIM_FREELIST).read()), HINT);
    wr::<u32>(h, 0x20, a32(interpolator_create(n, kfs)));
    wr::<u32>(h, 0, flags);
    wr::<i32>(h, 4, n);
    wr::<u32>(h, 0x14, 0);
    if flags & 2 == 0 {
        let m = rw_malloc(((n as u32) << 6).wrapping_add(0xF), HINT);
        wr::<u32>(h, 0xC, a32(m));
        wr::<u32>(h, 8, a32(m).wrapping_add(0xF) & !0xF);
    } else {
        wr::<u32>(h, 8, 0);
        wr::<u32>(h, 0xC, 0);
    }
    wr::<u32>(h, 0x10, a32(rw_malloc((n as u32) << 4, HINT)));
    for i in 0..n.max(0) {
        let nd = node(h, i);
        wr::<u32>(nd, 0xC, 0);
        if !ids.is_null() {
            wr::<i32>(nd, 0, ids.byte_add(i as isize * 4).read());
        }
        wr::<i32>(nd, 4, i);
        if !node_flags.is_null() {
            wr::<u32>(nd, 8, node_flags.byte_add(i as isize * 4).read());
        }
    }
    wr::<u32>(h, 0x18, a32(h));
    h
}

/// `RpHAnimHierarchyDestroy(hierarchy)` (0x7C4D30): unless flag 1 frees the matrices (not with flag 2)
/// and the node info; clears +0xC, +8, +0x10, destroys the interpolator, returns the hierarchy to its
/// freelist and clears the parent frame's hierarchy word. Returns NULL.
#[hook(addr = 0x7C4D30, conv = cdecl, name = "RpHAnimHierarchyDestroy", expect = "53 56 8B 74 24 0C 57 8B 06",
       unverified = "std6-rw-1", route = "L2", hazards = "heap-writes")]
pub fn rp_hanim_hierarchy_destroy(h: GPtr<u8>) -> u32 {
    let flags = rd::<u32>(h, 0);
    let parent = p8(rd::<u32>(h, 0x14));
    if flags & 1 == 0 {
        if flags & 2 == 0 {
            rw_free(p8(rd::<u32>(h, 0xC)));
        }
        rw_free(p8(rd::<u32>(h, 0x10)));
    }
    let interp = p8(rd::<u32>(h, 0x20));
    wr::<u32>(h, 0xC, 0);
    wr::<u32>(h, 8, 0);
    wr::<u32>(h, 0x10, 0);
    interpolator_destroy(interp);
    rw_memory_free(p8(g::<u32>(HANIM_FREELIST).read()), h);
    if !parent.is_null() {
        wr::<u32>(ext(parent), 4, 0);
    }
    0
}

/// `RpHAnimHierarchyCreateFromHierarchy(hierarchy, flags, maxKeyFrameSize)` (0x7C4ED0): a new hierarchy
/// of the same node count whose nodes copy flags, index and id (frames cleared).
#[hook(addr = 0x7C4ED0, conv = cdecl, name = "RpHAnimHierarchyCreateFromHierarchy", expect = "8B 4C 24 0C 8B 54 24 08 56",
       route = "L2", hazards = "heap-writes")]
pub fn rp_hanim_hierarchy_create_from_hierarchy(src: GPtr<u8>, flags: u32, kfs: i32) -> GPtr<u8> {
    let h = hierarchy_create_call(rd::<i32>(src, 4), GPtr::NULL, GPtr::NULL, flags, kfs);
    let mut i = 0;
    while i < rd::<i32>(h, 4) {
        let (d, s) = (node(h, i), node(src, i));
        wr::<u32>(d, 0xC, 0);
        wr::<u32>(d, 8, rd::<u32>(s, 8));
        wr::<u32>(d, 4, rd::<u32>(s, 4));
        wr::<u32>(d, 0, rd::<u32>(s, 0));
        i += 1;
    }
    h
}

/// Binds every node whose id equals `frame`'s HAnim id to `frame`.
fn bind_frame(h: GPtr<u8>, frame: GPtr<u8>) {
    let mut i = 0;
    while i < rd::<i32>(h, 4) {
        let nd = node(h, i);
        if rd::<u32>(ext(frame), 0) == rd::<u32>(nd, 0) {
            wr::<u32>(nd, 0xC, a32(frame));
        }
        i += 1;
    }
}

/// `RpHAnimHierarchyAttach(hierarchy)` (0x7C4F40): binds the parent frame and, through
/// `_rpHAnimHierarchyAttachFrameCB`, every descendant whose id matches a node; each bound frame's
/// objects are updated (`RwFrameUpdateObjects`). Returns the hierarchy.
#[hook(addr = 0x7C4F40, conv = cdecl, name = "RpHAnimHierarchyAttach", expect = "56 8B 74 24 08 33 D2 57",
       unverified = "std6-rw-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn rp_hanim_hierarchy_attach(h: GPtr<u8>) -> GPtr<u8> {
    let parent = p8(rd::<u32>(h, 0x14));
    bind_frame(h, parent);
    frame_update_objects(parent);
    frame_for_all_children(parent, 0x7C4F90, a32(h));
    h
}

/// `_rpHAnimHierarchyAttachFrameCB(frame, hierarchy)` (0x7C4F90): as `Attach` for one frame and its
/// children. Returns the frame.
#[hook(addr = 0x7C4F90, conv = cdecl, name = "_rpHAnimHierarchyAttachFrameCB", expect = "56 8B 74 24 0C 33 D2 57",
       unverified = "std6-rw-1", route = "L2", hazards = "none")]
pub fn rp_hanim_hierarchy_attach_frame_cb(frame: GPtr<u8>, h: GPtr<u8>) -> GPtr<u8> {
    bind_frame(h, frame);
    frame_update_objects(frame);
    frame_for_all_children(frame, 0x7C4F90, a32(h));
    frame
}

/// `RpHAnimHierarchyDetach(hierarchy)` (0x7C4FF0): every node's frame cleared. Returns the hierarchy.
#[hook(addr = 0x7C4FF0, conv = cdecl, name = "RpHAnimHierarchyDetach", expect = "8B 44 24 04 33 C9 8B 50 04",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn rp_hanim_hierarchy_detach(h: GPtr<u8>) -> GPtr<u8> {
    let mut i = 0;
    while i < rd::<i32>(h, 4) {
        wr::<u32>(node(h, i), 0xC, 0);
        i += 1;
    }
    h
}

/// Binds the nodes with id `id` to `frame` when the frame's HAnim id is `id`.
fn bind_frame_index(h: GPtr<u8>, frame: GPtr<u8>, id: u32) {
    if rd::<u32>(ext(frame), 0) != id {
        return;
    }
    let mut i = 0;
    while i < rd::<i32>(h, 4) {
        let nd = node(h, i);
        if rd::<u32>(nd, 0) == id {
            wr::<u32>(nd, 0xC, a32(frame));
        }
        i += 1;
    }
}

/// `RpHAnimHierarchyAttachFrameIndex(hierarchy, index)` (0x7C5020): binds node `index`'s id to the
/// parent frame or, through `_rpHAnimHierarchyAttachFrameIndexCB`, to the descendant carrying it.
/// Returns the hierarchy.
#[hook(addr = 0x7C5020, conv = cdecl, name = "RpHAnimHierarchyAttachFrameIndex", expect = "83 EC 08 8B 44 24 10 53",
       unverified = "std6-rw-1", route = "L2", hazards = "none")]
pub fn rp_hanim_hierarchy_attach_frame_index(h: GPtr<u8>, index: i32) -> GPtr<u8> {
    let id = rd::<u32>(node(h, index), 0);
    let parent = p8(rd::<u32>(h, 0x14));
    bind_frame_index(h, parent, id);
    frame_update_objects(parent);
    let mut data = [a32(h), id];
    frame_for_all_children(parent, 0x7C50A0, GPtr::from_raw(data.as_mut_ptr()).addr() as u32);
    h
}

/// `_rpHAnimHierarchyAttachFrameIndexCB(frame, data {hierarchy, id})` (0x7C50A0). Returns the frame.
#[hook(addr = 0x7C50A0, conv = cdecl, name = "_rpHAnimHierarchyAttachFrameIndexCB", expect = "A1 C8 B8 C9 00 53 8B 5C 24 08",
       unverified = "std6-rw-1", route = "L2", hazards = "none")]
pub fn rp_hanim_hierarchy_attach_frame_index_cb(frame: GPtr<u8>, data: GPtr<u32>) -> GPtr<u8> {
    let h = p8(data.read());
    let id = data.byte_add(4).read();
    bind_frame_index(h, frame, id);
    frame_update_objects(frame);
    frame_for_all_children(frame, 0x7C50A0, data.addr() as u32);
    frame
}

/// `RpHAnimHierarchyDetachFrameIndex(hierarchy, index)` (0x7C5100): node `index`'s frame cleared;
/// returns the hierarchy (EAX keeps it).
#[hook(addr = 0x7C5100, conv = cdecl, name = "RpHAnimHierarchyDetachFrameIndex", expect = "8B 44 24 04 8B 4C 24 08 C1 E1 04",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn rp_hanim_hierarchy_detach_frame_index(h: GPtr<u8>, index: i32) -> GPtr<u8> {
    wr::<u32>(node(h, index), 0xC, 0);
    h
}

/// `RpHAnimHierarchyGetMatrixArray(hierarchy)` (0x7C5120): +8.
#[hook(addr = 0x7C5120, conv = cdecl, name = "RpHAnimHierarchyGetMatrixArray", expect = "8B 44 24 04 8B 40 08 C3",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn rp_hanim_hierarchy_get_matrix_array(h: GPtr<u8>) -> u32 {
    rd::<u32>(h, 8)
}

/// `RpHAnimIDGetIndex(hierarchy, id)` (0x7C51A0): the first node with that id, else -1.
#[hook(addr = 0x7C51A0, conv = cdecl, name = "RpHAnimIDGetIndex", expect = "8B 54 24 04 56 83 C8 FF",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 {
    let n = rd::<i32>(h, 4);
    for i in 0..n.max(0) {
        if rd::<i32>(node(h, i), 0) == id {
            return i;
        }
    }
    -1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RP_HANIM_HIERARCHY_CREATE,
    HOOK_RP_HANIM_HIERARCHY_DESTROY,
    HOOK_RP_HANIM_HIERARCHY_CREATE_FROM_HIERARCHY,
    HOOK_RP_HANIM_HIERARCHY_ATTACH,
    HOOK_RP_HANIM_HIERARCHY_ATTACH_FRAME_CB,
    HOOK_RP_HANIM_HIERARCHY_DETACH,
    HOOK_RP_HANIM_HIERARCHY_ATTACH_FRAME_INDEX,
    HOOK_RP_HANIM_HIERARCHY_ATTACH_FRAME_INDEX_CB,
    HOOK_RP_HANIM_HIERARCHY_DETACH_FRAME_INDEX,
    HOOK_RP_HANIM_HIERARCHY_GET_MATRIX_ARRAY,
    HOOK_RP_HANIM_ID_GET_INDEX,
];
