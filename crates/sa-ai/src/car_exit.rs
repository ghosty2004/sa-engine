//! PORT-AI-3 (`docs/arch/ai-port.md`, section PORT-AI-3): vehicle exit and dragging: `CTaskComplexLeaveCar*`, `CTaskSimpleCarGetOut`/`JumpOut`/`SetPedOut`/
//! `ForcePedOut`/`WaitToSlowDown`/`SlowDragPedOut`/`FallOut`, `CTaskComplexCarSlowBeDraggedOut*`, `CTaskComplexDragPedFromCar`.
//! Ported from the 1.0 US bytes (the EXE wins over gta-reversed and the specs). Every call to another
//! catalogued function goes through `orig!` at the original address; task objects keep their original
//! vtables and come from the original task pool (`common::task_new`).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::*;

orig! {
    // ---- virtual slots ----
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CEvent::GetEventType` (CEvent vtable +4)
    fn v_event_get_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CVehicle::ProcessOpenDoor(CPed*, uint32 door, uint32 group, uint32 anim, float time)` (vtable +0x70)
    fn v_vehicle_process_open_door(v: GPtr<u8>, ped: GPtr<u8>, door: u32, group: u32, anim: u32, time: u32) = thiscall vslot 0x70 => unported;
    /// `CVehicle::GetNumContactWheels` (vtable +0xDC)
    fn v_vehicle_get_num_contact_wheels(v: GPtr<u8>) -> i32 = thiscall vslot 0xDC => unported;
    /// `CVehicle::CanPedStepOutCar(bool)` (vtable +0xE4)
    fn v_vehicle_can_ped_step_out_car(v: GPtr<u8>, ignore: u32) -> u8 = thiscall vslot 0xE4 => unported;
    /// `CVehicle::CanPedJumpOutCar(CPed*)` (vtable +0xE8)
    fn v_vehicle_can_ped_jump_out_car(v: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall vslot 0xE8 => unported;

    // ---- task constructors / destructors / members ----
    /// `CTaskSimplePause::Constructor(int32 time)` (0x48E750, `ret 4`)
    fn pause_ctor(t: GPtr<u8>, time: i32) -> GPtr<u8> = thiscall 0x48E750 => unported;
    /// `CTaskComplexGetUpAndStandStill::Constructor` (0x678130)
    fn get_up_and_stand_still_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x678130 => unported;
    /// `CTaskSimpleDie::Constructor-1` (0x62FA00, `ret 0x10`)
    fn simple_die_ctor1(t: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32) -> GPtr<u8> = thiscall 0x62FA00 => unported;
    /// `CTaskComplexLeaveBoat::Constructor(veh, delay)` (0x63BB60, `ret 8`)
    fn leave_boat_ctor(t: GPtr<u8>, veh: u32, delay: u32) -> GPtr<u8> = thiscall 0x63BB60 => unported;
    /// `CTaskSimpleCarDriveTimed::CTaskSimpleCarDriveTimed(veh, time)` (0x5FF940, `ret 8`)
    fn car_drive_timed_ctor(t: GPtr<u8>, veh: u32, time: u32) -> GPtr<u8> = thiscall 0x5FF940 => unported;
    /// `CTaskSimpleCarCloseDoorFromOutside::Constructor(veh, door, lineUp)` (0x6464F0, `ret 0xC`)
    fn close_door_from_outside_ctor(t: GPtr<u8>, veh: u32, door: u32, line_up: u32) -> GPtr<u8> = thiscall 0x6464F0 => unported;
    /// `CTaskSimpleCarCloseDoorFromOutside::MakeAbortable` (0x64B020, `ret 0xC`)
    fn close_door_from_outside_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall 0x64B020 => unported;
    /// `CTaskSimpleCarCloseDoorFromOutside::Destructor` (0x646570)
    fn close_door_from_outside_dtor(t: GPtr<u8>) = thiscall 0x646570 => unported;
    /// `CTaskSimpleCarWaitToSlowDown::Constructor(veh, mode)` (0x646990, `ret 8`)
    fn wait_to_slow_down_ctor(t: GPtr<u8>, veh: u32, mode: u32) -> GPtr<u8> = thiscall 0x646990 => unported;
    /// `CTaskSimpleCarWaitForDoorNotToBeInUse::Constructor(veh, door, other)` (0x646B70, `ret 0xC`)
    fn wait_for_door_ctor(t: GPtr<u8>, veh: u32, door: u32, other: u32) -> GPtr<u8> = thiscall 0x646B70 => unported;
    /// `CTaskSimpleCarGetOut::Constructor(veh, door, lineUp)` (0x6471D0, `ret 0xC`)
    fn car_get_out_ctor(t: GPtr<u8>, veh: u32, door: u32, line_up: u32) -> GPtr<u8> = thiscall 0x6471D0 => unported;
    /// `CTaskSimpleCarJumpOut::Constructor(veh, door, lineUp)` (0x64BE40, `ret 0xC`)
    fn car_jump_out_ctor(t: GPtr<u8>, veh: u32, door: u32, line_up: u32) -> GPtr<u8> = thiscall 0x64BE40 => unported;
    /// `CTaskSimpleCarForcePedOut` constructor (0x647710, `ret 8`)
    fn car_force_ped_out_ctor(t: GPtr<u8>, veh: u32, door: u32) -> GPtr<u8> = thiscall 0x647710 => unported;
    /// `CTaskSimpleCarSetPedOut` constructor (0x6478B0, `ret 0xC`)
    fn car_set_ped_out_ctor(t: GPtr<u8>, veh: u32, door: u32, switch_off_engine: u32) -> GPtr<u8> = thiscall 0x6478B0 => unported;
    /// `CTaskSimpleCarSetPedOut::ProcessPed(CPed*)` (0x647D10)
    fn car_set_ped_out_process_ped_orig(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x647D10 => unported;
    /// `CTaskSimpleCarSetPedOut` complete destructor (0x647950)
    fn car_set_ped_out_dtor(t: GPtr<u8>) = thiscall 0x647950 => unported;
    /// `CTaskSimpleCarSetPedOut::PositionPedOutOfCollision(CPed*, CVehicle*, int32 door)` (0x6479B0, cdecl)
    fn car_set_ped_out_position_ped_out_of_collision(ped: GPtr<u8>, veh: GPtr<u8>, door: u32) = cdecl 0x6479B0 => unported;
    /// `CTaskComplexCarSlowBeDraggedOut::Constructor(veh, door, flag)` (0x648490, `ret 0xC`)
    fn car_slow_be_dragged_out_ctor(t: GPtr<u8>, veh: u32, door: u32, flag: u32) -> GPtr<u8> = thiscall 0x648490 => unported;
    /// `CTaskSimpleCarGetOut::ComputeAnimID_ToHook(int32* group, int32* anim)` (0x647330, `ret 8`)
    fn car_get_out_compute_anim_id(t: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) = thiscall 0x647330 => unported;
    /// `CTaskSimpleCarGetOut::StartAnim(CPed*)` (0x64BDE0)
    fn car_get_out_start_anim_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x64BDE0 => unported;
    /// `CTaskSimpleCarJumpOut::ComputeAnimID_ToHook(int32* group, int32* anim)` (0x647690, `ret 8`)
    fn car_jump_out_compute_anim_id_orig(t: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) = thiscall 0x647690 => unported;
    /// `CTaskSimpleCarJumpOut::StartAnim(CPed*)` (0x64BF00)
    fn car_jump_out_start_anim_orig(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x64BF00 => unported;
    /// `CTaskComplexLeaveCar` 0x63BAB0 (`ComputeTargetDoor(CPed*)`, `ret 4`)
    fn leave_car_compute_target_door(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x63BAB0 => unported;
    /// `CTaskUtilityLineUpPedWithCar::ProcessPed(CPed*, CVehicle*, CAnimBlendAssociation*)` (0x6513A0, `ret 0xC`)
    fn line_up_ped_with_car_process_ped(u: GPtr<u8>, ped: GPtr<u8>, veh: GPtr<u8>, anim: GPtr<u8>) = thiscall 0x6513A0 => unported;

    // ---- ped / vehicle / world ----
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::SetPedPositionInCar` (0x5DF910)
    fn ped_set_ped_position_in_car(p: GPtr<u8>) = thiscall 0x5DF910 => unported;
    /// `CPed::UpdateStatLeavingVehicle` (0x5E01B0)
    fn ped_update_stat_leaving_vehicle(p: GPtr<u8>) = thiscall 0x5E01B0 => unported;
    /// `CPed::RestartNonPartialAnims` (0x5DED50)
    fn ped_restart_non_partial_anims(p: GPtr<u8>) = thiscall 0x5DED50 => unported;
    /// `CPed::ReplaceWeaponWhenExitingVehicle` (0x5E6490)
    fn ped_replace_weapon_when_exiting_vehicle(p: GPtr<u8>) = thiscall 0x5E6490 => unported;
    /// `CCarEnterExit::RemoveCarSitAnim(CPed*)` (0x64F680, cdecl)
    fn car_enter_exit_remove_car_sit_anim(p: GPtr<u8>) = cdecl 0x64F680 => unported;
    /// `CCarEnterExit::CarHasDoorToOpen(CVehicle*, int32 door)` (0x64EDD0, cdecl)
    fn car_enter_exit_car_has_door_to_open(v: GPtr<u8>, door: u32) -> u8 = cdecl 0x64EDD0 => unported;
    /// `CVehicle::ClearGettingOutFlags(uint8 doors)` (0x6D3060, `ret 4`)
    fn vehicle_clear_getting_out_flags(v: GPtr<u8>, doors: u32) = thiscall 0x6D3060 => unported;
    /// `CVehicle::RemoveDriver(bool)` (0x6D1950, `ret 4`)
    fn vehicle_remove_driver(v: GPtr<u8>, b: u32) = thiscall 0x6D1950 => unported;
    /// `CVehicle::IsLawEnforcementVehicle` (0x6D2370)
    fn vehicle_is_law_enforcement_vehicle(v: GPtr<u8>) -> u8 = thiscall 0x6D2370 => unported;
    /// `CVehicle::ChangeLawEnforcerState(bool)` (0x6D2330, `ret 4`)
    fn vehicle_change_law_enforcer_state(v: GPtr<u8>, b: u32) = thiscall 0x6D2330 => unported;
    /// `CVehicle::RemovePassenger(CPed*)` (0x6D1610, `ret 4`)
    fn vehicle_remove_passenger(v: GPtr<u8>, p: GPtr<u8>) = thiscall 0x6D1610 => unported;
    /// `CGarages::IsPointWithinAnyGarage(CVector&)` (0x448990, cdecl)
    fn garages_is_point_within_any_garage(p: GPtr<u8>) -> u8 = cdecl 0x448990 => unported;
    /// `CTaskSimplePlayerOnFoot::Constructor` (0x685750)
    fn player_on_foot_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x685750 => unported;
    /// `CTaskSimpleStandStill::Constructor(time, looped, useAnimIdleStance, blend)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, looped: u32, idle: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskComplexWander::GetWanderTaskByPedType(CPed*)` (0x673D00, cdecl)
    fn wander_get_wander_task_by_ped_type(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x673D00 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0, `ret 0xC`)
    fn task_manager_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: u32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CAnimManager::BlendAnimation-id(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0, `ret 8`)
    fn anim_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CEventDamage` 0x4ABCA0 (`HasKilledPed`)
    fn event_damage_has_killed_ped(e: GPtr<u8>) -> u8 = thiscall 0x4ABCA0 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CWorld::ProcessVerticalLine` (0x5674E0, cdecl, 11 arguments)
    fn world_process_vertical_line(pos: GPtr<u8>, z: u32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, s: u32, l: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `CPedPlacement::FindZCoorForPed(CVector*)` (0x616920, cdecl)
    fn ped_placement_find_z_coor_for_ped(p: GPtr<u8>) -> u8 = cdecl 0x616920 => unported;
    /// `CPlaceable::GetHeading` (0x441DB0, ST0)
    fn placeable_get_heading(e: GPtr<u8>) -> f64 = thiscall 0x441DB0 => unported;
    /// `CPlaceable::SetHeading(float)` (0x43E0C0, `ret 4`)
    fn placeable_set_heading(e: GPtr<u8>, h: u32) = thiscall 0x43E0C0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, cdecl, ST0)
    fn general_limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CDamageManager::GetDoorStatus(int32 door)` (0x6C2250, `ret 4`)
    fn damage_manager_get_door_status(d: GPtr<u8>, door: u32) -> u32 = thiscall 0x6C2250 => unported;
    /// `CDamageManager::SetDoorStatus(int32 door, uint32 status)` (0x6C21E0, `ret 8`)
    fn damage_manager_set_door_status(d: GPtr<u8>, door: u32, status: u32) = thiscall 0x6C21E0 => unported;
    /// `CVehicleAnimGroup::GetGroup(int32 animId)` (0x6E3B00, `ret 4`)
    fn vehicle_anim_group_get_group(g: GPtr<u8>, anim: u32) -> u32 = thiscall 0x6E3B00 => unported;
}

/// Pointer to a Rust stack buffer (stands in for an object the original builds on its own stack).
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// Entity position: matrix +0x30, else the placement at +4.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CTaskSimpleCarGetOut::FinishAnimCarGetOutCB(assoc, data)` (0x647480, cdecl): `data->+0xC = 0` (the anim),
/// `data->+8 = 1` (finished).
#[hook(addr = 0x647480, conv = cdecl, name = "CTaskSimpleCarGetOut::FinishAnimCarGetOutCB", expect = "8B 44 24 08 C7 40 0C 00 00 00 00")]
pub fn car_get_out_finish_anim_cb(_assoc: GPtr<u8>, data: GPtr<u8>) {
    wr::<u32>(data, 0xC, 0);
    wr::<u8>(data, 8, 1);
}

/// `CTaskSimpleCarGetOut::SetPedPosition(CPed*)` (0x6472F0): with an anim (+0xC) whose blend amount (+0x18)
/// is not equal to [0x858B50] (`test ah, 0x44; jnp`: NaN counts as not equal) the line-up utility (+0x1C)
/// runs `ProcessPed(ped, veh +0x14, anim)`; otherwise `ped->SetPedPositionInCar()`. Always true.
#[hook(addr = 0x6472F0, conv = thiscall, name = "CTaskSimpleCarGetOut::SetPedPosition", expect = "8B 51 0C 85 D2")]
pub fn car_get_out_set_ped_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let a = rd::<GPtr<u8>>(this, 0xC);
    if !a.is_null() && x(rd::<f32>(a, 0x18)).fcom(x(g::<f32>(0x858B50).read())) != Cmp::Equal {
        line_up_ped_with_car_process_ped(rd::<GPtr<u8>>(this, 0x1C), ped, rd::<GPtr<u8>>(this, 0x14), a);
        return 1;
    }
    ped_set_ped_position_in_car(ped);
    1
}

/// `CTaskSimpleCarGetOut::StartAnim(CPed*)` (0x64BDE0): `RemoveCarSitAnim(ped)`; `ComputeAnimID(&group, &anim)`
/// where `anim` is the original's ped argument slot, so it still holds the ped pointer when the door is
/// none of 8..11 (kept); `BlendAnimation(clump, group, anim, 1000.0)` stored in +0xC with finish callback
/// 0x647480 (data = this).
#[hook(addr = 0x64BDE0, conv = thiscall, name = "CTaskSimpleCarGetOut::StartAnim", expect = "51 56 57 8B 7C 24 10")]
pub fn car_get_out_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    car_enter_exit_remove_car_sit_anim(ped);
    let mut l = [0u32, ped.addr() as u32];
    let p = buf(&mut l);
    car_get_out_compute_anim_id(this, p, p.byte_add(4));
    let (grp, anim) = (rd::<u32>(p, 0), rd::<u32>(p, 4));
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), grp, anim, 0x447A_0000);
    wr::<GPtr<u8>>(this, 0xC, a);
    anim_set_finish_callback(a, 0x647480, this.addr() as u32);
}

/// `CTaskSimpleCarGetOut::ProcessPed(CPed*)` (0x64DC70). No vehicle (+0x14): true. Finished (+8): when the
/// byte [0x969164] is set, the ped is a player and the vehicle exists: vehicle +0x40 loses 0x800000 and
/// 0x40000, +0x42A gains 0x20; true. No anim yet: `StartAnim`, `SetPedPositionInCar`,
/// `ReplaceWeaponWhenExitingVehicle`, +9 = `CarHasDoorToOpen(veh, door +0x18)`; false. Otherwise: blend
/// amount == [0x858624] sets ped flag 0x2000 (+0x46C); with +9 set and +0x10 clear: `ComputeAnimID(&group,
/// &anim)` (group in the ped argument slot, anim in the slot that holds the pushed ECX = `this`, which
/// stays when the door is none of 8..11) and `veh->ProcessOpenDoor(ped, door, group, anim, time)` with the
/// anim id (movsx +0x2C) and time (+0x20) of the playing assoc; false.
#[hook(addr = 0x64DC70, conv = thiscall, name = "CTaskSimpleCarGetOut::ProcessPed", expect = "51 56 8B F1 8B 46 14")]
pub fn car_get_out_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0x14) == 0 {
        return 1;
    }
    if rd::<u8>(this, 8) != 0 {
        if g::<u8>(0x969164).read() != 0 && ped_is_player(ped) != 0 && rd::<u32>(this, 0x14) != 0 {
            let v = rd::<GPtr<u8>>(this, 0x14);
            wr::<u32>(v, 0x40, rd::<u32>(v, 0x40) & 0xFF7F_FFFF);
            let v = rd::<GPtr<u8>>(this, 0x14);
            wr::<u32>(v, 0x40, rd::<u32>(v, 0x40) & 0xFFFB_FFFF);
            let v = rd::<GPtr<u8>>(this, 0x14);
            wr::<u8>(v, 0x42A, rd::<u8>(v, 0x42A) | 0x20);
        }
        return 1;
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    if a.is_null() {
        car_get_out_start_anim_orig(this, ped);
        ped_set_ped_position_in_car(ped);
        ped_replace_weapon_when_exiting_vehicle(ped);
        let r = car_enter_exit_car_has_door_to_open(rd::<GPtr<u8>>(this, 0x14), rd::<u32>(this, 0x18));
        wr::<u8>(this, 9, r);
        return 0;
    }
    if x(rd::<f32>(a, 0x18)).fcom(x(g::<f32>(0x858624).read())) == Cmp::Equal {
        wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 0x2000);
    }
    if rd::<u8>(this, 9) != 0 && rd::<u8>(this, 0x10) == 0 {
        // [0] = group (the ped argument slot), [1] = anim (the slot holding the pushed ECX = this)
        let mut l = [ped.addr() as u32, this.addr() as u32];
        let p = buf(&mut l);
        car_get_out_compute_anim_id(this, p, p.byte_add(4));
        let a = rd::<GPtr<u8>>(this, 0xC);
        let time = rd::<u32>(a, 0x20);
        let id = rd::<i16>(a, 0x2C) as i32 as u32;
        let v = rd::<GPtr<u8>>(this, 0x14);
        v_vehicle_process_open_door(v, ped, rd::<u32>(this, 0x18), rd::<u32>(p, 0), id, time);
    }
    0
}

/// `CTaskSimpleCarJumpOut::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x647510). Priority 2: the
/// anim (+0xC), if any, gets blend delta -1000.0 (+0x1C); true. Priority 1 with an event of type 0x21 or
/// 0x42 (one virtual call, a second for the 0x42 test), an anim and a ped not in a vehicle (bit 8 of
/// +0x46C): blend delta -2.0, +8 = 1 (finished); true. Otherwise false.
#[hook(addr = 0x647510, conv = thiscall, name = "CTaskSimpleCarJumpOut::MakeAbortable", expect = "8B 44 24 08 83 F8 02")]
pub fn car_jump_out_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    if priority == 2 {
        let a = rd::<GPtr<u8>>(this, 0xC);
        if !a.is_null() {
            wr::<u32>(a, 0x1C, 0xC47A_0000);
        }
        return 1;
    }
    if priority != 1 || event.is_null() {
        return 0;
    }
    if v_event_get_type(event) != 0x21 && v_event_get_type(event) != 0x42 {
        return 0;
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    if a.is_null() || rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        return 0;
    }
    wr::<u32>(a, 0x1C, 0xC000_0000);
    wr::<u8>(this, 8, 1);
    1
}

/// `CTaskSimpleCarJumpOut::SetPedPosition(CPed*)` (0x647590). Not in a vehicle: false. Otherwise
/// `SetPedPositionInCar`; with an anim (+0xC): a copy of the ped position goes through
/// `CPedPlacement::FindZCoorForPed` with `CWorld::pIgnoreEntity` [0xB7CD68] = the vehicle (+0x10), reset to
/// null after. Rate = 0.07 (0x3D8F5C29), 0.27 (0x3E8A3D71) for bikes (+0x590 == 9) or +0x594 == 2. For anims
/// 0x180/0x181 when the found z is above the ped's z (strict; NaN skips): matrix z (+0x38, the matrix used
/// without a null check as in the original) = `(time / rate) * (z - mz) + mz`. True.
#[hook(addr = 0x647590, conv = thiscall, name = "CTaskSimpleCarJumpOut::SetPedPosition", expect = "83 EC 0C 56 57")]
pub fn car_jump_out_set_ped_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 {
        return 0;
    }
    ped_set_ped_position_in_car(ped);
    if rd::<u32>(this, 0xC) == 0 {
        return 1;
    }
    let src = ent_pos(ped);
    let mut pos = [rd::<u32>(src, 0), rd::<u32>(src, 4), rd::<u32>(src, 8)];
    let pp = buf(&mut pos);
    g::<u32>(0xB7CD68).write(rd::<u32>(this, 0x10));
    ped_placement_find_z_coor_for_ped(pp);
    g::<u32>(0xB7CD68).write(0);
    let v = rd::<GPtr<u8>>(this, 0x10);
    let rate = if rd::<i32>(v, 0x590) == 9 || rd::<i32>(v, 0x594) == 2 { 0x3E8A_3D71u32 } else { 0x3D8F_5C29 };
    let a = rd::<GPtr<u8>>(this, 0xC);
    if a.is_null() {
        return 1;
    }
    let id = rd::<u16>(a, 0x2C);
    if id != 0x180 && id != 0x181 {
        return 1;
    }
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pz = rd::<f32>(pp, 8);
    if x(pz).fcom(x(rd::<f32>(ent_pos(ped), 8))) != Cmp::Greater {
        return 1;
    }
    let mz = rd::<f32>(m, 0x38);
    let r = x(rd::<f32>(a, 0x20)).fdiv_m(f32::from_bits(rate)).fmul(x(pz).fsub_m(mz)).fadd_m(mz);
    wr::<f32>(m, 0x38, f32_of(r));
    1
}

/// `CTaskSimpleCarJumpOut::ComputeAnimID_ToHook(int32* group, int32* anim)` (0x647690): doors 8/9 ->
/// `*anim = 0x181`, 10/11 -> 0x180 (others leave it); `*group = vehicle (+0x10) anim group.GetGroup(*anim)`.
#[hook(addr = 0x647690, conv = thiscall, name = "CTaskSimpleCarJumpOut::ComputeAnimID_ToHook", expect = "8B 41 18 8B 54 24 08")]
pub fn car_jump_out_compute_anim_id(this: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) {
    match rd::<i32>(this, 0x18).wrapping_sub(8) {
        0 | 1 => wr::<u32>(anim, 0, 0x181),
        2 | 3 => wr::<u32>(anim, 0, 0x180),
        _ => {}
    }
    let v = rd::<GPtr<u8>>(this, 0x10);
    let grp = u32::from(rd::<u8>(rd::<GPtr<u8>>(v, 0x384), 0xDE));
    let r = vehicle_anim_group_get_group(GPtr::from_addr((grp * 0x94).wrapping_add(0xC1CDC0) as usize), rd::<u32>(anim, 0));
    wr::<u32>(group, 0, r);
}

/// `CTaskSimpleCarJumpOut::StartAnim(CPed*)` (0x64BF00): `ComputeAnimID(&group, &anim)` on two
/// uninitialised locals (the anim local stays uninitialised for doors other than 8..11; zeroed here),
/// `BlendAnimation(clump, group, anim, 8.0)` stored in +0xC with finish callback 0x647700 (data = this).
#[hook(addr = 0x64BF00, conv = thiscall, name = "CTaskSimpleCarJumpOut::StartAnim", expect = "83 EC 08 56 8B F1")]
pub fn car_jump_out_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let mut l = [0u32; 2];
    let p = buf(&mut l);
    car_jump_out_compute_anim_id_orig(this, p, p.byte_add(4));
    let (grp, anim) = (rd::<u32>(p, 0), rd::<u32>(p, 4));
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), grp, anim, 0x4100_0000);
    wr::<GPtr<u8>>(this, 0xC, a);
    anim_set_finish_callback(a, 0x647700, this.addr() as u32);
}

/// `CTaskSimpleCarWaitToSlowDown::ProcessPed(CPed*)` (0x646AD0). No vehicle (+8) or mode (+0x10) 2: true.
/// When the vehicle has a non-player driver and the ped is that driver: vehicle +0x3D0 = 0, +0x3BA = 0
/// (mission). Mode 0: `CanPedStepOutCar(false)` or `CanPedJumpOutCar(ped)`; mode 1: `CanPedStepOutCar(false)`
/// (a tail call); other modes false.
#[hook(addr = 0x646AD0, conv = thiscall, name = "CTaskSimpleCarWaitToSlowDown::ProcessPed", expect = "56 8B F1 8B 46 08")]
pub fn car_wait_to_slow_down_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let v = rd::<GPtr<u8>>(this, 8);
    if v.is_null() || rd::<i32>(this, 0x10) == 2 {
        return 1;
    }
    let d = rd::<GPtr<u8>>(v, 0x460);
    if !d.is_null() && ped_is_player(d) == 0 {
        let v = rd::<GPtr<u8>>(this, 8);
        if ped == rd::<GPtr<u8>>(v, 0x460) {
            wr::<u8>(v, 0x3D0, 0);
            wr::<u8>(rd::<GPtr<u8>>(this, 8), 0x3BA, 0);
        }
    }
    match rd::<i32>(this, 0x10) {
        0 => {
            if v_vehicle_can_ped_step_out_car(rd::<GPtr<u8>>(this, 8), 0) != 0 {
                return 1;
            }
            u8::from(v_vehicle_can_ped_jump_out_car(rd::<GPtr<u8>>(this, 8), ped) != 0)
        }
        1 => v_vehicle_can_ped_step_out_car(rd::<GPtr<u8>>(this, 8), 0),
        _ => 0,
    }
}

/// `CTaskSimpleCarSetPedOut::ProcessPed(CPed*)` (0x647D10). Clears ped bit 8 of +0x46C (in vehicle), sets bit 0
/// of +0x1C, `UpdateStatLeavingVehicle`. Unless knocked off a bike (+0x13): `PositionPedOutOfCollision(ped,
/// null, door +0xC)`. `RemoveCarSitAnim`, `RestartNonPartialAnims`. Unless +0x13, +0x12 or the target vehicle
/// (+8) has +0x594 == 5: ped move speed = 0. With a ped vehicle (+0x58C): door flags +0x14 (if any) go to the
/// target vehicle's `ClearGettingOutFlags`, +0x15 is subtracted from its +0x485; the driver is removed
/// (`RemoveDriver(!+0x10)`, entity status +0x36 = (s & 7) | 0x20, +0x4F8 5 -> 1, cop peds (+0x598 == 6) in a
/// law vehicle `ChangeLawEnforcerState(false)`), a passenger `RemovePassenger`; a ped position inside any
/// garage clears vehicle +0x4A8 bits 0x18 and +0x428 bit 0x40. Bikes (+0x590 == 9) when neither +0x12 nor
/// +0x13 and both |move x|, |move y| < 0.1 (f64 [0x86EEE0], strict, NaN skips) set +0x614 bit 0x10. New
/// primary task (slot 4): players `CTaskSimplePlayerOnFoot` (0x1C), mission peds (+0x484 == 2)
/// `CTaskSimpleStandStill(999999, true, false, 8.0)` (0x20), others `GetWanderTaskByPedType`; null when the
/// pool is full. `ReplaceWeaponWhenExitingVehicle`, +0x474 |= 8, ped state +0x530 = 1, and the target
/// vehicle's +0x40 bit 0x8000000 is copied to the ped. True.
// Not registered (PORT-AI-3): split L2 FAIL on every recording (window writes at 0x967F50, CCollision::ProcessColModels
// statics, differ), unresolved; was hooked with `addr = 0x647D10, conv = thiscall, expect = "64 A1 00 00 00 00"`.
pub fn car_set_ped_out_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let f1c = rd::<u32>(ped, 0x1C);
    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FEFF);
    wr::<u32>(ped, 0x1C, f1c | 1);
    ped_update_stat_leaving_vehicle(ped);
    if rd::<u8>(this, 0x13) == 0 {
        car_set_ped_out_position_ped_out_of_collision(ped, GPtr::NULL, rd::<u32>(this, 0xC));
    }
    car_enter_exit_remove_car_sit_anim(ped);
    ped_restart_non_partial_anims(ped);
    if rd::<u8>(this, 0x13) == 0 && rd::<u8>(this, 0x12) == 0 && rd::<i32>(rd::<GPtr<u8>>(this, 8), 0x594) != 5 {
        wr::<u32>(ped, 0x44, 0);
        wr::<u32>(ped, 0x48, 0);
        wr::<u32>(ped, 0x4C, 0);
    }
    if rd::<u32>(ped, 0x58C) != 0 {
        let fl = rd::<u8>(this, 0x14);
        if fl != 0 {
            vehicle_clear_getting_out_flags(rd::<GPtr<u8>>(this, 8), u32::from(fl));
        }
        let n = rd::<u8>(this, 0x15);
        if n != 0 {
            let t = rd::<GPtr<u8>>(this, 8);
            wr::<u8>(t, 0x485, rd::<u8>(t, 0x485).wrapping_sub(n));
        }
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if rd::<GPtr<u8>>(v, 0x460) == ped {
            vehicle_remove_driver(v, u32::from(rd::<u8>(this, 0x10) == 0));
            let v = rd::<GPtr<u8>>(ped, 0x58C);
            wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | 0x20);
            let v = rd::<GPtr<u8>>(ped, 0x58C);
            if rd::<i32>(v, 0x4F8) == 5 {
                wr::<u32>(v, 0x4F8, 1);
            }
            if rd::<i32>(ped, 0x598) == 6 && vehicle_is_law_enforcement_vehicle(rd::<GPtr<u8>>(ped, 0x58C)) != 0 {
                vehicle_change_law_enforcer_state(rd::<GPtr<u8>>(ped, 0x58C), 0);
            }
        } else {
            vehicle_remove_passenger(v, ped);
        }
        let src = ent_pos(ped);
        let mut pos = [rd::<u32>(src, 0), rd::<u32>(src, 4), rd::<u32>(src, 8)];
        if garages_is_point_within_any_garage(buf(&mut pos)) != 0 {
            let v = rd::<GPtr<u8>>(ped, 0x58C);
            wr::<u8>(v, 0x4A8, rd::<u8>(v, 0x4A8) & 0xE7);
            let v = rd::<GPtr<u8>>(ped, 0x58C);
            wr::<u8>(v, 0x428, rd::<u8>(v, 0x428) & 0xBF);
        }
    }
    if rd::<u8>(this, 0x12) == 0 && rd::<u8>(this, 0x13) == 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() && rd::<i32>(v, 0x590) == 9 {
            let lim = g::<f64>(0x86EEE0).read();
            if f64::from(rd::<f32>(v, 0x44)).abs() < lim && f64::from(rd::<f32>(v, 0x48)).abs() < lim {
                wr::<u8>(v, 0x614, rd::<u8>(v, 0x614) | 0x10);
            }
        }
    }
    let tm = |p: GPtr<u8>| rd::<GPtr<u8>>(p, 0x47C).byte_add(4);
    if ped_is_player(ped) != 0 {
        let t = task_new(0x1C);
        let t = if t.is_null() { GPtr::NULL } else { player_on_foot_ctor(t) };
        task_manager_set_task(tm(ped), t, 4, 0);
    } else if rd::<u8>(ped, 0x484) == 2 {
        let t = task_new(0x20);
        let t = if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 0xF423F, 1, 0, 0x4100_0000) };
        task_manager_set_task(tm(ped), t, 4, 0);
    } else {
        let m = tm(ped);
        let t = wander_get_wander_task_by_ped_type(ped);
        task_manager_set_task(m, t, 4, 0);
    }
    ped_replace_weapon_when_exiting_vehicle(ped);
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 8);
    wr::<u32>(ped, 0x530, 1);
    let v = rd::<GPtr<u8>>(this, 8);
    if !v.is_null() && rd::<u32>(v, 0x40) & 0x800_0000 != 0 {
        wr::<u32>(ped, 0x40, rd::<u32>(ped, 0x40) | 0x800_0000);
    }
    1
}

/// `CTaskSimpleCarJumpOut::ProcessPed(CPed*)` (0x64DD60). Finished (+8) or no vehicle (+0x10): true.
/// With an anim (+0xC): when its time (+0x20) is below [0x86F0E8] on a bike (+0x590 == 9) or +0x594 == 2, or
/// below [0x8631D0] on an automobile (+0x590 == 0) (strict; NaN skips), the factor +0x14 is multiplied by
/// `_CIpow([0x86F0E0] f64, timestep)` (stored) and `factor * timestep` is added to ped +0x4DC. Not in a
/// vehicle: automobile vehicles set ped +0x528 bit 8 and +0x474 bit 0x100000; returns "no anim". No anim:
/// `StartAnim`; false. Otherwise the jump threshold is [0x858CA8], [0x858F9C] for bikes / +0x594 == 2,
/// [0x86EC48] for +0x594 == 4. Time above it (strict): a stack `CTaskSimpleCarSetPedOut(veh, door +0x18,
/// true)` with door flags +0x1C / getting-in count +0x1D runs `ProcessPed`; `RemoveCarSitAnim`; heading =
/// vehicle heading (+pi [0x858CB8] through `LimitRadianAngle` when the vehicle up.z (matrix +0x28) is below
/// [0x858B50]) into +0x55C, +0x558 and `SetHeading`; ped move speed = vehicle move speed; ped +0x128 = the
/// vehicle (no reference); +0x470 bit 31; automobiles bump the door damage status by one when it is 0 or
/// 2 (`GetDoorStatus` re-read each time); the stack task is destroyed; false. Otherwise automobiles
/// call `ComputeAnimID(&group, &anim)` (anim in the ped argument slot: the ped pointer stays for doors
/// other than 8..11) and `veh->ProcessOpenDoor(ped, door, group, anim, time)`; false.
#[hook(addr = 0x64DD60, conv = thiscall, name = "CTaskSimpleCarJumpOut::ProcessPed", expect = "64 A1 00 00 00 00")]
pub fn car_jump_out_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 8) != 0 {
        return 1;
    }
    let v = rd::<GPtr<u8>>(this, 0x10);
    if v.is_null() {
        return 1;
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    if !a.is_null() {
        let t = x(rd::<f32>(a, 0x20));
        let bike = rd::<i32>(v, 0x590) == 9 || rd::<i32>(v, 0x594) == 2;
        let apply = (t.fcom(x(g::<f32>(0x86F0E8).read())) == Cmp::Less && bike)
            || (t.fcom(x(g::<f32>(0x8631D0).read())) == Cmp::Less && rd::<i32>(v, 0x590) == 0);
        if apply {
            let ts = g::<f32>(0xB7CB5C).read();
            let p = sa_world::physical::shared::ci_pow(X87::from_f64_exact(g::<f64>(0x86F0E0).read()), x(ts));
            let f = p.fmul_m(rd::<f32>(this, 0x14));
            wr::<f32>(this, 0x14, f32_of(f));
            let r = f.fmul_m(ts).fadd_m(rd::<f32>(ped, 0x4DC));
            wr::<f32>(ped, 0x4DC, f32_of(r));
        }
    }
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 {
        let v = rd::<GPtr<u8>>(this, 0x10);
        if !v.is_null() && rd::<i32>(v, 0x590) == 0 {
            wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) | 8);
            wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x10_0000);
        }
        return u8::from(rd::<u32>(this, 0xC) == 0);
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    if a.is_null() {
        car_jump_out_start_anim_orig(this, ped);
        return 0;
    }
    let v = rd::<GPtr<u8>>(this, 0x10);
    let ty = rd::<i32>(v, 0x590);
    let c = if ty == 9 || rd::<i32>(v, 0x594) == 2 {
        0x858F9C
    } else if rd::<i32>(v, 0x594) == 4 {
        0x86EC48
    } else {
        0x858CA8
    };
    if x(g::<f32>(c).read()).fcom(x(rd::<f32>(a, 0x20))) == Cmp::Less {
        let mut so = [0u32; 6];
        let sp = buf(&mut so);
        car_set_ped_out_ctor(sp, v.addr() as u32, rd::<u32>(this, 0x18), 1);
        wr::<u8>(sp, 0x14, rd::<u8>(this, 0x1C));
        wr::<u8>(sp, 0x15, rd::<u8>(this, 0x1D));
        car_set_ped_out_process_ped_orig(sp, ped);
        car_enter_exit_remove_car_sit_anim(ped);
        let v = rd::<GPtr<u8>>(this, 0x10);
        let upz = x(rd::<f32>(rd::<GPtr<u8>>(v, 0x14), 0x28));
        let h = if upz.fcom(x(g::<f32>(0x858B50).read())) == Cmp::Less {
            let s = X87::from_f64_exact(placeable_get_heading(v)).fadd_m(g::<f32>(0x858CB8).read());
            X87::from_f64_exact(general_limit_radian_angle(f32_of(s).to_bits()))
        } else {
            X87::from_f64_exact(placeable_get_heading(v))
        };
        wr::<f32>(ped, 0x55C, f32_of(h));
        let h = rd::<f32>(ped, 0x55C);
        wr::<f32>(ped, 0x558, h);
        placeable_set_heading(ped, h.to_bits());
        let v = rd::<GPtr<u8>>(this, 0x10);
        for i in 0..3 {
            wr::<u32>(ped, 0x44 + i * 4, rd::<u32>(v, 0x44 + i * 4));
        }
        wr::<u32>(ped, 0x128, rd::<u32>(this, 0x10));
        wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) | 0x8000_0000);
        let v = rd::<GPtr<u8>>(this, 0x10);
        if rd::<i32>(v, 0x590) == 0 {
            let dm = || rd::<GPtr<u8>>(this, 0x10).byte_add(0x5A0);
            let door = rd::<u32>(this, 0x18);
            if damage_manager_get_door_status(dm(), door) == 0 || damage_manager_get_door_status(dm(), rd::<u32>(this, 0x18)) == 2 {
                let s = damage_manager_get_door_status(dm(), rd::<u32>(this, 0x18));
                damage_manager_set_door_status(dm(), rd::<u32>(this, 0x18), s.wrapping_add(1));
            }
        }
        car_set_ped_out_dtor(sp);
        return 0;
    }
    if ty != 0 {
        return 0;
    }
    // [0] = group (a local), [1] = anim (the ped argument slot)
    let mut l = [0u32, ped.addr() as u32];
    let p = buf(&mut l);
    car_jump_out_compute_anim_id_orig(this, p, p.byte_add(4));
    let time = rd::<u32>(rd::<GPtr<u8>>(this, 0xC), 0x20);
    let v = rd::<GPtr<u8>>(this, 0x10);
    v_vehicle_process_open_door(v, ped, rd::<u32>(this, 0x18), rd::<u32>(p, 0), rd::<u32>(p, 4), time);
    0
}

/// `CTaskComplexLeaveCar::CreateSubTask(int32 taskType, CPed*)` (0x641530; the entry is `nop; jmp` to the
/// island 0x40181E, which loads the SEH head and jumps back to 0x641536). Allocates from the task pool and
/// constructs (null when the pool is full; unknown types null): 0xCA `CTaskSimplePause(-1)` (0x18); 0xCE
/// `CTaskComplexGetUpAndStandStill` (0xC); 0xD4 `CTaskSimpleDie(0, +0x24, +0x28, +0x2C)` (0x28); 0x2CC
/// `CTaskComplexLeaveBoat(veh +0xC, 0)` (0x14); 0x2CF `CTaskSimpleCarDriveTimed(veh, +0x14)` (0x70); 0x326
/// `CTaskSimpleCarCloseDoorFromOutside(veh, door +0x10, +0x1C)` (0x1C); 0x329 `CTaskSimpleCarWaitToSlowDown(veh,
/// mode)` (0x14) with mode 2 when +0x1A, else with +0x18 0 for players / 1 for others (`IsPlayer` before the
/// allocation), else 2; 0x32A `CTaskSimpleCarWaitForDoorNotToBeInUse(veh, door, null)` (0x14); 0x32D
/// `CTaskSimpleCarGetOut(veh, door, +0x1C)` (0x20); 0x32E `CTaskSimpleCarJumpOut(veh, door, +0x1C)` (0x24);
/// 0x32F `CTaskSimpleCarForcePedOut(veh, door)` (0x10); 0x330 `CTaskSimpleCarSetPedOut(veh, door, (u8)+0x18)`
/// (0x18); 0x337 `CTaskComplexCarSlowBeDraggedOut(veh, door, true)` (0x1C).
#[hook(addr = 0x641530, conv = thiscall, name = "CTaskComplexLeaveCar::CreateSubTask", expect = "90 E9 E8 02 DC FF")]
pub fn leave_car_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let f = |o: isize| rd::<u32>(this, o);
    let alloc = |size: u32, ctor: &dyn Fn(GPtr<u8>) -> GPtr<u8>| {
        let t = task_new(size);
        if t.is_null() { GPtr::NULL } else { ctor(t) }
    };
    match ty {
        0xCA => alloc(0x18, &|t| pause_ctor(t, -1)),
        0xCE => alloc(0xC, &get_up_and_stand_still_ctor),
        0xD4 => alloc(0x28, &|t| simple_die_ctor1(t, 0, f(0x24), f(0x28), f(0x2C))),
        0x2CC => alloc(0x14, &|t| leave_boat_ctor(t, f(0xC), 0)),
        0x2CF => alloc(0x70, &|t| car_drive_timed_ctor(t, f(0xC), f(0x14))),
        0x326 => alloc(0x1C, &|t| close_door_from_outside_ctor(t, f(0xC), f(0x10), f(0x1C))),
        0x329 => {
            let mode = if rd::<u8>(this, 0x1A) != 0 {
                2
            } else if rd::<u8>(this, 0x18) != 0 {
                if ped_is_player(ped) != 0 { 0 } else { 1 }
            } else {
                2
            };
            alloc(0x14, &|t| wait_to_slow_down_ctor(t, f(0xC), mode))
        }
        0x32A => alloc(0x14, &|t| wait_for_door_ctor(t, f(0xC), f(0x10), 0)),
        0x32D => alloc(0x20, &|t| car_get_out_ctor(t, f(0xC), f(0x10), f(0x1C))),
        0x32E => alloc(0x24, &|t| car_jump_out_ctor(t, f(0xC), f(0x10), f(0x1C))),
        0x32F => alloc(0x10, &|t| car_force_ped_out_ctor(t, f(0xC), f(0x10))),
        0x330 => alloc(0x18, &|t| car_set_ped_out_ctor(t, f(0xC), f(0x10), u32::from(rd::<u8>(this, 0x18)))),
        0x337 => alloc(0x1C, &|t| car_slow_be_dragged_out_ctor(t, f(0xC), f(0x10), 1)),
        _ => GPtr::NULL,
    }
}

/// "ClearFlags" of `CTaskComplexLeaveCar::MakeAbortable`: `veh->ClearGettingOutFlags(+0x20)`, +0x20 = 0,
/// `veh->+0x485 -= +0x21`, +0x21 = 0.
fn leave_car_clear_flags(this: GPtr<u8>) {
    vehicle_clear_getting_out_flags(rd::<GPtr<u8>>(this, 0xC), u32::from(rd::<u8>(this, 0x20)));
    let v = rd::<GPtr<u8>>(this, 0xC);
    let n = rd::<u8>(this, 0x21);
    wr::<u8>(this, 0x20, 0);
    wr::<u8>(v, 0x485, rd::<u8>(v, 0x485).wrapping_sub(n));
    wr::<u8>(this, 0x21, 0);
}

/// `CTaskComplexLeaveCar::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x641100). No vehicle (+0xC):
/// true. Priority 2: the sub-task is asked (result ignored); a ped still in the vehicle gets
/// `ComputeTargetDoor` when the door (+0x10) is 0, a stack `CTaskSimpleCarCloseDoorFromOutside(veh, door,
/// null)` asked to abort (priority 2), then a stack `CTaskSimpleCarSetPedOut(veh, door, (u8)+0x18)` runs
/// `ProcessPed`; both are destroyed (set-ped-out first); ClearFlags; true. Priority 1 (sub-task type
/// re-queried for each test): 0xD4 (a dead re-test against 0x2CF), or 0x329 / 0xCE with a damage event (type
/// 9) that `HasKilledPed`: the sub-task's answer, ClearFlags when true. 0x32E with an event of type 0x21 / 0x42:
/// type 0x21 sets +0x30; when the sub-task aborts: parachute weapon (0x2E in the current slot) + type 0x21 +
/// no contact wheels: planes (+0x594 == 4) moving faster than [0x858CC4] blend anim 0x7D (group 0) at 8.0,
/// otherwise non-players stop, players blend at 8.0 unless `ProcessVerticalLine(pos, -10.0, ...)` hits;
/// else type 0x42 with move z < [0x858EE4] blends at 16.0; false in every case. A remaining event with sub-task
/// 0x2CF returns the sub-task's answer (no ClearFlags). Everything else (also priority 0) false.
#[hook(addr = 0x641100, conv = thiscall, name = "CTaskComplexLeaveCar::MakeAbortable", expect = "64 A1 00 00 00 00")]
pub fn leave_car_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0xC) == 0 {
        return 1;
    }
    let sub = || rd::<GPtr<u8>>(this, 8);
    if priority == 2 {
        v_make_abortable(sub(), ped, 2, event);
        if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
            if rd::<u32>(this, 0x10) == 0 {
                leave_car_compute_target_door(this, ped);
            }
            let mut cd = [0u32; 7];
            let cp = buf(&mut cd);
            close_door_from_outside_ctor(cp, rd::<u32>(this, 0xC), rd::<u32>(this, 0x10), 0);
            close_door_from_outside_make_abortable(cp, ped, 2, event);
            let mut so = [0u32; 6];
            let sp = buf(&mut so);
            car_set_ped_out_ctor(sp, rd::<u32>(this, 0xC), rd::<u32>(this, 0x10), u32::from(rd::<u8>(this, 0x18)));
            car_set_ped_out_process_ped_orig(sp, ped);
            car_set_ped_out_dtor(sp);
            close_door_from_outside_dtor(cp);
        }
        leave_car_clear_flags(this);
        return 1;
    }
    if priority != 1 {
        return 0;
    }
    let abort_and_clear = || {
        let r = v_make_abortable(sub(), ped, 1, event);
        if r != 0 {
            leave_car_clear_flags(this);
        }
        r
    };
    if v_get_task_type(sub()) == 0xD4 {
        if v_get_task_type(sub()) == 0x2CF {
            return 0;
        }
        return abort_and_clear();
    }
    let killed = || !event.is_null() && v_event_get_type(event) == 9 && event_damage_has_killed_ped(event) != 0;
    if v_get_task_type(sub()) == 0x329 && killed() {
        return abort_and_clear();
    }
    if v_get_task_type(sub()) == 0xCE && killed() {
        return abort_and_clear();
    }
    if v_get_task_type(sub()) == 0x32E {
        if event.is_null() {
            return 0;
        }
        if v_event_get_type(event) == 0x21 || v_event_get_type(event) == 0x42 {
            if v_event_get_type(event) == 0x21 {
                wr::<u8>(this, 0x30, 1);
            }
            if v_make_abortable(sub(), ped, 1, event) == 0 {
                return 0;
            }
            let slot = rd::<i8>(ped, 0x718) as i32;
            let wt = rd::<i32>(ped, (slot * 0x1C) as isize + 0x5A0);
            let speed = if wt == 0x2E
                && v_event_get_type(event) == 0x21
                && v_vehicle_get_num_contact_wheels(rd::<GPtr<u8>>(this, 0xC)) == 0
            {
                let v = rd::<GPtr<u8>>(this, 0xC);
                let fast = rd::<i32>(v, 0x594) == 4
                    && X87::from_f64_exact(vector_magnitude(v.byte_add(0x44))).fcom(x(g::<f32>(0x858CC4).read())) == Cmp::Greater;
                if fast {
                    0x4100_0000
                } else {
                    if ped_is_player(ped) == 0 {
                        return 0;
                    }
                    let pos = ent_pos(ped);
                    let mut cpb = [0u32; 0xB];
                    // the entity out-pointer is the original's priority argument slot (holding 1)
                    let mut ent = [1u32];
                    let hit = world_process_vertical_line(pos, 0xC120_0000, buf(&mut cpb), buf(&mut ent), 1, 0, 0, 0, 1, 0, 0);
                    if hit != 0 {
                        return 0;
                    }
                    0x4100_0000
                }
            } else {
                if v_event_get_type(event) != 0x42 {
                    return 0;
                }
                if x(rd::<f32>(ped, 0x4C)).fcom(x(g::<f32>(0x858EE4).read())) != Cmp::Less {
                    return 0;
                }
                0x4180_0000
            };
            anim_blend_animation(rd::<u32>(ped, 0x18), 0, 0x7D, speed);
            return 0;
        }
    }
    if event.is_null() || v_get_task_type(sub()) != 0x2CF {
        return 0;
    }
    v_make_abortable(sub(), ped, 1, event)
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_CAR_GET_OUT_FINISH_ANIM_CB,
    HOOK_CAR_GET_OUT_SET_PED_POSITION,
    HOOK_CAR_GET_OUT_START_ANIM,
    HOOK_CAR_GET_OUT_PROCESS_PED,
    HOOK_CAR_JUMP_OUT_MAKE_ABORTABLE,
    HOOK_CAR_JUMP_OUT_SET_PED_POSITION,
    HOOK_CAR_JUMP_OUT_COMPUTE_ANIM_ID,
    HOOK_CAR_JUMP_OUT_START_ANIM,
    HOOK_CAR_WAIT_TO_SLOW_DOWN_PROCESS_PED,
    HOOK_CAR_JUMP_OUT_PROCESS_PED,
    HOOK_LEAVE_CAR_CREATE_SUB_TASK,
    HOOK_LEAVE_CAR_MAKE_ABORTABLE,
    HOOK_LEAVE_CAR_CREATE_NEXT_SUB_TASK,
];

orig! {
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020, `ret 4`)
    fn matrix_set_rotate_z_only(m: GPtr<u8>, h: u32) = thiscall 0x59B020 => unported;
    /// `CEntity` vtable +0x34 (the collision test used by `PositionPedOutOfCollision`, one bool argument)
    fn v_entity_test_collision(e: GPtr<u8>, b: u32) -> u8 = thiscall vslot 0x34 => unported;
    /// `CPed::PositionPedOutOfCollision(int32 door, CVehicle*, bool)` (0x5E0820, `ret 0xC`)
    fn ped_position_ped_out_of_collision(p: GPtr<u8>, door: u32, veh: GPtr<u8>, b: u32) -> u8 = thiscall 0x5E0820 => unported;
    /// `operator*(float, const CVector&)` (0x40FE90, cdecl, returns into `out`)
    fn vector_scale(out: GPtr<u8>, f: u32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00, `ret 4`)
    fn vector_add_assign(v: GPtr<u8>, o: GPtr<u8>) = thiscall 0x411A00 => unported;
    /// `CEntity::RegisterReference(CEntity** ref)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CWorld::TestSphereAgainstWorld(CVector centre (by value), float radius, CEntity* ignore, bool b, v, p, o, d, cam)` (0x569E20, cdecl)
    fn world_test_sphere_against_world(cx: u32, cy: u32, cz: u32, r: u32, ignore: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, c: u32) -> GPtr<u8> = cdecl 0x569E20 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool b, v, p, o, d, see, cam)` (0x56A490, cdecl)
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bb: u32, v: u32, p: u32, o: u32, d: u32, s: u32, c: u32) -> u8 = cdecl 0x56A490 => unported;
}

/// `CTaskSimpleCarSetPedOut::PositionPedOutOfCollision(CPed*, CVehicle*, int32 door)` (0x6479B0, cdecl).
/// Vehicle = the argument or the ped's (+0x58C); none: return. Heading `h` = `atan2(-fwd.x, fwd.y)` of the
/// vehicle matrix (stored f32; +0x10 without a matrix) into ped +0x55C, +0x558 and `SetRotateZOnly` (+0x10
/// without a ped matrix). Boats (+0x590 == 5): handling +0xCC bit 0x400 adds `up * [0x858B8C]` to the ped
/// matrix position (both matrices used without null checks); when the ped collides (vtable +0x34, arg 0):
/// with vehicle +0x42B bit 0x40 the ped is lowered by [0x858C24], and if it still collides raised again and
/// `CPed::PositionPedOutOfCollision(door, veh, true)`; otherwise it is moved back by `fwd * [0x858C24]` and
/// if it still collides moved forward by `0.3 * fwd` (`operator*` + `operator+=`) and
/// `PositionPedOutOfCollision(door, veh, true)`; not colliding (also after the move back): ped move speed =
/// vehicle move speed * [0x858C20], z -= [0x858B1C], +0x46C bit 0, and +0x568 = the vehicle (registered)
/// when empty. Other vehicles: `CWorld::pIgnoreEntity` [0xB7CD68] = veh; two `TestSphereAgainstWorld`
/// probes (radius 0.4) at the ped position with z -/+ [0x858CC4] count a hit other than vehicle +0xFC; a
/// blocked `GetIsLineOfSightClear(vehPos, pedPos, ...)` or a hit calls `PositionPedOutOfCollision(door,
/// veh, true)`; pIgnoreEntity = null.
// Not registered (PORT-AI-3): split L2 FAIL on every recording (window writes at 0x967F50, CCollision::ProcessColModels
// statics, differ), unresolved; was hooked with `addr = 0x6479B0, conv = cdecl, expect = "83 EC 0C 53 56"`.
pub fn car_set_ped_out_position_ped_out_of_collision_port(ped: GPtr<u8>, veh: GPtr<u8>, door: u32) {
    let veh = if veh.is_null() { rd::<GPtr<u8>>(ped, 0x58C) } else { veh };
    if veh.is_null() {
        return;
    }
    let vm = rd::<GPtr<u8>>(veh, 0x14);
    let h = if vm.is_null() {
        rd::<u32>(veh, 0x10)
    } else {
        f32_of(x(rd::<f32>(vm, 0x10)).fchs().fpatan(x(rd::<f32>(vm, 0x14)))).to_bits()
    };
    wr::<u32>(ped, 0x55C, h);
    wr::<u32>(ped, 0x558, h);
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    if pm.is_null() {
        wr::<u32>(ped, 0x10, h);
    } else {
        matrix_set_rotate_z_only(pm, h);
    }
    if rd::<i32>(veh, 0x590) == 5 {
        let c5 = g::<f32>(0x858B8C).read();
        let c3 = g::<f32>(0x858C24).read();
        if rd::<u32>(rd::<GPtr<u8>>(veh, 0x384), 0xCC) & 0x400 != 0 {
            let vm = rd::<GPtr<u8>>(veh, 0x14);
            let a = x(rd::<f32>(vm, 0x20)).fmul_m(c5);
            let b = x(rd::<f32>(vm, 0x24)).fmul_m(c5);
            let c = f32_of(x(rd::<f32>(vm, 0x28)).fmul_m(c5));
            let pm = rd::<GPtr<u8>>(ped, 0x14);
            wr::<f32>(pm, 0x30, f32_of(a.fadd_m(rd::<f32>(pm, 0x30))));
            wr::<f32>(pm, 0x34, f32_of(b.fadd_m(rd::<f32>(pm, 0x34))));
            wr::<f32>(pm, 0x38, f32_of(x(c).fadd_m(rd::<f32>(pm, 0x38))));
        }
        if v_entity_test_collision(ped, 0) != 0 {
            let pm = rd::<GPtr<u8>>(ped, 0x14);
            if rd::<u8>(veh, 0x42B) & 0x40 != 0 {
                wr::<f32>(pm, 0x38, f32_of(x(rd::<f32>(pm, 0x38)).fsub_m(c3)));
                if v_entity_test_collision(ped, 0) == 0 {
                    return;
                }
                let pm = rd::<GPtr<u8>>(ped, 0x14);
                wr::<f32>(pm, 0x38, f32_of(x(rd::<f32>(pm, 0x38)).fadd_m(c3)));
                ped_position_ped_out_of_collision(ped, door, veh, 1);
                return;
            }
            let a = x(rd::<f32>(pm, 0x10)).fmul_m(c3);
            let b = x(rd::<f32>(pm, 0x14)).fmul_m(c3);
            let c = f32_of(x(rd::<f32>(pm, 0x18)).fmul_m(c3));
            wr::<f32>(pm, 0x30, f32_of(x(rd::<f32>(pm, 0x30)).fsub(a)));
            wr::<f32>(pm, 0x34, f32_of(x(rd::<f32>(pm, 0x34)).fsub(b)));
            wr::<f32>(pm, 0x38, f32_of(x(rd::<f32>(pm, 0x38)).fsub_m(c)));
            if v_entity_test_collision(ped, 0) != 0 {
                let m = rd::<GPtr<u8>>(ped, 0x14);
                let mut t = [0u32; 3];
                let r = vector_scale(buf(&mut t), 0x3E99_999A, m.byte_add(0x10));
                vector_add_assign(m.byte_add(0x30), r);
                ped_position_ped_out_of_collision(ped, door, veh, 1);
                return;
            }
        }
        let c9 = g::<f32>(0x858C20).read();
        let mx = f32_of(x(rd::<f32>(veh, 0x44)).fmul_m(c9));
        let my = f32_of(x(rd::<f32>(veh, 0x48)).fmul_m(c9));
        let mz = f32_of(x(rd::<f32>(veh, 0x4C)).fmul_m(c9));
        wr::<f32>(ped, 0x44, mx);
        wr::<f32>(ped, 0x48, my);
        wr::<f32>(ped, 0x4C, mz);
        wr::<f32>(ped, 0x4C, f32_of(x(rd::<f32>(ped, 0x4C)).fsub_m(g::<f32>(0x858B1C).read())));
        wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 1);
        if rd::<u32>(ped, 0x568) == 0 {
            wr::<GPtr<u8>>(ped, 0x568, veh);
            entity_register_reference(veh, ped.byte_add(0x568));
        }
        return;
    }
    g::<u32>(0xB7CD68).write(veh.addr() as u32);
    let mut hit = false;
    let dz = g::<f32>(0x858CC4).read();
    for up in [false, true] {
        let p = ent_pos(ped);
        let y = f32_of(x(rd::<f32>(p, 4))).to_bits();
        let z = x(rd::<f32>(p, 8));
        let z = f32_of(if up { z.fadd_m(dz) } else { z.fsub_m(dz) }).to_bits();
        let e = world_test_sphere_against_world(rd::<u32>(p, 0), y, z, 0x3ECC_CCCD, veh, 1, 1, 0, 0, 0, 0);
        if !e.is_null() && e != rd::<GPtr<u8>>(veh, 0xFC) {
            hit = true;
        }
    }
    let pp = ent_pos(ped);
    let vp = ent_pos(veh);
    if world_get_is_line_of_sight_clear(vp, pp, 1, 0, 0, 1, 0, 0, 0) == 0 || hit {
        ped_position_ped_out_of_collision(ped, door, veh, 1);
    }
    g::<u32>(0xB7CD68).write(0);
}

orig! {
    /// `CTaskComplexLeaveCar::CreateSubTask(int32 taskType, CPed*)` (0x641530, `ret 8`)
    fn leave_car_create_sub_task_orig(t: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x641530 => unported;
    /// `CVehicle::IsPassenger(CPed*)` (0x6D1BD0, `ret 4`)
    fn vehicle_is_passenger(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6D1BD0 => unported;
    /// `CVehicle::IsDriver(CPed*)` (0x6D1C40, `ret 4`)
    fn vehicle_is_driver(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6D1C40 => unported;
    /// 0x646CF0 (cdecl `(CVehicle*, int32 door, bool* in, bool* out)`: the door-in-use check of the leave-car task)
    fn car_door_in_use_646cf0(v: GPtr<u8>, door: u32, a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x646CF0 => unported;
    /// `CTaskComplexLeaveCar` 0x63BA00 (`SetVehicleFlags(CPed*)`, `ret 4`)
    fn leave_car_set_vehicle_flags(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x63BA00 => unported;
    /// `CTaskComplexLeaveCar` 0x63BAE0 (`CreateLineUpTask(CPed*)`, `ret 4`)
    fn leave_car_create_line_up_task(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x63BAE0 => unported;
    /// `CPed::SetPedState(int32)` (0x5E4500, `ret 4`)
    fn ped_set_ped_state(p: GPtr<u8>, s: u32) = thiscall 0x5E4500 => unported;
    /// `CCarEnterExit::IsRoomForPedToLeaveCar(CVehicle*, int32 door, CVector*)` (0x6504C0, cdecl)
    fn car_enter_exit_is_room_for_ped_to_leave_car(v: GPtr<u8>, door: u32, pos: GPtr<u8>) -> u8 = cdecl 0x6504C0 => unported;
    /// `CCarEnterExit::CarHasDoorToClose(CVehicle*, int32 door)` (0x64EE10, cdecl)
    fn car_enter_exit_car_has_door_to_close(v: GPtr<u8>, door: u32) -> u8 = cdecl 0x64EE10 => unported;
}

/// `CTaskComplexLeaveCar::CreateNextSubTask(CPed*)` (0x6419F0). `C(x)` = `CreateSubTask(x, ped)` through the
/// original address (0x516 gives null). By the sub-task type (queried once): 0xCA, 0xD4 -> C(0x516); 0xCE ->
/// in a vehicle C(0x330) else C(0x516); 0x2CC -> +0x1A (die) ? C(0xD4) : C(0x516); 0x326 -> +0x24 = 0xF,
/// C(0x330); 0x2CF -> not in a vehicle C(0x516); neither passenger nor driver: clear bit 8 of +0x46C, C(0x516);
/// else `ComputeTargetDoor`, the door check 0x646CF0(veh, door, &in, &out) (in = the high byte of the slot
/// holding the pushed ECX, out = the low byte of the ped argument slot, both zeroed first) and unless +0x19
/// (force): out ? C(0x32A) : in ? C(0x516) : C(0x329); with force C(0x329). 0x329: the vehicle cannot step out
/// (`CanPedStepOutCar(false)`) -> flags, state 0, C(0x32E); room at the door -> OUT; else an alternative door
/// (8 -> 10, 9 -> 11, 10 -> 8, 11 -> 9, others 0; occupied seats or getting-in bits +0x486 force out, bikes
/// and handling bit 9 skip that test for 10/11), no room there or (handling bit 15, automobile, no car node
/// +0x648) -> FORCE, else door = alt and OUT. OUT: flags, state 0, +0x1A ? C(0x337) : line-up + C(0x32D).
/// FORCE: flags, state 0, line-up, C(0x32F). 0x32A -> C(0x329). 0x32D -> line-up +0x14 = 2; unless forced
/// with vehicle +0x4F8 == 1, a door to close gives C(0x326); else automobiles with a door to close bump the
/// door status when it is 0 or 2; C(0x330). 0x32E -> die: +0x24 = 0xF, +0x28 = 1000.0, +0x2C = 0.5, C(0xD4);
/// no vehicle / bike / +0x594 == 2: C(0x516); in air (+0x30) or drowning (+0x470 bit 0x80000): C(0x330); else
/// ClearFlags and C(0xCE). 0x32F -> C(0x330). 0x330 -> die ? C(0xD4) : C(0x516). 0x337 -> +0x24 = 0x24,
/// +0x28 = 1000.0, +0x2C = 0.5, vehicle +0x4F8 5 -> 1, C(0xD4). Other types: null.
#[hook(addr = 0x6419F0, conv = thiscall, name = "CTaskComplexLeaveCar::CreateNextSubTask", expect = "51 53 56 8B F1")]
pub fn leave_car_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let c = |ty: i32| leave_car_create_sub_task_orig(this, ty, ped);
    let veh = || rd::<GPtr<u8>>(this, 0xC);
    let in_veh = || rd::<u32>(ped, 0x46C) & 0x100 != 0;
    let die = || rd::<u8>(this, 0x1A) != 0;
    let force = |this: GPtr<u8>| {
        leave_car_set_vehicle_flags(this, ped);
        ped_set_ped_state(ped, 0);
        leave_car_create_line_up_task(this, ped);
        leave_car_create_sub_task_orig(this, 0x32F, ped)
    };
    let door_status_bump = |this: GPtr<u8>| {
        let dm = || rd::<GPtr<u8>>(this, 0xC).byte_add(0x5A0);
        if damage_manager_get_door_status(dm(), rd::<u32>(this, 0x10)) == 0
            || damage_manager_get_door_status(dm(), rd::<u32>(this, 0x10)) == 2
        {
            let s = damage_manager_get_door_status(dm(), rd::<u32>(this, 0x10));
            damage_manager_set_door_status(dm(), rd::<u32>(this, 0x10), s.wrapping_add(1));
        }
    };
    match v_get_task_type(rd::<GPtr<u8>>(this, 8)) {
        0xCA | 0xD4 => c(0x516),
        0xCE => c(if in_veh() { 0x330 } else { 0x516 }),
        0x2CC => c(if die() { 0xD4 } else { 0x516 }),
        0x326 => {
            wr::<u32>(this, 0x24, 0xF);
            c(0x330)
        }
        0x2CF => {
            if !in_veh() {
                return c(0x516);
            }
            if vehicle_is_passenger(rd::<GPtr<u8>>(ped, 0x58C), ped) == 0 && vehicle_is_driver(rd::<GPtr<u8>>(ped, 0x58C), ped) == 0 {
                wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FEFF);
                return c(0x516);
            }
            leave_car_compute_target_door(this, ped);
            // [0] = the pushed-ECX slot (its byte 3 is `in`), [1] = the ped argument slot (its byte 0 is `out`)
            let mut l = [(this.addr() as u32) & 0x00FF_FFFF, (ped.addr() as u32) & 0xFFFF_FF00];
            let p = buf(&mut l);
            car_door_in_use_646cf0(veh(), rd::<u32>(this, 0x10), p.byte_add(3), p.byte_add(4));
            if rd::<u8>(this, 0x19) != 0 {
                return c(0x329);
            }
            if rd::<u8>(p, 4) != 0 {
                return c(0x32A);
            }
            c(if rd::<u8>(p, 3) != 0 { 0x516 } else { 0x329 })
        }
        0x329 => {
            if v_vehicle_can_ped_step_out_car(veh(), 0) == 0 {
                leave_car_set_vehicle_flags(this, ped);
                ped_set_ped_state(ped, 0);
                return c(0x32E);
            }
            if car_enter_exit_is_room_for_ped_to_leave_car(veh(), rd::<u32>(this, 0x10), GPtr::NULL) == 0 {
                let v = veh();
                let bike_or_bit9 = || rd::<i32>(v, 0x590) == 9 || (rd::<u32>(rd::<GPtr<u8>>(v, 0x384), 0xCC) >> 9) & 1 != 0;
                let busy = |seat: isize, bit: u8| rd::<u32>(v, seat) != 0 || rd::<u8>(v, 0x486) & bit != 0;
                let alt: u32 = match rd::<i32>(this, 0x10) {
                    8 => {
                        if busy(0x460, 1) {
                            return force(this);
                        }
                        10
                    }
                    9 => {
                        if busy(0x468, 2) {
                            return force(this);
                        }
                        11
                    }
                    10 => {
                        if !bike_or_bit9() && busy(0x464, 4) {
                            return force(this);
                        }
                        8
                    }
                    11 => {
                        if !bike_or_bit9() && busy(0x46C, 8) {
                            return force(this);
                        }
                        9
                    }
                    _ => 0,
                };
                if car_enter_exit_is_room_for_ped_to_leave_car(veh(), alt, GPtr::NULL) == 0 {
                    return force(this);
                }
                let v = veh();
                if rd::<u32>(rd::<GPtr<u8>>(v, 0x384), 0xCC) & 0x8000 != 0
                    && rd::<u32>(v, 0x590) == 0
                    && rd::<u32>(v, 0x648 + alt as isize * 4) == 0
                {
                    return force(this);
                }
                wr::<u32>(this, 0x10, alt);
            }
            leave_car_set_vehicle_flags(this, ped);
            ped_set_ped_state(ped, 0);
            if die() {
                return c(0x337);
            }
            leave_car_create_line_up_task(this, ped);
            c(0x32D)
        }
        0x32A => c(0x329),
        0x32D => {
            wr::<u32>(rd::<GPtr<u8>>(this, 0x1C), 0x14, 2);
            if (rd::<u8>(this, 0x19) == 0 || rd::<i32>(veh(), 0x4F8) != 1)
                && car_enter_exit_car_has_door_to_close(veh(), rd::<u32>(this, 0x10)) != 0
            {
                return c(0x326);
            }
            if rd::<u32>(veh(), 0x590) == 0 && car_enter_exit_car_has_door_to_close(veh(), rd::<u32>(this, 0x10)) != 0 {
                door_status_bump(this);
            }
            c(0x330)
        }
        0x32E => {
            if die() {
                wr::<u32>(this, 0x24, 0xF);
                wr::<u32>(this, 0x28, 0x447A_0000);
                wr::<u32>(this, 0x2C, 0x3F00_0000);
                return c(0xD4);
            }
            let v = veh();
            if v.is_null() || rd::<i32>(v, 0x590) == 9 || rd::<i32>(v, 0x594) == 2 {
                return c(0x516);
            }
            if rd::<u8>(this, 0x30) != 0 || rd::<u32>(ped, 0x470) & 0x8_0000 != 0 {
                return c(0x330);
            }
            leave_car_clear_flags(this);
            c(0xCE)
        }
        0x32F => c(0x330),
        0x330 => c(if die() { 0xD4 } else { 0x516 }),
        0x337 => {
            wr::<u32>(this, 0x24, 0x24);
            wr::<u32>(this, 0x28, 0x447A_0000);
            wr::<u32>(this, 0x2C, 0x3F00_0000);
            let v = veh();
            if !v.is_null() && rd::<i32>(v, 0x4F8) == 5 {
                wr::<u32>(v, 0x4F8, 1);
            }
            c(0xD4)
        }
        _ => GPtr::NULL,
    }
}
