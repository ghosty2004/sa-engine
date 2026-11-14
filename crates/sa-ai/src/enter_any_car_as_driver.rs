//! `CTaskComplexEnterAnyCarAsDriver::CreateFirstSubTask` (PEDHIT: a ped near the BMX hit looks for a car). Ported
//! from the 1.0 US bytes; gta-reversed `TaskComplexEnterAnyCarAsDriver.cpp` as a reading aid.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CVehicleScanner::GetClosestVehicleInRange()` (0x5FFEE0: `[this+0x4C]`)
    fn scanner_closest_vehicle(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x5FFEE0 => unported;
    /// `CCarEnterExit::IsVehicleStealable(const CVehicle*, const CPed*)` (0x6510D0, cdecl)
    fn is_vehicle_stealable(v: GPtr<u8>, ped: GPtr<u8>) -> u8 = cdecl 0x6510D0 => unported;
    /// `CTaskComplexEnterCar::CTaskComplexEnterCar(CVehicle*, bool asDriver, bool, bool, bool)` (0x63A220)
    fn enter_car_ctor(t: GPtr<u8>, v: GPtr<u8>, driver: u32, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x63A220 => unported;
    /// `CTaskComplex::CTaskComplex()` (0x61A390)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
}

/// `CTaskComplexEnterCarAsDriver`'s vtable and the 8-byte fallback task's vtable.
const ENTER_CAR_AS_DRIVER_VT: u32 = 0x86_EAAC;
const FALLBACK_VT: u32 = 0x85_9CFC;

/// The position an entity's placement gives (`m_matrix ? &m_matrix->pos : &m_placement.pos`).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// A `CTaskComplexEnterCarAsDriver` for `v`: `CTaskComplexEnterCar(v, true, false, false, false)`, then its vtable.
fn enter_as_driver(v: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x50);
    if t.is_null() {
        return t;
    }
    enter_car_ctor(t, v, 1, 0, 0, 0);
    wr::<u32>(t, 0, ENTER_CAR_AS_DRIVER_VT);
    t
}

/// `CTaskComplexEnterAnyCarAsDriver::CreateFirstSubTask(CPed*)` (0x643510, thiscall, `ret 4`): the vehicle scanner's
/// (intelligence +0xD4) closest vehicle when stealable; else the nearest stealable vehicle of the scanner's 16
/// entities (+0xE0; squared distance `(dz² + dy²) + dx²` of `ped - vehicle`, strictly less than the best so far,
/// which starts at FLT_MAX); else an 8-byte task with vtable 0x859CFC. Each as a new task (null when the
/// allocation fails).
#[hook(addr = 0x643510, conv = thiscall, name = "CTaskComplexEnterAnyCarAsDriver::CreateFirstSubTask",
       expect = "64 A1 00 00 00 00 6A FF 68 31 22 84 00",
       unverified = "pedhit-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "the SEH frame is not reproduced")]
pub fn enter_any_car_as_driver_create_first_sub_task(_this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let v = scanner_closest_vehicle(intel.byte_add(0xD4));
    if !v.is_null() && is_vehicle_stealable(v, ped) != 0 {
        return enter_as_driver(v);
    }
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let mut best = f32::MAX;
    let mut closest = GPtr::<u8>::NULL;
    for i in 0..16 {
        let v = rd::<GPtr<u8>>(intel, 0xE0 + 4 * i);
        if v.is_null() {
            continue;
        }
        let (vp, pp) = (pos(v), pos(ped));
        let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(vp, 0));
        let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(vp, 4));
        let dz = x(rd::<f32>(pp, 8)).fsub_m(rd::<f32>(vp, 8));
        let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        let stored = d.to_f32();
        if d.fcom(x(best)).test_parity(5) {
            continue;
        }
        if is_vehicle_stealable(v, ped) != 0 {
            best = stored;
            closest = v;
        }
    }
    if !closest.is_null() {
        return enter_as_driver(closest);
    }
    let t = task_new(8);
    if t.is_null() {
        return t;
    }
    task_complex_ctor(t);
    wr::<u32>(t, 0, FALLBACK_VT);
    t
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_ENTER_ANY_CAR_AS_DRIVER_CREATE_FIRST_SUB_TASK];
