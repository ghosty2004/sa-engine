# PORT-STREAM: subsystem STREAM (crate `sa-stream`)

Every row of `catalog/state.csv` with `subsystem == STREAM` (552 rows: CdStream, CStreaming,
CStreamingInfo, CFileLoader, CIplStore, CTxdStore, CColStore, every model info class and `CModelInfo`,
`CLoadedCarGroup`, ModelIndices, `CLoadingScreen`, the RW `Gta` stream readers, the `CPool<T>`
instantiations of the stores and the subsystem's static initialisers) has a decision below. Follows
ADR-001, `docs/arch/pilot-ports.md` and `docs/arch/core-port.md`. Agent states (the main session sets
`catalog/state.csv`):

| state | rows |
|---|---|
| L1 | 70 |
| implemented-needs-L2 | 434 |
| ported-unhookable | 40 |
| deferred | 6 |
| blocked | 2 |
| **total** | 552 |

`ported-unhookable` rows that also have an L1 spec: 26. L1 = `sa-oracle l1 <key> --cases 1000000`
with 0 mismatches (signalling-NaN cases decided by the CPU where floats are involved). Ported = L1 +
needs-L2 + unhookable.

| owner | rows | L1 | needs-L2 | unhookable | deferred | blocked |
|---|---|---|---|---|---|---|
| A | 138 | 9 | 121 | 5 | 3 | 0 |
| B | 167 | 41 | 102 | 24 | 0 | 0 |
| C | 129 | 11 | 113 | 2 | 2 | 1 |
| M | 118 | 9 | 98 | 9 | 1 | 1 |

Sources: the 1.0 US bytes (capstone over the exe with `catalog/extents.csv`, `.HOODLUM` bodies and
protection chunks followed), the Ghidra decompilation of all 552 rows (headless, read-only), the
port-ready specs and deviation notes in `docs/re/specs/STREAM/`, and gta-reversed @ 20d8d2a as reading
aid / code source where it matches the bytes. Where they differ the original is followed.

## Crate decision

A new crate `crates/sa-stream` depending on `sa-core` (not a module tree inside sa-core):
- PLAN §2 fixes the crate name `sa-stream`, and ADR-001 §7 describes exactly this case ("new engine
  crates depend on sa-core and use `#[hook]` with `sa_core` in scope; sa-hook appends their
  `hooks::ALL` the same way");
- STREAM (552 rows) is larger than all of CORE and only consumes CORE (sa-core's calls into STREAM stay
  `orig!` declarations), so the dependency is one-way;
- later engine crates (sa-world, sa-rw) can depend on streaming without pulling in more, and sa-core's
  incremental builds stay small.

Same rules as sa-core: `#![no_std]`, `#![deny(unsafe_code)]` (only `unsafe impl GameData` for POD
structs), game state through `sa_core::globals!`/`GPtr`, calls to other catalogued functions (including
RenderWare `Rp*`/`Rw*` and the CRT) through `sa_core::orig!` with the original address, ports registered
with `#[hook]` and listed in their module's `HOOKS`.

## Layout and integration

| file | contents |
|---|---|
| `lib.rs`, `hooks.rs` | module list; `hooks::MODULES`/`ALL`/`COUNT`, `hooks::GLOBALS`; tests (every `#[hook(` in `src/**` is listed, addresses unique and not hooked by sa-core, initial values of every global against the exe image, no global of sa-stream overlaps another one of sa-stream or one of sa-core's — exe sizes, 64-bit hosts handled) |
| `shared.rs` | cross-module layouts (`CStreamingInfo`, `CBaseModelInfo`, `TxdDef`, `ColDef`, `IplDef`), resource ids, shared globals (`ms_aInfoForModel`, `ms_pArrayBase`, the three store pools; `ms_modelInfoPtrs` re-exported from sa-core), `null_deref`/`nonnull` (see below), `image()` |
| `cdstream.rs`, `iplstore.rs`, `txdstore.rs`, `loaded_car_group.rs`, `loading_screen.rs`, `rw_gta_stream.rs`, `misc.rs` | owner M |
| `streaming.rs`, `streaming_info.rs`, `colstore.rs` | owner A |
| `modelinfo/{base,atomic,time,weapon,clump,ped,model_info,model_indices}.rs` | owner B |
| `fileloader.rs`, `modelinfo/vehicle.rs` | owner C |

Integration: `sa-hook` depends on `sa-stream` (feature `in-process`); its table is
`LOCAL ++ sa_core::hooks::ALL ++ sa_stream::hooks::ALL` (`table.rs` `N`/`build()`, test
`port_hooks_follow_the_local_ones` covers both crates). `sa-oracle` depends on `sa-stream`; the STREAM L1
specs are in `specs_stream_{a,b,c,m}.rs` (chained into `specs::all()`); `specs_core`'s pool spec builders
(`pool_new_spec`, `ctor_spec`, `op_new_spec`) are `pub(crate)` and reused for the store pools.
`lib.rs` raises `recursion_limit` (large `orig!` blocks expand one macro step per declaration).

Conventions worth knowing for L2:
- **Null dereferences.** Many store accessors of the original are not null-checked (a free pool slot
  gives an access violation, gta-reversed returns quietly). The ports do not read through null; they stop
  at those points with `shared::null_deref(site)` (a panic naming the site). The game ends either way.
- **Protection chunks.** Several entries and bodies go through no-CD protection code: import thunk
  0x1141980 (`call [[cellptr] ^ 0x214D4C48]`, decoded per site), `stc; sbb`/`xor`/`not`/`rol`
  address computations (decoded statically and, where the original computes them at run time, computed
  the same way in phase A, e.g. `txdstore::stored_txd_cell`), and the protection VM at 0xCB73E0
  (an expression-tree interpreter, not decoded: see the blocked/open rows).
- **Entries that are `jmp rel32` into `.text` chunks** (not `.HOODLUM` stubs) carry a 6-byte `expect`, so
  sa-hook treats them as inline hooks (e.g. `CIplStore::LoadIpls` 0x405170).
- **Callbacks passed by address** (RW and quad-tree callbacks, thread entry) are passed as the original
  addresses, so the original code calls the (hooked) original entry.
- **Cross-module calls** go through `orig!`. 41 cross-owner targets whose Rust signature matches exactly
  were wired to the Rust port (`=> crate::module::fn`); the rest stay `=> unported` (pointer-type or
  bool/u8 signature differences need small adapters for phase B; listed per owner in the open points).
- **Threads.** `CdStreamThread` runs on the streaming thread and uses `Global`/`GPtr` there, as the
  original does: an exception to ADR-001's single-thread rule, limited to the CdStream globals and stream
  array. Its hook applies when the thread starts (the function never returns).
- **CRT initialisers** (`__init_*`) run before sa-hook attaches: their L2 needs a record from process
  start (or an L1 spec, as for `__init_g_bab358`).

## Divergence (ledger)

- `CdStreamSync` (0x406460): the port does not reproduce the original's lost wake-up (INF-15, ledger
  "original UB, not reproducible"). The `INFINITE` wait is a 20 ms polled wait that ends once the
  streaming thread is done with the stream (`nSectorsToRead == 0 && bInUse == 0`), consuming a release
  that raced with the check — the same logic as sa-hook's `cdsync.rs`. Everything observable of a
  completed read is unchanged.
- Not reproduced (stdout only): `CdStreamRead`'s `printf` on a failed `ReleaseSemaphore` (format string
  computed by the protection VM).
- Original UB that panics in the port instead of corrupting the stack: struct chunks longer than the
  original's stack buffers in the RW `Gta` readers and the atomic reader 0x72E270, IPL names longer than
  SetupRelatedIpls' frame; walks past `CLoadedCarGroup::PickRandomCar`'s local list (negative
  frequencies) read -1. Owner B/C ledger candidates are listed in their sections below.

## Functions (all 552 STREAM rows)


### Owner A: CStreaming, CStreamingInfo, CLinkList thunk, CColStore (`streaming.rs`, `streaming_info.rs`, `colstore.rs`; specs `specs_stream_a.rs`)

| address | name | conv | port (sa-stream) | source | deviations followed (original kept); *new* = not in gr-deviations.csv | state | notes (L2 signature / reason) |
|---|---|---|---|---|---|---|---|
| 0x407460 | CStreamingInfo::Init | thiscall | streaming_info::init | bytes + Ghidra + gr StreamingInfo.cpp:7 | new: flags (+6) not cleared (gr `*this = {}`) | L1 (1002000 cases, 0 mismatches) | L1 key CStreamingInfoInit; thiscall, 0 stack words, Void |
| 0x407480 | CStreamingInfo::AddToList | thiscall | streaming_info::add_to_list | bytes + Ghidra + gr StreamingInfo.cpp:8 | StreamingInfo.cpp:59 harmless | L1 (1002000 cases, 0 mismatches) | L1 key CStreamingInfoAddToList; thiscall, 1 stack words, Void |
| 0x4074e0 | CStreamingInfo::RemoveFromList | thiscall | streaming_info::remove_from_list | bytes + Ghidra + gr StreamingInfo.cpp:13 | StreamingInfo.cpp:70 harmless | L1 (1002000 cases, 0 mismatches) | L1 key CStreamingInfoRemoveFromList; thiscall, 0 stack words, Void |
| 0x407560 | CStreamingInfo::InList | thiscall | streaming_info::in_list | bytes + Ghidra + gr StreamingInfo.cpp:12 | StreamingInfo.cpp:48 (gr also tests prev); EAX 0/1 | L1 (1002000 cases, 0 mismatches) | L1 key CStreamingInfoInList; thiscall, 0 stack words, EAX |
| 0x407570 | CStreamingInfo::GetCdPosn | thiscall | streaming_info::get_cd_posn | bytes + Ghidra + gr StreamingInfo.cpp:9 |  | L1 (1002000 cases, 0 mismatches) | L1 key CStreamingInfoGetCdPosn; thiscall, 0 stack words, EAX |
| 0x4075a0 | CStreamingInfo::GetCdPosnAndSize | thiscall | streaming_info::get_cd_posn_and_size | bytes + Ghidra + gr StreamingInfo.cpp:11 |  | L1 (1002000 cases, 0 mismatches) | L1 key CStreamingInfoGetCdPosnAndSize; thiscall, 2 stack words, AL |
| 0x4075e0 | CStreamingInfo::SetCdPosnAndSize | thiscall | streaming_info::set_cd_posn_and_size | bytes + Ghidra + gr StreamingInfo.cpp:10 |  | L1 (1002000 cases, 0 mismatches) | L1 key CStreamingInfoSetCdPosnAndSize; thiscall, 2 stack words, Void |
| 0x4076a0 | CStreaming::IsVeryBusy | cdecl | streaming::is_very_busy | bytes + Ghidra + gr Streaming.cpp:40 |  | implemented-needs-L2 | cdecl, 0 stack words, EAX |
| 0x407800 | CStreaming::IsObjectInCdImage | cdecl | streaming::is_object_in_cd_image | bytes + Ghidra + spec CStreaming.md |  | implemented-needs-L2 | cdecl, 1 stack words, AL |
| 0x407a10 | CStreaming::ForceLayerToRead | cdecl | streaming::force_layer_to_read | bytes + Ghidra + spec CStreaming.md |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x407a30 | CStreaming::SetLoadVehiclesInLoadScene | cdecl | streaming::set_load_vehicles_in_load_scene | bytes + Ghidra + spec CStreaming.md |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x407a40 | CStreaming::ClearFlagForAll | cdecl | streaming::clear_flag_for_all | bytes + Ghidra + gr Streaming.cpp:25 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x407ac0 | CStreaming::RemoveUnusedModelsInLoadedList | cdecl | streaming::remove_unused_models_in_loaded_list | bytes + Ghidra + spec CStreaming.md |  | ported-unhookable | lone `ret` (relocate TooShort) |
| 0x407ad0 | CStreaming::AreAnimsUsedByRequestedModels | cdecl | streaming::are_anims_used_by_requested_models | bytes + Ghidra + gr Streaming.cpp:23 | protection cell [0x15606A0]-[0x646FE2] = 0x8E4C58 decoded statically; expect 6 bytes (jmp entry, not a stub) | implemented-needs-L2 | cdecl, 1 stack words, AL |
| 0x407be0 | CStreaming::ImGonnaUseStreamingMemory | cdecl | streaming::im_gonna_use_streaming_memory | bytes + Ghidra + spec CStreaming.md |  | ported-unhookable | lone `ret` |
| 0x407bf0 | CStreaming::IHaveUsedStreamingMemory | cdecl | streaming::i_have_used_streaming_memory | bytes + Ghidra + spec CStreaming.md |  | ported-unhookable | lone `ret` |
| 0x407c00 | CStreaming::GetDefaultCopModel | cdecl | streaming::get_default_cop_model | bytes + Ghidra + gr Streaming.cpp:93 |  | implemented-needs-L2 | cdecl, 0 stack words, EAX |
| 0x407c50 | CStreaming::GetDefaultCopCarModel | cdecl | streaming::get_default_cop_car_model | bytes + Ghidra + gr Streaming.cpp:92 |  | implemented-needs-L2 | cdecl, 1 stack words, EAX |
| 0x407d10 | CStreaming::DisableCopBikes | cdecl | streaming::disable_cop_bikes | bytes + Ghidra + gr Streaming.cpp:39 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x407d20 | CStreaming::GetDefaultMedicModel | cdecl | streaming::get_default_medic_model | bytes + Ghidra + gr Streaming.cpp:91 |  | implemented-needs-L2 | cdecl, 0 stack words, EAX |
| 0x407d40 | CStreaming::GetDefaultFiremanModel | cdecl | streaming::get_default_fireman_model | bytes + Ghidra + gr Streaming.cpp:90 |  | implemented-needs-L2 | cdecl, 0 stack words, EAX |
| 0x407d50 | CStreaming::GetDefaultCabDriverModel | cdecl | streaming::get_default_cab_driver_model | bytes + Ghidra + gr Streaming.cpp:89 + spec CStreaming.deviations.md | Streaming.cpp:2729 keep-original (original random formula, rand site 0x407D7E) | implemented-needs-L2 | cdecl, 0 stack words, EAX |
| 0x407f00 | CStreaming::HasSpecialCharLoaded | cdecl | streaming::has_special_char_loaded | bytes + Ghidra + gr Streaming.cpp:28 |  | implemented-needs-L2 | cdecl, 1 stack words, AL |
| 0x407f20 | CStreaming::LoadInitialVehicles | cdecl | streaming::load_initial_vehicles | bytes + Ghidra + spec CStreaming.md |  | ported-unhookable | lone `ret` |
| 0x407f80 | CStreaming::WeAreTryingToPhaseVehicleOut | cdecl | streaming::we_are_trying_to_phase_vehicle_out | bytes + Ghidra + gr Streaming.cpp:112 |  | implemented-needs-L2 | cdecl, 1 stack words, EAX |
| 0x407fb0 | CStreaming::FindMIPedSlotForInterior | cdecl | streaming::find_mi_ped_slot_for_interior | bytes + Ghidra + gr Streaming.cpp:59 | returns 7 when no slot holds a loaded ped (original constant) | implemented-needs-L2 | cdecl, 1 stack words, EAX |
| 0x408230 | thunk_FUN_01563580 | thiscall | streaming::link_list_insert | bytes + Ghidra + spec CLinkList.md | CLinkList<CEntity*>::Insert (spec CLinkList.md) | implemented-needs-L2 | thiscall, 1 stack words, EAX |
| 0x4084f0 | CStreaming::InstanceLoadedModels | cdecl | streaming::instance_loaded_models | bytes + Ghidra + gr Streaming.cpp:96 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x4090a0 | CStreaming::DeleteAllRwObjects | cdecl | streaming::delete_all_rw_objects | bytes + Ghidra + gr Streaming.cpp:31 | Streaming.cpp:673 harmless | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x409210 | CStreaming::DeleteRwObjectsAfterDeath | cdecl | streaming::delete_rw_objects_after_death | bytes + Ghidra + gr Streaming.cpp:33 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x4094b0 | CStreaming::RemoveBuildingsNotInArea | cdecl | streaming::remove_buildings_not_in_area | bytes + Ghidra + gr Streaming.cpp:72 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x4096c0 | CStreaming::StartRenderEntities | cdecl | streaming::start_render_entities | bytes + Ghidra + spec CStreaming.md |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x4096d0 | CStreaming::RenderEntity | cdecl | streaming::render_entity | bytes + Ghidra + gr Streaming.cpp:52 | protection cell decodes to 0x8E48A0 | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x409710 | CStreaming::RemoveEntity | cdecl | streaming::remove_entity | bytes + Ghidra + gr Streaming.cpp:76 | protection cell decodes to 0x8E48A0 | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40ba70 | CStreaming::PossiblyStreamCarOutAfterCreation | cdecl | streaming::possibly_stream_car_out_after_creation | bytes + Ghidra + gr Streaming.cpp:51 | computed call [0x156057C]-[0x6822B7]-1 = SetModelIsDeletable; rand site 0x156C5C7 | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x410390 | FUN_00410390 | cdecl | colstore::find_col_slot | bytes + Ghidra + spec CColStore.md |  | ported-unhookable | `or eax,-1; ret` (4 bytes) |
| 0x4103a0 | CColStore::AddCollisionNeededAtPosn | cdecl | colstore::add_collision_needed_at_posn | bytes + Ghidra + gr Collision/ColStore.cpp:434 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x4103d0 | CColStore::SetIfCollisionIsRequired | cdecl | colstore::set_if_collision_is_required | bytes + Ghidra + gr Collision/ColStore.cpp:454 |  | L1 (1060000 cases, 0 mismatches) | L1 key CColStoreSetIfCollisionIsRequired; cdecl, 2 stack words, Void |
| 0x410470 | CColStore::SetIfCollisionIsRequiredReducedBB | cdecl | colstore::set_if_collision_is_required_reduced_bb | bytes + Ghidra + gr Collision/ColStore.cpp:455 |  | L1 (1060000 cases, 0 mismatches) | L1 key CColStoreSetIfCollisionIsRequiredReducedBB; cdecl, 2 stack words, Void |
| 0x4107a0 | CColStore::AddRef | cdecl | colstore::add_ref | bytes + Ghidra + gr Collision/ColStore.cpp:436 + spec CColStore.deviations.md | ColStore.cpp:112 keep-original (no null check) | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x4107d0 | CColStore::RemoveRef | cdecl | colstore::remove_ref | bytes + Ghidra + gr Collision/ColStore.cpp:437 + spec CColStore.deviations.md | ColStore.cpp:388 keep-original (no null check) | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x410800 | CColStore::GetBoundingBox | cdecl | colstore::get_bounding_box | bytes + Ghidra + gr Collision/ColStore.cpp:450 |  | implemented-needs-L2 | cdecl, 1 stack words, EAX |
| 0x410820 | CColStore::IncludeModelIndex | cdecl | colstore::include_model_index | bytes + Ghidra + gr Collision/ColStore.cpp:445 |  | implemented-needs-L2 | cdecl, 2 stack words, Void |
| 0x407100 | CStreaming::RequestTxdModel | cdecl | streaming::request_txd_model | bytes + Ghidra + gr Streaming.cpp:57 |  | implemented-needs-L2 | cdecl, 2 stack words, Void |
| 0x407610 | CStreaming::AddImageToList | cdecl | streaming::add_image_to_list | bytes + Ghidra + gr Streaming.cpp:20 |  | implemented-needs-L2 | cdecl, 2 stack words, EAX |
| 0x407820 | CStreaming::HasVehicleUpgradeLoaded | cdecl | streaming::has_vehicle_upgrade_loaded | bytes + Ghidra + gr Streaming.cpp:29 | Streaming.cpp:421 harmless | implemented-needs-L2 | cdecl, 1 stack words, AL |
| 0x407dd0 | CStreaming::IsCarModelNeededInCurrentZone | cdecl | streaming::is_car_model_needed_in_current_zone | bytes + Ghidra + gr Streaming.cpp:97 | Streaming.cpp:2965; gang loop index through 0xCB73E0 (= 18 by consistency) | implemented-needs-L2 | cdecl, 1 stack words, AL |
| 0x4083c0 | CStreaming::InitImageList | cdecl | streaming::init_image_list | bytes + Ghidra + gr Streaming.cpp:95 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x4087e0 | CStreaming::RequestModel | cdecl | streaming::request_model | bytes + Ghidra + gr Streaming.cpp:56 | TXD request gets the full flags dword; unknown states take state 2 | implemented-needs-L2 | cdecl, 2 stack words, Void |
| 0x409650 | CStreaming::AddEntity | cdecl | streaming::add_entity | bytes + Ghidra + gr Streaming.cpp:19 |  | implemented-needs-L2 | cdecl, 1 stack words, EAX |
| 0x409a90 | CStreaming::AreTexturesUsedByRequestedModels | cdecl | streaming::are_textures_used_by_requested_models | bytes + Ghidra + gr Streaming.cpp:24 |  | implemented-needs-L2 | cdecl, 1 stack words, AL |
| 0x411140 | CColStore::AddColSlot | cdecl | colstore::add_col_slot | bytes + Ghidra + gr Collision/ColStore.cpp:435 | new: name not copied into ColDef (gr strcpy_s); "stadint" length via 0xCB73E0 (7) | implemented-needs-L2 | cdecl, 1 stack words, EAX |
| 0x408000 | CStreaming::AddToLoadedVehiclesList | cdecl | streaming::add_to_loaded_vehicles_list | bytes + Ghidra + gr Streaming.cpp:88 | inappropriate-cars group through 0xCB73E0 (= 0xC0E9C8, gr); gang groups 0..9 (original indexing) | implemented-needs-L2 | cdecl, 1 stack words, AL |
| 0x4084b0 | CStreaming::Shutdown | cdecl | streaming::shutdown | bytes + Ghidra + gr Streaming.cpp:100 + spec CStreaming.deviations.md | Streaming.cpp:3061/3069 keep-original: pointers left dangling | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x408c70 | CStreaming::RequestVehicleUpgrade | cdecl | streaming::request_vehicle_upgrade | bytes + Ghidra + gr Streaming.cpp:58 |  | implemented-needs-L2 | cdecl, 2 stack words, Void |
| 0x408e20 | CStreaming::GetNextFileOnCd | cdecl | streaming::get_next_file_on_cd | bytes + Ghidra + gr Streaming.cpp:27 + spec CStreaming.deviations.md | Streaming.cpp:386 keep-original (full 32-bit position, unsigned); retry without priority filter (tail call) | implemented-needs-L2 | cdecl, 2 stack words, EAX |
| 0x409430 | CStreaming::RequestBigBuildings | cdecl | streaming::request_big_buildings | bytes + Ghidra + gr Streaming.cpp:53 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40a120 | CStreaming::LoadInitialWeapons | cdecl | streaming::load_initial_weapons | bytes + Ghidra + gr Streaming.cpp:47 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40afa0 | CStreaming::ReclassifyLoadedCars | cdecl | streaming::reclassify_loaded_cars | bytes + Ghidra + gr Streaming.cpp:69 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40c270 | CStreaming::ProcessEntitiesInSectorList<CPtrListSingleLink<CEntity*>>- | cdecl | streaming::process_entities_in_sector_list_radius | bytes + Ghidra + gr Streaming.cpp:84 |  | implemented-needs-L2 | cdecl, 9 stack words, Void |
| 0x40c450 | CStreaming::ProcessEntitiesInSectorList | cdecl | streaming::process_entities_in_sector_list | bytes + Ghidra + spec CStreaming.md |  | implemented-needs-L2 | cdecl, 2 stack words, Void |
| 0x4104e0 | CColStore::SetCollisionRequired | cdecl | colstore::set_collision_required | bytes + Ghidra + gr Collision/ColStore.cpp:439 | protection cell [0x55E5C9]^[0x156058C] = CGame::currArea | implemented-needs-L2 | cdecl, 2 stack words, Void |
| 0x4113f0 | CColStore::Initialise | cdecl | colstore::initialise | bytes + Ghidra + gr Collision/ColStore.cpp:432 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x5bccd0 | CStreaming::ReadIniFile | cdecl | streaming::read_ini_file | bytes + Ghidra + gr Streaming.cpp:68 + spec CStreaming.deviations.md | Streaming.cpp:2031 keep-original (atol/atof, pe_* int32) | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40b4b0 | CStreaming::LoadZoneVehicle | cdecl | streaming::load_zone_vehicle | bytes + Ghidra + gr Streaming.cpp:50 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40b4f0 | CStreaming::StreamOneNewCar | cdecl | streaming::stream_one_new_car | bytes + Ghidra + gr Streaming.cpp:104 | new: first active cheat wins (gr: last); new: after a taxi the cabbie is picked when needed (gr never); computed calls decoded | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40c520 | CStreaming::AddLodsToRequestList | cdecl | streaming::add_lods_to_request_list | bytes + Ghidra + gr Streaming.cpp:21 |  | implemented-needs-L2 | cdecl, 2 stack words, Void |
| 0x40d3f0 | CStreaming::AddModelsToRequestList | — | — | bytes + Ghidra + gr Streaming.cpp:22 |  | deferred | deferred: 297 instructions of camera/frustum x87 maths over sector rings (entry chunk 0x402313); needs a byte-level pass with CPU-checked x87. Called through orig! (original) |
| 0x410690 | CColStore::LoadCol- | cdecl | colstore::load_col_file | bytes + Ghidra + gr Collision/ColStore.cpp:443 | ColStore.cpp:262 harmless | implemented-needs-L2 | cdecl, 2 stack words, Void |
| 0x4106d0 | CColStore::LoadCol-2 | cdecl | colstore::load_col_buffer | bytes + Ghidra + gr Collision/ColStore.cpp:444 |  | implemented-needs-L2 | cdecl, 3 stack words, AL |
| 0x410c00 | CColStore::RequestCollision | cdecl | colstore::request_collision | bytes + Ghidra + gr Collision/ColStore.cpp:440 |  | implemented-needs-L2 | cdecl, 2 stack words, Void |
| 0x410ce0 | CColStore::HasCollisionLoaded | cdecl | colstore::has_collision_loaded | bytes + Ghidra + gr Collision/ColStore.cpp:451 |  | implemented-needs-L2 | cdecl, 2 stack words, AL |
| 0x410ec0 | CColStore::BoundingBoxesPostProcess | cdecl | colstore::bounding_boxes_post_process | bytes + Ghidra + gr Collision/ColStore.cpp:449 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x5b6170 | CStreaming::LoadCdDirectory-char | cdecl | streaming::load_cd_directory_file | bytes + Ghidra + gr Streaming.cpp:44 + spec CStreaming.deviations.md | Streaming.cpp:1111/1150/1162/1197 (FindColSlot = -1 → always AddColSlot) | implemented-needs-L2 | cdecl, 2 stack words, Void |
| 0x5b82c0 | CStreaming::LoadCdDirectory-void | cdecl | streaming::load_cd_directory | bytes + Ghidra + gr Streaming.cpp:45 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x5d29a0 | CStreaming::Save | cdecl | streaming::save | bytes + Ghidra + gr Streaming.cpp:42 |  | implemented-needs-L2 | cdecl, 0 stack words, AL |
| 0x407a70 | CStreaming::DeleteRwObjectsInSectorList<CPtrListSingleLink<CEntity*>> | cdecl | streaming::delete_rw_objects_in_sector_list | bytes + Ghidra + gr Streaming.cpp:36 |  | implemented-needs-L2 | cdecl, 3 stack words, Void |
| 0x410730 | CColStore::RemoveCol | cdecl | colstore::remove_col | bytes + Ghidra + gr Collision/ColStore.cpp:442 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x5b8ad0 | CStreaming::Init2 | cdecl | streaming::init2 | bytes + Ghidra + gr Streaming.cpp:94 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x5d29e0 | CStreaming::Load | cdecl | streaming::load | bytes + Ghidra + gr Streaming.cpp:41 |  | implemented-needs-L2 | cdecl, 0 stack words, AL |
| 0x411330 | CColStore::RemoveColSlot | cdecl | colstore::remove_col_slot | bytes + Ghidra + gr Collision/ColStore.cpp:441 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x4114d0 | CColStore::Shutdown | cdecl | colstore::shutdown | bytes + Ghidra + gr Collision/ColStore.cpp:433 + spec CColStore.deviations.md | ColStore.cpp:35 harmless (fixed 255-slot ascending loop) | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x4080f0 | CStreaming::RemoveCarModel | cdecl | streaming::remove_car_model | bytes + Ghidra + gr Streaming.cpp:73 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x4089a0 | CStreaming::RemoveModel | cdecl | streaming::remove_model | bytes + Ghidra + gr Streaming.cpp:81 | RRR: nothing on unload (original) | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x409c10 | CStreaming::SetModelIsDeletable | cdecl | streaming::set_model_is_deletable | bytes + Ghidra + gr Streaming.cpp:99 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x409c70 | CStreaming::SetModelTxdIsDeletable | cdecl | streaming::set_model_txd_is_deletable | bytes + Ghidra + spec CStreaming.md |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x409050 | CStreaming::RequestFilesInChannel | cdecl | streaming::request_files_in_channel | bytes + Ghidra + gr Streaming.cpp:55 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x4093b0 | CStreaming::RemoveBigBuildings | cdecl | streaming::remove_big_buildings | bytes + Ghidra + gr Streaming.cpp:71 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x409760 | CStreaming::DeleteLeastUsedEntityRwObject | cdecl | streaming::delete_least_used_entity_rw_object | bytes + Ghidra + gr Streaming.cpp:32 |  | implemented-needs-L2 | cdecl, 2 stack words, AL |
| 0x409940 | CStreaming::DeleteRwObjectsBehindCameraInSectorList<CPtrListSingleLink<CEntity*>> | cdecl | streaming::delete_rw_objects_behind_camera_in_sector_list | bytes + Ghidra + gr Streaming.cpp:35 |  | implemented-needs-L2 | cdecl, 2 stack words, AL |
| 0x4099e0 | CStreaming::DeleteRwObjectsNotInFrustumInSectorList<CPtrListSingleLink<CEntity*>> | cdecl | streaming::delete_rw_objects_not_in_frustum_in_sector_list | bytes + Ghidra + gr Streaming.cpp:37 |  | implemented-needs-L2 | cdecl, 2 stack words, AL |
| 0x409c90 | CStreaming::SetMissionDoesntRequireModel | cdecl | streaming::set_mission_doesnt_require_model | bytes + Ghidra + gr Streaming.cpp:98 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40a080 | CStreaming::RequestFile | cdecl | streaming::request_file | bytes + Ghidra + gr Streaming.cpp:54 | computed call [0x1560510]^[0x540123] = RequestModel | implemented-needs-L2 | cdecl, 5 stack words, Void |
| 0x40a150 | CStreaming::StreamCopModels | cdecl | streaming::stream_cop_models | bytes + Ghidra + gr Streaming.cpp:102 + spec CStreaming.deviations.md | Streaming.cpp:3120 keep-original; wanted/bike part through 0xCB73E0 (gr semantics, see open questions) | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40a2a0 | CStreaming::StreamAmbulanceAndMedic | cdecl | streaming::stream_ambulance_and_medic | bytes + Ghidra + gr Streaming.cpp:101 |  | implemented-needs-L2 | cdecl, 1 stack words, AL |
| 0x40a400 | CStreaming::StreamFireEngineAndFireman | cdecl | streaming::stream_fire_engine_and_fireman | bytes + Ghidra + gr Streaming.cpp:103 | request part through 0xCB73E0 (gr semantics) | implemented-needs-L2 | cdecl, 1 stack words, AL |
| 0x40a560 | CStreaming::StreamZoneModels | — | — | bytes + Ghidra + gr Streaming.cpp:109 |  | deferred | deferred: 347 instructions; entry chunk 0x40145A computes a pointer with `ror [0x1560638], [0x727B61]` |
| 0x40aa10 | CStreaming::StreamZoneModels_Gangs | cdecl | streaming::stream_zone_models_gangs | bytes + Ghidra + gr Streaming.cpp:110 | Streaming.cpp:3642/3647: original indexes car groups 0..9 by gang number; keep | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40b080 | CStreaming::RemoveCurrentZonesModels | cdecl | streaming::remove_current_zones_models | bytes + Ghidra + gr Streaming.cpp:74 | Streaming.cpp:2197 keep-original | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40b340 | CStreaming::RemoveLoadedZoneModel | cdecl | streaming::remove_loaded_zone_model | bytes + Ghidra + gr Streaming.cpp:80 |  | implemented-needs-L2 | cdecl, 0 stack words, AL |
| 0x40b3a0 | CStreaming::RemoveInappropriatePedModels | cdecl | streaming::remove_inappropriate_ped_models | bytes + Ghidra + gr Streaming.cpp:77 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40baa0 | CStreaming::ClearSlots | cdecl | streaming::clear_slots | bytes + Ghidra + gr Streaming.cpp:26 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40bda0 | CStreaming::StreamPedsIntoRandomSlots | cdecl | streaming::stream_peds_into_random_slots | bytes + Ghidra + gr Streaming.cpp:106 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40be60 | CStreaming::RemoveDodgyPedsFromRandomSlots | cdecl | streaming::remove_dodgy_peds_from_random_slots | bytes + Ghidra + gr Streaming.cpp:75 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40c020 | CStreaming::RemoveLoadedVehicle | cdecl | streaming::remove_loaded_vehicle | bytes + Ghidra + gr Streaming.cpp:79 |  | implemented-needs-L2 | cdecl, 0 stack words, AL |
| 0x40c180 | CStreaming::RemoveTxdModel | cdecl | streaming::remove_txd_model | bytes + Ghidra + gr Streaming.cpp:82 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40c1e0 | CStreaming::PurgeRequestList | cdecl | streaming::purge_request_list | bytes + Ghidra + gr Streaming.cpp:66 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40cba0 | CStreaming::RequestModelStream | cdecl | streaming::request_model_stream | bytes + Ghidra + gr Streaming.cpp:62 | position/size locals uninitialised in the original before the first entry with a size (port: 0) | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40d2f0 | CStreaming::RemoveReferencedTxds | cdecl | streaming::remove_referenced_txds | bytes + Ghidra + gr Streaming.cpp:38 | Streaming.cpp:978 harmless ; Streaming.cpp:983 harmless | implemented-needs-L2 | cdecl, 1 stack words, AL |
| 0x410860 | CColStore::LoadCollision | cdecl | colstore::load_collision | bytes + Ghidra + gr Collision/ColStore.cpp:452 | expect 6 bytes (jmp to chunk 0x401ECF reading ms_disableStreaming); CVector by value (3 words) + bool | implemented-needs-L2 | cdecl, 4 stack words, Void |
| 0x410e00 | CColStore::RemoveAllCollision | cdecl | colstore::remove_all_collision | bytes + Ghidra + gr Collision/ColStore.cpp:446 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40b490 | CStreaming::SetMissionDoesntRequireSpecialChar | cdecl | streaming::set_mission_doesnt_require_special_char | bytes + Ghidra + spec CStreaming.md |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40b650 | CStreaming::StreamVehiclesAndPeds_Always | cdecl | streaming::stream_vehicles_and_peds_always | bytes + Ghidra + gr Streaming.cpp:108 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40b700 | CStreaming::StreamVehiclesAndPeds | cdecl | streaming::stream_vehicles_and_peds | bytes + Ghidra + gr Streaming.cpp:107 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40bbb0 | CStreaming::StreamPedsForInterior | cdecl | streaming::stream_peds_for_interior | bytes + Ghidra + gr Streaming.cpp:105 + spec CStreaming.deviations.md | Streaming.cpp:3271/3306/3317 keep-original; rand sites 0x40BCD1/0x40BCDB/0x40BC7A/0x40BBEF | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40cf80 | CStreaming::RemoveAllUnusedModels | cdecl | streaming::remove_all_unused_models | bytes + Ghidra + gr Streaming.cpp:70 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40cfd0 | CStreaming::RemoveLeastUsedModel | cdecl | streaming::remove_least_used_model | bytes + Ghidra + gr Streaming.cpp:78 |  | implemented-needs-L2 | cdecl, 1 stack words, AL |
| 0x40d3d0 | CStreaming::LoadInitialPeds | cdecl | streaming::load_initial_peds | bytes + Ghidra + gr Streaming.cpp:46 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x48b570 | CStreaming::SetMissionDoesntRequireAnim | cdecl | streaming::set_mission_doesnt_require_anim | bytes + Ghidra + spec CStreaming.md |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40d7c0 | CStreaming::DeleteRwObjectsBehindCamera | — | — | bytes + Ghidra + gr Streaming.cpp:34 + spec CStreaming.deviations.md | Streaming.cpp:892 keep-original | deferred | deferred: 738 instructions, same kind (DeleteRwObjectsBehindCamera; gr-deviation Streaming.cpp:892 REFUTED, no scan-code advance before the last pass) |
| 0x408cb0 | CStreaming::FinishLoadingLargeFile | cdecl | streaming::finish_loading_large_file | bytes + Ghidra + gr Streaming.cpp:60 | non-DFF/TXD ids: result = id low byte (original) | implemented-needs-L2 | cdecl, 2 stack words, AL |
| 0x40c6b0 | CStreaming::ConvertBufferToObject | cdecl | streaming::convert_buffer_to_object | bytes + Ghidra + gr Streaming.cpp:30 + spec CStreaming.deviations.md | Streaming.cpp:447 keep-original (no null check of the TXD slot); 2 stack words (callers push a 3rd, unused) | implemented-needs-L2 | cdecl, 2 stack words, AL |
| 0x40e120 | CStreaming::MakeSpaceFor | cdecl | streaming::make_space_for | bytes + Ghidra + gr Streaming.cpp:83 | chunk 0x4037EB rol-decoded MEMORY_AVAILABLE; expect 6 bytes | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x4076c0 | CStreaming::RetryLoadFile | cdecl | streaming::retry_load_file | bytes + Ghidra + gr Streaming.cpp:85 + spec CStreaming.deviations.md | Streaming.cpp:2574 keep-original (StopPadsShaking first; original error-state logic) | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x40e170 | CStreaming::ProcessLoadingChannel | cdecl | streaming::process_loading_channel | bytes + Ghidra + gr Streaming.cpp:65 |  | implemented-needs-L2 | cdecl, 1 stack words, AL |
| 0x40e3a0 | CStreaming::LoadRequestedModels | cdecl | streaming::load_requested_models | bytes + Ghidra + gr Streaming.cpp:86 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40e460 | CStreaming::FlushChannels | cdecl | streaming::flush_channels | bytes + Ghidra + gr Streaming.cpp:61 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40e4e0 | CStreaming::FlushRequestList | cdecl | streaming::flush_request_list | bytes + Ghidra + gr Streaming.cpp:87 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40ea10 | CStreaming::LoadAllRequestedModels | cdecl | streaming::load_all_requested_models | bytes + Ghidra + gr Streaming.cpp:43 | chunk 0x40A45E; expect 6 bytes | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x409d10 | CStreaming::RequestSpecialModel | cdecl | streaming::request_special_model | bytes + Ghidra + gr Streaming.cpp:64 |  | implemented-needs-L2 | cdecl, 3 stack words, Void |
| 0x40e560 | CStreaming::ReInit | cdecl | streaming::re_init | bytes + Ghidra + gr Streaming.cpp:67 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40ed80 | CStreaming::LoadSceneCollision | cdecl | streaming::load_scene_collision | bytes + Ghidra + gr Streaming.cpp:49 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x410ad0 | CColStore::EnsureCollisionIsInMemory | cdecl | colstore::ensure_collision_is_in_memory | bytes + Ghidra + gr Collision/ColStore.cpp:438 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |
| 0x410e60 | CColStore::LoadAllCollision | cdecl | colstore::load_all_collision | bytes + Ghidra + gr Collision/ColStore.cpp:447 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40b450 | CStreaming::RequestSpecialChar | cdecl | streaming::request_special_char | bytes + Ghidra + gr Streaming.cpp:63 |  | implemented-needs-L2 | cdecl, 3 stack words, Void |
| 0x40e670 | CStreaming::Update | cdecl | streaming::update | bytes + Ghidra + gr Streaming.cpp:111 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x4113d0 | CColStore::LoadAllBoundingBoxes | cdecl | colstore::load_all_bounding_boxes | bytes + Ghidra + gr Collision/ColStore.cpp:448 |  | implemented-needs-L2 | cdecl, 0 stack words, Void |
| 0x40eb70 | CStreaming::LoadScene | cdecl | streaming::load_scene | bytes + Ghidra + gr Streaming.cpp:48 |  | implemented-needs-L2 | cdecl, 1 stack words, Void |

### Owner B: model infos except CVehicleModelInfo, CModelInfo, ModelIndices, store initialisers (`modelinfo/*`; specs `specs_stream_b.rs`)

| address | name | conv | port (sa-stream) | source | deviations followed (original kept); *new* = not in gr-deviations.csv | state | notes (L2 signature / reason) |
|---|---|---|---|---|---|---|---|
| 0x407320 | CTimeInfo::GetTimeOn | thiscall | `time::get_time_on` | gr (Models/TimeInfo.cpp:12) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(4) entry (movzx eax, byte ptr [ecx]; ret ; nop ) |
| 0x407330 | CTimeInfo::GetTimeOff | thiscall | `time::get_time_off` | gr (Models/TimeInfo.cpp:11) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CTimeInfo, 0 stack words, ret EAX |
| 0x4212c0 | CBaseModelInfo::SwaysInWind | thiscall | `base::sways_in_wind` | gr (Models/BaseModelInfo.cpp:49) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret EAX |
| 0x49cc20 | CBaseModelInfo::IsTagModel | thiscall | `base::is_tag_model` | gr (Models/BaseModelInfo.cpp:47) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret EAX |
| 0x4c42f0 | CBaseModelInfo::SetHasBeenPreRendered | thiscall | `base::set_has_been_pre_rendered` | gr (Models/BaseModelInfo.cpp:41) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 1 stack words, ret Void |
| 0x4c44b0 | CAtomicModelInfo::GetAtomicFromDistance | thiscall | `atomic::get_atomic_from_distance` | gr (Models/AtomicModelInfo.cpp:29) + bytes | x87 PC=24 product `LODDistMultiplier * drawDist` vs distance, only strictly greater returns the atomic (NaN → null) | L1 (1,060,000 cases, 0 mismatches (native CPU cross-check: 41136 cases)) | L2: thiscall, this=CAtomicModelInfo, 1 stack words, ret EAX |
| 0x4c4570 | CAtomicModelInfo::SetupVehicleUpgradeFlags | thiscall | `atomic::setup_vehicle_upgrade_flags` | gr (Models/AtomicModelInfo.cpp:30) + bytes | *new*: original truncates the flags word to its low byte first (also when nothing matches; gr only rewrites the CarMod bits on a match); *new*: the original prefix is `wg_r_` (0x85BB58), gr has `wg_r`; flags truncated to the low byte before matching; prefix tables decoded from the stack setup; entry `nop; jmp 0x40222E` (protection chunk) | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CAtomicModelInfo, 1 stack words, ret Void |
| 0x4c4a00 | CBaseModelInfo::IsLod | thiscall | `base::is_lod` | gr (Models/BaseModelInfo.cpp:39) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret EAX |
| 0x4c4a10 | CBaseModelInfo::SetIsLod | thiscall | `base::set_is_lod` | gr (Models/BaseModelInfo.cpp:42) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 1 stack words, ret Void |
| 0x4c4a30 | CTimeInfo::GetOtherTimeModel | thiscall | `time::get_other_time_model` | gr (Models/TimeInfo.cpp:10) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CTimeInfo, 0 stack words, ret EAX |
| 0x4c4a80 | CBaseModelInfo::AsAtomicModelInfoPtr | thiscall | `base::as_atomic_model_info_ptr` | gr (Models/BaseModelInfo.cpp:15) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (xor eax, eax; ret ; nop ; nop ) |
| 0x4c4a90 | CBaseModelInfo::AsDamageAtomicModelInfoPtr | thiscall | `base::as_damage_atomic_model_info_ptr` | gr (Models/BaseModelInfo.cpp:16) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (xor eax, eax; ret ; nop ; nop ) |
| 0x4c4aa0 | CBaseModelInfo::AsLodAtomicModelInfoPtr | thiscall | `base::as_lod_atomic_model_info_ptr` | gr (Models/BaseModelInfo.cpp:17) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (xor eax, eax; ret ; nop ; nop ) |
| 0x4c4ab0 | CBaseModelInfo::GetTimeInfo | thiscall | `base::get_time_info` | gr (Models/BaseModelInfo.cpp:18) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (xor eax, eax; ret ; nop ; nop ) |
| 0x4c4ac0 | CBaseModelInfo::SetAnimFile | thiscall | `base::set_anim_file` | gr (Models/BaseModelInfo.cpp:21) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (ret 4; nop ; nop ) |
| 0x4c4ad0 | CBaseModelInfo::ConvertAnimFileIndex | thiscall | `base::convert_anim_file_index` | gr (Models/BaseModelInfo.cpp:22) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(1) entry (ret ; nop ; nop ; nop ; nop ) |
| 0x4c4ae0 | CBaseModelInfo::GetAnimFileIndex | thiscall | `base::get_anim_file_index` | gr (Models/BaseModelInfo.cpp:23) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(4) entry (or eax, 0xffffffff; ret ; nop ) |
| 0x4c4b10 | CBaseModelInfo::Init | thiscall | `base::init` | gr (Models/BaseModelInfo.cpp:19) + bytes | gr-equal semantics (draw distance 2000.0, flags 0xC0); store order differs from gr | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret Void |
| 0x4c4b70 | CBaseModelInfo::ClearTexDictionary | thiscall | `base::clear_tex_dictionary` | gr (Models/BaseModelInfo.cpp:25) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret Void |
| 0x4c4bc0 | CBaseModelInfo::SetColModel | thiscall | `base::set_col_model` | gr (Models/BaseModelInfo.cpp:30) + bytes | loop over the time-model chain; `owns` read as a byte | implemented-needs-L2 | L2: thiscall, this=CBaseModelInfo, 2 stack words, ret Void |
| 0x4c4c20 | CBaseModelInfo::Init2dEffects | thiscall | `base::init_2d_effects` | gr (Models/BaseModelInfo.cpp:31) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret Void |
| 0x4c4df0 | CBaseModelInfo::IsRoad | thiscall | `base::is_road` | gr (Models/BaseModelInfo.cpp:40) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret EAX |
| 0x4c4e00 | CBaseModelInfo::HasComplexHierarchy | thiscall | `base::has_complex_hierarchy` | gr (Models/BaseModelInfo.cpp:37) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret EAX |
| 0x4c5200 | CClumpModelInfo::SetAnimFile | thiscall | `clump::set_anim_file` | gr (Models/ClumpModelInfo.cpp:26) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CClumpModelInfo, 1 stack words, ret Void |
| 0x4c5560 | CAtomicModelInfo::AsAtomicModelInfoPtr | thiscall | `atomic::as_atomic_model_info_ptr` | gr (Models/AtomicModelInfo.cpp:20) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (mov eax, ecx; ret ; nop ; nop ) |
| 0x4c5570 | CAtomicModelInfo::GetModelType | thiscall | `atomic::get_model_type` | gr (Models/AtomicModelInfo.cpp:21) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (mov al, 1; ret ; nop ; nop ) |
| 0x4c5580 | CAtomicModelInfo::GetRwModelType | thiscall | `atomic::get_rw_model_type` | gr (Models/AtomicModelInfo.cpp:24) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CAtomicModelInfo, 0 stack words, ret EAX |
| 0x4c5590 | FUN_004c5590 | thiscall | `atomic::destructor` | spec (CAtomicModelInfo.md) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CAtomicModelInfo, 0 stack words, ret Void |
| 0x4c55c0 | CDamageAtomicModelInfo::AsDamageAtomicModelInfoPtr | thiscall | `atomic::as_damage_atomic_model_info_ptr` | gr (Models/DamageAtomicModelInfo.cpp:13) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (mov eax, ecx; ret ; nop ; nop ) |
| 0x4c55d0 | FUN_004c55d0 | thiscall | `atomic::damage_destructor` | spec (CDamageAtomicModelInfo.md) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CDamageAtomicModelInfo, 0 stack words, ret Void |
| 0x4c5600 | CLodAtomicModelInfo::GetModelType | thiscall | `atomic::lod_get_model_type` | gr (Models/LodAtomicModelInfo.cpp:11) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (mov al, 8; ret ; nop ; nop ) |
| 0x4c5610 | CLodAtomicModelInfo::AsLodAtomicModelInfoPtr | thiscall | `atomic::as_lod_atomic_model_info_ptr` | gr (Models/LodAtomicModelInfo.cpp:10) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (mov eax, ecx; ret ; nop ; nop ) |
| 0x4c5620 | FUN_004c5620 | thiscall | `atomic::lod_destructor` | spec (CLodAtomicModelInfo.md) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CLodAtomicModelInfo, 0 stack words, ret Void |
| 0x4c5660 | CTimeModelInfo::GetModelType | thiscall | `time::time_get_model_type` | gr (Models/TimeModelInfo.cpp:9) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (mov al, 3; ret ; nop ; nop ) |
| 0x4c5670 | CTimeModelInfo::GetTimeInfo | thiscall | `time::time_get_time_info` | gr (Models/TimeModelInfo.cpp:10) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(4) entry (lea eax, [ecx + 0x20]; ret ; nop ) |
| 0x4c5680 | FUN_004c5680 | thiscall | `time::time_destructor` | spec (CTimeModelInfo.md) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CTimeModelInfo, 0 stack words, ret Void |
| 0x4c56c0 | CLodTimeModelInfo::vfunc_4 | thiscall | `time::lod_time_get_model_type` | spec (CLodTimeModelInfo.md) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (mov al, 3; ret ; nop ; nop ) |
| 0x4c56d0 | CLodTimeModelInfo::vfunc_5 | thiscall | `time::lod_time_get_time_info` | spec (CLodTimeModelInfo.md) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(4) entry (lea eax, [ecx + 0x24]; ret ; nop ) |
| 0x4c56e0 | FUN_004c56e0 | thiscall | `time::lod_time_destructor` | spec (CLodTimeModelInfo.md) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CLodTimeModelInfo, 0 stack words, ret Void |
| 0x4c5710 | CClumpModelInfo::GetBoundingBox | thiscall | `clump::get_bounding_box` | gr (Models/ClumpModelInfo.cpp:29) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(4) entry (mov eax, dword ptr [ecx + 0x14]; ret ; nop ) |
| 0x4c5720 | CClumpModelInfo::GetModelType | thiscall | `clump::get_model_type` | gr (Models/ClumpModelInfo.cpp:19) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (mov al, 5; ret ; nop ; nop ) |
| 0x4c5730 | CClumpModelInfo::GetRwModelType | thiscall | `clump::get_rw_model_type` | gr (Models/ClumpModelInfo.cpp:23) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CClumpModelInfo, 0 stack words, ret EAX |
| 0x4c5740 | CClumpModelInfo::GetAnimFileIndex | thiscall | `clump::get_anim_file_index` | gr (Models/ClumpModelInfo.cpp:28) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(4) entry (mov eax, dword ptr [ecx + 0x20]; ret ; nop ) |
| 0x4c5780 | CWeaponModelInfo::GetModelType | thiscall | `weapon::get_model_type` | gr (Models/WeaponModelInfo.cpp:10) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (mov al, 4; ret ; nop ; nop ) |
| 0x4c5790 | FUN_004c5790 | thiscall | `weapon::destructor` | spec (CWeaponModelInfo.md) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CWeaponModelInfo, 0 stack words, ret Void |
| 0x4c57c0 | CPedModelInfo::GetModelType | thiscall | `ped::get_model_type` | gr (Models/PedModelInfo.cpp:9) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (mov al, 7; ret ; nop ; nop ) |
| 0x4c59b0 | CModelInfo::GetModelInfoFromHashKey | cdecl | `model_info::get_model_info_from_hash_key` | gr (Models/ModelInfo.cpp:34) + bytes | none | implemented-needs-L2 | L2: cdecl, 2 stack words, ret EAX |
| 0x4c5a60 | CModelInfo::Get2dEffectStore | cdecl | `model_info::get_2d_effect_store` | spec (CModelInfo.md) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: cdecl, 0 stack words, ret EAX |
| 0x4c5a70 | CModelInfo::IsBoatModel | cdecl | `model_info::is_boat_model` | gr (Models/ModelInfo.cpp:48) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret AL |
| 0x4c5aa0 | CModelInfo::IsCarModel | cdecl | `model_info::is_car_model` | gr (Models/ModelInfo.cpp:49) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret AL |
| 0x4c5ad0 | CModelInfo::IsTrainModel | cdecl | `model_info::is_train_model` | gr (Models/ModelInfo.cpp:50) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret AL |
| 0x4c5b00 | CModelInfo::IsHeliModel | cdecl | `model_info::is_heli_model` | gr (Models/ModelInfo.cpp:51) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret AL |
| 0x4c5b30 | CModelInfo::IsPlaneModel | cdecl | `model_info::is_plane_model` | gr (Models/ModelInfo.cpp:52) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret AL |
| 0x4c5b60 | CModelInfo::IsBikeModel | cdecl | `model_info::is_bike_model` | gr (Models/ModelInfo.cpp:53) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret AL |
| 0x4c5b90 | CModelInfo::IsFakePlaneModel | cdecl | `model_info::is_fake_plane_model` | gr (Models/ModelInfo.cpp:54) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret AL |
| 0x4c5bc0 | CModelInfo::IsMonsterTruckModel | cdecl | `model_info::is_monster_truck_model` | gr (Models/ModelInfo.cpp:55) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret AL |
| 0x4c5bf0 | CModelInfo::IsQuadBikeModel | cdecl | `model_info::is_quad_bike_model` | gr (Models/ModelInfo.cpp:56) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret AL |
| 0x4c5c20 | CModelInfo::IsBmxModel | cdecl | `model_info::is_bmx_model` | gr (Models/ModelInfo.cpp:57) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret AL |
| 0x4c5c50 | CModelInfo::IsTrailerModel | cdecl | `model_info::is_trailer_model` | gr (Models/ModelInfo.cpp:58) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret AL |
| 0x4c5c80 | CModelInfo::IsVehicleModelType | cdecl | `model_info::is_vehicle_model_type` | gr (Models/ModelInfo.cpp:59) + bytes | id >= 20000 → -1 (signed: negative ids read before the table, as the original) | implemented-needs-L2 | L2: cdecl, 1 stack words, ret EAX |
| 0x4c6200 | FUN_004c6200 | thiscall | `base::vcall_shutdown_thunk` | spec (CBaseModelInfo.md) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret Void |
| 0x4c6620 | CModelInfo::AddAtomicModel | cdecl | `model_info::add_atomic_model` | gr (Models/ModelInfo.cpp:38) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret EAX |
| 0x4c6650 | CModelInfo::AddDamageAtomicModel | cdecl | `model_info::add_damage_atomic_model` | gr (Models/ModelInfo.cpp:39) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret EAX |
| 0x4c6680 | CModelInfo::AddLodAtomicModel | cdecl | `model_info::add_lod_atomic_model` | gr (Models/ModelInfo.cpp:40) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret EAX |
| 0x4c66b0 | CModelInfo::AddTimeModel | cdecl | `model_info::add_time_model` | gr (Models/ModelInfo.cpp:41) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret EAX |
| 0x4c66e0 | CModelInfo::AddLodTimeModel | cdecl | `model_info::add_lod_time_model` | gr (Models/ModelInfo.cpp:42) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret EAX |
| 0x4c6710 | CModelInfo::AddWeaponModel | cdecl | `model_info::add_weapon_model` | gr (Models/ModelInfo.cpp:43) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret EAX |
| 0x4c6740 | CModelInfo::AddClumpModel | cdecl | `model_info::add_clump_model` | gr (Models/ModelInfo.cpp:44) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret EAX |
| 0x4c6770 | CModelInfo::AddVehicleModel | cdecl | `model_info::add_vehicle_model` | gr (Models/ModelInfo.cpp:45) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret EAX |
| 0x4c67a0 | CModelInfo::AddPedModel | cdecl | `model_info::add_ped_model` | gr (Models/ModelInfo.cpp:46) + bytes | none | implemented-needs-L2 | L2: cdecl, 1 stack words, ret EAX |
| 0x4c6d40 | CPedModelInfo::AddXtraAtomics | thiscall | `ped::add_xtra_atomics` | gr (Models/PedModelInfo.cpp:12) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (ret 4; nop ; nop ) |
| 0x4c6d50 | CPedModelInfo::SetFaceTexture | thiscall | `ped::set_face_texture` | gr (Models/PedModelInfo.cpp:13) + bytes | none | ported-unhookable; L1 (1,002,000 cases, 0 mismatches) | no hook: TooShort(3) entry (ret 4; nop ; nop ) |
| 0x4c7300 | CPedModelInfo::IncrementVoice | thiscall | `ped::increment_voice` | gr (Models/PedModelInfo.cpp:17) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CPedModelInfo, 0 stack words, ret Void |
| 0x5328b0 | CBaseModelInfo::HasBeenPreRendered | thiscall | `base::has_been_pre_rendered` | gr (Models/BaseModelInfo.cpp:36) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret EAX |
| 0x5328c0 | CBaseModelInfo::GetIsDrawLast | thiscall | `base::get_is_draw_last` | gr (Models/BaseModelInfo.cpp:35) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret EAX |
| 0x5328d0 | CBaseModelInfo::SetOwnsColModel | thiscall | `base::set_owns_col_model` | gr (Models/BaseModelInfo.cpp:43) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 1 stack words, ret Void |
| 0x5328f0 | CBaseModelInfo::IsBackfaceCulled | thiscall | `base::is_backface_culled` | gr (Models/BaseModelInfo.cpp:38) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret EAX |
| 0x532900 | CBaseModelInfo::IncreaseAlpha | thiscall | `base::increase_alpha` | gr (Models/BaseModelInfo.cpp:44) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret Void |
| 0x59f090 | CBaseModelInfo::IsBreakableStatuePart | thiscall | `base::is_breakable_statue_part` | gr (Models/BaseModelInfo.cpp:46) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret EAX |
| 0x5b3430 | CTimeInfo::SetTimes | thiscall | `time::set_times` | gr (Models/TimeInfo.cpp:14) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CTimeInfo, 2 stack words, ret Void |
| 0x5b3440 | CTimeInfo::SetOtherTimeModel | thiscall | `time::set_other_time_model` | gr (Models/TimeInfo.cpp:13) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CTimeInfo, 1 stack words, ret Void |
| 0x5b3ad0 | CBaseModelInfo::SetBaseModelInfoFlags | cdecl | `base::set_base_model_info_flags` | gr (Models/BaseModelInfo.cpp:51) + bytes | six byte RMWs folded into one byte write (same final value, single thread) | L1 (1,002,000 cases, 0 mismatches) | L2: cdecl, 2 stack words, ret Void |
| 0x5bca10 | ModelIndices::InitModelIndices | cdecl | `model_indices::init_model_indices` | gr (ModelIndices.cpp:177) + bytes | 116 cells decoded from the instruction stream | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x856230 | __dtor_CModelInfo__ms_atomicModelInfoStore | cdecl | `model_info::dtor_atomic_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x856240 | __dtor_CModelInfo__ms_damageAtomicModelInfoStore | cdecl | `model_info::dtor_damage_atomic_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x856250 | __dtor_CModelInfo__ms_lodAtomicModelInfoStore | cdecl | `model_info::dtor_lod_atomic_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x856260 | __dtor_CModelInfo__ms_timeModelInfoStore | cdecl | `model_info::dtor_time_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x856270 | __dtor_CModelInfo__ms_lodTimeModelInfoStore | cdecl | `model_info::dtor_lod_time_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x856280 | __dtor_CModelInfo__ms_weaponModelInfoStore | cdecl | `model_info::dtor_weapon_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x856290 | __dtor_CModelInfo__ms_clumpModelInfoStore | cdecl | `model_info::dtor_clump_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x8562a0 | __dtor_CModelInfo__ms_vehicleModelInfoStore | cdecl | `model_info::dtor_vehicle_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x8562b0 | __dtor_CModelInfo__ms_pedModelInfoStore | cdecl | `model_info::dtor_ped_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x4c4430 | CAtomicModelInfo::Init | thiscall | `atomic::init` | gr (Models/AtomicModelInfo.cpp:22) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CAtomicModelInfo, 0 stack words, ret Void |
| 0x4c47e0 | CTimeInfo::FindOtherTimeModel | thiscall | `time::find_other_time_model` | gr (Models/TimeInfo.cpp:9) + bytes | *new*: the original scans all models for the first one with a time info AND the key (gr: GetModelInfoFromHashKey = first key match, then tests its time info; differs when an earlier non-time model has the same key); unbounded strcpy into a 0x18-byte stack buffer (original UB for names ≥ 0x18; the port truncates at 0xFF) | implemented-needs-L2 | L2: thiscall, this=CTimeInfo, 1 stack words, ret EAX |
| 0x4c48b0 | CDamageAtomicModelInfo::Init | thiscall | `atomic::damage_init` | gr (Models/DamageAtomicModelInfo.cpp:12) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CDamageAtomicModelInfo, 0 stack words, ret Void |
| 0x4c4af0 | CBaseModelInfo::vfunc_0 | thiscall | `base::scalar_deleting_destructor` | spec (CBaseModelInfo.md) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CBaseModelInfo, 1 stack words, ret EAX |
| 0x4c4b80 | CBaseModelInfo::AddTexDictionaryRef | thiscall | `base::add_tex_dictionary_ref` | gr (Models/BaseModelInfo.cpp:26) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret Void |
| 0x4c4ba0 | CBaseModelInfo::AddRef | thiscall | `base::add_ref` | gr (Models/BaseModelInfo.cpp:28) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret Void |
| 0x4c4d20 | CBaseModelInfo::Add2dEffect | thiscall | `base::add_2d_effect` | gr (Models/BaseModelInfo.cpp:34) + bytes | index = signed `(effect - store - 4) >> 6` | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CBaseModelInfo, 1 stack words, ret Void |
| 0x4c4e40 | CClumpModelInfo::Init | thiscall | `clump::init` | gr (Models/ClumpModelInfo.cpp:20) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CClumpModelInfo, 0 stack words, ret Void |
| 0x4c5280 | CClumpModelInfo::SetAtomicRendererCB | cdecl | `clump::set_atomic_renderer_cb` | gr (Models/ClumpModelInfo.cpp:34) + bytes | none | implemented-needs-L2 | L2: cdecl, 2 stack words, ret EAX |
| 0x4c52a0 | CClumpModelInfo::FindFrameFromNameCB | cdecl | `clump::find_frame_from_name_cb` | gr (Models/ClumpModelInfo.cpp:35) + bytes | none | implemented-needs-L2 | L2: cdecl, 2 stack words, ret EAX |
| 0x4c52f0 | CClumpModelInfo::FindFrameFromNameWithoutIdCB | cdecl | `clump::find_frame_from_name_without_id_cb` | gr (Models/ClumpModelInfo.cpp:36) + bytes | none | implemented-needs-L2 | L2: cdecl, 2 stack words, ret EAX |
| 0x4c5350 | CClumpModelInfo::FindFrameFromIdCB | cdecl | `clump::find_frame_from_id_cb` | gr (Models/ClumpModelInfo.cpp:37) + bytes | none | implemented-needs-L2 | L2: cdecl, 2 stack words, ret EAX |
| 0x4c5390 | CClumpModelInfo::FillFrameArrayCB | cdecl | `clump::fill_frame_array_cb` | gr (Models/ClumpModelInfo.cpp:38) + bytes | none | implemented-needs-L2 | L2: cdecl, 2 stack words, ret EAX |
| 0x4c53c0 | CClumpModelInfo::GetFrameFromId | cdecl | `clump::get_frame_from_id` | gr (Models/ClumpModelInfo.cpp:39) + bytes | none | implemented-needs-L2 | L2: cdecl, 2 stack words, ret EAX |
| 0x4c5400 | CClumpModelInfo::GetFrameFromName | cdecl | `clump::get_frame_from_name` | gr (Models/ClumpModelInfo.cpp:40) + bytes | none | implemented-needs-L2 | L2: cdecl, 2 stack words, ret EAX |
| 0x4c5440 | CClumpModelInfo::FillFrameArray | cdecl | `clump::fill_frame_array` | gr (Models/ClumpModelInfo.cpp:41) + bytes | none | implemented-needs-L2 | L2: cdecl, 2 stack words, ret Void |
| 0x4c5460 | CClumpModelInfo::SetFrameIds | thiscall | `clump::set_frame_ids` | gr (Models/ClumpModelInfo.cpp:31) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CClumpModelInfo, 1 stack words, ret Void |
| 0x4c57a0 | CPedModelInfo::CPedModelInfo | thiscall | `ped::constructor` | spec (CPedModelInfo.md) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CPedModelInfo, 0 stack words, ret EAX |
| 0x4c5940 | CModelInfo::GetModelInfo-full | cdecl | `model_info::get_model_info_full` | gr (Models/ModelInfo.cpp:35) + bytes | entry protection chunk 0x4014ED computes &ms_lastPositionSearched (`rol([0x1560644], 16)`); *index written before ms_lastPositionSearched (gr: after) | implemented-needs-L2 | L2: cdecl, 2 stack words, ret EAX |
| 0x4c5a20 | CModelInfo::GetModelInfo-minmax | cdecl | `model_info::get_model_info_minmax` | gr (Models/ModelInfo.cpp:36) + bytes | none | implemented-needs-L2 | L2: cdecl, 3 stack words, ret EAX |
| 0x4c6210 | CAtomicModelInfo::vfunc_0 | thiscall | `atomic::scalar_deleting_destructor` | spec (CAtomicModelInfo.md) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CAtomicModelInfo, 1 stack words, ret EAX |
| 0x4c6230 | CDamageAtomicModelInfo::vfunc_0 | thiscall | `atomic::damage_scalar_deleting_destructor` | spec (CDamageAtomicModelInfo.md) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CDamageAtomicModelInfo, 1 stack words, ret EAX |
| 0x4c6250 | CLodAtomicModelInfo::vfunc_0 | thiscall | `atomic::lod_scalar_deleting_destructor` | spec (CLodAtomicModelInfo.md) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CLodAtomicModelInfo, 1 stack words, ret EAX |
| 0x4c6270 | CTimeModelInfo::vfunc_0 | thiscall | `time::time_scalar_deleting_destructor` | spec (CTimeModelInfo.md) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CTimeModelInfo, 1 stack words, ret EAX |
| 0x4c6290 | CLodTimeModelInfo::vfunc_0 | thiscall | `time::lod_time_scalar_deleting_destructor` | spec (CLodTimeModelInfo.md) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CLodTimeModelInfo, 1 stack words, ret EAX |
| 0x4c62b0 | CClumpModelInfo::vfunc_0 | thiscall | `clump::scalar_deleting_destructor` | spec (CClumpModelInfo.md) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CClumpModelInfo, 1 stack words, ret EAX |
| 0x4c62d0 | CWeaponModelInfo::vfunc_0 | thiscall | `weapon::scalar_deleting_destructor` | spec (CWeaponModelInfo.md) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CWeaponModelInfo, 1 stack words, ret EAX |
| 0x4c63b0 | CModelInfo::ReInit2dEffects | cdecl | `model_info::re_init_2d_effects` | gr (Models/ModelInfo.cpp:31) + bytes | *new*: original skips null entries; gr calls Init2dEffects on every slot without a null check | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x5b3b20 | CAtomicModelInfo::SetAtomicModelInfoFlags | cdecl | `atomic::set_atomic_model_info_flags` | gr (Models/AtomicModelInfo.cpp:31) + bytes | byte/word RMWs folded into one word write (same final value) | L1 (1,002,000 cases, 0 mismatches) | L2: cdecl, 2 stack words, ret Void |
| 0x5b3c30 | CClumpModelInfo::SetClumpModelInfoFlags | cdecl | `clump::set_clump_model_info_flags` | gr (Models/ClumpModelInfo.cpp:43) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: cdecl, 2 stack words, ret Void |
| 0x84bbf0 | __init_CModelInfo__ms_atomicModelInfoStore | cdecl | `model_info::init_atomic_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void; runs in CRT start-up before sa-hook attaches (L2 only with a start-up record) |
| 0x84bc10 | __init_CModelInfo__ms_damageAtomicModelInfoStore | cdecl | `model_info::init_damage_atomic_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void; runs in CRT start-up before sa-hook attaches (L2 only with a start-up record) |
| 0x84bc30 | __init_CModelInfo__ms_lodAtomicModelInfoStore | cdecl | `model_info::init_lod_atomic_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void; runs in CRT start-up before sa-hook attaches (L2 only with a start-up record) |
| 0x84bc50 | __init_CModelInfo__ms_timeModelInfoStore | cdecl | `model_info::init_time_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void; runs in CRT start-up before sa-hook attaches (L2 only with a start-up record) |
| 0x84bc70 | __init_CModelInfo__ms_lodTimeModelInfoStore | cdecl | `model_info::init_lod_time_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void; runs in CRT start-up before sa-hook attaches (L2 only with a start-up record) |
| 0x84bc90 | __init_CModelInfo__ms_weaponModelInfoStore | cdecl | `model_info::init_weapon_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void; runs in CRT start-up before sa-hook attaches (L2 only with a start-up record) |
| 0x84bcb0 | __init_CModelInfo__ms_clumpModelInfoStore | cdecl | `model_info::init_clump_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void; runs in CRT start-up before sa-hook attaches (L2 only with a start-up record) |
| 0x84bcd0 | __init_CModelInfo__ms_vehicleModelInfoStore | cdecl | `model_info::init_vehicle_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void; runs in CRT start-up before sa-hook attaches (L2 only with a start-up record) |
| 0x84bcf0 | __init_CModelInfo__ms_pedModelInfoStore | cdecl | `model_info::init_ped_store` | spec (_static-init.md) + bytes | none | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void; runs in CRT start-up before sa-hook attaches (L2 only with a start-up record) |
| 0x4c4b40 | CBaseModelInfo::SetTexDictionary | thiscall | `base::set_tex_dictionary` | gr (Models/BaseModelInfo.cpp:24) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CBaseModelInfo, 1 stack words, ret Void |
| 0x4c4c70 | CBaseModelInfo::Get2dEffect | thiscall | `base::get_2d_effect` | gr (Models/BaseModelInfo.cpp:33) + bytes | *new*: second virtual GetRwModelType call when the first test fails; no-geometry case reads `*(0 + plugin offset)` like the original (gr: RpGeometryGet2dFxAtIndex(nullptr)) | implemented-needs-L2 | L2: thiscall, this=CBaseModelInfo, 1 stack words, ret EAX |
| 0x4c4ef0 | CClumpModelInfo::SetHierarchyForSkinAtomic | cdecl | `clump::set_hierarchy_for_skin_atomic` | gr (Models/ClumpModelInfo.cpp:32) + bytes | none | implemented-needs-L2 | L2: cdecl, 2 stack words, ret EAX |
| 0x4c5250 | CClumpModelInfo::ConvertAnimFileIndex | thiscall | `clump::convert_anim_file_index` | gr (Models/ClumpModelInfo.cpp:27) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CClumpModelInfo, 0 stack words, ret Void |
| 0x4c54d0 | CLodAtomicModelInfo::Init | thiscall | `atomic::lod_init` | gr (Models/LodAtomicModelInfo.cpp:12) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CLodAtomicModelInfo, 0 stack words, ret Void |
| 0x4c59f0 | CModelInfo::GetModelInfoUInt16 | cdecl | `model_info::get_model_info_uint16` | gr (Models/ModelInfo.cpp:33) + bytes | index initialised to 0: a name not found writes 0 into *out (as gr) | implemented-needs-L2 | L2: cdecl, 2 stack words, ret EAX |
| 0x4c6d90 | CPedModelInfo::CreateHitColModelSkinned | thiscall | `ped::create_hit_col_model_skinned` | gr (Models/PedModelInfo.cpp:14) + bytes | no null check of `new CColModel` (as gr); C++ EH frame not reproduced (no observable effect) | implemented-needs-L2 | L2: thiscall, this=CPedModelInfo, 1 stack words, ret Void |
| 0x4c98f0 | CWeaponModelInfo::Init | thiscall | `weapon::init` | gr (Models/WeaponModelInfo.cpp:11) + bytes | none | L1 (1,002,000 cases, 0 mismatches) | L2: thiscall, this=CWeaponModelInfo, 0 stack words, ret Void |
| 0x4c4360 | CAtomicModelInfo::SetAtomic | thiscall | `atomic::set_atomic` | gr (Models/AtomicModelInfo.cpp:27) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CAtomicModelInfo, 1 stack words, ret Void |
| 0x4c48d0 | CDamageAtomicModelInfo::SetDamagedAtomic | thiscall | `atomic::set_damaged_atomic` | gr (Models/DamageAtomicModelInfo.cpp:18) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CDamageAtomicModelInfo, 1 stack words, ret Void |
| 0x4c4f30 | CClumpModelInfo::AtomicSetupLightingCB | cdecl | `clump::atomic_setup_lighting_cb` | gr (Models/ClumpModelInfo.cpp:33) + bytes | none | implemented-needs-L2 | L2: cdecl, 2 stack words, ret EAX |
| 0x4c4f70 | CClumpModelInfo::SetClump | thiscall | `clump::set_clump` | gr (Models/ClumpModelInfo.cpp:30) + bytes | ClumpModelInfo.cpp:196 unclear; gr-deviations ClumpModelInfo.cpp:196 (harmless, same arithmetic); old clump's 2dfx count via protection chunk 0x401307 (`[[0x1560618]^[0x5A6FDC]]` = 0xC3A1E0), computed at run time | implemented-needs-L2 | L2: thiscall, this=CClumpModelInfo, 1 stack words, ret Void |
| 0x4c6810 | CModelInfo::Initialise | cdecl | `model_info::initialise` | gr (Models/ModelInfo.cpp:29) + bytes | temp models 374–381 created inline (AddAtomicModel inlined); atomic count set to 1 directly | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x4c6f70 | CPedModelInfo::AnimatePedColModelSkinned | thiscall | `ped::animate_ped_col_model_skinned` | gr (Models/PedModelInfo.cpp:15) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CPedModelInfo, 1 stack words, ret EAX |
| 0x4c7170 | CPedModelInfo::AnimatePedColModelSkinnedWorld | thiscall | `ped::animate_ped_col_model_skinned_world` | gr (Models/PedModelInfo.cpp:16) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CPedModelInfo, 1 stack words, ret EAX |
| 0x5b57c0 | ModelIndices::MatchAllModelStrings | cdecl | `model_indices::match_all_model_strings` | gr (ModelIndices.cpp:178) + bytes | 160 (name, cell) pairs decoded from the instruction stream | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x4c7340 | CPedModelInfo::SetClump | thiscall | `ped::set_clump` | gr (Models/PedModelInfo.cpp:11) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CPedModelInfo, 1 stack words, ret Void |
| 0x4c9910 | CWeaponModelInfo::SetClump | thiscall | `weapon::set_clump` | gr (Models/WeaponModelInfo.cpp:12) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CWeaponModelInfo, 1 stack words, ret Void |
| 0x4c4c40 | CBaseModelInfo::DeleteCollisionModel | thiscall | `base::delete_collision_model` | gr (Models/BaseModelInfo.cpp:32) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret Void |
| 0x4c62f0 | CPedModelInfo::~CPedModelInfo | thiscall | `ped::destructor` | spec (CPedModelInfo.md) + bytes | hit col model deleted, pointer not cleared; EH frame not reproduced | implemented-needs-L2 | L2: thiscall, this=CPedModelInfo, 0 stack words, ret Void |
| 0x4c6350 | CPedModelInfo::vfunc_0 | thiscall | `ped::scalar_deleting_destructor` | spec (CPedModelInfo.md) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CPedModelInfo, 1 stack words, ret EAX |
| 0x4c63e0 | CModelInfo::ShutDown | cdecl | `model_info::shut_down` | gr (Models/ModelInfo.cpp:30) + bytes | counts compared signed and re-read per iteration (gr: uint32); C2dEffect shutdown inlined (lights: corona/shadow textures; type 7: text via CMemoryMgr::Free, atomic+frame) | implemented-needs-L2 | L2: cdecl, 0 stack words, ret Void |
| 0x4c4b90 | CBaseModelInfo::RemoveTexDictionaryRef | thiscall | `base::remove_tex_dictionary_ref` | gr (Models/BaseModelInfo.cpp:27) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret Void |
| 0x4c4bb0 | CBaseModelInfo::RemoveRef | thiscall | `base::remove_ref` | gr (Models/BaseModelInfo.cpp:29) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret Void |
| 0x4c4440 | CAtomicModelInfo::DeleteRwObject | thiscall | `atomic::delete_rw_object` | gr (Models/AtomicModelInfo.cpp:23) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CAtomicModelInfo, 0 stack words, ret Void |
| 0x4c44d0 | CAtomicModelInfo::CreateInstance-rwmat | thiscall | `atomic::create_instance_rwmat` | gr (Models/AtomicModelInfo.cpp:26) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CAtomicModelInfo, 1 stack words, ret EAX |
| 0x4c4530 | CAtomicModelInfo::CreateInstance-void | thiscall | `atomic::create_instance_void` | gr (Models/AtomicModelInfo.cpp:25) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CAtomicModelInfo, 0 stack words, ret EAX |
| 0x4c4960 | CDamageAtomicModelInfo::CreateInstance-void | thiscall | `atomic::damage_create_instance_void` | gr (Models/DamageAtomicModelInfo.cpp:15) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CDamageAtomicModelInfo, 0 stack words, ret EAX |
| 0x4c4e70 | CClumpModelInfo::DeleteRwObject | thiscall | `clump::delete_rw_object` | gr (Models/ClumpModelInfo.cpp:22) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CClumpModelInfo, 0 stack words, ret Void |
| 0x4c5140 | CClumpModelInfo::CreateInstance-void | thiscall | `clump::create_instance_void` | gr (Models/ClumpModelInfo.cpp:24) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CClumpModelInfo, 0 stack words, ret EAX |
| 0x4c4910 | CDamageAtomicModelInfo::CreateInstance-rwmat | thiscall | `atomic::damage_create_instance_rwmat` | gr (Models/DamageAtomicModelInfo.cpp:16) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CDamageAtomicModelInfo, 1 stack words, ret EAX |
| 0x4c49d0 | CDamageAtomicModelInfo::DeleteRwObject | thiscall | `atomic::damage_delete_rw_object` | gr (Models/DamageAtomicModelInfo.cpp:14) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CDamageAtomicModelInfo, 0 stack words, ret Void |
| 0x4c6c50 | CPedModelInfo::DeleteRwObject | thiscall | `ped::delete_rw_object` | gr (Models/PedModelInfo.cpp:10) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CPedModelInfo, 0 stack words, ret Void |
| 0x4c4d50 | CBaseModelInfo::Shutdown | thiscall | `base::shutdown` | gr (Models/BaseModelInfo.cpp:20) + bytes | col model deleted only with bIsLod (bit 0x80), as gr; store order: flags, col, 2dfx idx, 2dfx count, object info, txd | implemented-needs-L2 | L2: thiscall, this=CBaseModelInfo, 0 stack words, ret Void |
| 0x4c5110 | CClumpModelInfo::CreateInstance-mat | thiscall | `clump::create_instance_mat` | gr (Models/ClumpModelInfo.cpp:25) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CClumpModelInfo, 1 stack words, ret EAX |
| 0x4c4e60 | CClumpModelInfo::Shutdown | thiscall | `clump::shutdown` | gr (Models/ClumpModelInfo.cpp:21) + bytes | none | implemented-needs-L2 | L2: thiscall, this=CClumpModelInfo, 0 stack words, ret Void |

### Owner C: CFileLoader, CVehicleModelInfo (`fileloader.rs`, `modelinfo/vehicle.rs`; specs `specs_stream_c.rs`)

| address | name | conv | port (sa-stream) | source | deviations followed (original kept); *new* = not in gr-deviations.csv | state | notes (L2 signature / reason) |
|---|---|---|---|---|---|---|---|
| 0x4c5920 | CVehicleModelInfo::~CVehicleModelInfo | thiscall | vehicle::destructor | bytes + spec | - | L1 (VehDestructor) |  |
| 0x4c73c0 | CVehicleModelInfo::GetNumDoors | thiscall | vehicle::get_num_doors | bytes + gr | - | L1 (VehGetNumDoors) |  |
| 0x4c74b0 | CLinkedUpgradeList::AddUpgradeLink | thiscall | vehicle::add_upgrade_link | bytes + gr | - | L1 (LinkedAddUpgradeLink) |  |
| 0x4c74d0 | CLinkedUpgradeList::FindOtherUpgrade | thiscall | vehicle::find_other_upgrade | bytes + gr | - | L1 (LinkedFindOtherUpgrade) |  |
| 0x4c7650 | CVehicleModelInfo::GetModelType | - | vehicle::get_model_type | bytes + gr | - | ported-unhookable | cdecl-less `mov al,6; ret` (TooShort(3)) |
| 0x4c7660 | CVehicleModelInfo::GetAnimFileIndex | thiscall | vehicle::get_anim_file_index | bytes + gr | - | L1 (VehGetAnimFileIndex) |  |
| 0x4c77c0 | CVehicleModelInfo::HasAlphaMaterialCB | cdecl | vehicle::has_alpha_material_cb | bytes + gr | - | L1 (VehHasAlphaMaterialCB) |  |
| 0x4c7e10 | CVehicleModelInfo::IsValidCompRule | cdecl | vehicle::is_valid_comp_rule | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7e50 | CVehicleModelInfo::GetListOfComponentsNotUsedByRules | cdecl | vehicle::get_list_of_components_not_used_by_rules | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7f80 | CVehicleModelInfo::CountCompsInRule | cdecl | vehicle::count_comps_in_rule | bytes + gr | - | L1 (VehCountCompsInRule) |  |
| 0x4c8200 | CVehicleModelInfo::IsUpgradeAvailable | thiscall | vehicle::is_upgrade_available | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8220 | CVehicleModelInfo::SetEditableMaterialsCB-RpMaterial | cdecl | vehicle::set_editable_materials_cb_material | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c83b0 | CVehicleModelInfo::RemoveWindowAlphaCB | cdecl | vehicle::remove_window_alpha_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8460 | CVehicleModelInfo::ResetEditableMaterials | cdecl | vehicle::reset_editable_materials | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c84b0 | CVehicleModelInfo::SetVehicleColour | thiscall | vehicle::set_vehicle_colour | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c86b0 | CVehicleModelInfo::GetNumRemaps | thiscall | vehicle::get_num_remaps | bytes + gr | - | L1 (VehGetNumRemaps) |  |
| 0x4c86d0 | CVehicleModelInfo::AddRemap | thiscall | vehicle::add_remap | bytes + gr | - | L1 (VehAddRemap) |  |
| 0x4c8700 | CVehicleModelInfo::AddWheelUpgrade | cdecl | vehicle::add_wheel_upgrade | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8740 | CVehicleModelInfo::GetNumWheelUpgrades | cdecl | vehicle::get_num_wheel_upgrades | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8750 | CVehicleModelInfo::GetWheelUpgrade | cdecl | vehicle::get_wheel_upgrade | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8770 | CVehicleModelInfo::DeleteVehicleColourTextures | - | vehicle::delete_vehicle_colour_textures | bytes + gr | - | ported-unhookable | lone `ret` (TooShort(1)) |
| 0x4c8970 | CVehicleModelInfo::GetCustomCarPlateText | thiscall | vehicle::get_custom_car_plate_text | bytes + gr | - | L1 (VehGetCustomCarPlateText) |  |
| 0x4c8980 | CVehicleModelInfo::SetCustomCarPlateText | thiscall | vehicle::set_custom_car_plate_text | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c89b0 | CVehicleModelInfo::GetMaximumNumberOfPassengersFromNumberOfDoors | cdecl | vehicle::get_maximum_number_of_passengers_from_number_of_doors | bytes + gr | - | implemented-needs-L2 | handling id read as byte |
| 0x4c8c90 | CVehicleModelInfo::SetupLightFlags | cdecl | vehicle::setup_light_flags | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8d60 | CVehicleModelInfo::CVehicleStructure::CVehicleStructure | thiscall | vehicle::vehicle_structure_ctor | bytes + spec | - | implemented-needs-L2 |  |
| 0x536fe0 | CFileLoader::LoadLine-Bufer | cdecl | fileloader::load_line_buffer | bytes + gr + spec | FileLoader.cpp:250 keep-original ; FileLoader.cpp:251 unclear | implemented-needs-L2 | '\r' copied as space (:250/251) |
| 0x5b41c0 | CFileLoader::LoadPathHeader | cdecl | fileloader::load_path_header | bytes + gr | - | implemented-needs-L2 |  |
| 0x6a06f0 | CVehicleModelInfo::GetWheelSize | thiscall | vehicle::get_wheel_size | bytes + spec | - | L1 (VehGetWheelSize) |  |
| 0x4c7510 | CVehicleModelInfo::FindTextureCB | cdecl | vehicle::find_texture_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c75a0 | CVehicleModelInfo::UseCommonVehicleTexDicationary | cdecl | vehicle::use_common_vehicle_tex_dicationary | bytes + gr | - | implemented-needs-L2 | FindTextureCB address computed as the protection chunk 0x406413 does (rol [0x1560810], [0x71A0E2]) |
| 0x4c75c0 | CVehicleModelInfo::StopUsingCommonVehicleTexDicationary | cdecl | vehicle::stop_using_common_vehicle_tex_dicationary | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7670 | CVehicleModelInfo::SetAnimFile | thiscall | vehicle::set_anim_file | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7720 | CVehicleModelInfo::HideDamagedAtomicCB | cdecl | vehicle::hide_damaged_atomic_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7790 | CVehicleModelInfo::HideAllComponentsAtomicCB | cdecl | vehicle::hide_all_components_atomic_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7a30 | CVehicleModelInfo::SetAtomicRendererCB_Heli | cdecl | vehicle::set_atomic_renderer_cb_heli | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7b10 | CVehicleModelInfo::SetAtomicRenderCallbacks | thiscall | vehicle::set_atomic_render_callbacks | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7b90 | CVehicleModelInfo::SetAtomicFlagCB | cdecl | vehicle::set_atomic_flag_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7bb0 | CVehicleModelInfo::ClearAtomicFlagCB | cdecl | vehicle::clear_atomic_flag_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7bd0 | CVehicleModelInfo::GetOkAndDamagedAtomicCB | cdecl | vehicle::get_ok_and_damaged_atomic_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7c10 | CVehicleModelInfo::SetVehicleComponentFlags | thiscall | vehicle::set_vehicle_component_flags | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7fb0 | CVehicleModelInfo::ChooseComponent-Global | cdecl | vehicle::choose_component_global | bytes + gr | - | implemented-needs-L2 | rule 4 tail call overwrites its own arg slots (not reproduced, dead stack) |
| 0x4c83e0 | CVehicleModelInfo::SetEditableMaterialsCB-RpAtomic | cdecl | vehicle::set_editable_materials_cb_atomic | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8430 | CVehicleModelInfo::SetEditableMaterials | cdecl | vehicle::set_editable_materials | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8500 | CVehicleModelInfo::ChooseVehicleColour | thiscall | vehicle::choose_vehicle_colour | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8810 | CVehicleModelInfo::GetMatFXEffectMaterialCB | cdecl | vehicle::get_mat_fx_effect_material_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c88b0 | CVehicleModelInfo::SetEnvMapCoeffCB | cdecl | vehicle::set_env_map_coeff_cb | bytes + gr | - | implemented-needs-L2 | (float)(uint32)data * 0.001 under PC=24 |
| 0x4c8e10 | CVehicleModelInfo::vfunc_0 | thiscall | vehicle::scalar_deleting_destructor | bytes + spec | - | implemented-needs-L2 |  |
| 0x4c9410 | CVehicleModelInfo::SetEnvironmentMapAtomicCB | cdecl | vehicle::set_environment_map_atomic_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c9430 | CVehicleModelInfo::SetEnvMapCoeffAtomicCB | cdecl | vehicle::set_env_map_coeff_atomic_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c97e0 | CVehicleModelInfo::DisableEnvMap | thiscall | vehicle::disable_env_map | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c9800 | CVehicleModelInfo::SetEnvMapCoeff | thiscall | vehicle::set_env_map_coeff | bytes + gr | - | implemented-needs-L2 | `_ftol(floor((double)(coeff*1000)))`, floor 0x8219F0 inline |
| 0x536f80 | CFileLoader::LoadLine-File | cdecl | fileloader::load_line_file | bytes + gr | - | implemented-needs-L2 |  |
| 0x537580 | CFileLoader::LoadCollisionModel | cdecl | fileloader::load_collision_model | bytes + gr + spec | FileLoader.cpp:637 harmless ; FileLoader.cpp:672 keep-original | implemented-needs-L2 | ported from bytes (entry protection chunk pushes the EH handler); CCollisionData constructed; lines skipped, +6/+0x10 cleared |
| 0x537ae0 | CFileLoader::LoadCollisionModelVer4 | cdecl | fileloader::load_collision_model_ver4 | bytes + gr + spec | FileLoader.cpp:864 harmless ; FileLoader.cpp:874 harmless ; FileLoader.cpp:875 unclear ; FileLoader.cpp:920 harmless | implemented-needs-L2 |  |
| 0x537ce0 | CFileLoader::LoadCollisionModelVer3 | cdecl | fileloader::load_collision_model_ver3 | bytes + gr + spec | FileLoader.cpp:792 keep-original ; FileLoader.cpp:802 harmless ; FileLoader.cpp:803 unclear ; FileLoader.cpp:848 harmless | implemented-needs-L2 |  |
| 0x537ee0 | CFileLoader::LoadCollisionModelVer2 | cdecl | fileloader::load_collision_model_ver2 | bytes + gr + spec | FileLoader.cpp:719 keep-original ; FileLoader.cpp:729 harmless ; FileLoader.cpp:730 unclear ; FileLoader.cpp:776 harmless | implemented-needs-L2 | +0x20 not written (CFileLoader.deviations.md :730) |
| 0x5b3910 | CFileLoader::AddTexDictionaries | cdecl | fileloader::add_tex_dictionaries | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b41f0 | CFileLoader::LoadPedPathNode | - | - | bytes + gr | - | deferred | deferred: parses the line and computes x87/_ftol arguments only for CPathFind stub 0x44D2E0 (`ret` in 1.0 US); the exact argument derivation (protection-thunk format pointer 0x4022CD) is pending; callers keep calling the original |
| 0x5b4380 | CFileLoader::LoadCarPathNode | - | - | bytes + gr | - | deferred | deferred: as LoadPedPathNode (callees 0x44D2C0/0x44D2F0 are `ret 0x44` stubs) |
| 0x5b4530 | CFileLoader::LoadGarage | cdecl | fileloader::load_garage | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b4c80 | CFileLoader::LoadOcclusionVolume | cdecl | fileloader::load_occlusion_volume | bytes + gr + spec | FileLoader.cpp:1424 keep-original | implemented-needs-L2 | "int" test at len-7..len-5 (:1424) |
| 0x5b4d70 | CFileLoader::LoadAudioZone | cdecl | fileloader::load_audio_zone | bytes + gr | - | implemented-needs-L2 | *new*: flag passed as `flags != 0` (gr `== 1`) |
| 0x5b81d0 | CFileLoader::LoadTimeCyclesModifier | cdecl | fileloader::load_time_cycles_modifier | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7630 | CVehicleModelInfo::Init | thiscall | vehicle::init | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c76d0 | CVehicleModelInfo::ConvertAnimFileIndex | thiscall | vehicle::convert_anim_file_index | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c77e0 | CVehicleModelInfo::SetAtomicRendererCB | cdecl | vehicle::set_atomic_renderer_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7870 | CVehicleModelInfo::SetAtomicRendererCB_RealHeli | cdecl | vehicle::set_atomic_renderer_cb_real_heli | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7930 | CVehicleModelInfo::SetAtomicRendererCB_Plane | cdecl | vehicle::set_atomic_renderer_cb_plane | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c79a0 | CVehicleModelInfo::SetAtomicRendererCB_Boat | cdecl | vehicle::set_atomic_renderer_cb_boat | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7aa0 | CVehicleModelInfo::SetAtomicRendererCB_Train | cdecl | vehicle::set_atomic_renderer_cb_train | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7d20 | CVehicleModelInfo::GetWheelPosn | thiscall | vehicle::get_wheel_posn | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7dd0 | CVehicleModelInfo::GetOriginalCompPosition | thiscall | vehicle::get_original_comp_position | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8040 | CVehicleModelInfo::ChooseComponent | thiscall | vehicle::choose_component | bytes + gr + spec | VehicleModelInfo.cpp:344 keep-original ; VehicleModelInfo.cpp:348 keep-original ; VehicleModelInfo.cpp:1627 keep-original ; VehicleModelInfo.cpp:1632 keep-original ; VehicleModelInfo.cpp:1639 keep-original | implemented-needs-L2 | rng-helper rows :344/:348/:1627/:1632/:1639 (keep-original); rand caller site 0x4C80BC |
| 0x4c8120 | CVehicleModelInfo::ChooseSecondComponent | thiscall | vehicle::choose_second_component | bytes + gr + spec | VehicleModelInfo.cpp:363 keep-original ; VehicleModelInfo.cpp:367 keep-original | implemented-needs-L2 | :363/:367 (keep-original) |
| 0x4c8780 | CVehicleModelInfo::LoadEnvironmentMaps | cdecl | vehicle::load_environment_maps | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8b60 | CVehicleModelInfo::StoreAtomicUsedMaterialsCB | cdecl | vehicle::store_atomic_used_materials_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8e30 | CVehicleModelInfo::CollapseFramesCB | cdecl | vehicle::collapse_frames_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c9360 | CVehicleModelInfo::AssignRemapTxd | cdecl | vehicle::assign_remap_txd | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c9450 | CVehicleModelInfo::SetCarCustomPlate | thiscall | vehicle::set_car_custom_plate | bytes + gr | - | implemented-needs-L2 |  |
| 0x537990 | CFileLoader::LoadCarGenerator-0 | cdecl | fileloader::load_car_generator_struct | bytes + gr | - | implemented-needs-L2 |  |
| 0x538440 | CFileLoader::LoadCollisionFile-Buffer | cdecl | fileloader::load_collision_file_buffer | bytes + gr + spec | FileLoader.cpp:493 harmless ; FileLoader.cpp:511 unclear | implemented-needs-L2 | model-info register persists across entries; stops at <= 8 bytes left |
| 0x5b45d0 | CFileLoader::LoadStuntJump | cdecl | fileloader::load_stunt_jump | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b4ab0 | CFileLoader::LoadZone | cdecl | fileloader::load_zone | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b4b40 | CFileLoader::LoadCullZone | cdecl | fileloader::load_cull_zone | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b4e60 | CFileLoader::LoadCollisionFile-File | cdecl | fileloader::load_collision_file | bytes + gr + spec | FileLoader.cpp:553 keep-original | implemented-needs-L2 | *new*: a model without col model gets a new one attached, but the data is loaded into the PREVIOUS entry's col model (EDI not updated, 0x5B4F2C-0x5B4F5E); version only for COLL/2/3; first-entry version/target uninitialised (0/null here) |
| 0x5b5000 | CFileLoader::LoadCollisionFileFirstTime | cdecl | fileloader::load_collision_file_first_time | bytes + gr | FileLoader.cpp:577 harmless ; FileLoader.cpp:594 harmless | implemented-needs-L2 |  |
| 0x5b65a0 | CVehicleModelInfo::LoadVehicleUpgrades | cdecl | vehicle::load_vehicle_upgrades | bytes + gr | - | implemented-needs-L2 | state machine and strtok(" \t,") as in the bytes; GetModelInfo results not null-checked |
| 0x5b6890 | CVehicleModelInfo::LoadVehicleColours | cdecl | vehicle::load_vehicle_colours | bytes + gr + spec | VehicleModelInfo.cpp:1328 keep-original ; VehicleModelInfo.cpp:1330 keep-original | implemented-needs-L2 | :1328 no retry, r/g/b kept across lines (initial 0 here vs stack contents); protection chunk 0x405FB1 = ReadLine length 0x400 |
| 0x5b75e0 | CFileLoader::LoadTXDParent | cdecl | fileloader::load_txd_parent | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b7670 | CFileLoader::Load2dEffect | cdecl | fileloader::load_2d_effect | bytes + gr + spec | - | implemented-needs-L2 | spec CFileLoader.md (not reversed in gr); uninitialised locals of malformed lines are 0 |
| 0x4c7470 | CVehicleModelInfo::ShutdownLightTexture | cdecl | vehicle::shutdown_light_texture | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c7700 | CVehicleModelInfo::MoveObjectsCB | cdecl | vehicle::move_objects_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c87d0 | CVehicleModelInfo::ShutdownEnvironmentMaps | cdecl | vehicle::shutdown_environment_maps | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b3c60 | CFileLoader::LoadObject | cdecl | fileloader::load_object | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b3de0 | CFileLoader::LoadTimeObject | cdecl | fileloader::load_time_object | bytes + gr + spec | FileLoader.cpp:1777 unclear | implemented-needs-L2 | counts other than 1..3 keep the first parse (:1777) |
| 0x5b3fb0 | CFileLoader::LoadWeaponObject | cdecl | fileloader::load_weapon_object | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b4040 | CFileLoader::LoadClumpObject | cdecl | fileloader::load_clump_object | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b40c0 | CFileLoader::LoadAnimatedClumpObject | cdecl | fileloader::load_animated_clump_object | bytes + gr | - | implemented-needs-L2 | *new*: +0x13 bit 2 from flags & 0x20 (EDX = flags at the test); gr omits it |
| 0x5b4740 | CFileLoader::LoadCarGenerator-1 | cdecl | fileloader::load_car_generator_line | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b6f30 | CFileLoader::LoadVehicleObject | cdecl | fileloader::load_vehicle_object | bytes + gr + spec | FileLoader.cpp:1811 keep-original ; FileLoader.cpp:1845 harmless ; FileLoader.cpp:1846 harmless ; FileLoader.cpp:1854 keep-original ; FileLoader.cpp:1899 harmless ; FileLoader.cpp:1900 harmless | implemented-needs-L2 | wheel sizes not defaulted, f_heli=3, misses keep fields (:1811/1845/1846/1854/1899/1900); first game-name char never converted |
| 0x5b7420 | CFileLoader::LoadPedObject | cdecl | fileloader::load_ped_object | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8900 | CVehicleModelInfo::SetRenderPipelines | thiscall | vehicle::set_render_pipelines | bytes + gr | - | implemented-needs-L2 |  |
| 0x537150 | CFileLoader::SetRelatedModelInfoCB | cdecl | fileloader::set_related_model_info_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x538090 | CFileLoader::LoadObjectInstance-inst | - | - | bytes + gr | - | blocked | blocked: the heading path calls the CRT intrinsic `_CIacos` (0x822380) with its argument in ST0 (no `orig!` convention carries an ST0 argument, and sa-math has no CPU-checked acos model); port once an X87 acos exists or `orig!` gains an ST0-argument form |
| 0x5b3930 | CFileLoader::FindRelatedModelInfoCB | cdecl | fileloader::find_related_model_info_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b8030 | CFileLoader::LoadEntryExit | cdecl | fileloader::load_entry_exit | bytes + gr | FileLoader.cpp:1220 harmless | implemented-needs-L2 |  |
| 0x5b8400 | CFileLoader::LoadObjectTypes | cdecl | fileloader::load_object_types | bytes + gr + spec | FileLoader.cpp:2183 unclear ; FileLoader.cpp:2192 unclear ; FileLoader.cpp:2253 unclear ; FileLoader.cpp:2256 unclear | implemented-needs-L2 |  |
| 0x5d5db0 | CVehicleModelInfo::SetDirtTextures | cdecl | vehicle::set_dirt_textures | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8840 | CVehicleModelInfo::SetEnvironmentMapCB | cdecl | vehicle::set_environment_map_cb | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8bd0 | CVehicleModelInfo::ReduceMaterialsInVehicle | thiscall | vehicle::reduce_materials_in_vehicle | bytes + gr | - | implemented-needs-L2 |  |
| 0x538690 | CFileLoader::LoadObjectInstance-file | cdecl | fileloader::load_object_instance_line | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b3860 | CFileLoader::LoadTexDictionary | cdecl | fileloader::load_tex_dictionary | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c8e60 | CVehicleModelInfo::PreprocessHierarchy | thiscall | vehicle::preprocess_hierarchy | bytes + gr | - | implemented-needs-L2 | two passes as in the bytes |
| 0x5b51e0 | CFileLoader::LinkLods | cdecl | fileloader::link_lods | bytes + gr | - | implemented-needs-L2 | isCacheLoading via 0x5B2AC0 (protection thunk 0x40220F); product > 300 under PC=24 |
| 0x4c95c0 | CVehicleModelInfo::SetClump | thiscall | vehicle::set_clump | bytes + gr + spec | VehicleModelInfo.cpp:242 unclear | implemented-needs-L2 | :242 no dirt materials (keep-original) |
| 0x5371f0 | CFileLoader::LoadAtomicFile-stream | cdecl | fileloader::load_atomic_file_stream | bytes + gr + spec | FileLoader.cpp:172 keep-original | implemented-needs-L2 | no null check / no Stop on failure (:172) |
| 0x5372d0 | CFileLoader::LoadClumpFile-1 | cdecl | fileloader::load_clump_file_stream | bytes + gr | - | implemented-needs-L2 |  |
| 0x5373f0 | CFileLoader::StartLoadClumpFile | cdecl | fileloader::start_load_clump_file | bytes + gr | - | implemented-needs-L2 |  |
| 0x537450 | CFileLoader::FinishLoadClumpFile | cdecl | fileloader::finish_load_clump_file | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b39d0 | CFileLoader::LoadAtomicFile- | cdecl | fileloader::load_atomic_file | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b3a30 | CFileLoader::LoadClumpFile- | cdecl | fileloader::load_clump_file | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c9680 | CVehicleModelInfo::CreateInstance | thiscall | vehicle::create_instance | bytes + gr | - | implemented-needs-L2 |  |
| 0x4c9890 | CVehicleModelInfo::DeleteRwObject | thiscall | vehicle::delete_rw_object | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b8f00 | CVehicleModelInfo::SetupCommonData | cdecl | vehicle::setup_common_data | bytes + gr | VehicleModelInfo.cpp:1268 harmless | implemented-needs-L2 |  |
| 0x5b3680 | LoadingScreenLoadingFile | cdecl | fileloader::loading_screen_loading_file | bytes + spec | - | implemented-needs-L2 |  |
| 0x5b47b0 | CFileLoader::LoadPickup | cdecl | fileloader::load_pickup | bytes + gr | - | implemented-needs-L2 |  |
| 0x5b8700 | CFileLoader::LoadScene | cdecl | fileloader::load_scene | bytes + gr + spec | FileLoader.cpp:2034 unclear | implemented-needs-L2 | *new*: "path" header sets state 1; the next non-"end" line stops reading the file (path handler state 7 unreachable) |
| 0x5b9030 | CFileLoader::LoadLevel | cdecl | fileloader::load_level | bytes + gr + spec | FileLoader.cpp:1304 unclear ; FileLoader.cpp:1312 keep-original ; FileLoader.cpp:1358 unclear ; FileLoader.cpp:1368 unclear ; FileLoader.cpp:1373 keep-original | implemented-needs-L2 | IMG skips loading text, GTA_INT.IMG 19-byte compare, COLFILE line+10, SPLASH ignored |

### Owner M: CdStream, CIplStore, CTxdStore, CLoadedCarGroup, CLoadingScreen, RW Gta stream readers, pools and leaves (`cdstream.rs`, `iplstore.rs`, `txdstore.rs`, `loaded_car_group.rs`, `loading_screen.rs`, `rw_gta_stream.rs`, `misc.rs`; specs `specs_stream_m.rs`)

| address | name | conv | port (sa-stream) | source | deviations followed (original kept); *new* = not in gr-deviations.csv | state | notes (L2 signature / reason) |
|---|---|---|---|---|---|---|---|
| 0x406370 | CdStream::CdStreamShutdown | cdecl | `cdstream::cd_stream_shutdown` | bytes + gr | none | implemented-needs-L2 | Void; LocalFree/CloseHandle through the IAT cells |
| 0x4063E0 | CdStream::CdStreamGetStatus | cdecl (stub → .HOODLUM 0x1563850) | `cdstream::cd_stream_get_status` | bytes + gr | alertable flag of WaitForSingleObjectEx decoded from protection constants (= 1) | implemented-needs-L2 | EAX; reads the stream array |
| 0x406460 | CdStream::CdStreamSync | cdecl (stub → .HOODLUM 0x156CD80) | `cdstream::cd_stream_sync` | bytes + spec | `CdStreamInfo.cpp:109/:115/:119` keep-original (no lock); **divergence (ledger, INF-15)**: the INFINITE wait is a 20 ms polled wait that ends when the thread is done (no lost wake-up); GetOverlappedResult reached through the protection thunk 0x1141980 (decoded: IAT 0x858084), success only for a result of exactly 1 | implemented-needs-L2 | EAX status |
| 0x406560 | CdStream::CdStreamThread | stdcall (thread entry) | `cdstream::cd_stream_thread` | bytes + spec | `:235` keep-original; *new*: ReadFile result must be exactly 1 in the overlapped branch; empty queue gives stream -1 | implemented-needs-L2 | never returns: the hook applies when the thread starts; runs on the streaming thread (ADR-001 single-thread exception, as the original) |
| 0x406690 | CdStream::CdStreamRemoveImages | cdecl | `cdstream::cd_stream_remove_images` | bytes + gr | none | implemented-needs-L2 | Void |
| 0x4067B0 | CdStream::CdStreamOpen | cdecl (stub → .HOODLUM 0x1564A90) | `cdstream::cd_stream_open` (no hook) | bytes | *new*: `strcpy` of the name (gr `strncpy_s`); CreateFileA through the protection thunk (decoded: IAT 0x8580A0) | blocked | when the first free handle slot is 7/15/23/31 the slot index comes from the protection VM (0xCB73E0 → [0x1422EA0], an expression-tree interpreter run on the stack); not decoded, the port panics there. Needs a measurement with 8 open images |
| 0x4068F0 | CdStream::CdStreamInitThread | cdecl | `cdstream::cd_stream_init_thread` | bytes + gr | none | implemented-needs-L2 | Void; CreateSemaphoreA/CreateThread/LocalAlloc |
| 0x406A20 | CdStream::CdStreamRead | cdecl (stub → .HOODLUM 0x156C2C0) | `cdstream::cd_stream_read` (no hook) | bytes + spec | `:164` keep-original (`gLastCdStreamPosn = sectors + pos` with the file id byte); queue tail address decoded ([0x1560578]^[0x527D09] = 0x8E3FF4); ReadFile/SetFilePointer/ReleaseSemaphore through the protection thunk (decoded cells); synchronous path returns ReadFile's BOOL unchanged; *not reproduced*: the `printf` of a VM-computed string when ReleaseSemaphore fails (stdout only) | deferred | 0x406A20 is sa-hook's own determinism-mode hook (`Id::CdStreamRead`); two hooks cannot share the entry. sa-hook can call the port as its "original" later |
| 0x406B70 | CdStream::CdStreamInit | cdecl | `cdstream::cd_stream_init` | bytes + spec | `:286` keep-original (NO_BUFFERING when bytesPerSector <= 2048); bytesPerSector uninitialised when GetDiskFreeSpaceA fails (port: 0) | implemented-needs-L2 | Void; runs once at start-up (from CGame::InitialiseOnceBeforeRW) |
| 0x4045B0 | CIplStore::AddIplsNeededAtPosn | cdecl (stub) | `iplstore::add_ipls_needed_at_posn` | bytes + gr | none | implemented-needs-L2 | Void; writes 0x8E3FD0..DC, 0x8E3FA8 |
| 0x4045E0 | CIplStore::ClearIplsNeededAtPosn | cdecl (stub) | `iplstore::clear_ipls_needed_at_posn` | bytes + gr | none | implemented-needs-L2 | Void |
| 0x4045F0 | CIplStore::SetIfInteriorIplIsRequired | cdecl | `iplstore::set_if_interior_ipl_is_required` | bytes + gr | inline IsPointInside with the original's flag tests (NaN passes) | implemented-needs-L2 | quad-tree callback; reads 0x8E3EF8 |
| 0x404660 | CIplStore::SetIfIplIsRequired | cdecl | `iplstore::set_if_ipl_is_required` | bytes + gr | none | implemented-needs-L2 | calls CRect::IsPointInside(-140) |
| 0x404690 | CIplStore::SetIfIplIsRequiredReducedBB | cdecl | `iplstore::set_if_ipl_is_required_reduced_bb` | bytes + gr | x87 `bound - (±160)` compares with the original's flag tests | implemented-needs-L2 | reads 0x8E3EF8 |
| 0x404700 | CIplStore::SetIplsRequired | cdecl | `iplstore::set_ipls_required` | bytes + gr | callback chosen with `fcomp 900; test ah,1` (NaN → not the interior callback) | implemented-needs-L2 | FindPlayerPed, ForAllMatching with a local CVector2D |
| 0x404780 | CIplStore::GetNewIplEntityIndexArray | cdecl (stub) | `iplstore::get_new_ipl_entity_index_array` | bytes + gr | `operator new[]` before the count is read | implemented-needs-L2 | EAX |
| 0x4047B0 | CIplStore::GetIplEntityIndexArray | cdecl (stub) | `iplstore::get_ipl_entity_index_array` | bytes + gr | none | implemented-needs-L2 | EAX; global array read |
| 0x404A60 | CIplStore::GetIplName | cdecl | `iplstore::get_ipl_name` | bytes + gr | *new*: a free slot returns the address 0x10 | implemented-needs-L2 | EAX |
| 0x404A90 | CIplStore::SetIsInterior | cdecl | `iplstore::set_is_interior` | bytes + gr | *new*: no null check (free slot writes to 0x2C) | implemented-needs-L2 | Void |
| 0x404AC0 | CIplStore::FindIplSlot | cdecl | `iplstore::find_ipl_slot` | bytes + spec | `IplStore.cpp:197` harmless | implemented-needs-L2 | EAX; `_stricmp` |
| 0x404B20 | CIplStore::RemoveIpl | cdecl | `iplstore::remove_ipl` | bytes + spec | `:608` harmless; inclusive `last` loops, tail call RemoveCarGenerators(int32) (spec) | implemented-needs-L2 | Void; entity pools |
| 0x404C70 | CIplStore::GetBoundingBox | cdecl | `iplstore::get_bounding_box` | bytes + gr | none (0 for a free slot) | implemented-needs-L2 | EAX |
| 0x404C90 | CIplStore::IncludeEntity | cdecl (stub) | `iplstore::include_entity` | bytes + spec | `:281` keep-original (other types: no-op); int32 index vs sign-extended bounds; dummy pool through a protection chunk (decoded: 0xB744A0) | implemented-needs-L2 | Void |
| 0x404D30 | CIplStore::EnableDynamicStreaming | cdecl | `iplstore::enable_dynamic_streaming` | bytes + gr | *new*: no null check | implemented-needs-L2 | Void |
| 0x404DE0 | CIplStore::SetupRelatedIpls | cdecl | `iplstore::setup_related_ipls` | bytes + spec | `:767/:768/:779/:785` harmless; 32-byte name buffer followed by the getIplDef temporary (one buffer in the port; beyond it original UB → panic) | implemented-needs-L2 | EAX = count |
| 0x405110 | CIplStore::RemoveRelatedIpls | cdecl | `iplstore::remove_related_ipls` | bytes + spec | `:653` harmless | implemented-needs-L2 | Void |
| 0x405170 | CIplStore::LoadIpls | cdecl (entry `jmp` into chunk 0x403803) | `iplstore::load_ipls` | bytes + gr | *new*: slots walked 1..255 upwards (gr downwards); velocity clamp `min(40, v*80)` with x87 compare semantics; mission entities through inlined GetAtRef | implemented-needs-L2 | 4 stack words (CVector by value, bool) |
| 0x4053F0 | CIplStore::EnsureIplsAreInMemory | cdecl | `iplstore::ensure_ipls_are_in_memory` | bytes + gr | *new*: loadRequested cleared for every requested slot (gr only when it loads) | implemented-needs-L2 | Void |
| 0x405520 | CIplStore::RequestIpls | cdecl | `iplstore::request_ipls` | bytes + gr | *new*: calls SetIplsRequired(posn, arg2) first | implemented-needs-L2 | Void |
| 0x405600 | CIplStore::HaveIplsLoaded | cdecl | `iplstore::have_ipls_loaded` | bytes + gr | *new*: calls SetIplsRequired(posn, arg2) first; early return after clearing the remaining slots | implemented-needs-L2 | AL |
| 0x405720 | CIplStore::RemoveAllIpls | cdecl | `iplstore::remove_all_ipls` | bytes + gr | none (flags & 6 test on ms_aInfoForModel) | implemented-needs-L2 | Void |
| 0x405780 | CIplStore::LoadAllRemainingIpls | cdecl | `iplstore::load_all_remaining_ipls` | bytes + gr | none | implemented-needs-L2 | Void |
| 0x405850 | CIplStore::RequestIplAndIgnore | cdecl (stub) | `iplstore::request_ipl_and_ignore` | bytes + gr | *new*: no null check (writes after RequestModel) | implemented-needs-L2 | Void |
| 0x405890 | CIplStore::RemoveIplAndIgnore | cdecl (stub) | `iplstore::remove_ipl_and_ignore` | bytes + gr | *new*: no null check | implemented-needs-L2 | Void |
| 0x4058D0 | CIplStore::RemoveIplWhenFarAway | cdecl (stub) | `iplstore::remove_ipl_when_far_away` | bytes + gr | *new*: no null check | implemented-needs-L2 | Void |
| 0x405AC0 | CIplStore::AddIplSlot | cdecl (stub) | `iplstore::add_ipl_slot` | bytes + gr | *new*: unbounded `strcpy` of the name into name[18]; ±1e6 inverted box | implemented-needs-L2 | EAX |
| 0x405B60 | CIplStore::RemoveIplSlot | cdecl (stub) | `iplstore::remove_ipl_slot` | bytes + gr | inlined pool delete (map \|= 0x80, first_free) | implemented-needs-L2 | Void |
| 0x405C00 | CIplStore::LoadIplBoundingBox | cdecl | `iplstore::load_ipl_bounding_box` | bytes + spec | `:416/:431` harmless; in-place relocation of the instance offset; inline restrict on strict compares | implemented-needs-L2 | AL = 1 |
| 0x405EC0 | CIplStore::Initialise | cdecl | `iplstore::initialise` | bytes + gr | `:37` harmless | implemented-needs-L2 | Void |
| 0x405FA0 | CIplStore::Shutdown | cdecl (stub) | `iplstore::shutdown` | bytes + spec | `:45` harmless; fixed 256-slot loop; first `delete[]` through an obfuscated call (decoded 0x8213AE) | implemented-needs-L2 | Void |
| 0x406080 | CIplStore::LoadIpl | cdecl | `iplstore::load_ipl` | bytes + spec | `:319/:360` harmless; `loaded = 1` on every path; in-place relocation of instance and car-generator offsets | implemented-needs-L2 | AL |
| 0x5D5420 | CIplStore::Save | cdecl | `iplstore::save` | bytes + gr | *new*: writes the constant 256 as the count | implemented-needs-L2 | AL = 1 |
| 0x5D54A0 | CIplStore::Load | cdecl | `iplstore::load` | bytes + gr | *new*: the stored count is ignored, fixed 1..255 (`:814` harmless) | implemented-needs-L2 | AL = 1 |
| 0x408340 | CTxdStore::GetTxd | cdecl (stub) | `txdstore::get_txd` | bytes + gr | *new*: no null check (free slot → access violation) | implemented-needs-L2 | EAX |
| 0x408370 | CTxdStore::GetParentTxdSlot | cdecl (stub) | `txdstore::get_parent_txd_slot` | bytes + gr | *new*: no null check | implemented-needs-L2 | EAX (movsx) |
| 0x731680 | CTxdStore::RemoveIfRefCountIsGreaterThanOne | cdecl | `txdstore::remove_if_ref_count_is_greater_than_one` | bytes + gr | none | implemented-needs-L2 | EAX; RW callback |
| 0x7316A0 | CTxdStore::PushCurrentTxd | cdecl | `txdstore::push_current_txd` | bytes + gr | store through the protection chunk 0x401B78 (address computed at run time: 0xC88010) | implemented-needs-L2 | Void; entry is a `call` (kept-return trampoline) |
| 0x7316B0 | CTxdStore::PopCurrentTxd | cdecl | `txdstore::pop_current_txd` | bytes + gr | none | implemented-needs-L2 | Void |
| 0x731710 | CTxdStore::TxdStoreLoadCB | cdecl | `txdstore::txd_store_load_cb` (no hook) | bytes + gr | none (returns null) | ported-unhookable | `xor eax,eax; ret` (3 bytes) |
| 0x731720 | CTxdStore::TxdStoreFindCB | cdecl | `txdstore::txd_store_find_cb` | bytes + gr | none | implemented-needs-L2 | EAX |
| 0x731850 | CTxdStore::FindTxdSlot-name | cdecl | `txdstore::find_txd_slot_name` | bytes + spec | `TxdStore.cpp:177` (attributed to the hash overload) harmless | implemented-needs-L2 | EAX; writes 0xC88014 on a hit |
| 0x7318E0 | CTxdStore::FindTxdSlot-hash | cdecl | `txdstore::find_txd_slot_hash` | bytes + spec | none | implemented-needs-L2 | EAX |
| 0x731930 | CTxdStore::StartLoadTxd | cdecl | `txdstore::start_load_txd` | bytes + gr | *new*: no null check after a found chunk | implemented-needs-L2 | AL |
| 0x731990 | CTxdStore::Create | cdecl | `txdstore::create` | bytes + gr | *new*: no null check (RwTexDictionaryCreate is called first) | implemented-needs-L2 | Void |
| 0x7319C0 | CTxdStore::SetCurrentTxd | cdecl | `txdstore::set_current_txd` | bytes + gr | *new*: no null check | implemented-needs-L2 | Void |
| 0x731A00 | CTxdStore::AddRef | cdecl | `txdstore::add_ref` | bytes + gr | *new*: no null check | implemented-needs-L2 | Void |
| 0x731A30 | CTxdStore::RemoveRef | cdecl | `txdstore::remove_ref` | bytes + gr | *new*: no null check; `int16 <= 0` | implemented-needs-L2 | Void; tail call CStreaming::RemoveModel |
| 0x731A70 | CTxdStore::RemoveRefWithoutDelete | cdecl | `txdstore::remove_ref_without_delete` | bytes + gr | *new*: no null check | implemented-needs-L2 | Void |
| 0x731AA0 | CTxdStore::GetNumRefs | cdecl | `txdstore::get_num_refs` | bytes + gr | *new*: count sign-extended (gr zero-extends uint16); no null check | implemented-needs-L2 | EAX |
| 0x731C80 | CTxdStore::AddTxdSlot | cdecl | `txdstore::add_txd_slot` | bytes + gr | none | implemented-needs-L2 | EAX |
| 0x731CD0 | CTxdStore::RemoveTxdSlot | cdecl | `txdstore::remove_txd_slot` | bytes + gr | inlined pool delete | implemented-needs-L2 | Void |
| 0x731D50 | CTxdStore::SetupTxdParent | cdecl | `txdstore::setup_txd_parent` | bytes + gr | parent AddRef inlined; no null checks | implemented-needs-L2 | Void |
| 0x731DD0 | CTxdStore::LoadTxd-stream | cdecl | `txdstore::load_txd_stream` | bytes + gr | none | implemented-needs-L2 | AL |
| 0x731E40 | CTxdStore::FinishLoadTxd | cdecl | `txdstore::finish_load_txd` | bytes + gr | *new*: no null check | implemented-needs-L2 | AL |
| 0x731E90 | CTxdStore::RemoveTxd | cdecl | `txdstore::remove_txd` | bytes + gr | parent RemoveRef inlined | implemented-needs-L2 | Void |
| 0x731F20 | CTxdStore::Initialise | cdecl | `txdstore::initialise` | bytes + gr | AddTxdSlot inlined four times | implemented-needs-L2 | Void |
| 0x732000 | CTxdStore::Shutdown | cdecl | `txdstore::shutdown` | bytes + gr | arrays and pool to `free`; ms_pTxdPool kept | implemented-needs-L2 | Void |
| 0x732060 | CTxdStore::GameShutdown | cdecl | `txdstore::game_shutdown` | bytes + gr | none | implemented-needs-L2 | Void |
| 0x7320B0 | CTxdStore::LoadTxd-filename | cdecl (entry chunk 0x402161) | `txdstore::load_txd_filename` | bytes + gr | `sprintf("%s")` into 260 bytes (no limit, original UB beyond) | implemented-needs-L2 | AL |
| 0x611B90 | CLoadedCarGroup::Clear | thiscall | `loaded_car_group::clear` | bytes | none | L1 (1,002,000 cases, 0 mismatches) | |
| 0x611BB0 | CLoadedCarGroup::AddMember | thiscall, ret 4 | `loaded_car_group::add_member` (no hook) | bytes | stores the low word of the argument | ported-unhookable; L1 (1,002,000, 0) | `jl 0x611BB2` branches into the entry bytes |
| 0x611BD0 | CLoadedCarGroup::RemoveMember | thiscall, ret 4 | `loaded_car_group::remove_member` | bytes + spec | `LoadedCarGroup.cpp:38` harmless | L1 (1,002,000, 0) | |
| 0x611C20 | CLoadedCarGroup::GetMember | thiscall, ret 4 | `loaded_car_group::get_member` | bytes | `:139` harmless; no bounds check | L1 (1,002,000, 0) | |
| 0x611C30 | CLoadedCarGroup::CountMembers | thiscall | `loaded_car_group::count_members` (no hook) | bytes | none | ported-unhookable; L1 (1,002,000, 0) | `jl 0x611C34` branches into the entry bytes |
| 0x611C50 | CLoadedCarGroup::PickRandomCar | thiscall, ret 8 | `loaded_car_group::pick_random_car` | bytes + spec | `:54/:70/:71/:89/:104` harmless; `:61` keep-original (GetRandomNumberInRange<int> formula); spec: sum == 0 → -1 without rand() | implemented-needs-L2 | EAX; rand() at site 0x611D35; a walk past the list (negative m_nFrq) reads the stack in the original, -1 in the port (original UB) |
| 0x611E10 | CLoadedCarGroup::SortBasedOnUsage | thiscall | `loaded_car_group::sort_based_on_usage` | bytes | *new*: bubble sort, swap only when strictly lower (signed bytes); gr's rng::sort may order equal keys differently | implemented-needs-L2 | Void |
| 0x611E90 | CLoadedCarGroup::PickLeastUsedModel | thiscall, ret 4 | `loaded_car_group::pick_least_used_model` | bytes | none | implemented-needs-L2 | EAX |
| 0x53DED0 | CLoadingScreen::LoadingScreen | cdecl | `loading_screen::loading_screen` | bytes + gr | none (no FastLoader options) | implemented-needs-L2 | Void; tail call NewChunkLoaded |
| 0x58FF10 | CLoadingScreen::Shutdown | cdecl | `loading_screen::shutdown` | bytes + spec | TXD slot "loadscs" (gr "loadsc0") | implemented-needs-L2 | Void |
| 0x58FF60 | CLoadingScreen::RenderSplash | cdecl | `loading_screen::render_splash` | bytes + gr | `:76` harmless; splash -1 = 0xBAB358 read as a sprite when cross-fading from splash 0 | implemented-needs-L2 | Void; RW/sprite calls |
| 0x5900B0 | CLoadingScreen::LoadSplashes | cdecl | `loading_screen::load_splashes` | bytes + spec | `:127` keep-original (14 rand() calls, loadsc1..14), `:142` keep-original, `:112/:144/:146` harmless; slot "loadscs" | implemented-needs-L2 | Void; srand(QPC.LowPart) |
| 0x590220 | CLoadingScreen::DisplayMessage | cdecl | `loading_screen::display_message` | bytes + gr | unbounded strcpy into 16 bytes | implemented-needs-L2 | Void |
| 0x590240 | CLoadingScreen::SetLoadingBarMsg | cdecl | `loading_screen::set_loading_bar_msg` | bytes + gr | none | implemented-needs-L2 | Void |
| 0x590280 | CLoadingScreen::GetClockTime | cdecl, ST0 | `loading_screen::get_clock_time` (returns X87) | bytes | *new*: `* 0.001f` (gr `/ 1000.0f`), result kept in ST0 | implemented-needs-L2 | ST0; RsTimer |
| 0x5902B0 | CLoadingScreen::Init | cdecl | `loading_screen::init` | bytes + gr | first argument unused (as gr) | implemented-needs-L2 | Void |
| 0x590310 | CLoadingScreen::Pause | cdecl | `loading_screen::pause` (no hook) | bytes | none | ported-unhookable | lone `ret` |
| 0x590320 | CLoadingScreen::Continue | cdecl | `loading_screen::continue_` | bytes + gr | x87 chain `clock - onPause + pause` | implemented-needs-L2 | Void |
| 0x590370 | CLoadingScreen::RenderLoadingBar | cdecl | `loading_screen::render_loading_bar` | bytes + spec | `:208/:209` harmless; m_TimeBarAppeared only after drawing and only while 0 (spec, keep-original) | implemented-needs-L2 | Void |
| 0x5904D0 | CLoadingScreen::DisplayNextSplash | cdecl | `loading_screen::display_next_splash` | bytes + gr | splash index incremented even when BeginUpdate fails (as gr) | implemented-needs-L2 | Void |
| 0x590530 | CLoadingScreen::StartFading | cdecl | `loading_screen::start_fading` | bytes + gr | none | implemented-needs-L2 | Void |
| 0x590570 | CLoadingScreen::DisplayPCScreen | cdecl | `loading_screen::display_pc_screen` | bytes + gr | no FastLoader NoLoadBar option | implemented-needs-L2 | Void |
| 0x5905E0 | CLoadingScreen::Update | cdecl | `loading_screen::update` | bytes | *new*: fade duration 2.0 s only for splash >= 0 and (legal or fade-in); percent / legal timeout / next-splash formulas of the bytes (module docs) | implemented-needs-L2 | Void; ServiceLoadingTune |
| 0x590860 | CLoadingScreen::DoPCTitleFadeIn | cdecl (entry chunk 0x406465) | `loading_screen::do_pc_title_fade_in` | bytes + gr | DisplayPCScreen inlined | implemented-needs-L2 | Void |
| 0x590990 | CLoadingScreen::DoPCTitleFadeOut | cdecl | `loading_screen::do_pc_title_fade_out` | bytes + gr | none | implemented-needs-L2 | Void |
| 0x590AC0 | CLoadingScreen::DoPCScreenChange | cdecl | `loading_screen::do_pc_screen_change` | bytes + spec | `:334` keep-original (no wrap), `:327` harmless; tune amplitude (spec); inlined Shutdown | implemented-needs-L2 | Void |
| 0x590D00 | CLoadingScreen::NewChunkLoaded | cdecl | `loading_screen::new_chunk_loaded` | bytes + spec | `:395` keep-original, `:373` harmless | implemented-needs-L2 | Void |
| 0x744DB5 | CLoadingScreen::IsActive | cdecl | `loading_screen::is_active` (no hook) | bytes | the 1.0 US body is a lone `ret` (EAX unchanged; catalogue name doubtful) | ported-unhookable | lone `ret` |
| 0x84F2B0 | __init_g_bab358 | cdecl | `loading_screen::init_g_bab358` | spec | none | L1 (1,002,000, 0) | CRT initialiser (runs before sa-hook attaches; the store operand patched to scratch in L1) |
| 0x84F2D0 | __init_CLoadingScreen__m_aSplashes | cdecl | `loading_screen::init_splashes` | spec | none | implemented-needs-L2 | CRT initialiser: runs before sa-hook attaches (needs a start-up record) |
| 0x8565E0 | __dtor_CLoadingScreen__m_aSplashes | cdecl | `loading_screen::dtor_splashes` | spec | none | implemented-needs-L2 | `_atexit` callback |
| 0x72E140 | (missed by ghidra: cat7-call) = GeometryListStreamRead1 | stream on stack, list in EBX | `rw_gta_stream::geometry_list_stream_read1` (no hook) | spec | none | ported-unhookable | register argument (EBX) |
| 0x72E220 | FUN_0072e220 = GeometryListStreamRead2 | EBX = stream, EDI = list | `rw_gta_stream::geometry_list_stream_read2` (no hook) | spec | none | ported-unhookable | register arguments |
| 0x72E570 | RpClumpGtaStreamRead1 | cdecl, AL | `rw_gta_stream::rp_clump_gta_stream_read1` | spec + bytes | none | implemented-needs-L2 | AL |
| 0x72E620 | RpClumpGtaStreamRead2 | cdecl | `rw_gta_stream::rp_clump_gta_stream_read2` | spec + bytes | helpers 0x72E270/0x72E360 (UNK-game, register ABI) ported as private fns | implemented-needs-L2 | EAX |
| 0x730E60 | RwTextureGtaStreamRead | cdecl | `rw_gta_stream::rw_texture_gta_stream_read` | spec + bytes | load-time average in x87 PC=24 (unsigned conversions) | implemented-needs-L2 | EAX; native texture read through RwEngineInstance+0xB0 |
| 0x730FC0 | RwTexDictionaryGtaStreamRead | cdecl | `rw_gta_stream::rw_tex_dictionary_gta_stream_read` | spec + bytes | struct chunk > 12 bytes smashes the original's stack (port panics) | implemented-needs-L2 | EAX |
| 0x731070 | RwTexDictionaryGtaStreamRead1 | cdecl | `rw_gta_stream::rw_tex_dictionary_gta_stream_read1` | spec + bytes | as above | implemented-needs-L2 | EAX |
| 0x731150 | RwTexDictionaryGtaStreamRead2 | cdecl | `rw_gta_stream::rw_tex_dictionary_gta_stream_read2` | spec + bytes | ends with numberTextures = -1 | implemented-needs-L2 | EAX |
| 0x410F80 | thunk_FUN_015611f0 = CPool<ColDef>::CPool | thiscall, ret 8 (stub) | `misc::col_pool_ctor` | spec | sa-core pool ctor semantics (no fill, +0x11 untouched) | L1 (1,000,015, 0) | allocator stubbed in L1 (sa-core ctor spec builder) |
| 0x44D2C0 | FUN_0044d2c0 = CPathFind::StoreNodeInfoCar | thiscall, ret 0x44 | `misc::store_node_info_car` (no hook) | spec | none (empty) | ported-unhookable | `ret 0x44` (3 bytes) |
| 0x44D2F0 | FUN_0044d2f0 = CPathFind::StoreDetachedNodeInfoCar | thiscall, ret 0x44 | `misc::store_detached_node_info_car` (no hook) | spec | none (empty) | ported-unhookable | `ret 0x44` (3 bytes) |
| 0x5326E0 | CDummy::operator new | cdecl | `misc::dummy_operator_new` | spec | none | L1 (1,000,148, 0) | pool global patched to scratch (sa-core op-new spec builder) |
| 0x537510 | FUN_00537510 = CCollisionData::GetNoOfShadowVertices | thiscall | `misc::get_no_of_shadow_vertices` | spec | a count negative as int32 returns 1 | implemented-needs-L2 | EAX; follows the triangle pointer (no L1 with host-side pointers) |
| 0x537A10 | CAnimatedBuilding | thiscall | `misc::animated_building_ctor` | spec | none | implemented-needs-L2 | EAX = this; calls CBuilding::CBuilding |
| 0x59EA00 | CDummyObject::CDummyObject | thiscall | `misc::dummy_object_ctor` | spec | none | implemented-needs-L2 | EAX = this; calls CDummy::CDummy |
| 0x5B8340 | FUN_005b8340 = CPool<CVehicleStructure>::CPool | thiscall, ret 8 | `misc::vehicle_structure_pool_ctor` | spec | as the other pool ctors | L1 (1,000,015, 0) | |
| 0x731AD0 | FUN_00731ad0 = CPool<TxdDef>::CPool | thiscall, ret 8 | `misc::txd_pool_ctor` | spec | as the other pool ctors | L1 (1,000,015, 0) | |
| 0x731B80 | FUN_00731b80 = CPool<TxdDef>::New | thiscall | `misc::txd_pool_new` | spec | search from first_free + 1 (sa-core pool algorithm) | L1 (1,000,148, 0) | |

## Deviations found while porting (not yet in `catalog/gr-deviations-notes.csv`)

All keep-original (the ports follow the bytes). Candidates for CAT-14 notes. The rows above give the
details; the most important ones:

- CdStream: `CdStreamOpen` copies the name with an unbounded `strcpy` (gr `strncpy_s`); `CdStreamThread`
  needs a ReadFile result of exactly 1 in the overlapped branch; GetOverlappedResult success only for 1.
- CTxdStore: `GetTxd`, `GetParentTxdSlot`, `SetCurrentTxd`, `AddRef`, `RemoveRef`,
  `RemoveRefWithoutDelete`, `GetNumRefs`, `Create`, `FinishLoadTxd`, `RemoveTxd`, `SetupTxdParent`,
  `RemoveTxdSlot`, `StartLoadTxd`/`LoadTxd` (after a found chunk) are not null-checked; `GetNumRefs`
  sign-extends the count.
- CIplStore: `RequestIpls`/`HaveIplsLoaded` call `SetIplsRequired` first; `EnsureIplsAreInMemory` clears
  `loadRequested` of every requested slot; `LoadIpls` walks 1..255 upwards; `Save` writes 256, `Load`
  ignores the stored count; `GetIplName` of a free slot returns 0x10; `AddIplSlot` uses an unbounded
  `strcpy`; several setters are not null-checked.
- CLoadedCarGroup: `SortBasedOnUsage` is a bubble sort (equal keys keep their order).
- CLoadingScreen: `GetClockTime` multiplies by 0.001f and returns ST0; `Update`'s fade duration 2.0 s
  only for splash >= 0 and (legal screen or fade-in), and its percent / legal-timeout / next-splash
  formulas (see `loading_screen.rs`).
- CStreaming/CColStore (owner A): `CStreamingInfo::Init` keeps `m_Flags`; `CColStore::AddColSlot` never
  copies the name; `StreamOneNewCar` (first active traffic cheat wins, cabbie after a taxi);
  `StreamZoneModels_Gangs` and `AddToLoadedVehiclesList` index car groups 0..9 by gang and use group 11
  for boats; `FindMIPedSlotForInterior` returns 7 without a loaded ped; `FinishLoadingLargeFile` success
  value for non-DFF/TXD ids is the id's low byte; `ConvertBufferToObject` takes two arguments.
- Model infos (owner B): `SetupVehicleUpgradeFlags` clears the upper flags byte and uses the prefix
  "wg_r_"; `FindOtherTimeModel` takes the first model with a time info and the key (unbounded name copy);
  `ReInit2dEffects` skips null entries; `Get2dEffect` re-calls `GetRwModelType`; `GetModelInfo` writes
  `*index` first; `ShutDown` compares store counts signed.
- CFileLoader/CVehicleModelInfo (owner C): `LoadCollisionFile(filename)` loads into the previous entry's
  col model when a new one is created (EDI not updated); `LoadScene`'s "path" header sets state 1;
  `LoadAudioZone` passes `flags != 0`; `LoadAnimatedClumpObject` flag 0x20 → bit 2 of +0x13;
  `LoadVehicleObject` never converts the game name's first '_'; the passenger count reads the handling id
  as a byte; `StartLoadClumpFile` queries the model type before the chunk search; `ConvertAnimFileIndex`
  frees with `operator delete`.
- sa-core (found here, not changed): `OLD_KEY_STATE` (0xB72F20) overlaps `PAD0_OLD_CROSS` (0xB73138).

## Open points

- Blocked: `CdStreamOpen` 0x4067B0 (slot index 7/15/23/31 from the protection VM — needs a measurement
  with 8 open images); `CFileLoader::LoadObjectInstance(CFileObjectInstance*)` 0x538090 (`_CIacos` with an
  ST0 argument: no `orig!` form and no CPU-checked acos model).
- Deferred: `CdStreamRead` 0x406A20 (sa-hook's determinism hook owns the entry; the port can become its
  "original"); `CStreaming::AddModelsToRequestList` 0x40D3F0, `DeleteRwObjectsBehindCamera` 0x40D7C0,
  `StreamZoneModels` 0x40A560 (large x87 bodies, need a byte-level pass with CPU-checked x87);
  `CFileLoader::LoadPedPathNode` 0x5B41F0 / `LoadCarPathNode` 0x5B4380 (x87 argument derivation behind
  protection thunk 0x4022CD; the callees are a lone `ret`).
- Values the protection VM produces at run time that owner A took from gta-reversed or the surrounding
  code (L2 must confirm): StreamCopModels bike index 4, StreamFireEngineAndFireman request tail, the
  inappropriate-cars group 0xC0E9C8, IsCarModelNeededInCurrentZone's second loop start 18, AddColSlot's
  "stadint" length 7.
- Uninitialised locals the ports pass as 0 (L2 may show them): CdStreamInit `bytesPerSector` when
  GetDiskFreeSpaceA fails, RequestModelStream position/size, LoadCollisionFile version/target on the first
  entry, LoadVehicleObject wheel sizes, LoadTimeObject fields, LoadVehicleColours r/g/b, Load2dEffect
  per-type locals.
- `CPedModelInfo::m_pColNodeInfos` is declared from 0x8A630C as 83 dwords (owner B read the values from
  the exe; gta-reversed places the array at 0x8A6308); its last pad dword is
  `CVehicleModelInfo::ms_compsToUse` 0x8A6458.
- Foreign globals still declared in owner modules (no overlap with sa-core today): CWeather
  `NEW_WEATHER_TYPE`/`OLD_WEATHER_TYPE`, `G_STRING` 0xB71670, `PC_SCRATCH` 0xC8E0C8, CGame working
  matrices 0xB72920/0xB72924, the RW 2d-effect plugin offset 0xC3A1E0, `TheCamera.m_fLODDistMultiplier`
  0xB6F118 (a field of TheCamera: clashes once TheCamera is declared whole). Re-home them when their
  owners are ported.
- Cross-owner `orig!` targets still `unported` (signature adapters needed for phase B): the 45 listed by
  `rewire.py` (model-info calls taking `MiPtr`, `bool` vs `u8` returns, typed `CLoadedCarGroup`
  pointers) and calls into UNK-game helpers (store ctors/dtors 0x4C5CB0–0x4C67D0, 0x4C5840–0x4C6390,
  `CPool<CVehicleStructure>::New` 0x4C94C0, `CdStreamGetLastPosn` 0x406450, `GetGTA3ImgSize` 0x406360).


## STREAM-L2 (in-game L2 of the 434 `implemented` rows; agent results, states set by the main session)

Runs: lane 2, `tools/e2e/port-l2.sh` on the grove walk (`target/e2e/lane-2/inf4/rec.sarec`, 5,200 frames)
and pause-restart (`target/e2e/corpus/pause-restart.sarec`, 3,700 frames), `SA_L2_WINDOW=data`, extra
regions from the `# STREAM-L2` part of `tools/e2e/l2-extras.txt`. Batches: in-game (`SA_L2_FROM=205`,
`L2_PER_FRAME=4`) and all-frames (`SA_L2_FROM=0`, `L2_PER_FRAME=1000`, only for hooks whose calls are in
the loading frames 0 and 201; spool discarded through a FIFO, `L2_SPOOL_DISCARD=1`, so the per-frame
cap does not cost disk). L3: `port-l2.sh ports --runs "plain rust"` (every port on Rust) is
frame-identical on all 5,200 frames and 17 subsystems. The l2 traces of the final in-game batches are
identical to the plain trace (grove walk 5,200 frames, pause-restart 3,700 frames); batches that sample
the loading frames are not (heap allocations of loaders are not restored, see below), so their
verdicts are per case only.

Totals: PASS 164, no call in the corpus 142, not replayable in this harness 97, mismatch 15 (all with a
cause below; one ledger candidate), partial coverage 11 (0 mismatches), undeclared only 5.

### Harness limits found (why 97 rows have no L2 verdict)

`SA_L2_WINDOW=data` restores only .data/.bss, declared regions and the heap arena's bookkeeping. A
function whose original run changes heap objects that are not declared leaves the Rust run starting
from a different state, and destroying an object twice corrupts allocator state that lives inside heap
blocks:

- H1, RenderWare/D3D objects and heap object graphs (create/destroy textures, dictionaries, clumps,
  atomics, frames, materials, col data; RW free lists live in heap blocks): cases mismatch (texture
  pointers differ per run; `RemoveModel` 3/4, `RwTextureGtaStreamRead` 4/4) and the game later crashes
  in `_rwFreeListAllocReal` (0x801C65), `RwFrameForAllObjects` (0x7F18CF) or loops in the renderer
  (stall at frame 2532 after `CFileLoader::SetRelatedModelInfoCB` cases).
- H2, CD/file I/O and OS handles: the CD stream thread, file positions and handles are outside memory
  (`LoadRequestedModels` cases end in a crash in `CIplStore::LoadIpl` 0x406205; `AddImageToList` gets
  handle 0x208 instead of 0x204; `CFileLoader::LoadLine` differs only on a CRT buffer refill;
  `CdStreamInit` gets a new LocalAlloc block, semaphore and thread).
- H3, loading-screen rendering: `CLoadingScreen::DoPCTitleFadeIn` crashes the L2 run (execution at
  0x03460000) on its first call; the other loading-screen functions create/destroy splash textures.
- `SA_L2_WINDOW=all` (whole arena restored) crashes in the D3D9 runtime (read of 0xFFFFFFFF at
  0x04959C4D): RW objects reference D3D resources whose reference counts are outside the arena.

These rows stay `implemented` (L3 evidence only: frame-identical with every port on Rust). Making them
L2-checkable needs a harness that snapshots/restores the whole arena *and* replays without D3D/file
side effects (sa-hook work, not done here).

### Open questions, measured

- `CPedModelInfo::m_pColNodeInfos`: the array starts at 0x8A630C (static, bytes): the three users
  (0x4C6D90, 0x4C6F70, 0x4C7170) load `ebp/ebx/esi = 0x8A6318`, read fields at -0xC, -8, -4, 0, +4, +8,
  step 0x1C and stop at 0x8A6468: 12 entries of 0x1C, first byte read 0x8A630C, last 0x8A6457 (83
  dwords). gta-reversed's 0x8A6308 is 4 bytes early. `AnimatePedColModelSkinned`/`...World` 1000/1000
  cases identical.
- Protection-VM values: `AddToLoadedVehiclesList` writes the CLoadedCarGroup at 0xC0E9C8
  (inappropriate cars) in both runs (22/22 cases identical); gang groups at stride 0x2E from 0xC0EA28;
  `IsCarModelNeededInCurrentZone` 34/34 (all-frames) and 12 (in-game) cases identical (second loop start
  18); `CColStore::AddColSlot` 252/252 identical ("stadint" length 7); `StreamCopModels` 1000 cases
  identical, two of them take the cop-bike timer branch (both runs write
  `ms_timePassedSinceLastCopBike`); `StreamFireEngineAndFireman` 14+17 calls, all identical (whether the
  request tail ran is not recorded per case).
- Uninitialised locals passed as 0: `LoadVehicleObject` (skimmer, `vehicles.ide` id 460, type plane,
  11 fields): the original stores stack data as wheel sizes (+0x40 = 0x00000008, +0x44 = 0x0036F144) and
  changes the low byte of +0x48; the port writes 0 (case frame 201 ordinal 60, `sa-oracle l2 show`).
  Not reproducible (stack contents of earlier calls): ledger candidate "original UB, uninitialised
  locals". `LoadTimeObject` 160/160, `Load2dEffect` 97/97, `LoadVehicleColours` 1/1 identical (the corpus
  never leaves those locals unset). `RequestModelStream` and `CdStreamInit` are not replayable (H2).
- Foreign globals in owner modules: no L2 difference attributable to them.

### Per-function results

calls / cases = the best run (calls counted while L2 was on; for not-replayable rows: calls from a
count-only run, `SA_L2_FROM=100000`). L3 = the all-ports Rust run (frame-identical) for every row.

| address | name | calls | cases | mismatches | undeclared cases | L2 | note |
|---|---|---|---|---|---|---|---|
| 0x4076A0 | CStreaming::IsVeryBusy | 1159 | 1000 | 0 | 0 | PASS |  |
| 0x407800 | CStreaming::IsObjectInCdImage | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x407A10 | CStreaming::ForceLayerToRead | 2 | 2 | 0 | 0 | PASS |  |
| 0x407A30 | CStreaming::SetLoadVehiclesInLoadScene | 4 | 4 | 0 | 0 | PASS |  |
| 0x407A40 | CStreaming::ClearFlagForAll | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x407AD0 | CStreaming::AreAnimsUsedByRequestedModels | 3 | 3 | 0 | 0 | PASS |  |
| 0x407C00 | CStreaming::GetDefaultCopModel | 6 | 6 | 0 | 0 | PASS |  |
| 0x407C50 | CStreaming::GetDefaultCopCarModel | 248 | 239 | 0 | 0 | PASS |  |
| 0x407D10 | CStreaming::DisableCopBikes | 1 | 1 | 0 | 0 | PASS |  |
| 0x407D20 | CStreaming::GetDefaultMedicModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x407D40 | CStreaming::GetDefaultFiremanModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x407D50 | CStreaming::GetDefaultCabDriverModel | 9 | 9 | 0 | 0 | PASS |  |
| 0x407F00 | CStreaming::HasSpecialCharLoaded | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x407F80 | CStreaming::WeAreTryingToPhaseVehicleOut | 1132 | 1000 | 0 | 0 | PASS |  |
| 0x407FB0 | CStreaming::FindMIPedSlotForInterior | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x408230 | thunk_FUN_01563580 | 3613 | 1000 | 0 | 0 | PASS |  |
| 0x4084F0 | CStreaming::InstanceLoadedModels | 3 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4090A0 | CStreaming::DeleteAllRwObjects | 3 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x409210 | CStreaming::DeleteRwObjectsAfterDeath | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4094B0 | CStreaming::RemoveBuildingsNotInArea | 4 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4096C0 | CStreaming::StartRenderEntities | 1001 | 1000 | 0 | 0 | PASS |  |
| 0x4096D0 | CStreaming::RenderEntity | 50396 | 1000 | 0 | 0 | PASS |  |
| 0x409710 | CStreaming::RemoveEntity | 1001 | 1000 | 0 | 0 | PASS |  |
| 0x40BA70 | CStreaming::PossiblyStreamCarOutAfterCreation | 16 | 16 | 0 | 0 | PASS |  |
| 0x4103A0 | CColStore::AddCollisionNeededAtPosn | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4107A0 | CColStore::AddRef | 455 | 267 | 0 | 0 | PASS |  |
| 0x4107D0 | CColStore::RemoveRef | 224 | 139 | 0 | 0 | PARTIAL | in-game run only (per-frame cap): 139 of 224 calls, 0 mismatches |
| 0x410800 | CColStore::GetBoundingBox | 50329 | 1000 | 0 | 0 | PASS |  |
| 0x410820 | CColStore::IncludeModelIndex | 10155 | 1000 | 0 | 0 | PASS |  |
| 0x407100 | CStreaming::RequestTxdModel | 1849 | 659 | 0 | 0 | PASS |  |
| 0x407610 | CStreaming::AddImageToList | 4 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x407820 | CStreaming::HasVehicleUpgradeLoaded | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x407DD0 | CStreaming::IsCarModelNeededInCurrentZone | 34 | 34 | 0 | 0 | PASS |  |
| 0x4083C0 | CStreaming::InitImageList | 1 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x4087E0 | CStreaming::RequestModel | 45187 | 1000 | 0 | 0 | PASS |  |
| 0x409650 | CStreaming::AddEntity | 2628 | 1000 | 357 | 315 | MISMATCH | cases 1-608 identical; from frame 2553 the link list is full and the original path deletes another entity's RW object (heap entity + RW freelists not restored; H1) |
| 0x409A90 | CStreaming::AreTexturesUsedByRequestedModels | 297 | 202 | 0 | 0 | PASS |  |
| 0x411140 | CColStore::AddColSlot | 252 | 252 | 0 | 0 | PASS |  |
| 0x408000 | CStreaming::AddToLoadedVehiclesList | 32 | 30 | 0 | 0 | PASS |  |
| 0x4084B0 | CStreaming::Shutdown | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x408C70 | CStreaming::RequestVehicleUpgrade | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x408E20 | CStreaming::GetNextFileOnCd | 2511 | 1000 | 0 | 0 | PASS |  |
| 0x409430 | CStreaming::RequestBigBuildings | 6 | 5 | 0 | 0 | PASS |  |
| 0x40A120 | CStreaming::LoadInitialWeapons | 1 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x40AFA0 | CStreaming::ReclassifyLoadedCars | 8 | 7 | 0 | 0 | PARTIAL | 7 of 8 calls compared (one before frame 205), 0 mismatches |
| 0x40C270 | CStreaming::ProcessEntitiesInSectorList<CPtrListSingleLink<CEntity*>>- | 11267 | 1000 | 0 | 0 | PASS |  |
| 0x40C450 | CStreaming::ProcessEntitiesInSectorList | 1017 | 1000 | 0 | 0 | PASS |  |
| 0x4104E0 | CColStore::SetCollisionRequired | 1004 | 1000 | 0 | 0 | PASS |  |
| 0x4113F0 | CColStore::Initialise | 1 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x5BCCD0 | CStreaming::ReadIniFile | 1 | 1 | 0 | 0 | PASS |  |
| 0x40B4B0 | CStreaming::LoadZoneVehicle | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x40B4F0 | CStreaming::StreamOneNewCar | 10 | 10 | 0 | 0 | PASS |  |
| 0x40C520 | CStreaming::AddLodsToRequestList | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x410690 | CColStore::LoadCol- | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4106D0 | CColStore::LoadCol-2 | 288 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x410C00 | CColStore::RequestCollision | 2 | 1 | 0 | 0 | PASS |  |
| 0x410CE0 | CColStore::HasCollisionLoaded | 2 | 2 | 0 | 0 | PASS |  |
| 0x410EC0 | CColStore::BoundingBoxesPostProcess | 1 | 1 | 0 | 0 | PASS |  |
| 0x5B6170 | CStreaming::LoadCdDirectory-char | 5 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x5B82C0 | CStreaming::LoadCdDirectory-void | 1 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x5D29A0 | CStreaming::Save | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x407A70 | CStreaming::DeleteRwObjectsInSectorList<CPtrListSingleLink<CEntity*>> | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x410730 | CColStore::RemoveCol | 266 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x5B8AD0 | CStreaming::Init2 | 1 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x5D29E0 | CStreaming::Load | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x411330 | CColStore::RemoveColSlot | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4114D0 | CColStore::Shutdown | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4080F0 | CStreaming::RemoveCarModel | 9 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4089A0 | CStreaming::RemoveModel | 755154 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x409C10 | CStreaming::SetModelIsDeletable | 2299 | 1000 | 0 | 0 | PASS |  |
| 0x409C70 | CStreaming::SetModelTxdIsDeletable | 8 | 8 | 0 | 0 | PASS |  |
| 0x409050 | CStreaming::RequestFilesInChannel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4093B0 | CStreaming::RemoveBigBuildings | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x409760 | CStreaming::DeleteLeastUsedEntityRwObject | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x409940 | CStreaming::DeleteRwObjectsBehindCameraInSectorList<CPtrListSingleLink<CEntity*>> | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4099E0 | CStreaming::DeleteRwObjectsNotInFrustumInSectorList<CPtrListSingleLink<CEntity*>> | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x409C90 | CStreaming::SetMissionDoesntRequireModel | 1033 | 1000 | 0 | 0 | PASS |  |
| 0x40A080 | CStreaming::RequestFile | 33 | 24 | 0 | 0 | PASS |  |
| 0x40A150 | CStreaming::StreamCopModels | 1004 | 1000 | 0 | 0 | PASS |  |
| 0x40A2A0 | CStreaming::StreamAmbulanceAndMedic | 18 | 18 | 0 | 0 | PASS |  |
| 0x40A400 | CStreaming::StreamFireEngineAndFireman | 18 | 18 | 0 | 0 | PASS |  |
| 0x40AA10 | CStreaming::StreamZoneModels_Gangs | 1003 | 1000 | 0 | 0 | PASS |  |
| 0x40B080 | CStreaming::RemoveCurrentZonesModels | 1 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x40B340 | CStreaming::RemoveLoadedZoneModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x40B3A0 | CStreaming::RemoveInappropriatePedModels | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x40BAA0 | CStreaming::ClearSlots | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x40BDA0 | CStreaming::StreamPedsIntoRandomSlots | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x40BE60 | CStreaming::RemoveDodgyPedsFromRandomSlots | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x40C020 | CStreaming::RemoveLoadedVehicle | 8 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x40C180 | CStreaming::RemoveTxdModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x40C1E0 | CStreaming::PurgeRequestList | 1003 | 1000 | 0 | 0 | PASS |  |
| 0x40CBA0 | CStreaming::RequestModelStream | 11283 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x40D2F0 | CStreaming::RemoveReferencedTxds | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x410860 | CColStore::LoadCollision | 1003 | 1000 | 4 | 0 | MISMATCH | 998/1000 identical; 2 cases differ in CCollision::ms_colModelCache order (cache links live in CCollisionData heap blocks; H1) |
| 0x410E00 | CColStore::RemoveAllCollision | 1 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x40B490 | CStreaming::SetMissionDoesntRequireSpecialChar | 10 | 10 | 0 | 0 | PASS |  |
| 0x40B650 | CStreaming::StreamVehiclesAndPeds_Always | 1003 | 1000 | 0 | 0 | PASS |  |
| 0x40B700 | CStreaming::StreamVehiclesAndPeds | 1002 | 1000 | 0 | 0 | PASS |  |
| 0x40BBB0 | CStreaming::StreamPedsForInterior | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x40CF80 | CStreaming::RemoveAllUnusedModels | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x40CFD0 | CStreaming::RemoveLeastUsedModel | 302 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x40D3D0 | CStreaming::LoadInitialPeds | 1 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x48B570 | CStreaming::SetMissionDoesntRequireAnim | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x408CB0 | CStreaming::FinishLoadingLargeFile | 4 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x40C6B0 | CStreaming::ConvertBufferToObject | 1902 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x40E120 | CStreaming::MakeSpaceFor | 1675 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4076C0 | CStreaming::RetryLoadFile | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x40E170 | CStreaming::ProcessLoadingChannel | 1052 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x40E3A0 | CStreaming::LoadRequestedModels | 5009 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x40E460 | CStreaming::FlushChannels | 684 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x40E4E0 | CStreaming::FlushRequestList | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x40EA10 | CStreaming::LoadAllRequestedModels | 342 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x409D10 | CStreaming::RequestSpecialModel | 9 | 8 | 0 | 0 | PASS |  |
| 0x40E560 | CStreaming::ReInit | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x40ED80 | CStreaming::LoadSceneCollision | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x410AD0 | CColStore::EnsureCollisionIsInMemory | 4994 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x410E60 | CColStore::LoadAllCollision | 1 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x40B450 | CStreaming::RequestSpecialChar | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x40E670 | CStreaming::Update | 4994 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x4113D0 | CColStore::LoadAllBoundingBoxes | 1 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x40EB70 | CStreaming::LoadScene | 3 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x4C4BC0 | CBaseModelInfo::SetColModel | 1001 | 1000 | 0 | 0 | PASS |  |
| 0x4C5200 | CClumpModelInfo::SetAnimFile | 384 | 384 | 0 | 0 | PASS |  |
| 0x4C59B0 | CModelInfo::GetModelInfoFromHashKey | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C5A70 | CModelInfo::IsBoatModel | 225 | 225 | 0 | 0 | PASS |  |
| 0x4C5AA0 | CModelInfo::IsCarModel | 84 | 84 | 0 | 0 | PASS |  |
| 0x4C5AD0 | CModelInfo::IsTrainModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C5B00 | CModelInfo::IsHeliModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C5B30 | CModelInfo::IsPlaneModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C5B60 | CModelInfo::IsBikeModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C5B90 | CModelInfo::IsFakePlaneModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C5BC0 | CModelInfo::IsMonsterTruckModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C5BF0 | CModelInfo::IsQuadBikeModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C5C20 | CModelInfo::IsBmxModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C5C50 | CModelInfo::IsTrailerModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C5C80 | CModelInfo::IsVehicleModelType | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C6200 | FUN_004c6200 | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C6620 | CModelInfo::AddAtomicModel | 13976 | 1000 | 0 | 0 | PASS |  |
| 0x4C6650 | CModelInfo::AddDamageAtomicModel | 69 | 57 | 0 | 0 | PASS |  |
| 0x4C6680 | CModelInfo::AddLodAtomicModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C66B0 | CModelInfo::AddTimeModel | 160 | 160 | 0 | 0 | PASS |  |
| 0x4C66E0 | CModelInfo::AddLodTimeModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C6710 | CModelInfo::AddWeaponModel | 50 | 50 | 0 | 0 | PASS |  |
| 0x4C6740 | CModelInfo::AddClumpModel | 89 | 35 | 0 | 0 | PASS |  |
| 0x4C6770 | CModelInfo::AddVehicleModel | 212 | 212 | 0 | 0 | PASS |  |
| 0x4C67A0 | CModelInfo::AddPedModel | 276 | 276 | 0 | 0 | PASS |  |
| 0x5BCA10 | ModelIndices::InitModelIndices | 1 | 1 | 0 | 0 | PASS |  |
| 0x856230 | __dtor_CModelInfo__ms_atomicModelInfoStore | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x856240 | __dtor_CModelInfo__ms_damageAtomicModelInfoStore | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x856250 | __dtor_CModelInfo__ms_lodAtomicModelInfoStore | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x856260 | __dtor_CModelInfo__ms_timeModelInfoStore | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x856270 | __dtor_CModelInfo__ms_lodTimeModelInfoStore | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x856280 | __dtor_CModelInfo__ms_weaponModelInfoStore | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x856290 | __dtor_CModelInfo__ms_clumpModelInfoStore | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x8562A0 | __dtor_CModelInfo__ms_vehicleModelInfoStore | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x8562B0 | __dtor_CModelInfo__ms_pedModelInfoStore | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C47E0 | CTimeInfo::FindOtherTimeModel | 160 | 160 | 0 | 0 | PASS |  |
| 0x4C4AF0 | CBaseModelInfo::`scalar deleting destructor' | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C4B80 | CBaseModelInfo::AddTexDictionaryRef | 1220 | 850 | 0 | 0 | PASS |  |
| 0x4C4BA0 | CBaseModelInfo::AddRef | 3464 | 1000 | 0 | 0 | PASS |  |
| 0x4C5280 | CClumpModelInfo::SetAtomicRendererCB | 66 | 65 | 0 | 0 | PASS |  |
| 0x4C52A0 | CClumpModelInfo::FindFrameFromNameCB | 1 | 0 | 0 | 0 | PARTIAL | 1 call, nested in a sampled case (not sampled) |
| 0x4C52F0 | CClumpModelInfo::FindFrameFromNameWithoutIdCB | 40599 | 1000 | 0 | 0 | PASS |  |
| 0x4C5350 | CClumpModelInfo::FindFrameFromIdCB | 31517 | 1000 | 0 | 0 | PASS |  |
| 0x4C5390 | CClumpModelInfo::FillFrameArrayCB | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C53C0 | CClumpModelInfo::GetFrameFromId | 1013 | 1000 | 0 | 0 | PASS |  |
| 0x4C5400 | CClumpModelInfo::GetFrameFromName | 26 | 26 | 0 | 0 | PASS |  |
| 0x4C5440 | CClumpModelInfo::FillFrameArray | 231 | 231 | 0 | 0 | PASS |  |
| 0x4C5460 | CClumpModelInfo::SetFrameIds | 83 | 82 | 0 | 0 | PASS |  |
| 0x4C57A0 | CPedModelInfo::CPedModelInfo | 278 | 278 | 0 | 0 | PASS |  |
| 0x4C5940 | CModelInfo::GetModelInfo-full | 24833 | 1000 | 0 | 0 | PASS |  |
| 0x4C5A20 | CModelInfo::GetModelInfo-minmax | 1528 | 1000 | 0 | 0 | PASS |  |
| 0x4C6210 | CAtomicModelInfo::`scalar deleting destructor' | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C6230 | CDamageAtomicModelInfo::`scalar deleting destructor' | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C6250 | CLodAtomicModelInfo::`scalar deleting destructor' | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C6270 | CTimeModelInfo::`scalar deleting destructor' | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C6290 | CLodTimeModelInfo::`scalar deleting destructor' | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C62B0 | CClumpModelInfo::`scalar deleting destructor' | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C62D0 | CWeaponModelInfo::`scalar deleting destructor' | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C63B0 | CModelInfo::ReInit2dEffects | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x84BBF0 | __init_CModelInfo__ms_atomicModelInfoStore | 1 | 1 | 0 | 0 | PASS |  |
| 0x84BC10 | __init_CModelInfo__ms_damageAtomicModelInfoStore | 1 | 1 | 0 | 0 | PASS |  |
| 0x84BC30 | __init_CModelInfo__ms_lodAtomicModelInfoStore | 1 | 1 | 0 | 0 | PASS |  |
| 0x84BC50 | __init_CModelInfo__ms_timeModelInfoStore | 1 | 1 | 0 | 0 | PASS |  |
| 0x84BC70 | __init_CModelInfo__ms_lodTimeModelInfoStore | 1 | 1 | 0 | 0 | PASS |  |
| 0x84BC90 | __init_CModelInfo__ms_weaponModelInfoStore | 1 | 1 | 0 | 0 | PASS |  |
| 0x84BCB0 | __init_CModelInfo__ms_clumpModelInfoStore | 1 | 1 | 0 | 0 | PASS |  |
| 0x84BCD0 | __init_CModelInfo__ms_vehicleModelInfoStore | 1 | 1 | 0 | 0 | PASS |  |
| 0x84BCF0 | __init_CModelInfo__ms_pedModelInfoStore | 1 | 1 | 0 | 0 | PASS |  |
| 0x4C4B40 | CBaseModelInfo::SetTexDictionary | 14841 | 1000 | 0 | 0 | PASS |  |
| 0x4C4C70 | CBaseModelInfo::Get2dEffect | 345165 | 1000 | 0 | 0 | PASS |  |
| 0x4C4EF0 | CClumpModelInfo::SetHierarchyForSkinAtomic | 378 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C5250 | CClumpModelInfo::ConvertAnimFileIndex | 419 | 419 | 0 | 0 | PASS |  |
| 0x4C59F0 | CModelInfo::GetModelInfoUInt16 | 160 | 160 | 0 | 0 | PASS |  |
| 0x4C6D90 | CPedModelInfo::CreateHitColModelSkinned | 56 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C4360 | CAtomicModelInfo::SetAtomic | 994 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C48D0 | CDamageAtomicModelInfo::SetDamagedAtomic | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C4F30 | CClumpModelInfo::AtomicSetupLightingCB | 780 | 694 | 0 | 0 | PASS |  |
| 0x4C4F70 | CClumpModelInfo::SetClump | 112 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C6810 | CModelInfo::Initialise | 1 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C6F70 | CPedModelInfo::AnimatePedColModelSkinned | 14153 | 1000 | 0 | 0 | PASS |  |
| 0x4C7170 | CPedModelInfo::AnimatePedColModelSkinnedWorld | 1001 | 1000 | 0 | 0 | PASS |  |
| 0x5B57C0 | ModelIndices::MatchAllModelStrings | 1 | 1 | 0 | 0 | PASS |  |
| 0x4C7340 | CPedModelInfo::SetClump | 56 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C9910 | CWeaponModelInfo::SetClump | 7 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C4C40 | CBaseModelInfo::DeleteCollisionModel | 4309 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C62F0 | CPedModelInfo::~CPedModelInfo | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C6350 | CPedModelInfo::`scalar deleting destructor' | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C63E0 | CModelInfo::ShutDown | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C4B90 | CBaseModelInfo::RemoveTexDictionaryRef | 253 | 127 | 13 | 13 | MISMATCH | mismatching cases are the ones whose TXD reference drops to 0 (RwTexDictionaryDestroy; H1) |
| 0x4C4BB0 | CBaseModelInfo::RemoveRef | 2926 | 1000 | 0 | 0 | PASS |  |
| 0x4C4440 | CAtomicModelInfo::DeleteRwObject | 294 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C44D0 | CAtomicModelInfo::CreateInstance-rwmat | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C4530 | CAtomicModelInfo::CreateInstance-void | 36159 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C4960 | CDamageAtomicModelInfo::CreateInstance-void | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C4E70 | CClumpModelInfo::DeleteRwObject | 24 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C5140 | CClumpModelInfo::CreateInstance-void | 590 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C4910 | CDamageAtomicModelInfo::CreateInstance-rwmat | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C49D0 | CDamageAtomicModelInfo::DeleteRwObject | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C6C50 | CPedModelInfo::DeleteRwObject | 4 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C4D50 | CBaseModelInfo::Shutdown | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C5110 | CClumpModelInfo::CreateInstance-mat | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C4E60 | CClumpModelInfo::Shutdown | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C7E10 | CVehicleModelInfo::IsValidCompRule | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C7E50 | CVehicleModelInfo::GetListOfComponentsNotUsedByRules | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C8200 | CVehicleModelInfo::IsUpgradeAvailable | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C8220 | CVehicleModelInfo::SetEditableMaterialsCB-RpMaterial | 53149 | 1000 | 0 | 0 | PASS |  |
| 0x4C83B0 | CVehicleModelInfo::RemoveWindowAlphaCB | 1874 | 1000 | 0 | 0 | PASS |  |
| 0x4C8460 | CVehicleModelInfo::ResetEditableMaterials | 1001 | 1000 | 0 | 0 | PASS |  |
| 0x4C84B0 | CVehicleModelInfo::SetVehicleColour | 1001 | 1000 | 0 | 0 | PASS |  |
| 0x4C8700 | CVehicleModelInfo::AddWheelUpgrade | 29 | 29 | 0 | 0 | PASS |  |
| 0x4C8740 | CVehicleModelInfo::GetNumWheelUpgrades | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C8750 | CVehicleModelInfo::GetWheelUpgrade | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C8980 | CVehicleModelInfo::SetCustomCarPlateText | 8 | 8 | 0 | 0 | PASS |  |
| 0x4C89B0 | CVehicleModelInfo::GetMaximumNumberOfPassengersFromNumberOfDoors | 231 | 231 | 0 | 0 | PASS |  |
| 0x4C8C90 | CVehicleModelInfo::SetupLightFlags | 1001 | 1000 | 0 | 0 | PASS |  |
| 0x4C8D60 | CVehicleModelInfo::CVehicleStructure::CVehicleStructure | 32 | 30 | 0 | 0 | PASS |  |
| 0x536FE0 | CFileLoader::LoadLine-Bufer | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x5B41C0 | CFileLoader::LoadPathHeader | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C7510 | CVehicleModelInfo::FindTextureCB | 5184 | 1000 | 0 | 0 | PASS |  |
| 0x4C75A0 | CVehicleModelInfo::UseCommonVehicleTexDicationary | 32 | 30 | 0 | 0 | PASS |  |
| 0x4C75C0 | CVehicleModelInfo::StopUsingCommonVehicleTexDicationary | 32 | 30 | 0 | 0 | PASS |  |
| 0x4C7670 | CVehicleModelInfo::SetAnimFile | 212 | 212 | 0 | 0 | PASS |  |
| 0x4C7720 | CVehicleModelInfo::HideDamagedAtomicCB | 39 | 4 | 0 | 0 | PARTIAL | 4 of 39 calls compared (nested in SetClump cases / per-frame cap) |
| 0x4C7790 | CVehicleModelInfo::HideAllComponentsAtomicCB | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C7A30 | CVehicleModelInfo::SetAtomicRendererCB_Heli | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C7B10 | CVehicleModelInfo::SetAtomicRenderCallbacks | 32 | 30 | 0 | 0 | PASS |  |
| 0x4C7B90 | CVehicleModelInfo::SetAtomicFlagCB | 1574 | 1000 | 0 | 0 | PASS |  |
| 0x4C7BB0 | CVehicleModelInfo::ClearAtomicFlagCB | 36 | 36 | 0 | 0 | PASS |  |
| 0x4C7BD0 | CVehicleModelInfo::GetOkAndDamagedAtomicCB | 512 | 452 | 0 | 0 | PASS |  |
| 0x4C7C10 | CVehicleModelInfo::SetVehicleComponentFlags | 453 | 404 | 0 | 0 | PASS |  |
| 0x4C7FB0 | CVehicleModelInfo::ChooseComponent-Global | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C83E0 | CVehicleModelInfo::SetEditableMaterialsCB-RpAtomic | 22295 | 1000 | 865 | 0 | MISMATCH | materials of the atomic's geometry are heap objects (not expressible as regions); the RpMaterial callback passes once its material is declared |
| 0x4C8430 | CVehicleModelInfo::SetEditableMaterials | 1001 | 1000 | 3000 | 0 | MISMATCH | as above: the original run recolours the clump's materials in the heap, the Rust run then finds no magic colours |
| 0x4C8500 | CVehicleModelInfo::ChooseVehicleColour | 320 | 320 | 0 | 0 | PASS |  |
| 0x4C8810 | CVehicleModelInfo::GetMatFXEffectMaterialCB | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C88B0 | CVehicleModelInfo::SetEnvMapCoeffCB | 80721 | 1000 | 0 | 0 | PASS |  |
| 0x4C8E10 | CVehicleModelInfo::`scalar deleting destructor' | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C9410 | CVehicleModelInfo::SetEnvironmentMapAtomicCB | 5671 | 1000 | 0 | 0 | PASS |  |
| 0x4C9430 | CVehicleModelInfo::SetEnvMapCoeffAtomicCB | 22295 | 1000 | 0 | 0 | PASS |  |
| 0x4C97E0 | CVehicleModelInfo::DisableEnvMap | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C9800 | CVehicleModelInfo::SetEnvMapCoeff | 1001 | 1000 | 0 | 0 | PASS |  |
| 0x536F80 | CFileLoader::LoadLine-File | 36264 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x537580 | CFileLoader::LoadCollisionModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x537AE0 | CFileLoader::LoadCollisionModelVer4 | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x537CE0 | CFileLoader::LoadCollisionModelVer3 | 11157 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x537EE0 | CFileLoader::LoadCollisionModelVer2 | 1408 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x5B3910 | CFileLoader::AddTexDictionaries | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x5B4530 | CFileLoader::LoadGarage | 52 | 52 | 0 | 0 | PASS |  |
| 0x5B4C80 | CFileLoader::LoadOcclusionVolume | 995 | 995 | 0 | 0 | PASS |  |
| 0x5B4D70 | CFileLoader::LoadAudioZone | 155 | 155 | 0 | 0 | PASS |  |
| 0x5B81D0 | CFileLoader::LoadTimeCyclesModifier | 8 | 8 | 0 | 0 | PASS |  |
| 0x4C7630 | CVehicleModelInfo::Init | 212 | 212 | 0 | 0 | PASS |  |
| 0x4C76D0 | CVehicleModelInfo::ConvertAnimFileIndex | 212 | 212 | 0 | 0 | PASS |  |
| 0x4C77E0 | CVehicleModelInfo::SetAtomicRendererCB | 43 | 4 | 0 | 0 | PARTIAL | 4 of 43 calls compared (as above) |
| 0x4C7870 | CVehicleModelInfo::SetAtomicRendererCB_RealHeli | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C7930 | CVehicleModelInfo::SetAtomicRendererCB_Plane | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C79A0 | CVehicleModelInfo::SetAtomicRendererCB_Boat | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C7AA0 | CVehicleModelInfo::SetAtomicRendererCB_Train | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C7D20 | CVehicleModelInfo::GetWheelPosn | 1075 | 1000 | 0 | 0 | PASS |  |
| 0x4C7DD0 | CVehicleModelInfo::GetOriginalCompPosition | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C8040 | CVehicleModelInfo::ChooseComponent | 53 | 53 | 0 | 0 | PASS |  |
| 0x4C8120 | CVehicleModelInfo::ChooseSecondComponent | 53 | 53 | 0 | 0 | PASS |  |
| 0x4C8780 | CVehicleModelInfo::LoadEnvironmentMaps | 1 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C8B60 | CVehicleModelInfo::StoreAtomicUsedMaterialsCB | 572 | 76 | 0 | 76 | UNDECLARED | 0 mismatches; writes through its data cursor into the caller's stack list (not declared) |
| 0x4C8E30 | CVehicleModelInfo::CollapseFramesCB | 570 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C9360 | CVehicleModelInfo::AssignRemapTxd | 572 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C9450 | CVehicleModelInfo::SetCarCustomPlate | 32 | 30 | 77 | 9 | MISMATCH | the original run replaces the plate material texture in the heap; the Rust run then finds no plate material (H1) |
| 0x537990 | CFileLoader::LoadCarGenerator-0 | 167 | 102 | 0 | 0 | PASS |  |
| 0x538440 | CFileLoader::LoadCollisionFile-Buffer | 37 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x5B45D0 | CFileLoader::LoadStuntJump | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x5B4AB0 | CFileLoader::LoadZone | 384 | 384 | 0 | 0 | PASS |  |
| 0x5B4B40 | CFileLoader::LoadCullZone | 1266 | 1000 | 0 | 0 | PASS |  |
| 0x5B4E60 | CFileLoader::LoadCollisionFile-File | 1 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x5B5000 | CFileLoader::LoadCollisionFileFirstTime | 251 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x5B65A0 | CVehicleModelInfo::LoadVehicleUpgrades | 1 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x5B6890 | CVehicleModelInfo::LoadVehicleColours | 1 | 1 | 0 | 0 | PASS |  |
| 0x5B75E0 | CFileLoader::LoadTXDParent | 23 | 23 | 0 | 0 | PASS |  |
| 0x5B7670 | CFileLoader::Load2dEffect | 97 | 97 | 0 | 0 | PASS |  |
| 0x4C7470 | CVehicleModelInfo::ShutdownLightTexture | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C7700 | CVehicleModelInfo::MoveObjectsCB | 489 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C87D0 | CVehicleModelInfo::ShutdownEnvironmentMaps | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x5B3C60 | CFileLoader::LoadObject | 14045 | 1000 | 0 | 0 | PASS |  |
| 0x5B3DE0 | CFileLoader::LoadTimeObject | 160 | 160 | 0 | 0 | PASS |  |
| 0x5B3FB0 | CFileLoader::LoadWeaponObject | 50 | 50 | 0 | 0 | PASS |  |
| 0x5B4040 | CFileLoader::LoadClumpObject | 35 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x5B40C0 | CFileLoader::LoadAnimatedClumpObject | 54 | 54 | 0 | 0 | PASS |  |
| 0x5B4740 | CFileLoader::LoadCarGenerator-1 | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x5B6F30 | CFileLoader::LoadVehicleObject | 212 | 212 | 1 | 0 | MISMATCH | 211/212 identical; skimmer (vehicles.ide 460, 11 fields): wheel sizes +0x40/+0x44 and wheel model index +0x48 are uninitialised locals in the original (0x00000008, 0x0036F144 = stack data); the port writes 0. Ledger candidate |
| 0x5B7420 | CFileLoader::LoadPedObject | 276 | 276 | 0 | 0 | PASS |  |
| 0x4C8900 | CVehicleModelInfo::SetRenderPipelines | 31 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x537150 | CFileLoader::SetRelatedModelInfoCB | 994 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x5B3930 | CFileLoader::FindRelatedModelInfoCB | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x5B8030 | CFileLoader::LoadEntryExit | 376 | 376 | 0 | 0 | PASS |  |
| 0x5B8400 | CFileLoader::LoadObjectTypes | 57 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x5D5DB0 | CVehicleModelInfo::SetDirtTextures | 1001 | 1000 | 0 | 0 | PASS |  |
| 0x4C8840 | CVehicleModelInfo::SetEnvironmentMapCB | 20184 | 720 | 0 | 0 | PARTIAL | in-game run only: 720 of 20184 calls, 0 mismatches |
| 0x4C8BD0 | CVehicleModelInfo::ReduceMaterialsInVehicle | 31 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x538690 | CFileLoader::LoadObjectInstance-file | 9268 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x5B3860 | CFileLoader::LoadTexDictionary | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C8E60 | CVehicleModelInfo::PreprocessHierarchy | 31 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x5B51E0 | CFileLoader::LinkLods | 52 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C95C0 | CVehicleModelInfo::SetClump | 31 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x5371F0 | CFileLoader::LoadAtomicFile-stream | 994 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x5372D0 | CFileLoader::LoadClumpFile-1 | 110 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x5373F0 | CFileLoader::StartLoadClumpFile | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x537450 | CFileLoader::FinishLoadClumpFile | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x5B39D0 | CFileLoader::LoadAtomicFile- | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x5B3A30 | CFileLoader::LoadClumpFile- | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4C9680 | CVehicleModelInfo::CreateInstance | 255 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x4C9890 | CVehicleModelInfo::DeleteRwObject | 9 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x5B8F00 | CVehicleModelInfo::SetupCommonData | 1 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x5B3680 | LoadingScreenLoadingFile | 114 | 4 | 1 | 4 | MISMATCH | early run without the txd-pool regions; loads a TXD (H1) |
| 0x5B47B0 | CFileLoader::LoadPickup | 5 | 5 | 0 | 0 | PASS |  |
| 0x5B8700 | CFileLoader::LoadScene | 52 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x5B9030 | CFileLoader::LoadLevel | 2 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x406370 | CdStream::CdStreamShutdown | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4063E0 | CdStream::CdStreamGetStatus | 1240 | 866 | 0 | 0 | PARTIAL | 866 of 867 calls compared, 0 mismatches |
| 0x406460 | CdStream::CdStreamSync | 835 | 600 | 0 | 0 | PASS |  |
| 0x406560 | CdStream::CdStreamThread | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x406690 | CdStream::CdStreamRemoveImages | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4068F0 | CdStream::CdStreamInitThread | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x406B70 | CdStream::CdStreamInit | 1 | 1 | 1 | 1 | MISMATCH | LocalAlloc block, semaphore and thread handles of the second run differ (OS resources; H2) |
| 0x4045B0 | CIplStore::AddIplsNeededAtPosn | 198 | 198 | 0 | 0 | PASS |  |
| 0x4045E0 | CIplStore::ClearIplsNeededAtPosn | 2 | 2 | 0 | 0 | PASS |  |
| 0x4045F0 | CIplStore::SetIfInteriorIplIsRequired | 17008 | 1000 | 0 | 0 | PASS |  |
| 0x404660 | CIplStore::SetIfIplIsRequired | 3256 | 592 | 0 | 0 | PARTIAL | pause-restart only: 592 of 3256 calls compared (per-frame cap 4) |
| 0x404690 | CIplStore::SetIfIplIsRequiredReducedBB | 8471 | 1000 | 0 | 0 | PASS |  |
| 0x404700 | CIplStore::SetIplsRequired | 1006 | 1000 | 0 | 0 | PASS |  |
| 0x404780 | CIplStore::GetNewIplEntityIndexArray | 30 | 30 | 0 | 0 | PASS |  |
| 0x4047B0 | CIplStore::GetIplEntityIndexArray | 30 | 30 | 0 | 0 | PASS |  |
| 0x404A60 | CIplStore::GetIplName | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x404A90 | CIplStore::SetIsInterior | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x404AC0 | CIplStore::FindIplSlot | 196 | 193 | 0 | 0 | PASS |  |
| 0x404B20 | CIplStore::RemoveIpl | 216 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x404C70 | CIplStore::GetBoundingBox | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x404C90 | CIplStore::IncludeEntity | 52842 | 1000 | 0 | 0 | PASS |  |
| 0x404D30 | CIplStore::EnableDynamicStreaming | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x404DE0 | CIplStore::SetupRelatedIpls | 52 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x405110 | CIplStore::RemoveRelatedIpls | 52 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x405170 | CIplStore::LoadIpls | 4997 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x4053F0 | CIplStore::EnsureIplsAreInMemory | 4994 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x405520 | CIplStore::RequestIpls | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x405600 | CIplStore::HaveIplsLoaded | 2 | 2 | 0 | 0 | PASS |  |
| 0x405720 | CIplStore::RemoveAllIpls | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x405780 | CIplStore::LoadAllRemainingIpls | 1 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x405850 | CIplStore::RequestIplAndIgnore | 6 | 5 | 0 | 0 | PASS |  |
| 0x405890 | CIplStore::RemoveIplAndIgnore | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x4058D0 | CIplStore::RemoveIplWhenFarAway | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x405AC0 | CIplStore::AddIplSlot | 191 | 191 | 0 | 0 | PASS |  |
| 0x405B60 | CIplStore::RemoveIplSlot | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x405C00 | CIplStore::LoadIplBoundingBox | 190 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x405EC0 | CIplStore::Initialise | 1 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x405FA0 | CIplStore::Shutdown | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x406080 | CIplStore::LoadIpl | 232 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x5D5420 | CIplStore::Save | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x5D54A0 | CIplStore::Load | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x408340 | CTxdStore::GetTxd | 35 | 33 | 0 | 0 | PASS |  |
| 0x408370 | CTxdStore::GetParentTxdSlot | 260 | 116 | 0 | 0 | PASS |  |
| 0x731680 | CTxdStore::RemoveIfRefCountIsGreaterThanOne | 225 | 24 | 0 | 0 | PARTIAL | in-game run only: 24 of 225 calls, 0 mismatches |
| 0x7316A0 | CTxdStore::PushCurrentTxd | 211 | 71 | 0 | 0 | PASS |  |
| 0x7316B0 | CTxdStore::PopCurrentTxd | 211 | 71 | 0 | 0 | PASS |  |
| 0x731720 | CTxdStore::TxdStoreFindCB | 4420 | 1000 | 0 | 0 | PASS |  |
| 0x731850 | CTxdStore::FindTxdSlot-name | 19111 | 1000 | 0 | 0 | PASS |  |
| 0x7318E0 | CTxdStore::FindTxdSlot-hash | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x731930 | CTxdStore::StartLoadTxd | 4 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x731990 | CTxdStore::Create | 2 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x7319C0 | CTxdStore::SetCurrentTxd | 1426 | 918 | 0 | 0 | PASS |  |
| 0x731A00 | CTxdStore::AddRef | 6051 | 1000 | 0 | 0 | PASS |  |
| 0x731A30 | CTxdStore::RemoveRef | 3824 | 1000 | 2 | 2 | MISMATCH | 998/1000 identical; the 2 differing cases drop the count to 0 and destroy the RW dictionary (H1) |
| 0x731A70 | CTxdStore::RemoveRefWithoutDelete | 1024 | 350 | 0 | 0 | PARTIAL | in-game run only: 350 of 1024 calls, 0 mismatches |
| 0x731AA0 | CTxdStore::GetNumRefs | 1099 | 946 | 0 | 0 | PASS |  |
| 0x731C80 | CTxdStore::AddTxdSlot | 3597 | 1000 | 0 | 0 | PASS |  |
| 0x731CD0 | CTxdStore::RemoveTxdSlot | 3 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x731D50 | CTxdStore::SetupTxdParent | 311 | 220 | 0 | 0 | PASS |  |
| 0x731DD0 | CTxdStore::LoadTxd-stream | 259 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x731E40 | CTxdStore::FinishLoadTxd | 4 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x731E90 | CTxdStore::RemoveTxd | 63 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x731F20 | CTxdStore::Initialise | 1 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x732000 | CTxdStore::Shutdown | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x732060 | CTxdStore::GameShutdown | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x7320B0 | CTxdStore::LoadTxd-filename | 13 | 0 | 0 | 0 | not replayable | H2 CD/file I/O, OS handles |
| 0x611C50 | CLoadedCarGroup::PickRandomCar | 1179 | 1000 | 0 | 0 | PASS |  |
| 0x611E10 | CLoadedCarGroup::SortBasedOnUsage | 40 | 4 | 0 | 0 | PARTIAL | pause-restart only: 4 of 40 calls compared (per-frame cap) |
| 0x611E90 | CLoadedCarGroup::PickLeastUsedModel | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x53DED0 | CLoadingScreen::LoadingScreen | 132 | 6 | 0 | 6 | UNDECLARED | early run: undeclared CLoadingScreen message buffers + RW engine statics |
| 0x58FF10 | CLoadingScreen::Shutdown | 2 | 2 | 2 | 2 | MISMATCH | destroys splash sprites/TXD (H1/H3) |
| 0x58FF60 | CLoadingScreen::RenderSplash | 1106 | 12 | 0 | 12 | UNDECLARED | undeclared RW/D3D9 driver statics (render-state cache) |
| 0x5900B0 | CLoadingScreen::LoadSplashes | 2 | 2 | 2 | 2 | MISMATCH | creates RW textures: texture pointers differ per run (H1/H3) |
| 0x590220 | CLoadingScreen::DisplayMessage | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x590240 | CLoadingScreen::SetLoadingBarMsg | 117 | 4 | 0 | 4 | UNDECLARED | undeclared CLoadingScreen::m_LoadingGxtMsg1/2 |
| 0x590280 | CLoadingScreen::GetClockTime | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x5902B0 | CLoadingScreen::Init | 3 | 3 | 1 | 1 | MISMATCH | calls LoadSplashes (H1/H3) |
| 0x590320 | CLoadingScreen::Continue | 1 | 1 | 0 | 0 | PASS |  |
| 0x590370 | CLoadingScreen::RenderLoadingBar | 139 | 0 | 0 | 0 | not replayable | H3 loading-screen rendering |
| 0x5904D0 | CLoadingScreen::DisplayNextSplash | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x590530 | CLoadingScreen::StartFading | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x590570 | CLoadingScreen::DisplayPCScreen | 139 | 0 | 0 | 0 | not replayable | H3 loading-screen rendering |
| 0x5905E0 | CLoadingScreen::Update | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x590860 | CLoadingScreen::DoPCTitleFadeIn | 3 | 0 | 0 | 0 | not replayable | H3 loading-screen rendering |
| 0x590990 | CLoadingScreen::DoPCTitleFadeOut | 3 | 3 | 0 | 3 | UNDECLARED | undeclared RW/D3D9 driver statics |
| 0x590AC0 | CLoadingScreen::DoPCScreenChange | 2 | 2 | 2 | 2 | MISMATCH | renders frames and releases splashes (H3) |
| 0x590D00 | CLoadingScreen::NewChunkLoaded | 1137 | 1000 | 2 | 5 | MISMATCH | 998/1000 identical; the 2 differing cases call DoPCScreenChange (H3) |
| 0x84F2D0 | __init_CLoadingScreen__m_aSplashes | 1 | 0 | 0 | 0 | not replayable | H3 loading-screen rendering |
| 0x8565E0 | __dtor_CLoadingScreen__m_aSplashes | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x72E570 | RpClumpGtaStreamRead1 | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x72E620 | RpClumpGtaStreamRead2 | 0 | 0 | 0 | 0 | NOCALLS |  |
| 0x730E60 | RwTextureGtaStreamRead | 2642 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x730FC0 | RwTexDictionaryGtaStreamRead | 259 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x731070 | RwTexDictionaryGtaStreamRead1 | 4 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x731150 | RwTexDictionaryGtaStreamRead2 | 4 | 0 | 0 | 0 | not replayable | H1 RW/D3D objects, heap object graphs |
| 0x537510 | FUN_00537510 | 827 | 827 | 0 | 0 | PASS |  |
| 0x537A10 | CAnimatedBuilding | 68 | 68 | 0 | 0 | PASS |  |
| 0x59EA00 | CDummyObject::CDummyObject | 1001 | 1000 | 0 | 0 | PASS |  |
