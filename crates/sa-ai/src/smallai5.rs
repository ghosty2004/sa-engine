//! Small bodies of PORT-AI-5's families: `CAcquaintance` set/clear, `CObjectScanner`, `CPedScanner`,
//! `CDecisionMakerTypesFileLoader::ReStart`, `CPedScriptedTaskRecord::GetStatus`, `CTaskComplexSequence`
//! `MakeAbortable`/`AddTask`, the `CTaskComplexBeInGroup` constructor and the `CInformGroupEventQueue` statics.
//! Ported from the 1.0 US bytes (the exe wins over gta-reversed). Every call to another catalogued function goes
//! through `orig!` at the original address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CInformGroupEventQueue::ms_entries` (8 x 0x10: +0 ped, +4, +8 event, +0xC time)
const INFORM_GROUP_QUEUE: usize = 0xA9B018;

orig! {
    /// `CPed::IsAlive` (0x5E0170)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CEntityScanner::ScanForEntitiesInRange(int32, const CPed&)` (0x5FFA20, `ret 8`)
    fn entity_scanner_scan(s: GPtr<u8>, ty: u32, ped: GPtr<u8>) = thiscall 0x5FFA20 => unported;
    /// `CDecisionMakerTypes::GetInstance` (0x4684F0, cdecl)
    fn decision_maker_types_get_instance() -> GPtr<u8> = cdecl 0x4684F0 => unported;
    /// `CDecisionMakerTypes::RemoveDecisionMaker(int32)` (0x6043A0, `ret 4`)
    fn decision_maker_types_remove(t: GPtr<u8>, i: i32) = thiscall 0x6043A0 => unported;
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CInformGroupEvent::Process` (0x4B29E0)
    fn inform_group_event_process(e: GPtr<u8>) = thiscall 0x4B29E0 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: u32, e: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// scalar deleting destructor (vtable +0)
    fn v_delete(t: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
}

/// `CAcquaintance::ClearAsAcquaintance(int32 type, uint32 bits)` (0x608980, `ret 8`): clears the bits in set `type`
/// when any is set.
#[hook(addr = 0x608980, conv = thiscall, name = "CAcquaintance::ClearAsAcquaintance", expect = "8B 44 24 08 56")]
pub fn acquaintance_clear_as(this: GPtr<u8>, ty: i32, bits: u32) {
    let off = 4 * ty as isize;
    let d = rd::<u32>(this, off);
    if bits & d != 0 {
        wr::<u32>(this, off, !bits & d);
    }
}

/// `CAcquaintance::SetAsAcquaintance(int32 type, uint32 bits)` (0x608DA0, `ret 8`): when none of the bits is set in
/// set `type`, sets them there and clears them in the other four sets (only where any is set).
#[hook(addr = 0x608DA0, conv = thiscall, name = "CAcquaintance::SetAsAcquaintance", expect = "8B 44 24 08 56")]
pub fn acquaintance_set_as(this: GPtr<u8>, ty: i32, bits: u32) {
    let off = 4 * ty as isize;
    let d = rd::<u32>(this, off);
    if bits & d != 0 {
        return;
    }
    wr::<u32>(this, off, d | bits);
    for k in 0..5 {
        if k == ty {
            continue;
        }
        let v = rd::<u32>(this, 4 * k as isize);
        if bits & v != 0 {
            wr::<u32>(this, 4 * k as isize, !bits & v);
        }
    }
}

/// `CObjectScanner::CObjectScanner` (0x606010): +4 = 0, 16 zeroed slots (+0xC), +0x4C = 0, +8 = 0x10, vtable
/// 0x86CD8C. Returns this.
#[hook(addr = 0x606010, conv = thiscall, name = "CObjectScanner::CObjectScanner", expect = "8B D1 57 C7 42 04 00 00 00 00")]
pub fn object_scanner_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    for i in 0..16 {
        wr::<u32>(this, 0xC + 4 * i, 0);
    }
    wr::<u32>(this, 0x4C, 0);
    wr::<u32>(this, 8, 0x10);
    wr::<u32>(this, 0, 0x86CD8C);
    this
}

/// `CObjectScanner::ScanForObjectsInRange(const CPed&)` (0x5FFF30, `ret 4`): for a live ped not sitting in a vehicle,
/// `ScanForEntitiesInRange(2, ped)`.
#[hook(addr = 0x5FFF30, conv = thiscall, name = "CObjectScanner::ScanForObjectsInRange", expect = "56 8B 74 24 08")]
pub fn object_scanner_scan_for_objects_in_range(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u32>(ped, 0x58C) != 0 && rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        return;
    }
    if ped_is_alive(ped) != 0 {
        entity_scanner_scan(this, 2, ped);
    }
}

/// `CPedScanner::GetClosestPedInRange` (0x5FFF20): +0x4C.
/// Not hookable: `mov eax, [ecx+0x4C]; ret` is 4 bytes.
pub fn ped_scanner_get_closest_ped_in_range(this: GPtr<u8>) -> u32 {
    rd::<u32>(this, 0x4C)
}

/// `CDecisionMakerTypesFileLoader::ReStart` (0x607D00, static): removes decision makers 10..19 (the argument pushed
/// for `GetInstance` is consumed by `RemoveDecisionMaker`).
#[hook(addr = 0x607D00, conv = cdecl, name = "CDecisionMakerTypesFileLoader::ReStart", expect = "56 BE 0A 00 00 00")]
pub fn file_loader_restart() {
    for i in 10..20 {
        let t = decision_maker_types_get_instance();
        decision_maker_types_remove(t, i);
    }
}

/// `CPedScriptedTaskRecord::GetStatus(const CPed*, int32 opcode)` (0x608710, cdecl): the status (+0x10) of the
/// first of the 128 records (0xC0B1E8) for the ped whose opcode matches (any opcode for -1); -1 when none.
#[hook(addr = 0x608710, conv = cdecl, name = "CPedScriptedTaskRecord::GetStatus-opcode", expect = "8B 54 24 08 56")]
pub fn scripted_task_record_get_status(ped: u32, opcode: i32) -> u32 {
    for i in 0..128usize {
        let r: GPtr<u8> = GPtr::from_addr(0xC0B1E8 + 0x14 * i);
        if (opcode == -1 || rd::<i32>(r, 0) == opcode) && rd::<u32>(r, 0xC) == ped {
            return rd::<u32>(r, 0x10);
        }
    }
    0xFFFF_FFFF
}

/// `CTaskComplexSequence::MakeAbortable(CPed*, int32, const CEvent*)` (0x632C00, `ret 0xC`): the current sub-task's
/// `MakeAbortable` (tail jump through the vtable).
#[hook(addr = 0x632C00, conv = thiscall, name = "CTaskComplexSequence::MakeAbortable", expect = "8B 49 08 8B 01")]
pub fn sequence_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: u32, e: u32) -> u8 {
    v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, prio, e)
}

/// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10, `ret 4`): stores the task in the first free of the 8 slots
/// (+0x10) and returns true (AL); when all are used the task is deleted and false returned.
#[hook(addr = 0x632D10, conv = thiscall, name = "CTaskComplexSequence::AddTask-0", expect = "33 C0 8D 51 10")]
pub fn sequence_add_task(this: GPtr<u8>, task: GPtr<u8>) -> u8 {
    for i in 0..8 {
        if rd::<u32>(this, 0x10 + 4 * i) == 0 {
            wr(this, 0x10 + 4 * i, task);
            return 1;
        }
    }
    if !task.is_null() {
        v_delete(task, 1);
    }
    0
}

/// `CTaskComplexBeInGroup::CTaskComplexBeInGroup(int32 groupId, bool isLeader)` (0x632E50, `ret 8`): `CTaskComplex`
/// base, vtable 0x86E22C, +0xC group id, +0x10 flag byte, +0x14/+0x18/+0x20 = 0, +0x1C = 0xC8, +0x24 = -1. Returns
/// this.
#[hook(addr = 0x632E50, conv = thiscall, name = "CTaskComplexBeInGroup::Constructor", expect = "56 8B F1 E8 58 75 FE FF")]
pub fn be_in_group_ctor(this: GPtr<u8>, id: u32, leader: u8) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0xC, id);
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(this, 0x18, 0);
    wr::<u32>(this, 0x20, 0);
    wr::<u32>(this, 0, 0x86E22C);
    wr::<u8>(this, 0x10, leader);
    wr::<u32>(this, 0x1C, 0xC8);
    wr::<u32>(this, 0x24, 0xFFFF_FFFF);
    this
}

/// `CInformGroupEventQueue::Flush` (0x4AC410, static): every entry releases its ped reference, deletes its event and
/// gets time -1.
#[hook(addr = 0x4AC410, conv = cdecl, name = "CInformGroupEventQueue::Flush", expect = "56 BE 18 B0 A9 00")]
pub fn inform_group_event_queue_flush() {
    for i in 0..8usize {
        let e: GPtr<u8> = GPtr::from_addr(INFORM_GROUP_QUEUE + 0x10 * i);
        let p = rd::<GPtr<u8>>(e, 0);
        if !p.is_null() {
            entity_clean_up_old_reference(p, e);
            wr::<u32>(e, 0, 0);
        }
        let ev = rd::<GPtr<u8>>(e, 8);
        if !ev.is_null() {
            v_delete(ev, 1);
            wr::<u32>(e, 8, 0);
        }
        wr::<u32>(e, 0xC, 0xFFFF_FFFF);
    }
}

/// `CInformGroupEventQueue::Process` (0x4B2AE0, static): `CInformGroupEvent::Process` on each of the 8 entries.
#[hook(addr = 0x4B2AE0, conv = cdecl, name = "CInformGroupEventQueue::Process", expect = "56 BE 18 B0 A9 00")]
pub fn inform_group_event_queue_process() {
    for i in 0..8usize {
        inform_group_event_process(GPtr::from_addr(INFORM_GROUP_QUEUE + 0x10 * i));
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_ACQUAINTANCE_CLEAR_AS,
    HOOK_ACQUAINTANCE_SET_AS,
    HOOK_OBJECT_SCANNER_CTOR,
    HOOK_OBJECT_SCANNER_SCAN_FOR_OBJECTS_IN_RANGE,
    HOOK_FILE_LOADER_RESTART,
    HOOK_SCRIPTED_TASK_RECORD_GET_STATUS,
    HOOK_SEQUENCE_MAKE_ABORTABLE,
    HOOK_SEQUENCE_ADD_TASK,
    HOOK_BE_IN_GROUP_CTOR,
    HOOK_INFORM_GROUP_EVENT_QUEUE_FLUSH,
    HOOK_INFORM_GROUP_EVENT_QUEUE_PROCESS,
];
