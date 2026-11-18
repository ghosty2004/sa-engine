# Mod API reference (API 1.3)

The interface is defined once, in `crates/sa-mod-api/src/lib.rs` (`for_each_import!`, `for_each_export!`); this page
explains it. The Rust SDK (`crates/sa-mod`) wraps every function; other languages import the functions from module
`"sa"` (see "Other languages" in `getting-started.md`). Every host function returns an `i32`: a result `>= 0` or a
status (`-1` DENIED: the manifest does not grant it, `-2` BAD_ARG, `-3` NOT_FOUND, `-4` WRONG_PHASE: not allowed now, e.g.
inside a hook, `-5` FAILED: the game could not do it, the log says why).

A mod built for an older minor version (`api_version = "1.0"` / `"1.1"` / `"1.2"`), or rebuilt with a newer SDK while
its manifest keeps the older version, runs unchanged: entry points added later are
optional and the host skips them.

## Permissions (`mod.toml`, `[permissions]`)

| key | values | grants |
|---|---|---|
| `render` | `true` / `false` | overlay drawing, display modes, the video mode |
| `files` | `"none"` / `"own"` | the mod's own files (1.1) |
| `state` | `"none"` / `"read"` / `"write"` | game state queries, game events except frames and keys (1.2); `"write"`: also changes of the state (1.3) |
| `input` | `"none"` / `"read"` | `key_down`, key events (1.2) |
| `hooks` | a list of function names | before / after hooks on these functions (1.2); with `state = "write"`, decisions on the controllable ones (1.3) |
| `entities` | `true` / `false` | create and delete peds, vehicles, objects (1.3) |
| `scripts` | `true` / `false` | the allowed script commands (1.3) |
| `audio` | `true` / `false` | game sounds (1.3) |

`input = "capture"`, `ui` and `net` belong to later levels: 1.3 does not grant them (with a warning in the log). A
function without its permission returns DENIED; the mod keeps running.

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
them (`sa_mod::state::*`). Field types are the game's value types (`crates/sa-types`), and each record is filled from
the engine's record of the same thing (`crates/sa-access`, the engine's access layer, which the SA:MP adapter uses
too).

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

A hook observes; on the functions of `hook::CONTROLLABLE` a mod with `state = "write"` may also decide (1.3, below).
What is observed: calls that go through the engine's address registry (`orig!` dispatch): virtual calls
(`ProcessControl`, `Render`, ...), calls between crates and calls through function pointers, and (1.3) the calls a
port makes to a ported function of its own crate through its `orig!` binding (`=> crate::path`), which take the
observed path while any function is watched. A Rust call straight to a function (not through a binding) is not
seen; the mod's summary line at exit counts its hook calls, and a hook that never fires names such a function. Types come from the declaration the caller used: an argument a port declares as an integer is
passed as `Int` unless it is the address of a pooled entity.

Each mod receives at most 4096 hook calls per main-loop iteration (`hook::MAX_CALLS_PER_FRAME`); further calls in that
iteration are dropped and counted. Every hook call is a call into the mod (natively about 15 µs on the i686 build's
interpreter): hook functions that run a few times per frame, not per entity per pass.

## Cost and determinism

With the `mods` feature and no mod loaded, nothing is hooked: the registry dispatch checks one flag of the entry it
already looked up. Without the feature the check is not compiled in. Level-2 mods only read: the game runs the same
with and without them (the crossing run's report is identical apart from the mods' own lines and frame pacing).

## Level 3: changes, entities, script commands, sounds (1.3)

### When a change happens

The game is single-threaded and a mod is called between its frames (events at the start of a main-loop iteration,
the overlay phase after the game drew its frame) or in the middle of one (`sa_hook`). A change asked for between
frames happens at once, through the game's own code; inside a hook it is queued and applied at the start of the next
main-loop iteration (at most 256 per mod and frame; the call returns OK). Creating or deleting entities and script
commands are refused inside a hook (WRONG_PHASE). Every change goes through `sa-access`, the engine's access layer:
the ported script command handler of the change where the game has one (positions, headings, weapons, the wanted
level, the clock, entity creation with model requests, pool allocation and `CWorld::Add`, deletion with
`CWorld::Remove`), a plain field write where the game only stores a value (health, armour, money).

### Changes (`state = "write"`)

| function | SDK (`sa_mod::world`) | what |
|---|---|---|
| `set_ped_position(h, x, y, z)` | `set_ped_position` | `SET_CHAR_COORDINATES` (out of a vehicle, onto the ground) |
| `set_ped_heading(h, deg)` | `set_ped_heading` | `SET_CHAR_HEADING` |
| `set_ped_health(h, v)` / `set_ped_armour(h, v)` | `set_ped_health` / `set_ped_armour` | the ped's fields |
| `give_weapon(h, weapon, ammo)` | `give_weapon` | the weapon's model loaded, then `GIVE_WEAPON_TO_CHAR` |
| `set_money(v)` | `set_money` | the player's money and the HUD's displayed money |
| `set_wanted_level(0..6)` | `set_wanted_level` | `ALTER_WANTED_LEVEL` |
| `set_clock(h, m)` / `set_weather(w)` | `set_clock` / `set_weather` | `SET_TIME_OF_DAY` / the weather types |
| `set_vehicle_position`, `set_vehicle_heading`, `set_vehicle_health`, `set_vehicle_colours` | same names | `SET_CAR_COORDINATES`, `SET_CAR_HEADING`, the health field, `CHANGE_CAR_COLOUR` |

A stale handle is NOT_FOUND, a value out of range BAD_ARG.

### Entities (`entities = true`)

`create_vehicle(model, x, y, z, deg)`, `create_ped(model, x, y, z)` (a civilian), `create_object(model, x, y, z)`
(SDK `sa_mod::entities`) load the model (`REQUEST_MODEL` + `LOAD_ALL_MODELS_NOW`), create the entity with the game's
command (`CREATE_CAR`, `CREATE_CHAR`, `CREATE_OBJECT`) and return its handle. The mod owns what it created:
`destroy_vehicle` / `destroy_ped` / `destroy_object` delete only those (DENIED for any other entity), and when the
mod is disabled the host deletes them at the next frame boundary.

### Script commands (`scripts = true`)

`script_command(opcode, args_ptr, args_len, out_ptr)` (SDK `sa_mod::script::run`) runs one command of the game's
script language through its ported handler, on a private script thread of the host. Only the commands of
`sa_mod_api::script::COMMANDS` are allowed (gameplay commands on the player, the world, the camera, the HUD or an
entity named by its handle, e.g. `ADD_SCORE`, `SET_PLAYER_CONTROL`, `DISPLAY_HUD`, `SET_CHAR_PROOFS`), each with its
parameters, one letter each: `i` integer, `f` float, `p` ped, `v` vehicle, `P` the player (0), `o` output. The
arguments are one word per parameter; the host checks every handle before the command runs (the game's handlers trust
handles), runs it and writes the outputs, one word each, to `out_ptr`. It returns the condition result (0 / 1).

### Hook decisions (`hooks` + `state = "write"`)

On the functions of `sa_mod_api::hook::CONTROLLABLE` (the weapon and damage functions, the ped / player / car
`ProcessControl`, the enter-car task's first sub-task, ped and object rendering, camera shake, the crosshair), inside
`sa_hook`:

| function | SDK (`hooks::Call`) | phase | what |
|---|---|---|---|
| `hook_set_arg(index, word)` | `set_arg(i, Value)` | before | the argument becomes `word`, read as its kind (an integer, a float's bits, the handle of a live entity of the same kind); the game gets the entity's address |
| `hook_skip(result)` | `skip(Value)` | before | the call does not run; its caller gets `result` (read as the result's type; ignored for a function without one) |
| `hook_set_result(result)` | `set_result(Value)` | after | the caller gets `result` instead |

When several mods decide on the same call, the later one in the load order wins. The engine applies a decision with
the call's declared types (`sa_core::orig::observe`): what a skipped function would have done does not happen, which
is why only functions whose callers cope with that are controllable.

### Sounds (`audio = true`)

`play_frontend_sound(event, speed)` (SDK `sa_mod::audio::frontend`) reports front-end sound `event` (`AE_FRONTEND_*`,
1..=46) to the game's audio engine; `play_sound(sound, x, y, z)` (`sa_mod::audio::at`) is `ADD_ONE_OFF_SOUND` (while
its handler's audio code is not ported, the sounds whose whole effect is a front-end sound are played as that; the
others are FAILED).

### Determinism

Level-3 mods change the game by design; with no mod loaded the `mods` build runs the game as the default build does
(verify-all IDENTICAL), and the default build has none of this code.
