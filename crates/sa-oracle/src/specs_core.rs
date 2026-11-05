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

/// Every CORE spec.
pub fn all() -> Vec<Spec> {
    vec![
        pool_new_spec("CPedPoolNew", "CPedPool::New", 0x5E45E0, 0x7C4),
        pool_new_spec("CVehiclePoolNew", "CVehiclePool::New", 0x6E2A50, 0xA18),
        pool_new_spec("CObjectPoolNew", "CObjectPool::New", 0x5A1C20, 0x19C),
        ped_pool_get_at_ref(),
        object_pool_get_at(),
    ]
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
