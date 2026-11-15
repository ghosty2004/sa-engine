//! FRONTIER-2 part 4 (batch `frontier2-7`): the `CEvent*` destructors that ported code reached unported. The user hit
//! the first one shooting a driven car in free roam: the event group deletes its clone of the
//! `CEventVehicleDamageWeapon` the shot gave the driver (scalar deleting 0x61C390, generated) and stopped on its
//! complete destructor 0x61C310. The rest are the other complete destructors `gen_dtors.rs` and the event code call
//! unported (`vtbl = class; tail into the base destructor`, or the `CleanUpOldReference` shape with an SEH frame), and
//! the scalar deleting destructors of the event vtables whose pool delete is inlined. Classes of the unnamed vtables
//! from their `GetEventType` slot (eEventType). Entries that start in a no-CD protection fragment (`nop; jmp frag` or
//! `push -1; jmp frag`, the fragment storing the vtable or the SEH handler and jumping back) are ported with the
//! fragment inlined. Ported from the 1.0 US bytes.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{editable_response_dtor, event_base_dtor, event_vehicle_damage_dtor};
use crate::common::*;
use crate::events::sdd_inline;
use crate::mvpa_ai2::event_dtor_with_ref;
use crate::bindings::event_player_command_to_group_dtor;
use sa_world::bindings::{entity_clean_up_old_reference, event_acquaintance_ped_dtor, event_attractor_dtor};

orig! {
    /// `CEventRevive::Destructor` (0x4AECA0)
    fn event_revive_dtor_b(e: GPtr<u8>) = thiscall 0x4AECA0 => event_revive_dtor;
    /// `CEventSpecial::Destructor` (0x4B1B50)
    fn event_special_dtor_b(e: GPtr<u8>) = thiscall 0x4B1B50 => event_special_dtor;
}
use crate::bindings::event_fire_nearby_dtor;

/// `CEventVehicleDamageWeapon::Destructor` (0x61C310): `vtbl = 0x86D640`, then a tail jump into
/// `CEventVehicleDamage`'s destructor (0x4B1990). User-hit (free roam, shooting a driven car).
#[hook(addr = 0x61C310, conv = thiscall, name = "CEventVehicleDamageWeapon::Destructor", expect = "C7 01 40 D6 86 00 E9",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes", recs = "weapon-combat")]
pub fn event_vehicle_damage_weapon_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_D640);
    event_vehicle_damage_dtor(this);
}

/// `CEventVehicleDied::Destructor` (0x4B0DA0): vtable 0x85B400, releases the vehicle reference (+0xC), then the
/// `CEvent` vtable.
#[hook(addr = 0x4B0DA0, conv = thiscall, name = "CEventVehicleDied::Destructor", expect = "6A FF 68 58 A0 83 00",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_vehicle_died_dtor(this: GPtr<u8>) {
    event_dtor_with_ref(this, 0x85_B400, 0xC);
}

/// `CEventRevive::Destructor` (0x4AECA0, vtable 0x85B030, event type 18; folded): only the `CEvent` vtable. Its
/// scalar deleting destructor 0x4B6080 reads ECX after the call.
#[hook(addr = 0x4AECA0, conv = thiscall, name = "CEventRevive::Destructor", expect = "C7 01 38 AB 85 00 C3", preserve = "ecx",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes,preserved-regs")]
pub fn event_revive_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85_AB38);
}

/// `CEventSexyVehicle::Destructor` (0x4AF0A0, vtable 0x85B0F8, event type 21): releases the vehicle reference
/// (+0xC), then the `CEvent` vtable.
#[hook(addr = 0x4AF0A0, conv = thiscall, name = "CEventSexyVehicle::Destructor", expect = "6A FF 68 A8 9D 83 00",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_sexy_vehicle_dtor(this: GPtr<u8>) {
    event_dtor_with_ref(this, 0x85_B0F8, 0xC);
}

/// `CEventVehicleThreat::Destructor` (0x4AFBF0, vtable 0x85B298, event type 30): releases the vehicle reference
/// (+0x14), then the `CEvent` vtable.
#[hook(addr = 0x4AFBF0, conv = thiscall, name = "CEventVehicleThreat::Destructor", expect = "6A FF 68 28 9F 83 00",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_vehicle_threat_dtor(this: GPtr<u8>) {
    event_dtor_with_ref(this, 0x85_B298, 0x14);
}

/// `CEventVehicleHitAndRun::Destructor` (0x4AEA50; `push -1; jmp 0x40218C`, the fragment pushes the SEH handler):
/// vtable 0x85AFA8, releases the references at +0x10 and +0xC (in that order), then the `CEvent` vtable.
#[hook(addr = 0x4AEA50, conv = thiscall, name = "CEventVehicleHitAndRun::Destructor", expect = "6A FF E9 35 37 F5 FF",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_vehicle_hit_and_run_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85_AFA8);
    for off in [0x10, 0xC] {
        let e = rd::<GPtr<u8>>(this, off);
        if !e.is_null() {
            entity_clean_up_old_reference(e, this.byte_add(off));
        }
    }
    wr::<u32>(this, 0, 0x85_AB38);
}

/// `CEventDontJoinPlayerGroup::Destructor` (0x609140; `push -1; jmp 0x4021BC`): vtable 0x86D0E0, releases the
/// player reference (+0xC), then calls `CEvent`'s destructor 0x4AC010.
/// Phase B only: the no-CD protection reads its entry bytes (0x1569A80 reads 0x609143).
#[sa_core::native(addr = 0x609140, conv = thiscall, name = "CEventDontJoinPlayerGroup::Destructor")]
pub fn event_dont_join_player_group_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_D0E0);
    let e = rd::<GPtr<u8>>(this, 0xC);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(0xC));
    }
    event_base_dtor(this);
}

/// `CEventScriptedAttractor::Destructor` (0x5FEF70): vtable 0x86C9F0, then `CEventAttractor`'s destructor 0x4AF400.
#[hook(addr = 0x5FEF70, conv = thiscall, name = "CEventScriptedAttractor::Destructor", expect = "C7 01 F0 C9 86 00 E9",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_scripted_attractor_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_C9F0);
    event_attractor_dtor(this);
}

/// `CEventAcquaintancePedHateBadlyLit::Destructor` (0x5FF2C0): vtable 0x86CB10, then `CEventAcquaintancePed`'s
/// destructor 0x4AF890.
#[hook(addr = 0x5FF2C0, conv = thiscall, name = "CEventAcquaintancePedHateBadlyLit::Destructor",
       expect = "C7 01 10 CB 86 00 E9", unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_acquaintance_ped_hate_badly_lit_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_CB10);
    event_acquaintance_ped_dtor(this.cast());
}

/// `CEventHealthReallyLow::Destructor` (0x5FF520, vtable 0x86CBE8, event type 52): then `CEventEditableResponse`'s
/// destructor 0x4AC480.
#[hook(addr = 0x5FF520, conv = thiscall, name = "CEventHealthReallyLow::Destructor", expect = "C7 01 E8 CB 86 00 E9",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_health_really_low_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_CBE8);
    editable_response_dtor(this);
}

/// `CEventLowAngerAtPlayer::Destructor` (0x5FF5E0, vtable 0x86CC30, event type 50): then `CEventEditableResponse`'s
/// destructor 0x4AC480.
#[hook(addr = 0x5FF5E0, conv = thiscall, name = "CEventLowAngerAtPlayer::Destructor", expect = "C7 01 30 CC 86 00 E9",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_low_anger_at_player_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_CC30);
    editable_response_dtor(this);
}

/// `CEventHealthLow::Destructor` (0x5FF460, vtable 0x86CBA0, event type 53; `nop; jmp 0x401A42`, the fragment stores
/// the vtable): then `CEventEditableResponse`'s destructor 0x4AC480.
/// Phase B only: the no-CD protection reads its entry bytes (0x401673 reads 0x5FF461).
#[sa_core::native(addr = 0x5FF460, conv = thiscall, name = "CEventHealthLow::Destructor")]
pub fn event_health_low_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_CBA0);
    editable_response_dtor(this);
}

/// `CEventOnFire::Destructor` (0x5FF760): vtable 0x86CCC0, then `CEvent`'s destructor 0x4AC010.
#[hook(addr = 0x5FF760, conv = thiscall, name = "CEventOnFire::Destructor", expect = "C7 01 C0 CC 86 00 E9",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_on_fire_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_CCC0);
    event_base_dtor(this);
}

/// `CEventPlayerCommandToGroupAttack::Destructor` (0x5F63F0; `nop; jmp 0x40A472`): vtable 0x86C710, then
/// `CEventPlayerCommandToGroup`'s destructor 0x4B2470.
#[hook(addr = 0x5F63F0, conv = thiscall, name = "CEventPlayerCommandToGroupAttack::Destructor", expect = "90 E9 7C 40 E1 FF",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_player_command_to_group_attack_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_C710);
    event_player_command_to_group_dtor(this);
}

/// `CEventInteriorUseInfo::Destructor` (0x674ED0, vtable 0x8701A8, event type 81; `nop; jmp 0x401AC8`): then
/// `CEvent`'s destructor 0x4AC010.
#[hook(addr = 0x674ED0, conv = thiscall, name = "CEventInteriorUseInfo::Destructor", expect = "90 E9 F2 CB D8 FF",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_interior_use_info_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x87_01A8);
    event_base_dtor(this);
}

/// `CEventEscalator::Destructor` (0x5FF860): a bare `jmp` into `CEvent`'s destructor 0x4AC010. Not hookable (the
/// entry is a bare `jmp rel32`): phase B only.
#[sa_core::native(addr = 0x5FF860, conv = thiscall, name = "CEventEscalator::Destructor")]
pub fn event_escalator_dtor(this: GPtr<u8>) {
    event_base_dtor(this);
}

/// `CEventHitByWaterCannon::Destructor` (0x4B1320, folded): only the `CEvent` vtable; 0x4B65C0 reads ECX after it.
#[hook(addr = 0x4B1320, conv = thiscall, name = "CEventHitByWaterCannon::Destructor", expect = "C7 01 38 AB 85 00 C3", preserve = "ecx",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes,preserved-regs")]
pub fn event_hit_by_water_cannon_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85_AB38);
}

/// `CEventSpecial::Destructor` (0x4B1B50, folded): only the `CEvent` vtable; 0x4B6750 reads ECX after it.
#[hook(addr = 0x4B1B50, conv = thiscall, name = "CEventSpecial::Destructor", expect = "C7 01 38 AB 85 00 C3", preserve = "ecx",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes,preserved-regs")]
pub fn event_special_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85_AB38);
}

/// `CEventEditableResponse::~CEventEditableResponse` (0x4B5670, scalar deleting): 0x4AC480, then the inlined event
/// pool delete when `flags & 1`.
#[hook(addr = 0x4B5670, conv = thiscall, name = "CEventEditableResponse::~CEventEditableResponse", expect = "56 E8 0A 6E FF FF",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_editable_response_sdd(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, editable_response_dtor)
}

/// `CEventRevive::~CEventRevive` (0x4B6080, scalar deleting): 0x4AECA0, inlined pool delete.
#[hook(addr = 0x4B6080, conv = thiscall, name = "CEventRevive::~CEventRevive", expect = "56 E8 1A 8C FF FF",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_revive_sdd(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, event_revive_dtor_b)
}

/// `CEventHitByWaterCannon::~CEventHitByWaterCannon` (0x4B65C0, scalar deleting): 0x4B1320, inlined pool delete.
#[hook(addr = 0x4B65C0, conv = thiscall, name = "CEventHitByWaterCannon::~CEventHitByWaterCannon", expect = "56 E8 5A AD FF FF",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_hit_by_water_cannon_sdd(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, crate::bindings::event_hit_by_water_cannon_dtor)
}

/// `CEventSpecial::~CEventSpecial` (0x4B6750, scalar deleting): 0x4B1B50, inlined pool delete.
#[hook(addr = 0x4B6750, conv = thiscall, name = "CEventSpecial::~CEventSpecial", expect = "56 E8 FA B3 FF FF",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_special_sdd(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, event_special_dtor_b)
}

/// `CEventFireNearby::~CEventFireNearby` (0x4B6940, scalar deleting): 0x4B1F80, inlined pool delete.
#[hook(addr = 0x4B6940, conv = thiscall, name = "CEventFireNearby::~CEventFireNearby", expect = "56 E8 3A B6 FF FF",
       unverified = "frontier2-7", route = "L2", hazards = "heap-writes")]
pub fn event_fire_nearby_sdd(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    sdd_inline(this, flags, event_fire_nearby_dtor)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_EVENT_VEHICLE_DAMAGE_WEAPON_DTOR,
    HOOK_EVENT_VEHICLE_DIED_DTOR,
    HOOK_EVENT_REVIVE_DTOR,
    HOOK_EVENT_SEXY_VEHICLE_DTOR,
    HOOK_EVENT_VEHICLE_THREAT_DTOR,
    HOOK_EVENT_VEHICLE_HIT_AND_RUN_DTOR,

    HOOK_EVENT_SCRIPTED_ATTRACTOR_DTOR,
    HOOK_EVENT_ACQUAINTANCE_PED_HATE_BADLY_LIT_DTOR,
    HOOK_EVENT_HEALTH_REALLY_LOW_DTOR,
    HOOK_EVENT_LOW_ANGER_AT_PLAYER_DTOR,

    HOOK_EVENT_ON_FIRE_DTOR,
    HOOK_EVENT_PLAYER_COMMAND_TO_GROUP_ATTACK_DTOR,
    HOOK_EVENT_INTERIOR_USE_INFO_DTOR,
    HOOK_EVENT_HIT_BY_WATER_CANNON_DTOR,
    HOOK_EVENT_SPECIAL_DTOR,
    HOOK_EVENT_EDITABLE_RESPONSE_SDD,
    HOOK_EVENT_REVIVE_SDD,
    HOOK_EVENT_HIT_BY_WATER_CANNON_SDD,
    HOOK_EVENT_SPECIAL_SDD,
    HOOK_EVENT_FIRE_NEARBY_SDD,
];

/// This module's phase-B registrations.
pub const NATIVES: &[sa_core::abi::NativeReg] =
    &[NATIVE_EVENT_ESCALATOR_DTOR, NATIVE_EVENT_DONT_JOIN_PLAYER_GROUP_DTOR, NATIVE_EVENT_HEALTH_LOW_DTOR];
