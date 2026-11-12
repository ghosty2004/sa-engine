//! STD-7: phase B registrations (`#[native]`) of this crate's ports whose entries cannot be hooked in phase A.

/// Every `#[native]` STD-7 added to this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[crate::world::zones::NATIVE_FIND_SMALLEST_ZONE_FOR_POSITION];
