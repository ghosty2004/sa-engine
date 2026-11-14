//! Destructors of complex tasks that hold one entity reference (batch `frontier-1-2`), ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskComplex::~CTaskComplex()` (0x61A3D0)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => unported;
}

/// `FUN_006833d0` (0x6833D0): the complete destructor of the complex task with vtable 0x8706F8 (slot 0, its scalar
/// deleting destructor, calls it): `vtbl = 0x8706F8`, the entity reference at +0xC cleaned up when set, then
/// `CTaskComplex::~CTaskComplex`. Reached when a fist fight ends (FRONTIER-1 melee run, iteration 2930).
#[hook(addr = 0x6833D0, conv = thiscall, name = "FUN_006833d0", expect = "6A FF 68 B8 67 84 00",
       unverified = "frontier-1-2", route = "L2", hazards = "heap-writes",
       note = "the SEH frame is not reproduced")]
pub fn complex_task_8706f8_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x87_06F8);
    let e = rd::<GPtr<u8>>(this, 0xC);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(0xC));
    }
    task_complex_dtor(this);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_COMPLEX_TASK_8706F8_DTOR];
