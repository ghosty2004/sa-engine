//! SCRIPT subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PORT-SCRIPT-1, `docs/arch/script-port.md`):
//! the `CRunningScript` VM core ([`vm`]) and the `ProcessCommands0To99` switch ([`cmd0`]).
//!
//! Same rules as sa-core and sa-world (ADR-001): game state through `sa_core::globals!`/`GPtr`, calls
//! to other catalogued functions through `sa_core::orig!` with the original address, every port
//! registered with `#[hook]` and listed in its module's `HOOKS` ([`hooks::ALL`]).
#![no_std]
#![deny(unsafe_code)]
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod cmd0;
pub mod hooks;
pub mod vm;
