//! The cutscene manager's memory juggling (batch `intro-anim-1`): `RemoveEverythingBecauseCutsceneDoesntFitInMemory`
//! and `LoadEverythingBecauseCutsceneDeletedAllOfIt` (the player's weapons are stashed and given back around a
//! cutscene that needs the memory), plus the two empty head functions as phase B natives. Ported from the 1.0 US
//! bytes (gta-reversed `CutsceneMgr.cpp` as a reading aid). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::{HookReg, NativeReg};
use sa_core::mem::GPtr;
use sa_core::hook;

use sa_core::bindings::streaming_remove_current_zones_models;
use sa_core::bindings::clear_exciting_stuff_from_area;
use sa_core::bindings::col_store_remove_all_collision;
use sa_core::bindings::find_player_coors;
use sa_core::bindings::find_player_ped;
use sa_core::bindings::game_drastic_tidy_up_memory;
use sa_core::bindings::get_weapon_info as weapon_info_get;
use sa_core::bindings::load_all_requested_models as streaming_load_all_requested_models;
use sa_core::bindings::ped_clear_weapons;
use sa_core::bindings::ped_give_weapon;
use sa_core::bindings::ped_remove_weapon_anims;
use sa_core::bindings::radar_remove_radar_sections;
use sa_core::bindings::set_model_is_deletable as streaming_set_model_is_deletable;
use sa_core::bindings::set_model_txd_is_deletable as streaming_set_model_txd_is_deletable;
use sa_core::bindings::streaming_load_initial_peds;
use sa_core::bindings::streaming_load_initial_weapons;
use sa_core::bindings::streaming_request_model;
use sa_core::bindings::call_remove_loaded_vehicle as streaming_remove_loaded_vehicle;

/// `CCutsceneMgr::ms_hasFileInfo`-like flag 0xB5F85C: set while everything is removed
const EVERYTHING_REMOVED: usize = 0xB5F85C;
/// The stashed weapon count (i32) and the weapon types / ammo arrays (13 dwords each)
const NUM_WEAPONS: usize = 0xB5EB58;
const WEAPON_AMMO: usize = 0xB5EB5C;
const WEAPON_TYPES: usize = 0xB5EB90;
/// Byte 0xB7CD6D set to 1 before the zone models go
const STREAMING_FLAG: usize = 0xB7CD6D;
/// `CStreaming::ms_aInfoForModel` (0x14 bytes each; +6 the flags)
const STREAMING_INFO: usize = 0x8E4CC0;

use sa_core::field::g32 as gu;
use sa_core::field::pa as gb;

use sa_core::field::rd32;

/// `CCutsceneMgr::RemoveEverythingBecauseCutsceneDoesntFitInMemory()` (0x4D5AF0, cdecl). Already removed (flag
/// 0xB5F85C): `CGame::DrasticTidyUpMemory(true)` only. Else: `ClearExcitingStuffFromArea(FindPlayerCoors(-1), 120.0,
/// true)`, `CColStore::RemoveAllCollision`, byte 0xB7CD6D = 1, `RemoveCurrentZonesModels`, `RemoveLoadedVehicle`
/// until it fails, `CRadar::RemoveRadarSections`, models 7, 0x15A, 0x14E, 0x159 and their TXDs deletable; the
/// player's `RemoveWeaponAnims(0, -1000.0)`; every weapon slot (+0x5A0, 13 × 0x1C) with a type is stashed (type,
/// ammo +0xC) at 0xB5EB90 / 0xB5EB5C with the count at 0xB5EB58; `ClearWeapons`; the flag = 1;
/// `DrasticTidyUpMemory(true)`.
#[hook(addr = 0x4d5af0, conv = cdecl, name = "CCutsceneMgr::RemoveEverythingBecauseCutsceneDoesntFitInMemory", expect = "A0 5C F8 B5 00",
       unverified = "intro-anim-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "removes streamed models and collision; the player's weapons are stashed in globals")]
pub fn cutscene_remove_everything() {
    if gb(EVERYTHING_REMOVED).read() != 0 {
        game_drastic_tidy_up_memory(1);
        return;
    }
    let mut pos = [0u32; 3];
    let pos = GPtr::<f32>::cast::<u8>(find_player_coors(GPtr::<u8>::cast::<f32>(GPtr::from_raw(pos.as_mut_ptr()).cast()), -1));
    clear_exciting_stuff_from_area(pos, 120.0, 1);
    col_store_remove_all_collision();
    gb(STREAMING_FLAG).write(1);
    streaming_remove_current_zones_models();
    while u8::from(streaming_remove_loaded_vehicle()) != 0 {}
    radar_remove_radar_sections();
    for m in [7, 0x15A, 0x14E, 0x159] {
        streaming_set_model_is_deletable(m);
        streaming_set_model_txd_is_deletable(m);
    }
    let player = find_player_ped(-1);
    ped_remove_weapon_anims(player, 0, -1000.0);
    gu(NUM_WEAPONS).write(0);
    let mut n = 0usize;
    for slot in 0..13isize {
        let w = player.byte_add(0x5A0 + 0x1C * slot);
        let ty = rd32(w, 0);
        if ty != 0 {
            gu(WEAPON_TYPES + 4 * n).write(ty);
            gu(WEAPON_AMMO + 4 * n).write(rd32(w, 0xC));
            n += 1;
        }
    }
    gu(NUM_WEAPONS).write(n as u32);
    ped_clear_weapons(player);
    gb(EVERYTHING_REMOVED).write(1);
    game_drastic_tidy_up_memory(1);
}

/// `CCutsceneMgr::LoadEverythingBecauseCutsceneDeletedAllOfIt()` (0x4D5C10, cdecl): the flag (0xB5F85C) = 0,
/// `LoadInitialPeds`, `LoadInitialWeapons`; for every stashed weapon (the count re-read each pass): its model
/// (`GetWeaponInfo(type, 1)` +0xC) is requested (flags 2) and loaded, the second model (+0x10, unless -1) requested
/// (flags 0) and loaded; unless the model's streaming flags (+6) had bit 2 before, the model is made deletable;
/// `GiveWeapon(type, ammo, true)` on `FindPlayerPed(-1)`. The count = 0.
#[hook(addr = 0x4d5c10, conv = cdecl, name = "CCutsceneMgr::LoadEverythingBecauseCutsceneDeletedAllOfIt", expect = "51 57 C6 05 5C F8 B5 00 00",
       unverified = "intro-anim-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "loads models synchronously and gives the player its stashed weapons")]
pub fn cutscene_load_everything() {
    gb(EVERYTHING_REMOVED).write(0);
    streaming_load_initial_peds();
    streaming_load_initial_weapons();
    let player = find_player_ped(-1);
    let mut i = 0usize;
    if gu(NUM_WEAPONS).read() as i32 <= 0 {
        gu(NUM_WEAPONS).write(0);
        return;
    }
    loop {
        let ty = || gu(WEAPON_TYPES + 4 * i).read();
        let model = rd32(weapon_info_get(ty() as i32, 1), 0xC) as i32;
        let flags = gb(STREAMING_INFO + 0x14 * (model as usize) + 6).read();
        streaming_request_model(model, 2);
        streaming_load_all_requested_models(0);
        if rd32(weapon_info_get(ty() as i32, 1), 0x10) as i32 != -1 {
            let m2 = rd32(weapon_info_get(ty() as i32, 1), 0x10) as i32;
            streaming_request_model(m2, 0);
            streaming_load_all_requested_models(0);
        }
        if flags & 2 == 0 {
            streaming_set_model_is_deletable(model);
        }
        ped_give_weapon(player, (ty() as i32) as u32, gu(WEAPON_AMMO + 4 * i).read(), 1);
        i += 1;
        if i as i32 >= gu(NUM_WEAPONS).read() as i32 {
            break;
        }
    }
    gu(NUM_WEAPONS).write(0);
}

/// `CCutsceneMgr::AddCutsceneHead(CObject*, int32)` (0x5B0380): returns 0 (the body is `xor eax, eax; ret`; three
/// bytes, too short to hook: a phase B native).
#[sa_core::native(addr = 0x5B0380, conv = cdecl, name = "CCutsceneMgr::AddCutsceneHead")]
pub fn cutscene_add_cutscene_head(_obj: GPtr<u8>, _n: i32) -> u32 {
    0
}

/// `CCutsceneMgr::SetHeadAnim(const char*, CObject*)` (0x5B0440): an empty function (`ret`; a phase B native).
#[sa_core::native(addr = 0x5B0440, conv = cdecl, name = "CCutsceneMgr::SetHeadAnim")]
pub fn cutscene_set_head_anim(_name: GPtr<u8>, _obj: GPtr<u8>) {}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CUTSCENE_REMOVE_EVERYTHING, HOOK_CUTSCENE_LOAD_EVERYTHING];

/// The phase B natives of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_CUTSCENE_ADD_CUTSCENE_HEAD, NATIVE_CUTSCENE_SET_HEAD_ANIM];
