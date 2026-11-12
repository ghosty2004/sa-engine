//! WRITER-NATIVE (batch `native-19`): the `CEventPassObject` constructor (0x65DC70), reached on the native walk.
//! Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEvent::CEvent()` (0x4ABFC0)
    fn event_ctor(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ABFC0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// `CEventPassObject::CEventPassObject(CEntity* giver, bool dontPassObject)` (0x65DC70, `ret 8`; the entry `push
/// -1; jmp 0x4011D8` pushes the SEH handler from a protection chunk): `CEvent` base, vtable 0x86F8B8, +0xC = giver
/// with `RegisterReference(&+0xC)` when set, +0x10 = the byte. Returns `this`.
#[hook(addr = 0x65DC70, conv = thiscall, name = "CEventPassObject::Constructor",
       expect = "6A FF E9 61 35 DA FF",
       unverified = "native-19", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "entry jmp to the protection chunk 0x4011D8 (SEH handler push)")]
pub fn event_pass_object_ctor(this: GPtr<u8>, giver: GPtr<u8>, dont_pass: u32) -> GPtr<u8> {
    event_ctor(this);
    wr::<u32>(this, 0, 0x86_F8B8);
    wr::<GPtr<u8>>(this, 0xC, giver);
    if !giver.is_null() {
        entity_register_reference(giver, this.byte_add(0xC));
    }
    wr::<u8>(this, 0x10, dont_pass as u8);
    this
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_PASS_OBJECT_CTOR];
