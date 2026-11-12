//! WRITER-NATIVE (batch `native-2`): task `Clone` vslots reached on the native walk
//! (`CTaskSimpleGoToPoint::Clone`, `CTaskComplexChat::Clone`). Ported from the 1.0 US bytes; gta-reversed
//! as a reading aid.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new_c(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => crate::mvpa_ai1::task_operator_new;
    /// `CTaskSimpleGoToPoint::CTaskSimpleGoToPoint(moveState, const CVector&, radius, mustOverShoot, stopExactly)` (0x667CD0)
    fn goto_point_ctor(t: GPtr<u8>, state: u32, target: GPtr<u8>, radius: u32, over: u8, exact: u8) -> GPtr<u8> = thiscall 0x667CD0 => crate::walk::goto_point_ctor;
    /// `CTaskComplexChat::CTaskComplexChat(bool, CPed*, int32, int16)` (0x682CB0)
    fn chat_ctor(t: GPtr<u8>, is_chatter: u8, partner: GPtr<u8>, stage: i32, context: i16) -> GPtr<u8> = thiscall 0x682CB0 => crate::chat_task_ctor::task_complex_chat_ctor;
}

/// `CTaskSimpleGoToPoint::Clone()` (0x66CC60, SEH frame): `operator new(0x30)`, then the constructor with
/// +8 (move state), &+0xC (target), +0x18 (radius), +0x20 bit 0 (must overshoot) and bit 3 (stop exactly).
/// Null when the pool is full.
#[hook(addr = 0x66CC60, conv = thiscall, name = "CTaskSimpleGoToPoint::Clone", this = "CTaskSimpleGoToPoint",
       expect = "6A FF 68 2B 53 84 00",
       unverified = "native-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn goto_point_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new_c(0x30);
    if t.is_null() {
        return GPtr::NULL;
    }
    let b = rd::<u8>(this, 0x20);
    goto_point_ctor(t, rd::<u32>(this, 8), this.byte_add(0xC), rd::<u32>(this, 0x18), b & 1, (b >> 3) & 1)
}

/// `CTaskComplexChat::Clone()` (0x682DA0, SEH frame): `operator new(0x20)`, the constructor with +0xC (byte),
/// +0x10, +0x14 and +0x18 (word), then the clone's +0x1A byte = this +0x1A. The original writes +0x1A also
/// when the pool returned null (`[0x1A]`, kept).
#[hook(addr = 0x682DA0, conv = thiscall, name = "CTaskComplexChat::Clone", this = "CTaskComplexChat",
       expect = "6A FF 68 AB 66 84 00",
       unverified = "native-2", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "null-pool path writes address 0x1A (original bug)")]
pub fn chat_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new_c(0x20);
    let r = if t.is_null() {
        GPtr::NULL
    } else {
        chat_ctor(t, rd::<u8>(this, 0xC), rd::<GPtr<u8>>(this, 0x10), rd::<i32>(this, 0x14), rd::<i16>(this, 0x18))
    };
    wr::<u8>(r, 0x1A, rd::<u8>(this, 0x1A));
    r
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_GOTO_POINT_CLONE, HOOK_CHAT_CLONE];
