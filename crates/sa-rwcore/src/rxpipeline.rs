//! Rx heap and the pipeline module's open/close (PORT-RW-1 follow-up, written for the native boot).
//! Ported from the 1.0 US bytes; RW has no gta-reversed source.
//!
//! `RxHeap` (0x1C): +0 superBlockSize, +4 head block, +8 first free header, +0xC entry array
//! (8 bytes per entry: +0 size, +4 header), +0x10 entries allocated, +0x14 entries used, +0x18 dirty.
//! Heap block (`rwmalloc(size + 0x8B)`): +0 memory (block + 0x8B rounded down to 0x80), +4 size,
//! +8 next block. Header (0x20 bytes at the start and at the end of a block's memory): +0 prev,
//! +4 next, +8 size, +0xC entry.
//!
//! Pipeline module `P = G + [0xC9BC60]` (the offset is written by the pipe engine module, engine.rs):
//! +0 pipeline freelist, +4 default `RxRenderStateVector` (0x2C), +0x30, +0x34, +0x38 max nodes.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{
    error1, error2, g, p8, rd, rw_free, rw_globals, rw_malloc, rw_memory_alloc, rw_memory_free, rw_realloc, wr,
};

/// Pipeline-module offset global (`G + [0xC9BC60]`).
const PIPE_MODULE: usize = 0xC9BC60;

sa_core::globals! {
    list RX_GLOBALS;
    /// `_rxPipelineOpen` done flag
    pub static PIPELINE_OPEN @ 0xC9A6AC: u32 = 0;
    /// The global `RxHeap*` created by `_rxPipelineOpen`
    pub static RX_HEAP_GLOBAL @ 0xC9A6B0: u32 = 0;
    /// Static space of the pipeline freelist (`RwFreeList`, 0x24 bytes)
    pub static PIPELINE_FREELIST_SPACE @ 0xC9A688: [u32; 9] = [0; 9];
    /// Global heap size passed to `RxHeapCreate` (0x1000)
    pub static RX_HEAP_INITIAL_SIZE @ 0x8E2698: u32 = 0x1000;
    /// Default max nodes per pipeline, stored at `P + 0x38` (0x40)
    pub static PIPELINE_MAX_NODES @ 0x8E269C: u32 = 0x40;
    /// Pipeline freelist entries per block (0x40)
    pub static PIPELINE_FL_PER_BLOCK @ 0x8E26A0: u32 = 0x40;
    /// Pipeline freelist preallocated blocks (1)
    pub static PIPELINE_FL_PREALLOC @ 0x8E26A4: u32 = 1;
}

orig! {
    /// `RwFreeListCreateAndPreallocateSpace` (0x801B70)
    fn free_list_create_prealloc(size: u32, per_block: u32, align: u32, n: u32, space: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x801B70 => crate::memory::rw_freelist_create_and_preallocate_space;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    fn free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => crate::memory::rw_freelist_destroy;
    /// `RxHeapCreate(size)` (0x809F90)
    fn rx_heap_create_call(size: u32) -> GPtr<u8> = cdecl 0x809F90 => rx_heap_create;
    /// `RxHeapDestroy(heap)` (0x809F30)
    fn rx_heap_destroy_call(heap: GPtr<u8>) = cdecl 0x809F30 => rx_heap_destroy;
    /// `_rxHeapAddBlock(block, prevBlock, heap)` (0x809BF0)
    fn heap_add_block_call(block: GPtr<u8>, prev: GPtr<u8>, heap: GPtr<u8>) -> i32 = cdecl 0x809BF0 => rx_heap_add_block;
    /// `_rxHeapGetEntry(heap)` (0x8099F0)
    fn heap_new_entry_call(heap: GPtr<u8>) -> GPtr<u8> = cdecl 0x8099F0 => rx_heap_new_entry;
    /// `RxRenderStateVectorSetDefaultRenderStateVector(rsvp)` (0x80EA40)
    fn rsv_set_default_call(rsvp: GPtr<u8>) -> GPtr<u8> = cdecl 0x80EA40 => rx_render_state_vector_set_default;
    /// `_rxPipelineNodeDestroyRecurse(node, pipeline)` (0x8050C0)
    fn pipeline_node_destroy_recurse(node: GPtr<u8>, p: GPtr<u8>) = cdecl 0x8050C0 => unported;
}

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// `P = G + [0xC9BC60]`, recomputed on every access as the original reloads both globals.
fn pmod() -> GPtr<u8> {
    let off = g::<u32>(PIPE_MODULE).read();
    rw_globals().byte_add(off as isize)
}

/// `_rxPipelineOpen` (0x804FE0): creates the global Rx heap (`RxHeapCreate([0x8E2698])`), the
/// pipeline freelist (entry 0x34, hint 0x40409, static space 0xC9A688; failure destroys the heap),
/// stores the default max node count at `P+0x38`, sets the default render-state vector at `P+4`,
/// clears `P+0x34` / `P+0x30` and sets the open flag. Quirk kept: when the flag is already set it
/// returns FALSE (not TRUE) and changes nothing.
#[hook(addr = 0x804FE0, conv = cdecl, name = "_rxPipelineOpen", expect = "A1 AC A6 C9 00")]
pub fn rx_pipeline_open() -> i32 {
    if PIPELINE_OPEN.get() != 0 {
        return 0;
    }
    let heap = rx_heap_create_call(RX_HEAP_INITIAL_SIZE.get());
    RX_HEAP_GLOBAL.set(a32(heap));
    if heap.is_null() {
        return 0;
    }
    let fl = free_list_create_prealloc(
        0x34,
        PIPELINE_FL_PER_BLOCK.get(),
        4,
        PIPELINE_FL_PREALLOC.get(),
        PIPELINE_FREELIST_SPACE.ptr().cast(),
        0x40409,
    );
    wr::<u32>(pmod(), 0, a32(fl));
    if rd::<u32>(pmod(), 0) == 0 {
        rx_heap_destroy_call(p8(RX_HEAP_GLOBAL.get()));
        RX_HEAP_GLOBAL.set(0);
        return 0;
    }
    wr::<u32>(pmod(), 0x38, PIPELINE_MAX_NODES.get());
    rsv_set_default_call(pmod().byte_add(4));
    wr::<u32>(pmod(), 0x34, 0);
    wr::<u32>(pmod(), 0x30, 0);
    PIPELINE_OPEN.set(1);
    1
}

/// `_rxPipelineClose` (0x804F60): when open, destroys the pipeline freelist (`P+0` = NULL) and the
/// global heap, then clears the heap pointer and the open flag. Always returns TRUE.
#[hook(addr = 0x804F60, conv = cdecl, name = "_rxPipelineClose", expect = "A1 AC A6 C9 00")]
pub fn rx_pipeline_close() -> i32 {
    if PIPELINE_OPEN.get() != 0 {
        free_list_destroy(p8(rd::<u32>(pmod(), 0)));
        wr::<u32>(pmod(), 0, 0);
        rx_heap_destroy_call(p8(RX_HEAP_GLOBAL.get()));
        RX_HEAP_GLOBAL.set(0);
        PIPELINE_OPEN.set(0);
    }
    1
}

/// `RxHeapDestroy(heap)` (0x809F30): frees the entry array (and clears it), every block of the
/// block list, then the heap. NULL is ignored.
#[hook(addr = 0x809F30, conv = cdecl, name = "RxHeapDestroy", expect = "57 8B 7C 24 08")]
pub fn rx_heap_destroy(heap: GPtr<u8>) {
    if heap.is_null() {
        return;
    }
    let e = rd::<u32>(heap, 0xC);
    if e != 0 {
        rw_free(p8(e));
        wr::<u32>(heap, 0xC, 0);
    }
    let mut b = rd::<u32>(heap, 4);
    while b != 0 {
        let next = rd::<u32>(p8(b), 8);
        rw_free(p8(b));
        b = next;
    }
    rw_free(heap);
}

/// `RxHeapCreate(size)` (0x809F90): size at least 0x400, rounded up to 0x20 (then at least 0x80);
/// heap struct `rwmalloc(0x1C, 0x40409)`, first block `rwmalloc(size + 0x8B, 0x1040409)` with its
/// memory 0x80-aligned. Then the inlined reset as the bytes do it: the loop over the blocks after the
/// head (always empty here: the new block's `next` is 0), then `_rxHeapAddBlock(head, prev, heap)`;
/// `heap+8` = the head block's memory when no previous block was added; dirty flag cleared. On a
/// failed add both allocations are freed. Returns the heap or NULL.
#[hook(addr = 0x809F90, conv = cdecl, name = "RxHeapCreate", expect = "53 55 56 57 8B 7C 24 14")]
pub fn rx_heap_create(size: u32) -> GPtr<u8> {
    let mut sz = if size < 0x400 { 0x400 } else { size };
    let heap = rw_malloc(0x1C, 0x40409);
    if heap.is_null() {
        return GPtr::NULL;
    }
    sz = sz.wrapping_add(0x1F) & !0x1F;
    if sz < 0x80 {
        sz = 0x80;
    }
    let blk = rw_malloc(sz.wrapping_add(0x8B), 0x1040409);
    if blk.is_null() {
        rw_free(heap);
        return GPtr::NULL;
    }
    let mem = a32(blk).wrapping_add(0x8B) & !0x7F;
    wr::<u32>(blk, 4, sz);
    wr::<u32>(blk, 8, 0);
    wr::<u32>(blk, 0, mem);
    wr::<u32>(heap, 0, sz);
    wr::<u32>(heap, 4, a32(blk));
    wr::<u32>(heap, 0xC, 0);
    wr::<u32>(heap, 0x10, 0);
    wr::<u32>(heap, 0x18, 1);
    wr::<u32>(heap, 0x14, 0);
    let mut prev: u32 = 0;
    let mut b = rd::<u32>(blk, 8);
    let fail = |blk: GPtr<u8>, heap: GPtr<u8>| {
        rw_free(blk);
        rw_free(heap);
        GPtr::NULL
    };
    while b != 0 {
        if heap_add_block_call(p8(b), p8(prev), heap) == 0 {
            return fail(blk, heap);
        }
        if prev == 0 {
            wr::<u32>(heap, 8, rd::<u32>(p8(b), 0));
        }
        prev = b;
        b = rd::<u32>(p8(b), 8);
    }
    let head = p8(rd::<u32>(heap, 4));
    if heap_add_block_call(head, p8(prev), heap) == 0 {
        return fail(blk, heap);
    }
    if prev == 0 {
        wr::<u32>(heap, 8, rd::<u32>(head, 0));
    }
    wr::<u32>(heap, 0x18, 0);
    heap
}

/// `_rxHeapAddBlock(block, prevBlock, heap)` (0x809BF0, catalogue `FUN_00809bf0`): takes a free
/// entry; clears the first four words of the block's memory and copies its first 0x20 bytes to the
/// end header (`mem + size - 0x20`); one free header at `mem + 0x20` spanning to the end header,
/// linked both ways, with size `size - 0x60` and the entry; entry = {that size, header}. With a
/// previous block, its end header's `next` and this block's start header's `prev` link the two.
/// Returns FALSE when no entry could be had.
#[hook(addr = 0x809BF0, conv = cdecl, name = "FUN_00809bf0", expect = "8B 44 24 0C 56")]
pub fn rx_heap_add_block(block: GPtr<u8>, prev: GPtr<u8>, heap: GPtr<u8>) -> i32 {
    let entry = heap_new_entry_call(heap);
    if entry.is_null() {
        return 0;
    }
    let mem = p8(rd::<u32>(block, 0));
    let size = rd::<u32>(block, 4);
    let end = p8(a32(mem).wrapping_add(size).wrapping_sub(0x20));
    wr::<u32>(mem, 0, 0);
    wr::<u32>(mem, 4, 0);
    wr::<u32>(mem, 8, 0);
    wr::<u32>(mem, 0xC, 0);
    for i in 0..8isize {
        wr::<u32>(end, i * 4, rd::<u32>(mem, i * 4));
    }
    let h = mem.byte_add(0x20);
    wr::<u32>(mem, 4, a32(h));
    wr::<u32>(h, 0, a32(mem));
    wr::<u32>(h, 4, a32(end));
    wr::<u32>(end, 0, a32(h));
    let free = a32(end).wrapping_sub(a32(h)).wrapping_sub(0x20);
    wr::<u32>(h, 0xC, a32(entry));
    wr::<u32>(h, 8, free);
    wr::<u32>(entry, 4, a32(h));
    wr::<u32>(entry, 0, rd::<u32>(h, 8));
    if !prev.is_null() {
        let psize = rd::<u32>(prev, 4);
        let pmem = rd::<u32>(prev, 0);
        let pend = p8(psize.wrapping_add(pmem).wrapping_sub(0x20));
        wr::<u32>(pend, 4, a32(mem));
        wr::<u32>(mem, 0, a32(pend));
    }
    1
}

/// `_rxHeapGetEntry(heap)` (0x8099F0, catalogue `FUN_008099f0`): the next free slot of the entry
/// array. When it is full, the allocation grows by 0x20 entries (`rwrealloc(array, n * 8,
/// 0x1030409)`, the count is stored before the call); on failure `{1, E_RW_NOMEM(n * 8)}` is reported,
/// the count is reduced again and NULL returned; when the array moved, every used entry's header gets
/// its entry pointer back. Returns `&array[used++]`, or NULL when the array is NULL.
#[hook(addr = 0x8099F0, conv = cdecl, name = "FUN_008099f0", expect = "83 EC 08 53 56")]
pub fn rx_heap_new_entry(heap: GPtr<u8>) -> GPtr<u8> {
    let used = rd::<u32>(heap, 0x14);
    let alloced = rd::<u32>(heap, 0x10);
    let mut arr = rd::<u32>(heap, 0xC);
    if alloced <= used {
        let n = alloced.wrapping_add(0x20);
        wr::<u32>(heap, 0x10, n);
        arr = a32(rw_realloc(p8(arr), n << 3, 0x1030409));
        if arr == 0 {
            error2(0x8000_0013, rd::<u32>(heap, 0x10) << 3);
            wr::<u32>(heap, 0x10, rd::<u32>(heap, 0x10).wrapping_sub(0x20));
            return GPtr::NULL;
        }
        if arr != rd::<u32>(heap, 0xC) && used != 0 {
            let mut e = arr;
            let mut k = used;
            while k != 0 {
                let hdr = p8(rd::<u32>(p8(e), 4));
                wr::<u32>(hdr, 0xC, e);
                e = e.wrapping_add(8);
                k -= 1;
            }
        }
        wr::<u32>(heap, 0xC, arr);
    }
    if arr == 0 {
        return GPtr::NULL;
    }
    let n = rd::<u32>(heap, 0x14);
    let r = arr.wrapping_add(n.wrapping_mul(8));
    wr::<u32>(heap, 0x14, n.wrapping_add(1));
    p8(r)
}

/// `RxRenderStateVectorSetDefaultRenderStateVector(rsvp)` (0x80EA40): NULL reports
/// `{1, E_RW_NULLP}` (0x80000016). While the engine is started (`G+0x150 == 3`) the module's default
/// vector (`P+4`, 11 words) is copied into `rsvp`. Otherwise only the module's own vector may be set
/// (anything else reports `{1, 0x80000018}`): flags 7, shade 2, src/dest blend 5/6, raster NULL,
/// address U/V 1/1, filter 2, border and fog colour 0xFFFFFFFF, fog type 0. Returns `rsvp`, or NULL.
#[hook(addr = 0x80EA40, conv = cdecl, name = "RxRenderStateVectorSetDefaultRenderStateVector", expect = "8B 44 24 04 83 EC 08")]
pub fn rx_render_state_vector_set_default(rsvp: GPtr<u8>) -> GPtr<u8> {
    if rsvp.is_null() {
        error1(0x8000_0016);
        return GPtr::NULL;
    }
    let gl = rw_globals();
    let def = pmod().byte_add(4);
    if rd::<u32>(gl, 0x150) == 3 {
        for i in 0..11isize {
            wr::<u32>(rsvp, i * 4, rd::<u32>(def, i * 4));
        }
        return rsvp;
    }
    if rsvp != def {
        error1(0x8000_0018);
        return GPtr::NULL;
    }
    wr::<u32>(rsvp, 0x10, 0);
    wr::<u32>(rsvp, 4, 2);
    wr::<u32>(rsvp, 0x1C, 2);
    wr::<u32>(rsvp, 0x24, 0);
    wr::<u32>(rsvp, 0, 7);
    wr::<u32>(rsvp, 8, 5);
    wr::<u32>(rsvp, 0xC, 6);
    wr::<u32>(rsvp, 0x14, 1);
    wr::<u32>(rsvp, 0x18, 1);
    wr::<u32>(rsvp, 0x20, 0xFFFF_FFFF);
    wr::<u32>(rsvp, 0x28, 0xFFFF_FFFF);
    rsvp
}

/// `_rwSListCreate(entrySize, hint)` (0x809160): list struct `rwmalloc(0x10, 0x30000)` (failure:
/// `{1, E_RW_NOMEM(0x10)}`), +0xC entry size, +4 count 0, +8 capacity 0x14, +0 entries
/// `rwmalloc(entrySize * 20, hint | 0x1000000)` (failure: `{1, E_RW_NOMEM(capacity * size)}`, struct
/// freed). Returns the list or NULL.
#[hook(addr = 0x809160, conv = cdecl, name = "_rwSListCreate", expect = "A1 24 7B C9 00")]
pub fn rw_slist_create(entry_size: u32, hint: u32) -> GPtr<u8> {
    let s = rw_malloc(0x10, 0x30000);
    if s.is_null() {
        error2(0x8000_0013, 0x10);
        return GPtr::NULL;
    }
    wr::<u32>(s, 0xC, entry_size);
    wr::<u32>(s, 4, 0);
    wr::<u32>(s, 8, 0x14);
    let p = rw_malloc(entry_size.wrapping_mul(20), hint | 0x0100_0000);
    wr::<u32>(s, 0, a32(p));
    if p.is_null() {
        error2(0x8000_0013, rd::<u32>(s, 8).wrapping_mul(rd::<u32>(s, 0xC)));
        rw_free(s);
        return GPtr::NULL;
    }
    s
}

/// `_rwSListDestroy(list)` (0x809440): frees the entries when both the pointer and the capacity are
/// non-zero (and clears them), then the struct. Returns TRUE.
#[hook(addr = 0x809440, conv = cdecl, name = "_rwSListDestroy", expect = "56 8B 74 24 08")]
pub fn rw_slist_destroy(s: GPtr<u8>) -> i32 {
    let e = rd::<u32>(s, 0);
    if e != 0 && rd::<u32>(s, 8) != 0 {
        rw_free(p8(e));
        wr::<u32>(s, 0, 0);
        wr::<u32>(s, 8, 0);
    }
    rw_free(s);
    1
}

/// `RxPipelineCreate()` (0x8057B0): a pipeline from the module freelist (`G->memoryAlloc(P+0,
/// 0x30409)`), its 13 words cleared; on failure `{1, E_RW_NOMEM(0x34)}` and NULL.
#[hook(addr = 0x8057B0, conv = cdecl, name = "RxPipelineCreate", expect = "A1 24 7B C9 00")]
pub fn rx_pipeline_create() -> GPtr<u8> {
    let p = rw_memory_alloc(p8(rd::<u32>(pmod(), 0)), 0x30409);
    if p.is_null() {
        error2(0x8000_0013, 0x34);
        return GPtr::NULL;
    }
    for k in 0..13isize {
        wr::<u32>(p, k * 4, 0);
    }
    wr::<u32>(p, 0, 0);
    p
}

/// `_rxPipelineDestroy(pipeline)` (0x805820): NULL is ignored; each of the `numNodes` (+4) nodes
/// (0x28 bytes from +8) goes through `_rxPipelineNodeDestroyRecurse(node, pipeline)`, the node pointer
/// is cleared, the cluster/scratch block (+0x20) is freed when present (+0x20, +0x24 cleared) and the
/// pipeline returns to the module freelist.
#[hook(addr = 0x805820, conv = cdecl, name = "_rxPipelineDestroy", expect = "56 8B 74 24 08")]
pub fn rx_pipeline_destroy(p: GPtr<u8>) {
    if p.is_null() {
        return;
    }
    let n = rd::<u32>(p, 4);
    let mut node = rd::<u32>(p, 8);
    let mut k = n;
    while k != 0 {
        pipeline_node_destroy_recurse(p8(node), p);
        node = node.wrapping_add(0x28);
        k -= 1;
    }
    let b = rd::<u32>(p, 0x20);
    wr::<u32>(p, 8, 0);
    if b != 0 {
        rw_free(p8(b));
        wr::<u32>(p, 0x20, 0);
        wr::<u32>(p, 0x24, 0);
    }
    rw_memory_free(p8(rd::<u32>(pmod(), 0)), p);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RX_PIPELINE_OPEN,
    HOOK_RX_PIPELINE_CLOSE,
    HOOK_RX_HEAP_DESTROY,
    HOOK_RX_HEAP_CREATE,
    HOOK_RX_HEAP_ADD_BLOCK,
    HOOK_RX_HEAP_NEW_ENTRY,
    HOOK_RX_RENDER_STATE_VECTOR_SET_DEFAULT,
    HOOK_RW_SLIST_CREATE,
    HOOK_RW_SLIST_DESTROY,
    HOOK_RX_PIPELINE_CREATE,
    HOOK_RX_PIPELINE_DESTROY,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = RX_GLOBALS;
