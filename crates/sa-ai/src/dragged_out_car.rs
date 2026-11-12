//! `CEventHandler::ComputeDraggedOutCarResponse` (batch `intro-ai-5`): a ped dragged out of its vehicle. Ported from
//! the 1.0 US bytes (gta-reversed as a reading aid; the bytes win). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::Say(context, offset, volume, b, b, b)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, ctx: u32, offset: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CEventHandler::IsKillTaskAppropriate(CPed*, CPed*, const CEvent&)` (0x4BC3E0, static)
    fn handler_is_kill_task_appropriate(ped: GPtr<u8>, other: GPtr<u8>, e: GPtr<u8>) -> u8 = cdecl 0x4BC3E0 => crate::event_responses_simple::handler_is_kill_task_appropriate;
    /// `CGeneral::GetRandomNumberInRange<int32>(min, max)` (0x407180; draws its own rand)
    fn general_random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(x1, y1, x2, y2)` (0x53CBE0, ST0)
    fn general_get_radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn general_limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10, `ret 4`)
    fn sequence_add_task(t: GPtr<u8>, task: GPtr<u8>) = thiscall 0x632D10 => unported;
    fn smart_flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, d: u32, time: u32, period: u32, f: u32) -> GPtr<u8> = thiscall 0x65C430 => unported;
    fn leave_car_ctor(t: GPtr<u8>, veh: GPtr<u8>, a: i32, b: i32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    fn achieve_heading_ctor(t: GPtr<u8>, heading: u32, rate: u32, tol: u32) -> GPtr<u8> = thiscall 0x667E20 => unported;
    fn shake_fist_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x690B80 => unported;
    fn pause_ctor(t: GPtr<u8>, time: i32) -> GPtr<u8> = thiscall 0x48E750 => unported;
    fn steal_car_ctor(t: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x63B7D0 => unported;
    fn drag_ped_from_car_ctor(t: GPtr<u8>, ped: GPtr<u8>, a: u32) -> GPtr<u8> = thiscall 0x640430 => unported;
    fn enter_car_as_passenger_ctor(t: GPtr<u8>, veh: GPtr<u8>, seat: u32, b: u32) -> GPtr<u8> = thiscall 0x640340 => unported;
    fn enter_car_as_driver_ctor(t: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x6402F0 => unported;
    fn kill_ped_on_foot_ctor(t: GPtr<u8>, ped: GPtr<u8>, time: i32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x620E30 => unported;
    fn kill_criminal_ctor(t: GPtr<u8>, ped: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x68BE70 => unported;
    fn destroy_car_ctor(t: GPtr<u8>, veh: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x621C00 => unported;
}

const FLEE_SAFE_DIST: usize = 0xC18CEC;
const FLEE_TIME: usize = 0x86F674;
const FLEE_PERIOD: usize = 0x86F678;
const FLEE_SHIFT: usize = 0xC18CF0;
/// `CTaskSimpleAchieveHeading` defaults (rate, tolerance)
const HEADING_A: usize = 0x86FC7C;
const HEADING_B: usize = 0x86FC80;

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

/// The sequence's first task: `rand() & 0x3FF` (call site `ret_site`) above 0x200 → face the dragger
/// (`CTaskSimpleAchieveHeading(LimitRadianAngle(GetRadianAngleBetweenPoints(dragger, ped)), <defaults>)`, the
/// dragger NOT NULL-checked as the original) and shake a fist; else `CTaskSimplePause(GetRandomNumberInRange(500,
/// 1500))` (drawn after the allocation).
fn react(this: GPtr<u8>, seq: GPtr<u8>, dragger: GPtr<u8>, ret_site: u32) {
    if at_site(ret_site, rand) & 0x3FF > 0x200 {
        let pp = pos_ptr(ped(this));
        let dp = pos_ptr(dragger);
        let a = f32_of(X87::from_f64_exact(general_get_radian_angle_between_points(
            rd::<u32>(dp, 0), rd::<u32>(dp, 4), rd::<u32>(pp, 0), rd::<u32>(pp, 4),
        )));
        let heading = f32_of(X87::from_f64_exact(general_limit_radian_angle(a.to_bits())));
        let t = new_task(0x18, |t| achieve_heading_ctor(t, heading.to_bits(), gu(HEADING_A), gu(HEADING_B)));
        sequence_add_task(seq, t);
        sequence_add_task(seq, new_task(0x10, shake_fist_ctor));
    } else {
        let t = new_task(0x18, |t| pause_ctor(t, general_random_int_in_range(500, 1500)));
        sequence_add_task(seq, t);
    }
}

/// `CTaskComplexDragPedFromCar(dragger, 0)` then `CTaskComplexEnterCarAsPassenger(vehicle, 0, false)` (the second
/// allocation failing adds NULL).
fn drag_back(seq: GPtr<u8>, dragger: GPtr<u8>, veh: GPtr<u8>) {
    sequence_add_task(seq, new_task(0x54, |t| drag_ped_from_car_ctor(t, dragger, 0)));
    sequence_add_task(seq, new_task(0x50, |t| enter_car_as_passenger_ctor(t, veh, 0, 0)));
}

/// `CEventHandler::ComputeDraggedOutCarResponse(CEventDraggedOutCar*, CTask*, CTask*)` (0x4BCC30, thiscall, `ret
/// 0xC`). Event +0x14 the dragger, +0x18 the vehicle, +0x1C (byte) "dragged by its owner" (`own`). Nothing for a
/// player or without the vehicle. A vehicle whose handling (+0x384) byte +0xDE is 0xF → smart flee from the dragger
/// (defaults), done. Else `leave` = `CTaskComplexLeaveCar(vehicle, 0, 0, false, true)` when the ped is the vehicle's
/// driver (+0x460) or first passenger (+0x464); +0x24 = NULL; by task:
/// - 200: NULL. 0x2BC: `own` → `CTaskComplexEnterCarAsDriver(vehicle)`, else `CTaskComplexEnterCarAsPassenger(vehicle,
///   0, false)`. 0x3EB: `CTaskComplexDestroyCar(vehicle, 0, 0, 0)`. 0x451: with a dragger: a player → NULL, else
///   `CTaskComplexKillCriminal(dragger, false)`.
/// - 0x38F: a dragger → smart flee from it; none and no driver → `CTaskComplexStealCar(vehicle)`.
/// - 0x2BE: +0x24 = a sequence (NULL used as the original does): the reaction (`react`), `CPed::Say(0x78, 0, 1.0, 0,
///   0, 0)`, then `own` → steal the car back, else drag the dragger out and get in as a passenger.
/// - 0x3E8 with a dragger: `IsKillTaskAppropriate(ped, dragger, event)` fails → smart flee; else a sequence: the
///   reaction, `CTaskComplexKillPedOnFoot(dragger, -1, 0, 0, 0, true)`, say, then enter as driver (`own`) or
///   passenger. Without a dragger: a sequence: the reaction (facing a NULL dragger reads address 0x14, as the
///   original), then `own` → steal the car back, else drag a NULL ped out and enter as a passenger.
/// - Others: nothing. Finally, with `leave`, +0x24 = a new sequence of `leave` and the current response.
#[hook(addr = 0x4bcc30, conv = thiscall, name = "CEventHandler::ComputeDraggedOutCarResponse", expect = "6A FF 68 05 B2 83 00",
       this = "CEventHandler", route = "L2", hazards = "none", recs = "grove-walk")]
#[allow(clippy::too_many_lines)]
pub fn handler_compute_dragged_out_car_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let own = rd::<u8>(e, 0x1C) != 0;
    let dragger = rd::<GPtr<u8>>(e, 0x14);
    let veh = rd::<GPtr<u8>>(e, 0x18);
    if ped_is_player(ped(this)) != 0 || veh.is_null() {
        return;
    }
    if rd::<u8>(rd::<GPtr<u8>>(veh, 0x384), 0xDE) == 0xF {
        set(this, new_task(0x40, |t| {
            smart_flee_entity_ctor(t, dragger, 0, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT))
        }));
        return;
    }
    let p = ped(this);
    let ty = i32::from(rd::<i16>(e, 0xE));
    let mut leave = GPtr::<u8>::NULL;
    if rd::<GPtr<u8>>(veh, 0x460) == p || rd::<GPtr<u8>>(veh, 0x464) == p {
        leave = new_task(0x34, |t| leave_car_ctor(t, veh, 0, 0, 0, 1));
    }
    set(this, GPtr::NULL);
    let flee = |this: GPtr<u8>| {
        set(this, new_task(0x40, |t| {
            smart_flee_entity_ctor(t, dragger, 0, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT))
        }));
    };
    let say = |this: GPtr<u8>| {
        let p = ped(this);
        if !p.is_null() {
            ped_say(p, 0x78, 0, 0x3F80_0000, 0, 0, 0);
        }
    };
    match ty {
        200 => set(this, GPtr::NULL),
        0x2BC => {
            if own {
                set(this, new_task(0x50, |t| enter_car_as_driver_ctor(t, veh)));
            } else {
                set(this, new_task(0x50, |t| enter_car_as_passenger_ctor(t, veh, 0, 0)));
            }
        }
        0x2BE => {
            let seq = new_task(0x40, sequence_ctor);
            set(this, seq);
            react(this, seq, dragger, 0x4BCDBE);
            say(this);
            if own {
                sequence_add_task(seq, new_task(0x20, |t| steal_car_ctor(t, veh)));
            } else {
                drag_back(seq, dragger, veh);
            }
        }
        0x38F => {
            if !dragger.is_null() {
                flee(this);
            } else if rd::<GPtr<u8>>(veh, 0x460).is_null() {
                set(this, new_task(0x20, |t| steal_car_ctor(t, veh)));
            }
        }
        0x3E8 => {
            if dragger.is_null() {
                let seq = new_task(0x40, sequence_ctor);
                set(this, seq);
                react(this, seq, dragger, 0x4BD487);
                if own {
                    sequence_add_task(seq, new_task(0x20, |t| steal_car_ctor(t, veh)));
                } else {
                    drag_back(seq, GPtr::NULL, veh);
                }
            } else if handler_is_kill_task_appropriate(ped(this), dragger, e) == 0 {
                flee(this);
            } else {
                let seq = new_task(0x40, sequence_ctor);
                set(this, seq);
                react(this, seq, dragger, 0x4BD269);
                sequence_add_task(seq, new_task(0x38, |t| kill_ped_on_foot_ctor(t, dragger, -1, 0, 0, 0, 1)));
                say(this);
                if own {
                    sequence_add_task(seq, new_task(0x50, |t| enter_car_as_driver_ctor(t, veh)));
                } else {
                    sequence_add_task(seq, new_task(0x50, |t| enter_car_as_passenger_ctor(t, veh, 0, 0)));
                }
            }
        }
        0x3EB => set(this, new_task(0x20, |t| destroy_car_ctor(t, veh, 0, 0, 0))),
        0x451 => {
            if dragger.is_null() {
                return;
            }
            if ped_is_player(dragger) != 0 {
                set(this, GPtr::NULL);
            } else {
                set(this, new_task(0x20, |t| kill_criminal_ctor(t, dragger, 0)));
            }
        }
        _ => {}
    }
    if !leave.is_null() {
        let seq2 = new_task(0x40, sequence_ctor);
        sequence_add_task(seq2, leave);
        sequence_add_task(seq2, rd::<GPtr<u8>>(this, 0x24));
        set(this, seq2);
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_HANDLER_COMPUTE_DRAGGED_OUT_CAR_RESPONSE];
