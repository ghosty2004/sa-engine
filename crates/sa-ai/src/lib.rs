//! AI subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PLAN §2 crate `sa-ai`; PORT-AI-1,
//! `docs/arch/ai-port.md`): `CTask*` (simple and complex), `CEvent*`, the event handler/group/scanner,
//! `CPedIntelligence`, `CTaskManager`, `CPedGroup*`, `CDecisionMaker*`, `CAcquaintance`, attractors.
//!
//! Same rules as the other engine crates (ADR-001): game state through `sa_core::globals!`/`GPtr`,
//! calls to other catalogued functions through `sa_core::orig!` with the original address, every port
//! registered with `#[hook]` and listed in its module's `HOOKS` ([`hooks::ALL`]). Task and event objects
//! keep the original vtables (a ported virtual is a hook on the function, not a new vtable) and are
//! allocated from the original pools through the original `operator new`. Ports follow the original
//! bytes, including its bugs (PLAN §4), never gta-reversed's fixes.
#![no_std]
#![deny(unsafe_code)]
// Large `orig!` blocks expand one recursive macro step per declaration.
#![recursion_limit = "2048"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod car_drive;
pub mod car_enter;
pub mod car_exit;
pub mod collscan;
pub mod common;
pub mod conversation;
pub mod decisions;
pub mod dtors;
pub mod events2;
pub mod fall;
pub mod events;
pub mod fight;
pub mod fight_combo_style;
pub mod partner_meeting;
pub mod fighthelp;
pub mod gangtasks;
pub mod flee;
pub mod gang6;
mod gen_check;
pub mod gen_clones;
pub mod gen_ctors;
pub mod gen_dtors;
pub mod gen_leaves;
pub mod gen_subtasks;
pub mod gen_virtuals;
pub mod geh;
pub mod group;
pub mod groups;
pub mod handler;
pub mod hooks;
pub mod intel;
pub mod intelrest;
pub mod killtasks;
pub mod misc;
pub mod partner6;
pub mod mvpa_ai1;
pub mod mvpa_ai2;
pub mod event_stuck_in_air_priority;
pub mod wander_prostitute_stuck_in_air_dtors;
pub mod mvpc_ai1;
pub mod mvpc_ai2;
pub mod mvpc_ai3;
pub mod node_route_tasks;
pub mod anim_tasks;
pub mod ped_geometry;
pub mod player_on_foot;
pub mod car_drive_ped;
pub mod vehicle_collision_scan;
pub mod fire_scan;
pub mod walk_into_ped_scan;
pub mod walk_into_object_scan;
pub mod attractor_range_scan;
pub mod wander_chat_scan;
pub mod seek_entity_dtor;
pub mod conversation_ready;
pub mod ik_tasks;
pub mod hold_entity;
pub mod pgi;
pub mod seek;
pub mod small;
pub mod stuck;
pub mod smallai5;
pub mod taskmgr;
pub mod tasks;
pub mod trivial;
pub mod usegun;
pub mod cover_update;
pub mod event_global_group;
pub mod entity_scanner_scan;
pub mod mvpb_ai;
pub mod mvpb_ai2;
pub mod mvpb_ai3;
pub mod mvpb_ai4;
pub mod attractor_scanner;
pub mod walk;
pub mod walkround;
pub mod wander;

pub mod std6_natives;
pub mod std7_natives;
pub mod hold_entity_process;
pub mod ped_group_random_place;
pub mod walk_alongside_control;
pub mod ped_collision_responses;
pub mod entity_bbox_corners;
pub mod event_responses_simple;
pub mod event_responses_peds;
pub mod event_responses_misc;
pub mod event_responses_shot_vehicle;
pub mod player_ped_collision;
pub mod ped_player_collision;
pub mod dragged_out_car;
pub mod vehicle_potential_collision;
pub mod attractor_approach_event;
pub mod task_in_air_process_ped;
pub mod event_attractor_affects_ped;
pub mod stuck_in_air_subtask;
