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
use crate::bindings::task_complex_ctor;
use crate::frontier_a::{find_interior_info, rand_scaled};
use sa_stream::bindings::ped_is_player;
use crate::common::*;
use sa_core::bindings::ped_is_alive;
use sa_world::bindings::{
    achieve_heading_ctor, enter_car_as_passenger_ctor, entity_clean_up_old_reference, entity_register_reference,
    leave_car_ctor, membership_get_leader, membership_get_member, pause_ctor,
    sequence_add_task, sequence_ctor, simple_car_drive_ctor, stand_still_ctor, turn_to_face_entity_ctor, vector_sub,
    attractor_manager_has_empty_slot, get_ped_attractor_manager, get_task_swim, go_to_point_and_stand_still_ctor, ik_look_at,
    matrix_dtor, run_anim_ctor,
    ped_get_weapon_skill, weapon_is_type_melee,
};
use sa_world::world::common::{entity_pos, ped_active_weapon};

use crate::bindings::task_seek_radius_angle_ctor;
use crate::bindings::go_to_point_aiming_ctor;
use crate::bindings::task_trigger_look_at_ctor;
use crate::bindings::throw_projectile_ctor;
use crate::bindings::track_entity_ctor;
use crate::bindings::{climb_ctor, death_dtor, drive_to_point_ctor, go_to_point_ctor, ped_position_ped_out_of_collision, vehicle_is_driver};
use sa_world::bindings::{car_enter_exit_compute_passenger_index_from_car_door, col_sphere_set};
use crate::bindings::{run_anim_ctor2, task_simple_ctor, task_simple_dtor};
use sa_world::bindings::{camera_clear_player_weapon_mode, ped_set_current_weapon, player_clear_weapon_target};
use sa_world::bindings::{find_player_ped, ped_set_ped_state, stats_increment_stat};
use crate::bindings::event_death_ctor;
use crate::bindings::geom_compute_route_round_sphere;
orig! {
    /// `CCarEnterExit::ComputeOppositeDoorFlag(const CVehicle*, int32 door, bool)` (0x64E610, AL)
    fn compute_opposite_door_flag(v: GPtr<u8>, door: i32, b: u32) -> u8 = cdecl 0x64E610 => unported;
    /// `CTask::Clone()` (vtable +4)
    fn v_clone(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CEntitySeekPosCalculator::ComputeEntitySeekPos(const CPed&, const CEntity&, CVector&)` (vtable +0)
    fn v_compute_entity_seek_pos(c: GPtr<u8>, ped: GPtr<u8>, e: GPtr<u8>, out: GPtr<u8>) = thiscall vslot 0x0 => unported;
    /// `CTaskComplexSeekCoverUntilTargetDead::GetCoverPed(CPed*)` (0x622D50, `ret 4`)
    fn get_cover_ped_o(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x622D50 => seek_cover_get_cover_ped;
    /// `CTaskInteriorBeInHouse::GetInfoForPedToUse(CPed*, int32*)` (0x6750A0, `ret 8`)
    fn be_in_house_get_info_o(t: GPtr<u8>, ped: GPtr<u8>, time: GPtr<i32>) = thiscall 0x6750A0 => interior_be_in_house_get_info_for_ped_to_use;
    /// `CTaskInteriorBeInOffice::GetInfoForPedToUse(int32*)` (0x675300, `ret 4`)
    fn be_in_office_get_info_o(t: GPtr<u8>, time: GPtr<i32>) = thiscall 0x675300 => crate::frontier_a::interior_be_in_office_get_info_for_ped_to_use;
    /// `CTaskSimpleIKChain::Constructor(name, bone, CVector offset, part, entity, bone, CVector offset, speed, time,
    /// blend)` (0x6339C0, `ret 0x34`)
    fn ik_chain_ctor(t: GPtr<u8>, name: u32, bone: i32, ox: u32, oy: u32, oz: u32, part: i32, ent: u32, tbone: u32, tx: u32, ty: u32, tz: u32, speed: u32, time: u32, blend: u32) -> GPtr<u8> = thiscall 0x6339C0 => unported;
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
    let cover = get_cover_ped_o(this, ped);
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

/// `[+o, +o + 4, +o + 8]` of a task: a `CVector` passed by value.
fn vec_at(t: GPtr<u8>, o: isize) -> (u32, u32, u32) {
    (rd::<u32>(t, o), rd::<u32>(t, o + 4), rd::<u32>(t, o + 8))
}

/// `CTaskSimpleUseGun::Clone()` (0x622F20): `new CTaskSimpleUseGun(target +0x1C, the target position +0x20, command
/// +0x10, burst length +0x34 (u16), aim-flag +0x3B)`.
#[hook(addr = 0x622F20, conv = thiscall, name = "CTaskSimpleUseGun::Clone", expect = "6A FF 68 1B FA 83 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn use_gun_clone(this: GPtr<u8>) -> GPtr<u8> {
    new_task(0x3C, |m| {
        let (vx, vy, vz) = vec_at(this, 0x20);
        let (cmd, burst, aim) = (rd::<u8>(this, 0x10), rd::<u16>(this, 0x34), rd::<u8>(this, 0x3B));
        use_gun_ctor(m, rd::<GPtr<u8>>(this, 0x1C), vx, vy, vz, u32::from(cmd), u32::from(burst), u32::from(aim))
    })
}

/// `CTaskSimpleThrowProjectile::Clone()` (0x623030): `new CTaskSimpleThrowProjectile(target +0x10, position +0x14)`.
#[hook(addr = 0x623030, conv = thiscall, name = "CTaskSimpleThrowProjectile::Clone", expect = "6A FF 68 5B FA 83 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn throw_projectile_clone(this: GPtr<u8>) -> GPtr<u8> {
    new_task(0x24, |m| {
        let (vx, vy, vz) = vec_at(this, 0x14);
        throw_projectile_ctor(m, rd::<GPtr<u8>>(this, 0x10), vx, vy, vz)
    })
}

/// `CTaskComplexGoToPointAiming::Clone()` (0x66CD80): `new CTaskComplexGoToPointAiming(move state +0xC, &point +0x20,
/// aim entity +0x10, aim position +0x14, radius +0x2C, slow-down distance +0x30)`.
#[hook(addr = 0x66CD80, conv = thiscall, name = "CTaskComplexGoToPointAiming::Clone", expect = "6A FF 68 6B 53 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn go_to_point_aiming_clone(this: GPtr<u8>) -> GPtr<u8> {
    new_task(0x38, |m| {
        let (vx, vy, vz) = vec_at(this, 0x14);
        go_to_point_aiming_ctor(m, rd::<u32>(this, 0xC), this.byte_add(0x20).cast(), rd::<GPtr<u8>>(this, 0x10), vx, vy, vz,
            rd::<u32>(this, 0x2C), rd::<u32>(this, 0x30))
    })
}

/// `CTaskComplexGoToPointShooting::Clone()` (0x66CE10): the go-to-point-aiming constructor with the same arguments
/// (radius and distance through `fld` / `fstp`), then the vtable 0x86FE2C.
#[hook(addr = 0x66CE10, conv = thiscall, name = "CTaskComplexGoToPointShooting::Clone", expect = "6A FF 68 8B 53 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn go_to_point_shooting_clone(this: GPtr<u8>) -> GPtr<u8> {
    new_task(0x38, |m| {
        let d = x(rd::<f32>(this, 0x30)).to_f32().to_bits();
        let r = x(rd::<f32>(this, 0x2C)).to_f32().to_bits();
        let (vx, vy, vz) = vec_at(this, 0x14);
        go_to_point_aiming_ctor(m, rd::<u32>(this, 0xC), this.byte_add(0x20).cast(), rd::<GPtr<u8>>(this, 0x10), vx, vy, vz, r, d);
        wr::<u32>(m, 0, 0x86FE2C);
        m
    })
}

/// `CTaskSimpleTriggerLookAt::Clone()` (vt_86e3cc, 0x634560): `new` of the trigger look-at task (0x634440) with entity
/// +8, time +0xC, bone +0x10, offset +0x14, flag +0x20, speed +0x24, blend +0x28, priority +0x2D (sign-extended); with
/// a bone (+0x10 > -1) but no entity the bone becomes -1 and the time 100.
#[hook(addr = 0x634560, conv = thiscall, name = "vt_86e3cc::Clone", expect = "6A FF 68 9B 08 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn trigger_look_at_clone(this: GPtr<u8>) -> GPtr<u8> {
    let (mut time, mut bone) = (rd::<u32>(this, 0xC), rd::<i32>(this, 0x10));
    if bone > -1 && rd::<u32>(this, 8) == 0 {
        bone = -1;
        time = 0x64;
    }
    new_task(0x30, |m| {
        let (vx, vy, vz) = vec_at(this, 0x14);
        task_trigger_look_at_ctor(m, rd::<GPtr<u8>>(this, 8), time, bone as u32, vx, vy, vz, u32::from(rd::<u8>(this, 0x20)),
            rd::<u32>(this, 0x24), rd::<u32>(this, 0x28), i32::from(rd::<i8>(this, 0x2D)) as u32)
    })
}

/// `CTaskComplexTrackEntity::Clone()` (0x65F4E0): the time left (+0x38 set: `+0x34 - now + +0x30`, else +0x20) and
/// `new CTaskComplexTrackEntity(entity +0xC, offset +0x10, move state +0x1C, time, +0x24, +0x28, +0x2C)`; +0x3C copied
/// to the result (through it even when `new` failed, as the bytes).
#[hook(addr = 0x65F4E0, conv = thiscall, name = "CTaskComplexTrackEntity::Clone", expect = "6A FF 68 8B 44 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn track_entity_clone(this: GPtr<u8>) -> GPtr<u8> {
    let time = if rd::<u8>(this, 0x38) != 0 {
        rd::<u32>(this, 0x34).wrapping_sub(g::<u32>(0xB7CB84).read()).wrapping_add(rd::<u32>(this, 0x30))
    } else {
        rd::<u32>(this, 0x20)
    };
    let t = new_task(0x58, |m| {
        let (vx, vy, vz) = vec_at(this, 0x10);
        track_entity_ctor(m, rd::<GPtr<u8>>(this, 0xC), vx, vy, vz, u32::from(rd::<u8>(this, 0x1C)), time as i32,
            rd::<u32>(this, 0x24), rd::<u32>(this, 0x28), u32::from(rd::<u8>(this, 0x2C)))
    });
    wr::<u32>(t, 0x3C, rd::<u32>(this, 0x3C));
    t
}

/// `CTaskSimpleIKChain::Clone()` (0x633B00): `new CTaskSimpleIKChain("", bone +0x18, offset +0x1C, part +0x16, entity
/// +0x28, bone +0x2C, offset +0x30, speed +0x3C, time +8, blend +0xC)` (the bone tags sign-extended); with a chain
/// (+0x10) the four dwords +0x44..+0x50 are copied to the result (through it even when `new` failed).
#[hook(addr = 0x633B00, conv = thiscall, name = "CTaskSimpleIKChain::Clone", expect = "6A FF 68 BB 07 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn ik_chain_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = new_task(0x58, |m| {
        let (ox, oy, oz) = vec_at(this, 0x1C);
        let (tx, ty, tz) = vec_at(this, 0x30);
        ik_chain_ctor(m, 0x858B54, i32::from(rd::<i16>(this, 0x18)), ox, oy, oz, i32::from(rd::<i16>(this, 0x16)),
            rd::<u32>(this, 0x28), rd::<u32>(this, 0x2C), tx, ty, tz, rd::<u32>(this, 0x3C), rd::<u32>(this, 8),
            rd::<u32>(this, 0xC))
    });
    if rd::<u32>(this, 0x10) != 0 {
        for o in [0x44, 0x48, 0x4C, 0x50] {
            wr::<u32>(t, o, rd::<u32>(this, o));
        }
    }
    t
}

/// `CTaskInteriorBeInHouse::GetInfoForPedToUse(CPed* ped, int32* time)` (0x6750A0, `ret 8`). Ped types 7..16 (gang
/// members): the thresholds 0 / 100, others 90 / 10. +0x14 (the info) cleared; `r` = `rand` scaled to 100 (0x6750ED).
/// Below the first threshold: info type 3 or 4 (`rand() < 0x3FFF`, 0x675118, picks which is tried first, the other
/// as fallback) and `*time = -1`; then while no info: below the sum of both, type 1 (`*time = -1`) and then type 5
/// (`*time = 5000 - rand scaled to 25000`, 0x6751B8); then type 1 and type 2 (`*time = -1` each). Every lookup is
/// `FindInteriorInfo(type, &+0x14, &+0x10)` on the interior group (+0xC).
#[hook(addr = 0x6750A0, conv = thiscall, name = "CTaskInteriorBeInHouse::GetInfoForPedToUse", expect = "8B 44 24 04 8B 80 98 05 00 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn interior_be_in_house_get_info_for_ped_to_use(this: GPtr<u8>, ped: GPtr<u8>, time: GPtr<i32>) {
    let pt = rd::<i32>(ped, 0x598);
    let (a, b) = if (7..=0x10).contains(&pt) { (0, 0x64) } else { (0x5A, 0xA) };
    wr::<u32>(this, 0x14, 0);
    let find = |ty: i32| find_interior_info(rd::<GPtr<u8>>(this, 0xC), ty, this.byte_add(0x14), this.byte_add(0x10));
    let none = || rd::<u32>(this, 0x14) == 0;
    let r = rand_scaled(0x6750ED, 0x858628);
    if r < a {
        let (first, second) = if rand_at(0x675118) < 0x3FFF { (3, 4) } else { (4, 3) };
        if find(first) == 0 {
            find(second);
        }
        time.write(-1);
    }
    if none() && r < a + b {
        find(1);
        time.write(-1);
    }
    if none() && r < b + a {
        find(5);
        time.write(0x1388 - rand_scaled(0x6751B8, 0x870214));
    }
    if none() {
        find(1);
        time.write(-1);
        if none() {
            find(2);
            time.write(-1);
        }
    }
}

/// The `CTaskInteriorGoToInfo` the interior tasks build inline (0x1C bytes, `CTaskComplex` then vtable 0x8702E8): +0xC
/// the info, +0x10 the interior, +0x14 the time, +0x18 the flag; null when `new` fails.
fn new_go_to_info(info: u32, interior: u32, time: u32, flag: u8) -> GPtr<u8> {
    new_task(0x1C, |m| {
        task_complex_ctor(m.addr() as u32);
        wr::<u32>(m, 0, 0x8702E8);
        wr::<u32>(m, 0xC, info);
        wr::<u32>(m, 0x10, interior);
        wr::<u32>(m, 0x14, time);
        wr::<u8>(m, 0x18, flag);
        m
    })
}

/// `CTaskInteriorBeInHouse::CreateFirstSubTask(CPed* ped)` (0x6762B0, `ret 4`): `GetInfoForPedToUse(ped, &time)` (the
/// time kept in the argument slot), then the go-to-info task (info +0x14, interior +0x10, time, flag 1).
#[hook(addr = 0x6762B0, conv = thiscall, name = "CTaskInteriorBeInHouse::CreateFirstSubTask", expect = "6A FF 68 BB 5D 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn interior_be_in_house_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut time = ped.addr() as i32;
    be_in_house_get_info_o(this, ped, GPtr::from_raw(&mut time));
    new_go_to_info(rd::<u32>(this, 0x14), rd::<u32>(this, 0x10), time as u32, 1)
}

/// `CTaskInteriorBeInHouse::CreateNextSubTask(CPed* ped)` (0x676200, `ret 4`): ped +0x474 bit 7 set, then as
/// [`interior_be_in_house_create_first_sub_task`] with flag 0.
#[hook(addr = 0x676200, conv = thiscall, name = "CTaskInteriorBeInHouse::CreateNextSubTask", expect = "6A FF 68 9B 5D 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn interior_be_in_house_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x80);
    let mut time = ped.addr() as i32;
    be_in_house_get_info_o(this, ped, GPtr::from_raw(&mut time));
    new_go_to_info(rd::<u32>(this, 0x14), rd::<u32>(this, 0x10), time as u32, 0)
}

/// `CTaskInteriorBeInOffice::CreateNextSubTask(CPed* ped)` (0x676350, `ret 4`): ped +0x474 bit 7 set,
/// `GetInfoForPedToUse(&time)` (0x675300; the time in the argument slot), then the go-to-info task (info +0x14,
/// interior +0x10, time, flag 0).
#[hook(addr = 0x676350, conv = thiscall, name = "CTaskInteriorBeInOffice::CreateNextSubTask", expect = "6A FF 68 DB 5D 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn interior_be_in_office_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x80);
    let mut time = ped.addr() as i32;
    be_in_office_get_info_o(this, GPtr::from_raw(&mut time));
    new_go_to_info(rd::<u32>(this, 0x14), rd::<u32>(this, 0x10), time as u32, 0)
}

/// `CTaskInteriorShopKeeper::ControlSubTask(CPed*)` (0x675740, `ret 4`): once (+0xC, cleared) a
/// `CTaskSimpleRunAnim(group 0x56, anim 0x147, 4.0, false)`; otherwise the current sub-task (+8).
#[hook(addr = 0x675740, conv = thiscall, name = "CTaskInteriorShopKeeper::ControlSubTask", expect = "6A FF 68 3B 5C 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn interior_shop_keeper_control_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(this, 0xC) == 0 {
        return rd::<GPtr<u8>>(this, 8);
    }
    wr::<u8>(this, 0xC, 0);
    new_task(0x20, |m| run_anim_ctor(m, 0x56, 0x147, 4.0, 0))
}

/// `CTaskInteriorShopKeeper::CreateFirstSubTask(CPed*)` (0x676550, `ret 4`): without an info (+0x18) the interior
/// (+0x10) looks one of type 10 up (`FindInteriorInfo(10, &+0x18, &+0x1C)`); the info is marked used (+1 = 1); the
/// go-to-info task (info +0x18, interior +0x1C, time 0) with flag 0 when +0x14 was set, else +0x14 = 1 and flag 1.
#[hook(addr = 0x676550, conv = thiscall, name = "CTaskInteriorShopKeeper::CreateFirstSubTask", expect = "6A FF 68 46 5E 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn interior_shop_keeper_create_first_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(this, 0x18) == 0 {
        find_interior_info(rd::<GPtr<u8>>(this, 0x10), 0xA, this.byte_add(0x18), this.byte_add(0x1C));
    }
    wr::<u8>(rd::<GPtr<u8>>(this, 0x18), 1, 1);
    let flag = if rd::<u8>(this, 0x14) != 0 {
        0
    } else {
        wr::<u8>(this, 0x14, 1);
        1
    };
    let t = task_new(0x1C);
    if t.is_null() {
        return GPtr::NULL;
    }
    let (interior, info) = (rd::<u32>(this, 0x1C), rd::<u32>(this, 0x18));
    task_complex_ctor(t.addr() as u32);
    wr::<u8>(t, 0x18, flag);
    wr::<u32>(t, 0, 0x8702E8);
    wr::<u32>(t, 0xC, info);
    wr::<u32>(t, 0x10, interior);
    wr::<u32>(t, 0x14, 0);
    t
}

/// `vt_870480::MakeAbortable(CPed* ped, int32 priority, const CEvent* event)` (0x678AA0, `ret 0xC`): urgent (2) is
/// passed on to the sub-task (+8, a tail call); otherwise the sub-task is told the same unless it is a 0xCF task;
/// AL = its answer for priority 1 (0 for a 0xCF sub-task), 0 for any other priority.
#[hook(addr = 0x678AA0, conv = thiscall, name = "vt_870480::MakeAbortable", expect = "56 57 8B 7C 24 10",
       unverified = "frontier2-a8", route = "L2", hazards = "ax-al-return")]
pub fn vt_870480_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: i32, event: GPtr<u8>) -> u8 {
    let sub = || rd::<GPtr<u8>>(this, 8);
    if prio == 2 {
        return task_make_abortable(sub(), ped, 2, event);
    }
    if task_get_type(sub()) == 0xCF {
        return 0;
    }
    let r = task_make_abortable(sub(), ped, prio, event);
    if prio == 1 { r } else { 0 }
}

/// `CTaskComplexSeekEntityAnyMeansXYOffset::CreateFirstSubTask(CPed* ped)` (0x6960B0, `ret 4`): the seek position from
/// the calculator at +0x24 (`ComputeEntitySeekPos(ped, entity +0xC, &pos)`); +0x18 = now, +0x1C = +0x10, +0x20 = 1;
/// then `CTaskComplexDriveToPoint(null, &pos, 30.0, 0, -1, +0x14, 0)` (the radius kept in the argument slot).
#[hook(addr = 0x6960B0, conv = thiscall, name = "CTaskComplexSeekEntityAnyMeansXYOffset::CreateFirstSubTask", expect = "6A FF 68 BB 77 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes,uninit-locals",
       note = "the position stays uninitialised when the calculator writes nothing")]
pub fn seek_entity_any_means_xy_offset_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut pos = [0u32; 3];
    let pp = GPtr::from_raw(pos.as_mut_ptr()).cast::<u8>();
    v_compute_entity_seek_pos(this.byte_add(0x24), ped, rd::<GPtr<u8>>(this, 0xC), pp);
    let radius = rd::<u32>(this, 0x14);
    wr::<u32>(this, 0x18, g::<u32>(0xB7CB84).read());
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x10));
    wr::<u8>(this, 0x20, 1);
    new_task(0x3C, |m| drive_to_point_ctor(m, GPtr::NULL, pp, 0x41F0_0000, 0, u32::MAX, radius, 0))
}

/// `CTaskSimpleAffectSecondaryBehaviour::ProcessPed(CPed* ped)` (0x691320, `ret 4`): the ped's secondary task in slot
/// +0xC. Adding (+8): a current one must accept `MakeAbortable(ped, 1, null)` (else AL = 0), then a clone of +0x10
/// becomes the slot's task and AL = 1. Removing: a current one gets `MakeAbortable(ped, 0, null)` and AL = 0; none:
/// AL = 1.
#[hook(addr = 0x691320, conv = thiscall, name = "CTaskSimpleAffectSecondaryBehaviour::ProcessPed", expect = "53 56 57 8B 7C 24 10",
       unverified = "frontier2-a8", route = "L2", hazards = "ax-al-return,heap-writes")]
pub fn affect_secondary_behaviour_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let tm = |p: GPtr<u8>| rd::<GPtr<u8>>(p, 0x47C).byte_add(4);
    let cur = tm_get_task_secondary(tm(ped), rd::<i32>(this, 0xC));
    if rd::<u8>(this, 8) == 0 {
        if cur.is_null() {
            return 1;
        }
        task_make_abortable(cur, ped, 0, GPtr::NULL);
        return 0;
    }
    if !cur.is_null() && task_make_abortable(cur, ped, 1, GPtr::NULL) == 0 {
        return 0;
    }
    let (t, slot) = (tm(ped), rd::<i32>(this, 0xC));
    tm_set_task_secondary(t, v_clone(rd::<GPtr<u8>>(this, 0x10)), slot);
    1
}

/// `CTaskSimpleDrownInCar::ProcessPed(CPed* ped)` (0x62FF70, `ret 4`, SEH frame): ped state 0x36 (drowned), +0x46C bit
/// 9 cleared; for the player the stat 0xAA is incremented by 1.0 and the vehicle's +0x36 byte becomes `(b & 7) |
/// 0x48`; a stack `CEventDeath(true)` is added to the ped's event group (`Add(event, false)`) and destroyed. AL = 1.
#[hook(addr = 0x62FF70, conv = thiscall, name = "CTaskSimpleDrownInCar::ProcessPed", expect = "64 A1 00 00 00 00 6A FF 68 D8 03 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn drown_in_car_process_ped(_this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    ped_set_ped_state(ped, 0x36);
    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FDFF);
    if find_player_ped(-1) == ped {
        stats_increment_stat(0xAA, 1.0);
    }
    if find_player_ped(-1) == ped {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | 0x48);
    }
    let mut ev = [0u32; 5];
    let e = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
    event_death_ctor(e, 1);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
    death_dtor(e);
    1
}

/// `CTaskComplexUseScriptedAttractor::CreateFirstSubTask(CPed*)` (0x6390C0, `ret 4`): the scripted effect (attractor
/// +0xC, its effect +4) among the 64 at 0xC3AB00 (stride 0x40); none: null. Its id (0xC3A020 + 4 * index) goes into a
/// new 0x1C-byte complex task (vtable 0x86E518; +0xC id, +0x10 0, +0x14 -1, +0x18 0) and the effect pair's user count
/// (0xC178F0 + 0x40 * id + 0x3C) is incremented when `new` succeeds.
#[hook(addr = 0x6390C0, conv = thiscall, name = "CTaskComplexUseScriptedAttractor::CreateFirstSubTask", expect = "6A FF 68 8B 12 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn use_scripted_attractor_create_first_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    let effect = rd::<u32>(rd::<GPtr<u8>>(this, 0xC), 4);
    let Some(i) = (0..0x40u32).find(|&i| 0xC3AB00 + i * 0x40 == effect) else {
        return GPtr::NULL;
    };
    let id = g::<u32>(0xC3A020 + i as usize * 4).read();
    let t = task_new(0x1C);
    if t.is_null() {
        return GPtr::NULL;
    }
    task_complex_ctor(t.addr() as u32);
    wr::<u32>(t, 0xC, id);
    wr::<u32>(t, 0, 0x86E518);
    wr::<u32>(t, 0x10, 0);
    wr::<u32>(t, 0x14, u32::MAX);
    wr::<u32>(t, 0x18, 0);
    let users = (id << 6).wrapping_add(0xC178F0 + 0x3C) as usize;
    g::<u32>(users).write(g::<u32>(users).read().wrapping_add(1));
    t
}

/// `CPedGroupDefaultTaskAllocatorStandStill::AllocateDefaultTasks(CPedGroup* group, CPed* ped)` (0x5F6DA0, `ret 8`):
/// each of the seven members (+0x21C, stride 0x14) and then the leader (+0x2A8) that exists (and is `ped`, when one is
/// given) gets a new 0xC-byte complex task with vtable 0x86C7BC in its default-task slot (+4; null when `new` fails;
/// the old one is not deleted).
#[hook(addr = 0x5F6DA0, conv = thiscall, name = "CPedGroupDefaultTaskAllocatorStandStill::AllocateDefaultTasks", expect = "6A FF 68 66 DF 83 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn default_task_allocator_stand_still_allocate_default_tasks(_this: GPtr<u8>, group: GPtr<u8>, ped: GPtr<u8>) {
    let alloc = || {
        new_task(0xC, |m| {
            task_complex_ctor(m.addr() as u32);
            wr::<u32>(m, 0, 0x86C7BC);
            m
        })
    };
    for k in 0..7 {
        let slot = group.byte_add(0x21C + k * 0x14);
        let p = rd::<GPtr<u8>>(slot, 0);
        if !p.is_null() && (ped.is_null() || p == ped) {
            wr::<GPtr<u8>>(slot, 4, alloc());
        }
    }
    let l = rd::<GPtr<u8>>(group, 0x2A8);
    if !l.is_null() && (ped.is_null() || l == ped) {
        wr::<GPtr<u8>>(group, 0x2AC, alloc());
    }
}

/// `CTaskComplexInWater::CreateNextSubTask(CPed* ped)` (0x635110, `ret 4`): only after a 0x518 sub-task (the climb
/// test) that found something to climb (+0x38): ped +0x46C bit 9 set and a `CTaskSimpleClimb(entity +0x38, &pos +0x3C,
/// heading +0x48, surface +0x4C, height, false)`, the height 2 when `pos.z - ped.z < 1.4` (0x8D2F34) else 1, written
/// into the low byte of the argument slot (the ped pointer's upper bytes stay, as the bytes pass the whole dword).
/// Otherwise null.
#[hook(addr = 0x635110, conv = thiscall, name = "CTaskComplexInWater::CreateNextSubTask", expect = "6A FF 68 6B 09 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn in_water_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if task_get_type(rd::<GPtr<u8>>(this, 8)) != 0x518 {
        return GPtr::NULL;
    }
    let sub = rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(sub, 0x38) == 0 {
        return GPtr::NULL;
    }
    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 0x200);
    let pz = entity_pos(ped).cast::<f32>().wrapping_index(2).read();
    let low = !x(rd::<f32>(sub, 0x44)).fsub_m(pz).fcom(x(g::<f32>(0x8D2F34).read())).test_parity(5);
    let height = (ped.addr() as u32 & 0xFFFF_FF00) | if low { 2 } else { 1 };
    new_task(0x30, |m| {
        climb_ctor(m, rd::<GPtr<u8>>(sub, 0x38), sub.byte_add(0x3C), rd::<u32>(sub, 0x48), u32::from(rd::<u8>(sub, 0x4C)), height, 0)
    })
}

/// `CTaskComplexWalkRoundFire::CreateFirstSubTask(CPed* ped)` (0x6584E0, `ret 4`): the start (+0x2C) = the ped's
/// position; the fire as a stack `CColSphere` (radius +0x1C, centre +0x10, surface 0 / 0 / 0xFF);
/// `ComputeRouteRoundSphere(ped, sphere, start, target +0x20, &newTarget, &detour)`; then
/// `CTaskSimpleGoToPoint(move state +0xC, &detour, the radius at 0x86FC78, false, false)`.
#[hook(addr = 0x6584E0, conv = thiscall, name = "CTaskComplexWalkRoundFire::CreateFirstSubTask", expect = "64 A1 00 00 00 00 6A FF 68 AB 3A 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes,uninit-locals",
       note = "the detour local stays uninitialised when the route test leaves it untouched")]
pub fn walk_round_fire_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let p = entity_pos(ped);
    for k in 0..3 {
        wr::<u32>(this, 0x2C + 4 * k, rd::<u32>(p, 4 * k));
    }
    let mut sphere = [0u32; 5];
    let sp = GPtr::from_raw(sphere.as_mut_ptr()).cast::<u8>();
    col_sphere_set(sp, rd::<u32>(this, 0x1C), this.byte_add(0x10).cast(), 0, 0, 0xFF);
    let (mut new_target, mut detour) = ([0u32; 3], [0u32; 3]);
    let dp = GPtr::from_raw(detour.as_mut_ptr()).cast::<u8>();
    geom_compute_route_round_sphere(ped, sp, this.byte_add(0x2C), this.byte_add(0x20), GPtr::from_raw(new_target.as_mut_ptr()).cast(), dp);
    new_task(0x30, |m| go_to_point_ctor(m, rd::<u32>(this, 0xC), dp, g::<u32>(0x86FC78).read(), 0, 0))
}

/// `CTaskComplexSeekEntityAnyMeansXYOffset::ControlSubTask(CPed* ped)` (0x696160, `ret 4`). Without an entity (+0xC) a
/// new 0x14-byte complex task (vtable 0x858EB0; +0xC 0, +0x10 1, +0x11 0) replaces the sub-task (null when `new` fails).
/// With the timer on (+0x20): a restart flag (+0x21, cleared) restarts it at now (+0x18); once now reaches +0x18 +
/// +0x1C (unsigned) it restarts with the period +0x10 and the sub-task's target (+0x24) gets the calculator's (+0x24)
/// `ComputeEntitySeekPos(ped, entity, &pos)`. The sub-task (+8) stays.
#[hook(addr = 0x696160, conv = thiscall, name = "CTaskComplexSeekEntityAnyMeansXYOffset::ControlSubTask", expect = "64 A1 00 00 00 00 6A FF 68 DB 77 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes,uninit-locals",
       note = "the position stays uninitialised when the calculator writes nothing")]
pub fn seek_entity_any_means_xy_offset_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(this, 0xC) == 0 {
        let t = task_new(0x14);
        if t.is_null() {
            return GPtr::NULL;
        }
        task_complex_ctor(t.addr() as u32);
        wr::<u32>(t, 0xC, 0);
        wr::<u8>(t, 0x11, 0);
        wr::<u32>(t, 0, 0x858EB0);
        wr::<u8>(t, 0x10, 1);
        return t;
    }
    if rd::<u8>(this, 0x20) != 0 {
        if rd::<u8>(this, 0x21) != 0 {
            wr::<u32>(this, 0x18, g::<u32>(0xB7CB84).read());
            wr::<u8>(this, 0x21, 0);
        }
        let end = rd::<u32>(this, 0x1C).wrapping_add(rd::<u32>(this, 0x18));
        let now = g::<u32>(0xB7CB84).read();
        if now >= end {
            wr::<u32>(this, 0x18, now);
            wr::<u32>(this, 0x1C, rd::<u32>(this, 0x10));
            wr::<u8>(this, 0x20, 1);
            let sub = rd::<GPtr<u8>>(this, 8);
            let mut pos = [0u32; 3];
            v_compute_entity_seek_pos(this.byte_add(0x24), ped, rd::<GPtr<u8>>(this, 0xC), GPtr::from_raw(pos.as_mut_ptr()).cast());
            for (k, v) in pos.iter().enumerate() {
                wr::<u32>(sub, 0x24 + 4 * k as isize, *v);
            }
        }
    }
    rd::<GPtr<u8>>(this, 8)
}

/// `vt_86ee94::ProcessPed(CPed* ped)` (0x6477F0, `ret 4`): no vehicle (+8): AL = 1. The driver, or when the door
/// opposite +0xC (`ComputeOppositeDoorFlag(vehicle, door, false)`) is not among the vehicle's door flags (+0x487), or it
/// is but the seats it serves are empty (no driver +0x460 and no passenger in the first
/// `ComputePassengerIndexFromCarDoor(vehicle, door)` slots +0x464): `PositionPedOutOfCollision(door, vehicle, true)`
/// and AL = 1; otherwise AL = 0.
#[hook(addr = 0x6477F0, conv = thiscall, name = "vt_86ee94::ProcessPed", expect = "56 8B F1 8B 4E 08",
       unverified = "frontier2-a8", route = "L2", hazards = "ax-al-return,heap-writes")]
pub fn vt_86ee94_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let veh = rd::<GPtr<u8>>(this, 8);
    if veh.is_null() {
        return 1;
    }
    let out = || {
        ped_position_ped_out_of_collision(ped, rd::<u32>(this, 0xC), rd::<GPtr<u8>>(this, 8), 1);
        1
    };
    if vehicle_is_driver(veh, ped) != 0 {
        return out();
    }
    let flag = compute_opposite_door_flag(rd::<GPtr<u8>>(this, 8), rd::<i32>(this, 0xC), 0);
    let v = rd::<GPtr<u8>>(this, 8);
    if flag & rd::<u8>(v, 0x487) != 0 {
        if rd::<u32>(v, 0x460) != 0 {
            return 0;
        }
        let n = car_enter_exit_compute_passenger_index_from_car_door(v, rd::<i32>(this, 0xC));
        let v = rd::<GPtr<u8>>(this, 8);
        if (0..n.max(0)).any(|i| rd::<u32>(v, 0x464 + 4 * i as isize) != 0) {
            return 0;
        }
    }
    out()
}

/// `CTaskComplexUseMobilePhone::CreateFirstSubTask(CPed* ped)` (0x634C10, `ret 4`, SEH frame): ped +0x478 bit 1 and
/// +0x474 bit 22 set; the player's weapon target cleared; a remembered weapon of 0x37 (+0x70C) becomes the active
/// weapon's type and the ped switches to weapon 0; +0x480's +0x85 byte set; the player's camera weapon mode cleared; a
/// stack `CTaskSimpleStandStill(0, false, false, 8.0)` (built inline: vtable 0x86DD2C) processed once and destroyed;
/// the result `CTaskSimpleRunAnim(group 0, anim 0x91, 4.0, task 0x642, the name at 0x86E498, false)`.
#[hook(addr = 0x634C10, conv = thiscall, name = "CTaskComplexUseMobilePhone::CreateFirstSubTask", expect = "64 A1 00 00 00 00 6A FF 68 13 09 84 00",
       unverified = "frontier2-a8", route = "L2", hazards = "heap-writes")]
pub fn use_mobile_phone_create_first_sub_task(_this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let (f478, f474) = (rd::<u32>(ped, 0x478), rd::<u32>(ped, 0x474));
    wr::<u32>(ped, 0x478, f478 | 2);
    wr::<u32>(ped, 0x474, f474 | 0x40_0000);
    if ped_is_player(ped) != 0 {
        player_clear_weapon_target(ped);
    }
    if rd::<u32>(ped, 0x70C) == 0x37 {
        wr::<u32>(ped, 0x70C, rd::<u32>(ped_active_weapon(ped), 0));
        ped_set_current_weapon(ped, 0);
    }
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if !pd.is_null() {
        wr::<u8>(pd, 0x85, 1);
    }
    if ped_is_player(ped) != 0 {
        camera_clear_player_weapon_mode(g::<u8>(0xB6F028));
    }
    let mut stand = [0u32; 8];
    let st = GPtr::from_raw(stand.as_mut_ptr()).cast::<u8>();
    task_simple_ctor(st);
    wr::<u32>(st, 0, 0x86DD2C);
    for o in [8, 0xC, 0x10] {
        wr::<u32>(st, o, 0);
    }
    for o in [0x14, 0x15, 0x18, 0x19] {
        wr::<u8>(st, o, 0);
    }
    wr::<u32>(st, 0x1C, 0x4100_0000);
    stand_still_process_ped(st, ped);
    let t = new_task(0x20, |m| run_anim_ctor2(m, 0, 0x91, 0x4080_0000, 0x642, 0x86E498, 0));
    wr::<u32>(st, 0, 0x86DD2C);
    task_simple_dtor(st);
    t
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
    HOOK_USE_GUN_CLONE,
    HOOK_THROW_PROJECTILE_CLONE,
    HOOK_GO_TO_POINT_AIMING_CLONE,
    HOOK_GO_TO_POINT_SHOOTING_CLONE,
    HOOK_TRIGGER_LOOK_AT_CLONE,
    HOOK_TRACK_ENTITY_CLONE,
    HOOK_IK_CHAIN_CLONE,
    HOOK_INTERIOR_BE_IN_HOUSE_GET_INFO_FOR_PED_TO_USE,
    HOOK_INTERIOR_BE_IN_HOUSE_CREATE_FIRST_SUB_TASK,
    HOOK_INTERIOR_BE_IN_HOUSE_CREATE_NEXT_SUB_TASK,
    HOOK_INTERIOR_BE_IN_OFFICE_CREATE_NEXT_SUB_TASK,
    HOOK_INTERIOR_SHOP_KEEPER_CONTROL_SUB_TASK,
    HOOK_INTERIOR_SHOP_KEEPER_CREATE_FIRST_SUB_TASK,
    HOOK_VT_870480_MAKE_ABORTABLE,
    HOOK_SEEK_ENTITY_ANY_MEANS_XY_OFFSET_CREATE_FIRST_SUB_TASK,
    HOOK_AFFECT_SECONDARY_BEHAVIOUR_PROCESS_PED,
    HOOK_DROWN_IN_CAR_PROCESS_PED,
    HOOK_USE_SCRIPTED_ATTRACTOR_CREATE_FIRST_SUB_TASK,
    HOOK_DEFAULT_TASK_ALLOCATOR_STAND_STILL_ALLOCATE_DEFAULT_TASKS,
    HOOK_IN_WATER_CREATE_NEXT_SUB_TASK,
    HOOK_WALK_ROUND_FIRE_CREATE_FIRST_SUB_TASK,
    HOOK_SEEK_ENTITY_ANY_MEANS_XY_OFFSET_CONTROL_SUB_TASK,
    HOOK_VT_86EE94_PROCESS_PED,
    HOOK_USE_MOBILE_PHONE_CREATE_FIRST_SUB_TASK,
];
