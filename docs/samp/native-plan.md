# SAMP-3: native sa-game joins SA:MP / open.mp servers (plan and first slice)

Status: plan + first working slice (2026-11-12). User priority of 2026-11-12 (ROADMAP decisions): native
Linux sa-game joins SA:MP 0.3.7-R4 / open.mp 1.6.7 servers through samp-rs; then wasm32; browser networking
later through a separate WebSocket/WebTransport ↔ UDP proxy.

Inputs: `docs/samp/game-adapter.md` (API, 12-step migration, open questions), `docs/samp/dynamic.md` +
`catalog/samp-priority.csv` (what SA:MP executes), `docs/arch/platform.md` (native sa-game), samp-rs on branch
`sa-engine/adapter` (worktree `~/samp-rs-adapter`, read only for this agent, see §8).

## 1. Where we are (measured 2026-11-12)

| piece | state | evidence |
|---|---|---|
| native sa-game | boots, runs the front end and `CGame::Initialise`, presents in-world frames at Grove Street (`--start-in-world`, 10 exploration stubs, platform.md §15) | STD-9 |
| SA:MP-executed engine functions | 4,777 non-init functions; **4,471 in Rust (93.6 %), 2,061 of them verified (L1–L3)**, **306 not in Rust** | §4, `tools/native/samp_unported.py` |
| sa-samp-api | 15 traits, API v1 at d666ce0 (script threads, `area_raw`, `menu_active_dword`) | `crates/sa-samp-api` |
| original backend (samp-rs) | steps 1–3 committed on `sa-engine/adapter`; 4, 5 (world, HUD) and the network refactors by SAMP-4 (§10.2) | §10 |
| **native backend `sa-samp`** | **new**: read side served (§3.2), script threads through the ported interpreter, host tests | `crates/sa-samp`, 12 tests (x86_64 and i686) |
| **dispatch points** | **new**: `sa_core::samp::frame` in the `Game::Idle` and `CGame::Process` ports at samp's three hook sites | `crates/sa-core/src/samp.rs` |
| **`sa-game --samp HOST[:PORT]`** | **new skeleton**: installs the native adapter with a stand-in client; frame callbacks run in game | §3.4 |
| **samp-rs networking on Linux** | **works**: the whole samp-rs workspace builds for `i686-unknown-linux-gnu` unchanged; a host test joins our open.mp 1.6.7 copy (cookie → RakNet → auth → PlayerConnect → InitGame) in ~1.06 s | §6 |

The networking core needed no cfg work: samp-proto, samp-ui and samp-saa have no Windows code, and samp-client
already compiles its platform-independent half (netclient, scm, saafs, footsync, lifecycle, ...) on every
host target; only the 32 modules listed in §2 are `cfg(windows)`.

## 2. What samp-rs needs to run inside native sa-game

samp-client is 42,068 lines. About 22,900 are `cfg(windows)`; the rest already builds and is tested on Linux.

| Windows-only part | where (lines) | what it does | native replacement | migration step |
|---|---|---|---|---|
| DLL entry, init thread, `gGameState` wait, frame hooks, control channel | `dll.rs`, `dll/` (1,098) | attach, patch blocks, retargeted calls 0x53E981 / 0x53EB12 / 0x53C104, tick | `SampHost::install` + `GameHooks::frame` from sa-core dispatch points (done in sa-engine, §3.3) | 6 |
| byte patches, call retargets, vtable swaps | `patch.rs` (`ProcessMemory`), `patches.rs`, `vehpatches.rs`, `sectors.rs`, `stores.rs` | samp.dll's 384 patch rows | `SampModeConfig` read by the ports (SAMP-4, §5.2) | 8 |
| mid-frame hooks | `vehhooks.rs` (967), `combathooks*` (1,121), `worldhooks*` (1,594), `mainhooks.rs` (250), `plates.rs` (236), `pedhooks.rs`, `scmops.rs` | 103 hook rows / 157 addresses | `GameHooks` callbacks at dispatch points in the ports (game-adapter.md §5) | 7 |
| per-player context swap + SEH guards | `swap.rs` (553) | save/write/restore of pad, focus, camera, skills around remote players | `GameHooks::player_context`, applied by the ports | 7 |
| client logic that calls the game | `client*` (2,319), `players.rs` (941), `vehicles*` (2,624), `objects*` (1,461), `entities.rs` (834), `behaviourclient*` (1,229), `combatclient.rs` (440) | spawn, sync apply, RPC effects | typed adapter methods (step 5); then the code moves to `samp-core` | 5, 10 |
| D3D9 device proxy and UI renderer | `ui/d3d` (1,299), `ui/render` (1,629), `ui/glue` (1,822), `ui/dxut.rs`, `ui/preview.rs`, `ui/mattext.rs`, `ui/textdraws.rs`, `ui/menus.rs` | chat, dialogs, scoreboard, nametags, textdraws, model previews, material text | `render::Overlay` on sa-rw-wgpu (`Present` phase); samp-ui already holds the renderer-independent logic | 11 |
| window procedure, cursor | `ui/wndproc.rs`, `cursorhook.rs`, `ui/inputlock.rs` | key/char/mouse events, cursor, input lock | `GameHooks::window_event` from sa-platform's event loop; `RuntimeToggle::InputLock` | 6, 8 |
| file redirection | `filehook.rs` (317, kernel32 import slots) | samp.saa, `SAMP/` files | `FileProvider` consulted by sa-platform's `GameFs` (`NativeGame::files`) | 9 |
| audio streams | `audio.rs` BASS part (raw-dylib `bass.dll`) | RPC 41/42 internet radio | Rust decoder (user decision 2026-11-11, Q7) on sa-platform's audio output | 11+ |
| crash handler, diagnostics | `crash.rs`, `diag.rs` | SEH report with "SCM Op" / "lDbg" | panic hook + `ScriptCommands::last_command` | 10 |
| launcher, injection | `samp-launcher` (400) | CreateProcess suspended + LoadLibrary | not needed: samp-rs is linked into sa-game | — |
| paths | `game.rs` / `netclient.rs` (`gta_user_dir_path` 0xC92368, serial input 0xC9236C) | user folder, gpci serial | sa-game's documents folder (Q8, §7) | 9 |
| `Sleep`, `GetAsyncKeyState`, `QueryPerformanceCounter` | scattered `windows_sys` uses (30 files, 16 API groups) | timing and key polls | std / sa-platform (`Input::key_held`) | 5–10 |

What already runs on Linux: the protocol (samp-proto: cipher, RakNet reliability, every RPC/sync codec),
`netclient.rs` (session, join order, reconnect, serial fallback), `samp-saa`, samp-ui's logic, `scm.rs`
(samp's main.scm), the footsync/lifecycle state machines and samp-bot.

### 2.1 Adapter steps 4–12 and what each buys the native path

| step | samp-rs change (pure refactor until 10) | native value |
|---|---|---|
| 4 script threads (**done as a patch**, §8) | `Runner` = `ScriptThreadRef` of the adapter; all 294 `runner.run` sites now go through `ScriptCommands::run_on_thread` | `sa-samp` already runs these threads on the ported interpreter |
| 5 typed methods | world → HUD → streaming → peds → vehicles → objects → entities → camera → audio | each typed method is one `sa-samp` method; raw opcodes can stay where the engine's handler is ported |
| 6 frame callbacks | `game_process_hook`, `camera_process_hook`, `frame_hook`, `frame_after_2d`, `Present` → `GameHooks::frame` | the sa-core dispatch points call them |
| 7 mid-frame hooks | per module, typed sites | the ports call `GameHooks` at the same sites (needs one dispatch point per callback, §5.3) |
| 8 patches → config | `SampHost::install(config)` | ports read `SampModeConfig` (SAMP-4) |
| 9 files | `FileProvider` | `GameFs` overlay |
| 10 split | `samp-core` without `cfg(windows)` | **linkable into sa-game** |
| 11 overlay | `Overlay` UI renderer next to D3D | native UI |
| 12 engine backends | — | `sa-samp` complete |

Steps 4–9 can proceed without the engine; 10 is the point from which sa-game links the client. Until then the
skeleton (§3.4) stands in for the client.

## 3. How sa-game hosts the client

### 3.1 Process model
One 32-bit Linux process (i686; wasm32 later with the same layouts in linear memory). samp-rs is an rlib
dependency of sa-game (behind a cargo feature once `samp-core` exists). The game thread runs WinMain's loop as
today; samp-rs's network session keeps its own RakNet update thread (`netclient.rs`, 2 ms), exchanging messages
with the client tick exactly as in the DLL. Every adapter call stays on the game thread inside a `GameHooks`
callback (game-adapter.md §3).

### 3.2 The native backend `sa-samp` (this slice)
`crates/sa-samp`: `NativeGame<V: View>` implements `GameAdapter` + `SampHost`.
* **State access**: the ports keep the original 32-bit layouts (ADR-001); native sa-game maps the legacy arena
  at the original addresses and the pools hold objects with the original field offsets, so the backend reads
  them as the ports do (`GPtr::from_addr`, through `EngineView`). Tests use a fake memory.
* **Handles**: `PedRef`/`VehicleRef`/`ObjectRef` are the game's own pool references (`slot << 8 | counter`, the
  SCM handle). Every object read resolves the handle through the pool's byte map first, so a stale handle is
  `InvalidHandle`, never a wild read. `has_original_layout` is false (no original code).
* **Served**: game state, time step, game time, quit request; local player, player slots 0–1, focus, ped and
  vehicle and object handle ↔ script handle, ped liveness (samp's test), ped model test, `ped_snapshot`
  (matrix or placement, speed, health, armour, headings, state, model, weapon slot/type, vehicle, area, flags),
  `ped_weapons`, money; `vehicle_snapshot` (class from the vtable, matrix, speeds, health, model, driver,
  passengers, colours, siren, area, train speed), `local_vehicle`; object matrix / RW object; clock, weather,
  gravity, area; pads (new/old), game keys, mouse, screen size; menu byte/dword; full camera state;
  `SampHost::install` (config, hooks, file provider), frame dispatch, shutdown.
* **Script threads**: same mechanism as samp's ScriptCommand (private zeroed `CRunningScript`, IP at one
  encoded instruction) run through `orig!` → `CRunningScript::ProcessOneCommand` 0x469EB0 → the sa-script port.
  An unported opcode handler panics with its name (phase B gap report).
* **Not yet**: every write/typed command (`Unsupported`), bones, anim queries, damage state, streaming queries,
  `Overlay`, `start_game`, runtime toggles.

### 3.3 Dispatch points (this slice)
`sa_core::samp` holds one registered `fn(u8)` handler (a relaxed load when none is registered, so single-player
behaviour and phase A are unchanged). The ports call it where samp hooks the original:

| `FramePhase` | port and site | samp's original hook |
|---|---|---|
| `BeforeGameProcess` | `Game::Idle` (0x53E920), before `CGame::Process` | call 0x53E981 |
| `CameraProcess` / `AfterGameProcess` | `CGame::Process` (0x53BEE0), around `TheCamera.Process()` | call 0x53C104 |
| `Before2d` / `After2d` | `Game::Idle`, around `Render2dStuff` | call 0x53EB12 |
| `Present` | not yet: sa-game's present path (`d3d9.rs` `Present`) | D3D9 proxy |

Mid-frame callbacks need the same one-line dispatch in each port of game-adapter.md §5 (vehicle
`ProcessControl` ×10, `CWeapon::Fire`, `ComputeDamageResponse`, ...). They are added port by port when samp-rs
step 7 moves the matching hook; until then the port's single-player path is what runs.

### 3.4 `sa-game --samp HOST[:PORT] [--name NAME]` (this slice)
`crates/sa-game/src/samp.rs`: builds `sa_samp::engine()`, installs a stand-in `GameHooks` (counts phases, logs
what on-foot sync would read once per second of game time), registers the dispatch handler, and adds a report
section. No socket yet. Run (release, i686, `--start-in-world --frames 120`, STD-9's stubs):

```
sa-game: samp mode (skeleton): server 127.0.0.1:7810 as NativeRust; adapter Some(EngineNative); stand-in client (no network yet)
samp: t=532 ms state=Playing player=0x1 pos=(2488.56,-1666.86,13.61) health=100 armour=0 model=0 weapon=0 live=true clock=08:00 weather=(-1, 1, 1) area=0
samp: frame dispatches ["BeforeGameProcess=48", "CameraProcess=48", "AfterGameProcess=48", "Before2d=4", "After2d=4", "Present=0"]
```

(CJ at main.scm's `CREATE_PLAYER` position, read natively through the adapter.) Runs with and without `--samp`
vary between 94 and 110 "ok" steps run to run in both modes (real-time frame pacing decides how many world frames
reach `Render2dStuff` before the `call_render_cb` 0x7331E0 gap); no difference attributable to the option.

Next for the host, in order: (1) link samp-client's host build and drive `NetClient` from the stand-in (join +
spawn; on-foot sync from `ped_snapshot`/`pad`); (2) `start_game` (samp's menu-flag writes and `gGameState := 8`
become a sa-game start-up option instead of `--start-in-world`); (3) samp's `main.scm` / `script.img` from
samp.saa through `FileProvider`; (4) the custom opcodes 09C6/09D3 in sa-script's 2500–2599 switch; (5) the
`Present` dispatch and `Overlay` on sa-rw-wgpu; (6) window events from sa-platform.

### 3.5 samp.dll's patches → SAMP-4 "samp mode"
`SampModeConfig` (game-adapter.md §6) is stored by `install`; no port reads it yet. Order to wire it, by what a
join needs first:
1. `StartupOptions`: skip logos/intro/menu, frame limit, samp's main.scm, image list (a start-up path, not a
   per-function flag).
2. `Behaviour::remote_player_peds` and `Limits::player_slots` (210): without them remote `CPlayerPed`s index
   `CWorld::Players` with −1. Natively the array is just sized from the limit (no sector-grid relocation).
3. `Limits` pool sizes, model stores, colour palette: read once in `CPools::Initialise`/`CModelInfo` ports.
4. `FeatureSet` (46): one `if !config.disabled(f)` at each site samp NOPs; SAMP-1D confirmed none of those paths
   runs under SA:MP, so they cannot regress a join.
5. `RuntimeToggle` (input lock, clock, camera fade, clothes rebuild, manual lights, ...): `SampMode::set_toggle`.

## 4. Coverage of `catalog/samp-priority.csv` on master (57f2a99)

`tools/native/samp_unported.py` (inputs: samp-priority.csv, state.csv, `#[hook]`/`#[native]` registrations) →
`docs/samp/native-unported.csv` (one row per SA:MP-executed function not in Rust; ordered by subsystem, then by
SAMP-1D hit count). "In Rust" = state implemented/L1/L2/L3 or a registration, as `mvp_gaps.py` counts.

| subsystem | executed (non-init) | in Rust | of which verified (L1–L3) | **not in Rust** | of which specified |
|---|---:|---:|---:|---:|---:|
| AI | 761 | 684 | 303 | **77** | 72 |
| WORLD | 338 | 276 | 85 | **62** | 62 |
| UNK-game | 417 | 371 | 103 | **46** | 10 |
| VEH | 288 | 249 | 131 | **39** | 39 |
| RW | 820 | 790 | 279 | **30** | 23 |
| RENDER | 492 | 467 | 180 | **25** | 24 |
| SCRIPT | 103 | 97 | 9 | **6** | 6 |
| CALLED-ONLY | 23 | 18 | 4 | **5** | 2 |
| PED | 186 | 181 | 118 | **5** | 4 |
| WEAPON | 67 | 63 | 52 | **4** | 3 |
| AUDIO | 452 | 449 | 274 | **3** | 3 |
| PHYS | 22 | 20 | 2 | **2** | 2 |
| ANIM | 127 | 126 | 94 | **1** | 1 |
| CORE | 50 | 49 | 9 | **1** | 1 |
| CAM, COLL, HUD, INPUT, MATH, POP, SAVE, STREAM | 630 | 630 | 418 | 0 | 0 |
| **total** | **4,777** | **4,471** | **2,061** | **306** | **252** |

Static initialisers (role `init`): 1,649 executed, 1,244 in Rust (phase B takes the rest from the exe's
`.data`, platform.md §7). `CRunningScript::ProcessOneCommand` (0x469EB0, missing from the CSV because SAMP-1D's
counter detoured it) is implemented.

The 306 include the in-world blockers STD-9 stubs today: `_rpD3D9SkinAllInOneNode` 0x7C7B90,
`_rwD3D9ImmRenderPrimitiveNode` 0x80E270, `RenderWeaponPedsForPC` 0x732F30, the vehicle render callbacks
0x7331E0 / 0x733240 / 0x733F80, the car env-map callback 0x5D9900, `CPickups::DoPickUpEffects` 0x455720,
`CPostEffects::HeatHazeFX` 0x701780. 54 rows are `unknown` (unnamed, no spec yet).

**Verification debt is the bigger number**: 2,410 SA:MP-executed functions are in Rust but unverified. For the
native join they matter less than the 306 gaps (a gap stops the frame; an unverified port at worst diverges), but
the definition of done needs them.

### 4.1 Writer queue (proposal)
1. The 306 rows, subsystem by subsystem in table order, render callbacks first (they stop in-world frames).
   Batches ≤ 60, as the writer gate requires.
2. The dispatch points of game-adapter.md §5 in the ports that own them (one line each, plus the `PlayerContext`
   apply/restore in the three player ports and the ten vehicle `ProcessControl`s).
3. The SAMP-4 reads of §3.5 (items 1–3 first).
4. L2 for the unverified SA:MP-executed ports, highest SAMP-1D hit counts first.

## 5. Native backend: what comes next (order)

1. `start_game` + the samp start-up path (§3.5 item 1) so `--samp` reaches `Playing` the way samp does.
2. Writes needed for spawn and on-foot sync: `create_player_ped` (opcode 0053 / 01F5 through script threads),
   `set_ped_*` setters, `give_weapon`, `set_player_control`; `set_clock`/`set_weather`/`set_gravity`/`set_area`.
3. Vehicles: `create_vehicle`/`destroy_vehicle` (00A5/00A6), setters, damage state.
4. `Present` + `Overlay` (chat first), `window_event`.
5. Remote players: `Limits::player_slots`, `PlayerContext` in the ports.
6. The remaining typed methods in samp-rs step 5's order.

## 6. Networking on Linux (done, this slice)

* `cargo build --workspace --target i686-unknown-linux-gnu` of samp-rs (step-4 tree): clean, no source change.
  `cargo test -p samp-client --target i686-unknown-linux-gnu`: 197 + 1 passed.
* `tools/e2e/samp/native-handshake.sh SAMP_RS_DIR [AUTH_TABLE]`: copies `re/samp/server` (open.mp 1.6.7) to
  `target/samp-native/server`, binds 127.0.0.1 on UDP 7812 (samp-rs uses 7777/7790, samp-cov 7810), takes
  `target/locks/samp-native.lock`, and runs samp-rs's new `tests/native_handshake.rs` (32-bit Linux):

```
 1002 ms  Connecting
packet Accepted
 1042 ms  Joining
 1057 ms  Connected
InitGame: player id Some(0), ... server_name: "open.mp server", world_time: 12, weather: 10, gravity: 0.008 ...
test joins_a_server_from_linux ... ok
[connection] incoming connection: 127.0.0.1:37513 id: 0
[join] NativeLinux has joined the server (0:127.0.0.1)
[part] NativeLinux has left the server (0:1)
```

  (The first second is samp's open-connection retry interval: the first request goes out with the first update.)
* The auth key (`0x0c`) is answered from the 256-entry table (`data/auth_table.txt`, third-party data, never
  committed), not from game code, so the native path needs nothing from 0x4D3AA0 for the handshake (game-adapter
  Q3: answered for the handshake).
* Outside the game the client serial is random (`netclient.rs` falls back when 0xC9236C cannot be read); the
  native host must feed it the documents-path string (Q8, §7).

## 7. Open questions (answered 2026-11-12, see §10.1)

1. **samp-rs worktree access.** This agent could not write or run git in `~/samp-rs-adapter` (the harness
   confines worktree agents to their own worktree). Step 4 and the handshake test are therefore a patch
   (`docs/samp/patches/samp-rs-adapter-step4.patch`, applies to `sa-engine/adapter` 7422107 with `git apply`);
   the main session should apply and commit it there, then delete the patch file. Future samp-rs steps need a
   session that may write there.
2. **A/B join for step 4** was not run (no lane taken; the change is mechanical: same encoding, same
   `ProcessOneCommand` call, same fault handling, the thread buffer now lives in a table). Run SAMP-2's lane-7
   A/B (join + class + spawn, RPC/event sequence) before committing.
3. **API: `destroy_script_thread`.** samp-rs creates short-lived `Runner`s (train warp, `/save`, first-frame
   flags); the original backend frees them through an inherent `release_script_thread`. Add
   `ScriptCommands::destroy_script_thread` (default no-op) at the next API bump so every backend can free threads.
4. **API: `PadState::home`** ("home button / ped walk") is ambiguous: `CControllerState` has `m_bChatIndicated`
   (+0x28) and `m_bPedWalk` (+0x2A). `sa-samp` returns `m_bPedWalk`; rename the field or split it.
5. **gpci serial natively (Q8).** Proposal: sa-game passes the same string shape the original builds
   (`<documents>\GTA San Andreas User Files` from its 5th character) from its documents folder, so one user keeps
   one serial; or the user picks a fixed serial in a config file.
6. **`--samp` start path.** Keep `--start-in-world` as the interim way into the world, or wait for samp's own
   start-up (§3.5 item 1) before the first network join from sa-game?
7. **Where samp-rs lives in sa-game's build.** A cargo git dependency on samp-rs (pinned rev, like the reverse
   dependency) or a path dependency on a local checkout behind a feature (`--features samp`)? The first keeps
   sa-engine buildable alone.

## 8. Deliverables of this slice

* sa-engine (this branch): `crates/sa-samp` (native backend + 12 tests), `crates/sa-core/src/samp.rs` (dispatch
  points, 1 test) + the five calls in `mvpb_app.rs`, `crates/sa-game/src/samp.rs` + `--samp`/`--name`,
  `tools/native/samp_unported.py` → `docs/samp/native-unported.csv`, `tools/e2e/samp/native-handshake.sh`, this
  file, `docs/samp/patches/samp-rs-adapter-step4.patch`.
* samp-rs step 4 (patch): `sa-samp-api` pin b5d41ce → d666ce0; original backend `ScriptCommands` (thread table,
  `run_on_thread` with samp's SEH/fault path, locals, `last_command`), `area_raw`/`menu_active_dword` (unrouted
  neutral values like their siblings); `script::Runner` is now a thin owner of an adapter thread (all 294 call
  sites unchanged); `script::encode` takes `ScriptArg`; `tests/native_handshake.rs`. Verified on the copy:
  `cargo test --workspace` (all pass, +1), clippy `-D warnings` host and i686-pc-windows-gnu, release DLL build,
  rustfmt on the touched files.

## 9. Proposed ROADMAP rows (for the main session)

| ID | Task | Status | Acceptance | Notes |
|---|---|---|---|---|
| SAMP-3 | Native backend `sa-samp` + sa-game hosting | in-progress | sa-game `--samp` joins a local open.mp server, spawns and syncs on foot | 2026-11-12: plan docs/samp/native-plan.md; `sa-samp` read side + script threads (12 tests, clippy host/i686/wasm32); sa-core dispatch points in `Idle`/`CGame::Process`; `sa-game --samp` skeleton runs frame callbacks in game and reads CJ natively; samp-rs networking builds for i686 Linux unchanged and joins our open.mp copy (`tools/e2e/samp/native-handshake.sh`). Next: link the client's host build into `--samp`, `start_game`, spawn writes |
| SAMP-2 (update) | samp-rs original backend | in-progress | as before | step 4 (script threads) prepared as `docs/samp/patches/samp-rs-adapter-step4.patch`: tests, clippy host+i686, release DLL OK; lane A/B join still to run before committing on `sa-engine/adapter` |
| SAMP-GAPS | Port the 306 SA:MP-executed functions not in Rust (`docs/samp/native-unported.csv`, `tools/native/samp_unported.py`) | todo | 0 rows | render callbacks first (STD-9 stubs), then AI 77, WORLD 62, UNK-game 46, VEH 39, RW 30, RENDER 25 |
| SAMP-4 (update) | samp mode | todo | F13 | order in native-plan.md §3.5 (start-up path, remote player slots, limits, features, toggles) |

## 10. SAMP-4 (2026-11-12): native join, samp mode, adapter steps 4–5, browser-ready networking

### 10.1 Main-session decisions on §7
1. **A/B before step 4's commit**: done (§10.3), on lane 3 with the original exe.
2. **`ScriptCommands::destroy_script_thread`** added (default: keep the thread) and **`PadState::home` split** into
   `chat_indicated` (+0x28) and `ped_walk` (+0x2A): sa-samp-api v2 (7fd398c). v3 (b4cf80c) adds
   `WorldState::force_weather` (samp writes only `ForcedWeatherType` while the game clock runs).
3. **gpci serial natively**: a random 40-hex-digit seed made once per install and kept in sa-game's configuration
   (`$XDG_CONFIG_HOME/sa-engine/samp.cfg`, `serial_seed=`), never derived from user or host data;
   `NetClient::set_serial_seed` (samp-rs e706d17) turns it into the serial with `session::make_serial`.
4. **`--start-in-world` stays the interim way into the world** for `--samp`.
5. **samp-rs is a git dependency of sa-game pinned to a commit of `sa-engine/adapter`, feature `samp`**
   (`cargo build -p sa-game --features samp`). samp-client takes sa-samp-api and sa-net from this repository by URL;
   the workspace `[patch."file:///home/ghosty2004/sa-engine"]` points both back at the workspace crates.

### 10.2 samp-rs commits (branch `sa-engine/adapter`)
This agent could not run version control in `~/samp-rs-adapter` (worktree isolation, as SAMP-3), so it worked in a
clone of the branch at `target/samp-rs-adapter` inside its sa-engine worktree. **The main session must fetch them
from that clone before the worktree is removed** and fast-forward `sa-engine/adapter` (7422107 → the last commit below). Every
commit: `cargo test --workspace` (632 → 638 passed), clippy `-D warnings` host and i686-pc-windows-gnu, release DLL
build, rustfmt on the touched files, and the A/B join of §10.3 identical.

| commit | step | content |
|---|---|---|
| d54b1f4 | 4 | script commands through adapter script threads (SAMP-3's patch) + `destroy_script_thread`; pin v2 |
| c9af579 | 5 (world) | clock (direct bytes + `SET_TIME_OF_DAY`), weather (`set_weather` / `force_weather`), gravity, `SET_AREA_VISIBLE` through `WorldState`; the backend runs its opcodes on its own script thread; pin v3 |
| 3c27423 | 5 (HUD) | HUD switch, `menu_active` (byte) / `menu_active_dword`, `DISPLAY_ZONE_NAMES`, `ADD_SCORE`, `CLEAR_PRINTS`, `AddBigMessage` through `Hud` (the UI's RPC 73 game text stays in the UI until step 11) |
| e706d17 | — | `NetClient::set_serial_seed` (unused by the DLL) |
| 9e74177 | web (a) | `web_time::Instant` in samp-proto / samp-client (on native it *is* `std::time::Instant`); samp-proto builds for wasm32 |
| 1ef8e5d | web (b) | `Core.sock` → boxed `sa_net::DatagramTransport` from a `NetConnector` per attempt; `UdpNetConnector` = sa-net's `UdpTransport` (same socket calls); a transport closed while opening fails the attempt at once |
| 4908e11 | web (c) | `NetClient::new_unresolved("host:port")` (cipher key from the real port); wasm32: `WebSocketNetConnector { proxy_url }`, no update thread (`tick` updates), `rak_wait` no-op; samp-client clippy-clean for wasm32 |
| 3d25201 | 5 (streaming) | `REQUEST_MODEL`, `HAS_MODEL_LOADED`, `MARK_MODEL_AS_NO_LONGER_NEEDED`, `LOAD_ALL_MODELS_NOW`, `REQUEST_COLLISION`, vehicle-mod request/loaded through `Streaming`; the `&self` queries run on the process-wide backend thread (`backend_op`) |
| d8ea167 | 5 (camera) | `SET_FIXED_CAMERA_POSITION`, `POINT_CAMERA_AT_POINT` (mode 2/1), `SET_CAMERA_BEHIND_PLAYER`, `RESTORE_CAMERA_JUMPCUT` through `Camera` |
| 45fe143 | 5 (peds, part 1) | current weapon, critical hits, drops weapons, freeze, ped area, heading, player control, drunkenness through `Peds`; `ped_from_script_handle` (`CPool::GetAtRef`) |
| 538e6c2 | 5 (vehicles, part 1) | delete, heading, hydraulics, tyres, petrol tank, area through `Vehicles`; `vehicle_script_handle` / `vehicle_from_script_handle` with GTA's pool `GetRef` / `GetAt` |
| 78be658 | 5 (world entities) | `REMOVE_PICKUP`, `DELETE_CHECKPOINT`, `REMOVE_BLIP`, `CHANGE_BLIP_COLOUR`, `CHANGE_BLIP_SCALE` through `WorldEntities` |
| fb89010 | 5 (objects, part 1) | object visible / scale / attached / delete / remove elegantly through `Objects`; object pool handles |

Sites holding a script handle go through small `calls` modules (`adapter::original::{stream, cam, ped_calls,
veh_calls, world_calls, object_calls}`) that resolve the handle to the typed reference; the backend turns it back
into the same handle, so the opcode bytes are unchanged (a handle that names nothing issues no opcode, where the
old code would have faulted inside the opcode). Arguments the typed API cannot carry unchanged stay on the
`Runner` for now (`SWITCH_WIDESCREEN`'s raw byte, the mod-shop interior word, paintjobs from SCM events, the
remote players' `Arg` forms). Not done: step 5's second parts (creation, tasks, anims, proofs, money, trains,
trailers, blip creation, camera vector moves, audio), steps 6–12.

### 10.3 A/B join (original exe + samp-rs DLL, lane 3)
`tools/e2e/samp/samp-rs-ab.sh DLL_DIR NAME` (new): our open.mp copy on 127.0.0.1:7813 with `sampcov.pwn`, the
original `gta_sa.exe` with samp-rs's launcher and DLL (no samp.dll); scenario: connect, class selection (3
classes), spawn, walk, message, game text, position, weapon, money, clock and weather (night/day), vehicle create +
put in + drive + exit + delete, interior 17 and back, kill, respawn, `/q`. `sequence.txt` = the server's `[cov]`
events + game-state probes (weather, clock, `CClock::Update` byte, HUD clock format, gravity, area, money through
the DLL's `peek`) + the client log without timestamps, addresses and tick lines. Results: A vs A (base 7422107
twice) identical; every step identical to the build before it (328–329 lines). One line (`gGameState = 9`, written
by the init thread) races with the game thread's log and moved with lane load (reproduced on a rerun of the *old*
build); it is now compared as a set. The native handshake test (`tools/e2e/samp/native-handshake.sh`) passes after
the transport steps (join in ~1.1 s).

### 10.4 Native join (sa-game `--samp`, feature `samp`)
`crates/sa-game/src/samp_net.rs`: a minimal client on samp-rs's `NetClient` and `footsync` until samp-core is
linkable (step 10): world set-up like samp's own main.scm (densities 0, wanted level 0, roads off, control off),
join, class 0, spawn (samp's routine reduced to what the adapter serves), on-foot sync (207) from `ped_snapshot` +
`pad` when due, stats sync (205) every second, and the player RPCs applied through the adapter (position, facing,
health, armour, weapons, money, clock, weather, interior, control, camera). `tools/e2e/samp/native-join.sh`
(lock `target/locks/samp-native.lock`, UDP 7812): server copy + `sampcov.pwn`, sa-game `--start-in-world --samp`,
server commands after the spawn (including a server vehicle, streamed in and created natively with `CREATE_CAR`),
then W held (`SA_GAME_WALK`). Result (`target/samp4-evidence/` and
`target/samp4-walk/`, not committed):

```
[join] NativeCJ has joined the server (0:127.0.0.1)
[cov] class id=0 class=0 / state 0->8 / spawn id=0 / state 8->1
[cov] where st=1 pos=2495.00,-1687.00,13.51 hp=100 weapon=31        (on-foot sync from native sa-game)
hp|60 armour|40 money|5000 weapon|24|100 time|22 weather|4 pos|2495|-1680 pos|2440|-1660 time|12 ...
[cov] where st=1 pos=2495.00,-1680.33,13.33 hp=60 weapon=24         (SetPlayerPos applied, CJ fell to the ground)
samp: vehicle 5 (model 560) streamed in at (2445.0,-1650.0,13.4) as 0x1
[cov] where st=1 pos=2434.03,-1660.00 ... 2367.15,-1660.00          (W held: CJ walks down Grove Street)
sa-game: 1418 ok, 198 blocked, 0 crashed over 1616 iterations
```

The PNGs (`present/`, every 20th in-game frame) show Grove Street with the server's clock (22:04), money
($5000), weapon and lowered health, and — through SAMP-UI's engine-drawn overlay (merged here, 45366aa) — the
server's chat lines (`SendClientMessage`, yellow and white) and its game text ("NATIVE SA-GAME"). Chat input and
dialog answers go back as RPC 101 / 50 / 62 (`SampUi::poll_events`; not exercised offscreen). CJ himself is not
drawn (STD-9's skin-pipeline stub).

Engine work the join needed (phase B only unless stated):
* `CTaskSimpleInAir::ProcessPed` 0x680600 registered `#[native]` (ported and fixed by PORT-AI-4, unregistered in
  phase A until its L3 rerun): a spawn sets the ped 0.5 m above the point.
* `CPad` 0x540E80 / 0x540F80 (on-foot camera stick readers) ported from the bytes, `#[native]`.
* `CVisibilityPlugins::RenderWeaponCB` 0x733670 ported (`#[hook]`, unverified `samp4-render-1`): the weapon in
  CJ's hand; without it the loop stopped on three identical blocks.
* sa-game: after an unwound step the `FxManager_c` RwMatrix stack (g_fxMan +0x88) is reset; unwinds inside
  `CreateParticles` (unported `FxInfoAttractPt_c::GetValue` 0x4A56E0) leaked entries until a garbage matrix
  faulted (`SIGSEGV at 0x25b9c`).
* sa-game: `SA_GAME_WALK` never sent its key-down (`key_down` after `held.insert` saw the key as already held);
  fixed, repeated every 8 iterations.
* `CVehicle::CanPedOpenLocks` 0x6D1E20 registered `#[native]` (ported, not hookable in phase A).
* sa-samp: the client's hooks go back in place when an adapter call unwinds (an unported engine function), so
  one gap no longer drops the client for the rest of the run.
* Evidence-only stubs added to STD-9's list in `native-join.sh`: `RenderVehicleReallyLowDetailCB` 0x7331E0 and the
  collision sparks `dmgDrawCarCollidingParticles` 0x6A6DC0 (visual only).
* `WorldState::set_gravity`: `GAME_GRAVITY` is in `.rdata`, read-only in the native arena; the default value is
  accepted, others return `Unsupported` (samp unprotects the page; natively an engine gravity setting is needed).

### 10.5 sa-samp write side
Peds (position 00A1, heading 0173 or the raw fields, health/armour fields, weapons 01B2/01B9/048F, area 0860,
freeze 04D7, control 01B4, money fields, drunkenness 052C, vehicle warps 036A/0430/0362), world (clock, weather,
gravity, area 04BB, wanted 01F0, densities 03DE/01EB, roads 01E8, trains 06D7, restart 016C, pickups 0213/0215),
HUD (flags, 09BA, 0109, 00BE, `AddBigMessage` through `orig!`), camera (015F, 0160, 0373, 02EB, 02A3), streaming
(0247, 0249, 038B, 04E4, load state from `ms_aInfoForModel`), vehicles (00A5 + 0175 + 0229 + 0840, 00A6, heading,
health field, colours, area), objects (0107, 0108, 01BC + 0453). Opcode-backed methods run on the backend's own
script thread through the ported interpreter (the same opcodes samp issues); host tests check the encoded
commands, fields and created handles (16 tests). Not yet: remote players (`Limits::player_slots`), skins
(`set_ped_model`), animations, checkpoints, 3D text, object attach/material.

### 10.6 samp mode (SAMP-4)
`sa_core::samp` holds the disabled `Feature` bits and runtime toggles (single-player when 0); `sa-samp`'s
`install` hands `SampModeConfig::disabled` to it, sa-game `--samp` disables every feature samp does and turns typed
cheats off. Wired sites (the call samp NOPs, skipped when the bit is set): `CGame::Process` cutscenes 0x53BF28,
cheats (`DoCheats`, whole poll), car generators 0x53C06A, replay 0x53C090, sun/moon coronas 0x53C136, plants
0x53C159, random traffic 0x53C1C1; `CPopulation::AddPed` returns null; `GenerateRandomCars`' planes 0x434272;
`CHud::SetHelpMessage`; `CPlaceName::Process` 0x5720A5; idle camera 0x524582. Peds, traffic and wanted level also
go to zero through samp's own script commands (densities, wanted level, roads), which keeps `AddToPopulation` from
dereferencing the null ped (as under samp.dll). Missions: the game's main.scm still runs (`--start-in-world`);
samp's main.scm from samp.saa needs `FileProvider` (step 9) and the start-up path.

### 10.7 Open issues
1. Interactive play: `sa-game --window` keeps running game frames with the window's input after the `--frames`
   loop (input through SAMP-UI's `samp_input`), but the 32-bit process cannot open a window on this machine:
   winit fails with "Failed to load one of xlib's shared libraries" because the 32-bit libraries
   `libXcursor.so.1`, `libXrandr.so.2`, `libXi.so.6`, `libxkbcommon.so.0`, `libxkbcommon-x11.so.0` are missing
   from `/usr/lib32` (Arch: `lib32-libxcursor lib32-libxrandr lib32-libxi lib32-libxkbcommon
   lib32-libxkbcommon-x11`; a user decision, not installed). Until then the evidence runs are offscreen and driven
   by `SA_GAME_WALK` and the server (the join itself works in that run: `WindowCJ` joined and spawned).
2. Gaps hit in the join (SAMP-GAPS): `FxInfoAttractPt_c::GetValue` 0x4A56E0, `FUN_00678E80` (ped in-air,
   PreRender), `CAEWeatherAudioEntity::AddAudioEvent` 0x506800 (weather 8 thunder), `CMovingFog::Update` 0x716B10
   (weather 9), `CAEWeaponAudioEntity::PlayGunSounds` 0x503CE0, 0x449D10 (garages); CJ's skin pipeline 0x7C7B90.
3. The pinned URLs (`file:///home/ghosty2004/sa-engine` in samp-rs; the clone path in `crates/sa-game/Cargo.toml`)
   resolve only on this machine; after the fetch of §10.2 the sa-game dependency URL must be switched to
   `file:///home/ghosty2004/samp-rs` (same rev) and `cargo update -p samp-client`.
4. `regconv::hook_tests::x86` (6 tests) fail on i686-unknown-linux-gnu: the phase A naked entries read their
   arguments one stack word off under the test harness's `call_regs` on Linux (`al_and_stack_words` sees
   `(120, 0x401000, …)` for `(0x41, 0x12345678, 0x401000)`); the entries are phase A (Windows) only and native
   sa-game never calls them, so they are not in the native path's way. Not fixed.
5. The browser join: samp-client is wasm32-ready (§10.2); it needs WEB-1's page and a `WebSocketNetConnector` with
   the proxy URL.
6. Driving natively: `PutPlayerInVehicle` → `WARP_CHAR_INTO_CAR` reaches a task with a null vtable
   (`v_get_task_type (0x0)`) on the following frames, so the player is placed in the seat but the game does not
   treat him as the driver (`native-join.sh` with `PUTIN=1`). The client's driver sync (200) is written and runs
   once the ped reports its vehicle.

### 10.8 Proposed ROADMAP rows
| ID | Task | Status | Acceptance | Notes |
|---|---|---|---|---|
| SAMP-2 (update) | samp-rs original backend | in-progress | as before | 2026-11-12 (SAMP-4): step 4 and step 5's first parts (world, HUD, streaming, camera, peds, vehicles, world entities, objects) on `sa-engine/adapter` (native-plan §10.2), each with tests/clippy/DLL and the lane-3 A/B join (`tools/e2e/samp/samp-rs-ab.sh`) identical; sa-samp-api v2/v3. Next: step 5 second parts (creation, tasks, anims, audio), then step 6 |
| SAMP-3 (update) | native backend + hosting | in-progress | sa-game `--samp` joins, spawns and syncs on foot | 2026-11-12 (SAMP-4): **met for a local open.mp**: join, class, spawn, server-driven position/health/armour/weapon/money/clock/weather, on-foot sync of a W walk (`tools/e2e/samp/native-join.sh`); sa-samp write side; client = `sa-game/src/samp_net.rs` on samp-rs's NetClient until step 10 |
| SAMP-4 (update) | samp mode | in-progress | F13 | `sa_core::samp` feature bits + toggles; 14 sites wired (native-plan §10.6); limits, behaviour and start-up not read yet |
| SAMP-WEB-NET | samp-rs networking for wasm32 | done (samp-rs side) | native identical, wasm32 builds | 9e74177, 1ef8e5d, 4908e11: web-time clock, sa-net transports, unresolved host:port, no thread or waits on wasm32; A/B join and native handshake unchanged |
| SAMP-GAPS (update) | | | | add 0x4A56E0, 0x678E80, 0x506800, 0x716B10, 0x503CE0 (hit by the native join) |
