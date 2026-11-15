//! FRONTIER-2 turn 3, lane A (batch `frontier2-a6`): `AI` entries of the `tools/verify/frontier.py` list that the
//! roadblocks scenario reaches after the gang drive-by (the arrest task's control, the group task allocators) and
//! small transcendental bodies, ported from the 1.0 US bytes (gta-reversed read as an aid).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{get_group_id, group_intel_flush_tasks, task_simple_ctor, task_simple_dtor, tm_find_active_task_by_type};
use sa_core::bindings::{general_random_int_in_range, task_new};
use sa_world::bindings::{crt_printf, dtor_6389f0 as sequence_dtor};
use sa_world::bindings::{
    interesting_events_add, kill_ped_on_foot_ctor, look_about_ctor, membership_get_leader, membership_get_member,
    ped_groups_get_peds_group, sequence_add_task, sequence_ctor,
};
use crate::common::*;
use sa_core::bindings::vector_difference;
use sa_core::field::vec3f_ptr;
use sa_math::fpu::{fbits, lt, st0};
use sa_world::bindings::{
    car_enter_exit_is_room_for_ped_to_leave_car, ped_set_current_weapon, vector_magnitude_sqr, vehicle_is_on_its_side,
    vehicle_is_upside_down, weapon_is_type_melee,
};
use sa_world::world::common::{entity_pos, ped_active_weapon};

use crate::bindings::compute_closest_peds_o;
use crate::bindings::group_intel_set_event_response_task;
use crate::bindings::kill_ped_group_on_foot_ctor;
use crate::bindings::kill_ped_group_on_foot_dtor;
use crate::bindings::ped_do_we_have_weapon_available;
orig! {
    /// virtual `CTask::CreateFirstSubTask(CPed*)` of a complex task (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CWanted::IsClosestCop(CPed*, int32)` (0x5627D0, `ret 8`, AL)
    fn wanted_is_closest_cop(w: GPtr<u8>, cop: GPtr<u8>, n: i32) -> u8 = thiscall 0x5627D0 => unported;
    /// `CTaskComplexFallAndGetUp::IsFalling()` (0x6787D0, AL)
    fn fall_and_get_up_is_falling(t: GPtr<u8>) -> u8 = thiscall 0x6787D0 => unported;
    /// `CTaskComplexFallAndGetUp::SetDownTime(int32)` (0x6787A0, `ret 4`)
    fn fall_and_get_up_set_down_time(t: GPtr<u8>, ms: i32) = thiscall 0x6787A0 => unported;
    /// `CTaskComplexArrestPed::CreateSubTask(int32, CPed*)` (0x68CF80, `ret 8`)
    fn arrest_create_sub_task(t: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x68CF80 => crate::frontier_a_subtasks::arrest_ped_create_sub_task;
    /// `CTaskComplexSeekCoverUntilTargetDead::Constructor(int32 groupId)` (0x622B80, `ret 4`)
    fn seek_cover_ctor_group(t: GPtr<u8>, gid: i32) -> GPtr<u8> = thiscall 0x622B80 => unported;
    /// `CTaskComplexSeekCoverUntilTargetDead::Constructor(CPed*)` (0x622BC0, `ret 4`)
    fn seek_cover_ctor_ped(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x622BC0 => unported;
    /// `CTaskComplexSeekCoverUntilTargetDead::Destructor` (0x622C20)
    fn seek_cover_dtor(t: GPtr<u8>) = thiscall 0x622C20 => unported;
}

/// No task (`TASK_NONE` 0xC8): the arrest control keeps its sub task.
const TASK_NONE: i32 = 0xC8;

/// The arrest control's open-door cases: the suspect's leave-car task (0x2C0) and `|suspect - ped|^2` (computed
/// first, discarded unless needed): within 5 m of a leaving suspect in a vehicle, wait until it is out (0x33D); out
/// of its vehicle, kill it on foot (0x3E8). `None`: the suspect is still in its vehicle.
fn arrest_door_wait(suspect: GPtr<u8>, ped: GPtr<u8>) -> Option<i32> {
    let leave = tm_find_active_task_by_type(rd::<GPtr<u8>>(suspect, 0x47C).byte_add(4), 0x2C0);
    let mut d = [0f32; 3];
    let r = vector_sub(vec3f_ptr(&mut d), entity_pos(suspect), entity_pos(ped));
    let sq = vector_magnitude_sqr(r);
    let in_car = rd::<u32>(suspect, 0x46C) & 0x100 != 0;
    if !leave.is_null() && in_car && lt(sq, sa_math::fpu::x(25.0)) {
        return Some(0x33D);
    }
    if !in_car {
        return Some(0x3E8);
    }
    None
}

/// The arrest control's car-destroy fallback: a ped with a melee weapon switches to the shotgun (0x19) when it has
/// one, else the pistol (0x16); then destroy the car (0x3EB).
fn arrest_destroy_car(ped: GPtr<u8>) -> i32 {
    if weapon_is_type_melee(ped_active_weapon(ped)) != 0 {
        let w = if ped_do_we_have_weapon_available(ped, 0x19) != 0 { 0x19 } else { 0x16 };
        ped_set_current_weapon(ped, w);
    }
    0x3EB
}

/// `CTaskComplexArrestPed::ControlSubTask(CPed*)` (0x68D350, `ret 4`). Without a suspect (+0x10) or with its health
/// at most 0: finish (0x516). With the first-sub-task flag (+0xC): when the sub task (+8) aborts, the task's own
/// `CreateFirstSubTask`. A suspect being arrested (+0x474 bit 0x40) is sought (0x38B) unless the sub task already
/// arrests (0x44C) or seeks it. By the sub task: the open driver door (0x2D2) / passenger door (0x2D8) waits for a
/// leaving suspect, kills one out of its car, and switches to the other door when only that one has room (the
/// passenger door destroys the car when neither has); drag out / destroy car (0x2BF / 0x3EB) kill a suspect out of
/// its car; kill on foot (0x3E8) arrests a falling suspect within 2 m height and 3 m (down time 100000), seeks a
/// farther one, and follows a suspect into a boat / plane / heli (destroy car), or into a car when the ped is not the
/// closest cop with a gun (destroy car), or another car than +0x20 that it is not leaving (drag out of a bike / quad,
/// destroy an upside-down car or one on its side, else open the driver door). Then, for a player suspect, `Say(0xC7)`
/// with one cop in pursuit (`FindPlayerWanted` +0x18; called again otherwise), and the new sub task when one was
/// chosen and the current one aborts (`CreateSubTask` 0x68CF80).
#[hook(addr = 0x68D350, conv = thiscall, name = "CTaskComplexArrestPed::ControlSubTask", expect = "83 EC 2C 55 8B 6C 24 34",
       unverified = "frontier2-a6", route = "L2", hazards = "heap-writes", recs = "police-gang")]
pub fn arrest_ped_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let suspect = rd::<GPtr<u8>>(this, 0x10);
    let sub = rd::<GPtr<u8>>(this, 8);
    let mut new_type = TASK_NONE;
    let mut result = sub;
    if suspect.is_null() || !x(rd::<f32>(suspect, 0x540)).fcom(x(0.0)).test_parity(0x41) {
        new_type = 0x516;
    } else if rd::<u8>(this, 0xC) != 0 {
        if task_make_abortable(sub, ped, 1, GPtr::NULL) != 0 {
            result = v_create_first_sub_task(this, ped);
        }
    } else {
        new_type = arrest_choose(this, ped, suspect, sub);
    }
    let suspect = rd::<GPtr<u8>>(this, 0x10);
    if !suspect.is_null() && ped_is_player(suspect) != 0 {
        if rd::<u8>(find_player_wanted(-1), 0x18) == 1 {
            ped_say(ped, 0xC7, 0, 1.0, 0, 0, 0);
        } else {
            find_player_wanted(-1);
        }
    }
    if new_type != TASK_NONE && task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, GPtr::NULL) != 0 {
        return arrest_create_sub_task(this, new_type, ped);
    }
    result
}

/// The sub task switch of [`arrest_ped_control_sub_task`] (between its `push ebx` / `pop ebx`).
fn arrest_choose(this: GPtr<u8>, ped: GPtr<u8>, suspect: GPtr<u8>, sub: GPtr<u8>) -> i32 {
    if rd::<u8>(suspect, 0x474) & 0x40 != 0 {
        let t = task_get_type(sub);
        if t != 0x44C && task_get_type(rd::<GPtr<u8>>(this, 8)) != 0x38B {
            return 0x38B;
        }
    }
    let in_car = |p: GPtr<u8>| rd::<u32>(p, 0x46C) & 0x100 != 0;
    match task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0x2D2 => {
            let s = rd::<GPtr<u8>>(this, 0x10);
            if let Some(t) = arrest_door_wait(s, ped) {
                return t;
            }
            if car_enter_exit_is_room_for_ped_to_leave_car(rd::<GPtr<u8>>(s, 0x58C), 0xA, GPtr::NULL) != 0 {
                return TASK_NONE;
            }
            let s = rd::<GPtr<u8>>(this, 0x10);
            if car_enter_exit_is_room_for_ped_to_leave_car(rd::<GPtr<u8>>(s, 0x58C), 8, GPtr::NULL) != 0 {
                return 0x2D8;
            }
            TASK_NONE
        }
        0x2D8 => {
            let s = rd::<GPtr<u8>>(this, 0x10);
            if let Some(t) = arrest_door_wait(s, ped) {
                return t;
            }
            if car_enter_exit_is_room_for_ped_to_leave_car(rd::<GPtr<u8>>(s, 0x58C), 8, GPtr::NULL) != 0 {
                return TASK_NONE;
            }
            let s = rd::<GPtr<u8>>(this, 0x10);
            if car_enter_exit_is_room_for_ped_to_leave_car(rd::<GPtr<u8>>(s, 0x58C), 0xA, GPtr::NULL) != 0 {
                return 0x2D2;
            }
            arrest_destroy_car(ped)
        }
        0x2BF | 0x3EB => {
            if in_car(rd::<GPtr<u8>>(this, 0x10)) {
                TASK_NONE
            } else {
                0x3E8
            }
        }
        0x3E8 => {
            let s = rd::<GPtr<u8>>(this, 0x10);
            let fall = tm_find_active_task_by_type(rd::<GPtr<u8>>(s, 0x47C).byte_add(4), 0xD0);
            if !fall.is_null() && fall_and_get_up_is_falling(fall) != 0 {
                let s = rd::<GPtr<u8>>(this, 0x10);
                let mut d = [0f32; 3];
                vector_difference(vec3f_ptr(&mut d), entity_pos(ped), entity_pos(s));
                let dz = x(d[2]).fabs();
                d[2] = 0.0;
                if !dz.fcom(x(2.0)).test_any(0x41) {
                    return 0x38B;
                }
                let sq = vector_magnitude_sqr(vec3f_ptr(&mut d));
                if !x(3.0).fmul_m(3.0).fcom(sq).test_parity(5) {
                    return 0x38B;
                }
                fall_and_get_up_set_down_time(fall, 100000);
                return 0x44C;
            }
            let s = rd::<GPtr<u8>>(this, 0x10);
            if !in_car(s) {
                return TASK_NONE;
            }
            let v = rd::<GPtr<u8>>(s, 0x58C);
            if v.is_null() {
                return TASK_NONE;
            }
            if rd::<u32>(v, 0x590) == 5 || matches!(rd::<u32>(v, 0x594), 3 | 4) {
                return arrest_destroy_car(ped);
            }
            if weapon_is_type_melee(ped_active_weapon(ped)) == 0
                && wanted_is_closest_cop(find_player_wanted(-1), ped, 2) == 0
            {
                return 0x3EB;
            }
            let s = rd::<GPtr<u8>>(this, 0x10);
            if rd::<GPtr<u8>>(this, 0x20) == rd::<GPtr<u8>>(s, 0x58C)
                || !tm_find_active_task_by_type(rd::<GPtr<u8>>(s, 0x47C).byte_add(4), 0x2C0).is_null()
            {
                return TASK_NONE;
            }
            let v = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), 0x58C);
            if rd::<u32>(v, 0x590) == 9 || rd::<u32>(v, 0x594) == 2 {
                return 0x2BF;
            }
            if vehicle_is_upside_down(v) != 0
                || vehicle_is_on_its_side(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), 0x58C)) != 0
            {
                return 0x3EB;
            }
            0x2D2
        }
        _ => TASK_NONE,
    }
}

/// `CEvent::CalcSoundLevelIncrement(float level1, float level2)` (0x4AC050, cdecl, ST0): 0 when `level2` is ±0,
/// `level2` when `level1` is ±0, else `10 * log10((10^(level2/10) + a) / a)` with `a = f32(10^(level1/10))` (`_CIpow`
/// 0x822130, FYL2X with FLDLG2); the original stores `a` into its `level1` argument slot.
#[hook(addr = 0x4AC050, conv = cdecl, name = "CEvent::CalcSoundLevelIncrement", expect = "D9 44 24 08 D8 1D 50 8B",
       unverified = "frontier2-a6", route = "L2", hazards = "x87-transcendental",
       note = "pure, but _CIpow (orig::x87_binary) is the CRT's only in-process; FYL2X; writes its first argument's stack slot")]
pub fn calc_sound_level_increment(level1: f32, level2: f32) -> X87 {
    if !x(level2).fcom(X87::zero()).test_parity(0x44) {
        return X87::zero();
    }
    if !x(level1).fcom(X87::zero()).test_parity(0x44) {
        return x(level2);
    }
    let pow10 = |l: f32| st0(sa_core::orig::x87_binary(0x82_2130, 10.0, x(l).fmul_m(fbits(0x3DCCCCCD)).to_f64()));
    let a = pow10(level1).to_f32();
    let r = pow10(level2).fadd_m(a).fdiv_m(a);
    X87::lg2().fyl2x(r).fmul_m(10.0)
}

/// One event response of the random-group allocator: `main` with a `CTaskSimpleNone` (vtable 0x859CFC) secondary,
/// as the original builds and destroys it around `SetEventResponseTask`.
fn set_response(intel: GPtr<u8>, m: GPtr<u8>, main: GPtr<u8>) {
    let mut none = [0u32; 2];
    let none_p: GPtr<u8> = GPtr::from_raw(none.as_mut_ptr()).cast();
    task_simple_ctor(none_p);
    wr::<u32>(none_p, 0, 0x85_9CFC);
    group_intel_set_event_response_task(intel, m, 1, main, 0, none_p, -1);
    wr::<u32>(none_p, 0, 0x85_9CFC);
    task_simple_dtor(none_p);
}

/// Whether a member (melee weapon) facing a threat with a gun seeks cover instead (both `CWeapon::IsTypeMelee`).
fn seeks_cover(m: GPtr<u8>, threat: GPtr<u8>) -> bool {
    weapon_is_type_melee(ped_active_weapon(m)) != 0 && weapon_is_type_melee(ped_active_weapon(threat)) == 0
}

/// `CTaskAllocatorKillThreatsBasicRandomGroup::AllocateTasks(CPedGroupIntelligence*)` (0x69D460, `ret 4`): as
/// `CTaskAllocatorKillThreatsBasic::AllocateTasks` (0x69D170: the task pairs flushed; the threat's group the same as
/// ours prints 0x870F00; else every non-player member against its closest threat, interesting event 0x18; a lone
/// threat: the kill-on-foot / look-about sequence, event 0x17), except that a member with a melee weapon against a
/// threat without one gets `CTaskComplexSeekCoverUntilTargetDead` (the threat group's id, or the lone threat).
#[hook(addr = 0x69D460, conv = thiscall, name = "CTaskAllocatorKillThreatsBasicRandomGroup::AllocateTasks",
       expect = "55 8B EC 83 E4 F8 6A FF", unverified = "frontier2-a6", route = "L2", hazards = "heap-writes,stack-residue",
       recs = "police-gang", note = "the stack tasks are Rust buffers; the SEH frame is not reproduced")]
pub fn kill_threats_random_group_allocate_tasks(this: GPtr<u8>, intel: GPtr<u8>) {
    group_intel_flush_tasks(intel, intel.byte_add(0xC), GPtr::NULL);
    group_intel_flush_tasks(intel, intel.byte_add(0xAC), GPtr::NULL);
    let ours = rd::<GPtr<u8>>(intel, 0);
    let members = ours.byte_add(8);
    let threat = rd::<u32>(this, 4);
    if threat == 0 {
        return;
    }
    let tg = ped_groups_get_peds_group(threat);
    if tg != 0 {
        let gid = get_group_id(GPtr::from_addr(tg as usize));
        let theirs = GPtr::<u8>::from_addr((0xC0_9920 + gid.wrapping_mul(0x2D4)) as u32 as usize);
        if theirs == ours {
            crt_printf(0x87_0F00);
            return;
        }
        let mut closest = [0u32; 8];
        compute_closest_peds_o(ours, theirs, GPtr::from_raw(closest.as_mut_ptr()).cast());
        for i in 0..8 {
            let m = membership_get_member(members, i);
            if m.is_null() || ped_is_player(m) != 0 {
                continue;
            }
            let mut task = [0u32; 8];
            let tp: GPtr<u8> = GPtr::from_raw(task.as_mut_ptr()).cast();
            if seeks_cover(m, rd::<GPtr<u8>>(this, 4)) {
                seek_cover_ctor_group(tp, gid);
                set_response(intel, m, tp);
                seek_cover_dtor(tp);
            } else {
                kill_ped_group_on_foot_ctor(tp, gid, GPtr::from_addr(closest[i as usize] as usize));
                set_response(intel, m, tp);
                kill_ped_group_on_foot_dtor(tp);
            }
        }
        interesting_events_add(g(0xC0_B058), 0x18, membership_get_leader(members));
    } else {
        for i in 0..8 {
            let m = membership_get_member(members, i);
            if m.is_null() || ped_is_player(m) != 0 {
                continue;
            }
            if seeks_cover(m, rd::<GPtr<u8>>(this, 4)) {
                let mut task = [0u32; 8];
                let tp: GPtr<u8> = GPtr::from_raw(task.as_mut_ptr()).cast();
                seek_cover_ctor_ped(tp, rd::<GPtr<u8>>(this, 4));
                set_response(intel, m, tp);
                seek_cover_dtor(tp);
                continue;
            }
            let mut seq = [0u32; 0x10];
            let sp: GPtr<u8> = GPtr::from_raw(seq.as_mut_ptr()).cast();
            sequence_ctor(sp);
            let t = task_new(0x38);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                kill_ped_on_foot_ctor(t, rd::<GPtr<u8>>(this, 4), -1, 0, 0, 0, 1)
            };
            sequence_add_task(sp, t);
            let t = task_new(0x34);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                look_about_ctor(t, general_random_int_in_range(1000, 2000) as u32)
            };
            sequence_add_task(sp, t);
            set_response(intel, m, sp);
            sequence_dtor(sp);
        }
        interesting_events_add(g(0xC0_B058), 0x17, membership_get_leader(members));
    }
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_ARREST_PED_CONTROL_SUB_TASK,
    HOOK_CALC_SOUND_LEVEL_INCREMENT,
    HOOK_KILL_THREATS_RANDOM_GROUP_ALLOCATE_TASKS,
];
