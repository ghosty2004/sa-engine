//! The engine seam (ADR-003 §3.1): one host, two wasm engines behind these traits: wasmtime natively
//! (`backend::native`), the browser's WebAssembly on wasm32 (`backend::web`). A backend instantiates a checked module
//! with the host functions, runs entry points under a fuel budget and a memory cap, and reports traps; everything
//! else (manifests, checks, the API itself, scheduling, disabling) is the host's, written once.

use sa_mod_api::Val;

use crate::api::CallCtx;
use crate::manifest::Limits;

/// Why a call into a mod failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallError {
    /// The call used up its fuel (an endless or too long computation).
    OutOfFuel,
    /// A trap (`unreachable`, a Rust panic, an out-of-bounds access, a stack overflow, ...) or a host trap.
    Trap(String),
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallError::OutOfFuel => write!(f, "fuel budget exceeded"),
            CallError::Trap(m) => write!(f, "trap: {m}"),
        }
    }
}

/// A wasm engine.
pub trait Engine {
    /// Instantiates a module that passed [`crate::module::check`], with the host functions of
    /// `sa_mod_api::IMPORTS` and `limits` (memory cap; instantiation runs under the init fuel).
    fn instantiate(&self, wasm: &[u8], limits: &Limits) -> Result<Box<dyn Instance>, String>;
    /// The engine's name, for the log.
    fn name(&self) -> &'static str;
}

/// An instantiated mod.
pub trait Instance {
    /// Calls the export `name` (an `sa_mod_api::EXPORTS` entry returning `i32`) with `args` and at most `fuel`
    /// fuel; the host functions it calls run [`crate::api::dispatch`] on `ctx`.
    fn call(&mut self, name: &str, args: &[Val], fuel: u64, ctx: &mut CallCtx) -> Result<i32, CallError>;
}
