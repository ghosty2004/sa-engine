//! `CCarAI::CarHasReasonToStop` (stub 0x41C050, `#[native]`). Ported from the 1.0 US bytes (the body is
//! in `.HOODLUM` at 0x156FB90). Unverified (docs/verify/HOWTO.md).

use sa_core::mem::GPtr;

use crate::common::{g, wr};

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

/// `CCarAI::CarHasReasonToStop(CVehicle*)` (stub 0x41C050, cdecl): the autopilot's mission start time (+0x3AC) = now.
///
/// Registered for phase B only (`#[native]`): protection code reads the stub's bytes as data (`lea` at 0x14B2FB5 /
/// 0x14B2FC3 sees 0x41C054).
#[sa_core::native(addr = 0x41C050, conv = cdecl, name = "CCarAI::CarHasReasonToStop")]
pub fn car_has_reason_to_stop(veh: GPtr<u8>) {
    wr::<u32>(veh, 0x3AC, g::<u32>(TIME_MS).read());
}

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_CAR_HAS_REASON_TO_STOP];
