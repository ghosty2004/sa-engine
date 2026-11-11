//! The nearby-fire scanner `FUN_00603E70` (batch `mvpk-ai-4`): called from `CEventScanner::ScanForEvents`, raises
//! `CEventFireNearby` and `CEventPotentialWalkIntoFire` for a ped near the closest fire. Ported from the 1.0 US bytes.
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn tm_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360)
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CFireManager::FindNearestFire(const CVector&, bool, bool)` (0x538F40, `ret 0xC`)
    fn fire_find_nearest_fire(fm: GPtr<u8>, pos: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x538F40 => unported;
    /// `CEventFireNearby::CEventFireNearby(const CVector&)` (0x4B1F10, `ret 4`)
    fn event_fire_nearby_ctor(e: GPtr<u8>, pos: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B1F10 => unported;
    /// `CEventFireNearby::~CEventFireNearby()` (0x4B1F80)
    fn event_fire_nearby_dtor(e: GPtr<u8>) = thiscall 0x4B1F80 => unported;
    /// `CEventPotentialWalkIntoFire::CEventPotentialWalkIntoFire(const CVector&, float size, int32 moveState)` (0x4B1E20,
    /// `ret 0xC`)
    fn event_walk_into_fire_ctor(e: GPtr<u8>, pos: GPtr<u8>, size: u32, ms: u32) -> GPtr<u8> = thiscall 0x4B1E20 => unported;
    /// `CEventPotentialWalkIntoFire::~CEventPotentialWalkIntoFire()` (0x4B1F00)
    fn event_walk_into_fire_dtor(e: GPtr<u8>) = thiscall 0x4B1F00 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
}

/// `gFireManager` (0xB71F80), `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const FIRE_MANAGER: usize = 0xB71F80;
const TIMER_TIME_MS: usize = 0xB7CB84;

fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `FUN_00603E70` (thiscall, `ret 4`; the nearby-fire scanner of `CEventScanner`, timer +0 start, +4 period, +8 set, +9
/// reset): unset → (now, 0), set; reset → start = now; not `now >= start + period` (unsigned) → nothing; else (now, 100).
/// `ms` = the simplest active task's +8 when it is a go-to task, else 1. With the nearest fire (`FindNearestFire(ped
/// position, false, false)`): `d = fire pos (+4) - ped` (z as f32), `d2 = f32((dx² + dy²) + dz²)`; `d2` below 20² and
/// `|dz|` below 2 → a stack `CEventFireNearby(fire pos)` into the ped's event group. Then an active task not of type 0x202,
/// a go-to simplest task and that fire: 4² above `d2` and `|dz|` below 2 → a stack `CEventPotentialWalkIntoFire(fire pos,
/// fire +0x1C, ms)` likewise. The original keeps `ms` in its own ped argument slot.
#[hook(addr = 0x603e70, conv = thiscall, name = "FUN_00603e70", expect = "64 A1 00 00 00 00",
       unverified = "mvpk-ai-4", route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "the events live on the Rust stack; the ped argument slot holds the move state")]
pub fn nearby_fire_scanner_scan(this: GPtr<u8>, ped: GPtr<u8>) {
    let now = || g::<u32>(TIMER_TIME_MS).read();
    if rd::<u8>(this, 8) == 0 {
        wr::<u32>(this, 0, now());
        wr::<u8>(this, 8, 1);
        wr::<u32>(this, 4, 0);
    }
    if rd::<u8>(this, 9) != 0 {
        wr::<u32>(this, 0, now());
        wr::<u8>(this, 9, 0);
    }
    let due = rd::<u32>(this, 4).wrapping_add(rd::<u32>(this, 0));
    let t = now();
    if t < due {
        return;
    }
    wr::<u32>(this, 0, t);
    wr::<u32>(this, 4, 100);
    wr::<u8>(this, 8, 1);
    let tm = || rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
    let simplest = tm_get_simplest_active_task(tm());
    let ms = if !simplest.is_null() && task_is_go_to_task(simplest) != 0 { rd::<u32>(simplest, 8) } else { 1 };
    let active = tm_get_active_task(tm());
    let fire = fire_find_nearest_fire(GPtr::from_addr(FIRE_MANAGER), entity_pos(ped), 0, 0);
    let mut d2 = 0f32;
    let mut dz = 0f32;
    if !fire.is_null() {
        let p = entity_pos(ped);
        let fp = fire.byte_add(4);
        let dx = x(rd::<f32>(fp, 0)).fsub_m(rd::<f32>(p, 0));
        let dy = x(rd::<f32>(fp, 4)).fsub_m(rd::<f32>(p, 4));
        dz = x(rd::<f32>(fp, 8)).fsub_m(rd::<f32>(p, 8)).to_f32();
        let v = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(x(dz).fmul_m(dz));
        d2 = v.to_f32();
        let r = k(0x86C918);
        if !v.fcom(x(r).fmul_m(r)).test_parity(5) && !x(dz).fabs().fcom(x(k(0x858CA0))).test_parity(5) {
            let mut ev = [0u32; 0x10];
            let ev_p: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
            event_fire_nearby_ctor(ev_p, fp);
            event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ev_p, 0);
            event_fire_nearby_dtor(ev_p);
        }
    }
    if active.is_null() || v_task_type(active) == 0x202 {
        return;
    }
    let s = tm_get_simplest_active_task(tm());
    if s.is_null() || task_is_go_to_task(s) == 0 || fire.is_null() {
        return;
    }
    let r = k(0x86C91C);
    if x(r).fmul_m(r).fcom(x(d2)).test_any(0x41) {
        return;
    }
    if x(dz).fabs().fcom(x(k(0x858CA0))).test_parity(5) {
        return;
    }
    let mut ev = [0u32; 0x10];
    let ev_p: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    event_walk_into_fire_ctor(ev_p, fire.byte_add(4), rd::<u32>(fire, 0x1C), ms);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ev_p, 0);
    event_walk_into_fire_dtor(ev_p);
}

pub const HOOKS: &[HookReg] = &[HOOK_NEARBY_FIRE_SCANNER_SCAN];
