//! Phase B registrations (`#[native]`) of existing ports whose entries cannot be hooked in phase A
//! (STD-6: their docs say "not hookable"; collected by address).

/// Every `#[native]` STD-6 added to existing ports of this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[
    crate::ccm::NATIVE_GET_JOY_BUTTON_JUST_UP,
    crate::ccm::NATIVE_AFFECT_BUTTON_DOWN_DEBUG_STUFF,
    crate::ccm::NATIVE_AFFECT_BUTTON_UP_DEBUG_STUFF,
    crate::pad::NATIVE_DESTRUCTOR,
    crate::pad::NATIVE_PROCESS_PC_SPECIFIC_STUFF,
    crate::pad::NATIVE_STOP_SHAKING,
    crate::pad::NATIVE_EXIT_VEHICLE_JUST_DOWN,
];
