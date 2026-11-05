# Roadmap / task board

The single source of truth for what is done, in progress and next. The plan, the definition of done (F1–F13)
and the milestone acceptance live in `docs/PLAN.md`. Update this file at the end of every work step;
`docs/WORKLOG.md` keeps the history.

Status: `done` · `in-progress` · `todo` · `blocked` (needs the user) · `deferred`.
Function counts come from `catalog/subsystems.md` (regenerate with `tools/catalog/*.py`).

**PAUSED (2026-11-05, user request):** no new agents. The running ones (INF-9/10, CORE-L2, PORT-STREAM, RE-SPEC-2) finish, get verified and are committed; then the project stops. To resume: read this file, the WORKLOG tail and the "Next up" queue.

## Lanes now

| Slot | Running | Next |
|---|---|---|
| impl-1 | — | FMT-2 part b (after INF-4) |
| impl-2 | — | INF-15 / INF-9 |
| measurement / REA | — (CAT-6 done, CAT-13 partial) | CAT-13 (typed arguments) |
| harness | MATH-2 (port all MATH with L1, worktree) | INF-8 (after INF-4) |
| verifier | audit of the L1 oracle + sa-math claims (worktree) | — |

## Next up (ordered queue; the scheduler takes from the top)

1. CAT-5: classify the 9,454 unknown game-`.text` functions (EH funclets, CRT, the third-party lib at
   0x760000–0x7BFFFF, protection junk, real game functions)
2. INF-1: own Wine prefix and game copy, lanes, headless display
3. INF-3: `sa-launcher` + `sa-hook` skeleton
4. FMT-1…: `sa-formats` (can run in parallel with INF; no game needed)
5. CAT-6, CAT-8, CAT-9, CAT-10, CAT-11, SAMP-1
6. INF-4…INF-14

## Phase 0: catalogue (M0)

| ID | Task | Status | Verification | Notes |
|---|---|---|---|---|
| CAT-1 | Parse gta-reversed hook registrations + callee addresses | done | 8,635 registrations, 7,904 reversed / 731 not | `catalog/functions.csv`, `callees.csv` |
| CAT-2 | Verify every hook address on 1.0 US | done | 8,254 ok, 379 `.HOODLUM` stubs, 1 placeholder, 1 outside | `catalog/summary.md` |
| CAT-3 | How game code reaches `.HOODLUM` | done | 501 `jmp`, 0 `call` from game `.text` | `hoodlum_xrefs.py` |
| CAT-4 | Merge with the Ghidra procedure list | done | 20,823 Ghidra functions + 1,890 missed hook targets | `catalog/procedures.csv` |
| CAT-4b | Subsystem map | done | every hooked function has a subsystem, `OTHER` = 0 | `subsystems.py` → `catalog/subsystems.md` |
| CAT-5 | Classify the 9,454 unknowns | done | `classify_unknowns.py` → `catalog/classification.csv`, reproducible (re-run identical). Excluded 6,661 (EH funclets 3,556 via FuncInfo tables, D3DX9 949, protection 741, Ghidra mis-splits 653, CRT 508, libpng 1.0.12 + zlib 1.1.3 194, QuickTime glue 34, linker thunks 26). RW +312. Game 2,481 (1,015 assigned to a subsystem, 1,466 `UNK-game`) | notes `docs/re/cat5-unknowns.md`; 138 rows rest on link order only (marked) |
| CAT-6 | Name unknowns from plugin-sdk addresses (own pinned clone in `re/plugin-sdk`) | done | `tools/catalog/names_extra.py` (in the `run_all.sh` loop) → `catalog/names-extra.csv` (3,298 rows, 2,944 high/medium) + rw-names.csv wired in as the RW name source; final name precedence gr hook > rw-names (high/med) > CAT-5 > CAT-6 written by `subsystems.py` to `procedures.csv` (`name`, `name_source`, `name_confidence`, `role`, `name_hint`) and `state.csv` (`name`, new `role` column). Unnamed non-excluded rows 5,862 → 1,656 (non-RW 4,354 → 1,418; RW 1,508 → 238). Non-RW by source: static initialisers `__init_<global>` 1,666 + atexit `__dtor_<global>` 138 (role init/dtor), gta-reversed address comments 459, source wrappers 110, unknown-vtable slots 562 (141/164 vtables; class proven for 101, base only for 40 → `vt_<addr>::Slot`), `#define` 1. Low hints (354 + 237 rw-names) only in evidence. `CALLED-ONLY` 944 → 37. 0x750480 added (extra-code-addresses, rule rw-names). Spot checks 2×15 vs bytes/gta-reversed: consistent. Two runs sha256-identical | notes `docs/re/cat6-names.md`. 1,571 initialiser/dtor names are `g_<address>` (file-static floats with no source name). plugin-sdk-only names are medium (measured 2–3 % disagreement with gta-reversed, incl. swapped pairs). gta-reversed `TaskComplexClimb.h` Clone/GetTaskType comments are swapped (layout wins). |
| CAT-7 | Call graph + global read/write sets per function | done | `tools/catalog/callgraph.py ghidra` + `build` → `catalog/extents.csv` (11,945 bodies, 0 fallback), `callgraph.csv` (36,890 rows: 31,275 call, 811 tailcall, 208 import, 223+363 virtual/override, 1,874 eh-unwind, 4,715 unresolved indirect sites), `rwsets.csv` (42,677 rows, 13,282 globals, 14,600 named), `indirect-calls.csv`, `port-order.csv` (44 levels, 3,926 leaves, 15 SCCs / 64 functions). Spot check vs gta-reversed (2×10 functions): every miss is inlining or a checker false positive. Rerun incl. fresh Ghidra analysis byte-identical | notes `docs/re/cat7-callgraph.md`. Blind spots: virtual calls on non-`this` objects; 93 unresolved computed jumps. Found 153 called/vtable code addresses missing from the catalogue (`catalog/raw/cat7-noncatalogue-targets.csv`) |
| CAT-8 | Layout table from `VALIDATE_SIZE` / `VALIDATE_OFFSET` | done (table) | `catalog/layouts.csv`: 781 sizes + 39 offsets (20 symbolic expressions kept verbatim) | Rust const-assert generation happens when the crates that own the types exist |
| CAT-9 | Fix the `.HOODLUM` body/stub double count | done | `procedures.csv` rows = `subsystems.md` total = `summary.md` game functions = `state.csv` rows = 19,425; 497 `.HOODLUM` bodies are `stub-body` rows counted at their stub (extent = stub + body); 8,630 of 8,632 distinct hook addresses in the catalogue (the 2 others: 0x8d0aa8 in .data, 0xdeadbeef placeholder). Reconciliation section asserted in `subsystems.py` | notes `docs/re/cat9-12-pipeline.md` |
| CAT-10 | Script command table | done | part 1 done: 2,826 opcodes from eScriptCommands.h (gta-reversed: 770 handler, 713 unimplemented, 281 NOP, 13 unsupported, 1,049 unregistered). Part 2 done: `ProcessOneCommand` table 0x8A6168 → 27 `ProcessCommandsXToY` switches decoded (`script_commands.py`): `orig_handler` (case), `orig_range_fn`, `orig_dispatch` (1,970 case / 671 default / 185 no range function, opcodes ≥ 2700), `orig_case_shared`. Verified: all 35 gta-reversed case-address annotations fall in the decoded case block; 5 handlers compared by hand. 15 gta-reversed `handler` opcodes (e.g. RETURN_TRUE/FALSE, SET_CHAR_ARMOUR) hit the default (-1) in the PC exe. Part 3 todo: which opcodes `main.scm`/`script.img` use (needs FMT-3) | notes `docs/re/cat9-12-pipeline.md` |
| CAT-11 | Per-function state file (`catalog/state.csv`) + `tools/catalog/progress.py` → `catalog/progress.md` | done | 19,425 rows: excluded 6,664, specified 9,633 (7,897 gta-reversed, 1,736 REA), unknown 3,125, implemented 3 (0x53CB50, 0x441390, 0x561B10 passthrough; no oracle yet). F1 = 0 / 12,761 at ≥L2. Merge keeps states above `specified`; rerun byte-identical | `progress.py set ADDR STATE EVIDENCE` records changes |
| CAT-12 | Close the catalogue over call / vtable targets | done | `merge_ghidra.py` adds missed hooks (incl. `.HOODLUM` stubs) 1,894, gta-reversed callees 416, gta-reversed vtable slots 351, CAT-7 call targets 46 (`catalog/extra-code-addresses.csv`, fixpoint via `callgraph.py gaps`). `noncatalogue_targets` = 0; 49 `.HOODLUM`/protection call targets (75 sites) are SecuROM return-address thunks → `excluded:protection-thunk`. All 153 original gaps are rows (130 vtable slots, 112 named + 18 `vfunc_N`). Slot names from validated header layouts (`vtables.py`, 328/379 vtables). Whole pipeline: `tools/catalog/run_all.sh`, two runs sha256-identical | notes `docs/re/cat9-12-pipeline.md`; caveat: bytes after a SecuROM thunk call are junk in the caller's linear decode |
| CAT-12b | Close the catalogue over data-referenced code (function-pointer tables, structs, init tables, code immediates) | done | `tools/catalog/data_refs.py` (in the `run_all.sh` fixpoint loop) → `catalog/data-code-addresses.csv` (2,720 entries, `merge_ghidra.py` rule `data-<kind>`): crt-init (`__xc` dynamic initialisers) 1,666, code immediates 595, unknown vtables 343 (164 tables), cheat table 58/65, ProcessCommands table 24/27, anim-callback/d3dx9/RW unreferenced tables 17, RxNodeDefinition 8, RwDevice 4, struct arrays 4, fnptr variable 1. Catalogue 19,425 → 22,161 functions (15,485 live); 0 accepted entries outside the catalogue, `noncatalogue_targets` 0; reports `catalog/data-tables.csv` (1,615 tables) + `data-refs.csv` (every rejected value with reason); two runs sha256-identical | notes `docs/re/cat12b-data-tables.md`; cheat table = gta-reversed 92/92, script table = plugin-sdk 27/27; new rows are `unknown`/`UNK-game` (naming → CAT-6) |
| CAT-13 | Resolve virtual calls on non-`this` objects (2,791 tagged with a slot but no target): type propagation from gta-reversed member types and Ghidra data types, then use the result in the port order | in progress | `tools/catalog/cat13_types.py` + typed tokens in `callgraph.py build`: receiver class from validated header layouts (604 classes whose computed size = VALIDATE_SIZE and offsets = VALIDATE_OFFSET), 38 typed globals, 1,528 typed return values. Virtual-pattern sites 3,092: typed 596 (19.3 %) = 134 with targets (`virtual-typed` / `-override` edges, ≤16 overrides, in the port order) + 462 `virtual-typed-wide` (abstract base / >16 overrides, mostly CTask `m_pSubTask`). Unresolved indirect sites 5,566 → 4,971. Also: x87 stores now `W` in rwsets.csv; gta-reversed header vtables (8) used for `this` | notes `docs/re/cat6-names.md` §CAT-13. Open: typed stack arguments, pool slots, CAutomobile-family layouts (8 bytes off), fan-out >16 |
| CAT-14 | gta-reversed fidelity audit: list every place where its C++ deviates from the original on purpose (`NOTSA`, `FIX_BUGS`, `#ifdef` fixes, changed constants, "fixed" bugs, unported branches) per function → `catalog/gr-deviations.csv`, so ports keep original behaviour by default | done | 3,997 rows (`tools/catalog/gr_deviations.py`, in `run_all.sh`): keep-original/yes 840, keep-original/no 184, unclear 1,238, harmless 1,735; 2,764 mapped to 1,569 addresses; `state.csv` column `gr_deviations` (978 functions > 0). 26 reversible bug fixes (+1 undefined) with 53 use sites; 15 script-default + 70 unimplemented-with-case opcodes. 11 spot checks against the exe, all confirmed. Findings: gr's `CGeneral::GetRandomNumberInRange` differs from 0x407180/0x41BD90 (375 call sites), `GetAllValid` iterates in ascending order (98 sites), 24/27 ProcessCommandsXToY and 58/65 cheat functions are missing from the catalogue; fixed the CVehicleModelInfo names (75 rows). Two `run_all.sh` runs sha256-identical | docs/re/cat14-gr-deviations.md; hand checks in `catalog/gr-deviations-notes.csv` |
| CAT-15 | rwsets.csv misses x87 stores | done | `callgraph.py`: every x87 store mnemonic is now `W` whatever capstone reports (capstone marked `fstp m32` as R). 0x561B10 → 0xB7CB50 is now W; 10,226 W rows | |
| CAT-16 | Fall-through edges: WinMain falls into 0x7487CF (named `Input::Initialise` in gta-reversed), so `CGame::Initialise` is unreachable in the call graph; find every function that falls through into another catalogue entry and add `fallthrough` edges | done | `callgraph.py build` checks every live body instruction that runs on into another entry → `catalog/fallthrough.csv` (decision + evidence); `callgraph.py fallthrough` (run_all fixpoint) merges entries with no other reference into the faller via `catalog/fallthrough-splits.csv` → `excluded:mis-split`. Result: 7 mis-splits merged (6 auto, 1 hand: 0x7487cf, 0x4c88f4, 0x503219, 0x732307, 0x7f6781, 0x7f7c41, 0x43a7a0; 6 were gta-reversed hooks at mid-function / wrong addresses), 1 noreturn-call (dead code after `exit`), 0 real `fallthrough` edges (kind added to ORDER_KINDS and samp_usage FOLLOW). WinMain 206 → 2,023 B; WinMain → Game::InitialiseGame → CGame::Initialise reachable; 15,479 live. 10 disassembly spot checks; two run_all runs sha256-identical | notes `docs/re/cat16-fallthrough.md`; follow-ups: rw-names.csv 0x732307 name → 0x7322f0, sa-oracle L2 closure should follow `fallthrough` |
| RE-SPEC-1 | Port-ready specs for STREAM/COLL/PHYS/WORLD functions gta-reversed lacks + deviation notes | done | `docs/re/specs/`, `catalog/specs-index.csv` (560 rows): 392 full specs, 168 deviation notes (confirmed 185 / refuted 175 / unclear 2 gr rows); main-session spot check (CPickups::PictureTaken pays 100 per photo + 100,000 bonus) matches the bytes | open questions listed per spec; suspected catalogue name errors (TxdStore.cpp:177 → 0x7318E0, VehicleModelInfo.cpp:1627–1639 → 0x4C7FB0, CLodTimeModelInfo hooks, 0x44DF60 = CPathFind::RemoveInteriorLinks) → CAT-17 |
| CAT-17 | Apply the name corrections found by RE-SPEC-1/2 (incl. vehicle vfunc_0 = scalar deleting dtors, CVehicle vfunc_N map in VEH/CVehicle.md, ~CMonsterTruck 0x6C7D10, ~CTrain 0x6F54D0, ~CPlayerPed 0x6093B0, ~CCamera 0x50A870; extent fixes 0x5245B0 and 0x6C1470) (via names-extra.csv / rw-names.csv, with evidence) and rerun the pipeline | todo | names match the specs | |
| RE-SPEC-2 | Specs for ANIM, CAM, INPUT, PED, WEAPON, VEH | done | 405 full specs + 156 deviation notes (specs-index.csv now 1,122 rows); tooling tracked in `tools/specs/` (incl. `check_consts.py`: 294 constant claims checked against .rdata); main-session spot check 0x739F1F `fmul [0x8714A8]` = 0.075f matches | name corrections for CAT-17 and catalogue extent errors (0x5245B0 Process_FollowCar_SA, 0x6C1470 missing chunks, rwsets loop-end immediates) listed in the agent report / WORKLOG |

## Phase I: infrastructure (M1–M5)

| ID | Task | Status | Verification | Notes |
|---|---|---|---|---|
| INF-1 | Wine prefix `wine/prefix` + game copy `wine/gta-sa` (1.0 US exe + data), lanes `wine/lane-N` with symlinked data, DXVK | done | boots; `vorbisFile.dll` was missing at first (STATUS_DLL_NOT_FOUND), then copied | no intro movies; Space is pressed while gGameState is 2/4 |
| INF-2 | Headless gamescope per lane: start, screenshot, key injection, teardown (`tools/e2e/lib.sh`) | done | lane 1 main menu + gameplay screenshots | lanes created on demand by `e2e_lane_setup`; `flock target/locks/lane-N.lock`; teardown = `wineserver -k` for the lane prefix + its own gamescope group |
| INF-3 | `sa-launcher` + `sa-hook` skeleton | done | `tools/e2e/inf3-hooks.sh`: 3 hooks (CTimer::Update passthrough; CGeneral::LimitRadianAngle and CGameLogic::IsCoopGameGoingOn (a `.HOODLUM` stub) ported), live toggles + a 300-switch storm, 600 s soak 0 crashes, selftest 160,024 inputs 0 mismatches. Re-run by the main session: all PASS | x86 decoder checked against iced-x86 (18,613 entries, 1.53M-instruction sweep). The selftest is not yet a recorded L1/L2 oracle. Branch-into-patch check is heuristic (0x400 bytes) |
| INF-4 | Determinism mode | done | `tools/e2e/inf4-determinism.sh` (lane 2), re-run by the main session: record + 2 replays hash-identical on all 17 subsystems over 5,200 frames | virtual clock (40 ms step) at the game's time call sites, busy-wait sites patched (WinMain limiter, Game::Idle 0x53E930), rand 0x821B1E hooked, main-thread CRT heap arena at 0x20000000, synchronous CdStream, audio threads stepped from CAEAudioHardware::Service with a virtual DirectSound cursor. Open: user-track scan thread, window focus, Pads[1]/menu mouse not injected, non-main-thread heap. `docs/re/determinism.md` |
| INF-5 | Input recorder/replayer | done | per input tick (CPad::UpdatePads) from process start; replay desyncs=0 | format + `satool` in `crates/sa-oracle-format`; scripted input programs `tools/e2e/programs/` |
| INF-6 | Frame state hasher | done | 17 subsystem hashes; a 1-ulp player x change at frame 3700 is caught at 3700 in `peds` only | not hashed yet: ped health, AI tasks, audio, menu state |
| INF-7 | L1 Unicorn harness | done | `crates/sa-oracle` 14 tests; `l1 <fn> --cases 1000000`: 0 mismatches on 6 functions in `crates/sa-math`; re-run by the main session (tests + SolveQuadratic 1M + mutants 3/3) | `docs/re/l1-harness.md`. Unicorn 2.1.5 defects measured: MXCSR.FZ leaks into x87 (worked around), SNaN loads not quieted (those cases go to the CPU), transcendentals/FSCALE/FXTRACT unfaithful (routed to L2 by a code scan); a native CPU backend runs under Wine |
| INF-8 | L2 in-process oracle | done | `tools/e2e/inf8-l2.sh`, re-run by the main session: all PASS. 7 hooked ports: 1,000 cases each on the grove-walk replay with 0 mismatches and 0 undeclared writes (LimitAngle has no calls); the oracle does not perturb the run (hashes identical); deliberate faults caught; undeclared-write detection; 3,393 fixtures replay on the host | `docs/re/l2-oracle.md`. Limits: memory outside the watched windows is not compared unless `SA_L2_WINDOW=all`; snapshots go one level deep; allocation paths are unit-tested only |
| INF-9 | L3 runner + automatic divergence bisection (first divergent frame → subsystem → function by toggling hooks) | todo | finds an injected fault without help | |
| INF-10 | Execution coverage of the original (drcov under Wine, or breakpoint sampling in `sa-hook`) | todo | coverage report of the corpus | F2, F8 |
| INF-11 | D3D9 call-stream capture/diff + audio event log/diff + screenshot capture | todo | same capture twice on the original | modelled on samp-rs `d3d.rs` (reference only) |
| INF-12 | Mutation runner (inject faults into Rust functions, expect detection) | in-progress | F9 smoke test | seed: 3 faults in `solve_quadratic` (feature `mutants`), all caught; gta-reversed's `1/(2a)` is an equivalent mutant |
| VER-1 | Fixes from the L1 verifier (docs/re/verify-l1.md): caller-composite spec with `st0_exact` for GetATanOfXY + force the `_x87` API at the 35 ST0-consuming sites; record the CPU model in CPU-decided fixtures; native-only LimitAngle range 2^24..2^34 in acceptance; check that a 'hang' is a true fixed point; generator emits signed all-zero tuples; static scan flags `fcom … fsqrt … fnstsw` condition-code patterns; FPU control word per spec (captured per call site in L2); widen the compare window (stack writes, full scratch, EAX/EDX by spec) | todo | the verifier probes (`crates/sa-oracle/tests/verifier_l1.rs`) still pass and V-1..V-8 are closed | after MATH-2 merges (same files) |
| INF-13 | Acceptance scripts `tools/accept/*.sh` (F1–F13 + per-milestone) | todo | each exits 0/1 with a readable report | |
| INF-14 | Corpus v1: scripted recordings (routes per zone/time/weather, vehicle classes, combat drills) + SCM setup scripts | todo | manifest + coverage % | grows until F8 |
| INF-15 | Concurrent lanes hang | done | cause: a lost-wake-up race in the ORIGINAL CdStreamSync (.HOODLUM 0x156CD80) vs CdStreamThread (0x406560), ~7% of new-game loads even on one lane; fixed in sa-hook (`cdsync.rs`: polled wait at 0x156CDAE, logged recoveries). Agent: 0/40 hangs with 4 recoveries, 3× pair + 1× triple acceptance PASS; main session: inf3 (lane 1) + inf4 (lane 2) concurrently, 14 + 12 PASS | docs/re/lanes.md. The fix is a harness intervention, not a port (ledger) |
| INF-16 | The L1 native CPU backend (`sa-oracle-native/ref.exe`) writes into the base `wine/prefix` continuously; give it its own prefix (`wine/oracle-prefix`) so lane creation never copies a half-written registry | todo | lane setup while L1 tests run produces a working lane | found by INF-15 |

## Phase II: foundations (M6–M9)

| ID | Task | Funcs (rev/unrev) | Status | Verification |
|---|---|---|---|---|
| FMT-1 | IMG v2, RW chunk stream (DFF/TXD), COL1–4 | — | done | 8 IMG / 21,038 entries, 15,347 DFF, 4,037 TXD (32,869 textures), 253/254 COL byte-exact round trip. peds.col is an expected failure (not loaded as collision; ValidateVersion stamp bytes, see docs/re/formats.md). Open: verify the triangle index order against the exe's RW stream reader before rendering relies on it |
| FMT-2 | IPL text+binary, IDE, all DAT/CFG the exe loads | — | in-progress | part a done: MSVC 7.1 `sscanf`/`fgets`/LoadLine semantics, taken from REA decompiles of 1.0 US. Whole-install test: 0 failures; binary IPL (190), nodes (128), roadblox byte-exact; canonical dumps for part b. Part b todo (after INF-4): compare against the original's in-memory tables (incl. uninitialised fields, float parsing via `__strgtold12`, the obfuscated carcols stub at 0x405FB1). Findings in docs/re/formats.md |
| FMT-3 | IFP, GXT, SCM, carrec, SFX/stream audio containers, saves | — | done | re-run by the main session: game_data_fmt3 5/5 + game_data_audio + earlier tests pass, 86 unit tests. IFP 133 ANP3 + 148 ANPK, GXT 82,997 keys (exe CRC), carrec 426, audio 710 SFX banks + 16 stream packs (XOR key from 0x4F1750), all byte-exact. SCM: 413k instructions decoded by control flow, 0 failures, 1,568 opcodes used (`catalog/script-usage.csv`); per-opcode parameter reads from the exe (`catalog/script-params.csv`). Saves: synthetic only | open: 54 opcode parameter lists picked from data (confirm with L2), save block split needs real saves; see docs/re/formats.md |
| MATH | vectors, matrices, quaternions, rect, col primitives, compressed, CGeneral, CMaths, x87 helpers | 99 rows | in-progress | MATH-2: 99/99 ported in `crates/sa-math`; 96 L1 specs at 10^6 cases with 0 mismatches (79 Unicorn, 17 CPU-decided FSIN/FCOS/FPATAN); main-session spot check of 3 random specs at 1M: PASS. Needs L2: CMatrixLinkList::Init/Shutdown, CMaths::InitMathsTables. GetATanOfXY stays `implemented` (ledger). See docs/re/math-port.md |
| ARCH-1 | Foundation for every port: globals/layouts, typed calls into the original, `#[hook]` registration (`docs/arch/ADR-001-globals-and-layouts.md`), crates `sa-core` + `sa-macros` | — | implemented | agent: sa-core host/i686/wasm32 builds, clippy host+i686, sa-hook tests check every registration against the exe. To accept: main-session rerun; INF-8 L2 of the pilots |
| CORE | pools, timer, memory mgr, file mgr, game/app states | 323 rows | in-progress | 27 pilots (ARCH-1) + PORT-CORE: 270/298 ported, 56 L1, 191 need L2 (CORE-L2 / port-l2.sh), 25 deferred orchestrators, 3 blocked (FPTAN), 23 not hookable; docs/arch/core-port.md |
| CORE-L2 | In-game verification of the 27 ARCH-1 pilots | done | `tools/e2e/port-l2.sh` (new, reusable): 19 pilots L3 (L2 0 mismatches, frame-hash identical with all pilots on Rust, hook-off check); 8 never called in the corpus (save-game callers or dead code) stay implemented; main-session re-run on grove-walk: 0 mismatches. New corpus recording pause-restart. L2 harness fixes: startup calls, heap regions via `SA_L2_EXTRA` (`tools/e2e/l2-extras.txt`) | found: with all 777 ports on Rust the game panics in sa-stream (`modelinfo/vehicle.rs:2037` sscanf_n 38 > 34 targets) → STREAM-FIX-1 |
| STREAM-FIX-1 | sa-stream panic with all ports on Rust: `sscanf_n` has 34 targets, a caller passes 38 (fileloader); fix, then run port-l2.sh on all STREAM ports | todo | `port-l2.sh ports --runs "plain rust"` frame-identical | blocker for STREAM L2 |
| CORE-L2b | L2 in game for the 191 PORT-CORE `implemented` functions (port-l2.sh once CORE-L2 delivers it) | todo | per-function L2 table | |
| MATH-FPTAN | CPU-checked FPTAN model in sa_math::x87 (unblocks App::DoRWStuffStartOfFrame*, AppEventHandler) | todo | L1 0 mismatches vs CPU | |
| STREAM | CdStream, CStreaming, CFileLoader, stores, model infos | 552 rows | in-progress | PORT-STREAM: `crates/sa-stream`, 544/552 ported (504 hooks): 70 L1, 434 need L2, 40 not hookable, 6 deferred, 2 blocked (CdStreamOpen protection VM index; LoadObjectInstance needs a CPU-checked acos); docs/arch/stream-port.md |
| RW | RW core + plugins + pipelines + RW functions from REA (catalogue: 291 rev / 19 unrev / 313 unknown). The statically linked D3DX9 (949) and libpng/zlib (194) stay original in phase A; in phase B they are replaced by Rust equivalents for what RW uses (D3DX shader assembly for the pipelines, PNG reading) | 291/19 (+313) | todo | L2 + D3D diff Spec done (RW-RE): `docs/re/rw/` (life cycle, plugins, object layouts, D3D9 driver + pipelines, port plan with 361 L1 / 1,444 L2 candidates), `catalog/rw-names.csv` (1,829 names: 1,389 high, 203 medium, 237 low). gta-reversed `RwGlobals` lacks `fileFuncs` (size 0x12C, but 0x158 in the exe): use the plugin-sdk/exe layout. RW forces x87 single precision at 0x857404 and again every frame at 0x857432 |

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
| AUDIO | CAE* entities/managers/loaders/hardware; user-track MP3 playback goes through QuickTime (0x4E77C0–0x4E7CB0 → QuickTime.qts), which needs a Rust decoder in phase B | 718/26 | todo | F7 audio |
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
| SAMP-1 | Inventory of the GTA addresses used by samp-rs/samp.dll | done (static) | `docs/samp/samp_usage.py` (deterministic; rerun after catalogue changes) → `catalog/samp-usage.csv`, `docs/samp/`: 622 samp-rs addresses + 384 samp.dll patches classified; 317 directly required functions; static closure E = 10,425 (67 % of 15,486); samp's own disables cut only 182; 55 disabled features; 103 SCM opcodes + 2 custom ones (09C6, 09D3) in the 2500–2599 table | static reachability barely shrinks the engine; the dynamic run (SAMP-1D) decides what is really skipped |
| SAMP-1D | Dynamic measurement: original + samp.dll 0.3.7-R4 on a local open.mp server (our own copy, never samp-rs's), scenario list in docs/samp/README §6, record the executed game functions (INF-10 coverage), and confirm what SA:MP never runs and which opcodes samp.saa scripts use | todo | coverage report in docs/samp/ | needs INF-10 and the game lane; the open.mp server and samp.dll are copied read-only from samp-rs |
| SAMP-4 | "samp mode" engine options for every samp.dll memory patch | todo | F13 |
| SAMP-API | Document the engine API covering every samp-rs game access (proposal for the samp-rs side; we never edit samp-rs) | todo | F13 |

## Decisions made autonomously
- 2026-11-05 (PORT-STREAM): CdStreamThread's port runs game-memory accessors on the streaming thread, as the original does. This is an accepted exception to ADR-001's single-thread rule, because 1:1 requires the original threading.
- 2026-11-05 (INF-7): L1 has two backends. Unicorn decides L1-routed functions. Functions with x87 instructions measured unfaithful under Unicorn (transcendentals, FPREM, FSCALE, FXTRACT) go to L2; until INF-8 exists the real CPU (native backend under Wine) is their interim authority. Signalling-NaN inputs are always decided by the CPU. Unicorn runs with MXCSR.FZ cleared when the code has no SSE.
- 2026-11-05 (INF-7): ports of x87 code use `sa_math::x87::X87` (24-bit significand, CPU-measured NaN rules), never plain f32 arithmetic.
- 2026-11-05 (INF-3): a hook toggle is a 32-bit store into the hook's `TARGET` slot (`jmp [TARGET]`), applied by the game thread at the frame boundary (the CTimer::Update thunk). In-process comparisons run on the game thread so the game's FPU state applies (x87 CW 0x007F = 24-bit precision, MXCSR with FTZ on).
- 2026-11-05: game data = a copy of the Steam 3.0 data set already used by samp-rs (SA:MP files excluded) with the 1.0 US exe. The original and Rust run on the same data, so parity is unaffected. Data-dependent findings may not match retail 1.0 data, and the corpus records the data hashes.
- 2026-11-05: the catalogue's function set is Ghidra functions ∪ gta-reversed hook addresses that decode as
  code, because Ghidra misses vtable-only functions.
- 2026-11-05: gta-reversed's bug fixes (`FIX_BUGS`, `NOTSA` changes) are not ported as default behaviour.
  1:1 keeps the original bug, and fixes sit behind options that are off by default.

## Divergence ledger
Sections: *pending* (blocks "done") · *original UB, not reproducible* · *accepted (user-approved)*.

*pending*
- FSIN/FCOS/FPATAN modelled at 53 bits vs the CPU's 64 (sa_math::x87): every stored/product result matched on the 17 CPU-decided specs; the authority is one Intel CPU (an AMD CPU may differ) (MATH-2).
- Aliased output pointers are not modelled for IntersectRay, IntersectEdge, CRect::GetCenter, ConvertToEulerAngles and matrix ops whose result aliases an operand; explicit in-place variants exist; L2 must confirm the call sites (MATH-2).
- `CGeneral::GetATanOfXY` (0x53CC70) (state set back to `implemented` after verifier V-1: 35 call sites feed the full ST0 into x87 arithmetic; with the f32 API, 4,579/200k differ, while the `_x87` API has 0), path x > 0, y > 0, |x| ≥ |y|: the original returns the raw FPATAN value in ST0 (64-bit significand); `sa_math` has it to 53 bits. The stored f32 matched in all 1,003,364 cases on the CPU; ST0 beyond f32 differs in 78,162. Resolve by an x87-exact atan or by showing every caller stores ST0 to f32 first (call graph).

*original UB, not reproducible*
- `CountCompsInRule` never returns for negative input words (original and port both loop forever) (PORT-STREAM).
- CdStreamSync/CdStreamThread lost wake-up (0x156CD9D–0x156CDAE vs 0x406669–0x406678): the original can hang forever on a loading screen (~7% of new-game loads under Wine). The harness patches the wait (INF-15). The Rust CdStream port will not reproduce the hang; it keeps the observable behaviour of a completed read (decision recorded here).
- CMatrix `operator*(Mat, Mat)`, `operator+`, `Invert(const CMatrix&)` and `operator*=` copy four uninitialised stack dwords (offsets 0xC/0x1C/0x2C/0x3C) into the result. The port writes 0, and those 16 bytes are excluded from comparison (MATH-2).
- `CGeneral::LimitAngle` (0x53CB00) never returns for ±inf and |a| > 2^33 (2^33 itself returns, verifier V-3; `a - 360` rounds back to `a` under PC=24), so the game would hang. `limit_angle` panics there; `limit_angle_checked` returns `None` exactly for those inputs (L1-verified).

## Waiting for the user
- Human playthrough recordings of the missions (PLAN §6.1) are needed for F4. Not needed before M17. The
  recorder (INF-5) has to exist first.
