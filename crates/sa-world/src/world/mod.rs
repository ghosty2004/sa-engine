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
pub mod misc;
pub mod pickups;
pub mod restart;
pub mod shopping;
pub mod statics;
pub mod stats;
pub mod zones;
// PORT-WORLD-2 (docs/arch/world-port.md, part 2)
pub mod cworld2;
pub mod entity;
pub mod misc2;
pub mod object;
pub mod statics2;
