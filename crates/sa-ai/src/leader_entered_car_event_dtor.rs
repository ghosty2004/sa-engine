//! `CEventLeaderEnteredCarAsDriver`'s complete destructor 0x48E220 (batch `std14-ai-2`), reached when CJ, as his
//! group's leader, enters a car. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

use crate::bindings::editable_response_dtor as event_dtor;
use sa_world::bindings::entity_clean_up_old_reference;

/// `CEventLeaderEnteredCarAsDriver::~CEventLeaderEnteredCarAsDriver` (0x48E220, thiscall; entry `push -1; jmp
/// 0x40166A` into the SEH prologue): vtable 0x85A198, the vehicle reference (+0x14) dropped when set, then the
/// `CEvent` destructor.
#[hook(addr = 0x48E220, conv = thiscall, name = "FUN_0048e220", expect = "6A FF E9 43 34 F7 FF",
       unverified = "std14-ai-2", route = "L2", hazards = "heap-writes", recs = "enter-car")]
pub fn leader_entered_car_as_driver_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85A198);
    let v = rd::<GPtr<u8>>(this, 0x14);
    if !v.is_null() {
        entity_clean_up_old_reference(v, this.byte_add(0x14));
    }
    event_dtor(this);
}

pub const HOOKS: &[HookReg] = &[HOOK_LEADER_ENTERED_CAR_AS_DRIVER_DTOR];
