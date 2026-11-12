//! Using an attractor (batch `std13-ai-1`): `CTaskComplexUseAttractor`'s sub-task functions, the
//! `CTaskComplexSitDownThenIdleThenStandUp` constructor, `CTaskComplexGoToAttractor::MakeAbortable`, the queue head /
//! tail and `CPedAttractorManager::BroadcastDeparture` (0x5EC980, the type dispatch, and its template 0x5EC660).
//! Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPedAttractor::GetType()` (vtable slot 0)
    fn attractor_get_type(a: GPtr<u8>) -> u32 = thiscall vslot 0x0 => unported;
    /// `CPedAttractor` deleting destructor (vtable slot 4)
    fn attractor_delete(a: GPtr<u8>, flags: u32) = thiscall vslot 0x4 => unported;
    /// `CPedAttractor::BroadcastDeparture(CPed*)` (vtable slot 0x14, `ret 4`)
    fn broadcast_departure_v(a: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall vslot 0x14 => unported;
    /// `CTask::MakeAbortable(CPed*, int priority, const CEvent*)` (vtable slot 0x18, `ret 0xC`)
    fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CPedAttractorManager::IsPedRegisteredWithEffect(CPed*)` (0x5EBCB0, `ret 4`)
    fn is_ped_registered_with_effect(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5EBCB0 => crate::attractor_registry::is_ped_registered_with_effect_ped;
    /// `GetPedAttractorManager()` (0x5EE190)
    fn get_ped_attractor_manager() -> GPtr<u8> = cdecl 0x5EE190 => unported;
    /// `CInterestingEvents::Add(type, CEntity*)` on `g_InterestingEvents` (0x602590, `ret 8`)
    fn interesting_events_add(ie: GPtr<u8>, t: u32, e: GPtr<u8>) = thiscall 0x602590 => unported;
    /// `CGeneral::GetRandomNumberInRange(int, int)` (0x407180)
    fn random_int(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CTaskComplex::CTaskComplex()` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskSimpleUseAtm()` (0x48DFE0)
    fn use_atm_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x48DFE0 => unported;
    /// `CTaskComplexWaitForBus()` (0x632900)
    fn wait_for_bus_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632900 => unported;
    /// `CTaskComplexWaitForDryWeather()` (0x632800)
    fn wait_for_dry_weather_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632800 => unported;
    /// `CTaskSimpleStandStill(int ms, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, ms: i32, looped: u32, idle: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CScriptsForBrains::HasAttractorScriptBrainWithThisNameLoaded(const char*)` (0x46AB20, on 0xA90CF0)
    fn brain_loaded(s: GPtr<u8>, name: GPtr<u8>) -> u8 = thiscall 0x46AB20 => unported;
    /// `CScriptsForBrains::StartAttractorScriptBrainWithThisName(const char*, CPed*, bool)` (0x46B390, `ret 0xC`)
    fn brain_start(s: GPtr<u8>, name: GPtr<u8>, ped: GPtr<u8>, flag: u32) = thiscall 0x46B390 => unported;
    /// `CScriptedBrainTaskStore::Clear(CPed*)` (0x635850)
    fn brain_task_store_clear(ped: GPtr<u8>) = cdecl 0x635850 => unported;
    /// `CTaskComplexUseScriptedBrain(const char*)` (0x6358B0, `ret 4`)
    fn use_scripted_brain_ctor(t: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = thiscall 0x6358B0 => unported;
    /// `CTaskComplexUseScriptedAttractor(CPedAttractor*)` (0x633330, `ret 4`)
    fn use_scripted_attractor_ctor(t: GPtr<u8>, a: GPtr<u8>) -> GPtr<u8> = thiscall 0x633330 => unported;
    /// `CTaskComplexSunbathe(CObject* towel, bool)` (0x631F80, `ret 8`)
    fn sunbathe_ctor(t: GPtr<u8>, towel: u32, b: u32) -> GPtr<u8> = thiscall 0x631F80 => unported;
    fn sit_ctor_o(t: GPtr<u8>, ms: i32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x631460 => sit_down_then_idle_then_stand_up_ctor;
    fn broadcast_departure_t_o(m: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>, arr: GPtr<u8>) -> u8 = thiscall 0x5EC660 => broadcast_departure_t;
    fn manager_broadcast_departure_o(m: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>) -> u8 = thiscall 0x5EC980 => manager_broadcast_departure;
}

/// `g_InterestingEvents`
const INTERESTING_EVENTS: usize = 0xC0B058;
/// `CScriptsForBrains` instance
const SCRIPTS_FOR_BRAINS: usize = 0xA90CF0;

fn count4(a: GPtr<u8>, first: isize) -> i32 {
    let f = rd::<u32>(a, first);
    if f == 0 { 0 } else { (rd::<u32>(a, first + 4).wrapping_sub(f) as i32) >> 2 }
}

/// `CTaskComplexGoToAttractor::MakeAbortable(CPed*, int, const CEvent*)` (0x66B6B0, thiscall, `ret 0xC`): the sub-task's
/// (+8) `MakeAbortable` with the same arguments (tail jump).
#[hook(addr = 0x66B6B0, conv = thiscall, name = "CTaskComplexGoToAttractor::MakeAbortable", expect = "8B 49 08 8B 01 FF 60 18",
       unverified = "std13-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn go_to_attractor_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 {
    task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, prio, ev)
}

/// `CPedAttractor::GetHeadOfQueue()` (0x5EB590, thiscall): the first queued ped (+0x20), null for an empty queue.
#[hook(addr = 0x5EB590, conv = thiscall, name = "CPedAttractor::GetHeadOfQueue", expect = "8B 51 20 85 D2 74 0C",
       unverified = "std13-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn get_head_of_queue(this: GPtr<u8>) -> GPtr<u8> {
    if count4(this, 0x20) == 0 {
        return GPtr::NULL;
    }
    GPtr::<GPtr<u8>>::from_addr(rd::<u32>(this, 0x20) as usize).read()
}

/// `CPedAttractor::GetTailOfQueue()` (0x5EB5B0, thiscall): the last queued ped, null for an empty queue.
#[hook(addr = 0x5EB5B0, conv = thiscall, name = "CPedAttractor::GetTailOfQueue", expect = "8B 51 20 85 D2 74 0C",
       unverified = "std13-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn get_tail_of_queue(this: GPtr<u8>) -> GPtr<u8> {
    let n = count4(this, 0x20);
    if n == 0 {
        return GPtr::NULL;
    }
    GPtr::<GPtr<u8>>::from_addr(rd::<u32>(this, 0x20) as usize + 4 * (n as usize - 1)).read()
}

/// `CTaskComplexSitDownThenIdleThenStandUp(int durationMs, bool sitOnStep, bool instant)` (0x631460, thiscall,
/// `ret 0xC`): `CTaskComplex()`, vtable 0x86DF60, +0xC duration, +0x10 / +0x11 the two flags (bytes), +0x12 = 0.
#[hook(addr = 0x631460, conv = thiscall, name = "CTaskComplexSitDownThenIdleThenStandUp::Constructor",
       expect = "56 8B F1 E8 48 8F FE FF", unverified = "std13-ai-1", route = "L2", hazards = "heap-writes",
       recs = "grove-walk")]
pub fn sit_down_then_idle_then_stand_up_ctor(this: GPtr<u8>, ms: i32, on_step: u32, instant: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<i32>(this, 0xC, ms);
    wr::<u32>(this, 0, 0x86DF60);
    wr::<u8>(this, 0x10, on_step as u8);
    wr::<u8>(this, 0x11, instant as u8);
    wr::<u8>(this, 0x12, 0);
    this
}

/// `CTaskComplexUseAttractor::CreateNextSubTask(CPed*)` (0x632730, thiscall, `ret 4`): after an ATM (type 0) the
/// ped's money (+0x756, u16) grows by `20 - 20 * ftol((rand() & 0xFFFF) * 2^-15 * -50.0)`; after a pizza attractor
/// (type 3) it drops by 10 (to 0 at or below 10). Then `GetPedAttractorManager()->BroadcastDeparture(ped,
/// attractor)`; no sub-task (null).
#[hook(addr = 0x632730, conv = thiscall, name = "CTaskComplexUseAttractor::CreateNextSubTask",
       expect = "56 57 8B F9 8B 4F 0C 8B 01", unverified = "std13-ai-1", route = "L2", hazards = "none",
       recs = "grove-walk", note = "rand() at 0x63276E")]
pub fn use_attractor_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let a = rd::<GPtr<u8>>(this, 0xC);
    match attractor_get_type(a) {
        0 => {
            let r = at_site(0x632773, rand) & 0xFFFF;
            let v = ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x859B60).read()));
            let d = 20i32.wrapping_sub(v.wrapping_mul(20));
            wr::<u16>(ped, 0x756, rd::<u16>(ped, 0x756).wrapping_add(d as u16));
        }
        3 => {
            let w = rd::<u16>(ped, 0x756);
            wr::<u16>(ped, 0x756, w.saturating_sub(10));
        }
        _ => {}
    }
    manager_broadcast_departure_o(get_ped_attractor_manager(), ped, rd::<GPtr<u8>>(this, 0xC));
    GPtr::NULL
}

/// `CTaskComplexUseAttractor::ControlSubTask(CPed*)` (0x6327C0, thiscall, `ret 4`): when the attractor's entity is an
/// object (type 4) with flag 0x400 at +0x140, the sub-task is asked to abort (`MakeAbortable(ped, 0, null)`); returns
/// the sub-task.
#[hook(addr = 0x6327C0, conv = thiscall, name = "CTaskComplexUseAttractor::ControlSubTask",
       expect = "56 8B F1 8B 46 0C 85 C0", unverified = "std13-ai-1", route = "L2", hazards = "none",
       recs = "grove-walk")]
pub fn use_attractor_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let a = rd::<GPtr<u8>>(this, 0xC);
    if !a.is_null() {
        let e = rd::<GPtr<u8>>(a, 8);
        if !e.is_null() && rd::<u8>(e, 0x36) & 7 == 4 && rd::<u32>(e, 0x140) & 0x400 != 0 {
            task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 0, GPtr::NULL);
        }
    }
    rd::<GPtr<u8>>(this, 8)
}

/// `CTaskComplexUseAttractor::CreateFirstSubTask(CPed*)` (0x6385D0, thiscall, `ret 4`, SEH frame). Adds interesting
/// event 3 for the ped, then by the attractor's type: 0 `CTaskSimpleUseAtm`; 1 and 9 `CTaskComplexSitDownThenIdle
/// ThenStandUp(ms, type == 9, ped flag 0x200000)` with ms = 9999999 when a 0..100 draw is above 40, else 15000..60000;
/// 2 `CTaskComplexWaitForBus`; 3 stand still 6000..12000 ms; 4 `CTaskComplexWaitForDryWeather`; 5 the attractor's
/// script brain (`CTaskComplexUseScriptedBrain`, started unless the entity is an object without flag +0x143 bit 0;
/// none when the ped has flag 0x800000 or the brain is not loaded); 6 stand still 3000..6000 ms; 7
/// `CTaskComplexUseScriptedAttractor`; 8 `CTaskComplexSunbathe(null, 1)`. Without a task the attractor manager's
/// `BroadcastDeparture(ped, attractor)` runs. The ped's flag 0x200000 (+0x474) is cleared; returns the task.
#[hook(addr = 0x6385D0, conv = thiscall, name = "CTaskComplexUseAttractor::CreateFirstSubTask",
       expect = "64 A1 00 00 00 00 6A FF", unverified = "std13-ai-1", route = "L2", hazards = "heap-writes",
       recs = "grove-walk", note = "the sit task's flag arguments carry register residue above their low byte")]
pub fn use_attractor_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    interesting_events_add(GPtr::from_addr(INTERESTING_EVENTS), 3, ped);
    let a = rd::<GPtr<u8>>(this, 0xC);
    let flag21 = (rd::<u32>(ped, 0x474) >> 21) & 1;
    let new = |size: u32, f: &dyn Fn(GPtr<u8>) -> GPtr<u8>| {
        let t = task_new(size);
        if t.is_null() { GPtr::NULL } else { f(t) }
    };
    let task = match attractor_get_type(a) {
        0 => new(0x20, &|t| use_atm_ctor(t)),
        1 | 9 => {
            let ms = if random_int(0, 100) > 40 { 9_999_999 } else { random_int(15000, 60000) };
            let t = task_new(0x14);
            if t.is_null() {
                GPtr::NULL
            } else {
                let step = u32::from(attractor_get_type(rd::<GPtr<u8>>(this, 0xC)) == 9);
                sit_ctor_o(t, ms, step, flag21)
            }
        }
        2 => new(0x10, &|t| wait_for_bus_ctor(t)),
        3 => {
            let ms = random_int(6000, 12000);
            new(0x20, &|t| stand_still_ctor(t, ms, 0, 0, 0x4100_0000))
        }
        4 => new(0xC, &|t| wait_for_dry_weather_ctor(t)),
        5 => use_scripted_brain(this, ped),
        6 => {
            let ms = random_int(3000, 6000);
            new(0x20, &|t| stand_still_ctor(t, ms, 0, 0, 0x4100_0000))
        }
        7 => new(0x10, &|t| use_scripted_attractor_ctor(t, rd::<GPtr<u8>>(this, 0xC))),
        8 => new(0x38, &|t| sunbathe_ctor(t, 0, 1)),
        _ => GPtr::NULL,
    };
    if task.is_null() {
        manager_broadcast_departure_o(get_ped_attractor_manager(), ped, rd::<GPtr<u8>>(this, 0xC));
    }
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & 0xFFDF_FFFF);
    task
}

/// Type 5 of `CreateFirstSubTask`: the attractor's script brain.
fn use_scripted_brain(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(ped, 0x474) & 0x80_0000 != 0 {
        return GPtr::NULL;
    }
    let brains = GPtr::<u8>::from_addr(SCRIPTS_FOR_BRAINS);
    let a = rd::<GPtr<u8>>(this, 0xC);
    if brain_loaded(brains, a.byte_add(0x84)) == 0 {
        return GPtr::NULL;
    }
    let a = rd::<GPtr<u8>>(this, 0xC);
    let e = rd::<GPtr<u8>>(a, 8);
    if e.is_null() {
        return GPtr::NULL;
    }
    if rd::<u8>(e, 0x36) & 7 != 4 || rd::<u8>(e, 0x143) & 1 != 0 {
        brain_start(brains, a.byte_add(0x84), ped, (rd::<u32>(ped, 0x474) >> 21) & 1);
    }
    brain_task_store_clear(ped);
    let t = task_new(0x1C);
    if t.is_null() { GPtr::NULL } else { use_scripted_brain_ctor(t, rd::<GPtr<u8>>(this, 0xC).byte_add(0x84)) }
}

/// `CPedAttractorManager::BroadcastDeparture<CPedAttractor>(CPed*, CPedAttractor*, SArray&)` (0x5EC660, thiscall,
/// `ret 0xC`). AL = 0 for a null attractor or one not in the vector (searched by count). Otherwise its
/// `BroadcastDeparture(ped)` (vtable), and when no ped is registered with it any more (queue + attracted = 0) it is
/// erased from the vector (when found by address) and deleted (deleting destructor, flag 1). AL = 1.
#[hook(addr = 0x5EC660, conv = thiscall, name = "CPedAttractorManager::BroadcastDeparture<CPedAttractor>",
       expect = "53 56 57 8B 7C 24 14 85 FF", unverified = "std13-ai-1", route = "L2",
       hazards = "heap-writes,ax-al-return", recs = "grove-walk")]
pub fn broadcast_departure_t(_this: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>, arr: GPtr<u8>) -> u8 {
    if a.is_null() {
        return 0;
    }
    let n = count4(arr, 4);
    let first = rd::<u32>(arr, 4);
    let Some(c) = (0..n)
        .map(|i| GPtr::<GPtr<u8>>::from_addr(first as usize + 4 * i as usize).read())
        .find(|&c| c == a)
    else {
        return 0;
    };
    if c.is_null() {
        return 0;
    }
    broadcast_departure_v(c, ped);
    if count4(c, 0x20) + count4(c, 0x10) != 0 {
        return 1;
    }
    let (mut p, end) = (rd::<u32>(arr, 4), rd::<u32>(arr, 8));
    while p != end {
        if GPtr::<GPtr<u8>>::from_addr(p as usize).read() == c {
            let mut q = p.wrapping_add(4);
            let end = rd::<u32>(arr, 8);
            while q != end {
                GPtr::<u32>::from_addr(q as usize - 4).write(GPtr::<u32>::from_addr(q as usize).read());
                q = q.wrapping_add(4);
            }
            wr::<u32>(arr, 8, rd::<u32>(arr, 8).wrapping_sub(4));
            break;
        }
        p = p.wrapping_add(4);
    }
    attractor_delete(c, 1);
    1
}

/// `CPedAttractorManager::BroadcastDeparture(CPed*, CPedAttractor*)` (0x5EC980, thiscall, `ret 8`): AL = 0 for a null
/// attractor, a ped not registered with an effect or a type above 9; else the template on the type's vector.
#[hook(addr = 0x5EC980, conv = thiscall, name = "CPedAttractorManager::BroadcastDeparture-",
       expect = "56 57 8B 7C 24 10 85 FF", unverified = "std13-ai-1", route = "L2", hazards = "ax-al-return",
       recs = "grove-walk")]
pub fn manager_broadcast_departure(this: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>) -> u8 {
    if a.is_null() || is_ped_registered_with_effect(this, ped) == 0 {
        return 0;
    }
    let t = attractor_get_type(a);
    if t > 9 {
        return 0;
    }
    let arr = match t {
        0 => this.byte_add(0x10),
        1 => this,
        _ => this.byte_add(0x10 * t as isize),
    };
    broadcast_departure_t_o(this, ped, a, arr)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_GO_TO_ATTRACTOR_MAKE_ABORTABLE,
    HOOK_GET_HEAD_OF_QUEUE,
    HOOK_GET_TAIL_OF_QUEUE,
    HOOK_SIT_DOWN_THEN_IDLE_THEN_STAND_UP_CTOR,
    HOOK_USE_ATTRACTOR_CREATE_NEXT_SUB_TASK,
    HOOK_USE_ATTRACTOR_CONTROL_SUB_TASK,
    HOOK_USE_ATTRACTOR_CREATE_FIRST_SUB_TASK,
    HOOK_BROADCAST_DEPARTURE_T,
    HOOK_MANAGER_BROADCAST_DEPARTURE,
];
