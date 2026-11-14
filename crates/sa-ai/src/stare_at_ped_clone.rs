//! WRITER-NATIVE (batch `native-22`): `CTaskComplexStareAtPed::Clone` (0x660390), reached in fist fights on the
//! native build. Ported from the 1.0 US bytes.

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

use crate::bindings::stare_at_ped_ctor;
use sa_core::bindings::task_new as task_new_c;

/// `CTaskComplexStareAtPed::Clone()` (0x660390, SEH frame): the remaining time (timer +0x18 / +0x1C running, flag
/// +0x20: `start + duration - now`, else the stored time +0x14), `operator new(0x28)` and the constructor with
/// +0xC, +0x10 and that time. Null when the pool is full.
#[hook(addr = 0x660390, conv = thiscall, name = "CTaskComplexStareAtPed::Clone",
       expect = "6A FF 68 0B 46 84 00",
       unverified = "native-22", route = "L2", hazards = "heap-writes", recs = "weapon-combat,grove-walk")]
pub fn stare_at_ped_clone(this: GPtr<u8>) -> GPtr<u8> {
    let time = if rd::<u8>(this, 0x20) != 0 {
        rd::<u32>(this, 0x1C)
            .wrapping_sub(g::<u32>(0xB7_CB84).read())
            .wrapping_add(rd::<u32>(this, 0x18))
    } else {
        rd::<u32>(this, 0x14)
    };
    let t = task_new_c(0x28);
    if t.is_null() {
        return t;
    }
    stare_at_ped_ctor(t, GPtr::from_addr(rd::<u32>(this, 0xC) as usize), GPtr::from_addr(rd::<u32>(this, 0x10) as usize), time)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_STARE_AT_PED_CLONE];
