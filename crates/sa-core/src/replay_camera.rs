//! `CReplay::ShouldStandardCameraBeProcessed` (batch `std8-core-1`), ported from the 1.0 US bytes. Unverified
//! (docs/verify/HOWTO.md).

use crate::abi::HookReg;
use crate::hook;
use crate::mem::GPtr;

/// `CReplay::ShouldStandardCameraBeProcessed()` (0x45C440, a `.HOODLUM` stub into 0x156E610): `Mode` (0xA43088)
/// != 1 (`MODE_PLAYBACK`) in AL by `setne`; the rest of EAX is the caller's.
#[hook(addr = 0x45C440, conv = reg("ret = al:entry"), name = "CReplay::ShouldStandardCameraBeProcessed",
       expect = "E9 CB 21 11 01", unverified = "std8-core-1", route = "L2", hazards = "ax-al-return",
       recs = "grove-walk")]
pub fn replay_should_standard_camera_be_processed() -> u8 {
    (GPtr::<u8>::from_addr(0xA43088).read() != 1) as u8
}

pub const HOOKS: &[HookReg] = &[HOOK_REPLAY_SHOULD_STANDARD_CAMERA_BE_PROCESSED];
