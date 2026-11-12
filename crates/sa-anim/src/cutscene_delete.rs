//! `CCutsceneMgr::DeleteCutsceneData_overlay` (batch `mvpn-anim-1`): tears a loaded cutscene down and gives the world
//! back to the player. Ported from the 1.0 US bytes (gta-reversed `CutsceneMgr.cpp` as a reading aid; the bytes win:
//! the pad is cleared after the audio and memory tidy-up, the weapons are restored per entry with their models
//! streamed in). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

orig! {
    /// `CTimer::Suspend()` (0x5619D0)
    fn timer_suspend() = cdecl 0x5619D0 => unported;
    /// `CTimer::Resume()` (0x561A00)
    fn timer_resume() = cdecl 0x561A00 => unported;
    /// `CTimeCycle::StartExtraColour(int32, bool)` (0x55FEC0)
    fn time_cycle_start_extra_colour(c: u32, b: u32) = cdecl 0x55FEC0 => unported;
    /// `CTimeCycle::StopExtraColour(bool)` (0x55FF20)
    fn time_cycle_stop_extra_colour(b: u32) = cdecl 0x55FF20 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `FxManager_c::DestroyFxSystem(FxSystem_c*)` (0x4A9810, `ret 4`)
    fn fx_manager_destroy_fx_system(m: GPtr<u8>, s: GPtr<u8>) = thiscall 0x4A9810 => unported;
    /// `CMessages::ClearMessages(bool)` (0x69DCD0)
    fn messages_clear_messages(b: u32) = cdecl 0x69DCD0 => unported;
    /// `CRubbish::SetVisibility(bool)` (0x7204C0)
    fn rubbish_set_visibility(b: u32) = cdecl 0x7204C0 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CEntity::DeleteRwObject()` (vtable +0x20)
    fn v_delete_rw_object(e: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// scalar deleting destructor (vtable +0, `ret 4`)
    fn v_scalar_deleting_dtor(e: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CAnimManager::RemoveLastAnimFile()` (0x4D3ED0)
    fn anim_manager_remove_last_anim_file() = cdecl 0x4D3ED0 => unported;
    /// `CAnimBlendAssocGroup::DestroyAssociations()` (0x4CDFF0)
    fn assoc_group_destroy_associations(g: GPtr<u8>) = thiscall 0x4CDFF0 => unported;
    /// `CCamera::RestoreWithJumpCut()` (0x50BAB0)
    fn camera_restore_with_jump_cut(c: GPtr<u8>) = thiscall 0x50BAB0 => unported;
    /// `CCamera::SetWideScreenOff()` (0x50C150)
    fn camera_set_wide_screen_off(c: GPtr<u8>) = thiscall 0x50C150 => unported;
    /// `CCamera::DeleteCutSceneCamDataMemory()` (0x5B24A0)
    fn camera_delete_cut_scene_cam_data_memory(c: GPtr<u8>) = thiscall 0x5B24A0 => unported;
    /// `CIplStore::ClearIplsNeededAtPosn()` (0x4045E0)
    fn ipl_store_clear_ipls_needed_at_posn() = cdecl 0x4045E0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::Clear(bool, bool)` (0x541A70, `ret 8`)
    fn pad_clear(pad: GPtr<u8>, a: u32, b: u32) = thiscall 0x541A70 => unported;
    /// `CPlayerInfo::MakePlayerSafe(bool, float)` (0x56E870, `ret 8`)
    fn player_info_make_player_safe(pi: GPtr<u8>, safe: u32, radius: f32) = thiscall 0x56E870 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn crt_stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `CAudioEngine::StopCutsceneTrack(bool)` (0x507080, `ret 4`)
    fn audio_stop_cutscene_track(e: GPtr<u8>, b: u32) = thiscall 0x507080 => unported;
    /// `CAudioEngine::EnableEffectsLoading()` (0x506E90)
    fn audio_enable_effects_loading(e: GPtr<u8>) = thiscall 0x506E90 => unported;
    /// `CAEPedSpeechAudioEntity::EnableAllPedSpeech()` (0x4E3EC0)
    fn ped_speech_enable_all() = cdecl 0x4E3EC0 => unported;
    /// `CCamera::GetScreenFadeStatus()` (0x50AE20)
    fn camera_get_screen_fade_status(c: GPtr<u8>) -> i32 = thiscall 0x50AE20 => unported;
    /// `CGame::DrasticTidyUpMemory(bool)` (0x53C810)
    fn game_drastic_tidy_up_memory(b: u32) = cdecl 0x53C810 => unported;
    /// `CStreaming::LoadInitialPeds()` (0x40D3D0)
    fn streaming_load_initial_peds() = cdecl 0x40D3D0 => unported;
    /// `CStreaming::LoadInitialWeapons()` (0x40A120)
    fn streaming_load_initial_weapons() = cdecl 0x40A120 => unported;
    /// `CStreaming::LoadInitialVehicles()` (0x407F20)
    fn streaming_load_initial_vehicles() = cdecl 0x407F20 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn weapon_info_get(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: u32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    fn streaming_load_all_requested_models(b: u32) = cdecl 0x40EA10 => unported;
    /// `CStreaming::SetModelIsDeletable(int32)` (0x409C10)
    fn streaming_set_model_is_deletable(id: i32) = cdecl 0x409C10 => unported;
    /// `CPed::GiveWeapon(eWeaponType, uint32 ammo, bool)` (0x5E6080, `ret 0xC`)
    fn ped_give_weapon(ped: GPtr<u8>, ty: u32, ammo: u32, b: u32) -> u32 = thiscall 0x5E6080 => unported;
    /// `CScriptResourceManager::HasResourceBeenRequested(int32, int32)` (0x470620, `ret 8`), AL
    fn script_resource_has_been_requested(m: GPtr<u8>, id: i32, ty: u32) -> u8 = thiscall 0x470620 => unported;
    /// `CStreaming::SetMissionDoesntRequireModel(int32)` (0x409C90)
    fn streaming_set_mission_doesnt_require_model(id: i32) = cdecl 0x409C90 => unported;
    /// `FindPlayerCoors(CVector* out, int32)` (0x56E010)
    fn find_player_coors(out: GPtr<u8>, id: i32) -> GPtr<u8> = cdecl 0x56E010 => unported;
    /// `CStreaming::StreamZoneModels(const CVector&)` (0x40A560)
    fn streaming_stream_zone_models(p: GPtr<u8>) = cdecl 0x40A560 => unported;
    /// `CStreaming::ForceLayerToRead(int32)` (0x407A10)
    fn streaming_force_layer_to_read(layer: i32) = cdecl 0x407A10 => unported;
}

/// `CCutsceneMgr` state: load status (0xB5F84C), running (u8 0xB5F851), processing / LOD multiplier (u8 0xB5F852 /
/// 0xB5F853), anim loaded (u8 0xB5F850), data file loaded (u8 0xBC4006), restore everything (u8 0xBC4005), name
/// (0xBC3F0C), the finale's name (0x8689B4)
const LOAD_STATUS: usize = 0xB5F84C;
const RUNNING: usize = 0xB5F851;
const PROCESSING: usize = 0xB5F852;
const USE_LOD_MULT: usize = 0xB5F853;
const ANIM_LOADED: usize = 0xB5F850;
const DATA_FILE_LOADED: usize = 0xBC4006;
const RESTORE_EVERYTHING: usize = 0xBC4005;
const NAME: usize = 0xBC3F0C;
const FINALE: usize = 0x8689B4;
/// Saved densities (0xBC1D6C ped, 0xBC1D68 car) and their targets `CPopulation::PedDensityMultiplier` (0x8D2530),
/// `CCarCtrl::CarDensityMultiplier` (0x8A5B20); saved extra colour on (u8 0xBC4004) and index (0xBC4000)
const PREV_PED_DENSITY: usize = 0xBC1D6C;
const PREV_CAR_DENSITY: usize = 0xBC1D68;
const PED_DENSITY: usize = 0x8D2530;
const CAR_DENSITY: usize = 0x8A5B20;
const PREV_EXTRA_ON: usize = 0xBC4004;
const PREV_EXTRA: usize = 0xBC4000;
/// Hidden entities: count (0xBC3FF8), array (0xBC2968); particle effects: count (0xBC3FFC), array (0xBC1D90, 0x6C
/// each); cutscene objects: count (0xBC3FE4), array (0xBC3F18)
const HIDDEN_COUNT: usize = 0xBC3FF8;
const HIDDEN: usize = 0xBC2968;
const FX_COUNT: usize = 0xBC3FFC;
const FX: usize = 0xBC1D90;
const OBJ_COUNT: usize = 0xBC3FE4;
const OBJS: usize = 0xBC3F18;
/// `CWorld::bProcessCutsceneOnly`-style flag set on teardown (u8 0x8AC158), the first uncompressed anim name byte
/// (0xBC2CC0) and their count (0xBC3FF4)
const AFTER_CUTSCENE_FLAG: usize = 0x8AC158;
const UNCOMPRESSED_ANIMS: usize = 0xBC2CC0;
const UNCOMPRESSED_COUNT: usize = 0xBC3FF4;
/// The cutscene association group (0xBC4020), `g_fxMan` (0xA9AE80), `TheCamera` (0xB6F028), `AudioEngine` (0xB6BC90)
const ASSOCS: usize = 0xBC4020;
const FX_MAN: usize = 0xA9AE80;
const THE_CAMERA: usize = 0xB6F028;
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `CWorld::PlayerInFocus` (u8 0xB7CD74), `CWorld::Players` (0xB7CD98, 0x190 each)
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
const PLAYERS: usize = 0xB7CD98;
/// `CStreaming::ms_disableStreaming` (u8 0x9654B0), `CWorld::bProcessCutsceneOnly` (u8 0xB7CD6D)
const DISABLE_STREAMING: usize = 0x9654B0;
const PROCESS_CUTSCENE_ONLY: usize = 0xB7CD6D;
/// Weapons taken from the player: count (0xBC1CFC), types (0xBC1D34), ammo (0xBC1D00)
const WEAPON_COUNT: usize = 0xBC1CFC;
const WEAPON_TYPES: usize = 0xBC1D34;
const WEAPON_AMMO: usize = 0xBC1D00;
/// `CStreaming::ms_aInfoForModel[].m_nFlags` (0x8E4CC6, 0x14 each)
const MODEL_INFO_FLAGS: usize = 0x8E4CC6;
/// Models the cutscene requested: count (0xBC3FE8), ids (0xBC31C0); `CTheScripts::ScriptResourceManager` (0xA485A8)
const MODEL_COUNT: usize = 0xBC3FE8;
const MODELS: usize = 0xBC31C0;
const SCRIPT_RESOURCES: usize = 0xA485A8;

fn gu32(a: usize) -> GPtr<u32> {
    g::<u32>(a)
}
fn gu8(a: usize) -> GPtr<u8> {
    g::<u8>(a)
}
fn p(a: usize) -> GPtr<u8> {
    GPtr::from_addr(a)
}

/// `CCutsceneMgr::DeleteCutsceneData_overlay()` (0x5AFD60, cdecl).
///
/// Nothing when the load status is 0. Else, between `CTimer::Suspend` / `Resume`: the saved ped / car densities come
/// back, the saved extra colour restarts (`StartExtraColour(c, false)`) or `StopExtraColour(false)`; every hidden
/// entity still referenced gets `CleanUpOldReference(&slot)` and its visible bit (+0x1C bit 7) set; every particle
/// system is destroyed (`g_fxMan.DestroyFxSystem`) and its slot cleared (counts re-read per entry, then 0);
/// `ClearMessages(false)`, `CRubbish::SetVisibility(true)`, processing and LOD flags 0, 0x8AC158 = 1. The cutscene
/// objects go last to first: `CWorld::Remove`, `DeleteRwObject`, deleting destructor (non-null), slot 0 (until the
/// count is 0). The last anim file is removed when loaded, the cutscene associations destroyed, the uncompressed anim
/// list emptied; with a data file the camera restores with a jump cut, wide screen off, its cutscene data freed.
/// `ClearIplsNeededAtPosn`, status 0, not running; the player ped becomes visible, pad 0 loses its "safe for
/// cutscene" bit (+0x10E bit 7), `MakePlayerSafe(false, 10000)`. Outside the finale the cutscene track stops, effects
/// loading and ped speech come back. Streaming and the world run normally again; with a data file
/// `DrasticTidyUpMemory(fade status == 2)`. Pad 0 is cleared (`Clear(false, false)`). When everything must be
/// restored the initial peds / weapons / vehicles load and each saved weapon is given back (its model and second model
/// requested and loaded; the first made deletable unless its streaming flags had bit 1), count 0. Models the scripts
/// did not request stop being mission models, then model 1; the zone models around the player stream and layer 1 is
/// forced.
#[hook(addr = 0x5afd60, conv = cdecl, name = "CCutsceneMgr::DeleteCutsceneData_overlay", expect = "A1 4C F8 B5 00",
       unverified = "mvpn-anim-1", route = "L3-only", hazards = "heap-writes",
       note = "once per cutscene end; streams models, frees the cutscene objects and camera data")]
pub fn cutscene_delete_cutscene_data_overlay() {
    if gu32(LOAD_STATUS).read() == 0 {
        return;
    }
    timer_suspend();
    gu32(PED_DENSITY).write(gu32(PREV_PED_DENSITY).read());
    gu32(CAR_DENSITY).write(gu32(PREV_CAR_DENSITY).read());
    if gu8(PREV_EXTRA_ON).read() != 0 {
        time_cycle_start_extra_colour(gu32(PREV_EXTRA).read(), 0);
    } else {
        time_cycle_stop_extra_colour(0);
    }
    let mut i = 0usize;
    while (i as i32) < gu32(HIDDEN_COUNT).read() as i32 {
        let slot = p(HIDDEN + 4 * i);
        let e = rd::<GPtr<u8>>(slot, 0);
        if !e.is_null() {
            entity_clean_up_old_reference(e, slot);
            let e = rd::<GPtr<u8>>(slot, 0);
            wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) | 0x80);
        }
        i += 1;
    }
    gu32(HIDDEN_COUNT).write(0);
    let mut i = 0usize;
    while (i as i32) < gu32(FX_COUNT).read() as i32 {
        let slot = p(FX + 0x6C * i);
        let s = rd::<GPtr<u8>>(slot, 0);
        if !s.is_null() {
            fx_manager_destroy_fx_system(p(FX_MAN), s);
        }
        wr::<u32>(slot, 0, 0);
        i += 1;
    }
    gu32(FX_COUNT).write(0);
    messages_clear_messages(0);
    rubbish_set_visibility(1);
    gu8(PROCESSING).write(0);
    gu8(USE_LOD_MULT).write(0);
    gu8(AFTER_CUTSCENE_FLAG).write(1);
    let count = gu32(OBJ_COUNT);
    if count.read() != 0 {
        loop {
            let n = count.read().wrapping_sub(1);
            count.write(n);
            world_remove(rd::<GPtr<u8>>(p(OBJS), 4 * n as i32 as isize));
            v_delete_rw_object(rd::<GPtr<u8>>(p(OBJS), 4 * count.read() as i32 as isize));
            let k = count.read();
            let o = rd::<GPtr<u8>>(p(OBJS), 4 * k as i32 as isize);
            let k = if !o.is_null() {
                v_scalar_deleting_dtor(o, 1);
                count.read()
            } else {
                k
            };
            wr::<u32>(p(OBJS), 4 * k as i32 as isize, 0);
            if k == 0 {
                break;
            }
        }
    }
    count.write(0);
    if gu8(ANIM_LOADED).read() != 0 {
        anim_manager_remove_last_anim_file();
    }
    gu8(ANIM_LOADED).write(0);
    assoc_group_destroy_associations(p(ASSOCS));
    gu8(UNCOMPRESSED_ANIMS).write(0);
    gu32(UNCOMPRESSED_COUNT).write(0);
    let cam = p(THE_CAMERA);
    if gu8(DATA_FILE_LOADED).read() != 0 {
        camera_restore_with_jump_cut(cam);
        camera_set_wide_screen_off(cam);
        camera_delete_cut_scene_cam_data_memory(cam);
    }
    ipl_store_clear_ipls_needed_at_posn();
    gu32(LOAD_STATUS).write(0);
    gu8(RUNNING).write(0);
    let player = find_player_ped(-1);
    let info = p(PLAYERS + 0x190 * gu8(PLAYER_IN_FOCUS).read() as usize);
    wr::<u32>(player, 0x1C, rd::<u32>(player, 0x1C) | 0x80);
    let pad = pad_get_pad(0);
    wr::<u8>(pad, 0x10E, rd::<u8>(pad, 0x10E) & 0x7F);
    player_info_make_player_safe(info, 0, 10000.0);
    if crt_stricmp(p(NAME), p(FINALE)) != 0 {
        let a = p(AUDIO_ENGINE);
        audio_stop_cutscene_track(a, 1);
        audio_enable_effects_loading(a);
        ped_speech_enable_all();
    }
    gu8(DISABLE_STREAMING).write(0);
    gu8(PROCESS_CUTSCENE_ONLY).write(0);
    if gu8(DATA_FILE_LOADED).read() != 0 {
        game_drastic_tidy_up_memory((camera_get_screen_fade_status(cam) == 2) as u32);
    }
    pad_clear(pad_get_pad(0), 0, 0);
    if gu8(RESTORE_EVERYTHING).read() != 0 {
        streaming_load_initial_peds();
        streaming_load_initial_weapons();
        streaming_load_initial_vehicles();
        gu8(RESTORE_EVERYTHING).write(0);
        let ped = find_player_ped(-1);
        let mut i = 0usize;
        while (i as i32) < gu32(WEAPON_COUNT).read() as i32 {
            let ty = gu32(WEAPON_TYPES + 4 * i).read();
            let model = rd::<i32>(weapon_info_get(ty, 1), 0xC);
            let flags = gu8(MODEL_INFO_FLAGS + 0x14 * model as usize).read();
            streaming_request_model(model, 2);
            streaming_load_all_requested_models(0);
            if rd::<i32>(weapon_info_get(gu32(WEAPON_TYPES + 4 * i).read(), 1), 0x10) != -1 {
                let m2 = rd::<i32>(weapon_info_get(gu32(WEAPON_TYPES + 4 * i).read(), 1), 0x10);
                streaming_request_model(m2, 0);
                streaming_load_all_requested_models(0);
            }
            if flags & 2 == 0 {
                streaming_set_model_is_deletable(model);
            }
            ped_give_weapon(ped, gu32(WEAPON_TYPES + 4 * i).read(), gu32(WEAPON_AMMO + 4 * i).read(), 1);
            i += 1;
        }
        gu32(WEAPON_COUNT).write(0);
    }
    let mut i = 0usize;
    while (i as i32) < gu32(MODEL_COUNT).read() as i32 {
        let id = g::<i32>(MODELS + 4 * i).read();
        if script_resource_has_been_requested(p(SCRIPT_RESOURCES), id, 2) == 0 {
            streaming_set_mission_doesnt_require_model(g::<i32>(MODELS + 4 * i).read());
        }
        i += 1;
    }
    streaming_set_mission_doesnt_require_model(1);
    let mut pos = [0u32; 3];
    let at = find_player_coors(GPtr::from_raw(pos.as_mut_ptr()).cast(), -1);
    streaming_stream_zone_models(at);
    timer_resume();
    streaming_force_layer_to_read(1);
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CUTSCENE_DELETE_CUTSCENE_DATA_OVERLAY];
