//! WRITER-MVP-A batch `mvpa-core-2`: the `CGame` restart orchestrators (`ShutDownForRestart`,
//! `InitialiseWhenRestarting`, `ReInitGameObjectVariables`), on the new-game / load path. Ported from
//! the 1.0 US bytes: the same callees in the same order with the same arguments, every callee through
//! `orig!` at its original address. Unverified (docs/verify/HOWTO.md).

use crate::mem::GPtr;
use crate::{hook, orig};

const THE_CAMERA: u32 = 0xB6F028;
const THE_PATHS: u32 = 0x96F050;
const AUDIO_ENGINE: u32 = 0xB6BC90;
const MENU: u32 = 0xBA6748;
/// `FrontEndMenuManager.m_bLoadingData` (+0x60)
const MENU_LOADING_DATA: usize = 0xBA67A8;

orig! {
    /// `D3DResourceSystem::SetUseD3DResourceBuffering(bool)` (0x730AC0)
    fn d3d_set_use_resource_buffering(b: u32) = cdecl 0x730AC0 => unported;
    /// `CVehicleRecording::ShutDown()` (0x459400)
    fn vehicle_recording_shut_down() = cdecl 0x459400 => unported;
    /// `CReplay::FinishPlayback()` (0x45F050)
    fn replay_finish_playback() = cdecl 0x45F050 => unported;
    /// `CReplay::EmptyReplayBuffer()` (0x45EC90)
    fn replay_empty_replay_buffer() = cdecl 0x45EC90 => unported;
    /// `CMovingThings::Shutdown()` (0x717C20)
    fn moving_things_shutdown() = cdecl 0x717C20 => unported;
    /// `CPlayerInfo::Clear()` (0x56F330)
    fn player_info_clear(p: u32) = thiscall 0x56F330 => unported;
    /// `CTheScripts::UndoBuildingSwaps()` (0x481290)
    fn scripts_undo_building_swaps() = cdecl 0x481290 => unported;
    /// `CTheScripts::UndoEntityInvisibilitySettings()` (0x4812D0)
    fn scripts_undo_entity_invisibility_settings() = cdecl 0x4812D0 => unported;
    /// `InteriorManager_c::Exit()` (0x598010)
    fn interior_manager_exit(m: u32) = thiscall 0x598010 => unported;
    /// `ProcObjectMan_c::Exit()` (0x5A3EE0)
    fn proc_object_man_exit(m: u32) = thiscall 0x5A3EE0 => unported;
    /// `WaterCreatureManager_c::Exit()` (0x6E3FD0)
    fn water_creature_manager_exit(m: u32) = thiscall 0x6E3FD0 => unported;
    /// `CRopes::Shutdown()` (0x556B10)
    fn ropes_shutdown() = cdecl 0x556B10 => unported;
    /// `CWorld::ClearForRestart()` (0x564360)
    fn world_clear_for_restart() = cdecl 0x564360 => unported;
    /// `CGameLogic::ClearSkip(bool)` (0x441560)
    fn game_logic_clear_skip(b: u32) = cdecl 0x441560 => unported;
    /// `CTimer::Shutdown()` (0x5618C0)
    fn timer_shutdown() = cdecl 0x5618C0 => unported;
    /// `CStreaming::ReInit()` (0x40E560)
    fn streaming_re_init() = cdecl 0x40E560 => unported;
    /// `CRadar::RemoveRadarSections()` (0x584BF0)
    fn radar_remove_radar_sections() = cdecl 0x584BF0 => unported;
    /// `CMenuManager::UnloadTextures()` (0x574630)
    fn menu_unload_textures(m: u32) = thiscall 0x574630 => unported;
    /// `CSetPieces::Init()` (0x4994F0)
    fn set_pieces_init() = cdecl 0x4994F0 => unported;
    /// `CConversations::Clear()` (0x43A7B0)
    fn conversations_clear() = cdecl 0x43A7B0 => unported;
    /// `CPedToPlayerConversations::Clear()` (0x43AAE0)
    fn p2p_conversations_clear() = cdecl 0x43AAE0 => unported;
    /// `CPedType::Shutdown()` (0x608B00)
    fn ped_type_shutdown() = cdecl 0x608B00 => unported;
    /// `CSpecialFX::Shutdown()` (0x723390)
    fn special_fx_shutdown() = cdecl 0x723390 => unported;
    /// `CFireManager::Shutdown()` (0x539DD0)
    fn fire_manager_shutdown(m: u32) = thiscall 0x539DD0 => unported;
    /// `Fx_c::Reset()` (0x49EAE0)
    fn fx_reset(f: u32) = thiscall 0x49EAE0 => unported;
    /// `BreakManager_c::ResetAll()` (0x59E720)
    fn break_manager_reset_all(m: u32) = thiscall 0x59E720 => unported;
    /// `BoneNodeManager_c::Reset()` (0x617430)
    fn bone_node_manager_reset(m: u32) = thiscall 0x617430 => unported;
    /// `IKChainManager_c::Reset()` (0x618140)
    fn ik_chain_manager_reset(m: u32) = thiscall 0x618140 => unported;
    /// `CRealTimeShadowManager::Exit()` (0x706A60)
    fn real_time_shadow_manager_exit(m: u32) = thiscall 0x706A60 => unported;
    /// `CTheZones::ResetZonesRevealed()` (0x572110)
    fn zones_reset_zones_revealed() = cdecl 0x572110 => unported;
    /// `CEntryExitManager::ShutdownForRestart()` (0x440C40)
    fn entry_exit_shutdown_for_restart() = cdecl 0x440C40 => unported;
    /// `CShopping::ShutdownForRestart()` (0x49B640)
    fn shopping_shutdown_for_restart() = cdecl 0x49B640 => unported;
    /// `CTagManager::ShutdownForRestart()` (0x49CC60)
    fn tag_manager_shutdown_for_restart() = cdecl 0x49CC60 => unported;
    /// `CStuntJumpManager::ShutdownForRestart()` (0x49CB10)
    fn stunt_jump_manager_shutdown_for_restart() = cdecl 0x49CB10 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn world_find_player_ped(n: i32) -> u32 = cdecl 0x56E210 => unported;
}

orig! {
    /// `CRGBA::CRGBA(uint8, uint8, uint8, uint8)` (0x7170C0, `ret 0x10`)
    fn rgba_ctor(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CTimer::Initialise()` (0x5617E0)
    fn timer_initialise() = cdecl 0x5617E0 => unported;
    /// `CMenuManager::MessageScreen(const char*, bool, bool)` (0x579330, `ret 0xC`)
    fn menu_message_screen(m: u32, key: u32, a: u32, b: u32) = thiscall 0x579330 => unported;
    /// `CGenericGameStorage::RestoreForStartLoad()` (0x619000)
    fn storage_restore_for_start_load() = cdecl 0x619000 => unported;
    /// `CStreaming::RemoveBigBuildings()` (0x4093B0)
    fn streaming_remove_big_buildings() = cdecl 0x4093B0 => unported;
    /// `CGame::ReInitGameObjectVariables()` (0x53BCF0)
    fn game_re_init_game_object_variables() = cdecl 0x53BCF0 => re_init_game_object_variables;
    /// `CTimeCycle::InitForRestart()` (0x5601F0)
    fn time_cycle_init_for_restart() = cdecl 0x5601F0 => unported;
    /// `CWeaponEffects::Init()` (0x742AB0)
    fn weapon_effects_init() = cdecl 0x742AB0 => unported;
    /// `CPlane::InitPlaneGenerationAndRemoval()` (0x6CAD90)
    fn plane_init_plane_generation_and_removal() = cdecl 0x6CAD90 => unported;
    /// `CGenericGameStorage::InitRadioStationPositionList()` (0x618E70)
    fn storage_init_radio_station_position_list() = cdecl 0x618E70 => unported;
    /// `CGenericGameStorage::GenericLoad(bool& outFailed)` (0x5D17B0, AL)
    fn storage_generic_load(out: GPtr<u8>) -> u8 = cdecl 0x5D17B0 => unported;
    /// `CTrain::InitTrains()` (0x6F7440)
    fn train_init_trains() = cdecl 0x6F7440 => unported;
    /// `Win::MessageLoop()` (0x746870)
    fn win_message_loop() = cdecl 0x746870 => unported;
    /// `CCamera::SetFadeColour(uint8, uint8, uint8)` (0x50BF00)
    fn camera_set_fade_colour(c: u32, r: u32, g: u32, b: u32) = thiscall 0x50BF00 => unported;
    /// `CGame::ShutDownForRestart()` (0x53C550)
    fn game_shut_down_for_restart() = cdecl 0x53C550 => shut_down_for_restart;
    /// `CTimer::Stop()` (0x561AA0)
    fn timer_stop() = cdecl 0x561AA0 => unported;
    /// `CCollision::SortOutCollisionAfterLoad()` (0x411E30)
    fn collision_sort_out_collision_after_load() = cdecl 0x411E30 => unported;
    /// `CTimer::Update()` (0x561B10)
    fn timer_update() = cdecl 0x561B10 => unported;
    /// `CAudioEngine::ResetSoundEffects()` (0x507C30)
    fn audio_reset_sound_effects(a: u32) = thiscall 0x507C30 => unported;
    /// `CAudioEngine::Restart()` (0x506DB0)
    fn audio_restart(a: u32) = thiscall 0x506DB0 => unported;
    /// `CAudioEngine::SetMusicMasterVolume(int8)` (0x506DE0)
    fn audio_set_music_master_volume(a: u32, v: u32) = thiscall 0x506DE0 => unported;
    /// `CAudioEngine::SetEffectsMasterVolume(int8)` (0x506E10)
    fn audio_set_effects_master_volume(a: u32, v: u32) = thiscall 0x506E10 => unported;
    /// `CAudioEngine::SetBassEnhanceOnOff(bool)` (0x506F90)
    fn audio_set_bass_enhance_on_off(a: u32, v: u32) = thiscall 0x506F90 => unported;
    /// `CAudioEngine::SetRadioAutoRetuneOnOff(bool)` (0x506F80)
    fn audio_set_radio_auto_retune_on_off(a: u32, v: u32) = thiscall 0x506F80 => unported;
    /// `CAudioEngine::InitialiseRadioStationID(int8)` (0x506FC0)
    fn audio_initialise_radio_station_id(a: u32, v: u32) = thiscall 0x506FC0 => unported;
}

orig! {
    /// `CCamera::Init()` (0x5BC520)
    fn camera_init(c: u32) = thiscall 0x5BC520 => unported;
    /// `CGameLogic::InitAtStartOfGame()` (0x441210)
    fn game_logic_init_at_start_of_game() = cdecl 0x441210 => unported;
    /// `CGangWars::InitAtStartOfGame()` (0x443920)
    fn gang_wars_init_at_start_of_game() = cdecl 0x443920 => unported;
    /// `CWanted::InitialiseStaticVariables()` (0x561C70)
    fn wanted_initialise_static_variables() = cdecl 0x561C70 => unported;
    /// `CCamera::SetRwCamera(RwCamera*)` (0x50C100)
    fn camera_set_rw_camera(c: u32, rw: u32) = thiscall 0x50C100 => unported;
    /// `CSkidmarks::Clear()` (0x720590)
    fn skidmarks_clear() = cdecl 0x720590 => unported;
    /// `CWeather::Init()` (0x72A480)
    fn weather_init() = cdecl 0x72A480 => unported;
    /// `CCover::Init()` (0x698710)
    fn cover_init() = cdecl 0x698710 => unported;
    /// `CUserDisplay::Init()` (0x571EE0)
    fn user_display_init() = cdecl 0x571EE0 => unported;
    /// `CMessages::Init()` (0x69EE00)
    fn messages_init() = cdecl 0x69EE00 => unported;
    /// `CRestart::Initialise()` (0x460630)
    fn restart_initialise() = cdecl 0x460630 => unported;
    /// `CPostEffects::ScriptResetForEffects()` (0x7010F0)
    fn post_effects_script_reset_for_effects() = cdecl 0x7010F0 => unported;
    /// `CHud::ReInitialise()` (0x588880)
    fn hud_re_initialise() = cdecl 0x588880 => unported;
    /// `CRadar::Initialise()` (0x587FB0)
    fn radar_initialise() = cdecl 0x587FB0 => unported;
    /// `CCarCtrl::ReInit()` (0x4213B0)
    fn car_ctrl_re_init() = cdecl 0x4213B0 => unported;
    /// `CPathFind::ReInit()` (0x44E4E0)
    fn path_find_re_init(p: u32) = thiscall 0x44E4E0 => unported;
    /// `CTimeCycle::Initialise(bool)` (0x5BBAC0)
    fn time_cycle_initialise(b: u32) = cdecl 0x5BBAC0 => unported;
    /// `CPopCycle::Initialise()` (0x5BC090)
    fn pop_cycle_initialise() = cdecl 0x5BC090 => unported;
    /// `CDraw::SetFOV(float)` (0x6FF410)
    fn draw_set_fov(fov: u32) = cdecl 0x6FF410 => unported;
    /// `CPed::Initialise()` (0x5DEBB0)
    fn ped_initialise() = cdecl 0x5DEBB0 => unported;
    /// `CWeapon::InitialiseWeapons()` (0x73A300)
    fn weapon_initialise_weapons() = cdecl 0x73A300 => unported;
    /// `CPopulation::Initialise()` (0x610E10)
    fn population_initialise() = cdecl 0x610E10 => unported;
    /// `CGlass::Init()` (0x71A8D0)
    fn glass_init() = cdecl 0x71A8D0 => unported;
    /// `CTheScripts::Init()` (0x468D50)
    fn scripts_init() = cdecl 0x468D50 => unported;
    /// `CMenuSystem::Initialise()` (0x5822D0)
    fn menu_system_initialise() = cdecl 0x5822D0 => unported;
    /// `CGangs::Initialise()` (0x5DE680)
    fn gangs_initialise() = cdecl 0x5DE680 => unported;
    /// `CClock::Initialise(uint32)` (0x52CD90)
    fn clock_initialise(scale: u32) = cdecl 0x52CD90 => unported;
    /// `CTheCarGenerators::Init()` (0x6F3270)
    fn car_generators_init() = cdecl 0x6F3270 => unported;
    /// `CHeli::InitHelis()` (0x6C4560)
    fn heli_init_helis() = cdecl 0x6C4560 => unported;
    /// `CMovingThings::Init()` (0x717D00)
    fn moving_things_init() = cdecl 0x717D00 => unported;
    /// `CDarkel::Init()` (0x43CEB0)
    fn darkel_init() = cdecl 0x43CEB0 => unported;
    /// `CStats::Init()` (0x55C0C0)
    fn stats_init() = cdecl 0x55C0C0 => unported;
    /// `CGarages::Init_AfterRestart()` (0x448B60)
    fn garages_init_after_restart() = cdecl 0x448B60 => unported;
    /// `CRoadBlocks::Init()` (0x461100)
    fn road_blocks_init() = cdecl 0x461100 => unported;
}

orig! {
    /// `CStreaming::DisableCopBikes(bool)` (0x407D10)
    fn streaming_disable_cop_bikes(b: u32) = cdecl 0x407D10 => unported;
    /// `CSpecialFX::Init()` (0x7268F0)
    fn special_fx_init() = cdecl 0x7268F0 => unported;
    /// `CRopes::Init()` (0x555DC0)
    fn ropes_init() = cdecl 0x555DC0 => unported;
    /// `CWaterCannons::Init()` (0x728C80)
    fn water_cannons_init() = cdecl 0x728C80 => unported;
    /// `CDecisionMakerTypesFileLoader::ReStart()` (0x607D00)
    fn decision_makers_restart() = cdecl 0x607D00 => unported;
    /// `CVehicleRecording::Init()` (0x459390)
    fn vehicle_recording_init() = cdecl 0x459390 => unported;
    /// `CFireManager::Init()` (0x538BC0)
    fn fire_manager_init(m: u32) = thiscall 0x538BC0 => unported;
    /// `InteriorManager_c::Init()` (0x5C0500)
    fn interior_manager_init(m: u32) -> u8 = thiscall 0x5C0500 => unported;
    /// `ProcObjectMan_c::Init()` (0x5A3EA0)
    fn proc_object_man_init(m: u32) -> u8 = thiscall 0x5A3EA0 => unported;
    /// `WaterCreatureManager_c::Init()` (0x6E3F90)
    fn water_creature_manager_init(m: u32) -> u8 = thiscall 0x6E3F90 => unported;
    /// `CRealTimeShadowManager::Init()` (0x7067C0)
    fn real_time_shadow_manager_init(m: u32) = thiscall 0x7067C0 => unported;
    /// `CStreaming::RemoveInappropriatePedModels()` (0x40B3A0)
    fn streaming_remove_inappropriate_ped_models() = cdecl 0x40B3A0 => unported;
    /// `CAudioEngine::ResetStatistics()` (0x506DA0)
    fn audio_reset_statistics(a: u32) = thiscall 0x506DA0 => unported;
    /// `CCranes::InitCranes()` (0x6F3FC0)
    fn cranes_init_cranes() = cdecl 0x6F3FC0 => unported;
    /// `CTheScripts::StartTestScript()` (0x464D40)
    fn scripts_start_test_script() = cdecl 0x464D40 => unported;
    /// `CTheScripts::Process()` (0x46A000)
    fn scripts_process() = cdecl 0x46A000 => unported;
    /// `CCamera::Process()` (0x52B730)
    fn camera_process(c: u32) = thiscall 0x52B730 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(n: i32) -> u32 = cdecl 0x53FB70 => unported;
    /// `CPad::Clear(bool, bool)` (0x541A70, `ret 8`)
    fn pad_clear(p: u32, a: u32, b: u32) = thiscall 0x541A70 => unported;
}

/// `CGame::ShutDownForRestart()` (0x53C550, cdecl): resource buffering off, vehicle recordings, replay,
/// moving things, both `CPlayerInfo`s cleared, `CTheScripts::...` 0x19 dwords at 0xBA3730 zeroed, script
/// swaps undone, the interior/proc-object/water-creature managers' `Exit`, ropes, `CWorld::ClearForRestart`,
/// `ClearSkip(false)`, timer, streaming re-init, radar sections, menu textures, set pieces, conversations,
/// ped types, special FX, fires, `g_fx.Reset()`, the break/bone/IK managers' resets, real-time shadows,
/// revealed zones, entry-exits/shopping/tags/stunt jumps, `FindPlayerPed(0)` (result unused) and byte
/// 0xC1CC18 = 0.
#[hook(addr = 0x53C550, conv = cdecl, name = "CGame::ShutDownForRestart", expect = "56 57 6A 00 E8 67 45 1F 00",
       unverified = "mvpa-core-2", route = "L3-only", hazards = "none", recs = "grove-walk",
       note = "new-game / load restart path")]
pub fn shut_down_for_restart() {
    d3d_set_use_resource_buffering(0);
    vehicle_recording_shut_down();
    replay_finish_playback();
    replay_empty_replay_buffer();
    moving_things_shutdown();
    player_info_clear(0xB7CD98);
    player_info_clear(0xB7CD98 + 0x190);
    for i in 0..0x19usize {
        GPtr::<u32>::from_addr(0xBA3730 + 4 * i).write(0);
    }
    scripts_undo_building_swaps();
    scripts_undo_entity_invisibility_settings();
    interior_manager_exit(0xBAF670);
    proc_object_man_exit(0xBB7CB0);
    water_creature_manager_exit(0xC1DF30);
    ropes_shutdown();
    world_clear_for_restart();
    game_logic_clear_skip(0);
    timer_shutdown();
    streaming_re_init();
    radar_remove_radar_sections();
    menu_unload_textures(MENU);
    set_pieces_init();
    conversations_clear();
    p2p_conversations_clear();
    ped_type_shutdown();
    special_fx_shutdown();
    fire_manager_shutdown(0xB71F80);
    fx_reset(0xA9AE00);
    break_manager_reset_all(0xBB4240);
    bone_node_manager_reset(0xC10820);
    ik_chain_manager_reset(0xC15448);
    real_time_shadow_manager_exit(0xC40350);
    zones_reset_zones_revealed();
    entry_exit_shutdown_for_restart();
    shopping_shutdown_for_restart();
    tag_manager_shutdown_for_restart();
    stunt_jump_manager_shutdown_for_restart();
    world_find_player_ped(0);
    GPtr::<u8>::from_addr(0xC1CC18).write(0);
}

/// `CGame::InitialiseWhenRestarting()` (0x53C680, cdecl). A `CRGBA(255, 255, 255, 255)` is built in a
/// local and not used; `CTimer::Initialise`; `gbLARiots`... dword 0xC0B038 = 0. When the menu is loading
/// a save (`m_bLoadingData`): the "loading" message screen (0x863AB4) and, still loading, the start-load
/// restore and `RemoveBigBuildings`. Then `ReInitGameObjectVariables`, time cycle, weapon effects, plane
/// generation. When loading: the flag is cleared, the radio position list initialised and
/// `GenericLoad(&failed)` (the flag byte is a local the callee writes); success → `CTrain::InitTrains`;
/// failure → 50 rounds of `Win::MessageLoop` + message screen (0x863AAC when the local byte is set, else
/// 0x863AA4), black fade colour, `ShutDownForRestart`, timer stop/initialise, flag cleared again,
/// `ReInitGameObjectVariables`, `currLevel` (0xB7290C) = 0, byte 0xB72910 = 0 and
/// `SortOutCollisionAfterLoad`. Finally `CTimer::Update`, the audio engine's restart and settings from
/// the menu bytes 0xBA6798 / 0xBA6797 / 0xBA6799 / 0xBA6795 / 0xBA679A, and resource buffering on.
#[hook(addr = 0x53C680, conv = cdecl, name = "CGame::InitialiseWhenRestarting", expect = "83 EC 08 53 68 FF 00 00 00",
       unverified = "mvpa-core-2", route = "L3-only", hazards = "uninit-locals", recs = "grove-walk",
       note = "new-game / load restart path; GenericLoad's out byte starts uninitialised in the original")]
pub fn initialise_when_restarting() {
    let mut frame = [0u8; 8];
    let fp = GPtr::<u8>::from_raw(frame.as_mut_ptr());
    rgba_ctor(fp.byte_add(4), 0xFF, 0xFF, 0xFF, 0xFF);
    timer_initialise();
    let loading = GPtr::<u8>::from_addr(MENU_LOADING_DATA);
    GPtr::<u32>::from_addr(0xC0B038).write(0);
    if loading.read() != 0 {
        menu_message_screen(MENU, 0x863AB4, 1, 0);
        if loading.read() != 0 {
            storage_restore_for_start_load();
            streaming_remove_big_buildings();
        }
    }
    game_re_init_game_object_variables();
    time_cycle_init_for_restart();
    weapon_effects_init();
    plane_init_plane_generation_and_removal();
    if loading.read() != 0 {
        loading.write(0);
        storage_init_radio_station_position_list();
        if storage_generic_load(fp.byte_add(3)) != 0 {
            train_init_trains();
        } else {
            for _ in 0..50 {
                win_message_loop();
                let key = if fp.byte_add(3).read() != 0 { 0x863AAC } else { 0x863AA4 };
                menu_message_screen(MENU, key, 1, 0);
            }
            camera_set_fade_colour(THE_CAMERA, 0, 0, 0);
            game_shut_down_for_restart();
            timer_stop();
            timer_initialise();
            loading.write(0);
            game_re_init_game_object_variables();
            GPtr::<u32>::from_addr(0xB7290C).write(0);
            GPtr::<u8>::from_addr(0xB72910).write(0);
            collision_sort_out_collision_after_load();
        }
    }
    timer_update();
    audio_reset_sound_effects(AUDIO_ENGINE);
    audio_restart(AUDIO_ENGINE);
    let b = |a: usize| u32::from(GPtr::<u8>::from_addr(a).read());
    audio_set_music_master_volume(AUDIO_ENGINE, b(0xBA6798));
    audio_set_effects_master_volume(AUDIO_ENGINE, b(0xBA6797));
    audio_set_bass_enhance_on_off(AUDIO_ENGINE, b(0xBA6799));
    audio_set_radio_auto_retune_on_off(AUDIO_ENGINE, b(0xBA6795));
    audio_initialise_radio_station_id(AUDIO_ENGINE, b(0xBA679A));
    d3d_set_use_resource_buffering(1);
}

/// `CGame::ReInitGameObjectVariables()` (0x53BCF0, cdecl): camera init (twice, around game logic, gang
/// wars and wanted statics), `SetRwCamera(Scene.m_pRwCamera` 0xC1703C`)`, skidmarks, weather, cover, user
/// display, messages, restart points, post effects, `CWorld::bDoingCarCollisions`... byte 0xB7CD73 = 0,
/// HUD, radar, car control, paths, time cycle (`Initialise(false)`), pop cycle, FOV 70, far clip 500,
/// `currArea` = 0, peds, weapons, population, both `CPlayerInfo`s cleared, bytes 0xB7CD74, 0xB72959,
/// 0xB72958 = 0 (glass init in between), scripts, menu system, gangs, timer, clock (1000), car
/// generators, helis, moving things, darkel, stats, garages, road blocks, `DisableCopBikes(false)`,
/// special FX, ropes, water cannons, decision makers, vehicle recordings, fires, the four managers'
/// `Init`, inappropriate ped models, audio statistics; when not loading a save: cranes, the test script,
/// `CTheScripts::Process`, `TheCamera.Process()`, trains; finally pads 0 and 1 `Clear(true, true)`.
#[hook(addr = 0x53BCF0, conv = cdecl, name = "CGame::ReInitGameObjectVariables", expect = "53 56 57 B9 28 F0 B6 00",
       unverified = "mvpa-core-2", route = "L3-only", hazards = "none", recs = "grove-walk",
       note = "new-game / load restart path")]
pub fn re_init_game_object_variables() {
    camera_init(THE_CAMERA);
    game_logic_init_at_start_of_game();
    gang_wars_init_at_start_of_game();
    wanted_initialise_static_variables();
    camera_init(THE_CAMERA);
    camera_set_rw_camera(THE_CAMERA, GPtr::<u32>::from_addr(0xC1703C).read());
    skidmarks_clear();
    weather_init();
    cover_init();
    user_display_init();
    messages_init();
    restart_initialise();
    post_effects_script_reset_for_effects();
    GPtr::<u8>::from_addr(0xB7CD73).write(0);
    hud_re_initialise();
    radar_initialise();
    car_ctrl_re_init();
    path_find_re_init(THE_PATHS);
    time_cycle_initialise(0);
    pop_cycle_initialise();
    draw_set_fov(0x42F0_0000);
    GPtr::<u32>::from_addr(0xC3EF98).write(0x43FA_0000);
    GPtr::<u32>::from_addr(0xB72914).write(0);
    ped_initialise();
    weapon_initialise_weapons();
    population_initialise();
    player_info_clear(0xB7CD98);
    player_info_clear(0xB7CD98 + 0x190);
    GPtr::<u8>::from_addr(0xB7CD74).write(0);
    glass_init();
    GPtr::<u8>::from_addr(0xB72959).write(0);
    GPtr::<u8>::from_addr(0xB72958).write(0);
    scripts_init();
    menu_system_initialise();
    gangs_initialise();
    timer_initialise();
    clock_initialise(1000);
    car_generators_init();
    heli_init_helis();
    moving_things_init();
    darkel_init();
    stats_init();
    garages_init_after_restart();
    road_blocks_init();
    streaming_disable_cop_bikes(0);
    special_fx_init();
    ropes_init();
    water_cannons_init();
    decision_makers_restart();
    vehicle_recording_init();
    fire_manager_init(0xB71F80);
    interior_manager_init(0xBAF670);
    proc_object_man_init(0xBB7CB0);
    water_creature_manager_init(0xC1DF30);
    real_time_shadow_manager_init(0xC40350);
    streaming_remove_inappropriate_ped_models();
    audio_reset_statistics(AUDIO_ENGINE);
    if GPtr::<u8>::from_addr(MENU_LOADING_DATA).read() == 0 {
        cranes_init_cranes();
        scripts_start_test_script();
        scripts_process();
        camera_process(THE_CAMERA);
        train_init_trains();
    }
    pad_clear(pad_get_pad(0), 1, 1);
    pad_clear(pad_get_pad(1), 1, 1);
}

/// Every registration of this module.
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_SHUT_DOWN_FOR_RESTART,
    HOOK_INITIALISE_WHEN_RESTARTING,
    HOOK_RE_INIT_GAME_OBJECT_VARIABLES,
];
