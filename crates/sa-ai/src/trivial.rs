//! Byte-pattern AI leaves: constant `GetTaskType`/`GetEventType`/`GetEventPriority`/`GetLifeTime`, constant `bool`
//! virtuals, field getters and the static initialisers that copy one dword.
//! Generated from the 1.0 US bytes (every body below was matched instruction by instruction; the generator is
//! described in `docs/arch/ai-port.md`). Edit by hand only together with that table.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

/// `CEvent::DoInformVehicleOccupants` (0x420EC0): `xor al, al; ret 4` -> false (AL).
#[hook(addr = 0x420EC0, conv = thiscall, name = "CEvent::DoInformVehicleOccupants", expect = "32 C0 C2 04 00")]
pub fn event_do_inform_vehicle_occupants_420ec0(_this: GPtr<u8>, _a0: u32) -> u8 {
    0
}

/// `CEventAcquaintancePed::GetSourceEntity` (0x420F10): `mov eax, dword ptr [ecx + 0x14]; ret`.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn event_acquaintance_ped_get_source_entity_420f10(this: GPtr<u8>) -> u32 {
    rd::<u32>(this, 0x14)
}

/// `CEventAcquaintancePedHate::GetEventType` (0x420F40): `mov eax, 0x24; ret` -> 36.
/// Not hookable: protection code reads the patched bytes (`push 0x420F44` at 0x146747B, INF-20).
pub fn event_acquaintance_ped_hate_get_event_type_420f40(_this: GPtr<u8>) -> u32 {
    0x24
}

/// `CEventAcquaintancePedHate::GetEventPriority` (0x420FD0): `mov eax, 0x1a; ret` -> 26.
#[hook(addr = 0x420FD0, conv = thiscall, name = "CEventAcquaintancePedHate::GetEventPriority", expect = "B8 1A 00 00 00")]
pub fn event_acquaintance_ped_hate_get_event_priority_420fd0(_this: GPtr<u8>) -> u32 {
    0x1a
}

/// `CTaskComplex::GetSubTask` (0x421190): `mov eax, dword ptr [ecx + 8]; ret`.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn task_complex_get_sub_task_421190(this: GPtr<u8>) -> u32 {
    rd::<u32>(this, 0x8)
}

/// `CTaskComplex::IsSimple` (0x4211A0): `xor al, al; ret` -> false (AL).
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn task_complex_is_simple_4211a0(_this: GPtr<u8>) -> u8 {
    0
}

/// `CEvent::HasEditableResponse` (0x43E1E0): `xor al, al; ret` -> false (AL).
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn event_has_editable_response_43e1e0(_this: GPtr<u8>) -> u8 {
    0
}

/// `CEvent::GetSourceEntity` (0x43E1F0): `xor eax, eax; ret` -> 0.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn event_get_source_entity_43e1f0(_this: GPtr<u8>) -> u32 {
    0
}

/// `CTaskSimple::GetSubTask` (0x43E300): `xor eax, eax; ret` -> 0.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn task_simple_get_sub_task_43e300(_this: GPtr<u8>) -> u32 {
    0
}

/// `CTaskSimple::IsSimple` (0x43E310): `mov al, 1; ret` -> true (AL).
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn task_simple_is_simple_43e310(_this: GPtr<u8>) -> u8 {
    1
}

/// `CTaskSimple::SetPedPosition` (0x43E320): `xor al, al; ret 4` -> false (AL).
#[hook(addr = 0x43E320, conv = thiscall, name = "CTaskSimple::SetPedPosition", expect = "32 C0 C2 04 00")]
pub fn task_simple_set_ped_position_43e320(_this: GPtr<u8>, _a0: u32) -> u8 {
    0
}

/// `CTaskComplexWander::GetTaskType` (0x460CD0): `mov eax, 0x390; ret` -> 912.
#[hook(addr = 0x460CD0, conv = thiscall, name = "CTaskComplexWander::GetTaskType", expect = "B8 90 03 00 00")]
pub fn task_complex_wander_get_task_type_460cd0(_this: GPtr<u8>) -> u32 {
    0x390
}

/// `CTaskSimpleLeaveGroup::GetTaskType` (0x4635C0): `mov eax, 0xc8; ret` -> 200.
#[hook(addr = 0x4635C0, conv = thiscall, name = "CTaskSimpleLeaveGroup::GetTaskType", expect = "B8 C8 00 00 00")]
pub fn task_simple_leave_group_get_task_type_4635c0(_this: GPtr<u8>) -> u32 {
    0xc8
}

/// `CTaskSimpleNone::MakeAbortable` (0x4635D0): `mov al, 1; ret 12` -> true (AL).
#[hook(addr = 0x4635D0, conv = thiscall, name = "CTaskSimpleNone::MakeAbortable", expect = "B0 01 C2 0C 00")]
pub fn task_simple_none_make_abortable_4635d0(_this: GPtr<u8>, _a0: u32, _a1: u32, _a2: u32) -> u8 {
    1
}

/// `CTaskSimpleNone::ProcessPed` (0x4635E0): `mov al, 1; ret 4` -> true (AL).
#[hook(addr = 0x4635E0, conv = thiscall, name = "CTaskSimpleNone::ProcessPed", expect = "B0 01 C2 04 00")]
pub fn task_simple_none_process_ped_4635e0(_this: GPtr<u8>, _a0: u32) -> u8 {
    1
}

/// `CTaskSimpleCarDriveTimed::GetTaskType` (0x46F600): `mov eax, 0x2cf; ret` -> 719.
#[hook(addr = 0x46F600, conv = thiscall, name = "CTaskSimpleCarDriveTimed::GetTaskType", expect = "B8 CF 02 00 00")]
pub fn task_simple_car_drive_timed_get_task_type_46f600(_this: GPtr<u8>) -> u32 {
    0x2cf
}

/// `CTaskSimpleRunAnim::GetTaskType` (0x48DEA0): `movzx eax, word ptr [ecx + 0x1c]; ret`.
#[hook(addr = 0x48DEA0, conv = thiscall, name = "CTaskSimpleRunAnim::GetTaskType", expect = "0F B7 41 1C C3")]
pub fn task_simple_run_anim_get_task_type_48dea0(this: GPtr<u8>) -> u32 {
    u32::from(rd::<u16>(this, 0x1c))
}

/// `CTaskComplexWanderStandard::GetWanderType` (0x48E5D0): `xor eax, eax; ret` -> 0.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn task_complex_wander_standard_get_wander_type_48e5d0(_this: GPtr<u8>) -> u32 {
    0
}

/// `CTaskSimplePause::GetTaskType` (0x48E800): `mov eax, 0xca; ret` -> 202.
#[hook(addr = 0x48E800, conv = thiscall, name = "CTaskSimplePause::GetTaskType", expect = "B8 CA 00 00 00")]
pub fn task_simple_pause_get_task_type_48e800(_this: GPtr<u8>) -> u32 {
    0xca
}

/// `CEvent::AffectsPedGroup` (0x4ABFF0): `mov al, 1; ret 4` -> true (AL).
#[hook(addr = 0x4ABFF0, conv = thiscall, name = "CEvent::AffectsPedGroup", expect = "B0 01 C2 04 00")]
pub fn event_affects_ped_group_4abff0(_this: GPtr<u8>, _a0: u32) -> u8 {
    1
}

/// `CEvent::CanBeInterruptedBySameEvent` (0x4AC000): `xor al, al; ret` -> false (AL).
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn event_can_be_interrupted_by_same_event_4ac000(_this: GPtr<u8>) -> u8 {
    0
}

/// `CEventPedCollisionWithPed::GetEventType` (0x4ACA40): `mov eax, 0x2; ret` -> 2.
#[hook(addr = 0x4ACA40, conv = thiscall, name = "CEventPedCollisionWithPed::GetEventType", expect = "B8 02 00 00 00")]
pub fn event_ped_collision_with_ped_get_event_type_4aca40(_this: GPtr<u8>) -> u32 {
    0x2
}

/// `CEventPedCollisionWithPed::GetLifeTime` (0x4ACA50): `xor eax, eax; ret` -> 0.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn event_ped_collision_with_ped_get_life_time_4aca50(_this: GPtr<u8>) -> u32 {
    0
}

/// `CEventPedCollisionWithPed::GetEventPriority` (0x4ACA60): `mov eax, 0x3c; ret` -> 60.
#[hook(addr = 0x4ACA60, conv = thiscall, name = "CEventPedCollisionWithPed::GetEventPriority", expect = "B8 3C 00 00 00")]
pub fn event_ped_collision_with_ped_get_event_priority_4aca60(_this: GPtr<u8>) -> u32 {
    0x3c
}

/// `CEventObjectCollision::GetEventType` (0x4ACDA0): `mov eax, 0x5; ret` -> 5.
#[hook(addr = 0x4ACDA0, conv = thiscall, name = "CEventObjectCollision::GetEventType", expect = "B8 05 00 00 00")]
pub fn event_object_collision_get_event_type_4acda0(_this: GPtr<u8>) -> u32 {
    0x5
}

/// `CEventObjectCollision::GetEventPriority` (0x4ACDC0): `mov eax, 0x39; ret` -> 57.
#[hook(addr = 0x4ACDC0, conv = thiscall, name = "CEventObjectCollision::GetEventPriority", expect = "B8 39 00 00 00")]
pub fn event_object_collision_get_event_priority_4acdc0(_this: GPtr<u8>) -> u32 {
    0x39
}

/// `CEventBuildingCollision::GetEventType` (0x4ACFB0): `mov eax, 0x6; ret` -> 6.
#[hook(addr = 0x4ACFB0, conv = thiscall, name = "CEventBuildingCollision::GetEventType", expect = "B8 06 00 00 00")]
pub fn event_building_collision_get_event_type_4acfb0(_this: GPtr<u8>) -> u32 {
    0x6
}

/// `CEventBuildingCollision::GetLifeTime` (0x4ACFC0): `xor eax, eax; ret` -> 0.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn event_building_collision_get_life_time_4acfc0(_this: GPtr<u8>) -> u32 {
    0
}

/// `CEventBuildingCollision::CanBeInterruptedBySameEvent` (0x4ACFD0): `mov al, 1; ret` -> true (AL).
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn event_building_collision_can_be_interrupted_by_same_event_4acfd0(_this: GPtr<u8>) -> u8 {
    1
}

/// `CEventBuildingCollision::GetEventPriority` (0x4ACFE0): `mov eax, 0x3b; ret` -> 59.
#[hook(addr = 0x4ACFE0, conv = thiscall, name = "CEventBuildingCollision::GetEventPriority", expect = "B8 3B 00 00 00")]
pub fn event_building_collision_get_event_priority_4acfe0(_this: GPtr<u8>) -> u32 {
    0x3b
}

/// `CEventGroupEvent::GetEventType` (0x4AE030): `mov eax, 0x2f; ret` -> 47.
#[hook(addr = 0x4AE030, conv = thiscall, name = "CEventGroupEvent::GetEventType", expect = "B8 2F 00 00 00")]
pub fn event_group_event_get_event_type_4ae030(_this: GPtr<u8>) -> u32 {
    0x2f
}

/// `vt_85ae88::GetEventType` (0x4AE210): `mov eax, 0xc; ret` -> 12.
#[hook(addr = 0x4AE210, conv = thiscall, name = "vt_85ae88::GetEventType", expect = "B8 0C 00 00 00")]
pub fn vt_85ae88_get_event_type_4ae210(_this: GPtr<u8>) -> u32 {
    0xc
}

/// `vt_85ae88::GetEventPriority` (0x4AE260): `mov eax, 0x33; ret` -> 51.
#[hook(addr = 0x4AE260, conv = thiscall, name = "vt_85ae88::GetEventPriority", expect = "B8 33 00 00 00")]
pub fn vt_85ae88_get_event_priority_4ae260(_this: GPtr<u8>) -> u32 {
    0x33
}

/// `CEventPotentialWalkIntoVehicle::GetEventType` (0x4AE390): `mov eax, 0x38; ret` -> 56.
#[hook(addr = 0x4AE390, conv = thiscall, name = "CEventPotentialWalkIntoVehicle::GetEventType", expect = "B8 38 00 00 00")]
pub fn event_potential_walk_into_vehicle_get_event_type_4ae390(_this: GPtr<u8>) -> u32 {
    0x38
}

/// `CEventPotentialWalkIntoVehicle::GetEventPriority` (0x4AE3B0): `mov eax, 0x1d; ret` -> 29.
#[hook(addr = 0x4AE3B0, conv = thiscall, name = "CEventPotentialWalkIntoVehicle::GetEventPriority", expect = "B8 1D 00 00 00")]
pub fn event_potential_walk_into_vehicle_get_event_priority_4ae3b0(_this: GPtr<u8>) -> u32 {
    0x1d
}

/// `CEventPotentialWalkIntoPed::GetEventType` (0x4AE760): `mov eax, 0xd; ret` -> 13.
#[hook(addr = 0x4AE760, conv = thiscall, name = "CEventPotentialWalkIntoPed::GetEventType", expect = "B8 0D 00 00 00")]
pub fn event_potential_walk_into_ped_get_event_type_4ae760(_this: GPtr<u8>) -> u32 {
    0xd
}

/// `CEventPotentialWalkIntoPed::GetLifeTime` (0x4AE770): `xor eax, eax; ret` -> 0.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn event_potential_walk_into_ped_get_life_time_4ae770(_this: GPtr<u8>) -> u32 {
    0
}

/// `CEventPotentialWalkIntoPed::GetSourceEntity` (0x4AE780): `mov eax, dword ptr [ecx + 0x20]; ret`.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn event_potential_walk_into_ped_get_source_entity_4ae780(this: GPtr<u8>) -> u32 {
    rd::<u32>(this, 0x20)
}

/// `CEventPotentialWalkIntoPed::GetEventPriority` (0x4AE790): `mov eax, 0x1c; ret` -> 28.
#[hook(addr = 0x4AE790, conv = thiscall, name = "CEventPotentialWalkIntoPed::GetEventPriority", expect = "B8 1C 00 00 00")]
pub fn event_potential_walk_into_ped_get_event_priority_4ae790(_this: GPtr<u8>) -> u32 {
    0x1c
}

/// `vt_85b070::GetEventType` (0x4AED30): `mov eax, 0x13; ret` -> 19.
#[hook(addr = 0x4AED30, conv = thiscall, name = "vt_85b070::GetEventType", expect = "B8 13 00 00 00")]
pub fn vt_85b070_get_event_type_4aed30(_this: GPtr<u8>) -> u32 {
    0x13
}

/// `vt_85b070::GetEventPriority` (0x4AED50): `mov eax, 0x5; ret` -> 5.
#[hook(addr = 0x4AED50, conv = thiscall, name = "vt_85b070::GetEventPriority", expect = "B8 05 00 00 00")]
pub fn vt_85b070_get_event_priority_4aed50(_this: GPtr<u8>) -> u32 {
    0x5
}

/// `vt_85b0b0::GetEventType` (0x4AEE60): `mov eax, 0x14; ret` -> 20.
#[hook(addr = 0x4AEE60, conv = thiscall, name = "vt_85b0b0::GetEventType", expect = "B8 14 00 00 00")]
pub fn vt_85b0b0_get_event_type_4aee60(_this: GPtr<u8>) -> u32 {
    0x14
}

/// `vt_85b0b0::GetSourceEntity` (0x4AEE80): `mov eax, dword ptr [ecx + 0x14]; ret`.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn vt_85b0b0_get_source_entity_4aee80(this: GPtr<u8>) -> u32 {
    rd::<u32>(this, 0x14)
}

/// `vt_85b0b0::GetEventPriority` (0x4AEE90): `mov eax, 0x4; ret` -> 4.
#[hook(addr = 0x4AEE90, conv = thiscall, name = "vt_85b0b0::GetEventPriority", expect = "B8 04 00 00 00")]
pub fn vt_85b0b0_get_event_priority_4aee90(_this: GPtr<u8>) -> u32 {
    0x4
}

/// `vt_85b0f8::GetEventType` (0x4AF070): `mov eax, 0x15; ret` -> 21.
#[hook(addr = 0x4AF070, conv = thiscall, name = "vt_85b0f8::GetEventType", expect = "B8 15 00 00 00")]
pub fn vt_85b0f8_get_event_type_4af070(_this: GPtr<u8>) -> u32 {
    0x15
}

/// `vt_85b0f8::GetEventPriority` (0x4AF090): `mov eax, 0x3; ret` -> 3.
#[hook(addr = 0x4AF090, conv = thiscall, name = "vt_85b0f8::GetEventPriority", expect = "B8 03 00 00 00")]
pub fn vt_85b0f8_get_event_priority_4af090(_this: GPtr<u8>) -> u32 {
    0x3
}

/// `CEventAttractor::GetEventType` (0x4AF3D0): `mov eax, 0x1b; ret` -> 27.
#[hook(addr = 0x4AF3D0, conv = thiscall, name = "CEventAttractor::GetEventType", expect = "B8 1B 00 00 00")]
pub fn event_attractor_get_event_type_4af3d0(_this: GPtr<u8>) -> u32 {
    0x1b
}

/// `CEventAttractor::GetEventPriority` (0x4AF3F0): `xor eax, eax; ret` -> 0.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn event_attractor_get_event_priority_4af3f0(_this: GPtr<u8>) -> u32 {
    0
}

/// `CEventInAir::GetEventType` (0x4B0CD0): `mov eax, 0x21; ret` -> 33.
#[hook(addr = 0x4B0CD0, conv = thiscall, name = "CEventInAir::GetEventType", expect = "B8 21 00 00 00")]
pub fn event_in_air_get_event_type_4b0cd0(_this: GPtr<u8>) -> u32 {
    0x21
}

/// `CEventInAir::GetEventPriority` (0x4B0CF0): `mov eax, 0x3d; ret` -> 61.
#[hook(addr = 0x4B0CF0, conv = thiscall, name = "CEventInAir::GetEventPriority", expect = "B8 3D 00 00 00")]
pub fn event_in_air_get_event_priority_4b0cf0(_this: GPtr<u8>) -> u32 {
    0x3d
}

/// `CEventVehicleDamage::GetEventPriority` (0x4B1980): `mov eax, 0x31; ret` -> 49.
#[hook(addr = 0x4B1980, conv = thiscall, name = "CEventVehicleDamage::GetEventPriority", expect = "B8 31 00 00 00")]
pub fn event_vehicle_damage_get_event_priority_4b1980(_this: GPtr<u8>) -> u32 {
    0x31
}

/// `CEventSoundQuiet::GetEventType` (0x5DEA60): `mov eax, 0x3e; ret` -> 62.
#[hook(addr = 0x5DEA60, conv = thiscall, name = "CEventSoundQuiet::GetEventType", expect = "B8 3E 00 00 00")]
pub fn event_sound_quiet_get_event_type_5dea60(_this: GPtr<u8>) -> u32 {
    0x3e
}

/// `CEventSoundQuiet::GetSourceEntity` (0x5DEA80): `mov eax, dword ptr [ecx + 0x14]; ret`.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn event_sound_quiet_get_source_entity_5dea80(this: GPtr<u8>) -> u32 {
    rd::<u32>(this, 0x14)
}

/// `CEventSoundQuiet::GetLocalSoundLevel` (0x5DEA90): `fld dword ptr [ecx + 0x18]; ret` (ST0).
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn event_sound_quiet_get_local_sound_level_5dea90(this: GPtr<u8>) -> X87 {
    x(rd::<f32>(this, 0x18))
}

/// `CEventSoundQuiet::GetEventPriority` (0x5DEAD0): `mov eax, 0x8; ret` -> 8.
#[hook(addr = 0x5DEAD0, conv = thiscall, name = "CEventSoundQuiet::GetEventPriority", expect = "B8 08 00 00 00")]
pub fn event_sound_quiet_get_event_priority_5dead0(_this: GPtr<u8>) -> u32 {
    0x8
}

/// `CPedStepAttractor::GetType` (0x5EEC30): `mov eax, 0x9; ret` -> 9.
#[hook(addr = 0x5EEC30, conv = thiscall, name = "CPedStepAttractor::GetType", expect = "B8 09 00 00 00")]
pub fn cped_step_attractor_get_type_5eec30(_this: GPtr<u8>) -> u32 {
    0x9
}

/// `vt_86c6d0::GetEventType` (0x5F6260): `mov eax, 0x46; ret` -> 70.
#[hook(addr = 0x5F6260, conv = thiscall, name = "vt_86c6d0::GetEventType", expect = "B8 46 00 00 00")]
pub fn vt_86c6d0_get_event_type_5f6260(_this: GPtr<u8>) -> u32 {
    0x46
}

/// `vt_86c6d0::AffectsPed` (0x5F62F0): `mov al, 1; ret 4` -> true (AL).
#[hook(addr = 0x5F62F0, conv = thiscall, name = "vt_86c6d0::AffectsPed", expect = "B0 01 C2 04 00")]
pub fn vt_86c6d0_affects_ped_5f62f0(_this: GPtr<u8>, _a0: u32) -> u8 {
    1
}

/// `CEventPlayerCommandToGroupAttack::GetEventType` (0x5F6360): `mov eax, 0x47; ret` -> 71.
#[hook(addr = 0x5F6360, conv = thiscall, name = "CEventPlayerCommandToGroupAttack::GetEventType", expect = "B8 47 00 00 00")]
pub fn event_player_command_to_group_attack_get_event_type_5f6360(_this: GPtr<u8>) -> u32 {
    0x47
}

/// `vt_86c6d0::GetEventPriority` (0x5F6410): `mov eax, 0x6; ret` -> 6.
#[hook(addr = 0x5F6410, conv = thiscall, name = "vt_86c6d0::GetEventPriority", expect = "B8 06 00 00 00")]
pub fn vt_86c6d0_get_event_priority_5f6410(_this: GPtr<u8>) -> u32 {
    0x6
}

/// `CEventPedCollisionWithPlayer::GetEventType` (0x5FED90): `mov eax, 0x3; ret` -> 3.
#[hook(addr = 0x5FED90, conv = thiscall, name = "CEventPedCollisionWithPlayer::GetEventType", expect = "B8 03 00 00 00")]
pub fn event_ped_collision_with_player_get_event_type_5fed90(_this: GPtr<u8>) -> u32 {
    0x3
}

/// `vt_86ca38::GetEventType` (0x5FF040): `mov eax, 0x27; ret` -> 39.
#[hook(addr = 0x5FF040, conv = thiscall, name = "vt_86ca38::GetEventType", expect = "B8 27 00 00 00")]
pub fn vt_86ca38_get_event_type_5ff040(_this: GPtr<u8>) -> u32 {
    0x27
}

/// `vt_86ca80::GetEventType` (0x5FF100): `mov eax, 0x26; ret` -> 38.
#[hook(addr = 0x5FF100, conv = thiscall, name = "vt_86ca80::GetEventType", expect = "B8 26 00 00 00")]
pub fn vt_86ca80_get_event_type_5ff100(_this: GPtr<u8>) -> u32 {
    0x26
}

/// `vt_86cac8::GetEventType` (0x5FF1C0): `mov eax, 0x25; ret` -> 37.
#[hook(addr = 0x5FF1C0, conv = thiscall, name = "vt_86cac8::GetEventType", expect = "B8 25 00 00 00")]
pub fn vt_86cac8_get_event_type_5ff1c0(_this: GPtr<u8>) -> u32 {
    0x25
}

/// `CEventSeenCop::GetEventType` (0x5FF3B0): `mov eax, 0x48; ret` -> 72.
#[hook(addr = 0x5FF3B0, conv = thiscall, name = "CEventSeenCop::GetEventType", expect = "B8 48 00 00 00")]
pub fn event_seen_cop_get_event_type_5ff3b0(_this: GPtr<u8>) -> u32 {
    0x48
}

/// `CEventSeenCop::GetEventPriority` (0x5FF880): `mov eax, 0x15; ret` -> 21.
#[hook(addr = 0x5FF880, conv = thiscall, name = "CEventSeenCop::GetEventPriority", expect = "B8 15 00 00 00")]
pub fn event_seen_cop_get_event_priority_5ff880(_this: GPtr<u8>) -> u32 {
    0x15
}

/// `vt_86ca38::GetEventPriority` (0x5FF8B0): `mov eax, 0x17; ret` -> 23.
#[hook(addr = 0x5FF8B0, conv = thiscall, name = "vt_86ca38::GetEventPriority", expect = "B8 17 00 00 00")]
pub fn vt_86ca38_get_event_priority_5ff8b0(_this: GPtr<u8>) -> u32 {
    0x17
}

/// `vt_86ca80::GetEventPriority` (0x5FF8C0): `mov eax, 0x16; ret` -> 22.
#[hook(addr = 0x5FF8C0, conv = thiscall, name = "vt_86ca80::GetEventPriority", expect = "B8 16 00 00 00")]
pub fn vt_86ca80_get_event_priority_5ff8c0(_this: GPtr<u8>) -> u32 {
    0x16
}

/// `vt_86cac8::GetEventPriority` (0x5FF8D0): `mov eax, 0x18; ret` -> 24.
#[hook(addr = 0x5FF8D0, conv = thiscall, name = "vt_86cac8::GetEventPriority", expect = "B8 18 00 00 00")]
pub fn vt_86cac8_get_event_priority_5ff8d0(_this: GPtr<u8>) -> u32 {
    0x18
}

/// `FUN_005ffee0` (0x5FFEE0): `mov eax, dword ptr [ecx + 0x4c]; ret`.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn fun_005ffee0_5ffee0(this: GPtr<u8>) -> u32 {
    rd::<u32>(this, 0x4c)
}

/// `CTaskSimpleRunNamedAnim::GetTaskType` (0x61AA90): `mov eax, 0x191; ret` -> 401.
#[hook(addr = 0x61AA90, conv = thiscall, name = "CTaskSimpleRunNamedAnim::GetTaskType", expect = "B8 91 01 00 00")]
pub fn task_simple_run_named_anim_get_task_type_61aa90(_this: GPtr<u8>) -> u32 {
    0x191
}

/// `CTaskSimplePlayHandSignalAnim::GetTaskType` (0x61AEA0): `mov eax, 0x1a9; ret` -> 425.
#[hook(addr = 0x61AEA0, conv = thiscall, name = "CTaskSimplePlayHandSignalAnim::GetTaskType", expect = "B8 A9 01 00 00")]
pub fn task_simple_play_hand_signal_anim_get_task_type_61aea0(_this: GPtr<u8>) -> u32 {
    0x1a9
}

/// `CTaskComplexPlayHandSignalAnim::CreateNextSubTask` (0x61B570): `xor eax, eax; ret 4` -> 0.
#[hook(addr = 0x61B570, conv = thiscall, name = "CTaskComplexPlayHandSignalAnim::CreateNextSubTask", expect = "33 C0 C2 04 00")]
pub fn task_complex_play_hand_signal_anim_create_next_sub_task_61b570(_this: GPtr<u8>, _a0: u32) -> u32 {
    0
}

/// `CTaskSimpleStandStill::GetTaskType` (0x62F350): `mov eax, 0xcb; ret` -> 203.
#[hook(addr = 0x62F350, conv = thiscall, name = "CTaskSimpleStandStill::GetTaskType", expect = "B8 CB 00 00 00")]
pub fn task_simple_stand_still_get_task_type_62f350(_this: GPtr<u8>) -> u32 {
    0xcb
}

/// `CTaskComplexObserveTrafficLights::GetTaskType` (0x6317B0): `mov eax, 0xe0; ret` -> 224.
#[hook(addr = 0x6317B0, conv = thiscall, name = "CTaskComplexObserveTrafficLights::GetTaskType", expect = "B8 E0 00 00 00")]
pub fn task_complex_observe_traffic_lights_get_task_type_6317b0(_this: GPtr<u8>) -> u32 {
    0xe0
}

/// `CTaskComplexObserveTrafficLights::MakeAbortable` (0x6317D0): `mov al, 1; ret 12` -> true (AL).
#[hook(addr = 0x6317D0, conv = thiscall, name = "CTaskComplexObserveTrafficLights::MakeAbortable", expect = "B0 01 C2 0C 00")]
pub fn task_complex_observe_traffic_lights_make_abortable_6317d0(
    _this: GPtr<u8>,
    _a0: u32,
    _a1: u32,
    _a2: u32,
) -> u8 {
    1
}

/// `CTaskComplexObserveTrafficLightsAndAchieveHeading::GetTaskType` (0x631940): `mov eax, 0xe1; ret` -> 225.
#[hook(addr = 0x631940, conv = thiscall, name = "CTaskComplexObserveTrafficLightsAndAchieveHeading::GetTaskType", expect = "B8 E1 00 00 00")]
pub fn task_complex_observe_traffic_lights_and_achieve_heading_get_task_type_631940(_this: GPtr<u8>) -> u32 {
    0xe1
}

/// `CTaskComplexSequence::GetTaskType` (0x632C60): `mov eax, 0xf4; ret` -> 244.
#[hook(addr = 0x632C60, conv = thiscall, name = "CTaskComplexSequence::GetTaskType", expect = "B8 F4 00 00 00")]
pub fn task_complex_sequence_get_task_type_632c60(_this: GPtr<u8>) -> u32 {
    0xf4
}

/// `CTaskComplexLeaveCar::GetTaskType` (0x63B960): `mov eax, 0x2c0; ret` -> 704.
#[hook(addr = 0x63B960, conv = thiscall, name = "CTaskComplexLeaveCar::GetTaskType", expect = "B8 C0 02 00 00")]
pub fn task_complex_leave_car_get_task_type_63b960(_this: GPtr<u8>) -> u32 {
    0x2c0
}

/// `CTaskComplexCarDriveWander::GetTaskType` (0x63CB40): `mov eax, 0x2c7; ret` -> 711.
#[hook(addr = 0x63CB40, conv = thiscall, name = "CTaskComplexCarDriveWander::GetTaskType", expect = "B8 C7 02 00 00")]
pub fn task_complex_car_drive_wander_get_task_type_63cb40(_this: GPtr<u8>) -> u32 {
    0x2c7
}

/// `CTaskComplexEnterCarAsDriver::GetTaskType` (0x640320): `mov eax, 0x2bd; ret` -> 701.
#[hook(addr = 0x640320, conv = thiscall, name = "CTaskComplexEnterCarAsDriver::GetTaskType", expect = "B8 BD 02 00 00")]
pub fn task_complex_enter_car_as_driver_get_task_type_640320(_this: GPtr<u8>) -> u32 {
    0x2bd
}

/// `CTaskSimpleCarSetPedOut::GetTaskType` (0x647930): `mov eax, 0x330; ret` -> 816.
#[hook(addr = 0x647930, conv = thiscall, name = "CTaskSimpleCarSetPedOut::GetTaskType", expect = "B8 30 03 00 00")]
pub fn task_simple_car_set_ped_out_get_task_type_647930(_this: GPtr<u8>) -> u32 {
    0x330
}

/// `CTaskSimpleEvasiveStep::GetTaskType` (0x6531C0): `mov eax, 0x1f5; ret` -> 501.
#[hook(addr = 0x6531C0, conv = thiscall, name = "CTaskSimpleEvasiveStep::GetTaskType", expect = "B8 F5 01 00 00")]
pub fn task_simple_evasive_step_get_task_type_6531c0(_this: GPtr<u8>) -> u32 {
    0x1f5
}

/// `CTaskComplexEvasiveStep::GetTaskType` (0x653340): `mov eax, 0x1f6; ret` -> 502.
#[hook(addr = 0x653340, conv = thiscall, name = "CTaskComplexEvasiveStep::GetTaskType", expect = "B8 F6 01 00 00")]
pub fn task_complex_evasive_step_get_task_type_653340(_this: GPtr<u8>) -> u32 {
    0x1f6
}

/// `CTaskComplexEvasiveStep::ControlSubTask` (0x653550): `mov eax, dword ptr [ecx + 8]; ret 4`.
#[hook(addr = 0x653550, conv = thiscall, name = "CTaskComplexEvasiveStep::ControlSubTask", expect = "8B 41 08 C2 04 00")]
pub fn task_complex_evasive_step_control_sub_task_653550(this: GPtr<u8>, _a0: u32) -> u32 {
    rd::<u32>(this, 0x8)
}

/// `CTaskComplexWalkRoundObject::GetTaskType` (0x6550C0): `mov eax, 0x1fe; ret` -> 510.
#[hook(addr = 0x6550C0, conv = thiscall, name = "CTaskComplexWalkRoundObject::GetTaskType", expect = "B8 FE 01 00 00")]
pub fn task_complex_walk_round_object_get_task_type_6550c0(_this: GPtr<u8>) -> u32 {
    0x1fe
}

/// `CTaskComplexSmartFleePoint::GetTaskType` (0x65BDA0): `mov eax, 0x38e; ret` -> 910.
#[hook(addr = 0x65BDA0, conv = thiscall, name = "CTaskComplexSmartFleePoint::GetTaskType", expect = "B8 8E 03 00 00")]
pub fn task_complex_smart_flee_point_get_task_type_65bda0(_this: GPtr<u8>) -> u32 {
    0x38e
}

/// `CEventLeanOnVehicle::GetEventType` (0x65DB50): `mov eax, 0x5c; ret` -> 92.
#[hook(addr = 0x65DB50, conv = thiscall, name = "CEventLeanOnVehicle::GetEventType", expect = "B8 5C 00 00 00")]
pub fn event_lean_on_vehicle_get_event_type_65db50(_this: GPtr<u8>) -> u32 {
    0x5c
}

/// `CEventLeanOnVehicle::GetEventPriority` (0x65DE10): `mov eax, 0xc; ret` -> 12.
#[hook(addr = 0x65DE10, conv = thiscall, name = "CEventLeanOnVehicle::GetEventPriority", expect = "B8 0C 00 00 00")]
pub fn event_lean_on_vehicle_get_event_priority_65de10(_this: GPtr<u8>) -> u32 {
    0xc
}

/// `CTaskComplexGangFollower::GetTaskType` (0x65EBA0): `mov eax, 0x4b7; ret` -> 1207.
#[hook(addr = 0x65EBA0, conv = thiscall, name = "CTaskComplexGangFollower::GetTaskType", expect = "B8 B7 04 00 00")]
pub fn task_complex_gang_follower_get_task_type_65eba0(_this: GPtr<u8>) -> u32 {
    0x4b7
}

/// `CTaskGangHasslePed::GetTaskType` (0x65FF50): `mov eax, 0x4bc; ret` -> 1212.
#[hook(addr = 0x65FF50, conv = thiscall, name = "CTaskGangHasslePed::GetTaskType", expect = "B8 BC 04 00 00")]
pub fn task_gang_hassle_ped_get_task_type_65ff50(_this: GPtr<u8>) -> u32 {
    0x4bc
}

/// `CTaskGoToVehicleAndLean::GetTaskType` (0x660ED0): `mov eax, 0x4c0; ret` -> 1216.
#[hook(addr = 0x660ED0, conv = thiscall, name = "CTaskGoToVehicleAndLean::GetTaskType", expect = "B8 C0 04 00 00")]
pub fn task_go_to_vehicle_and_lean_get_task_type_660ed0(_this: GPtr<u8>) -> u32 {
    0x4c0
}

/// `CTaskComplexGoToAttractor::ControlSubTask` (0x66B710): `mov eax, dword ptr [ecx + 8]; ret 4`.
#[hook(addr = 0x66B710, conv = thiscall, name = "CTaskComplexGoToAttractor::ControlSubTask", expect = "8B 41 08 C2 04 00")]
pub fn task_complex_go_to_attractor_control_sub_task_66b710(this: GPtr<u8>, _a0: u32) -> u32 {
    rd::<u32>(this, 0x8)
}

/// `CTaskComplexWanderGang::GetWanderType` (0x66F630): `mov eax, 0x4; ret` -> 4.
#[hook(addr = 0x66F630, conv = thiscall, name = "CTaskComplexWanderGang::GetWanderType", expect = "B8 04 00 00 00")]
pub fn task_complex_wander_gang_get_wander_type_66f630(_this: GPtr<u8>) -> u32 {
    0x4
}

/// `CTaskComplexInAirAndLand::GetTaskType` (0x678CB0): `mov eax, 0xf0; ret` -> 240.
#[hook(addr = 0x678CB0, conv = thiscall, name = "CTaskComplexInAirAndLand::GetTaskType", expect = "B8 F0 00 00 00")]
pub fn task_complex_in_air_and_land_get_task_type_678cb0(_this: GPtr<u8>) -> u32 {
    0xf0
}

/// `CTaskSimpleInAir::GetTaskType` (0x678D40): `mov eax, 0xf1; ret` -> 241.
#[hook(addr = 0x678D40, conv = thiscall, name = "CTaskSimpleInAir::GetTaskType", expect = "B8 F1 00 00 00")]
pub fn task_simple_in_air_get_task_type_678d40(_this: GPtr<u8>) -> u32 {
    0xf1
}

/// `CTaskSimpleLand::GetTaskType` (0x678ED0): `mov eax, 0xf2; ret` -> 242.
#[hook(addr = 0x678ED0, conv = thiscall, name = "CTaskSimpleLand::GetTaskType", expect = "B8 F2 00 00 00")]
pub fn task_simple_land_get_task_type_678ed0(_this: GPtr<u8>) -> u32 {
    0xf2
}

/// `CTaskComplexPartnerDeal::GetTaskType` (0x681DF0): `mov eax, 0x4b2; ret` -> 1202.
#[hook(addr = 0x681DF0, conv = thiscall, name = "CTaskComplexPartnerDeal::GetTaskType", expect = "B8 B2 04 00 00")]
pub fn task_complex_partner_deal_get_task_type_681df0(_this: GPtr<u8>) -> u32 {
    0x4b2
}

/// `CTaskComplexPartnerChat::GetTaskType` (0x681E10): `mov eax, 0x4b4; ret` -> 1204.
#[hook(addr = 0x681E10, conv = thiscall, name = "CTaskComplexPartnerChat::GetTaskType", expect = "B8 B4 04 00 00")]
pub fn task_complex_partner_chat_get_task_type_681e10(_this: GPtr<u8>) -> u32 {
    0x4b4
}

/// `CTaskComplexBeInCouple::GetTaskType` (0x683770): `mov eax, 0x4bf; ret` -> 1215.
#[hook(addr = 0x683770, conv = thiscall, name = "CTaskComplexBeInCouple::GetTaskType", expect = "B8 BF 04 00 00")]
pub fn task_complex_be_in_couple_get_task_type_683770(_this: GPtr<u8>) -> u32 {
    0x4bf
}

/// `CTaskSimpleHoldEntity::GetTaskType` (0x691460): `mov eax, 0x133; ret` -> 307.
#[hook(addr = 0x691460, conv = thiscall, name = "CTaskSimpleHoldEntity::GetTaskType", expect = "B8 33 01 00 00")]
pub fn task_simple_hold_entity_get_task_type_691460(_this: GPtr<u8>) -> u32 {
    0x133
}

/// `CEventVehicleDamageCollision::GetEventType` (0x6A0660): `mov eax, 0x49; ret` -> 73.
#[hook(addr = 0x6A0660, conv = thiscall, name = "CEventVehicleDamageCollision::GetEventType", expect = "B8 49 00 00 00")]
pub fn event_vehicle_damage_collision_get_event_type_6a0660(_this: GPtr<u8>) -> u32 {
    0x49
}

/// `__init_g_c18ce4` (0x852680): copies the dword at 0x86F648 to 0xC18CE4 (static initialiser).
#[hook(addr = 0x852680, conv = cdecl, name = "__init_g_c18ce4", expect = "A1 48 F6 86 00")]
pub fn init_g_c18ce4_852680() {
    g::<u32>(0xc18ce4).write(g::<u32>(0x86f648).read());
}

/// `__init_g_c18ce8` (0x852690): copies the dword at 0x86F648 to 0xC18CE8 (static initialiser).
#[hook(addr = 0x852690, conv = cdecl, name = "__init_g_c18ce8", expect = "A1 48 F6 86 00")]
pub fn init_g_c18ce8_852690() {
    g::<u32>(0xc18ce8).write(g::<u32>(0x86f648).read());
}

/// `__init_g_c18cec` (0x8526A0): copies the dword at 0x86F648 to 0xC18CEC (static initialiser).
#[hook(addr = 0x8526A0, conv = cdecl, name = "__init_g_c18cec", expect = "A1 48 F6 86 00")]
pub fn init_g_c18cec_8526a0() {
    g::<u32>(0xc18cec).write(g::<u32>(0x86f648).read());
}

pub const HOOKS: &[HookReg] = &[
    HOOK_EVENT_DO_INFORM_VEHICLE_OCCUPANTS_420EC0,
    HOOK_EVENT_ACQUAINTANCE_PED_HATE_GET_EVENT_PRIORITY_420FD0,
    HOOK_TASK_SIMPLE_SET_PED_POSITION_43E320,
    HOOK_TASK_COMPLEX_WANDER_GET_TASK_TYPE_460CD0,
    HOOK_TASK_SIMPLE_LEAVE_GROUP_GET_TASK_TYPE_4635C0,
    HOOK_TASK_SIMPLE_NONE_MAKE_ABORTABLE_4635D0,
    HOOK_TASK_SIMPLE_NONE_PROCESS_PED_4635E0,
    HOOK_TASK_SIMPLE_CAR_DRIVE_TIMED_GET_TASK_TYPE_46F600,
    HOOK_TASK_SIMPLE_RUN_ANIM_GET_TASK_TYPE_48DEA0,
    HOOK_TASK_SIMPLE_PAUSE_GET_TASK_TYPE_48E800,
    HOOK_EVENT_AFFECTS_PED_GROUP_4ABFF0,
    HOOK_EVENT_PED_COLLISION_WITH_PED_GET_EVENT_TYPE_4ACA40,
    HOOK_EVENT_PED_COLLISION_WITH_PED_GET_EVENT_PRIORITY_4ACA60,
    HOOK_EVENT_OBJECT_COLLISION_GET_EVENT_TYPE_4ACDA0,
    HOOK_EVENT_OBJECT_COLLISION_GET_EVENT_PRIORITY_4ACDC0,
    HOOK_EVENT_BUILDING_COLLISION_GET_EVENT_TYPE_4ACFB0,
    HOOK_EVENT_BUILDING_COLLISION_GET_EVENT_PRIORITY_4ACFE0,
    HOOK_EVENT_GROUP_EVENT_GET_EVENT_TYPE_4AE030,
    HOOK_VT_85AE88_GET_EVENT_TYPE_4AE210,
    HOOK_VT_85AE88_GET_EVENT_PRIORITY_4AE260,
    HOOK_EVENT_POTENTIAL_WALK_INTO_VEHICLE_GET_EVENT_TYPE_4AE390,
    HOOK_EVENT_POTENTIAL_WALK_INTO_VEHICLE_GET_EVENT_PRIORITY_4AE3B0,
    HOOK_EVENT_POTENTIAL_WALK_INTO_PED_GET_EVENT_TYPE_4AE760,
    HOOK_EVENT_POTENTIAL_WALK_INTO_PED_GET_EVENT_PRIORITY_4AE790,
    HOOK_VT_85B070_GET_EVENT_TYPE_4AED30,
    HOOK_VT_85B070_GET_EVENT_PRIORITY_4AED50,
    HOOK_VT_85B0B0_GET_EVENT_TYPE_4AEE60,
    HOOK_VT_85B0B0_GET_EVENT_PRIORITY_4AEE90,
    HOOK_VT_85B0F8_GET_EVENT_TYPE_4AF070,
    HOOK_VT_85B0F8_GET_EVENT_PRIORITY_4AF090,
    HOOK_EVENT_ATTRACTOR_GET_EVENT_TYPE_4AF3D0,
    HOOK_EVENT_IN_AIR_GET_EVENT_TYPE_4B0CD0,
    HOOK_EVENT_IN_AIR_GET_EVENT_PRIORITY_4B0CF0,
    HOOK_EVENT_VEHICLE_DAMAGE_GET_EVENT_PRIORITY_4B1980,
    HOOK_EVENT_SOUND_QUIET_GET_EVENT_TYPE_5DEA60,
    HOOK_EVENT_SOUND_QUIET_GET_EVENT_PRIORITY_5DEAD0,
    HOOK_CPED_STEP_ATTRACTOR_GET_TYPE_5EEC30,
    HOOK_VT_86C6D0_GET_EVENT_TYPE_5F6260,
    HOOK_VT_86C6D0_AFFECTS_PED_5F62F0,
    HOOK_EVENT_PLAYER_COMMAND_TO_GROUP_ATTACK_GET_EVENT_TYPE_5F6360,
    HOOK_VT_86C6D0_GET_EVENT_PRIORITY_5F6410,
    HOOK_EVENT_PED_COLLISION_WITH_PLAYER_GET_EVENT_TYPE_5FED90,
    HOOK_VT_86CA38_GET_EVENT_TYPE_5FF040,
    HOOK_VT_86CA80_GET_EVENT_TYPE_5FF100,
    HOOK_VT_86CAC8_GET_EVENT_TYPE_5FF1C0,
    HOOK_EVENT_SEEN_COP_GET_EVENT_TYPE_5FF3B0,
    HOOK_EVENT_SEEN_COP_GET_EVENT_PRIORITY_5FF880,
    HOOK_VT_86CA38_GET_EVENT_PRIORITY_5FF8B0,
    HOOK_VT_86CA80_GET_EVENT_PRIORITY_5FF8C0,
    HOOK_VT_86CAC8_GET_EVENT_PRIORITY_5FF8D0,
    HOOK_TASK_SIMPLE_RUN_NAMED_ANIM_GET_TASK_TYPE_61AA90,
    HOOK_TASK_SIMPLE_PLAY_HAND_SIGNAL_ANIM_GET_TASK_TYPE_61AEA0,
    HOOK_TASK_COMPLEX_PLAY_HAND_SIGNAL_ANIM_CREATE_NEXT_SUB_TASK_61B570,
    HOOK_TASK_SIMPLE_STAND_STILL_GET_TASK_TYPE_62F350,
    HOOK_TASK_COMPLEX_OBSERVE_TRAFFIC_LIGHTS_GET_TASK_TYPE_6317B0,
    HOOK_TASK_COMPLEX_OBSERVE_TRAFFIC_LIGHTS_MAKE_ABORTABLE_6317D0,
    HOOK_TASK_COMPLEX_OBSERVE_TRAFFIC_LIGHTS_AND_ACHIEVE_HEADING_GET_TASK_TYPE_631940,
    HOOK_TASK_COMPLEX_SEQUENCE_GET_TASK_TYPE_632C60,
    HOOK_TASK_COMPLEX_LEAVE_CAR_GET_TASK_TYPE_63B960,
    HOOK_TASK_COMPLEX_CAR_DRIVE_WANDER_GET_TASK_TYPE_63CB40,
    HOOK_TASK_COMPLEX_ENTER_CAR_AS_DRIVER_GET_TASK_TYPE_640320,
    HOOK_TASK_SIMPLE_CAR_SET_PED_OUT_GET_TASK_TYPE_647930,
    HOOK_TASK_SIMPLE_EVASIVE_STEP_GET_TASK_TYPE_6531C0,
    HOOK_TASK_COMPLEX_EVASIVE_STEP_GET_TASK_TYPE_653340,
    HOOK_TASK_COMPLEX_EVASIVE_STEP_CONTROL_SUB_TASK_653550,
    HOOK_TASK_COMPLEX_WALK_ROUND_OBJECT_GET_TASK_TYPE_6550C0,
    HOOK_TASK_COMPLEX_SMART_FLEE_POINT_GET_TASK_TYPE_65BDA0,
    HOOK_EVENT_LEAN_ON_VEHICLE_GET_EVENT_TYPE_65DB50,
    HOOK_EVENT_LEAN_ON_VEHICLE_GET_EVENT_PRIORITY_65DE10,
    HOOK_TASK_COMPLEX_GANG_FOLLOWER_GET_TASK_TYPE_65EBA0,
    HOOK_TASK_GANG_HASSLE_PED_GET_TASK_TYPE_65FF50,
    HOOK_TASK_GO_TO_VEHICLE_AND_LEAN_GET_TASK_TYPE_660ED0,
    HOOK_TASK_COMPLEX_GO_TO_ATTRACTOR_CONTROL_SUB_TASK_66B710,
    HOOK_TASK_COMPLEX_WANDER_GANG_GET_WANDER_TYPE_66F630,
    HOOK_TASK_COMPLEX_IN_AIR_AND_LAND_GET_TASK_TYPE_678CB0,
    HOOK_TASK_SIMPLE_IN_AIR_GET_TASK_TYPE_678D40,
    HOOK_TASK_SIMPLE_LAND_GET_TASK_TYPE_678ED0,
    HOOK_TASK_COMPLEX_PARTNER_DEAL_GET_TASK_TYPE_681DF0,
    HOOK_TASK_COMPLEX_PARTNER_CHAT_GET_TASK_TYPE_681E10,
    HOOK_TASK_COMPLEX_BE_IN_COUPLE_GET_TASK_TYPE_683770,
    HOOK_TASK_SIMPLE_HOLD_ENTITY_GET_TASK_TYPE_691460,
    HOOK_EVENT_VEHICLE_DAMAGE_COLLISION_GET_EVENT_TYPE_6A0660,
    HOOK_INIT_G_C18CE4_852680,
    HOOK_INIT_G_C18CE8_852690,
    HOOK_INIT_G_C18CEC_8526A0,
];
