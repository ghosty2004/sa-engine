//! `CEventHandler::ComputeVehicleOnFireResponse` and `ComputeSpecialResponse` (batch `intro-ai-3`). Ported from the
//! 1.0 US bytes (gta-reversed `EventHandler.cpp` as a reading aid; the bytes win). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CVehicle::IsDriver(CPed*)` (0x6D1C40, `ret 4`)
    fn vehicle_is_driver(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6D1C40 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CEventHandler::ComputeKnockOffBikeResponse(CEvent*, CTask*, CTask*)` (0x4B9FF0, `ret 0xC`)
    fn compute_knock_off_bike_response(h: GPtr<u8>, e: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = thiscall 0x4B9FF0 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10, `ret 4`)
    fn sequence_add_task(t: GPtr<u8>, task: GPtr<u8>) = thiscall 0x632D10 => unported;
    fn die_ctor(t: GPtr<u8>, wt: u32, group: u32, anim: u32, delta: u32, speed: u32, stealth: u32, falling: u32, dir: u32, rail: u32) -> GPtr<u8> = thiscall 0x630040 => unported;
    fn leave_any_car_ctor(t: GPtr<u8>, delay: i32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x421150 => unported;
    fn leave_car_ctor(t: GPtr<u8>, veh: GPtr<u8>, a: i32, b: i32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    fn leave_car_and_flee_ctor(t: GPtr<u8>, veh: GPtr<u8>, pos: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x63BF90 => unported;
    fn leave_car_and_wander_ctor(t: GPtr<u8>, veh: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x63BDD0 => unported;
    fn smart_flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, d: u32, time: u32, period: u32, f: u32) -> GPtr<u8> = thiscall 0x65C430 => unported;
    fn flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, d: u32, time: u32, period: u32, f: u32) -> GPtr<u8> = thiscall 0x65B930 => unported;
    fn smart_flee_point_ctor(t: GPtr<u8>, pos: GPtr<u8>, b: u32, a: u32, c: u32) -> GPtr<u8> = thiscall 0x65BD20 => unported;
    fn car_drive_mission_flee_scene_ctor(t: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8920 => unported;
    fn kill_ped_on_foot_ctor(t: GPtr<u8>, ped: GPtr<u8>, time: i32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x620E30 => unported;
    fn arrest_ped_ctor(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x68B990 => unported;
    fn use_attractor_ctor(t: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6346F0 => unported;
    fn use_attractor_run_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8790 => unported;
    fn use_attractor_sprint_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8850 => unported;
}

/// 0.0
const K_ZERO: usize = 0x858B50;
const FLEE_SAFE_DIST: usize = 0xC18CEC;
const FLEE_TIME: usize = 0x86F674;
const FLEE_PERIOD: usize = 0x86F678;
const FLEE_SHIFT: usize = 0xC18CF0;
const FLEE_ENT_A: usize = 0x86F660;
const FLEE_ENT_B: usize = 0x86F664;
const FLEE_ENT_C: usize = 0x86F668;

fn gu(a: usize) -> u32 {
    g::<u32>(a).read()
}

fn ped(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0)
}

fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let t = task_new(size);
    if t.is_null() { t } else { ctor(t) }
}

fn set(this: GPtr<u8>, t: GPtr<u8>) {
    wr::<GPtr<u8>>(this, 0x24, t);
}

/// `CEventHandler::ComputeVehicleOnFireResponse` (0x4BB2E0): with the vehicle (+0x14). A wrecked vehicle (status
/// byte +0x36 & 0xF8 == 0x28) or one whose health (+0x4C0) is not above 0 (x87; unordered counts as above) that the
/// ped sits in: bike-like (+0x590 == 9 or +0x594 == 2) → `ComputeKnockOffBikeResponse(event, active, simplest)`;
/// else `CTaskComplexDie(0, 0, 0xF, 4.0, 0, 0, 0, 0, 0)`. Otherwise by task: 0x2C0 → `CTaskComplexLeaveAnyCar(0,
/// false, true)`; 0x2C2 → a sequence (a NULL sequence is used as the original does) of leave-any-car and
/// `CTaskComplexSmartFleeEntity(vehicle, false, 15.0, <defaults>)`; 0x2C3 → a sequence of leave-any-car only; 0x38F →
/// smart flee `(vehicle, false, 15.0, <defaults>)`; 0x38D → `CTaskComplexFleeEntity(vehicle, false, 15.0,
/// <defaults>)`; 0x2D6 → as the driver of its own vehicle `CTaskComplexCarDriveMissionFleeScene(own vehicle)`, else
/// smart flee from its own vehicle (+0x58C, possibly NULL, as the original). Others: nothing.
#[hook(addr = 0x4bb2e0, conv = thiscall, name = "CEventHandler::ComputeVehicleOnFireResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_vehicle_on_fire_response(this: GPtr<u8>, e: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(e, 0x14);
    if veh.is_null() {
        return;
    }
    let dead = rd::<u8>(veh, 0x36) & 0xF8 == 0x28
        || !x(rd::<f32>(veh, 0x4C0)).fcom(x(g::<f32>(K_ZERO).read())).test_parity(0x41);
    if dead {
        let p = ped(this);
        if rd::<u32>(p, 0x46C) & 0x100 != 0 && rd::<GPtr<u8>>(p, 0x58C) == veh {
            if rd::<i32>(veh, 0x590) == 9 || rd::<i32>(veh, 0x594) == 2 {
                compute_knock_off_bike_response(this, e, active, simplest);
            } else {
                set(this, new_task(0x28, |t| die_ctor(t, 0, 0, 0xF, 0x4080_0000, 0, 0, 0, 0, 0)));
            }
            return;
        }
    }
    let leave_any_car = || new_task(0x14, |t| leave_any_car_ctor(t, 0, 0, 1));
    match i32::from(rd::<i16>(e, 0xE)) {
        0x2C0 => set(this, leave_any_car()),
        ty @ (0x2C2 | 0x2C3) => {
            let seq = new_task(0x40, sequence_ctor);
            let leave = leave_any_car();
            sequence_add_task(seq, leave);
            if ty == 0x2C3 {
                set(this, seq);
                return;
            }
            let flee = new_task(0x40, |t| {
                smart_flee_entity_ctor(t, veh, 0, 0x4170_0000, gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT))
            });
            sequence_add_task(seq, flee);
            set(this, seq);
        }
        0x38F => set(this, new_task(0x40, |t| {
            smart_flee_entity_ctor(t, veh, 0, 0x4170_0000, gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT))
        })),
        0x38D => set(this, new_task(0x3C, |t| {
            flee_entity_ctor(t, veh, 0, 0x4170_0000, gu(FLEE_ENT_A), gu(FLEE_ENT_B), gu(FLEE_ENT_C))
        })),
        0x2D6 => {
            let p = ped(this);
            let own = rd::<GPtr<u8>>(p, 0x58C);
            if !own.is_null() && vehicle_is_driver(own, p) != 0 {
                set(this, new_task(0x2C, |t| car_drive_mission_flee_scene_ctor(t, rd::<GPtr<u8>>(ped(this), 0x58C))));
            } else {
                set(this, new_task(0x40, |t| {
                    let own = rd::<GPtr<u8>>(ped(this), 0x58C);
                    smart_flee_entity_ctor(t, own, 0, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT))
                }));
            }
        }
        _ => {}
    }
}

fn player_pos() -> GPtr<u8> {
    pos_ptr(find_player_ped(-1))
}

/// `CEventHandler::ComputeSpecialResponse` (0x4BB800): by task (others: nothing): 200 / 0x451 → NULL; 0x102 / 0x105
/// / 0x106 → the closest-free-scripted-attractor tasks; 0x2C0 → in a vehicle (+0x58C) `CTaskComplexLeaveCar(vehicle,
/// 0, 0, true, false)`; 0x2C2 → `CTaskComplexLeaveCarAndFlee(vehicle, &playerPos, 0, 0, 0)`; 0x2C3 →
/// `CTaskComplexLeaveCarAndWander(vehicle, 0, 0, true)` (the vehicle cases need one, else nothing); 0x2D6 →
/// `CTaskComplexCarDriveMissionFleeScene(NULL)`; 0x38E → `CTaskComplexSmartFleePoint(&playerPos, false, 1000.0,
/// 100000)`; 0x38F → smart flee from the player (defaults); 0x3E8 → `CTaskComplexKillPedOnFoot(player, -1, 0, 0, 0,
/// true)`; 0x44D → `CTaskComplexArrestPed(player)`. The player (`FindPlayerPed(-1)`) is looked up after the allocation.
#[hook(addr = 0x4bb800, conv = thiscall, name = "CEventHandler::ComputeSpecialResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-3", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_special_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let veh = || rd::<GPtr<u8>>(ped(this), 0x58C);
    match i32::from(rd::<i16>(e, 0xE)) {
        200 | 0x451 => set(this, GPtr::NULL),
        0x102 => set(this, new_task(0x10, |t| use_attractor_ctor(t, 0))),
        0x105 => set(this, new_task(0x10, use_attractor_run_ctor)),
        0x106 => set(this, new_task(0x10, use_attractor_sprint_ctor)),
        0x2C0 => {
            if veh().is_null() {
                return;
            }
            set(this, new_task(0x34, |t| leave_car_ctor(t, veh(), 0, 0, 1, 0)));
        }
        0x2C2 => {
            if veh().is_null() {
                return;
            }
            set(this, new_task(0x28, |t| {
                let pos = player_pos();
                leave_car_and_flee_ctor(t, veh(), pos, 0, 0, 0)
            }));
        }
        0x2C3 => {
            if veh().is_null() {
                return;
            }
            set(this, new_task(0x1C, |t| leave_car_and_wander_ctor(t, veh(), 0, 0, 1)));
        }
        0x2D6 => set(this, new_task(0x2C, |t| car_drive_mission_flee_scene_ctor(t, GPtr::NULL))),
        0x38E => set(this, new_task(0x44, |t| smart_flee_point_ctor(t, player_pos(), 0, 0x447A_0000, 100_000))),
        0x38F => set(this, new_task(0x40, |t| {
            let player = find_player_ped(-1);
            smart_flee_entity_ctor(t, player, 0, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT))
        })),
        0x3E8 => set(this, new_task(0x38, |t| kill_ped_on_foot_ctor(t, find_player_ped(-1), -1, 0, 0, 0, 1))),
        0x44D => set(this, new_task(0x24, |t| arrest_ped_ctor(t, find_player_ped(-1)))),
        _ => {}
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_HANDLER_COMPUTE_VEHICLE_ON_FIRE_RESPONSE, HOOK_HANDLER_COMPUTE_SPECIAL_RESPONSE];
