//! `sa-game`: the phase B executable (PLAN §2, F10; docs/arch/platform.md).
//!
//! Runs natively on Linux without Wine and without executing any original code (a 32-bit
//! process, `--target i686-unknown-linux-gnu`): it reads the user's install through sa-platform,
//! maps the legacy address arena with the data sections of the user's own gta_sa.exe ([`arena`]),
//! installs `orig!`'s address registry of every Rust port ([`native`]), binds the ported code's
//! Win32/CRT/DirectInput declarations to sa-platform ([`bind`]) and runs the ported entry points
//! of the original `WinMain` in order ([`boot`]), then a number of main-loop iterations. Every
//! step that is not in Rust, or that reaches an unported callee, is reported instead of crashing.
//!
//! ```text
//! sa-game --game wine/gta-sa [--frames N] [--report out.md] [--window [--exit-after S] [--auto-input]] [--screenshot out.png]
//! ```
//!
//! `--window` runs the same boot and main loop in a native window ([`game_window`]): every `Present` draws the
//! device's back buffer into the window surface, and the window's keyboard / mouse input reaches the game through the
//! platform layer (window messages and DirectInput). Without `--window` the loop runs offscreen with synthetic input.

#![deny(unsafe_code)]

// wasm32: stdout / stderr go nowhere, so the crate's `println!` / `eprintln!` write to the browser console
// (textual-scope macros shadow the std prelude ones for every module declared below).
#[cfg(target_arch = "wasm32")]
macro_rules! println {
    () => { $crate::web::log_line(String::new()) };
    ($($t:tt)*) => { $crate::web::log_line(format!($($t)*)) };
}
#[cfg(target_arch = "wasm32")]
macro_rules! eprintln {
    () => { $crate::web::log_line(String::new()) };
    ($($t:tt)*) => { $crate::web::log_line(format!($($t)*)) };
}

pub mod env;
#[cfg(target_arch = "wasm32")]
pub mod web;
#[cfg(target_arch = "wasm32")]
pub mod webfs;

pub mod arena;
pub mod bind;
pub mod boot;
pub mod cinit;
pub mod com;
pub mod crt;
pub mod crt_fmt;
pub mod d3d9;
pub mod d3d9_shader;
pub mod det;
pub mod detheap;
pub mod dsound;
pub mod dx;
#[cfg(not(target_arch = "wasm32"))]
pub mod game_window;
pub mod imports;
pub mod native;
pub mod rw_standin;
pub mod samp;
pub mod samp_assets;
#[cfg(feature = "samp")]
pub mod samp_net;
#[cfg(feature = "samp")]
pub mod samp_saa;
pub mod samp_input;
pub mod samp_ui_demo;
pub mod samp_ui_scene;
#[cfg(not(target_arch = "wasm32"))]
pub mod screen;
pub mod sha256;
pub mod vorbisfile;
pub mod winmain;

pub mod app;
