//! STD-7: phase B registrations (`#[native]`) of this crate's ports that are not hooked in phase A.

/// Every `#[native]` STD-7 added to this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[
    crate::collscan::NATIVE_SCAN_FOR_COLLISION_EVENTS,
    // 0x680600 CTaskSimpleInAir::ProcessPed is not listed: task_in_air_process_ped hooks it.
];
