//! L1 specs for the CORE pool pilots in `sa-core` (ARCH-1): the `CPool<T>` slot/handle logic of three
//! pool instantiations, with the pool header and byte map in scratch memory (header at SCRATCH,
//! `m_pObjects` = an input, `m_byteMap` = SCRATCH+0x14, the rest of scratch = map bytes). Sizes, first-free values and
//! handles are generated so every byte the original reads lies inside the compared scratch bytes —
//! including the reads *before* the map that the original's missing bounds checks allow (negative
//! slot indices read header bytes). Chunks outside a function's start extent (the 0x405B24
//! protection chunk of CObjectPool::New, the .HOODLUM body of CObjectPool::GetAt) come from
//! `catalog/extents.csv` through the allow-list.
//!
//! `CGeneral::GetRandomNumberInRange` int/float are covered by MATH-2's specs (`math_specs`,
//! `rand()` stubbed as an input); sa-core's ports use sa-math's implementation of them.

use crate::call::*;
use crate::generate::Rng;
use crate::specs::*;
use sa_core::pool::{self, ByteMap};
use std::sync::Arc;

/// Scratch offset of the byte map; the pool header is at 0.
pub const MAP_OFF: usize = 0x14;
pub const MAP_LEN: usize = SCRATCH_LEN - MAP_OFF;

// ------------------------------------------------------------------------------- pools -----------

/// The scratch block seen as the pool's byte map (index 0 at `MAP_OFF`).
struct ScratchMap([u8; SCRATCH_LEN]);

impl ByteMap for ScratchMap {
    fn get(&self, i: i32) -> u8 {
        let o = MAP_OFF as i32 + i;
        assert!(
            (0..SCRATCH_LEN as i32).contains(&o),
            "map index {i} outside scratch"
        );
        self.0[o as usize]
    }
    fn set(&mut self, i: i32, v: u8) {
        let o = MAP_OFF as i32 + i;
        assert!(
            (0..SCRATCH_LEN as i32).contains(&o),
            "map index {i} outside scratch"
        );
        self.0[o as usize] = v
    }
}

/// Inputs: [objects, size, first_free, flags word (+0x10..+0x14), map words x11, arg].
const POOL_INPUTS: usize = 4 + MAP_LEN / 4 + 1;

fn pool_scratch(i: &Inputs) -> [u8; SCRATCH_LEN] {
    let mut s = [0u8; SCRATCH_LEN];
    let mut put = |o: usize, v: u32| s[o..o + 4].copy_from_slice(&v.to_le_bytes());
    put(0x00, i[0]);
    put(0x04, SCRATCH + MAP_OFF as u32);
    put(0x08, i[1]);
    put(0x0C, i[2]);
    put(0x10, i[3]);
    for k in 0..MAP_LEN / 4 {
        put(MAP_OFF + 4 * k, i[4 + k]);
    }
    s
}

/// Byte-map contents: slots below `size` are free (bit 7) with a chosen probability and carry
/// arbitrary reuse counters; bytes from `size` on are arbitrary.
fn gen_map(r: &mut Rng, size: usize) -> Vec<u8> {
    let free_pct = r.pick(&[0, 0, 3, 30, 70, 100]);
    (0..MAP_LEN)
        .map(|k| {
            let counter = if r.chance(5) {
                r.pick(&[0x00, 0x7F])
            } else {
                (r.next_u32() & 0x7F) as u8
            };
            if k >= size {
                r.next_u32() as u8
            } else if r.below(100) < free_pct {
                counter | 0x80
            } else {
                counter
            }
        })
        .collect()
}

fn pack(objects: u32, size: i32, first_free: i32, flags: u32, map: &[u8], arg: u32) -> Inputs {
    let mut v = vec![objects, size as u32, first_free as u32, flags];
    for c in map.chunks(4) {
        v.push(u32::from_le_bytes([c[0], c[1], c[2], c[3]]));
    }
    v.push(arg);
    debug_assert_eq!(v.len(), POOL_INPUTS);
    v
}

fn gen_objects(r: &mut Rng) -> u32 {
    match r.below(4) {
        0 => r.pick(&[0, 0x1000_0000, 0xFFFF_F000, 0x8000_0000]),
        _ => r.next_u32() & !3,
    }
}

fn gen_pool_new(r: &mut Rng) -> Inputs {
    let size = if r.chance(5) {
        0
    } else {
        1 + r.below(MAP_LEN as u32) as i32
    };
    // `m_nFirstFree` is -1 on a fresh pool and a slot index afterwards
    let ff = if size == 0 || r.chance(10) {
        -1
    } else {
        r.below(size as u32) as i32
    };
    let mut map = gen_map(r, size as usize);
    if size == 0 {
        // No bounds check: an empty pool tests map[0] and takes it when free. With map[0] used the
        // original would scan on past the map until the index wraps; that state never occurs.
        map[0] |= 0x80;
    }
    pack(gen_objects(r), size, ff, r.next_u32(), &map, 0)
}

fn pool_new_edges() -> Vec<Inputs> {
    let mut e = Vec::new();
    for size in [0i32, 1, 2, 3, 44] {
        for ff in [-1i32, 0, size - 1] {
            if ff >= size.max(0) && ff != -1 {
                continue;
            }
            for fill in [0x00u8, 0x7F, 0x80, 0xFF] {
                if size == 0 && fill < 0x80 {
                    continue; // see gen_pool_new
                }
                let mut map = vec![fill; MAP_LEN];
                e.push(pack(0x2000_0000, size, ff, 0, &map, 0));
                // a single free slot at each end
                if size > 0 {
                    map.fill(0x05);
                    map[0] = 0x85;
                    e.push(pack(0x2000_0000, size, ff, 0, &map, 0));
                    map[0] = 0x05;
                    map[(size - 1) as usize] = 0xFF;
                    e.push(pack(0x2000_0000, size, ff, 0, &map, 0));
                }
            }
        }
    }
    e
}

fn pool_new_call(addr: u32, i: &Inputs) -> Call {
    let mut c = Call::new(addr, Conv::Thiscall, &[Arg::Ptr(SCRATCH)]);
    c.scratch = pool_scratch(i);
    c
}

fn pool_new_spec(key: &'static str, name: &'static str, addr: u32, stride: u32) -> Spec {
    Spec {
        key,
        name,
        addr,
        ret: RetKind::Eax,
        st0_exact: false,
        route: Route::L1,
        callees: &[],
        // worst case: two passes over a full map (~7 instructions per slot)
        budget: 50_000,
        floats: 0,
        generate: Arc::new(gen_pool_new),
        edges: Arc::new(pool_new_edges),
        call: Arc::new(move |i| pool_new_call(addr, i)),
        rust: Arc::new(move |i, s| {
            let mut m = ScratchMap(*s);
            let (slot, ff) = pool::new_slot(&mut m, i[1] as i32, i[2] as i32);
            let mut out = m.0;
            out[0x0C..0x10].copy_from_slice(&ff.to_le_bytes());
            let eax = slot.map_or(0, |k| i[0].wrapping_add((k as u32).wrapping_mul(stride)));
            Expect::Returns {
                ret: ExpRet::Eax(eax),
                scratch: out,
            }
        }),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
    }
}

/// Indices whose byte the original reads stay inside scratch: -MAP_OFF..MAP_LEN.
fn gen_index(r: &mut Rng) -> i32 {
    if r.chance(10) {
        r.below(MAP_OFF as u32) as i32 - MAP_OFF as i32
    } else {
        r.below(MAP_LEN as u32) as i32
    }
}

fn gen_pool_lookup(r: &mut Rng, by_ref: bool) -> Inputs {
    let size = 1 + r.below(MAP_LEN as u32) as i32;
    let map = gen_map(r, MAP_LEN);
    let mut v = pack(
        gen_objects(r),
        size,
        r.below(size as u32) as i32,
        r.next_u32(),
        &map,
        0,
    );
    let idx = gen_index(r);
    let arg = if by_ref {
        let byte = pool_scratch(&v)[(MAP_OFF as i32 + idx) as usize];
        let low = if r.chance(60) { byte } else { r.next_u32() as u8 };
        ((idx as u32) << 8) | low as u32
    } else {
        idx as u32
    };
    *v.last_mut().unwrap() = arg;
    v
}

fn pool_lookup_edges(by_ref: bool) -> Vec<Inputs> {
    let mut e = Vec::new();
    for fill in [0x00u8, 0x01, 0x7F, 0x80, 0x81, 0xFF] {
        let map = vec![fill; MAP_LEN];
        for idx in [-(MAP_OFF as i32), -1, 0, 1, MAP_LEN as i32 - 1] {
            let args: Vec<u32> = if by_ref {
                vec![
                    ((idx as u32) << 8) | fill as u32,
                    ((idx as u32) << 8) | (fill ^ 1) as u32,
                ]
            } else {
                vec![idx as u32]
            };
            for a in args {
                e.push(pack(0x2000_0000, 44, 0, 0, &map, a));
            }
        }
    }
    e
}

pub fn ped_pool_get_at_ref() -> Spec {
    Spec {
        key: "CPedPoolGetAtRef",
        name: "CPedPool::GetAtRef",
        addr: 0x404910,
        ret: RetKind::Eax,
        st0_exact: false,
        route: Route::L1,
        callees: &[],
        budget: 1_000,
        floats: 0,
        generate: Arc::new(|r| gen_pool_lookup(r, true)),
        edges: Arc::new(|| pool_lookup_edges(true)),
        call: Arc::new(|i| {
            let mut c = Call::new(
                0x404910,
                Conv::Thiscall,
                &[Arg::Ptr(SCRATCH), Arg::I32(i[POOL_INPUTS - 1])],
            );
            c.scratch = pool_scratch(i);
            c
        }),
        rust: Arc::new(|i, s| {
            let m = ScratchMap(*s);
            let eax = pool::at_ref_slot(&m, i[POOL_INPUTS - 1] as i32)
                .map_or(0, |k| i[0].wrapping_add((k as u32).wrapping_mul(0x7C4)));
            Expect::Returns {
                ret: ExpRet::Eax(eax),
                scratch: *s,
            }
        }),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
    }
}

pub fn object_pool_get_at() -> Spec {
    Spec {
        key: "CObjectPoolGetAt",
        name: "CObjectPool::GetAt",
        addr: 0x404870,
        ret: RetKind::Eax,
        st0_exact: false,
        route: Route::L1,
        callees: &[],
        budget: 1_000,
        floats: 0,
        generate: Arc::new(|r| gen_pool_lookup(r, false)),
        edges: Arc::new(|| pool_lookup_edges(false)),
        call: Arc::new(|i| {
            let mut c = Call::new(
                0x404870,
                Conv::Thiscall,
                &[Arg::Ptr(SCRATCH), Arg::I32(i[POOL_INPUTS - 1])],
            );
            c.scratch = pool_scratch(i);
            c
        }),
        rust: Arc::new(|i, s| {
            let m = ScratchMap(*s);
            let eax = pool::at_slot(&m, i[POOL_INPUTS - 1] as i32)
                .map_or(0, |k| i[0].wrapping_add((k as u32).wrapping_mul(0x19C)));
            Expect::Returns {
                ret: ExpRet::Eax(eax),
                scratch: *s,
            }
        }),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
    }
}

// ------------------------------------------------------------------- PORT-CORE pool specs ------

/// `mov ecx, imm32` style helpers for the byte patches below.
fn le(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

/// The pool pointer global `cell` reads as SCRATCH (the pool header) for the specs of functions that
/// load `ms_p*Pool` themselves (operator new/delete).
fn pool_global_patch(cell: u32) -> Vec<(u32, Vec<u8>)> {
    vec![(cell, le(SCRATCH).to_vec())]
}

/// `operator new` stand-in patched over 0x821195 (or 0x82119A): records the last two size arguments
/// (`[S+0x4E8]` = last, `[S+0x4EC]` = previous) and returns a two-entry queue (`[S+0x4F0]`, then
/// `[S+0x4F4]`; the head is refilled from the second entry).
fn alloc_stub(at: u32) -> Vec<(u32, Vec<u8>)> {
    let s = |o: u32| le(SCRATCH + o);
    let mut p = vec![0x8B, 0x0D];
    p.extend(s(0x4E8)); // mov ecx, [S+4E8]
    p.extend([0x89, 0x0D]);
    p.extend(s(0x4EC)); // mov [S+4EC], ecx
    p.extend([0x8B, 0x4C, 0x24, 0x04]); // mov ecx, [esp+4]
    p.extend([0x89, 0x0D]);
    p.extend(s(0x4E8)); // mov [S+4E8], ecx
    p.push(0xA1);
    p.extend(s(0x4F0)); // mov eax, [S+4F0]
    p.extend([0x8B, 0x0D]);
    p.extend(s(0x4F4)); // mov ecx, [S+4F4]
    p.extend([0x89, 0x0D]);
    p.extend(s(0x4F0)); // mov [S+4F0], ecx
    p.push(0xC3);
    vec![(at, p)]
}

/// Scratch layout of the constructor specs: header at 0, map anywhere in 0x20..0x4E0, allocator
/// cells at 0x4E8..0x4F8.
const CTOR_MAP_END: usize = 0x4E0;

/// Inputs: [size, name, q0 (objects), map offset, initial scratch words x (0x500 / 4)].
fn gen_ctor(r: &mut Rng) -> Inputs {
    let map_off = 0x20 + r.below((CTOR_MAP_END - 0x20) as u32) as usize;
    let room = (CTOR_MAP_END - map_off) as u32;
    let size = match r.below(10) {
        0 => r.pick(&[0u32, (-1i32) as u32, (-3i32) as u32, 1]),
        1 => room,
        _ => r.below(room + 1),
    };
    let mut v = vec![size, r.next_u32(), gen_objects(r), map_off as u32];
    for _ in 0..SCRATCH_LEN / 4 {
        v.push(r.next_u32());
    }
    v
}

fn ctor_edges() -> Vec<Inputs> {
    let mut e = Vec::new();
    for (size, off) in [(0u32, 0x20usize), (1, 0x20), (2, 0x4DE), ((-1i32) as u32, 0x100), (0x4C0, 0x20)] {
        for fill in [0u32, 0xFFFF_FFFF, 0x7F7F_7F7F] {
            let mut v = vec![size, 0, 0x2000_0000, off as u32];
            v.extend(std::iter::repeat_n(fill, SCRATCH_LEN / 4));
            e.push(v);
        }
    }
    e
}

fn ctor_scratch(i: &Inputs) -> [u8; SCRATCH_LEN] {
    let mut s = [0u8; SCRATCH_LEN];
    for k in 0..SCRATCH_LEN / 4 {
        s[4 * k..4 * k + 4].copy_from_slice(&i[4 + k].to_le_bytes());
    }
    // the allocator queue: objects first, then the map
    s[0x4F0..0x4F4].copy_from_slice(&i[2].to_le_bytes());
    s[0x4F4..0x4F8].copy_from_slice(&(SCRATCH + i[3]).to_le_bytes());
    s
}

/// A byte map at an arbitrary scratch offset.
struct MapAt([u8; SCRATCH_LEN], usize);

impl ByteMap for MapAt {
    fn get(&self, i: i32) -> u8 {
        let o = self.1 as i32 + i;
        assert!((0..SCRATCH_LEN as i32).contains(&o), "map index {i} outside scratch");
        self.0[o as usize]
    }
    fn set(&mut self, i: i32, v: u8) {
        let o = self.1 as i32 + i;
        assert!((0..SCRATCH_LEN as i32).contains(&o), "map index {i} outside scratch");
        self.0[o as usize] = v
    }
}

fn ctor_spec(key: &'static str, name: &'static str, addr: u32, stride: u32, alloc: u32) -> Spec {
    Spec {
        key,
        name,
        addr,
        ret: RetKind::Eax,
        st0_exact: false,
        route: Route::L1,
        callees: &[0x821195, 0x82119A],
        budget: 50_000,
        floats: 0,
        generate: Arc::new(gen_ctor),
        edges: Arc::new(ctor_edges),
        call: Arc::new(move |i| {
            let mut c = Call::new(
                addr,
                Conv::Thiscall,
                &[Arg::Ptr(SCRATCH), Arg::I32(i[0]), Arg::I32(i[1])],
            );
            c.scratch = ctor_scratch(i);
            c
        }),
        rust: Arc::new(move |i, s| {
            let size = i[0] as i32;
            let mut out = *s;
            let mut put = |o: usize, v: u32| out[o..o + 4].copy_from_slice(&v.to_le_bytes());
            // allocator cells after the two calls
            put(0x4EC, (size as u32).wrapping_mul(stride));
            put(0x4E8, size as u32);
            put(0x4F0, SCRATCH + i[3]);
            // header
            put(0x00, i[2]);
            put(0x04, SCRATCH + i[3]);
            put(0x08, size as u32);
            put(0x0C, u32::MAX);
            out[0x10] = 1;
            let mut m = MapAt(out, i[3] as usize);
            sa_core::pools::init_map(&mut m, size);
            Expect::Returns {
                ret: ExpRet::Eax(SCRATCH),
                scratch: m.0,
            }
        }),
        ignore: vec![],
        patches: alloc_stub(alloc),
        reads: vec![],
    }
}

/// `operator new` of a pooled type: loads the pool global (patched to SCRATCH) and runs `New`.
fn op_new_spec(key: &'static str, name: &'static str, addr: u32, new_addr: &'static [u32], cell: u32, stride: u32) -> Spec {
    let mut s = pool_new_spec(key, name, addr, stride);
    s.callees = new_addr;
    s.call = Arc::new(move |i| {
        let mut c = Call::new(addr, Conv::Cdecl, &[Arg::I32(i[POOL_INPUTS - 1])]);
        c.scratch = pool_scratch(i);
        c
    });
    s.patches = pool_global_patch(cell);
    s.reads = vec![(cell, cell + 4)];
    s
}

/// `operator delete` of a pooled type (`Delete` inlined): the argument is `objects + diff`.
fn gen_delete(r: &mut Rng, stride: u32) -> Inputs {
    let mut v = gen_pool_lookup(r, false);
    let idx = *v.last().unwrap() as i32;
    let off = if r.chance(70) { 0 } else { r.below(stride) as i32 };
    let diff = idx.wrapping_mul(stride as i32).wrapping_add(if idx < 0 { -off } else { off });
    *v.last_mut().unwrap() = v[0].wrapping_add(diff as u32);
    v
}

fn op_delete_spec(key: &'static str, name: &'static str, addr: u32, cell: u32, stride: u32) -> Spec {
    Spec {
        key,
        name,
        addr,
        ret: RetKind::Void,
        st0_exact: false,
        route: Route::L1,
        callees: &[],
        budget: 1_000,
        floats: 0,
        generate: Arc::new(move |r| gen_delete(r, stride)),
        edges: Arc::new(move || {
            pool_lookup_edges(false)
                .into_iter()
                .map(|mut v| {
                    let idx = *v.last().unwrap() as i32;
                    *v.last_mut().unwrap() = v[0].wrapping_add(idx.wrapping_mul(stride as i32) as u32);
                    v
                })
                .collect()
        }),
        call: Arc::new(move |i| {
            let mut c = Call::new(addr, Conv::Cdecl, &[Arg::Ptr(i[POOL_INPUTS - 1])]);
            c.scratch = pool_scratch(i);
            c
        }),
        rust: Arc::new(move |i, s| {
            let mut m = ScratchMap(*s);
            let idx = (i[POOL_INPUTS - 1].wrapping_sub(i[0]) as i32) / stride as i32;
            // the header field, read through the map view (index -MAP_OFF + 0x0C)
            let ff_of = |mm: &dyn ByteMap| {
                let b = |k: i32| mm.get(k - MAP_OFF as i32) as u32;
                (b(0x0C) | b(0x0D) << 8 | b(0x0E) << 16 | b(0x0F) << 24) as i32
            };
            let w = sa_core::pools::delete_at(&mut m, idx, ff_of);
            let mut out = m.0;
            if let Some(ff) = w {
                out[0x0C..0x10].copy_from_slice(&ff.to_le_bytes());
            }
            Expect::Returns {
                ret: ExpRet::Void,
                scratch: out,
            }
        }),
        ignore: vec![],
        patches: pool_global_patch(cell),
        reads: vec![(cell, cell + 4)],
    }
}

/// `CPools::CheckBuildingAtomics` (0x550170): returns -1.
fn check_building_atomics() -> Spec {
    Spec {
        key: "CPoolsCheckBuildingAtomics",
        name: "CPools::CheckBuildingAtomics",
        addr: 0x550170,
        ret: RetKind::Eax,
        st0_exact: false,
        route: Route::L1,
        callees: &[],
        budget: 100,
        floats: 0,
        generate: Arc::new(|r| vec![r.next_u32()]),
        edges: Arc::new(|| vec![vec![0]]),
        call: Arc::new(|_| Call::new(0x550170, Conv::Cdecl, &[])),
        rust: Arc::new(|_, s| Expect::Returns {
            ret: ExpRet::Eax(sa_core::pools::check_building_atomics() as u32),
            scratch: *s,
        }),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
    }
}

/// `CReferences::ListSize` (0x5719E0): a chain of 8-byte nodes in scratch (random order, other
/// bytes random), the head as the argument (null for an empty list).
fn gen_list(r: &mut Rng) -> Inputs {
    let slots = SCRATCH_LEN / 8;
    let n = if r.chance(10) { 0 } else { 1 + r.below(slots as u32) as usize };
    let mut order: Vec<usize> = (0..slots).collect();
    for k in (1..slots).rev() {
        order.swap(k, r.below(k as u32 + 1) as usize);
    }
    let mut words: Vec<u32> = (0..SCRATCH_LEN / 4).map(|_| r.next_u32()).collect();
    for k in 0..n {
        let next = if k + 1 < n { SCRATCH + 8 * order[k + 1] as u32 } else { 0 };
        words[2 * order[k]] = next;
    }
    words.push(if n == 0 { 0 } else { SCRATCH + 8 * order[0] as u32 });
    words
}

fn list_size() -> Spec {
    Spec {
        key: "CReferencesListSize",
        name: "CReferences::ListSize",
        addr: 0x5719E0,
        ret: RetKind::Eax,
        st0_exact: false,
        route: Route::L1,
        callees: &[],
        budget: 10_000,
        floats: 0,
        generate: Arc::new(gen_list),
        edges: Arc::new(|| {
            let mut v = vec![0u32; SCRATCH_LEN / 4 + 1];
            let mut e = vec![v.clone()];
            v[SCRATCH_LEN / 4] = SCRATCH; // one node pointing at null
            e.push(v);
            e
        }),
        call: Arc::new(|i| {
            let mut c = Call::new(0x5719E0, Conv::Cdecl, &[Arg::Ptr(i[SCRATCH_LEN / 4])]);
            for (k, w) in i[..SCRATCH_LEN / 4].iter().enumerate() {
                c.scratch[4 * k..4 * k + 4].copy_from_slice(&w.to_le_bytes());
            }
            c
        }),
        rust: Arc::new(|i, s| {
            let rd = |a: u32| {
                let o = (a - SCRATCH) as usize;
                u32::from_le_bytes(s[o..o + 4].try_into().unwrap())
            };
            let n = sa_core::references::count_links(i[SCRATCH_LEN / 4], |a| a == 0, rd);
            Expect::Returns {
                ret: ExpRet::Eax(n),
                scratch: *s,
            }
        }),
        ignore: vec![],
        patches: vec![],
        reads: vec![],
    }
}

/// CQuadTreeNode: the node region (rect, list head, four children, level) and the query region.
fn qt_node() -> crate::mspec::Region {
    use crate::mspec::{Kind, Region, W};
    let mut w = vec![W::F; 4];
    w.extend(std::iter::repeat_n(W::R, 6));
    Region { words: w, kind: Kind::Any }
}

/// A quarter of the generated levels are 0 (leaf).
fn qt_fix(i: &mut Inputs, l: &crate::mspec::Layout) {
    let slot = l.word_slot[0][9].unwrap();
    if i[slot].is_multiple_of(4) {
        i[slot] = 0;
    }
}

fn qt_rect(c: &crate::mspec::Ctx, r: usize) -> [f32; 4] {
    [c.fl(r, 0), c.fl(r, 1), c.fl(r, 2), c.fl(r, 3)]
}

fn gen_sector(r: &mut Rng) -> u32 {
    if r.chance(90) { r.below(4) } else { r.next_u32() }
}

fn quad_tree_specs() -> Vec<Spec> {
    use crate::call::Conv::Thiscall;
    use crate::mspec::{b, floats, vec2, A};
    use crate::specs::RetKind::{Al, Eax};
    use sa_core::quadtree as q;
    vec![
        b("CQuadTreeNodeFindSectorRect", "CQuadTreeNode<T>::FindSector-rect", 0x5525A0, Thiscall, Eax)
            .regions(vec![qt_node(), floats(4)])
            .args(vec![A::P(0), A::P(1)])
            .fix(qt_fix)
            .rust(|c| ExpRet::Eax(q::find_sector_rect_of(qt_rect(c, 0), c.u(0, 9), qt_rect(c, 1)) as u32)),
        b("CQuadTreeNodeFindSectorVec", "CQuadTreeNode<T>::FindSector-vec", 0x552640, Thiscall, Eax)
            .regions(vec![qt_node(), vec2()])
            .args(vec![A::P(0), A::P(1)])
            .fix(qt_fix)
            .rust(|c| {
                ExpRet::Eax(q::find_sector_vec_of(qt_rect(c, 0), c.u(0, 9), [c.fl(1, 0), c.fl(1, 1)]) as u32)
            }),
        b("CQuadTreeNodeInSector", "CQuadTreeNode<T>::InSector", 0x5526A0, Thiscall, Al)
            .regions(vec![qt_node(), floats(4)])
            .args(vec![A::P(0), A::P(1), A::I(gen_sector)])
            .fix(qt_fix)
            .rust(|c| {
                ExpRet::Al(q::in_sector_of(qt_rect(c, 0), c.u(0, 9), qt_rect(c, 1), c.i(2) as i32) as u8)
            }),
    ]
}

/// Every PORT-CORE pool, quad-tree and references spec.
pub fn port_core() -> Vec<Spec> {
    let mut v = vec![
        pool_new_spec("CBuildingPoolNew", "CBuildingPool::New", 0x403FA0, 0x38),
        pool_new_spec("CColModelPoolNew", "CColModelPool::New", 0x40FB80, 0x30),
        pool_new_spec("CPointRoutePoolNew", "CPointRoutePool::New", 0x41B5B0, 0x64),
        pool_new_spec("CPatrolRoutePoolNew", "CPatrolRoutePool::New", 0x41B660, 0x1A4),
        pool_new_spec("CNodeRoutePoolNew", "CNodeRoutePool::New", 0x41B710, 0x24),
        pool_new_spec("CEntryInfoNodePoolNew", "CEntryInfoNodePool::GetAt", 0x536D10, 0x14),
        pool_new_spec("CPtrNodeSingleLinkPoolNew", "CPtrNodeSingleLinkPool::New", 0x552240, 0x08),
        pool_new_spec("CPtrNodeDoubleLinkPoolNew", "CPtrNodeDoubleLinkPool::New", 0x5522E0, 0x0C),
        pool_new_spec("CPedIntelligencePoolNew", "CPedIntelligencePool::New", 0x605EC0, 0x294),
        op_new_spec("CNodeRouteOperatorNew", "CNodeRoute::operator new", 0x41B860, &[0x41B710], 0xB744B8, 0x24),
        op_new_spec("CEntryInfoNodeOperatorNew", "CEntryInfoNode::operator new", 0x536DC0, &[0x536D10], 0xB7448C, 0x14),
        op_delete_spec("CNodeRouteOperatorDelete", "CNodeRoute::operator delete", 0x41B870, 0xB744B8, 0x24),
        op_delete_spec("CEntryInfoNodeOperatorDelete", "CEntryInfoNode::operator delete", 0x536DD0, 0xB7448C, 0x14),
        // the ARCH-1 pilots that had host tests only (same inlined Delete / New jump)
        op_new_spec("CPedOperatorNew", "CPed::operator new-anon", 0x5E4720, &[0x5E45E0], 0xB74490, 0x7C4),
        op_delete_spec("CPedOperatorDelete", "CPed::operator delete-anon", 0x5E4760, 0xB74490, 0x7C4),
        check_building_atomics(),
        list_size(),
    ];
    let ctors: [(&'static str, &'static str, u32, u32); 19] = [
        ("CPoolCtor550180", "FUN_00550180", 0x550180, 0x08),
        ("CPoolCtor550250", "FUN_00550250", 0x550250, 0x0C),
        ("CPoolCtor550320", "FUN_00550320", 0x550320, 0x14),
        ("CPoolCtor5503F0", "FUN_005503f0", 0x5503F0, 0x7C4),
        ("CPoolCtor5504C0", "FUN_005504c0", 0x5504C0, 0xA18),
        ("CPoolCtor550570", "FUN_00550570", 0x550570, 0x38),
        ("CPoolCtor550640", "FUN_00550640", 0x550640, 0x19C),
        ("CPoolCtor5506F0", "FUN_005506f0", 0x5506F0, 0x38),
        ("CPoolCtor5507C0", "FUN_005507c0", 0x5507C0, 0x30),
        ("CPoolCtor550890", "FUN_00550890", 0x550890, 0x80),
        ("CPoolCtor550960", "FUN_00550960", 0x550960, 0x44),
        ("CPoolCtor550A30", "FUN_00550a30", 0x550A30, 0x64),
        ("CPoolCtor550B00", "FUN_00550b00", 0x550B00, 0x1A4),
        ("CPoolCtor550BD0", "FUN_00550bd0", 0x550BD0, 0x24),
        ("CPoolCtor550CA0", "FUN_00550ca0", 0x550CA0, 0x20),
        ("CPoolCtor550D70", "FUN_00550d70", 0x550D70, 0x294),
        ("CPoolCtor550E40", "FUN_00550e40", 0x550E40, 0xC4),
        ("CPoolCtor552AF0", "FUN_00552af0", 0x552AF0, 0x28),
        ("CIplDefPoolConstructor", "CIplDefPool::Constructor", 0x405900, 0x34),
    ];
    for (key, name, addr, stride) in ctors {
        let alloc = if addr == 0x552AF0 { 0x82119A } else { 0x821195 };
        v.push(ctor_spec(key, name, addr, stride, alloc));
    }
    v.extend(quad_tree_specs());
    v
}

/// Every CORE spec.
pub fn all() -> Vec<Spec> {
    vec![
        pool_new_spec("CPedPoolNew", "CPedPool::New", 0x5E45E0, 0x7C4),
        pool_new_spec("CVehiclePoolNew", "CVehiclePool::New", 0x6E2A50, 0xA18),
        pool_new_spec("CObjectPoolNew", "CObjectPool::New", 0x5A1C20, 0x19C),
        ped_pool_get_at_ref(),
        object_pool_get_at(),
    ]
    .into_iter()
    .chain(port_core())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{self, Backend, Cfg};

    fn quick() -> Cfg {
        Cfg {
            cases: 20_000,
            backend: Backend::Unicorn,
            threads: 4,
            fixtures: false,
            keep: 1,
            shrink: false,
            snan_native: false,
            ..Cfg::default()
        }
    }

    /// The specs discriminate: gta-reversed's pool search (from `m_nFirstFree`, not `m_nFirstFree + 1`)
    /// is caught. Skipped without the exe.
    #[test]
    fn deviating_ports_are_caught() {
        let Some(img) = crate::image_cached() else { return };
        // gta-reversed's FindFreeSlot starts at m_nFirstFree, not m_nFirstFree + 1
        let mut s = all().remove(0);
        s.rust = Arc::new(|i, sc| {
            let mut m = ScratchMap(*sc);
            let (slot, ff) = pool::new_slot(&mut m, i[1] as i32, (i[2] as i32).wrapping_sub(1));
            let mut out = m.0;
            out[0x0C..0x10].copy_from_slice(&ff.to_le_bytes());
            let eax = slot.map_or(0, |k| i[0].wrapping_add((k as u32).wrapping_mul(0x7C4)));
            Expect::Returns {
                ret: ExpRet::Eax(eax),
                scratch: out,
            }
        });
        assert!(runner::run(&s, &img, &quick()).unwrap().mismatches > 0);
    }

    /// The generators keep every byte the original reads inside the compared scratch block (the Rust
    /// model asserts it; run it over many generated cases).
    #[test]
    fn generated_pool_cases_stay_in_scratch() {
        for spec in all()
            .into_iter()
            .filter(|s| s.addr != 0x407180 && s.addr != 0x41BD90)
        {
            for k in 0..20_000u64 {
                let i = (spec.generate)(&mut Rng::for_case(7, k));
                let c = (spec.call)(&i);
                let _ = (spec.rust)(&i, &c.scratch);
            }
            for i in (spec.edges)() {
                let c = (spec.call)(&i);
                let _ = (spec.rust)(&i, &c.scratch);
            }
        }
    }
}
