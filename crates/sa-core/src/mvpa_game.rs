//! WRITER-MVP-A batch `mvpa-core-1`: the `CGame` initialisation orchestrators on the native boot path
//! (`Initialise`, `Init1`, `Init2`, `Init3`, `InitialiseCoreDataAfterRW`). Ported from the 1.0 US bytes:
//! the same callees in the same order with the same arguments, every callee through `orig!` at its
//! original address. The `bool` results are returned as the original leaves AL; the upper EAX bits are
//! whatever the last callee returned in the original (declared `ax-al-return`). Unverified
//! (docs/verify/HOWTO.md).

use crate::mem::GPtr;
use crate::{hook, orig};

/// `CGame::aDatFile` (char[32], 0xB728EC)
const DAT_FILE: usize = 0xB728EC;
/// `TheCamera`
const THE_CAMERA: u32 = 0xB6F028;
/// `ThePaths`
const THE_PATHS: u32 = 0x96F050;
/// The loading-screen title the init steps pass ("Loading the Game", 0x863A6C)
const LS_TITLE: u32 = 0x863A6C;

orig! {
    /// `CLoadingScreen::LoadingScreen(const char*, const char*, const char*)` (0x53DED0)
    fn loading_screen(a: u32, b: u32, c: u32) = cdecl 0x53DED0 => unported;
    /// `CGame::Init1(const char*)` (0x5BF840)
    fn game_init1(dat: GPtr<u8>) -> u8 = cdecl 0x5BF840 => init1;
    /// `CColAccel::startCache()` (0x5B31A0)
    fn col_accel_start_cache() = cdecl 0x5B31A0 => unported;
    /// `CFileLoader::LoadLevel(const char*)` (0x5B9030)
    fn file_loader_load_level(f: u32) = cdecl 0x5B9030 => unported;
    /// `CColAccel::endCache()` (0x5B2AD0)
    fn col_accel_end_cache() = cdecl 0x5B2AD0 => unported;
    /// `CGame::Init2(const char*)` (0x5BA1A0)
    fn game_init2(dat: GPtr<u8>) -> u8 = cdecl 0x5BA1A0 => init2;
    /// `CStencilShadows::Init()` (0x70F9E0)
    fn stencil_shadows_init() = cdecl 0x70F9E0 => unported;
    /// `CTheScripts::StartTestScript()` (0x464D40)
    fn scripts_start_test_script() = cdecl 0x464D40 => unported;
    /// `CTheScripts::Process()` (0x46A000)
    fn scripts_process() = cdecl 0x46A000 => unported;
    /// `CCamera::Process()` (0x52B730)
    fn camera_process(c: u32) = thiscall 0x52B730 => unported;
    /// `CGame::Init3(const char*)` (0x5BA400)
    fn game_init3(dat: GPtr<u8>) -> u8 = cdecl 0x5BA400 => init3;
    /// `CMaths::InitMathsTables()` (0x59AC90)
    fn maths_init_maths_tables() = cdecl 0x59AC90 => unported;
    /// `CPools::Initialise()` (0x550F10)
    fn pools_initialise() = cdecl 0x550F10 => unported;
    /// `CPlaceable::InitMatrixArray()` (0x54F3A0)
    fn placeable_init_matrix_array() = cdecl 0x54F3A0 => unported;
    /// `CIniFile::LoadIniFile()` (0x56D070)
    fn ini_file_load_ini_file() = cdecl 0x56D070 => unported;
    /// `D3DResourceSystem::SetUseD3DResourceBuffering(bool)` (0x730AC0)
    fn d3d_set_use_resource_buffering(b: u32) = cdecl 0x730AC0 => unported;
    /// `CTxdStore::AddTxdSlot(const char*)` (0x731C80)
    fn txd_add_txd_slot(name: u32) -> i32 = cdecl 0x731C80 => unported;
    /// `CTxdStore::Create(int32)` (0x731990)
    fn txd_create(i: i32) = cdecl 0x731990 => unported;
    /// `CTxdStore::AddRef(int32)` (0x731A00)
    fn txd_add_ref(i: i32) = cdecl 0x731A00 => unported;
    /// `CTxdStore::LoadTxd(int32, const char*)` (0x7320B0)
    fn txd_load_txd(i: i32, f: u32) = cdecl 0x7320B0 => unported;
}

orig! {
    /// `CTxdStore::SetCurrentTxd(int32)` (0x7319C0)
    fn txd_set_current_txd(i: i32) = cdecl 0x7319C0 => unported;
    /// `CGameLogic::InitAtStartOfGame()` (0x441210)
    fn game_logic_init_at_start_of_game() = cdecl 0x441210 => unported;
    /// `CGangWars::InitAtStartOfGame()` (0x443920)
    fn gang_wars_init_at_start_of_game() = cdecl 0x443920 => unported;
    /// `CConversations::Clear()` (0x43A7B0)
    fn conversations_clear() = cdecl 0x43A7B0 => unported;
    /// `CPedToPlayerConversations::Clear()` (0x43AAE0)
    fn p2p_conversations_clear() = cdecl 0x43AAE0 => unported;
    /// `CQuadTreeNode::InitPool()` (0x552C00)
    fn quad_tree_init_pool() = cdecl 0x552C00 => unported;
    /// `CPlantMgr::Initialise()` (0x5DD910, AL)
    fn plant_mgr_initialise() -> u8 = cdecl 0x5DD910 => unported;
    /// `CCustomRoadsignMgr::Initialise()` (0x6FE120, AL)
    fn roadsign_mgr_initialise() -> u8 = cdecl 0x6FE120 => unported;
    /// `CReferences::Init()` (0x5719B0)
    fn references_init() = cdecl 0x5719B0 => unported;
    /// 0x532240 (no catalogue name; called between `CReferences::Init` and `CTagManager::Init`)
    fn fun_00532240() = cdecl 0x532240 => unported;
    /// `CTagManager::Init()` (0x49CC50)
    fn tag_manager_init() = cdecl 0x49CC50 => unported;
    /// `CWeather::Init()` (0x72A480)
    fn weather_init() = cdecl 0x72A480 => unported;
    /// `CCover::Init()` (0x698710)
    fn cover_init() = cdecl 0x698710 => unported;
    /// `CCullZones::Init()` (0x72D6B0)
    fn cull_zones_init() = cdecl 0x72D6B0 => unported;
    /// `COcclusion::Init()` (0x71DCA0)
    fn occlusion_init() = cdecl 0x71DCA0 => unported;
    /// `CCollision::Init()` (0x416260)
    fn collision_init() = cdecl 0x416260 => unported;
    /// `CBirds::Init()` (0x711EC0)
    fn birds_init() = cdecl 0x711EC0 => unported;
    /// `CEntryExitManager::Init()` (0x43F880)
    fn entry_exit_manager_init() = cdecl 0x43F880 => unported;
    /// `CStuntJumpManager::Init()` (0x49CA50)
    fn stunt_jump_manager_init() = cdecl 0x49CA50 => unported;
    /// `CSetPieces::Init()` (0x4994F0)
    fn set_pieces_init() = cdecl 0x4994F0 => unported;
}

orig! {
    /// `CTheZones::Init()` (0x572670)
    fn zones_init() = cdecl 0x572670 => unported;
    /// `CUserDisplay::Init()` (0x571EE0)
    fn user_display_init() = cdecl 0x571EE0 => unported;
    /// `CMessages::Init()` (0x69EE00)
    fn messages_init() = cdecl 0x69EE00 => unported;
    /// `CMessages::ClearAllMessagesDisplayedByGame(bool)` (0x69EDC0)
    fn messages_clear_all_displayed_by_game(b: u32) = cdecl 0x69EDC0 => unported;
    /// `CVehicleRecording::Init()` (0x459390)
    fn vehicle_recording_init() = cdecl 0x459390 => unported;
    /// `CRestart::Initialise()` (0x460630)
    fn restart_initialise() = cdecl 0x460630 => unported;
    /// `CWorld::Initialise()` (0x5631E0)
    fn world_initialise() = cdecl 0x5631E0 => unported;
    /// `CAnimManager::Initialise()` (0x5BF6B0)
    fn anim_manager_initialise() = cdecl 0x5BF6B0 => unported;
    /// `CCutsceneMgr::Initialise()` (0x4D5A20)
    fn cutscene_mgr_initialise() = cdecl 0x4D5A20 => unported;
    /// `CCarCtrl::Init()` (0x4212E0)
    fn car_ctrl_init() = cdecl 0x4212E0 => unported;
    /// `ModelIndices::InitModelIndices()` (0x5BCA10)
    fn init_model_indices() = cdecl 0x5BCA10 => unported;
    /// `CModelInfo::Initialise()` (0x4C6810)
    fn model_info_initialise() = cdecl 0x4C6810 => unported;
    /// `CPickups::Init()` (0x454A70)
    fn pickups_init() = cdecl 0x454A70 => unported;
    /// `CTheCarGenerators::Init()` (0x6F3270)
    fn car_generators_init() = cdecl 0x6F3270 => unported;
    /// `CGarages::Init()` (0x447120)
    fn garages_init() = cdecl 0x447120 => unported;
    /// `CAudioZones::Init()` (0x5081A0)
    fn audio_zones_init() = cdecl 0x5081A0 => unported;
    /// `CStreaming::InitImageList()` (0x4083C0)
    fn streaming_init_image_list() = cdecl 0x4083C0 => unported;
    /// `CStreaming::ReadIniFile()` (0x5BCCD0)
    fn streaming_read_ini_file() = cdecl 0x5BCCD0 => unported;
    /// `CPathFind::Init()` (0x44D080)
    fn path_find_init(p: u32) = thiscall 0x44D080 => unported;
    /// 0x44D2B0 (`CPathFind` method, a bare `ret` in 1.0 US)
    fn path_find_fun_0044d2b0(p: u32) = thiscall 0x44D2B0 => unported;
}

orig! {
    /// `CTaskSimpleFight::LoadMeleeData()` (0x5BEDC0)
    fn fight_load_melee_data() = cdecl 0x5BEDC0 => unported;
    /// `CCheat::ResetCheats()` (0x438450)
    fn cheat_reset_cheats() = cdecl 0x438450 => unported;
    /// `Fx_c::Init()` (0x49EA90)
    fn fx_init(fx: u32) = thiscall 0x49EA90 => unported;
    /// `CTheZones::PostZoneCreation()` (0x572B70)
    fn zones_post_zone_creation() = cdecl 0x572B70 => unported;
    /// `CEntryExitManager::PostEntryExitsCreation()` (0x43F0A0)
    fn entry_exit_post_creation() = cdecl 0x43F0A0 => unported;
    /// 0x44D0E0 (`CPathFind` method, no catalogue name; `ThePaths.PreparePathData`)
    fn path_find_fun_0044d0e0(p: u32) = thiscall 0x44D0E0 => unported;
    /// `CPlayerInfo::Clear()` (0x56F330)
    fn player_info_clear(p: u32) = thiscall 0x56F330 => unported;
    /// `CPlayerInfo::LoadPlayerSkin()` (0x56F7D0)
    fn player_info_load_player_skin(p: u32) = thiscall 0x56F7D0 => unported;
    /// `TestModelIndices()` (0x53ED30)
    fn test_model_indices() = cdecl 0x53ED30 => unported;
    /// `CClothes::Init()` (0x5A80D0)
    fn clothes_init() = cdecl 0x5A80D0 => unported;
    /// `CWaterLevel::WaterLevelInitialise()` (0x6EAE80)
    fn water_level_initialise() = cdecl 0x6EAE80 => unported;
    /// `CDraw::SetFOV(float)` (0x6FF410)
    fn draw_set_fov(fov: u32) = cdecl 0x6FF410 => unported;
    /// `CCustomCarPlateMgr::Initialise()` (0x6FD500, AL)
    fn car_plate_mgr_initialise() -> u8 = cdecl 0x6FD500 => unported;
    /// `CStreaming::LoadInitialVehicles()` (0x407F20)
    fn streaming_load_initial_vehicles() = cdecl 0x407F20 => unported;
    /// `CStreaming::LoadInitialPeds()` (0x40D3D0)
    fn streaming_load_initial_peds() = cdecl 0x40D3D0 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    fn streaming_load_all_requested_models(b: u32) = cdecl 0x40EA10 => unported;
    /// `CAnimManager::LoadAnimFiles()` (0x4D5620)
    fn anim_manager_load_anim_files() = cdecl 0x4D5620 => unported;
    /// `CStreaming::LoadInitialWeapons()` (0x40A120)
    fn streaming_load_initial_weapons() = cdecl 0x40A120 => unported;
    /// `CPed::Initialise()` (0x5DEBB0)
    fn ped_initialise() = cdecl 0x5DEBB0 => unported;
    /// `CRenderer::Init()` (0x5531C0)
    fn renderer_init() = cdecl 0x5531C0 => unported;
}

orig! {
    /// `CRadar::Initialise()` (0x587FB0)
    fn radar_initialise() = cdecl 0x587FB0 => unported;
    /// `CRadar::LoadTextures()` (0x5827D0)
    fn radar_load_textures() = cdecl 0x5827D0 => unported;
    /// `CWeapon::InitialiseWeapons()` (0x73A300)
    fn weapon_initialise_weapons() = cdecl 0x73A300 => unported;
    /// `CRoadBlocks::Init()` (0x461100)
    fn road_blocks_init() = cdecl 0x461100 => unported;
    /// `CPopulation::Initialise()` (0x610E10)
    fn population_initialise() = cdecl 0x610E10 => unported;
    /// `CCoronas::Init()` (0x6FAA70)
    fn coronas_init() = cdecl 0x6FAA70 => unported;
    /// `CShadows::Init()` (0x706CD0)
    fn shadows_init() = cdecl 0x706CD0 => unported;
    /// `CWeaponEffects::Init()` (0x742AB0)
    fn weapon_effects_init() = cdecl 0x742AB0 => unported;
    /// `CSkidmarks::Init()` (0x7204E0)
    fn skidmarks_init() = cdecl 0x7204E0 => unported;
    /// `CGlass::Init()` (0x71A8D0)
    fn glass_init() = cdecl 0x71A8D0 => unported;
    /// `CGarages::Init_AfterRestart()` (0x448B60)
    fn garages_init_after_restart() = cdecl 0x448B60 => unported;
    /// `CTheScripts::Init()` (0x468D50)
    fn scripts_init() = cdecl 0x468D50 => unported;
    /// `CGangs::Initialise()` (0x5DE680)
    fn gangs_initialise() = cdecl 0x5DE680 => unported;
    /// `CClock::Initialise(uint32)` (0x52CD90)
    fn clock_initialise(scale: u32) = cdecl 0x52CD90 => unported;
    /// `CHeli::InitHelis()` (0x6C4560)
    fn heli_init_helis() = cdecl 0x6C4560 => unported;
    /// `CCranes::InitCranes()` (0x6F3FC0)
    fn cranes_init_cranes() = cdecl 0x6F3FC0 => unported;
    /// `CMovingThings::Init()` (0x717D00)
    fn moving_things_init() = cdecl 0x717D00 => unported;
    /// `CDarkel::Init()` (0x43CEB0)
    fn darkel_init() = cdecl 0x43CEB0 => unported;
    /// `CGridRef::Init()` (0x71D4E0)
    fn grid_ref_init() = cdecl 0x71D4E0 => unported;
    /// `CStats::Init()` (0x55C0C0)
    fn stats_init() = cdecl 0x55C0C0 => unported;
}

orig! {
    /// `CClouds::Init()` (0x7138D0)
    fn clouds_init() = cdecl 0x7138D0 => unported;
    /// `CSpecialFX::Init()` (0x7268F0)
    fn special_fx_init() = cdecl 0x7268F0 => unported;
    /// `CRopes::Init()` (0x555DC0)
    fn ropes_init() = cdecl 0x555DC0 => unported;
    /// `CWaterCannons::Init()` (0x728C80)
    fn water_cannons_init() = cdecl 0x728C80 => unported;
    /// `CBridge::Init()` (0x41BC70)
    fn bridge_init() = cdecl 0x41BC70 => unported;
    /// `CPedGroups::Init()` (0x5FB8C0)
    fn ped_groups_init() = cdecl 0x5FB8C0 => unported;
    /// `CInformFriendsEventQueue::Init()` (0x4B2990)
    fn inform_friends_event_queue_init() = cdecl 0x4B2990 => unported;
    /// `CInformGroupEventQueue::Init()` (0x4B2AD0)
    fn inform_group_event_queue_init() = cdecl 0x4B2AD0 => unported;
    /// `CReplay::Init()` (0x45E220)
    fn replay_init() = cdecl 0x45E220 => unported;
    /// `CShopping::Init()` (0x49C290)
    fn shopping_init() = cdecl 0x49C290 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(n: i32) -> u32 = cdecl 0x53FB70 => unported;
    /// `CPad::Clear(bool enablePlayerControls, bool resetPhase)` (0x541A70, `ret 8`)
    fn pad_clear(p: u32, a: u32, b: u32) = thiscall 0x541A70 => unported;
    /// `InteriorManager_c::Init()` (0x5C0500, AL)
    fn interior_manager_init(m: u32) -> u8 = thiscall 0x5C0500 => unported;
    /// `ProcObjectMan_c::Init()` (0x5A3EA0)
    fn proc_object_man_init(m: u32) -> u8 = thiscall 0x5A3EA0 => unported;
    /// `WaterCreatureManager_c::Init()` (0x6E3F90)
    fn water_creature_manager_init(m: u32) -> u8 = thiscall 0x6E3F90 => unported;
    /// `CRealTimeShadowManager::Init()` (0x7067C0)
    fn real_time_shadow_manager_init(m: u32) = thiscall 0x7067C0 => unported;
    /// `CTempColModels::Initialise()` (0x5BA8C0)
    fn temp_col_models_initialise() = cdecl 0x5BA8C0 => unported;
    /// `cHandlingDataMgr::LoadHandlingData()` (0x5BD830)
    fn handling_load_handling_data(m: u32) = thiscall 0x5BD830 => unported;
    /// `SurfaceInfos_c::Init()` (0x55F420)
    fn surface_infos_init(s: u32) = thiscall 0x55F420 => unported;
    /// `CPedStats::Initialise()` (0x5BF9D0)
    fn ped_stats_initialise() = cdecl 0x5BF9D0 => unported;
}

orig! {
    /// `CTimeCycle::Initialise(bool)` (0x5BBAC0)
    fn time_cycle_initialise(b: u32) = cdecl 0x5BBAC0 => unported;
    /// `CPopCycle::Initialise()` (0x5BC090)
    fn pop_cycle_initialise() = cdecl 0x5BC090 => unported;
    /// `CVehicleRecording::InitAtStartOfGame()` (0x45A1B0)
    fn vehicle_recording_init_at_start_of_game() = cdecl 0x45A1B0 => unported;
    /// `CAudioEngine::Initialise()` (0x5B9C60, AL)
    fn audio_engine_initialise(e: u32) -> u8 = thiscall 0x5B9C60 => unported;
    /// `BreakManager_c::Init()` (0x59E650, AL)
    fn break_manager_init(m: u32) -> u8 = thiscall 0x59E650 => unported;
    /// `BoneNodeManager_c::Init()` (0x6173F0, AL)
    fn bone_node_manager_init(m: u32) -> u8 = thiscall 0x6173F0 => unported;
    /// `IKChainManager_c::Init()` (0x6180A0, AL)
    fn ik_chain_manager_init(m: u32) -> u8 = thiscall 0x6180A0 => unported;
}

/// `CGame::Initialise(const char* datFile)` (0x53BC80, cdecl): `Init1(datFile)`, `CColAccel::startCache`,
/// `LoadLevel("DATA\DEFAULT.DAT")` (0x863A90) and `LoadLevel(datFile)`, `endCache`, `Init2(datFile)`,
/// `CStencilShadows::Init`, a loading screen ("Start script", 0x863A80), `StartTestScript`,
/// `CTheScripts::Process`, `TheCamera.Process()`, `Init3(datFile)`; returns true. The results of the
/// three init steps are ignored, as in the original.
#[hook(addr = 0x53BC80, conv = cdecl, name = "CGame::Initialise", expect = "56 8B 74 24 08",
       unverified = "mvpa-core-1", route = "L3-only", hazards = "ax-al-return", recs = "grove-walk",
       note = "the whole game load: one call per run, verify by L3 / native boot")]
pub fn initialise(dat: GPtr<u8>) -> u8 {
    game_init1(dat);
    col_accel_start_cache();
    file_loader_load_level(0x863A90);
    file_loader_load_level(dat.addr() as u32);
    col_accel_end_cache();
    game_init2(dat);
    stencil_shadows_init();
    loading_screen(LS_TITLE, 0x863A80, 0);
    scripts_start_test_script();
    scripts_process();
    camera_process(THE_CAMERA);
    game_init3(dat);
    1
}

/// `CGame::Init1(const char* datFile)` (0x5BF840, cdecl): `InitMathsTables`, `strcpy(aDatFile,
/// datFile)` (byte loop), the pools, matrix array, ini file, `SetUseD3DResourceBuffering(false)`,
/// `currLevel`/`currArea` (0xB7290C / 0xB72914) = 0, the "generic" txd slot (0x858A34; created, AddRef'd,
/// kept in `gameTxdSlot` 0xB728E8), the "particle" slot (0x85C648) loaded from `models\particle.txd`
/// (0x86AB6C) and AddRef'd, `SetCurrentTxd(gameTxdSlot)`, then the subsystem inits in the original order.
/// Returns false when `CPlantMgr::Initialise` or `CCustomRoadsignMgr::Initialise` fails, true at the end.
#[hook(addr = 0x5BF840, conv = cdecl, name = "CGame::Init1", expect = "E8 4B B4 FD FF",
       unverified = "mvpa-core-1", route = "L3-only", hazards = "ax-al-return", recs = "grove-walk",
       note = "one call per load")]
pub fn init1(dat: GPtr<u8>) -> u8 {
    maths_init_maths_tables();
    let mut k = 0;
    loop {
        let b = dat.byte_add(k).read();
        GPtr::<u8>::from_addr(DAT_FILE + k as usize).write(b);
        k += 1;
        if b == 0 {
            break;
        }
    }
    pools_initialise();
    placeable_init_matrix_array();
    ini_file_load_ini_file();
    d3d_set_use_resource_buffering(0);
    GPtr::<u32>::from_addr(0xB7290C).write(0);
    GPtr::<u32>::from_addr(0xB72914).write(0);
    let slot = txd_add_txd_slot(0x858A34);
    GPtr::<i32>::from_addr(0xB728E8).write(slot);
    txd_create(slot);
    txd_add_ref(GPtr::<i32>::from_addr(0xB728E8).read());
    let particle = txd_add_txd_slot(0x85C648);
    txd_load_txd(particle, 0x86AB6C);
    txd_add_ref(particle);
    txd_set_current_txd(GPtr::<i32>::from_addr(0xB728E8).read());
    game_logic_init_at_start_of_game();
    gang_wars_init_at_start_of_game();
    conversations_clear();
    p2p_conversations_clear();
    quad_tree_init_pool();
    if plant_mgr_initialise() == 0 || roadsign_mgr_initialise() == 0 {
        return 0;
    }
    references_init();
    fun_00532240();
    tag_manager_init();
    weather_init();
    cover_init();
    cull_zones_init();
    occlusion_init();
    collision_init();
    birds_init();
    entry_exit_manager_init();
    stunt_jump_manager_init();
    set_pieces_init();
    zones_init();
    user_display_init();
    messages_init();
    messages_clear_all_displayed_by_game(0);
    vehicle_recording_init();
    restart_initialise();
    world_initialise();
    anim_manager_initialise();
    cutscene_mgr_initialise();
    car_ctrl_init();
    init_model_indices();
    model_info_initialise();
    pickups_init();
    car_generators_init();
    garages_init();
    audio_zones_init();
    streaming_init_image_list();
    streaming_read_ini_file();
    path_find_init(THE_PATHS);
    path_find_fun_0044d2b0(THE_PATHS);
    fight_load_melee_data();
    cheat_reset_cheats();
    fx_init(0xA9AE00);
    1
}

/// `CGame::Init2(const char* datFile)` (0x5BA1A0, cdecl): the post-load steps between loading screens
/// (zones/entry-exits post creation, path data, the two `CPlayerInfo`s cleared and player 0's skin,
/// clothes, water, FOV 70, `CDraw::ms_fFarClipZ`... 0xC3EF98 = 500.0, car plates). Returns false when
/// `CCustomCarPlateMgr::Initialise` fails; else the initial models, animations, peds, renderer, radar,
/// weapons, population, effects, scripts, clock (1000 ms per minute) and the rest, true at the end.
#[hook(addr = 0x5BA1A0, conv = cdecl, name = "CGame::Init2", expect = "56 6A 00 68 A0 A5 86 00",
       unverified = "mvpa-core-1", route = "L3-only", hazards = "ax-al-return", recs = "grove-walk",
       note = "one call per load; the datFile argument is unused")]
pub fn init2(_dat: GPtr<u8>) -> u8 {
    loading_screen(LS_TITLE, 0x86A5A0, 0);
    zones_post_zone_creation();
    entry_exit_post_creation();
    loading_screen(LS_TITLE, 0x86A594, 0);
    path_find_fun_0044d0e0(THE_PATHS);
    let mut p = 0xB7CD98u32;
    while p < 0xB7D0B8 {
        player_info_clear(p);
        p += 0x190;
    }
    player_info_load_player_skin(0xB7CD98);
    test_model_indices();
    loading_screen(LS_TITLE, 0x86A588, 0);
    clothes_init();
    water_level_initialise();
    draw_set_fov(0x42F0_0000);
    GPtr::<u32>::from_addr(0xC3EF98).write(0x43FA_0000);
    if car_plate_mgr_initialise() == 0 {
        return 0;
    }
    loading_screen(LS_TITLE, 0x86A578, 0);
    streaming_load_initial_vehicles();
    streaming_load_initial_peds();
    streaming_load_all_requested_models(0);
    loading_screen(LS_TITLE, 0x86A568, 0);
    anim_manager_load_anim_files();
    loading_screen(LS_TITLE, 0x86A578, 0);
    streaming_load_initial_weapons();
    streaming_load_all_requested_models(0);
    loading_screen(LS_TITLE, 0x86A55C, 0);
    ped_initialise();
    renderer_init();
    loading_screen(LS_TITLE, 0x86A544, 0);
    radar_initialise();
    radar_load_textures();
    loading_screen(0, 0, 0);
    weapon_initialise_weapons();
    road_blocks_init();
    loading_screen(0, 0, 0);
    population_initialise();
    loading_screen(0, 0, 0);
    GPtr::<u8>::from_addr(0xB7CD74).write(0);
    coronas_init();
    shadows_init();
    loading_screen(0, 0, 0);
    weapon_effects_init();
    skidmarks_init();
    glass_init();
    garages_init_after_restart();
    loading_screen(LS_TITLE, 0x86A534, 0);
    scripts_init();
    gangs_initialise();
    loading_screen(LS_TITLE, 0x86A544, 0);
    clock_initialise(1000);
    heli_init_helis();
    cranes_init_cranes();
    loading_screen(0, 0, 0);
    moving_things_init();
    darkel_init();
    loading_screen(0, 0, 0);
    grid_ref_init();
    stats_init();
    clouds_init();
    special_fx_init();
    loading_screen(0, 0, 0);
    ropes_init();
    water_cannons_init();
    bridge_init();
    ped_groups_init();
    inform_friends_event_queue_init();
    inform_group_event_queue_init();
    loading_screen(0, 0, 0);
    GPtr::<u8>::from_addr(0xC6E97C).write(0);
    replay_init();
    shopping_init();
    loading_screen(0, 0, 0);
    1
}

/// `CGame::Init3(const char* datFile)` (0x5BA400, cdecl): a loading screen, pads 0 and 1 `Clear(true,
/// true)`, `SetUseD3DResourceBuffering(true)`, a loading screen, then `g_interiorMan.Init()`,
/// `g_procObjMan.Init()`, `g_waterCreatureMan.Init()`, `g_realTimeShadowMan.Init()`; returns true.
#[hook(addr = 0x5BA400, conv = cdecl, name = "CGame::Init3", expect = "6A 00 68 C8 A5 86 00",
       unverified = "mvpa-core-1", route = "L3-only", hazards = "ax-al-return", recs = "grove-walk",
       note = "one call per load; the datFile argument is unused")]
pub fn init3(_dat: GPtr<u8>) -> u8 {
    loading_screen(LS_TITLE, 0x86A5C8, 0);
    pad_clear(pad_get_pad(0), 1, 1);
    pad_clear(pad_get_pad(1), 1, 1);
    d3d_set_use_resource_buffering(1);
    loading_screen(LS_TITLE, 0x86A5B0, 0);
    interior_manager_init(0xBAF670);
    proc_object_man_init(0xBB7CB0);
    water_creature_manager_init(0xC1DF30);
    real_time_shadow_manager_init(0xC40350);
    1
}

/// `CGame::InitialiseCoreDataAfterRW()` (0x5BFA90, cdecl): `CTempColModels::Initialise`, the handling
/// data (`mod_HandlingManager`, 0xC2B9C8) loaded and its first five floats then set to 0.1, 0.9, 1.0,
/// 0.8, 0.98, `g_surfaceInfos.Init()` (0xB79538), ped stats, time cycle (`Initialise(false)`), pop cycle,
/// vehicle recordings, then `AudioEngine.Initialise()`, `g_breakMan.Init()` and `g_boneNodeMan.Init()`
/// (false when any fails). The result is the original's `neg al; sbb al, al; inc al` of
/// `g_ikChainMan.Init()`: 1 when it returned 0, else 0.
#[hook(addr = 0x5BFA90, conv = cdecl, name = "CGame::InitialiseCoreDataAfterRW", expect = "E8 2B AE FF FF",
       unverified = "mvpa-core-1", route = "L3-only", hazards = "ax-al-return", recs = "grove-walk",
       note = "one call per boot")]
pub fn initialise_core_data_after_rw() -> u8 {
    temp_col_models_initialise();
    handling_load_handling_data(0xC2B9C8);
    for (a, v) in [(0xC2B9C8usize, 0x3DCC_CCCDu32), (0xC2B9CC, 0x3F66_6666), (0xC2B9D0, 0x3F80_0000), (0xC2B9D4, 0x3F4C_CCCD), (0xC2B9D8, 0x3F7A_E148)] {
        GPtr::<u32>::from_addr(a).write(v);
    }
    surface_infos_init(0xB79538);
    ped_stats_initialise();
    time_cycle_initialise(0);
    pop_cycle_initialise();
    vehicle_recording_init_at_start_of_game();
    if audio_engine_initialise(0xB6BC90) == 0 || break_manager_init(0xBB4240) == 0 || bone_node_manager_init(0xC10820) == 0 {
        return 0;
    }
    (ik_chain_manager_init(0xC15448) == 0) as u8
}

/// Every registration of this module.
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_INITIALISE,
    HOOK_INIT1,
    HOOK_INIT2,
    HOOK_INIT3,
    HOOK_INITIALISE_CORE_DATA_AFTER_RW,
];
