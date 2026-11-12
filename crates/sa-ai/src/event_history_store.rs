//! `CEventHandlerHistory::StoreActiveEvent` (0x4B8BF0, batch `std12-ai-1`): reached when a ped's event handler keeps
//! its active event (ambient peds reacting near CJ). Ported from the 1.0 US bytes (gta-reversed as a reading aid).
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// virtual `CEvent::~CEvent()` deleting destructor (vtable slot 0, `push 1`)
    fn event_delete(e: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
}

/// `CEventHandlerHistory::StoreActiveEvent()` (0x4B8BF0, thiscall): the stored event (+0xC) deleted; the active
/// event (+4) becomes the stored one and the store timer (+0x10 start = now, +0x14 = 2000 ms, +0x18 started) starts.
#[hook(addr = 0x4B8BF0, conv = thiscall, name = "CEventHandlerHistory::StoreActiveEvent", expect = "56 8B F1 8B 4E 0C",
       unverified = "std12-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_history_store_active_event(this: GPtr<u8>) {
    let stored = rd::<GPtr<u8>>(this, 0xC);
    if !stored.is_null() {
        event_delete(stored, 1);
    }
    wr::<u32>(this, 0xC, rd::<u32>(this, 4));
    wr::<u32>(this, 0x10, g::<u32>(0xB7CB84).read());
    wr::<u32>(this, 0x14, 0x7D0);
    wr::<u8>(this, 0x18, 1);
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_HISTORY_STORE_ACTIVE_EVENT];
