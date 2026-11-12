//! `CTaskComplexWalkAlongsidePed::ControlSubTask` (batch `mvpn-ai-1`): keeps a ped walking beside its target ped,
//! matching the two walk animation speeds. Ported from the 1.0 US bytes (gta-reversed
//! `TaskComplexWalkAlongsidePed.cpp` as a reading aid; the bytes win, see the doc comment). Unverified
//! (docs/verify/HOWTO.md).
//!
//! Layout: +8 sub-task, +0xC target ped, +0x10 give-up distance, +0x14 offset (CVector). Seek sub-task (type 0x38B):
//! +0x20 minimum 2D entity distance, +0x44 the position calculator's offset.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18, `ret 0xC`)
    fn v_task_make_abortable(t: GPtr<u8>, p: GPtr<u8>, prio: u32, e: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn clump_get_association(clump: u32, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CPedIntelligence::GetMoveStateFromGoToTask()` (0x601D70)
    fn intel_get_move_state_from_go_to_task(intel: GPtr<u8>) -> i32 = thiscall 0x601D70 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, hidden result pointer first)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `DotProduct(const CVector*, const CVector*)` (0x40FDB0), ST0
    fn vector_dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `CVector::SquaredMagnitude()` (0x406DA0), ST0
    fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, cdecl, hidden result pointer first; reads the vector before
    /// writing the result)
    fn vector_scale(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00, `ret 4`; component by component)
    fn vector_add_assign(this: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x411A00 => unported;
    /// `std::max<float>(a, b)` (0x420800): `a > b ? a : b`, ST0
    fn max_f(a: f32, b: f32) -> f64 = cdecl 0x420800 => unported;
    /// `std::min<float>(a, b)` (0x404330): `a < b ? a : b`, ST0
    fn min_f(a: f32, b: f32) -> f64 = cdecl 0x404330 => unported;
    /// `CPed::SetMoveAnimSpeed(CAnimBlendAssociation*)` (0x5DEC10, `ret 4`)
    fn ped_set_move_anim_speed(ped: GPtr<u8>, anim: GPtr<u8>) = thiscall 0x5DEC10 => unported;
    /// `approxEqual(float a, float b, float eps)` (0x4EEA80): `|a - b| < eps`
    fn approx_equal(a: f32, b: f32, eps: f32) -> u32 = cdecl 0x4EEA80 => unported;
    /// `CTaskSimpleGoToPoint::UpdatePoint(const CVector&, float radius, bool)` (0x645700, `ret 0xC`)
    fn go_to_point_update_point(t: GPtr<u8>, p: GPtr<u8>, radius: f32, b: u32) = thiscall 0x645700 => unported;
    /// `CTaskComplexWalkAlongsidePed::CreateSubTask(int32, CPed*)` (0x685000, `ret 8`)
    fn walk_alongside_create_sub_task(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x685000 => crate::mvpb_ai3::walk_alongside_create_sub_task;
}

/// 0.0f (0x858B50), 2.0f (0x858CA0), 0.0625f (0x858620), 0.0125f (0x85A5C4), 4.0f (0x858B90)
const K_ZERO: usize = 0x858B50;
const K_TWO: usize = 0x858CA0;
const K_QUARTER_SQ: usize = 0x858620;
const K_SPEED_STEP: usize = 0x85A5C4;
const K_FOUR: usize = 0x858B90;
/// The go-to-point radius the walk-alongside task passes (0.5f, a global at 0x86FC78)
const GO_TO_RADIUS: usize = 0x86FC78;
/// Walk animation speed bounds and the approxEqual tolerance (immediates)
const SPEED_MAX: f32 = 1.25;
const SPEED_MIN: f32 = f32::from_bits(0x3F59999A);
const SPEED_EPS: f32 = f32::from_bits(0x3C54FDF4);
/// `bMoveAnimSpeedHasBeenSetByTask` (CPed +0x478 bit 16)
const MOVE_ANIM_SPEED_SET: u32 = 0x10000;

fn kf(a: usize) -> f32 {
    g::<f32>(a).read()
}
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}
fn set_move_anim_speed_flag(ped: GPtr<u8>, on: bool) {
    let f = rd::<u32>(ped, 0x478);
    wr::<u32>(ped, 0x478, (f & !MOVE_ANIM_SPEED_SET) | if on { MOVE_ANIM_SPEED_SET } else { 0 });
}

/// `speed - 0.0125` / `speed + 0.0125` rounded to the f32 argument slot.
fn step(speed: f32, up: bool) -> f32 {
    let s = X87::from_f32(speed);
    if up { s.fadd_m(kf(K_SPEED_STEP)) } else { s.fsub_m(kf(K_SPEED_STEP)) }.to_f32()
}

/// The equal-distance branch for one walk animation: the old speed is kept, `SetMoveAnimSpeed(anim)` computes the
/// new one; within 0.013 of it the old speed is written back, else the old speed steps 0.0125 towards it.
fn settle_speed(ped: GPtr<u8>, anim: GPtr<u8>) {
    let old = rd::<f32>(anim, 0x24);
    ped_set_move_anim_speed(ped, anim);
    let new = rd::<f32>(anim, 0x24);
    let v = if approx_equal(old, new, SPEED_EPS) as u8 != 0 {
        old
    } else if !X87::from_f32(old).fcom(X87::from_f32(new)).test_any(0x41) {
        step(old, false)
    } else {
        step(old, true)
    };
    wr::<f32>(anim, 0x24, v);
}

/// `CTaskComplexWalkAlongsidePed::ControlSubTask(CPed*)` (0x685170, thiscall, `ret 4`).
///
/// Without a target: `MakeAbortable(ped, 1, 0)` ? 0 : the sub-task. Otherwise the two walk associations (anim 0) and
/// whether the target's go-to move state is 4, 6 or 7 are taken; `d = (target - ped)` in 2D (f32 components, z 0) and
/// 0 is returned when `dx² + dy²` (extended) is strictly greater than `giveUp²` (the bytes; gta-reversed says `>=`).
/// A sub-task other than the seek-entity task (0x38B) clears both peds' move-anim-speed-set bits and is returned.
/// Else the seek task gets the offset and a 3.0 minimum distance, the ped's "static counter > 8" (intelligence +0x274)
/// is sampled, and with an active go-to-point task (0x384) and no follow-node-route task (0x38A) the point
/// `targetMatrix * offset` is updated (`UpdatePoint(point, [0x86FC78], false)`) after, when the target moves, the
/// ped is behind it (`ped·fwd - point·fwd < 0`) and the ped has a walk association: within 2.0 (squared 2D distance
/// to the point) the point moves `2 * forward` ahead; at a squared distance below 0.0625 the ped's walk speed drops
/// 0.0125 (min 0.85) and the target's rises (max 1.25); above 0.0625 the reverse; exactly 0.0625 (or NaN) runs the
/// `SetMoveAnimSpeed` settle step for each. With the counter > 8, the stored `dx² + dy²` (f32) below 4.0 and the
/// sub-task abortable, the offset becomes `(dx, dy, 0)` and a stand-still sub-task (0xCB) is returned. Otherwise the
/// ped's bit is set when its speed was changed, the target's when the target's was, and the sub-task is returned.
///
/// Original bugs kept: the `2 * forward` temporary overlaps the point's z (the original's stack slots), so the moved
/// point's z becomes `2 fwd.x + 2 fwd.z`; the dot test uses the ped's (not the target's) position; the original keeps
/// the ped's flag in its own argument slot (not observable).
#[hook(addr = 0x685170, conv = thiscall, name = "CTaskComplexWalkAlongsidePed::ControlSubTask",
       expect = "83 EC 3C 53 56 8B F1", unverified = "mvpn-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn walk_alongside_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let target = rd::<GPtr<u8>>(this, 0xC);
    if target.is_null() {
        let sub = rd::<GPtr<u8>>(this, 8);
        return if v_task_make_abortable(sub, ped, 1, 0) != 0 { GPtr::NULL } else { rd::<GPtr<u8>>(this, 8) };
    }
    let ped_walk = clump_get_association(rd::<u32>(ped, 0x18), 0);
    let target_walk = clump_get_association(rd::<u32>(rd::<GPtr<u8>>(this, 0xC), 0x18), 0);
    let ms = intel_get_move_state_from_go_to_task(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x47C));
    let moving = matches!(ms, 4 | 6 | 7);
    let pp = entity_pos(ped);
    let tp = entity_pos(rd::<GPtr<u8>>(this, 0xC));
    let dx = X87::from_f32(rd::<f32>(tp, 0)).fsub_m(rd::<f32>(pp, 0)).to_f32();
    let dy = X87::from_f32(rd::<f32>(tp, 4)).fsub_m(rd::<f32>(pp, 4)).to_f32();
    let d = X87::from_f32(dy).fmul_m(dy).fadd(X87::from_f32(dx).fmul_m(dx));
    let dist2 = d.to_f32();
    let give_up = X87::from_f32(rd::<f32>(this, 0x10));
    if !d.fcom(give_up.fmul(give_up)).test_any(0x41) {
        return GPtr::NULL;
    }
    let mut ped_flag = false;
    let mut target_flag = false;
    let sub = rd::<GPtr<u8>>(this, 8);
    if v_task_type(sub) == 0x38B {
        let intel = rd::<GPtr<u8>>(ped, 0x47C);
        let off = [rd::<u32>(this, 0x14), rd::<u32>(this, 0x18), rd::<u32>(this, 0x1C)];
        let counter_high = rd::<i32>(intel, 0x274) > 8;
        let seek = rd::<GPtr<u8>>(this, 8);
        for (k, v) in off.iter().enumerate() {
            wr::<u32>(seek, 0x44 + 4 * k as isize, *v);
        }
        wr::<f32>(seek, 0x20, 3.0);
        let route = tm_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x38A);
        let go_to = tm_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x384);
        if !go_to.is_null() && route.is_null() {
            // The original's frame: point at +0, the 2D difference at +0xC, the `2 * forward` temporary at +8.
            let mut buf = [0u32; 6];
            let base: GPtr<u8> = GPtr::from_raw(buf.as_mut_ptr()).cast();
            let point = base;
            let diff = base.byte_add(0xC);
            matrix_mul_vec(point, rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x14), this.byte_add(0x14));
            let ped_pos = entity_pos(ped);
            vector_sub(diff, point, ped_pos);
            wr::<f32>(diff, 8, 0.0);
            if moving {
                let tm = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x14);
                let fwd = tm.byte_add(0x10);
                let pt = |k: isize| rd::<f32>(point, 4 * k);
                let neg = X87::from_f32(pt(1))
                    .fmul_m(rd::<f32>(tm, 0x14))
                    .fadd(X87::from_f32(pt(2)).fmul_m(rd::<f32>(tm, 0x18)))
                    .fadd(X87::from_f32(pt(0)).fmul_m(rd::<f32>(tm, 0x10)))
                    .fchs()
                    .to_f32();
                let v = X87::from_f64_exact(vector_dot_product(ped_pos, fwd)).fadd_m(neg);
                if !v.fcom(X87::from_f32(kf(K_ZERO))).test_parity(5) {
                    let m2 = X87::from_f64_exact(vector_magnitude_sqr(diff)).to_f32();
                    if !ped_walk.is_null() {
                        if !X87::from_f32(m2).fcom(X87::from_f32(kf(K_TWO))).test_parity(5) {
                            let tmp = base.byte_add(8);
                            vector_scale(tmp, fwd, 2.0);
                            vector_add_assign(point, tmp);
                        }
                        let c = X87::from_f32(m2).fcom(X87::from_f32(kf(K_QUARTER_SQ)));
                        if !c.test_parity(5) {
                            // Closer than 0.25: the ped slows down, the target speeds up.
                            let s = max_f(SPEED_MIN, step(rd::<f32>(ped_walk, 0x24), false));
                            wr::<f32>(ped_walk, 0x24, X87::from_f64_exact(s).to_f32());
                            ped_flag = true;
                            if !target_walk.is_null() {
                                let s = min_f(SPEED_MAX, step(rd::<f32>(target_walk, 0x24), true));
                                wr::<f32>(target_walk, 0x24, X87::from_f64_exact(s).to_f32());
                                target_flag = true;
                            }
                        } else if !c.test_any(0x41) {
                            // Farther than 0.25: the ped speeds up, the target slows down.
                            let s = min_f(SPEED_MAX, step(rd::<f32>(ped_walk, 0x24), true));
                            wr::<f32>(ped_walk, 0x24, X87::from_f64_exact(s).to_f32());
                            ped_flag = true;
                            if !target_walk.is_null() {
                                let s = max_f(SPEED_MIN, step(rd::<f32>(target_walk, 0x24), false));
                                wr::<f32>(target_walk, 0x24, X87::from_f64_exact(s).to_f32());
                                target_flag = true;
                            }
                        } else {
                            settle_speed(ped, ped_walk);
                            ped_flag = true;
                            if !target_walk.is_null() {
                                settle_speed(rd::<GPtr<u8>>(this, 0xC), target_walk);
                                target_flag = true;
                            }
                        }
                    }
                }
            }
            go_to_point_update_point(go_to, point, kf(GO_TO_RADIUS), 0);
        }
        if counter_high
            && !X87::from_f32(dist2).fcom(X87::from_f32(kf(K_FOUR))).test_parity(5)
            && v_task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0
        {
            wr::<f32>(this, 0x14, dx);
            wr::<f32>(this, 0x18, dy);
            wr::<u32>(this, 0x1C, 0);
            return walk_alongside_create_sub_task(this, 0xCB, ped);
        }
    }
    set_move_anim_speed_flag(ped, ped_flag);
    set_move_anim_speed_flag(rd::<GPtr<u8>>(this, 0xC), target_flag);
    rd::<GPtr<u8>>(this, 8)
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_WALK_ALONGSIDE_CONTROL_SUB_TASK];
