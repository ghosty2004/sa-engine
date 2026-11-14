//! WRITER-NATIVE (batch `native-9`): the `CTaskComplexRoadRage` constructor (0x6220A0) and the event
//! destructor body 0x6A0650 reached after car collisions on the native walk (the `CEventVehicleOnFire`
//! destructor 0x4B1170 is WRITER-ATTR's, attr-6). Ported from the
//! 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

use sa_core::bindings::entity_register_reference;
use crate::bindings::task_complex_ctor;
use crate::bindings::event_vehicle_damage_dtor;

/// `CTaskComplexRoadRage::CTaskComplexRoadRage(CPed* target)` (0x6220A0, `ret 4`; the entry `push -1; jmp
/// 0x406017` pushes the SEH handler from a protection chunk): `CTaskComplex` base, vtable 0x86D9F0, +0xC = target
/// with `RegisterReference(&+0xC)` when set. Returns `this`.
#[hook(addr = 0x6220A0, conv = thiscall, name = "CTaskComplexRoadRage::Constructor",
       expect = "6A FF E9 70 3F DE FF",
       unverified = "native-9", route = "L2", hazards = "heap-writes", recs = "grove-walk,car-ai",
       note = "entry jmp to the protection chunk 0x406017 (SEH handler push)")]
pub fn road_rage_ctor(this: GPtr<u8>, target: GPtr<u8>) -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    wr::<u32>(this, 0, 0x86_D9F0);
    wr::<GPtr<u8>>(this, 0xC, target);
    if !target.is_null() {
        entity_register_reference(target, this.byte_add(0xC));
    }
    this
}

/// A `CEventVehicleDamage`-derived destructor body (0x6A0650, vtable 0x8710D0): sets its vtable and tail-jumps
/// to the `CEventVehicleDamage` destructor body 0x4B1990.
#[hook(addr = 0x6A0650, conv = thiscall, name = "FUN_006a0650",
       expect = "C7 01 D0 10 87 00 E9",
       unverified = "native-9", route = "L2", hazards = "heap-writes", recs = "grove-walk,car-ai")]
pub fn event_vehicle_damage_derived_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x87_10D0);
    event_vehicle_damage_dtor(this);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_ROAD_RAGE_CTOR, HOOK_EVENT_VEHICLE_DAMAGE_DERIVED_DTOR];
