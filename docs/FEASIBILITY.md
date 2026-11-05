# GTA San Andreas → Rust → WebAssembly: feasibility report

Date: 2026-11-05. Scope: `gta_sa.exe` 1.0 US, a Rust reimplementation, a browser build, and integration with
the existing Rust SA:MP client (`~/samp-rs`).

Evidence tags used throughout:

- **[V]** verified in this session (source inspected, command run, bytes read).
- **[R]** reported by a third party (README, blog, press). Not independently verified.
- **[A]** engineering assumption: reasoned, not measured.
- **[S]** speculative.

---

## 0. Verdict (short version)

1. **A complete, clean-room, 1:1 Rust rewrite of all of `gta_sa.exe`, done mostly autonomously, is not a
   realistic first target.** The engine has roughly 8,300 tracked game functions plus about 850 RenderWare
   functions, the CRT and the libraries **[V/A, §2.1]**. Most behaviour is stateful and chaotic: a
   one-ulp float difference grows into a different world within seconds. The community's best reconstruction
   (gta-reversed) has reached its reported 92% over many years of human work and still calls into the original
   binary at about 1,700 sites **[V]**.
2. **A Rust engine scoped to what SA:MP needs ("SA:MP profile") is feasible, and I could carry most of
   it incrementally,** using the same differential methodology as `samp-rs`. SA:MP turns off ambient peds and
   traffic, does not use the single-player campaign, cutscenes, saves, gangs, garages or stats, and replaces
   `main.scm` **[A: needs a census, §6.2]**. That removes a large share of the hardest and least testable
   code (the AI task/event system and mission scripting).
3. **The fastest way to get *behavioural* 1:1 in a browser is not a rewrite. It is static recompilation of the
   user's own `gta_sa.exe` to WebAssembly, plus a Rust HLE runtime (Win32 subset, D3D9 → wgpu, DirectSound →
   Web Audio).** The original code then *is* the behaviour, and the guest keeps the original virtual addresses,
   so the `samp-rs` memory-patch knowledge applies unchanged.
4. **Recommended plan: a hybrid "strangler".** Build the recompiled original as the oracle and the fallback.
   Replace subsystems with Rust one at a time, each replacement differentially tested against the recompiled
   original in the same process. In parallel, build the Rust-native layers that do not need the original at
   all: formats, RenderWare object model, wgpu renderer, platform layer, SA:MP integration. §5.4 compares the
   options and §8 gives the roadmap.
5. **Answer to the final question:** yes, I am willing, and I can do a large part of the work if it follows the
   methodology below. The conditions are in §7. They include human decisions on legal posture and scope,
   human review at milestone gates, and acceptance that "1:1" ends up as a **measured, bounded** claim, not a
   proven one.

---

## 1. What already exists locally (relevant inventory) [V]

| Item | State |
|---|---|
| `~/samp-rs/re/gta/1.0-US/gta_sa.exe` | 14,383,616 bytes, HOODLUM build, base 0x400000. It has **two** `.text` sections: 0x401000 (4.3 MB, game code) and 0xCB1000 (6.3 MB, protection/packer residue), plus `.HOODLUM`. |
| `~/samp-rs/re/gta-reversed` | Clone at `20d8d2a`, 2026-11-02. |
| `~/samp-rs/re/plugin-sdk` | Present. Class layouts and addresses for 1.0 US. |
| `samp-tracer` crate | An in-process DLL with an x86 decoder and inline hooks, plus D3D, BASS, SCM, memory-watch and coverage modules and a control channel. **This is about 60% of the instrumentation the plan needs.** |
| `tools/trace`, `tools/efx`, `tools/cov`, `tools/diff` | Call-trace diff, effect-log diff, D3D call-stream diff, coverage. Wine 11 + DXVK prefix, headless gamescope display. |
| REA + Ghidra 12.1.2 | Configured (the samp-rs snapshots exist). |

Address cross-check: I read 4 gta-reversed hook addresses (`0x7EE340` RwCameraClear, `0x809F90` RxHeapCreate,
`0x53E230`, `0x748710`) in the 1.0 US exe. All four land on valid function prologues **[V]**. gta-reversed's
addresses therefore line up with the 1.0 US code layout. Its "Compact exe" requirement (5,189,632 bytes)
probably comes from the protection section and not from different addresses **[A]**. A full-table check is
milestone M0.

---

## 2. Research: existing projects

### 2.1 gta-reversed (`github.com/gta-reversed/gta-reversed`) [V unless tagged]

- **What it is.** C++ (VS 2026, Conan, CMake) built as an **ASI DLL injected into the Compact exe**. Each
  reversed function is installed as a hook (`RH_ScopedInstall` ×6,250, `RH_ScopedVMTInstall` ×1,442,
  `RH_ScopedGlobalInstall` ×556, overloaded ×368). Hooks can be toggled at runtime ("reversible hooks").
  **Its architecture is the strangler pattern I recommend, already proven on this exact game.**
- **Progress.** `docs/hooks.json` lists 8,336 functions and 7,661 marked reversed (92%; the README text still
  says "50–60%"). The biggest gaps: `CCarCtrl` 26/110, `Interior_c` 0/52, `CCam` 20/45, `InteriorGroup_c` 3/24,
  `CStats`, `CTrain`, `CBike`, `CCamera` 97/114, `CPlayerPed` 50/65, `CRunningScript` 24/39, `CGarage`,
  `CMemoryMgr` 0/7.
- **What the 92% leaves out.** RenderWare is almost entirely *thunked*. `RenderWare/rw/*.cpp` contains 846
  raw calls such as `((RxHeap*(__cdecl*)(RwUInt32))0x809F90)(size)` and only 5 installed hooks. Across the
  whole tree there are 966 raw thunks and 742 `plugin::Call*` sites into the original exe. **A standalone
  build would need a RenderWare implementation**, and the hook statistics do not count it.
- **Code size.** 1,030 `.cpp` files, about 414k lines of `.cpp`/`.h`.
- **Licence.** **No LICENSE file in the repo.** By default that means all rights are reserved by the
  contributors, and the code is itself a reconstruction of Rockstar's code. **Porting it line by line to Rust
  produces a derivative work, and it conflicts with the `samp-rs` clean-room rule.** This is a human decision
  (§7.2).
- **Reuse value.** Very high as a *map*: names, class layouts, vtables, call graphs, address→function names
  and subsystem boundaries. The hook-toggle technique is the model for the in-process differential harness.
  `docs/ScriptCommandReversing.md` and the 1,824 `COMMAND_*` identifiers cover the SCM surface.

### 2.2 web-gta (`github.com/Sir-Teo/web-gta`) and the teozeng.dev post [V]

- **TypeScript**, 780 `.ts` files and about 92k lines. three.js 0.186 for rendering, **Rapier** (WASM) for
  physics, Vite, Vitest and Playwright. **No emulator, no recompiler, no Rust.**
- Gameplay is "ported from gta-reversed". Where gta-reversed has stubs, logic comes from the **iOS
  executable's disassembly** (for example `CVehicle::FlyingControl`, steering curves, touch layouts).
- **Fidelity, in the project's own words:** "gameplay controllers are authored approximations unless
  documented otherwise". Physics runs on Rapier, not on the original `CPhysical`/`CWorld::Process` collision
  response, so **it is not behaviourally 1:1** and cannot be made 1:1 without replacing the physics.
- It runs the original `main.scm` with its own interpreter. The census reports "1,570 distinct commands" used,
  127 not ported, and unported commands are skipped instead of being fatal.
- **It redistributes Rockstar assets publicly.** `public/` holds 904 MB (2,922 `.bin` chunks plus mp3, dff,
  txd, ipl and ide files), served from GitHub Pages. There is no LICENSE file. **Treat it as a legal
  liability: read it for research ideas only, copy nothing.**
- **Reusable ideas:** the browser streaming design (worker-side chunk inflate, Cache Storage keyed by hash,
  look-ahead windows, "streaming hold"), DXT1 → RGB565/5551 CPU decode on GPUs without S3TC, the adaptive
  resolution ladder, the headless "mission sweep" harness, and the script command census tool. These are
  measured browser lessons, not compatibility work.

### 2.3 OpenSA (`github.com/Avatarchik/opensa`) [V]

- TypeScript, about 439 `.ts` files, three.js, Rapier, bitecs (ECS) and React. **AGPL-3.0.** Last commit
  2026-06-25. The README says it was built "solo with Claude, in 3 weeks".
- A from-scratch "RenderWare-compatible" engine with **user-supplied assets** (the right legal posture). Its
  physics and gameplay are its own designs, so it has **no behavioural compatibility**.
- **Reuse:** loaders (DFF, TXD, COL, IMG, IPL, IDE) as a cross-check reference only. The AGPL means none of its
  code can go into a non-AGPL Rust project. Its BYO-assets UX and VFS design are useful to study.

### 2.4 v86 (`github.com/copy/v86`) [V]

- A full-system x86 PC emulator that JIT-compiles x86 to WASM at runtime. Rust core plus JS. The README lists
  missing features: some floating-point/SSE exceptions and **multicore**. It emulates **a VGA/SVGA card with no
  3D acceleration**. `docs/how-it-works.md` states that **x87 is emulated with softfloat ("very slow")**.
  Windows XP and later work only "under certain conditions".
- **Running GTA SA inside v86 is not feasible.** The game needs Direct3D 9 hardware T&L and uses x87 heavily,
  and v86 has no 3D path and runs single-core.
- **Reuse:** its JIT design notes (block discovery, wasm module generation, page-granular invalidation) are
  useful prior art for a *user-mode* translator. They are not needed for a static recompiler.

### 2.5 Other relevant work

| Project | Relevance | Tag |
|---|---|---|
| **re3 / reVC** (GTA III / VC reconstructions) + **librw** | Same engine generation. librw is an open reimplementation of RenderWare with D3D9, GL3 and PS2 backends and the most direct reference for an `sa-rw` crate. **Take-Two sued in Sept 2021. The case settled in 2023 (dismissed with prejudice for 4 named defendants, without prejudice for the rest), terms undisclosed.** | [R] [gamesradar](https://www.gamesradar.com/gta-owner-settles-lawsuit-against-creators-of-vice-city-reverse-engineering-project-before-case-reaches-court), [torrentfreak](https://torrentfreak.com/take-two-dismisses-claims-against-lead-defendants-in-gta-mods-lawsuit-230405/) |
| **DOS Zone reVC/re3 browser ports** | Proof that a *source-level reconstruction* of this engine family compiles to WASM (Emscripten FS, `index.wasm`) and runs at playable speed in the browser. **It got a DMCA takedown in 2025.** Forks still exist. | [R] [Tom's Hardware](https://www.tomshardware.com/video-games/gta-vice-city-and-gta-iii-return-to-web-browsers-after-a-dmca-takedown-open-source-reverse-engineering-brings-the-classics-to-the-web-with-over-100-fps-performance), [club386](https://www.club386.com/play-gta-vice-city-browser/) |
| **Static recompilers** (N64Recomp, XenonRecomp, xboxrecomp, PSXRecomp) | Evidence that fixed-binary game recompilation to C, and then native or WASM, is practical. xboxrecomp targets **x86** (Xbox) → C. "Pepsiman Recompiled" is reported as a WASM-first recompilation running at 60 fps in a tab. **I found no off-the-shelf Win32 x86 → WASM recompiler.** | [R] [xboxrecomp](https://github.com/sp00nznet/xboxrecomp/wiki), [heldgames](https://heldgames.com/guides/recompiled-games-list) |
| **plugin-sdk** (DK22Pac) | 1.0 US layouts and addresses. Already in `re/`. | [V present] |
| **Android/iOS builds (War Drum port)** | Shipped with symbols. web-gta uses them to fill gaps. Behaviour differs from PC 1.0 (touch, rendering, some fixes), so **they are useful for names, not as a PC 1.0 oracle**. | [R] |
| SkyGFX, SilentPatch | Document known renderer and gameplay bugs in PC 1.0. Useful for a "known divergence" ledger. | [A] |
| Wine, DXVK, Frida, DynamoRIO (drcov), Unicorn, Remill/LLVM, Ghidra BSim | Tracing, coverage, leaf-function emulation, lifting. | [A] |

---

## 3. Feasibility of full reverse engineering

### 3.1 Hardest technical obstacles (ranked)

1. **Chaotic state plus floating point.** The original is MSVC x86 code using **x87**. D3D9 `CreateDevice`
   without `D3DCREATE_FPU_PRESERVE` switches the x87 control word to 24-bit precision **[A: well known D3D9
   behaviour; verify by reading the control word under Wine]**. Rust `f32` on SSE then matches *most*
   arithmetic, but not: intermediate exponent range, `fsin`/`fcos`/`fpatan`/`fsqrt` versus libm, the
   rounding-mode of float→int conversions (`_ftol`), or the order in which the compiler kept values in
   registers. Bit-exact physics in Rust requires per-function care. Some functions need a deliberately
   x87-faithful helper library (`sa-x87`). Bit-exact under WASM is easier than native, because WASM float
   semantics are deterministic apart from NaN bits.
2. **Hidden coupling through globals and pools.** Hundreds of global singletons (`CWorld`, `CStreaming`,
   `CTimer`, `CCamera`, `CPools`, `TheCamera`, `CPad`…) are touched from everywhere. Object identity is the
   **pool slot index**, and SA:MP depends on it: `CPools::GetPedRef` handles appear in script opcodes. The Rust
   architecture must keep pool semantics (slot allocation order, flags byte, handle = index<<8 | flag), even
   if the Rust types are idiomatic.
3. **RenderWare.** Rendering, plugin data (skin, HAnim, MatFX, 2dfx, breakable, night-colours) and the
   streaming formats all go through it. It must be reimplemented (§2.1 shows gta-reversed did not).
   Object-model compatibility is needed. GPU output only needs to be *visually* equivalent.
4. **AI tasks and events** (`CTask*`, `CEvent*`, `CPedIntelligence`, decision makers). Hundreds of classes in
   a deep virtual hierarchy, heavily stateful. This is the hardest area to test differentially.
   **Mostly unnecessary for SA:MP** **[A]**.
5. **Threads.** The CdStream reader thread and the audio streaming threads introduce nondeterminism. The
   harness must make them synchronous.
6. **Undefined behaviour in the original.** Uninitialised reads and out-of-bounds pool accesses. The
   original's behaviour then depends on heap contents. These go into a divergence ledger; reproducing them is
   not feasible.
7. **Legal.** See §7.2.

### 3.2 Reconstructing the architecture

- **Static:** Ghidra/REA with names and types imported from plugin-sdk and gta-reversed (as facts about the
  binary: addresses, sizes, vtable slots). RTTI is mostly absent, so vtables come from constructor analysis.
  Output: a machine-readable **catalogue** (`catalog/functions.toml`: address, name, calling convention,
  signature, subsystem, callers, globals read and written). The globals read/write sets come from a
  data-flow pass in Ghidra. This catalogue drives everything else: the coverage denominator, the port order
  (leaves first, by the call graph), and the hook generation.
- **Dynamic:** the samp-tracer approach extended to whole-game function-entry coverage. Per-function
  read/write sets of globals and pool memory are recorded via page-guard watch or DynamoRIO.
- **Memory management:** `CMemoryMgr` (0/7 in gta-reversed), pool allocators (`CPool<T>`), RW freelists and
  streaming memory accounting (`CStreaming::ms_memoryUsed`). The streaming budget is behaviour-relevant
  because it decides what gets evicted, so the accounting has to be replicated even with a Rust allocator.

### 3.3 Behavioural compatibility without the x86 ABI

**Yes for the end state. No during the transition.** While Rust and original code share one process
(strangler), Rust must use `#[repr(C)]` layout-identical structs, `thiscall`/`stdcall`/`cdecl` shims and the
original vtable layouts. After a subsystem has no remaining original callers, its internals can be refactored
freely behind a stable façade. That gives two layers per subsystem: `compat` (layout- and ABI-faithful, used
for hooks) and `core` (idiomatic, used once the subsystem is fully replaced).

### 3.4 What must be exact vs what can be modern

| Must be exact (observable by servers, scripts, data or players) | Can use modern Rust abstractions |
|---|---|
| File formats (IMG, RW streams, COL, IPL/IDE, IFP, SCM, DATs, handling.cfg, timecyc) | Allocator, I/O, threading, async streaming |
| Model, weapon, vehicle, anim and skin IDs and names; pool handle semantics | Containers, ECS-style iteration *if order is preserved where it matters* |
| SCM opcode semantics (SA:MP's `main.scm` and samp.dll's own opcode use) | Renderer internals: GPU pipeline, shaders, batching (visual equivalence only) |
| Physics integration, collision response and handling (other SA:MP players see it; vehicle sync carries velocity) | Audio mixing backend (results need to sound right, not be sample-exact) |
| Camera maths that affects aiming and sync (aim sync packets) | Input layer (map to `CPad` state exactly, gather input any way) |
| `CTimer` step logic, frame-limiter behaviour, `rand()` LCG where gameplay uses it | Logging, debug UI, tools |

"Exact" means bit-exact in deterministic replay where achievable, otherwise within a stated, recorded
tolerance.

---

## 4. Differential tracing architecture

### 4.1 Oracle levels (cheapest and most precise first)

| Level | Oracle | Harness | Equivalence shown |
|---|---|---|---|
| **L0 Formats** | Original files plus the original's *parsed in-memory structures* (dumped after load under Wine) | Rust parse → compare against the dump. Byte-exact round-trip of every file in the install | Exact for all shipped files |
| **L1 Pure functions** | The original function bytes run in **Unicorn** on Linux (no Wine), with stub memory for referenced globals | Property-based fuzzing: same random inputs → compare outputs and written memory, ≥10⁶ cases each, plus edge cases (NaN, ±0, denormals, large angles) | Exact on the sampled input space |
| **L2 Stateful functions** | The original running in-process under Wine, with gta-reversed-style toggleable hooks | **Snapshot → run original → capture → restore → run Rust → capture → diff.** Snapshots cover the globals and pools in the function's read/write set (from the catalogue), with pointers normalised | Exact for recorded real-game states |
| **L3 Lockstep replay** | The whole game in deterministic mode | Recorded inputs (`CPad` states per frame) → per-frame **state hashes** per subsystem (entity transforms, velocities, pool occupancy, script vars, streaming state). On the first divergent frame, bisect automatically down to the subsystem and then the function (by switching hooks back to original) | Exact along the recorded trajectories |
| **L4 Presentation** | D3D9 call stream (already in `tools/trace/d3d_diff.py`), framebuffer captures, audio event logs (`efx`) | Draw-call semantic diff (geometry, texture, state). Image diff with perceptual tolerance. Audio *event* diff, not waveform | Visual and audible equivalence within tolerance |
| **L5 Network** | The original samp.dll on the original gta_sa | Existing `samp-dump` diff of the packet stream for the same scenario | Server-observable equivalence |

### 4.2 Determinism mode (prerequisite for L2/L3)

Hooks in the original **and** equivalent switches in Rust:

- `CTimer`: a fixed `timeStep` and fixed `ms_nTimeInMilliseconds`. `timeGetTime`/`QueryPerformanceCounter`
  return virtual time.
- `rand()`/`srand()`: the MSVC LCG with a recorded seed. Calls are logged with their caller, so an extra or
  missing `rand()` call is caught *where it happens* instead of 300 frames later.
- Streaming: the CdStream thread becomes synchronous ("load immediately"), so IO timing cannot change what is
  resident.
- Audio threads are disabled or made deterministic. Audio does not feed back into gameplay **[A, verify]**.
- Input: replay `CPad` (`NewState`/`OldState`) per frame, not OS input.
- The heap is seeded and deterministic. Under Wine, use a fixed allocator hook so uninitialised-memory
  contents are reproducible.

### 4.3 Normalisation

- Pointers → `(region, index, offset)`: pool pointers → `(pool, slot, field)`, model info → model ID, RW
  objects → `(type, creation ordinal)`, static data → symbol+offset, heap → allocation ordinal.
- Floats: bit-exact by default. Tolerance is allowed only per field, with a *recorded reason* in the
  divergence ledger.
- Time: virtual only. Thread interleavings are removed by determinism mode, not normalised away.
- **Every normaliser is a reviewable rule in one file**, and the harness's mutation tests (§6.4) check that
  normalisers do not hide real differences.

### 4.4 Reproducible regression tests

Every divergence found becomes a fixture: snapshot plus inputs plus expected output, stored compactly
(zstd, content-addressed). Each test records the original-exe hash it was generated from. CI runs L0, L1 and
L2 fixtures natively with no Wine (the original-side outputs are stored). L3 trajectories are re-recorded
nightly against the original.

### 4.5 Coverage

- **Function coverage:** catalogue functions entered in the original during the scenario corpus, against
  functions replaced by Rust and verified at L1–L3.
- **Basic-block and edge coverage of the original** (DynamoRIO drcov under Wine, or a Frida Stalker
  fallback), mapped to the Rust implementation: an original branch never exercised marks the matching Rust
  branch "unverified".
- **SCM opcode coverage** (per opcode and per parameter type), **file coverage** (every file and record parsed
  and compared), and **packet/RPC coverage** (already in samp-rs).
- Reported per subsystem as *verified / implemented-unverified / missing*. Nothing counts as verified without
  an oracle comparison.

### 4.6 What testing can and cannot show

- **Can show:** exact equivalence for every *input actually exercised*. Bounded equivalence (exact up to frame
  N) along recorded trajectories. Coverage numbers stating *how much* of the original's behaviour was ever
  compared.
- **Cannot show:** equivalence on unexercised branches or states (state space is effectively unbounded);
  behaviour that depends on UB or uninitialised memory; long-horizon equivalence once floats diverge by 1 ulp
  (chaos amplifies it, so exact equivalence after a divergence is meaningless and the test has to re-sync);
  timing-dependent behaviour under real frame rates; GPU and driver-dependent output; and interaction with
  third-party mods. **"1:1" can only ever be stated as: "exact on corpus C, coverage X%, known divergences
  listed in ledger L."**

---

## 5. Rust engine architecture

### 5.1 Workspace layout

```
sa-platform    traits: Fs (async read, OPFS/native), Clock, Input, AudioOut, Net, Spawn, Gpu surface
               impls: native (winit, cpal, std::fs, UDP) | web (web-sys, OPFS, AudioWorklet, WebSocket/WebTransport)
sa-formats     pure, no_std+alloc: IMG, RW chunk stream, TXD, DFF, COL1-4, IPL (text+bin), IDE, IFP, SCM,
               handling.cfg, timecyc, carcols, water.dat, nodes*.dat, surfinfo, pedstats, weapon.dat ...
sa-math        CVector/CMatrix/CQuaternion with exact semantics; sa-x87 helpers where needed
sa-rw          RenderWare object model: frames, atomics, clumps, geometry, materials, plugins (skin, HAnim,
               MatFX, 2dfx, night colours, breakable); no GPU code
sa-render      wgpu backend: D3D9 fixed-function emulation as WGSL, the GTA SA pipelines (building, vehicle,
               skin, water, post-FX), sprites/HUD/fonts; WebGPU + WebGL2 (wgpu GL) + Vulkan/Metal/D3D12
sa-pools       CPool<T> semantics (slot order, flags, handles), typed handles
sa-world       CWorld sectors/lists, CEntity hierarchy as enum + traits, model infos, CStreaming (budget,
               priorities, eviction), CTimer, CClock, CWeather, zones
sa-physics     CPhysical/CWorld::Process port, collision (CCollision), not Rapier
sa-anim        CAnimManager, blend associations, IFP clips, HAnim skeletons
sa-vehicle     CVehicle/CAutomobile/CBike/CBoat/CHeli/CPlane/CTrain, handling, damage manager
sa-ped         CPed/CPlayerPed, weapons, tasks needed by the player (SA:MP profile first), later AI
sa-scm         interpreter (CRunningScript), opcode table generated from the catalogue, handlers per subsystem
sa-audio       CAESoundManager/entities/banks; decoders; mixing to sa-platform::AudioOut
sa-hud         HUD, radar, fonts, text, menus
sa-game        main loop (Idle/RenderScene ordering), frame limiter, game states
sa-compat      (Windows only, transition) repr(C) layouts + hook shims to run Rust code inside gta_sa.exe
sa-oracle      harness: Unicorn L1 runner, snapshot/restore, normalisers, hashers, fixture format
samp-*         existing crates; new `samp-engine-adapter` trait (see 5.3)
```

The rule for native and WASM with minimal duplication: **only `sa-platform` and the thin `app-native` /
`app-web` binaries know which target they run on.** Everything else is `#![no_std]`-friendly or plain `std`
without OS calls. Async IO goes through the platform trait. The game loop is single-threaded and
deterministic, matching the original. Worker threads are used only for IO, decode and audio.

### 5.2 Notes per subsystem

- **World/entities:** keep the C++ hierarchy as data, an `Entity` header plus `enum Kind { Building, Dummy,
  Object, Ped(Box<Ped>), Vehicle(Box<Vehicle>) }`, and keep the update *order* (CWorld lists, sector scan
  order). Order affects collision resolution and `rand()` consumption.
- **Physics:** port the original algorithm. Rapier is not an option for compatibility (web-gta shows the
  outcome: approximation).
- **Rendering:** it never needs bit-exactness. Target the PC 1.0 look, with SkyGFX's research as a guide.
  Compressed textures: use BC/DXT on desktop (WebGPU `texture-compression-bc`), and transcode to ETC2/ASTC or
  decode to 16-bit where it is missing (the web-gta measurements show this matters on mobile).
- **Audio:** the logic (which sound, when, volume, position) is compared at the event level. Mixing is
  modern. SFX bank decoding and the radio stream format must be exact.
- **SCM:** the interpreter core (control flow, variables, timers, parameter decoding) is small and fully
  testable at L1/L2. The opcode handlers are the long tail.

### 5.3 SA:MP integration

Today `samp-rs` targets gta_sa.exe through addresses and memory patches (`docs/re/subsystems/gta-patches.md`).
Introduce a `GameAdapter` trait (spawn/remove ped and vehicle, set position/velocity/health, apply anim,
create object, textdraw, checkpoint, 3D text, camera, pad state in/out, script opcode call). It has three
implementations:

- `adapter-gta-exe`: the current approach.
- `adapter-recomp`: the same addresses, inside the recompiled image.
- `adapter-rust`: native API calls into the Rust engine.

The sync and RPC code stays shared. The parity loop then becomes: *same server scenario, original
samp.dll/gta_sa vs samp-rs/Rust engine, with the packet diff empty and the effect diff empty.*

---

## 6. Browser and WASM

### 6.1 Platform features [A unless marked; check versions at implementation time]

| Need | Approach |
|---|---|
| Rendering | wgpu → WebGPU (Chromium stable; Firefox and Safari have shipped it on some platforms) with a WebGL2 fallback through wgpu's GL backend. The game runs in a **Worker with OffscreenCanvas** so the main thread never blocks. |
| Audio | AudioWorklet, fed by a SharedArrayBuffer ring buffer from the game worker. |
| Files | **The user supplies their own game files** (directory picker via the File System Access API on Chromium, or drag-and-drop/zip), copied into **OPFS**. Reads use `createSyncAccessHandle` in a worker, which matches the original synchronous CdStream model. Nothing from Rockstar is hosted. |
| Threads | wasm threads need SharedArrayBuffer, which needs **cross-origin isolation** (COOP/COEP headers). Rust needs `+atomics,+bulk-memory` and `build-std` (nightly) or a stable toolchain that supports it at that point. `Atomics.wait` is not allowed on the main thread, which is another reason the game loop lives in a worker. |
| Memory | wasm32 gives up to 4 GiB. The original fits in a 2 GiB 32-bit address space with a ~50 MB default streaming budget, so this is not a constraint. memory64 is not needed. |
| Input | Pointer Lock, KeyboardEvent `code` (layout-independent), Gamepad API → `CPad` state. Touch is out of scope initially. |
| Streaming | IMG archives are random-access, so OPFS sync reads are enough. No chunk CDN is needed (because of BYO assets). |
| Networking | No UDP in browsers. Option A: **WebSocket ↔ UDP gateway** run by the user locally (one small Rust binary). Option B: WebTransport datagrams to a gateway (closer to UDP semantics; needs HTTP/3 and a certificate). Option C: Direct Sockets (Chromium Isolated Web Apps only). A *shared, hosted* gateway makes every player appear from the gateway's IP, which breaks per-IP limits and bans on SA:MP servers. **The local gateway is the honest default.** RakNet reliability already lives in `samp-proto` (sans-IO), so only the datagram transport changes. |

### 6.2 Scope reduction for SA:MP (needs a census, milestone M0)

SA:MP does not use: the story missions, cutscenes (mostly), the save system, ambient ped/traffic population,
gang wars, garages and mod shops (partly), stats-driven progression, or most of the AI task/event system
(remote players are puppets driven by sync packets). It does use the player ped tasks, weapons, vehicles of
all types, physics, objects, pickups, checkpoints, radar/HUD/textdraws, interiors, camera, anims, audio,
weather/time and the streaming of the whole map. **[A]** The real list comes from running samp.dll under the
coverage tracer through representative servers and recording which game functions are ever entered.
`tools/cov` already does this for samp.dll. Extending it to gta_sa functions gives the SA:MP-profile
denominator.

### 6.3 Translation vs rewrite vs hybrid

| | **A. Full Rust rewrite** | **B. Static recompilation + HLE** | **C. Hybrid strangler (recommended)** |
|---|---|---|---|
| Time to first playable in browser | Longest. Every subsystem has to exist first | Shortest: playable once recompiler, Win32 HLE, D3D9→wgpu and DSound→WebAudio work | Same as B for "playable". Rust share grows over time |
| Behavioural fidelity | Only as good as the tests | Near-exact by construction (the original code), apart from x87 emulation choices | Exact where still recompiled. Tested where replaced |
| Test oracle | Separate Wine process | Built in | **Built in, in-process, same address space**: L2 snapshots become cheap and run under the browser runtime too |
| SA:MP integration | New adapter | The original addresses keep working | Both |
| Main risks | Scope, endless chaos-divergence debugging | Indirect-branch discovery, x87 semantics cost, D3D9 FF emulation, Win32 surface, the protection section (use a clean exe) | The sum of B and part of A |
| Legal | Best case if genuinely clean-room | Recompiled output *is* Rockstar code. It must run **only locally from the user's exe** (recompile in the user's browser or tool, never distributed) | Same as B for the recompiled part |
| Performance in browser | Best | Good. Recompiled 2004-era code is far below modern budgets **[A]** | Good |
| Long-term maintainability | Best | Poor (opaque generated code) | Improves monotonically |

Why C over A: A has no executable reference inside the same process, so every subsystem must be complete
before anything runs, and integration bugs surface late. C keeps a working game at every commit, and every
replacement is differential-tested against the original under the *same* inputs, in the same address space.
Why C over B alone: B never yields an engine you can fix, extend, or port to new platforms cleanly, and it
keeps the legal dependency on the original code forever.

**Recompiler design notes [A]:** the binary is fixed, so recompile ahead of time. Function boundaries come
from the catalogue (Ghidra plus gta-reversed names, about 9.2k known functions in the game and RW, plus the
CRT). Indirect calls go through a VA→function table: vtables and function pointers point into `.text`, so any
VA with an entry becomes a table slot. Switch jump tables come from Ghidra recovery. Unknown indirect targets
trap to an interpreter fallback and get logged. Map the guest image into linear memory **at its original
VAs** (0x400000…), so guest pointers equal wasm addresses. x87 runs as 64-bit doubles in "precision control =
24" mode, with the exact 80-bit path only where the catalogue marks it as needed. Lift through
Remill/LLVM or a custom x86 → Rust/C emitter. Ignore the second `.text` (0xCB1000) and `.HOODLUM` sections and
verify the game never jumps into them (coverage will tell).

---

## 7. Autonomous development: realistic assessment

### 7.1 What I can do autonomously

- Static analysis at scale with REA/Ghidra: catalogue generation, call graphs, read/write sets, layout recovery
  cross-checked against plugin-sdk.
- Tooling: extend samp-tracer into the L2/L3 harness, the Unicorn L1 runner, normalisers, fixture formats,
  coverage pipelines and dashboards.
- Format crates to exact parity, the RenderWare object model, the wgpu renderer, the platform layer, the
  OPFS/AudioWorklet/worker plumbing and the WebSocket/WebTransport gateway.
- Function-by-function ports in leaf-first order, driven by diffs, in long loops with
  `PLAN/ROADMAP/WORKLOG` exactly as in samp-rs.
- Recompiler components (lifting, table generation, HLE stubs) and the Win32/D3D9 HLE.
- Divergence bisection and root-causing. This is mechanical once L3 exists.

### 7.2 What needs human decisions or validation

1. **Legal posture (blocking).** Is gta-reversed (no licence, a reconstruction) allowed as a source of *code*,
   or only of *facts* (names, addresses, sizes)? Is a recompiled original acceptable when generated locally
   from the user's exe? What licence will the Rust engine carry? Note that **one model acting as both
   analyst and implementer is not a legal clean room**. Separate agents with a spec boundary are process
   discipline, not a legal guarantee.
2. **Target binary:** 1.0 US HOODLUM (what samp-rs uses) versus Compact. Recommendation: **1.0 US**, ignoring
   the protection section.
3. **Scope:** SA:MP profile first (recommended), or single-player completeness.
4. **"Feel" and visual sign-off:** driving feel, camera and rendering look. Diffs catch regressions, but a
   human has to accept tolerances.
5. **Milestone acceptance** and any entry in the divergence ledger that is *accepted*, as opposed to
   *pending*.
6. Real-device browser checks (GPUs, Safari/Firefox) and costs (tokens and machine time).

### 7.3 Subagent coordination

- **Main session (integrator):** owns ROADMAP, catalogue state, merges and commits, and the Wine lane lock.
  Agents never commit (the same rule as samp-rs).
- **RE analyst agents:** one subsystem per agent. They produce specs (layouts, state machines, constants with
  addresses) and catalogue entries. Read-only on code.
- **Implementer agents:** take a spec plus catalogue entries and produce Rust plus harness bindings. They
  work in worktrees, one subsystem each, and never touch the harness or normalisers.
- **Harness agent:** the only role allowed to change oracles and normalisers. Changes need review.
- **Verifier agent (adversarial):** gets the code and the oracle, **not** the implementer's reasoning. It
  writes extra fuzz cases, mutation tests and edge cases, and tries to break the "verified" claim.
- Concurrency is limited by the oracle lanes (Wine/gamescope instances), not by agents. Expect 2–4 parallel
  lanes on this machine **[A]**. Watch disk: 67 GB is free now, fixtures and drcov logs must have quotas, and
  ≥20 GB must stay free.

### 7.4 Preventing false-positive compatibility

- **The oracle is always the original binary.** A fixture's expected values are never generated by Rust code.
- **Mutation testing of the harness:** a nightly job injects faults into replaced Rust functions (swapped
  operands, a dropped `rand()` call, an off-by-one in a loop) and requires the harness to catch ≥ N% of them.
  If it does not, the normaliser or coverage is hiding things.
- **Coverage gates:** a function is "verified" only if its original basic blocks were hit at ≥ X% in the
  corpus. Uncovered branches stay marked.
- **No silent tolerances:** every float tolerance or skipped field has a ledger entry with a reason, an owner
  and an approver.
- **Divergence ledger** (the samp-rs "Decisions made autonomously" pattern), split into *original bug kept*,
  *original UB not reproducible* and *accepted deviation (human-approved)*.
- **Re-recording:** L3 trajectories are re-recorded from the original periodically, so an old fixture cannot
  drift into agreement with a wrong Rust change.
- **The hook-off check:** for every replaced function, the same L3 trajectory runs once with the Rust version
  and once with the original toggled back on. Both must hash identically.

### 7.5 Tracking progress

The catalogue is the single source of truth. Each function has a state: `unknown → specified → implemented →
L1/L2-verified → L3-verified`, with coverage %. Each subsystem shows *share of SA:MP-profile functions
verified* and *share of corpus trajectories bit-exact for N frames*. A standalone native build appears when
the remaining-original-callees set for the SA:MP profile is empty. The browser build follows (the platform
layer is shared).

---

## 8. Roadmap (smallest independently testable first)

Durations are omitted on purpose. They depend on lane count, review cadence and cost limits. Every milestone
has a mechanical acceptance test.

| # | Milestone | Acceptance |
|---|---|---|
| **M0** | Decisions + catalogue + census | Legal/scope decisions recorded. `catalog/functions.toml` covers 100% of the functions Ghidra finds in `.text` (0x401000–0x857000), with gta-reversed/plugin-sdk names mapped and **every** gta-reversed address verified as a function start in 1.0 US. SA:MP-profile function set measured by coverage over ≥5 scripted server scenarios. |
| **M1** | `sa-formats` | Every file in the user's install parses. Byte-exact round-trip for all RW-stream, IMG, COL and IPL-binary files. In-memory comparison against the original's dumped `CModelInfo`, `CColModel`, handling, timecyc and anim tables: 0 differences. |
| **M2** | `sa-math` + L1 Unicorn harness | ≥100 leaf functions (vector/matrix/quaternion, `CGeneral`, collision primitives) bit-exact over 10⁶ random inputs each. Mutation test catches ≥95% of injected faults. |
| **M3** | Determinism mode for the original | Two runs of the same 10-minute input recording under Wine produce identical per-frame state hashes. |
| **M4** | `sa-rw` + `sa-render` (native, then web) | Static scenes (buildings, LODs, a skinned ped, a vehicle) render. D3D9 call-stream semantic diff against the original for identical camera/time: same set of atomics drawn, same textures. Image diff under a stated threshold. Runs in Chrome via WebGPU and via WebGL2. |
| **M5** | Platform + browser shell | BYO-assets import into OPFS. A fly-camera traverses the whole map with streaming in the browser, budget accounting matching the original's `CStreaming` decisions on a recorded camera path (L3 on the streaming state). |
| **M6** | SCM core | Interpreter control flow and variables L2-exact against the original for SA:MP's `main.scm` and a corpus of synthetic scripts. |
| **M7** | Physics core | `CPhysical`/collision for objects on static world: L3 bit-exact for 60 s trajectories (drops, slides, impacts). |
| **M8** | Player ped on foot | Locomotion, anim blending, camera: L3 exact for recorded walking/sprinting/jumping/aiming routes. |
| **M9** | **SA:MP vertical slice** | Browser client connects through the local gateway to an unmodified open.mp/samp-server: join, class selection, spawn, on-foot sync. Packet-stream diff against the original samp.dll + gta_sa is empty for the scripted scenario (the samp-rs L5 criteria). |
| **M10** | Vehicles (car → bike → boat → heli/plane) | Per class: L3 exact for recorded driving trials. Vehicle-sync diff empty. |
| **M11** | Weapons, damage, objects, pickups, checkpoints, HUD/radar/textdraws | L2/L3 per subsystem. samp-rs RPC coverage harness passes on the Rust engine. |
| **M12** | Audio | Event-level diff empty for scripted scenarios. Subjective sign-off. |
| **M13** | SA:MP-profile standalone | 0 remaining original callees in the profile. Full samp-rs parity suite green on the Rust engine, native and browser. |
| **M14+** | (Optional) single-player completeness | AI tasks/events, population, missions, saves. |

**Hybrid variant:** if the recompiler track (B) is approved, insert **R1–R3** after M3: the recompiler emits
a runnable native image (R1), the Win32/D3D9/DSound HLE boots to the menu (R2), and it plays in the browser
(R3). From then on, M6–M13 replace functions *inside* the recompiled image, and L2 runs in-process in both
native and browser builds.

---

## 9. Final answer

**Willing:** yes.

**Technically capable, incrementally, with RE and differential testing as the primary method:** yes, for:

- the infrastructure;
- formats and the RenderWare object model;
- the renderer and browser platform;
- the SA:MP-profile engine through M13.

This assumes three things: (1) the oracle infrastructure (M0–M3) is built first and kept honest by mutation
testing; (2) a human makes the legal and scope decisions and signs off at milestone gates; (3) progress is
reported as *measured, bounded equivalence*, never as proven 1:1.

**Not credible as a near-term commitment:** a *complete* single-player 1:1 Rust rewrite, mainly because of the
AI task/event and mission long tail and the limits of what testing can prove. Treat it as M14+, not as the
goal that defines success.

**Most realistic path to a working GTA SA in a browser:** recompiled original plus Rust HLE for the
fastest exact result (local, BYO exe), with Rust subsystems replacing it progressively. Or, if a recompiled
original is ruled out legally: the SA:MP-profile Rust engine (M0–M13), whose first browser-visible proof is
M4/M5 and whose first *useful* proof is M9.
