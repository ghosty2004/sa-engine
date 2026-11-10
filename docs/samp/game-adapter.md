# SAMP-API: the game adapter between samp-rs and the game

Status: design (ROADMAP SAMP-API, 2026-11-11). Types and traits: `crates/sa-samp-api` (no implementation).
Nothing in `~/samp-rs` was changed. Every samp-rs file name below is relative to
`~/samp-rs/crates/samp-client/src/`.

## 1. Goal and constraints

The user wants to join SA:MP servers (client 0.3.7-R4, open.mp 1.6.7 and samp-server 0.3.7) from a
**native Linux build** of sa-engine, with the Rust SA:MP client `~/samp-rs`. Today samp-rs is an i686
Windows DLL injected into `gta_sa.exe` 1.0 US. It reaches the game through 622 distinct addresses
(811 `samp-rs` rows in `catalog/samp-usage.csv`: 157 hook rows, 96 calls, 99 globals, 62 struct fields,
104 SCM opcodes, 194 `patch-modify` and 55 `patch-disable` rows) plus samp.dll's 384 patch rows.

Constraints:

1. **samp-rs keeps working with the original `gta_sa.exe`.** Its current address-based code becomes the
   *original backend* and stays the default. No migration step may change what the original path does;
   every step is checked with samp-rs's own parity tools (tracer diff, D3D call-stream diff, RPC harness).
2. A second implementation of the same interface runs on sa-engine: **phase A** in-process under Wine
   (sa-hook replaces functions with Rust ports, the memory layout stays the original's), **phase B**
   native (no `gta_sa.exe`, no fixed addresses).
3. The engine never runs samp-specific code paths unless "samp mode" asks for them (PLAN F13): every
   samp patch becomes an engine option whose default is single-player behaviour.

Inputs used: `catalog/samp-usage.csv` and `docs/samp/README.md` (SAMP-1 static inventory),
`docs/samp/dynamic.md` + `catalog/samp-priority.csv` (SAMP-1D, what runs under SA:MP),
`docs/samp/scm-opcodes-dynamic.csv`, samp-rs sources (read only) and samp-rs
`docs/re/subsystems/gta-patches.md`.

## 2. Architecture

```
          samp-rs                                         backends (implement GameAdapter + SampHost)
 ┌──────────────────────────────┐                ┌──────────────────────────────────────────────────┐
 │ samp-proto, samp-ui, samp-saa│                │ original  : samp-rs `adapter::original` (today's │
 │ samp-core (new: client logic,│  &mut dyn      │             memory/patch/hook code), gta_sa.exe  │
 │   no addresses, no cfg(win)) │ ─GameAdapter─▶ │ engine A  : sa-engine `sa-samp` crate in         │
 │                              │                │             sa_hook.dll, same layout, hooks via  │
 │   impl GameHooks, FileProvider│ ◀─GameHooks── │             sa-hook dispatch points              │
 └──────────────────────────────┘                │ engine B  : sa-engine `sa-samp` crate, native,   │
                                                 │             engine-owned state, typed handles    │
                                                 └──────────────────────────────────────────────────┘
```

* `sa-samp-api` (this repository, `crates/sa-samp-api`) has **no dependencies**, so samp-rs can depend on
  it without pulling in any engine crate. It builds for host, `i686-pc-windows-gnu` and `wasm32`.
* **`GameAdapter`** (backend → samp-rs): the union of 15 feature traits (§4). Reads take `&self`, writes
  `&mut self`.
* **`GameHooks`** (samp-rs → backend): one callback per hook site group (§5). Every callback names its
  call site with a typed enum, which replaces samp-rs's continuation addresses and return-address tests.
  Every method has a single-player default, so samp-rs can move hooks over one at a time.
* **`SampHost`**: `install(config, hooks, files)` once, then `start_game()`.
* **`SampModeConfig` / `RuntimeToggle` / `SampMode`**: samp.dll's patches as data (§6).
* **`FileProvider`**: samp.saa redirection (§4.11).

How each backend implements the traits:

| | original (default) | engine phase A (in-process, Wine) | engine phase B (native) |
|---|---|---|---|
| where the code lives | samp-rs `adapter/original` (moved from today's modules) | sa-engine crate `sa-samp` (new), linked into `sa_hook.dll`; samp-rs DLL loaded next to it | `sa-samp` in the native sa-engine binary; samp-rs linked as an rlib |
| handles | `CPed*`/`CVehicle*`/`CObject*` addresses (unchanged) | same (layout is the original's, ADR-001) | generational pool references |
| reads/writes | `ProcessMemory` at the addresses of §4 | `sa_core::globals!` / `GPtr` accessors (same addresses in phase A) | engine state through the same accessors (ADR-001: phase B storage) |
| typed calls | the `orig` function addresses or `Runner` + SCM opcode | `sa_core::orig!` (Rust port when installed, original otherwise) | the Rust port |
| hooks | byte patches, call retargets, vtable swaps (today) | **dispatch points**: a ported function calls `samp::hooks()` at the site; for unported functions sa-hook installs the original-style hook that calls the same dispatcher | dispatch points only |
| samp patches | samp.dll's bytes (today's tables) | ports read `SampModeConfig`; byte patches only for still-original functions | ports read `SampModeConfig` |
| D3D / overlay | D3D9 device proxy (today) | D3D9 device proxy (the game still renders through D3D9 under Wine) | wgpu overlay (`sa-rw-wgpu`) |
| files | kernel32 import-slot hooks (today) | same | `sa_platform::fs::GameFs` overlay asks `FileProvider` |

Phase A is the bridge: because the layout is identical, samp-rs's *original backend* already works in
phase A as long as the hooked functions are not yet ported. Once sa-hook replaces, say, `CWeapon::Fire`
with a Rust port, a byte patch at 0x7424CB no longer has an effect; the port must call the
`GameHooks::fire` dispatcher itself. That is why every hook is defined by meaning (callback +
`FireSite`) and not by address.

## 3. Threading and frame contract

The original game runs its logic on one thread (plus the CdStream reader thread). samp-rs today uses
four contexts: its init thread (waits for `gGameState == 7`, applies patches, then services its control
channel), the game thread (frame hooks and mid-frame hooks), its network/audio worker threads, and the
D3D `Present` path on the game thread. The contract:

1. **Install.** `SampHost::install` is called once. Original backend: from samp-rs's init thread at
   `gGameState == 7` (frontend idle, the moment samp.dll applies its main block), while the game thread
   idles in the frontend loop. Engine backends: from `main` before the world is created. Afterwards
   `start_game()` (menu flags, `gGameState := 8`).
2. **Only the game thread touches the game.** Every `GameAdapter` method is called on the game thread
   and only inside a `GameHooks` callback, which receives the adapter as an argument. samp-rs never stores
   the adapter. Network threads only exchange messages with the client tick (as today).
3. **Frame callbacks** (`GameHooks::frame(phase, &mut dyn GameAdapter)`) run at fixed points, in order:

   | `FramePhase` | original site | what samp-rs does there today |
   |---|---|---|
   | `BeforeGameProcess` | call `Idle` → `CGame::Process` 0x53E981 (retargeted) | frame timer and limiter (`frametimer.rs`), object pre-step |
   | `CameraProcess` | call 0x53C104 → `CCamera::Process` 0x52B730 | camera follow of attached objects, before the game's camera |
   | `AfterGameProcess` | same hook, after the call | first-frame ped flags, chat render target, model previews |
   | `Before2d` | call `Idle` → `Render2dStuff` 0x53EB12 (retargeted) | `tick()`: netgame tick, audio frame, UI entry, quit |
   | `After2d` | after `Render2dStuff` | textdraws (`ui::frame_after_2d`) |
   | `Present` | D3D9 proxy `EndScene`/`Present` | chat, dialogs, scoreboard, nametags, labels, cursor |

   All adapter calls are allowed in frame callbacks, with these limits: `Text2d` only in `After2d`;
   `Overlay` only in `Present` (`render_model_preview` in `AfterGameProcess`).
4. **Mid-frame callbacks** (all other `GameHooks` methods) run inside game code: a vehicle's
   `ProcessControl`, `CWeapon::Fire`, `CPed::Render`, `CFileLoader::LoadObjectInstance` and so on. They
   receive `&dyn GameAdapter`, so only `&self` reads compile. Anything else is returned through the
   callback's typed result (`HookAction`, `Option<…>`, `&mut Shot`, `&mut Damage`, `VehicleControl`) or
   queued by samp-rs for the next frame callback. This is the pattern samp-rs already follows (its
   `SHARED` state written by the client tick, and the `HookEvent`/`Outgoing` queues of `vehhooks.rs`,
   `combathooks.rs` and `worldhooks.rs`).
5. **Per-player context.** Around a remote player's code the backend asks
   `GameHooks::player_context(ped, site)` and applies the returned `PlayerContext` (pad, player in focus,
   camera vectors/mode/FOV, aspect ratio, weapon mode, weapon skills, aim-rotation and `UpdatePosition`
   switches) for exactly that call, restoring the local values afterwards. In the original backend this is
   `swap.rs` (save/write/restore of the documented globals). The native engine can instead pass the
   context to `CPad::GetPad`/`TheCamera` users directly; the observable behaviour must be the same.
6. **Faults.** The original backend keeps samp's SEH guards (`swap::guarded_thiscall`) around
   `CGame::Process`, remote `ProcessControl` and every script command, and reports them as
   `GameError::Fault`. Engine backends never fault into samp-rs; a panic in a port is a bug.
7. **Streaming thread.** Only `FileProvider` (`Send + Sync`) is called off the game thread (IMG reads).
   samp-rs's CdStream lost-wake-up fix (`cdstream.rs`, hook 0x156CDAE) is not part of the API: it is a
   game bug fix the engine already has (INF-15, ROADMAP divergence ledger) and the original backend keeps.
8. **Frame timing.** samp's limiter writes `CTimer::ms_fTimeStep`/`NonClipped`, the pad counter and sleeps
   with kernel32 `Sleep` before `CGame::Process`. In the API this is `BeforeGameProcess` plus
   `Input::set_pad_counter`; the time-step override is an engine-internal concern of samp's limiter, which
   the native backend exposes through its own clock (open question Q5).

## 4. The trait surface, by feature

Rust signatures are in `crates/sa-samp-api/src/*.rs`; every method's doc comment names the original
function, global or SCM opcode it stands for. The tables group methods and give: samp-rs call sites,
gta_sa addresses, and how the two kinds of backend implement them. "opcode" means the original backend
runs it through samp-rs's `Runner` (`CRunningScript::ProcessOneCommand` 0x469EB0) and the native backend
calls the ported opcode handler (or a direct port of what the handler does).

### 4.1 Peds and players (`peds::Peds`)

| methods | samp-rs today | gta_sa | original backend | engine backend |
|---|---|---|---|---|
| `local_player`, `player_ped`, `player_in_focus`, `ped_script_handle`, `ped_from_script_handle` | `game.rs` (`PLAYER_PED`, `PED_POOL`), `client/gta.rs`, `swap.rs`, `audio.rs` | `CWorld::Players` 0xB7CD98 (+0x0 `m_pPed`), `PlayerInFocus` 0xB7CD74, `FindPlayerPed` 0x56E210, ped pool 0xB74490 | memory reads | `CWorld`/`CPools` ports (L2 already: `FindPlayerPed`) |
| `ped_in_world`, `is_ped_model` | `client/gta.rs::ped_in_world`, `game.rs::is_ped_model` | vtable `CPlaceable` 0x863C40, `CPhysical::m_pMovingList` +0xB4, `CPedModelInfo` vtable 0x85BDC0, `ms_modelInfoPtrs` 0xA9B0C8 | memory | entity liveness from the pool; model-info kind |
| `create_player_ped` | `players.rs`, `client/spawn.rs` | opcodes 0053, 01F5; `CPlayerPed::SetInitialState` 0x60CD20; `CAEPedSpeechAudioEntity::Initialise` 0x4E68D0 (`CPed::m_pedSpeech` +0x294) | opcode + calls | ported handler; needs `Limits::player_slots` |
| `create_actor`, `destroy_ped`, `set_ped_model` | `entities.rs`, `players.rs`, `client/*.rs` | opcode 009A, `CClothes::RebuildPlayer` 0x5A82C0 (gated by `RuntimeToggle::ClothesRebuild`), opcode 070D | opcode | ported handler |
| `ped_snapshot`, `ped_weapons` | `game.rs::read_player`, `read_local_sample`, `footsync.rs` | `CPlaceable::m_matrix` +0x14, `m_vecMoveSpeed` +0x44, `m_ePedState` +0x530, `m_fHealth` +0x540, `m_fArmour` +0x548, heading +0x558/+0x55C, `m_pVehicle` +0x58C, `m_aWeapons` +0x5A0, `m_nSelectedWepSlot` +0x718, `m_nModelIndex` +0x22, flags +0x46C…, `m_pIntelligence` +0x47C | struct reads | field reads of the ported `CPed` |
| `bone_position`, `weapon_skill`, `ped_on_fire`, `anim_playing` | `combathooks.rs`, `ui/glue/*.rs`, `behaviourclient.rs` | `GetBonePosition` 0x5E4280, `GetTransformedBonePosition` 0x5E01C0, `GetWeaponSkill` 0x5E6580, `m_pFire` +0x730, `GetAnimAssociation` 0x4D3A60 | calls | ports |
| `set_ped_position/matrix/move_speed/heading/health/armour`, `set_ped_flags` | `players.rs`, `remote.rs`, `client/*.rs`, `dll.rs::first_frame_ped_flags` | same fields; opcode 0173; `CMatrix::UpdateRwMatrix` 0x59AD70, `CEntity::UpdateRwFrame` 0x532B00 | writes (+ RW frame update) | port setters (`CPlaceable::SetMatrix` etc.) |
| `give_weapon`, `set_current_weapon`, `clear_weapons`, `set_weapon_slot`, `remove_weapon_entering_vehicle` | `game.rs`, `combatclient.rs`, `client.rs` | 0x5E6080, 0x5E6280 / opcode 01B9, 0x5E6320, `CWeapon::m_nState` +0x4, 0x5E6370 | calls/writes | ports |
| `set_proofs`, `set_suffers_critical_hits`, `set_drops_weapons_when_dead`, `freeze_ped`, `set_ped_area`, `set_decision_maker`, `set_player_money`, `set_player_control`, `set_drunkenness`, `explode_head` | `client.rs`, `entities.rs`, `players.rs` | opcodes 02AB, 0446, 087E, 04D7, 0860, 060B, 03FE (+ `Players[0]+0xB8` 0xB7CE50), 01B4, 052C, 0321 | opcode | ported handlers |
| `extinguish_ped` | `client.rs`, `combathooks.rs` | `CFire::Extinguish` 0x5393F0 | call | port |
| `warp_into_vehicle`, `warp_out_of_vehicle` | `vehicles/enter_exit.rs`, `client/*.rs` | opcodes 036A, 0430, 0362 | opcode | ported handlers |
| `start_task` | `vehicles/*.rs`, `client.rs`, `specials.rs`, `behaviourclient.rs` | opcodes 05CB, 05CA, 05CD, 05C4, 0729, 0713, 070A, 07FE; `SetTaskDuckSecondary` 0x601230 | opcode / call | ported handlers / task ctors |
| `flush_tasks`, `delete_task` | `entities.rs`, `vehhooks.rs`, `vehicles.rs`, `game.rs` | `FlushImmediately` 0x601640; `CTask::operator delete` 0x61A5B0; deleting dtors 0x6275E0, 0x6801D0, 0x639330 | calls | ports (Rust ownership) |
| `play_anim`, `stop_anims` | `behaviourclient/animation.rs`, `pedanims.rs`, `anims.rs` | opcode 0812; `BlendAnimation` 0x4D4610 | opcode / call | ports |
| `look_at` | `players.rs` | `IKChainManager_c::LookAt` 0x618970 on `g_ikChainMan` 0xC15448 | call | port |
| `set_jetpack`, `has_jetpack` | `players.rs`, `game.rs` | `CTaskSimpleJetPack` vtable 0x8705C4, dtor 0x6801D0 | task scan | task tree query |

### 4.2 Vehicles (`vehicles::Vehicles`)

| methods | samp-rs today | gta_sa | original | engine |
|---|---|---|---|---|
| `vehicle_script_handle`, `vehicle_from_script_handle`, `local_vehicle` | `vehicles/mem.rs`, `vehicles/mod.rs` | pool 0xB74494, unnamed `POOL_GET_AT` 0x4048E0 | memory/call | pool port |
| `vehicle_snapshot`, `vehicle_damage`, `vehicle_in_water` | `vehicle.rs`, `vehlabels.rs`, `vehicles/local_sync.rs` | class vtables 0x871120 … 0x872370, health +0x4C0, driver +0x460, max passengers +0x488, opcode 02BF | struct reads + vtable compare | `VehicleClass` from the type |
| `create_vehicle`, `destroy_vehicle` | `vehicles/stream.rs`, `vehicles/rpc.rs` | opcodes 00A5, 00A6, 06D8, 07BD | opcode | ported handlers; `Limits::vehicles` = 710 |
| `set_vehicle_matrix/speed/heading/health`, `set_train_position/speed` | `vehicles/remote_apply.rs` | fields; opcodes 0175, 07C7 | writes/opcode | port setters |
| `set_vehicle_damage`, `fix_vehicle` | `vehicles/damage.rs` | `SetupDamageAfterLoad` 0x6B3E90, `CAutomobile::Fix` 0x6A3440 | calls | ports |
| `set_vehicle_colours`, `set_paintjob`, `add_component`, `remove_component`, `set_hydraulics`, `set_vehicle_proofs`, `set_tyres_can_burst`, `set_petrol_tank_weakpoint`, `set_vehicle_area` | `vehicles/*.rs`, `vehicles/modshop.rs` | opcodes 06ED, 06E7, 06E8, 07FF, 02AC, 053F, 09C4, 0840; colour table relocation (`Limits::vehicle_colours`) | opcode | ported handlers |
| `set_door`, `set_doors_locked`, `set_window_open`, `set_landing_gear`, `set_siren`, `set_engine_lights` | `vehicles/*.rs` | `OpenDoor` 0x6A6AE0, `SetWindowOpenFlag` 0x6D3080, `ClearWindowOpenFlag` 0x6D30B0, `SetGearUp/Down` 0x6CAC20/0x6CAC70, lights byte +0x428 bit 0x40 | calls/writes | ports |
| `set_trailer` | `vehicles/trailers.rs` | opcodes 0893, 07AC | opcode | ported handlers |
| `set_plate_text` | `plates.rs` | hooks 0x6FDED6 (`CreatePlateTexture` → `RwRasterCreate` 0x7FB230), `RwRasterDestroy` 0x7FB020 (slot 0x59C721), `RwEngineInstance` 0xC97B24; D3D `SetTexture` substitution | D3D texture + proxy | engine renders the plate text into the plate texture |

### 4.3 Objects (`objects::Objects`)

| methods | samp-rs today | gta_sa | original | engine |
|---|---|---|---|---|
| `object_script_handle`, `object_from_script_handle`, `object_matrix`, `object_has_rw`, `object_attached`, `object_bounds` | `objects.rs`, `objects/*.rs`, `worldhooks.rs` | pool 0xB7449C, vtable `CObject` 0x866F60, `m_pRwObject` +0x18, opcode 0685, `GetBoundCentre` 0x534250, col model +0x24/+0x2C | memory/opcode/call | ports |
| `object_ray_test` | `objects.rs` (picking) | `TestLineSphere` 0x417470, `SphereCastVsEntity` 0x419F00, `gpColCache` 0x9655CC | calls | `sa-coll` port (COLL is mostly L1+) |
| `create_object`, `destroy_object`, `set_object_transform`, `set_object_scale`, `set_object_visible`, `attach_object`, `detach_object`, `play_object_anim` | `objects.rs`, `objects/motion.rs`, `specials.rs` | opcodes 0107, 0108, 09A2, 0453, 08D2, 0750, 0681, 069A, 069B, 0682, 075A | opcode | ported handlers; `Behaviour::create_object_negative_models` |
| `set_object_matrix`, `set_object_collision` | `objects/motion.rs` | `CPlaceable` matrix, `CEntity::UpdateRwFrame` 0x532B00, `CPhysical::PositionAttachedEntity` 0x546FF0 | writes/calls | ports |
| `set_object_material` | `objects/materials.rs`, `worldhooks/materials.rs`, `ui/mattext.rs` | applied in samp's `CObject::Render` hook (0x59F180, slot 0x866FA8): `RpClumpForAllAtomics` 0x749B70, raster ext 0xB4E9E0, TXD calls | today: per-render material swap in the hook | backend keeps an override table per object and applies it in its renderer |
| `remove_buildings` | `objects.rs` (RPC 43), `worldhooks/buildings.rs` | `LoadObjectInstance` hook 0x538090 (slot 0x538084), dummy pool 0xB744A0, `COcclusion::Occluders` 0xC73FA0 | memory + hook | world port drops instances; plus `GameHooks::load_instance` |
| `camera_ignore` | `worldhooks.rs` | `CWorld::CameraToIgnoreThisObject` 0x563F40, `pIgnoreEntity` 0xB7CD68 | call | port |

### 4.4 World state and entities (`world::WorldState`, `world::WorldEntities`)

| methods | samp-rs today | gta_sa | original | engine |
|---|---|---|---|---|
| `clock`, `set_clock` | `world.rs`, `client.rs` | 0xB70152/0xB70153, opcode 00C0 | writes / opcode | `CClock` port |
| `weather`, `set_weather` | `world.rs` | 0xC81318/0xC8131C/0xC81320 | writes | `CWeather` port |
| `gravity`, `set_gravity`, `set_stunt_bonus` | `behaviourclient.rs`, `client.rs` | `GAME_GRAVITY` 0x863984, SCM global 709 at 0xA4A474 | writes | engine values |
| `area`, `set_area` | `behaviourclient.rs`, `vehhooks.rs`, `worldhooks.rs` | `CGame::currArea` 0xB72914, opcodes 077E, 04BB | read / opcode | port |
| `set_max_wanted_level`, `set_densities`, `switch_roads_off`, `set_random_trains`, `set_restart_point` | `scm.rs`, `client.rs` | opcodes 01F0, 03DE, 01EB, 01E8, 06D7, 016C; samp A also writes the restart block 0xA4326C… | opcode | ported handlers |
| `ground_z`, `line_of_sight`, `line_clear` | `client.rs`, `client/gta.rs`, `entities.rs` | opcode 02CE, `CWorld::ProcessLineOfSight` 0x56BA00, opcode 06BD (missing from samp-rs's list, dynamic.md §4) | opcode/call | `CWorld` port (sector relocation not needed natively) |
| `add_explosion`, `create_fx`, `kill_fx`, `load_shop` | `combatclient.rs`, `specials.rs`, `behaviourclient.rs` | opcodes 0948, 066A, 064C, 0650; `CShopping::LoadShop` 0x49BBE0 | opcode/call | ports |
| pickups: `create_pickup`, `remove_pickup`, `pickup_collected`, `pickup_index` | `entities.rs`, `worldents.rs`, `worldhooks/pickups.rs` | opcodes 0213, 0215, 0214, `CPickups::aPickUps` 0x9788C0, `CPickup::IsVisible` patch 0x454CC9 | opcode + hook | ports |
| checkpoints: `create_checkpoint`, `delete_checkpoint`, `locate_ped` | `entities.rs` | opcodes 06D5, 06D6, 00FE | opcode | ported handlers |
| blips: `add_blip`, `remove_blip`, `set_blip_colour/scale/display` | `players.rs`, `entities.rs`, `worldents.rs` | opcodes 0187, 0161, 02A8, 04CE, 02A7, 0570, 0164, 0165, 0168, 018B | opcode | `CRadar` port |

Gang zones are not an adapter method: samp-rs draws them itself from `GameHooks::radar_overlay` through
a `RadarPainter` (`CRadar::DrawAreaOnRadar` 0x5853D0), as it does today.

### 4.5 Camera (`camera::Camera`)

| methods | samp-rs today | gta_sa | original | engine |
|---|---|---|---|---|
| `camera` | `behaviourclient/camera.rs`, `client.rs`, `swap.rs`, `entities.rs`, `vehlabels.rs`, `ui/glue/*.rs` | `TheCamera` 0xB6F028; `m_mCameraMatrix` pos 0xB6F9CC; `Cams[0]` front 0xB6F32C, source 0xB6F338, mode 0xB6F1A8, FOV 0xB6F250, looking 0xB6F1A4/5; `CDraw::ms_fAspectRatio` 0xC3EFA4; weapon mode 0xB6F858 | memory | `CCamera` port |
| `world_to_screen`, `sphere_visible` | `ui/render/world.rs`, `worldhooks.rs` | `RwCameraFrustumTestSphere` 0x7EE2D0 on `Scene.camera` 0xC1703C | samp-rs's own projection + call | engine camera |
| `set_fixed_position`, `point_at`, `point_at_ped`, `point_at_vehicle`, `behind_player`, `restore`, `interpolate`, `set_widescreen` | `behaviourclient/camera.rs`, `camtarget.rs`, `client.rs` | opcodes 015F, 0160, 0159, 0158, 0373, 02EB, 0936, 0920, 0930, 092F, 0925, 02A3 | opcode | ported handlers (+ `Behaviour::point_camera_at_char_ped_mode`) |
| `shake`, `set_fov` | `combathooks.rs`, `swap.rs` | `CamShakeNoPos` 0x50A970, FOV 0xB6F250 | call/write | port |

### 4.6 HUD, 2D text and menus (`hud::Hud`, `hud::Text2d`, `hud::Menus`)

| methods | samp-rs today | gta_sa | original | engine |
|---|---|---|---|---|
| `set_hud_visible`, `menu_active` | `client/gta.rs::set_hud`, `ui/glue/adapters.rs`, `audio.rs` | `FrontEndMenuManager+0x21` 0xBA6769, `CHud::bScriptDontDisplayRadar` 0xBAA3FB, `+0x5C` 0xBA67A4 | writes | engine flags |
| `big_message`, `clear_prints`, `set_zone_names`, `add_score`, `expand_game_text`, `gxt` | `behaviourclient.rs`, `ui/glue/*.rs`, `ui/textdraws.rs` | `AddBigMessage` 0x69F2B0, opcodes 00BE, 09BA, 0109, `InsertPlayerControlKeysInString` 0x69E160, `InsertNumberInString` 0x69DE90, `CText::Get` 0x6A0050 | calls/opcode | `CMessages`/`CText` ports |
| `Text2d::print`, `text_width` | `ui/textdraws.rs` | 14 `CFont::Set*` 0x719380…0x719610 then `PrintString` 0x71A700; HUD scale floats 0x859520/0x859524 | the same 15 calls in samp's order | `CFont` port; `FontStyle` carries the whole state |
| `load_txd`, `sprite`, `draw_sprite`, `release_sprite` | `ui/textdraws.rs` | `CTxdStore` 0x731850, 0x731C80, 0x7320B0, 0x731A00, 0x7316A0, 0x7319C0, 0x7316B0; `CSprite2d` 0x727270, 0x728350, 0x7281E0 | calls | ports (`CTxdStore::FindTxdSlot` already L2) |
| `Menus::show_menu`, `menu_selection`, `delete_menu` | `ui/menus.rs` | opcodes 08D4, 08DB, 09DB, 08D9, 08D7, 08DA (all 6 missing from samp-rs's opcode list, dynamic.md §4) + `CText::Get` hook 0x6A0043/0x6A0050 | opcode + GXT hook | `CMenuSystem` port; strings passed directly |

### 4.7 Input (`input::Input`)

| methods | samp-rs today | gta_sa | original | engine |
|---|---|---|---|---|
| `pad`, `set_pad`, `game_key_down`, `set_pad_counter` | `game.rs`, `ui/menus.rs`, `vehicles/*.rs`, `frametimer.rs`, `combatclient.rs` | `CPad::Pads` 0xB73458 (+0x10/+0x40 d-pad, +0x1E triangle, +0x24 shock, +0x116 counter), `NewKeyState` 0xB73190 | memory | `CPad` port |
| `key_held` | `client/gta.rs` | `GetAsyncKeyState` | Win32 | `sa-platform` input state |
| `mouse`, `clear_mouse_history`, `update_pads` | `ui/inputlock.rs` | `NewMouseControllerState` 0xB73418 (+0xC/+0x10), `ClearMouseHistory` 0x541BD0, `UpdatePads` 0x541DD0 | memory/calls | port |
| `set_cursor_visible`, `screen_size`, `native_window` | `cursorhook.rs`, `ui/render/mod.rs`, `mainhooks.rs`, `audio.rs`, `crash.rs` | `ShowCursor` import slot, `RsGlobal` 0xC17044/0xC17048, `WindowHandle` 0xC97C1C | Win32/memory | `sa-platform` window |
| input lock | `ui/inputlock.rs` | see `RuntimeToggle::Input` (§6.3) | byte patches | flag read by the `CPad` port |
| window events | `ui/wndproc.rs` (`SetWindowLongA` subclass) | `__MainWndProc` | subclass → `GameHooks::window_event` | winit events → `GameHooks::window_event` |

### 4.8 Audio (`audio::GameAudio`)

| methods | samp-rs today | gta_sa | original | engine |
|---|---|---|---|---|
| `radio_volume`, `stop_radio`, `start_radio` | `audio.rs` | 0xB5FCC8; `StopRadio` 0x506F70, `StartRadio` 0x507DC0 on `AudioEngine` 0xB6BC90 | memory/calls | AUDIO ports |
| `mission_ambience` | `audio.rs` | 0x4D6D50 / 0x4D6D60 on 0x8AC15C | calls | port |
| `one_off_sound` | `script.rs` users | opcode 018C | opcode | ported handler |
| `set_weather_sound_muted` | `audio.rs::install_weather_hook` | samp-rs's weather audio hook | hook | flag |

The audio stream itself (RPC 41/42, BASS) stays in samp-rs. On native Linux samp-rs needs a non-BASS
decoder or the Linux BASS build (open question Q7).

### 4.9 Streaming (`streaming::Streaming`)

| methods | samp-rs today | gta_sa | original | engine |
|---|---|---|---|---|
| `model_kind` | `objects.rs`, `game.rs`, `ui/preview.rs` | model-info vtables 0x85BBF0…0x85BDC0, `ms_modelInfoPtrs` 0xA9B0C8 (relocated, ±32767) | memory | model-info port |
| `request_model`, `model_loaded`, `release_model`, `load_all_now`, `request_collision`, `request_anim_block`, `anim_block_loaded`, `request_component`, `component_loaded` | `client/gta.rs::load_model`, `behaviourclient.rs`, `anims.rs`, `vehicles/*.rs` | opcodes 0247, 0248, 0249, 038B, 04E4, 04ED, 04EE, 06E9, 06EA | opcode | `CStreaming` port (RequestModel is L2) |
| `collision_loaded` | `vehhooks.rs` | `CColStore::HasCollisionLoaded` 0x410CE0 | call | port |
| `streaming_memory` | `ui/glue/adapters.rs` | 0x8A5A80, 0x8E4CB4 | memory | port |

### 4.10 Overlay rendering (`render::Overlay`)

samp-rs draws its UI with D3D9 + D3DX (`ui/d3d/*.rs`, `ui/render/*.rs`, `ui/dxut.rs`, `ui/mattext.rs`,
`ui/preview.rs`) through a proxy that replaces `_RwD3DDevice` 0xC97C28 on the first frame. Its parity
check is a D3D call-stream diff against samp.dll, so the original backend must keep issuing the same D3D
calls. The API therefore offers two levels:

* `render_api()` returns `RenderApi::Direct3D9 { device }` on the original and phase A backends, and
  samp-rs keeps its D3D renderer unchanged there;
* the `Overlay` primitives (textures, triangle lists, lines, model previews, screenshots) are what the
  native backend implements on wgpu; samp-rs adds a second renderer behind its own UI layer that uses
  them (samp-ui is already renderer-independent; glyph atlases replace D3DX fonts).

`render_model_preview` replaces `ui/preview.rs`'s off-screen RW scene (`Scene.world` 0xC17038,
`RwEngineInstance` 0xC97B24, entity vtable slots +0x08/+0x0C/+0x38/+0x44).

### 4.11 Files (`files::FileProvider`) and start-up

* samp.saa redirection: samp-rs's rules (`saafs.rs`) implement `FileProvider`; `filehook.rs` (kernel32
  import slots) becomes the original backend's way to consult it. Natively, the `GameFs` overlay asks the
  provider first. IMG reads come from the streaming thread, hence `Send + Sync`.
* `main.scm`: `StartupOptions::main_scm` (today: retarget of the open call 0x468EC9 → `CFileMgr::OpenFile`
  0x538900).
* User folder: `gta_user_dir_path` 0xC92368 (`ui/glue`, screenshots, `sa-mp.cfg`). **Note:**
  `netclient.rs` reads 0xC9236C (`gta_user_dir_path + 4`) as input for the join serial (gpci); the native
  backend must produce the same string or servers that check gpci will see a different serial (Q8).
* Version check: `game.rs` (`TABLE;VERSION_PROBE_ADDR` 0x747483, `VERSION_GIVE_UP_FLAG` 0xBA6831) and the
  samp auth handler's 20-byte read of 0x4D3AA0 (patch row 376) are original-layout facts. They stay in the
  original backend; `BackendInfo::has_original_layout` tells samp-rs whether they apply. The auth read is
  answered by the server's challenge, not the game: samp-rs must supply the expected bytes itself on
  backends without the original code (Q3).

## 5. Hooks: `GameHooks` callbacks

Every samp hook row and every samp-rs hook address belongs to one callback. "rows" are lines of samp-rs
`docs/re/data/gta-patches-r4-annotated.csv`.

| callback | sites (gta_sa) | samp-rs today | engine dispatch point |
|---|---|---|---|
| `frame(BeforeGameProcess/AfterGameProcess)` | call 0x53E981 → `CGame::Process` 0x53BEE0 | `dll.rs::game_process_hook` | `Idle` port, around `CGame::Process` |
| `frame(CameraProcess)` | call 0x53C104 → `CCamera::Process` 0x52B730 | `dll.rs::camera_process_hook` | `CGame::Process` port |
| `frame(Before2d/After2d)` | call 0x53EB12 → `Render2dStuff` 0x53E230 | `dll.rs::frame_hook` | `Idle` port |
| `frame(Present)`, `device_lost` | D3D9 proxy | `ui/d3d/proxy.rs` | renderer end of frame |
| `idle_wait` | call 0x53E930 (Sleep(1) before the 14 ms wait) | `mainhooks.rs` | `Idle` port |
| `shutdown` | `CGame::Shutdown` 0x53C900 | `dll.rs::game_shutdown_hook` | shutdown port |
| `window_event` | `__MainWndProc` subclass | `ui/wndproc.rs` | platform event loop |
| `player_context` | slots 0x86D190 (`CPlayerPed::ProcessControl` 0x60EA90), 0x86D194 (`CPhysical::ProcessCollision` 0x54DFB0), 0x86D744 (`CTaskSimpleUseGun::SetPedPosition` 0x624ED0); toggles 0x5E92F4, 0x6884C4, 0x688200 | `swap.rs` | the three ports, plus vehicle `ProcessControl` |
| `civilian_process_control` | slot 0x86C0D0 (`CCivilianPed::ProcessControl` 0x5DDBF0) | `worldhooks/peds.rs` | port |
| `ped_rendered` | slots 0x86C0F0, 0x86C168, 0x86C248, 0x86C3A0, 0x86D1B0 (`CPed::Render` 0x5E7680) | `worldhooks/peds.rs` | ped render port |
| `animation_started` | indirect jmps 0x4D3934, 0x4D4609, `BlendAnimation` 0x4D4610, `AddAnimation` 0x4D3AA0, `UncompressAnimation` 0x4D41C0; `ms_aAnimAssocGroups` 0xB4EA34 | `behaviourclient.rs`, `pedanims.rs` | `CAnimManager` ports |
| `enter_vehicle_task`, `leave_vehicle_task`, `enter_refused` | ctors 0x6402F0 (slot 0x6919BB, resume 0x6402F7), 0x63B8C0 (slot 0x63B8BA, resume 0x63B8C7, SEH scope 0x841618); caller tests 0x5703FC, 0x5704A1, 0x570A20, 0x570A99; resume 0x570A9E | `vehhooks.rs`, `vehicles/enter_exit.rs` | task ctor ports, `CPlayerInfo::Process` port |
| `fire`, `add_bullet` | `CWeapon::Fire` sites 0x742495, 0x7424CB, 0x7424EC, 0x742548 → `FireInstantHit` 0x73FB10 / `FireSniper` 0x73AAC0; `AddBullet` 0x736010 (site 0x73AC4B) | `combathooks/shot.rs` | `CWeapon` ports |
| `bullet_line_of_sight` | 0x740721, 0x740B69, 0x736247 | `combathooks/shot.rs` | same ports |
| `ped_damage`, `damage_affects_ped`, `vehicle_damage` | `ComputeDamageResponse` 0x4B5AC0 (slot 0x4B5ABC, resume 0x4B5AC6), `CEventDamage::AffectsPed` 0x4B35A0 (slot 0x4B3433, resume 0x4B35A6), `CVehicle::InflictDamage` 0x6D7C90 (continue 0x404CDC) | `combathooks/damage.rs` | ports |
| `detonate`, `satchel_update`, `take_photo` | 0x738877 (slot 0x73885B, blow 0x738880, skip 0x7388DB), 0x738F3A (slot 0x738B1B, resume 0x738F40), 0x73C252 (resume 0x73C260; `bPhotographHasBeenTaken` 0xC8A7C0/C1) | `combathooks/weapons.rs` | ports |
| `weapon_skill` | `GetWeaponSkill` 0x5E6580; `RenderWeaponPedsForPC` site 0x7330A2 | `combathooks.rs` | ports |
| `camera_shake`, `draw_crosshair` | `CamShakeNoPos` 0x50A970, sniper site 0x73ACE2; `DrawCrossHairs` 0x58E020 via 0x58FBBF | `combathooks.rs`, `mainhooks.rs` | ports |
| `vehicle_process_control` | slot +0x28 of 10 classes (0x871148, 0x871388, 0x871550, 0x8716A8, 0x871800, 0x871970, 0x871B10, 0x871C50, 0x8721C8, 0x872398); the originals are compared at 0x6B1880 … 0x6F86A0 | `vehhooks.rs` | `CVehicle::ProcessControl` dispatch |
| `vehicle_collision` | slot +0x58 of 8 classes (0x871178, 0x8713B8, 0x871580, 0x8716D8, 0x871830, 0x8719A0, 0x871B40, 0x8721F8) | `vehhooks.rs` | `ProcessEntityCollision` ports |
| `break_tow_link` | `BreakTowLink` 0x6A4400 (5 slots 0x871218 …) | `vehhooks.rs` | port |
| `uses_siren` | `UsesSiren` 0x6D8470; sites 0x4F77DA, 0x501B1D, 0x501B42, 0x501FC2, 0x502067, 0x5021AE, 0x6B2BCB, 0x6E0954; `ProcessVehicleSirenAlarmHorn` 0x5002C0 | `vehhooks.rs` | ports |
| `train_derail` | 0x6F8CF8 | `vehhooks.rs` | `CTrain` port |
| `vehicle_dirt` | 0x6D0E7E → `SetDirtTextures` 0x5D5DB0 | `vehhooks.rs` | `CVehicle::Render` port |
| `player_speed` | `FindPlayerSpeed` 0x56E090, radar site 0x586C0A | `vehhooks.rs` | ports |
| `police_scanner_ped` | 0x4E7427 → `FindPlayerPed` | `dll.rs::scanner_find_player_ped` | port |
| `render_pass` | `RenderScene` 0x53DFDD, 0x53E019; `RenderEverythingBarRoads` 0x553AA0, `RenderFadingInEntities` 0x5531E0, `RenderOneNonRoad` 0x553260, `RenderRoadsignAtomic` 0x6FF350 | `worldhooks/render.rs` | renderer |
| `object_render`, `object_streamed_in`, `entity_streamed_out` | `CObject::Render` 0x59F180 (0x866FA8), `CreateRwObject` 0x59F110 (0x866F7C), `CEntity::DeleteRwObject` 0x534030 (0x866F80, 0x8585E8) | `worldhooks/entity.rs`, `objects/streaming.rs` | entity ports |
| `load_instance` | `LoadObjectInstance` 0x538090 (slot 0x538084) | `worldhooks/buildings.rs` | `CFileLoader` port |
| `add_to_render_list`, `render_effects`, `attached_entities_done`, `vertical_line_entity`, `object_break`, `camera_collides`, `escalators` | 0x5534B0 (slot 0x5534A6), 0x5342F9, 0x5648D3, 0x5674E0 (slot 0x5674DB), 0x59E8F9 (caller test), 0x41AB78 / 0x41AF80 / 0x41B02E, 0x718599 (+ `CEscalator::Update` 0x717D30, temp floats 0x858BA4/0x858F84, call 0x71791F) | `worldhooks/*.rs` | ports |
| `pickup_collected` | `CPickup::Update` 0x4579C6 | `worldhooks/pickups.rs` | port |
| `radar_trace_colour`, `radar_overlay`, `draw_radar` | 0x584770 (slot 0x584A79), `DrawRadarGangOverlay` 0x586650 (sites 0x5869BF, rows 41/42), `PrintMap` 0x5759E4, `DrawRadar` 0x58A330 via 0x58FC53 | `worldhooks/radar.rs`, `mainhooks.rs` | `CRadar`/`CHud` ports |
| `gxt` | `CText::Get` 0x6A0050 (slot 0x6A0043, continue 0x6A0059) | `ui/menus.rs` | `CText` port |
| `cheat_char` | 0x438576 (resume 0x438581, slot 0x4385AA; `m_bHasPlayerCheated` 0x96918C) | `mainhooks.rs` | `CCheat` port (L2/implemented already) |
| `custom_opcode` | `ProcessCommands2500To2599` jump table 0x47BF54 (09C6), 0x47BF88 (09D3), unimplemented target 0x47BF2B; params via 0x464080, 0xA43C78, 0x4859D0 | `scmops.rs` | script interpreter extension point |

Not callbacks (stay inside the original backend): the sector-grid relocation (`sectors.rs`, ~100
sites), the model-info store and pointer relocations (`stores.rs`, 707 sites), the colour-table move, the
`CFont` stack-frame rewrite — these only exist to raise limits of the original binary and become
`Limits` (§6.1); the CdStream fix (§3.7); the crash handler and SEH guards (`crash.rs`, `swap.rs`).

## 6. samp.dll patches as engine configuration (feeds SAMP-4)

### 6.1 `Limits`
Pool sizes (15 rows: 0x550F46 … 0x551283, `CTxdStore::Initialise` 0x731F60, `SetupCommonData`
0x5B8FE4, `InitMatrixArray` 0x54F3A1), the atomic/ped model stores (20000/319), the signed model range
(±32767), the 256-colour palette, player slots (210 — natively just the array size, which makes the
sector-grid relocation unnecessary), streaming memory (0x5B8E6A), the `CFont` line buffer (0x32C frame)
and the bullet world bounds (±20000). The original backend applies them as today; engines size their
pools from them at init.

### 6.2 `FeatureSet` (46 static features)
The 55 disable labels of README §4.3 split into 46 init-time features (`config::Feature`, each with its
sites in the doc comment) and 9 runtime toggles (§6.3). Engines test `config.disabled.contains(f)` at the
call site the patch NOPs (for example `CGame::Process` port: `if !disabled(Replay) { CReplay::Update() }`).
SAMP-1D confirmed that none of these paths runs under samp (dynamic.md §3.3).

### 6.3 `RuntimeToggle`
Input lock (modes 2/3/4: 0x541DF5, 0x53F417/0x53F41F, 0x53F47A/0x53F49A/0x53F4B3, `RsMouseSetPos`
0x6194A0), game clock (0x52CF10 + format 0x859A6C), stat changes (0x55A070), camera fade (0x50AC20),
clothes rebuild (0x5A82C0), clump collision stream read (0x41B1D0), `CVehicle::SetupRender` (0x6D64F0),
manual vehicle lights (0x6E1BA0…0x6E1DBC), CJ walk style (0x609A4E), wanted level (0x58DB60), HUD scale
fix (0x859520/0x859524, 0x866B74/0x866B78), typed cheats. Gravity and stunt bonus are plain
`WorldState` setters. The per-player `UpdatePosition` call and aim-rotation stores are part of
`PlayerContext`.

### 6.4 `Behaviour`
The `patch-modify` rows that change logic (remote-player safety NOPs, ped camera mode for 0159, negative
model ids for 0107, constant gun-aim step, `UpdatePosition`/`AttachPedToEntity`/`GetHandBrake` changes,
weapon kept on car entry, ped-group condition, vehicle audio from the hook, stencil alpha, mouse init,
timer start-up). `remote_player_peds` is the important one: without it the engine indexes
`CWorld::Players` with −1 for remote `CPlayerPed`s.

### 6.5 `StartupOptions`
Skip logos/intro/menu (0x747483 and the `gGameState` 5/8 writes, menu flags 0xBA6831/0xBA67A4/
0xBA677B), frame limit 200 (0xC1704C), main.scm, image archives (samp disables `InitImageList` 0x4083C0
and registers its own).

## 7. Migration plan for samp-rs (small steps, original path always default)

Each step is one samp-rs branch commit (made by the samp-rs owner/main session), behaviour-neutral on
the original exe, verified with samp-rs's parity tools before the next one: `tools/trace/diff.py` (call
diff), `tools/trace/d3d_diff.py`, `tools/diff/rpc-coverage.sh`, `tools/diff/cross-client.sh`. A step
that shows any diff is reverted, not patched forward.

1. **Vendor the API.** Add `sa-samp-api` as a dependency (git dependency pinned by commit, or a vendored
   copy under `samp-rs/crates/` kept byte-identical by a check script; Q1). No code change.
2. **Original backend skeleton.** New module `adapter/original` with `struct OriginalGame` implementing
   every trait with `Err(Unsupported)`; `dll.rs` constructs it. No caller yet. Build + parity unchanged.
3. **Reads.** Implement the read methods by moving the code of `game.rs::read_player/snapshot`,
   `vehicle.rs`, `client/gta.rs`, `swap.rs` constants and `ui/glue/adapters.rs` into the backend; switch
   their callers. Pure refactor; the tracer diff must be empty (reads are not traced, so also compare
   `samp-rs.log` state lines).
4. **Script commands.** `ScriptCommands::run_command` wraps `script::Runner`; route every `runner.run`
   through it (same opcode, same args, same order — D3D/tracer diff empty). Then add the 7 opcodes
   SAMP-1D found missing (06BD and the menu family) to the opcode list.
5. **Typed methods, feature by feature** (one PR each, in this order: world/time/weather → HUD →
   streaming → peds → vehicles → objects → world entities → camera → audio): replace the opcode call or
   memory write with the typed method; inside the original backend the method issues exactly the same
   opcode/write. Diff empty after each.
6. **Frame callbacks.** Turn `game_process_hook`, `camera_process_hook`, `frame_hook`,
   `frame_after_2d` and the D3D `Present` path into calls of `GameHooks::frame(phase, game)`; the hook
   stubs stay in the backend.
7. **Mid-frame hooks, module by module** (`swap.rs` → `vehhooks.rs` → `combathooks.rs` → `worldhooks.rs`
   → `plates.rs`, `mainhooks.rs`, `ui/menus.rs`, `scmops.rs`): the naked stubs and continuation addresses
   stay in `adapter/original`; their Rust bodies call the `GameHooks` method with the typed site enum.
   Their logic moves into the client's `GameHooks` impl. The existing `SHARED`/queue design maps 1:1.
8. **Patches → config.** `patches.rs`, `vehpatches.rs`, `sectors.rs`, `stores.rs`, the input lock and the
   world/clock writes become the original backend's implementation of `SampHost::install(config)` and
   `SampMode::set_toggle`. samp-rs builds the `SampModeConfig` from its tables (one value per row).
9. **Files.** `filehook.rs` consults `FileProvider` (implemented by `saafs::Redirector`).
10. **Split the crate.** Move everything above the adapter into `samp-core` (no `cfg(windows)`, builds
    on Linux and is tested there); `samp-client` becomes the DLL shell = original backend + D3D renderer +
    BASS + crash handler. From here samp-core can be linked into sa-engine.
11. **Overlay renderer.** Add the `Overlay`-based UI renderer next to the D3D one; select by
    `render_api()`. The D3D path stays the default and parity-checked.
12. **Engine backends.** In sa-engine: crate `sa-samp` implements the traits (phase A inside `sa_hook.dll`
    with dispatch points; phase B in the native binary). samp-rs is unchanged by this step.

Steps 1–10 are pure refactors of samp-rs; steps 11–12 add code. At no step does the original exe path
lose a feature.

## 8. Engine functions to port first for the native backend

Cross-checked with `catalog/samp-priority.csv` (functions executed under SA:MP and below L1) and
`catalog/state.csv`.

**Directly named by the API** (calls, hook sites, compared originals): 185 functions (plus the excluded
protection stub 0x404CDC, a continuation address). 132 of them are in the priority list (125 `specified`,
6 `implemented`, 1 `unknown`: `POOL_GET_AT` 0x4048E0). Of the other 53: some are verified already (`CStreaming::RequestModel` L2, `CdStream::CdStreamSync` L2,
`CWorld::FindPlayerPed` L2, `CTxdStore::FindTxdSlot` L2, `CMatrix::UpdateRwMatrix` L1,
`CTimer::GetCurrentTimeInCycles` L3), the rest did not run in the SAMP-1D scenarios (for example
`CVehicle::InflictDamage`, `CWeapon::FireSniper`, `CBulletInfo::AddBullet`, trailers, monster truck, quad
bike, `RwRasterDestroy`) and stay required because the API names them. **`CRunningScript::ProcessOneCommand`
(0x469EB0) is executed but missing from the priority CSV**, because the SCM counter detoured its entry;
it is the most important single function here.

Order (each tier also needs its callees from the priority list):

1. **Frame skeleton and timing** (CORE): `Game::Idle` 0x53E920, `CGame::Process` 0x53BEE0,
   `Game::RenderScene` 0x53DF40, `Game::Render2dStuff` 0x53E230, `CCamera::Process` 0x52B730,
   `CGame::Shutdown` 0x53C900, `CFileMgr::OpenFile` (implemented). Gives every `FramePhase`.
2. **Script interpreter** (SCRIPT): `ProcessOneCommand` 0x469EB0, `CollectParameters` 0x464080,
   `UpdateCompareFlag` 0x4859D0, `CTheScripts::Init` 0x468D50, the `ProcessCommandsXToY` range functions
   (26 for samp-rs's list) holding the 78 executed opcodes (0x465E60 … 0x47A760; list in `scm-opcodes-dynamic.csv`) and the
   2500–2599 extension point. This one tier backs most typed `Peds`/`Vehicles`/`Objects`/`World`/`Camera`
   methods, and samp's own `main.scm` needs it anyway.
3. **Peds/players and the context switch** (PED, AI): `CPlayerPed::ProcessControl` 0x60EA90,
   `CPhysical::ProcessCollision` 0x54DFB0, `CTaskSimpleUseGun::SetPedPosition` 0x624ED0,
   `CPlayerInfo::Process` 0x56F8D0, `CPlayerPed::SetInitialState` 0x60CD20, the weapon functions
   (0x5E6080, 0x5E6280, 0x5E6320, 0x5E6370, 0x5E6580), bones (0x5E4280, 0x5E01C0), `CPed::Render`
   0x5E7680, `CCivilianPed::ProcessControl` 0x5DDBF0, `ProcessAnimGroups` 0x6098F0, the anim manager
   (0x4D3A60, 0x4D3AA0, 0x4D41C0, 0x4D4610), `IKChainManager_c::LookAt` 0x618970, the enter/leave task
   ctors (0x6402F0, 0x63B8C0), `FlushImmediately` 0x601640, `SetTaskDuckSecondary` 0x601230.
4. **Combat**: `CWeapon::Fire` 0x742300, `FireInstantHit` 0x73FB10, `FireSniper` 0x73AAC0,
   `CBulletInfo::AddBullet/Update` 0x736010/0x7360D0, `CProjectileInfo::Update` 0x738B20 and
   `RemoveDetonatorProjectiles` 0x738860, `ComputeDamageResponse` 0x4B5AC0, `CEventDamage::AffectsPed`
   0x4B35A0, `CVehicle::InflictDamage` 0x6D7C90, `CWorld::ProcessLineOfSight` 0x56BA00,
   `CWeaponInfo::GetWeaponInfo` 0x743C60, `CCamera::CamShakeNoPos` 0x50A970, `CHud::DrawCrossHairs` 0x58E020.
5. **Vehicles**: the 10 `ProcessControl`s (0x6B1880, 0x6B9250, 0x6BFA30, 0x6C7050, 0x6C8250, 0x6C9260,
   0x6CDCC0, 0x6CED20, 0x6F1770, 0x6F86A0), the 4 `ProcessEntityCollision`s (0x546D00 implemented,
   0x6ACE70, 0x6BDEA0, 0x6C8AE0), `BreakTowLink` 0x6A4400, `UsesSiren` 0x6D8470,
   `ProcessSirenAndHorn` 0x6E0950, `CVehicle::Render` 0x6D0E60, the vehicle audio entity (0x4F7670,
   0x5002C0, 0x501AB0, 0x501E10, 0x502280), damage/doors/windows/gear (0x6A3440, 0x6A6AE0, 0x6B3E90,
   0x6D3080, 0x6D30B0, 0x6CAC20, 0x6CAC70), plate creation 0x6FDEA0.
6. **World entities and rendering hooks**: `CObject::Render/CreateRwObject` 0x59F180/0x59F110,
   `CEntity::DeleteRwObject/CreateRwObject/RenderEffects/UpdateRwFrame/UpdateRpHAnim/GetBoundCentre`,
   `CRenderer::AddEntityToRenderList/RenderEverythingBarRoads/RenderFadingInEntities/RenderOneNonRoad`,
   `CWorld::Process/ProcessAttachedEntities/ProcessVerticalLine/CameraToIgnoreThisObject/
   ProcessPedsAfterPreRender`, `CPhysical::PositionAttachedEntity`, `CFileLoader::LoadObjectInstance`
   0x538090, `CPickup::Update` 0x457410, `BreakObject_c::Init`, camera collision (0x41A990, 0x41AB20,
   0x41AC40, 0x41B000), escalators (0x717D30, 0x718580), `CShopping::LoadShop`.
7. **HUD/2D/radar/menus**: `CFont` (14 setters + `PrintString`), `CSprite2d` (0x727270, 0x728350,
   0x7281E0), `CTxdStore` (0x7316A0…0x731C80, `LoadTxd` implemented), `CMessages` (0x69DE90, 0x69E160,
   0x69F2B0), `CText::Get` 0x6A0050, `CHud::Draw/DrawRadar` 0x58FAE0/0x58A330, `CRadar::DrawMap/
   DrawRadarMap/DrawAreaOnRadar/GetRadarTraceColour/DrawRadarGangOverlay`, `CMenuManager::PrintMap`.
8. **Input and audio**: `CPad::UpdatePads/ClearMouseHistory`, `CAudioEngine::StartRadio/StopRadio`,
   `CAEAmbienceTrackManager` (0x4D6D50/0x4D6D60), `CAEPedSpeechAudioEntity::Initialise`,
   `CAEPoliceScannerAudioEntity::AddAudioEvent` 0x4E71E0.
9. **The rest of `samp-priority.csv`** (4,777 non-CRT rows) by subsystem, largest first: RW 820, AI 761,
   RENDER 492, AUDIO 452, UNK-game 417, WORLD 338, VEH 288, PED 186, HUD 154, STREAM 150, ANIM 127,
   POP 123, SCRIPT 103. Plus the closure misses of dynamic.md §3.1 (RW plugin callbacks, model-info vtable
   methods, CdStream thread).

The full direct list with states is reproducible with `docs/samp/samp_usage.py` + `samp-priority.csv`
(join on `function`).

## 9. Coverage of the inventory

| samp-rs rows (811) | where they go |
|---|---|
| 104 scm-opcode (103 used) | `ScriptCommands::run_command` + typed methods (§4); +7 opcodes found by SAMP-1D |
| 96 call | typed methods (§4) |
| 157 hook (incl. continuation/return-address rows) | `GameHooks` (§5); continuation and slot addresses stay private to the original backend |
| 99 global read/write/access | typed reads/writes (§4) or `PlayerContext` |
| 62 struct-field | snapshots (`PedSnapshot`, `VehicleSnapshot`, `CameraState`, …) |
| 17 code-compare | `VehicleClass` and typed call sites (`FireSite`, `LosSite`, `ContextSite`, enter/leave callers) |
| 13 patch-operand, 194 patch-modify, 55 patch-disable | `SampModeConfig` (`Limits`, `Behaviour`, `FeatureSet`, `StartupOptions`) and `RuntimeToggle` (§6) |

Deliberately not in the API: crash handler and SEH frames, the auth code read, the exe version probe,
the CdStream wake-up fix, D3D device internals (kept behind `RenderApi::Direct3D9`), BASS.

## 10. Open questions (for the user and the samp-rs agent)

**Answered by the user (2026-11-11):** Q1 git dependency pinned to a commit; Q2 sa-engine writes the original backend in samp-rs on a separate branch; Q7 Rust decoder for audio streams (no BASS). The rest are decided by the implementer with the defaults in this section unless they affect samp-rs behaviour against the original exe.

* **Q1 — how samp-rs gets the crate.** Git dependency on the sa-engine repository (pinned commit), a path
  dependency in a shared checkout, or a vendored copy in samp-rs kept identical by a check? The crate has
  no dependencies so any of these works; vendoring keeps samp-rs buildable alone.
* **Q2 — who owns the original backend.** The plan puts it in samp-rs (`adapter/original`), moved out of
  today's modules by the samp-rs owner. Alternatively sa-engine writes it on a samp-rs branch (allowed by
  the 2026-11-10 rule, but it touches most samp-rs modules). Which agent does steps 2–10?
* **Q3 — auth challenge.** samp's auth handler reads 20 bytes of game code at 0x4D3AA0 (an indirect-jmp
  hook site). On the native backend there is no such code. Does samp-rs compute the answer from a stored
  copy of those bytes (they are fixed for 1.0 US + samp's patch), and is that acceptable for open.mp's
  checks?
* **Q4 — parity target for the native UI.** The D3D call-stream diff cannot apply to the wgpu overlay.
  Is a screenshot diff of chat/dialogs/scoreboard against the original enough for the native renderer?
* **Q5 — frame limiter on native.** samp's limiter writes the time step and sleeps with `Sleep`; samp-rs
  found that the sleep granularity changes vehicle physics under Wine. Natively, should the engine own the
  limiter (with samp's numbers as config) or keep samp-rs's `frametimer.rs` driving it through a
  time-step setter?
* **Q6 — handles across backends.** The API passes handles as opaque `u32`. samp-rs stores raw pointers in
  several tables today (remote slots, vehicle pool, objects). Is it fine to keep `u32` (works for i686
  original and a native generational index), or does samp-rs prefer `u64` for a future 64-bit native
  build (wasm is 32-bit anyway)?
* **Q7 — audio streams on Linux.** BASS has a Linux build, but samp-rs imports `bass.dll` statically.
  Use the Linux BASS, or a Rust decoder (symphonia) for the native build?
* **Q8 — join serial (gpci).** `netclient.rs` uses `gta_user_dir_path + 4` (0xC9236C) as serial input.
  What must the native backend return there so the serial stays stable for the same user?
* **Q9 — the 7 opcodes and 42 unexecuted ones.** SAMP-1D executed 71 of samp-rs's 113 opcodes and 7 not
  in its list. The remaining 42 belong to features nobody played (trailers, mod shop, parachute, jetpack,
  object editing). Should the samp-rs agent run those scenarios against the original so their opcodes and
  hooks enter the port order?
* **Q10 — `PlayerContext` scope.** samp swaps 11 weapon skill stats and the camera vectors; does samp-rs
  also need the mouse state or `CWorld::Players[slot]` contents swapped in some path not yet measured
  (passenger drive-by, jetpack)?
* **Q11 — phase A dispatch.** In phase A, do we want samp-rs's original backend to keep applying its byte
  hooks for unported functions (simplest, no sa-hook change), or should sa-hook own every samp hook site
  from the start so ports and originals share one dispatcher (cleaner, more work in sa-hook)?
* **Q12 — vehicle palette and the 707-site model table.** The 256-colour palette is data in samp.dll's
  `.data`. samp-rs already ships it (`vehpatches.rs`); confirm it may be passed through
  `SampModeConfig::vehicle_colours` to the engine (it is interop data, not Rockstar data).
