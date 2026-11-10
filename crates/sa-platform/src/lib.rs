//! Platform layer of the phase B/C engine (PLAN §2 `sa-platform`, FEASIBILITY §5.1).
//!
//! Only this crate (and thin executables) know which target they run on. Everything here that
//! is not under [`native`] is target-independent and builds for `wasm32-unknown-unknown`:
//!
//! * [`fs`]: the game-folder file system ([`fs::GameFs`]): whole-file reads and random-access
//!   handles (the IMG archives), with the game's case-insensitive, backslash-separated paths;
//! * [`input`]: a layout-independent key set and per-frame input state;
//! * [`clock`]: a monotonic clock;
//! * [`spawn_worker`]: background workers (IO/decode threads; the game loop itself stays
//!   single-threaded).
//!
//! [`native`] (feature `native`, not on wasm32) implements them with `std::fs`, `std::thread`,
//! `std::time` and a winit window/event loop.

pub mod clock;
pub mod fs;
pub mod input;

#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
pub mod native;

/// Runs `f` on a background worker named `name`.
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_worker(name: &str, f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new().name(name.to_owned()).spawn(f).expect("spawn worker thread");
}
