//! STD-15 (batch `std15-ai-3`): the one-instruction destructor body 0x4ADE90 the drive reaches (resets the vtable to
//! `CEvent`'s, 0x85AB38). Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
//! (`CPopulation::ChoosePolicePedOccupation` 0x610F40 is registered natively in `short_body_natives.rs`.)

use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

/// The destructor body 0x4ADE90 (thiscall; `CEventDeath` and other events chain to it): `*this = 0x85AB38`.
#[hook(addr = 0x4ADE90, conv = thiscall, name = "FUN_004ade90", expect = "C7 01 38 AB 85 00 C3",
       unverified = "std15-ai-3", route = "L2", hazards = "none", recs = "car-drive")]
pub fn event_base_dtor_body(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85AB38);
}

pub const HOOKS: &[HookReg] = &[HOOK_EVENT_BASE_DTOR_BODY];
