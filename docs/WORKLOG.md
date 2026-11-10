# Worklog

## 2026-11-05: feasibility study, project start, M0 catalogue (part 1)
- Feasibility research: local gta-reversed, plus web-gta, OpenSA and v86 cloned and inspected. Report in
  `docs/FEASIBILITY.md`.
- User decisions recorded in CLAUDE.md. PLAN.md written.
- `tools/catalog/build_catalog.py`: parses every `RH_Scoped*Install` in gta-reversed (8,635 registrations,
  8,632 distinct addresses, 7,904 reversed / 731 `.Reversed = false`, matching docs/hooks.json's 8,336
  closely enough; the difference is RW and the tooling categories). It also collects 1,340 distinct addresses
  still called into the original (1,343 sites; 944 of them are not hooked anywhere).
- Verified against 1.0 US with capstone: 8,254 addresses decode as normal code in game `.text`. 379 are
  `jmp` stubs into `.HOODLUM`. There is 1 placeholder (`0xDEADBEEF`, CPedAttractor::GetNoOfRegisteredPeds)
  and 1 outside game `.text`.
- `tools/catalog/hoodlum_xrefs.py`: game `.text` has 501 `jmp` and 0 `call` instructions into `.HOODLUM`, so
  hooking at the stub address is sufficient on 1.0 US.

## 2026-11-05: M0 catalogue (part 2)
- REA/Ghidra snapshot of 1.0 US: 20,823 procedures + 214 imports (KERNEL32 137, USER32 44, VORBISFILE 7,
  WS2_32 7, ADVAPI32 6, WINMM 4, OLE32 3, EAX/GDI32/D3D9/DINPUT8/DSOUND/DDRAW 1 each). The evidence bundle
  was exported to `catalog/raw/`.
- `tools/catalog/merge_ghidra.py` → `catalog/procedures.csv`. Game `.text` holds 18,598 functions: 7,894
  reversed in gta-reversed, 730 hooked but unreversed, 528 only called through plugin::Call/thunks, and 9,446
  unknown (see ROADMAP CAT-5 for the breakdown).
- Ghidra missed 1,890 hook targets (861 of them VMT installs). The catalogue therefore takes the union.

## 2026-11-05: master plan and roadmap
- Wrote `docs/PLAN.md`: definition of done F1–F13, architecture/crates, phases M0–M23, standard loop,
  no-guessing rule, parity loop, verification levels and function states, corpus, scheduler, concurrency
  and escalation.
- Wrote `docs/ROADMAP.md` with the task IDs (CAT, INF, FMT, subsystems, STD, WEB, REL, SAMP).
- `tools/catalog/subsystems.py`: every catalogued function is assigned to a subsystem
  (`catalog/subsystems.md`; owner rules take precedence over gta-reversed categories). Script commands in
  gta-reversed: 781 handlers, 716 unimplemented, 283 NOP.

## 2026-11-05: goal set; INF-1 partial; agents started
- Initial commit 28ba9b1.
- INF-1: copied the Wine prefix (d3dx9 + DXVK) and the GTA files (no SA:MP files) from samp-rs, read-only,
  into `wine/prefix` and `wine/gta-sa`, with the 1.0 US exe. Added `tools/wine-env.sh` and the Cargo
  workspace. Lanes are still to be created.
- Started: CAT-5 (RE agent), FMT-1 (impl-1, worktree), INF-2/INF-3 (impl-2, worktree).
- CAT-8: `tools/catalog/layouts.py` → `catalog/layouts.csv` (781 size and 39 offset assertions; 20 symbolic
  expressions such as `sizeof(void*)` are kept verbatim).
- CAT-10 part 1: `tools/catalog/script_commands.py` → `catalog/script-commands.csv`. 2,826 enum opcodes:
  770 handlers, 713 unimplemented, 281 NOP, 13 unsupported, 1,049 not registered in gta-reversed.

## 2026-11-05: FMT-1 integrated
- `crates/sa-formats` (impl-1): IMG v2, RW chunk tree, typed DFF/TXD, COL1–4. Re-verified by the main
  session: `cargo test -p sa-formats` gives 18 passed; `SA_GAME_DIR=… cargo test -p sa-formats --release
  --test game_data -- --ignored` passes with every IMG/DFF/TXD/COL byte-exact and 1 expected failure
  (peds.col). Clippy is clean; the crate builds for wasm32. Findings are in `docs/re/formats.md`.

## 2026-11-05: CAT-5 integrated
- The RE agent classified all 9,454 unknowns (see ROADMAP CAT-5 and `docs/re/cat5-unknowns.md`).
  Re-verified by the main session: the full pipeline build → merge → classify → subsystems reproduces
  `procedures.csv` byte-identically. The real remaining backlog is 2,793 unknown game/RW functions, plus
  730 hooked-unreversed and 528 called-only.
- plugin-sdk cloned shallowly into `re/plugin-sdk` (15f15b6) for naming.

## 2026-11-05: INF-2 + INF-3 integrated
- `crates/sa-launcher`, `crates/sa-hook`, `tools/e2e/lib.sh`, `tools/e2e/inf3-hooks.sh` (impl-2). Details in
  the ROADMAP INF-3 row. Re-verified by the main session: i686 build and clippy clean; host tests pass
  (decoder differential against iced-x86); `inf3-hooks.sh` on lane 1 all PASS (menu, gameplay screenshot
  checked visually, toggles, storm, selftest 0 mismatches, no crash). The agent's 600 s soak passed too.
- Measured: about 25 fps in gameplay under Wine/DXVK headless, about 63 fps in the menu. Not investigated
  yet.

## 2026-11-05: FMT-2 part a integrated
- `sa-formats` data modules (impl-1 plus 3 forked helpers): every data definition file gta_sa.exe loads,
  parsed with the exe's own line reader and an MSVC `sscanf` model. Re-verified by the main session: 63 unit
  tests pass; `game_data_defs` and `game_data` ignored tests pass on the install; clippy clean.
- Original quirks kept as-is (1:1), e.g. the trailing space on gta.dat paths breaks the `occl` interior
  test, `path` sections end text IPL reads, timecyc `dir_mult` is never assigned, and 11 vehicles keep
  uninitialised wheel scales (gta-reversed sets 0.7, which is a fix). Details in docs/re/formats.md.
- Process note: this agent used REA while CAT-7 was also running. REA is single-user (PLAN §8), so briefs
  now say explicitly whether the agent may use REA.

## 2026-11-05: CAT-7 call graph, read/write sets, extents, port order (RE agent)
- `tools/catalog/callgraph.py` (`ghidra`, `build`, `spotcheck`, `analyse`) and
  `tools/catalog/ghidra/DumpCat7.java`. Ghidra 12.1.2 headless: fresh import + auto-analysis into
  `target/ghidra` (≈160 s), then a `-readOnly` dump. The dump removes the CAT-5 mis-split/protection
  procedures, clears the wrong non-returning flag on `_free` and its thunks (800 truncated callers),
  creates 2,044 functions (all missed hook targets, 0 failures), and heals undecoded branch targets.
  Analysis in Python/capstone.
- Outputs: `catalog/extents.csv`, `callgraph.csv`, `rwsets.csv`, `indirect-calls.csv`,
  `port-order.csv` (all < 2.3 MB). Totals and blind spots: `docs/re/cat7-callgraph.md`.
- Verified: rerun twice and once after deleting the Ghidra project; all outputs byte-identical. Spot
  check vs gta-reversed on 2×10 random functions: 11/15 calls and 35/36 globals found. Each miss was
  checked: 4 are inlined in the original, 1 is a checker false positive.
- Found: 153 code addresses that are called or are vtable slots, but missing from `procedures.csv`
  (`catalog/raw/cat7-noncatalogue-targets.csv`).
- Cost: ~10 min of Ghidra CPU; `target/ghidra` 112 MB, `catalog/raw` 10 MB.
- Main session re-verified CAT-7: `callgraph.py build` reproduces all 5 CSVs (sha256 identical);
  `spotcheck 5 99`: globals 7/7, calls 3/5 (misses explained as original inlines in the agent's notes).
  Added CAT-12 (153 targets missing from the catalogue) and CAT-13 (virtual call resolution).

## 2026-11-05: session restart; OOM note
- The Claude Code process exited while 4 agents were running. All of them were resumed from their
  transcripts. The journal has no kill record for the Claude process. The only OOM event (04:07) was
  triggered by an `sa-oracle` allocation and killed a 10.7 GB python process. Added a memory-cap rule to
  PLAN §8 and told the INF-7 agent.

## 2026-11-05: CAT-9, CAT-10 part 2, CAT-11, CAT-12 (RE agent, no REA)
- CAT-12: `merge_ghidra.py` now adds every evidenced function entry that Ghidra missed: hooks incl.
  `.HOODLUM` stubs (1,894), gta-reversed callees (416), gta-reversed vtable slots (351), and CAT-7 call
  targets (46, `catalog/extra-code-addresses.csv`, appended by the new `callgraph.py gaps` fixpoint step;
  bootstrap took 3 passes). SecuROM return-address thunks in `.HOODLUM`/protection `.text` (49 targets,
  75 sites) are classified `excluded:protection-thunk`. Result: 0 call/tail-call/vtable targets outside
  the catalogue; all 153 earlier gaps are rows. New `tools/catalog/vtables.py`: slot layouts rebuilt from
  gta-reversed headers and validated against hooks, slot counts and inline `return <int>` bodies (328/379
  vtables), so vtable slots get names such as `CEventGunShot::GetEventType`.
- CAT-9: `.HOODLUM` bodies are `stub-body` rows counted at their stub. Totals reconcile (19,425 everywhere);
  the reconciliation section in summary.md is asserted by `subsystems.py`.
- CAT-10 part 2: handler table 0x8A6168 (27 range functions) and all 27 switches decoded; per opcode case
  address / range function / default. Verified: 35/35 gta-reversed case annotations inside the decoded case
  blocks, plus 5 handlers checked by hand.
- CAT-11: `catalog/state.csv` + `tools/catalog/progress.py` (merge / set / report) → `catalog/progress.md`.
  The 3 sa-hook hooks are `implemented`; F1 = 0 / 12,761.
- `tools/catalog/run_all.sh` regenerates everything. Verified: two consecutive runs give sha256-identical
  `catalog/` (23 files); the Ghidra dump log (timestamps) moved to `target/ghidra/dump.log`.
- Notes: `docs/re/cat9-12-pipeline.md`. Cost: ~10 pipeline runs × ~1.5–3 min CPU; no new disk use
  beyond `target/catalog-logs`.
- Main session re-verified CAT-9/10b/11/12: `run_all.sh` under an 8 GB memory cap reproduced every
  catalogue file sha256-identically. Set CTimer::Update back to `specified`, because a passthrough hook is
  not an implementation (PLAN §5).

## 2026-11-05: INF-7 integrated
- `crates/sa-oracle` (L1 harness: Unicorn backend + native CPU backend under Wine, guard rails, seeded
  runner with shrinking, x87 fidelity probes, mutants) and `crates/sa-math` (x87 model + 6 functions).
  Re-verified by the main session under a 6 GB cap: sa-math 11/11, sa-oracle 14/14; `l1 SolveQuadratic
  --cases 1000000` PASS (peak RSS 509 MB); mutants caught 3/3. Workspace build and i686 sa-hook build
  green.
- OOM root cause (agent): the `fidelity` command held 2.52M probe results in RAM at once (2.7 GB) next to an
  unrelated 10.7 GB python process. Now chunked (peak 1.3 GB), and threads are capped.
- Ledger: 1 pending (GetATanOfXY ST0 precision beyond f32), 1 original-UB (LimitAngle hang).

## 2026-11-05: FMT-3 integrated
- sa-formats: IFP (ANP3/ANPK), GXT, carrec, saves (structure), SCM (headers, script.img, control-flow
  decoder with parameter reads taken from the exe's handlers), audio containers (CONFIG, SFX banks, XOR'd
  stream packs with Ogg framing). Re-verified by the main session: all ignored whole-install tests pass,
  86 unit tests, clippy clean. CAT-10 part 3 (opcodes used by main.scm/script.img) is now
  `catalog/script-usage.csv`.

## 2026-11-05: CAT-14 gta-reversed fidelity audit
- `tools/catalog/gr_deviations.py` (run by `run_all.sh` before `progress.py`): lexes `source/game_sa` and
  `source/app`, parses function extents, maps markers to hook addresses (overloads by signature, `// 0xADDR`
  comments, script handlers → range functions) and writes `catalog/gr-deviations.csv` (3,997 rows) and
  `catalog/gr-deviations-summary.json`. Hand checks live in `catalog/gr-deviations-notes.csv`. `progress.py`
  merge adds the `state.csv` column `gr_deviations` (rows with deviates yes/maybe; never changes states or
  `updated`). New Ghidra post-script `tools/catalog/ghidra/DecompileList.java` (read-only).
- Counts: keep-original/yes 840, keep-original/no 184, unclear 1,238, harmless 1,735. Reversible bug fixes:
  26 defined (+1 used but undefined), 53 use sites, enabled by default in gr. Script opcodes: 15 default +
  70 unimplemented-with-case.
- Spot checks (Ghidra decompile + capstone): 0x6FD5B0, 0x4EF2E0, 0x42CD10, 0x64E550, 0x7189B0, 0x5F3970,
  0x66C3E0, 0x7295E0, 0x6E1A60, 0x4D0C50 and 0x407180/0x41BD90: all 11 match gr's description of the
  original.
- Findings: gr's integer and float `GetRandomNumberInRange` do not reproduce the original formula
  (375 call sites tagged); `GetAllValid` walks pools upwards, while the original often walks downwards.
  The catalogue lacks 24/27 ProcessCommandsXToY and 58/65 cheat functions (data-referenced code, not closed
  by CAT-12). `RH_ScopedClass` is now block-scoped in `build_catalog.py`: it fixed the names of 75
  CVehicleModelInfo hooks; no state changed.
- Verified: `run_all.sh` twice under an 8 GB cap → sha256-identical `catalog/`. Cost: ~4 pipeline runs
  (~3 min each) plus one Ghidra decompile run; no new disk use beyond the scratchpad.
- Main session re-verified CAT-14: `run_all.sh` reproduces `catalog/` sha256-identically. Sent the
  GetRandomNumberInRange finding to the MATH-2 lane. Follow-up CAT-12b: about 24 ProcessCommands* range
  functions and 58 cheat functions are reached only through data tables and are missing from the
  catalogue.

## 2026-11-05: INF-4/5/6 integrated
- sa-hook determinism mode (oracle.rs, detwin.rs, detaudio.rs, arena.rs; 11 hooks installed only with
  `SA_DET*`/`SA_HASH`), `crates/sa-oracle-format` (hasher, recording + trace formats, `satool`),
  `tools/e2e/inf4-determinism.sh`. Re-verified by the main session: workspace tests pass, i686 build;
  `inf4-determinism.sh` exit 0 (identical hashes, perturbation detected at frame 3700);
  `inf3-hooks.sh` (determinism off) exit 0 when run alone.
- Found: running lane 1 (real-time mode) at the same time as the lane-2 determinism run hung lane 1 on the
  loading screen. New task INF-15; until it is fixed, run one game lane at a time.

## 2026-11-05: L1 verifier report integrated
- Verifier (adversarial) on INF-7 + sa-math: 5 of 6 claims stand. GetATanOfXY goes back to `implemented`
  (V-1: 35 callers consume the 64-bit ST0). 15 new mutants, all caught; branch coverage complete. Probes
  in `crates/sa-oracle/tests/verifier_l1.rs`; re-run by the main session at reduced size: 5/5 pass.
  Report: docs/re/verify-l1.md. PLAN §5 L1 definition amended (CPU-decided evidence, caller-used outputs,
  FPU state). Ledger wording corrected (> 2^33). Follow-ups as VER-1.

## 2026-11-05: CAT-12b data-referenced code
- New `tools/catalog/data_refs.py`, which runs in the `run_all.sh` fixpoint loop after `callgraph.py gaps`.
  It scans aligned dwords in `.rdata`/`.data`/`_TEXT_HA`/`_rwdseg`, plus the code immediates that
  `callgraph.py build` now writes to `catalog/raw/cat7-code-immediates.csv`, for values in game code that
  are not catalogue entries. A value is accepted only when the bytes show an entry (16-byte aligned,
  preceded by padding, a ret or a switch table, a clean decode, not inside a Ghidra body except where an
  absorbed entry is proven) and the data gives context: a pointer table, a vtable install, the
  `RxNodeDefinition`/`RwDevice` layout, a struct array, a pointer variable, or an immediate.
  `merge_ghidra.py` reads the result, `catalog/data-code-addresses.csv` (rule `data-<kind>`).
  Reports: `catalog/data-tables.csv` and `catalog/data-refs.csv`. Notes: `docs/re/cat12b-data-tables.md`.
- Result: 2,720 entries. 1,666 are C++ static initialisers (`__xc`), 595 code immediates (callbacks,
  `atexit` destructors), 343 slots of 164 vtables unknown to gta-reversed, 58/65 cheats, 24/27
  ProcessCommands ranges, and 34 others (RW node defs and device, anim-callback table, d3dx9 tables,
  zlib config, QuickTime glue). Another 16 CAT-7 call targets came up in the new bodies. Catalogue
  19,425 → 22,161 functions; live 12,761 → 15,485.
- Verified: the convergence run took 5 passes. A second `run_all.sh` closed in 1 pass and gave
  sha256-identical `catalog/` files. Data/immediate entries outside the catalogue: 0.
  `noncatalogue_targets`: 0. The cheat table equals gta-reversed `Cheat.cpp` (92/92 slots). The
  script table equals plugin-sdk (27/27). Rejected candidates were reviewed by hand (see the notes).
- Cost: about 4 full pipeline runs (8 min the first time, about 3 min each after that) under an 8 GB cap,
  plus short Python scans. Disk: +0.7 MB in `catalog/`.

## 2026-11-05: RW-RE integrated
- The RE agent (REA + headless Ghidra) named every RW catalogue row and all 862 RW thunks
  (`catalog/rw-names.csv`, 1,829 rows) and wrote the RenderWare spec in `docs/re/rw/`. Key facts:
  gta-reversed's RwGlobals is 0x2C short (no fileFuncs); plugin-sdk misnames 0x4C7510; part of the D3D9
  driver lives at 0x4C9970–0x4CD820; RW re-forces x87 PC=24 every frame (0x857432). Open: plugin offsets
  were derived, not measured; the transform kernel (x87/SSE/SSE2) on the oracle machine; 0x7FFF00 has no
  catalogue row.

## 2026-11-05: INF-8 integrated
- L2 oracle (sa-hook l2/l2win, sa-oracle-format l2/normalise, sa-oracle l2 CLI, tools/e2e/inf8-l2.sh).
  Re-verified by the main session: workspace tests pass (I fixed a sa-hook test that assumed the last
  column of procedures.csv), and `inf8-l2.sh` passes everything. L2 reached in game:
  LimitRadianAngle, GetRadianAngleBetweenPoints, SolveQuadratic, CVector::Normalise and
  IsCoopGameGoingOn (1,000 cases each, 0 mismatches). GetATanOfXY: f32 equal, 517/1,000 ST0 cases on the
  pending ledger entry. LimitAngle had no call in the corpus. The state.csv update waits until CAT-6
  finishes regenerating the catalogue. New: CAT-15 (x87 stores missing from rwsets).

## 2026-11-05: SAMP-1 (static) integrated
- samp-rs (read-only) and samp.dll patches classified against the catalogue: `docs/samp/`,
  `catalog/samp-usage.csv`. Result: SA:MP keeps the whole frame loop, so the static closure is 67% of the
  catalogue; samp's disables remove only 182 functions. A real reduction needs the dynamic measurement
  (SAMP-1D). New: CAT-16 (fall-through edges).

## 2026-11-05: CAT-6 names + CAT-13 typed virtual calls (measurement agent)
- New `tools/catalog/names_extra.py`, which runs in the `run_all.sh` loop after `data_refs.py` (exit 3 =
  the names changed and the loop reruns). It writes `catalog/names-extra.csv` from these sources:
  gta-reversed/plugin-sdk one-statement wrappers, `// 0xADDR` comments, plugin-sdk `Converted from` and
  `#define FUNC_`, the `__xc` static initialisers (`__init_<global>`) and their `atexit` callbacks
  (`__dtor_<global>`), the classes of unknown vtables (installing constructor, slot-0 deleting
  destructor, header `RH_ScopedVirtualClass`, unique-slot owner) and their slot names from validated
  layouts. Low-confidence hints (`sub_<addr>_from_<caller>`, string) are written but never used as
  names. `catalog/rw-names.csv` is now the RW name source (high/medium). Precedence: gr hook >
  rw-names > CAT-5 > CAT-6. That keeps plugin-sdk's wrong `RwTextureRead`@0x4c7510 out. The final
  name goes into `procedures.csv` (`name`, `name_source`, `name_confidence`, `role`, `name_hint`,
  written by `subsystems.py`) and `state.csv` (`name` + new `role` column; low hints only in the
  evidence). `callgraph.py` uses the final names.
- Result: unnamed non-excluded rows 5,862 → 1,656 (non-RW 4,354 → 1,418, RW 1,508 → 238). States:
  specified 9,727 → 13,919, unknown 5,751 → 1,560. CALLED-ONLY 944 → 37 (853 RW thunks moved to RW).
  Added 0x750480 `_rpWorldObjCameraSync` via `extra-code-addresses.csv`; 0x7fff00 was already a row.
  Refreshed the rw-names `[catalogue: …]` tags (`names_extra.py refresh-rw-tags`, 1,318 rows, tags only).
- CAT-13: new `tools/catalog/cat13_types.py` computes class layouts from the gta-reversed headers; it
  keeps a layout only when the size and offsets equal VALIDATE_SIZE/VALIDATE_OFFSET (604 classes).
  `callgraph.py` dataflow gained the tokens OBJ/VTC/SLOTC from members, typed globals and return
  types. Results: 596 / 3,092 virtual-pattern sites typed (134 with edges `virtual-typed[-override]`,
  in the port order; 462 wide), unresolved indirect sites 5,566 → 4,971. Fixed: x87 stores were
  recorded as reads in `rwsets.csv`. gta-reversed vtables declared in headers are now used.
- Verified: two `run_all.sh` runs give sha256-identical `catalog/` (excluding rw-names.csv and
  samp-usage.csv, which another agent wrote). Spot checks against the bytes and gta-reversed: 15
  random new names (seed 6) and 15 non-initialisers (seed 21), all consistent. The source extractors
  were measured against already-known names: gta-reversed comments agree 98.5 %, plugin-sdk wrappers
  97.6 %, so plugin-sdk-only names are medium. Found that gta-reversed `TaskComplexClimb.h` has the
  Clone/GetTaskType comments swapped; the validated layout wins.
- Cost: about 7 full pipeline runs (2.5–5 min each, 8 GB cap), plus short scans. Disk: about +1 MB in
  `catalog/`. Notes: `docs/re/cat6-names.md`.

## 2026-11-05: CAT-6/13 integrated, CAT-15 fixed, first L2 states
- CAT-6 (agent): unnamed non-excluded functions fell from 5,862 to 1,656 (init/dtor 1,804, gta-reversed
  comments 459, vtables 562, …); CAT-13 typed 596/3,092 virtual sites (134 resolved). The main session
  re-ran `run_all.sh`: identical. The agent's x87-store fix did not take effect (capstone marks `fstp m32`
  as R, not "no flags"). Fixed in callgraph.py (CAT-15); verified on 0x561B10/0xB7CB50.
- state.csv: L2 for LimitRadianAngle, GetRadianAngleBetweenPoints, SolveQuadratic, CVector::Normalise,
  CGameLogic::IsCoopGameGoingOn (INF-8 evidence).

## 2026-11-05: MATH-2 integrated
- 99 MATH functions ported (`crates/sa-math`), 96 L1-verified at 10^6 cases (state.csv: 91 at L1 + 4 at L2
  + GetATanOfXY implemented). Harness extensions in sa-oracle (0x500-byte scratch compared whole, fake TIB,
  image patches, content-hashed native files, the FSQRT condition-code hazard in the scan). Re-verified by
  the main session after merging with INF-8 (one conflict in lib.rs): sa-math/sa-oracle/sa-hook tests pass
  (incl. verifier_l1 5/5), and 3 random specs at 1M cases PASS. Ledger: 2 pending + 1 original-UB entries
  added.
## 2026-11-05: ARCH-1 (impl-1, worktree): globals/layouts/orig/hook foundation + CORE pilots
- Design: `docs/arch/ADR-001-globals-and-layouts.md`. Game state through `sa_core::mem::Global<T>`
  (`globals!`: address + type + image initial value; fixed address in phase A, Rust-owned static
  otherwise) and `GPtr<T>`; calls into possibly-original functions through `orig!` (original address
  in phase A so the callee's hook decides, Rust path in B/C, `site` form for import call sites);
  `#[hook]` proc macro (`crates/sa-macros`) emits the x86 shim + a `HookReg`; `sa_core::hooks::ALL` is
  appended to sa-hook's table (group `Port`, installed only with `SA_HOOK_PORTS`). Layout constants and
  asserts generated by `tools/gen/layouts_rs.py` from `catalog/layouts.csv`.
- Pilots (27 functions, state `implemented`, list in `docs/arch/pilot-ports.md`): CGeneral
  GetRandomNumberInRange int/float, CPool New/GetAtRef/GetAt/Delete/GetRef for peds, vehicles,
  objects (incl. CPools::Get*/Get*Ref and CPed operator new/delete), CTimer except Update. Ported from
  the 1.0 US bytes; new gta-reversed deviations found (pool search start, missing bounds checks,
  CTimer::Initialise low-dword divider, UpdateVariables accumulation).
- Harness: sa-oracle specs may patch the image and allow extra chunks (`specs::Env`); `RetKind::Eax`.
  sa-hook: `relocate_keep_return` keeps a stolen trailing call's original return address; rand/srand
  caller attribution via `orig::at_site`. sa-math: `X87::fild`, `X87::ftol`.
- Verified (agent): 7 L1 specs at 10^6 cases, 0 mismatches; Unicorn = CPU on 200k non-SNaN cases each;
  gta-reversed's formulas and an off-by-one pool search are caught; sa-hook tests check every
  registration against the exe (bytes, names, relocation, inbound branches, no overlap). Builds: host,
  i686 (sa-hook DLL), wasm32 (sa-core); clippy -D warnings host + i686. The game was not run.
- Merged with master (INF-8, MATH-2): sa-core's GetRandomNumberInRange hooks now call MATH-2's sa-math
  implementation (its two L1 specs replace mine); `X87::fild/ftol` dropped for MATH-2's
  `from_i32`/`ftol`; the CORE pool specs use MATH-2's Spec shape and extents-based allow-list (no
  `specs::Env`); Port hooks carry an L2 `Sig` (conv, stack words, return kind, `this` type) so INF-8's
  `l2` mode applies to them; sa-hook table = 19 local + 27 ports.

## 2026-11-05: ARCH-1 integrated (squash of fa3365b)
- `crates/sa-core` (globals!/GPtr/GameData, orig! calls, FpuGuard, layouts.rs generated from
  catalog/layouts.csv), `crates/sa-macros` (#[hook] with ABI shims and L2 signatures), 27 pilot ports
  (pools, CTimer except Update, GetRandomNumberInRange through sa-math). ADR:
  docs/arch/ADR-001-globals-and-layouts.md. Re-verified by the main session on master: i686 + wasm32
  builds, all host tests, `--test l1` twice (15/15 both times; the agent saw 3 unexplained failures once,
  probably the content-hashed native files shared with a concurrent run; watch for it), clippy
  --workspace --all-targets clean. In-game verification is CORE-L2 (needs the game lane).

## 2026-11-05: CAT-16 fall-through into another catalogue entry (measurement agent, main checkout)
- `callgraph.py build` now lists every live-body instruction whose flow runs on into another entry
  (`catalog/fallthrough.csv`). Each case is decided as `noreturn-call` (non-returning set: named MSVC
  runtime + `exit` by shape + fixpoint, 28 functions), `fallthrough` (the target has its own
  references, so an edge is added), `mis-split` (no other reference, so it is merged), or `undecided`
  (the pipeline stops). The new `callgraph.py fallthrough` step in the `run_all.sh` fixpoint appends
  merges to `catalog/fallthrough-splits.csv`. `classify_unknowns.py` makes those rows
  `excluded:mis-split` (hooks included), and `subsystems.py` sets the state of such hooks to excluded.
  `fallthrough` was added to ORDER_KINDS and to `docs/samp/samp_usage.py` FOLLOW (only change there).
- Result: 7 mid-function entries merged (WinMain's 0x7487cf `Input::Initialise`, three gta-reversed
  hooks at wrong addresses (0x43a7a0 mid-instruction, 0x4c88f4 and 0x732307 on a `ret`), two naked
  WindowedMode patch points, one Ghidra split in `CAEWaterCannonAudioEntity::Service`). 1 dead fall-out
  after `exit(1)`. 0 real fall-through edges. WinMain 206 → 2,023 B, and CGame::Initialise is
  reachable from it (474 → 3,654 functions reachable). Live functions 15,486 → 15,479.
- Verified: 10 spot checks with disassembly + gta-reversed source (docs/re/cat16-fallthrough.md);
  `run_all.sh` twice → 35 catalog files sha256-identical; `samp_usage.py` rerun (B 6,037 → 6,054,
  D 6,615 → 6,629). Cost: 4 full pipeline runs (~2.5–7 min each, under an 8G scope), no Ghidra re-analysis.
- Main session re-verified CAT-16: two `run_all.sh` runs identical, sa-hook tests pass. Moved the RW name
  of 0x732307 (a ret inside the function) to 0x7322F0 in rw-names.csv. Follow-up for INF-8:
  `sa-oracle/src/l2.rs` closures should also follow `fallthrough` edges (none exist yet).

## 2026-11-05: INF-15 integrated
- Root cause of the lane hang: a lost-wake-up race inside the original's CdStreamSync. It happens even
  with one lane (~7%). sa-hook's `cdsync.rs` patches the wait. The agent also fixed the inf3 selftest
  (x87 model run under the game's FTZ). The main session merged it with ARCH-1 (conflict in win.rs) and
  re-verified: tests and i686 clippy pass; inf3 on lane 1 and inf4 on lane 2 at the same time both pass.
  PLAN §8 now allows several lanes. Ledger: original-UB entry. New task INF-16.

## 2026-11-05: RE-SPEC-1 integrated
- 392 full specs and 168 deviation notes for STREAM/COLL/PHYS/WORLD (`docs/re/specs/`,
  `catalog/specs-index.csv`). Main-session spot check against the bytes: CPickups::PictureTaken
  (0x456C57 `add edi,0x64` to Players[0].money; 0x456CD8 `+0x186A0` bonus to PlayerInFocus) matches the
  spec. New tasks CAT-17 (name corrections) and RE-SPEC-2.

## 2026-11-05: PORT-CORE integrated
- 270 of 298 remaining CORE functions ported in `crates/sa-core` (248 new `#[hook]`s; four forks plus the
  agent): 56 L1 at 1M cases, 191 need L2 in game, 23 ported but not hookable, 25 deferred
  (CGame/Game/platform orchestrators), 3 blocked (FPTAN has no CPU-checked model). New `orig!` forms
  (cell/vslot/table/dyn). The pilot `CPed::operator delete` had a read/write order bug, now fixed (found
  by the 0x536DD0 L1 spec). 56 new keep-original deviations are listed in docs/arch/core-port.md.
- Re-verified by the main session on master: i686 + wasm32 builds, sa-core 58, sa-hook 31, sa-math 14,
  `--test l1` 15/15, clippy host + i686 clean; 3 random new L1 specs at 1M PASS. state.csv: 147 L1 and
  240 implemented in total.

## 2026-11-05: PORT-STREAM integrated
- New crate `crates/sa-stream` (504 hooks; three forks plus the agent): 544/552 STREAM functions ported;
  70 L1, 434 need L2, 40 not hookable, 6 deferred, 2 blocked. The CdStreamSync port does not reproduce
  the lost wake-up (ledger). Re-verified by the main session on master: i686 + wasm32 builds,
  sa-stream 10, sa-core 58, sa-hook 31 (incl. install/catalogue checks), `--test l1` 15/15, clippy host +
  i686 clean, 3 random STREAM specs at 1M PASS. state.csv: 217 L1, 5 L2, 714 implemented.
- Open (from the agent): values decided at run time by the protection VM need L2 confirmation;
  uninitialised locals passed as 0; CPedModelInfo::m_pColNodeInfos address (0x8A630C vs gta-reversed
  0x8A6308); globals placed in the wrong owner modules; the OLD_KEY_STATE/PAD0_OLD_CROSS overlap in
  sa-core.

## 2026-11-05: RE-SPEC-2 integrated
- 405 full specs + 156 deviation notes for ANIM/CAM/INPUT/PED/WEAPON/VEH (docs/re/specs/,
  catalog/specs-index.csv). Many gta-reversed logic errors were confirmed against the bytes (e.g.
  CShotInfo::Update uses 0.075f where gr has 0.0075f, which the main session re-checked; CPed::SetAmmo
  uses min, not max; AddGunshell makes an extra rand() call). The spec tooling is now tracked in
  tools/specs/.

## 2026-11-05: CORE-L2 integrated
- `tools/e2e/port-l2.sh` (reusable L2/L3/hook-off runner for any set of port hooks), the
  `record-program.sh` recorder, the pause-restart corpus recording, and the L2 harness fixes (game thread
  known at attach; heap regions via SA_L2_EXTRA). 19 pilots → L3. The main-session re-run of grove-walk
  (plain/l2/rust) gives 0 mismatches everywhere; the script exits 1 only because of the per-frame
  sampling cap on 2 hooks, which the agent's solo runs covered completely. F1 progress is now in
  catalog/progress.md.
- Found: the game panics in sa-stream with all ports on Rust (STREAM-FIX-1).

## 2026-11-05: INF-9 / INF-10 integrated
- INF-9 (done): `sa-oracle l3` + `tools/e2e/l3.sh`: per-frame L3 comparison against a cached all-original
  reference, automatic bisection to the minimal culprit with a ready L2 command, and the hook-off check.
  Main-session verification after merging with CORE-L2 (one conflict in sa-hook l2.rs, both sides kept):
  tests and clippy pass; `l3.sh --selftest --rust <8 sa-hook + 27 pilot hooks>` with hook-off for 2
  hooks: clean set matches over 5,200 frames, the injected LimitRadianAngle fault is found at frame 210
  without help (PASS). The default `--selftest` (every port on Rust) currently stops at the STREAM-FIX-1
  panic.
- INF-10 (in progress): DynamoRIO does not run under this Wine 11 (WoW64). Breakpoint coverage instead
  (169,618 entry/block points, `sa-hook/coverage.rs`, `sa-oracle coverage`). The grove-walk coverage
  run still stops in the loading-screen fade loop because entry stubs are read as data (one point,
  0x55C410, is excluded; at least one more remains; `tools/e2e/cov-bisect.py` finds them). Partial
  coverage (boot to loading screen): 3,084/15,479 functions.
- Agent process note: one `pkill -f` on its own processes (against PLAN §8); no damage.

## 2026-11-07: resumed; harness display fix; STREAM-FIX-1 part 1
- Resumed from 1d26881 at the user's request (ROADMAP banner replaced).
- Every game run exited before frame 1 (`D3D9: EnterFullscreenMode: Failed to change display mode`) on
  lanes 1 and 2, also with every hook on the original. Cause, measured with WINEDEBUG=+system and a
  small EnumDisplaySettings probe: `winepath` in `e2e_game_start` ran before `env -u WAYLAND_DISPLAY
  DISPLAY=<lane>`, so it started the Wine session (explorer.exe) on the user's desktop; the session
  then reported the desktop's modes (7680x2160@82, 800x600@240), GetBestRefreshRate chose 240 Hz and
  the mode change failed. Directly launched gta_sa.exe on the lane display got 1280x720@60. Fix:
  `lib.sh` clears the desktop display variables and exports only the lane DISPLAY. After the fix the
  all-original grove-walk run reaches 5,200/5,200 frames.
- STREAM-FIX-1: `sscanf_n` widened to 38 targets (the 0x8698F8 format has 38 conversions, read
  from the exe). The all-ports run then crashed in .HOODLUM 0x1562440 (CStreamedScripts::Initialise
  body) writing to 0x44C0: the Rust CStreaming::Init2 called 0x470660 as cdecl without `this`; the
  original sets ecx = 0xA47B60 at 0x5B8DEF. Fixed to thiscall. A static audit of every non-thiscall
  `orig!` declaration into game .text (callee reads ecx before writing it, `push ecx` ignored) found
  no other case (5 hits, all `or ecx, -1` writes).
- Verification: sa-hook/sa-launcher i686 release build, `cargo test -p sa-stream` 10/10, clippy host +
  i686 (sa-stream, sa-hook) clean, wasm32 build of sa-stream OK. `port-l2.sh ports --runs "plain
  rust"` (lane 2): plain 5,200 frames, rust 5,200 frames, no crash/panic; L3 compare: 15/17
  subsystems identical over all frames, `rand` + `randcalls` diverge from frame 1.
- Open: bisect the rand divergence (`l3.sh --rust ports`, lane 3, running); STREAM-FIX-1 stays
  in-progress until the all-ports run is frame-identical.

## 2026-11-07: CAT-17 integrated (sub-agent, worktree cat17)
- Name/extent corrections from RE-SPEC-1/2 through two new evidence-carrying inputs
  (`catalog/name-corrections.csv`, `catalog/extent-seeds.csv`); pipeline readers in names_extra.py,
  subsystems.py, progress.py, callgraph.py (also `SA_GHIDRA_PROJ`, since Ghidra rejects paths with '.'
  elements). Agent evidence: two run_all.sh runs sha256-identical over 46 catalog files; 113 names, 9
  unknown → specified, 4 subsystem moves, F1 unchanged.
- Main-session verification: patch applied to master cleanly; byte spot checks (0x6C7D10 vptr store +
  jmp CAutomobile dtor, 0x6F54D0, CTimeModelInfo vtable 0x85BCB0 slots 4/5 = 0x4C5660/0x4C5670 with
  time info at +0x20 vs CLodTimeModelInfo 0x4C56C0/0x4C56D0 at +0x24, 0x6B8950 scalar deleting dtor
  pattern, __dtor_TheCamera → 0x50A870) all consistent. 16 sa-stream hook names updated to the new
  catalogue names; cargo test sa-hook (incl. matches_catalogue_and_exe) / sa-stream / sa-core pass,
  clippy host + i686 clean.

## 2026-11-07: STREAM-FIX-1 done
- `l3.sh --rust ports` (lane 3) bisected the rand divergence to CLoadingScreen::LoadSplashes alone
  (rest of the set matched all 5,200 frames). Causes: QueryPerformanceCounter called through the IAT
  cell 0x8580C8 instead of the determinism-patched call site 0x5900FF (so srand got real time); then
  srand in LoadSplashes and Game::GameInit not attributed to the original return addresses 0x59010F /
  0x5BF3CE (randcalls hash). Audit: the 9 determinism-patched sites inside catalogue functions were
  mapped to their functions; the ported ones (CdStreamSync by design, GetCurrentTimeInCycles,
  psTimer, LoadSplashes) all use `site` now; every Rust rand/srand call goes through `at_site`.
- Harness: nested `l3.sh --probe` runs re-source lib.sh; it now keeps the inherited lane display
  (E2E_DISPLAY) after clearing the desktop variables (the first bisection attempt failed on this).
- Verification: i686 release build, cargo test sa-core 58 / sa-stream 10 / sa-hook 33, clippy host +
  i686 clean; `port-l2.sh ports --runs "plain rust"` (lane 2): exit 0, plain and rust 5,200 frames,
  rust trace IDENTICAL to the all-original trace over 5,200 frames, 17 subsystems.

## 2026-11-07: CORE/STREAM L1 verifier audit integrated
- Adversarial verifier (worktree verifier, Unicorn only, Wine backend disabled) over the 126 PORT-CORE
  / PORT-STREAM L1 claims; report docs/re/verify-l1-core-stream.md, tests
  crates/sa-oracle/tests/verifier_core_stream.rs (7), scan tools in tools/verify/.
- Findings: V-9 wrong claim (SetBaseModelInfoFlags 0x5B3AD0: caller 0x5B4177 reads DL after the call,
  the original never writes EDX, the Rust shim clobbers it); V-10 44 specs exercise a helper or a
  hand model instead of the hooked entry (no error found; 6 re-run through the hooked entries, 0
  mismatches); V-11 GTATranslateKey generator hits chr == 0xFF ~2 per 10^6; V-12 seven float specs
  differ under CW 0x027F (they assume PC=24); V-13 instrumented audit found no hidden outputs; V-14
  minor. Sample of 17 specs at 10^6: 0 mismatches.
- Main-session verification: caller bytes at 0x5B4177 and callee 0x5B3AD0 disassembled (confirmed);
  7/7 verifier tests pass on master (158 s); CStreamingInfoAddToList at 10^6 PASS; own scan of all 776
  hooks (540 with direct callers): only 0x5B4177 reads ECX/EDX after a hooked call (11 other hits are
  writes, stack-allocation pushes, alignment `lea ecx,[ecx]`, or a misdecoded site).
- 0x5B3AD0 set back to `implemented`; ledger entry (pending); VER-2 queued for the harness slot.

## 2026-11-07: CORE-L2b (part 1) and VER-2 integrated
- CORE-L2b agent (lane 1): no port bug; ~200 evidence-backed L2 regions in tools/e2e/l2-extras.txt;
  pool/heap/CRT/delay-import/RW-driver regions documented in docs/arch/core-port.md.
- Main-session re-run on master, lane 1 (target/corel2b/verify.sh): vB (14 burst functions, 1000/frame)
  exit 0, (b) identical; vC exit 0; vA (138, 4/frame) (b) identical over 5,200 frames, only real
  mismatch Ps::InitialiseLanguage (CRT locale refcounts 0x8E2EC0/0x8E377C, double-run effect), other
  FAIL lines are the 4/frame sampling cap for functions covered by vB/vC; vR all 191 on Rust IDENTICAL
  5,200 frames / 17 subsystems; vPRR pause-restart all 191 on Rust IDENTICAL 3,700 frames; vPRB exit 0.
  81 functions set to L2 (target/corel2b/pass.tsv). F1: 24 → 105 (0.68 %).
- VER-2 agent: preserve-register hooks, caller scan, hosted specs, generator bias, FPU caller audit
  (details in ROADMAP). Main session: confirmed the second case (0x71DC62 → re-push of ECX/EDX at
  0x71DC7C on the je path), cargo test --workspace, sa-oracle lib/l1/verifier_l1 (5 passed, 27 min),
  clippy --workspace --all-targets host + i686, wasm32 builds, 10^6 re-runs of
  SetBaseModelInfoFlags and GTATranslateKey PASS, all-ports grove walk on Rust IDENTICAL (lane 1,
  exit 0). 0x5B3AD0 back to L1; its ledger entry removed (resolved).
- Disk incident: a STREAM-L2 run wrote a 43 GB spool (/home 5.8 GB free); deleted by the main
  session (regenerable), agent given a 10 GB budget; INF-18 (spool quota) queued.
- New ledger entry (original UB): LightsCreate uninitialised alpha/pad words. New tasks INF-17, INF-18.

## 2026-11-07: STREAM-L2 (part 1) integrated
- STREAM-L2 agent (lane 2): no port bug; 164 PASS, 142 NOCALLS, 97 not replayable by double-run L2
  (RW/D3D objects, file/OS handles, loading-screen rendering), 15 mismatches all traced to harness
  limits or original uninitialised data (LoadVehicleObject skimmer wheel sizes), 11 partial, 5
  undeclared-only. Open questions answered (m_pColNodeInfos at 0x8A630C, protection-VM values measured
  identical). Tools: STREAM section in l2-extras.txt (`doc:` lines), port-l2.sh `L2_SPOOL_DISCARD=1`,
  hook-names.py `doc:` selector. Merge conflict in l2-extras.txt resolved keeping both sections.
- Main-session re-run of the 164 PASS rows (lane 2, target/streaml2/verify.sh): sIG grove in-game
  (SA_L2_FROM=205, 4/frame) and sPR pause-restart: l2 traces IDENTICAL (5,200 / 3,700 frames), 0
  mismatches; sAF all-frames (1000/frame) crashed at frame 4,096 after the end of sampling as the agent
  reported, so it gave no per-hook table; its text report showed 60 RemoveWindowAlphaCB mismatches:
  a double-run artifact (material colour and restore list not declared); declared from the bytes
  (0x4C83C5/0x4C83D2/0x4C83D8), targeted run 1000/1000 cases, 0 mismatches. 65 rows confirmed and set to
  L2 (F1 105 → 170); the remaining 99 PASS claims need per-batch re-runs (in progress).

## 2026-11-07 ~22:45: paused (user request)
- State: F1 170 / 15,479 (1.10 %). Session commits 50c928e..this one. Unmerged work in worktrees coll,
  harness2, inf10 (see the ROADMAP PAUSED banner); STREAM-L2 re-verification of 99 claims interrupted.

## 2026-11-07 (night): PORT-COLL integrated
- Agent (worktree coll): crates/sa-world with 121/145 COLL functions (120 hooks), sa-oracle specs
  specs_coll_{prim,data,surface}.rs (80), PORT-COLL section in l2-extras.txt, docs/arch/coll-port.md.
  No harness-core change. Catalogue notes: 0x4161F0 = CLinkList<CCollisionData*>::Insert (registered as
  thunk_FUN_01570010), 0x50D590 = GetNearestDistanceOfPedSphereToCameraNearClip.
- Main-session verification: patch applied (l2-extras conflict resolved keeping all sections); i686 +
  host release builds, cargo test --workspace (sa-oracle excluded; its lib/l1 ran in the agent run and
  after VER-2), clippy --workspace --all-targets host + i686, wasm32 build of sa-world. Byte checks:
  0x417F76/0x417F83 (EDX live across TestSphereSphere), 0x415234 (sphere A radius for both spheres),
  0x416450 sum order. L1 10^6 samples: SurfaceInfosIsAudioSand, IsBeach PASS; GetPrincipleAxis,
  ProcessSphereSphere, ProcessLineSphere 0 mismatches but INCOMPLETE under Unicorn only (SNaN cases);
  ProcessSphereSphere with the native backend: 53,566 CPU-decided, 0 differ, PASS. Lane 1 re-runs
  (target/collv/verify.sh): vc3/vcpr (b)+(c) IDENTICAL (5,200 / 3,700 frames), vcb and vcn exit 0,
  vcports (all 897 ports on Rust) IDENTICAL. 84 → L2 (F1 170 → 254, 1.64 %), 36 → implemented.
- COLL L1: all 80 specs re-run at 10^6 by the main session with the default backends (real CPU decides SNaN cases): 80/80 PASS, 0 mismatches; 25 implemented rows → L1 (COLL now 84 L2, 25 L1, 11 implemented, 24 deferred, 1 unknown).

## 2026-11-08: STREAM-L2 re-verification (parts 2–3)
- Lane 2, all-frames batches (SA_L2_FROM=0, 1000/frame, spool drained): sR0–sR2 73 PASS, 0 mismatches,
  0 undeclared (sR0 l2 trace leaves plain at frame 202: loader sampling shifts heap allocation; cases
  stay original vs Rust from the same restored state); sR3 crashed at frame 4,096 (no table); split
  into three: sR301/sR302 16 PASS, identical l2 traces; sR300 (8 hooks) crashes at 4,096 again → open.
  Total confirmed from the agent's 164: 65 + 73 + 16 = 154 → L2. F1 = 343 / 15,479 (2.22 %).

## 2026-11-08 ~00:50: agents stopped by the user, work merged
- The user ran out of tokens and asked to stop the agents and bring their work in. Lanes 3/4
  stopped (wineserver -k), agent processes in the harness2/inf10 worktrees killed, both sessions
  confirmed "stopped". Their work was saved as branches wip-harness2 / wip-inf10 and merged into
  master; conflicts in tools/e2e/port-l2.sh (spool discard FIFO + disk check + L2_EXTRA + doc: extras)
  and l2-extras.txt (all sections kept) resolved by hand.
- Verified by the main session: i686 + host release builds, cargo test --workspace (sa-oracle lib
  16/16), clippy --workspace --all-targets host + i686 clean, `bash -n port-l2.sh`.
- NOT verified: the agents' in-game results (INF-18 quota test, INF-17 split-run verdicts, INF-10
  coverage). Documented in the ROADMAP rows INF-10/INF-17/INF-18 and the new FPU-GUARD task.

## 2026-11-10: resumed in Claude Code; FPU-GUARD fix
- Resumed (main session). FPU-GUARD: sa-core's FpuGuard now keeps a per-thread "game MXCSR" (table keyed
  by TEB thread id, no_std); every phase-A `orig!` call goes through `abi::call_original`, which runs the
  original callee under the game MXCSR and keeps what the callee leaves (D3D device creation sets 0x9FA0);
  the guard's exit loads that value instead of the stale entry value.
- Verified: i686 + wasm32 builds, sa-core 59 / sa-hook 37 tests, clippy host + i686 clean;
  `port-l2.sh ports --runs "plain rust"`: grove walk (lane 1) and pause-restart (lane 2), all ports on Rust,
  IDENTICAL over 5,200 / 3,700 frames on 17 subsystems, 0 crash lines.
