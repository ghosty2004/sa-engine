//! `CEventStuckInAir::GetEventPriority` (0x4B1600, batch `std10-ai-1`): ported from the 1.0 US bytes (gta-reversed's
//! version as a reading aid). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEventHandler::GetCurrentEventType()` (0x4B8CC0, thiscall)
    fn get_current_event_type(h: GPtr<u8>) -> i32 = thiscall 0x4B8CC0 => crate::mvpa_ai2::event_handler_get_current_event_type;
}

/// `CEventStuckInAir::GetEventPriority()` (0x4B1600, thiscall): 75 when the ped (+0xC) is set, its event handler
/// (intelligence +0x47C, +0x34) is not responding to event type 0x5D and the intelligence's +0x70 is above 1 (signed);
/// else 63.
#[hook(addr = 0x4B1600, conv = thiscall, name = "CEventStuckInAir::GetEventPriority", expect = "56 8B F1 8B 46 0C 85 C0 74 27",
       unverified = "std10-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn event_stuck_in_air_get_event_priority(this: GPtr<u8>) -> i32 {
    let ped = rd::<GPtr<u8>>(this, 0xC);
    if ped.is_null() {
        return 0x3F;
    }
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    if get_current_event_type(intel.byte_add(0x34)) == 0x5D {
        return 0x3F;
    }
    let intel = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x47C);
    if rd::<i32>(intel, 0x70) > 1 { 0x4B } else { 0x3F }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_STUCK_IN_AIR_GET_EVENT_PRIORITY];
