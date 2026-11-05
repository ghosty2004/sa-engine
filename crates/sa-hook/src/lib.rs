//! sa-hook: the phase A DLL. `sa-launcher` injects it into gta_sa.exe 1.0 US (suspended) under Wine.
//!
//! - [`table`]: the hook table, keyed by original address, each with a name, the expected entry bytes
//!   and its Rust implementation.
//! - [`x86`]: length decoder + relocator for inline hooks.
//! - [`control`]: TCP control-channel protocol (`ping`, `frames`, `hooks`, `hook`, `peek`, `selftest`).
//! - [`game`]: Rust ports of original functions (pure parts are host-testable).
//! - `win`: DllMain, hook installation, frame-boundary toggles, control server, log (Windows only).

pub mod control;
pub mod game;
pub mod table;
pub mod x86;

#[cfg(test)]
mod testexe;

#[cfg(windows)]
mod win;
