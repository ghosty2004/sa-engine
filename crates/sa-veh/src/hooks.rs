//! Every `#[hook]` of this crate, in a fixed order (sa-hook appends them to its table after
//! `sa_pop::hooks::ALL`, in this order), and every module's globals.

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;

use crate::{
    air2, automobile, automobile2, bike, bike2, boat, boat2, checks, checks2, damage, door, door2, enterexit,
    enterexit2, handling, handling2, heli, misc, mvpb_a1, mvpb_a2, mvpb_a3, mvpb_a4, mvpb_a5, mvpd_v1,
    mvpd_v2, mvpd_v3, mvpd_v4, mvpd_v5, mvpd_v6, mvpd_v7, statics, statics2, train, train2, transmission,
    vehicle, vehicle2,
};

/// Every module's registrations, in this order.
pub const MODULES: &[&[HookReg]] = &[
    damage::HOOKS,
    vehicle::HOOKS,
    automobile::HOOKS,
    bike::HOOKS,
    door::HOOKS,
    enterexit::HOOKS,
    handling::HOOKS,
    train::HOOKS,
    heli::HOOKS,
    boat::HOOKS,
    checks::HOOKS,
    statics::HOOKS,
    vehicle2::HOOKS,
    crate::vehicle_save::HOOKS,
    statics2::HOOKS,
    misc::HOOKS,
    automobile2::HOOKS,
    bike2::HOOKS,
    transmission::HOOKS,
    air2::HOOKS,
    boat2::HOOKS,
    train2::HOOKS,
    door2::HOOKS,
    handling2::HOOKS,
    enterexit2::HOOKS,
    checks2::HOOKS,
    // WRITER-MVP-C (unverified batches)
    crate::mvpc_veh1::HOOKS,
    // WRITER-MVP-E (unverified batches)
    crate::mvpe_veh1::HOOKS,
    mvpb_a1::HOOKS,
    mvpb_a2::HOOKS,
    mvpb_a3::HOOKS,
    mvpb_a4::HOOKS,
    mvpb_a5::HOOKS,
    // WRITER-MVP-F (unverified batches)
    crate::vehicle_collision_anims::HOOKS,
    crate::vehicle_pre_collision::HOOKS,
    mvpd_v1::HOOKS,
    mvpd_v2::HOOKS,
    mvpd_v3::HOOKS,
    mvpd_v4::HOOKS,
    mvpd_v5::HOOKS,
    mvpd_v6::HOOKS,
    mvpd_v7::HOOKS,
    crate::heli_cone::HOOKS,
    // WRITER-VEH-BIG (unverified batches)
    crate::automobile_control::HOOKS,
    crate::fire_truck::HOOKS,
    // FRONTIER-2 lane B
    crate::frontier_b::HOOKS,
    crate::automobile_wheels::HOOKS,
    crate::automobile_damage::HOOKS,
    crate::automobile_ai::HOOKS,
    crate::vehicle_lights::HOOKS,
    crate::vehicle_wheel_particles::HOOKS,
    crate::bmx_ai::HOOKS,
    crate::car_enter_exit_offsets::HOOKS,
    crate::automobile_render::HOOKS,
    crate::train_control::HOOKS,
    crate::bike_control::HOOKS,
    crate::bike_render::HOOKS,
    crate::generated::HOOKS,
    // STD-12 (unverified batches)
    crate::automobile_random_damage::HOOKS,
    crate::bike_fix_hands::HOOKS,
    crate::vehicle_damage_react::HOOKS,
    crate::float_pow::HOOKS,
    // STD-12 (unverified batches)
    crate::car_enter_quit::HOOKS,
    crate::bike_rider_anims::HOOKS,
    // WRITER-NATIVE
    crate::vehicle_collision_particles::HOOKS,
    // STD-14 (unverified batches)
    crate::car_door_positions::HOOKS,
    crate::vehicle_process_open_door::HOOKS,
    crate::bike_knock_off_rider::HOOKS,
    crate::car_enter_stealable::HOOKS,
    // PORT-VEH-3 (unverified batches)
    crate::vehicle_suspension_nodes::HOOKS,
    crate::vehicle_part_damage::HOOKS,
    crate::train_stations::HOOKS,
    crate::vehicle_flight_targets::HOOKS,
    crate::vehicle_tow_hitch::HOOKS,
    crate::vehicle_upgrade_remove::HOOKS,
    crate::bus_doors_nitro_plates::HOOKS,
    crate::automobile_tyre_fix::HOOKS,
    crate::special_vehicle_control::HOOKS,
    crate::vehicle_kill_entering_peds::HOOKS,
    crate::vehicle_soft_ground::HOOKS,
    // PORT-VEH-4
    crate::automobile_nitrous::HOOKS,
    crate::boat_pre_render::HOOKS,
    crate::heli_pre_render::HOOKS,
    crate::boat_render::HOOKS,
    crate::wake_locks_component_rotation::HOOKS,
    crate::automobile_flying_component::HOOKS,
    crate::quad_bike_controls::HOOKS,
    crate::vehicle_plane_weapons::HOOKS,
    crate::train_track_position::HOOKS,
    crate::vehicle_drive_by::HOOKS,
    crate::heli_controls::HOOKS,
    crate::plane_controls::HOOKS,
    crate::automobile_blow_up::HOOKS,
    crate::automobile_controls::HOOKS,
    crate::bike_controls::HOOKS,
    crate::vehicle_blade_collision::HOOKS,
    crate::heli_dust::HOOKS,
    crate::automobile_close_doors::HOOKS,
    crate::plane_damage::HOOKS,
    crate::vehicle_tail_calls::HOOKS,
    crate::vehicle_fire_rates_upgrades::HOOKS,
    crate::vehicle_teleport_destroy::HOOKS,
    crate::vehicle_misc_queries::HOOKS,
    crate::stored_car_restore::HOOKS,
    crate::vehicle_can_ped_enter::HOOKS,
    crate::recording_start::HOOKS,
    // FRONTIER-2
    crate::trailer_susp::HOOKS,
    crate::moving_collision::HOOKS,
    crate::trailer_ai::HOOKS,
    crate::inflict_damage::HOOKS,
    // FRONTIER-2 turn 2, lane B
    crate::frontier_b2::HOOKS,
    crate::frontier_b2_veh::HOOKS,
    crate::frontier_b2_veh2::HOOKS,
    // FRONTIER-2 turn 3, lane B
    crate::frontier_b3_veh::HOOKS,
    // FRONTIER-2 turn 4, lane B
    crate::frontier_b4_heli::HOOKS,
    crate::frontier_b4_veh::HOOKS,
];

/// Every module's globals (initial values checked against the exe image by a test).
pub const GLOBALS: &[&[GlobalDesc]] = &[
    vehicle::GLOBALS,
    automobile::GLOBALS,
    bike::GLOBALS,
    door::GLOBALS,
    enterexit::GLOBALS,
    handling::GLOBALS,
    train::GLOBALS,
    heli::GLOBALS,
    boat::GLOBALS,
    checks::GLOBALS,
    statics::GLOBALS,
    vehicle2::GLOBALS,
    statics2::GLOBALS,
    misc::GLOBALS,
    automobile2::GLOBALS,
    bike2::GLOBALS,
    transmission::GLOBALS,
    air2::GLOBALS,
    boat2::GLOBALS,
    train2::GLOBALS,
    door2::GLOBALS,
    handling2::GLOBALS,
    enterexit2::GLOBALS,
    checks2::GLOBALS,
    mvpb_a1::GLOBALS,
    mvpb_a2::GLOBALS,
    mvpb_a3::GLOBALS,
    mvpb_a4::GLOBALS,
    mvpb_a5::GLOBALS,
    mvpd_v1::GLOBALS,
    mvpd_v2::GLOBALS,
    mvpd_v3::GLOBALS,
    mvpd_v4::GLOBALS,
    mvpd_v5::GLOBALS,
    mvpd_v6::GLOBALS,
    mvpd_v7::GLOBALS,
    crate::automobile_control::GLOBALS,
    crate::automobile_wheels::GLOBALS,
    crate::automobile_damage::GLOBALS,
    crate::automobile_ai::GLOBALS,
    crate::vehicle_lights::GLOBALS,
    crate::vehicle_wheel_particles::GLOBALS,
    crate::bmx_ai::GLOBALS,
    crate::car_enter_exit_offsets::GLOBALS,
    crate::automobile_render::GLOBALS,
    crate::train_control::GLOBALS,
    crate::bike_control::GLOBALS,
    crate::bike_render::GLOBALS,
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
