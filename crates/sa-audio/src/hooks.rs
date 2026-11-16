//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table after
//! `sa_hud::hooks::ALL`, in this order), and every module's globals.

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

use crate::{
    engine, entities, gen_bodies, gen_ctors, gen_dtors, gen_leaves, gen_virtuals, hardware, ped, ped_events,
    port2_a, port2_b, port3_ent, port3_lead, port3_veh, radio, sound, statics, utility, vehicle,
};

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
    gen_leaves::HOOKS,
    gen_virtuals::HOOKS,
    gen_ctors::HOOKS,
    gen_dtors::HOOKS,
    gen_bodies::HOOKS,
    port2_a::HOOKS,
    port2_b::HOOKS,
    port3_lead::HOOKS,
    port3_veh::HOOKS,
    port3_ent::HOOKS,
    crate::port4_hw::HOOKS,
    crate::port4_init::HOOKS,
    crate::port4_service::HOOKS,
    ped_events::HOOKS,
    crate::global_weapon_ambient::HOOKS,
    crate::audio_data_stream::HOOKS,
    crate::beat_info::HOOKS,
    crate::weather_update_parameters::HOOKS,
    crate::generated::HOOKS,
    crate::vehicle_add_audio_event::HOOKS,
    crate::player_vehicle_engine::HOOKS,
    // WRITER-NATIVE
    crate::weapon_gun_sounds::HOOKS,
    crate::cj_mood::HOOKS,
    // FRONTIER-2
    crate::ped_hit::HOOKS,
    crate::explosion_audio::HOOKS,
    crate::frontier_c::HOOKS,
    crate::mission_audio_event::HOOKS,
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
    port2_a::GLOBALS,
    port2_b::GLOBALS,
    port3_lead::GLOBALS,
    port3_veh::GLOBALS,
    port3_ent::GLOBALS,
    crate::port4_hw::GLOBALS,
    crate::port4_init::GLOBALS,
    crate::port4_service::GLOBALS,
];

/// Every module's `#[native]` registrations (phase B entries of ports that cannot be hooked in
/// phase A); `sa_ports::NATIVES` lists each of these.
pub const NATIVE_MODULES: &[&[sa_core::abi::NativeReg]] = &[
    crate::utility::NATIVES,
    crate::port4_hw::NATIVES,
    crate::port4_init::NATIVES,
    crate::port4_service::NATIVES,
    crate::bicycle_tyre_volume::NATIVES,
    crate::player_vehicle_engine::NATIVES,
    crate::frontier_c::NATIVES,
    crate::weather_update_parameters::NATIVES,
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
