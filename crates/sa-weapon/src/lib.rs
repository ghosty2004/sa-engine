//! WEAPON subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PLAN §2 crate `sa-weapon`; PORT-WEAPON-PED,
//! `docs/arch/weapon-ped-port.md`): `CWeapon`, `CWeaponInfo`, projectiles, `CBulletInfo`, `CShotInfo`,
//! `CExplosion`, `CFire`/`CFireManager`, `CCreepingFire`, water cannons, weapon effects.
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

pub mod bullet_shot;
pub mod common;
pub mod effects;
pub mod explosion;
pub mod fire;
pub mod hooks;
pub mod projectile;
pub mod statics;
pub mod water_cannon;
pub mod weapon;
pub mod weapon_fire;
pub mod weapon_impact;
pub mod weapon_info;
