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
| `sa-mod-api` | the interface definition and its Rust types (shared by host and SDK); no `unsafe`, builds for every target | `sa-samp-api` types |
| `sa-mod-host` | loads and runs mods; one implementation, two engine backends (wasmtime / browser) behind one trait (ADR-003) | `sa-mod-api`, `sa-samp`, platform/render layer |
| `sa-mod` (SDK) | what a mod author links: safe Rust bindings, macros for the entry points, examples | `sa-mod-api` |
| `examples/mods/*` | reference mods: `resolution`, `msaa`, `fps-overlay`, later `samp` | `sa-mod` |

`sa-game` gets `--features mods`; without it the host crate is not compiled in.

### 3.2 The interface and the ABI

- One interface definition is the source of truth: **WIT** (WebAssembly Interface Types). Guest bindings come from
  `wit-bindgen`; the host side natively uses wasmtime's component model.
- **Open point (decide in the first step):** browsers do not run components directly. Two options:
  (a) transpile each component with `jco` to core modules + JS glue at build/load time;
  (b) define the ABI as **core wasm** (plain imports/exports, structs in linear memory, a small generated glue on both
  sides), and keep WIT only as documentation/generator input.
  Leaning: (b) for the first version (one ABI, identical natively and on the web, no extra toolchain in the browser),
  revisited when browsers ship the component model.
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
files    = "own"            # none | own (the mod's folder, through the host)
```

The host grants only what the manifest asks for and what the user's mod settings allow. A mod that calls an API it has
no permission for gets an error, not a crash.

### 3.4 Execution model

- The game is single-threaded (`docs/samp/game-adapter.md` §3); mods are called on the game thread, in a fixed order
  (the load order), at fixed points:
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
| M2 | level 2: events, read-only state, before/after hooks on registry functions | an example mini-map / trainer read-only overlay |
| M3 | level 3: state write, entities, script commands, audio | an example gameplay mod |
| M4 | level 4: network, storage; samp-rs client core split from its hosts | samp-rs builds as a mod |
| M5 | **SA:MP as a mod**: joins a 0.3.7-R4 / open.mp server natively and from the browser | the samp-rs e2e suite passes on the mod host; the DLL host still passes |
| M6 | SDK docs, the `sa-mod` crate, templates, a mod folder and settings UI | an outside developer builds a mod from the docs alone |

## 8. Risks

- **The browser and the component model** (§3.2): mitigated by the core-wasm ABI.
- **Call overhead** at mid-frame hooks (SA:MP hooks ped rendering, weapon fire, vehicle control): a wasm call is tens of
  nanoseconds; measured with the PERF-0 instrumentation from M2 on.
- **API stability**: once outside developers depend on it, breaking changes need a major version; keep level 1 small.
- **wasmtime size**: a few MB in the native binary, only with `mods`.
- **Security**: sandboxing covers memory; the permission system covers files and network; the host never gives a mod a
  raw pointer.
