//! STD-15 (batch `std15-ai-5`): phase-B registrations (`#[native]`) of two bodies shorter than the 5-byte hook patch,
//! reached while driving and leaving a car: `CBridge::ThisIsABridgeObjectMovingUp` (0x41BCB0, `xor al, al; ret`) and
//! `CPopulation::ChoosePolicePedOccupation` (0x610F40, `xor eax, eax; ret`). Ported from the 1.0 US bytes.
//! Unverified (docs/verify/HOWTO.md).

/// `CBridge::ThisIsABridgeObjectMovingUp(int32 modelIndex)` (0x41BCB0, cdecl): always false (AL = 0).
#[sa_core::native(addr = 0x41BCB0, conv = cdecl, name = "CBridge::ThisIsABridgeObjectMovingUp")]
pub fn bridge_this_is_a_bridge_object_moving_up(_model: i32) -> u8 {
    0
}

/// `CPopulation::ChoosePolicePedOccupation()` (0x610F40, cdecl): always 0.
#[sa_core::native(addr = 0x610F40, conv = cdecl, name = "CPopulation::ChoosePolicePedOccupation")]
pub fn choose_police_ped_occupation() -> u32 {
    0
}

/// This module's phase-B registrations.
pub const NATIVES: &[sa_core::abi::NativeReg] =
    &[NATIVE_BRIDGE_THIS_IS_A_BRIDGE_OBJECT_MOVING_UP, NATIVE_CHOOSE_POLICE_PED_OCCUPATION];
