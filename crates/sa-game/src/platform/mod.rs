//! The platform backends of sa-game (ADR-003 §3.1): the native executable ([`native_main`], [`game_window`],
//! [`screen`]) and the browser page ([`web`], [`webfs`]). The only place in sa-game that selects code by target;
//! the session, the boot and the frame pipeline ([`crate::app`], [`crate::frontend`]) are shared.

// wasm32: stdout / stderr go nowhere, so the crate's `println!` / `eprintln!` write to the browser console
// (textual-scope macros shadow the std prelude ones for every module declared after `platform` in lib.rs).
#[cfg(target_arch = "wasm32")]
macro_rules! println {
    () => { $crate::platform::web::log_line(String::new()) };
    ($($t:tt)*) => { $crate::platform::web::log_line(format!($($t)*)) };
}
#[cfg(target_arch = "wasm32")]
macro_rules! eprintln {
    () => { $crate::platform::web::log_line(String::new()) };
    ($($t:tt)*) => { $crate::platform::web::log_line(format!($($t)*)) };
}

pub mod perf_probe;
pub mod registry_entry;

#[cfg(not(target_arch = "wasm32"))]
pub mod game_window;
#[cfg(not(target_arch = "wasm32"))]
pub mod native_main;
#[cfg(not(target_arch = "wasm32"))]
pub mod screen;
#[cfg(target_arch = "wasm32")]
pub mod web;
#[cfg(target_arch = "wasm32")]
pub mod webfs;

/// How the phase B registry dispatches on this target (the boot line's note).
pub const REGISTRY_DISPATCH: &str = if cfg!(target_arch = "wasm32") {
    "; wasm32: type-erased entries (sa_core::wabi)"
} else if cfg!(target_arch = "x86") {
    ""
} else {
    "; NOT dispatched on this target (build for i686-unknown-linux-gnu)"
};

/// Whether the main loop stops after the same failure three times in a row (natively; in the browser one unported
/// path should not end the session: the page keeps the last frame and shows the blocked function).
pub const STOP_ON_REPEATED_FAILURE: bool = !cfg!(target_arch = "wasm32");
