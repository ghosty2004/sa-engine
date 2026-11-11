//! `CPedGroupIntelligence::Process` and `ComputeScriptCommandTasks` (PORT-AI-2, pass 1's deferred rows). Ported
//! from the 1.0 US bytes; gta-reversed's use-after-free fix in `Process` (PedGroupIntelligence.cpp:192) is not
//! taken (the original deletes the response allocator while the primary one may still point to it).
//!
//! `CPedGroupIntelligence`: +0 the `CPedGroup` (+8 its `CPedGroupMembership`, members at +0xC + 4i, i = 0..7),
//! +4 current event, +8 highest-priority event (both `CEventGroupEvent`, base event at +0x10), pair arrays of 8
//! x 0x14 (`CPedTaskPair`: +0 ped, +4 task) at +0xC (primary), +0xAC (secondary), +0x14C (script command),
//! +0x1EC (default), +0x28C default task allocator, +0x290 primary allocator, +0x294 event response allocator.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    fn v_delete(o: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CEvent::GetEventType` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CEvent::Clone` (vtable +0x10)
    fn v_event_clone(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x10 => unported;
    /// `CEvent` vtable +0x3C (bool, no argument)
    fn v_event_3c(e: GPtr<u8>) -> u8 = thiscall vslot 0x3C => unported;
    /// `CPedGroupDefaultTaskAllocator::AllocateDefaultTasks(CPedGroup*, CPed*)` (vtable +0)
    fn v_alloc_default_tasks(a: GPtr<u8>, group: GPtr<u8>, ped: GPtr<u8>) = thiscall vslot 0x0 => unported;
    /// `CTaskAllocator::AllocateTasks(CPedGroupIntelligence*)` (vtable +4)
    fn v_alloc_allocate_tasks(a: GPtr<u8>, i: GPtr<u8>) = thiscall vslot 0x4 => unported;
    /// `CTaskAllocator::ProcessGroup(CPedGroupIntelligence*)` (vtable +8)
    fn v_alloc_process_group(a: GPtr<u8>, i: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x8 => unported;
    /// `CPedGroupIntelligence::ProcessIgnorePlayerGroup` (0x5F87A0)
    fn pgi_process_ignore_player_group(i: GPtr<u8>) = thiscall 0x5F87A0 => unported;
    /// `CPedGroupIntelligence::IsGroupResponding` (0x5F7760)
    fn pgi_is_group_responding(i: GPtr<u8>) -> u8 = thiscall 0x5F7760 => unported;
    /// `CPedGroupIntelligence::IsCurrentEventValid` (0x5F77A0)
    fn pgi_is_current_event_valid(i: GPtr<u8>) -> u8 = thiscall 0x5F77A0 => unported;
    /// `CPedGroupIntelligence::ComputeScriptCommandTasks` (0x5F7800)
    fn pgi_compute_script_command_tasks(i: GPtr<u8>) = thiscall 0x5F7800 => unported;
    /// `CPedGroupIntelligence::ComputeEventResponseTasks` (0x5FC440)
    fn pgi_compute_event_response_tasks(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x5FC440 => unported;
    /// `CEventGroupEvent::BaseEventTakesPriorityOverBaseEvent(const CEventGroupEvent&)` (0x4AE100, `ret 4`)
    fn group_event_base_takes_priority(e: GPtr<u8>, o: GPtr<u8>) -> u8 = thiscall 0x4AE100 => unported;
    /// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470, `ret 4`)
    fn pgi_add_event(i: GPtr<u8>, e: GPtr<u8>) = thiscall 0x5F7470 => unported;
}

fn p(this: GPtr<u8>, off: isize) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, off)
}

/// Deletes `this+off` when set and clears it.
fn delete_field(this: GPtr<u8>, off: isize) {
    let o = p(this, off);
    if !o.is_null() {
        v_delete(o, 1);
    }
    wr::<u32>(this, off, 0);
}

/// `CPedGroupIntelligence::Process` (0x5FC4A0; the first test continues through the island 0x403CAF).
///
/// `ProcessIgnorePlayerGroup`; a current event without a primary allocator that the group no longer responds to
/// or that is no longer valid is deleted with the response allocator. Members that changed (slot i of the
/// membership vs. the primary pair's ped) are written into the primary, secondary and default pairs; on any change
/// the primary allocator re-allocates (`AllocateTasks`), or else the default tasks are deleted and re-allocated
/// (`AllocateDefaultTasks(group, null)`), the script command tasks recomputed and, with a current event, the
/// response allocator re-allocates (after the primary tasks are deleted) or is computed. Then the highest-priority
/// event becomes current when there is none or it takes priority (`BaseEventTakesPriorityOverBaseEvent`, unless both
/// base events have the same type and the new one's +0x3C is false); a same-type event with +0x3C true is cloned
/// and added at the end. A finished primary allocator (`ProcessGroup` null) is deleted; a leftover highest-priority
/// event is deleted; without a primary allocator the response allocator's `ProcessGroup` runs (result ignored).
#[hook(addr = 0x5FC4A0, conv = thiscall, name = "CPedGroupIntelligence::Process", expect = "51 53 55 56 57")]
pub fn pgi_process(this: GPtr<u8>) {
    pgi_process_ignore_player_group(this);
    let membership = p(this, 0).byte_add(8);
    if !p(this, 4).is_null()
        && p(this, 0x290).is_null()
        && (pgi_is_group_responding(this) == 0 || pgi_is_current_event_valid(this) == 0)
    {
        if !p(this, 4).is_null() {
            v_delete(p(this, 4), 1);
        }
        wr::<u32>(this, 4, 0);
        if !p(this, 0x294).is_null() {
            v_delete(p(this, 0x294), 1);
        }
        wr::<u32>(this, 0x294, 0);
    }
    let mut changed = false;
    for k in 0..8isize {
        let m = rd::<u32>(membership, 4 + k * 4);
        if m != rd::<u32>(this, 0xC + k * 0x14) {
            wr::<u32>(this, 0xC + k * 0x14, m);
            wr::<u32>(this, 0xAC + k * 0x14, m);
            wr::<u32>(this, 0x1EC + k * 0x14, m);
            changed = true;
        }
    }
    if changed {
        let primary = p(this, 0x290);
        if !primary.is_null() {
            v_alloc_allocate_tasks(primary, this);
        } else {
            for k in 0..8isize {
                delete_field(this, 0x1F0 + k * 0x14);
            }
            v_alloc_default_tasks(p(this, 0x28C), p(this, 0), GPtr::NULL);
            pgi_compute_script_command_tasks(this);
            if !p(this, 4).is_null() {
                if !p(this, 0x294).is_null() {
                    for k in 0..8isize {
                        delete_field(this, 0x10 + k * 0x14);
                    }
                    v_alloc_allocate_tasks(p(this, 0x294), this);
                } else {
                    let a = pgi_compute_event_response_tasks(this);
                    wr::<GPtr<u8>>(this, 0x294, a);
                }
            }
        }
    }
    // 0x5FC69F
    let mut clone = GPtr::<u8>::NULL;
    if !p(this, 8).is_null() {
        let mut make_current = p(this, 4).is_null();
        if !make_current {
            let ta = v_event_type(p(p(this, 8), 0x10));
            let tb = v_event_type(p(p(this, 4), 0x10));
            let skip_priority = ta == tb && v_event_3c(p(p(this, 8), 0x10)) == 0;
            if !skip_priority && group_event_base_takes_priority(p(this, 8), p(this, 4)) != 0 {
                make_current = true;
            } else {
                // 0x5FC6F0: the current event's base type first, then the new one's
                let a = p(p(this, 8), 0x10);
                let tb = v_event_type(p(p(this, 4), 0x10));
                let ta = v_event_type(a);
                if ta == tb && v_event_3c(p(p(this, 8), 0x10)) != 0 {
                    clone = v_event_clone(p(this, 8));
                }
            }
        }
        if make_current {
            // 0x5FC72F
            if !p(this, 4).is_null() {
                v_delete(p(this, 4), 1);
            }
            let h = p(this, 8);
            wr::<GPtr<u8>>(this, 4, h);
            wr::<u32>(this, 8, 0);
            if !p(this, 0x294).is_null() {
                v_delete(p(this, 0x294), 1);
            }
            wr::<u32>(this, 0x294, 0);
            let a = pgi_compute_event_response_tasks(this);
            wr::<GPtr<u8>>(this, 0x294, a);
        }
    }
    // 0x5FC768
    let primary = p(this, 0x290);
    if !primary.is_null() && v_alloc_process_group(primary, this).is_null() {
        delete_field(this, 0x290);
    }
    let h = p(this, 8);
    if !h.is_null() && p(this, 4) != h {
        v_delete(h, 1);
        wr::<u32>(this, 8, 0);
    }
    if p(this, 0x290).is_null() && !p(this, 4).is_null() && !p(this, 0x294).is_null() {
        v_alloc_process_group(p(this, 0x294), this);
    }
    if !clone.is_null() {
        pgi_add_event(this, clone);
        v_delete(clone, 1);
    }
}

/// `CPedGroupIntelligence::ComputeScriptCommandTasks` (0x5F7800): re-indexes the script command pairs (+0x14C) so
/// that pair i belongs to member i (the task of the pair whose ped is member i, or null; the first pair with a
/// null ped when the member is null), deletes (and clears) every task whose ped has no entry with a task in the
/// new order, then writes the new peds and tasks back (the other pair fields are left alone).
#[hook(addr = 0x5F7800, conv = thiscall, name = "CPedGroupIntelligence::ComputeScriptCommandTasks", expect = "81 EC A8 00 00 00")]
pub fn pgi_compute_script_command_tasks_port(this: GPtr<u8>) {
    let membership = p(this, 0).byte_add(8);
    let pair = |i: usize| this.byte_add(0x14C + (i as isize) * 0x14);
    let mut tmp = [(0u32, 0u32); 8];
    for (i, t) in tmp.iter_mut().enumerate() {
        let m = rd::<u32>(membership, 4 + (i as isize) * 4);
        *t = (rd::<u32>(pair(i), 0), rd::<u32>(pair(i), 4));
        if m != t.0 {
            t.0 = m;
            t.1 = (0..8)
                .find(|j| rd::<u32>(pair(*j), 0) == m)
                .map_or(0, |j| rd::<u32>(pair(j), 4));
        }
    }
    for k in 0..8 {
        let task = rd::<GPtr<u8>>(pair(k), 4);
        if task.is_null() {
            continue;
        }
        let ped = rd::<u32>(pair(k), 0);
        let keep = tmp.iter().find(|t| t.0 == ped).is_some_and(|t| t.1 != 0);
        if !keep {
            v_delete(task, 1);
            wr::<u32>(pair(k), 0, 0);
            wr::<u32>(pair(k), 4, 0);
        }
    }
    for (i, t) in tmp.iter().enumerate() {
        wr::<u32>(pair(i), 0, t.0);
        wr::<u32>(pair(i), 4, t.1);
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PGI_PROCESS, HOOK_PGI_COMPUTE_SCRIPT_COMMAND_TASKS_PORT];
