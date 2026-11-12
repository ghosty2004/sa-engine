//! WRITER-NATIVE (batch `native-30`): `CTaskComplexWander::ComputeTargetHeading` (0x66F530), seen once per long
//! walk (ambient wander). Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPathFind::TakeWidthIntoAccountForWandering(CVector* out, CNodeAddress, uint16 seed)` (0x4509A0) on
    /// `ThePaths` (0x96F050)
    fn path_take_width_for_wandering(p: GPtr<u8>, out: GPtr<u8>, node: u32, seed: u32) -> GPtr<u8> = thiscall 0x4509A0 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(x1, y1, x2, y2)` (0x53CBE0, ST0)
    fn general_radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn general_limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
}

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
    let a = X87::from_f64_exact(general_radian_angle_between_points(dx, dy, 0.0, 0.0)).to_f32();
    X87::from_f64_exact(general_limit_radian_angle(a))
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_WANDER_COMPUTE_TARGET_HEADING];
