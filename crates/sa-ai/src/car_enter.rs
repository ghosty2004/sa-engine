//! PORT-AI-3 (`docs/arch/ai-port.md`, section PORT-AI-3): vehicle entry: `CTaskComplexEnterCar*`, `CTaskSimpleCarAlign`/`OpenDoorFromOutside`/`GetIn`/`CloseDoor*`,
//! `CTaskSimpleCarSetPedInAsDriver`/`Passenger`, `CTaskComplexGoToCarDoorAndStandStill`, `CTaskUtilityLineUpPedWithCar`.
//! Ported from the 1.0 US bytes (the EXE wins over gta-reversed and the specs). Every call to another
//! catalogued function goes through `orig!` at the original address; task objects keep their original
//! vtables and come from the original task pool (`common::task_new`).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    // ---- CVehicle virtual slots ----
    /// `CVehicle::OpenDoor(CPed*, int32 component, eDoors door, float ratio, bool playSound)` (vtable +0x6C)
    fn v_veh_open_door(v: GPtr<u8>, ped: GPtr<u8>, component: u32, door: u32, ratio: f32, sound: u32) = thiscall vslot 0x6C => unported;
    /// `CVehicle::ProcessOpenDoor(CPed*, uint32 door, uint32 group, uint32 anim, float time)` (vtable +0x70)
    fn v_veh_process_open_door(v: GPtr<u8>, ped: GPtr<u8>, door: u32, group: u32, anim: u32, t: f32) = thiscall vslot 0x70 => unported;
    /// `CVehicle::GetDooorAngleOpenRatio(uint32 door)` (vtable +0x74, result in ST0)
    fn v_veh_get_door_angle_open_ratio(v: GPtr<u8>, door: u32) -> f64 = thiscall vslot 0x74 => unported;
    /// `CVehicle` vtable +0x8C (bool, door argument)
    fn v_veh_slot_8c(v: GPtr<u8>, door: u32) -> u8 = thiscall vslot 0x8C => unported;
    /// `CVehicle` vtable +0x94 (bool, door argument)
    fn v_veh_slot_94(v: GPtr<u8>, door: u32) -> u8 = thiscall vslot 0x94 => unported;

    // ---- other catalogued functions ----
    /// `CCarEnterExit::GetPositionToOpenCarDoor(CVector* out, CVehicle*, int32 door)` (0x64E740, cdecl)
    fn car_enter_exit_get_position_to_open_car_door(out: GPtr<u8>, veh: GPtr<u8>, door: u32) -> GPtr<u8> = cdecl 0x64E740 => unported;
    /// `CTaskUtilityLineUpPedWithCar::ProcessPed(CPed*, CVehicle*, CAnimBlendAssociation*)` (0x6513A0, `ret 0xC`)
    fn line_up_process_ped(u: GPtr<u8>, ped: GPtr<u8>, veh: GPtr<u8>, a: GPtr<u8>) = thiscall 0x6513A0 => unported;
    /// `CAnimManager::BlendAnimation-id(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimManager::AddAnimation-id(RpClump*, AssocGroupId, AnimationId)` (0x4D3AA0)
    fn anim_add_animation(clump: u32, group: u32, id: u32) -> GPtr<u8> = cdecl 0x4D3AA0 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0)
    fn anim_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CVehicleAnimGroup::GetGroup(int32 animId)` (0x6E3B00, `ret 4`)
    fn vehicle_anim_group_get_group(g: GPtr<u8>, anim: u32) -> u32 = thiscall 0x6E3B00 => unported;
    /// `CPedPlacement::FindZCoorForPed(CVector*)` (0x616920, cdecl)
    fn ped_placement_find_z_coor_for_ped(v: GPtr<u8>) -> u8 = cdecl 0x616920 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70, cdecl)
    fn pad_get_pad(i: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetTarget()` (0x540670)
    fn pad_get_target(pad: GPtr<u8>) -> u8 = thiscall 0x540670 => unported;
    /// `CDamageManager::GetEngineStatus()` (0x6C22C0)
    fn damage_get_engine_status(d: GPtr<u8>) -> u32 = thiscall 0x6C22C0 => unported;
    /// `CDamageManager::GetDoorStatus_Component(int32 door)` (0x6C2250, `ret 4`)
    fn damage_get_door_status_component(d: GPtr<u8>, door: u32) -> u32 = thiscall 0x6C2250 => unported;
    /// `CDamageManager::SetDoorStatus_Component(int32 door, uint32 status)` (0x6C21E0, `ret 8`)
    fn damage_set_door_status_component(d: GPtr<u8>, door: u32, status: u32) = thiscall 0x6C21E0 => unported;
    /// `CPed::RemoveWeaponWhenEnteringVehicle(int32)` (0x5E6370, `ret 4`)
    fn ped_remove_weapon_when_entering_vehicle(ped: GPtr<u8>, a: i32) = thiscall 0x5E6370 => unported;
    /// `operator*(CVector* out, float, const CVector*)` (0x40FE90, cdecl, returns out)
    fn vector_scale(out: GPtr<u8>, k: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `CPhysical::ApplyMoveForce-vec(CVector)` (0x5429F0, vector by value, `ret 0xC`)
    fn physical_apply_move_force(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x5429F0 => unported;
    /// `CPlaceable::GetTopDirection(CVector* out)` (0x41CCB0, returns out)
    fn placeable_get_top_direction(p: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x41CCB0 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(x1, y1, x2, y2)` (0x53CBE0, cdecl, result in ST0)
    fn general_get_radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, cdecl, result in ST0)
    fn general_limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CCarEnterExit::ComputeDoorFlag(CVehicle*, int32 door, bool)` (0x64E550, cdecl)
    fn car_enter_exit_compute_door_flag(veh: GPtr<u8>, door: u32, b: u32) -> u32 = cdecl 0x64E550 => unported;
    /// `CVehicle::SetGettingInFlags(uint8)` (0x6D3000, reads the low byte of its argument)
    fn vehicle_set_getting_in_flags(v: GPtr<u8>, f: u32) = thiscall 0x6D3000 => unported;
    /// `CCarEnterExit::CarHasDoorToOpen(CVehicle*, int32 door)` (0x64EDD0, cdecl)
    fn car_enter_exit_car_has_door_to_open(veh: GPtr<u8>, door: u32) -> u8 = cdecl 0x64EDD0 => unported;
    /// `CCarEnterExit::CarHasOpenableDoor(CVehicle*, int32 door, CPed*)` (0x64EE50, cdecl)
    fn car_enter_exit_car_has_openable_door(veh: GPtr<u8>, door: u32, ped: GPtr<u8>) -> u8 = cdecl 0x64EE50 => unported;
    /// `CPedIntelligence::GetTaskDuck(bool)` (0x6010A0)
    fn intelligence_get_task_duck(i: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6010A0 => unported;
    /// CRT `operator new` (0x82119A)
    fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `CTaskUtilityLineUpPedWithCar::Constructor(const CVector& offset, int32 time, int32, int32 door)` (0x64FBB0, `ret 0x10`)
    fn line_up_ped_with_car_ctor(t: GPtr<u8>, offset: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x64FBB0 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPed` vtable +0x5C (no arguments, result unused)
    fn v_ped_slot_5c(ped: GPtr<u8>) = thiscall vslot 0x5C => unported;
    /// `CEntity::CleanUpOldReference(CEntity** ref)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity** ref)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, t: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0, `ret 0xC`)
    fn tm_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: i32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CCarEnterExit::ComputeTargetDoorToExit(CVehicle*, CPed*)` (0x64F110, cdecl)
    fn car_enter_exit_compute_target_door_to_exit(veh: GPtr<u8>, ped: GPtr<u8>) -> u32 = cdecl 0x64F110 => unported;
    /// `CCarEnterExit::ComputePassengerIndexFromCarDoor(CVehicle*, int32 door)` (0x64F1E0, cdecl)
    fn car_enter_exit_compute_passenger_index_from_car_door(veh: GPtr<u8>, door: u32) -> i32 = cdecl 0x64F1E0 => unported;
    /// `CCarEnterExit::RemoveGetInAnims(CPed*)` (0x64F6E0, cdecl)
    fn car_enter_exit_remove_get_in_anims(ped: GPtr<u8>) = cdecl 0x64F6E0 => unported;
    /// `CCarEnterExit::AddInCarAnim(CVehicle*, CPed*, bool asDriver)` (0x64F720, cdecl)
    fn car_enter_exit_add_in_car_anim(veh: GPtr<u8>, ped: GPtr<u8>, driver: u32) = cdecl 0x64F720 => unported;
    /// `CTaskSimpleCarSetPedOut` constructor (0x6478B0, `ret 0xC`)
    fn car_set_ped_out_ctor(t: GPtr<u8>, veh: GPtr<u8>, door: u32, switch_off_engine: u32) -> GPtr<u8> = thiscall 0x6478B0 => unported;
    /// `CTaskSimpleCarSetPedOut::ProcessPed(CPed*)` (0x647D10)
    fn car_set_ped_out_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x647D10 => unported;
    /// `CTaskSimpleCarSetPedOut` complete destructor (0x647950)
    fn car_set_ped_out_dtor(t: GPtr<u8>) = thiscall 0x647950 => unported;
    /// `CTaskSimpleCarDrive::Constructor(veh, utility, a2)` (0x63C340, `ret 0xC`)
    fn car_drive_simple_ctor(t: GPtr<u8>, veh: GPtr<u8>, a1: u32, a2: u32) -> GPtr<u8> = thiscall 0x63C340 => unported;
    /// `CPedGroups::GetPedsGroup(CPed*)` (0x5F7E80, cdecl)
    fn ped_groups_get_peds_group(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::IsLeader(CPed*)` (0x5F69C0)
    fn group_membership_is_leader(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5F69C0 => unported;
    /// `CEventLeaderEnteredCarAsDriver::Constructor(CVehicle*)` (0x48E1C0)
    fn event_leader_entered_car_as_driver_ctor(e: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x48E1C0 => unported;
    /// `CEventGroupEvent::CEventGroupEvent(CPed*, CEvent*)` (0x4ADFD0)
    fn event_group_event_ctor(e: GPtr<u8>, ped: GPtr<u8>, ev: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ADFD0 => unported;
    /// `CEventGroupEvent::~CEventGroupEvent` (0x4AE070)
    fn event_group_event_dtor(e: GPtr<u8>) = thiscall 0x4AE070 => unported;
    /// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470)
    fn group_intelligence_add_event(gi: GPtr<u8>, e: GPtr<u8>) = thiscall 0x5F7470 => unported;
    /// `CVehicle::SetDriver(CPed*)` (0x6D16A0)
    fn vehicle_set_driver(v: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6D16A0 => unported;
    /// `CVehicle::AddPassenger-Fixed-Seat(CPed*, uint8 seat)` (0x6D14D0)
    fn vehicle_add_passenger_fixed(v: GPtr<u8>, ped: GPtr<u8>, seat: i32) -> u8 = thiscall 0x6D14D0 => unported;
    /// `CVehicle::AddPassenger-Auto-Seat(CPed*)` (0x6D13A0)
    fn vehicle_add_passenger_auto(v: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6D13A0 => unported;
    /// `CVehicle::ClearGettingInFlags(uint8)` (0x6D3040)
    fn vehicle_clear_getting_in_flags(v: GPtr<u8>, f: u32) = thiscall 0x6D3040 => unported;
    /// `CVehicle::IsLawEnforcementVehicle()` (0x6D2370)
    fn vehicle_is_law_enforcement(v: GPtr<u8>) -> u8 = thiscall 0x6D2370 => unported;
    /// `CPlayerPed::ClearAdrenaline()` (0x60A070)
    fn player_ped_clear_adrenaline(p: GPtr<u8>) = thiscall 0x60A070 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(i: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CCrime::ReportCrime(eCrimeType, CEntity*, CPed*)` (0x532010, cdecl)
    fn crime_report_crime(ty: i32, victim: GPtr<u8>, ped: GPtr<u8>) = cdecl 0x532010 => unported;
    /// `CPed::UpdateStatEnteringVehicle()` (0x5E01A0)
    fn ped_update_stat_entering_vehicle(ped: GPtr<u8>) = thiscall 0x5E01A0 => unported;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00)
    fn ped_set_move_state(ped: GPtr<u8>, s: i32) = thiscall 0x5DEC00 => unported;
    /// `CPed::SetPedState(ePedState)` (0x5E4500)
    fn ped_set_ped_state(ped: GPtr<u8>, s: i32) = thiscall 0x5E4500 => unported;
    /// `CEventCopCarBeingStolen::Constructor(CPed*, CVehicle*)` (0x4B1740)
    fn event_cop_car_being_stolen_ctor(e: GPtr<u8>, ped: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B1740 => unported;
    /// `CEventCopCarBeingStolen` complete destructor (0x4B17F0)
    fn event_cop_car_being_stolen_dtor(e: GPtr<u8>) = thiscall 0x4B17F0 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50, cdecl)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;

    // ---- this family (called through the original entries) ----
    /// `CTaskSimpleCarAlign::FixHeading(CPed*)` (0x645CB0)
    fn car_align_fix_heading_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x645CB0 => unported;
    /// `CTaskSimpleCarAlign::StartAnim(CPed*)` (0x64AC00)
    fn car_align_start_anim_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x64AC00 => unported;
    /// `CTaskSimpleCarOpenDoorFromOutside::ComputeAnimID_ToHook(int32* group, int32* anim)` (0x645FA0)
    fn open_door_compute_anim_id_orig(t: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) = thiscall 0x645FA0 => unported;
    /// `CTaskSimpleCarOpenDoorFromOutside::StartAnim(CPed*)` (0x64AD90)
    fn open_door_start_anim_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x64AD90 => unported;
    /// `CTaskSimpleCarCloseDoorFromInside::ComputeAnimID(int32* group, int32* anim)` (0x646410)
    fn close_inside_compute_anim_id_orig(t: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) = thiscall 0x646410 => unported;
    /// `CTaskSimpleCarCloseDoorFromInside::StartAnim(CPed*)` (0x64AFB0)
    fn close_inside_start_anim_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x64AFB0 => unported;
    /// `CTaskSimpleCarCloseDoorFromOutside::ComputeAnimID(int32* group, int32* anim)` (0x646600)
    fn close_outside_compute_anim_id_orig(t: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) = thiscall 0x646600 => unported;
    /// `CTaskSimpleCarCloseDoorFromOutside::StartAnim(CPed*)` (0x64B080)
    fn close_outside_start_anim_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x64B080 => unported;
    /// `CTaskSimpleCarGetIn::StartAnim(CPed*)` (0x64B2D0)
    fn get_in_start_anim_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x64B2D0 => unported;
}

/// The vehicle's anim group record: `0xC1CDC0 + 0x94 * (byte +0xDE of the model info +0x384)`.
fn veh_anim_group(veh: GPtr<u8>) -> GPtr<u8> {
    let grp = u32::from(rd::<u8>(rd::<GPtr<u8>>(veh, 0x384), 0xDE));
    GPtr::from_addr((grp.wrapping_mul(0x94)).wrapping_add(0xC1CDC0) as usize)
}

/// A pointer to a Rust local standing in for a slot of the original's stack frame.
fn slot<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

/// The shared `ComputeAnimID` body: `*anim` by door (+0x14) 8/9/10/11 (other doors leave `*anim`),
/// `*group = vehicle anim group.GetGroup(*anim)`.
fn compute_anim_id_by_door(this: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>, ids: [u32; 4]) {
    match rd::<i32>(this, 0x14).wrapping_sub(8) {
        0 => wr::<u32>(anim, 0, ids[0]),
        1 => wr::<u32>(anim, 0, ids[1]),
        2 => wr::<u32>(anim, 0, ids[2]),
        3 => wr::<u32>(anim, 0, ids[3]),
        _ => {}
    }
    let a = rd::<u32>(anim, 0);
    let r = vehicle_anim_group_get_group(veh_anim_group(rd::<GPtr<u8>>(this, 0x10)), a);
    wr::<u32>(group, 0, r);
}

// ---------------------------------------------------------------------------------------------
// SetPedPosition / finish-anim callbacks / ComputeAnimID
// ---------------------------------------------------------------------------------------------

/// `CTaskSimpleCarAlign::SetPedPosition(CPed*)` (0x645C70): `GetPositionToOpenCarDoor(local, veh +0x10,
/// door +0x20)` (the result is never read), then the line-up utility (+0x24) `ProcessPed(ped, veh +0x10,
/// anim +0xC)`; true.
#[hook(addr = 0x645C70, conv = thiscall, name = "CTaskSimpleCarAlign::SetPedPosition", expect = "83 EC 0C 56 8B F1")]
pub fn car_align_set_ped_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let mut v = [0u32; 3];
    car_enter_exit_get_position_to_open_car_door(
        slot(&mut v),
        rd::<GPtr<u8>>(this, 0x10),
        rd::<u32>(this, 0x20),
    );
    line_up_process_ped(
        rd::<GPtr<u8>>(this, 0x24),
        ped,
        rd::<GPtr<u8>>(this, 0x10),
        rd::<GPtr<u8>>(this, 0xC),
    );
    1
}

/// `CTaskSimpleCarAlign::FinishAnimCarAlignCB(CAnimBlendAssociation*, void* task)` (0x645E40, cdecl):
/// task anim (+0xC) = null, finished (+8) = 1.
#[hook(addr = 0x645E40, conv = cdecl, name = "CTaskSimpleCarAlign::FinishAnimCarAlignCB", expect = "8B 44 24 08 C7 40 0C 00 00 00 00")]
pub fn car_align_finish_anim_cb(_anim: GPtr<u8>, task: GPtr<u8>) {
    wr::<u32>(task, 0xC, 0);
    wr::<u8>(task, 8, 1);
}

/// `CTaskSimpleCarOpenDoorFromOutside::SetPedPosition(CPed*)` (0x645F80): line-up utility (+0x1C)
/// `ProcessPed(ped, veh +0x10, anim +0xC)`; true.
#[hook(addr = 0x645F80, conv = thiscall, name = "CTaskSimpleCarOpenDoorFromOutside::SetPedPosition", expect = "8B 41 0C 8B 51 10")]
pub fn open_door_set_ped_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    line_up_process_ped(
        rd::<GPtr<u8>>(this, 0x1C),
        ped,
        rd::<GPtr<u8>>(this, 0x10),
        rd::<GPtr<u8>>(this, 0xC),
    );
    1
}

/// `CTaskSimpleCarOpenDoorFromOutside::ComputeAnimID_ToHook(int32* group, int32* anim)` (0x645FA0): door
/// (+0x14) 8/9/10/11 -> anim 0x164/0x166/0x163/0x165 (jump table 0x646004; other doors leave `*anim`),
/// `*group = vehicle anim group.GetGroup(*anim)`.
#[hook(addr = 0x645FA0, conv = thiscall, name = "CTaskSimpleCarOpenDoorFromOutside::ComputeAnimID_ToHook", expect = "8B 41 14 8B 54 24 08")]
pub fn open_door_compute_anim_id(this: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) {
    compute_anim_id_by_door(this, group, anim, [0x164, 0x166, 0x163, 0x165]);
}

/// `CTaskSimpleCarOpenDoorFromOutside::FinishAnimCarOpenDoorFromOutsideCB(assoc, void* task)` (0x646020,
/// cdecl): anim (+0xC) = null, finished (+8) = 1, `ComputeAnimID(&group, &anim)` and the vehicle's
/// `ProcessOpenDoor(null, door +0x14, group, anim, 1.0)` (vtable +0x70). The anim out-slot is the
/// callback's own `task` argument slot: for doors outside 8..11 the anim id is the task pointer.
#[hook(addr = 0x646020, conv = cdecl, name = "CTaskSimpleCarOpenDoorFromOutside::FinishAnimCarOpenDoorFromOutsideCB", expect = "51 56 8B 74 24 10")]
pub fn open_door_finish_anim_cb(_anim: GPtr<u8>, task: GPtr<u8>) {
    let mut group = 0u32;
    let mut anim = task.addr() as u32;
    wr::<u32>(task, 0xC, 0);
    wr::<u8>(task, 8, 1);
    open_door_compute_anim_id_orig(task, slot(&mut group), slot(&mut anim));
    v_veh_process_open_door(
        rd::<GPtr<u8>>(task, 0x10),
        GPtr::NULL,
        rd::<u32>(task, 0x14),
        group,
        anim,
        1.0,
    );
}

/// `CTaskSimpleCarCloseDoorFromInside::SetPedPosition(CPed*)` (0x6463F0): line-up utility (+0x18)
/// `ProcessPed(ped, veh +0x10, anim +0xC)`; true.
#[hook(addr = 0x6463F0, conv = thiscall, name = "CTaskSimpleCarCloseDoorFromInside::SetPedPosition", expect = "8B 41 0C 8B 51 10")]
pub fn close_inside_set_ped_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    line_up_process_ped(
        rd::<GPtr<u8>>(this, 0x18),
        ped,
        rd::<GPtr<u8>>(this, 0x10),
        rd::<GPtr<u8>>(this, 0xC),
    );
    1
}

/// `CTaskSimpleCarCloseDoorFromInside::ComputeAnimID(int32* group, int32* anim)` (0x646410): door (+0x14)
/// 8/9/10/11 -> anim 0x170/0x172/0x16F/0x171 (jump table 0x646474; other doors leave `*anim`),
/// `*group = vehicle anim group.GetGroup(*anim)`.
#[hook(addr = 0x646410, conv = thiscall, name = "CTaskSimpleCarCloseDoorFromInside::ComputeAnimID", expect = "8B 41 14 8B 54 24 08")]
pub fn close_inside_compute_anim_id(this: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) {
    compute_anim_id_by_door(this, group, anim, [0x170, 0x172, 0x16F, 0x171]);
}

/// `CTaskSimpleCarCloseDoorFromInside::FinishAnimCarCloseDoorFromInsideCB(assoc, void* task)` (0x646490,
/// cdecl): finished (+8) = 1; with a vehicle (+0x10): `ComputeAnimID(&group, &anim)` (anim out-slot = the
/// `task` argument slot, so the task pointer for doors outside 8..11) and `ProcessOpenDoor(null, door
/// +0x14, group, anim, 1.0)`. Then the anim (+0xC), if any, gets blend delta -1000.0; +0xC = null.
#[hook(addr = 0x646490, conv = cdecl, name = "CTaskSimpleCarCloseDoorFromInside::FinishAnimCarCloseDoorFromInsideCB", expect = "51 56 8B 74 24 10")]
pub fn close_inside_finish_anim_cb(_anim: GPtr<u8>, task: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(task, 0x10);
    wr::<u8>(task, 8, 1);
    if !veh.is_null() {
        let mut group = 0u32;
        let mut anim = task.addr() as u32;
        close_inside_compute_anim_id_orig(task, slot(&mut group), slot(&mut anim));
        v_veh_process_open_door(
            rd::<GPtr<u8>>(task, 0x10),
            GPtr::NULL,
            rd::<u32>(task, 0x14),
            group,
            anim,
            1.0,
        );
    }
    let a = rd::<GPtr<u8>>(task, 0xC);
    if !a.is_null() {
        wr::<u32>(a, 0x1C, 0xC47A_0000);
    }
    wr::<u32>(task, 0xC, 0);
}

/// `CTaskSimpleCarCloseDoorFromOutside::SetPedPosition(CPed*)` (0x6465E0): line-up utility (+0x18)
/// `ProcessPed(ped, veh +0x10, anim +0xC)`; true.
#[hook(addr = 0x6465E0, conv = thiscall, name = "CTaskSimpleCarCloseDoorFromOutside::SetPedPosition", expect = "8B 41 0C 8B 51 10")]
pub fn close_outside_set_ped_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    line_up_process_ped(
        rd::<GPtr<u8>>(this, 0x18),
        ped,
        rd::<GPtr<u8>>(this, 0x10),
        rd::<GPtr<u8>>(this, 0xC),
    );
    1
}

/// `CTaskSimpleCarCloseDoorFromOutside::FinishAnimCarCloseDoorFromOutsideCB(assoc, void* task)` (0x646680,
/// cdecl): anim (+0xC) = null, finished (+8) = 1.
#[hook(addr = 0x646680, conv = cdecl, name = "CTaskSimpleCarCloseDoorFromOutside::FinishAnimCarCloseDoorFromOutsideCB", expect = "8B 44 24 08 C7 40 0C 00 00 00 00")]
pub fn close_outside_finish_anim_cb(_anim: GPtr<u8>, task: GPtr<u8>) {
    wr::<u32>(task, 0xC, 0);
    wr::<u8>(task, 8, 1);
}

/// `CTaskSimpleCarGetIn::SetPedPosition(CPed*)` (0x6467A0): line-up utility (+0x20) `ProcessPed(ped,
/// veh +0x18, anim +0xC)`; true.
#[hook(addr = 0x6467A0, conv = thiscall, name = "CTaskSimpleCarGetIn::SetPedPosition", expect = "8B 41 0C 8B 51 18")]
pub fn get_in_set_ped_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    line_up_process_ped(
        rd::<GPtr<u8>>(this, 0x20),
        ped,
        rd::<GPtr<u8>>(this, 0x18),
        rd::<GPtr<u8>>(this, 0xC),
    );
    1
}

/// `CTaskSimpleCarGetIn::FinishAnimCarGetInCB(assoc, void* task)` (0x6467F0, cdecl): finished (+8) = 1;
/// the anim (+0xC), if any, gets blend delta -1000.0; +0xC = null.
#[hook(addr = 0x6467F0, conv = cdecl, name = "CTaskSimpleCarGetIn::FinishAnimCarGetInCB", expect = "8B 44 24 08 8B 48 0C")]
pub fn get_in_finish_anim_cb(_anim: GPtr<u8>, task: GPtr<u8>) {
    let a = rd::<GPtr<u8>>(task, 0xC);
    wr::<u8>(task, 8, 1);
    if !a.is_null() {
        wr::<u32>(a, 0x1C, 0xC47A_0000);
    }
    wr::<u32>(task, 0xC, 0);
}

/// `CTaskSimpleCarAlign::FixHeading(CPed*)` (0x645CB0). Automobiles only (vehicle +0x10, +0x590 == 0):
/// `v` = the vehicle matrix right row (no null check), times [0x858C1C] (-1.0) for doors (+0x20) 10/11.
/// With `d = (v.z*t.z + v.y*t.y) + v.x*t.x` against the ped's top direction `t`, only 0 < d < [0x858B1C]
/// (0.1) goes on (`GetTopDirection` is called twice there, the first result unused): with the new `d`,
/// `w = (v.x*d, v.y*d, v.z)` (z is not scaled in the original), a third `GetTopDirection`, and the
/// heading `LimitRadianAngle(GetRadianAngleBetweenPoints(t.x - w.x - w.x, t.y - w.y - w.y, 0, 0))` (the
/// angle rounded to f32 on the way) goes to ped +0x55C and +0x558.
#[hook(addr = 0x645CB0, conv = thiscall, name = "CTaskSimpleCarAlign::FixHeading", expect = "8B 41 10 83 EC 18")]
pub fn car_align_fix_heading(this: GPtr<u8>, ped: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(this, 0x10);
    if veh.is_null() || rd::<u32>(veh, 0x590) != 0 {
        return;
    }
    let door = rd::<i32>(this, 0x20);
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let mut v = [rd::<f32>(m, 0), rd::<f32>(m, 4), rd::<f32>(m, 8)];
    if door == 0xA || door == 0xB {
        let k = g::<f32>(0x858C1C).read();
        for c in v.iter_mut() {
            *c = f32_of(x(*c).fmul_m(k));
        }
    }
    let mut tl = [0f32; 3];
    let dot = |v: &[f32; 3], t: GPtr<u8>| {
        x(v[2])
            .fmul_m(rd::<f32>(t, 8))
            .fadd(x(v[1]).fmul_m(rd::<f32>(t, 4)))
            .fadd(x(v[0]).fmul_m(rd::<f32>(t, 0)))
    };
    let t = placeable_get_top_direction(ped, slot(&mut tl));
    let d = dot(&v, t);
    if d.fcom(x(g::<f32>(0x858B50).read())).test_any(0x41) {
        return;
    }
    if d.fcom(x(g::<f32>(0x858B1C).read())).test_parity(5) {
        return;
    }
    placeable_get_top_direction(ped, slot(&mut tl));
    let t = placeable_get_top_direction(ped, slot(&mut tl));
    let d = dot(&v, t);
    let w = [f32_of(x(v[0]).fmul(d)), f32_of(x(v[1]).fmul(d)), v[2]];
    let t = placeable_get_top_direction(ped, slot(&mut v));
    let a = f32_of(x(rd::<f32>(t, 0)).fsub_m(w[0]).fsub_m(w[0]));
    let b = f32_of(x(rd::<f32>(t, 4)).fsub_m(w[1]).fsub_m(w[1]));
    let ang = f32_of(X87::from_f64_exact(general_get_radian_angle_between_points(
        a, b, 0.0, 0.0,
    )));
    let h = f32_of(X87::from_f64_exact(general_limit_radian_angle(ang)));
    wr::<f32>(ped, 0x55C, h);
    wr::<f32>(ped, 0x558, h);
}

// ---------------------------------------------------------------------------------------------
// StartAnim
// ---------------------------------------------------------------------------------------------

/// `CTaskSimpleCarAlign::StartAnim(CPed*)` (0x64AC00): `dz = door position z (GetPositionToOpenCarDoor(veh
/// +0x10, door +0x20)) - ped z` (matrix +0x38, else +0xC), clamped below at [0x858B50] (0.0; a NaN is kept).
/// Doors 10/11: dz > [0x86EC44] (4.4) -> anim 0x161, else 0x15F; other doors: > 4.4 -> 0x162, else 0x160.
/// `BlendAnimation(clump, group of anim, anim, 4.0)` into +0xC, finish callback 0x645E40.
#[hook(addr = 0x64AC00, conv = thiscall, name = "CTaskSimpleCarAlign::StartAnim", expect = "83 EC 0C 53 56")]
pub fn car_align_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let mut v = [0f32; 3];
    car_enter_exit_get_position_to_open_car_door(
        slot(&mut v),
        rd::<GPtr<u8>>(this, 0x10),
        rd::<u32>(this, 0x20),
    );
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pos = if m.is_null() {
        ped.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    let zero = g::<f32>(0x858B50).read();
    let mut dz = x(v[2]).fsub_m(rd::<f32>(pos, 8));
    if !dz.fcom(x(zero)).test_parity(5) {
        dz = x(zero);
    }
    let high = !dz.fcom(x(g::<f32>(0x86EC44).read())).test_any(0x41);
    let door = rd::<i32>(this, 0x20);
    let anim = if door == 0xA || door == 0xB {
        if high { 0x161 } else { 0x15F }
    } else if high {
        0x162
    } else {
        0x160
    };
    let group = vehicle_anim_group_get_group(veh_anim_group(rd::<GPtr<u8>>(this, 0x10)), anim);
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), group, anim, 0x4080_0000);
    wr::<GPtr<u8>>(this, 0xC, a);
    anim_set_finish_callback(a, 0x645E40, this.addr() as u32);
}

/// `CTaskSimpleCarOpenDoorFromOutside::StartAnim(CPed*)` (0x64AD90): `ComputeAnimID(&group, &anim)`,
/// `BlendAnimation(clump, group, anim, 4.0)` into +0xC, finish callback 0x646020. The anim local is
/// uninitialised in the original for doors outside 8..11 (zeroed here).
#[hook(addr = 0x64AD90, conv = thiscall, name = "CTaskSimpleCarOpenDoorFromOutside::StartAnim", expect = "83 EC 08 56 8B F1")]
pub fn open_door_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let mut anim = 0u32;
    let mut group = 0u32;
    open_door_compute_anim_id_orig(this, slot(&mut group), slot(&mut anim));
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), group, anim, 0x4080_0000);
    wr::<GPtr<u8>>(this, 0xC, a);
    anim_set_finish_callback(a, 0x646020, this.addr() as u32);
}

/// `CTaskSimpleCarCloseDoorFromInside::StartAnim(CPed*)` (0x64AFB0): `ComputeAnimID(&group, &anim)`,
/// `BlendAnimation(clump, group, anim, 1000.0)` into +0xC, finish callback 0x646490. The anim local is
/// uninitialised in the original for doors outside 8..11 (zeroed here).
#[hook(addr = 0x64AFB0, conv = thiscall, name = "CTaskSimpleCarCloseDoorFromInside::StartAnim", expect = "83 EC 08 56 8B F1")]
pub fn close_inside_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let mut anim = 0u32;
    let mut group = 0u32;
    close_inside_compute_anim_id_orig(this, slot(&mut group), slot(&mut anim));
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), group, anim, 0x447A_0000);
    wr::<GPtr<u8>>(this, 0xC, a);
    anim_set_finish_callback(a, 0x646490, this.addr() as u32);
}

/// `CTaskSimpleCarCloseDoorFromOutside::StartAnim(CPed*)` (0x64B080). The ped position (matrix +0x30, else
/// +4) copied to a local goes through `FindZCoorForPed`; when `ped z - [0x858624]` (1.0) is above the found
/// z the task just finishes (+8 = 1). Otherwise the vehicle (+0x10) gets +0xB8 = 0 and its move speed
/// (+0x44..) and turn speed (+0x50..) each += [0x858CDC] (0.001). A player whose pad (`GetPad(0/1)` by
/// +0x598; for other values the pad pointer is the ped itself: the original reuses its argument slot) has
/// no input (+0x10E word clear and vehicle +0x4F8 != 6 and no `GetTarget` and words +0/+2/+0x10..+0x16
/// clear, or the +0x10E/+0x4F8 test fails), with no vehicle +0x490 and no damaged engine (automobile,
/// `GetEngineStatus` >= 0xE1), gets the anim; all other players finish at once, an automobile first
/// stepping the door (+0x14) status to status + 1 when it is 0 or 2. The anim: `ComputeAnimID(&group,
/// &anim)` (anim out-slot = the ped argument slot), `AddAnimation(clump, group, anim)` into +0xC, finish
/// callback 0x646680. Non-players always get the anim.
#[hook(addr = 0x64B080, conv = thiscall, name = "CTaskSimpleCarCloseDoorFromOutside::StartAnim", expect = "83 EC 10 55 8B 6C 24 18")]
pub fn close_outside_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let ped_pos = || {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        if m.is_null() {
            ped.byte_add(4)
        } else {
            m.byte_add(0x30)
        }
    };
    let p = ped_pos();
    let mut v = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
    ped_placement_find_z_coor_for_ped(slot(&mut v));
    let p = ped_pos();
    let d = x(rd::<f32>(p, 8)).fsub_m(g::<f32>(0x858624).read());
    if !d.fcom(x(f32::from_bits(v[2]))).test_any(0x41) {
        wr::<u8>(this, 8, 1);
        return;
    }
    let veh = rd::<GPtr<u8>>(this, 0x10);
    wr::<u8>(veh, 0xB8, 0);
    let k = g::<f32>(0x858CDC).read();
    for off in [0x44isize, 0x48, 0x4C, 0x50, 0x54, 0x58] {
        let veh = rd::<GPtr<u8>>(this, 0x10);
        wr::<f32>(veh, off, f32_of(x(rd::<f32>(veh, off)).fadd_m(k)));
    }
    let mut anim = ped.addr() as u32;
    if ped_is_player(ped) != 0 {
        let pad = match rd::<i32>(ped, 0x598) {
            0 => pad_get_pad(0),
            1 => pad_get_pad(1),
            _ => ped,
        };
        let veh = rd::<GPtr<u8>>(this, 0x10);
        let mut engine_bad = false;
        if rd::<u32>(veh, 0x590) == 0 && damage_get_engine_status(veh.byte_add(0x5A0)) >= 0xE1 {
            engine_bad = true;
        }
        let input = if rd::<u16>(pad, 0x10E) != 0 || rd::<i32>(rd::<GPtr<u8>>(this, 0x10), 0x4F8) == 6 {
            false
        } else {
            pad_get_target(pad) != 0
                || [0isize, 2, 0x10, 0x12, 0x14, 0x16]
                    .iter()
                    .any(|&o| rd::<u16>(pad, o) != 0)
        };
        let finish = input || rd::<u32>(rd::<GPtr<u8>>(this, 0x10), 0x490) != 0 || engine_bad;
        if finish {
            let veh = rd::<GPtr<u8>>(this, 0x10);
            if rd::<u32>(veh, 0x590) == 0 {
                let dm = |t: GPtr<u8>| rd::<GPtr<u8>>(t, 0x10).byte_add(0x5A0);
                let door = rd::<u32>(this, 0x14);
                if damage_get_door_status_component(dm(this), door) == 0
                    || damage_get_door_status_component(dm(this), rd::<u32>(this, 0x14)) == 2
                {
                    let s = damage_get_door_status_component(dm(this), rd::<u32>(this, 0x14));
                    damage_set_door_status_component(dm(this), rd::<u32>(this, 0x14), s.wrapping_add(1));
                }
            }
            wr::<u8>(this, 8, 1);
            return;
        }
    }
    let gp = slot(&mut v).byte_add(4);
    close_outside_compute_anim_id_orig(this, gp, slot(&mut anim));
    let a = anim_add_animation(rd::<u32>(ped, 0x18), rd::<u32>(gp, 0), anim);
    wr::<GPtr<u8>>(this, 0xC, a);
    anim_set_finish_callback(a, 0x646680, this.addr() as u32);
}

/// `CTaskSimpleCarGetIn::StartAnim(CPed*)` (0x64B2D0): door (+0x1C) 8/9/10/11/18 -> anim 0x168/0x16A/
/// 0x167/0x169/0x16B (byte table 0x64B3B4, jump table 0x64B39C); for other doors the anim id is the ped
/// pointer (the original reads its argument slot). `group = vehicle (+0x18) anim group.GetGroup(anim)`;
/// group 0x6B with door 10 becomes 0x58 when the vehicle's vtable +0x94 (10) is true or its vtable +0x8C
/// (door) is false. `BlendAnimation(clump, group, anim, 4.0)` into +0xC, finish callback 0x6467F0.
#[hook(addr = 0x64B2D0, conv = thiscall, name = "CTaskSimpleCarGetIn::StartAnim", expect = "53 56 8B F1 8B 46 1C")]
pub fn get_in_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let anim = match rd::<i32>(this, 0x1C).wrapping_sub(8) {
        0 => 0x168,
        1 => 0x16A,
        2 => 0x167,
        3 => 0x169,
        10 => 0x16B,
        _ => ped.addr() as u32,
    };
    let mut group = vehicle_anim_group_get_group(veh_anim_group(rd::<GPtr<u8>>(this, 0x18)), anim);
    if group == 0x6B
        && rd::<i32>(this, 0x1C) == 0xA
        && (v_veh_slot_94(rd::<GPtr<u8>>(this, 0x18), 0xA) != 0
            || v_veh_slot_8c(rd::<GPtr<u8>>(this, 0x18), rd::<u32>(this, 0x1C)) == 0)
    {
        group = 0x58;
    }
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), group, anim, 0x4080_0000);
    wr::<GPtr<u8>>(this, 0xC, a);
    anim_set_finish_callback(a, 0x6467F0, this.addr() as u32);
}

// ---------------------------------------------------------------------------------------------
// ProcessPed of the simple entry tasks
// ---------------------------------------------------------------------------------------------

/// `CTaskSimpleCarAlign::ProcessPed(CPed*)` (0x64D6D0): true when finished (+8) or without a vehicle
/// (+0x10); without an anim (+0xC): `FixHeading(ped)` then `StartAnim(ped)`; false.
#[hook(addr = 0x64D6D0, conv = thiscall, name = "CTaskSimpleCarAlign::ProcessPed", expect = "56 8B F1 8A 46 08")]
pub fn car_align_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 8) != 0 || rd::<u32>(this, 0x10) == 0 {
        return 1;
    }
    if rd::<u32>(this, 0xC) == 0 {
        car_align_fix_heading_orig(this, ped);
        car_align_start_anim_orig(this, ped);
    }
    0
}

/// `CTaskSimpleCarOpenDoorFromOutside::ProcessPed(CPed*)` (0x64D710). True when finished (+8) or without a
/// vehicle (+0x10). When bit 6 of the anim group's flags (+8 of the record) is set the door is opened at
/// once: `ComputeAnimID`, `ProcessOpenDoor(ped, door +0x14, group, anim, 1.0)`, true (anim local zeroed
/// where the original's is uninitialised). Otherwise: on the first call (+0xC null) +0x20 = the door's
/// open ratio (vtable +0x74), `StartAnim`, and when the driver of the vehicle is a player, +0x18 is set,
/// and the ped's +0x598 is 6: driver +0x474 bit 26 cleared, +0x19 = 1. Then with anim time (+0x20) below
/// the group's open time (record +0x20), on an automobile: `OpenDoor(ped, door, component, (1 - t/T) *
/// +0x20, false)` (component 3/5/2/4 for doors 8..11; for other doors the original passes the time's
/// bits, read from its reused argument slot); else `ProcessOpenDoor(ped, door, GetGroup(anim id), anim
/// id, time)`. False.
#[hook(addr = 0x64D710, conv = thiscall, name = "CTaskSimpleCarOpenDoorFromOutside::ProcessPed", expect = "83 EC 08 56 8B F1")]
pub fn open_door_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 8) != 0 {
        return 1;
    }
    let veh = rd::<GPtr<u8>>(this, 0x10);
    if veh.is_null() {
        return 1;
    }
    if rd::<u32>(veh_anim_group(veh), 8) & 0x40 != 0 {
        let mut anim = 0u32;
        let mut group = 0u32;
        open_door_compute_anim_id_orig(this, slot(&mut group), slot(&mut anim));
        v_veh_process_open_door(
            rd::<GPtr<u8>>(this, 0x10),
            ped,
            rd::<u32>(this, 0x14),
            group,
            anim,
            1.0,
        );
        return 1;
    }
    if rd::<u32>(this, 0xC) == 0 {
        let r = v_veh_get_door_angle_open_ratio(veh, rd::<u32>(this, 0x14));
        wr::<f32>(this, 0x20, f32_of(X87::from_f64_exact(r)));
        open_door_start_anim_orig(this, ped);
        let v = rd::<GPtr<u8>>(this, 0x10);
        if !v.is_null() {
            let d = rd::<GPtr<u8>>(v, 0x460);
            if !d.is_null()
                && ped_is_player(d) != 0
                && rd::<u8>(this, 0x18) != 0
                && !ped.is_null()
                && rd::<i32>(ped, 0x598) == 6
            {
                let d = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), 0x460);
                wr::<u32>(d, 0x474, rd::<u32>(d, 0x474) & 0xFBFF_FFFF);
                wr::<u8>(this, 0x19, 1);
            }
        }
    }
    let veh = rd::<GPtr<u8>>(this, 0x10);
    let rec = veh_anim_group(veh);
    let a = rd::<GPtr<u8>>(this, 0xC);
    let total = rd::<f32>(rec, 0x20);
    let t = rd::<f32>(a, 0x20);
    if !x(t).fcom(x(total)).test_parity(5) && rd::<u32>(veh, 0x590) == 0 {
        let door = rd::<u32>(this, 0x14);
        let comp = match (door as i32).wrapping_sub(8) {
            0 => 3,
            1 => 5,
            2 => 2,
            3 => 4,
            _ => t.to_bits(),
        };
        let r = x(t)
            .fdiv_m(total)
            .fsubr_m(g::<f32>(0x858624).read())
            .fmul_m(rd::<f32>(this, 0x20));
        v_veh_open_door(veh, ped, door, comp, f32_of(r), 0);
        return 0;
    }
    let id = rd::<i16>(a, 0x2C) as i32 as u32;
    let group = vehicle_anim_group_get_group(rec, id);
    let a = rd::<GPtr<u8>>(this, 0xC);
    let t = f32_of(x(rd::<f32>(a, 0x20)));
    v_veh_process_open_door(
        rd::<GPtr<u8>>(this, 0x10),
        ped,
        rd::<u32>(this, 0x14),
        group,
        rd::<i16>(a, 0x2C) as i32 as u32,
        t,
    );
    0
}

/// `CTaskSimpleCarCloseDoorFromInside::ProcessPed(CPed*)` (0x64D9A0). True when finished (+8) or without
/// a vehicle (+0x10). When bit 1 of the anim group's flags (record +8) is set: `ComputeAnimID`,
/// `ProcessOpenDoor(ped, door +0x14, group, anim, 1.0)`, true (anim local zeroed where the original's is
/// uninitialised). Otherwise without an anim: true for a ped in state 0x3F (+0x530), else `StartAnim`;
/// then `ProcessOpenDoor(ped, door, GetGroup(anim id), anim id, anim time)`; false.
#[hook(addr = 0x64D9A0, conv = thiscall, name = "CTaskSimpleCarCloseDoorFromInside::ProcessPed", expect = "83 EC 08 56 8B F1")]
pub fn close_inside_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 8) != 0 {
        return 1;
    }
    let veh = rd::<GPtr<u8>>(this, 0x10);
    if veh.is_null() {
        return 1;
    }
    if rd::<u32>(veh_anim_group(veh), 8) & 2 != 0 {
        let mut anim = 0u32;
        let mut group = 0u32;
        close_inside_compute_anim_id_orig(this, slot(&mut group), slot(&mut anim));
        v_veh_process_open_door(
            rd::<GPtr<u8>>(this, 0x10),
            ped,
            rd::<u32>(this, 0x14),
            group,
            anim,
            1.0,
        );
        return 1;
    }
    if rd::<u32>(this, 0xC) == 0 {
        if rd::<i32>(ped, 0x530) == 0x3F {
            return 1;
        }
        close_inside_start_anim_orig(this, ped);
    }
    process_open_door_from_anim(this, ped);
    0
}

/// The common tail: `ProcessOpenDoor(ped, door +0x14, GetGroup(anim id), anim id, anim time)` with the anim
/// at +0xC (id = word +0x2C sign-extended, time +0x20) and the vehicle at +0x10.
fn process_open_door_from_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let id = rd::<i16>(rd::<GPtr<u8>>(this, 0xC), 0x2C) as i32 as u32;
    let group = vehicle_anim_group_get_group(veh_anim_group(rd::<GPtr<u8>>(this, 0x10)), id);
    let a = rd::<GPtr<u8>>(this, 0xC);
    let t = f32_of(x(rd::<f32>(a, 0x20)));
    v_veh_process_open_door(
        rd::<GPtr<u8>>(this, 0x10),
        ped,
        rd::<u32>(this, 0x14),
        group,
        rd::<i16>(a, 0x2C) as i32 as u32,
        t,
    );
}

/// `CTaskSimpleCarCloseDoorFromOutside::ProcessPed(CPed*)` (0x64DA90). True when finished (+8), without a
/// vehicle (+0x10) or when bit 0 of the anim group's flags (record +8) is set. Without an anim:
/// `StartAnim`, true when it still has none. Then `ComputeAnimID(&group, &anim)` (anim out-slot = the ped
/// argument slot) and `ProcessOpenDoor(ped, door +0x14, group, anim, anim time)`; false.
#[hook(addr = 0x64DA90, conv = thiscall, name = "CTaskSimpleCarCloseDoorFromOutside::ProcessPed", expect = "51 56 8B F1 8A 46 08")]
pub fn close_outside_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 8) != 0 {
        return 1;
    }
    let veh = rd::<GPtr<u8>>(this, 0x10);
    if veh.is_null() || rd::<u8>(veh_anim_group(veh), 8) & 1 != 0 {
        return 1;
    }
    if rd::<u32>(this, 0xC) == 0 {
        close_outside_start_anim_orig(this, ped);
        if rd::<u32>(this, 0xC) == 0 {
            return 1;
        }
    }
    let mut group = 0u32;
    let mut anim = ped.addr() as u32;
    close_outside_compute_anim_id_orig(this, slot(&mut group), slot(&mut anim));
    let t = f32_of(x(rd::<f32>(rd::<GPtr<u8>>(this, 0xC), 0x20)));
    v_veh_process_open_door(
        rd::<GPtr<u8>>(this, 0x10),
        ped,
        rd::<u32>(this, 0x14),
        group,
        anim,
        t,
    );
    0
}

/// `CTaskSimpleCarGetIn::ProcessPed(CPed*)` (0x64DB30). True without a vehicle (+0x18). Finished (+8):
/// `RemoveWeaponWhenEnteringVehicle(0)`; when [0x969164] is set and the ped is a player, the vehicle gets
/// +0x40 |= 0x800000 and 0x40000 and +0x42A bit 5 cleared; true. Running: `StartAnim` without an anim;
/// on a vehicle of class 10 (+0x594) whose anim (+0xC) is running (bit 0 of +0x2E) with id 0x167..0x16A
/// and time (+0x20) past [0x8D2ED4] * [0x858F10] (10 * 1/30), applies the move force
/// `(timestep [0xB7CB5C] * mass +0x8C * [0x8D2ED0]) * matrix +0x10` (`operator*` 0x40FE90). False.
#[hook(addr = 0x64DB30, conv = thiscall, name = "CTaskSimpleCarGetIn::ProcessPed", expect = "83 EC 0C 56 8B F1")]
pub fn get_in_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0x18) == 0 {
        return 1;
    }
    if rd::<u8>(this, 8) != 0 {
        ped_remove_weapon_when_entering_vehicle(ped, 0);
        if g::<u8>(0x969164).read() != 0 && ped_is_player(ped) != 0 {
            let v = rd::<GPtr<u8>>(this, 0x18);
            if !v.is_null() {
                wr::<u32>(v, 0x40, rd::<u32>(v, 0x40) | 0x80_0000);
                let v = rd::<GPtr<u8>>(this, 0x18);
                wr::<u32>(v, 0x40, rd::<u32>(v, 0x40) | 0x4_0000);
                let v = rd::<GPtr<u8>>(this, 0x18);
                wr::<u8>(v, 0x42A, rd::<u8>(v, 0x42A) & 0xDF);
            }
        }
        return 1;
    }
    if rd::<u32>(this, 0xC) == 0 {
        get_in_start_anim_orig(this, ped);
    }
    let veh = rd::<GPtr<u8>>(this, 0x18);
    if rd::<i32>(veh, 0x594) != 0xA {
        return 0;
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    if a.is_null() || rd::<u8>(a, 0x2E) & 1 == 0 {
        return 0;
    }
    if !matches!(rd::<u16>(a, 0x2C), 0x167..=0x16A) {
        return 0;
    }
    let lim = x(g::<f32>(0x8D2ED4).read()).fmul_m(g::<f32>(0x858F10).read());
    if lim.fcom(x(rd::<f32>(a, 0x20))).test_parity(5) {
        return 0;
    }
    let f = x(g::<f32>(0xB7CB5C).read())
        .fmul_m(rd::<f32>(veh, 0x8C))
        .fmul_m(g::<f32>(0x8D2ED0).read());
    let mut out = [0u32; 3];
    let r = vector_scale(
        slot(&mut out),
        f32_of(f),
        rd::<GPtr<u8>>(veh, 0x14).byte_add(0x10),
    );
    physical_apply_move_force(veh, rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8));
    0
}

// ---------------------------------------------------------------------------------------------
// CTaskSimpleCarSetPedInAsDriver / AsPassenger
// ---------------------------------------------------------------------------------------------

/// Shared start of both set-ped-in tasks: `ped->m_pVehicle` (+0x58C) is re-pointed at the task's vehicle
/// (+0x10): `CleanUpOldReference` on the old one (if any), store, `RegisterReference`.
fn set_ped_vehicle_ref(this: GPtr<u8>, ped: GPtr<u8>) {
    let r = ped.byte_add(0x58C);
    let old = rd::<GPtr<u8>>(r, 0);
    if !old.is_null() {
        entity_clean_up_old_reference(old, r);
    }
    let v = rd::<GPtr<u8>>(this, 0x10);
    wr::<GPtr<u8>>(r, 0, v);
    entity_register_reference(v, r);
}

/// The occupant of the seat being taken (read again through `occ` before every use, as the original
/// does) is thrown out unless it is the ped or has an active task 0x2C0, 0x338, 0x2C1 or 0x33A: a stack
/// `CTaskSimpleCarSetPedOut(veh, ComputeTargetDoorToExit(veh, occupant), true)` (0x18 bytes, zeroed
/// here) with +0x11 = 1 is run once on the occupant and destroyed.
fn throw_out_occupant(this: GPtr<u8>, ped: GPtr<u8>, occ: impl Fn() -> GPtr<u8>) {
    let o = occ();
    if o == ped || o.is_null() {
        return;
    }
    for ty in [0x2C0, 0x338, 0x2C1, 0x33A] {
        if !tm_find_active_task_by_type(rd::<GPtr<u8>>(occ(), 0x47C).byte_add(4), ty).is_null() {
            return;
        }
    }
    let door = car_enter_exit_compute_target_door_to_exit(rd::<GPtr<u8>>(this, 0x10), occ());
    let mut so = [0u32; 6];
    let sp = slot(&mut so);
    car_set_ped_out_ctor(sp, rd::<GPtr<u8>>(this, 0x10), door, 1);
    wr::<u8>(sp, 0x11, 1);
    car_set_ped_out_process_ped(sp, occ());
    car_set_ped_out_dtor(sp);
}

/// Warp: the ped's position (matrix +0x30.., else +4..) = the vehicle's (matrix +0x30.., else +4..).
fn warp_ped_to_vehicle(this: GPtr<u8>, ped: GPtr<u8>) {
    let v = rd::<GPtr<u8>>(this, 0x10);
    let vm = rd::<GPtr<u8>>(v, 0x14);
    let src = if vm.is_null() {
        v.byte_add(4)
    } else {
        vm.byte_add(0x30)
    };
    let p = [rd::<f32>(src, 0), rd::<f32>(src, 4), rd::<f32>(src, 8)];
    for (i, c) in (0isize..).zip(p) {
        let pm = rd::<GPtr<u8>>(ped, 0x14);
        let c = f32_of(x(c));
        if pm.is_null() {
            wr::<f32>(ped, 4 + i * 4, c);
        } else {
            wr::<f32>(pm, 0x30 + i * 4, c);
        }
    }
}

/// Player bookkeeping shared by both tasks: players clear adrenaline (+0x480 -> +0x8D = 1); a player
/// entering a vehicle without bit 1 of +0x42A sets it and reports crime 6 (`FindPlayerPed(-1)`).
fn set_ped_in_player_bits(this: GPtr<u8>, ped: GPtr<u8>) {
    if ped_is_player(ped) != 0 {
        wr::<u8>(rd::<GPtr<u8>>(ped, 0x480), 0x8D, 1);
        player_ped_clear_adrenaline(ped);
    }
    if ped_is_player(ped) != 0 {
        let v = rd::<GPtr<u8>>(this, 0x10);
        let b = rd::<u8>(v, 0x42A);
        if b & 2 == 0 {
            wr::<u8>(v, 0x42A, b | 2);
            let p = find_player_ped(-1);
            crime_report_crime(6, rd::<GPtr<u8>>(this, 0x10), p);
        }
    }
}

/// Ped state shared by both tasks after the seat is taken: `UpdateStatEnteringVehicle`,
/// `SetMoveState(0)`, ped vtable +0x5C, +0x1C bit 0 cleared, vehicle +0x45C word 0xFFFF -> 15000.
fn set_ped_in_ped_state(this: GPtr<u8>, ped: GPtr<u8>) {
    ped_update_stat_entering_vehicle(ped);
    ped_set_move_state(ped, 0);
    v_ped_slot_5c(ped);
    wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) & 0xFFFF_FFFE);
    let v = rd::<GPtr<u8>>(this, 0x10);
    if rd::<u16>(v, 0x45C) == 0xFFFF {
        wr::<u16>(v, 0x45C, 0x3A98);
    }
}

/// `CTaskSimpleCarSetPedInAsDriver::ProcessPed(CPed*)` (0x64B950). Re-points the ped's vehicle (+0x58C),
/// throws out another driver (+0x460, see `throw_out_occupant`), +0x55C = +0x558, warps when +0x18 is set.
/// A group leader with no group +0x34 tells its group (`CEventLeaderEnteredCarAsDriver(veh)`, 0x18-byte
/// event, wrapped in a stack `CEventGroupEvent` passed to `AddEvent`). `SetDriver`, ped +0x46C |= 0x100,
/// player bits, ped state, vehicle +0x428 bit 4 = !(+0x42D bit 1), `SetPedState(0x32)`. Vehicle status
/// (+0x36 bits 3..7): players 0; other peds 7 (vehicle class +0x594 == 6) or 3, except when it is 2 and
/// ped +0x484 != 2. +0x19 -> `ClearGettingInFlags`, +0x1A subtracted from vehicle +0x485,
/// `RemoveGetInAnims`, `AddInCarAnim(veh, ped, true)`, `RemoveWeaponWhenEnteringVehicle(0)`; ped +0x46C
/// bit 13 cleared when vehicle +0x429 bit 1 is set and the model (+0x22) is not 0x1AF, else set. Primary
/// task 4 = `CTaskSimpleCarDrive(veh, +0x14 or 0 when warping, 0)` (0x60 bytes, null when the pool is full).
/// A player taking a law-enforcement vehicle adds a stack `CEventCopCarBeingStolen(ped, veh)` to the global
/// event group. True.
#[hook(addr = 0x64B950, conv = thiscall, name = "CTaskSimpleCarSetPedInAsDriver::ProcessPed", expect = "6A FF 64 A1 00 00 00 00")]
pub fn set_ped_in_as_driver_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    set_ped_vehicle_ref(this, ped);
    throw_out_occupant(this, ped, || rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), 0x460));
    wr::<u32>(ped, 0x55C, rd::<u32>(ped, 0x558));
    if rd::<u8>(this, 0x18) != 0 {
        warp_ped_to_vehicle(this, ped);
    }
    let grp = ped_groups_get_peds_group(ped);
    if !grp.is_null() && group_membership_is_leader(grp.byte_add(8), ped) != 0 && rd::<u32>(grp, 0x34) == 0 {
        let e = event_new(0x18);
        let ev = if e.is_null() {
            GPtr::NULL
        } else {
            event_leader_entered_car_as_driver_ctor(e, rd::<GPtr<u8>>(this, 0x10))
        };
        let mut ge = [0u32; 6];
        let gp = slot(&mut ge);
        event_group_event_ctor(gp, ped, ev);
        group_intelligence_add_event(grp.byte_add(0x30), gp);
        event_group_event_dtor(gp);
    }
    vehicle_set_driver(rd::<GPtr<u8>>(this, 0x10), ped);
    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 0x100);
    set_ped_in_player_bits(this, ped);
    set_ped_in_ped_state(this, ped);
    let v = rd::<GPtr<u8>>(this, 0x10);
    let b = rd::<u8>(v, 0x428);
    wr::<u8>(
        v,
        0x428,
        if rd::<u8>(v, 0x42D) & 2 == 0 {
            b | 0x10
        } else {
            b & 0xEF
        },
    );
    ped_set_ped_state(ped, 0x32);
    let player = ped_is_player(ped) != 0;
    let v = rd::<GPtr<u8>>(this, 0x10);
    let st = rd::<u8>(v, 0x36);
    if player {
        wr::<u8>(v, 0x36, st & 7);
    } else if st & 0xF8 != 0x10 || rd::<u8>(ped, 0x484) == 2 {
        wr::<u8>(
            v,
            0x36,
            (st & 7) | if rd::<i32>(v, 0x594) == 6 { 0x38 } else { 0x18 },
        );
    }
    let f = rd::<u8>(this, 0x19);
    if f != 0 {
        vehicle_clear_getting_in_flags(rd::<GPtr<u8>>(this, 0x10), u32::from(f));
    }
    let n = rd::<u8>(this, 0x1A);
    if n != 0 {
        let v = rd::<GPtr<u8>>(this, 0x10);
        wr::<u8>(v, 0x485, rd::<u8>(v, 0x485).wrapping_sub(n));
    }
    car_enter_exit_remove_get_in_anims(ped);
    car_enter_exit_add_in_car_anim(rd::<GPtr<u8>>(this, 0x10), ped, 1);
    ped_remove_weapon_when_entering_vehicle(ped, 0);
    let v = rd::<GPtr<u8>>(this, 0x10);
    let fl = rd::<u32>(ped, 0x46C);
    if rd::<u8>(v, 0x429) & 2 != 0 && rd::<u16>(v, 0x22) != 0x1AF {
        wr::<u32>(ped, 0x46C, fl & 0xFFFF_DFFF);
    } else {
        wr::<u32>(ped, 0x46C, fl | 0x2000);
    }
    let t = task_new(0x60);
    let t = if t.is_null() {
        GPtr::NULL
    } else if rd::<u8>(this, 0x18) == 0 {
        car_drive_simple_ctor(t, rd::<GPtr<u8>>(this, 0x10), rd::<u32>(this, 0x14), 0)
    } else {
        car_drive_simple_ctor(t, rd::<GPtr<u8>>(this, 0x10), 0, 0)
    };
    tm_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4, 0);
    if ped_is_player(ped) != 0 {
        let v = rd::<GPtr<u8>>(this, 0x10);
        if !v.is_null() && vehicle_is_law_enforcement(v) != 0 {
            let mut ev = [0u32; 8];
            let ep = slot(&mut ev);
            event_cop_car_being_stolen_ctor(ep, ped, rd::<GPtr<u8>>(this, 0x10));
            let g = get_event_global_group();
            event_group_add(g, ep, 0);
            event_cop_car_being_stolen_dtor(ep);
        }
    }
    1
}

/// `CTaskSimpleCarSetPedInAsPassenger::ProcessPed(CPed*)` (0x64B5D0, entry `push -1; nop; jmp 0x401DB5`, a
/// protection island that loads `fs:[0]` and jumps back to 0x64B5D8). A ped already in a vehicle (+0x46C
/// bit 8) with a parent task (+4) makes one `GetTaskType` call on it (result unused). Re-points the ped's
/// vehicle, +0x46C |= 0x100, +0x55C = +0x558, player bits. Unless vehicle +0x429 bit 1 is set, the seat
/// is `ComputePassengerIndexFromCarDoor(veh, door +0x14)` and its occupant (+0x464 + 4 * seat) is thrown
/// out (`throw_out_occupant`); warps when +0x1C is set; `AddPassenger(ped, seat)` for a seat, else the
/// auto-seat `AddPassenger(ped)`. Ped state, `SetPedState(0x32)`, +0x1D -> `ClearGettingInFlags`, +0x1E
/// subtracted from vehicle +0x485, `RemoveWeaponWhenEnteringVehicle(0)`, ped +0x46C bit 13 cleared when
/// vehicle +0x429 bit 1 is set (no model test here), else set; `RemoveGetInAnims`, `AddInCarAnim(veh,
/// ped, false)`; without warp the line-up utility (+0x18) `ProcessPed(ped, veh, null)`. Primary task 4 =
/// `CTaskSimpleCarDrive(veh, +0x18, 0)` (0x60 bytes, null when the pool is full). True.
#[hook(addr = 0x64B5D0, conv = thiscall, name = "CTaskSimpleCarSetPedInAsPassenger::ProcessPed", expect = "6A FF 90 E9 DD 67 DB FF")]
pub fn set_ped_in_as_passenger_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let parent = rd::<GPtr<u8>>(this, 4);
        if !parent.is_null() {
            v_get_task_type(parent);
        }
    }
    set_ped_vehicle_ref(this, ped);
    let fl = rd::<u32>(ped, 0x46C);
    wr::<u32>(ped, 0x55C, rd::<u32>(ped, 0x558));
    wr::<u32>(ped, 0x46C, fl | 0x100);
    set_ped_in_player_bits(this, ped);
    let v = rd::<GPtr<u8>>(this, 0x10);
    let seat = if rd::<u8>(v, 0x429) & 2 != 0 {
        -1
    } else {
        let s = car_enter_exit_compute_passenger_index_from_car_door(v, rd::<u32>(this, 0x14));
        if s != -1 {
            let off = (s as isize).wrapping_mul(4).wrapping_add(0x464);
            throw_out_occupant(this, ped, || rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), off));
        }
        s
    };
    if rd::<u8>(this, 0x1C) != 0 {
        warp_ped_to_vehicle(this, ped);
    }
    if seat != -1 {
        vehicle_add_passenger_fixed(rd::<GPtr<u8>>(this, 0x10), ped, seat);
    } else {
        vehicle_add_passenger_auto(rd::<GPtr<u8>>(this, 0x10), ped);
    }
    set_ped_in_ped_state(this, ped);
    ped_set_ped_state(ped, 0x32);
    let f = rd::<u8>(this, 0x1D);
    if f != 0 {
        vehicle_clear_getting_in_flags(rd::<GPtr<u8>>(this, 0x10), u32::from(f));
    }
    let n = rd::<u8>(this, 0x1E);
    if n != 0 {
        let v = rd::<GPtr<u8>>(this, 0x10);
        wr::<u8>(v, 0x485, rd::<u8>(v, 0x485).wrapping_sub(n));
    }
    ped_remove_weapon_when_entering_vehicle(ped, 0);
    let bit = rd::<u8>(rd::<GPtr<u8>>(this, 0x10), 0x429) & 2 != 0;
    let fl = rd::<u32>(ped, 0x46C);
    wr::<u32>(ped, 0x46C, if bit { fl & 0xFFFF_DFFF } else { fl | 0x2000 });
    car_enter_exit_remove_get_in_anims(ped);
    car_enter_exit_add_in_car_anim(rd::<GPtr<u8>>(this, 0x10), ped, 0);
    if rd::<u8>(this, 0x1C) == 0 {
        line_up_process_ped(
            rd::<GPtr<u8>>(this, 0x18),
            ped,
            rd::<GPtr<u8>>(this, 0x10),
            GPtr::NULL,
        );
    }
    let t = task_new(0x60);
    let t = if t.is_null() {
        GPtr::NULL
    } else {
        car_drive_simple_ctor(t, rd::<GPtr<u8>>(this, 0x10), rd::<u32>(this, 0x18), 0)
    };
    tm_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4, 0);
    1
}

// ---------------------------------------------------------------------------------------------
// CTaskComplexEnterCar helpers, CTaskComplexGoToCarDoorAndStandStill::IsVehicleInRange
// ---------------------------------------------------------------------------------------------

/// `CTaskComplexEnterCar::SetVehicleFlags(CPed*)` (0x63AB90): `f = ComputeDoorFlag(veh +0xC, door +0x14,
/// true)`, `SetGettingInFlags(f)`, +0x44 = f (byte), +0x28 = 1, vehicle +0x485 += 1. With a second door
/// (+0x18): its flag (low byte stored over the ped argument slot) differs from `f` -> `SetGettingInFlags`
/// gets that whole slot dword (flag byte + the upper three bytes of the ped pointer; the callee reads the
/// low byte) and +0x44 |= flag.
#[hook(addr = 0x63AB90, conv = thiscall, name = "CTaskComplexEnterCar::SetVehicleFlags", expect = "51 53 56 8B F1")]
pub fn enter_car_set_vehicle_flags(this: GPtr<u8>, ped: GPtr<u8>) {
    let f = car_enter_exit_compute_door_flag(rd::<GPtr<u8>>(this, 0xC), rd::<u32>(this, 0x14), 1);
    vehicle_set_getting_in_flags(rd::<GPtr<u8>>(this, 0xC), f);
    let v = rd::<GPtr<u8>>(this, 0xC);
    wr::<u8>(this, 0x44, f as u8);
    wr::<u8>(this, 0x28, 1);
    wr::<u8>(v, 0x485, rd::<u8>(v, 0x485).wrapping_add(1));
    let d2 = rd::<u32>(this, 0x18);
    if d2 == 0 {
        return;
    }
    let f2 = car_enter_exit_compute_door_flag(rd::<GPtr<u8>>(this, 0xC), d2, 1) as u8;
    if u32::from(f2) == f {
        return;
    }
    let s = ((ped.addr() as u32) & 0xFFFF_FF00) | u32::from(f2);
    vehicle_set_getting_in_flags(rd::<GPtr<u8>>(this, 0xC), s);
    wr::<u8>(this, 0x44, rd::<u8>(this, 0x44) | f2);
}

/// `CTaskComplexEnterCar::PrepareVehicleForPedEnter(CPed*)` (0x63AC10): a non-zero vehicle (+0xC) byte
/// +0x3D0 is copied to +0x48 as a float (`fild`). Then +0x3D0 = 0, except for a player when the door
/// (+0x14) is one to open (`CarHasDoorToOpen`) but not openable (`CarHasOpenableDoor(veh, door, ped)`).
#[hook(addr = 0x63AC10, conv = thiscall, name = "CTaskComplexEnterCar::PrepareVehicleForPedEnter", expect = "51 56 8B F1 8B 46 0C")]
pub fn enter_car_prepare_vehicle_for_ped_enter(this: GPtr<u8>, ped: GPtr<u8>) {
    let b = rd::<u8>(rd::<GPtr<u8>>(this, 0xC), 0x3D0);
    if b != 0 {
        wr::<f32>(this, 0x48, f32_of(xi(i32::from(b))));
    }
    if ped_is_player(ped) != 0
        && car_enter_exit_car_has_door_to_open(rd::<GPtr<u8>>(this, 0xC), rd::<u32>(this, 0x14)) != 0
        && car_enter_exit_car_has_openable_door(rd::<GPtr<u8>>(this, 0xC), rd::<u32>(this, 0x14), ped) == 0
    {
        return;
    }
    wr::<u8>(rd::<GPtr<u8>>(this, 0xC), 0x3D0, 0);
}

/// `CTaskComplexEnterCar::PreparePedForVehicleEnter(CPed*)` (0x63AC80, `this` unused): ped +0x1C bit 0
/// cleared; with a duck task (`GetTaskDuck(true)` on +0x47C) a second `GetTaskDuck(true)` is asked to
/// abort (`MakeAbortable(ped, 1, null)`).
#[hook(addr = 0x63AC80, conv = thiscall, name = "CTaskComplexEnterCar::PreparePedForVehicleEnter", expect = "56 8B 74 24 08")]
pub fn enter_car_prepare_ped_for_vehicle_enter(_this: GPtr<u8>, ped: GPtr<u8>) {
    wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) & 0xFFFF_FFFE);
    if intelligence_get_task_duck(rd::<GPtr<u8>>(ped, 0x47C), 1).is_null() {
        return;
    }
    let d = intelligence_get_task_duck(rd::<GPtr<u8>>(ped, 0x47C), 1);
    v_make_abortable(d, ped, 1, GPtr::NULL);
}

/// `CTaskComplexEnterCar::CreateTaskUtilityLineUpPedWithCar(CPed*)` (0x63ACC0): `offset =
/// GetPositionToOpenCarDoor(veh +0xC, door +0x14) - ped position` (matrix +0x30, else +4; each component
/// rounded to f32), +0x38 = a CRT-heap (`operator new(0x1C)`) `CTaskUtilityLineUpPedWithCar(offset, 600,
/// 0, door)`, null when the allocation fails.
#[hook(addr = 0x63ACC0, conv = thiscall, name = "CTaskComplexEnterCar::CreateTaskUtilityLineUpPedWithCar", expect = "64 A1 00 00 00 00")]
pub fn enter_car_create_task_utility_line_up_ped_with_car(this: GPtr<u8>, ped: GPtr<u8>) {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pp = if m.is_null() {
        ped.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    let mut out = [0u32; 3];
    let r = car_enter_exit_get_position_to_open_car_door(
        slot(&mut out),
        rd::<GPtr<u8>>(this, 0xC),
        rd::<u32>(this, 0x14),
    );
    let mut off = [0f32; 3];
    for (i, o) in (0isize..).zip(off.iter_mut()) {
        *o = f32_of(x(rd::<f32>(r, i * 4)).fsub_m(rd::<f32>(pp, i * 4)));
    }
    let t = operator_new(0x1C);
    let u = if t.is_null() {
        GPtr::NULL
    } else {
        line_up_ped_with_car_ctor(t, slot(&mut off), 0x258, 0, rd::<u32>(this, 0x14))
    };
    wr::<GPtr<u8>>(this, 0x38, u);
}

/// `CTaskComplexGoToCarDoorAndStandStill::IsVehicleInRange(CPed*)` (0x6458A0): with `d` = vehicle (+0xC)
/// position - ped position (matrix +0x30, else +4; kept in x87 precision), true when `r * r >
/// (dz*dz + dx*dx) + dy*dy` for the range `r` at +0x20 (false for NaN). Returns 0/1 in EAX.
#[hook(addr = 0x6458A0, conv = thiscall, name = "CTaskComplexGoToCarDoorAndStandStill::IsVehicleInRange", expect = "8B 41 0C 8B 50 14", preserve = "ecx")]
pub fn go_to_car_door_is_vehicle_in_range(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let v = rd::<GPtr<u8>>(this, 0xC);
    let vm = rd::<GPtr<u8>>(v, 0x14);
    let vp = if vm.is_null() {
        v.byte_add(4)
    } else {
        vm.byte_add(0x30)
    };
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    let pp = if pm.is_null() {
        ped.byte_add(4)
    } else {
        pm.byte_add(0x30)
    };
    let dx = x(rd::<f32>(vp, 0)).fsub_m(rd::<f32>(pp, 0));
    let dy = x(rd::<f32>(vp, 4)).fsub_m(rd::<f32>(pp, 4));
    let dz = x(rd::<f32>(vp, 8)).fsub_m(rd::<f32>(pp, 8));
    let r = x(rd::<f32>(this, 0x20));
    let s = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    u32::from(!r.fmul(r).fcom(s).test_any(0x41))
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_CAR_ALIGN_SET_PED_POSITION,
    HOOK_CAR_ALIGN_FINISH_ANIM_CB,
    HOOK_OPEN_DOOR_SET_PED_POSITION,
    HOOK_OPEN_DOOR_COMPUTE_ANIM_ID,
    HOOK_OPEN_DOOR_FINISH_ANIM_CB,
    HOOK_CLOSE_INSIDE_SET_PED_POSITION,
    HOOK_CLOSE_INSIDE_COMPUTE_ANIM_ID,
    HOOK_CLOSE_INSIDE_FINISH_ANIM_CB,
    HOOK_CLOSE_OUTSIDE_SET_PED_POSITION,
    HOOK_CLOSE_OUTSIDE_FINISH_ANIM_CB,
    HOOK_GET_IN_SET_PED_POSITION,
    HOOK_GET_IN_FINISH_ANIM_CB,
    HOOK_CAR_ALIGN_START_ANIM,
    HOOK_OPEN_DOOR_START_ANIM,
    HOOK_CLOSE_INSIDE_START_ANIM,
    HOOK_CLOSE_OUTSIDE_START_ANIM,
    HOOK_GET_IN_START_ANIM,
    HOOK_CAR_ALIGN_PROCESS_PED,
    HOOK_OPEN_DOOR_PROCESS_PED,
    HOOK_CLOSE_INSIDE_PROCESS_PED,
    HOOK_CLOSE_OUTSIDE_PROCESS_PED,
    HOOK_GET_IN_PROCESS_PED,
    HOOK_SET_PED_IN_AS_DRIVER_PROCESS_PED,
    HOOK_SET_PED_IN_AS_PASSENGER_PROCESS_PED,
    HOOK_CAR_ALIGN_FIX_HEADING,
    HOOK_ENTER_CAR_SET_VEHICLE_FLAGS,
    HOOK_ENTER_CAR_PREPARE_VEHICLE_FOR_PED_ENTER,
    HOOK_ENTER_CAR_PREPARE_PED_FOR_VEHICLE_ENTER,
    HOOK_ENTER_CAR_CREATE_TASK_UTILITY_LINE_UP_PED_WITH_CAR,
    HOOK_GO_TO_CAR_DOOR_IS_VEHICLE_IN_RANGE,
];
