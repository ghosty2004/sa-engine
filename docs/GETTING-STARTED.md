# Getting started with sa-engine

This guide is for a developer who has never seen the project. Read §1 to understand what you are looking
at, follow §2–§5 to get something running, then use §6–§10 as a reference while you work.

Contents:
1. [What this project is](#1-what-this-project-is)
2. [Machine setup](#2-machine-setup)
3. [Your game files](#3-your-game-files)
4. [Build](#4-build)
5. [Run it](#5-run-it)
6. [Repository map](#6-repository-map)
7. [How a function gets ported](#7-how-a-function-gets-ported)
8. [How ports are verified](#8-how-ports-are-verified)
9. [Where the work stands and what is next](#9-where-the-work-stands-and-what-is-next)
10. [Rules](#10-rules)
11. [Troubleshooting](#11-troubleshooting)
12. [Glossary](#12-glossary)

---

## 1. What this project is

sa-engine reimplements `gta_sa.exe` (GTA San Andreas PC **1.0 US**) in Rust, **one original function at a
time**, and proves each port behaves exactly like the original before it counts as done. "Exactly" means the
same results, the same memory writes and the same bugs: a port follows the original machine code, not a
cleaned-up idea of it. [gta-reversed](https://github.com/gta-reversed/gta-reversed) (pinned copy in
`re/gta-reversed`) is used as a reading aid; where it differs from the bytes, the bytes win
(`catalog/gr-deviations.csv`).

### The three phases

The same Rust code runs in three settings:

| phase | what runs | used for |
|---|---|---|
| **A: inside the original** | `gta_sa.exe` under Wine, with our DLL `sa-hook` injected by `sa-launcher`. Each ported function is installed as a hook over the original one; unported functions stay original. | Verification. Any port can be switched between Rust and the original at run time and compared. |
| **B: native** | `sa-game`, a 32-bit Linux executable with no Wine and no original code. It maps the exe's data sections at their original addresses (the "legacy arena"), and an address registry dispatches every call to the Rust port of that address. | The playable Linux game. |
| **C: other targets** | the same `sa-game` for `wasm32` (the browser). wasm32 pointers are 32-bit, so the arena keeps the original addresses there too. x86_64 needs native data layouts and is later work. | The browser build. |

In phase B/C an **unported** function cannot fall back to anything. Calling one "blocks" the current main-loop
iteration (native) or is caught and skipped (browser). Finding and porting those blockers is how the native
game was brought up.

### The verification levels

| level | meaning | tool |
|---|---|---|
| **L1** | a pure function gives the same result as the original on ~10^6 random inputs, run on the real CPU or Unicorn | `crates/sa-oracle` (`cargo test -p sa-oracle --test l1`, `sa-oracle l1`) |
| **L2** | inside real game runs, every call of the function gives the same outputs and memory writes as the original on the same inputs | `tools/e2e/port-l2.sh` (in-process double run, or "split" runs for functions with OS side effects) |
| **L3** | whole-game replays of recorded play sessions give identical per-frame state hashes (17 subsystems) with the ports on and off | `tools/e2e/l3.sh`, `tools/e2e/verify-all.sh` |

A port is **verified** at L2 or L3 (F1 progress in `catalog/progress.md`). Everything else is
**implemented** (written, compiled, not yet proven). See [PLAN.md](PLAN.md) §3/§5 for the full definitions.

### The pieces around the engine

- **The catalogue** (`catalog/`): every function of the exe (15,479 non-excluded), with name, subsystem,
  call graph and state (`catalog/state.csv`: unknown → specified → implemented → L1 → L2 → L3).
- **The oracle** (`crates/sa-oracle*`, `tools/e2e`): recordings, replays, state hashes, L1/L2/L3 runners.
- **SA:MP**: `crates/sa-samp-api` (the interface samp-rs uses to talk to "the game"), `crates/sa-samp` (that
  interface implemented on the engine), `crates/sa-samp-ui` (SA:MP's chat, dialogs, nametags, scoreboard drawn
  by the engine), `crates/sa-udp-proxy` + `crates/sa-net` (browser networking). The samp-rs side lives in
  `~/samp-rs` on branch `sa-engine/adapter` (see [samp/game-adapter.md](samp/game-adapter.md)).

---

## 2. Machine setup

Developed on Arch Linux x86_64. Anything with the same tools works.

**Required:**

| tool | why |
|---|---|
| Rust (stable, ≥ 1.90) via rustup, targets `i686-pc-windows-gnu`, `i686-unknown-linux-gnu`, `wasm32-unknown-unknown` | `rustup target add i686-pc-windows-gnu i686-unknown-linux-gnu wasm32-unknown-unknown` |
| MinGW-w64 (i686) | links the phase A DLL (`i686-w64-mingw32-gcc`) |
| 32-bit system libraries for the native build: `lib32-systemd` (udev), `lib32-alsa-lib`, and their pkg-config files in `/usr/lib32/pkgconfig` | `sa-game` is a 32-bit Linux process; `.cargo/config.toml` points pkg-config there |
| Wine (WoW64 build; 11.x is in use) + DXVK | runs the original `gta_sa.exe` for phase A and verification |
| gamescope, xdotool | headless, isolated displays for game runs (never your desktop) |
| Python 3 with a venv in `.venv` containing `capstone` and `pefile` | catalogue and verification tools (`.venv/bin/python -I …`) |
| ffmpeg | videos from captured frames |

**Optional:**

| tool | why |
|---|---|
| Node.js + Chromium | headless browser runs (`tools/web/headless.mjs`) |
| `wasm-bindgen-cli` **0.2.130** (must match the `wasm-bindgen` crate in `Cargo.lock`) | the browser build |
| 32-bit X11 client libraries (`lib32-libxcursor`, `lib32-libxrandr`, `lib32-libxi`, `lib32-libxinerama`, `lib32-libxrender`, `lib32-libxkbcommon`, `lib32-libxkbcommon-x11`) | `sa-game --window`. If you can't install them system-wide, extract them into a folder and put it on `LD_LIBRARY_PATH` (this is what the play package's `lib32/` is) |
| Ghidra 12.x, an analysed project in `target/ghidra` | regenerating the catalogue's call graph |
| ~60 GB free disk | each build tree is several GB; keep at least 20 GB free at all times (the e2e scripts refuse to run below that) |

```sh
python3 -m venv .venv && .venv/bin/pip install capstone pefile
```

---

## 3. Your game files

You need your own **GTA San Andreas PC 1.0 US** install. The exe must match
SHA-256 `a559aa77…cbd26` (full value in [CLAUDE.md](../CLAUDE.md)); other versions are not supported.

Layout (all of `re/` and `wine/` is gitignored; nothing in them is ever committed):

```text
re/gta/1.0-US/gta_sa.exe         reference copy of the exe (catalogue, REA/Ghidra, L1)
re/snapshots/…                   Ghidra snapshot
re/gta-reversed/                 gta-reversed, pinned at 20d8d2a (reading aid)
re/samp/client/                  your SA:MP 0.3.7-R4 client files (samp.dll, samp.saa, sampgui.png, …)
re/samp/server/                  an open.mp 1.6.7 server copy for local tests
wine/prefix/                     the Wine prefix (DXVK installed)
wine/gta-sa/                     the game copy: gta_sa.exe 1.0 US + data (incl. the install's vorbisFile.dll)
wine/lane-N/{prefix,gta-sa}      per-lane copies, created on demand by tools/e2e/lib.sh (prefix copy + symlinked data)
```

Set-up of the Wine side (INF-1, see ROADMAP): create `wine/prefix` (`WINEPREFIX=$PWD/wine/prefix wineboot`),
install DXVK into it, copy your game folder to `wine/gta-sa` with the 1.0 US exe. `source tools/wine-env.sh`
gives you the environment every script uses (prefix, game dir, DXVK without vsync, audio to a private null
sink).

The native and browser builds only need `wine/gta-sa` (they read the data files directly; no Wine).

---

## 4. Build

```sh
# host: unit tests, tools
cargo test -p sa-hook --lib                 # the registration checks (fast; run this a lot)
cargo test -p sa-core --lib
cargo test -p sa-script --lib

# phase A: the DLL and launcher injected into gta_sa.exe under Wine
cargo build --release --target i686-pc-windows-gnu -p sa-hook -p sa-launcher

# phase B: the native game
cargo build --release --target i686-unknown-linux-gnu -p sa-game
#   with the SA:MP client linked in:  … -p sa-game --features samp      (integration branch, §9)

# oracle tools (host)
cargo build --release -p sa-oracle-format -p sa-oracle    # satool, sa-oracle

# browser (integration branch, §9)
tools/web/build.sh                          # FEATURES=samp tools/web/build.sh for SA:MP
```

Use `-j8` and, on a shared machine, a memory cap: `systemd-run --user --scope -p MemoryMax=8G cargo …`.
Delete `target/debug` when you're done with it; the trees are large.

---

## 5. Run it

### 5.1 The play package (easiest)

`target/play/play.sh` (built by the native lanes, not committed) runs the latest native build in a window
with your keyboard and mouse:

```sh
target/play/play.sh
```

Enter through the menus. `--start-in-world` (on in the package) skips the intro mission; F enters/leaves cars;
`README.txt` next to it lists what works and what doesn't.

### 5.2 Native, offscreen (development)

```sh
B=target/i686-unknown-linux-gnu/release/sa-game
SA_GAME_PRESENT_GAME=1 $B --game wine/gta-sa --start-in-world --fixed-step 40 --frames 800 \
    --present-dir /tmp/frames --report /tmp/run.md
```

Useful options (all documented in [arch/platform.md](arch/platform.md) §13–§22):

| option | meaning |
|---|---|
| `--window` | real window, real input (needs the 32-bit X11 libs) |
| `--fixed-step MS` | deterministic mode: virtual clock, synchronous streaming, fixed heap; two runs are identical |
| `--start-in-world` | skip `START_MISSION 2` (the intro) — not 1:1, a development shortcut |
| `--present-dir DIR` + `SA_GAME_PRESENT_GAME=1` | save every in-game frame as PNG |
| `--report FILE` | step table + **blocker list** (unported functions the run hit) |
| `SA_GAME_KEYS=w:40:200,shift:300:304,…` | scripted keys `key:from:to` (iterations) |
| `SA_GAME_STUB=ADDR=RET/N,…` | exploration only: make an unported function return a constant |
| `SA_GAME_SCRIPT_SKIP=1` | skip unported script opcodes — not 1:1 |
| `SA_GAME_AUDIO=cpal` | real audio output natively |
| `--samp HOST:PORT --name N` (+ `SA_SAMP_DIR`) | SA:MP client (built with `--features samp`) |

The **blocker list** in the report is the main development loop of phase B: run a scenario, port what it lists,
repeat.

### 5.3 The browser

See `web/README.md` (integration branch). Short version:

```sh
tools/web/build.sh && tools/web/serve.sh --game wine/gta-sa      # localhost only
# open http://localhost:8642/?autostart=1   (&backend=gl forces WebGL2, &full=1 fills the window)
```

### 5.4 SA:MP in the browser

```sh
FEATURES=samp tools/web/build.sh
SAMP_DIR=re/samp/client tools/web/samp-demo.sh     # private open.mp + sa-udp-proxy + web server, all on 127.0.0.1
                                                   # (SAMP_DIR support: SAMP-FX branch; without it the SA:MP loading screen/skin fall back to GTA's)
# prints the URL, e.g. http://127.0.0.1:8644/?autostart=1&backend=gl&samp=127.0.0.1:7813&proxy=ws://127.0.0.1:7781&name=WebCJ
```

The proxy (`sa-udp-proxy`) relays WebSocket ↔ UDP; by default it only allows the one server it was started for.
See [samp/web-proxy.md](samp/web-proxy.md).

### 5.5 The original game under Wine (phase A)

Game runs always go through the e2e library (a locked lane, a headless gamescope display, teardown):
`tools/e2e/lib.sh`. Recordings: `tools/e2e/record-program.sh` with a program from `tools/e2e/programs/`
(format in `crates/sa-oracle-format/src/program.rs`); the corpus is in `target/e2e/corpus/*.sarec`
([e2e/corpus.md](e2e/corpus.md)).

---

## 6. Repository map

```text
crates/
  sa-core        globals!/GPtr (ADR-001), orig!, #[hook] runtime, address registry, timers, static initialisers
  sa-macros      #[hook] / #[native] / globals! proc macros
  sa-hook        phase A DLL: installer, hook table, determinism, the registration tests (run them!)
  sa-launcher    phase A launcher (starts gta_sa.exe suspended, injects sa-hook)
  sa-game        phase B/C executable: boot (WinMain), legacy arena, platform shims (CRT, Win32, DirectInput,
                 DirectSound, D3D9 device), window loop, deterministic mode, SA:MP host, web front end
  sa-platform    files, time, input, threads for phase B/C
  sa-rw-wgpu     the D3D9 → wgpu seam (WebGPU / Vulkan / WebGL2)
  sa-rw, sa-rwcore   RenderWare
  sa-math        vectors, matrices, x87 models (bit-exact FSIN/FCOS/FPATAN…)
  sa-stream, sa-world, sa-ped, sa-veh, sa-ai, sa-anim, sa-cam, sa-hud, sa-audio, sa-render, sa-pop,
  sa-script, sa-weapon, sa-input, sa-formats   the game subsystems (one crate each)
  sa-ports       links every port crate into one registry
  sa-oracle, sa-oracle-format   the oracle (L1 specs, recordings, satool)
  sa-samp-api, sa-samp, sa-samp-ui, sa-net, sa-udp-proxy   SA:MP (§1)
  sa-viewer      model/texture viewer
catalog/         the function catalogue (generated + state.csv, verify-queue.csv)
docs/            PLAN, ROADMAP, WORKLOG, CONTINUE, arch/ (design + per-lane notes), verify/, samp/, re/, e2e/
tools/
  catalog/       catalogue builders (run_all.sh)
  e2e/           game runs: lib.sh, record/replay, port-l2.sh, l3.sh, verify-all.sh, l2-extras.txt, programs/
  verify/        writer-gate.sh, queue.py, bisect-batch.sh, static checks
  gen/           code generators (patterns.py, ai_patterns.py, …) — their output is never edited by hand
  native/        phase B gap lists (mvp_gaps.py, samp_unported.py, static_inits.py)
  web/           browser build, dev server, headless driver (integration branch)
  wine-env.sh    the Wine environment
assets/          freely licensed assets only (e.g. Liberation Sans for the SA:MP UI)
re/, wine/, target/   local, gitignored (your game, prefixes, builds, recordings)
```

Key documents:

| doc | what's in it |
|---|---|
| [PLAN.md](PLAN.md) | method, oracle levels, definition of done (F1…F13), writer/verifier split |
| [ROADMAP.md](ROADMAP.md) | task board, user decisions, divergence ledger |
| [WORKLOG.md](WORKLOG.md) | one entry per integration step |
| [CONTINUE.md](CONTINUE.md) | **the current hand-off: unmerged branches and next steps** |
| [verify/HOWTO.md](verify/HOWTO.md) | the #[hook] attributes, writer gate, verifier procedure, hazards |
| [arch/ADR-001-globals-and-layouts.md](arch/ADR-001-globals-and-layouts.md) | how ports access game memory and call other functions |
| [arch/ADR-002-partial-ports.md](arch/ADR-002-partial-ports.md) | partially ported script switches |
| [arch/platform.md](arch/platform.md) | phase B: sa-game, the arena, the D3D9 seam, every native milestone (§13–§22) |
| [samp/game-adapter.md](samp/game-adapter.md), [samp/native-plan.md](samp/native-plan.md) | the SA:MP architecture |
| `docs/arch/*-port.md` | per-subsystem port notes and verdicts |

---

## 7. How a function gets ported

1. **Pick it.** `catalog/procedures.csv` + `catalog/state.csv` (state `specified`/`unknown`), or a blocker
   from a native run's report, or `docs/samp/native-unported.csv` for SA:MP. Check nobody else has it: grep
   `crates/` for the address, and look at open branches ([CONTINUE.md](CONTINUE.md)).
2. **Read it.** The bytes are the truth: disassemble from `re/gta/1.0-US/gta_sa.exe` (objdump, Ghidra or REA,
   read-only). gta-reversed helps to read; check `catalog/gr-deviations.csv`.
3. **Write it** in the subsystem crate, in a file named after its content (never after a task or agent):

   ```rust
   #[hook(addr = 0x6D1A40, conv = thiscall, name = "CVehicle::Foo", this = "CVehicle",
          expect = "83 EC 10 56 8B F1",
          unverified = "veh5-a", route = "L2", hazards = "heap-writes", recs = "driving")]
   pub fn vehicle_foo(this: GPtr<CVehicle>) -> u32 { … }
   ```

   - game memory through `globals!`/`GPtr` and `catalog/layouts.csv` (ADR-001);
   - calls to other original functions through `orig!` (ADR-001), never raw addresses;
   - **keep the original's bugs**; byte-faithful x87 order (`X87`/`sa_math::x87` for full-width values;
     `orig!` ST0 results are read as `f64`);
   - direct `rand()`/`srand()` calls go through `sa_core::orig::at_site(<return address>, …)` so determinism
     replays stay identical;
   - functions taking arguments in odd registers: `conv = reg("args = …; ret = …; keep = …")`;
   - entries the no-CD protection reads as data, or bodies too short to patch: `#[native(...)]` (phase B/C
     only) or a `keycells` reader — see [verify/HOWTO.md](verify/HOWTO.md).
   - Register it in the crate's `hooks.rs` list. New hooks are always `unverified = "<batch>"` with `route`
     and `hazards`; the macro refuses them otherwise.
4. **Gate it:** `tools/verify/writer-gate.sh` (clippy for host/i686/wasm32, builds, hook tests, L1), then
   `python3 -I tools/verify/queue.py`. `cargo test -p sa-hook --lib` must pass: it checks every registration
   against the exe (entry bytes, one registration per address, callers that read registers, protection reads).
5. **Commit** (plain English message, no attribution lines). A verifier later promotes it (§8).

Regular, mechanical functions (getters, destructors, thunks, small AI task methods) come from the generators
`tools/gen/patterns.py` and `tools/gen/ai_patterns.py`. Never edit their output; change the generator or its
exclude list (`ai_patterns_exclude.txt`, `exclude_inflight.txt`) and regenerate with `--check` clean.

---

## 8. How ports are verified

The full procedure is [verify/HOWTO.md](verify/HOWTO.md). In short, for a batch of unverified hooks:

```sh
SA_ROOT=$PWD tools/e2e/verify-all.sh TAG 4,5             # all verified ports + the batch on all 13 recordings
SA_UNVERIFIED=<batch> tools/e2e/l3.sh --rust unverified --fixed verified …   # L3 of one batch
tools/e2e/port-l2.sh --runs "plain l2 rust" …            # L2 per hook (split route: "plain split rust")
tools/verify/bisect-batch.sh …                           # narrow a divergence to one hook
python3 -I tools/verify/queue.py set …                   # record verdicts
```

A hook is promoted (its `unverified` attribute removed, its state raised in `catalog/state.csv`) when it has
L2/split evidence with 0 mismatches and 0 undeclared writes **and** an all-ports `verify-all` with it in the
default set is IDENTICAL on all 13 recordings. Known differences that cannot be removed (e.g. uninitialised
stack bytes the original copies into save files) go into the ROADMAP's divergence ledger, never silently.

Practical rules learned the hard way:
- L2 double runs are invalid for functions that mutate the heap or the OS; use the split route.
- `sa-oracle` embeds the hook table: rebuild it after promotions.
- Game lanes are `flock`ed (`target/locks/lane-N.lock`); the e2e scripts refuse to run with < 20 GB free.
- Integrations: bring each branch's **own** commits (cherry-pick or diff), union the lists (hooks, modules,
  `l2-extras.txt`, keycells READERS by address), keep one registration per address (verified beats
  unverified, hand port beats generated), regenerate generated files, then verify-all.

---

## 9. Where the work stands and what is next

**Read [CONTINUE.md](CONTINUE.md) first** — it is kept current at every hand-off. As of 2026-11-12:

- **master** (verified IDENTICAL on all 13 recordings): the native Linux game (walking, sprint, jump, fists,
  window loop, deterministic mode, traffic, shadows), ~84 % of the exe's functions written in Rust, 34 % verified
  at L2/L3, SA:MP read side, the proxy.
- **Integration branch `worktree-agent-ac8a20a070c956b08` (`f94f1ee3`, also verified 13/13)**: everything above
  plus the browser build, driving, climbing, SA:MP natively and in the browser (join, SA:MP loading screen,
  class selection, spawn, chat/dialogs/UI), browser audio. It still has to be combined with master's last
  verification commits — that is step 1 in CONTINUE.md.
- Other branches listed in CONTINUE.md hold the intro mission (New Game runs the real intro cutscene),
  streamed audio, script callees for the first missions, SA:MP skins/animations/weapons, more WORLD/weapon ports.

Live numbers: `catalog/progress.md` (`python3 tools/catalog/progress.py`), `catalog/verify-queue.csv`.

---

## 10. Rules

These are binding (from the project owner, see [CLAUDE.md](../CLAUDE.md) and ROADMAP decisions):

- **Never commit Rockstar or SA:MP files** (exe, data, textures, audio, `main.scm`, `samp.saa`, screenshots or
  recordings derived from them). They are read at run time from the user's install. Microsoft fonts too.
- **Byte-faithful 1:1.** Follow the original machine code, including its bugs. Any remaining difference goes into
  the divergence ledger.
- **Single player stays untouched by SA:MP**: SA:MP behaviour is enabled only in samp mode, through hooks at the
  same places samp.dll patches (see `docs/samp/startup.md`).
- **samp-rs**: write only on its branch `sa-engine/adapter` (worktree `~/samp-rs-adapter`); samp-rs must keep
  working with the original `gta_sa.exe`; every adapter step is a pure refactor checked by its tests and an A/B join.
- **Commits**: plain English, no attribution trailers. Pushing is done by the owner.
- **Code, comments and docs in English.**
- Visual checks only on isolated headless displays (gamescope lanes, offscreen), never on someone's desktop.
- Keep ≥ 20 GB free on the disk.
- New files are named after their content.

---

## 11. Troubleshooting

| symptom | cause / fix |
|---|---|
| `cargo test -p sa-hook --lib` fails "X overlaps X" / "unverified_selection" | an address is registered twice (two branches ported the same function). Keep one. |
| native run stops: "calls unported NAME (0xADDR)" / "the same failure three times in a row" | a phase B blocker: port that function (or stub it for exploration with `SA_GAME_STUB`) |
| `sa-game --window` fails to open | 32-bit X11 libraries missing (§2); use `LD_LIBRARY_PATH` to a folder with them |
| verify-all: every recording "no comparison line" | stale host `sa-oracle`/`satool`: rebuild them (`cargo build --release -p sa-oracle-format -p sa-oracle`) |
| verify-all skips recordings | the disk guard (< 20 GB free) or a held lane lock; free space and rerun |
| L1 test hangs in Wine at 0 % CPU | a stuck `ref.exe`; stop and rerun |
| an i686 build error about X87 returns in `orig!` | `orig!` ST0 results must be declared `f64` (register-convention `orig!` may return `X87`) |
| browser page stuck on black / "create_surface failed" | no WebGPU/WebGL2 in that browser profile; try `&backend=gl`, or enable WebGPU (Chromium: `--enable-unsafe-webgpu --enable-features=Vulkan`) |
| browser: SA:MP loading screen/skin missing | the page can't reach the SA:MP files: start the server with `--samp-dir`/`SAMP_DIR=re/samp/client` |
| replays diverge only sometimes | a `rand()` call without `at_site`, or an uninitialised read; see `docs/re/determinism.md` |

---

## 12. Glossary

| term | meaning |
|---|---|
| port | a Rust function replacing one original function (`#[hook]` or `#[native]`) |
| hook | the phase A installation of a port over the original's entry bytes |
| `#[native]` | a port used only in phase B/C (not hookable in the exe) |
| `orig!` | a typed call to another original function's address; dispatches to its port if one exists |
| arena | the exe's `.data`/`.rdata`/`.bss` mapped at their original addresses in phase B/C |
| batch | a group of unverified hooks one writer committed together; the verifier's unit |
| writer / verifier | writers port without running the game; verifiers prove batches on game lanes |
| lane | one isolated Wine prefix + headless display for game runs (`wine/lane-N`) |
| recording / corpus | a recorded input session replayed deterministically (`target/e2e/corpus/*.sarec`) |
| blocker | an unported function a native run reached |
| keycells | entries the no-CD protection reads as data; sa-hook keeps a copy so a hook doesn't break the protection |
| `.HOODLUM` | the no-CD build's code section; 501 functions start with a 5-byte jump into it |
| ledger | the list of differences from the original that are known, explained and accepted |
| samp mode | the engine options and hooks that reproduce samp.dll's patches, active only with `--samp` |
