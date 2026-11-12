//! `CPedGroupIntelligence` task-pair helpers (set / get / flush / report-finished over the four `CPedTaskPair`
//! arrays) and allocator setters, `CEventHandlerHistory` stored-event helpers, `CPedIntelligence::GetActivePrimaryTask`
//! and `CTaskComplexSequence::SetCanBeEmptied` (PORT-AI-7). Ported from the 1.0 US bytes (the exe wins over
//! gta-reversed). Every call to another catalogued function goes through `orig!` at the original address.
//!
//! `CPedGroupIntelligence`: +0 the group, pair arrays of 8 x 0x14 `CPedTaskPair` (+0 ped, +4 task, +8 slot, ...) at
//! +0xC (primary / event response), +0xAC (secondary), +0x14C (script command), +0x1EC (default); +0x28C default
//! task allocator, +0x290 primary task allocator, +0x294 event response task allocator.
//! `CEventHandlerHistory`: +4 non-temporary event, +8 temporary event, +0xC stored abort event.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `sizeof(CPedTaskPair)`
const PAIR: isize = 0x14;
/// Pairs per array.
const PAIRS: isize = 8;

orig! {
    /// Virtual slot +0 (scalar deleting destructor, `ret 4`): `delete obj` with flags 1.
    fn v_delete(o: GPtr<u8>, flags: u32) -> u32 = thiscall vslot 0x0 => unported;
    /// `CEvent::GetEventPriority` (vtable +8)
    fn v_event_priority(e: GPtr<u8>) -> i32 = thiscall vslot 0x8 => unported;
    /// `CPedGroupDefaultTaskAllocator::AllocateDefaultTasks(CPedGroup*, CPed*)` (vtable +0, `ret 8`)
    fn v_allocate_default_tasks(a: GPtr<u8>, group: GPtr<u8>, ped: GPtr<u8>) = thiscall vslot 0x0 => unported;
    /// `CPedGroupIntelligence::SetTask(CPed*, const CTask&, CPedTaskPair*, int32 slot, bool force)` (0x5F7540,
    /// `ret 0x14`)
    fn pgi_set_task(i: GPtr<u8>, ped: GPtr<u8>, task: GPtr<u8>, pairs: GPtr<u8>, slot: i32, force: u32)
        = thiscall 0x5F7540 => unported;
    /// `CPedGroupIntelligence::ReportAllTasksFinished(CPedTaskPair*)` (0x5F7730, `ret 4`; ECX unused)
    fn o_report_pairs(i: GPtr<u8>, pairs: GPtr<u8>) = thiscall 0x5F7730 => unported;
    /// `CPedGroupIntelligence::FlushTasks(CPedTaskPair*, CPed*)` (0x5F79C0, `ret 8`; ECX unused)
    fn o_flush_tasks(i: GPtr<u8>, pairs: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x5F79C0 => unported;
    /// `CTaskComplexSequence::Flush` (0x632C10)
    fn sequence_flush(s: GPtr<u8>) = thiscall 0x632C10 => unported;
}

/// The task of `ped` in the 8 pairs at `pairs`, or null (the first pair whose ped matches).
fn pair_task(pairs: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    for k in 0..PAIRS {
        if rd::<GPtr<u8>>(pairs, k * PAIR) == ped {
            return rd::<GPtr<u8>>(pairs, k * PAIR + 4);
        }
    }
    GPtr::NULL
}

/// `delete *slot` through its scalar deleting destructor if it is set (the slot keeps its value).
fn delete_if_set(o: GPtr<u8>) {
    if !o.is_null() {
        v_delete(o, 1);
    }
}

/// `CPedGroupIntelligence::SetPrimaryTaskAllocator(CTaskAllocator*)` (0x5F7410, `ret 4`): deletes the current
/// allocator at +0x290 (if any) and stores the new one.
#[hook(addr = 0x5F7410, conv = thiscall, name = "CPedGroupIntelligence::SetPrimaryTaskAllocator",
       expect = "56 8B F1 8B 8E 90 02 00 00", unverified = "ai7-6", route = "L2", hazards = "heap-writes",
       note = "frees the old allocator (task pool)")]
pub fn pgi_set_primary_task_allocator(this: GPtr<u8>, alloc: GPtr<u8>) {
    delete_if_set(rd::<GPtr<u8>>(this, 0x290));
    wr::<GPtr<u8>>(this, 0x290, alloc);
}

/// `CPedGroupIntelligence::SetEventResponseTaskAllocator(CTaskAllocator*)` (0x5F7440, `ret 4`): as
/// `SetPrimaryTaskAllocator` for +0x294.
#[hook(addr = 0x5F7440, conv = thiscall, name = "CPedGroupIntelligence::SetEventResponseTaskAllocator",
       expect = "56 8B F1 8B 8E 94 02 00 00", unverified = "ai7-6", route = "L2", hazards = "heap-writes",
       note = "frees the old allocator (task pool)")]
pub fn pgi_set_event_response_task_allocator(this: GPtr<u8>, alloc: GPtr<u8>) {
    delete_if_set(rd::<GPtr<u8>>(this, 0x294));
    wr::<GPtr<u8>>(this, 0x294, alloc);
}

/// `CPedGroupIntelligence::GetTask(CPed*, CPedTaskPair*)` (0x5F7660, `ret 8`; ECX unused): the task of the first
/// pair whose ped is `ped`, else null.
#[hook(addr = 0x5F7660, conv = thiscall, name = "CPedGroupIntelligence::GetTask", expect = "56 8B 74 24 0C",
       unverified = "ai7-6", route = "L2", hazards = "none")]
pub fn pgi_get_task(_this: GPtr<u8>, ped: GPtr<u8>, pairs: GPtr<u8>) -> GPtr<u8> {
    pair_task(pairs, ped)
}

/// `CPedGroupIntelligence::ReportAllTasksFinished(CPedTaskPair*)` (0x5F7730, `ret 4`; ECX unused): deletes the task
/// of every pair that has one and clears it (the pairs' peds stay).
#[hook(addr = 0x5F7730, conv = thiscall, name = "CPedGroupIntelligence::ReportAllTasksFinished",
       expect = "56 8B 74 24 08", unverified = "ai7-6", route = "L2", hazards = "heap-writes",
       note = "frees tasks (task pool)")]
pub fn pgi_report_all_tasks_finished_pairs(_this: GPtr<u8>, pairs: GPtr<u8>) {
    for k in 0..PAIRS {
        let t = rd::<GPtr<u8>>(pairs, k * PAIR + 4);
        if !t.is_null() {
            v_delete(t, 1);
            wr::<u32>(pairs, k * PAIR + 4, 0);
        }
    }
}

/// `CPedGroupIntelligence::FlushTasks(CPedTaskPair*, CPed*)` (0x5F79C0, `ret 8`; ECX unused): for every pair of
/// `ped` (every pair when `ped` is null) deletes the task if there is one and clears the task field.
#[hook(addr = 0x5F79C0, conv = thiscall, name = "CPedGroupIntelligence::FlushTasks", expect = "53 56 8B 74 24 0C",
       unverified = "ai7-6", route = "L2", hazards = "heap-writes", note = "frees tasks (task pool)")]
pub fn pgi_flush_tasks(_this: GPtr<u8>, pairs: GPtr<u8>, ped: GPtr<u8>) {
    for k in 0..PAIRS {
        if !ped.is_null() && rd::<GPtr<u8>>(pairs, k * PAIR) != ped {
            continue;
        }
        delete_if_set(rd::<GPtr<u8>>(pairs, k * PAIR + 4));
        wr::<u32>(pairs, k * PAIR + 4, 0);
    }
}

/// `CPedGroupIntelligence::SetEventResponseTask(CPed*, bool, const CTask&, bool, const CTask&, int32)` (0x5F8510,
/// `ret 0x18`): with the first flag (AL of the slot) `SetTask(ped, task1, +0xC pairs, -1, false)`, then with the
/// second `SetTask(ped, task2, +0xAC pairs, slot, false)`.
#[hook(addr = 0x5F8510, conv = thiscall, name = "CPedGroupIntelligence::SetEventResponseTask",
       expect = "8A 44 24 08 84 C0", unverified = "ai7-6", route = "L2", hazards = "heap-writes",
       note = "SetTask clones tasks into the pairs")]
pub fn pgi_set_event_response_task(this: GPtr<u8>, ped: GPtr<u8>, primary: u32, task1: GPtr<u8>, secondary: u32,
                                   task2: GPtr<u8>, slot: i32) {
    if primary as u8 != 0 {
        pgi_set_task(this, ped, task1, this.byte_add(0xC), -1, 0);
    }
    if secondary as u8 != 0 {
        pgi_set_task(this, ped, task2, this.byte_add(0xAC), slot, 0);
    }
}

/// `CPedGroupIntelligence::SetScriptCommandTask(CPed*, const CTask&)` (0x5F8560, `ret 8`): `SetTask(ped, task,
/// +0x14C pairs, -1, true)`.
#[hook(addr = 0x5F8560, conv = thiscall, name = "CPedGroupIntelligence::SetScriptCommandTask",
       expect = "8B 54 24 08 6A 01", unverified = "ai7-6", route = "L2", hazards = "heap-writes",
       note = "SetTask clones the task into the pairs")]
pub fn pgi_set_script_command_task(this: GPtr<u8>, ped: GPtr<u8>, task: GPtr<u8>) {
    pgi_set_task(this, ped, task, this.byte_add(0x14C), -1, 1);
}

/// `CPedGroupIntelligence::SetDefaultTask(CPed*, const CTask&)` (0x5F8580, `ret 8`): `SetTask(ped, task, +0x1EC
/// pairs, -1, false)`.
#[hook(addr = 0x5F8580, conv = thiscall, name = "CPedGroupIntelligence::SetDefaultTask",
       expect = "8B 54 24 08 6A 00", unverified = "ai7-6", route = "L2", hazards = "heap-writes",
       note = "SetTask clones the task into the pairs")]
pub fn pgi_set_default_task(this: GPtr<u8>, ped: GPtr<u8>, task: GPtr<u8>) {
    pgi_set_task(this, ped, task, this.byte_add(0x1EC), -1, 0);
}

/// `CPedGroupIntelligence::GetTaskScriptCommand(CPed*)` (0x5F8690, `ret 4`): the task of `ped` in the +0x14C pairs.
#[hook(addr = 0x5F8690, conv = thiscall, name = "CPedGroupIntelligence::GetTaskScriptCommand",
       expect = "56 8D B1 4C 01 00 00", unverified = "ai7-6", route = "L2", hazards = "none")]
pub fn pgi_get_task_script_command(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    pair_task(this.byte_add(0x14C), ped)
}

/// `CPedGroupIntelligence::GetTaskDefault(CPed*)` (0x5F86C0, `ret 4`): the task of `ped` in the +0x1EC pairs.
#[hook(addr = 0x5F86C0, conv = thiscall, name = "CPedGroupIntelligence::GetTaskDefault",
       expect = "56 8D B1 EC 01 00 00", unverified = "ai7-6", route = "L2", hazards = "none")]
pub fn pgi_get_task_default(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    pair_task(this.byte_add(0x1EC), ped)
}

/// `CPedGroupIntelligence::ReportAllTasksFinished()` (0x5F8750): the pair version on +0xC, +0x14C, +0xAC in that
/// order.
#[hook(addr = 0x5F8750, conv = thiscall, name = "CPedGroupIntelligence::ReportAllTasksFinished",
       expect = "56 8B F1 8D 46 0C", unverified = "ai7-6", route = "L2", hazards = "heap-writes",
       note = "frees tasks (task pool)")]
pub fn pgi_report_all_tasks_finished(this: GPtr<u8>) {
    o_report_pairs(this, this.byte_add(0xC));
    o_report_pairs(this, this.byte_add(0x14C));
    o_report_pairs(this, this.byte_add(0xAC));
}

/// `CPedGroupIntelligence::ReportAllBarScriptTasksFinished` (0x5F8780): the pair version on +0xC and +0xAC.
#[hook(addr = 0x5F8780, conv = thiscall, name = "CPedGroupIntelligence::ReportAllBarScriptTasksFinished",
       expect = "56 8B F1 8D 46 0C", unverified = "ai7-6", route = "L2", hazards = "heap-writes",
       note = "frees tasks (task pool)")]
pub fn pgi_report_all_bar_script_tasks_finished(this: GPtr<u8>) {
    o_report_pairs(this, this.byte_add(0xC));
    o_report_pairs(this, this.byte_add(0xAC));
}

/// `CPedGroupIntelligence::ComputeDefaultTasks(CPed*)` (0x5F88D0, `ret 4`): `FlushTasks(+0x1EC pairs, ped)`, then
/// the default allocator's `AllocateDefaultTasks(group, ped)`.
#[hook(addr = 0x5F88D0, conv = thiscall, name = "CPedGroupIntelligence::ComputeDefaultTasks",
       expect = "56 57 8B 7C 24 0C", unverified = "ai7-6", route = "L2", hazards = "heap-writes",
       note = "frees and allocates tasks (task pool)")]
pub fn pgi_compute_default_tasks(this: GPtr<u8>, ped: GPtr<u8>) {
    o_flush_tasks(this, this.byte_add(0x1EC), ped);
    let alloc = rd::<GPtr<u8>>(this, 0x28C);
    v_allocate_default_tasks(alloc, rd::<GPtr<u8>>(this, 0), ped);
}

/// `CPedIntelligence::GetActivePrimaryTask` (0x4B85B0): the first non-null of the task manager's primary tasks
/// 0, 1, 2 (+4, +8, +0xC; slot 2's value even when null).
#[hook(addr = 0x4B85B0, conv = thiscall, name = "CPedIntelligence::GetActivePrimaryTask", expect = "8B 41 04 85 C0",
       unverified = "ai7-6", route = "L2", hazards = "none")]
pub fn intel_get_active_primary_task(this: GPtr<u8>) -> GPtr<u8> {
    for off in [4, 8] {
        let t = rd::<GPtr<u8>>(this, off);
        if !t.is_null() {
            return t;
        }
    }
    rd::<GPtr<u8>>(this, 0xC)
}

/// `CEventHandlerHistory::TickStoredEvent(CPed*)` (0x4B8C20, `ret 4`; the ped is unused): with a stored event at
/// +0xC and no temporary event, the stored event becomes the non-temporary one (+4) and +0xC is cleared.
#[hook(addr = 0x4B8C20, conv = thiscall, name = "CEventHandlerHistory::TickStoredEvent", expect = "8B 41 0C 85 C0",
       unverified = "ai7-6", route = "L2", hazards = "heap-writes")]
pub fn history_tick_stored_event(this: GPtr<u8>, _ped: GPtr<u8>) {
    let stored = rd::<u32>(this, 0xC);
    if stored != 0 && rd::<u32>(this, 8) == 0 {
        wr::<u32>(this, 4, stored);
        wr::<u32>(this, 0xC, 0);
    }
}

/// `CEventHandlerHistory::ClearTempEvent` (0x4B8C40): deletes the temporary event (+8) if set and clears it.
#[hook(addr = 0x4B8C40, conv = thiscall, name = "CEventHandlerHistory::ClearTempEvent", expect = "56 8B F1 8B 4E 08",
       unverified = "ai7-6", route = "L2", hazards = "heap-writes", note = "frees an event (event pool)")]
pub fn history_clear_temp_event(this: GPtr<u8>) {
    delete_if_set(rd::<GPtr<u8>>(this, 8));
    wr::<u32>(this, 8, 0);
}

/// `CEventHandlerHistory::ClearNonTempEvent` (0x4B8C60): deletes the non-temporary event (+4) if set and clears it.
#[hook(addr = 0x4B8C60, conv = thiscall, name = "CEventHandlerHistory::ClearNonTempEvent",
       expect = "56 8B F1 8B 4E 04", unverified = "ai7-6", route = "L2", hazards = "heap-writes",
       note = "frees an event (event pool)")]
pub fn history_clear_non_temp_event(this: GPtr<u8>) {
    delete_if_set(rd::<GPtr<u8>>(this, 4));
    wr::<u32>(this, 4, 0);
}

/// `CEventHandlerHistory::GetCurrentEventPriority` (0x4B8C80): the priority of the temporary event (+8), else of the
/// non-temporary one (+4) (tail jumps to its `GetEventPriority`), else -1.
#[hook(addr = 0x4B8C80, conv = thiscall, name = "CEventHandlerHistory::GetCurrentEventPriority",
       expect = "8B C1 8B 48 08", unverified = "ai7-6", route = "L2", hazards = "none")]
pub fn history_get_current_event_priority(this: GPtr<u8>) -> i32 {
    for off in [8, 4] {
        let e = rd::<GPtr<u8>>(this, off);
        if !e.is_null() {
            return v_event_priority(e);
        }
    }
    -1
}

/// `CTaskComplexSequence::SetCanBeEmptied(bool)` (0x463610, `ret 4`): with no reference count (+0x3C == 0) and the
/// flag byte exactly 1, clears +0x38 and flushes the sequence; otherwise stores the flag byte at +0x38.
#[hook(addr = 0x463610, conv = thiscall, name = "CTaskComplexSequence::SetCanBeEmptied", expect = "8B 41 3C 85 C0",
       unverified = "ai7-6", route = "L2", hazards = "heap-writes", note = "Flush frees the sequence's tasks")]
pub fn sequence_set_can_be_emptied(this: GPtr<u8>, b: u32) {
    let flag = b as u8;
    if rd::<u32>(this, 0x3C) == 0 && flag == 1 {
        wr::<u8>(this, 0x38, 0);
        sequence_flush(this);
    } else {
        wr::<u8>(this, 0x38, flag);
    }
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_PGI_SET_PRIMARY_TASK_ALLOCATOR,
    HOOK_PGI_SET_EVENT_RESPONSE_TASK_ALLOCATOR,
    HOOK_PGI_GET_TASK,
    HOOK_PGI_REPORT_ALL_TASKS_FINISHED_PAIRS,
    HOOK_PGI_FLUSH_TASKS,
    HOOK_PGI_SET_EVENT_RESPONSE_TASK,
    HOOK_PGI_SET_SCRIPT_COMMAND_TASK,
    HOOK_PGI_SET_DEFAULT_TASK,
    HOOK_PGI_GET_TASK_SCRIPT_COMMAND,
    HOOK_PGI_GET_TASK_DEFAULT,
    HOOK_PGI_REPORT_ALL_TASKS_FINISHED,
    HOOK_PGI_REPORT_ALL_BAR_SCRIPT_TASKS_FINISHED,
    HOOK_PGI_COMPUTE_DEFAULT_TASKS,
    HOOK_INTEL_GET_ACTIVE_PRIMARY_TASK,
    HOOK_HISTORY_TICK_STORED_EVENT,
    HOOK_HISTORY_CLEAR_TEMP_EVENT,
    HOOK_HISTORY_CLEAR_NON_TEMP_EVENT,
    HOOK_HISTORY_GET_CURRENT_EVENT_PRIORITY,
    HOOK_SEQUENCE_SET_CAN_BE_EMPTIED,
];
