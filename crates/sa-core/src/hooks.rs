//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table in this order).
//!
//! The list is explicit on purpose: no linker-section magic, the same order on every build, and a
//! test fails when a `#[hook]` is missing here.

use crate::abi::HookReg;
use crate::general;
use crate::mem::GlobalDesc;

/// Every module's registrations, in this order. Each port module keeps its own `pub const HOOKS`
/// list next to its `#[hook]`s (one list per module keeps parallel porting work conflict-free).
pub const MODULES: &[&[HookReg]] = &[
    crate::general::HOOKS,
    crate::pool::HOOKS,
    crate::timer::HOOKS,
    crate::pools::HOOKS,
    crate::quadtree::HOOKS,
    crate::references::HOOKS,
    crate::directory::HOOKS,
    crate::keygen::HOOKS,
    crate::filemgr::HOOKS,
    crate::inifile::HOOKS,
    crate::memory::HOOKS,
    crate::loadmonitor::HOOKS,
    crate::common::HOOKS,
    crate::platform::HOOKS,
    crate::input::HOOKS,
    crate::video::HOOKS,
    crate::light::HOOKS,
    crate::game::HOOKS,
    crate::mvpa_game::HOOKS,
    crate::mvpa_game2::HOOKS,
    crate::mvpb_app::HOOKS,
    crate::gameinit::HOOKS,
    crate::psinit::HOOKS,
    crate::camerasize::HOOKS,
    crate::cmdline::HOOKS,
    crate::cmatrix::HOOKS,
    crate::pool_ctors::HOOKS,
    crate::object_ctors::HOOKS,
    crate::vector_ops::HOOKS,
    crate::matrix_ops::HOOKS,
    crate::matrix_rotate::HOOKS,
    crate::matrix_more::HOOKS,
    crate::generated::HOOKS,
];

/// Every `#[native]` of this crate (phase B entries of ports that cannot be hooked in phase A).
pub const NATIVES: &[crate::abi::NativeReg] = &[
    crate::memory::NATIVE_INIT,
    crate::memory::NATIVE_NULLSUB_72F3C0,
    crate::filemgr::NATIVE_SET_DIR,
    crate::platform::NATIVE_WIN32_INIT_APPLICATION,
    crate::platform::NATIVE_CREATE_DEBUG_FONT,
    crate::platform::NATIVE_RS_CAMERA_SHOW_RASTER,
    crate::timer::NATIVE_UPDATE,
    crate::game::NATIVE_VECTOR_NORMALISE,
    crate::game::NATIVE_BREAK_MANAGER_INIT,
    crate::game::NATIVE_DEBUG_DISPLAY_TEXT_BUFFER,
    crate::game::NATIVE_FLUSH_OBRS_PRINTFS,
    crate::game::NATIVE_STOP_PADS_SHAKING,
    crate::game::NATIVE_IM_GONNA_USE_STREAMING_MEMORY,
    crate::game::NATIVE_I_HAVE_USED_STREAMING_MEMORY,
    crate::gameinit::NATIVES[0],
    crate::gameinit::NATIVES[1],
    crate::gameinit::NATIVES[2],
    crate::gameinit::NATIVES[3],
    crate::gameinit::NATIVES[4],
    crate::gameinit::NATIVES[5],
    crate::gameinit::NATIVES[6],
    crate::gameinit::NATIVES[7],
    crate::gameinit::NATIVES[8],
    crate::gameinit::NATIVES[9],
    crate::gameinit::NATIVES[10],
    crate::gameinit::NATIVES[11],
    crate::gameinit::NATIVES[12],
    crate::object_ctors::NATIVES[0],
    crate::object_ctors::NATIVES[1],
    crate::object_ctors::NATIVES[2],
    crate::object_ctors::NATIVES[3],
    crate::object_ctors::NATIVES[4],
    crate::object_ctors::NATIVES[5],
    crate::object_ctors::NATIVES[6],
    crate::object_ctors::NATIVES[7],
    crate::pool_ctors::NATIVES[0],
    crate::matrix_ops::NATIVES[0],
    crate::matrix_ops::NATIVES[1],
];

/// Every module's globals (initial values checked against the exe image by a test).
pub const GLOBALS: &[&[GlobalDesc]] = &[
    crate::timer::TIMER_GLOBALS,
    crate::pool::POOL_GLOBALS,
    crate::pools::GLOBALS,
    crate::quadtree::GLOBALS,
    crate::references::GLOBALS,
    crate::directory::GLOBALS,
    crate::keygen::GLOBALS,
    crate::filemgr::GLOBALS,
    crate::inifile::GLOBALS,
    crate::memory::GLOBALS,
    crate::loadmonitor::GLOBALS,
    crate::common::GLOBALS,
    crate::platform::GLOBALS,
    crate::input::GLOBALS,
    crate::video::GLOBALS,
    crate::video::SELECT_GLOBALS,
    crate::light::GLOBALS,
    crate::game::GLOBALS,
    crate::gameinit::GLOBALS,
    crate::psinit::GLOBALS,
    crate::camerasize::GLOBALS,
    crate::cmdline::GLOBALS,
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
    let mut out = [general::HOOKS[0]; N];
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

// A `const` (not a `static`): sa-hook reads it while building its own table in const evaluation.
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
                            if o < rsize {
                                exe[(raw + o) as usize]
                            } else {
                                0
                            }
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
        for g in GLOBALS.iter().flat_map(|l| l.iter()) {
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

    /// A global is declared once (two declarations would be two separate statics in phases B/C).
    #[test]
    fn globals_are_declared_once() {
        let mut seen = BTreeSet::new();
        for g in GLOBALS.iter().flat_map(|l| l.iter()) {
            assert!(seen.insert(g.addr), "{} @ {:#x} declared twice", g.name, g.addr);
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
