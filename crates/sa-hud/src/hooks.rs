//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table after
//! `sa_script::hooks::ALL`, in this order), and every module's globals.

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

use crate::{
    font, font_print, hud, hud_draw, menu, menu2, menu_mgr, messages, place, radar, radar_draw, radar_draw2, statics, statics2,
    text, text_load, timer,
    credits, hud_draw3, menu3, menu_ctrl, menu_draw, menu_input, menu_map, radar_draw3,
    menu_controls, menu_map_input, menu_stats_file,
};

/// Every module's registrations, in this order.
pub const MODULES: &[&[HookReg]] = &[
    font::HOOKS,
    hud::HOOKS,
    radar::HOOKS,
    messages::HOOKS,
    text::HOOKS,
    timer::HOOKS,
    menu::HOOKS,
    statics::HOOKS,
    font_print::HOOKS,
    hud_draw::HOOKS,
    radar_draw::HOOKS,
    text_load::HOOKS,
    menu_mgr::HOOKS,
    statics2::HOOKS,
    menu2::HOOKS,
    radar_draw2::HOOKS,
    place::HOOKS,
    hud_draw3::HOOKS,
    radar_draw3::HOOKS,
    menu_map::HOOKS,
    menu_draw::HOOKS,
    menu_input::HOOKS,
    menu3::HOOKS,
    credits::HOOKS,
    menu_ctrl::HOOKS,
    menu_map_input::HOOKS,
    menu_stats_file::HOOKS,
    menu_controls::HOOKS,
    crate::generated::HOOKS,
];

/// Every module's globals (initial values checked against the exe image by a test).
pub const GLOBALS: &[&[GlobalDesc]] = &[
    font::GLOBALS,
    hud::GLOBALS,
    radar::GLOBALS,
    messages::GLOBALS,
    text::GLOBALS,
    timer::GLOBALS,
    menu::GLOBALS,
    statics::GLOBALS,
    font_print::GLOBALS,
    hud_draw::GLOBALS,
    radar_draw::GLOBALS,
    text_load::GLOBALS,
    menu_mgr::GLOBALS,
    statics2::GLOBALS,
    menu2::GLOBALS,
    radar_draw2::GLOBALS,
    place::GLOBALS,
    hud_draw3::GLOBALS,
    radar_draw3::GLOBALS,
    menu_map::GLOBALS,
    menu_draw::GLOBALS,
    menu_input::GLOBALS,
    menu3::GLOBALS,
    credits::GLOBALS,
    menu_ctrl::GLOBALS,
    menu_map_input::GLOBALS,
    menu_stats_file::GLOBALS,
    menu_controls::GLOBALS,
];

use sa_core::hooks::total;

use sa_core::hooks::concat;

#[allow(clippy::large_const_arrays)]
const ALL_ARRAY: [HookReg; total(MODULES)] = concat(MODULES);

/// All registrations of this crate (the concatenation of [`MODULES`]).
pub const ALL: &[HookReg] = &ALL_ARRAY;

/// Every module's `#[native]` registrations (phase B entries of ports that cannot be hooked in
/// phase A); `sa_ports::NATIVES` lists each of these.
pub const NATIVE_MODULES: &[&[sa_core::abi::NativeReg]] = &[crate::font_print::NATIVES, crate::hud::NATIVES, crate::menu_system_init::NATIVES, crate::statics2::NATIVES];

pub const COUNT: usize = ALL.len();

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use sa_testkit::{count_hooks, exe_size};
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;
    
    use std::vec::Vec;

    /// Phase B/C storage starts with the original image's initial values (checked when the exe is
    /// available: `$SA_GTA_EXE` or `~/sa-engine/re/gta/1.0-US/gta_sa.exe`).
    #[test]
    fn global_initial_values_match_the_exe() {
        sa_testkit::check_global_initial_values(GLOBALS);
    }

    /// No global of this crate overlaps another one of this crate or one of sa-core's / sa-stream's /
    /// sa-world's / sa-pop's.
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
