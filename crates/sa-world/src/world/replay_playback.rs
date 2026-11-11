//! `CReplay::FinishPlayback` (batch `mvpi-world-1`): leaving a replay. Ported from the 1.0 US bytes;
//! unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{find_player_vehicle, g};

/// `CReplay::Mode` (u8; 1 = playback)
const MODE: usize = 0xA43088;
/// `CReplay::bDoLoadSceneWhenDone` (u8) and the three dwords of the load-scene position (0x97FAC0..)
const DO_LOAD_SCENE_WHEN_DONE: usize = 0x97FACC;
/// The byte that asks for `Init` + `MarkEverythingAsNew` after playback (0x97FAE1)
const REINIT_WHEN_DONE: usize = 0x97FAE1;
/// The radio station saved when the replay started (0x97FABC)
const OLD_RADIO_STATION: usize = 0x97FABC;
/// `CGame::currLevel`
const CURR_LEVEL: usize = 0xB7290C;
/// `TheCamera`, `gFireManager`, `AudioEngine`
const THE_CAMERA: usize = 0xB6F028;
const FIRE_MANAGER: usize = 0xB71F80;
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `FrontEndMenuManager.m_nSfxVolume` / `m_nRadioVolume` (u8)
const SFX_VOLUME: usize = 0xBA6797;
const RADIO_VOLUME: usize = 0xBA6798;

orig! {
    /// `CReplay::RestoreStuffFromMem()` (0x45ECD0)
    fn restore_stuff_from_mem() = cdecl 0x45ECD0 => unported;
    /// `CTheZones::GetLevelFromPosition(const CVector&)` (0x572300)
    fn get_level_from_position(p: GPtr<u32>) -> u32 = cdecl 0x572300 => unported;
    /// `CCollision::SortOutCollisionAfterLoad()` (0x411E30)
    fn sort_out_collision_after_load() = cdecl 0x411E30 => unported;
    /// `CStreaming::LoadScene(const CVector&)` (0x40EB70)
    fn load_scene(p: GPtr<u32>) = cdecl 0x40EB70 => unported;
    /// `CReplay::Init()` (0x45E220)
    fn replay_init() = cdecl 0x45E220 => unported;
    /// `CReplay::MarkEverythingAsNew()` (0x45D430)
    fn mark_everything_as_new() = cdecl 0x45D430 => unported;
    /// `CCamera::SetFadeColour(uint8, uint8, uint8)` (0x50BF00)
    fn camera_set_fade_colour(cam: GPtr<u8>, r: u32, gg: u32, b: u32) = thiscall 0x50BF00 => unported;
    /// `CCamera::Fade(float, int16)` (0x50AC20)
    fn camera_fade(cam: GPtr<u8>, t: u32, dir: u32) = thiscall 0x50AC20 => unported;
    /// `CCamera::ProcessFade()` (0x50B5D0)
    fn camera_process_fade(cam: GPtr<u8>) = thiscall 0x50B5D0 => unported;
    /// `CFireManager::CreateAllFxSystems()` (0x539D50)
    fn fire_manager_create_all_fx_systems(m: GPtr<u8>) = thiscall 0x539D50 => unported;
    /// `CAudioEngine::SetEffectsMasterVolume(int8)` (0x506E10)
    fn audio_set_effects_master_volume(e: GPtr<u8>, v: u32) = thiscall 0x506E10 => unported;
    /// `CAudioEngine::SetMusicMasterVolume(int8)` (0x506DE0)
    fn audio_set_music_master_volume(e: GPtr<u8>, v: u32) = thiscall 0x506DE0 => unported;
    /// `CAEVehicleAudioEntity::TurnOnRadioForVehicle()` (0x4F5B20)
    fn turn_on_radio_for_vehicle(a: GPtr<u8>) = thiscall 0x4F5B20 => unported;
    /// `CAudioEngine::RetuneRadio(int8)` (0x507E10)
    fn audio_retune_radio(e: GPtr<u8>, station: u32) = thiscall 0x507E10 => unported;
}

/// `CReplay::FinishPlayback()` (0x45F050, cdecl): only in playback mode (1). `RestoreStuffFromMem()`,
/// mode 0; with `bDoLoadSceneWhenDone` a stack vector of the dwords at 0x97FAC8, 0x97FAC4, 0x97FAC0 (in
/// that order) gives `CGame::currLevel = GetLevelFromPosition(v)`, then
/// `SortOutCollisionAfterLoad()` and `LoadScene(v)`; the flag is cleared. With the byte 0x97FAE1:
/// `Init()` and `MarkEverythingAsNew()`. The camera fades back in (`SetFadeColour(0, 0, 0)`, `Fade(0,
/// 0)`, `ProcessFade()`, `Fade(1.0, 1)`), the fires' FX systems are recreated, the sfx and radio master
/// volumes restored from the menu settings, and in a vehicle (`FindPlayerVehicle(-1, false)`, called
/// twice) its radio is turned on and retuned to the saved station.
#[hook(addr = 0x45f050, conv = cdecl, name = "CReplay::FinishPlayback", expect = "A0 88 30 A4 00",
       unverified = "mvpi-world-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn finish_playback() {
    if g::<u8>(MODE).read() != 1 {
        return;
    }
    restore_stuff_from_mem();
    let load = g::<u8>(DO_LOAD_SCENE_WHEN_DONE).read();
    g::<u8>(MODE).write(0);
    if load != 0 {
        let mut v = [g::<u32>(0x97FAC8).read(), g::<u32>(0x97FAC4).read(), g::<u32>(0x97FAC0).read()];
        let p = GPtr::from_raw(v.as_mut_ptr());
        g::<u32>(CURR_LEVEL).write(get_level_from_position(p));
        sort_out_collision_after_load();
        load_scene(p);
    }
    let reinit = g::<u8>(REINIT_WHEN_DONE).read();
    g::<u8>(DO_LOAD_SCENE_WHEN_DONE).write(0);
    if reinit != 0 {
        replay_init();
        mark_everything_as_new();
    }
    let cam = GPtr::from_addr(THE_CAMERA);
    camera_set_fade_colour(cam, 0, 0, 0);
    camera_fade(cam, 0, 0);
    camera_process_fade(cam);
    camera_fade(cam, 0x3F80_0000, 1);
    fire_manager_create_all_fx_systems(GPtr::from_addr(FIRE_MANAGER));
    let ae = GPtr::from_addr(AUDIO_ENGINE);
    audio_set_effects_master_volume(ae, u32::from(g::<u8>(SFX_VOLUME).read()));
    audio_set_music_master_volume(ae, u32::from(g::<u8>(RADIO_VOLUME).read()));
    if !find_player_vehicle(-1, 0).is_null() {
        let veh = find_player_vehicle(-1, 0);
        turn_on_radio_for_vehicle(veh.byte_add(0x138));
        audio_retune_radio(ae, u32::from(g::<u8>(OLD_RADIO_STATION).read()));
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_FINISH_PLAYBACK];
