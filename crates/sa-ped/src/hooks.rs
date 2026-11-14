//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table after
//! sa-core's, sa-stream's and sa-world's, sa-weapon's registrations, in this order), and every module's globals.
//! Each porting module adds its `HOOKS` / `GLOBALS` lists here.

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

/// Every module's registrations, in this order.
pub const MODULES: &[&[HookReg]] = &[
    crate::wanted::HOOKS,
    crate::ped_type::HOOKS,
    crate::damage::HOOKS,
    crate::ped::HOOKS,
    crate::ped_weapon::HOOKS,
    crate::player_ped::HOOKS,
    crate::player_info::HOOKS,
    crate::clothes::HOOKS,
    crate::clothes_builder::HOOKS,
    crate::default_txd_slot::HOOKS,
    crate::misc::HOOKS,
    crate::small::HOOKS,
    crate::ped_life::HOOKS,
    crate::ped_render::HOOKS,
    crate::ped_physics::HOOKS,
    crate::player_ctrl::HOOKS,
    crate::ped_control::HOOKS,
    crate::ped_misc3::HOOKS,
    crate::ped_coll::HOOKS,
    crate::player_target::HOOKS,
    crate::ped_entity_collision::HOOKS,
    crate::ped_type_init::HOOKS,
    crate::player_cross_hair::HOOKS,
    crate::generated::HOOKS,
    // WRITER-NATIVE
    crate::ped_pointer_valid::HOOKS,
    crate::player_sprint_results::HOOKS,
    crate::ped_dead_money::HOOKS,
    crate::ped_kill_with_car::HOOKS,
    crate::ped_tyres_bloody::HOOKS,
    crate::save_blocks::HOOKS,
    crate::ped_is_pointer_valid::HOOKS,
];

/// Every module's globals (initial values checked against the exe image by a test).
pub const GLOBALS: &[&[GlobalDesc]] = &[
    crate::wanted::GLOBALS,
    crate::ped_type::GLOBALS,
    crate::player_ped::GLOBALS,
    crate::clothes::GLOBALS,
    crate::misc::GLOBALS,
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

    /// No global of this crate overlaps another one of this crate or one of another crate's (a
    /// global declared twice would be two separate statics in phases B/C). Sizes are the exe's
    /// (32-bit).
    #[test]
    fn globals_are_declared_once_across_crates() {
        let mine: Vec<(u32, u32, &str)> = GLOBALS
            .iter()
            .flat_map(|l| l.iter())
            .map(|g| (g.addr, g.addr + exe_size(g) as u32, g.name))
            .collect();
        let core: Vec<(u32, u32, &str)> = sa_core::hooks::GLOBALS
            .iter()
            .chain(sa_stream::hooks::GLOBALS.iter())
            .chain(sa_world::hooks::GLOBALS.iter())
            .chain(sa_weapon::hooks::GLOBALS.iter())
            .flat_map(|l| l.iter())
            .map(|g| (g.addr, g.addr + exe_size(g) as u32, g.name))
            .collect();
        let mut bad = Vec::new();
        for (i, a) in mine.iter().enumerate() {
            for b in mine[i + 1..].iter().chain(core.iter()) {
                if !(a.1 <= b.0 || b.1 <= a.0) {
                    bad.push(std::format!("{} @ {:#x} overlaps {} @ {:#x}", a.2, a.0, b.2, b.0));
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    /// Every `#[hook(` in the sources is in `ALL`; addresses are unique, also against sa-core's and
    /// sa-stream's.
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
            .chain(sa_weapon::hooks::ALL.iter())
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
