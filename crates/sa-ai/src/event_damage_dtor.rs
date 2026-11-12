//! WRITER-NATIVE (batch `native-6`): the non-virtual `CEventDamage` destructor body (0x4AD960), reached by
//! every on-stack damage event (weapon impacts, car hits, script damage) on the native walk.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
}

/// `CEventDamage::~CEventDamage()` (0x4AD960, SEH frame, no `delete`): vtable 0x85AD78, `CleanUpOldReference`
/// on the source entity (+0x14) when set, then the `CEvent` vtable 0x85AB38.
#[hook(addr = 0x4AD960, conv = thiscall, name = "FUN_004ad960",
       expect = "6A FF 68 E8 9A 83 00",
       unverified = "native-6", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_damage_destructor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85_AD78);
    let src = rd::<GPtr<u8>>(this, 0x14);
    if !src.is_null() {
        entity_clean_up_old_reference(src, this.byte_add(0x14));
    }
    wr::<u32>(this, 0, 0x85_AB38);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_DAMAGE_DESTRUCTOR];
