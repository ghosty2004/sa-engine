//! Entering and leaving a replay: `CReplay::TriggerPlayback`, the saved game state (`StoreStuffInMem` /
//! `RestoreStuffFromMem`) and `FinishPlayback`.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{find_player_vehicle, g, rd};

/// `CReplay::Mode` (u8; 1 = playback)
const MODE: usize = 0xA43088;
/// `CReplay::bDoLoadSceneWhenDone` (u8) and the three dwords of the load-scene position (0x97FAC0..)
const DO_LOAD_SCENE_WHEN_DONE: usize = 0x97FACC;
/// The byte that asks for `Init` + `MarkEverythingAsNew` after playback (0x97FAE1)
const REINIT_WHEN_DONE: usize = 0x97FAE1;
/// The radio station saved when the replay started (0x97FABC)
const OLD_RADIO_STATION: usize = 0x97FABC;
/// `CGame::currLevel`, `CGame::currArea`
const CURR_LEVEL: usize = 0xB7290C;
const CURR_AREA: usize = 0xB7_2914;
/// `TheCamera`, `gFireManager`, `AudioEngine`
const THE_CAMERA: usize = 0xB6F028;
const FIRE_MANAGER: usize = 0xB71F80;
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `FrontEndMenuManager.m_nSfxVolume` / `m_nRadioVolume` (u8)
const SFX_VOLUME: usize = 0xBA6797;
const RADIO_VOLUME: usize = 0xBA6798;

use crate::bindings::audio_retune_radio;
use sa_core::bindings::audio_set_effects_master_volume;
use sa_core::bindings::audio_set_music_master_volume;
use crate::bindings::get_level_from_position;
use crate::bindings::set_game_clock;
use crate::bindings::mark_everything_as_new;
use sa_core::bindings::camera_process_fade;
use sa_core::bindings::camera_set_fade_colour;
use sa_core::bindings::collision_sort_out_collision_after_load as sort_out_collision_after_load;
use sa_core::bindings::replay_init;
use crate::bindings::turn_on_radio_for_vehicle;
use sa_core::bindings::camera_fade;
use sa_core::bindings::streaming_load_scene as load_scene;
orig! {
    /// `CReplay::RestoreStuffFromMem()` (0x45ECD0)
    fn restore_stuff_from_mem() = cdecl 0x45ECD0 => replay_restore_stuff_from_mem;
    /// `CPlayerPedData::DeAllocateData()` (0x56F890)
    fn player_ped_data_deallocate(d: GPtr<u8>) = thiscall 0x56F890 => unported;
    /// `CFireManager::CreateAllFxSystems()` (0x539D50)
    fn fire_manager_create_all_fx_systems(m: GPtr<u8>) = thiscall 0x539D50 => unported;
    /// `CFireManager::DestroyAllFxSystems()` (0x539D10)
    fn fire_manager_destroy_all_fx_systems(m: GPtr<u8>) = thiscall 0x539D10 => unported;
    /// `CPlaneBanners::Init()` (0x717820)
    fn plane_banners_init() = cdecl 0x717820 => unported;
}

/// The timer values saved for a replay: (`CTimer` cell, its copy).
const TIMER_CELLS: [(usize, usize); 11] = [
    (0xB7_CB84, 0x97_FB60), (0xB7_CB80, 0x97_FB5C), (0xB7_CB78, 0x97_FB58), (0xB7_CB74, 0x97_FB54),
    (0xB7_CB70, 0x97_FB50), (0xB7_CB6C, 0x97_FB4C), (0xB7_CB7C, 0x97_FB48), (0xB7_CB4C, 0x97_FB44),
    (0xB7_CB64, 0x97_FB3C), (0xB7_CB5C, 0x97_FB38), (0xB7_CB58, 0x97_FB34),
];
/// The population counters saved and restored: (`CPopulation` cell, its copy).
const POPULATION_CELLS: [(usize, usize); 9] = [
    (0xC0_EC70, 0x97_FAB4), (0xC0_EC6C, 0x97_FAB0), (0xC0_EC68, 0x97_FAAC), (0xC0_EC64, 0x97_FAA8),
    (0xC0_EC34, 0x97_FA78), (0xC0_EC30, 0x97_FA74), (0xC0_EC2C, 0x97_FA70), (0xC0_EC28, 0x97_FA6C),
    (0xC0_EC24, 0x97_FA68),
];
/// The eleven counters from 0xC0EC38 that are saved (from 0x97FA7C) but never restored: (first cell, first copy, n).
const POPULATION_SAVED_ONLY: (usize, usize, usize) = (0xC0_EC38, 0x97_FA7C, 11);
/// The `CPlayerInfo` dwords cleared after a copy (the live one's by the store, the saved one's by the restore).
const PLAYER_INFO_CLEARED: [usize; 4] = [4, 8, 0xC, 0x184];
/// `CWorld::Players[0]` and its saved copy.
const PLAYER_INFO: usize = 0xB7_CD98;
const STORED_PLAYER_INFO: usize = 0xA4_30B0;
/// `CReplay::pStoredCam`.
const STORED_CAM: usize = 0x97_FAF8;
/// The size of `TheCamera` in dwords.
const CAMERA_DWORDS: usize = 0x35E;

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
        g::<u32>(CURR_LEVEL).write(get_level_from_position(p.cast::<f32>()) as u32);
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
    camera_fade(cam, f32::from_bits(0), 0);
    camera_process_fade(cam);
    camera_fade(cam, f32::from_bits(0x3F80_0000), 1);
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

/// `CReplay::EnableReplays()` (stub 0x45B160 → 0x1568BC0; STD-7): `CReplay::bReplayEnabled` (0x8A6160) = 1.
#[hook(addr = 0x45B160, conv = cdecl, name = "CReplay::EnableReplays", expect = "E9 5B DA 10 01",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn enable_replays() {
    g::<u8>(0x8A6160).write(1);
}

/// `CReplay::RestoreStuffFromMem()` (0x45ECD0, cdecl). Every pool ped (stride 0x7C4) and vehicle (0xA18) made for the
/// replay (`bUsedForReplay`: ped +0x478 bit 21, vehicle +0x42F bit 3) is removed from the world and deleted (a ped's
/// player data deallocated first), every other one added back; the pool's size is re-read after each. `TheCamera` is
/// copied back from `pStoredCam` (0x35E dwords), which is freed and cleared; `CWorld::Players[0] = stored player info`
/// (0xA430B0, through its `operator=`), after which the stored copy's +4 / +8 / +0xC / +0x184 are zeroed (the bytes;
/// gta-reversed has none of it); the eleven timer values; `SetGameClock(hours, ++minutes, 0)`; the weather types and
/// interpolation, `CGame::currArea` (a byte, zero-extended) and the nine population counters.
#[hook(addr = 0x45ECD0, conv = cdecl, name = "CReplay::RestoreStuffFromMem", expect = "A1 90 44 B7 00 8B 48 08",
       unverified = "frontier2-a19", route = "L2", hazards = "heap-writes",
       note = "leaving a replay (FinishPlayback)")]
pub fn replay_restore_stuff_from_mem() {
    use super::pickups::{entity_vdelete, world_remove};
    use sa_core::bindings::{crt_free_8214bd, world_add};
    for (pool_cell, stride, replay) in [(0xB7_4490usize, 0x7C4usize, true), (0xB7_4494, 0xA18, false)] {
        let mut i = 0;
        while i < rd::<i32>(g::<GPtr<u8>>(pool_cell).read(), 8) {
            let pool = g::<GPtr<u8>>(pool_cell).read();
            let e = rd::<GPtr<u8>>(pool, 0).byte_add((i as usize * stride) as isize);
            if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) >= 0 && !e.is_null() {
                let used = if replay { rd::<u32>(e, 0x478) & 0x20_0000 != 0 } else { rd::<u8>(e, 0x42F) & 8 != 0 };
                if used {
                    if replay && !rd::<GPtr<u8>>(e, 0x480).is_null() {
                        player_ped_data_deallocate(rd::<GPtr<u8>>(e, 0x480));
                    }
                    world_remove(e);
                    entity_vdelete(e, 1);
                } else {
                    world_add(e);
                }
            }
            i += 1;
        }
    }
    let stored = g::<GPtr<u32>>(STORED_CAM).read();
    for k in 0..CAMERA_DWORDS {
        g::<u32>(THE_CAMERA + 4 * k).write(stored.wrapping_index(k as i32).read());
    }
    crt_free_8214bd(stored.cast());
    g::<u32>(STORED_CAM).write(0);
    super::frontier_c::player_info_assign(GPtr::from_addr(PLAYER_INFO), GPtr::from_addr(STORED_PLAYER_INFO));
    for (live, stored) in TIMER_CELLS.into_iter().chain(POPULATION_CELLS) {
        g::<u32>(live).write(g::<u32>(stored).read());
    }
    let minutes = g::<u8>(0x97_FAEE).read().wrapping_add(1);
    let hours = g::<u8>(0x97_FAEF).read();
    for off in PLAYER_INFO_CLEARED {
        g::<u32>(STORED_PLAYER_INFO + off).write(0);
    }
    g::<u8>(0x97_FAEE).write(minutes);
    set_game_clock(u32::from(hours), u32::from(minutes), 0);
    g::<u16>(0xC8_1320).write(g::<u16>(0x97_FAEC).read());
    g::<u16>(0xC8_131C).write(g::<u16>(0x97_FAE8).read());
    g::<u32>(0xC8_130C).write(g::<u32>(0x97_FAE4).read());
    g::<u32>(CURR_AREA).write(u32::from(g::<u8>(0x97_FAB8).read()));
}

/// `CReplay::StoreStuffInMem()` (stub 0x45F180 → 0x15689D0, cdecl), the counterpart of
/// [`replay_restore_stuff_from_mem`]: `TheCamera` copied into a new `pStoredCam` (`operator new(0xD78)`), the player
/// info into its saved copy (`operator=`), after which the live one's +4 / +8 / +0xC / +0x184 / +0x4C / +0x50 are
/// cleared; the timers, the clock's hours (0xB70153) and minutes (0xB70152), the weather types and interpolation and
/// twenty population counters (nine of them restored later).
#[hook(addr = 0x45F180, conv = cdecl, name = "CReplay::StoreStuffInMem", expect = "E9 4B 98 10 01",
       unverified = "frontier2-b31", route = "L2", hazards = "heap-writes",
       note = "entering a replay (TriggerPlayback)")]
pub fn replay_store_stuff_in_mem() {
    use sa_core::bindings::crt_new_821195;
    let stored = crt_new_821195((CAMERA_DWORDS * 4) as u32).cast::<u32>();
    g::<GPtr<u32>>(STORED_CAM).write(stored);
    for k in 0..CAMERA_DWORDS {
        stored.wrapping_index(k as i32).write(g::<u32>(THE_CAMERA + 4 * k).read());
    }
    super::frontier_c::player_info_assign(GPtr::from_addr(STORED_PLAYER_INFO), GPtr::from_addr(PLAYER_INFO));
    for off in PLAYER_INFO_CLEARED.into_iter().chain([0x4C, 0x50]) {
        g::<u32>(PLAYER_INFO + off).write(0);
    }
    let (live, saved, n) = POPULATION_SAVED_ONLY;
    let saved_only = (0..n).map(|k| (live + 4 * k, saved + 4 * k));
    for (live, stored) in TIMER_CELLS.into_iter().chain(POPULATION_CELLS).chain(saved_only) {
        g::<u32>(stored).write(g::<u32>(live).read());
    }
    g::<u8>(0x97_FAEF).write(g::<u8>(0xB7_0153).read());
    g::<u8>(0x97_FAEE).write(g::<u8>(0xB7_0152).read());
    g::<u16>(0x97_FAEC).write(g::<u16>(0xC8_1320).read());
    g::<u16>(0x97_FAE8).write(g::<u16>(0xC8_131C).read());
    g::<u32>(0x97_FAE4).write(g::<u32>(0xC8_130C).read());
}

/// `CReplay::TriggerPlayback(eReplayCamMode mode, CVector fixedCamPos, bool loadScene)` (stub 0x4600F0 → 0x156D320,
/// cdecl). Only while recording (mode 0).
///
/// `CSpecialFX::ReplayStarted()`; the camera mode byte (0x97FB30) and position (x 0x97FB2C, y 0x97FB28, z 0x97FB24)
/// stored, playback mode, 0x97FAE1 = 0, 0x97FAE0 = 1, 0x97FAD0 = 0. In a vehicle the current radio station
/// (`GetCurrentRadioStationID`, the protection's computed call) is saved sign-extended and the radio stopped, else 0
/// is saved; the area byte saved. The playback slot is the first buffer after the recording one (status 2; with none
/// the search starts at 9 & 7 = 1) whose status is 1 or 2; its base and a zero offset form the playback address.
/// Temporary objects within 999999.875 of the origin are deleted, the game state stored, the ped then vehicle
/// conversion tables built, the pools emptied, the skid marks cleared, the recorded models streamed and the player ped
/// created into `Players[0]`. Without `loadScene` the camera position (its matrix's, else its placement's; the
/// protection decodes the `TheCamera.m_matrix` cell) is kept for the load scene after playback and the first focus
/// coordinate sets the level, sorts out collision and loads the scene; with it nothing is loaded. Then the cross
/// hairs, explosions, plane banners and the fires' FX systems (ECX = `gFireManager` from the protection's VM) are
/// cleared, the camera restored, the FOV set to 70 and the screen faded in from black over 1.5 s.
#[hook(addr = 0x4600F0, conv = cdecl, name = "CReplay::TriggerPlayback", expect = "E9 2B D2 10 01",
       unverified = "frontier2-b31", route = "L2", hazards = "heap-writes",
       note = "replay start (F1 while recording)")]
pub fn trigger_playback(mode: u32, x: f32, y: f32, z: f32, keep_scene: u32) {
    use super::frontier_b3::replay_stream_all_necessary_cars_and_peds;
    use super::mvpd_w4::weapon_effects_clear_cross_hairs;
    use super::replay_peds::create_player_ped;
    use super::replay_setup::{
        empty_peds_and_vehicle_pools_no_destructors, initialise_ped_pool_conversion_table,
        initialise_vehicle_pool_conversion_table, special_fx_replay_started, BUFFERS, BUFFER_SIZE, BUFFER_STATUS,
    };
    use crate::bindings::{
        audio_stop_radio, camera_restore, explosion_clear_all, object_delete_all_temp_objects_in_area,
        radio_get_current_radio_station_id_call,
    };
    use sa_core::bindings::{draw_set_fov, skidmarks_clear};

    if g::<u8>(MODE).read() != 0 {
        return;
    }
    special_fx_replay_started();
    g::<u8>(0x97_FB30).write(mode as u8);
    g::<u8>(MODE).write(1);
    g::<f32>(0x97_FB2C).write(x);
    g::<f32>(0x97_FB28).write(y);
    g::<f32>(0x97_FB24).write(z);
    g::<u8>(REINIT_WHEN_DONE).write(0);
    g::<u8>(0x97_FAE0).write(1);
    g::<u32>(0x97_FAD0).write(0);
    if find_player_vehicle(-1, 0).is_null() {
        g::<u32>(OLD_RADIO_STATION).write(0);
    } else {
        let station = radio_get_current_radio_station_id_call(GPtr::from_addr(0x8C_B6F8));
        g::<i32>(OLD_RADIO_STATION).write(i32::from(station as i8));
        audio_stop_radio(GPtr::from_addr(AUDIO_ENGINE), 0, 0);
    }
    g::<u8>(0x97_FAB8).write(g::<u8>(CURR_AREA).read());
    let status = |i: u32| g::<u8>(BUFFER_STATUS + i as usize).read();
    let mut slot = (0..8).find(|&i| status(i) == 2).unwrap_or(8).wrapping_add(1) & 7;
    while status(slot) != 2 && status(slot) != 1 {
        slot = (slot + 1) & 7;
    }
    g::<u8>(0x97_FB6C).write(slot as u8);
    g::<u32>(0x97_FB68).write((BUFFERS + BUFFER_SIZE * slot as usize) as u32);
    g::<u32>(0x97_FB64).write(0);
    object_delete_all_temp_objects_in_area(0.0, 0.0, 0.0, f32::from_bits(0x4974_23FE));
    replay_store_stuff_in_mem();
    initialise_ped_pool_conversion_table();
    initialise_vehicle_pool_conversion_table();
    empty_peds_and_vehicle_pools_no_destructors();
    skidmarks_clear();
    replay_stream_all_necessary_cars_and_peds();
    g::<GPtr<u8>>(PLAYER_INFO).write(create_player_ped());
    if keep_scene as u8 == 0 {
        g::<u8>(DO_LOAD_SCENE_WHEN_DONE).write(1);
        let matrix = g::<GPtr<u8>>(THE_CAMERA + 0x14).read();
        let pos = if matrix.is_null() { GPtr::<u8>::from_addr(THE_CAMERA + 4) } else { matrix.byte_add(0x30) };
        g::<u32>(0x97_FAC8).write(rd::<u32>(pos, 0));
        g::<u32>(0x97_FAC4).write(rd::<u32>(pos, 4));
        g::<u32>(0x97_FAC0).write(rd::<u32>(pos, 8));
        let mut focus = [0u32; 3];
        let p = GPtr::from_raw(focus.as_mut_ptr());
        super::replay_setup::find_first_focus_coordinate(p.cast());
        g::<u32>(CURR_LEVEL).write(get_level_from_position(p.cast::<f32>()) as u32);
        sort_out_collision_after_load();
        load_scene(p);
    } else {
        g::<u8>(DO_LOAD_SCENE_WHEN_DONE).write(0);
    }
    weapon_effects_clear_cross_hairs();
    explosion_clear_all();
    plane_banners_init();
    fire_manager_destroy_all_fx_systems(GPtr::from_addr(FIRE_MANAGER));
    let cam = GPtr::from_addr(THE_CAMERA);
    camera_restore(cam);
    draw_set_fov(70.0);
    camera_set_fade_colour(cam, 0, 0, 0);
    camera_fade(cam, 0.0, 0);
    camera_process_fade(cam);
    camera_fade(cam, 1.5, 1);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_FINISH_PLAYBACK,
    HOOK_ENABLE_REPLAYS,
    HOOK_REPLAY_RESTORE_STUFF_FROM_MEM,
    HOOK_REPLAY_STORE_STUFF_IN_MEM,
    HOOK_TRIGGER_PLAYBACK,
];
