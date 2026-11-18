# Mod API reference (API 1.2)

The interface is defined once, in `crates/sa-mod-api/src/lib.rs` (`for_each_import!`, `for_each_export!`); this page
explains it. The Rust SDK (`crates/sa-mod`) wraps every function; other languages import the functions from module
`"sa"` (see "Other languages" in `getting-started.md`). Every host function returns an `i32`: a result `>= 0` or a
status (`-1` DENIED: the manifest does not grant it, `-2` BAD_ARG, `-3` NOT_FOUND).

A mod built for an older minor version (`api_version = "1.0"` / `"1.1"`) runs unchanged: entry points added later are
optional and the host skips them.

## Permissions (`mod.toml`, `[permissions]`)

| key | values | grants |
|---|---|---|
| `render` | `true` / `false` | overlay drawing, display modes, the video mode |
| `files` | `"none"` / `"own"` | the mod's own files (1.1) |
| `state` | `"none"` / `"read"` | game state queries, game events except frames and keys (1.2) |
| `input` | `"none"` / `"read"` | `key_down`, key events (1.2) |
| `hooks` | a list of function names | before / after hooks on these functions (1.2) |

`state = "write"` and `input = "capture"` belong to later levels: 1.2 grants only `read` for them (with a warning in
the log). A function without its permission returns DENIED; the mod keeps running.

## Entry points (exports)

| export | SDK | since | when |
|---|---|---|---|
| `sa_api_version() -> i32` | `entry!` | 1.0 | at load (`major << 16 \| minor`) |
| `sa_init() -> i32` | `Mod::init` | 1.0 | once after loading; subscribe to events and hooks here |
| `sa_overlay(frame) -> i32` | `Mod::overlay` | 1.0 | every presented frame, after the game's drawing |
| `sa_settings() -> i32` | `Mod::settings` | 1.1 | once in the boot, before the D3D device exists |
| `sa_resize(w, h) -> i32` | `Mod::resize` | 1.1 | the window / canvas changed size |
| `sa_event(kind, a, b) -> i32` | `Mod::event` | 1.2 | a subscribed game event |
| `sa_hook(hook, phase) -> i32` | `Mod::hook` | 1.2 | a hooked game function runs |

## Level 1: overlay, display, files (1.0, 1.1)

`log`, `screen_width`, `screen_height`, `overlay_rect`, `overlay_text`, `display_mode_count`, `display_mode_width`,
`display_mode_height`, `request_video_mode`, `file_read`, `file_write`: see `getting-started.md` and the doc
comments in `sa-mod-api`.

## Game state (1.2, `state = "read"`)

Every query reads the game as it is at the moment of the call (in `sa_hook`: in the middle of the game's frame).
Records are written into a buffer of the mod's memory as little-endian 32-bit words in field order; the SDK decodes
them (`sa_mod::state::*`). Field types are the SA:MP game adapter's own (`crates/sa-samp-api`), and each record is
filled from the adapter's snapshot of the same thing: the mod API and samp-rs read the game the same way.

| function | SDK | record / result |
|---|---|---|
| `game_state()` | `state::game_state()` | `gGameState` (7 frontend idle, 8 loading, 9 playing) |
| `player_info(buf, len)` | `state::player()` | `PlayerInfo`: ped handle (or none), money, wanted level, `PedInfo` |
| `ped_info(handle, buf, len)` | `state::ped(p)` | `PedInfo`: model, position, move speed, heading, health, armour, ped state, weapon, vehicle, area, in world; NOT_FOUND for a handle that names no live ped |
| `vehicle_info(handle, buf, len)` | `state::vehicle(v)` | `VehicleInfo`: model, position, forward, move speed, health, driver, colours, siren, area |
| `world_info(buf, len)` | `state::world()` | `WorldInfo`: game state, game time, time step, clock, weather (old, new, forced), area, gravity |
| `camera_info(buf, len)` | `state::camera()` | `CameraInfo`: position, front, up, field of view, mode, aspect ratio |
| `ped_list(buf, len)` | `state::peds()` | the ped pool's handles (as many as fit); returns how many there are |
| `vehicle_list(buf, len)` | `state::vehicles()` | the vehicle pool's handles, as `ped_list` |
| `key_down(vk)` (`input = "read"`) | `state::key_down(vk)` | 1 while the key (Windows virtual-key code) is held in the game's keyboard state |

Handles are the game's pool references (`slot << 8 | reuse counter`, the script handles): a handle stays valid while
the slot holds the same entity; afterwards queries return NOT_FOUND. Mods never see game addresses.

## Events (1.2)

`events_subscribe(mask)` (SDK `events::subscribe(&[kinds])`) subscribes to the kinds in `mask` (bit `1 << kind`) and
returns the mask granted. The host derives the events at the start of every main-loop iteration from the game state
as it was at the previous one (the game is never interrupted for them), in this order:

| kind | `a`, `b` | permission |
|---|---|---|
| 0 `FRAME_BEGIN` | iteration counter | none |
| 1 `FRAME_END` (after the overlay phase) | iteration counter | none |
| 2 `GAME_STATE` | new, old `gGameState` | `state` |
| 3 `PLAYER_SPAWN` | the player's ped (it appeared or came back to life) | `state` |
| 4 `PLAYER_DEATH` | the player's ped (health 0, or dying / dead) | `state` |
| 5 `VEHICLE_ENTER` | vehicle, the player's ped | `state` |
| 6 `VEHICLE_EXIT` | vehicle (may no longer exist), the player's ped | `state` |
| 7 `AREA` | new, old area / interior | `state` |
| 8 `KEY_DOWN` / 9 `KEY_UP` | virtual-key code | `input` |

A main-loop iteration is not always a game step: when the frame limiter holds the game back, an iteration runs no
game logic (frame events still come).

## Hooks (1.2, `hooks = [...]`)

`hook_subscribe(name, phases)` (SDK `hooks::subscribe(name, before, after)`) hooks the game function `name`, which the
manifest's `hooks` list must name, and returns the hook's id. Names are the hook registry's (the catalogue's), e.g.
`CPed::ProcessControl`, `CPed::Render`, `CWorld::FindPlayerHeading`; NOT_FOUND when the registry has no such
function. Then, for every call of it the game dispatches through the registry, the host calls
`sa_hook(id, phase)` (phase 1 before the function runs, 2 after it returned); inside, `hook_arg_count()` and
`hook_value(index, out)` (`index` -1: the result, after the call) give the values (SDK `hooks::Call::arg / result`):

| kind | value |
|---|---|
| 0 `Int` | an integer argument |
| 1 `Float` | a float argument |
| 2 `Ped`, 3 `Vehicle`, 4 `Object` | a pointer to a pooled entity, as its handle |
| 5 `Pointer` | any other pointer (hidden: mods never see addresses) |

Hooks are read-only at this level: a hook observes, it cannot change an argument, the result, or skip the call. What
is observed: calls that go through the engine's address registry (`orig!` dispatch): virtual calls (`ProcessControl`,
`Render`, ...), calls between crates and calls through function pointers. A call a port makes directly to a ported
function in its own crate is not seen. The mod's summary line at exit counts its hook calls; a hook that never fires
names such a function. Types come from the declaration the caller used: an argument a port declares as an integer is
passed as `Int` unless it is the address of a pooled entity.

Each mod receives at most 4096 hook calls per main-loop iteration (`hook::MAX_CALLS_PER_FRAME`); further calls in that
iteration are dropped and counted. Every hook call is a call into the mod (natively about 15 µs on the i686 build's
interpreter): hook functions that run a few times per frame, not per entity per pass.

## Cost and determinism

With the `mods` feature and no mod loaded, nothing is hooked: the registry dispatch checks one flag of the entry it
already looked up. Without the feature the check is not compiled in. Level-2 mods only read: the game runs the same
with and without them (the crossing run's report is identical apart from the mods' own lines and frame pacing).
