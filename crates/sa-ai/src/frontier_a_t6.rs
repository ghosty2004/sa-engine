//! FRONTIER-2 turn 6, lane A (batch `frontier2-a9`): AI frontier entries ported from the 1.0 US bytes (gta-reversed
//! read as an aid; where it differs the bytes win, noted per function).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::bindings::clump_get_association;
use sa_core::filemgr::strcpy;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{can_sunbathe, go_to_point_ctor, in_air_ctor, simple_dead_ctor, task_new, task_simple_ctor};
use crate::bindings::{compute_clear_target, event_takes_priority_over, should_load_gang_anims};
use crate::bindings::{car_enter_exit_is_vehicle_stealable, run_anim_ctor2, scanner_closest_vehicle, task_timer_is_out_of_time};
use crate::common::X87;
use sa_core::bindings::entity_register_reference;
use sa_math::fpu::{fcom_any, fcom_ne};
use sa_core::bindings::{frame_get_ltm, fx_play};
use sa_world::bindings::{fx_copy_parent_matrix, fx_create_system, fx_set_const_time, fx_set_local_particles, fx_set_offset_pos, fx_system_set_matrix};
use sa_pop::bindings::add_weapon_model;
use sa_world::bindings::{die_ctor, intel_find_task_by_type};
use crate::common::{ftol32, g, rd, task_get_type, task_make_abortable, wr, x, xi};
use sa_core::bindings::{
    find_player_ped, general_get_radian_angle_between_points, general_limit_radian_angle, vector_normalise,
};
use sa_core::crt::rand_at;
use sa_math::fpu::{fld_fst, st0};
use sa_pop::bindings::wander_gang_ctor;
use sa_stream::bindings::{anim_remove_block_ref, get_animation_block_index};
use sa_anim::bindings::anim_add_block_ref;
use crate::fall::fild_u32;
use sa_core::bindings::{
    clump_get_frame_from_name, get_anim_hierarchy_from_skin_clump, rp_hanim_hierarchy_get_matrix_array,
    rw_frame_update_objects, rw_matrix_rotate, rw_matrix_translate,
};
use sa_math::fpu::{ge_or_nan, gt, le_or_nan, lt};
use sa_stream::bindings::streaming_request_model;
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::{
    anim_blend_animation, follow_node_route_ctor, fx_kill, go_to_point_and_stand_still_ctor, intel_clear_tasks,
    ped_remove_weapon_model, ped_set_current_weapon, rp_clump_render, run_named_anim_ctor,
};

orig! {
    /// `CTaskComplexGoToPointAnyMeans::CreateSubTask(int32 type, CPed*)` (0x6705D0, `ret 8`)
    fn go_to_point_any_means_create_sub_task_o(t: u32, ty: u32, ped: u32) -> u32 = thiscall 0x6705D0 => crate::gen_subtasks::g_task_complex_go_to_point_any_means_create_sub_task_6705d0;
    /// `CTaskSimplePlayerOnFoot::PlayerControlZelda(CPlayerPed*, bool)` (0x6883D0, `ret 8`)
    fn player_control_zelda_o(t: GPtr<u8>, ped: GPtr<u8>, b: u8) = thiscall 0x6883D0 => crate::player_on_foot::on_foot_player_control_zelda;
    /// `CEvent::GetSourceEntity()` (vtable +0x28)
    fn v_event_source_entity(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x28 => unported;
    /// `CEvent::GetEventPriority()` (vtable +8)
    fn v_event_priority(e: GPtr<u8>) -> i32 = thiscall vslot 0x8 => unported;
    /// `CTaskComplexFollowLeaderAnyMeans::Constructor(CPedGroup*, CPed* leader, const CVector& offset)` (0x694AA0,
    /// `ret 0xC`)
    fn follow_leader_any_means_ctor(t: u32, group: u32, leader: u32, offset: u32) -> u32 = thiscall 0x694AA0 => crate::gen_ctors::g_task_complex_follow_leader_any_means_constructor_694aa0;
    /// `CTaskSimpleJetPack::DoJetPackEffect(CPed*)` (0x67B7F0, `ret 4`)
    fn jet_pack_do_effect_o(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x67B7F0 => jet_pack_do_effect;
    /// `CEvent::GetEventType()` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `FUN_006348f0` (0x6348F0, `ret 4`): the mobile phone task's weapon restore
    fn use_mobile_phone_restore_weapon_o(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6348F0 => use_mobile_phone_restore_weapon;
    /// `FUN_0066d550` (0x66D550): reverses a patrol route
    fn patrol_route_reverse_o(r: GPtr<u8>) = thiscall 0x66D550 => patrol_route_reverse;
    /// `FUN_00670a70` (0x670A70): the patrol route task's next sub-task type
    fn follow_patrol_route_next_type_o(t: GPtr<u8>) -> i32 = thiscall 0x670A70 => follow_patrol_route_next_type;
    /// `FUN_00670b00` (0x670B00, `ret 4`): the patrol route task's CreateSubTask
    fn follow_patrol_route_create_sub_task_o(t: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x670B00 => follow_patrol_route_create_sub_task;
    /// `CTaskComplexSunbathe::ShouldLoadSunbatheAnims()` (0x632190, AL)
    fn should_load_sunbathe_anims_o() -> u8 = cdecl 0x632190 => should_load_sunbathe_anims;
}

/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIME_MS: usize = 0xB7CB84;

// ------------------------------------------------------------------------------------------- the mobile phone task

/// Clears `bIsTalking` (+0x478 bit 2) and the phone flag (+0x474 bit 0x400000) of the ped.
fn clear_phone_flags(ped: GPtr<u8>) {
    let a = rd::<u32>(ped, 0x478) & 0xFFFF_FFFD;
    let b = rd::<u32>(ped, 0x474) & 0xFFBF_FFFF;
    wr::<u32>(ped, 0x478, a);
    wr::<u32>(ped, 0x474, b);
}

/// `FUN_006348f0` (0x6348F0, thiscall, `ret 4`; gta-reversed's spec name `CTaskComplexUseMobilePhone::sub_6348F0`):
/// the task itself is not read. `ped->RemoveWeaponModel(330)` (the cell phone), `SetCurrentWeapon(+0x70C)`, +0x70C =
/// 0x37 (none), and with player data (+0x480) its byte +0x85 cleared.
#[hook(addr = 0x6348F0, conv = thiscall, name = "FUN_006348f0", expect = "56 8B 74 24 08 68 4A 01",
       unverified = "frontier2-a9", route = "L2", hazards = "none")]
pub fn use_mobile_phone_restore_weapon(_this: GPtr<u8>, ped: GPtr<u8>) {
    ped_remove_weapon_model(ped, 0x14A);
    ped_set_current_weapon(ped, rd::<i32>(ped, 0x70C));
    wr::<u32>(ped, 0x70C, 0x37);
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if !pd.is_null() {
        wr::<u8>(pd, 0x85, 0);
    }
}

/// `CTaskComplexUseMobilePhone::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x634930, `ret 0xC`).
/// * Priority 2 (immediate): the sub-task's `MakeAbortable(ped, 2, event)`; when it agrees the ped's phone flags are
///   cleared and, with a weapon slot other than 0x37, 0x6348F0 restores the weapon. Returns the sub-task's answer.
/// * A sub-task of type 0xCA: the flags cleared, true.
/// * Otherwise: an event of type 0x3A on a player returns true at once; any other event sets +0x1C. Then +0x10 = now,
///   +0x14 = +0xC, +0x18 = 1 (the quit timer started), a sub-task of type 0x643 gets `MakeAbortable(ped, priority,
///   event)` (its answer ignored), false.
#[hook(addr = 0x634930, conv = thiscall, name = "CTaskComplexUseMobilePhone::MakeAbortable",
       expect = "8B 44 24 08 53 56 8B F1", this = "CTaskComplexUseMobilePhone",
       unverified = "frontier2-a9", route = "L2", hazards = "none")]
pub fn use_mobile_phone_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    let sub = rd::<GPtr<u8>>(this, 8);
    if priority == 2 {
        let r = task_make_abortable(sub, ped, 2, event);
        if r != 0 {
            clear_phone_flags(ped);
            if rd::<u32>(ped, 0x70C) != 0x37 {
                use_mobile_phone_restore_weapon_o(this, ped);
            }
        }
        return r;
    }
    if task_get_type(sub) == 0xCA {
        clear_phone_flags(ped);
        return 1;
    }
    if !event.is_null() {
        if v_event_type(event) == 0x3A && ped_is_player(ped) != 0 {
            return 1;
        }
        wr::<u8>(this, 0x1C, 1);
    }
    wr::<u32>(this, 0x10, g::<u32>(TIME_MS).read());
    wr::<u32>(this, 0x14, rd::<u32>(this, 0xC));
    wr::<u8>(this, 0x18, 1);
    if task_get_type(rd::<GPtr<u8>>(this, 8)) == 0x643 {
        task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, priority, event);
    }
    0
}

// ------------------------------------------------------------------------------------------- the patrol route task
//
// `CPatrolRoute` (the task's +0x1C): +0 the point count, +4 + i * 0x28 the point's anim name (0x18 bytes) and anim
// block name (+0x18, 0x10 bytes), +0x144 + i * 0xC the point's position. The task (vtable 0x870114): +0xC (i16) the
// route mode, +0xE (i16) the laps done, +0x10 (i16) the move state, +0x12 (i16) the current point, +0x14 the radius,
// +0x18 the move-state radius, +0x20 flags.

/// The position of point `i`.
pub(crate) fn route_pos(route: GPtr<u8>, i: i32) -> GPtr<u8> {
    route.byte_add(((i + 0x1B) * 0xC) as isize)
}

/// The anim name of point `i`.
fn route_name(route: GPtr<u8>, i: i32) -> GPtr<u8> {
    route.byte_add((i * 0x28 + 4) as isize)
}

/// `FUN_0066d550` (0x66D550, thiscall on the route): reverses the route in place. Points `i < j` from both ends are
/// swapped: the positions (dword by dword), then the two names through a stack buffer (0x18 + 0x10 bytes, `strcpy`
/// each way).
#[hook(addr = 0x66D550, conv = thiscall, name = "FUN_0066d550", expect = "83 EC 44 57 8B 39 4F 85",
       unverified = "frontier2-a9", route = "L2", hazards = "none")]
pub fn patrol_route_reverse(route: GPtr<u8>) {
    let mut j = rd::<i32>(route, 0) - 1;
    let mut i = 0;
    if j <= 0 {
        return;
    }
    let mut tmp = [0u8; 0x28];
    let t = GPtr::<u8>::from_raw(tmp.as_mut_ptr());
    while i < j {
        let (pi, pj) = (route_pos(route, i), route_pos(route, j));
        let v = [rd::<u32>(pi, 0), rd::<u32>(pi, 4), rd::<u32>(pi, 8)];
        for k in 0..3 {
            wr::<u32>(pi, k * 4, rd::<u32>(pj, k * 4));
        }
        for (k, w) in v.iter().enumerate() {
            wr::<u32>(pj, k as isize * 4, *w);
        }
        let (ni, nj) = (route_name(route, i), route_name(route, j));
        strcpy(t, ni);
        strcpy(t.byte_add(0x18), ni.byte_add(0x18));
        strcpy(ni, nj);
        strcpy(ni.byte_add(0x18), nj.byte_add(0x18));
        strcpy(nj, t);
        strcpy(nj.byte_add(0x18), t.byte_add(0x18));
        i += 1;
        j -= 1;
    }
}

/// `FUN_00670a70` (0x670A70, thiscall): the patrol route task's next sub-task type. No points: 0x516 (finished).
/// Before the last point 0x384 (go to the next point), at it 0x387 (go there and stand still). Past it (the current
/// point equals the count) the lap counter (+0xE) goes up and the mode decides: 0 finished; 1 reverse once (on the
/// first lap only, else finished); 2 reverse; 3 start again; with the point reset to 0 and the type asked again (a
/// tail call). Any other state: 0xC8.
#[hook(addr = 0x670A70, conv = thiscall, name = "FUN_00670a70", expect = "56 8B F1 8B 4E 1C 8B 11",
       unverified = "frontier2-a9", route = "L2", hazards = "none")]
pub fn follow_patrol_route_next_type(this: GPtr<u8>) -> i32 {
    let route = rd::<GPtr<u8>>(this, 0x1C);
    let n = rd::<i32>(route, 0);
    if n == 0 {
        return 0x516;
    }
    let cur = rd::<i16>(this, 0x12) as i32;
    if cur + 1 < n {
        return 0x384;
    }
    if cur + 1 == n {
        return 0x387;
    }
    if cur != n {
        return 0xC8;
    }
    let mode = rd::<i16>(this, 0xC) as i32;
    let laps = rd::<u16>(this, 0xE).wrapping_add(1);
    wr::<u16>(this, 0xE, laps);
    match mode {
        0 => return 0x516,
        1 if laps != 1 => return 0x516,
        1 | 2 => patrol_route_reverse_o(route),
        3 => {}
        _ => return 0xC8,
    }
    wr::<u16>(this, 0x12, 0);
    follow_patrol_route_next_type_o(this)
}

/// `FUN_00670b00` (0x670B00, thiscall, `ret 4`): the patrol route task's CreateSubTask(type).
/// * 0x384: `CTaskSimpleGoToPoint(moveState, the point, radius, false, false)`;
/// * 0x191: `CTaskSimpleRunNamedAnim(name, block, 0x58, 4.0, -1, false, false, false, false)` of the point;
/// * 0xC8: a bare `CTaskSimple` with vtable 0x859CFC (8 bytes);
/// * 0x387: `CTaskComplexGoToPointAndStandStill(moveState, the point, radius, +0x18, false, false)`, the move-state
///   radius 0 in modes 2 and 3, and in mode 1 before the first lap is done;
/// * 0x38A: with points, the current point clamped to the last (when it equals the count), its position copied to the
///   stack, then `CTaskComplexFollowNodeRoute(moveState, that copy, [0x86FCA0], [0x86FCA4], [0x86FCA8], false, -1,
///   true)`;
/// * anything else (0x516 included): null. A failed allocation returns null.
#[hook(addr = 0x670B00, conv = thiscall, name = "FUN_00670b00", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes",
       note = "SEH frame not modelled")]
pub fn follow_patrol_route_create_sub_task(this: GPtr<u8>, ty: i32) -> GPtr<u8> {
    let ms = |t: GPtr<u8>| rd::<i16>(t, 0x10) as i32 as u32;
    let cur = |t: GPtr<u8>| rd::<i16>(t, 0x12) as i32;
    match ty {
        0x384 => {
            let m = task_new(0x30);
            if m.is_null() {
                return GPtr::NULL;
            }
            let pos = route_pos(rd::<GPtr<u8>>(this, 0x1C), cur(this));
            go_to_point_ctor(m, ms(this), pos, rd::<u32>(this, 0x14), 0, 0)
        }
        0x191 => {
            let name = route_name(rd::<GPtr<u8>>(this, 0x1C), cur(this));
            let m = task_new(0x64);
            if m.is_null() {
                return GPtr::NULL;
            }
            run_named_anim_ctor(m, name, name.byte_add(0x18), 0x58, 0x4080_0000, 0xFFFF_FFFF, 0, 0, 0, 0)
        }
        0xC8 => {
            let m = task_new(8);
            if m.is_null() {
                return GPtr::NULL;
            }
            task_simple_ctor(m);
            wr::<u32>(m, 0, 0x859CFC);
            m
        }
        0x387 => {
            let mode = rd::<i16>(this, 0xC) as i32;
            let mut msr = rd::<u32>(this, 0x18);
            if mode == 2 || mode == 3 || (mode == 1 && rd::<i16>(this, 0xE) == 0) {
                msr = 0;
            }
            let m = task_new(0x28);
            if m.is_null() {
                return GPtr::NULL;
            }
            let pos = route_pos(rd::<GPtr<u8>>(this, 0x1C), cur(this));
            go_to_point_and_stand_still_ctor(m, ms(this), pos.cast(), f32::from_bits(rd::<u32>(this, 0x14)),
                                             f32::from_bits(msr), 0, 0)
        }
        0x38A => {
            let route = rd::<GPtr<u8>>(this, 0x1C);
            let n = rd::<i32>(route, 0);
            if n == 0 {
                return GPtr::NULL;
            }
            let c = rd::<u16>(this, 0x12);
            if c as i16 as i32 == n {
                wr::<u16>(this, 0x12, c.wrapping_sub(1));
            }
            let p = route_pos(route, cur(this));
            let mut v = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
            let m = task_new(0x60);
            if m.is_null() {
                return GPtr::NULL;
            }
            let (a, b, c) = (g::<u32>(0x86FCA0).read(), g::<u32>(0x86FCA4).read(), g::<u32>(0x86FCA8).read());
            follow_node_route_ctor(m, ms(this), GPtr::from_raw(v.as_mut_ptr()).cast(), a, b, c, 0, 0xFFFF_FFFF, 1)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexFollowPatrolRoute::CreateNextSubTask(CPed*)` (0x670D80, catalogue `vt_870114::CreateNextSubTask`,
/// `ret 4`; the ped is not read). After a follow-node-route sub-task (0x38A) the next type; after 0xC8 the finish
/// type 0x516; after the point's anim (0x191) the next point; after reaching a point, its anim when the point has one
/// (first byte not NUL, the `repe cmpsb` against "" at 0x858B54), else the next point. "The next point" increments
/// +0x12 first.
#[hook(addr = 0x670D80, conv = thiscall, name = "vt_870114::CreateNextSubTask", expect = "53 8B D9 8B 4B 08 8B 01",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes")]
pub fn follow_patrol_route_create_next_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    if task_get_type(rd::<GPtr<u8>>(this, 8)) != 0x38A {
        if task_get_type(rd::<GPtr<u8>>(this, 8)) == 0xC8 {
            return follow_patrol_route_create_sub_task_o(this, 0x516);
        }
        if task_get_type(rd::<GPtr<u8>>(this, 8)) != 0x191 {
            let name = route_name(rd::<GPtr<u8>>(this, 0x1C), rd::<i16>(this, 0x12) as i32);
            if name.read() != g::<u8>(0x858B54).read() {
                return follow_patrol_route_create_sub_task_o(this, 0x191);
            }
        }
        wr::<u16>(this, 0x12, rd::<u16>(this, 0x12).wrapping_add(1));
    }
    let ty = follow_patrol_route_next_type_o(this);
    follow_patrol_route_create_sub_task_o(this, ty)
}

// ------------------------------------------------------------------------------------------- fall to death

/// `|z| < k` as the original computes it: `fchs` only when `z < 0`, then `fcomp k`; NaN compares false.
fn abs_lt(z: f32, k: f32) -> bool {
    let a = if z < 0.0 { -z } else { z };
    a < k
}

/// `CTaskComplexFallToDeath::CreateNextSubTask(CPed*)` (0x679270, `ret 4`). +0x18 the fall anim, +0x1C the second
/// anim, +0x21 flags (bit 1 retried in air, bit 4 landed hard).
/// * After CTaskSimpleInAir (0xF1): with the ped's move speed z (+0x4C) `|z| < 0.1` a `CTaskSimpleLand` (inlined:
///   `CTaskSimple`, vtable 0x8704FC, +8 = 0, +0xC = -1, flag byte +0x10 `(b & 0xFE) | 6`) and bit 4 cleared; else
///   one with anim 0x18 and `(b & 0xFC) | 4`, bit 4 set (the flag set even when `new` fails). gta-reversed has the
///   comparison the other way round; the bytes win.
/// * After CTaskSimpleLand (0xF2): not yet retried and `|z| >= 0.01` (or NaN) with no contact entity (+0x584) or a
///   building: bit 1 set, a new `CTaskSimpleInAir(false, true, false)`. Otherwise, without bit 4, the fall anim's
///   association (when not -1) gets blend delta -1000, +0x1C = 0x18 and `BlendAnimation(clump, 0, 0x18, 8.0)`; then
///   `ClearTasks(false, true)` and a `CTaskSimpleDead(now, false)`.
/// * Anything else: null.
#[hook(addr = 0x679270, conv = thiscall, name = "CTaskComplexFallToDeath::CreateNextSubTask",
       expect = "64 A1 00 00 00 00 6A FF", this = "CTaskComplexFallToDeath",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes",
       note = "SEH frame not modelled")]
pub fn fall_to_death_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    match task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0xF1 => {
            let soft = abs_lt(rd::<f32>(ped, 0x4C), f32::from_bits(0x3DCC_CCCD));
            let m = task_new(0x14);
            if !m.is_null() {
                task_simple_ctor(m);
                let f = rd::<u8>(m, 0x10);
                wr::<u8>(m, 0x10, if soft { (f & 0xFE) | 6 } else { (f & 0xFC) | 4 });
                wr::<u32>(m, 0, 0x8704FC);
                wr::<u32>(m, 8, 0);
                wr::<u32>(m, 0xC, if soft { 0xFFFF_FFFF } else { 0x18 });
            }
            let b = rd::<u8>(this, 0x21);
            wr::<u8>(this, 0x21, if soft { b & 0xFB } else { b | 4 });
            m
        }
        0xF2 => {
            let fl = rd::<u8>(this, 0x21);
            if fl & 1 == 0 && !abs_lt(rd::<f32>(ped, 0x4C), f32::from_bits(0x3C23_D70A)) {
                let contact = rd::<GPtr<u8>>(ped, 0x584);
                if contact.is_null() || rd::<u8>(contact, 0x36) & 7 == 1 {
                    wr::<u8>(this, 0x21, fl | 1);
                    let m = task_new(0x3C);
                    if m.is_null() {
                        return GPtr::NULL;
                    }
                    return in_air_ctor(m, 0, 1, 0);
                }
            }
            if fl & 4 == 0 {
                let anim = rd::<u32>(this, 0x18);
                if anim != 0xFFFF_FFFF {
                    let a = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), anim);
                    if !a.is_null() {
                        wr::<u32>(a, 0x1C, 0xC47A_0000);
                    }
                }
                wr::<u32>(this, 0x1C, 0x18);
                anim_blend_animation(rd::<GPtr<u8>>(ped, 0x18), 0, 0x18, 8.0);
            }
            intel_clear_tasks(rd::<GPtr<u8>>(ped, 0x47C), 0, 1);
            let m = task_new(0x10);
            if m.is_null() {
                return GPtr::NULL;
            }
            simple_dead_ctor(m, g::<u32>(TIME_MS).read(), 0)
        }
        _ => GPtr::NULL,
    }
}

// ------------------------------------------------------------------------------------------- sunbathe

/// `CTaskComplexSunbathe::ShouldLoadSunbatheAnims()` (0x632190, the entry a `nop; nop; jmp` to the island 0x402274
/// that reads `CWorld::PlayerInFocus` (0xB7CD74) and jumps back; ECX is not read). The player in focus's ped
/// (`CWorld::Players[i]`, 0xB7CD98 + i * 0x190): in a vehicle (+0x46C bit 0x100, +0x58C) whose move speed squared
/// (`x*x + y*y + z*z` on the x87 stack) is above 0.04 (0x863244): false; otherwise (NaN included) true.
#[hook(addr = 0x632190, conv = cdecl, name = "CTaskComplexSunbathe::ShouldLoadSunbatheAnims",
       expect = "90 90 E9 DD 00 DD FF",
       unverified = "frontier2-a9", route = "L2", hazards = "none")]
pub fn should_load_sunbathe_anims() -> u8 {
    let i = g::<u8>(0xB7CD74).read() as u32;
    let ped = g::<GPtr<u8>>(0xB7CD98 + (i * 0x190) as usize).read();
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 || veh.is_null() {
        return 1;
    }
    let (vx, vy, vz) = (x(rd::<f32>(veh, 0x44)), x(rd::<f32>(veh, 0x48)), x(rd::<f32>(veh, 0x4C)));
    let sq = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz));
    if le_or_nan(sq, x(g::<f32>(0x863244).read())) {
        1
    } else {
        0
    }
}

/// `CTaskComplexSunbathe::ControlSubTask(CPed*)` (0x6381A0, `ret 4`). +0xD bathing, +0xE / +0xF the beach /
/// sunbathe anim blocks referenced, +0x10 aborted, +0x14 / +0x18 / +0x1C / +0x1D the bathing timer (start, interval,
/// started, stopped), +0x24 / +0x28 the anim blocks, +0x2C / +0x30 their indices.
/// * Aborted: null.
/// * The beach block referenced once loaded (`AddAnimBlockRef`), else requested (model 0x63E7 + index, flags 8); the
///   sunbathe block likewise, but only while `ShouldLoadSunbatheAnims()`.
/// * Bathing: ped +0x528 |= 8, +0x474 |= 0x100000.
/// * When `CanSunbathe()` fails: a started timer is stopped (+0x1D = 1, the interval reduced by the time elapsed) and a
///   sub-task of type 0x1A2 gets `MakeAbortable(ped, 0, null)`.
///
/// Returns the sub-task (+8, read last).
#[hook(addr = 0x6381A0, conv = thiscall, name = "CTaskComplexSunbathe::ControlSubTask", expect = "56 8B F1 8A 46 10 84 C0",
       this = "CTaskComplexSunbathe",
       unverified = "frontier2-a9", route = "L2", hazards = "none")]
pub fn sunbathe_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(this, 0x10) != 0 {
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0xE) == 0 {
        if rd::<u8>(rd::<GPtr<u8>>(this, 0x24), 0x10) != 0 {
            anim_add_block_ref(rd::<i32>(this, 0x2C));
            wr::<u8>(this, 0xE, 1);
        } else {
            streaming_request_model(rd::<i32>(this, 0x2C).wrapping_add(0x63E7), 8);
        }
    }
    if rd::<u8>(this, 0xF) == 0 && should_load_sunbathe_anims_o() != 0 {
        if rd::<u8>(rd::<GPtr<u8>>(this, 0x28), 0x10) != 0 {
            anim_add_block_ref(rd::<i32>(this, 0x30));
            wr::<u8>(this, 0xF, 1);
        } else {
            streaming_request_model(rd::<i32>(this, 0x30).wrapping_add(0x63E7), 8);
        }
    }
    if rd::<u8>(this, 0xD) != 0 {
        wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) | 8);
        wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x10_0000);
    }
    if can_sunbathe() as u8 == 0 {
        if rd::<u8>(this, 0x1C) != 0 {
            let start = rd::<u32>(this, 0x14);
            let interval = rd::<u32>(this, 0x18);
            wr::<u8>(this, 0x1D, 1);
            let elapsed = g::<u32>(TIME_MS).read().wrapping_sub(start);
            wr::<u32>(this, 0x18, interval.wrapping_sub(elapsed));
        }
        if task_get_type(rd::<GPtr<u8>>(this, 8)) == 0x1A2 {
            task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 0, GPtr::NULL);
        }
    }
    rd::<GPtr<u8>>(this, 8)
}

// ------------------------------------------------------------------------------------------- jet pack

/// Rotates a jet ball frame's matrix (`RwMatrixRotate(X axis 0x8D232C, rx, replace)`, then `(Y axis 0x8D2338, ry,
/// preconcat)`) and puts its saved position back.
fn rotate_jet_ball(m: GPtr<u8>, pos: [u32; 3], rx: f32, ry: f32) {
    rw_matrix_rotate(m, GPtr::from_addr(0x8D232C), rx, 0);
    rw_matrix_rotate(m, GPtr::from_addr(0x8D2338), ry, 1);
    for (k, v) in pos.iter().enumerate() {
        wr::<u32>(m, 0x30 + k as isize * 4, *v);
    }
}

/// `v` clamped to [-90, 90] as the original tests it (`90 < v` → 90, else `-90 > v` → -90, else `v`; NaN stays).
fn clamp_90(v: f32) -> f32 {
    if lt(x(g::<f32>(0x85991C).read()), x(v)) {
        g::<f32>(0x85991C).read()
    } else if gt(x(g::<f32>(0x859CE8).read()), x(v)) {
        f32::from_bits(0xC2B4_0000)
    } else {
        v
    }
}

/// `CTaskSimpleJetPack::RenderJetPack(CPed*)` (0x67F6A0, `ret 4`). +8 finished, +0xD thrust stop, +0x10 strafe, +0x14
/// thrust angle, +0x3C the previous heading, +0x40 the jet pack clump, +0x64 / +0x68 the effect systems.
/// * No clump or finished: both effects killed (`FxSystem_c::Kill`, slot cleared).
/// * Otherwise the clump's frame matrix becomes the ped's skin bone matrix 3 (16 dwords), translated by 0x8D2F68 and
///   rotated 90 degrees about 0x8D2F74 (both preconcat). The heading change: the ped's aiming rotation (+0x55C)
///   brought within pi of the previous heading (`cur > prev + pi` → -2pi, else `cur < prev - pi` → +2pi; gta-reversed
///   has `>=` / `<=`), minus it, clamped to [-0.2, 0.2] on the x87 stack, times 57.2958 and 10 (f32 `r`). The jet
///   balls (`jetball1`, `jetball2`; skipped when absent), their position kept: with thrust stop `t = time /
///   79.577469`, rotations `(sin t * 45, cos t * 45)` (`-45` for the second); otherwise `(clamp_90(angle * 57.2958 +
///   r), max(0, strafe * 45))` for the first and `(clamp_90(angle * 57.2958 - r), min(0, strafe * 45))` for the
///   second (gta-reversed uses `angle - r` for both). Then `RwFrameUpdateObjects`, `RpClumpRender`,
///   `DoJetPackEffect(ped)`.
///
/// Both paths end with +0x3C = the ped's current rotation (+0x558).
#[hook(addr = 0x67F6A0, conv = thiscall, name = "CTaskSimpleJetPack::RenderJetPack", expect = "83 EC 1C 55 8B E9 8B 45 40",
       this = "CTaskSimpleJetPack",
       unverified = "frontier2-a9", route = "L2", hazards = "x87-transcendental,heap-writes")]
pub fn jet_pack_render(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u32>(this, 0x40) == 0 || rd::<u8>(this, 8) != 0 {
        for o in [0x64, 0x68] {
            let fx = rd::<GPtr<u8>>(this, o);
            if !fx.is_null() {
                fx_kill(fx);
                wr::<u32>(this, o, 0);
            }
        }
        wr::<u32>(this, 0x3C, rd::<u32>(ped, 0x558));
        return;
    }
    let hier = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(ped, 0x18));
    let src = rp_hanim_hierarchy_get_matrix_array(hier).byte_add(0xC0);
    let frame = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x40), 4);
    let m = frame.byte_add(0x10);
    for k in 0..16 {
        wr::<u32>(m, k * 4, rd::<u32>(src, k * 4));
    }
    rw_matrix_translate(m, GPtr::from_addr(0x8D2F68), 1);
    rw_matrix_rotate(m, GPtr::from_addr(0x8D2F74), 90.0, 1);

    let pi = g::<f32>(0x858CB8).read();
    let two_pi = g::<f32>(0x858CBC).read();
    let prev = rd::<f32>(this, 0x3C);
    let mut cur = x(rd::<f32>(ped, 0x55C));
    if gt(cur, x(prev).fadd_m(pi)) {
        cur = cur.fsub_m(two_pi);
    } else if lt(cur, x(prev).fsub_m(pi)) {
        cur = cur.fadd_m(two_pi);
    }
    let mut d = cur.fsub_m(prev);
    let lim = g::<f32>(0x8D2F88).read();
    if gt(d, x(lim)) {
        d = x(lim);
    } else if lt(d, x(-lim)) {
        d = x(-lim);
    }
    let r = d.fmul_m(g::<f32>(0x859878).read()).fmul_m(g::<f32>(0x8D2F84).read()).to_f32();

    for (name, left) in [(0x8705F4usize, true), (0x8705E8, false)] {
        let jb = clump_get_frame_from_name(rd::<GPtr<u8>>(this, 0x40), GPtr::from_addr(name));
        if jb.is_null() {
            continue;
        }
        let jm = jb.byte_add(0x10);
        let pos = [rd::<u32>(jm, 0x30), rd::<u32>(jm, 0x34), rd::<u32>(jm, 0x38)];
        let (rx, ry) = if rd::<u8>(this, 0xD) != 0 {
            let t = fild_u32(g::<i32>(TIME_MS).read()).fdiv_m(g::<f32>(0x8D2F80).read());
            let k = if left { 0x858CB4 } else { 0x87063C };
            let rx = t.fsin().fmul_m(g::<f32>(k).read()).to_f32();
            let ry = t.fcos().fmul_m(g::<f32>(0x858CB4).read()).to_f32();
            (rx, ry)
        } else {
            let a = x(rd::<f32>(this, 0x14)).fmul_m(g::<f32>(0x859878).read());
            let v = if left { a.fadd_m(r) } else { a.fsub_m(r) }.to_f32();
            let rx = clamp_90(v);
            let y = x(rd::<f32>(this, 0x10)).fmul_m(g::<f32>(0x858CB4).read());
            let zero = x(g::<f32>(0x858B50).read());
            let ry = if left {
                if le_or_nan(zero, y) { y.to_f32() } else { 0.0 }
            } else if ge_or_nan(zero, y) {
                y.to_f32()
            } else {
                0.0
            };
            (rx, ry)
        };
        rotate_jet_ball(jm, pos, rx, ry);
    }
    rw_frame_update_objects(frame);
    rp_clump_render(rd::<GPtr<u8>>(this, 0x40));
    jet_pack_do_effect_o(this, ped);
    wr::<u32>(this, 0x3C, rd::<u32>(ped, 0x558));
}

// ------------------------------------------------------------------------------------------- small virtuals

/// `CEventGunAimedAt::TakesPriorityOver(const CEvent&)` (0x4B0810, `ret 4`). With a player as the aimer (+0x14): an
/// event from that same source entity (`other.GetSourceEntity()`, vtable +0x28) compares priorities (`this >=
/// other`, the other's read first); otherwise events of type 9, 0x24 and 0x27 lose to it and the rest go to
/// `CEvent::TakesPriorityOver`. Without a player aimer: the priority comparison.
#[hook(addr = 0x4B0810, conv = thiscall, name = "CEventGunAimedAt::TakesPriorityOver", expect = "56 57 8B F9 8B 4F 14 85 C9",
       this = "CEventGunAimedAt",
       unverified = "frontier2-a9", route = "L2", hazards = "none")]
pub fn gun_aimed_at_takes_priority_over(this: GPtr<u8>, other: GPtr<u8>) -> u8 {
    let aimer = rd::<GPtr<u8>>(this, 0x14);
    if !aimer.is_null() && ped_is_player(aimer) != 0 {
        if v_event_source_entity(other).addr() as u32 == rd::<u32>(this, 0x14) {
            let o = v_event_priority(other);
            return u8::from(v_event_priority(this) >= o);
        }
        return match v_event_type(other) {
            9 | 0x24 | 0x27 => 1,
            _ => event_takes_priority_over(this, other),
        };
    }
    let o = v_event_priority(other);
    u8::from(v_event_priority(this) >= o)
}

/// `CEntitySeekPosCalculatorRadiusAngleOffset::ComputeEntitySeekPos(const CPed&, const CEntity&, CVector&)` (0x6946F0,
/// `ret 0xC`). +4 the radius, +8 the angle offset. The entity's forward (matrix +0x10, or `(-sin h, cos h)` of its
/// heading +0x10 without a matrix, stored as f32) gives `a = LimitRadianAngle(f32(LimitRadianAngle(f32(
/// GetRadianAngleBetweenPoints(f.x, f.y, 0, 0))) + offset))`; the target is the entity's position plus `(-sin a * r, r
/// * cos a, r * 0)` (each sum stored as f32; FSIN / FCOS of the x87 result), and
/// `CPedGeometryAnalyser::ComputeClearTarget(ped, target, out)` writes the result.
#[hook(addr = 0x6946F0, conv = thiscall, name = "CEntitySeekPosCalculatorRadiusAngleOffset::ComputeEntitySeekPos",
       expect = "83 EC 18 56 8B 74 24 24", this = "CEntitySeekPosCalculatorRadiusAngleOffset",
       unverified = "frontier2-a9", route = "L2", hazards = "x87-transcendental")]
pub fn radius_angle_offset_compute_entity_seek_pos(this: GPtr<u8>, ped: GPtr<u8>, entity: GPtr<u8>, out: GPtr<u8>) {
    let m = rd::<GPtr<u8>>(entity, 0x14);
    let (fx, fy) = if !m.is_null() {
        (rd::<f32>(m, 0x10), rd::<f32>(m, 0x14))
    } else {
        let h = x(rd::<f32>(entity, 0x10));
        (h.fsin().fchs().to_f32(), h.fcos().to_f32())
    };
    let a0 = general_get_radian_angle_between_points(fx, fy, 0.0, 0.0).to_f32();
    let a1 = st0(general_limit_radian_angle(a0)).fadd_m(rd::<f32>(this, 8)).to_f32();
    let a = st0(general_limit_radian_angle(a1));
    let r = rd::<f32>(this, 4);
    let s = a.fsin().fchs();
    let c = a.fcos().to_f32();
    let dx = s.fmul_m(r);
    let dy = x(r).fmul_m(c);
    let dz = x(r).fmul_m(g::<f32>(0x858B50).read()).to_f32();
    let m = rd::<GPtr<u8>>(entity, 0x14);
    let p = if m.is_null() { entity.byte_add(4) } else { m.byte_add(0x30) };
    let mut t = [
        dx.fadd_m(rd::<f32>(p, 0)).to_f32(),
        dy.fadd_m(rd::<f32>(p, 4)).to_f32(),
        x(dz).fadd_m(rd::<f32>(p, 8)).to_f32(),
    ];
    compute_clear_target(ped, GPtr::from_raw(t.as_mut_ptr()).cast(), out);
}

/// `CTaskComplexGangJoinRespond::ControlSubTask(CPed*)` (0x661950, `ret 4`). The ped turns to the player: `(player -
/// ped)` in x / y (f32, z 0) normalised, `+0x55C = LimitRadianAngle(f32(GetRadianAngleBetweenPoints(d.x, d.y, 0,
/// 0)))`. The "gangs" anim block (+0xD referenced): not referenced and `ShouldLoadGangAnims()`: referenced once loaded
/// (state byte == 1), else requested (model 0x63E7 + index, flags 8); referenced and no longer wanted: released. Returns
/// the sub-task.
#[hook(addr = 0x661950, conv = thiscall, name = "CTaskComplexGangJoinRespond::ControlSubTask", expect = "83 EC 0C 53 8B 5C 24 14",
       this = "CTaskComplexGangJoinRespond",
       unverified = "frontier2-a9", route = "L2", hazards = "none")]
pub fn gang_join_respond_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    let pp = if pm.is_null() { ped.byte_add(4) } else { pm.byte_add(0x30) };
    let pl = find_player_ped(-1);
    let lm = rd::<GPtr<u8>>(pl, 0x14);
    let lp = if lm.is_null() { pl.byte_add(4) } else { lm.byte_add(0x30) };
    let mut d = [
        x(rd::<f32>(lp, 0)).fsub_m(rd::<f32>(pp, 0)).to_f32(),
        x(rd::<f32>(lp, 4)).fsub_m(rd::<f32>(pp, 4)).to_f32(),
        0.0,
    ];
    vector_normalise(GPtr::from_raw(d.as_mut_ptr()).cast());
    let a = general_get_radian_angle_between_points(d[0], d[1], 0.0, 0.0).to_f32();
    wr::<f32>(ped, 0x55C, st0(general_limit_radian_angle(a)).to_f32());
    let gangs = GPtr::<u8>::from_addr(0x86F92C);
    if rd::<u8>(this, 0xD) == 0 {
        if should_load_gang_anims() != 0 {
            let idx = get_animation_block_index(gangs);
            if g::<u8>(0xB5D4B0 + ((idx as u32) << 5) as usize).read() == 1 {
                anim_add_block_ref(idx);
                wr::<u8>(this, 0xD, 1);
            } else {
                streaming_request_model(idx.wrapping_add(0x63E7), 8);
            }
        }
    } else if should_load_gang_anims() == 0 {
        let idx = get_animation_block_index(gangs);
        anim_remove_block_ref(idx as u32);
        wr::<u8>(this, 0xD, 0);
    }
    rd::<GPtr<u8>>(this, 8)
}

/// `CPedGroupDefaultTaskAllocatorFollowAnyMeans::AllocateDefaultTasks(CPedGroup*, CPed*)` (0x5F6B40, `ret 8`; `this`
/// unused). The group intelligence's seven follower slots (+0x21C, 0x14 each): a member (all, or only `ped` when given)
/// gets `CTaskComplexFollowLeaderAnyMeans(group, leader +0x28, offset k)` with the offset `(x, y)` of the formation
/// table 0xC196E8 (y through `fld` / `fstp`), stored at the slot +4 (null when `new` fails). The leader slot (+0x2A8)
/// likewise gets `CTaskComplexWanderGang(4, ftol((rand() & 0xFFFF) * [0x858B14] * [0x859000]), 5000, true,
/// [0x86FCB0])` at +0x2AC.
#[hook(addr = 0x5F6B40, conv = thiscall, name = "CPedGroupDefaultTaskAllocatorFollowAnyMeans::AllocateDefaultTasks",
       expect = "64 A1 00 00 00 00 6A FF 68 26 DF 83 00",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes",
       note = "rand() through rand_at (0x5F6C12); SEH frame not modelled")]
pub fn follow_any_means_allocate_default_tasks(_this: GPtr<u8>, group: GPtr<u8>, ped: GPtr<u8>) {
    for k in 0..7usize {
        let slot = group.byte_add(0x21C + k as isize * 0x14);
        let m = rd::<GPtr<u8>>(slot, 0);
        if m.is_null() || (!ped.is_null() && m != ped) {
            continue;
        }
        let mem = task_new(0x24);
        let t = if mem.is_null() {
            GPtr::NULL
        } else {
            let mut off = [g::<u32>(0xC196E8 + k * 8).read(), fld_fst(g::<f32>(0xC196EC + k * 8).read()).to_bits()];
            GPtr::from_addr(follow_leader_any_means_ctor(
                mem.addr() as u32,
                group.addr() as u32,
                rd::<u32>(group, 0x28),
                GPtr::<u32>::from_raw(off.as_mut_ptr()).addr() as u32,
            ) as usize)
        };
        wr::<GPtr<u8>>(slot, 4, t);
    }
    let m = rd::<GPtr<u8>>(group, 0x2A8);
    if m.is_null() || (!ped.is_null() && m != ped) {
        return;
    }
    let mem = task_new(0x38);
    let t = if mem.is_null() {
        GPtr::NULL
    } else {
        let r = rand_at(0x5F6C12) & 0xFFFF;
        let f = g::<u32>(0x86FCB0).read();
        let dir = ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x859000).read()));
        wander_gang_ctor(mem, 4, dir, 5000, 1, f)
    };
    wr::<GPtr<u8>>(group, 0x2AC, t);
}

/// `CTaskComplexGoToPointAnyMeans::ControlSubTask(CPed*)` (0x672A50, `ret 4`). Only a follow-node-route sub-task
/// (0x38A) is controlled. +0xC the target, +0x20 the vehicle to steal, +0x24 the timer flag (-1: no timer), +0x28 /
/// +0x2C / +0x30 / +0x31 the task timer. With a timer (started at 3000 ms when not yet) that is out of time (`now >=
/// start + interval`, unsigned; a pending restart (+0x31) restarts it first): `CreateSubTask(0x340)` and the timer
/// restarted. Otherwise, more than 50 m (`50 * 50 < d²`, x87) from the target, the ped's closest scanned vehicle
/// (`FUN_005ffee0` on the intelligence +0xD4), when not the remembered one and `CCarEnterExit::IsVehicleStealable`,
/// is remembered (+0x20, with `RegisterReference`) and `CreateSubTask(0x2BD)` returned. Else the sub-task.
#[hook(addr = 0x672A50, conv = thiscall, name = "CTaskComplexGoToPointAnyMeans::ControlSubTask", expect = "51 53 55 56 8B F1 57 8B 7E 08",
       this = "CTaskComplexGoToPointAnyMeans",
       unverified = "frontier2-a9", route = "L2", hazards = "none")]
pub fn go_to_point_any_means_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    if task_get_type(sub) != 0x38A {
        return sub;
    }
    if rd::<i32>(this, 0x24) != -1 && rd::<u8>(this, 0x30) == 0 {
        wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
        wr::<u32>(this, 0x2C, 3000);
        wr::<u8>(this, 0x30, 1);
    }
    if rd::<u8>(this, 0x30) != 0 {
        if rd::<u8>(this, 0x31) != 0 {
            wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
            wr::<u8>(this, 0x31, 0);
        }
        let end = rd::<u32>(this, 0x2C).wrapping_add(rd::<u32>(this, 0x28));
        if g::<u32>(TIME_MS).read() >= end {
            let t = go_to_point_any_means_create_sub_task_o(this.addr() as u32, 0x340, ped.addr() as u32);
            wr::<u32>(this, 0x2C, 3000);
            wr::<u8>(this, 0x30, 1);
            wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
            return GPtr::from_addr(t as usize);
        }
    }
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let p = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let dx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(this, 0xC));
    let dy = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(this, 0x10));
    let dz = x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(this, 0x14));
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    let k = g::<f32>(0x86FD04).read();
    if ge_or_nan(x(k).fmul_m(k), d2) {
        return sub;
    }
    let veh = scanner_closest_vehicle(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0xD4));
    if veh.is_null() || veh.addr() as u32 == rd::<u32>(this, 0x20) {
        return sub;
    }
    if car_enter_exit_is_vehicle_stealable(veh, ped) == 0 {
        return sub;
    }
    wr::<GPtr<u8>>(this, 0x20, veh);
    let t = go_to_point_any_means_create_sub_task_o(this.addr() as u32, 0x2BD, ped.addr() as u32);
    entity_register_reference(rd::<GPtr<u8>>(this, 0x20), this.byte_add(0x20));
    GPtr::from_addr(t as usize)
}

/// `CTaskComplexUseMobilePhone::ControlSubTask(CPed*)` (0x634D60, `ret 4`). With player data (+0x480) its +0x85 is set
/// first. Unless the sub-task is a pause (0xCA): the phone anims 0x91 (take out) and 0x92 (put away): while taking it
/// out, from anim time 0.85 the phone model (330) is added; while putting it away, at time >= 1.566 and with `time -
/// delta` below it, 0x6348F0 restores the weapon; with neither, once the task timer (+0x10) is out of time, a
/// `CTaskSimpleRunAnim(0, 0x92, 4.0, 0x643, 0x86E48C, false)` replaces the sub-task (null when `new` fails). Then, with
/// player data: +0x84 set, a negative +0x18 cleared, `PlayerControlZelda(ped, true)` on the ped's task of type 0 (when
/// found), +0x84 cleared.
#[hook(addr = 0x634D60, conv = thiscall, name = "CTaskComplexUseMobilePhone::ControlSubTask", expect = "64 A1 00 00 00 00 6A FF 68 2B 09 84 00",
       this = "CTaskComplexUseMobilePhone",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes",
       note = "SEH frame not modelled")]
pub fn use_mobile_phone_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut res = rd::<GPtr<u8>>(this, 8);
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if !pd.is_null() {
        wr::<u8>(pd, 0x85, 1);
    }
    if task_get_type(rd::<GPtr<u8>>(this, 8)) != 0xCA {
        let a1 = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), 0x91);
        let a2 = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), 0x92);
        let k = X87::from_f64_exact(g::<f64>(0x86E4A0).read());
        if !a1.is_null() {
            if !fcom_any(x(rd::<f32>(a1, 0x20)), x(g::<f32>(0x862E1C).read()), 1) {
                add_weapon_model(ped, 0x14A);
            }
        } else if !a2.is_null() {
            if !fcom_any(x(rd::<f32>(a2, 0x20)), k, 1)
                && lt(x(rd::<f32>(a2, 0x20)).fsub_m(rd::<f32>(a2, 0x28)), k)
            {
                use_mobile_phone_restore_weapon_o(this, ped);
            }
        } else if task_timer_is_out_of_time(this.byte_add(0x10)) != 0 {
            let m = task_new(0x20);
            res = if m.is_null() {
                GPtr::NULL
            } else {
                run_anim_ctor2(m, 0, 0x92, 0x4080_0000, 0x643, 0x86E48C, 0)
            };
        }
    }
    if !rd::<GPtr<u8>>(ped, 0x480).is_null() {
        wr::<u8>(rd::<GPtr<u8>>(ped, 0x480), 0x84, 1);
        let pd = rd::<GPtr<u8>>(ped, 0x480);
        if lt(x(rd::<f32>(pd, 0x18)), x(g::<f32>(0x858B50).read())) {
            wr::<u32>(pd, 0x18, 0);
        }
        let t = intel_find_task_by_type(rd::<GPtr<u8>>(ped, 0x47C), 0);
        if !t.is_null() {
            player_control_zelda_o(t, ped, 1);
        }
        wr::<u8>(rd::<GPtr<u8>>(ped, 0x480), 0x84, 0);
    }
    res
}

/// `CTaskComplexFallToDeath::ControlSubTask(CPed*)` (0x679510, `ret 4`). A ped flagged +0x40 bit 0x100 (once, +0x21 bit
/// 2): +0x470 |= 0x80000 and a `CTaskComplexDie(0x35, 0, 0x8C, 4.0, 1.0, false, false, 0, false)` (null when `new`
/// fails). Otherwise, retried in air (bit 1) but not yet down (bit 2), with a sub-task other than 0xDA and `|z| < 0.01`
/// (move speed +0x4C): bits 2 and 4 set, both fall anims (+0x18, +0x1C, when not -1) get blend delta -1000, and an
/// inlined `CTaskSimpleLand(0x18)` (flags `(b & 0xFC) | 4`) replaces the sub-task (null when `new` fails). Else the
/// sub-task.
#[hook(addr = 0x679510, conv = thiscall, name = "CTaskComplexFallToDeath::ControlSubTask", expect = "64 A1 00 00 00 00 6A FF 68 96 60 84 00",
       this = "CTaskComplexFallToDeath",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes",
       note = "SEH frame not modelled")]
pub fn fall_to_death_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(ped, 0x40) & 0x100 != 0 && rd::<u8>(this, 0x21) & 2 == 0 {
        wr::<u8>(this, 0x21, rd::<u8>(this, 0x21) | 2);
        wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) | 0x8_0000);
        let m = task_new(0x28);
        if m.is_null() {
            return GPtr::NULL;
        }
        return die_ctor(m, 0x35, 0, 0x8C, 4.0, 1.0, 0, 0, 0, 0);
    }
    let fl = rd::<u8>(this, 0x21);
    if fl & 1 == 0 || fl & 2 != 0 || task_get_type(sub) == 0xDA {
        return sub;
    }
    if !abs_lt(rd::<f32>(ped, 0x4C), f32::from_bits(0x3C23_D70A)) {
        return sub;
    }
    wr::<u8>(this, 0x21, rd::<u8>(this, 0x21) | 6);
    for o in [0x18, 0x1C] {
        let anim = rd::<u32>(this, o);
        if anim != 0xFFFF_FFFF {
            let a = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), anim);
            if !a.is_null() {
                wr::<u32>(a, 0x1C, 0xC47A_0000);
            }
        }
    }
    let m = task_new(0x14);
    if m.is_null() {
        return GPtr::NULL;
    }
    task_simple_ctor(m);
    wr::<u8>(m, 0x10, (rd::<u8>(m, 0x10) & 0xFC) | 4);
    wr::<u32>(m, 0, 0x8704FC);
    wr::<u32>(m, 8, 0);
    wr::<u32>(m, 0xC, 0x18);
    m
}

/// `CTaskSimpleJetPack::DoJetPackEffect(CPed*)` (0x67B7F0, `ret 4`). With a ped clump whose frame matrix pointer is
/// not null, each missing thruster effect (+0x64, then +0x68) is created (`g_fxMan.CreateFxSystem("jetpack", (0, 0,
/// 0), that matrix, false)`), played, set to local particles and to copy its parent matrix, the key time +0x6C reset.
/// The static offset 0xC195D8 = (0, 0, -0.15) is set up once (guard 0xC195E4 bit 0). Each live effect follows its
/// jet ball frame's LTM with that offset. The key time moves by 0.1 towards 1 when thrusting (+0xE, or a strafe +0x10
/// that is not 0, NaN included) and towards 0 otherwise (compared on the x87 value before the f32 store, clamped to
/// 1 / 0), and becomes each live effect's constant time.
#[hook(addr = 0x67B7F0, conv = thiscall, name = "CTaskSimpleJetPack::DoJetPackEffect", expect = "8B 44 24 04 8B 40 18 83 EC 0C",
       this = "CTaskSimpleJetPack",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes")]
pub fn jet_pack_do_effect(this: GPtr<u8>, ped: GPtr<u8>) {
    let clump = rd::<GPtr<u8>>(ped, 0x18);
    if !clump.is_null() {
        let mat = rd::<GPtr<u8>>(clump, 4).byte_add(0x10);
        if !mat.is_null() {
            for o in [0x64, 0x68] {
                if rd::<u32>(this, o) != 0 {
                    continue;
                }
                let mut zero = [0u32; 3];
                let fx = fx_create_system(GPtr::from_addr(0xA9AE80), GPtr::from_addr(0x870600),
                                          GPtr::from_raw(zero.as_mut_ptr()).cast(), mat, 0);
                wr::<GPtr<u8>>(this, o, fx);
                if !fx.is_null() {
                    fx_play(fx);
                    fx_set_local_particles(rd::<GPtr<u8>>(this, o), 1);
                    fx_copy_parent_matrix(rd::<GPtr<u8>>(this, o));
                    wr::<u32>(this, 0x6C, 0);
                }
            }
        }
    }
    if g::<u32>(0xC195E4).read() & 1 == 0 {
        g::<u32>(0xC195E4).write(g::<u32>(0xC195E4).read() | 1);
        g::<u32>(0xC195D8).write(0);
        g::<u32>(0xC195DC).write(0);
        g::<u32>(0xC195E0).write(0xBE19_999A);
    }
    for (o, name) in [(0x64, 0x8705F4usize), (0x68, 0x8705E8)] {
        if rd::<u32>(this, o) == 0 {
            continue;
        }
        let f = clump_get_frame_from_name(rd::<GPtr<u8>>(this, 0x40), GPtr::from_addr(name));
        let ltm = frame_get_ltm(f);
        fx_system_set_matrix(rd::<GPtr<u8>>(this, o), ltm);
        fx_set_offset_pos(rd::<GPtr<u8>>(this, o), GPtr::from_addr(0xC195D8));
    }
    let on = rd::<u8>(this, 0xE) != 0 || fcom_ne(x(rd::<f32>(this, 0x10)), x(g::<f32>(0x858B50).read()));
    let v = xi(i32::from(on));
    let key = rd::<f32>(this, 0x6C);
    let step = g::<f32>(0x858B1C).read();
    if gt(v, x(key)) {
        let n = x(key).fadd_m(step);
        wr::<f32>(this, 0x6C, n.to_f32());
        if gt(n, x(g::<f32>(0x858624).read())) {
            wr::<u32>(this, 0x6C, 0x3F80_0000);
        }
    } else if lt(v, x(key)) {
        let n = x(key).fsub_m(step);
        wr::<f32>(this, 0x6C, n.to_f32());
        if lt(n, x(g::<f32>(0x858B50).read())) {
            wr::<u32>(this, 0x6C, 0);
        }
    }
    for o in [0x64, 0x68] {
        let fx = rd::<GPtr<u8>>(this, o);
        if !fx.is_null() {
            fx_set_const_time(fx, 1, rd::<f32>(this, 0x6C));
        }
    }
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_USE_MOBILE_PHONE_RESTORE_WEAPON,
    HOOK_USE_MOBILE_PHONE_MAKE_ABORTABLE,
    HOOK_PATROL_ROUTE_REVERSE,
    HOOK_FOLLOW_PATROL_ROUTE_NEXT_TYPE,
    HOOK_FOLLOW_PATROL_ROUTE_CREATE_SUB_TASK,
    HOOK_FOLLOW_PATROL_ROUTE_CREATE_NEXT_SUB_TASK,
    HOOK_FALL_TO_DEATH_CREATE_NEXT_SUB_TASK,
    HOOK_SHOULD_LOAD_SUNBATHE_ANIMS,
    HOOK_SUNBATHE_CONTROL_SUB_TASK,
    HOOK_JET_PACK_RENDER,
    HOOK_GUN_AIMED_AT_TAKES_PRIORITY_OVER,
    HOOK_RADIUS_ANGLE_OFFSET_COMPUTE_ENTITY_SEEK_POS,
    HOOK_GANG_JOIN_RESPOND_CONTROL_SUB_TASK,
    HOOK_FOLLOW_ANY_MEANS_ALLOCATE_DEFAULT_TASKS,
    HOOK_GO_TO_POINT_ANY_MEANS_CONTROL_SUB_TASK,
    HOOK_USE_MOBILE_PHONE_CONTROL_SUB_TASK,
    HOOK_FALL_TO_DEATH_CONTROL_SUB_TASK,
    HOOK_JET_PACK_DO_EFFECT,
];
