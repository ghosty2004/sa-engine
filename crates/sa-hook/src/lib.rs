//! sa-hook: the phase A DLL. `sa-launcher` injects it into gta_sa.exe 1.0 US (suspended) under Wine.
//!
//! - [`table`]: the hook table, keyed by original address, each with a name, the expected entry bytes
//!   and its Rust implementation.
//! - [`x86`]: length decoder + relocator for inline hooks.
//! - [`control`]: TCP control-channel protocol (`ping`, `frames`, `hooks`, `hook`, `peek`, `selftest`,
//!   `hashes`, `det`, `record`, `replay`).
//! - [`game`]: Rust ports of original functions (pure parts are host-testable).
//! - [`oracle`]: determinism mode, input recording/replay and frame-state hashing (INF-4/5/6):
//!   configuration, virtual clock, input state machine.
//! - `win`: DllMain, hook installation, frame-boundary toggles, control server, log (Windows only).
//! - `detwin`: the Windows side of the oracle features (Windows only).
//! - `cdsync`: lost-wake-up fix for `CdStreamSync` (INF-15, Windows only).
//! - [`l2`] / `l2win`: the L2 oracle (INF-8): original vs Rust at real call sites, see
//!   `docs/re/l2-oracle.md`.
//! - [`coverage`]: execution coverage of the original code (INF-10, first-hit breakpoints), see
//!   `docs/re/l3-coverage.md`.

pub mod arena;
pub mod control;
pub mod coverage;
pub mod game;
pub mod l2;
pub mod oracle;
pub mod table;
pub mod x86;

#[cfg(test)]
mod callers;
#[cfg(test)]
mod testexe;

#[cfg(windows)]
mod cdsync;
#[cfg(windows)]
mod detaudio;
#[cfg(windows)]
mod detwin;
#[cfg(windows)]
mod l2win;
#[cfg(windows)]
mod win;
