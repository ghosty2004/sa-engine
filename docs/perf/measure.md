# Measuring a frame (PERF-0)

The per-frame measurement of sa-game: where the time of an in-game frame goes (phase timers) and how much work it
does (counters), natively and in the browser, without a profiler. It is off by default and costs one relaxed atomic
load per instrumented call when off (verify-all stays IDENTICAL; see the WORKLOG entry of 2026-11-16).

## Running it

Native (offscreen, frame limiter off: `0xBA6794` is `FrontEndMenuManager.m_bPrefsFrameLimiter`):

```sh
cargo build --release -p sa-game --target i686-unknown-linux-gnu
SA_GAME_POKE=5:0xBA6794:0:1 target/i686-unknown-linux-gnu/release/sa-game --game wine/gta-sa \
    --start-in-world --frames 3000 --perf target/perf/native-offscreen.csv
```

`--perf FILE` (or `SA_GAME_PERF=FILE`) writes one CSV row per in-game frame to FILE at exit and prints the summary
table after the `frame pacing` line. It works with `--window` too (the window's event loop then counts as `other`).

Browser (`tools/web/build.sh`, `tools/web/serve.sh --game DIR`, then the headless driver):

```sh
node tools/web/headless.mjs --url 'http://localhost:8642/?autostart=1&auto=1&SA_GAME_PERF=600&backend=webgpu' \
    --out target/perf/web-webgpu --seconds 240 --gpu native
sed -n '/^frame,tag,/,/^sa-game: perf: CSV end/p' target/perf/web-webgpu/game-log.txt | sed '$d' > target/perf/web-webgpu.csv
```

`?SA_GAME_PERF=N` measures the first N in-game frames (default 600), then prints the summary and the CSV to the
console (the CSV as one message, between `sa-game: perf: CSV follows` and `sa-game: perf: CSV end`), which
headless.mjs collects into `game-log.txt`. `backend=gl` forces
WebGL2 (no GPU timestamps there). Keep the CSVs under `target/perf/` (not committed).

## What a row is

One in-game frame: every main-loop iteration up to and including one that ran `Game::Idle` (the frame limiter's
waiting iterations and message iterations belong to the next frame). Loading iterations (`gGameState` not IDLE) are
dropped. Phase times are exclusive (a nested phase is not charged to its parent), so a row's phases add up to
`total_ms`, the wall time since the previous row.

| phase | what |
|---|---|
| `other` | not covered by a phase: the main-loop bookkeeping, the window event loop, the browser between iterations |
| `loop` | the main-loop iteration outside `Game::Idle`: the `gGameState` machine, the frame limiter's wait |
| `pump` | input: front-end events queued as window messages, `PeekMessageA` / `DispatchMessageA` (the window procedure) |
| `idle.spin` | `Game::Idle`'s busy-wait before `CTimer::Update` (the original's 14 ms rule, see below) |
| `logic` | `Game::Idle` from `CTimer::Update` up to `CRenderer::ConstructRenderList`: the rest of `CGame::Process`, the lights |
| `logic.pad` / `.streaming` / `.script` / `.world` / `.population` / `.camera` / `.fx` | `CPad::UpdatePads`; `CStreaming::Update` + `LoadAllRequestedModels`; `CTheScripts::Process`; `CWorld::Process` (peds, vehicles, objects, physics); `CPopulation::Update` + `CCarCtrl::GenerateRandomCars`; `CCamera::Process`; `Fx_c::Update` |
| `audio` | `CAudioEngine::Service` |
| `render` | `Game::Idle` from `ConstructRenderList` on: pre-render, the RW pipelines and the ported RW D3D9 driver, effects, 2D, HUD, fonts (without our D3D9 layer) |
| `render.prerender` | `CRenderer::PreRender`, `CWorld::ProcessPedsAfterPreRender` |
| `render.list` | `CRenderer::ConstructRenderList` (the visibility scan) |
| `render.world` | `RenderRoads`, `RenderEverythingBarRoads`, the fading-in entity lists, the first-person vehicle, `RenderReallyDrawLastObjects` |
| `render.shadows` | `CRealTimeShadowManager::Update`, `CShadows` static / stored shadows, `CStencilShadows::RenderStencilShadows` |
| `render.water` / `render.sky` | `CWaterLevel::RenderWater`; `CClouds` (sky polys, clouds, volumetric clouds, moving fog), `CMovingThings::Render_BeforeClouds` |
| `render.effects` / `render.plants` / `render.coronas` | birds, weapon effects, special FX, glass, skidmarks, rain streaks, `CPostEffects::Render`; `CPlantMgr::Render`; `CCoronas::Render` + reflections + sun reflection |
| `render.hud` | `CHud::Draw`, `CFont::DrawFonts` |
| `d3d9` | our D3D9 COM methods: the translation to the seam and `WgpuDevice` (state, buffer / texture uploads, draws, render-pass encoding) |
| `present` | `IDirect3DDevice9::Present`: the last pass's submission, the window blit and the surface present |
| `perf` | the measurement's own bookkeeping at the end of a frame |
| `mods` | the mod host's overlay phase inside `Present` (feature `mods`, MOD-2): the mods' calls and the overlay draw; 0 without mods |
| `io` | file reads served by the web backend (`webfs`: the worker's block cache, and the waits below); not measured natively (local reads) |
| `gpu_ms` | (late column, not in the sum) GPU time of the frame's render passes from timestamp queries; empty without `TIMESTAMP_QUERY` (WebGL2) |

Counters per frame: `d3d9_calls` (COM method calls), `d3d9_set_calls` (`IDirect3DDevice9::Set*`), `draws`,
`passes` (render passes = queue submissions of draws), `pipelines_created`, `pipeline_changes` /
`texture_group_changes` (draws whose pipeline / texture bind group differs from the previous draw's),
`buffer_uploads` / `buffer_bytes` (vertex / index buffer writes), `texture_uploads` / `texture_bytes`,
`stream_bytes` (per-draw uniforms, shader constants and UP vertices written per pass), `dispatch_calls` (calls through
the `orig!` address registry), `platform_calls` (Win32 / CRT / DirectX stand-ins served by sa-game),
`game_allocs` (game heap allocations), `iterations` (main-loop iterations in the frame), `io_waits` / `io_wait_us`
(web: reads that had to wait for bytes the worker did not hold, and the microseconds waited; RENDER-2 R2a).

## A slow link (RENDER-2 R2a)

`tools/web/serve.sh --throttle-rtt MS --throttle-mbit N` delays every `/game/` answer by MS and shares N Mbit/s
between them (the page and its module are not throttled). `tools/web/headless.mjs --teleport 'S:X:Y:Z,...'` moves
the player S seconds after the game is in the world, to force streaming in new areas; a run's Chromium profile
(`target/web1-chrome-PORT`) keeps the persistent cache, so delete it for a cold run and keep it for a warm one.
`--web DIR` serves a saved copy of `web/` (a build from before a change) for before/after runs.

## How it is built (ADR-003)

- `sa_platform::clock`: one monotonic clock (`web_time::Instant`: `std::time::Instant` natively, `performance.now()`
  on the web; 5 us resolution in the cross-origin-isolated page `tools/web/serve.sh` serves).
- `sa_platform::perf`: the mechanism (scopes, exclusive phase times, counters, rows, CSV and summary).
- `sa-game/src/perf.rs`: the game's phases and counters, the frame boundaries (`Session::finish`) and the outputs.
- Phases are opened around ports, never inside them: the address registry's entries of the probed ports are replaced
  by timing wrappers when the measurement is on (`sa-game/src/platform/perf_probe.rs`, typed per convention on i686,
  one type-erased wrapper on wasm32); the D3D9 COM methods, `Present`, the message pump and the main loop open theirs
  in sa-game's platform layer.
- GPU time: `sa_rw_wgpu::gpu_timer` (timestamp writes at the start and end of every render pass, resolved and read back
  asynchronously some frames later; `TIMESTAMP_QUERY` is requested only while measuring).
- Probes see the calls that go through the registry (from lower crates' `=> unported` declarations); a call a port
  makes directly to a port in its own or a lower crate is charged to the caller's phase.

## First results (2026-11-16, PERF-0)

Core i7-14700F, RX 7800 XT (RADV); the browser is headless Chromium (`--gpu native`). In world at Grove Street, standing (`SA_GAME_WALK=1`), frame limiter off.
CSVs: `target/perf/` (not committed). Phase means in ms per frame:

| phase | native offscreen (2975 fr.) | web WebGPU (1500 fr.) | web WebGL2 (1500 fr.) |
|---|---:|---:|---:|
| frame (mean / median / p95) | 20.98 / 17.11 / 30.92 | 29.69 / 27.64 / 41.31 | 30.07 / 29.68 / 43.48 |
| `idle.spin` | 5.52 | 4.81 | 4.92 |
| `other` (outside iterations) | 0.01 | 4.68 | 4.64 |
| `logic.world` | 2.77 | 3.98 | 3.27 |
| `d3d9` | 2.79 | 2.19 | 3.24 |
| `render.world` | 1.83 | 2.40 | 2.04 |
| `logic` (rest of `CGame::Process`) | 1.50 | 2.01 | 2.00 |
| `render.list` | 1.25 | 1.53 | 1.55 |
| `render.shadows` | 1.14 | 1.49 | 1.38 |
| `render.coronas` (median ~0, p95) | 0.84 (3.67) | 1.85 (5.84) | 2.50 (8.31) |
| `render.prerender` | 0.78 | 1.01 | 0.74 |
| `render.plants` | 0.50 | 0.68 | 0.68 |
| `audio` | 0.43 | 0.58 | 0.58 |
| `render.hud` | 0.36 | 0.50 | 0.48 |
| everything else | 1.30 | 2.06 | 1.91 |
| `gpu_ms` (not in the sum) | 0.81 | 0.89 | n/a |

Counters per frame (native; the web's are the same within a few %): ~2,790 D3D9 calls (1,830 `Set*`), ~568
draws in ~17 passes, 134 pipeline and 396 texture bind-group changes, ~118 buffer uploads of ~1.0 MB, ~1.17 MB of
uniforms / constants / UP vertices, ~32,000 `orig!` registry dispatches, ~200 platform calls (~260,000 in frames with
an `idle.spin`: the busy-wait polls `QueryPerformanceCounter`).

Reading:

- `idle.spin` is the original's own rule in `Game::Idle` (0x53E920): it spins while `elapsed / cyclesPerMs -
  g_LastTickTime < 14` (unsigned), where `elapsed` counts from the previous `CTimer::Update` and `g_LastTickTime`
  holds the previous frame's value. Whatever the frame costs, every second frame waits until it is 14 ms longer than
  the one before (frames alternate ~W and ~W + 14 ms): the mean frame is W + ~7 ms. Without it the native frame
  would be ~15.5 ms (~65 fps) instead of 21 ms.
- The browser spends ~4.7 ms per frame outside the game: `web/worker.js` yields with `setTimeout(tick, 0)` after
  each ~12 ms batch, which a frame of 25 ms already exceeds, so every frame pays a clamped timer (and the
  cooperative audio threads).
- The GPU is idle most of the frame (0.8 ms of 21); the frame is CPU-bound: game logic (~5.5 ms), the game's render
  code (~6.5 ms), our D3D9 translation (~2.8 ms).

## PERF-1 (2026-11-16): first optimisations

Native offscreen, limiter off, `--perf` (frame mean / median / p95, ms): PERF-0 base 22.11 / 20.17 / 34.50;
`Game::Idle`'s 14 ms rule skipped with the limiter off (ROADMAP ledger) 17.03 / 16.21 / 22.22; registry dispatch O(1)
and the host stand-in filter 9.62 / 9.74 / 11.64. Phase means after both: `logic.world` 1.10, `d3d9` 2.98,
`render.world` 0.46, `logic` 0.76, `render.list` 0.72, `render.shadows` 0.76, `render.coronas` 1.23. With the limiter
on the frame stays at the 25 fps limit (40.2 ms) and `idle.spin` is unchanged (6.8 ms). Web WebGPU: 28.08 -> 24.78 ms
with the Idle skip (the worker-yield and registry web runs are still to be measured). Details: WORKLOG 2026-11-16.
