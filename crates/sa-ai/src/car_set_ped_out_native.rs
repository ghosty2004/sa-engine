//! WRITER-NATIVE (batch `native-7`): `CTaskSimpleCarSetPedOut`'s constructor (0x6478B0) and destructor
//! (0x647950), reached when a rider is knocked off a bike on the native walk, and a phase-B registration of
//! PORT-AI-3's `ProcessPed` (0x647D10) body.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskSimple::CTaskSimple()` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => crate::mvpa_ai1::task_simple_ctor;
    /// `CTaskSimple::~CTaskSimple()` (0x61A3A0)
    fn task_simple_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
}

/// `CTaskSimpleCarSetPedOut` vtable
const VTABLE: u32 = 0x86_EEB8;

/// `CTaskSimpleCarSetPedOut::CTaskSimpleCarSetPedOut(CVehicle*, int32 door, bool switchOffEngine)` (0x6478B0,
/// `ret 0xC`; the entry `push -1; jmp 0x401D02` pushes the SEH handler from a protection chunk): `CTaskSimple`
/// base, +0xC = door, vtable 0x86EEB8, +8 = vehicle, +0x10 = switchOffEngine, +0x11..+0x15 = 0;
/// `RegisterReference(&+8)` on the vehicle when set. Returns `this`.
#[hook(addr = 0x6478B0, conv = thiscall, name = "CTaskSimpleCarSetPedOut",
       expect = "6A FF E9 4B A4 DB FF",
       unverified = "native-7", route = "L2", hazards = "heap-writes", recs = "driving,grove-walk",
       note = "entry jmp to the protection chunk 0x401D02 (SEH handler push)")]
pub fn car_set_ped_out_ctor(this: GPtr<u8>, veh: GPtr<u8>, door: i32, switch_off_engine: u8) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<i32>(this, 0xC, door);
    wr::<u32>(this, 0, VTABLE);
    wr::<GPtr<u8>>(this, 8, veh);
    wr::<u8>(this, 0x10, switch_off_engine);
    for off in 0x11..=0x15 {
        wr::<u8>(this, off, 0);
    }
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(8));
    }
    this
}

/// `CTaskSimpleCarSetPedOut::~CTaskSimpleCarSetPedOut()` (0x647950, SEH frame, no `delete`): vtable 0x86EEB8,
/// `CleanUpOldReference(&+8)` when the vehicle is set, then the `CTaskSimple` destructor.
#[hook(addr = 0x647950, conv = thiscall, name = "FUN_00647950",
       expect = "6A FF 68 E8 27 84 00",
       unverified = "native-7", route = "L2", hazards = "heap-writes", recs = "driving,grove-walk")]
pub fn car_set_ped_out_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, VTABLE);
    let v = rd::<GPtr<u8>>(this, 8);
    if !v.is_null() {
        entity_clean_up_old_reference(v, this.byte_add(8));
    }
    task_simple_dtor(this);
}

/// `CTaskSimpleCarSetPedOut::ProcessPed(CPed*)` (0x647D10): PORT-AI-3's body (`car_exit.rs`), not hooked there
/// (its split L2 runs differ in `CCollision` statics, unresolved). Registered for phases B/C only.
#[sa_core::native(addr = 0x647D10, conv = thiscall, name = "CTaskSimpleCarSetPedOut::ProcessPed")]
pub fn car_set_ped_out_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    crate::car_exit::car_set_ped_out_process_ped(this, ped)
}

/// `CTaskSimpleCarSetPedOut::PositionPedOutOfCollision(CPed*, CVehicle*, int door)` (0x6479B0, cdecl): PORT-AI-3's
/// body (`car_exit.rs`), not hooked there for the same reason. Registered for phases B/C only (STD-14: reached by
/// parking cars' drivers and the enter-car task on the native walk).
#[sa_core::native(addr = 0x6479B0, conv = cdecl, name = "CTaskSimpleCarSetPedOut::PositionPedOutOfCollision")]
pub fn car_set_ped_out_position_ped_out_of_collision(ped: GPtr<u8>, veh: GPtr<u8>, door: u32) {
    crate::car_exit::car_set_ped_out_position_ped_out_of_collision_port(ped, veh, door)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CAR_SET_PED_OUT_CTOR, HOOK_CAR_SET_PED_OUT_DTOR];

/// This module's phase-B registrations.
pub const NATIVES: &[sa_core::abi::NativeReg] =
    &[NATIVE_CAR_SET_PED_OUT_PROCESS_PED, NATIVE_CAR_SET_PED_OUT_POSITION_PED_OUT_OF_COLLISION];
