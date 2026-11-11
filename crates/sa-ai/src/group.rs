//! Hand-ported `CEventGroup` functions (PORT-AI-2). Ported from the 1.0 US bytes (the exe wins over gta-reversed).
//! Every call to another catalogued function goes through `orig!` at the original address, event virtuals
//! through their vtable slot.
//!
//! `CEventGroup` (0x4AB340): +0 vtable 0x85AAB0, +4 the ped, +8 the event count, +0xC sixteen event pointers.
//! Event vtable slots used: +0 scalar deleting destructor, +4 `GetEventType`, +8 `GetEventPriority`, +0x10
//! `Clone`, +0x14 `AffectsPed(ped)`, +0x20 `ReportCriminalEvent(ped)`, +0x24 `HasEditableResponse`, +0x38
//! `IsValid(ped)`. `CEventScriptCommand` (type 0x20) holds its task at +0x10.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    fn v_delete(e: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    fn v_event_priority(e: GPtr<u8>) -> i32 = thiscall vslot 0x8 => unported;
    fn v_clone(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x10 => unported;
    fn v_affects_ped(e: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall vslot 0x14 => unported;
    fn v_report_criminal_event(e: GPtr<u8>, ped: GPtr<u8>) = thiscall vslot 0x20 => unported;
    fn v_has_editable_response(e: GPtr<u8>) -> u8 = thiscall vslot 0x24 => unported;
    fn v_is_valid(e: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall vslot 0x38 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CEventEditableResponse::ComputeResponseTaskOfType(CPed*, int32)` (0x4B5730, `ret 8`)
    fn er_compute_response_task_of_type(e: GPtr<u8>, ped: GPtr<u8>, ty: i32) -> u8 = thiscall 0x4B5730 => unported;
    /// `CEventEditableResponse::ComputeResponseTaskType(CPed*, bool)` (0x4B56C0, `ret 8`)
    fn er_compute_response_task_type(e: GPtr<u8>, ped: GPtr<u8>, b: u32) = thiscall 0x4B56C0 => unported;
    /// `CEventEditableResponse::WillRespond` (0x4AC490)
    fn er_will_respond(e: GPtr<u8>) -> u8 = thiscall 0x4AC490 => unported;
    /// `CEventEditableResponse::InformGroup(CPed*)` (0x4B7DF0, `ret 4`)
    fn er_inform_group(e: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x4B7DF0 => unported;
    /// `CEventEditableResponse::InformRespectedFriends(CPed*)` (0x4B2B00, `ret 4`)
    fn er_inform_respected_friends(e: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x4B2B00 => unported;
    /// `CEventEditableResponse::TriggerLookAt(CPed*)` (0x4AC5A0, `ret 4`)
    fn er_trigger_look_at(e: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x4AC5A0 => unported;
    /// `CEventEditableResponse::InformVehicleOccupants(CPed*)` (0x4AC4A0, `ret 4`)
    fn er_inform_vehicle_occupants(e: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x4AC4A0 => unported;
    /// `CPedIntelligence::RecordEventForScript(int32 type, int32 priority)` (0x602050, `ret 8`)
    fn intel_record_event_for_script(i: GPtr<u8>, ty: i32, prio: i32) = thiscall 0x602050 => unported;
}

fn count(this: GPtr<u8>) -> i32 {
    rd::<i32>(this, 8)
}

fn slot_off(i: i32) -> isize {
    0xC + (i as isize) * 4
}

fn ev_at(this: GPtr<u8>, i: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, slot_off(i))
}

/// `CEventGroup::CEventGroup(CPed*)` (0x4AB340, `ret 4`): vtable 0x85AAB0, +4 = ped, count 0, the 16 slots zeroed.
#[hook(addr = 0x4AB340, conv = thiscall, name = "CEventGroup::Constructor", expect = "8B 44 24 04 8B D1")]
pub fn event_group_ctor(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wr::<GPtr<u8>>(this, 4, ped);
    wr::<u32>(this, 0, 0x85AAB0);
    wr::<u32>(this, 8, 0);
    for i in 0..16 {
        wr::<u32>(this, slot_off(i), 0);
    }
    this
}

/// `CEventGroup::Flush(bool keepScriptTask)` (0x4AB370, `ret 4`): with the flag, the first script-command event
/// (type 0x20) whose task is a 0xF3 is kept (the scan reads every slot without a null test); every other event
/// is deleted; the kept one becomes the only event.
#[hook(addr = 0x4AB370, conv = thiscall, name = "CEventGroup::Flush", expect = "51 8A 44 24 08")]
pub fn event_group_flush(this: GPtr<u8>, keep: u8) {
    let mut saved = GPtr::<u8>::NULL;
    if keep != 0 {
        let mut i = 0;
        while i < count(this) {
            let e = ev_at(this, i);
            if v_event_type(e) == 0x20 {
                let t = rd::<GPtr<u8>>(e, 0x10);
                if !t.is_null() && v_task_type(t) == 0xF3 {
                    saved = ev_at(this, i);
                    wr::<u32>(this, slot_off(i), 0);
                    break;
                }
            }
            i += 1;
        }
    }
    let mut i = 0;
    while i < count(this) {
        let e = ev_at(this, i);
        if !e.is_null() {
            v_delete(e, 1);
            wr::<u32>(this, slot_off(i), 0);
        }
        i += 1;
    }
    wr::<u32>(this, 8, 0);
    if !saved.is_null() {
        wr::<GPtr<u8>>(this, 0xC, saved);
        wr::<u32>(this, 8, 1);
    }
}

/// `CEventGroup::Add(CEvent*, bool valid)` (0x4AB420, `ret 8`). With a ped (+4): an editable-response event
/// computes its response flags (`ComputeResponseTaskOfType` 0x4B0 / 0x6A4 / 0x12C, `ComputeResponseTaskType(ped,
/// false)`) and is not stored when it will not respond (except a damage event, type 9, with +0xC set); an event
/// that does not affect the ped returns null; else the editable informs (group, respected friends unless a
/// damage event with +0x25 bit 4, look-at, vehicle occupants), `RecordEventForScript(type, priority)`. Storing
/// (count < 16): `Clone()`, clone+8 = `valid`, `ReportCriminalEvent(ped)` when there is a ped; returns the clone.
#[hook(addr = 0x4AB420, conv = thiscall, name = "CEventGroup::Add-", expect = "51 53 56 8B 74 24 10")]
pub fn event_group_add(this: GPtr<u8>, ev: GPtr<u8>, valid: u8) -> GPtr<u8> {
    if !rd::<GPtr<u8>>(this, 4).is_null() {
        let mut add = true;
        let (mut group, mut friends, mut look) = (0u8, 0u8, 0u8);
        if v_has_editable_response(ev) != 0 {
            group = er_compute_response_task_of_type(ev, rd::<GPtr<u8>>(this, 4), 0x4B0);
            friends = er_compute_response_task_of_type(ev, rd::<GPtr<u8>>(this, 4), 0x6A4);
            look = er_compute_response_task_of_type(ev, rd::<GPtr<u8>>(this, 4), 0x12C);
            er_compute_response_task_type(ev, rd::<GPtr<u8>>(this, 4), 0);
            if er_will_respond(ev) == 0 && !(v_event_type(ev) == 9 && rd::<u8>(ev, 0xC) != 0) {
                add = false;
            }
        }
        if v_affects_ped(ev, rd::<GPtr<u8>>(this, 4)) == 0 {
            return GPtr::NULL;
        }
        if v_has_editable_response(ev) != 0 {
            if group != 0 {
                er_inform_group(ev, rd::<GPtr<u8>>(this, 4));
            }
            if friends != 0 && !(v_event_type(ev) == 9 && rd::<u8>(ev, 0x25) & 0x10 != 0) {
                er_inform_respected_friends(ev, rd::<GPtr<u8>>(this, 4));
            }
            if look != 0 {
                er_trigger_look_at(ev, rd::<GPtr<u8>>(this, 4));
            }
            er_inform_vehicle_occupants(ev, rd::<GPtr<u8>>(this, 4));
        }
        let intel = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 4), 0x47C);
        let prio = v_event_priority(ev);
        let ty = v_event_type(ev);
        intel_record_event_for_script(intel, ty, prio);
        if !add {
            return GPtr::NULL;
        }
    }
    if count(this) >= 0x10 {
        return GPtr::NULL;
    }
    let c = v_clone(ev);
    wr::<u8>(c, 8, valid);
    let ped = rd::<GPtr<u8>>(this, 4);
    if !ped.is_null() {
        v_report_criminal_event(c, ped);
    }
    let n = count(this);
    wr::<GPtr<u8>>(this, slot_off(n), c);
    wr::<i32>(this, 8, rd::<i32>(this, 8).wrapping_add(1));
    c
}

/// `CEventGroup::Remove(CEvent*)` (0x4AB5A0, `ret 4`): a non-null event found in the slots is cleared and
/// deleted (tail jump into its scalar deleting destructor with flag 1).
#[hook(addr = 0x4AB5A0, conv = thiscall, name = "CEventGroup::Remove", expect = "57 8B F9 8B 4C 24 08")]
pub fn event_group_remove(this: GPtr<u8>, ev: GPtr<u8>) {
    if ev.is_null() {
        return;
    }
    for i in 0..count(this) {
        if ev_at(this, i) == ev {
            wr::<u32>(this, slot_off(i), 0);
            v_delete(ev, 1);
            return;
        }
    }
}

/// `CEventGroup::HasEventOfType(const CEvent*)` (0x4AB5E0, `ret 4`): true for null; else whether a stored event
/// has the same `GetEventType()` (the stored event's type is asked first, then the argument's, every round).
#[hook(addr = 0x4AB5E0, conv = thiscall, name = "CEventGroup::HasEventOfType", expect = "51 53 55 8B 6C 24 10")]
pub fn event_group_has_event_of_type(this: GPtr<u8>, ev: GPtr<u8>) -> u8 {
    if ev.is_null() {
        return 1;
    }
    let mut i = 0;
    while i < count(this) {
        let a = v_event_type(ev_at(this, i));
        if v_event_type(ev) == a {
            return 1;
        }
        i += 1;
    }
    0
}

/// `CEventGroup::GetEventOfType(int32)` (0x4AB650, `ret 4`): the first stored event of that type, else null.
#[hook(addr = 0x4AB650, conv = thiscall, name = "CEventGroup::GetEventOfType", expect = "53 55 56 57 8B F9")]
pub fn event_group_get_event_of_type(this: GPtr<u8>, ty: i32) -> GPtr<u8> {
    let mut i = 0;
    while i < count(this) {
        if v_event_type(ev_at(this, i)) == ty {
            return ev_at(this, i);
        }
        i += 1;
    }
    GPtr::NULL
}

/// `CEventGroup::HasEvent(const CEvent*)` (0x4AB6A0, `ret 4`): whether the pointer is in the slots.
#[hook(addr = 0x4AB6A0, conv = thiscall, name = "CEventGroup::HasEvent", expect = "8B 51 08 56 33 C0")]
pub fn event_group_has_event(this: GPtr<u8>, ev: GPtr<u8>) -> u8 {
    (0..count(this)).any(|i| ev_at(this, i) == ev) as u8
}

/// `CEventGroup::TickEvents` (0x4AB6D0): every stored event's +4 (time active) incremented (no null test).
#[hook(addr = 0x4AB6D0, conv = thiscall, name = "CEventGroup::TickEvents", expect = "8B 41 08 33 D2")]
pub fn event_group_tick_events(this: GPtr<u8>) {
    let mut i = 0;
    while i < count(this) {
        let e = ev_at(this, i);
        wr::<i32>(e, 4, rd::<i32>(e, 4).wrapping_add(1));
        i += 1;
    }
}

/// `CEventGroup::Reorganise` (0x4AB700): moves the non-null events to the front in order; count = their number.
#[hook(addr = 0x4AB700, conv = thiscall, name = "CEventGroup::Reorganise", expect = "8B 41 08 83 EC 40")]
pub fn event_group_reorganise(this: GPtr<u8>) {
    let mut tmp = [GPtr::<u8>::NULL; 16];
    let mut n = 0usize;
    let mut i = 0;
    while i < count(this) {
        let e = ev_at(this, i);
        if !e.is_null() {
            tmp[n] = e;
            n += 1;
            wr::<u32>(this, slot_off(i), 0);
        }
        i += 1;
    }
    wr::<i32>(this, 8, n as i32);
    for (k, e) in tmp.iter().take(n).enumerate() {
        wr::<GPtr<u8>>(this, slot_off(k as i32), *e);
    }
}

/// `CEventGroup::RemoveInvalidEvents(bool)` (0x4AB760, `ret 4`): deletes and clears every stored event that is
/// not `IsValid(ped)`, and with the flag every valid one that is not a script command (type 0x20).
#[hook(addr = 0x4AB760, conv = thiscall, name = "CEventGroup::RemoveInvalidEvents", expect = "53 55 8B E9 8B 45 08")]
pub fn event_group_remove_invalid_events(this: GPtr<u8>, flag: u8) {
    let mut i = 0;
    while i < count(this) {
        let e = ev_at(this, i);
        if !e.is_null() {
            let remove =
                v_is_valid(e, rd::<GPtr<u8>>(this, 4)) == 0 || (flag != 0 && v_event_type(e) != 0x20);
            if remove {
                v_delete(e, 1);
                wr::<u32>(this, slot_off(i), 0);
            }
        }
        i += 1;
    }
}

/// `CEventGroup::GetHighestPriorityEvent` (0x4AB7C0): over the slots (no null test), `GetEventPriority` then
/// `GetEventType` of each; a candidate needs priority >= the best so far (strictly > for script commands, type
/// 0x20; the best starts at -1) and `AffectsPed(ped)` (re-read slot); returns the last such event, else null.
#[hook(addr = 0x4AB7C0, conv = thiscall, name = "CEventGroup::GetHighestPriorityEvent", expect = "83 EC 0C 55 8B E9")]
pub fn event_group_get_highest_priority_event(this: GPtr<u8>) -> GPtr<u8> {
    let mut best_p = -1i32;
    let mut best = GPtr::<u8>::NULL;
    let mut i = 0;
    while i < count(this) {
        let e = ev_at(this, i);
        let p = v_event_priority(e);
        let candidate = if v_event_type(e) == 0x20 {
            p > best_p
        } else {
            p >= best_p
        };
        if candidate && v_affects_ped(ev_at(this, i), rd::<GPtr<u8>>(this, 4)) != 0 {
            best_p = p;
            best = e;
        }
        i += 1;
    }
    best
}

/// `CEventGroup::HasScriptCommandOfTaskType(int32)` (0x4AB840, `ret 4`): whether a stored script command (type
/// 0x20) holds a task (+0x10) of that type.
#[hook(addr = 0x4AB840, conv = thiscall, name = "CEventGroup::HasScriptCommandOfTaskType", expect = "53 55 56 8B E9")]
pub fn event_group_has_script_command_of_task_type(this: GPtr<u8>, ty: i32) -> u8 {
    let mut i = 0;
    while i < count(this) {
        let e = ev_at(this, i);
        if !e.is_null() && v_event_type(e) == 0x20 {
            let t = rd::<GPtr<u8>>(ev_at(this, i), 0x10);
            if !t.is_null() && v_task_type(t) == ty {
                return 1;
            }
        }
        i += 1;
    }
    0
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_EVENT_GROUP_CTOR,
    HOOK_EVENT_GROUP_FLUSH,
    HOOK_EVENT_GROUP_ADD,
    HOOK_EVENT_GROUP_REMOVE,
    HOOK_EVENT_GROUP_HAS_EVENT_OF_TYPE,
    HOOK_EVENT_GROUP_GET_EVENT_OF_TYPE,
    HOOK_EVENT_GROUP_HAS_EVENT,
    HOOK_EVENT_GROUP_TICK_EVENTS,
    HOOK_EVENT_GROUP_REORGANISE,
    HOOK_EVENT_GROUP_REMOVE_INVALID_EVENTS,
    HOOK_EVENT_GROUP_GET_HIGHEST_PRIORITY_EVENT,
    HOOK_EVENT_GROUP_HAS_SCRIPT_COMMAND_OF_TASK_TYPE,
];
