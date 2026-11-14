# ADR-003: code architecture and patterns for the whole codebase

Status: accepted (user, 2026-11-14). Applies to every crate, generator and tool, and to every agent brief.
DEDUP-1 brings the existing code in line; SAMP-UI-DEDUP does the SA:MP UI part. ADR-001 (globals, layouts,
`orig!`, `#[hook]`) and ADR-002 (partial ports) stay in force; this ADR adds the structure around them.

## 1. Problem

The port grew fast, with many agents working in parallel in isolated worktrees and integration checks that
measured behaviour (L1–L3, verify-all) but not structure. The result (measured 2026-11-14):

- small helpers copied into many crates (`rd`/`wr` 19/18 crates, `g` 17, `x` 15, `find_player_ped` 15,
  `find_player_vehicle` 14, `entity_pos` 10, `stricmp` 9, `normalise` 9, `w32` 8, `model_info` 8, …);
- the same original function declared again and again (2,795 addresses have `orig!` declarations in more
  than one file; one is declared in 133 files);
- parallel implementations of the same thing: `sa-samp-ui` beside samp-rs's UI; the native front end
  (`game_window.rs`) beside the web front end (`web.rs`), each with its own input and frame handling (one
  mouse bug had to be fixed twice on 2026-11-14); the demo viewer stack (`sa-rw`, `sa-rw-wgpu`, `sa-viewer`)
  beside the ported RenderWare (`sa-rwcore`); `sa-formats` parsers beside the ported loaders;
- generators (`tools/gen/patterns.py`, `tools/gen/ai_patterns.py`) each with their own x86 interpreter and exe
  / catalogue readers;
- platform `cfg`s in game code (a wasm32 build broke when a new static took over `parse_args`'s `cfg`).

User decisions (2026-11-14): no duplicated code anywhere; abstraction wherever it applies.

## 2. The constraint that shapes everything

A ported function's **body** mirrors the original: the same operations in the same order, the same offsets,
the same x87 rounding, the same calls. That is what L1/L2/L3 verify. So:

- the patterns below go **around** the ports (how they reach data, other functions and the platform), never
  inside their logic;
- no refactoring of port bodies for style (extracting methods, reordering, "cleaner" control flow);
- no OOP hierarchies or ECS over the game's classes: the vtables are the original ones in `.rdata`.

The idiomatic rewrite of the game logic, if ever, comes after everything is verified.

### 2.1 Build flavours and memory (decision 2026-11-14)

- **compat (32-bit x86):** a real 32-bit process with the game's memory at the original addresses; it keeps
  binary compatibility with existing mods and the original samp.dll (MOD-1).
- **portable (x86_64, ARM, wasm32): pointer compression.** All game memory lives in one reserved 4 GiB arena;
  game pointers stay 32-bit offsets into it (the original addresses become offsets, e.g. `base + 0xB7CD98`), so
  every layout, offset and port body is unchanged and the corpus replays with the same hashes. Only the memory
  access layer (`sa-core::mem`: `GPtr`, `globals!`, the `field` accessors) adds the arena base. wasm32 already
  works this way (its linear memory is the arena). Details and the remaining work: ROADMAP STD-3.

## 3. Patterns

### 3.1 Ports and adapters: one implementation, one backend per platform

Everything platform-specific goes through a trait with one backend per platform: window and main loop, input,
audio output, file system, GPU, networking, the SA:MP UI renderer.

- The game loop, the input pipeline (3.6) and the SA:MP client are written once and are generic over those
  traits. `sa-game`'s native window and web page become two backends of one front end.
- samp-rs's UI draws through a renderer trait: the D3D9/DXUT backend on the original exe, the engine backend
  (CSprite2d / Im2D / CFont) in sa-engine (SAMP-UI-DEDUP).
- `cfg(target_arch = …)` / `cfg(windows)` / `cfg(sa_phase_a)` appear only in backends and in the binding layer
  (3.2), never in game code or front-end logic.

### 3.2 Single source of truth: everything defined once, by its owner

- **Original functions:** each address is declared once, in a generated binding module of the crate that owns
  the function (catalogue subsystem → crate), with its name, calling convention and signature. Other crates
  import it. A port, when one exists, is what the binding reaches (phases B/C) or what its hook switches to
  (phase A), exactly as `orig!` works today. This replaces the repeated per-file `orig!` blocks.
- **Globals:** one `globals!` declaration, in the owning crate.
- **Helpers** (C strings, x87 helpers, vector maths, entity position, model info, player lookups, raw memory
  accessors): one definition, in the lowest layer (3.4) that owns the concept (`sa-core`, `sa-math`,
  `sa-world`, `sa-stream`, …).
- **samp-rs** crates are reused, not reimplemented (samp-ui, samp-client, samp-proto).

### 3.3 Typed views over the original memory

Thin zero-cost wrappers generated from `catalog/layouts.csv` replace raw offset reads in new and touched code:
`Vehicle(GPtr<u8>)` with `.pos()`, `.autopilot()`, `.model_info()`, `Ped(GPtr<u8>)` with `.intelligence()`, …
The offsets are the original ones, so parity is unchanged; offset mistakes become type errors. `rd(ptr, 0x…)`
stays legal for fields the layout table does not know yet (then the field is added to the table).

### 3.4 Layers with one-way dependencies

`sa-math` → `sa-core` → `sa-rwcore` / `sa-stream` / `sa-anim` / `sa-input` → `sa-world` → `sa-pop` →
`sa-ped` / `sa-veh` / `sa-cam` / `sa-ai` / `sa-weapon` / `sa-render` → `sa-script` / `sa-hud` / `sa-audio` →
`sa-samp` / `sa-ports` → `sa-rw-wgpu` / `sa-hook` → `sa-game` / `sa-oracle` (the leaf crates `sa-macros`,
`sa-platform`, `sa-net`, `sa-samp-api`, `sa-oracle-format`, `sa-launcher`, `sa-udp-proxy` at the bottom). This is
the order the code has (measured 2026-11-14; `sa-pop` sits below the vehicle, camera and render crates, which call
its car-control code). A helper lives in the lowest layer that owns its concept. No cycles. `tools/lib/layers.py`
holds the order; the writer gate's `layers` step (`tools/verify/crate_layers.py`) fails a dependency on a crate of
the same or a higher layer.

### 3.5 Generation over copying

Whatever is repetitive (bindings, typed views, mechanical hooks) is generated from one source (the catalogue
and the exe bytes). Generators share one library (`tools/lib`: exe/section reader, catalogue reader, x86
decoding and the symbolic interpreter) instead of each carrying its own. Every generator has `--check`, run by
the writer gate. Generated output is never edited by hand.

### 3.6 One input and frame pipeline

Platform events → `InputState` → Win32 message queue and DirectInput state → the game, with one accumulation
rule (relative mouse motion and wheel accumulate until `GetDeviceState` reads them; keys pressed between frames
stay pending until a frame sees them) shared by every backend.

### 3.7 Errors and `unsafe`

- `unsafe` only in the memory-access layer (`sa-core::mem`, the arena) and in backends.
- No `unwrap()` / `expect()` in game code (allowed in tools and tests); typed errors (`Result`) at the platform
  boundary.

### 3.8 Tests by layer

- Ports: the oracle (L1/L2/L3) as now.
- New shared code (helpers, backends, generators, typed views): unit tests of its own.
- Generators: `--check` in the writer gate.

## 4. Enforcement

- Writer gate checks (DEDUP-1 pass 2): `bindings` (`tools/gen/bindings.py --check`: the generated modules match
  `catalog/bindings.csv`; no module declares a bound address again; no address is declared in two modules without
  a binding; C varargs, `bindings.VARARGS`, are the one exception: each caller passes its own argument list),
  `dupes` (`tools/verify/dupes.py --check`: no cross-crate copy beyond `tools/verify/dupes-allow.txt`, each line
  with its reason; the list only shrinks), `cfg` (`tools/verify/cfg_check.py`: platform `cfg`s outside the
  backends / binding layer only up to `tools/verify/cfg-baseline.txt`, which only shrinks), `layers` (3.4).
- Every agent brief states this ADR's rules: search and reuse before writing; shared logic in the shared place;
  never port an address another branch already registers (`tools/gen/inflight.py`).
- Integration reviews structure as well as behaviour.
- PLAN.md §1 F14 is the code-quality criterion (these checks with empty allow-lists).

## 5. Migration (DEDUP-1)

In this order, each step with builds, tests, the writer gate and verify-all IDENTICAL on all recordings:

1. shared helpers (3.2, 3.4);
2. one front end with native and web backends, one input pipeline (3.1, 3.6);
3. one binding per original address, generated (3.2, 3.5);
4. the viewer stack and `sa-formats` on top of the ported code, or retired where only the demo used them (3.1);
5. the shared generator library (3.5);
6. typed views, introduced crate by crate (3.3);
7. the enforcement checks (§4) and the PLAN.md criterion.

SAMP-UI-DEDUP covers the SA:MP UI part of 3.1 / 3.2 in parallel.

### 5.1 Binding rules (DEDUP-1 pass 2)

- **The signature comes from the original.** Where the declarations of an address disagreed, `bindings.py
  --resolve` took the convention and the stack arguments from the bytes (`tools/lib/abi.py`: the callee's `ret N`,
  whether it reads ECX, `add esp, N` at its call sites), a pointer where any caller passes one, `f32` where any
  passes a float, integers as wide as the callee reads them, and the result as the original's callers use it (AL,
  AX sign- or zero-extended, EAX, ST0). A binding that targets a port has the port's own signature.
- **Callers convert at the call expression** (`GPtr::from_addr(x as usize)`, `p.addr() as u32`, `f32::from_bits`,
  `x as u8`, `.cast()`, `.to_f64()`), never inside the port's logic.
- **`X87` results.** A `cdecl` / `stdcall` / `thiscall` binding returning `X87` (all 80 bits of ST0) is rendered
  as the equivalent `reg("args = …")` form, so ports returning `X87` are bound directly; the per-crate phase B
  adapters that rounded them to `f64` are gone.
- **Ported twins.** A binding sits in the lowest crate its callers share. When that crate is below the crate of
  the address's registered port (a lower caller reaches the port through the phase B registry), the CSV names the
  port (`port` column) and the generator also writes the binding into the port's crate with the port as target;
  callers that may depend on that crate import the twin, so phases B/C and host tests call the port directly.
  Both are generated from one CSV row.
- **Shared build script.** The port crates share `crates/phase_a_build.rs` (`build = "../phase_a_build.rs"`).

### 5.2 Step 4: the viewer stack (decided in DEDUP-1 pass 2)

Only the NATIVE-DEMO viewer used `sa-viewer`, `sa-formats` (its parsers), `sa-rw` (its RenderWare loader) and
`sa-rw-wgpu`'s world `Renderer`; nothing else depended on them. They are retired. `sa-rw-wgpu` keeps the D3D9 seam
(`d3d9`, `sm`) sa-game uses and the software DXT decoder (`dxt`, moved from `sa-rw`). The ported RenderWare
(`sa-rwcore`) renders the world through the seam.
