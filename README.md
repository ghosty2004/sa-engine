# sa-engine

A function-by-function Rust reimplementation of **GTA San Andreas PC 1.0 US** (`gta_sa.exe`), checked
against the original game as it goes. Two goals:

1. **A native game:** `sa-game` runs on Linux without Wine and in the browser (WebAssembly).
2. **SA:MP:** the same engine joins SA:MP 0.3.7-R4 / open.mp servers through the Rust SA:MP client
   samp-rs (in [`samp-rs/`](samp-rs/)), natively and in the browser (through a WebSocket↔UDP proxy).

Nothing from Rockstar or SA:MP is in this repository. Every run reads **your own** game install (and your own
SA:MP client files); `gta_sa.exe` is checked by SHA-256.

**New here? Read [docs/GETTING-STARTED.md](docs/GETTING-STARTED.md).** It takes you from a fresh clone to a
running native game, the browser build and the verification tools, and explains how the project works.

| I want to… | Go to |
|---|---|
| set up the machine and run something in 30 minutes | [docs/GETTING-STARTED.md](docs/GETTING-STARTED.md) §2–§5 |
| understand the method (phases A/B/C, oracle levels) | [docs/GETTING-STARTED.md](docs/GETTING-STARTED.md) §1, [docs/PLAN.md](docs/PLAN.md) |
| port a function | [docs/GETTING-STARTED.md](docs/GETTING-STARTED.md) §7, [docs/verify/HOWTO.md](docs/verify/HOWTO.md) |
| verify ports against the original | [docs/GETTING-STARTED.md](docs/GETTING-STARTED.md) §8, [docs/verify/HOWTO.md](docs/verify/HOWTO.md) |
| know what is done and what is next | [docs/CONTINUE.md](docs/CONTINUE.md), [docs/ROADMAP.md](docs/ROADMAP.md), [catalog/progress.md](catalog/progress.md) |
| the project rules | [CLAUDE.md](CLAUDE.md), [docs/GETTING-STARTED.md](docs/GETTING-STARTED.md) §10 |
