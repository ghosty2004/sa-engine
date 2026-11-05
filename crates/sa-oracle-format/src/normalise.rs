//! Pointer normalisation for the L2 oracle (PLAN §4.3, §5). This is the ONE reviewed file that decides
//! when two different raw values count as "the same": every L2 comparison (in-process in `sa-hook`, and
//! the host replay in `sa-oracle`) goes through [`words_equal`] / [`bytes_equal`].
//!
//! Rule: raw-identical bytes are always equal. Only where raw bytes differ, each aligned 32-bit word is
//! classified, and the two words are equal iff both classify to the same normal form:
//!
//! | class | normal form | when |
//! |---|---|---|
//! | heap allocation made *during the case* | `Alloc { ordinal, off }` | inside a block the run allocated (determinism arena), ordinal = allocation index within that run |
//! | entity pool slot | `Pool { pool, slot, off }` | inside an entity pool's storage (ped, vehicle, building, object, dummy; strides from INF-6) |
//! | model info | `Model { id, off }` | inside `CModelInfo::ms_modelInfoPtrs[id]` .. +0x20 (`CBaseModelInfo` size, layouts.csv) |
//! | exe image | `Image { off }` | 0x400000 + off inside the image (symbol + offset; symbolised by the host report) |
//! | anything else | `Raw(v)` | |
//!
//! Inside one process, pools, model infos and the image are at the same addresses for the original's
//! run and the Rust run, so these classes only ever make raw-equal words equal; the class that changes a
//! verdict in-process is `Alloc` (the original and the Rust implementation allocate the "same" block at
//! different arena addresses). The other classes make fixtures comparable across runs. A word that is
//! not 4-byte aligned inside the compared buffer is never normalised (raw comparison).

use crate::subsys::PoolMap;

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8, catalogue rwsets name; gta-reversed ModelInfo.h).
pub const MODEL_INFO_PTRS: u32 = 0xA9B0C8;
/// `NUM_MODEL_INFOS` (gta-reversed ModelInfo.h: `ms_modelInfoPtrs[20000]`).
pub const NUM_MODEL_INFOS: u32 = 20000;
/// `sizeof(CBaseModelInfo)` (catalog/layouts.csv).
pub const BASE_MODEL_INFO_SIZE: u32 = 0x20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Norm {
    Raw(u32),
    Image { off: u32 },
    Pool { pool: u8, slot: u32, off: u32 },
    Model { id: u32, off: u32 },
    Alloc { ordinal: u32, off: u32 },
}

/// What the normaliser knows about one run (the original's or the Rust implementation's).
#[derive(Debug, Clone, Default)]
pub struct NormCtx {
    /// Exe image [start, end).
    pub image: (u32, u32),
    pub pools: PoolMap,
    /// Model info pointers, indexed by model id (empty: class not used).
    pub model_infos: Vec<u32>,
    /// Blocks allocated by this run during the case, in allocation order: (address, size).
    pub allocs: Vec<(u32, u32)>,
}

impl NormCtx {
    pub fn normalise(&self, v: u32) -> Norm {
        for (i, &(a, n)) in self.allocs.iter().enumerate() {
            if v >= a && (v as u64) < a as u64 + n.max(1) as u64 {
                return Norm::Alloc {
                    ordinal: i as u32,
                    off: v - a,
                };
            }
        }
        let (tag, rest) = self.pools.normalise(v);
        if tag != v {
            return Norm::Pool {
                pool: ((tag >> 24) & 0x1F) as u8,
                slot: rest >> 16,
                off: rest & 0xFFFF,
            };
        }
        if v != 0 {
            // Exact model info objects (first BASE_MODEL_INFO_SIZE bytes only: derived sizes vary).
            if let Some(id) = self
                .model_infos
                .iter()
                .position(|&p| p != 0 && v >= p && v - p < BASE_MODEL_INFO_SIZE)
            {
                return Norm::Model {
                    id: id as u32,
                    off: v - self.model_infos[id],
                };
            }
        }
        if v >= self.image.0 && v < self.image.1 {
            return Norm::Image {
                off: v - self.image.0,
            };
        }
        Norm::Raw(v)
    }
}

/// Two words from the original's run (`a`, context `ca`) and the Rust run (`b`, context `cb`).
pub fn words_equal(a: u32, ca: &NormCtx, b: u32, cb: &NormCtx) -> bool {
    a == b || ca.normalise(a) == cb.normalise(b)
}

/// Byte buffers starting at `addr` (alignment is taken from the absolute address). Returns the first
/// differing offset after normalisation, or None when equal.
pub fn bytes_equal(addr: u32, a: &[u8], ca: &NormCtx, b: &[u8], cb: &NormCtx) -> Option<usize> {
    if a == b {
        return None;
    }
    if a.len() != b.len() {
        return Some(a.len().min(b.len()));
    }
    let mut i = 0usize;
    while i < a.len() {
        if a[i] == b[i] {
            i += 1;
            continue;
        }
        // The aligned word containing byte i, if it lies completely inside both buffers.
        let abs = addr as u64 + i as u64;
        let ws = abs & !3;
        if ws >= addr as u64 && ws + 4 <= addr as u64 + a.len() as u64 {
            let w = (ws - addr as u64) as usize;
            let wa = u32::from_le_bytes([a[w], a[w + 1], a[w + 2], a[w + 3]]);
            let wb = u32::from_le_bytes([b[w], b[w + 1], b[w + 2], b[w + 3]]);
            if words_equal(wa, ca, wb, cb) {
                i = w + 4;
                continue;
            }
        }
        return Some(i);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subsys::{PoolHdr, addr};

    fn ctx(allocs: Vec<(u32, u32)>) -> NormCtx {
        let mut pools = PoolMap {
            pools: vec![PoolHdr::default(); addr::POOL_COUNT as usize],
        };
        pools.pools[3] = PoolHdr {
            objects: 0x3000_0000,
            bytemap: 0x3100_0000,
            size: 140,
        };
        NormCtx {
            image: (0x400000, 0x1577000),
            pools,
            model_infos: vec![0, 0x00A0_0000, 0x00A0_1000],
            allocs,
        }
    }

    #[test]
    fn classes() {
        let c = ctx(vec![(0x2000_0100, 0x40)]);
        assert_eq!(c.normalise(5), Norm::Raw(5));
        assert_eq!(
            c.normalise(0x2000_0110),
            Norm::Alloc {
                ordinal: 0,
                off: 0x10
            }
        );
        assert_eq!(
            c.normalise(0x3000_0000 + 2 * addr::PED_STRIDE + 8),
            Norm::Pool {
                pool: 3,
                slot: 2,
                off: 8
            }
        );
        assert_eq!(c.normalise(0x00A0_1004), Norm::Model { id: 2, off: 4 });
        assert_eq!(c.normalise(0x00B7_CD98), Norm::Image { off: 0x77CD98 });
        assert_eq!(c.normalise(0x00A0_1020), Norm::Image { off: 0x601020 });
    }

    #[test]
    fn allocation_ordinals_make_different_addresses_equal() {
        let (a, b) = (ctx(vec![(0x2000_0100, 0x40)]), ctx(vec![(0x2000_0200, 0x40)]));
        assert!(words_equal(0x2000_0104, &a, 0x2000_0204, &b));
        assert!(!words_equal(0x2000_0104, &a, 0x2000_0208, &b));
        // A raw value that is no allocation in either run must be identical.
        assert!(!words_equal(0x1234, &a, 0x1235, &b));
        // Pool / image pointers: raw equal only.
        assert!(!words_equal(0x3000_0000, &a, 0x3000_0000 + addr::PED_STRIDE, &b));
        let mut x = vec![0u8; 12];
        let mut y = vec![0u8; 12];
        x[4..8].copy_from_slice(&0x2000_0104u32.to_le_bytes());
        y[4..8].copy_from_slice(&0x2000_0204u32.to_le_bytes());
        assert_eq!(bytes_equal(0x1000, &x, &a, &y, &b), None);
        y[9] = 1;
        assert_eq!(bytes_equal(0x1000, &x, &a, &y, &b), Some(9));
        // A word cut by the buffer start is never normalised (raw comparison).
        assert_eq!(bytes_equal(0x1005, &x[5..], &a, &y[5..], &b), Some(0));
        assert_eq!(bytes_equal(0x1002, &x[2..], &a, &y[2..], &b), Some(7));
    }
}
