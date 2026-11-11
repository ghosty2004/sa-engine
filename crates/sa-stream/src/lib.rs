//! STREAM subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PORT-STREAM, `docs/arch/stream-port.md`):
//! CdStream, CStreaming/CStreamingInfo, CFileLoader, the IPL/TXD/COL stores, every model info class
//! and `CModelInfo`, `CLoadedCarGroup`, `CLoadingScreen`, the RW `Gta` stream readers and the
//! subsystem's static initialisers.
//!
//! A separate crate on top of `sa-core` (ADR-001 §7: "new engine crates depend on sa-core and use
//! `#[hook]` with `sa_core` in scope; sa-hook appends their `hooks::ALL`"), named as PLAN §2 fixes it.
//! Reasons for not putting it into sa-core: STREAM is ~550 functions (more than all of CORE), it is a
//! consumer of CORE and never the other way round (sa-core's calls into STREAM stay `orig!`
//! declarations), and a separate crate keeps later engine crates (sa-world, sa-rw) from depending on
//! all of CORE's ports when they only need streaming, and keeps incremental builds of sa-core small.
//!
//! Same rules as sa-core: game state through `sa_core::globals!`/`GPtr`, calls to other catalogued
//! functions (including RenderWare `Rp*`/`Rw*`, not ported yet) through `sa_core::orig!`, every port
//! registered with `#[hook]` and listed in its module's `HOOKS` ([`hooks::ALL`]). Memory access only
//! through sa-core's audited accessors, so this crate has no `unsafe` beyond `GameData` impls.
#![no_std]
#![deny(unsafe_code)]
// Large `orig!` blocks expand one recursive macro step per declaration.
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod hooks;
pub mod shared;

// --- owner M (main agent)
pub mod cdstream;
pub mod iplstore;
pub mod loaded_car_group;
pub mod name_damage;
pub mod loading_screen;
pub mod misc;
pub mod rw_gta_stream;
pub mod txdstore;
// --- owner A
pub mod colstore;
pub mod streaming;
pub mod streaming_info;
// --- owner C
pub mod fileloader;
// --- owner B (C owns modelinfo::vehicle)
pub mod modelinfo;
pub mod img_size;
pub mod object_instance;

pub mod std6_natives;
