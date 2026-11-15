//! FRONTIER-2 turn 5, lane A (batch `frontier2-a8`): AI frontier entries ported from the 1.0 US bytes (gta-reversed
//! read as an aid; where it differs the bytes win, noted per function).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::bindings::{general_get_radian_angle_between_points, general_limit_radian_angle, general_random_int_in_range};
use sa_core::bindings::{task_new, vector_normalise};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::{lt, st0};

use crate::bindings::{
    membership_count_members, ped_groups_get_peds_group, seek_entity_xy_offset_ctor, stand_still_dtor, stand_still_process_ped,
};
use crate::common::*;
use sa_core::bindings::ped_is_alive;
use sa_world::bindings::{
    achieve_heading_ctor, enter_car_as_passenger_ctor, entity_clean_up_old_reference, entity_register_reference,
    leave_car_ctor, membership_get_leader, membership_get_member, pause_ctor,
    sequence_add_task, sequence_ctor, simple_car_drive_ctor, stand_still_ctor, turn_to_face_entity_ctor, vector_sub,
    weapon_is_type_melee,
};
use sa_world::world::common::{entity_pos, ped_active_weapon};

use crate::bindings::task_seek_radius_angle_ctor;
orig! {
    /// `CTaskComplexSeekCoverUntilTargetDead::GetTargetPed()` (0x622CC0)
    fn seek_cover_get_target_ped(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x622CC0 => crate::group_scan_loops::seek_cover_until_target_dead_get_target_ped;
}

/// `CTaskComplexFollowLeaderInFormation::CreateSubTask(int32 type, CPed* ped)` (0x6962A0, `ret 8`; the entry is a
/// `nop; jmp` to the island 0x401913 that fetches the SEH link and jumps back to 0x6962A6). +0xC the group, +0x10 the
/// leader, +0x14 the formation offset. 0xCA pause (2000); 0xCB stand still (500, 8.0); 0x2BC enter the leader's vehicle
/// as passenger; 0x2C0 leave the ped's vehicle; 0x2C5 simple car drive in it; 0x38B seek the leader (50000, 1000, the
/// radii at 0x86FC2C / 0x86FC30, 2.0) with the offset copied to +0x44 and mode 7 at +0x50 (written through the result
/// even when `new` fails, as the bytes); 0x386 achieve a heading ([`follow_leader_turn_task`]); 0x516 and anything
/// else: null.
#[hook(addr = 0x6962A0, conv = thiscall, name = "CTaskComplexFollowLeaderInFormation::CreateSubTask",
       expect = "90 E9 6D B6 D6 FF", this = "CTaskComplexFollowLeaderInFormation",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn follow_leader_in_formation_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let veh = |p: GPtr<u8>| rd::<GPtr<u8>>(p, 0x58C);
    match ty {
        0xCA => new_task(0x18, |m| pause_ctor(m, 0x7D0)),
        0xCB => new_task(0x20, |m| stand_still_ctor(m, 0x1F4, 0, 0, 8.0)),
        0x2BC => new_task(0x50, |m| enter_car_as_passenger_ctor(m, veh(rd::<GPtr<u8>>(this, 0x10)), 0, 0)),
        0x2C0 => new_task(0x34, |m| leave_car_ctor(m, veh(ped), 0, 0, 1, 0)),
        0x2C5 => new_task(0x60, |m| simple_car_drive_ctor(m, veh(ped), GPtr::NULL, 0)),
        0x386 => follow_leader_turn_task(this, ped),
        0x38B => {
            let m = task_new(0x58);
            let t = if m.is_null() {
                GPtr::NULL
            } else {
                let (r1, r2) = (g::<u32>(0x86FC2C).read(), g::<u32>(0x86FC30).read());
                seek_entity_xy_offset_ctor(m, rd::<GPtr<u8>>(this, 0x10), 0xC350, 0x3E8, r1, 0x40000000, r2, 0, 1)
            };
            for k in 0..3 {
                wr::<u32>(t, 0x44 + k * 4, rd::<u32>(this, 0x14 + k * 4));
            }
            wr::<u32>(t, 0x50, 7);
            t
        }
        _ => GPtr::NULL,
    }
}

/// Case 0x386 of 0x6962A0 (0x69650E): a random member of the group (`GetRandomNumberInRange(0, CountMembers())`; the
/// leader when that is the ped itself). With one and `GetRandomNumberInRange(0, 50) == 20`, a
/// `CTaskSimpleAchieveHeading` towards it: `LimitRadianAngle(GetRadianAngleBetweenPoints(d.x, d.y, 0, 0))` of the
/// normalised `member - ped`, the angle computed before `new`; otherwise one keeping the ped's heading (+0x558). The
/// rate / tolerance at 0x86FC7C / 0x86FC80.
fn follow_leader_turn_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let ms = rd::<GPtr<u8>>(this, 0xC).byte_add(8);
    let n = membership_count_members(ms);
    let r = general_random_int_in_range(0, n);
    let mut other = membership_get_member(ms, r);
    if other == ped {
        other = membership_get_leader(rd::<GPtr<u8>>(this, 0xC).byte_add(8));
    }
    let (rate, tol) = (|| g::<u32>(0x86FC7C).read(), || g::<u32>(0x86FC80).read());
    if !other.is_null() && general_random_int_in_range(0, 0x32) == 0x14 {
        let mut a = [0f32; 3];
        let op = entity_pos(other).cast::<f32>();
        for (k, v) in a.iter_mut().enumerate() {
            *v = op.wrapping_index(k as i32).read();
        }
        let mut d = [0f32; 3];
        let dp = GPtr::from_raw(d.as_mut_ptr()).cast::<u8>();
        vector_sub(dp, GPtr::from_raw(a.as_mut_ptr()).cast(), entity_pos(ped));
        vector_normalise(dp.cast());
        let ang = general_get_radian_angle_between_points(d[0], d[1], 0.0, 0.0).to_f32();
        let m = task_new(0x18);
        if m.is_null() {
            return GPtr::NULL;
        }
        let h = st0(general_limit_radian_angle(ang)).to_f32();
        return achieve_heading_ctor(m, h, rate(), tol());
    }
    new_task(0x18, |m| achieve_heading_ctor(m, rd::<f32>(ped, 0x558), rate(), tol()))
}

/// `CTaskComplexSeekCoverUntilTargetDead::GetCoverPed(CPed* ped)` (0x622D50, thiscall, `ret 4`; the task is not read):
/// the nearest member (slots 0..7) of the ped's group other than the ped that is alive and whose active weapon is
/// not a melee weapon; the squared distance `(dz^2 + dx^2) + dy^2` (ped - member, on the x87 stack) must be below the
/// best so far (from FLT_MAX, stored as an f32). Null without a group.
#[hook(addr = 0x622D50, conv = thiscall, name = "CTaskComplexSeekCoverUntilTargetDead::GetCoverPed", expect = "83 EC 08 53 8B 5C 24 10",
       unverified = "frontier2-a8", route = "L2", hazards = "none")]
pub fn seek_cover_get_cover_ped(_this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut best = f32::from_bits(0x7F7F_FFFF);
    let mut found = GPtr::NULL;
    let grp = ped_groups_get_peds_group(ped.addr() as u32);
    if grp == 0 {
        return GPtr::NULL;
    }
    let ms = GPtr::<u8>::from_addr(grp as usize + 8);
    for i in 0..8 {
        let m = membership_get_member(ms, i);
        if m.is_null() || m == ped || weapon_is_type_melee(ped_active_weapon(m)) != 0 || ped_is_alive(m) as u8 == 0 {
            continue;
        }
        let (mp, pp) = (entity_pos(m).cast::<f32>(), entity_pos(ped).cast::<f32>());
        let d = |k: i32| x(pp.wrapping_index(k).read()).fsub_m(mp.wrapping_index(k).read());
        let (dx, dy, dz) = (d(0), d(1), d(2));
        let s = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
        if lt(s, x(best)) {
            best = s.to_f32();
            found = m;
        }
    }
    found
}

/// `CTaskComplexSeekCoverUntilTargetDead::CreateFirstSubTask(CPed* ped)` (0x62E6C0, `ret 4`, SEH frame). The old
/// target (+0x10) and cover ped (+0x14) references are released; cover = [`seek_cover_get_cover_ped`], target =
/// `GetTargetPed()`; without either: null (the new pointers stay, unregistered). Else both references are registered
/// (target first) and a sequence is built: `CTaskComplexSeekEntityRadiusAngleOffset(cover, 50000, 1000, 1.0, the
/// radii at 0x85A410 / 0x85A414, true, true)` with its calculator's radius 1.5 and angle pi written over +0x44 / +0x48
/// (through the result even when `new` failed, as the bytes), `CTaskComplexTurnToFaceEntityOrCoord(target, the rates
/// at 0x86FC7C / 0x86FC80)`, `CTaskSimpleStandStill(2000, 0, 0, 8.0)`; `AddTask` runs on a null sequence and with
/// null tasks too. The bytes also build the calculator as a stack temporary (vtable 0x85A384) that nothing reads.
#[hook(addr = 0x62E6C0, conv = thiscall, name = "CTaskComplexSeekCoverUntilTargetDead::CreateFirstSubTask", expect = "64 A1 00 00 00 00 6A FF 68 A4 01 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn seek_cover_until_target_dead_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let (target_slot, cover_slot) = (this.byte_add(0x10), this.byte_add(0x14));
    for slot in [target_slot, cover_slot] {
        let e = rd::<GPtr<u8>>(slot, 0);
        if !e.is_null() {
            entity_clean_up_old_reference(e, slot);
        }
    }
    let cover = seek_cover_get_cover_ped(this, ped);
    wr::<GPtr<u8>>(cover_slot, 0, cover);
    let target = seek_cover_get_target_ped(this);
    wr::<GPtr<u8>>(target_slot, 0, target);
    if rd::<u32>(cover_slot, 0) == 0 || target.is_null() {
        return GPtr::NULL;
    }
    entity_register_reference(target, target_slot);
    entity_register_reference(rd::<GPtr<u8>>(cover_slot, 0), cover_slot);
    let seq = new_task(0x40, sequence_ctor);
    let t = task_new(0x54);
    let seek = if t.is_null() {
        GPtr::NULL
    } else {
        let (r1, r2) = (g::<u32>(0x85A410).read(), g::<u32>(0x85A414).read());
        task_seek_radius_angle_ctor(t, rd::<GPtr<u8>>(cover_slot, 0), 0xC350, 0x3E8, 0x3F80_0000, r1, r2, 1, 1)
    };
    wr::<u32>(seek, 0x44, 0x3FC0_0000);
    wr::<u32>(seek, 0x48, 0x4049_0FDB);
    sequence_add_task(seq, seek);
    let turn = new_task(0x28, |m| {
        let (a, b) = (g::<u32>(0x86FC7C).read(), g::<u32>(0x86FC80).read());
        turn_to_face_entity_ctor(m, rd::<GPtr<u8>>(target_slot, 0), a, b)
    });
    sequence_add_task(seq, turn);
    let stand = new_task(0x20, |m| stand_still_ctor(m, 0x7D0, 0, 0, 8.0));
    sequence_add_task(seq, stand);
    seq
}

/// `CTaskSimpleWaitUntilPedIsOutCar::ProcessPed(CPed*)` (0x649550, `ret 4`, SEH frame). No ped to wait for (+8): AL =
/// 1 (EAX = 1). With the first-time flag (+0xC, cleared): a stack `CTaskSimpleStandStill(0, false, false, 8.0)` is
/// processed once for the ped (its result unused) and destroyed, and the ped's aimed heading (+0x55C) becomes
/// `LimitRadianAngle(GetRadianAngleBetweenPoints(+0x10, +0x14, 0, 0))` (both through f32 stores). AL = the waited-for
/// ped is not in a vehicle (+0x46C bit 8 clear), with `flags >> 8`'s upper bits in EAX.
#[hook(addr = 0x649550, conv = thiscall, name = "CTaskSimpleWaitUntilPedIsOutCar::ProcessPed", expect = "64 A1 00 00 00 00 6A FF 68 78 2B 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "ax-al-return")]
pub fn wait_until_ped_is_out_car_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    if rd::<u32>(this, 8) == 0 {
        return 1;
    }
    if rd::<u8>(this, 0xC) != 0 {
        wr::<u8>(this, 0xC, 0);
        let mut stand = [0u32; 8];
        let st = GPtr::from_raw(stand.as_mut_ptr()).cast::<u8>();
        stand_still_ctor(st, 0, 0, 0, 8.0);
        stand_still_process_ped(st, ped);
        let a = general_get_radian_angle_between_points(rd::<f32>(this, 0x10), rd::<f32>(this, 0x14), 0.0, 0.0).to_f32();
        wr::<f32>(ped, 0x55C, st0(general_limit_radian_angle(a)).to_f32());
        stand_still_dtor(st);
    }
    let f = rd::<u32>(rd::<GPtr<u8>>(this, 8), 0x46C) >> 8;
    (f & 0xFFFF_FF00) | (!f & 1)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_FOLLOW_LEADER_IN_FORMATION_CREATE_SUB_TASK,
    HOOK_SEEK_COVER_GET_COVER_PED,
    HOOK_SEEK_COVER_UNTIL_TARGET_DEAD_CREATE_FIRST_SUB_TASK,
    HOOK_WAIT_UNTIL_PED_IS_OUT_CAR_PROCESS_PED,
];
