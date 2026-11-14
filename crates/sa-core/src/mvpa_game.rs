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

use crate::bindings::col_accel_end_cache;
use crate::bindings::col_accel_start_cache;
use crate::bindings::d3d_set_use_resource_buffering;
use crate::bindings::ini_file_load_ini_file;
use crate::bindings::loading_screen;
use crate::bindings::placeable_init_matrix_array;
use crate::bindings::pools_initialise;
use crate::bindings::scripts_process;
use crate::bindings::scripts_start_test_script;
use crate::bindings::stencil_shadows_init;
use crate::bindings::txd_add_ref;
use crate::bindings::txd_create;
orig! {
    /// `CGame::Init1(const char*)` (0x5BF840)
    fn game_init1(dat: GPtr<u8>) -> u8 = cdecl 0x5BF840 => init1;
    /// `CFileLoader::LoadLevel(const char*)` (0x5B9030)
    fn file_loader_load_level(f: u32) = cdecl 0x5B9030 => unported;
    /// `CGame::Init2(const char*)` (0x5BA1A0)
    fn game_init2(dat: GPtr<u8>) -> u8 = cdecl 0x5BA1A0 => init2;
    /// `CCamera::Process()` (0x52B730)
    fn camera_process(c: u32) = thiscall 0x52B730 => unported;
    /// `CGame::Init3(const char*)` (0x5BA400)
    fn game_init3(dat: GPtr<u8>) -> u8 = cdecl 0x5BA400 => init3;
    /// `CMaths::InitMathsTables()` (0x59AC90)
    fn maths_init_maths_tables() = cdecl 0x59AC90 => unported;
    /// `CTxdStore::AddTxdSlot(const char*)` (0x731C80)
    fn txd_add_txd_slot(name: u32) -> i32 = cdecl 0x731C80 => unported;
    /// `CTxdStore::LoadTxd(int32, const char*)` (0x7320B0)
    fn txd_load_txd(i: i32, f: u32) = cdecl 0x7320B0 => unported;
}

use crate::bindings::collision_init;
use crate::bindings::conversations_clear;
use crate::bindings::cover_init;
use crate::bindings::cull_zones_init;
use crate::bindings::debug_init_text_buffer as fun_00532240;
use crate::bindings::game_logic_init_at_start_of_game;
use crate::bindings::gang_wars_init_at_start_of_game;
use crate::bindings::occlusion_init;
use crate::bindings::p2p_conversations_clear;
use crate::bindings::quad_tree_init_pool;
use crate::bindings::references_init;
use crate::bindings::set_current_txd as txd_set_current_txd;
use crate::bindings::set_pieces_init;
use crate::bindings::stunt_jump_manager_init;
use crate::bindings::tag_manager_init;
use crate::bindings::weather_init;
orig! {
    /// `CPlantMgr::Initialise()` (0x5DD910, AL)
    fn plant_mgr_initialise() -> u8 = cdecl 0x5DD910 => unported;
    /// `CCustomRoadsignMgr::Initialise()` (0x6FE120, AL)
    fn roadsign_mgr_initialise() -> u8 = cdecl 0x6FE120 => unported;
    /// `CBirds::Init()` (0x711EC0)
    fn birds_init() = cdecl 0x711EC0 => unported;
    /// `CEntryExitManager::Init()` (0x43F880)
    fn entry_exit_manager_init() = cdecl 0x43F880 => unported;
}

use crate::bindings::anim_manager_initialise;
use crate::bindings::audio_zones_init;
use crate::bindings::car_ctrl_init;
use crate::bindings::car_generators_init;
use crate::bindings::cutscene_mgr_initialise;
use crate::bindings::garages_init;
use crate::bindings::init_model_indices;
use crate::bindings::messages_clear_all_displayed_by_game;
use crate::bindings::messages_init;
use crate::bindings::model_info_initialise;
use crate::bindings::pickups_init;
use crate::bindings::restart_initialise;
use crate::bindings::streaming_init_image_list;
use crate::bindings::streaming_read_ini_file;
use crate::bindings::the_zones_init as zones_init;
use crate::bindings::user_display_init;
use crate::bindings::vehicle_recording_init;
use crate::bindings::world_initialise;
orig! {
    /// `CPathFind::Init()` (0x44D080)
    fn path_find_init(p: u32) = thiscall 0x44D080 => unported;
    /// 0x44D2B0 (`CPathFind` method, a bare `ret` in 1.0 US)
    fn path_find_fun_0044d2b0(p: u32) = thiscall 0x44D2B0 => unported;
}

use crate::bindings::anim_manager_load_anim_files;
use crate::bindings::car_plate_mgr_initialise;
use crate::bindings::cheat_reset_cheats;
use crate::bindings::clothes_init;
use crate::bindings::draw_set_fov;
use crate::bindings::entry_exit_post_creation;
use crate::bindings::fight_load_melee_data;
use crate::bindings::load_all_requested_models as streaming_load_all_requested_models;
use crate::bindings::ped_initialise;
use crate::bindings::player_info_clear;
use crate::bindings::renderer_init;
use crate::bindings::streaming_load_initial_peds;
use crate::bindings::streaming_load_initial_vehicles;
use crate::bindings::streaming_load_initial_weapons;
use crate::bindings::test_model_indices;
use crate::bindings::the_zones_post_zone_creation as zones_post_zone_creation;
use crate::bindings::water_level_initialise;
orig! {
    /// `Fx_c::Init()` (0x49EA90)
    fn fx_init(fx: u32) = thiscall 0x49EA90 => unported;
    /// 0x44D0E0 (`CPathFind` method, no catalogue name; `ThePaths.PreparePathData`)
    fn path_find_fun_0044d0e0(p: u32) = thiscall 0x44D0E0 => unported;
    /// `CPlayerInfo::LoadPlayerSkin()` (0x56F7D0)
    fn player_info_load_player_skin(p: u32) = thiscall 0x56F7D0 => unported;
}

use crate::bindings::clock_initialise;
use crate::bindings::coronas_init;
use crate::bindings::cranes_init_cranes;
use crate::bindings::darkel_init;
use crate::bindings::gangs_initialise;
use crate::bindings::garages_init_after_restart;
use crate::bindings::glass_init;
use crate::bindings::grid_ref_init;
use crate::bindings::heli_init_helis;
use crate::bindings::moving_things_init;
use crate::bindings::population_initialise;
use crate::bindings::radar_initialise;
use crate::bindings::radar_load_textures;
use crate::bindings::road_blocks_init;
use crate::bindings::scripts_init;
use crate::bindings::shadows_init;
use crate::bindings::skidmarks_init;
use crate::bindings::stats_init;
use crate::bindings::weapon_effects_init;
use crate::bindings::weapon_initialise_weapons;

use crate::bindings::bridge_init;
use crate::bindings::clouds_init;
use crate::bindings::inform_friends_event_queue_init;
use crate::bindings::inform_group_event_queue_init;
use crate::bindings::interior_manager_init;
use crate::bindings::ped_groups_init;
use crate::bindings::ped_stats_initialise;
use crate::bindings::proc_object_man_init;
use crate::bindings::real_time_shadow_manager_init;
use crate::bindings::replay_init;
use crate::bindings::ropes_init;
use crate::bindings::shopping_init;
use crate::bindings::special_fx_init;
use crate::bindings::temp_col_models_initialise;
use crate::bindings::water_cannons_init;
use crate::bindings::water_creature_manager_init;
orig! {
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(n: i32) -> u32 = cdecl 0x53FB70 => unported;
    /// `CPad::Clear(bool enablePlayerControls, bool resetPhase)` (0x541A70, `ret 8`)
    fn pad_clear(p: u32, a: u32, b: u32) = thiscall 0x541A70 => unported;
    /// `cHandlingDataMgr::LoadHandlingData()` (0x5BD830)
    fn handling_load_handling_data(m: u32) = thiscall 0x5BD830 => unported;
    /// `SurfaceInfos_c::Init()` (0x55F420)
    fn surface_infos_init(s: u32) = thiscall 0x55F420 => unported;
}

use crate::bindings::pop_cycle_initialise;
use crate::bindings::time_cycle_initialise;
use crate::bindings::vehicle_recording_init_at_start_of_game;
orig! {
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
