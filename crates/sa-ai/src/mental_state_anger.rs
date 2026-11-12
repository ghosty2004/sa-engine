//! `CMentalState::IncrementAnger(int32)` (0x421050, batch `std12-ai-1`): reached every frame a ped is bumped or
//! annoyed (the ambient peds around CJ in Grove Street). The entry is a `.HOODLUM` stub (`jmp 0x156FC20`); ported from
//! those bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`.
const TIME_MS: usize = 0xB7CB84;

/// `CMentalState::IncrementAnger(int32 anger)` (0x421050 → 0x156FC20, thiscall, `ret 4`). The anger timer (+0x178
/// start, +0x17C interval, +0x180 started, +0x181 reset pending) must be started; a pending reset restarts it at the
/// current time (read through the protection's decoded pointer `[0x15605C0] - [0x741FD8] - 1` = 0xB7CB84). Once
/// `now >= start + interval` (unsigned): start = now, interval 3000 ms, started, and the anger byte (+0x174) grows
/// by the argument's low byte.
#[hook(addr = 0x421050, conv = thiscall, name = "CMentalState::IncrementAnger", expect = "E9 CB EB 14 01",
       unverified = "std12-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn increment_anger(this: GPtr<u8>, anger: u32) {
    if rd::<u8>(this, 0x180) == 0 {
        return;
    }
    if rd::<u8>(this, 0x181) != 0 {
        wr::<u32>(this, 0x178, g::<u32>(TIME_MS).read());
        wr::<u8>(this, 0x181, 0);
    }
    let end = rd::<u32>(this, 0x17C).wrapping_add(rd::<u32>(this, 0x178));
    let now = g::<u32>(TIME_MS).read();
    if now >= end {
        wr::<u32>(this, 0x178, now);
        wr::<u32>(this, 0x17C, 0xBB8);
        wr::<u8>(this, 0x180, 1);
        wr::<u8>(this, 0x174, rd::<u8>(this, 0x174).wrapping_add(anger as u8));
    }
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_INCREMENT_ANGER];
