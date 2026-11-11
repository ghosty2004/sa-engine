//! Hand-ported `CTaskManager` task switching and abort functions (PORT-AI-2). Ported from the 1.0 US bytes (the
//! exe wins over gta-reversed and the specs). Every call to another catalogued function goes through `orig!` at
//! the original address (including this module's own functions), virtual calls through their vtable slot.
//!
//! Layout (0x681AF0 ff.): +0x00..+0x14 the five primary tasks, +0x14..+0x2C the six secondary tasks, +0x2C the
//! owning ped. Task vtable slots used: +0 scalar deleting destructor, +8 `GetSubTask`, +0xC `IsSimple`, +0x14
//! `StopTimer(event)`, +0x18 `MakeAbortable(ped, priority, event)`, +0x1C `SetSubTask(task)` (complex) /
//! `ProcessPed(ped)` (simple), +0x20 `CreateNextSubTask(ped)`, +0x24 `CreateFirstSubTask(ped)`, +0x28
//! `ControlSubTask(ped)`.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// Task scalar deleting destructor (vtable +0), called with flags 1
    fn v_delete(t: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CTask::GetSubTask` (vtable +8)
    fn v_get_sub_task(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x8 => unported;
    /// `CTask::IsSimple` (vtable +0xC)
    fn v_is_simple(t: GPtr<u8>) -> u8 = thiscall vslot 0xC => unported;
    /// `CTask::StopTimer(const CEvent*)` (vtable +0x14)
    fn v_stop_timer(t: GPtr<u8>, event: GPtr<u8>) = thiscall vslot 0x14 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::SetSubTask(CTask*)` (vtable +0x1C on complex tasks)
    fn v_set_sub_task(t: GPtr<u8>, sub: GPtr<u8>) = thiscall vslot 0x1C => unported;
    /// `CTaskSimple::ProcessPed(CPed*)` (vtable +0x1C on simple tasks)
    fn v_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall vslot 0x1C => unported;
    /// `CTaskComplex::CreateNextSubTask(CPed*)` (vtable +0x20)
    fn v_create_next_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x20 => unported;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CTaskComplex::ControlSubTask(CPed*)` (vtable +0x28)
    fn v_control_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x28 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskManager::AddSubTasks(CTask*)` (0x681A30, `ret 4`)
    fn tm_add_sub_tasks(tm: GPtr<u8>, task: GPtr<u8>) = thiscall 0x681A30 => unported;
    /// `CTaskManager::SetNextSubTask(CTaskComplex*)` (0x681920, `ret 4`)
    fn tm_set_next_sub_task(tm: GPtr<u8>, task: GPtr<u8>) = thiscall 0x681920 => unported;
    /// `CTaskManager::ParentsControlChildren(CTask*)` (0x681A80, `ret 4`)
    fn tm_parents_control_children(tm: GPtr<u8>, task: GPtr<u8>) = thiscall 0x681A80 => unported;
    /// `CTaskManager::GetLastTaskOf(CTask*)` (0x681970, static cdecl)
    fn tm_get_last_task_of_task(task: GPtr<u8>) -> GPtr<u8> = cdecl 0x681970 => unported;
}

/// `this[i]` for a slot offset (primary `i*4`, secondary `0x14 + i*4`).
fn slot(this: GPtr<u8>, off: isize) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, off)
}

fn ped_of(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0x2C)
}

/// The inlined "last task of the chain" walk: the last task reached by `GetSubTask` from `t`, null for a null
/// `t` (one `GetSubTask` call per task of the chain).
fn last_of(t: GPtr<u8>) -> GPtr<u8> {
    let mut last = GPtr::NULL;
    let mut cur = t;
    while !cur.is_null() {
        last = cur;
        cur = v_get_sub_task(cur);
    }
    last
}

/// `CTaskManager::CTaskManager(CPed*)` (0x6816A0, `ret 4`): +0x2C = ped, the 11 task slots zeroed; returns this.
#[hook(addr = 0x6816A0, conv = thiscall, name = "CTaskManager::Constructor", expect = "8B 44 24 04 8B D1")]
pub fn task_manager_ctor(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wr::<GPtr<u8>>(this, 0x2C, ped);
    let mut o = 0;
    while o < 0x2C {
        wr::<u32>(this, o, 0);
        o += 4;
    }
    this
}

/// Deletes (`delete(1)`) and clears every non-null task of the 5 primary then the 6 secondary slots. The
/// destructor (0x6816D0, through the island 0x403AAB that clears the slot) and `Flush` (0x681850) share it.
fn flush_all(this: GPtr<u8>) {
    for i in 0..5 {
        let t = slot(this, i * 4);
        if !t.is_null() {
            v_delete(t, 1);
        }
        wr::<u32>(this, i * 4, 0);
    }
    for i in 0..6 {
        let off = 0x14 + i * 4;
        let t = slot(this, off);
        if !t.is_null() {
            v_delete(t, 1);
        }
        wr::<u32>(this, off, 0);
    }
}

/// `CTaskManager::~CTaskManager` (0x6816D0): the primary loop continues through the island 0x403AAB (`mov
/// [edi+esi*4], 0; jmp 0x6816EA`); same effect as `Flush`.
#[hook(addr = 0x6816D0, conv = thiscall, name = "CTaskManager::Destructor", expect = "56 57 8B F9 33 F6")]
pub fn task_manager_dtor(this: GPtr<u8>) {
    flush_all(this);
}

/// `CTaskManager::Flush` (0x681850): deletes and clears every primary and secondary task.
#[hook(addr = 0x681850, conv = thiscall, name = "CTaskManager::Flush", expect = "56 57 8B F9 33 F6")]
pub fn task_manager_flush(this: GPtr<u8>) {
    flush_all(this);
}

/// `CTaskManager::GetActiveTask` (0x681720): the first non-null primary task, else null. Not hookable: the
/// loop branches back to 0x681722, inside the 5-byte patch.
pub fn task_manager_get_active_task(this: GPtr<u8>) -> GPtr<u8> {
    for i in 0..5 {
        let t = slot(this, i * 4);
        if !t.is_null() {
            return t;
        }
    }
    GPtr::NULL
}

/// `CTaskManager::FindTaskByType(int32 slot, int32 type)` (0x6817D0, `ret 8`): walks the chain of primary slot
/// `slot`; while nothing matched, compares `GetTaskType()` with `type` and then calls `GetSubTask` (also on the
/// task that matched). Returns the match or null.
#[hook(addr = 0x6817D0, conv = thiscall, name = "CTaskManager::FindTaskByType", expect = "8B 44 24 04 56")]
pub fn task_manager_find_task_by_type(this: GPtr<u8>, s: i32, ty: i32) -> GPtr<u8> {
    let mut t = slot(this, (s as isize).wrapping_mul(4));
    let mut found = GPtr::NULL;
    while !t.is_null() && found.is_null() {
        if v_get_task_type(t) == ty {
            found = t;
        }
        t = v_get_sub_task(t);
    }
    found
}

/// `CTaskManager::HasTaskSecondary(const CTask*)` (0x681820, `ret 4`): whether one of the six secondary slots
/// holds `task`. Not registered (INF-20): the protection code at 0xE1BC65 stores the immediate 0x681824, inside
/// the patched entry bytes.
pub fn task_manager_has_task_secondary(this: GPtr<u8>, task: GPtr<u8>) -> u8 {
    for i in 0..6 {
        if slot(this, 0x14 + i * 4) == task {
            return 1;
        }
    }
    0
}

/// `CTaskManager::FlushImmediately` (0x6818A0): for every primary then secondary task, asks
/// `MakeAbortable(ped, 2 /*ABORT_PRIORITY_IMMEDIATE*/, null)`; when it agrees, deletes the slot's task (re-read)
/// and clears the slot.
#[hook(addr = 0x6818A0, conv = thiscall, name = "CTaskManager::FlushImmediately", expect = "53 56 57 8B F9")]
pub fn task_manager_flush_immediately(this: GPtr<u8>) {
    let mut offs = [0isize; 11];
    for (i, o) in offs.iter_mut().enumerate() {
        *o = i as isize * 4;
    }
    for off in offs {
        let t = slot(this, off);
        if !t.is_null() && v_make_abortable(t, ped_of(this), 2, GPtr::NULL) != 0 {
            let t = slot(this, off);
            if !t.is_null() {
                v_delete(t, 1);
            }
            wr::<u32>(this, off, 0);
        }
    }
}

/// `CTaskManager::SetNextSubTask(CTaskComplex*)` (0x681920, `ret 4`): from `task` up its parents (+4): the first
/// whose `CreateNextSubTask(ped)` returns a task gets it (`SetSubTask`) and `AddSubTasks` runs on it; every task
/// before it gets `SetSubTask(null)`.
#[hook(addr = 0x681920, conv = thiscall, name = "CTaskManager::SetNextSubTask", expect = "53 56 8B 74 24 0C")]
pub fn task_manager_set_next_sub_task(this: GPtr<u8>, task: GPtr<u8>) {
    let mut t = task;
    while !t.is_null() {
        let next = v_create_next_sub_task(t, ped_of(this));
        if !next.is_null() {
            v_set_sub_task(t, next);
            tm_add_sub_tasks(this, next);
            return;
        }
        v_set_sub_task(t, GPtr::NULL);
        t = rd::<GPtr<u8>>(t, 4);
    }
}

/// `CTaskManager::GetLastTaskOf(CTask*)` (0x681970, static cdecl): the end of the task's sub-task chain (null
/// for null).
#[hook(addr = 0x681970, conv = cdecl, name = "CTaskManager::GetLastTaskOf-of-task", expect = "8B 4C 24 04 33 C0")]
pub fn task_manager_get_last_task_of_task(task: GPtr<u8>) -> GPtr<u8> {
    last_of(task)
}

/// `CTaskManager::StopTimers(const CEvent*)` (0x6819A0, `ret 4`): `StopTimer(event)` on every non-null primary
/// task (not the secondaries).
#[hook(addr = 0x6819A0, conv = thiscall, name = "CTaskManager::StopTimers", expect = "53 8B 5C 24 08")]
pub fn task_manager_stop_timers(this: GPtr<u8>, event: GPtr<u8>) {
    for i in 0..5 {
        let t = slot(this, i * 4);
        if !t.is_null() {
            v_stop_timer(t, event);
        }
    }
}

/// `CTaskManager::GetSimplestActiveTask` (0x6819D0): the last task of the first non-null primary chain, null
/// when every primary slot is empty. Not hookable: the loop branches back to 0x6819D2, inside the patch.
pub fn task_manager_get_simplest_active_task(this: GPtr<u8>) -> GPtr<u8> {
    last_of(task_manager_get_active_task(this))
}

/// `CTaskManager::GetLastTaskOf(int32 slot)` (0x681A00, `ret 4`): the end of primary slot `slot`'s chain.
#[hook(addr = 0x681A00, conv = thiscall, name = "CTaskManager::GetLastTaskOf-at-index", expect = "8B 44 24 04 8B 0C 81")]
pub fn task_manager_get_last_task_of_index(this: GPtr<u8>, s: i32) -> GPtr<u8> {
    last_of(slot(this, (s as isize).wrapping_mul(4)))
}

/// `CTaskManager::AddSubTasks(CTask*)` (0x681A30, `ret 4`): down from `task` while the task is complex:
/// `CreateFirstSubTask(ped)`; a non-null result becomes its sub-task (`SetSubTask`) and the walk continues with
/// it; a null result calls `SetNextSubTask(task->m_pParent)` and stops.
#[hook(addr = 0x681A30, conv = thiscall, name = "CTaskManager::AddSubTasks", expect = "53 56 8B 74 24 0C")]
pub fn task_manager_add_sub_tasks(this: GPtr<u8>, task: GPtr<u8>) {
    let mut t = task;
    while !t.is_null() {
        if v_is_simple(t) != 0 {
            return;
        }
        let sub = v_create_first_sub_task(t, ped_of(this));
        if !sub.is_null() {
            v_set_sub_task(t, sub);
        } else {
            tm_set_next_sub_task(this, rd::<GPtr<u8>>(t, 4));
        }
        t = sub;
    }
}

/// `CTaskManager::ParentsControlChildren(CTask*)` (0x681A80, `ret 4`): down from `task` while the task is
/// complex: `sub = GetSubTask()`, `ctl = ControlSubTask(ped)`; when they differ, `sub->MakeAbortable(ped, 1
/// /*URGENT*/, null)` (result ignored), `SetSubTask(ctl)`, `AddSubTasks(ctl)` and stop; else continue with
/// `GetSubTask()` (called again).
#[hook(addr = 0x681A80, conv = thiscall, name = "CTaskManager::ParentsControlChildren", expect = "55 56 8B 74 24 0C")]
pub fn task_manager_parents_control_children(this: GPtr<u8>, task: GPtr<u8>) {
    let mut t = task;
    while !t.is_null() {
        if v_is_simple(t) != 0 {
            return;
        }
        let sub = v_get_sub_task(t);
        let ctl = v_control_sub_task(t, ped_of(this));
        if sub != ctl {
            v_make_abortable(sub, ped_of(this), 1, GPtr::NULL);
            v_set_sub_task(t, ctl);
            tm_add_sub_tasks(this, ctl);
            return;
        }
        t = v_get_sub_task(t);
    }
}

/// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0, `ret 0xC`; the bool is unused). A null task
/// deletes and clears a non-null slot. Otherwise, unless the slot already holds `task`: deletes the old task,
/// stores `task`, `AddSubTasks(task)`; if the slot is still non-null and its chain does not end in a simple task
/// (`GetLastTaskOf` 0x681970 + `IsSimple`), the slot's task is deleted and the slot cleared.
#[hook(addr = 0x681AF0, conv = thiscall, name = "CTaskManager::SetTask", expect = "53 8B 5C 24 08")]
pub fn task_manager_set_task(this: GPtr<u8>, task: GPtr<u8>, s: i32, _unused: u8) {
    let off = (s as isize).wrapping_mul(4);
    let delete_slot = |this: GPtr<u8>| {
        let t = slot(this, off);
        if !t.is_null() {
            v_delete(t, 1);
        }
        wr::<u32>(this, off, 0);
    };
    if task.is_null() && !slot(this, off).is_null() {
        // 0x681B46: the slot's task (non-null here) is deleted without a second test
        v_delete(slot(this, off), 1);
        wr::<u32>(this, off, 0);
        return;
    }
    let cur = slot(this, off);
    if cur == task {
        return;
    }
    if !cur.is_null() {
        v_delete(cur, 1);
    }
    wr::<GPtr<u8>>(this, off, task);
    tm_add_sub_tasks(this, task);
    let t = slot(this, off);
    if t.is_null() {
        return;
    }
    let last = tm_get_last_task_of_task(t);
    if v_is_simple(last) != 0 {
        return;
    }
    delete_slot(this);
}

/// `CTaskManager::SetTaskSecondary(CTask*, int32 slot)` (0x681B60, `ret 8`): as `SetTask` on secondary slot
/// `slot` (+0x14), without the null-task shortcut and with the chain walk inlined.
#[hook(addr = 0x681B60, conv = thiscall, name = "CTaskManager::SetTaskSecondary", expect = "53 8B 5C 24 08")]
pub fn task_manager_set_task_secondary(this: GPtr<u8>, task: GPtr<u8>, s: i32) {
    let off = 0x14 + (s as isize).wrapping_mul(4);
    let cur = slot(this, off);
    if cur == task {
        return;
    }
    if !cur.is_null() {
        v_delete(cur, 1);
    }
    wr::<GPtr<u8>>(this, off, task);
    tm_add_sub_tasks(this, task);
    let t = slot(this, off);
    if t.is_null() {
        return;
    }
    if v_is_simple(last_of(t)) != 0 {
        return;
    }
    let t = slot(this, off);
    if !t.is_null() {
        v_delete(t, 1);
    }
    wr::<u32>(this, off, 0);
}

/// `CTaskManager::ClearTaskEventResponse` (0x681BD0): deletes and clears primary slot 1 (event response
/// temp) if set, else slot 2 (event response non-temp) if set.
#[hook(addr = 0x681BD0, conv = thiscall, name = "CTaskManager::ClearTaskEventResponse", expect = "56 8B F1 8B 46 04")]
pub fn task_manager_clear_task_event_response(this: GPtr<u8>) {
    let t = slot(this, 4);
    if !t.is_null() {
        v_delete(t, 1);
        wr::<u32>(this, 4, 0);
        return;
    }
    let t = slot(this, 8);
    if !t.is_null() {
        v_delete(t, 1);
        wr::<u32>(this, 8, 0);
    }
}

/// Deletes primary/secondary slot `off`'s task (re-read, null-checked) and clears the slot.
fn delete_and_clear(this: GPtr<u8>, off: isize) {
    let t = slot(this, off);
    if !t.is_null() {
        v_delete(t, 1);
    }
    wr::<u32>(this, off, 0);
}

/// `CTaskManager::ManageTasks` (0x681C10), once per ped per frame.
///
/// Primary: the first non-null slot `i`. If its chain does not end in a simple task, the slot is deleted and
/// cleared and the function returns (the secondaries are NOT processed on this path). Otherwise up to 11
/// rounds (the 11th sets the stop flag first): `ParentsControlChildren(slot)`; if the chain's end is not
/// simple, `SetNextSubTask(end->m_pParent)`, and if the new end is still not simple the slot is deleted and
/// cleared (go to the secondaries). Then `end->ProcessPed(ped)`; false ends the primary part; true:
/// `SetNextSubTask(end->m_pParent)`; when the slot's task has no sub-task any more it is deleted and cleared;
/// otherwise another round unless the stop flag is set.
///
/// Secondary slots 0..5 (task held in a register, not re-read): `ParentsControlChildren(t)`; a chain not ending
/// in a simple task deletes `t` and clears the slot; else `end->ProcessPed(ped)`; true: `SetNextSubTask(end->
/// m_pParent)` and, while `t` still has a sub-task, another round, else `t` is deleted and the slot cleared.
#[hook(addr = 0x681C10, conv = thiscall, name = "CTaskManager::ManageTasks", expect = "51 53 55 56 57")]
pub fn task_manager_manage_tasks(this: GPtr<u8>) {
    let mut i = 0isize;
    while i < 5 && slot(this, i * 4).is_null() {
        i += 1;
    }
    if i < 5 {
        let off = i * 4;
        if v_is_simple(last_of(slot(this, off))) == 0 {
            delete_and_clear(this, off);
            return;
        }
        let mut count = 0u32;
        let mut stop = false;
        loop {
            count += 1;
            if count > 10 {
                stop = true;
            }
            tm_parents_control_children(this, slot(this, off));
            let end = last_of(slot(this, off));
            if v_is_simple(end) == 0 {
                tm_set_next_sub_task(this, rd::<GPtr<u8>>(end, 4));
                if v_is_simple(last_of(slot(this, off))) == 0 {
                    delete_and_clear(this, off);
                    break;
                }
            }
            let end = last_of(slot(this, off));
            if v_process_ped(end, ped_of(this)) == 0 {
                break;
            }
            tm_set_next_sub_task(this, rd::<GPtr<u8>>(end, 4));
            if v_get_sub_task(slot(this, off)).is_null() {
                delete_and_clear(this, off);
                break;
            }
            if stop {
                break;
            }
        }
    }
    for k in 0..6isize {
        let off = 0x14 + k * 4;
        let t = slot(this, off);
        if t.is_null() {
            continue;
        }
        loop {
            tm_parents_control_children(this, t);
            if v_is_simple(last_of(t)) == 0 {
                v_delete(t, 1);
                wr::<u32>(this, off, 0);
                break;
            }
            let end = last_of(t);
            if v_process_ped(end, ped_of(this)) == 0 {
                break;
            }
            tm_set_next_sub_task(this, rd::<GPtr<u8>>(end, 4));
            if v_get_sub_task(t).is_null() {
                v_delete(t, 1);
                wr::<u32>(this, off, 0);
                break;
            }
        }
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_TASK_MANAGER_CTOR,
    HOOK_TASK_MANAGER_DTOR,
    HOOK_TASK_MANAGER_FLUSH,
    HOOK_TASK_MANAGER_FIND_TASK_BY_TYPE,
    HOOK_TASK_MANAGER_FLUSH_IMMEDIATELY,
    HOOK_TASK_MANAGER_SET_NEXT_SUB_TASK,
    HOOK_TASK_MANAGER_GET_LAST_TASK_OF_TASK,
    HOOK_TASK_MANAGER_STOP_TIMERS,
    HOOK_TASK_MANAGER_GET_LAST_TASK_OF_INDEX,
    HOOK_TASK_MANAGER_ADD_SUB_TASKS,
    HOOK_TASK_MANAGER_PARENTS_CONTROL_CHILDREN,
    HOOK_TASK_MANAGER_SET_TASK,
    HOOK_TASK_MANAGER_SET_TASK_SECONDARY,
    HOOK_TASK_MANAGER_CLEAR_TASK_EVENT_RESPONSE,
    HOOK_TASK_MANAGER_MANAGE_TASKS,
];
