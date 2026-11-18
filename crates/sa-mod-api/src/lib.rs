//! The mod API's interface definition (`docs/arch/ADR-004-mods.md` §3.2): the one source of truth of the
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

pub mod state;

pub use sa_types;

/// The import module name of every host function.
pub const IMPORT_MODULE: &str = "sa";
/// The name under which a mod exports its linear memory.
pub const EXPORT_MEMORY: &str = "memory";
/// The API's major version.
pub const API_MAJOR: u16 = 1;
/// The API's minor version.
pub const API_MINOR: u16 = 3;

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
    /// The file does not exist (`file_read`), the handle names no live entity, there is no local player, the
    /// registry has no function of that name, a hook value is absent.
    pub const NOT_FOUND: i32 = -3;
    /// Not now: the call is not allowed in this phase (an entity created or a script command run from inside a
    /// hook, a game call before the game exists).
    pub const WRONG_PHASE: i32 = -4;
    /// The game could not do it (a model that does not load, a pool that is full, game code that is not ported
    /// yet); the host's log says why.
    pub const FAILED: i32 = -5;
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
    /// `state = "read"`: game state queries and the game events (API 1.2).
    State,
    /// `input = "read"`: key state and key events (API 1.2).
    Input,
    /// `hooks = ["name", ...]`: before / after hooks on the named game functions (API 1.2).
    Hooks,
    /// `state = "write"`: changes of the game state (API 1.3).
    Write,
    /// `hooks` and `state = "write"`: decisions in a hook (changed arguments, a skipped call, a changed result) on
    /// the functions of [`hook::CONTROLLABLE`] (API 1.3).
    HookControl,
    /// `entities = true`: creating and deleting peds, vehicles and objects (API 1.3).
    Entities,
    /// `scripts = true`: the script commands of [`script::COMMANDS`] (API 1.3).
    Scripts,
    /// `audio = true`: game sounds (API 1.3).
    Audio,
}

/// Game events (API 1.2): what a mod receives in `sa_event(kind, a, b)` after subscribing with
/// `events_subscribe(mask)` (bit `1 << kind`). The host derives them from the game state at frame boundaries, in
/// this order within a frame; it never changes the game.
pub mod event {
    use super::Permission;

    /// A main-loop iteration begins (before the game's logic). `a`: the frame counter.
    pub const FRAME_BEGIN: i32 = 0;
    /// A frame is presented (after the game's drawing and the overlay phase). `a`: the frame counter.
    pub const FRAME_END: i32 = 1;
    /// The game state machine (`gGameState`) changed. `a`: the new state, `b`: the old one (raw values: 7
    /// frontend idle, 8 loading, 9 playing).
    pub const GAME_STATE: i32 = 2;
    /// The local player's ped appeared or came back to life. `a`: its handle.
    pub const PLAYER_SPAWN: i32 = 3;
    /// The local player's ped died (health at 0 or its state dying / dead). `a`: its handle.
    pub const PLAYER_DEATH: i32 = 4;
    /// The local player entered a vehicle. `a`: the vehicle, `b`: the player's ped.
    pub const VEHICLE_ENTER: i32 = 5;
    /// The local player left a vehicle. `a`: the vehicle (it may no longer exist), `b`: the player's ped.
    pub const VEHICLE_EXIT: i32 = 6;
    /// The current area / interior changed. `a`: the new area, `b`: the old one.
    pub const AREA: i32 = 7;
    /// A key went down. `a`: its Windows virtual-key code.
    pub const KEY_DOWN: i32 = 8;
    /// A key went up. `a`: its Windows virtual-key code.
    pub const KEY_UP: i32 = 9;
    /// The number of event kinds.
    pub const COUNT: i32 = 10;

    use sa_types::{GameState, PedRef, VehicleRef};

    /// A decoded event.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Event {
        /// [`FRAME_BEGIN`].
        FrameBegin(u32),
        /// [`FRAME_END`].
        FrameEnd(u32),
        /// [`GAME_STATE`].
        GameState {
            /// The new state.
            new: GameState,
            /// The old one.
            old: GameState,
        },
        /// [`PLAYER_SPAWN`].
        PlayerSpawn(PedRef),
        /// [`PLAYER_DEATH`].
        PlayerDeath(PedRef),
        /// [`VEHICLE_ENTER`].
        VehicleEnter {
            /// The vehicle.
            vehicle: VehicleRef,
            /// The player's ped.
            ped: PedRef,
        },
        /// [`VEHICLE_EXIT`].
        VehicleExit {
            /// The vehicle.
            vehicle: VehicleRef,
            /// The player's ped.
            ped: PedRef,
        },
        /// [`AREA`].
        Area {
            /// The new area.
            new: i32,
            /// The old one.
            old: i32,
        },
        /// [`KEY_DOWN`].
        KeyDown(u16),
        /// [`KEY_UP`].
        KeyUp(u16),
    }

    impl Event {
        /// The event of `sa_event(kind, a, b)`.
        pub fn decode(kind: i32, a: i32, b: i32) -> Option<Event> {
            Some(match kind {
                FRAME_BEGIN => Event::FrameBegin(a as u32),
                FRAME_END => Event::FrameEnd(a as u32),
                GAME_STATE => Event::GameState { new: GameState::from_raw(a), old: GameState::from_raw(b) },
                PLAYER_SPAWN => Event::PlayerSpawn(PedRef(a as u32)),
                PLAYER_DEATH => Event::PlayerDeath(PedRef(a as u32)),
                VEHICLE_ENTER => Event::VehicleEnter { vehicle: VehicleRef(a as u32), ped: PedRef(b as u32) },
                VEHICLE_EXIT => Event::VehicleExit { vehicle: VehicleRef(a as u32), ped: PedRef(b as u32) },
                AREA => Event::Area { new: a, old: b },
                KEY_DOWN => Event::KeyDown(a as u16),
                KEY_UP => Event::KeyUp(a as u16),
                _ => return None,
            })
        }
    }

    /// The permission an event needs.
    pub const fn permission(kind: i32) -> Permission {
        match kind {
            FRAME_BEGIN | FRAME_END => Permission::None,
            KEY_DOWN | KEY_UP => Permission::Input,
            _ => Permission::State,
        }
    }
}

/// Before / after hooks on game functions (API 1.2).
pub mod hook {
    use sa_types::{ObjectRef, PedRef, VehicleRef};

    /// `hook_subscribe` phases (bits) and `sa_hook`'s `phase` argument: before the function runs (its arguments).
    pub const BEFORE: i32 = 1;
    /// After it returned (its arguments and its result).
    pub const AFTER: i32 = 2;
    /// The most arguments a hook passes on.
    pub const MAX_ARGS: usize = 16;
    /// Hook calls one mod receives per frame at most; the rest of the frame's calls are dropped (and counted).
    pub const MAX_CALLS_PER_FRAME: u32 = 4096;

    /// The functions whose calls a hook may change (`hook_set_arg`, `hook_skip`, `hook_set_result`): the ones
    /// where the game copes with a changed argument, a skipped call or a changed result because the original
    /// game's own code paths produce the same (a shot that does not happen, damage that is not dealt, a ped or
    /// vehicle that does not run its control this step, an entity that is not drawn).
    pub const CONTROLLABLE: &[&str] = &[
        "CWeapon::Fire",
        "CWeapon::GenerateDamageEvent",
        "CBulletInfo::AddBullet",
        "CEventDamage::AffectsPed",
        "CPedDamageResponseCalculator::ComputeDamageResponse",
        "CPed::ProcessControl",
        "CPlayerPed::ProcessControl",
        "CAutomobile::ProcessControl",
        "CTaskComplexEnterCar::CreateFirstSubTask",
        "CPed::Render",
        "CObject::Render",
        "CCamera::CamShake",
        "CHud::DrawCrossHairs",
    ];

    /// Whether a hook may change the calls of `name` ([`CONTROLLABLE`]).
    pub fn controllable(name: &str) -> bool {
        CONTROLLABLE.contains(&name)
    }

    /// An argument or result of a hooked function, as a mod sees it: plain values as they are, a pointer to a ped,
    /// vehicle or object as its handle, any other pointer hidden.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Value {
        /// An integer (signed or not, 32 bits at most).
        Int(i32),
        /// A float.
        Float(f32),
        /// A ped.
        Ped(PedRef),
        /// A vehicle.
        Vehicle(VehicleRef),
        /// An object.
        Object(ObjectRef),
        /// Another pointer (mods never see game addresses).
        Pointer,
    }

    impl Value {
        /// `(kind, word)`: what `hook_value` returns and writes.
        pub fn encode(self) -> (i32, u32) {
            match self {
                Value::Int(v) => (0, v as u32),
                Value::Float(v) => (1, v.to_bits()),
                Value::Ped(p) => (2, p.0),
                Value::Vehicle(v) => (3, v.0),
                Value::Object(o) => (4, o.0),
                Value::Pointer => (5, 0),
            }
        }

        /// The value of `(kind, word)`.
        pub fn decode(kind: i32, word: u32) -> Option<Value> {
            Some(match kind {
                0 => Value::Int(word as i32),
                1 => Value::Float(f32::from_bits(word)),
                2 => Value::Ped(PedRef(word)),
                3 => Value::Vehicle(VehicleRef(word)),
                4 => Value::Object(ObjectRef(word)),
                5 => Value::Pointer,
                _ => return None,
            })
        }
    }
}

/// Script commands (API 1.3, `scripts = true`): `script_command(opcode, args, out)` runs one command of the game's
/// script language through the game's own command handler, on a private script thread of the host.
pub mod script {
    /// The most arguments of one command.
    pub const MAX_ARGS: usize = 16;

    /// The commands a mod may run, with their parameters, one letter each: `i` an integer, `f` a float, `p` a ped
    /// handle, `v` a vehicle handle, `P` a player (0, the local player), `o` an output. Gameplay commands whose
    /// handlers the engine has ported and that act on the player, the world, the camera, the HUD or an entity named
    /// by its handle; the host checks every handle before the command runs (the game's handlers trust them).
    /// Entity creation and deletion are not here: they are `create_*` / `destroy_*`, which keep track of what a mod
    /// owns.
    pub const COMMANDS: &[(u16, &str, &str)] = &[
        (0x00C0, "SET_TIME_OF_DAY", "ii"),
        (0x00DF, "IS_CHAR_IN_ANY_CAR", "p"),
        (0x00EC, "LOCATE_CHAR_ANY_MEANS_2D", "pffffi"),
        (0x0109, "ADD_SCORE", "Pi"),
        (0x0110, "CLEAR_WANTED_LEVEL", "P"),
        (0x0118, "IS_CHAR_DEAD", "p"),
        (0x0119, "IS_CAR_DEAD", "v"),
        (0x015F, "SET_FIXED_CAMERA_POSITION", "ffffff"),
        (0x0160, "POINT_CAMERA_AT_POINT", "fffi"),
        (0x01B4, "SET_PLAYER_CONTROL", "Pi"),
        (0x01B6, "FORCE_WEATHER_NOW", "i"),
        (0x01B7, "RELEASE_WEATHER", ""),
        (0x01EB, "SET_CAR_DENSITY_MULTIPLIER", "f"),
        (0x01F0, "SET_MAX_WANTED_LEVEL", "i"),
        (0x020A, "LOCK_CAR_DOORS", "vi"),
        (0x0223, "SET_CHAR_HEALTH", "pi"),
        (0x0224, "SET_CAR_HEALTH", "vi"),
        (0x0229, "CHANGE_CAR_COLOUR", "vii"),
        (0x02A3, "SWITCH_WIDESCREEN", "i"),
        (0x02AB, "SET_CHAR_PROOFS", "piiiii"),
        (0x02AC, "SET_CAR_PROOFS", "viiiii"),
        (0x02EB, "RESTORE_CAMERA_JUMPCUT", ""),
        (0x0330, "SET_PLAYER_NEVER_GETS_TIRED", "Pi"),
        (0x0373, "SET_CAMERA_BEHIND_PLAYER", ""),
        (0x0397, "SWITCH_CAR_SIREN", "vi"),
        (0x03C0, "STORE_CAR_CHAR_IS_IN_NO_SAVE", "po"),
        (0x03DE, "SET_PED_DENSITY_MULTIPLIER", "f"),
        (0x04BB, "SET_AREA_VISIBLE", "i"),
        (0x0581, "DISPLAY_RADAR", "i"),
        (0x05CB, "TASK_ENTER_CAR_AS_DRIVER", "pvi"),
        (0x0687, "CLEAR_CHAR_TASKS", "p"),
        (0x0826, "DISPLAY_HUD", "i"),
    ];

    /// The parameters of `opcode` ([`COMMANDS`]), if a mod may run it.
    pub fn params(opcode: i32) -> Option<&'static str> {
        COMMANDS.iter().find(|c| i32::from(c.0) == opcode).map(|c| c.2)
    }
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
            /// (1.2) Subscribes to the game events of `mask` (bits `1 << event::KIND`; replaces the earlier
            /// subscription). Returns the mask granted: the events the manifest's permissions allow
            /// ([`event::permission`]).
            [None] fn events_subscribe(mask: i32) -> i32;
            /// (1.2) The game state machine's value (`gGameState`: 7 frontend idle, 8 loading, 9 playing).
            [State] fn game_state() -> i32;
            /// (1.2) Writes the local player ([`state::PlayerInfo`]) to the buffer (`buf_ptr`, `buf_len` bytes).
            /// Returns the bytes written, `BAD_ARG` (the buffer is too small or outside the memory).
            [State] fn player_info(buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.2) Writes the ped `handle` ([`state::PedInfo`]) to the buffer, as `player_info`; `NOT_FOUND`
            /// when the handle names no live ped.
            [State] fn ped_info(handle: i32, buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.2) Writes the vehicle `handle` ([`state::VehicleInfo`]) to the buffer, as `ped_info`.
            [State] fn vehicle_info(handle: i32, buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.2) Writes time, weather and the world ([`state::WorldInfo`]) to the buffer, as `player_info`.
            [State] fn world_info(buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.2) Writes the camera ([`state::CameraInfo`]) to the buffer, as `player_info`.
            [State] fn camera_info(buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.2) Writes the handles of the peds in the ped pool (one `u32` each, slot order) to the buffer, as
            /// many as fit, and returns how many there are.
            [State] fn ped_list(buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.2) The vehicles in the vehicle pool, as `ped_list`.
            [State] fn vehicle_list(buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.2) 1 while the key with Windows virtual-key code `vk` (0..=255) is held, else 0.
            [Input] fn key_down(vk: i32) -> i32;
            /// (1.2) Subscribes to the game function `name` (UTF-8, `name_len` bytes at `name_ptr`: its name in
            /// the hook registry, e.g. `CPed::Render`), which the manifest's `hooks` list must name, in the
            /// `phases` [`hook::BEFORE`] / [`hook::AFTER`]. Returns the hook's id (passed to `sa_hook`),
            /// `DENIED` (not in the manifest), `NOT_FOUND` (the registry has no such function).
            [Hooks] fn hook_subscribe(name_ptr: i32, name_len: i32, phases: i32) -> i32;
            /// (1.2) In `sa_hook`: the number of arguments of the call.
            [Hooks] fn hook_arg_count() -> i32;
            /// (1.2) In `sa_hook`: argument `index` (`0 .. hook_arg_count()`) or, with `index` -1 in the after
            /// phase, the result: writes its word (4 bytes) at `out_ptr` and returns its kind
            /// ([`hook::Value::encode`]); `NOT_FOUND` when there is no such value, `BAD_ARG` outside `sa_hook`.
            [Hooks] fn hook_value(index: i32, out_ptr: i32) -> i32;
            /// (1.3) In `sa_hook`'s before phase, for a function of [`hook::CONTROLLABLE`]: replaces argument `index`
            /// with `word`, read as the argument's kind (an integer, a float's bits, or the handle of a live ped,
            /// vehicle or object for an entity argument). `DENIED` for a function outside the list, `BAD_ARG`
            /// outside the before phase or for a hidden pointer, `NOT_FOUND` for a stale handle.
            [HookControl] fn hook_set_arg(index: i32, word: i32) -> i32;
            /// (1.3) In `sa_hook`'s before phase, for a function of [`hook::CONTROLLABLE`]: the game does not run
            /// the call; its caller gets `result` (read as the result's kind; ignored for a function without
            /// one). Later hooks of the same call are still told (with the arguments as they are).
            [HookControl] fn hook_skip(result: i32) -> i32;
            /// (1.3) In `sa_hook`'s after phase, for a function of [`hook::CONTROLLABLE`]: replaces the result with
            /// `result` (read as the result's kind).
            [HookControl] fn hook_set_result(result: i32) -> i32;
            /// (1.3) Moves ped `handle` to (`x`, `y`, `z`) (out of a vehicle first, onto the ground).
            [Write] fn set_ped_position(handle: i32, x: f32, y: f32, z: f32) -> i32;
            /// (1.3) Turns ped `handle` to `degrees` (0 = north, counter-clockwise).
            [Write] fn set_ped_heading(handle: i32, degrees: f32) -> i32;
            /// (1.3) Sets ped `handle`'s health.
            [Write] fn set_ped_health(handle: i32, health: f32) -> i32;
            /// (1.3) Sets ped `handle`'s armour.
            [Write] fn set_ped_armour(handle: i32, armour: f32) -> i32;
            /// (1.3) Gives ped `handle` weapon `weapon` (`eWeaponType`) with `ammo` rounds (its model loaded first).
            [Write] fn give_weapon(handle: i32, weapon: i32, ammo: i32) -> i32;
            /// (1.3) Sets the local player's money (and the HUD's displayed amount).
            [Write] fn set_money(money: i32) -> i32;
            /// (1.3) Sets the local player's wanted level (0..=6).
            [Write] fn set_wanted_level(level: i32) -> i32;
            /// (1.3) Sets the game clock.
            [Write] fn set_clock(hour: i32, minute: i32) -> i32;
            /// (1.3) Sets the weather at once (`eWeatherType`).
            [Write] fn set_weather(weather: i32) -> i32;
            /// (1.3) Moves vehicle `handle` to (`x`, `y`, `z`).
            [Write] fn set_vehicle_position(handle: i32, x: f32, y: f32, z: f32) -> i32;
            /// (1.3) Turns vehicle `handle` to `degrees`.
            [Write] fn set_vehicle_heading(handle: i32, degrees: f32) -> i32;
            /// (1.3) Sets vehicle `handle`'s health (1000 new, below 250 burning).
            [Write] fn set_vehicle_health(handle: i32, health: f32) -> i32;
            /// (1.3) Sets vehicle `handle`'s colours (`carcols.dat` indices).
            [Write] fn set_vehicle_colours(handle: i32, primary: i32, secondary: i32) -> i32;
            /// (1.3) Creates a vehicle of `model` at (`x`, `y`, `z`) facing `degrees` (the model is loaded first).
            /// Returns its handle. The mod owns it: it is deleted when the mod is disabled. `WRONG_PHASE` inside a
            /// hook.
            [Entities] fn create_vehicle(model: i32, x: f32, y: f32, z: f32, degrees: f32) -> i32;
            /// (1.3) Creates a civilian ped of `model` at (`x`, `y`, `z`), as `create_vehicle`.
            [Entities] fn create_ped(model: i32, x: f32, y: f32, z: f32) -> i32;
            /// (1.3) Creates an object of `model` at (`x`, `y`, `z`), as `create_vehicle`.
            [Entities] fn create_object(model: i32, x: f32, y: f32, z: f32) -> i32;
            /// (1.3) Deletes vehicle `handle`, which the mod created; `DENIED` for any other vehicle.
            [Entities] fn destroy_vehicle(handle: i32) -> i32;
            /// (1.3) Deletes ped `handle`, which the mod created; `DENIED` for any other ped.
            [Entities] fn destroy_ped(handle: i32) -> i32;
            /// (1.3) Deletes object `handle`, which the mod created; `DENIED` for any other object.
            [Entities] fn destroy_object(handle: i32) -> i32;
            /// (1.3) Runs script command `opcode` ([`script::COMMANDS`]) with the `args_len` argument words at
            /// `args_ptr`, one per parameter (an integer, a float's bits, a handle; any value for an output), and
            /// writes the outputs, one word each, to `out_ptr`. Returns the condition result (0 / 1). `DENIED` for
            /// an opcode outside the list, `BAD_ARG` for a wrong argument count, `NOT_FOUND` for a stale handle,
            /// `WRONG_PHASE` inside a hook.
            [Scripts] fn script_command(opcode: i32, args_ptr: i32, args_len: i32, out_ptr: i32) -> i32;
            /// (1.3) Plays front-end sound `event` (`AE_FRONTEND_*`, 1..=46) at playback speed `speed`.
            [Audio] fn play_frontend_sound(event: i32, speed: f32) -> i32;
            /// (1.3) Plays mission sound `sound` (`ADD_ONE_OFF_SOUND`) at (`x`, `y`, `z`).
            [Audio] fn play_sound(sound: i32, x: f32, y: f32, z: f32) -> i32;
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
            /// (1.2) A game event the mod subscribed to (`events_subscribe`): its kind ([`event`]) and values.
            /// Returns 0.
            [Optional] fn sa_event(kind: i32, a: i32, b: i32) -> i32;
            /// (1.2) A hooked game function (`hook_subscribe`) runs: `hook` is the id `hook_subscribe` returned,
            /// `phase` [`hook::BEFORE`] or [`hook::AFTER`]; its values through `hook_arg_count` / `hook_value`.
            /// Called in the middle of the game's frame: the state functions read the state as it is at that
            /// moment. Returns 0.
            [Optional] fn sa_hook(hook: i32, phase: i32) -> i32;
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
/// The name of the `sa_event` export (1.2).
pub const EXPORT_EVENT: &str = "sa_event";
/// The name of the `sa_hook` export (1.2).
pub const EXPORT_HOOK: &str = "sa_hook";

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
        for n in [EXPORT_API_VERSION, EXPORT_INIT, EXPORT_OVERLAY, EXPORT_SETTINGS, EXPORT_RESIZE, EXPORT_EVENT, EXPORT_HOOK] {
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

    #[test]
    fn level_two() {
        assert_eq!(IMPORTS[import_index("player_info").unwrap()].permission, Permission::State);
        assert_eq!(IMPORTS[import_index("hook_subscribe").unwrap()].permission, Permission::Hooks);
        assert_eq!(event::permission(event::FRAME_END), Permission::None);
        assert_eq!(event::permission(event::KEY_UP), Permission::Input);
        assert_eq!(event::permission(event::VEHICLE_ENTER), Permission::State);
        for v in [
            hook::Value::Int(-5),
            hook::Value::Float(2.5),
            hook::Value::Ped(sa_types::PedRef(0x205)),
            hook::Value::Vehicle(sa_types::VehicleRef(7)),
            hook::Value::Object(sa_types::ObjectRef(9)),
            hook::Value::Pointer,
        ] {
            let (k, w) = v.encode();
            assert_eq!(hook::Value::decode(k, w), Some(v));
        }
        assert_eq!(hook::Value::decode(6, 0), None);
        assert!(!export(EXPORT_HOOK).unwrap().required);
    }

    #[test]
    fn level_three() {
        assert_eq!(IMPORTS[import_index("create_vehicle").unwrap()].permission, Permission::Entities);
        assert_eq!(IMPORTS[import_index("hook_skip").unwrap()].permission, Permission::HookControl);
        assert_eq!(IMPORTS[import_index("set_money").unwrap()].permission, Permission::Write);
        assert!(hook::controllable("CWeapon::Fire") && !hook::controllable("CGame::Process"));
        assert_eq!(script::params(0x01B4), Some("Pi"));
        assert_eq!(script::params(0x00A5), None);
        let ops: Vec<u16> = script::COMMANDS.iter().map(|c| c.0).collect();
        assert!(ops.windows(2).all(|w| w[0] < w[1]), "sorted, no duplicates");
        assert!(script::COMMANDS.iter().all(|c| c.2.len() <= script::MAX_ARGS && c.2.chars().all(|k| "ifpvPo".contains(k))));
    }
}
