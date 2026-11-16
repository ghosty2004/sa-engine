//! The two `std::vector<T*>` instantiations the ped-attractor registry grows (batch `writerq-ai-3`): `CPedAttractor*`
//! (the manager's per-type vectors: `push_back` 0x5EE400, `_Insert_n` 0x5EDA50, `uninitialized_fill_n` 0x5EB390,
//! `uninitialized_copy` 0x5EAEE0), `CPed*` (an attractor's ped vectors: 0x5EE200, 0x5ECF10, 0x5EB2A0, 0x5EAE10) and,
//! batch `std13-ai-1`, `CTask*` (`CPedAttractor::ms_tasks`: 0x5EE300, 0x5ED4B0, 0x5EB310, 0x5EAE70).
//! The two sets are byte-identical up to their callees' addresses. Ported from the 1.0 US bytes (MSVC 7.1 STL).
//! Batch `frontier2-a9` adds `std::vector<CVector>` (the shelter displacements 0xC0986C): `_Insert_n` 0x5ED750 and
//! its helpers, the same body with 0xC-byte elements (counts divided by 12, max_size 0x15555555), the size taken by
//! a call (0x5E9850) when the grown capacity is too small, and the value copied into a local.
//! Unverified (docs/verify/HOWTO.md).
//!
//! Layout: +0 allocator, +4 first, +8 last, +0xC end.

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

use crate::bindings::fill_n_t_o;
use crate::bindings::insert_n_t_o;
use crate::bindings::vector3_fill_n;
use sa_core::bindings::crt_free_8214bd as operator_delete;
use sa_core::bindings::crt_new as operator_new;
orig! {
    /// `vector<CPedAttractor*>::_Xlen()` (0x5ECCB0, throws `length_error`)
    fn xlen_a(v: GPtr<u8>) = thiscall 0x5ECCB0 => unported;
    /// `vector<CPed*>::_Xlen()` (0x5ECAB0, throws `length_error`)
    fn xlen_p(v: GPtr<u8>) = thiscall 0x5ECAB0 => unported;
    /// `vector<CPedAttractor*>::_Ucopy(first, last, dest)` (0x5EBA00, `ret 0xC`)
    fn ucopy_a(v: GPtr<u8>, first: u32, last: u32, dest: u32) -> u32 = thiscall 0x5EBA00 => ucopy_a_port;
    /// `vector<CPed*>::_Ucopy(first, last, dest)` (0x5EB930, `ret 0xC`)
    fn ucopy_p(v: GPtr<u8>, first: u32, last: u32, dest: u32) -> u32 = thiscall 0x5EB930 => ucopy_p_port;
    /// `vector<CPedAttractor*>::_Ufill(dest, count, const T*)` (0x5EC1B0, `ret 0xC`)
    fn ufill_a(v: GPtr<u8>, dest: u32, n: u32, val: GPtr<u8>) -> u32 = thiscall 0x5EC1B0 => ufill_a_port;
    /// `vector<CPed*>::_Ufill(dest, count, const T*)` (0x5EC070, `ret 0xC`)
    fn ufill_p(v: GPtr<u8>, dest: u32, n: u32, val: GPtr<u8>) -> u32 = thiscall 0x5EC070 => ufill_p_port;
    /// `std::copy_backward` for `CPedAttractor**` (0x5EB260, iterator result through the first argument)
    fn copy_backward_a(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> = cdecl 0x5EB260 => copy_backward_a_port;
    /// `std::copy_backward` for `CPed**` (0x5EB140)
    fn copy_backward_p(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> = cdecl 0x5EB140 => copy_backward_p_port;
    /// `std::fill` for `CPedAttractor**` (0x5EAB10)
    fn fill_a(first: u32, last: u32, val: GPtr<u8>) = cdecl 0x5EAB10 => fill_a_port;
    /// `std::fill` for `CPed**` (0x5EAAA0)
    fn fill_p(first: u32, last: u32, val: GPtr<u8>) = cdecl 0x5EAAA0 => fill_p_port;
    fn fill_n_a_o(dest: u32, n: u32, val: GPtr<u8>) -> u32 = cdecl 0x5EB390 => fill_n_a;
    fn fill_n_p_o(dest: u32, n: u32, val: GPtr<u8>) -> u32 = cdecl 0x5EB2A0 => fill_n_p;
    fn copy_a_o(first: u32, last: u32, dest: u32) -> u32 = cdecl 0x5EAEE0 => copy_a;
    fn copy_p_o(first: u32, last: u32, dest: u32) -> u32 = cdecl 0x5EAE10 => copy_p;
    fn insert_n_a_o(v: GPtr<u8>, at: u32, n: u32, val: GPtr<u8>) = thiscall 0x5EDA50 => insert_n_a;
    fn insert_n_p_o(v: GPtr<u8>, at: u32, n: u32, val: GPtr<u8>) = thiscall 0x5ECF10 => insert_n_p;
    /// `vector<CTask*>::_Xlen()` (0x5ECBB0, throws `length_error`)
    fn xlen_t(v: GPtr<u8>) = thiscall 0x5ECBB0 => unported;
    /// `vector<CTask*>::_Ucopy(first, last, dest)` (0x5EB990, `ret 0xC`)
    fn ucopy_t(v: GPtr<u8>, first: u32, last: u32, dest: u32) -> u32 = thiscall 0x5EB990 => ucopy_t_port;
    /// `vector<CTask*>::_Ufill(dest, count, const T*)` (0x5EC110, `ret 0xC`)
    fn ufill_t(v: GPtr<u8>, dest: u32, n: u32, val: GPtr<u8>) -> u32 = thiscall 0x5EC110 => ufill_t_port;
    /// `std::copy_backward` for `CTask**` (0x5EB1B0)
    fn copy_backward_t(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> = cdecl 0x5EB1B0 => copy_backward_t_port;
    /// `std::fill` for `CTask**` (0x5EAAF0)
    fn fill_t(first: u32, last: u32, val: GPtr<u8>) = cdecl 0x5EAAF0 => fill_t_port;
    fn copy_t_o(first: u32, last: u32, dest: u32) -> u32 = cdecl 0x5EAE70 => copy_t;
    /// `vector<CVector>::_Xlen()` (0x5ECC30, throws `length_error`)
    fn xlen_v(v: GPtr<u8>) = thiscall 0x5ECC30 => unported;
    /// `vector<CVector>::size()` (0x5E9850, out of line)
    fn size_v_o(v: u32) -> u32 = thiscall 0x5E9850 => crate::gen_bodies::g_fun_005e9850_5e9850;
    fn ucopy_v_o(v: GPtr<u8>, first: u32, last: u32, dest: u32) -> u32 = thiscall 0x5EB9D0 => ucopy_v;
    fn ufill_v_o(v: GPtr<u8>, dest: u32, n: u32, val: GPtr<u8>) -> u32 = thiscall 0x5EC140 => ufill_v;
    fn copy_backward_v_o(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> = cdecl 0x5EB220 => copy_backward_v;
    fn copy_backward_opt_v_o(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> = cdecl 0x5EACC0 => copy_backward_opt_v;
    fn fill_v_o(first: u32, last: u32, val: GPtr<u8>) = cdecl 0x5EB1E0 => fill_v;
    fn copy_v_o(first: u32, last: u32, dest: u32) -> u32 = cdecl 0x5EAEA0 => copy_v;
    fn insert_n_v_o(v: GPtr<u8>, at: u32, n: u32, val: GPtr<u8>) = thiscall 0x5ED750 => insert_n_v;
}

/// One instantiation's callees.
struct Inst {
    /// dwords per element
    w: u32,
    /// `max_size()`
    max: u32,
    /// the out-of-line `size()` the growth path calls (`vector<CVector>` only)
    size: Option<fn(GPtr<u8>) -> u32>,
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
    w: 1,
    max: 0x3FFF_FFFF,
    size: None,
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
    w: 1,
    max: 0x3FFF_FFFF,
    size: None,
    xlen: xlen_p,
    ucopy: ucopy_p,
    ufill: ufill_p,
    copy_backward: copy_backward_p,
    fill: fill_p,
    fill_n: fill_n_p_o,
    copy: copy_p_o,
    insert_n: insert_n_p_o,
};

/// `vector<CTask*>` (`CPedAttractor::ms_tasks`, 0xC0985C; batch `std13-ai-1`).
const TASKS: Inst = Inst {
    w: 1,
    max: 0x3FFF_FFFF,
    size: None,
    xlen: xlen_t,
    ucopy: ucopy_t,
    ufill: ufill_t,
    copy_backward: copy_backward_t,
    fill: fill_t,
    fill_n: fill_n_t_o,
    copy: copy_t_o,
    insert_n: insert_n_t_o,
};

/// `std::vector<CVector>` (batch `frontier2-a9`; no push_back of its own is ported here, 0x5EE370 calls `insert`).
const VECTORS: Inst = Inst {
    w: 3,
    max: 0x1555_5555,
    size: Some(size_v),
    xlen: xlen_v,
    ucopy: ucopy_v_o,
    ufill: ufill_v_o,
    copy_backward: copy_backward_v_o,
    fill: fill_v_o,
    fill_n: vector3_fill_n,
    copy: copy_v_o,
    insert_n: insert_n_v_o,
};

/// `size()` through its out-of-line copy 0x5E9850.
fn size_v(v: GPtr<u8>) -> u32 {
    size_v_o(v.addr() as u32)
}

/// Element count of a byte difference: `sar 2` for pointers, the signed `imul 0x2AAAAAAB` division by 12 (truncating)
/// for 3-dword elements.
fn count(w: u32, diff: u32) -> u32 {
    if w == 1 {
        ((diff as i32) >> 2) as u32
    } else {
        ((diff as i32) / (w * 4) as i32) as u32
    }
}

/// Element count from two pointers, 0 when `first` is null (MSVC's `size()` / `capacity()`).
fn span(i: &Inst, v: GPtr<u8>, hi: isize) -> u32 {
    let first = rd::<u32>(v, 4);
    if first == 0 {
        0
    } else {
        count(i.w, rd::<u32>(v, hi).wrapping_sub(first))
    }
}

/// Copies one element of `w` dwords (+0, +4, ... in order).
fn copy_elem(w: u32, d: u32, s: u32) {
    for k in 0..w {
        let o = (k * 4) as usize;
        GPtr::<u32>::from_addr(d as usize + o).write(GPtr::<u32>::from_addr(s as usize + o).read());
    }
}

/// `uninitialized_fill_n(dest, count, &val)`: count (unsigned) elements of `w` dwords from `dest`, a null slot pointer
/// skipped but still advanced. Returns the end (0 for a zero count: the original returns the count).
fn fill_n_w(w: u32, dest: u32, n: u32, val: GPtr<u8>) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut p = dest;
    for _ in 0..n {
        if p != 0 {
            copy_elem(w, p, val.addr() as u32);
        }
        p = p.wrapping_add(w * 4);
    }
    p
}

/// [`fill_n_w`] for pointers.
fn fill_n(dest: u32, n: u32, val: GPtr<u8>) -> u32 {
    fill_n_w(1, dest, n, val)
}

/// `uninitialized_copy(first, last, dest)` of `w`-dword elements: null destination slots skipped but advanced.
/// Returns the end.
fn copy_w(w: u32, first: u32, last: u32, dest: u32) -> u32 {
    let (mut s, mut d) = (first, dest);
    while s != last {
        if d != 0 {
            copy_elem(w, d, s);
        }
        s = s.wrapping_add(w * 4);
        d = d.wrapping_add(w * 4);
    }
    d
}

/// [`copy_w`] for pointers.
fn copy(first: u32, last: u32, dest: u32) -> u32 {
    copy_w(1, first, last, dest)
}

/// `push_back(const T&)`: with storage and size below capacity (unsigned) the value goes to `last` (`fill_n`) and
/// last += 4; otherwise `_Insert_n(last, 1, &val)`.
fn push_back(i: &Inst, v: GPtr<u8>, val: GPtr<u8>) {
    let size = span(i, v, 8);
    if rd::<u32>(v, 4) != 0 && size < span(i, v, 0xC) {
        let last = rd::<u32>(v, 8);
        (i.fill_n)(last, 1, val);
        wr::<u32>(v, 8, last.wrapping_add(4));
    } else {
        (i.insert_n)(v, rd::<u32>(v, 8), 1, val);
    }
}

/// `_Insert_n(where, count, const T& val)`. The value is copied first (pointers: into the original's argument slot;
/// `CVector`: into a local; the copy is what gets inserted). count 0 → nothing; size + count over `max_size()` →
/// `_Xlen`. Capacity below size + count: new capacity = cap + cap/2 (0 when that overflows `max_size()`), at least
/// size + count (`CVector`: the size then taken by the call 0x5E9850); `operator new`, copy [first, where), fill
/// count, copy [where, last), `operator delete` of the old block, first / last / end set. Otherwise in place through
/// the instantiation's `_Ucopy` / `_Ufill` / `copy_backward` / `fill`.
fn insert_n(i: &Inst, v: GPtr<u8>, at: u32, n: u32, valp: GPtr<u8>) {
    let mut val = [0u32; 3];
    for (k, d) in val.iter_mut().take(i.w as usize).enumerate() {
        *d = rd::<u32>(valp, k as isize * 4);
    }
    let vp: GPtr<u8> = GPtr::from_raw(val.as_mut_ptr()).cast();
    let es = i.w * 4;
    let cap = span(i, v, 0xC);
    if n == 0 {
        return;
    }
    let size = span(i, v, 8);
    if i.max - size < n {
        (i.xlen)(v);
        return;
    }
    if cap < size.wrapping_add(n) {
        let half = cap >> 1;
        let mut new_cap = if i.max - half < cap { 0 } else { cap + half };
        let need = span(i, v, 8).wrapping_add(n);
        if new_cap < need {
            new_cap = match i.size {
                Some(f) => f(v).wrapping_add(n),
                None => need,
            };
        }
        let bytes = new_cap.wrapping_mul(es);
        let buf = GPtr::<u8>::addr(operator_new(bytes)) as u32;
        let p = (i.copy)(rd::<u32>(v, 4), at, buf);
        (i.fill_n)(p, n, vp);
        (i.copy)(at, rd::<u32>(v, 8), p.wrapping_add(n.wrapping_mul(es)));
        let new_size = n.wrapping_add(span(i, v, 8));
        let old = rd::<u32>(v, 4);
        if old != 0 {
            operator_delete(GPtr::from_addr(old as usize));
        }
        wr::<u32>(v, 0xC, buf.wrapping_add(bytes));
        wr::<u32>(v, 8, buf.wrapping_add(new_size.wrapping_mul(es)));
        wr::<u32>(v, 4, buf);
        return;
    }
    let last = rd::<u32>(v, 8);
    let fill_end = if count(i.w, last.wrapping_sub(at)) >= n {
        let mid = last.wrapping_sub(n.wrapping_mul(es));
        let e = (i.ucopy)(v, mid, last, last);
        wr::<u32>(v, 8, e);
        let mut out = 0u32;
        (i.copy_backward)(GPtr::from_raw(&raw mut out).cast(), at, mid, last);
        at.wrapping_add(n.wrapping_mul(es))
    } else {
        (i.ucopy)(v, at, last, at.wrapping_add(n.wrapping_mul(es)));
        let tail = count(i.w, rd::<u32>(v, 8).wrapping_sub(at));
        (i.ufill)(v, rd::<u32>(v, 8), n.wrapping_sub(tail), vp);
        let nl = rd::<u32>(v, 8).wrapping_add(n.wrapping_mul(es));
        wr::<u32>(v, 8, nl);
        nl.wrapping_sub(n.wrapping_mul(es))
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

/// `std::vector<CTask*>::push_back(const T&)` (0x5EE300, thiscall, `ret 4`; `CPedAttractor::ms_tasks`): see
/// `push_back`.
#[hook(addr = 0x5EE300, conv = thiscall, name = "FUN_005ee300", expect = "56 8B F1 8B 56 04 85 D2",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn push_back_t(v: GPtr<u8>, val: GPtr<u8>) {
    push_back(&TASKS, v, val);
}

/// `std::vector<CTask*>::_Insert_n(iterator, size_t, const T&)` (0x5ED4B0, thiscall, `ret 0xC`, SEH frame whose
/// handler push is in the protection fragment 0x4019E5): see `insert_n`.
#[hook(addr = 0x5ED4B0, conv = thiscall, name = "FUN_005ed4b0", expect = "55 8B EC 6A FF",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "the original copies the value into its own argument slot")]
pub fn insert_n_t(v: GPtr<u8>, at: u32, n: u32, val: GPtr<u8>) {
    insert_n(&TASKS, v, at, n, val);
}

/// `uninitialized_fill_n<CTask**>` (0x5EB310, cdecl): see `fill_n`.
#[hook(addr = 0x5EB310, conv = cdecl, name = "FUN_005eb310", expect = "8B 44 24 08 85 C0",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn fill_n_t(dest: u32, n: u32, val: GPtr<u8>) -> u32 {
    fill_n(dest, n, val)
}

/// `uninitialized_copy<CTask**>` (0x5EAE70, cdecl): see `copy`.
#[hook(addr = 0x5EAE70, conv = cdecl, name = "FUN_005eae70", expect = "8B 4C 24 04 8B 54 24 08",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn copy_t(first: u32, last: u32, dest: u32) -> u32 {
    copy(first, last, dest)
}

// ---- frontier2-c1: the in-place insertion helpers of the three instantiations (byte-identical bodies) ----

/// `std::fill(first, last, const T&)` for a pointer vector: every dword of `[first, last)` becomes `*val` (re-read
/// for each element, `mov esi, [edx]; mov [eax], esi`).
fn fill_ptrs(first: u32, last: u32, val: GPtr<u8>) {
    fill_w(1, first, last, val);
}

/// `std::fill(first, last, const T&)` of `w`-dword elements: every element becomes `*val` (re-read each time).
fn fill_w(w: u32, first: u32, last: u32, val: GPtr<u8>) {
    let mut p = first;
    while p != last {
        copy_elem(w, p, val.addr() as u32);
        p = p.wrapping_add(w * 4);
    }
}

/// `std::copy_backward(first, last, dest_last)` for a pointer vector, the iterator result through `out`: dwords
/// moved from the back (`last` down to `first`); `*out` = the destination start. Returns `out`.
fn copy_backward_ptrs(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> {
    copy_backward_w(1, out, first, last, dest_last)
}

/// `std::copy_backward(first, last, dest_last)` of `w`-dword elements, the iterator result through `out`: elements
/// moved from the back (dwords +0, +4, ... of each); `*out` = the destination start. Returns `out`.
fn copy_backward_w(w: u32, out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> {
    let (mut s, mut d) = (last, dest_last);
    while s != first {
        s = s.wrapping_sub(w * 4);
        d = d.wrapping_sub(w * 4);
        copy_elem(w, d, s);
    }
    wr::<u32>(out, 0, d);
    out
}

/// `std::fill<CPed**>` (0x5EAAA0, cdecl): see `fill_ptrs`.
#[hook(addr = 0x5EAAA0, conv = cdecl, name = "FUN_005eaaa0", expect = "8B 44 24 04 8B 4C 24 08 3B C1",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn fill_p_port(first: u32, last: u32, val: GPtr<u8>) {
    fill_ptrs(first, last, val);
}

/// `std::fill<CTask**>` (0x5EAAF0, cdecl): see `fill_ptrs`.
#[hook(addr = 0x5EAAF0, conv = cdecl, name = "FUN_005eaaf0", expect = "8B 44 24 04 8B 4C 24 08 3B C1",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn fill_t_port(first: u32, last: u32, val: GPtr<u8>) {
    fill_ptrs(first, last, val);
}

/// `std::fill<CPedAttractor**>` (0x5EAB10, cdecl): see `fill_ptrs`.
#[hook(addr = 0x5EAB10, conv = cdecl, name = "FUN_005eab10", expect = "8B 44 24 04 8B 4C 24 08 3B C1",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn fill_a_port(first: u32, last: u32, val: GPtr<u8>) {
    fill_ptrs(first, last, val);
}

/// `std::copy_backward<CPed**>` (0x5EB140, cdecl): see `copy_backward_ptrs`.
#[hook(addr = 0x5EB140, conv = cdecl, name = "FUN_005eb140", expect = "8B 54 24 08 8B 44 24 0C 3B D0",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn copy_backward_p_port(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> {
    copy_backward_ptrs(out, first, last, dest_last)
}

/// `std::copy_backward<CTask**>` (0x5EB1B0, cdecl): see `copy_backward_ptrs`.
#[hook(addr = 0x5EB1B0, conv = cdecl, name = "FUN_005eb1b0", expect = "8B 54 24 08 8B 44 24 0C 3B D0",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn copy_backward_t_port(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> {
    copy_backward_ptrs(out, first, last, dest_last)
}

/// `std::copy_backward<CPedAttractor**>` (0x5EB260, cdecl): see `copy_backward_ptrs`.
#[hook(addr = 0x5EB260, conv = cdecl, name = "FUN_005eb260", expect = "8B 54 24 08 8B 44 24 0C 3B D0",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn copy_backward_a_port(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> {
    copy_backward_ptrs(out, first, last, dest_last)
}

/// `vector<CPed*>::_Ucopy(first, last, dest)` (0x5EB930, thiscall, `ret 0xC`): `uninitialized_copy` 0x5EAE10 (the
/// allocator and the destination are pushed again after the three arguments, unread); its result.
#[hook(addr = 0x5EB930, conv = thiscall, name = "FUN_005eb930", expect = "8B 44 24 0C 8B 54 24 08 50 51",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn ucopy_p_port(_v: GPtr<u8>, first: u32, last: u32, dest: u32) -> u32 {
    copy_p_o(first, last, dest)
}

/// `vector<CTask*>::_Ucopy(first, last, dest)` (0x5EB990, thiscall, `ret 0xC`): `uninitialized_copy` 0x5EAE70.
#[hook(addr = 0x5EB990, conv = thiscall, name = "FUN_005eb990", expect = "8B 44 24 0C 8B 54 24 08 50 51",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn ucopy_t_port(_v: GPtr<u8>, first: u32, last: u32, dest: u32) -> u32 {
    copy_t_o(first, last, dest)
}

/// `vector<CPedAttractor*>::_Ucopy(first, last, dest)` (0x5EBA00, thiscall, `ret 0xC`): `uninitialized_copy`
/// 0x5EAEE0.
#[hook(addr = 0x5EBA00, conv = thiscall, name = "FUN_005eba00", expect = "8B 44 24 0C 8B 54 24 08 50 51",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn ucopy_a_port(_v: GPtr<u8>, first: u32, last: u32, dest: u32) -> u32 {
    copy_a_o(first, last, dest)
}

/// `vector<CPed*>::_Ufill(dest, count, const T&)` (0x5EC070, thiscall, `ret 0xC`): `uninitialized_fill_n` 0x5EB2A0
/// (its result unused), then `dest + count * 4`.
#[hook(addr = 0x5EC070, conv = thiscall, name = "FUN_005ec070", expect = "8B 44 24 0C 56 8B 74 24 0C 57",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn ufill_p_port(_v: GPtr<u8>, dest: u32, n: u32, val: GPtr<u8>) -> u32 {
    fill_n_p_o(dest, n, val);
    dest.wrapping_add(n.wrapping_mul(4))
}

/// `vector<CTask*>::_Ufill(dest, count, const T&)` (0x5EC110, thiscall, `ret 0xC`): `uninitialized_fill_n` 0x5EB310,
/// then `dest + count * 4`.
#[hook(addr = 0x5EC110, conv = thiscall, name = "FUN_005ec110", expect = "8B 44 24 0C 56 8B 74 24 0C 57",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn ufill_t_port(_v: GPtr<u8>, dest: u32, n: u32, val: GPtr<u8>) -> u32 {
    fill_n_t_o(dest, n, val);
    dest.wrapping_add(n.wrapping_mul(4))
}

/// `vector<CPedAttractor*>::_Ufill(dest, count, const T&)` (0x5EC1B0, thiscall, `ret 0xC`): `uninitialized_fill_n`
/// 0x5EB390, then `dest + count * 4`.
#[hook(addr = 0x5EC1B0, conv = thiscall, name = "FUN_005ec1b0", expect = "8B 44 24 0C 56 8B 74 24 0C 57",
       unverified = "frontier2-c1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn ufill_a_port(_v: GPtr<u8>, dest: u32, n: u32, val: GPtr<u8>) -> u32 {
    fill_n_a_o(dest, n, val);
    dest.wrapping_add(n.wrapping_mul(4))
}

// ---- frontier2-a9: `std::vector<CVector>` ----

/// `std::vector<CVector>::_Insert_n(iterator, size_t, const CVector&)` (0x5ED750, thiscall, `ret 0xC`, EBP frame and
/// SEH; the catch funclet 0x5ED92A, an excluded `X-eh-funclet` row, frees the new block and rethrows, so the body is
/// the two ranges 0x5ED750-0x5ED92A and 0x5ED93F-0x5EDA4B): see `insert_n`.
#[hook(addr = 0x5ED750, conv = thiscall, name = "FUN_005ed750", expect = "55 8B EC 6A FF 68 90 DB 83 00",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes",
       note = "SEH frame / catch funclet not modelled (operator new does not throw in the game)")]
pub fn insert_n_v(v: GPtr<u8>, at: u32, n: u32, val: GPtr<u8>) {
    insert_n(&VECTORS, v, at, n, val);
}

/// `uninitialized_fill_n<CVector*>` (0x5EB340, cdecl; callers push two more dwords, unread): see `fill_n_w`.
#[hook(addr = 0x5EB340, conv = cdecl, name = "FUN_005eb340", expect = "8B 44 24 08 85 C0 76 2E",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes")]
pub fn fill_n_v(dest: u32, n: u32, val: GPtr<u8>) -> u32 {
    fill_n_w(3, dest, n, val)
}

/// `uninitialized_copy<CVector*>` (0x5EAEA0, cdecl; two more dwords pushed, unread): see `copy_w`.
#[hook(addr = 0x5EAEA0, conv = cdecl, name = "FUN_005eaea0", expect = "8B 4C 24 04 8B 54 24 08 3B CA",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes")]
pub fn copy_v(first: u32, last: u32, dest: u32) -> u32 {
    copy_w(3, first, last, dest)
}

/// `vector<CVector>::_Ucopy(first, last, dest)` (0x5EB9D0, thiscall, `ret 0xC`): `uninitialized_copy` 0x5EAEA0; its
/// result.
#[hook(addr = 0x5EB9D0, conv = thiscall, name = "FUN_005eb9d0", expect = "8B 44 24 0C 8B 54 24 08 50 51",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes")]
pub fn ucopy_v(_v: GPtr<u8>, first: u32, last: u32, dest: u32) -> u32 {
    copy_v_o(first, last, dest)
}

/// `vector<CVector>::_Ufill(dest, count, const T&)` (0x5EC140, thiscall, `ret 0xC`): `uninitialized_fill_n`
/// 0x5EB340, then `dest + count * 12`.
#[hook(addr = 0x5EC140, conv = thiscall, name = "FUN_005ec140", expect = "8B 44 24 0C 56 8B 74 24 0C 57",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes")]
pub fn ufill_v(_v: GPtr<u8>, dest: u32, n: u32, val: GPtr<u8>) -> u32 {
    vector3_fill_n(dest, n, val);
    dest.wrapping_add(n.wrapping_mul(0xC))
}

/// `std::_Copy_backward_opt<CVector*>` (0x5EACC0, cdecl; a fifth dword pushed, unread): see `copy_backward_w`.
#[hook(addr = 0x5EACC0, conv = cdecl, name = "FUN_005eacc0", expect = "8B 54 24 08 8B 44 24 0C 3B D0",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes")]
pub fn copy_backward_opt_v(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> {
    copy_backward_w(3, out, first, last, dest_last)
}

/// `std::copy_backward<CVector*>` (0x5EB220, cdecl): passes its four arguments (and `dest_last` again) to
/// 0x5EACC0; returns `out`.
#[hook(addr = 0x5EB220, conv = cdecl, name = "FUN_005eb220", expect = "8B 44 24 10 8B 54 24 0C 56",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes")]
pub fn copy_backward_v(out: GPtr<u8>, first: u32, last: u32, dest_last: u32) -> GPtr<u8> {
    copy_backward_opt_v_o(out, first, last, dest_last);
    out
}

/// `std::fill<CVector*>` (0x5EB1E0, cdecl): see `fill_w`.
#[hook(addr = 0x5EB1E0, conv = cdecl, name = "FUN_005eb1e0", expect = "8B 44 24 04 8B 4C 24 08 3B C1",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes")]
pub fn fill_v(first: u32, last: u32, val: GPtr<u8>) {
    fill_w(3, first, last, val);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_INSERT_N_V,
    HOOK_FILL_N_V,
    HOOK_COPY_V,
    HOOK_UCOPY_V,
    HOOK_UFILL_V,
    HOOK_COPY_BACKWARD_OPT_V,
    HOOK_COPY_BACKWARD_V,
    HOOK_FILL_V,
    HOOK_FILL_P_PORT,
    HOOK_FILL_T_PORT,
    HOOK_FILL_A_PORT,
    HOOK_COPY_BACKWARD_P_PORT,
    HOOK_COPY_BACKWARD_T_PORT,
    HOOK_COPY_BACKWARD_A_PORT,
    HOOK_UCOPY_P_PORT,
    HOOK_UCOPY_T_PORT,
    HOOK_UCOPY_A_PORT,
    HOOK_UFILL_P_PORT,
    HOOK_UFILL_T_PORT,
    HOOK_UFILL_A_PORT,
    HOOK_PUSH_BACK_T,
    HOOK_INSERT_N_T,
    HOOK_FILL_N_T,
    HOOK_COPY_T,
    HOOK_PUSH_BACK_A,
    HOOK_PUSH_BACK_P,
    HOOK_INSERT_N_A,
    HOOK_INSERT_N_P,
    HOOK_FILL_N_A,
    HOOK_FILL_N_P,
    HOOK_COPY_A,
    HOOK_COPY_P,
];
