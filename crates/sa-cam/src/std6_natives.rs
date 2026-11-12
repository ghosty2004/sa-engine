//! Phase B registrations (`#[native]`) of existing ports whose entries cannot be hooked in phase A
//! (STD-6: their docs say "not hookable"; collected by address).

/// Every `#[native]` STD-6 added to existing ports of this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[
    crate::camctl::NATIVE_IS_SPHERE_VISIBLE_WITH_MATRIX,
    // SAMP-4: the on-foot PC camera's stick readers (phase B only until their L2 run).
    crate::padlook::NATIVE_PAD_LOOK_X,
    crate::padlook::NATIVE_PAD_LOOK_Y,
];
