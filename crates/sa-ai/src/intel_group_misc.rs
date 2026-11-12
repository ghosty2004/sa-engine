//! Small `CPedIntelligence` queries (respect, car tasks, interesting entities), `AddTaskPhysResponse` /
//! `AddTaskPrimaryMaybeInGroup`, `CPedGroup` member loops (`IsAnyoneUsingCar`, `RemoveAllFollowers`, destructor,
//! `CPedGroups::CleanUpForShutDown`) and `CTaskSimpleHoldEntity::CanThrowEntity` / `ReleaseEntity` (PORT-AI-7).
//! Ported from the 1.0 US bytes (the exe wins over gta-reversed). Every call to another catalogued function goes
//! through `orig!` at the original address.
//!
//! `CPedIntelligence`: +0 the ped, +4 the `CTaskManager`, +0x68 the event group, +0x288 interesting entities[3]
//! (registered references). `CPedGroup`: +8 `CPedGroupMembership` (+4 members[8], member 7 the leader), +0x30
//! `CPedGroupIntelligence`, +0x2D0 bool.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CPedGroups::ms_groups`
const GROUPS: usize = 0xC09920;
/// `sizeof(CPedGroup)`
const GROUP_SIZE: usize = 0x2D4;

orig! {
    /// Virtual slot +0 (scalar deleting destructor, `ret 4`).
    fn v_delete(o: GPtr<u8>, flags: u32) -> u32 = thiscall vslot 0x0 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0, `ret 0xC`)
    fn tm_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: i32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0, `ret 4`)
    fn intel_find_task_by_type(i: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CAcquaintance::GetAcquaintances(int32)` (0x608970, `ret 4`)
    fn acquaintance_get(a: GPtr<u8>, i: i32) -> u32 = thiscall 0x608970 => unported;
    /// `CPedType::GetPedFlag(ePedType)` (0x608830, cdecl)
    fn ped_type_get_ped_flag(t: u32) -> u32 = cdecl 0x608830 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80, cdecl)
    fn groups_get_peds_group(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPedGroupIntelligence::SetScriptCommandTask(CPed*, const CTask&)` (0x5F8560, `ret 8`)
    fn pgi_set_script_command_task(i: GPtr<u8>, ped: GPtr<u8>, t: GPtr<u8>) = thiscall 0x5F8560 => unported;
    /// `CPedGroupIntelligence::Flush` (0x5F7350)
    fn pgi_flush(i: GPtr<u8>) = thiscall 0x5F7350 => unported;
    /// `CPedGroupMembership::RemoveMember(int32)` (0x5F80D0, `ret 4`)
    fn membership_remove_member(m: GPtr<u8>, i: i32) = thiscall 0x5F80D0 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32 primaryTaskIndex, CTask*, bool)` (0x4B0A00, `ret 0xC`)
    fn event_script_command_ctor(e: GPtr<u8>, idx: i32, t: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand` (`FUN_004b0a50`)
    fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
}

/// Drops the registered entity reference in `slot` (if any) and clears it.
fn release_reference(slot: GPtr<u8>) {
    let e = rd::<GPtr<u8>>(slot, 0);
    if !e.is_null() {
        entity_clean_up_old_reference(e, slot);
        wr::<u32>(slot, 0, 0);
    }
}

// ---- CPedIntelligence --------------------------------------------------------------------------------------------

/// `CPedIntelligence::AddTaskPhysResponse(CTask*, bool)` (0x600DC0, `ret 8`): `SetTask(task, 0, b)`.
#[hook(addr = 0x600DC0, conv = thiscall, name = "CPedIntelligence::AddTaskPhysResponse", expect = "8B 44 24 08 8B 54 24 04",
       unverified = "ai7-8", route = "L2", hazards = "heap-writes")]
pub fn intel_add_task_phys_response(this: GPtr<u8>, task: GPtr<u8>, b: u32) {
    tm_set_task(this.byte_add(4), task, 0, b);
}

/// `CPedIntelligence::Respects(CPed*)` (0x601C90, `ret 4`): the ped's acquaintance set 0 (respect, ped +0x4E0) has
/// the other ped type's flag. EAX is `GetPedFlag`'s with AL = the `setne` result.
#[hook(addr = 0x601C90, conv = thiscall, name = "CPedIntelligence::Respects", expect = "8B 09 56 6A 00",
       unverified = "ai7-8", route = "L2", hazards = "none")]
pub fn intel_respects(this: GPtr<u8>, other: GPtr<u8>) -> u32 {
    let acq = acquaintance_get(rd::<GPtr<u8>>(this, 0).byte_add(0x4E0), 0);
    let flag = ped_type_get_ped_flag(rd::<u32>(other, 0x598));
    (flag & 0xFFFF_FF00) | u32::from(acq & flag != 0)
}

/// `CPedIntelligence::IsInACarOrEnteringOne` (0x601CC0): the vehicle of an active enter-car-as-driver (0x2BD) or
/// -as-passenger (0x2BC) task (+0xC), else of an active 0x2C5 task (+8), else null.
#[hook(addr = 0x601CC0, conv = thiscall, name = "CPedIntelligence::IsInACarOrEnteringOne", expect = "56 8D 71 04 68 BD 02 00 00",
       unverified = "ai7-8", route = "L2", hazards = "none")]
pub fn intel_is_in_a_car_or_entering_one(this: GPtr<u8>) -> u32 {
    let tm = this.byte_add(4);
    let mut t = tm_find_active_task_by_type(tm, 0x2BD);
    if t.is_null() {
        t = tm_find_active_task_by_type(tm, 0x2BC);
    }
    if !t.is_null() {
        return rd::<u32>(t, 0xC);
    }
    let t = tm_find_active_task_by_type(tm, 0x2C5);
    if t.is_null() { 0 } else { rd::<u32>(t, 8) }
}

/// `CPedIntelligence::IsInterestingEntity(CEntity*)` (0x6020A0, `ret 4`): 1 when one of the three interesting
/// entities (+0x288) is `e`, else 0 (EAX whole: the loop index's upper bytes are 0).
#[hook(addr = 0x6020A0, conv = thiscall, name = "CPedIntelligence::IsInterestingEntity", expect = "8B 54 24 04 33 C0",
       unverified = "ai7-8", route = "L2", hazards = "none")]
pub fn intel_is_interesting_entity(this: GPtr<u8>, e: GPtr<u8>) -> u32 {
    u32::from((0..3).any(|k| rd::<GPtr<u8>>(this, 0x288 + k * 4) == e))
}

/// `CPedIntelligence::RemoveAllInterestingEntities` (0x602320): every interesting entity reference is dropped and
/// cleared.
#[hook(addr = 0x602320, conv = thiscall, name = "CPedIntelligence::RemoveAllInterestingEntities",
       expect = "56 57 8D B1 88 02 00 00", unverified = "ai7-8", route = "L2", hazards = "heap-writes")]
pub fn intel_remove_all_interesting_entities(this: GPtr<u8>) {
    for k in 0..3 {
        let slot = this.byte_add(0x288 + k * 4);
        let e = rd::<GPtr<u8>>(slot, 0);
        if !e.is_null() {
            entity_clean_up_old_reference(e, slot);
        }
        wr::<u32>(slot, 0, 0);
    }
}

/// `CPedIntelligence::AddTaskPrimaryMaybeInGroup(CTask*, bool)` (0x600E20, `ret 8`): a non-player ped in a group
/// hands the task to the group as its script command task and deletes it (when non-null); otherwise a stack
/// `CEventScriptCommand(3, task, b)` (Rust local) is added to the ped's event group.
#[hook(addr = 0x600E20, conv = thiscall, name = "CPedIntelligence::AddTaskPrimaryMaybeInGroup", expect = "64 A1 00 00 00 00",
       unverified = "ai7-8", route = "L2", hazards = "heap-writes")]
pub fn intel_add_task_primary_maybe_in_group(this: GPtr<u8>, task: GPtr<u8>, b: u32) {
    let grp = groups_get_peds_group(rd::<GPtr<u8>>(this, 0));
    if ped_is_player(rd::<GPtr<u8>>(this, 0)) == 0 && !grp.is_null() {
        pgi_set_script_command_task(grp.byte_add(0x30), rd::<GPtr<u8>>(this, 0), task);
        if !task.is_null() {
            v_delete(task, 1);
        }
        return;
    }
    let mut ev = [0u32; 8];
    let e: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    event_script_command_ctor(e, 3, task, b);
    event_group_add(this.byte_add(0x68), e, 0);
    event_script_command_dtor(e);
}

// ---- CPedGroup / CPedGroups --------------------------------------------------------------------------------------

/// `CPedGroup::IsAnyoneUsingCar(const CVehicle*)` (0x5F7DB0, `ret 4`): AL = 1 when a member sits in the vehicle
/// (+0x58C with in-vehicle flag 0x100 of +0x46C) or has an enter-car task (`FindTaskByType` 0x2BD, then 0x2BC) for
/// it.
pub fn group_is_anyone_using_car(this: GPtr<u8>, veh: GPtr<u8>) -> u8 {
    for k in 0..8 {
        let m = rd::<GPtr<u8>>(this, 0xC + k * 4);
        if m.is_null() {
            continue;
        }
        let v = rd::<GPtr<u8>>(m, 0x58C);
        if !v.is_null() && rd::<u32>(m, 0x46C) & 0x100 != 0 && v == veh {
            return 1;
        }
        for ty in [0x2BD, 0x2BC] {
            let t = intel_find_task_by_type(rd::<GPtr<u8>>(m, 0x47C), ty);
            if t.is_null() {
                if ty == 0x2BC {
                    break;
                }
                continue;
            }
            if rd::<GPtr<u8>>(t, 0xC) == veh {
                return 1;
            }
        }
    }
    0
}

/// `CPedGroup::RemoveAllFollowers` (0x5FB7D0): `RemoveMember(i)` for every present member except the leader (7).
#[hook(addr = 0x5FB7D0, conv = thiscall, name = "CPedGroup::RemoveAllFollowers", expect = "53 56 8D 59 08",
       unverified = "ai7-8", route = "L2", hazards = "heap-writes")]
pub fn group_remove_all_followers(this: GPtr<u8>) {
    let ms = this.byte_add(8);
    for i in 0..8 {
        if rd::<u32>(ms, 4 + i as isize * 4) != 0 && i != 7 {
            membership_remove_member(ms, i);
        }
    }
}

/// `CPedGroup::~CPedGroup` (0x5FC190): flushes the group intelligence, then removes every present member.
#[hook(addr = 0x5FC190, conv = thiscall, name = "CPedGroup::Destructor", expect = "6A FF 68 BB E4 83 00",
       unverified = "ai7-8", route = "L2", hazards = "heap-writes")]
pub fn group_destructor(this: GPtr<u8>) {
    pgi_flush(this.byte_add(0x30));
    let ms = this.byte_add(8);
    for i in 0..8 {
        if rd::<u32>(ms, 4 + i as isize * 4) != 0 {
            membership_remove_member(ms, i);
        }
    }
}

/// `CPedGroups::CleanUpForShutDown` (0x5FB930): every group removes all members, flushes its intelligence and
/// clears its +0x2D0 flag.
#[hook(addr = 0x5FB930, conv = cdecl, name = "CPedGroups::CleanUpForShutDown", expect = "51 55 56 57 BF 28 99 C0 00",
       unverified = "ai7-8", route = "split", hazards = "heap-writes", note = "shutdown code")]
pub fn groups_clean_up_for_shut_down() {
    for gi in 0..8 {
        let grp: GPtr<u8> = GPtr::from_addr(GROUPS + gi * GROUP_SIZE);
        let ms = grp.byte_add(8);
        for i in 0..8 {
            if rd::<u32>(ms, 4 + i as isize * 4) != 0 {
                membership_remove_member(ms, i);
            }
        }
        pgi_flush(grp.byte_add(0x30));
        wr::<u8>(grp, 0x2D0, 0);
    }
}

// ---- CTaskSimpleHoldEntity ---------------------------------------------------------------------------------------

/// `CTaskSimpleHoldEntity::CanThrowEntity` (0x691700): AL = 1 with an entity (+8) held with bone frame 0x10 (+0x19)
/// and hold type 6 (+0x18). EAX whole: the entity pointer's upper bytes (0 without an entity).
#[hook(addr = 0x691700, conv = thiscall, name = "CTaskSimpleHoldEntity::CanThrowEntity", expect = "8B 41 08 85 C0",
       unverified = "ai7-8", route = "L2", hazards = "none")]
pub fn hold_entity_can_throw_entity(this: GPtr<u8>) -> u32 {
    let e = rd::<u32>(this, 8);
    if e == 0 {
        return 0;
    }
    (e & 0xFFFF_FF00) | u32::from(rd::<u8>(this, 0x19) == 0x10 && rd::<u8>(this, 0x18) == 6)
}

/// `CTaskSimpleHoldEntity::ReleaseEntity` (0x6916E0): drops the held entity's reference (+8) and clears it.
#[hook(addr = 0x6916E0, conv = thiscall, name = "CTaskSimpleHoldEntity::ReleaseEntity", expect = "56 8D 71 08 8B 0E",
       unverified = "ai7-8", route = "L2", hazards = "heap-writes")]
pub fn hold_entity_release_entity(this: GPtr<u8>) {
    release_reference(this.byte_add(8));
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_INTEL_ADD_TASK_PHYS_RESPONSE,
    HOOK_INTEL_RESPECTS,
    HOOK_INTEL_IS_IN_A_CAR_OR_ENTERING_ONE,
    HOOK_INTEL_IS_INTERESTING_ENTITY,
    HOOK_INTEL_REMOVE_ALL_INTERESTING_ENTITIES,
    HOOK_INTEL_ADD_TASK_PRIMARY_MAYBE_IN_GROUP,
    HOOK_GROUP_REMOVE_ALL_FOLLOWERS,
    HOOK_GROUP_DESTRUCTOR,
    HOOK_GROUPS_CLEAN_UP_FOR_SHUT_DOWN,
    HOOK_HOLD_ENTITY_CAN_THROW_ENTITY,
    HOOK_HOLD_ENTITY_RELEASE_ENTITY,
];
