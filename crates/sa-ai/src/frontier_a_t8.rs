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
use crate::bindings::{pad_jump_just_down, use_gun_control_gun, use_gun_control_gun_move, use_gun_ctor};
use sa_core::bindings::{
    entity_clean_up_old_reference, entity_register_reference, intel_get_task_use_gun, ped_is_alive, vector_normalise,
};
use sa_world::bindings::{ctor_48df30, run_anim_ctor, sequence_add_task, sequence_ctor, stand_still_ctor};
use crate::bindings::{can_ped_target_ped, intel_can_see_entity_with_lights, investigate_disturbance_ctor};
use sa_math::fpu::{fcom_ne, fcom_parity, st0};
use crate::bindings::{run_anim_ctor2, run_timed_anim_ctor2};
use sa_core::bindings::general_random_int_in_range;
use sa_world::bindings::{
    get_vehicle_appearance, is_sunbather, pause_ctor, ped_set_ped_state, switch_vehicle_to_real_physics,
};
use crate::bindings::{can_sunbathe, event_group_add, history_get_current_event, task_timer_is_out_of_time};
use sa_math::fpu::{ftol32, le_or_nan};
use crate::bindings::{stand_still_dtor, stand_still_process_ped};
use sa_world::bindings::{enter_car_as_passenger_ctor, ped_groups_get_peds_group};
use sa_math::x87::X87;
use sa_anim::bindings::assoc_set_finish_callback;
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::{anim_blend_animation, pad_get_target, physical_apply_move_force_5429f0};
use crate::bindings::{be_hit_ctor, car_drive_timed_ctor, fall_and_get_up_ctor, task_simple_ctor};
use sa_core::bindings::{get_pad, vector2d_normalise};
use sa_math::vector::CVector;
use crate::bindings::{clean_up_this_vehicle, cleanup_remove_entity_from_list};
use sa_core::bindings::{eh_vector_ctor, eh_vector_dtor};
use sa_world::bindings::{
    camera_is_sphere_visible, create_car_for_script, find_node_closest_to_coors, find_node_coors_for_script,
    is_position_clear_for_ped, set_ped_in_as_driver_ctor, set_ped_in_as_driver_dtor, set_ped_in_as_driver_process_ped,
};
use sa_core::crt::rand_at;
use sa_input::bindings::pad_sprint_just_down;
use crate::common::{task_create_next_sub_task, task_make_abortable, task_new, xi};
use sa_anim::bindings::anim_add_block_ref;
use sa_core::bindings::{
    general_get_radian_angle_between_points, matrix_mul_vec, tm_get_task_secondary, tm_set_task_secondary,
};
use sa_stream::bindings::streaming_request_model;
use sa_world::bindings::{
    achieve_heading_ctor, entity_get_col_model, get_pad_from_player, go_to_point_and_stand_still_ctor, hold_entity_ctor,
    pad_get_ped_walk_left_right, pad_get_ped_walk_up_down, update_rw_matrix,
};
use sa_world::world::entity::{get_matrix, set_heading};

/// `CTimer::m_snTimeInMilliseconds`.
const TIME_MS: usize = 0xB7CB84;

orig! {
    /// `CTaskComplexMedicTreatInjuredPed::CreateSubTask(int32 type)` (0x658DB0, `ret 4`)
    fn medic_create_sub_task(t: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x658DB0 => crate::frontier_a_subtasks::medic_treat_injured_ped_create_sub_task;
}

use sa_world::world::cworld::in_vehicle;

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

// ------------------------------------------------------------------------------------------- go pick up entity

orig! {
    /// `CTaskSimplePickUpEntity::CTaskSimplePickUpEntity(CEntity*, const CVector*, uint8 bone, uint8 flags, AnimationId,
    /// AssocGroupId, float)` (0x6917B0, `ret 0x1C`)
    fn pick_up_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, pos: GPtr<u8>, bone: u32, flags: u32, anim: u32, grp: u32, f: u32) -> GPtr<u8> = thiscall 0x6917B0 => unported;
    /// `CEntity::SetIsStatic(bool)` (vtable +0x10)
    fn v_set_is_static(e: GPtr<u8>, v: u32) = thiscall vslot 0x10 => unported;
}

/// `CTaskComplexGoPickUpEntity::CreateNextSubTask(CPed*)` (0x691AE0, `ret 4`; vt_870b98). +0xC the entity (none:
/// NULL), +0x10 the hold offset, +0x1C the pick-up position, +0x28 a time, +0x2C the anim group. By the sub-task's
/// type: 0x387 (arrived) → `CTaskSimpleAchieveHeading(atan2(-dx, dy), 1.0, 0.001)` towards the entity; 0x386 (turned)
/// → group 0x51: 0x53 with the anim speed at 0x870A88 when the entity's bounding-box min z plus its z is above the
/// ped's, else 0x52 (0x870A84) when the ped's z is below that plus 0x866CAC, else 0x51 (0x870A80); then
/// `CTaskSimplePickUpEntity(entity, &+0x10, 6, 1, 0x13D, +0x2C, speed)` with the pick-up position copied to its +0x40
/// (written through a NULL task too, as the bytes); 0x134 (picked up) → +0x28 = now and, without a secondary task 4,
/// `SetTaskSecondary(CTaskSimpleHoldEntity(entity, &+0x10, 6, 1, 0x13E, 0x51, false), 4)`, NULL; others NULL.
#[hook(addr = 0x691AE0, conv = thiscall, name = "vt_870b98::CreateNextSubTask",
       expect = "64 A1 00 00 00 00 6A FF 68 21 73 84 00",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the 0x387 case stores dy over its own ped argument slot")]
pub fn go_pick_up_entity_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let ent = rd::<GPtr<u8>>(this, 0xC);
    if ent.is_null() {
        return GPtr::NULL;
    }
    match task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0x387 => {
            let pp = entity_pos(ped);
            let ep = entity_pos(ent);
            let dy = x(rd::<f32>(ep, 4)).fsub_m(rd::<f32>(pp, 4)).to_f32();
            let dx = x(rd::<f32>(ep, 0)).fsub_m(rd::<f32>(pp, 0)).to_f32();
            let m = task_new(0x18);
            if m.is_null() {
                return GPtr::NULL;
            }
            let h = x(dx).fchs().fpatan(x(dy)).to_f32();
            achieve_heading_ctor(m, h, 0x3F800000, 0x3A83126F)
        }
        0x386 => {
            let mut speed = g::<u32>(0x870A80).read();
            if rd::<i32>(this, 0x2C) == 0x51 {
                let ep = entity_pos(ent);
                let col = entity_get_col_model(ent);
                let top = x(rd::<f32>(col, 8)).fadd_m(rd::<f32>(ep, 8));
                let pz = rd::<f32>(entity_pos(ped), 8);
                if gt(top, x(pz)) {
                    wr::<u32>(this, 0x2C, 0x53);
                    speed = g::<u32>(0x870A88).read();
                } else if lt(x(pz), x(g::<f32>(0x866CAC).read()).fadd(top)) {
                    wr::<u32>(this, 0x2C, 0x52);
                    speed = g::<u32>(0x870A84).read();
                }
            }
            let m = task_new(0x4C);
            let t = if m.is_null() {
                GPtr::NULL
            } else {
                pick_up_entity_ctor(m, ent, this.byte_add(0x10), 6, 1, 0x13D, rd::<u32>(this, 0x2C), speed)
            };
            for i in 0..3 {
                wr::<u32>(t, 0x40 + i * 4, rd::<u32>(this, 0x1C + i * 4));
            }
            t
        }
        0x134 => {
            wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
            let tm = rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
            if tm_get_task_secondary(tm, 4).is_null() {
                let m = task_new(0x3C);
                let t = if m.is_null() {
                    GPtr::NULL
                } else {
                    hold_entity_ctor(m, rd::<GPtr<u8>>(this, 0xC), this.byte_add(0x10), 6, 1, 0x13E, 0x51, 0)
                };
                tm_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4);
            }
            GPtr::NULL
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexGoPickUpEntity::ControlSubTask(CPed*)` (0x691D50, `ret 4`; vt_870b98). With an anim group (+0x2C) and
/// no block reference (+0x30): the group's block loaded → `AddAnimBlockRef`, +0x30 = 1, else `RequestModel(block +
/// 0x63E7, 8)`. No sub-task: NULL. Picking up (0x134) with neither its +0x38 nor +0x34 set, more than 0x8D2FFC ms
/// after +0x28: `MakeAbortable(ped, 1, NULL)` and NULL. Going to the point (0x387): more than 0x8D2FF8 ms after +0x28 →
/// abort as above; a player with a pad whose jump or sprint was just pressed → abort; a stick pushed past 0x858F34
/// (`|stick| * 0x858B88`) with its camera-relative heading (`GetRadianAngleBetweenPoints(0, 0, -lr, ud) -
/// 0xB6F178`, brought within pi of the heading to the point) more than 0x859AB0 above that heading → abort. Else the
/// sub-task.
#[hook(addr = 0x691D50, conv = thiscall, name = "vt_870b98::ControlSubTask", expect = "83 EC 08 53 55 56 8B F1",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the original stores the stick's up-down value and the pad heading over its own ped argument slot")]
pub fn go_pick_up_entity_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let grp = rd::<u32>(this, 0x2C);
    if grp != 0 && rd::<u8>(this, 0x30) == 0 {
        let b = crate::fight_combo_style::group_block(grp);
        let i = crate::fight_combo_style::block_index(b);
        if rd::<u8>(b, 0x10) != 0 {
            anim_add_block_ref(i);
            wr::<u8>(this, 0x30, 1);
        } else {
            streaming_request_model(i.wrapping_add(0x63E7), 8);
        }
    }
    let sub = rd::<GPtr<u8>>(this, 8);
    if sub.is_null() {
        return sub;
    }
    let now = || g::<u32>(TIME_MS).read();
    let abort = || {
        task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, GPtr::NULL);
        GPtr::NULL
    };
    match task_get_type(sub) {
        0x134 => {
            if rd::<u32>(sub, 0x38) == 0 && rd::<u8>(sub, 0x34) == 0
                && now() > rd::<u32>(this, 0x28).wrapping_add(g::<u32>(0x8D2FFC).read())
            {
                return abort();
            }
            sub
        }
        0x387 => {
            if now() > rd::<u32>(this, 0x28).wrapping_add(g::<u32>(0x8D2FF8).read()) {
                return abort();
            }
            if rd::<u32>(ped, 0x480) == 0 {
                return sub;
            }
            let pad = get_pad_from_player(ped);
            if pad.is_null() {
                return sub;
            }
            let ud = f32::from(pad_get_ped_walk_up_down(pad));
            let lr = xi(i32::from(pad_get_ped_walk_left_right(pad)));
            let mag = x(ud).fmul_m(ud).fadd(lr.fmul(lr)).fsqrt().fmul_m(g::<f32>(0x858B88).read()).to_f32();
            let pos = entity_pos(ped);
            let task_ang = x(rd::<f32>(sub, 0x10)).fsub_m(rd::<f32>(pos, 0)).fchs()
                .fpatan(x(rd::<f32>(sub, 0x14)).fsub_m(rd::<f32>(pos, 4))).to_f32();
            let mut ang = general_get_radian_angle_between_points(0.0, 0.0, lr.fchs().to_f32(), ud)
                .fsub_m(g::<f32>(0xB6F178).read()).to_f32();
            let pi = g::<f32>(0x858CB8).read();
            let two_pi = g::<f32>(0x858CBC).read();
            if lt(x(task_ang).fadd_m(pi), x(ang)) {
                ang = x(ang).fsub_m(two_pi).to_f32();
            } else if gt(x(task_ang).fsub_m(pi), x(ang)) {
                ang = x(ang).fadd_m(two_pi).to_f32();
            }
            if pad_jump_just_down(pad) != 0 || pad_sprint_just_down(pad) != 0 {
                return abort();
            }
            if !gt(x(mag), x(g::<f32>(0x858F34).read())) {
                return sub;
            }
            if gt(x(ang).fsub_m(task_ang), x(g::<f32>(0x859AB0).read())) { abort() } else { sub }
        }
        _ => sub,
    }
}

/// `CTaskComplexGoPickUpEntity::CreateFirstSubTask(CPed*)` (0x693610, `ret 4`; vt_870b98). No entity (+0xC): NULL.
/// +0x28 = now; `d = ped - entity` (f32); +0x10 = 0x870A74, +0x14 = -(box min y) - 0x870A78, +0x18 = -(box min z) of
/// the entity's model collision. An entity twice as long in x as in y (`2 max.y < max.x`) gets the heading
/// `atan2(-dx, dy)`, twice as long in y `atan2(-dx, dy) - 0x858FE4`, otherwise, its matrix's up z below 0x858C20, its
/// own heading again (`SetHeading`). An object (type 4) loses its move and turn speeds and becomes static; its RW
/// matrix is updated. In the entity's frame (`r = d · right`, `f = d · forward`): `r > |f|` → +0x1C = (0x870A7C + max
/// x, 0), `-|f| > r` → (min x - 0x870A7C, 0), else (0, f > 0 ? 0x870A7C + max y : min y - 0x870A7C); +0x24 = min z +
/// 1. Returns `CTaskComplexGoToPointAndStandStill(4, matrix * +0x1C, 0.2, 0, false, true)`.
#[hook(addr = 0x693610, conv = thiscall, name = "vt_870b98::CreateFirstSubTask",
       expect = "64 A1 00 00 00 00 6A FF 68 BB 74 84 00",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the original stores dy and two angles over its own ped argument slot")]
pub fn go_pick_up_entity_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let ent = rd::<GPtr<u8>>(this, 0xC);
    if ent.is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
    let ep = entity_pos(ent);
    let pp = entity_pos(ped);
    let dz = x(rd::<f32>(pp, 8)).fsub_m(rd::<f32>(ep, 8)).to_f32();
    let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(ep, 4)).to_f32();
    let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(ep, 0)).to_f32();
    let mi = g::<GPtr<u8>>(0xA9B0C8 + (i32::from(rd::<i16>(ent, 0x22)) * 4) as usize).read();
    let col = rd::<GPtr<u8>>(mi, 0x14);
    wr::<u32>(this, 0x10, g::<u32>(0x870A74).read());
    wr::<f32>(this, 0x14, x(rd::<f32>(col, 4)).fchs().fsub_m(g::<f32>(0x870A78).read()).to_f32());
    wr::<f32>(this, 0x18, x(rd::<f32>(col, 8)).fchs().to_f32());
    let (min_x, min_y, min_z) = (rd::<f32>(col, 0), rd::<f32>(col, 4), rd::<f32>(col, 8));
    let (max_x, max_y) = (rd::<f32>(col, 0xC), rd::<f32>(col, 0x10));
    let to_ped = || x(dx).fchs().fpatan(x(dy));
    if lt(x(max_y).fadd(x(max_y)), x(max_x)) {
        set_heading(ent, to_ped().to_f32());
    } else if lt(x(max_x).fadd(x(max_x)), x(max_y)) {
        set_heading(ent, to_ped().fsub_m(g::<f32>(0x858FE4).read()).to_f32());
    } else {
        let m = get_matrix(ent);
        if lt(x(rd::<f32>(m, 0x28)), x(g::<f32>(0x858C20).read())) {
            let e = rd::<GPtr<u8>>(this, 0xC);
            let em = rd::<GPtr<u8>>(e, 0x14);
            let h = if em.is_null() {
                rd::<f32>(e, 0x10)
            } else {
                x(rd::<f32>(em, 0x10)).fchs().fpatan(x(rd::<f32>(em, 0x14))).to_f32()
            };
            set_heading(e, h);
        }
    }
    if rd::<u8>(rd::<GPtr<u8>>(this, 0xC), 0x36) & 7 == 4 {
        for off in [0x44, 0x48, 0x4C, 0x50, 0x54, 0x58] {
            wr::<u32>(rd::<GPtr<u8>>(this, 0xC), off, 0);
        }
        v_set_is_static(rd::<GPtr<u8>>(this, 0xC), 1);
    }
    update_rw_matrix(rd::<GPtr<u8>>(this, 0xC));
    let m = get_matrix(rd::<GPtr<u8>>(this, 0xC));
    let r = x(dz).fmul_m(rd::<f32>(m, 8)).fadd(x(dy).fmul_m(rd::<f32>(m, 4))).fadd(x(dx).fmul_m(rd::<f32>(m, 0))).to_f32();
    let m = get_matrix(rd::<GPtr<u8>>(this, 0xC));
    let f = x(dz).fmul_m(rd::<f32>(m, 0x18)).fadd(x(dy).fmul_m(rd::<f32>(m, 0x14))).fadd(x(dx).fmul_m(rd::<f32>(m, 0x10)));
    let off = g::<f32>(0x870A7C).read();
    if gt(x(r), f.fabs()) {
        wr::<f32>(this, 0x1C, x(off).fadd_m(max_x).to_f32());
        wr::<u32>(this, 0x20, 0);
    } else if gt(f.fabs().fchs(), x(r)) {
        wr::<f32>(this, 0x1C, x(min_x).fsub_m(off).to_f32());
        wr::<u32>(this, 0x20, 0);
    } else {
        wr::<u32>(this, 0x1C, 0);
        let v = if gt(f, x(g::<f32>(0x858B50).read())) { x(off).fadd_m(max_y) } else { x(min_y).fsub_m(off) };
        wr::<f32>(this, 0x20, v.to_f32());
    }
    wr::<f32>(this, 0x24, x(min_z).fadd_m(g::<f32>(0x858624).read()).to_f32());
    let m = get_matrix(rd::<GPtr<u8>>(this, 0xC));
    let mut p = [0f32; 3];
    let pp: GPtr<u8> = GPtr::from_raw(p.as_mut_ptr()).cast();
    matrix_mul_vec(pp, m, this.byte_add(0x1C));
    let t = task_new(0x28);
    if t.is_null() {
        return GPtr::NULL;
    }
    go_to_point_and_stand_still_ctor(t, 4, pp.cast(), 0.2, 0.0, 0, 1)
}

// ------------------------------------------------------------------------------------------- go to point aiming

orig! {
    /// `CTask::GetTaskType()` of a complex task, read through its own vtable (+0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
}

/// `CTaskComplexGoToPointAiming::ControlSubTask(CPed*)` (0x6689E0, `ret 4`). +0x10 the aim entity, +0x14 the aim point,
/// +0x20 the move point, +0x2C the target radius, +0x34 bit 0 a new target. The current weapon's info is fetched first.
/// A new target: `CreateFirstSubTask(ped)`. Going to the point (0x387): `GoToPoint(+0x20, 0x86FC84, 0x86FC88, false)`;
/// a weapon aimed with the arm (info +0x18 bit 1): no attack secondary task → `SetTaskSecondary(CTaskSimpleUseGun(
/// +0x10, +0x14, 1, 1, false), 0)`; else with a gun task the secondary's `ControlGun(ped, +0x10, 2)` for a shooting
/// task (0x385) one time in 64 (`rand() & 0x3F`), else `ControlGun(.., 1)`; without one the secondary's
/// `MakeAbortable(ped, 1, NULL)`. Gun control (0x3FC) with a gun task: the move point relative to the ped in its
/// matrix frame (`r = d · right`, `f = d · forward * 0x858C1C`), normalised when longer than the radius (else zero),
/// to the attack secondary's `ControlGunMove`. Returns the sub-task.
#[hook(addr = 0x6689E0, conv = thiscall, name = "CTaskComplexGoToPointAiming::ControlSubTask",
       expect = "6A FF 64 A1 00 00 00 00 68 4B 4F 84 00",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn go_to_point_aiming_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    let wi = crate::kill_ped_on_foot_armed::current_weapon_info(ped);
    if rd::<u8>(this, 0x34) & 1 != 0 {
        return v_create_first_sub_task(this, ped);
    }
    let intel = || rd::<GPtr<u8>>(ped, 0x47C);
    let attack = || tm_get_task_secondary(intel().byte_add(4), 0);
    match task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0x3FC => {
            if intel_get_task_use_gun(intel()).is_null() {
                return sub;
            }
            let p = entity_pos(ped);
            let m = rd::<GPtr<u8>>(ped, 0x14);
            let dx = x(rd::<f32>(this, 0x20)).fsub_m(rd::<f32>(p, 0));
            let dy = x(rd::<f32>(this, 0x24)).fsub_m(rd::<f32>(p, 4));
            let dz = x(rd::<f32>(this, 0x28)).fsub_m(rd::<f32>(p, 8));
            let r = dz.fmul_m(rd::<f32>(m, 8)).fadd(dy.fmul_m(rd::<f32>(m, 4))).fadd(dx.fmul_m(rd::<f32>(m, 0))).to_f32();
            let f = dz.fmul_m(rd::<f32>(m, 0x18)).fadd(dy.fmul_m(rd::<f32>(m, 0x14))).fadd(dx.fmul_m(rd::<f32>(m, 0x10)))
                .fmul_m(g::<f32>(0x858C1C).read());
            let len2 = f.fmul(f).fadd(x(r).fmul_m(r));
            let rad = x(rd::<f32>(this, 0x2C));
            let mut v = [0f32; 2];
            if gt(len2, rad.fmul(rad)) {
                let inv = len2.fsqrt().fdivr_m(g::<f32>(0x858624).read());
                v = [x(r).fmul(inv).to_f32(), inv.fmul(f).to_f32()];
            }
            use_gun_control_gun_move(attack(), GPtr::from_raw(v.as_mut_ptr()).cast());
        }
        0x387 => {
            crate::walk::gtpss_go_to_point(sub, this.byte_add(0x20), g::<u32>(0x86FC84).read(),
                g::<u32>(0x86FC88).read(), 0);
            if (rd::<u32>(wi, 0x18) >> 1) & 1 == 0 {
                return sub;
            }
            if attack().is_null() {
                let m = task_new(0x3C);
                let t = if m.is_null() {
                    GPtr::NULL
                } else {
                    use_gun_ctor(m, rd::<GPtr<u8>>(this, 0x10), rd::<u32>(this, 0x14), rd::<u32>(this, 0x18),
                        rd::<u32>(this, 0x1C), 1, 1, 0)
                };
                tm_set_task_secondary(intel().byte_add(4), t, 0);
            } else if !intel_get_task_use_gun(intel()).is_null() {
                let cmd = if v_task_type(this) == 0x385 && rand_at(0x668BF5) & 0x3F == 0 { 2 } else { 1 };
                use_gun_control_gun(attack(), ped, rd::<GPtr<u8>>(this, 0x10), cmd);
            } else {
                task_make_abortable(attack(), ped, 1, GPtr::NULL);
            }
        }
        _ => {}
    }
    sub
}

// ------------------------------------------------------------------------------------------- follow patrol route

/// `CTaskComplexFollowPatrolRoute::CreateFirstSubTask(CPed*)` (0x670E20, catalogue `vt_870114::CreateFirstSubTask`,
/// `ret 4`). +0x20 bit 0 cleared. No points: `CreateSubTask(0xC8)`. Otherwise the point to start from (+0x12): over
/// every leg `i → (i + 1) % n` the ped's projection `t` on the leg's unit direction, when `0 < t < |leg|`, gives the
/// foot point; the leg end becomes the candidate when the squared distance to the foot point is below the square of
/// the best value so far (FLT_MAX first), which then takes that squared distance (the bytes compare a squared distance
/// with a squared one). No leg: the point whose squared distance is below the square of the best (0x863A3C first),
/// kept the same way. Then +0x20 bit 1 set → cleared and `CreateSubTask(0x38A)`, else `CreateSubTask(next type)`.
#[hook(addr = 0x670E20, conv = thiscall, name = "vt_870114::CreateFirstSubTask", expect = "83 EC 38 53 8B D9 8B 4B 20",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the original stores the projections and distances over its own ped argument slot")]
pub fn follow_patrol_route_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    use crate::frontier_a_t6::{follow_patrol_route_create_sub_task as sub, follow_patrol_route_next_type, route_pos};
    wr::<u32>(this, 0x20, rd::<u32>(this, 0x20) & !1);
    let route = rd::<GPtr<u8>>(this, 0x1C);
    let n = rd::<i32>(route, 0);
    if n == 0 {
        return sub(this, 0xC8);
    }
    let pp = entity_pos(ped);
    let p = [rd::<f32>(pp, 0), rd::<f32>(pp, 4), rd::<f32>(pp, 8)];
    let mut idx: i32 = -1;
    let mut best = f32::from_bits(0x7F7F_FFFF);
    for i in 0..n {
        let next = (i + 1) % n;
        let a = route_pos(rd::<GPtr<u8>>(this, 0x1C), i % n);
        let b = route_pos(rd::<GPtr<u8>>(this, 0x1C), next);
        let leg = |k: usize| x(rd::<f32>(b, k as isize * 4)).fsub_m(rd::<f32>(a, k as isize * 4));
        let mut dir = [leg(0).to_f32(), leg(1).to_f32(), leg(2).to_f32()];
        let len2 = leg(2).fmul_m(dir[2]).fadd(x(dir[0]).fmul_m(dir[0])).fadd(x(dir[1]).fmul_m(dir[1])).to_f32();
        vector_normalise(GPtr::from_raw(dir.as_mut_ptr()).cast());
        let d = |k: usize| x(p[k]).fsub_m(rd::<f32>(a, k as isize * 4));
        let tx = d(2).fmul_m(dir[2]).fadd(d(0).fmul_m(dir[0])).fadd(d(1).fmul_m(dir[1]));
        let t = tx.to_f32();
        if !gt(tx, x(g::<f32>(0x858B50).read())) || !gt(x(len2).fsqrt(), x(t)) {
            continue;
        }
        let fx = x(dir[0]).fmul_m(t).fadd_m(rd::<f32>(a, 0)).to_f32();
        let fy = x(dir[1]).fmul_m(t).fadd_m(rd::<f32>(a, 4));
        let fz = x(x(dir[2]).fmul_m(t).to_f32()).fadd_m(rd::<f32>(a, 8));
        let ex = x(p[0]).fsub_m(fx).to_f32();
        let ey = x(p[1]).fsub(fy).to_f32();
        let ez = x(p[2]).fsub(fz);
        let d2 = ez.fmul(ez).fadd(x(ey).fmul_m(ey)).fadd(x(ex).fmul_m(ex));
        if lt(d2, x(best).fmul_m(best)) {
            best = d2.to_f32();
            idx = next;
        }
    }
    if idx == -1 {
        let mut min = x(g::<f32>(0x863A3C).read());
        for j in 0..n {
            let q = route_pos(rd::<GPtr<u8>>(this, 0x1C), j);
            let dx = x(p[0]).fsub_m(rd::<f32>(q, 0));
            let dy = x(p[1]).fsub_m(rd::<f32>(q, 4));
            let dz = x(p[2]).fsub_m(rd::<f32>(q, 8));
            let s = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
            if lt(s, min.fmul(min)) {
                idx = j;
                min = x(s.to_f32());
            }
        }
    }
    wr::<u16>(this, 0x12, idx as u16);
    let f = rd::<u32>(this, 0x20);
    wr::<u32>(this, 0x20, f & !2);
    if f & 2 != 0 {
        sub(this, 0x38A)
    } else {
        sub(this, follow_patrol_route_next_type(this))
    }
}

// ------------------------------------------------------------------------------------------- seek cover

/// `CTaskComplexSeekCoverUntilTargetDead::ControlSubTask(CPed*)` (0x62E8B0, `ret 4`). A dead or missing target (+0x10)
/// is replaced by `GetTargetPed()` (references moved); still none and the sub-task aborts (`MakeAbortable(ped, 1,
/// NULL)`) → `CTaskSimpleStandStill(-1, false, false, 8.0)`. A dead or missing cover ped (+0x14) is replaced by
/// `GetTargetPed()` as well (the bytes call the target getter here too); none → the stand-still on an abortable
/// sub-task, else the sub-task; a new one → on an abortable sub-task the seek-cover sequence (as `CreateFirstSubTask`),
/// else the sub-task. A living cover ped keeps the sub-task.
#[hook(addr = 0x62E8B0, conv = thiscall, name = "CTaskComplexSeekCoverUntilTargetDead::ControlSubTask",
       expect = "64 A1 00 00 00 00 6A FF 68 FA 01 84 00",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn seek_cover_until_target_dead_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    use crate::group_scan_loops::seek_cover_until_target_dead_get_target_ped as get_target;
    let alive = |e: GPtr<u8>| !e.is_null() && ped_is_alive(e) as u8 != 0;
    let replace = |slot: GPtr<u8>| {
        let old = rd::<GPtr<u8>>(slot, 0);
        if !old.is_null() {
            entity_clean_up_old_reference(old, slot);
        }
        let new = get_target(this);
        wr::<GPtr<u8>>(slot, 0, new);
        if !new.is_null() {
            entity_register_reference(new, slot);
        }
    };
    let abortable = || task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, GPtr::NULL) != 0;
    let stand = || crate::common::new_task(0x20, |m| stand_still_ctor(m, -1, 0, 0, 8.0));
    let target_slot = this.byte_add(0x10);
    if !alive(rd::<GPtr<u8>>(target_slot, 0)) {
        replace(target_slot);
        if rd::<u32>(target_slot, 0) == 0 && abortable() {
            return stand();
        }
    }
    let cover_slot = this.byte_add(0x14);
    if alive(rd::<GPtr<u8>>(cover_slot, 0)) {
        return rd::<GPtr<u8>>(this, 8);
    }
    replace(cover_slot);
    if rd::<u32>(cover_slot, 0) == 0 {
        return if abortable() { stand() } else { rd::<GPtr<u8>>(this, 8) };
    }
    if abortable() {
        crate::frontier_a_t5::seek_cover_sequence(this)
    } else {
        rd::<GPtr<u8>>(this, 8)
    }
}

// ------------------------------------------------------------------------------------------- kill ped on foot stealth

/// `CTaskComplexKillPedOnFootStealth::ControlSubTask(CPed*)` (0x62B840, `ret 4`). No target (+0x10): the base
/// `CTaskComplexKillPedOnFoot::ControlSubTask`. +0x44 the time the target was lost (-1 while seen), +0x38 its last
/// seen position, +0x40 a search range. Tracking: `CanPedTargetPed(ped, target, true)` while lost (-1 → +0x44 = now if
/// not seen, else the position stored); while seen: seen again with `CanSeeEntityWithLights(target, true) > 0` and the
/// unit direction to it dotted with the ped's forward above 0xC0B034 → +0x44 = -1 and the position stored. Lost for
/// more than 5000 ms: `Say(0xCE)`, then with +0x40 above 0x86DB14 a non-investigating (0x3A7) abortable sub-task →
/// `CTaskComplexInvestigateDisturbance(&+0x38, target)`; with +0x40 at most 0x86DB14 (ordered) a sub-task other than
/// 0xF4 that aborts → a sequence of `CTaskSimpleStandStill(1000)`, `CTaskSimpleRunAnim(ped anim group, 4, 4.0)` and
/// the 0x48DF30 task. Seen (-1) with an investigating or 0xF4 sub-task that aborts: the base `CreateFirstSubTask`.
/// Otherwise the sub-task.
#[hook(addr = 0x62B840, conv = thiscall, name = "CTaskComplexKillPedOnFootStealth::ControlSubTask",
       expect = "64 A1 00 00 00 00 6A FF 68 E7 00 84 00",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the original stores the facing dot product over its own ped argument slot")]
pub fn kill_ped_on_foot_stealth_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    use crate::kill_ped_on_foot::{kill_ped_on_foot_control_sub_task, kill_ped_on_foot_create_first_sub_task};
    let target = rd::<GPtr<u8>>(this, 0x10);
    if target.is_null() {
        return kill_ped_on_foot_control_sub_task(this, ped);
    }
    let seen = can_ped_target_ped(ped, target, 1) != 0;
    let store_pos = || {
        wr::<u32>(this, 0x44, u32::MAX);
        let p = entity_pos(rd::<GPtr<u8>>(this, 0x10));
        for i in 0..3 {
            wr::<u32>(this, 0x38 + i * 4, rd::<u32>(p, i * 4));
        }
    };
    if rd::<i32>(this, 0x44) == -1 {
        if seen {
            store_pos();
        } else {
            wr::<u32>(this, 0x44, g::<u32>(TIME_MS).read());
        }
    } else if seen {
        let tp = entity_pos(rd::<GPtr<u8>>(this, 0x10));
        let pp = entity_pos(ped);
        let mut d = [0f32; 3];
        for (k, v) in d.iter_mut().enumerate() {
            *v = x(rd::<f32>(tp, k as isize * 4)).fsub_m(rd::<f32>(pp, k as isize * 4)).to_f32();
        }
        vector_normalise(GPtr::from_raw(d.as_mut_ptr()).cast());
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let dot = x(d[2]).fmul_m(rd::<f32>(m, 0x18)).fadd(x(d[1]).fmul_m(rd::<f32>(m, 0x14)))
            .fadd(x(d[0]).fmul_m(rd::<f32>(m, 0x10))).to_f32();
        let v = intel_can_see_entity_with_lights(rd::<GPtr<u8>>(ped, 0x47C), rd::<GPtr<u8>>(this, 0x10), 1);
        if gt(st0(v), x(g::<f32>(0x858B50).read())) && gt(x(dot), x(g::<f32>(0xC0B034).read())) {
            store_pos();
        }
    }
    let sub = || rd::<GPtr<u8>>(this, 8);
    let abortable = || task_make_abortable(sub(), ped, 1, GPtr::NULL) != 0;
    let lost = rd::<i32>(this, 0x44);
    if lost == -1 {
        if !sub().is_null() && matches!(task_get_type(sub()), 0x3A7 | 0xF4) && abortable() {
            return kill_ped_on_foot_create_first_sub_task(this, ped);
        }
        return sub();
    }
    if (g::<u32>(TIME_MS).read().wrapping_sub(lost as u32) as i32) <= 5000 {
        return sub();
    }
    ped_say(ped, 0xCE, 0, 1.0, 0, 0, 0);
    let range = x(rd::<f32>(this, 0x40));
    let lim = x(g::<f32>(0x86DB14).read());
    if gt(range, lim) && !sub().is_null() && task_get_type(sub()) != 0x3A7 && abortable() {
        return crate::common::new_task(0x1C, |m| {
            investigate_disturbance_ctor(m, this.byte_add(0x38), rd::<GPtr<u8>>(this, 0x10))
        });
    }
    if fcom_parity(range, lim, 0x41) || sub().is_null() || task_get_type(sub()) == 0xF4 || !abortable() {
        return sub();
    }
    let seq = crate::common::new_task(0x40, sequence_ctor);
    sequence_add_task(seq, crate::common::new_task(0x20, |m| stand_still_ctor(m, 1000, 0, 0, 8.0)));
    let grp = rd::<u32>(ped, 0x4D4);
    sequence_add_task(seq, crate::common::new_task(0x20, |m| run_anim_ctor(m, grp, 4, 4.0, 0)));
    sequence_add_task(seq, crate::common::new_task(0x20, ctor_48df30));
    seq
}

// ------------------------------------------------------------------------------------------- sunbathe, small helpers

/// `CTaskTimer::GetTimeLeft()` (0x62EB80, catalogue `FUN_0062eb80`): a started timer (+8) → `interval (+4) - now +
/// start (+0)` (wrapping), else 0. The entry's `nop; jmp` goes to the island 0x40131C, which loads the time as
/// `[[0x4EC103] ^ [0x156061C]]` = `[0xB7CB84]` (static image values) and jumps back.
#[hook(addr = 0x62EB80, conv = thiscall, name = "FUN_0062eb80", expect = "8A 41 08 84 C0 74 0E",
       unverified = "frontier2-a11", route = "L2", hazards = "none",
       note = "the island's pushfd / popfd keeps the flags; EDX = the time on the started path")]
pub fn task_timer_get_time_left(this: GPtr<u8>) -> i32 {
    if rd::<u8>(this, 8) == 0 {
        return 0;
    }
    rd::<u32>(this, 4).wrapping_sub(g::<u32>(TIME_MS).read()).wrapping_add(rd::<u32>(this, 0)) as i32
}

/// `CTaskComplexSunbathe::CreateSubTask(int32 type, CPed*)` (0x638290, `ret 8`; the entry's `nop; jmp` goes to the
/// island 0x409B02 that fetches the SEH link and jumps back to 0x638296). +0x20 the sunbather type, +0x24 / +0x28 the
/// two anim blocks (+0x10 loaded), +0x14 a task timer. The current sub-task when it already has the type. 0xCA:
/// `CTaskSimplePause(10000)` when the second block is not loaded and the anims should load, or the first block is
/// not loaded; else `CTaskSimplePause(GetRandomNumberInRange(1000, 5000))` (drawn after `new`). 0x1A2:
/// `CTaskSimpleRunTimedAnim(0x2F, type + 0xEF, 4.0, -4.0, time, 0x1A2, 0x86E098, true)`, the time `timer left + 1000`,
/// or for types 0 / 2 with the second block loaded `GetRandomNumberInRange(3000, min(left + 1000, 12000))`. 0x1AC /
/// 0x1AD / 0x1AE: `CTaskSimpleRunAnim(0x30, anim, 4.0, type, name, hold)` with anim `type + 0xF4` / (0xFE or 0x101 +
/// `GetRandomNumberInRange(0, 3)` for types 0 / 2, else 0x1AD: the bytes read the type argument's slot) / `type +
/// 0xF9`, names 0x86E654 / 0x86E644 / 0x86E634, hold 1 / 1 / 0. Anything else (0x516 too): NULL.
#[hook(addr = 0x638290, conv = thiscall, name = "CTaskComplexSunbathe::CreateSubTask", expect = "90 E9 6C 18 DD FF",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn sunbathe_create_sub_task(this: GPtr<u8>, ty: i32, _ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    if !sub.is_null() && task_get_type(sub) == ty {
        return rd::<GPtr<u8>>(this, 8);
    }
    let kind = rd::<i32>(this, 0x20);
    let loaded = |off: isize| rd::<u8>(rd::<GPtr<u8>>(this, off), 0x10) != 0;
    let anim = |a: u32, ty: u32, name: u32, hold: u8| {
        crate::common::new_task(0x20, |m| run_anim_ctor2(m, 0x30, a, 0x4080_0000, ty, name, hold))
    };
    match ty {
        0xCA => {
            let long = (!loaded(0x28) && crate::frontier_a_t6::should_load_sunbathe_anims() != 0) || !loaded(0x24);
            if long {
                crate::common::new_task(0x18, |m| pause_ctor(m, 10000))
            } else {
                crate::common::new_task(0x18, |m| pause_ctor(m, general_random_int_in_range(1000, 5000) as u32))
            }
        }
        0x1A2 => {
            let a = kind.wrapping_add(0xEF) as u32;
            let timer = this.byte_add(0x14);
            let time = if loaded(0x28) && (kind == 0 || kind == 2) {
                let hi = if task_timer_get_time_left(timer).wrapping_add(1000) > 12000 {
                    12000
                } else {
                    task_timer_get_time_left(timer).wrapping_add(1000)
                };
                general_random_int_in_range(3000, hi)
            } else {
                task_timer_get_time_left(timer).wrapping_add(1000)
            };
            crate::common::new_task(0x34, |m| {
                run_timed_anim_ctor2(m, 0x2F, a, 0x4080_0000, 0xC080_0000, time as u32, 0x1A2, 0x86E098, 1)
            })
        }
        0x1AC => anim(kind.wrapping_add(0xF4) as u32, 0x1AC, 0x86E654, 1),
        0x1AD => {
            let a = match kind {
                0 => general_random_int_in_range(0, 3) + 0xFE,
                2 => general_random_int_in_range(0, 3) + 0x101,
                _ => ty,
            };
            anim(a as u32, 0x1AD, 0x86E644, 1)
        }
        0x1AE => anim(kind.wrapping_add(0xF9) as u32, 0x1AE, 0x86E634, 0),
        _ => GPtr::NULL,
    }
}

/// `FUN_006955d0(float)` (0x6955D0, catalogue name `(missed by ghidra: cat7-call)`, thiscall, `ret 4`): a value
/// different from +0x18 (NaN too) is stored there, with +0x28 = now, +0x2C = 0 and +0x30 = 1 (a restarted task timer).
#[hook(addr = 0x6955D0, conv = thiscall, name = "(missed by ghidra: cat7-call)", expect = "D9 44 24 04 D8 59 18",
       unverified = "frontier2-a11", route = "L2", hazards = "none")]
pub fn set_value_and_restart_timer_6955d0(this: GPtr<u8>, v: f32) {
    if fcom_ne(x(v), x(rd::<f32>(this, 0x18))) {
        wr::<f32>(this, 0x18, v);
        wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
        wr::<u32>(this, 0x2C, 0);
        wr::<u8>(this, 0x30, 1);
    }
}

/// `CTaskComplexDieInCar::PreparePedVehicleForPedDeath(CVehicle*)` (0x62FD00, thiscall, `ret 4`; `this` unused): a
/// vehicle whose status (+0x36 >> 3) is 2 is switched to real physics (`CCarCtrl::SwitchVehicleToRealPhysics`); then
/// its autopilot gets +0x3D0 = 0, +0x3BA = 0, mission +0x3BB = 6 and +0x3BC = now + 2000. AL = 0.
#[hook(addr = 0x62FD00, conv = thiscall, name = "CTaskComplexDieInCar::PreparePedVehicleForPedDeath",
       expect = "56 8B 74 24 08 8A 46 36", unverified = "frontier2-a11", route = "L2", hazards = "heap-writes,ax-al-return",
       note = "AL is 0 from the cleared bytes; EAX's upper bytes come from the switch call or the status byte")]
pub fn die_in_car_prepare_ped_vehicle_for_ped_death(_this: GPtr<u8>, veh: GPtr<u8>) -> u8 {
    if rd::<u8>(veh, 0x36) & 0xF8 == 0x10 {
        switch_vehicle_to_real_physics(veh);
    }
    wr::<u8>(veh, 0x3D0, 0);
    wr::<u8>(veh, 0x3BA, 0);
    wr::<u8>(veh, 0x3BB, 6);
    wr::<u32>(veh, 0x3BC, g::<u32>(TIME_MS).read().wrapping_add(2000));
    0
}

/// The sunbathe task's anim block at `off` (+0x24 / +0x28) loaded (+0x10).
fn sunbathe_block_loaded(this: GPtr<u8>, off: isize) -> bool {
    rd::<u8>(rd::<GPtr<u8>>(this, off), 0x10) != 0
}

/// `CTaskComplexSunbathe::CreateFirstSubTask(CPed*)` (0x639CB0, `ret 4`). A `rand()` whose value is dropped (0x639CB9),
/// then the sunbather type (+0x20): peds of type 5 `2 - ftol((rand() & 0xFFFF) * 0x858B14 * 0x859024)`; sunbather
/// models `rand() < 0x3FFF`; others 1. Unloaded anim blocks (+0x24, +0x28) are requested (`+0x2C / +0x30 + 0x63E7`,
/// flags 8); +0xD = 0. With +0xC set and `ShouldLoadSunbatheAnims()`: 0x1AC when +0x34 is 0 and the second block is
/// loaded, else 0xCA. Otherwise: the first block loaded → +0x14 = now, +0x18 = `20000 - ftol((rand() & 0xFFFF) *
/// 0x858B14 * 0x86E668)`, +0x1C = 1, +0xD = 1 and 0x1A2; not loaded → 0xCA when the anims should load, else NULL.
#[hook(addr = 0x639CB0, conv = thiscall, name = "CTaskComplexSunbathe::CreateFirstSubTask", expect = "56 57 8B F1 E8 65 7E 1E 00",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn sunbathe_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    rand_at(0x639CB9);
    let scaled = |site: u32, k: usize| {
        let r = rand_at(site) & 0xFFFF;
        ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(k).read()))
    };
    let kind = if rd::<i32>(ped, 0x598) == 5 {
        2 - scaled(0x639CCB, 0x859024)
    } else if is_sunbather(i32::from(rd::<i16>(ped, 0x22))) != 0 {
        i32::from(rand_at(0x639D0B) < 0x3FFF)
    } else {
        1
    };
    wr::<i32>(this, 0x20, kind);
    for (blk, idx) in [(0x24, 0x2C), (0x28, 0x30)] {
        if !sunbathe_block_loaded(this, blk) {
            streaming_request_model(rd::<i32>(this, idx).wrapping_add(0x63E7), 8);
        }
    }
    wr::<u8>(this, 0xD, 0);
    let should = crate::frontier_a_t6::should_load_sunbathe_anims() != 0;
    let sub = |ty: i32| sunbathe_create_sub_task(this, ty, ped);
    if rd::<u8>(this, 0xC) != 0 && should {
        return if rd::<u32>(this, 0x34) == 0 && sunbathe_block_loaded(this, 0x28) { sub(0x1AC) } else { sub(0xCA) };
    }
    if sunbathe_block_loaded(this, 0x24) {
        let r = rand_at(0x639DD1) & 0xFFFF;
        wr::<u32>(this, 0x14, g::<u32>(TIME_MS).read());
        let t = ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x86E668).read()));
        wr::<i32>(this, 0x18, 20000 - t);
        wr::<u8>(this, 0x1C, 1);
        wr::<u8>(this, 0xD, 1);
        return sub(0x1A2);
    }
    if should { sub(0xCA) } else { GPtr::NULL }
}

/// `CTaskComplexSunbathe::CreateNextSubTask(CPed*)` (0x6399F0, `ret 4`). Peds of type 0x16 or 5 add an interesting
/// event (2). By the sub-task's type: 0xCA → with +0xD: 0x1AE when the anims should load and the second block is
/// loaded, else 0x516; without: 0x1AC when both blocks are loaded, else 0x516. 0x1A2 → the second block not loaded:
/// 0xCA; else 0x1AE unless `CanSunbathe()`, the timer +0x14 not out of time and type 0 or 2 → 0x1AD. 0x1AC → the first
/// block loaded: +0x14 = now, +0x18 = `GetRandomNumberInRange(20000, 100000)`, +0x1C = 1, 0x1A2 and +0xD = 1 after it;
/// else 0x516. 0x1AD → 0x1A2 with the first block loaded, else 0x516. 0x1AE → 0x516 and +0xD = 0. Others: NULL.
#[hook(addr = 0x6399F0, conv = thiscall, name = "CTaskComplexSunbathe::CreateNextSubTask", expect = "55 8B 6C 24 08 8B 85 98 05 00 00",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn sunbathe_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let pt = rd::<i32>(ped, 0x598);
    if pt == 0x16 || pt == 5 {
        interesting_events_add(GPtr::from_addr(0xC0B058), 2, ped);
    }
    let sub = |ty: i32| sunbathe_create_sub_task(this, ty, ped);
    let (b24, b28) = (|| sunbathe_block_loaded(this, 0x24), || sunbathe_block_loaded(this, 0x28));
    match task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0xCA => {
            if rd::<u8>(this, 0xD) != 0 {
                if crate::frontier_a_t6::should_load_sunbathe_anims() != 0 && b28() { sub(0x1AE) } else { sub(0x516) }
            } else if b28() && b24() {
                sub(0x1AC)
            } else {
                sub(0x516)
            }
        }
        0x1A2 => {
            if !b28() {
                return sub(0xCA);
            }
            if can_sunbathe() as u8 == 0 || task_timer_is_out_of_time(this.byte_add(0x14)) != 0 {
                return sub(0x1AE);
            }
            let kind = rd::<i32>(this, 0x20);
            if kind == 0 || kind == 2 { sub(0x1AD) } else { sub(0x1AE) }
        }
        0x1AC => {
            if !b24() {
                return sub(0x516);
            }
            let now = g::<u32>(TIME_MS).read();
            let r = general_random_int_in_range(20000, 100000);
            wr::<u32>(this, 0x14, now);
            wr::<i32>(this, 0x18, r);
            wr::<u8>(this, 0x1C, 1);
            let t = sub(0x1A2);
            wr::<u8>(this, 0xD, 1);
            t
        }
        0x1AD => if b24() { sub(0x1A2) } else { sub(0x516) },
        0x1AE => {
            let t = sub(0x516);
            wr::<u8>(this, 0xD, 0);
            t
        }
        _ => GPtr::NULL,
    }
}

orig! {
    /// `CTaskComplexDieInCar::CreateSubTask(int32 type, CPed*)` (0x62FD50, `ret 8`)
    fn die_in_car_create_sub_task(t: u32, ty: u32, ped: u32) -> u32 = thiscall 0x62FD50 => crate::gen_subtasks::g_task_complex_die_in_car_create_sub_task_62fd50;
    /// `CEvent::GetEventType()` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CEvent::Clone()` (vtable +0x10)
    fn v_event_clone(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x10 => unported;
    /// The scalar deleting destructor (vtable +0)
    fn v_delete(o: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CVehicle::CanPedStepOutCar(bool)` (vtable +0xE4)
    fn v_vehicle_can_ped_step_out_car(v: GPtr<u8>, ignore: u32) -> u8 = thiscall vslot 0xE4 => unported;
}

/// `CTaskComplexDieInCar::CreateFirstSubTask(CPed*)` (0x6375F0, `ret 4`). `SetPedState(0x36)`. A current event of type
/// 9 is passed on (cloned, its +0xC cleared, added to the event group +0x68, the clone deleted) to the vehicle's
/// driver and each passenger (`+0x488` seats from +0x464) other than the ped. Then 0xD6 (+0x478 bit 7, the task's +0xC
/// 0x33, a vehicle of class 3 or 4 (+0x594)); 0x2C1 when `CanPedStepOutCar(false)`; a normal-appearance vehicle (1)
/// driven by the ped: `PreparePedVehicleForPedDeath`, the timer (+0x10 now, +0x14 2000, +0x18 1) and 0x2C5; else
/// 0xD6.
#[hook(addr = 0x6375F0, conv = thiscall, name = "CTaskComplexDieInCar::CreateFirstSubTask", expect = "83 EC 08 53 55 57 8B 7C 24 18",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn die_in_car_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    ped_set_ped_state(ped, 0x36);
    let ev = history_get_current_event(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x38));
    if !ev.is_null() && v_event_type(ev) == 9 {
        let pass_on = |other: GPtr<u8>| {
            if !other.is_null() && other != ped {
                let c = v_event_clone(ev);
                wr::<u8>(c, 0xC, 0);
                event_group_add(rd::<GPtr<u8>>(other, 0x47C).byte_add(0x68), c, 0);
                v_delete(c, 1);
            }
        };
        pass_on(rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x58C), 0x460));
        let mut i = 0;
        while i < i32::from(rd::<u8>(rd::<GPtr<u8>>(ped, 0x58C), 0x488)) {
            pass_on(rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x58C), 0x464 + i as isize * 4));
            i += 1;
        }
    }
    let sub = |ty: u32| GPtr::from_addr(die_in_car_create_sub_task(this.addr() as u32, ty, ped.addr() as u32) as usize);
    if rd::<i8>(ped, 0x478) < 0 || rd::<u32>(this, 0xC) == 0x33
        || matches!(rd::<u32>(rd::<GPtr<u8>>(ped, 0x58C), 0x594), 3 | 4)
    {
        return sub(0xD6);
    }
    if v_vehicle_can_ped_step_out_car(rd::<GPtr<u8>>(ped, 0x58C), 0) != 0 {
        return sub(0x2C1);
    }
    if get_vehicle_appearance(rd::<GPtr<u8>>(ped, 0x58C)) == 1 && rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x58C), 0x460) == ped {
        die_in_car_prepare_ped_vehicle_for_ped_death(this, rd::<GPtr<u8>>(ped, 0x58C));
        wr::<u32>(this, 0x10, g::<u32>(TIME_MS).read());
        wr::<u32>(this, 0x14, 2000);
        wr::<u8>(this, 0x18, 1);
        return sub(0x2C5);
    }
    sub(0xD6)
}

// ------------------------------------------------------------------------------------------- flee entity

/// `CTaskComplexFleePoint::SetFleePosition(const CVector&, float radius, bool scream)` (0x65B7B0, thiscall, `ret 0xC`):
/// a point (+0xC) or radius (+0x34) that differs (NaN too) is stored with +0x39 = 1 (new point); the scream flag
/// (+0x38) is stored either way.
#[hook(addr = 0x65B7B0, conv = thiscall, name = "CTaskComplexFleePoint::SetFleePosition", expect = "D9 41 0C 8B 54 24 04",
       unverified = "frontier2-a11", route = "L2", hazards = "none")]
pub fn flee_point_set_flee_position(this: GPtr<u8>, pos: GPtr<u8>, radius: f32, scream: u8) {
    let ne = |off: isize, v: f32| fcom_ne(x(rd::<f32>(this, off)), x(v));
    if ne(0xC, rd::<f32>(pos, 0)) || ne(0x10, rd::<f32>(pos, 4)) || ne(0x14, rd::<f32>(pos, 8)) || ne(0x34, radius) {
        for i in 0..3 {
            wr::<u32>(this, 0xC + i * 4, rd::<u32>(pos, i * 4));
        }
        wr::<f32>(this, 0x34, radius);
        wr::<u8>(this, 0x39, 1);
    }
    wr::<u8>(this, 0x38, scream);
}

/// `CTaskComplexFleeEntity::ControlSubTask(CPed*)` (0x65BBB0, `ret 4`). Without an entity (+0xC) the sub-task is
/// asked to abort (`MakeAbortable(ped, 0, NULL)`). A flee-point sub-task (0x38C) with the timer started (+0x28; +0x29
/// restarts it at now) and expired (`now >= +0x20 + +0x24`), the entity farther than +0x38 from the stored point
/// (`(dz² + dy²) + dx² > r²`, unrounded): the timer restarts (+0x20 now, +0x24 = +0x34), the point is refreshed and
/// passed on (`SetFleePosition(&+0x10, +0x2C, +0x30)`), and a stack `CEventSeenPanickedPed(ped)` goes to the global
/// event group. Always the current sub-task.
#[hook(addr = 0x65BBB0, conv = thiscall, name = "CTaskComplexFleeEntity::ControlSubTask",
       expect = "64 A1 00 00 00 00 6A FF 68 B8 3F 84 00",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn flee_entity_control_sub_task_65bbb0(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(this, 0xC) == 0 {
        task_make_abortable(sub, ped, 0, GPtr::NULL);
        return rd::<GPtr<u8>>(this, 8);
    }
    if task_get_type(sub) != 0x38C || rd::<u8>(this, 0x28) == 0 {
        return rd::<GPtr<u8>>(this, 8);
    }
    if rd::<u8>(this, 0x29) != 0 {
        wr::<u32>(this, 0x20, g::<u32>(TIME_MS).read());
        wr::<u8>(this, 0x29, 0);
    }
    let end = rd::<u32>(this, 0x24).wrapping_add(rd::<u32>(this, 0x20));
    let now = g::<u32>(TIME_MS).read();
    if now < end {
        return rd::<GPtr<u8>>(this, 8);
    }
    let ep = entity_pos(rd::<GPtr<u8>>(this, 0xC));
    let r = x(rd::<f32>(this, 0x38));
    if r.fmul(r).fcom(crate::flee::dist2(this.byte_add(0x10), ep)).test_parity(5) {
        return rd::<GPtr<u8>>(this, 8);
    }
    wr::<u32>(this, 0x20, now);
    wr::<u32>(this, 0x24, rd::<u32>(this, 0x34));
    wr::<u8>(this, 0x28, 1);
    let p = entity_pos(rd::<GPtr<u8>>(this, 0xC));
    for i in 0..3 {
        wr::<u32>(this, 0x10 + i * 4, rd::<u32>(p, i * 4));
    }
    flee_point_set_flee_position(rd::<GPtr<u8>>(this, 8), this.byte_add(0x10), rd::<f32>(this, 0x2C), rd::<u8>(this, 0x30));
    crate::flee::broadcast_seen_panicked_ped(ped);
    rd::<GPtr<u8>>(this, 8)
}

// ------------------------------------------------------------------------------------------- follow leader in formation

/// The bounding-sphere radius of a vehicle's model collision (`ms_modelInfoPtrs[model]->m_pColModel` +0x24).
fn vehicle_col_radius(veh: GPtr<u8>) -> f32 {
    let mi = g::<GPtr<u8>>(0xA9B0C8 + (i32::from(rd::<i16>(veh, 0x22)) * 4) as usize).read();
    rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24)
}

/// `|ped - leader|²` through `CVector::Difference` and the magnitude helper 0x406DA0.
fn ped_leader_dist2(ped: GPtr<u8>, leader: GPtr<u8>) -> X87 {
    let mut d = [0f32; 3];
    let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
    vector_difference(dp, entity_pos(ped), entity_pos(leader));
    vector_magnitude_sqr(dp)
}

/// `CTaskComplexFollowLeaderInFormation::ControlSubTask(CPed*)` (0x696940, `ret 4`; the epilogue restores the SEH link
/// in the island 0x4019D9). No leader (+0x10): NULL. A 0x2C5 sub-task is kept. The leader in a vehicle: a 0x38B
/// sub-task gets the vehicle's collision radius (0x6955D0); with a follow radius (+0x24 > 0) and the ped farther
/// than it from the leader, an aborted sub-task is replaced by a pause (2000) after one `ProcessPed` of a stack
/// `CTaskSimpleStandStill(0, false, false, 8.0)`; a 0xCB sub-task of a ped without a group: within the vehicle's
/// radius (`|d|² < r²`) and a free passenger seat (+0x484 < +0x488) an aborted sub-task becomes
/// `CTaskComplexEnterCarAsPassenger(vehicle, 0, false)`, else within +0x24 `CreateSubTask(0x38B)` after an abort.
/// On foot: 0x2BC aborts into `CreateFirstSubTask`; 0xCA / 0xCB within +0x24 of the leader (`r² > (dz² + dy²) +
/// dx²`) → `CreateSubTask(0x38B)`. Otherwise the sub-task.
#[hook(addr = 0x696940, conv = thiscall, name = "CTaskComplexFollowLeaderInFormation::ControlSubTask",
       expect = "64 A1 00 00 00 00 6A FF 68 BE 78 84 00",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the original stores the follow radius (or the squared vehicle radius) over its own ped argument slot")]
pub fn follow_leader_in_formation_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    use crate::frontier_a_t5::follow_leader_in_formation_create_sub_task as create_sub;
    let sub = rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(this, 0x10) == 0 {
        return GPtr::NULL;
    }
    if task_get_type(sub) == 0x2C5 {
        return sub;
    }
    let leader = rd::<GPtr<u8>>(this, 0x10);
    let abort = || task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 0, GPtr::NULL) != 0;
    if rd::<u32>(leader, 0x46C) & 0x100 != 0 && rd::<u32>(leader, 0x58C) != 0 {
        let ty = task_get_type(rd::<GPtr<u8>>(this, 8));
        if ty == 0x38B {
            set_value_and_restart_timer_6955d0(rd::<GPtr<u8>>(this, 8),
                vehicle_col_radius(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), 0x58C)));
            if !gt(x(rd::<f32>(this, 0x24)), x(g::<f32>(0x858B50).read())) {
                return sub;
            }
            let m = ped_leader_dist2(ped, rd::<GPtr<u8>>(this, 0x10));
            let r = rd::<f32>(this, 0x24);
            if x(r).fmul_m(r).fcom(m).test_parity(5) || !abort() {
                return sub;
            }
            let mut stand = [0u32; 8];
            let st = GPtr::from_raw(stand.as_mut_ptr()).cast::<u8>();
            stand_still_ctor(st, 0, 0, 0, 8.0);
            stand_still_process_ped(st, ped);
            let t = crate::common::new_task(0x18, |m| pause_ctor(m, 2000));
            stand_still_dtor(st);
            return t;
        }
        if task_get_type(rd::<GPtr<u8>>(this, 8)) != 0xCB || ped_groups_get_peds_group(ped.addr() as u32) != 0 {
            return sub;
        }
        let veh = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), 0x58C);
        let r = vehicle_col_radius(veh);
        let v = x(r).fmul_m(r).to_f32();
        let m = ped_leader_dist2(ped, rd::<GPtr<u8>>(this, 0x10));
        if !m.fcom(x(v)).test_parity(5) {
            if rd::<u8>(veh, 0x484) >= rd::<u8>(veh, 0x488) || !abort() {
                return sub;
            }
            return crate::common::new_task(0x50, |t| {
                enter_car_as_passenger_ctor(t, rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), 0x58C), 0, 0)
            });
        }
        let f = rd::<f32>(this, 0x24);
        if m.fcom(x(f).fmul_m(f)).test_parity(5) || !abort() {
            return sub;
        }
        return create_sub(this, 0x38B, ped);
    }
    match task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0x2BC => if abort() { v_create_first_sub_task(this, ped) } else { sub },
        0xCA | 0xCB => {
            let r = x(rd::<f32>(this, 0x24));
            let d2 = crate::flee::dist2(entity_pos(ped), entity_pos(rd::<GPtr<u8>>(this, 0x10)));
            if le_or_nan(r.fmul(r), d2) { sub } else { create_sub(this, 0x38B, ped) }
        }
        _ => sub,
    }
}

// ------------------------------------------------------------------------------------------- track entity

/// `CTaskSimpleGoToPointFine::SetTargetPos(CVector)` (0x65F330, thiscall, `ret 0xC`, the vector by value): a target
/// (+0xC) differing in any component, or a radius (+0x18) other than 0x86FC78 (NaN too), stores the target, the
/// radius 0x86FC78 and +0x1C = `(+0x1C & 0xF0) | 0x20`. ECX is left as `this` (the caller reuses it).
#[hook(addr = 0x65F330, conv = thiscall, name = "CTaskSimpleGoToPointFine::SetTargetPos", expect = "D9 05 78 FC 86 00",
       unverified = "frontier2-a11", route = "L2", hazards = "none")]
pub fn go_to_point_fine_set_target_pos(this: GPtr<u8>, px: f32, py: f32, pz: f32) {
    let c = x(g::<f32>(0x86FC78).read());
    let ne = |off: isize, v: X87| fcom_ne(x(rd::<f32>(this, off)), v);
    if ne(0xC, x(px)) || ne(0x10, x(py)) || ne(0x14, x(pz)) || ne(0x18, c) {
        wr::<f32>(this, 0x18, c.to_f32());
        wr::<f32>(this, 0xC, px);
        wr::<f32>(this, 0x10, py);
        wr::<f32>(this, 0x14, pz);
        wr::<u8>(this, 0x1C, (rd::<u8>(this, 0x1C) & 0xF0) | 0x20);
    }
}

/// `CTaskComplexTrackEntity::CalcTargetPos(CPed*)` (0x65F780, `ret 4`). +0x44 = the entity's (+0xC) position plus the
/// offset (+0x10, +0x14): along the entity's right / forward axes with +0x2C set (the y and z products through f32
/// stores, the x products unrounded), else added to x and y; a vehicle or ped (type 2 / 3) adds its move speed times
/// the time step. +0x50 = the 2D squared distance to the ped (`dy² + dx²`).
#[hook(addr = 0x65F780, conv = thiscall, name = "CTaskComplexTrackEntity::CalcTargetPos", expect = "83 EC 24 53 56 57 8B F9",
       this = "CTaskComplexTrackEntity", unverified = "frontier2-a11", route = "L2", hazards = "none")]
pub fn track_entity_calc_target_pos(this: GPtr<u8>, ped: GPtr<u8>) {
    let t = this.byte_add(0x44);
    let p = entity_pos(rd::<GPtr<u8>>(this, 0xC));
    for i in 0..3 {
        wr::<u32>(t, i * 4, rd::<u32>(p, i * 4));
    }
    let add = |k: isize, v: X87| wr::<f32>(t, k * 4, v.fadd_m(rd::<f32>(t, k * 4)).to_f32());
    let add_axis = |axis: [f32; 3], o: f32| {
        let ax = x(axis[0]).fmul_m(o);
        let ay = x(axis[1]).fmul_m(o).to_f32();
        let az = x(axis[2]).fmul_m(o).to_f32();
        add(0, ax);
        add(1, x(ay));
        add(2, x(az));
    };
    if rd::<u8>(this, 0x2C) != 0 {
        let m = get_matrix(rd::<GPtr<u8>>(this, 0xC));
        let right = [rd::<f32>(m, 0), rd::<f32>(m, 4), rd::<f32>(m, 8)];
        let m = get_matrix(rd::<GPtr<u8>>(this, 0xC));
        let fwd = [rd::<f32>(m, 0x10), rd::<f32>(m, 0x14), rd::<f32>(m, 0x18)];
        add_axis(right, rd::<f32>(this, 0x10));
        add_axis(fwd, rd::<f32>(this, 0x14));
    } else {
        add(0, x(rd::<f32>(this, 0x10)));
        add(1, x(rd::<f32>(this, 0x14)));
    }
    let e = rd::<GPtr<u8>>(this, 0xC);
    if matches!(rd::<u8>(e, 0x36) & 7, 2 | 3) {
        let ts = g::<f32>(0xB7CB5C).read();
        let vx = x(ts).fmul_m(rd::<f32>(e, 0x44));
        let vy = x(ts).fmul_m(rd::<f32>(e, 0x48));
        let vz = x(ts).fmul_m(rd::<f32>(e, 0x4C)).to_f32();
        add(0, vx);
        add(1, vy);
        add(2, x(vz));
    }
    let pp = entity_pos(ped);
    let dx = x(rd::<f32>(t, 0)).fsub_m(rd::<f32>(pp, 0));
    let dy = x(rd::<f32>(t, 4)).fsub_m(rd::<f32>(pp, 4));
    wr::<f32>(this, 0x50, dy.fmul(dy).fadd(dx.fmul(dx)).to_f32());
}

/// `CTaskComplexTrackEntity::CalcMoveRatio(CPed*)` (0x65F930, `ret 4`; the ped unused). Function statics (guard
/// 0xC18D08): the squares of 0x86F9D0 / 0x86F9CC / 0x86F9C8 and `1 / (c2 - c1)`, `1 / (c1 - c0)`. From the squared
/// distance (+0x50): below c0² → 0; above c2² → 1; else `(sqrt(d) - c0) / (c1 - c0) * 0.5` below c1², otherwise
/// `(sqrt(d) - c1) / (c2 - c1) * 0.5 + 0.5`. Then `ratio = sqrt(ratio) * 0x858B3C`, raised to 2 above 0x858CA0 unless
/// +0x1C; +0x3C moves towards it by at most 0x86F9C4 per call.
#[hook(addr = 0x65F930, conv = thiscall, name = "CTaskComplexTrackEntity::CalcMoveRatio", expect = "A1 08 8D C1 00 A8 01",
       this = "CTaskComplexTrackEntity", unverified = "frontier2-a11", route = "L2", hazards = "none")]
pub fn track_entity_calc_move_ratio(this: GPtr<u8>, _ped: GPtr<u8>) {
    let guard = g::<u32>(0xC18D08);
    let c = |a: usize| g::<f32>(a).read();
    let (c0, c1, c2, one) = (0x86F9D0, 0x86F9CC, 0x86F9C8, 0x858624);
    let mut gv = guard.read();
    for (bit, dst, v) in [
        (1u32, 0xC18D04usize, None),
        (2, 0xC18D00, None),
        (4, 0xC18CFC, None),
        (8, 0xC18CF8, Some((c2, c1))),
        (0x10, 0xC18CF4, Some((c1, c0))),
    ] {
        if gv & bit == 0 {
            gv |= bit;
            let r = match v {
                None => {
                    let k = [c0, c1, c2][bit.trailing_zeros() as usize];
                    x(c(k)).fmul_m(c(k))
                }
                Some((a, b)) => x(c(a)).fsub_m(c(b)).fdivr_m(c(one)),
            };
            guard.write(gv);
            g::<f32>(dst).write(r.to_f32());
        }
    }
    let d = x(rd::<f32>(this, 0x50));
    let ratio = if !d.fcom(x(c(0xC18D04))).test_parity(5) {
        x(0.0)
    } else if gt(d, x(c(0xC18CFC))) {
        x(1.0)
    } else if !d.fcom(x(c(0xC18D00))).test_parity(5) {
        d.fsqrt().fsub_m(c(c0)).fmul_m(c(0xC18CF4)).fmul_m(c(0x858B8C))
    } else {
        d.fsqrt().fsub_m(c(c1)).fmul_m(c(0xC18CF8)).fmul_m(c(0x858B8C)).fadd_m(c(0x858B8C))
    };
    wr::<f32>(this, 0x40, ratio.to_f32());
    let r = x(rd::<f32>(this, 0x40)).fsqrt().fmul_m(c(0x858B3C));
    wr::<f32>(this, 0x40, r.to_f32());
    if rd::<u8>(this, 0x1C) == 0 && gt(r, x(c(0x858CA0))) {
        wr::<u32>(this, 0x40, 0x4000_0000);
    }
    let step = c(0x86F9C4);
    if gt(x(rd::<f32>(this, 0x40)).fsub_m(rd::<f32>(this, 0x3C)), x(step)) {
        wr::<f32>(this, 0x3C, x(step).fadd_m(rd::<f32>(this, 0x3C)).to_f32());
    } else {
        wr::<u32>(this, 0x3C, rd::<u32>(this, 0x40));
    }
}

/// `CTaskComplexTrackEntity::ControlSubTask(CPed*)` (0x663640, `ret 4`). No entity (+0xC), or a started timer (+0x38;
/// +0x39 restarts it at now) that expired (`now >= +0x30 + +0x34`): the sub-task aborts (`MakeAbortable(ped, 1,
/// NULL)`) → NULL, else the sub-task. Otherwise `CalcTargetPos`, then by the sub-task's type: 0x38A → NULL when the
/// squared distance (+0x50) is above `(+0x28)²`; below `(+0x24)²` an aborting sub-task is replaced by
/// `CreateNextSubTask`. 0x39E → at or beyond `(+0x24)²` an aborting sub-task is replaced likewise; else
/// `CalcMoveRatio`, `SetTargetPos(+0x44)` and the go-to's move ratio (+0x20) = +0x3C. Otherwise the sub-task.
#[hook(addr = 0x663640, conv = thiscall, name = "CTaskComplexTrackEntity::ControlSubTask", expect = "56 8B F1 8B 46 0C 85 C0",
       this = "CTaskComplexTrackEntity", unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn track_entity_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let abort = || task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, GPtr::NULL) != 0;
    let mut expired = rd::<u32>(this, 0xC) == 0;
    if !expired && rd::<u8>(this, 0x38) != 0 {
        if rd::<u8>(this, 0x39) != 0 {
            wr::<u32>(this, 0x30, g::<u32>(TIME_MS).read());
            wr::<u8>(this, 0x39, 0);
        }
        expired = g::<u32>(TIME_MS).read() >= rd::<u32>(this, 0x34).wrapping_add(rd::<u32>(this, 0x30));
    }
    if expired {
        return if abort() { GPtr::NULL } else { rd::<GPtr<u8>>(this, 8) };
    }
    track_entity_calc_target_pos(this, ped);
    let d = x(rd::<f32>(this, 0x50));
    let sq = |off: isize| {
        let r = x(rd::<f32>(this, off));
        r.fmul(r)
    };
    match task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0x38A => {
            if gt(d, sq(0x28)) {
                return GPtr::NULL;
            }
            if d.fcom(sq(0x24)).test_parity(5) || !abort() {
                return rd::<GPtr<u8>>(this, 8);
            }
            task_create_next_sub_task(this, ped)
        }
        0x39E => {
            if !d.fcom(sq(0x24)).test_any(1) && abort() {
                return task_create_next_sub_task(this, ped);
            }
            track_entity_calc_move_ratio(this, ped);
            let sub = rd::<GPtr<u8>>(this, 8);
            go_to_point_fine_set_target_pos(sub, rd::<f32>(this, 0x44), rd::<f32>(this, 0x48), rd::<f32>(this, 0x4C));
            wr::<f32>(sub, 0x20, x(rd::<f32>(this, 0x3C)).to_f32());
            rd::<GPtr<u8>>(this, 8)
        }
        _ => rd::<GPtr<u8>>(this, 8),
    }
}

// ------------------------------------------------------------------------------------------- choking

/// `CTaskSimpleChoking::ProcessPed(CPed*)` (0x620490, `ret 4`, AL). Finished (+0x19): 1. A non-player ped with an
/// attacker (+8) and +0x18 clear turns to face it when it is in front (`d · forward > 0`, the ped's matrix read
/// unchecked): +0x55C = `atan2(-dx, dy)`. +0x10 counts down by `ftol(timeStep * 0x858B38 * 0x858C4C)` (to 0). No anim
/// (+0xC): `BlendAnimation(clump, 0, 0x15, 4.0)` with the finish callback 0x620480, its speed (+0x24) `rand() *
/// 0x858C7C * 0x858C84 + 0x858F34`. A run-out timer with anim 0x15: that anim's callback cleared (0x4D1390), anim 0xA
/// blended in the same way, +0x10 = `now - +0x14` kept when below `GetRandomNumberInRange(8000, 12000)`, else a second
/// draw; with another anim it fades out (blend delta -4, flag +0x2E bit 2). Then `Say(0x154)`; AL = 0.
#[hook(addr = 0x620490, conv = thiscall, name = "CTaskSimpleChoking::ProcessPed", expect = "56 8B F1 8A 46 19 84 C0",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes,ax-al-return",
       note = "AL result; EAX's upper bytes come from the callees")]
pub fn choking_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x19) != 0 {
        return 1;
    }
    let attacker = rd::<GPtr<u8>>(this, 8);
    if !attacker.is_null() && ped_is_player(ped) == 0 && rd::<u8>(this, 0x18) == 0 {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let pp = entity_pos(ped);
        let ap = entity_pos(rd::<GPtr<u8>>(this, 8));
        let dx = x(rd::<f32>(ap, 0)).fsub_m(rd::<f32>(pp, 0));
        let dy = x(rd::<f32>(ap, 4)).fsub_m(rd::<f32>(pp, 4));
        let dz = x(rd::<f32>(ap, 8)).fsub_m(rd::<f32>(pp, 8));
        let dot = dz.fmul_m(rd::<f32>(m, 0x18)).fadd(dy.fmul_m(rd::<f32>(m, 0x14))).fadd(dx.fmul_m(rd::<f32>(m, 0x10)));
        if gt(dot, x(g::<f32>(0x858B50).read())) {
            wr::<f32>(ped, 0x55C, dx.fchs().fpatan(dy).to_f32());
        }
    }
    let ts = g::<f32>(0xB7CB5C).read();
    let t = ftol32(x(ts).fmul_m(g::<f32>(0x858B38).read()).fmul_m(g::<f32>(0x858C4C).read())) as u32;
    let left = rd::<u32>(this, 0x10);
    wr::<u32>(this, 0x10, if left > t { left - t } else { 0 });
    let blend = |anim: u32| {
        let a = anim_blend_animation(rd::<GPtr<u8>>(ped, 0x18), 0, anim, 4.0);
        wr::<GPtr<u8>>(this, 0xC, a);
        assoc_set_finish_callback(a, 0x620480, this.addr() as u32);
    };
    let anim = rd::<GPtr<u8>>(this, 0xC);
    if anim.is_null() {
        blend(0x15);
        let r = rand_at(0x62057C);
        let v = xi(r).fmul_m(g::<f32>(0x858C7C).read()).fmul_m(g::<f32>(0x858C84).read()).fadd_m(g::<f32>(0x858F34).read());
        wr::<f32>(rd::<GPtr<u8>>(this, 0xC), 0x24, v.to_f32());
    } else if rd::<u32>(this, 0x10) == 0 {
        if rd::<i16>(anim, 0x2C) == 0x15 {
            assoc_set_finish_callback(anim, 0x4D1390, 0);
            blend(0xA);
            let e = g::<u32>(TIME_MS).read().wrapping_sub(rd::<u32>(this, 0x14));
            wr::<u32>(this, 0x10, e);
            if e >= general_random_int_in_range(8000, 12000) as u32 {
                wr::<i32>(this, 0x10, general_random_int_in_range(8000, 12000));
            }
        } else {
            wr::<u32>(anim, 0x1C, 0xC080_0000);
            let a = rd::<GPtr<u8>>(this, 0xC);
            wr::<u8>(a, 0x2E, rd::<u8>(a, 0x2E) | 4);
        }
    }
    ped_say(ped, 0x154, 0, 1.0, 0, 0, 0);
    0
}

// ------------------------------------------------------------------------------------------- vt_86d868 (hit reaction)

/// `FUN_00620c30(int32 type)` (0x620C30, catalogue `thunk_FUN_004020ff`, thiscall, `ret 4`; vt_86d868's CreateSubTask;
/// the entry's `nop; jmp` fetches the SEH link in the island 0x4020FF). 0xD0 → `CTaskComplexFallAndGetUp(+0x1C == 2 ?
/// 0x1A : 0x14, 0, 1000)`; 0xC8 → a bare `CTaskSimple` with vtable 0x859CFC; 0x3F0 → `CTaskSimpleBeHit(+0x14 when it is
/// a ped (type 3) else NULL, +0x20, +0x1C, ftol(+0x18))` (the float converted after `new`); others NULL.
#[hook(addr = 0x620C30, conv = thiscall, name = "thunk_FUN_004020ff", expect = "90 E9 C9 14 DE FF",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn hit_reaction_create_sub_task(this: GPtr<u8>, ty: i32) -> GPtr<u8> {
    match ty {
        0xD0 => {
            let anim = if rd::<i32>(this, 0x1C) == 2 { 0x1A } else { 0x14 };
            crate::common::new_task(0x18, |m| fall_and_get_up_ctor(m, anim, 0, 1000))
        }
        0xC8 => crate::common::new_task(8, |m| {
            task_simple_ctor(m);
            wr::<u32>(m, 0, 0x859CFC);
            m
        }),
        0x3F0 => {
            let src = rd::<GPtr<u8>>(this, 0x14);
            let src = if !src.is_null() && rd::<u8>(src, 0x36) & 7 == 3 { src.addr() as u32 } else { 0 };
            crate::common::new_task(0x28, |m| {
                let t = ftol32(x(rd::<f32>(this, 0x18)));
                be_hit_ctor(m, src, rd::<u32>(this, 0x20), rd::<u32>(this, 0x1C), t)
            })
        }
        _ => GPtr::NULL,
    }
}

/// `vt_86d868::CreateFirstSubTask(CPed*)` (0x625C70, `ret 4`). +0xC = 0. A player hit with more than 9 (+0x10)
/// whose pad 0 is not disabled and shows input (`GetTarget`, or any of the stick / shoulder words +0, +2, +0x10..+0x16)
/// → 0xC8. Direction 3 (+0x20) with the ped's +0x470 bit 26: weapon types 0x1B / 0x19, or 0x18 for a non-player →
/// knocked over: the 2D direction from the ped to the source (+0x14), normalised, times 0x858FC8 with z 5.0 as a move
/// force (+0x46C bit 0 cleared first) and 0xD0. Else, with direction 3, a hit below 9 and health (+0x540) below
/// 0x858CA4 → 0xD0. Otherwise 0x3F0.
#[hook(addr = 0x625C70, conv = thiscall, name = "vt_86d868::CreateFirstSubTask", expect = "83 EC 14 53 55 56 57 8B 7C 24 28",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn hit_reaction_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wr::<u8>(this, 0xC, 0);
    let hit = || rd::<i32>(this, 0x10);
    if ped_is_player(ped) != 0 && hit() > 9 {
        let pad = get_pad(0);
        if rd::<u16>(pad, 0x10E) == 0
            && (pad_get_target(pad) != 0 || [0, 2, 0x10, 0x12, 0x14, 0x16].iter().any(|&o| rd::<u16>(pad, o) != 0))
        {
            return hit_reaction_create_sub_task(this, 0xC8);
        }
    }
    if rd::<i32>(this, 0x20) != 3 {
        return hit_reaction_create_sub_task(this, 0x3F0);
    }
    if rd::<u32>(ped, 0x470) & 0x400_0000 != 0 {
        let t = hit();
        if t == 0x1B || t == 0x19 || (t == 0x18 && ped_is_player(ped) == 0) {
            let sp = entity_pos(rd::<GPtr<u8>>(this, 0x14));
            let pp = entity_pos(ped);
            let mut v = [
                x(rd::<f32>(sp, 0)).fsub_m(rd::<f32>(pp, 0)).to_f32(),
                x(rd::<f32>(sp, 4)).fsub_m(rd::<f32>(pp, 4)).to_f32(),
            ];
            vector2d_normalise(GPtr::from_raw(v.as_mut_ptr()).cast());
            let k = g::<f32>(0x858FC8).read();
            let f = CVector { x: x(v[0]).fmul_m(k).to_f32(), y: x(v[1]).fmul_m(k).to_f32(), z: 5.0 };
            wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & !1);
            physical_apply_move_force_5429f0(ped, f);
            return hit_reaction_create_sub_task(this, 0xD0);
        }
    }
    if hit() < 9 && lt(x(rd::<f32>(ped, 0x540)), x(g::<f32>(0x858CA4).read())) {
        return hit_reaction_create_sub_task(this, 0xD0);
    }
    hit_reaction_create_sub_task(this, 0x3F0)
}

// ------------------------------------------------------------------------------------------- vt_87004c (seek entity)

/// `FUN_00673070(int32 type, CPed*)` (0x673070, catalogue `thunk_FUN_00402204`, `ret 8`; vt_87004c's CreateSubTask, a
/// `CTaskComplexSeekEntity` instance with the XYOffset layout: flags +0x54, move state +0x50; the entry's `nop; jmp`
/// fetches the SEH link in the island 0x402204). The bytes of 0x665270 except 0x2CF, which calls
/// `CTaskSimpleCarDriveTimed(vehicle, 2000)` (0x5FF940) where 0x665270 inlines it.
#[hook(addr = 0x673070, conv = thiscall, name = "thunk_FUN_00402204", expect = "90 E9 8E F1 D8 FF",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn seek_87004c_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    if ty == 0x2CF {
        let veh = rd::<GPtr<u8>>(ped, 0x58C);
        return crate::common::new_task(0x70, |m| car_drive_timed_ctor(m, veh, 2000));
    }
    crate::seek::seek_create_sub_task(this, ty, ped, 0x54)
}

/// `vt_87004c::CreateFirstSubTask(CPed*)` (0x6733E0, `ret 4`): the shared seek-entity body (0x665600's bytes) with
/// `CreateSubTask` 0x673070.
#[hook(addr = 0x6733E0, conv = thiscall, name = "vt_87004c::CreateFirstSubTask", expect = "83 EC 0C 56 8B F1",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn seek_87004c_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    crate::seek::seek_create_first_sub_task(this, ped, 0x54, seek_87004c_create_sub_task)
}

/// `vt_87004c::CreateNextSubTask(CPed*)` (0x672E80, `ret 4`): the shared seek-entity body (0x665080's bytes) with
/// `CreateSubTask` 0x673070.
#[hook(addr = 0x672E80, conv = thiscall, name = "vt_87004c::CreateNextSubTask", expect = "64 A1 00 00 00 00 6A FF 68 58 58 84 00",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn seek_87004c_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    crate::seek::seek_create_next_sub_task(this, ped, 0x54, seek_87004c_create_sub_task)
}

/// `vt_87004c::ControlSubTask(CPed*)` (0x673540, `ret 4`): instruction for instruction 0x665760 (the XYOffset
/// instance), with `CreateSubTask` 0x673070 and the seek position 0x673BA0 (a copy of 0x665DC0).
#[hook(addr = 0x673540, conv = thiscall, name = "vt_87004c::ControlSubTask", expect = "6A FF 68 22 59 84 00",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn seek_87004c_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    crate::seek::seek_control_sub_task_with(this, ped, true, seek_87004c_create_sub_task, crate::misc::fun_00673ba0)
}

// ------------------------------------------------------------------------------------------- create car and get in

/// `CTaskSimpleCreateCarAndGetIn::ProcessPed(CPed*)` (0x64CF40, `ret 4`, AL; SEH frame). +8 the position, +0x14 the
/// model, +0x18 the car, +0x1C the road node position, +0x28 / +0x2C / +0x30 / +0x31 a task timer. The car-path node
/// closest to +8 (`FindNodeClosestToCoors(.., 0, 1e6, 1, 1, 0, 0, 0)`, `FindNodeCoorsForScript`): none found → the
/// timer (2000 ms, started on first use) decides: expired 1, else 0. Found: +0x1C = the node, +0x30 = 0; without a car
/// yet, the node visible (`IsSphereVisible(node, 3.0)`) or the ped visible (its model's bounding radius) or the node
/// not clear for a ped (`IsPositionClearForPed(ped pos, 3.0, -1, NULL, 1, 1, 1)`) → 1. Then a stack
/// `CTaskSimpleStandStill(0, false, false, 8.0)` is processed once; the model not loaded → `RequestModel(model, 0xC)`,
/// 0. Else the car is created (`CreateCarForScript(model, node, true)`) with the ped set in as driver (a stack
/// `CTaskSimpleCarSetPedInAsDriver(car, NULL)` with +0x18 = 1, processed once); a car flagged +0x1C bit 2 or bit 18 →
/// 0; else it is released to the world (`CleanUpThisVehicle`, `RemoveEntityFromList(vehicle ref, 1)`), +0x18 = 0, 1.
#[hook(addr = 0x64CF40, conv = thiscall, name = "CTaskSimpleCreateCarAndGetIn::ProcessPed",
       expect = "6A FF 64 A1 00 00 00 00 68 A0 31 84 00",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes,ax-al-return,stack-residue",
       note = "AL result; the original stores the ped's bounding radius over its own ped argument slot")]
pub fn create_car_and_get_in_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let paths = GPtr::<u8>::from_addr(0x96F050);
    let mut node = [0u32; 1];
    find_node_closest_to_coors(paths, GPtr::from_raw(node.as_mut_ptr()), rd::<f32>(this, 8), rd::<f32>(this, 0xC),
        rd::<f32>(this, 0x10), 0, f32::from_bits(0x4974_23FE), 1, 1, 0, 0, 0);
    let mut found = [0u8; 4];
    let mut pos = [0u32; 3];
    find_node_coors_for_script(paths, GPtr::from_raw(pos.as_mut_ptr()).cast(), node[0],
        GPtr::from_raw(found.as_mut_ptr()));
    if found[0] == 0 {
        if rd::<u8>(this, 0x30) == 0 {
            wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
            wr::<u32>(this, 0x2C, 2000);
            wr::<u8>(this, 0x30, 1);
        }
        if rd::<u8>(this, 0x31) != 0 {
            wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
            wr::<u8>(this, 0x31, 0);
        }
        let end = rd::<u32>(this, 0x2C).wrapping_add(rd::<u32>(this, 0x28));
        return u8::from(g::<u32>(TIME_MS).read() >= end);
    }
    for (i, v) in pos.iter().enumerate() {
        wr::<u32>(this, 0x1C + i as isize * 4, *v);
    }
    let target = this.byte_add(0x1C);
    wr::<u8>(this, 0x30, 0);
    let camera = GPtr::<u8>::from_addr(0xB6F028);
    if rd::<u32>(this, 0x18) == 0 {
        if camera_is_sphere_visible(camera, target, 3.0) != 0 {
            return 1;
        }
        let mi = g::<GPtr<u8>>(0xA9B0C8 + (i32::from(rd::<i16>(ped, 0x22)) * 4) as usize).read();
        let r = rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24);
        if camera_is_sphere_visible(camera, entity_pos(ped), r) != 0 {
            return 1;
        }
        if is_position_clear_for_ped(entity_pos(ped), 3.0, -1, GPtr::NULL, 1, 1, 1) == 0 {
            return 1;
        }
    }
    let mut stand = [0u32; 8];
    let st = GPtr::from_raw(stand.as_mut_ptr()).cast::<u8>();
    stand_still_ctor(st, 0, 0, 0, 8.0);
    stand_still_process_ped(st, ped);
    let model = rd::<i32>(this, 0x14);
    if g::<u8>((0x8E4CD0 + model * 20) as usize).read() != 1 {
        streaming_request_model(model, 0xC);
        stand_still_dtor(st);
        return 0;
    }
    if rd::<u32>(this, 0x18) == 0 {
        let car = create_car_for_script(model as u32, rd::<u32>(target, 0), rd::<u32>(target, 4), rd::<u32>(target, 8), 1);
        wr::<GPtr<u8>>(this, 0x18, car);
        let mut set_in = [0u32; 7];
        let si = GPtr::from_raw(set_in.as_mut_ptr()).cast::<u8>();
        set_ped_in_as_driver_ctor(si, car, GPtr::NULL);
        wr::<u8>(si, 0x18, 1);
        set_ped_in_as_driver_process_ped(si, ped);
        set_ped_in_as_driver_dtor(si);
    }
    let car = rd::<GPtr<u8>>(this, 0x18);
    let f = rd::<u32>(car, 0x1C);
    if f & 4 != 0 || f & 0x4_0000 != 0 {
        stand_still_dtor(st);
        return 0;
    }
    clean_up_this_vehicle(car);
    let handle = sa_core::pool::get_vehicle_ref(rd::<GPtr<u8>>(this, 0x18).cast());
    cleanup_remove_entity_from_list(GPtr::from_addr(0xA90850), handle, 1);
    wr::<u32>(this, 0x18, 0);
    stand_still_dtor(st);
    1
}

// ------------------------------------------------------------------------------------------- chat allocator

/// `CPedGroupDefaultTaskAllocatorChat::AllocateDefaultTasks(CPedGroup*, CPed*)` (0x5F8180, `ret 8`; `this` unused; SEH
/// frame). The group's occupied slots (members +0xC..+0x24, leader +0x28) in order; eight stack sequences
/// (`eh vector constructor iterator`, 0x40 bytes). Each occupied slot's sequence gets
/// `CTaskComplexGoToPointAndStandStill(4, its ped's position, 0x86FC84, 0x86FC88)` and `CTaskSimpleAchieveHeading(ped
/// +0x558, 0x86FC7C, 0x86FC80)` and repeats (+0x30 = 1). Then `n` rounds: a talker `ftol(n * (rand() & 0xFFFF) *
/// 0x858B14)` and a time `3000 - ftol((rand() & 0xFFFF) * 0x858B14 * 0x859A90)`; the talker's sequence gets a
/// `CTaskSimpleRunTimedAnim(0, 0xC, 4.0, -4.0, time, 0x1A0, 0x86C7B4, false)` re-tabled 0x86C78C, the others a
/// `CTaskSimpleStandStill(time, false, false, 8.0)`. Each slot's pair (+0x21C, 0x14 each) whose ped is the given one
/// (or for a NULL ped) takes a clone of its sequence; the stack sequences are destroyed.
#[hook(addr = 0x5F8180, conv = thiscall, name = "CPedGroupDefaultTaskAllocatorChat::AllocateDefaultTasks",
       expect = "6A FF 68 8E E0 83 00",
       unverified = "frontier2-a11", route = "L2", hazards = "heap-writes")]
pub fn chat_allocate_default_tasks(_this: GPtr<u8>, group: GPtr<u8>, ped: GPtr<u8>) {
    let mut list = [0i32; 8];
    let mut n = 0usize;
    for i in 0..8 {
        if rd::<u32>(group, 0xC + i as isize * 4) != 0 {
            list[n] = i;
            n += 1;
        }
    }
    let mut seqs = [0u32; 0x80];
    let sp = GPtr::from_raw(seqs.as_mut_ptr()).cast::<u8>();
    eh_vector_ctor(sp, 0x40, 8, 0x632BD0, 0x6389F0);
    let seq = |i: i32| sp.byte_add(i as isize * 0x40);
    for &i in &list[..n] {
        let m = rd::<GPtr<u8>>(group, 0xC + i as isize * 4);
        let go = crate::common::new_task(0x28, |t| {
            let (a, b) = (g::<f32>(0x86FC84).read(), g::<f32>(0x86FC88).read());
            go_to_point_and_stand_still_ctor(t, 4, entity_pos(m).cast(), a, b, 0, 0)
        });
        sequence_add_task(seq(i), go);
        let h = crate::common::new_task(0x18, |t| {
            achieve_heading_ctor(t, rd::<f32>(m, 0x558), g::<u32>(0x86FC7C).read(), g::<u32>(0x86FC80).read())
        });
        sequence_add_task(seq(i), h);
        wr::<u32>(seq(i), 0x30, 1);
    }
    if n > 0 {
        let nf = xi(n as i32).to_f32();
        for _ in 0..n {
            let r = rand_at(0x5F8365) & 0xFFFF;
            let talker = ftol32(x(nf).fmul(xi(r).fmul_m(g::<f32>(0x858B14).read())));
            let r = rand_at(0x5F838E) & 0xFFFF;
            let time = 3000 - ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x859A90).read()));
            for (b, &slot) in list[..n].iter().enumerate() {
                let t = if talker == b as i32 {
                    let m = task_new(0x34);
                    if m.is_null() {
                        GPtr::NULL
                    } else {
                        run_timed_anim_ctor2(m, 0, 0xC, 0x4080_0000, 0xC080_0000, time as u32, 0x1A0, 0x86C7B4, 0);
                        wr::<u32>(m, 0, 0x86C78C);
                        m
                    }
                } else {
                    crate::common::new_task(0x20, |m| stand_still_ctor(m, time, 0, 0, 8.0))
                };
                sequence_add_task(seq(slot), t);
            }
        }
    }
    for &i in &list[..n] {
        let pair = group.byte_add(0x21C + i as isize * 0x14);
        if ped.is_null() || rd::<GPtr<u8>>(pair, 0) == ped {
            wr::<GPtr<u8>>(pair, 4, crate::group_scan_loops::sequence_clone(seq(i)));
        }
    }
    eh_vector_dtor(sp, 0x40, 8, 0x6389F0);
}

/// The batch's hooks.
pub const HOOKS: &[HookReg] = &[
    HOOK_MEDIC_TREAT_INJURED_PED_CREATE_NEXT_SUB_TASK,
    HOOK_MEDIC_TREAT_INJURED_PED_CONTROL_SUB_TASK,
    HOOK_GO_PICK_UP_ENTITY_CREATE_NEXT_SUB_TASK,
    HOOK_GO_PICK_UP_ENTITY_CONTROL_SUB_TASK,
    HOOK_GO_PICK_UP_ENTITY_CREATE_FIRST_SUB_TASK,
    HOOK_GO_TO_POINT_AIMING_CONTROL_SUB_TASK,
    HOOK_FOLLOW_PATROL_ROUTE_CREATE_FIRST_SUB_TASK,
    HOOK_SEEK_COVER_UNTIL_TARGET_DEAD_CONTROL_SUB_TASK,
    HOOK_KILL_PED_ON_FOOT_STEALTH_CONTROL_SUB_TASK,
    HOOK_TASK_TIMER_GET_TIME_LEFT,
    HOOK_SUNBATHE_CREATE_SUB_TASK,
    HOOK_SET_VALUE_AND_RESTART_TIMER_6955D0,
    HOOK_DIE_IN_CAR_PREPARE_PED_VEHICLE_FOR_PED_DEATH,
    HOOK_SUNBATHE_CREATE_FIRST_SUB_TASK,
    HOOK_SUNBATHE_CREATE_NEXT_SUB_TASK,
    HOOK_DIE_IN_CAR_CREATE_FIRST_SUB_TASK,
    HOOK_FLEE_POINT_SET_FLEE_POSITION,
    HOOK_FLEE_ENTITY_CONTROL_SUB_TASK_65BBB0,
    HOOK_FOLLOW_LEADER_IN_FORMATION_CONTROL_SUB_TASK,
    HOOK_GO_TO_POINT_FINE_SET_TARGET_POS,
    HOOK_TRACK_ENTITY_CALC_TARGET_POS,
    HOOK_TRACK_ENTITY_CALC_MOVE_RATIO,
    HOOK_TRACK_ENTITY_CONTROL_SUB_TASK,
    HOOK_CHOKING_PROCESS_PED,
    HOOK_HIT_REACTION_CREATE_SUB_TASK,
    HOOK_HIT_REACTION_CREATE_FIRST_SUB_TASK,
    HOOK_SEEK_87004C_CREATE_SUB_TASK,
    HOOK_SEEK_87004C_CREATE_FIRST_SUB_TASK,
    HOOK_SEEK_87004C_CREATE_NEXT_SUB_TASK,
    HOOK_SEEK_87004C_CONTROL_SUB_TASK,
    HOOK_CREATE_CAR_AND_GET_IN_PROCESS_PED,
    HOOK_CHAT_ALLOCATE_DEFAULT_TASKS,
];
