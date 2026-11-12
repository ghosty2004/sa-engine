//! `CTaskComplexDriveToPoint::IsTargetBlocked` (INTRO-2: the intro's police car drives to a point). Both overloads,
//! ported from the 1.0 US bytes; gta-reversed `TaskComplexDriveToPoint.cpp` as a reading aid.
//!
//! Task layout: the target point is at +0x24/+0x28/+0x2C.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskComplexDriveToPoint::IsTargetBlocked(CPed*, CEntity** entities, int32 count)` (0x6432A0, `ret 0xC`)
    fn is_target_blocked_by_o(t: GPtr<u8>, ped: GPtr<u8>, list: GPtr<u8>, n: i32) -> u8 = thiscall 0x6432A0
        => crate::drive_to_point_target_blocked::is_target_blocked_by;
}

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;

/// The position of an entity: the matrix's (+0x14) translation (+0x30) when it has one, else the placement (+4).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// The bounding-sphere radius of an entity's model: `ms_modelInfoPtrs[+0x22]` → collision model (+0x14) → +0x24.
fn bound_radius(e: GPtr<u8>) -> f32 {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * (rd::<i16>(e, 0x22) as i32 as usize)).read();
    rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24)
}

/// `(dz² + dy²) + dx²` of `p - target` (`fld; fsub` ×3, the x87 order of 0x6432A0).
fn dist2_zyx(this: GPtr<u8>, p: GPtr<u8>) -> X87 {
    let dx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(this, 0x24));
    let dy = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(this, 0x28));
    let dz = x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(this, 0x2C));
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx))
}

/// `CTaskComplexDriveToPoint::IsTargetBlocked(CPed*, CEntity** entities, int32 count)` (0x6432A0, thiscall, `ret 0xC`).
///
/// False without a vehicle (+0x58C). R = the vehicle's bounding radius, D = `(dz² + dy²) + dx²` from the vehicle to
/// the target, stored as f32. For each non-null entity other than the vehicle, with r its radius (stored over the
/// ped argument's stack slot, no visible effect) and d its `(dz² + dy²) + dx²` to the target: when `r² > d` (strict,
/// `test ah, 0x41; jne` skips) and `(r + R)² * 1.5 > D` (strict), true. Else false.
#[hook(addr = 0x6432A0, conv = thiscall, name = "CTaskComplexDriveToPoint::IsTargetBlocked", this = "CTaskComplexDriveToPoint", preserve = "ecx",
       expect = "8B 44 24 04 83 EC 08 57",
       unverified = "intro3-2", route = "L2", hazards = "none", recs = "intro")]
pub fn is_target_blocked_by(this: GPtr<u8>, ped: GPtr<u8>, list: GPtr<u8>, n: i32) -> u8 {
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if veh.is_null() {
        return 0;
    }
    let own_r = bound_radius(veh);
    let veh_d2 = dist2_zyx(this, entity_pos(veh)).to_f32();
    let mut i = 0;
    while i < n {
        let e = rd::<GPtr<u8>>(list, 4 * i as isize);
        if !e.is_null() && e != veh {
            let d2 = dist2_zyx(this, entity_pos(e));
            let r = bound_radius(e);
            if !x(r).fmul_m(r).fcom(d2).test_any(0x41) {
                let s = x(r).fadd_m(own_r);
                let lhs = s.fmul(s).fmul(X87::from_f64_exact(f64::from_bits(0x3FF8_0000_0000_0000)));
                if !lhs.fcom(x(veh_d2)).test_any(0x41) {
                    return 1;
                }
            }
        }
        i += 1;
    }
    0
}

/// `CTaskComplexDriveToPoint::IsTargetBlocked(CPed*)` (0x6452C0, thiscall, `ret 4`).
///
/// False when the ped is farther than 6.0 from the target (`36 < (dz² + dx²) + dy²`, `test ah, 5; jp` falls
/// through). Else the list overload over the ped's intelligence (+0x47C) vehicle scanner entities (+0x130, 16), then
/// (when that is false) over its ped scanner entities (+0xE0, 16).
#[hook(addr = 0x6452C0, conv = thiscall, name = "CTaskComplexDriveToPoint::IsTargetBlocked", this = "CTaskComplexDriveToPoint", preserve = "ecx",
       expect = "56 8B 74 24 08 8B 46 14",
       unverified = "intro3-2", route = "L2", hazards = "none", recs = "intro")]
pub fn is_target_blocked(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let p = entity_pos(ped);
    let dx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(this, 0x24));
    let dy = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(this, 0x28));
    let dz = x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(this, 0x2C));
    let d2 = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    let lim = x(f32::from_bits(0x40C0_0000)).fmul_m(f32::from_bits(0x40C0_0000));
    if !lim.fcom(d2).test_parity(0x05) {
        return 0;
    }
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    if is_target_blocked_by_o(this, ped, intel.byte_add(0x130), 0x10) != 0 {
        return 1;
    }
    u8::from(is_target_blocked_by_o(this, ped, intel.byte_add(0xE0), 0x10) != 0)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_IS_TARGET_BLOCKED_BY, HOOK_IS_TARGET_BLOCKED];
