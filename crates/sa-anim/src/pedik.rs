//! ANIM: `CPedIK` (PORT-ANIM-INPUT, fork D). Ported from the 1.0 US bytes; gta-reversed `PedIK.cpp`
//! as a reading aid. See `docs/arch/anim-input-port.md`.
//!
//! `CPedIK` (0x20): +0x00 `m_pPed`, +0x04 torso orientation (yaw, pitch), +0x0C upper arm, +0x14
//! lower arm, +0x1C flags. `LimbOrientation` = {yaw, pitch}; `LimbMovementInfo` (0x18) = {max yaw,
//! min yaw, yaw step, max pitch, min pitch, pitch step}.
//!
//! Deferred (see the port doc): `PointGunInDirection`, `PointGunAtPosition` (ped bone/task glue with
//! an `RwMatrixCreate`/`Destroy` temporary and argument-slot stores) and `PitchForSlope` (2 KB, SEH,
//! ped/world collision tests); `PitchForSlope` is now ported (batch `mvpk-anim-1`).

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

orig! {
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    fn get_anim_hierarchy_from_skin_clump(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32 id)` (0x7C51A0)
    fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120)
    fn rp_hanim_hierarchy_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `CMatrix::CMatrix(RwMatrix*, bool ownsAttached)` (0x59C050, `ret 8`)
    fn matrix_ctor_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59C050 => unported;
    /// `CMatrix::SetTranslate(float, float, float)` (0x59AF40, `ret 0xC`)
    fn matrix_set_translate(m: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x59AF40 => unported;
    /// `CMatrix::RotateX(float)` (0x59B1E0, `ret 4`)
    fn matrix_rotate_x(m: GPtr<u8>, a: f32) = thiscall 0x59B1E0 => unported;
    /// `CMatrix::RotateY(float)` (0x59B2C0, `ret 4`)
    fn matrix_rotate_y(m: GPtr<u8>, a: f32) = thiscall 0x59B2C0 => unported;
    /// `CMatrix::operator*=(const CMatrix&)` (0x411A80, `ret 4`)
    fn matrix_mul_assign(m: GPtr<u8>, r: GPtr<u8>) = thiscall 0x411A80 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn general_limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
}

/// The anim blend clump data plugin offset (0xB5F878)
const CLUMP_DATA_OFFSET: usize = 0xB5F878;
/// The "use the absolute bone slope" switch (0xC0AFC0, byte)
const SLOPE_ABS_SWITCH: usize = 0xC0AFC0;

fn kf(a: usize) -> f32 {
    g::<f32>(a).read()
}
/// Outside the dead zone (-0.01, 0.01): above 0.01 (`test ah, 0x41`), else below -0.01 (`test ah, 5`).
fn outside_dead_zone(v: f32) -> bool {
    !x(v).fcom(x(kf(0x858C58))).test_any(0x41) || !x(v).fcom(x(kf(0x86C310))).test_parity(5)
}
/// Clamps `this + off` to ±`limit` (f32 compares; the negative bound stored from the register negation).
fn clamp_sym(this: GPtr<u8>, off: isize, limit: f32) {
    if !x(rd::<f32>(this, off)).fcom(x(limit)).test_any(0x41) {
        wr::<f32>(this, off, limit);
    }
    let n = x(limit).fchs();
    if !n.fcom(x(rd::<f32>(this, off))).test_any(0x41) {
        wr::<f32>(this, off, n.to_f32());
    }
}
/// The keyframe quaternion of bone `id` (`clumpData->frames[RpHAnimIDGetIndex(h, id)].keyFrame`, 0x18 per frame).
fn bone_quat(clump_data: GPtr<u8>, h: GPtr<u8>, id: i32) -> GPtr<f32> {
    let idx = rp_hanim_id_get_index(h, id);
    rd::<GPtr<f32>>(rd::<GPtr<u8>>(clump_data, 0x10), idx as isize * 0x18 + 0x10)
}
/// The ped's RW frame modelling matrix (clump +4 → frame +0x10), or NULL without an RW object.
fn ped_rw_matrix(ped: GPtr<u8>) -> GPtr<u8> {
    let rw = rd::<GPtr<u8>>(ped, 0x18);
    if rw.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(rw, 4).byte_add(0x10) }
}

/// One leg of `PitchForSlope`: `s = f32 acos(up.z)` of bone `hip`'s matrix (`_CIacos`), then three `RtQuatRotate(.., Z
/// axis, angle, 2)` on `hip`, `knee`, `foot`: `f32(the 0xC0AFC0 switch ? |s| * pitch * 30 : 30 * pitch * s)`, `f32(max(-(s *
/// pitch), 0) * -30)` and `f32(max(s, 0) * pitch * -30)` (the maxima keep NaN).
fn pitch_leg(cd: GPtr<u8>, h: GPtr<u8>, pitch: f32, hip: i32, knee: i32, foot: i32) {
    let idx = rp_hanim_id_get_index(h, hip);
    let m = rp_hanim_hierarchy_get_matrix_array(h).byte_add(idx as isize * 0x40);
    let s = X87::from_f64_exact(sa_core::orig::x87_unary(0x821E70, f64::from(rd::<f32>(m, 0x18)))).to_f32();
    let a1 = if g::<u8>(SLOPE_ABS_SWITCH).read() != 0 {
        x(s).fabs().fmul_m(pitch).fmul_m(kf(0x86C8A0))
    } else {
        x(kf(0x86C8A0)).fmul_m(pitch).fmul_m(s)
    }
    .to_f32();
    let z = GPtr::<f32>::from_addr(Z_AXIS);
    rt_quat_rotate(bone_quat(cd, h, hip), z, a1, 2);
    let t = x(s).fmul_m(pitch).fchs();
    let zero = x(kf(0x858B50));
    let t = if !zero.fcom(t).test_any(0x41) { zero } else { t };
    let a2 = t.fmul_m(kf(0x86C89C)).to_f32();
    rt_quat_rotate(bone_quat(cd, h, knee), z, a2, 2);
    let u = if !zero.fcom(x(s)).test_any(0x41) { zero } else { x(s) };
    let a3 = u.fmul_m(pitch).fmul_m(kf(0x86C8A4)).to_f32();
    rt_quat_rotate(bone_quat(cd, h, foot), z, a3, 2);
}

/// One bone of the roll part: `r = LimitRadianAngle(f32(fpatan(at.y, at.x) - ped heading +0x558))` of the bone matrix,
/// `RtQuatRotate(quat, &(0, f32 cos r, f32 sin r), f32(roll * rad2deg), 1)` (register r).
fn roll_bone(cd: GPtr<u8>, h: GPtr<u8>, ped: GPtr<u8>, id: i32, roll: f32) {
    let idx = rp_hanim_id_get_index(h, id);
    let q = rd::<GPtr<f32>>(rd::<GPtr<u8>>(cd, 0x10), idx as isize * 0x18 + 0x10);
    let m = rp_hanim_hierarchy_get_matrix_array(h).byte_add(idx as isize * 0x40);
    let a = x(rd::<f32>(m, 0x24)).fpatan(x(rd::<f32>(m, 0x20))).fsub_m(rd::<f32>(ped, 0x558)).to_f32();
    let r = X87::from_f64_exact(general_limit_radian_angle(a));
    let mut axis = [0f32, r.fcos().to_f32(), r.fsin().to_f32()];
    rt_quat_rotate(q, GPtr::from_raw(axis.as_mut_ptr()), x(roll).fmul_m(RAD2DEG).to_f32(), 1);
}

/// `CPedIK::PitchForSlope()` (0x5FE0E0, thiscall). x87 (FSIN, FCOS, FPATAN, `_CIacos`). `CPedIK` +0xC body pitch, +0x10
/// smoothed slope limit, +0x14 roll, +0x18 third angle, +0x1C flags. `dead` = the ped's state +0x530 is 0x36/0x37 or
/// +0x474 bit 20.
///
/// - Pitch outside the dead zone ±0.01 and not dying: `f` = 0.85 (move state 4) or 1.0; +0x10 = f32(0.1 f + 0.9
///   +0x10); the pitch is raised to `+0x10 * 0.5236` when below it, else lowered to `+0x10 * -0.6109` when above it.
///   Roll outside the dead zone and not dying: clamped to ±π/4; +0x18 outside it: clamped to ±π/6.
/// - Dying: a stack `CMatrix` on the ped's RW frame matrix times a translation (0, 0, 1) rotated by `-pitch` (X) and
///   the roll (Y), its z - 1, `UpdateRW`, `UpdateRwFrame`.
/// - Else the pitch inside the dead zone → 0; outside: bone 2 rotates by `pitch * rad2deg` when the active task is
///   0x403, otherwise by `-pitch * rad2deg * 0.75` plus both legs (bones 0x33/0x34/0x35 and 0x29/0x2A/0x2B, see
///   `pitch_leg`) and the frame moves: a stack `CMatrix` on the RW frame matrix times `RotateX(-(0.5 pitch))`, its
///   position += `sin(pitch) * 0.75 * up` (x, y) and z += that + `0.3 / cos(roll) - 0.3`; `UpdateRW`, `UpdateRwFrame`.
///   Ped +0x470 |= 0x4000. Then the roll inside the dead zone → 0; outside: bones 0x2B and 0x35 roll (`roll_bone`) and
///   +0x470 |= 0x4000.
/// - +0x1C bit 3 cleared.
#[hook(addr = 0x5fe0e0, conv = thiscall, name = "CPedIK::PitchForSlope", expect = "6A FF 64 A1 00 00 00 00",
       this = "CPedIK", unverified = "mvpk-anim-1", route = "L2", hazards = "x87-transcendental,heap-writes", recs = "grove-walk",
       note = "two CMatrix temporaries live on the Rust stack; rotates bone keyframe quaternions and the RW frame (RW heap)")]
pub fn ped_ik_pitch_for_slope(this: GPtr<u8>) {
    let ped = rd::<GPtr<u8>>(this, 0);
    let clump = rd::<GPtr<u8>>(ped, 0x18);
    let cd = rd::<GPtr<u8>>(clump, g::<u32>(CLUMP_DATA_OFFSET).read() as isize);
    let h = get_anim_hierarchy_from_skin_clump(clump);
    let st = rd::<u32>(ped, 0x530);
    let dying = st == 0x36 || st == 0x37 || rd::<u32>(ped, 0x474) & 0x10_0000 != 0;
    if outside_dead_zone(rd::<f32>(this, 0xC)) && st != 0x36 && st != 0x37 {
        let f = if rd::<u32>(ped, 0x534) == 4 { x(kf(0x86C87C)) } else { x(kf(0x86C878)) };
        let k = kf(0x86C880);
        let v = x(kf(0x858624)).fsub_m(k).fmul(f).fadd(x(k).fmul_m(rd::<f32>(this, 0x10))).to_f32();
        wr::<f32>(this, 0x10, v);
        let hi = x(v).fmul_m(kf(0x86C870));
        if !hi.fcom(x(rd::<f32>(this, 0xC))).test_parity(5) {
            wr::<f32>(this, 0xC, hi.to_f32());
        } else {
            let lo = x(v).fmul_m(kf(0x86C874));
            if !lo.fcom(x(rd::<f32>(this, 0xC))).test_any(0x41) {
                wr::<f32>(this, 0xC, lo.to_f32());
            }
        }
    }
    let st = rd::<u32>(ped, 0x530);
    if outside_dead_zone(rd::<f32>(this, 0x14)) && st != 0x36 && st != 0x37 {
        clamp_sym(this, 0x14, kf(0x86C884));
    }
    if outside_dead_zone(rd::<f32>(this, 0x18)) {
        clamp_sym(this, 0x18, kf(0x86C888));
    }
    if dying {
        let mut a = [0u32; 0x12];
        let a_p: GPtr<u8> = GPtr::from_raw(a.as_mut_ptr()).cast();
        matrix_ctor_attach(a_p, ped_rw_matrix(ped), 0);
        let mut b = [0u32; 0x12];
        let b_p: GPtr<u8> = GPtr::from_raw(b.as_mut_ptr()).cast();
        matrix_set_translate(b_p, 0, 0, 0x3F80_0000);
        matrix_rotate_x(b_p, x(rd::<f32>(this, 0xC)).fchs().to_f32());
        matrix_rotate_y(b_p, rd::<f32>(this, 0x14));
        matrix_mul_assign(a_p, b_p);
        wr::<f32>(a_p, 0x38, x(rd::<f32>(a_p, 0x38)).fsub_m(kf(0x858624)).to_f32());
        matrix_update_rw(a_p);
        entity_update_rw_frame(rd::<GPtr<u8>>(this, 0));
        matrix_dtor(b_p);
        matrix_dtor(a_p);
    } else {
        if !outside_dead_zone(rd::<f32>(this, 0xC)) {
            wr::<u32>(this, 0xC, 0);
        } else {
            let q = bone_quat(cd, h, 2);
            let z = GPtr::<f32>::from_addr(Z_AXIS);
            let tm = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C).byte_add(4);
            if !tm_get_active_task(tm).is_null() && v_task_type(tm_get_active_task(tm)) == 0x403 {
                rt_quat_rotate(q, z, x(rd::<f32>(this, 0xC)).fmul_m(RAD2DEG).to_f32(), 1);
                let p = rd::<GPtr<u8>>(this, 0);
                wr::<u32>(p, 0x470, rd::<u32>(p, 0x470) | 0x4000);
            } else {
                let a = x(rd::<f32>(this, 0xC)).fchs().fmul_m(RAD2DEG).fmul_m(kf(0x86C898)).to_f32();
                rt_quat_rotate(q, z, a, 1);
                let p = rd::<GPtr<u8>>(this, 0);
                wr::<u32>(p, 0x470, rd::<u32>(p, 0x470) | 0x4000);
                let pitch = rd::<f32>(this, 0xC);
                pitch_leg(cd, h, pitch, 0x33, 0x34, 0x35);
                pitch_leg(cd, h, rd::<f32>(this, 0xC), 0x29, 0x2A, 0x2B);
                let mut c = [0u32; 0x12];
                let c_p: GPtr<u8> = GPtr::from_raw(c.as_mut_ptr()).cast();
                matrix_ctor_attach(c_p, ped_rw_matrix(rd::<GPtr<u8>>(this, 0)), 0);
                let mut d = [0u32; 0x12];
                let d_p: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
                matrix_set_translate(d_p, 0, 0, 0);
                matrix_rotate_x(d_p, x(kf(0x86C894)).fmul_m(rd::<f32>(this, 0xC)).fchs().to_f32());
                matrix_mul_assign(c_p, d_p);
                let s = x(rd::<f32>(this, 0xC)).fsin().fmul_m(kf(0x86C88C));
                let m = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x14);
                let (sz, sy, sx) = (s.fmul_m(rd::<f32>(m, 0x18)), s.fmul_m(rd::<f32>(m, 0x14)), s.fmul_m(rd::<f32>(m, 0x10)));
                wr::<f32>(c_p, 0x30, x(rd::<f32>(c_p, 0x30)).fadd(sx).to_f32());
                wr::<f32>(c_p, 0x34, x(rd::<f32>(c_p, 0x34)).fadd(sy).to_f32());
                let k = kf(0x86C890);
                let w = x(rd::<f32>(this, 0x14)).fcos().fdivr_m(k).fsub_m(k);
                wr::<f32>(c_p, 0x38, x(rd::<f32>(c_p, 0x38)).fadd(sz).fadd(w).to_f32());
                matrix_update_rw(c_p);
                entity_update_rw_frame(rd::<GPtr<u8>>(this, 0));
                matrix_dtor(d_p);
                matrix_dtor(c_p);
            }
        }
        if !outside_dead_zone(rd::<f32>(this, 0x14)) {
            wr::<u32>(this, 0x14, 0);
        } else {
            roll_bone(cd, h, rd::<GPtr<u8>>(this, 0), 0x2B, rd::<f32>(this, 0x14));
            roll_bone(cd, h, rd::<GPtr<u8>>(this, 0), 0x35, rd::<f32>(this, 0x14));
            let p = rd::<GPtr<u8>>(this, 0);
            wr::<u32>(p, 0x470, rd::<u32>(p, 0x470) | 0x4000);
        }
    }
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & !8);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_PED_IK_PITCH_FOR_SLOPE,
    HOOK_CONSTRUCTOR,
    HOOK_GET_WORLD_MATRIX,
    HOOK_MOVE_LIMB,
    HOOK_MOVE_LIMB_NORMALIZED,
    HOOK_ROTATE_TORSO,
    HOOK_ROTATE_TORSO_FOR_ARM,
];
