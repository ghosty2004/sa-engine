//! `CAudioEngine` (PORT-AUDIO): the engine facade (`AudioEngine` 0xB6BC90, 0x1FD8 bytes; +0xB4 the
//! front-end entity, +0x2A0 the script entity, +0x4BC the collision entity, +0x1E34 the global weapon
//! entity pointer, +0x1E38 the pedless speech entity, +0x1F50 the door entity). Ported from the 1.0 US
//! bytes; gta-reversed `AudioEngine.cpp` and `docs/re/specs/AUDIO/CAudioEngine.deviations.md` as reading
//! aids.
//!
//! Most members are one `mov ecx, <manager>` / `add ecx, <member>` and a tail `jmp` into the manager or
//! entity. The ports call the target through `orig!` with the same `this` and pass every stack word
//! through unchanged as `u32` (bit-exact also for float and byte arguments, whose upper bytes the
//! original forwards as they are); a non-void target's EAX is returned whole, as the tail jump does.
//! The real C++ parameter types are in each target's doc comment.

#![allow(clippy::too_many_arguments)]

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{
    AE_AMBIENCE_TRACK_MANAGER, AE_AUDIO_HARDWARE, AE_CUTSCENE_TRACK_MANAGER, AE_RADIO_TRACK_MANAGER,
    AE_SOUND_MANAGER, f32_of, g, rd, wr, xi,
};

/// No globals of its own (`AudioEngine` itself is reached by address).
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// `CAESoundManager::PauseManually(bool)` (0x4EF510)
    pub fn sound_manager_pause_manually(this: GPtr<u8>, pause: u32) = thiscall 0x4EF510 => unported;
    /// `CAESoundManager::Service()` (0x4F0000)
    pub fn sound_manager_service(this: GPtr<u8>) = thiscall 0x4F0000 => unported;
    /// `CAEAudioHardware::SetMusicMasterScalingFactor(float)` (0x4D9530)
    pub fn hw_set_music_master_scaling_factor(this: GPtr<u8>, f: f32) = thiscall 0x4D9530 => unported;
    /// `CAEAudioHardware::SetEffectsMasterScalingFactor(float)` (0x4D9540)
    pub fn hw_set_effects_master_scaling_factor(this: GPtr<u8>, f: f32) = thiscall 0x4D9540 => unported;
    /// `CAEAudioHardware::GetBeatInfo(tBeatInfo*)` (0x4D8FA0)
    pub fn hw_get_beat_info(this: GPtr<u8>, out: GPtr<u8>) = thiscall 0x4D8FA0 => unported;
    /// `CAEAudioHardware::GetTrackPlayTime()` (0x4D8F60)
    pub fn hw_get_track_play_time(this: GPtr<u8>) -> u32 = thiscall 0x4D8F60 => unported;
    /// `CAEAudioHardware::Service()` (0x4D9870; hooked by sa-hook's determinism mode)
    pub fn hw_service(this: GPtr<u8>) = thiscall 0x4D9870 => unported;
    /// `CAECollisionAudioEntity::ReportGlassCollisionEvent(eAudioEvents, const CVector&, uint32)` (0x4DA070)
    pub fn coll_report_glass_collision_event(this: GPtr<u8>, event: u32, pos: u32, time: u32) = thiscall 0x4DA070 => unported;
    /// `CAECollisionAudioEntity::ReportWaterSplash(CVector posn, float volume)` (0x4DA190)
    pub fn coll_report_water_splash_at(this: GPtr<u8>, x: u32, y: u32, z: u32, volume: u32) = thiscall 0x4DA190 => unported;
    /// `CAEDoorAudioEntity::AddAudioEvent(eAudioEvents, CPhysical*)` (0x4DC9F0)
    pub fn door_add_audio_event(this: GPtr<u8>, event: u32, physical: u32) = thiscall 0x4DC9F0 => unported;
    /// `CAERadioTrackManager::SetBassSetting(int8, float)` (0x4E82F0)
    pub fn radio_set_bass_setting(this: GPtr<u8>, setting: u32, gain: u32) = thiscall 0x4E82F0 => unported;
    /// `CAERadioTrackManager::StartRadio(eRadioID, eBassSetting, float, uint8)` (0x4EB3C0)
    pub fn radio_start_radio_manual(this: GPtr<u8>, id: u32, bass: u32, a3: u32, a4: u32) = thiscall 0x4EB3C0 => unported;
    /// `CAERadioTrackManager::StartRadio(tVehicleAudioSettings*)` (0x4EB550)
    pub fn radio_start_radio_settings(this: GPtr<u8>, settings: u32) = thiscall 0x4EB550 => unported;
    /// `CAERadioTrackManager::IsVehicleRadioActive()` (0x4E9800)
    pub fn radio_is_vehicle_radio_active(this: GPtr<u8>) -> u8 = thiscall 0x4E9800 => unported;
    /// `CAEVehicleAudioEntity::StaticGetPlayerVehicleAudioSettingsForRadio()` (0x4F4ED0)
    pub fn vehicle_settings_for_radio() -> u32 = cdecl 0x4F4ED0 => unported;
    /// `CAECutsceneTrackManager::PlayPreloadedCutsceneTrack()` (0x4DBE70)
    pub fn cut_play_preloaded(this: GPtr<u8>) = thiscall 0x4DBE70 => unported;
    /// `CAECutsceneTrackManager::StopCutsceneTrack()` (0x4DBE80)
    pub fn cut_stop(this: GPtr<u8>) = thiscall 0x4DBE80 => unported;
    /// `CAECutsceneTrackManager::IsCutsceneTrackActive()` (0x4DBEB0)
    pub fn cut_is_active(this: GPtr<u8>) -> u8 = thiscall 0x4DBEB0 => unported;
    /// `CAECutsceneTrackManager::Service(int32 trackPlayTime)` (0x4DBFB0)
    pub fn cut_service(this: GPtr<u8>, play_time: u32) = thiscall 0x4DBFB0 => unported;
    /// `CAEAmbienceTrackManager::StopAmbienceTrack()` (0x4D6CC0)
    pub fn amb_stop(this: GPtr<u8>) = thiscall 0x4D6CC0 => unported;
    /// `CAEAmbienceTrackManager::IsAmbienceTrackActive()` (0x4D6CF0)
    pub fn amb_is_active(this: GPtr<u8>) -> u8 = thiscall 0x4D6CF0 => unported;
    /// `CAEAmbienceTrackManager::Service(int32 trackPlayTime)` (0x4D76C0)
    pub fn amb_service(this: GPtr<u8>, play_time: u32) = thiscall 0x4D76C0 => unported;
    /// `CAEScriptAudioEntity::ReportMissionAudioEvent(uint16, CPhysical*, float, float)` (0x4EE960)
    pub fn script_report_mission_audio_event(this: GPtr<u8>, event: u32, physical: u32, a3: u32, a4: u32) = thiscall 0x4EE960 => unported;
    /// `CAEFrontendAudioEntity::AddAudioEvent(eAudioEvents, float, float)` (0x4DD4A0)
    pub fn frontend_add_audio_event(this: GPtr<u8>, event: u32, volume: u32, speed: u32) = thiscall 0x4DD4A0 => unported;
}

/// 1.0f as the word the original pushes
const ONE_F32: u32 = 0x3F80_0000;

/// `CAudioEngine::Restart()` (0x506DB0): `AESoundManager.PauseManually(false)`.
#[hook(addr = 0x506DB0, conv = thiscall, name = "CAudioEngine::Restart", expect = "6A 00 B9 B0 2C B6 00")]
pub fn restart(_this: GPtr<u8>) {
    sound_manager_pause_manually(g(AE_SOUND_MANAGER), 0);
}

/// `fild` of the sign-extended byte times 1/64 (`[0x85A7D4]`), stored as f32.
fn volume_factor(v: u32) -> f32 {
    f32_of(xi(v as u8 as i8 as i32).fmul_m(0.015625))
}

/// `CAudioEngine::SetMusicMasterVolume(int8)` (0x506DE0): `AEAudioHardware.SetMusicMasterScalingFactor(v / 64)`.
#[hook(addr = 0x506DE0, conv = thiscall, name = "CAudioEngine::SetMusicMasterVolume", expect = "0F BE 44 24 04")]
pub fn set_music_master_volume(_this: GPtr<u8>, v: u32) {
    hw_set_music_master_scaling_factor(g(AE_AUDIO_HARDWARE), volume_factor(v));
}

/// `CAudioEngine::SetEffectsMasterVolume(int8)` (0x506E10): `AEAudioHardware.SetEffectsMasterScalingFactor(v / 64)`.
#[hook(addr = 0x506E10, conv = thiscall, name = "CAudioEngine::SetEffectsMasterVolume", expect = "0F BE 44 24 04")]
pub fn set_effects_master_volume(_this: GPtr<u8>, v: u32) {
    hw_set_effects_master_scaling_factor(g(AE_AUDIO_HARDWARE), volume_factor(v));
}

/// `CAudioEngine::ReportGlassCollisionEvent(eAudioEvents, const CVector&)` (0x506EE0): the collision
/// entity's `ReportGlassCollisionEvent(event, pos, 0)`.
#[hook(addr = 0x506EE0, conv = thiscall, name = "CAudioEngine::ReportGlassCollisionEvent", expect = "8B 44 24 08 8B 54 24 04")]
pub fn report_glass_collision_event(this: GPtr<u8>, event: u32, pos: u32) {
    coll_report_glass_collision_event(this.byte_add(0x4BC), event, pos, 0);
}

/// `CAudioEngine::ReportWaterSplash(CVector posn, float volume)` (0x506F10): copies the vector and
/// calls the collision entity's `ReportWaterSplash(posn, volume)`.
#[hook(addr = 0x506F10, conv = thiscall, name = "CAudioEngine::ReportWaterSplash-vec", expect = "8B 44 24 10 50")]
pub fn report_water_splash_vec(this: GPtr<u8>, x: u32, y: u32, z: u32, volume: u32) {
    coll_report_water_splash_at(this.byte_add(0x4BC), x, y, z, volume);
}

/// `CAudioEngine::ReportDoorMovement(CPhysical*)` (0x506F50): the door entity's `AddAudioEvent(0xA3, physical)`.
#[hook(addr = 0x506F50, conv = thiscall, name = "CAudioEngine::ReportDoorMovement", expect = "8B 44 24 04 50")]
pub fn report_door_movement(this: GPtr<u8>, physical: u32) {
    door_add_audio_event(this.byte_add(0x1F50), 0xA3, physical);
}

/// `CAudioEngine::SetRadioBassSetting(eBassSetting)` (0x506FA0): `AERadioTrackManager.SetBassSetting(s, 1.0)`.
#[hook(addr = 0x506FA0, conv = thiscall, name = "CAudioEngine::SetRadioBassSetting", expect = "8B 44 24 04 68 00 00 80 3F")]
pub fn set_radio_bass_setting(_this: GPtr<u8>, setting: u32) {
    radio_set_bass_setting(g(AE_RADIO_TRACK_MANAGER), setting, ONE_F32);
}

/// `CAudioEngine::StopCutsceneTrack(bool)` (0x507080). `this` +0 "cutscene track playing", +1 "stopped
/// while not waiting", +2 the radio station to restart (signed byte, < 0 = the player vehicle's).
/// With `wait`, services the track (`AEAudioHardware.Service`) until it is inactive, then restarts the
/// radio if the vehicle radio is active.
#[hook(addr = 0x507080, conv = thiscall, name = "CAudioEngine::StopCutsceneTrack", expect = "56 8B F1 B9 54 E5 8A 00")]
pub fn stop_cutscene_track(this: GPtr<u8>, wait: u32) {
    let cut = g::<u8>(AE_CUTSCENE_TRACK_MANAGER);
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    let radio = g::<u8>(AE_RADIO_TRACK_MANAGER);
    cut_stop(cut);
    if wait as u8 == 0 {
        if rd::<u8>(this, 0) != 0 {
            wr::<u8>(this, 1, 1);
            wr::<u8>(this, 0, 0);
        }
        return;
    }
    if cut_is_active(cut) != 0 {
        loop {
            let t = hw_get_track_play_time(hw);
            cut_service(cut, t);
            hw_service(hw);
            sa_core::orig::busy_wait();
            if cut_is_active(cut) == 0 {
                break;
            }
        }
    }
    let station = rd::<u8>(this, 2);
    if (station as i8) < 0 {
        if radio_is_vehicle_radio_active(radio) != 0 {
            let s = vehicle_settings_for_radio();
            radio_start_radio_settings(radio, s);
        }
        wr::<u8>(this, 0, 0);
        return;
    }
    if radio_is_vehicle_radio_active(radio) != 0 {
        radio_start_radio_manual(radio, u32::from(rd::<u8>(this, 2)), 0, 0, 0);
    }
    wr::<u8>(this, 2, 0xFF);
    wr::<u8>(this, 0, 0);
}

/// `CAudioEngine::PlayPreloadedBeatTrack(bool)` (0x507180): plays the preloaded cutscene track and
/// stores the flag at +0.
#[hook(addr = 0x507180, conv = thiscall, name = "CAudioEngine::PlayPreloadedBeatTrack", expect = "56 8B F1 B9 54 E5 8A 00")]
pub fn play_preloaded_beat_track(this: GPtr<u8>, flag: u32) {
    cut_play_preloaded(g(AE_CUTSCENE_TRACK_MANAGER));
    wr::<u8>(this, 0, flag as u8);
}

orig! {
    /// `CAudioEngine::StopCutsceneTrack(bool)` (0x507080)
    pub fn stop_cutscene_track_call(this: GPtr<u8>, wait: u32) = thiscall 0x507080 => stop_cutscene_track;
}

/// `CAudioEngine::StopBeatTrack()` (0x5071A0): `StopCutsceneTrack(true)`.
#[hook(addr = 0x5071A0, conv = thiscall, name = "CAudioEngine::StopBeatTrack", expect = "6A 01 E8 D9 FE FF FF")]
pub fn stop_beat_track(this: GPtr<u8>) {
    stop_cutscene_track_call(this, 1);
}

/// `CAudioEngine::GetBeatInfo()` (0x5071B0): fills `this + 8` from the hardware and returns it.
#[hook(addr = 0x5071B0, conv = thiscall, name = "CAudioEngine::GetBeatInfo", expect = "56 8D 71 08 56")]
pub fn get_beat_info(this: GPtr<u8>) -> GPtr<u8> {
    let info = this.byte_add(8);
    hw_get_beat_info(g(AE_AUDIO_HARDWARE), info);
    info
}

/// `CAudioEngine::IsBeatInfoPresent()` (0x5071D0): fills `this + 8`; EAX = 1 when that pointer is not
/// null and its +0xA0 dword is not 0, else 0.
#[hook(addr = 0x5071D0, conv = thiscall, name = "CAudioEngine::IsBeatInfoPresent", expect = "56 8D 71 08 56")]
pub fn is_beat_info_present(this: GPtr<u8>) -> u32 {
    let info = this.byte_add(8);
    hw_get_beat_info(g(AE_AUDIO_HARDWARE), info);
    u32::from(!info.is_null() && rd::<u32>(info, 0xA0) != 0)
}

/// `CAudioEngine::StopAmbienceTrack(bool)` (0x507220): stops the ambience track; with `wait`, services
/// it (`AEAudioHardware.Service`) until it is inactive.
#[hook(addr = 0x507220, conv = thiscall, name = "CAudioEngine::StopAmbienceTrack", expect = "B9 5C C1 8A 00")]
pub fn stop_ambience_track(_this: GPtr<u8>, wait: u32) {
    let amb = g::<u8>(AE_AMBIENCE_TRACK_MANAGER);
    let hw = g::<u8>(AE_AUDIO_HARDWARE);
    amb_stop(amb);
    if wait as u8 == 0 || amb_is_active(amb) == 0 {
        return;
    }
    loop {
        let t = hw_get_track_play_time(hw);
        amb_service(amb, t);
        hw_service(hw);
        sa_core::orig::busy_wait();
        if amb_is_active(amb) == 0 {
            break;
        }
    }
}

/// `CAudioEngine::DoesAmbienceTrackOverrideRadio()` (0x507270): AL = the ambience manager's first byte
/// (`mov al, [0x8AC15C]`; the upper EAX bytes are the caller's).
#[hook(addr = 0x507270, conv = thiscall, name = "CAudioEngine::DoesAmbienceTrackOverrideRadio", expect = "A0 5C C1 8A 00")]
pub fn does_ambience_track_override_radio(_this: GPtr<u8>) -> u8 {
    g::<u8>(AE_AMBIENCE_TRACK_MANAGER).read()
}

/// `CAudioEngine::ReportMissionAudioEvent(uint16, CObject*)` (0x507350): the script entity's
/// `ReportMissionAudioEvent(event, object, 0.0, 1.0)`.
#[hook(addr = 0x507350, conv = thiscall, name = "CAudioEngine::ReportMissionAudioEvent-obj", expect = "8B 44 24 08 8B 54 24 04")]
pub fn report_mission_audio_event_obj(this: GPtr<u8>, event: u32, object: u32) {
    script_report_mission_audio_event(this.byte_add(0x2A0), event, object, 0, ONE_F32);
}

/// `CAudioEngine::ReportMissionAudioEvent(uint16, CPed*)` (0x507370): as the object variant.
#[hook(addr = 0x507370, conv = thiscall, name = "CAudioEngine::ReportMissionAudioEvent-ped", expect = "8B 44 24 08 8B 54 24 04")]
pub fn report_mission_audio_event_ped(this: GPtr<u8>, event: u32, ped: u32) {
    script_report_mission_audio_event(this.byte_add(0x2A0), event, ped, 0, ONE_F32);
}

/// `CAudioEngine::ReportMissionAudioEvent(uint16, CVehicle*)` (0x507390): as the object variant.
#[hook(addr = 0x507390, conv = thiscall, name = "CAudioEngine::ReportMissionAudioEvent-veh", expect = "8B 44 24 08 8B 54 24 04")]
pub fn report_mission_audio_event_veh(this: GPtr<u8>, event: u32, vehicle: u32) {
    script_report_mission_audio_event(this.byte_add(0x2A0), event, vehicle, 0, ONE_F32);
}

/// `CAudioEngine::StartLoadingTune()` (0x507410): the front-end entity's `AddAudioEvent(0x32, 0.0, 1.0)`,
/// then `AESoundManager.Service()` (tail jump).
#[hook(addr = 0x507410, conv = thiscall, name = "CAudioEngine::StartLoadingTune", expect = "68 00 00 80 3F")]
pub fn start_loading_tune(this: GPtr<u8>) {
    frontend_add_audio_event(this.byte_add(0xB4), 0x32, 0, ONE_F32);
    sound_manager_service(g(AE_SOUND_MANAGER));
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RESTART,
    HOOK_SET_MUSIC_MASTER_VOLUME,
    HOOK_SET_EFFECTS_MASTER_VOLUME,
    HOOK_REPORT_GLASS_COLLISION_EVENT,
    HOOK_REPORT_WATER_SPLASH_VEC,
    HOOK_REPORT_DOOR_MOVEMENT,
    HOOK_SET_RADIO_BASS_SETTING,
    HOOK_STOP_CUTSCENE_TRACK,
    HOOK_PLAY_PRELOADED_BEAT_TRACK,
    HOOK_STOP_BEAT_TRACK,
    HOOK_GET_BEAT_INFO,
    HOOK_IS_BEAT_INFO_PRESENT,
    HOOK_STOP_AMBIENCE_TRACK,
    HOOK_DOES_AMBIENCE_TRACK_OVERRIDE_RADIO,
    HOOK_REPORT_MISSION_AUDIO_EVENT_OBJ,
    HOOK_REPORT_MISSION_AUDIO_EVENT_PED,
    HOOK_REPORT_MISSION_AUDIO_EVENT_VEH,
    HOOK_START_LOADING_TUNE,
    // the tail-jump forwarders
    HOOK_IS_LOADING_TUNE_ACTIVE,
    HOOK_RESET_STATISTICS,
    HOOK_SET_MUSIC_FADER_SCALING_FACTOR,
    HOOK_SET_EFFECTS_FADER_SCALING_FACTOR,
    HOOK_SET_NON_STREAM_FADER_SCALING_FACTOR,
    HOOK_SET_STREAM_FADER_SCALING_FACTOR,
    HOOK_DISABLE_EFFECTS_LOADING,
    HOOK_ENABLE_EFFECTS_LOADING,
    HOOK_REPORT_FRONTEND_AUDIO_EVENT,
    HOOK_REPORT_COLLISION,
    HOOK_REPORT_BULLET_HIT,
    HOOK_REPORT_OBJECT_DESTRUCTION,
    HOOK_REPORT_WATER_SPLASH_PHYSICAL,
    HOOK_REPORT_WEAPON_EVENT,
    HOOK_STOP_RADIO,
    HOOK_SET_RADIO_AUTO_RETUNE_ON_OFF,
    HOOK_SET_BASS_ENHANCE_ON_OFF,
    HOOK_INITIALISE_RADIO_STATION_ID,
    HOOK_IS_RADIO_ON,
    HOOK_HAS_RADIO_RETUNE_JUST_STARTED,
    HOOK_IS_RADIO_RETUNE_IN_PROGRESS,
    HOOK_GET_RADIO_STATION_NAME,
    HOOK_GET_RADIO_STATION_NAME_KEY,
    HOOK_GET_RADIO_STATION_LISTEN_TIMES,
    HOOK_DISPLAY_RADIO_STATION_NAME,
    HOOK_GET_CURRENT_RADIO_STATION_ID,
    HOOK_IS_VEHICLE_RADIO_ACTIVE,
    HOOK_PLAY_PRELOADED_CUTSCENE_TRACK,
    HOOK_IS_CUTSCENE_TRACK_ACTIVE,
    HOOK_GET_CUTSCENE_TRACK_STATUS,
    HOOK_GET_BEAT_TRACK_STATUS,
    HOOK_PAUSE_BEAT_TRACK,
    HOOK_IS_AMBIENCE_TRACK_ACTIVE,
    HOOK_IS_AMBIENCE_RADIO_ACTIVE,
    HOOK_PRELOAD_MISSION_AUDIO,
    HOOK_GET_MISSION_AUDIO_LOADING_STATUS,
    HOOK_PLAY_LOADED_MISSION_AUDIO,
    HOOK_IS_MISSION_AUDIO_SAMPLE_FINISHED,
    HOOK_GET_MISSION_AUDIO_EVENT,
    HOOK_GET_MISSION_AUDIO_POSITION,
    HOOK_CLEAR_MISSION_AUDIO,
    HOOK_SET_MISSION_AUDIO_POSITION,
    HOOK_ATTACH_MISSION_AUDIO_TO_PED,
    HOOK_ATTACH_MISSION_AUDIO_TO_OBJECT,
    HOOK_ATTACH_MISSION_AUDIO_TO_PHYSICAL,
    HOOK_REPORT_MISSION_AUDIO_EVENT_VEC,
    HOOK_REPORT_MISSION_AUDIO_EVENT_PHYSICAL,
    HOOK_SAY_PEDLESS,
    HOOK_PAUSE_ALL_SOUNDS,
    HOOK_RESUME_ALL_SOUNDS,
];

// ---- tail-jump forwarders (generated from the bytes: target, this adjustment, stack words) ----

orig! {
    /// `CAEFrontendAudioEntity::IsLoadingTuneActive()` (0x4DD470)
    pub fn t_is_loading_tune_active(this: GPtr<u8>) -> u8 = thiscall 0x4DD470 => unported;
    /// `CAERadioTrackManager::ResetStatistics()` (0x4E8200)
    pub fn t_reset_statistics(this: GPtr<u8>) = thiscall 0x4E8200 => unported;
    /// `CAEAudioHardware::SetMusicFaderScalingFactor(factor: float)` (0x4D9550)
    pub fn t_set_music_fader_scaling_factor(this: GPtr<u8>, factor: u32) = thiscall 0x4D9550 => unported;
    /// `CAEAudioHardware::SetEffectsFaderScalingFactor(factor: float)` (0x4D9560)
    pub fn t_set_effects_fader_scaling_factor(this: GPtr<u8>, factor: u32) = thiscall 0x4D9560 => unported;
    /// `CAEAudioHardware::SetNonStreamFaderScalingFactor(factor: float)` (0x4D95A0)
    pub fn t_set_non_stream_fader_scaling_factor(this: GPtr<u8>, factor: u32) = thiscall 0x4D95A0 => unported;
    /// `CAEAudioHardware::SetStreamFaderScalingFactor(factor: float)` (0x4D95B0)
    pub fn t_set_stream_fader_scaling_factor(this: GPtr<u8>, factor: u32) = thiscall 0x4D95B0 => unported;
    /// `CAEAudioHardware::DisableEffectsLoading()` (0x4D8E50)
    pub fn t_disable_effects_loading(this: GPtr<u8>) = thiscall 0x4D8E50 => unported;
    /// `CAEAudioHardware::EnableEffectsLoading()` (0x4D8E40)
    pub fn t_enable_effects_loading(this: GPtr<u8>) = thiscall 0x4D8E40 => unported;
    /// `CAEFrontendAudioEntity::AddAudioEvent(event: eAudioEvents, volume_change: float, speed: float)` (0x4DD4A0)
    pub fn t_report_frontend_audio_event(this: GPtr<u8>, event: u32, volume_change: u32, speed: u32) = thiscall 0x4DD4A0 => unported;
    /// `CAECollisionAudioEntity::ReportCollision(entity1: CEntity*, entity2: CEntity*, surf1: eSurfaceType, surf2: eSurfaceType, pos: const CVector&, normal: const CVector*, impact1: float, impact2: float, one_shot: bool, unknown: bool)` (0x4DBA10)
    pub fn t_report_collision(this: GPtr<u8>, entity1: u32, entity2: u32, surf1: u32, surf2: u32, pos: u32, normal: u32, impact1: u32, impact2: u32, one_shot: u32, unknown: u32) = thiscall 0x4DBA10 => unported;
    /// `CAECollisionAudioEntity::ReportBulletHit(entity: CEntity*, surface: eSurfaceType, pos: const CVector&, angle: float)` (0x4DBDF0)
    pub fn t_report_bullet_hit(this: GPtr<u8>, entity: u32, surface: u32, pos: u32, angle: u32) = thiscall 0x4DBDF0 => unported;
    /// `CAECollisionAudioEntity::ReportObjectDestruction(entity: CEntity*)` (0x4DAB60)
    pub fn t_report_object_destruction(this: GPtr<u8>, entity: u32) = thiscall 0x4DAB60 => unported;
    /// `CAECollisionAudioEntity::ReportWaterSplash-for-physical(physical: CPhysical*, volume: float, force: bool)` (0x4DAE40)
    pub fn t_report_water_splash_physical(this: GPtr<u8>, physical: u32, volume: u32, force: u32) = thiscall 0x4DAE40 => unported;
    /// `CAEGlobalWeaponAudioEntity::AddAudioEvent(event: int32, weapon_type: eWeaponType, physical: CPhysical*)` (0x4DFAA0)
    pub fn t_report_weapon_event(this: GPtr<u8>, event: u32, weapon_type: u32, physical: u32) = thiscall 0x4DFAA0 => unported;
    /// `CAERadioTrackManager::StopRadio(settings: tVehicleAudioSettings*, during_pause: bool)` (0x4E9820)
    pub fn t_stop_radio(this: GPtr<u8>, settings: u32, during_pause: u32) = thiscall 0x4E9820 => unported;
    /// `CAERadioTrackManager::SetRadioAutoRetuneOnOff(on: bool)` (0x4E82E0)
    pub fn t_set_radio_auto_retune_on_off(this: GPtr<u8>, on: u32) = thiscall 0x4E82E0 => unported;
    /// `CAERadioTrackManager::SetBassEnhanceOnOff(on: bool)` (0x4E9DB0)
    pub fn t_set_bass_enhance_on_off(this: GPtr<u8>, on: u32) = thiscall 0x4E9DB0 => unported;
    /// `CAERadioTrackManager::InitialiseRadioStationID(id: eRadioID)` (0x4E8330)
    pub fn t_initialise_radio_station_id(this: GPtr<u8>, id: u32) = thiscall 0x4E8330 => unported;
    /// `CAERadioTrackManager::IsRadioOn()` (0x4E8350)
    pub fn t_is_radio_on(this: GPtr<u8>) -> u8 = thiscall 0x4E8350 => unported;
    /// `CAERadioTrackManager::HasRadioRetuneJustStarted()` (0x4E8370)
    pub fn t_has_radio_retune_just_started(this: GPtr<u8>) -> u8 = thiscall 0x4E8370 => unported;
    /// `CAEFrontendAudioEntity::IsRadioTuneSoundActive()` (0x4DD480)
    pub fn t_is_radio_retune_in_progress(this: GPtr<u8>) -> u8 = thiscall 0x4DD480 => unported;
    /// `CAERadioTrackManager::GetRadioStationName(id: eRadioID)` (0x4E9E10)
    pub fn t_get_radio_station_name(this: GPtr<u8>, id: u32) -> u32 = thiscall 0x4E9E10 => unported;
    /// `CAERadioTrackManager::GetRadioStationNameKey(id: eRadioID, out: char(&)[8])` (0x4E8380)
    pub fn t_get_radio_station_name_key(this: GPtr<u8>, id: u32, out: u32) = thiscall 0x4E8380 => unported;
    /// `CAERadioTrackManager::GetRadioStationListenTimes()` (0x4E83E0)
    pub fn t_get_radio_station_listen_times(this: GPtr<u8>) -> u32 = thiscall 0x4E83E0 => unported;
    /// `CAERadioTrackManager::DisplayRadioStationName()` (0x4E9E50)
    pub fn t_display_radio_station_name(this: GPtr<u8>) = thiscall 0x4E9E50 => unported;
    /// `CAERadioTrackManager::GetCurrentRadioStationID()` (0x4E83F0)
    pub fn t_get_current_radio_station_id(this: GPtr<u8>) -> u8 = thiscall 0x4E83F0 => unported;
    /// `CAERadioTrackManager::IsVehicleRadioActive()` (0x4E9800)
    pub fn t_is_vehicle_radio_active(this: GPtr<u8>) -> u8 = thiscall 0x4E9800 => unported;
    /// `CAECutsceneTrackManager::PlayPreloadedCutsceneTrack()` (0x4DBE70)
    pub fn t_play_preloaded_cutscene_track(this: GPtr<u8>) = thiscall 0x4DBE70 => unported;
    /// `CAECutsceneTrackManager::IsCutsceneTrackActive()` (0x4DBEB0)
    pub fn t_is_cutscene_track_active(this: GPtr<u8>) -> u8 = thiscall 0x4DBEB0 => unported;
    /// `CAECutsceneTrackManager::GetCutsceneTrackStatus()` (0x4DBEC0)
    pub fn t_get_cutscene_track_status(this: GPtr<u8>) -> u8 = thiscall 0x4DBEC0 => unported;
    /// `CAECutsceneTrackManager::GetCutsceneTrackStatus()` (0x4DBEC0)
    pub fn t_get_beat_track_status(this: GPtr<u8>) -> u8 = thiscall 0x4DBEC0 => unported;
    /// `CAECutsceneTrackManager::PauseTrack(pause: bool)` (0x4DBF00)
    pub fn t_pause_beat_track(this: GPtr<u8>, pause: u32) = thiscall 0x4DBF00 => unported;
    /// `CAEAmbienceTrackManager::IsAmbienceTrackActive()` (0x4D6CF0)
    pub fn t_is_ambience_track_active(this: GPtr<u8>) -> u8 = thiscall 0x4D6CF0 => unported;
    /// `CAEAmbienceTrackManager::IsAmbienceRadioActive()` (0x4D6D40)
    pub fn t_is_ambience_radio_active(this: GPtr<u8>) -> u8 = thiscall 0x4D6D40 => unported;
    /// `CAEScriptAudioEntity::PreloadMissionAudio(slot: uint8, event: eAudioEvents)` (0x4EC190)
    pub fn t_preload_mission_audio(this: GPtr<u8>, slot: u32, event: u32) = thiscall 0x4EC190 => unported;
    /// `CAEScriptAudioEntity::GetMissionAudioLoadingStatus(slot: uint8)` (0x4EBF60)
    pub fn t_get_mission_audio_loading_status(this: GPtr<u8>, slot: u32) -> u8 = thiscall 0x4EBF60 => unported;
    /// `CAEScriptAudioEntity::PlayLoadedMissionAudio(slot: uint8)` (0x4EC270)
    pub fn t_play_loaded_mission_audio(this: GPtr<u8>, slot: u32) = thiscall 0x4EC270 => unported;
    /// `CAEScriptAudioEntity::IsMissionAudioSampleFinished(slot: uint8)` (0x4EBFE0)
    pub fn t_is_mission_audio_sample_finished(this: GPtr<u8>, slot: u32) -> u8 = thiscall 0x4EBFE0 => unported;
    /// `CAEScriptAudioEntity::GetMissionAudioEvent(slot: uint8)` (0x4EC020)
    pub fn t_get_mission_audio_event(this: GPtr<u8>, slot: u32) -> u32 = thiscall 0x4EC020 => unported;
    /// `CAEScriptAudioEntity::GetMissionAudioPosition(slot: uint8)` (0x4EC4D0)
    pub fn t_get_mission_audio_position(this: GPtr<u8>, slot: u32) -> u32 = thiscall 0x4EC4D0 => unported;
    /// `CAEScriptAudioEntity::ClearMissionAudio(slot: uint8)` (0x4EC040)
    pub fn t_clear_mission_audio(this: GPtr<u8>, slot: u32) = thiscall 0x4EC040 => unported;
    /// `CAEScriptAudioEntity::SetMissionAudioPosition(slot: uint8, pos: CVector&)` (0x4EC0C0)
    pub fn t_set_mission_audio_position(this: GPtr<u8>, slot: u32, pos: u32) = thiscall 0x4EC0C0 => unported;
    /// `CAEScriptAudioEntity::AttachMissionAudioToPhysical(slot: uint8, ped: CPed*)` (0x4EC100)
    pub fn t_attach_mission_audio_to_ped(this: GPtr<u8>, slot: u32, ped: u32) = thiscall 0x4EC100 => unported;
    /// `CAEScriptAudioEntity::AttachMissionAudioToPhysical(slot: uint8, object: CObject*)` (0x4EC100)
    pub fn t_attach_mission_audio_to_object(this: GPtr<u8>, slot: u32, object: u32) = thiscall 0x4EC100 => unported;
    /// `CAEScriptAudioEntity::AttachMissionAudioToPhysical(slot: uint8, physical: CPhysical*)` (0x4EC100)
    pub fn t_attach_mission_audio_to_physical(this: GPtr<u8>, slot: u32, physical: u32) = thiscall 0x4EC100 => unported;
    /// `CAEScriptAudioEntity::ReportMissionAudioEvent-2(event: uint16, pos: CVector&)` (0x4EE940)
    pub fn t_report_mission_audio_event_vec(this: GPtr<u8>, event: u32, pos: u32) = thiscall 0x4EE940 => unported;
    /// `CAEScriptAudioEntity::ReportMissionAudioEvent-1(event: uint16, physical: CPhysical*, a3: float, a4: float)` (0x4EE960)
    pub fn t_report_mission_audio_event_physical(this: GPtr<u8>, event: u32, physical: u32, a3: u32, a4: u32) = thiscall 0x4EE960 => unported;
    /// `CAEPedlessSpeechAudioEntity::AddSayEvent(event: eAudioEvents, ctx: eGlobalSpeechContext, attach_to: CEntity*, delay_ms: uint32, probability: float, override_silence: bool, force_audible: bool, frontend: bool)` (0x4E60D0)
    pub fn t_say_pedless(this: GPtr<u8>, event: u32, ctx: u32, attach_to: u32, delay_ms: u32, probability: u32, override_silence: u32, force_audible: u32, frontend: u32) = thiscall 0x4E60D0 => unported;
    /// `CAEAudioHardware::PauseAllSounds()` (0x4D95F0)
    pub fn t_pause_all_sounds(this: GPtr<u8>) = thiscall 0x4D95F0 => unported;
    /// `CAEAudioHardware::ResumeAllSounds()` (0x4D9630)
    pub fn t_resume_all_sounds(this: GPtr<u8>) = thiscall 0x4D9630 => unported;
}

/// `CAudioEngine::IsLoadingTuneActive` (0x506D90): `jmp CAEFrontendAudioEntity::IsLoadingTuneActive` on `this + 0xB4`. AL is the target's (`bool` / `int8`; tail jump).
#[hook(addr = 0x506D90, conv = thiscall, name = "CAudioEngine::IsLoadingTuneActive", expect = "81 C1 B4 00 00 00")]
pub fn is_loading_tune_active(this: GPtr<u8>) -> u8 {
    t_is_loading_tune_active(this.byte_add(0xB4))
}

/// `CAudioEngine::ResetStatistics` (0x506DA0): `jmp CAERadioTrackManager::ResetStatistics` on `AE_RADIO_TRACK_MANAGER`.
#[hook(addr = 0x506DA0, conv = thiscall, name = "CAudioEngine::ResetStatistics", expect = "B9 F8 B6 8C 00")]
pub fn reset_statistics(_this: GPtr<u8>) {
    t_reset_statistics(g(AE_RADIO_TRACK_MANAGER))
}

/// `CAudioEngine::SetMusicFaderScalingFactor` (0x506E40): `jmp CAEAudioHardware::SetMusicFaderScalingFactor` on `AE_AUDIO_HARDWARE`.
#[hook(addr = 0x506E40, conv = thiscall, name = "CAudioEngine::SetMusicFaderScalingFactor", expect = "B9 B8 F8 B5 00")]
pub fn set_music_fader_scaling_factor(_this: GPtr<u8>, factor: u32) {
    t_set_music_fader_scaling_factor(g(AE_AUDIO_HARDWARE), factor)
}

/// `CAudioEngine::SetEffectsFaderScalingFactor` (0x506E50): `jmp CAEAudioHardware::SetEffectsFaderScalingFactor` on `AE_AUDIO_HARDWARE`.
#[hook(addr = 0x506E50, conv = thiscall, name = "CAudioEngine::SetEffectsFaderScalingFactor", expect = "B9 B8 F8 B5 00")]
pub fn set_effects_fader_scaling_factor(_this: GPtr<u8>, factor: u32) {
    t_set_effects_fader_scaling_factor(g(AE_AUDIO_HARDWARE), factor)
}

/// `CAudioEngine::SetNonStreamFaderScalingFactor` (0x506E60): `jmp CAEAudioHardware::SetNonStreamFaderScalingFactor` on `AE_AUDIO_HARDWARE`.
#[hook(addr = 0x506E60, conv = thiscall, name = "CAudioEngine::SetNonStreamFaderScalingFactor", expect = "B9 B8 F8 B5 00")]
pub fn set_non_stream_fader_scaling_factor(_this: GPtr<u8>, factor: u32) {
    t_set_non_stream_fader_scaling_factor(g(AE_AUDIO_HARDWARE), factor)
}

/// `CAudioEngine::SetStreamFaderScalingFactor` (0x506E70): `jmp CAEAudioHardware::SetStreamFaderScalingFactor` on `AE_AUDIO_HARDWARE`.
#[hook(addr = 0x506E70, conv = thiscall, name = "CAudioEngine::SetStreamFaderScalingFactor", expect = "B9 B8 F8 B5 00")]
pub fn set_stream_fader_scaling_factor(_this: GPtr<u8>, factor: u32) {
    t_set_stream_fader_scaling_factor(g(AE_AUDIO_HARDWARE), factor)
}

/// `CAudioEngine::DisableEffectsLoading` (0x506E80): `jmp CAEAudioHardware::DisableEffectsLoading` on `AE_AUDIO_HARDWARE`.
#[hook(addr = 0x506E80, conv = thiscall, name = "CAudioEngine::DisableEffectsLoading", expect = "B9 B8 F8 B5 00")]
pub fn disable_effects_loading(_this: GPtr<u8>) {
    t_disable_effects_loading(g(AE_AUDIO_HARDWARE))
}

/// `CAudioEngine::EnableEffectsLoading` (0x506E90): `jmp CAEAudioHardware::EnableEffectsLoading` on `AE_AUDIO_HARDWARE`.
#[hook(addr = 0x506E90, conv = thiscall, name = "CAudioEngine::EnableEffectsLoading", expect = "B9 B8 F8 B5 00")]
pub fn enable_effects_loading(_this: GPtr<u8>) {
    t_enable_effects_loading(g(AE_AUDIO_HARDWARE))
}

/// `CAudioEngine::ReportFrontendAudioEvent` (0x506EA0): `jmp CAEFrontendAudioEntity::AddAudioEvent` on `this + 0xB4`.
#[hook(addr = 0x506EA0, conv = thiscall, name = "CAudioEngine::ReportFrontendAudioEvent", expect = "81 C1 B4 00 00 00")]
pub fn report_frontend_audio_event(this: GPtr<u8>, event: u32, volume_change: u32, speed: u32) {
    t_report_frontend_audio_event(this.byte_add(0xB4), event, volume_change, speed)
}

/// `CAudioEngine::ReportCollision` (0x506EB0): `jmp CAECollisionAudioEntity::ReportCollision` on `this + 0x4BC`.
#[hook(addr = 0x506EB0, conv = thiscall, name = "CAudioEngine::ReportCollision", expect = "81 C1 BC 04 00 00")]
pub fn report_collision(
    this: GPtr<u8>,
    entity1: u32,
    entity2: u32,
    surf1: u32,
    surf2: u32,
    pos: u32,
    normal: u32,
    impact1: u32,
    impact2: u32,
    one_shot: u32,
    unknown: u32,
) {
    t_report_collision(
        this.byte_add(0x4BC),
        entity1,
        entity2,
        surf1,
        surf2,
        pos,
        normal,
        impact1,
        impact2,
        one_shot,
        unknown,
    )
}

/// `CAudioEngine::ReportBulletHit` (0x506EC0): `jmp CAECollisionAudioEntity::ReportBulletHit` on `this + 0x4BC`.
#[hook(addr = 0x506EC0, conv = thiscall, name = "CAudioEngine::ReportBulletHit", expect = "81 C1 BC 04 00 00")]
pub fn report_bullet_hit(this: GPtr<u8>, entity: u32, surface: u32, pos: u32, angle: u32) {
    t_report_bullet_hit(this.byte_add(0x4BC), entity, surface, pos, angle)
}

/// `CAudioEngine::ReportObjectDestruction` (0x506ED0): `jmp CAECollisionAudioEntity::ReportObjectDestruction` on `this + 0x4BC`.
#[hook(addr = 0x506ED0, conv = thiscall, name = "CAudioEngine::ReportObjectDestruction", expect = "81 C1 BC 04 00 00")]
pub fn report_object_destruction(this: GPtr<u8>, entity: u32) {
    t_report_object_destruction(this.byte_add(0x4BC), entity)
}

/// `CAudioEngine::ReportWaterSplash-phs` (0x506F00): `jmp CAECollisionAudioEntity::ReportWaterSplash-for-physical` on `this + 0x4BC`.
#[hook(addr = 0x506F00, conv = thiscall, name = "CAudioEngine::ReportWaterSplash-phs", expect = "81 C1 BC 04 00 00")]
pub fn report_water_splash_physical(this: GPtr<u8>, physical: u32, volume: u32, force: u32) {
    t_report_water_splash_physical(this.byte_add(0x4BC), physical, volume, force)
}

/// `CAudioEngine::ReportWeaponEvent` (0x506F40): `jmp CAEGlobalWeaponAudioEntity::AddAudioEvent` on `*(this + 0x1E34)`.
#[hook(addr = 0x506F40, conv = thiscall, name = "CAudioEngine::ReportWeaponEvent", expect = "8B 89 34 1E 00 00")]
pub fn report_weapon_event(this: GPtr<u8>, event: u32, weapon_type: u32, physical: u32) {
    t_report_weapon_event(rd::<GPtr<u8>>(this, 0x1E34), event, weapon_type, physical)
}

/// `CAudioEngine::StopRadio` (0x506F70): `jmp CAERadioTrackManager::StopRadio` on `AE_RADIO_TRACK_MANAGER`.
#[hook(addr = 0x506F70, conv = thiscall, name = "CAudioEngine::StopRadio", expect = "B9 F8 B6 8C 00")]
pub fn stop_radio(_this: GPtr<u8>, settings: u32, during_pause: u32) {
    t_stop_radio(g(AE_RADIO_TRACK_MANAGER), settings, during_pause)
}

/// `CAudioEngine::SetRadioAutoRetuneOnOff` (0x506F80): `jmp CAERadioTrackManager::SetRadioAutoRetuneOnOff` on `AE_RADIO_TRACK_MANAGER`.
#[hook(addr = 0x506F80, conv = thiscall, name = "CAudioEngine::SetRadioAutoRetuneOnOff", expect = "B9 F8 B6 8C 00")]
pub fn set_radio_auto_retune_on_off(_this: GPtr<u8>, on: u32) {
    t_set_radio_auto_retune_on_off(g(AE_RADIO_TRACK_MANAGER), on)
}

/// `CAudioEngine::SetBassEnhanceOnOff` (0x506F90): `jmp CAERadioTrackManager::SetBassEnhanceOnOff` on `AE_RADIO_TRACK_MANAGER`.
#[hook(addr = 0x506F90, conv = thiscall, name = "CAudioEngine::SetBassEnhanceOnOff", expect = "B9 F8 B6 8C 00")]
pub fn set_bass_enhance_on_off(_this: GPtr<u8>, on: u32) {
    t_set_bass_enhance_on_off(g(AE_RADIO_TRACK_MANAGER), on)
}

/// `CAudioEngine::InitialiseRadioStationID` (0x506FC0): `jmp CAERadioTrackManager::InitialiseRadioStationID` on `AE_RADIO_TRACK_MANAGER`.
#[hook(addr = 0x506FC0, conv = thiscall, name = "CAudioEngine::InitialiseRadioStationID", expect = "B9 F8 B6 8C 00")]
pub fn initialise_radio_station_id(_this: GPtr<u8>, id: u32) {
    t_initialise_radio_station_id(g(AE_RADIO_TRACK_MANAGER), id)
}

/// `CAudioEngine::IsRadioOn` (0x506FD0): `jmp CAERadioTrackManager::IsRadioOn` on `AE_RADIO_TRACK_MANAGER`. AL is the target's (`bool` / `int8`; tail jump).
#[hook(addr = 0x506FD0, conv = thiscall, name = "CAudioEngine::IsRadioOn", expect = "B9 F8 B6 8C 00")]
pub fn is_radio_on(_this: GPtr<u8>) -> u8 {
    t_is_radio_on(g(AE_RADIO_TRACK_MANAGER))
}

/// `CAudioEngine::HasRadioRetuneJustStarted` (0x506FE0): `jmp CAERadioTrackManager::HasRadioRetuneJustStarted` on `AE_RADIO_TRACK_MANAGER`. AL is the target's (`bool` / `int8`; tail jump).
#[hook(addr = 0x506FE0, conv = thiscall, name = "CAudioEngine::HasRadioRetuneJustStarted", expect = "B9 F8 B6 8C 00")]
pub fn has_radio_retune_just_started(_this: GPtr<u8>) -> u8 {
    t_has_radio_retune_just_started(g(AE_RADIO_TRACK_MANAGER))
}

/// `CAudioEngine::IsRadioRetuneInProgress` (0x506FF0): `jmp CAEFrontendAudioEntity::IsRadioTuneSoundActive` on `this + 0xB4`. AL is the target's (`bool` / `int8`; tail jump).
#[hook(addr = 0x506FF0, conv = thiscall, name = "CAudioEngine::IsRadioRetuneInProgress", expect = "81 C1 B4 00 00 00")]
pub fn is_radio_retune_in_progress(this: GPtr<u8>) -> u8 {
    t_is_radio_retune_in_progress(this.byte_add(0xB4))
}

/// `CAudioEngine::GetRadioStationName` (0x507000): `jmp CAERadioTrackManager::GetRadioStationName` on `AE_RADIO_TRACK_MANAGER`. EAX is the target's (tail jump).
#[hook(addr = 0x507000, conv = thiscall, name = "CAudioEngine::GetRadioStationName", expect = "B9 F8 B6 8C 00")]
pub fn get_radio_station_name(_this: GPtr<u8>, id: u32) -> u32 {
    t_get_radio_station_name(g(AE_RADIO_TRACK_MANAGER), id)
}

/// `CAudioEngine::GetRadioStationNameKey` (0x507010): `jmp CAERadioTrackManager::GetRadioStationNameKey` on `AE_RADIO_TRACK_MANAGER`.
#[hook(addr = 0x507010, conv = thiscall, name = "CAudioEngine::GetRadioStationNameKey", expect = "B9 F8 B6 8C 00")]
pub fn get_radio_station_name_key(_this: GPtr<u8>, id: u32, out: u32) {
    t_get_radio_station_name_key(g(AE_RADIO_TRACK_MANAGER), id, out)
}

/// `CAudioEngine::GetRadioStationListenTimes` (0x507020): `jmp CAERadioTrackManager::GetRadioStationListenTimes` on `AE_RADIO_TRACK_MANAGER`. EAX is the target's (tail jump).
#[hook(addr = 0x507020, conv = thiscall, name = "CAudioEngine::GetRadioStationListenTimes", expect = "B9 F8 B6 8C 00")]
pub fn get_radio_station_listen_times(_this: GPtr<u8>) -> u32 {
    t_get_radio_station_listen_times(g(AE_RADIO_TRACK_MANAGER))
}

/// `CAudioEngine::DisplayRadioStationName` (0x507030): `jmp CAERadioTrackManager::DisplayRadioStationName` on `AE_RADIO_TRACK_MANAGER`.
#[hook(addr = 0x507030, conv = thiscall, name = "CAudioEngine::DisplayRadioStationName", expect = "B9 F8 B6 8C 00")]
pub fn display_radio_station_name(_this: GPtr<u8>) {
    t_display_radio_station_name(g(AE_RADIO_TRACK_MANAGER))
}

/// `CAudioEngine::GetCurrentRadioStationID` (0x507040): `jmp CAERadioTrackManager::GetCurrentRadioStationID` on `AE_RADIO_TRACK_MANAGER`. AL is the target's (`bool` / `int8`; tail jump).
#[hook(addr = 0x507040, conv = thiscall, name = "CAudioEngine::GetCurrentRadioStationID", expect = "B9 F8 B6 8C 00")]
pub fn get_current_radio_station_id(_this: GPtr<u8>) -> u8 {
    t_get_current_radio_station_id(g(AE_RADIO_TRACK_MANAGER))
}

/// `CAudioEngine::IsVehicleRadioActive` (0x507050): `jmp CAERadioTrackManager::IsVehicleRadioActive` on `AE_RADIO_TRACK_MANAGER`. AL is the target's (`bool` / `int8`; tail jump).
#[hook(addr = 0x507050, conv = thiscall, name = "CAudioEngine::IsVehicleRadioActive", expect = "B9 F8 B6 8C 00")]
pub fn is_vehicle_radio_active(_this: GPtr<u8>) -> u8 {
    t_is_vehicle_radio_active(g(AE_RADIO_TRACK_MANAGER))
}

/// `CAudioEngine::PlayPreloadedCutsceneTrack` (0x507070): `jmp CAECutsceneTrackManager::PlayPreloadedCutsceneTrack` on `AE_CUTSCENE_TRACK_MANAGER`.
#[hook(addr = 0x507070, conv = thiscall, name = "CAudioEngine::PlayPreloadedCutsceneTrack", expect = "B9 54 E5 8A 00")]
pub fn play_preloaded_cutscene_track(_this: GPtr<u8>) {
    t_play_preloaded_cutscene_track(g(AE_CUTSCENE_TRACK_MANAGER))
}

/// `CAudioEngine::IsCutsceneTrackActive` (0x507150): `jmp CAECutsceneTrackManager::IsCutsceneTrackActive` on `AE_CUTSCENE_TRACK_MANAGER`. AL is the target's (`bool` / `int8`; tail jump).
#[hook(addr = 0x507150, conv = thiscall, name = "CAudioEngine::IsCutsceneTrackActive", expect = "B9 54 E5 8A 00")]
pub fn is_cutscene_track_active(_this: GPtr<u8>) -> u8 {
    t_is_cutscene_track_active(g(AE_CUTSCENE_TRACK_MANAGER))
}

/// `CAudioEngine::GetCutsceneTrackStatus` (0x507160): `jmp CAECutsceneTrackManager::GetCutsceneTrackStatus` on `AE_CUTSCENE_TRACK_MANAGER`. AL is the target's (`bool` / `int8`; tail jump).
#[hook(addr = 0x507160, conv = thiscall, name = "CAudioEngine::GetCutsceneTrackStatus", expect = "B9 54 E5 8A 00")]
pub fn get_cutscene_track_status(_this: GPtr<u8>) -> u8 {
    t_get_cutscene_track_status(g(AE_CUTSCENE_TRACK_MANAGER))
}

/// `CAudioEngine::GetBeatTrackStatus` (0x507170): `jmp CAECutsceneTrackManager::GetCutsceneTrackStatus` on `AE_CUTSCENE_TRACK_MANAGER`. AL is the target's (`bool` / `int8`; tail jump).
#[hook(addr = 0x507170, conv = thiscall, name = "CAudioEngine::GetBeatTrackStatus", expect = "B9 54 E5 8A 00")]
pub fn get_beat_track_status(_this: GPtr<u8>) -> u8 {
    t_get_beat_track_status(g(AE_CUTSCENE_TRACK_MANAGER))
}

/// `CAudioEngine::PauseBeatTrack` (0x507200): `jmp CAECutsceneTrackManager::PauseTrack` on `AE_CUTSCENE_TRACK_MANAGER`.
#[hook(addr = 0x507200, conv = thiscall, name = "CAudioEngine::PauseBeatTrack", expect = "B9 54 E5 8A 00")]
pub fn pause_beat_track(_this: GPtr<u8>, pause: u32) {
    t_pause_beat_track(g(AE_CUTSCENE_TRACK_MANAGER), pause)
}

/// `CAudioEngine::IsAmbienceTrackActive` (0x507210): `jmp CAEAmbienceTrackManager::IsAmbienceTrackActive` on `AE_AMBIENCE_TRACK_MANAGER`. AL is the target's (`bool` / `int8`; tail jump).
#[hook(addr = 0x507210, conv = thiscall, name = "CAudioEngine::IsAmbienceTrackActive", expect = "B9 5C C1 8A 00")]
pub fn is_ambience_track_active(_this: GPtr<u8>) -> u8 {
    t_is_ambience_track_active(g(AE_AMBIENCE_TRACK_MANAGER))
}

/// `CAudioEngine::IsAmbienceRadioActive` (0x507280): `jmp CAEAmbienceTrackManager::IsAmbienceRadioActive` on `AE_AMBIENCE_TRACK_MANAGER`. AL is the target's (`bool` / `int8`; tail jump).
#[hook(addr = 0x507280, conv = thiscall, name = "CAudioEngine::IsAmbienceRadioActive", expect = "B9 5C C1 8A 00")]
pub fn is_ambience_radio_active(_this: GPtr<u8>) -> u8 {
    t_is_ambience_radio_active(g(AE_AMBIENCE_TRACK_MANAGER))
}

/// `CAudioEngine::PreloadMissionAudio` (0x507290): `jmp CAEScriptAudioEntity::PreloadMissionAudio` on `this + 0x2A0`.
#[hook(addr = 0x507290, conv = thiscall, name = "CAudioEngine::PreloadMissionAudio", expect = "81 C1 A0 02 00 00")]
pub fn preload_mission_audio(this: GPtr<u8>, slot: u32, event: u32) {
    t_preload_mission_audio(this.byte_add(0x2A0), slot, event)
}

/// `CAudioEngine::GetMissionAudioLoadingStatus` (0x5072A0): `jmp CAEScriptAudioEntity::GetMissionAudioLoadingStatus` on `this + 0x2A0`. AL is the target's (`bool` / `int8`; tail jump).
#[hook(addr = 0x5072A0, conv = thiscall, name = "CAudioEngine::GetMissionAudioLoadingStatus", expect = "81 C1 A0 02 00 00")]
pub fn get_mission_audio_loading_status(this: GPtr<u8>, slot: u32) -> u8 {
    t_get_mission_audio_loading_status(this.byte_add(0x2A0), slot)
}

/// `CAudioEngine::PlayLoadedMissionAudio` (0x5072B0): `jmp CAEScriptAudioEntity::PlayLoadedMissionAudio` on `this + 0x2A0`.
#[hook(addr = 0x5072B0, conv = thiscall, name = "CAudioEngine::PlayLoadedMissionAudio", expect = "81 C1 A0 02 00 00")]
pub fn play_loaded_mission_audio(this: GPtr<u8>, slot: u32) {
    t_play_loaded_mission_audio(this.byte_add(0x2A0), slot)
}

/// `CAudioEngine::IsMissionAudioSampleFinished` (0x5072C0): `jmp CAEScriptAudioEntity::IsMissionAudioSampleFinished` on `this + 0x2A0`. AL is the target's (`bool` / `int8`; tail jump).
#[hook(addr = 0x5072C0, conv = thiscall, name = "CAudioEngine::IsMissionAudioSampleFinished", expect = "81 C1 A0 02 00 00")]
pub fn is_mission_audio_sample_finished(this: GPtr<u8>, slot: u32) -> u8 {
    t_is_mission_audio_sample_finished(this.byte_add(0x2A0), slot)
}

/// `CAudioEngine::GetMissionAudioEvent` (0x5072D0): `jmp CAEScriptAudioEntity::GetMissionAudioEvent` on `this + 0x2A0`. EAX is the target's (tail jump).
#[hook(addr = 0x5072D0, conv = thiscall, name = "CAudioEngine::GetMissionAudioEvent", expect = "81 C1 A0 02 00 00")]
pub fn get_mission_audio_event(this: GPtr<u8>, slot: u32) -> u32 {
    t_get_mission_audio_event(this.byte_add(0x2A0), slot)
}

/// `CAudioEngine::GetMissionAudioPosition` (0x5072E0): `jmp CAEScriptAudioEntity::GetMissionAudioPosition` on `this + 0x2A0`. EAX is the target's (tail jump).
#[hook(addr = 0x5072E0, conv = thiscall, name = "CAudioEngine::GetMissionAudioPosition", expect = "81 C1 A0 02 00 00")]
pub fn get_mission_audio_position(this: GPtr<u8>, slot: u32) -> u32 {
    t_get_mission_audio_position(this.byte_add(0x2A0), slot)
}

/// `CAudioEngine::ClearMissionAudio` (0x5072F0): `jmp CAEScriptAudioEntity::ClearMissionAudio` on `this + 0x2A0`.
#[hook(addr = 0x5072F0, conv = thiscall, name = "CAudioEngine::ClearMissionAudio", expect = "81 C1 A0 02 00 00")]
pub fn clear_mission_audio(this: GPtr<u8>, slot: u32) {
    t_clear_mission_audio(this.byte_add(0x2A0), slot)
}

/// `CAudioEngine::SetMissionAudioPosition` (0x507300): `jmp CAEScriptAudioEntity::SetMissionAudioPosition` on `this + 0x2A0`.
#[hook(addr = 0x507300, conv = thiscall, name = "CAudioEngine::SetMissionAudioPosition", expect = "81 C1 A0 02 00 00")]
pub fn set_mission_audio_position(this: GPtr<u8>, slot: u32, pos: u32) {
    t_set_mission_audio_position(this.byte_add(0x2A0), slot, pos)
}

/// `CAudioEngine::AttachMissionAudioToPed` (0x507310): `jmp CAEScriptAudioEntity::AttachMissionAudioToPhysical` on `this + 0x2A0`.
#[hook(addr = 0x507310, conv = thiscall, name = "CAudioEngine::AttachMissionAudioToPed", expect = "81 C1 A0 02 00 00")]
pub fn attach_mission_audio_to_ped(this: GPtr<u8>, slot: u32, ped: u32) {
    t_attach_mission_audio_to_ped(this.byte_add(0x2A0), slot, ped)
}

/// `CAudioEngine::AttachMissionAudioToObject` (0x507320): `jmp CAEScriptAudioEntity::AttachMissionAudioToPhysical` on `this + 0x2A0`.
#[hook(addr = 0x507320, conv = thiscall, name = "CAudioEngine::AttachMissionAudioToObject", expect = "81 C1 A0 02 00 00")]
pub fn attach_mission_audio_to_object(this: GPtr<u8>, slot: u32, object: u32) {
    t_attach_mission_audio_to_object(this.byte_add(0x2A0), slot, object)
}

/// `CAudioEngine::AttachMissionAudioToPhysical` (0x507330): `jmp CAEScriptAudioEntity::AttachMissionAudioToPhysical` on `this + 0x2A0`.
#[hook(addr = 0x507330, conv = thiscall, name = "CAudioEngine::AttachMissionAudioToPhysical", expect = "81 C1 A0 02 00 00")]
pub fn attach_mission_audio_to_physical(this: GPtr<u8>, slot: u32, physical: u32) {
    t_attach_mission_audio_to_physical(this.byte_add(0x2A0), slot, physical)
}

/// `CAudioEngine::ReportMissionAudioEvent-vec` (0x507340): `jmp CAEScriptAudioEntity::ReportMissionAudioEvent-2` on `this + 0x2A0`.
#[hook(addr = 0x507340, conv = thiscall, name = "CAudioEngine::ReportMissionAudioEvent-vec", expect = "81 C1 A0 02 00 00")]
pub fn report_mission_audio_event_vec(this: GPtr<u8>, event: u32, pos: u32) {
    t_report_mission_audio_event_vec(this.byte_add(0x2A0), event, pos)
}

/// `CAudioEngine::ReportMissionAudioEvent-phs` (0x5073B0): `jmp CAEScriptAudioEntity::ReportMissionAudioEvent-1` on `this + 0x2A0`.
#[hook(addr = 0x5073B0, conv = thiscall, name = "CAudioEngine::ReportMissionAudioEvent-phs", expect = "81 C1 A0 02 00 00")]
pub fn report_mission_audio_event_physical(this: GPtr<u8>, event: u32, physical: u32, a3: u32, a4: u32) {
    t_report_mission_audio_event_physical(this.byte_add(0x2A0), event, physical, a3, a4)
}

/// `CAudioEngine::SayPedless` (0x5073C0): `jmp CAEPedlessSpeechAudioEntity::AddSayEvent` on `this + 0x1E38`.
#[hook(addr = 0x5073C0, conv = thiscall, name = "CAudioEngine::SayPedless", expect = "81 C1 38 1E 00 00")]
pub fn say_pedless(
    this: GPtr<u8>,
    event: u32,
    ctx: u32,
    attach_to: u32,
    delay_ms: u32,
    probability: u32,
    override_silence: u32,
    force_audible: u32,
    frontend: u32,
) {
    t_say_pedless(
        this.byte_add(0x1E38),
        event,
        ctx,
        attach_to,
        delay_ms,
        probability,
        override_silence,
        force_audible,
        frontend,
    )
}

/// `CAudioEngine::PauseAllSounds` (0x507430): `jmp CAEAudioHardware::PauseAllSounds` on `AE_AUDIO_HARDWARE`.
#[hook(addr = 0x507430, conv = thiscall, name = "CAudioEngine::PauseAllSounds", expect = "B9 B8 F8 B5 00")]
pub fn pause_all_sounds(_this: GPtr<u8>) {
    t_pause_all_sounds(g(AE_AUDIO_HARDWARE))
}

/// `CAudioEngine::ResumeAllSounds` (0x507440): `jmp CAEAudioHardware::ResumeAllSounds` on `AE_AUDIO_HARDWARE`.
#[hook(addr = 0x507440, conv = thiscall, name = "CAudioEngine::ResumeAllSounds", expect = "B9 B8 F8 B5 00")]
pub fn resume_all_sounds(_this: GPtr<u8>) {
    t_resume_all_sounds(g(AE_AUDIO_HARDWARE))
}
