//! Phase B registrations (`#[native]`) of existing ports whose entries cannot be hooked in phase A
//! (STD-6: their docs say "not hookable"; collected by address).

/// Every `#[native]` STD-6 added to existing ports of this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[
    crate::building::NATIVE_PROCESS_CONTROL,
    crate::building::NATIVE_PROCESS_COLLISION,
    crate::clumpdata::NATIVE_LOAD_FRAMES_INTO_SPR,
    crate::clumpdata::NATIVE_FOR_ALL_FRAMES_IN_SPR,
    crate::group::NATIVE_GET_NUM_ANIMATIONS,
    crate::group::NATIVE_GET_ANIM_BLOCK,
    crate::ik::NATIVE_GET_PRIORITY,
    crate::sequence::NATIVE_PRINT,
];
