//! FRONTIER-2 (batch `frontier2-3`): the crews of the emergency vehicles `GenerateOneEmergencyServicesCar` creates
//! (`CCarAI::AddAmbulanceOccupants` 0x41C4A0, `CCarAI::AddFiretruckOccupants` 0x41C600; the task constructors are the
//! generated ports of sa-ai gen_ctors.rs). Ported from the 1.0 US bytes
//! (bodies in `.HOODLUM`, 0x1563CE0 / 0x1568C10).

use sa_core::abi::HookReg;
use sa_core::bindings::task_new;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_world::bindings::{simple_car_drive_ctor, tm_set_task};

use crate::bindings::{set_up_driver, setup_passenger};
use crate::common::rd;

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// `CTaskComplexDriveFireTruck::CTaskComplexDriveFireTruck(CVehicle*, CPed* partner, bool isDriver)` (0x659310)
    fn drive_fire_truck_ctor(t: GPtr<u8>, veh: GPtr<u8>, partner: GPtr<u8>, driver: u32) -> GPtr<u8> = thiscall 0x659310 => unported;
    /// `CTaskComplexMedicTreatInjuredPed::CTaskComplexMedicTreatInjuredPed(CVehicle*, CPed* partner, bool isDriver)` (0x658BA0)
    fn medic_treat_injured_ped_ctor(t: GPtr<u8>, veh: GPtr<u8>, partner: GPtr<u8>, driver: u32) -> GPtr<u8> = thiscall 0x658BA0 => unported;
}

/// A service task constructor `(memory, vehicle, partner, isDriver)`.
type TaskCtor = fn(GPtr<u8>, GPtr<u8>, GPtr<u8>, u32) -> GPtr<u8>;

/// The crew of an emergency vehicle: a driver (`SetUpDriver(-1, false, false)`) and a passenger in `seat`
/// (`SetupPassenger(seat, -1, false, false)`); each gets a `CTaskSimpleCarDrive(veh, null, false)` (0x60 bytes) as task 4
/// and the service task (`size` bytes, `ctor(veh, partner, isDriver)`) as task 3, driver first. The last `SetTask`'s
/// bool comes from the no-CD decoder at 0xCB73E0 (both bodies); taken as the 0 of the other three calls.
fn add_crew(veh: GPtr<u8>, seat: i32, size: u32, ctor: TaskCtor) {
    let driver = set_up_driver(veh, -1, 0, 0);
    let passenger = setup_passenger(veh, seat, -1, 0, 0);
    let drive = |ped: GPtr<u8>| {
        let t = task_new(0x60);
        let t = if t.is_null() { GPtr::NULL } else { simple_car_drive_ctor(t, veh, GPtr::NULL, 0) };
        tm_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4, 0);
    };
    let service = |ped: GPtr<u8>, partner: GPtr<u8>, is_driver: u32| {
        let t = task_new(size);
        let t = if t.is_null() { GPtr::NULL } else { ctor(t, veh, partner, is_driver) };
        tm_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 3, 0);
    };
    drive(driver);
    service(driver, passenger, 1);
    drive(passenger);
    service(passenger, driver, 0);
}

/// `CCarAI::AddAmbulanceOccupants(CVehicle*)` (stub 0x41C4A0 → 0x1563CE0): driver and front passenger (seat 1) with
/// `CTaskComplexMedicTreatInjuredPed` (0x2C bytes).
#[hook(addr = 0x41C4A0, conv = cdecl, name = "CCarAI::AddAmbulanceOccupants", expect = "E9 3B 78 14 01",
       unverified = "frontier2-3", route = "L2", hazards = "heap-writes",
       note = "the last SetTask's bool comes from no-CD code (0xCB73E0), taken as 0")]
pub fn add_ambulance_occupants(veh: GPtr<u8>) {
    add_crew(veh, 1, 0x2C, medic_treat_injured_ped_ctor);
}

/// `CCarAI::AddFiretruckOccupants(CVehicle*)` (stub 0x41C600 → 0x1568C10): driver and passenger (seat 0) with
/// `CTaskComplexDriveFireTruck` (0x1C bytes).
#[hook(addr = 0x41C600, conv = cdecl, name = "CCarAI::AddFiretruckOccupants", expect = "E9 0B C6 14 01",
       unverified = "frontier2-3", route = "L2", hazards = "heap-writes",
       note = "the last SetTask's bool comes from no-CD code (0xCB73E0), taken as 0")]
pub fn add_firetruck_occupants(veh: GPtr<u8>) {
    add_crew(veh, 0, 0x1C, drive_fire_truck_ctor);
}

pub const HOOKS: &[HookReg] = &[HOOK_ADD_AMBULANCE_OCCUPANTS, HOOK_ADD_FIRETRUCK_OCCUPANTS];
