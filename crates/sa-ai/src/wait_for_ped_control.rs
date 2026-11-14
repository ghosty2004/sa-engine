//! `CTaskComplexWaitForPed::ControlSubTask` (0x6834E0, batch `newgame-1`): the wait a partner task gives its ped
//! until the partner comes close. Ported from the 1.0 US bytes (gta-reversed as a reading aid). Unverified
//! (docs/verify/HOWTO.md).
//!
//! `CTaskComplexWaitForPed`: +8 sub task, +0xC the ped waited for, +0x10 the distance (f32), +0x14 the time, +0x18
//! face the ped, +0x1C task timer (+0x24 its "set" byte), +0x28 the facing pass counter (not set by the constructor).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

use crate::bindings::event_dead_ped_ctor;
use crate::bindings::event_dead_ped_dtor;
use crate::bindings::task_timer_is_out_of_time;
use sa_core::bindings::general_get_radian_angle_between_points as radian_angle_between_points;
use sa_core::bindings::general_limit_radian_angle as limit_radian_angle;
use sa_core::bindings::matrix_mul_vec;
use sa_core::bindings::ped_is_alive;
use sa_world::bindings::event_group_add;
use sa_world::bindings::intel_find_task_by_type;
use sa_world::bindings::vector_sub;

use sa_world::world::common::entity_pos as pos;

/// `CTaskComplexWaitForPed::ControlSubTask(CPed* ped)` (0x6834E0, thiscall, `ret 4`):
/// - no ped to wait for → null;
/// - that ped dead → a `CEventDeadPed(target, false, CTimer::m_snTimeInMilliseconds)` (word +0xE = 600) added to
///   `ped`'s event group (intelligence +0x68), null;
/// - the timer set (+0x24) and out of time → null;
/// - the difference: with a 0x4B8 task in `ped`'s intelligence, `ped matrix * task offset (+0x14)` minus the target
///   position, else `ped` position minus the target position; `|d|^2 < f*f` (x87, `test ah, 5` / `jp`) → null;
/// - else, facing (+0x18) on the counter's first pass: `ped+0x55C = LimitRadianAngle(GetRadianAngleBetweenPoints(
///   (target - ped).xy, 0, 0))`; the counter (+0x28) increments and wraps to 0 above 10; the sub task (+8).
#[hook(addr = 0x6834E0, conv = thiscall, name = "vt_8706f8::ControlSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "newgame-1", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "pause-restart",
       note = "reached after New Game from the pause menu (--start-in-world run, frame 484); the dead-ped event is a stack local")]
pub fn wait_for_ped_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let target = rd::<GPtr<u8>>(this, 0xC);
    if target.is_null() {
        return GPtr::NULL;
    }
    if ped_is_alive(target) == 0 {
        let mut ev = [0u32; 16];
        let e: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
        let now = g::<u32>(0xB7CB84).read();
        event_dead_ped_ctor(e, rd::<GPtr<u8>>(this, 0xC), 0, now);
        let intel = rd::<GPtr<u8>>(ped, 0x47C);
        wr::<u16>(e, 0xE, 600);
        event_group_add(intel.byte_add(0x68), e, 0);
        event_dead_ped_dtor(e);
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0x24) != 0 && task_timer_is_out_of_time(this.byte_add(0x1C)) != 0 {
        return GPtr::NULL;
    }
    let task = intel_find_task_by_type(rd::<GPtr<u8>>(ped, 0x47C), 0x4B8);
    let (dx, dy, dz) = if !task.is_null() {
        let mut v = [rd::<u32>(task, 0x14), rd::<u32>(task, 0x18), rd::<u32>(task, 0x1C)];
        let mut out = [0u32; 3];
        let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
        let op: GPtr<u8> = GPtr::from_raw(out.as_mut_ptr()).cast();
        matrix_mul_vec(op, rd::<GPtr<u8>>(ped, 0x14), vp);
        let t = pos(rd::<GPtr<u8>>(this, 0xC));
        (
            x(f32::from_bits(out[0])).fsub_m(rd::<f32>(t, 0)),
            x(f32::from_bits(out[1])).fsub_m(rd::<f32>(t, 4)),
            x(f32::from_bits(out[2])).fsub_m(rd::<f32>(t, 8)),
        )
    } else {
        let t = pos(rd::<GPtr<u8>>(this, 0xC));
        let p = pos(ped);
        (
            x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(t, 0)),
            x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(t, 4)),
            x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(t, 8)),
        )
    };
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    let f = x(rd::<f32>(this, 0x10));
    // `fcompp` of |d|^2 against f*f; `test ah, 5` / `jp`: on to the facing when not below (or unordered)
    if !d2.fcom(f.fmul(f)).test_parity(5) {
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0x18) != 0 && rd::<u32>(this, 0x28) == 0 {
        let p = pos(ped);
        let t = pos(rd::<GPtr<u8>>(this, 0xC));
        let mut out = [0u32; 3];
        let op: GPtr<u8> = GPtr::from_raw(out.as_mut_ptr()).cast();
        vector_sub(op, t, p);
        let a = f32_of(X87::from_f64_exact(radian_angle_between_points(out[0], out[1], 0, 0)));
        let h = f32_of(X87::from_f64_exact(limit_radian_angle(a.to_bits())));
        wr::<f32>(ped, 0x55C, h);
    }
    let n = rd::<i32>(this, 0x28).wrapping_add(1);
    wr::<i32>(this, 0x28, n);
    if n > 10 {
        wr::<u32>(this, 0x28, 0);
    }
    rd::<GPtr<u8>>(this, 8)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_WAIT_FOR_PED_CONTROL_SUB_TASK];
