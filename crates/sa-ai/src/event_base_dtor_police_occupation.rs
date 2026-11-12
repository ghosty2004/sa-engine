//! STD-15 (batch `std15-ai-3`): two one-instruction bodies the drive reaches: the destructor body 0x4ADE90 (resets
//! the vtable to `CEvent`'s, 0x85AB38) and `CPopulation::ChoosePolicePedOccupation` (0x610F40, returns 0). Ported
//! from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

/// The destructor body 0x4ADE90 (thiscall; `CEventDeath` and other events chain to it): `*this = 0x85AB38`.
#[hook(addr = 0x4ADE90, conv = thiscall, name = "FUN_004ade90", expect = "C7 01 38 AB 85 00 C3",
       unverified = "std15-ai-3", route = "L2", hazards = "none", recs = "car-drive")]
pub fn event_base_dtor_body(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85AB38);
}

/// `CPopulation::ChoosePolicePedOccupation()` (0x610F40, cdecl): always 0.
#[hook(addr = 0x610F40, conv = cdecl, name = "CPopulation::ChoosePolicePedOccupation", expect = "33 C0 C3 90 90",
       unverified = "std15-ai-3", route = "L2", hazards = "none", recs = "car-drive")]
pub fn choose_police_ped_occupation() -> u32 {
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_EVENT_BASE_DTOR_BODY, HOOK_CHOOSE_POLICE_PED_OCCUPATION];
