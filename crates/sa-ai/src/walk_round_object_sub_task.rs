//! `CTaskComplexWalkRoundObject::CreateSubTask(int type, CPed*)` (0x655290, batch `std13-ai-1`; catalogue
//! `FUN_00655290`). Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskSimpleStandStill(int ms, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, ms: i32, looped: u32, idle: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskComplexWalkRoundObject::CreateRouteTask(CPed*)` (0x655140, `ret 4`)
    fn create_route_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x655140 => crate::walkround::object_create_route_task;
}

/// `CTaskComplexWalkRoundObject::CreateSubTask(int type, CPed*)` (0x655290, thiscall, `ret 8`, SEH frame): type 0xCB
/// a `CTaskSimpleStandStill(500, 0, 0, 8.0)` (0x20 bytes), type 0x389 `CreateRouteTask(ped)`, any other type null.
#[hook(addr = 0x655290, conv = thiscall, name = "FUN_00655290", expect = "64 A1 00 00 00 00 8B 54",
       unverified = "std13-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn walk_round_object_create_sub_task(this: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> {
    match ty {
        0xCB => {
            let t = task_new(0x20);
            if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 500, 0, 0, 0x4100_0000) }
        }
        0x389 => create_route_task(this, ped),
        _ => GPtr::NULL,
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_WALK_ROUND_OBJECT_CREATE_SUB_TASK];
