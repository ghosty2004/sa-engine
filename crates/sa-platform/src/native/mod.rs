//! Native implementation: `std::fs` game folder ([`NativeFs`]), a winit window/event loop
//! ([`run_windowed`]), game pads through gilrs (feature `gamepad`) and audio through cpal (feature
//! `audio`).

#[cfg(feature = "audio")]
mod audio;
mod fs;
#[cfg(feature = "gamepad")]
mod gamepad;
mod window;

#[cfg(feature = "audio")]
pub use audio::CpalAudio;
pub use fs::NativeFs;
#[cfg(feature = "gamepad")]
pub use gamepad::Pads;
pub use window::{WindowApp, WindowConfig, map_key, run_windowed, set_pointer_capture};
/// The native window type handed to [`WindowApp::init`] (implements the raw window handle
/// traits wgpu needs for a surface).
pub use winit::window::Window;

/// The per-user folder that stands in for `My Documents` (`SHGetFolderPathA(CSIDL_PERSONAL)`):
/// `$XDG_DATA_HOME/sa-engine/documents`, else `~/.local/share/sa-engine/documents`.
pub fn documents_dir() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("sa-engine").join("documents")
}
