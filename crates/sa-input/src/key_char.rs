//! `FUN_00719360`: the font character of a key code (INTRO-2: the BMX help text names the controls). Ported from the
//! 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

/// The two-byte entries whose first byte is the font character of key codes 0x80 and up (0x8D5AD8).
const KEY_CHARS: usize = 0x8D_5AD8;

/// `FUN_00719360(uint8 c)` (0x719360, cdecl), AL: codes below 0x80 are their own character; others read the byte
/// at `0x8D5AD8 + c * 2`.
#[hook(addr = 0x719360, conv = cdecl, name = "FUN_00719360", expect = "8A 44 24 04 3C 80",
       unverified = "intro3-16", route = "L2", hazards = "none", recs = "intro")]
pub fn key_char(c: u32) -> u8 {
    let c = c as u8;
    if c < 0x80 {
        return c;
    }
    GPtr::<u8>::from_addr(KEY_CHARS + usize::from(c) * 2).read()
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_KEY_CHAR];
