//! The Rust SDK for sa-engine mods (MOD-2, `docs/arch/ADR-004-mods.md`; how to build and run one:
//! `docs/mods/getting-started.md`).
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

pub use sa_mod_api::{API_MAJOR, API_MINOR, level, status};

#[doc(hidden)]
pub mod ffi;

/// Why a host function refused a call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The mod's manifest does not grant the permission (`status::DENIED`).
    Denied,
    /// An argument was out of range (`status::BAD_ARG`).
    BadArg,
    /// Another negative status.
    Other(i32),
}

fn check(r: i32) -> Result<i32, Error> {
    match r {
        r if r >= 0 => Ok(r),
        status::DENIED => Err(Error::Denied),
        status::BAD_ARG => Err(Error::BadArg),
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

    pub fn sa_overlay<T>(frame: i32) -> i32 {
        INSTANCE.with_borrow_mut(|i| {
            if let Some(m) = i.as_mut() {
                m.overlay(frame as u32, &mut Overlay { _private: () });
            }
        });
        0
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
        assert_eq!(sa_mod_api::unpack_version(__sa_mod_entry::sa_api_version()), (API_MAJOR, API_MINOR));
    }
}
