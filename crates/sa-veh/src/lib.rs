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
pub mod statics2;
pub mod train2;
pub mod transmission;
pub mod vehicle2;
