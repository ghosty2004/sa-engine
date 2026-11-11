//! Phase B registrations (`#[native]`) of existing ports whose entries cannot be hooked in phase A
//! (STD-6: their docs say "not hookable"; collected by address).

/// Every `#[native]` STD-6 added to existing ports of this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[
    crate::loadmonitor::NATIVE_DESTRUCTOR,
    crate::loadmonitor::NATIVE_RENDER,
];
