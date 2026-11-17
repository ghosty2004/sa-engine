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
//! `--window` runs the same boot and main loop in a native window ([`platform::game_window`]): every `Present` draws the
//! device's back buffer into the window surface, and the window's keyboard / mouse input reaches the game through the
//! platform layer (window messages and DirectInput). Without `--window` the loop runs offscreen with synthetic input.

#![deny(unsafe_code)]

// The platform backends first: on wasm32 their `println!` / `eprintln!` reach every module below.
#[macro_use]
pub mod platform;

pub mod env;

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
pub mod frontend;
pub mod imports;
#[cfg(feature = "mods")]
pub mod mods;
pub mod native;
pub mod perf;
pub mod rw_standin;
pub mod samp;
pub mod samp_assets;
#[cfg(feature = "samp")]
pub mod samp_net;
#[cfg(feature = "samp")]
pub mod samp_saa;
pub mod samp_input;
#[cfg(feature = "samp")]
pub mod samp_ui_demo;
#[cfg(feature = "samp")]
pub mod samp_ui_scene;
pub mod sha256;
pub mod vorbisfile;
pub mod winmain;

pub mod app;
