# Master plan: rebuilding GTA San Andreas (PC 1.0 US) 1:1 in Rust

This is the end-to-end plan Claude follows autonomously. `docs/ROADMAP.md` tracks the live status of every
item. `docs/WORKLOG.md` records what happened. `docs/FEASIBILITY.md` is the background study (its §4 oracle
design and §5 crate design are still valid; its strategy and roadmap are superseded by this file).

User decisions (2026-11-05): gta-reversed may be used as a code source. Scope is the full single-player game.
The target binary is `gta_sa.exe` 1.0 US. `~/samp-rs` is never written to. See CLAUDE.md.

---

## 1. Definition of done ("1:1")

The project is finished when every criterion below holds. Each is proven by a script that exits 0, with its
output recorded in the WORKLOG. "Original" always means `re/gta/1.0-US/gta_sa.exe` (sha256 `a559aa77…cbd26`)
with the user's game data under Wine.

| # | Criterion | Proof |
|---|---|---|
| F1 | **Function coverage.** Every function in `catalog/procedures.csv` that is game code (not EH funclet, CRT, protection junk or a replaced third-party library, each exclusion justified in the catalogue) has a Rust implementation in state ≥ `L2` (see §5). | `tools/accept/f1-functions.sh`: catalogue state report, 0 rows below L2 |
| F2 | **Zero original code at runtime (phase A end).** With every hook set to Rust, a full corpus run executes no instruction of the original game `.text` other than hook entry stubs. | `tools/accept/f2-no-original.sh`: execution coverage of the original code = only stubs |
| F3 | **Lockstep corpus, phase A.** Every recording in the corpus (§6) replays with per-frame state hashes bit-identical to the original's for its full length. | `tools/accept/f3-lockstep.sh` |
| F4 | **Missions.** Every mission and side activity of `main.scm` (story missions, odd jobs, schools, races, collectibles, gang wars, property, girlfriends, casino games) has a recorded playthrough that replays bit-exact to the mission-passed state. | `tools/accept/f4-missions.sh`: per-mission table |
| F5 | **Save compatibility.** Saves are byte-identical for the same game state. Original saves load in Rust and Rust saves load in the original, and the state hashes after loading are identical. | `tools/accept/f5-saves.sh` |
| F6 | **Formats.** Every file of the install parses. Re-serialisable formats round-trip byte-exactly. Every load-time table (model infos, handling, timecyc, paths, pedstats, weapon data, surfaces…) is field-identical to the original's in-memory copy after loading. | `tools/accept/f6-formats.sh` |
| F7 | **Presentation, phase A.** The D3D9 call-stream semantic diff (draw calls, states, geometry, textures, constants) is empty for the screenshot corpus (§6.3), and the audio event log diff is empty for the corpus. | `tools/accept/f7-present.sh` |
| F8 | **Behavioural coverage.** The corpus executes ≥ 90% of the basic blocks of the original's game code (excluding F1's exclusions). Every block not executed is listed with a reason (dead code, debug-only, or not reachable on PC) and reviewed. | `tools/accept/f8-coverage.sh` |
| F9 | **Harness integrity.** Nightly mutation testing: ≥ 95% of injected faults in Rust functions are caught by L1–L3. | `tools/accept/f9-mutation.sh` |
| F10 | **Standalone native (phase B).** The `sa-game` executable runs with no gta_sa.exe, Win32 or D3D9 dependency. F3, F4 and F5 recordings replay bit-exact against the stored original hashes. Rendering stays within the image-diff threshold of the phase A captures. | `tools/accept/f10-standalone.sh` |
| F11 | **Browser (phase C).** The same build runs in Chrome via WebGPU and via WebGL2, with the user's own assets in OPFS, and replays the F3 corpus bit-exact. Performance: ≥ 30 fps at 1280×720 on the reference machine for the "drive" corpus. | `tools/accept/f11-browser.sh` (headless Chromium) |
| F12 | **Stability.** A 4-hour free-roam soak and every F4 playthrough in sequence, natively and in the browser, with no crash, hang or memory growth beyond the original's. | `tools/accept/f12-soak.sh` |
| F13 | **SA:MP readiness.** Every samp.dll memory patch (inventory SAMP-1) has an engine option ("samp mode") with an L2/L3 diff between *original + patch* and *engine + option*. A documented engine API covers every game access samp-rs makes. | `tools/accept/f13-samp.sh` |

Known limits of what testing can prove (FEASIBILITY §4.6) apply. "1:1" means: **all of the above green,
plus an empty "pending" section in the divergence ledger.** Every accepted divergence carries the user's
approval.

---

## 2. Architecture (summary)

- **Phase A, in-process strangler.** `sa-hook` is an i686-pc-windows-gnu DLL injected by `sa-launcher` into
  the original 1.0 US under Wine. It contains the whole Rust engine. Every original function has a hook
  slot (address → Rust fn) that can be toggled between *Rust* and *original* at runtime. The data stays in
  the original memory layout (`#[repr(C)]` structs at the original global addresses), so original and Rust
  code interoperate freely. 1.0 US specifics: 501 functions are `jmp` stubs into `.HOODLUM`, so they are
  hooked at the stub address (CLAUDE.md).
- **Phase B, standalone.** Once F2 holds, the engine crates link into `sa-game` with the `sa-platform` native
  implementation (winit, wgpu, cpal, std fs). Globals move from fixed addresses into an engine context. The
  RW D3D9 driver layer is replaced by a wgpu driver behind the same RW device interface.
- **Phase C, browser.** `sa-platform` web implementation: a game worker plus OffscreenCanvas, WebGPU/WebGL2
  via wgpu, AudioWorklet over a SharedArrayBuffer, OPFS for BYO assets, COOP/COEP isolation. Networking via
  a WebSocket/WebTransport gateway (for samp-rs later).

Crates (created when their milestone starts; names are fixed now):

| Crate | Content | Targets |
|---|---|---|
| `sa-formats` | IMG, RW stream (DFF/TXD/ANM), COL1–4, IPL text/bin, IDE, IFP, SCM, GXT, DAT/CFG files, SFX/stream audio containers, saves | all |
| `sa-math` | CVector, CMatrix, CQuaternion, CGeneral, CMaths; `x87` module for bit-faithful helpers | all |
| `sa-rw` | RenderWare: rwcore/rpworld object model, frames, atomics, clumps, geometry, materials, raster/texture, plugins (skin, HAnim, MatFX, 2dfx, UVAnim, breakable, night colours, collision), the device interface | all |
| `sa-rw-d3d9` | RW device driver calling IDirect3DDevice9 (phase A) | i686-windows |
| `sa-rw-wgpu` | RW device driver on wgpu (phases B/C) | native, wasm |
| `sa-core` | pools, timer, memory manager, file manager, game loop states, globals registry | all |
| `sa-stream` | CdStream, CStreaming, CFileLoader, IPL/TXD/COL stores, model infos | all |
| `sa-world` | CWorld, entities, CPhysical, collision, zones, clock, weather, water, paths, interiors, garages, pickups, objects | all |
| `sa-anim` | anim manager, blend associations, IK, bones | all |
| `sa-ped` / `sa-veh` / `sa-weapon` | peds and player, vehicles of all classes, weapons/projectiles/explosions/fire | all |
| `sa-ai` | tasks, events, decision makers, ped intelligence, groups, attractors, conversations | all |
| `sa-pop` | population, cargens, car control/traffic, roadblocks, gangs, gang wars | all |
| `sa-script` | CTheScripts, CRunningScript, all command handlers | all |
| `sa-render` | CRenderer, visibility, shadows, post-effects, FX, coronas, clouds, timecycle, sprites | all |
| `sa-hud` | HUD, radar, fonts, text/GXT, messages, menus, loading screen | all |
| `sa-audio` | the CAE* system; backends: DirectSound (phase A, original), cpal, AudioWorklet | all |
| `sa-cam`, `sa-input`, `sa-save`, `sa-gameplay` (wanted, stats, cheats, replay, cutscenes, shopping, restart) | | all |
| `sa-platform` | Fs, Clock, Input, AudioOut, Gpu surface, Spawn; `native` and `web` impls | native, wasm |
| `sa-hook` | phase A DLL: hook table, toggles, control channel, determinism mode, oracle probes | i686-windows |
| `sa-launcher` | starts gta_sa.exe suspended under Wine and injects `sa-hook` | i686-windows |
| `sa-oracle` | L1 Unicorn runner, snapshot/normalise/diff, frame hasher, recording format, fixture store | host |
| `sa-game` | phase B/C entry point | native, wasm |

---

## 3. Phases and milestones

Every milestone runs the standard loop (§4) and must keep all earlier acceptance scripts green. The order
inside a phase follows the call graph: leaves first.

| Phase | Milestone | Scope | Acceptance | Depends on |
|---|---|---|---|---|
| **0 Catalogue** | M0 | complete function catalogue, call graph, global read/write sets, layout table, subsystem map, SA:MP address inventory | ROADMAP CAT-* done. `catalog/` regenerates from scripts | — |
| **I Infrastructure** | M1 | own Wine prefix + game copy, headless display lanes, `sa-launcher` + `sa-hook` skeleton (hook table, toggle, control channel) | the DLL loads, hooks a trivial function, toggles it live. `tools/accept/m1.sh` | M0 |
| I | M2 | determinism mode (timer, rand, CdStream sync, audio threads, heap), input recorder/replayer, frame state hasher | two replays of a 10-min recording are hash-identical. `m2.sh` | M1 |
| I | M3 | L1 Unicorn harness, L2 snapshot/restore/normaliser, L3 lockstep runner with divergence bisection, coverage collector, mutation runner, fixture store | the harness self-tests and mutation smoke test pass. `m3.sh` | M2 |
| I | M4 | D3D9 call-stream capture/diff, audio event log/diff, screenshot capture on the headless lane | identical capture twice on the original. `m4.sh` | M1 |
| I | M5 | corpus v1 (§6): scripted recordings + first user-recorded sessions | corpus manifest with coverage report | M2 |
| **II Foundations** | M6 | `sa-formats` (F6 for every format) | `f6-formats.sh` | M0 |
| II | M7 | MATH, CORE (pools, timer, memory, file mgr, CGeneral, rand) | all functions ≥ L1/L2 | M3 |
| II | M8 | STREAM: CdStream, CStreaming, CFileLoader, stores, model infos | L2 + L3 on streaming paths | M7 |
| II | M9 | RW core: object model, stream readers, frames/atomics/clumps, raster/texture, plugins; `sa-rw-d3d9` driver | L2 + D3D diff on static scenes | M8, M4 |
| **III World** | M10 | COLL + PHYS + WORLD core (CWorld, CEntity, CPhysical, CObject, zones, clock, weather, water, buoyancy) | L3 object trajectories | M9 |
| III | M11 | RENDER (renderer, visibility, shadows, post FX, FX system, coronas, clouds, timecycle, glass, plants, markers) | F7 subset on the screenshot corpus | M9, M10 |
| **IV Actors** | M12 | ANIM, INPUT, CAM | L3 on camera/anim recordings | M10 |
| IV | M13 | PED (player first, then all ped types), WEAPON | L3 on on-foot and combat recordings | M12 |
| IV | M14 | VEH: automobile, bike, BMX, quad, boat, heli, plane, train, trailer, monster truck; handling, damage, enter/exit, recordings | L3 per vehicle class | M12 |
| **V Simulation** | M15 | AI: tasks (1,800+ functions), events, decision makers, intelligence, groups, attractors, conversations | L3 on ambient recordings | M13, M14 |
| V | M16 | POP: population, popcycle, car control, cargens, roadblocks, gangs, gang wars, paths | L3 on ambient recordings over all zones/times | M15 |
| **VI Game** | M17 | SCRIPT: interpreter + all command handlers (781 ported in gta-reversed, 716 unimplemented, 283 NOP) | F4 playthroughs | M15, M16 |
| VI | M18 | HUD, menus, text, loading screen; SAVE; gameplay (wanted, stats, cheats, replay, cutscenes, shopping, garages, interiors, stunt jumps) | F5, F7, F4 | M17 |
| VI | M19 | AUDIO (CAE*: entities, managers, loaders, hardware layer on DirectSound) | F7 audio part | M13, M14 |
| VI | M20 | long tail: every remaining catalogue row; F1, F2, F8 | `f1`, `f2`, `f8` | all |
| **VII Standalone** | M21 | `sa-platform` native, `sa-rw-wgpu`, audio/input/fs backends, globals → engine context | F10 | M20 |
| **VIII Browser** | M22 | wasm build, worker/OffscreenCanvas, OPFS import, AudioWorklet, performance work | F11 | M21 |
| **IX Release gates** | M23 | F9 nightly, F12 soak, F13 samp mode + API doc | all F-scripts in one pass | M22 |

Phases III–VI overlap wherever files do not (§7). The leaf-first rule also applies across milestones: a
function can be ported as soon as all its callees are Rust or callable as original.

---

## 4. Standard loop per milestone / task

1. **Gap check.** Pick catalogue rows, leaves first. For each, record the source: gta-reversed (port) or
   original-only (REA decompile + Ghidra data flow). List unknowns.
2. **Measure the original** where behaviour is unclear: tracer probes in `sa-hook` with hooks on
   *original*, on the corpus. No guessing (§4b).
3. **Implement** in the owning crate. When porting gta-reversed, port its *semantics*, and check every
   `NOTSA`, bug fix and `#ifdef FIX_BUGS` against the original's bytes. gta-reversed sometimes fixes
   original bugs, and 1:1 means **keeping the original bug**. Its fixes go behind an option that is off by
   default. Register the hook.
4. **Verify** (§5): L1 for pure functions, L2 for every function, L3 for the subsystem, plus the hook-off
   check.
5. **Accept.** The main session reruns the milestone script, all earlier acceptance scripts,
   `cargo test --workspace`, and clippy for host and i686. Nothing counts on an agent's word.
6. **Record.** Catalogue state, ROADMAP status, WORKLOG entry. Residual differences go into the divergence
   ledger.

### 4b. No-guessing rule

Anything uncertain is resolved before it is implemented: (1) static analysis with REA/Ghidra, (2) dynamic
measurement on the running original, (3) only then implementation and diff. If both fail, the item stays
`blocked`. It is never filled with a plausible value. An agent that hits an unknown stops that item and asks
the main session for a measurement.

### 4c. Parity loop

Every implementation agent runs the same input on the original (hooks on *original*) and on Rust (hooks on
*Rust*), and iterates until L2/L3 are empty. The milestone acceptance includes the empty diffs. Any residual
difference must be a ledger entry.

---

## 5. Verification levels and function states

| State | Meaning |
|---|---|
| `unknown` | not yet identified (CAT-5) |
| `specified` | named, signature and layout known, source chosen (gta-reversed / REA) |
| `implemented` | Rust code exists and the hook is registered, not verified |
| `L1` | pure function: bit-exact against the original bytes on ≥ 10⁶ generated inputs plus edge cases, run under Unicorn, or on the real CPU under Wine for instructions Unicorn emulates unfaithfully (the evidence records 'CPU-decided' and the CPU model; FPATAN and other transcendentals are implementation-specific). Every observable output the callers use is compared (e.g. the full ST0 when a caller consumes it), under the FPU state of the real call sites |
| `L2` | in-process: for every corpus state at which the function is called (sampled up to N=1000 per function), original-vs-Rust output plus read/write set are identical after normalisation |
| `L3` | the subsystem's corpus recordings are frame-hash identical with this function on Rust, *and* with only this function switched back to original (hook-off check) |
| `excluded` | not game code (EH funclet, CRT, protection junk, replaced library), with the justification in the catalogue |

The oracle is always the original binary, and expected values are never produced by Rust. Normalisers live
in one reviewed file (`sa-oracle/src/normalise.rs`). Float tolerances are forbidden except as approved
ledger entries. L3 recordings are re-recorded from the original weekly, so no fixture can drift into
agreement with a wrong change.

x87: the game runs with the x87 precision control set to 24 bits by D3D9. Rust uses f32 by default. Any
function whose L1/L2 shows a precision divergence gets an explicit `sa_math::x87` implementation, never a
tolerance.

### 5.1 Writer/verifier split (user, 2026-11-11)

Porting is split between WRITER agents (write ports, no game lanes) and VERIFIER agents (verify on lanes,
one batch at a time). Procedure and tools: `docs/verify/HOWTO.md`; queue: `docs/verify/QUEUE.md`.

- **Unverified code never changes default master behaviour.** A port registered
  `#[hook(unverified = "BATCH", route = ..., hazards = ...)]` is compiled and in `sa_ports::ALL` (every
  table/callers/patch_refs/keycells/INF-20 test checks it), but phase A installs it only when a run
  selects it (`SA_UNVERIFIED=all|BATCH|@FILE`, or the hook named in `SA_HOOK_MODE`/`SA_L2`). Every
  all-ports run runs the verified set only. Phases B/C use unverified ports like any other.
- **Writers:** every new hook is `unverified` with a batch id (one batch = one writer commit or ≤ ~60
  hooks of one subsystem) and the hints `route` (L1-pure | L2 | split | L3-only) and `hazards`; they pass
  `tools/verify/writer-gate.sh` (builds host/i686/wasm32, clippy -D warnings, sa-hook lib tests, sa-oracle
  l1 tests, the hook check, L1 runs of L1-pure hooks) and regenerate the queue (`tools/verify/queue.py`).
- **Verifiers:** take one batch at a time, run it on top of the verified set
  (`tools/verify/bisect-batch.sh`, which narrows a divergence to the batch and then to the hook), L2 or
  split per hook, and promote by removing `unverified` only after L2/split evidence (L1 for L1-pure) and
  an all-ports run (`verify-all.sh`) IDENTICAL on all recordings with the promoted hooks in the default
  set. Failed hooks go back to the writer queue (status `failed`, the bisect output as evidence).

---

## 6. Corpus

The corpus is the finite set of inputs on which 1:1 is demonstrated. It must grow until F8 (≥ 90% block
coverage) holds.

### 6.1 Recordings (L3)
Format: start state (new game, or a save file) + engine settings + per-frame `CPad` state + the determinism
seed. Sources:
- **Scripted** (autonomous): generated input programs (walk/sprint/jump/swim routes, drive and fly routes
  per vehicle class, combat drills, camera sweeps, every zone at 4 times of day × weather set), plus SCM
  test scripts that set up situations (spawn vehicle X at Y, give weapons, set wanted level).
- **Human** (the user): playthroughs of story missions and activities recorded with the `sa-hook` input
  recorder on the original. Needed for F4 because autonomous play of every mission is not realistic. This
  is the main item that depends on the user (ROADMAP "Waiting for the user").
- **Save-based starts:** the user's 100% save plus per-mission saves let recordings start right before
  each mission.

### 6.2 Function states (L2)
Snapshots are captured automatically while the original replays the corpus: for each function, up to N
call sites with their read set. They are stored content-addressed with zstd under `fixtures/` (quota
20 GB, gitignored, rebuildable from the corpus).

### 6.3 Screenshot corpus (F7, F10)
A fixed list of camera positions × time × weather × scenario. The D3D call stream is captured in phase A,
images in phase B/C.

---

## 7. Scheduler: what to do at every goal re-injection

The goal text states only the end condition. The next action is derived from the repository state:

1. **Collect results** of finished agents. Verify them (§4 step 5). Update ROADMAP and WORKLOG.
2. **Fill free slots** from the ROADMAP "Next up" queue, top to bottom: take the first item whose
   dependencies are `done` and whose lane is free. Slots:
   - two implementation agents (separate crates/subsystems, separate git worktrees);
   - one measurement/RE agent (the only REA user at a time);
   - one harness agent (the only writer of `sa-oracle`, normalisers and acceptance scripts);
   - one verifier agent (adversarial: gets code + oracle, not the implementer's reasoning; writes extra
     cases and mutation tests).
   Never two writers on the same files.
3. **Refill the queue** when it runs low: expand the next milestone of §3 into catalogue-driven tasks
   (one task = one class or a leaf-closed group of ≤ ~60 functions), each with its acceptance.
4. **Nothing startable** → wait for the next agent notification. If blocked only on the user, add it to
   "Waiting for the user" and continue with something else.
5. **Done check:** when every F-script exits 0, run them all once more in one pass and show the outputs.

---

## 8. Concurrency and resource rules

- Several game lanes may run at once (lanes 1–7; replays are CPU-bound since INF-19, each lane needs ~1.5–2 GB RAM; `e2e_disk_check` waits for ≥4 GB available memory before a run); real-time lanes rely on sa-hook's CdStreamSync lost-wake-up fix (INF-15, docs/re/lanes.md), without which any real-time run can hang on a loading screen (~7% of new-game loads). Create new lane prefixes only while no wineserver runs for `wine/prefix`. Lanes are separate Wine prefixes plus game copies (`wine/lane-N`, data symlinked read-only) and headless
  gamescope displays. Every lane run is wrapped in `flock target/locks/lane-N.lock`. Start with 3 lanes.
- REA (revised 2026-11-11, measured: parallel queries on one open target all succeed): agents may query REA
  concurrently, read-only, on the one target the main session keeps open (`re/gta/1.0-US/gta_sa.exe` with
  `re/snapshots/gta_sa_1.0US.snapshot`). Agents never open another target, never call `close_binary`, and never
  annotate in REA (a cancelled request re-imports the program and drops every session's annotations); names go
  to the catalogue. objdump and `catalog/raw/` remain fine alternatives.
- Port crates are registered in ONE place: `crates/sa-ports` (Cargo.toml dependency + one `CRATES` line, appended at the end). sa-hook's table, its callers test, sa-oracle's hook_preserve and the L1 preserve test read `sa_ports::ALL`; do not chain crates anywhere else.
- Shared crates (`sa-core`, `sa-math`, `sa-rw`, `sa-oracle`) must keep `cargo build --workspace` green after
  every edit.
- Git: only the main session commits, after a green build and tests, with plain English messages and no
  trailers. Agents work in worktrees and never commit. Push only when the user asks.
- Agents never kill processes outside their lane, never use `pkill -f`, never screenshot the user's
  desktop, never write to `~/samp-rs`.
- Memory: the machine has 31 GB RAM and no swap. Heavy processes (oracle runs, Ghidra headless, bulk Python, game lanes) run under a cap: `systemd-run --user --scope -q -p MemoryMax=<N>G -p MemorySwapMax=0 <cmd>` (default 6G; Ghidra 8G). Reason: on 2026-11-05 an `sa-oracle` run triggered the kernel OOM killer.
- Disk: `fixtures/`, `traces/` and coverage logs have quotas. Keep ≥ 20 GB free on /home, checked by the
  scheduler before starting a lane run.
- Nested agents (user, 2026-11-11 evening; replaces the 2-helper rule of that morning): sub-agents start **no** agents, helpers, forks or workflows; only the main session spawns agents. Every brief states this. Reasons: unbounded nesting (20 agents on 2026-11-10), and helpers misrouting messages, colliding in the shared scratchpad and adding disk/RAM load (2026-11-11).
- Budget: ≤ ~1.5M tokens per agent step. Workflows only for broad sweeps (for example CAT-5
  classification).

## 9. Escalation to the user

Only for: pushing to GitHub, legal or publishing questions, system installs (sudo), irreversible actions
outside the project, recording human playthroughs (§6.1), approving accepted divergences, and real-hardware
browser checks. Everything else is decided autonomously and logged in ROADMAP "Decisions made
autonomously".
