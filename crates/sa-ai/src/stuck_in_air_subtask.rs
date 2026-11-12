//! `CTaskComplexStuckInAir::CreateSubTask` (0x67BA80), `CTaskComplexStuckInAir::ControlSubTask` (0x67BE50) and the `CEventObjectCollision` destructor (0x4ACDD0), reached
//! when a ped is stuck in the air / bumps an object while walking (batch `std11-ai-1`). Ported from the 1.0 US bytes;
//! unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, thiscall, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CTaskSimpleFallAndGetUp(int32 anim, int32 group, int32 time)` (0x6786C0)
    fn fall_and_get_up_ctor(t: GPtr<u8>, anim: u32, group: u32, time: i32) -> GPtr<u8> = thiscall 0x6786C0 => unported;
    /// `CTaskSimpleStandStill(int32 time, bool looped, bool keep, float delta)` (0x62F310)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, looped: u32, keep: u32, delta: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskComplex::CTaskComplex()` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskComplexGoToPointAndStandStill(...)`-style constructor (0x65B390): (target, mode, radius, time)
    fn go_to_point_ctor(t: GPtr<u8>, target: GPtr<u8>, mode: u32, radius: u32, time: u32) -> GPtr<u8> = thiscall 0x65B390 => unported;
    /// `CTask::GetSubTask()` (vtable slot 8)
    fn task_get_sub_task(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x8 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, eAbortPriority, const CEvent*)` (vtable slot 0x18)
    fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CPed::GetPadFromPlayer()` (0x609560)
    fn get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPad::GetPedWalkLeftRight()` (0x53FC90)
    fn pad_get_ped_walk_left_right(p: GPtr<u8>) -> u16 = thiscall 0x53FC90 => unported;
    /// `CPad::GetPedWalkUpDown()` (0x53FD30)
    fn pad_get_ped_walk_up_down(p: GPtr<u8>) -> u16 = thiscall 0x53FD30 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, ST0)
    fn general_get_radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn general_limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CPad::JumpJustDown()` (0x540770)
    fn pad_jump_just_down(pad: GPtr<u8>) -> u8 = thiscall 0x540770 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association(clump: GPtr<u8>, anim: i32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
}

/// `CEventObjectCollision::~CEventObjectCollision` (0x4ACDD0, thiscall; entry `push -1` then the protection chunk
/// 0x406A96, which builds the SEH frame and returns to 0x4ACDD7): vtable 0x85ACA8; with the object (+0x14) set,
/// `object->CleanUpOldReference(&this->object)`; then the `CEvent` vtable 0x85AB38.
#[hook(addr = 0x4ACDD0, conv = thiscall, name = "FUN_004acdd0", expect = "6A FF E9 BF 9C F5 FF",
       unverified = "std11-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_object_collision_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85ACA8);
    let o = rd::<GPtr<u8>>(this, 0x14);
    if !o.is_null() {
        entity_clean_up_old_reference(o, this.byte_add(0x14));
    }
    wr::<u32>(this, 0, 0x85AB38);
}

/// `CTaskComplexStuckInAir::CreateSubTask(int32 type, CPed* ped)` (0x67BA80, thiscall, `ret 8`; `this` is not read).
/// 0xD0: the intelligence's +0x270 / +0x272 words cleared, `new CTaskSimpleFallAndGetUp(0x1A, 0, 1000)` (0x18 bytes).
/// 0xCB: standing flag set, `new CTaskSimpleStandStill(5000, false, false, 8.0)` (0x20). 0xD3: standing flag set, the
/// words cleared, a 0x14-byte `CTaskComplex` with vtable 0x870570, byte +0x10 = 0 and +0xC = -1. 0x38C: standing flag
/// set; T = position - forward * k (k = [0x858B8C]; x, y as f32, z with the unrounded product), then ± right * k by
/// `rand() & 1` (+ when odd), `new` (0x3C) task 0x65B390 (&T, 0, 5.0, 10000). Other types, or a failed allocation: 0.
///
/// Stack: the original keeps forward.y * k in its own `ped` argument slot (not reproduced: the slot is dead after it).
#[hook(addr = 0x67BA80, conv = thiscall, name = "CTaskComplexStuckInAir::CreateSubTask", expect = "64 A1 00 00 00 00 8B 4C 24 04",
       unverified = "std11-ai-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk")]
pub fn task_complex_stuck_in_air_create_sub_task(_this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let clear_words = || {
        let intel = rd::<GPtr<u8>>(ped, 0x47C);
        wr::<u16>(intel, 0x270, 0);
        wr::<u16>(intel, 0x272, 0);
    };
    let standing = || wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 1);
    match ty {
        0xD0 => {
            clear_words();
            let t = task_new(0x18);
            if t.is_null() { t } else { fall_and_get_up_ctor(t, 0x1A, 0, 1000) }
        }
        0xCB => {
            standing();
            let t = task_new(0x20);
            if t.is_null() { t } else { stand_still_ctor(t, 5000, 0, 0, 0x4100_0000) }
        }
        0xD3 => {
            standing();
            clear_words();
            let t = task_new(0x14);
            if t.is_null() {
                return t;
            }
            task_complex_ctor(t);
            wr::<u8>(t, 0x10, 0);
            wr::<u32>(t, 0, 0x870570);
            wr::<i32>(t, 0xC, -1);
            t
        }
        0x38C => {
            standing();
            let m = rd::<GPtr<u8>>(ped, 0x14);
            let k = g::<f32>(0x858B8C).read();
            let az = x(rd::<f32>(m, 0x18)).fmul_m(k);
            let fy = f32_of(x(rd::<f32>(m, 0x14)).fmul_m(k));
            let fx = f32_of(x(rd::<f32>(m, 0x10)).fmul_m(k));
            let p = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
            let tz = az.fsubr_m(rd::<f32>(p, 8));
            let ty_ = x(rd::<f32>(p, 4)).fsub_m(fy);
            let mut t = [f32_of(x(rd::<f32>(p, 0)).fsub_m(fx)), f32_of(ty_), f32_of(tz)];
            let odd = at_site(0x67BC62, rand) & 1 != 0;
            let r = rd::<GPtr<u8>>(ped, 0x14);
            let (rz, ry, rx) = (x(rd::<f32>(r, 8)).fmul_m(k), x(rd::<f32>(r, 4)).fmul_m(k), x(rd::<f32>(r, 0)).fmul_m(k));
            for (i, v) in [(0, rx), (1, ry), (2, rz)] {
                t[i] = f32_of(if odd { v.fadd_m(t[i]) } else { x(t[i]).fsub(v) });
            }
            let task = task_new(0x3C);
            if task.is_null() {
                return task;
            }
            go_to_point_ctor(task, GPtr::from_raw(t.as_mut_ptr()).cast(), 0, 0x40A0_0000, 10000)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexStuckInAir::ControlSubTask(CPed* ped)` (0x67BE50, thiscall, `ret 4`). By the sub-task's (+8) type:
/// 0x38C with the intelligence's +0x272 word set → `CreateSubTask(0xCB)`. 0xD3 with the word set and a sub-sub-task
/// of type 0xF1 (or one level deeper) → `CreateSubTask(0xCB)`. 0xCB: without the word → 0; else, for a player, the
/// pad's walk left/right and up/down (as floats; either non-zero in absolute value) turn the aiming rotation (+0x55C) to
/// `LimitRadianAngle(GetRadianAngleBetweenPoints(0, 0, -lr, ud) - TheCamera orientation [0xB6F178])`, and a jump
/// press → `CreateSubTask(0xD3)`; then the in-the-air flag cleared, the standing flag set and the anims 0x74..0x79
/// with a blend amount above 0 and a delta not below 0 get delta -8. 0xD0 with a sub-sub-task of type 0xCF whose byte
/// +8 is set and the word set: standing flag set. A result other than the current sub-task aborts it
/// (`MakeAbortable(ped, 1, null)`).
///
/// Stack: the original keeps the up/down float and the angle in its own `ped` argument slot (dead afterwards).
#[hook(addr = 0x67BE50, conv = thiscall, name = "CTaskComplexStuckInAir::ControlSubTask", expect = "51 55 56 57 8B F9 8B 6F 08",
       unverified = "std11-ai-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk")]
pub fn task_complex_stuck_in_air_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    let word = || rd::<u16>(rd::<GPtr<u8>>(ped, 0x47C), 0x272) != 0;
    let cur = || rd::<GPtr<u8>>(this, 8);
    let mut res = sub;
    match task_get_type(sub) {
        0x38C => {
            if word() {
                res = task_complex_stuck_in_air_create_sub_task(this, 0xCB, ped);
            }
        }
        0xD3 => {
            if word() && !task_get_sub_task(cur()).is_null() {
                let deep = task_get_type(task_get_sub_task(cur())) == 0xF1 || {
                    let ss = task_get_sub_task(task_get_sub_task(cur()));
                    !ss.is_null() && task_get_type(task_get_sub_task(task_get_sub_task(cur()))) == 0xF1
                };
                if deep {
                    res = task_complex_stuck_in_air_create_sub_task(this, 0xCB, ped);
                }
            }
        }
        0xCB => {
            if !word() {
                res = GPtr::NULL;
            } else {
                if rd::<u32>(ped, 0x480) != 0 {
                    let pad = get_pad_from_player(ped);
                    let lr = f32_of(X87::from_i32(i32::from(pad_get_ped_walk_left_right(pad) as i16)));
                    let ud = f32_of(X87::from_i32(i32::from(pad_get_ped_walk_up_down(pad) as i16)));
                    let z = x(g::<f32>(0x858B50).read());
                    let moved = !x(lr).fabs().fcom(z).test_any(0x41) || !x(ud).fabs().fcom(z).test_any(0x41);
                    if moved {
                        let neg_lr = f32_of(x(lr).fchs());
                        let a = general_get_radian_angle_between_points(0, 0, neg_lr.to_bits(), ud.to_bits());
                        let a = f32_of(X87::from_f64_exact(a).fsub_m(g::<f32>(0xB6F178).read()));
                        let r = general_limit_radian_angle(a);
                        wr::<f32>(ped, 0x55C, f32_of(X87::from_f64_exact(r)));
                    }
                    if pad_jump_just_down(pad) != 0 {
                        res = task_complex_stuck_in_air_create_sub_task(this, 0xD3, ped);
                    }
                }
                wr::<u32>(ped, 0x46C, (rd::<u32>(ped, 0x46C) & 0xFFFF_FDFF) | 1);
                for id in 0x74..=0x79 {
                    let a = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), id);
                    if a.is_null() {
                        continue;
                    }
                    let z = x(g::<f32>(0x858B50).read());
                    if !x(rd::<f32>(a, 0x18)).fcom(z).test_any(0x41) && !x(rd::<f32>(a, 0x1C)).fcom(z).test_any(0x01) {
                        wr::<u32>(a, 0x1C, 0xC100_0000);
                    }
                }
            }
        }
        0xD0 => {
            let ss = task_get_sub_task(cur());
            if !ss.is_null()
                && task_get_type(task_get_sub_task(cur())) == 0xCF
                && rd::<u8>(task_get_sub_task(cur()), 8) != 0
                && word()
            {
                wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 1);
            }
        }
        _ => {}
    }
    let cur = rd::<GPtr<u8>>(this, 8);
    if res != cur {
        task_make_abortable(cur, ped, 1, 0);
    }
    res
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_EVENT_OBJECT_COLLISION_DTOR,
    HOOK_TASK_COMPLEX_STUCK_IN_AIR_CREATE_SUB_TASK,
    HOOK_TASK_COMPLEX_STUCK_IN_AIR_CONTROL_SUB_TASK,
];
