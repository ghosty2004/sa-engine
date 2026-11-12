//! The default-TXD slot getter (batch `std8-ped-1`), ported from the 1.0 US bytes. Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

/// `FUN_007316d0(int32 i)` (0x7316D0, cdecl): the sign-extended TXD slot `i` of the default-TXD table of 16-bit
/// slots at 0xC88004.
#[hook(addr = 0x7316D0, conv = cdecl, name = "FUN_007316d0", expect = "8B 44 24 04 0F BF 04 45 04 80 C8 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn default_txd_slot(i: i32) -> i32 {
    GPtr::<i16>::from_addr(0xC88004usize.wrapping_add((i as usize).wrapping_mul(2))).read() as i32
}

pub const HOOKS: &[HookReg] = &[HOOK_DEFAULT_TXD_SLOT];
