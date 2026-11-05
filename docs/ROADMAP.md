# Roadmap / task board

The single source of truth for what is done, in progress and next. The plan, the definition of done (F1–F13)
and the milestone acceptance live in `docs/PLAN.md`. Update this file at the end of every work step;
`docs/WORKLOG.md` keeps the history.

Status: `done` · `in-progress` · `todo` · `blocked` (needs the user) · `deferred`.
Function counts come from `catalog/subsystems.md` (regenerate with `tools/catalog/*.py`).

## Lanes now

| Slot | Running | Next |
|---|---|---|
| impl-1 | FMT-2 (next) | FMT-3 |
| impl-2 | INF-2 + INF-3 launcher/sa-hook (worktree) | INF-4 |
| measurement / REA | CAT-5 | CAT-7 |
| harness | — | — |
| verifier | — | — |

## Next up (ordered queue; the scheduler takes from the top)

1. CAT-5: classify the 9,454 unknown game-`.text` functions (EH funclets, CRT, the third-party lib at
   0x760000–0x7BFFFF, protection junk, real game functions)
2. CAT-7: call graph + global read/write sets (Ghidra headless script)
3. INF-1: own Wine prefix and game copy, lanes, headless display
4. INF-3: `sa-launcher` + `sa-hook` skeleton
5. FMT-1…: `sa-formats` (can run in parallel with INF; no game needed)
6. CAT-6, CAT-8, CAT-9, CAT-10, CAT-11, SAMP-1
7. INF-4…INF-14

## Phase 0: catalogue (M0)

| ID | Task | Status | Verification | Notes |
|---|---|---|---|---|
| CAT-1 | Parse gta-reversed hook registrations + callee addresses | done | 8,635 registrations, 7,904 reversed / 731 not | `catalog/functions.csv`, `callees.csv` |
| CAT-2 | Verify every hook address on 1.0 US | done | 8,254 ok, 379 `.HOODLUM` stubs, 1 placeholder, 1 outside | `catalog/summary.md` |
| CAT-3 | How game code reaches `.HOODLUM` | done | 501 `jmp`, 0 `call` from game `.text` | `hoodlum_xrefs.py` |
| CAT-4 | Merge with the Ghidra procedure list | done | 20,823 Ghidra functions + 1,890 missed hook targets | `catalog/procedures.csv` |
| CAT-4b | Subsystem map | done | every hooked function has a subsystem, `OTHER` = 0 | `subsystems.py` → `catalog/subsystems.md` |
| CAT-5 | Classify unknowns: `UNK-eh-funclets` 3,689 · `UNK-game` 3,145 · `UNK-thirdparty-lib` 930 · `UNK-rw-crt` 899 · `UNK-protection-mix` 791 | todo | each row gets a subsystem or `excluded` with a reason | confirm EH funclets via FuncInfo; identify the library through strings/xrefs; split RW from CRT through the RW names known from rwcore headers |
| CAT-6 | Name unknowns from plugin-sdk addresses (own pinned clone in `re/plugin-sdk`) | todo | match count recorded | |
| CAT-7 | Call graph + global read/write sets per function | todo | Ghidra headless script output in `catalog/callgraph.*`, `catalog/rwsets.*` | needed for leaf-first order and L2 snapshots |
| CAT-8 | Layout table from `VALIDATE_SIZE` / `VALIDATE_OFFSET` | done (table) | `catalog/layouts.csv`: 781 sizes + 39 offsets (20 symbolic expressions kept verbatim) | Rust const-assert generation happens when the crates that own the types exist |
| CAT-9 | Fix the `.HOODLUM` body/stub double count | todo | totals reconcile | |
| CAT-10 | Script command table | in-progress | part 1 done: `catalog/script-commands.csv`, 2,826 opcodes from eScriptCommands.h (gta-reversed: 770 handler, 713 unimplemented, 281 NOP, 13 unsupported, 1,049 unregistered). Part 2 todo: the original's handler per opcode from the `ProcessCommands*` switch ranges (REA, after CAT-5), and which opcodes `main.scm`/`script.img` actually use (needs FMT-3) | |
| CAT-11 | Per-function state file (`catalog/state.csv`) + `tools/catalog/progress.py` → `catalog/progress.md` | todo | report per subsystem and state (PLAN §5) | |

## Phase I: infrastructure (M1–M5)

| ID | Task | Status | Verification | Notes |
|---|---|---|---|---|
| INF-1 | Wine prefix `wine/prefix` + game copy `wine/gta-sa` (1.0 US exe + data), lanes `wine/lane-N` with symlinked data, DXVK | in-progress | single-player boots on a headless lane | modelled on samp-rs `tools/run-gta.sh` (read-only reference; game data copied, never modified there) |
| INF-2 | Headless gamescope per lane: start, screenshot, key injection, teardown (`tools/e2e/lib.sh`) | todo | screenshot of the main menu from lane 1 | never the user's desktop |
| INF-3 | `sa-launcher` (suspended start + injection) and `sa-hook` skeleton: hook table from the catalogue, inline jmp + trampoline, `.HOODLUM` stub handling, runtime toggle, TCP control channel, logging | todo | hook `CTimer::Update` (Rust passthrough), toggle live, 0 crashes in a 10-min run | |
| INF-4 | Determinism mode: virtual `timeGetTime`/QPC, fixed timestep, `rand`/`srand` logged with caller, synchronous CdStream, audio thread control, deterministic heap | todo | two runs of the same recording are frame-hash identical | |
| INF-5 | Input recorder/replayer (`CPad` per frame) + recording format (`sa-oracle`) | todo | record → replay → same hashes | |
| INF-6 | Frame state hasher: per-subsystem hashes (pools, entity transforms/velocities, world lists, streaming, script vars, camera, clock/weather, rand state) | todo | detects a 1-ulp change injected in a ped position | |
| INF-7 | L1 Unicorn harness: map the original image, call a function with generated args/memory, compare with Rust | todo | self-test on 5 math functions | |
| INF-8 | L2: snapshot/restore of the read/write set in-process, call original then Rust, normalise, diff, store fixtures | todo | self-test + mutation smoke test | needs CAT-7 |
| INF-9 | L3 runner + automatic divergence bisection (first divergent frame → subsystem → function by toggling hooks) | todo | finds an injected fault without help | |
| INF-10 | Execution coverage of the original (drcov under Wine, or breakpoint sampling in `sa-hook`) | todo | coverage report of the corpus | F2, F8 |
| INF-11 | D3D9 call-stream capture/diff + audio event log/diff + screenshot capture | todo | same capture twice on the original | modelled on samp-rs `d3d.rs` (reference only) |
| INF-12 | Mutation runner (inject faults into Rust functions, expect detection) | todo | F9 smoke test | |
| INF-13 | Acceptance scripts `tools/accept/*.sh` (F1–F13 + per-milestone) | todo | each exits 0/1 with a readable report | |
| INF-14 | Corpus v1: scripted recordings (routes per zone/time/weather, vehicle classes, combat drills) + SCM setup scripts | todo | manifest + coverage % | grows until F8 |

## Phase II: foundations (M6–M9)

| ID | Task | Funcs (rev/unrev) | Status | Verification |
|---|---|---|---|---|
| FMT-1 | IMG v2, RW chunk stream (DFF/TXD), COL1–4 | — | done | 8 IMG / 21,038 entries, 15,347 DFF, 4,037 TXD (32,869 textures), 253/254 COL byte-exact round trip. peds.col is an expected failure (not loaded as collision; ValidateVersion stamp bytes, see docs/re/formats.md). Open: verify the triangle index order against the exe's RW stream reader before rendering relies on it |
| FMT-2 | IPL text+binary, IDE, all DAT/CFG (handling, timecyc, carcols, pedstats, weapon, surfinfo, water, paths/nodes, object.dat, popcycle, …) | — | todo | the original's in-memory tables equal after load |
| FMT-3 | IFP (ANP3), GXT, SCM (main + script.img), SFX/stream audio containers, saves | — | todo | same as above |
| MATH | vectors, matrices, quaternions, CGeneral, CMaths, x87 helpers | 96/0 | todo | L1 |
| CORE | pools, timer, memory mgr, file mgr, game/app states | 265/10 | todo | L2, L3 boot sequence |
| STREAM | CdStream, CStreaming, CFileLoader, stores, model infos | 472/1 | todo | L2 + L3 streaming paths |
| RW | RW core + plugins + pipelines + about 850 thunked RW functions from REA | 291/19 (+~850) | todo | L2 + D3D diff |

## Phase III: world (M10–M11)

| ID | Task | Funcs | Status | Verification |
|---|---|---|---|---|
| COLL | collision | 141/0 | todo | L1 for primitives, L2 |
| PHYS | CPhysical, buoyancy | 63/2 | todo | L3 object trajectories |
| WORLD | CWorld, entities, objects, zones, interiors, garages, pickups, misc world | 621/123 | todo | L3 |
| RENDER | renderer, visibility, shadows, FX, post FX, coronas, clouds, timecycle, weather, water, glass, plants, markers | 676/50 | todo | F7 subset |

## Phase IV: actors (M12–M14)

| ID | Task | Funcs | Status | Verification |
|---|---|---|---|---|
| ANIM | anim manager, blends, IK, bones, cutscene anims | 243/1 | todo | L3 |
| INPUT | pad, controller config, mouse/keyboard | 141/0 | todo | L2 |
| CAM | camera, cams, idle cam, shakes | 140/42 | todo | L3 camera recordings |
| PED | peds and player | 312/32 | todo | L3 on-foot recordings |
| WEAPON | weapons, projectiles, explosions, fire, water cannon | 116/18 | todo | L3 combat drills |
| VEH | all vehicle classes, handling, damage, enter/exit, recordings | 523/64 | todo | L3 per class |

## Phase V: simulation (M15–M16)

| ID | Task | Funcs | Status | Verification |
|---|---|---|---|---|
| AI | tasks, events, decision makers, intelligence, groups, attractors, conversations | 2,342/189 | todo | L3 ambient |
| POP | population, popcycle, car control, cargens, roadblocks, gangs, paths | 308/99 | todo | L3 ambient, all zones/times |

## Phase VI: game (M17–M20)

| ID | Task | Funcs | Status | Verification |
|---|---|---|---|---|
| SCRIPT | interpreter + handlers (see CAT-10) | 115/28 + commands | todo | F4 |
| HUD | HUD, radar, fonts, text, menus, loading screen | 298/11 | todo | F7 |
| SAVE | generic game storage | 13/15 | todo | F5 |
| AUDIO | CAE* entities/managers/loaders/hardware | 718/26 | todo | F7 audio |
| TAIL | `CALLED-ONLY` 528 + classified `UNK-game` remainder | 528 + ? | todo | F1, F2, F8 |

## Phase VII–IX: standalone, browser, release gates

| ID | Task | Status | Verification |
|---|---|---|---|
| STD-1 | `sa-platform` native (winit, cpal, fs, clock, input) | todo | F10 |
| STD-2 | `sa-rw-wgpu` driver (D3D9 fixed function + game pipelines in WGSL) | todo | image diff against phase A captures |
| STD-3 | Globals → engine context; remove every fixed-address dependency | todo | builds for x86_64 |
| WEB-1 | wasm32 build (atomics, build-std), worker + OffscreenCanvas, OPFS import, AudioWorklet | todo | F11 functional |
| WEB-2 | Performance: ≥ 30 fps at 720p on the reference machine | todo | F11 perf |
| REL-1 | Nightly mutation (F9), soak (F12) | todo | |
| SAMP-1 | Read-only inventory of the 542 GTA addresses used by samp-rs `crates/samp-client/src`, classified (call / global / patch / hook) and mapped to catalogue names → `docs/samp/` | todo | every address resolved or listed |
| SAMP-4 | "samp mode" engine options for every samp.dll memory patch | todo | F13 |
| SAMP-API | Document the engine API covering every samp-rs game access (proposal for the samp-rs side; we never edit samp-rs) | todo | F13 |

## Decisions made autonomously
- 2026-11-05: game data = a copy of the Steam 3.0 data set already used by samp-rs (SA:MP files excluded) with the 1.0 US exe. The original and Rust run on the same data, so parity is unaffected. Data-dependent findings may not match retail 1.0 data, and the corpus records the data hashes.
- 2026-11-05: the catalogue's function set is Ghidra functions ∪ gta-reversed hook addresses that decode as
  code, because Ghidra misses vtable-only functions.
- 2026-11-05: gta-reversed's bug fixes (`FIX_BUGS`, `NOTSA` changes) are not ported as default behaviour.
  1:1 keeps the original bug, and fixes sit behind options that are off by default.

## Divergence ledger
Sections: *pending* (blocks "done") · *original UB, not reproducible* · *accepted (user-approved)*.
(empty)

## Waiting for the user
- Human playthrough recordings of the missions (PLAN §6.1) are needed for F4. Not needed before M17. The
  recorder (INF-5) has to exist first.
