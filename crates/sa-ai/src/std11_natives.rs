//! STD-11: phase B registrations (`#[native]`) of this crate's ports that are not hooked in phase A.

/// Every `#[native]` STD-11 added to this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[crate::task_in_air_process_ped::NATIVE_TASK_SIMPLE_IN_AIR_678E80];
