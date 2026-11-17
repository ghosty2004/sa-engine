//! The per-platform backends (ADR-003 §3.1; the only place of this crate with platform cfgs): the wasm engine
//! ([`crate::engine::Engine`]) and where mods are read from ([`crate::ModSource`]).

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::{Wasmtime as DefaultEngine, sources};

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::{Browser as DefaultEngine, sources};

#[cfg(any(target_arch = "wasm32", test))]
pub mod meter;
