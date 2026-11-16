//! FRONTIER-2 turn 6, lane A (batch `frontier2-a9`): AI frontier entries ported from the 1.0 US bytes (gta-reversed
//! read as an aid; where it differs the bytes win, noted per function).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::bindings::clump_get_association;
use sa_core::filemgr::strcpy;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{go_to_point_ctor, in_air_ctor, simple_dead_ctor, task_new, task_simple_ctor};
use crate::common::{g, rd, task_get_type, task_make_abortable, wr};
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::{
    anim_blend_animation, follow_node_route_ctor, go_to_point_and_stand_still_ctor, intel_clear_tasks,
    ped_remove_weapon_model, ped_set_current_weapon, run_named_anim_ctor,
};

orig! {
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
fn route_pos(route: GPtr<u8>, i: i32) -> GPtr<u8> {
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

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_USE_MOBILE_PHONE_RESTORE_WEAPON,
    HOOK_USE_MOBILE_PHONE_MAKE_ABORTABLE,
    HOOK_PATROL_ROUTE_REVERSE,
    HOOK_FOLLOW_PATROL_ROUTE_NEXT_TYPE,
    HOOK_FOLLOW_PATROL_ROUTE_CREATE_SUB_TASK,
    HOOK_FOLLOW_PATROL_ROUTE_CREATE_NEXT_SUB_TASK,
    HOOK_FALL_TO_DEATH_CREATE_NEXT_SUB_TASK,
];
