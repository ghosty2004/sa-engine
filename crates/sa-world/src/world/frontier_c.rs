//! Small world helpers reached through the original by ported code (FRONTIER-2 lane C). Ported from the 1.0 US bytes.
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;

/// The female ped-type test (stub 0x43A6A0 → .HOODLUM 0x15697D0, cdecl; unnamed): EAX = 1 for ped types 22
/// (prostitute) and 5 (civilian female), else 0.
#[hook(addr = 0x43A6A0, conv = cdecl, name = "thunk_FUN_015697d0", expect = "E9 2B F1 12 01",
       unverified = "frontier2-c2", route = "L2", hazards = "none")]
pub fn is_female_ped_type(ty: u32) -> u32 {
    u32::from(ty == 0x16 || ty == 5)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_IS_FEMALE_PED_TYPE];
