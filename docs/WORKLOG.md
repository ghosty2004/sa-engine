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
