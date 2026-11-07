//! WORLD subsystems of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PLAN §2 crate `sa-world`). So far the
//! collision subsystem COLL (PORT-COLL, `docs/arch/coll-port.md`): `CCollision`, `CCollisionData`,
//! `CColModel`, `CColTrianglePlane`, `SurfaceInfos_c`, `CColAccel`, `CTempColModels`.
//!
//! Same rules as sa-core and sa-stream (ADR-001): game state through `sa_core::globals!`/`GPtr`, calls
//! to other catalogued functions through `sa_core::orig!` with the original address, every port
//! registered with `#[hook]` and listed in its module's `HOOKS` ([`hooks::ALL`]). Memory access only
//! through sa-core's audited accessors, so this crate has no `unsafe` beyond `GameData` impls.
#![no_std]
#![deny(unsafe_code)]
// Large `orig!` blocks expand one recursive macro step per declaration.
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod collision;
pub mod hooks;
