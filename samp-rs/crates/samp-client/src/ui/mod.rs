//! In-game UI: chat window and input, local commands, game text, client messages, dialogs, textdraws,
//! menus, scoreboard, nametags, death window and chat bubbles.
//!
//! [`core`] holds the platform-independent state on top of `samp-ui` (tested on the host); [`colours`] and
//! [`inputlock`] are host-testable too. [`render`] is the UI pass and [`session`] the per-frame pass state and
//! mouse routing, both written once and drawn through `sa_samp_api::ui::UiBackend` (sa-engine
//! docs/samp/overlay.md §10). The Windows-only modules are the Direct3D 9 backend (`d3d`, its `renderer`,
//! `dxut`), the original exe's textdraw / menu / material-text / preview code (`textdraws`, `menus`,
//! `mattext`, `preview`), the window subclass (`wndproc`) and the entry points used by `dll` and `client`
//! (`glue`).
//! Spec: docs/re/subsystems/ui-rendering.md and ui-chat-audio.md; notes in docs/phase3/m4-notes.md.

pub mod colours;
pub mod host;
pub mod core;
pub mod inputlock;
pub mod render;
pub mod session;

#[cfg(windows)]
mod d3d;
#[cfg(windows)]
mod dxut;
#[cfg(windows)]
mod glue;
#[cfg(windows)]
pub mod mattext;
#[cfg(windows)]
mod menus;
#[cfg(windows)]
mod preview;
#[cfg(windows)]
mod textdraws;
#[cfg(windows)]
mod wndproc;

#[cfg(windows)]
pub use glue::*;

/// Test hook for the control-channel command `devreset`: the device reports "lost, reset possible" for
/// the next `checks` `TestCooperativeLevel` calls.
#[cfg(windows)]
pub fn force_device_lost(checks: u32) {
    d3d::FORCE_LOST.store(checks, std::sync::atomic::Ordering::SeqCst);
}
