//! `CTaskComplexGoToCarDoorAndStandStill::ControlSubTask` (0x64A820, batch `std14-ai-2`). Ported from the 1.0 US
//! bytes (gta-reversed as a reading aid). Unverified (docs/verify/HOWTO.md).
//!
//! Layout used: +8 sub-task, +0xC vehicle, +0x14 as driver, +0x18 move state, +0x34 target door, +0x38 target
//! position, +0x44 the target door fixed (no new nearest-door search).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskComplexGoToCarDoorAndStandStill::CreateSubTask(int, CPed*)` (0x64A5F0, `ret 8`)
    fn create_sub_task(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x64A5F0 => unported;
    /// `CTaskComplexGoToCarDoorAndStandStill::IsVehicleInRange(const CPed&)` (0x6458A0)
    fn is_vehicle_in_range(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6458A0 => unported;
    /// `CTaskSimpleGoToPoint::UpdatePoint(const CVector&, int moveState, float radius)` (0x645700)
    fn go_to_point_update_point(t: GPtr<u8>, pos: GPtr<u8>, ms: u32, r: u32) = thiscall 0x645700 => unported;
    /// `CTaskComplex::CreateFirstSubTask` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    fn get_position_to_open_car_door(out: GPtr<u8>, v: GPtr<u8>, door: u32) -> GPtr<u8> = cdecl 0x64E740 => unported;
    fn get_nearest_car_door(ped: GPtr<u8>, v: GPtr<u8>, out: GPtr<u8>, door: GPtr<u8>) -> u8 = cdecl 0x6528F0 => unported;
    /// `CCarEnterExit::GetNearestCarPassengerDoor(ped, veh, out, door, occupiedTandem, enterable, roomToGetIn)` (0x650BB0)
    fn get_nearest_car_passenger_door(ped: GPtr<u8>, v: GPtr<u8>, out: GPtr<u8>, door: GPtr<u8>, a: u32, b: u32, c: u32) -> u8 = cdecl 0x650BB0 => unported;
    fn path_to_door_blocked(ped: GPtr<u8>, v: GPtr<u8>, pos: GPtr<u8>) -> u8 = cdecl 0x651210 => unported;
    /// `operator-(const CVector&, const CVector&)` into `out` (0x40FE60)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::Magnitude()` (0x406DA0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
}

fn vp(v: &mut [u32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

fn set_target(this: GPtr<u8>, p: [u32; 3]) {
    for (k, c) in p.iter().enumerate() {
        wr::<u32>(this, 0x38 + 4 * k as isize, *c);
    }
}

fn door_pos(this: GPtr<u8>) -> [u32; 3] {
    let mut p = [0u32; 3];
    get_position_to_open_car_door(vp(&mut p), rd::<GPtr<u8>>(this, 0xC), rd::<u32>(this, 0x34));
    p
}

/// `CTaskComplexGoToCarDoorAndStandStill::ControlSubTask(CPed*)` (0x64A820, `ret 4`). Null without a vehicle. A
/// non-cop ped facing a player driver in ped state 0x3F stands still (0xCB). By the sub-task: 0x33C / 0xCA / 0xCB /
/// 0x32A keep it (the target refreshed from the target door); 0x389 (route): out of range → stand still, a free
/// path → go to the point (0x384), otherwise the first sub-task again once the door moved more than 0.01 from the
/// target; 0x384 (go to point): out of range → stand still; a fixed door (+0x44) or a new nearest door (driver:
/// `GetNearestCarDoor`; passenger: `GetNearestCarPassengerDoor(1, 1, 1)`) updates the point (move state +0x18,
/// radius 0); none: drivers stand still, passengers try `GetNearestCarPassengerDoor(1, 0, 1)` and wait for the door
/// (0x33C) or stand still; these 0x384 results set ped +0x474 bit 31. Other sub-tasks give null.
#[hook(addr = 0x64A820, conv = thiscall, name = "CTaskComplexGoToCarDoorAndStandStill::ControlSubTask", expect = "83 EC 48 56 8B F1 8B 46 0C",
       unverified = "std14-ai-2", route = "L2", hazards = "uninit-locals", recs = "enter-car",
       note = "the failed nearest-door searches leave the door position in the output local, as the original's stack slot holds it")]
pub fn control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let v = rd::<GPtr<u8>>(this, 0xC);
    if v.is_null() {
        return GPtr::NULL;
    }
    if rd::<u32>(ped, 0x598) != 6 {
        let d = rd::<GPtr<u8>>(v, 0x460);
        if !d.is_null()
            && ped_is_player(d) != 0
            && rd::<u32>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x460), 0x530) == 0x3F
        {
            return create_sub_task(this, 0xCB, ped);
        }
    }
    match v_task_type(rd::<GPtr<u8>>(this, 8)) {
        0x33C | 0xCA | 0xCB | 0x32A => {
            if rd::<u32>(this, 0x34) != 0 {
                let p = door_pos(this);
                set_target(this, p);
            }
            rd::<GPtr<u8>>(this, 8)
        }
        0x389 => {
            let mut p = door_pos(this);
            if is_vehicle_in_range(this, ped) == 0 {
                set_target(this, p);
                return create_sub_task(this, 0xCB, ped);
            }
            if path_to_door_blocked(ped, rd::<GPtr<u8>>(this, 0xC), vp(&mut p)) == 0 {
                set_target(this, p);
                return create_sub_task(this, 0x384, ped);
            }
            let mut d = [0u32; 3];
            vector_sub(vp(&mut d), this.byte_add(0x38), vp(&mut p));
            let len = X87::from_f64_exact(vector_magnitude(vp(&mut d)));
            if len.fcom(x(g::<f32>(0x86EC40).read())).test_any(0x41) {
                return rd::<GPtr<u8>>(this, 8);
            }
            set_target(this, p);
            v_create_first_sub_task(this, ped)
        }
        0x384 => {
            let p = door_pos(this);
            let r = go_to_point_control(this, ped, p);
            wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x8000_0000);
            r
        }
        _ => GPtr::NULL,
    }
}

/// The go-to-point branch (0x64AA04..0x64ABCA) before the ped flag.
fn go_to_point_control(this: GPtr<u8>, ped: GPtr<u8>, p: [u32; 3]) -> GPtr<u8> {
    if is_vehicle_in_range(this, ped) == 0 {
        set_target(this, p);
        return create_sub_task(this, 0xCB, ped);
    }
    let v = rd::<GPtr<u8>>(this, 0xC);
    if rd::<u32>(this, 0x44) != 0 {
        let p2 = door_pos(this);
        set_target(this, p2);
        let st = rd::<GPtr<u8>>(this, 8);
        go_to_point_update_point(st, this.byte_add(0x38), rd::<u32>(this, 0x18), 0);
        return rd::<GPtr<u8>>(this, 8);
    }
    let mut out = p;
    let mut door = 0u32;
    let dp = GPtr::from_raw(&raw mut door).cast::<u8>();
    let driver = rd::<u8>(this, 0x14) != 0;
    let ok = if driver {
        get_nearest_car_door(ped, v, vp(&mut out), dp)
    } else {
        get_nearest_car_passenger_door(ped, v, vp(&mut out), dp, 1, 1, 1)
    };
    if ok != 0 {
        let st = rd::<GPtr<u8>>(this, 8);
        set_target(this, out);
        wr::<u32>(this, 0x34, door);
        go_to_point_update_point(st, this.byte_add(0x38), rd::<u32>(this, 0x18), 0);
        return st;
    }
    if driver {
        set_target(this, out);
        return create_sub_task(this, 0xCB, ped);
    }
    if get_nearest_car_passenger_door(ped, rd::<GPtr<u8>>(this, 0xC), vp(&mut out), dp, 1, 0, 1) != 0 {
        set_target(this, out);
        wr::<u32>(this, 0x34, door);
        return create_sub_task(this, 0x33C, ped);
    }
    set_target(this, out);
    create_sub_task(this, 0xCB, ped)
}

pub const HOOKS: &[HookReg] = &[HOOK_CONTROL_SUB_TASK];
