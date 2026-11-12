# sa-engine

A Rust reimplementation of GTA San Andreas (`gta_sa.exe` 1.0 US) that aims for 1:1 behaviour. It will later
also build for WebAssembly, and it will host the Rust SA:MP client from `~/samp-rs`.

## Read first
- `docs/PLAN.md`: strategy, oracle levels, definition of done.
- `docs/ROADMAP.md`: task board and decisions. Read it before working, update it after every step.
- `docs/WORKLOG.md`: append one entry per work step (what was done, how it was verified, cost).
- `docs/FEASIBILITY.md`: the original feasibility report (background; PLAN.md supersedes it).

## User decisions (2026-11-05)
- gta-reversed (`re/gta-reversed`, pinned at `20d8d2a`) **may be used as a source of code**: port its C++ to Rust.
  For functions it has not reversed (`.Reversed = false`, `plugin::Call*`, raw thunks, RenderWare), we
  analyse the original ourselves with REA/Ghidra.
- Scope: the **complete single-player game**. The user will later adapt it to samp-rs.
- Target binary: `gta_sa.exe` **1.0 US**, at `re/gta/1.0-US/gta_sa.exe` (sha256 `a559aa77…cbd26`).
  Ghidra snapshot: `re/snapshots/gta_sa_1.0US.snapshot`. `re/` is gitignored and holds our own copies.

## Rules
- **SA:MP client (changed 2026-11-12, user):** the SA:MP client samp-rs now lives in this repository, in `samp-rs/` (its own Cargo workspace, imported as a plain copy of `sa-engine/adapter` at fb89010, game screenshots left out). All SA:MP work happens here; the separate `~/samp-rs` repository (public) is no longer worked on from this project.
- Never commit or ship Rockstar files (exe, assets). They are read from the user's install. `catalog/`
  holds only addresses and names.
- Commits are made by the main session only: plain English messages, no attribution trailers, push only when
  the user asks.
- Every replaced function goes through the parity loop (PLAN.md §3). It is "verified" only after an oracle
  comparison against the original binary. A remaining difference must be recorded in the ROADMAP's
  divergence ledger, never left silent.
- Visual checks only on an isolated headless display (our own gamescope setup, modelled on samp-rs `tools/e2e/lib.sh`), never the user's desktop. Wine runs use our own prefix and game copy under `wine/` (gitignored), never `~/samp-rs/wine`.
- Keep ≥ 20 GB free on /home (`df -h`). Fixture and trace directories have quotas.
- Code, comments and docs are in English.

## 1.0 US specifics (verified, see catalog/summary.md)
- `.text` 0x401000–0x857000 is game code. A second `.text` at 0xCB1000 and `.HOODLUM` at 0x1556000 belong
  to the no-CD build.
- 501 `jmp` sites (497 catalogue functions) in game `.text` are 5-byte stubs into `.HOODLUM`, where their
  bodies live. 379 of them are gta-reversed hooks. The catalogue counts each once, at the stub (CAT-9). Game code reaches `.HOODLUM` only through these stubs (0 direct calls from game
  `.text`), so hooking the stub address is enough.

## Layout
- `tools/catalog/`: catalogue builders. Regenerate everything with **`tools/catalog/run_all.sh`**
  (deterministic: two runs give byte-identical `catalog/`; ~2 min with an analysed Ghidra project).
  Order: `build_catalog.py` → `merge_ghidra.py` → `classify_unknowns.py` (CAT-5, needs `re/plugin-sdk`)
  → `subsystems.py` → `callgraph.py ghidra` + `build` (CAT-7, Ghidra headless project in `target/ghidra`)
  → `callgraph.py gaps` (CAT-12: closes the catalogue over call targets, loops back when it adds rows)
  → `data_refs.py` (CAT-12b: functions reached only through data tables; same loop)
  → `callgraph.py fallthrough` (CAT-16: fall-through / mis-split decisions → `catalog/fallthrough-splits.csv`)
  → `names_extra.py` (CAT-6 names: gta-reversed comments, plugin-sdk, `rw-names.csv`, init/dtor, unknown vtables; `cat13_types.py` = typed virtual calls)
  → `layouts.py` → `script_commands.py` (CAT-10) → `gr_deviations.py` (CAT-14) → `progress.py` (CAT-11). Logs in
  `target/catalog-logs`. `vtables.py` = validated gta-reversed vtable slot names. `hoodlum_xrefs.py` is a
  one-off check. Python venv in `.venv`; run with `.venv/bin/python -I`. Notes: `docs/re/cat*.md`.
- `catalog/`: generated catalogue (`procedures.csv` = one row per function, `subsystems.md`,
  `summary.md`, CAT-7 CSVs, `script-commands.csv`, `progress.md`). Hand-maintained inputs:
  `catalog/state.csv` (per-function state, PLAN §5; `run_all.sh` merges it and never lowers a state above
  `specified`; record changes with `progress.py set ADDRESS STATE EVIDENCE`) and
  `catalog/extra-code-addresses.csv` (CAT-12 call targets, appended by `callgraph.py gaps`),
  `catalog/data-code-addresses.csv` (CAT-12b data-only entries), `catalog/gr-deviations-notes.csv`
  (CAT-14 hand-checked deviations).
- Per-function porting inputs: `catalog/gr-deviations.csv` (where gta-reversed differs from the
  original; ports follow the original), `catalog/port-order.csv`, `catalog/rwsets.csv`, `catalog/layouts.csv`.
- `crates/`: Rust workspace (created milestone by milestone, see PLAN.md §4). Every port follows
  `docs/arch/ADR-001-globals-and-layouts.md`: game state through `sa_core::globals!`/`GPtr`, calls to
  other catalogued functions through `sa_core::orig!`, registration with `#[hook]` + the crate's
  `hooks::ALL`. `tools/gen/layouts_rs.py` regenerates `sa-core/src/layouts.rs` from `catalog/layouts.csv`.
