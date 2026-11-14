//! Car-jacking ("slow" drag-out) tasks: the jacker's `CTaskSimpleCarSlowDragPedOut` and the victim's
//! `CTaskComplexCarSlowBeDraggedOutAndStandUp` → `CTaskComplexCarSlowBeDraggedOut` → `CTaskSimpleCarSlowBeDraggedOut` /
//! `CTaskSimpleCarSetPedSlowDraggedOut`, plus `SayJacking` (batch `carjack-1`). Ported from the 1.0 US bytes
//! (gta-reversed as a reading aid; the bytes win where they differ, noted per function). Unverified
//! (docs/verify/HOWTO.md). Field offsets:
//! - `CTaskSimpleCarSlowDragPedOut` (0x20): +8 anim finished, +0xC assoc, +0x10 vehicle, +0x14 door, +0x18 line-up.
//! - `CTaskSimpleCarSlowBeDraggedOut`: +8 finished, +0xC assoc, +0x10 vehicle, +0x14 door, +0x18 open-door flag,
//!   +0x19 anim flag (assoc flag 0x800), +0x1C line-up.
//! - `CTaskSimpleCarSetPedSlowDraggedOut` (0x10): +8 vehicle, +0xC door.
//! - `CTaskComplexCarSlowBeDraggedOut`: +8 sub-task, +0xC vehicle, +0x10 door, +0x14 dragged by self, +0x18 line-up.
//! - `CTaskComplexCarSlowBeDraggedOutAndStandUp`: +8 sub-task, +0xC vehicle, +0x10 door.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::*;

orig! {
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CVehicle::ProcessOpenDoor(CPed*, uint32 door, uint32 group, uint32 anim, float time)` (vtable +0x70)
    fn v_vehicle_process_open_door(v: GPtr<u8>, ped: GPtr<u8>, door: u32, group: u32, anim: u32, time: u32) = thiscall vslot 0x70 => unported;
    /// `CPed::SetMoveAnim` (vtable +0x5C)
    fn v_ped_set_move_anim(p: GPtr<u8>) = thiscall vslot 0x5C => unported;

    /// `CVehicleAnimGroup::GetGroup(int32 animId)` (0x6E3B00, `ret 4`)
    fn vehicle_anim_group_get_group(g: GPtr<u8>, anim: u32) -> u32 = thiscall 0x6E3B00 => unported;
    /// `CVehicleAnimGroup::ComputeAnimDoorOffsets(CVector* out, int32 door)` (0x6E3D10, `ret 8`)
    fn vehicle_anim_group_compute_anim_door_offsets(g: GPtr<u8>, out: GPtr<u8>, door: u32) -> GPtr<u8> = thiscall 0x6E3D10 => unported;
    /// `CDamageManager::GetDoorStatus(int32 door)` (0x6C2250, `ret 4`)
    fn damage_manager_get_door_status(d: GPtr<u8>, door: u32) -> u32 = thiscall 0x6C2250 => unported;
    /// `CDamageManager::SetDoorStatus(int32 door, uint32 status)` (0x6C21E0, `ret 8`)
    fn damage_manager_set_door_status(d: GPtr<u8>, door: u32, status: u32) = thiscall 0x6C21E0 => unported;
    /// `CTaskSimpleCarSlowDragPedOut::StartAnim(const CPed*)` (0x64C010, `ret 4`)
    fn slow_drag_ped_out_start_anim(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x64C010 => unported;
    /// `CTaskSimpleCarSlowDragPedOut::ComputeAnimID_Wrapper(int32* group, int32* anim)` (0x648100, `ret 8`)
    fn slow_drag_ped_out_compute_anim_id_o(t: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) = thiscall 0x648100 => slow_drag_ped_out_compute_anim_id;
    /// `CTaskSimpleCarSlowBeDraggedOut::ComputeAnimID(int32* group, int32* anim)` (0x648330, `ret 8`)
    fn slow_be_dragged_out_compute_anim_id(t: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) = thiscall 0x648330 => unported;
    /// `CTaskSimpleCarSlowBeDraggedOut::StartAnim(const CPed*)` (0x64C190, `ret 4`)
    fn slow_be_dragged_out_start_anim_o(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x64C190 => slow_be_dragged_out_start_anim;
    /// `CCarEnterExit::ComputePassengerIndexFromCarDoor(const CVehicle*, int32)` (0x64F1E0, cdecl)
    fn compute_passenger_index_from_car_door(v: GPtr<u8>, door: u32) -> i32 = cdecl 0x64F1E0 => unported;
    /// `CCarEnterExit::RemoveCarSitAnim(const CPed*)` (0x64F680, cdecl)
    fn car_enter_exit_remove_car_sit_anim(p: GPtr<u8>) = cdecl 0x64F680 => unported;
    /// `CCarEnterExit::ComputeDoorFlag(CVehicle*, int32 door, bool)` (0x64E550, cdecl)
    fn car_enter_exit_compute_door_flag(v: GPtr<u8>, door: u32, b: u32) -> u32 = cdecl 0x64E550 => unported;
    /// `CCarEnterExit::IsRoomForPedToLeaveCar(const CVehicle*, int32 door, CVector* offset)` (0x6504C0, cdecl)
    fn car_enter_exit_is_room_for_ped_to_leave_car(v: GPtr<u8>, door: u32, offset: GPtr<u8>) -> u8 = cdecl 0x6504C0 => unported;
    /// `SayJacked(CPed* jacked, CVehicle*, uint32 offset)` (0x6497A0, cdecl)
    fn say_jacked(jacked: GPtr<u8>, v: GPtr<u8>, offset: u32) -> u8 = cdecl 0x6497A0 => unported;
    /// `SayJacking(CPed* jacker, CPed* jacked, CVehicle*, uint32 offset)` (0x6497F0, cdecl)
    fn say_jacking_o(jacker: GPtr<u8>, jacked: GPtr<u8>, v: GPtr<u8>, offset: u32) -> u8 = cdecl 0x6497F0 => say_jacking;
    /// `CAEVehicleAudioEntity::GetVehicleTypeForAudio` (0x4F4F00): a 16-bit result
    fn vehicle_audio_get_vehicle_type_for_audio(a: GPtr<u8>) -> u32 = thiscall 0x4F4F00 => unported;
    /// `CPed::Say(uint16 id, uint32 offset, float, uint8, uint8, uint8)` (0x5EFFE0, `ret 0x18`): a 16-bit result
    fn ped_say(p: GPtr<u8>, id: u32, offset: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CAEPedSpeechAudioEntity` vtable +0x1C (the jacked ped's speech entity, a bool)
    fn v_speech_1c(s: GPtr<u8>) -> u8 = thiscall vslot 0x1C => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610, cdecl)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0, `ret 8`)
    fn anim_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0, cdecl)
    fn rp_anim_blend_clump_get_association(clump: u32, anim: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;

    /// `CTaskSimpleCarSlowBeDraggedOut::Constructor(veh, door, lineUp, flag)` (0x648230, `ret 0x10`)
    fn slow_be_dragged_out_ctor(t: GPtr<u8>, veh: u32, door: u32, line_up: u32, flag: u32) -> GPtr<u8> = thiscall 0x648230 => unported;
    /// `CTaskSimpleCarSetPedOut::Constructor(veh, door, switchOffEngine)` (0x6478B0, `ret 0xC`)
    fn car_set_ped_out_ctor(t: GPtr<u8>, veh: u32, door: u32, b: u32) -> GPtr<u8> = thiscall 0x6478B0 => unported;
    /// `CTaskSimpleCarSetPedSlowDraggedOut::Constructor(veh, door)` (0x6483B0, `ret 8`)
    fn set_ped_slow_dragged_out_ctor(t: GPtr<u8>, veh: u32, door: u32) -> GPtr<u8> = thiscall 0x6483B0 => unported;
    /// `CTaskSimpleCarFallOut::Constructor(veh, door, lineUp)` (0x648D60, `ret 0xC`)
    fn car_fall_out_ctor(t: GPtr<u8>, veh: u32, door: u32, line_up: u32) -> GPtr<u8> = thiscall 0x648D60 => unported;
    /// `CTaskComplexCarSlowBeDraggedOut::CreateSubTask(int32 type)` (0x64C6A0, `ret 4`)
    fn complex_slow_be_dragged_out_create_sub_task_o(t: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x64C6A0 => complex_slow_be_dragged_out_create_sub_task;
    /// `CTaskComplexCarSlowBeDraggedOutAndStandUp::CreateSubTask(int32 type, CPed*)` (0x648710, `ret 8`)
    fn stand_up_create_sub_task(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x648710 => unported;
    /// CRT `operator new` (0x82119A, cdecl)
    fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `CTaskUtilityLineUpPedWithCar::CTaskUtilityLineUpPedWithCar(const CVector&, int32, int32, int32 door)` (0x64FBB0, `ret 0x10`)
    fn line_up_ctor(u: GPtr<u8>, offset: GPtr<u8>, time: u32, a: u32, door: u32) -> GPtr<u8> = thiscall 0x64FBB0 => unported;

    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::SetRadioStation` (0x5DFD90)
    fn ped_set_radio_station(p: GPtr<u8>) = thiscall 0x5DFD90 => unported;
    /// `CPed::RestartNonPartialAnims` (0x5DED50)
    fn ped_restart_non_partial_anims(p: GPtr<u8>) = thiscall 0x5DED50 => unported;
    /// `CPed::UpdateStatLeavingVehicle` (0x5E01B0)
    fn ped_update_stat_leaving_vehicle(p: GPtr<u8>) = thiscall 0x5E01B0 => unported;
    /// `CPed::ReplaceWeaponWhenExitingVehicle` (0x5E6490)
    fn ped_replace_weapon_when_exiting_vehicle(p: GPtr<u8>) = thiscall 0x5E6490 => unported;
    /// `CPed::CalculateNewOrientation` (0x5E52E0)
    fn ped_calculate_new_orientation(p: GPtr<u8>) = thiscall 0x5E52E0 => unported;
    /// `CPed::PositionPedOutOfCollision(int32 door, CVehicle*, bool)` (0x5E0820, `ret 0xC`)
    fn ped_position_ped_out_of_collision(p: GPtr<u8>, door: u32, veh: GPtr<u8>, b: u32) -> u8 = thiscall 0x5E0820 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CAEVehicleAudioEntity::PlayerAboutToExitVehicleAsDriver` (0x4F5BA0)
    fn vehicle_audio_player_about_to_exit_as_driver(a: GPtr<u8>) = thiscall 0x4F5BA0 => unported;
    /// `CAEPedAudioEntity::AddAudioEvent(id, volume, speed, …)` (0x4E2BB0, `ret 0x1C`)
    fn ped_audio_add_audio_event(e: GPtr<u8>, ev: u32, vol: u32, speed: u32, a3: u32, a4: u32, a5: u32, a6: u32) = thiscall 0x4E2BB0 => unported;
    /// `CVehicle::RemoveDriver(bool)` (0x6D1950, `ret 4`)
    fn vehicle_remove_driver(v: GPtr<u8>, b: u32) = thiscall 0x6D1950 => unported;
    /// `CVehicle::RemovePassenger(CPed*)` (0x6D1610, `ret 4`)
    fn vehicle_remove_passenger(v: GPtr<u8>, p: GPtr<u8>) = thiscall 0x6D1610 => unported;
    /// `CVehicle::IsLawEnforcementVehicle` (0x6D2370)
    fn vehicle_is_law_enforcement_vehicle(v: GPtr<u8>) -> u8 = thiscall 0x6D2370 => unported;
    /// `CVehicle::ChangeLawEnforcerState(bool)` (0x6D2330, `ret 4`)
    fn vehicle_change_law_enforcer_state(v: GPtr<u8>, b: u32) = thiscall 0x6D2330 => unported;
    /// `CTaskSimplePlayerOnFoot::Constructor` (0x685750)
    fn player_on_foot_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x685750 => unported;
    /// `CTaskSimpleStandStill::Constructor(time, looped, useAnimIdleStance, blend)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, looped: u32, idle: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskComplexWander::GetWanderTaskByPedType(CPed*)` (0x673D00, cdecl)
    fn wander_get_wander_task_by_ped_type(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x673D00 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0, `ret 0xC`)
    fn task_manager_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: u32, b: u32) = thiscall 0x681AF0 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix*, const CVector*)` (0x59C790, cdecl)
    fn multiply3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CPedPlacement::FindZCoorForPed(CVector*)` (0x616920, cdecl)
    fn ped_placement_find_z_coor_for_ped(p: GPtr<u8>) -> u8 = cdecl 0x616920 => unported;
}

/// `CVehicleAnimGroup` array (`CVehicleAnimGroupData::m_vehicleAnimGroups`, 0x94 bytes each).
const VEH_ANIM_GROUPS: usize = 0xC1CDC0;
/// `CCarEnterExit::ms_vecPedGetUpAnimOffset` (CVector)
const PED_GET_UP_ANIM_OFFSET: usize = 0xC18C3C;
/// The be-dragged-out time after which the line-up utility stops (1.7)
const BE_DRAGGED_OUT_LINE_UP_TIME: usize = 0x859B58;
/// `TARGET_DOOR_DRIVER` / `TARGET_DOOR_FRONT_RIGHT`
const DOOR_DRIVER: u32 = 10;
const DOOR_FRONT_RIGHT: u32 = 8;

/// Pointer to a Rust stack buffer (stands in for an object the original builds on its own stack).
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// The vehicle's anim group byte: handling (+0x384) +0xDE.
fn anim_group_byte(veh: GPtr<u8>) -> u32 {
    u32::from(rd::<u8>(rd::<GPtr<u8>>(veh, 0x384), 0xDE))
}

/// The vehicle's `CVehicleAnimGroup`.
fn anim_group(veh: GPtr<u8>) -> GPtr<u8> {
    GPtr::from_addr(VEH_ANIM_GROUPS + anim_group_byte(veh) as usize * 0x94)
}

/// Coach (0x67), tank (0x70), or bus (0x68) through the front-right door: nobody is dragged out of these
/// (`anim group byte + 0x58` compared, as the originals do).
fn no_drag_out(veh: GPtr<u8>, door: u32) -> bool {
    let g = anim_group_byte(veh) + 0x58;
    g == 0x67 || g == 0x70 || door == DOOR_FRONT_RIGHT && g == 0x68
}

/// `CTaskSimpleCarSlowDragPedOut::ComputeAnimID_Wrapper(int32* group, int32* anim)` (0x648100, `ret 8`): by door
/// (+0x14) 8/9 → anim 0x16D, 10/11 → 0x16C, 18 → 0x16E, any other door leaves `*anim` as it is; then `*group =
/// GetGroup(*anim)` of the vehicle's (+0x10) anim group.
#[hook(addr = 0x648100, conv = thiscall, name = "CTaskSimpleCarSlowDragPedOut::ComputeAnimID_Wrapper", expect = "8B 41 14 8B 54 24 08",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn slow_drag_ped_out_compute_anim_id(this: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) {
    match rd::<u32>(this, 0x14) {
        8 | 9 => wr::<u32>(anim, 0, 0x16D),
        10 | 11 => wr::<u32>(anim, 0, 0x16C),
        18 => wr::<u32>(anim, 0, 0x16E),
        _ => {}
    }
    let veh = rd::<GPtr<u8>>(this, 0x10);
    let gid = vehicle_anim_group_get_group(anim_group(veh), rd::<u32>(anim, 0));
    wr::<u32>(group, 0, gid);
}

/// `CTaskSimpleCarSlowDragPedOut::FinishAnimCarSlowDragPedOutCB(CAnimBlendAssociation*, void* task)` (0x648180,
/// cdecl): +8 = 1, +0xC = NULL; with a vehicle: `ComputeAnimID(&group, &anim)` (anim in the task argument slot,
/// so it stays the task pointer for a door outside the table; group in the slot of the pushed ECX, always
/// written) and `veh->ProcessOpenDoor(NULL, door, group, anim, 1.0)`; then an automobile (+0x590 == 0) whose
/// door status (damage manager +0x5A0) is 0 or 2 gets status + 1.
#[hook(addr = 0x648180, conv = cdecl, name = "CTaskSimpleCarSlowDragPedOut::FinishAnimCarSlowDragPedOutCB", expect = "51 56 8B 74 24 10",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn slow_drag_ped_out_finish_anim_cb(_assoc: GPtr<u8>, task: GPtr<u8>) {
    wr::<u8>(task, 8, 1);
    wr::<u32>(task, 0xC, 0);
    if !rd::<GPtr<u8>>(task, 0x10).is_null() {
        let mut l = [0u32, task.addr() as u32];
        let p = buf(&mut l);
        slow_drag_ped_out_compute_anim_id_o(task, p, p.byte_add(4));
        let v = rd::<GPtr<u8>>(task, 0x10);
        v_vehicle_process_open_door(v, GPtr::NULL, rd::<u32>(task, 0x14), l[0], l[1], 0x3F80_0000);
    }
    let v = rd::<GPtr<u8>>(task, 0x10);
    if v.is_null() || rd::<u32>(v, 0x590) != 0 {
        return;
    }
    let dm = |t: GPtr<u8>| rd::<GPtr<u8>>(t, 0x10).byte_add(0x5A0);
    let st = damage_manager_get_door_status(dm(task), rd::<u32>(task, 0x14));
    if st == 0 || damage_manager_get_door_status(dm(task), rd::<u32>(task, 0x14)) == 2 {
        let st = damage_manager_get_door_status(dm(task), rd::<u32>(task, 0x14));
        damage_manager_set_door_status(dm(task), rd::<u32>(task, 0x14), st.wrapping_add(1));
    }
}

/// `CTaskSimpleCarSlowDragPedOut::ProcessPed(CPed*)` (0x64E060). Finished (+8) or no vehicle: true; a vehicle nobody
/// is dragged out of (coach, tank, bus by the front-right door): true. Without an assoc: `StartAnim(ped)`; then the
/// jacked ped (the driver for door 10, else the passenger of `ComputePassengerIndexFromCarDoor`) and `rand() &
/// 0x3FF` (call site 0x64E0EF): above 0x200 `SayJacking(ped, jacked, veh, 0)`, failing that `SayJacked(jacked, veh,
/// 0)`; else the other way round. Always: `ComputeAnimID(&group, &anim)` (anim in the ped argument slot, group in
/// the pushed-ECX slot = `this`) and `veh->ProcessOpenDoor(ped, door, group, anim, assoc time +0x20)`; false.
#[hook(addr = 0x64E060, conv = thiscall, name = "CTaskSimpleCarSlowDragPedOut::ProcessPed", expect = "51 56 8B F1 8A 46 08",
       unverified = "carjack-1", route = "L2", hazards = "ax-al-return,heap-writes", recs = "carjack")]
pub fn slow_drag_ped_out_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 8) != 0 {
        return 1;
    }
    let veh = rd::<GPtr<u8>>(this, 0x10);
    if veh.is_null() || no_drag_out(veh, rd::<u32>(this, 0x14)) {
        return 1;
    }
    if rd::<u32>(this, 0xC) == 0 {
        slow_drag_ped_out_start_anim(this, ped);
        let veh = rd::<GPtr<u8>>(this, 0x10);
        if !veh.is_null() {
            let door = rd::<u32>(this, 0x14);
            let jacked = if door == DOOR_DRIVER {
                rd::<GPtr<u8>>(veh, 0x460)
            } else {
                let i = compute_passenger_index_from_car_door(veh, door);
                let v = rd::<GPtr<u8>>(this, 0x10);
                rd::<GPtr<u8>>(v, (0x464 + i.wrapping_mul(4)) as isize)
            };
            if !jacked.is_null() {
                if at_site(0x64E0EF, rand) & 0x3FF > 0x200 {
                    if say_jacking_o(ped, jacked, rd::<GPtr<u8>>(this, 0x10), 0) == 0 {
                        say_jacked(jacked, rd::<GPtr<u8>>(this, 0x10), 0);
                    }
                } else if say_jacked(jacked, rd::<GPtr<u8>>(this, 0x10), 0) == 0 {
                    say_jacking_o(ped, jacked, rd::<GPtr<u8>>(this, 0x10), 0);
                }
            }
        }
    }
    // [0] = group (the pushed-ECX slot = this), [1] = anim (the ped argument slot)
    let mut l = [this.addr() as u32, ped.addr() as u32];
    let p = buf(&mut l);
    slow_drag_ped_out_compute_anim_id_o(this, p, p.byte_add(4));
    let time = rd::<u32>(rd::<GPtr<u8>>(this, 0xC), 0x20);
    let v = rd::<GPtr<u8>>(this, 0x10);
    v_vehicle_process_open_door(v, ped, rd::<u32>(this, 0x14), l[0], l[1], time);
    0
}

/// `SayJacking(CPed* jacker, CPed* jacked, CVehicle*, uint32 offset)` (0x6497F0, cdecl): by the vehicle's audio type
/// (`GetVehicleTypeForAudio`, 16 bits, asked twice): 1 → the jacker says 0x79; 0 → 0x7A when the jacked ped's speech
/// entity (+0x294) answers true through its vtable +0x1C, else 0x7B; others → 0x7C (`Say(id, offset, 1.0, 0, 0,
/// 0)`). True when `Say` returned other than -1 (16 bits).
#[hook(addr = 0x6497F0, conv = cdecl, name = "SayJacking", expect = "56 8B 74 24 10",
       unverified = "carjack-1", route = "L2", hazards = "ax-al-return", recs = "carjack")]
pub fn say_jacking(jacker: GPtr<u8>, jacked: GPtr<u8>, veh: GPtr<u8>, offset: u32) -> u8 {
    let audio = veh.byte_add(0x138);
    let id = if vehicle_audio_get_vehicle_type_for_audio(audio) as u16 == 1 {
        0x79
    } else if vehicle_audio_get_vehicle_type_for_audio(audio) as u16 == 0 {
        if v_speech_1c(jacked.byte_add(0x294)) != 0 { 0x7A } else { 0x7B }
    } else {
        0x7C
    };
    u8::from(ped_say(jacker, id, offset, 0x3F80_0000, 0, 0, 0) as u16 != 0xFFFF)
}

/// `CTaskSimpleCarSlowBeDraggedOut::StartAnim(const CPed*)` (0x64C190): `RemoveCarSitAnim(ped)`; `ComputeAnimID(&group,
/// &anim)` (anim in the ped argument slot, group in the pushed-ECX slot = `this`); +0xC = `BlendAnimation(clump,
/// group, anim, 1000.0)` with finish callback 0x6483A0 (data = this); +0x19 = assoc flag 0x800 (+0x2E).
#[hook(addr = 0x64C190, conv = thiscall, name = "CTaskSimpleCarSlowBeDraggedOut::StartAnim", expect = "51 56 57 8B 7C 24 10",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn slow_be_dragged_out_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    car_enter_exit_remove_car_sit_anim(ped);
    let mut l = [this.addr() as u32, ped.addr() as u32];
    let p = buf(&mut l);
    slow_be_dragged_out_compute_anim_id(this, p, p.byte_add(4));
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), l[0], l[1], 0x447A_0000);
    wr::<GPtr<u8>>(this, 0xC, a);
    anim_set_finish_callback(a, 0x6483A0, this.addr() as u32);
    let f = rd::<u16>(rd::<GPtr<u8>>(this, 0xC), 0x2E);
    wr::<u8>(this, 0x19, ((f >> 11) & 1) as u8);
}

/// `CTaskSimpleCarSlowBeDraggedOut::ProcessPed(CPed*)` (0x64E180). No vehicle (+0x10): true. Finished (+8): true when
/// the ped's clump already plays anim 0x27 or 0x24; else blends (group 0) 0x27 when +0x19 is set, else 0x24 (delta
/// 1000.0); false. No assoc yet: `StartAnim(ped)`; false. A ped no longer in a vehicle (+0x46C bit 8): true. Else:
/// assoc time > [0x859B58] (fcomp, NaN not) sets the line-up's (+0x1C) +0x14 to 2; with +0x18: `ComputeAnimID(&group,
/// &anim)` (group in the pushed-ECX slot = `this`, anim in the ped argument slot) and `veh->ProcessOpenDoor(ped,
/// door, group, anim, time)`; false.
#[hook(addr = 0x64E180, conv = thiscall, name = "CTaskSimpleCarSlowBeDraggedOut::ProcessPed", expect = "51 56 8B F1 8B 46 10",
       unverified = "carjack-1", route = "L2", hazards = "ax-al-return,heap-writes", recs = "carjack")]
pub fn slow_be_dragged_out_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0x10) == 0 {
        return 1;
    }
    if rd::<u8>(this, 8) != 0 {
        if !rp_anim_blend_clump_get_association(rd::<u32>(ped, 0x18), 0x27).is_null()
            || !rp_anim_blend_clump_get_association(rd::<u32>(ped, 0x18), 0x24).is_null()
        {
            return 1;
        }
        let id = if rd::<u8>(this, 0x19) != 0 { 0x27 } else { 0x24 };
        anim_blend_animation(rd::<u32>(ped, 0x18), 0, id, 0x447A_0000);
        return 0;
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    if a.is_null() {
        slow_be_dragged_out_start_anim_o(this, ped);
        return 0;
    }
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 {
        return 1;
    }
    if x(rd::<f32>(a, 0x20)).fcom(x(g::<f32>(BE_DRAGGED_OUT_LINE_UP_TIME).read())) == Cmp::Greater {
        wr::<u32>(rd::<GPtr<u8>>(this, 0x1C), 0x14, 2);
    }
    if rd::<u8>(this, 0x18) != 0 {
        let mut l = [this.addr() as u32, ped.addr() as u32];
        let p = buf(&mut l);
        slow_be_dragged_out_compute_anim_id(this, p, p.byte_add(4));
        let time = rd::<u32>(rd::<GPtr<u8>>(this, 0xC), 0x20);
        let v = rd::<GPtr<u8>>(this, 0x10);
        v_vehicle_process_open_door(v, ped, rd::<u32>(this, 0x14), l[0], l[1], time);
    }
    0
}

/// `CTaskSimpleCarSetPedSlowDraggedOut::ProcessPed(CPed*)` (0x64C220). No ped vehicle (+0x58C) or no task vehicle
/// (+8): true. `ComputeDoorFlag(veh, door, true)` (result unused). The driver (+0x460): `RemoveDriver(true)`, door
/// lock (+0x4F8) 5 → 1, a cop (+0x598 == 6) in a law-enforcement vehicle → `ChangeLawEnforcerState(false)`; else every
/// passenger seat (+0x464, count byte +0x488) holding the ped → `RemovePassenger(ped)`. The ped leaves the vehicle
/// (+0x46C bit 8 cleared) and gets a primary default task: a player → `CTaskSimplePlayerOnFoot`, created-by (+0x484)
/// 2 → `CTaskSimpleStandStill(999999, true, false, 8.0)`, else `GetWanderTaskByPedType`. `RestartNonPartialAnims`,
/// +0x1C |= 1, `UpdateStatLeavingVehicle`, `ReplaceWeaponWhenExitingVehicle`. Placement: `offset =
/// (ComputeAnimDoorOffsets(6 for doors 8/9, else 5).xy - ms_vecPedGetUpAnimOffset.xy, 0)`, `Multiply3x3(ped
/// matrix, offset)` + ped position, `FindZCoorForPed`, the move speed (+0x44) zeroed, the position written (matrix
/// or placement). Heading: the *vehicle's* (`atan2(-fwd.x, fwd.y)` of its matrix, else its placement +0x10; gta-reversed
/// reads the ped's) into +0x55C and +0x558, `CalculateNewOrientation`. No room to leave (`IsRoomForPedToLeaveCar(veh,
/// door, &offset)`) → `PositionPedOutOfCollision(door, veh, true)`. +0x474 |= 8, state (+0x530) and move state
/// (+0x538) 1, `SetMoveAnim` (vtable +0x5C); true.
#[hook(addr = 0x64C220, conv = thiscall, name = "CTaskSimpleCarSetPedSlowDraggedOut::ProcessPed", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "carjack-1", route = "L2", hazards = "ax-al-return,heap-writes,x87-transcendental", recs = "carjack")]
#[allow(clippy::too_many_lines)]
pub fn set_ped_slow_dragged_out_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(ped, 0x58C) == 0 || rd::<u32>(this, 8) == 0 {
        return 1;
    }
    car_enter_exit_compute_door_flag(rd::<GPtr<u8>>(this, 8), rd::<u32>(this, 0xC), 1);
    let veh = rd::<GPtr<u8>>(this, 8);
    if rd::<GPtr<u8>>(veh, 0x460) == ped {
        vehicle_remove_driver(veh, 1);
        let v = rd::<GPtr<u8>>(this, 8);
        if rd::<u32>(v, 0x4F8) == 5 {
            wr::<u32>(v, 0x4F8, 1);
        }
        if rd::<u32>(ped, 0x598) == 6 && vehicle_is_law_enforcement_vehicle(rd::<GPtr<u8>>(this, 8)) != 0 {
            vehicle_change_law_enforcer_state(rd::<GPtr<u8>>(this, 8), 0);
        }
    } else if rd::<u8>(veh, 0x488) != 0 {
        let mut i = 0u32;
        let mut v = veh;
        loop {
            if rd::<GPtr<u8>>(v, (0x464 + i * 4) as isize) == ped {
                vehicle_remove_passenger(v, ped);
            }
            v = rd::<GPtr<u8>>(this, 8);
            i += 1;
            if i >= u32::from(rd::<u8>(v, 0x488)) {
                break;
            }
        }
    }
    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FEFF);
    // the intelligence (+0x47C) is read after the task is built, except on the wander path (read before it)
    let (task, intel) = if ped_is_player(ped) != 0 {
        let t = task_new(0x1C);
        let t = if t.is_null() { t } else { player_on_foot_ctor(t) };
        (t, rd::<GPtr<u8>>(ped, 0x47C))
    } else if rd::<u8>(ped, 0x484) == 2 {
        let t = task_new(0x20);
        let t = if t.is_null() { t } else { stand_still_ctor(t, 999_999, 1, 0, 0x4100_0000) };
        (t, rd::<GPtr<u8>>(ped, 0x47C))
    } else {
        let intel = rd::<GPtr<u8>>(ped, 0x47C);
        (wander_get_wander_task_by_ped_type(ped), intel)
    };
    task_manager_set_task(intel.byte_add(4), task, 4, 0);
    ped_restart_non_partial_anims(ped);
    wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) | 1);
    ped_update_stat_leaving_vehicle(ped);
    ped_replace_weapon_when_exiting_vehicle(ped);

    let gx = g::<f32>(PED_GET_UP_ANIM_OFFSET).read();
    let gy = g::<f32>(PED_GET_UP_ANIM_OFFSET + 4).read();
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    let door = rd::<u32>(this, 0xC);
    let mut door_off = [0u32; 3];
    vehicle_anim_group_compute_anim_door_offsets(
        anim_group(rd::<GPtr<u8>>(this, 8)),
        buf(&mut door_off),
        if door == 8 || door == 9 { 6 } else { 5 },
    );
    // offset = (door.x - getup.x, door.y - getup.y, 0)
    let mut offset = [
        f32_of(x(f32::from_bits(door_off[0])).fsub_m(gx)).to_bits(),
        f32_of(x(f32::from_bits(door_off[1])).fsub_m(gy)).to_bits(),
        0,
    ];
    let mut pos = [0u32; 3];
    multiply3x3(buf(&mut pos), pm, buf(&mut offset));
    let base = if rd::<u32>(ped, 0x14) != 0 { rd::<GPtr<u8>>(ped, 0x14).byte_add(0x30) } else { ped.byte_add(4) };
    for (i, c) in pos.iter_mut().enumerate() {
        *c = f32_of(x(f32::from_bits(*c)).fadd_m(rd::<f32>(base, i as isize * 4))).to_bits();
    }
    ped_placement_find_z_coor_for_ped(buf(&mut pos));
    wr::<u32>(ped, 0x44, 0);
    wr::<u32>(ped, 0x48, 0);
    wr::<u32>(ped, 0x4C, 0);
    let m = rd::<GPtr<u8>>(ped, 0x14);
    if m.is_null() {
        wr::<u32>(ped, 4, pos[0]);
        wr::<u32>(ped, 8, pos[1]);
        wr::<u32>(ped, 0xC, pos[2]);
    } else {
        wr::<u32>(m, 0x30, pos[0]);
        wr::<u32>(rd::<GPtr<u8>>(ped, 0x14), 0x34, pos[1]);
        wr::<u32>(rd::<GPtr<u8>>(ped, 0x14), 0x38, pos[2]);
    }
    let veh = rd::<GPtr<u8>>(this, 8);
    let vm = rd::<GPtr<u8>>(veh, 0x14);
    let h = if vm.is_null() {
        x(rd::<f32>(veh, 0x10))
    } else {
        x(rd::<f32>(vm, 0x10)).fchs().fpatan(x(rd::<f32>(vm, 0x14)))
    };
    wr::<f32>(ped, 0x55C, f32_of(h));
    wr::<f32>(ped, 0x558, f32_of(h));
    ped_calculate_new_orientation(ped);
    if car_enter_exit_is_room_for_ped_to_leave_car(rd::<GPtr<u8>>(this, 8), rd::<u32>(this, 0xC), buf(&mut offset)) == 0 {
        ped_position_ped_out_of_collision(ped, rd::<u32>(this, 0xC), rd::<GPtr<u8>>(this, 8), 1);
    }
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 8);
    wr::<u32>(ped, 0x530, 1);
    wr::<u32>(ped, 0x538, 1);
    v_ped_set_move_anim(ped);
    1
}

/// `CTaskComplexCarSlowBeDraggedOut::CreateTaskUtilityLineUpPedWithCar(CPed*)` (0x648520): +0x18 = `new` (CRT, 0x1C)
/// `CTaskUtilityLineUpPedWithCar((0, 0, 0), 0, 0, door +0x10)` (NULL when the allocation fails; the previous one is
/// not freed, as the original).
#[hook(addr = 0x648520, conv = thiscall, name = "CTaskComplexCarSlowBeDraggedOut::CreateTaskUtilityLineUpPedWithCar", expect = "6A FF 68 0B 29 84 00",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn complex_slow_be_dragged_out_create_line_up(this: GPtr<u8>, _ped: GPtr<u8>) {
    let mut zero = [0u32; 3];
    let u = operator_new(0x1C);
    let r = if u.is_null() { u } else { line_up_ctor(u, buf(&mut zero), 0, 0, rd::<u32>(this, 0x10)) };
    wr::<GPtr<u8>>(this, 0x18, r);
}

/// `CTaskComplexCarSlowBeDraggedOut::PrepareVehicleForPedExit(CPed*)` (0x6485A0): door 10 only: the vehicle's
/// status bits (+0x36, high 5 bits) become 9 (`STATUS_FORCED_STOP`) when its driver exists and is a player, else 4
/// (`STATUS_ABANDONED`).
#[hook(addr = 0x6485A0, conv = thiscall, name = "CTaskComplexCarSlowBeDraggedOut::PrepareVehicleForPedExit", expect = "56 8B F1 83 7E 10 0A",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn complex_slow_be_dragged_out_prepare_vehicle_for_ped_exit(this: GPtr<u8>, _ped: GPtr<u8>) {
    if rd::<u32>(this, 0x10) != DOOR_DRIVER {
        return;
    }
    let d = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x460);
    let bits = if !d.is_null() && ped_is_player(d) != 0 { 0x48 } else { 0x20 };
    let v = rd::<GPtr<u8>>(this, 0xC);
    wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | bits);
}

/// `CTaskComplexCarSlowBeDraggedOut::CreateSubTask(int32 type)` (0x64C6A0, `ret 4`; entry `nop; jmp` into the
/// protection's relocated `mov eax, fs:[0]`): 0x335 → `CTaskSimpleCarSlowBeDraggedOut(veh, door, lineUp, flag +0x14)`,
/// 0x330 → `CTaskSimpleCarSetPedOut(veh, door, flag == 0)`, 0x336 → `CTaskSimpleCarSetPedSlowDraggedOut(veh, door)`,
/// 0x342 → `CTaskSimpleCarFallOut(veh, door, lineUp)`; 0x516 and a failed allocation → NULL; any other type → 0.
#[hook(addr = 0x64C6A0, conv = thiscall, name = "CTaskComplexCarSlowBeDraggedOut::CreateSubTask", expect = "90 E9 B7 70 DB FF",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn complex_slow_be_dragged_out_create_sub_task(this: GPtr<u8>, ty: u32) -> GPtr<u8> {
    let veh = rd::<u32>(this, 0xC);
    let door = rd::<u32>(this, 0x10);
    let alloc = |size: u32, ctor: &dyn Fn(GPtr<u8>) -> GPtr<u8>| {
        let t = task_new(size);
        if t.is_null() { t } else { ctor(t) }
    };
    match ty {
        0x335 => alloc(0x20, &|t| {
            slow_be_dragged_out_ctor(t, veh, door, rd::<u32>(this, 0x18), u32::from(rd::<u8>(this, 0x14)))
        }),
        0x330 => alloc(0x18, &|t| car_set_ped_out_ctor(t, veh, door, u32::from(rd::<u8>(this, 0x14) == 0))),
        0x336 => alloc(0x10, &|t| set_ped_slow_dragged_out_ctor(t, veh, door)),
        0x342 => alloc(0x1C, &|t| car_fall_out_ctor(t, veh, door, rd::<u32>(this, 0x18))),
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexCarSlowBeDraggedOut::CreateNextSubTask(CPed*)` (0x64C810): NULL unless the ped is in a vehicle
/// (+0x46C bit 8 and +0x58C); by the sub-task's type: 0x330 and 0x336 → `CreateSubTask(0x516)` (gta-reversed sends
/// 0x330 to 0x336; the bytes finish), 0x335 → 0x336, 0x342 → 0x330, others NULL.
#[hook(addr = 0x64C810, conv = thiscall, name = "CTaskComplexCarSlowBeDraggedOut::CreateNextSubTask", expect = "8B 44 24 04 56 8B F1",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn complex_slow_be_dragged_out_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 || rd::<u32>(ped, 0x58C) == 0 {
        return GPtr::NULL;
    }
    let next = match v_get_task_type(rd::<GPtr<u8>>(this, 8)) {
        0x330 | 0x336 => 0x516,
        0x335 => 0x336,
        0x342 => 0x330,
        _ => return GPtr::NULL,
    };
    complex_slow_be_dragged_out_create_sub_task_o(this, next)
}

/// `CTaskComplexCarSlowBeDraggedOutAndStandUp::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x6486F0):
/// priority 2 → the sub-task's (+8, not NULL-checked) `MakeAbortable(ped, 2, event)` (tail jump); else false.
#[hook(addr = 0x6486F0, conv = thiscall, name = "CTaskComplexCarSlowBeDraggedOutAndStandUp::MakeAbortable", expect = "83 7C 24 08 02 75 10",
       unverified = "carjack-1", route = "L2", hazards = "ax-al-return", recs = "carjack")]
pub fn stand_up_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    if priority != 2 {
        return 0;
    }
    v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 2, event)
}

/// `CTaskComplexCarSlowBeDraggedOutAndStandUp::CreateFirstSubTask(CPed*)` (0x648A10). The player
/// (`FindPlayerPed(0)`) with a vehicle → its audio's `PlayerAboutToExitVehicleAsDriver`; anyone else who drives the
/// vehicle (+0x58C, read unchecked) → `SetRadioStation`. Coach/tank, or bus through door 8 → `CreateSubTask(0x2C0)`
/// (leave car). Else by the vehicle's anim group byte (≤ 0x1C, the table at 0x648B54): group 0 → audio event 0x45
/// (door 10) / 0x46 (door 8), 1 → 0x47, 2 → 0x49 (`AddAudioEvent(id, 0, 1.0)` on the ped's audio +0x138); then
/// `CreateSubTask(0x337)` (be dragged out).
#[hook(addr = 0x648A10, conv = thiscall, name = "CTaskComplexCarSlowBeDraggedOutAndStandUp::CreateFirstSubTask", expect = "56 57 6A 00 8B F9",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn stand_up_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let player = find_player_ped(0);
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if ped == player {
        if !veh.is_null() {
            vehicle_audio_player_about_to_exit_as_driver(veh.byte_add(0x138));
        }
    } else if rd::<GPtr<u8>>(veh, 0x460) == ped {
        ped_set_radio_station(ped);
    }
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if !veh.is_null() && no_drag_out(veh, rd::<u32>(this, 0x10)) {
        return stand_up_create_sub_task(this, 0x2C0, ped);
    }
    const GROUP_CASE: [u8; 29] = [0, 1, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 0, 0, 3, 3, 2, 0, 0, 3, 3, 0, 0, 3, 3, 3, 3, 0];
    let b = anim_group_byte(veh);
    let ev = match GROUP_CASE.get(b as usize).copied().unwrap_or(3) {
        0 => match rd::<u32>(this, 0x10) {
            DOOR_DRIVER => Some(0x45),
            DOOR_FRONT_RIGHT => Some(0x46),
            _ => None,
        },
        1 => Some(0x47),
        2 => Some(0x49),
        _ => None,
    };
    if let Some(ev) = ev {
        ped_audio_add_audio_event(ped.byte_add(0x138), ev, 0, 0x3F80_0000, 0, 0, 0, 0);
    }
    stand_up_create_sub_task(this, 0x337, ped)
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_SLOW_DRAG_PED_OUT_COMPUTE_ANIM_ID,
    HOOK_SLOW_DRAG_PED_OUT_FINISH_ANIM_CB,
    HOOK_SLOW_DRAG_PED_OUT_PROCESS_PED,
    HOOK_SAY_JACKING,
    HOOK_SLOW_BE_DRAGGED_OUT_START_ANIM,
    HOOK_SLOW_BE_DRAGGED_OUT_PROCESS_PED,
    HOOK_SET_PED_SLOW_DRAGGED_OUT_PROCESS_PED,
    HOOK_COMPLEX_SLOW_BE_DRAGGED_OUT_CREATE_LINE_UP,
    HOOK_COMPLEX_SLOW_BE_DRAGGED_OUT_PREPARE_VEHICLE_FOR_PED_EXIT,
    HOOK_COMPLEX_SLOW_BE_DRAGGED_OUT_CREATE_SUB_TASK,
    HOOK_COMPLEX_SLOW_BE_DRAGGED_OUT_CREATE_NEXT_SUB_TASK,
    HOOK_STAND_UP_MAKE_ABORTABLE,
    HOOK_STAND_UP_CREATE_FIRST_SUB_TASK,
];
