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
pub const API_MINOR: u16 = 9;

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
    /// Nothing now, try again later: no datagram is queued (`net_recv`), or the frame's send budget is used up
    /// (`net_send`).
    pub const AGAIN: i32 = -6;
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
    /// `net = ["udp:HOST:PORT", ...]`: UDP sockets to the servers the rules allow (API 1.4).
    Net,
    /// `storage = true`: the mod's persistent key / value storage (API 1.4).
    Storage,
    /// `http = ["https://host/path/", ...]`: GET requests to URLs under these prefixes (API 1.7).
    Http,
    /// `input = "capture"`: the platform's input events before the game sees them, and taking the keyboard and
    /// the mouse from the game (API 1.5).
    Capture,
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
    /// The longest text argument, in bytes (without its NUL).
    pub const MAX_TEXT: usize = 63;

    /// The commands a mod may run, with their parameters, one letter each: `i` an integer, `f` a float, `p` a ped
    /// handle, `v` a vehicle handle, `P` a player (0, the local player), `o` an output, `s` a text (1.5: the word is
    /// the address in the mod's memory of a NUL-terminated string of at most [`MAX_TEXT`] bytes). Gameplay commands
    /// whose handlers the engine has ported and that act on the player, the world, the camera, the HUD or an entity named
    /// by its handle; the host checks every handle before the command runs (the game's handlers trust them).
    /// Entity creation and deletion are not here: they are `create_*` / `destroy_*`, which keep track of what a mod
    /// owns.
    pub const COMMANDS: &[(u16, &str, &str)] = &[
        (0x00BE, "CLEAR_PRINTS", ""),
        (0x00C0, "SET_TIME_OF_DAY", "ii"),
        (0x00DF, "IS_CHAR_IN_ANY_CAR", "p"),
        (0x00EC, "LOCATE_CHAR_ANY_MEANS_2D", "pffffi"),
        (0x00FE, "LOCATE_CHAR_ANY_MEANS_3D", "pffffffi"),
        (0x0109, "ADD_SCORE", "Pi"),
        (0x0110, "CLEAR_WANTED_LEVEL", "P"),
        (0x0118, "IS_CHAR_DEAD", "p"),
        (0x0119, "IS_CAR_DEAD", "v"),
        (0x015F, "SET_FIXED_CAMERA_POSITION", "ffffff"),
        (0x0160, "POINT_CAMERA_AT_POINT", "fffi"),
        (0x016C, "ADD_HOSPITAL_RESTART", "ffffi"),
        (0x01B4, "SET_PLAYER_CONTROL", "Pi"),
        (0x01B6, "FORCE_WEATHER_NOW", "i"),
        (0x01B7, "RELEASE_WEATHER", ""),
        (0x01B9, "SET_CURRENT_CHAR_WEAPON", "pi"),
        (0x01E8, "SWITCH_ROADS_OFF", "ffffff"),
        (0x01EB, "SET_CAR_DENSITY_MULTIPLIER", "f"),
        (0x01F0, "SET_MAX_WANTED_LEVEL", "i"),
        (0x020A, "LOCK_CAR_DOORS", "vi"),
        (0x0223, "SET_CHAR_HEALTH", "pi"),
        (0x0224, "SET_CAR_HEALTH", "vi"),
        (0x0229, "CHANGE_CAR_COLOUR", "vii"),
        (0x0247, "REQUEST_MODEL", "i"),
        (0x0248, "HAS_MODEL_LOADED", "i"),
        (0x02A3, "SWITCH_WIDESCREEN", "i"),
        (0x02AB, "SET_CHAR_PROOFS", "piiiii"),
        (0x02AC, "SET_CAR_PROOFS", "viiiii"),
        (0x02EB, "RESTORE_CAMERA_JUMPCUT", ""),
        (0x0321, "EXPLODE_CHAR_HEAD", "p"),
        (0x0330, "SET_PLAYER_NEVER_GETS_TIRED", "Pi"),
        (0x0362, "WARP_CHAR_FROM_CAR_TO_COORD", "pfff"),
        (0x036A, "WARP_CHAR_INTO_CAR", "pv"),
        (0x0373, "SET_CAMERA_BEHIND_PLAYER", ""),
        (0x038B, "LOAD_ALL_MODELS_NOW", ""),
        (0x0397, "SWITCH_CAR_SIREN", "vi"),
        (0x03C0, "STORE_CAR_CHAR_IS_IN_NO_SAVE", "po"),
        (0x03DE, "SET_PED_DENSITY_MULTIPLIER", "f"),
        (0x0430, "WARP_CHAR_INTO_CAR_AS_PASSENGER", "pvi"),
        (0x0446, "SET_CHAR_SUFFERS_CRITICAL_HITS", "pi"),
        (0x048F, "REMOVE_ALL_CHAR_WEAPONS", "p"),
        (0x04BB, "SET_AREA_VISIBLE", "i"),
        (0x04D7, "FREEZE_CHAR_POSITION", "pi"),
        (0x04E4, "REQUEST_COLLISION", "ff"),
        (0x04ED, "REQUEST_ANIMATION", "s"),
        (0x04EE, "HAS_ANIMATION_LOADED", "s"),
        (0x052C, "SET_PLAYER_DRUNKENNESS", "Pi"),
        (0x0581, "DISPLAY_RADAR", "i"),
        (0x05C4, "TASK_HANDS_UP", "pi"),
        (0x05CA, "TASK_ENTER_CAR_AS_PASSENGER", "pvii"),
        (0x05CB, "TASK_ENTER_CAR_AS_DRIVER", "pvi"),
        (0x05CD, "TASK_LEAVE_CAR", "pv"),
        (0x060B, "SET_CHAR_DECISION_MAKER", "pi"),
        (0x0687, "CLEAR_CHAR_TASKS", "p"),
        (0x06BD, "IS_LINE_OF_SIGHT_CLEAR", "ffffffiiiii"),
        (0x06D7, "SWITCH_RANDOM_TRAINS", "i"),
        (0x0792, "CLEAR_CHAR_TASKS_IMMEDIATELY", "p"),
        (0x0812, "TASK_PLAY_ANIM_NON_INTERRUPTABLE", "pssfiiiii"),
        (0x0826, "DISPLAY_HUD", "i"),
        (0x0860, "SET_CHAR_AREA_VISIBLE", "pi"),
        (0x087E, "SET_CHAR_DROPS_WEAPONS_WHEN_DEAD", "pi"),
        (0x09BA, "DISPLAY_ZONE_NAMES", "i"),
    ];

    /// The parameters of `opcode` ([`COMMANDS`]), if a mod may run it.
    pub fn params(opcode: i32) -> Option<&'static str> {
        COMMANDS.iter().find(|c| i32::from(c.0) == opcode).map(|c| c.2)
    }
}

/// Network sockets (API 1.4, `net = [...]`): connected, non-blocking UDP sockets, natively real ones, in the browser
/// relayed through a WebSocket to the page's UDP proxy (one datagram per binary frame).
pub mod net {
    /// `net_state`: opening (the browser's WebSocket to the proxy); datagrams sent now are queued.
    pub const CONNECTING: i32 = 0;
    /// `net_state`: datagrams flow.
    pub const OPEN: i32 = 1;
    /// `net_state`: closed or refused (the log says why); sends fail, received datagrams can still be read.
    pub const CLOSED: i32 = 2;
    /// Sockets one mod may hold open at once.
    pub const MAX_SOCKETS: usize = 8;
    /// The largest datagram `net_send` sends (the largest UDP payload over IPv4).
    pub const MAX_DATAGRAM: usize = 65_507;
    /// Datagrams one mod may send per main-loop iteration, over all its sockets.
    pub const MAX_SENDS_PER_FRAME: u32 = 1024;
    /// Bytes one mod may send per main-loop iteration, over all its sockets.
    pub const MAX_SEND_BYTES_PER_FRAME: usize = 1 << 20;
}

/// HTTP requests (API 1.7, `http = [...]`): GET only, run by the host in the background (natively a thread per
/// request, in the browser `fetch`).
pub mod http {
    /// Requests one mod may hold at once (running or finished, until `http_close`).
    pub const MAX_REQUESTS: usize = 4;
    /// The longest URL.
    pub const MAX_URL: usize = 2048;
    /// The largest response body; a longer one fails the request.
    pub const MAX_BODY: usize = 16 << 20;
    /// How long a request may take, from `http_get` to the whole body.
    pub const TIMEOUT_MS: u64 = 30_000;

    /// Whether `prefix` is a valid `http` permission entry: a [`valid_url`] whose path starts with `/` (so that
    /// `https://example.org/` cannot be stretched to `https://example.org.evil/`).
    pub fn valid_prefix(prefix: &str) -> bool {
        let rest = prefix.strip_prefix("https://").or_else(|| prefix.strip_prefix("http://")).unwrap_or("");
        rest.find('/').is_some_and(|i| i > 0) && valid_url(prefix)
    }

    /// Whether `url` is a URL the host requests: `http://` or `https://` with a host, at most [`MAX_URL`] bytes, no
    /// whitespace, control characters, backslash or fragment, and no `..` or `%2e` (a path cannot climb out of the
    /// prefix that allowed it).
    pub fn valid_url(url: &str) -> bool {
        let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"));
        rest.is_some_and(|r| !r.is_empty() && !r.starts_with('/'))
            && url.len() <= MAX_URL
            && url.bytes().all(|b| b.is_ascii_graphic() && b != b'#' && b != b'\\')
            && !url.contains("..")
            && !url.to_ascii_lowercase().contains("%2e")
    }

    /// Whether the `http` list `prefixes` allows `url`.
    pub fn allows(prefixes: &[String], url: &str) -> bool {
        valid_url(url) && prefixes.iter().any(|p| url.starts_with(p.as_str()))
    }
}

/// Overlay images (API 1.6, `render = true`): RGBA8 pixels a mod uploads once and draws quads of.
pub mod image {
    /// Images one mod may hold at once.
    pub const MAX_IMAGES: usize = 64;
    /// The longest side of an image, in pixels.
    pub const MAX_SIDE: u32 = 4096;
    /// The pixel bytes (`width * height * 4`) one mod's images may hold together.
    pub const MAX_BYTES: usize = 32 << 20;
    /// `image_create` flag: linear filtering over a box-filtered mip chain when a quad scales the image (else the
    /// nearest texel).
    pub const SMOOTH: i32 = 1;
    /// `overlay_quads`: the image handle of plain-colour quads.
    pub const NONE: i32 = -1;
    /// `overlay_quads`: words per quad (`x`, `y`, `w`, `h`, `u0`, `v0`, `u1`, `v1` as `f32`, then `rgba`).
    pub const QUAD_WORDS: usize = 9;
    /// Quads one `overlay_quads` call draws at most.
    pub const MAX_QUADS_PER_CALL: usize = 16_384;
}

/// The game's 2D drawing (API 1.6): text in its font (`game_text`, the [`sa_types::text::FontText`] record as
/// words) and its textures (`game_texture`, `game_sprite`).
pub mod text {
    use sa_types::text::FontText;

    /// Words of a [`FontText`]: `x`, `y`, `scale.0`, `scale.1` (`f32`), `colour`, `orientation`, `wrap_x`,
    /// `centre_size` (`f32`), `background`, `background_colour`, `proportional`, `drop_colour`, `edge`, `shadow`,
    /// `style`.
    pub const WORDS: usize = 15;
    /// The longest string, in bytes.
    pub const MAX_TEXT: usize = 400;
    /// Strings and sprites one mod may queue per frame.
    pub const MAX_PER_FRAME: usize = 256;
    /// Game textures one mod may hold (`game_texture`).
    pub const MAX_TEXTURES: usize = 256;

    /// `t` as words.
    pub fn encode(t: &FontText) -> [u32; WORDS] {
        let f = f32::to_bits;
        [
            f(t.x),
            f(t.y),
            f(t.scale.0),
            f(t.scale.1),
            t.colour,
            u32::from(t.orientation),
            f(t.wrap_x),
            f(t.centre_size),
            u32::from(t.background),
            t.background_colour,
            u32::from(t.proportional),
            t.drop_colour,
            u32::from(t.edge),
            u32::from(t.shadow),
            u32::from(t.style),
        ]
    }

    /// The record of `w`; `None` for a float that is not finite or a value out of range (orientation 0..=2, style
    /// 0..=3, edge and shadow 0..=8).
    pub fn decode(w: &[u32; WORDS]) -> Option<FontText> {
        let f = |i: usize| Some(f32::from_bits(w[i])).filter(|v| v.is_finite());
        let small = |i: usize, max: u32| u8::try_from(w[i]).ok().filter(|&v| u32::from(v) <= max);
        Some(FontText {
            x: f(0)?,
            y: f(1)?,
            scale: (f(2)?, f(3)?),
            colour: w[4],
            orientation: small(5, 2)?,
            wrap_x: f(6)?,
            centre_size: f(7)?,
            background: w[8] != 0,
            background_colour: w[9],
            proportional: w[10] != 0,
            drop_colour: w[11],
            edge: small(12, 8)?,
            shadow: small(13, 8)?,
            style: small(14, 3)?,
        })
    }
}

/// Pickups (API 1.8).
pub mod pickup {
    /// The highest pickup type (`ePickupType`) `create_pickup` accepts.
    pub const MAX_TYPE: i32 = 22;
}

/// Checkpoint markers (API 1.8).
pub mod checkpoint {
    /// The highest checkpoint type `create_checkpoint` accepts (the game's race checkpoint types).
    pub const MAX_TYPE: i32 = 8;
}

/// Radar blips (API 1.8).
pub mod blip {
    /// `create_coord_blip` flag: shown on the whole radar and map (`ADD_SPRITE_BLIP_FOR_COORD`), else only near
    /// the player (`ADD_SHORT_RANGE_SPRITE_BLIP_FOR_COORD`).
    pub const GLOBAL: i32 = 1;
    /// `create_coord_blip` flag: a contact point (`ADD_SPRITE_BLIP_FOR_CONTACT_POINT` and its short-range form).
    pub const CONTACT_POINT: i32 = 2;
    /// `create_entity_blip` kind: a ped.
    pub const PED: i32 = 0;
    /// `create_entity_blip` kind: a vehicle.
    pub const VEHICLE: i32 = 1;
}

/// Radar areas (API 1.8): coloured world rectangles on the radar and the map.
pub mod radar {
    /// Radar area ids one mod may use (`0 .. MAX_AREAS`).
    pub const MAX_AREAS: i32 = 1024;
/// Models (API 1.9): `model_info`'s answer.
pub mod model {
    use sa_types::ModelKind;

    /// Model ids (`CModelInfo::ms_modelInfoPtrs`).
    pub const COUNT: i32 = 20_000;
    /// `model_info`: no model info.
    pub const NONE: i32 = 0;
    /// `model_info`: an atomic model (buildings and most objects).
    pub const ATOMIC: i32 = 1;
    /// `model_info`: a clump model.
    pub const CLUMP: i32 = 2;
    /// `model_info`: a ped.
    pub const PED: i32 = 3;
    /// `model_info`: a vehicle.
    pub const VEHICLE: i32 = 4;
    /// `model_info`: a weapon.
    pub const WEAPON: i32 = 5;
    /// `model_info` bit: the model is loaded.
    pub const LOADED: i32 = 0x100;

    /// The kind's number.
    pub fn kind_number(k: ModelKind) -> i32 {
        match k {
            ModelKind::None => NONE,
            ModelKind::Atomic => ATOMIC,
            ModelKind::Clump => CLUMP,
            ModelKind::Ped => PED,
            ModelKind::Vehicle => VEHICLE,
            ModelKind::Weapon => WEAPON,
        }
    }

    /// The kind of a `model_info` answer.
    pub fn kind(info: i32) -> ModelKind {
        match info & 0xFF {
            ATOMIC => ModelKind::Atomic,
            CLUMP => ModelKind::Clump,
            PED => ModelKind::Ped,
            VEHICLE => ModelKind::Vehicle,
            WEAPON => ModelKind::Weapon,
            _ => ModelKind::None,
        }
    }
}

/// `attach_object`'s target kinds (API 1.9).
pub mod attach {
    /// A ped.
    pub const PED: i32 = 0;
    /// A vehicle.
    pub const VEHICLE: i32 = 1;
    /// An object.
    pub const OBJECT: i32 = 2;
}

/// Object materials (API 1.9, `set_object_material`).
pub mod material {
    /// Material slots of an object.
    pub const SLOTS: i32 = 16;
    /// The longest dictionary or texture name, in bytes.
    pub const MAX_NAME: usize = 31;
}

/// The persistent key / value storage (API 1.4, `storage = true`): one area per mod id, natively a folder under
/// the user's data directory, in the browser the page's IndexedDB.
pub mod storage {
    /// The largest value.
    pub const MAX_VALUE: usize = 1 << 20;
    /// The most keys one mod keeps.
    pub const MAX_KEYS: usize = 1024;
    /// The most bytes (values together) one mod keeps.
    pub const MAX_TOTAL: usize = 16 << 20;

    /// True for a key the storage functions accept: 1..=64 bytes of `A-Z a-z 0-9 . _ -`, not starting with `.`.
    pub fn valid_key(key: &str) -> bool {
        (1..=64).contains(&key.len())
            && key.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            && !key.starts_with('.')
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
            /// (1.4) Writes two `u64` (little-endian, 16 bytes) at `out_ptr`: the microseconds since the host
            /// started (monotonic) and the wall clock as microseconds since 1970-01-01 UTC. Returns 0.
            [None] fn clock(out_ptr: i32) -> i32;
            /// (1.4) Opens a connected, non-blocking UDP socket to `target` (UTF-8, `target_len` bytes at
            /// `target_ptr`: `HOST:PORT`, `[IPV6]:PORT`), which a rule of the manifest's `net` list must allow.
            /// Returns the socket's handle (`0 .. net::MAX_SOCKETS`), `DENIED` (no rule allows the target),
            /// `BAD_ARG` (not `HOST:PORT`), `FAILED` (it does not resolve, the mod holds `net::MAX_SOCKETS`
            /// sockets; the log says why). In the browser the socket starts `net::CONNECTING`.
            [Net] fn net_open(target_ptr: i32, target_len: i32) -> i32;
            /// (1.4) The state of socket `handle`: `net::CONNECTING`, `net::OPEN`, `net::CLOSED`; `NOT_FOUND`
            /// for a handle the mod does not hold.
            [Net] fn net_state(handle: i32) -> i32;
            /// (1.4) Sends one datagram (`data_len` bytes at `data_ptr`, at most `net::MAX_DATAGRAM`) without
            /// blocking. Returns its length, `AGAIN` (the frame's send budget is used up), `FAILED` (the socket
            /// is closed or the send failed), `NOT_FOUND`.
            [Net] fn net_send(handle: i32, data_ptr: i32, data_len: i32) -> i32;
            /// (1.4) Takes one received datagram into the buffer (`buf_ptr`, `buf_len`; truncated to the
            /// buffer, as UDP does) without blocking. Returns its length, `AGAIN` (none queued), `NOT_FOUND`.
            [Net] fn net_recv(handle: i32, buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.4) Closes socket `handle` (the host closes every socket of a mod it disables). Returns 0 or
            /// `NOT_FOUND`.
            [Net] fn net_close(handle: i32) -> i32;
            /// (1.4) Reads the value of `key` (UTF-8, `key_len` bytes at `key_ptr`, [`storage::valid_key`]) into
            /// the buffer: copies `min(size, buf_len)` bytes and returns the value's size, or `NOT_FOUND`.
            [Storage] fn storage_read(key_ptr: i32, key_len: i32, buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.4) Writes (replaces) the value of `key` (at most `storage::MAX_VALUE` bytes). Returns 0,
            /// `FAILED` (over `storage::MAX_KEYS` / `storage::MAX_TOTAL`, or the storage could not write; the
            /// log says why).
            [Storage] fn storage_write(key_ptr: i32, key_len: i32, data_ptr: i32, data_len: i32) -> i32;
            /// (1.4) Deletes `key`. Returns 0 or `NOT_FOUND`.
            [Storage] fn storage_delete(key_ptr: i32, key_len: i32) -> i32;
            /// (1.4) Writes the mod's keys, sorted, each followed by `\n`, into the buffer (as many bytes as
            /// fit) and returns the length of the whole list.
            [Storage] fn storage_list(buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.5) 1 while the pause menu is up, else 0.
            [State] fn menu_active() -> i32;
            /// (1.5) Writes the local player's pad ([`state::PadInfo`]: the game's controller state, sticks and
            /// buttons as the game reads them this frame) to the buffer, as `player_info`.
            [Input] fn pad_state(buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.5) Writes ped `handle`'s transform ([`state::MatrixInfo`]) to the buffer, as `ped_info`.
            [State] fn ped_matrix(handle: i32, buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.5) Writes vehicle `handle`'s transform ([`state::MatrixInfo`]) to the buffer, as `vehicle_info`.
            [State] fn vehicle_matrix(handle: i32, buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.5) Gives the player ped `handle` the ped model `model` (loaded first). `FAILED` for a ped that is no
            /// player's or a model that is not a ped's.
            [Write] fn set_ped_model(handle: i32, model: i32) -> i32;
            /// (1.5) Shows (1) or hides (0) the HUD and the radar.
            [Write] fn set_hud_visible(on: i32) -> i32;
            /// (1.5) Switches one of the game's own features ([`option`]) on (1) or off (0). The host turns every
            /// option back on when it disables the mod.
            [Write] fn set_game_option(option: i32, on: i32) -> i32;
            /// (1.5) Shows `text` (UTF-8, `text_len` bytes; the game's font draws Latin-1, anything else as `?`) as the
            /// game's big message in style `style` (0..=6) for `time_ms`.
            [Write] fn big_message(text_ptr: i32, text_len: i32, time_ms: i32, style: i32) -> i32;
            /// (1.5) What the mod takes from the game until it says otherwise: `keyboard` (the game sees no keys
            /// and no held key), `mouse` (no mouse movement or buttons) and `cursor` (the platform shows its
            /// pointer: natively the window's, in the browser the page releases the pointer lock). Returns 0.
            [Capture] fn input_capture(keyboard: i32, mouse: i32, cursor: i32) -> i32;
            /// (1.6) Creates an overlay image of `width` x `height` RGBA8 pixels (`data_len` = `width * height * 4`
            /// bytes at `data_ptr`, rows from the top, straight alpha); `flags`: [`image::SMOOTH`]. Returns its
            /// handle, `BAD_ARG` (a side of 0 or above [`image::MAX_SIDE`], a wrong length), `FAILED` (the mod holds
            /// [`image::MAX_IMAGES`] images or [`image::MAX_BYTES`] bytes). Valid in every phase; the image can be
            /// drawn from the next overlay phase on.
            [Render] fn image_create(width: i32, height: i32, flags: i32, data_ptr: i32, data_len: i32) -> i32;
            /// (1.6) Deletes image `handle`. Returns 0 or `NOT_FOUND`.
            [Render] fn image_destroy(handle: i32) -> i32;
            /// (1.6) Draws `count` quads of image `handle` ([`image::NONE`]: plain colour) in the 2D overlay, in
            /// order: [`image::QUAD_WORDS`] little-endian words each at `quads_ptr` (`x`, `y`, `w`, `h` in pixels from
            /// the top left, `u0`, `v0`, `u1`, `v1` the image's texture coordinates 0..=1, `rgba` `0xRRGGBBAA` tinting
            /// the image), cut to the clip rectangle (`overlay_clip`). Returns the quads drawn (outside the clip
            /// rectangle none), `NOT_FOUND` (no such image), `BAD_ARG` (outside the overlay phase, more than
            /// [`image::MAX_QUADS_PER_CALL`], a value that is not finite, the frame's quad budget used up).
            [Render] fn overlay_quads(handle: i32, quads_ptr: i32, count: i32) -> i32;
            /// (1.6) Cuts the following overlay drawing of this phase (rectangles, text, quads) to the rectangle
            /// (`x`, `y`, `w`, `h`); a negative `w` or `h` removes the clip. Every overlay phase starts without one.
            /// Returns 0.
            [Render] fn overlay_clip(x: f32, y: f32, w: f32, h: f32) -> i32;
            /// (1.6) Prints `text` (`text_len` bytes at `text_ptr`, at most [`text::MAX_TEXT`]; the game font's
            /// code page, its `~x~` codes) in the game's own font with the `CFont` state of the [`text::WORDS`]
            /// words at `style_ptr` ([`text::encode`]). The game draws it in its next frame's 2D phase, with the HUD's
            /// text (under the overlay). Returns 0, `BAD_ARG` (outside the overlay phase, a bad record, over
            /// [`text::MAX_PER_FRAME`] strings this frame).
            [Render] fn game_text(style_ptr: i32, text_ptr: i32, text_len: i32) -> i32;
            /// (1.6) Where world point (`x`, `y`, `z`) is on the screen with the game's camera
            /// (`CSprite::CalcScreenCoors`): writes three `f32` (screen x and y in pixels, the depth in front of the
            /// camera) at `out_ptr` and returns 1, or 0 when it is off screen or behind the camera.
            [State] fn world_to_screen(x: f32, y: f32, z: f32, out_ptr: i32) -> i32;
            /// (1.6) The texture `name` of the game's texture dictionary `dict` (UTF-8, `(ptr, len)` each); without a
            /// dictionary of that name, the game folder's file `file` (a relative path, `\` or `/`; empty: none) is
            /// loaded into a new one, which stays loaded. The game looks it up in its next 2D phase: until then the
            /// call returns `AGAIN`, afterwards a handle for `game_sprite` (the same for the same texture) or
            /// `NOT_FOUND` (no such file or texture). `FAILED`: the mod asked for [`text::MAX_TEXTURES`] textures;
            /// `WRONG_PHASE`: inside a hook.
            [Render] fn game_texture(dict_ptr: i32, dict_len: i32, file_ptr: i32, file_len: i32, name_ptr: i32, name_len: i32) -> i32;
            /// (1.6) Draws game texture `texture` (`game_texture`) into the rectangle (`x`, `y`, `w`, `h`) tinted with
            /// `rgba` (`0xRRGGBBAA`), as the game draws its 2D sprites (`CSprite2d::Draw`, bilinear), in the next
            /// frame's 2D phase in order with `game_text` (under the overlay). Returns 0, `NOT_FOUND`, `BAD_ARG`
            /// (outside the overlay phase, over [`text::MAX_PER_FRAME`] this frame).
            [Render] fn game_sprite(texture: i32, x: f32, y: f32, w: f32, h: f32, rgba: i32) -> i32;
            /// (1.6) Sets the game's millisecond timer (`CTimer::m_snTimeInMilliseconds`, 0..=0x3FFFFFFF): the time
            /// the game's timed events count in.
            [Write] fn set_game_time(ms: i32) -> i32;
            /// (1.7) Starts a GET request of `url` (UTF-8, `url_len` bytes at `url_ptr`, at most [`http::MAX_URL`],
            /// `http://` or `https://`), which must start with a prefix of the manifest's `http` list; it runs in
            /// the background. Returns the request's handle (`0 .. http::MAX_REQUESTS`), `DENIED` (no prefix allows
            /// the URL), `BAD_ARG` (not such a URL), `FAILED` (the mod holds [`http::MAX_REQUESTS`] requests, the host
            /// has no HTTP client; the log says why).
            [Http] fn http_get(url_ptr: i32, url_len: i32) -> i32;
            /// (1.7) Where request `handle` stands: `AGAIN` while it runs, the response's HTTP status (100..=599) once
            /// the whole body is there, `FAILED` when it failed (no connection, TLS, over [`http::MAX_BODY`] bytes, over
            /// [`http::TIMEOUT_MS`]; the log says why, once), `NOT_FOUND` for a handle the mod does not hold.
            [Http] fn http_status(handle: i32) -> i32;
            /// (1.7) Copies `min(size, buf_len)` bytes of request `handle`'s response body into the buffer and
            /// returns the body's size; `AGAIN` while it runs, `FAILED`, `NOT_FOUND` as `http_status`.
            [Http] fn http_body(handle: i32, buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.7) Forgets request `handle` (a running one is abandoned; the host drops every request of a mod it
            /// disables). Returns 0 or `NOT_FOUND`.
            [Http] fn http_close(handle: i32) -> i32;
            /// (1.8) Creates a pickup of `model` and pickup type `kind` (`ePickupType`, [`pickup::MAX_TYPE`] at most)
            /// at (`x`, `y`, `z`) through `CREATE_PICKUP`, the model loaded first. Returns its handle (the game's
            /// pickup handle). The mod owns it: it is removed when the mod is disabled. `WRONG_PHASE` inside a hook.
            [Entities] fn create_pickup(model: i32, kind: i32, x: f32, y: f32, z: f32) -> i32;
            /// (1.8) Removes pickup `handle` (`REMOVE_PICKUP`), which the mod created; `DENIED` for any other.
            [Entities] fn destroy_pickup(handle: i32) -> i32;
            /// (1.8) 1 when the game has collected pickup `handle` (`HAS_PICKUP_BEEN_COLLECTED`), else 0;
            /// `NOT_FOUND` for a handle that names no pickup.
            [State] fn pickup_collected(handle: i32) -> i32;
            /// (1.8) 1 while the local player stands where the game's pickup update picks pickup `handle` up (on
            /// foot, alive, within the pickup's collection range, the pickup shown), else 0; `NOT_FOUND` for a handle
            /// that names no pickup. The test reads the state between frames; the game's own update runs in the frame.
            [State] fn pickup_touched(handle: i32) -> i32;
            /// (1.8) Creates a checkpoint marker of type `kind` (`CREATE_CHECKPOINT`, the game's race checkpoint
            /// types, [`checkpoint::MAX_TYPE`] at most) at (`x`, `y`, `z`) pointing to (`nx`, `ny`, `nz`), `size` its
            /// radius. Returns its handle; the mod owns it. `WRONG_PHASE` inside a hook.
            [Entities] fn create_checkpoint(kind: i32, x: f32, y: f32, z: f32, nx: f32, ny: f32, nz: f32, size: f32) -> i32;
            /// (1.8) Deletes checkpoint `handle` (`DELETE_CHECKPOINT`), which the mod created; `DENIED` for any other.
            [Entities] fn destroy_checkpoint(handle: i32) -> i32;
            /// (1.8) Adds a radar blip at (`x`, `y`, `z`) with radar sprite `sprite` (0: a plain marker) and
            /// [`blip`] `flags` (`ADD_SHORT_RANGE_SPRITE_BLIP_FOR_COORD` and its global / contact-point forms).
            /// Returns its handle; the mod owns it. `WRONG_PHASE` inside a hook.
            [Entities] fn create_coord_blip(x: f32, y: f32, z: f32, sprite: i32, flags: i32) -> i32;
            /// (1.8) Adds a radar blip on ped ([`blip::PED`]) or vehicle ([`blip::VEHICLE`]) `handle`
            /// (`ADD_BLIP_FOR_CHAR`, `ADD_BLIP_FOR_CAR`). Returns its handle; the mod owns it. `NOT_FOUND` for a
            /// stale entity handle.
            [Entities] fn create_entity_blip(kind: i32, handle: i32) -> i32;
            /// (1.8) Removes blip `handle` (`REMOVE_BLIP`), which the mod created; `DENIED` for any other.
            [Entities] fn destroy_blip(handle: i32) -> i32;
            /// (1.8) Sets the colour of the mod's blip `handle` (`CHANGE_BLIP_COLOUR`): the game's blip colours 0..=8,
            /// any other value an `0xRRGGBBAA` colour.
            [Entities] fn set_blip_colour(handle: i32, colour: i32) -> i32;
            /// (1.8) Sets the size of the mod's blip `handle` (`CHANGE_BLIP_SCALE`, 1..=4 in the game's scripts).
            [Entities] fn set_blip_scale(handle: i32, scale: i32) -> i32;
            /// (1.8) Where the mod's blip `handle` shows (`CHANGE_BLIP_DISPLAY`: 1 marker only, 2 blip only, 3 both).
            [Entities] fn set_blip_display(handle: i32, display: i32) -> i32;
            /// (1.8) Shows the radar area `id` (`0 .. radar::MAX_AREAS`, the mod's own numbering; drawn in id order
            /// after the game's gang areas, on the radar and the map, as `CRadar::DrawAreaOnRadar` draws) over the
            /// world rectangle (`min_x`, `min_y`) - (`max_x`, `max_y`) in `rgba` (`0xRRGGBBAA`); replaces an area of
            /// the same id (not flashing). Returns 0, `BAD_ARG`.
            [Render] fn set_radar_area(id: i32, min_x: f32, min_y: f32, max_x: f32, max_y: f32, rgba: i32) -> i32;
            /// (1.8) Makes radar area `id` alternate between its colour and `rgba` every `period_ms` (0 stops it).
            /// Returns 0, `NOT_FOUND`, `BAD_ARG`.
            [Render] fn flash_radar_area(id: i32, rgba: i32, period_ms: i32) -> i32;
            /// (1.8) Removes radar area `id`. Returns 0, `NOT_FOUND`.
            [Render] fn remove_radar_area(id: i32) -> i32;
            /// (1.8) 1 when the game defines model `model` (it has model info), else 0.
            [State] fn model_exists(model: i32) -> i32;
            /// (1.9) Writes object `handle`'s state ([`state::ObjectInfo`]) to the buffer, as `ped_info`; `NOT_FOUND`
            /// when the handle names no live object.
            [State] fn object_info(handle: i32, buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.9) Writes object `handle`'s transform ([`state::MatrixInfo`]) to the buffer, as `object_info`.
            [State] fn object_matrix(handle: i32, buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.9) The objects in the object pool, as `ped_list`.
            [State] fn object_list(buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.9) Writes the world transform ([`state::MatrixInfo`]) of node `node` (`ePedNode`, 1..=18: 1 upper
            /// torso, 2 head, 3 left arm, 4 right arm, 5 left hand, 6 right hand, 7 left thigh, 8 right thigh, 9 left
            /// foot, 10 right foot, 11 right calf, 12 left calf, 13 left forearm, 14 right forearm, 15 left clavicle,
            /// 16 right clavicle, 17 neck, 18 jaw) of ped `handle`'s skeleton, as the game's animation left it, to the
            /// buffer. `NOT_FOUND` for a stale handle, `BAD_ARG` for another node, `FAILED` while the ped has no
            /// skeleton (not streamed in).
            [State] fn ped_bone_matrix(handle: i32, node: i32, buf_ptr: i32, buf_len: i32) -> i32;
            /// (1.9) What model `model` is ([`model`]: its kind, plus [`model::LOADED`] while it is loaded); `BAD_ARG`
            /// outside `0 .. model::COUNT`.
            [State] fn model_info(model: i32) -> i32;
            /// (1.9) Requests model `model` from the streamer (`REQUEST_MODEL`), and with `now` 1 loads every requested
            /// model at once (`LOAD_ALL_MODELS_NOW`, a stall). Returns 1 when it is loaded, else 0 (it loads in the
            /// coming frames). The host releases the mod's requests when it disables the mod. `BAD_ARG` for a model
            /// without a model info, `WRONG_PHASE` inside a hook.
            [Entities] fn model_request(model: i32, now: i32) -> i32;
            /// (1.9) Marks model `model` as no longer needed (`MARK_MODEL_AS_NO_LONGER_NEEDED`): the streamer may unload
            /// it once no entity uses it.
            [Entities] fn model_release(model: i32) -> i32;
            /// (1.9) Sets the draw distance of atomic model `model` (the distance its instances are drawn up to, as its
            /// definition file sets it); the host restores the old one when it disables the mod. `BAD_ARG` for a
            /// model that is not an atomic one or a distance that is not positive.
            [Write] fn set_model_draw_distance(model: i32, distance: f32) -> i32;
            /// (1.9) Moves object `handle` to (`x`, `y`, `z`) keeping its rotation, as the game teleports an object
            /// (`CObject::Teleport`: out of the world, the position, its RenderWare frame, back in).
            [Write] fn set_object_position(handle: i32, x: f32, y: f32, z: f32) -> i32;
            /// (1.9) Gives object `handle` the transform of the 12 little-endian `f32` at `matrix_ptr` (right, forward,
            /// up, position; rows as they are, so scaled rows scale it), as `set_object_position` moves it. Not inside
            /// a hook (`WRONG_PHASE`).
            [Write] fn set_object_matrix(handle: i32, matrix_ptr: i32) -> i32;
            /// (1.9) Turns object `handle` to the Euler angles (`x`, `y`, `z`) in degrees (`SET_OBJECT_ROTATION`).
            [Write] fn set_object_rotation(handle: i32, x: f32, y: f32, z: f32) -> i32;
            /// (1.9) Draws object `handle` scaled by `scale` (`SET_OBJECT_SCALE`).
            [Write] fn set_object_scale(handle: i32, scale: f32) -> i32;
            /// (1.9) Shows (1) or hides (0) object `handle` (`SET_OBJECT_VISIBLE`).
            [Write] fn set_object_visible(handle: i32, on: i32) -> i32;
            /// (1.9) Switches object `handle`'s collision on (1) or off (0) (`SET_OBJECT_COLLISION`).
            [Write] fn set_object_collision(handle: i32, on: i32) -> i32;
            /// (1.9) Writes object `handle`'s move speed and turn speed (6 little-endian `f32` at `speeds_ptr`, units
            /// per 1/50 s): what peds standing on it move with. It does not move the object. Not inside a hook.
            [Write] fn set_object_speed(handle: i32, speeds_ptr: i32) -> i32;
            /// (1.9) Attaches object `handle` to entity `target` of kind `kind` ([`attach`]) with the offset and the
            /// rotation in degrees of the 6 little-endian `f32` at `offsets_ptr` (`ATTACH_OBJECT_TO_CHAR` / `_CAR` /
            /// `_OBJECT`): the game then places it every frame. `NOT_FOUND` for a stale handle. Not inside a hook.
            [Write] fn attach_object(handle: i32, kind: i32, target: i32, offsets_ptr: i32) -> i32;
            /// (1.9) Detaches object `handle` where it is, with collision (`DETACH_OBJECT`).
            [Write] fn detach_object(handle: i32) -> i32;
            /// (1.9) Draws material `slot` (0 .. [`material::SLOTS`], the materials of the object's model in geometry
            /// order) of object `handle` with the texture `name` of texture dictionary `dict` (UTF-8, `(ptr, len)`
            /// each, at most [`material::MAX_NAME`] bytes) that model `model` loads (the model is loaded first;
            /// `model` -1: keep the material's texture) and modulated by colour `rgba` (`0xRRGGBBAA`; 0: the
            /// material's own colour). `model` -1 with `rgba` 0 gives the slot back its own material. `NOT_FOUND`: a
            /// stale handle, or no such dictionary or texture. Not inside a hook.
            [Write] fn set_object_material(handle: i32, slot: i32, model: i32, dict_ptr: i32, dict_len: i32, name_ptr: i32, name_len: i32, rgba: i32) -> i32;
            /// (1.9) Removes the world's buildings (and the dummies and objects of the map) of model `model` (-1: any)
            /// whose position is within `radius` of (`x`, `y`, `z`) for the rest of the session: they move into an
            /// interior area and below the world, the occluders within 500 units of the point stop hiding anything,
            /// and the ones the world streams in later are removed as they load. Returns the entities removed now. Not
            /// inside a hook.
            [Write] fn remove_buildings(model: i32, x: f32, y: f32, z: f32, radius: f32) -> i32;
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
            /// (1.5) A platform input event ([`input`]: its kind and values), before the game sees it, for mods
            /// with `input = "capture"`, in load order. Returns 1 to consume it (the game and later mods do not
            /// see it), else 0.
            [Optional] fn sa_input(kind: i32, a: i32, b: i32) -> i32;
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
/// The name of the `sa_input` export (1.5).
pub const EXPORT_INPUT: &str = "sa_input";

/// The game's own features a mod may switch off (`set_game_option`, API 1.5): the numbers are
/// [`sa_types::GameOption::ALL`]'s order.
pub mod option {
    use sa_types::GameOption;

    /// The option numbered `n`.
    pub fn from_number(n: i32) -> Option<GameOption> {
        usize::try_from(n).ok().and_then(|i| GameOption::ALL.get(i).copied())
    }

    /// The number of `o`.
    pub fn number(o: GameOption) -> i32 {
        GameOption::ALL.iter().position(|x| *x == o).map_or(-1, |i| i as i32)
    }
}

/// Platform input events (`sa_input`, API 1.5). Keys are Windows virtual-key codes on every platform, positions
/// pixels of the game's drawable from the top left.
pub mod input {
    /// A key went down: `a` the key, `b` 1 for an auto-repeat.
    pub const KEY_DOWN: i32 = 0;
    /// A key went up: `a` the key.
    pub const KEY_UP: i32 = 1;
    /// A character was typed: `a` its Unicode scalar value.
    pub const CHAR: i32 = 2;
    /// The mouse moved: `a`, `b` its position.
    pub const MOUSE_MOVE: i32 = 3;
    /// A mouse button: `a` the button (0 left, 1 right, 2 middle), `b` 1 down, 0 up.
    pub const MOUSE_BUTTON: i32 = 4;
    /// The wheel turned: `a` the distance in 1/120 notches, positive away from the user.
    pub const WHEEL: i32 = 5;
    /// The window gained (`a` 1) or lost (0) the focus.
    pub const FOCUS: i32 = 6;
}

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
        assert!(script::COMMANDS.iter().all(|c| c.2.len() <= script::MAX_ARGS && c.2.chars().all(|k| "ifpvPos".contains(k))));
    }

    #[test]
    fn level_seven() {
        assert_eq!(IMPORTS[import_index("http_get").unwrap()].permission, Permission::Http);
        assert!(http::valid_prefix("https://api.open.mp/") && http::valid_prefix("http://127.0.0.1:8080/list/"));
        for p in ["https://api.open.mp", "ftp://a/", "https:///x", "https://a b/"] {
            assert!(!http::valid_prefix(p), "{p:?}");
        }
        let list = vec!["https://api.open.mp/".to_string()];
        assert!(http::allows(&list, "https://api.open.mp/servers/"));
        assert!(!http::allows(&list, "https://api.open.mp.evil.org/servers/"));
        assert!(!http::allows(&list, "https://api.open.mp/a b"));
        assert!(!http::allows(&list, "https://api.open.mp/../x") && !http::allows(&list, "https://api.open.mp/%2E%2e/x"));
    }

    #[test]
    fn markers_and_radar_areas() {
        assert_eq!(IMPORTS[import_index("create_pickup").unwrap()].permission, Permission::Entities);
        assert_eq!(IMPORTS[import_index("pickup_touched").unwrap()].permission, Permission::State);
        assert_eq!(IMPORTS[import_index("set_radar_area").unwrap()].permission, Permission::Render);
        assert_eq!(script::params(0x00FE), Some("pffffffi"));
    }

    #[test]
    fn objects_and_models() {
        assert_eq!(IMPORTS[import_index("set_object_matrix").unwrap()].permission, Permission::Write);
        assert_eq!(IMPORTS[import_index("object_info").unwrap()].permission, Permission::State);
        assert_eq!(IMPORTS[import_index("model_request").unwrap()].permission, Permission::Entities);
        for k in [
            sa_types::ModelKind::None,
            sa_types::ModelKind::Atomic,
            sa_types::ModelKind::Clump,
            sa_types::ModelKind::Ped,
            sa_types::ModelKind::Vehicle,
            sa_types::ModelKind::Weapon,
        ] {
            assert_eq!(model::kind(model::kind_number(k) | model::LOADED), k);
        }
    }

    #[test]
    fn level_four() {
        assert_eq!(IMPORTS[import_index("net_open").unwrap()].permission, Permission::Net);
        assert_eq!(IMPORTS[import_index("storage_list").unwrap()].permission, Permission::Storage);
        assert_eq!(IMPORTS[import_index("clock").unwrap()].permission, Permission::None);
        assert!(storage::valid_key("server.cfg") && storage::valid_key("a-b_c.1"));
        for k in ["", ".x", "a/b", "a b", &"k".repeat(65)] {
            assert!(!storage::valid_key(k), "{k:?}");
        }
    }
}
