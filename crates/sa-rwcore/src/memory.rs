//! RW freelists, `_rwMemoryOpen`, RESOURCESMODULE (resource entries and the per-frame purge) and the
//! resource arena (PORT-RW-1). Ported from the 1.0 US bytes; `docs/re/specs/RW/RwMemory.md` is the
//! reading aid.
//!
//! `RwFreeList` (0x24): +0 entrySize, +4 entriesPerBlock, +8 heapSize (bitmap bytes), +0xC alignment,
//! +0x10 block list head, +0x18 flags, +0x1C link in the global freelist list (head 0xC9A604). A block
//! is `next +0, prev +4, bitmap +8`, entries at `(block + alignment + heapSize + 7) & ~(alignment - 1)`.
//!
//! Not reproduced: `_rwFreeListCreateInternal` keeps the block size in its `alignment` argument slot
//! and `_rwFreeListAllocReal` its remaining-entries counter in its `fl` argument slot.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{
    error2, g, llink, module, p8, rd, rw_free, rw_globals, rw_malloc, rw_memory_alloc, rw_memory_free, wr,
};
use crate::layout::modules::RESOURCES;

/// Global freelist list head (next 0xC9A604, prev 0xC9A608).
const FREELIST_LIST: usize = 0xC9A604;
/// Static `RwFreeList` of the freelist-of-freelists.
const FREELIST_OF_FREELISTS_SPACE: u32 = 0xC9A60C;
/// Memory-open flag.
const MEMORY_OPEN: usize = 0xC9A630;
/// The freelist of `RwFreeList` structs.
const FREELIST_OF_FREELISTS: usize = 0xC9A634;
/// Freelist preallocation flag (`.data`, 1).
const FREELIST_PREALLOC: usize = 0x8E266C;
/// RESOURCESMODULE instance counter.
const RESOURCES_INSTANCES: usize = 0xC9BC5C;

orig! {
    /// `_rwFreeListCreateInternal(...)` (0x8019B0)
    pub fn freelist_create_internal_call(size: u32, per_block: u32, align: u32, n: u32, space: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x8019B0 => rw_freelist_create_internal;
    /// `RwFreeListPurge(fl)` (0x801E00)
    pub fn freelist_purge_call(fl: GPtr<u8>) -> i32 = cdecl 0x801E00 => rw_freelist_purge;
    /// `RwFreeListCreateAndPreallocateSpace(...)` (0x801B70)
    pub fn freelist_create_prealloc_call(size: u32, per_block: u32, align: u32, n: u32, space: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x801B70 => rw_freelist_create_internal;
    /// `_rwResourcesArenaInit(mem, size)` (0x80FFE0)
    pub fn arena_init_call(mem: GPtr<u8>, size: u32) -> u32 = cdecl 0x80FFE0 => rw_resources_arena_init;
    /// `_rwResourcesArenaFreeBlock(data)` (0x810040)
    pub fn arena_free_block_call(data: GPtr<u8>) = cdecl 0x810040 => rw_resources_arena_free_block;
    /// `_rwResourcesArenaFindFree(arena, size)` (0x8100C0)
    pub fn arena_find_free_call(arena: GPtr<u8>, size: u32) -> GPtr<u8> = cdecl 0x8100C0 => rw_resources_arena_find_free;
    /// `_rwResourcesArenaSplitBlock(b, size)` (0x810140)
    pub fn arena_split_block_call(b: GPtr<u8>, size: u32) -> GPtr<u8> = cdecl 0x810140 => rw_resources_arena_split_block;
    /// `RwResEntry::destroyNotify(e)` (stored pointer)
    pub fn call_destroy_notify(f: u32, e: GPtr<u8>) = cdecl dyn => unported;
}

/// `_rwFreeListSetFlags(flag)` (0x801970): `[0x8E266C] = flag`.
#[hook(addr = 0x801970, conv = cdecl, name = "_rwFreeListSetFlags", expect = "8B 44 24 04 A3 6C 26 8E 00")]
pub fn rw_freelist_set_flags(flag: u32) {
    g::<u32>(FREELIST_PREALLOC).write(flag);
}

/// `_rwFreeListCreate(entrySize, entriesPerBlock, alignment, hint)` (0x801980).
#[hook(addr = 0x801980, conv = cdecl, name = "_rwFreeListCreate", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn rw_freelist_create(size: u32, per_block: u32, align: u32, hint: u32) -> GPtr<u8> {
    freelist_create_internal_call(size, per_block, align, 1, p8(0), hint)
}

/// Bytes of one block: `entriesPerBlock * entrySize + heapSize + alignment + 7`.
fn block_size(fl: GPtr<u8>) -> u32 {
    rd::<u32>(fl, 4)
        .wrapping_mul(rd::<u32>(fl, 0))
        .wrapping_add(rd::<u32>(fl, 8))
        .wrapping_add(rd::<u32>(fl, 0xC))
        .wrapping_add(7)
}

/// The first entry of block `b`.
fn entries(fl: GPtr<u8>, b: GPtr<u8>) -> u32 {
    let align = rd::<u32>(fl, 0xC);
    (b.addr() as u32)
        .wrapping_add(align)
        .wrapping_add(rd::<u32>(fl, 8))
        .wrapping_add(7)
        & !align.wrapping_sub(1)
}

fn zero_bitmap(b: GPtr<u8>, n: u32) {
    for i in 0..n as isize {
        wr::<u8>(b, 8 + i, 0);
    }
}

/// Frees `fl` itself: back to the freelist-of-freelists unless it is that list or none exists.
fn free_freelist_struct(fl: GPtr<u8>) {
    let ff = g::<u32>(FREELIST_OF_FREELISTS).read();
    if ff != fl.addr() as u32 && ff != 0 {
        rw_memory_free(p8(ff), fl);
    } else {
        rw_free(fl);
    }
}

/// Unlinks and frees every block of `fl` (head.next first).
fn free_blocks(fl: GPtr<u8>) {
    let head = fl.byte_add(0x10);
    loop {
        let b = llink::next(head);
        if b.addr() == head.addr() {
            break;
        }
        llink::remove(b);
        rw_free(b);
    }
}

/// `_rwFreeListCreateInternal(entrySize, entriesPerBlock, alignment, numBlocks, inPlaceSpace, hint)`
/// (0x8019B0).
#[hook(addr = 0x8019B0, conv = cdecl, name = "_rwFreeListCreateInternal", expect = "A1 6C 26 8E 00")]
pub fn rw_freelist_create_internal(
    size: u32,
    per_block: u32,
    align: u32,
    n: u32,
    space: GPtr<u8>,
    hint: u32,
) -> GPtr<u8> {
    let mut n = n;
    if g::<u32>(FREELIST_PREALLOC).read() == 0 {
        n = 0;
    }
    let align = if align == 0 { 4 } else { align };
    let fl = if space.is_null() {
        let ff = g::<u32>(FREELIST_OF_FREELISTS).read();
        let fl = if ff != 0 {
            rw_memory_alloc(p8(ff), hint & 0xFF_0000)
        } else {
            rw_malloc(0x24, hint & 0xFF_0000)
        };
        if fl.is_null() {
            return fl;
        }
        wr::<u32>(fl, 0x18, 2);
        fl
    } else {
        wr::<u32>(space, 0x18, 3);
        space
    };
    let heap = per_block.wrapping_add(7) >> 3;
    let head = fl.byte_add(0x10);
    wr::<u32>(
        fl,
        0,
        size.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1),
    );
    wr::<u32>(fl, 4, per_block);
    wr::<u32>(fl, 0xC, align);
    wr::<u32>(fl, 8, heap);
    llink::init(head);
    if n != 0 {
        let bsize = block_size(fl);
        loop {
            let b = rw_malloc(bsize, hint);
            if b.is_null() {
                free_blocks(fl);
                if rd::<u8>(fl, 0x18) & 1 == 0 {
                    free_freelist_struct(fl);
                }
                return p8(0);
            }
            wr::<u32>(b, 4, 0);
            wr::<u32>(b, 0, 0);
            llink::add_head(head, b);
            zero_bitmap(b, heap);
            n -= 1;
            if n == 0 {
                break;
            }
        }
    }
    llink::add_head(p8(FREELIST_LIST as u32), fl.byte_add(0x1C));
    fl
}

/// `RwFreeListDestroy(fl)` (0x801B80): unlink, free every block, free the struct unless static.
#[hook(addr = 0x801B80, conv = cdecl, name = "RwFreeListDestroy", expect = "56 8B 74 24 08 57")]
pub fn rw_freelist_destroy(fl: GPtr<u8>) -> u32 {
    llink::remove(fl.byte_add(0x1C));
    free_blocks(fl);
    if rd::<u8>(fl, 0x18) & 1 == 0 {
        free_freelist_struct(fl);
    }
    1
}

/// `_rwFreeListAllocReal(fl, hint)` (0x801C30): the first clear bitmap bit of the first block that has
/// one (blocks from the list head; `rem` = entries left in the block), else a new block at the head.
#[hook(addr = 0x801C30, conv = cdecl, name = "_rwFreeListAllocReal", expect = "51 53 55 8B 6C 24 10")]
pub fn rw_freelist_alloc_real(fl: GPtr<u8>, hint: u32) -> GPtr<u8> {
    let head = fl.byte_add(0x10);
    let heap = rd::<u32>(fl, 8);
    let mut b = llink::next(head);
    while b.addr() != head.addr() {
        let mut rem = rd::<u32>(fl, 4);
        let mut i = 0u32;
        while i < heap {
            let byte = rd::<u8>(b, 8 + i as isize);
            if byte == 0xFF {
                rem = rem.wrapping_sub(8);
            } else {
                let mut j = 0u32;
                while rem != 0 {
                    let mask = (0x80u32 >> j) as u8;
                    if byte & mask == 0 {
                        wr::<u8>(b, 8 + i as isize, byte | mask);
                        let p = entries(fl, b).wrapping_add((j + i * 8).wrapping_mul(rd::<u32>(fl, 0)));
                        if p != 0 {
                            return p8(p);
                        }
                        break;
                    }
                    j += 1;
                    rem = rem.wrapping_sub(1);
                    if j >= 8 {
                        break;
                    }
                }
            }
            i += 1;
        }
        b = llink::next(b);
    }
    let nb = rw_malloc(block_size(fl), hint);
    if nb.is_null() {
        return nb;
    }
    zero_bitmap(nb, heap);
    llink::add_head(head, nb);
    wr::<u8>(nb, 8, 0x80);
    p8(entries(fl, nb))
}

/// `_rwFreeListFreeReal(fl, p)` (0x801D50): clears the entry's bit in the first block whose unaligned
/// range `[b + heapSize + 8, + entriesPerBlock * entrySize]` (inclusive) holds it; with flag 2 an empty
/// block is unlinked and freed. Returns fl, or NULL when no block holds p.
#[hook(addr = 0x801D50, conv = cdecl, name = "_rwFreeListFreeReal", expect = "53 55 56 57 8B 7C 24 14")]
pub fn rw_freelist_free_real(fl: GPtr<u8>, p: GPtr<u8>) -> GPtr<u8> {
    let head = fl.byte_add(0x10);
    let heap = rd::<u32>(fl, 8);
    let pa = p.addr() as u32;
    let mut b = llink::next(head);
    while b.addr() != head.addr() {
        let start = (b.addr() as u32).wrapping_add(heap).wrapping_add(8);
        let end = rd::<u32>(fl, 4)
            .wrapping_mul(rd::<u32>(fl, 0))
            .wrapping_add(start);
        if pa >= start && pa <= end {
            let idx = (pa - start) / rd::<u32>(fl, 0);
            let o = 8 + (idx >> 3) as isize;
            let mask = !((0x80u32 >> (idx & 7)) as u8);
            wr::<u8>(b, o, rd::<u8>(b, o) & mask);
            if rd::<u8>(fl, 0x18) & 2 != 0 {
                let mut sum = 0u32;
                for k in 0..heap as isize {
                    sum = sum.wrapping_add(u32::from(rd::<u8>(b, 8 + k)));
                }
                if sum == 0 {
                    llink::remove(b);
                    rw_free(b);
                }
            }
            return fl;
        }
        b = llink::next(b);
    }
    p8(0)
}

/// `RwFreeListPurge(fl)` (0x801E00): frees the empty blocks, re-inserts the others at the head (in
/// reverse order); returns `entrySize * freed blocks`.
#[hook(addr = 0x801E00, conv = cdecl, name = "RwFreeListPurge", expect = "51 8B 4C 24 08")]
pub fn rw_freelist_purge(fl: GPtr<u8>) -> i32 {
    let head = fl.byte_add(0x10);
    let heap = rd::<u32>(fl, 8);
    let mut count = 0u32;
    let mut b = llink::next(head);
    while b.addr() != head.addr() {
        llink::remove(b);
        let next = llink::next(b);
        let mut sum = 0u32;
        for k in 0..heap as isize {
            sum = sum.wrapping_add(u32::from(rd::<u8>(b, 8 + k)));
        }
        if sum == 0 {
            rw_free(b);
            count = count.wrapping_add(1);
        } else {
            llink::add_head(head, b);
        }
        b = next;
    }
    rd::<u32>(fl, 0).wrapping_mul(count) as i32
}

/// `RwFreeListPurgeAllFreeLists()` (0x801F90): the sum of the positive purge results.
#[hook(addr = 0x801F90, conv = cdecl, name = "RwFreeListPurgeAllFreeLists", expect = "56 8B 35 04 A6 C9 00")]
pub fn rw_freelist_purge_all() -> i32 {
    let mut total = 0i32;
    let mut l = p8(g::<u32>(FREELIST_LIST).read());
    while l.addr() != FREELIST_LIST {
        let r = freelist_purge_call(l.byte_add(-0x1C));
        if r > 0 {
            total = total.wrapping_add(r);
        }
        l = llink::next(l);
    }
    total
}

/// `_rwMemoryOpen(funcs)` (0x801FD0).
#[hook(addr = 0x801FD0, conv = cdecl, name = "_rwMemoryOpen", expect = "68 00 00 04 00")]
pub fn rw_memory_open(funcs: GPtr<u8>) -> u32 {
    llink::init(p8(FREELIST_LIST as u32));
    g::<u32>(MEMORY_OPEN).write(1);
    let fl = freelist_create_internal_call(0x24, 0x10, 4, 0, p8(FREELIST_OF_FREELISTS_SPACE), 0x40000);
    g::<u32>(FREELIST_OF_FREELISTS).write(fl.addr() as u32);
    if fl.is_null() {
        g::<u32>(MEMORY_OPEN).write(0);
        return 0;
    }
    llink::remove(fl.byte_add(0x1C));
    let gl = rw_globals();
    if !funcs.is_null() {
        for k in 0..4 {
            wr::<u32>(gl, 0x134 + k * 4, rd::<u32>(funcs, k * 4));
        }
    } else {
        wr::<u32>(gl, 0x134, 0x8020A0);
        wr::<u32>(rw_globals(), 0x138, 0x82413F);
        wr::<u32>(rw_globals(), 0x13C, 0x8020B0);
        wr::<u32>(rw_globals(), 0x140, 0x8020D0);
    }
    1
}

fn rmod() -> GPtr<u8> {
    module(RESOURCES)
}

/// `cb_RESOURCESMODULE_RwEngine_Open(instance, offset, size)` (0x807C90).
#[hook(addr = 0x807C90, conv = cdecl, name = "cb_RESOURCESMODULE_RwEngine_Open", expect = "8B 4C 24 08 A1 24 7B C9 00")]
pub fn resources_module_open(instance: u32, offset: u32, _size: u32) -> u32 {
    g::<u32>(RESOURCES).write(offset);
    let gl = rw_globals();
    let size0 = rd::<u32>(gl, 0x154);
    let r = gl.byte_add(offset as i32 as isize);
    if size0 != 0 {
        let mem = rw_malloc(size0, 0x4040B);
        wr::<u32>(r, 0xC, mem.addr() as u32);
        if mem.is_null() {
            error2(0x8000_0013, size0);
            return 0;
        }
        if arena_init_call(mem, size0) == 0 {
            rw_free(p8(rd::<u32>(r, 0xC)));
            error2(0xC, 0);
            return 0;
        }
    } else {
        wr::<u32>(r, 0xC, 0);
    }
    let (a, b) = (r.byte_add(0x10), r.byte_add(0x18));
    wr::<u32>(r, 0x14, a.addr() as u32);
    wr::<u32>(r, 0x1C, b.addr() as u32);
    wr::<u32>(a, 0, a.addr() as u32);
    wr::<u32>(b, 0, b.addr() as u32);
    wr::<u32>(r, 0x24, a.addr() as u32);
    wr::<u32>(r, 0x20, b.addr() as u32);
    wr::<u32>(r, 0, size0);
    wr::<u32>(r, 4, 0);
    wr::<u32>(r, 8, 0);
    let c = g::<u32>(RESOURCES_INSTANCES);
    c.write(c.read().wrapping_add(1));
    instance
}

/// Destroys a resource entry as `RwResourcesFreeResEntry` and the purge loop of
/// `RwResourcesAllocateResEntry` do.
fn destroy_entry(e: GPtr<u8>) {
    let f = rd::<u32>(e, 0x14);
    if f != 0 {
        call_destroy_notify(f, e);
    }
    let r = rd::<u32>(e, 0x10);
    if r != 0 {
        wr::<u32>(p8(r), 0, 0);
    }
    if rd::<u32>(e, 0) != 0 {
        llink::remove(e);
        let m = rmod();
        wr::<u32>(m, 4, rd::<u32>(m, 4).wrapping_sub(rd::<u32>(e, 8)));
        arena_free_block_call(e);
    } else {
        rw_free(e);
    }
}

/// `RwResourcesFreeResEntry(e)` (0x807DE0); returns 1.
#[hook(addr = 0x807DE0, conv = cdecl, name = "RwResourcesFreeResEntry", expect = "56 8B 74 24 08 8B 46 14")]
pub fn rw_resources_free_res_entry(e: GPtr<u8>) -> u32 {
    destroy_entry(e);
    1
}

/// `_rwResourcesPurge()` (0x807E50): appends the older list to the current one and swaps them;
/// `reusageSize = 0`.
#[hook(addr = 0x807E50, conv = cdecl, name = "_rwResourcesPurge", expect = "8B 15 58 BC C9 00")]
pub fn rw_resources_purge() {
    let m = rmod();
    let cur = p8(rd::<u32>(m, 0x24));
    let old = p8(rd::<u32>(m, 0x20));
    let first = llink::next(old);
    if first.addr() != old.addr() {
        if llink::next(cur).addr() == cur.addr() {
            wr::<u32>(cur, 0, first.addr() as u32);
            wr::<u32>(first, 4, cur.addr() as u32);
            let last = llink::prev(old);
            wr::<u32>(cur, 4, last.addr() as u32);
            wr::<u32>(last, 0, cur.addr() as u32);
        } else {
            let tail = llink::prev(cur);
            wr::<u32>(tail, 0, first.addr() as u32);
            wr::<u32>(first, 4, tail.addr() as u32);
            let last = llink::prev(old);
            wr::<u32>(last, 0, cur.addr() as u32);
            wr::<u32>(cur, 4, last.addr() as u32);
        }
        llink::init(old);
    }
    wr::<u32>(rmod(), 0x24, old.addr() as u32);
    wr::<u32>(rmod(), 0x20, cur.addr() as u32);
    wr::<u32>(rmod(), 8, 0);
}

/// `RwResourcesAllocateResEntry(owner, ownerRef, size, destroyNotify)` (0x807ED0).
#[hook(addr = 0x807ED0, conv = cdecl, name = "RwResourcesAllocateResEntry", expect = "83 EC 08 53 56 57")]
pub fn rw_resources_allocate_res_entry(
    owner: u32,
    owner_ref: GPtr<u32>,
    size: u32,
    destroy_notify: u32,
) -> GPtr<u8> {
    let e = loop {
        let e = arena_find_free_call(p8(rd::<u32>(rmod(), 0xC)), size.wrapping_add(0x18));
        if !e.is_null() {
            break e;
        }
        let older = p8(rd::<u32>(rmod(), 0x20));
        let last = llink::prev(older);
        if last.addr() != older.addr() {
            destroy_entry(last);
            continue;
        }
        let cur = p8(rd::<u32>(rmod(), 0x24));
        let last = llink::prev(cur);
        if last.addr() == cur.addr() {
            if !owner_ref.is_null() {
                owner_ref.write(0);
            }
            error2(0xC, size);
            return p8(0);
        }
        let m = rmod();
        wr::<u32>(m, 8, rd::<u32>(m, 8).wrapping_add(rd::<u32>(last, 8)));
        destroy_entry(last);
    };
    let cur = p8(rd::<u32>(rmod(), 0x24));
    wr::<u32>(e, 0, rd::<u32>(cur, 0));
    wr::<u32>(e, 4, cur.addr() as u32);
    wr::<u32>(p8(rd::<u32>(cur, 0)), 4, e.addr() as u32);
    wr::<u32>(cur, 0, e.addr() as u32);
    wr::<u32>(e, 0xC, owner);
    wr::<u32>(e, 8, size);
    wr::<u32>(e, 0x10, owner_ref.addr() as u32);
    wr::<u32>(e, 0x14, destroy_notify);
    let m = rmod();
    wr::<u32>(m, 4, rd::<u32>(m, 4).wrapping_add(size));
    if !owner_ref.is_null() {
        owner_ref.write(e.addr() as u32);
    }
    e
}

/// `_rwResourcesOpen()` (0x8082E0; by its bytes RW's `_rwPluginRegistryOpen`): the plugin-registry
/// entry freelist `[0xC9A6E4]`, `[0xC9A6EC] = 0`.
#[hook(addr = 0x8082E0, conv = cdecl, name = "FUN_008082e0", expect = "A1 B0 26 8E 00")]
pub fn rw_resources_open() -> u32 {
    let n = g::<u32>(0x8E26B0).read();
    let per = g::<u32>(0x8E26AC).read();
    let fl = freelist_create_prealloc_call(0x3C, per, 4, n, p8(0xC9A6C0), 0x40000);
    g::<u32>(0xC9A6E4).write(fl.addr() as u32);
    if fl.is_null() {
        return 0;
    }
    g::<u32>(0xC9A6EC).write(0);
    1
}

/// `_rwResourcesArenaInit(mem, size)` (0x80FFE0).
#[hook(addr = 0x80FFE0, conv = cdecl, name = "_rwResourcesArenaInit", expect = "8B 54 24 04 8B 4C 24 08")]
pub fn rw_resources_arena_init(mem: GPtr<u8>, size: u32) -> u32 {
    let m = mem.addr() as u32;
    let blk = m.wrapping_add(0x27) & !0x1F;
    let avail = (m.wrapping_add(size) & !0x1F)
        .wrapping_sub(blk)
        .wrapping_sub(0x20);
    if (avail as i32) < 0x20 {
        return 0;
    }
    let b = p8(blk);
    wr::<u32>(b, 0, m);
    wr::<u32>(b, 4, 0);
    wr::<u32>(b, 8, 0);
    wr::<u32>(b, 0x10, 0);
    wr::<u32>(b, 0xC, avail);
    wr::<u32>(mem, 0, blk);
    wr::<u32>(mem, 4, blk);
    1
}

fn arena_block_free(b: GPtr<u8>) -> bool {
    rd::<u32>(b, 0x10) & 1 == 0
}

/// `_rwResourcesArenaFreeBlock(data)` (0x810040): marks the block free, lowers the hint, merges with a
/// free previous and then a free next block.
#[hook(addr = 0x810040, conv = cdecl, name = "_rwResourcesArenaFreeBlock", expect = "8B 44 24 04 56 83 C0 E0")]
pub fn rw_resources_arena_free_block(data: GPtr<u8>) {
    let b = data.byte_add(-0x20);
    let arena = p8(rd::<u32>(b, 0));
    let prev = p8(rd::<u32>(b, 8));
    let next = p8(rd::<u32>(b, 4));
    wr::<u32>(b, 0x10, 0);
    let hint = rd::<u32>(arena, 4);
    if hint == 0 || (b.addr() as u32) < hint {
        wr::<u32>(arena, 4, b.addr() as u32);
    }
    let mut cur = b;
    if !prev.is_null() && arena_block_free(prev) {
        wr::<u32>(prev, 4, next.addr() as u32);
        if !next.is_null() {
            wr::<u32>(next, 8, prev.addr() as u32);
        }
        let s = rd::<u32>(b, 0xC).wrapping_add(0x20);
        wr::<u32>(prev, 0xC, rd::<u32>(prev, 0xC).wrapping_add(s));
        cur = prev;
    }
    if !next.is_null() && arena_block_free(next) {
        let nn = rd::<u32>(next, 4);
        wr::<u32>(cur, 4, nn);
        if nn != 0 {
            wr::<u32>(p8(nn), 8, cur.addr() as u32);
        }
        let s = rd::<u32>(next, 0xC).wrapping_add(0x20);
        wr::<u32>(cur, 0xC, rd::<u32>(cur, 0xC).wrapping_add(s));
    }
}

/// `_rwResourcesArenaFindFree(arena, size)` (0x8100C0): first free block from the hint with room for
/// `(size + 0x1F) & ~0x1F`; split when more than 0x40 bytes would remain; hint advanced past used
/// blocks when it was the hint. Returns the data pointer (block + 0x20) or NULL.
#[hook(addr = 0x8100C0, conv = cdecl, name = "FUN_008100c0", expect = "8B 44 24 08 56 57")]
pub fn rw_resources_arena_find_free(arena: GPtr<u8>, size: u32) -> GPtr<u8> {
    let need = size.wrapping_add(0x1F) & !0x1F;
    let mut found = p8(0);
    let mut b = p8(rd::<u32>(arena, 4));
    while !b.is_null() {
        if arena_block_free(b) && rd::<u32>(b, 0xC) >= need {
            found = b;
            break;
        }
        b = p8(rd::<u32>(b, 4));
    }
    if found.is_null() {
        return found;
    }
    if rd::<u32>(found, 0xC) > need.wrapping_add(0x40) {
        arena_split_block_call(found, need);
    }
    if found.addr() as u32 == rd::<u32>(arena, 4) {
        loop {
            let h = p8(rd::<u32>(arena, 4));
            let n = rd::<u32>(h, 4);
            wr::<u32>(arena, 4, n);
            if n == 0 || rd::<u8>(p8(n), 0x10) & 1 == 0 {
                break;
            }
        }
    }
    wr::<u32>(found, 0x10, 1);
    found.byte_add(0x20)
}

/// `_rwResourcesArenaSplitBlock(b, size)` (0x810140): the new block after `size` bytes of data absorbs
/// a free next block (`size = next.size - size + b.size`, as the bytes compute it).
#[hook(addr = 0x810140, conv = cdecl, name = "FUN_00810140", expect = "8B 4C 24 04 56 57")]
pub fn rw_resources_arena_split_block(b: GPtr<u8>, size: u32) -> GPtr<u8> {
    let next = p8(rd::<u32>(b, 4));
    let n = b.byte_add(size as i32 as isize + 0x20);
    if !next.is_null() && arena_block_free(next) {
        wr::<u32>(n, 4, rd::<u32>(next, 4));
        let s = rd::<u32>(p8(rd::<u32>(b, 4)), 0xC)
            .wrapping_sub(size)
            .wrapping_add(rd::<u32>(b, 0xC));
        wr::<u32>(n, 0xC, s);
    } else {
        wr::<u32>(n, 4, next.addr() as u32);
        wr::<u32>(n, 0xC, rd::<u32>(b, 0xC).wrapping_sub(size).wrapping_sub(0x20));
    }
    wr::<u32>(b, 4, n.addr() as u32);
    let nn = rd::<u32>(n, 4);
    wr::<u32>(n, 0x10, 0);
    wr::<u32>(n, 8, b.addr() as u32);
    if nn != 0 {
        wr::<u32>(p8(nn), 8, n.addr() as u32);
    }
    wr::<u32>(b, 0xC, size);
    wr::<u32>(n, 0, rd::<u32>(b, 0));
    n
}

/// Every `#[hook]` of this module.
/// `RwFreeListCreateAndPreallocateSpace(entrySize, entriesPerBlock, alignment, numBlocks,
/// inPlaceSpace, hint)` (0x801B70): a `jmp _rwFreeListCreateInternal` (STD-3, written, unverified).
// Not hooked: a 5-byte `jmp` into .text (sa-hook only takes jmp stubs into .HOODLUM).
#[sa_core::native(addr = 0x801B70, conv = cdecl, name = "RwFreeListCreateAndPreallocateSpace")]
pub fn rw_freelist_create_and_preallocate_space(
    size: u32,
    per_block: u32,
    align: u32,
    n: u32,
    space: GPtr<u8>,
    hint: u32,
) -> GPtr<u8> {
    freelist_create_internal_call(size, per_block, align, n, space, hint)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RW_FREELIST_SET_FLAGS,
    HOOK_RW_FREELIST_CREATE,
    HOOK_RW_FREELIST_CREATE_INTERNAL,
    HOOK_RW_FREELIST_DESTROY,
    HOOK_RW_FREELIST_ALLOC_REAL,
    HOOK_RW_FREELIST_FREE_REAL,
    HOOK_RW_FREELIST_PURGE,
    HOOK_RW_FREELIST_PURGE_ALL,
    HOOK_RW_MEMORY_OPEN,
    HOOK_RESOURCES_MODULE_OPEN,
    HOOK_RW_RESOURCES_FREE_RES_ENTRY,
    HOOK_RW_RESOURCES_PURGE,
    HOOK_RW_RESOURCES_ALLOCATE_RES_ENTRY,
    HOOK_RW_RESOURCES_OPEN,
    HOOK_RW_RESOURCES_ARENA_INIT,
    HOOK_RW_RESOURCES_ARENA_FREE_BLOCK,
    HOOK_RW_RESOURCES_ARENA_FIND_FREE,
    HOOK_RW_RESOURCES_ARENA_SPLIT_BLOCK,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_RW_FREELIST_CREATE_AND_PREALLOCATE_SPACE];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
