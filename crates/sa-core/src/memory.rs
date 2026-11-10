//! `CMemoryMgr` (0x72F3B0–0x72F4FF) and `CMemoryHeap::Init` (0x72F180).
//!
//! On PC the memory manager is a thin layer over the CRT heap: `Malloc`/`Free`/`Realloc`/`Calloc` call
//! `malloc` (0x824257), `free` (0x82413F), `realloc` (0x824269), `calloc` (0x824416); `MallocAlign`
//! over-allocates and stores the raw pointer in the word before the aligned block. `CMemoryHeap` (the
//! console heap) is only initialised for the 16 KiB scratch pad. Everything follows the 1.0 US bytes;
//! `CMemoryHeap::Init` is written at the byte-offset level because that is what the bytes define:
//!
//! ```text
//! CMemoryHeap: +0x00 first block, +0x04 last block, +0x08 free-list begin (HeapFreeBlockDesc, 0x18),
//!              +0x20 free-list end (0x18), +0x38 common sizes, +0x3C moving restricted (byte),
//!              +0x40 mem used, +0x44 mem id
//! HeapBlockDesc: +0x00 size, +0x04 in use (byte), +0x06 mem id (u16), +0x0C previous block;
//!                HeapFreeBlockDesc adds +0x10 next, +0x14 prev
//! CommonSize (0x3C): +0x10, +0x2C, +0x30 size, +0x34, +0x38 (see `common_size_init`)
//! ```
//!
//! Where gta-reversed's `MemoryHeap.cpp` differs, the original is kept: it writes neither
//! `m_FreeListBegin.m_Prev` (+0x1C) nor `m_FreeListEnd.m_Next` (+0x30), and it clears
//! `m_paCommonSizes` (+0x38) only when `bInitSizes` is set.

use crate::mem::GPtr;
use crate::{globals, hook, orig};

globals! {
    list GLOBALS;
}

/// `gScratchPadHeap` (the `CMemoryHeap` that `InitScratchPad` initialises).
pub const SCRATCH_PAD_HEAP: u32 = 0xC87BD0;
/// `PC_Scratch` / `s_MemoryHeapBuffer` (16 KiB).
pub const SCRATCH_PAD: u32 = 0xC8E0C8;

orig! {
    /// CRT `malloc` (0x824257)
    pub fn malloc(size: u32) -> GPtr<u8> = cdecl 0x824257 => unported;
    /// CRT `free` (0x82413F)
    pub fn free(p: GPtr<u8>) = cdecl 0x82413F => unported;
    /// CRT `realloc` (0x824269)
    pub fn realloc(p: GPtr<u8>, size: u32) -> GPtr<u8> = cdecl 0x824269 => unported;
    /// CRT `calloc` (0x824416)
    pub fn calloc(n: u32, size: u32) -> GPtr<u8> = cdecl 0x824416 => unported;
    /// `CMemoryHeap::Malloc` (0x72ED80, thiscall)
    pub fn heap_malloc(heap: GPtr<u8>, size: u32) -> GPtr<u8> = thiscall 0x72ED80 => unported;
    /// `CMemoryHeap::Init` (0x72F180, thiscall), as `InitScratchPad` calls it
    pub fn heap_init_call(heap: GPtr<u8>, mem: GPtr<u8>, size: u32, init_sizes: u8) = thiscall 0x72F180 => heap_init;
}

/// `CMemoryMgr::Init` (0x72F3B0): empty (`ret`). Not hookable: a lone `ret` cannot be relocated.
pub fn init() {}

/// 0x72F3C0: an empty function (`ret`) called once from WinMain (gta-reversed
/// `nullsub_0x72F3C0`). Not hookable (lone `ret`).
pub fn nullsub_72f3c0() {}

/// `CMemoryMgr::Malloc(size)` (0x72F420, cdecl).
#[hook(addr = 0x72F420, conv = cdecl, name = "CMemoryMgr::Malloc-", expect = "8B 44 24 04 50")]
pub fn mgr_malloc(size: u32) -> GPtr<u8> {
    malloc(size)
}

/// `CMemoryMgr::Free(p)` (0x72F430, cdecl): `jmp free`.
#[hook(addr = 0x72F430, conv = cdecl, name = "CMemoryMgr::Free", expect = "E9 0A 4D 0F 00 90 90 90")]
pub fn mgr_free(p: GPtr<u8>) {
    free(p)
}

/// `CMemoryMgr::Realloc(p, size)` (0x72F440, cdecl).
#[hook(addr = 0x72F440, conv = cdecl, name = "CMemoryMgr::Realloc-", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn mgr_realloc(p: GPtr<u8>, size: u32) -> GPtr<u8> {
    realloc(p, size)
}

/// `CMemoryMgr::Calloc(n, size)` (0x72F460, cdecl).
#[hook(addr = 0x72F460, conv = cdecl, name = "CMemoryMgr::Calloc-", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn mgr_calloc(n: u32, size: u32) -> GPtr<u8> {
    calloc(n, size)
}

/// `CMemoryMgr::InitScratchPad()` (0x72F480, cdecl):
/// `gScratchPadHeap.Init(PC_Scratch, 0x4000, false)`.
#[hook(addr = 0x72F480, conv = cdecl, name = "CMemoryMgr::InitScratchPad", expect = "6A 00 68 00 40 00 00")]
pub fn init_scratch_pad() {
    heap_init_call(
        GPtr::from_addr(SCRATCH_PAD_HEAP as usize),
        GPtr::from_addr(SCRATCH_PAD as usize),
        0x4000,
        0,
    )
}

/// The aligned pointer `MallocAlign` returns for a raw block: `(raw + align) & ~(align - 1)`
/// (32-bit wrapping; no null check in the original).
pub fn align_up(raw: u32, align: u32) -> u32 {
    raw.wrapping_add(align) & !align.wrapping_sub(1)
}

/// `CMemoryMgr::MallocAlign(size, align)` (0x72F4C0, cdecl): `raw = malloc(size + align)`, the
/// aligned pointer gets `raw` in the word before it. A failed `malloc` is not checked (the original
/// then writes near address 0 as well).
#[hook(addr = 0x72F4C0, conv = cdecl, name = "CMemoryMgr::MallocAlign-", expect = "8B 44 24 04 56")]
pub fn malloc_align(size: u32, align: u32) -> GPtr<u8> {
    let raw = malloc(size.wrapping_add(align));
    let p = GPtr::<u8>::from_addr(align_up(raw.addr() as u32, align) as usize);
    p.byte_add(-4).cast::<u32>().write(raw.addr() as u32);
    p
}

/// `CMemoryMgr::FreeAlign(p)` (0x72F4F0, cdecl): `free(((void**)p)[-1])` (tail jump).
#[hook(addr = 0x72F4F0, conv = cdecl, name = "CMemoryMgr::FreeAlign", expect = "8B 44 24 04 8B 48 FC")]
pub fn free_align(p: GPtr<u8>) {
    let raw = p.byte_add(-4).cast::<u32>().read();
    free(GPtr::from_addr(raw as usize))
}

fn w32(base: GPtr<u8>, off: isize, v: u32) {
    base.byte_add(off).cast::<u32>().write(v)
}

/// `CommonSize::Init(size)` as inlined (unrolled x8) in `CMemoryHeap::Init`: `+0x10 = this + 0x18`,
/// `+0x2C = this`, `+0x30 = size`, `+0x34 = 0`, `+0x38 = 0`.
pub fn common_size_init(cs: GPtr<u8>, size: u32) {
    w32(cs, 0x10, cs.addr() as u32 + 0x18);
    w32(cs, 0x2C, cs.addr() as u32);
    w32(cs, 0x34, 0);
    w32(cs, 0x38, 0);
    w32(cs, 0x30, size);
}

/// `CMemoryHeap::Init(void* mem, uint32 size, bool bInitSizes)` (0x72F180, thiscall, `ret 0Ch`).
#[hook(addr = 0x72F180, conv = thiscall, name = "CMemoryHeap::Init", expect = "8B 44 24 08 53 83 E8 10")]
pub fn heap_init(heap: GPtr<u8>, mem: GPtr<u8>, size: u32, init_sizes: u8) {
    let h = heap.addr() as u32;
    let first = mem.addr() as u32;
    let last = first.wrapping_add(size.wrapping_sub(0x10));
    // 32-bit wrapping `mem + size - 0x10` (offset form keeps 64-bit host builds on the same memory)
    let lastp = mem.byte_add(size.wrapping_sub(0x10) as i32 as isize);
    w32(heap, 0x00, first);
    w32(heap, 0x04, last);
    w32(heap, 0x40, 0);
    heap.byte_add(0x3C).write(1);
    w32(heap, 0x44, 0xFFFF_FFFF);
    lastp.byte_add(4).write(1);
    lastp.byte_add(6).cast::<u16>().write(0xFFFF);
    w32(lastp, 0, 0);
    w32(lastp, 0x0C, first);
    w32(mem, 0, size.wrapping_sub(0x20));
    mem.byte_add(4).write(0);
    w32(mem, 0x0C, 0);
    let begin = h.wrapping_add(8);
    let end = h.wrapping_add(0x20);
    w32(heap, 0x34, begin); // end.prev = begin
    w32(heap, 0x20, 0x7FFF_FFFF); // end.size
    w32(heap, 0x18, end); // begin.next = end
    // InsertHeapFreeBlock(first) after begin
    let next = heap.byte_add(0x18).cast::<u32>().read();
    // `next` is the `end` just stored (re-read as the original does); keep the full-width pointer
    // so a 64-bit build addresses the same memory.
    let next_p = if next == end {
        heap.byte_add(0x20)
    } else {
        GPtr::from_addr(next as usize)
    };
    w32(mem, 0x10, next);
    w32(next_p, 0x14, first);
    w32(mem, 0x14, begin);
    w32(heap, 0x18, first);
    if init_sizes != 0 {
        w32(heap, 0x38, 0);
        let cs = heap_malloc(heap, 0xF00);
        w32(heap, 0x38, cs.addr() as u32);
        for i in 0..64u32 {
            // `[esi+38h]` is re-read for every element (it is `cs`, see above)
            let v = heap.byte_add(0x38).cast::<u32>().read();
            let base = if v == cs.addr() as u32 {
                cs
            } else {
                GPtr::from_addr(v as usize)
            };
            common_size_init(base.byte_add((0x3C * i) as isize), 16 * (i + 1));
        }
    }
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_MGR_MALLOC,
    HOOK_MGR_FREE,
    HOOK_MGR_REALLOC,
    HOOK_MGR_CALLOC,
    HOOK_INIT_SCRATCH_PAD,
    HOOK_MALLOC_ALIGN,
    HOOK_FREE_ALIGN,
    HOOK_HEAP_INIT,
];

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn align() {
        assert_eq!(align_up(0x1001, 16), 0x1010);
        assert_eq!(align_up(0x1000, 16), 0x1010);
        assert_eq!(align_up(0x100F, 16), 0x1010);
        init();
        nullsub_72f3c0();
    }

    #[test]
    fn heap_init_without_sizes() {
        let mut heap = vec![0xAAu8; 0x48];
        let mut mem = vec![0xBBu8; 0x100];
        let (h, m) = (
            GPtr::from_raw(heap.as_mut_ptr()),
            GPtr::from_raw(mem.as_mut_ptr()),
        );
        heap_init(h, m, 0x100, 0);
        let r = |b: &[u8], o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let (ha, ma) = (h.addr() as u32, m.addr() as u32);
        assert_eq!(r(&heap, 0), ma);
        assert_eq!(r(&heap, 4), ma + 0xF0);
        assert_eq!(r(&heap, 0x18), ma);
        assert_eq!(r(&heap, 0x20), 0x7FFF_FFFF);
        assert_eq!(r(&heap, 0x1C), 0xAAAA_AAAA); // begin.prev untouched
        assert_eq!(r(&heap, 0x30), 0xAAAA_AAAA); // end.next untouched
        assert_eq!(r(&heap, 0x38), 0xAAAA_AAAA); // common sizes untouched
        assert_eq!(heap[0x3C], 1);
        assert_eq!(r(&heap, 0x40), 0);
        assert_eq!(r(&heap, 0x44), 0xFFFF_FFFF);
        assert_eq!(r(&mem, 0), 0xE0);
        assert_eq!(mem[4], 0);
        assert_eq!(r(&mem, 0x0C), 0);
        assert_eq!(r(&mem, 0x10), ha + 0x20);
        assert_eq!(r(&mem, 0x14), ha + 8);
        assert_eq!(r(&mem, 0xF0), 0);
        assert_eq!(mem[0xF4], 1);
        assert_eq!(&mem[0xF6..0xF8], &[0xFF, 0xFF]);
        assert_eq!(r(&mem, 0xFC), ma);
        assert_eq!(r(&heap, 0x20 + 0x14), ma); // end.prev = first
    }
}
