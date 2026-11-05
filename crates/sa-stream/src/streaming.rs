//! `CStreaming` (0x407100–0x40ED80, 0x48B570, 0x5B6170–0x5D29E0): the model streamer (request/loaded
//! lists threaded through `ms_aInfoForModel`, two CdStream channels, the RW object instance list,
//! zone/ped/vehicle streaming, load scene). PORT-STREAM owner A.
//!
//! Ported from the 1.0 US bytes (capstone + Ghidra over `.HOODLUM` bodies and protection chunks);
//! gta-reversed `Streaming.cpp` is a reading aid only. Deviations from gta-reversed kept here
//! (catalog/gr-deviations.csv rows and specs `CStreaming.md` / `CStreaming.deviations.md`):
//! - `ImGonnaUseStreamingMemory` / `IHaveUsedStreamingMemory` / `RemoveUnusedModelsInLoadedList` /
//!   `LoadInitialVehicles` are a lone `ret` (gr pushes/pops memory ids);
//! - `GetDefaultCabDriverModel` uses the original random formula (`rand() >= 16384` → offset 1);
//! - `GetNextFileOnCd` compares the full 32-bit position (FileID byte included) and retries itself
//!   without the priority filter (tail call) when nothing is found;
//! - `Shutdown` leaves `ms_pStreamingBuffer[0]` and `ms_pExtraObjectsDir` dangling;
//! - `ReadIniFile`: `atol`/`atof` (f64 then f32), `pe_leftx/rightx/topy/bottomy` stored as int32;
//! - `StreamPedsForInterior`: wife = husband - GetRandomNumberInRange(-3, 3) with the original helper;
//! - `RetryLoadFile` calls `CPad::StopPadsShaking` first and keeps the original error-state logic;
//! - `ConvertBufferToObject` reads the TXD slot without a null check;
//! - `DeleteRwObjectsBehindCamera` has no scan-code advance before its last pass (deferred, see below).
//!
//! Protection: several bodies read globals through computed cells (`[a] - [b]`, `[a] ^ [b]`,
//! `rol [a], [b]`); each was decoded statically from the image (the value is fixed) and the port uses
//! the decoded address. Three spots go through the no-CD indirection 0xCB73E0 (run-time value): the
//! second gang loop index of `IsCarModelNeededInCurrentZone` (18, the first loop's final index), the
//! inappropriate-cars group of `AddToLoadedVehiclesList` (0xC0E9C8) and the cheat model of
//! `StreamOneNewCar`; the values are documented per function.
//!
//! x87: coordinates are converted to sectors as `_ftol(floor(x * 0.02f + 60.0f))` with PC=24
//! (`fmul`/`fadd` rounded to 24 bits, then the CRT `floor` on the double); distances follow the
//! decompiled operation order with `sa_math::x87::X87`.

use sa_core::abi::HookReg;
use sa_core::mem::{GBool, GPtr, GameData};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::shared::{
    field, info, model_info, CStreamingInfo, INFO_FOR_MODEL, RESOURCE_ID_COL, RESOURCE_ID_DAT, RESOURCE_ID_IFP,
    RESOURCE_ID_IPL, RESOURCE_ID_RRR, RESOURCE_ID_SCM, RESOURCE_ID_TXD, STREAMING_INFO_ARRAY_BASE, TXD_POOL,
};
use crate::streaming_info as si;

/// `tStreamingFileDesc` (0x30).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StreamingFile {
    /// +0x00 `Name[40]`
    pub name: [u8; 40],
    /// +0x28 `IsNotPlayerImg`
    pub not_player_img: u8,
    pub _pad: [u8; 3],
    /// +0x2C `StreamHandle` (`FileID << 24`)
    pub handle: u32,
}
// SAFETY: bytes and an integer.
#[allow(unsafe_code)]
unsafe impl GameData for StreamingFile {}
const _: () = assert!(core::mem::size_of::<StreamingFile>() == 0x30);

impl StreamingFile {
    pub const ZERO: StreamingFile = StreamingFile { name: [0; 40], not_player_img: 0, _pad: [0; 3], handle: 0 };
}

/// `tStreamingChannel` (0x98).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StreamingChannel {
    /// +0x00 `modelIds[16]`
    pub model_ids: [i32; 16],
    /// +0x40 `modelStreamingBufferOffsets[16]`
    pub offsets: [i32; 16],
    /// +0x80 `LoadStatus` (0 idle, 1 reading, 2 large file, 3 error)
    pub load_status: i32,
    /// +0x84 `iLoadingLevel`
    pub loading_level: i32,
    /// +0x88 `offsetAndHandle`
    pub offset_and_handle: u32,
    /// +0x8C `sectorCount`
    pub sector_count: i32,
    /// +0x90 `totalTries`
    pub total_tries: i32,
    /// +0x94 `m_nCdStreamStatus`
    pub status: i32,
}
// SAFETY: integers only.
#[allow(unsafe_code)]
unsafe impl GameData for StreamingChannel {}
const _: () = assert!(core::mem::size_of::<StreamingChannel>() == 0x98);

impl StreamingChannel {
    pub const ZERO: StreamingChannel = StreamingChannel {
        model_ids: [0; 16],
        offsets: [0; 16],
        load_status: 0,
        loading_level: 0,
        offset_and_handle: 0,
        sector_count: 0,
        total_tries: 0,
        status: 0,
    };
}

sa_core::globals! {
    list GLOBALS;
    /// `CStreaming::ms_memoryAvailable`
    pub static MEMORY_AVAILABLE @ 0x8A5A80: u32 = 0x0186_A000;
    /// `CStreaming::desiredNumVehiclesLoaded`
    pub static DESIRED_NUM_VEHICLES_LOADED @ 0x8A5A84: i32 = 12;
    /// `CStreaming::ms_bLoadVehiclesInLoadScene`
    pub static LOAD_VEHICLES_IN_LOAD_SCENE @ 0x8A5A88: GBool = GBool::TRUE;
    /// `CStreaming::ms_aDefaultCopCarModel[5]` (element 4 = `ms_DefaultCopBikeModel`, 0x8A5A9C)
    pub static DEFAULT_COP_CAR_MODEL @ 0x8A5A8C: [i32; 5] = [0x257, 0x254, 0x255, 0x256, 0x20B];
    /// `CStreaming::ms_aDefaultCopModel[5]` (element 4 = `ms_DefaultCopBikerModel`, 0x8A5AB0)
    pub static DEFAULT_COP_MODEL @ 0x8A5AA0: [i32; 5] = [0x11B, 0x118, 0x119, 0x11A, 0x11C];
    /// `CStreaming::ms_aDefaultAmbulanceModel[4]`
    pub static DEFAULT_AMBULANCE_MODEL @ 0x8A5AB4: [i32; 4] = [-1, 0x1A0, 0x1A0, 0x1A0];
    /// `CStreaming::ms_aDefaultMedicModel[4]`
    pub static DEFAULT_MEDIC_MODEL @ 0x8A5AC4: [i32; 4] = [-1, 0x112, 0x114, 0x113];
    /// `CStreaming::ms_aDefaultFireEngineModel[4]`
    pub static DEFAULT_FIRE_ENGINE_MODEL @ 0x8A5AD4: [i32; 4] = [-1, 0x197, 0x197, 0x197];
    /// `CStreaming::ms_aDefaultFiremanModel[4]`
    pub static DEFAULT_FIREMAN_MODEL @ 0x8A5AE4: [i32; 4] = [-1, 0x115, 0x117, 0x116];
    /// `ms_aDefaultCabDriverModel[6]` (gr local static table)
    pub static DEFAULT_CAB_DRIVER_MODEL @ 0x8A5AF4: [i32; 6] = [0x106, 0x105, 0xDC, 0xEA, 0xB6, 0xCE];
    /// `CStreaming::ms_renderEntityLink` (`CLink<CEntity*>*`)
    pub static RENDER_ENTITY_LINK @ 0x8E48A0: GPtr<u8> = GPtr::NULL;
    /// index of GTA_INT.IMG in `ms_files` (`CStreaming::ms_gtaIntImg`)
    pub static GTA_INT_IMG @ 0x8E48A4: i32 = 0;
    /// index of GTA3.IMG in `ms_files`
    pub static GTA3_IMG @ 0x8E48A8: i32 = 0;
    /// `gRwStream` (an `RwStream`, 0x24 bytes)
    pub static RW_STREAM @ 0x8E48AC: [u32; 9] = [0; 9];
    /// `CStreaming::ms_pExtraObjectsDir` (`CDirectory*`)
    pub static EXTRA_OBJECTS_DIR @ 0x8E48D0: GPtr<u8> = GPtr::NULL;
    /// `CStreaming::ms_files[8]`
    pub static FILES @ 0x8E48D8: [StreamingFile; 8] = [StreamingFile::ZERO; 8];
    /// `CStreaming::ms_bLoadingBigModel`
    pub static LOADING_BIG_MODEL @ 0x8E4A58: GBool = GBool::FALSE;
    /// `CStreaming::ms_channel[2]`
    pub static CHANNEL @ 0x8E4A60: [StreamingChannel; 2] = [StreamingChannel::ZERO; 2];
    /// `CStreaming::ms_channelError`
    pub static CHANNEL_ERROR @ 0x8E4B90: i32 = 0;
    /// `CStreaming::ms_oldSectorY`
    pub static OLD_SECTOR_Y @ 0x8E4B94: i32 = 0;
    /// `CStreaming::ms_oldSectorX`
    pub static OLD_SECTOR_X @ 0x8E4B98: i32 = 0;
    /// `CStreaming::m_bHarvesterModelsRequested`
    pub static HARVESTER_MODELS_REQUESTED @ 0x8E4B9C: GBool = GBool::FALSE;
    /// `CStreaming::m_bStreamHarvesterModelsThisFrame`
    pub static STREAM_HARVESTER_MODELS_THIS_FRAME @ 0x8E4B9D: GBool = GBool::FALSE;
    /// `CStreaming::ms_numPriorityRequests`
    pub static NUM_PRIORITY_REQUESTS @ 0x8E4BA0: u32 = 0;
    /// `CStreaming::ms_lastCullZone`
    pub static LAST_CULL_ZONE @ 0x8E4BA4: i32 = 0;
    /// `CStreaming::ms_loadedGangCars`
    pub static LOADED_GANG_CARS @ 0x8E4BA8: u16 = 0;
    /// `CStreaming::ms_loadedGangs`
    pub static LOADED_GANGS @ 0x8E4BAC: u16 = 0;
    /// `CStreaming::ms_numPedsLoaded`
    pub static NUM_PEDS_LOADED @ 0x8E4BB0: i32 = 0;
    /// `CStreaming::ms_NextPedToLoadFromGroup[18]`
    pub static NEXT_PED_TO_LOAD_FROM_GROUP @ 0x8E4BB8: [i32; 18] = [0; 18];
    /// `CStreaming::ms_pedsLoaded[8]`
    pub static PEDS_LOADED @ 0x8E4C00: [i32; 8] = [0; 8];
    /// `CStreaming::ms_currentZoneType`
    pub static CURRENT_ZONE_TYPE @ 0x8E4C20: i32 = 0;
    /// `CStreaming::ms_vehiclesLoaded` (`CLoadedCarGroup`, 0x2E bytes)
    pub static VEHICLES_LOADED @ 0x8E4C24: [u16; 23] = [0; 23];
    /// `CStreaming::ms_pEndRequestedList`
    pub static END_REQUESTED_LIST @ 0x8E4C54: GPtr<CStreamingInfo> = GPtr::NULL;
    /// `CStreaming::ms_pStartRequestedList`
    pub static START_REQUESTED_LIST @ 0x8E4C58: GPtr<CStreamingInfo> = GPtr::NULL;
    /// `CStreaming::ms_pEndLoadedList`
    pub static END_LOADED_LIST @ 0x8E4C5C: GPtr<CStreamingInfo> = GPtr::NULL;
    /// `CStreaming::ms_startLoadedList`
    pub static START_LOADED_LIST @ 0x8E4C60: GPtr<CStreamingInfo> = GPtr::NULL;
    /// `CStreaming::ms_lastImageRead`
    pub static LAST_IMAGE_READ @ 0x8E4C64: i32 = 0;
    /// `CStreaming::ms_imageSize` (GetGTA3ImgSize, then in sectors)
    pub static IMAGE_SIZE @ 0x8E4C68: u32 = 0;
    /// `CStreaming::ms_imageOffsets[6]`
    pub static IMAGE_OFFSETS @ 0x8E4C8C: [i32; 6] = [0; 6];
    /// `CStreaming::ms_bEnableRequestListPurge`
    pub static ENABLE_REQUEST_LIST_PURGE @ 0x8E4CA4: GBool = GBool::FALSE;
    /// `CStreaming::ms_streamingBufferSize` (sectors)
    pub static STREAMING_BUFFER_SIZE @ 0x8E4CA8: u32 = 0;
    /// `CStreaming::ms_pStreamingBuffer[2]`
    pub static STREAMING_BUFFER @ 0x8E4CAC: [GPtr<u8>; 2] = [GPtr::NULL; 2];
    /// `CStreaming::ms_memoryUsedBytes`
    pub static MEMORY_USED @ 0x8E4CB4: u32 = 0;
    /// `CStreaming::ms_numModelsRequested`
    pub static NUM_MODELS_REQUESTED @ 0x8E4CB8: i32 = 0;
    /// `CStreaming::ms_disableStreaming`
    pub static DISABLE_STREAMING @ 0x9654B0: GBool = GBool::FALSE;
    /// `CStreaming::ms_bIsInitialised`
    pub static IS_INITIALISED @ 0x9654B8: i32 = 0;
    /// `CStreaming::m_bBoatsNeeded`
    pub static BOATS_NEEDED @ 0x9654BC: GBool = GBool::FALSE;
    /// `CStreaming::ms_bLoadingScene`
    pub static LOADING_SCENE @ 0x9654BD: GBool = GBool::FALSE;
    /// `CStreaming::m_bCopBikeLoaded`
    pub static COP_BIKE_LOADED @ 0x9654BE: GBool = GBool::FALSE;
    /// `CStreaming::m_bDisableCopBikes`
    pub static DISABLE_COP_BIKES @ 0x9654BF: GBool = GBool::FALSE;
    /// `CStreaming::ms_nTimePassedSinceLastCopBikeStreamedIn`
    pub static TIME_PASSED_SINCE_LAST_COP_BIKE @ 0x9654C0: u32 = 0;
    /// `CStreaming::ms_bReadLayerForceFully` (= `m_bModelStreamNotLoaded`)
    pub static READ_LAYER_FORCE_FULLY @ 0x9654C4: GBool = GBool::FALSE;
    /// written by `ForceLayerToRead`, never read
    pub static FORCE_LAYER_ARG @ 0x9654C8: i32 = 0;
    /// `timeBeforeNextLoad` (StreamZoneModels)
    pub static TIME_BEFORE_NEXT_LOAD @ 0x9654CC: i32 = 0;
    /// `timeBeforeNextGangLoad` (StreamZoneModels)
    pub static TIME_BEFORE_NEXT_GANG_LOAD @ 0x9654D0: i32 = 0;
    /// `CurrentGangMemberToLoad`
    pub static CURRENT_GANG_MEMBER_TO_LOAD @ 0x9654D4: i32 = 0;
    /// `CStreaming::ms_rwObjectInstances` (`CLinkList<CEntity*>`, 0x34 bytes)
    pub static RW_OBJECT_INSTANCES @ 0x9654F0: [u32; 13] = [0; 13];
    /// `s_LastRandomIndex` (GetDefaultCabDriverModel)
    pub static LAST_RANDOM_CAB_INDEX @ 0x965524: i32 = 0;
    /// `lastCarModelStreamedIn` (StreamOneNewCar)
    pub static LAST_CAR_MODEL_STREAMED_IN @ 0x965528: i32 = 0;
    /// `lastZonePopulationType` (StreamVehiclesAndPeds_Always)
    pub static LAST_ZONE_POPULATION_TYPE @ 0x96552C: i32 = 0;
    /// `framesBeforeStreamingNextNewCar` (StreamVehiclesAndPeds)
    pub static FRAMES_BEFORE_NEXT_NEW_CAR @ 0x965530: i32 = 0;
    /// `currentChannel` (LoadRequestedModels)
    pub static CURRENT_CHANNEL @ 0x965534: i32 = 0;
    /// `CStreaming::m_bLoadingAllRequestedModels`
    pub static LOADING_ALL_REQUESTED_MODELS @ 0x965538: GBool = GBool::FALSE;
}

// ------------------------------------------------------------------------- foreign addresses ---

/// `TheCamera` (0xB6F028): `+0x14` matrix pointer (0xB6F03C), `+0x4` placement position (0xB6F02C),
/// `+0xF0` LOD distance multiplier (0xB6F118), `+0x954` `m_pRwCamera` (0xB6F97C).
const CAMERA: usize = 0xB6F028;
/// `CTheZones::m_CurrLevel` (0xBA6718)
const ZONES_CURR_LEVEL: usize = 0xBA6718;
/// `CGame::currArea` (0xB72914)
fn curr_area() -> i32 {
    sa_core::game::CURR_AREA.get()
}
/// `CWorld::ms_nCurrentScanCode` (0xB7CD78)
const SCAN_CODE: usize = 0xB7CD78;
/// `CWorld::ms_aSectors` (120 × 120 × 8: buildings, dummies) and `ms_aRepeatSectors` (16 × 16 × 0xC:
/// vehicles, peds, objects) — the objects list of a repeat sector is at 0xB992C0.
const SECTORS: usize = 0xB7D0B8;
const REPEAT_SECTOR_OBJECTS: usize = 0xB992C0;
/// `CWorld::ms_aLodPtrLists` (30 × 30 × 4) at 0xB99EB8
const LOD_LISTS: usize = 0xB99EB8;
/// `CPopulation::m_pCurrZoneInfo` (0xC0BC68)
const POP_CURR_ZONE_INFO: usize = 0xC0BC68;
/// `CPopulation::m_nNumPedsInGroup` (i16, 0xC0ECC0), `m_PedGroups` (u16 × 21, 0xC0F358)
const PED_GROUP_COUNT: usize = 0xC0ECC0;
const PED_GROUPS: usize = 0xC0F358;
/// `CPopulation::m_nNumCarsInGroup` (i16, 0xC0EC9C), `m_CarGroups` (u16 × 23, 0xC0F074)
const CAR_GROUP_COUNT: usize = 0xC0EC9C;
const CAR_GROUPS: usize = 0xC0F074;
/// `CPopulation::m_LoadedBoats` / `m_InAppropriateLoadedCars` / `m_AppropriateLoadedCars` /
/// `m_LoadedGangCars[10]` (stride 0x2E)
const LOADED_BOATS: usize = 0xC0E998;
const INAPPROPRIATE_LOADED_CARS: usize = 0xC0E9C8;
const APPROPRIATE_LOADED_CARS: usize = 0xC0E9F8;
const LOADED_GANG_CARS_GROUPS: usize = 0xC0EA28;
/// `CPopulation::CurrentWorldZone` (0xC0FCBC)
const POP_CURRENT_WORLD_ZONE: usize = 0xC0FCBC;
/// ped group ids per world zone (`CPopulation::m_TranslationArray`, 3 dwords per entry): gangs from
/// 0x8D2618 (stride 0xC, 10 entries), dealers 0x8D2690, shopkeepers 0x8D269C, office 0x8D26A8,
/// husbands 0x8D26B4, wives 0x8D26C0.
const GANG_GROUP_IDS: usize = 0x8D2618;
const DEALER_GROUP_ID: usize = 0x8D2690;
const SHOPKEEPER_GROUP_IDS: usize = 0x8D269C;
const OFFICE_GROUP_IDS: usize = 0x8D26A8;
const HUSBAND_GROUP_IDS: usize = 0x8D26B4;
const WIFE_GROUP_IDS: usize = 0x8D26C0;
/// `CAnimManager::ms_aAnimBlocks` (stride 0x20, `bLoaded` at +0x10 → 0xB5D4B0)
const ANIM_BLOCK_LOADED: usize = 0xB5D4B0;
/// `CCutsceneMgr::ms_cutsceneProcessing` (0xB5F852)
const CUTSCENE_PROCESSING: usize = 0xB5F852;
/// `CGame::CanSeeOutSideFromCurrArea`-style flag `CGame::bMissionPackGame`? no: `CRenderer::m_loadingPriority`
/// (0xB76850) — the "very busy" flag read by IsVeryBusy/Update.
const RENDERER_LOADING_PRIORITY: usize = 0xB76850;
/// `CTimer::m_UserPause` / `m_CodePause` (0xB7CB49 / 0xB7CB48), `m_FrameCounter` (0xB7CB4C)
const TIMER_USER_PAUSE: usize = 0xB7CB49;
const TIMER_CODE_PAUSE: usize = 0xB7CB48;
const TIMER_FRAME_COUNTER: usize = 0xB7CB4C;
/// `CRenderer::ms_bRenderOutsideTunnels`?? (0xB745C1, gr `CRenderer::m_loadingPriority`-adjacent flag
/// tested by Update before `AddModelsToRequestList`)
const RENDERER_FLAG_B745C1: usize = 0xB745C1;
/// `CWorld::Players[0].m_pPed` (0xB7CD98) and `FindPlayerPed` (0x56E210)
const PLAYER0_PED: usize = 0xB7CD98;
/// `CTheScripts::bPlayerIsOffTheMap`? `CGame::bMissionPackGame`: `0xA43088` (Update)
const FLAG_A43088: usize = 0xA43088;
/// `CStreaming::ms_pedsLoaded` special: `CWorld::ms_pPlayerTargetEntity`? no: the camera-target entity
/// used by Update (0xB7CE48, `CTheScripts::pActiveScripts`-unrelated "load at entity" pointer)
const UPDATE_FOCUS_ENTITY: usize = 0xB7CE48;
/// harvester model table (4 × i32 at 0x8CD6E4)
const HARVESTER_MODELS: usize = 0x8CD6E4;
/// `CScriptsForBrains`-unrelated: streamed script slots `loaded` bytes (`CStreamedScripts::m_aScripts`,
/// stride 0x20, +0x?? at 0xA47B64)
const STREAMED_SCRIPT_LOADED: usize = 0xA47B64;
/// x87 constants of the image
const F_0_02: u32 = 0x3CA3_D70A; // 0x858B38
const F_60: u32 = 0x4270_0000; // 0x858B34
const F_3: u32 = 0x4040_0000; // 0x858B3C
const F_40: u32 = 0x4220_0000; // 0x858A10
const F_80: u32 = 0x42A0_0000; // 0x858A14
const F_50: u32 = 0x4248_0000; // 0x858B40 (load scene height margin)
const F_INV_32768: u32 = 0x3800_0000; // 0x858B14
const F_M2: u32 = 0xC000_0000; // 0x858B18
const F_6: u32 = 0x40C0_0000; // 0x858B44

fn g<T: GameData>(addr: usize) -> GPtr<T> {
    GPtr::from_addr(addr)
}
fn k(bits: u32) -> f32 {
    f32::from_bits(bits)
}
fn x(v: f32) -> X87 {
    X87::from_f32(v)
}

orig! {
    // --- CdStream (owner M)
    /// `CdStreamOpen(const char*, bool locked)` (0x4067B0)
    pub fn cd_stream_open(path: GPtr<u8>, locked: u8) -> u32 = cdecl 0x4067B0 => unported;
    /// `CdStreamRead(channel, buffer, offset, sectors)` (0x406A20)
    pub fn cd_stream_read(ch: i32, buf: GPtr<u8>, pos: u32, sectors: i32) -> i32 = cdecl 0x406A20 => unported;
    /// `CdStreamGetStatus(channel)` (0x4063E0)
    pub fn cd_stream_get_status(ch: i32) -> i32 = cdecl 0x4063E0 => unported;
    /// `CdStreamSync(channel)` (0x406460)
    pub fn cd_stream_sync(ch: i32) -> i32 = cdecl 0x406460 => unported;
    /// `CdStreamGetLastPosn()` (0x406450)
    pub fn cd_stream_get_last_posn() -> u32 = cdecl 0x406450 => unported;
    /// `GetGTA3ImgSize()` (0x406360)
    pub fn get_gta3_img_size() -> u32 = cdecl 0x406360 => unported;
    // --- CLoadedCarGroup (owner M)
    /// `CLoadedCarGroup::Clear` (0x611B90)
    pub fn car_group_clear(g: GPtr<u8>) = thiscall 0x611B90 => unported;
    /// `CLoadedCarGroup::AddMember` (0x611BB0)
    pub fn car_group_add_member(g: GPtr<u8>, model: i32) = thiscall 0x611BB0 => unported;
    /// `CLoadedCarGroup::RemoveMember` (0x611BD0)
    pub fn car_group_remove_member(g: GPtr<u8>, model: i32) = thiscall 0x611BD0 => unported;
    /// `CLoadedCarGroup::GetMember` (0x611C20)
    pub fn car_group_get_member(g: GPtr<u8>, i: i32) -> i32 = thiscall 0x611C20 => unported;
    /// `CLoadedCarGroup::CountMembers` (0x611C30)
    pub fn car_group_count_members(g: GPtr<u8>) -> i32 = thiscall 0x611C30 => unported;
    /// `CLoadedCarGroup::SortBasedOnUsage` (0x611E10)
    pub fn car_group_sort_based_on_usage(g: GPtr<u8>) = thiscall 0x611E10 => unported;
    /// `CLoadedCarGroup::PickLeastUsedModel(int32 minRefs)` (0x611E90)
    pub fn car_group_pick_least_used_model(g: GPtr<u8>, min_refs: i32) -> i32 = thiscall 0x611E90 => unported;
    /// `CLoadedCarGroup::RemoveMember` alias used by RemoveCarModel (0x6119D0: `CPopulation::RemoveSpecificDriverModelsForCar`)
    pub fn population_remove_specific_driver_models_for_car(model: i32) = cdecl 0x6119D0 => unported;
    // --- CTxdStore (owner M)
    /// `CTxdStore::GetTxd(slot)` (0x408340)
    pub fn txd_get_txd(slot: i32) -> GPtr<u8> = cdecl 0x408340 => crate::txdstore::get_txd;
    /// `CTxdStore::GetParentTxdSlot(slot)` (0x408370)
    pub fn txd_get_parent_txd_slot(slot: i32) -> i32 = cdecl 0x408370 => crate::txdstore::get_parent_txd_slot;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    pub fn txd_find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => crate::txdstore::find_txd_slot_name;
    /// `CTxdStore::AddTxdSlot(const char*)` (0x731C80)
    pub fn txd_add_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731C80 => crate::txdstore::add_txd_slot;
    /// `CTxdStore::SetCurrentTxd(slot)` (0x7319C0)
    pub fn txd_set_current_txd(slot: i32) = cdecl 0x7319C0 => crate::txdstore::set_current_txd;
    /// `CTxdStore::AddRef(slot)` (0x731A00)
    pub fn txd_add_ref(slot: i32) = cdecl 0x731A00 => crate::txdstore::add_ref;
    /// `CTxdStore::RemoveRefWithoutDelete(slot)` (0x731A70)
    pub fn txd_remove_ref_without_delete(slot: i32) = cdecl 0x731A70 => crate::txdstore::remove_ref_without_delete;
    /// `CTxdStore::GetNumRefs(slot)` (0x731AA0)
    pub fn txd_get_num_refs(slot: i32) -> i32 = cdecl 0x731AA0 => crate::txdstore::get_num_refs;
    /// `CTxdStore::RemoveTxd(slot)` (0x731E90)
    pub fn txd_remove_txd(slot: i32) = cdecl 0x731E90 => crate::txdstore::remove_txd;
    /// `CTxdStore::StartLoadTxd(slot, RwStream*)` (0x731930)
    pub fn txd_start_load_txd(slot: i32, stream: GPtr<u8>) -> u8 = cdecl 0x731930 => unported;
    /// `CTxdStore::LoadTxd(slot, RwStream*)` (0x731DD0)
    pub fn txd_load_txd_stream(slot: i32, stream: GPtr<u8>) -> u8 = cdecl 0x731DD0 => crate::txdstore::load_txd_stream;
    /// `CTxdStore::FinishLoadTxd(slot, RwStream*)` (0x731E40)
    pub fn txd_finish_load_txd(slot: i32, stream: GPtr<u8>) -> u8 = cdecl 0x731E40 => crate::txdstore::finish_load_txd;
    /// `CVehicleModelInfo::AssignRemapTxd(const char*, int16 slot)` (0x4C9360)
    pub fn vehicle_assign_remap_txd(name: GPtr<u8>, slot: i32) = cdecl 0x4C9360 => unported;
    // --- CIplStore (owner M)
    /// `CIplStore::FindIplSlot(const char*)` (0x404AC0)
    pub fn ipl_find_ipl_slot(name: GPtr<u8>) -> i32 = cdecl 0x404AC0 => crate::iplstore::find_ipl_slot;
    /// `CIplStore::AddIplSlot(const char*)` (0x405AC0)
    pub fn ipl_add_ipl_slot(name: GPtr<u8>) -> i32 = cdecl 0x405AC0 => crate::iplstore::add_ipl_slot;
    /// `CIplStore::RemoveIpl(slot)` (0x404B20)
    pub fn ipl_remove_ipl(slot: i32) = cdecl 0x404B20 => crate::iplstore::remove_ipl;
    /// `CIplStore::LoadIpl(slot, data, size)` (0x406080)
    pub fn ipl_load_ipl(slot: i32, data: GPtr<u8>, size: i32) -> u8 = cdecl 0x406080 => crate::iplstore::load_ipl;
    /// `CIplStore::LoadIpls(CVector, bool)` (0x405170)
    pub fn ipl_load_ipls(px: f32, py: f32, pz: f32, avoid_loading_in_player_vehicle: u8) = cdecl 0x405170 => crate::iplstore::load_ipls;
    /// `CIplStore::EnsureIplsAreInMemory(const CVector&)` (0x4053F0)
    pub fn ipl_ensure_ipls_are_in_memory(pos: GPtr<f32>) = cdecl 0x4053F0 => crate::iplstore::ensure_ipls_are_in_memory;
    /// `CIplStore::AddIplsNeededAtPosn(const CVector&)` (0x4045B0)
    pub fn ipl_add_ipls_needed_at_posn(pos: GPtr<f32>) = cdecl 0x4045B0 => unported;
    // --- CColStore (owner A, this crate)
    /// `CColStore::AddColSlot(const char*)` (0x411140)
    pub fn col_add_col_slot(name: GPtr<u8>) -> i32 = cdecl 0x411140 => crate::colstore::add_col_slot;
    /// `CColStore::FindColSlot(const char*)` (0x410390; not hookable, the original is called)
    pub fn col_find_col_slot(name: GPtr<u8>) -> i32 = cdecl 0x410390 => crate::colstore::find_col_slot;
    /// `CColStore::RemoveCol(slot)` (0x410730)
    pub fn col_remove_col(slot: i32) = cdecl 0x410730 => crate::colstore::remove_col;
    /// `CColStore::LoadCol(slot, data, size)` (0x4106D0)
    pub fn col_load_col(slot: i32, data: GPtr<u8>, size: i32) -> bool = cdecl 0x4106D0 => crate::colstore::load_col_buffer;
    /// `CColStore::LoadCollision(CVector, bool)` (0x410860)
    pub fn col_load_collision(px: f32, py: f32, pz: f32, ignore: u8) = cdecl 0x410860 => crate::colstore::load_collision;
    /// `CColStore::EnsureCollisionIsInMemory(const CVector&)` (0x410AD0)
    pub fn col_ensure_collision_is_in_memory(pos: GPtr<f32>) = cdecl 0x410AD0 => crate::colstore::ensure_collision_is_in_memory;
    /// `CColStore::AddCollisionNeededAtPosn(const CVector&)` (0x4103A0)
    pub fn col_add_collision_needed_at_posn(pos: GPtr<f32>) = cdecl 0x4103A0 => crate::colstore::add_collision_needed_at_posn;
    // --- model infos (owners B/C)
    /// `CModelInfo::GetModelInfo(const char*, int32* index)` (0x4C5940)
    pub fn mi_get_model_info_by_name(name: GPtr<u8>, index: GPtr<i32>) -> GPtr<u8> = cdecl 0x4C5940 => unported;
    /// `CVehicleModelInfo::ms_linkedUpgrades.FindOtherUpgrade(model)` (0x4C74D0)
    pub fn linked_upgrades_find_other_upgrade(list: GPtr<u8>, model: i32) -> i16 = thiscall 0x4C74D0 => unported;
    /// `CBaseModelInfo::SetTexDictionary(const char*)` (0x4C4B40)
    pub fn mi_set_tex_dictionary(mi: GPtr<u8>, name: GPtr<u8>) = thiscall 0x4C4B40 => unported;
    /// `CBaseModelInfo::ClearTexDictionary()` (0x4C4B70)
    pub fn mi_clear_tex_dictionary(mi: GPtr<u8>) = thiscall 0x4C4B70 => unported;
    /// `CBaseModelInfo::RemoveRef()` (0x4C4BB0)
    pub fn mi_remove_ref(mi: GPtr<u8>) = thiscall 0x4C4BB0 => unported;
    // --- CFileLoader (owner C)
    /// `CFileLoader::LoadAtomicFile(RwStream*, uint32 model)` (0x5371F0)
    pub fn fl_load_atomic_file(stream: GPtr<u8>, model: i32) -> u8 = cdecl 0x5371F0 => unported;
    /// `CFileLoader::LoadClumpFile(RwStream*, uint32 model)` (0x5372D0)
    pub fn fl_load_clump_file(stream: GPtr<u8>, model: i32) -> u8 = cdecl 0x5372D0 => unported;
    /// `RpClumpGtaCancelStream()` (0x72E700)
    pub fn rp_clump_gta_cancel_stream() = cdecl 0x72E700 => unported;
    // --- RenderWare (not ported)
    /// `_rwStreamInitialize(RwStream*, rwOwned, type, access, data)` (0x7EC810)
    pub fn rw_stream_initialize(s: GPtr<u8>, owned: i32, kind: i32, access: i32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EC810 => unported;
    /// `RwStreamClose(RwStream*, void* data)` (0x7ECE20)
    pub fn rw_stream_close(s: GPtr<u8>, data: GPtr<u8>) -> i32 = cdecl 0x7ECE20 => unported;
    /// `RwStreamReadChunkHeaderInfo(RwStream*, RwChunkHeaderInfo*)` (0x7ED590)
    pub fn rw_stream_read_chunk_header_info(s: GPtr<u8>, info: GPtr<u8>) -> i32 = cdecl 0x7ED590 => unported;
    /// `RtDictSchemaStreamReadDict(schema, stream)` (0x7CF240)
    pub fn rt_dict_schema_stream_read_dict(schema: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> = cdecl 0x7CF240 => unported;
    /// `RtDictSchemaSetCurrentDict(schema, dict)` (0x7CEEF0)
    pub fn rt_dict_schema_set_current_dict(schema: GPtr<u8>, dict: GPtr<u8>) -> GPtr<u8> = cdecl 0x7CEEF0 => unported;
    /// `RtDictDestroy(dict)` (0x7CF130)
    pub fn rt_dict_destroy(dict: GPtr<u8>) -> i32 = cdecl 0x7CF130 => unported;
    /// `CMemoryMgr::MallocAlign(size, align, hint)` (0x72F4C0)
    pub fn memory_malloc_align(size: u32, align: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F4C0 => unported;
    /// `CMemoryMgr::FreeAlign(p)` (0x72F4F0)
    pub fn memory_free_align(p: GPtr<u8>) = cdecl 0x72F4F0 => unported;
    // --- CRT
    /// CRT `rand` (0x821B1E)
    pub fn rand() -> i32 = cdecl 0x821B1E => sa_core::crt::rand_owned;
    /// CRT `_stricmp` (0x8229B6)
    pub fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// CRT `_strnicmp` (0x835AC7)
    pub fn strnicmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x835AC7 => unported;
    /// CRT `sscanf(s, "%d", int*)` (0x8220AD)
    pub fn sscanf_int(s: GPtr<u8>, fmt: GPtr<u8>, out: GPtr<i32>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `operator new` (0x82119A)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CRT `free` (0x8214BD)
    pub fn free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CFileMgr::CloseFile(FILE*)` (0x5389D0)
    pub fn fclose(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => sa_core::filemgr::close_file;
    /// CRT `strtok` (0x82244B)
    pub fn strtok(s: GPtr<u8>, delim: GPtr<u8>) -> GPtr<u8> = cdecl 0x82244B => unported;
    /// CRT `atol` (0x82258E)
    pub fn atol(s: GPtr<u8>) -> i32 = cdecl 0x82258E => unported;
    /// CRT `atof` (0x823CEE)
    pub fn atof(s: GPtr<u8>) -> f64 = cdecl 0x823CEE => unported;
    // --- core / misc
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    pub fn file_loader_load_line(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x536F80 => crate::fileloader::load_line_file;
    /// `CFileMgr::Read(FILE*, buf, size)` (0x538950)
    pub fn file_read(f: GPtr<u8>, buf: GPtr<u8>, size: u32) -> u32 = cdecl 0x538950 => sa_core::filemgr::read;
    /// `CDirectory::AddItem(const DirectoryInfo&)` (0x532310)
    pub fn directory_add_item(dir: GPtr<sa_core::directory::CDirectory>, info: GPtr<sa_core::directory::DirectoryInfo>) = thiscall 0x532310 => sa_core::directory::add_item;
    /// `CDirectory::FindItem(name, pos&, size&)` (0x5324A0)
    pub fn directory_find_item(dir: GPtr<sa_core::directory::CDirectory>, name: GPtr<u8>, pos: GPtr<u32>, size: GPtr<u32>) -> bool = thiscall 0x5324A0 => sa_core::directory::find_item_by_name;
    /// `CDirectory::CDirectory(capacity)` (0x5322A0)
    pub fn directory_ctor(dir: GPtr<sa_core::directory::CDirectory>, capacity: u32) -> GPtr<sa_core::directory::CDirectory> = thiscall 0x5322A0 => sa_core::directory::constructor_capacity;
    /// `CDirectory::~CDirectory()` (0x5322D0)
    pub fn directory_dtor(dir: GPtr<sa_core::directory::CDirectory>) = thiscall 0x5322D0 => sa_core::directory::destructor;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    pub fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => sa_core::keygen::get_uppercase_key;
    /// `CClock::GetIsTimeInRange(uint8 from, uint8 to)` (0x52CEE0)
    pub fn clock_get_is_time_in_range(from: u8, to: u8) -> u8 = cdecl 0x52CEE0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, include_remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `FindPlayerCoors(CVector* out, int32 id)` (0x56E010)
    pub fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => unported;
    /// `FindPlayerWanted(int32)` (0x56E230)
    pub fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CPad::StopPadsShaking()` (0x541D70)
    pub fn pad_stop_pads_shaking() = cdecl 0x541D70 => unported;
    /// `CLoadingScreen::Pause()` / `Continue()` (0x590310 / 0x590320)
    pub fn loading_screen_pause() = cdecl 0x590310 => unported;
    pub fn loading_screen_continue() = cdecl 0x590320 => crate::loading_screen::continue_;
    /// `CPopulation::DoesCarGroupHaveModelId(group, model)` (0x406F50)
    pub fn population_does_car_group_have_model_id(group: i32, model: i32) -> u8 = cdecl 0x406F50 => unported;
    /// `CTheZones::GetLevelFromPosition(const CVector&)` (0x572300)
    pub fn zones_get_level_from_position(pos: GPtr<f32>) -> i32 = cdecl 0x572300 => unported;
    /// `CPopCycle::Update`-free: `CTheZones::GetZoneInfo(const CVector&, CZone**)` (0x572400)
    pub fn zones_get_zone_info(pos: GPtr<f32>, zone: GPtr<u8>) -> GPtr<u8> = cdecl 0x572400 => unported;
    /// `CPopulation::ManageAllPopulation()` (0x6160A0)
    pub fn population_manage_all_population() = cdecl 0x6160A0 => unported;
    /// `CTimer::Suspend()` / `Resume()`
    pub fn timer_suspend() = cdecl 0x5619D0 => sa_core::timer::suspend;
    pub fn timer_resume() = cdecl 0x561A00 => sa_core::timer::resume;
    /// `CEntity::LivesInThisNonOverlapSector(int32, int32)` (0x533050, thiscall)
    pub fn entity_lives_in_this_non_overlap_sector(e: GPtr<u8>, sx: i32, sy: i32) -> u8 = thiscall 0x533050 => unported;
    /// `CEntity::IsVisibleOnScreen`? `CEntity::GetIsOnScreen()` (0x534540, thiscall)
    pub fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `CRenderer::ShouldModelBeStreamed(CEntity*, const CVector&, float)` (0x554EB0)
    pub fn renderer_should_model_be_streamed(e: GPtr<u8>, pos: GPtr<f32>, far: f32) -> u8 = cdecl 0x554EB0 => unported;
    // --- virtual calls
    /// `CEntity::CreateRwObject()` (vtable +0x1C)
    pub fn entity_create_rw_object(e: GPtr<u8>) = thiscall vslot 0x1C => unported;
    /// `CEntity::DeleteRwObject()` (vtable +0x20)
    pub fn entity_delete_rw_object(e: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// `CBaseModelInfo::AsAtomicModelInfoPtr()` (vtable +0x04)
    pub fn mi_as_atomic(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x04 => unported;
    /// `CBaseModelInfo::GetModelType()` (vtable +0x10, AL)
    pub fn mi_get_model_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
    /// `CBaseModelInfo::GetTimeInfo()` (vtable +0x14)
    pub fn mi_get_time_info(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x14 => unported;
    /// `CBaseModelInfo::DeleteRwObject()` (vtable +0x20)
    pub fn mi_delete_rw_object(mi: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// `CBaseModelInfo::GetRwModelType()` (vtable +0x24)
    pub fn mi_get_rw_model_type(mi: GPtr<u8>) -> i32 = thiscall vslot 0x24 => unported;
    /// `CBaseModelInfo::GetAnimFileIndex()` (vtable +0x38)
    pub fn mi_get_anim_file_index(mi: GPtr<u8>) -> i32 = thiscall vslot 0x38 => unported;
    // --- this module (called, not inlined, by the original)
    /// `CStreaming::RequestModel(id, flags)` (0x4087E0)
    pub fn call_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => request_model;
    /// `CStreaming::RemoveModel(id)` (0x4089A0)
    pub fn call_remove_model(id: i32) = cdecl 0x4089A0 => remove_model;
    /// `CStreaming::RequestTxdModel(slot, flags)` (0x407100)
    pub fn call_request_txd_model(slot: i32, flags: i32) = cdecl 0x407100 => request_txd_model;
    /// `CStreaming::SetModelIsDeletable(id)` (0x409C10)
    pub fn call_set_model_is_deletable(id: i32) = cdecl 0x409C10 => set_model_is_deletable;
    /// `CStreaming::SetMissionDoesntRequireModel(id)` (0x409C90)
    pub fn call_set_mission_doesnt_require_model(id: i32) = cdecl 0x409C90 => set_mission_doesnt_require_model;
    /// `CStreaming::GetNextFileOnCd(pos, notPriority)` (0x408E20)
    pub fn call_get_next_file_on_cd(pos: u32, priority: u8) -> i32 = cdecl 0x408E20 => get_next_file_on_cd;
    /// `CStreaming::RequestModelStream(channel)` (0x40CBA0)
    pub fn call_request_model_stream(ch: i32) = cdecl 0x40CBA0 => request_model_stream;
    /// `CStreaming::ProcessLoadingChannel(channel)` (0x40E170)
    pub fn call_process_loading_channel(ch: i32) -> bool = cdecl 0x40E170 => process_loading_channel;
    /// `CStreaming::FlushChannels()` (0x40E460)
    pub fn call_flush_channels() = cdecl 0x40E460 => flush_channels;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    pub fn call_load_all_requested_models(only_priority: u8) = cdecl 0x40EA10 => load_all_requested_models;
    /// `CStreaming::RemoveLeastUsedModel(flags)` (0x40CFD0)
    pub fn call_remove_least_used_model(flags: u32) -> bool = cdecl 0x40CFD0 => remove_least_used_model;
    /// `CStreaming::DeleteRwObjectsBehindCamera(memory)` (0x40D7C0, deferred: always the original)
    pub fn call_delete_rw_objects_behind_camera(memory: i32) = cdecl 0x40D7C0 => unported;
    /// `CStreaming::RemoveLoadedVehicle()` (0x40C020)
    pub fn call_remove_loaded_vehicle() -> bool = cdecl 0x40C020 => remove_loaded_vehicle;
    /// `CStreaming::RemoveLoadedZoneModel()` (0x40B340)
    pub fn call_remove_loaded_zone_model() -> bool = cdecl 0x40B340 => remove_loaded_zone_model;
    /// `CStreaming::DeleteLeastUsedEntityRwObject(bool notOnScreen, uint32 flags)` (0x409760)
    pub fn call_delete_least_used_entity_rw_object(not_on_screen: u8, flags: u32) -> bool = cdecl 0x409760 => delete_least_used_entity_rw_object;
    /// `CStreaming::AddToLoadedVehiclesList(model)` (0x408000)
    pub fn call_add_to_loaded_vehicles_list(model: i32) -> bool = cdecl 0x408000 => add_to_loaded_vehicles_list;
    /// `CStreaming::IsCarModelNeededInCurrentZone(model)` (0x407DD0)
    pub fn call_is_car_model_needed_in_current_zone(model: i32) -> bool = cdecl 0x407DD0 => is_car_model_needed_in_current_zone;
    /// `CStreaming::AreTexturesUsedByRequestedModels(txd)` (0x409A90)
    pub fn call_are_textures_used_by_requested_models(txd: i32) -> bool = cdecl 0x409A90 => are_textures_used_by_requested_models;
    /// `CStreaming::AreAnimsUsedByRequestedModels(anim)` (0x407AD0)
    pub fn call_are_anims_used_by_requested_models(anim: i32) -> bool = cdecl 0x407AD0 => are_anims_used_by_requested_models;
    /// `CStreaming::FinishLoadingLargeFile(buf, id)` (0x408CB0)
    pub fn call_finish_loading_large_file(buf: GPtr<u8>, id: i32) -> bool = cdecl 0x408CB0 => finish_loading_large_file;
    /// `CStreaming::ConvertBufferToObject(buf, id, channel)` (0x40C6B0)
    pub fn call_convert_buffer_to_object(buf: GPtr<u8>, id: i32) -> bool = cdecl 0x40C6B0 => convert_buffer_to_object;
    /// `CStreaming::MakeSpaceFor(size)` (0x40E120)
    pub fn call_make_space_for(size: i32) = cdecl 0x40E120 => make_space_for;
    /// `CStreaming::RetryLoadFile(channel)` (0x4076C0)
    pub fn call_retry_load_file(ch: i32) = cdecl 0x4076C0 => retry_load_file;
    /// `CStreaming::RemoveTxdModel(slot)` (0x40C180)
    pub fn call_remove_txd_model(slot: i32) = cdecl 0x40C180 => remove_txd_model;
    /// `CStreaming::RequestModelStream`-sibling `StreamOneNewCar()` (0x40B4F0)
    pub fn call_stream_one_new_car() = cdecl 0x40B4F0 => stream_one_new_car;
    /// `CStreaming::StreamCopModels(level)` (0x40A150)
    pub fn call_stream_cop_models(level: i32) = cdecl 0x40A150 => stream_cop_models;
    /// `CStreaming::ClearSlots(n)` (0x40BAA0)
    pub fn call_clear_slots(n: i32) = cdecl 0x40BAA0 => clear_slots;
    /// `CStreaming::StreamZoneModels_Gangs(const CVector&)` (0x40AA10)
    pub fn call_stream_zone_models_gangs(pos: GPtr<f32>) = cdecl 0x40AA10 => stream_zone_models_gangs;
    /// `CStreaming::ReclassifyLoadedCars()` (0x40AFA0)
    pub fn call_reclassify_loaded_cars() = cdecl 0x40AFA0 => reclassify_loaded_cars;
    /// `CStreaming::StreamVehiclesAndPeds_Always(const CVector&)` (0x40B650)
    pub fn call_stream_vehicles_and_peds_always(pos: GPtr<f32>) = cdecl 0x40B650 => stream_vehicles_and_peds_always;
    /// `CStreaming::StreamVehiclesAndPeds()` (0x40B700)
    pub fn call_stream_vehicles_and_peds() = cdecl 0x40B700 => stream_vehicles_and_peds;
    /// `CStreaming::StreamZoneModels(const CVector&)` (0x40A560, deferred)
    pub fn call_stream_zone_models(pos: GPtr<f32>) = cdecl 0x40A560 => unported;
    /// `CStreaming::IsVeryBusy()` (0x4076A0)
    pub fn call_is_very_busy() -> u32 = cdecl 0x4076A0 => is_very_busy;
    /// `CStreaming::LoadLoadRequestedModels` (0x40E3A0)
    pub fn call_load_requested_models() = cdecl 0x40E3A0 => load_requested_models;
    /// `CStreaming::AddLodsToRequestList(const CVector&, flags)` (0x40C520)
    pub fn call_add_lods_to_request_list(pos: GPtr<f32>, flags: i32) = cdecl 0x40C520 => add_lods_to_request_list;
    /// `CStreaming::AddModelsToRequestList(const CVector&, flags)` (0x40D3F0, deferred)
    pub fn call_add_models_to_request_list(pos: GPtr<f32>, flags: i32) = cdecl 0x40D3F0 => unported;
    /// `CStreaming::DeleteAllRwObjects()` (0x4090A0)
    pub fn call_delete_all_rw_objects() = cdecl 0x4090A0 => delete_all_rw_objects;
    /// `CStreaming::RequestBigBuildings(const CVector&)` (0x409430)
    pub fn call_request_big_buildings(pos: GPtr<f32>) = cdecl 0x409430 => request_big_buildings;
    /// `CStreaming::InstanceLoadedModels(const CVector&)` (0x4084F0)
    pub fn call_instance_loaded_models(pos: GPtr<f32>) = cdecl 0x4084F0 => instance_loaded_models;
    /// `CStreaming::RemoveAllUnusedModels()` (0x40CF80)
    pub fn call_remove_all_unused_models() = cdecl 0x40CF80 => remove_all_unused_models;
    /// `CStreaming::FlushRequestList()` (0x40E4E0)
    pub fn call_flush_request_list() = cdecl 0x40E4E0 => flush_request_list;
    /// `CStreaming::RemoveCarModel(model)` (0x4080F0)
    pub fn call_remove_car_model(model: i32) = cdecl 0x4080F0 => remove_car_model;
    /// `CStreaming::LoadCdDirectory(const char*, int32)` (0x5B6170)
    pub fn call_load_cd_directory_file(path: GPtr<u8>, img: i32) = cdecl 0x5B6170 => load_cd_directory_file;
    /// `CStreaming::LoadCdDirectory()` (0x5B82C0)
    pub fn call_load_cd_directory() = cdecl 0x5B82C0 => load_cd_directory;
    /// `CStreaming::InitImageList()` (0x4083C0)
    pub fn call_init_image_list() = cdecl 0x4083C0 => init_image_list;
    /// `CStreaming::RemoveCurrentZonesModels()` (0x40B080)
    pub fn call_remove_current_zones_models() = cdecl 0x40B080 => remove_current_zones_models;
    /// `CStreaming::ProcessEntitiesInSectorList(list, flags)` (0x40C450)
    pub fn call_process_entities_in_sector_list(list: GPtr<u8>, flags: i32) = cdecl 0x40C450 => process_entities_in_sector_list;
    /// `CStreaming::ProcessEntitiesInSectorList(list, x, y, minX, minY, maxX, maxY, radius, flags)` (0x40C270)
    pub fn call_process_entities_in_sector_list_radius(list: GPtr<u8>, px: f32, py: f32, min_x: f32, min_y: f32, max_x: f32, max_y: f32, radius: f32, flags: i32) = cdecl 0x40C270 => process_entities_in_sector_list_radius;
    /// `CLinkList<CEntity*>::Insert(const CEntity*&)` (stub 0x408230)
    pub fn call_link_list_insert(list: GPtr<u8>, data: GPtr<GPtr<u8>>) -> GPtr<u8> = thiscall 0x408230 => link_list_insert;
    /// `CStreamingInfo::AddToList(after)` (0x407480)
    pub fn call_add_to_list(this: GPtr<CStreamingInfo>, after: GPtr<CStreamingInfo>) = thiscall 0x407480 => si::add_to_list;
    /// `CStreamingInfo::RemoveFromList()` (0x4074E0)
    pub fn call_remove_from_list(this: GPtr<CStreamingInfo>) = thiscall 0x4074E0 => si::remove_from_list;
    /// `CStreamingInfo::Init()` (0x407460)
    pub fn call_info_init(this: GPtr<CStreamingInfo>) = thiscall 0x407460 => si::init;
    // --- added
    /// `CFileLoader::FinishLoadClumpFile(RwStream*, uint32 model)` (0x537450)
    pub fn fl_finish_load_clump_file(stream: GPtr<u8>, model: i32) -> u8 = cdecl 0x537450 => unported;
    /// CRT `strchr` (0x8213D0)
    pub fn strchr(s: GPtr<u8>, c: i32) -> GPtr<u8> = cdecl 0x8213D0 => unported;
    /// `CFileMgr::OpenFile(path, mode)` (0x538900)
    pub fn open_file(name: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => sa_core::filemgr::open_file;
    /// `CPopCycle::IsPedAppropriateForCurrentZone(model)` (0x610150)
    pub fn popcycle_is_ped_appropriate_for_current_zone(model: i32) -> u8 = cdecl 0x610150 => unported;
    /// `CPopCycle::PickARandomGroupOfOtherPeds()` (0x610420)
    pub fn popcycle_pick_a_random_group_of_other_peds() -> i32 = cdecl 0x610420 => unported;
    /// `CCarCtrl::ChooseCarModelToLoad(int32 group)` (0x421900)
    pub fn car_ctrl_choose_car_model_to_load(group: i32) -> i32 = cdecl 0x421900 => unported;
    /// `CPathFind::IsWaterNodeNearby(CVector, float)` (0x450DE0, thiscall on `ThePaths`)
    pub fn path_find_is_water_node_nearby(paths: GPtr<u8>, x: f32, y: f32, z: f32, radius: f32) -> u8 = thiscall 0x450DE0 => unported;
    /// `CCamera::CalculateGroundHeight(eGroundHeightType)` (0x514B80, thiscall, ST0)
    pub fn camera_calculate_ground_height(cam: GPtr<u8>, kind: i32) -> f32 = thiscall 0x514B80 => unported;
    /// `CRadar::StreamRadarSections(const CVector&)` (0x5858D0)
    pub fn radar_stream_radar_sections(pos: GPtr<f32>) = cdecl 0x5858D0 => unported;
    /// `CPathFind::LoadSceneForPathNodes(CVector)` (0x44DE00, thiscall on `ThePaths`)
    pub fn path_find_load_scene_for_path_nodes(paths: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x44DE00 => unported;
    /// `CEntity::IsVisible()` (0x536BC0, thiscall)
    pub fn entity_is_visible(e: GPtr<u8>) -> u8 = thiscall 0x536BC0 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    pub fn call_get_random_number_in_range_i32(min: i32, max: i32) -> i32 = cdecl 0x407180 => sa_core::general::get_random_number_in_range_i32;
    /// `CPopulation::LoadSpecificDriverModelsForCar(model)` (0x6117F0)
    pub fn population_load_specific_driver_models_for_car(model: i32) = cdecl 0x6117F0 => unported;
    /// `CCheat::IsZoneStreamingAllowed()` (0x407410, AL)
    pub fn cheat_is_zone_streaming_allowed() -> u8 = cdecl 0x407410 => unported;
    /// `CGangWars::TellStreamingWhichGangsAreNeeded(uint32* mask)` (0x443D50)
    pub fn gang_wars_tell_streaming_which_gangs_are_needed(mask: GPtr<i32>) = cdecl 0x443D50 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    pub fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CWanted::AreSwatRequired/AreFbiRequired/AreArmyRequired/NumOfHelisRequired` (thiscall)
    pub fn wanted_are_swat_required(w: GPtr<u8>) -> u8 = thiscall 0x561F40 => unported;
    pub fn wanted_are_fbi_required(w: GPtr<u8>) -> u8 = thiscall 0x561F60 => unported;
    pub fn wanted_are_army_required(w: GPtr<u8>) -> u8 = thiscall 0x561F80 => unported;
    pub fn wanted_num_of_helis_required(w: GPtr<u8>) -> i32 = thiscall 0x561FA0 => unported;
    /// `CStreaming::RequestSpecialModel(id, name, flags)` (0x409D10)
    pub fn call_request_special_model(id: i32, name: GPtr<u8>, flags: i32) = cdecl 0x409D10 => request_special_model;
    /// `CPed::IsPlayer()` / `CPed::CanBeDeletedEvenInVehicle()` (thiscall, AL)
    pub fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    pub fn ped_can_be_deleted_even_in_vehicle(p: GPtr<u8>) -> u8 = thiscall 0x5DF150 => unported;
    /// `CTheScripts::RemoveThisPed(CPed*)` (0x486240)
    pub fn the_scripts_remove_this_ped(p: GPtr<u8>) = cdecl 0x486240 => unported;
    /// `CObject::CanBeDeleted()` (0x59F120, thiscall, AL)
    pub fn object_can_be_deleted(o: GPtr<u8>) -> u8 = thiscall 0x59F120 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280), `CWorld::RemoveReferencesToDeletedObject` (0x565510)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    pub fn world_remove_references_to_deleted_object(e: GPtr<u8>) = cdecl 0x565510 => unported;
    /// the entity's scalar deleting destructor (vtable +0x00)
    pub fn entity_scalar_deleting_destructor(e: GPtr<u8>, flags: i32) -> GPtr<u8> = thiscall vslot 0x00 => unported;
    /// `CAnimManager` block functions
    pub fn anim_add_anim_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    pub fn anim_remove_anim_block_ref_without_delete(i: i32) = cdecl 0x4D3FF0 => unported;
    pub fn anim_remove_anim_block(i: i32) = cdecl 0x4D3F40 => unported;
    pub fn anim_get_num_refs_to_anim_block(i: i32) -> i32 = cdecl 0x4D4010 => unported;
    pub fn anim_load_anim_file(stream: GPtr<u8>, compress: u8, uncompressed: GPtr<u8>) = cdecl 0x4D47F0 => unported;
    pub fn anim_create_anim_assoc_groups() = cdecl 0x4D3CC0 => unported;
    pub fn anim_register_anim_block(name: GPtr<u8>) -> i32 = cdecl 0x4D3E50 => unported;
    /// `CPathFind::LoadPathFindData(RwStream*, int32)` / `UnloadPathFindData(int32)` (thiscall `ThePaths`)
    pub fn path_find_load_path_find_data(paths: GPtr<u8>, stream: GPtr<u8>, i: i32) = thiscall 0x4529F0 => unported;
    pub fn path_find_unload_path_find_data(paths: GPtr<u8>, i: i32) = thiscall 0x44D0F0 => unported;
    /// `CVehicleRecording::Load(RwStream*, int32, int32)` (0x45A8F0), `RegisterRecordingFile` (0x459F80)
    pub fn vehicle_recording_load(stream: GPtr<u8>, i: i32, size: i32) = cdecl 0x45A8F0 => unported;
    pub fn vehicle_recording_register_recording_file(name: GPtr<u8>) -> i32 = cdecl 0x459F80 => unported;
    /// `CStreamedScripts` (thiscall on 0xA47B60)
    pub fn streamed_scripts_load_streamed_script(ss: GPtr<u8>, stream: GPtr<u8>, i: i32) = thiscall 0x470840 => unported;
    pub fn streamed_scripts_remove_from_memory(ss: GPtr<u8>, i: i32) = thiscall 0x4708E0 => unported;
    pub fn streamed_scripts_register_script(ss: GPtr<u8>, name: GPtr<u8>) -> i32 = thiscall 0x4706C0 => unported;
    pub fn streamed_scripts_re_initialise(ss: GPtr<u8>) = thiscall 0x4706A0 => unported;
    pub fn streamed_scripts_initialise() = cdecl 0x470660 => unported;
    /// CRT `operator new` through its `jmp` entry 0x821195 (Init2's link array)
    pub fn operator_new_alias(size: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// `CGenericGameStorage::SaveDataToWorkBuffer` / `LoadDataFromWorkBuffer` (0x5D1270 / 0x5D1300)
    pub fn generic_game_storage_save_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1270 => unported;
    pub fn generic_game_storage_load_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1300 => unported;
    /// `CStreamingInfo::GetCdPosnAndSize` / `SetCdPosnAndSize` (called)
    pub fn call_get_cd_posn_and_size(this: GPtr<CStreamingInfo>, pos: GPtr<u32>, size: GPtr<u32>) -> bool = thiscall 0x4075A0 => si::get_cd_posn_and_size;
    pub fn call_set_cd_posn_and_size(this: GPtr<CStreamingInfo>, pos: u32, size: u32) = thiscall 0x4075E0 => si::set_cd_posn_and_size;
}

// -------------------------------------------------------------------------------- helpers ---

/// `ms_files[img].StreamHandle`.
pub fn file_handle(img: i32) -> u32 {
    FILES.ptr().cast::<StreamingFile>().wrapping_index(img).read().handle
}
fn file(img: i32) -> GPtr<StreamingFile> {
    FILES.ptr().cast::<StreamingFile>().wrapping_index(img)
}
fn channel(ch: i32) -> GPtr<StreamingChannel> {
    CHANNEL.ptr().cast::<StreamingChannel>().wrapping_index(ch)
}
fn ch_field<T: GameData>(ch: i32, off: isize) -> GPtr<T> {
    field(channel(ch), off)
}
const CH_IDS: isize = 0x00;
const CH_OFFSETS: isize = 0x40;
const CH_STATUS: isize = 0x80;
const CH_LEVEL: isize = 0x84;
const CH_POS: isize = 0x88;
const CH_SECTORS: isize = 0x8C;
const CH_TRIES: isize = 0x90;
const CH_CD_STATUS: isize = 0x94;
fn ch_id(ch: i32, i: i32) -> GPtr<i32> {
    ch_field::<i32>(ch, CH_IDS).wrapping_index(i)
}
fn ch_offset(ch: i32, i: i32) -> GPtr<i32> {
    ch_field::<i32>(ch, CH_OFFSETS).wrapping_index(i)
}

fn load_state(id: i32) -> u8 {
    field::<u8, _>(info(id), 0x10).read()
}
fn set_load_state(id: i32, s: u8) {
    field::<u8, _>(info(id), 0x10).write(s)
}
fn flags(id: i32) -> u8 {
    field::<u8, _>(info(id), 0x06).read()
}
fn set_flags(id: i32, f: u8) {
    field::<u8, _>(info(id), 0x06).write(f)
}
fn next_index(id: i32) -> i16 {
    field::<i16, _>(info(id), 0x00).read()
}
fn cd_size(id: i32) -> u32 {
    field::<u32, _>(info(id), 0x0C).read()
}
/// `info.GetCdPosn()` inlined: `ms_files[img].handle + offset`.
fn cd_posn(id: i32) -> u32 {
    let i = info(id).read();
    file_handle(i.img_id as i32).wrapping_add(i.cd_offset)
}
/// Index of an info pointer (`(p - &ms_aInfoForModel[0]) / 0x14`, MSVC signed division).
fn id_of(p: GPtr<CStreamingInfo>) -> i32 {
    p.index_from(INFO_FOR_MODEL.ptr().cast())
}
/// `GetNext()`: `m_NextIndex == -1 ? null : &ms_pArrayBase[m_NextIndex]`.
fn next_of(p: GPtr<CStreamingInfo>) -> GPtr<CStreamingInfo> {
    let n = field::<i16, _>(p, 0x0).read();
    if n == -1 { GPtr::NULL } else { STREAMING_INFO_ARRAY_BASE.get().wrapping_index(n as i32) }
}
/// `GetPrev()`.
fn prev_of(p: GPtr<CStreamingInfo>) -> GPtr<CStreamingInfo> {
    let n = field::<i16, _>(p, 0x2).read();
    if n == -1 { GPtr::NULL } else { STREAMING_INFO_ARRAY_BASE.get().wrapping_index(n as i32) }
}

fn mi(id: i32) -> GPtr<u8> {
    model_info(id).cast()
}
fn mi_txd(m: GPtr<u8>) -> i32 {
    field::<i16, u8>(m, 0x0A).read() as i32
}
fn mi_refs(m: GPtr<u8>) -> i16 {
    field::<i16, u8>(m, 0x08).read()
}

/// `CPool<TxdDef>::GetAt(slot)` (null when free).
fn txd_def(slot: i32) -> GPtr<u8> {
    let pool = TXD_POOL.get().read();
    if (pool.byte_map.wrapping_index(slot).read() as i8) < 0 {
        GPtr::NULL
    } else {
        pool.objects.wrapping_index(slot).cast()
    }
}

fn e_flags(e: GPtr<u8>) -> u32 {
    field::<u32, u8>(e, 0x1C).read()
}
fn e_model(e: GPtr<u8>) -> i32 {
    field::<i16, u8>(e, 0x22).read() as i32
}
fn e_rw(e: GPtr<u8>) -> GPtr<u8> {
    field::<GPtr<u8>, u8>(e, 0x18).read()
}
fn e_area(e: GPtr<u8>) -> u8 {
    field::<u8, u8>(e, 0x2F).read()
}
/// `CPlaceable::GetPosition()`: `m_matrix ? &m_matrix->pos : &m_placement.pos`.
fn e_pos(e: GPtr<u8>) -> GPtr<f32> {
    let m = field::<GPtr<u8>, u8>(e, 0x14).read();
    if m.is_null() { e.byte_add(4).cast() } else { m.byte_add(0x30).cast() }
}
fn camera_pos() -> GPtr<f32> {
    e_pos(g(CAMERA))
}
/// area test of the sector loops: `area == currArea || area == 13` (byte compares).
fn in_current_area(e: GPtr<u8>) -> bool {
    let a = e_area(e) as i32;
    a == curr_area() || a == 13
}

/// Walks a `CPtrListSingleLink` (`list->head`, node = {item, next}), reading `next` before `f` runs.
fn for_each_in_list(list: GPtr<u8>, mut f: impl FnMut(GPtr<u8>)) {
    let mut node = field::<GPtr<u8>, u8>(list, 0).read();
    while !node.is_null() {
        let e = field::<GPtr<u8>, u8>(node, 0).read();
        node = field::<GPtr<u8>, u8>(node, 4).read();
        f(e);
    }
}

/// `floor` as the CRT computes it on a double (exact for every double).
fn floor_f64(v: f64) -> f64 {
    if v.is_nan() || v.is_infinite() || v.abs() >= 4_503_599_627_370_496.0 {
        return v;
    }
    let t = v as i64 as f64;
    if t > v { t - 1.0 } else { t }
}
/// `_ftol(floor(v * 0.02 + 60))` with PC=24 (`CWorld::GetSectorX/Y`).
fn sector_of(v: X87) -> i32 {
    let s = v.fmul_m(k(F_0_02)).fadd_m(k(F_60));
    X87::from_f64_exact(floor_f64(s.to_f64())).ftol() as u32 as i32
}
/// `GetSectorX/Y` of an f32 coordinate.
fn sector_of_f32(v: f32) -> i32 {
    sector_of(x(v))
}
/// clamp to `[0, 119]` as `CWorld::GetSector` does: `(i < 1 ? 0 : i)` then `> 118 → 119`.
fn clamp_sector(i: i32) -> i32 {
    let v = if i < 1 { 0 } else { i };
    if v > 0x76 { 0x77 } else { v }
}
fn sector_lists(sx: i32, sy: i32) -> (GPtr<u8>, GPtr<u8>) {
    let idx = clamp_sector(sy) * 0x78 + clamp_sector(sx);
    let s = GPtr::<u8>::from_addr(SECTORS + 8 * idx as usize);
    (s, s.byte_add(4))
}
fn repeat_objects(sx: i32, sy: i32) -> GPtr<u8> {
    let idx = ((sx & 0xF) + (sy & 0xF) * 0x10) as usize;
    GPtr::from_addr(REPEAT_SECTOR_OBJECTS + 0xC * idx)
}

/// The body of `SetModelIsDeletable` (0x409C10), as several functions inline it.
fn set_model_is_deletable_inl(id: i32) {
    set_flags(id, flags(id) & 0xFD);
    if flags(id) & 4 != 0 {
        return;
    }
    if load_state(id) == 1 {
        if next_index(id) == -1 {
            si::add_to_list(info(id), START_LOADED_LIST.get());
        }
    } else if flags(id) & 8 == 0 {
        call_remove_model(id);
    }
}

// ------------------------------------------------------------------------------ leaves ---

/// `CStreaming::RequestTxdModel(slot, flags)` (0x407100): rewrites its argument to `slot + 20000` and
/// tail-jumps into `RequestModel`.
#[hook(addr = 0x407100, conv = cdecl, name = "CStreaming::RequestTxdModel", expect = "8B 4C 24 04 81 C1 20 4E 00 00")]
pub fn request_txd_model(slot: i32, flags: i32) {
    call_request_model(slot.wrapping_add(RESOURCE_ID_TXD), flags)
}

/// `CStreaming::AddImageToList(const char* name, bool notPlayerImg)` (0x407610 → 0x1567B90): first
/// free `ms_files` entry (name[0] == 0): copy the name, `CdStreamOpen(name, false)`, flag; returns the
/// index, or 0 when all 8 are used.
#[hook(addr = 0x407610, conv = cdecl, name = "CStreaming::AddImageToList", expect = "E9 7B 05 16 01")]
pub fn add_image_to_list(name: GPtr<u8>, not_player_img: u8) -> i32 {
    for i in 0..8 {
        let f = file(i);
        if f.read().name[0] == 0 {
            copy_name(f.cast(), name);
            let h = cd_stream_open(name, 0);
            field::<u32, _>(f, 0x2C).write(h);
            field::<u8, _>(f, 0x28).write(not_player_img);
            return i;
        }
    }
    0
}

/// The inlined `strcpy` of a NUL-terminated string (byte loop, terminator included).
fn copy_name(dst: GPtr<u8>, src: GPtr<u8>) {
    let mut i = 0isize;
    loop {
        let c = src.byte_add(i).read();
        dst.byte_add(i).write(c);
        if c == 0 {
            return;
        }
        i += 1;
    }
}

/// `CStreaming::IsVeryBusy()` (0x4076A0 → 0x156D7A0, EAX 0/1): `CRenderer::m_loadingPriority ||
/// ms_numModelsRequested > 5` (signed).
#[hook(addr = 0x4076A0, conv = cdecl, name = "CStreaming::IsVeryBusy", expect = "E9 FB 60 16 01")]
pub fn is_very_busy() -> u32 {
    (g::<u8>(RENDERER_LOADING_PRIORITY).read() != 0 || NUM_MODELS_REQUESTED.get() > 5) as u32
}

/// `CStreaming::IsObjectInCdImage(int32)` (0x407800 → 0x1565170, AL; spec CStreaming.md): `cdSize != 0`.
/// EAX bits 8..31 hold `(id * 5) >> 8` in the original (callers test AL only).
#[hook(addr = 0x407800, conv = cdecl, name = "CStreaming::IsObjectInCdImage", expect = "E9 6B D9 15 01")]
pub fn is_object_in_cd_image(id: i32) -> bool {
    cd_size(id) != 0
}

/// `CStreaming::ForceLayerToRead(int32)` (0x407A10 → 0x156FEE0): flag byte, then the unused dword.
#[hook(addr = 0x407A10, conv = cdecl, name = "CStreaming::ForceLayerToRead", expect = "E9 CB 84 16 01")]
pub fn force_layer_to_read(arg: i32) {
    READ_LAYER_FORCE_FULLY.set(GBool::TRUE);
    FORCE_LAYER_ARG.set(arg);
}

/// `CStreaming::SetLoadVehiclesInLoadScene(bool)` (0x407A30 → 0x1562C80): the raw low byte.
#[hook(addr = 0x407A30, conv = cdecl, name = "CStreaming::SetLoadVehiclesInLoadScene", expect = "E9 4B B2 15 01")]
pub fn set_load_vehicles_in_load_scene(enable: u8) {
    LOAD_VEHICLES_IN_LOAD_SCENE.set(GBool(enable));
}

/// `CStreaming::ClearFlagForAll(uint32)` (0x407A40 → 0x1566490): `flags &= ~f` (byte) for every entry.
#[hook(addr = 0x407A40, conv = cdecl, name = "CStreaming::ClearFlagForAll", expect = "E9 4B EA 15 01")]
pub fn clear_flag_for_all(f: u32) {
    for id in 0..crate::shared::RESOURCE_ID_TOTAL as i32 {
        set_flags(id, flags(id) & !(f as u8));
    }
}

/// `CStreaming::RemoveUnusedModelsInLoadedList()` (0x407AC0): `ret` (not hookable).
pub fn remove_unused_models_in_loaded_list() {}

/// `CStreaming::AreAnimsUsedByRequestedModels(int32 animSlot)` (0x407AD0, AL): entry `jmp 0x4019B9`
/// loads `ms_pStartRequestedList` through a protection cell (`[0x15606A0] - [0x646FE2]` = 0x8E4C58).
/// Walks the requested list (DFF ids only: `GetAnimFileIndex() == slot`), then both channels'
/// model ids (`!= -1 && < 20000`), channel 0 before channel 1 per index.
#[hook(addr = 0x407AD0, conv = cdecl, name = "CStreaming::AreAnimsUsedByRequestedModels", expect = "E9 E4 9E FF FF 66")]
pub fn are_anims_used_by_requested_models(anim: i32) -> bool {
    let end = END_REQUESTED_LIST.get();
    let mut p = next_of(START_REQUESTED_LIST.get());
    while p != end {
        let id = id_of(p);
        if id < 20000 && mi_get_anim_file_index(mi(id)) == anim {
            return true;
        }
        p = next_of(p);
    }
    for i in 0..16 {
        for ch in 0..2 {
            let id = ch_id(ch, i).read();
            if id != -1 && id < 20000 && mi_get_anim_file_index(mi(id)) == anim {
                return true;
            }
        }
    }
    false
}

/// `CStreaming::ImGonnaUseStreamingMemory()` (0x407BE0): `ret` (spec; gr PushMemId not in 1.0 US).
pub fn im_gonna_use_streaming_memory() {}

/// `CStreaming::IHaveUsedStreamingMemory()` (0x407BF0): `ret` (spec; gr PopMemId not in 1.0 US).
pub fn i_have_used_streaming_memory() {}

fn cop_model(i: i32) -> i32 {
    DEFAULT_COP_MODEL.ptr().cast::<i32>().wrapping_index(i).read()
}
fn cop_car_model(i: i32) -> i32 {
    DEFAULT_COP_CAR_MODEL.ptr().cast::<i32>().wrapping_index(i).read()
}
fn level() -> i32 {
    g::<i32>(ZONES_CURR_LEVEL).read()
}

/// `CStreaming::GetDefaultCopModel()` (0x407C00 → 0x156C9E0): the level's cop if loaded, else the
/// first loaded of the four, else -1.
#[hook(addr = 0x407C00, conv = cdecl, name = "CStreaming::GetDefaultCopModel", expect = "E9 DB 4D 16 01")]
pub fn get_default_cop_model() -> i32 {
    let m = cop_model(level());
    if load_state(m) == 1 {
        return m;
    }
    for i in 0..4 {
        if load_state(cop_model(i)) == 1 {
            return cop_model(i);
        }
    }
    -1
}

/// `CStreaming::GetDefaultCopCarModel(bool ignoreCopBike)` (0x407C50): the cop bike when bikes are
/// loaded and not ignored (whole dword tested), else the level's car (when car and cop are loaded),
/// else the first loaded pair of 5 (4 when ignoring the bike), else -1.
#[hook(addr = 0x407C50, conv = cdecl, name = "CStreaming::GetDefaultCopCarModel", expect = "A0 BE 54 96 00")]
pub fn get_default_cop_car_model(ignore_cop_bike: i32) -> i32 {
    let lv = level();
    if COP_BIKE_LOADED.get().get()
        && ignore_cop_bike == 0
        && load_state(cop_model(4)) == 1
        && load_state(cop_car_model(4)) == 1
    {
        return cop_car_model(4);
    }
    let car = cop_car_model(lv);
    if load_state(cop_model(lv)) == 1 && load_state(car) == 1 {
        return car;
    }
    let n = if ignore_cop_bike != 0 { 4 } else { 5 };
    for i in 0..n {
        if load_state(cop_model(i)) == 1 && load_state(cop_car_model(i)) == 1 {
            return cop_car_model(i);
        }
    }
    -1
}

/// `CStreaming::DisableCopBikes(bool)` (0x407D10 → 0x15703D0): raw byte.
#[hook(addr = 0x407D10, conv = cdecl, name = "CStreaming::DisableCopBikes", expect = "E9 BB 86 16 01")]
pub fn disable_cop_bikes(disable: u8) {
    DISABLE_COP_BIKES.set(GBool(disable));
}

/// `CStreaming::GetDefaultMedicModel()` (0x407D20 → 0x1563A50).
#[hook(addr = 0x407D20, conv = cdecl, name = "CStreaming::GetDefaultMedicModel", expect = "E9 2B BD 15 01")]
pub fn get_default_medic_model() -> i32 {
    DEFAULT_MEDIC_MODEL.ptr().cast::<i32>().wrapping_index(level()).read()
}

/// `CStreaming::GetDefaultFiremanModel()` (0x407D40 → 0x156CD70).
#[hook(addr = 0x407D40, conv = cdecl, name = "CStreaming::GetDefaultFiremanModel", expect = "E9 2B 50 16 01")]
pub fn get_default_fireman_model() -> i32 {
    DEFAULT_FIREMAN_MODEL.ptr().cast::<i32>().wrapping_index(level()).read()
}

/// `CStreaming::GetDefaultCabDriverModel()` (0x407D50; gr-deviations `Streaming.cpp:2729`,
/// keep-original): when the current driver is not loaded and the level is not 0:
/// `index = 2*level - _ftol((rand() & 0xFFFF) * (1/32768) * -2) - 2` (x87 PC=24; `rand` attributed to
/// the return address 0x407D7E).
#[hook(addr = 0x407D50, conv = cdecl, name = "CStreaming::GetDefaultCabDriverModel", expect = "51 8B 0D 24 55 96 00")]
pub fn get_default_cab_driver_model() -> i32 {
    let table = DEFAULT_CAB_DRIVER_MODEL.ptr().cast::<i32>();
    let m = table.wrapping_index(LAST_RANDOM_CAB_INDEX.get()).read();
    if load_state(m) != 0 {
        return m;
    }
    let lv = level();
    if lv != 0 {
        let r = sa_core::orig::at_site(0x407D7E, rand) & 0xFFFF;
        let v = X87::from_i32(r).fmul_m(k(F_INV_32768)).fmul_m(k(F_M2)).ftol() as u32 as i32;
        LAST_RANDOM_CAB_INDEX.set((2 * lv).wrapping_sub(v).wrapping_sub(2));
    }
    table.wrapping_index(LAST_RANDOM_CAB_INDEX.get()).read()
}

/// `CStreaming::HasSpecialCharLoaded(int32 slot)` (0x407F00 → 0x15642C0, AL): model `290 + slot` loaded.
#[hook(addr = 0x407F00, conv = cdecl, name = "CStreaming::HasSpecialCharLoaded", expect = "E9 BB C3 15 01")]
pub fn has_special_char_loaded(slot: i32) -> bool {
    load_state(slot.wrapping_add(290)) == 1
}

/// `CStreaming::LoadInitialVehicles()` (0x407F20): `ret` (not hookable).
pub fn load_initial_vehicles() {}

/// `CStreaming::WeAreTryingToPhaseVehicleOut(int32)` (0x407F80 → 0x1567B60, EAX 0/1): loaded and in a
/// list (`next >= 0 || prev >= 0`, signed).
#[hook(addr = 0x407F80, conv = cdecl, name = "CStreaming::WeAreTryingToPhaseVehicleOut", expect = "E9 DB FB 15 01")]
pub fn we_are_trying_to_phase_vehicle_out(id: i32) -> u32 {
    let i = info(id).read();
    (i.load_state == 1 && (i.next_index >= 0 || i.prev_index >= 0)) as u32
}

/// `CStreaming::FindMIPedSlotForInterior(int32 randFactor)` (0x407FB0): the first loaded ped of
/// `ms_pedsLoaded[(i + r) % 8]` (C remainder: `and 0x80000007` with the negative fix-up), i = 0..7;
/// none → 7 (the original's constant).
#[hook(addr = 0x407FB0, conv = cdecl, name = "CStreaming::FindMIPedSlotForInterior", expect = "8B 54 24 04 33 C9")]
pub fn find_mi_ped_slot_for_interior(rand_factor: i32) -> i32 {
    for i in 0..8i32 {
        let s = i.wrapping_add(rand_factor).wrapping_rem(8);
        let m = PEDS_LOADED.ptr().cast::<i32>().wrapping_index(s).read();
        if m >= 0 && load_state(m) == 1 {
            return m;
        }
    }
    7
}

/// `CLinkList<CEntity*>::Insert(CEntity* const&)` (stub 0x408230 → 0x1563580, thiscall, `ret 4`;
/// spec CLinkList.md): the first free link (or null), data stored, unlinked, linked after the used
/// head.
#[hook(addr = 0x408230, conv = thiscall, name = "thunk_FUN_01563580", expect = "E9 4B B3 15 01")]
pub fn link_list_insert(list: GPtr<u8>, data: GPtr<GPtr<u8>>) -> GPtr<u8> {
    let link = field::<GPtr<u8>, u8>(list, 0x20).read();
    if link == list.byte_add(0x24) {
        return GPtr::NULL;
    }
    field::<GPtr<u8>, u8>(link, 0).write(data.read());
    let next = field::<GPtr<u8>, u8>(link, 8).read();
    field::<GPtr<u8>, u8>(next, 4).write(field::<GPtr<u8>, u8>(link, 4).read());
    let prev = field::<GPtr<u8>, u8>(link, 4).read();
    field::<GPtr<u8>, u8>(prev, 8).write(field::<GPtr<u8>, u8>(link, 8).read());
    field::<GPtr<u8>, u8>(link, 8).write(field::<GPtr<u8>, u8>(list, 8).read());
    let head_next = field::<GPtr<u8>, u8>(list, 8).read();
    field::<GPtr<u8>, u8>(head_next, 4).write(link);
    field::<GPtr<u8>, u8>(link, 4).write(list);
    field::<GPtr<u8>, u8>(list, 8).write(link);
    link
}

/// `CStreaming::InitImageList()` (0x4083C0): clears the 8 names and handles, then the inlined
/// `AddImageToList("MODELS\\GTA3.IMG", true)` and `("MODELS\\GTA_INT.IMG", true)` storing the slots
/// in 0x8E48A8 / 0x8E48A4 (0 when no slot is free).
#[hook(addr = 0x4083C0, conv = cdecl, name = "CStreaming::InitImageList", expect = "B8 04 49 8E 00")]
pub fn init_image_list() {
    for i in 0..8 {
        field::<u8, _>(file(i), 0).write(0);
        field::<u32, _>(file(i), 0x2C).write(0);
    }
    GTA3_IMG.set(add_image_inline(0x858AF4));
    GTA_INT_IMG.set(add_image_inline(0x858B20));
}
fn add_image_inline(name: usize) -> i32 {
    for i in 0..8 {
        if file(i).read().name[0] == 0 {
            copy_name(file(i).cast(), GPtr::from_addr(name));
            let h = cd_stream_open(GPtr::from_addr(name), 0);
            field::<u32, _>(file(i), 0x2C).write(h);
            field::<u8, _>(file(i), 0x28).write(1);
            return i;
        }
    }
    0
}

/// `CStreaming::Shutdown()` (0x4084B0 → 0x1563AB0; gr-deviations `Streaming.cpp:3061`/`:3069`
/// keep-original): `FreeAlign(ms_pStreamingBuffer[0])`, size 0, the extra-objects directory
/// destructed and freed; neither pointer is cleared.
#[hook(addr = 0x4084B0, conv = cdecl, name = "CStreaming::Shutdown", expect = "E9 FB B5 15 01")]
pub fn shutdown() {
    memory_free_align(STREAMING_BUFFER.ptr().cast::<GPtr<u8>>().read());
    STREAMING_BUFFER_SIZE.set(0);
    let dir = EXTRA_OBJECTS_DIR.get();
    if !dir.is_null() {
        directory_dtor(dir.cast());
        free(dir);
    }
}

/// `CStreaming::RequestModel(int32 id, int32 flags)` (0x4087E0; the load-state read goes through the
/// chunk 0x404C95). Requested (2) + new priority flag → priority count; loaded/reading/finishing
/// drop the priority bit; flags are or'ed. Loaded (1) and in a list: unlinked (inlined
/// `RemoveFromList`), then re-added to the loaded list unless a ped (7) / vehicle (6) model or
/// game/mission required. Not loaded (0): TXD (full flags dword) and anim (8) requests, added to the
/// requested list, counters; then (also for unknown states) `flags = 0 | low byte`, state 2.
#[hook(addr = 0x4087E0, conv = cdecl, name = "CStreaming::RequestModel", expect = "53 8B 5C 24 0C")]
pub fn request_model(id: i32, flags_arg: i32) {
    let mut bl = flags_arg as u8;
    let st = load_state(id);
    if st == 2 {
        if bl & 0x10 != 0 && flags(id) & 0x10 == 0 {
            NUM_PRIORITY_REQUESTS.set(NUM_PRIORITY_REQUESTS.get().wrapping_add(1));
            set_flags(id, flags(id) | 0x10);
        }
    } else if st != 0 {
        bl &= 0xEF;
    }
    set_flags(id, flags(id) | bl);
    let p = info(id);
    match load_state(id) {
        1 => {
            if next_index(id) == -1 {
                return;
            }
            let base = STREAMING_INFO_ARRAY_BASE.get();
            let prev = field::<i16, _>(p, 2).read();
            field::<i16, _>(base.wrapping_index(next_index(id) as i32), 2).write(prev);
            let next = next_index(id);
            let base = STREAMING_INFO_ARRAY_BASE.get();
            field::<i16, _>(base.wrapping_index(field::<i16, _>(p, 2).read() as i32), 0).write(next);
            field::<i16, _>(p, 0).write(-1);
            field::<i16, _>(p, 2).write(-1);
            if id < 20000 && (mi_get_model_type(mi(id)) == 7 || mi_get_model_type(mi(id)) == 6) {
                return;
            }
            if flags(id) & 6 == 0 {
                call_add_to_list(p, START_LOADED_LIST.get());
            }
            return;
        }
        2..=4 => return,
        0 => {
            if id < 20000 {
                let m = mi(id);
                call_request_txd_model(mi_txd(m), flags_arg);
                let anim = mi_get_anim_file_index(m);
                if anim != -1 {
                    call_request_model(anim + RESOURCE_ID_IFP, 8);
                }
            } else if id < RESOURCE_ID_COL {
                let parent = txd_get_parent_txd_slot(id - RESOURCE_ID_TXD);
                if parent != -1 {
                    call_request_txd_model(parent, flags_arg);
                }
            }
            call_add_to_list(p, START_REQUESTED_LIST.get());
            NUM_MODELS_REQUESTED.set(NUM_MODELS_REQUESTED.get().wrapping_add(1));
            if bl & 0x10 != 0 {
                NUM_PRIORITY_REQUESTS.set(NUM_PRIORITY_REQUESTS.get().wrapping_add(1));
            }
        }
        _ => {}
    }
    set_flags(id, 0);
    set_flags(id, flags(id) | bl);
    set_load_state(id, 2);
}

// ----------------------------------------------------------------------------- sectors ---

/// `fld v; fmul 0.02; fadd 60; fst dword t; fstp qword; floor; ftol` → `n`; then
/// `n > 0 ? ftol(floor(t)) : 0` (the second conversion reloads the f32 copy).
fn sector_min(v: f32) -> i32 {
    let s = x(v).fmul_m(k(F_0_02)).fadd_m(k(F_60));
    let n = X87::from_f64_exact(floor_f64(s.to_f64())).ftol() as u32 as i32;
    if n < 1 {
        0
    } else {
        X87::from_f64_exact(floor_f64(s.to_f32() as f64)).ftol() as u32 as i32
    }
}
/// As [`sector_min`] for the upper bound: `n < 0x77 ? ftol(floor(t)) : 0x77`.
fn sector_max(v: f32) -> i32 {
    let s = x(v).fmul_m(k(F_0_02)).fadd_m(k(F_60));
    let n = X87::from_f64_exact(floor_f64(s.to_f64())).ftol() as u32 as i32;
    if n < 0x77 {
        X87::from_f64_exact(floor_f64(s.to_f32() as f64)).ftol() as u32 as i32
    } else {
        0x77
    }
}

/// `CStreaming::InstanceLoadedModels(const CVector&)` (0x4084F0): radius 80 in the world (40 in an
/// interior); `x - r`, `x + r`, `y - r`, `y + r` stored as f32, converted to clamped sector bounds;
/// per sector (y outer, x inner): buildings, repeat-sector objects, dummies — every entity in the
/// current area (or 13) without an RW object gets `CreateRwObject()`.
#[hook(addr = 0x4084F0, conv = cdecl, name = "CStreaming::InstanceLoadedModels", expect = "A1 14 29 B7 00")]
pub fn instance_loaded_models(pos: GPtr<f32>) {
    let r = if curr_area() != 0 { k(F_40) } else { k(F_80) };
    let (px, py) = (pos.read(), pos.wrapping_index(1).read());
    let min_x = x(px).fsub_m(r).to_f32();
    let max_x = x(r).fadd_m(px).to_f32();
    let min_y = x(py).fsub_m(r).to_f32();
    let max_y = x(r).fadd_m(py).to_f32();
    let (x0, y0, x1, y1) = (sector_min(min_x), sector_min(min_y), sector_max(max_x), sector_max(max_y));
    let mut sy = y0;
    while sy <= y1 {
        let mut sx = x0;
        while sx <= x1 {
            let (b, d) = sector_lists(sx, sy);
            let f = |e: GPtr<u8>| {
                if in_current_area(e) && e_rw(e).is_null() {
                    entity_create_rw_object(e);
                }
            };
            for_each_in_list(b, f);
            for_each_in_list(repeat_objects(sx, sy), f);
            for_each_in_list(d, f);
            sx += 1;
        }
        sy += 1;
    }
}

/// `if ((flags & 0x2400) == 0) DeleteRwObject()` over a list.
fn delete_unprotected(list: GPtr<u8>) {
    for_each_in_list(list, |e| {
        if e_flags(e) & 0x2400 == 0 {
            entity_delete_rw_object(e);
        }
    });
}

/// `CStreaming::DeleteAllRwObjects()` (0x4090A0): every sector (x outer, y inner): buildings,
/// repeat objects, dummies; entities without `m_bImBeingRendered|m_bStreamingDontDelete` (0x2400)
/// lose their RW object. (The camera position's two sector conversions are computed and discarded.)
#[hook(addr = 0x4090A0, conv = cdecl, name = "CStreaming::DeleteAllRwObjects", expect = "A1 3C F0 B6 00")]
pub fn delete_all_rw_objects() {
    for sx in 0..0x78 {
        for sy in 0..0x78 {
            let (b, d) = sector_lists(sx, sy);
            delete_unprotected(b);
            delete_unprotected(repeat_objects(sx, sy));
            delete_unprotected(d);
        }
    }
}

/// `CStreaming::DeleteRwObjectsAfterDeath(const CVector&)` (0x409210): the sectors with
/// `|sx - x| > 3 && |sy - y| > 3` (fild/fabs/fcomp `test ah,0x41; jnp`) as in DeleteAllRwObjects.
#[hook(addr = 0x409210, conv = cdecl, name = "CStreaming::DeleteRwObjectsAfterDeath", expect = "83 EC 10 53 55")]
pub fn delete_rw_objects_after_death(pos: GPtr<f32>) {
    let sx0 = sector_of_f32(pos.read());
    let sy0 = sector_of_f32(pos.wrapping_index(1).read());
    let far = |d: i32| {
        let a = X87::from_f32(X87::from_i32(d).to_f32().abs());
        a.fcom(x(k(F_3))).test_parity(0x41)
    };
    for sx in 0..0x78 {
        for sy in 0..0x78 {
            if far(sx0.wrapping_sub(sx)) && far(sy0.wrapping_sub(sy)) {
                let (b, d) = sector_lists(sx, sy);
                delete_unprotected(b);
                delete_unprotected(repeat_objects(sx, sy));
                delete_unprotected(d);
            }
        }
    }
}

/// The building / object / dummy pools walked backwards as the sector-less functions do.
fn pool_backwards(pool: GPtr<u8>, slot: i32, mut f: impl FnMut(GPtr<u8>)) {
    let objects = field::<GPtr<u8>, u8>(pool, 0).read();
    let map = field::<GPtr<u8>, u8>(pool, 4).read();
    let mut i = field::<i32, u8>(pool, 8).read();
    while i != 0 {
        i -= 1;
        if (map.byte_add(i as isize).read() as i8) >= 0 {
            let e = objects.byte_add((i.wrapping_mul(slot)) as isize);
            if !e.is_null() {
                f(e);
            }
        }
    }
}
fn building_pool() -> GPtr<u8> {
    sa_core::pools::BUILDING_POOL.get().cast()
}
fn object_pool() -> GPtr<u8> {
    sa_core::pool::OBJECT_POOL.get().cast()
}
fn dummy_pool() -> GPtr<u8> {
    sa_core::pools::DUMMY_POOL.get().cast()
}
fn ped_pool() -> GPtr<u8> {
    sa_core::pool::PED_POOL.get().cast()
}

/// `CStreaming::RemoveBigBuildings()` (0x4093B0): buildings (backwards) with `m_bIsBIGBuilding`
/// (0x100) and not `m_bImBeingRendered` (0x2000): DeleteRwObject, then RemoveModel when the model
/// info's ref count is 0 (the model info pointer is read before the delete).
#[hook(addr = 0x4093B0, conv = cdecl, name = "CStreaming::RemoveBigBuildings", expect = "51 8B 0D 98 44 B7 00")]
pub fn remove_big_buildings() {
    pool_backwards(building_pool(), 0x38, |e| {
        if e_flags(e) & 0x100 != 0 {
            let m = mi(e_model(e));
            if e_flags(e) & 0x2000 == 0 {
                entity_delete_rw_object(e);
                if mi_refs(m) == 0 {
                    call_remove_model(e_model(e));
                }
            }
        }
    });
}

/// `CStreaming::RequestBigBuildings(const CVector&)` (0x409430): buildings (backwards) with
/// `m_bIsBIGBuilding` for which `CRenderer::ShouldModelBeStreamed(e, pos, farPlane)` → RequestModel(0).
/// The far plane is `TheCamera.m_pRwCamera->farPlane` (read once).
#[hook(addr = 0x409430, conv = cdecl, name = "CStreaming::RequestBigBuildings", expect = "51 A1 7C F9 B6 00")]
pub fn request_big_buildings(pos: GPtr<f32>) {
    let far = field::<f32, u8>(g::<GPtr<u8>>(CAMERA + 0x954).read(), 0x84).read();
    pool_backwards(building_pool(), 0x38, |e| {
        if e_flags(e) & 0x100 != 0 && renderer_should_model_be_streamed(e, pos, far) != 0 {
            call_request_model(e_model(e), 0);
        }
    });
}

/// `CStreaming::RemoveBuildingsNotInArea(int32 area)` (0x4094B0): buildings with an RW object outside
/// the area (and 13) and without 0x2100; objects (0x19C) likewise without 0x2000 and with
/// `m_nObjectType == 1` (byte +0x13C); dummies without 0x2000 → DeleteRwObject. Backwards walks.
#[hook(addr = 0x4094B0, conv = cdecl, name = "CStreaming::RemoveBuildingsNotInArea", expect = "53 55 8B 6C 24 0C")]
pub fn remove_buildings_not_in_area(area: i32) {
    let out = |e: GPtr<u8>| {
        let a = e_area(e) as u32;
        a != area as u32 && a != 13
    };
    pool_backwards(building_pool(), 0x38, |e| {
        if !e_rw(e).is_null() && out(e) && e_flags(e) & 0x2100 == 0 {
            entity_delete_rw_object(e);
        }
    });
    pool_backwards(object_pool(), 0x19C, |e| {
        if !e_rw(e).is_null() && out(e) && e_flags(e) & 0x2000 == 0 && field::<u8, u8>(e, 0x13C).read() == 1 {
            entity_delete_rw_object(e);
        }
    });
    pool_backwards(dummy_pool(), 0x38, |e| {
        if !e_rw(e).is_null() && out(e) && e_flags(e) & 0x2000 == 0 {
            entity_delete_rw_object(e);
        }
    });
}

fn rw_instances() -> GPtr<u8> {
    RW_OBJECT_INSTANCES.ptr().cast()
}

/// `CStreaming::AddEntity(CEntity*)` (0x409650): peds/vehicles (type 3/2) → null; otherwise insert into
/// `ms_rwObjectInstances`; when full, the first used entry (from the used tail backwards: the
/// list's `usedListTail.prev` walk) without 0x2400 loses its RW object, then insert again.
#[hook(addr = 0x409650, conv = cdecl, name = "CStreaming::AddEntity", expect = "8B 44 24 04 8A 40 36")]
pub fn add_entity(e: GPtr<u8>) -> GPtr<u8> {
    let t = field::<u8, u8>(e, 0x36).read() & 7;
    if t == 3 || t == 2 {
        return GPtr::NULL;
    }
    let mut data = e;
    let l = call_link_list_insert(rw_instances(), GPtr::from_raw(&raw mut data));
    if !l.is_null() {
        return l;
    }
    // usedListTail.prev (0x965500)
    let mut link = field::<GPtr<u8>, u8>(rw_instances(), 0x10).read();
    if link != rw_instances() {
        loop {
            let ent = field::<GPtr<u8>, u8>(link, 0).read();
            if e_flags(ent) & 0x2400 == 0 {
                entity_delete_rw_object(ent);
                break;
            }
            link = field::<GPtr<u8>, u8>(link, 4).read();
            if link == rw_instances() {
                break;
            }
        }
    }
    call_link_list_insert(rw_instances(), GPtr::from_raw(&raw mut data))
}

/// `CStreaming::StartRenderEntities()` (0x4096C0): `ms_renderEntityLink = &ms_rwObjectInstances`.
#[hook(addr = 0x4096C0, conv = cdecl, name = "CStreaming::StartRenderEntities", expect = "C7 05 A0 48 8E 00 F0 54 96 00")]
pub fn start_render_entities() {
    RENDER_ENTITY_LINK.set(rw_instances());
}

/// `CStreaming::RenderEntity(CLink<CEntity*>*)` (0x4096D0 → 0x156D750): a non-null link other than
/// `ms_renderEntityLink` is moved after it and becomes the new render link (stored through a
/// protection cell, `[0x156059C] - [0x638292]` = 0x8E48A0).
#[hook(addr = 0x4096D0, conv = cdecl, name = "CStreaming::RenderEntity", expect = "E9 7B 40 16 01")]
pub fn render_entity(link: GPtr<u8>) {
    if link.is_null() {
        return;
    }
    let r = RENDER_ENTITY_LINK.get();
    if link == r {
        return;
    }
    let l = |p: GPtr<u8>, o: isize| field::<GPtr<u8>, u8>(p, o);
    l(l(link, 8).read(), 4).write(l(link, 4).read());
    l(l(link, 4).read(), 8).write(l(link, 8).read());
    l(link, 8).write(l(r, 8).read());
    l(l(r, 8).read(), 4).write(link);
    l(link, 4).write(r);
    l(r, 8).write(link);
    RENDER_ENTITY_LINK.set(link);
}

/// `CStreaming::RemoveEntity(CLink<CEntity*>*)` (0x409710 → 0x1561520): a non-null link that is the
/// render link moves the render link to its prev (protection cell `[0x651888] ^ [0x1560478]` =
/// 0x8E48A0); then unlinked and put after the free-list head (0x965508).
#[hook(addr = 0x409710, conv = cdecl, name = "CStreaming::RemoveEntity", expect = "E9 0B 7E 15 01")]
pub fn remove_entity(link: GPtr<u8>) {
    if link.is_null() {
        return;
    }
    let l = |p: GPtr<u8>, o: isize| field::<GPtr<u8>, u8>(p, o);
    if link == RENDER_ENTITY_LINK.get() {
        RENDER_ENTITY_LINK.set(l(RENDER_ENTITY_LINK.get(), 4).read());
    }
    l(l(link, 8).read(), 4).write(l(link, 4).read());
    l(l(link, 4).read(), 8).write(l(link, 8).read());
    let free_head = rw_instances().byte_add(0x18);
    l(link, 8).write(l(free_head, 8).read());
    l(l(free_head, 8).read(), 4).write(link);
    l(link, 4).write(free_head);
    l(free_head, 8).write(link);
}

// -------------------------------------------------------------------- RW object deletion ---

/// `CStreaming::DeleteLeastUsedEntityRwObject(bool notOnScreen, uint32 flags)` (0x409760, AL): walks
/// `ms_rwObjectInstances` from the used tail backwards. Per entity without 0x2400: LOD distance
/// `f32(LODMult * drawDist)` (× [0x8CD804] for big buildings); distance from the camera to the
/// entity (or its LOD's) position, `sqrt(dz² + dy² + dx²)` in x87 order, stored f32. The entity is
/// kept when not loading a scene, (`!notOnScreen` or the top LOD / entity is on screen), in the
/// current area, `lod + 50 >= dist` and `colModel.radius + farPlane >= dist`. Otherwise, when in a
/// list, not `flags`-protected and not the player's vehicle: DeleteRwObject, and RemoveModel → true
/// when the model has no refs.
#[hook(addr = 0x409760, conv = cdecl, name = "CStreaming::DeleteLeastUsedEntityRwObject", expect = "83 EC 18 53 55")]
pub fn delete_least_used_entity_rw_object(not_on_screen: u8, flags_arg: u32) -> bool {
    let mut link = field::<GPtr<u8>, u8>(rw_instances(), 0x10).read();
    let player = find_player_ped(-1);
    let far = field::<f32, u8>(g::<GPtr<u8>>(CAMERA + 0x954).read(), 0x84).read();
    while link != rw_instances() {
        let e = field::<GPtr<u8>, u8>(link, 0).read();
        link = field::<GPtr<u8>, u8>(link, 4).read();
        let ef = e_flags(e);
        if ef & 0x2400 != 0 {
            continue;
        }
        let model = e_model(e);
        let m = mi(model);
        let mut lod = x(g::<f32>(CAMERA + 0xF0).read()).fmul_m(field::<f32, u8>(m, 0x18).read()).to_f32();
        if ef & 0x100 != 0 {
            lod = x(g::<f32>(0x8CD804).read()).fmul_m(lod).to_f32();
        }
        let lod_ent = field::<GPtr<u8>, u8>(e, 0x30).read();
        let p = if lod_ent.is_null() { e_pos(e) } else { e_pos(lod_ent) };
        let c = camera_pos();
        let dx = x(p.read()).fsub_m(c.read());
        let dy = x(p.wrapping_index(1).read()).fsub_m(c.wrapping_index(1).read());
        let dz = x(p.wrapping_index(2).read()).fsub_m(c.wrapping_index(2).read());
        let dist = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt().to_f32();
        let mut top = e;
        let mut l = lod_ent;
        while !l.is_null() {
            top = l;
            l = field::<GPtr<u8>, u8>(l, 0x30).read();
        }
        let keep = !LOADING_SCENE.get().get()
            && (not_on_screen == 0 || entity_get_is_on_screen(top) != 0)
            && in_current_area(e)
            && x(lod).fadd_m(k(F_50)).fcom(x(dist)).test_parity(0x05)
            && {
                let cm = field::<GPtr<u8>, u8>(mi(e_model(e)), 0x14).read();
                x(field::<f32, u8>(cm, 0x24).read()).fadd_m(far).fcom(x(dist)).test_parity(0x05)
            };
        if keep {
            continue;
        }
        if next_index(model) != -1
            && flags(model) & (flags_arg as u8) == 0
            && (player.is_null()
                || field::<u32, u8>(player, 0x46C).read() & 0x100 != 0
                || field::<GPtr<u8>, u8>(player, 0x584).read() != e)
        {
            entity_delete_rw_object(e);
            if mi_refs(mi(model)) == 0 {
                call_remove_model(model);
                return true;
            }
        }
    }
    false
}

/// `CStreaming::DeleteRwObjectsInSectorList(list, int32 sx, int32 sy)` (0x407A70): entities of the
/// list (when `sx < 0` all, else those with `LivesInThisNonOverlapSector(sx, sy)`) without 0x2400 lose
/// their RW object.
#[hook(addr = 0x407A70, conv = cdecl, name = "CStreaming::DeleteRwObjectsInSectorList<CPtrListSingleLink<CEntity*>>", expect = "8B 44 24 04 57")]
pub fn delete_rw_objects_in_sector_list(list: GPtr<u8>, sx: i32, sy: i32) {
    for_each_in_list(list, |e| {
        if (sx < 0 || is_area_visible_sector(e, sx, sy)) && e_flags(e) & 0x2400 == 0 {
            entity_delete_rw_object(e);
        }
    });
}
fn is_area_visible_sector(e: GPtr<u8>, sx: i32, sy: i32) -> bool {
    entity_lives_in_this_non_overlap_sector(e, sx, sy) != 0
}

/// `CStreaming::DeleteRwObjectsBehindCameraInSectorList(list, int32 memory)` (0x409940, AL; the scan
/// code is loaded in the chunk 0x409BD3): per entity with a new scan code (then stamped), without
/// 0x2400, with an RW object, whose model is in a list and which is not the player's vehicle
/// (`FindPlayerPed(-1)->m_pVehicle`, no null check): DeleteRwObject; with no refs left RemoveModel,
/// then `true` once `ms_memoryUsedBytes < memory` (signed).
#[hook(addr = 0x409940, conv = cdecl, name = "CStreaming::DeleteRwObjectsBehindCameraInSectorList<CPtrListSingleLink<CEntity*>>", expect = "8B 44 24 04 53")]
pub fn delete_rw_objects_behind_camera_in_sector_list(list: GPtr<u8>, memory: i32) -> bool {
    let mut node = field::<GPtr<u8>, u8>(list, 0).read();
    while !node.is_null() {
        let e = field::<GPtr<u8>, u8>(node, 0).read();
        node = field::<GPtr<u8>, u8>(node, 4).read();
        let sc = g::<u16>(SCAN_CODE).read();
        if field::<u16, u8>(e, 0x2C).read() == sc {
            continue;
        }
        field::<u16, u8>(e, 0x2C).write(sc);
        if e_flags(e) & 0x2400 != 0 || e_rw(e).is_null() {
            continue;
        }
        let model = e_model(e);
        if next_index(model) == -1 {
            continue;
        }
        if field::<GPtr<u8>, u8>(find_player_ped(-1), 0x584).read() == e {
            continue;
        }
        entity_delete_rw_object(e);
        if mi_refs(mi(model)) == 0 {
            call_remove_model(model);
            if (MEMORY_USED.get() as i32) < memory {
                return true;
            }
        }
    }
    false
}

/// `CStreaming::DeleteRwObjectsNotInFrustumInSectorList(list, int32 memory)` (0x4099E0, AL): as the
/// behind-camera variant, but an entity that `IsVisible()` and is not `m_bOffscreen` (0x20000) is
/// kept, and there is no player-vehicle test.
#[hook(addr = 0x4099E0, conv = cdecl, name = "CStreaming::DeleteRwObjectsNotInFrustumInSectorList<CPtrListSingleLink<CEntity*>>", expect = "8B 44 24 04 53")]
pub fn delete_rw_objects_not_in_frustum_in_sector_list(list: GPtr<u8>, memory: i32) -> bool {
    let mut node = field::<GPtr<u8>, u8>(list, 0).read();
    while !node.is_null() {
        let e = field::<GPtr<u8>, u8>(node, 0).read();
        node = field::<GPtr<u8>, u8>(node, 4).read();
        let sc = g::<u16>(SCAN_CODE).read();
        if field::<u16, u8>(e, 0x2C).read() == sc {
            continue;
        }
        field::<u16, u8>(e, 0x2C).write(sc);
        if e_flags(e) & 0x2400 != 0 || e_rw(e).is_null() {
            continue;
        }
        if entity_is_visible(e) != 0 && e_flags(e) & 0x20000 == 0 {
            continue;
        }
        let model = e_model(e);
        if next_index(model) == -1 {
            continue;
        }
        entity_delete_rw_object(e);
        if mi_refs(mi(model)) == 0 {
            call_remove_model(model);
            if (MEMORY_USED.get() as i32) < memory {
                return true;
            }
        }
    }
    false
}

// ------------------------------------------------------------------ requested-list queries ---

/// `CStreaming::AreTexturesUsedByRequestedModels(int32 txd)` (0x409A90 → 0x15664B0, AL): requested
/// DFFs whose model info TXD is `txd`, requested TXDs whose parent (`GetAt(slot)+6`, no null check)
/// is `txd`; then both channels' ids: DFF → model info TXD, else only for channel slots whose
/// address lies in 0x8F8378..=0x8FD197 (the original's range test on the id pointer)
/// `CTxdStore::GetParentTxdSlot(id - 20000)`.
#[hook(addr = 0x409A90, conv = cdecl, name = "CStreaming::AreTexturesUsedByRequestedModels", expect = "E9 1B CA 15 01")]
pub fn are_textures_used_by_requested_models(txd: i32) -> bool {
    let end = END_REQUESTED_LIST.get();
    let mut p = next_of(START_REQUESTED_LIST.get());
    while p != end {
        let id = id_of(p);
        if id < 20000 {
            if mi_txd(mi(id)) == txd {
                return true;
            }
        } else if id < RESOURCE_ID_COL {
            let def = txd_def(id - RESOURCE_ID_TXD);
            if field::<i16, u8>(def, 6).read() as i32 == txd {
                return true;
            }
        }
        p = next_of(p);
    }
    for i in 0..16 {
        for ch in 0..2 {
            let slot = ch_id(ch, i);
            let id = slot.read();
            if id == -1 {
                continue;
            }
            let t = if id < 20000 {
                mi_txd(mi(id))
            } else {
                let a = slot.addr() as u32 as i32;
                if !(0x8F8378..=0x8FD197).contains(&a) {
                    continue;
                }
                txd_get_parent_txd_slot(id - RESOURCE_ID_TXD)
            };
            if t == txd {
                return true;
            }
        }
    }
    false
}

/// `CStreaming::SetModelIsDeletable(int32)` (0x409C10).
#[hook(addr = 0x409C10, conv = cdecl, name = "CStreaming::SetModelIsDeletable", expect = "56 8B 74 24 08")]
pub fn set_model_is_deletable(id: i32) {
    set_flags(id, flags(id) & 0xFD);
    if flags(id) & 4 != 0 {
        return;
    }
    if load_state(id) == 1 {
        if next_index(id) == -1 {
            call_add_to_list(info(id), START_LOADED_LIST.get());
        }
    } else if flags(id) & 8 == 0 {
        call_remove_model(id);
    }
}

/// `CStreaming::SetModelTxdIsDeletable(int32)` (0x409C70; spec): `SetModelIsDeletable(txd + 20000)`
/// (tail jump with the argument slot rewritten; no null check of the model info).
#[hook(addr = 0x409C70, conv = cdecl, name = "CStreaming::SetModelTxdIsDeletable", expect = "8B 44 24 04 8B 0C 85 C8 B0 A9 00")]
pub fn set_model_txd_is_deletable(id: i32) {
    call_set_model_is_deletable(mi_txd(mi(id)) + RESOURCE_ID_TXD)
}

/// `CStreaming::SetMissionDoesntRequireModel(int32)` (0x409C90): clears MISSION_REQUIRED; unless
/// game required: loaded and not in a list → loaded list, not loaded and not KEEP_IN_MEMORY →
/// RemoveModel; then the same for the model's TXD (loop while the id is a DFF).
#[hook(addr = 0x409C90, conv = cdecl, name = "CStreaming::SetMissionDoesntRequireModel", expect = "56 8B 74 24 08")]
pub fn set_mission_doesnt_require_model(id: i32) {
    let mut id = id;
    loop {
        set_flags(id, flags(id) & 0xFB);
        if flags(id) & 2 == 0 {
            if load_state(id) == 1 {
                if next_index(id) == -1 {
                    call_add_to_list(info(id), START_LOADED_LIST.get());
                }
            } else if flags(id) & 8 == 0 {
                call_remove_model(id);
            }
        }
        if id > 19999 {
            return;
        }
        id = mi_txd(mi(id)) + RESOURCE_ID_TXD;
    }
}

/// `CStreaming::RequestFile(int32 id, int32 posn, int32 size, int32 imgId, int32 flags)` (0x40A080 →
/// 0x15663B0): when the entry already has this size and `posn | img << 24` position, only
/// `RequestModel(id, flags)` (called through a computed return, `[0x1560510] ^ [0x540123]` =
/// 0x4087E0); otherwise RemoveModel, `offset = posn & 0xFFFFFF`, size, img byte, RequestModel.
#[hook(addr = 0x40A080, conv = cdecl, name = "CStreaming::RequestFile", expect = "E9 2B C3 15 01")]
pub fn request_file(id: i32, posn: u32, size: u32, img: i32, flags_arg: i32) {
    let want = posn | (img as u32) << 24;
    let sz = cd_size(id);
    if sz != 0 && cd_posn(id) == want && sz == size {
        call_request_model(id, flags_arg);
        return;
    }
    call_remove_model(id);
    let p = info(id);
    field::<u32, _>(p, 0x8).write(want & 0xFF_FFFF);
    field::<u32, _>(p, 0xC).write(size);
    field::<u8, _>(p, 0x7).write(img as u8);
    call_request_model(id, flags_arg);
}

/// `CStreaming::LoadInitialWeapons()` (0x40A120 → 0x1566460): RequestModel 346, 334, 345 (flags 2).
#[hook(addr = 0x40A120, conv = cdecl, name = "CStreaming::LoadInitialWeapons", expect = "E9 3B C3 15 01")]
pub fn load_initial_weapons() {
    call_request_model(0x15A, 2);
    call_request_model(0x14E, 2);
    call_request_model(0x159, 2);
}

/// `SetModelIsDeletable(id)` and its TXD (`SetModelAndItsTxdDeletable`), both called.
fn set_model_and_txd_deletable(id: i32) {
    call_set_model_is_deletable(id);
    call_set_model_is_deletable(mi_txd(mi(id)) + RESOURCE_ID_TXD);
}

/// `CStreaming::StreamCopModels(int32 level)` (0x40A150 → 0x156C100): only with `currArea == 0`.
/// The wanted/cop-bike part runs through computed calls and the no-CD indirection 0xCB73E0; it is
/// ported with gta-reversed's semantics, which match every decodable instruction (FindPlayerWanted
/// via `[0x1560574] + [0x63F8A9]`, level < 3 test, `ms_nTimePassedSinceLastCopBikeStreamedIn <
/// time` unsigned, toggle + `GetRandomNumberInRange(30000, 50000)` through 0x407180), bike index 4.
#[hook(addr = 0x40A150, conv = cdecl, name = "CStreaming::StreamCopModels", expect = "E9 AB 1F 16 01")]
pub fn stream_cop_models(level_arg: i32) {
    if curr_area() != 0 {
        return;
    }
    let mut lv = level_arg;
    let wanted = find_player_wanted(-1);
    if !wanted.is_null() {
        let w = find_player_wanted(-1);
        if field::<i32, u8>(w, 0x2C).read() < 3 && lv != 0 && !DISABLE_COP_BIKES.get().get() {
            let now = sa_core::timer::TIME_MS.get();
            if TIME_PASSED_SINCE_LAST_COP_BIKE.get() < now {
                COP_BIKE_LOADED.set(GBool::from_bool(!COP_BIKE_LOADED.get().get()));
                let r = sa_core::orig::at_site(0x156C194, || call_get_random_number_in_range_i32(30000, 50000));
                TIME_PASSED_SINCE_LAST_COP_BIKE.set((r as u32).wrapping_add(now));
            }
            if COP_BIKE_LOADED.get().get() {
                lv = 4;
            }
        } else {
            COP_BIKE_LOADED.set(GBool::FALSE);
        }
    } else {
        COP_BIKE_LOADED.set(GBool::FALSE);
    }
    if load_state(cop_model(lv)) == 1 && load_state(cop_car_model(lv)) == 1 {
        for i in 0..4 {
            if i != lv {
                if lv != 4 {
                    call_set_model_is_deletable(cop_model(i));
                    call_set_model_is_deletable(mi_txd(mi(cop_model(i))) + RESOURCE_ID_TXD);
                }
                call_set_model_is_deletable(cop_car_model(i));
                call_set_model_is_deletable(mi_txd(mi(cop_car_model(i))) + RESOURCE_ID_TXD);
            }
        }
    }
    call_request_model(cop_model(lv), 2);
    call_request_model(cop_car_model(lv), 2);
}

/// `CStreaming::StreamAmbulanceAndMedic(bool)` (0x40A2A0, AL): medics 1..3 and ambulances 1..3
/// deletable (model then TXD); for a level other than 0 and `stream`: request medic and ambulance of
/// the level (8), true when both are loaded.
#[hook(addr = 0x40A2A0, conv = cdecl, name = "CStreaming::StreamAmbulanceAndMedic", expect = "A1 C8 5A 8A 00")]
pub fn stream_ambulance_and_medic(stream: u8) -> bool {
    let lv = level();
    let medic = DEFAULT_MEDIC_MODEL.ptr().cast::<i32>();
    let amb = DEFAULT_AMBULANCE_MODEL.ptr().cast::<i32>();
    for i in 1..4 {
        set_model_and_txd_deletable(medic.wrapping_index(i).read());
    }
    for i in 1..4 {
        set_model_and_txd_deletable(amb.wrapping_index(i).read());
    }
    if lv != 0 && stream != 0 {
        call_request_model(medic.wrapping_index(lv).read(), 8);
        call_request_model(amb.wrapping_index(lv).read(), 8);
        return load_state(medic.wrapping_index(lv).read()) == 1 && load_state(amb.wrapping_index(lv).read()) == 1;
    }
    false
}

/// `CStreaming::StreamFireEngineAndFireman(bool)` (0x40A400 → 0x1570230, AL): as
/// StreamAmbulanceAndMedic with firemen / fire engines; the request part goes through the no-CD
/// indirection 0xCB73E0 and follows gta-reversed (request fireman then engine with 8, both loaded).
#[hook(addr = 0x40A400, conv = cdecl, name = "CStreaming::StreamFireEngineAndFireman", expect = "E9 2B 5E 16 01")]
pub fn stream_fire_engine_and_fireman(stream: u8) -> bool {
    let lv = level();
    let man = DEFAULT_FIREMAN_MODEL.ptr().cast::<i32>();
    let eng = DEFAULT_FIRE_ENGINE_MODEL.ptr().cast::<i32>();
    for i in 1..4 {
        set_model_and_txd_deletable(man.wrapping_index(i).read());
    }
    for i in 1..4 {
        set_model_and_txd_deletable(eng.wrapping_index(i).read());
    }
    if lv != 0 && stream != 0 {
        call_request_model(man.wrapping_index(lv).read(), 8);
        call_request_model(eng.wrapping_index(lv).read(), 8);
        return load_state(man.wrapping_index(lv).read()) == 1 && load_state(eng.wrapping_index(lv).read()) == 1;
    }
    false
}

fn ped_group_count(group: i32) -> i32 {
    g::<i16>(PED_GROUP_COUNT + 2 * group as usize).read() as i32
}
fn ped_group_model(group: i32, i: i32) -> i32 {
    g::<u16>(PED_GROUPS).wrapping_index(group * 21 + i).read() as i32
}
fn group_id(table: usize, i: i32) -> i32 {
    g::<i32>(table).wrapping_index(i).read()
}
fn car_group_model(group: i32, i: i32) -> i32 {
    g::<u16>(CAR_GROUPS).wrapping_index(group * 23 + i).read() as i32
}
fn car_group_count(group: i32) -> i32 {
    g::<i16>(CAR_GROUP_COUNT).wrapping_index(group).read() as i32
}

/// `CStreaming::StreamZoneModels_Gangs(const CVector&)` (0x40AA10; gr-deviations `Streaming.cpp:3642`
/// etc., keep-original). Gang bits from the zone's 10 gang densities (+ all 8 low bits with the gang
/// cheat 0x96915B), `CGangWars::TellStreamingWhichGangsAreNeeded`; when the mask changed: per gang,
/// newly needed → request two members (`CurrentGangMemberToLoad % n` and `+ 1`), no longer needed →
/// every member and TXD deletable (inlined); then the gang cars: with no car of the gang loaded, a
/// newly needed gang requests a random car of car group `gang` (0..9: the original indexes the car
/// groups by the gang number, not by the gang car groups 18..27), a gang no longer needed makes its
/// cars deletable unless they are in `m_AppropriateLoadedCars` or in car group `other` of another
/// loaded gang (`ms_loadedGangs`).
#[hook(addr = 0x40AA10, conv = cdecl, name = "CStreaming::StreamZoneModels_Gangs", expect = "A1 68 BC C0 00")]
pub fn stream_zone_models_gangs(_pos: GPtr<f32>) {
    let zi = g::<GPtr<u8>>(POP_CURR_ZONE_INFO).read();
    if zi.is_null() {
        return;
    }
    let mut mask: i32 = 0;
    for i in 0..10 {
        if zi.byte_add(i).read() != 0 {
            mask |= 1 << i;
        }
    }
    if g::<u8>(0x96915B).read() != 0 {
        mask |= 0xFF;
    }
    gang_wars_tell_streaming_which_gangs_are_needed(GPtr::from_raw(&raw mut mask));
    let cars_mask = mask;
    let loaded_gangs = LOADED_GANGS.get() as i16 as i32;
    if mask == loaded_gangs && mask == LOADED_GANG_CARS.get() as i16 as i32 {
        return;
    }
    for gang in 0..10 {
        let bit = 1i32 << gang;
        let group = group_id(GANG_GROUP_IDS, 3 * gang);
        let lg = LOADED_GANGS.get() as i16 as i32;
        if mask & bit != 0 && lg & bit == 0 {
            let n = ped_group_count(group);
            let cur = CURRENT_GANG_MEMBER_TO_LOAD.get();
            call_request_model(ped_group_model(group, cur.wrapping_rem(n)), 8);
            call_request_model(ped_group_model(group, cur.wrapping_add(1).wrapping_rem(ped_group_count(group))), 8);
            LOADED_GANGS.set(LOADED_GANGS.get() | bit as u16);
        } else if mask & bit == 0 && lg & bit != 0 {
            let mut i = 0;
            while i < ped_group_count(group) {
                let m = ped_group_model(group, i);
                set_model_is_deletable_inl(m);
                set_model_is_deletable_inl(mi_txd(mi(ped_group_model(group, i))) + RESOURCE_ID_TXD);
                i += 1;
            }
            LOADED_GANGS.set(LOADED_GANGS.get() & !(bit as u16));
        }
        let gang_cars = GPtr::<u8>::from_addr(LOADED_GANG_CARS_GROUPS + 0x2E * gang as usize);
        if car_group_count_members(gang_cars) < 1 {
            let lgc = LOADED_GANG_CARS.get() as i16 as i32;
            if cars_mask & bit != 0 && lgc & bit == 0 {
                // car group `gang` (0..9, not the gang groups 18..27: the original's indexing)
                let n = car_group_count(gang);
                let r = sa_core::orig::at_site(0x40ACD7, rand);
                let m = car_group_model(gang, r.wrapping_rem(n));
                if load_state(m) != 1 {
                    call_request_model(m, 8);
                }
            } else if cars_mask & bit == 0 && lgc & bit != 0 {
                let mut copy = gang_cars.cast::<[u16; 23]>().read();
                let local: GPtr<u8> = GPtr::from_raw(copy.as_mut_ptr()).cast();
                let mut j = 0;
                while j < car_group_count_members(local) {
                    let car = car_group_get_member(local, j);
                    let mut used = false;
                    let appropriate = g::<u8>(APPROPRIATE_LOADED_CARS);
                    let mut k2 = 0;
                    while k2 < car_group_count_members(appropriate) {
                        if car == car_group_get_member(appropriate, k2) {
                            used = true;
                        }
                        k2 += 1;
                    }
                    let lg2 = LOADED_GANGS.get() as i16 as i32;
                    for other in 0..10 {
                        if other != gang && lg2 & (1 << other) != 0 {
                            for s in 0..23 {
                                let v = g::<u16>(CAR_GROUPS).wrapping_index(other * 23 + s).read() as i32;
                                if v != 2000 && v == car {
                                    used = true;
                                }
                            }
                        }
                    }
                    if !used {
                        set_model_is_deletable_inl(car);
                        set_model_is_deletable_inl(mi_txd(mi(car)) + RESOURCE_ID_TXD);
                    }
                    j += 1;
                }
            }
        }
    }
    LOADED_GANG_CARS.set(cars_mask as u16);
}

/// `CStreaming::ReclassifyLoadedCars()` (0x40AFA0; the first copy's `mov ecx, 0xB` is in the chunk
/// 0x4059CD): copies of the appropriate and inappropriate groups, both cleared, every member of each
/// copy re-added to appropriate / inappropriate by `IsCarModelNeededInCurrentZone`.
#[hook(addr = 0x40AFA0, conv = cdecl, name = "CStreaming::ReclassifyLoadedCars", expect = "83 EC 60 56 57")]
pub fn reclassify_loaded_cars() {
    let mut a = g::<[u16; 23]>(APPROPRIATE_LOADED_CARS).read();
    let mut b = g::<[u16; 23]>(INAPPROPRIATE_LOADED_CARS).read();
    car_group_clear(g(APPROPRIATE_LOADED_CARS));
    car_group_clear(g(INAPPROPRIATE_LOADED_CARS));
    for copy in [GPtr::from_raw(a.as_mut_ptr()).cast::<u8>(), GPtr::from_raw(b.as_mut_ptr()).cast::<u8>()] {
        let mut i = 0;
        while i < car_group_count_members(copy) {
            let m = car_group_get_member(copy, i);
            let dst = if call_is_car_model_needed_in_current_zone(m) { APPROPRIATE_LOADED_CARS } else { INAPPROPRIATE_LOADED_CARS };
            car_group_add_member(g(dst), m);
            i += 1;
        }
    }
}

/// `CStreaming::RemoveCurrentZonesModels()` (0x40B080 → 0x15629D0): every loaded ped slot (and its TXD)
/// deletable (inlined), slots -1, count 0; RequestModel(7, 2); every gang's first 5 group members
/// (skipping 2000) and their TXDs deletable (inlined); zone type -1, gang masks 0.
#[hook(addr = 0x40B080, conv = cdecl, name = "CStreaming::RemoveCurrentZonesModels", expect = "E9 4B 79 15 01")]
pub fn remove_current_zones_models() {
    let peds = PEDS_LOADED.ptr().cast::<i32>();
    for i in 0..8 {
        let m = peds.wrapping_index(i).read();
        if m >= 0 {
            set_model_is_deletable_inl(m);
            set_model_is_deletable_inl(mi_txd(mi(peds.wrapping_index(i).read())) + RESOURCE_ID_TXD);
            peds.wrapping_index(i).write(-1);
        }
    }
    NUM_PEDS_LOADED.set(0);
    call_request_model(7, 2);
    for gang in 0..10 {
        let group = group_id(GANG_GROUP_IDS, 3 * gang);
        for i in 0..5 {
            let m = ped_group_model(group, i);
            if m != 2000 {
                set_model_is_deletable_inl(m);
                set_model_is_deletable_inl(mi_txd(mi(ped_group_model(group, i))) + RESOURCE_ID_TXD);
            }
        }
    }
    CURRENT_ZONE_TYPE.set(-1);
    LOADED_GANGS.set(0);
    LOADED_GANG_CARS.set(0);
}

/// `CStreaming::RemoveLoadedZoneModel()` (0x40B340 → 0x1566EF0, AL): with a current zone type and
/// `!CCheat::IsZoneStreamingAllowed()`: the first loaded, not required, unreferenced ped slot is
/// removed → true.
#[hook(addr = 0x40B340, conv = cdecl, name = "CStreaming::RemoveLoadedZoneModel", expect = "E9 AB BB 15 01")]
pub fn remove_loaded_zone_model() -> bool {
    if CURRENT_ZONE_TYPE.get() == -1 || cheat_is_zone_streaming_allowed() != 0 {
        return false;
    }
    for i in 0..8 {
        let m = PEDS_LOADED.ptr().cast::<i32>().wrapping_index(i).read();
        if m >= 0 && load_state(m) == 1 && flags(m) & 6 == 0 && mi_refs(mi(m)) == 0 {
            call_remove_model(m);
            return true;
        }
    }
    false
}

/// `CStreaming::RemoveInappropriatePedModels()` (0x40B3A0 → 0x156CA30): with a zone info: slots whose
/// ped is not appropriate for the zone (`CPopCycle::IsPedAppropriateForCurrentZone`) become deletable
/// (called), -1, count decremented; without: every slot, count 0.
#[hook(addr = 0x40B3A0, conv = cdecl, name = "CStreaming::RemoveInappropriatePedModels", expect = "E9 8B 16 16 01")]
pub fn remove_inappropriate_ped_models() {
    let peds = PEDS_LOADED.ptr().cast::<i32>();
    if !g::<GPtr<u8>>(POP_CURR_ZONE_INFO).read().is_null() {
        for i in 0..8 {
            let s = peds.wrapping_index(i);
            if s.read() >= 0 && popcycle_is_ped_appropriate_for_current_zone(s.read()) == 0 {
                call_set_model_is_deletable(s.read());
                call_set_model_is_deletable(mi_txd(mi(s.read())) + RESOURCE_ID_TXD);
                NUM_PEDS_LOADED.set(NUM_PEDS_LOADED.get() - 1);
                s.write(-1);
            }
        }
        return;
    }
    for i in 0..8 {
        let s = peds.wrapping_index(i);
        if s.read() >= 0 {
            call_set_model_is_deletable(s.read());
            call_set_model_is_deletable(mi_txd(mi(s.read())) + RESOURCE_ID_TXD);
            s.write(-1);
        }
    }
    NUM_PEDS_LOADED.set(0);
}

/// `CStreaming::RequestSpecialChar(int32 slot, const char* name, int32 flags)` (0x40B450): tail jump
/// into `RequestSpecialModel(slot + 290, name, flags)`.
#[hook(addr = 0x40B450, conv = cdecl, name = "CStreaming::RequestSpecialChar", expect = "8B 54 24 04 81 C2 22 01 00 00")]
pub fn request_special_char(slot: i32, name: GPtr<u8>, flags_arg: i32) {
    call_request_special_model(slot.wrapping_add(290), name, flags_arg)
}

/// `CStreaming::SetMissionDoesntRequireSpecialChar(int32)` (0x40B490; spec): tail jump with +290.
#[hook(addr = 0x40B490, conv = cdecl, name = "CStreaming::SetMissionDoesntRequireSpecialChar", expect = "8B 44 24 04 05 22 01 00 00")]
pub fn set_mission_doesnt_require_special_char(slot: i32) {
    call_set_mission_doesnt_require_model(slot.wrapping_add(290))
}

/// `CStreaming::SetMissionDoesntRequireAnim(int32)` (0x48B570; spec): tail jump with +25575.
#[hook(addr = 0x48B570, conv = cdecl, name = "CStreaming::SetMissionDoesntRequireAnim", expect = "8B 44 24 04 05 E7 63 00 00")]
pub fn set_mission_doesnt_require_anim(slot: i32) {
    call_set_mission_doesnt_require_model(slot.wrapping_add(RESOURCE_ID_IFP))
}

/// `CStreaming::LoadZoneVehicle(const CVector&)` (0x40B4B0): with a zone info: `GetZoneInfo(pos, 0)`
/// (result unused), `ChooseCarModelToLoad(PickARandomGroupOfOtherPeds())`, request (8) when >= 0.
#[hook(addr = 0x40B4B0, conv = cdecl, name = "CStreaming::LoadZoneVehicle", expect = "A1 68 BC C0 00")]
pub fn load_zone_vehicle(pos: GPtr<f32>) {
    if g::<GPtr<u8>>(POP_CURR_ZONE_INFO).read().is_null() {
        return;
    }
    zones_get_zone_info(pos, GPtr::NULL);
    let grp = popcycle_pick_a_random_group_of_other_peds();
    let m = car_ctrl_choose_car_model_to_load(grp);
    if m >= 0 {
        call_request_model(m, 8);
    }
}

/// `CStreaming::StreamOneNewCar()` (0x40B4F0 → 0x1563B30): with a zone info: the first active traffic
/// cheat in the original's order (0x969159 → group 4 (decoded `[0x15604D4] - [0x7059AA]`), 0x96917B
/// → 3, 0x96915E → 0x1E, 0x96915F → 0x1F, 0x96915C → 0x20, 0x969176 → 0x21; *new*: gr lets the last
/// active cheat win) picks the group; else boats when needed (`CountMembers(boats) < 2`, or `<= 2`
/// and `(rand() & 7) == 3`; counts called through computed returns); else taxi/cabbie: neither loaded
/// → after a taxi: cabbie when needed, else taxi when needed (*new*: gr never picks the cabbie
/// here); otherwise taxi when needed (computed call), else cabbie; the pick is remembered. Fallback:
/// `ChooseCarModelToLoad(PickARandomGroupOfOtherPeds())`. Request (8) +
/// `LoadSpecificDriverModelsForCar`.
#[hook(addr = 0x40B4F0, conv = cdecl, name = "CStreaming::StreamOneNewCar", expect = "E9 3B 86 15 01")]
pub fn stream_one_new_car() {
    if g::<GPtr<u8>>(POP_CURR_ZONE_INFO).read().is_null() {
        return;
    }
    let cheats: [(usize, i32); 6] =
        [(0x969159, 4), (0x96917B, 3), (0x96915E, 0x1E), (0x96915F, 0x1F), (0x96915C, 0x20), (0x969176, 0x21)];
    if let Some(&(_, grp)) = cheats.iter().find(|(a, _)| g::<u8>(*a).read() != 0) {
        let m = car_ctrl_choose_car_model_to_load(grp);
        if m < 0 {
            return;
        }
        call_request_model(m, 8);
        population_load_specific_driver_models_for_car(m);
        return;
    }
    let mut model = -1;
    if BOATS_NEEDED.get().get() {
        let boats = g::<u8>(LOADED_BOATS);
        let want = car_group_count_members(boats) < 2
            || (car_group_count_members(boats) <= 2 && sa_core::orig::at_site(0x1563BF1, rand) & 7 == 3);
        if want {
            model = car_ctrl_choose_car_model_to_load(0x1D);
        }
    }
    if model < 0 {
        if load_state(0x1A4) != 1 && load_state(0x1B6) != 1 {
            let pick = if LAST_CAR_MODEL_STREAMED_IN.get() == 0x1A4 {
                if call_is_car_model_needed_in_current_zone(0x1B6) {
                    0x1B6
                } else if call_is_car_model_needed_in_current_zone(0x1A4) {
                    0x1A4
                } else {
                    -1
                }
            } else if call_is_car_model_needed_in_current_zone(0x1A4) {
                0x1A4
            } else if call_is_car_model_needed_in_current_zone(0x1B6) {
                0x1B6
            } else {
                -1
            };
            if pick >= 0 {
                model = pick;
                LAST_CAR_MODEL_STREAMED_IN.set(pick);
            }
        }
        if model < 0 {
            let grp = popcycle_pick_a_random_group_of_other_peds();
            model = car_ctrl_choose_car_model_to_load(grp);
            if model < 0 {
                return;
            }
        }
    }
    call_request_model(model, 8);
    population_load_specific_driver_models_for_car(model);
}

/// `CStreaming::StreamVehiclesAndPeds_Always(const CVector&)` (0x40B650): skipped when the player's
/// vehicle is a train (type 4, +0x594) or a plane (3) moving faster than 0.1 (`sqrt(vx² + vy²)`);
/// every 64 frames with fewer than 3 appropriate cars StreamOneNewCar; StreamZoneModels_Gangs; when
/// the zone's population type (`zoneInfo+0xF & 0x1F`) changed: ReclassifyLoadedCars.
#[hook(addr = 0x40B650, conv = cdecl, name = "CStreaming::StreamVehiclesAndPeds_Always", expect = "6A 00 6A FF E8 77 2A 16 00")]
pub fn stream_vehicles_and_peds_always(pos: GPtr<f32>) {
    let v = find_player_vehicle(-1, 0);
    if !v.is_null() {
        let t = field::<i32, u8>(v, 0x594).read();
        if t == 4 {
            return;
        }
        if t == 3 {
            let vx = x(field::<f32, u8>(v, 0x44).read());
            let vy = x(field::<f32, u8>(v, 0x48).read());
            let s = vx.fmul(vx).fadd(vy.fmul(vy)).fsqrt();
            if !s.fcom(x(f32::from_bits(0x3DCC_CCCD))).test_any(0x41) {
                return;
            }
        }
    }
    if g::<u8>(TIMER_FRAME_COUNTER).read() & 0x3F == 0 && car_group_count_members(g(APPROPRIATE_LOADED_CARS)) < 3 {
        call_stream_one_new_car();
    }
    call_stream_zone_models_gangs(pos);
    let zi = g::<GPtr<u8>>(POP_CURR_ZONE_INFO).read();
    if !zi.is_null() && (zi.byte_add(0xF).read() & 0x1F) as i32 != LAST_ZONE_POPULATION_TYPE.get() {
        call_reclassify_loaded_cars();
        LAST_ZONE_POPULATION_TYPE.set((g::<GPtr<u8>>(POP_CURR_ZONE_INFO).read().byte_add(0xF).read() & 0x1F) as i32);
    }
}

/// `CStreaming::StreamVehiclesAndPeds()` (0x40B700): cop models for the level; SWAT/FBI/army
/// vehicles and peds requested or deletable by the wanted state (each CWanted query on the result
/// of a fresh `FindPlayerWanted(-1)`); helis; the dealers' group (0x8D2690): one member (by the weather/zone rule below) requested
/// (2), the others deletable (inlined); every 350 frames a new car when the loaded count is within
/// `desiredNumVehiclesLoaded`; harvester models (0x8CD6E4..) requested while
/// `m_bStreamHarvesterModelsThisFrame`, removed once no longer needed.
/// Dealer pick: `CWeather::Rain`-style value at 0xC0E978 > 0.03 → (`CGame::currLevel`-like 0xC81314
/// == 2 → 3; else zone info +0x10 bit 0 → 0, bit 1 → 1, else 2).
#[hook(addr = 0x40B700, conv = cdecl, name = "CStreaming::StreamVehiclesAndPeds", expect = "A1 18 67 BA 00")]
pub fn stream_vehicles_and_peds() {
    call_stream_cop_models(level());
    let w = find_player_wanted(-1);
    if wanted_are_swat_required(w) != 0 {
        call_request_model(0x1AB, 2);
        call_request_model(0x11D, 2);
    } else {
        call_set_model_is_deletable(0x1AB);
        if load_state(0x1AB) != 1 {
            call_set_model_is_deletable(0x11D);
        }
    }
    let w = find_player_wanted(-1);
    if wanted_are_fbi_required(w) != 0 {
        call_request_model(0x1EA, 2);
        call_request_model(0x11E, 2);
    } else {
        call_set_model_is_deletable(0x1EA);
        if load_state(0x1EA) != 1 {
            call_set_model_is_deletable(0x11E);
        }
    }
    let w = find_player_wanted(-1);
    if wanted_are_army_required(w) != 0 {
        call_request_model(0x1B0, 2);
        call_request_model(0x1B1, 2);
        call_request_model(0x11F, 2);
    } else {
        call_set_model_is_deletable(0x1B1);
        call_set_model_is_deletable(0x1B0);
        if load_state(0x1B1) != 1 && load_state(0x1B0) != 1 {
            call_set_model_is_deletable(0x11F);
        }
    }
    let w = find_player_wanted(-1);
    if wanted_num_of_helis_required(w) < 1 {
        call_set_model_is_deletable(0x1E8);
        call_set_model_is_deletable(0x1F1);
    } else {
        call_request_model(0x1F1, 2);
        let w = find_player_wanted(-1);
        if wanted_num_of_helis_required(w) < 2 || g::<u8>(0xB7CB8C).read() == 0 {
            call_set_model_is_deletable(0x1E8);
        } else {
            call_request_model(0x1E8, 2);
        }
    }
    let mut pick = -1;
    if !x(g::<f32>(0xC0E978).read()).fcom(x(k(0x3CF5_C28F))).test_any(0x41) {
        if g::<i32>(0xC81314).read() == 2 {
            pick = 3;
        } else {
            let zi = g::<GPtr<u8>>(POP_CURR_ZONE_INFO).read();
            if !zi.is_null() {
                let f = zi.byte_add(0x10).read();
                pick = if f & 1 != 0 { 0 } else { 2 - ((f & 2) != 0) as i32 };
            }
        }
    }
    let mut i = 0;
    while i < ped_group_count(g::<i32>(DEALER_GROUP_ID).read()) {
        let m = ped_group_model(g::<i32>(DEALER_GROUP_ID).read(), i);
        if i == pick {
            call_request_model(m, 2);
        } else {
            set_model_is_deletable_inl(m);
        }
        i += 1;
    }
    if FRAMES_BEFORE_NEXT_NEW_CAR.get() < 0 {
        if car_group_count_members(VEHICLES_LOADED.ptr().cast()) <= DESIRED_NUM_VEHICLES_LOADED.get()
            && !g::<GPtr<u8>>(POP_CURR_ZONE_INFO).read().is_null()
        {
            call_stream_one_new_car();
            FRAMES_BEFORE_NEXT_NEW_CAR.set(0x15E);
        }
    } else {
        FRAMES_BEFORE_NEXT_NEW_CAR.set(FRAMES_BEFORE_NEXT_NEW_CAR.get() - 1);
    }
    let harv = |i: i32| g::<i32>(HARVESTER_MODELS).wrapping_index(i).read();
    if !STREAM_HARVESTER_MODELS_THIS_FRAME.get().get() {
        if HARVESTER_MODELS_REQUESTED.get().get() {
            for i in 0..4 {
                call_remove_model(harv(i));
            }
            HARVESTER_MODELS_REQUESTED.set(GBool::FALSE);
        }
        STREAM_HARVESTER_MODELS_THIS_FRAME.set(GBool::FALSE);
        return;
    }
    for i in 0..4 {
        call_request_model(harv(i), 2);
    }
    HARVESTER_MODELS_REQUESTED.set(GBool::TRUE);
    STREAM_HARVESTER_MODELS_THIS_FRAME.set(GBool::FALSE);
}

/// `CStreaming::PossiblyStreamCarOutAfterCreation(int32)` (0x40BA70 → 0x156C5B0): vehicle model info
/// flag byte +0x4E bit 0 and `rand() & 1` → `SetModelIsDeletable(model)` (called through a computed
/// return, `[0x156057C] - [0x6822B7] - 1` = 0x409C10).
#[hook(addr = 0x40BA70, conv = cdecl, name = "CStreaming::PossiblyStreamCarOutAfterCreation", expect = "E9 3B 0B 16 01")]
pub fn possibly_stream_car_out_after_creation(model: i32) {
    if mi(model).byte_add(0x4E).read() & 1 != 0 && sa_core::orig::at_site(0x156C5C7, rand) & 1 != 0 {
        call_set_model_is_deletable(model);
    }
}

/// `CStreaming::ClearSlots(int32 n)` (0x40BAA0): the first `n` ped slots (inlined deletable for the
/// model and its TXD), count decremented, slot -1.
#[hook(addr = 0x40BAA0, conv = cdecl, name = "CStreaming::ClearSlots", expect = "8B 44 24 04 57")]
pub fn clear_slots(n: i32) {
    let peds = PEDS_LOADED.ptr().cast::<i32>();
    let mut i = 0;
    while i < n {
        let m = peds.wrapping_index(i).read();
        if m >= 0 {
            set_model_is_deletable_inl(m);
            set_model_is_deletable_inl(mi_txd(mi(peds.wrapping_index(i).read())) + RESOURCE_ID_TXD);
            NUM_PEDS_LOADED.set(NUM_PEDS_LOADED.get() - 1);
            peds.wrapping_index(i).write(-1);
        }
        i += 1;
    }
}

/// `CStreaming::StreamPedsForInterior(int32 type)` (0x40BBB0; gr-deviations `Streaming.cpp:3271`,
/// `:3306`, `:3317`, keep-original; spec CStreaming.deviations.md). Rand sites 0x40BCD1, 0x40BCDB
/// (type 0), 0x40BC7A (type 1), 0x40BBEF (type 2). Type 2's group count is read through a
/// protection chunk (0x4084B5) that decodes to the office group table 0x8D26A8.
#[hook(addr = 0x40BBB0, conv = cdecl, name = "CStreaming::StreamPedsForInterior", expect = "51 8B 44 24 08")]
pub fn stream_peds_for_interior(kind: i32) {
    let zone = g::<i32>(POP_CURRENT_WORLD_ZONE).read();
    let peds = PEDS_LOADED.ptr().cast::<i32>();
    match kind {
        0 => {
            let nh = ped_group_count(group_id(HUSBAND_GROUP_IDS, 0));
            let husband = sa_core::orig::at_site(0x40BCD1, rand).wrapping_rem(nh);
            let r = sa_core::orig::at_site(0x40BCDB, rand) & 0xFFFF;
            let rr = X87::from_i32(r).fmul_m(k(F_INV_32768)).fmul_m(k(F_6)).ftol() as u32 as i32;
            let mut wife = husband.wrapping_sub(rr).wrapping_add(3);
            if wife < 0 {
                wife = 0;
            }
            let nw = ped_group_count(group_id(WIFE_GROUP_IDS, 0)) - 1;
            if nw < wife {
                wife = nw;
            }
            let hm = ped_group_model(group_id(HUSBAND_GROUP_IDS, zone), husband);
            let wm = ped_group_model(group_id(WIFE_GROUP_IDS, zone), wife);
            call_clear_slots(2);
            call_request_model(hm, 8);
            peds.write(hm);
            NUM_PEDS_LOADED.set(NUM_PEDS_LOADED.get() + 1);
            call_request_model(wm, 8);
            NUM_PEDS_LOADED.set(NUM_PEDS_LOADED.get() + 1);
            peds.wrapping_index(1).write(wm);
        }
        1 => {
            let n = ped_group_count(group_id(SHOPKEEPER_GROUP_IDS, 0));
            let rem = sa_core::orig::at_site(0x40BC7A, rand).wrapping_rem(n);
            let m = ped_group_model(group_id(SHOPKEEPER_GROUP_IDS, zone), rem);
            call_clear_slots(1);
            call_request_model(m, 8);
            peds.write(m);
            NUM_PEDS_LOADED.set(NUM_PEDS_LOADED.get() + 1);
        }
        2 => {
            call_clear_slots(8);
            let n = ped_group_count(group_id(OFFICE_GROUP_IDS, 0));
            let r = sa_core::orig::at_site(0x40BBEF, rand) & 0xFFFF;
            let rr = X87::from_i32(r).fmul_m(k(F_INV_32768)).fmul(X87::from_i32(n)).ftol() as u32 as i32;
            for i in 0..8i32 {
                let cnt = ped_group_count(group_id(OFFICE_GROUP_IDS, 0));
                let zone_now = g::<i32>(POP_CURRENT_WORLD_ZONE).read();
                let m = ped_group_model(group_id(OFFICE_GROUP_IDS, zone_now), i.wrapping_add(rr).wrapping_rem(cnt));
                call_request_model(m, 8);
                NUM_PEDS_LOADED.set(NUM_PEDS_LOADED.get() + 1);
                peds.wrapping_index(i).write(m);
            }
        }
        _ => {}
    }
}

/// `CStreaming::StreamPedsIntoRandomSlots(const int32 models[8])` (0x40BDA0 → 0x15703E0): per slot:
/// a model >= 0 replaces the slot (old one deletable, called; request 8); -2 just clears the slot.
#[hook(addr = 0x40BDA0, conv = cdecl, name = "CStreaming::StreamPedsIntoRandomSlots", expect = "E9 3B 46 16 01")]
pub fn stream_peds_into_random_slots(models: GPtr<i32>) {
    let peds = PEDS_LOADED.ptr().cast::<i32>();
    for i in 0..8 {
        let want = models.wrapping_index(i).read();
        let s = peds.wrapping_index(i);
        if want < 0 {
            if want == -2 && s.read() >= 0 {
                call_set_model_is_deletable(s.read());
                call_set_model_is_deletable(mi_txd(mi(s.read())) + RESOURCE_ID_TXD);
                s.write(-1);
                NUM_PEDS_LOADED.set(NUM_PEDS_LOADED.get() - 1);
            }
        } else {
            if s.read() >= 0 {
                call_set_model_is_deletable(s.read());
                call_set_model_is_deletable(mi_txd(mi(s.read())) + RESOURCE_ID_TXD);
                NUM_PEDS_LOADED.set(NUM_PEDS_LOADED.get() - 1);
                s.write(-1);
            }
            call_request_model(models.wrapping_index(i).read(), 8);
            s.write(models.wrapping_index(i).read());
            NUM_PEDS_LOADED.set(NUM_PEDS_LOADED.get() + 1);
        }
    }
}

/// `CStreaming::RemoveDodgyPedsFromRandomSlots()` (0x40BE60): slots holding 0x57, 0xF4, 0xF6, 0x100 or
/// 0x101 are cleared (inlined deletable, count decremented).
#[hook(addr = 0x40BE60, conv = cdecl, name = "CStreaming::RemoveDodgyPedsFromRandomSlots", expect = "53 56 57 BF 00 4C 8E 00")]
pub fn remove_dodgy_peds_from_random_slots() {
    let peds = PEDS_LOADED.ptr().cast::<i32>();
    for i in 0..8 {
        let m = peds.wrapping_index(i).read();
        if matches!(m, 0x57 | 0xF4 | 0xF6 | 0x100 | 0x101) {
            set_model_is_deletable_inl(m);
            set_model_is_deletable_inl(mi_txd(mi(peds.wrapping_index(i).read())) + RESOURCE_ID_TXD);
            NUM_PEDS_LOADED.set(NUM_PEDS_LOADED.get() - 1);
            peds.wrapping_index(i).write(-1);
        }
    }
}

/// A removable vehicle: no refs, not game/mission required, loaded.
fn removable_vehicle(m: i32) -> bool {
    mi_refs(mi(m)) == 0 && flags(m) & 6 == 0 && load_state(m) == 1
}
fn remove_first_removable(group: GPtr<u8>) -> bool {
    let mut i = 0;
    while i < car_group_count_members(group) {
        let m = car_group_get_member(group, i);
        if removable_vehicle(m) {
            call_remove_model(m);
            return true;
        }
        i += 1;
    }
    false
}

/// `CStreaming::RemoveLoadedVehicle()` (0x40C020, AL): the first removable inappropriate car; else (when
/// more boats than needed: 2 with `m_bBoatsNeeded`, else 0) the first removable boat; else the
/// appropriate cars copied and sorted by usage, the first removable.
#[hook(addr = 0x40C020, conv = cdecl, name = "CStreaming::RemoveLoadedVehicle", expect = "83 EC 30 53 56")]
pub fn remove_loaded_vehicle() -> bool {
    if remove_first_removable(g(INAPPROPRIATE_LOADED_CARS)) {
        return true;
    }
    let keep = if BOATS_NEEDED.get().get() { 2 } else { 0 };
    if keep < car_group_count_members(g(LOADED_BOATS)) && remove_first_removable(g(LOADED_BOATS)) {
        return true;
    }
    let mut copy = g::<[u16; 23]>(APPROPRIATE_LOADED_CARS).read();
    let local: GPtr<u8> = GPtr::from_raw(copy.as_mut_ptr()).cast();
    car_group_sort_based_on_usage(local);
    remove_first_removable(local)
}

/// `CStreaming::RemoveTxdModel(int32 slot)` (0x40C180): `RemoveModel(slot + 20000)` (tail jump).
#[hook(addr = 0x40C180, conv = cdecl, name = "CStreaming::RemoveTxdModel", expect = "8B 44 24 04 05 20 4E 00 00")]
pub fn remove_txd_model(slot: i32) {
    call_remove_model(slot.wrapping_add(RESOURCE_ID_TXD))
}

/// Walks the requested list backwards from `ms_pEndRequestedList.prev` to the start, removing every
/// entry whose flags have none of `mask` (`PurgeRequestList` uses 0x1E, `LoadScene` 0x3E).
fn purge_requested(mask: u8) {
    let start = START_REQUESTED_LIST.get();
    let mut p = prev_of(END_REQUESTED_LIST.get());
    while p != start {
        let prev = prev_of(p);
        if field::<u8, _>(p, 6).read() & mask == 0 {
            call_remove_model(id_of(p));
        }
        p = prev;
    }
}

/// `CStreaming::PurgeRequestList()` (0x40C1E0).
#[hook(addr = 0x40C1E0, conv = cdecl, name = "CStreaming::PurgeRequestList", expect = "A1 54 4C 8E 00")]
pub fn purge_request_list() {
    purge_requested(0x1E);
}

/// The common entity filter of both `ProcessEntitiesInSectorList`s: new scan code (stamped), not
/// loaded, not `m_bStreamingDontDelete`, in the current area (or 13), visible and streamable, and the
/// time info (if any) in range. Returns the model info.
fn sector_entity_wanted(e: GPtr<u8>) -> Option<GPtr<u8>> {
    let sc = g::<u16>(SCAN_CODE).read();
    if field::<u16, u8>(e, 0x2C).read() == sc {
        return None;
    }
    field::<u16, u8>(e, 0x2C).write(sc);
    if load_state(e_model(e)) == 1 {
        return None;
    }
    let f = e_flags(e);
    if f & 0x400 != 0 || !in_current_area(e) || f & 0x80000 != 0 || f & 0x80 == 0 {
        return None;
    }
    let m = mi(e_model(e));
    let ti = mi_get_time_info(m);
    if !ti.is_null() && clock_get_is_time_in_range(ti.read(), ti.byte_add(1).read()) == 0 {
        return None;
    }
    Some(m)
}

/// `CStreaming::ProcessEntitiesInSectorList(list, int32 flags)` (0x40C450; spec CStreaming.md).
#[hook(addr = 0x40C450, conv = cdecl, name = "CStreaming::ProcessEntitiesInSectorList", expect = "8B 44 24 04 55")]
pub fn process_entities_in_sector_list(list: GPtr<u8>, flags_arg: i32) {
    for_each_in_list(list, |e| {
        if let Some(m) = sector_entity_wanted(e) {
            if !field::<GPtr<u8>, u8>(m, 0x1C).read().is_null() && e_rw(e).is_null() {
                entity_create_rw_object(e);
            }
            call_request_model(e_model(e), flags_arg);
        }
    });
}

/// `CStreaming::ProcessEntitiesInSectorList(list, x, y, minX, minY, maxX, maxY, radius, flags)`
/// (0x40C270): `radius² ` stored f32 first; per wanted entity: limit = `(LODMult * drawDist)²`
/// (x87) unless `radius² < it` (then radius²), stored f32; the position strictly inside
/// `(minX, maxX) × (minY, maxY)` and `(y - ey)² + (x - ex)² < limit` → CreateRwObject (when the model
/// has an RW object and the entity none) and RequestModel.
#[hook(addr = 0x40C270, conv = cdecl, name = "CStreaming::ProcessEntitiesInSectorList<CPtrListSingleLink<CEntity*>>-", expect = "51 D9 44 24 24")]
#[allow(clippy::too_many_arguments)]
pub fn process_entities_in_sector_list_radius(
    list: GPtr<u8>,
    px: f32,
    py: f32,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    radius: f32,
    flags_arg: i32,
) {
    let r2 = x(radius).fmul_m(radius).to_f32();
    for_each_in_list(list, |e| {
        let Some(m) = sector_entity_wanted(e) else { return };
        let a = x(g::<f32>(CAMERA + 0xF0).read()).fmul_m(field::<f32, u8>(m, 0x18).read());
        let lod = a.fmul(a);
        let limit = if x(r2).fcom(lod).test_parity(0x05) { lod.to_f32() } else { r2 };
        let p = e_pos(e);
        if x(p.read()).fcom(x(min_x)).test_any(0x41) {
            return;
        }
        if x(p.read()).fcom(x(max_x)).test_parity(0x05) {
            return;
        }
        if x(p.wrapping_index(1).read()).fcom(x(min_y)).test_any(0x41) {
            return;
        }
        if x(p.wrapping_index(1).read()).fcom(x(max_y)).test_parity(0x05) {
            return;
        }
        let dy = x(py).fsub_m(p.wrapping_index(1).read());
        let dx = x(px).fsub_m(p.read());
        let d2 = dy.fmul(dy).fadd(dx.fmul(dx));
        if d2.fcom(x(limit)).test_parity(0x05) {
            return;
        }
        if !field::<GPtr<u8>, u8>(m, 0x1C).read().is_null() && e_rw(e).is_null() {
            entity_create_rw_object(e);
        }
        call_request_model(e_model(e), flags_arg);
    });
}

/// `CStreaming::AddLodsToRequestList(const CVector&, int32 flags)` (0x40C520): `d =
/// CRenderer::ms_fFarClipPlane`-like 0xB76848; bounds `x ± d`, `y ± d` stored f32; LOD grid cells
/// `_ftol(v * 0.005 + 15)` (no floor) clamped to 0..=29; scan code advanced (0xFFFF →
/// `CWorld::ClearScanCodes`, 1); every cell's LOD list through the radius variant.
#[hook(addr = 0x40C520, conv = cdecl, name = "CStreaming::AddLodsToRequestList", expect = "83 EC 20 A1 48 68 B7 00")]
pub fn add_lods_to_request_list(pos: GPtr<f32>, flags_arg: i32) {
    let d = g::<f32>(0xB76848).read();
    let (px, py) = (pos.read(), pos.wrapping_index(1).read());
    let min_x = x(px).fsub_m(d).to_f32();
    let max_x = x(d).fadd_m(px).to_f32();
    let min_y = x(py).fsub_m(d).to_f32();
    let max_y = x(d).fadd_m(py).to_f32();
    let cell = |v: f32| x(v).fmul_m(f32::from_bits(0x3BA3_D70A)).fadd_m(f32::from_bits(0x4170_0000)).ftol() as u32 as i32;
    let x0 = cell(min_x).max(0);
    let y0 = cell(min_y).max(0);
    let x1 = cell(max_x).min(0x1D);
    let y1 = cell(max_y).min(0x1D);
    let sc = g::<u16>(SCAN_CODE);
    if sc.read() == 0xFFFF {
        world_clear_scan_codes();
        sc.write(1);
    } else {
        sc.write(sc.read() + 1);
    }
    let mut cy = y0;
    while cy <= y1 {
        let mut cx = x0;
        while cx <= x1 {
            let list = GPtr::<u8>::from_addr(LOD_LISTS + 4 * (cy * 30 + cx) as usize);
            call_process_entities_in_sector_list_radius(
                list,
                pos.read(),
                pos.wrapping_index(1).read(),
                min_x,
                min_y,
                max_x,
                max_y,
                d,
                flags_arg,
            );
            cx += 1;
        }
        cy += 1;
    }
}

// ------------------------------------------------------------------------- loading core ---

/// `RwMemory` (the `_rwStreamInitialize` data of a memory stream).
#[repr(C)]
#[derive(Clone, Copy)]
struct RwMemory {
    start: GPtr<u8>,
    length: u32,
}

/// `_rwStreamInitialize(&gRwStream, 0, rwSTREAMMEMORY, rwSTREAMREAD, &mem)`.
fn open_memory_stream(mem: &mut RwMemory) -> GPtr<u8> {
    rw_stream_initialize(RW_STREAM.ptr().cast(), 0, 3, 1, GPtr::from_raw(mem as *mut RwMemory).cast())
}
fn close_stream(s: GPtr<u8>, mem: &mut RwMemory) {
    rw_stream_close(s, GPtr::from_raw(mem as *mut RwMemory).cast());
}

/// `CStreaming::RemoveModel(int32)` (0x4089A0). Loaded (1): DFF → `DeleteRwObject`, a ped model is
/// cleared from the ped slots, a vehicle model `RemoveCarModel`; TXD/COL/IPL/DAT/IFP/SCM unloaded by
/// their stores (RRR: nothing); `memoryUsed -= size * 2048`. Not in a list: when reading (3), the id is
/// cleared from both channels; in a list: requested (2) → `--ms_numModelsRequested` and the priority
/// count, unlinked. Finishing (4): DFF `RpClumpGtaCancelStream`, TXD/COL/IPL/IFP/SCM unloaded. Then
/// state 0.
#[hook(addr = 0x4089A0, conv = cdecl, name = "CStreaming::RemoveModel", expect = "56 8B 74 24 08")]
pub fn remove_model(id: i32) {
    let st = load_state(id);
    if st == 0 {
        return;
    }
    if st == 1 {
        if id < 20000 {
            let m = mi(id);
            mi_delete_rw_object(m);
            if mi_get_model_type(m) == 7 {
                let peds = PEDS_LOADED.ptr().cast::<i32>();
                for i in 0..8 {
                    if peds.wrapping_index(i).read() == id {
                        peds.wrapping_index(i).write(-1);
                        NUM_PEDS_LOADED.set(NUM_PEDS_LOADED.get() - 1);
                    }
                }
            }
            if mi_get_model_type(m) == 6 {
                call_remove_car_model(id);
            }
        } else if id < RESOURCE_ID_COL {
            txd_remove_txd(id - RESOURCE_ID_TXD);
        } else if id < RESOURCE_ID_IPL {
            col_remove_col(id - RESOURCE_ID_COL);
        } else if id < RESOURCE_ID_DAT {
            ipl_remove_ipl(id - RESOURCE_ID_IPL);
        } else if id < RESOURCE_ID_IFP {
            path_find_unload_path_find_data(g(0x96F050), id - RESOURCE_ID_DAT);
        } else if id < RESOURCE_ID_RRR {
            anim_remove_anim_block(id - RESOURCE_ID_IFP);
        } else if id > RESOURCE_ID_SCM - 1 {
            streamed_scripts_remove_from_memory(g(0xA47B60), id - RESOURCE_ID_SCM);
        }
        MEMORY_USED.set(MEMORY_USED.get().wrapping_add(cd_size(id).wrapping_mul(0xFFFF_F800)));
    }
    if next_index(id) == -1 {
        if load_state(id) == 3 {
            for i in 0..16 {
                for ch in 0..2 {
                    if ch_id(ch, i).read() == id {
                        ch_id(ch, i).write(-1);
                    }
                }
            }
        }
    } else {
        if load_state(id) == 2 {
            NUM_MODELS_REQUESTED.set(NUM_MODELS_REQUESTED.get() - 1);
            if flags(id) & 0x10 != 0 {
                NUM_PRIORITY_REQUESTS.set(NUM_PRIORITY_REQUESTS.get().wrapping_sub(1));
                set_flags(id, flags(id) & 0xEF);
            }
        }
        let p = info(id);
        let prev = field::<i16, _>(p, 2).read();
        let base = STREAMING_INFO_ARRAY_BASE.get();
        field::<i16, _>(base.wrapping_index(next_index(id) as i32), 2).write(prev);
        let next = next_index(id);
        let base = STREAMING_INFO_ARRAY_BASE.get();
        field::<i16, _>(base.wrapping_index(field::<i16, _>(p, 2).read() as i32), 0).write(next);
        field::<i16, _>(p, 0).write(-1);
        field::<i16, _>(p, 2).write(-1);
    }
    if load_state(id) == 4 {
        if id < 20000 {
            rp_clump_gta_cancel_stream();
        } else if id < RESOURCE_ID_COL {
            txd_remove_txd(id - RESOURCE_ID_TXD);
        } else if id < RESOURCE_ID_IPL {
            col_remove_col(id - RESOURCE_ID_COL);
        } else if id < RESOURCE_ID_DAT {
            ipl_remove_ipl(id - RESOURCE_ID_IPL);
        } else if (RESOURCE_ID_IFP..RESOURCE_ID_RRR).contains(&id) {
            anim_remove_anim_block(id - RESOURCE_ID_IFP);
        } else if id > RESOURCE_ID_SCM - 1 {
            streamed_scripts_remove_from_memory(g(0xA47B60), id - RESOURCE_ID_SCM);
        }
    }
    set_load_state(id, 0);
}

/// `CStreaming::RequestVehicleUpgrade(int32 model, int32 flags)` (0x408C70 → 0x156C970): the model and
/// its linked upgrade (`CVehicleModelInfo::ms_linkedUpgrades.FindOtherUpgrade`, 0xB4E6D8).
#[hook(addr = 0x408C70, conv = cdecl, name = "CStreaming::RequestVehicleUpgrade", expect = "E9 FB 3C 16 01")]
pub fn request_vehicle_upgrade(model: i32, flags_arg: i32) {
    call_request_model(model, flags_arg);
    let other = linked_upgrades_find_other_upgrade(g(LINKED_UPGRADES), model);
    if other != -1 {
        call_request_model(other as i32, flags_arg);
    }
}
/// `CVehicleModelInfo::ms_linkedUpgrades` (0xB4E6D8)
const LINKED_UPGRADES: usize = 0xB4E6D8;

/// `CStreaming::HasVehicleUpgradeLoaded(int32)` (0x407820 → 0x156A100, AL): loaded, and its linked
/// upgrade (if any) loaded.
#[hook(addr = 0x407820, conv = cdecl, name = "CStreaming::HasVehicleUpgradeLoaded", expect = "E9 DB 28 16 01")]
pub fn has_vehicle_upgrade_loaded(model: i32) -> bool {
    let st = load_state(model);
    if st == 1 {
        let other = linked_upgrades_find_other_upgrade(g(LINKED_UPGRADES), model);
        if other != -1 {
            return load_state(other as i32) == 1;
        }
    }
    st == 1
}

/// `CStreaming::FinishLoadingLargeFile(uint8* buf, int32 id)` (0x408CB0, AL). Not finishing (4): a DFF
/// drops its model info ref; false. Otherwise a memory stream over `size * 2048` bytes: DFF →
/// `SetCurrentTxd`, `FinishLoadClumpFile`, then `AddToLoadedVehiclesList` on success, model info ref
/// and TXD ref (without delete) dropped, anim block ref dropped; TXD → `AddRef`, `FinishLoadTxd`,
/// `RemoveRefWithoutDelete`; any other id: the result is the id's low byte. Stream closed, memory used
/// += size * 2048, state 1; failure → RemoveModel + RequestModel(id, flags), false.
#[hook(addr = 0x408CB0, conv = cdecl, name = "CStreaming::FinishLoadingLargeFile", expect = "83 EC 08 56 8B 74 24 14")]
pub fn finish_loading_large_file(buf: GPtr<u8>, id: i32) -> bool {
    if load_state(id) != 4 {
        if id < 20000 {
            mi_remove_ref(mi(id));
        }
        return false;
    }
    let mut mem = RwMemory { start: buf, length: cd_size(id) << 11 };
    let s = open_memory_stream(&mut mem);
    let ok = if id < 20000 {
        let m = mi(id);
        txd_set_current_txd(mi_txd(m));
        let mut ok = false;
        if fl_finish_load_clump_file(s, id) != 0 {
            ok = call_add_to_loaded_vehicles_list(id);
        }
        mi_remove_ref(m);
        txd_remove_ref_without_delete(mi_txd(m));
        if mi_get_anim_file_index(m) != -1 {
            anim_remove_anim_block_ref_without_delete(mi_get_anim_file_index(m));
        }
        ok
    } else if id < RESOURCE_ID_COL {
        txd_add_ref(id - RESOURCE_ID_TXD);
        let ok = txd_finish_load_txd(id - RESOURCE_ID_TXD, s) != 0;
        txd_remove_ref_without_delete(id - RESOURCE_ID_TXD);
        ok
    } else {
        id as u8 != 0
    };
    close_stream(s, &mut mem);
    MEMORY_USED.set(MEMORY_USED.get().wrapping_add(cd_size(id) << 11));
    set_load_state(id, 1);
    if !ok {
        call_remove_model(id);
        call_request_model(id, flags(id) as i32);
        return false;
    }
    true
}

/// `CStreaming::GetNextFileOnCd(uint32 pos, bool priorityOnly)` (0x408E20; gr-deviations
/// `Streaming.cpp:386`, keep-original): walks the requested list; a DFF whose TXD (or anim) is neither
/// loaded nor being read requests it (8 for anims, the model's flags for the TXD) and is skipped;
/// a TXD whose parent is not loaded/reading likewise; an IFP during a cutscene or while model 0x7
/// (`info(7)` at 0x8E4D5C) is not loaded is skipped. Of the rest: the lowest full position, and the
/// lowest at or after `pos` (unsigned). Nothing found with priority requests pending → the count is
/// zeroed and the search repeats without the priority filter.
#[hook(addr = 0x408E20, conv = cdecl, name = "CStreaming::GetNextFileOnCd", expect = "83 EC 0C A1 58 4C 8E 00")]
pub fn get_next_file_on_cd(pos: u32, priority_only: u8) -> i32 {
    let mut prio = priority_only != 0;
    loop {
        let mut first_pos = u32::MAX;
        let mut first = -1;
        let mut next_pos = u32::MAX;
        let mut next = -1;
        let end = END_REQUESTED_LIST.get();
        let mut p = next_of(START_REQUESTED_LIST.get());
        while p != end {
            let id = id_of(p);
            let nxt = next_of(p);
            if !prio || NUM_PRIORITY_REQUESTS.get() == 0 || field::<u8, _>(p, 6).read() & 0x10 != 0 {
                let mut skip = false;
                if id < 20000 {
                    let t = mi_txd(mi(id)) + RESOURCE_ID_TXD;
                    let ts = load_state(t);
                    if ts != 1 && ts != 3 {
                        call_request_model(t, flags(id) as i32);
                        skip = true;
                    } else {
                        let a = mi_get_anim_file_index(mi(id));
                        if a != -1 {
                            let ai = a + RESOURCE_ID_IFP;
                            let s = load_state(ai);
                            if s != 1 && s != 3 {
                                call_request_model(ai, 8);
                                skip = true;
                            }
                        }
                    }
                } else if id < RESOURCE_ID_COL {
                    let def = txd_def(id - RESOURCE_ID_TXD);
                    let parent = field::<i16, u8>(def, 6).read() as i32;
                    if parent != -1 {
                        let pi = parent + RESOURCE_ID_TXD;
                        let s = load_state(pi);
                        if s != 1 && s != 3 {
                            call_request_model(pi, 8);
                            skip = true;
                        }
                    }
                } else if (RESOURCE_ID_IFP..RESOURCE_ID_RRR).contains(&id)
                    && (g::<u8>(CUTSCENE_PROCESSING).read() != 0 || load_state(7) != 1)
                {
                    skip = true;
                }
                if !skip {
                    let cp = cd_posn(id);
                    if cp < first_pos {
                        first_pos = cp;
                        first = id;
                    }
                    if cp < next_pos && cp >= pos {
                        next_pos = cp;
                        next = id;
                    }
                }
            }
            p = nxt;
        }
        if next != -1 {
            return next;
        }
        if first != -1 {
            return first;
        }
        if NUM_PRIORITY_REQUESTS.get() == 0 {
            return -1;
        }
        NUM_PRIORITY_REQUESTS.set(0);
        prio = false;
    }
}

/// `CStreaming::RequestFilesInChannel(int32 ch)` (0x409050): every id of the channel: RemoveModel then
/// RequestModel with its (pre-removal) flags.
#[hook(addr = 0x409050, conv = cdecl, name = "CStreaming::RequestFilesInChannel", expect = "53 55 56 57 8B 7C 24 14")]
pub fn request_files_in_channel(ch: i32) {
    for i in 0..16 {
        let id = ch_id(ch, i).read();
        if id != -1 {
            let f = flags(id);
            call_remove_model(id);
            call_request_model(id, f as i32);
        }
    }
}

/// `CStreaming::RequestSpecialModel(int32 id, const char* name, int32 flags)` (0x409D10): same key and
/// a CD entry → just RequestModel. Otherwise (refs > 0) script peds (not the player, deletable even in
/// a vehicle) and deletable objects of the model are removed (backwards, while refs > 0); the model's
/// key is set to the new name; the TXD of the last model info with the old key (if loaded) is kept
/// referenced around RemoveModel; the entry is looked up in the extra-objects directory, the TXD set
/// (`"generic"` when the name has no TXD slot), CD position/size/image from the entry (`nextOnCd =
/// -1`), RequestModel.
#[hook(addr = 0x409D10, conv = cdecl, name = "CStreaming::RequestSpecialModel", expect = "83 EC 08 53 8B 5C 24 10")]
pub fn request_special_model(id: i32, name: GPtr<u8>, flags_arg: i32) {
    let m = mi(id);
    let old_key = field::<u32, u8>(m, 4).read();
    let mut pos: u32 = 0;
    let mut size: u32 = 0;
    if get_uppercase_key(name) == old_key && cd_size(id) != 0 {
        call_request_model(id, flags_arg);
        return;
    }
    if mi_refs(m) > 0 {
        pool_backwards(ped_pool(), 0x7C4, |e| {
            if mi_refs(m) >= 1
                && e_model(e) == id
                && ped_is_player(e) == 0
                && ped_can_be_deleted_even_in_vehicle(e) != 0
            {
                the_scripts_remove_this_ped(e);
            }
        });
        pool_backwards(object_pool(), 0x19C, |e| {
            if mi_refs(m) >= 1 && e_model(e) == id && object_can_be_deleted(e) != 0 {
                world_remove(e);
                world_remove_references_to_deleted_object(e);
                entity_scalar_deleting_destructor(e, 1);
            }
        });
    }
    let old_key = field::<u32, u8>(m, 4).read();
    field::<u32, u8>(m, 4).write(get_uppercase_key(name));
    let mut last = GPtr::<u8>::NULL;
    for i in 0..20000 {
        let o = mi(i);
        if !o.is_null() && field::<u32, u8>(o, 4).read() == old_key {
            last = o;
        }
    }
    let mut kept = false;
    if !last.is_null() {
        let t = mi_txd(last);
        if t != -1 && !field::<GPtr<u8>, u8>(txd_def(t), 0).read().is_null() {
            txd_add_ref(t);
            call_remove_model(id);
            txd_remove_ref_without_delete(t);
            kept = true;
        }
    }
    if !kept {
        call_remove_model(id);
    }
    directory_find_item(
        EXTRA_OBJECTS_DIR.get().cast(),
        name,
        GPtr::from_raw(&raw mut pos),
        GPtr::from_raw(&raw mut size),
    );
    mi_clear_tex_dictionary(m);
    let txd_name = if txd_find_txd_slot(name) == -1 { GPtr::from_addr(0x858A34) } else { name };
    mi_set_tex_dictionary(m, txd_name);
    let p = info(id);
    field::<u32, _>(p, 0x8).write(pos & 0xFF_FFFF);
    field::<u32, _>(p, 0xC).write(size);
    field::<u8, _>(p, 0x7).write((pos >> 24) as u8);
    field::<i16, _>(p, 0x4).write(-1);
    call_request_model(id, flags_arg);
}

/// `CStreaming::ConvertBufferToObject(uint8* buf, int32 id)` (0x40C6B0, AL; gr-deviations
/// `Streaming.cpp:436/437/447/478/618`; the TXD slot is read without a null check). See the module
/// docs and the body for the per-type paths.
#[hook(addr = 0x40C6B0, conv = cdecl, name = "CStreaming::ConvertBufferToObject", expect = "83 EC 20 8B 4C 24 24")]
pub fn convert_buffer_to_object(buf: GPtr<u8>, id: i32) -> bool {
    let size = cd_size(id) << 11;
    let mut mem = RwMemory { start: buf, length: size };
    let mut s = open_memory_stream(&mut mem);
    let fail = |s: GPtr<u8>, mem: &mut RwMemory, rerequest: bool| {
        call_remove_model(id);
        if rerequest {
            call_request_model(id, flags(id) as i32);
        }
        close_stream(s, mem);
        false
    };
    let mut m = GPtr::<u8>::NULL;
    if id < 20000 {
        m = mi(id);
        let anim = mi_get_anim_file_index(m);
        let txd = mi_txd(m);
        let def = txd_def(txd);
        if field::<GPtr<u8>, u8>(def, 0).read().is_null()
            || (anim != -1 && g::<u8>(ANIM_BLOCK_LOADED + 0x20 * anim as usize).read() == 0)
        {
            return fail(s, &mut mem, true);
        }
        txd_add_ref(txd);
        if anim != -1 {
            anim_add_anim_block_ref(anim);
        }
        txd_set_current_txd(mi_txd(m));
        let mut ok;
        if mi_get_rw_model_type(m) == 1 {
            let mut hdr = [0u32; 5];
            let mut dict = GPtr::<u8>::NULL;
            rw_stream_read_chunk_header_info(s, GPtr::from_raw(hdr.as_mut_ptr()).cast());
            if hdr[0] == 0x2B {
                dict = rt_dict_schema_stream_read_dict(g(UV_ANIM_DICT_SCHEMA), s);
                rt_dict_schema_set_current_dict(g(UV_ANIM_DICT_SCHEMA), dict);
            }
            close_stream(s, &mut mem);
            s = open_memory_stream(&mut mem);
            ok = fl_load_atomic_file(s, id) != 0;
            if !dict.is_null() {
                rt_dict_destroy(dict);
            }
        } else {
            mi_get_model_type(m);
            ok = fl_load_clump_file(s, id) != 0;
        }
        if load_state(id) != 4 {
            txd_remove_ref_without_delete(mi_txd(m));
            if anim != -1 {
                anim_remove_anim_block_ref_without_delete(anim);
            }
            if !ok {
                return fail(s, &mut mem, true);
            }
            if mi_get_model_type(m) == 6 {
                ok = call_add_to_loaded_vehicles_list(id);
            }
        }
        if !ok {
            return fail(s, &mut mem, true);
        }
    } else if id < RESOURCE_ID_COL {
        let slot = id - RESOURCE_ID_TXD;
        let parent = field::<i16, u8>(txd_def(slot), 6).read() as i32;
        if parent != -1 && txd_get_txd(parent).is_null() {
            return fail(s, &mut mem, true);
        }
        if flags(id) & 0xE == 0 && !call_are_textures_used_by_requested_models(slot) {
            return fail(s, &mut mem, false);
        }
        let ok = if !LOADING_BIG_MODEL.get().get() {
            txd_load_txd_stream(slot, s) != 0
        } else {
            if txd_start_load_txd(slot, s) == 0 {
                return fail(s, &mut mem, true);
            }
            set_load_state(id, 4);
            true
        };
        if !ok {
            return fail(s, &mut mem, true);
        }
    } else if id < RESOURCE_ID_IPL {
        if !col_load_col(id - RESOURCE_ID_COL, buf, size as i32) {
            return fail(s, &mut mem, true);
        }
    } else if id < RESOURCE_ID_DAT {
        if ipl_load_ipl(id - RESOURCE_ID_IPL, buf, size as i32) == 0 {
            return fail(s, &mut mem, true);
        }
    } else if id < RESOURCE_ID_IFP {
        path_find_load_path_find_data(g(0x96F050), s, id - RESOURCE_ID_DAT);
    } else if id < RESOURCE_ID_RRR {
        if flags(id) & 0xE == 0 && !call_are_anims_used_by_requested_models(id - RESOURCE_ID_IFP) {
            return fail(s, &mut mem, false);
        }
        anim_load_anim_file(s, 1, GPtr::NULL);
        anim_create_anim_assoc_groups();
    } else if id < RESOURCE_ID_SCM {
        vehicle_recording_load(s, id - RESOURCE_ID_RRR, size as i32);
    } else {
        streamed_scripts_load_streamed_script(g(0xA47B60), s, id - RESOURCE_ID_SCM);
    }
    close_stream(s, &mut mem);
    let add = if id < 20000 {
        let t = mi_get_model_type(m);
        if t == 6 || mi_get_model_type(m) == 7 {
            false
        } else {
            let a = mi_as_atomic(m);
            if !a.is_null() {
                a.byte_add(0xC).write(if flags(id) & 0x24 != 0 { 0xFF } else { 0 });
            }
            flags(id) & 6 == 0
        }
    } else {
        let other = id >= RESOURCE_ID_COL && !((RESOURCE_ID_IFP..RESOURCE_ID_RRR).contains(&id) || id >= RESOURCE_ID_SCM);
        !other && flags(id) & 6 == 0
    };
    if add {
        call_add_to_list(info(id), START_LOADED_LIST.get());
    }
    if load_state(id) != 4 {
        set_load_state(id, 1);
        MEMORY_USED.set(MEMORY_USED.get().wrapping_add(cd_size(id) << 11));
    }
    true
}
/// `RpUVAnimDictSchema` (0x8DED50)
const UV_ANIM_DICT_SCHEMA: usize = 0x8DED50;

/// `CStreaming::RequestModelStream(int32 ch)` (0x40CBA0). The first file at/after the last CdStream
/// position (priority first); TXDs not used by requested models and anims not used are removed and
/// the search continues after them; a file larger than the streaming buffer starts a big-model load
/// (only on channel 0 with channel 1 idle). Then up to 16 consecutive files (`m_NextIndexOnCd`) that
/// are requested (2), priority-matching, fit in the buffer (except the first), and whose TXD/anim
/// are loaded or being read; at most one ped (7) and one vehicle (6) / one file > 200 sectors per
/// read. Each is marked reading (3), unlinked, the counters adjusted; the channel's remaining slots
/// get -1; `CdStreamRead(ch, buffer[ch], pos, sectors)`; channel status 1, level 0, position/size
/// kept, tries 0; `ms_bReadLayerForceFully` cleared. The position/size locals keep their previous
/// values when an entry has no CD size (uninitialised in the original before the first; the port
/// starts from 0 — L2 never sees it, every requested entry has a size).
#[hook(addr = 0x40CBA0, conv = cdecl, name = "CStreaming::RequestModelStream", expect = "83 EC 58 55 33 ED")]
pub fn request_model_stream(ch: i32) {
    let mut total: i32 = 0;
    let mut pos: u32 = 0;
    let mut size: u32 = 0;
    let mut has_ped = false;
    let mut has_veh = false;
    let mut id = call_get_next_file_on_cd(cd_stream_get_last_posn(), 1);
    if id == -1 {
        return;
    }
    while flags(id) & 0xE == 0
        && (((RESOURCE_ID_TXD..RESOURCE_ID_COL).contains(&id) && !call_are_textures_used_by_requested_models(id - RESOURCE_ID_TXD))
            || ((RESOURCE_ID_IFP..RESOURCE_ID_RRR).contains(&id) && !call_are_anims_used_by_requested_models(id - RESOURCE_ID_IFP)))
    {
        call_remove_model(id);
        if cd_size(id) != 0 {
            pos = cd_posn(id);
            size = cd_size(id);
        }
        id = call_get_next_file_on_cd(pos.wrapping_add(size), 1);
        if id == -1 {
            return;
        }
    }
    if cd_size(id) != 0 {
        pos = cd_posn(id);
        size = cd_size(id);
    }
    if STREAMING_BUFFER_SIZE.get() < size {
        if ch == 1 || channel(1).read().load_status != 0 {
            return;
        }
        LOADING_BIG_MODEL.set(GBool::TRUE);
    }
    let mut n = 0;
    while n < 16 {
        if id == -1 || load_state(id) != 2 {
            break;
        }
        if cd_size(id) != 0 {
            size = cd_size(id);
        }
        if NUM_PRIORITY_REQUESTS.get() != 0 && flags(id) & 0x10 == 0 {
            break;
        }
        let ok = if id < 20000 {
            let m = mi(id);
            (!has_ped || mi_get_model_type(m) != 7)
                && (!has_veh || mi_get_model_type(m) != 6)
                && matches!(load_state(mi_txd(m) + RESOURCE_ID_TXD), 1 | 3)
                && {
                    let a = mi_get_anim_file_index(m);
                    a == -1 || matches!(load_state(a + RESOURCE_ID_IFP), 1 | 3)
                }
        } else if !(RESOURCE_ID_IFP..RESOURCE_ID_RRR).contains(&id) {
            !has_veh || size < 0xC9
        } else {
            g::<u8>(CUTSCENE_PROCESSING).read() == 0 && load_state(7) == 1
        };
        if !ok {
            break;
        }
        ch_offset(ch, n).write(total);
        ch_id(ch, n).write(id);
        total = total.wrapping_add(size as i32);
        if (STREAMING_BUFFER_SIZE.get() as i32) < total && n > 0 {
            total = total.wrapping_sub(size as i32);
            break;
        }
        if id < 20000 {
            if mi_get_model_type(mi(id)) == 7 {
                has_ped = true;
            }
            if mi_get_model_type(mi(id)) == 6 {
                has_veh = true;
            }
        } else if size > 200 {
            has_veh = true;
        }
        set_load_state(id, 3);
        call_remove_from_list(info(id));
        NUM_MODELS_REQUESTED.set(NUM_MODELS_REQUESTED.get() - 1);
        if flags(id) & 0x10 != 0 {
            NUM_PRIORITY_REQUESTS.set(NUM_PRIORITY_REQUESTS.get().wrapping_sub(1));
            set_flags(id, flags(id) & 0xEF);
        }
        id = field::<i16, _>(info(id), 4).read() as i32;
        n += 1;
    }
    for i in n..16 {
        ch_id(ch, i).write(-1);
    }
    let buf = STREAMING_BUFFER.ptr().cast::<GPtr<u8>>().wrapping_index(ch).read();
    cd_stream_read(ch, buf, pos, total);
    ch_field::<i32>(ch, CH_STATUS).write(1);
    ch_field::<i32>(ch, CH_LEVEL).write(0);
    ch_field::<i32>(ch, CH_SECTORS).write(total);
    ch_field::<u32>(ch, CH_POS).write(pos);
    ch_field::<i32>(ch, CH_TRIES).write(0);
    if READ_LAYER_FORCE_FULLY.get().get() {
        READ_LAYER_FORCE_FULLY.set(GBool::FALSE);
    }
}

/// `CStreaming::RemoveAllUnusedModels()` (0x40CF80 → 0x15645F0): RemoveLoadedVehicle ×50, then every
/// loaded DFF 1000..19999 without refs: RemoveModel and its flags byte cleared.
#[hook(addr = 0x40CF80, conv = cdecl, name = "CStreaming::RemoveAllUnusedModels", expect = "E9 6B 76 15 01")]
pub fn remove_all_unused_models() {
    for _ in 0..50 {
        call_remove_loaded_vehicle();
    }
    for id in 1000..20000 {
        if load_state(id) == 1 && mi_refs(mi(id)) == 0 {
            call_remove_model(id);
            set_flags(id, 0);
        }
    }
}

fn vehicles_loaded_count() -> i32 {
    car_group_count_members(VEHICLES_LOADED.ptr().cast())
}

/// `CStreaming::RemoveLeastUsedModel(uint32 flags)` (0x40CFD0, AL): the loaded list from its end
/// backwards: an entry without `flags` that is an unreferenced DFF, a TXD without refs not used by
/// requests, an anim block without refs not used by requests, or an unloaded streamed script slot is
/// removed → true. Otherwise (camera height above ground `camZ - CalculateGroundHeight(0)`):
/// above 50 → zone peds (more than 4) or vehicles (more than 4); then (unless loading a scene) least
/// used entity RW objects (off screen), zone peds, vehicles (7+, or 4+ in an interior); then on
/// screen. The exact order is the original's short-circuit chain.
#[hook(addr = 0x40CFD0, conv = cdecl, name = "CStreaming::RemoveLeastUsedModel", expect = "A1 5C 4C 8E 00")]
pub fn remove_least_used_model(flags_arg: u32) -> bool {
    let start = START_LOADED_LIST.get();
    let mut p = prev_of(END_LOADED_LIST.get());
    while p != start {
        let id = id_of(p);
        if field::<u8, _>(p, 6).read() & (flags_arg as u8) == 0 {
            if id < 20000 {
                if mi_refs(mi(id)) == 0 {
                    call_remove_model(id);
                    return true;
                }
            } else if id < RESOURCE_ID_COL {
                if txd_get_num_refs(id - RESOURCE_ID_TXD) == 0
                    && !call_are_textures_used_by_requested_models(id - RESOURCE_ID_TXD)
                {
                    call_remove_model(id);
                    return true;
                }
            } else if (RESOURCE_ID_IFP..RESOURCE_ID_RRR).contains(&id) {
                if anim_get_num_refs_to_anim_block(id - RESOURCE_ID_IFP) == 0
                    && !call_are_anims_used_by_requested_models(id - RESOURCE_ID_IFP)
                {
                    call_remove_model(id);
                    return true;
                }
            } else if id > RESOURCE_ID_SCM - 1 && g::<u8>(STREAMED_SCRIPT_LOADED + 0x20 * (id - RESOURCE_ID_SCM) as usize).read() == 0 {
                call_remove_model(id);
                return true;
            }
        }
        p = prev_of(p);
    }
    let cam_z = camera_pos().wrapping_index(2).read();
    let ground = camera_calculate_ground_height(g(CAMERA), 0);
    let high = !x(cam_z).fsub_m(ground).fcom(x(k(F_50))).test_any(0x41);
    let peds_zone = || NUM_PEDS_LOADED.get() > 4 && call_remove_loaded_zone_model();
    let vehicles = |lim_world: i32| {
        let c = vehicles_loaded_count();
        let over = !(c < lim_world && (curr_area() == 0 || vehicles_loaded_count() < 5));
        over && call_remove_loaded_vehicle()
    };
    if high && (peds_zone() || (vehicles_loaded_count() > 4 && call_remove_loaded_vehicle())) {
        return true;
    }
    if !LOADING_SCENE.get().get()
        && (call_delete_least_used_entity_rw_object(0, flags_arg) || peds_zone() || vehicles(8))
    {
        return true;
    }
    call_delete_least_used_entity_rw_object(1, flags_arg) || vehicles(8) || peds_zone()
}

/// `CStreaming::RemoveReferencedTxds(int32 memory)` (0x40D2F0, AL): the loaded list from its end
/// backwards: TXDs not LOADING_SCENE-flagged with no refs are removed, until `memoryUsed < memory`
/// (signed) → true.
#[hook(addr = 0x40D2F0, conv = cdecl, name = "CStreaming::RemoveReferencedTxds", expect = "A1 5C 4C 8E 00")]
pub fn remove_referenced_txds(memory: i32) -> bool {
    let start = START_LOADED_LIST.get();
    let mut p = prev_of(END_LOADED_LIST.get());
    while p != start {
        let id = id_of(p);
        let prev = prev_of(p);
        if (RESOURCE_ID_TXD..RESOURCE_ID_COL).contains(&id)
            && field::<u8, _>(p, 6).read() & 0x20 == 0
            && txd_get_num_refs(id - RESOURCE_ID_TXD) == 0
        {
            call_remove_model(id);
            if (MEMORY_USED.get() as i32) < memory {
                return true;
            }
        }
        p = prev;
    }
    false
}

/// `CStreaming::LoadInitialPeds()` (0x40D3D0 → 0x15638D0): RequestModel(7, 2), StreamCopModels(1).
#[hook(addr = 0x40D3D0, conv = cdecl, name = "CStreaming::LoadInitialPeds", expect = "E9 FB 64 15 01")]
pub fn load_initial_peds() {
    call_request_model(7, 2);
    call_stream_cop_models(1);
}

/// `CStreaming::MakeSpaceFor(int32 size)` (0x40E120; the memory limit is read in the chunk 0x4037EB
/// through `rol [0x1560758], [0x156F87C]` = 0x8A5A80): while `memoryUsed >= available - size`
/// (unsigned) remove the least used model (0x20); when none: `DeleteRwObjectsBehindCamera(available
/// - size)` (tail jump).
#[hook(addr = 0x40E120, conv = cdecl, name = "CStreaming::MakeSpaceFor", expect = "E9 C6 56 FF FF 8B")]
pub fn make_space_for(size: i32) {
    while MEMORY_USED.get() >= MEMORY_AVAILABLE.get().wrapping_sub(size as u32) {
        if !call_remove_least_used_model(0x20) {
            call_delete_rw_objects_behind_camera(MEMORY_AVAILABLE.get().wrapping_sub(size as u32) as i32);
            return;
        }
    }
}

/// `CStreaming::ProcessLoadingChannel(int32 ch)` (0x40E170, AL). A failed CdStream status
/// (not 0/0xFF/0xFA) → status 3, and when no channel error is recorded yet: error = ch,
/// `RetryLoadFile(ch)`, true; 0xFF/0xFA → false. Done (0): big-file state (2) →
/// `FinishLoadingLargeFile`; otherwise each id: a vehicle beyond the desired count that cannot free
/// a loaded vehicle and is not required is dropped (TXD removed when unreferenced, else re-requested);
/// the rest make space (except IPLs) and are converted; a big file switches the channel to state 2.
/// After a big load the second channel's ids are cleared.
#[hook(addr = 0x40E170, conv = cdecl, name = "CStreaming::ProcessLoadingChannel", expect = "83 EC 08 56 8B 74 24 10")]
pub fn process_loading_channel(ch: i32) -> bool {
    let st = cd_stream_get_status(ch);
    if st != 0 {
        if st != 0xFF && st != 0xFA {
            ch_field::<i32>(ch, CH_CD_STATUS).write(st);
            let none = CHANNEL_ERROR.get() == -1;
            ch_field::<i32>(ch, CH_STATUS).write(3);
            if none {
                CHANNEL_ERROR.set(ch);
                call_retry_load_file(ch);
                return true;
            }
        }
        return false;
    }
    let prev = ch_field::<i32>(ch, CH_STATUS).read();
    ch_field::<i32>(ch, CH_STATUS).write(0);
    let buf = STREAMING_BUFFER.ptr().cast::<GPtr<u8>>().wrapping_index(ch).read();
    if prev == 2 {
        let off = ch_offset(ch, 0).read();
        call_finish_loading_large_file(buf.byte_add((off.wrapping_mul(0x800)) as isize), ch_id(ch, 0).read());
        ch_id(ch, 0).write(-1);
    } else {
        let mut i = 0;
        while i < 16 {
            let id = ch_id(ch, i).read();
            if id != -1 {
                let size = cd_size(id);
                let drop = id < 20000
                    && mi_get_model_type(mi(id)) == 6
                    && DESIRED_NUM_VEHICLES_LOADED.get() <= vehicles_loaded_count()
                    && !call_remove_loaded_vehicle()
                    && flags(id) & 6 == 0;
                if drop {
                    let t = mi_txd(mi(id));
                    call_remove_model(id);
                    if flags(id) & 6 == 0 {
                        if txd_get_num_refs(t) == 0 {
                            call_remove_txd_model(t);
                        }
                    } else {
                        call_request_model(id, flags(id) as i32);
                    }
                } else {
                    if !(RESOURCE_ID_IPL..RESOURCE_ID_DAT).contains(&id) {
                        call_make_space_for((size << 11) as i32);
                    }
                    let off = ch_offset(ch, i).read();
                    call_convert_buffer_to_object(buf.byte_add((off.wrapping_mul(0x800)) as isize), id);
                    if load_state(id) == 4 {
                        ch_field::<i32>(ch, CH_STATUS).write(2);
                        ch_offset(ch, 0).write(ch_offset(ch, i).read());
                        ch_id(ch, 0).write(id);
                        if i == 0 {
                            i += 1;
                            continue;
                        }
                    }
                    ch_id(ch, i).write(-1);
                }
            }
            i += 1;
        }
    }
    if LOADING_BIG_MODEL.get().get() && ch_field::<i32>(ch, CH_STATUS).read() != 2 {
        LOADING_BIG_MODEL.set(GBool::FALSE);
        for i in 0..16 {
            ch_id(1, i).write(-1);
        }
    }
    true
}

/// `CStreaming::LoadRequestedModels()` (0x40E3A0 → 0x15670A0): alternating channels (channel 0 while a
/// big model loads); the other channel is fed through a computed call (`[0x1560538] - [0x646FE2]` =
/// 0x40CBA0) when idle.
#[hook(addr = 0x40E3A0, conv = cdecl, name = "CStreaming::LoadRequestedModels", expect = "E9 FB 8C 15 01")]
pub fn load_requested_models() {
    if LOADING_BIG_MODEL.get().get() {
        CURRENT_CHANNEL.set(0);
    }
    let ch = CURRENT_CHANNEL.get();
    if ch_field::<i32>(ch, CH_STATUS).read() != 0 {
        call_process_loading_channel(ch);
    }
    if !LOADING_BIG_MODEL.get().get() {
        let ch = CURRENT_CHANNEL.get();
        if ch_field::<i32>(1 - ch, CH_STATUS).read() == 0 {
            call_request_model_stream(1 - ch);
        }
        let ch = CURRENT_CHANNEL.get();
        if ch_field::<i32>(ch, CH_STATUS).read() == 0 && !LOADING_BIG_MODEL.get().get() {
            call_request_model_stream(ch);
        }
    }
    let ch = CURRENT_CHANNEL.get();
    if ch_field::<i32>(ch, CH_STATUS).read() != 2 {
        CURRENT_CHANNEL.set(1 - ch);
    }
}

/// `CStreaming::FlushChannels()` (0x40E460).
#[hook(addr = 0x40E460, conv = cdecl, name = "CStreaming::FlushChannels", expect = "83 3D 78 4B 8E 00 02")]
pub fn flush_channels() {
    if ch_field::<i32>(1, CH_STATUS).read() == 2 {
        call_process_loading_channel(1);
    }
    if ch_field::<i32>(0, CH_STATUS).read() == 1 {
        cd_stream_sync(0);
        ch_field::<i32>(0, CH_LEVEL).write(100);
        call_process_loading_channel(0);
    }
    if ch_field::<i32>(0, CH_STATUS).read() == 2 {
        call_process_loading_channel(0);
    }
    if ch_field::<i32>(1, CH_STATUS).read() == 1 {
        cd_stream_sync(1);
        ch_field::<i32>(1, CH_LEVEL).write(100);
        call_process_loading_channel(1);
    }
    if ch_field::<i32>(1, CH_STATUS).read() == 2 {
        call_process_loading_channel(1);
    }
}

/// `CStreaming::FlushRequestList()` (0x40E4E0): every requested entry removed (front to back), then
/// FlushChannels.
#[hook(addr = 0x40E4E0, conv = cdecl, name = "CStreaming::FlushRequestList", expect = "A1 58 4C 8E 00")]
pub fn flush_request_list() {
    let end = END_REQUESTED_LIST.get();
    let mut p = next_of(START_REQUESTED_LIST.get());
    while p != end {
        let nxt = next_of(p);
        call_remove_model(id_of(p));
        p = nxt;
    }
    call_flush_channels();
}

/// `CStreaming::ReInit()` (0x40E560 → 0x156D000): streamed scripts re-initialised, request list
/// flushed, RW objects deleted, unused models removed; mission-required models released; scripts
/// 0..81 removed; flags reset; special models 290..299, 384..393, 300..319 removed and renamed
/// `""` (key of the string at 0x858B54).
#[hook(addr = 0x40E560, conv = cdecl, name = "CStreaming::ReInit", expect = "E9 9B EA 15 01")]
pub fn re_init() {
    streamed_scripts_re_initialise(g(0xA47B60));
    call_flush_request_list();
    call_delete_all_rw_objects();
    call_remove_all_unused_models();
    for id in 0..crate::shared::RESOURCE_ID_TOTAL as i32 {
        if flags(id) & 4 != 0 {
            call_set_mission_doesnt_require_model(id);
        }
    }
    for i in 0..0x52 {
        call_remove_model(i + RESOURCE_ID_SCM);
    }
    DISABLE_STREAMING.set(GBool::FALSE);
    CURRENT_ZONE_TYPE.set(-1);
    BOATS_NEEDED.set(GBool::FALSE);
    COP_BIKE_LOADED.set(GBool::FALSE);
    DISABLE_COP_BIKES.set(GBool::FALSE);
    TIME_PASSED_SINCE_LAST_COP_BIKE.set(0);
    for range in [290..300, 0x180..0x18A, 300..0x140] {
        for id in range {
            call_remove_model(id);
            let m = mi(id);
            field::<u32, u8>(m, 4).write(get_uppercase_key(g(0x858B54)));
        }
    }
}

/// `CStreaming::LoadAllRequestedModels(bool onlyPriority)` (0x40EA10; entry `jmp 0x40A45E` reads
/// `m_bLoadingAllRequestedModels`): re-entrancy guard; FlushChannels; up to `max(10, 2 *
/// requested)` rounds alternating channels (0 while a big model loads): sync and finish the active
/// channel, stop when only priority requests are wanted and none remain, feed the other channel and
/// this one; FlushChannels.
#[hook(addr = 0x40EA10, conv = cdecl, name = "CStreaming::LoadAllRequestedModels", expect = "E9 49 BA FF FF 56")]
pub fn load_all_requested_models(only_priority: u8) {
    if LOADING_ALL_REQUESTED_MODELS.get().get() {
        return;
    }
    LOADING_ALL_REQUESTED_MODELS.set(GBool::TRUE);
    call_flush_channels();
    let req = NUM_MODELS_REQUESTED.get().wrapping_mul(2);
    let mut rounds = if req < 10 { 10 } else { req };
    let mut ch = 0;
    loop {
        let empty = prev_of(END_REQUESTED_LIST.get()) == START_REQUESTED_LIST.get()
            && ch_field::<i32>(0, CH_STATUS).read() == 0
            && ch_field::<i32>(1, CH_STATUS).read() == 0;
        if empty || rounds <= 0 {
            break;
        }
        if LOADING_BIG_MODEL.get().get() {
            ch = 0;
        }
        if ch_field::<i32>(ch, CH_STATUS).read() != 0 {
            cd_stream_sync(ch);
            ch_field::<i32>(ch, CH_LEVEL).write(100);
        }
        if ch_field::<i32>(ch, CH_STATUS).read() == 1 {
            call_process_loading_channel(ch);
            if ch_field::<i32>(ch, CH_STATUS).read() == 2 {
                call_process_loading_channel(ch);
            }
        }
        if only_priority != 0 && NUM_PRIORITY_REQUESTS.get() == 0 {
            break;
        }
        if !LOADING_BIG_MODEL.get().get() {
            if ch_field::<i32>(1 - ch, CH_STATUS).read() == 0 {
                call_request_model_stream(1 - ch);
            }
            if ch_field::<i32>(ch, CH_STATUS).read() == 0 && !LOADING_BIG_MODEL.get().get() {
                call_request_model_stream(ch);
            }
        }
        if ch_field::<i32>(0, CH_STATUS).read() == 0 && ch_field::<i32>(1, CH_STATUS).read() == 0 {
            break;
        }
        ch = 1 - ch;
        rounds -= 1;
    }
    call_flush_channels();
    LOADING_ALL_REQUESTED_MODELS.set(GBool::FALSE);
}

/// `CStreaming::LoadScene(const CVector&)` (0x40EB70): `GetLevelFromPosition(pos)`; the player's
/// position (when there is a player) kept; `ms_bLoadingScene = 1`; requested entries without 0x3E
/// purged (end → start); loading priority cleared; all RW objects deleted; big buildings requested;
/// IPLs loaded (`CIplStore::LoadIpls(pos, true)`); load all; models around the position requested
/// (0x20); radar sections streamed; post-IPL call (0x565690); in the world: zone vehicles (×5, when
/// the vehicles-in-load-scene flag is set and the zone differs from the player's) and cop models for
/// the level; load all; instance loaded models; LOADING_SCENE flag cleared everywhere;
/// `ms_bLoadingScene = 0`; with a player `CPopulation::ManageAllPopulation()`.
#[hook(addr = 0x40EB70, conv = cdecl, name = "CStreaming::LoadScene", expect = "83 EC 1C 55 8B 6C 24 24")]
pub fn load_scene(pos: GPtr<f32>) {
    let lvl = zones_get_level_from_position(pos);
    let mut player_pos = [0f32; 3];
    LOADING_SCENE.set(GBool::TRUE);
    if !g::<GPtr<u8>>(PLAYER0_PED).read().is_null() {
        let mut tmp = [0f32; 3];
        let p = find_player_coors(GPtr::from_raw(tmp.as_mut_ptr()), -1);
        player_pos = [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()];
    }
    purge_requested(0x3E);
    g::<u8>(RENDERER_LOADING_PRIORITY).write(0);
    call_delete_all_rw_objects();
    call_request_big_buildings(pos);
    ipl_load_ipls(pos.read(), pos.wrapping_index(1).read(), pos.wrapping_index(2).read(), 1);
    call_load_all_requested_models(0);
    call_add_models_to_request_list(pos, 0x20);
    radar_stream_radar_sections(pos);
    path_find_load_scene_for_path_nodes(g(0x96F050), pos.read(), pos.wrapping_index(1).read(), pos.wrapping_index(2).read());
    if curr_area() == 0 {
        if LOAD_VEHICLES_IN_LOAD_SCENE.get().get() {
            let a = zones_get_zone_info(pos, GPtr::NULL);
            let b = zones_get_zone_info(GPtr::from_raw(player_pos.as_mut_ptr()), GPtr::NULL);
            if a != b {
                for _ in 0..5 {
                    if !g::<GPtr<u8>>(POP_CURR_ZONE_INFO).read().is_null() {
                        zones_get_zone_info(pos, GPtr::NULL);
                        let grp = popcycle_pick_a_random_group_of_other_peds();
                        let m = car_ctrl_choose_car_model_to_load(grp);
                        if m >= 0 {
                            call_request_model(m, 8);
                        }
                    }
                }
            }
        }
        call_stream_cop_models(lvl);
    }
    call_load_all_requested_models(0);
    call_instance_loaded_models(pos);
    for id in 0..crate::shared::RESOURCE_ID_TOTAL as i32 {
        set_flags(id, flags(id) & 0xDF);
    }
    LOADING_SCENE.set(GBool::FALSE);
    if !g::<GPtr<u8>>(PLAYER0_PED).read().is_null() {
        population_manage_all_population();
    }
}

/// `CStreaming::LoadSceneCollision(const CVector&)` (0x40ED80): `CColStore::LoadCollision(pos, true)`,
/// `CIplStore::LoadIpls(pos, true)`, `LoadAllRequestedModels(false)`.
#[hook(addr = 0x40ED80, conv = cdecl, name = "CStreaming::LoadSceneCollision", expect = "56 8B 74 24 08")]
pub fn load_scene_collision(pos: GPtr<f32>) {
    col_load_collision(pos.read(), pos.wrapping_index(1).read(), pos.wrapping_index(2).read(), 1);
    ipl_load_ipls(pos.read(), pos.wrapping_index(1).read(), pos.wrapping_index(2).read(), 1);
    call_load_all_requested_models(0);
}

/// `CStreaming::Update()` (0x40E670). See the module docs for the gating; heights: `h = f32(camZ -
/// CalculateGroundHeight(0))`, `h >= 50` (or NaN) → LOD requests (world only), else models around the
/// camera when 0xB745C1 is set; every 128 frames (`(frame & 0x7F) == 0x6A`) boats needed when the
/// camera is below 500 and `ThePaths.IsWaterNodeNearby(camPos, 80)`; vehicles/peds/zone models in the
/// world below 50; LoadRequestedModels; collision and IPLs around the player (or the focus entity at
/// 0xB7CE48, with the player position registered as needed); request-list purge when enabled.
#[hook(addr = 0x40E670, conv = cdecl, name = "CStreaming::Update", expect = "A1 B8 4C 8E 00")]
pub fn update() {
    g::<i32>(0xB729BC).write(NUM_MODELS_REQUESTED.get());
    if g::<u8>(TIMER_USER_PAUSE).read() != 0 || g::<u8>(TIMER_CODE_PAUSE).read() != 0 {
        return;
    }
    let ground = camera_calculate_ground_height(g(CAMERA), 0);
    let h = x(camera_pos().wrapping_index(2).read()).fsub_m(ground).to_f32();
    if !DISABLE_STREAMING.get().get() && g::<u8>(RENDERER_LOADING_PRIORITY).read() == 0 {
        if x(h).fcom(x(k(F_50))).test_parity(0x05) {
            if curr_area() == 0 {
                call_add_lods_to_request_list(camera_pos(), 0);
            }
        } else if g::<u8>(RENDERER_FLAG_B745C1).read() != 0 {
            call_add_models_to_request_list(camera_pos(), 0);
        }
    }
    if g::<u32>(TIMER_FRAME_COUNTER).read() & 0x7F == 0x6A {
        BOATS_NEEDED.set(GBool::FALSE);
        if !x(camera_pos().wrapping_index(2).read()).fcom(x(f32::from_bits(0x43FA_0000))).test_parity(0x05) {
            let c = camera_pos();
            let r = path_find_is_water_node_nearby(
                g(0x96F050),
                c.read(),
                c.wrapping_index(1).read(),
                c.wrapping_index(2).read(),
                k(F_80),
            );
            BOATS_NEEDED.set(GBool(r));
        }
    }
    if !DISABLE_STREAMING.get().get()
        && g::<u8>(CUTSCENE_PROCESSING).read() == 0
        && curr_area() == 0
        && g::<u8>(FLAG_A43088).read() != 1
        && !x(h).fcom(x(k(F_50))).test_parity(0x05)
    {
        let mut tmp = [0f32; 3];
        let p = find_player_coors(GPtr::from_raw(tmp.as_mut_ptr()), -1);
        call_stream_vehicles_and_peds_always(p);
        if call_is_very_busy() as u8 == 0 {
            call_stream_vehicles_and_peds();
            let mut tmp2 = [0f32; 3];
            let p2 = find_player_coors(GPtr::from_raw(tmp2.as_mut_ptr()), -1);
            call_stream_zone_models(p2);
        }
    }
    call_load_requested_models();
    let mut v = [0f32; 3];
    find_player_coors(GPtr::from_raw(v.as_mut_ptr()), -1);
    let vp: GPtr<f32> = GPtr::from_raw(v.as_mut_ptr());
    let focus = g::<GPtr<u8>>(UPDATE_FOCUS_ENTITY).read();
    if focus.is_null() {
        col_load_collision(v[0], v[1], v[2], 0);
        col_ensure_collision_is_in_memory(vp);
        ipl_load_ipls(v[0], v[1], v[2], 0);
        ipl_ensure_ipls_are_in_memory(vp);
    } else {
        col_add_collision_needed_at_posn(vp);
        let p = e_pos(focus);
        col_load_collision(p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read(), 0);
        col_ensure_collision_is_in_memory(e_pos(g::<GPtr<u8>>(UPDATE_FOCUS_ENTITY).read()));
        ipl_add_ipls_needed_at_posn(vp);
        let p = e_pos(g::<GPtr<u8>>(UPDATE_FOCUS_ENTITY).read());
        ipl_load_ipls(p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read(), 0);
        ipl_ensure_ipls_are_in_memory(e_pos(g::<GPtr<u8>>(UPDATE_FOCUS_ENTITY).read()));
    }
    if ENABLE_REQUEST_LIST_PURGE.get().get() {
        purge_requested(0x1E);
    }
}

/// `CStreaming::RetryLoadFile(int32 ch)` (0x4076C0; gr-deviations `Streaming.cpp:2574`, keep-original:
/// `CPad::StopPadsShaking()` first, original error-state logic; spec CStreaming.deviations.md).
#[hook(addr = 0x4076C0, conv = cdecl, name = "CStreaming::RetryLoadFile", expect = "56 E8 AA A6 13 00")]
pub fn retry_load_file(ch: i32) {
    pad_stop_pads_shaking();
    if CHANNEL_ERROR.get() == -1 {
        loading_screen_continue();
        return;
    }
    loading_screen_pause();
    if CHANNEL_ERROR.get() == -1 {
        loading_screen_continue();
        return;
    }
    loop {
        let st = ch_field::<i32>(ch, CH_STATUS).read();
        let mut reread = false;
        match st {
            0 => reread = true,
            1 => {
                if call_process_loading_channel(ch) {
                    if ch_field::<i32>(ch, CH_STATUS).read() == 2 {
                        call_process_loading_channel(ch);
                    }
                    CHANNEL_ERROR.set(-1);
                    break;
                }
            }
            3 => {
                let t = ch_field::<i32>(ch, CH_TRIES);
                t.write(t.read().wrapping_add(1));
                let s = cd_stream_get_status(ch);
                if s == 0xFF {
                    if cd_stream_get_status(ch) != 0xFF {
                        reread = true;
                    }
                } else if s != 0xFA {
                    reread = true;
                }
            }
            _ => {}
        }
        if reread {
            let buf = STREAMING_BUFFER.ptr().cast::<GPtr<u8>>().wrapping_index(ch).read();
            cd_stream_read(
                ch,
                buf,
                ch_field::<u32>(ch, CH_POS).read(),
                ch_field::<i32>(ch, CH_SECTORS).read(),
            );
            ch_field::<i32>(ch, CH_STATUS).write(1);
            ch_field::<i32>(ch, CH_LEVEL).write(-600);
        }
        if CHANNEL_ERROR.get() == -1 {
            break;
        }
    }
    loading_screen_continue();
}

/// `CStreaming::AddToLoadedVehiclesList(int32)` (0x408000 → 0x15662F0, AL): `m_nTimesUsed (+0x50) = 0`;
/// boats → `m_LoadedBoats`, else appropriate / inappropriate (the latter's address comes from the no-CD
/// indirection 0xCB73E0; gta-reversed's `m_InAppropriateLoadedCars` 0xC0E9C8); every gang car group
/// (0..9 rows of `m_CarGroups` from 0xC0F074, not the gang groups) containing it adds it to
/// `m_LoadedGangCars[i]`; `ms_vehiclesLoaded.AddMember`; true.
#[hook(addr = 0x408000, conv = cdecl, name = "CStreaming::AddToLoadedVehiclesList", expect = "E9 EB E2 15 01")]
pub fn add_to_loaded_vehicles_list(model: i32) -> bool {
    mi(model).byte_add(0x50).write(0);
    let n = g::<i16>(0xC0ECB2).read() as i32;
    let mut grp = 0usize;
    for i in 0..n.max(0) {
        if g::<u16>(0xC0F26E).wrapping_index(i).read() as i32 == model {
            grp = LOADED_BOATS;
            break;
        }
    }
    if grp == 0 {
        grp = if call_is_car_model_needed_in_current_zone(model) {
            APPROPRIATE_LOADED_CARS
        } else {
            INAPPROPRIATE_LOADED_CARS
        };
    }
    car_group_add_member(g(grp), model);
    for gang in 0..10 {
        let cnt = car_group_count(gang);
        for i in 0..cnt.max(0) {
            if car_group_model(gang, i) == model {
                car_group_add_member(g(LOADED_GANG_CARS_GROUPS + 0x2E * gang as usize), model);
                break;
            }
        }
    }
    car_group_add_member(VEHICLES_LOADED.ptr().cast(), model);
    true
}

/// `CStreaming::RemoveCarModel(int32)` (0x4080F0 → 0x156C0A0): removed from boats, appropriate,
/// inappropriate, the 10 gang car groups and `ms_vehiclesLoaded`; then
/// `CPopulation::RemoveSpecificDriverModelsForCar`.
#[hook(addr = 0x4080F0, conv = cdecl, name = "CStreaming::RemoveCarModel", expect = "E9 AB 3F 16 01")]
pub fn remove_car_model(model: i32) {
    car_group_remove_member(g(LOADED_BOATS), model);
    car_group_remove_member(g(APPROPRIATE_LOADED_CARS), model);
    car_group_remove_member(g(INAPPROPRIATE_LOADED_CARS), model);
    for gang in 0..10 {
        car_group_remove_member(g(LOADED_GANG_CARS_GROUPS + 0x2E * gang), model);
    }
    car_group_remove_member(VEHICLES_LOADED.ptr().cast(), model);
    population_remove_specific_driver_models_for_car(model);
}

/// `CStreaming::IsCarModelNeededInCurrentZone(int32)` (0x407DD0 → 0x15704A0, AL; gr-deviations
/// `Streaming.cpp:2965`): no zone info → false; traffic cheats (0x969159 → 4, 0x96917B → 3, 0x96915E
/// → 0x1E, 0x96915F → 0x1F, 0x96915C → 0x20, 0x969176 → 0x21) answer with their group; otherwise the
/// 18 car groups with a non-zero frequency for the zone's population type, level and day/night
/// (`m_nCarGroupFreq` row at 0xC0BC78, unsigned `> 0`), then the 10 gang groups 18..27 with a non-zero
/// zone gang density (the loop index arrives through the no-CD indirection 0xCB73E0; it continues at
/// 18, the only value consistent with the `[esi + ebx - 0x12]` density read and the `< 10` bound).
#[hook(addr = 0x407DD0, conv = cdecl, name = "CStreaming::IsCarModelNeededInCurrentZone", expect = "E9 CB 86 16 01")]
pub fn is_car_model_needed_in_current_zone(model: i32) -> bool {
    let zi = g::<GPtr<u8>>(POP_CURR_ZONE_INFO).read();
    if zi.is_null() {
        return false;
    }
    let cheats: [(usize, i32); 6] =
        [(0x969159, 4), (0x96917B, 3), (0x96915E, 0x1E), (0x96915F, 0x1F), (0x96915C, 0x20), (0x969176, 0x21)];
    for (a, grp) in cheats {
        if g::<u8>(a).read() != 0 {
            return population_does_car_group_have_model_id(grp, model) != 0;
        }
    }
    let row = (g::<i32>(0xC0BC70).read() + g::<i32>(0xC0BC74).read() * 2) * 20 + (zi.byte_add(0xF).read() & 0x1F) as i32;
    let freq = GPtr::<u8>::from_addr(0xC0BC78).byte_add((row.wrapping_mul(0x12)) as isize);
    for i in 0..0x12 {
        if freq.byte_add(i as isize).read() != 0 && population_does_car_group_have_model_id(i, model) != 0 {
            return true;
        }
    }
    for i in 0x12..0x1C {
        if zi.byte_add((i - 0x12) as isize).read() != 0 && population_does_car_group_have_model_id(i, model) != 0 {
            return true;
        }
    }
    false
}

/// `CStreaming::Init2()` (0x5B8AD0): see the module docs; every info `Init()` (called), the list
/// heads, models 374..381 marked loaded, model infos with an RW object marked loaded + GAME_REQUIRED
/// (atomic alpha 0xFF), TXD slots with a dictionary marked loaded, the ped slots, the extra objects
/// directory (550 entries), streamed scripts, the CD directories, the two streaming buffers (size
/// rounded up to even, `MallocAlign(size * 2048, 2048)`, buffer 1 in the middle), 50 MB / 22
/// vehicles, the 1000-link RW object list.
#[hook(addr = 0x5B8AD0, conv = cdecl, name = "CStreaming::Init2", expect = "6A FF 68 3B D5 83 00")]
pub fn init2() {
    for id in 0..crate::shared::RESOURCE_ID_TOTAL as i32 {
        call_info_init(info(id));
    }
    STREAMING_INFO_ARRAY_BASE.set(INFO_FOR_MODEL.ptr().cast());
    START_LOADED_LIST.set(info(26312));
    END_LOADED_LIST.set(info(26313));
    START_REQUESTED_LIST.set(info(26314));
    END_REQUESTED_LIST.set(info(26315));
    field::<i16, _>(info(26312), 0).write(0x66C9);
    field::<i16, _>(info(26312), 2).write(-1);
    field::<i16, _>(info(26313), 0).write(-1);
    field::<i16, _>(info(26313), 2).write(0x66C8);
    field::<i16, _>(info(26314), 0).write(0x66CB);
    field::<i16, _>(info(26314), 2).write(-1);
    field::<i16, _>(info(26315), 0).write(-1);
    field::<i16, _>(info(26315), 2).write(0x66CA);
    OLD_SECTOR_X.set(0);
    OLD_SECTOR_Y.set(0);
    for id in 374..382 {
        set_load_state(id, 1);
    }
    STREAMING_BUFFER_SIZE.set(0);
    DISABLE_STREAMING.set(GBool::FALSE);
    MEMORY_USED.set(0);
    CHANNEL_ERROR.set(-1);
    LOADING_BIG_MODEL.set(GBool::FALSE);
    ENABLE_REQUEST_LIST_PURGE.set(GBool::TRUE);
    STREAM_HARVESTER_MODELS_THIS_FRAME.set(GBool::FALSE);
    HARVESTER_MODELS_REQUESTED.set(GBool::FALSE);
    ch_field::<i32>(0, CH_STATUS).write(0);
    ch_field::<i32>(1, CH_STATUS).write(0);
    for i in 0..16 {
        ch_id(0, i).write(-1);
        ch_id(1, i).write(-1);
    }
    for id in 0..20000 {
        let m = mi(id);
        if !m.is_null() && !field::<GPtr<u8>, u8>(m, 0x1C).read().is_null() {
            let a = mi_as_atomic(m);
            set_flags(id, 0);
            set_flags(id, flags(id) | 2);
            set_load_state(id, 1);
            if !a.is_null() {
                a.byte_add(0xC).write(0xFF);
            }
        }
    }
    for slot in 0..5000 {
        let d = txd_def(slot);
        if !d.is_null() && !field::<GPtr<u8>, u8>(d, 0).read().is_null() {
            set_load_state(slot + RESOURCE_ID_TXD, 1);
        }
    }
    car_group_clear(VEHICLES_LOADED.ptr().cast());
    for i in 0..8 {
        PEDS_LOADED.ptr().cast::<i32>().wrapping_index(i).write(-1);
    }
    NUM_PEDS_LOADED.set(0);
    for i in 0..18 {
        NEXT_PED_TO_LOAD_FROM_GROUP.ptr().cast::<i32>().wrapping_index(i).write(0);
    }
    let p = operator_new(0x10);
    let dir = if p.is_null() { GPtr::NULL } else { directory_ctor(p.cast(), 0x226).cast() };
    EXTRA_OBJECTS_DIR.set(dir);
    CURRENT_ZONE_TYPE.set(-1);
    LOADED_GANGS.set(0);
    LAST_CULL_ZONE.set(-1);
    NUM_PRIORITY_REQUESTS.set(0);
    streamed_scripts_initialise();
    call_load_cd_directory();
    if STREAMING_BUFFER_SIZE.get() & 1 != 0 {
        STREAMING_BUFFER_SIZE.set(STREAMING_BUFFER_SIZE.get() + 1);
    }
    let b0 = memory_malloc_align(STREAMING_BUFFER_SIZE.get() << 11, 0x800, 0);
    let bufs = STREAMING_BUFFER.ptr().cast::<GPtr<u8>>();
    bufs.write(b0);
    STREAMING_BUFFER_SIZE.set(((STREAMING_BUFFER_SIZE.get() as i32) / 2) as u32);
    bufs.wrapping_index(1).write(b0.byte_add((STREAMING_BUFFER_SIZE.get() as i32).wrapping_mul(0x800) as isize));
    MEMORY_AVAILABLE.set(0x0320_0000);
    DESIRED_NUM_VEHICLES_LOADED.set(0x16);
    // ms_rwObjectInstances.Init(1000)
    let l = rw_instances();
    let links = operator_new_alias(12000);
    field::<GPtr<u8>, u8>(l, 0x30).write(links);
    let used_head = l;
    let used_tail = l.byte_add(0xC);
    let free_head = l.byte_add(0x18);
    let free_tail = l.byte_add(0x24);
    field::<GPtr<u8>, u8>(used_head, 8).write(used_tail);
    field::<GPtr<u8>, u8>(used_tail, 4).write(used_head);
    field::<GPtr<u8>, u8>(free_head, 8).write(free_tail);
    field::<GPtr<u8>, u8>(free_tail, 4).write(free_head);
    let mut off = 12000isize;
    while off != 0 {
        off -= 0xC;
        let link = field::<GPtr<u8>, u8>(l, 0x30).read().byte_add(off);
        field::<GPtr<u8>, u8>(link, 8).write(field::<GPtr<u8>, u8>(free_head, 8).read());
        let fn_ = field::<GPtr<u8>, u8>(free_head, 8).read();
        field::<GPtr<u8>, u8>(fn_, 4).write(link);
        field::<GPtr<u8>, u8>(link, 4).write(free_head);
        field::<GPtr<u8>, u8>(free_head, 8).write(link);
    }
    IS_INITIALISED.set(1);
}

/// `CStreaming::LoadCdDirectory()` (0x5B82C0): image offsets (0, -1 × 5), `ms_imageSize =
/// GetGTA3ImgSize()`, every used `ms_files` entry flagged not-player read with
/// `LoadCdDirectory(name, index)`, `ms_lastImageRead = 0`, `ms_imageSize >>= 11`.
#[hook(addr = 0x5B82C0, conv = cdecl, name = "CStreaming::LoadCdDirectory-void", expect = "83 C8 FF A3 90 4C 8E 00")]
pub fn load_cd_directory() {
    let off = IMAGE_OFFSETS.ptr().cast::<i32>();
    for i in 1..5 {
        off.wrapping_index(i).write(-1);
    }
    off.write(0);
    off.wrapping_index(5).write(-1);
    IMAGE_SIZE.set(get_gta3_img_size());
    if file(0).read().name[0] != 0 {
        let mut i = 0;
        loop {
            if i >= 8 {
                break;
            }
            if file(i).read().not_player_img != 0 {
                call_load_cd_directory_file(file(i).cast(), i);
            }
            i += 1;
            if i >= 8 || file(i).read().name[0] == 0 {
                break;
            }
        }
    }
    LAST_IMAGE_READ.set(0);
    IMAGE_SIZE.set(IMAGE_SIZE.get() >> 11);
}

/// `CStreaming::LoadCdDirectory(const char* path, int32 img)` (0x5B6170; gr-deviations
/// `Streaming.cpp:1111/1150/1162/1197`; spec CStreaming.deviations.md). Signed `fopen` result test;
/// header and count read; per 32-byte entry: buffer size max (signed vs. the u16 size); extension
/// within 20 characters: DFF (known model → its id, unknown → `pos |= img << 24`, extra directory,
/// skip), TXD (find/add slot, `CVehicleModelInfo::AssignRemapTxd`), COL (`FindColSlot` = -1 →
/// `AddColSlot`), IPL, DAT (`sscanf(name + 5, "%d")`), IFP, RRR, SCM; unknown → '.' restored, skip.
/// An entry that already has a CD position is skipped; otherwise img byte, `SizeInArchive` replacing a
/// non-zero size, `SetCdPosnAndSize(pos, size)`, flags 0, chained as `nextOnCd` of the previous
/// accepted entry (a skipped entry breaks the chain).
#[hook(addr = 0x5B6170, conv = cdecl, name = "CStreaming::LoadCdDirectory-char", expect = "83 EC 3C 8B 44 24 40")]
pub fn load_cd_directory_file(path: GPtr<u8>, img: i32) {
    let f = open_file(path, GPtr::from_addr(0x859C6C));
    if (f.addr() as u32 as i32) <= 0 {
        return;
    }
    let mut header = [0u8; 5];
    file_read(f, GPtr::from_raw(header.as_mut_ptr()), 4);
    let mut count: i32 = 0;
    file_read(f, GPtr::from_raw(&raw mut count).cast(), 4);
    let mut prev = -1;
    let mut id: i32 = 0;
    while count != 0 {
        count -= 1;
        let mut e = sa_core::directory::DirectoryInfo { pos: 0, size: 0, size_in_archive: 0, name: [0; 24] };
        let ep = GPtr::from_raw(&raw mut e);
        file_read(f, ep.cast(), 0x20);
        if (e.size as i32) > STREAMING_BUFFER_SIZE.get() as i32 {
            STREAMING_BUFFER_SIZE.set(e.size as u32);
        }
        let name: GPtr<u8> = ep.byte_add(8).cast();
        name.byte_add(23).write(0);
        let dot = strchr(name, 0x2E);
        if dot.is_null() || dot.addr().wrapping_sub(name.addr()) as i32 > 0x14 {
            name.byte_add(23).write(0);
            prev = -1;
            continue;
        }
        let ext = dot.byte_add(1);
        dot.write(0);
        let is = |s: u32| strnicmp(ext, GPtr::from_addr(s as usize), 3) == 0;
        if is(0x86964C) {
            if mi_get_model_info_by_name(name, GPtr::from_raw(&raw mut id)).is_null() {
                let mut e2 = ep.read();
                e2.pos |= (img as u32) << 24;
                ep.write(e2);
                directory_add_item(EXTRA_OBJECTS_DIR.get().cast(), ep);
                prev = -1;
                continue;
            }
        } else if is(0x869648) {
            id = txd_find_txd_slot(name);
            if id == -1 {
                id = txd_add_txd_slot(name);
                vehicle_assign_remap_txd(name, id);
            }
            id += RESOURCE_ID_TXD;
        } else if is(0x869644) {
            id = col_find_col_slot(name);
            if id == -1 {
                id = col_add_col_slot(name);
            }
            id += RESOURCE_ID_COL;
        } else if is(0x869640) {
            id = ipl_find_ipl_slot(name);
            if id == -1 {
                id = ipl_add_ipl_slot(name);
            }
            id += RESOURCE_ID_IPL;
        } else if is(0x86963C) {
            sscanf_int(name.byte_add(5), GPtr::from_addr(0x859508), GPtr::from_raw(&raw mut id));
            id += RESOURCE_ID_DAT;
        } else if is(0x869638) {
            id = anim_register_anim_block(name) + RESOURCE_ID_IFP;
        } else if is(0x869634) {
            id = vehicle_recording_register_recording_file(name) + RESOURCE_ID_RRR;
        } else if is(0x869630) {
            id = streamed_scripts_register_script(g(0xA47B60), name) + RESOURCE_ID_SCM;
        } else {
            dot.write(b'.');
            prev = -1;
            continue;
        }
        let mut dummy_pos: u32 = 0;
        let mut dummy_size: u32 = 0;
        if call_get_cd_posn_and_size(info(id), GPtr::from_raw(&raw mut dummy_pos), GPtr::from_raw(&raw mut dummy_size)) {
            prev = -1;
            continue;
        }
        let e2 = ep.read();
        field::<u8, _>(info(id), 7).write(img as u8);
        let mut size = e2.size;
        if e2.size_in_archive != 0 {
            size = e2.size_in_archive;
            ep.cast::<u8>().byte_add(4).cast::<u16>().write(size);
        }
        call_set_cd_posn_and_size(info(id), e2.pos, size as u32);
        set_flags(id, 0);
        if prev != -1 {
            field::<i16, _>(info(prev), 4).write(id as i16);
        }
        prev = id;
    }
    fclose(f);
}

/// `CStreaming::ReadIniFile()` (0x5BCCD0; gr-deviations `Streaming.cpp:2031`, keep-original): see the
/// module docs. `memory` (first one only) / `devkit_memory` → `atol << 10`; `vehicles`;
/// `dontbuildpaths`; `pe_lightchangerate/basecap/basemult` → `(float)atof`; `pe_leftx/rightx/topy/
/// bottomy` → int32 `atol`; `pe_bRadiosity` → `atol != 0`; `def_brightness_pal` → `atol`.
#[hook(addr = 0x5BCCD0, conv = cdecl, name = "CStreaming::ReadIniFile", expect = "53 55 68 3C A5 85 00")]
pub fn read_ini_file() {
    let mut memory_set = false;
    let f = open_file(GPtr::from_addr(0x86A8CC), GPtr::from_addr(0x85A53C));
    let mut line = file_loader_load_line(f);
    let s = |a: usize| GPtr::<u8>::from_addr(a);
    while !line.is_null() {
        let c = line.read();
        if c != b'#' && c != 0 {
            let key = strtok(line, s(0x86A8C8));
            let val = strtok(GPtr::NULL, s(0x86A8C8));
            if stricmp(key, s(0x86A8C0)) == 0 && !memory_set {
                MEMORY_AVAILABLE.set((atol(val) << 10) as u32);
            } else if stricmp(key, s(0x86A8B0)) == 0 {
                MEMORY_AVAILABLE.set((atol(val) << 10) as u32);
                memory_set = true;
            } else if stricmp(key, s(0x86A8A4)) == 0 {
                DESIRED_NUM_VEHICLES_LOADED.set(atol(val));
            } else if stricmp(key, s(0x86A894)) == 0 {
                g::<u8>(0x96F016).write(1);
            } else if stricmp(key, s(0x86A880)) == 0 {
                g::<f32>(0x8D5168).write(atof(val) as f32);
            } else if stricmp(key, s(0x86A86C)) == 0 {
                g::<f32>(0x8D516C).write(atof(val) as f32);
            } else if stricmp(key, s(0x86A858)) == 0 {
                g::<f32>(0x8D5170).write(atof(val) as f32);
            } else if stricmp(key, s(0x86A84C)) == 0 {
                g::<i32>(0x8D5150).write(atol(val));
            } else if stricmp(key, s(0x86A840)) == 0 {
                g::<i32>(0x8D5154).write(atol(val));
            } else if stricmp(key, s(0x86A838)) == 0 {
                g::<i32>(0x8D5158).write(atol(val));
            } else if stricmp(key, s(0x86A82C)) == 0 {
                g::<i32>(0x8D515C).write(atol(val));
            } else if stricmp(key, s(0x86A81C)) == 0 {
                g::<u8>(0xC402CC).write((atol(val) != 0) as u8);
            } else if stricmp(key, s(0x86A808)) == 0 {
                g::<i32>(0xBA6784).write(atol(val));
            }
        }
        line = file_loader_load_line(f);
    }
    fclose(f);
}

/// `CStreaming::Save()` (0x5D29A0, AL): per info the flags byte when loaded, else 0xFF, written one byte
/// at a time (`CGenericGameStorage::SaveDataToWorkBuffer(&b, 1)`); true.
#[hook(addr = 0x5D29A0, conv = cdecl, name = "CStreaming::Save", expect = "51 56 BE C6 4C 8E 00")]
pub fn save() -> bool {
    for id in 0..crate::shared::RESOURCE_ID_TOTAL as i32 {
        let mut b: u8 = if load_state(id) == 1 { flags(id) } else { 0xFF };
        generic_game_storage_save_data(GPtr::from_raw(&raw mut b), 1);
    }
    true
}

/// `CStreaming::Load()` (0x5D29E0, AL): per info one byte read; loaded entries or it into their flags
/// unless 0xFF; true.
#[hook(addr = 0x5D29E0, conv = cdecl, name = "CStreaming::Load", expect = "51 56 BE C6 4C 8E 00")]
pub fn load() -> bool {
    for id in 0..crate::shared::RESOURCE_ID_TOTAL as i32 {
        let mut b: u8 = 0;
        generic_game_storage_load_data(GPtr::from_raw(&raw mut b), 1);
        if load_state(id) == 1 && b != 0xFF {
            set_flags(id, flags(id) | b);
        }
    }
    true
}

// Not ported (deferred), always the original through `orig!`:
// - 0x40D3F0 `CStreaming::AddModelsToRequestList` (297 instructions of camera/frustum x87 maths over
//   sector rings; needs a byte-level pass with CPU-checked x87),
// - 0x40D7C0 `CStreaming::DeleteRwObjectsBehindCamera` (738 instructions, same),
// - 0x40A560 `CStreaming::StreamZoneModels` (347 instructions; its entry chunk 0x40145A computes a
//   pointer with `ror [0x1560638], [0x727B61]`).

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[HookReg] = &[
    HOOK_REQUEST_TXD_MODEL,
    HOOK_ADD_IMAGE_TO_LIST,
    HOOK_IS_VERY_BUSY,
    HOOK_IS_OBJECT_IN_CD_IMAGE,
    HOOK_FORCE_LAYER_TO_READ,
    HOOK_SET_LOAD_VEHICLES_IN_LOAD_SCENE,
    HOOK_CLEAR_FLAG_FOR_ALL,
    HOOK_ARE_ANIMS_USED_BY_REQUESTED_MODELS,
    HOOK_GET_DEFAULT_COP_MODEL,
    HOOK_GET_DEFAULT_COP_CAR_MODEL,
    HOOK_DISABLE_COP_BIKES,
    HOOK_GET_DEFAULT_MEDIC_MODEL,
    HOOK_GET_DEFAULT_FIREMAN_MODEL,
    HOOK_GET_DEFAULT_CAB_DRIVER_MODEL,
    HOOK_HAS_SPECIAL_CHAR_LOADED,
    HOOK_WE_ARE_TRYING_TO_PHASE_VEHICLE_OUT,
    HOOK_FIND_MI_PED_SLOT_FOR_INTERIOR,
    HOOK_LINK_LIST_INSERT,
    HOOK_INIT_IMAGE_LIST,
    HOOK_SHUTDOWN,
    HOOK_REQUEST_MODEL,
    HOOK_INSTANCE_LOADED_MODELS,
    HOOK_DELETE_ALL_RW_OBJECTS,
    HOOK_DELETE_RW_OBJECTS_AFTER_DEATH,
    HOOK_REMOVE_BIG_BUILDINGS,
    HOOK_REQUEST_BIG_BUILDINGS,
    HOOK_REMOVE_BUILDINGS_NOT_IN_AREA,
    HOOK_ADD_ENTITY,
    HOOK_START_RENDER_ENTITIES,
    HOOK_RENDER_ENTITY,
    HOOK_REMOVE_ENTITY,
    HOOK_DELETE_LEAST_USED_ENTITY_RW_OBJECT,
    HOOK_DELETE_RW_OBJECTS_IN_SECTOR_LIST,
    HOOK_DELETE_RW_OBJECTS_BEHIND_CAMERA_IN_SECTOR_LIST,
    HOOK_DELETE_RW_OBJECTS_NOT_IN_FRUSTUM_IN_SECTOR_LIST,
    HOOK_ARE_TEXTURES_USED_BY_REQUESTED_MODELS,
    HOOK_SET_MODEL_IS_DELETABLE,
    HOOK_SET_MODEL_TXD_IS_DELETABLE,
    HOOK_SET_MISSION_DOESNT_REQUIRE_MODEL,
    HOOK_REQUEST_FILE,
    HOOK_LOAD_INITIAL_WEAPONS,
    HOOK_STREAM_COP_MODELS,
    HOOK_STREAM_AMBULANCE_AND_MEDIC,
    HOOK_STREAM_FIRE_ENGINE_AND_FIREMAN,
    HOOK_STREAM_ZONE_MODELS_GANGS,
    HOOK_RECLASSIFY_LOADED_CARS,
    HOOK_REMOVE_CURRENT_ZONES_MODELS,
    HOOK_REMOVE_LOADED_ZONE_MODEL,
    HOOK_REMOVE_INAPPROPRIATE_PED_MODELS,
    HOOK_REQUEST_SPECIAL_CHAR,
    HOOK_SET_MISSION_DOESNT_REQUIRE_SPECIAL_CHAR,
    HOOK_SET_MISSION_DOESNT_REQUIRE_ANIM,
    HOOK_LOAD_ZONE_VEHICLE,
    HOOK_STREAM_ONE_NEW_CAR,
    HOOK_STREAM_VEHICLES_AND_PEDS_ALWAYS,
    HOOK_STREAM_VEHICLES_AND_PEDS,
    HOOK_POSSIBLY_STREAM_CAR_OUT_AFTER_CREATION,
    HOOK_CLEAR_SLOTS,
    HOOK_STREAM_PEDS_FOR_INTERIOR,
    HOOK_STREAM_PEDS_INTO_RANDOM_SLOTS,
    HOOK_REMOVE_DODGY_PEDS_FROM_RANDOM_SLOTS,
    HOOK_REMOVE_LOADED_VEHICLE,
    HOOK_REMOVE_TXD_MODEL,
    HOOK_PURGE_REQUEST_LIST,
    HOOK_PROCESS_ENTITIES_IN_SECTOR_LIST,
    HOOK_PROCESS_ENTITIES_IN_SECTOR_LIST_RADIUS,
    HOOK_ADD_LODS_TO_REQUEST_LIST,
    HOOK_REMOVE_MODEL,
    HOOK_REQUEST_VEHICLE_UPGRADE,
    HOOK_HAS_VEHICLE_UPGRADE_LOADED,
    HOOK_FINISH_LOADING_LARGE_FILE,
    HOOK_GET_NEXT_FILE_ON_CD,
    HOOK_REQUEST_FILES_IN_CHANNEL,
    HOOK_REQUEST_SPECIAL_MODEL,
    HOOK_CONVERT_BUFFER_TO_OBJECT,
    HOOK_REQUEST_MODEL_STREAM,
    HOOK_REMOVE_ALL_UNUSED_MODELS,
    HOOK_REMOVE_LEAST_USED_MODEL,
    HOOK_REMOVE_REFERENCED_TXDS,
    HOOK_LOAD_INITIAL_PEDS,
    HOOK_MAKE_SPACE_FOR,
    HOOK_PROCESS_LOADING_CHANNEL,
    HOOK_LOAD_REQUESTED_MODELS,
    HOOK_FLUSH_CHANNELS,
    HOOK_FLUSH_REQUEST_LIST,
    HOOK_RE_INIT,
    HOOK_LOAD_ALL_REQUESTED_MODELS,
    HOOK_LOAD_SCENE,
    HOOK_LOAD_SCENE_COLLISION,
    HOOK_UPDATE,
    HOOK_RETRY_LOAD_FILE,
    HOOK_ADD_TO_LOADED_VEHICLES_LIST,
    HOOK_REMOVE_CAR_MODEL,
    HOOK_IS_CAR_MODEL_NEEDED_IN_CURRENT_ZONE,
    HOOK_INIT2,
    HOOK_LOAD_CD_DIRECTORY,
    HOOK_LOAD_CD_DIRECTORY_FILE,
    HOOK_READ_INI_FILE,
    HOOK_SAVE,
    HOOK_LOAD,
];
