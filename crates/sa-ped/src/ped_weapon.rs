//! `CPed` weapon-slot and model helpers (PORT-WEAPON-PED): current weapon, clearing, vehicle/cutscene
//! weapon swaps, goggles, dress/undress. Ported from the 1.0 US bytes (gta-reversed `Ped.cpp`).
//!
//! `CPed` +0x5A0 weapons[13] (0x1C: type, state, clip, total, ..., +0x15 goggles on), +0x70C weapon saved
//! across vehicles/cutscenes (0x37 = none), +0x718 active slot, +0x4FC goggles object, +0x500 goggles flag
//! pointer.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

const NO_WEAPON: i32 = 0x37;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;

orig! {
    /// `CPed::SetCurrentWeapon(int32 slot)` (0x5E61F0)
    pub fn set_current_weapon_slot_call(p: GPtr<u8>, slot: i32) = thiscall 0x5E61F0 => set_current_weapon_slot;
    /// `CPed::RemoveWeaponModel(int32 model)` (0x5E3990)
    pub fn remove_weapon_model(p: GPtr<u8>, model: i32) = thiscall 0x5E3990 => unported;
    /// `CPed::AddWeaponModel(int32 model)` (0x5E5ED0)
    pub fn add_weapon_model(p: GPtr<u8>, model: i32) = thiscall 0x5E5ED0 => unported;
    /// `CPed::RemoveGogglesModel` (0x5DF170)
    pub fn remove_goggles_model(p: GPtr<u8>) = thiscall 0x5DF170 => remove_goggles_model_port;
    /// `CPed::AddGogglesModel(int32 model, bool& flag)` (0x5E3A90)
    pub fn add_goggles_model_call(p: GPtr<u8>, model: i32, flag: GPtr<u8>) = thiscall 0x5E3A90 => add_goggles_model;
    /// `CWeapon::Shutdown` (0x73A380)
    pub fn weapon_shutdown(w: GPtr<u8>) = thiscall 0x73A380 => unported;
    /// `CPlayerPed::GetPlayerInfoForThisPlayerPed` (0x609FF0)
    pub fn get_player_info_for_this_player_ped(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609FF0 => crate::player_ped::get_player_info_for_this_player_ped;
    /// `CBaseModelInfo::CreateInstance()` (vtable slot 0x2C)
    pub fn mi_create_instance(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x2C => unported;
    /// `CBaseModelInfo::AddRef` (0x4C4BA0)
    pub fn mi_add_ref(mi: GPtr<u8>) = thiscall 0x4C4BA0 => unported;
    /// `CEntity::DeleteRwObject` (vtable slot 0x20)
    pub fn entity_delete_rw_object(e: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// `CEntity::SetModelIndex(uint32)` (vtable slot 0x14)
    pub fn entity_set_model_index(e: GPtr<u8>, m: i32) = thiscall vslot 0x14 => unported;
    /// `CStreaming::RequestSpecialModel(int32 model, const char* name, int32 flags)` (0x409D10)
    pub fn request_special_model(m: i32, name: u32, flags: i32) = cdecl 0x409D10 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    pub fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CVisibilityPlugins::GetClumpModelInfo(RpClump*)` (0x732AC0)
    pub fn get_clump_model_info(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x732AC0 => unported;
    /// `CBaseModelInfo::RemoveRef()` (0x4C4BB0)
    pub fn mi_remove_ref(mi: GPtr<u8>) = thiscall 0x4C4BB0 => unported;
    /// `RwHelper::GetFirstAtomic(RpClump*)` (0x734820)
    pub fn get_first_atomic(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `RpSkinGeometryGetSkin(RpGeometry*)` (0x7C7550)
    pub fn rp_skin_geometry_get_skin(g: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7550 => unported;
    /// `RpClumpForAllAtomics(RpClump*, callback, void*)` (0x749B70)
    pub fn rp_clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `RpClumpDestroy(RpClump*)` (0x74A310)
    pub fn rp_clump_destroy(c: GPtr<u8>) -> i32 = cdecl 0x74A310 => unported;
}

/// The atomic callback `RemoveGogglesModel` passes to `RpClumpForAllAtomics` (0x734B90, the skinned
/// atomic's anim removal).
const ATOMIC_REMOVE_ANIM_FROM_SKIN_CB: u32 = 0x734B90;

fn weapon(p: GPtr<u8>, slot: i32) -> GPtr<u8> {
    p.byte_add(0x5A0 + (slot as isize).wrapping_mul(0x1C))
}

fn wtype(p: GPtr<u8>, slot: i32) -> i32 {
    rd::<i32>(weapon(p, slot), 0)
}

fn active(p: GPtr<u8>) -> i32 {
    i32::from(rd::<i8>(p, 0x718))
}

fn info_slot(ty: i32) -> i32 {
    rd::<i32>(get_weapon_info(ty, 1), 0x14)
}

fn info_model(ty: i32) -> i32 {
    rd::<i32>(get_weapon_info(ty, 1), 0xC)
}

/// `CPed::SetCurrentWeapon(int32 slot)` (0x5E61F0, `ret 4`): -1 does nothing; removes the old weapon's
/// model, sets the active slot (and the player data's chosen slot), adds the new weapon's model.
#[hook(addr = 0x5E61F0, conv = thiscall, name = "CPed::SetCurrentWeapon-Slot", expect = "53 8B 5C 24 08", this = "CPed")]
pub fn set_current_weapon_slot(this: GPtr<u8>, slot: i32) {
    if slot == -1 {
        return;
    }
    let old = wtype(this, active(this));
    if old != 0 {
        remove_weapon_model(this, info_model(old));
    }
    wr::<i8>(this, 0x718, slot as i8);
    let d = rd::<GPtr<u8>>(this, 0x480);
    if !d.is_null() {
        wr::<i8>(d, 0x20, slot as i8);
    }
    if wtype(this, slot) != 0 {
        add_weapon_model(this, info_model(wtype(this, active(this))));
    }
}

/// `CPed::SetCurrentWeapon(eWeaponType)` (0x5E6280, `ret 4`)
#[hook(addr = 0x5E6280, conv = thiscall, name = "CPed::SetCurrentWeapon-WepType", expect = "8B 44 24 04 56", this = "CPed")]
pub fn set_current_weapon_type(this: GPtr<u8>, ty: i32) {
    set_current_weapon_slot_call(this, info_slot(ty));
}

/// `CPed::ClearWeapon(eWeaponType)` (0x5E62B0, `ret 4`): when the type's slot holds it: switch to the
/// unarmed slot if it is active, shut the weapon down, and drop the goggles for types 0x2C/0x2D.
#[hook(addr = 0x5E62B0, conv = thiscall, name = "CPed::ClearWeapon", expect = "56 57 8B 7C 24 0C", this = "CPed")]
pub fn clear_weapon(this: GPtr<u8>, ty: i32) {
    let slot = info_slot(ty);
    if slot == -1 || wtype(this, slot) != ty {
        return;
    }
    if active(this) == slot {
        set_current_weapon_slot_call(this, info_slot(0));
    }
    weapon_shutdown(weapon(this, slot));
    if ty == 0x2C || ty == 0x2D {
        remove_goggles_model(this);
    }
}

/// `CPed::ClearWeapons` (0x5E6320): removes the weapon model (-1) and the goggles, shuts all 13 weapons
/// down and selects the unarmed slot.
#[hook(addr = 0x5E6320, conv = thiscall, name = "CPed::ClearWeapons", expect = "53 56 57 6A FF", this = "CPed")]
pub fn clear_weapons(this: GPtr<u8>) {
    remove_weapon_model(this, -1);
    remove_goggles_model(this);
    for i in 0..13 {
        weapon_shutdown(weapon(this, i));
    }
    set_current_weapon_slot_call(this, info_slot(0));
}

/// `CPed::ReplaceWeaponForScriptedCutscene` (0x5E6530): saves the current weapon type, selects slot 0.
#[hook(addr = 0x5E6530, conv = thiscall, name = "CPed::ReplaceWeaponForScriptedCutscene", expect = "0F BE 81 18 07 00 00", this = "CPed")]
pub fn replace_weapon_for_scripted_cutscene(this: GPtr<u8>) {
    wr::<i32>(this, 0x70C, wtype(this, active(this)));
    set_current_weapon_slot_call(this, 0);
}

/// `CPed::RemoveWeaponForScriptedCutscene` (0x5E6550): restores the saved weapon (if any).
#[hook(addr = 0x5E6550, conv = thiscall, name = "CPed::RemoveWeaponForScriptedCutscene", expect = "56 8B F1 8B 86 0C 07 00 00", this = "CPed")]
pub fn remove_weapon_for_scripted_cutscene(this: GPtr<u8>) {
    let s = rd::<i32>(this, 0x70C);
    if s == NO_WEAPON {
        return;
    }
    set_current_weapon_slot_call(this, info_slot(s));
    wr::<i32>(this, 0x70C, NO_WEAPON);
}

/// `CPed::ReplaceWeaponWhenExitingVehicle` (0x5E6490): clears player data flag 0x800; players restore the
/// saved weapon, or re-add the current weapon's model when none was saved; others re-add the model.
#[hook(addr = 0x5E6490, conv = thiscall, name = "CPed::ReplaceWeaponWhenExitingVehicle", expect = "56 8B F1 8B 86 80 04 00 00", this = "CPed")]
pub fn replace_weapon_when_exiting_vehicle(this: GPtr<u8>) {
    let d = rd::<GPtr<u8>>(this, 0x480);
    if !d.is_null() {
        wr::<u32>(d, 0x34, rd::<u32>(d, 0x34) & 0xFFFF_F7FF);
    }
    if matches!(rd::<i32>(this, 0x598), 0 | 1) {
        let s = rd::<i32>(this, 0x70C);
        if s != NO_WEAPON {
            set_current_weapon_slot_call(this, info_slot(s));
            wr::<i32>(this, 0x70C, NO_WEAPON);
            return;
        }
    }
    add_weapon_model(this, info_model(wtype(this, active(this))));
}

/// `CPed::RemoveWeaponWhenEnteringVehicle(int32 arg)` (0x5E6370, `ret 4`): sets player data flag 0x800;
/// with no weapon saved, a player who may drive-by (player info +0x153) switches to slot 4 (SMG types
/// 0x1C/0x20, or 0x1D, with ammo) or, for `arg == 1`, to slot 2 when slot 3 holds 0x1A with ammo or slot 2
/// holds 0x16 with ammo (the original picks slot 2 in both cases), saving the current weapon; otherwise
/// the current weapon's model is removed.
#[hook(addr = 0x5E6370, conv = thiscall, name = "CPed::RemoveWeaponWhenEnteringVehicle", expect = "56 8B F1 8B 86 80 04 00 00", this = "CPed")]
pub fn remove_weapon_when_entering_vehicle(this: GPtr<u8>, arg: i32) {
    let d = rd::<GPtr<u8>>(this, 0x480);
    if !d.is_null() {
        wr::<u32>(d, 0x34, rd::<u32>(d, 0x34) | 0x800);
    }
    if rd::<i32>(this, 0x70C) != NO_WEAPON {
        return;
    }
    let mut pick: Option<i32> = None;
    if matches!(rd::<i32>(this, 0x598), 0 | 1) && rd::<u8>(get_player_info_for_this_player_ped(this), 0x153) != 0 {
        let t4 = rd::<i32>(this, 0x610);
        let smg = t4 == 0x1C || t4 == 0x20 || (arg != 1 && t4 == 0x1D);
        if smg && rd::<i32>(this, 0x61C) > 0 {
            pick = Some(4);
        } else if arg == 1
            && ((rd::<i32>(this, 0x5F4) == 0x1A && rd::<i32>(this, 0x600) > 0)
                || (rd::<i32>(this, 0x5D8) == 0x16 && rd::<i32>(this, 0x5E4) > 0))
        {
            pick = Some(2);
        }
    }
    match pick {
        Some(slot) => {
            if rd::<i32>(this, 0x70C) == NO_WEAPON {
                wr::<i32>(this, 0x70C, wtype(this, active(this)));
            }
            set_current_weapon_slot_call(this, info_slot(wtype(this, slot)));
        }
        None => remove_weapon_model(this, info_model(wtype(this, active(this)))),
    }
}

/// `CPed::AddGogglesModel(int32 model, bool& flag)` (0x5E3A90, `ret 8`): replaces the goggles object by a
/// new instance of the model, references the model, stores the flag pointer and sets it.
#[hook(addr = 0x5E3A90, conv = thiscall, name = "CPed::AddGogglesModel", expect = "8B 44 24 04 83 F8 FF", this = "CPed")]
pub fn add_goggles_model(this: GPtr<u8>, model: i32, flag: GPtr<u8>) {
    if model == -1 {
        return;
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).byte_add((model as isize).wrapping_mul(4)).read();
    if !rd::<GPtr<u8>>(this, 0x4FC).is_null() {
        remove_goggles_model(this);
    }
    wr::<GPtr<u8>>(this, 0x4FC, mi_create_instance(mi));
    mi_add_ref(mi);
    wr::<GPtr<u8>>(this, 0x500, flag);
    flag.write(1);
}

/// `CPed::RemoveGogglesModel()` (0x5DF170, thiscall; the +0x4FC load sits in the fragment 0x40151C):
/// with a goggles clump (+0x4FC): `RemoveRef` on its model info (`GetClumpModelInfo`); when its first
/// atomic's geometry (+0x18) is skinned, `RpClumpForAllAtomics(clump, 0x734B90, 0)`;
/// `RpClumpDestroy(clump)`, +0x4FC = 0, and the goggles flag (+0x500) when set: `*flag = 0`, +0x500 = 0.
/// Every use re-reads +0x4FC.
#[hook(addr = 0x5df170, conv = thiscall, name = "CPed::RemoveGogglesModel", expect = "56 8B F1 90 E9 A3 23 E2 FF",
       route = "L2", hazards = "entry-protection-read,heap-writes",
       recs = "grove-walk", note = "destroys the goggles clump (RW heap); 0x409AC8 reads entry bytes +4 (keycells)")]
pub fn remove_goggles_model_port(this: GPtr<u8>) {
    if rd::<GPtr<u8>>(this, 0x4FC).is_null() {
        return;
    }
    mi_remove_ref(get_clump_model_info(rd::<GPtr<u8>>(this, 0x4FC)));
    let a = get_first_atomic(rd::<GPtr<u8>>(this, 0x4FC));
    if !a.is_null() && !rp_skin_geometry_get_skin(rd::<GPtr<u8>>(a, 0x18)).is_null() {
        rp_clump_for_all_atomics(rd::<GPtr<u8>>(this, 0x4FC), ATOMIC_REMOVE_ANIM_FROM_SKIN_CB, 0);
    }
    rp_clump_destroy(rd::<GPtr<u8>>(this, 0x4FC));
    let flag = rd::<GPtr<u8>>(this, 0x500);
    wr::<u32>(this, 0x4FC, 0);
    if !flag.is_null() {
        flag.write(0);
        wr::<u32>(this, 0x500, 0);
    }
}

fn goggles_weapon(this: GPtr<u8>) -> Option<GPtr<u8>> {
    let w = weapon(this, info_slot(0x2D));
    let t = rd::<i32>(w, 0);
    if t == 0x2D || t == 0x2C { Some(w) } else { None }
}

/// `CPed::PutOnGoggles` (0x5E3AE0): with night/thermal goggles (0x2D/0x2C) in their slot: the goggles
/// model with the effect flag (0xC402B9 night / 0xC402B8 infrared), the slot's goggles flag (+0x15) set,
/// and the weapon model removed when that slot is active.
#[hook(addr = 0x5E3AE0, conv = thiscall, name = "CPed::PutOnGoggles", expect = "56 57 6A 01 6A 2D", this = "CPed")]
pub fn put_on_goggles(this: GPtr<u8>) {
    let Some(w) = goggles_weapon(this) else { return };
    let m = info_model(rd::<i32>(w, 0));
    let flag = if rd::<i32>(w, 0) == 0x2D { 0xC402B9 } else { 0xC402B8 };
    add_goggles_model_call(this, m, g::<u8>(flag));
    wr::<u8>(w, 0x15, 1);
    if w == weapon(this, active(this)) {
        remove_weapon_model(this, m);
    }
}

/// `CPed::TakeOffGoggles` (0x5E6010): the reverse of `PutOnGoggles` (goggles model removed, flag cleared,
/// weapon model re-added when active).
#[hook(addr = 0x5E6010, conv = thiscall, name = "CPed::TakeOffGoggles", expect = "56 57 6A 01 6A 2D", this = "CPed")]
pub fn take_off_goggles(this: GPtr<u8>) {
    let Some(w) = goggles_weapon(this) else { return };
    let m = info_model(rd::<i32>(w, 0));
    remove_goggles_model(this);
    wr::<u8>(w, 0x15, 0);
    if w == weapon(this, active(this)) {
        add_weapon_model(this, m);
    }
}

/// `CPed::Undress(const char* name)` (0x5E00F0, `ret 4`): deletes the RW object, requests the special
/// model (model index, 0 for players) with flags 0xC and removes the ped from the world.
#[hook(addr = 0x5E00F0, conv = thiscall, name = "CPed::Undress", expect = "56 8B F1 8B 06", this = "CPed")]
pub fn undress(this: GPtr<u8>, name: u32) {
    let mut m = i32::from(rd::<i16>(this, 0x22));
    entity_delete_rw_object(this);
    if matches!(rd::<i32>(this, 0x598), 0 | 1) {
        m = 0;
    }
    request_special_model(m, name, 0xC);
    world_remove(this);
}

/// `CPed::Dress` (0x5E0130): `SetModelIndex(model)` with the model index cleared first, state 1 unless
/// 0x32, back into the world, heading rate from the stat.
#[hook(addr = 0x5E0130, conv = thiscall, name = "CPed::Dress", expect = "56 8B F1 0F BF 46 22", this = "CPed")]
pub fn dress(this: GPtr<u8>) {
    let m = i32::from(rd::<i16>(this, 0x22));
    wr::<i16>(this, 0x22, -1);
    entity_set_model_index(this, m);
    if rd::<i32>(this, 0x530) != 0x32 {
        wr::<i32>(this, 0x530, 1);
    }
    world_add(this);
    wr::<u32>(this, 0x560, rd::<u32>(rd::<GPtr<u8>>(this, 0x59C), 0x20));
}

// --- hook list (generated) ---

pub const HOOKS: &[HookReg] = &[
    HOOK_REMOVE_GOGGLES_MODEL_PORT,
    HOOK_SET_CURRENT_WEAPON_SLOT,
    HOOK_SET_CURRENT_WEAPON_TYPE,
    HOOK_CLEAR_WEAPON,
    HOOK_CLEAR_WEAPONS,
    HOOK_REPLACE_WEAPON_FOR_SCRIPTED_CUTSCENE,
    HOOK_REMOVE_WEAPON_FOR_SCRIPTED_CUTSCENE,
    HOOK_REPLACE_WEAPON_WHEN_EXITING_VEHICLE,
    HOOK_REMOVE_WEAPON_WHEN_ENTERING_VEHICLE,
    HOOK_ADD_GOGGLES_MODEL,
    HOOK_PUT_ON_GOGGLES,
    HOOK_TAKE_OFF_GOGGLES,
    HOOK_UNDRESS,
    HOOK_DRESS,
];
