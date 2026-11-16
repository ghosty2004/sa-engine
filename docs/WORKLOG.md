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
- Initial commit ebae837.
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
- Resumed from c2261fe at the user's request (ROADMAP banner replaced).
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
- State: F1 170 / 15,479 (1.10 %). Session commits 148e1c3..this one. Unmerged work in worktrees coll,
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
- INF-17 re-run (lane 3, main session): split-L2 groups l/m/h/solo on the grove walk after FPU-GUARD:
  26 PASS → L2 (F1 343 → 369), 6 PASS-FP not set, 20 NOCASES, 2 FAIL open (RsEventHandler,
  psCameraShowRaster solo). Every splito/splitr trace IDENTICAL to plain.

## 2026-11-10: INF-19 workflow speed-ups
- DXVK frame limiter/vsync off by default: a 5,200-frame replay takes 34 s instead of 113 s, with an
  IDENTICAL state trace (target/inf19). inf3/inf4 pass with it.
- port-l2.sh caches the all-original trace (target/l2cache): the same pilot batch takes 43 s on a cache
  hit vs 78 s cold, and ~240 s before INF-19.
- lib.sh waits for ≥4 GB available RAM before a game run; PLAN §8 now allows up to 7 lanes.
- sccache installation was blocked by the permission classifier (left for the user).
- sccache (installed by the user) measured: a clean i686 sa-hook build is only ~10 s, and two fresh
  target directories gave 0 cache hits / 28 misses (the cache key includes the output paths, which
  differ between worktrees). No benefit, so it is not enabled in the repo.

## 2026-11-10: PORT-WORLD part 1 integrated
- 248 WORLD functions ported (sa-world/src/world): zones, CWorld leaves, pickups, garages, entry-exits,
  escalators, stunt jumps, game logic, restart, stats, cheats, shopping, static initialisers. New
  recording target/e2e/corpus/world-cheats.sarec (17 typed cheats + suicide + respawn).
- Main-session verification: tests + clippy host/i686 + i686 build; lanes 3/6: world-p1-l2 list on the
  grove walk and world-cheats: 0 mismatches, 0 undeclared, l2 and rust traces IDENTICAL (the FAIL lines
  are only the default 4-per-frame sampling cap; the agent's runs at 1000/frame compared every call);
  all ports on Rust IDENTICAL on pause-restart. States: 83 L2, 21 L1, 144 implemented → F1 = 452
  (2.92 %).
- Found: the protection reads some hooked function entries as data → INF-20.

## 2026-11-10: PORT-PHYS integrated
- 62 PHYS functions ported (sa-world/src/physical), merged with PORT-WORLD (keep-both conflicts in
  hook/spec/module lists and l2-extras). Main-session verification on lanes 3/6/7: tests + clippy host/i686
  + wasm32; phys-sec L2 IDENTICAL; split groups s1–s5 re-run: the 8 claimed PASS confirmed, the 4 collision
  checkers FAIL/DIVERGED as reported; AddToMovingList/RemoveFromMovingList show undeclared writes at
  0xB9ACC8 (agent claimed 0) → held at implemented; PhysSetVelocity L1 1M PASS; all ports
  (CORE+STREAM+COLL+WORLD+PHYS) on Rust IDENTICAL on grove walk, pause-restart and world-cheats.
  States: 29 L2, 6 L1 → F1 = 481 (3.11 %).
- PLAN §8: nested-agent cap (≤2 helpers per sub-agent, one level) after nested forks reached 20 agents.

## 2026-11-10: INF-10 integrated
- Coverage of the original done: 12 validated data-read exclusions; grove walk + pause-restart coverage
  runs IDENTICAL to plain; the corpus executes 48.2 % of the non-excluded functions (7,462/15,479) —
  this is why ~half of the ports end NOCALLS; F2 report with all ports on Rust (no port lets its original
  body run). Main-session re-verification: sa-oracle lib 17/17, pause-restart coverage run (76 s)
  IDENTICAL.

## 2026-11-10: STREAM harness work integrated (keycells, split fixes)
- Agent: the "crash at frame 4,096" was the 0x410860 hook overwriting bytes a protection chunk reads as a
  decode key (fault at 0x401E4C); new sa-hook keycells.rs redirects every protection reader of hooked code
  bytes to a copy of the original dword, with a test that fails on any unlisted reader. The test caught a
  new reader at the merge (0x40A4C3 reads PHYS hook 0x54D690), added by the main session (6 readers).
  Other fixes: Foreign rule for per-process region addresses, split-report --fn streaming, port-l2.sh stops
  on unhandled exceptions, colstore LoadAllBoundingBoxes call path.
- Main-session verification: tests (sa-hook 39) + clippy host/i686; all ports on Rust IDENTICAL on grove
  walk, pause-restart, world-cheats with `keycells: 6/6`. One plain run hit the known post-exit fault at
  7BCF0729 (after the trace was complete).
- States: STREAM verdicts with 0 mismatches AND 0 undeclared → 29 L2. Split PASS rows that list
  undeclared writes are not L2 (PLAN §5); for consistency 11 CORE functions set to L2 earlier today from
  split runs with undeclared writes were demoted to implemented. F1 = 499.

## 2026-11-11 harness: INF-20 done, callers through calls, ColModels pads (main session)
- Agent abc057e8 (lane 1). INF-20: `catalog/code-data-reads.csv` + `code_data_reads.py` in run_all.sh; sa-hook install
  test now rejects hooks whose bytes the protection reads unless keycells redirects them. Callers test follows
  ECX/EDX into callees (depth 3); only the 4 declared ApplySpeed sites found across 8,765 call sites.
- Normaliser: ProcessColModels static matrix pads, TheCamera inverse-matrix pads, CRT NLG frame. Split PASS for
  CheckCollision, CheckCollision_SimpleCar, ProcessShift, ProcessEntityCollision (states not changed until
  undeclared counts are checked). RsEventHandler still FAIL (OS handle, 0xC96438).
- Verified by main session: sa-hook 44 + sa-oracle-format 22 tests pass. Open: L1 `CPhysical::ApplyMoveForce-vec`
  fails on 82 signalling-NaN cases (payload 0x7FC00001 vs 0x7FC00000) — PHYS port, ledger.

## 2026-11-11 SAMP-1D dynamic SA:MP coverage (main session)
- Agent ab5d93ec (lane 4): original gta_sa.exe + samp.dll 0.3.7-R4 joined a private open.mp server and played all
  docs/samp/README §6 scenarios; 7,003 functions executed. Results: catalog/samp-priority.csv, docs/samp/dynamic.md,
  scm-opcodes-dynamic.csv; tooling tools/e2e/samp-cov.sh + tools/e2e/samp/; sa-hook coverage gains SA_COV_SCM
  (opcode counter detour, coverage-only) and a flush at DLL_PROCESS_DETACH.
- Main session: re-applied only the agent's win.rs hunk onto master (a whole-file checkout had dropped
  `keycells::install`, caught by an unused-code warning); i686 build has 0 warnings, sa-hook 45 tests pass.
- Open: no frequency counts (SA_COV_CAP>1 hangs under samp); trailers/mod shop/jetpack/second client not covered.

## 2026-11-11 NATIVE-DEMO: native wgpu world viewer (main session)
- Agent a280cefa: new crates sa-platform, sa-rw, sa-rw-wgpu, sa-viewer (+ docs/arch/native-demo.md,
  tools/native-demo/screenshots.sh). Native x86_64 binary loads IMG/IDE/IPL/timecyc/water from the install and
  renders area 0 with LODs, day/night prelight, fog, colour filter. Reuses sa-formats and four L1 helpers.
- Verified by main session: release build, 28 unit tests pass; offscreen screenshot of Grove Street at 19:30
  streamed in 0.15 s and looks like the game. Screenshots stay in target/ (rendered Rockstar assets, never committed).

## 2026-11-11 RE-SPEC-3 partial (main session)
- Agent add9c262 with 5 writers (started before the nested-agent ban; stopped early on it). 683 new spec rows for
  POP/RENDER/SCRIPT/AUDIO/HUD/SAVE; AI not started. Main session re-ran check_consts on every changed spec file:
  817 claims, 0 mismatches (agent: 1,258 over all new files).
- Notable gta-reversed errors (ports follow the exe): SCRIPT 0384 is real; many gr handlers return -1 where the exe
  returns 0; CPopCycle renormalises rows to 100; HUD InsertStringInString uses "~a~"; FxEmitter rate ×0.001.
- CAT-17 proposals: 0x6E7680 not RenderIfDoesntFit; vt_866db0 is a WinSock TCP helper; 0x4F1020 CAEStaticChannel
  deleting dtor; 0x44DA30 TakeWidthIntoAccountForCoors; 0x44E790 GeneratePedCreationCoors; 0x450FB0
  AddInteriorNodeLink; 0x421A50 WouldCollideAtThisAngle; 0x4410E0 IsDPadRightPressed; 0x41BDD0 SetEngineOn;
  0x45AFB0 SetTurnSpeed; 0x821B40 _ftol2; missing from catalogue: 0x477DA0, 0x43EF90, 0x6E76E0.

## 2026-11-11 PHYS fix: ApplyMoveForce NaN order (main session)
- L1 failure reported by the harness agent: CPhysical::ApplyMoveForce-vec, 82 signalling-NaN cases (payload
  0x7FC00001 vs 0x7FC00000). Cause: 0x5429F0 and 0x5430A0 load z onto the x87 stack before 1/mass and multiply it
  as a register (`fmul st,st(2)`); the port used the memory form that ApplyForce (0x542B50) uses. New
  `move_part_zreg` for the two functions; ApplyForce/ApplyFrictionForce keep `move_part`.
- Verified: `sa-oracle l1` 1,000,000 cases each, unicorn, 131,411 sNaN inputs decided by the CPU: PASS, 0
  mismatches for ApplyMoveForce-vec and ApplyFrictionMoveForce. clippy -D warnings, i686 build, sa-world tests OK.

## 2026-11-11 SAMP-API design (agent; merged by main session, clippy + test re-run)
- Read samp-rs (read only) and the SAMP-1/1D outputs; grouped every samp-rs access by feature.
- `docs/samp/game-adapter.md`: GameAdapter (15 feature traits) / GameHooks (callbacks with typed call sites) /
  SampHost / FileProvider; three backends (original exe default, phase A in-process, phase B native);
  frame and threading contract (`&mut dyn` in frame callbacks, `&dyn` in mid-frame hooks); samp.dll patches as
  `Limits`/`FeatureSet`/`Behaviour`/`StartupOptions`/`RuntimeToggle`; 12-step samp-rs migration that keeps the
  original path default; engine port order cross-checked with samp-priority.csv (185 directly named functions,
  132 in the priority list; ProcessOneCommand missing from it because the SCM counter detoured its entry).
- `crates/sa-samp-api`: traits and types only, no dependencies. Verified: `cargo clippy -p sa-samp-api
  --all-targets -- -D warnings` (host), same for `--target wasm32-unknown-unknown` and `--target
  i686-pc-windows-gnu`, `cargo test -p sa-samp-api` (1 test), `RUSTDOCFLAGS=-D warnings cargo doc`.

## 2026-11-11 SAMP-2 steps 1–3 + API revision (main session)
- Agent a08af60d: samp-rs worktree ~/samp-rs-adapter, branch sa-engine/adapter (4 commits, samp-rs main and its
  working tree untouched). Steps 1–3 of the migration; verification in ROADMAP SAMP-2.
- Main session: freed 17 GB (target/ of four integrated worktrees; /home was at 23 GB free). Applied the agent's
  API proposals to sa-samp-api (menu_active byte/dword, area_raw, time_step_clipped, script threads,
  GameError::Unreadable, infallible-read contract); clippy host+wasm32, test, rustdoc -D warnings clean.

## 2026-11-11 c-bug corpus recording (main session, user request)
- tools/e2e/programs/c-bug.txt: SA:MP "c-bug" (fire, crouch a few frames later, fire from the crouch, stand up) with
  the desert eagle and the sawn-off, fire->crouch delays 1/2/3/5/8 frames, plus a no-crouch control block. Pad
  controls (r1 aim, circle fire, l3 crouch): the e2e setup runs the game in pad mode, where mouse buttons and the
  C key do not reach the controls (second attempt showed no aim/fire). HESOYAM/ASNAEB keep CJ alive (the first
  attempt died to the police and played the sawn-off block after respawn).
- Recorded on lane 7 with all hooks original: 3220 frames, replay IDENTICAL; screenshots show crouched fire
  and a used clip. Recording at target/e2e/corpus/c-bug.sarec (local, gitignored); use it in all-ports L3 runs
  once WEAPON/PED/task ports land. Whether the crouch shortens the fire cycle is not measured here; the user
  will check the feel manually on the Rust build.

## 2026-11-11 PORT-POP first pass integrated (main session)
- Agent ad992e74 (lane 1): new crate sa-pop, 74 POP functions from the exe bytes; docs/arch/pop-port.md,
  pop-port-verdicts.tsv, lists tools/e2e/lists/pop-*.txt, L2 extras section `# PORT-POP`.
- Main session: squash merge clean; sa-hook lib 45 tests (keycells, INF-20, callers), sa-pop 3 tests, i686 build 0
  warnings; all-ports on Rust IDENTICAL on grove (5200), c-bug (3220, new) and world-cheats (3276).
- States: 61 → L2 (L2 runs 0 mismatches and 0 undeclared), 13 → implemented. F1 = 560 / 15,479 (3.62 %).

## 2026-11-11 PORT-SCRIPT pass 1 integrated (main session)
- Agent a740d873 (lane 4): new crate sa-script (VM core, ProcessCommands0To99); docs/arch/script-port.md + verdicts.
- Main session: squash merge with conflicts against PORT-POP in sa-hook table.rs/callers.rs/Cargo.toml and sa-oracle
  (combined: pop then script in the table), l2-extras (both sections kept). sa-hook lib 45, sa-pop 3, sa-script 3 tests;
  i686 build and clippy clean; all-ports IDENTICAL on grove, c-bug, pause-restart. States: 9 → L2, 6 → implemented.
- Decision recorded in ROADMAP PORT-SCRIPT (partial switch ports with original fallback).

## 2026-11-11 L2-EXTRAS integrated (main session)
- Agent a9db9276 (lane 6): extras for 190 rows, all final runs 0 undeclared / 0 mismatches; tool changes listed in
  ROADMAP L2-EXTRAS. Merge conflict in l2-extras.txt resolved keeping all sections.
- States: 109 → L2 (at least one plain PASS, no FAIL, orchestrators excluded). F1 = 678 / 15,479 (4.38 %).
- Policy note: an extras line covering the whole static data with access D does not count toward L2 (PLAN §5 asks for
  0 undeclared writes against an evidence-backed set; a blanket region proves nothing).

## 2026-11-11 RE-SPEC-AI pass 1 (main session)
- Agent ad07387a + 2 helpers: docs/re/specs/AI/ (159 files), 564 index rows. Main session re-ran check_consts over all
  AI files: 228 claims, 0 mismatches (claim density is low; ports must re-read the bytes for rows marked
  medium/low: 0x4BDB80, 0x4BE7D0, 0x662A10, 0x683AD0, 0x644470, 0x642AE0, 0x681FE0, 0x674140, 0x6479B0, 0x64FC10 low,
  and the CPedGeometryAnalyser notes, which nobody checked against the bytes).
- gr errors the ports must not copy: EventHandler response functions keep m_EventResponseTask on null source;
  0x4C2610 2D squared radius; Sequence repeat count; CTaskSimpleCarDrive::TriggerIK rand only for some missions
  (RNG stream!); GangLeader 1% roll; KillCriminal ped types ≥ 24; several structure sizes/offsets
  (WalkRoundObject up to +0x53, GoToVehicleAndLean 0x24, CTaskComplexPartner +8).
- CAT-17 proposals: scalar deleting destructors catalogued as ~X (vtable slot 0) in CTaskComplex.md/CTaskSimple.md
  tables; names 0x4ABCA0 CEventDamage::HasKilledPed, 0x5FFEE0 CEntityScanner::GetClosestEntityInRange, 0x606060
  CDecisionMaker::MakeDecision, 0x646CF0 CCarEnterExit::IsCarDoorInUse, 0x667310 CPathNode::GetNodeCoors, others in files.

## 2026-11-11 PORT-WEAPON-PED integrated (main session)
- Agent ac3e5c3c (lane 5) + helpers: crates sa-weapon (121 hooks) and sa-ped (217), docs/arch/weapon-ped-port.md,
  tools/verify/hookscan.py, weapon-combat program/recording, l2-extras section.
- Merge: conflicts with POP/SCRIPT registrations resolved (order core, stream, world, pop, script, weapon, ped). The
  branch (based on 861e7ea, before keycells) had unregistered four protection-read hooks to avoid the 0x401E4C crash;
  master's keycells redirect already fixes that crash, so the four stay registered (files restored to master).
- Verified: sa-hook lib 45 (keycells, INF-20, callers), sa-weapon/sa-ped tests, clippy, i686 build; all-ports IDENTICAL
  on grove 5200, weapon-combat 3500, pause-restart 3700, c-bug 3220. A first c-bug comparison "DIVERGED at 1421: pad"
  was a harness artefact: the loop reused one --tag for several recordings, so a stale weapon-combat Rust trace was
  compared; rerun alone and l3.sh both IDENTICAL. Rule: one --tag per recording.
- States: 150 → L2, 31 → L1, 161 → implemented. F1 = 828 / 15,479 (5.35 %).

## 2026-11-11 PORT-ANIM-INPUT integrated (main session)
- Agent acb5cdca (lane 2): crates sa-anim, sa-input; sa-macros/sa-core `entry_eax`; docs/arch/anim-input-port.md;
  anim-input program/recording; lists and l2-extras section.
- Merge: registration conflicts resolved (order core, stream, world, pop, script, weapon, ped, input, anim); sa-stream
  unregistrations dropped (keycells covers them). Master's INF-20 and callers tests (newer than the branch base)
  failed on 2 hooks; fixed as in ROADMAP. sa-hook lib 45, sa-anim/sa-input/sa-core/sa-macros tests, workspace clippy
  -D warnings, i686 build OK; all-ports IDENTICAL on six recordings (grove run: known post-exit fault 7BCF0729).
- States: 132 → L2, 115 → L1, 155 → implemented. F1 = 960 / 15,479 (6.20 %).

## 2026-11-11 PORT-VEH integrated (main session)
- Agent a65cade7 (lane 7) + helpers: crate sa-veh, docs/arch/veh-port.md + verdicts, driving program/recording,
  keycells reader 0x401635, lists, l2-extras section.
- Merge: registration conflicts resolved (veh appended after anim). sa-hook lib 45, sa-veh 3 tests, workspace clippy,
  i686 build OK; all-ports IDENTICAL on six recordings incl. driving (4338).
- States: 90 → L2, 124 → implemented. Decisions in ROADMAP PORT-VEH.

## 2026-11-11 RE-SPEC-AI pass 2 (main session)
- Agent a1d60d08 + 2 helpers: 920 AI rows. Main session re-ran check_consts over docs/re/specs/AI (all files): 0
  mismatches. Caveats: helper B re-sorted sections of 13 pass-1 files (originals kept in the scratchpad ai2/hB/orig);
  ~715 generated trivial sections are barely sampled. gr errors worth knowing for PORT-AI: CMeleeInfo layout
  (+0x50 hitLevel[5], +0x55 damage[5], +0x48/+0x4C f32), CEventDeadPed::AffectsPed 400 > dist², CTaskComplexDie
  drowning/DieInCar weapon, KnockOffBike ×1/70 and anim 0xF, CTaskSimpleUseGun::ProcessPed 3 inverted conditions,
  Swim ProcessEffects matrix 3 and ×0.33333334, inlined rand()*3.0518509e-05f in 0x5E9600 family.

## 2026-11-11 PORT-SCRIPT-2 integrated (main session)
- Agent a7b97b06 + 2 helpers: ADR-002 partial ports (docs/arch/ADR-002-partial-ports.md), 343 opcodes, split tooling.
- Merge: sa-macros conflict (entry_eax from ANIM-INPUT vs fallback) combined; sa-hook lib 45, script/macros/core
  tests, workspace clippy, i686 build OK.
- All-ports L3: first batch left 3 recordings without a Rust trace — the stale wine.log bug PORT-HUD found
  (port-l2.sh read the previous run's exit fault as a crash and killed the next run). Took that fix from the HUD
  branch; rerun IDENTICAL on weapon-combat 3500, driving 4338, cheats 3276; grove, c-bug, pause-restart, anim-input
  IDENTICAL in the first batch. No state changes (partial switches stay implemented). Decisions in ROADMAP.
- Known red: sa-oracle `--test l1` preserved_registers_match_the_hook_registrations (sa-script ECX preserves without
  specs) — next small task.

## 2026-11-11 PORT-HUD integrated (main session)
- Agent a6f6e6f4 + 2 helpers: crate sa-hud, 3 spec files, docs/arch/hud-port.md + verdicts, lists, extras section;
  port-l2.sh stale wine.log fix (taken earlier for the SCRIPT-2 reruns).
- Merge: registration conflicts resolved (hud after veh). sa-hook lib 45, sa-hud 3, workspace clippy, i686 OK;
  all-ports IDENTICAL on all 7 recordings. States: 78 → L2, 14 → L1, 56 → implemented. F1 = 1128 (7.29 %).

## 2026-11-11 HARNESS-2 integrated (main session)
- Agent abbb4a8e (lane 5): see ROADMAP HARNESS-2. Merge conflicts: HookReg fields (`set_original` from ADR-002 and
  `arg8`/`arg16`) and port-l2.sh (wine.log fix + stale-output cleanup) combined. sa-hook lib 48, sa-oracle lib 19,
  sa-oracle-format 26, workspace clippy, i686 build OK; all-ports IDENTICAL on grove, weapon-combat, driving.
- States: 6 → L2 from the agent's split reruns (PASS, compared > 0, 0 mismatches, 0 undeclared in every run).
  F1 = 1134 / 15,479 (7.33 %). Note: the core split-report needs ~9 GB RAM (use MemoryMax ≥ 12G).

## 2026-11-11 PORT-WEAPON-2 integrated (main session)
- Agent a186f35f + 2 helpers (lane 2). Merge: port-l2.sh wine.log fix duplicated master's (kept master's), l2-extras
  sections combined. sa-hook lib 48, sa-weapon/sa-core tests, workspace clippy, i686 OK; all-ports IDENTICAL on
  weapon-combat, c-bug, grove, driving, cheats, pause-restart, anim-input. 6 → L2, 8 → implemented. F1 = 1140 (7.36 %).

## 2026-11-11 RE-SPEC-RW pass 1 (main session)
- Agent a9f1509f + 2 helpers: docs/re/specs/RW/ (21 files, 370 rows; specs-index 3,659 lines). check_consts re-run by
  the main session. Notable exe facts: sqrt/inv-sqrt are 12-bit table lookups; stream readers check version window
  [0x34000, 0x36003] and read chunk len into fixed stack structs unclamped; RwD3D9SetTexture always sets
  MAXANISOTROPY on stage 0; camera offset signs asymmetric. CAT-17 proposals: 0x7eede0 parallel frustum builder,
  0x7ef1f0 _rwCameraBuildParallelViewMatrix, 0x7fdcd0 _rwD3D9RWSetRasterStage, 0x7f2160 _rwMatrixInvertGeneral;
  RwStream rwOwned at +0x20 (fix docs/re/specs/STREAM/RwGtaStream.md).

## 2026-11-11 PORT-CAM integrated (main session)
- Agent abe46fff + 2 helpers (lane 1): crate sa-cam, specs_cam.rs, docs/arch/cam-port.md + verdicts, lists, extras.
- Merge: registration conflicts resolved with a scratch script (cam after hud); a duplicated `pub mod specs_core_util`
  from the merge removed. sa-hook lib 48, sa-cam tests, workspace clippy, i686 OK; all-ports IDENTICAL on 7 recordings.

## 2026-11-11 PORT-AI-1 integrated (main session)
- Agent a4ff952d + 2 helpers (lane 7): crate sa-ai, docs/arch/ai-port.md + verdicts, lists, extras section.
- Merge (ai after cam); sa-hook lib 48, sa-ai 3, workspace clippy, i686 OK; all-ports IDENTICAL on all 7 recordings
  (c-bug first). States: 199 → L2, 267 → implemented. F1 = 1420 / 15,479 (9.17 %).

## 2026-11-11 PORT-AUDIO-1 integrated (main session)
- Agent a3243295 + 2 helpers (lane 6): crate sa-audio, docs/arch/audio-port.md + verdicts, lists, extras. Merge (audio
  after ai); sa-hook lib 48, sa-audio 3, workspace clippy, i686 OK; all-ports IDENTICAL on 7 recordings.

## 2026-11-11 FIXUP integrated (main session)
- Agent a34e4dbd (lane 4): specs_script.rs (10 ECX-preserving VM hooks, 1,002,000 Unicorn + 202,000 native cases each,
  0 mismatches; RetKind::Ax); `sa-oracle --test l1` green again (16/16, re-run by the main session after the merge).
- CText::Get 0x6A0050 hooked: the two "protection readers" from PORT-HUD were mis-decodes inside `call 0x1243940`
  (not added to keycells); code_data_reads.py now lists dwords starting up to 3 bytes before an entry; keycells test
  skips rows that are neither chunk nor sweep. SetJustify/SetOrientation hooked after callers.rs follows x87
  branches for slot-reserving pushes. 3 → L2.
- Workspace tests: only verifier_l1 `verifier_unicorn_agrees_with_cpu_outside_snan` fails on
  CMessages::InsertStringInString (Unicorn loses EFLAGS across rep movsb, known L1 harness bug).
- Main session: sa-hook lib 48, clippy, i686 OK; all-ports IDENTICAL on grove, c-bug, pause-restart.

## 2026-11-11 RE-SPEC-RW pass 2 (main session)
- Agent aecf73c2 + 2 helpers: 17 files under docs/re/specs/RW/, +227 index rows; 21 helper claims re-read against the
  bytes, all correct. Details in ROADMAP RE-SPEC-RW. REA not used.

## 2026-11-11 RE-SPEC-UNK (main session)
- Agent a6e5e98f + 2 helpers: classification proposal for all 2,883 UNK-game functions and 2,141 specs in
  docs/re/specs/<SUB>/_unk-*.md and _static-init-unk.md. Main session re-ran check_consts on those 58 files: 2,891
  claims, 0 mismatches. Spot checks by the agent: 12 helper claims (1 in 50), all correct; ~180 pattern-generated
  never-executed specs need byte checks when ported.
- Findings: initialisers 0x84CED0/0x84CF80/0x84D0F0 read .bss nobody wrote yet (order bug or misattribution);
  original bugs (0x5D02D0 adds 0x32000 to TotalProcessed, Resize 0x730020/0x730330 count > capacity, 0x5335E0 writes
  to address 4 on pool null, 0x4A85C0 keys[-1]); catalogue extent errors: 0x72E860 folded into 0x855670, relocated
  pieces 0x40210A→0x4048E0, 0x401C62→0x4A8DB0, 0x4096E1→0x4A9F70, 0x40A18C→0x4AB270, 0x406A96→0x4ACDD0,
  0x401EE5→0x4CAD90, 0x401492→0x420A60, 0x71D780 tail at 0x71D78A missing; CAT-17: 0x40F700 CColModel::~CColModel,
  0x4072E0 CWorld::IncrementCurrentScanCode, 0x46A5F0 not a destructor; 0x508930–0x509230 looks like a remote debug tool.

## 2026-11-11 PORT-RW-1 integrated (main session)
- Agent a385d5f8 + 2 helpers (lane 2): crate sa-rwcore, specs_rw.rs, docs/arch/rw-port.md + verdicts, lists, extras.
  Merge (rwcore after audio); sa-hook lib 48, sa-rwcore tests, workspace clippy, i686 OK; all-ports IDENTICAL on 7.

## 2026-11-11 RE-SPEC-UNK + RE-SPEC-REST specs committed (main session)
- 146 changed/new spec files under docs/re/specs (UNK classification specs, REST specs, AI appendices); check_consts over
  all of them: 2,949 claims, 0 mismatches. RE-SPEC-FINAL / RW-3 / SPEC-QA run concurrently and may append later.

## 2026-11-11 PORT-RENDER-1 integrated (main session)
- Agent ac7bd72f + 2 helpers (lane 1): crate sa-render, docs/arch/render-port.md + verdicts, lists, extras. Merge
  (render after rwcore); sa-hook lib 48, sa-render tests, workspace clippy, i686 OK; all-ports IDENTICAL on 7.
- Also: CWeapon::Fire 0x742300 demoted L2 → implemented (HARNESS-3 found a c-bug split mismatch; WEAPON-FIX on it).

## 2026-11-11 SPEC-QA (main session)
- Agent a1c883e1 + helpers re-derived ~4,000 generated spec sections and 2,880 initialiser rows from the bytes; full
  report docs/re/specs/QA-2026-11-11.md. No error that would change a port's behaviour. Corrections appended as
  `> QA (2026-11-11)` notes: 10 attractor scalar-deleting dtors name the wrong pool (0xB744C4 is ms_pPedAttractorPool),
  RwD3D9RasterLock level is u8 not s8, 4 wrong "readers" cells, gr's 0x8D2ED4 "30.f" is 10.0f in the image. The
  suspected init-order bug at 0x84CED0/0x84CF80/0x84D0F0 is intended (zero-initialised VolMax/VolMin).
  Hand-written sample: 2 MINOR in 58 (~3 %), 0 WRONG → ports must still re-read conditions and signedness.

## 2026-11-11 HARNESS-3 integrated (main session)
- Agent a456341d (lane 7): see ROADMAP HARNESS-3. Clean squash merge; sa-hook lib 50, sa-oracle lib 21,
  sa-oracle-format 27, workspace clippy, i686 OK; all-ports IDENTICAL on grove, c-bug, weapon-combat.
- Promoted to L2 the rerun rows with verdict PASS, compared > 0, 0 mismatches, 0 undeclared (CWeapon::Fire held back).

## 2026-11-11 PORT-AI-3 integrated (main session)
- Agent a8de4982 + 2 helpers (lane 4). Clean merge; sa-hook lib 50, sa-ai tests, clippy, i686 OK; all-ports
  IDENTICAL on the 7 older recordings (car-ai divergence predates the pass; bisect running).

## 2026-11-11 PORT-AI-2 integrated; parallel verification (main session)
- Agent aff5c540 + 2 helpers (lane 5). Merge conflicts in sa-ai hooks.rs/lib.rs (AI-3 modules vs AI-2 modules), ai-port.md,
  l2-extras combined. sa-hook lib 50, sa-ai tests, clippy, i686 OK.
- New tools/e2e/verify-all.sh: all-ports L3 of every corpus recording spread over several lanes; 11 recordings on 3
  lanes in 10 min (was ~50 min serial). Result: 10 IDENTICAL; air-water DIVERGED at 2742 (pre-existing).

## 2026-11-11 RE-SPEC-RW-3, RE-SPEC-FINAL, CORPUS-1 (main session)
- Spec files from RW-3 (811 rows) and FINAL (565 rows) committed after check_consts on all changed files. FINAL's
  RENDER/CCustomCarEnvMapPipeline.md duplicates RW-3's 0x5D8590/0x5DA790 (unindexed cross-reference, kept).
- CORPUS-1 programs and docs/e2e/corpus.md taken from its branch; recordings live in target/e2e/corpus (local).

## 2026-11-11 X87-MODELS merge backed out (main session)
- Squash-merged X87-MODELS (wide X87 values, fyl2x/fptan models, 5 AUDIO hooks re-registered); tests/clippy/i686 green,
  but verify-all (main-x87) DIVERGED on 6 of 11 recordings: anim-input 2409, car-ai 2431, weapon-combat 2373,
  world-cheats 2112 (objects, pools, world), c-bug 2494 (buildings), air-water 2742 (known). A baseline build of master
  in a separate worktree (base-x87chk) was IDENTICAL on world-cheats, weapon-combat and anim-input → the merge causes
  it. Staged files restored to master (only the merge's own files); not committed. Sent back to the agent.

## 2026-11-11 WEAPON-FIX integrated (main session)
- Agent aaebd0b6 (lane 1): CWeapon::Fire's c-bug split FAIL is caused by the original FireInstantHit 0x73FB10 reading an
  uninitialised stack local (gunshell direction, read at 0x740DAC) whose residue depends on the caller frames; Fire port
  faithful. New opt-in SA_L2_STACKFILL (SPLIT_STACKFILL in port-l2.sh) zeroes stack below each sampled split call;
  SA_COV_PEEK for register/stack peeks. Fire back to L2. Ledger: FireInstantHit's uninitialised gunshell direction
  (original bug; a future port must document its value). AI page 0x967000 PASS-FP: ProcessColModels static sphere
  array 0x967F40 element 0 surface bytes from residue; writer of the source sphere unknown; no rule yet.
- Merge conflicts (port-l2.sh, l2-oracle.md) combined; sa-hook lib 50, clippy, i686 OK; all-ports IDENTICAL on grove,
  c-bug.

## 2026-11-11 Single port registry sa-ports (main session)
- New crate sa-ports: CRATES (16 port crates in table order), COUNT, ALL (const concat). sa-hook table.rs (N and
  build), callers.rs, sa-oracle hook_preserve and tests/l1.rs now read sa_ports::ALL — adding a crate is one
  Cargo line + one CRATES line. PLAN rule added.
- The L1 preserve test used to chain only 9 crates; over all 16 it found 15 older hooks with `preserve` but no
  register-comparing spec. They are listed in PRESERVE_SPECS_PENDING (may only shrink; new offenders fail) and queued.
- Verified: sa-hook lib 50, l1 preserve/routes tests, workspace clippy, i686 OK; verify-all main-reg IDENTICAL on grove
  and driving (c-bug pending).

## 2026-11-11 AI-GEN integrated (main session)
- Agent ae0179cf (lane 6): generator tools/gen/ai_patterns.py + crates/sa-ai/src/gen_*.rs, docs/arch/ai-gen.md + verdicts.
- Merge: sa-ai hooks.rs/lib.rs conflicts combined; generator re-run on master (its output differed because AI-2/AI-3
  were merged meanwhile; --check clean afterwards). verify-all main-aigen: IDENTICAL on all 10 recordings (air-water
  excluded until its port fixes land).

## 2026-11-11 Bisection fixes integrated (main session)
- Harness (BISECT-CARAI 375b838): SA_HOOK_MODE / SA_L2 values over ~30,000 chars go through a file (@PATH; lib.sh
  e2e_list_env; sa-hook list_value). Before this, l3.sh probes over the full port universe exceeded the 32,767-char
  Windows env limit, sa-hook ignored the list and every probe ran all-original ("MATCH" without testing). port-l2.sh
  `ports` mode (SA_HOOK_PORTS) was never affected, so the all-ports verifications stand; l3 bisections before this are void.
- Ports (BISECT-AIRWATER): CTaskSimpleCarJumpOut::ProcessPed 0x64DD60 ran its factor update only with an anim (the
  original also runs it without); CStreaming::StreamOneNewCar rand at_site moved to the return address 0x1563BF7;
  l2-extras for JumpOut's callees. (Their duplicate l3 fix 6e58402 not taken.)
- car-ai frame 2363 root cause (BISECT-CARAI) = WEAPON-FIX's: original FireInstantHit reads an uninitialised stack
  local; visible when Rust frames change residue. Ledger (original bug). X87 agent is adding a stack-footprint helper
  so ports whose residue is read later leave the original's bytes.
- verify-all main-bisfix: IDENTICAL on all 11 recordings, air-water included. 2 → L2.

## 2026-11-11 PRESERVE-SPECS integrated (main session)
- Agent a94dc0e7: register-comparing L1 specs for the 15 pending preserve hooks (new specs_veh.rs, specs_ai.rs, cam,
  input); hosted.rs gains Widened images and fixed_map. 10^6 Unicorn + native cases each, 0 mismatches.
- The test then caught one new offender from AI-GEN (0x4B1F80 dtor, preserve ecx) → pending list (spec queued).
- Ledger (FSIN 53-bit model): RwMatrixRotate 0x7F1FD0 mismatches the CPU in 2,476 of 65,000 native cases (stays
  implemented); CCamera ease weight sin(x)+1 near -1 in 0x50D350/0x50D430 (~2.7% of cases) — both need the 64-bit
  fsin/fcos model (X87 work).

## 2026-11-11 VEH-2 integrated (main session)
- Agent ac9fab53: 299 remaining VEH functions ported (CVehicle, CAutomobile/CBike/CBmx/CBoat/CPlane/CHeli/CTrain,
  CVehicleRecording, transmission, handling conversion, statics). Verdicts in docs/arch/veh-port-2-verdicts.tsv.
- verify-all main-veh2: IDENTICAL on all 11 recordings. 172 → L2; 21 L2 verdicts whose extras declare the whole
  .data/.bss stay implemented (policy), as do L2-split rows with undeclared writes or quota-cut case counts.
- CVehicle::Save stays unregistered (F5 work). 4 protection-read hooks queued for keycells. 4 preserve hooks added to
  PRESERVE_SPECS_PENDING (0x6D1080, 0x6D2600, 0x6D4900, 0x6D3F30).

## 2026-11-11 X87 integrated (main session)
- Agent a746e68d: sa_math::x87 models FYL2X/FPTAN with 64-bit results (AudioLog10 users hooked); orig! FFI keeps
  X87 out of returns (compile-time width guard); #[hook(footprint = W)] + sa_core::abi::stack_footprint for ports
  whose stack residue original code reads (first: RwV3dNormalize); determinism mode zeroes FireInstantHit's frame
  [S-0x1B4, S-0xC) at entry (original reads an uninitialised shot direction at 0x740DAC; ledger: original bug).
- l1.rs: KNOWN_CPU_MISMATCHES (may only shrink) holds 0x7F1FD0 RwMatrixRotate, which already failed on master.
- verify-all main-x87: IDENTICAL on all 11 recordings.

## 2026-11-11 PED-2 and WORLD-2 integrated (main session)
- PED-2 (a14c82d2): 56 ped functions (47 hooks). WORLD-2 (acc0cbbc): 495 WORLD functions (481 hooks: statics,
  entity, object, CWorld, misc). WORLD-2's c-bug 2494 divergence is gone on top of X87's FireInstantHit scrub.
- verify-all main-pw (both merged): IDENTICAL on all 11 recordings. States promoted from the verdict TSVs (only
  exact L2 rows, minus whole-data-D extras); 3 WORLD-2 preserve hooks queued in PRESERVE_SPECS_PENDING.
- Open (WORLD-2): ports calling CEntity::Constructor directly break split pairing (route through orig! per
  ADR-001); GetHeading/GetRoll/FindPlayerHeading withdrawn until a 64-bit FPATAN model exists.

## 2026-11-11 RENDER-2 and POP-2 integrated (main session)
- RENDER-2 (a792c7c8): 250 RENDER functions (244 hooks: CWaterLevel, CTimeCycle, 3D markers, car plates, CGlass,
  stencil shadows, occlusion, regular Fx infos). POP-2 (af6f833d): 335 POP functions (321 hooks: 106 statics,
  CPathFind, CPopulation, CPopCycle, CCarCtrl, gangs, darkel, car generators).
- verify-all main-rp (both merged): IDENTICAL on all 11 recordings. States from the verdict TSVs (exact L2 only).
- 3 POP-2 preserve hooks queued in PRESERVE_SPECS_PENDING. sa-hook's matches_catalogue_and_exe now runs on a
  256 MiB thread (the debug HOOKS table overflowed the default test stack).
- Open: CPopCycle::FindNewPedType 0x60FBD0 one split mismatch per recording (stays implemented; POP-3 owns it);
  car-ai splito Wine fault at frame 2304 (harness).

## 2026-11-11 Verification gap found and closed; AI-4, AI-5, CAM-2, HARNESS-4 integrated (main session)
- verify-all never built the DLL: the X87, PED-2+WORLD-2 and RENDER-2+POP-2 runs (main-x87/main-pw/main-rp) tested
  the 10:56 VEH-2 DLL. verify-all now builds sa-launcher+sa-hook first, aborts on a build failure, and records the
  HEAD and the DLL's sha256 next to the summary.
- The X87 widening broke the i686 build for 13 orig! ST0 returns declared X87 (VEH-2, POP-2, AI-4/5, RENDER-2):
  declared f64 with `_st0` + X87 adapter functions (as X87 decided). Duplicate hook CPedIntelligence::operator new
  (PED-2 and AI-5): sa-ped keeps it.
- Fresh-DLL full run: air-water DIVERGED at 4585 (randcalls). Commit bisection (separate worktree): X87 IDENTICAL,
  X87+PED-2 IDENTICAL, +WORLD-2 DIVERGED; hook bisection over WORLD-2's 481 hooks →
  WaterCreatureManager_c::GetRandomWaterCreatureId 0x6E4040 (rand without at_site). Fixed there and in
  InteriorGroup_c::GetRandomInterior 0x5948C0 and Interior_c::GetRandomTile 0x591B20 (no other unsited rand left).
- verify-all main-full3 (VEH-2..POP-2 + AI-4 + AI-5 + CAM-2 + HARNESS-4, fresh DLL 3e8a73b): IDENTICAL on all 11.
  States promoted for AI-4 (13 L2), AI-5 (65 L2), CAM-2 (25 L2); PRESERVE_SPECS_PENDING += 0x50E180, 0x600530,
  0x669650.
- HARNESS-4: writer/verifier split (#[hook(unverified = BATCH, route, hazards, recs)], SA_UNVERIFIED selection,
  catalog/verify-queue.csv + docs/verify/QUEUE.md, tools/verify/{bisect-batch.sh,writer-gate.sh,queue.py},
  PLAN §5.1). Unverified hooks never change default behaviour (demo: broken unverified hook, default run IDENTICAL).

## 2026-11-11 AI-6, HUD-2 and SCRIPT-3 integrated (main session)
- AI-6 (af46d8d4): 85 hooks (CGroupEventHandler responses, gang/partner tasks). HUD-2 (a6712480): 224 HUD/menu/
  font/text/radar ports, 21 L1 specs. SCRIPT-3 (aeb0a3fe): 14 of 25 ProcessCommands switches complete (982/1845
  cases), all 119 SA:MP opcodes outside 0To99 in Rust, CMissionCleanup add/remove.
- HUD-2's 17 orig! ST0 returns declared X87 broke the i686 build: converted to f64 `_st0` + X87 wrappers (+ f64
  adapters where `=> target` is a Rust port). PRESERVE_SPECS_PENDING += 0x4637E0.
- verify-all main-g2 (fresh DLL 3b33f75): IDENTICAL on all 11. States promoted (AI-6 40 L2, HUD-2 136 L2 + 11 L1).

## 2026-11-11 AUDIO-2, PED-3, POP-3, RW-4 (+ RW-5 unverified) integrated (main session)
- AUDIO-2 (aead9c69): 370 audio hooks. PED-3 (a074427b): 63 ped drivers (ProcessControl family, CPlayerInfo::Process,
  ped collision, player targeting). POP-3 (aa84c63d): 96 POP functions (car AI, population, gangs; 7 partial
  ports with ADR-002 fallback). PORT-RW-4 (afaf8637): 323 RW D3D9 driver/raster/skin/MatFX functions. WRITER-RW-5
  (a15b5ca1): callers-test fix for register save/restore, 94 unverified RW hooks (off by default).
- Two more X87 orig! returns (path-qualified `crate::common::X87` in sa-audio) converted to f64 adapters.
  POP-3's pathfind.rs taken whole (same ST0 fix). PRESERVE_SPECS_PENDING += 0x7FC2D0.
- verify-all main-g3 (fresh DLL): IDENTICAL on all 11. States promoted from the four verdict TSVs.
- Process: the shared scratchpad collided (VERIFIER-3 overwrote the main session's union.sh); the main session's
  helpers now live in scratchpad/main/, agents use their own subfolders.
- MOD-1 (mod support) added to the ROADMAP as nice-to-have after the native MVP.

## 2026-11-11 Writers integrated (main session): POP-4, WRITER-MVP-A/B/C/E (unverified, off by default)
- 637 unverified hooks now on master in the verification queue (catalog/verify-queue.csv); 601 catalogue rows move
  from specified to implemented with evidence "written, unverified (batch …)". They are installed only when selected
  with SA_UNVERIFIED, so default behaviour is unchanged.
- POP-4 also fixes a POP-2 bug in misc2::model_loaded (load state read 0x10 too far; [model*20 + 0x8E4CD0]); the two
  gang-ped hooks depending on it went back to unverified.
- keycells READERS merged to 20 entries (POP-3, POP-4, MVP-B, MVP-C).
- verify-all main-g4 (fresh DLL): IDENTICAL on all 11.
- Rules from the user today: sub-agents start no agents at all (PLAN updated); new files are named after their
  content, never after writer/batch (the 27 mvp*-named files will be renamed once their batches are verified).

## 2026-11-11 MVP-D, MVP-F, VERIFIER-1, VERIFIER-3 integrated (main session)
- WRITER-MVP-D (125 unverified: shadows, weather, FX render, vehicle wheels/suspension, CWorld::Process, replay,
  game logic) and WRITER-MVP-F (28 unverified: ProcessColModels partial, ped/vehicle entity collision,
  CEntity::PreRender, LoadWeaponData, script init). 682 unverified hooks in the queue.
- VERIFIER-1 (mvpa batches): fixed CPedTaskPair::Flush (protection chunk 0x403956 clears the ped slot), promoted 91
  hooks; finding: in-process L2 double runs are invalid for heap-mutating hooks (allocators, CReferences free list,
  colmodel cache, anim associations) → those need the split route. VERIFIER-3 (rw5 batches): promoted 17; 74 rw5
  hooks are never called by the corpus; verify-all now builds the checkout it runs from (worktree runs used
  main's DLL before). 108 promotions recorded as L2 in state.csv.
- verify-all main-g5 + main-g5b (fresh DLL 32279fd): IDENTICAL on all 11 (3 recordings re-run after the disk guard
  skipped them).

## 2026-11-11 STD-6 integrated (main session): native boot through Init1–3, plus everything it merged
- STD-6's branch (base master 927d4c2) carries STD-4/5/6 (native boot: main menu, New Game, Init1/2/3, main.scm
  running, CCamera::Process; blocked at ServiceAmbientGunFire 0x4DF210), PORT-AUDIO-3 final, PORT-HUD-3 final
  (incl. the GetStringWidth/UseCommonVehicleTexDicationary constant fix), PORT-CAM-3 (nested footprints),
  WRITER-VEH-BIG, WRITER-RENDER-BIG/-2, WRITER-MVP-I/J, VERIFIER-4 (AddParticle / ComputeEventResponseTasks fixes,
  SA_CRASH_LOG). Squash-merged as is (master was its ancestor). PRESERVE_SPECS_PENDING += 0x464C00.
- verify-all main-std6 (fresh DLL 4224b26): IDENTICAL on all 11.
- Catalogue: every address with a #[hook]/#[native] registration is at least `implemented` (1,067 rows moved);
  L2 promoted from HUD-3/CAM-3/AUDIO-3 verdicts and the VERIFIER-2/4 mvpb promotions.

## 2026-11-11 VERIFIER-5 integrated (main session)
- All 13 VEH-BIG drivers promoted (split 0/0 on driving and pause-restart, all-ports IDENTICAL on 11). Fix:
  CBike::ProcessControl's skipped-physics lean-recovery condition was inverted (0x6B9B88 `test cl,cl; je`).
  New reviewed normaliser rule RULE_LEAN_MATRIX_PADS (CBike lean-matrix pad words are stack residue).
- verify-all main-v5 + main-v5b (driving re-run after the disk guard skipped it): IDENTICAL on all 11.

## 2026-11-12 HARNESS-5 and X87-2 integrated (main session)
- HARNESS-5: `conv = reg("args = …; ret = …; keep = …; pop")` register conventions for #[hook] and orig!
  (sa_core::regconv, naked phase-A shims, X87 results in ST0 with all 80 bits, sa-hook regconv_check against
  the exe, L1 runner sets entry registers). 7 ports (unverified harness5-1), incl. AudioLog10 and
  ShadowRenderTriangleCB. Merged by applying HARNESS-5's own diff onto master's sa-macros/abi.
- X87-2: FPATAN/FSIN/FCOS/FSINCOS models fitted to the CPU (FPATAN 99.9995 % full-80-bit exact, FSIN/FCOS 99.5 %,
  stored f32 100 %); X87-returning hooks return all 80 bits in ST0; Process_FollowPed_SA hooked (replaces
  STD-6's native), GetHeading/FindPlayerHeading re-hooked, GetRoll unverified; RwMatrixRotate passes the CPU
  → KNOWN_CPU_MISMATCHES is empty.
- L1 tests 18/18 (a first run hung 55 min in ref.exe under Wine at 0 % CPU; re-run passed).
- verify-all main-hx (fresh DLL 7cc2a0d): IDENTICAL on all 12 recordings (menus-map-controls is new, by PORT-MENU).

## 2026-11-12 INTEGRATE-2 fast-forwarded onto master (main session)
- Agent INTEGRATE-2 applied each branch's own diff onto master 4235edc: WRITER-MVP-I..M, WRITER-REGCONV (26
  register-convention ports), VERIFIER-6 (28 render promotions, DrawTriPlants srand at_site, 0xC96438 normaliser
  exclusion), GEN-ALL-2 (generator regenerated on the integrated tree: 852 unverified hooks, 25 L1-pure), SRAND-FIX.
  Duplicates settled (see docs/verify/integration-2.md). verify-all int2: IDENTICAL on all 12; L1 18/18.
- Catalogue: VERIFIER-6 promotions → L2; every registered address at least `implemented`.

## 2026-11-12 INTEGRATE-3: the native path on master (main session)
- Agent INTEGRATE-3 brought STD-7/8/9 (native boot through in-world frames: sa-game, sa-platform, 870 static-initialiser
  natives, CRope::Update, the in-world render chain, `--start-in-world`), PORT-MENU (menu-1..3 verified), WRITER-CLUMP
  (CreateSkinnedClump tree), WRITER-MVP-N and WRITER-INTRO (58 CEventHandler/cutscene hooks) onto master e6013d9.
  patterns.py fixed (rows marked implemented by the catalogue no longer hide the generator's own addresses): 841
  generated hooks. Duplicates: 0x4D2E10 keeps STD-7's copy, 0x54EEA0 keeps MVP-K's hook (docs/verify/integration-3.md).
- Tests: sa-hook 58, sa-script 8, sa-core 70, L1 18/18. verify-all int3/int3b: IDENTICAL on all 13 recordings.
  Native smoke run (--start-in-world, offscreen) reaches Grove Street frames; the next native gaps (0x7331E0,
  0x410090) are ported on STD-10's branch, where CJ is visible and walks (not yet on master).
- Catalogue: every registered address at least `implemented`.

## 2026-11-12 INTEGRATE-4: VERIFIER-7 generated-hook promotions and WRITER-O (main session)
- VERIFIER-7 verified the 852 gen2 hooks: L3 MATCH on all 13 recordings with all on Rust; 25 L1 (1M cases each)
  and 514 split promotions; generator bug fixed (tail-call forwarders into void hand ports were generated as
  returning EAX — found by split on CGrassRenderer::FlushTriPlantBuffer). INTEGRATE-4 merged its patterns.py with
  INTEGRATE-3's fix, regenerated (841 hooks, 532 promoted, 309 unverified), and took WRITER-O's ports
  (CAEVehicleAudioEntity::AddAudioEvent, FxEmitterBP_c::Render/RenderHeatHaze, RtQuat functions).
- Tests: sa-hook 58, sa-script 8, sa-core 70, L1 18/18. verify-all int4: IDENTICAL on all 13 recordings.
- Catalogue: 532 generated addresses promoted (25 L1, 507 L2).

## 2026-11-12 INTEGRATE-5: VERIFIER-8, WRITER-P and Save/Load (main session)
- VERIFIER-8 (527 writer hooks, L3 MATCH on 13 recordings after 4 fixes: FUN_006070F0 3 stack args, ProcessVerticalLineSectorList
  argument order, IsEntityOccluded max corner, CreateIndexBuffer format argument) → 67 L2. WRITER-P: 15 boot hooks + 1 native.
  PORT-SAVE (SAVE-1): 73 Save/Load ports, 61 L2, F5 cross-load identical; ledger SAVE-1 in ROADMAP.
- Integration by agent INTEGRATE-5 (own diffs, generator regenerated). Tests: sa-hook 58, sa-script 9, sa-core 71, L1 18/18.
  verify-all int5: IDENTICAL on all 13 recordings (save-load: save bytes identical except 750 residue bytes, ledger SAVE-1).
- Catalogue: 128 addresses → L2; F1 30.88 %.

## 2026-11-12 SA:MP native groundwork and the browser proxy on master (main session)
- SAMP-3 (d1d019c, 88e23a8): crate sa-samp (NativeGame on the sa-samp-api traits, read side), frame dispatch points in
  sa-core samp.rs (Game::Idle / CGame::Process, inert without a handler), `sa-game --samp` host skeleton,
  docs/samp/native-plan.md, samp-rs networking verified on i686 Linux (join to our open.mp copy in ~1.06 s).
- SAMP-PROXY (b4161d9, ffc1ba9): crates sa-udp-proxy (WebSocket↔UDP relay with allowlist/limits/shim mode) and
  sa-net (DatagramTransport: UDP native, WebSocket wasm32); samp-rs join through the proxy in 1.13 s.
- verify-all samp3b (fresh DLL, sa-oracle rebuilt after a stale-binary first attempt): IDENTICAL on all 13 recordings.

## 2026-11-12 INTEGRATE-7: the native game, writers and SA:MP UI on master (main session)
- Agent INTEGRATE-7 brought onto master (picks only, 132 commits): the native path STD-10..14 (CJ visible, walking,
  window loop, deterministic mode, attractors, traffic, shadow fix, enter-car up to the door), WRITER-NATIVE,
  WRITER-ATTR, RENDER-SHADOW, PORT-AI-7/8 (generator regenerated; 0x5F2B80/0x5F3730/0x5F3B40 excluded for a
  call-arity bug), VERIFIER-9 (163 promotions minus 0x5F1FA0, kept unverified; LoadAnimFile header fix),
  PORT-VEH-3/4, WRITER-WORLD, WRITER-SAMP, SAMP-UI. Fixes: ped_attractor_base orig! X87 → f64 (phase A build),
  PRESERVE_SPECS_PENDING for 0x4F5F30/0x4F6000.
- Tests: sa-hook 58, sa-script 9, sa-core 72, sa-rw-wgpu 58, L1 18/18 after the fix. Native smoke run 316 ok, 0
  blocked. verify-all int7: IDENTICAL on all 13 recordings (int7b recheck of the final DLL: grove + driving IDENTICAL).
- Not in yet: WRITER-SAMP-2, SAMP-4/WEB-*/SAMP-START (samp-rs dependency path), STD-15, INTEGRATE-6.
- Catalogue: VERIFIER-9 promotions recorded; every registered address at least `implemented`.

## 2026-11-12 late morning: hand-off state (main session)
- See docs/CONTINUE.md for every unmerged branch, its head and what it holds, and the next steps.
- Reached on branches today (not yet on master): native driving (STD-15), climbing (WRITER-CLIMB), sa-game in the
  browser (WEB-1/2/3: boot → world → walk/sprint/jump/climb/drive, WebGL2 + WebGPU, input, full window, audio),
  SA:MP in the browser through sa-udp-proxy (join, class selection, spawn, chat, interiors), SA:MP startup 1:1
  (SAMP-START, samp.dll patch points), SA:MP UI compared with the original client (SAMP-UI-2).
- samp-rs: branch sa-engine/adapter fast-forwarded to fb89010 (SAMP-4's 13 verified commits) with the user's approval.

## 2026-11-12 INTEGRATE-6 / VERIFIER-10 / VERIFIER-11 on master (main session)
- INTEGRATE-6's commits (rebased by it onto 7a1d1a68, verified there: verify-all i6-final/i6-final2 IDENTICAL on all 13)
  applied unchanged onto master: VERIFIER-10 (void-forwarder generator fix, 23 promotions), the audio-thread Start fix
  (ResumeThread through the original call sites — the stream thread ran free in determinism mode), L3-batch status
  for 157 SA:MP-priority batches (union L3 MATCH 13/13), 370 SA:MP-priority promotions (232 L2, 136 split, 2 L1).
- Catalogue: 393 addresses promoted; F1 34.34 %.
- Open (docs/verify/integration-6.md): generators treat catalogue state `implemented` as verified, so regenerating
  now would drop `unverified` from generated hooks — fix the generators' promotion rule before the next regeneration;
  CShopping::LoadStats, 0x606890, CObject::Render show L2 differences worth a writer's look.

## 2026-11-12 INTEGRATE-9: one tree for everything (main session)
- master moved to INTEGRATE-9's 29ee0fc0: INTEGRATE-8's tree (web build, SA:MP native + browser, driving, climbing,
  browser audio, SAMP-4/START/UI-2) with every master commit since 518819bb re-applied (INTEGRATE-6 promotions,
  docs, samp-rs/ monorepo import). Every one of the 15 master commits is present by subject; origin/master
  (d95bb320) is an ancestor, so the push is a fast-forward.
- sa-game takes samp-client / samp-proto / samp-saa from samp-rs/ by path; no git+file sources left; every document
  and script points at samp-rs/ (historical records annotated); auth_table.txt expected under re/samp/ (not committed).
- Generator fix: ai_patterns.py keeps listed hooks unverified until L1/L2/L3 (INTEGRATE-8's regeneration had dropped
  `unverified` from 554 PORT-AI-7 generated hooks; restored). Queue: 2,328 unverified hooks.
- Tests: sa-hook 58, L1 18/18, all lib tests, samp-rs 638; builds (DLL, native ± samp, wasm ± samp); smoke 0 blocked.
  verify-all int9: IDENTICAL on all 13 recordings.

## 2026-11-12 INTEGRATE-10: SAMP-FX on master (main session)
- SAMP-FX's 16 commits (skin/animations/frontend sounds/special actions/give-weapon + HUD icon, game-clock toggle,
  SA:MP files served to the browser, samp's main.scm by default with CHeli::SearchLightCone 0x6C58E0 — no more
  "Fat +" box while connecting — and the adaptive web-audio latency) fast-forwarded onto master.
- Tests: lib tests, samp-rs 638, builds (DLL, native ± samp, wasm + samp), smoke 0 blocked, native SA:MP join
  0 blocked with no "Fat" box and the weapon icon shown. verify-all int10: IDENTICAL on all 13 recordings.
- Open: HUD ammo after give-weapon reads 143-7 instead of 100; inflight.py does not see #[sa_core::native( registrations.

## 2026-11-14 — MOD-1 analysis: binary compatibility with existing .asi mods (main session)
- Checked the architecture: arena keeps globals at the original addresses, ports use original layouts and calling conventions, vtables sit in `.rdata`; but in phases B/C `orig!` `=> path` calls Rust directly (mod entry patches would be bypassed), `.text` is mapped read-only and never executable (mid-function patches have nothing to act on), D3D9 objects are trap vtables (EndScene/Present hooks cannot chain), and ASIs need a PE loader + Win32/SEH.
- Estimate: route A (phase A as a product: original exe + sa-hook, patch detection → per-function fallback to the original bytes) ~1–2 months, medium; route B (native Linux without Wine) ~4–8 months, hard. User chose route A first; ROADMAP MOD-1 and the decisions list updated.

## 2026-11-14 — INTRO-2 on master (main session)
- Fast-forwarded master to INTRO-2's head ca33426a (44 commits on 97e55daf): AUDIO-STREAM (5), INTRO-1 (12, without its AUDIO-STREAM copies), SCRIPT-5 (13), intro ports batches intro3-1..16 (StartPlaybackRecordedCar 0x45A980, IsTargetBlocked ×2, FrameUpdateCallBackNonSkinnedWithVelocityExtraction, JoinCarWithRoadSystemGotoCoors, CreateDeadPedMoney, CastShadowSectorList, …), native --fixed-step cooperative audio threads (determinism), wasm build fix.
- Evidence from the agent: verify-all intro2b 4,5 IDENTICAL 13/13 at 40bd7b4e (later commit docs only); lib tests, i686/wasm builds clean; intro runs 0 blocked through the police-car ride and the Jefferson drop-off; BMX ride blocks at CPed::KillPedWithCar 0x5F0360. Details: docs/verify/integration-intro2.md.
- Next for the intro: KillPedWithCar, the route east to Grove Street, arrival cutscene, Big Smoke marker. Integration notes: FRAMELIMIT also carries the CAEDataStream ctor 0x4DC620 (keep one), WRITER-SAMP-2 also ports 0x6E91D0.

## 2026-11-14 FRAMELIMIT: the Display Setup frame-limiter toggle natively (agent a1c6b8a6b0d220dec)
- Why the user's play build stopped on `hw_get_beat_info`: when the limiter is OFF, WinMain (0x748D62) calls
  `AudioEngine.IsBeatInfoPresent` -> `CAEAudioHardware::GetBeatInfo` in every iteration. ad85454a ports GetBeatInfo and
  GetTrackInfo. With INTRO-2's working stream thread, a track is active from boot, so the path runs `GetTrackInfo` ->
  the `CAEDataStream` constructor (master's, from AUDIO-STREAM) in every iteration: 2,283 calls in the repro, 0 blocked.
- The toggle itself: `psCameraShowRaster` (0x745240) passes the vsync flag from `m_bPrefsFrameLimiter`, so every toggle
  resets the D3D9 device (`_rwD3D9RasterShowRaster`). The restore callback 0x5BA120 sets
  `g_realTimeShadowMan.m_bNeedsReinit`, and the next `CRealTimeShadowManager::Update` reached the unported `ReInit`.
- Ported from the 1.0 US bytes (all unverified): `CRealTimeShadowManager::ReInit` 0x706870 (framelimit-render). Then
  the world blockers the shifted timing reaches on the Grove Street walk (framelimit-ai): `CEventVehicleToSteal`
  dtor body 0x4AF700, `CTaskSimpleSetPedAsAutoDriver::ProcessPed` 0x649020,
  `CTaskGoToVehicleAndLean::CreateNextSubTask` 0x664590, `CTaskLeanOnVehicle::ProcessPed` 0x6648C0,
  `CEventVehicleCollision` dtor body 0x4AC920. Also `CEventVehicleCollision::AffectsPed` 0x4B2EE0, taken unchanged
  from CARJACK (worktree-agent-a271da3abdc02b710, 66be0e6c, batch carjack-1) except for a clippy allow. CARJACK also
  registers 0x4AC920 (jack_events.rs), so keep one of each when integrating. sa-game: `SA_GAME_KEYS` accepts `esc`.
- Repro (offscreen, deterministic; scripts in target/framelimit/scripts/run.sh): `--start-in-world --fixed-step 40
  --frames 2600`, `SA_GAME_SCRIPT_SKIP=1`, SA_GAME_KEYS = esc 100, down x5, enter 160, down x2, enter 192, down x6,
  enter 246, down 270, enter 280/300/320 (ON->OFF->ON->OFF), esc 360/380/400/420, w 450-900 and 1500-1800.
  Result: 2616 iterations, 0 blocked. gta_sa.set byte 0xB35 (`m_bPrefsFrameLimiter`) is saved as 0. Window run
  (headless gamescope + xdotool, same path, target/framelimit/scripts/win.sh): 0 blocked. The limiter ON runs at 25 fps
  (median 39-41 ms). OFF runs at 60-115 fps in the menu and 33-40 fps on the walk. A fresh run that loads the saved
  OFF setting runs at 39 fps (median 22.7 ms). In a car with the radio on (`SA_GAME_TELEPORT=60:2456.5:-1761.7:13.6`,
  F), the toggle and GetTrackInfo cause 0 blocks. That drive later hits `ProcessTrainCrossingBehaviour` 0x5A0B50,
  which it also hits without the menu. Evidence (not committed): target/framelimit/.
- verify-all framelimit-final (lanes 1,2,3,6) with `SA_UNVERIFIED=framelimit-render,framelimit-ai,carjack-1,stream-1,batch`
  (18 hooks installed): IDENTICAL on all 13 recordings. The DLL was built on f14da1d0; 9a40b1e6 changes only sa-game
  and doc comments. The branch is rebased on 9a40b1e6, and the repro and window run were repeated there (0 blocked).
- Gate: sa-hook lib tests 58/58, i686-windows DLL, wasm32 sa-ports, queue regenerated. The clippy -D warnings
  failures are already on master: sa-pop carctrl4.rs doc lists, sa-game crt.rs/detheap.rs, sa-render
  postfx_speed_fx.rs.

## 2026-11-14 — FRAMELIMIT on master (main session)
- Fast-forwarded master to FRAMELIMIT's 283d0235 (rebased on 9a40b1e6): CRealTimeShadowManager::ReInit 0x706870 (the vsync toggle resets the D3D9 device; the reset callback 0x5BA120 makes the next shadow update reinit), plus walk blockers surfaced by the menu timing (0x4AF700, 0x649020, 0x664590, 0x6648C0, 0x4AC920) and CEventVehicleCollision::AffectsPed 0x4B2EE0 copied from CARJACK; `SA_GAME_KEYS` accepts `esc`.
- Agent evidence: scripted toggle ON→OFF→ON→OFF then 2616 iterations, 0 blocked; window run 0 blocked; gta_sa.set byte 0xB35 saved; verify-all framelimit-final IDENTICAL 13/13 with the new batches on. Main session: sa-hook lib 58/58, i686 sa-game build, play package refreshed.
- CARJACK registers 0x4AC920 and 0x4B2EE0 too: keep master's when it lands. Driving later hits ProcessTrainCrossingBehaviour 0x5A0B50 (independent of the menu).
## 2026-11-14 — CARJACK: pulling drivers out of their cars natively (agent, branch worktree-agent-a271da3abdc02b710)
- Blocker from the play build: `calls unported v_task_make_abortable (0x6486f0)` right after a drag-out. Ported from
  the bytes (batch `carjack-1`, 32 hooks + 1 native, all unverified; rebased on master 03c9d93e, whose FRAMELIMIT
  registrations of 0x4AC920 and 0x4B2EE0 — the same bodies — replaced CARJACK's): the slow drag-out family
  (`crates/sa-ai/src/car_slow_drag_out.rs`: 0x648100, 0x648180, 0x64E060, SayJacking 0x6497F0, 0x64C190, 0x64E180,
  0x64C220, 0x648520, 0x6485A0, 0x64C6A0 (+ keycells reader 0x402265), 0x64C810, 0x6486F0, 0x648A10), jack events
  (`jack_events.rs`: CEventPedEnteredMyVehicle ctor/dtor 0x4AEAC0/0x4AEB90, CEventSeenPanickedPed dtor 0x4B2130,
  CEventVehicleCollision dtor 0x4AC920), CEventVehicleCollision::AffectsPed 0x4B2EE0, CTaskSimpleCarDrive's close-door
  callback 0x63C840, CTaskComplexLeaveCarAndFlee ctor/CreateNextSubTask 0x63BF90/0x642390, the retaliation tasks
  (`kill_ped_on_foot.rs`: CTaskComplexKillPedOnFoot dtor 0x620EF0, CreateSubTask 0x625E70, ControlSubTask 0x626260,
  CreateNextSubTask 0x62B150, CreateFirstSubTask 0x62B490, taunt 0x65E9A0; `retaliation_car_tasks.rs`:
  CTaskComplexDestroyCar::ControlSubTask 0x6288C0, CTaskComplexDragPedFromCar::CreateFirstSubTask 0x643D00,
  CTaskComplexDestroyCarArmed ctor 0x621F50 (the original registers its vehicle reference at its own argument slot;
  the port uses a static cell — see the doc comment) and CalculateSearchPositionAndRanges 0x628C80,
  CTaskComplexScreamInCarThenLeave::CreateSubTask 0x642510 (+ keycells reader 0x40837C),
  CTaskSimpleCarOpenDoorFromOutside::MakeAbortable 0x64AD00),
  CAEPedSpeechAudioEntity::GetCurrentCJMood 0x4E53B0 (sa-audio `cj_mood.rs`), CPed::UpdateStatLeavingVehicle 0x5E01B0
  (lone `ret`, native). sa-ai's std7 NATIVES now lists 16 generated `#[native]`s no list carried (0x655E00 was a
  blocker). Dropped in the rebase: 0x485B10 / 0x719360 (INTRO-2 ported them).
- `SA_GAME_JACK=ITER[:DIST[:SIDE]],...` (exploration): teleports CJ in front of the nearest non-police driven car
  (drivers stop for a ped in their path) and prints the jacked driver's state for 400 iterations.
- Evidence on master ed2becfc: `SA_GAME_SCRIPT_SKIP=1 sa-game --start-in-world --fixed-step 40 --frames 4000` with
  `SA_GAME_JACK=1250:7,1650:7,2050:7,2450:7,2850:7` and F/W keys per jack (target/carjack/jack-summary.txt): 4 drivers
  jacked (models 475, 600, 439, 475), each dragged out (state 50 → 0 → 1, out of the vehicle, walks away), CJ drives
  off each time; 4000 iterations, blocker list empty. On the rebased head (FRAMELIMIT changed the traffic, so the route
  changed): `SA_GAME_JACK=1600:7,2000:7,2400:7,2800:7`, keys `f:T+30:T+34,w:T+200:T+300,f:T+340:T+344` per jack T,
  `--frames 3600`: 3 drivers jacked (492, 401, 517), 0 blocked (target/carjack/rebased-jack-summary.txt; at 4500
  frames ProstituteSolicit::IsTaskValid 0x661BB0 blocks at 3709). Contact sheets + MP4 in target/carjack/.
- Other routes still stop on: CTaskSimpleGunControl::ProcessPed 0x625270, ProstituteSolicit::IsTaskValid 0x661BB0,
  KillCriminal::ControlSubTask 0x68E950, WaitForPed ctor 0x683340, GenerateOneEmergencyServicesCar 0x42B7D0,
  KillPedOnFootMelee::CreateSubTask 0x626AC0 (+ its ControlSubTask 0x626D90 /
  CreateNextSubTask 0x62BC10), CTaskSimpleGoToPointFine::ProcessPed 0x663540 (+ SetBlendedMoveAnim 0x65EF80),
  CVehicle::IsSphereTouchingVehicle 0x6D84D0, UpdateCarAI's law-enforcer/wanted path (police after a jack),
  MakeTyresMuddySectorList 0x6AE0D0, 0x649550, 0x6DF3D0, IsEntityBlockingTarget 0x5F3970, 0x661BB0, the walk-into-fire
  events 0x4B6840/0x4B1F00, burning car 0x4DCBE0; KillPedWithCar 0x5F0360 (INTRO-2 lane) and 0x649020 (FRAMELIMIT).
- Gates on the rebased head: sa-hook 58/58, ai_patterns.py --check clean, queue regenerated (2437 unverified hooks),
  writer gate PASS hooks/queue/i686/wasm32/hooktest (l1test/l1 skipped: no L1-pure hook); host clippy fails only on
  master's sa-game det.rs `set_busy_wait_handler`, i686 clippy only on master's `draw_quad_bits` (clippy of sa-ai,
  sa-audio, sa-core and the Linux sa-game target clean). verify-all carjack2 (lanes 4,5,7): IDENTICAL on all 13.

## 2026-11-14 — CARJACK on master (main session)
- Rebased CARJACK (03471585, 10 commits) onto 0f751942 in a temporary worktree (queue conflicts resolved by regeneration: 2438 unverified hooks, 380 batches) and fast-forwarded master. 32 hooks + 1 native, batch carjack-1: the CTaskComplexCarSlowBeDraggedOutAndStandUp / CarSlowDragPedOut / CarSlowBeDraggedOut family (0x6486F0 was the user's blocker), CEventPedEnteredMyVehicle, KillPedOnFoot base, DestroyCar / DestroyCarArmed / DragPedFromCar / ScreamInCarThenLeave, LeaveCarAndFlee, GetCurrentCJMood, 16 previously unlisted generated natives; `SA_GAME_JACK` exploration knob.
- Agent evidence: verify-all carjack2 IDENTICAL 13/13; 3 drivers jacked, 0 blocked over 3600 iterations. Main session on the rebased head: sa-hook lib 58/58, ai_patterns --check clean, i686 build, the jack repro 0 blocked over 3600 iterations; play package refreshed. Ledger entry for 0x621F50 (reference cell).
- Frontier the agent saw: jacked peds fighting back (GunControl 0x625270, KillPedOnFootMelee), police after a jack (UpdateCarAI law-enforcer path), GoToPointFine::ProcessPed 0x663540, ProstituteSolicit::IsTaskValid 0x661BB0, IsSphereTouchingVehicle 0x6D84D0, burning car 0x4DCBE0.
## 2026-11-14 SAMP-CLASSPANEL: SA:MP class-selection panel in the engine-drawn UI (agent, branch worktree-agent-a386e2d86a077ef0c)
- `crates/sa-samp-ui/src/classpanel.rs`: samp's class-selection GUI dialog (0x1026eaa4) per samp-rs `class_panel` /
  `ui-rendering.md` §4.2: 310 × 40 on 0x960A0A0A at ((W − 310) / 2, H − 90), `<<` `>>` `Spawn` 90 × 30 DXUT buttons
  with the hover look; drawn after the chat input, not with the scoreboard; DXUT press-release clicks. sa-samp-api v4:
  `SampUi::show_class_panel`, `UiEvent::ClassPanelClicked(ClassButton)`.
- The skin is sampled trilinearly like samp's D3DX sprite (box-filtered mip chain written per level,
  `rwFILTERLINEARMIPLINEAR`); found by the pixel comparison (bilinear rims were up to 60 levels off).
- `sa-game/src/samp_net.rs`: panel shown / kept / hidden as samp-rs; clicks act as Left / Right / Shift through
  `classsel` (guards, sounds 1052 / 1053, RPC 128 / 129); the class tick waits while a dialog is open; Shift ignored
  with the chat input open. Browser: the page hands the pointer to the UI while it shows its cursor (`uiCursor`).
  `SA_GAME_POINTER` (native) and `headless.mjs --pointer` (browser) for scripted clicks.
- Verified: sa-samp-ui 44 tests (6 new), sa-samp-api / sa-samp tests, sa-game samp tests (i686); clippy -D warnings
  for sa-samp-ui / sa-samp-api / sa-samp (host, i686 all targets, wasm32) and sa-game --features samp (i686 lib/bin,
  wasm32 lib); samp-rs: `cargo test -p samp-client` 201 + 1, clippy i686-pc-windows-gnu clean. Pre-existing, not
  touched: sa-game host clippy (dead `call_this` / `call_cmp` in crt.rs), sa-game i686 `--all-targets` clippy
  (`is_multiple_of` in detheap.rs tests).
- Runs against the shared open.mp server (UDP 7813): browser (headless Brave, GPU, own serve.py on 8660) and native
  i686 offscreen: panel at class selection, hover, `>>` → "requesting class 1 (sound 1052)", `<<` → class 0
  (sound 1053), `Spawn` → RPC 129, spawn response 1, spawned; 0 blocked. Original gta_sa.exe + samp.dll on lane 6
  against the same server with xdotool clicks: side-by-side at 800 × 600, the same scene; the panel matches to the
  pixel except the GUI font 0 labels (about 12 % narrower in the engine; ledger, overlay.md §6.2). Evidence under
  `target/classpanel-evidence/` (not committed).

## 2026-11-14 — SAMP-CLASSPANEL on master (main session)
- Rebased SAMP-CLASSPANEL (792d81ee, 3 commits) onto c64cb63f (doc conflicts unioned) and fast-forwarded master: the SA:MP class-selection panel (`<<` `>>` `Spawn`) in sa-samp-ui (sa-samp-api v4: `show_class_panel`, `UiEvent::ClassPanelClicked`), trilinear GUI skin, native client wiring (clicks act as Left/Right/Shift through classsel), browser pointer release while the SA:MP cursor shows, `SA_GAME_POINTER`, `headless.mjs --pointer`.
- Main session: `sa-game --window` also releases the pointer while the SA:MP UI cursor shows (2caa6117). Checks: sa-samp-ui 44, samp-client 201+1, sa-hook lib 58/58, i686 clippy with and without `samp`; native join against the local open.mp server with scripted clicks: Next → class 1 (sound 1052, skin 105), Spawn → spawn requested and spawned, 0 blocked over 2800 iterations.
- Agent evidence: panel pixel-identical to the original 0.3.7-R4 except label glyph width (~12% narrower, UI font size rule; ledger + overlay.md §8 item 9).

## 2026-11-14 PEDHIT: vehicles and the player hitting peds natively (agent a8dce841187c8089c)
- Goal: any vehicle (car, BMX) or the player on foot hits peds (bump, fast hit, run over, thrown over the bonnet,
  killed) natively, 1:1 with gta_sa.exe 1.0 US, and play continues. Branch `worktree-agent-a8dce841187c8089c`,
  rebased on master 4d97535d (CARJACK, SAMP-CLASSPANEL).
- Ported from the 1.0 US bytes, batch `pedhit-1` (13 hooks, unverified, route L2): `CPed::KillPedWithCar` 0x5F0360
  (1139 instructions: the gates, the strong path with pad shake, side hit / runover / bonnet-roof throw with lift
  and bonnet removal, the weak bump, crime report, the damage event and knock-down anims, the force on the car),
  `CPed::MakeTyresMuddySectorList` 0x6AE0D0 (bloody tyres; the spec's "uninitialised bike local" is set to 0 by the
  prologue), the `CEventGotKnockedOverByCar` / `CEventPedCollisionWithPlayer` destructors 0x4B1C10 / 0x5FED80,
  `FxInfoDir_c::GetValue` 0x4A64D0, `CPickups::DoMoneyEffects` 0x454E80, `CreatePickupCoorsCloseToCoors` 0x458A80,
  `GenerateNewOne_WeaponType` 0x457380, `CShadows::AddPermanentShadow` 0x706F60, `CGarage::CountCarsInside`
  0x4495F0, `CTaskComplexProstituteSolicit::IsTaskValid` / `CreateSubTask` 0x661BB0 / 0x666360,
  `CTaskComplexEnterAnyCarAsDriver::CreateFirstSubTask` 0x643510. Phase B natives: `Fx_c::TriggerWaterHydrant`
  0x4A0D70 and `CIdleCam::SetTarget` 0x50A280 (entries are jumps into protection chunks read as keys; the chunks'
  loads evaluated on the image: `[0xB6F03C]` and `[0xB7CB84]`), `FxSystem_c::GetPlayStatus` 0x4AA900 (5-byte body).
  0x4AC920, 0x4B2EE0, 0x664590, 0x6648C0 were also needed and ported here too; master's (FRAMELIMIT / CARJACK)
  registrations were kept and these dropped at the rebase (same bodies from the same bytes).
- sa-game (exploration): `SA_GAME_TELEPORT` takes `;`-separated entries and moves the player's vehicle (with a
  heading); the veh-list prints the main-loop iteration, vehicle headings / speeds and, with
  `SA_GAME_VEHLIST_PEDS=R`, peds within R units (state, health). Scenarios: `tools/e2e/scenarios/pedhit.sh`.
- Runs (offscreen, `--fixed-step`, deterministic, 0 blocked, 0 crashed):
  car 2916 iterations (KillPedWithCar 14 calls: ped bumped to 61 hp at 375, one killed (54 → 55) and one bumped
  to 70 hp at 467-478, slow approach, hydrant water, blood pool, dropped weapon); foot (the main session's repro,
  CEventPedCollisionWithPlayer at ~918) 3516 iterations; intro BMX 7316 iterations (ped killed by the BMX at 5082).
  Evidence (not committed): `target/pedhit-evidence/{car,foot,intro-bmx}{.mp4,-sheet.png,/}`.
- Not covered: a motorbike (class 9; no parked one near Grove Street, the traffic bikes are police bikes:
  jacking is CARJACK's). Next blockers seen: `CTaskComplexProstituteSolicit::ControlSubTask` 0x6669D0 (~700
  instructions, when the player idles in a car next to a prostitute), `CObject::ProcessTrainCrossingBehaviour`
  0x5A0B50 (driving south past the tracks). One early intro run (pre-rebase build, other route) crashed at
  iteration 6803 (null + 0x44 in a Rust port); not reproduced on the final tree.
- Gate: hooks PASS, sa-hook lib 58/58, i686 DLL, wasm32 sa-ports, l1test PASS; host clippy clean for every crate
  except sa-game (pre-existing: `sa_core::orig::set_busy_wait_handler` is 32-bit only, so the 64-bit sa-game build
  fails on master too); i686 clippy only the known `draw_quad_bits`. Queue regenerated (2451 unverified in 381
  batches, `--check` clean).
- verify-all pedhit 1,2,3,6 with `SA_UNVERIFIED=pedhit-1` on the c64cb63f-based head: IDENTICAL on all 13
  recordings (17 subsystems each). The 4d97535d rebase brings only sa-game / SA:MP UI changes (no game-function
  ports, nothing in the phase A DLL); hooktest, queue and the three scenarios were rerun there with the same results.

## 2026-11-14 — PEDHIT, clippy clean-up and NEWGAME on master (main session)
- PEDHIT (de4c178a, batch pedhit-1, 13 hooks + 3 natives: CPed::KillPedWithCar 0x5F0360, MakeTyresMuddySectorList, the CEventPedCollisionWithPlayer / GotKnockedOverByCar dtors, pickups/money/weapon drop, blood pool, hydrant water, ProstituteSolicit::IsTaskValid/CreateSubTask, EnterAnyCarAsDriver::CreateFirstSubTask) fast-forwarded. Agent: verify-all IDENTICAL 13/13 with pedhit-1 on; car / foot / intro-BMX scenarios 0 blocked (`tools/e2e/scenarios/pedhit.sh`). Main session: car scenario 0 blocked over 2916 steps on master.
- Clippy: workspace host clippy and i686-pc-windows sa-hook clippy clean again (64-bit check build gates the busy-wait handler and registry callers; phase A dead `draw_quad_bits`; `is_multiple_of`); writer gate all PASS after the queue regeneration (b5f7ca4e).
- NEWGAME's last two commits (CMenuSystem::Initialise 0x5822D0 as a phase B native, CTaskComplexWaitForPed ctor/ControlSubTask, --start-in-world on every new game) cherry-picked; its CAEDataStream / CAEStreamingDecoder constructors and stream-thread fix dropped (master has them); duplicate `esc` arm removed. sa-hook lib 58/58, i686 build. Not re-proven by a scripted pause-menu run (menu position not found); the user's report is the check.
- Found while scripting the pause menu: Enter on pause-menu item 4 (down ×3) SIGSEGVs in a Rust port (host pointer 0x58dc4f9c) — queued for the frontier agent.

## 2026-11-14 SAMP-UI-DEDUP: one SA:MP UI, samp-rs's, drawn through a backend trait (agent, branch worktree-agent-ad98c9348a6e71dda)
- Inventory and design first (docs/samp/overlay.md §10.1–10.2): `crates/sa-samp-ui` (4,894 lines) re-did samp-ui's
  logic, samp-client's `UiCore` routing and its D3D renderer's layout. Per the user's direction (one implementation,
  abstraction where it applies; ADR-003 §3.1 / §3.2): `sa_samp_api::ui::UiBackend` (API v5, replaces `SampUi`);
  samp-client's render code is platform-independent and generic over it (`ui::render`), the pass state and clicks are
  `ui::session`, the adapter-side host is `ui::host::UiHost`; the D3D9 renderer implements the trait
  (`ui/d3d/renderer`), the engine backend is `sa-samp/src/overlay` (atlas incl. the user's gtaweap3.ttf / sampaux3.ttf,
  D3DX DrawText layout, skin mip chain). samp-ui gains `dxut` (skin + control geometry), `classpanel`,
  `textdraw::sprite_source`, `EditKey::from_vk`; `send_outgoing` moved into `ui::core`. sa-samp-ui deleted; sa-game's
  native client and `--samp-ui-demo` / `SA_SAMP_UI_SCENE=ref` drive `UiCore`.
- Behaviour differences recorded with their resolution (overlay.md §10.4: chat ring paging, `●` mask, GTAWEAPON3 / SAMPAUX3
  glyphs, label anchoring, scoreboard double click, cursor modes, WM_CHAR emulation, game-text buffer, D3DX VCENTER).
- Verified: samp-rs `cargo test --workspace` (samp-ui 103, samp-client 205 incl. 4 new pass tests on a recording
  backend); sa-samp 30 (atlas, symbol cmap, layout, backend), sa-samp-api, sa-game --features samp lib; clippy -D warnings:
  sa-samp / sa-samp-api host all targets, i686, wasm32; sa-game --features samp host, i686, wasm32 lib; samp-client
  i686-pc-windows-gnu all targets and host. UI-layer comparison old vs new (target/uicmp): chat, chat input, six
  dialog styles, scoreboard, F7, class panel identical to the pixel; only the death window and the password mask differ
  (§10.4 items 2, 3). Native join on open.mp 7813 with SA_GAME_POINTER: class panel region 0 px different incl. hover,
  `<<` → class 2. Not done: browser run, D3D call-stream diff on the original exe.
- Lines: sa-samp-ui 4,894 → 0; samp-ui 6,066 → 6,344; samp-client ui 8,645 → 9,329; sa-samp overlay 847 → 1,876;
  sa-samp-api ui 373 → 350; sa-game samp_net / demo / scene 1,897 → 1,638. Total 22,722 → 19,537 (incl. new tests).

## 2026-11-14 — SAMP-UI-DEDUP on master (main session)
- Fast-forwarded master to 9f827946: one SA:MP UI, samp-rs's own, drawn through `sa_samp_api::ui::UiBackend` (API v5): samp-client's pass (`ui::render`, `ui::session`, `ui::host`) generic over the trait, its D3D9/DXUT code the original-exe backend, `sa-samp`'s overlay the engine backend; shared geometry moved into samp-ui (dxut, classpanel, scoreboard/deathwindow layout); `crates/sa-samp-ui` deleted (22,722 → 19,537 lines incl. new tests). Behaviour differences resolved by samp-rs/spec, listed in overlay.md section 10.4.
- Main session: the browser build panicked at start-up (`std::time::SystemTime` in the UI core; wasm32 has no std clock) → web_time; GETTING-STARTED and the Makefile `test` target updated. Checks: samp-rs workspace tests green, sa-samp 30, sa-samp-api; browser (headless Brave, GPU) joins the local open.mp server, class-panel Spawn click → spawned, 0 blocked.
- Not re-measured: the D3D9 call stream on the original exe (builds and clippy clean).

## 2026-11-14 FRONTIER-1: the phase B frontier, the user-hit blockers and the pause-menu crash (agent adab96e32a9742efe)
- Integration (step 0): WRITER-WORLD-2's own commits world6-1/3/4/5 cherry-picked (world3/5/6-2 were on master
  already); its BreakObject_c::DoCollision 0x59E1F0 (world5-1) moved into master's BreakObject_c port (master's file
  had every other method). WRITER-SAMP-2's a089a405 / 855a5ba0 picked (FireInstantHit, FireProjectile, AddProjectile,
  bullet trace / impact, checkpoint render, water line test / split / wake / ripple, glass, boat buoyancy); master's
  0x49FF20 (fx_debris_wheel_grass), 0x706F60 (shadows_permanent) and 0x6E91D0 (waterlevel, intro3-9) kept, SAMP-2's
  copies dropped, and its 0x61D6F0 file kept as master's. The remaining "in-flight" addresses of both branches were
  `#[sa_core::native(` / `#[crate::native(` registrations master already has: `inflight.py` now matches path-prefixed
  attributes. FireInstantHit's determinism scrub (detwin.rs) and the hook no longer conflict: the installer takes the
  scrub stub as the hook's original (`detwin::redirected_entry`, win.rs `install_one`), so the original path keeps the
  scrub. keycells READERS merged (51). Generators regenerated (`--check` clean), queue regenerated.
- `tools/verify/frontier.py` (step 1): direct `=> unported` literal/table targets of `orig!` declarations and
  unregistered unknown/specified vtable slots, minus X-* rows and the protection's code; registered = `#[hook]` /
  `#[native]` (any path prefix), the lone-ret / static-init tables and sa-game's address entries; addresses another
  branch or agent worktree registers (inflight.py's `elsewhere()`) are marked and left out (`--all` shows them).
  Counts: after the integration 523 direct + 217 virtual (740); at hand-back 512 + 211 (723).
- Pause-menu crash (Esc, down x3, Enter = Stats): root cause in the phase B `orig!` path, not in a port.
  `stats_strings.rs` declares `CStats::GetStatValue` as `reg("args = stack") -> X87`; phase B dispatched every
  unported `reg(...)` declaration as `cdecl`, i.e. `extern "C" fn(u32) -> X87`, a struct return through a hidden
  pointer the registered entry (the hook's ST0 shim) does not take: the call returned into `orig::host::TABLE`.
  Fix: phase B calls the registered entry of a `reg(...)` declaration through a `RegFrame` / `call_regs` (the
  original convention, as phase A does); `call_regs` now aligns ESP to 16 at its `call` (the X87 shim's `movaps`
  faulted on the 4-aligned stack). The only other unported `reg` declaration with a registration whose convention
  differed was 0x4CAAE0 (`raster_copy`, EAX source, registered as a cdecl native): its declaration now calls the
  port directly. Repro `tools/e2e/scenarios/frontier.sh pausemenu`: 400 iterations, Stats pages drawn, 0 blocked.
- Ports (unverified): batch `frontier-1-1` (15): CTaskComplexProstituteSolicit::ControlSubTask 0x6669D0,
  CObject::ProcessTrainCrossingBehaviour 0x5A0B50, CPedGeometryAnalyser::IsEntityBlockingTarget 0x5F3970,
  CTaskSimpleBikeJacked ctor 0x648B90 (+ keycells reader 0x156DAD4) / ProcessPed 0x64C970,
  CTaskSimpleGoToPointFine ProcessPed 0x663540 / SetBlendedMoveAnim 0x65EF80, CTaskComplexKillPedOnFootMelee
  CreateSubTask 0x626AC0 / ControlSubTask 0x626D90 / CreateNextSubTask 0x62BC10, CTaskSimpleFightingControl ctor
  0x61DC10, CVehicle::IsSphereTouchingVehicle 0x6D84D0, the walk-into-fire event destructors 0x4B1F00 / 0x4B6840,
  CEventPlayerCollisionWithPed's destructor 0x5FEE80. Batch `frontier-1-2` (5): CTaskSimpleFight::FightHitPed
  0x61CBA0, CTaskSimpleFight::BeHitWhileBlocking 0x61C650, CPedGeometryAnalyser::ComputePedShotSide 0x5F13F0,
  Fx_c::AddBlood 0x49EB00, the complex-task destructor 0x6833D0. Catalogue names (CAT-17 name-corrections):
  0x4B1F00, 0x5FEE80, 0x61CBA0, 0x61C650.
- No new helper copies (ADR-003): `sa_math::x87::ci_acos` replaces sa-anim's and sa-script's private copies,
  `sa_world::world::common::entity_pos` is the shared entity-position helper, and the `orig!` declarations the new
  ports share live in sa-ai's / sa-render's / sa-veh's `common.rs`.
- Not done (handed to the next lane): GunControl::ProcessPed 0x625270, the police path (UpdateCarAI law enforcer,
  KillCriminal::ControlSubTask 0x68E950 (4.2 KB), GenerateOneEmergencyServicesCar 0x42B7D0), the burning car
  0x4DCBE0. New blockers the scenarios reach: CAEPedAudioEntity::HandlePedHit 0x4E1CC0 (melee, 2 iterations; nested
  jump tables and a protection fragment), CAutomobile::UpdateMovingCollision 0x6A1460 and
  CTrailer::SetupSuspensionLines 0x6CF1A0 (crossing run).
- Gates: writer gate on the fdd3277b-based head: hooks / queue / clippy / i686 / i686clip / wasm32 / hooktest / l1
  PASS; l1test 17/18 then 18/18 after queuing 0x4B1F00's preserved-ECX spec (the only failing test, rerun).
  sa-hook lib 58/58, generators `--check` clean, queue 2544 unverified hooks in 391 batches.
- verify-all (lanes 4-7): `frontier1` with frontier-1-1 + the integrated batches (samp2-*, world5-1, world6-1/3/4/5)
  IDENTICAL 13/13; `frontier2` with frontier-1-2 added, IDENTICAL 13/13 (17 subsystems each).
- Repros (`tools/e2e/scenarios/frontier.sh NAME OUT`, `--fixed-step 40`; call counts with
  `WRAP="gdb -q -batch -x tools/e2e/scenarios/frontier-counts.gdb --args"`): pausemenu 400 iterations, 0 blocked;
  solicit 4000, 0 blocked (ControlSubTask 0x6669D0 479 calls, IsEntityBlockingTarget 1); crossing 1200
  (ProcessTrainCrossingBehaviour 1944 calls; blocked on UpdateMovingCollision / CTrailer::SetupSuspensionLines);
  melee 2600 (FightHitPed 7 hits; blocked on HandlePedHit 0x4E1CC0 and ComputeDamageAnim 0x4B3FC0). The bike-jack,
  GoToPointFine and KillPedOnFootMelee ports have no scripted route yet (no call in these runs).

## 2026-11-14 — FRONTIER-1 on master (main session)
- Fast-forwarded master to fee1c2af (rebased on 18e878c9): WRITER-WORLD-2 and WRITER-SAMP-2 integrated (master's copies kept on duplicates), `tools/verify/frontier.py` (723 unported callees left: 512 direct + 211 virtual), the native `reg(...)` call path fixed (register frame + 16-byte stack alignment; root cause of the pause-menu SIGSEGV), 20 user-hit ports (frontier-1-1/2: ProstituteSolicit::ControlSubTask, ProcessTrainCrossingBehaviour, BikeJacked, GoToPointFine, KillPedOnFootMelee, FightHitPed, …).
- Agent: verify-all frontier2 IDENTICAL 13/13 with all the new batches on. Main session: sa-hook lib 58/58, i686 and wasm (samp) builds, pausemenu scenario 0 blocked (416 steps); play package refreshed.
- Still open (FRONTIER-2): GunControl 0x625270, police path, burning car, melee HandlePedHit 0x4E1CC0 / ComputeDamageAnim 0x4B3FC0, crossing 0x6A1460 / 0x6CF1A0.

## 2026-11-14 — DEDUP-1 pass 1: ADR-003 migration steps 1–3 (agent, branch worktree-agent-af0f2f8c9349698a3)
- Detector: `tools/verify/dupes.py` (`make dupes`): free functions defined in several crates (same names; identical
  bodies; `--renamed` = identical modulo parameter / local / generic names, bounds, path prefixes, integer spelling,
  implied turbofish, via `tools/verify/helpernorm.py`). Generated files, `#[hook]`/`#[native]` ports, `orig!`
  entries and tests excluded. Before → after this pass: renamed-copy groups across crates 175 groups / 956
  redundant copies → 124 / 227; exact-body groups 150 / 797 → 82 / 142; names in ≥ 2 crates 592 → 552.
- Step 1 (shared helpers): `sa_core::field` (rd/wr/at/rd32/wr32/rdf/rd8/ldx/rdp/rd3f/rd3u/copy3/addr32, g/gu/pa/p8,
  gf/gfu/gx, gr32/gw32/…, ptr_of), `sa_math::fpu` (x/xi/xc/st0/f32_of/ftol32/fbits/fabs/floor_f64 and the `fcom`
  tests le_or_nan/gt/ge_or_nan/lt/fcom_any/fcom_parity), `sa_world::world::common` (entity_pos + _f/_u/3,
  entity_type), `sa_core::crt::rand_at`, `sa_core::hooks::{total, concat}`. 790 copies became imports (local names
  kept with `use … as`); patterns.py / ai_patterns.py emit imports. sa-script gained a direct sa-world dependency.
  Only copies equal modulo renaming were merged; semantically different paths stay separate helpers (raw
  `MODEL_INFO_PTRS` address vs the `Global`, raw TIME_MS reads vs `timer::TIME_MS.get()`, `rd(a: usize) -> u32`
  style single-argument readers, samp-rs's raw-pointer readers).
- Step 2 (one front end): `sa-game/src/frontend.rs` = the input pipeline (keys pending until a frame presents,
  releases before a frame deferred, motion / wheel handed over once; `Session::iterate` keeps DirectInput's
  accumulation), the pointer rule `in_menu` (state ≠ IDLE, MENU_ACTIVE, SA:MP UI cursor), client-pixel scaling and
  the iteration driver. `game_window.rs` (winit) and `web.rs` (worker; new export `in_menu`, used by main.js) are its
  backends. Native now also applies the key-pending rule (previously web only).
- Step 3 (one binding per address): `tools/gen/bindings.py` (`make bindings`, `--check` in the writer gate):
  `catalog/bindings.csv` → `crates/<crate>/src/bindings.rs`; `--migrate` merged the literal-address `orig!`
  declarations that are identical in convention, types and target (9064 declarations → 1881 bindings in 15
  crates; owner crate by catalogue subsystem when every caller depends on it, else the highest common dependency).
  Literal-address declarations 19921 → 12738; addresses declared in more than one file 2837 → 2048 (max 138 → 42
  files). Left (pass 2): 3070 declarations whose types / target differ from the binding (mostly `u32` vs `GPtr<u8>`
  arguments, `u8` vs `u32` bools: merging them changes caller code), 574 with relative `=>` paths, 393 with
  non-primitive types, 126 under attributes / in macro bodies, 73 whose names are `orig::host` stand-in keys, and
  the generators' own declarations (step 5). Report: `tools/gen/bindings.py --migrate --dry --report FILE`.
- `tools/verify/state_check.py` (writer gate `state`): a row is implemented/L1/L2/L3 only with a registration or a
  Rust function documented with its address. 52 rows lowered to `specified` (evidence "not-ported", "deferred",
  "protection"; e.g. 0x5105C0 Process_M16_1stPerson, the vt_866db0 user-track player, HandlePedHit 0x4E1CC0).
- Gates: writer gate PASS for each step (hooks, queue, bindings, state, clippy, i686, i686clip, wasm32, hooktest,
  l1test, l1); make test; i686 sa-game (+samp); FEATURES=samp web build; samp-rs workspace tests (step 3).
- verify-all: step 1 `dedup1-s1` IDENTICAL 13/13; `SA_UNVERIFIED=all` 12/13 IDENTICAL, weapon-combat crashes in the
  rust run at frame ~1745-1857 (read of 0x9FC2) — the same on master 2767cc0c with `SA_UNVERIFIED=all`
  (`master-u-wc`), so pre-existing in some unverified batch, not from this pass. Step 2: DLL byte-identical to
  step 1 (sha 08e48c17…), `dedup1-s2` IDENTICAL 13/13. Step 3 (DLL 5f21fcbf…): `dedup1-s3u` (`SA_UNVERIFIED=all`)
  12/13 IDENTICAL + the same pre-existing weapon-combat crash (1792 common frames, as on master); `dedup1-s3`
  (verified set) IDENTICAL 13/13.
- Front-end checks: native window (headless gamescope, xdotool) step 1 vs step 2 binaries: menu screenshots with
  mouse moves pixel-identical; in game, mouse-look turns the camera. Offscreen 600 frames: same report. Browser
  (brave, own serve.py on 8662, the running open.mp 7813 / proxy 7781): SA:MP join, class panel `>>` and Spawn
  clicks with `--pointer`, spawned, 0 blocked (repeated on the step-3 build; native window and offscreen runs of the
  step-3 binary identical to step 2's).
- Also found: `tools/gen/audio_patterns.py --check` already failed on master (regeneration adds 3 hooks:
  0x4F4620, 0x59CD50, 0x59CB70); left as is.

## 2026-11-15 — DEDUP-1 pass 2: binding signatures from the original, ADR-003 steps 4, 5, 7 (agent, branch worktree-agent-a1a2b663eb9c1be36)
- Base: master 3b114f6a (fast-forwarded). Commits: A e43c5f6c, B 9dd4cff4, D 89cc5da7, E 3689046b, C 29b4fa4d +
  fbb0da85 + 8c8b4290, F (gates, docs, queue). Scratch scripts under target/dedup2 (not committed).
- A (binding conflicts): `tools/gen/bindings.py --resolve` (`tools/gen/bindings_migrate.py`) merged every declaration
  of a repeated or bound address into its binding. Where declarations disagreed, the signature came from the exe
  (`tools/lib/abi.py` on `tools/lib/exe.py`: the callee's `ret N`, ECX reads, `add esp, N` at call sites, the
  width each argument is read with, how the original's callers use the result: AL / AX movsx/movzx / EAX / ST0);
  a pointer where any caller passes one, `f32` where any passes a float; a binding that targets a port has the
  port's signature. Callers convert at the call expression only (a rustc-diagnostics fixer handled the calls the
  textual rewrite could not type). Round A: 4183 declarations merged, bindings 1881 → 2559, 324 signatures changed.
  Real mismatches with the original found (potential port bugs, all fixed at the call site):
  - results read wider than the original returns (AL / AX taken as EAX, the high bits undefined): 49 call sites —
    0x4076A0 IsVeryBusy, 0x420920 / 0x420950 node compares, 0x43AC40, 0x448D30 IsEntityEntirelyOutside, 0x44DD10
    AreNodesLoadedForArea, 0x454C70 pickup IsVisible, 0x4C74D0 FindOtherUpgrade (i16), 0x4D6760, 0x4DA030
    GetIsTypePhysical, 0x4DC670, 0x4EEA80 (2), 0x4EF570 / 0x4EF5D0 (u16), 0x53C4A0 / 0x53C4B0 CanSeeOutside /
    CanSeeWater, the CPad axes 0x53FB80 / 0x53FBD0 / 0x53FC90 / 0x53FD30 / 0x540040 / 0x541040 / 0x5410C0 (i16,
    8 sites), 0x55E710 IsWater and the nine 0x55EA30..0x55EB30 IsAudio* surface tests, 0x562330 IsInPursuit,
    0x5E0170 CPed::IsAlive (2), 0x5EFFE0 CPed::Say (i16, 6 sites), 0x5F1C40 CanPedTargetPed, 0x5F6A10 IsMember (2).
    These callers now get the original's width zero- / sign-extended (behaviour changes only where the callee left
    high bits set; verify-all IDENTICAL, so none of the recordings exercised a difference).
  - convention / arity: 0x49E650 Fx_c::Render is stdcall(2), declared thiscall(3) in sa-core; 0x4E9800, 0x531F20,
    0x540950, 0x540980, 0x65E7F0 are static (ECX unread), declared thiscall with a `this` (calls drop it; the menu's
    analogue callbacks take a closure); 0x7F3820 RwTextureDestroy declared with an extra argument; 0x61C740's reg
    form lacked `; pop` (the callee pops 4); 0x743C60 GetWeaponInfo reads `skill` as a byte (u8 now).
  - ports whose signature drops a result the original's callers use: 0x571B70 CEntity::RegisterReference (its EAX is
    returned by CCoverPoint::ReserveCoverPointForPed) and 0x4087E0 CStreaming::RequestModel (a script-brain caller
    keeps EAX's high word): the bindings keep `-> u32` and stay on the phase B registry (no ported twin) — the ports
    return nothing there (ROADMAP note).
- B: relative `=>` paths, typed struct pointers (binding by full path or `GPtr<u8>` + `.cast()`), local / global /
  path aliases (`Ent`, `CdPtr`, `MiPtr`, array aliases), lint-only attributes, L1 host stand-in keys (renamed to
  the binding's path), CVector-by-value spellings (the binding takes the vector; word callers build it), X87
  results (a plain-convention binding returning `X87` renders as `reg("args = …")`), the per-crate phase B adapters
  of registered ports (55 `*_native` / `*_port` / `*_abi` / `*_st0_rust` functions) replaced by the port. Bindings
  below their port's crate get a generated ported twin in the port's crate (`port` column, 297 rows) so phases B/C
  and the host L1 specs call the port directly as before (found: l1test failed on round A until the twins). Final:
  2633 bindings in 16 crates; outside the binding modules only C varargs stay declared per caller (96 declarations
  of 7 addresses, `bindings.VARARGS`); `--check` fails on any other. audio_patterns.py: the user-track player
  (0x59CB70 / 0x59CD50) and 0x4F4620 are excluded (decoding / file output stay original, state.csv "not-ported"),
  so its `--check` passes. Not done: the generators' own declarations (1223 word-level `u32` declarations in
  generated files, 515 of them at 350 bound addresses, 133 addresses in more than one generated file).
- C: one shared build script (`crates/phase_a_build.rs`) for the 16 port crates; sa-testkit (dev-dependency) for the
  hooks.rs test helpers copied 15× (exe_image_bytes, exe_size, count_hooks, the initial-values check);
  `sa_world::world::common` re-exports sa_core::field's at/rd/wr/g; the other helper copies became imports
  (generic ones in sa_core::field / sa_math::fpu, the rest in the lowest crate every copy may depend on; a copy
  merges only when every free name it uses resolves to the same item, re-exports followed). dupes.py: renamed-copy
  groups across crates 124 / 227 redundant copies → 44 / 68 (exact-body 82 / 142 → 30 / 40; names in ≥ 2 crates
  552 → 515). `--check` + `tools/verify/dupes-allow.txt`: 43 groups listed (25 across the samp-rs workspace, 10
  different things — one offset read for fields of different classes, ports of different functions —, 9 pending
  a hand merge).
- D: sa-game's platform cfgs (app.rs 11, lib.rs 6 conditions) moved into `crates/sa-game/src/platform/` (native_main,
  game_window, screen, web, webfs; mod.rs is the only target switch).
- E: step 4 — sa-viewer, sa-formats, sa-rw and sa-rw-wgpu's world Renderer were used only by the NATIVE-DEMO viewer:
  retired (−28.7k lines); sa-rw-wgpu keeps the D3D9 seam + `dxt`. Step 5 — `tools/lib/exe.py` (exe, catalogue,
  callee facts) used by patterns.py / ai_patterns.py (their `--check` output unchanged), `abi.py`, `layers.py`.
- F: writer gate steps `dupes`, `cfg` (`tools/verify/cfg_check.py` + `cfg-baseline.txt`: 367 conditions in 139 files
  outside the backends, only shrinks), `layers` (`tools/verify/crate_layers.py`, ADR-003 §3.4 order as measured:
  sa-pop below veh/cam/render); `bindings` strict. PLAN.md F14 (code quality). ADR-003 §3.4, §4, §5.1, §5.2.
  README / GETTING-STARTED / Makefile (`make structure`).
- Phase B: a native offscreen run of the tree crashed in RsInitialize (`free(): invalid pointer`) and RwInitialize:
  bindings had taken a Rust target some callers' declarations named where the others had reached the phase B
  registry's implementation (CMemoryMgr::FreeAlign → sa-game's stand-in, paired with its MallocAlign; CRT pow / floor
  / strncpy → the registry's CRT replacements). Fixed (c88a829d), and `bindings.py --check` now fails when a binding
  a host stand-in replaces is not `unported`. Offscreen 600 frames `--fixed-step 33.333`: the same report as master's
  sa-game (616 ok, 0 blocked, 0 crashed) except the registry call counts (fewer calls go through it). The host L2
  specs needed the same: `sa_oracle::hosted::install_crt` registers the CRT stand-ins (pow, floor, strncpy) those
  ports reach. 46 bindings still target a Rust function other than the address's registered port (adapters /
  alternatives the callers' declarations named): pass 3 checks each.
- Gates: host clippy -D warnings, i686 DLL + i686 clippy, wasm32 sa-ports, i686 sa-game (+samp), `FEATURES=samp
  tools/web/build.sh` (wasm-bindgen 0.2.130), make test, sa-hook lib tests, l1test (after B: 18 passed; final: 18 passed, 1 ignored),
  bindings / dupes / cfg / layers / state checks. samp-rs not touched.
- verify-all: A `dedup2-a` IDENTICAL 13/13 (DLL 4df7befa…), B `dedup2-b` IDENTICAL 13/13 (DLL a55c3fed…), final
  `dedup2-final` (8c8b4290, DLL 61cd9877…; the later commits change only phase B targets, unused code and the L1
  harness) IDENTICAL 13/13; `dedup2-final-u` (`SA_UNVERIFIED=all`, lanes 4-5) 12/13 IDENTICAL + weapon-combat
  1792 common frames, the same as master (UNVERIFIED-WC, not worse). Not run: a native window or browser session
  (the D refactor's backends are checked by the offscreen run and the web build only).

## 2026-11-15 — integrate DEDUP-1 pass 2 (main session)

- master fast-forwarded 3b114f6a → 3b0757dd (DEDUP-1 pass 2, 10 commits).
- Checked on master: `make structure` (dupes 43/43 allowed, cfg 367 ≤ baseline, layers 0 violations), bindings
  `--check` (2,633 in 16 crates), sa-hook lib 58/58, i686 sa-game with and without samp, `FEATURES=samp
  tools/web/build.sh`.
- Native offscreen scenarios (`tools/e2e/scenarios/frontier.sh`): pausemenu and solicit 0 blocked; crossing blocked
  on CTrailer::SetupSuspensionLines 0x6CF1A0 and melee on ComputeDamageAnim 0x4B3FC0 / HandlePedHit 0x4E1CC0, the
  known FRONTIER-2 items; no crash.
- Browser (headless Brave, native GPU, samp demo stack): joins the open.mp server, 0 blocked, in world at Grove Street.
- `target/play/sa-game` and `web/pkg` refreshed.

## 2026-11-15 — UNVERIFIED-WC: weapon-combat crash with every unverified batch on (agent, lanes 4-5, 6 for one spot run)

- Reproduced on 016e40db: `SA_UNVERIFIED=all` port-l2 `ports` on weapon-combat, rust run stops at 1792 frames. With
  `SA_CRASH_LOG=1` the fault (read of 0x9FC2) is in the `CAutomobile::BurstTyre` port (0x6A32B0, batch veh3-3) with
  `this` = 0x9FA0, called from 0x6D7E92. No batch bisection was needed: CVehicle::InflictDamage 0x6D7E80 does `mov
  ebp, [esi]; mov ecx, esi; call 0x6D7BC0; add al, 0xD; push eax; call [ebp+0xB0]`, i.e. the virtual BurstTyre call
  takes `this` from the ECX that `CVehicle::FindTyreNearestPoint` (0x6D7BC0, batch veh4-4) returns with. The original
  never writes ECX; the port returned with a stale one.
- Fix: FindTyreNearestPoint declares `preserve = "ecx"` (hazards `preserved-regs`). New L1 spec
  `VehFindTyreNearestPoint` (specs_veh.rs; +0x590 patched into the window, matrix pointer widened on the host side,
  ECX compared) found a second difference: the forward component uses the unrounded `dy` the original keeps on the
  x87 stack after `fst [esp+4]` (the port rounded it first; differs only near f32 overflow). Fixed; L1 native
  1,060,000 cases 0 mismatches. port-l2 `veh4-4-0x6d7bc0-wc` (weapon-combat, plain l2 rust): 3 cases, 0
  mismatches, (b)/(c) IDENTICAL 3500 frames; queue status L2.
- Detection gap closed: sa-hook's callers scan ended a path at any indirect call, so this ECX use was invisible. It
  now reports a virtual call through a register that, on the straight-line code before the hooked call, was loaded
  as the vtable of the object the call passes in ECX (`mov r, [s]` ... `mov ecx, s`), when ECX is untouched between
  the two calls. Over all 12,689 hooks (56,060 call sites) this finds exactly 0x6D7E84; a looser rule (any `call
  [r+d]` after a vtable load on the path) gave 49 COM/RenderWare stdcall false reports. Negative test
  `virtual_call_on_this_after_a_hook_is_flagged`.
- verify-all `wc-fix-u` (`SA_UNVERIFIED=all`, lanes 4-5, DLL 4db157d2…) IDENTICAL 13/13 (save-load: residue only,
  SAVE-1); `wc-fix` (default set, same DLL) IDENTICAL 13/13. Writer gate PASS (hooks, queue, bindings, state, dupes,
  cfg, layers, clippy, i686, i686clip, wasm32, hooktest, l1test 18 passed / 1 ignored, l1).
- Promotion: no batch qualifies yet under HOWTO §4.6 (no batch has L2/L1 evidence for every row; queue: 1,322
  L3-batch, 1,218 written, 4 L2). 201 batches have every row at L3-batch or better (VERIFIER-11's union) and now
  also pass the full union run on all 13 recordings; they need per-hook L2 runs before promotion. Nothing promoted.

## 2026-11-15 — FRONTIER-2: the user-hit blockers, frontier imports / partial ports (agent, branch worktree-agent-acdd0a918985c68f5)
- Base: master 016e40db (DEDUP-1 pass 2). Every new port is ported from the 1.0 US bytes (gta-reversed read as an aid;
  where it differs the bytes are followed and the docs say how), registered `unverified` (batch `frontier2-1`), and
  checked against `tools/gen/inflight.py` (only 0x4AD420 / 0x645600 are in flight elsewhere, neither touched).
- Batch `frontier2-1` (34 hooks + 2 phase-B natives):
  - procedural objects (user-hit walking around): ProcObjectMan_c::ProcessTriangleAdded 0x5A3F20 / ProcessTriangleRemoved
    0x5A3F70, ProcSurfaceInfo_c::AddObjects 0x5A3850 / AddObject 0x5A32D0 / IsPtInTriangle2D 0x5A2F80, the matrix row
    scale 0x5A2E60 (gta-reversed differs three times: the grid restarts y per column, the scale fix-up is for buildings,
    either scale != 1);
  - melee (scenario `melee`, now 0 blocked): CEventDamage::ComputeDamageAnim 0x4B3FC0 (knock-back normalised in 2D),
    CTaskSimpleFight::IsComboSet 0x4ABDC0 / IsHitComboSet 0x4ABDF0 / MakeAbortable 0x6239F0,
    CAEPedAudioEntity::HandlePedHit 0x4E1CC0 (three nested byte jump tables; the surface volume 0x4DA510 was ported
    already), CTaskSimpleBeHit::StartAnim 0x620910, CTaskSimpleFightingControl::ProcessPed 0x62A0A0 / CalcMoveCommand
    0x624B50 / MakeAbortable 0x61DD00, CTaskAllocatorKillThreatsBasic::ComputeClosestPeds 0x69C850 / AllocateTasks
    0x69D170, the event destructor 0x5FF6A0, CPedGeometryAnalyser::ComputePedHitSide(CPhysical) 0x5F3640,
    CTaskComplexHitPedWithCar::HitHurtsPed 0x653AE0 / ComputeEvasiveStepMoveDir 0x653B40 / CreateSubTask 0x6560E0,
    CTaskAllocator::operator new 0x69D980 (`#[native]`: the no-CD chunk at 0x40A08F reads its jmp as a key and a
    keycells redirect would move bytes 0x514B7F reads, sa-hook data_reads);
  - crossing: CTrailer::SetupSuspensionLines 0x6CF1A0 (`|y0 + y1| * 0.5`, all four wheel positions),
    CAutomobile::UpdateMovingCollision 0x6A1460, CVehicle::GetSpecialColModel 0x6DF3D0, the vertex compression
    0x536E90, CTrailer::ProcessAI 0x6CF590 / ScanForTowLink 0x6CF030 / GetTowHitchPos 0x6CEEA0 / ProcessSuspension
    0x6CF6A0 / PreRender 0x6CFAC0, CVehicle::InflictDamage 0x6D7C90 (the frontier's top direct callee);
  - CTaskSimpleGunControl::ProcessPed 0x625270 (a burst draws `RandomInt(0, 2)` twice when the first result is >= 1;
    the task finishes only for the listed use-gun states), the burning car's CAEExplosionAudioEntity::AddAudioEvent
    0x4DCBE0;
  - CCarCtrl::GenerateOneEmergencyServicesCar 0x42B7D0: ported before but unhookable (protection reads its entry), so
    it never reached the phase B registry; now a `#[native]`.
- Win32 imports (FRONTIER-2 (a)): sa-game's import backend implements USER32 `GetWindowLongA` (the value last set with
  `SetWindowLongA`, which now stores and returns the previous value), `GetMenu` (none) and `AdjustWindowRectEx`.
  `tools/verify/frontier.py` lists the imports `orig!` declarations reach (`cell` / `site` forms, named from the exe's
  import directory) that neither the `natives!` table nor a path-bound host stand-in implements (`--imports`): 4 left
  (DeleteFileA, GetLocalTime, GetVolumeInformationA, DialogBoxParamA).
- Partial ports (FRONTIER-2 (d)): `frontier.py --partial` lists the 14 `#[hook(fallback)]` ports (17 when the row was
  written; the script switches now show their unported opcode counts, 1200-1299: 28, 2500-2599: 5).
- ADR-003: no new copies (`kf` → `sa_core::field::gf`; the CPlaceable heading helper moved to
  `sa_world::world::common::placeable_heading` for sa-ped and sa-veh; sa-veh's magnitude reused); new bindings for
  every address now declared by two modules (0x4ABDA0, 0x4ABDC0, 0x61C5E0, 0x5F1E70, 0x5F79C0, 0x61DE60, 0x6924B0,
  0x61E040, 0x61E200, 0x61E0C0, 0x6A65D0, 0x6A4EC0, 0x6AFB10, 0x4594C0, 0x61C2E0, 0x6D7BC0).
- Repros (`tools/e2e/scenarios/frontier.sh`, offscreen, --fixed-step 40): melee 2616 ok / 0 blocked (was 1688 blocked
  from frame 912); pausemenu 0 blocked; solicit 0 blocked; crossing: the trailers, the moving collision and the damage
  now run; it stops at frame ~855 on `update_car_ai_original (0x0)` — the damage the car does reports a crime, the
  police come and CCarAI::UpdateCarAI's law-enforcer path is still the original's (see the ROADMAP).
- Batch `frontier2-2` (8 hooks + 4 natives): the CPostEffects switches / light passes / day-night balance / screen-mode
  init (0x701120, 0x701140, 0x701160, 0x7012E0, 0x701410, 0x701430, 0x7034D0, 0x7046D0; 0x7034B0 native: the no-CD
  chunk at 0x4084D2 reads its jmp) and the police choices CCarAI::FindPoliceCarMissionForWantedLevel 0x41C9D0 /
  FindPoliceBikeMissionForWantedLevel 0x41CA40 / FindPoliceCarSpeedForWantedLevel 0x41CAA0 (natives: protection
  code reads or calls into their entries, and 0x41CA40 is 3 bytes long).
- verify-all `frontier2-1` (lanes 1-3, SA_UNVERIFIED=frontier2-1): 12/13 IDENTICAL; the grove walk diverged at frame
  4888 (randcalls). `tools/verify/bisect-batch.sh frontier2-1 grove-walk 6`: single culprit
  ProcSurfaceInfo_c::AddObjects 0x5A3850, the rest of the batch identical. Cause: its two `srand` calls carried a Rust
  return address in the determinism rand log; they now run `at_site` the original's return addresses (0x5A3C82,
  0x5A3E00), as CGame::InitialiseRenderWare does.
- verify-all `frontier2-2` (3719c807, lanes 1-3, SA_UNVERIFIED=frontier2-1,frontier2-2): IDENTICAL 13/13 (grove walk
  included). Writer gate green on both commits (frontier2-1 after the expect / native / L1-route fixes the hooktest and
  l1 steps found; frontier2-2 full). i686 sa-game with and without `samp`, `FEATURES=samp tools/web/build.sh` (with the
  wasm-bindgen 0.2.130 of ~/sa-engine/target/tools/bin) built. Final repros: pausemenu 416 ok / 0 blocked, solicit
  4016 / 0, melee 2616 / 0, crossing 871 ok / 345 blocked on `update_car_ai_original (0x0)` (police), no crash.
- frontier.py before / after: direct 511 + virtual 220 = 731 → direct 480 + virtual 211 (+ imports 4, partial 14 newly
  listed).
- Left open: CCarAI::UpdateCarAI's remaining cases (the law-enforcer path with a wanted level and the missions the
  partial port does not cover; ~2,500 instructions of the 3,361); CTaskComplexKillCriminal::ControlSubTask 0x68E950
  (1,179 instructions); CCam::Process_M16_1stPerson 0x5105C0 (1,446); the other partial ports' cases; the four
  imports; the rest of the frontier list (the ~250-function turn was not reached).


## 2026-11-15 — integrate UNVERIFIED-WC and FRONTIER-2 part 1 (main session)

- UNVERIFIED-WC (4c402a9a) fast-forwarded; sa-hook lib 59/59, i686 sa-game.
- FRONTIER-2 part 1 rebased onto 4c402a9a (WORKLOG conflict: both entries kept) and fast-forwarded. On the rebased
  tree: sa-hook lib 59/59, `make structure` clean, bindings `--check` (2,649), i686 sa-game; scenarios pausemenu /
  solicit / melee 0 blocked, crossing 345 blocked on `update_car_ai_original (0x0)` (police path, open), no crash;
  verify-all `f2-int` (lanes 1-3) IDENTICAL 13/13. `target/play/sa-game` and `web/pkg` refreshed.


## 2026-11-15 — FRONTIER-2 part 2: police path, first-person aim, user-hit IK point arm / wheel particles, imports (agent, branch worktree-agent-acb224bd5911fd3a7)
- Base: master 8be51029. Every port from the 1.0 US bytes (gta-reversed read as an aid; it reverses neither UpdateCarAI's
  whole switch faithfully nor ControlSubTask / Process_M16_1stPerson), checked with `tools/gen/inflight.py` first.
- Batch `frontier2-3` (police path; scenario `crossing` was 345 blocked on `update_car_ai_original`):
  - `CCarAI::UpdateCarAI` 0x41DA30 is now a whole port (new module sa-pop update_car_ai.rs; the ADR-002 fallback is gone):
    the mission switch of status 2/3/12 (jump tables 0x420694 / 0x420708: ram / block the player far / close, goto
    coordinates and straight line (plain / accurate), target far / close (ram / block / follow car, kill ped, drive-by),
    slowly drive to the player, block forward and back, boat attack / circle, approach the player (with the cop bike's
    drive-by sequence), escorts, the law-enforcer default), the pre-switch mission 0x3B and police cruise speed, the
    `PoliceAbandonCars` give-up, and the tail (stuck test, bridge physics, bike → block, chase steering 0x41FE46 with the
    steer angle, the player's land / water vehicle test, gang-area cruise speed). Bytes over gta-reversed: switch distances
    are 2D lengths vs the straight-line byte / `FindSwitchDistanceFar`, mission 0x3B also sets mission 0x16, the cop-bike
    exit time is 20 s, mission 18 switches to 17 when far.
  - `CCarCtrl::SteerAICarWithPhysics_OnlyMission` 0x436A90 whole (was partial): ram the player (mission 3, the side aim,
    passenger `Say(0x26)`), approach close (0x2C), follow car close (0x35), block (5 / 6 / 0x12), escorts 0x1D..0x20,
    circle target (0x3C), the heli / plane / boat entries. The hook was verified before; it is now `unverified =
    "frontier2-3"` (the default set runs the original body until it is promoted).
  - `CCarAI::TellOccupantsToLeaveCar` 0x41C760 (passengers draw an unused `rand()` at 0x1569B68; the driver's leave-car
    task size / bool come through no-CD code: 0x34 decoded, the bool taken as the passengers' 1),
    `AddAmbulanceOccupants` 0x41C4A0 / `AddFiretruckOccupants` 0x41C600 (the last `SetTask` bool comes from the no-CD
    decoder 0xCB73E0, taken as 0).
  - `CTaskComplexKillCriminal::ControlSubTask` 0x68E950 (1,179 instructions, unreversed in gta-reversed),
    `ChangeTarget` 0x68C6E0, `FindNextCriminalToKill` 0x68C3C0 (its reference distance is never updated: the last
    nearer entry wins, original quirk) in sa-ai killtasks.rs.
  - Cleanups (ADR-003): carctrl2's unused copies of 0x41CAA0 / 0x41C9D0 removed (police_ai.rs has the natives); new
    bindings 0x41C760, 0x41CAA0, 0x41C9D0, 0x41CA50, 0x56E120 (FindPlayerEntity), 0x72DD60 (PoliceAbandonCars), 0x6A2450.
- Batch `frontier2-4`: `CCam::Process_M16_1stPerson` 0x5105C0 (new sa-cam m16.rs, Fpu machine, frame at the original's
  B offsets: zoom by wheel / pad, drive-by turn, mouse / pad look with `pow` smoothing, shake, drive-by source and beta /
  alpha limits by seat, on-foot source, line-of-sight near clip, camera-mode near clip); `CCheat::WeaponCheat1/2/3`
  0x4385B0 / 0x438890 / 0x438B30 (reached by the new scenario's KJKSZPJ).
- Batch `frontier2-5` (user-hit, from the coordinator): `CTaskSimpleIKPointArm::CreateIKChain` 0x6342F0 and
  `UpdatePointArmInfo` 0x634370 (sa-ai ik_tasks.rs / bike_door_swim_samp.rs, next to their LookAt twins);
  `Fx_c::AddWheelGravel` 0x4A0170 / `AddWheelMud` 0x4A03C0 (byte-identical to AddWheelGrass except the colour: one shared
  body `wheel_bits`, rand sites offset 0x250 / 0x4A0) and `AddWheelSand` 0x4A0610 / `AddWheelDust` 0x4A09C0 (one shared
  `wheel_cloud`, they differ in colour and the life formula). No deterministic scenario reaches them yet: `solicit` calls
  `IKChainManager_c::PointArm` twice (CanAccept refuses), and the scenarios' tyres touch grass only.
- Imports (FRONTIER-2 (a)): sa-game implements `DeleteFileA` (VFS remove), `GetLocalTime` (host UTC, as `_strdate`),
  `GetVolumeInformationA` (FALSE; only reached for CD drives, which `GetDriveTypeA` never reports) and `DialogBoxParamA`
  (IDOK, selection unchanged). frontier.py --imports: 0.
- Scenario `sniper` in tools/e2e/scenarios/frontier.sh (weapon cheat, six `e`, right mouse button held: Process_M16_1stPerson
  runs 750 times); `SA_GAME_RMB` added to sa-game's evidence input (next to `SA_GAME_LMB`);
  tools/e2e/scenarios/frontier2-3-counts.gdb counts this part's ports.
- Repros (native, --fixed-step 40): crossing 1216 / 0 blocked (was 871 / 345); melee 2616 / 0; solicit 4016 / 0; pausemenu
  416 / 0; sniper 1216 / 0; no crash. A 4000-frame crossing run reaches further unported code (firetruck tasks:
  `vslot_18` MakeAbortable, 0x729B60, CreateFirstSubTask 0x62BF00) and a pistol shot at a car `SetUpWheelColModel`
  0x6A3060: left for the frontier turn.
- Gates: writer gate green (8be51029..), `make structure` clean, bindings `--check` 2,656; verify-all `frontier2-p2b`
  (lanes 1-3, SA_UNVERIFIED=frontier2-1..frontier2-5) IDENTICAL 13/13; i686 sa-game with and without `samp`, `FEATURES=samp
  tools/web/build.sh` (wasm-bindgen 0.2.130) built.
- frontier.py before / after: direct 480 + virtual 211 + imports 4 + partial 14 → direct 476 + virtual 210 + imports 0 +
  partial 12.
- Left open: the 12 remaining partial ports. Only unreachable / undefined cases fall back in CHud::DrawVehicleName (state
  outside 0..4), CWaterLevel AddWaterLevelQuad / RenderFlat*_OneLayer (stack-garbage cases), GenerateRoadBlockPedsForCar
  (offset indices beyond the tables), CPopulation::AddPed (types < 4, 21, > 31 read a stale argument slot) and
  CCarGenerator::DoInternalProcessing (hotdog vans: AddPed's makeWander is the caller's EDI); reachable ones still to
  port: CGangWars::Update (active wars), CRoadBlocks::GenerateRoadBlocks (wanted-level road blocks), CCollision::
  ProcessColModels (A's disks), the script switches 1200-1299 (28 opcodes) / 2500-2599 (5). The ~250-function frontier
  turn was not reached.

## 2026-11-15 — integrate FRONTIER-2 part 2 (main session)

- master fast-forwarded 8be51029 → 8e9b827e (police path, Process_M16_1stPerson, weapon cheats, IK point arm, Fx_c
  wheel particles, the four Win32 imports; the agent's verify-all `frontier2-p2b` IDENTICAL 13/13 on the same base).
- Checked on master: sa-hook lib 59/59, `make structure` clean, i686 sa-game, `FEATURES=samp tools/web/build.sh`;
  scenarios pausemenu / solicit / crossing / melee / sniper all 0 blocked, no crash. `target/play/sa-game` and
  `web/pkg` refreshed.

## 2026-11-15 — GIT-1: prepare the history rewrite (agent, not run on the real repository)

- `docs/history/commit-messages.tsv`: a Conventional Commits subject for each of master's 755 non-merge commits (up to
  fd955e95), written by hand in batches from the original message and diff stat; `Refs` only where the original names
  the task (307 rows). Cherry-picked duplicates share their subject.
- `tools/git/rewrite_history.py`: rebuilds each commit object from its raw bytes (new subject + original message verbatim
  + `Refs:` + `Original-commit:`; other headers byte-identical, parents mapped), verifies every pair before touching a
  ref, archives refs under `refs/archive/pre-rewrite/` plus a bundle, moves master and the fully merged branches, writes
  `docs/history/hash-map.tsv`, rewrites the cited commit ids and commits that. `--check`, `--draft-missing`,
  `--dry-run DIR`; refuses on dirty tracked files or registered agent worktrees unless `--force`.
  `tools/git/rebase_after_rewrite.py` moves unmerged branches (`git rebase --onto`, in a temporary worktree).
  `commit_lint.py --all`; the body width rule does not apply to messages with `Original-commit:`.
- Dry run on a fresh clone (master = this branch, 182 branches recreated): 766 commits, tree / author / committer /
  dates identical and parents mapped 766/766, original messages kept 766/766, `commit_lint.py --range master --all`
  0 bad of 759, 3,280 cited ids updated in 29 files, 0 ambiguous, 6 decimal-only ids (all real citations), 34 refs
  moved, two unmerged branches rebased with identical trees. Clone removed. Procedure: docs/history/README.md.
## 2026-11-15 — PORT-EAX and VERIFY-PROMOTE-1 (agent, lanes 4-5, 6-7 for spot runs)

- PORT-EAX (commit 5eee5336): from the bytes, the EAX on every path. `CEntity::RegisterReference` 0x571B70: the
  node it found or linked, 0 when pruning frees nothing, the caller's EAX with AL = [this+0x2E] (0) for a plain
  building (`entry_eax`, read before any call). `CStreaming::RequestModel` 0x4087E0: the load state over the
  caller's EAX on the exits that only read it, `prev * 5` or the last `GetModelType` EAX (all eight are `mov al, N;
  ret`: vtable pointer's upper bytes) on the loaded path, `AddToList`'s EAX on the list tails.
  `CStreamingInfo::AddToList` 0x407480 leaves `movsx eax, word [this]` and now returns it (L1 spec compares EAX:
  native 1,002,000 cases 0 mismatches). `bindings.py --twin`: twins in sa-stream / sa-world, 149 callers moved.
  port-l2 `eax-pr` (pause-restart) and `eax-drv` (driving), plain l2 l2all rust: 1000 cases each, 0 mismatches,
  0 undeclared (AddToList with the streaming-info array declared), (b)/(c) IDENTICAL. sa-hook lib 59 passed.
- VERIFY-PROMOTE-1: `tools/verify/batch-l2.py` (plan / run / report / promote / record / setkind / forget / reparse;
  HOWTO §5). In-process L2 lists of 30-150 hooks either crashed at boot (a double-run creator) or timed out at
  1500 s (the static window is snapshotted per case), so every L2 row was moved to split runs (SPLIT_PER_FRAME 20,
  SPLIT_STACKFILL 4096); L3-only rows by `l3.sh --rust LIST --fixed verified --hookoff all` on pause-restart (61
  hooks: L3 MATCH, every hook-off MATCH; 8 never called in the INF-10 coverage). Swept: pause-restart, grove-walk,
  driving (old split rows), car-ai, weapon-combat, police-gang, save-load, menus-map-controls, menus-hud, c-bug,
  air-water; world-cheats and anim-input partly; stopped on the coordinator's request.
- Verdicts over 1,324 hooks: 234 L2 (split PASS / PASS-FP, 0 mismatches, 0 undeclared), 51 L3, 8 without calls, 3
  mismatches, 1,028 open (831 not run on every recording yet, 167 with undeclared writes, 30 split pairing
  artefacts). 47 hooks whose only finding was static-memory undeclared writes got the .data/.bss D range in
  l2-extras (not re-run yet). Queue statuses recorded for all hooks with evidence.
- Port fixes (37c559e5): pluginEnvMatStreamReadCB 0x5D9660 returned the material (the original returns the
  stream: split EAX mismatch on 120 cases; after the fix 200 cases 0 mismatches), InteriorManager_c ctor 0x5984C0
  missed the stores +0x4740 / +0x474C = 1e6 and +0x4748 / +0x4744 = -1e6 (after the fix 0 mismatches),
  CanPedTargetPoint / CanPedTargetPed 0x5F1B70 / 0x5F1C40 now return AL (callers test AL only; the full-EAX
  compare showed upper-byte residue the u8 line-of-sight binding cannot carry; split PASS 200 cases each).
  Mismatches left open: CCamera::Process 0x52B730, FrameUpdateCallBackSkinnedWithVelocityExtraction 0x4D1680,
  CShopping::LoadStats 0x49B6A0 (calls FindSection / GetNextSection directly instead of through their hooks).
- Promoted (d2f5e811): 30 batches, 48 hooks (28 L2, 20 L3) — see the ROADMAP row VERIFY-PROMOTE-1.
- Checks: writer gate (base 4c402a9a) PASS on hooks, bindings, state, dupes, cfg, layers, clippy, i686, i686clip,
  wasm32, hooktest, l1test, l1; queue FAILed once because `batch-l2.py record` was rewriting the status file
  while the gate ran, `queue.py --check` afterwards: up to date. verify-all `bl2-promote` (default set, DLL
  33b96aa6…) IDENTICAL 13/13 (save-load: residue only, SAVE-1).
- verify-all `bl2-promote-u` (`SA_UNVERIFIED=all`, same DLL) IDENTICAL 13/13 (save-load: residue only, SAVE-1).
  The commit-msg hook of the shared config runs tools/verify/commit_lint.py from the worktree, which this branch
  (based on 4c402a9a) lacks: master's copy was placed there untracked so the check ran on these commits.

## 2026-11-15 — FRONTIER-2 part 3: user-hit IK point arm / plane spawning, car tyres, the fire-truck and armed-cop chain (agent, branch worktree-agent-a1fa130bdd6dc9931)
- Base: master 3a76ad1c (fast-forwarded to 26eec393 before the first commit, for the commit-msg hook). Scope cut by the user mid-run to the user-hit blockers: the partial-port cases, the ~250-function
  frontier turn and the 0xCB73E0 boolean fixes stay open. Every port from the 1.0 US bytes (gta-reversed read as an aid),
  checked with `tools/gen/inflight.py` first; all in batch `frontier2-6` (unverified).
- User-hit (free roam): `CTaskSimpleIKPointArm::~CTaskSimpleIKPointArm` 0x634240 and `::Clone` 0x634250 (with them every
  slot of vtable 0x86E3A4 is ported) and the twin `CTaskSimpleIKLookAt::Clone` 0x633F00 (the point-arm clone copies
  +0x44..+0x50 but not the part word +0x16, the look-at clone both); `CPlane::FindPlaneCreationCoors` 0x6CD090 (the
  try loop compares the root distance, FCOS / FSIN of the angle, the orientation from the unrounded x87 sums; the box
  argument of CheckCameraCollisionBuildings is an uninitialised local the callee never reads; a new binding 0x41A820).
- Car tyres: `CAutomobile::SetUpWheelColModel` 0x6A3060 (tractor / combine / kart none; sphere copied before the box;
  the middle wheels only when both frames exist) and `CBike::SetUpWheelColModel` 0x6B67E0 (wheel centres from the
  modelling matrices combined up to the frame at +0x5A4). New scenario `shootcar` in tools/e2e/scenarios/frontier.sh:
  the crossing Greenwood left on the road, CJ 7 m south of it, KJKSZPJ, two `e`, right button (camera mode 0x35: the
  shot line includes car tyres) and left button: SetUpWheelColModel runs 100 times, the bike's 15, 0 blocked.
- Crossing 4000 (the fire-truck crew and the armed police): `CAutomobile::FireTruckControl` 0x729B60 (player camera /
  pad aim and the AI's aim at the fire, the cannon's start and direction; FPATAN / FSIN / FCOS),
  `CTaskComplexKillPedOnFootArmed` `CreateFirstSubTask` 0x62BF00 (the bytes compare the root distance with 30 / 6,
  gta-reversed the squares), `CreateSubTask` 0x626FC0 (unreversed in gta-reversed; the gun-control command is a byte
  written over the ped argument's slot, so its upper bytes are the ped pointer's), `ControlSubTask` 0x62CCE0,
  `LineOfSightClearForAttack` 0x621500 (the ducking / standing eye positions are the other way round in gta-reversed)
  and `IsPedInLeaderFiringLine` 0x621300 (new sa-ai kill_ped_on_foot_armed.rs, new sa-veh fire_truck.rs). Blocked
  steps 2298 → 2163, the first block moved from frame 1702 to 1837; still blocking: `CreateNextSubTask` 0x62C190
  (~700 instructions with the cover-point logic), `CWaterCannon::PushPeds` 0x7295E0, `CPedIK::PointGunAtPosition`
  0x5FDE20, and behind them `CCover::FindAndReserveCoverPoint` 0x6992B0 / `FindCoordinatesCoverPoint` 0x699570.
- The 0xCB73E0 booleans (item 4, decoded before the scope cut; evaluated on the exe image with unicorn from each call
  site, descriptor + return address): TellOccupantsToLeaveCar's driver leave-car flag = 1 (descriptor 0x155FFC8, as
  assumed), AddFiretruckOccupants' last SetTask flag = 0 (0x155FFA4, as assumed); AddAmbulanceOccupants' evaluates to
  0x947B2A9D + 0x3C7D482 = 0x9842FF1F (0x155FF44), but `CTaskManager::SetTask` 0x681AF0 never reads its third
  argument, so the ports' 0 has no effect. No code change.
- ADR-003: new bindings 0x41A820 (CheckCameraCollisionBuildings), 0x601230 (SetTaskDuckSecondary), 0x61F8B0
  (CTaskSimpleThrowControl ctor; cmd1600.rs and player_on_foot.rs now import them).
- frontier.py before / after: direct 471 + virtual 209 + imports 0 + partial 12 = 692 → direct 470 + virtual 203 +
  imports 0 + partial 12 = 685.
- Repros (native, --fixed-step 40, default lengths): pausemenu 416 / 0 blocked, solicit 4016 / 0, crossing 1216 / 0,
  melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0; no crash. crossing 4000: 1853 ok / 2163 blocked (see above).
- verify-all `frontier2-p3` (lanes 1-3, SA_UNVERIFIED=frontier2-1..frontier2-6, DLL of 0a9eaa21; the following commit
  only swaps local helpers for the shared ones and fixes clippy findings): IDENTICAL 13/13.
- Commit messages follow Conventional Commits (the branch was fast-forwarded to 26eec393 so the commit-msg hook finds
  tools/verify/commit_lint.py).

## 2026-11-15 — integrate PORT-EAX + promotions and FRONTIER-2 part 3 (main session)

- Promotions branch (PORT-EAX, three port fixes, `tools/verify/batch-l2.py`, 30 batches / 48 hooks promoted) rebased
  onto 26eec393 (conflicts: the CanPedTargetPed binding row takes the agent's `u8` return and keeps master's 0x5F1E70
  row; the verification queue regenerated; ROADMAP / WORKLOG rows merged), FRONTIER-2 part 3 stacked on top, queue
  regenerated once more (6209bcc2). Master fast-forwarded.
- Checked: sa-hook lib 59/59, `make structure` clean, bindings `--check` (2,659), state_check 0 inconsistent, i686
  sa-game; scenarios pausemenu / solicit / crossing / melee / sniper / shootcar 0 blocked, no crash; verify-all
  `int3-default` (lanes 1-3) IDENTICAL 13/13; `int3-f2b` (`SA_UNVERIFIED=frontier2-1..6`) IDENTICAL 13/13 (a first
  run `int3-f2` failed on every recording only because target/release/sa-oracle was stale and lacked the new specs;
  rebuilt). `target/play/sa-game` and `web/pkg` refreshed.

## 2026-11-15 — FRONTIER-2 part 4: user-hit event destructors, the crossing-4000 chain, partial-port cases, AI frontier (agent, branch worktree-agent-accfa614f52d5f0c1)
- Base: master 8c9cbf77. Every port from the 1.0 US bytes (gta-reversed read as an aid), `tools/gen/inflight.py` checked
  first. Mid-run scope change from the coordinator: two more agents took the frontier by subsystem; item 4 here is
  limited to the AI subsystem, Wine lanes 1-3.
- Batch `frontier2-7` (user-hit: a shot at a driven car in free roam stopped on 0x61C310):
  CEventVehicleDamageWeapon::Destructor 0x61C310 (vtbl 0x86D640, tail into CEventVehicleDamage's 0x4B1990) and every
  other CEvent* complete destructor that ported code reached unported (0x4B0DA0, 0x4AECA0, 0x4AF0A0, 0x4AFBF0, 0x4AEA50,
  0x5FEF70, 0x5FF2C0, 0x5FF520, 0x5FF5E0, 0x5FF760, 0x5F63F0, 0x674ED0, 0x4B1320, 0x4B1B50; natives 0x5FF860 (bare
  jmp), 0x609140 and 0x5FF460 (the no-CD protection reads their entry bytes: 0x1569A80 / 0x401673, sa-hook keycells
  test); the protection-fragment entries inlined), the scalar deleting destructors with the inlined event-pool delete
  0x4B5670 / 0x4B6080 / 0x4B65C0 / 0x4B6750 / 0x4B6940 (new sa-ai event_dtors.rs). Unnamed bodies named in
  catalog/name-corrections.csv (+ procedures.csv / state.csv names); the classes of the unnamed vtables come from their
  GetEventType slot (eEventType): CEventRevive, CEventSexyVehicle, CEventVehicleThreat, CEventHealthReallyLow,
  CEventLowAngerAtPlayer, CEventHealthLow, CEventInteriorUseInfo. CStats::UpdateStatsWhenWeaponHit 0x55CEB0 (the hit
  stops on it before the event exists).
- Scenario `shootdriver` (frontier.sh): SA_GAME_JACK puts CJ 7 m in front of the nearest driven car, KJKSZPJ, the
  pistol, the aim pitched down (SA_GAME_MOUSE), RMB + LMB: InflictDamage gives the driver the event, 0x61C390 -> 0x61C310
  runs (5 calls under gdb, tools/e2e/scenarios/frontier2-p4-counts.gdb), 0 blocked. `shootcar` never creates the event:
  its Greenwood is empty and its shots miss (the default aim passes over the car).
- Batch `frontier2-8` (crossing 4000): CTaskComplexKillPedOnFootArmed::CreateNextSubTask 0x62C190 (unreversed in
  gta-reversed; 9 rand sites), CPedIK::PointGunAtPosition 0x5FDE20 (in sa-ai: it needs sa-world's GetWeaponSkill,
  above sa-anim), CTaskComplexKillCriminal::MakeAbortable 0x68DAD0, CCover FindAndReserveCoverPoint 0x6992B0 (the bytes
  take the nearest point and return it even when both slots hold other peds; type 2 accepts a point whose second slot is
  taken; gta-reversed takes the farthest) / FindCoordinatesCoverPoint 0x699570 / DoesCoverPointStillProvideCover
  0x698DD0, CPointList::Empty 0x699F00, CFormation::FindCoverPointsBehindBox 0x699FF0 (the bytes measure the corners'
  distance to the target and step towards the corner opposite the nearest; gta-reversed measures the sides),
  CWaterCannon PushPeds 0x7295E0 (one section per ped) / Render 0x728DA0, CTaskComplexWalkRoundCar::
  ComputeRouteRoundBigCar 0x656BB0 (unreversed), ComputeClosestSurfacePoint-pos 0x5F36F0, CTaskSimpleHurtPedWithCar::
  ProcessPed 0x653F90, CTaskComplexFleeAnyMeans::CreateSubTask 0x65D2E0.
- Batch `frontier2-9`: the flee chain (CTaskComplexFleeAnyMeans ControlSubTask 0x65D780 / CreateNextSubTask 0x65D540,
  CTaskComplexFleeShooting::ControlSubTask 0x65CA90) and small AI frontier entries (0x63C320, 0x63D220, 0x6492E0 sharing
  one urgent-only MakeAbortable body with 0x6486F0; 0x65B420, 0x632B80, 0x62F520, 0x639900, 0x640420, 0x675460, 0x661720,
  0x632050, 0x68AA50, 0x654720, 0x600880, 0x5F13A0, 0x67C2E0, 0x67C360, 0x62FA60; new ai_frontier_p4.rs).
- Partial ports (item 3): ProcessCommands1200To1299's 27 default-reaching opcodes are explicit `-1` cases (only 0503
  CREATE_SWAT_ROPE stays on the fallback: it passes an uninitialised stack vector to ClearSpaceForMissionEntity, not
  reproducible); ProcessCommands2500To2599's 5 are by design (09C6 / 09D3 SA:MP-patched table entries, 09CD / 09CE /
  09DF default); CGangWars::Update, CRoadBlocks::GenerateRoadBlocks and CCollision::ProcessColModels' disk cases not
  reached (time went to the crossing chain).
- Bindings (ADR-003): rows 0x4B1F80, 0x5FDC00, 0x5FDE20, 0x601390, 0x5F3640, 0x4B1320, 0x668C70, 0x678CD0; 0x4B2470
  moved to sa-world (sa-ai and sa-ped call it); local declarations replaced; `make structure` clean (two new copies the
  first gate found were replaced by entity_pos / rd3u).
- Repros (native, --fixed-step 40): crossing 4000: 1853 ok / 2163 blocked -> 4014 ok / 2 blocked (first block frame 3903,
  CTaskComplexKillCriminal::CreateNextSubTask 0x68E4F0, 391 instructions, open); pausemenu 416 / 0, solicit 4016 / 0,
  crossing 1216 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0; no crash.
- Gates: writer gate (base 8c9cbf77) on a06f78e1: hooks / bindings / state / dupes / cfg / layers / commits / clippy / i686 /
  i686clip / wasm32 / hooktest / l1 PASS; queue then failed only because the queue records the writer commit (fixed by
  fdb07b39, `queue.py --check` up to date at HEAD); l1test failed on `preserved_registers_match_the_hook_registrations`
  (three preserve-ecx destructors without specs; queued in PRESERVE_SPECS_PENDING by 4d552c24, that test passes; the other
  17 L1 tests passed in the gate run). First gate run (0e036130 before amending) had found two new copies (cover.rs
  pos_of, walkround.rs vec3), a wrong `expect` (0x5FDE20: two nops) and the two protection-read entries now natives.
- verify-all (lanes 1-3, sa-oracle rebuilt first): `f2p4-u` (SA_UNVERIFIED=frontier2-1..frontier2-8, DLL of 0e036130
  before the native conversion) IDENTICAL 13/13; `f2p4-u2` (frontier2-1..frontier2-9, DLL ba5179…, the final code)
  IDENTICAL 13/13; `f2p4-def` (default set, same DLL) IDENTICAL 13/13.
- Builds: i686 sa-game with and without `samp`, `FEATURES=samp tools/web/build.sh` (wasm-bindgen of
  ~/sa-engine/target/tools/bin) built.
- frontier.py before / after: direct 470 + virtual 203 + imports 0 + partial 12 = 685 → direct 291 (+145 registered on the
  other two agents' branches) + virtual 172 (+11) + imports 0 + partial 12 = 475; AI entries 164 → 143.
- Left open: CTaskComplexKillCriminal::CreateNextSubTask 0x68E4F0 (crossing 4000's last block); 0503 CREATE_SWAT_ROPE;
  CGangWars::Update, CRoadBlocks::GenerateRoadBlocks, CCollision::ProcessColModels' disk cases; L1 specs for
  0x4AECA0 / 0x4B1320 / 0x4B1B50 (preserve ecx); the rest of the AI frontier (the ~250-function turn was not reached:
  56 hooks + 3 natives this part).
## 2026-11-15 — FRONTIER-2 lane B: VEH / WORLD / RENDER / RW / CAM / … frontier turn (agent, branch worktree-agent-a64af3d60c6e9085b)
- Base: master 8c9cbf77. Lane B of the three-way split (subsystems VEH, WORLD, RENDER, RW, CAM, WEAPON, POP, PED, ANIM,
  STREAM, CORE, MATH, SAVE, PHYS). Every port from the 1.0 US bytes (gta-reversed as an aid only), every address checked
  against `inflight.py`'s `elsewhere()` before porting and again before hand-back (0 overlaps with master / other
  branches). 67 new unverified hooks in four batches, plus 3 phase B natives:
  - `frontier2-b1` (RENDER + MATH + one STREAM row): the `Fx_c` / `FxManager_c` shutdown and particle-recycling paths
    (0x49E630, 0x49E850, 0x4A12D0, 0x4A90A0, 0x4A9400, 0x4A9A10, 0x4A9C80, 0x4AA1B0, 0x4A2510, 0x4AAC70), the `Fx_c`
    helpers (CreateMatFromVec 0x49E950 on the literal x87 stack, AddTyreBurst 0x49F300, AddWood 0x49EE10 with its rand
    sites, TriggerBullet/FootSplash 0x4A10E0 / 0x4A1150), `FxManager_c::CreateFxSystem` 0x4A95C0 / 0x4A9BB0,
    `FxEmitter_c::AddParticle` (matrix overload) 0x4A4050; `CPostEffects` 0x700BE0, 0x701170, 0x702F00, 0x7011C0,
    0x701320, 0x702F40; `CSprite2d::Draw` 0x7282C0, `CShadowCamera::Update(RpAtomic*)` 0x705C80,
    `CRopes::IsCarriedByRope` 0x555F80, `CShadows::RemoveOilInArea` 0x7074F0, `CCheckpoints::UpdatePos` 0x722900 /
    `SetHeading` 0x722970, `RpClumpGtaCancelStream` 0x72E700, `CGeneral::GetAngleBetweenPoints` 0x53CEA0 (L1-pure,
    the L1 run 1,060,000 cases on the native backend passes; registered after every older sa-core hook so sa-hook's
    table order test holds).
  - `frontier2-b2` (RW, new sa-rwcore `frontier_b.rs`): sector teardown / lookup (0x74ECA0, 0x74ED50, 0x74F4E0, 0x751660,
    0x751300, 0x74EC00), geometry / atomic (0x74CE60, 0x74D6D0, 0x74BF40), UV anim (0x7CC530, 0x7CC4B0, 0x7C8D40,
    0x7CC760 with FPATAN), strings / lists / resources (0x80A470, 0x80A4A0, 0x809240, 0x801E90, 0x808320), files /
    images / MatFX (0x80E9B0, 0x800810 with the jump table decoded, 0x803790, 0x816290), `_rwMatrixIdentityError`
    0x7F1590 (ST0); native 0x7C8830 (3-byte `xor eax, eax; ret`).
  - `frontier2-b3` (VEH): `CQuadBike::CQuadBike` 0x6CE370, `CAutomobile::SetupDamageAfterLoad` 0x6B3E90, `BreakTowLink`
    0x6A4400, `GetTowBarPos` 0x6AF250, `CTrain::GetDooorAngleOpenRatioU32` 0x6F5930, `CHeli::TestSniperCollision`
    0x6C6890, `CBike::BlowUpCar` 0x6BEA10.
  - `frontier2-b4` (RW + SAVE): the sector tie destroys 0x74FFA0 / 0x74FFF0 (revealed by b2), `RtDictDestroy` 0x7CF130,
    `CObjectSaveStructure::Construct` 0x5D2160 / `Extract` 0x5D2220 (new sa-core `object_save.rs`).
  - Phase B natives: `CCarAI::TellCarToFollowOtherCar` 0x41C960 (unhookable: the protection writes its entry; the
    existing port now registered), `CPlayerInfo::CancelPlayerEnteringCars` 0x56E860 (empty `ret 4`).
- Hazards recorded on the hooks: stack-residue where the original keeps temporaries in its own argument slots (AddWood,
  RwFreeListForAllUsed, the same-depth image copy, TestSniperCollision); TestSniperCollision compares
  `CCollision::DistToLine`'s result through the f64 binding (exact for f32-derived values, noted).
- ADR-003: no new bindings rows; repeated declarations avoided by making the existing ones `pub` and importing them
  (0x749C00 clumpstream, 0x74ECA0 / 0x74ED50 worldcreate now targeting the ports, 0x809440 anim, 0x6D0B10 mvpd_v2);
  `make structure` clean.
- Writer gate (base 8c9cbf77): all steps PASS (hooks 67, queue, bindings, state, dupes, cfg, layers, commits, clippy,
  i686, i686clip, wasm32, hooktest 59/59, l1test, l1 0x53CEA0).
- frontier.py (lane B subsystems, to port, excluding addresses other branches register): before 263 (direct 228,
  virtual 27, partial 8) → after 197 (direct 169, virtual 20, partial 8); the partial ports are lane A's. Newly revealed
  in lane B: 0x59B9F0 `DecompressIntoFullMatrix` (sa-math has the L1-verified model; not hooked yet).
- Scenarios (native i686 sa-game, --fixed-step 40, default lengths): pausemenu 416 / 0 blocked, solicit 4016 / 0,
  crossing 1216 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0; no crash.
- verify-all `frontier2-lb1` (lanes 4-5, `SA_UNVERIFIED=frontier2-b1..b4`, head 3f216dfe, fresh release sa-oracle):
  IDENTICAL 13/13; default set `frontier2-lb-default` (lanes 4-5): IDENTICAL 13/13.
- i686 sa-game with and without `--features samp`, `FEATURES=samp tools/web/build.sh`: build.

## 2026-11-15 — FRONTIER-2 lane C: UNK-game, SCRIPT, AUDIO, CALLED-ONLY and the not-in-catalogue frontier (agent, branch worktree-agent-a9ea24442dceda2e3)
- Base: master 8c9cbf77. Scope: the frontier.py rows of subsystems UNK-game, SCRIPT, AUDIO, CALLED-ONLY and the rows not in
  the catalogue (226 at the start). Every port from the 1.0 US bytes (protection fragments and `.HOODLUM` bodies followed,
  the obfuscated constants decoded from the image; gta-reversed read as an aid), every address checked against
  `inflight.elsewhere()` first (none in flight). The unnamed entries were identified from their vtables (slot 4
  `GetTaskType` / slot 1 `GetEventType` against gta-reversed's enums), callers and bytes; 39 names added through
  `catalog/name-corrections.csv` (CAT-17), the catalogue regenerated with `tools/catalog/run_all.sh` (state.csv kept at
  master's rows except the renamed ones: run_all reset unrelated rows that state_check had recorded), the AI / pattern
  generators, bindings (`--resolve` for 0x4861F0) and the queue regenerated by their tools.
- Batch `frontier2-c1` (44 hooks + 3 natives): the attractor `std::vector` helpers (`fill`, `copy_backward`, `_Ucopy`,
  `_Ufill` of the CPed* / CTask* / CPedAttractor* / CPedTaskPair instantiations, 0x5EAAA0..0x5EC1B0, 0x5EAC50); 26 AI
  task destructors (new module sa-ai frontier_c_dtors.rs; 0x4994E0, 0x61BCA0, 0x659E00 are natives: a branch into the
  patched range / jmp thunks into game .text); CQuadTreeNode::operator delete 0x552C90, ~CQuadTreeNode 0x552520,
  CQuadTreeNodePool::Delete 0x552BC0, CPatrolRoute::operator delete 0x41B820.
- Batch `frontier2-c2` (35): the FxInfo complete destructors (ten), FxInfoAnimTexture / ColourRange / Smoke GetValue,
  FxEmitterBP_c's scalar destructor; CScriptsForBrains GetIndexOfScriptBrainWithThisName 0x46AA30 (the whole EAX as
  the original leaves it), Request / MarkNoLongerNeeded / StartAttractorScriptBrainWithThisName 0x46AA80 / 0x46AAE0 /
  0x46B390 (the obfuscated call decoded: 0x46B270), SwitchAllObjectBrainsWithThisID 0x46A900; DrawDebugSquare 0x486840
  (the release body: four ProcessVerticalLine probes); ThisIsAValidRandomPed 0x489490; Char / Car / ObjectInAreaCheck
  0x488B50 / 0x488EC0 / 0x489150 (unreversed in gta-reversed; one shared `area_check` tail); CEventSexyVehicle /
  CEventVehicleThreat constructors, CEventKnockOffBike::operator=, the PresentIDToCop / InteriorSitOnChair /
  DuckWhileShotsWhizzing constructors; object / vehicle pool CreateAtRef 0x59F610 / 0x6D5DD0; CMatrix::Rotate(vec)
  0x541E90; the female ped-type test 0x43A6A0; the script attractor slot claim 0x492CB0.
- Batch `frontier2-c3` (11): the active camera's under-water test 0x50B830, the script attractor ped-type test 0x5FE960,
  CStreamedScripts::FindStreamedScriptQuiet 0x4706F0, CScripted2dEffects::AddScripted2DEffect / GetEffectPairs 0x6FA7C0 /
  0x6FA840, CTheScripts::AddScriptSphere 0x483B30 / IsEntityWithinSearchLight 0x493900, the GoPickUpEntity /
  ReactToAttack constructors 0x6919C0 / 0x620B00, CPointRoute::Reverse 0x667720, the patrol anim record constructor
  0x46A340.
- Batch `frontier2-c4` (10 hooks + 15 natives): CRemote::TakeRemoteControlledCarFromPlayer 0x45AE80, the patrol route
  point add 0x46AE80, CAEMP3TrackLoader::IsCurrentAudioStreamAvailable 0x4E0BD0, COctTree::empty 0x5A6FC0 + its pool
  Delete 0x5A73C0, IsEntityWithinAnySearchLight 0x493960, StartNewScript(indexed) 0x464C90, the AvoidEntity constructor
  0x66AA20, FallAndStayDown::CreateSubTask 0x678BD0, the extinguish-fire task's fire search 0x659990. Natives for the
  not-in-catalogue rows: these are tail-jump targets Ghidra merged into their jumper's extent (CAT-12 gap; they are
  real functions, but cannot be hooked while outside the catalogue): the nine model-info store destructors
  0x4C5840..0x4C6390, CCheat::HealthCheat 0x438D60, the water sync stamp 0x6E76E0, the 0x977BE8 destructor body 0x454200,
  the explosion audio destructor 0x506B90, CHudColours::~CHudColours 0x58FD90, CVehicleStructure::operator new 0x4C9570.
- Gates: writer gate green on 1a7ea4e5 (c1..c3, 90 hooks) and on 3d3ccbee (c1..c4, 100 hooks): hooks / queue / bindings /
  state / dupes / cfg / layers / commits / clippy / i686 / i686clip / wasm32 / hooktest (59/59) / l1test / l1. The first gate
  run found three unpatchable entries (now natives) and two clippy findings. i686 sa-game with and without `samp`,
  `FEATURES=samp tools/web/build.sh` (wasm-bindgen of ~/sa-engine/target/tools/bin) built.
- verify-all (lanes 6-7): `lanec-c3` (SA_UNVERIFIED=frontier2-c1..c3) IDENTICAL 13/13; `lanec-c4` (c1..c4, 3d3ccbee)
  IDENTICAL 13/13; `lanec-default` (default set, 3d3ccbee) IDENTICAL 13/13.
- Scenarios (`tools/e2e/scenarios/frontier.sh`, i686 native, default lengths) with c1..c3 and again with c1..c4: pausemenu
  416 / 0 blocked, solicit 4016 / 0, crossing 1216 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0; no crash.
- frontier.py, lane C subsystems: 226 rows at the start (UNK-game 122, AUDIO 52, SCRIPT 29 incl. 2 partial, CALLED-ONLY 4,
  not in the catalogue 18) → 113 (UNK-game 45, AUDIO 51, SCRIPT 11, CALLED-ONLY 3, not in the catalogue 3); 116 of the
  starting addresses are registered by this branch. Whole frontier at the end: direct 268 + virtual 172 + partial 12.
- Left open / not ported by design: the audio decoders (CAEMFDecoder, CAEWMADecoder, CAEWaveDecoder), the CAEDataStream
  IStream stubs, CAEAudioChannel / CAEStreamingChannel / CAEStreamingDecoder destructors and CAEStreamThread's destructor
  0x4F11E0: they are excluded classes in `tools/gen/audio_patterns.py` (sound output and decoding stay original), and
  frontier.py has no exclusion mechanism for them (they stay listed). The four `vector::_Xlen` helpers 0x5ECAB0.. throw
  `length_error` (not worth a port). Big bodies left: CAEScriptAudioEntity::ProcessMissionAudioEvent, the aircraft engine
  audio (ProcessPlayerProp / DummyProp / PlayerHeli / PlayerSeaPlane), the weather thunder event, the angled-area script
  checks, tReplayVehicleBlock::ExtractVehicleUpdateData, the D3D9 raster helpers 0x4CA5A0 / 0x4CC0C0 / 0x4CD050, the
  remaining AI helper bodies (0x61AC90 / 0x61AD10 / 0x61ADF0 / 0x61FD20 / 0x61FF40 / 0x66AB40 / 0x66AD10 / 0x66FD30 /
  0x620C30 / 0x6221D0 / 0x65A110 / 0x66CA40 / 0x674930 / 0x4993D0 / 0x603570 / 0x603AF0 / 0x61D0B0). The partial script
  switches 0x47A760 / 0x48B590 were left to lane A. The CAT-12 gap behind the not-in-catalogue rows (tail-jump targets
  merged into their jumper's extent) is not fixed in the catalogue tooling; the natives cover phase B.

## 2026-11-15 — integrate FRONTIER-2 part 4 and lanes B and C (main session)

- Lane A (part 4) fast-forwarded onto 8c9cbf77; lane B rebased on it (conflicts: sa-ports NATIVES lists unioned,
  queue regenerated, the ROADMAP FRONTIER-2 row merged); lane C rebased on top (bindings.csv / name-corrections.csv
  unioned; 10 binding rows the union duplicated dropped; queue regenerated) — head 5ec1cfdb.
- Checked on the combined tree: sa-hook lib 59/59, host clippy, `make structure`, bindings `--check` (2,668),
  state_check 0 inconsistent, queue `--check`, i686 sa-game; scenarios pausemenu / solicit / crossing / melee /
  sniper / shootcar / shootdriver 0 blocked, no crash; verify-all (lanes 1-7, fresh sa-oracle) `intabc-u`
  (frontier2-1..9, b1..b4, c1..c4 on) IDENTICAL 13/13 and `intabc-def` IDENTICAL 13/13 (A+B alone: `intab-u` /
  `intab-def` IDENTICAL 13/13). `target/play/sa-game` and `web/pkg` refreshed.

## 2026-11-15 — FRONTIER-2 turn 2, lane B: WORLD / VEH / RW / RENDER / WEAPON / POP / MATH frontier (agent, branch worktree-agent-a01dd37b0c8f4b2db)
- Base: master d05f6e24. Lane B of the three-way split (subsystems WORLD, VEH, RW, RENDER, CAM, WEAPON, POP, STREAM, PED,
  ANIM, CORE, MATH, SAVE, PHYS). Every port from the 1.0 US bytes (gta-reversed read as an aid), every address checked
  against `inflight.py` before porting and again before hand-back (0 overlaps). The `.HOODLUM` bodies were followed
  through the no-CD protection: the xor / adc-decoded cells computed from the image, the 0xCB73E0 VM calls evaluated
  with unicorn on the exe image (the `pop ss` trap probe skipped): TriggerMessage's copy destination 0x96C014,
  ProcessGunShot's AddExplosion visibility 0, DoWeaponStuffAtStartOf2PlayerGame's player 0 (the VM rewrites its key
  0x14AA974D; V + K' = 0), RemoveInteriorLinks' 0x96EAC0, RegisterScriptRoadBlock's index 7. 45 new unverified hooks +
  1 phase B native in five batches:
  - `frontier2-b5` (25 + native; WORLD / VEH / POP / MATH): CGarages::TriggerMessage 0x447B80 (the end time is
    inclusive in the bytes), GivePlayerDetonator 0x448660, the hideout restore 0x4485C0 and the other-car test 0x449100
    (unnamed, called by CGarage::Update), 0x449A50 (a byte copy of StoreAndRemoveCarsForThisHideOut: same port),
    CGameLogic::RestorePedsWeapons 0x441D30 (checks the first model's load state twice, as the bytes do),
    DoWeaponStuffAtStartOf2PlayerGame 0x4428B0, ResetStuffUponResurrection 0x442980, CPickup::ExtractAmmoFromPickup
    0x454BE0 (obfuscated call decoded: GrantAmmo), ProcessGunShot 0x4588B0, CReplay::GoToNextBlock 0x45E2A0,
    PlayBackThisFrame 0x4604A0, SaveReplayToHD 0x45C340 / PlayReplayFromHD 0x460390 (route split),
    CCheat::EverybodyAttacksPlayerCheat 0x439C70, CWorld::FindMissionEntitiesIntersectingCube 0x569240 /
    FindObjectsIntersectingCube 0x568DD0, CShopping::StoreVehicleMods 0x49B280 / RestoreVehicleMods 0x49B3C0 (jump
    tables decoded), CStuckCarCheck AddCarToCheck 0x465970 / RemoveCarFromCheck 0x463B80 / AttemptToWarpVehicle 0x463A60,
    CPathFind::RemoveInteriorLinks 0x44DF60 (6 link bytes per node; gta-reversed declares 8),
    CRoadBlocks::RegisterScriptRoadBlock 0x460DF0, CCompressedMatrixNotAligned::DecompressIntoFullMatrix 0x59B9F0 (turn
    1's open item); native CCarAI::GetCarToGoToCoorsAccurate 0x41D0E0 (the existing unhookable port's phase B entry).
  - `frontier2-b6` (VEH, 6): CVehicle::CreateUpgradeAtomic 0x6D3510, AddUpgrade 0x6DFA20, CAutomobile::BoostJumpControl
    0x6A3A60, CTrain::RemoveRandomPassenger 0x6F6850 (rand site 0x6F6969), CAutomobile::SetTowLink 0x6B4410,
    CTrailer::SetTowLink 0x6CFDF0.
  - `frontier2-b7` (WORLD): CWorld::CallOffChaseForArea 0x566A60, FindNearestObjectOfType 0x5693F0,
    Interior_c::AddPickups 0x591F90, Interior_c::FindBoundingBox 0x5922C0.
  - `frontier2-b8` (RW / RENDER / WEAPON): _rpSkinBlendPositionsRigid 0x7CA330, _rpSkinBlendPositions1Weight 0x7CA240,
    _rpSkinGeometryNativeRead 0x7C88B0, the MatFX D3D9 mesh draw 0x812C90, the image depth copy 0x803840,
    CClouds::MovingFog_Create 0x713760 / MovingFog_Update 0x716B10, CWeapon::FindNearestTargetEntityWithScreenCoors 0x73E240.
  - `frontier2-b9` (VEH): CVehicle::PossiblyDropFreeFallBombForPlayer 0x6E07E0, GetPlaneOrdnancePosition 0x6D46E0.
- Hazards recorded where the original keeps values in its own argument slots (stack-residue / "not modelled" notes as
  turn 1 did), x87-transcendental for the FPATAN / FSQRT users.
- ADR-003: helpers shared instead of copied (sa-core `field::c_str_eq`; sa-world `mvpb_world` `scan_rect` /
  `sector_index` / `cell` / `advance_scan_code` made pub(crate); sa-veh `stuck_reset`; garage_update::now), private
  declarations made pub and imported (mvpd_w1 / mvpd_w4 / automobile_control / mvpd_v6 / sa-pop misc2 / misc3 /
  interior_bounding_box); bindings: new rows 0x447E40, 0x59B6A0, 0x569240, 0x6E3400, 0x457380, 0x7C7560; moved down to
  the lowest shared crate 0x4342A0, 0x6C21A0, 0x6DF930, 0x6C2150, 0x63BDD0 (twins where the port sits higher);
  `make structure` clean.
- Writer gate (base d05f6e24): the first run (b5 + b6 committed) found a needless late init (clippy), a copied
  `time_ms`, a second declaration of 0x5922C0 and the stale queue; fixed in the b7 commit. Second run on 01d7ec6b (45 hooks): hooks / queue / bindings / state / dupes / cfg / layers / commits / clippy / i686 /
  i686clip / wasm32 / hooktest / l1test / l1 all PASS; `make structure` clean.
- verify-all (lanes 4-5, fresh release sa-oracle): `lb2-u` (SA_UNVERIFIED=frontier2-b5..b9, head 5f8becb6) IDENTICAL
  13/13; default set `lb2-def` (head 01d7ec6b) IDENTICAL 13/13.
- Scenarios (`tools/e2e/scenarios/frontier.sh`, i686 native sa-game of this branch, default lengths): pausemenu 416 /
  0 blocked, solicit 4016 / 0, crossing 1216 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0;
  no crash.
- Builds: i686 sa-game with and without `--features samp`, `FEATURES=samp tools/web/build.sh` (wasm-bindgen of
  ~/sa-engine/target/tools/bin): build.
- frontier.py, lane B subsystems (to port, rows other branches register left out): 197 at the start → 159 (direct 179,
  virtual 165, partial 12 overall at the end, +57 / +5 registered on other branches). New rows my ports revealed: the
  replay callees 0x45D4B0, 0x45D760, 0x45F380, 0x4600F0 and the call-off-chase sector lists 0x563A80 / 0x563D00.
- Left open: the CCam / CCamera bodies (large x87, FPTAN chains), CWorld::FindObjectsIntersectingAngledCollisionBox, the
  CallOffChase sector lists, the replay playback chain, Interior_c furnishing (PlaceFurniture's uninitialised outputs),
  CReplay::RestoreStuffFromMem, SprayPaintWorld, RpLightGetConeAngle (inlined acos), the RW stream writers and D3D9
  raster helpers, the big vehicle bodies (FlyingControl, ProcessBoatControl, the BlowUpCars / PreRenders / ProcessControls
  of heli / plane / boat / quad), the remaining WEAPON bodies, GangWars::TellGangMembersTo (event / VM mix),
  ClitargetOrientationToLink. Names for the unnamed garage entries 0x4485C0 / 0x449100 / 0x449A50 and the RW helpers
  0x7C88B0 / 0x812C90 / 0x803840 / 0x7CA240 / 0x7CA330 are proposed in the port docs (not added to name-corrections).
## 2026-11-15 — FRONTIER-2 turn 2 lane C: stays-original list, CAT-12 tail targets, generator drift, batches c5 / c6 (agent, branch worktree-agent-afdd623e21965992f)
- Base: master d05f6e24. Scope: frontier.py rows of AUDIO, UNK-game, SCRIPT, CALLED-ONLY and not-in-catalogue, plus three
  tooling items. Every port address checked against `tools/gen/inflight.py` first (none in flight).
- Tooling (a), stays original: `catalog/stays-original.csv` (key = address or `class:NAME`, reason), read through
  `tools/lib/stays_original.py` by frontier.py (left out of the lists, `stays original N` in the summary line,
  `--stays-original` lists them with the reason), audio_patterns.py (replaces its EXCLUDE_CLASS regex) and patterns.py
  (which had parsed that regex out of audio_patterns.py). Rows: the DirectSound channels, the stream / smooth-fade
  threads, the decoders (streaming base, MF, WMA, Wave, Vorbis), CAEDataStream (its IStream stubs), CAEStreamThread's
  destructor 0x4F11E0, the class vt_866db0 (a Winsock client: socket / connect / send / closesocket through WS2_32
  ordinals; its guessed slot names SynchPlayback / Play are wrong, it is not the user-track player), the
  std::logic_error / length_error virtuals 0x4F3D50 / 0x4F3D60 / 0x4F4110 (reached only through a thrown exception) and
  the bug-report class 0x862E70 (0x508F90 / 0x509170, development tooling). 0x4F4620, which audio_patterns.py excluded
  as "user-tracks file output", is a `std::vector<T*>::push_back` (kept generator-local).
- Tooling (b), CAT-12 tail targets: `callgraph.py build` (`split_tail_target`) emits a `tailcall` edge for a `jmp` into
  the jumper's own extent when the target starts a body range of its own on a 16-byte boundary after padding or `ret`,
  the jump sits in the range holding the entry (protection fragments jumping back are not tail jumps; the first version
  without this test added 0x5BD890, a loop head of LoadHandlingData, removed again) and the target is before the entry
  or a catalogued entry lies between. No Ghidra re-analysis: `gaps` appends the target, the next pass's Ghidra dump
  creates the function. 34 rows (33 game, 0x819770 QuickTime): lane C's 15 natives' bodies, CAEStreamThread /
  CAESoundManager / cBuoyancy / allocator / break-manager / oct-tree destructors, CMemoryHeap::CMemoryHeap,
  CPad::IsLeftShockPressed, CWaterLevel::HandleBeachToysStuff, CAEWeatherAudioEntity's destructor. Side effect:
  AUDIO-GEN no longer matches `__dtor_AESmoothFadeThread` 0x856300 (verified) once its tail target 0x4EE9E0 is a
  function; re-added by hand (frontier2-c5, needs re-verification) with a native for 0x4EE9E0. Notes in
  `docs/re/cat9-12-pipeline.md`.
- Tooling (c), generator drift: `cfg_check.py` counted the generators' own test gate `cfg(all(test, not(sa_phase_a)))`
  in every new generated file (not in the baseline), so lane C left the new sa-audio / sa-hud generated_leaves.rs out and
  master's `patterns.py --check` differed (plus lane B's natives and new catalogue names never regenerated). Generator
  output is no longer counted for that gate (baseline 139 → 86 files), all three generators regenerated, and the writer
  gate has a `gen` step (patterns / ai_patterns / audio_patterns `--check`, project venv).
- Catalogue: `run_all.sh` three times (CAT-12 change; batch c5 names; batch c6 names); each time `catalog/state.csv` was
  restored to the base rows except new and renamed addresses (run_all reset 74-75 unrelated rows that state_check had
  recorded). 23 CAT-17 names in `catalog/name-corrections.csv`.
- Batch frontier2-c5 (22 hooks): FurnitureManager_c / ProcObjectMan_c / WaterCreatureManager_c / IKChainManager_c /
  CMatrixLinkList destructors, List_c::GetItemOffset 0x4A9010, CMemoryHeap::Malloc 0x72ED80, CAESoundManager::~ 0x4EFD60,
  CTheScripts::IsPointWithinSearchLight 0x493280, the streamed-script release 0x40C1C0, CTaskComplexBeCop /
  FollowPatrolRoute / SeekEntity (0x66CA40) constructors, the looped-middle anim task (vtable 0x86D594) constructors
  0x61AC90 / 0x61AD10 and anim start 0x61ADF0, the be-kicked-on-ground anim start 0x61FD20, the kill-all-threats scan
  0x6221D0, the avoid-entity look release 0x66AD10, RpGeometryReplaceOldMaterialWithNewMaterial 0x734DE0, the D3D9 raster
  format setter 0x4CC580, `__dtor_AESmoothFadeThread` 0x856300.
- Batch frontier2-c6 (12 hooks): CPatrolRoute::operator new 0x41B810, COctTree::~COctTree 0x5A7400 / FillPalette 0x5A70F0,
  BreakManager_c::~ 0x59E640, the six CPedGroupDefaultTaskAllocator* destructors 0x5F6460..0x5F6550,
  FxInfoAttractLine_c::GetValue 0x4A5850, the scripted 2D effect's nearest ped 0x603570.
- Natives (9): the four `std::vector<...>::_Xlen` 0x5ECAB0.. (they throw length_error; phase B cannot raise a C++
  exception, the native stops the game), CPathFind::StoreDetachedNodeInfoPed 0x44D2E0 (`ret 0x38`),
  CAEFrontendAudioEntity's destructor 0x506B20 and CTaskComplexReactToAttack::CreateSubTask 0x620C30 (the protection reads
  their entry bytes as data; the latter was a hook until the sa-hook keycells / patch_refs tests refused it), the
  smooth-fade thread destructor 0x4EE9E0, COctTreeBase::~ 0x5A4140 (a 5-byte `jmp` into game .text); AUDIO-GEN's
  CAEAudioEntity::UpdateParameters 0x4D7E30 registered.
- Not ported: 0x422F80 (ST0 result called through an `f64` declaration), CMemoryHeap::FillInBlockData 0x72EC80 and
  0x66D4B0 / 0x603AF0 (protection fragments inside), the big bodies (ProcessMissionAudioEvent, the four aircraft engine
  audio functions, the weather thunder event, the angled-area checks, ExtractVehicleUpdateData, the D3D9 raster helpers
  0x4CA5A0 / 0x4CC0C0 / 0x4CD050), 0x61D0B0 / 0x61FF40 / 0x66AB40 / 0x66FD30 / 0x65A110 (stack objects / x87 results),
  0x477DA0 (still not in the catalogue: reached only through an obfuscated call), the partial script switches (lane A).
- frontier.py, lane C subsystems: 114 rows at the start (UNK-game 46, AUDIO 51, SCRIPT 11, CALLED-ONLY 3, not in the
  catalogue 3) → 38 to port (UNK-game 16, AUDIO 9, SCRIPT 9, CALLED-ONLY 3, not in the catalogue 1; 4 of them new callees
  of the ports: 0x4A4880, 0x4CC1D0, 0x66D4B0, 0x72EC80) + 48 listed as stays original. Whole frontier: to port 456 → 380
  (`--no-branches`), stays original 48.
- Gates: writer gate (base d05f6e24, on 1a07f443) green: hooks (37 new, all unverified) / bindings / state / dupes / cfg / gen / layers / commits / clippy / i686 / i686clip / wasm32 / hooktest 59/59 / l1test / l1; queue up to date after its regeneration commit. i686 sa-game with and without `samp`, `FEATURES=samp tools/web/build.sh`
  built.
- Scenarios (`frontier.sh`, i686 native, default lengths; output in target/lanec2-scen): pausemenu 416 / 0 blocked,
  solicit 4016 / 0, crossing 1216 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0; no crash.
- verify-all (lanes 6-7, fresh sa-oracle, DLL of 1a07f443): `lanec2-c56` (SA_UNVERIFIED=frontier2-c5,frontier2-c6) IDENTICAL
  13/13; `lanec2-default` (default set) IDENTICAL 13/13. (A first `lanec2-c5` run was stopped after 4/13 IDENTICAL in
  favour of the combined one.)
## 2026-11-15 — FRONTIER-2 turn 2, lane A: the crossing-4000 block, GenerateRoadBlocks, AI frontier (agent, branch worktree-agent-a3af6b88de448512f)
- Base: master d05f6e24. Lane A of the turn-2 split (AI subsystem plus lane A's open items). Every port from the 1.0 US
  bytes (gta-reversed read as an aid), every address checked first against `inflight.py`'s output (other branches /
  worktrees) and against this checkout's own registrations (one address, 0x62F2D0, turned out to be a generated port
  already: dropped).
- Batch `frontier2-a1`: CTaskComplexKillCriminal::CreateNextSubTask 0x68E4F0 (crossing 4000's last block) and the
  CreateSubTask 0x68C050 it calls (the ControlSubTask switch `create_chosen` reused after its reuse test). Bytes vs
  gta-reversed: the leave-car case tests the vehicle pointer, the enter-as-driver case sets +0x1A. New scenario
  `crossing-long` (the crossing run to 4000): 4016 ok / 0 blocked (CreateNextSubTask runs: 2 calls under gdb,
  tools/e2e/scenarios/frontier2-a-counts.gdb).
- L1 specs for the preserve-ecx event destructors 0x4AECA0 / 0x4B1320 / 0x4B1B50 (one shared builder in specs_ai.rs),
  removed from PRESERVE_SPECS_PENDING; `sa-oracle l1 --backend native --cases 100000` PASS each.
- CRoadBlocks::GenerateRoadBlocks 0x4629E0: whole port (the ADR-002 fallback removed): the wanted-level road blocks
  (InOrOut, the per-frame sixteenth of RoadBlockNodes, NumRoadBlocks read through the protection's obfuscated pointer
  at 0x402033, decoded to 0xA43580; rand at 0x462BE9; the width and no-width cases) and the frame's script road block.
  Bytes vs gta-reversed in the no-width case: `fabs` on the dot product, the search stops after the first match, and
  the block across the node itself is made whenever nothing matched. CreateRoadBlockBetween2Points 0x4619C0 stays on
  the original (POP, open). port-l2 on police-gang: L2 4992 of 4992 calls compared, 0 mismatches / 0 undeclared; the
  rust run IDENTICAL over 5200 frames. New scenario `roadblocks` (the wanted-level cheat OSRBLHH typed twice in the
  Greenwood, then driven): GenerateRoadBlocks no longer blocks.
- Batch `frontier2-a2` (23 hooks + 2 natives): constructors (CTaskSimpleJetPack 0x67B4E0, ArrestPed 0x68B990,
  GoToPointAiming 0x668790, FollowLeaderInFormation 0x6949A0, Sunbathe 0x631F80; natives: WanderMedic 0x658770 with
  its bare-jmp entry, the island's float read from 0x86FCB0), CanSunbathe 0x632140, CTaskAllocatorAttack::AllocateTasks
  0x69C3F0, OnFire::ComputeFireDamage 0x6333D0, TreatAccident::ComputeHeading 0x658AF0, GangDriveBy::MakeAbortable
  0x62D290 + its anim start 0x627B20, CreateCarAndGetIn::Destructor 0x64CEA0, the CreateSubTask switches of CopInCar
  0x68C9E0, MedicTreatInjuredPed 0x658DB0, EvasiveCower 0x655570, ArrestPed 0x68CF80, WaitForBackup 0x6973B0,
  ShuffleSeats 0x63D240 (+ CreateTaskUtilityLineUpPedWithCar 0x63D610), EnterCarAsPassengerWait 0x6408D0,
  ReactToGunAimedAt 0x697B70 (native: UseSwatRope 0x659620, the protection reads its entry at 0x15694D9), the swat
  rope anim task ctor 0x6586A0, KillPedGroupOnFoot::ComputeNearestLivingGroupPed 0x622450. Original quirks kept: the
  null-task `+0x1C` write and the `AddTask` on a null sequence of 0x6408D0, the heading byte written over the ped
  argument slot of 0x697B70 (passed as that dword).
- Batch `frontier2-a3` (5): CPedAttractorManager::GetPedUsingEffect<CPedAttractor> 0x5EB740,
  CTaskInteriorBeInOffice::GetInfoForPedToUse 0x675300, CTaskSimpleJetPack::DropJetPack 0x67B660 / ApplyRollAndPitch
  0x67B720, CConversations::IsConversationAtNode 0x43B000 (its upper EAX bytes per path).
- Batch `frontier2-a4` (2): CTaskComplexCopInCar::ControlSubTask 0x68FD50 (the roadblocks scenario's next block),
  CTaskSimpleHoldEntity::ChoosePutDownHeight 0x693440.
- Bindings (ADR-003, `tools/gen/bindings.py --resolve`): rows 0x4654B0 (moved down to sa-ai), 0x486670, 0x63B030,
  0x63CE00, 0x646810, 0x649450, 0x458A80, 0x601390 / 0x668C70 / 0x678CD0 (restored), 0x5F3B70's result (u32);
  `vec3f_ptr` added to sa-core field.rs (replaces a copy in route_round_entity_bbox.rs; dupes clean).
- `new_task` (the task-pool `new` + constructor idiom) now defined once in sa-ai common.rs; seven sa-script copies import
  it (dupes.py found the new cross-crate group).
- Scenarios (frontier.sh, i686 native, default lengths, final tree): pausemenu 416 / 0 blocked, solicit 4016 / 0,
  crossing 1216 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0, crossing-long 4016 / 0
  (was 2 blocked on 0x68E4F0), roadblocks 291 / 1725 blocked (CTaskSimpleGangDriveBy::ProcessPed 0x62D3B0, AI, open;
  CHeli::GenerateHeli 0x6C6520, VEH); no crash.
- verify-all (lanes 1-3, fresh sa-oracle): `f2a-u3` (SA_UNVERIFIED=frontier2-a1..a3) IDENTICAL 13/13; `f2a-u4`
  (frontier2-a1..a4) IDENTICAL 13/13; `f2a-def` (default set, includes the GenerateRoadBlocks whole port)
  IDENTICAL 13/13. save-load: save files identical except the SAVE-1 stack residue, as on master.
- Writer gate (base d05f6e24, target dir target-gate): hooks (32 new, all unverified, batches frontier2-a1..a4) /
  bindings / state / dupes / cfg / layers / commits / clippy / i686 / i686clip / wasm32 / hooktest (59/59) / l1test /
  l1 PASS; queue up to date at HEAD (regenerated last). The first gate run failed dupes and i686clip only because the
  `new_task` refactor landed mid-run.
- Builds: i686 sa-game with and without `--features samp`, `FEATURES=samp tools/web/build.sh` (wasm-bindgen of
  ~/sa-engine/target/tools/bin) built.
- frontier.py: AI 143 (41 direct + 102 virtual) → 112 (14 direct + 98 virtual); partial ports 12 → 11.
- Left open: CTaskSimpleGangDriveBy::ProcessPed 0x62D3B0 and its unported helpers 0x621960 / 0x627600 / 0x627CC0 /
  0x628350 (FPATAN / FSIN / FCOS; the police drive-by in a wanted-level chase), CreateRoadBlockBetween2Points 0x4619C0
  (POP), CGangWars::Update's active-war path and CCollision::ProcessColModels' disk cases (not reached), the task
  allocators 0x69BCA0 / 0x69CC30 / 0x69D460 / 0x69C4D0, EndConversation 0x43AB10 (obfuscated booleans), the
  transcendental bodies (CalcSoundLevelIncrement, GetDisplacement, CalcBlendRatio), the rest of the AI frontier.

## 2026-11-15 — FRONTIER-2 turn 2, lanes A, B and C on master (main session)
- Fast-forwarded master to 54dd163d: lane B (b5..b9, 9 commits), lane C (tooling + c5/c6, 6 commits) and lane A (a1..a4,
  10 commits) rebased in that order onto 1f765b2a (the cpal 0.18 fix), plus two main-session commits: 44f39ff6 drops the
  four binding rows (0x4342A0, 0x6C2150, 0x6C21A0, 0x6DF930) that lane B moved down and lane C's catalogue regeneration
  kept, and 54dd163d regenerates the verify queue (2905 unverified hooks in 390 batches; lane B's own queue was stale).
  Conflicts: queue files (regenerated), sa-ports NATIVES and sa-rwcore module lists (union), the FRONTIER-2 ROADMAP row
  (each lane's appended clause kept), WORKLOG (both entries).
- Verification on the merged tree: writer gate base d05f6e24 all PASS except `gen`, which needs the main checkout's
  .venv (patterns / ai_patterns / audio_patterns --check run by hand with it and SA_ROOT: no difference; without SA_ROOT a
  worktree outside .claude/worktrees reads no coverage and drops the `recs` hints, an environment effect only).
  verify-all int3-u (SA_UNVERIFIED = the 28 frontier2 batches) IDENTICAL 13/13; int3-def (default set) 12/13: the
  air-water Rust run stopped 0.6 s after start with no trace (Wine start-up), its re-run int3-def2 IDENTICAL over 5800
  frames. Scenarios on the merged i686 sa-game: pausemenu, solicit, crossing, crossing-long, melee, sniper, shootcar,
  shootdriver 0 blocked; roadblocks 291 ok / 1725 blocked (GangDriveBy::ProcessPed 0x62D3B0, lane A turn 3;
  CHeli::GenerateHeli 0x6C6520, lane B turn 3).
- frontier.py --no-branches: 456 → 311 to port (direct 183, virtual 117, partial 11) + 48 stays original.
- User-hit on samp.rss-ro.com (web build): CQuadBike::ProcessControl 0x6CDCC0 (vtable 0x871AE8 slot +0x28), handed to
  lane B turn 3 as its first item.
- Play package: target/play/sa-game is now built with `--features audio` and play.sh sets SA_GAME_AUDIO=cpal (the user
  confirmed sound after 1f765b2a); SA_GAME_AUDIO=null mutes it.

## 2026-11-15 — FRONTIER-2 turn 3 lane C: state.csv guard, X87 binding, batches c7 / c8 (agent, branch worktree-agent-a04a02581f061bd35)
- Base: lane C turn 2 head a7d67339 (branch worktree-agent-afdd623e21965992f). Every port address checked against
  `tools/gen/inflight.py` first (none in flight).
- Tooling (item 4), state.csv never lowered: the ~75 rows run_all reset were (a) the 51 rows `state_check.py --fix` had
  demoted (evidence `state_check: no Rust port (was ...)`), whose evidence and date `progress.py merge` rewrote on every
  run, one of them (0x43E0E0, unnamed) falling from specified to unknown, and (b) 23 legitimate updates (CAT-17 names
  that earlier lanes had reverted by restoring state.csv by hand). merge now keeps a row whose evidence is such a hand
  note while the derived state is not above it; new `progress.py guard OLD NEW [--allow-hand]` fails on any lowered
  row; run_all.sh runs it around the merge (step `state_guard`), the writer gate's `state` step against the base commit
  (state_check demotions allowed). Two run_all runs give the same state.csv; the guard reports 0 rows lowered.
- Binding layer (item 2, 0x422F80): `CGeneral::GetRadianAngleBetweenPoints` 0x53CBE0 is bound with its full ST0
  (`sa_math::x87::X87`; its phase B native returns the sa-math X87 model); the 29 existing callers convert with
  `.to_f64()` at the call (behaviour unchanged). 0x422F80 is bound to its port with an X87 result (carctrl6's f64
  `orig!` removed).
- Batch frontier2-c7 (6 hooks): CRunningScript::CharInAngledAreaCheckCommand 0x487F60 (new sa-script angled_area.rs:
  the corner / projection x87 data flow followed store by store), FindClosestPtOnLine 0x4A4880, the plane route
  planner's probes 0x422E10 / 0x422F80 (new sa-pop frontier_c.rs), the group route-round 0x66FD30 (sa-ai), the D3D9
  format check by raster type 0x4CC0C0 (args in EAX / ECX / EDX; regconv test passes). New bindings: 0x422E10,
  0x422F80, 0x66F6C0, 0x485EF0, 0x486990, 0x4CBE20 (d3d9rast's local declaration merged by `--resolve`), 0x4CBF20.
- Batch frontier2-c8 (10 hooks, item 3): the turn-1 natives whose bodies the CAT-12 tail-target split made catalogue
  rows: the nine model-info store destructor bodies 0x4C5840..0x4C6390 (CAT-17 names `CModelInfo::ms_*Store::~CStore`,
  sa-hook's table test wants the catalogue name) and CCheat::HealthCheat 0x438D60. Left natives: 0x454200, 0x506B90,
  0x58FD90 (catalogued, not converted for time; 0x506B90's neighbour 0x506B20 has its entry read as data, check the
  keycells first), 0x6E76E0 and 0x4C9570 (still not catalogue rows).
- Not ported (the angled-area Object / Flame variants 0x4883F0 / 0x488780 have a different x87 data flow from Char's:
  dx and the squared length stay 80-bit, the second side length comes from the f32 stores, so they need their own
  bodies, not the Char helpers): ProcessMissionAudioEvent, the four aircraft engine audio functions, the weather thunder
  event 0x506800, ExtractVehicleUpdateData 0x45BA90 / 0x45DEF0, the D3D9 raster helpers 0x4CA5A0 / 0x4CD050 / 0x4CC1D0,
  the protection-fragment bodies 0x72EC80 / 0x66D4B0 / 0x603AF0, 0x61D0B0 / 0x61FF40 / 0x66AB40 / 0x65A110, 0x477DA0.
- Gates: writer gate (base a7d67339, on cc15a52c) all PASS: hooks (16 new, all unverified; notes: c7 spans CALLED-ONLY /
  SCRIPT / UNK-game, c8 STREAM / WORLD) / queue / bindings / state (state guard 0 rows lowered) / dupes / cfg / gen /
  layers / commits / clippy / i686 / i686clip / wasm32 / hooktest 59/59 / l1test / l1 (no L1-pure hook). The first
  hooktest run caught the c8 names (the catalogue name is required) and HealthCheat's 8-byte steal; the first
  verify-all run caught `#[sa_core::hook(` (hook-names.py / queue.py only match `#[hook(`).
- Scenarios (`frontier.sh`, i686 native sa-game of cc15a52c, default lengths; target/lc3/scen): pausemenu 416 / 0 blocked,
  solicit 4016 / 0, crossing 1216 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0; no crash.
- verify-all (lanes 6-7, shared with two other sessions, so ~70 min a run): `lanec3-c78b` (SA_UNVERIFIED=frontier2-c7,
  frontier2-c8) IDENTICAL 13/13; `lanec3-default` (default set) IDENTICAL 13/13 (DLL of cc15a52c).
- frontier.py: whole frontier to port 302 → 288 (direct 175 → 168, virtual 115 → 108); lane C subsystems 38 → 36 rows
  (0x485EF0 HighlightImportantAngledArea, 0x486990 DrawDebugAngledSquare and 0x66F6C0 are new callees of the ports).

## 2026-11-15 — FRONTIER-2 turn 3, lane B: VehicleCheat, aircraft / boat blow-ups, GenerateHeli, glass, call-off-chase (agent, branch worktree-agent-a75c5759aead26958)
- Base: worktree-agent-a01dd37b0c8f4b2db 92ae87f1 (turn 2 lane B, being integrated in parallel). Lane B subsystems; every
  port from the 1.0 US bytes (gta-reversed read as an aid), addresses checked against `inflight.py` / frontier.py's
  other-branch exclusions (0 overlaps). Two batches, 14 hooks:
  - `frontier2-b11` (user-hit, coordinator-added, first): CQuadBike::ProcessControl 0x6CDCC0 (the user hit it in the
    SA:MP web build: every iteration blocked with a quad nearby; gta-reversed only calls the original, ported from the
    bytes: the player quad's damping, two `_CIpow(x, timeStep)` through orig::x87_binary, two turn forces with the bytes'
    f32 stores) and what the new `frontier.sh quad` scenario (quad cheat AKJJYGLC, CJ gets on and drives) reached next:
    CQuadBike::ProcessAI 0x6CE460 (lean forces, air steering, wheelie angle; the bytes differ from gta-reversed's
    reading: tasks 0x329 / 0x337 / 0x339 or state 0x3F brake, air steering only when not accelerating, at
    `centreOfMass + up`) and CQuadBike::PreRender 0x6CEAD0 (axles by FPATAN, handlebars), plus sa-game's DirectSound
    IDirectSoundBuffer8::SetFX (removing succeeds, adding fails as on a buffer without FX control; the radio's bass EQ
    reached it). Scenario quad: 1216 steps, 0 blocked (ProcessControl / ProcessAI 998 calls, PreRender 555,
    ProcessControlInputs 216);
  - `frontier2-b10` (10 hooks):
  - priority 1: CCheat::VehicleCheat 0x43A0B0 (the `nop; jmp` protection stub entry; jump table 0x43A470 decoded: model
    info type 1 monster truck, 2 quad, 3 heli, 4 plane, 5 boat, 9 bike, 10 BMX, 11 trailer, the rest automobiles; the
    null-`new` bike path faults in the original, the port stops), CPlane::BlowUpCar 0x6CCCF0 / CHeli::BlowUpCar 0x6C6D30
    (crash start for AI aircraft, player bonus with rand sites 0x6CCD82 / 0x6C6DC8, body damage (the plane checks the
    bonnet frame, the heli does not), fire, explosion type 12 for the RC models else 7, the hide flag never used),
    CBoat::BlowUpCar 0x6F21B0 (explosion type 6 with the hide flag, the moving part thrown off as a temporary CObject),
    CHeli::GenerateHeli 0x6C6520 (added by the coordinator for lane A's `roadblocks`; FSIN / FCOS through sa_math::x87,
    the cosine taken of the unrounded product as the bytes do);
  - priority 2: CWorld::CallOffChaseForAreaSectorListVehicles 0x563A80 (grown rectangle, law-enforcer chase missions,
    the speed toward the area cut) and ...Peds 0x563D00 (the original rectangle; kill / chase targets, stand-still
    script-command event), FindObjectsIntersectingAngledCollisionBox 0x568FF0, CGlass::WindowRespondsToCollision
    0x71BC40 (minZ / maxZ and the farthest vertex with the x87 compares' NaN choices);
  - priority 3: CReplay::StreamAllNecessaryCarsAndPeds 0x45D4B0 (.HOODLUM body 0x156A8C0).
- Hazards: stack-residue where the original keeps values in its own argument slots (blow-ups, GenerateHeli, the glass
  port), x87-transcendental for GenerateHeli; notes as turn 2.
- ADR-003: the automobile blow-up / flying-component declarations made pub(crate) and shared; sa-world
  `mvpd_w3::ensure_matrix` made pub and reused by the glass port (the gate's dupes step caught a copy); the existing
  callers' orig! declarations (cheat.rs, frontier_b2.rs, air2.rs) now target the ports; bindings.csv: 0x71BC40 gets its
  sa-render twin (port column, regenerated).
- frontier.py, lane B subsystems: 159 → 146 rows (overall direct 178 → 165, virtual 165 → 152; other branches now
  register +64 / +12); no new lane B rows revealed by the ports.
- Scenarios (`frontier.sh`, i686 native sa-game of this branch, default lengths): pausemenu 416 / 0 blocked, solicit
  4016 / 0, crossing 1216 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0, quad (new)
  1216 / 0 (b11 build; the same seven were 0 blocked on the b10 build too). Lane A's
  `roadblocks` (definition copied from worktree-agent-a3af6b88de448512f, not committed): 254 / 1762 blocked: 4 on lane
  A's GenerateRoadBlocks fallback (not in this branch), then CHeli::ProcessControl 0x6C7050 (the generated police heli
  now exists; next lane B target).
- Writer gate (base 92ae87f1): b10 run all PASS but queue (line numbers moved by the dupes fix; regenerated and
  `queue.py --check` up to date); final run on 425166bf (b10 + b11) with `GATE_SKIP=i686` (verify-all was using the DLL):
  hooks (13) / queue / bindings / state / dupes / cfg / layers / commits / clippy / wasm32 / hooktest / l1test / l1 PASS;
  i686clip run by hand afterwards: PASS; the i686 DLL build is the one verify-all lb3-u2 made.
- verify-all (lanes 4-5, fresh release sa-oracle): `lb3-u` (SA_UNVERIFIED=frontier2-b10, DLL of 747a33d8) IDENTICAL
  13/13; `lb3-u2` (SA_UNVERIFIED=frontier2-b10,frontier2-b11, DLL of 425166bf) IDENTICAL 13/13; default set `lb3-def` (same head) IDENTICAL 13/13.
- Builds: i686 sa-game (scenarios), the i686 DLL (verify-all), wasm32 sa-ports (gate). Coordinator heads-up on the
  0x53CBE0 binding change: no code of this branch calls it.
## 2026-11-15 — FRONTIER-2 turn 3, lane A: the gang drive-by, the roadblocks AI chain (agent, branch worktree-agent-a79dbccbc537c6d52)
- Base: worktree-agent-a3af6b88de448512f (07c98248, lane A turn 2). Every port from the 1.0 US bytes (gta-reversed read as an
  aid; it reverses none of the drive-by bodies, and its unhooked ArrestPed::ControlSubTask draft differs from the bytes),
  every address checked against `inflight.py` first.
- Batch `frontier2-a5` (6 hooks): CTaskSimpleGangDriveBy::ProcessPed 0x62D3B0 (the command state machine, the anim
  crossings at 0.2667 / 0.3 / 0.4333, the attack timer, the player's pad, the speed `Say`s) and its helpers: the player's
  anim choice 0x621960 and the AI's 0x627600 (FPATAN of the aim / target direction minus the vehicle heading, the
  quadrant by `_ftol`; the AI's side / front flags, one of which can never be set), the line-of-sight test 0x621B10, the
  shot 0x627CC0 (the muzzle through the hand bone, the vehicle side / front plane crossing, the no-CD island at 0x628234
  that ORs ped +0x528 with 1, CWeapon::Fire, DoGunFlash) and the gun pointing 0x628350 (FSIN / FCOS of the bike lean,
  `_CIasin` through orig::x87_unary). sa-world common gained `ped_active_weapon` (sa-ped player_target's copy imports it).
- Batch `frontier2-a6` (8 hooks): the roadblocks scenario's next blocks, one after the other:
  CTaskComplexArrestPed::ControlSubTask 0x68D350 / CreateNextSubTask 0x690220, CTaskSimpleArrestPed::ProcessPed 0x68B8A0
  (an uninitialised bone local without a suspect: zero here), CTaskAllocatorKillThreatsBasicRandomGroup::AllocateTasks
  0x69D460, CTaskComplexDestroyCarArmed::CreateNextSubTask 0x62DF20; plus CEvent::CalcSoundLevelIncrement 0x4AC050
  (`_CIpow`, FYL2X; route L2 since `_CIpow` is the CRT's only in-process), CTaskComplexFollowPointRoute::CalcBlendRatio
  0x66E710 (FCOS), CTaskComplexGoToPointAndStandStillTimed::Clone 0x66CF30.
- Bindings (bindings.py --resolve): 0x627B20, 0x6D3080, 0x6D5CF0, 0x5DF300, 0x5F8510, 0x622240, 0x6222B0, 0x69C850, 0x669340
  bound in sa-ai; 0x5E5ED0 moved down to sa-pop with its sa-ped twin (port column set).
- sa-hook callers scan: `or r8, 0FFh` counts as a write of the byte, like `or r, -1` (0x627828 after the call to 0x621B10).
- Scenarios (frontier.sh, i686 native, default lengths): pausemenu 416 / 0 blocked, solicit 4016 / 0, crossing 1216 / 0,
  melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0, crossing-long 4016 / 0, roadblocks 291 ok /
  1725 blocked → 875 / 1141, blocked only on lane B's CCam::ArrestCamOne 0x518500 (1135, reached once the arrest runs) and
  CHeli::GenerateHeli 0x6C6520 (6); no crash.
- frontier.py: AI 112 (14 direct + 98 virtual) → 103 (12 + 91); total to-port 346 → 330.
- Left open: CRoadBlocks::CreateRoadBlockBetween2Points 0x4619C0 (~1,000 instructions with SEH-guarded CMatrix locals, not
  started), EndConversation 0x43AB10 (its AbortLookAt time comes through the 0xCB73E0 decoder: needs the unicorn
  evaluation), CPedShelterAttractor::GetDisplacement 0x5EF420 (calls the unported vector push_back 0x5EE370), the
  allocators 0x69BCA0 / 0x69CC30 / 0x69C4D0, CGangWars::Update's wars, ProcessColModels' disks, the rest of the AI
  frontier.
- verify-all (lanes 1-3, fresh sa-oracle): the first `f2a3-u` run (SA_UNVERIFIED=frontier2-a5,frontier2-a6) diverged on
  car-ai at frame 2069 (randcalls); bisect-batch: a6 IDENTICAL, a5's culprit 0x621B10 alone (its `rand()` was not
  attributed to the original call site): fixed with `orig::at_site(0x621B99, rand)`. Then `f2a3-u2` (a5 + a6) IDENTICAL
  13/13 and `f2a3-def2` (default set) IDENTICAL 13/13; save-load identical except the SAVE-1 stack residue.
- Writer gate (base 07c98248): all 16 steps PASS (hooks 14 new, all unverified; queue up to date at HEAD). The first run
  failed hooktest: 0x4AC050's expect ended inside an instruction, and the callers scan read `or dl, 0FFh` at 0x627828 as
  a use of EDX (fixed in sa-hook callers.rs: `or r8, 0FFh` is a write).
- Integration note: lane C turn 3 makes the 0x53CBE0 binding return X87; the `st0(general_get_radian_angle_between_points
  (..))` sites here (frontier_a_t3.rs ArrestPed ProcessPed, gang_drive_by.rs point-gun x2) drop `st0()` then.
## 2026-11-16 — FRONTIER-2 turn 3, lanes B and A on master (main session)
- Lane C turn 3 went in first (d9866059, with the X87 caller fix 7d1189ef; verify-all `intc3` IDENTICAL 13/13 both modes).
- Lanes B (worktree-agent-a75c5759aead26958) and A (worktree-agent-a79dbccbc537c6d52) rebased in that order onto d9866059
  (`rebase --onto`; queue → theirs, lib.rs / hooks.rs / WORKLOG union, ROADMAP docmerge, bindings / name-corrections sorted
  union), each followed by `queue.py gen` (1cc13a25, 48633fb3). Lane A's three `st0(general_get_radian_angle_between_points
  (..))` sites now take the X87 result (`.to_f32()` or the X87 value directly; 7c52736c), dropping an unused fpu import.
- Writer gate on the B tree all PASS; on the A tree clippy host / i686 PASS after the import fix (the rest unchanged).
- Scenarios (sa-game i686 with `--features audio`): pausemenu 416 / 0 blocked, solicit 4016 / 0, crossing 1216 / 0,
  crossing-long 4016 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0, quad 1216 / 0 (the
  user-hit quad blocker is gone), roadblocks 258 / 1758, every block on CHeli::ProcessControl 0x6C7050 (the police heli now
  spawns through GenerateHeli; its control is lane B turn 4's first item).
- verify-all (lanes 1-7, fresh sa-oracle): `intab3-u` (SA_UNVERIFIED = every frontier2 batch) IDENTICAL 13/13 and
  `intab3-def` IDENTICAL 13/13. Master fast-forwarded to 48633fb3.
- frontier.py --no-branches: 309 → 287 to port (172 direct, 104 virtual, 11 partial) + 48 staying original.
- Play package: target/play/sa-game refreshed from this tree (audio build); web/pkg rebuilt (FEATURES=samp).
## 2026-11-16 — FRONTIER-2 turn 4 lane C: angled areas, aircraft / weather audio, heap, natives to hooks, batches c9 / c10 (agent, branch worktree-agent-a8e8e96036c1da50f)
- Base: lane C turn 3 head 77a7c29d (branch worktree-agent-a04a02581f061bd35). Every port address checked against
  `tools/gen/inflight.py` first (none in flight).
- Batch frontier2-c9 (5 hooks): CRunningScript::ObjectInAngledAreaCheckCommand 0x4883F0 and
  FlameInAngledAreaCheckCommand 0x488780 with their own area data flow (the object check keeps dx and the squared length
  on the x87 and takes len2 from the stored side vector; the flame check normalises the stored edge and side vectors in
  place for every flame-thrower shot it tests, so they are re-normalised per shot); the corner computation, the parameter
  layout, the z test and the highlight / debug tails are now shared helpers with the Char check (`corners`,
  `area_params`, `z_ok`, `highlight`; the Char data flow unchanged). The callees: CTheScripts::HighlightImportantAngledArea
  0x485EF0 (x midpoints compared as the x87 holds them, y after the f32 store; no +2 on the ground z, unlike
  HighlightImportantArea), DrawDebugAngledSquare 0x486990 (four vertical-line probes; the first probe's length is pushed
  by the SecuROM fragment 0x401B61, `[0x15606B8] - [0x6D3045]` = 0x447A0000 = 1000.0 from the exe image), the ped-list
  bounding sphere 0x66F6C0 (sa-ai). New binding CShotInfo::GetFlameThrowerShotPosn 0x739DE0; the three callee bindings
  now reach the ports.
- Batch frontier2-c10 (9 hooks): CAEVehicleAudioEntity::ProcessPlayerHeli 0x4FE420 (ProcessDummyHeli's bytes plus
  GetAccelAndBrake and the accelerator factor; both hooks share one body `heli_engine`); the last three tail-target
  natives as hooks with CAT-17 names (dtor_977be8_body 0x454200, CAEExplosionAudioEntity::~CAEExplosionAudioEntity
  0x506B90: keycells checked, only 0x506B22 inside its neighbour is read as data; CHudColours::~CHudColours 0x58FD90;
  catalogue regenerated with run_all, state guard 0 lowered); the sub-task factory 0x65A110 and its heading helper
  0x6592A0 (X87 binding `task_heading_to_ped`; the original pushes the rate / tolerance before the heading call, whose
  `ret 4` leaves them as the constructor's arguments); CMemoryHeap::FillInBlockData 0x72EC80 (protection fragment
  0x40687B = plain `cmp ecx, 0x400`) and its callee CombineFreeBlocks 0x72E870; CPatrolRoute::operator= 0x66D4B0
  (fragment 0x401764 zeroes the loop counter kept in the argument slot).
- Native (not a hook): CAEWeatherAudioEntity::AddAudioEvent 0x506800 (the thunder event; shares `thunder_volume` with
  UpdateParameters). The hook failed sa-hook's INF-20 install test: its 7-byte steal is seen by the sweep-decoded indexed
  reader 0x14274C2 in the executable `.data` (`fmul qword ptr [ebx + 0x506805]`), which keycells cannot redirect; it is a
  phase B native until that reader is shown to be junk.
- Not ported (time): ProcessMissionAudioEvent 0x4ECCF0 (6.5 KB), ProcessPlayerProp / DummyProp 0x4FD290 / 0x4FD8F0
  (137 differing lines between the two, no shared ported body; AIProp differs in 442), ProcessPlayerSeaPlane 0x4FF320,
  ExtractVehicleUpdateData 0x45BA90 / 0x45DEF0, the D3D9 raster helpers 0x4CA5A0 / 0x4CD050 / 0x4CC1D0, 0x603AF0,
  0x61D0B0 / 0x61FF40 / 0x66AB40, 0x477DA0 (CAT-12 not extended).
- Gates: writer gate (base 77a7c29d, on 603d48d8) all PASS (hooks: 14 new, all unverified, notes: c9 spans SCRIPT /
  UNK-game, c10 AUDIO / HUD / UNK-game). The first gate run caught a clippy type_complexity and the 0x506800 INF-20 read.
- Scenarios (`frontier.sh`, i686 native sa-game of 01832503, before only the 0x506800 hook→native switch; target/lc4/scen):
  pausemenu 416 / 0 blocked, solicit 4016 / 0, crossing 1216 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0,
  shootdriver 1216 / 0; crossing 4000 frames 4014 / 2 blocked (lane A's KillCriminal::CreateNextSubTask 0x68E4F0, as
  before); no crash.
- verify-all (lanes 6-7): `lanec4-c910` (SA_UNVERIFIED=frontier2-c9,frontier2-c10; DLL of e80796bb) IDENTICAL 13/13;
  `lanec4-default` (default set) IDENTICAL 13/13 (DLL of 603d48d8).
- frontier.py: lane C subsystems 36 → 26 rows (2 of them lane A's partial switches); whole frontier 285 → 263 at the
  end (other branches' registrations included).

## 2026-11-15 — FRONTIER-2 turn 4, lane B: CHeli::ProcessControl, the arrest camera, heli flying stuff (agent, branch worktree-agent-a743952821d102dbe)
- Base: worktree-agent-a75c5759aead26958 6d34a21e (turn 3 lane B, integrated in parallel). Lane B subsystems; every port from
  the 1.0 US bytes (gta-reversed only calls the original or has a stub), addresses checked against `inflight.py` (0
  overlaps). Two batches, 6 hooks + 1 native:
  - `frontier2-b12` (5 + native): CHeli::ProcessControl 0x6C7050 (priority 1, lane A's `roadblocks` police heli: gun-flash FX,
    horn search-light toggle, light target by mission, the one-second target history and its blend, intensity by distance,
    the police heli's shots by wanted level with the line-of-sight check and FireOneInstantHitRound, swat ropes, winch,
    weapons, the interesting event), CHeli::SendDownSwat 0x6C69C0, CRopes::SetSpeedOfTopNode 0x555DF0 (sa-render), the phase B
    native of the unhookable generated CHeli::FindSwatPositionRelativeToHeli 0x6C4760 (callers keep EDX);
    CCam::ProcessArrestCamOne 0x518500 (coordinator priority: `roadblocks` blocked 1135 steps on it with lane A's arrest
    tasks; ported from the bytes with docs/re/specs/CAM/CCam.md: reset mode search with the 0x512EF0 / 0x515D80 / 0x516010 /
    lamp-post helpers, running modes 1 / 2 / 4..7 / 8, KEEP / APPLY tails; the reset tail reads `out` uninitialised when no
    mode was found: hazard uninit-locals, zeros here); CHeli::ProcessFlyingCarStuff 0x6C4E60 (the new `heli` scenario blocked
    every step on it: rotor run-down / spin-up, FlyingControl model 2 / 6, blade collision, wind modifier, chop sound);
  - `frontier2-b13` (1): CMonsterTruck::PreRender 0x6C7DE0.
- Bindings: 0x73AF00 CWeapon::FireOneInstantHitRound bound in sa-veh (sa-script's private declaration removed); air2's
  ProcessWeapons and camproc's ArrestCamOne dispatch target the ports; mvpb_a1's FlyingControl declaration shared.
- Scenarios: frontier.sh `heli` (Hunter cheat OHDUDE) and `plane` (stunt plane cheat URKQSRK) added. A `boat` scenario
  (teleport to the Santa Maria pier, 836, -2060) hung at 100 % CPU right after the teleport (frame 109, no output; ptrace not
  permitted to get a stack) and was left out: open.
- frontier.py, lane B subsystems: 146 → 142 rows (0x6C7050, 0x518500, 0x6C4E60, 0x6C7DE0); no new rows revealed.
- Scenarios (`frontier.sh`, i686 native sa-game of this branch at f0448758, default lengths): pausemenu 416 / 0 blocked,
  solicit 4016 / 0, crossing 1216 / 0, crossing 4000 4014 / 2 (lane A's KillCriminal::CreateNextSubTask 0x68E4F0, as
  before), melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0, quad 1216 / 0, heli (new) 1216 / 0,
  plane (new) 210 / 1006 blocked on lane C's audio CAEPropellerAudioEntity path 0x4FD8F0; no crash. Lane A's `roadblocks`
  (definition of 7f81981d copied locally, not committed; lane A's arrest tasks not in this tree): 254 / 1762 blocked, now on
  CVehicle::FlyingControl 0x6D85F0 (1758, the police heli) and lane A's GenerateRoadBlocks fallback (4).
- Writer gate (base 6d34a21e): first run all PASS but dupes (copies of `v3_ptr` / `time_ms`, fixed in dc4f0ce6); final run
  on 05aa729d: hooks (6) / queue / bindings / state / dupes / cfg / layers / commits / clippy / i686 / i686clip / wasm32 /
  hooktest / l1test / l1 all PASS, no step skipped.
- verify-all (lanes 4-5, fresh release sa-oracle, DLL of 05aa729d): `lb4-u` (SA_UNVERIFIED=frontier2-b12,frontier2-b13)
  IDENTICAL 13/13; default set `lb4-def` IDENTICAL 13/13.
- Open (lane B): CVehicle::FlyingControl 0x6D85F0 (now the `roadblocks` blocker; ~10 KB body behind a `nop; jmp` protection
  entry, continues at 0x6D85F6), ProcessBoatControl 0x6DBCE0, CBoat::ProcessControl, CPlane::PreRender /
  ProcessFlyingCarStuff, a boat scenario (the pier teleport hang), CCamera::IsItTimeForNewcam / TryToStartNewCamMode, item 4
  (Interior_c::PlaceObject chain, replay playback, RW leftovers, WEAPON bodies incl. FireOneInstantHitRound 0x73AF00 now
  bound in sa-veh).

## 2026-11-16 — FRONTIER-2 turn 4, lane A: road blocks between two points, gang wars, allocators, AI frontier (agent, branch worktree-agent-a198f2e01294e8cc9)
- Base: worktree-agent-a79dbccbc537c6d52 (7f81981d, lane A turn 3). Every port from the 1.0 US bytes (gta-reversed read as an
  aid; where it differs the bytes win), every address checked against `inflight.py` first. One batch, `frontier2-a7` (16
  hooks: 15 new + CGangWars::Update made whole; spans AI and POP).
- CRoadBlocks::CreateRoadBlockBetween2Points 0x4619C0 (sa-pop road_block_between.rs, ~1,000 instructions): split into
  helpers that follow the original's blocks (line set-up / vehicle model / slot count / one slot / cones); the SEH-guarded
  stack CMatrix is modelled as the original makes it (the inlined constructor's two cleared fields, the destructor 0x59ACD0
  on every exit, including the riot-car -1 and cop-bike exits). GenerateRoadBlocks now reaches it. Hazard uninit-locals:
  the first slot's RotateZ rotates the matrix's uninitialised position (overwritten right after).
- CGangWars::Update 0x446610: whole port (the ADR-002 fallback removed; partial ports 11 → 10): provocation timer, the
  offensive state machine (waves 0..2, GW_CLR1/2, victory), the presence test (GW_WARN; GW_FLEE queued with GW_WARN in the
  brief array, as the bytes), the 30000 cap, the defensive war (GW_WON / GW_SLOW / GW_NOSH, stat 0x44), the blinking blip
  (RadarBlip read through the protection's obfuscated pointer at 0x406434, decoded to 0x96AB98) and the gang streaming. No
  scenario starts a war: bGangWarsActive is set by the story script (no cheat or start-in-world path reaches it); the
  inactive path runs every frame (796 calls in roadblocks under gdb).
- Allocators: CTaskAllocatorKillOnFoot::AllocateTasks 0x69BCA0 (flag byte 0xC1A554 and SetTask for the second list: both
  differ from gta-reversed), CTaskAllocatorPlayerCommandAttack::AllocateTasks 0x69C4D0 (a negative ped type defers to
  CTaskAllocatorAttack; SetTask), CTaskAllocatorKillThreatsDriveby::AllocateTasks 0x69CC30; the CPedList helpers Empty
  0x699DB0 and BuildListOfPedsOfPedType 0x69A3B0; the CFormation calls they reach: FindCoverPoints 0x69B860,
  DistributeDestinations_CoverPoints 0x69B5B0, DistributeDestinations_PedsToAttack 0x69B700 (the best pair is not reset and
  starts from an uninitialised slot). The lone-threat sequence / event-response helpers are shared with 0x69D460.
- EndConversation 0x43AB10: new tool `tools/re/vm_eval.py` (unicorn on the exe image; the `.venv`'s unicorn, else
  `SA_UNICORN_PATH` / `target/pyunicorn`, see its docs) evaluates a 0xCB73E0 call site: `vm_eval.py 1566FBB 156704F --call`
  gives the partner pointer `~[0x1560530] ^ [0x72D9F4]` = 0x9691C0 (read again), the blend time 0xFA and the computed call
  IKChainManager_c::AbortLookAt 0x618280.
- CPedShelterAttractor::GetDisplacement 0x5EF420 (FCOS / FSIN) with the vector push_back 0x5EE370 (FUN_005ee370; it calls
  the unported UNK-game fill 0x5EB340, lane C). Then AI frontier entries: CTaskSimpleJetPack::MakeAbortable 0x67E6E0,
  CTaskComplexTrackEntity::CreateNextSubTask 0x65F590, CPedIntelligence::TestForStealthKill 0x601E00.
- Bindings (ADR-003): 0x5A1980, 0x583D20, 0x5A1B60, 0x4E2A70 (sa-audio twin) bound in sa-world; 0x699F00, 0x699FF0, 0x65EEB0
  bound in sa-ai; 0x6C26F0 moved down to sa-pop (sa-veh twin); the local declarations replaced by imports. No new `st0()`
  wrappers on 0x53CBE0 (the new `st0()` uses wrap f64 vtable / FindGroundZFor3DCoord results).
- frontier.py: AI 103 (12 direct + 91 virtual) → 95 (8 + 87); total to port 319 → 310; partial 11 → 10. New rows outside
  lane A: 0x5EB340 (UNK-game, lane C), 0x6A27F0 CAutomobile::SetTotalDamage (VEH, lane B).
- Evidence: port-l2 police-gang 0x4619C0 (split + rust IDENTICAL over 5200 frames, NOCASES); the allocators and CPedList
  helpers on police-gang / weapon-combat (l2 + rust IDENTICAL, NOCALLS). The corpus calls none of the war / allocator /
  road-block paths; the roadblocks scenario does not create a block in its 2000 frames (gdb: 0 calls of 0x4619C0; new
  tools/e2e/scenarios/frontier2-a4-counts.gdb).
- Scenarios (frontier.sh, i686 native, final tree): pausemenu 416 / 0 blocked, solicit 4016 / 0, crossing 1216 / 0,
  crossing-long 4016 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0, roadblocks 875 / 1141
  blocked only on lane B (CCam::ArrestCamOne 0x518500 1135, CHeli::GenerateHeli 0x6C6520 6); no crash.
- verify-all (lanes 1-3, fresh sa-oracle): `f2a4-u2` (SA_UNVERIFIED=frontier2-a7) IDENTICAL 13/13 and `f2a4-def` (default
  set; CGangWars::Update is now in frontier2-a7, so the default set runs the original there) IDENTICAL 13/13; save-load
  identical except the SAVE-1 stack residue. Both ran on the DLL built before the hook-name-only fix 694d7c3e (0x5EE370's
  name string; no code change).
- Writer gate (base 7f81981d, target dir target/gate): all 16 steps PASS (hooks 15 new, all unverified, batch frontier2-a7;
  queue up to date). The first run failed hooktest: 0x5EE370 was named `std::vector<CVector>::push_back`, the catalogue has
  FUN_005ee370 (fixed).

## 2026-11-16 — FRONTIER-2 turn 5, lane A: road block and gang war scenarios, AI frontier (agent, branch worktree-agent-a84f43d95c6a883c2)
- Base: worktree-agent-a198f2e01294e8cc9 (728a6fa5, lane A turn 4). Every port from the 1.0 US bytes (gta-reversed read as an
  aid), every address checked against `inflight.py` first. One batch, `frontier2-a8` (36 hooks, all AI).
- 0x53CBE0's binding returns X87 as on master (770ffde5 takes master's call-site lines verbatim, so the rebase merges
  them); the new ports use the X87 value directly (`.to_f32()`), no `st0()` around it.
- Scenarios: sa-game gains `SA_GAME_POKE=ITER:ADDR:VALUE[:SIZE]` (ADDR may be `[0xP]+0xOFF`). New frontier.sh scenarios
  `roadblock-script` (a script road block poked into aScriptRoadBlocks[0]: CreateRoadBlockBetween2Points 0x4619C0 runs
  once), `roadblock-wanted` (the roadblocks car with four stars teleported to Idlewood's east-west road, where five
  ROADBLOX.DAT nodes come within 90 m, and the CWanted +0x1C chance poked to 127 every 10 iterations: 9 road blocks), `gangwar`
  (Ballas Idlewood, gang wars on 0x96AB90 and the provocation timer 0x96AB5C poked to 3.0: StartOffensiveGangWar,
  StartDefensiveGangWar, PickZoneToAttack and three CreateAttackWave calls). frontier2-a5-counts.gdb counts them on the
  Rust functions (ports reach each other directly, not through their native entries; turn 4's "0 calls" count of 0x4619C0
  watched the native entry only).
- Ports (batch frontier2-a8): CTaskComplexFollowLeaderInFormation::CreateSubTask 0x6962A0 (catalogue name
  thunk_FUN_00401913; the `nop; jmp` island only fetches the SEH link), CTaskSimpleIKChain::CreateIKChain 0x633BD0 / Clone
  0x633B00, CPedShelterAttractor::BroadcastDeparture 0x5EF570 (shares the attracted-peds tail with 0x5EF160),
  SeekCoverUntilTargetDead::CreateFirstSubTask 0x62E6C0 + GetCoverPed 0x622D50, CTaskSimpleWaitUntilPedIsOutCar::ProcessPed
  0x649550 (roadblock-wanted's block), GoToPointAiming::CreateFirstSubTask 0x66DDB0, BeInShop::ScanForStuff 0x6754D0, the
  direct entries 0x634740 (ComputeClosestFreeScriptedEffect), 0x6796C0 (FallToDeath::CalcFall, rand at both sites) and
  0x689640 (Swim::ProcessControlAI, FPATAN), the Clone virtuals 0x622F20 / 0x623030 / 0x66CD80 / 0x66CE10 / 0x634560 /
  0x65F4E0, the interior tasks 0x6750A0 / 0x6762B0 / 0x676200 / 0x676350 / 0x675740 / 0x676550, 0x678AA0, 0x6960B0,
  0x696160, 0x691320, 0x62FF70, 0x6390C0, 0x5F6DA0, 0x635110, 0x6584E0, 0x6477F0, 0x634C10, 0x679120.
- Bindings (bindings.py --resolve, rows placed without reordering the CSV): 0x493730, 0x4ADDF0, 0x5F1890, 0x61F660,
  0x65F3B0 bound in sa-ai; 0x634440 / 0x668790 moved down from sa-script.
- Not ported: 0x5ED750 (std::vector<CVector>::_Insert_n: mis-split body continuing past 0x5ED92A, try/catch, its callees
  0x5EB340 / 0x5EAEA0 / 0x5ECC30 are lane C UNK-game), PlaceFormationGroup 0x5FC9B0 / PlaceChatGroup 0x5FCE80 /
  RenderJetPack 0x67F6A0 (big, unreached), 0x634930 / 0x670D80 / 0x679270 (callees 0x6348F0 / 0x670B00 / 0x670A70 /
  0x630590 not in the catalogue), Sunbathe::ControlSubTask 0x6381A0 (needs ShouldLoadSunbatheAnims 0x632190);
  CCollision::ProcessColModels' A-disk path (not started: the loader clears the disk flag, nothing reaches it).
- frontier.py: AI 95 (8 direct + 87 virtual) → 61 (4 + 57); to port 301 → 266 (all lanes, this checkout).
- Scenarios (frontier.sh, i686 native, final ports, SA_ROOT=~/sa-engine): pausemenu 416 / 0 blocked, solicit 4016 / 0,
  crossing 1216 / 0, crossing-long 4016 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0,
  roadblock-script 1016 / 0, roadblocks 875 / 1141 (lane B: CCam::ArrestCamOne 0x518500, CHeli::GenerateHeli 0x6C6520),
  roadblock-wanted 1015 / 1001 (lane B: 0x518500, CWorld::FindObjectsIntersectingCube 0x568DD0, 0x6C6520), gangwar 2015 / 1
  (lane B: CGangWars::TellGangMembersTo 0x444530), quad (master's frontier.sh) 1215 / 1 (CCheat::VehicleCheat 0x43A0B0, in
  flight on a lane B worktree); 0x568DD0 and 0x6C6520 are on master already; no crash.
- verify-all (lanes 1-3, fresh sa-oracle / satool, DLL of 29609990): `f2a5-u` (SA_UNVERIFIED=frontier2-a8) IDENTICAL 13/13
  and `f2a5-def` (default set) IDENTICAL 13/13; save-load identical except the SAVE-1 stack residue. a9a354c7 after it
  only renames 0x6962A0's hook to the catalogue name and replaces a local zero test by sa_math::fpu::fcom_ne (no
  behaviour change).
- Writer gate (base 728a6fa5, target dir target/gate): all 15 steps PASS (hooks 36 new, all unverified, batch frontier2-a8;
  queue up to date). The first run failed dupes (a local copy of an existing zero test), clippy (collapsible match) and
  hooktest (0x6962A0 is catalogued as thunk_FUN_00401913): fixed in a9a354c7.
## 2026-11-16 — FRONTIER-2 turn 5 lane C: aircraft prop / sea plane audio, remote cars, oct tree, batch c11 (agent, branch worktree-agent-acc988eb6f9ea226b)
- Base: lane C turn 4 head 47bd4a75 (branch worktree-agent-a8e8e96036c1da50f). Every port address checked against
  `tools/gen/inflight.py` first (none in flight).
- Batch frontier2-c11 (12 hooks):
  - CAEVehicleAudioEntity::ProcessPlayerSeaPlane 0x4FF320 (engine factor from +0x5A4, clamped / floored at the double
    0x862D08, the player's accelerator factor, the camera-facing volume split); ProcessPlayerProp 0x4FD290 and
    ProcessDummyProp 0x4FD8F0: they differ only before the camera part (the dummy derives and stores the pedals, the
    player calls GetAccelAndBrake) and in one extra first entry of the player's stall table (+0xB4 > 0 with vehicle
    +0x960 clear), so both share `prop_engine` (landing-gear stall factor / volume, the +0xB8 / +0xBC random stall
    timer). The shared distance-volume helper now compares the unrounded square root against 128 (the originals do
    `fst` then `fcomp`), and the verified ProcessAIProp uses the shared smoothing / distance helpers instead of its
    inline copy (same bytes; affects only a root within half an ulp of 128).
  - CTheScripts::AddToBuildingSwapArray 0x481140 (.HOODLUM body 0x1569840); CStoredCar::StoreCar 0x449760.
  - CRemote::GivePlayerRemoteControlledCar 0x45AB10 and TakeRemoteControlOfCar 0x45AD40 (.HOODLUM 0x156E620) with
    shared remote-car state / player-vehicle / camera helpers. The jump-cut switch the no-CD VM 0xCB73E0 pushes
    (descriptor 0x1560094) evaluates to 1 on the exe image (unicorn; the VM's `push ss; pop ss` anti-trace pair is
    skipped); a different previous remote car gives 2.
  - COctTree::InsertTree 0x5A75B0 and its callees CPool<COctTree>::New 0x5A7350 (generic pool New), the constructor
    0x5A6DB0 and RemoveChildren 0x5A74F0.
  - CAEPedlessSpeechAudioEntity::PlayLoadedSound 0x4E6380 (virtual).
- Bindings: new 0x5A6DB0 / 0x5A7350 / 0x5A74F0 / 0x5A7490 (sa-core), 0x45AB10 / 0x45AD40 / 0x4C5B00 / 0x4C5B30 /
  0x50C1B0 (sa-script; the cmd800 / cmd1100 / cmd1800 / cmd2000 local declarations removed), 0x449760 now reaches the
  port (no result); 0x481140's cmd900 declaration reaches the port.
- Not ported (time): ProcessMissionAudioEvent 0x4ECCF0, ExtractVehicleUpdateData 0x45BA90 / 0x45DEF0, the D3D9 raster
  helpers 0x4CA5A0 / 0x4CD050 / 0x4CC1D0, 0x603AF0, 0x61D0B0 / 0x61FF40 / 0x66AB40, ScriptTaskPickUpObject 0x46AF50
  (three obfuscated pushes and a call/ret trampoline), 0x6002F0, AddSayEvent 0x4E60D0 (pedless variant of the ported
  ped one, own callees), LoadUserTrack 0x4F35F0, 0x477DA0 (CAT-12 not extended), the 0x14274C2 junk proof.
- Gates: writer gate (base 47bd4a75, on ce6322a3) all PASS (hooks: 12 new, all unverified, batch frontier2-c11; note: the
  batch spans AUDIO / CALLED-ONLY / SCRIPT / UNK-game). commit_lint caught one 73-character subject (reworded).
- Scenarios (`frontier.sh`, i686 native sa-game of ce6322a3; target/lc5/scen): pausemenu 416 / 0 blocked, solicit
  4016 / 0, crossing 1216 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0; crossing 4000
  frames 4014 / 2 blocked (lane A's KillCriminal::CreateNextSubTask 0x68E4F0, as before); quad (master's frontier.sh,
  not yet on this branch) 1215 / 1 blocked on CCheat::VehicleCheat 0x43A0B0 (not lane C; registered on another branch);
  no crash.
- verify-all (lanes 6-7): `lanec5-c11` (SA_UNVERIFIED=frontier2-c11; DLL of ce6322a3) IDENTICAL 13/13; `lanec5-default` (default set) IDENTICAL 13/13.
- frontier.py: lane C subsystems 26 → 17 rows (2 of them lane A's partial switches; InsertTree's three callees came
  and went); whole frontier 263 → 251 (other branches' registrations included).
## 2026-11-16 — FRONTIER-2 turn 5, lane B: CVehicle::FlyingControl (agent, branch worktree-agent-a39ff8008721099ec)
- Base: worktree-agent-a743952821d102dbe 7f557d6e (turn 4 lane B). `inflight.py` checked: no other branch registers 0x6D85F0.
- Batch `frontier2-b14` (1 hook): CVehicle::FlyingControl 0x6D85F0 (`crates/sa-veh/src/flying_control.rs`), the `roadblocks`
  blocker. The body is ~2,900 instructions behind the `nop; jmp 0x403CE8` protection entry (the fragment does `sub esp, 0x250`
  and jumps back to 0x6D85F6). Ported from the 1.0 US bytes, with gta-reversed as a reading aid only: its operand orders,
  several constants (`mass * ts * 0.0072` for the dodo's ceiling lift; the ceiling handled per model) and the Hydra
  automobile check differ from the bytes. Split into the jump-table blocks 0x6DAEE4 / 0x6DAEF4: `dodo` (model 0), `plane`
  (1 / 3 / 4 / 5: stick-to-square 1 / cos, ground / water / air thrust kept in the accel argument's slot, the Hydra nozzle,
  side slip, yaw, roll, roll stabiliser, pitch, stall counter, lift with HeightAboveCeiling called up to three times), `heli`
  (2 / 6 / 8: move damping, autogyro pusher without the 0.008 scale and rotor spin, heli thrust with the minimum-tilt
  asin / acos quadrupling, hover force, level-seeking roll / pitch, upside-down recovery, pad incl. the heli-controls cheat,
  pitch / roll / side / yaw) and the shared `resistance` tail (turn damping, speed-resistance counter torques, the 1.5 / 0.2
  speed clamps). Shared helpers for the original's interleaved x87 store patterns (`chain3`, `side_force`, `clamp_tilt`);
  `_CIpow` / `_CIasin` / `_CIacos` via sa_core::orig::x87_* (reusing sa-world `ci_pow` and sa-veh `ci_asin`), HeightAboveCeiling
  through `reg("args = ecx stack stack; pop")` (an `X87` result needs the reg form in phase A). mvpb_a1's FlyingControl
  declaration now targets the port (CHeli / CPlane ProcessFlyingCarStuff reach it).
- frontier.py, lane B subsystems: 142 → 141 rows (0x6D85F0); no new rows revealed.
- Scenarios (`frontier.sh`, i686 native sa-game of this branch, default lengths): pausemenu 416 / 0 blocked, solicit 4016 / 0,
  crossing 1216 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0, quad 1216 / 0, heli 1216 / 0,
  plane 210 / 1006 (lane C's CAEPropellerAudioEntity path 0x4FD8F0, unchanged); no crash. Lane A's `roadblocks` (definition of
  7f81981d, run locally, not committed): 254 / 1762 blocked, no longer on FlyingControl: lane A's CTaskSimpleGangDriveBy
  MakeAbortable 0x62D290 (885) / ProcessPed 0x62D3B0 (37), CTaskComplexCopInCar::CreateSubTask 0x68C9E0 (2),
  CTaskAllocatorKillThreatsBasicRandomGroup::AllocateTasks 0x69D460 (1), the GenerateRoadBlocks fallback (61), and lane C's
  CAEPedlessSpeechAudioEntity::AddSayEvent 0x4E60D0 (776). 0 blocked on lane B functions.
- Writer gate (base 7f557d6e, `GATE_SKIP=i686` while verify-all used the DLL): hooks / bindings / state / dupes / cfg / layers /
  commits / clippy / wasm32 / hooktest / l1test / l1 PASS; queue FAIL on the first run (the amended commit's hash), regenerated,
  `queue.py --check` up to date; full rerun on e9d663c2 (no step skipped): all 16 steps PASS.
- verify-all (lanes 4-5, fresh release sa-oracle, DLL of e33f4a8d): `lb5-u` (SA_UNVERIFIED=frontier2-b14) IDENTICAL 13/13; default set `lb5-def` IDENTICAL 13/13.
- Not done (time): the boat scenario and ports (ProcessBoatControl 0x6DBCE0, CBoat::ProcessControl 0x6F1770), CPlane::PreRender
  0x6C94A0 / ProcessFlyingCarStuff 0x6CB7C0 (docs/re/specs/VEH/CPlane.md has a full spec), the CCamera / CCam bodies, item 4.

## 2026-11-16 — FRONTIER-2 turn 6, lane B: CPlane flight / pre-render, CBoat::ProcessControl, the pier hang (agent, branch worktree-agent-a10f9d8e814fb0ac6)
- Base: worktree-agent-a39ff8008721099ec 93b96b88 (turn 5 lane B). `inflight.py` checked before each port: no other branch
  registers 0x6CB7C0, 0x6C94A0, 0x6F1770.
- Batch `frontier2-b15` (3 hooks), all from the 1.0 US bytes:
  - CPlane::ProcessFlyingCarStuff 0x6CB7C0 (`crates/sa-veh/src/plane_flying.rs`): damage wave, speed factor, the low-health
    propeller loss and control skew, per-component damage (propellers: throttle by a sine of the wave; rudder / elevators /
    ailerons with their bouncing panels) and smoke puffs, propeller speed toward the throttle target with the ceiling cut,
    FlyingControl (RC Baron 1, Hydra hover 6 with the Hunter's flying handling, 3). docs/re/specs/VEH/CPlane.md is wrong on
    one point: the Hydra hover call passes `-leftRight` (0x6CC43F reads the cLR slot), not `-upDown`.
  - CPlane::PreRender 0x6C94A0 (same file): suspension with the Andromada wheel tilt, wheel matrices, propeller angle, the
    control surfaces through the stack CMatrix / CQuaternion (aileron stretch with the gear out), propeller frames, landing
    gear by model (jump table 0x6CAAE0 / byte table 0x6CAAF8 decoded), Andromada door, Hydra nozzles / jet thrust / dust,
    Vortex skirt, panels and force copies, shadow, wing-tip streaks, smoke ejector, Skimmer splashes.
  - CBoat::ProcessControl 0x6F1770 (`crates/sa-veh/src/boat_control.rs`): gta-reversed differs from the bytes (heading
    marker -9999.99 not -10000, the fire timer adds whole milliseconds `ftol(ts * 0.02 * 1000)`, the fire offset's z + 1.0,
    statuses 2 / 3 are the AI cases).
- Shared instead of declared again (ADR-003): new bindings CMotionBlurStreaks::RegisterStreak 0x721DC0 (sa-world, twin in
  sa-render; entity_prerender.rs's private declaration removed) and FindPlayerCentreOfWorld_NoSniperShift 0x56E320
  (sa-world; sa-hud menu_map.rs's removed); within sa-veh the private declarations of HeightAboveCeiling / pos_z,
  Get/SetAeroplaneCompStatus (now targeting the damage.rs ports), DoHeliDustEffect, the vtable 0xD0 slot, CQuaternion::Set,
  ProcessPanel, DoBoatSplashes and ProcessBoatControl are pub(crate) and reused; sa-world break_object's `rand_unit` is public.
- The Santa Maria pier hang (turn 4): found with gdb on the native i686 sa-game (run under gdb, SIGINT to the child): the
  main thread looped in CAEStreamingChannel's fill -> `vorbis_set_cursor` -> sa-game's `ov_time_seek`, which re-opened
  the stream for a seek to 0.001 s (granule 24, inside the first audio page); lewton's page seek left the reader at its end
  with no position, the channel seeked again, forever (93 500 seeks in 60 s with SA_GAME_VORBIS_TRACE). Fixed in
  `crates/sa-game/src/vorbisfile.rs` (ece2d244): without a page end after the page seek the stream decodes from the start
  and drops up to the target. The pier teleport now runs (300 frames, 0 blocked), but no boat spawns there.
- New scenario `boat` (frontier.sh): the script's boat car generators are at the Bayside marina (CarGeneratorArray 0xC27AD0
  dumped with SA_GAME_DUMP: Marquis / Squalo / Jetmax at -2227, 2446); CJ is teleported onto the dock next to them.
- frontier.py, lane B subsystems: 140 → 137 rows (0x6CB7C0, 0x6C94A0, 0x6F1770); no new rows revealed.
- Scenarios (i686 native sa-game of 595aae66, SA_ROOT=~/sa-engine; master's frontier.sh for its names, this branch's for heli /
  plane / boat): pausemenu 416 / 0 blocked, solicit 4016 / 0, crossing 1216 / 0, crossing-long 4014 / 2 (lane A's
  KillCriminal::CreateNextSubTask 0x68E4F0), roadblocks 254 / 1762 (lane A: GangDriveBy MakeAbortable 0x62D290 885 /
  ProcessPed 0x62D3B0 37, CopInCar 0x68C9E0 2, KillThreats 0x69D460 1, GenerateRoadBlocks fallback 61; lane C: AddSayEvent
  0x4E60D0 776), melee 2616 / 0, sniper / shootcar / shootdriver / quad / heli 1216 / 0, plane 210 / 1006 (lane C's
  CAEPropellerAudioEntity 0x4FD8F0; with lane C's branch merged in a scratch tree, not committed: 386 / 830, lane C's
  UNK-game vector ctor 0x43E0E0 826, lane A's 0x658DB0 3 / 0x6375F0 1: 0 on lane B), boat 199 / 1017 on
  CVehicle::ProcessBoatControl 0x6DBCE0 (lane B, not ported this turn); no crash.
- Writer gate (base 93b96b88, on 79862a78, no step skipped): hooks (3) / queue / bindings / state / dupes / cfg / layers /
  commits / clippy / i686 / i686clip / wasm32 / hooktest / l1test / l1 all PASS.
- verify-all (lanes 4-5, fresh release sa-oracle, DLL of c9f56a9e): `lb6-u` (SA_UNVERIFIED=frontier2-b15) IDENTICAL 13/13;
  default set `lb6-def` IDENTICAL 13/13.
- Open (lane B): CVehicle::ProcessBoatControl 0x6DBCE0 (the `boat` blocker; 5.2 KB with CVector operator calls, gta-reversed
  has a reading), DoBoatSplashes 0x6DD130 (CPlane::PreRender / CBoat::PreRender reach it), the CCamera / CCam bodies
  (IsItTimeForNewcam 0x51D770, TryToStartNewCamMode 0x51E560, Get_TwoPlayer_AimVector 0x513E40, DW_HeliChaseCam 0x51A740,
  DW_BirdyCam 0x51B850), item 4 (FireOneInstantHitRound 0x73AF00, Interior_c::PlaceObject 0x5934E0, GeneratePanesForWindow
  0x71B620, SetTotalDamage 0x6A27F0, the RW leftovers).

## 2026-11-16 — FRONTIER-2 turn 7, lane B: CVehicle::ProcessBoatControl, DoBoatSplashes, CMatrix::ScaleAll, SetTotalDamage (agent, branch worktree-agent-aa54999b4e7af5432)
- Base: worktree-agent-a10f9d8e814fb0ac6 904c32ea (turn 6 lane B). `inflight.py` checked: no other branch registers 0x6DBCE0,
  0x6DD130, 0x45AF40, 0x6A27F0.
- Batch `frontier2-b16` (VEH, 3 hooks), from the 1.0 US bytes, gta-reversed as a reading aid only:
  - CVehicle::ProcessBoatControl 0x6DBCE0 (`crates/sa-veh/src/boat_physics.rs`, the `boat` blocker): buoyancy, drowning,
    the Skimmer's take-off, aquaplane lift, wake point, traction, thrust point depth, steered thrust / side traction /
    steering drag, keel side force, handbrake drag, water resistance, turn resistance, waves. gta-reversed differs: the
    drowning flag +0x42B bit 6 is not cleared while the hull drowns; the Skimmer timer resets only when the time is strictly
    past it; the thrust depth is clamped by comparing the depth (not its square) with 1; a negative post-collision z force
    becomes 0; the wave strength is `(int16)_ftol`; the automobile case is `+0x590 == 0` storing `|force|` at +0x950.
  - CVehicle::DoBoatSplashes 0x6DD130 (same file): both splashes are thrown back along `-forward` (gta-reversed reads
    `-right.y, -right.z`), the size is `min((s * 10 + 0.75) * 0.1, 1)`.
  - CAutomobile::SetTotalDamage 0x6A27F0 (`automobile_random_damage.rs`, sharing SetRandomDamage's helpers; reached by
    CRoadBlocks::GenerateRoadBlockCopsForCar).
- Batch `frontier2-b17` (MATH, 1 hook): CMatrix::ScaleAll 0x45AF40 (sa-core cmatrix.rs; `lea eax, [ecx+8]; jmp 0x401BA5`, the
  fragment sets the row counter 4 and jumps to the loop 0x45AF48), reached by lane C's replay vehicle code.
- ADR-003: the drowning-driver event of CAutomobile::ProcessBuoyancy is now `mvpd_v5::drown_occupant` (used by both);
  automobile_render.rs' GetLookingForwardFirstPerson declaration is pub(crate) and reused; `cv` / `rd3f` imported.
- Priority 2 of the brief: CCam::ArrestCamOne 0x518500, CHeli::GenerateHeli 0x6C6520 and CWorld::FindObjectsIntersectingCube
  0x568DD0 are already ported on master and this branch (turns 2-4); lane A's tree (144ab2bd, base d05f6e24) lacks them, so
  they block there only until lane B is integrated. With lane A's `roadblocks` / `roadblock-wanted` definitions run on this
  tree, neither blocks on lane B.
- frontier.py, lane B subsystems: 137 → 135 rows (0x6DBCE0, 0x6DD130; 0x45AF40 and 0x6A27F0 are frontier rows only on the
  lanes that call them); no new rows revealed.
- Scenarios (i686 native sa-game of 023bbb6b, SA_ROOT=~/sa-engine; lane A's definitions from 144ab2bd for roadblocks /
  roadblock-script / roadblock-wanted / gangwar / jetpack): pausemenu 416 / 0 blocked, solicit 4016 / 0, crossing 1216 / 0,
  crossing-long 4014 / 2 (lane A 0x68E4F0), melee 2616 / 0, sniper / shootcar / shootdriver / quad / heli 1216 / 0, plane
  210 / 1006 (lane C 0x4FD8F0), **boat 1216 / 0** (was 199 / 1017), roadblock-script 1016 / 0, roadblocks 254 / 1762 (lane A
  0x62D290 885 / 0x62D3B0 37 / 0x68C9E0 2 / 0x69D460 1 / GenerateRoadBlocks fallback 61; lane C 0x4E60D0 776),
  roadblock-wanted 250 / 1766 (lane C 0x4E60D0 1585; lane A fallback 173, 0x68C9E0 7, 0x68B990 1), gangwar 2015 / 1 (lane A
  0x68E4F0), jetpack 1015 / 1 (lane A 0x67B4E0); no crash; 0 blocked on lane B everywhere.
- Writer gate (base 904c32ea): first run on 72c47906 with `GATE_SKIP=i686` (verify-all was using the DLL): hooks (4) / queue /
  bindings / state / dupes / cfg / layers / commits / clippy / wasm32 / hooktest / l1test / l1 PASS; full rerun on the final
  head, no step skipped: all PASS.
- verify-all (lanes 4-5, fresh release sa-oracle, DLL of 023bbb6b): `lb7-u` (SA_UNVERIFIED=frontier2-b16,frontier2-b17)
  IDENTICAL 13/13; default set `lb7-def` IDENTICAL 13/13.
- Open (lane B): the CCamera / CCam bodies (IsItTimeForNewcam 0x51D770 and TryToStartNewCamMode 0x51E560, specified in
  docs/re/specs/CAM/CCamera.md, ~1,000 / ~800 instructions; Get_TwoPlayer_AimVector 0x513E40, DW_HeliChaseCam 0x51A740,
  DW_BirdyCam 0x51B850), FireOneInstantHitRound 0x73AF00 (its entry runs through the VM-style fragment 0x4015A0: needs
  vm_eval.py), Interior_c::PlaceObject 0x5934E0 chain, GeneratePanesForWindow 0x71B620, the RW leftovers.
## 2026-11-16 — FRONTIER-2 turn 6, lane A: TellGangMembersTo, the AI frontier, vector<CVector>::_Insert_n, jet pack render (agent, branch worktree-agent-aa16f3bdf26614476)
- Base: worktree-agent-a84f43d95c6a883c2 (235b6b1a, lane A turn 5). Every port from the 1.0 US bytes (gta-reversed read as an
  aid; where it differs the bytes win), every address checked against `inflight.py` first (none of them registered
  elsewhere). One batch, `frontier2-a9` (28 hooks: AI, plus POP 0x444530 and the UNK-game vector helpers).
- CGangWars::TellGangMembersTo 0x444530 (sa-pop gang_war_tell.rs; `.HOODLUM` body 0x1568E00): the ped pool from the last
  slot; EBX = -1 out of the no-CD VM 0xCB73E0 (vm_eval.py 1568E37 1568E4F); in-car gang peds get only the car task
  (gta-reversed also gives them the wander task); argument values other than 0 / 1 do nothing. Bindings 0x66F5C0 /
  0x6D1C40 moved down to sa-pop. `gangwar` now 0 blocked (frontier2-a5-counts.gdb: 1 call).
- CAT-12 item: 0x6348F0, 0x670B00, 0x670A70 and 0x630590 were already catalogue procedures (UNK-game / AI rows since the
  first catalogue; `callgraph.py gaps` has nothing to add and needs the main checkout's `catalog/raw`), so no
  extra-code-addresses.csv row and no state change; turn 5's "not in the catalogue" was wrong. Ported: FUN_006348f0
  0x6348F0 + UseMobilePhone::MakeAbortable 0x634930 / ControlSubTask 0x634D60, the patrol route task (route reverse
  0x66D550, next type 0x670A70, CreateSubTask 0x670B00, CreateNextSubTask 0x670D80), FallToDeath::CreateNextSubTask
  0x679270 / ControlSubTask 0x679510 (the |z| tests follow the bytes, gta-reversed inverts them); 0x630590 (ported,
  ai7-10) bound in sa-ai.
- ShouldLoadSunbatheAnims 0x632190 (nop; nop; jmp island 0x402274) and Sunbathe::ControlSubTask 0x6381A0; CanSunbathe
  0x632140 bound (sa-pop row, sa-ai port).
- 0x5ED750 vector<CVector>::_Insert_n with 0x5EB340 / 0x5EAEA0 / 0x5EB9D0 / 0x5EC140 / 0x5EB220 / 0x5EACC0 / 0x5EB1E0 (lane
  C's UNK-game callees, unregistered everywhere, so taken): attractor_ptr_vectors.rs's shared insert_n / fill / copy now
  take the element width (one body for the four instantiations); the CVector one divides by 12, has max_size 0x15555555
  and takes the size through 0x5E9850 on the growth path. _Xlen 0x5ECC30 stays original like its siblings (now a lane C
  direct frontier row). Split decision: none needed: the catalogue already has the body as 0x5ED750-0x5ED92A +
  0x5ED93F-0x5EDA4B with the catch funclet 0x5ED92A an excluded X-eh-funclet row, and CAT-16 finds no fall-out there.
- CTaskSimpleJetPack::RenderJetPack 0x67F6A0 (x87 heading clamp, FSIN / FCOS jet balls; the bytes have strict wrap tests
  and `angle + r` / `angle - r` per ball) and DoJetPackEffect 0x67B7F0; new scenario `jetpack` (ROCKETMAN): RenderJetPack
  157 calls in 1000 iterations, blocked on lane A's ProcessPed 0x6801F0 (needs ProcessControlInput 0x67E7B0, ProcessThrust
  0x67EF20, ~3.8 KB). CPedGroupPlacer::PlaceFormationGroup 0x5FC9B0 (no scenario reaches it).
- Virtuals: CEventGunAimedAt::TakesPriorityOver 0x4B0810, RadiusAngleOffset::ComputeEntitySeekPos 0x6946F0,
  GangJoinRespond::ControlSubTask 0x661950, FollowAnyMeans::AllocateDefaultTasks 0x5F6B40, GoToPointAnyMeans::ControlSubTask
  0x672A50. Bindings added: 0x5EB340, 0x630590, 0x4ABC30, 0x5F5D80 (sa-ai), 0x632140 (sa-pop).
- frontier.py: AI 61 (4 direct + 57 virtual) → 47 (1 + 46); to port 265 → 242 (all lanes, this checkout).
- Scenarios (frontier.sh, i686 native, SA_ROOT=~/sa-engine): final tree: pausemenu 416 / 0 blocked,
  solicit 4016 / 0, crossing 1216 / 0, crossing-long 4016 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0,
  shootdriver 1216 / 0, roadblock-script 1016 / 0, gangwar 2016 / 0 (was 1: TellGangMembersTo), roadblocks 875 / 1141 (lane B:
  CCam::ArrestCamOne 0x518500, CHeli::GenerateHeli 0x6C6520), roadblock-wanted 1015 / 1001 (lane B: 0x518500,
  CWorld::FindObjectsIntersectingCube 0x568DD0, 0x6C6520), jetpack (new) 226 / 790 (lane A: CTaskSimpleJetPack::ProcessPed
  0x6801F0); no crash.
- verify-all (lanes 1-3, fresh sa-oracle): `f2a6-u` (SA_UNVERIFIED=frontier2-a9, DLL of
  cf830f09: every port; 8d0d5d28 after it only replaces a helper by its import) IDENTICAL 13/13 and `f2a6-def` (default
  set, DLL of cdfa2de5) IDENTICAL 13/13; save-load identical except the SAVE-1 stack residue.
- Writer gate (base 235b6b1a): the final run (head 8d0d5d28) passed every step but
  queue (the refactor moved a source line; queue regenerated in cdfa2de5, `queue.py --check` up to date); hooks 28 new, all
  unverified, batch frontier2-a9 (spans AI, POP, UNK-game). Earlier runs failed queue (a racing regeneration) and dupes
  (PlaceFormationGroup's local copy of entity_pos, fixed in 8d0d5d28).
- Not done: PlaceChatGroup 0x5FCE80, the jet pack ProcessPed chain, the other 46 AI virtuals, the partial script switches,
  ProcessColModels' disk cases.

## 2026-11-16 — FRONTIER-2 turn 7, lane A: the jet pack ProcessPed chain, PlaceChatGroup, AI virtuals (agent, branch worktree-agent-aa15ab7791f090943)
- Base: worktree-agent-aa16f3bdf26614476 (3796d82e, lane A turn 6). Every port from the 1.0 US bytes (gta-reversed read as an
  aid; where it differs the bytes win), every address checked against `inflight.py` first (none registered elsewhere).
  One batch, `frontier2-a10` (21 hooks, all AI), in the new module sa-ai `frontier_a_t7.rs` (PlaceChatGroup in
  `ped_group_formation_place.rs`).
- Jet pack (the `jetpack` scenario's lane A block): CTaskSimpleJetPack::ProcessPed 0x6801F0, ProcessControlInput 0x67E7B0
  and ProcessThrust 0x67EF20, x87 throughout with the original's f32 stack slots modelled (`fstp` / `fld` round trips);
  pow through `ci_pow` (orig::x87_binary 0x822130), FPATAN / FSIN / FCOS through X87; rand at 0x6802A9 / 0x680315;
  ProcessControlInput's `mov eax, [edi+0x71C]` runs in the protection fragment 0x4075E5. Bytes vs gta-reversed:
  ProcessPed sets the weapons-switched flag (+0xC) for any ped; the mouse heading negates the camera front instead of
  subtracting pi; the targeted-object pull divides by max(0.1, dist) and scales by (1 - dist / limit) * 3; the
  non-mouse forward thrust ignores the pad's up-down; the stop force includes the mass; the forward thrust clamps the
  summed z (not the forward part) to 0.75; the height factor is `1 - (z - h) * 0.05`.
- CPedGroupPlacer::PlaceChatGroup 0x5FCE80 (gta-reversed only calls the original): the jittered circle, ground / clear /
  sight / height tests, the chat allocator 0xC09910; the first point is an uninitialised stack vector until point 0 is
  placed (hazard uninit-locals); a failed AddPed calls RemovePed(NULL) as the bytes. PlaceFormationGroup's visibility
  test and group set-up are now the shared helpers `seen_near_player` / `build_group` (no behaviour change).
- Virtuals: 0x62E540 StealthKill::ProcessPed, 0x62DC20 DestroyCarMelee::CreateNextSubTask, 0x63D420
  ShuffleSeats::CreateNextSubTask, 0x640730 EnterCarAsPassengerWait::ControlSubTask, 0x675890 InteriorGoToInfo::
  CreateNextSubTask, 0x697F10 WaitForBackup::CreateNextSubTask, 0x694C70 SeekEntityAiming::ControlSubTask (the seven
  named first), then 0x639E40 UsePairedAttractor::CreateNextSubTask, 0x65B660 FleePoint::ControlSubTask, 0x622980
  KillPedFromBoat::ControlSubTask, 0x6335C0 PlayerOnFire::ProcessPed (CEventDamage on the stack), 0x657820
  DiveFromAttachedEntityAndGetUp::CreateFirstSubTask, 0x676640 InteriorGoToInfo::CreateFirstSubTask, 0x62F6E0
  DuckToggle::ProcessPed, 0x61F9F0 ThrowControl::ProcessPed, 0x5F6FC0 SitInLeaderCar::AllocateDefaultTasks, 0x66C820
  InvestigateDisturbance::CreateFirstSubTask. Generated natives (0x628A70, 0x638D30) are reached through `orig!`.
  Several originals store a value over their own ped argument slot (hazard stack-residue, noted per hook).
- Reuse (ADR-003): kill_ped_on_foot_armed's `current_weapon_info` shared, `sa_core::field::gf` imported (a local copy
  failed dupes once); no new bindings rows needed (bindings.py --check clean; CAEPedAudioEntity 0x4E28A0 / 0x4E0EE0,
  SayPedless 0x5073C0 and PedsOnFire 0x56D2D0 declared once, in frontier_a_t7.rs).
- frontier.py: AI 47 (1 direct + 46 virtual) → 28 (0 + 28); to port 240 → 217 (all lanes, this checkout).
- Scenarios (frontier.sh, i686 native, final tree, SA_ROOT=~/sa-engine): pausemenu 416 / 0 blocked, solicit 4016 / 0,
  crossing 1216 / 0, crossing-long 4016 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0,
  roadblock-script 1016 / 0, gangwar 2016 / 0, roadblocks 875 / 1141 (lane B: CCam::ArrestCamOne 0x518500, CHeli::
  GenerateHeli 0x6C6520), roadblock-wanted 1015 / 1001 (lane B: 0x518500, CWorld::FindObjectsIntersectingCube 0x568DD0,
  0x6C6520), jetpack 228 / 788: no lane A blocker left; it now blocks only on lane C's CAEPedAudioEntity::UpdateJetPack
  0x4E0EE0 at the end of ProcessThrust (565 bytes, unregistered everywhere); no crash.
- Writer gate (base 3796d82e, target dir target/gate): all 15 steps PASS (hooks 21 new, all unverified, batch
  frontier2-a10; queue up to date).
- verify-all (lanes 1-3, sa-oracle rebuilt from this tree, DLL sha256 39955e05… of 144ab2bd): `f2a7-u2`
  (SA_UNVERIFIED=frontier2-a10) IDENTICAL 13/13 and `f2a7-def` (default set) IDENTICAL 13/13; save-load identical except
  the SAVE-1 stack residue. The first `f2a7-u` attempt stopped with "no spec entry for 0x5F6FC0 0x66C820" (the host
  sa-oracle predated the last two ports); killed by PID, rebuilt, rerun.
- Not done: the other 28 AI virtuals (0x65A990 / 0x65ABF0 / 0x64CF40 / 0x62E8B0 / 0x6689E0 / 0x670E20 / the vt_87004c and
  vt_870b98 families / the interior ProcessPeds and others), the partial script switches, ProcessColModels' disk cases;
  jetpack's remaining block is lane C's UpdateJetPack 0x4E0EE0.

## 2026-11-16 — FRONTIER-2 turn 8, lane A: 22 of the 28 AI virtuals and their callees (agent, branch worktree-agent-a673f3b762ca00e8d)
- Base: worktree-agent-aa15ab7791f090943 (c04d5a6e, lane A turn 7). Every port from the 1.0 US bytes (gta-reversed read as an
  aid; where it differs the bytes win), every address checked against `inflight.py` first (none registered elsewhere).
  One batch, `frontier2-a11` (31 hooks: AI, plus the UNK-game helpers 0x62EB80 / 0x673BA0) and 2 natives, in the new
  module sa-ai `frontier_a_t8.rs` (0x673BA0 in misc.rs next to its twin 0x665DC0).
- Ports: CTaskComplexMedicTreatInjuredPed CreateNextSubTask 0x65A990 / ControlSubTask 0x65ABF0; vt_870b98 =
  CTaskComplexGoPickUpEntity (task 0x136): CreateFirstSubTask 0x693610, CreateNextSubTask 0x691AE0, ControlSubTask 0x691D50;
  GoToPointAiming::ControlSubTask 0x6689E0; FollowPatrolRoute::CreateFirstSubTask 0x670E20 (vt_870114; both searches compare
  a squared distance with the square of the best squared distance, as the bytes); SeekCoverUntilTargetDead::ControlSubTask
  0x62E8B0 (a dead cover ped is replaced through GetTargetPed, as the bytes); KillPedOnFootStealth::ControlSubTask 0x62B840;
  Sunbathe CreateSubTask 0x638290 (native) / CreateFirstSubTask 0x639CB0 (a dropped rand at 0x639CB9) / CreateNextSubTask
  0x6399F0; CTaskTimer::GetTimeLeft 0x62EB80 (its island's time load `[[0x4EC103] ^ [0x156061C]]` = `[0xB7CB84]`, static
  image values); 0x6955D0 (catalogue name `(missed by ghidra: cat7-call)`); DieInCar PreparePedVehicleForPedDeath 0x62FD00 /
  CreateFirstSubTask 0x6375F0; FleeEntity::ControlSubTask 0x65BBB0 + FleePoint::SetFleePosition 0x65B7B0;
  FollowLeaderInFormation::ControlSubTask 0x696940; TrackEntity ControlSubTask 0x663640 / CalcTargetPos 0x65F780 /
  CalcMoveRatio 0x65F930 (function statics, guard 0xC18D08) + GoToPointFine::SetTargetPos 0x65F330 (preserve ecx: the caller
  reuses ECX); Choking::ProcessPed 0x620490; vt_86d868 CreateFirstSubTask 0x625C70 + its CreateSubTask 0x620C30 (native);
  vt_87004c = a CTaskComplexSeekEntity with the XYOffset layout: CreateSubTask 0x673070 / CreateFirstSubTask 0x6733E0 /
  CreateNextSubTask 0x672E80 / ControlSubTask 0x673540 on seek.rs's shared bodies (made pub(crate);
  `seek_control_sub_task_with` takes the instance's CreateSubTask and seek-position copy 0x673BA0); CreateCarAndGetIn::
  ProcessPed 0x64CF40; the chat allocator 0x5F8180.
- Natives: 0x638290 and 0x620C30 have code-key entries (protection reads 0x638292 at 0x156D78F and 0x620C31 at 0x40135D;
  the hook table tests caught it), so they are phase B natives (frontier_a_t8::NATIVES, registered in sa-ports).
- Reuse (ADR-003): fight_combo_style's group_block / block_index, frontier_a_t5's seek_cover_sequence (split out of
  0x62E6C0), frontier_a_t6's route_pos, flee.rs's dist2 and the new broadcast_seen_panicked_ped, cworld's in_vehicle (a
  local copy failed dupes once). Bindings: CPad::SprintJustDown 0x5407F0 (sa-input, shared with sa-ped), the
  InvestigateDisturbance constructor 0x66C720 and the BeHit constructor 0x620780 (sa-ai; their local declarations removed);
  rows placed without reordering the CSV.
- frontier.py: AI 28 virtuals → 6; to port 213 → 190 (all lanes, this checkout).
- Scenarios (frontier.sh, i686 native, SA_ROOT=~/sa-engine): pausemenu 416 / 0 blocked, solicit 4016 / 0, crossing 1216 / 0,
  crossing-long 4016 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0, roadblock-script 1016 / 0,
  gangwar 2016 / 0, roadblocks 875 / 1141 (lane B: CCam::ArrestCamOne 0x518500, CHeli::GenerateHeli 0x6C6520),
  roadblock-wanted 1015 / 1001 (lane B: 0x518500, CWorld::FindObjectsIntersectingCube 0x568DD0, 0x6C6520), jetpack 228 / 788
  (lane C: CAEPedAudioEntity::UpdateJetPack 0x4E0EE0); no lane A blocker, no crash.
- Writer gate (base c04d5a6e, target dir target): all 17 steps PASS on fa2eca9e (hooks 31 new, all unverified, batch
  frontier2-a11 spanning AI and UNK-game; queue up to date). Earlier runs failed queue (regenerated after later commits),
  clippy (`saturating_sub`), hooktest (the two code-key entries; ECX read after 0x65F330 by 0x66376C, now `preserve =
  "ecx"`) and l1test (0x65F330 added to the pending preserve specs).
- verify-all (lanes 1-3, sa-oracle rebuilt from this tree, DLL sha256 a7b4fb29… of 5cb47b31 / fa2eca9e): `f2a8-u2`
  (SA_UNVERIFIED=frontier2-a11) IDENTICAL 13/13 and `f2a8-def` (default set) IDENTICAL 13/13; save-load identical except
  the SAVE-1 stack residue. A first `f2a8-u` (DLL still hooking the two code-key entries) was IDENTICAL 13/13 too. The
  lanes are shared with other sessions (lock waits).
- Not done: KillPedWithCar::ProcessPed 0x6564A0 (needs 0x653CF0 / 0x6563D0), UseSwatRope::ControlSubTask 0x65A460 (needs
  CRopes::FindCoorsAlongRope 0x555E40), GangHassleVehicle::ControlSubTask 0x6637C0 (0x6D3 bytes), the interior ProcessPeds
  0x676D30 / 0x6772E0 / 0x677920; the partial script switches 0x47A760 / 0x48B590; ProcessColModels' disk cases.

## 2026-11-16 — FRONTIER-2 turn 9, lane A: KillPedWithCar, the swat rope, gang hassle, the interior bed / chair tasks (agent, branch worktree-agent-a811558b41ef1a9e5)
- Base: worktree-agent-a673f3b762ca00e8d (7a65de4a, lane A turn 8). Every port from the 1.0 US bytes (gta-reversed read as an
  aid; where it differs the bytes win), every address checked against `inflight.py` first (none registered elsewhere).
  Batches `frontier2-a12` (6 hooks: AI + RENDER 0x555E40) and `frontier2-a13` (2 hooks, AI), all plain `#[hook(`, in the
  new module sa-ai `frontier_a_t9.rs` (0x555E40 in sa-render `ropes_register.rs`). The hook table tests found no code-key
  entry, so no phase B native this turn.
- CTaskSimpleKillPedWithCar: ProcessPed 0x6564A0, DamageCarBonnet 0x653CF0, the pad shake 0x6563D0 (`FUN_006563d0`; the
  frequency dword's upper bytes are the `1 / mass` stack slot, as the original). Bytes vs gta-reversed: the run-over test
  uses `forward · speed > 0`, the bonnet height uses max.z, the span for `forward.z > 0.1` depends on the travel direction,
  the low front hit is direction 5 (piece 9), the bonnet's speed starts from the ped's speed. Three events on the stack
  (hit-and-run, damage calculator, CEventDamage). CLocalisation::KnockDownPeds 0x56D290 and RemoveBonnetInPedCollision
  0x6AA200 became sa-pop bindings (twins in sa-hud / sa-veh), sa-ped's KillPedWithCar imports them.
- CRopes::FindCoorsAlongRope 0x555E40 (gta-reversed only calls the original; free per inflight) and
  CTaskComplexUseSwatRope::ControlSubTask 0x65A460 (a rope that is not found aborts into CreateSubTask(0xC8); gta-reversed
  keeps the sub-task).
- CTaskGangHassleVehicle::ControlSubTask 0x6637C0 (~2.5 KB with the `.text` island 0x4010BF; gta-reversed only calls
  the original): anim block ref, kill / hassle-ped hand-offs, look-at, the 0..3 state machine, secondary gang anims, the
  car hit through a stack CTaskSimpleFight + CMatrix (ProcessColModels, FightHitCar, health restored). fighthelp's
  FightHitCar / sphere set-up declarations made crate-visible and reused; `cover_update::speed` reused for |v| (a local
  copy failed dupes once).
- Interior: CTaskInteriorLieInBed::ProcessPed 0x6772E0 and vt_870314::ProcessPed 0x676D30 (the house chair task,
  "TaskSitInChair", anims 0x157 / 0x159 / 0x158, look-at tail at the closest interior info) on shared helpers
  (`interior_blend`, `interior_start_timer`, `interior_step_to_info` = the identical x87 tail 0x677606 / 0x67705B);
  frontier_a_t5's FindClosestInteriorInfo declaration made crate-visible. Both originals store floats over their own ped
  argument slot (hazard stack-residue).
- frontier.py: AI 6 virtuals → 1 (CTaskInteriorSitAtDesk::ProcessPed 0x677920); to port 179 → 174 (all lanes, this
  checkout). POP / PED rows unchanged: 0x41D0E0 / 0x44DF60 / 0x460DF0 are on master; 0x422760, 0x60E530 and the POP
  partials 0x461170 / 0x612710 / 0x6F34D0 not started.
- Scenarios (frontier.sh, i686 native, SA_ROOT=~/sa-engine): pausemenu 416 / 0 blocked, solicit 4016 / 0, crossing
  1216 / 0, crossing-long 4016 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0,
  roadblock-script 1016 / 0, gangwar 2016 / 0, roadblocks 875 / 1141 (lane B: CCam::ArrestCamOne 0x518500, CHeli::
  GenerateHeli 0x6C6520), roadblock-wanted 1015 / 1001 (lane B: 0x518500, CWorld::FindObjectsIntersectingCube 0x568DD0,
  0x6C6520), jetpack 228 / 788 (lane C: CAEPedAudioEntity::UpdateJetPack 0x4E0EE0); no lane A blocker, no crash.
- Writer gate (base 7a65de4a): all 17 steps PASS on b12e114d (hooks 8 new, all unverified, batches frontier2-a12 /
  frontier2-a13; queue up to date; no new L1-pure hook).
- verify-all (lanes 1-3, sa-oracle rebuilt from this tree, DLL sha256 cc1401e8… of 2628c6eb): `f2a9-u`
  (SA_UNVERIFIED=frontier2-a12,frontier2-a13) IDENTICAL 13/13 and `f2a9-def` (default set, same DLL) IDENTICAL 13/13; save-load identical
  except the SAVE-1 stack residue. The lanes are shared with other sessions (lock waits, ~1 h per run).
- Not done: CTaskInteriorSitAtDesk::ProcessPed 0x677920 (same shape as 0x6772E0, plus StartRandomLoopAnim /
  StartRandomOneOffAnim calls), the partial script switches 0x47A760 / 0x48B590, ProcessColModels' disk cases, POP
  0x422760 and the three POP partials, PED 0x60E530.

## 2026-11-16 — FRONTIER-2 turn 6 lane C: ProcessMissionAudioEvent, replay vehicle / player info copies, pedless speech, user tracks, script pick-up, batch c12 (agent, branch worktree-agent-a666e9cf719f6e793)
- Base: lane C turn 5 head 49ab9148 (branch worktree-agent-acc988eb6f9ea226b). Every port address checked against
  `tools/gen/inflight.py` first (none in flight).
- Batch frontier2-c12 (10 hooks + 1 native):
  - CAEScriptAudioEntity::ProcessMissionAudioEvent 0x4ECCF0 (new module `sa-audio/src/mission_audio_event.rs`): the
    compare tree with five jump tables is one `match` on the 16-bit event. The case map came from a unicorn run of the
    dispatch for all 65536 event values (137 case blocks), each block then traced call by call under variants (physical
    / no physical, callee results 0 / 1, +0x7D / +0x7E) and turned into arms over shared helpers (mission bank sound,
    resident sound, frontend event, special ambience track, cancel with the physical variant, door event at the
    physical's or the given position) plus the per-block helpers (crane winch / attach / move / stop / detach,
    bank-by-loaded, the +0x7E toggle, stereo pair, blood crunch, water splash). gta-reversed's commented body has wrong
    case labels. The 0x4A6 block's bank 0x8F is pushed by the SecuROM fragment 0x405BA1.
  - tReplayVehicleBlock::ExtractVehicleUpdateData 0x45BA90 (sa-veh `replay_vehicle.rs`): a phase B native, not a hook
    (the protection reads the entry dword 0x45BA91 as a key: 0x401831, 0x40A486, 0x156A03F, 0x156D5E5; the sa-hook INF-20
    install test refused the hook). Unlike gta-reversed the bytes sign-extend the speed / steer / wheel / door bytes and
    store the area byte directly. Its callee ApplyPanelDamageToCar 0x59CA10 is a hook.
  - CPlayerInfo::operator= 0x45DEF0 (.HOODLUM 0x156AE50) and CPlayerData::operator= 0x45C4B0 (.HOODLUM 0x1561CF0;
    sa-world `world/frontier_c.rs`): the no-CD body of the latter has ten NOPs where the source load of the first +0x34
    bit-field step was, so that step reads the caller's ESI (a `reg("args = ecx stack esi; pop")` argument; the outer
    copy passes `this`).
  - CAEPedlessSpeechAudioEntity::AddSayEvent 0x4E60D0 next to the ped version (its declarations and checks reused).
  - CAEUserRadioTrackManager::LoadUserTrack 0x4F35F0 and GetTrackPath 0x4F3050 (`sa-audio/src/user_track.rs`; the
    decoders and CAEDataStream stay original).
  - CRunningScript::ScriptTaskPickUpObject 0x46AF50 (.HOODLUM 0x156B440): the three protection values evaluated on the
    image: `~[0x1560560] ^ [0x4C4572]` = 0 (hold-entity flag), `[0x1560564] - [0x6A9663] - 1` = 4 (secondary slot for
    opcode 0x70A), VM 0xCB73E0 descriptor 0x1560028 = 0 (other opcodes, `tools/re/vm_eval.py` of lane A); the call/ret
    trampoline's target `rol([0x1560568], [0x4D7191])` = 0x632D10 (CTaskComplexSequence::AddTask).
  - The closest attractor search 0x6002F0 and the may-react-against test 0x603AF0 (sa-ai `frontier_c.rs`).
- Bindings: new 0x4EC550 / 0x4EC6D0 / 0x4D6D60 / 0x4EFB90 / 0x5026B0 (sa-audio), 0x59B9F0 (sa-core), 0x601C90 (sa-ai),
  0x4DCBE0 (sa-world, sa-audio port twin); the local declarations they replace removed; 0x4ECCF0 / 0x46AF50 / the
  stream thread's 0x4F35F0 declaration reach the ports.
- Not ported: 0x61D0B0, 0x61FF40, 0x66AB40 (time), the D3D9 raster helpers 0x4CA5A0 / 0x4CD050 / 0x4CC1D0, 0x477DA0
  (CAT-12 not extended: needs a run_all with the Ghidra project), the 0x14274C2 junk proof. New callee left to lane B
  (MATH): CMatrix::ScaleAll 0x45AF40 (its loop counter set in the fragment 0x401BA5).
- VM tool note for the main session: `tools/re/vm_eval.py` evaluated 0x1560028 without help; lane C turn 5's
  target/lc5/tools/vm.py had no feature it lacks for this site.
- Gates: writer gate (base 49ab9148) all PASS (hooks: 10 new, all unverified, batch frontier2-c12; note: the batch spans
  AUDIO / SCRIPT / UNK-game). The first run failed hooktest on 0x45BA90 (protection key reads of its entry, now a native)
  and the 0x59CA10 hook name (now the catalogue's).
- Scenarios (master's `frontier.sh`, i686 native sa-game of this branch; target/lc6/scen): pausemenu 416 / 0 blocked,
  solicit 4016 / 0, crossing 1216 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0;
  crossing-long 4014 / 2 (lane A's KillCriminal::CreateNextSubTask 0x68E4F0); roadblocks 254 / 1762 (registered on
  master, not on this branch: CTaskSimpleGangDriveBy MakeAbortable 0x62D290 / ProcessPed 0x62D3B0, the
  GenerateRoadBlocks fallback, CHeli::GenerateHeli 0x6C6520); quad 1215 / 1 (CCheat::VehicleCheat 0x43A0B0, on master);
  0 blocked on lane C functions; no crash.
- verify-all (lanes 6-7): `lanec6-c12` (SA_UNVERIFIED=frontier2-c12; DLL of 21aa7b02) IDENTICAL 13/13; `lanec6-default` (default set) IDENTICAL 13/13.
- frontier.py: lane C subsystems 17 → 9 rows (2 of them lane A's partial switches); whole frontier "to port" 218 → 199
  (other branches' registrations included); new MATH row 0x45AF40 (lane B).

## 2026-11-16 — FRONTIER-2 turn 7 lane C: fight / be-hit / avoid-entity helpers, D3D9 raster helpers, natives to hooks, 0x477DA0, batch c13 (agent, branch worktree-agent-a6e18bff35f33ca8b)
- Base: lane C turn 6 head e20c571b (branch worktree-agent-a666e9cf719f6e793). Every port address checked against
  `tools/gen/inflight.py` first (none in flight).
- Batch frontier2-c13 (9 hooks + 1 native):
  - CTaskSimpleFight's vehicle hit 0x61D0B0 (sa-ai `fight_hit_object.rs`, next to its object twin 0x61D400; the two
    share the chainsaw-audio / hit-sound / punch-impact tail): strike damage, chainsaw sparks along the bone 0x18
    matrix `at` vector, `VehicleDamage(mass * dmg * 0.00075 | 0.01)`, ReportCrime 0x15, a CEventVehicleDamageWeapon
    (ctor 0x4B18D0 + vtable 0x86D640) for the driver and every passenger, the last-damage weapon / entity when the
    health dropped. New sa-ai binding Fx_c::AddPunchImpact 0x49F670.
  - CTaskSimpleBeHitWhileMoving::StartAnim 0x61FF40 (task type 1009, vtable 0x86D7FC; sa-ai `be_hit.rs`): the anim
    from hit type / direction / rand(), the +0x744 timer (x87, `_ftol`), the finish callback 0x61FF30 pushed by the
    SecuROM fragment 0x401122 (`ror([0x15605F8], [0x698EFA])` on the image); the paths that leave +0x14 null keep the
    original's SetFinishCallback through null. ClearAimFlag / ClearLookFlag bindings moved to sa-world (sa-ped twins).
  - The avoid-entity task's look-at 0x66AB40 (vtable 0x86FF00, "TaskAvoidEntity"): the twin of
    CTaskSimpleGoTo::SetUpIK 0x667AD0; the direction / dot tests and the x87 look-at point are one helper
    (`look_at_walk_direction`) for both.
  - The D3D9 raster helpers (sa-rwcore): the format resolver 0x4CC1D0 (pure format logic over the ported
    `_rwD3D9CheckValid(Camera)TextureFormat`; no device call), `_rwD3D9RasterCreateFromDDSFormat` 0x4CD050
    (IDirect3DDevice9::CreateTexture through the device vtable binding of d3d9rast.rs, as the other raster create
    paths) and the textured-quad render 0x4CA5A0 (register convention; every path reaches a device vtable call
    before `ret`, so a phase B native next to its twin 0x4CAAE0 in raster_copy.rs, reached through the dispatcher's
    binding). New callee for lane B (RW): `_rwD3D9TestState` 0x7F8300.
  - The angled-area test 0x477DA0 (sa-script `angled_area.rs`): catalogued by hand (catalog/extra-code-addresses.csv,
    rule cat17-hand: the E8 calls at 0x4798FA (case 09C0 of ProcessCommands2400To2499, which has no Ghidra body) and
    0x477FEA) and run_all on a copy of the main checkout's Ghidra project (the main checkout untouched); its x87 flow
    keeps dy on the stack (dx stored); inside_2d's range tests are shared (`in_range`, `side_ok`).
  - CAEPedAudioEntity::UpdateJetPack 0x4E0EE0 (sa-audio `ped.rs`; coordinator request: the last blocker of lane A's
    `jetpack` scenario): the four level fields step up / down against their `.data` bounds by the thrust against the
    double 0.5; the three jet-pack sounds' volumes / speeds from them and sin(angle) (the x87 |s| for +0x9C, the f32's
    for +0xA4).
  - The phase B natives CAEWeatherAudioEntity::AddAudioEvent 0x506800 and tReplayVehicleBlock::ExtractVehicleUpdateData
    0x45BA90 are hooks now (see sa-hook below).
- sa-hook (INF-20): an inline hook writes only its 5-byte `jmp`; the rest of the relocated instructions keeps its
  bytes (the installer already refuses branches into the relocated range). `data_reads::changed` and the keycells test
  model the 5 bytes. The indexed `.data` reader 0x14274C2 (`fmul qword ptr [ebx + 0x506805]`) then sees no hook byte
  of 0x506800 (no junk proof needed; static notes: no instruction on a linear sweep of game `.text` / `.HOODLUM`
  branches into the executable `.data` 0x1301000, the block holding 0x14274C2 has entropy 7.33 bits/byte against
  6.2-6.6 for code). The four protection key reads of 0x45BA91 (0x401831 pointer key, 0x40A486 / 0x156A03F /
  0x156D5E5 rotate counts) join keycells; two keycell readers of entry + 5 bytes became unnecessary and were dropped
  (0x401375 SetFOV, 0x407E2C GetIsWearingBalaclava). docs/re/l2-oracle.md updated.
- 0x45C4B0 ten NOPs (.HOODLUM 0x1561D4C..0x1561D55): no dword and no rel32 anywhere in the image points into
  0x1561D40..0x1561D60 (static scan of every section), `.HOODLUM` is stored unpacked (raw = virtual size 0x21000);
  the hook is kept as it is (the step reads the caller's ESI). Not confirmed at run time: a /proc/PID/mem sampler
  around verify-all (26 gta_sa.exe processes, 250 samples) read zeros / 0x3F at every probed address, exe bytes
  included, i.e. not the game's 32-bit image view under Wine's WoW64 (tool problem, no evidence either way).
- Not ported (other lanes): the new RW callee `_rwD3D9TestState` 0x7F8300 (device lost / reset; only 0x4CA5A0 calls
  it) and CMatrix::ScaleAll 0x45AF40 (lane B); lane A's partial switches 0x47A760 / 0x48B590.
- Gates: writer gate (base e20c571b, on 12f3e0ae) all PASS (hooks: 9 new, all unverified, batch frontier2-c13; note:
  the batch spans AUDIO / UNK-game).
- Scenarios (master's `frontier.sh`, i686 native sa-game of ca47a074; target/lc7/scen): pausemenu 416 / 0 blocked,
  solicit 4016 / 0, crossing 1216 / 0, melee 2616 / 0, sniper 1216 / 0, shootcar 1216 / 0, shootdriver 1216 / 0;
  crossing-long 4014 / 2 (lane A's KillCriminal::CreateNextSubTask 0x68E4F0); roadblocks 254 / 1762 (0x62D290,
  0x62D3B0, the GenerateRoadBlocks fallback, CHeli::GenerateHeli 0x6C6520: registered on other branches); quad / heli
  / plane 1215 / 1 (CCheat::VehicleCheat 0x43A0B0, other branch); 0 blocked on lane C functions; no crash.
- verify-all (lanes 6-7): `lanec7-c13` (SA_UNVERIFIED=frontier2-c13; DLL of 7fd4b0bb) IDENTICAL 13/13; `lanec7-c13b`
  (same batch, DLL of 12f3e0ae, with 0x4E0EE0) IDENTICAL 13/13; `lanec7-default` (default set, VERIFY_NO_BUILD on the DLL of
  7fd4b0bb; the writer gate rebuilt the DLL at a5010982 during the run, which adds only the unverified 0x4E0EE0)
  IDENTICAL 13/13.
- frontier.py: lane C subsystems 9 → 2 rows (both lane A's partial switches); whole frontier "to port" 193 → 170
  (other branches' registrations included); new RW row 0x7F8300 (lane B).

## 2026-11-16 — FRONTIER-2 turn 8 lane C: RW leftovers, Interior_c::PlaceObject, GeneratePanesForWindow, batches c14 / c15 / c16 (agent, branch worktree-agent-acd1111ff68d2fe92)
- Base: lane C turn 7 head a102bfd2 (branch worktree-agent-a6e18bff35f33ca8b). Lane C's own subsystems had no rows left
  besides lane A's partial switches, so this turn took lane B's RW / WORLD / RENDER leftovers (camera bodies and
  0x73AF00 stay with lane B). Every address checked with frontier.py / `tools/gen/inflight.py` first (none in flight).
- Batch frontier2-c14 (RW, 6 hooks):
  - `_rwD3D9TestState` 0x7F8300 (sa-rwcore `d3d9cam.rs`; the new row of 0x4CA5A0): TestCooperativeLevel, the
    DEVICENOTRESET reset (INVALIDCALL on a windowed device retries with the desktop format; depth table 0x7F84BC equals
    `format_depth`), post-reset sequence (result ignored) + restore callback, BeginScene. `_rwD3D9CameraBeginUpdate`
    ends with the same code inlined: its tail now calls the port (one definition).
  - RwCameraStreamRead 0x808DE0 (`camera.rs`); the oversized-struct handling of `_rpClumpAtomicStreamRead` became one
    helper (`stream::read_struct_local`) used by both.
  - `_rwMemoryClose` 0x8020F0 (`memory.rs`, every freelist destroyed through the ported RwFreeListDestroy).
  - `_rpMeshWrite` 0x758D70 (`mesh.rs`); the non-native object test of `_rpMeshRead` is now `mesh::non_native`.
  - RpLightGetConeAngle 0x751AE0 (`light.rs`): the inlined fdlibm acosf, full width in ST0; RtQuatSetupSlerpCache
    inlines the same acosf, now one function `rt_quat::acosf_inline` (X87). The D3D9 light setup's binding returns X87
    instead of an f64.
  - RwD3D9CameraAttachWindow 0x7F8D70 (`d3d9cam.rs`): swap chain for another window; the device calls go through the
    existing vtable bindings of d3d9rast / d3d9sys (made crate-visible, not redeclared; no platform cfg). Its surface
    out-parameter is the hwnd argument slot in the original (port: initialised with hwnd).
- Batch frontier2-c15 (WORLD, 1 hook): Interior_c::PlaceObject 0x5934E0 (sa-world `interior_place_furniture.rs`, next to
  its twin PlaceFurniture, sharing its matrix helpers; unreversed in gta-reversed): point from the interior origin and
  the model's bbox min z, matrix = interior * RotateZ(rot deg) + point, a CObject on a free furniture item, the steal
  records of the interior manager (a stolen object is removed and deleted again). Its "chain" (FurnishBedroom 0x593FC0,
  FurnishLounge 0x597740, Kitchen_FurnishEdges 0x596930, Office_PlaceDesk 0x5993E0, InteriorGroup_c peds) are its
  callers, 450-800 instructions each: not ported this turn.
- Batch frontier2-c16 (RENDER, 1 hook): CGlass::GeneratePanesForWindow 0x71B620 (sa-render `glass.rs`; entry `nop; jmp
  0x403985`). From the bytes; gta-reversed differs: counts `_ftol(max(len + 0.75, 1)) * sections`, pane offsets `i *
  len / n`, delayed time `_ftol(|pos - centre| * 100 + now)`, the random rotation takes the rand() calls in the order
  (b, a, c). rand() sites 0x71B9BB / 0x71B9DB / 0x71BAF5 / 0x71BAFC / 0x71BB31.
- The first three c14 hooks were defined but missing from their HOOKS tables (fixed in 519aca70); clippy's
  needless_range_loop in the pane loop fixed in e14d3e1a.
- Gates: writer gate (base a102bfd2) all PASS on e14d3e1a (hooks: 8 new, all unverified, batches frontier2-c14 / c15 / c16; the first run failed clippy on the pane loop, fixed).
- Scenarios (master's `frontier.sh`, i686 native sa-game of 264ffb54 (e14d3e1a only changes a loop's form)): pausemenu
  416 / 0 blocked, solicit 4016 / 0, crossing 1216 / 0, crossing-long 4014 / 2 (lane A 0x68E4F0), melee 2616 / 0,
  sniper / shootcar / shootdriver 1216 / 0, quad / heli / plane 1215 / 1 (CCheat::VehicleCheat 0x43A0B0, other branch),
  roadblocks 254 / 1762 and roadblock-wanted 250 / 1766 (0x62D290 / 0x62D3B0, the GenerateRoadBlocks fallback, CHeli::
  GenerateHeli 0x6C6520: other branches), roadblock-script 1016 / 0, gangwar 2015 / 1 (0x68E4F0); no crash; 0 blocked
  on this turn's functions or lane C.
- verify-all (lanes 6-7, host sa-oracle and satool rebuilt; DLL of e14d3e1a): `lanec8-c1416` (SA_UNVERIFIED=frontier2-c14,frontier2-c15,frontier2-c16) IDENTICAL 13/13; `lanec8-default` (default set) IDENTICAL 13/13. (A first attempt failed every recording: this worktree had no target/release/satool.)
- frontier.py: whole "to port" 141 -> 133; RW 36 -> 30, WORLD 26 -> 25, RENDER 20 -> 19 rows; no new rows.
- Open (RW): the image / raster helpers 0x7F44C0 / 0x7F4690 / 0x7F4AE0 / 0x7FF450 / 0x7FFFF0 / 0x800C80 / 0x801390 /
  0x803950 / 0x8030C0 / 0x80CA20, the MatFX renders 0x812AC0 / 0x812D40 / 0x813A10 / 0x814460 / 0x810830, the skin
  paths 0x7C8060 / 0x7C9DA0 / 0x7CA410 / 0x7CA850, RtAnimInterpolatorAddAnimTime 0x7CD8D0, _rpClumpAtomicStreamWrite
  0x74A880 (read; needs RpGeometryStreamWrite 0x74CED0, unbound), 0x4CAEA0, 0x75A960, the world-sector native stream
  0x763F20 / 0x7641C0, 0x7CC110, 0x7CF240, 0x7CF600, 0x5D0820, 0x81F360; WORLD / RENDER: PlaceObject's callers above
  and the rest of frontier.py's rows.
## 2026-11-16 — peds without legs: the game heap's address range (agent, branch worktree-agent-a7644b7409a362326)

- User report: in the play package (real time, `--system-heap`) some peds render without their lower body, randomly,
  not tied to one model. Base master fd8240d3 (the worktree was created at 1f765b2a and fast-forwarded).
- Root cause (offscreen evidence, temporary diagnostics not committed: bone-matrix checks after the skin matrix
  preparation, per-ped key-frame dumps, a probe in `CPedIK::PitchForSlope`): the leg bones' matrices are NaN.
  `CClumpModelInfo::CreateInstance` 0x4C5140 → `RtAnimInterpolatorSetCurrentAnim` 0x7CD5A0 writes RW's key-frame
  pointers over the x / y of SA's header-less interpolated frames (checked against the bytes: the original does the
  same). Until the first on-screen anim update those are the bone quaternions; with the original's heap (far below
  0x3F800000) they read as tiny floats, with glibc's heap after the 32-bit PIE (0x5Bxxxxxx) as about 1e17, so the
  bone matrices become inf / NaN. `CPedIK::PitchForSlope` 0x5FE0E0 reads the previous frame's thigh matrix (asin of
  up.z) and keeps the thigh / calf / foot key frames NaN while the ped stands on a slope (the user's screenshot: a ramp);
  freed NaN matrix arrays seed later peds through `RpHAnimHierarchyCreate`'s uninitialised array. Ruled out on the way:
  the D3D9 seam (no draw errors, buffer / alpha-test states), the skin instancing and blenders, the mesh data (the cop
  models have one mesh and one material), texture alpha.
- Fix (sa-game platform layer, no port changed): every run serves the game heap from the fixed-address region at
  0x18000000 (`det::set_fixed_heap(!system_heap)`; offscreen real-time runs used glibc before), help text and
  `detheap` doc updated, `--system-heap` documented as not 1:1. platform.md section 23, ROADMAP STD-3 note.
- Verified: real time, 6,000 iterations from Grove Street: glibc heap 10 of 10 runs with NaN skeletons, 6 with NaN legs
  (bfypro, fam2 / wfybe, bmyst, sfypro, bmydj, bmycr, fam3, bfyst); fixed region 0 of 6 (3 forced before the fix, 3 with
  it). Deterministic run (`--fixed-step 40`, CJ running into Grove Street) with the region moved to 0x60000000: 58
  NaN-leg frames, `target/legs/evidence/before-legless-ped{,-crop}.png` (a ped drawn without legs); at 0x18000000: 0
  (`after-*.png`). Long session (16,000 real-time iterations, nine teleports in Los Santos): heap top flat at 55 MiB
  from iteration 14,000 on (platform.md section 23). Writer gate and wasm32: see the hand-back.
- For the main session: the play package's `play.sh` passes `--system-heap` and keeps the bug; dropping it fixes it
  (the long-session measurement above was the open condition). Seen but not investigated: the prepared skin matrices of
  the right / left hand and finger bones (12..14, 18..20) sometimes have rows outside 0.5..2 in length, with either heap.

## 2026-11-16 — FRONTIER-2 turn 8, lane B: the cinema camera decisions, Get_TwoPlayer_AimVector, FireOneInstantHitRound (agent, branch worktree-agent-a721df8655aabe1f9)
- Base: worktree-agent-aa54999b4e7af5432 007e0a41 (turn 7 lane B). Item 3 of the brief (Interior_c::PlaceObject chain,
  GeneratePanesForWindow, RW leftovers) moved to lane C by the coordinator mid-turn; none of it was started here.
- Batch `frontier2-b18` (CAM, 3 hooks), from the 1.0 US bytes with docs/re/specs/CAM/CCamera.md as reading aid
  (`crates/sa-cam/src/newcam.rs`, `twoplayer.rs`):
  - CCamera::IsItTimeForNewcam 0x51D770: the elapsed time compared with 20000 in the register and stored f32 for the
    15000 tests, the right-stick step (the latch is set for |x| <= 32: the spec's `< 32` is wrong, `test ah, 0x41; jp`
    falls through on equality), the 30-entry switch (LOS / distance / moving tests, the near-clip cases, the DW cams).
  - CCamera::TryToStartNewCamMode 0x51E560: wheel cameras 0 / 8, the side / ahead / angled fixed cameras with the
    bytes' x87 orders and ground / roof / water fixes, the police car on a string 7, DW cams, near-clip cases. Beyond the
    spec: case 1 computes its position before the boat test; cases 15..17 call VectorSub twice; FindGroundZ / FindRoofZ
    are called as their sa-world ports and GetATanOfXY through sa_math's x87 form so ST0 keeps 64 bits.
  - CCam::Get_TwoPlayer_AimVector 0x513E40: the entry fragment 0x407E0F loads Players[0].m_pPed through
    `[0x156089C] - [0x727253]` (= 0xB7CD98); the nearest screen target, else the FPTAN projection of the screen point.
  - The camproc DW processor bindings return AL and target the ported DW cams; obbe.rs' bindings target the ports.
- Batch `frontier2-b19` (WEAPON, 1 hook): CWeapon::FireOneInstantHitRound 0x73AF00 (`crates/sa-weapon/src/
  fire_one_instant_hit.rs`). Its entry pushes the light's green through the protection fragment 0x4015A0 (`NOT
  [0x1560654] XOR [0x499692]`), evaluated with master's tools/re/vm_eval.py (unicorn): 0x3E6147AE = 0.22 (the tool is
  not on this branch; nothing to extend). gta-reversed differs: the shot animation's association gets blend 0 / delta 8.
- ADR-003: CBulletTraces::AddTrace 0x723750 and CWeapon::FindNearestTargetEntityWithScreenCoors 0x73E240 are now
  catalog/bindings.csv rows (sa-world, twins on the sa-render / sa-weapon ports; sa-render's two private declarations
  removed); weapon_impact's `ci_asin` is pub(crate) and reused; newcam reuses sa-world's `player_vehicle` and sa-core's
  `rd3f`.
- frontier.py, lane B subsystems: 135 → 131 rows (0x51D770, 0x51E560, 0x513E40, 0x73AF00); no new rows revealed.
- Scenarios (i686 native sa-game of 5a0b2348, SA_ROOT=~/sa-engine): pausemenu 416 / 0 blocked, solicit 4016 / 0,
  crossing / sniper / shootcar / shootdriver / quad / heli / boat 1216 / 0, melee 2616 / 0, plane 210 / 1006 (lane C's
  CAEPropellerAudioEntity 0x4FD8F0, unchanged); no crash; 0 blocked on lane B. No scenario reaches the cinema camera
  (it needs the in-vehicle camera cycled to the cinematic mode).
- verify-all (lanes 4-5, fresh release sa-oracle): `lb8-u` (SA_UNVERIFIED=frontier2-b18,frontier2-b19, DLL of 28b6ef78) IDENTICAL 13/13; default set `lb8-def` (same DLL) IDENTICAL 13/13; after the bindings / dupes refactors, `lb8-u2` (b18, b19 on, DLL of be55d9ae, the final code) IDENTICAL 13/13.
- Writer gate (base 007e0a41): the first run (`GATE_SKIP=i686`) failed bindings (0x723750, 0x73E240 declared per module), dupes (`player_vehicle`, then the three-float readers), clippy (a range pattern) and hooktest (the hook name of 0x73AF00 must be the catalogue's `CWeapon::FireOneInstantHitRound`); fixed in 1e562dc1 / be55d9ae; full rerun on the final head, no step skipped: all PASS.
- Open (lane B): CCam::Process_DW_HeliChaseCam 0x51A740 and Process_DW_BirdyCam 0x51B850 (both fully specified in
  docs/re/specs/CAM/CCam.md, ~300 / ~500 instructions, BirdyCam with an SEH frame), the virtual VEH rows
  (CAutomobile::AddMovingCollisionSpeed 0x6A1ED0, DoHoverSuspensionRatios 0x6A45C0, CBike::BurstTyre 0x6BEB20, ...), the
  remaining WEAPON / CAM / VEH direct rows; a scenario that reaches the cinema camera.

## 2026-11-16 — FRONTIER-2 turns 4–9 on master; the porting round closes (main session)
- Integrated, in order, each with `tools/verify/writer-gate.sh` against master, the i686 sa-game / sa-oracle builds, every
  `tools/e2e/scenarios/frontier.sh` scenario and verify-all on lanes 1-7 (SA_UNVERIFIED on and the default set):
  - lane B turn 4 (heli; 0x6C4760 hand-ported for its native so patterns.py keeps one definition, rebase-duplicated
    bindings rows dropped), lanes A turns 4+5, lane C turn 5, lane B turn 5 (FlyingControl), lane B turn 6 (planes,
    CBoat::ProcessControl; one l1test run hit a Wine start-up artifact, the test passed alone), lane B turn 7 (boat
    physics, CMatrix::ScaleAll, SetTotalDamage): each IDENTICAL 13/13 in both modes.
  - lanes A turns 6–9 + lane C turns 6–8 + the heap fix 933c87cd as one stacked rebase (tag intac): IDENTICAL 13/13 in
    both modes; 115 new hooks. Fixes made while stacking: lane C's 0x59B9F0 binding now targets master's port, the
    0x71B620 binding keeps master's visibility with lane C's port as target, sa-ports no longer lists the
    replay_vehicle natives lane C removed, 0x5073C0 (SayPedless) bound once (`bindings.py --resolve`), and four
    lane A wrapper comments reworded so ai_patterns.py keeps the generated bodies they call (27166275).
  - lane B turn 8 (camera new-cam logic, FireOneInstantHitRound; tag intb8): IDENTICAL 13/13 in both modes.
- Scenarios on master af54a76b: every one runs with no crash; blocked steps only in roadblocks (1, 0x7CF240),
  roadblock-wanted (1, CObject::Explode 0x5A1340) and plane (392, 0x43E0E0).
- frontier.py --no-branches: 277 (start of the day) → 129 to port; 51 stay original.
- Tooling (session scratchpad): the rebase loop now merges catalog/bindings.csv by address (HEAD + the picked commit's
  own row changes) and regenerates bindings.rs, and unions frontier.sh's scenario lists.
- User-hit, fixed: peds without legs (random, on slopes) came from the glibc heap's high addresses in the play script
  (`--system-heap`); the play package no longer passes it and 933c87cd makes every run use the fixed region.
- target/play/sa-game and web/pkg (FEATURES=samp) refreshed. Per the user, no new porting turns: next is performance
  (PERF-1, RENDER-2), measured first.

## 2026-11-16 — PERF-0: per-frame measurement of sa-game, and the first reading (agent, branch worktree-agent-a52e8d3133fe02405)
- Base master b31ab3de (the worktree was created at 1f765b2a and reset to master). How to run it and the full
  tables: docs/perf/measure.md.
- Built (ADR-003: one implementation in the platform layer, phases around the ports, no platform cfg outside the
  backends): `sa_platform::clock` (one clock on `web_time::Instant`: std natively, `performance.now()` on the web;
  det.rs's private copy of `StdClock` removed), `sa_platform::perf` (exclusive phase scopes, counters, rows, CSV,
  summary; one relaxed atomic load per hook point when off), `sa_rw_wgpu::gpu_timer` + new `D3d9Stats` counters
  (pipeline / texture bind-group changes, buffer / texture uploads and bytes, streamed bytes), sa-game `perf.rs`
  (27 phases, 16 counters, `gpu_ms`) and `platform/perf_probe.rs` (46 registry probes: with the measurement on, the
  registry entries of `Game::Idle`, `CTimer::Update`, `CGame::Process`'s big callees and the render functions are
  wrapped by timing wrappers of the same ABI; typed on i686, one type-erased wrapper on wasm32). `--perf FILE` /
  `SA_GAME_PERF=FILE` natively, `?SA_GAME_PERF=N` on the web page (summary + CSV to the console as one message each:
  the worker keeps only 500 lines between status posts).
- Measured (i7-14700F, RX 7800 XT; in world, standing, limiter off with `SA_GAME_POKE=5:0xBA6794:0:1`; CSVs in
  target/perf/): native offscreen 2975 frames, mean 20.98 ms (47.7 fps), median 17.11, p95 30.92; web WebGPU 1500
  frames 29.69 ms (33.7 fps); web WebGL2 30.07 ms (33.3 fps); native with the default 25 fps limiter 40.24 ms (`loop`
  16.9 ms waiting); walking (`SA_GAME_WALK=100:2800`) 21.65 ms. Overhead: the same native run with the measurement
  off gave 20.24 ms mean / 18.49 median (the run-to-run spread of the alternating frames below), the `perf` phase
  itself 0.03 ms. GPU 0.8 ms per frame (timestamp queries; WebGL2 has none).
- Top costs (native, mean ms of 20.98): `idle.spin` 5.52 (`Game::Idle`'s 14 ms rule, the original's: it compares the
  time since the previous `CTimer::Update` with the previous frame's, so every second frame waits until it is 14 ms
  longer than the one before; the mean frame is the work + ~7 ms), `d3d9` 2.79 (~2,790 COM calls, 1,830 `Set*`),
  `logic.world` 2.77, `render.world` 1.83, `logic` 1.50, `render.list` 1.25, `render.shadows` 1.14,
  `render.coronas` 0.84 (p95 3.67: `CCoronas::Render` in ~20% of the frames), `render.prerender` 0.78,
  `render.plants` 0.50, `audio` 0.43, `render.hud` 0.36. Web: the same order, game code ~1.3x slower, plus `other`
  4.7 ms per frame outside the game (`web/worker.js` yields with `setTimeout(tick, 0)` after every frame longer than
  its 12 ms batch).
- Counters per frame (native): ~568 draws in ~17 passes, 134 pipeline and 396 texture bind-group changes, ~118
  buffer uploads of ~1.0 MB, ~1.17 MB streamed uniforms / constants / UP vertices, ~32,000 `orig!` registry
  dispatches, ~200 platform calls (~260,000 in the frames that spin: `QueryPerformanceCounter` polling).
- Not run: `--window` under the lane's headless gamescope: the i686 build needs the 32-bit libxkbcommon (X11: "Failed
  to load one of xlib's shared libraries"; Wayland: `XKBNotFound`), which this machine does not have.
- Verified: verify-all lanes 1-7 (tag `perf0-def`, head 683b8b47, default mode) IDENTICAL 13/13; offscreen runs with
  the measurement on: 0 blocked in 3016 / 616 iterations, the browser runs 0 blocked; with it off no perf output and
  no registry change. i686 clippy `-D warnings` (sa-game all targets), wasm32 clippy (sa-game lib), sa-platform tests
  24/24 (new: perf, clock), cfg 0 over baseline, dupes 0 new.
- Writer gate (base b31ab3de, head c0ae4432): all 18 steps PASS. Its queue step failed on master already (the writer
  branch of batches frontier2-b18 / b19 changed from "(no branch)" to master once they were merged); c0ae4432
  regenerates catalog/verify-queue.csv and docs/verify/QUEUE.md (no hook or status changed).

## 2026-11-16 — PERF-1 first optimisations: Idle 14 ms rule, worker yield, registry dispatch (agent)
- c2b94050 `perf(sa-game)`: with the frame limiter off (0xBA6794 == 0) and outside `--fixed-step`, sa-game's WinMain
  step sets `g_LastTickTime` (0xB72CA8) to now - 14 ms before rsIDLE, so the unchanged `Game::Idle` port does not
  spin (user decision 2026-11-16; ROADMAP divergence ledger, accepted). Limiter on: 1:1 (`idle.spin` 6.8 ms before
  and after). Phase A untouched; no corpus recording pokes 0xBA6794 (only save-load has header pokes).
- 1ff9632e `perf(web)`: web/worker.js yields with scheduler.postTask / MessageChannel instead of setTimeout(tick, 0).
- 22dc83c0 `perf(sa-core)`: registry lookup via an open-addressing hash index (was a binary search); call / miss
  counting only when on (`--perf`, `?SA_GAME_PERF`, `--report`); host stand-in lookup skipped through a compile-time
  key hash + 4096-bit filter (was a spin lock and a 512-slot scan on every unported call: the big cost).
- Native offscreen, limiter off, `--perf` (mean / median / p95 ms): base 22.11 / 20.17 / 34.50 -> after the Idle skip
  17.03 / 16.21 / 22.22 -> after the registry change 9.62 / 9.74 / 11.64 (104 fps). Without `--perf`: 14.96 / 17.03
  (Idle skip, two runs) -> 8.86 / 9.62 (registry). Limiter on: 40.24 / 40.27 / 40.23 (the 25 fps limit; the work
  shrinks, `loop` waits longer). Deterministic crossing scenario (1200 iterations, `--fixed-step 40`): the report's
  per-function dispatch counts are IDENTICAL base vs final (5423 functions, 56,995,849 calls); frame pacing 29.1 ->
  17.7 ms. Dispatches per frame rise in real-time runs (32k -> 39k, bimodal): timing-dependent original paths run
  more often at the higher frame rate.
- Web WebGPU (1500 frames, limiter off): base 28.08 ms, after the Idle skip 24.78 ms. The worker-yield and registry
  web runs and all WebGL2 runs were not completed (stopped at the usage limit).
- Not run (usage limit): writer gate, verify-all lanes 1-3, frontier.sh beyond `crossing`. sa-core tests pass (host
  75/75; i686 orig:: 6/6 incl. the new index test).
