//! Constructors and destructors of the car get-in/get-out and drive tasks that SA:MP executes but the catalogue
//! left unnamed (UNK-game rows of `docs/samp/native-unported.csv`, batch `samp-unk-1`). Names proposed from the
//! vtables they store and their callers (`docs/samp/native-unported-names.csv`). Ported from the 1.0 US bytes; the
//! SEH frames have no other effect. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTaskSimple::CTaskSimple()` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CTaskSimple::~CTaskSimple()` (0x61A3A0)
    fn task_simple_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `CTaskSimpleCarDrive::~CTaskSimpleCarDrive()` (0x63C460)
    fn task_simple_car_drive_dtor(t: GPtr<u8>) = thiscall 0x63C460 => unported;
    /// `CTaskComplexEnterCar::~CTaskComplexEnterCar()` (0x63DFA0)
    fn task_complex_enter_car_dtor(t: GPtr<u8>) = thiscall 0x63DFA0 => unported;
}

/// `FUN_0046f690` (0x46F690), proposed `CTaskSimpleCarDriveTimed::~CTaskSimpleCarDriveTimed` (thiscall): vtable
/// 0x859E50, then tail-jumps to `CTaskSimpleCarDrive`'s destructor 0x63C460. Its caller is the deleting
/// destructor 0x46F670.
#[hook(addr = 0x46F690, conv = thiscall, name = "FUN_0046f690", expect = "C7 01 50 9E 85 00",
       unverified = "samp-unk-1", route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn task_simple_car_drive_timed_dtor_46f690(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85_9E50);
    task_simple_car_drive_dtor(this);
}

/// `FUN_00640330` (0x640330), proposed `CTaskComplexEnterCarAsDriver::~CTaskComplexEnterCarAsDriver`
/// (thiscall): vtable 0x86EAAC, then tail-jumps to `CTaskComplexEnterCar`'s destructor 0x63DFA0.
#[hook(addr = 0x640330, conv = thiscall, name = "FUN_00640330", expect = "C7 01 AC EA 86 00",
       unverified = "samp-unk-1", route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn task_complex_enter_car_as_driver_dtor_640330(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86_EAAC);
    task_complex_enter_car_dtor(this);
}

/// `FUN_00647710` (0x647710), proposed `CTaskSimpleCarForcePedOut::CTaskSimpleCarForcePedOut(CVehicle*, int32
/// door)` (thiscall, `ret 8`): `CTaskSimple`'s constructor, vtable 0x86EE94, the vehicle at +8 and the door at +0xC,
/// then `RegisterReference(&this->vehicle)` when the vehicle is set. Returns `this`. Callers:
/// `CTaskComplexLeaveCar::CreateSubTask` and its `Clone`.
#[hook(addr = 0x647710, conv = thiscall, name = "FUN_00647710", expect = "6A FF 68 88 27 84 00",
       unverified = "samp-unk-1", route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn task_simple_car_force_ped_out_ctor_647710(this: GPtr<u8>, veh: GPtr<u8>, door: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0, 0x86_EE94);
    wr(this, 8, veh);
    wr::<u32>(this, 0xC, door);
    if !veh.is_null() {
        register_reference(veh, this.byte_add(8));
    }
    this
}

/// The shared body of the two car-task destructors below: vtable `vt`, release the vehicle at +8, then
/// `CTaskSimple`'s destructor.
fn car_task_dtor(this: GPtr<u8>, vt: u32) {
    wr::<u32>(this, 0, vt);
    let veh = rd::<GPtr<u8>>(this, 8);
    if !veh.is_null() {
        clean_up_old_reference(veh, this.byte_add(8));
    }
    task_simple_dtor(this);
}

/// `FUN_00647790` (0x647790), proposed `CTaskSimpleCarForcePedOut::~CTaskSimpleCarForcePedOut` (thiscall):
/// vtable 0x86EE94, releases the vehicle at +8, then `CTaskSimple`'s destructor 0x61A3A0.
#[hook(addr = 0x647790, conv = thiscall, name = "FUN_00647790", expect = "6A FF 68 A8 27 84 00",
       unverified = "samp-unk-1", route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn task_simple_car_force_ped_out_dtor_647790(this: GPtr<u8>) {
    car_task_dtor(this, 0x86_EE94);
}

/// `CTaskSimpleCarSetPedOut::CTaskSimpleCarSetPedOut(CVehicle*, int32 door, bool switchOffEngine)` (0x6478B0,
/// thiscall, `ret 0xC`): `CTaskSimple`'s constructor, the door at +0xC, vtable 0x86EEB8, the vehicle at +8, the
/// engine flag at +0x10 and bytes +0x11..+0x15 cleared, then `RegisterReference(&this->vehicle)` when the vehicle is
/// set. Returns `this`. The entry `push -1; jmp 0x401D02` is a no-CD chunk that pushes the SEH handler 0x8427C8 and
/// returns to 0x6478B7 (decoded from the exe image).
#[hook(addr = 0x6478B0, conv = thiscall, name = "CTaskSimpleCarSetPedOut", expect = "6A FF E9 4B A4 DB FF",
       unverified = "samp-unk-1", route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn task_simple_car_set_ped_out_ctor(this: GPtr<u8>, veh: GPtr<u8>, door: u32, switch_off_engine: u8) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0xC, door);
    wr::<u32>(this, 0, 0x86_EEB8);
    wr(this, 8, veh);
    wr::<u8>(this, 0x10, switch_off_engine);
    for o in 0x11..=0x15 {
        wr::<u8>(this, o, 0);
    }
    if !veh.is_null() {
        register_reference(veh, this.byte_add(8));
    }
    this
}

/// `FUN_00647950` (0x647950), proposed `CTaskSimpleCarSetPedOut::~CTaskSimpleCarSetPedOut` (thiscall): vtable
/// 0x86EEB8, releases the vehicle at +8, then `CTaskSimple`'s destructor 0x61A3A0. 25 callers.
#[hook(addr = 0x647950, conv = thiscall, name = "FUN_00647950", expect = "6A FF 68 E8 27 84 00",
       unverified = "samp-unk-1", route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn task_simple_car_set_ped_out_dtor_647950(this: GPtr<u8>) {
    car_task_dtor(this, 0x86_EEB8);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_TASK_SIMPLE_CAR_DRIVE_TIMED_DTOR_46F690,
    HOOK_TASK_COMPLEX_ENTER_CAR_AS_DRIVER_DTOR_640330,
    HOOK_TASK_SIMPLE_CAR_FORCE_PED_OUT_CTOR_647710,
    HOOK_TASK_SIMPLE_CAR_FORCE_PED_OUT_DTOR_647790,
    HOOK_TASK_SIMPLE_CAR_SET_PED_OUT_CTOR,
    HOOK_TASK_SIMPLE_CAR_SET_PED_OUT_DTOR_647950,
];
