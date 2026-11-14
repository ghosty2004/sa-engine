//! STD-7: phase B registrations (`#[native]`) of this crate's ports that are not hooked in phase A.

/// Every `#[native]` STD-7 added to this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[
    crate::collscan::NATIVE_SCAN_FOR_COLLISION_EVENTS,
    // CARJACK: generated `#[native]`s no list carried (bare `jmp` thunks and unhookable CreateSubTasks); a jack
    // reached 0x655E00 (`thunk_FUN_0061a6f0`) as a blocker.
    crate::gen_bodies::NATIVE_G_THUNK_FUN_0061A6F0_637320,
    crate::gen_bodies::NATIVE_G_THUNK_FUN_0061A6F0_637350,
    crate::gen_bodies::NATIVE_G_THUNK_FUN_0061A6F0_637380,
    crate::gen_bodies::NATIVE_G_THUNK_FUN_0061A6F0_6373B0,
    crate::gen_bodies::NATIVE_G_THUNK_FUN_0061A6F0_6373E0,
    crate::gen_bodies::NATIVE_G_THUNK_FUN_0061A6F0_637410,
    crate::gen_bodies::NATIVE_G_THUNK_FUN_0061A6F0_637440,
    crate::gen_bodies::NATIVE_G_THUNK_FUN_0061A6F0_637470,
    crate::gen_bodies::NATIVE_G_THUNK_FUN_0061A6F0_655DD0,
    crate::gen_bodies::NATIVE_G_THUNK_FUN_0061A6F0_655E00,
    crate::gen_bodies::NATIVE_G_THUNK_FUN_00668870_66DFA0,
    crate::gen_bodies::NATIVE_G_THUNK_FUN_00691630_6935B0,
    crate::gen_bodies::NATIVE_G_THUNK_FUN_00691630_6935E0,
    crate::gen_subtasks::NATIVE_G_TASK_COMPLEX_DESTROY_CAR_MELEE_CREATE_SUB_TASK_628A70,
    crate::gen_subtasks::NATIVE_G_TASK_COMPLEX_USE_PAIRED_ATTRACTOR_CREATE_SUB_TASK_638D30,
    crate::gen_subtasks::NATIVE_G_TASK_COMPLEX_TREAT_ACCIDENT_CREATE_SUB_TASK_659E90,
    // 0x680600 CTaskSimpleInAir::ProcessPed is not listed: task_in_air_process_ped hooks it.
];
