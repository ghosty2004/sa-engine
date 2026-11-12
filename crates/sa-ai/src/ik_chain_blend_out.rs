//! `CTaskSimpleIKChain::BlendOut(int32)` (0x633C40, batch `std12-ai-1`): reached when a ped's look-at / point IK
//! chain ends (ambient peds looking at CJ). Ported from the 1.0 US bytes (gta-reversed as a reading aid). Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

/// `CTaskSimpleIKChain::BlendOut(int32 time)` (0x633C40, thiscall, `ret 4`): unless already blending out (+0x54): a
/// time of -1 (+8) becomes 0, the end time (+0x48) = now + `time`, and +0x54 = 1.
#[hook(addr = 0x633C40, conv = thiscall, name = "CTaskSimpleIKChain::BlendOut", expect = "8B 41 54 85 C0 75 23",
       unverified = "std12-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn ik_chain_blend_out(this: GPtr<u8>, time: i32) {
    if rd::<u32>(this, 0x54) != 0 {
        return;
    }
    if rd::<i32>(this, 8) == -1 {
        wr::<i32>(this, 8, 0);
    }
    wr::<i32>(this, 0x48, g::<i32>(0xB7CB84).read().wrapping_add(time));
    wr::<u32>(this, 0x54, 1);
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_IK_CHAIN_BLEND_OUT];
