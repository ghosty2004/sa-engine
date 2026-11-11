//! ANIM: `CPedIK` (PORT-ANIM-INPUT, fork D). Ported from the 1.0 US bytes; gta-reversed `PedIK.cpp`
//! as a reading aid. See `docs/arch/anim-input-port.md`.
//!
//! `CPedIK` (0x20): +0x00 `m_pPed`, +0x04 torso orientation (yaw, pitch), +0x0C upper arm, +0x14
//! lower arm, +0x1C flags. `LimbOrientation` = {yaw, pitch}; `LimbMovementInfo` (0x18) = {max yaw,
//! min yaw, yaw step, max pitch, min pitch, pitch step}.
//!
//! Deferred (see the port doc): `PointGunInDirection`, `PointGunAtPosition` (ped bone/task glue with
//! an `RwMatrixCreate`/`Destroy` temporary and argument-slot stores) and `PitchForSlope` (2 KB, SEH,
//! ped/world collision tests).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::{Cmp, X87};

use crate::common::{g, rd, wr, x};
use crate::ik::rt_quat_rotate;

sa_core::globals! {
    list GLOBALS;
    /// `bRotateWithNeck` (1 in .data)
    pub static ROTATE_WITH_NECK @ 0x8D2354: u8 = 1;
}

/// `XaxisIK` (0x8D232C) and `ZaxisIK` (0x8D2344): the rotation axes (`CVector` in .data).
const X_AXIS: usize = 0x8D232C;
const Z_AXIS: usize = 0x8D2344;
const RAD2DEG: f32 = f32::from_bits(0x4265_2EE0); // 0x859878
const PI: f32 = f32::from_bits(0x4049_0FDB); // 0x858CB8
const TWO_PI: f32 = f32::from_bits(0x40C9_0FDB); // 0x858CBC
const ARM_MAX: f32 = f32::from_bits(0x3F49_0FDB); // 0x86C860 / 0x86C864 (pi/4)
const ARM_MIN: f32 = f32::from_bits(0xBF86_0A92); // 0x86C868 (-pi/3)
const ARM_MIN_CLAMP: f32 = f32::from_bits(0xBEB2_B8C3); // 0x86C86C
const RATIO_LIMIT: f32 = f32::from_bits(0x3F66_6666); // 0x858C20 (0.9)

orig! {
    /// `RwMatrixTransform(matrix, transform, combineOp)` (0x7F25A0)
    pub fn rw_matrix_transform(m: GPtr<u8>, t: GPtr<u8>, op: i32) -> GPtr<u8> = cdecl 0x7F25A0 => unported;
}

/// `CPedIK::CPedIK(CPed* ped)` (0x5FD8C0): `m_pPed = ped`, everything else 0; returns this.
#[hook(addr = 0x5FD8C0, conv = thiscall, name = "CPedIK::Constructor", expect = "8B C1 8B 4C 24 04", this = "CPedIK")]
pub fn constructor(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, ped.addr() as u32);
    wr::<u32>(this, 0x1C, 0);
    for o in (4..0x1C).step_by(4) {
        wr::<u32>(this, o, 0);
    }
    this
}

/// `CPedIK::GetWorldMatrix(RwFrame* frame, RwMatrix* out)` (0x5FD8F0, cdecl): the frame's modelling
/// matrix, post-concatenated with every parent's (`RwMatrixTransform(out, &parent->modelling, 2)`);
/// returns `out`.
#[hook(addr = 0x5FD8F0, conv = cdecl, name = "CPedIK::GetWorldMatrix", expect = "8B 44 24 04 53")]
pub fn get_world_matrix(frame: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> {
    for k in 0..16 {
        wr::<u32>(out, 4 * k, rd::<u32>(frame, 0x10 + 4 * k));
    }
    let mut p = rd::<GPtr<u8>>(frame, 4);
    while !p.is_null() {
        rw_matrix_transform(out, p.byte_add(0x10), 2);
        p = rd::<GPtr<u8>>(p, 4);
    }
    out
}

fn lt(a: X87, b: f32) -> bool {
    a.fcom(x(b)) == Cmp::Less
}
fn gt(a: X87, b: f32) -> bool {
    a.fcom(x(b)) == Cmp::Greater
}
fn fabs(a: X87) -> X87 {
    X87::from_f64_exact(a.to_f64().abs())
}

/// `CPedIK::MoveLimb(LimbOrientation& limb, float yaw, float pitch, LimbMovementInfo& info)`
/// (0x5FDA60, stdcall): step yaw and pitch toward the target by the steps (a target closer than a
/// step is taken directly), clamp to max then min. Status: 2 when the yaw target was taken
/// directly, 0 when the yaw was clamped, else 1; a pitch step sets 1, a pitch taken directly keeps
/// the status, a pitch clamp returns 0 at once.
#[hook(addr = 0x5FDA60, conv = stdcall, name = "CPedIK::MoveLimb-non-normalized", expect = "8B 4C 24 04 D9 01")]
pub fn move_limb(limb: GPtr<u32>, yaw: f32, pitch: f32, info: GPtr<f32>) -> i32 {
    let lf = |k: i32| f32::from_bits(limb.wrapping_index(k).read());
    let inf = |k: i32| info.wrapping_index(k).read();
    let mut st = 1;
    // yaw
    if lt(fabs(x(lf(0)).fsub_m(yaw)), inf(2)) {
        limb.write(yaw.to_bits());
        st = 2;
    } else if lt(x(lf(0)), yaw) {
        limb.write(x(lf(0)).fadd_m(inf(2)).to_f32().to_bits());
    } else if gt(x(lf(0)), yaw) {
        limb.write(x(lf(0)).fsub_m(inf(2)).to_f32().to_bits());
    }
    if gt(x(lf(0)), inf(0)) {
        limb.write(info.cast::<u32>().read());
        st = 0;
    } else if lt(x(lf(0)), inf(1)) {
        limb.write(info.cast::<u32>().wrapping_index(1).read());
        st = 0;
    }
    // pitch
    let lp = limb.wrapping_index(1);
    if lt(fabs(x(lf(1)).fsub_m(pitch)), inf(5)) {
        lp.write(pitch.to_bits());
    } else if lt(x(lf(1)), pitch) {
        lp.write(x(lf(1)).fadd_m(inf(5)).to_f32().to_bits());
        st = 1;
    } else if gt(x(lf(1)), pitch) {
        lp.write(x(lf(1)).fsub_m(inf(5)).to_f32().to_bits());
        st = 1;
    }
    if gt(x(lf(1)), inf(3)) {
        lp.write(info.cast::<u32>().wrapping_index(3).read());
        return 0;
    }
    if lt(x(lf(1)), inf(4)) {
        lp.write(info.cast::<u32>().wrapping_index(4).read());
        return 0;
    }
    st
}

/// `CPedIK::MoveLimb(LimbOrientation& limb, float yaw, float pitch, LimbMovementInfo& info, float
/// ratio)` (0x5FDB60, stdcall): `limb = (yaw, pitch) * ratio`, the yaw compared unrounded with max
/// then min, the pitch as stored; any clamp returns 0 (the pitch clamps return at once); otherwise
/// 2 when the yaw was not clamped and `ratio > 0.9`, else 1 / 0. The original also stores the
/// scaled pitch over its own `yaw` argument slot (stack only, not reproduced).
#[hook(addr = 0x5FDB60, conv = stdcall, name = "CPedIK::MoveLimb-normalized", expect = "D9 44 24 08 8B 54 24 04")]
pub fn move_limb_normalized(limb: GPtr<u32>, yaw: f32, pitch: f32, info: GPtr<u32>, ratio: f32) -> i32 {
    let mut st = 1;
    let y = x(yaw).fmul_m(ratio);
    limb.write(y.to_f32().to_bits());
    let pf = x(pitch).fmul_m(ratio).to_f32();
    limb.wrapping_index(1).write(pf.to_bits());
    let inf = |k: i32| f32::from_bits(info.wrapping_index(k).read());
    if gt(y, inf(0)) {
        limb.write(info.read());
        st = 0;
    } else if lt(y, inf(1)) {
        limb.write(info.wrapping_index(1).read());
        st = 0;
    }
    if gt(x(pf), inf(3)) {
        limb.wrapping_index(1).write(info.wrapping_index(3).read());
        return 0;
    }
    if lt(x(pf), inf(4)) {
        limb.wrapping_index(1).write(info.wrapping_index(4).read());
        return 0;
    }
    if st != 0 && gt(x(ratio), RATIO_LIMIT) {
        return 2;
    }
    st
}

/// `CPedIK::RotateTorso(AnimBlendFrameData* bone, LimbOrientation& orient, bool changeYaw)`
/// (0x5FDDB0): rotate the bone's keyframe quaternion about the X axis by `yaw` degrees (combine
/// op 1 when `changeYaw`, else 0 = replace), then about Z by `pitch` (op 1); set ped flag 0x4000
/// at +0x470.
#[hook(addr = 0x5FDDB0, conv = thiscall, name = "CPedIK::RotateTorso", expect = "8B 44 24 04 53", this = "CPedIK")]
pub fn rotate_torso(this: GPtr<u8>, bone: GPtr<u8>, orient: GPtr<f32>, change_yaw: u8) {
    let q = rd::<GPtr<f32>>(bone, 0x10);
    let a = x(orient.read()).fmul_m(RAD2DEG).to_f32();
    rt_quat_rotate(q, g(X_AXIS), a, i32::from(change_yaw != 0));
    let b = x(orient.wrapping_index(1).read()).fmul_m(RAD2DEG).to_f32();
    rt_quat_rotate(q, g(Z_AXIS), b, 1);
    let ped = rd::<GPtr<u8>>(this, 0);
    wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) | 0x4000);
}

/// `CPedIK::RotateTorsoForArm(const CVector& target)` (0x5FDF90): the heading difference to the
/// target (`atan2(-(dx), dy)` wrapped into heading ± pi, minus the heading), reduced above pi/4
/// (clamped to pi/4) or below -pi/3 (clamped to -0.349), nothing when it ends exactly 0 or lies in
/// between; rotate the neck bone (+0x4CC) by half of it when `bRotateWithNeck` (the half goes
/// through an f32 argument slot and is reloaded), then the torso bone (+0x48C) by it (op 2).
#[hook(addr = 0x5FDF90, conv = thiscall, name = "CPedIK::RotateTorsoForArm", expect = "56 57 8B F9 8B 0F", this = "CPedIK")]
pub fn rotate_torso_for_arm(this: GPtr<u8>, target: GPtr<f32>) {
    let ped = rd::<GPtr<u8>>(this, 0);
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pos: GPtr<f32> = if m.is_null() { ped.byte_add(4).cast() } else { m.byte_add(0x30).cast() };
    let a = x(target.read()).fsub_m(pos.read()).fchs();
    let b = x(target.wrapping_index(1).read()).fsub_m(pos.wrapping_index(1).read());
    let mut ang = a.fpatan(b);
    let heading = rd::<f32>(ped, 0x558);
    if ang.fcom(x(heading).fadd_m(PI)) == Cmp::Greater {
        ang = ang.fsub_m(TWO_PI);
    } else if ang.fcom(x(heading).fsub_m(PI)) == Cmp::Less {
        ang = ang.fadd_m(TWO_PI);
    }
    let mut d = ang.fsub_m(heading);
    if d.fcom(x(ARM_MAX)) == Cmp::Greater {
        d = d.fsub_m(ARM_MAX);
        if d.fcom(x(ARM_MAX)) == Cmp::Greater {
            d = x(ARM_MAX);
        }
    } else if d.fcom(x(ARM_MIN)) == Cmp::Less {
        d = d.fsub_m(ARM_MIN);
        if d.fcom(x(ARM_MIN_CLAMP)) == Cmp::Less {
            d = x(ARM_MIN_CLAMP);
        }
    } else {
        return;
    }
    if d.fcom(x(0.0)) == Cmp::Equal {
        return;
    }
    if ROTATE_WITH_NECK.get() != 0 {
        let half = d.fmul_m(0.5).to_f32();
        let q = rd::<GPtr<f32>>(rd::<GPtr<u8>>(ped, 0x4CC), 0x10);
        rt_quat_rotate(q, g(X_AXIS), x(half).fmul_m(RAD2DEG).to_f32(), 2);
        d = x(half);
    }
    let q = rd::<GPtr<f32>>(rd::<GPtr<u8>>(ped, 0x48C), 0x10);
    rt_quat_rotate(q, g(X_AXIS), d.fmul_m(RAD2DEG).to_f32(), 2);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_CONSTRUCTOR,
    HOOK_GET_WORLD_MATRIX,
    HOOK_MOVE_LIMB,
    HOOK_MOVE_LIMB_NORMALIZED,
    HOOK_ROTATE_TORSO,
    HOOK_ROTATE_TORSO_FOR_ARM,
];
