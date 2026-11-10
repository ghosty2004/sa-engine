//! Native implementation: `std::fs` game folder ([`NativeFs`]) and a winit window/event loop
//! ([`run_windowed`]).

mod fs;
mod window;

pub use fs::NativeFs;
pub use window::{WindowApp, WindowConfig, run_windowed};
/// The native window type handed to [`WindowApp::init`] (implements the raw window handle
/// traits wgpu needs for a surface).
pub use winit::window::Window;
