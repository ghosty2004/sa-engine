# NATIVE-DEMO: `sa-viewer`, the first standalone (phase B) executable

`sa-viewer` is a native Linux x86_64 program that loads San Andreas from the user's game folder and
lets you fly a free camera through the world. It needs no Wine, no `gta_sa.exe` and no Rockstar
code: it reads only the data files (`gta.dat`, IDE/IPL, `gta3.img`, `gta_int.img`, `timecyc.dat`,
`water.dat`, `particle.txd`). It is a demo of the phase B stack, not a 1:1 renderer. The 1:1
renderer is verified later against D3D9 captures (PLAN F7/F10).

## How to run

```sh
cargo build --release -p sa-viewer
target/release/sa-viewer --game /path/to/gta-sa                    # window
target/release/sa-viewer --game /path/to/gta-sa --pos 2495,-1670,30 --time 21:00
```

Options:

| option | meaning |
|---|---|
| `--game DIR` | game folder (required); paths inside are resolved case-insensitively |
| `--pos x,y,z` | start position (default Grove Street, 2495,-1670,20) |
| `--dir x,y,z` | look direction |
| `--time HH:MM` | time of day (default 12:00) |
| `--weather N` | time cycle weather row 0..22 (default 0, EXTRASUNNY_LA) |
| `--size WxH` | window / image size (default 1280x720) |
| `--draw-dist F` | draw distance multiplier (default 1.0) |
| `--budget-mb N` | GPU memory budget before eviction (default 1536) |
| `--area N` | area code to show (default 0 = outside world; 13 is always shown) |
| `--screenshot out.png` | headless: stream everything visible, render one frame offscreen, write a PNG |
| `--bench SECONDS [--bench-out f]` | headless scripted fly-through (Grove St → downtown LS → … → San Fierro at 80 m/s), prints FPS and memory |
| `--exit-after SECONDS` | window mode: quit after N seconds and print the average FPS |

Window controls: left click captures the mouse (Esc releases it, Esc again quits). Mouse or arrow keys
look around. W/S forward/back, A/D strafe, Space or E up, Q or Ctrl down. Shift ×5 speed, Alt ×0.2,
mouse wheel changes the base speed. `[`/`]` (or `-`/`=`) time −/+1 hour, `T` lets time run (1 game
minute per second), `,`/`.` previous/next weather, PageUp/PageDown draw distance, 1–4 jump to Grove
Street / downtown LS / San Fierro / Las Venturas. The title bar shows FPS, time, weather, position,
visible entities, draw calls, loaded models, GPU memory and pending loads.

`tools/native-demo/screenshots.sh [GAME_DIR] [OUT_DIR]` renders the reference screenshots and runs a
60 s benchmark (`OUT_DIR/bench.txt`).

## Crates

| crate | content | targets |
|---|---|---|
| `sa-platform` | traits: `fs::GameFs` (whole-file reads and `RandomRead` handles, game-style case-insensitive `\` paths), `input::{Key, InputState}`, `clock::Clock`, `spawn_worker`; `fs::MemFs`. `native` module (feature `native`, not on wasm32): `NativeFs` (std::fs, cached case-insensitive directory lookup, `pread`), `run_windowed` (winit 0.30 window, key/mouse translation, pointer capture). No other crate makes OS calls (the viewer binary reads `/proc/self/status` for its RSS report). | host, wasm32 (traits) |
| `sa-rw` | RenderWare object model from `sa-formats`' DFF/TXD stream types: `Frame` (modelling matrix, parent, node name), `Atomic` (frame, shared `Geometry`, flags, pipeline id), `Clump` (LTM), `Geometry` (morph target 0, prelit, night colours, UV set 0, materials, bin-mesh meshes incl. tristrip → list), `Material`, `TexDictionary`/`Texture`/`Raster` (DXT1/3/5 kept as blocks, every other D3D8/D3D9 format converted to RGBA8), software DXT decoder. Field names follow `docs/re/rw/objects.md`; layouts are Rust-native (no `#[repr(C)]` mirrors yet). | host, wasm32 |
| `sa-rw-wgpu` | wgpu 29 renderer: raster upload (BC1/2/3 when `TEXTURE_COMPRESSION_BC` is available and the size is a multiple of 4, otherwise RGBA8 decode), geometry instancing (one vertex/index buffer per geometry, one bind group per mesh), samplers from the texture's RW filter/addressing, instanced world pass, sky gradient, colour filter post pass. Reversed infinite depth. | host (Vulkan here), wasm32 builds |
| `sa-viewer` | the binary: world loading, streaming, visibility, camera, time cycle, CLI front ends (window, screenshot, benchmark) | host |

## What is ported game code and what is new viewer code

Ported and verified code that runs unchanged (no `orig!` call is reached):

* **`sa-formats`** (FMT-1/2): IMG v2 directories, RW chunk streams, DFF and TXD typed decoders,
  `gta.dat`/`default.dat` (`CFileLoader::LoadLevel` directive rules), IDE (`LoadObjectTypes`
  formats and fallbacks), text IPL (`LoadScene`), binary IPL (`CIplStore::LoadIpl` layout),
  `timecyc.dat` (`CTimeCycle::Initialise`, 52-field `sscanf`), `water.dat`
  (`CWaterLevel::WaterLevelInitialise`). All use the exe's `LoadLine` and MSVC `sscanf` semantics.
* **`sa_stream::modelinfo::base::base_model_info_flags_byte`** (`CBaseModelInfo::SetBaseModelInfoFlags`
  0x5B3AD0, L1) and **`sa_stream::modelinfo::atomic::atomic_flags_word`**
  (`CAtomicModelInfo::SetAtomicModelInfoFlags` 0x5B3B20, L1): every model's draw-last, additive,
  no-z-write and back-face-culling flags come from these.
* **`sa_math::CQuaternion::get`** (0x59C080, L1) and **`CMatrix::set_rotate_z_only`** (0x59B020, L1):
  instance matrices, in the two branches of `CFileLoader::LoadObjectInstance` (0x538090).

New viewer code that follows the original's rules as gta-reversed describes them (reading aid, not
measured, not ported):

* instance matrix branch selection, quaternion conjugation and the heading `acos(w) * ±2`
  (`LoadObjectInstance`; Rust's `acos`, the original's `_CIacos` is not ported);
* LOD links: text IPL `lod` = index in the same file, binary `<name>_streamN.ipl` `lod` = index in the
  text IPL whose stem is a prefix (`CIplStore::SetupRelatedIpls`);
* render states: main list alpha ref 140 outdoors, sorted (draw-last) list alpha ref 100 or 0 without
  z writes (`CRenderer::RenderEverythingBarRoads`, `CVisibilityPlugins::RenderEntity`), LODs drawn
  with a z shift of 100 (`RwCamera::zShift`), back-face culling per model flag, vertex alpha on;
* day/night prelight blend (`CCustomBuildingDNPipeline`, `NVCPipelineProcess`) with the balance of
  `CCustomBuildingRenderer::UpdateDayNightBalanceParam` (0x5D7F80, gta-reversed's commented body);
* time cycle sampling at hours 0,5,6,7,12,19,20,22 (`CTimeCycle::CalcColoursForPoint`, reduced);
* colour filter `F + F·c1·a1 + F·c2·a2` with `a = uint8(a_file·2)` (`CPostEffects::ColourFilter`,
  `CTimeCycle::Initialise`);
* `CClock::GetIsTimeInRange` for time objects; models with draw distance < 2 hidden.

Entirely new (no game counterpart yet): the IMG loader threads, the streaming queue and eviction,
the visibility traversal, frustum culling, the camera, the wgpu renderer itself.

The ported `sa-stream` streaming code (`CStreaming`, `CdStream`, `CIplStore`, model-info classes) is
**not** used: it works on game memory through `GPtr` and calls unported RW and CRT functions via
`orig!` (e.g. `RwFrameCreate`, `strncmp`, `LoadObjectInstance` itself), which panic in phase B.

## Verification

* `cargo test` for the four crates (unit tests: DXT decoding, pixel conversions, bin mesh, night
  colours, tristrip conversion, matrices, case-insensitive fs, input edges, frustum, time ranges,
  DN balance, instance matrix branches).
* `SA_GAME_DIR=… cargo test -p sa-rw --release -- --ignored`: every DFF and TXD of `gta3.img` and
  `gta_int.img` builds (12,955 + 1,901 clumps, 2,750 + 517 dictionaries, 31,128 textures decode to
  the expected size). The same test measured the winding convention of the triangle lists against
  the stored normals: 1,052,534 counter-clockwise vs 10,821 clockwise (gta3.img), so front faces are
  CCW in a right-handed world.
* clippy `-D warnings` on host for all four crates and on `wasm32-unknown-unknown` for `sa-rw`,
  `sa-platform` (`--no-default-features`) and `sa-rw-wgpu`.
* Headless screenshots (`tools/native-demo/screenshots.sh`): Grove Street at noon and at 23:00,
  downtown LS, San Fierro, the Las Venturas strip, Red County countryside. Inspected: the world,
  LODs in the distance, alpha-tested foliage, night prelight with lit windows, water, fog and sky
  colours all look like the game; nothing is missing or mis-placed at those spots.
* Window mode was run inside an isolated headless gamescope (`gamescope --backend headless`, child
  without `WAYLAND_DISPLAY`): 644 fps average over 15 s at Grove Street (Mailbox present mode).

Performance (RX 7800 XT, RADV, 1280×720, release build, 60 s scripted fly-through at 80 m/s; every
frame waits for the GPU, so these are CPU+GPU frame times):

| run | avg fps | 1% low | worst frame | peak RSS | peak GPU (models+textures) | evictions |
|---|---|---|---|---|---|---|
| `--bench 60` (defaults) | 765 | 221 | 9.4 ms | 222 MB | 89 MB | 0 |
| `--bench 60 --draw-dist 2 --budget-mb 64` (eviction stress) | 776 | 344 | 9.1 ms | 216 MB | 64 MB | 3,134 |

World loading (all IDEs, 50,769 instances
from text and binary IPLs, IMG directories) takes < 0.1 s; a fully streamed screenshot takes
≈0.2 s.

## Known gaps versus 1:1 (not yet verified against D3D9 captures)

Visibility and streaming:
* No `CRenderer` port: no sector scan, no `SetupBigBuildingVisibility`, no entity fading
  (`CalculateFadingAtomicAlpha`), no occluders (`occl`), no cull zones. The LOD rule is a viewer
  rule: a LOD is replaced by its in-range children once all of them are loaded.
* Draw distance multiplier 1.0 (the game's `ms_lodDistScale` from the menu setting is not applied);
  entities beyond the time cycle far clip + 300 m are dropped.
* No `CStreaming` semantics (memory limit, priorities, channels); loads are nearest-first with a GPU
  memory budget and LRU eviction.
* Only area 0 (and 13); interiors, `object.dat` dummy behaviour, glass objects hidden by
  `CGlass::IsObjectGlass`, underwater/tunnel instance flags and car generators are ignored.
* For DFFs with several atomics the viewer skips `_dam`/`_vlo` atomics and keeps the first remaining
  one for atomic models (`SetRelatedModelInfoCB`/`GetNameAndDamage` are not ported).

Rendering:
* No vehicles, peds, pickups, 2d effects (lights, coronas), shadows, clouds, sun, stars, rain,
  roads-first pass, MatFX/env maps, the vehicle pipeline, UV animation, skinning.
* Sky: a two-colour gradient (top/bottom) with an assumed band height; fog is linear per fragment on
  view depth between `fog_start` and `far_clip` with the sky bottom colour.
* Lighting model is assumed, not measured: prelit geometry with `rpGEOMETRYLIGHT` gets the world
  ambient (`amb` × surface ambient) added and a fixed directional light (`dir`) when it has normals;
  geometry without prelight uses `amb_obj` + `dir`. The light direction is fixed, not the sun's.
* Time cycle: no time cycle boxes, no weather interpolation, no smog blending, weather does not
  follow the zone; the colour filter omits the player-lighting factor; no other post effects
  (radiosity, grain, heat haze).
* `zShift` is applied as `z_ndc = near / (d + 100)` (sign and formula of the RW camera z shift not
  verified).
* Water: flat polygons from `water.dat` with the time cycle water colour and `waterclear256` at an
  assumed UV scale (1/32 m); no waves, no `CWaterLevel` rendering logic.
* No MSAA or anisotropic filtering; samplers follow the RW filter mode (a non-mip filter samples
  level 0 only, as D3D9 does).
