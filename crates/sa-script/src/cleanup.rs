//! `CMissionCleanup` list maintenance (PORT-SCRIPT-3): `AddEntityToList`, `RemoveEntityFromList`.
//! Follows the 1.0 US bytes (`.HOODLUM` bodies behind the stubs) and `docs/re/specs/SCRIPT/CMissionCleanup.md`.
//!
//! Layout (gr `MissionCleanup.h`, size 0x25C): 75 entries `{ u8 type +0, i32 handle +4 }` (stride 8), `i8
//! m_Count` at +0x258. Entry types: 1 vehicle, 2 ped, 3 object (others are not looked up here).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// Number of entries.
const ENTRIES: u16 = 0x4B;
/// `m_Count` (i8).
const COUNT: isize = 0x258;

use sa_world::bindings::entity_get_is_static;
use sa_world::bindings::physical_add_to_moving_list;

/// `CMissionCleanup::AddEntityToList(int32 handle, MissionCleanUpEntityType type)` (stub 0x4637E0 → 0x1565D40,
/// `ret 8`): the first free entry (type 0, u16 counter below 75) gets `handle` (+4), then the low byte of
/// `type` (+0); `m_Count` + 1 (byte, wraps). A full list changes nothing. Duplicates are not checked. The
/// body never writes ECX (`preserve = "ecx"`).
#[hook(addr = 0x4637E0, conv = thiscall, name = "CMissionCleanup::AddEntityToList", expect = "E9 5B 25 10 01", this = "CMissionCleanup", preserve = "ecx")]
pub fn add_entity_to_list(m: GPtr<u8>, handle: i32, ty: u32) {
    let mut i: u16 = 0;
    while i < ENTRIES {
        let e = m.byte_add(8 * i as isize);
        if e.read() == 0 {
            e.byte_add(4).cast::<i32>().write(handle);
            e.write(ty as u8);
            let c = m.byte_add(COUNT);
            c.write(c.read().wrapping_add(1));
            return;
        }
        i += 1;
    }
}

/// `CMissionCleanup::RemoveEntityFromList(int32 handle, MissionCleanUpEntityType type)` (stub 0x4654B0 →
/// 0x156E810, `ret 8`): every one of the 75 entries (no early exit) whose type byte equals the low byte of
/// `type` and whose handle equals `handle`: types 1/2/3 look the handle up in the vehicle/ped/object pool
/// (`idx = handle >> 8`, valid when `flags[idx] == (u8)handle`, `objects + idx * size`); a found entity with
/// `m_nFlags` (+0x1C) bit 0x40000 gets the bit cleared and, when `!GetIsStatic()`, `AddToMovingList()`.
/// Then the entry is cleared (handle, then type) and `m_Count` - 1 (byte). With type 0 every empty slot
/// whose handle matches is "removed" again (original behaviour).
#[hook(addr = 0x4654B0, conv = thiscall, name = "CMissionCleanup::RemoveEntityFromList", expect = "E9 5B 93 10 01", this = "CMissionCleanup")]
pub fn remove_entity_from_list(m: GPtr<u8>, handle: i32, ty: u32) {
    for i in 0..ENTRIES as isize {
        let e = m.byte_add(8 * i);
        let t = e.read();
        if t != ty as u8 || e.byte_add(4).cast::<i32>().read() != handle {
            continue;
        }
        let pool = match t {
            1 => Some((0xB74494usize, 0xA18i32)),
            2 => Some((0xB74490, 0x7C4)),
            3 => Some((0xB7449C, 0x19C)),
            _ => None,
        };
        if let Some((cell, size)) = pool {
            let p = GPtr::<GPtr<u8>>::from_addr(cell).read();
            let flags = p.byte_add(4).cast::<GPtr<u8>>().read();
            let idx = handle >> 8;
            if flags.byte_add(idx as isize).read() == handle as u8 {
                let objects = p.cast::<GPtr<u8>>().read();
                let ent = objects.byte_add(idx.wrapping_mul(size) as isize);
                if !ent.is_null() {
                    let fl = ent.byte_add(0x1C).cast::<u32>();
                    let v = fl.read();
                    if v & 0x40000 != 0 {
                        fl.write(v & 0xFFFB_FFFF);
                        if entity_get_is_static(ent) == 0 {
                            physical_add_to_moving_list(ent);
                        }
                    }
                }
            }
        }
        e.byte_add(4).cast::<i32>().write(0);
        e.write(0);
        let c = m.byte_add(COUNT);
        c.write(c.read().wrapping_sub(1));
    }
}

use crate::bindings::audio_engine_clear_mission_audio;
use sa_world::bindings::camera_initialise_scriptable_components;
use sa_world::bindings::camera_restore;
use crate::bindings::clean_up_this_object as scripts_clean_up_this_object;
use crate::bindings::clean_up_this_ped as scripts_clean_up_this_ped;
use crate::bindings::clean_up_this_vehicle as scripts_clean_up_this_vehicle;
use crate::bindings::clear_all_script_fire_flags as fire_manager_clear_all_script_fire_flags;
use crate::bindings::clear_all_suppressed_car_models as scripts_clear_all_suppressed_car_models;
use crate::bindings::clear_script_road_blocks as road_blocks_clear_script_road_blocks;
use crate::bindings::decision_maker_unload;
use crate::bindings::disable_random_trains as train_disable_random_trains;
use crate::bindings::enable_ped_speech_for_script_speech as ped_enable_ped_speech_for_script_speech;
use crate::bindings::force_group_to_always_follow as player_ped_force_group_to_always_follow;
use crate::bindings::make_player_group_reappear as player_ped_make_player_group_reappear;
use crate::bindings::markers_force_render;
use crate::bindings::pad_set_drunk_input_delay;
use crate::bindings::path_find_release_requested_nodes;
use crate::bindings::path_find_unmark_all_road_nodes_as_dont_wander;
use crate::bindings::phone_task_stop;
use crate::bindings::plane_switch_ambient_planes;
use crate::bindings::player_ped_force_group_to_never_follow;
use crate::bindings::release_mission_trains as train_release_mission_trains;
use crate::bindings::remove_script_checkpoint as scripts_remove_script_checkpoint;
use crate::bindings::remove_script_effect_system as scripts_remove_script_effect_system;
use crate::bindings::remove_script_texture_dictionary as scripts_remove_script_texture_dictionary;
use crate::bindings::stuck_car_check_init;
use sa_core::bindings::camera_fade;
use sa_core::bindings::camera_set_wide_screen_off;
use sa_core::bindings::clear_skip as game_logic_clear_skip;
use sa_core::bindings::clear_specific_zones_to_trigger_gang_war as gang_wars_clear_specific_zones;
use sa_core::bindings::find_player_ped as world_find_player_ped;
use sa_core::bindings::get_pad;
use sa_core::bindings::hud_set_help_message;
use sa_core::bindings::player_info_make_player_safe;
use sa_core::bindings::post_effects_script_reset_for_effects;
use sa_core::bindings::streaming_disable_cop_bikes;
use sa_core::bindings::time_cycle_stop_extra_colour;
use sa_core::bindings::zones_fill_zones_with_gang_colours;
use sa_stream::bindings::find_player_wanted as world_find_player_wanted;
use sa_stream::bindings::set_mission_doesnt_require_special_char as streaming_set_mission_doesnt_require_special_char;
use sa_world::bindings::all_resprays_close_or_open as garages_all_resprays_close_or_open;
use sa_world::bindings::enable_ped_speech as ped_enable_ped_speech;
use sa_world::bindings::fx_kill as fx_system_kill;
use sa_world::bindings::get_actual_script_thing_index as scripts_get_actual_script_thing_index;
use sa_world::bindings::group_intel_set_decision_maker_type;
use sa_world::bindings::group_intel_set_default_task_allocator_type;
use sa_world::bindings::sequence_flush;
use sa_world::bindings::tm_find_task_by_type as task_manager_find_task_by_type;
use sa_world::bindings::weather_release_weather;
use crate::bindings::cleanup_remove_entity_from_list;
use crate::bindings::clear_ped_type_as_acquaintance as ped_type_clear_ped_type_as_acquaintance;
use crate::bindings::remove_script_search_light as scripts_remove_script_search_light;
use sa_world::bindings::camera_reset_ducking_system;
use sa_world::bindings::get_ped_flag as ped_type_get_ped_flag;
use sa_world::bindings::ped_groups_remove_group;
orig! {
    /// `CPickups::RemoveMissionPickUps()` (0x456DE0)
    fn pickups_remove_mission_pickups() = cdecl 0x456DE0 => unported;
    /// `CPathFind::TidyUpNodeSwitchesAfterMission()` (0x44D3B0)
    fn path_find_tidy_up_node_switches_after_mission(pf: GPtr<u8>) = thiscall 0x44D3B0 => unported;
    /// `CVehicleRecording::RemoveAllRecordingsThatArentUsed()` (0x45A160)
    fn vehicle_recording_remove_unused() = cdecl 0x45A160 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CRestart::ClearRespawnPointForDurationOfMission()` (0x460840)
    fn restart_clear_respawn_point_for_mission() = cdecl 0x460840 => unported;
    /// `CPedType::ClearPedTypeAcquaintances(int32 acquaintance, ePedType)` (0x608A20)
    fn ped_type_clear_ped_type_acquaintances(id: i32, ty: i32) = cdecl 0x608A20 => unported;
}

/// `TheCamera` (0xB6F028)
const THE_CAMERA: usize = 0xB6F028;
/// `CWorld::Players` (0xB7CD98, 0x190 per player; `m_pPed` +0)
const PLAYERS: usize = 0xB7CD98;
/// `ThePaths` (0x96F050)
const THE_PATHS: usize = 0x96F050;
/// `AudioEngine` (0xB6BC90)
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `gFireManager` (0xB71F80)
const FIRE_MANAGER: usize = 0xB71F80;
/// The script stuck-car checker (0xA90AB0)
const STUCK_CARS: usize = 0xA90AB0;
/// `CPedGroups::ms_groups` (0xC09920, 0x2D4 per group; intelligence +0x30)
const PED_GROUPS: usize = 0xC09920;
/// `CTaskSequences::ms_taskSequence` (0xC178F0, 0x40 per sequence; flush flag +0x38, reference count +0x3C)
const TASK_SEQUENCES: usize = 0xC178F0;
/// `CTaskSequences::ms_bIsOpened` (0xC17898, one byte per sequence)
const SEQUENCE_OPEN: usize = 0xC17898;
/// `CTheScripts::ScriptSequenceTaskArray` (0xA43F68, 4 bytes per sequence; the "used" byte +0)
const SCRIPT_SEQUENCES: usize = 0xA43F68;
/// `CTheScripts::ScriptEffectSystemArray` (0xA44110, 8 per entry; the `FxSystem_c*` +4)
const SCRIPT_EFFECT_SYSTEMS: usize = 0xA44110;
/// The script things of type 6 (0xC3A1A0, one "used" byte per entry)
const SCRIPT_THING_6_USED: usize = 0xC3A1A0;

fn w8(a: usize, v: u8) {
    GPtr::<u8>::from_addr(a).write(v);
}
fn w16(a: usize, v: u16) {
    GPtr::<u16>::from_addr(a).write(v);
}
fn w32(a: usize, v: u32) {
    GPtr::<u32>::from_addr(a).write(v);
}
use sa_core::field::gr8 as r8;
fn player_ped() -> GPtr<u8> {
    GPtr::<GPtr<u8>>::from_addr(PLAYERS).read()
}
/// The player ped's `m_pPlayerData` (+0x480).
fn player_data() -> GPtr<u8> {
    player_ped().byte_add(0x480).cast::<GPtr<u8>>().read()
}
/// The pool slot of `handle` in the pool at `cell` (index = handle >> 8, its flags byte must equal the low byte of
/// the handle; no bounds check), else NULL.
fn pool_at(cell: usize, size: i32, handle: i32) -> GPtr<u8> {
    let p = GPtr::<GPtr<u8>>::from_addr(cell).read();
    let idx = handle >> 8;
    let flags = p.byte_add(4).cast::<GPtr<u8>>().read();
    if flags.byte_add(idx as isize).read() != handle as u8 {
        return GPtr::NULL;
    }
    p.cast::<GPtr<u8>>().read().byte_add(idx.wrapping_mul(size) as isize)
}

/// `CMissionCleanup::Process()` (0x468560, thiscall): the end-of-mission reset, in this order.
///
/// - Byte 0xA4448A set: cleared, and unless the byte at 0xB7CE74 is 1 or 2, `TheCamera.Fade(0.5, 1)` and pad 0's
///   byte +0x10E loses bit 0x20.
/// - `CTrain::DisableRandomTrains(false)`, `ReleaseMissionTrains()`, `CPlane::SwitchAmbientPlanes(true)`; floats
///   0x8D2530 and 0x8A5B20 = 1.0; byte 0x96AB91 set → cleared and `FillZonesWithGangColours(byte 0x96AB90 == 0)`.
/// - Dword 0x8D2534 = -1, byte 0x96AB93 = 0, `ClearSpecificZonesToTriggerGangWar()`, bytes 0xC0FCB2..B4 and 0x96C008
///   = 0, `AllRespraysCloseOrOpen(true)`, byte 0xC87ACD = 0, `FindPlayerWanted(-1)` +0x14 = 1.0,
///   `RemoveMissionPickUps()`, `ClearScriptRoadBlocks()`, `DisableCopBikes(false)`, bytes 0xB72994 = 1, 0xBB4A72 = 0,
///   0x8D0A24 = 1, `ThePaths` `ReleaseRequestedNodes` / `UnMarkAllRoadNodesAsDontWander` /
///   `TidyUpNodeSwitchesAfterMission`, `RemoveAllRecordingsThatArentUsed()`.
/// - Camera: `SetWideScreenOff()`, words 0xB6F06E..0xB6F074 = 0x32, 0x31, 3, 0x30, bytes 0xB6F06D and 0xB6FD18 = 0,
///   `InitialiseScriptableComponents()`, `ResetDuckingSystem(NULL)`; float 0x8CCB84 = 1.0, bytes 0xB6EC2C, 0x96A8A8
///   = 0, dword 0xA44490 = 0, bytes 0xC7C70C/D = 0, `CPostEffects::ScriptResetForEffects()`, byte 0x96A7C8 = 0 (after
///   the dword 0xB72914 was read); that dword 0 → `CTimeCycle::StopExtraColour(false)`.
/// - `AudioEngine.ClearMissionAudio(0..3)`, `CWeather::ReleaseWeather()`, dword 0xB728E0 = 99999,
///   `SetMissionDoesntRequireSpecialChar(0..9)`, `ClearAllSuppressedCarModels()`, dword 0xA4448C = -1, byte 0x9654B0
///   = 0, word 0xBAB1DC = 0xFFFF, bytes 0xBAA3F8..FB, 0xBA677C, 0xA444A1..A3 = 0, `C3dMarkers::ForceRender(false)`,
///   byte 0xA444A0 = 1, dwords 0xA4449C/0xA44498 = 0, bytes 0xA44494..96 = 1, 0xBA18D9 and 0xA44B66 = 0, words
///   0xA44B64/0xA44B60 = 0, bytes 0xA44489/0xA44488 = 1.
/// - The player's group (`ms_groups[playerData->m_nPlayerGroup]`, data +0x38): intelligence
///   `SetDefaultTaskAllocatorType(1)`, `SetGroupDecisionMakerType(-1)`, group +0x2C = the dword at 0x86C6C0;
///   player data +0x34 loses bit 0x100; `ForceGroupToAlwaysFollow(false)`, `ForceGroupToNeverFollow(false)`,
///   `MakePlayerGroupReappear()`; player data byte +0x43 = 0x63; ped float +0x734 = 1.0; bytes 0xB7CDDD = 1,
///   0xB7CDDE = 0; `EnablePedSpeech()`, `EnablePedSpeechForScriptSpeech()`.
/// - Pad 0: `SetDrunkInputDelay(0)`, bytes +0x118..0x11F = 0 (one `GetPad(0)` each); byte 0xB7CEEB = 1.
/// - The player's primary task of type 0x640 (mobile phone; `FindTaskByType(3, 0x640)` whose `GetTaskType()` is
///   0x640) → 0x634A40(task, player ped).
/// - Bytes 0xC1CC00/01 = 0, `CGameLogic::ClearSkip(true)`; unless the byte at 0x96A8B0 is 1 or 2 or player ped -1's
///   +0x530 is 0x37 or 0x36 (two `FindPlayerPed(-1)` calls): `CRestart::ClearRespawnPointForDurationOfMission()`.
///   Byte 0xA444A5 = 0, `gFireManager.ClearAllScriptFireFlags()`, dword 0xA447EC = -1, byte 0xA447F0 = 1, the 6
///   pairs at 0xA4892C = (-1, 0), the stuck-car checker's `Init()`, byte 0x8CDE56 = 1, floats 0x8A625C = -1.0 and
///   0x8D0934 = 200.0, byte 0xC1CC18 = 0.
/// - Byte 0xA444A8 clear: the focused player's dword +0xB0 zero → `TheCamera.Restore()`; the player data's
///   `m_pWanted` (+0) byte +0x1E loses bit 1, then bit 4 (a NULL player data dereferences address 0x1E: original);
///   `Players[0].MakePlayerSafe(false, 10000.0)`; `CHud::SetHelpMessage(NULL, true, false, false)`.
/// - The 75 entries: type 1/2/3 with a live pool handle → `CleanUpThisVehicle/Ped/Object`; 4 → script effect
///   system (`GetActualScriptThingIndex(h, 1)`; a live `FxSystem_c` → `Kill()` and `RemoveScriptEffectSystem(h)`);
///   5 → decision maker (`(h, 7)`, `UnloadDecisionMaker`); 7 → ped group (`(h, 8)`, `CPedGroups::RemoveGroup`); 8 →
///   sequence (`(h, 4)`: reference count zero → flush flag 0 and `Flush()`, else flush flag 1; then
///   `ms_bIsOpened[i]` and the script sequence's used byte = 0); 9 → `(h, 6)`, byte 0xC3A1A0[i] = 0; 11 → search
///   light (`(h, 2)`, `RemoveScriptSearchLight(h)`); 12 → checkpoint (`(h, 3)`, `RemoveScriptCheckpoint(h)`); 13 →
///   `RemoveScriptTextureDictionary()`. Every non-empty entry then `RemoveEntityFromList(handle, type)` (both
///   re-read after the case).
/// - `ClearPedTypeAsAcquaintance(a, t, GetPedFlag(f))` for t 0..23, f 24..31, a 0..4 (`GetPedFlag` once per call);
///   `ClearPedTypeAcquaintances(a, t)` for t 24..31, a 0..4.
#[hook(addr = 0x468560, conv = thiscall, name = "CMissionCleanup::Process", expect = "51 A0 8A 44 A4 00",
       this = "CMissionCleanup", unverified = "mvpk-script-1", route = "L2", hazards = "heap-writes",
       note = "the end-of-mission reset: many globals plus the player ped, its data, wanted, group and pad 0; runs only when a mission ends")]
pub fn mission_cleanup_process(this: GPtr<u8>) {
    if r8(0xA4448A) != 0 {
        let st = r8(0xB7CE74);
        w8(0xA4448A, 0);
        if st != 2 && st != 1 {
            camera_fade(GPtr::from_addr(THE_CAMERA), 0.5, 1);
            let b = get_pad(0).byte_add(0x10E);
            b.write(b.read() & 0xDF);
        }
    }
    train_disable_random_trains(0);
    train_release_mission_trains();
    plane_switch_ambient_planes(1);
    w32(0x8D2530, 0x3F80_0000);
    w32(0x8A5B20, 0x3F80_0000);
    if r8(0x96AB91) != 0 {
        let b = u32::from(r8(0x96AB90) == 0);
        w8(0x96AB91, 0);
        zones_fill_zones_with_gang_colours(b);
    }
    w32(0x8D2534, u32::MAX);
    w8(0x96AB93, 0);
    gang_wars_clear_specific_zones();
    w8(0xC0FCB2, 0);
    w8(0xC0FCB3, 0);
    w8(0xC0FCB4, 0);
    w8(0x96C008, 0);
    garages_all_resprays_close_or_open(1);
    w8(0xC87ACD, 0);
    world_find_player_wanted(-1).byte_add(0x14).cast::<u32>().write(0x3F80_0000);
    pickups_remove_mission_pickups();
    road_blocks_clear_script_road_blocks();
    streaming_disable_cop_bikes(0);
    w8(0xB72994, 1);
    w8(0xBB4A72, 0);
    w8(0x8D0A24, 1);
    path_find_release_requested_nodes(GPtr::from_addr(THE_PATHS));
    path_find_unmark_all_road_nodes_as_dont_wander(GPtr::from_addr(THE_PATHS));
    path_find_tidy_up_node_switches_after_mission(GPtr::from_addr(THE_PATHS));
    vehicle_recording_remove_unused();
    camera_set_wide_screen_off(GPtr::from_addr(THE_CAMERA));
    w16(0xB6F06E, 0x32);
    w16(0xB6F070, 0x31);
    w16(0xB6F072, 3);
    w16(0xB6F074, 0x30);
    w8(0xB6F06D, 0);
    w8(0xB6FD18, 0);
    camera_initialise_scriptable_components(GPtr::from_addr(THE_CAMERA));
    camera_reset_ducking_system(GPtr::from_addr(THE_CAMERA), GPtr::NULL);
    w32(0x8CCB84, 0x3F80_0000);
    w8(0xB6EC2C, 0);
    w8(0x96A8A8, 0);
    w32(0xA44490, 0);
    w8(0xC7C70C, 0);
    w8(0xC7C70D, 0);
    post_effects_script_reset_for_effects();
    let extra = GPtr::<u32>::from_addr(0xB72914).read();
    w8(0x96A7C8, 0);
    if extra == 0 {
        time_cycle_stop_extra_colour(0);
    }
    for slot in 0..4 {
        audio_engine_clear_mission_audio(GPtr::from_addr(AUDIO_ENGINE), slot);
    }
    weather_release_weather();
    w32(0xB728E0, 99999);
    for i in 0..10 {
        streaming_set_mission_doesnt_require_special_char(i);
    }
    scripts_clear_all_suppressed_car_models();
    w32(0xA4448C, u32::MAX);
    w8(0x9654B0, 0);
    w16(0xBAB1DC, 0xFFFF);
    for a in [0xBAA3FB, 0xBAA3F9, 0xBAA3F8, 0xBAA3FA, 0xBA677C, 0xA444A3, 0xA444A2, 0xA444A1] {
        w8(a, 0);
    }
    markers_force_render(0);
    w8(0xA444A0, 1);
    w32(0xA4449C, 0);
    w32(0xA44498, 0);
    w8(0xA44496, 1);
    w8(0xA44495, 1);
    w8(0xA44494, 1);
    w8(0xBA18D9, 0);
    w8(0xA44B66, 0);
    w16(0xA44B64, 0);
    w16(0xA44B60, 0);
    w8(0xA44489, 1);
    w8(0xA44488, 1);
    let gi = player_data().byte_add(0x38).cast::<i32>().read();
    let group = GPtr::<u8>::from_addr(PED_GROUPS).byte_add(gi.wrapping_mul(0x2D4) as isize);
    group_intel_set_default_task_allocator_type(group.byte_add(0x30), 1);
    group_intel_set_decision_maker_type(group.byte_add(0x30), -1);
    group.byte_add(0x2C).cast::<u32>().write(GPtr::<u32>::from_addr(0x86C6C0).read());
    let f = player_data().byte_add(0x34).cast::<u32>();
    f.write(f.read() & 0xFFFF_FEFF);
    player_ped_force_group_to_always_follow(player_ped(), 0);
    player_ped_force_group_to_never_follow(player_ped(), 0);
    player_ped_make_player_group_reappear(player_ped());
    player_data().byte_add(0x43).write(0x63);
    player_ped().byte_add(0x734).cast::<u32>().write(0x3F80_0000);
    w8(0xB7CDDD, 1);
    w8(0xB7CDDE, 0);
    ped_enable_ped_speech(player_ped());
    ped_enable_ped_speech_for_script_speech(player_ped());
    pad_set_drunk_input_delay(get_pad(0), 0);
    for off in 0x118..0x120 {
        get_pad(0).byte_add(off).write(0);
    }
    w8(0xB7CEEB, 1);
    let tm = player_ped().byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(4);
    let t = task_manager_find_task_by_type(tm, 3, 0x640);
    if !t.is_null() && v_task_get_task_type(t) == 0x640 {
        phone_task_stop(t, player_ped());
    }
    w8(0xC1CC00, 0);
    w8(0xC1CC01, 0);
    game_logic_clear_skip(1);
    let st = r8(0x96A8B0);
    if st != 2
        && st != 1
        && world_find_player_ped(-1).byte_add(0x530).cast::<u32>().read() != 0x37
        && world_find_player_ped(-1).byte_add(0x530).cast::<u32>().read() != 0x36
    {
        restart_clear_respawn_point_for_mission();
    }
    w8(0xA444A5, 0);
    fire_manager_clear_all_script_fire_flags(GPtr::from_addr(FIRE_MANAGER));
    w32(0xA447EC, u32::MAX);
    w8(0xA447F0, 1);
    for i in 0..6 {
        w32(0xA4892C + 8 * i, u32::MAX);
        w32(0xA48930 + 8 * i, 0);
    }
    stuck_car_check_init(GPtr::from_addr(STUCK_CARS));
    let keep_player = r8(0xA444A8);
    w8(0x8CDE56, 1);
    w32(0x8A625C, 0xBF80_0000);
    w32(0x8D0934, 0x4348_0000);
    w8(0xC1CC18, 0);
    if keep_player == 0 {
        let focus = usize::from(r8(0xB7CD74));
        if GPtr::<u32>::from_addr(PLAYERS + focus * 0x190 + 0xB0).read() == 0 {
            camera_restore(GPtr::from_addr(THE_CAMERA));
        }
        for mask in [0xFEu8, 0xFB] {
            let d = player_data();
            let wanted = if d.is_null() { GPtr::NULL } else { d.cast::<GPtr<u8>>().read() };
            let b = wanted.byte_add(0x1E);
            b.write(b.read() & mask);
        }
        player_info_make_player_safe(GPtr::from_addr(PLAYERS), 0, 10000.0);
        hud_set_help_message(GPtr::NULL, 1, 0, 0);
    }
    for i in 0..ENTRIES as isize {
        let e = this.byte_add(8 * i);
        let h = || e.byte_add(4).cast::<i32>().read();
        match e.read() {
            0 => continue,
            1 => {
                let v = pool_at(0xB74494, 0xA18, h());
                if !v.is_null() {
                    scripts_clean_up_this_vehicle(v);
                }
            }
            2 => {
                let p = pool_at(0xB74490, 0x7C4, h());
                if !p.is_null() {
                    scripts_clean_up_this_ped(p);
                }
            }
            3 => {
                let o = pool_at(0xB7449C, 0x19C, h());
                if !o.is_null() {
                    scripts_clean_up_this_object(o);
                }
            }
            4 => {
                let k = scripts_get_actual_script_thing_index(h() as u32, 1);
                if k >= 0 {
                    let fx = GPtr::<GPtr<u8>>::from_addr(SCRIPT_EFFECT_SYSTEMS + 4 + 8 * k as usize).read();
                    if !fx.is_null() {
                        fx_system_kill(fx);
                        scripts_remove_script_effect_system(h() as u32);
                    }
                }
            }
            5 => {
                let k = scripts_get_actual_script_thing_index(h() as u32, 7);
                if k >= 0 {
                    decision_maker_unload(k);
                }
            }
            7 => {
                let k = scripts_get_actual_script_thing_index(h() as u32, 8);
                if k >= 0 {
                    ped_groups_remove_group(k as u32);
                }
            }
            8 => {
                let k = scripts_get_actual_script_thing_index(h() as u32, 4);
                if k >= 0 {
                    let seq = GPtr::<u8>::from_addr(TASK_SEQUENCES + 0x40 * k as usize);
                    if seq.byte_add(0x3C).cast::<i32>().read() == 0 {
                        seq.byte_add(0x38).write(0);
                        sequence_flush(seq);
                    } else {
                        seq.byte_add(0x38).write(1);
                    }
                    w8(SEQUENCE_OPEN + k as usize, 0);
                    w8(SCRIPT_SEQUENCES + 4 * k as usize, 0);
                }
            }
            9 => {
                let k = scripts_get_actual_script_thing_index(h() as u32, 6);
                if k >= 0 {
                    w8(SCRIPT_THING_6_USED + k as usize, 0);
                }
            }
            11 => {
                if scripts_get_actual_script_thing_index(h() as u32, 2) >= 0 {
                    scripts_remove_script_search_light(h() as u32);
                }
            }
            12 => {
                if scripts_get_actual_script_thing_index(h() as u32, 3) >= 0 {
                    scripts_remove_script_checkpoint(h());
                }
            }
            13 => scripts_remove_script_texture_dictionary(),
            _ => {}
        }
        cleanup_remove_entity_from_list(this, h(), u32::from(e.read()));
    }
    for t in 0..24 {
        for f in 24..32 {
            for a in 0..5 {
                let flag = ped_type_get_ped_flag(f);
                ped_type_clear_ped_type_as_acquaintance(a as u32, t as u32, flag);
            }
        }
    }
    for t in 24..32 {
        for a in 0..5 {
            ped_type_clear_ped_type_acquaintances(a, t);
        }
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_ADD_ENTITY_TO_LIST, HOOK_REMOVE_ENTITY_FROM_LIST, HOOK_MISSION_CLEANUP_PROCESS];
