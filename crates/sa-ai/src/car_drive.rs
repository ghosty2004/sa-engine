//! PORT-AI-3 (`docs/arch/ai-port.md`, section PORT-AI-3): driving: `CTaskComplexCarDrive` sub-task
//! creation/control, the `SetUpCar` family (`CarDrive`, `CarDriveWander`, `CarDriveMission`, `DriveToPoint`) and
//! `CTaskComplexDriveToPoint::Drive`.
//! Ported from the 1.0 US bytes (the EXE wins over gta-reversed and the specs). Every call to another
//! catalogued function goes through `orig!` at the original address; task objects keep their original
//! vtables and come from the original task pool (`common::task_new`).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CTaskComplexCarDrive::SetUpCar()` (vtable +0x2C)
    fn v_set_up_car(t: GPtr<u8>) = thiscall vslot 0x2C => unported;
    /// `CTaskComplexCarDrive::CreateSubTaskCannotGetInCar(CPed*)` (vtable +0x30)
    fn v_create_sub_task_cannot_get_in_car(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x30 => unported;
    /// `CTaskComplexCarDrive::Drive(CPed*)` (vtable +0x34)
    fn v_drive(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x34 => unported;

    /// `CTaskComplexCarDrive::CreateSubTask(int32 taskType, CPed*)` (0x642FA0, `ret 8`)
    fn car_drive_create_sub_task_orig(t: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x642FA0 => unported;
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskSimpleCarDrive::Constructor(veh, a1, a2)` (0x63C340, `ret 0xC`)
    fn car_drive_simple_ctor(t: GPtr<u8>, veh: u32, a1: u32, a2: u32) -> GPtr<u8> = thiscall 0x63C340 => unported;
    /// `CTaskComplexEnterCarAsDriver::CTaskComplexEnterCarAsDriver(CVehicle*)` (0x6402F0, `ret 4`)
    fn enter_car_as_driver_ctor(t: GPtr<u8>, veh: u32) -> GPtr<u8> = thiscall 0x6402F0 => unported;
    /// `CTaskComplexEnterCarAsPassenger::CTaskComplexEnterCarAsPassenger(CVehicle*, int32, bool)` (0x640340, `ret 0xC`)
    fn enter_car_as_passenger_ctor(t: GPtr<u8>, veh: u32, a1: u32, a2: u32) -> GPtr<u8> = thiscall 0x640340 => unported;
    /// `CTaskComplexLeaveAnyCar::CTaskComplexLeaveAnyCar(int32, bool, bool)` (0x421150, `ret 0xC`)
    fn leave_any_car_ctor(t: GPtr<u8>, a0: u32, a1: u32, a2: u32) -> GPtr<u8> = thiscall 0x421150 => unported;
    /// `CTaskComplexWander::GetWanderTaskByPedType(CPed*)` (0x673D00, cdecl)
    fn wander_get_wander_task_by_ped_type(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x673D00 => unported;
    /// `CTaskSimpleCreateCarAndGetIn::CTaskSimpleCreateCarAndGetIn(const CVector*, int32 model)` (0x6493E0, `ret 8`)
    fn create_car_and_get_in_ctor(t: GPtr<u8>, pos: GPtr<u8>, model: u32) -> GPtr<u8> = thiscall 0x6493E0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity** ref)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity** ref)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CUpsideDownCarCheck::IsCarUpsideDown-vehicle(const CVehicle*)` (0x463830, `this` unused)
    fn upside_down_car_check_is_car_upside_down(c: GPtr<u8>, v: GPtr<u8>) -> u8 = thiscall 0x463830 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystem(CVehicle*)` (0x42F5A0, cdecl)
    fn car_ctrl_join_car_with_road_system(v: GPtr<u8>) = cdecl 0x42F5A0 => unported;
    /// `CTaskComplexDriveToPoint::IsTargetBlocked(CPed*)` (0x6452C0, `ret 4`)
    fn drive_to_point_is_target_blocked_orig(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6452C0 => unported;
    /// `CCarAI::GetCarToGoToCoors(CVehicle*, const CVector*, int32 style, bool)` (0x41CE30; ST0 result, popped)
    fn car_ai_get_car_to_go_to_coors(v: GPtr<u8>, p: GPtr<u8>, style: u32, b: u32) -> f64 = cdecl 0x41CE30 => unported;
    /// `CCarAI::GetCarToGoToCoorsAccurate` (0x41D0E0; ST0 result, popped)
    fn car_ai_get_car_to_go_to_coors_accurate(v: GPtr<u8>, p: GPtr<u8>, style: u32, b: u32) -> f64 = cdecl 0x41D0E0 => unported;
    /// `CCarAI::GetCarToGoToCoorsStraightLine` (0x41CFB0; ST0 result, popped)
    fn car_ai_get_car_to_go_to_coors_straight_line(v: GPtr<u8>, p: GPtr<u8>, style: u32, b: u32) -> f64 = cdecl 0x41CFB0 => unported;
    /// `CCarAI::GetCarToGoToCoorsRacing` (0x41D210; ST0 result, popped)
    fn car_ai_get_car_to_go_to_coors_racing(v: GPtr<u8>, p: GPtr<u8>, style: u32, b: u32) -> f64 = cdecl 0x41D210 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

/// True when the ped is in a vehicle: `ped->m_pVehicle` (+0x58C) set and `bInVehicle` (bit 8 of +0x46C).
/// (The bytes test the two in either order; both are plain reads.)
fn ped_in_vehicle(ped: GPtr<u8>) -> bool {
    !rd::<GPtr<u8>>(ped, 0x58C).is_null() && rd::<u32>(ped, 0x46C) & 0x100 != 0
}

/// The part every `SetUpCar` starts with: saves the vehicle's driving style (+0x3B9), mission (+0x3BA) and
/// cruise speed (+0x3D0) bytes into +0x1D/+0x1E/+0x1F and sets +0x20.
fn save_autopilot(this: GPtr<u8>, veh: GPtr<u8>) {
    wr::<u8>(this, 0x1D, rd::<u8>(veh, 0x3B9));
    wr::<u8>(this, 0x1E, rd::<u8>(veh, 0x3BA));
    wr::<u8>(this, 0x1F, rd::<u8>(veh, 0x3D0));
    wr::<u8>(this, 0x20, 1);
}

/// Cruise speed from the task's float (+0x10): `veh->+0x3D0 = (u8)_ftol(+0x10)`, then `veh->+0x3C8 =
/// (float)veh->+0x3D0` (fild of the zero-extended byte).
fn set_cruise_speed(this: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(this, 0xC);
    wr::<u8>(veh, 0x3D0, ftol32(x(rd::<f32>(this, 0x10))) as u8);
    let veh = rd::<GPtr<u8>>(this, 0xC);
    wr::<f32>(veh, 0x3C8, f32_of(xi(rd::<u8>(veh, 0x3D0) as i32)));
}

/// Vehicle flag byte +0x428: bit 0x10 set unless bit 2 of +0x42D is set, else cleared.
fn set_flag_428(veh: GPtr<u8>) {
    let f = rd::<u8>(veh, 0x428);
    if rd::<u8>(veh, 0x42D) & 2 == 0 {
        wr::<u8>(veh, 0x428, f | 0x10);
    } else {
        wr::<u8>(veh, 0x428, f & 0xEF);
    }
}

/// `CTaskComplexCarDrive::SetUpCar()` (0x63CAE0): saves the autopilot bytes of the vehicle (+0xC) only.
#[hook(addr = 0x63CAE0, conv = thiscall, name = "CTaskComplexCarDrive::SetUpCar", expect = "8B 41 0C 8A 90 B9 03 00 00")]
pub fn car_drive_set_up_car(this: GPtr<u8>) {
    save_autopilot(this, rd::<GPtr<u8>>(this, 0xC));
}

/// `CTaskComplexCarDriveWander::SetUpCar()` (0x63CB60): saves the autopilot bytes; unless the byte at
/// 0x9690CC is set: `CCarCtrl::JoinCarWithRoadSystem(veh)`, entity status bits (+0x36) = 3 (`& 7 | 0x18`),
/// mission (+0x3BA) = 1, cruise speed from +0x10, driving style (+0x3B9) = +0x18. Then the +0x428 flag and
/// `veh->+0x3AC = now`.
#[hook(addr = 0x63CB60, conv = thiscall, name = "CTaskComplexCarDriveWander::SetUpCar", expect = "51 56 8B F1 8B 46 0C")]
pub fn car_drive_wander_set_up_car(this: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(this, 0xC);
    save_autopilot(this, veh);
    if g::<u8>(0x9690CC).read() == 0 {
        car_ctrl_join_car_with_road_system(veh);
        let v = rd::<GPtr<u8>>(this, 0xC);
        wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | 0x18);
        wr::<u8>(rd::<GPtr<u8>>(this, 0xC), 0x3BA, 1);
        set_cruise_speed(this);
        wr::<u8>(rd::<GPtr<u8>>(this, 0xC), 0x3B9, rd::<u8>(this, 0x18));
    }
    set_flag_428(rd::<GPtr<u8>>(this, 0xC));
    wr::<u32>(rd::<GPtr<u8>>(this, 0xC), 0x3AC, g::<u32>(TIME_MS).read());
}

/// `CTaskComplexCarDriveMission::SetUpCar()` (0x63CD20): saves the autopilot bytes,
/// `JoinCarWithRoadSystem(veh)`, status bits (+0x36) = 3, mission (+0x3BA) = +0x28, cruise speed from +0x10,
/// driving style = +0x18, target vehicle (+0x41C) = +0x24, `veh->+0x3AC = now`; registers the target
/// reference when set; then the +0x428 flag.
#[hook(addr = 0x63CD20, conv = thiscall, name = "CTaskComplexCarDriveMission::SetUpCar", expect = "51 56 8B F1 8B 46 0C")]
pub fn car_drive_mission_set_up_car(this: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(this, 0xC);
    save_autopilot(this, veh);
    car_ctrl_join_car_with_road_system(veh);
    let v = rd::<GPtr<u8>>(this, 0xC);
    wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | 0x18);
    wr::<u8>(rd::<GPtr<u8>>(this, 0xC), 0x3BA, rd::<u8>(this, 0x28));
    set_cruise_speed(this);
    wr::<u8>(rd::<GPtr<u8>>(this, 0xC), 0x3B9, rd::<u8>(this, 0x18));
    wr::<u32>(rd::<GPtr<u8>>(this, 0xC), 0x41C, rd::<u32>(this, 0x24));
    let now = g::<u32>(TIME_MS).read();
    wr::<u32>(rd::<GPtr<u8>>(this, 0xC), 0x3AC, now);
    let target = rd::<GPtr<u8>>(this, 0x24);
    if !target.is_null() {
        entity_register_reference(target, rd::<GPtr<u8>>(this, 0xC).byte_add(0x41C));
    }
    set_flag_428(rd::<GPtr<u8>>(this, 0xC));
}

/// `CTaskComplexDriveToPoint::SetUpCar()` (0x63CF00): saves the autopilot bytes; if the cruise speed float
/// (+0x10) > 0.0 [0x858B50] (strict, NaN skips) `veh->+0x3D0 = (u8)_ftol(+0x10)`; driving style = +0x18;
/// `veh->+0x3AC = now`.
#[hook(addr = 0x63CF00, conv = thiscall, name = "CTaskComplexDriveToPoint::SetUpCar", expect = "56 8B F1 D9 46 10")]
pub fn drive_to_point_set_up_car(this: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(this, 0xC);
    save_autopilot(this, veh);
    if x(rd::<f32>(this, 0x10)).fcom(x(g::<f32>(0x858B50).read())) == sa_math::x87::Cmp::Greater {
        wr::<u8>(rd::<GPtr<u8>>(this, 0xC), 0x3D0, ftol32(x(rd::<f32>(this, 0x10))) as u8);
    }
    wr::<u8>(rd::<GPtr<u8>>(this, 0xC), 0x3B9, rd::<u8>(this, 0x18));
    wr::<u32>(rd::<GPtr<u8>>(this, 0xC), 0x3AC, g::<u32>(TIME_MS).read());
}

/// `CTaskComplexCarDrive::CreateSubTask(int32 taskType, CPed*)` (0x642FA0). 0x2BC: `CTaskComplexEnterCarAsPassenger
/// (veh, 0, 0)` (0x50 bytes); 0x2BD: `CTaskComplexEnterCarAsDriver(veh)` (0x50); 0x2C5: `SetUpCar()` (vtable
/// +0x2C) first, then `CTaskSimpleCarDrive(veh, 0, 0)` (0x60); 0x2CA: `CTaskComplexLeaveAnyCar(0, 1, 0)`
/// (0x14); 0x2CD: a bare `CTaskComplex` with vtable 0x86EA14 (`CTaskComplexEnterAnyCarAsDriver`, 0xC);
/// 0x340: `CTaskSimpleCreateCarAndGetIn(ped position (matrix +0x30 or placement +4), model +0x14)` (0x34);
/// 0x390: `CTaskComplexWander::GetWanderTaskByPedType(ped)`; anything else (incl. 0x516): null. Null when
/// the pool is full.
#[hook(addr = 0x642FA0, conv = thiscall, name = "CTaskComplexCarDrive::CreateSubTask", expect = "64 A1 00 00 00 00")]
pub fn car_drive_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let veh = || rd::<u32>(this, 0xC);
    match ty {
        0x2BC => {
            let t = task_new(0x50);
            if t.is_null() { GPtr::NULL } else { enter_car_as_passenger_ctor(t, veh(), 0, 0) }
        }
        0x2BD => {
            let t = task_new(0x50);
            if t.is_null() { GPtr::NULL } else { enter_car_as_driver_ctor(t, veh()) }
        }
        0x2C5 => {
            v_set_up_car(this);
            let t = task_new(0x60);
            if t.is_null() { GPtr::NULL } else { car_drive_simple_ctor(t, veh(), 0, 0) }
        }
        0x2CA => {
            let t = task_new(0x14);
            if t.is_null() { GPtr::NULL } else { leave_any_car_ctor(t, 0, 1, 0) }
        }
        0x2CD => {
            let t = task_new(0xC);
            if t.is_null() {
                return GPtr::NULL;
            }
            task_complex_ctor(t);
            wr::<u32>(t, 0, 0x86EA14);
            t
        }
        0x340 => {
            let t = task_new(0x34);
            if t.is_null() {
                return GPtr::NULL;
            }
            let m = rd::<GPtr<u8>>(ped, 0x14);
            let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
            create_car_and_get_in_ctor(t, pos, rd::<u32>(this, 0x14))
        }
        0x390 => wander_get_wander_task_by_ped_type(ped),
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexCarDrive::CreateSubTaskCannotGetInCar(CPed*)` (0x643200): `CreateSubTask(0x516, ped)` (null).
#[hook(addr = 0x643200, conv = thiscall, name = "CTaskComplexCarDrive::CreateSubTaskCannotGetInCar", expect = "8B 44 24 04 50")]
pub fn car_drive_create_sub_task_cannot_get_in_car(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    car_drive_create_sub_task_orig(this, 0x516, ped)
}

/// `CTaskComplexCarDrive::CreateNextSubTask(CPed*)` (0x644E20), on the finished sub-task's type (no null
/// check): 0x2BC (enter as passenger) -> in a vehicle ? 0x2C5 : 0x516; 0x2BD (enter as driver) -> in a vehicle ?
/// 0x2C5 : 0x2CD; 0x2C5 (car drive) -> `CreateFirstSubTask(ped)` (vtable +0x24); 0x2CA (leave any car) ->
/// without a vehicle (+0xC) `+0x1C ? 0x2CD : 0x516`, with one `+0x1C ? 0x2BD : 0x2BC`; 0x2CD (enter any car)
/// -> in a vehicle: the vehicle reference (+0xC) moves to the ped's vehicle (`CleanUpOldReference` of the
/// old one when set, `RegisterReference`) and 0x2C5, else `+0x14 >= 0 ? 0x340 :
/// CreateSubTaskCannotGetInCar(ped)` (vtable +0x30); 0x340 (create car) -> in a vehicle: +0xC = the ped's
/// vehicle, registered (no clean-up of the old one), 0x2C5, else vtable +0x30; 0x396 -> 0x516; others null.
#[hook(addr = 0x644E20, conv = thiscall, name = "CTaskComplexCarDrive::CreateNextSubTask", expect = "53 56 8B F1 8B 4E 08")]
pub fn car_drive_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = |ty: i32| car_drive_create_sub_task_orig(this, ty, ped);
    match v_get_task_type(rd::<GPtr<u8>>(this, 8)) {
        0x2BC => sub(if ped_in_vehicle(ped) { 0x2C5 } else { 0x516 }),
        0x2BD => sub(if ped_in_vehicle(ped) { 0x2C5 } else { 0x2CD }),
        0x2C5 => v_create_first_sub_task(this, ped),
        0x2CA => {
            let has = !rd::<GPtr<u8>>(this, 0xC).is_null();
            let flag = rd::<u8>(this, 0x1C) != 0;
            sub(match (has, flag) {
                (false, true) => 0x2CD,
                (false, false) => 0x516,
                (true, true) => 0x2BD,
                (true, false) => 0x2BC,
            })
        }
        0x2CD => {
            if ped_in_vehicle(ped) {
                let r = this.byte_add(0xC);
                let old = rd::<GPtr<u8>>(this, 0xC);
                if !old.is_null() {
                    entity_clean_up_old_reference(old, r);
                }
                let v = rd::<GPtr<u8>>(ped, 0x58C);
                wr::<GPtr<u8>>(this, 0xC, v);
                entity_register_reference(v, r);
                sub(0x2C5)
            } else if rd::<i32>(this, 0x14) >= 0 {
                sub(0x340)
            } else {
                v_create_sub_task_cannot_get_in_car(this, ped)
            }
        }
        0x340 => {
            if ped_in_vehicle(ped) {
                let v = rd::<GPtr<u8>>(ped, 0x58C);
                wr::<GPtr<u8>>(this, 0xC, v);
                entity_register_reference(v, this.byte_add(0xC));
                sub(0x2C5)
            } else {
                v_create_sub_task_cannot_get_in_car(this, ped)
            }
        }
        0x396 => sub(0x516),
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexCarDrive::CreateFirstSubTask(CPed*)` (0x645100). Without a vehicle (+0xC): a ped in a vehicle
/// takes it (+0xC = ped vehicle, `RegisterReference`) and gets 0x2C5; otherwise `+0x1C ? 0x2CD : null`. With
/// one: a ped in a vehicle gets 0x2C5 when it is that vehicle, else 0x2CA; otherwise bikes (+0x590 == 9) get
/// `+0x1C ? 0x2BD : 0x2BC`, other vehicles that `CUpsideDownCarCheck::IsCarUpsideDown` (on a stack object it
/// never reads) reports upside down get `+0x1C ? 0x2CD : null`, the rest `+0x1C ? 0x2BD : 0x2BC`.
#[hook(addr = 0x645100, conv = thiscall, name = "CTaskComplexCarDrive::CreateFirstSubTask", expect = "83 EC 30 53 8B 5C 24 38")]
pub fn car_drive_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = |ty: i32| car_drive_create_sub_task_orig(this, ty, ped);
    let flag = || rd::<u8>(this, 0x1C) != 0;
    let veh = rd::<GPtr<u8>>(this, 0xC);
    if veh.is_null() {
        if ped_in_vehicle(ped) {
            let v = rd::<GPtr<u8>>(ped, 0x58C);
            wr::<GPtr<u8>>(this, 0xC, v);
            entity_register_reference(v, this.byte_add(0xC));
            return sub(0x2C5);
        }
        return if flag() { sub(0x2CD) } else { GPtr::NULL };
    }
    if ped_in_vehicle(ped) {
        return sub(if rd::<GPtr<u8>>(ped, 0x58C) == veh { 0x2C5 } else { 0x2CA });
    }
    if rd::<i32>(veh, 0x590) != 9 {
        let mut check = [0u32; 12];
        let cp: GPtr<u8> = GPtr::from_raw(&mut check as *mut [u32; 12]).cast();
        if upside_down_car_check_is_car_upside_down(cp, veh) != 0 {
            return if flag() { sub(0x2CD) } else { GPtr::NULL };
        }
    }
    sub(if flag() { 0x2BD } else { 0x2BC })
}

/// `CTaskComplexCarDrive::ControlSubTask(CPed*)` (0x645240): the sub-task (+8) is kept unless the ped is in
/// a vehicle and the sub-task is a car drive (0x2C5: returns `Drive(ped)`, vtable +0x34) or 0x396 (+0xC = the
/// ped's vehicle, registered, then `CreateSubTask(0x2C5, ped)`).
#[hook(addr = 0x645240, conv = thiscall, name = "CTaskComplexCarDrive::ControlSubTask", expect = "53 56 57 8B 7C 24 10")]
pub fn car_drive_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    if !ped_in_vehicle(ped) {
        return sub;
    }
    match v_get_task_type(sub) {
        0x2C5 => v_drive(this, ped),
        0x396 if ped_in_vehicle(ped) => {
            let v = rd::<GPtr<u8>>(ped, 0x58C);
            wr::<GPtr<u8>>(this, 0xC, v);
            entity_register_reference(v, this.byte_add(0xC));
            car_drive_create_sub_task_orig(this, 0x2C5, ped)
        }
        _ => sub,
    }
}

/// `CTaskComplexDriveToPoint::Drive(CPed*)` (0x645420). The sub-task (+8) is read at entry. `dist =
/// sqrt((dz*dz + dy*dy) + dx*dx)` of `m_Point (+0x24) - vehicle position` (x87, unrounded). `dist < radius
/// (+0x34)`: mission (+0x3BA) = 0, +0x38 = 1, `CreateSubTask(0x516)`; else `dist < 3.0` [0x86E6A4] with mission
/// 0: +0x38 = 1, `CreateSubTask(0x516)`; else a zero cruise speed byte (+0x3D0) takes `_ftol(+0x10)`;
/// `IsTargetBlocked(ped)`: +0x38 = 1, `CreateSubTask(0x516)`; else by +0x30 (0..3) `CCarAI::GetCarToGoToCoors`
/// / `Accurate` / `StraightLine` / `Racing` (veh, &m_Point, +0x18, 0) and the sub-task read at entry is
/// returned (also for +0x30 > 3, without a call: the original's default branch keeps that stale value).
#[hook(addr = 0x645420, conv = thiscall, name = "CTaskComplexDriveToPoint::Drive", expect = "53 55 56 8B F1")]
pub fn drive_to_point_drive(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    let veh = rd::<GPtr<u8>>(this, 0xC);
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let pos = if m.is_null() { veh.byte_add(4) } else { m.byte_add(0x30) };
    let dx = x(rd::<f32>(this, 0x24)).fsub_m(rd::<f32>(pos, 0));
    let dy = x(rd::<f32>(this, 0x28)).fsub_m(rd::<f32>(pos, 4));
    let dz = x(rd::<f32>(this, 0x2C)).fsub_m(rd::<f32>(pos, 8));
    let dist = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    let finish = || {
        wr::<u8>(this, 0x38, 1);
        car_drive_create_sub_task_orig(this, 0x516, ped)
    };
    if !dist.fcom(x(rd::<f32>(this, 0x34))).test_parity(5) {
        wr::<u8>(veh, 0x3BA, 0);
        return finish();
    }
    if !dist.fcom(x(g::<f32>(0x86E6A4).read())).test_parity(5) && rd::<u8>(veh, 0x3BA) == 0 {
        return finish();
    }
    if rd::<u8>(veh, 0x3D0) == 0 {
        wr::<u8>(veh, 0x3D0, ftol32(x(rd::<f32>(this, 0x10))) as u8);
    }
    if drive_to_point_is_target_blocked_orig(this, ped) != 0 {
        return finish();
    }
    let p = this.byte_add(0x24);
    let (v, style) = (rd::<GPtr<u8>>(this, 0xC), rd::<u32>(this, 0x18));
    match rd::<u32>(this, 0x30) {
        0 => {
            car_ai_get_car_to_go_to_coors(v, p, style, 0);
        }
        1 => {
            car_ai_get_car_to_go_to_coors_accurate(v, p, style, 0);
        }
        2 => {
            car_ai_get_car_to_go_to_coors_straight_line(v, p, style, 0);
        }
        3 => {
            car_ai_get_car_to_go_to_coors_racing(v, p, style, 0);
        }
        _ => {}
    }
    sub
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_CAR_DRIVE_SET_UP_CAR,
    HOOK_CAR_DRIVE_WANDER_SET_UP_CAR,
    HOOK_CAR_DRIVE_MISSION_SET_UP_CAR,
    HOOK_DRIVE_TO_POINT_SET_UP_CAR,
    HOOK_CAR_DRIVE_CREATE_SUB_TASK,
    HOOK_CAR_DRIVE_CREATE_SUB_TASK_CANNOT_GET_IN_CAR,
    HOOK_CAR_DRIVE_CREATE_NEXT_SUB_TASK,
    HOOK_CAR_DRIVE_CREATE_FIRST_SUB_TASK,
    HOOK_CAR_DRIVE_CONTROL_SUB_TASK,
    HOOK_DRIVE_TO_POINT_DRIVE,
];
