//! Deterministic allocator for determinism mode (INF-4).
//!
//! Under Wine the CRT heap of gta_sa.exe hands out different addresses on every run (measured: pool
//! storage at 0x8bc0020 in one run and 0x8d70020 in the next; docs/re/determinism.md §5), and the
//! game feeds heap addresses into its logic (`CEntity::CreateEffects` seeds `srand` with the entity
//! pointer). In determinism mode every allocation the main thread makes through the CRT is served
//! from this arena instead, which lives in a region reserved at a fixed address, so the same sequence
//! of requests always yields the same addresses.
//!
//! The arena only computes addresses; metadata lives outside the managed memory (side tables), so
//! the algorithm is plain Rust and host-tested. Policy:
//! - every block is 16-byte aligned and its capacity is the request rounded up to 16 (0 → 16);
//! - capacities up to [`SMALL_MAX`] use one LIFO free list per capacity (no coalescing);
//! - larger blocks use best fit (smallest free run that fits, lowest address on ties) with
//!   coalescing of adjacent free runs; the unused tail of a run is returned to the free set;
//! - otherwise memory comes from a bump pointer.

use std::collections::{BTreeMap, BTreeSet};

pub const ALIGN: u32 = 16;
pub const SMALL_MAX: u32 = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// Bytes requested (what `_msize` reports).
    pub size: u32,
    /// Bytes reserved for the block (multiple of 16).
    pub cap: u32,
}

#[derive(Debug, Clone)]
pub struct Arena {
    pub base: u32,
    pub end: u32,
    bump: u32,
    live: BTreeMap<u32, Block>,
    small: Vec<Vec<u32>>,
    /// Large free runs: (len, addr) for best fit and addr → len for coalescing.
    free_by_len: BTreeSet<(u32, u32)>,
    free_by_addr: BTreeMap<u32, u32>,
    pub allocs: u64,
    pub frees: u64,
    pub high_water: u32,
}

fn cap_of(size: u32) -> Option<u32> {
    let s = size.max(1);
    s.checked_add(ALIGN - 1).map(|v| v & !(ALIGN - 1))
}

impl Arena {
    pub fn new(base: u32, len: u32) -> Arena {
        let base = (base + ALIGN - 1) & !(ALIGN - 1);
        Arena {
            base,
            end: base.saturating_add(len & !(ALIGN - 1)),
            bump: base,
            live: BTreeMap::new(),
            small: vec![Vec::new(); (SMALL_MAX / ALIGN + 1) as usize],
            free_by_len: BTreeSet::new(),
            free_by_addr: BTreeMap::new(),
            allocs: 0,
            frees: 0,
            high_water: 0,
        }
    }

    pub fn contains(&self, p: u32) -> bool {
        (self.base..self.end).contains(&p)
    }

    pub fn block(&self, p: u32) -> Option<Block> {
        self.live.get(&p).copied()
    }

    pub fn live_count(&self) -> usize {
        self.live.len()
    }

    fn insert_free_run(&mut self, mut addr: u32, mut len: u32) {
        // coalesce with the previous and the next run
        if let Some((&pa, &pl)) = self.free_by_addr.range(..addr).next_back()
            && pa + pl == addr
        {
            self.free_by_addr.remove(&pa);
            self.free_by_len.remove(&(pl, pa));
            addr = pa;
            len += pl;
        }
        if let Some(&nl) = self.free_by_addr.get(&(addr + len)) {
            self.free_by_addr.remove(&(addr + len));
            self.free_by_len.remove(&(nl, addr + len));
            len += nl;
        }
        if addr + len == self.bump {
            self.bump = addr; // give the tail back to the bump region
        } else {
            self.free_by_addr.insert(addr, len);
            self.free_by_len.insert((len, addr));
        }
    }

    /// Returns the block address, or None when the arena is exhausted.
    pub fn alloc(&mut self, size: u32) -> Option<u32> {
        let cap = cap_of(size)?;
        let addr = if cap <= SMALL_MAX {
            self.small[(cap / ALIGN) as usize].pop()
        } else {
            None
        };
        let addr = match addr {
            Some(a) => a,
            None => {
                let fit = if cap > SMALL_MAX {
                    self.free_by_len.range((cap, 0)..).next().copied()
                } else {
                    None
                };
                match fit {
                    Some((len, a)) => {
                        self.free_by_len.remove(&(len, a));
                        self.free_by_addr.remove(&a);
                        if len > cap {
                            self.insert_free_run(a + cap, len - cap);
                        }
                        a
                    }
                    None => {
                        let a = self.bump;
                        let e = a.checked_add(cap).filter(|&e| e <= self.end)?;
                        self.bump = e;
                        self.high_water = self.high_water.max(e - self.base);
                        a
                    }
                }
            }
        };
        self.live.insert(addr, Block { size, cap });
        self.allocs += 1;
        Some(addr)
    }

    /// Frees a block; false if `p` is not a live block of this arena.
    pub fn free(&mut self, p: u32) -> bool {
        let Some(b) = self.live.remove(&p) else {
            return false;
        };
        self.frees += 1;
        if b.cap <= SMALL_MAX {
            self.small[(b.cap / ALIGN) as usize].push(p);
        } else {
            self.insert_free_run(p, b.cap);
        }
        true
    }

    /// Resizes in place when the capacity allows; returns false if the block must move.
    pub fn resize_in_place(&mut self, p: u32, size: u32) -> bool {
        match (self.live.get_mut(&p), cap_of(size)) {
            (Some(b), Some(cap)) if cap <= b.cap => {
                b.size = size;
                true
            }
            _ => false,
        }
    }

    pub fn used(&self) -> u32 {
        self.bump - self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic pseudo-random workload (LCG), mixing sizes and frees.
    fn workload(a: &mut Arena, seed: u32, n: usize) -> Vec<u32> {
        let mut s = seed;
        let mut live: Vec<u32> = Vec::new();
        let mut trace = Vec::new();
        for _ in 0..n {
            s = s.wrapping_mul(1_103_515_245).wrapping_add(12345);
            if live.is_empty() || !s.is_multiple_of(3) {
                let size = match (s >> 8) % 4 {
                    0 => (s >> 12) % 64,
                    1 => (s >> 12) % 2048,
                    2 => (s >> 12) % 65536,
                    _ => (s >> 12) % 200_000,
                };
                let p = a.alloc(size).expect("arena large enough");
                assert_eq!(p % ALIGN, 0);
                trace.push(p);
                live.push(p);
            } else {
                let i = (s >> 16) as usize % live.len();
                assert!(a.free(live.swap_remove(i)));
            }
        }
        trace
    }

    #[test]
    fn same_requests_same_addresses() {
        let mut a = Arena::new(0x2000_0000, 0x3000_0000);
        let mut b = Arena::new(0x2000_0000, 0x3000_0000);
        assert_eq!(workload(&mut a, 7, 20_000), workload(&mut b, 7, 20_000));
        assert_eq!(a.used(), b.used());
    }

    #[test]
    fn no_overlap_and_reuse() {
        let mut a = Arena::new(0x1000, 0x10_0000);
        let p = a.alloc(10).unwrap();
        let q = a.alloc(10).unwrap();
        assert_eq!((p, q), (0x1000, 0x1010));
        assert_eq!(a.block(p), Some(Block { size: 10, cap: 16 }));
        assert!(a.free(p));
        assert!(!a.free(p));
        assert_eq!(a.alloc(16).unwrap(), p); // LIFO small list
        let big = a.alloc(10_000).unwrap();
        let big2 = a.alloc(10_000).unwrap();
        let tail = a.alloc(5_000).unwrap();
        assert!(a.free(big));
        assert!(a.free(big2)); // coalesced with big
        assert_eq!(a.alloc(20_000).unwrap(), big); // best fit reuses the coalesced run
        assert!(a.free(tail)); // tail returns to the bump region
        assert_eq!(a.used(), big + 20_000 - 0x1000);
        assert!(a.resize_in_place(big, 19_990));
        assert_eq!(
            a.block(big),
            Some(Block {
                size: 19_990,
                cap: 20_000
            })
        );
        assert!(!a.resize_in_place(big, 20_001));
        assert_eq!(a.alloc(0x20_0000), None);
        assert!(a.contains(big) && !a.contains(0x20_0000));
        // live blocks never overlap
        let mut all: Vec<(u32, u32)> = a.live.iter().map(|(&p, b)| (p, b.cap)).collect();
        all.sort();
        for w in all.windows(2) {
            assert!(w[0].0 + w[0].1 <= w[1].0);
        }
    }

    #[test]
    fn fragmented_workload_keeps_invariants() {
        let mut a = Arena::new(0x2000_0000, 0x3000_0000);
        workload(&mut a, 99, 50_000);
        let mut all: Vec<(u32, u32)> = a.live.iter().map(|(&p, b)| (p, b.cap)).collect();
        all.sort();
        for w in all.windows(2) {
            assert!(w[0].0 + w[0].1 <= w[1].0, "{w:?}");
        }
        for (&fa, &fl) in &a.free_by_addr {
            assert!(fa + fl <= a.bump);
            assert!(all.iter().all(|&(p, c)| p + c <= fa || p >= fa + fl));
        }
    }
}
