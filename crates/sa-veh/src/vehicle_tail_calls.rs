//! PORT-VEH-4 (batch `veh4-3`): the vehicle functions whose 1.0 US body only tail-jumps into the
//! automobile's: the monster truck and trailer destructors (vtable reset, then `~CAutomobile`) and the
//! quad bike's suspension virtuals. Ported from the bytes.

use sa_core::abi::{HookReg, NativeReg};
use sa_core::mem::GPtr;
use sa_core::{hook, native, orig};

use crate::common::wr;

orig! {
    /// `CAutomobile::~CAutomobile()` (0x6A61E0)
    fn automobile_dtor(v: GPtr<u8>) = thiscall 0x6A61E0 => unported;
    /// `CAutomobile::ResetSuspension()` (0x6A2AE0)
    fn automobile_reset_suspension(v: GPtr<u8>) = thiscall 0x6A2AE0 => unported;
    /// `CAutomobile::ProcessSuspension()` (0x6AFB10)
    fn automobile_process_suspension(v: GPtr<u8>) = thiscall 0x6AFB10 => unported;
}

/// `CMonsterTruck::~CMonsterTruck()` (0x6C7D10): vtable 0x8717D8, then `CAutomobile::~CAutomobile`.
#[hook(addr = 0x6C7D10, conv = thiscall, name = "CMonsterTruck::~CMonsterTruck", this = "CMonsterTruck",
       expect = "C7 01 D8 17 87 00",
       unverified = "veh4-3", route = "L2", hazards = "heap-writes",
       note = "the original tail-jumps (jmp 0x6A61E0); destroys the vehicle's members")]
pub fn monster_truck_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x0087_17D8);
    automobile_dtor(this);
}

/// `CTrailer::~CTrailer()` (0x6CED10): vtable 0x871C28, then `CAutomobile::~CAutomobile`.
#[hook(addr = 0x6CED10, conv = thiscall, name = "CTrailer::~CTrailer", this = "CTrailer",
       expect = "C7 01 28 1C 87 00",
       unverified = "veh4-3", route = "L2", hazards = "heap-writes",
       note = "the original tail-jumps (jmp 0x6A61E0); destroys the vehicle's members")]
pub fn trailer_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x0087_1C28);
    automobile_dtor(this);
}

/// `CQuadBike::ResetSuspension()` (0x6CDCB0): `jmp CAutomobile::ResetSuspension`. A phase B native: a
/// plain 5-byte `jmp` outside `.HOODLUM` cannot be hooked.
#[native(addr = 0x6CDCB0, conv = thiscall, name = "CQuadBike::ResetSuspension")]
pub fn quad_bike_reset_suspension(this: GPtr<u8>) {
    automobile_reset_suspension(this);
}

/// `CQuadBike::ProcessSuspension()` (0x6CE270): `jmp CAutomobile::ProcessSuspension`. A phase B native
/// (see `quad_bike_reset_suspension`).
#[native(addr = 0x6CE270, conv = thiscall, name = "CQuadBike::ProcessSuspension")]
pub fn quad_bike_process_suspension(this: GPtr<u8>) {
    automobile_process_suspension(this);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_MONSTER_TRUCK_DTOR, HOOK_TRAILER_DTOR];

/// This module's natives.
pub const NATIVES: &[NativeReg] = &[NATIVE_QUAD_BIKE_RESET_SUSPENSION, NATIVE_QUAD_BIKE_PROCESS_SUSPENSION];
