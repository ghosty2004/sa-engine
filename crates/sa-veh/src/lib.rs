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
// SAVE-1 (docs/arch/save-port.md)
pub mod vehicle_save;

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
pub mod bike_car_horn;
pub mod vehicle_damage_react;
pub mod generated;
pub mod float_pow;
pub mod bike_rider_anims;
pub mod vehicle_collision_particles;
pub mod car_door_positions;
pub mod player_quit_car_enter;
pub mod bike_knock_off_rider;
pub mod car_enter_stealable;

// PORT-VEH-3 (unverified batches veh3-*)
pub mod vehicle_suspension_nodes;
pub mod vehicle_part_damage;
pub mod train_stations;
pub mod vehicle_flight_targets;
pub mod vehicle_tow_hitch;
pub mod vehicle_upgrade_remove;
