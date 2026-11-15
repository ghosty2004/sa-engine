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
    climb_test_for_climb, membership_count_members, ped_groups_get_peds_group, seek_entity_xy_offset_ctor, stand_still_dtor,
    stand_still_process_ped,
    event_base_dtor, event_group_add, use_gun_ctor,
};
use sa_core::bindings::{
    get_weapon_info, matrix_set_scale, tm_get_task_secondary, tm_set_task_secondary, world_get_is_line_of_sight_clear,
};
use sa_core::crt::rand_at;
use sa_stream::bindings::ped_is_player;
use crate::common::*;
use sa_core::bindings::ped_is_alive;
use sa_world::bindings::{
    achieve_heading_ctor, enter_car_as_passenger_ctor, entity_clean_up_old_reference, entity_register_reference,
    leave_car_ctor, membership_get_leader, membership_get_member, pause_ctor,
    sequence_add_task, sequence_ctor, simple_car_drive_ctor, stand_still_ctor, turn_to_face_entity_ctor, vector_sub,
    attractor_manager_has_empty_slot, get_ped_attractor_manager, get_task_swim, go_to_point_and_stand_still_ctor, ik_look_at,
    matrix_dtor,
    ped_get_weapon_skill, weapon_is_type_melee,
};
use sa_world::world::common::{entity_pos, ped_active_weapon};

use crate::bindings::task_seek_radius_angle_ctor;
orig! {
    /// `InteriorGroup_c::FindClosestInteriorInfo(int32 type, CVector pos, float maxDist, InteriorInfo_t**, Interior_c**,
    /// float*)` (0x594A50, `ret 0x20`)
    fn interior_group_find_closest_info(g: GPtr<u8>, ty: i32, x: f32, y: f32, z: f32, max: f32, info: GPtr<GPtr<u8>>, interior: GPtr<GPtr<u8>>, dist: GPtr<f32>) -> u8 = thiscall 0x594A50 => sa_world::world::interior_closest_info::find_closest_interior_info;
    /// the interior use-info event's constructor (0x674EA0, `ret 0x10`)
    fn event_interior_use_info_ctor(e: GPtr<u8>, info: GPtr<u8>, interior: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x674EA0 => unported;
    /// `CTaskComplexGoToPointAiming::CreateSubTask(int32 type)` (0x6688D0, `ret 4`)
    fn go_to_point_aiming_create_sub_task(t: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x6688D0 => unported;
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

/// `CTaskComplexGoToPointAiming::CreateFirstSubTask(CPed* ped)` (0x66DDB0, `ret 4`, SEH frame). +0x34 bit 0 cleared.
/// The weapon info of the ped's active weapon at its current skill: with flag bit 1 (+0x18) the ped's secondary
/// attack task (slot 0) is told to abort (`MakeAbortable(ped, urgent, null)`) when there is one, else a
/// `CTaskSimpleUseGun(target +0x10, the target position +0x14, command 3 / 3 for task type 0x385, else 1 / 1)`
/// becomes it (`SetTaskSecondary` with a null task too when `new` fails); without bit 1, flag bit 0 and fire type 1
/// or 3 give `CreateSubTask(0x3FC)`. Otherwise (and after the bit-1 case) the result is
/// `CTaskComplexGoToPointAndStandStill(move state +0xC, the point +0x20, radius +0x2C, +0x30, false, false)`, null
/// when `new` fails.
#[hook(addr = 0x66DDB0, conv = thiscall, name = "CTaskComplexGoToPointAiming::CreateFirstSubTask", expect = "64 A1 00 00 00 00 6A FF 68 0C 55 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn go_to_point_aiming_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wr::<u8>(this, 0x34, rd::<u8>(this, 0x34) & 0xFE);
    let wtype = rd::<i32>(ped_active_weapon(ped), 0);
    let info = get_weapon_info(wtype, ped_get_weapon_skill(ped));
    let tm = || rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
    if (rd::<u32>(info, 0x18) >> 1) & 1 != 0 {
        if tm_get_task_secondary(tm(), 0).is_null() {
            let n = if task_get_type(this) == 0x385 { 3 } else { 1 };
            let t = task_new(0x3C);
            let gun = if t.is_null() {
                GPtr::NULL
            } else {
                let (px, py, pz) = (rd::<u32>(this, 0x14), rd::<u32>(this, 0x18), rd::<u32>(this, 0x1C));
                use_gun_ctor(t, rd::<GPtr<u8>>(this, 0x10), px, py, pz, n, n, 0)
            };
            tm_set_task_secondary(tm(), gun, 0);
        } else {
            task_make_abortable(tm_get_task_secondary(tm(), 0), ped, 1, GPtr::NULL);
        }
    } else if rd::<u32>(info, 0x18) & 1 != 0 && matches!(rd::<u32>(info, 0), 1 | 3) {
        return go_to_point_aiming_create_sub_task(this, 0x3FC);
    }
    new_task(0x28, |m| {
        let (r, mr) = (rd::<f32>(this, 0x2C), rd::<f32>(this, 0x30));
        go_to_point_and_stand_still_ctor(m, rd::<u32>(this, 0xC), this.byte_add(0x20).cast(), r, mr, 0, 0)
    })
}

/// `CTaskInteriorBeInShop::ScanForStuff(CPed* ped)` (0x6754D0, `ret 4`, SEH frame). Only while scanning (+0x30): the
/// closest free interior info of any type within 2.0 of the ped in the task's interior group (+0x28); a new one (not
/// +0x2C) is handled by type and then remembered in +0x2C: type 8 with `GetRandomNumberInRange(0, 100) > 50` gives the
/// ped's event group (intelligence +0x68) a stack interior use-info event for it ([`raise_use_info_event`]), else
/// the ped looks at the interior's position for 3000 ms (`g_ikChainMan.LookAt("", ped, null, 3000, -1, &interior->pos,
/// true, 0.25, 500, 3, false)`, the name at 0x870280); type 9 with `> 80` stops scanning (+0x30 = 0) and raises the
/// event.
#[hook(addr = 0x6754D0, conv = thiscall, name = "CTaskInteriorBeInShop::ScanForStuff", expect = "6A FF 68 00 5C 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn be_in_shop_scan_for_stuff(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(this, 0x30) == 0 {
        return;
    }
    let (mut info, mut interior, mut dist) = (GPtr::<u8>::NULL, GPtr::<u8>::NULL, 0f32);
    let p = entity_pos(ped).cast::<f32>();
    let (px, py, pz) = (p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read());
    let found = interior_group_find_closest_info(
        rd::<GPtr<u8>>(this, 0x28), -1, px, py, pz, 2.0,
        GPtr::from_raw(&mut info), GPtr::from_raw(&mut interior), GPtr::from_raw(&mut dist),
    );
    if found == 0 || rd::<GPtr<u8>>(this, 0x2C) == info {
        return;
    }
    match rd::<u8>(info, 0) {
        8 => {
            if general_random_int_in_range(0, 0x64) > 0x32 {
                raise_use_info_event(ped, info, interior);
            } else {
                ik_look_at(
                    g::<u8>(0xC15448), g::<u8>(0x870280), ped, GPtr::NULL, 0xBB8, -1, interior.byte_add(4), 1, 0.25, 0x1F4,
                    3, 0,
                );
            }
        }
        9 => {
            if general_random_int_in_range(0, 0x64) > 0x50 {
                wr::<u8>(this, 0x30, 0);
                raise_use_info_event(ped, info, interior);
            }
        }
        _ => {}
    }
    wr::<GPtr<u8>>(this, 0x2C, info);
}

/// A stack interior use-info event (0x674EA0: `info`, `interior`, 0, 0) added to the ped's event group
/// (`CEventGroup::Add(event, false)`), then destroyed (vtable 0x8701A8 written back, `CEvent`'s destructor 0x4AC010).
fn raise_use_info_event(ped: GPtr<u8>, info: GPtr<u8>, interior: GPtr<u8>) {
    let mut ev = [0u32; 8];
    let e = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
    event_interior_use_info_ctor(e, info, interior, 0, 0);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
    wr::<u32>(e, 0, 0x8701A8);
    event_base_dtor(e);
}

/// `CTaskComplexUseClosestFreeScriptedAttractor::ComputeClosestFreeScriptedEffect(CPed* ped)` (0x634740, thiscall,
/// `ret 4`, SEH frame; the task is not read): among the 64 scripted 2D effects (`CScripted2dEffects`: active flags
/// 0xC3A1A0, the 0x40-byte effects at 0xC3AB00, the 0x24-byte user restrictions at 0xC3A200) the nearest active one
/// the ped may use (unrestricted (+0x20 clear), or the ped's model among the four models, or a -2 model slot whose
/// ped type matches +0x598) with an empty attractor slot (`GetPedAttractorManager()->HasEmptySlot(effect, null)`).
/// The squared distance `(dz^2 + dy^2) + dx^2` (ped - effect on the x87 stack) is stored as an f32 in the argument
/// slot and must be below the best (from FLT_MAX). A closer candidate also builds a stack `CMatrix` (attach matrix
/// and ownership cleared, `SetScale(1.0)`), destroyed after the slot test, as the bytes do.
#[hook(addr = 0x634740, conv = thiscall, name = "CTaskComplexUseClosestFreeScriptedAttractor::ComputeClosestFreeScriptedEffect",
       expect = "6A FF 68 B8 08 84 00", unverified = "frontier2-a8", route = "L2", hazards = "none")]
pub fn use_closest_free_scripted_attractor_compute_closest_free_scripted_effect(_this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut best = f32::from_bits(0x7F7F_FFFF);
    let mut found = GPtr::NULL;
    for i in 0..64usize {
        if g::<u8>(0xC3A1A0 + i).read() == 0 {
            continue;
        }
        let user = 0xC3A200 + i * 0x24;
        if g::<u8>(user + 0x20).read() != 0 {
            let model = i32::from(rd::<i16>(ped, 0x22));
            let by_model = (0..4).any(|k| g::<i32>(user + 4 * k).read() == model);
            let ty = rd::<i32>(ped, 0x598);
            if !by_model && !(0..4).any(|k| g::<i32>(user + 4 * k).read() == -2 && g::<i32>(user + 0x10 + 4 * k).read() == ty) {
                continue;
            }
        }
        let effect = g::<u8>(0xC3AB00 + i * 0x40);
        let (pp, ep) = (entity_pos(ped).cast::<f32>(), effect.cast::<f32>());
        let d = |k: i32| x(pp.wrapping_index(k).read()).fsub_m(ep.wrapping_index(k).read());
        let (dx, dy, dz) = (d(0), d(1), d(2));
        let s = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        let s32 = s.to_f32();
        if !lt(s, x(best)) {
            continue;
        }
        let mut m = [0u32; 0x12];
        let mp = GPtr::from_raw(m.as_mut_ptr()).cast::<u8>();
        matrix_set_scale(mp, 1.0);
        if attractor_manager_has_empty_slot(get_ped_attractor_manager(), effect, GPtr::NULL) != 0 {
            best = s32;
            found = effect;
        }
        matrix_dtor(mp);
    }
    found
}

/// `CWorld::GetIsLineOfSightClear(a, b, buildings, vehicles, false, objects, false, false, false)` between two stack
/// points, as `CalcFall` tests its probes.
fn fall_probe_clear(a: &mut [f32; 3], b: &mut [f32; 3]) -> u8 {
    let (pa, pb) = (GPtr::from_raw(a.as_mut_ptr()).cast(), GPtr::from_raw(b.as_mut_ptr()).cast());
    world_get_is_line_of_sight_clear(pa, pb, 1, 1, 0, 1, 0, 0, 0)
}

/// `CTaskComplexFallToDeath::CalcFall(CPed* ped, int32& dir, bool& fallToDeath)` (0x6796C0, cdecl). Not in a vehicle
/// (+0x46C bit 8) nor with +0x40 bit 8: the directions 0..3 shuffled by eight swaps of `rand() & 3` pairs (return
/// addresses 0x67971A / 0x679724); for the first two, `dir` = it and `v` = the matrix forward (0), -right (1),
/// -forward (2) or right (3) (negated per component through f32 stores); the ledge probe from `pos + v * 1.5 +
/// (0, 0, 0.375)` (0x8D2F00 / 0x8D2F08; x rounded to f32 first, y / z unrounded) straight down by 4.0 (0x8D2F04) must be
/// clear, and the probe from there plus `v * 0.7` back to `pos + (0, 0, 0.375)` must be blocked (buildings, vehicles,
/// objects); then `fallToDeath` = the line from `pos - (0, 0, 0.5)` (0x8D2F0C) to `pos + v * 1.5 - (0, 0, 0.5) + v *
/// 0.7` is blocked and AL = 1. Otherwise AL = 0.
#[hook(addr = 0x6796C0, conv = cdecl, name = "CTaskComplexFallToDeath::CalcFall", expect = "81 EC 94 00 00 00 53 8B 9C 24 9C 00 00 00",
       unverified = "frontier2-a8", route = "L2", hazards = "ax-al-return")]
pub fn fall_to_death_calc_fall(ped: GPtr<u8>, dir: GPtr<i32>, fall_to_death: GPtr<u8>) -> u8 {
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 || rd::<u32>(ped, 0x40) & 0x100 != 0 {
        return 0;
    }
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let mut dirs = [0i32, 1, 2, 3];
    for _ in 0..8 {
        let i = (rand_at(0x67971A) & 3) as usize;
        let j = (rand_at(0x679724) & 3) as usize;
        dirs.swap(i, j);
    }
    let k = |a: usize| g::<f32>(a).read();
    let pos = || entity_pos(ped).cast::<f32>();
    let mut v = [0f32; 3];
    for &d in &dirs[..2] {
        dir.write(d);
        let row = |o: isize, neg: bool| {
            let r = [rd::<f32>(m, o), rd::<f32>(m, o + 4), rd::<f32>(m, o + 8)];
            if neg { r.map(|c| x(c).fmul_m(k(0x858C1C)).to_f32()) } else { r }
        };
        match d {
            0 => v = row(0x10, false),
            1 => v = row(0, true),
            2 => v = row(0x10, true),
            3 => v = row(0, false),
            _ => {}
        }
        let ax = x(v[0]).fmul_m(k(0x8D2F00)).to_f32();
        let kz = k(0x8D2F08);
        let ay = x(v[1]).fmul_m(k(0x8D2F00));
        let az = x(v[2]).fmul_m(k(0x8D2F00));
        let p = pos();
        let (px, py) = (p.read(), p.wrapping_index(1).read());
        let top = x(kz).fadd_m(p.wrapping_index(2).read()).to_f32();
        let sx = x(px).fadd_m(ax).to_f32();
        let sy = x(py).fadd(ay).to_f32();
        let sz = x(top).fadd(az).to_f32();
        let ez = x(sz).fsub_m(k(0x8D2F04)).to_f32();
        if fall_probe_clear(&mut [sx, sy, sz], &mut [sx, sy, ez]) == 0 {
            continue;
        }
        let k2 = k(0x858CB0);
        let (bx, by) = (x(v[0]).fmul_m(k2), x(v[1]).fmul_m(k2));
        let bz = x(v[2]).fmul_m(k2).to_f32();
        let mut back = [bx.fadd_m(sx).to_f32(), by.fadd_m(sy).to_f32(), x(bz).fadd_m(sz).to_f32()];
        let p = pos();
        let (px, py) = (p.read(), p.wrapping_index(1).read());
        let hz = x(k(0x8D2F08)).fadd_m(p.wrapping_index(2).read()).to_f32();
        if fall_probe_clear(&mut back, &mut [px, py, hz]) == 0 {
            continue;
        }
        let k5 = k(0x8D2F0C);
        let p = pos();
        let mut from = [p.read(), p.wrapping_index(1).read(), x(p.wrapping_index(2).read()).fsub_m(k5).to_f32()];
        let (axk, ayk) = (x(v[0]).fmul_m(k(0x8D2F00)), x(v[1]).fmul_m(k(0x8D2F00)));
        let azk = x(v[2]).fmul_m(k(0x8D2F00)).to_f32();
        let p = pos();
        let low = x(p.wrapping_index(2).read()).fsub_m(k5).to_f32();
        let fx = x(p.read()).fadd(axk).to_f32();
        let fy = x(p.wrapping_index(1).read()).fadd(ayk);
        let fz = x(low).fadd_m(azk);
        let (cx, cy) = (x(v[0]).fmul_m(k2), x(v[1]).fmul_m(k2));
        let cz = x(v[2]).fmul_m(k2).to_f32();
        let mut to = [cx.fadd_m(fx).to_f32(), cy.fadd(fy).to_f32(), x(cz).fadd(fz).to_f32()];
        let r = fall_probe_clear(&mut from, &mut to);
        fall_to_death.write(u8::from(r == 0));
        return 1;
    }
    0
}

/// `x == 0.0` after `fcomp; test ah, 44h; jp` (ordered equality).
fn eq0(v: f32) -> bool {
    !x(v).fcom(x(0.0)).test_parity(0x44)
}

/// `sqrt(dy^2 + dx^2)` of the swim target (+0x14) minus the ped's position, on the x87 stack (x first).
fn swim_target_dist_2d(this: GPtr<u8>, ped: GPtr<u8>) -> X87 {
    let p = entity_pos(ped).cast::<f32>();
    let dx = x(rd::<f32>(this, 0x14)).fsub_m(p.read());
    let dy = x(rd::<f32>(this, 0x18)).fsub_m(p.wrapping_index(1).read());
    dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt()
}

/// `CTaskSimpleSwim::ProcessControlAI(CPed* ped)` (0x689640, `ret 4`). Swim state (+0xA) 0. With a ped to follow
/// (+0x20): the normalised direction to it (f32 stores, `CVector::Normalise`), the follow distance (+0x50, when 0.0:
/// `(rand() & 3) * 0.5 + 1.5`, return address 0x6896B4) and the target (+0x14) = its position minus direction *
/// distance. With a target other than (0, 0, 0): the aimed heading (+0x55C) = `atan2(-(dx), dy)` (FPATAN), and beyond
/// 1.0 (0.5 for the player; 2D, FSQRT) state 1; then when the followed ped leads the ped's group (the flag kept in the
/// argument slot) and swims in state 2, state 2 beyond 5.0 of the target (`VectorSub`, 2D); without that leader, state
/// 2 when the speed (+0x10) is above 1.0 and the target beyond 2.0. A climb test every 128 frames (`(+0x20 byte +
/// FrameCounter - 4) & 0x7F == 0`; `TestForClimb(ped, +0x3C, +0x48, +0x4C, true)` → state 5) runs when the followed
/// ped is on the ground (+0x46C bit 0) and not +0x40 bit 8, or the ped has +0x478 bit 13, the followed ped not +0x40
/// bit 8 (read through the pointer even when null, as the bytes) and the target within 1.0. +0x2C / +0x30 cleared;
/// unless led or the player, a negative speed halves the ped's +0x4D8 / +0x4DC (0x8D2FF4).
#[hook(addr = 0x689640, conv = thiscall, name = "CTaskSimpleSwim::ProcessControlAI", expect = "83 EC 28 56 8B F1 8B 46 20",
       unverified = "frontier2-a8", route = "L2", hazards = "x87-transcendental")]
pub fn swim_process_control_ai(this: GPtr<u8>, ped: GPtr<u8>) {
    wr::<u16>(this, 0xA, 0);
    let follow = rd::<GPtr<u8>>(this, 0x20);
    if !follow.is_null() {
        let (pp, tp) = (entity_pos(ped).cast::<f32>(), entity_pos(follow).cast::<f32>());
        let d = |k: i32| x(tp.wrapping_index(k).read()).fsub_m(pp.wrapping_index(k).read()).to_f32();
        let (dz, dy, dx) = (d(2), d(1), d(0));
        let mut dir = [dx, dy, dz];
        vector_normalise(GPtr::from_raw(dir.as_mut_ptr()).cast());
        if eq0(rd::<f32>(this, 0x50)) {
            let r = rand_at(0x6896B4) & 3;
            wr::<f32>(this, 0x50, xi(r).fmul_m(g::<f32>(0x858B8C).read()).fadd_m(g::<f32>(0x858CE8).read()).to_f32());
        }
        let f = rd::<f32>(this, 0x50);
        let ox = x(dir[0]).fmul_m(f).to_f32();
        let oy = x(dir[1]).fmul_m(f).to_f32();
        let oz = x(f).fmul_m(dir[2]);
        let tp = entity_pos(rd::<GPtr<u8>>(this, 0x20)).cast::<f32>();
        let tx = x(tp.read()).fsub_m(ox).to_f32();
        let ty = x(tp.wrapping_index(1).read()).fsub_m(oy).to_f32();
        let tz = x(tp.wrapping_index(2).read()).fsub(oz).to_f32();
        wr::<f32>(this, 0x14, tx);
        wr::<f32>(this, 0x18, ty);
        wr::<f32>(this, 0x1C, tz);
    }
    let mut led = false;
    if !(eq0(rd::<f32>(this, 0x14)) && eq0(rd::<f32>(this, 0x18)) && eq0(rd::<f32>(this, 0x1C))) {
        let p = entity_pos(ped).cast::<f32>();
        let dy = x(rd::<f32>(this, 0x18)).fsub_m(p.wrapping_index(1).read());
        let dx = x(rd::<f32>(this, 0x14)).fsub_m(p.read());
        wr::<f32>(ped, 0x55C, dx.fchs().fpatan(dy).to_f32());
        let far = g::<f32>(0x870874).read();
        let thr = if ped_is_player(ped) != 0 { g::<f32>(0x870878).read() } else { far };
        if !swim_target_dist_2d(this, ped).fcom(x(thr)).test_any(0x41) {
            wr::<u16>(this, 0xA, 1);
            let follow = rd::<GPtr<u8>>(this, 0x20);
            let grp = if follow.is_null() { 0 } else { ped_groups_get_peds_group(ped.addr() as u32) };
            if grp != 0
                && membership_get_leader(GPtr::from_addr(ped_groups_get_peds_group(ped.addr() as u32) as usize + 8))
                    == rd::<GPtr<u8>>(this, 0x20)
            {
                led = true;
                let leader = rd::<GPtr<u8>>(this, 0x20);
                if !get_task_swim(rd::<GPtr<u8>>(leader, 0x47C)).is_null() {
                    let ts = get_task_swim(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x20), 0x47C));
                    if rd::<u16>(ts, 0xA) == 2 {
                        let mut out = [0f32; 3];
                        let o = vector_sub(GPtr::from_raw(out.as_mut_ptr()).cast(), this.byte_add(0x14), entity_pos(ped)).cast::<f32>();
                        let (oy, ox) = (x(o.wrapping_index(1).read()), x(o.read()));
                        let s = ox.fmul(ox).fadd(oy.fmul(oy)).fsqrt();
                        if !s.fcom(x(g::<f32>(0x858C80).read())).test_any(0x41) {
                            wr::<u16>(this, 0xA, 2);
                        }
                    }
                }
            } else if !x(rd::<f32>(this, 0x10)).fcom(x(g::<f32>(0x858624).read())).test_any(0x41)
                && !swim_target_dist_2d(this, ped).fcom(x(g::<f32>(0x858CA0).read())).test_any(0x41)
            {
                wr::<u16>(this, 0xA, 2);
            }
        }
    }
    let follow = rd::<GPtr<u8>>(this, 0x20);
    let on_ground = !follow.is_null() && rd::<u8>(follow, 0x46C) & 1 != 0 && rd::<u32>(follow, 0x40) & 0x100 == 0;
    let climb = on_ground
        || (rd::<u32>(ped, 0x478) & 0x2000 != 0
            && rd::<u32>(follow, 0x40) & 0x100 == 0
            && !swim_target_dist_2d(this, ped).fcom(x(g::<f32>(0x858624).read())).test_parity(5));
    if climb
        && (u32::from(rd::<u8>(ped, 0x20)).wrapping_add(g::<u32>(0xB7CB4C).read()).wrapping_sub(4)) & 0x7F == 0
        && !climb_test_for_climb(ped, this.byte_add(0x3C), this.byte_add(0x48), this.byte_add(0x4C), 1).is_null()
    {
        wr::<u16>(this, 0xA, 5);
    }
    wr::<u32>(this, 0x30, 0);
    wr::<u32>(this, 0x2C, 0);
    if led || ped_is_player(ped) != 0 {
        return;
    }
    if !x(rd::<f32>(this, 0x10)).fcom(x(0.0)).test_parity(5) {
        let k = g::<f32>(0x8D2FF4).read();
        wr::<f32>(ped, 0x4D8, x(k).fmul_m(rd::<f32>(ped, 0x4D8)).to_f32());
        wr::<f32>(ped, 0x4DC, x(k).fmul_m(rd::<f32>(ped, 0x4DC)).to_f32());
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_FOLLOW_LEADER_IN_FORMATION_CREATE_SUB_TASK,
    HOOK_SEEK_COVER_GET_COVER_PED,
    HOOK_SEEK_COVER_UNTIL_TARGET_DEAD_CREATE_FIRST_SUB_TASK,
    HOOK_WAIT_UNTIL_PED_IS_OUT_CAR_PROCESS_PED,
    HOOK_GO_TO_POINT_AIMING_CREATE_FIRST_SUB_TASK,
    HOOK_BE_IN_SHOP_SCAN_FOR_STUFF,
    HOOK_USE_CLOSEST_FREE_SCRIPTED_ATTRACTOR_COMPUTE_CLOSEST_FREE_SCRIPTED_EFFECT,
    HOOK_FALL_TO_DEATH_CALC_FALL,
    HOOK_SWIM_PROCESS_CONTROL_AI,
];
