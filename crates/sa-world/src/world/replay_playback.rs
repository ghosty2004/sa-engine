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
    use sa_core::field::rd;
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
    let stored = g::<GPtr<u32>>(0x97_FAF8).read();
    for k in 0..0x35E {
        g::<u32>(THE_CAMERA + 4 * k).write(stored.wrapping_index(k as i32).read());
    }
    crt_free_8214bd(stored.cast());
    g::<u32>(0x97_FAF8).write(0);
    super::frontier_c::player_info_assign(GPtr::from_addr(0xB7_CD98), GPtr::from_addr(0xA4_30B0));
    for (dst, src) in [
        (0xB7_CB84usize, 0x97_FB60usize), (0xB7_CB78, 0x97_FB58), (0xB7_CB80, 0x97_FB5C), (0xB7_CB74, 0x97_FB54),
        (0xB7_CB7C, 0x97_FB48), (0xB7_CB5C, 0x97_FB38), (0xB7_CB6C, 0x97_FB4C), (0xB7_CB64, 0x97_FB3C),
        (0xB7_CB70, 0x97_FB50), (0xB7_CB4C, 0x97_FB44), (0xB7_CB58, 0x97_FB34),
    ] {
        g::<u32>(dst).write(g::<u32>(src).read());
    }
    let minutes = g::<u8>(0x97_FAEE).read().wrapping_add(1);
    let hours = g::<u8>(0x97_FAEF).read();
    for z in [0xA4_30B4usize, 0xA4_30B8, 0xA4_30BC, 0xA4_3234] {
        g::<u32>(z).write(0);
    }
    g::<u8>(0x97_FAEE).write(minutes);
    set_game_clock(u32::from(hours), u32::from(minutes), 0);
    g::<u16>(0xC8_1320).write(g::<u16>(0x97_FAEC).read());
    g::<u16>(0xC8_131C).write(g::<u16>(0x97_FAE8).read());
    g::<u32>(0xC8_130C).write(g::<u32>(0x97_FAE4).read());
    g::<u32>(0xB7_2914).write(u32::from(g::<u8>(0x97_FAB8).read()));
    for (dst, src) in [
        (0xC0_EC70usize, 0x97_FAB4usize), (0xC0_EC6C, 0x97_FAB0), (0xC0_EC68, 0x97_FAAC), (0xC0_EC64, 0x97_FAA8),
        (0xC0_EC34, 0x97_FA78), (0xC0_EC30, 0x97_FA74), (0xC0_EC2C, 0x97_FA70), (0xC0_EC28, 0x97_FA6C),
        (0xC0_EC24, 0x97_FA68),
    ] {
        g::<u32>(dst).write(g::<u32>(src).read());
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_FINISH_PLAYBACK, HOOK_ENABLE_REPLAYS, HOOK_REPLAY_RESTORE_STUFF_FROM_MEM];
