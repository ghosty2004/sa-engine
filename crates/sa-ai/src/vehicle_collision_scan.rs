//! `CVehiclePotentialCollisionScanner::ScanForVehiclePotentialCollisionEvents` (batch `mvpk-ai-4`): a walking ped about
//! to walk into the vehicle in front of it raises `CEventPotentialWalkIntoVehicle`. Ported from the 1.0 US bytes
//! (gta-reversed as a reading aid; the bytes win). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

use crate::bindings::dtor_4ae3c0 as event_walk_into_vehicle_dtor;
use crate::bindings::geometry_line_of_sight_clear;
use crate::bindings::intel_get_move_state_from_go_to_task;
use crate::bindings::task_is_go_to_task;
use sa_core::bindings::tm_get_simplest_active_task;
use sa_world::bindings::entity_get_col_model;
use sa_world::bindings::event_group_add;
use sa_core::bindings::vector_magnitude_sqr as vector_squared_magnitude;
use crate::bindings::event_walk_into_vehicle_ctor;
use sa_core::bindings::vector_from_multiply;

/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIMER_TIME_MS: usize = 0xB7CB84;

use sa_core::field::gf as k;
use sa_world::world::common::entity_pos;

/// `CVehiclePotentialCollisionScanner::ScanForVehiclePotentialCollisionEvents(const CPed&, CEntity**, int32)` (0x603720,
/// thiscall, `ret 0xC`; only the ped is used). x87 compares. The task timer (+0 start, +4 period, +8 set, +9 reset):
/// unset → (now, 500), set; reset → start = now; not yet `now >= start + period` (unsigned) → nothing; else (now, 500).
///
/// The ped's simplest active task must be a go-to task and its intelligence's nearest vehicle (+0x120) present. With
/// `d = ped - vehicle` (f32), `m1 = FromMultiply(M, box min)`, `m2 = FromMultiply(M, box max)`, the vehicle's "at" row
/// `a` and `s = -1.0 * a` (f32 per component): the ped lies between the two planes when `a · ped + -(a · m2)` (register)
/// and `s · ped + f32(-(s · m1))` are both below 0.5. Then the radius `r` = 5.0 (10.0 for vehicle subtype +0x594 == 6);
/// unless ped +0x474 bit 3, `|d|²` must be below `r²`; `GetIsLineOfSightClear(ped, task +0xC, vehicle, &dist)` (dist
/// zeroed first) blocked with `dist` above 0.5 → a stack `CEventPotentialWalkIntoVehicle(vehicle, GetMoveStateFromGoToTask())`
/// into the ped's event group. The original keeps `r` and `dist` in its own ped argument slot (popped by `ret 0xC`).
#[hook(addr = 0x603720, conv = thiscall, name = "CVehiclePotentialCollisionScanner::ScanForVehiclePotentialCollisionEvents",
       expect = "64 A1 00 00 00 00", route = "L2", hazards = "stack-residue", recs = "grove-walk",
       note = "the event lives on the Rust stack; the ped argument slot holds floats (see doc)")]
pub fn vehicle_scanner_scan_for_potential_collision_events(this: GPtr<u8>, ped: GPtr<u8>, _list: GPtr<u8>, _n: i32) {
    let now = || g::<u32>(TIMER_TIME_MS).read();
    if rd::<u8>(this, 8) == 0 {
        wr::<u32>(this, 0, now());
        wr::<u8>(this, 8, 1);
        wr::<u32>(this, 4, 500);
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
    wr::<u32>(this, 4, 500);
    wr::<u8>(this, 8, 1);
    let task = tm_get_simplest_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    if task.is_null() || task_is_go_to_task(task) == 0 {
        return;
    }
    let veh = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x47C), 0x120);
    if veh.is_null() {
        return;
    }
    let (vp, pp) = (entity_pos(veh), entity_pos(ped));
    let mut d = [
        x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(vp, 0)).to_f32(),
        x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(vp, 4)).to_f32(),
        x(rd::<f32>(pp, 8)).fsub_m(rd::<f32>(vp, 8)).to_f32(),
    ];
    let col = entity_get_col_model(veh);
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let mut m1 = [0f32; 3];
    vector_from_multiply(GPtr::from_raw(m1.as_mut_ptr()).cast(), m, col);
    let mut m2 = [0f32; 3];
    vector_from_multiply(GPtr::from_raw(m2.as_mut_ptr()).cast(), m, col.byte_add(0xC));
    let a = [rd::<f32>(m, 0x20), rd::<f32>(m, 0x24), rd::<f32>(m, 0x28)];
    let neg = k(0x858C1C);
    let big_a = x(a[2]).fmul_m(m2[2]).fadd(x(a[1]).fmul_m(m2[1])).fadd(x(a[0]).fmul_m(m2[0])).fchs();
    let s = [x(a[0]).fmul_m(neg).to_f32(), x(a[1]).fmul_m(neg).to_f32(), x(a[2]).fmul_m(neg).to_f32()];
    let big_b = x(m1[1]).fmul_m(s[1]).fadd(x(m1[2]).fmul_m(s[2])).fadd(x(s[0]).fmul_m(m1[0])).fchs().to_f32();
    let half = x(k(0x86C8E8));
    let pp = entity_pos(ped);
    let first = x(a[2]).fmul_m(rd::<f32>(pp, 8)).fadd(x(a[1]).fmul_m(rd::<f32>(pp, 4))).fadd(x(a[0]).fmul_m(rd::<f32>(pp, 0))).fadd(big_a);
    let inside = !first.fcom(half).test_parity(5) && {
        let pp = entity_pos(ped);
        let second = x(s[1]).fmul_m(rd::<f32>(pp, 4)).fadd(x(s[2]).fmul_m(rd::<f32>(pp, 8))).fadd(x(s[0]).fmul_m(rd::<f32>(pp, 0))).fadd_m(big_b);
        !second.fcom(half).test_parity(5)
    };
    let r = if rd::<u32>(veh, 0x594) == 6 { x(k(0x86C8DC)).fdouble().to_f32() } else { k(0x86C8DC) };
    if !inside {
        return;
    }
    if rd::<u8>(ped, 0x474) & 8 == 0 {
        let m2v = X87::from_f64_exact(vector_squared_magnitude(GPtr::<u8>::cast::<f32>(GPtr::from_raw(d.as_mut_ptr()).cast())));
        if x(r).fmul_m(r).fcom(m2v).test_any(0x41) {
            return;
        }
    }
    let mut dist = 0f32;
    if geometry_line_of_sight_clear(ped, task.byte_add(0xC), veh, GPtr::from_raw(&raw mut dist).cast()) != 0 {
        return;
    }
    if x(dist).fcom(x(k(0x86C8E0))).test_any(0x41) {
        return;
    }
    let ms = intel_get_move_state_from_go_to_task(rd::<GPtr<u8>>(ped, 0x47C)) as u32;
    let mut ev = [0u32; 0x10];
    let ev_p: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    event_walk_into_vehicle_ctor(ev_p, veh, ms);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ev_p, 0);
    event_walk_into_vehicle_dtor(ev_p);
}

pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_SCANNER_SCAN_FOR_POTENTIAL_COLLISION_EVENTS];
