//! FRONTIER-2 (batch `frontier2-4`): the weapon-set cheats `CCheat::WeaponCheat1/2/3` (0x4385B0 / 0x438890 /
//! 0x438B30), reached by the `sniper` scenario's KJKSZPJ. Ported from the 1.0 US bytes: each requests its weapon
//! models (`CStreaming::RequestModel(m, 2)`), loads them (`LoadAllRequestedModels(false)`), gives the weapon list
//! (`CPed::GiveWeapon(type, ammo, true)`, inlined; gta-reversed calls `GiveWeaponSet1/2/3`) to `FindPlayerPed(-1)` and,
//! when there is one, to `FindPlayerPed(1)`, then marks the models deletable (WeaponCheat1 in its own order).

use sa_core::abi::HookReg;
use sa_core::bindings::{load_all_requested_models, ped_give_weapon, set_model_is_deletable, streaming_request_model};
use sa_core::hook;
use sa_core::mem::GPtr;

use super::common::find_player_ped;

/// One weapon cheat as the bytes order it.
fn weapon_cheat(request: &[i32], give: &[(u32, u32)], deletable: &[i32]) {
    for &m in request {
        streaming_request_model(m, 2);
    }
    load_all_requested_models(0);
    let give_to = |id: i32| {
        for &(ty, ammo) in give {
            let p: GPtr<u8> = find_player_ped(id);
            ped_give_weapon(p, ty, ammo, 1);
        }
    };
    give_to(-1);
    if !find_player_ped(1).is_null() {
        give_to(1);
    }
    for &m in deletable {
        set_model_is_deletable(m);
    }
}

/// `CCheat::WeaponCheat1()` (0x4385B0): weapon types 1, 5, 0x12, 0x16, 0x19, 0x1C, 0x1E, 0x21, 0x23, 0x29.
#[hook(addr = 0x4385B0, conv = cdecl, name = "CCheat::WeaponCheat1", expect = "6A 02 68 4B 01 00 00",
       unverified = "frontier2-4", route = "L2", hazards = "heap-writes")]
pub fn weapon_cheat1() {
    weapon_cheat(
        &[0x14B, 0x150, 0x158, 0x15A, 0x15D, 0x160, 0x163, 0x165, 0x167, 0x16D],
        &[(1, 1), (5, 1), (0x12, 0xA), (0x16, 0x64), (0x19, 0x32), (0x1C, 0x96), (0x1E, 0x78), (0x21, 0x19), (0x23, 0xC8), (0x29, 0xC8)],
        &[0x14B, 0x160, 0x150, 0x158, 0x15D, 0x163, 0x165, 0x167, 0x16D],
    );
}

/// `CCheat::WeaponCheat2()` (0x438890): weapon types 4, 0x10, 0x18, 0x1A, 0x20, 0x1F (M4), 0x22 (sniper rifle), 0x25,
/// 0x2A.
#[hook(addr = 0x438890, conv = cdecl, name = "CCheat::WeaponCheat2", expect = "6A 02 68 4F 01 00 00",
       unverified = "frontier2-4", route = "L2", hazards = "heap-writes")]
pub fn weapon_cheat2() {
    let models = [0x14F, 0x156, 0x15C, 0x15E, 0x174, 0x164, 0x166, 0x169, 0x16E];
    weapon_cheat(
        &models,
        &[(4, 0), (0x10, 0xA), (0x18, 0x28), (0x1A, 0x28), (0x20, 0x96), (0x1F, 0x96), (0x22, 0x15), (0x25, 0x1F4), (0x2A, 0xC8)],
        &models,
    );
}

/// `CCheat::WeaponCheat3()` (0x438B30): weapon types 9, 0x27, 0x17, 0x1B, 0x1D, 0x1F, 0x24.
#[hook(addr = 0x438B30, conv = cdecl, name = "CCheat::WeaponCheat3", expect = "6A 02 68 55 01 00 00",
       unverified = "frontier2-4", route = "L2", hazards = "heap-writes")]
pub fn weapon_cheat3() {
    let models = [0x155, 0x15B, 0x15F, 0x161, 0x164, 0x168, 0x16B, 0x16C];
    weapon_cheat(
        &models,
        &[(9, 0), (0x27, 5), (0x17, 0x28), (0x1B, 0x1E), (0x1D, 0x64), (0x1F, 0x96), (0x24, 0xC8)],
        &models,
    );
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_WEAPON_CHEAT1, HOOK_WEAPON_CHEAT2, HOOK_WEAPON_CHEAT3];
