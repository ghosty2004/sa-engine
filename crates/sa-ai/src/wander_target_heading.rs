//! WRITER-NATIVE (batch `native-30`): `CTaskComplexWander::ComputeTargetHeading` (0x66F530), seen once per long
//! walk (ambient wander). Ported from the 1.0 US bytes.

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

use crate::bindings::path_take_width_for_wandering;
use sa_core::bindings::general_get_radian_angle_between_points as general_radian_angle_between_points;
use sa_core::bindings::general_limit_radian_angle;

/// `float CTaskComplexWander::ComputeTargetHeading(CPed* ped)` (0x66F530, `ret 4`, ST0): the wander target
/// (`ThePaths.TakeWidthIntoAccountForWandering(next node +0x1C, ped seed +0x20)`) minus the ped position (f32 x / y),
/// `LimitRadianAngle(GetRadianAngleBetweenPoints(dx, dy, 0, 0))`. The original keeps the angle in the ped argument
/// slot.
#[hook(addr = 0x66F530, conv = thiscall, name = "CTaskComplexWander::ComputeTargetHeading",
       expect = "83 EC 18 8B 49 1C",
       unverified = "native-30", route = "L2", hazards = "x87-transcendental,stack-residue", recs = "grove-walk",
       note = "ST0 result")]
pub fn wander_compute_target_heading(this: GPtr<u8>, ped: GPtr<u8>) -> X87 {
    let mut out = [0f32; 3];
    let r = path_take_width_for_wandering(
        GPtr::from_addr(0x96_F050),
        GPtr::from_raw(out.as_mut_ptr()).cast(),
        rd::<u32>(this, 0x1C),
        u32::from(rd::<u16>(ped, 0x20)),
    );
    let (rx, ry) = (rd::<f32>(r, 0), rd::<f32>(r, 4));
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let p = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let dx = x(rx).fsub_m(rd::<f32>(p, 0)).to_f32();
    let dy = x(ry).fsub_m(rd::<f32>(p, 4)).to_f32();
    let a = X87::from_f64_exact(general_radian_angle_between_points(dx, dy, 0.0, 0.0).to_f64()).to_f32();
    X87::from_f64_exact(general_limit_radian_angle(a))
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_WANDER_COMPUTE_TARGET_HEADING];
