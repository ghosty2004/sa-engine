//! `CTaskComplexBeInGroup::Clone` (0x636BE0), taken from PORT-AI-8's generated clone (batch `ai8g-3`,
//! `tools/gen/ai_patterns.py`, branch worktree-agent-a70c1c8af9cc7a9c7) without the rest of its regenerated
//! `gen_clones.rs`. Reached on the native walk (STD-13 19.8).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new_c(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => crate::mvpa_ai1::task_operator_new;
    /// `CTaskComplex::CTaskComplex()` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
}

/// `CTaskComplexBeInGroup::Clone` (0x636BE0, SEH frame): `operator new(0x28)`, the inlined constructor
/// (`CTaskComplex` base, +0x10 = this +0x10 byte, +0xC = this +0xC, +0x14 / +0x18 / +0x20 = 0, vtable
/// 0x86E22C, +0x1C = 200, +0x24 = -1). Null when the pool is full.
#[hook(addr = 0x636BE0, conv = thiscall, name = "CTaskComplexBeInGroup::Clone", expect = "6A FF 68 7B 0E 84 00",
       unverified = "ai8g-3", route = "L2", hazards = "heap-writes")]
pub fn be_in_group_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new_c(0x28);
    if t.is_null() {
        return t;
    }
    let b = rd::<u8>(this, 0x10);
    let group = rd::<u32>(this, 0xC);
    task_complex_ctor(t);
    wr::<u8>(t, 0x10, b);
    wr::<u32>(t, 0xC, group);
    wr::<u32>(t, 0x14, 0);
    wr::<u32>(t, 0x18, 0);
    wr::<u32>(t, 0x20, 0);
    wr::<u32>(t, 0, 0x86_E22C);
    wr::<u32>(t, 0x1C, 0xC8);
    wr::<u32>(t, 0x24, 0xFFFF_FFFF);
    t
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_BE_IN_GROUP_CLONE];
