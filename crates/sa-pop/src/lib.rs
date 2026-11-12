//! POP subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PLAN §2 crate `sa-pop`, PORT-POP,
//! `docs/arch/pop-port.md`): population, popcycle, car control, car AI, car generators, road blocks,
//! gangs and gang wars, and the small POP-catalogued managers (`CDarkel`, `CTagManager`,
//! `CTrafficLights`, `CAccidentManager`).
//!
//! Same rules as sa-core, sa-stream and sa-world (ADR-001): game state through `sa_core::globals!`/`GPtr`,
//! calls to other catalogued functions through `sa_core::orig!` with the original address, every port
//! registered with `#[hook]` and listed in its module's `HOOKS` ([`hooks::ALL`]). Ported from the 1.0 US
//! bytes; gta-reversed and `docs/re/specs/POP/*.md` are reading aids. Where they differ, the bytes win.
#![no_std]
#![deny(unsafe_code)]
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod common;
pub mod hooks;

pub mod carctrl;
pub mod cargen;
pub mod gangs;
pub mod misc;
pub mod popcycle;
pub mod population;
pub mod statics;
pub mod statics_gen;

pub mod carctrl2;
pub mod misc2;
pub mod pathfind;
pub mod population2;

pub mod carctrl3;
pub mod carctrl4;
pub mod carctrl5;
pub mod path_car_creation_nodes;
pub mod gang_war_release_peds;
pub mod misc3;
pub mod misc4;
pub mod population3;

pub mod carctrl6;
pub mod carctrl7;
pub mod misc5;
pub mod misc6;
pub mod population4;
pub mod generated;
pub mod car_ai_reason_to_stop;
