//! `CTaskComplexSmartFleePoint::MakeAbortable` (0x65BDC0, batch `std12-ai-1`): reached when a fleeing ambient ped's
//! task is aborted. Unreversed in gta-reversed; ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::MakeAbortable(CPed*, eAbortPriority, const CEvent*)` (vtable slot 0x18)
    fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: u32, ev: GPtr<u8>) -> u32 = thiscall vslot 0x18 => unported;
}

/// `CTaskComplexSmartFleePoint::MakeAbortable(CPed* ped, eAbortPriority priority, const CEvent* event)` (0x65BDC0,
/// thiscall, `ret 0xC`). Priority 0 (urgent): +0x28 = -1 and the timer at +0x34 restarted (start = now, interval -1,
/// started). Then the sub-task's (+8) `MakeAbortable(ped, priority, event)`, whose EAX is returned unchanged (a tail
/// call in effect).
#[hook(addr = 0x65BDC0, conv = thiscall, name = "CTaskComplexSmartFleePoint::MakeAbortable", expect = "8B 44 24 08 85 C0 56",
       unverified = "std12-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn smart_flee_point_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: u32, ev: GPtr<u8>) -> u32 {
    if prio == 0 {
        wr::<i32>(this, 0x28, -1);
        wr::<u32>(this, 0x34, g::<u32>(0xB7CB84).read());
        wr::<i32>(this, 0x38, -1);
        wr::<u8>(this, 0x3C, 1);
    }
    task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, prio, ev)
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_SMART_FLEE_POINT_MAKE_ABORTABLE];
