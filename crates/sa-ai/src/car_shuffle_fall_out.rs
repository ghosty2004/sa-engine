//! `CTaskSimpleCarShuffle` (moving across to the driver's seat) and `CTaskSimpleCarFallOut` (falling out of a moving
//! vehicle): anim start, finish callbacks, `ProcessPed`, `SetPedPosition`, `MakeAbortable`, `ComputeAnimID`
//! (PORT-AI-7). Ported from the 1.0 US bytes (the exe wins over gta-reversed). Every call to another catalogued
//! function goes through `orig!` at the original address.
//!
//! Both tasks: +8 finished, +0xC the anim association, +0x10 the vehicle, +0x14 the door (fall out), +0x18 the
//! `CTaskUtilityLineUpPedWithCar*`.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CVehicleAnimGroupData::m_vehicleAnimGroups` (0x94 bytes each)
const VEH_ANIM_GROUPS: u32 = 0xC1CDC0;
/// 1000.0f
const K_1000: u32 = 0x447A_0000;
/// -1000.0f
const K_MINUS_1000: f32 = -1000.0;
/// `CDefaultAnimCallback::DefaultAnimCB`
const DEFAULT_ANIM_CB: u32 = 0x4D1390;
/// `CTaskSimpleCarShuffle::FinishAnimCarShuffleCB`
const SHUFFLE_CB: u32 = 0x646970;
/// `CTaskSimpleCarFallOut::FinishAnimFallOutCB`
const FALL_OUT_CB: u32 = 0x648EE0;

orig! {
    /// `CVehicleAnimGroup::GetGroup(int32 animId)` (0x6E3B00, `ret 4`)
    fn veh_anim_group_get_group(g: GPtr<u8>, anim: u32) -> u32 = thiscall 0x6E3B00 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610, cdecl)
    fn anim_blend_animation(clump: u32, group: u32, anim: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0, `ret 8`)
    fn assoc_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CPed::SetPedPositionInCar` (0x5DF910)
    fn ped_set_ped_position_in_car(p: GPtr<u8>) = thiscall 0x5DF910 => unported;
    /// `CTaskUtilityLineUpPedWithCar::ProcessPed(CPed*, CVehicle*, CAnimBlendAssociation*)` (0x6513A0, `ret 0xC`)
    fn line_up_process_ped(u: GPtr<u8>, ped: GPtr<u8>, veh: GPtr<u8>, a: GPtr<u8>) = thiscall 0x6513A0 => unported;
    /// `CCarEnterExit::RemoveCarSitAnim(const CPed*)` (0x64F680, cdecl)
    fn car_enter_exit_remove_car_sit_anim(ped: GPtr<u8>) = cdecl 0x64F680 => unported;
    /// `CDamageManager::GetDoorStatus(eDoors)` (0x6C2250, `ret 4`)
    fn damage_get_door_status(d: GPtr<u8>, door: u32) -> u32 = thiscall 0x6C2250 => unported;
    /// `CDamageManager::SetDoorStatus(eDoors, eDoorStatus)` (0x6C21E0, `ret 8`)
    fn damage_set_door_status(d: GPtr<u8>, door: u32, st: u32) = thiscall 0x6C21E0 => unported;
    /// `CVehicle` virtual +0x70 (`ProcessOpenDoor(CPed*, uint32 door, uint32 animGroup, uint32 animId, float time)`,
    /// `ret 0x14`)
    fn v_process_open_door(v: GPtr<u8>, ped: GPtr<u8>, door: u32, group: u32, anim: u32, t: u32) = thiscall vslot 0x70 => unported;
    /// `CTaskSimpleCarShuffle::StartAnim(const CPed*)` (0x64B3E0, `ret 4`)
    fn o_shuffle_start_anim(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x64B3E0 => unported;
    /// `CTaskSimpleCarFallOut::ComputeAnimID(int32& group, int32& anim)` (0x648E90, `ret 8`)
    fn o_fall_out_compute_anim_id(t: GPtr<u8>, group: GPtr<u8>, anim: GPtr<u8>) = thiscall 0x648E90 => unported;
    /// `CTaskSimpleCarFallOut::StartAnim(const CPed*)` (0x64CCE0, `ret 4`)
    fn o_fall_out_start_anim(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x64CCE0 => unported;
}

/// The vehicle's anim group record (`m_vehicleAnimGroups[handling +0xDE]`, the handling at +0x384).
fn veh_anim_group(veh: GPtr<u8>) -> GPtr<u8> {
    let id = u32::from(rd::<u8>(rd::<GPtr<u8>>(veh, 0x384), 0xDE));
    GPtr::from_addr(id.wrapping_mul(0x94).wrapping_add(VEH_ANIM_GROUPS) as usize)
}

/// The fall-out anim for a door: 0x183 for doors 0xA / 0xB, else 0x184.
fn fall_out_anim(door: u32) -> u32 {
    if door == 0xA || door == 0xB { 0x183 } else { 0x184 }
}

// ---- CTaskSimpleCarShuffle ---------------------------------------------------------------------------------------

/// `CTaskSimpleCarShuffle::FinishAnimCarShuffleCB(CAnimBlendAssociation*, void* task)` (0x646970, cdecl): the task's
/// association (if any) blends out at -1000.0; the association is dropped and the task finished.
#[hook(addr = 0x646970, conv = cdecl, name = "CTaskSimpleCarShuffle::FinishAnimCarShuffleCB", expect = "8B 44 24 08 8B 48 0C",
       unverified = "ai7-9", route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn car_shuffle_finish_anim_cb(_assoc: GPtr<u8>, task: GPtr<u8>) {
    let a = rd::<GPtr<u8>>(task, 0xC);
    if !a.is_null() {
        wr::<f32>(a, 0x1C, K_MINUS_1000);
    }
    wr::<u32>(task, 0xC, 0);
    wr::<u8>(task, 8, 1);
}

/// `CTaskSimpleCarShuffle::ProcessPed(CPed*)` (0x64DC40, `ret 4`): done (AL = 1) when finished or without a vehicle;
/// otherwise starts the anim if none is playing and AL = 0.
#[hook(addr = 0x64DC40, conv = thiscall, name = "CTaskSimpleCarShuffle::ProcessPed", expect = "8A 41 08 84 C0",
       unverified = "ai7-9", route = "L2", hazards = "ax-al-return,heap-writes", recs = "car-ai")]
pub fn car_shuffle_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 8) != 0 || rd::<u32>(this, 0x10) == 0 {
        return 1;
    }
    if rd::<u32>(this, 0xC) == 0 {
        o_shuffle_start_anim(this, ped);
    }
    0
}

/// `CTaskSimpleCarShuffle::SetPedPosition(CPed*)` (0x646920, `ret 4`): a ped already in the vehicle (+0x46C bit
/// 0x100) without an association, or with one whose blend amount is not above [0x858C20], is placed in its seat;
/// otherwise the line-up utility positions it. AL = 1.
#[hook(addr = 0x646920, conv = thiscall, name = "CTaskSimpleCarShuffle::SetPedPosition", expect = "8B 51 0C 56 8B 74 24 08",
       unverified = "ai7-9", route = "L2", hazards = "ax-al-return,heap-writes", recs = "car-ai")]
pub fn car_shuffle_set_ped_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let a = rd::<GPtr<u8>>(this, 0xC);
    if rd::<u32>(ped, 0x46C) & 0x100 != 0
        && (a.is_null() || x(rd::<f32>(a, 0x18)).fcom(x(g::<f32>(0x858C20).read())).test_any(0x41))
    {
        ped_set_ped_position_in_car(ped);
        return 1;
    }
    line_up_process_ped(rd::<GPtr<u8>>(this, 0x18), ped, rd::<GPtr<u8>>(this, 0x10), a);
    1
}

/// `CTaskSimpleCarShuffle::StartAnim(const CPed*)` (0x64B3E0, `ret 4`): blends anim 0x174 of the vehicle's anim
/// group at 1000.0, keeps the association (+0xC) and sets `FinishAnimCarShuffleCB` with the task.
#[hook(addr = 0x64B3E0, conv = thiscall, name = "CTaskSimpleCarShuffle::StartAnim", expect = "56 8B F1 8B 46 10",
       unverified = "ai7-9", route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn car_shuffle_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let group = veh_anim_group_get_group(veh_anim_group(rd::<GPtr<u8>>(this, 0x10)), 0x174);
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), group, 0x174, K_1000);
    wr::<GPtr<u8>>(this, 0xC, a);
    assoc_set_finish_callback(a, SHUFFLE_CB, this.addr() as u32);
}

// ---- CTaskSimpleCarFallOut ---------------------------------------------------------------------------------------

/// `CTaskSimpleCarFallOut::ComputeAnimID(int32& group, int32& anim)` (0x648E90, `ret 8`): `anim` = 0x183 for doors
/// 0xA / 0xB, else 0x184; `group` = the vehicle anim group's group for that anim (`anim` re-read after the store).
#[hook(addr = 0x648E90, conv = thiscall, name = "CTaskSimpleCarFallOut::ComputeAnimID", expect = "8B 41 14 83 F8 0A",
       unverified = "ai7-9", route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn car_fall_out_compute_anim_id(this: GPtr<u8>, group: GPtr<u32>, anim: GPtr<u32>) {
    anim.write(fall_out_anim(rd::<u32>(this, 0x14)));
    let ag = veh_anim_group(rd::<GPtr<u8>>(this, 0x10));
    group.write(veh_anim_group_get_group(ag, anim.read()));
}

/// `CTaskSimpleCarFallOut::FinishAnimFallOutCB(CAnimBlendAssociation*, void* task)` (0x648EE0, cdecl): finishes the
/// task and drops the association; for a vehicle whose +0x590 is 0 the door's status (damage manager +0x5A0)
/// advances by one when it is 0 or 2.
#[hook(addr = 0x648EE0, conv = cdecl, name = "CTaskSimpleCarFallOut::FinishAnimFallOutCB", expect = "8B 44 24 08 8B 48 10",
       unverified = "ai7-9", route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn car_fall_out_finish_anim_cb(_assoc: GPtr<u8>, task: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(task, 0x10);
    wr::<u8>(task, 8, 1);
    wr::<u32>(task, 0xC, 0);
    if veh.is_null() || rd::<u32>(veh, 0x590) != 0 {
        return;
    }
    let door = rd::<u32>(task, 0x14);
    let dm = veh.byte_add(0x5A0);
    if damage_get_door_status(dm, door) != 0 && damage_get_door_status(dm, door) != 2 {
        return;
    }
    let st = damage_get_door_status(dm, door).wrapping_add(1);
    damage_set_door_status(dm, door, st);
}

/// `CTaskSimpleCarFallOut::SetPedPosition(CPed*)` (0x648E40, `ret 4`): with a vehicle, a ped whose task is not
/// finished and whose association is missing or at blend amount exactly 0.0 is placed in its seat; otherwise the
/// line-up utility positions it. AL = 1.
#[hook(addr = 0x648E40, conv = thiscall, name = "CTaskSimpleCarFallOut::SetPedPosition", expect = "8B 51 10 85 D2",
       unverified = "ai7-9", route = "L2", hazards = "ax-al-return,heap-writes", recs = "car-ai")]
pub fn car_fall_out_set_ped_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let veh = rd::<GPtr<u8>>(this, 0x10);
    if veh.is_null() {
        return 1;
    }
    if rd::<u8>(this, 8) == 0 {
        let a = rd::<GPtr<u8>>(this, 0xC);
        if a.is_null() || !x(rd::<f32>(a, 0x18)).fcom(x(g::<f32>(0x858B50).read())).test_parity(0x44) {
            ped_set_ped_position_in_car(ped);
            return 1;
        }
    }
    line_up_process_ped(rd::<GPtr<u8>>(this, 0x18), ped, veh, rd::<GPtr<u8>>(this, 0xC));
    1
}

/// `CTaskSimpleCarFallOut::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x64CC60, `ret 0xC`): only urgent
/// aborts (priority 2, AL = 1): the association blends out at -1000.0 with the default callback and is dropped, and
/// with a vehicle its virtual +0x70 runs with the computed group / anim at time 1.0. Otherwise AL = 0. The original
/// keeps the anim id in its own priority argument slot (dead afterwards); a local here.
#[hook(addr = 0x64CC60, conv = thiscall, name = "CTaskSimpleCarFallOut::MakeAbortable", expect = "51 83 7C 24 0C 02",
       unverified = "ai7-9", route = "L2", hazards = "ax-al-return,heap-writes", recs = "car-ai")]
pub fn car_fall_out_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, _event: u32) -> u8 {
    if priority != 2 {
        return 0;
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    if !a.is_null() {
        wr::<f32>(a, 0x1C, K_MINUS_1000);
        assoc_set_finish_callback(rd::<GPtr<u8>>(this, 0xC), DEFAULT_ANIM_CB, 0);
        wr::<u32>(this, 0xC, 0);
    }
    if rd::<u32>(this, 0x10) != 0 {
        let mut ga = [0u32; 2];
        let p: GPtr<u32> = GPtr::from_raw(ga.as_mut_ptr());
        o_fall_out_compute_anim_id(this, p.cast(), p.wrapping_index(1).cast());
        let (group, anim) = (p.read(), p.wrapping_index(1).read());
        v_process_open_door(rd::<GPtr<u8>>(this, 0x10), ped, rd::<u32>(this, 0x14), group, anim, 0x3F80_0000);
    }
    1
}

/// `CTaskSimpleCarFallOut::ProcessPed(CPed*)` (0x64E2C0, `ret 4`): done (AL = 1) without a vehicle, when finished
/// or when the ped is no longer in the vehicle; starts the anim when none is playing; while playing, the vehicle's
/// virtual +0x70 runs with the computed group / anim and the association's current time (+0x20). The original
/// writes the anim id into its own ped argument slot; a local here.
#[hook(addr = 0x64E2C0, conv = thiscall, name = "CTaskSimpleCarFallOut::ProcessPed", expect = "51 56 8B F1 8B 46 10",
       unverified = "ai7-9", route = "L2", hazards = "ax-al-return,heap-writes", recs = "car-ai")]
pub fn car_fall_out_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0x10) == 0 || rd::<u8>(this, 8) != 0 {
        return 1;
    }
    if rd::<u32>(this, 0xC) == 0 {
        o_fall_out_start_anim(this, ped);
        return 0;
    }
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 {
        return 1;
    }
    let mut ga = [0u32; 2];
    let p: GPtr<u32> = GPtr::from_raw(ga.as_mut_ptr());
    o_fall_out_compute_anim_id(this, p.cast(), p.wrapping_index(1).cast());
    let t = rd::<u32>(rd::<GPtr<u8>>(this, 0xC), 0x20);
    let (group, anim) = (p.read(), p.wrapping_index(1).read());
    v_process_open_door(rd::<GPtr<u8>>(this, 0x10), ped, rd::<u32>(this, 0x14), group, anim, t);
    0
}

/// `CTaskSimpleCarFallOut::StartAnim(const CPed*)` (0x64CCE0, `ret 4`): removes the car-sit anim, blends the fall-out
/// anim (0x183 / 0x184 by door) of the vehicle's group at 1000.0, keeps the association and sets
/// `FinishAnimFallOutCB` with the task.
#[hook(addr = 0x64CCE0, conv = thiscall, name = "CTaskSimpleCarFallOut::StartAnim", expect = "53 8B 5C 24 08",
       unverified = "ai7-9", route = "L2", hazards = "heap-writes", recs = "car-ai")]
pub fn car_fall_out_start_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    car_enter_exit_remove_car_sit_anim(ped);
    let anim = fall_out_anim(rd::<u32>(this, 0x14));
    let group = veh_anim_group_get_group(veh_anim_group(rd::<GPtr<u8>>(this, 0x10)), anim);
    let a = anim_blend_animation(rd::<u32>(ped, 0x18), group, anim, K_1000);
    wr::<GPtr<u8>>(this, 0xC, a);
    assoc_set_finish_callback(a, FALL_OUT_CB, this.addr() as u32);
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_CAR_SHUFFLE_FINISH_ANIM_CB,
    HOOK_CAR_SHUFFLE_PROCESS_PED,
    HOOK_CAR_SHUFFLE_SET_PED_POSITION,
    HOOK_CAR_SHUFFLE_START_ANIM,
    HOOK_CAR_FALL_OUT_COMPUTE_ANIM_ID,
    HOOK_CAR_FALL_OUT_FINISH_ANIM_CB,
    HOOK_CAR_FALL_OUT_SET_PED_POSITION,
    HOOK_CAR_FALL_OUT_MAKE_ABORTABLE,
    HOOK_CAR_FALL_OUT_PROCESS_PED,
    HOOK_CAR_FALL_OUT_START_ANIM,
];
