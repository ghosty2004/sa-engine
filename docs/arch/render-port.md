# PORT-RENDER: subsystem RENDER, first pass (crate `sa-render`)

First pass over the catalogue subsystem RENDER (832 rows; 512 in `catalog/samp-priority.csv`). The pass
starts with the `CRenderer` visibility / sector-scanning core (the native viewer's largest 1:1 gap,
`docs/arch/native-demo.md`), the `COcclusion` leaves it uses, and then the SA:MP-priority leaves of the
2D / sky / light classes (helper A) and of shadows / post effects / FX (helper B). Follows ADR-001 and
the PORT-CAM pattern. The main session sets `catalog/state.csv`; the per-function verdicts are in
`docs/arch/render-port-verdicts.tsv` and the table at the end.

Ported: **226 functions**: 216 `#[hook]` registrations (CRenderer 31 and COcclusion 5 by the agent, helper A 76,
helper B 104; see the sections below) + 10 ported but not hooked (6 entries under 5 bytes, 4 read by protection).
Verdict counts: **L2 76**, **implemented 71**, **implemented-needs-L2 57**, **L1 12**, **ported-not-hooked 6**, **ported-unhookable 4**. All engine ports on Rust are frame-identical on all seven recordings.

## Crate

`crates/sa-render` (PLAN §2 names it). Same rules as sa-world / sa-pop / sa-cam: `#![no_std]`,
`#![deny(unsafe_code)]`, game state through `globals!`/`GPtr`, every other catalogued function (RenderWare,
D3D and the `RwEngineInstance` render-state table included) through `orig!`, every port registered with
`#[hook]` and listed in its module's `HOOKS` (`sa_render::hooks::ALL`, appended by sa-hook after
`sa_cam::hooks::ALL`). Builds for host, i686 (`--features in-process`) and wasm32. Depends on sa-core,
sa-math, sa-stream, sa-world (`Fpu`, field helpers), sa-pop.

| file | contents |
|---|---|
| `common.rs` | sa-world's field helpers and `Fpu` re-exported, `TheCamera` address, `k(bits)`, `jp` / `jne` compare idioms |
| `renderer.rs` | `CRenderer`: render lists, LOD lists, sector scan lists, visibility (`SetupEntityVisibility`, `SetupMapEntityVisibility`, `SetupBigBuildingVisibility`), `ScanSectorList`, `ScanBigBuildingList`, `ScanWorld`, `ConstructRenderList`, `PreRender`, the render passes, streaming requests, `GetObjectsInFrustum`, `RequestObjectsInFrustum` |
| `occlusion.rs` | `COcclusion::Init`, `OccluderHidesBehind`, `CActiveOccluder::IsPointWithinOcclusionArea` / `IsPointBehindOccluder`, `COccluder::NearCamera` |
| `sprite2d.rs`, `sprite.rs`, `clouds.rs`, `weather.rs`, `coronas.rs`, `pointlights.rs`, `cullzones.rs` | helper A (see below) |
| `shadows.rs`, `bullettraces.rs`, `specialfx.rs`, `mirrors.rs`, `posteffects.rs`, `fx.rs` (+ `fx/`) | helper B (see below) |

Shared-crate changes: sa-hook `table.rs` (sa-render's hooks appended after sa-cam's), `callers.rs` test and
sa-oracle `hook_preserve` / `tests/l1.rs` chain sa-render's registrations; sa-oracle `specs_render*.rs`
(registered in `specs.rs`). Lists `tools/e2e/lists/render-*.txt`; `tools/e2e/l2-extras.txt` section
`# PORT-RENDER`.

## CRenderer: conventions and findings

- Ported from the 1.0 US bytes (objdump of the exe, 0x5531C0..0x555DB0); gta-reversed `Renderer.cpp` and
  `docs/re/specs/RENDER/CRenderer*.md` as reading aids. Every `expect` is whole instructions.
- Render states go through `RwEngineInstance->dOpenDevice.fpRenderStateSet` (`orig!` `table 0xC97B24 0x20`),
  exactly as the inlined `RwRenderStateSet` macro does; virtual calls (`Render`, `SetupLighting`,
  `RemoveLighting`, `PreRender`, `CreateRwObject`, `DeleteRwObject`, model-info `AsAtomicModelInfoPtr` /
  `GetModelType` / `GetTimeInfo`) through `vslot`.
- Float arguments that the original passes on as raw dwords (distances, light multipliers) are passed as
  `u32` so a signalling NaN is not quieted by the Rust ABI; where the original goes through `fld`/`fstp`
  (`AddToLodRenderList`, the `*out` distance stores, the restored camera z-shift) the port quiets it
  explicitly (`quiet`).
- x87: the literal operand order and stack use (`sa_math::x87::X87`), e.g. the three different summation
  orders of the camera distance: `(dz*dz + dy*dy) + dx*dx` (sorted list, visibility, LOD), `(dz*dz +
  dx*dx) + dy*dy` (underwater list, `ScanWorld`'s camera-moved test); `ShouldModelBeStreamed` keeps the
  distance unrounded on the stack. `ScanSectorList` / `ScanBigBuildingList` / `ConstructRenderList` use
  FPATAN (sa-math's f64 model, documented approximation; only compared against 0.36 / 0.7 rad or stored as
  the camera heading).
- Protection chunks inlined: `AddEntityToRenderList` 0x406B11 (`eax = ms_modelInfoPtrs[eax]`),
  `SetupLightingForEntity` 0x401FC0 (pushes `[0x1560700] + [0x4E9CA6]` = 1.0f), `ScanPtrList_RequestModels`
  0x401E0F (`ax = ms_nCurrentScanCode`), `IsPointBehindOccluder` 0x4063FD (`ebx = [0xB6F03C]`, computed as
  `[0x156080C] - [0x60E533] - 1`).
- gta-reversed differences kept from the bytes: `RenderEverythingBarRoads` inserts a translucent ped into
  the sorted alpha list (gr renders it at once); every counter loop re-reads its counter
  (`PreRender`, `RenderRoads`, `RenderEverythingBarRoads`, the LOD list loops re-read `ms_pLodRenderList`);
  `ConstructRenderList` sets `0xB745C1 = (attr & 0x80) || !(attr & 0x800)`; `SetupEntityVisibility`
  marks the time-model alpha 255 when the *other* model is loaded; `SetupMapEntityVisibility` returns 0
  (not 2) from the near-draw-distance off-screen path; `RenderOneNonRoad` passes `RemoveLighting` the whole
  argument dword whose low byte is the `SetupLighting` result; `SetupBigBuildingVisibility` passes
  `SetupMapEntityVisibility` the entity slot dword with the time flag in its low byte.
- `CRenderer::Init` / `Shutdown` are a single `ret` (1-byte entries): ported, not hooked.
- `CRenderer::SetupLightingForEntity` 0x553E40 is ported but not hooked: protection reads its entry's jump
  displacement as data (`mov edi, [0x553E45]` at 0x403C1B; sa-hook `every_hook_installs_on_the_exe` and
  `patch_refs`).
- Not ported this pass: `CRenderer::RequestObjectsInDirection` 0x555CB0 (builds an RwMatrix through
  `RwMatrixRotate`; next pass), the remaining `COcclusion` workers (`AddOne`, `ProcessBeforeRendering`,
  `COccluder::ProcessOneOccluder` / `ProcessLineSegment`, `IsPositionOccluded`).

## Helper A: CCullZones, CSprite2d, CSprite, CClouds, CWeather, CPointLights, CBrightLights, CCoronas

76 hooks + 3 ported, not hooked. Unhookable (protection reads the entry bytes / inbound branch):
`CCullZones::FindTunnelAttributesForCoors` 0x72D9F0 (displacement at 0x72D9F4 read by 0x401C18 /
0x1566FC3), `CSprite2d::DrawRect-CRGBA` 0x727B60 (`mov ecx, [0x727B61]` at 0x401462),
`CClouds::MovingFog_GetFirstFreeSlot` 0x713710 (its loop branches back to 0x713712, inside the patch).
Single-`ret` entries not ported: `CSprite2d::SetRecipNearClip` 0x727260, `CSprite::Initialise` 0x70CE10.
`CWeather::FindWeatherTypesList` 0x72A520 has `preserve = "ecx,edx"` (caller `CWeather::Update` 0x72B916
keeps both). `CSprite::InitSpriteBuffer` and `CWeather::ForceWeatherNow` inline protection chunks
(0x40A196, 0x402157). L1: `CullZonesIsPointWithinArbitraryArea` 1,060,000 cases, 0 mismatches (131,411
signalling-NaN cases unverified under Unicorn, as COLL/CAM); `WeatherFindWeatherTypesList` 1,002,000, 0
mismatches (ECX/EDX compared).

Bytes over gta-reversed: CCullZones::Init clears only the attribute-zone count and the two flag words;
zone sums go through `_ftol` low 16 bits; CSprite2d CRect `SetVertices` uses u/v 1/1024 and 1+1/1024 and
the explicit-UV variants give v2 = (u4, v4), v3 = (u3, v3); `SetVertices-iffCRGBA`'s z bias is 1e-4
(0x858FC4); CClouds::Update converts `IndividualRotation` as uint32; `Shutdown` destroys textures without
null checks; CWeather::SetWeatherToAppropriateTypeNow uses the region list's first entry;
UpdateWeatherRegion stores 1 then 0 for y >= -850; regions outside 0..4 give the default list;
CPointLights compares y/z against the unrounded ST0, x after an f32 store; GetLightMultiplier has a
16-bit loop index; AddLight checks the light count before the distance and computes dz twice.
FSIN (CClouds::Update) via sa-math's f64 model.

## Helper B: FX system, CShadows, CPostEffects, CSpecialFX, CMirrors, CBulletTraces

104 hooks (fx.rs 56 incl. `fx/info.rs` 31 `FxInfo*_c` constructors, shadows 7, posteffects 6,
specialfx 2, mirrors 1, bullettraces 1) + 4 ported, not hooked (entries under 5 bytes:
`Fx_c::GetFxQuality` 0x49EA50, `FxManager_c::GetFrustumInfo` 0x4A9130, `FxMemoryPool_c::FxMemoryPool_c`
0x4A9C10, `FxSystem_c::GetPlayStatus` 0x4AA900). L1 (14 specs, 10^6 each, 0 mismatches; SetConstTime /
SetRateMult INCOMPLETE on 12,117 signalling-NaN cases each). Not ported: protection-decoded entries
(`Fx_c::AddSparks`, `Fx_c::RenderEnd`, `CPostEffects::ImmediateModeRenderStatesStore`,
`CShadows::StoreStaticShadow`), entries jumping into unanalysed chunks (`Fx_c::Render`,
`FxManager_c::CreateFxSystem-3`, `FxEmitter_c::CreateParticle`, `CShadows::StoreShadowForPole`,
`StoreShadowToBeRendered-Texture`), and the big workers (`RenderStoredShadows`, `CastShadowEntityXY`,
`CBulletTraces::AddTrace`, emitter render/particle update, `CreateParticles`, HeatHaze / post-effect
render, mirror render, loaders, `ProcSurfaceInfo_c`, `CRealTimeShadow::Create`).

Bytes over gta-reversed: `CRealTimeShadowManager::GetRealTimeShadow` (gr's shadow-0 test is inverted: a
ped whose +0x598 is null takes shadow 0, otherwise the last free of 1..15; refused when the player's
vehicle squared speed > 0.09, unordered allows); `CMirrors::CreateBuffer` 512x256 fallback (FX quality < 1
or failed 1024x512 allocation, the first raster destroyed); `FxManager_c::FindFxSystemBP` keeps the
failure `sprintf`; `GetCompositeMatrix` returns 0 or the multiply result in EAX; `FxSystemBP_c::Update`
returns 0; `CPostEffects::Update` turns rain off unless the value is > 0 (unordered turns it off).
`Fx_c::RenderBegin`: the original lets `RwRenderStateGet` write into its own argument slot; the port
uses a local (stack only). The SEH frame of the `FxInfo*` constructors is not reproduced (nothing in
them can throw).

## Verification (commands, results)

All game runs on lane 1 (`SA_ROOT=/home/ghosty2004/sa-engine LANE=1`, under `systemd-run --user --scope -p
MemoryMax=…`), outputs in the worktree's `target/e2e/lane-1/port-l2-*`. Lists: `render-all.txt` (216),
`render-l2.txt` (94, plain l2), `render-split.txt` (122, split mode: resource create/destroy, heap and FX
pool allocation, streaming requests, `CreateRwObject` / `DeleteRwObject`, D3D drawing, `rand`, one-time
initialisers), `render-cr.txt` / `render-cr-split.txt` (CRenderer).

- A plain-l2 run of every hook crashed at frame 0: `CPostEffects::Initialise` run twice creates its rasters
  twice (`port-l2-rall-g-l2`). A first CRenderer-only run at `L2_PER_FRAME=1000` made no progress after
  frame 202 (29 hooks of ~10^4 calls per frame each, every case snapshotting the 4 MiB data window); it
  was stopped. Hence the list split.
- **L3, all engine ports on Rust** (`ports`, 2,752 hooks incl. the 216 RENDER ones, `--runs "plain
  rust"`), tags `render-{g,pr,cheats,cbug,anim,wc,drv}-ports`: **IDENTICAL over every frame of all seven
  recordings** (grove 5200, pause-restart 3700, world-cheats 3276, c-bug 3220, anim-input 3300,
  weapon-combat 3500, driving 4338), 17 subsystems, no crash / panic / install failure. Also every RENDER
  hook alone on Rust over grove: IDENTICAL (`rall-g-rust`). Hook-off runs were not made, so no row claims
  L3.
- **What the trace does not cover**: the 17 subsystem hashes include entity flags, pools, streaming and
  the world, so the visibility flags (`+0x1C`, `+0x35`), LOD counters and model requests are covered
  indirectly; the render lists themselves (`ms_aVisible*Ptrs`, PC_Scratch LOD lists), the alpha lists of
  CVisibilityPlugins, the D3D9 call stream (render states, draws), sprite / cloud / corona / shadow vertex
  buffers, FX particle state and post-effect rasters are **not** hashed. For those, L2/split are the
  evidence; F7 (D3D9 call-stream diff) is the missing check.
- `cargo test -p sa-render` 3/3; `cargo test --release -p sa-hook --lib` 48/48 (install, catalogue,
  callers, keycells, data_reads, patch_refs); clippy `-D warnings` sa-render host (all targets) and i686
  (`--features in-process`), sa-hook i686, sa-oracle host (all targets); `cargo build -p sa-render --target
  wasm32-unknown-unknown`. `cargo test --release -p sa-oracle --test l1`: 15/16, the failing
  `preserved_registers_match_the_hook_registrations` is the known PORT-SCRIPT item
  (`CRunningScript::GetPointerToLocalVariable` 0x463CA0 preserves ECX without a spec), not a RENDER hook.

### Run results (lane 1)

| run | tags | result |
|---|---|---|
| every RENDER hook on Rust | `rall-g-rust` | grove IDENTICAL, 17 subsystems |
| plain l2, every hook | `rall-g-l2` | crashed at frame 0 (`CPostEffects::Initialise` double run) → list split |
| plain l2, 95 then 94 hooks | `rl2-g` (4/frame), `rl2b-g` (1000/frame, with the PORT-RENDER extras), `rl2c-g` (2 reruns) | (b) l2 == plain IDENTICAL; 0 mismatches; `AddEntityToRenderList` moved to split (the alpha-list node of `InsertEntityIntoSortedList` is allocated twice in a double run: 516 mismatches at 0xC88158); after declaring the regions every hook PASS except `CClouds::SetUpOneSkyPoly` (0 cases) |
| split, 122 hooks | `rsp-g` (`SPLIT_PER_FRAME=20`, `--max 300`) | splito / splitr traces IDENTICAL to plain; 64 PASS, 33 PASS-FP, 2 FAIL, 3 DIVERGED; `CPostEffects::Update` hit the spool quota at frame 416 |
| split, CRenderer 21 hooks | `rcrsp-g`, then `rcrsp2-g` after the fixes | first: DIVERGED (Rust caller's stack pointer to the distance local) → extras declare the pointee; then SetupEntityVisibility 60 / SetupBigBuildingVisibility 186 pointee mismatches → port bug fixed (one distance local per scan call, as the original). `rcrsp2-g`: 16 PASS-FP, 3 FAIL: `RenderOneNonRoad` / `RenderEverythingBarRoads` (RW stack address at 0xC96438), `SetupEntityVisibility` 6 cases (the first iteration's uninitialised local: orig 0x00000A04, Rust 0) |
| all engine ports on Rust | `render-{g,pr,cheats,cbug,anim,wc,drv}-ports` (first build), `render-*-ports2` (final build) | IDENTICAL on all seven recordings |

## Divergence notes (ledger candidates)

- `CRenderer::ScanSectorList` / `ScanBigBuildingList`: the distance local passed to
  `SetupEntityVisibility` / `SetupBigBuildingVisibility` lives for the whole call, and paths that return 1
  without writing it (dynamic models, model type 6) pass the previous entity's distance to
  `AddEntityToRenderList`. The port keeps one local per call as the original does (found by the split run
  `rcrsp-g`: 60 / 186 pointee mismatches with a per-iteration local), but the first iteration's value is
  uninitialised stack in the original and 0.0 in the port.
- `SetupEntityVisibility` / `SetupBigBuildingVisibility` pass `SetupMapEntityVisibility` a dword whose
  upper three bytes are stack garbage (the time flag is the low byte; the callee reads the byte). The port
  passes the byte (`u8` argument, compared as one byte).
- `RenderOneNonRoad` / `RenderEverythingBarRoads` split FAIL (2 mismatches each): the only difference is
  0xC96438, a stack address stored by RenderWare code below them (`0x36E40C` vs `0x36E3A0`, the Rust
  frames sit deeper); the same page 0xC96000 is the open INF-17 item (written at 0x772C0D). Not a port
  difference; needs a normaliser decision (stack-address class for 0xC96438).
- FPATAN in `ScanSectorList` / `ScanBigBuildingList` / `ConstructRenderList` (heading), FSIN in
  `CClouds::Update`: sa-math's f64 model. `ST0` results of original callees (`GenerateLightsAffectingObject`,
  `GetLightingFromCol`, `Magnitude`, `LimitRadianAngle`, `CalculateGroundHeight`) reach Rust as f64 (exact
  at PC=24).
- `Fx_c::RenderBegin` writes its `RwRenderStateGet` result to a local instead of its own argument slot
  (stack only).

## Open items

- Undeclared writes of the split hooks (FX constructors and pool allocations, CRenderer render passes and
  scans: heap entity fields, alpha-list nodes, page 0xC96000) need narrow, evidence-backed regions before
  they can reach L2; until then they stay `implemented` (split PASS / PASS-FP).
- `CPostEffects::Update` fills the 2 GiB split spool by frame 416 (`rsp-g`); give it its own run with a
  small `SPLIT_PER_FRAME`.
- `CClouds::SetUpOneSkyPoly`: 24,535 calls, 0 L2 cases (only reached nested inside other L2 calls).
- Not ported: `CRenderer::RequestObjectsInDirection`, `COcclusion::AddOne` / `ProcessBeforeRendering`,
  `COccluder::ProcessOneOccluder` / `ProcessLineSegment`, `COcclusion::IsPositionOccluded`, CWaterLevel
  (31 SA:MP rows), CTimeCycle (18), CStencilShadows (11), C3dMarkers, CGlass, CCustomCarPlateMgr, the big
  CShadows / FX / post-effect / corona / cloud / weather workers listed above.
- No recording reaches mirrors, first-person vehicles, heat haze / night vision, most script-driven
  weather / cull-zone setters, so those hooks have no case.
- Hook-off runs (per-hook L3).

## Functions

| address | name | state | evidence |
|---|---|---|---|
| 0x712FA0 | CClouds::Shutdown | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x712FF0 | CClouds::Update | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x713060 | CClouds::SetUpOneSkyPoly | implemented | L2 grove:0/24535 FAIL (0 mismatches, 0 undeclared) |
| 0x7131C0 | CClouds::VolumetricCloudsInit | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x7135C0 | CClouds::VolumetricClouds_GetFirstFreeSlot | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x7135F0 | CClouds::VolumetricClouds_Delete | L2 | L2 grove:5/5 PASS (0 mismatches, 0 undeclared) |
| 0x713630 | CClouds::VolumetricCloudsGetMaxDistance | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x713660 | CClouds::MovingFogInit | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x7136D0 | CClouds::MovingFog_GetFXIntensity | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x7136E0 | CClouds::MovingFog_GetWind | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x713730 | CClouds::MovingFog_Delete | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x70CE30 | CSprite::CalcScreenCoors | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x70CF20 | CSprite::FlushSpriteBuffer | L2 | split grove:300/300 PASS (0 mismatches, 0 undeclared) |
| 0x70CFB0 | CSprite::InitSpriteBuffer | L2 | split grove:300/300 PASS (0 mismatches, 0 undeclared) |
| 0x70E1C0 | CSprite::Set4Vertices2D-CRect | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x70E2D0 | CSprite::Set4Vertices2D-1 | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x70E3E0 | CSprite::CalcHorizonCoors | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x726AA0 | CSpecialFX::Update | implemented | split grove:211/211 PASS-FP (0 mismatches, 33 undeclared) |
| 0x726AD0 | CSpecialFX::Render | implemented | split grove:123/123 PASS-FP (0 mismatches, 123 undeclared) |
| 0x7010F0 | CPostEffects::ScriptResetForEffects | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x7034F0 | CPostEffects::IsVisionFXActive | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x7046A0 | CPostEffects::Update | implemented | split grove:210/210 PASS-FP (0 mismatches, 0 undeclared) |
| 0x700D70 | CPostEffects::ImmediateModeRenderStatesSet | L2 | L2 grove:31/31 PASS (0 mismatches, 0 undeclared) |
| 0x704630 | CPostEffects::Initialise | implemented | split grove:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x700E00 | CPostEffects::ImmediateModeRenderStatesReStore | L2 | L2 grove:31/31 PASS (0 mismatches, 0 undeclared) |
| 0x7230A0 | CMirrors::CreateBuffer | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x6FAA70 | CCoronas::Init | implemented | split grove:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x6FAB00 | CCoronas::Shutdown | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x727230 | CSprite2d::Constructor | L2 | L2 grove:247/247 PASS (0 mismatches, 0 undeclared) |
| 0x727240 | CSprite2d::Delete | L2 | split grove:61/61 PASS (0 mismatches, 0 undeclared) |
| 0x727270 | CSprite2d::SetTexture- | L2 | split grove:40/40 PASS (0 mismatches, 0 undeclared) |
| 0x7272B0 | CSprite2d::SetTexture-mask | L2 | split grove:32/32 PASS (0 mismatches, 0 undeclared) |
| 0x7272E0 | CSprite2d::SetAddressingUV | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x727320 | CSprite2d::SetAddressing | L2 | L2 grove:23/23 PASS (0 mismatches, 0 undeclared) |
| 0x727350 | CSprite2d::InitPerFrame | L2 | split grove:300/300 PASS (0 mismatches, 0 undeclared) |
| 0x727390 | CSprite2d::IsVertexBufferEmpty | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x7273A0 | CSprite2d::IsVertexBufferFull | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x7273D0 | CSprite2d::RenderVertexBuffer | implemented | split grove:300/300 PASS (0 mismatches, 300 undeclared) |
| 0x727420 | CSprite2d::SetVertices-CRectCRGBA4 | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x727590 | CSprite2d::SetVertices-ffffffffCRGBA | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x727710 | CSprite2d::SetVertices-CRectCRGBA4ffffffff | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x727890 | CSprite2d::SetVertices-iffCRGBA | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x727920 | CSprite2d::SetVertices-ifCRGBA | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x7279B0 | CSprite2d::SetMaskVertices | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x727A00 | CSprite2d::SetVertices-RwD3D9Vertex | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x727B30 | CSprite2d::SetRenderState | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x727BE0 | CSprite2d::DrawTxRect | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x727C10 | CSprite2d::DrawRect-CRGBA4 | implemented | split grove:2/2 PASS (0 mismatches, 2 undeclared) |
| 0x727C50 | CSprite2d::DrawRectXLU | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x727CC0 | CSprite2d::DrawAnyRect | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x707670 | CStaticShadow::Free | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x707770 | CShadows::TidyUpShadows | implemented | split grove:1/1 PASS-FP (0 mismatches, 0 undeclared) |
| 0x707F40 | CShadows::UpdateStaticShadows | implemented | split grove:123/123 PASS-FP (0 mismatches, 123 undeclared) |
| 0x707460 | CShadows::SetRenderModeForShadowType | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x7067C0 | CRealTimeShadowManager::Init | implemented | split grove:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x7076C0 | CShadows::CalcPedShadowValues | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x706970 | CRealTimeShadowManager::GetRealTimeShadow | L2 | L2 grove:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x72D6B0 | CCullZones::Init | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x72D970 | CCullZones::FindAttributesForCoors | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x72DA70 | CCullZones::FindMirrorAttributesForCoors | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x72DAD0 | CCullZones::FindZoneWithStairsAttributeForPlayer | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x72DB50 | CCullZones::AddTunnelAttributeZone | L2 | L2 grove:36/36 PASS (0 mismatches, 0 undeclared) |
| 0x72DC10 | CCullZones::AddMirrorAttributeZone | L2 | L2 grove:65/65 PASS (0 mismatches, 0 undeclared) |
| 0x72DD20 | CCullZones::CamCloseInForPlayer | L2 | L2 grove:832/832 PASS (0 mismatches, 0 undeclared) |
| 0x72DD30 | CCullZones::CamStairsForPlayer | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x72DD40 | CCullZones::Cam1stPersonForPlayer | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x72DD50 | CCullZones::NoPolice | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x72DD60 | CCullZones::PoliceAbandonCars | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x72DD70 | CCullZones::InRoomForAudio | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x72DD80 | CCullZones::FewerCars | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x72DD90 | CCullZones::FewerPeds | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x72DDA0 | CCullZones::DoINeedToLoadCollision | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x72DDB0 | CCullZones::CamNoRain | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x72DDC0 | CCullZones::PlayerNoRain | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x72DDD0 | CCullZones::DoExtraAirResistanceForPlayer | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x72DDE0 | CCullZones::IsPointWithinArbitraryArea | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x72DEC0 | CCullZones::Update | implemented | split grove:210/210 PASS-FP (0 mismatches, 0 undeclared) |
| 0x72DF70 | CCullZones::AddCullZone | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x6FFB40 | CPointLights::Init | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x6FFE70 | CPointLights::GetLightMultiplier | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x6FFFF0 | CPointLights::ProcessVerticalLineUsingCache | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x7000E0 | CPointLights::AddLight | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x722140 | CBrightLights::Init | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x5531E0 | CRenderer::RenderFadingInEntities | implemented | split grove:300/300 PASS-FP (0 mismatches, 300 undeclared) |
| 0x553220 | CRenderer::RenderFadingInUnderwaterEntities | implemented | split grove:300/300 PASS-FP (0 mismatches, 33 undeclared) |
| 0x553230 | CRenderer::RenderOneRoad | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x553260 | CRenderer::RenderOneNonRoad | implemented | split grove:300/300 FAIL (2 mismatches, 300 undeclared) |
| 0x553390 | CRenderer::RemoveVehiclePedLights | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x5534B0 | CRenderer::AddEntityToRenderList | implemented | split grove:300/300 PASS-FP (0 mismatches, 233 undeclared) |
| 0x553540 | CRenderer::SetupScanLists | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x5535D0 | CRenderer::ScanSectorList_ListModels | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x553650 | CRenderer::ScanSectorList_ListModelsVisible | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x5536D0 | CRenderer::GetLodRenderListBase | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x5536E0 | CRenderer::GetLodDontRenderListBase | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x5536F0 | CRenderer::ResetLodRenderLists | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x553710 | CRenderer::AddToLodRenderList | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x553740 | CRenderer::AddToLodDontRenderList | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x553770 | CRenderer::ProcessLodRenderLists | implemented | split grove:300/300 PASS-FP (0 mismatches, 0 undeclared) |
| 0x553910 | CRenderer::PreRender | implemented | split grove:300/300 PASS-FP (0 mismatches, 300 undeclared) |
| 0x553A10 | CRenderer::RenderRoads | implemented | split grove:300/300 PASS-FP (0 mismatches, 300 undeclared) |
| 0x553AA0 | CRenderer::RenderEverythingBarRoads | implemented | split grove:300/300 FAIL (2 mismatches, 300 undeclared) |
| 0x553D00 | CRenderer::RenderFirstPersonVehicle | implemented | split grove:300/300 PASS-FP (0 mismatches, 0 undeclared) |
| 0x553F60 | CRenderer::SetupMapEntityVisibility | implemented | split grove:300/300 PASS-FP (0 mismatches, 31 undeclared) |
| 0x554230 | CRenderer::SetupEntityVisibility | implemented | split grove:300/300 FAIL (6 mismatches, 31 undeclared) |
| 0x554650 | CRenderer::SetupBigBuildingVisibility | implemented | split grove:300/300 PASS-FP (0 mismatches, 1 undeclared) |
| 0x554840 | CRenderer::ScanSectorList | implemented | split grove:300/300 PASS-FP (0 mismatches, 27 undeclared) |
| 0x554B10 | CRenderer::ScanBigBuildingList | implemented | split grove:300/300 PASS-FP (0 mismatches, 78 undeclared) |
| 0x554EB0 | CRenderer::ShouldModelBeStreamed | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x555680 | CRenderer::ScanPtrList_RequestModels<CPtrListSingleLink<CPhysical*>> | implemented | split grove:50/50 PASS-FP (0 mismatches, 4 undeclared) |
| 0x555900 | CRenderer::ScanSectorList_RequestModels | implemented | split grove:39/39 PASS-FP (0 mismatches, 7 undeclared) |
| 0x5556E0 | CRenderer::ConstructRenderList | implemented | split grove:300/300 PASS-FP (0 mismatches, 300 undeclared) |
| 0x554FE0 | CRenderer::ScanWorld | implemented | split grove:300/300 PASS-FP (0 mismatches, 300 undeclared) |
| 0x554C60 | CRenderer::GetObjectsInFrustum | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x555960 | CRenderer::RequestObjectsInFrustum | implemented | split grove:1/1 PASS-FP (0 mismatches, 1 undeclared) |
| 0x723FB0 | CBulletTraces::Update | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x4A4C20 | FxInfoEmSize_c::FxInfoEmSize_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A4D00 | FxInfoEmSpeed_c::FxInfoEmSpeed_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A4DC0 | FxInfoEmDir_c::FxInfoEmDir_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A4E90 | FxInfoEmAngle_c::FxInfoEmAngle_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A4F50 | FxInfoEmLife_c::FxInfoEmLife_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A5010 | FxInfoEmPos_c::FxInfoEmPos_c | implemented | split grove:5/5 PASS (0 mismatches, 5 undeclared) |
| 0x4A50E0 | FxInfoEmWeather_c::FxInfoEmWeather_c | implemented | split grove:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x4A51B0 | FxInfoEmRotation_c::FxInfoEmRotation_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A5450 | FxInfoForce_c::FxInfoForce_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A5550 | FxInfoFriction_c::FxInfoFriction_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A5650 | FxInfoAttractPt_c::FxInfoAttractPt_c | implemented | split grove:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x4A57C0 | FxInfoAttractLine_c::FxInfoAttractLine_c | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4A5980 | FxInfoGroundCollide_c::FxInfoGroundCollide_c | implemented | split grove:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x4A5A10 | FxInfoWind_c::FxInfoWind_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A5B20 | FxInfoJitter_c::FxInfoJitter_c | implemented | split grove:8/8 PASS (0 mismatches, 8 undeclared) |
| 0x4A5CB0 | FxInfoRotSpeed_c::FxInfoRotSpeed_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A5DA0 | FxInfoFloat_c::FxInfoFloat_c | L2 | split grove:3/3 PASS (0 mismatches, 0 undeclared) |
| 0x4A5E20 | FxInfoUnderwater_c::FxInfoUnderwater_c | L2 | split grove:3/3 PASS (0 mismatches, 0 undeclared) |
| 0x4A5EA0 | FxInfoColour_c::FxInfoColour_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A5FA0 | FxInfoSize_c::FxInfoSize_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A60B0 | FxInfoSpriteRect_c::FxInfoSpriteRect_c | implemented | split grove:8/8 PASS (0 mismatches, 8 undeclared) |
| 0x4A61A0 | FxInfoHeatHaze_c::FxInfoHeatHaze_c | L2 | split grove:9/9 PASS (0 mismatches, 0 undeclared) |
| 0x4A6220 | FxInfoTrail_c::FxInfoTrail_c | implemented | split grove:2/2 PASS (0 mismatches, 2 undeclared) |
| 0x4A6320 | FxInfoFlat_c::FxInfoFlat_c | implemented | split grove:6/6 PASS (0 mismatches, 6 undeclared) |
| 0x4A6440 | FxInfoDir_c::FxInfoDir_c | implemented | split grove:12/12 PASS (0 mismatches, 12 undeclared) |
| 0x4A6550 | FxInfoAnimTexture_c::FxInfoAnimTexture_c | implemented | split grove:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x4A6630 | FxInfoColourRange_c::FxInfoColourRange_c | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4A6760 | FxInfoSelfLit_c::FxInfoSelfLit_c | L2 | split grove:20/20 PASS (0 mismatches, 0 undeclared) |
| 0x4A67D0 | FxInfoColourBright_c::FxInfoColourBright_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A68E0 | FxInfoSmoke_c::FxInfoSmoke_c | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4A6A00 | FxInfoEmRate_c::FxInfoEmRate_c | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4ABF50 | CWeather::IsRainy | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x72A480 | CWeather::Init | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x72A4E0 | CWeather::ForceWeather | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x72A4F0 | CWeather::ForceWeatherNow | L2 | L2 grove:2/2 PASS (0 mismatches, 0 undeclared) |
| 0x72A510 | CWeather::ReleaseWeather | L2 | L2 grove:5/5 PASS (0 mismatches, 0 undeclared) |
| 0x72A520 | CWeather::FindWeatherTypesList | L1 | L1 WeatherFindWeatherTypesList 1,002,000 cases, 0 mismatches (ECX/EDX compared); L2 grove:3/3 PASS (0 mismatches, 0 undeclared) |
| 0x72A590 | CWeather::ForecastWeather | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x72A640 | CWeather::UpdateWeatherRegion | L2 | L2 grove:315/315 PASS (0 mismatches, 0 undeclared) |
| 0x72A790 | CWeather::SetWeatherToAppropriateTypeNow | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x49E620 | Fx_c::Constructor | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x49E640 | Fx_c::Update | implemented | split grove:210/210 PASS-FP (0 mismatches, 76 undeclared) |
| 0x49EA40 | Fx_c::SetFxQuality | L2 | L2 grove:3/3 PASS (0 mismatches, 0 undeclared) |
| 0x4A2550 | FxEmitter_c::Init_Reversed | L1 | L1 FxEmitterInit 1,002,000, 0 mismatches; L2 grove:93/93 PASS (0 mismatches, 0 undeclared) |
| 0x4A2570 | FxEmitter_c::Reset_Reversed | L2 | L2 grove:136/136 PASS (0 mismatches, 0 undeclared) |
| 0x4A8440 | FxInterpInfoFloat_c::FxInterpInfoFloat_c | L1 | L1 FxInterpInfoFloatCtor 1,002,000, 0 mismatches; split grove:20/20 PASS (0 mismatches, 0 undeclared) |
| 0x4A87D0 | FxInterpInfoU255_c::FxInterpInfoU255_c | L1 | L1 FxInterpInfoU255Ctor 1,002,000, 0 mismatches; split grove:20/20 PASS (0 mismatches, 0 undeclared) |
| 0x4A8990 | FxInterpInfo32_c::FxInterpInfo32_c | L2 | split grove:20/20 PASS (0 mismatches, 0 undeclared) |
| 0x4A8B50 | FxInterpInfo255_c::FxInterpInfo255_c | L2 | split grove:17/17 PASS (0 mismatches, 0 undeclared) |
| 0x4A93B0 | FxManager_c::ReturnParticle | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4A93C0 | FxManager_c::GetParticle | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4A93E0 | FxManager_c::SetWindData | L1 | L1 FxManagerSetWindData 1,002,000, 0 mismatches; L2 grove:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x4A9440 | FxManager_c::FxRwMatrixCreate | L1 | L1 FxManagerFxRwMatrixCreate 1,002,000, 0 mismatches; split grove:300/300 PASS (0 mismatches, 0 undeclared) |
| 0x4A9460 | FxManager_c::FxRwMatrixDestroy | L1 | L1 FxManagerFxRwMatrixDestroy 1,002,000, 0 mismatches; split grove:300/300 PASS (0 mismatches, 0 undeclared) |
| 0x4A9BE0 | FxManager_c::CreateFxSystem-4 | implemented | split grove:32/32 PASS-FP (0 mismatches, 0 undeclared) |
| 0x4A9CA0 | FxMemoryPool_c::GetMem | L1 | L1 FxMemoryPoolGetMem 1,002,000, 0 mismatches; split grove:20/20 PASS (0 mismatches, 0 undeclared) |
| 0x4A9CD0 | FxMemoryPool_c::Optimise | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x4A18B0 | FxEmitterPrt_c::operator new[] | implemented | split grove:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x4AA100 | FxSystemBP_c::operator new | implemented | split grove:20/20 PASS (0 mismatches, 20 undeclared) |
| 0x4A9F50 | FxPrim_c::Enable | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4A9CF0 | FxPrimBP_c::FxPrimBP_c | L1 | L1 FxPrimBPCtor 1,002,000, 0 mismatches; split grove:20/20 PASS (0 mismatches, 0 undeclared) |
| 0x4A18D0 | FxEmitterBP_c::Constructor | L2 | split grove:20/20 PASS (0 mismatches, 0 undeclared) |
| 0x4A24D0 | FxEmitterBP_c::IsFxInfoPresent | L2 | L2 grove:322/322 PASS (0 mismatches, 0 undeclared) |
| 0x4AA0D0 | FxSystemBP_c::Constructor | L1 | L1 FxSystemBPCtor 1,002,000, 0 mismatches; split grove:20/20 PASS (0 mismatches, 0 undeclared) |
| 0x4AA130 | FxSystemBP_c::Update | L2 | split grove:300/300 PASS (0 mismatches, 0 undeclared) |
| 0x4AAF00 | FxSystem_c::Constructor | implemented | split grove:32/32 PASS-FP (0 mismatches, 0 undeclared) |
| 0x4AA390 | FxSystem_c::Stop | implemented | split grove:11/11 PASS-FP (0 mismatches, 0 undeclared) |
| 0x4AA3D0 | FxSystem_c::PlayAndKill | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4AA3F0 | FxSystem_c::Kill | implemented | split grove:11/11 PASS-FP (0 mismatches, 0 undeclared) |
| 0x4AA610 | FxSystem_c::EnablePrim | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4AA630 | FxSystem_c::SetMatrix | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4AA660 | FxSystem_c::SetOffsetPos | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4AA6C0 | FxSystem_c::SetConstTime | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4AA6F0 | FxSystem_c::SetRateMult | L2 | L2 grove:90/90 PASS (0 mismatches, 0 undeclared) |
| 0x4AA730 | FxSystem_c::SetVelAdd | L1 | L1 FxSystemSetVelAdd 1,002,000, 0 mismatches |
| 0x4AA890 | FxSystem_c::CopyParentMatrix | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4AA8C0 | FxSystem_c::GetCompositeMatrix | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4AA910 | FxSystem_c::SetLocalParticles | L1 | L1 FxSystemSetLocalParticles 1,002,000, 0 mismatches |
| 0x4AAF30 | FxSystem_c::IsVisible | L2 | L2 grove:871/871 PASS (0 mismatches, 0 undeclared) |
| 0x49EA90 | Fx_c::Init | implemented | split grove:1/1 PASS (0 mismatches, 1 undeclared) |
| 0x4A1280 | Fx_c::DestroyEntityFx | implemented | split grove:11/11 PASS-FP (0 mismatches, 0 undeclared) |
| 0x4A13B0 | Fx_c::RenderBegin | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4A1070 | Fx_c::TriggerWaterSplash | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x4A2B40 | FxEmitterBP_c::CreateInstance | implemented | split grove:32/32 PASS-FP (0 mismatches, 0 undeclared) |
| 0x4A2BC0 | FxEmitterBP_c::Update | L2 | split grove:300/300 PASS (0 mismatches, 0 undeclared) |
| 0x4A4A10 | FxInfoManager_c::ProcessMovementInfo | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x4A9470 | FxManager_c::Constructor | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x4A9360 | FxManager_c::FindFxSystemBP | implemented | split grove:32/32 PASS-FP (0 mismatches, 0 undeclared) |
| 0x4A9A80 | FxManager_c::Update | implemented | split grove:210/210 PASS-FP (0 mismatches, 0 undeclared) |
| 0x4A9C30 | FxMemoryPool_c::Init | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x4AA160 | FxSystemBP_c::Render | implemented | split grove:300/300 PASS-FP (0 mismatches, 0 undeclared) |
| 0x4AA200 | FxSystemBP_c::SetBoundingSphere | L2 | L2 grove:82/82 PASS (0 mismatches, 0 undeclared) |
| 0x4AA260 | FxSystem_c::Destructor | implemented | split grove:11/11 PASS-FP (0 mismatches, 0 undeclared) |
| 0x4AA2F0 | FxSystem_c::Play | implemented | split grove:17/17 PASS-FP (0 mismatches, 0 undeclared) |
| 0x4AA840 | FxSystem_c::Exit | implemented | split grove:11/11 PASS-FP (0 mismatches, 0 undeclared) |
| 0x4AAAD0 | FxSystem_c::GetBoundingSphereWld | implemented-needs-L2 | no case in the runs (NOCALLS / NOCASES) |
| 0x71DCA0 | COcclusion::Init | L2 | split grove:1/1 PASS (0 mismatches, 0 undeclared) |
| 0x71E080 | COcclusion::OccluderHidesBehind | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x71E580 | CActiveOccluder::IsPointWithinOcclusionArea | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x71F960 | COccluder::NearCamera | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x71FA40 | CActiveOccluder::IsPointBehindOccluder | L2 | L2 grove:1000/1001 PASS (0 mismatches, 0 undeclared) |
| 0x5531C0 | CRenderer::Init | ported-not-hooked | 1-byte entry (ret) |
| 0x5531D0 | CRenderer::Shutdown | ported-not-hooked | 1-byte entry (ret) |
| 0x553E40 | CRenderer::SetupLightingForEntity | ported-unhookable | protection reads 0x553E45 (0x403C1B) |
| 0x72D9F0 | CCullZones::FindTunnelAttributesForCoors | ported-unhookable | protection reads 0x72D9F4 (0x401C18, 0x1566FC3) |
| 0x727B60 | CSprite2d::DrawRect-CRGBA | ported-unhookable | protection reads 0x727B61 (0x401462) |
| 0x713710 | CClouds::MovingFog_GetFirstFreeSlot | ported-unhookable | loop branches back into the patched entry (0x713712) |
| 0x49EA50 | Fx_c::GetFxQuality | ported-not-hooked | entry < 5 bytes |
| 0x4A9130 | FxManager_c::GetFrustumInfo | ported-not-hooked | entry < 5 bytes |
| 0x4A9C10 | FxMemoryPool_c::FxMemoryPool_c | ported-not-hooked | entry < 5 bytes |
| 0x4AA900 | FxSystem_c::GetPlayStatus | ported-not-hooked | entry < 5 bytes |


# PORT-RENDER-2: second pass (agent + 2 helpers, lane 1)

Second pass over what pass 1 left ("Not done" above): the COcclusion workers, CRenderer::RequestObjectsInDirection,
CBulletTraces, CGlass, CStencilShadows, the regular FX info bodies (agent), CWaterLevel (helper A), CTimeCycle +
CColourSet, C3dMarker(s), CCustomCarPlateMgr (helper B). SA:MP-priority rows first (`catalog/samp-priority.csv`),
then rows the corpus executes (`catalog/raw/coverage-all.hits.csv`). Per-function verdicts:
`docs/arch/render-port-2-verdicts.tsv` (one row per hook plus the not-hooked / skipped rows).

Ported: **250 functions**: 244 `#[hook]` registrations + 6 ported but not hooked / unhookable.
Verdicts: **L2 60**, **implemented 56** (0 mismatches everywhere; undeclared writes or a truncated split spool), **implemented-needs-L2 128** (no case in the grove walk), **ported-not-hooked 4**, **ported-unhookable 2**; 17 rows skipped (1 FYL2X, 16 size / time). Per module (L2 / implemented / needs-L2): occlusion 3/0/1, renderer 0/1/0, bullettraces 0/1/2, glass 6/2/8, stencilshadows 4/6/3, fx/regular 8/24/73, waterlevel 13/4/18, timecycle 16/4/2, markers 5/10/15, carplates 5/4/6.

## Files

| file | contents |
|---|---|
| `occlusion.rs` (extended) | `COcclusion::AddOne`, `ProcessBeforeRendering`, `IsPositionOccluded`, `COccluder::ProcessLineSegment` (protection chunk 0x401759 `cl = flags[corner1]` inlined; entry `mov eax,[esp+4]; nop; jmp`) |
| `renderer.rs` (extended) | `CRenderer::RequestObjectsInDirection` |
| `bullettraces.rs` (extended) | `CBulletTraces::Init`, `AddTrace-Wrapper` (0x726AF0, three `rand` sites attributed 0x726C3C / 0x726C8F / 0x726CB9), `Render` |
| `glass.rs` | CGlass: IsObjectGlass, Init, FindFreePane, AskForObjectToBeRenderedInGlass, CalcAlphaWithNormal, Update, Render + the three poly batches, WasGlassHitByBullet (`rand` at 0x71C138), WindowRespondsToSoftCollision, FindWindowSectorList, WindowRespondsToExplosion, the `aGlassPanes` static init / dtor |
| `stencilshadows.rs` | CStencilShadows: GraphicsHighQuality, Init, Shutdown, UpdateHierarchy, the batch flush 0x710CC0, Render (volume batching), RenderStencilShadows, Process, the sun-direction setters 0x710B50 / 0x710AF0, CStencilShadowObject::Destroy, the static init / dtor |
| `fx/regular.rs` | 105 regular `FxInfo*_c` / `FxInterpInfo*_c` bodies, generated from the bytes by exact instruction-shape matching (interp `Load` wrappers, SEH destructors, scalar deleting destructors, vtable-only destructors, flag `GetValue`s, `GetValue`s that copy an interp `GetVal` result); 33 irregular bodies are not ported |
| `waterlevel.rs` (helper A) | CWaterLevel: 35 hooks (wave/level queries, block scan, level loader helpers, render rectangles/triangles, fog) + SetUpWaterFog unhookable; three partial ports with `fallback` (ADR-002) |
| `timecycle.rs`, `markers.rs`, `carplates.rs` (helper B) | CTimeCycle + CColourSet (22 hooks, CalcColoursForPoint whole), C3dMarker(s) (30), CCustomCarPlateMgr (15) |

Shared-file changes: `hooks.rs` / `lib.rs` (six new modules), `fx.rs` (`regular` part), sa-oracle `specs_render_water.rs`,
`specs_render_c.rs`, `specs_render_2.rs` (registered, empty: no function of this pass is a pure leaf),
`tools/e2e/l2-extras.txt` (`# PORT-RENDER-2` blocks), lists `tools/e2e/lists/render2-*.txt`.

## Conventions and findings (agent part)

- `ProcessBeforeRendering` keeps the original's 15-bit `m_nNextIndex` relinking (`shl/sar` sign extension, the
  `xor`/`and 0x7FFF`/`xor` field update) and re-reads `NumActiveOccluders` after every `ProcessOneOccluder`; the
  hidden-occluder removal moves the tail with `rep movsd` semantics and restarts the inner loop past the end.
- `ProcessLineSegment` computes the near-plane crossing the same way for both corners (`t = |zb-1.1| /
  (|za-1.1|+|zb-1.1|)`, `t*A + (1-t)*B`); the original stores `za` and the `h` out value into its own argument
  slots (stack only; locals here). `AddOne` wraps angles into [0, 360] (exactly 360 stays), converts to radians
  and stores `_ftol(rad * 40.743664)` bytes in the order z, y, x; the sizes are taken in the order length, width,
  height.
- `RequestObjectsInDirection(posn, angle)`: rotate 90 about x (replace), `angle * 57.29578 + 180` about z
  (post-concat), translate, then `RequestObjectsInFrustum(m, 0x20)`: the streaming flags are the constant 0x20.
  The original's matrix flags word is uninitialised stack ORed with 0x20003 and its pad words are uninitialised;
  the port uses 0x20003 and zero pads (the same class as the PORT-RW RwMatrix pad decision).
- `CBulletTraces::AddTrace-Wrapper`: a second random length replaces the first when the first is shorter than the
  remaining length (the original's own logic). `AddTrace` itself (0x723750) executes FYL2X: skipped.
- `CBulletTraces::Render`, `CStencilShadows::Render`: the stack vertices' normals / UVs are uninitialised in the
  original, zero here (seen only by the D3D stream). `CStencilShadows::Render` uses the stored index count as the
  index base (equal to the vertex count unless a flush happened between).
- `CGlass::Init` sums the first two pane polygons p0+p1+p2 and the other three p2+p1+p0. `CGlass::IsObjectGlass`
  clears only AL on the non-object path. `CGlass::Render` builds the fog colour with ORs of the 16-bit components.
- `CStencilShadows::RenderStencilShadows` darkens with `CRGBA(0,0,0, strength * 0x32 >> 8)` (dword read of the
  time-cycle shadow strength). The catalogue names of 0x710B50 (`RenderBuffer`) and 0x710CC0 (`sub_710CC0`) are kept
  although the bytes set the sun direction and flush the batch.
- `fx/regular.rs`: `GetValue`s copy `out[0]` through `fld`/`fstp` (a signalling NaN is quieted, `renderer::quiet`)
  and the rest as dwords; SEH frames are not reproduced (torn down before return, fs:0 unchanged).

Helper findings (bytes over gta-reversed) are in the module docs; the main ones: CWaterLevel::AddWaterLevelQuad
reads uninitialised stack with 1 or 3 vertices on the minimum-y line (that case runs the original body through
`fallback`), the flat-water OneLayer renderers fall back for layers other than 0/1, GetWaterLevel calls
AddWaveToResult also without waves when `level - z <= 3`; CTimeCycle::CalcColoursForPoint truncates the
below-horizon grey to a byte and computes the far clip above 200 m as `(z-200) * (1/300)`; PlaceMarker's bob is
`sin(rad(angle)) * 0.3` (gta-reversed has `sin(rad * 0.3)`); GeneratePlateText can write up to `len + 3` bytes.

## Not hookable / not ported

- Not hooked: CStencilShadowObject::Constructor (3-byte entry), FxInfoSelfLit_c::Load (`ret 8`), CTimeCycle::Shutdown
  and `__dtor_CTimeCycle__m_BelowHorizonGrey` (single `ret`). Unhookable: CWaterLevel::SetUpWaterFog (protection reads
  its stub displacement), CCustomCarPlateMgr::GetCharacterPositionInCharSet (character in AL: no `#[hook]`
  convention for it).
- FYL2X: CBulletTraces::AddTrace (0x723750). No FPTAN function was met.
- Not ported (size / time): COccluder::ProcessOneOccluder (5,008 bytes), CStencilShadows::RegisterStencilShadows,
  RenderForVehicle, RenderForObject, CGlass::BreakGlassPhysically, WindowRespondsToCollision,
  HasGlassBeenShatteredAtCoors, CWaterLevel::RenderWater, RenderWakeSegment, RenderBoatWakes, WaterLevelInitialise,
  SplitWaterTriangleAlongX/YLine, the two high-detail layer workers 0x6E91D0 / 0x6E8780, 33 irregular FX info
  bodies (the GetValue workers with arithmetic, FxInfoManager_c::Load / AddFxInfo, the interp `Load` / `GetVal`),
  and the large shadow / post-effect / corona / cloud / weather workers listed in pass 1.

## Verification (lane 1, `SA_ROOT=/home/ghosty2004/sa-engine`)

Runs from a source snapshot of the worktree (`target/snap-src`, own target dir) so the helpers' files in progress
could not break a run. Lists: `render2-l2.txt` / `render2-split.txt` (agent), `render2-fx-*.txt`,
`render2-water-*.txt`, `render2-b-*.txt`.

| run | tags | result |
|---|---|---|
| plain l2, agent hooks, 4/frame then 1000/frame | `r2l2-g`, `r2b-l2` | l2 == plain and rust == plain IDENTICAL; 0 mismatches; undeclared writes of ProcessBeforeRendering / ProcessLineSegment / UpdateHierarchy (identical on both sides) → extras; ProcessLineSegment first failed to install (`expect` missed the `nop` before its `jmp`), fixed |
| plain l2, agent + FX (1000/frame) | `r2c-l2` | 0 mismatches (6,302 cases); AddOne undeclared → extras |
| plain l2, all 106 plain-l2 hooks of the pass (1000/frame) | `r2d-l2`, then `r2e-l2` with the extras | l2 == plain, rust == plain IDENTICAL (5,200 frames, 17 subsystems); **0 mismatches in 34,151 cases** (CalcColoursForPoint, GetWaterLevel, the wave functions included); `r2e`: every hook PASS or NOCALLS except CTimeCycle::Update / CalcColoursForPoint (undeclared m_VectorToSun 0xB7CA50.., extras added after the run, not re-run) and AddWaterLevelTriangle / Quad (only nested calls: 0 cases) |
| split, all 138 split hooks (`SPLIT_PER_FRAME=20`, `--max 300`, `SPLIT_STACKFILL=16384`) | `r2d-sp` | splito / splitr / rust traces IDENTICAL to plain; split-report 61 PASS, 3 FAIL, 1 DIVERGED, 3 COUNT, 70 NOCASES, **0 output mismatches except the 3 FAIL**: FxInfoFloat/Underwater/HeatHaze_c::Load returned the callee's leftover EAX (vtable vs stack) → the Load wrappers are void now; DIVERGED CStencilShadows::Render: its Rust caller passes a Rust stack CRGBA (input pointer) → extras declare the pointee; COUNT: the three marker draws hit the 2 GiB spool quota at frame 482 (cases before it compared, 0 mismatches) |
| split rerun of those six | `r2f-sp` | all PASS, 0 mismatches; the three Loads 0 undeclared (L2) |
| all engine ports on Rust, 11 recordings | `render2-final2` (lane 1) | **IDENTICAL on all 11** (grove 5200, air-water 5800, anim-input 3300, car-ai 5000, c-bug 3220, driving 4338, menus-hud 5700, pause-restart 3700, police-gang 5200, weapon-combat 3500, world-cheats 3276 frames; 17 subsystems; build = the committed code) |

The undeclared writes of the split PASS hooks (RW objects, heap material colours, stack info structs, the
FX interp key buffers the Load wrappers fill, the marker / plate RW objects) are identical on both sides; they need
narrow regions before those hooks can reach L2, as in pass 1.

Other checks: `cargo test -p sa-render` 3/3; `cargo test --release -p sa-hook --lib` 51/51 (install, catalogue,
callers, keycells, data_reads, patch_refs); clippy `-D warnings` sa-render host (all targets) and i686
(`--features in-process`), sa-hook i686, sa-oracle host (all targets); `cargo build -p sa-render --target
wasm32-unknown-unknown`. `cargo test --release -p sa-oracle --test l1`: 15/16; `l2_specs_pass_on_the_cpu` fails on
`RwMatrixRotate` 0x7F1FD0 (FSIN/FCOS, 2,476 memory differences vs the CPU), a PORT-RW spec, not a RENDER hook.

## Decisions needed

- Narrow regions for the split PASS hooks with undeclared writes (56 `implemented`), as pass 1's open item.
- `CCustomCarPlateMgr::GetCharacterPositionInCharSet` takes its argument in AL: a `#[hook]` register convention
  would be needed to hook it.
- `RequestObjectsInDirection`: RwMatrix flags / pad words from uninitialised stack → the PORT-RW normaliser decision.
- CWaterLevel mid-CRenPar padding (+0xE/+0xF): the original lerps into uninitialised locals; the port writes 0.
- 0x8D37D5 is `SPEED_FX` in posteffects.rs but `m_bWaterFogScript` in gta-reversed (RenderWaterFog reads it by address).

