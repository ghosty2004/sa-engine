//! Small PED bodies ported by impl-1 after the helper pass (PORT-WEAPON-PED): forwarding thunks and tiny
//! predicates. Ported from the 1.0 US bytes. The one-instruction `ret` bodies of this subsystem
//! (`CPed::Update`, `CWanted::ResetPolicePursuit`, ...) are not hookable (shorter than the 5-byte patch).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::DeleteRwObject` (0x534030)
    pub fn entity_delete_rw_object(e: GPtr<u8>) = thiscall 0x534030 => unported;
    /// `CPedType::Initialise` (0x608E40)
    pub fn ped_type_initialise() = cdecl 0x608E40 => unported;
    /// 0x64F860 (catalogue name `CCarEnterExit::SetAnimOffsetForEnterOrExitVehicle`, `void()`)
    pub fn car_enter_exit_set_anim_offset() = cdecl 0x64F860 => unported;
    /// `CAEPedSpeechAudioEntity::DisablePedSpeech(int16)` (0x4E56D0, `ret 4`)
    pub fn speech_disable(s: GPtr<u8>, b: i32) = thiscall 0x4E56D0 => unported;
    /// `CAEPedSpeechAudioEntity::DisablePedSpeechForScriptSpeech(int16)` (0x4E5700, `ret 4`)
    pub fn speech_disable_for_script(s: GPtr<u8>, b: i32) = thiscall 0x4E5700 => unported;
    /// `CAEPedSpeechAudioEntity::CanPedHoldConversation` (0x4E69E0)
    pub fn speech_can_hold_conversation(s: GPtr<u8>) -> u8 = thiscall 0x4E69E0 => unported;
    /// `CPed::GetWeaponSkill(eWeaponType)` (0x5E3B60)
    pub fn get_weapon_skill_type_call(p: GPtr<u8>, ty: i32) -> u8 = thiscall 0x5E3B60 => crate::ped::get_weapon_skill_type;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    pub fn get_weapon_info_call(ty: i32, skill: i32) -> GPtr<u8> = cdecl 0x743C60 => unported;
}

/// `CPed::DeleteRwObject` (0x5DEBF0, inline `jmp CEntity::DeleteRwObject`).
#[hook(addr = 0x5DEBF0, conv = thiscall, name = "CPed::DeleteRwObject", expect = "E9 3B 54 F5 FF 90", this = "CPed")]
pub fn delete_rw_object(this: GPtr<u8>) {
    entity_delete_rw_object(this);
}

/// `CPed::Initialise` (0x5DEBB0): `CPedType::Initialise()`, then a tail jump to 0x64F860.
#[hook(addr = 0x5DEBB0, conv = cdecl, name = "CPed::Initialise", expect = "E8 8B A2 02 00")]
pub fn initialise() {
    ped_type_initialise();
    car_enter_exit_set_anim_offset();
}

/// `CPed::DisablePedSpeech(int16)` (0x5EFF60): forwards to the speech entity at +0x294.
#[hook(addr = 0x5EFF60, conv = thiscall, name = "CPed::DisablePedSpeech", expect = "81 C1 94 02 00 00", this = "CPed")]
pub fn disable_ped_speech(this: GPtr<u8>, b: i32) {
    speech_disable(this.byte_add(0x294), b);
}

/// `CPed::DisablePedSpeechForScriptSpeech(int16)` (0x5EFF80): forwards to the speech entity at +0x294.
#[hook(addr = 0x5EFF80, conv = thiscall, name = "CPed::DisablePedSpeechForScriptSpeech", expect = "81 C1 94 02 00 00", this = "CPed")]
pub fn disable_ped_speech_for_script_speech(this: GPtr<u8>, b: i32) {
    speech_disable_for_script(this.byte_add(0x294), b);
}

/// `CPed::CanPedHoldConversation` (0x5EFFA0): forwards to the speech entity at +0x294.
#[hook(addr = 0x5EFFA0, conv = thiscall, name = "CPed::CanPedHoldConversation", expect = "81 C1 94 02 00 00", this = "CPed")]
pub fn can_ped_hold_conversation(this: GPtr<u8>) -> u8 {
    speech_can_hold_conversation(this.byte_add(0x294))
}

/// `CPad::IsDPadUpPressed` (0x53EE60): `NewState.DPadUp (+0x10) != 0 && OldState.DPadUp (+0x40) == 0`
/// as an int.
#[hook(addr = 0x53EE60, conv = thiscall, name = "CPad::IsDPadUpPressed", expect = "66 83 79 10 00")]
pub fn is_dpad_up_pressed(this: GPtr<u8>) -> u32 {
    (rd::<i16>(this, 0x10) != 0 && rd::<i16>(this, 0x40) == 0) as u32
}

/// `FUN_00609380`: true when any of the bytes 0x96917C, 0x96917D, 0x96917E is set.
#[hook(addr = 0x609380, conv = cdecl, name = "FUN_00609380", expect = "A0 7C 91 96 00")]
pub fn any_of_96917c() -> u8 {
    let b = |a: usize| g::<u8>(a).read();
    (b(0x96917C) != 0 || b(0x96917D) != 0 || b(0x96917E) != 0) as u8
}

/// `FUN_004b1370` (thiscall, `ret 4`; called from 0x5E1FA0): the constructor of a 0x10-byte object with
/// vtable 0x85B4C8: +4 = 0, +8 (byte) = 0, +0xC = the argument; returns `this`.
#[hook(addr = 0x4B1370, conv = thiscall, name = "FUN_004b1370", expect = "8B C1 33 C9 89 48 04")]
pub fn ctor_85b4c8(this: GPtr<u8>, arg: u32) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u32>(this, 0, 0x85B4C8);
    wr::<u32>(this, 0xC, arg);
    this
}

/// `CPed::CanWeRunAndFireWithWeapon` (0x5E88E0): bit 1 of the flags (+0x18) of
/// `GetWeaponInfo(w, GetWeaponSkill(w))` for the active slot's weapon type.
#[hook(addr = 0x5E88E0, conv = thiscall, name = "CPed::CanWeRunAndFireWithWeapon", expect = "0F BE 81 18 07 00 00", this = "CPed")]
pub fn can_we_run_and_fire_with_weapon(this: GPtr<u8>) -> u8 {
    let slot = i32::from(rd::<i8>(this, 0x718));
    let ty = rd::<i32>(this, (slot * 0x1C + 0x5A0) as isize);
    let skill = get_weapon_skill_type_call(this, ty);
    let wi = get_weapon_info_call(ty, i32::from(skill));
    ((rd::<u32>(wi, 0x18) >> 1) & 1) as u8
}

pub const HOOKS: &[HookReg] = &[
    HOOK_DELETE_RW_OBJECT,
    HOOK_INITIALISE,
    HOOK_DISABLE_PED_SPEECH,
    HOOK_DISABLE_PED_SPEECH_FOR_SCRIPT_SPEECH,
    HOOK_CAN_PED_HOLD_CONVERSATION,
    HOOK_IS_DPAD_UP_PRESSED,
    HOOK_ANY_OF_96917C,
    HOOK_CTOR_85B4C8,
    HOOK_CAN_WE_RUN_AND_FIRE_WITH_WEAPON,
];
