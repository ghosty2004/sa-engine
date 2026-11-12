//! WORLD part 1 (PORT-WORLD, `docs/arch/world-port.md`): the leaf-ward world classes that do not depend
//! on `CPhysical`: zones, `CWorld` helper leaves, pickups, entry-exits, garages, escalators, bridge,
//! cranes, stunt jumps, stats, cheats, restart, game logic, shopping.

pub mod cheat;
pub mod common;
pub mod cworld;
pub mod entryexit;
pub mod escalators;
pub mod gamelogic;
pub mod garages;
pub mod garage_update;
pub mod misc;
pub mod pickups;
pub mod replay_playback;
pub mod restart;
pub mod shopping;
pub mod statics;
pub mod stats;
pub mod zones;
// PORT-WORLD-2 (docs/arch/world-port.md, part 2)
pub mod cworld2;
pub mod entity;
pub mod misc2;
pub mod mvpb_stats;
pub mod mvpb_world;
pub mod mvpb_world2;
pub mod mvpb_world3;
pub mod mvpd_w1;
pub mod mvpd_w2;
pub mod mvpd_w3;
pub mod mvpd_w4;
pub mod mvpd_w5;
pub mod object;
pub mod statics2;
// WRITER-MVP-A (unverified batches)
pub mod entity_lights;
pub mod entity_prerender;
pub mod entry_exit_add;
pub mod garage_add;
pub mod mvpa1;
pub mod mvpa2;
pub mod mvpa_math;
pub mod mvpc_world1;
pub mod mvpc_world2;
pub mod mvpe_ent1;
pub mod object_data;
pub mod pickup_generate;
pub mod static_matrix;
pub mod zone_create;
pub mod tree_wind;
pub mod object_collision;
pub mod x87_leaves;
pub mod replay_vehicle_block;
// SAVE-1 (docs/arch/save-port.md)
pub mod save_blocks;
pub mod stats_strings;
