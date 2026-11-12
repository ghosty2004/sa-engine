//! WRITER-NATIVE (batch `native-12`): the `CTaskSimpleCarWaitToSlowDown` constructor (0x646990) and the
//! destructor body 0x46F690 of the `CTaskSimpleCarDrive`-derived task with vtable 0x859E50, reached by the
//! road-rage / leave-car sequences on the native walk. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskSimple::CTaskSimple()` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => crate::mvpa_ai1::task_simple_ctor;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTaskSimpleCarDrive::~CTaskSimpleCarDrive()` body (0x63C460)
    fn car_drive_dtor(t: GPtr<u8>) = thiscall 0x63C460 => unported;
}

/// `CTaskSimpleCarWaitToSlowDown::CTaskSimpleCarWaitToSlowDown(CVehicle*, int32 slowType)` (0x646990, `ret 8`,
/// SEH frame): `CTaskSimple` base, vtable 0x86ED98, +8 = vehicle, +0x10 = slow type; `RegisterReference(&+8)` with
/// a vehicle. Returns `this`.
#[hook(addr = 0x646990, conv = thiscall, name = "CTaskSimpleCarWaitToSlowDown::Constructor",
       expect = "6A FF 68 B8 25 84 00",
       unverified = "native-12", route = "L2", hazards = "heap-writes", recs = "grove-walk,car-ai")]
pub fn car_wait_to_slow_down_ctor(this: GPtr<u8>, veh: GPtr<u8>, slow_type: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0, 0x86_ED98);
    wr::<GPtr<u8>>(this, 8, veh);
    wr::<u32>(this, 0x10, slow_type);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(8));
    }
    this
}

/// The destructor body 0x46F690 of the task with vtable 0x859E50 (a `CTaskSimpleCarDrive` re-tabled by the
/// seek / road-rage code): sets its vtable and tail-jumps to `CTaskSimpleCarDrive`'s destructor body 0x63C460.
#[hook(addr = 0x46F690, conv = thiscall, name = "FUN_0046f690",
       expect = "C7 01 50 9E 85 00 E9",
       unverified = "native-12", route = "L2", hazards = "heap-writes", recs = "grove-walk,car-ai")]
pub fn car_drive_derived_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85_9E50);
    car_drive_dtor(this);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CAR_WAIT_TO_SLOW_DOWN_CTOR, HOOK_CAR_DRIVE_DERIVED_DTOR];
