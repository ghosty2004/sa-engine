# ADR-004: mods: a WebAssembly mod API, with SA:MP as a mod

Status: **accepted** (user, 2026-11-17).
ROADMAP task: MOD-2. Related: ADR-001 (globals, `orig!`, `#[hook]`), ADR-003 (architecture), MOD-1 (`.asi` compatibility),
PERF-1 / RENDER-2, `crates/sa-samp-api` (the SA:MP game adapter), `docs/samp/game-adapter.md`.

## 1. Context and decisions

The game is a 1:1 port of `gta_sa.exe` 1.0 US, verified against the original (L1–L3, verify-all). Anything that is not
the original — higher resolutions, anti-aliasing, modern lighting, new UI, SA:MP itself — must not change what the
port is, and must not make the oracle comparison meaningless.

User decisions (2026-11-17):

1. **The game stays original; extras come as mods.** A compile-time Cargo feature `mods` adds mod support. The default
   build has none of it, so the parity build and the oracle are untouched.
2. **Mods are WebAssembly modules.** The same `.wasm` runs natively (wasmtime) and in the browser (the browser's own
   WebAssembly engine), sandboxed: a mod cannot touch game memory, files or the network except through the API.
3. **The first API is visual**: graphics settings (resolution, MSAA, texture filtering, draw distance), post-processing,
   2D/3D overlays, HUD/UI, input. It cannot change game logic.
4. **SA:MP becomes a mod.** samp-rs is restructured onto the mod API, so the API grows, step by step, to everything
   SA:MP needs (§5). SA:MP is the first large mod and the proof that the API is complete enough for other developers.

## 2. What stays out of scope

- **MOD-1 stays as it is**: existing `.asi` mods on the original `gta_sa.exe` (route A). ADR-004 is the new, portable
  mod system; MOD-1 is compatibility with the old one. They do not share code paths.
- **SA:MP on the original exe stays**: samp-rs as a DLL inside the unmodified `gta_sa.exe` (the `original` backend of
  `sa-samp-api`) keeps working, 1:1 with 0.3.7-R4, with both loading mechanisms. The mod is an additional host for the
  same client code, not a replacement of that route.
- No changes to port bodies (ADR-003 §2). Every mod entry point sits around ports: at the frame loop, in the platform
  layer, or at a registry hook (before/after), never inside a body.

## 3. Architecture

```
  mod.wasm (any language with a wasm32 target; Rust first, via the sa-mod SDK crate)
      │  imports: the API (one interface definition, §3.2)      exports: init / callbacks
      ▼
  sa-mod-host  (new crate, behind feature `mods`)
      ├─ loader: manifest, permissions, instantiate, call scheduling, errors, unload
      ├─ api: implementations over sa-samp-api traits + the render/platform layer
      └─ engine backend: wasmtime (native)  │  browser WebAssembly via JS glue (web)
      ▼
  sa-game  (frame loop, platform layer, hook registry)     ← unchanged without `mods`
```

### 3.1 Crates

| crate | what | depends on |
|---|---|---|
| `sa-mod-api` | the interface definition and its Rust types (shared by host and SDK); no `unsafe`, builds for every target | `sa-types` (since M3; M2: `sa-samp-api`) |
| `sa-mod-host` | loads and runs mods; one implementation, two engine backends (wasmtime / browser) behind one trait (ADR-003) | `sa-mod-api`, `sa-access` (since M3; M2: `sa-samp`), platform/render layer |
| `sa-mod` (SDK) | what a mod author links: safe Rust bindings, macros for the entry points, examples | `sa-mod-api` |
| `mods/*` | first-party mods the project uses: `resolution` (M1), `samp` (M4: SA:MP's mod host) | `sa-mod` |
| `examples/mods/*` | the SDK's example and test mods: `hello`, `trainer-hud`, `courier`, `broken` | `sa-mod` |

`sa-game` gets `--features mods`; without it the host crate is not compiled in.

### 3.2 The interface and the ABI

- **Decision (M0, 2026-11-17): a core-wasm ABI.** A mod is a plain core WebAssembly module: it imports the host
  functions from the module `"sa"`, exports its entry points (`sa_api_version`, `sa_init`, `sa_overlay`, ...) and its
  memory as `"memory"`, passes only `i32` / `f32` values, and hands text and buffers over as `(ptr, len)` in its own
  linear memory, which the host reads with bounds checks (a bad range is a status code, never a wild read). Every
  host function returns an `i32` (a result `>= 0` or a status: `-1` denied, `-2` bad argument). No component
  model, no WIT, no `jco`.
- **The interface is defined once**, in `crates/sa-mod-api` (no dependencies, no `unsafe`): two `macro_rules!` lists,
  `for_each_import!` (name, parameters, result, the permission it needs) and `for_each_export!`. Everything is
  generated from them: the host's import table and typed call decoder (`IMPORTS`, `Call::decode`) and the export
  signature checks (`EXPORTS`); the SDK's `extern "C"` import block and its exported entry points (`sa-mod`'s `ffi`
  and `entry!`). The browser glue (`web/mods.js`) knows no API function: the host passes it the import names at load
  time and every import calls back into one host entry with its index. Adding a function is one line in
  `sa-mod-api`.
- **Why core wasm** (the spike, `target/mod-spike`, and M0 itself):
  - it is the one format both engines run as is: wasmtime natively (Cranelift on x86_64; on i686, which has no
    Cranelift backend, Cranelift's Pulley bytecode and wasmtime's interpreter: fuel, traps and memory limits work
    the same; a host call costs ~120 ns there) and the browser's `WebAssembly.Module` / `Instance`, synchronously, in
    the game's worker. The component model would need `jco` transpiling to core modules plus generated JS per mod
    in the browser and wasmtime's component runtime natively: two ABIs to keep equal;
  - one ABI means one host implementation of every API function (`sa-mod-host/src/api.rs`), reached through a
    backend-neutral `dispatch(index, values, memory reader)` from both engines;
  - mods stay buildable from any language with a wasm32 target and no extra toolchain (`cargo build --target
    wasm32-unknown-unknown`; hello.wasm is 26 KB).
  - The cost: hand-kept value conventions (UTF-8 `(ptr, len)`, `0xRRGGBBAA` colours, handles) instead of WIT's rich
    types. They stay few because the definition is one place. Revisit when browsers ship the component model; the
    `sa-mod-api` lists can then generate a WIT file.
- Budgets per engine: wasmtime fuel natively; on the web the host instruments the module at load time (one counter
  global, decremented at every function entry and loop header, a trap below zero; `sa-mod-host/src/backend/meter.rs`),
  which also catches endless loops, which a time check could not interrupt. The memory cap is the store limiter
  natively and the instrumented memory's maximum on the web.
- The API is **versioned** (`api_version` in the manifest, semver). The host supports the current major version; a mod
  built for a newer minor version is refused with a clear message.
- Handles, never pointers: peds, vehicles and objects are the generational pool references `sa-samp` already uses
  (`slot << 8 | reuse`). A stale handle is an error, not a wild read. Mods never see game addresses.

### 3.3 Manifest and permissions

Each mod ships a `mod.toml` next to its `.wasm`:

```toml
[mod]
id = "resolution"          # unique
name = "Resolution & MSAA"
version = "0.1.0"
api_version = "1.0"

[permissions]
render   = true             # graphics settings, post-processing, overlays
input    = "read"           # none | read | capture
ui       = true
state    = "none"           # none | read | write            (level 2+)
hooks    = []               # registry hooks, by name          (level 2+)
entities = false            #                                   (level 3+)
net      = []               # allowed host:port targets         (level 4)
http     = []               # URL prefixes for GET requests     (API 1.7)
files    = "own"            # none | own (the mod's folder, through the host)
```

The host grants only what the manifest asks for and what the user's mod settings allow. A mod that calls an API it has
no permission for gets an error, not a crash.

### 3.4 Execution model

- The game is single-threaded (`docs/samp/game-adapter.md` §3); mods are called on the game thread, in a fixed order
  (the load order), at fixed points:
  - the settings phase (API 1.1, M1): once in the boot, after the game read its settings file and before it selects
    its video mode and creates its D3D device (graphics settings requested there apply from the first frame);
  - frame phases (before logic, after logic, before render, after render / overlay, before present);
  - platform events (input, resize, focus);
  - registry hooks (before/after a named original function), from level 2.
- Mid-frame hooks are **read-only**, as in `sa-samp-api`: mutations are queued and applied at the next frame phase.
- Each call has a fuel/time budget (wasmtime fuel natively, a time check on the web). A mod that traps or exceeds its
  budget is disabled for the session and reported; the game keeps running.
- Memory per mod is capped (manifest default, user-overridable).

## 4. API levels

| level | contents | permissions | first users |
|---|---|---|---|
| **1 — visual** (first) | graphics settings: resolution / window mode, MSAA, anisotropic filtering, draw distance multiplier, frame limiter; post-processing passes (WGSL shaders on the final image); 2D overlay (text, rectangles, images); 3D debug overlay (lines, boxes); HUD/UI widgets; input read and capture; per-mod config files | `render`, `input`, `ui`, `files` | resolution mod (replaces the paused `--resolution` work), MSAA / filtering, FPS overlay |
| **2 — events and hooks** | named game events (ped created/destroyed, enter/exit vehicle, weapon fired, damage, death); before/after hooks on registry functions with typed arguments; game state **read** (player, peds, vehicles, objects, camera, time, weather, zones) | `state = "read"`, `hooks` | trainers, mini-maps, telemetry |
| **3 — game control** | state **write** (position, health, weapons, tasks, camera, weather, time); entity creation and removal (peds, vehicles, objects, pickups, markers, checkpoints); script commands; audio | `state = "write"`, `entities` | gameplay mods |
| **4 — services** | networking (UDP natively; on the web WebSocket through `sa-udp-proxy`, `sa-net`'s transports); persistent storage; inter-mod messages | `net`, `files` | **SA:MP** |

Levels 2–4 map almost one to one onto the `sa-samp-api` traits (`Peds`, `Vehicles`, `Objects`, `WorldState`,
`WorldEntities`, `Camera`, `Hud`, `Text2d`, `Menus`, `Input`, `GameAudio`, `Streaming`, `ScriptCommands`,
`Overlay`, `GameHooks`). The mod API is that adapter, exposed across the wasm boundary, plus the render/settings
surface of level 1. One implementation: the `sa-samp` engine backend serves both.

### 4.1 Level 1 and the renderer

Graphics settings that the original game does not have (MSAA at any level, anisotropic filtering, resolutions above
the mode list, a longer draw distance) live in the platform/render layer (RENDER-2), not in the game. With no mod
loaded, the renderer behaves exactly as today (image diff vs the parity renderer stays the RENDER-2 acceptance).
A setting that the original *does* have (video mode, frame limiter, draw distance slider) is applied through the
original's own menu state, so the game sees what it would see if the player had chosen it in the menu.

The video mode (M1, `crates/sa-game/src/mods.rs`): the D3D9 adapter (platform layer) lists the real display's modes
plus the requested size instead of its fixed table; a size the original's mode list offers (4:3, 5:4, 5:3, 16:10)
becomes the menu's video mode as `gta_sa.set` would hold it, any other (16:9, ...) takes `psSelectDevice`'s other
original path (`IsVMNotSelected` clear, `gCurrentVideoMode` = the mode), the menu then naming the closest listed mode.
No port body changes. A different video mode is a different game input (screen size and aspect reach the camera and
the HUD): its reference is the original at the same mode, not the 800x600 run (WORKLOG, MOD-2 M1).

### 4.1a Level 2 (M2, 2026-11-18): state, events, hooks

API 1.2 (a minor version; reference `docs/mods/api.md`). Decisions:

- **One vocabulary with SA:MP** (the dependency on sa-samp / sa-samp-api was inverted in M3, §4.1b: the types are
  `sa-types`, the reads `sa-access`). The state records (`sa_mod_api::state`: `PlayerInfo`, `PedInfo`, `VehicleInfo`,
  `WorldInfo`, `CameraInfo`) have the game adapter's field types (`sa-samp-api`: `Vec3`, `PedRef`, `VehicleRef`,
  `ModelId`, `GameState`, ...) and are filled from the adapter's snapshots (`PedSnapshot`, `VehicleSnapshot`,
  `CameraState`); `sa-mod-api` depends on `sa-samp-api` (its own layer above the leaves). The host reads the game only
  through `Rc<dyn GameAdapter>`: in the game that is `sa-samp`'s engine backend, the same reads samp-rs makes, so a
  read the mod API needs is added to the adapter (wanted level, pool lists and held keys were, for M2) and serves
  both. On the wire a record is fixed little-endian 32-bit words in field order (`Words`), one definition for host
  and SDK.
- **Events are derived, not hooked.** Frame begin (start of a main-loop iteration) and end (after the overlay phase),
  game state, player spawn / death, vehicle enter / exit, area and keys come from comparing the adapter's state with
  the previous iteration's (`sa-mod-host/src/events.rs`): no port body or call site changes, and the reads happen only
  for kinds some mod subscribed to (`events_subscribe`). Events at fixed points other than these (weapon fired,
  damage, ped created) map onto the adapter's `GameHooks` callbacks and come with the dispatch points samp-rs needs
  (M3 / M4).
- **Hooks are registry observation.** A before / after hook watches an entry of the `orig!` address registry
  (`sa_core::orig::observe`): when a dispatch reaches a watched entry, the observer gets copies of the argument words
  with the declared types and, after the call, of the result. Values reach the mod typed (`hook::Value`): integers,
  floats, a pointer to a pooled ped / vehicle / object as its handle, other pointers hidden. Read-only: the call runs
  exactly as without the hook. Names are the registry's (catalogue) names, and the manifest lists the ones a mod may
  hook. Cost: the check is one relaxed flag load of the entry the dispatch already looked up, compiled in only with
  the `sa-core` feature `observe` (enabled by sa-game's `mods`); without it `Entry` has no flag and the dispatch is
  unchanged. Limit: calls a port makes directly to a ported function of its own crate do not pass the registry and
  are not seen; virtual calls (`ProcessControl`, `Render`, the vtable redirects samp patches), cross-crate calls and
  function pointers are. Each mod gets at most 4096 hook calls per iteration (the rest are dropped and counted).
- **Permissions** `state = "read"`, `input = "read"`, `hooks = [names]`, checked per host function and per hooked
  name; event kinds need the permission of what they reveal (frames none, keys `input`, the rest `state`).
- **Determinism.** Level-2 mods only read: the crossing run with the trainer HUD loaded gives the same report
  (5000+ functions' dispatch counts, platform calls, blocked iterations) as without it, frame pacing aside.

What SA:MP needs and M2 does not give yet: writes and mid-frame decisions (`HookAction::Skip`, changed arguments:
`Shot`, `Damage`, `PlayerContext` swaps around `ProcessControl`), entity creation, script commands, audio (M3); the
network and storage (M4). The `GameHooks` callbacks that need a dispatch point inside a port (`fire`, `ped_damage`,
`enter_vehicle_task`, ...) get it there, as samp-rs's engine host needs the same points.

### 4.1b Level 3 (M3, 2026-11-18): changes, entities, script commands, sounds, hook decisions

API 1.3 (reference `docs/mods/api.md`). Decisions:

- **The engine stays a GTA SA rewrite (user, 2026-11-18).** The engine exposes a generic mod API only, never
  SA:MP-specific paths; `crates/sa-samp` (with sa-game's `samp` feature and the SA:MP adapter) leaves `crates/` once
  SA:MP runs as a mod (M5), and samp-rs then builds as a mod against the generic API alone. M3 inverted M2's
  dependency (sa-mod-api on sa-samp-api, the host reading the game through `sa_samp::engine()`):
  - `crates/sa-types` (leaf, no dependencies): the game's value types (vectors, matrices, handles, model / weapon
    ids, `GameState`, the ped / vehicle / camera records, script arguments, `GameError`). sa-samp-api re-exports them
    at their old paths, so samp-rs is unchanged; sa-mod-api's records are expressed in them.
  - `crates/sa-access` (engine side, above sa-core): the running game's state read and changed through handles
    (`Game<V: View>`: pool lookups, records, keys, the private script threads, changes, entity creation and deletion,
    sounds; the fake memory for tests). Everything moved here from sa-samp (its view, addresses, script threads,
    reads and the changes samp-rs already used) without a copy: sa-samp's `NativeGame` holds an `sa_access::Game`,
    derefs to it and its trait methods delegate. M5 deletes sa-samp; sa-access stays.
  - sa-mod-api depends on sa-types only, sa-mod-host on sa-mod-api, sa-access and sa-types: no SA:MP crate below the
    host.
- **Changes only between frames.** A change asked for inside a hook is queued and applied at the next frame
  boundary; entity creation / deletion and script commands are refused there (WRONG_PHASE). Between frames they run
  at once, through the game's own code (the ported script command handlers: positions, weapons, wanted level,
  clock, entity creation with model requests, pool allocation, `CWorld::Add`), plain field writes only where the game
  itself only stores the value. A panic of unported game code inside a change becomes FAILED with a log line
  (natively; on wasm32 a panic aborts, which is why only commands whose handlers are ported are offered).
- **Ownership.** A mod deletes only entities it created; the host deletes them when it disables the mod (at the next
  frame boundary).
- **Script commands are a whitelist with signatures** (`sa_mod_api::script::COMMANDS`: opcode, name, parameter
  letters). The game's handlers trust handles; the host checks every `p` / `v` argument against the pools first.
  Creation and deletion stay typed calls (ownership).
- **Hook decisions** (`hook_set_arg`, `hook_skip`, `hook_set_result`) need `state = "write"` and work only on
  `hook::CONTROLLABLE` functions. In the engine, `sa_core::orig::observe::around` takes the argument words mutably and
  gives the call the words as the observer left them (read back with the declared types), returns a chosen result
  without calling (skip), or replaces the result; values are encoded per declared type (integers truncated to their
  width, floats, x87, pointers: a handle becomes its entity's address).
- **Dispatch points for direct calls.** Calls a port makes to a ported function of its own crate went through the
  `orig!` binding straight to the Rust function (`=> crate::path`) and were invisible. With the feature `observe`,
  such a binding with a literal address checks one relaxed counter of watched entries and, when some function is
  watched and the callee is, takes the same observed path as a registry dispatch (`observe::watched_at`). Without the
  feature the check is a constant `None` and the binding is the direct call it was; with it and nothing watched it is
  one load. The registry's call counts are not touched (the crossing report stays comparable).
- **Sounds**: front-end sounds through `CAudioEngine::ReportFrontendAudioEvent`; mission sounds through
  `ADD_ONE_OFF_SOUND` (its unported audio case falls back as sa-samp did).

### 4.1c Level 4 (M4, 2026-11-18): sockets, storage, the clock; samp-rs's core apart from its hosts

API 1.4 (reference `docs/mods/api.md`). Decisions:

- **Sockets are sa-net's transports.** `net_open` / `net_send` / `net_recv` / `net_state` / `net_close` work on
  connected, non-blocking datagram sockets; the host's `Network` (`sa-mod-host/src/net.rs`) opens them through
  `sa-net`, the crate samp-rs's RakNet layer already polls: natively `UdpTransport` (a real UDP socket), in the browser
  `WebSocketTransport` to `sa-udp-proxy` (the page's `?proxy=`, default `ws://127.0.0.1:7781`), one binary frame per
  datagram. One host implementation of the API, two backends; no new transport code. sa-net's browser transport is
  now its feature `websocket`, so a wasm32 module without it (a mod) imports nothing from JavaScript.
- **Permission: allow rules on the target text.** `net = ["udp:HOST:PORT", ...]`, `*` for any host or port, compared
  (host without case) with the `HOST:PORT` the mod opens, before resolving: the same check natively and in the
  browser, where the proxy resolves (and keeps its own allow list). A rule for any host is allowed with a warning
  (SA:MP joins servers the user names); a target no rule allows is DENIED.
- **Polled, budgeted, closed with the mod.** Nothing blocks and no callback enters a mod: mods poll in a frame phase
  (AGAIN = `-6`, a new status, when nothing is queued). Per mod: 8 sockets, datagrams up to 65507 bytes, 1024 sends
  and 1 MiB per main-loop iteration (AGAIN beyond, until the next iteration). A disabled mod's sockets are closed.
- **Storage is per mod id and persistent** (`storage = true`): `storage_read` / `write` / `delete` / `list` on keys
  of `A-Z a-z 0-9 . _ -`, at most 1 MiB per value, 1024 keys and 16 MiB per mod. Natively a folder under the user's
  data directory (`$XDG_DATA_HOME` / `~/.local/share` / `%APPDATA%` + `sa-engine/mods/ID`, `$SA_MOD_DATA` replaces
  the root), one file per key written through a renamed temporary file; in the browser IndexedDB (`sa-mod-storage`),
  read into the worker before the game starts (the host's calls are synchronous) and written back in the background.
  `files = "own"` keeps its meaning (the files shipped in the mod's folder).
- **A clock.** wasm32 has none, and protocol code needs one: `clock` gives the host's monotonic time and the wall
  clock in microseconds, without a permission.
- **samp-rs's core apart from its hosts** (§5). The core is `samp-client`'s platform-independent part (the RakNet
  session `netclient`, the connection lifecycle, sync builders, class selection, remote players, vehicles, combat,
  the UI core) with `samp-proto`, `samp-ui` and `samp-saa`; it reaches its platform through seams that already were
  traits or became one: the transport (`netclient::NetConnector` opening `sa_net::DatagramTransport`s), the clock
  (`samp_proto::time`: `std::time` natively, the browser's with `samp-proto`'s feature `web`, otherwise the clock a
  host installs with `set_clock`) and the game (`sa-samp-api`'s `GameAdapter` / `GameHooks`). Its hosts:
  `original` (the `cfg(windows)` DLL modules of `samp-client`, unchanged), `engine-native` (`sa-game`'s `samp`
  feature with `crates/sa-samp`; it enables `samp-client`'s feature `websocket` for the browser connector) and
  **`mod`** (`mods/samp`, `sa-mod-samp`: the mod's sockets as the transport, the host's clock, settings in the mod's
  storage). `samp-client` without features builds for `wasm32-unknown-unknown` with no JavaScript imports, and
  `mods/samp` builds into a mod the host accepts (`samp.wasm`, imports from `"sa"` only).
- What M5 has to add: a `GameAdapter` over the mod API (levels 2-3) so the core's game logic (the `samp_net` client of
  `sa-game` today, then samp-rs's full client) runs in the mod; the UI drawn through the overlay; joining a 0.3.7 /
  open.mp server natively and from the browser; then `crates/sa-samp`, `sa-game`'s `samp` feature and its SA:MP glue
  leave the engine.

### 4.1d Level 5 (M5, 2026-11-18): SA:MP as a mod, the engine without SA:MP

API 1.5 (reference `docs/mods/api.md` level 5). Decisions:

- **The engine has no SA:MP code.** `crates/sa-samp`, sa-game's `samp` feature and its glue, sa-core's samp frame
  points left; `sa-samp-api` (the adapter traits samp-rs's client is written against) lives in `samp-rs/crates`,
  implemented by the DLL's original-exe backend and by the samp mod. samp mode's flags became `sa_core::options`:
  switches of the game's own features (all on by default), exposed as `set_game_option`; the ports test them where
  they tested the flags.
- **What SA:MP needed became generic API**: the player's pad, entity transforms, the pause menu, player models,
  the HUD, big messages, game options, text script arguments and the commands for tasks / animations / warps /
  streaming, input capture (events before the game, keyboard / mouse / cursor taken), several mods on the command
  line.
- **Servers are the user's choice, approved by the host** (user, 2026-11-18): a manifest asks for
  `net = ["udp:user-approved"]`; the first `net_open` to a target the user has not answered for shows the host's own
  prompt (drawn over the mods' overlay, keys taken until the answer) and returns AGAIN; the answer is kept per mod
  id in the host's storage area (`sa-engine-host`, natively a folder, in the browser IndexedDB) and asked once. Any
  network mod can use it; explicit `udp:HOST:PORT` rules keep working.
- **The samp mod** shows a connect screen (a server browser, §4.1e), then runs
  `samp_client::hosted` (the engine client moved out of sa-game, plus remote players) on its game adapter over the
  mod API; the adapter answers `Unsupported` for what the API does not offer, as any backend lacking a feature.
- Multiplayer starts with `--mods target/mods/resolution,target/mods/samp` (single player without the samp mod).
- **A mod's UI is images and quads (API 1.6).** The overlay draws quads of mod images (RGBA8, per-mod budget: 64
  images, 32 MiB, sides up to 4096; `SMOOTH`: linear over a box-filtered mip chain), plain-colour quads and a clip
  rectangle, natively and in the browser through the same wgpu pass before the present. Text is the mod's own
  business (a glyph atlas is an image); the game's own look comes from two generic calls drawn in the game's 2D phase
  (after `Game::Render2dStuff`, with the HUD, under the overlay): `game_text` (a `CFont` state and a string) and
  `game_sprite` of a `game_texture` (a texture of the game's dictionaries, a dictionary loaded from the game folder
  when missing; looked up in that phase, where loading is safe, so the call answers AGAIN first). `world_to_screen`
  is the game's own projection. No SA:MP type crosses the API.
- **samp-rs draws the same UI on every backend.** The engine overlay's code that left with crates/sa-samp (glyph
  atlas with pre-outlined glyphs and the symbol fonts, D3DX `DrawText` layout, draw list, skin) is
  `samp-rs/crates/samp-overlay`: `QuadUi` implements `UiBackend` on a `Canvas` of images, quads, game text and game
  sprites; the samp mod's canvas is the mod API. Measured against the 72ea67c0 engine overlay with the reference
  scene (`samp_client::ui::scene`, the mod's `ui_scene = ref`; 800x600, same game data and UI files): 98-99.7 % of
  the pixels within 8 levels per step, the UI elements identical; the rest is the world (sun corona, lighting) and
  the 3D label's anchor (the scene places it from the camera's front vector, the old demo from its matrix).
- **SA:MP's UI files are the user's**, copied into the mod's folder (`files = "own"`: `sampgui.png`, `mouse.png`,
  `gtaweap3.ttf`, `sampaux3.ttf`, optionally `arialbd.ttf`); settings a page's empty storage lacks (the server,
  `auto_connect`, the auth table) are read from files of the same name there too, so the browser needs no page
  option and `serve.py` no SA:MP option.
- **The start screens are a game option** (`StartScreens`, 1.6): the samp mod turns them off in `sa_init` like
  samp.dll's start-up; `--start-in-world` (skipping main.scm's intro mission) stays a run option, as samp.dll ships
  its own main.scm.

### 4.1e HTTP requests (API 1.7, 2026-11-19)

API 1.7 (reference `docs/mods/api.md` level 7). Decisions:

- **GET only, by URL prefix.** `http = ["https://HOST/PATH/", ...]` in the manifest; a prefix needs a path starting
  with `/`, so it cannot be stretched to another host, and a URL with `..` / `%2e` cannot climb out of it. The
  manifest is the consent (a list fetched over HTTP is data, not a server the mod talks to), unlike UDP targets,
  which the user approves one by one.
- **Non-blocking, bounded.** `http_get` starts a request; `http_status` / `http_body` poll it; at most 4 per mod,
  16 MiB, 30 s. The host's `HttpClient` (`sa-mod-host/src/http.rs`) has one backend per platform: natively a
  thread per request with ureq and rustls (no OpenSSL, the Mozilla roots), in the browser the worker's `fetch`
  through the glue (`web/mods.js`), which needs CORS on the server.
- **First user: the samp mod's Internet tab.** It reads the open.mp server list (`https://api.open.mp/servers/`,
  JSON with hostname, players, mode, language, password, version); the selected server's ping, rules and players come
  from SA:MP's UDP query (`samp_proto::query`) through the approved sockets. Tests point the mod at a local fixture
  (`server_list_url`), never at the public list.
### 4.1f Pickups, checkpoints, blips, radar areas (API 1.8, 2026-11-19)

API 1.8 (reference `docs/mods/api.md`). What SA:MP's world RPCs need, as GTA SA's own concepts:

- **Pickups, checkpoint markers and blips are owned entities** (`entities = true`): `create_pickup` (`CREATE_PICKUP`,
  the model loaded first), `create_checkpoint` (`CREATE_CHECKPOINT`, the game's checkpoint types with their pointing
  direction), `create_coord_blip` (the four sprite blip commands: short range or global, contact point or not) and
  `create_entity_blip` (`ADD_BLIP_FOR_CHAR` / `CAR`); each has its `destroy_*`, and colour, scale and display change
  only the mod's own blips. The host deletes what a disabled mod owned, as for peds, vehicles and objects. Handles are
  the game's own (pickup handle, checkpoint and blip script handles): the game's code already ignores a stale one.
- **Reads** (`state = "read"`): `pickup_collected` (`HAS_PICKUP_BEEN_COLLECTED`), `pickup_touched` (the local player
  stands where `CPickup::Update` picks the pickup up: shown, a type with the pick-up test, on foot, alive, within the
  original's height and distance constants) and `model_exists`. `pickup_touched` is a state test between frames, not
  a hook in `CPickup::Update` (no port body changes); samp's DLL reports from that hook, so a report can come up to a
  frame earlier or later than the original's (ledger).
- **The area test with its marker** is the script command `LOCATE_CHAR_ANY_MEANS_3D` (whitelisted with
  `HAS_MODEL_LOADED`, `EXPLODE_CHAR_HEAD`, `SET_CHAR_DECISION_MAKER`, `CLEAR_CHAR_TASKS_IMMEDIATELY`), as the game's
  missions do it every frame.
- **Radar areas are visual** (`render = true`): `set_radar_area` / `flash_radar_area` / `remove_radar_area` keep
  per-mod rectangles (ids `0 .. 1024`, the mod's numbering) that the host draws in id order with
  `CRadar::DrawAreaOnRadar`, after `CRadar::DrawRadarGangOverlay` (registry observation of its call, watched from the
  first area on; the radar and the map), alternating with the flash colour every period. sa-access reaches
  `DrawAreaOnRadar` through a binding in sa-core (`catalog/bindings.csv`, twin in sa-hud).
- **Actors** need nothing new: `create_ped` plus the whitelisted commands (heading, proofs, decision maker, animations,
  task flush, health). samp's actors are ped type 5, `create_ped` makes type 4 (ledger).
### 4.1g Objects, models and building removal (API 1.9, 2026-11-19)

Reference `docs/mods/api.md` level 9. What SA:MP's objects needed, in GTA terms:

- **Objects by handle**, like peds and vehicles: their state (`object_info`: model, area, streamed in, visible,
  collides, attached, scale, speeds, bounding sphere), transform and pool list (`state = "read"`); changes on any
  live object (`state = "write"`): the position as the game teleports an object (`CObject::Teleport`, a `vslot`
  call, so the RenderWare frame and the sector lists follow), a whole matrix (rows written, then that teleport), the
  rotation, scale, visibility and collision through their script commands, the move and turn speed (what peds
  standing on a moving object move with), attaching to a ped, vehicle or object and detaching through the game's
  attach commands. A mod that moves an object interpolates it itself every frame (the API has no "move to"): one
  matrix write per object and frame, as samp.dll does.
- **The skeleton**: `ped_bone_matrix` (an `ePedNode`'s world matrix from the skin hierarchy after
  `CEntity::UpdateRpHAnim`), so a mod can place an object on a bone.
- **Materials**: a slot (geometry order, as the original's material list) drawn with a texture of a model's
  dictionary (`RwTextureRead` with the dictionary current, a reference the slot holds) and / or modulated by a
  colour (prelighting off, surface properties 1, 0, 1), applied around `CObject::Render` and restored after it. The
  host observes `CObject::Render` only while some object has slots; deleting the object releases them.
- **Building removal**: buildings, dummies and objects of a model (or any) in a sphere go into interior area 1 and
  2000 units down and the occluders within 500 units are cleared (their list links kept); the session's removals
  apply to every instance a streamed IPL loads later (`CFileLoader::LoadObjectInstance` observed after the first
  removal). Removals last for the session: the host cannot give a removed building back.
- **Models**: `model_info` (kind from the model info's class, loaded flag), `model_request` / `model_release`
  (`entities = true`; released when the mod is disabled), `set_model_draw_distance` (atomic models; restored when the
  mod is disabled). The id range and what exists are the original's (20,000 model infos, its definition files):
  models SA:MP adds (`samp.img`, ids 18631..=19999) exist only once data mods (§4.2) can register them.
- **Layer**: everything lives in `sa-access` (`objects.rs`), which calls the game's own functions (teleport,
  `RwTextureRead`, the hierarchy lookups) through `sa-core`'s bindings and a `vslot`; the host keeps the session
  state the render and IPL observers need (`sa_access::objects` thread-locals) and per mod what it must give back.

### 4.1h Vehicle damage, speeds, gravity, statistics; more script commands (API 1.11, 2026-11-19)

Reference `docs/mods/api.md` level 10. What SA:MP's vehicle, player and camera RPCs needed, in GTA terms:

- **Vehicles** (`state = "read"` / `"write"`): the class (from the vtable, as the game's `IsAutomobile` & co.
  decide), the damage manager's panel, door and light states and the tyre states, and setting them as a loaded
  saved game does (`CAutomobile::SetupDamageAfterLoad`, through an `sa-core` binding of the port); the move and turn
  speed.
- **Peds** (`state = "write"`): the move speed, with the standing flags toggled so the ped leaves the ground.
- **World** (`state = "write"`): the gravity and the float statistics (`CStats::StatTypesFloat`, written as the game
  stores them, without its on-screen notice). `GAME_GRAVITY` is constant data (`.rdata`), which sa-game maps
  read-only natively as the original's loader does: the engine registers an image writer
  (`sa_core::orig::registry::set_image_writer`, sa-game's `arena::write_image`, which lifts the protection for the
  write the way samp.dll's `VirtualProtect` does), and `View::write_image` uses it; the browser build has no page
  protection. The host gives the gravity back when it disables the mod.
- **Script commands** (`scripts = true`): the vehicle tuning, door, train, trailer, siren, engine and lights,
  plane gear, proofs and water commands, the camera's point-at, scripted move and FOV commands, explosions, the
  ground height, jetpack, look-at, melee style, mobile phone, weapon slots, radio channel, shops and fires. The
  handlers of several of them trust a vehicle's class or an index (an upgrade on a bike, a train command on a car,
  a door past the car nodes, an explosion type past the table): `sa-access` refuses those arguments before the
  command runs (`Game::refused_arguments`), as it refuses unknown animation blocks and models.
- **Not here**: train creation (mission trains are script-thread things the private thread does not own), particle
  effects (registered with the calling script's cleanup), window flags, the plate texture and GXT entries for menus
  stay for a later level.

### 4.2 Data mods (added by the user, 2026-11-17)

Code mods (levels 1–4) cannot add content on their own: a new vehicle, ped, weapon or building needs files (models,
textures, collision, definitions). A mod may therefore also ship a **data folder** next to its `.wasm` (a mod may be
data only, with no `.wasm`):

- **Virtual file system overlay.** The mod's `data/` tree is laid over the game folder: a file with the same path
  replaces the original for that session (`models/gta3.img` entries, `data/handling.cfg`, `data/vehicles.ide`,
  `models/generic/*.txd`, audio banks, `.ifp` animations, `.col`, `.dat` / `.ide` / `.ipl` lines), without changing the
  user's install. Text definition files are merged line by line (a mod adds or overrides entries; it does not ship a
  full copy). Load order decides conflicts and is shown to the user. The game reads every file through the existing
  file layer, so the overlay sits there (platform layer, ADR-003), not in the ports.
- **New model IDs.** A mod registers new content through the API ("a new vehicle based on class `automobile`, with
  these files"); the host assigns free IDs and inserts the definitions as if they came from `vehicles.ide`,
  `handling.cfg`, `carcols.dat` and friends. Typed registrations for vehicles, peds, weapons, objects and their
  handling / colours / sounds.
- **Raised limits only with `mods`.** The original has fixed limits (model info count, IMG entries, the vehicle, ped,
  object and building pools, the TXD and COL slots). The parity build keeps them exactly (ADR-001 layouts, oracle).
  The `mods` build may raise them (a "limits" table read at start-up), recorded in the divergence ledger as a
  mods-only difference; verify-all runs on the parity build.
- **New behaviour on top** comes from levels 2–3 (hooks and state), e.g. a vehicle class with custom physics. A new
  vehicle on an existing class (car, bike, boat, plane, heli, quad, monster truck) needs data only.
- **SA:MP:** 0.3.7 has no custom models (0.3.DL added them), so on 0.3.7 servers custom content is single-player or
  needs a server that supports it explicitly; the SA:MP mod decides.
- Permission: `data = true` in `mod.toml` (`[permissions]`); a data mod that adds model IDs also declares them
  (`[content]` section: kind, name, base class, files).

## 5. SA:MP as a mod (MOD-2 level 4)

- `samp-rs` is split into a **client core** (protocol, RakNet, sync, UI logic: `samp-proto`, `samp-client`, `samp-ui`,
  `samp-saa`) that only talks to the `sa-samp-api` traits, and **hosts**:
  - `original` (DLL inside the unmodified `gta_sa.exe`, today's route, unchanged);
  - `engine-native` (today's `sa-samp`, linked into sa-game);
  - **`mod`**: the client core compiled to `wasm32` as a mod, reaching the game through the mod API levels 2–4 and the
    network through the `net` permission.
- 1:1 behaviour with 0.3.7-R4 is checked the same way as today (the samp-rs e2e and protocol tests), on all hosts.
- **Gameplay mods on SA:MP:** SA:MP itself is a gameplay mod. Whether *other* gameplay mods may run while connected to a
  server is a policy of the SA:MP mod (for example visual-only by default, as the user leaned earlier), not of the host.

## 6. Determinism and verification

- Without `mods`, nothing changes: same binary content, verify-all IDENTICAL.
- With `mods` and **no mod loaded**, verify-all must still be IDENTICAL (the host adds call points, not behaviour).
- A level-1 mod does not change game state; the corpus replays must stay IDENTICAL with it loaded (only the image
  differs). This is the test that the visual API cannot leak into logic.
- Levels 2+ change game state by design; their mods carry their own tests.

## 7. Plan

| step | content | acceptance |
|---|---|---|
| M0 | decide the ABI (§3.2 open point); `sa-mod-api` level 1 definition; `sa-mod-host` skeleton with wasmtime and the browser backend; `--features mods`; a "hello" mod that draws text | the same `.wasm` draws the text natively and in the browser; verify-all IDENTICAL with and without `mods` |
| M1 | level 1 complete; the **resolution** mod (desktop resolution / any size, window and fullscreen, saved per mod), MSAA, anisotropic filtering, FPS overlay | 1920×1080 natively and in the browser; corpus replays IDENTICAL with the mods loaded |
| MD | data mods (§4.2): the virtual file system overlay with line-merged definition files, content registration with new model IDs, the mods-only limits table | an example mod adds a new car (own model, textures, handling) that spawns and drives natively and in the browser; the parity build and verify-all unchanged |
| M2 | level 2: events, read-only state, before/after hooks on registry functions (done 2026-11-18, §4.1a) | an example mini-map / trainer read-only overlay (`examples/mods/trainer-hud`) |
| M3 | level 3: state write, entities, script commands, audio (done 2026-11-18, §4.1b) | an example gameplay mod (`examples/mods/courier`) |
| M4 | level 4: network, storage; samp-rs client core split from its hosts (done 2026-11-18, §4.1c) | samp-rs builds as a mod (`mods/samp`); the UDP ping example (`examples/mods/net-ping`) |
| M5 | **SA:MP as a mod**: joins a 0.3.7-R4 / open.mp server natively and from the browser | the samp-rs e2e suite passes on the mod host; the DLL host still passes |
| M6 | SDK docs, the `sa-mod` crate, templates, a mod folder and settings UI | an outside developer builds a mod from the docs alone |

## 8. Risks

- **The browser and the component model** (§3.2): avoided by the core-wasm ABI (decided in M0).
- **Call overhead** at mid-frame hooks (SA:MP hooks ped rendering, weapon fire, vehicle control): a host-to-mod call is
  ~15 µs on the i686 build's Pulley interpreter (M0), so a hook on a function called per entity and pass costs real
  time: the per-iteration limit caps it, the trainer HUD's two hooks (~30 calls per step) cost 0.1-0.2 ms per frame
  in total with its overlay and events (M2). Precompiled / JIT engines natively are the way out if SA:MP's hooks need
  it.
- **API stability**: once outside developers depend on it, breaking changes need a major version; keep level 1 small.
- **wasmtime size**: a few MB in the native binary, only with `mods`.
- **Security**: sandboxing covers memory; the permission system covers files and network; the host never gives a mod a
  raw pointer.
