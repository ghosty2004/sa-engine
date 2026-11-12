//! The `CTaskComplexLeaveCar` constructor (0x63B8C0), taken from PORT-AI-8's generated constructor (batch
//! `ai7-1`, `tools/gen/ai_patterns.py`, branch worktree-agent-a70c1c8af9cc7a9c7) without the rest of its
//! regenerated `gen_ctors.rs`. Reached by the road-rage task on the native walk.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskComplex::CTaskComplex()` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// `CTaskComplexLeaveCar::CTaskComplexLeaveCar(CVehicle*, int32 door, int32 delay, bool sensible, bool
/// forceGetOut)` (0x63B8C0, `ret 0x14`, SEH frame): `CTaskComplex` base, +0x10 = door, +0x14 = delay, +0x18 =
/// sensible, vtable 0x86E828, +0xC = vehicle, +0x19 = forceGetOut, +0x1A / +0x1C / +0x20 / +0x21 / +0x30 = 0,
/// +0x24 = 15, +0x28 = 4.0, +0x2C = 1.0; `RegisterReference(&+0xC)` with a vehicle. Returns `this`.
#[hook(addr = 0x63B8C0, conv = thiscall, name = "CTaskComplexLeaveCar", expect = "6A FF 68 18 16 84 00",
       unverified = "ai7-1", route = "L2", hazards = "heap-writes")]
pub fn leave_car_ctor(this: GPtr<u8>, veh: GPtr<u8>, door: u32, delay: u32, sensible: u32, force: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0x10, door);
    wr::<u32>(this, 0x14, delay);
    wr::<u8>(this, 0x18, sensible as u8);
    wr::<u32>(this, 0, 0x86_E828);
    wr::<GPtr<u8>>(this, 0xC, veh);
    wr::<u8>(this, 0x19, force as u8);
    wr::<u8>(this, 0x1A, 0);
    wr::<u32>(this, 0x1C, 0);
    wr::<u8>(this, 0x20, 0);
    wr::<u8>(this, 0x21, 0);
    wr::<u32>(this, 0x24, 0xF);
    wr::<u32>(this, 0x28, 0x4080_0000);
    wr::<u32>(this, 0x2C, 0x3F80_0000);
    wr::<u8>(this, 0x30, 0);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(0xC));
    }
    this
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_LEAVE_CAR_CTOR];
