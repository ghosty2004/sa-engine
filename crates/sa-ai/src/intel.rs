//! Hand-ported `CPedIntelligence` task getters and small helpers (PORT-AI-2). Ported from the 1.0 US bytes (the
//! exe wins over gta-reversed). Every call to another catalogued function goes through `orig!` at the original
//! address. `CPedIntelligence`: +0 the ped, +4 the `CTaskManager`, +0xBC hearing range, +0xC0 seeing range.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn tm_get_task_secondary(tm: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskManager::GetSimplestActiveTask` (0x6819D0)
    fn tm_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTaskManager::FindTaskByType(int32 slot, int32 type)` (0x6817D0, `ret 8`)
    fn tm_find_task_by_type(tm: GPtr<u8>, slot: i32, ty: i32) -> GPtr<u8> = thiscall 0x6817D0 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0, `ret 0xC`)
    fn tm_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: i32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CAcquaintance::GetAcquaintances(int32)` (0x608970, `ret 4`)
    fn acquaintance_get(a: GPtr<u8>, i: i32) -> u32 = thiscall 0x608970 => unported;
    /// `CPedType::GetPedFlag(ePedType)` (0x608830, cdecl)
    fn ped_type_get_ped_flag(t: u32) -> u32 = cdecl 0x608830 => unported;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360, cdecl)
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*, uint32 flags)` (0x4D6A70)
    fn clump_get_first_association_flags(clump: u32, flags: u32) -> GPtr<u8> = cdecl 0x4D6A70 => unported;
    /// `RpAnimBlendGetNextAssociation(CAnimBlendAssociation*)` (0x4D6AB0)
    fn anim_get_next_association(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D6AB0 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn crt_stricmp(a: u32, b: u32) -> i32 = cdecl 0x8229B6 => unported;
}

fn tm(this: GPtr<u8>) -> GPtr<u8> {
    this.byte_add(4)
}

/// The inlined "secondary task `slot` if it has type `ty`" pattern: `GetTaskSecondary` called once for the null
/// test, once for the type and once more for the result.
fn secondary_of_type(this: GPtr<u8>, slot: i32, ty: i32) -> GPtr<u8> {
    if tm_get_task_secondary(tm(this), slot).is_null() {
        return GPtr::NULL;
    }
    if v_task_type(tm_get_task_secondary(tm(this), slot)) != ty {
        return GPtr::NULL;
    }
    tm_get_task_secondary(tm(this), slot)
}

/// The simplest active task if its type is `ty` (one `GetTaskType` call), else null.
fn simplest_of_type(this: GPtr<u8>, ty: i32) -> GPtr<u8> {
    let t = tm_get_simplest_active_task(tm(this));
    if !t.is_null() && v_task_type(t) == ty {
        t
    } else {
        GPtr::NULL
    }
}

/// `CPedIntelligence::AddTaskEventResponseTemp(CTask*, bool)` (0x600DE0, `ret 8`): `SetTask(task, 1, b)`.
#[hook(addr = 0x600DE0, conv = thiscall, name = "CPedIntelligence::AddTaskEventResponseTemp", expect = "8B 44 24 08 8B 54 24 04")]
pub fn intel_add_task_event_response_temp(this: GPtr<u8>, task: GPtr<u8>, b: u32) {
    tm_set_task(tm(this), task, 1, b);
}

/// `CPedIntelligence::AddTaskEventResponseNonTemp(CTask*, bool)` (0x600E00, `ret 8`): `SetTask(task, 2, b)`.
#[hook(addr = 0x600E00, conv = thiscall, name = "CPedIntelligence::AddTaskEventResponseNonTemp", expect = "8B 44 24 08 8B 54 24 04")]
pub fn intel_add_task_event_response_non_temp(this: GPtr<u8>, task: GPtr<u8>, b: u32) {
    tm_set_task(tm(this), task, 2, b);
}

/// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0, `ret 4`): `CTaskManager::FindTaskByType` over the primary
/// slots 4, 3, 1, 2 in that order; the first non-null result (slot 0 is not searched).
#[hook(addr = 0x600EE0, conv = thiscall, name = "CPedIntelligence::FindTaskByType", expect = "56 57 8B 7C 24 0C")]
pub fn intel_find_task_by_type(this: GPtr<u8>, ty: i32) -> GPtr<u8> {
    for slot in [4, 3, 1] {
        let t = tm_find_task_by_type(tm(this), slot, ty);
        if !t.is_null() {
            return t;
        }
    }
    tm_find_task_by_type(tm(this), 2, ty)
}

/// `CPedIntelligence::GetTaskFighting` (0x600F30): secondary slot 0 when it is a 0x3F8 (`TASK_SIMPLE_FIGHT`); the
/// compare continues through the island 0x406471.
#[hook(addr = 0x600F30, conv = thiscall, name = "CPedIntelligence::GetTaskFighting", expect = "56 8D 71 04 6A 00")]
pub fn intel_get_task_fighting(this: GPtr<u8>) -> GPtr<u8> {
    secondary_of_type(this, 0, 0x3F8)
}

/// `CPedIntelligence::GetTaskThrow` (0x600FB0): secondary slot 0 when it is a 0x3FA.
#[hook(addr = 0x600FB0, conv = thiscall, name = "CPedIntelligence::GetTaskThrow", expect = "56 8D 71 04 6A 00")]
pub fn intel_get_task_throw(this: GPtr<u8>) -> GPtr<u8> {
    secondary_of_type(this, 0, 0x3FA)
}

/// `CPedIntelligence::GetTaskHold(bool ignoreSimplest)` (0x600FF0, `ret 4`): secondary slot 4 when it is a 0x133;
/// else, unless the argument is set, the simplest active task when its type is 0x134 or (second call) 0x135.
#[hook(addr = 0x600FF0, conv = thiscall, name = "CPedIntelligence::GetTaskHold", expect = "56 8D 71 04 6A 04")]
pub fn intel_get_task_hold(this: GPtr<u8>, ignore: u8) -> GPtr<u8> {
    let t = secondary_of_type(this, 4, 0x133);
    if !t.is_null() {
        return t;
    }
    if ignore != 0 {
        return GPtr::NULL;
    }
    let t = tm_get_simplest_active_task(tm(this));
    if !t.is_null() && (v_task_type(t) == 0x134 || v_task_type(t) == 0x135) {
        return t;
    }
    GPtr::NULL
}

/// `CPedIntelligence::GetTaskSwim` (0x601070): the simplest active task when it is a 0x518.
#[hook(addr = 0x601070, conv = thiscall, name = "CPedIntelligence::GetTaskSwim", expect = "56 83 C1 04 E8 57 09 08 00")]
pub fn intel_get_task_swim(this: GPtr<u8>) -> GPtr<u8> {
    simplest_of_type(this, 0x518)
}

/// `CPedIntelligence::GetTaskDuck(bool ignoreSimplest)` (0x6010A0, `ret 4`): secondary slot 1 when it is a 0x19F
/// (`TASK_SIMPLE_DUCK`); else, unless the argument is set, the simplest active task when it is a 0x19F.
#[hook(addr = 0x6010A0, conv = thiscall, name = "CPedIntelligence::GetTaskDuck", expect = "56 8D 71 04 6A 01")]
pub fn intel_get_task_duck(this: GPtr<u8>, ignore: u8) -> GPtr<u8> {
    let t = secondary_of_type(this, 1, 0x19F);
    if !t.is_null() {
        return t;
    }
    if ignore != 0 {
        return GPtr::NULL;
    }
    simplest_of_type(this, 0x19F)
}

/// `CPedIntelligence::GetTaskJetPack` (0x601110): for a player, the simplest active task when it is a 0x517.
#[hook(addr = 0x601110, conv = thiscall, name = "CPedIntelligence::GetTaskJetPack", expect = "56 8B F1 8B 0E")]
pub fn intel_get_task_jet_pack(this: GPtr<u8>) -> GPtr<u8> {
    if ped_is_player(rd::<GPtr<u8>>(this, 0)) == 0 {
        return GPtr::NULL;
    }
    simplest_of_type(this, 0x517)
}

/// `CPedIntelligence::GetTaskInAir` (0x601150): the simplest active task when it is a 0xF1.
#[hook(addr = 0x601150, conv = thiscall, name = "CPedIntelligence::GetTaskInAir", expect = "56 83 C1 04 E8 77 08 08 00")]
pub fn intel_get_task_in_air(this: GPtr<u8>) -> GPtr<u8> {
    simplest_of_type(this, 0xF1)
}

/// `CPedIntelligence::GetTaskClimb` (0x601180): the simplest active task when it is a 0xFE.
#[hook(addr = 0x601180, conv = thiscall, name = "CPedIntelligence::GetTaskClimb", expect = "56 83 C1 04 E8 47 08 08 00")]
pub fn intel_get_task_climb(this: GPtr<u8>) -> GPtr<u8> {
    simplest_of_type(this, 0xFE)
}

/// `CPedIntelligence::GetUsingParachute` (0x6011B0): the active weapon (slot `(int8)+0x718`, type at +0x5A0 +
/// 0x1C*slot) is the parachute (0x2E), the ped is not in a vehicle (+0x46C bit 0) and +0x40 bit 8 is clear, and
/// one of the clump's associations with flag 0x10 has a hierarchy whose anim block name (0xB5D4A0 + 0x20 * block)
/// equals the string at 0x86CD54 (`_stricmp`).
#[hook(addr = 0x6011B0, conv = thiscall, name = "CPedIntelligence::GetUsingParachute", expect = "8B 01 0F BE 88 18 07 00 00")]
pub fn intel_get_using_parachute(this: GPtr<u8>) -> u8 {
    let ped = rd::<GPtr<u8>>(this, 0);
    let slot = rd::<i8>(ped, 0x718) as i32;
    if rd::<i32>(ped, (0x5A0 + slot.wrapping_mul(0x1C)) as isize) != 0x2E {
        return 0;
    }
    if rd::<u8>(ped, 0x46C) & 1 != 0 || rd::<u32>(ped, 0x40) & 0x100 != 0 {
        return 0;
    }
    let mut a = clump_get_first_association_flags(rd::<u32>(ped, 0x18), 0x10);
    while !a.is_null() {
        let block = rd::<u32>(rd::<GPtr<u8>>(a, 0x14), 0xC);
        if crt_stricmp((block << 5).wrapping_add(0xB5D4A0), 0x86CD54) == 0 {
            return 1;
        }
        a = anim_get_next_association(a);
    }
    0
}

/// `CPedIntelligence::IsFriendlyWith(const CPed&)` (0x601BC0, `ret 4`; continues through the island 0x40A1AC):
/// 1 when both peds have the same ped type (+0x598) or the other's ped-type flag is in this ped's acquaintance
/// sets 0 (respect) or 1 (like), else 0 (EAX).
#[hook(addr = 0x601BC0, conv = thiscall, name = "CPedIntelligence::IsFriendlyWith", expect = "53 55 56 8B F1")]
pub fn intel_is_friendly_with(this: GPtr<u8>, other: GPtr<u8>) -> u32 {
    let a0 = acquaintance_get(rd::<GPtr<u8>>(this, 0).byte_add(0x4E0), 0);
    let a1 = acquaintance_get(rd::<GPtr<u8>>(this, 0).byte_add(0x4E0), 1);
    let f = ped_type_get_ped_flag(rd::<u32>(other, 0x598));
    if rd::<u32>(rd::<GPtr<u8>>(this, 0), 0x598) == rd::<u32>(other, 0x598) || a0 & f != 0 || a1 & f != 0 {
        1
    } else {
        0
    }
}

/// `CPedIntelligence::IsThreatenedBy(const CPed&)` (0x601C30, `ret 4`): 1 when the other's ped-type flag is in
/// this ped's acquaintance sets 3 (dislike) or 4 (hate), else 0 (EAX).
#[hook(addr = 0x601C30, conv = thiscall, name = "CPedIntelligence::IsThreatenedBy", expect = "56 8B F1 8B 0E")]
pub fn intel_is_threatened_by(this: GPtr<u8>, other: GPtr<u8>) -> u32 {
    let a3 = acquaintance_get(rd::<GPtr<u8>>(this, 0).byte_add(0x4E0), 3);
    let a4 = acquaintance_get(rd::<GPtr<u8>>(this, 0).byte_add(0x4E0), 4);
    let f = ped_type_get_ped_flag(rd::<u32>(other, 0x598));
    (a3 & f != 0 || a4 & f != 0) as u32
}

/// `CPedIntelligence::GetMoveStateFromGoToTask` (0x601D70): the simplest active task's +8 (move state) when it is
/// a go-to task (`CTask::IsGoToTask`), else 1 (PEDMOVE_STILL).
#[hook(addr = 0x601D70, conv = thiscall, name = "CPedIntelligence::GetMoveStateFromGoToTask", expect = "56 83 C1 04 E8 57 FC 07 00")]
pub fn intel_get_move_state_from_go_to_task(this: GPtr<u8>) -> i32 {
    let t = tm_get_simplest_active_task(tm(this));
    if !t.is_null() && task_is_go_to_task(t) != 0 {
        return rd::<i32>(t, 8);
    }
    1
}

/// `CPedIntelligence::SetHearingRange(float)` (0x600BE0, `ret 4`): +0xBC.
#[hook(addr = 0x600BE0, conv = thiscall, name = "CPedIntelligence::SetHearingRange", expect = "8B 44 24 04 89 81 BC 00 00 00")]
pub fn intel_set_hearing_range(this: GPtr<u8>, v: u32) {
    wr::<u32>(this, 0xBC, v);
}

/// `CPedIntelligence::SetSeeingRange(float)` (0x600BF0, `ret 4`): +0xC0.
#[hook(addr = 0x600BF0, conv = thiscall, name = "CPedIntelligence::SetSeeingRange", expect = "8B 44 24 04 89 81 C0 00 00 00")]
pub fn intel_set_seeing_range(this: GPtr<u8>, v: u32) {
    wr::<u32>(this, 0xC0, v);
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_INTEL_ADD_TASK_EVENT_RESPONSE_TEMP,
    HOOK_INTEL_ADD_TASK_EVENT_RESPONSE_NON_TEMP,
    HOOK_INTEL_FIND_TASK_BY_TYPE,
    HOOK_INTEL_GET_TASK_FIGHTING,
    HOOK_INTEL_GET_TASK_THROW,
    HOOK_INTEL_GET_TASK_HOLD,
    HOOK_INTEL_GET_TASK_SWIM,
    HOOK_INTEL_GET_TASK_DUCK,
    HOOK_INTEL_GET_TASK_JET_PACK,
    HOOK_INTEL_GET_TASK_IN_AIR,
    HOOK_INTEL_GET_TASK_CLIMB,
    HOOK_INTEL_GET_USING_PARACHUTE,
    HOOK_INTEL_IS_FRIENDLY_WITH,
    HOOK_INTEL_IS_THREATENED_BY,
    HOOK_INTEL_GET_MOVE_STATE_FROM_GO_TO_TASK,
    HOOK_INTEL_SET_HEARING_RANGE,
    HOOK_INTEL_SET_SEEING_RANGE,
];
