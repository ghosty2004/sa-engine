//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table after
//! `sa_hud::hooks::ALL`, in this order), and every module's globals.

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

use crate::{engine, entities, hardware, ped, radio, sound, statics, utility, vehicle};

/// Every module's registrations, in this order.
pub const MODULES: &[&[HookReg]] = &[
    engine::HOOKS,
    hardware::HOOKS,
    sound::HOOKS,
    utility::HOOKS,
    radio::HOOKS,
    vehicle::HOOKS,
    ped::HOOKS,
    entities::HOOKS,
    statics::HOOKS,
];

/// Every module's globals (initial values checked against the exe image by a test).
pub const GLOBALS: &[&[GlobalDesc]] = &[
    engine::GLOBALS,
    hardware::GLOBALS,
    sound::GLOBALS,
    utility::GLOBALS,
    radio::GLOBALS,
    vehicle::GLOBALS,
    ped::GLOBALS,
    entities::GLOBALS,
    statics::GLOBALS,
];

const fn total(parts: &[&[HookReg]]) -> usize {
    let (mut n, mut i) = (0, 0);
    while i < parts.len() {
        n += parts[i].len();
        i += 1;
    }
    n
}

const fn concat<const N: usize>(parts: &[&[HookReg]]) -> [HookReg; N] {
    let mut out = [sa_core::hooks::ALL[0]; N];
    let (mut k, mut i) = (0, 0);
    while i < parts.len() {
        let mut j = 0;
        while j < parts[i].len() {
            out[k] = parts[i][j];
            k += 1;
            j += 1;
        }
        i += 1;
    }
    assert!(k == N);
    out
}

#[allow(clippy::large_const_arrays)]
const ALL_ARRAY: [HookReg; total(MODULES)] = concat(MODULES);

/// All registrations of this crate (the concatenation of [`MODULES`]).
pub const ALL: &[HookReg] = &ALL_ARRAY;

pub const COUNT: usize = ALL.len();

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;
    use std::string::String;
    use std::vec::Vec;

    /// Initial bytes of a virtual address range in the exe image (`.bss` beyond the raw data = 0).
    fn exe_image_bytes(exe: &[u8], va: u32, n: usize) -> Option<Vec<u8>> {
        let u16_at = |o: usize| u16::from_le_bytes([exe[o], exe[o + 1]]) as usize;
        let u32_at = |o: usize| u32::from_le_bytes(exe[o..o + 4].try_into().unwrap());
        let pe = u32_at(0x3C) as usize;
        let (nsec, opt) = (u16_at(pe + 6), u16_at(pe + 20));
        let base = u32_at(pe + 24 + 28);
        for s in 0..nsec {
            let h = pe + 24 + opt + s * 40;
            let (vsize, va0, rsize, raw) = (
                u32_at(h + 8),
                base + u32_at(h + 12),
                u32_at(h + 16),
                u32_at(h + 20),
            );
            if va >= va0 && va + n as u32 <= va0 + vsize.max(rsize) {
                return Some(
                    (0..n as u32)
                        .map(|k| {
                            let o = va + k - va0;
                            if o < rsize { exe[(raw + o) as usize] } else { 0 }
                        })
                        .collect(),
                );
            }
        }
        None
    }

    /// The exe's size of a global (pointer-sized elements are 4 bytes there, possibly 8 on a 64-bit host).
    fn exe_size(g: &GlobalDesc) -> usize {
        let p = core::mem::size_of::<usize>();
        if p == 8 && g.size.is_multiple_of(8) && g.init.iter().all(|b| *b == 0) {
            g.size / 2
        } else {
            g.size
        }
    }

    /// Phase B/C storage starts with the original image's initial values (checked when the exe is
    /// available: `$SA_GTA_EXE` or `~/sa-engine/re/gta/1.0-US/gta_sa.exe`).
    #[test]
    fn global_initial_values_match_the_exe() {
        let path = std::env::var("SA_GTA_EXE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                    .join("sa-engine/re/gta/1.0-US/gta_sa.exe")
            });
        let Ok(exe) = std::fs::read(&path) else {
            std::eprintln!("note: gta_sa.exe not found, initial values not checked");
            return;
        };
        for g in GLOBALS.iter().flat_map(|l| l.iter()) {
            let n = exe_size(g);
            let img =
                exe_image_bytes(&exe, g.addr, n).unwrap_or_else(|| panic!("{} not in the image", g.name));
            if n == g.size {
                assert_eq!(&img[..], &g.init[..n], "{} @ {:#x}", g.name, g.addr);
            } else {
                assert!(img.iter().all(|b| *b == 0), "{} @ {:#x}", g.name, g.addr);
            }
        }
    }

    /// No global of this crate overlaps another one of this crate or one of sa-core's / sa-stream's /
    /// sa-world's / ... / sa-hud's.
    #[test]
    fn globals_are_declared_once_across_crates() {
        let mine: Vec<(u32, u32, &str)> = GLOBALS
            .iter()
            .flat_map(|l| l.iter())
            .map(|g| (g.addr, g.addr + exe_size(g) as u32, g.name))
            .collect();
        let others: Vec<(u32, u32, &str)> = sa_core::hooks::GLOBALS
            .iter()
            .chain(sa_stream::hooks::GLOBALS.iter())
            .chain(sa_world::hooks::GLOBALS.iter())
            .chain(sa_pop::hooks::GLOBALS.iter())
            .chain(sa_script::hooks::GLOBALS.iter())
            .chain(sa_pop::hooks::GLOBALS.iter())
            .flat_map(|l| l.iter())
            .map(|g| (g.addr, g.addr + exe_size(g) as u32, g.name))
            .collect();
        let mut bad = Vec::new();
        for (i, a) in mine.iter().enumerate() {
            for b in mine[i + 1..].iter().chain(others.iter()) {
                if !(a.1 <= b.0 || b.1 <= a.0) {
                    bad.push(std::format!("{} @ {:#x} overlaps {} @ {:#x}", a.2, a.0, b.2, b.0));
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    fn count_hooks(dir: &Path) -> usize {
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                n += count_hooks(&p);
            } else if p.extension().is_some_and(|x| x == "rs") && p.file_name().unwrap() != "hooks.rs" {
                let s: String = std::fs::read_to_string(&p).unwrap();
                n += s
                    .lines()
                    .filter(|l| l.trim_start().starts_with("#[hook("))
                    .count();
            }
        }
        n
    }

    /// Every `#[hook(` in the sources is in `ALL`; addresses are unique, also against the other crates.
    #[test]
    fn list_is_complete_and_unique() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        assert_eq!(
            count_hooks(&dir),
            COUNT,
            "a #[hook] is missing from a module's HOOKS"
        );
        let addrs: BTreeSet<u32> = ALL.iter().map(|h| h.addr).collect();
        assert_eq!(addrs.len(), COUNT);
        for h in sa_core::hooks::ALL
            .iter()
            .chain(sa_stream::hooks::ALL.iter())
            .chain(sa_world::hooks::ALL.iter())
            .chain(sa_pop::hooks::ALL.iter())
            .chain(sa_script::hooks::ALL.iter())
            .chain(sa_pop::hooks::ALL.iter())
            .chain(sa_script::hooks::ALL.iter())
        {
            assert!(!addrs.contains(&h.addr), "{} hooked by another crate too", h.name);
        }
        for h in ALL {
            #[cfg(not(target_arch = "x86"))]
            assert_eq!((h.entry)(), 0, "no shims off x86");
            assert_eq!((h.calls)(), 0);
        }
    }
}
