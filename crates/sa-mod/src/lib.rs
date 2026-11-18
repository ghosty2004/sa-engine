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
pub use sa_mod_api::sa_types::text::FontText;
pub use sa_mod_api::sa_types::{GameState, ObjectRef, PedRef, Vec3, VehicleRef};
pub use sa_mod_api::state::{CameraInfo, MatrixInfo, PadInfo, PedInfo, PlayerInfo, VehicleInfo, WorldInfo};
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
    /// Not allowed in this phase, e.g. inside a hook (`status::WRONG_PHASE`).
    WrongPhase,
    /// The game could not do it; the log says why (`status::FAILED`).
    Failed,
    /// Nothing now, try again later (`status::AGAIN`).
    Again,
    /// Another negative status.
    Other(i32),
}

fn check(r: i32) -> Result<i32, Error> {
    match r {
        r if r >= 0 => Ok(r),
        status::DENIED => Err(Error::Denied),
        status::BAD_ARG => Err(Error::BadArg),
        status::NOT_FOUND => Err(Error::NotFound),
        status::WRONG_PHASE => Err(Error::WrongPhase),
        status::FAILED => Err(Error::Failed),
        status::AGAIN => Err(Error::Again),
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

    /// Draws `quads` of `image` (`None`: plain colour), in order (API 1.6); returns how many were inside the clip
    /// rectangle.
    pub fn quads(&mut self, image: Option<&Image>, quads: &[Quad]) -> Result<u32, Error> {
        let mut drawn = 0;
        for chunk in quads.chunks(sa_mod_api::image::MAX_QUADS_PER_CALL) {
            let words: Vec<u32> = chunk.iter().flat_map(Quad::words).collect();
            let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            let handle = image.map_or(sa_mod_api::image::NONE, |i| i.handle);
            // SAFETY: a host function call; the host reads the quads through a bounds-checked copy
            drawn += check(unsafe { ffi::overlay_quads(handle, bytes.as_ptr() as usize as i32, chunk.len() as i32) })?;
        }
        Ok(drawn as u32)
    }

    /// Prints `text` in the game's own font with `style`'s `CFont` state; the game draws it in its next frame's 2D
    /// phase, under the overlay (API 1.6).
    pub fn game_text(&mut self, style: &sa_mod_api::sa_types::text::FontText, text: &[u8]) -> Result<(), Error> {
        let words = sa_mod_api::text::encode(style);
        let (t, n) = ptr_len(text);
        // SAFETY: a host function call; the host reads the record and the text through bounds-checked copies
        check(unsafe { ffi::game_text(words.as_ptr() as usize as i32, t, n) }).map(|_| ())
    }

    /// Draws `texture` into `rect` (`[x, y, w, h]`) tinted with `colour`, as the game draws its sprites, in the next
    /// frame's 2D phase (API 1.6).
    pub fn game_sprite(&mut self, texture: GameTexture, rect: [f32; 4], colour: Rgba) -> Result<(), Error> {
        let [x, y, w, h] = rect;
        // SAFETY: a host function call with plain values
        check(unsafe { ffi::game_sprite(texture.0, x, y, w, h, colour.0 as i32) }).map(|_| ())
    }

    /// Cuts the following drawing of this overlay phase to a rectangle (`None`: no clip; API 1.6).
    pub fn clip(&mut self, rect: Option<[f32; 4]>) -> Result<(), Error> {
        let [x, y, w, h] = rect.unwrap_or([0.0, 0.0, -1.0, -1.0]);
        // SAFETY: a host function call with plain values
        check(unsafe { ffi::overlay_clip(x, y, w, h) }).map(|_| ())
    }
}

/// A quad of an overlay image ([`Overlay::quads`]): the screen rectangle, the image's texture coordinates
/// `[u0, v0, u1, v1]` (0..=1) and a tint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    /// Left, top, width, height in pixels.
    pub rect: [f32; 4],
    /// Texture coordinates.
    pub uv: [f32; 4],
    /// Tint (multiplies the image).
    pub colour: Rgba,
}

impl Quad {
    fn words(&self) -> [u32; sa_mod_api::image::QUAD_WORDS] {
        let [x, y, w, h] = self.rect;
        let [u0, v0, u1, v1] = self.uv;
        let f = [x, y, w, h, u0, v0, u1, v1].map(f32::to_bits);
        [f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], self.colour.0]
    }
}

/// A texture of the game's texture dictionaries (API 1.6), drawn with [`Overlay::game_sprite`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameTexture(i32);

impl GameTexture {
    /// The texture `name` of the game's dictionary `dict`; without a dictionary of that name, the game folder's
    /// `file` (`None`: none) is loaded into a new one. `NotFound` when the file or the texture is not there.
    pub fn find(dict: &str, file: Option<&str>, name: &str) -> Result<GameTexture, Error> {
        let (d, dn) = ptr_len(dict.as_bytes());
        let (f, fl) = ptr_len(file.unwrap_or("").as_bytes());
        let (n, nl) = ptr_len(name.as_bytes());
        // SAFETY: a host function call; the host reads the names through bounds-checked copies
        check(unsafe { ffi::game_texture(d, dn, f, fl, n, nl) }).map(GameTexture)
    }
}

/// An overlay image (API 1.6): RGBA8 pixels kept by the host and drawn with [`Overlay::quads`]; deleted when
/// dropped.
#[derive(Debug)]
pub struct Image {
    handle: i32,
    size: (u32, u32),
}

impl Image {
    /// Uploads `width` x `height` RGBA8 pixels (`rgba`: rows from the top, straight alpha); `smooth`: linear
    /// filtering over a box-filtered mip chain when scaled, else the nearest texel. `Failed` beyond the host's
    /// budget (`sa_mod_api::image`).
    pub fn new(width: u32, height: u32, smooth: bool, rgba: &[u8]) -> Result<Image, Error> {
        let (p, n) = ptr_len(rgba);
        let flags = if smooth { sa_mod_api::image::SMOOTH } else { 0 };
        // SAFETY: a host function call; the host reads the pixels through a bounds-checked copy
        let handle = check(unsafe { ffi::image_create(width as i32, height as i32, flags, p, n) })?;
        Ok(Image { handle, size: (width, height) })
    }

    /// Its size in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.size
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        // SAFETY: a host function call with a plain value
        let _ = unsafe { ffi::image_destroy(self.handle) };
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
    use super::{Error, check, ffi, ptr_len, read_sized};

    /// The file's bytes, `None` when it does not exist.
    pub fn read(name: &str) -> Result<Option<Vec<u8>>, Error> {
        let (np, nl) = ptr_len(name.as_bytes());
        // SAFETY: a host function call; the host writes at most `bl` bytes at `bp`, inside the buffer
        read_sized(|bp, bl| unsafe { ffi::file_read(np, nl, bp, bl) })
    }

    /// Writes (replaces) the file.
    pub fn write(name: &str, data: &[u8]) -> Result<(), Error> {
        let (np, nl) = ptr_len(name.as_bytes());
        let (dp, dl) = ptr_len(data);
        // SAFETY: a host function call; the host reads both ranges through bounds-checked copies
        check(unsafe { ffi::file_write(np, nl, dp, dl) }).map(|_| ())
    }
}

/// Reads a value of unknown size into a growing buffer: `call(ptr, len)` copies at most `len` bytes and returns the
/// whole size; `None` for `NOT_FOUND`.
fn read_sized(mut call: impl FnMut(i32, i32) -> i32) -> Result<Option<Vec<u8>>, Error> {
    let mut buf = vec![0u8; 4096];
    loop {
        let (bp, bl) = (buf.as_mut_ptr() as usize as i32, buf.len() as i32);
        let r = call(bp, bl);
        if r == status::NOT_FOUND {
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

/// The host's clock (API 1.4).
pub mod time {
    use super::ffi;

    /// What the clock reads.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Now {
        /// Microseconds since the host started (monotonic).
        pub monotonic_us: u64,
        /// Microseconds since 1970-01-01 UTC (the wall clock).
        pub unix_us: u64,
    }

    /// Now.
    pub fn now() -> Now {
        let mut out = [0u8; 16];
        // SAFETY: a host function call; the host writes 16 bytes at `out`
        unsafe { ffi::clock(out.as_mut_ptr() as usize as i32) };
        let word = |i: usize| u64::from_le_bytes(out[i..i + 8].try_into().unwrap_or_default());
        Now { monotonic_us: word(0), unix_us: word(8) }
    }
}

/// UDP sockets (permission `net = ["udp:HOST:PORT", ...]`; API 1.4): connected and non-blocking; natively real UDP
/// sockets, in the browser relayed by the page's UDP proxy. Poll them every frame ([`Mod::overlay`] or a
/// `FRAME_BEGIN` event).
pub mod net {
    use super::{Error, check, ffi, ptr_len};
    pub use sa_mod_api::net::{MAX_DATAGRAM, MAX_SOCKETS};

    /// Where a socket stands.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum State {
        /// Opening (in the browser: the WebSocket to the proxy); datagrams sent now are queued.
        Connecting,
        /// Datagrams flow.
        Open,
        /// Closed or refused (the log says why).
        Closed,
    }

    /// A UDP socket connected to one target; closed when dropped.
    #[derive(Debug)]
    pub struct Socket(i32);

    impl Socket {
        /// Opens a socket to `target` (`HOST:PORT`), which a rule of the manifest's `net` list must allow.
        pub fn open(target: &str) -> Result<Socket, Error> {
            let (p, n) = ptr_len(target.as_bytes());
            // SAFETY: a host function call; the host reads `target` through a bounds-checked copy
            check(unsafe { ffi::net_open(p, n) }).map(Socket)
        }

        /// Its state.
        pub fn state(&self) -> State {
            // SAFETY: a host function call with a plain value
            match unsafe { ffi::net_state(self.0) } {
                sa_mod_api::net::CONNECTING => State::Connecting,
                sa_mod_api::net::OPEN => State::Open,
                _ => State::Closed,
            }
        }

        /// Sends one datagram; [`Error::Again`] when the frame's send budget is used up.
        pub fn send(&self, datagram: &[u8]) -> Result<usize, Error> {
            let (p, n) = ptr_len(datagram);
            // SAFETY: a host function call; the host reads the datagram through a bounds-checked copy
            check(unsafe { ffi::net_send(self.0, p, n) }).map(|n| n as usize)
        }

        /// Takes one received datagram into `buf` (truncated to it); [`Error::Again`] when none is queued.
        pub fn recv(&self, buf: &mut [u8]) -> Result<usize, Error> {
            let (p, n) = (buf.as_mut_ptr() as usize as i32, buf.len() as i32);
            // SAFETY: a host function call; the host writes at most `n` bytes at `p`, inside `buf`
            check(unsafe { ffi::net_recv(self.0, p, n) }).map(|n| n as usize)
        }
    }

    impl Drop for Socket {
        fn drop(&mut self) {
            // SAFETY: a host function call with a plain value
            unsafe { ffi::net_close(self.0) };
        }
    }
}

/// The mod's persistent key / value storage (permission `storage = true`; API 1.4): natively a folder under the
/// user's data directory, in the browser the page's IndexedDB. Keys are `A-Z a-z 0-9 . _ -` (1..=64 bytes, not
/// starting with `.`).
pub mod storage {
    use super::{Error, check, ffi, ptr_len, read_sized};
    pub use sa_mod_api::storage::{MAX_KEYS, MAX_TOTAL, MAX_VALUE, valid_key};

    /// The value of `key`, `None` when there is none.
    pub fn read(key: &str) -> Result<Option<Vec<u8>>, Error> {
        let (kp, kl) = ptr_len(key.as_bytes());
        // SAFETY: a host function call; the host writes at most `bl` bytes at `bp`, inside the buffer
        read_sized(|bp, bl| unsafe { ffi::storage_read(kp, kl, bp, bl) })
    }

    /// Writes (replaces) the value of `key`.
    pub fn write(key: &str, data: &[u8]) -> Result<(), Error> {
        let (kp, kl) = ptr_len(key.as_bytes());
        let (dp, dl) = ptr_len(data);
        // SAFETY: a host function call; the host reads both ranges through bounds-checked copies
        check(unsafe { ffi::storage_write(kp, kl, dp, dl) }).map(|_| ())
    }

    /// Deletes `key`; false when there was none.
    pub fn delete(key: &str) -> Result<bool, Error> {
        let (kp, kl) = ptr_len(key.as_bytes());
        // SAFETY: a host function call; the host reads `key` through a bounds-checked copy
        match check(unsafe { ffi::storage_delete(kp, kl) }) {
            Ok(_) => Ok(true),
            Err(Error::NotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// The mod's keys, sorted.
    pub fn keys() -> Result<Vec<String>, Error> {
        // SAFETY: a host function call; the host writes at most `bl` bytes at `bp`, inside the buffer
        let list = read_sized(|bp, bl| unsafe { ffi::storage_list(bp, bl) })?.unwrap_or_default();
        Ok(String::from_utf8_lossy(&list).lines().map(str::to_string).collect())
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

    /// Where world point `p` is on the screen with the game's camera: screen x and y in pixels and the depth in
    /// front of the camera, `None` off screen or behind the camera (API 1.6).
    pub fn world_to_screen(p: Vec3) -> Option<(f32, f32, f32)> {
        let mut out = [0f32; 3];
        // SAFETY: the host writes 12 bytes at `out`
        let r = unsafe { ffi::world_to_screen(p.x, p.y, p.z, out.as_mut_ptr() as usize as i32) };
        (r == 1).then_some((out[0], out[1], out[2]))
    }

    /// Whether the pause menu is up (API 1.5).
    pub fn menu_active() -> bool {
        // SAFETY: a host function call without arguments
        unsafe { ffi::menu_active() == 1 }
    }

    /// The local player's pad this frame (permission `input = "read"`; API 1.5).
    pub fn pad() -> Option<PadInfo> {
        // SAFETY: as in `player`
        record(|p, n| unsafe { ffi::pad_state(p, n) })
    }

    /// A ped's transform (API 1.5); `None` when the handle names no live ped.
    pub fn ped_matrix(ped: PedRef) -> Option<MatrixInfo> {
        // SAFETY: as in `player`
        record(|p, n| unsafe { ffi::ped_matrix(ped.0 as i32, p, n) })
    }

    /// A vehicle's transform (API 1.5); `None` when the handle names no live vehicle.
    pub fn vehicle_matrix(vehicle: VehicleRef) -> Option<MatrixInfo> {
        // SAFETY: as in `player`
        record(|p, n| unsafe { ffi::vehicle_matrix(vehicle.0 as i32, p, n) })
    }
}

/// Before / after hooks on game functions (API 1.2, permission `hooks = [...]`): subscribe in [`Mod::init`],
/// receive the calls in [`Mod::hook`]. A hook sees the values; on the functions of `sa_mod_api::hook::CONTROLLABLE`
/// a mod with `state = "write"` may also change them ([`Call::set_arg`], [`Call::skip`], [`Call::set_result`]).
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

        /// Before the call: replaces argument `i` (an integer, a float, or a ped / vehicle / object of the same
        /// kind as the argument's).
        pub fn set_arg(&self, i: usize, v: Value) -> Result<(), Error> {
            let (_, word) = v.encode();
            // SAFETY: a host function call with plain values
            check(unsafe { ffi::hook_set_arg(i32::try_from(i).map_err(|_| Error::BadArg)?, word as i32) }).map(drop)
        }

        /// Before the call: the game does not run it; its caller gets `result` (for a function with a result:
        /// an integer, a float's bits via [`Value::Float`], or an entity's handle).
        pub fn skip(&self, result: Value) -> Result<(), Error> {
            let (_, word) = result.encode();
            // SAFETY: a host function call with plain values
            check(unsafe { ffi::hook_skip(word as i32) }).map(drop)
        }

        /// After the call: replaces its result.
        pub fn set_result(&self, result: Value) -> Result<(), Error> {
            let (_, word) = result.encode();
            // SAFETY: a host function call with plain values
            check(unsafe { ffi::hook_set_result(word as i32) }).map(drop)
        }
    }
}

/// Changes of the game state (permission `state = "write"`; API 1.3). Called outside a hook a change happens at
/// once (the game is between frames); inside a hook it is queued for the next frame boundary.
pub mod world {
    use super::*;

    fn done(r: i32) -> Result<(), Error> {
        check(r).map(drop)
    }

    // SAFETY (every function below): a host function call with plain values.

    /// Moves `ped` to `pos` (out of a vehicle first, onto the ground).
    pub fn set_ped_position(ped: PedRef, pos: Vec3) -> Result<(), Error> {
        done(unsafe { ffi::set_ped_position(ped.0 as i32, pos.x, pos.y, pos.z) })
    }
    /// Turns `ped` to `degrees`.
    pub fn set_ped_heading(ped: PedRef, degrees: f32) -> Result<(), Error> {
        done(unsafe { ffi::set_ped_heading(ped.0 as i32, degrees) })
    }
    /// Sets `ped`'s health.
    pub fn set_ped_health(ped: PedRef, health: f32) -> Result<(), Error> {
        done(unsafe { ffi::set_ped_health(ped.0 as i32, health) })
    }
    /// Sets `ped`'s armour.
    pub fn set_ped_armour(ped: PedRef, armour: f32) -> Result<(), Error> {
        done(unsafe { ffi::set_ped_armour(ped.0 as i32, armour) })
    }
    /// Gives `ped` weapon `weapon` (`eWeaponType`) with `ammo` rounds.
    pub fn give_weapon(ped: PedRef, weapon: u8, ammo: u32) -> Result<(), Error> {
        done(unsafe { ffi::give_weapon(ped.0 as i32, i32::from(weapon), ammo.min(i32::MAX as u32) as i32) })
    }
    /// Sets the player's money.
    pub fn set_money(money: i32) -> Result<(), Error> {
        done(unsafe { ffi::set_money(money) })
    }
    /// Sets the player's wanted level (0..=6).
    pub fn set_wanted_level(level: u8) -> Result<(), Error> {
        done(unsafe { ffi::set_wanted_level(i32::from(level)) })
    }
    /// Sets the game clock.
    pub fn set_clock(hour: u8, minute: u8) -> Result<(), Error> {
        done(unsafe { ffi::set_clock(i32::from(hour), i32::from(minute)) })
    }
    /// Sets the weather at once (`eWeatherType`).
    pub fn set_weather(weather: u8) -> Result<(), Error> {
        done(unsafe { ffi::set_weather(i32::from(weather)) })
    }
    /// Moves `vehicle` to `pos`.
    pub fn set_vehicle_position(vehicle: VehicleRef, pos: Vec3) -> Result<(), Error> {
        done(unsafe { ffi::set_vehicle_position(vehicle.0 as i32, pos.x, pos.y, pos.z) })
    }
    /// Turns `vehicle` to `degrees`.
    pub fn set_vehicle_heading(vehicle: VehicleRef, degrees: f32) -> Result<(), Error> {
        done(unsafe { ffi::set_vehicle_heading(vehicle.0 as i32, degrees) })
    }
    /// Sets `vehicle`'s health.
    pub fn set_vehicle_health(vehicle: VehicleRef, health: f32) -> Result<(), Error> {
        done(unsafe { ffi::set_vehicle_health(vehicle.0 as i32, health) })
    }
    /// Sets `vehicle`'s colours.
    pub fn set_vehicle_colours(vehicle: VehicleRef, primary: u8, secondary: u8) -> Result<(), Error> {
        done(unsafe { ffi::set_vehicle_colours(vehicle.0 as i32, i32::from(primary), i32::from(secondary)) })
    }
}

/// Entities the mod creates and owns (permission `entities = true`; API 1.3): deleted by the host when the mod is
/// disabled. Not inside a hook.
pub mod entities {
    use super::*;

    // SAFETY (every function below): a host function call with plain values.

    /// A vehicle of `model` at `pos` facing `degrees`.
    pub fn create_vehicle(model: i32, pos: Vec3, degrees: f32) -> Result<VehicleRef, Error> {
        check(unsafe { ffi::create_vehicle(model, pos.x, pos.y, pos.z, degrees) }).map(|h| VehicleRef(h as u32))
    }
    /// A civilian ped of `model` at `pos`.
    pub fn create_ped(model: i32, pos: Vec3) -> Result<PedRef, Error> {
        check(unsafe { ffi::create_ped(model, pos.x, pos.y, pos.z) }).map(|h| PedRef(h as u32))
    }
    /// An object of `model` at `pos`.
    pub fn create_object(model: i32, pos: Vec3) -> Result<ObjectRef, Error> {
        check(unsafe { ffi::create_object(model, pos.x, pos.y, pos.z) }).map(|h| ObjectRef(h as u32))
    }
    /// Deletes a vehicle the mod created.
    pub fn destroy_vehicle(v: VehicleRef) -> Result<(), Error> {
        check(unsafe { ffi::destroy_vehicle(v.0 as i32) }).map(drop)
    }
    /// Deletes a ped the mod created.
    pub fn destroy_ped(p: PedRef) -> Result<(), Error> {
        check(unsafe { ffi::destroy_ped(p.0 as i32) }).map(drop)
    }
    /// Deletes an object the mod created.
    pub fn destroy_object(o: ObjectRef) -> Result<(), Error> {
        check(unsafe { ffi::destroy_object(o.0 as i32) }).map(drop)
    }
}

/// Script commands (permission `scripts = true`; API 1.3): the commands of `sa_mod_api::script::COMMANDS`.
pub mod script {
    use super::*;

    /// One argument of [`run`].
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Arg<'a> {
        /// An integer (`i`, and `P`: the player, 0).
        Int(i32),
        /// A float (`f`).
        Float(f32),
        /// A ped (`p`).
        Ped(PedRef),
        /// A vehicle (`v`).
        Vehicle(VehicleRef),
        /// An output (`o`).
        Out,
        /// A text (`s`, API 1.5; at most `sa_mod_api::script::MAX_TEXT` bytes, no NUL).
        Text(&'a [u8]),
    }

    /// What a command returned.
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Outcome {
        /// The condition result.
        pub condition: bool,
        /// The outputs, one word each, in parameter order.
        pub outputs: Vec<u32>,
    }

    /// Runs script command `opcode` with `args` (one per parameter of its `COMMANDS` entry). Not inside a hook.
    pub fn run(opcode: u16, args: &[Arg<'_>]) -> Result<Outcome, Error> {
        let texts: Vec<Vec<u8>> = args
            .iter()
            .filter_map(|a| match a {
                Arg::Text(t) => Some(t.iter().copied().chain([0]).collect()),
                _ => None,
            })
            .collect();
        let mut text = texts.iter();
        let words: Vec<i32> = args
            .iter()
            .map(|a| match *a {
                Arg::Int(i) => i,
                Arg::Float(f) => f.to_bits() as i32,
                Arg::Ped(p) => p.0 as i32,
                Arg::Vehicle(v) => v.0 as i32,
                Arg::Out => 0,
                Arg::Text(_) => text.next().map_or(0, |t| t.as_ptr() as usize as i32),
            })
            .collect();
        let mut outputs = vec![0u32; args.iter().filter(|a| **a == Arg::Out).count()];
        let args_ptr = words.as_ptr() as usize as i32;
        let out_ptr = if outputs.is_empty() { 0 } else { outputs.as_mut_ptr() as usize as i32 };
        // SAFETY: the host reads the argument words and writes one word per output through bounds-checked copies
        let r = check(unsafe { ffi::script_command(i32::from(opcode), args_ptr, words.len() as i32, out_ptr) })?;
        Ok(Outcome { condition: r != 0, outputs })
    }
}

/// The player's view of the game (API 1.5): the HUD, on-screen messages, the player's model and the game's own
/// features (permission `state = "write"`).
pub mod game {
    use super::*;
    pub use sa_mod_api::sa_types::GameOption;

    // SAFETY (every function below): a host function call with plain values or a buffer of the mod's own.

    /// Gives the player ped `ped` the ped model `model` (loaded first).
    pub fn set_ped_model(ped: PedRef, model: i32) -> Result<(), Error> {
        check(unsafe { ffi::set_ped_model(ped.0 as i32, model) }).map(drop)
    }

    /// Shows or hides the HUD and the radar.
    pub fn set_hud_visible(on: bool) -> Result<(), Error> {
        check(unsafe { ffi::set_hud_visible(i32::from(on)) }).map(drop)
    }

    /// Switches one of the game's own features on or off (the host turns it back on when it disables the mod).
    pub fn set_option(option: GameOption, on: bool) -> Result<(), Error> {
        check(unsafe { ffi::set_game_option(sa_mod_api::option::number(option), i32::from(on)) }).map(drop)
    }

    /// Shows `text` as the game's big message in `style` (0..=6) for `time_ms`.
    pub fn big_message(text: &str, time_ms: u32, style: u8) -> Result<(), Error> {
        check(unsafe {
            ffi::big_message(text.as_ptr() as usize as i32, text.len() as i32, time_ms as i32, i32::from(style))
        })
        .map(drop)
    }
}

/// Platform input before the game sees it (permission `input = "capture"`; API 1.5): events in [`Mod::input`],
/// and taking the keyboard and the mouse from the game.
pub mod input {
    use super::*;
    use sa_mod_api::input as k;

    /// A platform input event (keys are Windows virtual-key codes).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Input {
        /// A key went down.
        KeyDown {
            /// The key.
            vk: u16,
            /// An auto-repeat.
            repeat: bool,
        },
        /// A key went up.
        KeyUp {
            /// The key.
            vk: u16,
        },
        /// A character was typed.
        Char(char),
        /// The mouse moved.
        MouseMove {
            /// Pixels from the left.
            x: i32,
            /// Pixels from the top.
            y: i32,
        },
        /// A mouse button went down or up.
        MouseButton {
            /// 0 left, 1 right, 2 middle.
            button: u8,
            /// Down, else up.
            down: bool,
        },
        /// The wheel turned, in 1/120 notches.
        Wheel(i32),
        /// The window gained or lost the focus.
        Focus(bool),
    }

    impl Input {
        /// Decodes `sa_input`'s values.
        pub fn decode(kind: i32, a: i32, b: i32) -> Option<Input> {
            Some(match kind {
                k::KEY_DOWN => Input::KeyDown { vk: u16::try_from(a).ok()?, repeat: b != 0 },
                k::KEY_UP => Input::KeyUp { vk: u16::try_from(a).ok()? },
                k::CHAR => Input::Char(char::from_u32(a as u32)?),
                k::MOUSE_MOVE => Input::MouseMove { x: a, y: b },
                k::MOUSE_BUTTON => Input::MouseButton { button: u8::try_from(a).ok()?, down: b != 0 },
                k::WHEEL => Input::Wheel(a),
                k::FOCUS => Input::Focus(a != 0),
                _ => return None,
            })
        }
    }

    /// Takes the keyboard (the game sees no keys), the mouse, and shows the platform's cursor, until changed.
    pub fn capture(keyboard: bool, mouse: bool, cursor: bool) -> Result<(), Error> {
        // SAFETY: a host function call with plain values
        check(unsafe { ffi::input_capture(i32::from(keyboard), i32::from(mouse), i32::from(cursor)) }).map(drop)
    }
}

/// Game sounds (permission `audio = true`; API 1.3).
pub mod audio {
    use super::*;

    /// Plays front-end sound `event` (`AE_FRONTEND_*`, 1..=46) at speed 1.
    pub fn frontend(event: u32) -> Result<(), Error> {
        // SAFETY: a host function call with plain values
        check(unsafe { ffi::play_frontend_sound(event as i32, 1.0) }).map(drop)
    }

    /// Plays mission sound `sound` at `pos`.
    pub fn at(sound: i32, pos: Vec3) -> Result<(), Error> {
        // SAFETY: a host function call with plain values
        check(unsafe { ffi::play_sound(sound, pos.x, pos.y, pos.z) }).map(drop)
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

    /// A platform input event before the game sees it (permission `input = "capture"`; API 1.5). Returns true
    /// to consume it.
    fn input(&mut self, event: input::Input) -> bool {
        let _ = event;
        false
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

    pub fn sa_input<T>(kind: i32, a: i32, b: i32) -> i32 {
        let Some(e) = input::Input::decode(kind, a, b) else { return 0 };
        let mut consumed = false;
        with(|m| consumed = m.input(e));
        i32::from(consumed)
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
