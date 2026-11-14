//! The `CEventPedToFlee` destructor (INTRO-2: peds flee the Ballas drive-by after the intro cutscene). Ported from
//! the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity** ref)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
}

/// `CEventPedToFlee::~CEventPedToFlee()` (0x4AF2D0, thiscall; entry `push -1; jmp 0x402297`, the protection chunk
/// that pushes the SEH handler): vtable 0x85B178, `CleanUpOldReference(&+0xC)` when the ped (+0xC) is set, then
/// `CEvent`'s vtable 0x85AB38 (its inlined destructor). The SEH frame is not reproduced.
#[hook(addr = 0x4AF2D0, conv = thiscall, name = "CEventPedToFlee::~CEventPedToFlee", expect = "6A FF E9 C0 2F F5 FF",
       unverified = "intro3-14", route = "L2", hazards = "heap-writes", recs = "intro",
       note = "SEH frame (pushed by the protection chunk 0x402297) not reproduced")]
pub fn event_ped_to_flee_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x0085_B178);
    let p = rd::<GPtr<u8>>(this, 0xC);
    if !p.is_null() {
        entity_clean_up_old_reference(p, this.byte_add(0xC));
    }
    wr::<u32>(this, 0, 0x0085_AB38);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_PED_TO_FLEE_DTOR];
