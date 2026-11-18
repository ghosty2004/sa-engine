//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table after
//! sa-core's, sa-stream's and sa-world's registrations, in this order), and every module's globals.
//! Each porting module adds its `HOOKS` / `GLOBALS` lists here.

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

/// Every module's registrations, in this order.
pub const MODULES: &[&[HookReg]] = &[
    crate::weapon_info::HOOKS,
    crate::fire::HOOKS,
    crate::weapon::HOOKS,
    crate::weapon_fire::HOOKS,
    crate::weapon_impact::HOOKS,
    crate::fire_one_instant_hit::HOOKS,
    crate::explosion::HOOKS,
    crate::bullet_shot::HOOKS,
    crate::projectile::HOOKS,
    crate::water_cannon::HOOKS,
    crate::effects::HOOKS,
    crate::statics::HOOKS,
    crate::mvpb_a::HOOKS,
    crate::weapon_data::HOOKS,
    crate::generated::HOOKS,
    crate::projectile_ctor::HOOKS,
    crate::fire_instant_hit::HOOKS,
    crate::fire_projectile::HOOKS,
    crate::projectile_add::HOOKS,
    // FRONTIER-2 turn 2, lane B
    crate::frontier_b2::HOOKS,
    // FRONTIER-2 turn 11, lane B
    crate::doom_aiming::HOOKS,
    crate::photograph::HOOKS,
];

/// Every module's globals (initial values checked against the exe image by a test).
pub const GLOBALS: &[&[GlobalDesc]] = &[
    crate::weapon_info::GLOBALS,
    crate::fire::GLOBALS,
    crate::weapon::GLOBALS,
    crate::weapon_fire::GLOBALS,
    crate::weapon_impact::GLOBALS,
    crate::explosion::GLOBALS,
    crate::bullet_shot::GLOBALS,
    crate::projectile::GLOBALS,
    crate::water_cannon::GLOBALS,
    crate::effects::GLOBALS,
    crate::statics::GLOBALS,
    crate::mvpb_a::GLOBALS,
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
