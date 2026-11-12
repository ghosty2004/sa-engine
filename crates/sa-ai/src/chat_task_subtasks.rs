//! `CTaskComplexChat`'s sub-task functions (batch `attr-6`): `CreateFirstSubTask` 0x682E20 and `ControlSubTask`
//! 0x683060, reached by peds chatting at attractors. Layout (0x20 bytes): +0xC chatter flag, +0x10 the partner (a
//! registered reference), +0x14 stage, +0x18 global speech context (i16), +0x1A finished, +0x1C no-chat counter.
//! Ported from the 1.0 US bytes (gta-reversed as a reading aid; where it differs, the bytes are followed). Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPed::Say(uint16, uint32, float, bool, bool, bool)` (0x5EFFE0, `ret 0x18`), the float as its bits
    fn ped_say(p: GPtr<u8>, ctx: u32, offset: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CInterestingEvents::Add(type, CEntity*)` (0x602590, `ret 8`)
    fn interesting_events_add(ie: GPtr<u8>, t: u32, e: GPtr<u8>) = thiscall 0x602590 => unported;
    /// `CGeneral::GetRandomNumberInRange(int, int)` (0x407180; draws its own rand)
    fn random_int(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CTaskSimpleRunAnim(AssocGroupId, AnimationId, float blend, bool)` (0x61A8B0, `ret 0x10`)
    fn run_anim_ctor(t: GPtr<u8>, group: u32, id: u32, blend: u32, b: u32) -> GPtr<u8> = thiscall 0x61A8B0 => unported;
    /// `CTaskSimpleChat(int durationMs)` (0x5F65A0, `ret 4`)
    fn simple_chat_ctor(t: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x5F65A0 => unported;
    /// `CTaskSimpleStandStill(int ms, bool looped, bool useAnimIdle, float blend)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, ms: i32, looped: u32, idle: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTask::MakeAbortable(CPed*, int priority, const CEvent*)` (vtable slot 0x18, `ret 0xC`)
    fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CPedIntelligence::FindTaskByType(int)` (0x600EE0, `ret 4`)
    fn intel_find_task_by_type(i: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CPed::GetPedTalking()` (0x5EFF50)
    fn ped_get_talking(ped: GPtr<u8>) -> u8 = thiscall 0x5EFF50 => unported;
}

/// `g_InterestingEvents`
const INTERESTING_EVENTS: usize = 0xC0B058;

/// `CTaskComplexChat::CreateFirstSubTask(CPed*)` (0x682E20, thiscall, `ret 4`, SEH frame). Null without a partner.
/// Not the chatter: `CTaskSimpleStandStill(999999, 0, 0, 8.0)`. The chatter says the context when it is above -1
/// (signed 16-bit; the pushed dword's upper half is the partner pointer's, as the original leaves EAX), then
/// `g_InterestingEvents.Add(1, ped)`; contexts 0xA4, 0xA7 and 0xA8 end the chat with `GetRandomNumberInRange(0, 3)`:
/// 0..2 a `CTaskSimpleRunAnim(0, 0xA6 + n, 4.0, 0)`, else (unreachable) `CTaskSimpleChat(4000)`; any other context
/// `CTaskSimpleChat(999999)`.
#[hook(addr = 0x682E20, conv = thiscall, name = "CTaskComplexChat::CreateFirstSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "attr-6", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn chat_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let partner = rd::<u32>(this, 0x10);
    if partner == 0 {
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0xC) == 0 {
        let t = task_new(0x20);
        return if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 999_999, 0, 0, 0x4100_0000) };
    }
    let ctx = rd::<u16>(this, 0x18);
    if ctx as i16 > -1 {
        ped_say(ped, (partner & 0xFFFF_0000) | ctx as u32, 0, 0x3F80_0000, 0, 0, 0);
    }
    interesting_events_add(GPtr::from_addr(INTERESTING_EVENTS), 1, ped);
    let c = rd::<i16>(this, 0x18) as i32;
    if c == 0xA4 || (c > 0xA6 && c <= 0xA8) {
        let n = random_int(0, 3);
        if (0..3).contains(&n) {
            let t = task_new(0x20);
            return if t.is_null() { GPtr::NULL } else { run_anim_ctor(t, 0, 0xA6 + n as u32, 0x4080_0000, 0) };
        }
        let t = task_new(0x34);
        return if t.is_null() { GPtr::NULL } else { simple_chat_ctor(t, 4000) };
    }
    let t = task_new(0x34);
    if t.is_null() { GPtr::NULL } else { simple_chat_ctor(t, 999_999) }
}

/// `CTaskComplexChat::ControlSubTask(CPed*)` (0x683060, thiscall, `ret 4`). Null without a partner, or when finished
/// (+0x1A) and the sub-task's `MakeAbortable(ped, 1, null)` succeeds. The partner's chat task (its intelligence's
/// `FindTaskByType(0x4C2)`): a lower stage, or not finished with the same chatter flag, finishes this one. A chatter
/// not talking: a non-chatter partner task that is not finished finishes both; the no-chat counter (+0x1C) counts
/// up and above 50 finishes this one and the partner's. Returns the sub-task.
#[hook(addr = 0x683060, conv = thiscall, name = "CTaskComplexChat::ControlSubTask", expect = "56 8B F1 8B 46 10 85 C0",
       unverified = "attr-6", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn chat_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(this, 0x10) == 0 {
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0x1A) != 0 && task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0 {
        return GPtr::NULL;
    }
    let partner = rd::<GPtr<u8>>(this, 0x10);
    let other = intel_find_task_by_type(rd::<GPtr<u8>>(partner, 0x47C), 0x4C2);
    if !other.is_null() {
        if rd::<i32>(other, 0x14) < rd::<i32>(this, 0x14) {
            wr::<u8>(this, 0x1A, 1);
        }
        if rd::<u8>(other, 0x1A) == 0 && rd::<u8>(this, 0xC) == rd::<u8>(other, 0xC) {
            wr::<u8>(this, 0x1A, 1);
        }
    }
    if rd::<u8>(this, 0xC) != 0 && ped_get_talking(ped) == 0 {
        if !other.is_null() && rd::<u8>(other, 0x1A) == 0 && rd::<u8>(other, 0xC) == 0 {
            wr::<u8>(this, 0x1A, 1);
            wr::<u8>(other, 0x1A, 1);
        }
        let n = rd::<i32>(this, 0x1C).wrapping_add(1);
        wr::<i32>(this, 0x1C, n);
        if n > 0x32 {
            wr::<u8>(this, 0x1A, 1);
            if !other.is_null() {
                wr::<u8>(other, 0x1A, 1);
            }
        }
    }
    rd::<GPtr<u8>>(this, 8)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CHAT_CREATE_FIRST_SUB_TASK, HOOK_CHAT_CONTROL_SUB_TASK];
