//! VEH subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PLAN §2 crate `sa-vehicle`, created as
//! `sa-veh`; PORT-VEH, `docs/arch/veh-port.md`): vehicles of every class, the damage manager, handling,
//! doors, enter/exit helpers, recordings and the small vehicle checks.
//!
//! Same rules as sa-core, sa-stream, sa-world and sa-pop (ADR-001): game state through
//! `sa_core::globals!`/`GPtr`, calls to other catalogued functions through `sa_core::orig!` with the
//! original address, every port registered with `#[hook]` and listed in its module's `HOOKS`
//! ([`hooks::ALL`]). Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/VEH/*.md` are reading
//! aids. Where they differ, the bytes win.
#![no_std]
#![deny(unsafe_code)]
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod common;
pub mod hooks;

pub mod automobile;
pub mod bike;
pub mod boat;
pub mod checks;
pub mod damage;
pub mod door;
pub mod enterexit;
pub mod handling;
pub mod heli;
pub mod statics;
pub mod train;
pub mod vehicle;

// PORT-VEH-2
pub mod air2;
pub mod automobile2;
pub mod bike2;
pub mod boat2;
pub mod checks2;
pub mod door2;
pub mod enterexit2;
pub mod handling2;
pub mod misc;
pub mod mvpc_veh1;
pub mod mvpe_veh1;
pub mod vehicle_collision_anims;
pub mod vehicle_pre_collision;
pub mod statics2;
pub mod train2;
pub mod transmission;
pub mod vehicle2;

// WRITER-MVP-B (helper A)
pub mod mvpb_a1;
pub mod mvpb_a2;
pub mod mvpb_a3;
pub mod mvpb_a4;
pub mod mvpb_a5;

// WRITER-MVP-D (helper VEH)
pub mod mvpd_v1;
pub mod mvpd_v2;
pub mod mvpd_v3;
pub mod mvpd_v4;
pub mod mvpd_v5;
pub mod mvpd_v6;
pub mod mvpd_v7;

// WRITER-VEH-BIG (unverified batches)
pub mod automobile_control;
pub mod automobile_wheels;
pub mod automobile_damage;
pub mod automobile_ai;
pub mod vehicle_lights;
pub mod vehicle_wheel_particles;
pub mod bmx_ai;
pub mod car_enter_exit_offsets;
pub mod automobile_render;
pub mod train_control;
pub mod bike_control;
pub mod bike_render;
pub mod automobile_random_damage;
pub mod bike_fix_hands;
pub mod vehicle_damage_react;
pub mod generated;
pub mod float_pow;
pub mod bike_rider_anims;
pub mod vehicle_collision_particles;
pub mod car_door_positions;
pub mod bike_knock_off_rider;
pub mod car_enter_stealable;

// PORT-VEH-3 (unverified batches veh3-*)
pub mod vehicle_suspension_nodes;
pub mod vehicle_part_damage;
pub mod train_stations;
pub mod vehicle_flight_targets;
pub mod vehicle_tow_hitch;
pub mod vehicle_upgrade_remove;
pub mod bus_doors_nitro_plates;
pub mod automobile_tyre_fix;
pub mod special_vehicle_control;
pub mod car_enter_quit;
pub mod vehicle_kill_entering_peds;
pub mod vehicle_soft_ground;

// PORT-VEH-4 (unverified batches veh4-*)
pub mod automobile_nitrous;
pub mod vehicle_plane_weapons;
pub mod quad_bike_controls;
pub mod automobile_flying_component;
pub mod wake_locks_component_rotation;
pub mod boat_render;
pub mod heli_pre_render;
pub mod boat_pre_render;
pub mod train_track_position;
pub mod vehicle_drive_by;
pub mod heli_controls;
pub mod plane_controls;
pub mod automobile_blow_up;
pub mod automobile_controls;
pub mod bike_controls;
pub mod vehicle_blade_collision;
pub mod heli_dust;
pub mod plane_damage;
pub mod vehicle_tail_calls;
pub mod vehicle_stub_natives;
pub mod vehicle_fire_rates_upgrades;
pub mod vehicle_teleport_destroy;
pub mod vehicle_misc_queries;
