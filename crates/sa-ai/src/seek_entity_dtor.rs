//! `CTaskComplexSeekEntity<CEntitySeekPosCalculatorStandard>` destructor (batch `mvpl-ai-3`). Ported from the 1.0 US
//! bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CTaskComplex::~CTaskComplex()` (`FUN_0061a3d0`)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => unported;
}

/// `FUN_0046ad70` (0x46AD70, thiscall): `CTaskComplexSeekEntity<CEntitySeekPosCalculatorStandard>::~...`. Vtable
/// 0x859DF8; the seek target (+0xC), when set, has its reference to +0xC cleaned up (`CleanUpOldReference`); the
/// position calculator member (+0x40) gets its vtable 0x859DC4 back; then the `CTaskComplex` part (0x61A3D0). The SEH
/// frame of the original is not needed (nothing in between throws).
#[hook(addr = 0x46ad70, conv = thiscall, name = "FUN_0046ad70", expect = "6A FF 68 63 7E 83 00",
       unverified = "mvpl-ai-3", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the task (task pool) and the target entity's reference list")]
pub fn seek_entity_standard_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x859DF8);
    let target = rd::<GPtr<u8>>(this, 0xC);
    if !target.is_null() {
        entity_clean_up_old_reference(target, this.byte_add(0xC));
    }
    wr::<u32>(this, 0x40, 0x859DC4);
    task_complex_dtor(this);
}

pub const HOOKS: &[HookReg] = &[HOOK_SEEK_ENTITY_STANDARD_DTOR];
