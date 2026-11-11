//! Platform layer of the phase B/C engine (PLAN §2 `sa-platform`, FEASIBILITY §5.1,
//! docs/arch/platform.md).
//!
//! Only this crate (and thin executables) know which target they run on. Everything here that
//! is not under [`native`] is target-independent and builds for `wasm32-unknown-unknown`:
//!
//! * [`fs`]: the game-folder file system ([`fs::GameFs`]): whole-file reads and random-access
//!   handles (the IMG archives), with the game's case-insensitive, backslash-separated paths;
//! * [`vfs`]: the Windows namespace the game sees (game folder, current directory, My Documents
//!   user-files folder) and the CRT/Win32 open-file table behind `CFileMgr` and `CdStream`;
//! * [`input`]: a layout-independent key set, per-frame input state and the DirectInput 8
//!   mouse/joystick records `CPad` reads;
//! * [`win32`]: the Win32 semantics the ported code relies on (window messages, virtual keys,
//!   `MapVirtualKeyA`, `GetKeyState`, `QueryPerformanceCounter`/`timeGetTime`);
//! * [`clock`]: a monotonic clock;
//! * [`kernel`]: kernel32 objects (semaphores, threads, `LocalAlloc`) for `CdStream`;
//! * [`audio`]: the PCM sink of the phase B mixer;
//! * [`gpu`]: the `IDirect3DDevice9` seam the ported RW D3D9 driver calls (implemented on wgpu by
//!   `sa-rw-wgpu`);
//! * [`spawn_worker`]: background workers (IO/decode threads; the game loop itself stays
//!   single-threaded).
//!
//! [`native`] (feature `native`, not on wasm32) implements the OS side with `std::fs`,
//! `std::thread`, `std::time`, a winit window/event loop, gilrs pads (feature `gamepad`) and cpal
//! audio (feature `audio`).

pub mod audio;
pub mod clock;
pub mod fs;
pub mod gpu;
pub mod input;
pub mod kernel;
pub mod vfs;
pub mod win32;

#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
pub mod native;

/// Runs `f` on a background worker named `name`.
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_worker(name: &str, f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(f)
        .expect("spawn worker thread");
}
