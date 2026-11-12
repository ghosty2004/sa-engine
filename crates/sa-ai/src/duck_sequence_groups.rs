//! Small `CTaskSimpleDuck` control functions, `CTaskComplexSequence` task-array helpers and `CPedGroups` statics
//! (PORT-AI-7). Ported from the 1.0 US bytes (the exe wins over gta-reversed). Every call to another catalogued
//! function goes through `orig!` at the original address.
//!
//! `CTaskSimpleDuck`: +8 start time, +0xC duration (u16), +0xE length (i16), +0x10 / +0x14 the down / up anim
//! associations, +0x18 finished, +0x19 / +0x1A / +0x1B flags, +0x1C / +0x20 move commands, +0x24 control type.
//! `CTaskComplexSequence`: +0x10 tasks[8], +0x38 can-be-emptied flag, +0x3C reference count.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CPedGroups::ms_activeGroups` (bool[8])
const ACTIVE: usize = 0xC098E0;
/// `CPedGroups::ms_groups`
const GROUPS: u32 = 0xC09920;
/// `sizeof(CPedGroup)`
const GROUP_SIZE: u32 = 0x2D4;
/// 0.0f
const K_ZERO: usize = 0x858B50;

orig! {
    /// Virtual slot +0 (scalar deleting destructor, `ret 4`).
    fn v_delete(o: GPtr<u8>, flags: u32) -> u32 = thiscall vslot 0x0 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskComplexSequence::Flush` (0x632C10)
    fn sequence_flush(s: GPtr<u8>) = thiscall 0x632C10 => unported;
    /// `CPedGroup::RemoveAllFollowers` (0x5FB7D0)
    fn group_remove_all_followers(g: GPtr<u8>) = thiscall 0x5FB7D0 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(void (*)(CAnimBlendAssociation*, void*), void*)` (0x4CEBE0,
    /// `ret 8`)
    fn assoc_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
}

/// `CDefaultAnimCallback::DefaultAnimCB` (the callback the duck task restores on its associations)
const DEFAULT_ANIM_CB: u32 = 0x4D1390;

// ---- CTaskSimpleDuck ---------------------------------------------------------------------------------------------

/// `CTaskSimpleDuck::SetDuckTimer(uint16)` (0x692530, `ret 4`): unless the control type (+0x24) is 4, restarts the
/// timer (+8 = now) with the new duration word (+0xC).
#[hook(addr = 0x692530, conv = thiscall, name = "CTaskSimpleDuck::SetDuckTimer", expect = "80 79 24 04 74 11",
       unverified = "ai7-6", route = "L2", hazards = "heap-writes")]
pub fn duck_set_duck_timer(this: GPtr<u8>, ms: u32) {
    if rd::<u8>(this, 0x24) != 4 {
        wr::<u32>(this, 8, g::<u32>(TIME_MS).read());
        wr::<u16>(this, 0xC, ms as u16);
    }
}

/// `CTaskSimpleDuck::SetControlType(uint8)` (0x6924F0, `ret 4`): refused (AL = 0) once finished (+0x18) or aborting
/// (+0x19); otherwise type 4 clears the duration word, the type is stored (+0x24), the timer restarts and +0x1B is
/// set; AL = 1. The upper EAX bytes stay the caller's on both paths.
#[hook(addr = 0x6924F0, conv = thiscall, name = "CTaskSimpleDuck::SetControlType", expect = "8A 41 18 84 C0",
       unverified = "ai7-6", route = "L2", hazards = "ax-al-return,heap-writes")]
pub fn duck_set_control_type(this: GPtr<u8>, ty: u32) -> u8 {
    if rd::<u8>(this, 0x18) != 0 || rd::<u8>(this, 0x19) != 0 {
        return 0;
    }
    let ty = ty as u8;
    if ty == 4 {
        wr::<u16>(this, 0xC, 0);
    }
    let now = g::<u32>(TIME_MS).read();
    wr::<u8>(this, 0x24, ty);
    wr::<u32>(this, 8, now);
    wr::<u8>(this, 0x1B, 1);
    1
}

/// `CTaskSimpleDuck::AbortBecauseOfOtherDuck(CPed*)` (0x692340, `ret 4`; the ped is unused): the down association
/// (+0x10) gets the default finish callback and is dropped; the up association (+0x14) blends out (delta -8.0)
/// with the default callback and is dropped; +0x1A and +0x18 are set.
#[hook(addr = 0x692340, conv = thiscall, name = "CTaskSimpleDuck::AbortBecauseOfOtherDuck", expect = "56 8B F1 8B 4E 10",
       unverified = "ai7-6", route = "L2", hazards = "heap-writes")]
pub fn duck_abort_because_of_other_duck(this: GPtr<u8>, _ped: GPtr<u8>) {
    let down = rd::<GPtr<u8>>(this, 0x10);
    if !down.is_null() {
        assoc_set_finish_callback(down, DEFAULT_ANIM_CB, 0);
        wr::<u32>(this, 0x10, 0);
    }
    let up = rd::<GPtr<u8>>(this, 0x14);
    if !up.is_null() {
        wr::<f32>(up, 0x1C, -8.0);
        assoc_set_finish_callback(rd::<GPtr<u8>>(this, 0x14), DEFAULT_ANIM_CB, 0);
        wr::<u32>(this, 0x14, 0);
    }
    wr::<u8>(this, 0x1A, 1);
    wr::<u8>(this, 0x18, 1);
}

/// `CTaskSimpleDuck::RestartTask(CPed*)` (0x692390, `ret 4`): returns `this`. When +0x1A is set the ped's flag
/// 0x4000000 (+0x46C) is set and +0x1A cleared; the timer restarts (+8 = now), and when the length word (+0xE, i16)
/// is not negative it becomes `1000 - _ftol((rand() & 0xFFFF) · [0x858B14] · [0x870BE8])` (rand at 0x6923C0).
#[hook(addr = 0x692390, conv = thiscall, name = "CTaskSimpleDuck::RestartTask", expect = "56 8B F1 8A 46 1A",
       unverified = "ai7-6", route = "L2", hazards = "heap-writes", note = "draws one rand (site 0x6923C0)")]
pub fn duck_restart_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(this, 0x1A) != 0 {
        wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 0x400_0000);
        wr::<u8>(this, 0x1A, 0);
    }
    let len = rd::<u16>(this, 0xE) as i16;
    wr::<u32>(this, 8, g::<u32>(TIME_MS).read());
    if len > -1 {
        let r = sa_core::orig::at_site(0x6923C0, rand) & 0xFFFF;
        let v = ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x870BE8).read()));
        wr::<u16>(this, 0xE, 1000i32.wrapping_sub(v) as u16);
    }
    this
}

/// `CTaskSimpleDuck::IsTaskInUseByOtherTasks` (0x61C3D0): EAX = 1 when either move command (+0x1C, +0x20) is not
/// 0.0 (or unordered), there is no down association (+0x10), its blend amount (+0x18) is not below [0x858624]
/// (`test ah, 5; jnp`), +0x19 is set or the length word (+0xE) is positive; else 0.
#[hook(addr = 0x61C3D0, conv = thiscall, name = "CTaskSimpleDuck::IsTaskInUseByOtherTasks", expect = "D9 41 1C D8 1D 50 8B 85 00",
       unverified = "ai7-6", route = "L2", hazards = "none")]
pub fn duck_is_task_in_use_by_other_tasks(this: GPtr<u8>) -> u32 {
    let zero = x(g::<f32>(K_ZERO).read());
    if x(rd::<f32>(this, 0x1C)).fcom(zero).test_parity(0x44) || x(rd::<f32>(this, 0x20)).fcom(zero).test_parity(0x44) {
        return 1;
    }
    duck_in_use_tail(this)
}

/// The shared tail of `IsTaskInUseByOtherTasks` / `StopFireGun` (from the down-association test on).
fn duck_in_use_tail(this: GPtr<u8>) -> u32 {
    let down = rd::<GPtr<u8>>(this, 0x10);
    if down.is_null() || !x(rd::<f32>(down, 0x18)).fcom(x(g::<f32>(0x858624).read())).test_parity(0x05) {
        return 1;
    }
    if rd::<u8>(this, 0x19) != 0 || rd::<u16>(this, 0xE) as i16 > 0 {
        return 1;
    }
    0
}

/// `CTaskSimpleDuck::StopFireGun` (0x61C420): as `IsTaskInUseByOtherTasks` without the +0x20 move-command test.
#[hook(addr = 0x61C420, conv = thiscall, name = "CTaskSimpleDuck::StopFireGun", expect = "D9 41 1C D8 1D 50 8B 85 00",
       unverified = "ai7-6", route = "L2", hazards = "none")]
pub fn duck_stop_fire_gun(this: GPtr<u8>) -> u32 {
    if x(rd::<f32>(this, 0x1C)).fcom(x(g::<f32>(K_ZERO).read())).test_parity(0x44) {
        return 1;
    }
    duck_in_use_tail(this)
}

// ---- CTaskComplexSequence ----------------------------------------------------------------------------------------

/// `CTaskComplexSequence::DeRegister` (0x636BC0): decrements the reference count (+0x3C); when it reaches 0 with
/// the can-be-emptied flag (+0x38) set, clears the flag and flushes (tail jump).
#[hook(addr = 0x636BC0, conv = thiscall, name = "CTaskComplexSequence::DeRegister", expect = "FF 49 3C 75 10",
       unverified = "ai7-6", route = "L2", hazards = "heap-writes", note = "Flush frees the sequence's tasks")]
pub fn sequence_de_register(this: GPtr<u8>) {
    let n = rd::<u32>(this, 0x3C).wrapping_sub(1);
    wr::<u32>(this, 0x3C, n);
    if n == 0 && rd::<u8>(this, 0x38) != 0 {
        wr::<u8>(this, 0x38, 0);
        sequence_flush(this);
    }
}

/// `CTaskComplexSequence::AddTask(int32 index, CTask*)` (0x632D50, `ret 8`): for an index below 8 (signed) the old
/// task there is deleted and the new one stored; otherwise the new task is deleted.
#[hook(addr = 0x632D50, conv = thiscall, name = "CTaskComplexSequence::AddTask-1", expect = "56 57 8B 7C 24 0C",
       unverified = "ai7-6", route = "L2", hazards = "heap-writes", note = "frees tasks (task pool)")]
pub fn sequence_add_task_at(this: GPtr<u8>, index: i32, task: GPtr<u8>) {
    if index < 8 {
        let off = 0x10 + index as isize * 4;
        let old = rd::<GPtr<u8>>(this, off);
        if !old.is_null() {
            v_delete(old, 1);
        }
        wr::<GPtr<u8>>(this, off, task);
    } else if !task.is_null() {
        v_delete(task, 1);
    }
}

/// `CTaskComplexSequence::Contains(eTaskType)` (0x41BF10, `ret 4`): AL = 1 when one of the 8 tasks has the type
/// (one `GetTaskType` call per non-null task, in order).
#[hook(addr = 0x41BF10, conv = thiscall, name = "CTaskComplexSequence::Contains", expect = "53 8B 5C 24 08",
       unverified = "ai7-6", route = "L2", hazards = "ax-al-return",
       note = "the original's upper EAX bytes are the last GetTaskType result's (callers test AL)")]
pub fn sequence_contains(this: GPtr<u8>, ty: i32) -> u8 {
    for k in 0..8 {
        let t = rd::<GPtr<u8>>(this, 0x10 + k * 4);
        if !t.is_null() && v_task_type(t) == ty {
            return 1;
        }
    }
    0
}

// ---- CPedGroups --------------------------------------------------------------------------------------------------

/// `CPedGroups::RegisterKillByPlayer` (0x5F7E30): increments the kill counter 0xC098EC when the byte 0xC098E8 is set.
#[hook(addr = 0x5F7E30, conv = cdecl, name = "CPedGroups::RegisterKillByPlayer", expect = "A0 E8 98 C0 00",
       unverified = "ai7-6", route = "L2", hazards = "none")]
pub fn groups_register_kill_by_player() {
    if g::<u8>(0xC098E8).read() != 0 {
        let c = g::<u32>(0xC098EC);
        c.write(c.read().wrapping_add(1));
    }
}

/// `CPedGroups::IsGroupLeader(CPed*)` (0x5F7E40, cdecl): AL = 1 when an active group's leader is `ped` (non-null).
/// EAX is returned whole: the loop pointer's upper bytes (`&ms_groups[i].leader` of the match, or the end pointer
/// 0xC0AFE8 when none matches) with AL = 1 / 0.
#[hook(addr = 0x5F7E40, conv = cdecl, name = "CPedGroups::IsGroupLeader", expect = "8B 54 24 04 33 C9",
       unverified = "ai7-6", route = "L2", hazards = "none")]
pub fn groups_is_group_leader(ped: GPtr<u8>) -> u32 {
    for i in 0..8u32 {
        let leader = GROUPS + 0x28 + i * GROUP_SIZE;
        if g::<u8>(ACTIVE + i as usize).read() != 0 && !ped.is_null() && g::<u32>(leader as usize).read() == ped.addr() as u32 {
            return (leader & 0xFFFF_FF00) | 1;
        }
    }
    (GROUPS + 0x28 + 8 * GROUP_SIZE) & 0xFFFF_FF00
}

/// `CPedGroups::RemoveAllFollowersFromGroup(int32)` (0x5FB8A0, cdecl): `ms_groups[id].RemoveAllFollowers()` for an
/// active group (tail jump).
#[hook(addr = 0x5FB8A0, conv = cdecl, name = "CPedGroups::RemoveAllFollowersFromGroup", expect = "8B 4C 24 04 8A 81 E0 98 C0 00",
       unverified = "ai7-6", route = "L2", hazards = "heap-writes")]
pub fn groups_remove_all_followers_from_group(id: i32) {
    if g::<u8>((ACTIVE as i32 + id) as u32 as usize).read() != 0 {
        let grp = (id as u32).wrapping_mul(GROUP_SIZE).wrapping_add(GROUPS);
        group_remove_all_followers(GPtr::from_addr(grp as usize));
    }
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_DUCK_SET_DUCK_TIMER,
    HOOK_DUCK_SET_CONTROL_TYPE,
    HOOK_DUCK_ABORT_BECAUSE_OF_OTHER_DUCK,
    HOOK_DUCK_RESTART_TASK,
    HOOK_DUCK_IS_TASK_IN_USE_BY_OTHER_TASKS,
    HOOK_DUCK_STOP_FIRE_GUN,
    HOOK_SEQUENCE_DE_REGISTER,
    HOOK_SEQUENCE_ADD_TASK_AT,
    HOOK_SEQUENCE_CONTAINS,
    HOOK_GROUPS_REGISTER_KILL_BY_PLAYER,
    HOOK_GROUPS_IS_GROUP_LEADER,
    HOOK_GROUPS_REMOVE_ALL_FOLLOWERS_FROM_GROUP,
];
