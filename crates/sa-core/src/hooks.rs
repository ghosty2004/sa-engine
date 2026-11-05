//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table in this order).
//!
//! The list is explicit on purpose: no linker-section magic, the same order on every build, and a
//! test fails when a `#[hook]` is missing here.

use crate::abi::HookReg;
use crate::{general, pool, timer};

pub const ALL: &[HookReg] = &[
    // CGeneral
    general::HOOK_GET_RANDOM_NUMBER_IN_RANGE_I32,
    general::HOOK_GET_RANDOM_NUMBER_IN_RANGE_F32,
    // CPool<T> instantiations and the CPools wrappers
    pool::HOOK_PED_POOL_NEW,
    pool::HOOK_VEHICLE_POOL_NEW,
    pool::HOOK_OBJECT_POOL_NEW,
    pool::HOOK_PED_POOL_AT_REF,
    pool::HOOK_OBJECT_POOL_AT,
    pool::HOOK_PED_OPERATOR_NEW,
    pool::HOOK_PED_OPERATOR_DELETE,
    pool::HOOK_GET_PED_REF,
    pool::HOOK_GET_PED,
    pool::HOOK_GET_VEHICLE_REF,
    pool::HOOK_GET_VEHICLE,
    pool::HOOK_GET_OBJECT_REF,
    pool::HOOK_GET_OBJECT,
    // CTimer (not Update: sa-hook's frame-boundary passthrough)
    timer::HOOK_INITIALISE,
    timer::HOOK_SHUTDOWN,
    timer::HOOK_UPDATE_VARIABLES,
    timer::HOOK_SUSPEND,
    timer::HOOK_RESUME,
    timer::HOOK_GET_CYCLES_PER_MILLISECOND,
    timer::HOOK_GET_CYCLES_PER_FRAME,
    timer::HOOK_GET_CURRENT_TIME_IN_CYCLES,
    timer::HOOK_STOP,
    timer::HOOK_GET_IS_SLOW_MOTION_ACTIVE,
    timer::HOOK_START_USER_PAUSE,
    timer::HOOK_END_USER_PAUSE,
];

pub const COUNT: usize = ALL.len();

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::string::String;

    /// Initial bytes of a virtual address range in the exe image (`.bss` beyond the raw data = 0).
    fn exe_image_bytes(exe: &[u8], va: u32, n: usize) -> Option<std::vec::Vec<u8>> {
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
        for g in crate::timer::TIMER_GLOBALS
            .iter()
            .chain(crate::pool::POOL_GLOBALS)
        {
            // pointer-sized globals are 4 bytes in the exe; owned storage may be wider (64-bit host)
            let n = g.size.min(if g.init.len() == core::mem::size_of::<usize>() {
                4
            } else {
                g.size
            });
            let img =
                exe_image_bytes(&exe, g.addr, n).unwrap_or_else(|| panic!("{} not in the image", g.name));
            assert_eq!(&img[..], &g.init[..n], "{} @ {:#x}", g.name, g.addr);
        }
    }

    /// Every `#[hook(` in the sources is in `ALL` (counted over all files) and addresses are unique.
    #[test]
    fn list_is_complete_and_unique() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut n = 0;
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "rs") && p.file_name().unwrap() != "hooks.rs" {
                let s: String = std::fs::read_to_string(&p).unwrap();
                n += s
                    .lines()
                    .filter(|l| l.trim_start().starts_with("#[hook("))
                    .count();
            }
        }
        assert_eq!(n, COUNT, "a #[hook] is missing from hooks::ALL");
        let addrs: BTreeSet<u32> = ALL.iter().map(|h| h.addr).collect();
        assert_eq!(addrs.len(), COUNT);
        assert!(
            !ALL.iter().any(|h| h.addr == 0x561B10),
            "CTimer::Update stays sa-hook's"
        );
        for h in ALL {
            #[cfg(not(target_arch = "x86"))]
            assert_eq!((h.entry)(), 0, "no shims off x86");
            assert_eq!((h.calls)(), 0);
        }
    }
}
