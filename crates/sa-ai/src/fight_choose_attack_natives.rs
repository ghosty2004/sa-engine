//! WRITER-NATIVE: phase B/C registrations (`#[native]`) of PORT-AI-5's `CTaskSimpleFight` attack choosers
//! (`fighthelp.rs`), which are ported but not hooked in phase A: their split runs differ only in the stale upper
//! half of EAX (callers read AX only). Reached by every punch on the native build.

/// `FUN_00624710(CPed*)` (0x624710, `CTaskSimpleFight::ChooseAttackPlayer`, `ret 4`, AX).
#[sa_core::native(addr = 0x624710, conv = thiscall, name = "FUN_00624710")]
pub fn fight_choose_attack_player(this: sa_core::mem::GPtr<u8>, ped: sa_core::mem::GPtr<u8>) -> u32 {
    crate::fighthelp::fight_choose_attack_player(this, ped)
}

/// `FUN_00624a40(CPed*)` (0x624A40, `CTaskSimpleFight::ChooseAttackAI`, `ret 4`, AX).
#[sa_core::native(addr = 0x624A40, conv = thiscall, name = "FUN_00624a40")]
pub fn fight_choose_attack_ai(this: sa_core::mem::GPtr<u8>, ped: sa_core::mem::GPtr<u8>) -> u32 {
    crate::fighthelp::fight_choose_attack_ai(this, ped)
}

/// This module's phase-B registrations.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_FIGHT_CHOOSE_ATTACK_PLAYER, NATIVE_FIGHT_CHOOSE_ATTACK_AI];
