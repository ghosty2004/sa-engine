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
use crate::bindings::pad_jump_just_down;
use sa_input::bindings::pad_sprint_just_down;
use crate::common::{task_make_abortable, task_new, xi};
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

/// The batch's hooks.
pub const HOOKS: &[HookReg] = &[
    HOOK_MEDIC_TREAT_INJURED_PED_CREATE_NEXT_SUB_TASK,
    HOOK_MEDIC_TREAT_INJURED_PED_CONTROL_SUB_TASK,
    HOOK_GO_PICK_UP_ENTITY_CREATE_NEXT_SUB_TASK,
    HOOK_GO_PICK_UP_ENTITY_CONTROL_SUB_TASK,
    HOOK_GO_PICK_UP_ENTITY_CREATE_FIRST_SUB_TASK,
];
