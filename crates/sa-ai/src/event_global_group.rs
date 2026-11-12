//! STD-7: `GetEventGlobalGroup` (batch `std7-ai-1`), ported from the 1.0 US bytes; unverified.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// The global event group singleton pointer
const EVENT_GLOBAL_GROUP: usize = 0xA9AF6C;
/// `CEventGlobalGroup` vtable
const VTABLE: u32 = 0x85AAB4;

orig! {
    /// `operator new(size)` (0x82119A)
    fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
}

/// `GetEventGlobalGroup()` (0x4ABA50, cdecl): the singleton (0xA9AF6C); when null, a new 0x4C-byte object
/// (`operator new`; null stays null) with vtable 0x85AAB4, +4 = +8 = 0 and +0xC..+0x4C zeroed, stored and
/// returned.
#[hook(addr = 0x4ABA50, conv = cdecl, name = "RwHelper::GetEventGlobalGroup", expect = "A1 6C AF A9 00 85 C0",
       unverified = "std7-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn get_event_global_group() -> GPtr<u8> {
    let cell = g::<GPtr<u8>>(EVENT_GLOBAL_GROUP);
    let cur = cell.read();
    if !cur.is_null() {
        return cur;
    }
    let p = operator_new(0x4C);
    if !p.is_null() {
        wr::<u32>(p, 4, 0);
        wr::<u32>(p, 8, 0);
        for i in 0..16 {
            wr::<u32>(p, 0xC + i * 4, 0);
        }
        wr::<u32>(p, 0, VTABLE);
    }
    cell.write(p);
    p
}

pub const HOOKS: &[HookReg] = &[HOOK_GET_EVENT_GLOBAL_GROUP];
