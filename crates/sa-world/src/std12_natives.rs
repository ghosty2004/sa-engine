//! STD-12: phase B registrations (`#[native]`) of this crate's ports whose entries cannot be hooked in phase A.

/// Every `#[native]` STD-12 added to this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[crate::world::clear_scan_codes::NATIVE_CLEAR_SCAN_CODES];
