//! `CTaskComplexKillPedOnFootMelee` (batch `frontier-1-1`): a ped fist-fighting another (a jacked driver who fights
//! back, a ped CJ punches). `CreateSubTask`, `ControlSubTask`, `CreateNextSubTask`, and the
//! `CTaskSimpleFightingControl` constructor its fight sub-task uses. Ported from the 1.0 US bytes (gta-reversed is a
//! reading aid; its range tests and the water branches differ from the bytes, which win).
//!
//! Fields: +8 sub-task, +0xC target, +0x10 the search position, +0x1C the arrive range, +0x20 the attack range,
//! +0x24 the attack angle, +0x31 "new target", +0x34 the pause timer (-1 = not started).

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_world::world::common::entity_pos;

use crate::common::*;

use crate::bindings::stand_still_dtor;
use crate::bindings::stand_still_process_ped;
use sa_world::bindings::seek_entity_standard_ctor;
use sa_world::bindings::sequence_ctor;
use sa_world::bindings::shake_fist_ctor;
use sa_world::bindings::turn_to_face_entity_ctor;
use sa_world::bindings::pause_ctor;
use sa_world::bindings::sequence_add_task;
use sa_world::bindings::stand_still_ctor;
orig! {
    /// `CTaskComplexKillPedOnFootMelee::CreateSubTask(eTaskType, CPed*)` (0x626AC0)
    fn create_sub_task(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x626AC0 => kill_ped_on_foot_melee_create_sub_task;
    /// `CTaskComplexKillPedOnFootMelee::CalculateSearchPositionAndRanges(CPed*)` (0x621100)
    fn calculate_search_position_and_ranges(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x621100 => unported;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable slot 0x24)
    fn task_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CTaskSimpleFightingControl::CTaskSimpleFightingControl(CEntity*, float angle, float range)` (0x61DC10)
    fn fighting_control_ctor(t: GPtr<u8>, target: GPtr<u8>, angle: u32, range: u32) -> GPtr<u8> = thiscall 0x61DC10 => fighting_control_ctor_61dc10;
}

/// `CTaskSimpleFightingControl::CTaskSimpleFightingControl(CEntity* target, float angle, float range)` (0x61DC10,
/// `ret 0xC`; entry `push -1; jmp 0x406842`, a protection fragment that only pushes the SEH handler): `CTaskSimple`
/// base, vtable 0x86D700, +8 = 0 (byte), +0xA = 0 (word), the target at +0xC (its reference registered when
/// non-null), angle +0x10, range +0x14, +0x1C = +0x20 = 0, +0x18 = 1.0.
#[hook(addr = 0x61DC10, conv = thiscall, name = "CTaskSimpleFightingControl::Constructor",
       expect = "6A FF E9 2B 8C DE FF",
       unverified = "frontier-1-1", route = "L2", hazards = "heap-writes",
       note = "the SEH frame (protection fragment 0x406842) is not reproduced")]
pub fn fighting_control_ctor_61dc10(this: GPtr<u8>, target: GPtr<u8>, angle: u32, range: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    wr::<u32>(this, 0, 0x86_D700);
    wr::<u8>(this, 8, 0);
    wr::<u16>(this, 0xA, 0);
    wr::<GPtr<u8>>(this, 0xC, target);
    if !target.is_null() {
        entity_register_reference(target, this.byte_add(0xC));
    }
    wr::<u32>(this, 0x10, angle);
    wr::<u32>(this, 0x14, range);
    wr::<u32>(this, 0x1C, 0);
    wr::<u32>(this, 0x20, 0);
    wr::<u32>(this, 0x18, 0x3F80_0000);
    this
}

/// `CTaskComplexKillPedOnFootMelee::CreateSubTask(eTaskType, CPed*)` (0x626AC0, thiscall, `ret 8`):
/// - 0xF4: the pause timer = -1; a sequence of `CTaskComplexTurnToFaceEntityOrCoord(target, [0x86FC7C], [0x86FC80])`
///   and `CTaskSimpleShakeFist` (each added even when its allocation failed; `AddTask` is called on a null sequence
///   when the sequence's allocation failed, as the original);
/// - 0xCB: the timer = -1; `CTaskSimpleStandStill(2000, false, false, 8.0)`;
/// - 0xCA: a stack `CTaskSimpleStandStill(0, false, false, 8.0)` processes the ped once, the timer starts (now) when
///   it is -1; `CTaskSimplePause(100)`;
/// - 0x38B: the timer = -1; `CTaskComplexSeekEntity<Standard>(target, 50000, 1000, [0x85BACC], [0x859E30],
///   [0x859E34], true, true)`;
/// - 0x3FB: the timer = -1; `CTaskSimpleFightingControl(target, +0x24, +0x20)`;
/// - 0x516: the timer = -1, null; anything else (or a failed allocation): null.
#[hook(addr = 0x626AC0, conv = thiscall, name = "CTaskComplexKillPedOnFootMelee::CreateSubTask",
       expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier-1-1", route = "L2", hazards = "heap-writes",
       note = "the stack CTaskSimpleStandStill is zero-initialised before its constructor; the SEH frame is not reproduced")]
pub fn kill_ped_on_foot_melee_create_sub_task(this: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> {
    let target = || rd::<GPtr<u8>>(this, 0xC);
    match ty {
        0xF4 => {
            wr::<i32>(this, 0x34, -1);
            let s = task_new(0x40);
            let seq = if s.is_null() { s } else { sequence_ctor(s) };
            let t = task_new(0x28);
            let turn = if t.is_null() {
                t
            } else {
                turn_to_face_entity_ctor(t, target(), g::<u32>(0x86_FC7C).read(), g::<u32>(0x86_FC80).read())
            };
            sequence_add_task(seq, turn);
            let f = task_new(0x10);
            let fist = if f.is_null() { f } else { shake_fist_ctor(f) };
            sequence_add_task(seq, fist);
            seq
        }
        0xCB => {
            wr::<i32>(this, 0x34, -1);
            let t = task_new(0x20);
            if t.is_null() { t } else { stand_still_ctor(t, 2000, 0, 0, 8.0) }
        }
        0xCA => {
            let mut st = [0u32; 8];
            let sp: GPtr<u8> = GPtr::from_raw(st.as_mut_ptr()).cast();
            stand_still_ctor(sp, 0, 0, 0, 8.0);
            stand_still_process_ped(sp, ped);
            stand_still_dtor(sp);
            if rd::<i32>(this, 0x34) == -1 {
                wr::<u32>(this, 0x34, g::<u32>(0xB7_CB84).read());
            }
            let t = task_new(0x18);
            if t.is_null() { t } else { pause_ctor(t, 100) }
        }
        0x38B => {
            wr::<i32>(this, 0x34, -1);
            let t = task_new(0x4C);
            if t.is_null() {
                t
            } else {
                seek_entity_standard_ctor(
                    t,
                    target(),
                    50_000,
                    1000,
                    g::<u32>(0x85_BACC).read(),
                    g::<u32>(0x85_9E30).read(),
                    g::<u32>(0x85_9E34).read(),
                    1,
                    1,
                )
            }
        }
        0x3FB => {
            wr::<i32>(this, 0x34, -1);
            let t = task_new(0x24);
            if t.is_null() {
                t
            } else {
                fighting_control_ctor(t, target(), rd::<u32>(this, 0x24), rd::<u32>(this, 0x20))
            }
        }
        0x516 => {
            wr::<i32>(this, 0x34, -1);
            GPtr::NULL
        }
        _ => GPtr::NULL,
    }
}

/// `|search position - ped|²` after `CalculateSearchPositionAndRanges(ped)`, compared with `range²` (`fcompp`):
/// the x87 status of `range² vs distance²`.
fn range_vs_distance(this: GPtr<u8>, ped: GPtr<u8>, range_off: isize) -> sa_math::x87::Cmp {
    calculate_search_position_and_ranges(this, ped);
    let r = rd::<f32>(this, range_off);
    let mut d = [0u32; 3];
    let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
    let out = vector_sub(dp, this.byte_add(0x10), entity_pos(ped));
    let sq = vector_magnitude_sqr_x(out);
    x(r).fmul_m(r).fcom(sq)
}

/// `CTaskComplexKillPedOnFootMelee::ControlSubTask(CPed*)` (0x626D90, `ret 4`). The ped says 0x59 (1.0); a new
/// target restarts (`CreateFirstSubTask`); no target, or one whose health (+0x540) is not above 0, ends (null). By
/// the sub-task's type: 0xF4 (sequence) → fight (0x3FB) unless the target is in water (+0x40 bit 8); 0xCA / 0x38B
/// (pause, seek) → the sequence when in water, else fight when `range² > |d|²` (arrive range); 0x3FB (fight) → the
/// sequence when in water, else when `attack range² < |d|²` a pause (`bStayInSamePlace`, +0x46C bit 21) or a
/// seek. A change happens only when the sub-task aborts (`MakeAbortable(ped, 1, null)`); otherwise the sub-task.
///
/// Stack: the original stores the range into its own `ped` argument slot; not reproduced.
#[hook(addr = 0x626D90, conv = thiscall, name = "CTaskComplexKillPedOnFootMelee::ControlSubTask",
       expect = "83 EC 18 53 56 57",
       unverified = "frontier-1-1", route = "L2", hazards = "heap-writes",
       note = "the range is stored into the ped argument slot by the original")]
pub fn kill_ped_on_foot_melee_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    ped_say(ped, 0x59, 0, 1.0, 0, 0, 0);
    if rd::<u8>(this, 0x31) != 0 {
        return task_create_first_sub_task(this, ped);
    }
    let target = rd::<GPtr<u8>>(this, 0xC);
    if target.is_null() || !x(rd::<f32>(target, 0x540)).fcom(x(g::<f32>(0x85_8B50).read())).test_parity(0x41) {
        return GPtr::NULL;
    }
    let in_water = || rd::<u32>(rd::<GPtr<u8>>(this, 0xC), 0x40) & 0x100 != 0;
    let abort_and = |ty: u32| {
        if task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, GPtr::NULL) != 0 {
            create_sub_task(this, ty, ped)
        } else {
            sub
        }
    };
    match task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0xF4 => {
            if in_water() {
                sub
            } else {
                abort_and(0x3FB)
            }
        }
        0xCA | 0x38B => {
            if in_water() {
                return abort_and(0xF4);
            }
            if range_vs_distance(this, ped, 0x1C).test_any(0x41) {
                sub
            } else {
                abort_and(0x3FB)
            }
        }
        0x3FB => {
            if in_water() {
                return abort_and(0xF4);
            }
            if range_vs_distance(this, ped, 0x20).test_parity(0x05) {
                return sub;
            }
            abort_and(if rd::<u32>(ped, 0x46C) & 0x20_0000 != 0 { 0xCA } else { 0x38B })
        }
        _ => sub,
    }
}

/// `CTaskComplexKillPedOnFootMelee::CreateNextSubTask(CPed*)` (0x62BC10, `ret 4`). Null without a target. By the
/// finished sub-task: 0x38B (seek) → fight; 0xCB / 0xF4 → stand still (0xCB) when the target is in water, else
/// `CreateFirstSubTask`; 0xCA (pause) → fight when `arrive range² > |d|²` (or equal, or NaN), else a seek unless
/// `bStayInSamePlace`, then finished (0x516) once the pause timer has run more than 3000 ms, else another pause;
/// 0x3FB (fight) → a pause with `bStayInSamePlace`, else after `CalculateSearchPositionAndRanges` a seek when
/// `arrive range² < (dz² + dx²) + dy²` (inline, x87), else a pause; 0x516 and anything else → null.
#[hook(addr = 0x62BC10, conv = thiscall, name = "CTaskComplexKillPedOnFootMelee::CreateNextSubTask",
       expect = "83 EC 0C 56 8B F1",
       unverified = "frontier-1-1", route = "L2", hazards = "heap-writes",
       note = "the arrive range is stored into the ped argument slot by the original (pause case)")]
pub fn kill_ped_on_foot_melee_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(this, 0xC).is_null() {
        return GPtr::NULL;
    }
    let stay = || rd::<u32>(ped, 0x46C) & 0x20_0000 != 0;
    match task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0x38B => create_sub_task(this, 0x3FB, ped),
        0xCB | 0xF4 => {
            if rd::<u32>(rd::<GPtr<u8>>(this, 0xC), 0x40) & 0x100 != 0 {
                create_sub_task(this, 0xCB, ped)
            } else {
                task_create_first_sub_task(this, ped)
            }
        }
        0xCA => {
            if range_vs_distance(this, ped, 0x1C).test_parity(0x05) {
                return create_sub_task(this, 0x3FB, ped);
            }
            if !stay() {
                return create_sub_task(this, 0x38B, ped);
            }
            let t = rd::<i32>(this, 0x34);
            if t != -1 && g::<u32>(0xB7_CB84).read().wrapping_sub(t as u32) > 3000 {
                create_sub_task(this, 0x516, ped)
            } else {
                create_sub_task(this, 0xCA, ped)
            }
        }
        0x3FB => {
            if stay() {
                return create_sub_task(this, 0xCA, ped);
            }
            calculate_search_position_and_ranges(this, ped);
            let p = entity_pos(ped);
            let dx = x(rd::<f32>(this, 0x10)).fsub_m(rd::<f32>(p, 0));
            let dy = x(rd::<f32>(this, 0x14)).fsub_m(rd::<f32>(p, 4));
            let dz = x(rd::<f32>(this, 0x18)).fsub_m(rd::<f32>(p, 8));
            let r = x(rd::<f32>(this, 0x1C));
            let sq = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
            if r.fmul(r).fcom(sq).test_parity(0x05) {
                create_sub_task(this, 0xCA, ped)
            } else {
                create_sub_task(this, 0x38B, ped)
            }
        }
        _ => GPtr::NULL,
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_FIGHTING_CONTROL_CTOR_61DC10,
    HOOK_KILL_PED_ON_FOOT_MELEE_CREATE_SUB_TASK,
    HOOK_KILL_PED_ON_FOOT_MELEE_CONTROL_SUB_TASK,
    HOOK_KILL_PED_ON_FOOT_MELEE_CREATE_NEXT_SUB_TASK,
];
