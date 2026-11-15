//! FRONTIER-2 lane B: phase B natives of PED functions whose 1.0 US entry is shorter than the 5-byte patch.

use sa_core::mem::GPtr;

/// `CPlayerInfo::CancelPlayerEnteringCars(CVehicle*)` (0x56E860, thiscall): empty (`ret 4`).
#[sa_core::native(addr = 0x56E860, conv = thiscall, name = "CPlayerInfo::CancelPlayerEnteringCars")]
pub fn player_info_cancel_player_entering_cars(_this: GPtr<u8>, _v: GPtr<u8>) {}

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_PLAYER_INFO_CANCEL_PLAYER_ENTERING_CARS];
