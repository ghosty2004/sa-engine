//! The Rust SDK for sa-engine mods (`docs/arch/ADR-004-mods.md`; how to build and run one:
//! `docs/mods/getting-started.md`, the API: `docs/mods/api.md`).
//!
//! A mod is a `cdylib` crate built for `wasm32-unknown-unknown`. It implements [`Mod`] on a `Default` type and names
//! it with [`entry!`]:
//!
//! ```ignore
//! #[derive(Default)]
//! struct Hello;
//!
//! impl sa_mod::Mod for Hello {
//!     fn overlay(&mut self, _frame: u32, o: &mut sa_mod::Overlay) {
//!         let _ = o.text(8.0, 8.0, 16.0, sa_mod::Rgba::WHITE, "hello");
//!     }
//! }
//!
//! sa_mod::entry!(Hello);
//! ```
//!
//! Everything here is safe; the wasm imports and exports live in the binding layer (`ffi`), generated from the one
//! interface definition in `sa-mod-api`.

use std::cell::RefCell;

pub use sa_mod_api::event::Event;
pub use sa_mod_api::hook::Value;
pub use sa_mod_api::sa_samp_api::{GameState, ObjectRef, PedRef, Vec3, VehicleRef};
pub use sa_mod_api::state::{CameraInfo, PedInfo, PlayerInfo, VehicleInfo, WorldInfo};
pub use sa_mod_api::{API_MAJOR, API_MINOR, event as event_kind, level, status};

#[doc(hidden)]
pub mod ffi;

/// Why a host function refused a call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The mod's manifest does not grant the permission (`status::DENIED`).
    Denied,
    /// An argument was out of range (`status::BAD_ARG`).
    BadArg,
    /// No such entity, function or value (`status::NOT_FOUND`).
    NotFound,
    /// Another negative status.
    Other(i32),
}

fn check(r: i32) -> Result<i32, Error> {
    match r {
        r if r >= 0 => Ok(r),
        status::DENIED => Err(Error::Denied),
        status::BAD_ARG => Err(Error::BadArg),
        status::NOT_FOUND => Err(Error::NotFound),
        r => Err(Error::Other(r)),
    }
}

/// A colour, `0xRRGGBBAA`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba(pub u32);

impl Rgba {
    /// Opaque white.
    pub const WHITE: Rgba = Rgba(0xFFFF_FFFF);
    /// Opaque black.
    pub const BLACK: Rgba = Rgba(0x0000_00FF);

    /// From its components.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Rgba {
        Rgba(u32::from_be_bytes([r, g, b, a]))
    }
}

/// `(ptr, len)` of a byte slice in the mod's linear memory (wasm32 addresses are 32-bit).
fn ptr_len(s: &[u8]) -> (i32, i32) {
    (s.as_ptr() as usize as i32, s.len() as i32)
}

/// Writes `text` to the game's log at `level` (`sa_mod::level`).
pub fn log(level: i32, text: &str) {
    let (p, n) = ptr_len(text.as_bytes());
    // SAFETY: a host function call with plain values; the host reads `text` through bounds-checked copies
    let _ = unsafe { ffi::log(level, p, n) };
}

/// Writes an information line to the game's log.
pub fn info(text: &str) {
    log(level::INFO, text);
}

/// The 2D overlay of the current frame, handed to [`Mod::overlay`]. Coordinates are pixels from the top left of
/// the game's image.
pub struct Overlay {
    _private: (),
}

impl Overlay {
    /// The overlay's size in pixels.
    pub fn size(&self) -> (u32, u32) {
        // SAFETY: host function calls without arguments
        let (w, h) = unsafe { (ffi::screen_width(), ffi::screen_height()) };
        (w.max(0) as u32, h.max(0) as u32)
    }

    /// Fills a rectangle.
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, colour: Rgba) -> Result<(), Error> {
        // SAFETY: a host function call with plain values
        check(unsafe { ffi::overlay_rect(x, y, w, h, colour.0 as i32) }).map(|_| ())
    }

    /// Draws `text` with its top left at (`x`, `y`), `size` pixels per glyph cell; returns its width in pixels.
    pub fn text(&mut self, x: f32, y: f32, size: f32, colour: Rgba, text: &str) -> Result<u32, Error> {
        let (p, n) = ptr_len(text.as_bytes());
        // SAFETY: a host function call; the host reads `text` through a bounds-checked copy
        check(unsafe { ffi::overlay_text(x, y, size, colour.0 as i32, p, n) }).map(|w| w as u32)
    }
}

/// The display and the game's video mode (permission `render`; API 1.1).
pub mod display {
    use super::{Error, check, ffi};

    /// The desktop: natively the monitor's current mode (offscreen the platform's fixed desktop mode), in the
    /// browser the canvas's size on the page in device pixels. `(0, 0)` without the `render` permission.
    pub fn desktop() -> (u32, u32) {
        // SAFETY: host function calls with plain values
        let (w, h) = unsafe { (ffi::display_mode_width(-1), ffi::display_mode_height(-1)) };
        (w.max(0) as u32, h.max(0) as u32)
    }

    /// The display modes the platform offers, `(width, height)`.
    pub fn modes() -> Vec<(u32, u32)> {
        // SAFETY: host function calls with plain values
        let n = unsafe { ffi::display_mode_count() }.max(0);
        (0..n)
            .map(|i| {
                // SAFETY: as above
                let (w, h) = unsafe { (ffi::display_mode_width(i), ffi::display_mode_height(i)) };
                (w.max(0) as u32, h.max(0) as u32)
            })
            .collect()
    }

    /// Requests the game's video mode `width` x `height`, in a window or full screen (natively). In
    /// [`crate::Mod::settings`] it applies from the first frame; later at the next frame boundary, when the adapter
    /// lists the size.
    pub fn request_video_mode(width: u32, height: u32, fullscreen: bool) -> Result<(), Error> {
        let (w, h) = (width.min(i32::MAX as u32) as i32, height.min(i32::MAX as u32) as i32);
        // SAFETY: a host function call with plain values
        check(unsafe { ffi::request_video_mode(w, h, fullscreen as i32) }).map(|_| ())
    }
}

/// The mod's own files (permission `files = "own"`; API 1.1): natively in the mod's folder, in the browser kept for
/// the session. Names are plain file names (`A-Z a-z 0-9 . _ -`), not `mod.toml`, not a `.wasm`.
pub mod files {
    use super::{Error, check, ffi, ptr_len};

    /// The file's bytes, `None` when it does not exist.
    pub fn read(name: &str) -> Result<Option<Vec<u8>>, Error> {
        let (np, nl) = ptr_len(name.as_bytes());
        let mut buf = vec![0u8; 4096];
        loop {
            let (bp, bl) = (buf.as_mut_ptr() as usize as i32, buf.len() as i32);
            // SAFETY: a host function call; the host writes at most `bl` bytes at `bp`, inside `buf`
            let r = unsafe { ffi::file_read(np, nl, bp, bl) };
            if r == crate::status::NOT_FOUND {
                return Ok(None);
            }
            let size = check(r)? as usize;
            if size <= buf.len() {
                buf.truncate(size);
                return Ok(Some(buf));
            }
            buf.resize(size, 0);
        }
    }

    /// Writes (replaces) the file.
    pub fn write(name: &str, data: &[u8]) -> Result<(), Error> {
        let (np, nl) = ptr_len(name.as_bytes());
        let (dp, dl) = ptr_len(data);
        // SAFETY: a host function call; the host reads both ranges through bounds-checked copies
        check(unsafe { ffi::file_write(np, nl, dp, dl) }).map(|_| ())
    }
}

/// Game events (API 1.2; `docs/mods/api.md`): subscribe in [`Mod::init`], receive them in [`Mod::event`].
pub mod events {
    use super::ffi;

    /// Subscribes to the events of `kinds` (`sa_mod::event_kind::*`; replaces the earlier subscription). Returns
    /// the kinds granted as a mask (`1 << kind`): frame events always, game-state events with `state = "read"`, key
    /// events with `input = "read"`.
    pub fn subscribe(kinds: &[i32]) -> i32 {
        let mask = kinds.iter().filter(|k| (0..31).contains(*k)).fold(0, |m, k| m | 1 << k);
        // SAFETY: a host function call with a plain value
        unsafe { ffi::events_subscribe(mask) }
    }
}

/// Read-only game state (API 1.2, permission `state = "read"`; keys: `input = "read"`). Every function reads the
/// game as it is at the moment of the call.
pub mod state {
    use sa_mod_api::state::{Words, from_bytes};

    use super::*;

    fn record<T: Words>(f: impl FnOnce(i32, i32) -> i32) -> Option<T> {
        let mut buf = vec![0u8; 4 * T::WORDS];
        let (p, n) = (buf.as_mut_ptr() as usize as i32, buf.len() as i32);
        check(f(p, n)).ok()?;
        from_bytes(&buf)
    }

    fn handles(f: impl Fn(i32, i32) -> i32) -> Vec<u32> {
        let mut buf = vec![0u8; 4 * 256];
        loop {
            let (p, n) = (buf.as_mut_ptr() as usize as i32, buf.len() as i32);
            let Ok(count) = check(f(p, n)) else { return Vec::new() };
            let count = count as usize;
            if 4 * count <= buf.len() {
                return buf.as_chunks::<4>().0.iter().take(count).map(|c| u32::from_le_bytes(*c)).collect();
            }
            buf.resize(4 * count, 0);
        }
    }

    /// The game state machine.
    pub fn game_state() -> Option<GameState> {
        // SAFETY: a host function call without arguments
        check(unsafe { ffi::game_state() }).ok().map(GameState::from_raw)
    }

    /// The local player (`handle` is `None` while there is no player ped).
    pub fn player() -> Option<PlayerInfo> {
        // SAFETY: the host writes at most `n` bytes at `p`, inside the record's buffer
        record(|p, n| unsafe { ffi::player_info(p, n) })
    }

    /// A ped; `None` when the handle names no live ped.
    pub fn ped(ped: PedRef) -> Option<PedInfo> {
        // SAFETY: as in `player`
        record(|p, n| unsafe { ffi::ped_info(ped.0 as i32, p, n) })
    }

    /// A vehicle; `None` when the handle names no live vehicle.
    pub fn vehicle(vehicle: VehicleRef) -> Option<VehicleInfo> {
        // SAFETY: as in `player`
        record(|p, n| unsafe { ffi::vehicle_info(vehicle.0 as i32, p, n) })
    }

    /// Time, weather and the world.
    pub fn world() -> Option<WorldInfo> {
        // SAFETY: as in `player`
        record(|p, n| unsafe { ffi::world_info(p, n) })
    }

    /// The camera.
    pub fn camera() -> Option<CameraInfo> {
        // SAFETY: as in `player`
        record(|p, n| unsafe { ffi::camera_info(p, n) })
    }

    /// Every ped in the ped pool.
    pub fn peds() -> Vec<PedRef> {
        // SAFETY: the host writes at most `n` bytes at `p`, inside the buffer
        handles(|p, n| unsafe { ffi::ped_list(p, n) }).into_iter().map(PedRef).collect()
    }

    /// Every vehicle in the vehicle pool.
    pub fn vehicles() -> Vec<VehicleRef> {
        // SAFETY: as in `peds`
        handles(|p, n| unsafe { ffi::vehicle_list(p, n) }).into_iter().map(VehicleRef).collect()
    }

    /// Whether the key with Windows virtual-key code `vk` is held (permission `input = "read"`).
    pub fn key_down(vk: u8) -> bool {
        // SAFETY: a host function call with a plain value
        unsafe { ffi::key_down(i32::from(vk)) == 1 }
    }
}

/// Before / after hooks on game functions (API 1.2, permission `hooks = [...]`): subscribe in [`Mod::init`],
/// receive the calls in [`Mod::hook`]. Read-only: a hook sees the values, it cannot change them.
pub mod hooks {
    use super::*;

    /// A hook, as [`subscribe`] returned it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct HookId(pub u32);

    /// When a hook runs.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Phase {
        /// Before the function runs.
        Before,
        /// After it returned.
        After,
    }

    /// Hooks the game function `name` (its name in the hook registry, listed in the manifest's `hooks`), before
    /// and / or after it runs. `NotFound`: the registry has no such function; `Denied`: the manifest does not
    /// name it.
    pub fn subscribe(name: &str, before: bool, after: bool) -> Result<HookId, Error> {
        let (p, n) = ptr_len(name.as_bytes());
        let phases = if before { sa_mod_api::hook::BEFORE } else { 0 } | if after { sa_mod_api::hook::AFTER } else { 0 };
        // SAFETY: a host function call; the host reads `name` through a bounds-checked copy
        check(unsafe { ffi::hook_subscribe(p, n, phases) }).map(|id| HookId(id as u32))
    }

    /// The hooked call [`Mod::hook`] runs for.
    pub struct Call {
        pub(crate) _private: (),
    }

    impl Call {
        /// Its number of arguments.
        pub fn arg_count(&self) -> usize {
            // SAFETY: a host function call without arguments
            unsafe { ffi::hook_arg_count() }.max(0) as usize
        }

        fn value(&self, index: i32) -> Option<Value> {
            let mut word = 0u32;
            // SAFETY: the host writes 4 bytes at the address of `word`
            let kind = unsafe { ffi::hook_value(index, (&raw mut word) as usize as i32) };
            check(kind).ok().and_then(|k| Value::decode(k, word))
        }

        /// Argument `i`.
        pub fn arg(&self, i: usize) -> Option<Value> {
            self.value(i32::try_from(i).ok()?)
        }

        /// The result (after the call, of a function that returns a value).
        pub fn result(&self) -> Option<Value> {
            self.value(-1)
        }
    }
}

/// A mod: its callbacks, called by the host on the game thread in the load order.
pub trait Mod: 'static {
    /// Called once after loading. An error refuses the mod (its message goes to the log).
    fn init(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// The overlay phase of a frame (after the game's own drawing, before the present). `frame` counts the
    /// overlay calls since the mod was loaded.
    fn overlay(&mut self, frame: u32, overlay: &mut Overlay) {
        let _ = (frame, overlay);
    }

    /// The settings phase (API 1.1): once at start-up, before the game selects its video mode and creates its
    /// D3D device ([`display::request_video_mode`] applies from the first frame).
    fn settings(&mut self) {}

    /// The drawable the game is shown in (the native window's client area, the browser canvas in device pixels)
    /// changed size (API 1.1).
    fn resize(&mut self, width: u32, height: u32) {
        let _ = (width, height);
    }

    /// A game event the mod subscribed to ([`events::subscribe`]; API 1.2).
    fn event(&mut self, event: Event) {
        let _ = event;
    }

    /// A hooked game function runs ([`hooks::subscribe`]; API 1.2). Called in the middle of the game's frame.
    fn hook(&mut self, hook: hooks::HookId, phase: hooks::Phase, call: &hooks::Call) {
        let _ = (hook, phase, call);
    }
}

/// The entry points behind [`entry!`]'s exports (generic over the mod type).
#[doc(hidden)]
pub mod rt {
    use super::*;

    thread_local! {
        static INSTANCE: RefCell<Option<Box<dyn Mod>>> = const { RefCell::new(None) };
    }

    pub fn sa_api_version<T>() -> i32 {
        sa_mod_api::pack_version(API_MAJOR, API_MINOR)
    }

    pub fn sa_init<T: Mod + Default>() -> i32 {
        // a panic aborts the mod (a trap, which disables it): its message goes to the game's log first
        std::panic::set_hook(Box::new(|info| log(level::ERROR, &info.to_string())));
        let mut m = T::default();
        match m.init() {
            Ok(()) => {
                INSTANCE.with_borrow_mut(|i| *i = Some(Box::new(m)));
                0
            }
            Err(e) => {
                log(level::ERROR, &e);
                1
            }
        }
    }

    fn with(f: impl FnOnce(&mut dyn Mod)) -> i32 {
        INSTANCE.with_borrow_mut(|i| {
            if let Some(m) = i.as_mut() {
                f(m.as_mut());
            }
        });
        0
    }

    pub fn sa_overlay<T>(frame: i32) -> i32 {
        with(|m| m.overlay(frame as u32, &mut Overlay { _private: () }))
    }

    pub fn sa_settings<T>() -> i32 {
        with(|m| m.settings())
    }

    pub fn sa_resize<T>(width: i32, height: i32) -> i32 {
        with(|m| m.resize(width.max(0) as u32, height.max(0) as u32))
    }

    pub fn sa_event<T>(kind: i32, a: i32, b: i32) -> i32 {
        match Event::decode(kind, a, b) {
            Some(e) => with(|m| m.event(e)),
            None => 0,
        }
    }

    pub fn sa_hook<T>(hook: i32, phase: i32) -> i32 {
        let phase = if phase == sa_mod_api::hook::AFTER { hooks::Phase::After } else { hooks::Phase::Before };
        with(|m| m.hook(hooks::HookId(hook as u32), phase, &hooks::Call { _private: () }))
    }
}

/// Exports the mod's entry points for the `Default` type `$t` implementing [`Mod`].
#[macro_export]
macro_rules! entry {
    ($t:ty) => {
        #[doc(hidden)]
        pub mod __sa_mod_entry {
            #[allow(unused_imports)]
            use super::*;
            use $crate::__sa_mod_exports as __sa_mod_cb;
            $crate::__api::for_each_export!(__sa_mod_cb [$t]);
        }
    };
}

#[doc(hidden)]
pub use sa_mod_api as __api;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Probe;
    impl Mod for Probe {}
    entry!(Probe);

    #[test]
    fn colours_and_status() {
        assert_eq!(Rgba::new(0x12, 0x34, 0x56, 0x78).0, 0x1234_5678);
        assert_eq!(check(5), Ok(5));
        assert_eq!(check(status::DENIED), Err(Error::Denied));
        assert_eq!(__sa_mod_entry::sa_init(), 0);
        assert_eq!(__sa_mod_entry::sa_overlay(0), 0);
        assert_eq!(__sa_mod_entry::sa_settings(), 0);
        assert_eq!(__sa_mod_entry::sa_resize(640, 480), 0);
        assert_eq!(__sa_mod_entry::sa_event(event_kind::KEY_DOWN, 0x41, 0), 0);
        assert_eq!(__sa_mod_entry::sa_hook(0, sa_mod_api::hook::AFTER), 0);
        assert!(state::peds().is_empty());
        assert_eq!(display::desktop(), (0, 0));
        assert_eq!(sa_mod_api::unpack_version(__sa_mod_entry::sa_api_version()), (API_MAJOR, API_MINOR));
    }
}
