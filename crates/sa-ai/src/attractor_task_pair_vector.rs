//! `std::vector<CPedTaskPair>` (0x14-byte elements: ped, task, slot, 4 bytes, the "used" flag at +0x10), the vector
//! an attractor keeps its handed-out tasks in (+0x2C): `push_back` 0x5EE270, `insert` 0x5EDD90, `_Insert_n` 0x5ED1B0,
//! `uninitialized_fill_n` 0x5EB2E0, `uninitialized_copy` 0x5EAE40, `std::copy` 0x5EAB80 (the erase move) (batch
//! `std13-ai-1`; `size()` 0x5E9800 is in `misc.rs`). MSVC 7.1 STL, the same algorithm as the pointer vectors of
//! `attractor_ptr_vectors.rs` with 0x14-byte strides (counts by the signed `/ 0x14` multiply idiom, limit 0x0CCCCCCC).
//! Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
//!
//! Layout: +0 allocator, +4 first, +8 last, +0xC end.

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

use sa_core::bindings::crt_free_8214bd as operator_delete;
use sa_core::bindings::crt_new as operator_new;
orig! {
    /// `vector<CPedTaskPair>::_Xlen()` (0x5ECB30, throws `length_error`)
    fn xlen(v: GPtr<u8>) = thiscall 0x5ECB30 => unported;
    /// `vector<CPedTaskPair>::_Ucopy(first, last, dest)` (0x5EB960, `ret 0xC`; in-place insertion only)
    fn ucopy(v: GPtr<u8>, first: u32, last: u32, dest: u32) -> u32 = thiscall 0x5EB960 => ucopy_port;
    /// `vector<CPedTaskPair>::_Ufill(dest, count, const T*)` (0x5EC0E0, `ret 0xC`; in-place insertion only)
    fn ufill(v: GPtr<u8>, dest: u32, n: u32, val: GPtr<u8>) -> u32 = thiscall 0x5EC0E0 => ufill_port;
    /// `std::copy_backward` for `CPedTaskPair*` (0x5EB170; in-place insertion only)
    fn copy_backward(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> = cdecl 0x5EB170 => copy_backward_port;
    /// `std::fill` for `CPedTaskPair*` (0x5EAAC0; in-place insertion only)
    fn fill(first: u32, last: u32, val: GPtr<u8>) = cdecl 0x5EAAC0 => fill_port;
    fn fill_n_o(dest: u32, n: u32, val: GPtr<u8>) -> u32 = cdecl 0x5EB2E0 => fill_n;
    fn ucopy_o(first: u32, last: u32, dest: u32) -> u32 = cdecl 0x5EAE40 => uninitialized_copy;
    /// `vector<CPedTaskPair>::size()` (0x5E9800, ported in `misc.rs`)
    fn size_o(v: GPtr<u8>) -> u32 = thiscall 0x5E9800 => unported;
    fn insert_n_o(v: GPtr<u8>, at: u32, n: u32, val: GPtr<u8>) = thiscall 0x5ED1B0 => insert_n;
    fn insert_o(v: GPtr<u8>, out: GPtr<u8>, at: u32, val: GPtr<u8>) -> GPtr<u8> = thiscall 0x5EDD90 => insert;
    /// `std::_Copy_backward_opt` for `CPedTaskPair*` (0x5EAC50, cdecl)
    fn copy_backward_opt_o(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> = cdecl 0x5EAC50 => copy_backward_opt;
}

/// Bytes per element.
const E: u32 = 0x14;

/// Element count of `v`'s span from first to the pointer at `hi` (`(p - first) / 0x14`, truncating; 0 when first is
/// null).
fn span(v: GPtr<u8>, hi: isize) -> u32 {
    let first = rd::<u32>(v, 4);
    if first == 0 { 0 } else { (rd::<u32>(v, hi).wrapping_sub(first) as i32 / E as i32) as u32 }
}

fn copy5(dst: u32, src: u32) {
    for i in 0..5 {
        GPtr::<u32>::from_addr(dst as usize + 4 * i).write(GPtr::<u32>::from_addr(src as usize + 4 * i).read());
    }
}

/// `uninitialized_fill_n(dest, count, const CPedTaskPair&)` (0x5EB2E0, cdecl; the caller pushes two more dwords):
/// count (unsigned) elements of 5 dwords from `dest`, a null destination skipped but advanced. Returns the end; a zero
/// count returns the count (0). The copy loop runs through the protection fragment 0x40A25B (`mov ecx, 5`).
#[hook(addr = 0x5EB2E0, conv = cdecl, name = "FUN_005eb2e0", expect = "8B 44 24 08 85 C0 76 25",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "the 5-dword copy count is set in the protection fragment 0x40A25B")]
pub fn fill_n(dest: u32, n: u32, val: GPtr<u8>) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut p = dest;
    for _ in 0..n {
        if p != 0 {
            copy5(p, val.addr() as u32);
        }
        p = p.wrapping_add(E);
    }
    p
}

/// `uninitialized_copy(first, last, dest)` (0x5EAE40, cdecl): 5-dword elements, null destinations skipped but
/// advanced. Returns the destination end.
#[hook(addr = 0x5EAE40, conv = cdecl, name = "FUN_005eae40", expect = "8B 54 24 04 8B 44 24 0C",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn uninitialized_copy(first: u32, last: u32, dest: u32) -> u32 {
    let (mut s, mut d) = (first, dest);
    while s != last {
        if d != 0 {
            copy5(d, s);
        }
        s = s.wrapping_add(E);
        d = d.wrapping_add(E);
    }
    d
}

/// `std::copy(first, last, dest)` for `CPedTaskPair*` (0x5EAB80, cdecl, the iterator result through the first
/// argument): forward 5-dword moves (the erase shift). `*out` = the destination end.
#[hook(addr = 0x5EAB80, conv = cdecl, name = "FUN_005eab80", expect = "8B 44 24 08 53 8B 5C 24",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn copy(out: GPtr<u8>, first: u32, last: u32, dest: u32) -> GPtr<u8> {
    let (mut s, mut d) = (first, dest);
    while s != last {
        copy5(d, s);
        s = s.wrapping_add(E);
        d = d.wrapping_add(E);
    }
    wr::<u32>(out, 0, d);
    out
}

/// `vector<CPedTaskPair>::_Insert_n(where, count, const T&)` (0x5ED1B0, thiscall, `ret 0xC`, SEH frame). The value is
/// copied to a local first. count 0 → nothing; size + count above 0x0CCCCCCC → `_Xlen`. Capacity below size + count:
/// new capacity cap + cap/2 (0 when that passes 0x0CCCCCCC), at least `size()` + count; `operator new`, copy [first,
/// where), fill count, copy [where, last), free the old block, set first / last / end. Otherwise in place through
/// `_Ucopy` / `_Ufill` / `copy_backward` / `fill` (never reached from `push_back`, the only caller of `insert`).
#[hook(addr = 0x5ED1B0, conv = thiscall, name = "FUN_005ed1b0", expect = "55 8B EC 6A FF 68 70 DB",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn insert_n(v: GPtr<u8>, at: u32, n: u32, valp: GPtr<u8>) {
    let mut val: [u32; 5] = core::array::from_fn(|i| rd::<u32>(valp, 4 * i as isize));
    let vp: GPtr<u8> = GPtr::from_raw(val.as_mut_ptr()).cast();
    let cap = span(v, 0xC);
    if n == 0 {
        return;
    }
    let size = span(v, 8);
    if 0x0CCC_CCCC - size < n {
        xlen(v);
        return;
    }
    if cap < size.wrapping_add(n) {
        let half = cap >> 1;
        let mut new_cap = if 0x0CCC_CCCC - half < cap { 0 } else { cap + half };
        if new_cap < span(v, 8).wrapping_add(n) {
            new_cap = size_o(v).wrapping_add(n);
        }
        let bytes = new_cap * E;
        let buf = GPtr::<u8>::addr(operator_new(bytes)) as u32;
        let p = ucopy_o(rd::<u32>(v, 4), at, buf);
        fill_n_o(p, n, vp);
        ucopy_o(at, rd::<u32>(v, 8), p.wrapping_add(n * E));
        let new_size = n.wrapping_add(span(v, 8));
        let old = rd::<u32>(v, 4);
        if old != 0 {
            operator_delete(GPtr::from_addr(old as usize));
        }
        wr::<u32>(v, 0xC, buf.wrapping_add(bytes));
        wr::<u32>(v, 8, buf.wrapping_add(new_size * E));
        wr::<u32>(v, 4, buf);
        return;
    }
    let last = rd::<u32>(v, 8);
    let tail = (last.wrapping_sub(at) as i32 / E as i32) as u32;
    let fill_end = if tail >= n {
        let mid = last.wrapping_sub(n * E);
        let e = ucopy(v, mid, last, last);
        wr::<u32>(v, 8, e);
        let mut out = 0u32;
        copy_backward(GPtr::from_raw(&raw mut out).cast(), at, mid, last);
        at.wrapping_add(n * E)
    } else {
        ucopy(v, at, last, at.wrapping_add(n * E));
        ufill(v, rd::<u32>(v, 8), n.wrapping_sub(tail), vp);
        let nl = rd::<u32>(v, 8).wrapping_add(n * E);
        wr::<u32>(v, 8, nl);
        nl.wrapping_sub(n * E)
    };
    fill(at, fill_end, vp);
}

/// `vector<CPedTaskPair>::insert(where, const T&)` (0x5EDD90, thiscall, `ret 0xC`, the iterator result through the
/// first argument): index = (where - first) / 0x14 (0 without storage), `_Insert_n(where, 1, val)`, `*out` = first +
/// index * 0x14 (the new storage). The original keeps the size in its third argument slot (dead).
#[hook(addr = 0x5EDD90, conv = thiscall, name = "FUN_005edd90", expect = "53 8B 5C 24 0C 56 8B F1",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk")]
pub fn insert(v: GPtr<u8>, out: GPtr<u8>, at: u32, val: GPtr<u8>) -> GPtr<u8> {
    let first = rd::<u32>(v, 4);
    let idx = if first == 0 || span(v, 8) == 0 {
        0
    } else {
        (at.wrapping_sub(first) as i32 / E as i32) as u32
    };
    insert_n_o(v, at, 1, val);
    wr::<u32>(out, 0, rd::<u32>(v, 4).wrapping_add(idx * E));
    out
}

/// `vector<CPedTaskPair>::push_back(const T&)` (0x5EE270, thiscall, `ret 4`): with storage and size below capacity
/// (unsigned) `uninitialized_fill_n(last, 1, val)` and last += 0x14; otherwise `insert(last, val)` (its iterator
/// result written over the argument slot).
#[hook(addr = 0x5EE270, conv = thiscall, name = "FUN_005ee270", expect = "53 56 8B F1 57 8B 7E 04",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk")]
pub fn push_back(v: GPtr<u8>, val: GPtr<u8>) {
    let size = span(v, 8);
    if rd::<u32>(v, 4) != 0 && size < span(v, 0xC) {
        let last = rd::<u32>(v, 8);
        fill_n_o(last, 1, val);
        wr::<u32>(v, 8, last.wrapping_add(E));
    } else {
        let mut it = 0u32;
        insert_o(v, GPtr::from_raw(&raw mut it).cast(), rd::<u32>(v, 8), val);
    }
}

// ---- frontier2-c1: the in-place insertion helpers ----

/// `std::fill<CPedTaskPair*>` (0x5EAAC0, cdecl): every element of `[first, last)` becomes the 5 dwords at `val`
/// (`rep movsd`, ECX = 5).
#[hook(addr = 0x5EAAC0, conv = cdecl, name = "FUN_005eaac0", expect = "8B 44 24 04 8B 54 24 08 3B C2",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn fill_port(first: u32, last: u32, val: GPtr<u8>) {
    let mut p = first;
    while p != last {
        let d = p;
        p = p.wrapping_add(E);
        copy5(d, val.addr() as u32);
    }
}

/// `std::_Copy_backward_opt<CPedTaskPair*>` (0x5EAC50, cdecl; a fifth argument pushed by the caller, unread): 5-dword
/// moves from the back; `*out` = the destination start (`dest_last` when the range is empty). Returns `out`.
#[hook(addr = 0x5EAC50, conv = cdecl, name = "FUN_005eac50", expect = "8B 44 24 0C 53 8B 5C 24 0C",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn copy_backward_opt(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> {
    let (mut s, mut d) = (last, dest_last);
    while s != first {
        s = s.wrapping_sub(E);
        d = d.wrapping_sub(E);
        copy5(d, s);
    }
    wr::<u32>(out, 0, d);
    out
}

/// `std::copy_backward<CPedTaskPair*>` (0x5EB170, cdecl): `_Copy_backward_opt` 0x5EAC50 with the same arguments (and
/// `dest_last` again); returns `out`.
#[hook(addr = 0x5EB170, conv = cdecl, name = "FUN_005eb170", expect = "8B 44 24 10 8B 54 24 0C 56",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn copy_backward_port(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> {
    copy_backward_opt_o(out, first, last, dest_last);
    out
}

/// `vector<CPedTaskPair>::_Ucopy(first, last, dest)` (0x5EB960, thiscall, `ret 0xC`): `uninitialized_copy` 0x5EAE40;
/// its result.
#[hook(addr = 0x5EB960, conv = thiscall, name = "FUN_005eb960", expect = "8B 44 24 0C 8B 54 24 08 50 51",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn ucopy_port(_v: GPtr<u8>, first: u32, last: u32, dest: u32) -> u32 {
    ucopy_o(first, last, dest)
}

/// `vector<CPedTaskPair>::_Ufill(dest, count, const T&)` (0x5EC0E0, thiscall, `ret 0xC`): `uninitialized_fill_n`
/// 0x5EB2E0, then `dest + count * 0x14`.
#[hook(addr = 0x5EC0E0, conv = thiscall, name = "FUN_005ec0e0", expect = "8B 44 24 0C 56 8B 74 24 0C 57",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn ufill_port(_v: GPtr<u8>, dest: u32, n: u32, val: GPtr<u8>) -> u32 {
    fill_n_o(dest, n, val);
    dest.wrapping_add(n.wrapping_mul(5).wrapping_mul(4))
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_FILL_PORT,
    HOOK_COPY_BACKWARD_OPT,
    HOOK_COPY_BACKWARD_PORT,
    HOOK_UCOPY_PORT,
    HOOK_UFILL_PORT,
    HOOK_FILL_N,
    HOOK_UNINITIALIZED_COPY,
    HOOK_COPY,
    HOOK_INSERT_N,
    HOOK_INSERT,
    HOOK_PUSH_BACK,
];
