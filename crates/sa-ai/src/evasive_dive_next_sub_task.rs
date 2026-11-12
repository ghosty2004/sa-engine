//! `CTaskComplexEvasiveDiveAndGetUp::CreateNextSubTask` (INTRO-2: CJ dives away from the Ballas drive-by after the
//! intro cutscene). Ported from the 1.0 US bytes; gta-reversed `TaskComplexEvasiveDiveAndGetUp.cpp` as a reading aid.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskComplexEvasiveDiveAndGetUp::CreateSubTask(int32 taskType)` (0x653820, `ret 4`)
    fn create_sub_task(t: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x653820 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, cdecl, ST0)
    fn general_limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
}

/// `CTaskComplexEvasiveDiveAndGetUp::CreateNextSubTask(CPed*)` (0x656000, thiscall, `ret 4`).
///
/// By the sub-task's (+8) type: 0x386 → `CreateSubTask(0x1F7)`; 0x1F7 → 0xCA; 0xCD → 0x516; 0xCA → the ped's
/// current heading (+0x558) becomes `LimitRadianAngle(heading) - (double)f32(pi / 2)` (`fst` into +0x558, `fstp`
/// into the aimed heading +0x55C), then 0xCD. Other types: null.
#[hook(addr = 0x656000, conv = thiscall, name = "CTaskComplexEvasiveDiveAndGetUp::CreateNextSubTask",
       expect = "56 8B F1 8B 4E 08",
       unverified = "intro3-10", route = "L2", hazards = "heap-writes", recs = "intro")]
pub fn evasive_dive_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    match v_get_task_type(rd::<GPtr<u8>>(this, 8)) {
        0x386 => create_sub_task(this, 0x1F7),
        0x1F7 => create_sub_task(this, 0xCA),
        0xCD => create_sub_task(this, 0x516),
        0xCA => {
            let a = X87::from_f64_exact(general_limit_radian_angle(rd::<u32>(ped, 0x558)))
                .fsub(X87::from_f64_exact(f64::from_bits(0x3FF9_21FB_6000_0000)))
                .to_f32();
            wr::<f32>(ped, 0x558, a);
            wr::<f32>(ped, 0x55C, a);
            create_sub_task(this, 0xCD)
        }
        _ => GPtr::NULL,
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_EVASIVE_DIVE_CREATE_NEXT_SUB_TASK];
