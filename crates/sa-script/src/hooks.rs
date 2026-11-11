//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table after
//! `sa_world::hooks::ALL`), and every module's globals.

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

use crate::{
    cleanup, cmd0, cmd100, cmd200, cmd300, cmd400, cmd500, cmd600, cmd700, cmd800, cmd900, cmd1000, cmd1100,
    cmd1200, cmd1300, cmd1400, cmd1500, cmd1600, cmd1700, cmd1800, cmd1900, cmd2000, cmd2100, cmd2200,
    cmd2300, cmd2400, cmd2500, misc, script_brain_requests, script_brains, script_draw, script_init,
    script_objects, script_process, script_registries, script_resources, script_tables, script_tasks,
    script_things, script_locate, vm,
};

/// Every module's registrations, in this order.
pub const MODULES: &[&[HookReg]] = &[
    vm::HOOKS,
    cmd0::HOOKS,
    cmd100::HOOKS,
    cmd200::HOOKS,
    cmd300::HOOKS,
    cmd400::HOOKS,
    cmd500::HOOKS,
    cmd600::HOOKS,
    cmd700::HOOKS,
    cmd800::HOOKS,
    cmd900::HOOKS,
    cmd1000::HOOKS,
    cmd1100::HOOKS,
    cmd1200::HOOKS,
    cmd1300::HOOKS,
    cmd1400::HOOKS,
    cmd1500::HOOKS,
    cmd1600::HOOKS,
    cmd1700::HOOKS,
    cmd1800::HOOKS,
    cmd1900::HOOKS,
    cmd2000::HOOKS,
    cmd2100::HOOKS,
    cmd2200::HOOKS,
    cmd2300::HOOKS,
    cmd2400::HOOKS,
    cmd2500::HOOKS,
    cleanup::HOOKS,
    script_init::HOOKS,
    script_draw::HOOKS,
    script_brains::HOOKS,
    script_objects::HOOKS,
    script_brain_requests::HOOKS,
    script_resources::HOOKS,
    script_process::HOOKS,
    script_tables::HOOKS,
    script_registries::HOOKS,
    script_tasks::HOOKS,
    script_things::HOOKS,
    crate::the_scripts_init::HOOKS,
    misc::HOOKS,
    // script_search_lights::HOOKS (mvpk-script-2, full port of 0x4939F0) is not registered: the_scripts_init
    // registers the same address (std6-script-1, partial with fallback); keep one of them.
    script_locate::HOOKS,
    crate::script_thing_index::HOOKS,
    crate::streamed_script_load::HOOKS,
    crate::script_entity_cleanup::HOOKS,
];

/// The partial switches (ADR-002) and their Rust cases: (first opcode of the switch, `PORTED`).
pub const PARTIAL: &[(u16, &[u16])] = &[
    (100, cmd100::PORTED),
    (200, cmd200::PORTED),
    (300, cmd300::PORTED),
    (400, cmd400::PORTED),
    (500, cmd500::PORTED),
    (600, cmd600::PORTED),
    (700, cmd700::PORTED),
    (800, cmd800::PORTED),
    (900, cmd900::PORTED),
    (1000, cmd1000::PORTED),
    (1100, cmd1100::PORTED),
    (1200, cmd1200::PORTED),
    (1300, cmd1300::PORTED),
    (1400, cmd1400::PORTED),
    (1500, cmd1500::PORTED),
    (1600, cmd1600::PORTED),
    (1700, cmd1700::PORTED),
    (1800, cmd1800::PORTED),
    (1900, cmd1900::PORTED),
    (2000, cmd2000::PORTED),
    (2100, cmd2100::PORTED),
    (2200, cmd2200::PORTED),
    (2300, cmd2300::PORTED),
    (2400, cmd2400::PORTED),
    (2500, cmd2500::PORTED),
];

/// Every module's globals (initial values checked against the exe image by a test).
pub const GLOBALS: &[&[GlobalDesc]] = &[vm::GLOBALS, cmd0::GLOBALS];

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

    /// The exe's size of a global: pointer-sized elements are 4 bytes there, possibly 8 on a 64-bit
    /// host. Globals made of pointers are checked as all-zero (their exe image must be zero too).
    fn exe_size(g: &GlobalDesc) -> usize {
        let p = core::mem::size_of::<usize>();
        if p == 8 && g.size.is_multiple_of(8) && g.init.iter().all(|b| *b == 0) && g.size == 8 {
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
                assert_eq!(&img[..], g.init, "{} @ {:#x}", g.name, g.addr);
            } else {
                assert!(img.iter().all(|b| *b == 0), "{} @ {:#x}", g.name, g.addr);
            }
        }
    }

    /// No global of this crate overlaps another one of this crate or one of the other engine crates'.
    #[test]
    fn globals_are_declared_once_across_crates() {
        let range = |g: &GlobalDesc| (g.addr, g.addr + exe_size(g) as u32, g.name);
        let mine: Vec<(u32, u32, &str)> = GLOBALS.iter().flat_map(|l| l.iter()).map(range).collect();
        let others: Vec<(u32, u32, &str)> = sa_core::hooks::GLOBALS
            .iter()
            .chain(sa_stream::hooks::GLOBALS.iter())
            .chain(sa_world::hooks::GLOBALS.iter())
            .flat_map(|l| l.iter())
            .map(range)
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

    /// Every `#[hook(` in the sources is in `ALL`; addresses are unique, also against the other
    /// engine crates'.
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
        {
            assert!(
                !addrs.contains(&h.addr),
                "{} hooked by another engine crate too",
                h.name
            );
        }
        for h in ALL {
            #[cfg(not(target_arch = "x86"))]
            assert_eq!((h.entry)(), 0, "no shims off x86");
            assert_eq!((h.calls)(), 0);
        }
    }

    /// The opcodes with a jump-table case in the switch starting at `lo` (`catalog/script-commands.csv`,
    /// `orig_dispatch == case`; the other opcodes of the range reach the switch's default, -1).
    fn jump_table_cases(lo: u16) -> BTreeSet<u16> {
        let csv = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog/script-commands.csv");
        let text = std::fs::read_to_string(csv).unwrap();
        text.lines()
            .skip(1)
            .filter_map(|l| {
                let f: Vec<&str> = l.split(',').collect();
                let op = u16::from_str_radix(f[0], 16).ok()?;
                (op / 100 * 100 == lo && f.get(6) == Some(&"case")).then_some(op)
            })
            .collect()
    }

    /// Switch ports (ADR-002): every `PORTED` list is ascending, inside the switch's range and a subset
    /// of its jump-table cases, and every listed opcode has its own match arm (and every arm is
    /// listed). A switch whose hook has no fallback is complete: `PORTED` covers every jump-table case
    /// and its last arm is the original default (`_ => return -1`).
    #[test]
    fn partial_switches_list_their_rust_cases() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for (lo, ported) in PARTIAL {
            let name = std::format!("CRunningScript::ProcessCommands{lo}To{}", lo + 99);
            let h = ALL
                .iter()
                .find(|h| h.name == name)
                .unwrap_or_else(|| panic!("{name} not hooked"));
            let complete = h.set_original.is_none();
            assert!(
                ported.windows(2).all(|w| w[0] < w[1]),
                "{name}: PORTED not ascending"
            );
            let cases = jump_table_cases(*lo);
            for op in *ported {
                assert!(
                    cases.contains(op),
                    "{name}: {op:04X} in PORTED has no jump-table case"
                );
            }
            if complete {
                let missing: Vec<String> = cases
                    .iter()
                    .filter(|op| !ported.contains(op))
                    .map(|op| std::format!("{op:04X}"))
                    .collect();
                assert!(
                    missing.is_empty(),
                    "{name}: no fallback but not ported: {missing:?}"
                );
            }
            let src: String = std::fs::read_to_string(dir.join(std::format!("cmd{lo}.rs"))).unwrap();
            // Arms of the switch's `match command {` up to the last arm:
            // `0x00EC..=0x00F1 | 0x00FE => {`
            let body = src
                .split("match command {")
                .nth(1)
                .unwrap_or_else(|| panic!("{name}: no match"));
            // A complete switch has no fallback call left; its last arm (`_ =>`) is the original
            // default (-1, after the switch's common destructor tail where it has one).
            if complete {
                assert!(
                    !src.contains("fallback("),
                    "{name}: complete but still calls fallback"
                );
            }
            let last = if complete { "_ =>" } else { "fallback(" };
            assert!(body.contains(last), "{name}: no `{last}` arm");
            let body = body.split(last).next().unwrap();
            let mut arms: Vec<(u32, u32)> = Vec::new();
            for l in body
                .lines()
                .map(str::trim_start)
                .filter(|l| l.starts_with("0x") && l.contains("=>"))
            {
                for alt in l.split("=>").next().unwrap().split('|') {
                    let mut it = alt.trim().split("..=");
                    let a = u32::from_str_radix(it.next().unwrap().trim_start_matches("0x"), 16).unwrap();
                    let b = it.next().map_or(a, |b| {
                        u32::from_str_radix(b.trim().trim_start_matches("0x"), 16).unwrap()
                    });
                    arms.push((a, b));
                }
            }
            for &op in *ported {
                assert!(
                    (*lo..*lo + 100).contains(&op),
                    "{name}: {op:04X} outside the switch"
                );
                assert!(
                    arms.iter().any(|&(a, b)| (a..=b).contains(&u32::from(op))),
                    "{name}: {op:04X} listed in PORTED without a match arm"
                );
            }
            for &(a, b) in &arms {
                for op in a..=b {
                    assert!(
                        ported.contains(&(op as u16)),
                        "{name}: arm {op:04X} missing from PORTED"
                    );
                }
            }
        }
    }
}
