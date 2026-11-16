//! FRONTIER-2 turn 8, lane A (batch `frontier2-a11`): AI frontier virtuals ported from the 1.0 US bytes (gta-reversed
//! read as an aid; where it differs the bytes win, noted per function).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{interesting_events_add, vehicle_is_passenger};
use crate::common::{g, rd, task_get_type, vector_magnitude_sqr, vector_sub, wr, x};
use sa_core::bindings::{tm_get_active_task, vector_difference};
use sa_math::fpu::{gt, hypot_x87, lt};
use sa_world::bindings::ped_say;
use sa_world::world::common::entity_pos;

orig! {
    /// `CTaskComplexMedicTreatInjuredPed::CreateSubTask(int32 type)` (0x658DB0, `ret 4`)
    fn medic_create_sub_task(t: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x658DB0 => crate::frontier_a_subtasks::medic_treat_injured_ped_create_sub_task;
}

/// The ped's `bInVehicle` (+0x46C bit 8).
fn in_vehicle(ped: GPtr<u8>) -> bool {
    rd::<u32>(ped, 0x46C) & 0x100 != 0
}

// ------------------------------------------------------------------------------------------- medic

/// `CTaskComplexMedicTreatInjuredPed::FindNearestAccident(const CVector&)` (0x658CC0): +0x18 = the nearest free
/// accident, returned.
fn medic_find_nearest_accident(this: GPtr<u8>, pos: GPtr<u8>) -> GPtr<u8> {
    let a = crate::gen_bodies::g_task_complex_medic_treat_injured_ped_find_nearest_accident_658cc0(
        this.addr() as u32, pos.addr() as u32);
    GPtr::from_addr(a as usize)
}

/// `CTaskComplexMedicTreatInjuredPed::CalcTargetPosWithOffset(CPed*, CPed* injured)` (0x658D20).
fn medic_calc_target_pos(this: GPtr<u8>, ped: GPtr<u8>, injured: u32) {
    crate::gen_bodies::g_task_complex_medic_treat_injured_ped_calc_target_pos_with_offset_658d20(
        this.addr() as u32, ped.addr() as u32, injured);
}

/// `CTaskComplexMedicTreatInjuredPed::CreateDealWithNextAccidentTask(CPed*, CAccident*)` (0x65A020).
fn medic_deal_with_next(this: GPtr<u8>, ped: GPtr<u8>, acc: u32) -> GPtr<u8> {
    let t = crate::gen_bodies::g_task_complex_medic_treat_injured_ped_create_deal_with_next_accident_task_65a020(
        this.addr() as u32, ped.addr() as u32, acc);
    GPtr::from_addr(t as usize)
}

/// The accident (+0x18) taken: its `m_bIsTreated` (+4) set, the target position computed from its ped (+0), then
/// `CreateSubTask(0x387)` (the shared tail 0x65ABAC / 0x65AC58).
fn medic_go_to_accident(this: GPtr<u8>, ped: GPtr<u8>, acc: GPtr<u8>) -> GPtr<u8> {
    wr::<u8>(acc, 4, 1);
    medic_calc_target_pos(this, ped, rd::<u32>(rd::<GPtr<u8>>(this, 0x18), 0));
    medic_create_sub_task(this, 0x387)
}

/// `(+0x86F440)² > |vehicle (+0xC) - target (+0x1C)|²` (0x65ACBC / 0x65AEB1, `fcompp; test ah, 41h; jne` skips).
fn medic_vehicle_near_target(this: GPtr<u8>) -> bool {
    let mut d = [0f32; 3];
    let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
    vector_difference(dp, entity_pos(rd::<GPtr<u8>>(this, 0xC)), this.byte_add(0x1C));
    let m = vector_magnitude_sqr(dp);
    let r = g::<f32>(0x86F440).read();
    gt(x(r).fmul_m(r), m)
}

/// `CTaskComplexMedicTreatInjuredPed::CreateNextSubTask(CPed*)` (0x65A990, `ret 4`). By the sub-task's type: 0x2BC /
/// 0x2BD (entered) → 0x2C5 in a vehicle, else 0x390; 0x65 (treated) → `FindNearestAccident` and the accident's tail
/// (0x387) when one is found, else as 0xCB; 0xCB → 0x2BD as driver (+0x14), else 0x2BC; 0x387 (arrived) → 0x65 when
/// the 2D distance to the target is at most 0x86F448 and |dz| at most 0x858CA0 (NaN too), else
/// `CreateDealWithNextAccidentTask(ped, +0x18)`; 0x2C0 (left the vehicle) → `CInterestingEvents::Add(12, ped)`, then
/// the driver keeps a current untreated accident (`[+0x18]` read unchecked, as the bytes), else searches (+0x18 = 0
/// first): found → the accident's tail, else 0x2BD (driver) / 0xCB; anything else NULL.
#[hook(addr = 0x65A990, conv = thiscall, name = "CTaskComplexMedicTreatInjuredPed::CreateNextSubTask",
       expect = "83 EC 0C 56 8B F1 8B 4E 08", this = "CTaskComplexMedicTreatInjuredPed",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn medic_treat_injured_ped_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let driver = || rd::<u8>(this, 0x14) != 0;
    let enter = || medic_create_sub_task(this, if driver() { 0x2BD } else { 0x2BC });
    match task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0x2BC | 0x2BD => medic_create_sub_task(this, if in_vehicle(ped) { 0x2C5 } else { 0x390 }),
        0x65 => {
            let acc = medic_find_nearest_accident(this, entity_pos(ped));
            if acc.is_null() { enter() } else { medic_go_to_accident(this, ped, acc) }
        }
        0xCB => enter(),
        0x387 => {
            let p = entity_pos(ped);
            let dx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(this, 0x1C));
            let dy = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(this, 0x20));
            let dz = x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(this, 0x24)).to_f32();
            if gt(hypot_x87(dy, dx), x(g::<f32>(0x86F448).read()))
                || gt(x(dz).fabs(), x(g::<f32>(0x858CA0).read()))
            {
                medic_deal_with_next(this, ped, rd::<u32>(this, 0x18))
            } else {
                medic_create_sub_task(this, 0x65)
            }
        }
        0x2C0 => {
            interesting_events_add(GPtr::from_addr(0xC0B058), 0xC, ped);
            if driver() {
                let acc = rd::<GPtr<u8>>(this, 0x18);
                let injured = rd::<u32>(acc, 0);
                if injured != 0 && rd::<u8>(acc, 4) != 0 {
                    medic_calc_target_pos(this, ped, injured);
                    return medic_create_sub_task(this, 0x387);
                }
            }
            let acc = medic_find_nearest_accident(this, entity_pos(ped));
            if !acc.is_null() {
                medic_go_to_accident(this, ped, acc)
            } else if driver() {
                medic_create_sub_task(this, 0x2BD)
            } else {
                medic_create_sub_task(this, 0xCB)
            }
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexMedicTreatInjuredPed::ControlSubTask(CPed*)` (0x65ABF0, `ret 4`). Keeps the sub-task except: 0x2C7
/// (wandering, in a vehicle) → an accident found: its tail with 0x2C6; 0x2C6 (driving there, in a vehicle) → without a
/// current untreated accident: +0x18 = 0 and 0x2C7, else once the vehicle is within 0x86F440 of the target:
/// `Say(0xE7)` and 0x2C0; 0x387 (walking to it) → without a current accident: `CreateDealWithNextAccidentTask(ped,
/// NULL)`; unless the go-to sub-task's move state (+0xC) is already 4: within sqrt(0x858FE8) of the target, any of the
/// 16 scanned entities (intelligence +0xE0..+0x11C) blocking the target (radius 0x86F444) → the same deal task, else
/// the target recomputed, move state 4 and `GoToPoint(target, 0x86FC84, 0x86FC88, false)`; 0x2C5 (in a vehicle) → the
/// driver: 0x2C7 once the partner (+0x10) is a passenger; the passenger (+0x28 clear): the partner's active task a
/// 0x64 with a current accident → its target copied and, the vehicle within range, `Say(0xE7)` and 0x2C0.
#[hook(addr = 0x65ABF0, conv = thiscall, name = "CTaskComplexMedicTreatInjuredPed::ControlSubTask",
       expect = "83 EC 20 53 55 56 8B F1", this = "CTaskComplexMedicTreatInjuredPed",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn medic_treat_injured_ped_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    let current = |t: GPtr<u8>| {
        let a = rd::<GPtr<u8>>(t, 0x18);
        !a.is_null() && rd::<u32>(a, 0) != 0
    };
    let leave = || {
        ped_say(ped, 0xE7, 0, 1.0, 0, 0, 0);
        medic_create_sub_task(this, 0x2C0)
    };
    match task_get_type(sub) {
        0x2C7 => {
            if !in_vehicle(ped) {
                return sub;
            }
            let acc = medic_find_nearest_accident(this, entity_pos(ped));
            if acc.is_null() {
                return sub;
            }
            wr::<u8>(acc, 4, 1);
            medic_calc_target_pos(this, ped, rd::<u32>(rd::<GPtr<u8>>(this, 0x18), 0));
            medic_create_sub_task(this, 0x2C6)
        }
        0x2C6 => {
            if !in_vehicle(ped) {
                return sub;
            }
            if !current(this) {
                wr::<u32>(this, 0x18, 0);
                return medic_create_sub_task(this, 0x2C7);
            }
            if medic_vehicle_near_target(this) { leave() } else { sub }
        }
        0x387 => {
            if !current(this) {
                return medic_deal_with_next(this, ped, 0);
            }
            if rd::<u32>(sub, 0xC) == 4 {
                return sub;
            }
            let target = this.byte_add(0x1C);
            let mut d = [0f32; 3];
            let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
            let v = vector_sub(dp, target, entity_pos(ped));
            if !lt(vector_magnitude_sqr(v), x(g::<f32>(0x858FE8).read())) {
                return sub;
            }
            let intel = rd::<GPtr<u8>>(ped, 0x47C);
            for off in (0xE0..0x120).step_by(4) {
                let e = rd::<GPtr<u8>>(intel, off);
                if !e.is_null() && crate::ped_geometry::is_entity_blocking_target(e, target, g::<f32>(0x86F444).read()) != 0 {
                    return medic_deal_with_next(this, ped, 0);
                }
            }
            medic_calc_target_pos(this, ped, rd::<u32>(rd::<GPtr<u8>>(this, 0x18), 0));
            wr::<u32>(sub, 0xC, 4);
            crate::walk::gtpss_go_to_point(sub, target, g::<u32>(0x86FC84).read(), g::<u32>(0x86FC88).read(), 0);
            sub
        }
        0x2C5 => {
            if !in_vehicle(ped) {
                return sub;
            }
            if rd::<u8>(this, 0x14) != 0 {
                let p = vehicle_is_passenger(rd::<GPtr<u8>>(this, 0xC), rd::<GPtr<u8>>(this, 0x10));
                return if p != 0 { medic_create_sub_task(this, 0x2C7) } else { sub };
            }
            let partner = rd::<GPtr<u8>>(this, 0x10);
            if rd::<u8>(this, 0x28) != 0 || partner.is_null() {
                return sub;
            }
            let t = tm_get_active_task(rd::<GPtr<u8>>(partner, 0x47C).byte_add(4));
            if t.is_null() || task_get_type(t) != 0x64 || !current(t) {
                return sub;
            }
            for i in 0..3 {
                wr::<u32>(this, 0x1C + i * 4, rd::<u32>(t, 0x1C + i * 4));
            }
            if medic_vehicle_near_target(this) { leave() } else { sub }
        }
        _ => sub,
    }
}

/// The batch's hooks.
pub const HOOKS: &[HookReg] = &[
    HOOK_MEDIC_TREAT_INJURED_PED_CREATE_NEXT_SUB_TASK,
    HOOK_MEDIC_TREAT_INJURED_PED_CONTROL_SUB_TASK,
];
