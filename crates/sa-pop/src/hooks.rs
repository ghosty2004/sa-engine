//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table after
//! `sa_world::hooks::ALL`, in this order), and every module's globals.

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

use crate::{
    carctrl, carctrl2, carctrl3, carctrl4, carctrl5, carctrl6, carctrl7, cargen, gangs, misc, misc2, misc3,
    misc4, misc5, misc6, pathfind, popcycle, population, population2, population3, population4, statics,
    statics_gen,
};

/// Every module's registrations, in this order.
pub const MODULES: &[&[HookReg]] = &[
    popcycle::HOOKS,
    population::HOOKS,
    carctrl::HOOKS,
    cargen::HOOKS,
    gangs::HOOKS,
    misc::HOOKS,
    statics::HOOKS,
    statics_gen::HOOKS,
    pathfind::HOOKS,
    population2::HOOKS,
    carctrl2::HOOKS,
    misc2::HOOKS,
    carctrl3::HOOKS,
    carctrl4::HOOKS,
    carctrl5::HOOKS,
    population3::HOOKS,
    misc3::HOOKS,
    misc4::HOOKS,
    carctrl6::HOOKS,
    carctrl7::HOOKS,
    misc5::HOOKS,
    misc6::HOOKS,
    population4::HOOKS,
    crate::path_car_creation_nodes::HOOKS,
    crate::gang_war_release_peds::HOOKS,
    crate::generated::HOOKS,
    // WRITER-NATIVE
    crate::weave_heading_test::HOOKS,
    crate::weave_for_ped::HOOKS,
    crate::join_road_goto_coors::HOOKS,
    crate::update_car_ai::HOOKS,
    crate::emergency_occupants::HOOKS,
    // FRONTIER-2 turn 2, lane B
    crate::frontier_b2::HOOKS,
    crate::frontier_c::HOOKS,
    crate::road_block_between::HOOKS,
    crate::gang_war_tell::HOOKS,
];

/// Every module's globals (initial values checked against the exe image by a test).
pub const GLOBALS: &[&[GlobalDesc]] = &[
    popcycle::GLOBALS,
    population::GLOBALS,
    carctrl::GLOBALS,
    cargen::GLOBALS,
    gangs::GLOBALS,
    misc::GLOBALS,
    pathfind::GLOBALS,
    population2::GLOBALS,
    carctrl2::GLOBALS,
    misc2::GLOBALS,
    carctrl3::GLOBALS,
    carctrl4::GLOBALS,
    crate::update_car_ai::GLOBALS,
    crate::emergency_occupants::GLOBALS,
    carctrl5::GLOBALS,
    population3::GLOBALS,
    misc3::GLOBALS,
    misc4::GLOBALS,
    carctrl6::GLOBALS,
    carctrl7::GLOBALS,
    misc5::GLOBALS,
    misc6::GLOBALS,
    population4::GLOBALS,
];

use sa_core::hooks::total;

use sa_core::hooks::concat;

#[allow(clippy::large_const_arrays)]
const ALL_ARRAY: [HookReg; total(MODULES)] = concat(MODULES);

/// All registrations of this crate (the concatenation of [`MODULES`]).
pub const ALL: &[HookReg] = &ALL_ARRAY;

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
    /// sa-world's.
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
