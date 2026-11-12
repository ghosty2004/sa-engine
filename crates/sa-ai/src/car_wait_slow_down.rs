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
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new_c(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => crate::mvpa_ai1::task_operator_new;
    /// `CTaskSimple::~CTaskSimple()` (0x61A3A0)
    fn task_simple_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEvent::GetEventType()` (vtable slot 4)
    fn event_get_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CPed::SetPedPositionInCar()` (0x5DF910)
    fn ped_set_ped_position_in_car(p: GPtr<u8>) = thiscall 0x5DF910 => unported;
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

/// `CTaskSimpleCarWaitToSlowDown::Clone()` (0x649CB0, SEH frame): `operator new(0x14)`, then the constructor
/// with +8 (vehicle) and +0x10 (slow type). Taken from PORT-AI-8's generated clone (ai7-4).
#[hook(addr = 0x649CB0, conv = thiscall, name = "CTaskSimpleCarWaitToSlowDown::Clone", expect = "6A FF 68 FB 2C 84 00",
       unverified = "ai7-4", route = "L2", hazards = "none")]
pub fn car_wait_to_slow_down_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new_c(0x14);
    if t.is_null() {
        return t;
    }
    car_wait_to_slow_down_ctor(t, rd::<GPtr<u8>>(this, 8), rd::<u32>(this, 0x10))
}

/// `CTaskSimpleCarWaitToSlowDown::~CTaskSimpleCarWaitToSlowDown()` (0x646A00, SEH frame, no `delete`): vtable
/// 0x86ED98, `CleanUpOldReference(&+8)` with a vehicle, then the `CTaskSimple` destructor (PORT-AI-8, ai7-3).
#[hook(addr = 0x646A00, conv = thiscall, name = "CTaskSimpleCarWaitToSlowDown::Destructor", expect = "6A FF 68 D8 25 84 00",
       unverified = "ai7-3", route = "L2", hazards = "heap-writes")]
pub fn car_wait_to_slow_down_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_ED98);
    let v = rd::<GPtr<u8>>(this, 8);
    if !v.is_null() {
        entity_clean_up_old_reference(v, this.byte_add(8));
    }
    task_simple_dtor(this);
}

/// `CTaskSimpleCarWaitToSlowDown::GetTaskType()` (0x6469F0): 0x329 (PORT-AI-8, ai7-3).
#[hook(addr = 0x6469F0, conv = thiscall, name = "CTaskSimpleCarWaitToSlowDown::GetTaskType", expect = "B8 29 03 00 00",
       unverified = "ai7-3", route = "L2", hazards = "none")]
pub fn car_wait_to_slow_down_get_task_type(_this: GPtr<u8>) -> u32 {
    0x329
}

/// `CTaskSimpleCarWaitToSlowDown::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x646A60, `ret 0xC`, bool
/// in AL): immediate (2) aborts; urgent (1) aborts only for a damage event (type 9) with +0x40 and +0xC set;
/// otherwise +0x10 = 2 (finish) and false (PORT-AI-8, ai7-3).
#[hook(addr = 0x646A60, conv = thiscall, name = "CTaskSimpleCarWaitToSlowDown::MakeAbortable", expect = "8B 44 24 08 83 F8 02",
       unverified = "ai7-3", route = "L2", hazards = "ax-al-return,heap-writes")]
pub fn car_wait_to_slow_down_make_abortable(this: GPtr<u8>, _ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 {
    if prio == 2 {
        return 1;
    }
    if prio == 1 && !ev.is_null() && event_get_type(ev) == 9 && rd::<u8>(ev, 0x40) != 0 && rd::<u8>(ev, 0xC) != 0 {
        return 1;
    }
    wr::<u32>(this, 0x10, 2);
    0
}

/// `CTaskSimpleCarWaitToSlowDown::SetPedPosition(CPed*)` (0x646AB0, `ret 4`): `SetPedPositionInCar` with a
/// vehicle (+8); true (PORT-AI-8, ai7-3).
#[hook(addr = 0x646AB0, conv = thiscall, name = "CTaskSimpleCarWaitToSlowDown::SetPedPosition", expect = "8B 41 08 85 C0",
       unverified = "ai7-3", route = "L2", hazards = "ax-al-return")]
pub fn car_wait_to_slow_down_set_ped_position(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if !rd::<GPtr<u8>>(this, 8).is_null() {
        ped_set_ped_position_in_car(ped);
    }
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_CAR_WAIT_TO_SLOW_DOWN_CTOR,
    HOOK_CAR_DRIVE_DERIVED_DTOR,
    HOOK_CAR_WAIT_TO_SLOW_DOWN_CLONE,
    HOOK_CAR_WAIT_TO_SLOW_DOWN_DTOR,
    HOOK_CAR_WAIT_TO_SLOW_DOWN_GET_TASK_TYPE,
    HOOK_CAR_WAIT_TO_SLOW_DOWN_MAKE_ABORTABLE,
    HOOK_CAR_WAIT_TO_SLOW_DOWN_SET_PED_POSITION,
];
