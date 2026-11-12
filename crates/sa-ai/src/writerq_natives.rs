//! WRITER-Q: phase B registrations (`#[native]`) of this crate's ports that are not hooked in phase A.

/// Every `#[native]` WRITER-Q added to this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[crate::chat_task_ctor::NATIVE_TASK_COMPLEX_CHAT_CTOR];
