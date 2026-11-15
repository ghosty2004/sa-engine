//! FRONTIER-2 turn 5, lane A (batch `frontier2-a8`): AI frontier entries ported from the 1.0 US bytes (gta-reversed
//! read as an aid; where it differs the bytes win, noted per function).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::bindings::{general_get_radian_angle_between_points, general_limit_radian_angle, general_random_int_in_range};
use sa_core::bindings::{task_new, vector_normalise};
use sa_core::mem::GPtr;
use sa_core::hook;
use sa_math::fpu::st0;

use crate::bindings::{membership_count_members, seek_entity_xy_offset_ctor};
use crate::common::*;
use sa_world::bindings::{
    achieve_heading_ctor, enter_car_as_passenger_ctor, leave_car_ctor, membership_get_leader, membership_get_member,
    pause_ctor, simple_car_drive_ctor, stand_still_ctor, vector_sub,
};
use sa_world::world::common::entity_pos;

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

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_FOLLOW_LEADER_IN_FORMATION_CREATE_SUB_TASK];
