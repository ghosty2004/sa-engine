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
- **Never write into `~/samp-rs`.** Another agent works there. This project does not depend on any of its
  paths. Reading its code or docs for reference is fine; changes there belong to that project.
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
- 501 functions in game `.text` are 5-byte `jmp` stubs into `.HOODLUM`, where their bodies live. 379 of them
  are gta-reversed hooks. Game code reaches `.HOODLUM` only through these stubs (0 direct calls from game
  `.text`), so hooking the stub address is enough.

## Layout
- `tools/catalog/`: catalogue builders, run in this order: `build_catalog.py re/gta-reversed re/gta/1.0-US/gta_sa.exe catalog`, `merge_ghidra.py catalog/raw/rea-evidence.json catalog`, `classify_unknowns.py re/gta/1.0-US/gta_sa.exe catalog` (CAT-5, needs `re/plugin-sdk`), `subsystems.py catalog`, `layouts.py re/gta-reversed catalog`, `callgraph.py ghidra` then `callgraph.py build` (CAT-7, Ghidra headless project in `target/ghidra`) (`hoodlum_xrefs.py` is a one-off check). Python venv in `.venv`.
  Run with `.venv/bin/python -I`.
- `catalog/`: generated `functions.csv`, `callees.csv`, `summary.md`.
- `crates/`: Rust workspace (created milestone by milestone, see PLAN.md §4).
