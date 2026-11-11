//! Small executed AI functions (PORT-AI-2, samp-priority order): `CTaskSimplePause`, `CEventEditableResponse` and
//! `CEventScriptCommand` pieces, `CTaskComplexSequence` constructor/flush. Ported from the 1.0 US bytes.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    fn v_delete(t: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CTask::Clone` (vtable +4)
    fn v_task_clone(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CPed::IsAlive` (0x5E0170)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CTaskSimple::~CTaskSimple` (0x61A3A0)
    fn task_simple_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `CTaskSimple::ProcessPed(CPed*)` (vtable +0x1C)
    fn v_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall vslot 0x1C => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`
fn time_ms() -> u32 {
    g::<u32>(0xB7CB84).read()
}

/// `CTaskSimplePause::CTaskSimplePause(int32 time)` (0x48E750, a `.HOODLUM` stub; body at 0x1562630, `ret 4`):
/// `CTaskSimple` base, vtable 0x85A278, timer start/interval 0, the two timer flags clear, +0x14 = time.
#[hook(addr = 0x48E750, conv = thiscall, name = "CTaskSimplePause::Constructor", expect = "E9 DB 3E 0D 01")]
pub fn pause_ctor(this: GPtr<u8>, time: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0, 0x85A278);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<u8>(this, 0x10, 0);
    wr::<u8>(this, 0x11, 0);
    wr::<u32>(this, 0x14, time);
    this
}

/// `CTaskSimplePause::MakeAbortable(CPed*, int32, const CEvent*)` (0x48E810, `ret 0xC`): restarts the timer with
/// interval -1 (start = now, +0x10 set) whatever the priority; true.
#[hook(addr = 0x48E810, conv = thiscall, name = "CTaskSimplePause::MakeAbortable", expect = "A1 84 CB B7 00")]
pub fn pause_make_abortable(this: GPtr<u8>, _ped: GPtr<u8>, _prio: i32, _ev: GPtr<u8>) -> u8 {
    wr::<u32>(this, 8, time_ms());
    wr::<u32>(this, 0xC, 0xFFFF_FFFF);
    wr::<u8>(this, 0x10, 1);
    1
}

/// `CTaskSimplePause::ProcessPed(CPed*)` (0x48E830, `ret 4`): starts the timer (now, +0x14) on the first call;
/// a pending restart (+0x11) resets the start; true once `now >= start + interval` (unsigned, wrapping sum).
#[hook(addr = 0x48E830, conv = thiscall, name = "CTaskSimplePause::ProcessPed", expect = "8A 41 10 84 C0")]
pub fn pause_process_ped(this: GPtr<u8>, _ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x10) == 0 {
        let iv = rd::<u32>(this, 0x14);
        wr::<u32>(this, 8, time_ms());
        wr::<u32>(this, 0xC, iv);
        wr::<u8>(this, 0x10, 1);
        if rd::<u8>(this, 0x10) == 0 {
            return 0;
        }
    }
    if rd::<u8>(this, 0x11) != 0 {
        wr::<u32>(this, 8, time_ms());
        wr::<u8>(this, 0x11, 0);
    }
    let end = rd::<u32>(this, 0xC).wrapping_add(rd::<u32>(this, 8));
    (time_ms() >= end) as u8
}

/// `CEventEditableResponse::CEventEditableResponse` (0x4AC450): CEvent fields +4 = 0, +8 = 0, vtable 0x85AB80,
/// +0xC = 1 (add to event group), +0xE = 200 (no response task), +0x10 = -1; returns this.
#[hook(addr = 0x4AC450, conv = thiscall, name = "CEventEditableResponse::Constructor", expect = "8B C1 33 C9 89 48 04")]
pub fn editable_response_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u32>(this, 0, 0x85AB80);
    wr::<u8>(this, 0xC, 1);
    wr::<u16>(this, 0xE, 0xC8);
    wr::<u16>(this, 0x10, 0xFFFF);
    this
}

/// `CEventEditableResponse::WillRespond` (0x4AC490): EAX = (+0xE != 200).
#[hook(addr = 0x4AC490, conv = thiscall, name = "CEventEditableResponse::WillRespond", expect = "33 C0 66 81 79 0E C8 00")]
pub fn editable_response_will_respond(this: GPtr<u8>) -> u32 {
    (rd::<u16>(this, 0xE) != 0xC8) as u32
}

/// `CEventScriptCommand::CEventScriptCommand(int32 primaryTaskIndex, CTask*, bool affectsDeadPeds)` (0x4B0A00,
/// `ret 0xC`): +4 = 0, +8 = 0, +0xC = index, vtable 0x85B378, +0x10 = task, +0x14 = flag; returns this.
#[hook(addr = 0x4B0A00, conv = thiscall, name = "CEventScriptCommand::Constructor", expect = "8B 54 24 08 8B C1")]
pub fn script_command_ctor(this: GPtr<u8>, index: u32, task: u32, dead: u8) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u32>(this, 0xC, index);
    wr::<u32>(this, 0, 0x85B378);
    wr::<u32>(this, 0x10, task);
    wr::<u8>(this, 0x14, dead);
    this
}

/// `CEventScriptCommand::CloneScriptTask` (0x4B0AA0): the task's `Clone()` (tail call), null without a task.
#[hook(addr = 0x4B0AA0, conv = thiscall, name = "CEventScriptCommand::CloneScriptTask", expect = "8B 49 10 33 C0")]
pub fn script_command_clone_script_task(this: GPtr<u8>) -> GPtr<u8> {
    let t = rd::<GPtr<u8>>(this, 0x10);
    if t.is_null() { GPtr::NULL } else { v_task_clone(t) }
}

/// `CEventScriptCommand::AffectsPed(CPed*)` (0x4B0AF0, `ret 4`): EAX = 1 for a living ped or with the
/// affects-dead-peds flag (+0x14), else 0.
#[hook(addr = 0x4B0AF0, conv = thiscall, name = "CEventScriptCommand::AffectsPed", expect = "56 8B F1 8B 4C 24 08")]
pub fn script_command_affects_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    (ped_is_alive(ped) != 0 || rd::<u8>(this, 0x14) != 0) as u32
}

/// `CTaskComplexSequence::CTaskComplexSequence` (0x632BD0; continues through the island 0x4015DA that writes the
/// vtable 0x86E200): `CTaskComplex` base, +0xC / +0x30 / +0x34 / +0x3C = 0, +0x38 = 0 (byte), the 8 task slots
/// (+0x10) zeroed; returns this.
#[hook(addr = 0x632BD0, conv = thiscall, name = "CTaskComplexSequence::Constructor", expect = "56 57 8B F1 E8 D7 77 FE FF")]
pub fn sequence_ctor(this: GPtr<u8>) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0xC, 0);
    wr::<u32>(this, 0x30, 0);
    wr::<u32>(this, 0x34, 0);
    wr::<u8>(this, 0x38, 0);
    wr::<u32>(this, 0x3C, 0);
    wr::<u32>(this, 0, 0x86E200);
    for i in 0..8 {
        wr::<u32>(this, 0x10 + i * 4, 0);
    }
    this
}

/// `CTaskComplexSequence::Flush` (0x632C10): deletes and clears the 8 task slots, then +0xC (current index),
/// +0x30 (repeat mode) and +0x34 (repeated times) = 0.
#[hook(addr = 0x632C10, conv = thiscall, name = "CTaskComplexSequence::Flush", expect = "53 56 8B D9 57")]
pub fn sequence_flush(this: GPtr<u8>) {
    for i in 0..8 {
        let off = 0x10 + i * 4;
        let t = rd::<GPtr<u8>>(this, off);
        if !t.is_null() {
            v_delete(t, 1);
        }
        wr::<u32>(this, off, 0);
    }
    wr::<u32>(this, 0xC, 0);
    wr::<u32>(this, 0x30, 0);
    wr::<u32>(this, 0x34, 0);
}

/// `CTaskSimpleIKManager::CTaskSimpleIKManager` (0x6337F0): `CTaskSimple` base, vtable 0x86E358, the four chain
/// slots (+8) zeroed, +0x18 (aborting) clear; returns this.
#[hook(addr = 0x6337F0, conv = thiscall, name = "CTaskSimpleIKManager::Constructor", expect = "56 8B F1 E8 98 6B FE FF")]
pub fn ik_manager_ctor(this: GPtr<u8>) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0, 0x86E358);
    for i in 0..4 {
        wr::<u32>(this, 8 + i * 4, 0);
    }
    wr::<u8>(this, 0x18, 0);
    this
}

/// `CTaskSimpleIKManager::GetTaskType` (0x633820): 0x109.
#[hook(addr = 0x633820, conv = thiscall, name = "CTaskSimpleIKManager::GetTaskType", expect = "B8 09 01 00 00")]
pub fn ik_manager_get_task_type(_this: GPtr<u8>) -> i32 {
    0x109
}

/// `CTaskSimpleIKManager::~CTaskSimpleIKManager` (0x633830): vtable 0x86E358, deletes the non-null chain tasks
/// (slots not cleared), then `CTaskSimple::~CTaskSimple`.
#[hook(addr = 0x633830, conv = thiscall, name = "CTaskSimpleIKManager::Destructor", expect = "6A FF 68 58 07 84 00")]
pub fn ik_manager_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86E358);
    for i in 0..4 {
        let t = rd::<GPtr<u8>>(this, 8 + i * 4);
        if !t.is_null() {
            v_delete(t, 1);
        }
    }
    task_simple_dtor(this);
}

/// `CTaskSimpleIKManager::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x6338A0, `ret 0xC`): priority 2
/// (immediate) deletes and clears every chain task and returns true; otherwise sets +0x18 and returns false.
#[hook(addr = 0x6338A0, conv = thiscall, name = "CTaskSimpleIKManager::MakeAbortable", expect = "83 7C 24 08 02")]
pub fn ik_manager_make_abortable(this: GPtr<u8>, _ped: GPtr<u8>, prio: i32, _ev: GPtr<u8>) -> u8 {
    if prio != 2 {
        wr::<u8>(this, 0x18, 1);
        return 0;
    }
    for i in 0..4 {
        let t = rd::<GPtr<u8>>(this, 8 + i * 4);
        if !t.is_null() {
            v_delete(t, 1);
            wr::<u32>(this, 8 + i * 4, 0);
        }
    }
    1
}

/// `CTaskSimpleIKManager::ProcessPed(CPed*)` (0x6338E0, `ret 4`): true once aborting (+0x18); else runs every
/// chain task's `ProcessPed`, deleting and clearing the finished ones; true when all slots finished or were empty.
#[hook(addr = 0x6338E0, conv = thiscall, name = "CTaskSimpleIKManager::ProcessPed", expect = "8A 41 18 84 C0")]
pub fn ik_manager_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x18) != 0 {
        return 1;
    }
    let mut done = true;
    for i in 0..4 {
        let off = 8 + i * 4;
        let t = rd::<GPtr<u8>>(this, off);
        if t.is_null() {
            continue;
        }
        if v_process_ped(t, ped) != 0 {
            let t = rd::<GPtr<u8>>(this, off);
            if !t.is_null() {
                v_delete(t, 1);
            }
            wr::<u32>(this, off, 0);
        } else {
            done = false;
        }
    }
    done as u8
}

/// `CTaskSimpleIKManager::AddIKChainTask(CTaskSimpleIKChain*, int32 slot)` (0x633940, `ret 8`): a slot >= 0 is
/// written directly; a negative slot takes the first empty one (none: nothing stored, returns 0); returns the
/// slot index (EAX).
#[hook(addr = 0x633940, conv = thiscall, name = "CTaskSimpleIKManager::AddIKChainTask", expect = "8B 44 24 08 85 C0")]
pub fn ik_manager_add_ik_chain_task(this: GPtr<u8>, task: u32, s: i32) -> i32 {
    let mut slot = s;
    if slot < 0 {
        match (0..4).find(|i| rd::<u32>(this, 8 + (*i as isize) * 4) == 0) {
            Some(i) => slot = i,
            None => return 0,
        }
    }
    wr::<u32>(this, 8 + (slot as isize).wrapping_mul(4), task);
    slot
}

/// `CTaskSimpleIKManager::GetTaskAtSlot(int32)` (0x6339B0, `ret 4`): the chain task of that slot.
#[hook(addr = 0x6339B0, conv = thiscall, name = "CTaskSimpleIKManager::GetTaskAtSlot", expect = "8B 44 24 04 8B 44 81 08")]
pub fn ik_manager_get_task_at_slot(this: GPtr<u8>, s: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 8 + (s as isize).wrapping_mul(4))
}

/// `CPedScriptedTaskRecordData::CPedScriptedTaskRecordData` (0x608330): +0 = -1 (opcode), +4 / +8 / +0xC = 0,
/// +0x10 = 3 (status); returns this.
#[hook(addr = 0x608330, conv = thiscall, name = "CPedScriptedTaskRecordData::Constructor", expect = "8B C1 33 C9 C7 00 FF FF FF FF")]
pub fn record_data_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<i32>(this, 0, -1);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<u32>(this, 0x10, 3);
    this
}

/// `CPedScriptedTaskRecordData::Flush` (0x608350): releases the ped reference (+0xC, `CleanUpOldReference`),
/// then +0xC / +4 / +8 = 0, +0x10 = 3, +0 = -1.
#[hook(addr = 0x608350, conv = thiscall, name = "CPedScriptedTaskRecordData::Flush", expect = "56 8B F1 8B 4E 0C")]
pub fn record_data_flush(this: GPtr<u8>) {
    let p = rd::<GPtr<u8>>(this, 0xC);
    if !p.is_null() {
        entity_clean_up_old_reference(p, this.byte_add(0xC));
    }
    wr::<u32>(this, 0xC, 0);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0x10, 3);
    wr::<i32>(this, 0, -1);
}

/// `CPedScriptedTaskRecordData::Set(CPed*, int32 opcode, CEventScriptCommand*)` (0x608390, `ret 0xC`): the flush
/// above (status 3, opcode -1), the ped stored at +0xC and registered (`RegisterReference`), then +8 = 0, +0x10 =
/// 0, +0 = opcode, +4 = event.
#[hook(addr = 0x608390, conv = thiscall, name = "CPedScriptedTaskRecordData::Set-CEventScriptCommand", expect = "53 56 8B F1 8B 4E 0C")]
pub fn record_data_set_event(this: GPtr<u8>, ped: GPtr<u8>, opcode: i32, ev: u32) {
    let p = rd::<GPtr<u8>>(this, 0xC);
    if !p.is_null() {
        entity_clean_up_old_reference(p, this.byte_add(0xC));
    }
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0x10, 3);
    wr::<i32>(this, 0, -1);
    wr::<GPtr<u8>>(this, 0xC, ped);
    entity_register_reference(ped, this.byte_add(0xC));
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0x10, 0);
    wr::<i32>(this, 0, opcode);
    wr::<u32>(this, 4, ev);
}

/// `CPedScriptedTaskRecordData::AssociateWithTask(CTask*)` (0x608520, `ret 4`): +4 = 0, +8 = task, +0x10 = 1.
#[hook(addr = 0x608520, conv = thiscall, name = "CPedScriptedTaskRecordData::AssociateWithTask", expect = "8B 44 24 04 C7 41 04 00 00 00 00")]
pub fn record_data_associate_with_task(this: GPtr<u8>, task: u32) {
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, task);
    wr::<u32>(this, 0x10, 1);
}

/// `CPedScriptedTaskRecord::ms_scriptedTasks` (0xC0B1E8, 128 records of 0x14 bytes).
const SCRIPTED_TASKS: usize = 0xC0B1E8;

/// `CPedScriptedTaskRecord::GetVacantSlot` (0x608540, static; continues through the island 0x4021D7): the first
/// record whose event (+4), task (+8) and ped (+0xC) are all null, else -1.
#[hook(addr = 0x608540, conv = cdecl, name = "CPedScriptedTaskRecord::GetVacantSlot", expect = "83 CA FF 33 C0")]
pub fn scripted_task_record_get_vacant_slot() -> i32 {
    for i in 0..128 {
        let r = g::<u8>(SCRIPTED_TASKS + i * 0x14);
        if rd::<u32>(r, 4) == 0 && rd::<u32>(r, 8) == 0 && rd::<u32>(r, 0xC) == 0 {
            return i as i32;
        }
    }
    -1
}

/// `CPedScriptedTaskRecord::GetRecordAssociatedWithEvent(CEvent*)` (0x608580, static): the record whose event
/// (+4) is the argument, else null.
#[hook(addr = 0x608580, conv = cdecl, name = "CPedScriptedTaskRecord::GetRecordAssociatedWithEvent", expect = "56 8B 74 24 08")]
pub fn scripted_task_record_get_record_associated_with_event(ev: u32) -> GPtr<u8> {
    for i in 0..128 {
        let r = g::<u8>(SCRIPTED_TASKS + i * 0x14);
        if rd::<u32>(r, 4) == ev {
            return r;
        }
    }
    GPtr::NULL
}

/// `CPedGroupMembership::GetMember(int32)` (0x5F69B0, `ret 4`): member slot `i` (+4 + 4i).
#[hook(addr = 0x5F69B0, conv = thiscall, name = "CPedGroupMembership::GetMember", expect = "8B 44 24 04 8B 44 81 04")]
pub fn membership_get_member(this: GPtr<u8>, i: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 4 + (i as isize).wrapping_mul(4))
}

/// `CPedGroupMembership::IsFollower(const CPed*)` (0x5F69E0, `ret 4`): a non-null ped in one of the 7 member slots.
#[hook(addr = 0x5F69E0, conv = thiscall, name = "CPedGroupMembership::IsFollower", expect = "56 8B 74 24 08")]
pub fn membership_is_follower(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped.is_null() {
        return 0;
    }
    (0..7).any(|i| rd::<GPtr<u8>>(this, 4 + i * 4) == ped) as u8
}

/// `CPedGroupMembership::IsMember(const CPed*)` (0x5F6A10, `ret 4`): EAX = 1 for a non-null ped in the member
/// slots or the leader slot (+0x20), else 0.
#[hook(addr = 0x5F6A10, conv = thiscall, name = "CPedGroupMembership::IsMember", expect = "56 8B 74 24 08")]
pub fn membership_is_member(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    if ped.is_null() {
        return 0;
    }
    ((0..7).any(|i| rd::<GPtr<u8>>(this, 4 + i * 4) == ped) || rd::<GPtr<u8>>(this, 0x20) == ped) as u32
}

/// `CPedGroupMembership::CountMembersExcludingLeader` (0x5F6AA0): the number of non-null member slots (+4..+0x1C).
#[hook(addr = 0x5F6AA0, conv = thiscall, name = "CPedGroupMembership::CountMembersExcludingLeader", expect = "8B 51 04 33 C0")]
pub fn membership_count_members_excluding_leader(this: GPtr<u8>) -> i32 {
    (0..7)
        .filter(|i| !rd::<GPtr<u8>>(this, 4 + i * 4).is_null())
        .count() as i32
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_PAUSE_CTOR,
    HOOK_PAUSE_MAKE_ABORTABLE,
    HOOK_PAUSE_PROCESS_PED,
    HOOK_EDITABLE_RESPONSE_CTOR,
    HOOK_EDITABLE_RESPONSE_WILL_RESPOND,
    HOOK_SCRIPT_COMMAND_CTOR,
    HOOK_SCRIPT_COMMAND_CLONE_SCRIPT_TASK,
    HOOK_SCRIPT_COMMAND_AFFECTS_PED,
    HOOK_SEQUENCE_CTOR,
    HOOK_SEQUENCE_FLUSH,
    HOOK_IK_MANAGER_CTOR,
    HOOK_IK_MANAGER_GET_TASK_TYPE,
    HOOK_IK_MANAGER_DTOR,
    HOOK_IK_MANAGER_MAKE_ABORTABLE,
    HOOK_IK_MANAGER_PROCESS_PED,
    HOOK_IK_MANAGER_ADD_IK_CHAIN_TASK,
    HOOK_IK_MANAGER_GET_TASK_AT_SLOT,
    HOOK_RECORD_DATA_CTOR,
    HOOK_RECORD_DATA_FLUSH,
    HOOK_RECORD_DATA_SET_EVENT,
    HOOK_RECORD_DATA_ASSOCIATE_WITH_TASK,
    HOOK_SCRIPTED_TASK_RECORD_GET_VACANT_SLOT,
    HOOK_SCRIPTED_TASK_RECORD_GET_RECORD_ASSOCIATED_WITH_EVENT,
    HOOK_MEMBERSHIP_GET_MEMBER,
    HOOK_MEMBERSHIP_IS_FOLLOWER,
    HOOK_MEMBERSHIP_IS_MEMBER,
    HOOK_MEMBERSHIP_COUNT_MEMBERS_EXCLUDING_LEADER,
];
