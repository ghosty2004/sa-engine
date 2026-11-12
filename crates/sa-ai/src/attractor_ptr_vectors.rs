//! The two `std::vector<T*>` instantiations the ped-attractor registry grows (batch `writerq-ai-3`): `CPedAttractor*`
//! (the manager's per-type vectors: `push_back` 0x5EE400, `_Insert_n` 0x5EDA50, `uninitialized_fill_n` 0x5EB390,
//! `uninitialized_copy` 0x5EAEE0) and `CPed*` (an attractor's ped vectors: 0x5EE200, 0x5ECF10, 0x5EB2A0, 0x5EAE10).
//! The two sets are byte-identical up to their callees' addresses. Ported from the 1.0 US bytes (MSVC 7.1 STL).
//! Unverified (docs/verify/HOWTO.md).
//!
//! Layout: +0 allocator, +4 first, +8 last, +0xC end.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `operator new(size_t)` (0x82119A)
    fn operator_new(n: u32) -> u32 = cdecl 0x82119A => unported;
    /// `operator delete(void*)` (0x8214BD)
    fn operator_delete(p: u32) = cdecl 0x8214BD => unported;
    /// `vector<CPedAttractor*>::_Xlen()` (0x5ECCB0, throws `length_error`)
    fn xlen_a(v: GPtr<u8>) = thiscall 0x5ECCB0 => unported;
    /// `vector<CPed*>::_Xlen()` (0x5ECAB0, throws `length_error`)
    fn xlen_p(v: GPtr<u8>) = thiscall 0x5ECAB0 => unported;
    /// `vector<CPedAttractor*>::_Ucopy(first, last, dest)` (0x5EBA00, `ret 0xC`)
    fn ucopy_a(v: GPtr<u8>, first: u32, last: u32, dest: u32) -> u32 = thiscall 0x5EBA00 => unported;
    /// `vector<CPed*>::_Ucopy(first, last, dest)` (0x5EB930, `ret 0xC`)
    fn ucopy_p(v: GPtr<u8>, first: u32, last: u32, dest: u32) -> u32 = thiscall 0x5EB930 => unported;
    /// `vector<CPedAttractor*>::_Ufill(dest, count, const T*)` (0x5EC1B0, `ret 0xC`)
    fn ufill_a(v: GPtr<u8>, dest: u32, n: u32, val: GPtr<u8>) -> u32 = thiscall 0x5EC1B0 => unported;
    /// `vector<CPed*>::_Ufill(dest, count, const T*)` (0x5EC070, `ret 0xC`)
    fn ufill_p(v: GPtr<u8>, dest: u32, n: u32, val: GPtr<u8>) -> u32 = thiscall 0x5EC070 => unported;
    /// `std::copy_backward` for `CPedAttractor**` (0x5EB260, iterator result through the first argument)
    fn copy_backward_a(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> = cdecl 0x5EB260 => unported;
    /// `std::copy_backward` for `CPed**` (0x5EB140)
    fn copy_backward_p(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> = cdecl 0x5EB140 => unported;
    /// `std::fill` for `CPedAttractor**` (0x5EAB10)
    fn fill_a(first: u32, last: u32, val: GPtr<u8>) = cdecl 0x5EAB10 => unported;
    /// `std::fill` for `CPed**` (0x5EAAA0)
    fn fill_p(first: u32, last: u32, val: GPtr<u8>) = cdecl 0x5EAAA0 => unported;
    fn fill_n_a_o(dest: u32, n: u32, val: GPtr<u8>) -> u32 = cdecl 0x5EB390 => fill_n_a;
    fn fill_n_p_o(dest: u32, n: u32, val: GPtr<u8>) -> u32 = cdecl 0x5EB2A0 => fill_n_p;
    fn copy_a_o(first: u32, last: u32, dest: u32) -> u32 = cdecl 0x5EAEE0 => copy_a;
    fn copy_p_o(first: u32, last: u32, dest: u32) -> u32 = cdecl 0x5EAE10 => copy_p;
    fn insert_n_a_o(v: GPtr<u8>, at: u32, n: u32, val: GPtr<u8>) = thiscall 0x5EDA50 => insert_n_a;
    fn insert_n_p_o(v: GPtr<u8>, at: u32, n: u32, val: GPtr<u8>) = thiscall 0x5ECF10 => insert_n_p;
}

/// One instantiation's callees.
struct Inst {
    xlen: fn(GPtr<u8>),
    ucopy: fn(GPtr<u8>, u32, u32, u32) -> u32,
    ufill: fn(GPtr<u8>, u32, u32, GPtr<u8>) -> u32,
    copy_backward: fn(GPtr<u8>, u32, u32, u32) -> GPtr<u8>,
    fill: fn(u32, u32, GPtr<u8>),
    fill_n: fn(u32, u32, GPtr<u8>) -> u32,
    copy: fn(u32, u32, u32) -> u32,
    insert_n: fn(GPtr<u8>, u32, u32, GPtr<u8>),
}

const ATTRACTORS: Inst = Inst {
    xlen: xlen_a,
    ucopy: ucopy_a,
    ufill: ufill_a,
    copy_backward: copy_backward_a,
    fill: fill_a,
    fill_n: fill_n_a_o,
    copy: copy_a_o,
    insert_n: insert_n_a_o,
};

const PEDS: Inst = Inst {
    xlen: xlen_p,
    ucopy: ucopy_p,
    ufill: ufill_p,
    copy_backward: copy_backward_p,
    fill: fill_p,
    fill_n: fill_n_p_o,
    copy: copy_p_o,
    insert_n: insert_n_p_o,
};

/// Element count from two pointers (`sar 2`), 0 when `first` is null (MSVC's `size()` / `capacity()`).
fn span(v: GPtr<u8>, hi: isize) -> u32 {
    let first = rd::<u32>(v, 4);
    if first == 0 {
        0
    } else {
        ((rd::<u32>(v, hi).wrapping_sub(first) as i32) >> 2) as u32
    }
}

/// `uninitialized_fill_n(dest, count, &val)`: count (unsigned) slots from `dest`, a null slot pointer skipped but
/// still advanced. Returns the end (0 for a zero count: the original returns the count).
fn fill_n(dest: u32, n: u32, val: GPtr<u8>) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut p = dest;
    for _ in 0..n {
        if p != 0 {
            GPtr::<u32>::from_addr(p as usize).write(rd::<u32>(val, 0));
        }
        p = p.wrapping_add(4);
    }
    p
}

/// `uninitialized_copy(first, last, dest)`: null destination slots skipped but advanced. Returns the end.
fn copy(first: u32, last: u32, dest: u32) -> u32 {
    let (mut s, mut d) = (first, dest);
    while s != last {
        if d != 0 {
            GPtr::<u32>::from_addr(d as usize).write(GPtr::<u32>::from_addr(s as usize).read());
        }
        s = s.wrapping_add(4);
        d = d.wrapping_add(4);
    }
    d
}

/// `push_back(const T&)`: with storage and size below capacity (unsigned) the value goes to `last` (`fill_n`) and
/// last += 4; otherwise `_Insert_n(last, 1, &val)`.
fn push_back(i: &Inst, v: GPtr<u8>, val: GPtr<u8>) {
    let size = span(v, 8);
    if rd::<u32>(v, 4) != 0 && size < span(v, 0xC) {
        let last = rd::<u32>(v, 8);
        (i.fill_n)(last, 1, val);
        wr::<u32>(v, 8, last.wrapping_add(4));
    } else {
        (i.insert_n)(v, rd::<u32>(v, 8), 1, val);
    }
}

/// `_Insert_n(where, count, const T& val)`. The value is copied first (into the original's argument slot; the copy is
/// what gets inserted). count 0 → nothing; size + count over 0x3FFFFFFF → `_Xlen`. Capacity below size + count:
/// new capacity = cap + cap/2 (0 when that overflows 0x3FFFFFFF), at least size + count; `operator new`, copy [first,
/// where), fill count, copy [where, last), `operator delete` of the old block, first / last / end set. Otherwise in
/// place through the instantiation's `_Ucopy` / `_Ufill` / `copy_backward` / `fill` (never reached from `push_back`,
/// the only caller).
fn insert_n(i: &Inst, v: GPtr<u8>, at: u32, n: u32, valp: GPtr<u8>) {
    let mut val = rd::<u32>(valp, 0);
    let vp: GPtr<u8> = GPtr::from_raw(&raw mut val).cast();
    let cap = span(v, 0xC);
    if n == 0 {
        return;
    }
    let size = span(v, 8);
    if 0x3FFF_FFFF - size < n {
        (i.xlen)(v);
        return;
    }
    if cap < size.wrapping_add(n) {
        let half = cap >> 1;
        let mut new_cap = if 0x3FFF_FFFF - half < cap { 0 } else { cap + half };
        let need = span(v, 8).wrapping_add(n);
        if new_cap < need {
            new_cap = need;
        }
        let bytes = new_cap << 2;
        let buf = operator_new(bytes);
        let p = (i.copy)(rd::<u32>(v, 4), at, buf);
        (i.fill_n)(p, n, vp);
        (i.copy)(at, rd::<u32>(v, 8), p.wrapping_add(n * 4));
        let new_size = n.wrapping_add(span(v, 8));
        let old = rd::<u32>(v, 4);
        if old != 0 {
            operator_delete(old);
        }
        wr::<u32>(v, 0xC, buf.wrapping_add(bytes));
        wr::<u32>(v, 8, buf.wrapping_add(new_size * 4));
        wr::<u32>(v, 4, buf);
        return;
    }
    let last = rd::<u32>(v, 8);
    let fill_end = if (last.wrapping_sub(at) as i32 >> 2) as u32 >= n {
        let mid = last.wrapping_sub(n * 4);
        let e = (i.ucopy)(v, mid, last, last);
        wr::<u32>(v, 8, e);
        let mut out = 0u32;
        (i.copy_backward)(GPtr::from_raw(&raw mut out).cast(), at, mid, last);
        at.wrapping_add(n * 4)
    } else {
        (i.ucopy)(v, at, last, at.wrapping_add(n * 4));
        let tail = (last.wrapping_sub(at) as i32 >> 2) as u32;
        (i.ufill)(v, rd::<u32>(v, 8), n.wrapping_sub(tail), vp);
        let nl = rd::<u32>(v, 8).wrapping_add(n * 4);
        wr::<u32>(v, 8, nl);
        nl.wrapping_sub(n * 4)
    };
    (i.fill)(at, fill_end, vp);
}

/// `std::vector<CPedAttractor*>::push_back(const T&)` (0x5EE400, thiscall, `ret 4`): see `push_back`.
#[hook(addr = 0x5EE400, conv = thiscall, name = "FUN_005ee400", expect = "56 8B F1 8B 56 04",
       unverified = "writerq-ai-3", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn push_back_a(v: GPtr<u8>, val: GPtr<u8>) {
    push_back(&ATTRACTORS, v, val);
}

/// `std::vector<CPed*>::push_back(const T&)` (0x5EE200, thiscall, `ret 4`): see `push_back`.
#[hook(addr = 0x5EE200, conv = thiscall, name = "FUN_005ee200", expect = "56 8B F1 8B 56 04",
       unverified = "writerq-ai-3", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn push_back_p(v: GPtr<u8>, val: GPtr<u8>) {
    push_back(&PEDS, v, val);
}

/// `std::vector<CPedAttractor*>::_Insert_n(iterator, size_t, const T&)` (0x5EDA50, thiscall, `ret 0xC`, SEH frame):
/// see `insert_n`.
#[hook(addr = 0x5EDA50, conv = thiscall, name = "FUN_005eda50", expect = "55 8B EC 6A FF",
       unverified = "writerq-ai-3", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "the original copies the value into its own argument slot")]
pub fn insert_n_a(v: GPtr<u8>, at: u32, n: u32, val: GPtr<u8>) {
    insert_n(&ATTRACTORS, v, at, n, val);
}

/// `std::vector<CPed*>::_Insert_n(iterator, size_t, const T&)` (0x5ECF10, thiscall, `ret 0xC`, SEH frame): see
/// `insert_n`.
#[hook(addr = 0x5ECF10, conv = thiscall, name = "FUN_005ecf10", expect = "55 8B EC 6A FF",
       unverified = "writerq-ai-3", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "the original copies the value into its own argument slot")]
pub fn insert_n_p(v: GPtr<u8>, at: u32, n: u32, val: GPtr<u8>) {
    insert_n(&PEDS, v, at, n, val);
}

/// `uninitialized_fill_n<CPedAttractor**>` (0x5EB390, cdecl): see `fill_n`.
#[hook(addr = 0x5EB390, conv = cdecl, name = "FUN_005eb390", expect = "8B 44 24 08 85 C0",
       unverified = "writerq-ai-3", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn fill_n_a(dest: u32, n: u32, val: GPtr<u8>) -> u32 {
    fill_n(dest, n, val)
}

/// `uninitialized_fill_n<CPed**>` (0x5EB2A0, cdecl): see `fill_n`.
#[hook(addr = 0x5EB2A0, conv = cdecl, name = "FUN_005eb2a0", expect = "8B 44 24 08 85 C0",
       unverified = "writerq-ai-3", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn fill_n_p(dest: u32, n: u32, val: GPtr<u8>) -> u32 {
    fill_n(dest, n, val)
}

/// `uninitialized_copy<CPedAttractor**>` (0x5EAEE0, cdecl): see `copy`.
#[hook(addr = 0x5EAEE0, conv = cdecl, name = "FUN_005eaee0", expect = "8B 4C 24 04 8B 54 24 08",
       unverified = "writerq-ai-3", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn copy_a(first: u32, last: u32, dest: u32) -> u32 {
    copy(first, last, dest)
}

/// `uninitialized_copy<CPed**>` (0x5EAE10, cdecl): see `copy`.
#[hook(addr = 0x5EAE10, conv = cdecl, name = "FUN_005eae10", expect = "8B 4C 24 04 8B 54 24 08",
       unverified = "writerq-ai-3", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn copy_p(first: u32, last: u32, dest: u32) -> u32 {
    copy(first, last, dest)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_PUSH_BACK_A,
    HOOK_PUSH_BACK_P,
    HOOK_INSERT_N_A,
    HOOK_INSERT_N_P,
    HOOK_FILL_N_A,
    HOOK_FILL_N_P,
    HOOK_COPY_A,
    HOOK_COPY_P,
];
