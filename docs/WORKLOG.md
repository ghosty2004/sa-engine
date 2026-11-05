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
