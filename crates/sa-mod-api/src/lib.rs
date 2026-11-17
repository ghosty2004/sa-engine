//! The mod API's interface definition (MOD-2, `docs/arch/ADR-004-mods.md` §3.2): the one source of truth of the
//! ABI between the game's mod host (`sa-mod-host`) and a mod (`sa-mod`, the Rust SDK, or any language with a wasm32
//! target).
//!
//! # The ABI (core WebAssembly)
//!
//! A mod is a core wasm module (not a component). It
//! * imports the host functions of [`for_each_import!`] from the module [`IMPORT_MODULE`] (`"sa"`);
//! * exports the entry points of [`for_each_export!`] and its linear memory as [`EXPORT_MEMORY`];
//! * passes only `i32` / `f32` values ([`Ty`]); text and buffers are `(ptr, len)` pairs into its own linear memory,
//!   which the host reads (bounds-checked: a bad range is [`status::BAD_ARG`], never a wild read);
//! * gets handles, never game addresses.
//!
//! The same definition generates both sides: the host builds its import table, its call decoder ([`Call`]) and its
//! export signature checks from [`IMPORTS`] / [`EXPORTS`]; the SDK expands [`for_each_import!`] into its `extern`
//! block and [`for_each_export!`] into its exported entry points; the browser's JS glue gets the names and arities at
//! run time from the host. Nothing else declares an API function.
//!
//! # Versioning
//!
//! [`API_MAJOR`].[`API_MINOR`] (semver): a minor version only adds imports or exports. The host runs a mod whose
//! manifest `api_version` has the host's major version and a minor version not above the host's ([`compatible`]).

#![forbid(unsafe_code)]

/// The import module name of every host function.
pub const IMPORT_MODULE: &str = "sa";
/// The name under which a mod exports its linear memory.
pub const EXPORT_MEMORY: &str = "memory";
/// The API's major version.
pub const API_MAJOR: u16 = 1;
/// The API's minor version.
pub const API_MINOR: u16 = 1;

/// `major << 16 | minor`, the value of the `sa_api_version` export.
pub const fn pack_version(major: u16, minor: u16) -> i32 {
    ((major as i32) << 16) | minor as i32
}

/// The `(major, minor)` of a packed version.
pub const fn unpack_version(v: i32) -> (u16, u16) {
    ((v >> 16) as u16, v as u16)
}

/// True when a host of API `host` runs a mod built for API `wanted`: the same major version, the mod's minor version
/// not above the host's.
pub const fn compatible(host: (u16, u16), wanted: (u16, u16)) -> bool {
    host.0 == wanted.0 && wanted.1 <= host.1
}

/// The status codes the host functions return (`i32`); values `>= 0` are results.
pub mod status {
    /// Done.
    pub const OK: i32 = 0;
    /// The mod's manifest does not grant the permission this function needs.
    pub const DENIED: i32 = -1;
    /// An argument is invalid: a `(ptr, len)` range outside the mod's memory, text that is not UTF-8, a value out
    /// of range.
    pub const BAD_ARG: i32 = -2;
    /// The file does not exist (`file_read`).
    pub const NOT_FOUND: i32 = -3;
}

/// Log levels of the `log` import.
pub mod level {
    /// Information.
    pub const INFO: i32 = 0;
    /// A warning.
    pub const WARN: i32 = 1;
    /// An error.
    pub const ERROR: i32 = 2;
}

/// A wasm value type of the ABI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    /// `i32`
    I32,
    /// `f32`
    F32,
}

/// A wasm value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Val {
    /// `i32`
    I32(i32),
    /// `f32`
    F32(f32),
}

impl Val {
    /// The value's type.
    pub fn ty(self) -> Ty {
        match self {
            Val::I32(_) => Ty::I32,
            Val::F32(_) => Ty::F32,
        }
    }
}

/// The manifest permission a host function needs (ADR-004 §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    /// Always allowed.
    None,
    /// `render = true`: graphics settings, post-processing, overlays.
    Render,
    /// `files = "own"`: the mod's own files (its folder, through the host).
    Files,
}

/// One host function (an import of the mod).
#[derive(Debug)]
pub struct ImportDef {
    /// Its name in [`IMPORT_MODULE`].
    pub name: &'static str,
    /// Parameter names and types.
    pub params: &'static [(&'static str, Ty)],
    /// The result type.
    pub result: Ty,
    /// The permission it needs.
    pub permission: Permission,
}

/// One mod entry point (an export of the mod).
#[derive(Debug)]
pub struct ExportDef {
    /// Its export name.
    pub name: &'static str,
    /// Every mod exports it (API 1.0); an optional one may be missing (the host skips its call point).
    pub required: bool,
    /// Parameter names and types.
    pub params: &'static [(&'static str, Ty)],
    /// The result type.
    pub result: Ty,
}

/// Expands `$cb! { ... }` with the host functions (the mod's imports), one item per function:
/// `$(#[doc])* [Permission] fn name(param: ty, ...) -> ty;` with `ty` one of `i32`, `f32`. An optional
/// `[tokens]` after `$cb` is passed through first.
///
/// Every host function returns an `i32`: a result `>= 0` or a [`status`] code.
#[macro_export]
macro_rules! for_each_import {
    ($cb:ident $([$($extra:tt)*])?) => {
        $cb! {
            $([$($extra)*])?
            /// Writes `text` (UTF-8, `text_len` bytes at `text_ptr`) to the game's log at `level` ([`level`]).
            [None] fn log(level: i32, text_ptr: i32, text_len: i32) -> i32;
            /// The overlay's width in pixels (the game's back buffer).
            [None] fn screen_width() -> i32;
            /// The overlay's height in pixels (the game's back buffer).
            [None] fn screen_height() -> i32;
            /// Fills the rectangle (`x`, `y`, `w`, `h`; pixels from the top left) of the 2D overlay with colour
            /// `rgba` (`0xRRGGBBAA`, alpha-blended over the game's image). Valid in the overlay phase.
            [Render] fn overlay_rect(x: f32, y: f32, w: f32, h: f32, rgba: i32) -> i32;
            /// Draws `text` (UTF-8, `text_len` bytes at `text_ptr`) in the 2D overlay with its top left at (`x`,
            /// `y`), in the host's fixed-cell font, `size` pixels per glyph cell (8 is one font pixel per screen
            /// pixel), colour `rgba` (`0xRRGGBBAA`). Characters the font lacks draw as `?`. Returns the text's width
            /// in pixels. Valid in the overlay phase.
            [Render] fn overlay_text(x: f32, y: f32, size: f32, rgba: i32, text_ptr: i32, text_len: i32) -> i32;
            /// (1.1) The number of display modes the platform offers (natively the monitor's video modes, offscreen
            /// the platform's fixed mode table; in the browser the canvas size). Valid in every phase.
            [Render] fn display_mode_count() -> i32;
            /// (1.1) The width of display mode `index` (`0 .. display_mode_count()`); `index` -1 is the desktop
            /// (natively the monitor's current mode; in the browser the canvas's size on the page in device
            /// pixels, i.e. CSS pixels times the device pixel ratio).
            [Render] fn display_mode_width(index: i32) -> i32;
            /// (1.1) The height of display mode `index`; -1: the desktop (see `display_mode_width`).
            [Render] fn display_mode_height(index: i32) -> i32;
            /// (1.1) Requests the game's video mode `width` x `height` (32-bit colour), in a window (`fullscreen`
            /// 0) or full screen (1; natively). In the settings phase it applies from the game's first frame,
            /// through the original's own video-mode selection; later (a resize event, the overlay phase) it is
            /// applied at the next frame boundary as the menu's resolution change, when the adapter lists the
            /// size. The last request of a phase wins. Returns 0, or `BAD_ARG` (a size outside 320x240 ..
            /// 16384x16384).
            [Render] fn request_video_mode(width: i32, height: i32, fullscreen: i32) -> i32;
            /// (1.1) Reads the mod's own file `name` (UTF-8, `name_len` bytes at `name_ptr`: a plain file name
            /// of `A-Z a-z 0-9 . _ -`, not `mod.toml`, not a `.wasm`) into the buffer (`buf_ptr`, `buf_len`):
            /// copies `min(size, buf_len)` bytes and returns the file's size, or `NOT_FOUND`.
            [Files] fn file_read(name_ptr: i32, name_len: i32, buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.1) Writes (replaces) the mod's own file `name` (as `file_read`) with `data_len` bytes at
            /// `data_ptr` (at most 1 MiB). Natively the file lives in the mod's folder; in the browser it is kept
            /// for the session (the page keeps no files). Returns 0.
            [Files] fn file_write(name_ptr: i32, name_len: i32, data_ptr: i32, data_len: i32) -> i32;
        }
    };
}

/// Expands `$cb! { ... }` with the mod's entry points (its exports), one item per function:
/// `$(#[doc])* [Required|Optional] fn name(param: ty, ...) -> ty;`. An optional `[tokens]` after `$cb` is passed
/// through first. `Required` entry points are those of API 1.0; the ones added later are `Optional` (a mod built for
/// an older minor version lacks them; the host then skips that call point).
#[macro_export]
macro_rules! for_each_export {
    ($cb:ident $([$($extra:tt)*])?) => {
        $cb! {
            $([$($extra)*])?
            /// The API version the mod was built against ([`pack_version`]).
            [Required] fn sa_api_version() -> i32;
            /// Called once after loading; anything but 0 refuses the mod.
            [Required] fn sa_init() -> i32;
            /// The overlay phase: called once per presented frame, after the game's own drawing, before the
            /// present. `frame` counts the overlay calls since the mod was loaded (0, 1, ...). Returns 0.
            [Required] fn sa_overlay(frame: i32) -> i32;
            /// (1.1) The settings phase: called once at start-up, after the game has read its settings file and
            /// before it selects its video mode and creates its D3D device. Graphics settings requested here
            /// (`request_video_mode`) apply from the first frame. Returns 0.
            [Optional] fn sa_settings() -> i32;
            /// (1.1) A platform event: the drawable the game is shown in (the native window's client area, the
            /// browser canvas's size on the page in device pixels) changed to `width` x `height`. The game's video
            /// mode does not follow by itself (the image is scaled); a mod may request a new one. Returns 0.
            [Optional] fn sa_resize(width: i32, height: i32) -> i32;
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __ty {
    (i32) => {
        $crate::Ty::I32
    };
    (f32) => {
        $crate::Ty::F32
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __required {
    (Required) => {
        true
    };
    (Optional) => {
        false
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __get {
    (i32, $v:expr) => {
        match $v {
            $crate::Val::I32(x) => x,
            _ => return None,
        }
    };
    (f32, $v:expr) => {
        match $v {
            $crate::Val::F32(x) => x,
            _ => return None,
        }
    };
}

macro_rules! import_table {
    ($( $(#[$m:meta])* [$perm:ident] fn $name:ident( $($p:ident : $t:ident),* ) -> $r:ident; )*) => {
        /// Every host function, in a fixed order: the index of an entry is the import's index in calls between
        /// a backend and the host's dispatcher.
        pub static IMPORTS: &[ImportDef] = &[$(
            ImportDef {
                name: stringify!($name),
                params: &[$((stringify!($p), __ty!($t))),*],
                result: __ty!($r),
                permission: Permission::$perm,
            },
        )*];

        /// A decoded host function call (the arguments of an [`IMPORTS`] entry).
        #[allow(non_camel_case_types, missing_docs)]
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum Call {
            $( $(#[$m])* $name { $($p: $t),* }, )*
        }

        impl Call {
            /// Decodes a call of import `index` with `args`; `None` when the index or the argument types do not
            /// match [`IMPORTS`].
            pub fn decode(index: usize, args: &[Val]) -> Option<Call> {
                let def = IMPORTS.get(index)?;
                if args.len() != def.params.len() {
                    return None;
                }
                let mut it = args.iter().copied();
                let mut i = 0usize;
                $(
                    if i == index {
                        return Some(Call::$name { $($p: __get!($t, it.next()?)),* });
                    }
                    i += 1;
                )*
                let _ = (i, &mut it);
                None
            }
        }
    };
}
for_each_import!(import_table);

macro_rules! export_table {
    ($( $(#[$m:meta])* [$req:ident] fn $name:ident( $($p:ident : $t:ident),* ) -> $r:ident; )*) => {
        /// Every mod entry point.
        pub static EXPORTS: &[ExportDef] = &[$(
            ExportDef {
                name: stringify!($name),
                required: __required!($req),
                params: &[$((stringify!($p), __ty!($t))),*],
                result: __ty!($r),
            },
        )*];
    };
}
for_each_export!(export_table);

/// The name of the `sa_api_version` export.
pub const EXPORT_API_VERSION: &str = "sa_api_version";
/// The name of the `sa_init` export.
pub const EXPORT_INIT: &str = "sa_init";
/// The name of the `sa_overlay` export.
pub const EXPORT_OVERLAY: &str = "sa_overlay";
/// The name of the `sa_settings` export (1.1).
pub const EXPORT_SETTINGS: &str = "sa_settings";
/// The name of the `sa_resize` export (1.1).
pub const EXPORT_RESIZE: &str = "sa_resize";

/// `request_video_mode`'s accepted sizes (inclusive).
pub const VIDEO_MODE_MIN: (i32, i32) = (320, 240);
/// See [`VIDEO_MODE_MIN`].
pub const VIDEO_MODE_MAX: i32 = 16_384;
/// The largest file `file_write` writes.
pub const MAX_FILE: usize = 1 << 20;

/// True for a name `file_read` / `file_write` accept: 1..=64 bytes of `A-Z a-z 0-9 . _ -`, not starting with `.`,
/// not `mod.toml`, not a `.wasm`.
pub fn valid_file_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        && !name.starts_with('.')
        && !name.eq_ignore_ascii_case("mod.toml")
        && !name.to_ascii_lowercase().ends_with(".wasm")
}

/// The index of the import named `name` in [`IMPORTS`].
pub fn import_index(name: &str) -> Option<usize> {
    IMPORTS.iter().position(|d| d.name == name)
}

/// The export named `name`.
pub fn export(name: &str) -> Option<&'static ExportDef> {
    EXPORTS.iter().find(|d| d.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_and_decoder_agree() {
        let i = import_index("overlay_rect").unwrap();
        let args = [Val::F32(1.0), Val::F32(2.0), Val::F32(3.0), Val::F32(4.0), Val::I32(-1)];
        assert_eq!(Call::decode(i, &args), Some(Call::overlay_rect { x: 1.0, y: 2.0, w: 3.0, h: 4.0, rgba: -1 }));
        assert_eq!(Call::decode(i, &args[..4]), None);
        assert_eq!(Call::decode(i, &[Val::I32(0); 5]), None);
        assert_eq!(Call::decode(IMPORTS.len(), &[]), None);
        assert_eq!(Call::decode(import_index("screen_width").unwrap(), &[]), Some(Call::screen_width {}));
        assert_eq!(IMPORTS[i].permission, Permission::Render);
    }

    #[test]
    fn exports_are_named_once() {
        for n in [EXPORT_API_VERSION, EXPORT_INIT, EXPORT_OVERLAY, EXPORT_SETTINGS, EXPORT_RESIZE] {
            assert_eq!(EXPORTS.iter().filter(|d| d.name == n).count(), 1, "{n}");
        }
        assert!(export(EXPORT_INIT).unwrap().required);
        assert!(!export(EXPORT_SETTINGS).unwrap().required);
    }

    #[test]
    fn file_names() {
        assert!(valid_file_name("config.toml"));
        for n in ["", ".hidden", "a/b", "..", "mod.toml", "MOD.TOML", "x.wasm", "a b"] {
            assert!(!valid_file_name(n), "{n:?}");
        }
    }

    #[test]
    fn versions() {
        let v = pack_version(API_MAJOR, API_MINOR);
        assert_eq!(unpack_version(v), (API_MAJOR, API_MINOR));
        assert!(compatible((1, 2), (1, 0)));
        assert!(!compatible((1, 0), (1, 1)));
        assert!(!compatible((1, 0), (2, 0)));
    }
}
