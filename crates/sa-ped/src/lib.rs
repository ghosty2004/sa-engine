//! PED subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US, leaf-ward part (PLAN §2 crate `sa-ped`;
//! PORT-WEAPON-PED, `docs/arch/weapon-ped-port.md`): `CPedType`, `CPedStats`, `CPlayerInfo`/`CPlayerPedData`
//! helpers, clothes data, `CPed` accessors and small helpers, `CPedDamageResponseCalculator`, `CWanted`
//! leaves. The big `CPed`/`CPlayerPed` process functions and everything that drives AI tasks come later.
//!
//! Same rules as sa-core, sa-stream and sa-world (ADR-001): game state through `sa_core::globals!`/`GPtr`,
//! calls to other catalogued functions through `sa_core::orig!` with the original address, every port
//! registered with `#[hook]` and listed in its module's `HOOKS` ([`hooks::ALL`]). Ports follow the
//! original bytes, including its bugs (PLAN §4), never gta-reversed's fixes.
#![no_std]
#![deny(unsafe_code)]
// Large `orig!` blocks expand one recursive macro step per declaration.
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod clothes;
pub mod common;
pub mod damage;
pub mod hooks;
pub mod misc;
pub mod ped;
pub mod ped_type;
pub mod ped_weapon;
pub mod player_info;
pub mod player_ped;
pub mod small;
pub mod wanted;
pub mod ped_life;
pub mod ped_render;
pub mod ped_physics;
pub mod player_ctrl;
pub mod ped_control;
pub mod ped_misc3;
pub mod ped_coll;
pub mod player_target;
pub mod ped_entity_collision;
pub mod ped_type_init;
pub mod player_cross_hair;
