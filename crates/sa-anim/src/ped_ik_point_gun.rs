//! `CPedIK::PointGunInDirection` (batch `samp-anim-1`), ported from the 1.0 US bytes (gta-reversed `PedIK.cpp` as a
//! reading aid). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::*;

orig! {
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    fn get_anim_hierarchy_from_skin_clump(clump: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0)
    fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120)
    fn rp_hanim_hierarchy_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `RwMatrixCreate()` (0x7F2A50) / `RwMatrixDestroy(RwMatrix*)` (0x7F2A20)
    fn rw_matrix_create() -> GPtr<u8> = cdecl 0x7F2A50 => unported;
    fn rw_matrix_destroy(m: GPtr<u8>) -> u32 = cdecl 0x7F2A20 => unported;
    /// `CPedIK::MoveLimb(LimbOrientation&, float yaw, float pitch, LimbMovementInfo&)` (0x5FDA60, thiscall, `ret 0x10`)
    fn move_limb(ik: GPtr<u8>, limb: GPtr<u8>, yaw: u32, pitch: u32, info: u32) -> i32 = thiscall 0x5FDA60 => unported;
    /// `CPedIK::MoveLimb(LimbOrientation&, float yaw, float pitch, LimbMovementInfo&, float normalize)` (0x5FDB60,
    /// thiscall, `ret 0x14`)
    fn move_limb_normalized(ik: GPtr<u8>, limb: GPtr<u8>, yaw: u32, pitch: u32, info: u32, n: u32) -> i32 = thiscall 0x5FDB60 => unported;
    /// `RtQuatRotate(RtQuat*, const RwV3d* axis, float angle, RwOpCombineType)` (0x7EB7C0)
    fn rt_quat_rotate(q: GPtr<u8>, axis: GPtr<u8>, angle: f32, op: i32) -> GPtr<u8> = cdecl 0x7EB7C0 => unported;
}

/// `CPedIK::ms_torsoInfo` (0x8D22E4), `XaxisIK` (0x8D232C), the radians-to-degrees factor 0x859878 (57.2958f).
const TORSO_INFO: u32 = 0x8D_22E4;
const X_AXIS_IK: usize = 0x8D_232C;
const RAD_TO_DEG: u32 = 0x4265_2EE0;

/// `CPedIK::PointGunInDirection(float zAngle, float distance, bool flag, float normalize)` (0x5FDC00, thiscall,
/// `ret 0x10`, AL). `this +0` the ped, +4 the torso orientation (yaw, pitch), +0x1C the flags. Result starts true;
/// flags: bit 0 (gun reached target) cleared, bit 1 (torso used) set. `angle = f32(LimitRadianAngle(f32(zAngle -
/// ped.rotation(+0x558))))`. The right clavicle's matrix (`RpHAnimIDGetIndex(hierarchy, ped.bones(+0x4C8)->tag)`)
/// is copied into a matrix from `RwMatrixCreate` and destroyed unused, as in the original. `MoveLimb(torso, angle,
/// distance, ms_torsoInfo)` when `normalize < 0` (C0, so NaN too), else with `normalize`: 0 → result false, 2 →
/// flag bit 0. `h = -LimitRadianAngle(f32(fpatan(-head.at.y, -head.at.x) - rotation))` (matrix 2 of the hierarchy);
/// the axis `(0, cos h, sin h)` with `flag`, else `(0, -sin h, cos h)` (x87 `fsin`/`fcos`, stored as f32). The upper
/// torso key frame's quaternion (`ped.bones(+0x48C)->+0x10`) is rotated about that axis by `pitch * 57.2958f` and
/// then about `XaxisIK` by `yaw * 57.2958f` (`RtQuatRotate(..., rwCOMBINEPOSTCONCAT)`); ped flags +0x470 |= 0x4000.
#[hook(addr = 0x5FDC00, conv = thiscall, name = "CPedIK::PointGunInDirection", expect = "83 EC 10 53 55 8B E9",
       unverified = "samp-anim-1", route = "L2", hazards = "x87-transcendental", recs = "weapon-combat",
       note = "FPATAN, FSIN, FCOS; AL result; the original stores the limited angles into its zAngle argument slot")]
pub fn ped_ik_point_gun_in_direction(this: GPtr<u8>, z_angle: f32, distance: u32, flag: u32, normalize: f32) -> u8 {
    let ped = rd::<GPtr<u8>>(this, 0);
    let mut result = 1u8;
    let a = X87::from_f32(z_angle).fsub_m(rd::<f32>(ped, 0x558)).to_f32();
    let angle = X87::from_f64_exact(limit_radian_angle(a)).to_f32();
    wr::<u32>(this, 0x1C, (rd::<u32>(this, 0x1C) & !1) | 2);
    let hier = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(ped, 0x18));
    let index = rp_hanim_id_get_index(hier, rd::<i32>(rd::<GPtr<u8>>(ped, 0x4C8), 0x14));
    let m = rw_matrix_create();
    let src = rp_hanim_hierarchy_get_matrix_array(hier).byte_add((index << 6) as isize);
    for i in 0..16isize {
        wr::<u32>(m, 4 * i, rd::<u32>(src, 4 * i));
    }
    rw_matrix_destroy(m);
    let torso = this.byte_add(4);
    let r = if X87::from_f32(normalize).fcom(X87::from_f32(0.0)).c0() {
        move_limb(this, torso, angle.to_bits(), distance, TORSO_INFO)
    } else {
        move_limb_normalized(this, torso, angle.to_bits(), distance, TORSO_INFO, normalize.to_bits())
    };
    if r == 0 {
        result = 0;
    } else if r == 2 {
        wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 1);
    }
    let mats = rp_hanim_hierarchy_get_matrix_array(get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(ped, 0x18)));
    let y = X87::from_f32(rd::<f32>(mats, 0xA4)).fchs();
    let xv = X87::from_f32(rd::<f32>(mats, 0xA0)).fchs();
    let d = y.fpatan(xv).fsub_m(rd::<f32>(ped, 0x558)).to_f32();
    let h = X87::from_f64_exact(limit_radian_angle(d)).fchs();
    let (ay, az) = if flag as u8 != 0 {
        (h.fcos().to_f32(), h.fsin().to_f32())
    } else {
        (h.fsin().fchs().to_f32(), h.fcos().to_f32())
    };
    let mut axis = [0f32, ay, az];
    let q = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x48C), 0x10);
    let k = f32::from_bits(RAD_TO_DEG);
    rt_quat_rotate(q, GPtr::from_raw(axis.as_mut_ptr()).cast(), X87::from_f32(rd::<f32>(this, 8)).fmul_m(k).to_f32(), 2);
    rt_quat_rotate(q, GPtr::from_addr(X_AXIS_IK), X87::from_f32(rd::<f32>(torso, 0)).fmul_m(k).to_f32(), 2);
    let pf = ped.byte_add(0x470).cast::<u32>();
    pf.write(pf.read() | 0x4000);
    result
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PED_IK_POINT_GUN_IN_DIRECTION];
