//! `CEventHandler` responses to dead / panicking peds and knock-overs, `Flush` and the static
//! `ComputeEventResponseTask(const CPed&, const CEvent&)` (batch `intro-ai-3`). Ported from the 1.0 US bytes
//! (gta-reversed `EventHandler.cpp` as a reading aid; the bytes win). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEvent::GetSourceEntity` (vtable +0x28)
    fn v_event_source(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x28 => unported;
    /// Event / task scalar deleting destructor (vtable +0)
    fn v_delete(e: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPedIntelligence::GetTaskFighting` (0x600F30)
    fn intel_get_task_fighting(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F30 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::LookAt(...)` (0x618970, `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: u32, torso: u32,
                  speed: u32, blend: i32, prio: i32, b: u32) = thiscall 0x618970 => unported;
    /// `CCam::GetWeaponFirstPersonOn` (0x509DC0)
    fn cam_get_weapon_first_person_on(cam: GPtr<u8>) -> u8 = thiscall 0x509DC0 => unported;
    /// `CEventHandler::IsKillTaskAppropriate(CPed*, CPed*, const CEvent&)` (0x4BC3E0, static)
    fn handler_is_kill_task_appropriate(ped: GPtr<u8>, other: GPtr<u8>, e: GPtr<u8>) -> u8 = cdecl 0x4BC3E0 => crate::event_responses_simple::handler_is_kill_task_appropriate;
    /// `CEventHandler::ComputeEventResponseTask(CEvent*, CTask*)` (0x4C3870, `ret 8`)
    fn handler_compute_event_response_task(h: GPtr<u8>, e: GPtr<u8>, aborted: GPtr<u8>) = thiscall 0x4C3870 => unported;
    /// `CEventHandler::Flush` (0x4C3790, ported below)
    fn handler_flush_orig(h: GPtr<u8>) = thiscall 0x4C3790 => handler_flush;
    fn smart_flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, d: u32, time: u32, period: u32, f: u32) -> GPtr<u8> = thiscall 0x65C430 => unported;
    fn flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, d: u32, time: u32, period: u32, f: u32) -> GPtr<u8> = thiscall 0x65B930 => unported;
    fn duck_ctor(t: GPtr<u8>, ty: u32, time: u32, counter: i32) -> GPtr<u8> = thiscall 0x691FC0 => unported;
    fn investigate_dead_ped_ctor(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x696DF0 => unported;
    fn kill_ped_on_foot_ctor(t: GPtr<u8>, ped: GPtr<u8>, time: i32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x620E30 => unported;
    fn kill_criminal_ctor(t: GPtr<u8>, ped: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x68BE70 => unported;
}

/// `g_ikChainMan`
const IK_CHAIN_MANAGER: usize = 0xC15448;
/// "CompDeadPedResp"-style look-at name
const S_DEAD_PED: u32 = 0x85B9E4;
/// The squared distance below which a player looks at a dead ped
const K_DEAD_PED_LOOK_DIST2: usize = 0x85B9F4;
/// `TheCamera.ActiveCam` (u8) and `TheCamera.Cams` (0x238 bytes each, +0xC the mode)
const ACTIVE_CAM: usize = 0xB6F081;
const CAMS: usize = 0xB6F19C;
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

fn smart_flee(other: GPtr<u8>, b: u32) -> GPtr<u8> {
    new_task(0x40, |t| smart_flee_entity_ctor(t, other, b, gu(FLEE_SAFE_DIST), gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT)))
}

/// `CEventHandler::ComputeDeadPedResponse` (0x4B9470, thiscall, `ret 0xC`): with the dead ped (+0x14). `d2` = the
/// 2D squared distance (x87, f32; the original stores it over its event argument slot).
/// - A player: `d2` below the threshold (0x85B9F4), no fighting task, not already looking, the active camera's mode
///   not 7 / 0x2E / 8 / 0x33 / 0x22 / 0x10 / 0x2D and not first-person weapon mode → `LookAt(0x85B9E4, ped, dead,
///   500, 5, NULL, true, 0.25, 500, 3, false)`. No response write.
/// - Else by task: 200 → NULL; 0x258 → `CTaskComplexInvestigateDeadPed(dead)`; 0x1AB → `CTaskSimpleDuck(0, 0xE0FF,
///   -1)`; 0x38F → smart flee `(dead, true, <defaults>)`; others: nothing.
#[hook(addr = 0x4b9470, conv = thiscall, name = "CEventHandler::ComputeDeadPedResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-3", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_dead_ped_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let dead = rd::<GPtr<u8>>(e, 0x14);
    if dead.is_null() {
        return;
    }
    let dp = pos_ptr(dead);
    let p = ped(this);
    let mp = pos_ptr(p);
    let dx = x(rd::<f32>(mp, 0)).fsub_m(rd::<f32>(dp, 0));
    let dy = x(rd::<f32>(mp, 4)).fsub_m(rd::<f32>(dp, 4));
    let d2 = f32_of(dy.fmul(dy).fadd(dx.fmul(dx)));
    if ped_is_player(p) != 0 {
        if x(d2).fcom(x(g::<f32>(K_DEAD_PED_LOOK_DIST2).read())).test_parity(0x05) {
            return;
        }
        if !intel_get_task_fighting(rd::<GPtr<u8>>(ped(this), 0x47C)).is_null() {
            return;
        }
        if ik_is_looking(GPtr::from_addr(IK_CHAIN_MANAGER), ped(this)) != 0 {
            return;
        }
        let cam = GPtr::<u8>::from_addr(CAMS + 0x238 * usize::from(g::<u8>(ACTIVE_CAM).read()));
        let mode = rd::<u16>(cam, 0xC);
        if [7u16, 0x2E, 8, 0x33, 0x22, 0x10, 0x2D].contains(&mode) || cam_get_weapon_first_person_on(cam) != 0 {
            return;
        }
        ik_look_at(GPtr::from_addr(IK_CHAIN_MANAGER), S_DEAD_PED, ped(this), dead, 500, 5, 0, 1, 0x3E80_0000, 500, 3, 0);
        return;
    }
    match i32::from(rd::<i16>(e, 0xE)) {
        200 => set(this, GPtr::NULL),
        0x258 => set(this, new_task(0x3C, |t| investigate_dead_ped_ctor(t, dead))),
        0x1AB => set(this, new_task(0x28, |t| duck_ctor(t, 0, 0xE0FF, -1))),
        0x38F => set(this, smart_flee(dead, 1)),
        _ => {}
    }
}

/// `CEventHandler::ComputeGotKnockedOverByCarResponse` (0x4C3430): with the vehicle (+0x14); `driver` = its +0x460.
/// Task 200 → NULL; 0x38F → smart flee from the driver, or from the vehicle without one; 0x3E8 → with a driver,
/// `IsKillTaskAppropriate(ped, driver, event)` → `CTaskComplexKillPedOnFoot(driver, -1, 0, 0, 0, true)`, else smart
/// flee from the driver; 0x451 → with a driver: a player → NULL, else `CTaskComplexKillCriminal(driver, false)`.
/// Others (and the driver cases without a driver): nothing.
#[hook(addr = 0x4c3430, conv = thiscall, name = "CEventHandler::ComputeGotKnockedOverByCarResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-3", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_got_knocked_over_by_car_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(e, 0x14);
    if veh.is_null() {
        return;
    }
    let driver = rd::<GPtr<u8>>(veh, 0x460);
    match i32::from(rd::<i16>(e, 0xE)) {
        200 => set(this, GPtr::NULL),
        0x38F => set(this, smart_flee(if driver.is_null() { veh } else { driver }, 0)),
        0x3E8 => {
            if driver.is_null() {
                return;
            }
            if handler_is_kill_task_appropriate(ped(this), driver, e) != 0 {
                set(this, new_task(0x38, |t| kill_ped_on_foot_ctor(t, driver, -1, 0, 0, 0, 1)));
            } else {
                set(this, smart_flee(driver, 0));
            }
        }
        0x451 => {
            if driver.is_null() {
                return;
            }
            if ped_is_player(driver) != 0 {
                set(this, GPtr::NULL);
            } else {
                set(this, new_task(0x20, |t| kill_criminal_ctor(t, driver, 0)));
            }
        }
        _ => {}
    }
}

/// `CEventHandler::ComputeSeenPanickedPedResponse` (0x4C35F0): the panicking ped = the event's source entity; its
/// intelligence's current event (+0x40, else +0x3C) must exist and have a source entity `threat`. Task 0x1AB →
/// `CTaskSimpleDuck(0, 0xE0FF, -1)`; 0x38D → `CTaskComplexFleeEntity(threat, true, 45.0, <defaults>)`; 0x38F →
/// `CTaskComplexSmartFleeEntity(threat, true, 45.0, <defaults>)`; others: nothing.
#[hook(addr = 0x4c35f0, conv = thiscall, name = "CEventHandler::ComputeSeenPanickedPedResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-3", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_seen_panicked_ped_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let src = v_event_source(e);
    if src.is_null() {
        return;
    }
    let intel = rd::<GPtr<u8>>(src, 0x47C);
    let mut cur = rd::<GPtr<u8>>(intel, 0x40);
    if cur.is_null() {
        cur = rd::<GPtr<u8>>(intel, 0x3C);
        if cur.is_null() {
            return;
        }
    }
    let threat = v_event_source(cur);
    if threat.is_null() {
        return;
    }
    match i32::from(rd::<i16>(e, 0xE)) {
        0x1AB => set(this, new_task(0x28, |t| duck_ctor(t, 0, 0xE0FF, -1))),
        0x38D => set(this, new_task(0x3C, |t| flee_entity_ctor(t, threat, 1, 0x4234_0000, gu(FLEE_ENT_A), gu(FLEE_ENT_B), gu(FLEE_ENT_C)))),
        0x38F => set(this, new_task(0x40, |t| smart_flee_entity_ctor(t, threat, 1, 0x4234_0000, gu(FLEE_TIME), gu(FLEE_PERIOD), gu(FLEE_SHIFT)))),
        _ => {}
    }
}

/// `CEventHandler::Flush` (0x4C3790, thiscall): deletes (scalar deleting destructor, flag 1) and clears, in this
/// order, the tasks at +0x20, +0x24, +0x28, +0x2C, +0x30 and the history's events at +0xC, +8, +0x10 (+0x10 is only
/// written when it was set).
#[hook(addr = 0x4c3790, conv = thiscall, name = "CEventHandler::Flush", expect = "56 8B F1 8B 4E 20",
       this = "CEventHandler", unverified = "intro-ai-3", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_flush(this: GPtr<u8>) {
    for o in [0x20isize, 0x24, 0x28, 0x2C, 0x30, 0xC, 8] {
        let t = rd::<GPtr<u8>>(this, o);
        if !t.is_null() {
            v_delete(t, 1);
        }
        wr::<GPtr<u8>>(this, o, GPtr::NULL);
    }
    let t = rd::<GPtr<u8>>(this, 0x10);
    if !t.is_null() {
        v_delete(t, 1);
        wr::<GPtr<u8>>(this, 0x10, GPtr::NULL);
    }
}

/// `CEventHandler::ComputeEventResponseTask(const CPed&, const CEvent&)` (0x4C4220, static cdecl): a stack
/// `CEventHandler` (0x34 bytes: +0 the ped, the rest zero) runs `ComputeEventResponseTask(event, NULL)`; its response
/// (+0x24) is taken (and cleared); `Flush`; the history's +0xC, +8, +0x10 are deleted when still set (the inlined
/// destructor). Returns the response.
#[hook(addr = 0x4c4220, conv = cdecl, name = "CEventHandler::ComputeEventResponseTask-Ped", expect = "6A FF 68 08 BC 83 00",
       unverified = "intro-ai-3", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn handler_compute_event_response_task_for_ped(p: GPtr<u8>, e: GPtr<u8>) -> GPtr<u8> {
    let mut h = [0u32; 13];
    let hp: GPtr<u8> = GPtr::from_raw(h.as_mut_ptr()).cast();
    wr::<GPtr<u8>>(hp, 0, p);
    handler_compute_event_response_task(hp, e, GPtr::NULL);
    let mut result = GPtr::NULL;
    let r = rd::<GPtr<u8>>(hp, 0x24);
    if !r.is_null() {
        result = r;
        wr::<GPtr<u8>>(hp, 0x24, GPtr::NULL);
    }
    handler_flush_orig(hp);
    for o in [0xCisize, 8, 0x10] {
        let t = rd::<GPtr<u8>>(hp, o);
        if !t.is_null() {
            v_delete(t, 1);
        }
    }
    result
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_HANDLER_COMPUTE_DEAD_PED_RESPONSE,
    HOOK_HANDLER_COMPUTE_GOT_KNOCKED_OVER_BY_CAR_RESPONSE,
    HOOK_HANDLER_COMPUTE_SEEN_PANICKED_PED_RESPONSE,
    HOOK_HANDLER_FLUSH,
    HOOK_HANDLER_COMPUTE_EVENT_RESPONSE_TASK_FOR_PED,
];
