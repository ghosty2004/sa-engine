# Mod API reference (API 1.8)

The interface is defined once, in `crates/sa-mod-api/src/lib.rs` (`for_each_import!`, `for_each_export!`); this page
explains it. The Rust SDK (`crates/sa-mod`) wraps every function; other languages import the functions from module
`"sa"` (see "Other languages" in `getting-started.md`). Every host function returns an `i32`: a result `>= 0` or a
status (`-1` DENIED: the manifest does not grant it, `-2` BAD_ARG, `-3` NOT_FOUND, `-4` WRONG_PHASE: not allowed now, e.g.
inside a hook, `-5` FAILED: the game could not do it, the log says why, `-6` AGAIN: nothing now, try again later,
1.4).

A mod built for an older minor version (`api_version = "1.0"` ... `"1.6"`), or rebuilt with a newer SDK while
its manifest keeps the older version, runs unchanged: entry points added later are
optional and the host skips them.

## Permissions (`mod.toml`, `[permissions]`)

| key | values | grants |
|---|---|---|
| `render` | `true` / `false` | overlay drawing (rectangles, text, images, 1.6: the game's font and textures), display modes, the video mode |
| `files` | `"none"` / `"own"` | the mod's own files (1.1) |
| `state` | `"none"` / `"read"` / `"write"` | game state queries (1.6: `world_to_screen`), game events except frames and keys (1.2); `"write"`: also changes of the state (1.3) |
| `input` | `"none"` / `"read"` / `"capture"` | `key_down`, key events (1.2), `pad_state` (1.5); `"capture"`: also the input events before the game and `input_capture` (1.5) |
| `hooks` | a list of function names | before / after hooks on these functions (1.2); with `state = "write"`, decisions on the controllable ones (1.3) |
| `entities` | `true` / `false` | create and delete peds, vehicles, objects (1.3), pickups, checkpoint markers, blips (1.8) |
| `scripts` | `true` / `false` | the allowed script commands (1.3) |
| `audio` | `true` / `false` | game sounds (1.3) |
| `net` | a list of `"udp:HOST:PORT"` rules, `*` for any host or port | UDP sockets to the targets a rule allows (1.4) |
| `storage` | `true` / `false` | the mod's persistent key / value storage (1.4) |
| `http` | a list of URL prefixes, `"https://HOST/PATH/"` | GET requests to URLs under these prefixes (1.7) |

`ui` belongs to a later level. A function without its permission returns DENIED; the mod keeps running.

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
| `sa_input(kind, a, b) -> i32` | `Mod::input` | 1.5 | a platform input event, before the game (`input = "capture"`); 1 consumes it |

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

## Level 4: sockets, storage, the clock (1.4)

### The clock

`clock(out_ptr)` (SDK `sa_mod::time::now()`, no permission) writes two little-endian `u64`: the microseconds since the
host started (monotonic) and the wall clock in microseconds since 1970-01-01 UTC. A mod has no other clock (wasm32
has none of its own).

### Sockets (`net = ["udp:HOST:PORT", ...]`)

| function | SDK (`sa_mod::net::Socket`) | what |
|---|---|---|
| `net_open(target_ptr, target_len)` | `Socket::open(target)` | a connected, non-blocking UDP socket to `HOST:PORT` (`[IPV6]:PORT`); returns its handle |
| `net_state(h)` | `state()` | 0 connecting, 1 open, 2 closed |
| `net_send(h, ptr, len)` | `send(datagram)` | one datagram (at most 65507 bytes); its length, AGAIN when the frame's budget is used up |
| `net_recv(h, ptr, len)` | `recv(buf)` | one received datagram (truncated to the buffer); its length, AGAIN when none is queued |
| `net_close(h)` | drop | closes it |

The host compares the target text the mod passes with the manifest's rules before anything is resolved: `udp:` then
a host (a name or an address, compared without case, `[...]` around an IPv6 address) or `*`, then a port or `*`.
`net = ["udp:127.0.0.1:7790"]` allows one local server, `["udp:*:7777"]` any server on port 7777 (the log warns about
rules for any host). A target no rule allows is DENIED; a target that does not resolve, or an eighth open socket
(`net::MAX_SOCKETS`), is FAILED with a log line.

Natively a socket is a real UDP socket (sa-net's `UdpTransport`: bound to an ephemeral port, connected). In the
browser it is a WebSocket to the page's UDP proxy (`sa-udp-proxy`, the page's `?proxy=ws://...`, default
`ws://127.0.0.1:7781`), which resolves the target and relays one datagram per binary frame; it starts `connecting`
and datagrams sent meanwhile are queued (64 at most). The proxy has its own allow list of targets. The page serves the WebSocket only
between the game's main-loop iterations: a socket opened in `sa_init` waits through the boot, and the proxy closes a
connection that sends no target within 10 s, so open sockets from a frame phase, and open a new one when one is
`closed` (`examples/mods/net-ping` does both).

Nothing blocks: poll the sockets once per frame (in `sa_overlay`, or on the `FRAME_BEGIN` event). Each mod may send
1024 datagrams and 1 MiB per main-loop iteration over all its sockets (`net::MAX_SENDS_PER_FRAME`,
`net::MAX_SEND_BYTES_PER_FRAME`); further sends return AGAIN until the next iteration. Received datagrams wait in
the socket (natively the system's buffer, in the browser up to 4096). When the host disables a mod it closes its
sockets.

### Storage (`storage = true`)

| function | SDK (`sa_mod::storage`) | what |
|---|---|---|
| `storage_read(key_ptr, key_len, buf_ptr, buf_len)` | `read(key)` | the value's size (copies what fits), NOT_FOUND |
| `storage_write(key_ptr, key_len, data_ptr, data_len)` | `write(key, data)` | replaces the value (at most 1 MiB) |
| `storage_delete(key_ptr, key_len)` | `delete(key)` | OK, NOT_FOUND |
| `storage_list(buf_ptr, buf_len)` | `keys()` | the keys, sorted, one per line; returns the list's length |

One area per mod id, kept across sessions: natively the folder `sa-engine/mods/ID/` under the user's data directory
(`$XDG_DATA_HOME`, else `~/.local/share`, else `%APPDATA%`; `$SA_MOD_DATA` replaces the whole root), one file per key,
written through a temporary file; in the browser the page's IndexedDB database `sa-mod-storage` (keys `ID/KEY`),
read into memory before the game starts and written back in the background. Keys are 1..=64 bytes of
`A-Z a-z 0-9 . _ -`, not starting with `.`. A mod keeps at most 1024 keys and 16 MiB (`storage::MAX_KEYS`,
`storage::MAX_TOTAL`): a write beyond is FAILED. `files = "own"` stays what it was: the files that ship in the mod's
folder (natively writable, in the browser kept for the session).

### Determinism

Sockets and storage do not touch the game: with no mod loaded (or mods that do not ask for them) nothing changes.

## Level 5: the player's view, input capture, game options (1.5)

What a multiplayer client needs to drive the local player and show its own UI, expressed in the game's terms.

### Reads

| function | permission | SDK | what |
|---|---|---|---|
| `menu_active()` | `state` | `state::menu_active()` | 1 while the pause menu is up |
| `pad_state(buf, len)` | `input` | `state::pad()` | the local player's controller state this frame (`PadInfo`: sticks -128..=128, buttons 0 / 255, the walk key) |
| `ped_matrix(h, buf, len)` | `state` | `state::ped_matrix(p)` | a ped's transform (`MatrixInfo`: right, forward, up, position) |
| `vehicle_matrix(h, buf, len)` | `state` | `state::vehicle_matrix(v)` | a vehicle's transform |

### Changes (`state = "write"`)

| function | SDK (`sa_mod::game`) | what |
|---|---|---|
| `set_ped_model(h, model)` | `set_ped_model` | the player ped `h` gets ped model `model` (loaded first; `SET_PLAYER_MODEL`); FAILED for another ped or a model that is not a ped's |
| `set_hud_visible(on)` | `set_hud_visible` | the HUD and the radar on or off (the menu's HUD flag and the radar flag) |
| `set_game_option(option, on)` | `set_option` | one of the game's own features ([`GameOption`], numbered in declaration order): typed cheats, the clock, ambient peds, random traffic, car generators, ambient planes, cutscenes, the replay buffer, plants, the sun and moon, the idle camera, help boxes, zone names, (1.6) the start screens (logos, intro movie, main menu; read when the game starts, so set it in `sa_init` or `sa_settings`). The ports skip exactly the original call that feature names; the host turns every option a mod switched off back on when it disables the mod |
| `big_message(ptr, len, time_ms, style)` | `big_message` | the game's big message (`CMessages::AddBigMessage`, styles 0..=6); not inside a hook |

`set_ped_model` and `set_hud_visible` are changes like those of level 3 (queued inside a hook). Options switch at once.

### Script commands

Text arguments (`s`): the argument word is the address of a NUL-terminated string in the mod's memory (at most 63
bytes); the SDK's `script::Arg::Text(&[u8])` does that. 1.5 adds the commands a client needs for the player, tasks,
animations, streaming and the world: `CLEAR_PRINTS`, `ADD_HOSPITAL_RESTART`, `SET_CURRENT_CHAR_WEAPON`,
`SWITCH_ROADS_OFF`, `REQUEST_MODEL`, `WARP_CHAR_FROM_CAR_TO_COORD`, `WARP_CHAR_INTO_CAR`, `LOAD_ALL_MODELS_NOW`,
`WARP_CHAR_INTO_CAR_AS_PASSENGER`, `SET_CHAR_SUFFERS_CRITICAL_HITS`, `REMOVE_ALL_CHAR_WEAPONS`,
`FREEZE_CHAR_POSITION`, `REQUEST_COLLISION`, `REQUEST_ANIMATION`, `HAS_ANIMATION_LOADED`, `SET_PLAYER_DRUNKENNESS`,
`TASK_HANDS_UP`, `TASK_ENTER_CAR_AS_PASSENGER`, `TASK_LEAVE_CAR`, `SWITCH_RANDOM_TRAINS`,
`TASK_PLAY_ANIM_NON_INTERRUPTABLE`, `SET_CHAR_AREA_VISIBLE`, `SET_CHAR_DROPS_WEAPONS_WHEN_DEAD`, `DISPLAY_ZONE_NAMES`
(the list with signatures: `sa_mod_api::script::COMMANDS`).

### Input capture (`input = "capture"`)

Every platform input event reaches the capturing mods first, in load order, as `sa_input(kind, a, b)`
(`sa_mod_api::input`; SDK `Mod::input(Input)`): key down (Windows virtual-key code, auto-repeat) and up, a typed
character (Unicode), mouse movement (pixels), a mouse button, the wheel (1/120 notches), focus. A mod that returns 1
consumes the event: the game and later mods do not see it. `input_capture(keyboard, mouse, cursor)` (SDK
`input::capture`) takes the keyboard (the game sees no key and no held key: typing does not move the player) and the
mouse from the game until the mod says otherwise, and shows the platform's cursor (natively the window's; in the
browser the page releases the pointer lock and sends absolute positions and clicks, as in the menus). The host
releases what a disabled mod held.

### Determinism

Nothing of level 5 runs without a mod that asks for it; the option switches are all on by default (the original),
so the default build, the mods build without mods and verify-all are unchanged.

## Level 6: images, the game's 2D drawing, projection (1.6)

What a mod needs to draw a full user interface over the game: its own images (fonts rendered into an atlas, skins,
icons) as textured quads, and the game's own font and textures where it should look like the game.

### Overlay images (`render = true`)

| function | SDK (`sa_mod`) | what |
|---|---|---|
| `image_create(w, h, flags, ptr, len)` | `Image::new(w, h, smooth, rgba)` | an RGBA8 image (`len` = `w * h * 4`, rows from the top, straight alpha); `image::SMOOTH`: linear filtering over a box-filtered mip chain when a quad scales it, else the nearest texel. Valid in every phase; drawable from the next overlay phase on |
| `image_destroy(h)` | drop | deletes it |
| `overlay_quads(h, ptr, count)` | `Overlay::quads(Some(&image), &[Quad])` | quads of image `h` (`image::NONE`, -1: plain colour), 9 words each: `x`, `y`, `w`, `h`, `u0`, `v0`, `u1`, `v1` (`f32`), `rgba` (tints the image); returns how many were inside the clip rectangle |
| `overlay_clip(x, y, w, h)` | `Overlay::clip(Some([x, y, w, h]))` | cuts the rest of this overlay phase's drawing (rectangles, text, quads) to a rectangle; a negative `w` or `h` (SDK `None`) removes it. Every phase starts without one |

Budget per mod (`sa_mod_api::image`): 64 images, 32 MiB of pixels together, sides up to 4096, 16384 quads per
call, 65536 quads (rectangles, text runs, quads) per frame. Over the budget `image_create` is FAILED with a log line.
A disabled mod's images are deleted. Natively and in the browser the overlay is the same wgpu pass over the game's
image, before the present; the game's own drawing is not touched.

### The game's 2D drawing (`render = true`)

Drawn in the game's next frame right after `Game::Render2dStuff` (with the HUD, under the overlay), in the order the
mods queued them in their overlay phase (at most 256 per mod and frame):

| function | SDK | what |
|---|---|---|
| `game_text(style_ptr, text_ptr, text_len)` | `Overlay::game_text(&FontText, text)` | a string in the game's font: the `CFont` state of `sa_types::text::FontText` (position, scale, colour, orientation, wrap, centre size, background box and colour, proportional, drop colour, edge or shadow, font style; 15 words, `sa_mod_api::text::encode`), the text in the game font's code page with its `~x~` codes, up to 400 bytes |
| `game_texture(dict, file, name)` (three `(ptr, len)` pairs) | `GameTexture::find(dict, file, name)` | texture `name` of the game's texture dictionary `dict`; without a dictionary of that name, the game folder's `file` (e.g. `models\txd\LD_SPAC.txd`) is loaded into a new one, which stays loaded. The game looks it up in its next 2D phase: AGAIN until then, afterwards a handle (the same for the same texture) or NOT_FOUND; at most 256 per mod; WRONG_PHASE in a hook |
| `game_sprite(texture, x, y, w, h, rgba)` | `Overlay::game_sprite(texture, [x, y, w, h], colour)` | `CSprite2d::Draw` of such a texture, bilinear, tinted |

### World settings (`state = "write"`)

`set_game_time(ms)` (`world::set_game_time`): the game's millisecond timer (`CTimer::m_snTimeInMilliseconds`,
0..=0x3FFFFFFF), the time its timed events count in; a change like those of level 3 (queued inside a hook).

### Projection (`state = "read"`)

`world_to_screen(x, y, z, out_ptr)` (SDK `state::world_to_screen(p)`): where a world point is on the screen with the
game's camera (`CSprite::CalcScreenCoors` with both visibility checks): writes the screen x and y in pixels and the
depth in front of the camera (three `f32`) and returns 1, or 0 off screen or behind the camera.

### Script commands

1.6 adds `IS_LINE_OF_SIGHT_CLEAR` (06BD: from, to, buildings, vehicles, peds, objects, dummies), a world query.

### Determinism

Nothing of level 6 runs without a mod that asks for it; the game's 2D phase is observed (`Render2dStuff` watched)
only from the first queued string, sprite or texture lookup on.

## Level 7: HTTP requests (1.7)

### Requests (`http = ["https://HOST/PATH/", ...]`)

| function | SDK (`sa_mod::http::Request`) | what |
|---|---|---|
| `http_get(url_ptr, url_len)` | `Request::get(url)` | starts a GET request in the background; returns its handle |
| `http_status(h)` | `status()` | AGAIN while it runs, then the HTTP status (100..=599), or FAILED |
| `http_body(h, ptr, len)` | `body()` | copies the body into the buffer; returns its whole size (AGAIN / FAILED as `http_status`) |
| `http_close(h)` | drop | forgets it (a running request is abandoned) |

A prefix is `http://` or `https://`, a host (and port) and a path that starts with `/`: `"https://api.open.mp/"`
allows every URL under it and no other host (`https://api.open.mp.example/` does not start with it). A URL with
whitespace, a backslash, a fragment, `..` or `%2e` is BAD_ARG, one no prefix allows DENIED. The manifest is the whole
consent: there is no prompt per request.

A mod holds at most 4 requests (`http::MAX_REQUESTS`, running or finished until closed). A request fails (FAILED, the
reason in the log once) without a connection, on a TLS error, with a body over 16 MiB (`http::MAX_BODY`) or after
30 s (`http::TIMEOUT_MS`); a 4xx / 5xx answer is a status like any other. Natively each request runs on its own
thread (ureq with rustls and the Mozilla root certificates; no system TLS library); in the browser it is the worker's
`fetch` without credentials, so a cross-origin URL needs the server's `Access-Control-Allow-Origin` (api.open.mp sends
`*`). Poll `http_status` once per frame. When the host disables a mod it drops its requests.

### Determinism

Requests run only for a mod that starts them; their timing is the network's, so a mod that waits for one is not
deterministic across runs (as with sockets).
## Level 8: pickups, checkpoints, blips, radar areas (1.8)

The game's own markers, as its missions use them. Everything a mod creates here it owns: it removes only its own, and
the host removes them when it disables the mod. Creation is refused inside a hook (WRONG_PHASE).

### Pickups (`entities = true`; the reads `state = "read"`)

| function | SDK (`sa_mod::markers`) | what |
|---|---|---|
| `create_pickup(model, kind, x, y, z)` | `create_pickup(model, kind, pos)` | `CREATE_PICKUP` with the pickup type `kind` (`ePickupType`, 1..=22), the model loaded first; returns the game's pickup handle |
| `destroy_pickup(h)` | `destroy_pickup(p)` | `REMOVE_PICKUP` |
| `pickup_collected(h)` | `pickup_collected(p)` | `HAS_PICKUP_BEEN_COLLECTED` (1 / 0; NOT_FOUND for a handle that names no pickup) |
| `pickup_touched(h)` | `pickup_touched(p)` | 1 while the local player stands where `CPickup::Update` picks the pickup up: the pickup shown and not waiting to come back, a type with the pick-up test (not the mines and packages, 9..=14), its object pickable, the player on foot and alive, closer than the original's height and distance constants. A test of the state between frames |
| `model_exists(model)` | `model_exists(model)` | 1 when the game defines the model (it has model info) |

### Checkpoint markers and the area test

| function | SDK | what |
|---|---|---|
| `create_checkpoint(kind, x, y, z, nx, ny, nz, size)` | `create_checkpoint(kind, pos, next, size)` | `CREATE_CHECKPOINT`: the game's checkpoint types 0..=8 (race arrows, finish flags, rings), pointing at `next`, `size` its radius |
| `destroy_checkpoint(h)` | `destroy_checkpoint(c)` | `DELETE_CHECKPOINT` |

The area test with the game's red marker is a script command (`scripts = true`): `LOCATE_CHAR_ANY_MEANS_3D` (00FE:
ped, x, y, z, radius x, y, z, marker), run every frame as the missions do; the marker shows for that frame.

### Blips

| function | SDK | what |
|---|---|---|
| `create_coord_blip(x, y, z, sprite, flags)` | `create_coord_blip(pos, sprite, flags)` | a radar blip at a point with radar sprite `sprite` (0: a plain marker); `blip::GLOBAL` shows it on the whole radar and map (`ADD_SPRITE_BLIP_FOR_COORD`, else `ADD_SHORT_RANGE_SPRITE_BLIP_FOR_COORD`), `blip::CONTACT_POINT` makes it a contact point |
| `create_entity_blip(kind, h)` | `create_ped_blip(p)`, `create_vehicle_blip(v)` | `ADD_BLIP_FOR_CHAR` / `ADD_BLIP_FOR_CAR` (`blip::PED`, `blip::VEHICLE`) |
| `destroy_blip(h)` | `destroy_blip(b)` | `REMOVE_BLIP` |
| `set_blip_colour(h, colour)` | `set_blip_colour(b, colour)` | `CHANGE_BLIP_COLOUR`: the game's colours 0..=8, any other value `0xRRGGBBAA` |
| `set_blip_scale(h, scale)` | `set_blip_scale(b, scale)` | `CHANGE_BLIP_SCALE` |
| `set_blip_display(h, display)` | `set_blip_display(b, display)` | `CHANGE_BLIP_DISPLAY` (1 marker only, 2 blip only, 3 both) |

### Radar areas (`render = true`)

| function | SDK (`sa_mod::radar`) | what |
|---|---|---|
| `set_radar_area(id, min_x, min_y, max_x, max_y, rgba)` | `set_area(id, min, max, colour)` | shows area `id` (0..1024, the mod's own numbering) over a world rectangle, replacing one of that id (not flashing) |
| `flash_radar_area(id, rgba, period_ms)` | `flash_area(id, colour, period_ms)` | the area alternates with `rgba` every `period_ms` (0 stops it); NOT_FOUND without such an area |
| `remove_radar_area(id)` | `remove_area(id)` | removes it |

The host draws the areas with `CRadar::DrawAreaOnRadar` right after the game's gang areas
(`CRadar::DrawRadarGangOverlay`), on the radar and on the map, each mod's in id order, the mods in load order.

### Script commands

1.8 adds `LOCATE_CHAR_ANY_MEANS_3D` (00FE), `HAS_MODEL_LOADED` (0248), `EXPLODE_CHAR_HEAD` (0321),
`SET_CHAR_DECISION_MAKER` (060B) and `CLEAR_CHAR_TASKS_IMMEDIATELY` (0792): what actors (peds a mod created, with a
heading, proofs, a decision maker, animations and health) need beyond level 3.

### Determinism

Nothing of level 8 runs without a mod that asks for it; the gang overlay is observed only from a mod's first radar
area on.
