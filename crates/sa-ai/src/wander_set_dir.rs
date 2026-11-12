//! `CTaskComplexWander::SetDir` (0x669D30, batch `std13-ai-1`). Ported from the 1.0 US bytes. Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

/// `FUN_00669d30` (0x669D30, thiscall, `ret 4`; `CTaskComplexWander::SetDir(uint8)`): when the direction byte (+0x10,
/// zero-extended) differs from the whole argument dword, the byte becomes the argument's low byte and flag 2 is set
/// in +0x24.
#[hook(addr = 0x669D30, conv = thiscall, name = "FUN_00669d30", expect = "0F B6 51 10 8B 44 24 04",
       unverified = "std13-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn wander_set_dir(this: GPtr<u8>, dir: u32) {
    if u32::from(rd::<u8>(this, 0x10)) != dir {
        wr::<u8>(this, 0x10, dir as u8);
        wr::<u8>(this, 0x24, rd::<u8>(this, 0x24) | 2);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_WANDER_SET_DIR];
