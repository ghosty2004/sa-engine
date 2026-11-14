//! `CTaskComplexFollowPedFootsteps::ControlSubTask` (INTRO-2: a ped follows CJ's footsteps after the intro's
//! drop-off). Ported from the 1.0 US bytes (the vtable-only body at 0x695090); gta-reversed as a reading aid.
//!
//! Task layout: +8 sub-task, +0xC the followed ped, +0x10 / +0x14 timers, +0x18 the `CPointRoute*` (+0 count, +4
//! eight 12-byte points), +0x1C the move state (i16), +0x1E flags (bit 0: line of sight to the followed ped).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPed::IsAlive()` (0x5E0170), AL
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, eAbortPriority, const CEvent*)` (vtable +0x18), AL
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: u32, event: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplexFollowPedFootsteps::CreateSubTask(int32, CPed*)` (vtable +0x2C)
    fn v_create_sub_task(t: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x2C => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490), AL
    #[allow(clippy::too_many_arguments)]
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, c1: u32, c2: u32, c3: u32, c4: u32, c5: u32, c6: u32, c7: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `operator-(const CVector&, const CVector&)` into `out` (0x40FE60, cdecl, returns `out`)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::Magnitude()` (0x406DA0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CPointRoute::AddPoints(const CVector&)` (0x48E180, `ret 4`)
    fn point_route_add(r: GPtr<u8>, p: GPtr<u8>) = thiscall 0x48E180 => unported;
    /// `CPointRoute` remove the point at an index (0x5F13A0, `ret 4`)
    fn point_route_remove(r: GPtr<u8>, i: i32) = thiscall 0x5F13A0 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn task_manager_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360, cdecl), AL
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => unported;
    /// `CTaskSimpleGoToPoint::UpdatePoint(const CVector&, float radius, bool)` (0x645700, `ret 0xC`)
    fn go_to_point_update_point(t: GPtr<u8>, p: GPtr<u8>, radius: u32, b: u32) = thiscall 0x645700 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn kf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// `K * K` against `v`: `fld K; fmul K; fcomp v; fnstsw`.
fn sq_cmp(k: f32, v: X87) -> sa_math::x87::Cmp {
    x(k).fmul_m(k).fcom(v)
}

/// `CTaskComplexFollowPedFootsteps::ControlSubTask(CPed*)` (0x695090, thiscall, `ret 4`).
///
/// No followed ped, no route or a dead followed ped → abort the sub-task (`MakeAbortable(ped, 1, null)`) and on
/// success `CreateSubTask(0x516)`. A sub-task of type 0x38B is kept. The followed ped in a vehicle → 0x38B; the ped's
/// intelligence +0x274 above [0x86C938] (30) → 0x1F4. Every 500 ms (+0x14) flag bit 0 = line of sight (ped →
/// followed, `true, false, false, true, false x3`). With `D = followed - ped`, `d3 = |D|` (f32) and `d2 = f32 sqrt(D.y²
/// + D.x²)`: `1.4² > d3` (the squared constant against the unsquared distance, as the bytes) and a sub-task other
/// than 0xCB → with line of sight route count 0 and 0xCB, without the sub-task kept. `8² < d3` → 0x38B; `d2 < 1` and
/// `|D.z| > 2` → 0x38B. With line of sight: route count 0, the followed position added, +0x10 = now - 332, and a
/// go-to simplest active task has its point updated (radius [0x86FC78]). Then at most every 166 ms (+0x10) while
/// the route holds fewer than 8 points: the followed position is added unless it is within 0.35 of the last point
/// (`0.35² >= |V|`), and when two or more points exist and the normalised last segment and V have a dot of at least
/// 0.95 the last point is dropped first. The route length is summed from point 0 to each later point (point 0 kept
/// as the start, as the bytes do) plus `|ped - followed|`; the move state +0x1C = 4 when `3.5²` is not below it, else
/// 6; a sub-task of type 0x384 takes it at +8. Otherwise, a 0xCB sub-task with more than one route point → 0x384.
/// The sub-task is returned unless a new one was made.
#[hook(addr = 0x695090, conv = thiscall, name = "CTaskComplexFollowPedFootsteps::ControlSubTask", expect = "83 EC 48 53 55 56",
       unverified = "intro3-15", route = "L2", hazards = "heap-writes", recs = "intro")]
pub fn follow_ped_footsteps_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    let switch_to = |ty: i32| -> GPtr<u8> {
        if v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0 {
            v_create_sub_task(this, ty, ped)
        } else {
            sub
        }
    };
    let followed = rd::<GPtr<u8>>(this, 0xC);
    if followed.is_null() || rd::<u32>(this, 0x18) == 0 || ped_is_alive(followed) == 0 {
        return switch_to(0x516);
    }
    if v_task_type(rd::<GPtr<u8>>(this, 8)) == 0x38B {
        return rd::<GPtr<u8>>(this, 8);
    }
    let followed = rd::<GPtr<u8>>(this, 0xC);
    if rd::<u32>(followed, 0x46C) & 0x100 != 0 {
        return switch_to(0x38B);
    }
    if rd::<i32>(rd::<GPtr<u8>>(ped, 0x47C), 0x274) > g::<i32>(0x86C938).read() {
        return switch_to(0x1F4);
    }
    let now = g::<u32>(TIME_MS).read();
    if now.wrapping_sub(rd::<u32>(this, 0x14)) > 0x1F4 {
        wr::<u32>(this, 0x14, now);
        let los = world_get_is_line_of_sight_clear(entity_pos(ped), entity_pos(followed), 1, 0, 0, 1, 0, 0, 0) & 1;
        wr::<u16>(this, 0x1E, (rd::<u16>(this, 0x1E) & 0xFFFE) | u16::from(los));
    }
    let tp = entity_pos(rd::<GPtr<u8>>(this, 0xC));
    let mut t = [rd::<u32>(tp, 0), rd::<u32>(tp, 4), rd::<u32>(tp, 8)];
    let tptr = GPtr::from_raw(t.as_mut_ptr()).cast::<u8>();
    let mut tmp = [0u32; 3];
    let r = vector_sub(GPtr::from_raw(tmp.as_mut_ptr()).cast(), tptr, entity_pos(ped));
    let mut d = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
    let d3 = X87::from_f64_exact(vector_magnitude(GPtr::from_raw(d.as_mut_ptr()).cast())).to_f32();
    let d2 = x(d[1]).fmul_m(d[1]).fadd(x(d[0]).fmul_m(d[0])).fsqrt().to_f32();
    let los = || rd::<u16>(this, 0x1E) & 1 != 0;
    let route = || rd::<GPtr<u8>>(this, 0x18);
    if !sq_cmp(kf(0x870C20), x(d3)).test_any(0x41) && v_task_type(rd::<GPtr<u8>>(this, 8)) != 0xCB {
        if !los() {
            return sub;
        }
        wr::<u32>(route(), 0, 0);
        return switch_to(0xCB);
    }
    if !sq_cmp(kf(0x870C38), x(d3)).test_parity(0x05) {
        return switch_to(0x38B);
    }
    if !x(d2).fcom(x(kf(0x858624))).test_parity(0x05) && !x(d[2]).fabs().fcom(x(kf(0x858CA0))).test_any(0x41) {
        return switch_to(0x38B);
    }
    if los() {
        wr::<u32>(route(), 0, 0);
        point_route_add(route(), entity_pos(rd::<GPtr<u8>>(this, 0xC)));
        wr::<u32>(this, 0x10, g::<u32>(TIME_MS).read().wrapping_sub(0x14C));
        let tm = rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
        if task_is_go_to_task(task_manager_get_simplest_active_task(tm)) != 0 {
            let fp = entity_pos(rd::<GPtr<u8>>(this, 0xC));
            let task = task_manager_get_simplest_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
            go_to_point_update_point(task, fp, g::<u32>(0x86FC78).read(), 0);
        }
    }
    let now = g::<u32>(TIME_MS).read();
    if now.wrapping_sub(rd::<u32>(this, 0x10)) < 0xA6 || rd::<i32>(route(), 0) >= 8 {
        if v_task_type(rd::<GPtr<u8>>(this, 8)) == 0xCB && rd::<i32>(route(), 0) > 1 {
            return switch_to(0x384);
        }
        return sub;
    }
    wr::<u32>(this, 0x10, now);
    let rt = route();
    let n = rd::<i32>(rt, 0);
    if n == 0 {
        point_route_add(rt, tptr);
    } else {
        let pt = |i: i32| rt.byte_add(4 + 12 * i as isize);
        let last = [rd::<f32>(pt(n - 1), 0), rd::<f32>(pt(n - 1), 4), rd::<f32>(pt(n - 1), 8)];
        let mut v = [0f32; 3];
        for (k, item) in v.iter_mut().enumerate() {
            *item = x(f32::from_bits(t[k])).fsub_m(last[k]).to_f32();
        }
        let vp = GPtr::from_raw(v.as_mut_ptr()).cast::<u8>();
        let m = X87::from_f64_exact(vector_magnitude(vp));
        if !sq_cmp(kf(0x870C24), m).test_parity(0x05) {
            if n >= 2 {
                let prev = [rd::<f32>(pt(n - 2), 0), rd::<f32>(pt(n - 2), 4), rd::<f32>(pt(n - 2), 8)];
                let mut w = [0f32; 3];
                for (k, item) in w.iter_mut().enumerate() {
                    *item = x(last[k]).fsub_m(prev[k]).to_f32();
                }
                let wp = GPtr::from_raw(w.as_mut_ptr()).cast::<u8>();
                vector_normalise(wp);
                vector_normalise(vp);
                let dot = x(w[1]).fmul_m(v[1]).fadd(x(w[0]).fmul_m(v[0])).fadd(x(w[2]).fmul_m(v[2]));
                if !dot.fcom(x(kf(0x858EF0))).test_any(0x01) {
                    point_route_remove(route(), rd::<i32>(route(), 0) - 1);
                }
            }
            point_route_add(route(), tptr);
        }
    }
    let rt = route();
    let p0 = [rd::<f32>(rt, 4), rd::<f32>(rt, 8), rd::<f32>(rt, 0xC)];
    let mut total = 0f32;
    let mut i = 1;
    while i < rd::<i32>(rt, 0) {
        let p = rt.byte_add(4 + 12 * i as isize);
        let mut v = [0f32; 3];
        for (k, item) in v.iter_mut().enumerate() {
            *item = x(rd::<f32>(p, 4 * k as isize)).fsub_m(p0[k]).to_f32();
        }
        total = X87::from_f64_exact(vector_magnitude(GPtr::from_raw(v.as_mut_ptr()).cast())).fadd_m(total).to_f32();
        i += 1;
    }
    let mut tmp = [0u32; 3];
    let r = vector_sub(GPtr::from_raw(tmp.as_mut_ptr()).cast(), entity_pos(ped), entity_pos(rd::<GPtr<u8>>(this, 0xC)));
    let len = X87::from_f64_exact(vector_magnitude(r)).fadd_m(total);
    let state: u16 = if sq_cmp(kf(0x870C2C), len).test_parity(0x05) { 4 } else { 6 };
    wr::<u16>(this, 0x1C, state);
    let s = rd::<GPtr<u8>>(this, 8);
    if v_task_type(s) == 0x384 {
        wr::<i32>(rd::<GPtr<u8>>(this, 8), 8, i32::from(state as i16));
    }
    sub
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FOLLOW_PED_FOOTSTEPS_CONTROL_SUB_TASK];
