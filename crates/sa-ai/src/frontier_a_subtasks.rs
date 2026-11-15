//! FRONTIER-2 turn 2, lane A (batch `frontier2-a2`): the `CreateSubTask` switches of the cop-in-car, medic, swat rope
//! and evasive cower tasks, ported from the 1.0 US bytes. Every sub-task comes from the original task pool (`new` of
//! the original size) and its original constructor.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{police_pursuit_ctor, run_anim_ctor2, task_simple_ctor};
use crate::common::*;
use crate::frontier_a::wander_medic_ctor;
use sa_core::bindings::{general_get_radian_angle_between_points, general_random_int_in_range};
use sa_math::fpu::{le_or_nan, st0};
use sa_pop::bindings::{find_police_car_mission, find_police_car_speed};
use sa_world::bindings::{
    achieve_heading_ctor, car_drive_mission_ctor, car_drive_wander_ctor, enter_car_as_driver_ctor,
    enter_car_as_passenger_ctor, go_to_point_and_stand_still_ctor, leave_car_ctor, pause_ctor,
    set_ped_decision_maker_type, simple_car_drive_ctor, stand_still_ctor, task_wander_cop_ctor,
};
use sa_world::world::cworld::in_vehicle;

use crate::bindings::drive_to_point_ctor;
orig! {
    /// `CTaskComplexTreatAccident::Constructor(CAccident*)` (0x658AB0, `ret 4`)
    fn treat_accident_ctor(t: GPtr<u8>, accident: u32) -> GPtr<u8> = thiscall 0x658AB0 => unported;
    /// the swat rope's anim task constructor (0x6586A0)
    fn swat_rope_anim_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x6586A0 => swat_rope_anim_task_ctor;
}

/// `new` of `size` bytes from the task pool, then `ctor` on it (null stays null).
fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let m = task_new(size);
    if m.is_null() { GPtr::NULL } else { ctor(m) }
}

/// `CTaskComplexCopInCar::CreateSubTask(int32 type, CPed* ped)` (0x68C9E0, `ret 8`; the tail runs through the island
/// 0x40600B). +0xC the vehicle, +0x14 the target ped. 0x2BC / 0x2BD: the ped's decision maker type 6, enter the vehicle
/// as passenger (seat 0) / driver; 0x2C0: decision maker 1, leave it; 0x2C5: simple car drive; 0xCB: stand still (1000,
/// looped, 8.0); 0x390: `CTaskComplexWanderCop(4, GetRandomNumberInRange(0, 8))`; 0x44F: police pursuit; 0x2D4: a
/// car-drive mission after the target (its vehicle when it is in one): for a player in a vehicle the wanted-level mission
/// and speed (`FindPoliceCarMissionForWantedLevel` / `FindPoliceCarSpeedForWantedLevel`), on foot mission 0x2B with that
/// speed; for anyone else the handling's +0x88 * 54 and, in a vehicle, mission 4 when `rand() & 3` (return address
/// 0x68CC57) is below 2 else 2, on foot 0x37; the speed goes through `_ftol`, the radius is 10. Anything else: null.
#[hook(addr = 0x68C9E0, conv = thiscall, name = "CTaskComplexCopInCar::CreateSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn cop_in_car_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let veh = || rd::<GPtr<u8>>(this, 0xC);
    let dm = |t: i32| set_ped_decision_maker_type(rd::<GPtr<u8>>(ped, 0x47C), t);
    match ty {
        0x2BC => {
            dm(6);
            new_task(0x50, |m| enter_car_as_passenger_ctor(m, veh(), 0, 0))
        }
        0xCB => new_task(0x20, |m| stand_still_ctor(m, 0x3E8, 1, 0, 8.0)),
        0x2BD => {
            dm(6);
            new_task(0x50, |m| enter_car_as_driver_ctor(m, veh()))
        }
        0x2C0 => {
            dm(1);
            new_task(0x34, |m| leave_car_ctor(m, veh(), 0, 0, 1, 0))
        }
        0x2C5 => new_task(0x60, |m| simple_car_drive_ctor(m, veh(), GPtr::NULL, 0)),
        0x390 => new_task(0x50, |m| {
            let dir = general_random_int_in_range(0, 8);
            task_wander_cop_ctor(m, 4, dir as u32)
        }),
        0x44F => new_task(0x18, police_pursuit_ctor),
        0x2D4 => {
            let target = || rd::<GPtr<u8>>(this, 0x14);
            let (mission, speed): (i32, f32) = if ped_is_player(target()) != 0 {
                if in_vehicle(target()) {
                    let m = i32::from(find_police_car_mission() as i8);
                    (m, xi(find_police_car_speed(veh())).to_f32())
                } else {
                    (0x2B, xi(find_police_car_speed(veh())).to_f32())
                }
            } else {
                let speed = x(rd::<f32>(rd::<GPtr<u8>>(veh(), 0x384), 0x88)).fmul_m(g::<f32>(0x858C94).read()).to_f32();
                if in_vehicle(target()) {
                    let r = (sa_core::crt::rand_at(0x68CC57) & 3) as i8;
                    (if r < 2 { 4 } else { 2 }, speed)
                } else {
                    (0x37, speed)
                }
            };
            let m = task_new(0x2C);
            if m.is_null() {
                return GPtr::NULL;
            }
            let t = target();
            let e = if in_vehicle(t) { rd::<GPtr<u8>>(t, 0x58C) } else { t };
            car_drive_mission_ctor(m, veh(), e, mission as u32, ftol32(x(speed)) as u32, 0x41200000)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexMedicTreatInjuredPed::CreateSubTask(int32 type)` (0x658DB0, `ret 4`). +0xC the ambulance, +0x18 the
/// accident, +0x1C the target position, +0x28 a flag. 0xCB stand still (10000); 0x65 treat the accident; 0x2BC enter as
/// passenger (seat 11); 0x2BD as driver; 0x2C0 (+0x28 = 1 first) leave the vehicle; 0x2C5 simple car drive; 0x2C6 drive
/// to the target (30, mode 0, model -1, radius -1, style 2); 0x2C7 car-drive wander (style 2, 30); 0x387 go to the target
/// and stand still (move state 6, the radii at 0x86F444 / 0x86FC88); 0x390 `CTaskComplexWanderMedic(4,
/// GetRandomNumberInRange(0, 8), true)`; anything else null.
#[hook(addr = 0x658DB0, conv = thiscall, name = "CTaskComplexMedicTreatInjuredPed::CreateSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn medic_treat_injured_ped_create_sub_task(this: GPtr<u8>, ty: i32) -> GPtr<u8> {
    let veh = || rd::<GPtr<u8>>(this, 0xC);
    match ty {
        0xCB => new_task(0x20, |m| stand_still_ctor(m, 0x2710, 0, 0, 8.0)),
        0x65 => new_task(0x10, |m| treat_accident_ctor(m, rd::<u32>(this, 0x18))),
        0x2BC => new_task(0x50, |m| enter_car_as_passenger_ctor(m, veh(), 0xB, 0)),
        0x2BD => new_task(0x50, |m| enter_car_as_driver_ctor(m, veh())),
        0x2C0 => {
            wr::<u8>(this, 0x28, 1);
            new_task(0x34, |m| leave_car_ctor(m, veh(), 0, 0, 1, 0))
        }
        0x2C5 => new_task(0x60, |m| simple_car_drive_ctor(m, veh(), GPtr::NULL, 0)),
        0x2C6 => new_task(0x3C, |m| drive_to_point_ctor(m, veh(), this.byte_add(0x1C), 0x41F00000, 0, u32::MAX, 0xBF800000, 2)),
        0x2C7 => new_task(0x24, |m| car_drive_wander_ctor(m, veh(), 2, 30.0)),
        0x387 => new_task(0x28, |m| {
            let (r, s) = (g::<f32>(0x86F444).read(), g::<f32>(0x86FC88).read());
            go_to_point_and_stand_still_ctor(m, 6, this.byte_add(0x1C).cast(), r, s, 0, 0)
        }),
        0x390 => new_task(0x28, |m| {
            let dir = general_random_int_in_range(0, 8);
            wander_medic_ctor(m, 4, dir as u32, 1)
        }),
        _ => GPtr::NULL,
    }
}

/// The swat rope's anim task constructor (0x6586A0): `CTaskSimpleRunAnim(group 0, anim 0x98, 4.0, task 0x1A7, the
/// name at 0x86F484, false)`, vtable 0x86F45C.
#[hook(addr = 0x6586A0, conv = thiscall, name = "FUN_006586a0", expect = "56 6A 00 68 84 F4 86 00",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn swat_rope_anim_task_ctor(this: GPtr<u8>) -> GPtr<u8> {
    run_anim_ctor2(this, 0, 0x98, 0x40800000, 0x1A7, 0x86F484, 0);
    wr::<u32>(this, 0, 0x86F45C);
    this
}

/// `CTaskComplexUseSwatRope::CreateSubTask(int32 type, CPed* ped)` (0x659620, `ret 8`; the entry fetches the SEH link
/// through the island 0x40113D). 0x1A7 the rope anim task (0x6586A0); 0xC8 `CTaskSimpleNone` (vtable 0x859CFC); 0xCA
/// pause (100000); 0x516: the ped's +0x1C bit 0 set and its move speed (+0x44) zeroed, null; anything else null.
/// Not hookable (the no-CD protection reads the entry bytes at 0x15694D9): phase B only.
#[sa_core::native(addr = 0x659620, conv = thiscall, name = "CTaskComplexUseSwatRope::CreateSubTask")]
pub fn use_swat_rope_create_sub_task(_this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    match ty {
        0x1A7 => new_task(0x20, swat_rope_anim_ctor),
        0xC8 => new_task(8, |m| {
            task_simple_ctor(m);
            wr::<u32>(m, 0, 0x859CFC);
            m
        }),
        0xCA => new_task(0x18, |m| pause_ctor(m, 0x186A0)),
        0x516 => {
            wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) | 1);
            wr::<u32>(ped, 0x44, 0);
            wr::<u32>(ped, 0x48, 0);
            wr::<u32>(ped, 0x4C, 0);
            GPtr::NULL
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexEvasiveCower::CreateSubTask(int32 type)` (0x655570, `ret 4`). 0x386: achieve the heading
/// `GetRadianAngleBetweenPoints(-dir.x, -dir.y, 0, 0)` (+0xC / +0x10, stored as f32; rate 2.0, tolerance at 0x86FC80);
/// 0x19C: `CTaskSimpleRunAnim(0, 0x8F, 4.0, 0x19C, the name at 0x85A0F8, false)` with vtable 0x85A0D0; anything else
/// null.
#[hook(addr = 0x655570, conv = thiscall, name = "CTaskComplexEvasiveCower::CreateSubTask", expect = "6A FF E9 4E E7 DA FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn evasive_cower_create_sub_task(this: GPtr<u8>, ty: i32) -> GPtr<u8> {
    match ty {
        0x386 => new_task(0x18, |m| {
            let h = general_get_radian_angle_between_points(-rd::<f32>(this, 0xC), -rd::<f32>(this, 0x10), 0.0, 0.0);
            achieve_heading_ctor(m, st0(h).to_f32(), 0x40000000, g::<u32>(0x86FC80).read())
        }),
        0x19C => new_task(0x20, |m| {
            run_anim_ctor2(m, 0, 0x8F, 0x40800000, 0x19C, 0x85A0F8, 0);
            wr::<u32>(m, 0, 0x85A0D0);
            m
        }),
        _ => GPtr::NULL,
    }
}

use crate::bindings::t_wait_until_ped_out as wait_until_ped_is_out_car_ctor;
orig! {
    /// `CTaskComplexEnterCarAsDriverTimed`-style constructor `(CVehicle*)` (0x640390, `ret 4`)
    fn enter_car_ctor_640390(t: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x640390 => unported;
    /// the enter-car constructor `(CVehicle*, int32)` (0x6403E0, `ret 8`)
    fn enter_car_ctor_6403e0(t: GPtr<u8>, v: GPtr<u8>, a: u32) -> GPtr<u8> = thiscall 0x6403E0 => unported;
    /// `CTaskSimpleArrestPed::CTaskSimpleArrestPed(CPed*)` (0x68B620, `ret 4`)
    fn simple_arrest_ped_ctor(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x68B620 => unported;
}

use crate::bindings::{drag_ped_from_car_ctor, vehicle_is_driver};
use sa_world::bindings::{destroy_car_ctor, kill_ped_on_foot_ctor, seek_entity_standard_ctor, task_gun_control_ctor, vector_sub};
use sa_core::bindings::general_limit_radian_angle;
use sa_world::world::common::entity_pos;

/// `CTaskComplexArrestPed::CreateSubTask(int32 type, CPed* ped)` (0x68CF80, `ret 8`). +0x10 the suspect. 0x2BF drag the
/// suspect from its car (100000); 0x2D2 / 0x2D8 the enter-car tasks 0x640390 (its vehicle) / 0x6403E0 (its vehicle, 8);
/// 0x33D wait until the suspect is out of its car, with `suspect - ped` (`VectorSub` into a local, before the
/// allocation); 0x38B seek the suspect (50000, 1000, radius 3.0, or 4.0 with its +0x474 bit 0x40, then the floats at
/// 0x859E30 / 0x859E34, true, true); 0x3E8 kill it on foot; 0x3EB destroy its vehicle; 0x44C: a suspect driving its
/// vehicle marks the vehicle (+0x428 bit 0x20, +0x36 status 9 keeping the type bits), then `CTaskSimpleArrestPed`;
/// anything else null.
#[hook(addr = 0x68CF80, conv = thiscall, name = "CTaskComplexArrestPed::CreateSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn arrest_ped_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let suspect = || rd::<GPtr<u8>>(this, 0x10);
    let sveh = || rd::<GPtr<u8>>(suspect(), 0x58C);
    match ty {
        0x2BF => new_task(0x54, |m| drag_ped_from_car_ctor(m, suspect(), 0x186A0)),
        0x2D2 => new_task(0x50, |m| enter_car_ctor_640390(m, sveh())),
        0x2D8 => new_task(0x50, |m| enter_car_ctor_6403e0(m, sveh(), 8)),
        0x33D => {
            let mut d = [0f32; 3];
            let dp = sa_core::field::vec3f_ptr(&mut d);
            vector_sub(dp, entity_pos(suspect()), entity_pos(ped));
            new_task(0x1C, |m| wait_until_ped_is_out_car_ctor(m, suspect(), dp))
        }
        0x38B => {
            let r = if rd::<u8>(suspect(), 0x474) & 0x40 != 0 { 0x870980 } else { 0x87097C };
            let radius = g::<f32>(r).read();
            new_task(0x4C, |m| {
                let (a, b) = (g::<u32>(0x859E30).read(), g::<u32>(0x859E34).read());
                seek_entity_standard_ctor(m, suspect(), 0xC350, 0x3E8, radius, a, b, 1, 1)
            })
        }
        0x3E8 => new_task(0x38, |m| kill_ped_on_foot_ctor(m, suspect(), -1, 0, 0, 0, 1)),
        0x3EB => new_task(0x20, |m| destroy_car_ctor(m, sveh(), 0, 0, 0)),
        0x44C => {
            if !sveh().is_null() && vehicle_is_driver(sveh(), suspect()) != 0 {
                let v = sveh();
                wr::<u8>(v, 0x428, rd::<u8>(v, 0x428) | 0x20);
                let v = sveh();
                wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | 0x48);
            }
            new_task(0x14, |m| simple_arrest_ped_ctor(m, suspect()))
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexWaitForBackup::CreateSubTask(int32 type, CPed* ped)` (0x6973B0, `ret 8`). +0x10 the other ped. 0x2C0
/// leave the ped's vehicle; 0xCA pause (`GetRandomNumberInRange(1000, 2000)` for a ped in its vehicle, else 0); 0x386:
/// the heading `LimitRadianAngle(GetRadianAngleBetweenPoints(other - ped (x, y, f32), 0, 0))` is also stored into the
/// ped's +0x55C, then achieve it (the floats at 0x86FC7C / 0x86FC80); 0x3FC `CTaskSimpleGunControl(other, null, null,
/// 1, 100, 2000)`; 0x516 and anything else null.
#[hook(addr = 0x6973B0, conv = thiscall, name = "CTaskComplexWaitForBackup::CreateSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn wait_for_backup_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    match ty {
        0x2C0 => new_task(0x34, |m| leave_car_ctor(m, rd::<GPtr<u8>>(ped, 0x58C), 0, 0, 1, 0)),
        0xCA => {
            let t = if !rd::<GPtr<u8>>(ped, 0x58C).is_null() && in_vehicle(ped) {
                general_random_int_in_range(0x3E8, 0x7D0) as u32
            } else {
                0
            };
            new_task(0x18, |m| pause_ctor(m, t))
        }
        0x386 => {
            let h = heading_to(ped, rd::<GPtr<u8>>(this, 0x10));
            new_task(0x18, |m| achieve_heading_ctor(m, h, g::<u32>(0x86FC7C).read(), g::<u32>(0x86FC80).read()))
        }
        0x3FC => new_task(0x3C, |m| task_gun_control_ctor(m, rd::<GPtr<u8>>(this, 0x10), GPtr::NULL, 0, 1, 100, 0x7D0)),
        _ => GPtr::NULL,
    }
}

use crate::bindings::car_shuffle_ctor;
use crate::bindings::enter_car_as_passenger_timed_ctor;
orig! {
    /// `CTaskSimpleWaitUntilPedIsInCar::Constructor(CPed*)` (0x649620, `ret 4`)
    fn wait_until_ped_is_in_car_ctor(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x649620 => unported;
}

use crate::bindings::{car_drive_timed_ctor, compute_closest_surface_point_ped, line_up_ped_with_car_ctor};
use sa_core::bindings::{car_set_ped_out_ctor, crt_new};
use sa_world::bindings::{
    car_enter_exit_get_position_to_open_car_door, compute_target_door_to_enter_as_passenger, compute_target_door_to_exit,
    sequence_add_task, sequence_ctor, set_ped_in_as_driver_ctor, set_ped_in_as_passenger_ctor, turn_to_face_entity_ctor,
    vehicle_remove_driver, vehicle_remove_passenger,
};

/// `CTaskComplexShuffleSeats::CreateSubTask(int32 type, CPed* ped)` (0x63D240, `ret 8`; the entry fetches the SEH link
/// through the island 0x4020B0). +0xC the vehicle, +0x10 / +0x14 the doors, +0x24 the line-up utility. 0x32B: the ped
/// leaves its seat (`RemoveDriver(true)` for the driver, else `RemovePassenger(ped)`), then set in as passenger
/// (+0x14); 0x328: shuffle (+0x10); 0x32C: `RemovePassenger(ped)`, then set in as driver; 0x330: set out of its vehicle
/// through `ComputeTargetDoorToExit`, switching the engine off; 0x516: a ped not in a vehicle gets +0x1C bit 0, null;
/// anything else null.
#[hook(addr = 0x63D240, conv = thiscall, name = "CTaskComplexShuffleSeats::CreateSubTask", expect = "90 E9 6A 4E DC FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn shuffle_seats_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let veh = || rd::<GPtr<u8>>(this, 0xC);
    match ty {
        0x32B => {
            if rd::<GPtr<u8>>(veh(), 0x460) == ped {
                vehicle_remove_driver(veh(), 1);
            } else {
                vehicle_remove_passenger(veh(), ped);
            }
            new_task(0x20, |m| set_ped_in_as_passenger_ctor(m, veh(), rd::<u32>(this, 0x14), rd::<GPtr<u8>>(this, 0x24)))
        }
        0x328 => new_task(0x1C, |m| car_shuffle_ctor(m, veh(), rd::<u32>(this, 0x10), rd::<u32>(this, 0x24))),
        0x32C => {
            vehicle_remove_passenger(veh(), ped);
            new_task(0x1C, |m| set_ped_in_as_driver_ctor(m, veh(), rd::<GPtr<u8>>(this, 0x24)))
        }
        0x330 => {
            let door = compute_target_door_to_exit(rd::<GPtr<u8>>(ped, 0x58C), ped);
            new_task(0x18, |m| car_set_ped_out_ctor(m, rd::<GPtr<u8>>(ped, 0x58C), door as u32, 1))
        }
        0x516 => {
            if !ped.is_null() && !in_vehicle(ped) {
                wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) | 1);
            }
            GPtr::NULL
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexShuffleSeats::CreateTaskUtilityLineUpPedWithCar(CPed* ped)` (0x63D610, `ret 4`): the door the ped sits at
/// (+0x10: 10 driver, 8 / 11 the first two passenger slots, else 9), the offset `GetPositionToOpenCarDoor(vehicle,
/// door) - ped position` (each component stored as f32) and `new` (CRT 0x82119A, 0x1C bytes)
/// `CTaskUtilityLineUpPedWithCar(&offset, 600, 0, door)` into +0x24 (null when the allocation fails).
#[hook(addr = 0x63D610, conv = thiscall, name = "CTaskComplexShuffleSeats::CreateTaskUtilityLineUpPedWithCar", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn shuffle_seats_create_task_utility_line_up_ped_with_car(this: GPtr<u8>, ped: GPtr<u8>) {
    let v = rd::<GPtr<u8>>(this, 0xC);
    let door = if rd::<GPtr<u8>>(v, 0x460) == ped {
        10
    } else if rd::<GPtr<u8>>(v, 0x464) == ped {
        8
    } else if rd::<GPtr<u8>>(v, 0x468) == ped {
        11
    } else {
        9
    };
    wr::<u32>(this, 0x10, door);
    let pos = entity_pos(ped);
    let mut t = [0f32; 3];
    let p = car_enter_exit_get_position_to_open_car_door(sa_core::field::vec3f_ptr(&mut t), v, rd::<i32>(this, 0x10));
    let mut off = [0f32; 3];
    for (k, o) in off.iter_mut().enumerate() {
        *o = x(rd::<f32>(p, k as isize * 4)).fsub_m(rd::<f32>(pos, k as isize * 4)).to_f32();
    }
    let m = crt_new(0x1C);
    let u = if m.is_null() {
        GPtr::NULL
    } else {
        line_up_ped_with_car_ctor(m, sa_core::field::vec3f_ptr(&mut off), 0x258, 0, rd::<u32>(this, 0x10))
    };
    wr::<GPtr<u8>>(this, 0x24, u);
}

/// `CTaskComplexEnterCarAsPassengerWait::CreateSubTask(int32 type, CPed* ped)` (0x6408D0, `ret 8`; the tail runs through
/// the island 0x401E7E). +0xC the vehicle, +0x10 the leader, +0x18 a flag, +0x1C the move state. 0x341 wait until the
/// leader is in the car; 0xF4 a sequence: turn to face the vehicle (the floats at 0x86FC7C / 0x86FC80), stand still (1000,
/// 8.0) (a failed allocation adds null; a failed sequence allocation calls `AddTask` on null, as the original); 0x2C8
/// enter as passenger (timed -1, true) at `ComputeTargetDoorToEnterAsPassenger(vehicle, 0)` with +0x18 set, else door
/// 0, then the new task's +0x1C = this +0x1C (written through a null task when the allocation fails, as the original);
/// 0x2CF timed car drive (0); 0x387 go to the vehicle's closest surface point (`ComputeClosestSurfacePoint`; radius 1.5)
/// or, without one, to its position (radius: its col model's bounding radius + 1.5, f32), the second radius at
/// 0x86FC88; 0x516 and anything else null.
#[hook(addr = 0x6408D0, conv = thiscall, name = "CTaskComplexEnterCarAsPassengerWait::CreateSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn enter_car_as_passenger_wait_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let veh = || rd::<GPtr<u8>>(this, 0xC);
    match ty {
        0x341 => new_task(0x10, |m| wait_until_ped_is_in_car_ctor(m, rd::<GPtr<u8>>(this, 0x10))),
        0xF4 => {
            let seq = new_task(0x40, sequence_ctor);
            let turn = new_task(0x28, |m| {
                turn_to_face_entity_ctor(m, veh(), g::<u32>(0x86FC7C).read(), g::<u32>(0x86FC80).read())
            });
            sequence_add_task(seq, turn);
            let stand = new_task(0x20, |m| stand_still_ctor(m, 0x3E8, 0, 0, 8.0));
            sequence_add_task(seq, stand);
            seq
        }
        0x2C8 => {
            let t = if rd::<u8>(this, 0x18) != 0 {
                let door = compute_target_door_to_enter_as_passenger(veh(), 0);
                new_task(0x2C, |m| enter_car_as_passenger_timed_ctor(m, veh(), door, -1i32 as u32, 1))
            } else {
                new_task(0x2C, |m| enter_car_as_passenger_timed_ctor(m, veh(), 0, -1i32 as u32, 1))
            };
            wr::<u32>(t, 0x1C, rd::<u32>(this, 0x1C));
            t
        }
        0x2CF => new_task(0x70, |m| car_drive_timed_ctor(m, veh(), 0)),
        0x387 => {
            let mut p = [0f32; 3];
            let pp = sa_core::field::vec3f_ptr(&mut p);
            if compute_closest_surface_point_ped(ped, veh(), pp) as u8 != 0 {
                new_task(0x28, |m| {
                    go_to_point_and_stand_still_ctor(m, rd::<u32>(this, 0x1C), pp.cast(), 1.5, g::<f32>(0x86FC88).read(), 0, 0)
                })
            } else {
                let mi = g::<GPtr<u8>>(0xA9B0C8 + rd::<i16>(veh(), 0x22) as isize as usize * 4).read();
                let r = x(rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24)).fadd_m(g::<f32>(0x858CE8).read()).to_f32();
                new_task(0x28, |m| {
                    let pos = entity_pos(veh());
                    go_to_point_and_stand_still_ctor(m, rd::<u32>(this, 0x1C), pos.cast(), r, g::<f32>(0x86FC88).read(), 0, 0)
                })
            }
        }
        _ => GPtr::NULL,
    }
}

use sa_core::bindings::{general_get_node_heading_from_vector, ped_is_alive};
use sa_world::bindings::{
    cower_ctor, hands_up_ctor, membership_get_leader, membership_get_member, ped_groups_are_in_same_group,
    ped_groups_get_peds_group, smart_flee_entity_ctor, wander_standard_ctor,
};

/// `LimitRadianAngle(GetRadianAngleBetweenPoints(other - ped (x, y, each stored as f32), 0, 0))`, also stored into the
/// ped's +0x55C (the heading of 0x6973B0 / 0x697B70).
fn heading_to(ped: GPtr<u8>, other: GPtr<u8>) -> f32 {
    let (p, o) = (entity_pos(ped), entity_pos(other));
    let dx = x(rd::<f32>(o, 0)).fsub_m(rd::<f32>(p, 0)).to_f32();
    let dy = x(rd::<f32>(o, 4)).fsub_m(rd::<f32>(p, 4)).to_f32();
    let a = st0(general_get_radian_angle_between_points(dx, dy, 0.0, 0.0)).to_f32();
    let h = st0(general_limit_radian_angle(a)).to_f32();
    wr::<f32>(ped, 0x55C, h);
    h
}

/// `CTaskComplexReactToGunAimedAt::CreateSubTask(int32 type, CPed* ped)` (0x697B70, `ret 8`; the entry fetches the SEH
/// link through the island 0x40A427). +0xC the aimer. 0x19C cower; 0xCA pause (`GetRandomNumberInRange(1000, 2000)` for
/// a ped in its vehicle, else 0); 0x19D hands up (`GetRandomNumberInRange(3000, 5000)`, drawn after the allocation);
/// 0x2C0 leave the ped's vehicle; 0x386 achieve the heading to the aimer (also stored into the ped's +0x55C; the floats
/// at 0x86FC7C / 0x86FC80); 0x38F smart-flee the aimer (the floats at 0xC18CEC, 0x86F674, 0x86F678, 0xC18CF0); 0x390
/// `CTaskComplexWanderStandard(4, GetNodeHeadingFromVector(ped - aimer), true)` (the heading byte is written over the
/// low byte of the ped argument slot and that whole dword passed, as the original), then +0x10 = now, +0x14 = 10000,
/// +0x18 = 1; 0x516 and anything else null.
#[hook(addr = 0x697B70, conv = thiscall, name = "CTaskComplexReactToGunAimedAt::CreateSubTask", expect = "90 E9 B1 28 D7 FF",
       unverified = "frontier2-a2", route = "L2", hazards = "heap-writes")]
pub fn react_to_gun_aimed_at_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let aimer = || rd::<GPtr<u8>>(this, 0xC);
    match ty {
        0x19C => new_task(0x20, cower_ctor),
        0xCA => {
            let t = if !rd::<GPtr<u8>>(ped, 0x58C).is_null() && in_vehicle(ped) {
                general_random_int_in_range(0x3E8, 0x7D0) as u32
            } else {
                0
            };
            new_task(0x18, |m| pause_ctor(m, t))
        }
        0x19D => new_task(0x34, |m| {
            let t = general_random_int_in_range(0xBB8, 0x1388);
            hands_up_ctor(m, t as u32)
        }),
        0x2C0 => new_task(0x34, |m| leave_car_ctor(m, rd::<GPtr<u8>>(ped, 0x58C), 0, 0, 1, 0)),
        0x386 => {
            let h = heading_to(ped, aimer());
            new_task(0x18, |m| achieve_heading_ctor(m, h, g::<u32>(0x86FC7C).read(), g::<u32>(0x86FC80).read()))
        }
        0x38F => new_task(0x40, |m| {
            let f = |a: usize| g::<u32>(a).read();
            smart_flee_entity_ctor(m, aimer(), 0, f32::from_bits(f(0xC18CEC)), f(0x86F674), f(0x86F678), f(0xC18CF0))
        }),
        0x390 => {
            let (p, a) = (entity_pos(ped), entity_pos(aimer()));
            let dy = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(a, 4)).to_f32();
            let dx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(a, 0)).to_f32();
            let dir = general_get_node_heading_from_vector(dx, dy);
            let slot = (ped.addr() as u32 & 0xFFFF_FF00) | u32::from(dir);
            let t = new_task(0x38, |m| wander_standard_ctor(m, 4, slot, 1));
            wr::<u32>(this, 0x10, g::<u32>(0xB7CB84).read());
            wr::<u32>(this, 0x14, 0x2710);
            wr::<u8>(this, 0x18, 1);
            t
        }
        _ => GPtr::NULL,
    }
}

orig! {
    /// `CTaskComplexKillPedGroupOnFoot::ComputeNearestLivingGroupPed(CPed*, bool)` (0x622450, `ret 8`), recursive
    fn nearest_living_group_ped(t: GPtr<u8>, ped: GPtr<u8>, recursed: u8) -> GPtr<u8> = thiscall 0x622450 => kill_ped_group_on_foot_compute_nearest_living_group_ped;
}

/// `CTaskComplexKillPedGroupOnFoot::ComputeNearestLivingGroupPed(CPed* ped, bool recursed)` (0x622450, `ret 8`). The
/// group is `CPedGroups::ms_groups` entry +0xC (its members at 0xC09928 + index * 0x2D4). Not recursed: the leader of
/// `ped`'s own group (not `ped` itself) is excluded through its +0x71C entity when that is a ped, else through the
/// nearest member found for the leader (recursion). Of the eight members, the nearest (squared 3D distance `(dz² + dy²)
/// + dx²` stored as f32, strictly below the best, starting at FLT_MAX) that is not the excluded one, alive with health
/// above 0, not `ped`, not in `ped`'s group, and not a wanted player while `ped` is not a cop and has +0x484 == 1 (the
/// player's `CWanted` read through its player data, as the original, without a null test); none: the excluded one.
#[hook(addr = 0x622450, conv = thiscall, name = "CTaskComplexKillPedGroupOnFoot::ComputeNearestLivingGroupPed", expect = "83 EC 08 8A 44 24 10",
       unverified = "frontier2-a2", route = "L2", hazards = "none")]
pub fn kill_ped_group_on_foot_compute_nearest_living_group_ped(this: GPtr<u8>, ped: GPtr<u8>, recursed: u8) -> GPtr<u8> {
    let group = GPtr::<u8>::from_addr(0xC09928 + rd::<u32>(this, 0xC).wrapping_mul(0x2D4) as usize);
    let mut excluded = GPtr::NULL;
    if recursed == 0 {
        let g = ped_groups_get_peds_group(ped.addr() as u32);
        if g != 0 {
            let leader = membership_get_leader(GPtr::from_addr(g as usize + 8));
            if !leader.is_null() && leader != ped {
                excluded = rd::<GPtr<u8>>(leader, 0x71C);
                if excluded.is_null() || rd::<u8>(excluded, 0x36) & 7 != 3 {
                    excluded = nearest_living_group_ped(this, leader, 1);
                }
            }
        }
    }
    let mut best = GPtr::NULL;
    let mut best_d = f32::MAX;
    for i in 0..8 {
        let m = membership_get_member(group, i);
        if m.is_null() || m == excluded || ped_is_alive(m) as u8 == 0 {
            continue;
        }
        if le_or_nan(x(rd::<f32>(m, 0x540)), x(0.0)) || m == ped || ped_groups_are_in_same_group(m, ped) != 0 {
            continue;
        }
        let (a, b) = (entity_pos(m).cast::<f32>(), entity_pos(ped).cast::<f32>());
        let d = |k: i32| x(a.wrapping_index(k).read()).fsub_m(b.wrapping_index(k).read());
        let (dx, dy, dz) = (d(0), d(1), d(2));
        let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).to_f32();
        if sa_math::fpu::ge_or_nan(x(d2), x(best_d)) {
            continue;
        }
        if ped_is_player(m) != 0 {
            let pd = rd::<GPtr<u8>>(m, 0x480);
            let wanted = if pd.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(pd, 0) };
            if rd::<u8>(wanted, 0x18) != 0 && rd::<i32>(ped, 0x598) != 6 && rd::<u8>(ped, 0x484) == 1 {
                continue;
            }
        }
        best_d = d2;
        best = m;
    }
    if best.is_null() { excluded } else { best }
}

// ---------------------------------------------------------------------------------------------------------
// Batch `frontier2-a4`: CTaskComplexCopInCar::ControlSubTask (the roadblocks scenario's next block).

orig! {
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x20)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x20 => unported;
    /// `CTaskComplexCopInCar::CreateSubTask(int32, CPed*)` (0x68C9E0, `ret 8`)
    fn cop_in_car_create_sub_task_call(t: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x68C9E0 => cop_in_car_create_sub_task;
}

use crate::bindings::task_timer_is_out_of_time;
use sa_math::fpu::{ge_or_nan, gt};
use sa_world::bindings::{vehicle_is_on_its_side, vehicle_is_upside_down};

/// `|move speed|² * TimeStep * 50 > 1` (the vehicle's +0x44 copied into a local), else `|offset|² > 100` (`test ah,
/// 41h` on both: NaN fails).
fn far_or_fast(veh: GPtr<u8>, offset: GPtr<u8>) -> bool {
    let mut s = [rd::<f32>(veh, 0x44), rd::<f32>(veh, 0x48), rd::<f32>(veh, 0x4C)];
    let fast = vector_magnitude_sqr(sa_core::field::vec3f_ptr(&mut s))
        .fmul_m(g::<f32>(0xB7CB5C).read())
        .fmul_m(g::<f32>(0x858B40).read());
    gt(fast, x(g::<f32>(0x858624).read())) || gt(vector_magnitude_sqr(offset), x(g::<f32>(0x858628).read()))
}

/// Flag byte +0x30 with bit 3 set to the target's in-vehicle bit (+0x46C bit 8), as the bytes' xor sequence.
fn with_target_in_vehicle_bit(this: GPtr<u8>, f: u8) -> u8 {
    let b = (((rd::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x46C) >> 8) as u8) << 3 ^ f) & 8;
    f ^ b
}

/// `CTaskComplexCopInCar::ControlSubTask(CPed* ped)` (0x68FD50, `ret 4`). +8 the sub-task, +0xC the cop's vehicle, +0x10
/// the partner, +0x14 the target, +0x18 / +0x24 timers (`CTaskTimer`, +0x20 the first one's set flag), +0x30 flags (bit 0
/// partner gone, 1 chase on foot allowed, 2 short timer, 3 target was in a vehicle). No target or a dead one (state
/// 0x37): finished (0x516) when the sub-task aborts. An empty vehicle's siren flag +0x3BA is cleared; a missing or dead
/// partner sets bit 0. By the sub-task's type: 0x2D4 car-drive mission: a change of the target's in-vehicle bit
/// restarts the mission (bit 3 updated); else with bit 1 a simple car drive (0x2C5). 0x2C5: with bit 1 and bit 0 clear
/// the first sub-task again (bit 1 cleared); with bit 0 the timer +0x18 starts (3000 + rand 0..2000 with bit 2, else
/// 250 + rand 0..1250); once started, a target in another vehicle that moves fast or is more than 10 away restarts the
/// mission (bits 1 / 2 cleared, timer off, bit 3 updated), else the timer's end restarts the first sub-task. 0x390:
/// when the timer +0x24 ends, police pursuit (0x44F). 0x44F: a target in another vehicle, the cop's vehicle alive (+0x4C0
/// > 0), not burning (+0x490), upright and not on its side, bit 0 set, the target vehicle fast or far, and the cop's
/// vehicle within 4 of the ped (squared distance stored as f32, below 16): enter as driver (0x2BD). Every switch needs
/// the sub-task's `MakeAbortable(ped, 1, null)`; otherwise the sub-task stays.
#[hook(addr = 0x68FD50, conv = thiscall, name = "CTaskComplexCopInCar::ControlSubTask", expect = "83 EC 1C 53 56 8B F1",
       unverified = "frontier2-a4", route = "L2", hazards = "heap-writes")]
pub fn cop_in_car_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    let target = rd::<GPtr<u8>>(this, 0x14);
    let abort = || v_make_abortable(sub, ped, 1, GPtr::NULL) != 0;
    let create = |ty: i32| cop_in_car_create_sub_task_call(this, ty, ped);
    if target.is_null() || rd::<i32>(target, 0x530) == 0x37 {
        return if abort() { create(0x516) } else { sub };
    }
    let v = rd::<GPtr<u8>>(this, 0xC);
    if !v.is_null() && rd::<u32>(v, 0x460) == 0 && rd::<u8>(v, 0x3BA) != 0 {
        wr::<u8>(v, 0x3BA, 0);
    }
    let f = rd::<u8>(this, 0x30);
    if f & 1 == 0 {
        let p = rd::<GPtr<u8>>(this, 0x10);
        if p.is_null() || rd::<i32>(p, 0x530) == 0x37 {
            wr::<u8>(this, 0x30, f | 1);
        }
    }
    match v_task_type(rd::<GPtr<u8>>(this, 8)) {
        0x2D4 => {
            let t = rd::<GPtr<u8>>(this, 0x14);
            if ((rd::<u32>(t, 0x46C) >> 5) ^ u32::from(rd::<u8>(this, 0x30))) & 8 != 0 && abort() {
                let nf = with_target_in_vehicle_bit(this, rd::<u8>(this, 0x30));
                wr::<u8>(this, 0x30, nf);
                return create(0x2D4);
            }
            if rd::<u8>(this, 0x30) & 2 != 0 && abort() {
                return create(0x2C5);
            }
            sub
        }
        0x2C5 => {
            let f = rd::<u8>(this, 0x30);
            if f & 2 == 0 {
                return sub;
            }
            if f & 1 == 0 {
                if !abort() {
                    return sub;
                }
                wr::<u8>(this, 0x30, rd::<u8>(this, 0x30) & 0xFD);
                return v_create_first_sub_task(this, ped);
            }
            if rd::<u8>(this, 0x20) == 0 {
                let d = if f & 4 != 0 {
                    general_random_int_in_range(0, 0x7D0) + 0xBB8
                } else {
                    general_random_int_in_range(0, 0x4E2) + 0xFA
                };
                wr::<i32>(this, 0x1C, d);
                wr::<u32>(this, 0x18, g::<u32>(0xB7CB84).read());
                wr::<u8>(this, 0x20, 1);
                return sub;
            }
            let t = rd::<GPtr<u8>>(this, 0x14);
            let tv = rd::<GPtr<u8>>(t, 0x58C);
            if in_vehicle(t) && !tv.is_null() && tv != rd::<GPtr<u8>>(this, 0xC) {
                let mut a = [0f32; 3];
                let ap = sa_core::field::vec3f_ptr(&mut a);
                vector_sub(ap, entity_pos(tv), entity_pos(ped));
                if far_or_fast(tv, ap) && abort() {
                    let nf = rd::<u8>(this, 0x30) & 0xF9;
                    wr::<u8>(this, 0x30, nf);
                    wr::<u8>(this, 0x20, 0);
                    let nf = with_target_in_vehicle_bit(this, nf);
                    wr::<u8>(this, 0x30, nf);
                    return create(0x2D4);
                }
            }
            if task_timer_is_out_of_time(this.byte_add(0x18)) != 0 && abort() {
                wr::<u8>(this, 0x30, rd::<u8>(this, 0x30) & 0xFD);
                wr::<u8>(this, 0x20, 0);
                return v_create_first_sub_task(this, ped);
            }
            sub
        }
        0x390 => {
            if task_timer_is_out_of_time(this.byte_add(0x24)) != 0 && abort() {
                return create(0x44F);
            }
            sub
        }
        0x44F => {
            let t = rd::<GPtr<u8>>(this, 0x14);
            let tv = rd::<GPtr<u8>>(t, 0x58C);
            let cv = rd::<GPtr<u8>>(this, 0xC);
            if !in_vehicle(t) || tv.is_null() || tv == cv || cv.is_null() || rd::<u8>(this, 0x30) & 1 == 0 {
                return sub;
            }
            if le_or_nan(x(rd::<f32>(cv, 0x4C0)), x(0.0)) || rd::<u32>(cv, 0x490) != 0 {
                return sub;
            }
            if vehicle_is_upside_down(cv) != 0 || vehicle_is_on_its_side(rd::<GPtr<u8>>(this, 0xC)) != 0 {
                return sub;
            }
            let tv = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x14), 0x58C);
            let mut a = [0f32; 3];
            let ap = sa_core::field::vec3f_ptr(&mut a);
            vector_sub(ap, entity_pos(rd::<GPtr<u8>>(this, 0xC)), entity_pos(ped));
            let near = vector_magnitude_sqr(ap).to_f32();
            vector_sub(ap, entity_pos(tv), entity_pos(ped));
            if !far_or_fast(tv, ap) || ge_or_nan(x(near), x(g::<f32>(0x8599D0).read())) {
                return sub;
            }
            if abort() { create(0x2BD) } else { sub }
        }
        _ => sub,
    }
}

/// Phase B registrations of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_USE_SWAT_ROPE_CREATE_SUB_TASK];

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_ARREST_PED_CREATE_SUB_TASK,
    HOOK_WAIT_FOR_BACKUP_CREATE_SUB_TASK,
    HOOK_COP_IN_CAR_CREATE_SUB_TASK,
    HOOK_MEDIC_TREAT_INJURED_PED_CREATE_SUB_TASK,
    HOOK_SWAT_ROPE_ANIM_TASK_CTOR,
    HOOK_EVASIVE_COWER_CREATE_SUB_TASK,
    HOOK_SHUFFLE_SEATS_CREATE_SUB_TASK,
    HOOK_SHUFFLE_SEATS_CREATE_TASK_UTILITY_LINE_UP_PED_WITH_CAR,
    HOOK_ENTER_CAR_AS_PASSENGER_WAIT_CREATE_SUB_TASK,
    HOOK_REACT_TO_GUN_AIMED_AT_CREATE_SUB_TASK,
    HOOK_KILL_PED_GROUP_ON_FOOT_COMPUTE_NEAREST_LIVING_GROUP_PED,
    HOOK_COP_IN_CAR_CONTROL_SUB_TASK,
];
