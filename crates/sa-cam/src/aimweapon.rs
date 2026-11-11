//! PORT-CAM-3 (helper A): `CCam::Process_AimWeapon` (0x521500), the over-the-shoulder aiming camera
//! (modes 53 / 55 / 65). Ported from the 1.0 US bytes (gta-reversed: `NOTSA_UNREACHABLE()` stub;
//! `docs/re/specs/CAM/CCam.md` 0x521500 as reading aid; the bytes win where they differ).
//!
//! Same technique as [`crate::followped`]: the locals live in a frame array at the original's
//! `B`-relative offsets (B = ESP after the four register pushes; the target argument slot `B+0x6C` is
//! scratch once the target is copied), and the x87 code runs on the [`Fpu`] machine. FPTAN uses
//! sa-math's 64-bit model; FPATAN / FSIN / FCOS the f64 models; `_CIasin`, `_CIacos`, `_CIpow` and
//! `_ftol2` run or model the CRT as the other CAM ports do. `CCamera::Find3rdPersonQuickAimPitch`'s ST0
//! comes back through `orig!` as an f64; it is exact (the FPATAN result is rounded to 24 bits by the
//! `fadd` before the return).
//!
//! Also here: `CCamera::Find3rdPersonQuickAimPitch` (0x50AD40) and `Find3rdPersonCamTargetVector`
//! (0x514970), both with FPTAN (sa-math's 64-bit model).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::camcol::{get_task_jet_pack, vector_magnitude, vector_sub};
use crate::cammodes::{get_weapon_info, ped_get_bone_position, placeable_set_heading, update_rw_matrix};
use crate::camproc::{
    cross_product, get_pad, get_task_fighting, get_vectors_ready_for_rw, max_f, vec_add, vec_s_mul,
};
use crate::common::{Fpu, ftol32, g, k, rd, the_camera, wr};
use crate::followped::{
    camera_col_det_and_react, camera_generic_mode_special_cases, ci_asin, ci_pow, clamped_asin_of_dot,
    clip_beta, copy3, d, entity_pos, fcos, fpatan, fsin, get_task_use_gun, improve_near_clip, ped_is_player,
    push_st0, reset_ducking_system,
};

orig! {
    /// `CWeapon::IsTypeMelee()` (0x73B1C0, AL)
    pub fn weapon_is_type_melee(w: GPtr<u8>) -> u8 = thiscall 0x73B1C0 => unported;
    /// `CVector::operator*=(float)` (0x40FEF0, `ret 4`); the float as a dword
    pub fn vector_scale_assign(v: GPtr<u8>, s: u32) -> GPtr<u8> = thiscall 0x40FEF0 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool buildings, bool vehicles,
    /// bool peds, bool objects, bool dummies, bool seeThrough, bool ignoreSomeObjects)` (0x56A490, AL)
    pub fn get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, ig: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CEntity::UpdateRpHAnim()` (0x532B20)
    pub fn update_rp_h_anim(e: GPtr<u8>) = thiscall 0x532B20 => unported;
    /// `CPedClothesDesc::HasVisibleNewHairCut(int32)` (0x5A7970, AL)
    pub fn has_visible_new_hair_cut(c: GPtr<u8>, n: i32) -> u8 = thiscall 0x5A7970 => unported;
    /// `CPad::GetEnterTargeting()` (0x5406B0, AL)
    pub fn pad_get_enter_targeting(pad: GPtr<u8>) -> u8 = thiscall 0x5406B0 => unported;
    /// `CPad::GetWeapon(CPed*)` (0x540180, int32 in EAX)
    pub fn pad_get_weapon(pad: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x540180 => unported;
    /// `CCamera::HandleCameraMotionForDuckingDuringAim(CPed*, CVector*, CVector*, bool)` (0x50D090)
    pub fn handle_camera_motion_for_ducking_during_aim(cam: GPtr<u8>, ped: GPtr<u8>, src: GPtr<u8>, targ: GPtr<u8>, instant: u32) = thiscall 0x50D090 => crate::camfx::handle_camera_motion_for_ducking_during_aim;
    /// `CCamera::SetColVarsAimWeapon(int32)` (0x50CBF0, cdecl)
    pub fn set_col_vars_aim_weapon(mode: i32) = cdecl 0x50CBF0 => crate::camfx::set_col_vars_aim_weapon;
    /// `CCamera::CameraPedAimModeSpecialCases(CPed*)` (0x50CDA0)
    pub fn camera_ped_aim_mode_special_cases(cam: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x50CDA0 => crate::camera::camera_ped_aim_mode_special_cases;
    /// `CCamera::Find3rdPersonQuickAimPitch()` (0x50AD40, ST0)
    pub fn find_3rd_person_quick_aim_pitch(cam: GPtr<u8>) -> f64 = thiscall 0x50AD40 => unported;
}

/// CRT `_CIacos` (0x822380): operand and result in ST0.
const CI_ACOS: usize = 0x822380;

/// Statics: the init guard (0xB70110), the lock-on point (0xB70104), the melee pitch / yaw offsets
/// (0xB70100 / 0xB700FC), the lock-on blend (0xB700F4), the line-of-sight timer (0xB700F8).
const GUARD: usize = 0xB70110;
const S_LOCK: usize = 0xB70104;
const S_A: usize = 0xB70100;
const S_B: usize = 0xB700FC;
const S_C: usize = 0xB700F4;
const S_LOS_TIMER: usize = 0xB700F8;
/// Auto-aim flag (u8 0xB6EC44), idle time (i32 0xB6EC48), the passenger targeting time (u32 0xB6EC4C),
/// the remembered heading (0x8CC530) and its two thresholds (i32 0x8CC534 / 0x8CC538).
const S_AUTO_AIM: usize = 0xB6EC44;
const S_IDLE_MS: usize = 0xB6EC48;
const S_TARGET_MS: usize = 0xB6EC4C;
const S_HEADING: usize = 0x8CC530;
const IDLE_AUTO: usize = 0x8CC534;
const IDLE_SOFT: usize = 0x8CC538;
/// The aim type table (7 f32 per row, `.data`): +0 baseDist, +4 distScale, +8 alphaDistMul,
/// +0xC alphaDefault, +0x10 zOffset, +0x14 alphaMax, +0x18 alphaMin.
const AT: usize = 0x8CC4C0;
/// The script heading step (0xA44498) and its target (0xA4449C).
const SCRIPT_STEP: usize = 0xA44498;
const SCRIPT_HEADING: usize = 0xA4449C;
/// `TheCamera` fields and other globals.
const TRANSITION_STATE: usize = 0xB6F080;
const CAM_DIRECTLY_BEHIND: usize = 0xB6F042;
const CAM_DIRECTLY_IN_FRONT: usize = 0xB6F043;
const CAMERA_TARGET: usize = 0xB6F980;
const USE_MOUSE_3RD_PERSON: usize = 0xB6EC2E;
const MOUSE_ACCEL_VERTICAL: usize = 0xB6EC18;
const MOUSE_ACCEL_HORZNTL: usize = 0xB6EC1C;
const MOUSE_X: usize = 0xB73424;
const MOUSE_Y: usize = 0xB73428;
const CHAIR_MULT_X: usize = 0xB6EC14;
const CHAIR_MULT_Y: usize = 0xB6EC10;
const ASPECT_RATIO: usize = 0xC3EFA4;
const G_CAMERA_DIRECTION: usize = 0x8CC384;
const TIME_STEP: usize = 0xB7CB5C;
const TIME_MS: usize = 0xB7CB84;
/// `CWeapon::ms_fExtinguisherAimAngle` (0x8D610C)
const EXTINGUISHER_AIM_ANGLE: usize = 0x8D610C;
/// The side-offset switch (u8 0x8CCE64).
const SIDE_FROM_CAMERA: usize = 0x8CCE64;

/// `.rdata` constants (bits; the address in the comment).
const K_1: u32 = 0x3F80_0000; // [0x858624], [0x862F1C]
const K_0: u32 = 0; // [0x858B50], qword 0.0 [0x859EF8]
const K_M1: u32 = 0xBF80_0000; // [0x858C1C]
const K_0_1: u32 = 0x3DCC_CCCD; // [0x858B1C]
const K_0_02: u32 = 0x3CA3_D70A; // [0x858B38]
const K_0_5: u32 = 0x3F00_0000; // [0x858B8C]
const K_4: u32 = 0x4080_0000; // [0x858B90]
const K_1000: u32 = 0x447A_0000; // [0x858C4C]
const K_2P32: u32 = 0x4F80_0000; // [0x858C54]
const K_2: u32 = 0x4000_0000; // [0x858CA0]
const K_PI: u32 = 0x4049_0FDB; // [0x858CB8]
const K_2PI: u32 = 0x40C9_0FDB; // [0x858CBC]
const K_MPI: u32 = 0xC049_0FDB; // [0x858CC0]
const K_0_2: u32 = 0x3E4C_CCCD; // [0x858CC4]
const K_70: u32 = 0x428C_0000; // [0x858CE0], [0x862F2C]
const K_30DEG: u32 = 0x3F06_0A92; // [0x858F20]
const K_2_PI: u32 = 0x3F22_F983; // 2/pi [0x858FB8]
const K_HALF_PI: u32 = 0x3FC9_0FDB; // [0x858FE4]
const K_M100: u32 = 0xC2C8_0000; // [0x859014]
const K_DEG2RAD: u32 = 0x3C8E_FA35; // [0x8595EC]
const K_M1000: u32 = 0xC47A_0000; // [0x859948]
const K_1_14: u32 = 0x3D92_4925; // [0x859B50]
const K_0_0125: u32 = 0x3C4C_CCCD; // [0x85A5C4]
const K_M0_05: u32 = 0xBD4C_CCCD; // [0x862F18]
const K_3: u32 = 0x4040_0000; // [0x862F20]
const K_20: u32 = 0x41A0_0000; // [0x862F24]
const K_0_96: u32 = 0x3F75_C28F; // [0x862F30]
const K_3_70: u32 = 0x3D2F_8AF9; // [0x8631AC]
const K_M2_5: u32 = 0xC020_0000; // [0x8631B0]
const K_M2PI: u32 = 0xC0C9_0FDB; // [0x863234]
const K_0_7: u32 = 0x3F33_3333; // the pushed 0.7f
const K_100: u32 = 0x42C8_0000;
const K_M100_TIMER: u32 = 0xC2C8_0000;

/// `ped->m_aWeapons[ped->m_nActiveWeaponSlot]` (`+0x5A0 + 0x1C * (int8)+0x718`).
fn active_weapon(ped: GPtr<u8>) -> GPtr<u8> {
    ped.byte_add(0x5A0 + 0x1C * isize::from(rd::<i8>(ped, 0x718)))
}

/// The clamp of the f32 at `v` to [-1, 1] into `B+slot` (`fld 1.0; fcomp m; test ah, 5; jnp` /
/// `fld m; fld -1.0; fcomp st(1); fstp st(0); test ah, 0x41; jne` / `fld 1.0; fcomp m; jp`), then
/// `fld B+slot; call _CIasin`.
fn clamped_asin_of_mem(f: &mut Fpu, b: GPtr<u8>, slot: isize, v: GPtr<u8>) {
    f.fld(k(K_1));
    f.fcomp_m(rd(v, 0));
    let lo = f.jp(0x5) && {
        f.fld(rd(v, 0));
        f.fld(k(K_M1));
        f.fcomp_st(1);
        f.fstp_st(0);
        !f.jne(0x41)
    };
    if lo {
        wr::<u32>(b, slot, K_M1);
    } else {
        f.fld(k(K_1));
        f.fcomp_m(rd(v, 0));
        let r = if f.jp(0x5) { rd::<u32>(v, 0) } else { K_1 };
        wr::<u32>(b, slot, r);
    }
    f.fld(rd(b, slot));
    ci_asin(f);
}

/// The wrap of ST0 (a target angle `t`) around the angle at `a`: `fld st(0); fsub [a]; fcom pi;
/// test ah, 0x41; jne` -> `fstp st(0); fsub 2pi`, else `fcomp -pi; test ah, 5; jp` -> `fadd 2pi`.
fn wrap_around(f: &mut Fpu, a: f32) {
    f.fld_st(0);
    f.fsub_m(a);
    f.fcom_m(k(K_PI));
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fsub_m(k(K_2PI));
    } else {
        f.fcomp_m(k(K_MPI));
        if !f.jp(0x5) {
            f.fadd_m(k(K_2PI));
        }
    }
}

/// The step of ST0 (`d`) through the dead zone at `B+0x6C` (`dead`): `d > dead` -> `d - dead`,
/// `d < -dead` -> `d + dead`, else 0.0 (`fld -dead; fld st(1); fcompp; test ah, 5; jp`).
fn dead_zone(f: &mut Fpu, b: GPtr<u8>) {
    f.fcom_m(rd(b, 0x6C));
    if !f.jne(0x41) {
        f.fsub_m(rd(b, 0x6C));
        return;
    }
    f.fld(rd(b, 0x6C));
    f.fchs();
    f.fld_st(1);
    f.fcompp();
    if !f.jp(0x5) {
        f.fadd_m(rd(b, 0x6C));
    } else {
        f.fstp_st(0);
        f.fld(k(K_0));
    }
}

/// `CCam::Process_AimWeapon(const CVector& target, float, float, float)` (0x521500, thiscall,
/// `ret 0x10`). See `docs/re/specs/CAM/CCam.md` (0x521500); the order of calls, re-reads and x87
/// operations follows the bytes. Details the bytes have and the spec simplifies:
/// - the mouse step uses `m_fMouseAccelVertical` for alpha and `m_fMouseAccelHorzntl` for beta, and
///   calls `CPad::GetPad(0)` before each of its three tests;
/// - the non-melee offsets compute `tan(FOV * 0.5 * 0.017453292)` with FPTAN and both angles with
///   `fld1; fpatan`;
/// - the driver's auto-aim alpha step has no "dead > 0" test (its dead zone is always 10 degrees),
///   while the beta step tests it;
/// - the ped heading step is skipped when the weapon flag bit 1 is set and the ped is not ducking
///   (`+0x46C & 0x4000000`).
#[hook(addr = 0x521500, conv = thiscall, name = "CCam::Process_AimWeapon", expect = "A0 10 01 B7 00", this = "CCam")]
pub fn process_aim_weapon(
    this: GPtr<u8>,
    target: GPtr<u8>,
    _orientation: u32,
    _speed_var: u32,
    _speed_var_wanted: u32,
) {
    let mut fr = [0u32; 0x1C];
    let b = GPtr::<u32>::from_raw(fr.as_mut_ptr()).cast::<u8>();
    let mut f = Fpu::new();
    let f = &mut f;
    let ts = || d(TIME_STEP);
    let guard = g::<u32>(GUARD);
    if guard.read() & 1 == 0 {
        guard.write(guard.read() | 1);
        for o in [0, 4, 8] {
            g::<u32>(S_LOCK + o).write(0);
        }
    }
    let t0 = rd::<GPtr<u8>>(this, 0x21C);
    if rd::<u8>(t0, 0x36) & 7 != 3 || ped_is_player(t0) == 0 {
        return;
    }
    let ped = rd::<GPtr<u8>>(this, 0x21C);
    let intel = || rd::<GPtr<u8>>(ped, 0x47C);
    // weapon info and aim type
    let wi = if !get_task_use_gun(intel()).is_null() {
        rd::<GPtr<u8>>(get_task_use_gun(intel()), 0x30)
    } else {
        get_weapon_info(rd::<u32>(active_weapon(ped), 0), 1)
    };
    wr(b, 0x1C, wi);
    wr::<i32>(b, 0x18, 0);
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        let ty = if !v.is_null() && (rd::<i32>(v, 0x590) == 9 || rd::<i32>(v, 0x594) == 2) {
            1
        } else {
            2
        };
        wr::<i32>(b, 0x18, ty);
    } else if !get_task_jet_pack(intel()).is_null() {
        wr::<i32>(b, 0x18, 1);
    } else if weapon_is_type_melee(active_weapon(ped)) != 0 {
        wr::<i32>(b, 0x18, 3);
    }
    let wtype = rd::<i32>(active_weapon(ped), 0);
    wr::<u32>(b, 0x14, K_70);
    if wtype == 0x1E || wtype == 0x1F {
        wr::<u32>(b, 0x14, g::<u32>(0x8CC4B4).read());
    } else if wtype == 0x21 {
        wr::<u32>(b, 0x14, g::<u32>(0x8CC4B8).read());
    }
    // 0x521658: FOV toward the weapon's FOV
    if g::<u8>(TRANSITION_STATE).read() == 0 {
        if rd::<u8>(this, 0xA) != 0 && wtype != 0x21 {
            wr::<u32>(this, 0xB4, rd::<u32>(b, 0x14));
        } else {
            f.fld(ts());
            f.fmul_m(k(K_1));
            wr::<f32>(b, 0x28, f.fst());
            f.fadd_m(rd(this, 0xB4));
            f.fld(rd(b, 0x14));
            f.fcomp_st(1);
            if !f.jne(0x41) {
                wr::<f32>(this, 0xB4, f.fstp());
            } else {
                f.fstp_st(0);
                f.fld(rd(this, 0xB4));
                f.fsub_m(rd(b, 0x28));
                f.fld(rd(b, 0x14));
                f.fcomp_st(1);
                if !f.jp(0x5) {
                    wr::<f32>(this, 0xB4, f.fstp());
                } else {
                    let v = rd::<u32>(b, 0x14);
                    f.fstp_st(0);
                    wr::<u32>(this, 0xB4, v);
                }
            }
        }
    }
    // 0x5216D2: yaw / pitch offsets (B+0x10 / B+0x20)
    if rd::<i32>(wi, 0) == 0 {
        melee_offsets(this, b, ped, f);
    } else {
        f.fld(rd(this, 0xB4));
        g::<u32>(S_C).write(0);
        f.fmul_m(k(K_0_5));
        f.fmul_m(k(K_DEG2RAD));
        f.fptan();
        f.fstp_st(0);
        f.fld(d(CHAIR_MULT_X));
        f.fsub_m(k(K_0_5));
        f.fadd_st(0);
        f.fmul_st(1);
        f.fld1();
        fpatan(f);
        wr::<f32>(b, 0x10, f.fstp());
        f.fld(k(K_0_5));
        f.fsub_m(d(CHAIR_MULT_Y));
        f.fadd_st(0);
        f.fld(k(K_1));
        f.fdiv_m(d(ASPECT_RATIO));
        f.fmulp(1);
        f.fmulp(1);
        f.fld1();
        fpatan(f);
    }
    let reset = rd::<u8>(this, 0xA) != 0;
    wr::<f32>(b, 0x20, f.fstp());
    if reset {
        reset_block(this, b, ped, f);
    }
    script_heading_step(this, b, f);
    // 0x521C65: target point
    copy3(b.byte_add(0x38), target);
    wr::<u32>(b, 0x24, rd::<u32>(b, 0x40));
    update_rp_h_anim(ped);
    let pos = entity_pos(ped);
    let at = AT + 0x1C * rd::<i32>(b, 0x18) as usize;
    f.fld(rd(pos, 8));
    f.fadd_m(k(K_0_5));
    f.fadd_m(d(at + 0x10));
    f.fld(rd(this, 0xB4));
    f.fcomp_m(k(K_70));
    if !f.jp(0x5) {
        f.fld(k(K_70));
        f.fsub_m(rd(this, 0xB4));
        f.fld(k(K_70));
        f.fsub_m(d(0x8CC4B4));
        f.fdivp(1);
        f.fld(k(K_1));
        f.fcomp_st(1);
        if !f.jp(0x5) {
            f.fstp_st(0);
            f.fld(k(K_1));
        }
        f.fmul_m(k(K_0_1));
        f.faddp(1);
    }
    wr::<f32>(b, 0x40, f.fst());
    let wflags = rd::<u32>(wi, 0x18);
    f.fsub_m(rd(b, 0x24));
    wr::<f32>(b, 0x24, f.fstp());
    wr::<u32>(b, 0x6C, K_0_2);
    let hair = (wflags >> 1) & 1 == 0
        && has_visible_new_hair_cut(rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x480), 4), 1) != 0;
    if hair {
        wr::<u32>(b, 0x6C, 0x3E99_999A);
    } else {
        f.fld(rd(this, 0xB4));
        f.fcomp_m(k(K_70));
        if !f.jp(0x5) {
            f.fld(k(K_70));
            f.fsub_m(rd(this, 0xB4));
            f.fld(k(K_70));
            f.fsub_m(d(0x8CC4B8));
            f.fdivp(1);
            f.fld(k(K_1));
            f.fcomp_st(1);
            if !f.jp(0x5) {
                f.fstp_st(0);
                f.fld(k(K_1));
            }
            f.fmul_m(k(K_0_1));
            f.fadd_m(k(K_0_2));
            wr::<f32>(b, 0x6C, f.fstp());
        }
    }
    side_offset(this, b, ped, f);
    lock_on_point(this, b, ped, wi, f);
    if !rd::<GPtr<u8>>(ped, 0x71C).is_null() {
        lock_on_orientation(this, b, wi, at, f);
    } else {
        free_orientation(this, b, ped, wi, at, f);
    }
    clip_and_output(this, b, ped, wi, at, f);
}

/// 0x5216DB: the melee offsets (`a` B+0x20, `b` B+0x14, `c` B+0x28), the line-of-sight timer and the
/// smoothing of the statics; leaves `sA * deg2rad` in ST0 and stores `sB * deg2rad` in B+0x10.
fn melee_offsets(this: GPtr<u8>, b: GPtr<u8>, ped: GPtr<u8>, f: &mut Fpu) {
    let ts = || d(TIME_STEP);
    let guard = g::<u32>(GUARD);
    let mut gv = guard.read();
    if gv & 2 == 0 {
        gv |= 2;
        guard.write(gv);
        g::<u32>(S_A).write(K_3);
    }
    if gv & 4 == 0 {
        gv |= 4;
        guard.write(gv);
        g::<u32>(S_B).write(K_20);
    }
    wr::<u32>(b, 0x20, K_3);
    wr::<u32>(b, 0x14, K_20);
    wr::<u32>(b, 0x28, 0);
    let set_abc = |b: GPtr<u8>| {
        wr::<u32>(b, 0x20, 0);
        wr::<u32>(b, 0x14, K_70);
        wr::<u32>(b, 0x28, K_1);
    };
    if !get_task_fighting(rd::<GPtr<u8>>(ped, 0x47C)).is_null() && rd::<i32>(ped, 0x534) < 4 && {
        let o = rd::<GPtr<u8>>(ped, 0x71C);
        !o.is_null()
    } {
        let o = rd::<GPtr<u8>>(ped, 0x71C);
        f.fld(d(S_LOS_TIMER));
        f.fcomp_m(ts());
        let step = if !f.jne(0x41) {
            f.fld(d(S_LOS_TIMER));
            f.fsub_m(ts());
            true
        } else {
            f.fld(ts());
            f.fchs();
            f.fcomp_m(d(S_LOS_TIMER));
            if !f.jne(0x41) {
                f.fld(ts());
                f.fadd_m(d(S_LOS_TIMER));
                true
            } else {
                false
            }
        };
        if step {
            g::<f32>(S_LOS_TIMER).write(f.fst());
            f.fcomp_m(k(K_0));
            if !f.jne(0x1) {
                set_abc(b);
            }
        } else if los_test(b, ped, o) {
            g::<u32>(S_LOS_TIMER).write(K_100);
            set_abc(b);
        } else {
            g::<u32>(S_LOS_TIMER).write(K_M100_TIMER);
        }
    }
    if rd::<u8>(this, 0xA) != 0 {
        g::<u32>(S_A).write(rd::<u32>(b, 0x20));
        g::<u32>(S_B).write(rd::<u32>(b, 0x14));
        g::<u32>(S_C).write(0);
    } else if g::<u8>(TRANSITION_STATE).read() == 0 {
        f.fld(k(K_0_96));
        f.fld(ts());
        ci_pow(f);
        f.fld(k(K_1));
        f.fsub_st(1);
        f.fld(rd(b, 0x20));
        f.fmul_st(1);
        f.fld(d(S_A));
        f.fmul_st(3);
        f.faddp(1);
        g::<f32>(S_A).write(f.fstp());
        f.fld(rd(b, 0x14));
        f.fmul_st(1);
        f.fld(d(S_B));
        f.fmul_st(3);
        f.faddp(1);
        g::<f32>(S_B).write(f.fstp());
        f.fmul_m(rd(b, 0x28));
        f.fld(d(S_C));
        f.fmul_st(2);
        f.faddp(1);
        g::<f32>(S_C).write(f.fstp());
        f.fstp_st(0);
    }
    f.fld(d(S_B));
    f.fmul_m(k(K_DEG2RAD));
    wr::<f32>(b, 0x10, f.fstp());
    f.fld(d(S_A));
    f.fmul_m(k(K_DEG2RAD));
}

/// 0x5217EF: `P0 = pos(ped) + (0, 0, 0.75)` (B+0x44), `R = cross(pos(o) - pos(ped), (0, 0, 1))`
/// (B+0x2C, scaled by `2 / max(0.7, |R|)`), `P1 = R + P0` (B+0x50); the line of sight P0 -> P1.
fn los_test(b: GPtr<u8>, ped: GPtr<u8>, o: GPtr<u8>) -> bool {
    let mut f = Fpu::new();
    let m = rd::<GPtr<u8>>(ped, 0x14);
    wr::<u32>(b, 0x2C, 0);
    wr::<u32>(b, 0x30, 0);
    wr::<u32>(b, 0x34, 0x3F40_0000);
    let pp = if m.is_null() {
        ped.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    vec_add(b.byte_add(0x44), pp, b.byte_add(0x2C));
    wr::<u32>(b, 0x50, 0);
    wr::<u32>(b, 0x54, 0);
    wr::<u32>(b, 0x58, K_1);
    let pped = if m.is_null() {
        ped.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    let po = entity_pos(o);
    let s = vector_sub(b.byte_add(0x5C), po, pped);
    cross_product(b.byte_add(0x2C), s, b.byte_add(0x50));
    push_st0(&mut f, vector_magnitude(b.byte_add(0x2C)));
    let mag = f.fstp().to_bits();
    push_st0(&mut f, max_f(K_0_7, mag));
    f.fdivr_m(k(K_2));
    vector_scale_assign(b.byte_add(0x2C), f.fstp().to_bits());
    for i in 0..3 {
        f.fld(rd(b, 0x2C + 4 * i));
        f.fadd_m(rd(b, 0x44 + 4 * i));
        wr::<f32>(b, 0x50 + 4 * i, f.fstp());
    }
    get_is_line_of_sight_clear(b.byte_add(0x44), b.byte_add(0x50), 1, 1, 0, 1, 0, 1, 1) != 0
}

/// 0x5219F7: the statics reset (`m_bResetStatics`).
fn reset_block(this: GPtr<u8>, b: GPtr<u8>, ped: GPtr<u8>, f: &mut Fpu) {
    reset_ducking_system(the_camera(), ped);
    wr::<u8>(this, 0xB, 0);
    wr::<u8>(this, 0x3, 1);
    wr::<u32>(this, 0xB0, 0);
    wr::<u32>(this, 0xC0, 0);
    let mouse = g::<u8>(USE_MOUSE_3RD_PERSON).read();
    g::<u8>(S_AUTO_AIM).write(1);
    g::<u32>(S_IDLE_MS).write(60000);
    g::<u32>(S_HEADING).write(0xC47A_4000);
    g::<u32>(S_TARGET_MS).write(0);
    if mouse != 0 && rd::<GPtr<u8>>(ped, 0x71C).is_null() {
        return;
    }
    let at = AT + 0x1C * rd::<i32>(b, 0x18) as usize;
    wr::<u32>(this, 0xAC, g::<u32>(at + 0xC).read());
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 && !rd::<GPtr<u8>>(ped, 0x58C).is_null() {
        f.fld(rd(ped, 0x558));
        f.fsub_m(k(K_HALF_PI));
        f.fsub_m(rd(b, 0x10));
        wr::<f32>(this, 0xBC, f.fstp());
        let vm = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x58C), 0x14);
        clamped_asin_of_mem(f, b, 0x14, vm.byte_add(0x18));
        f.fadd_m(rd(this, 0xAC));
        wr::<f32>(this, 0xAC, f.fstp());
    } else if rd::<GPtr<u8>>(ped, 0x71C).is_null() {
        f.fld(rd(ped, 0x558));
        f.fsub_m(k(K_HALF_PI));
        f.fadd_m(rd(b, 0x10));
        wr::<f32>(this, 0xBC, f.fstp());
        if rd::<u8>(ped, 0x46C) & 1 != 0 {
            clamped_asin_of_dot(f, b, 0x14, ped);
            f.fsubr_m(rd(this, 0xAC));
            wr::<f32>(this, 0xAC, f.fst());
            if rd::<i32>(active_weapon(ped), 0) == 0x2A {
                f.fadd_m(d(EXTINGUISHER_AIM_ANGLE));
                wr::<f32>(this, 0xAC, f.fstp());
            } else {
                f.fstp_st(0);
            }
        }
    }
}

/// 0x521BBB: the script's heading step (`[0xA44498]` per frame toward `[0xA4449C]`).
fn script_heading_step(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu) {
    f.fld(d(SCRIPT_STEP));
    f.fcomp_m(k(K_0));
    if f.jne(0x41) {
        return;
    }
    f.fld(rd(this, 0xBC));
    f.fsub_m(d(SCRIPT_HEADING));
    f.fcom_m(k(K_0));
    if !f.jp(0x5) {
        f.fadd_m(k(K_2PI));
    }
    f.fld(k(K_2PI));
    f.fsub_st(1);
    wr::<f32>(b, 0x28, f.fstp());
    f.fcom_m(d(SCRIPT_STEP));
    let snap = !f.jp(0x5) || {
        f.fld(rd(b, 0x28));
        f.fcomp_m(d(SCRIPT_STEP));
        !f.jp(0x5)
    };
    if snap {
        let h = g::<u32>(SCRIPT_HEADING).read();
        f.fstp_st(0);
        wr::<u32>(this, 0xBC, h);
        g::<u32>(SCRIPT_STEP).write(0);
        return;
    }
    f.fcomp_m(rd(b, 0x28));
    if !f.jne(0x41) {
        f.fld(d(SCRIPT_STEP));
        f.fadd_m(rd(this, 0xBC));
    } else {
        f.fld(rd(this, 0xBC));
        f.fsub_m(d(SCRIPT_STEP));
    }
    wr::<f32>(this, 0xBC, f.fstp());
}

/// 0x521D8F: the sideways shift of T (B+0x38..0x40) by `sideOff` (B+0x6C): along the camera's right
/// vector scaled by `1 - acos(clamp(R . pedRight, 0, 1)) * 2/pi` (switch 0x8CCE64 set), else along the
/// ped's right vector.
fn side_offset(this: GPtr<u8>, b: GPtr<u8>, ped: GPtr<u8>, f: &mut Fpu) {
    if g::<u8>(SIDE_FROM_CAMERA).read() == 0 {
        let m = rd::<GPtr<u8>>(ped, 0x14);
        f.fld(rd(b, 0x6C));
        f.fmul_m(rd(m, 0));
        f.fld(rd(b, 0x6C));
        f.fmul_m(rd(m, 4));
        f.fld(rd(b, 0x6C));
        f.fmul_m(rd(m, 8));
        wr::<f32>(b, 0x64, f.fstp());
        f.fxch(1);
        f.fadd_m(rd(b, 0x38));
        wr::<f32>(b, 0x38, f.fstp());
        f.fadd_m(rd(b, 0x3C));
        wr::<f32>(b, 0x3C, f.fstp());
        f.fld(rd(b, 0x64));
    } else {
        cross_product(b.byte_add(0x2C), this.byte_add(0x190), this.byte_add(0x1B4));
        f.fld(rd(b, 0x30));
        let m = rd::<GPtr<u8>>(ped, 0x14);
        f.fmul_m(rd(m, 4));
        f.fld(rd(b, 0x34));
        f.fmul_m(rd(m, 8));
        f.faddp(1);
        f.fld(rd(b, 0x2C));
        f.fmul_m(rd(m, 0));
        f.faddp(1);
        wr::<f32>(b, 0x14, f.fstp());
        f.fld(k(K_1));
        f.fcomp_m(rd(b, 0x14));
        if !f.jp(0x5) {
            f.fld(k(K_1));
            wr::<f32>(b, 0x28, f.fstp());
        } else {
            f.fld(rd(b, 0x14));
            f.fld(k(K_0));
            f.fcomp_m(rd(b, 0x14));
            if f.jne(0x41) {
                wr::<f32>(b, 0x28, f.fstp());
            } else {
                f.fstp_st(0);
                wr::<u32>(b, 0x28, 0);
            }
        }
        f.fld(rd(b, 0x28));
        let v = f.pop();
        push_st0(f, sa_core::orig::x87_unary(CI_ACOS, v.to_f64()));
        f.fmul_m(k(K_2_PI));
        f.fsubr_m(k(K_1));
        f.fmul_m(rd(b, 0x6C));
        f.fld(rd(b, 0x2C));
        f.fmul_st(1);
        f.fld(rd(b, 0x30));
        f.fmul_st(2);
        wr::<f32>(b, 0x54, f.fstp());
        f.fld(rd(b, 0x34));
        f.fmul_st(2);
        wr::<f32>(b, 0x58, f.fstp());
        f.fadd_m(rd(b, 0x38));
        wr::<f32>(b, 0x38, f.fstp());
        f.fstp_st(0);
        f.fld(rd(b, 0x3C));
        f.fadd_m(rd(b, 0x54));
        wr::<f32>(b, 0x3C, f.fstp());
        f.fld(rd(b, 0x58));
    }
    f.fadd_m(rd(b, 0x40));
    wr::<f32>(b, 0x40, f.fstp());
}

/// 0x521E9A: the lock-on point L (B+0x2C..0x34) and its smoothed static `sLock`.
fn lock_on_point(this: GPtr<u8>, b: GPtr<u8>, ped: GPtr<u8>, wi: GPtr<u8>, f: &mut Fpu) {
    let o = rd::<GPtr<u8>>(ped, 0x71C);
    if o.is_null() {
        return;
    }
    wr::<u32>(b, 0x2C, 0);
    wr::<u32>(b, 0x30, 0);
    wr::<u32>(b, 0x34, 0);
    if rd::<u8>(o, 0x36) & 7 == 3 && rd::<i32>(wi, 0) != 0 {
        ped_get_bone_position(o, b.byte_add(0x2C), 3, 1);
    } else {
        copy3(b.byte_add(0x2C), entity_pos(o));
    }
    if rd::<i32>(wi, 0) == 0 {
        f.fld(rd(b, 0x24));
        f.fmul_m(d(0x8CCE60));
        f.fadd_m(rd(b, 0x34));
        wr::<f32>(b, 0x34, f.fstp());
    }
    if rd::<u8>(this, 0xA) == 0 && !get_task_fighting(rd::<GPtr<u8>>(ped, 0x47C)).is_null() {
        f.fld(d(0x8CC39C));
        f.fld(d(TIME_STEP));
        ci_pow(f);
        wr::<f32>(b, 0x6C, f.fstp());
        f.fld(k(K_1));
        f.fsub_m(rd(b, 0x6C));
        let q = f.fstp().to_bits();
        let r1 = vec_s_mul(b.byte_add(0x5C), q, b.byte_add(0x2C));
        let p = rd::<u32>(b, 0x6C);
        let r2 = vec_s_mul(b.byte_add(0x50), p, g::<u8>(S_LOCK));
        let r = vec_add(b.byte_add(0x44), r2, r1);
        copy3(g::<u8>(S_LOCK), r);
    } else {
        copy3(g::<u8>(S_LOCK), b.byte_add(0x2C));
    }
}

/// 0x521FD4: orientation toward the lock-on point: yaw / pitch of `sLock - T`, the melee pitch
/// `cos(yawOff) * pitch` or the distance ratio on both offsets, then the clamped steps of alpha and beta.
fn lock_on_orientation(this: GPtr<u8>, b: GPtr<u8>, wi: GPtr<u8>, at: usize, f: &mut Fpu) {
    let lock = |i: usize| d(S_LOCK + 4 * i);
    f.fld(lock(0));
    f.fsub_m(rd(b, 0x38));
    let fire = rd::<i32>(wi, 0);
    f.fld(lock(1));
    f.fsub_m(rd(b, 0x3C));
    f.fld(lock(2));
    f.fsub_m(rd(b, 0x40));
    wr::<f32>(b, 0x58, f.fstp());
    f.fld_st(1);
    f.fchs();
    f.fld_st(1);
    fpatan(f);
    f.fsub_m(k(K_HALF_PI));
    wr::<f32>(b, 0x2C, f.fstp());
    f.fld_st(1);
    f.fmul_st(2);
    f.fld_st(1);
    f.fmul_st(2);
    f.faddp(1);
    wr::<f32>(b, 0x28, f.fstp());
    f.fstp_st(0);
    f.fstp_st(0);
    f.fld(rd(b, 0x58));
    f.fld(rd(b, 0x28));
    f.fsqrt();
    f.fxch(1);
    f.fxch(1);
    fpatan(f);
    wr::<f32>(b, 0x6C, f.fstp());
    if fire == 0 {
        f.fld(rd(b, 0x10));
        fcos(f);
        f.fmul_m(rd(b, 0x6C));
        wr::<f32>(b, 0x6C, f.fstp());
    } else {
        let src = this.byte_add(0x19C);
        f.fld(rd(b, 0x38));
        f.fsub_m(rd(src, 0));
        f.fld(rd(b, 0x3C));
        f.fsub_m(rd(src, 4));
        f.fld(rd(b, 0x40));
        f.fsub_m(rd(src, 8));
        f.fld_st(0);
        f.fmul_st(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fld_st(3);
        f.fmul_st(4);
        f.faddp(1);
        f.fsqrt();
        f.fstp_st(3);
        f.fstp_st(0);
        f.fstp_st(0);
        f.fcomp_m(d(at));
        if !f.jp(0x5) {
            push_st0(
                f,
                vector_magnitude(vector_sub(b.byte_add(0x5C), b.byte_add(0x38), src)),
            );
        } else {
            f.fld(d(at));
        }
        f.fld(rd(b, 0x58));
        f.fmul_m(rd(b, 0x58));
        f.fadd_m(rd(b, 0x28));
        f.fsqrt();
        f.fld_st(0);
        f.faddp(2);
        f.fdivp(1);
        f.fld(rd(b, 0x20));
        f.fmul_st(1);
        wr::<f32>(b, 0x20, f.fstp());
        f.fmul_m(rd(b, 0x10));
        wr::<f32>(b, 0x10, f.fstp());
    }
    // 0x5220D2: targets tB = yaw + yawOff, tA = pitch - pitchOff (wrapped by 2pi)
    f.fld(rd(b, 0x2C));
    f.fadd_m(rd(b, 0x10));
    f.fld(rd(b, 0x6C));
    f.fsub_m(rd(b, 0x20));
    f.fcom_m(k(K_MPI));
    if !f.jp(0x5) {
        f.fadd_m(k(K_2PI));
    } else {
        f.fcom_m(k(K_PI));
        if !f.jne(0x41) {
            f.fsub_m(k(K_2PI));
        }
    }
    f.fld(d(TIME_STEP));
    let reset = rd::<u8>(this, 0xA) != 0;
    f.fmul_m(d(0x8CC4A4));
    wr::<f32>(b, 0x6C, f.fstp());
    if reset {
        wr::<u32>(b, 0x6C, K_1000);
    }
    let alpha = this.byte_add(0xAC);
    let beta = this.byte_add(0xBC);
    // alpha toward tA
    f.fld_st(0);
    f.fsub_m(rd(alpha, 0));
    wr::<f32>(b, 0x14, f.fst());
    f.fabs();
    f.fcomp_m(rd(b, 0x6C));
    if f.jp(0x5) {
        f.fstp_st(0);
        f.fld(rd(b, 0x14));
        f.fcomp_m(k(K_0));
        if f.jp(0x5) {
            f.fld(rd(b, 0x6C));
            f.fadd_m(rd(alpha, 0));
        } else {
            f.fld(rd(alpha, 0));
            f.fsub_m(rd(b, 0x6C));
        }
    }
    wr::<f32>(alpha, 0, f.fstp());
    // beta toward tB (wrapped around beta)
    wrap_around(f, rd(beta, 0));
    f.fld_st(0);
    f.fsub_m(rd(beta, 0));
    wr::<f32>(b, 0x14, f.fst());
    f.fabs();
    f.fcomp_m(rd(b, 0x6C));
    if f.jp(0x5) {
        f.fstp_st(0);
        f.fld(rd(b, 0x14));
        f.fcomp_m(k(K_0));
        if f.jp(0x5) {
            f.fld(rd(b, 0x6C));
            f.fadd_m(rd(beta, 0));
        } else {
            f.fld(rd(beta, 0));
            f.fsub_m(rd(b, 0x6C));
        }
    }
    wr::<f32>(beta, 0, f.fstp());
    wr::<u32>(this, 0xB0, 0);
    wr::<u32>(this, 0xC0, 0);
}

/// 0x5221FC: no lock-on: the mouse step, or the pad step with the passenger / driver auto-aim.
fn free_orientation(this: GPtr<u8>, b: GPtr<u8>, ped: GPtr<u8>, wi: GPtr<u8>, at: usize, f: &mut Fpu) {
    let ts = || d(TIME_STEP);
    let alpha = this.byte_add(0xAC);
    let beta = this.byte_add(0xBC);
    if g::<u8>(USE_MOUSE_3RD_PERSON).read() != 0 && rd::<u16>(get_pad(0), 0x10E) == 0 && {
        let _ = get_pad(0);
        f.fld(d(MOUSE_X));
        f.fcomp_m(k(K_0));
        f.jp(0x44) || {
            let _ = get_pad(0);
            f.fld(d(MOUSE_Y));
            f.fcomp_m(k(K_0));
            f.jp(0x44)
        }
    } {
        let _ = get_pad(0);
        f.fld(rd(this, 0xB4));
        f.fmul_m(k(K_0_0125));
        wr::<u32>(b, 0x30, g::<u32>(MOUSE_Y).read());
        f.fld(rd(b, 0x30));
        f.fmul_m(k(K_4));
        wr::<u32>(b, 0x2C, g::<u32>(MOUSE_X).read());
        f.fmul_st(1);
        f.fmul_m(d(MOUSE_ACCEL_VERTICAL));
        f.fld(rd(b, 0x2C));
        f.fmul_m(k(K_M2_5));
        f.fmul_st(2);
        f.fmul_m(d(MOUSE_ACCEL_HORZNTL));
        wr::<u32>(this, 0xC0, 0);
        wr::<u32>(this, 0xB0, 0);
        f.fadd_m(rd(beta, 0));
        wr::<f32>(beta, 0, f.fstp());
        f.fadd_m(rd(alpha, 0));
        wr::<f32>(alpha, 0, f.fstp());
        f.fstp_st(0);
        return;
    }
    // 0x5222CF: pad
    let h = i32::from(crate::camproc::aim_weapon_left_right(get_pad(0), ped)).wrapping_neg();
    wr::<i32>(b, 0x6C, h);
    f.fild(rd(b, 0x6C));
    wr::<f32>(b, 0x24, f.fstp());
    let v = i32::from(crate::camproc::aim_weapon_up_down(get_pad(0), ped));
    wr::<i32>(b, 0x6C, v);
    f.fild(rd(b, 0x6C));
    wr::<u32>(b, 0x6C, g::<u32>(0x8CCE5C).read());
    wr::<f32>(b, 0x28, f.fstp());
    f.fld(rd(b, 0x24));
    f.fabs();
    f.fld(rd(this, 0xB4));
    f.fmul_m(k(K_0_0125));
    f.fld(k(K_1_14));
    f.fmul_st(1);
    f.fmul_st(2);
    f.fmul_m(rd(b, 0x24));
    f.fmul_m(ts());
    f.fmul_m(d(0x8CC4A0));
    f.fmul_m(d(0x8CC4A0));
    wr::<f32>(b, 0x14, f.fstp());
    f.fld(rd(b, 0x28));
    f.fabs();
    wr::<f32>(b, 0x2C, f.fstp());
    f.fmul_m(k(K_3_70));
    f.fmul_m(rd(b, 0x2C));
    f.fmul_m(rd(b, 0x28));
    f.fmul_m(ts());
    f.fmul_m(d(0x8CC4A0));
    f.fmul_m(d(0x8CC4A0));
    wr::<f32>(b, 0x20, f.fstp());
    f.fcomp_m(k(K_2));
    if !f.jp(0x5) && {
        f.fld(rd(b, 0x2C));
        f.fcomp_m(k(K_2));
        !f.jp(0x5)
    } {
        wr::<u32>(b, 0x6C, g::<u32>(0x8CCE58).read());
    }
    f.fld(rd(b, 0x6C));
    f.fld(ts());
    ci_pow(f);
    wr::<f32>(b, 0x6C, f.fstp());
    f.fld(k(K_1));
    f.fsub_m(rd(b, 0x6C));
    wr::<f32>(b, 0x2C, f.fstp());
    f.fld(rd(b, 0x6C));
    f.fmul_m(rd(this, 0xC0));
    f.fld(rd(b, 0x2C));
    f.fmul_m(rd(b, 0x14));
    f.faddp(1);
    wr::<f32>(this, 0xC0, f.fst());
    f.fld(rd(b, 0x2C));
    f.fmul_m(rd(b, 0x20));
    f.fld(rd(b, 0x6C));
    f.fmul_m(rd(this, 0xB0));
    f.faddp(1);
    wr::<f32>(this, 0xB0, f.fst());
    let in_veh = (rd::<u32>(ped, 0x46C) >> 8) & 1;
    f.fxch(1);
    wr::<f32>(b, 0x14, f.fstp());
    wr::<f32>(b, 0x20, f.fstp());
    let v = rd::<GPtr<u8>>(ped, 0x58C);
    let auto_path = if in_veh != 0 && !v.is_null() && rd::<GPtr<u8>>(v, 0x460) != ped {
        // passenger: a quick look back on "enter targeting"
        if pad_get_enter_targeting(get_pad(0)) != 0 {
            let now = g::<u32>(TIME_MS).read();
            let dt = now.wrapping_sub(g::<u32>(S_TARGET_MS).read());
            wr::<u32>(b, 0x6C, dt);
            f.fild(dt as i32);
            if (dt as i32) < 0 {
                f.fadd_m(k(K_2P32));
            }
            f.fcomp_m(d(0x8CCE54));
            if !f.jp(0x5) {
                wr::<u32>(b, 0x14, K_PI);
                wr::<u32>(b, 0x20, 0);
            } else {
                g::<u32>(S_TARGET_MS).write(now);
            }
        }
        g::<u8>(S_AUTO_AIM).read() == 0
    } else if in_veh != 0 && {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        !v.is_null() && rd::<GPtr<u8>>(v, 0x460) == ped
    } {
        driver_idle(this, b, ped, f)
    } else {
        g::<u8>(S_AUTO_AIM).read() == 0
    };
    if !auto_path {
        // 0x5225B9: apply the pad step
        f.fld(rd(b, 0x14));
        g::<u32>(S_HEADING).write(0xC47A_4000);
        f.fadd_m(rd(beta, 0));
        wr::<f32>(beta, 0, f.fstp());
        f.fld(rd(b, 0x20));
        f.fadd_m(rd(alpha, 0));
        wr::<f32>(alpha, 0, f.fstp());
        return;
    }
    auto_aim(this, b, ped, wi, at, f);
}

/// 0x5224AD: the driver's idle time; returns true for the automatic heading (0x522641), false for the
/// pad step (0x5225B9, which sets `sAutoAim = 1` itself).
fn driver_idle(this: GPtr<u8>, b: GPtr<u8>, ped: GPtr<u8>, f: &mut Fpu) -> bool {
    f.fld(rd(b, 0x24));
    f.fcomp_m(k(K_0));
    let moved = f.jp(0x44) || {
        f.fld(rd(b, 0x28));
        f.fcomp_m(k(K_0));
        f.jp(0x44)
    };
    if moved {
        g::<u32>(S_IDLE_MS).write(0);
    } else if pad_get_weapon(get_pad(0), ped) == 0 {
        f.fld(d(TIME_STEP));
        f.fmul_m(k(K_0_02));
        f.fmul_m(k(K_1000));
        let n = ftol32(f.pop());
        let p = g::<i32>(S_IDLE_MS);
        p.write(p.read().wrapping_add(n));
    }
    let lim = g::<i32>(IDLE_AUTO).read();
    let idle = g::<i32>(S_IDLE_MS).read();
    if idle > lim {
        f.fld(rd(ped, 0x558));
        g::<u8>(S_AUTO_AIM).write(0);
        f.fsub_m(k(K_HALF_PI));
        f.fadd_m(rd(b, 0x10));
        g::<f32>(S_HEADING).write(f.fstp());
        return true;
    }
    if idle > g::<i32>(IDLE_SOFT).read() {
        f.fld(rd(ped, 0x558));
        f.fsub_m(k(K_HALF_PI));
        f.fld(rd(b, 0x10));
        f.fsubr_st(1);
        f.fsub_m(rd(this, 0xBC));
        f.fcom_m(k(K_2PI));
        if !f.jne(0x41) {
            f.fsub_m(k(K_2PI));
        } else {
            f.fcom_m(k(K_M2PI));
            if !f.jp(0x5) {
                f.fadd_m(k(K_2PI));
            }
        }
        f.fcomp_m(k(K_30DEG));
        if !f.jp(0x5) {
            f.fadd_m(rd(b, 0x10));
            g::<u8>(S_AUTO_AIM).write(0);
            g::<i32>(S_IDLE_MS).write(lim.wrapping_add(1));
            g::<f32>(S_HEADING).write(f.fstp());
            return true;
        }
        f.fstp_st(0);
        g::<u8>(S_AUTO_AIM).write(1);
        return false;
    }
    g::<u8>(S_AUTO_AIM).write(1);
    if pad_get_weapon(get_pad(0), ped) != 0 {
        g::<u32>(S_IDLE_MS).write(0);
    }
    g::<u8>(S_AUTO_AIM).read() == 0
}

/// 0x522641: the automatic heading: the remembered heading (`sHeading`, set from the ped when below
/// -1000), the ped turned to it (non-melee weapons without flag bit 1), beta toward `hd - yawOff`
/// (driver: a quarter of the rate and a 10 degree dead zone), the driver's alpha toward the vehicle
/// pitch + alphaDefault; otherwise `alpha += pad step`.
fn auto_aim(this: GPtr<u8>, b: GPtr<u8>, ped: GPtr<u8>, wi: GPtr<u8>, at: usize, f: &mut Fpu) {
    let beta = this.byte_add(0xBC);
    f.fld(rd(ped, 0x558));
    f.fsub_m(k(K_HALF_PI));
    wr::<f32>(b, 0x6C, f.fstp());
    f.fld(d(S_HEADING));
    f.fcomp_m(k(K_M1000));
    if !f.jp(0x5) {
        g::<u32>(S_HEADING).write(rd::<u32>(b, 0x6C));
    } else {
        wr::<u32>(b, 0x6C, g::<u32>(S_HEADING).read());
    }
    if (rd::<u32>(wi, 0x18) >> 1) & 1 == 0 && rd::<i32>(wi, 0) != 0 {
        f.fld(d(S_HEADING));
        f.fadd_m(k(K_HALF_PI));
        wr::<f32>(b, 0x2C, f.fst());
        let hb = rd::<u32>(b, 0x2C);
        wr::<f32>(ped, 0x55C, f.fst());
        wr::<f32>(ped, 0x558, f.fstp());
        placeable_set_heading(ped, hb);
        update_rw_matrix(ped);
    }
    f.fld(rd(b, 0x6C));
    let flags = rd::<u32>(ped, 0x46C);
    f.fsub_m(rd(b, 0x10));
    f.fld(d(TIME_STEP));
    f.fmul_m(d(0x8CC4A8));
    wr::<u32>(b, 0x6C, 0);
    wr::<f32>(b, 0x14, f.fstp());
    let driver = |ped: GPtr<u8>| {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        !v.is_null() && rd::<GPtr<u8>>(v, 0x460) == ped
    };
    if flags & 0x100 != 0 && driver(ped) {
        f.fld(rd(b, 0x14));
        let dz = g::<u32>(0x8CC4B0).read();
        f.fmul_m(d(0x8CC4AC));
        wr::<u32>(b, 0x6C, dz);
        wr::<f32>(b, 0x14, f.fstp());
    }
    wrap_around(f, rd(beta, 0));
    f.fsub_m(rd(beta, 0));
    f.fld(rd(b, 0x6C));
    f.fcomp_m(k(K_0));
    if !f.jne(0x41) {
        dead_zone(f, b);
    }
    f.fld_st(0);
    f.fabs();
    f.fcomp_m(rd(b, 0x14));
    if !f.jp(0x5) {
        g::<u8>(S_AUTO_AIM).write(1);
    } else {
        f.fcomp_m(k(K_0));
        if f.jp(0x5) {
            f.fld(rd(b, 0x14));
        } else {
            f.fld(rd(beta, 0));
            f.fsub_m(rd(b, 0x14));
            wr::<f32>(beta, 0, f.fstp());
            return driver_alpha_or_step(this, b, ped, at, f);
        }
    }
    f.fadd_m(rd(beta, 0));
    wr::<f32>(beta, 0, f.fstp());
    driver_alpha_or_step(this, b, ped, at, f);
}

/// 0x5227CD: the driver's alpha toward `asin(clamp(vehicle forward.z)) + alphaDefault` (wrapped around
/// alpha, dead zone `B+0x6C`, rate `B+0x14`); anyone else adds the pad step `B+0x20`.
fn driver_alpha_or_step(this: GPtr<u8>, b: GPtr<u8>, ped: GPtr<u8>, at: usize, f: &mut Fpu) {
    let alpha = this.byte_add(0xAC);
    let v = rd::<GPtr<u8>>(ped, 0x58C);
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 || v.is_null() || rd::<GPtr<u8>>(v, 0x460) != ped {
        f.fld(rd(b, 0x20));
        f.fadd_m(rd(alpha, 0));
        wr::<f32>(alpha, 0, f.fstp());
        return;
    }
    let vm = rd::<GPtr<u8>>(v, 0x14);
    clamped_asin_of_mem(f, b, 0x28, vm.byte_add(0x18));
    f.fadd_m(d(at + 0xC));
    wrap_around(f, rd(alpha, 0));
    f.fsub_m(rd(alpha, 0));
    dead_zone(f, b);
    f.fld_st(0);
    f.fabs();
    f.fcomp_m(rd(b, 0x14));
    if f.jp(0x5) {
        f.fcomp_m(k(K_0));
        if !f.jp(0x5) {
            f.fld(rd(alpha, 0));
            f.fsub_m(rd(b, 0x14));
            wr::<f32>(alpha, 0, f.fstp());
            return;
        }
        f.fld(rd(b, 0x14));
    }
    f.fadd_m(rd(alpha, 0));
    wr::<f32>(alpha, 0, f.fstp());
}

/// 0x5225E3 to the end: `ClipBeta`, the alpha clamp, distance and front from alpha / beta, the source,
/// ducking, collision, the lock-on blend of front, the ped heading and the quick-aim pitch.
fn clip_and_output(this: GPtr<u8>, b: GPtr<u8>, ped: GPtr<u8>, wi: GPtr<u8>, at: usize, f: &mut Fpu) {
    let alpha = this.byte_add(0xAC);
    let beta = this.byte_add(0xBC);
    clip_beta(this);
    f.fld(rd(alpha, 0));
    f.fcomp_m(d(at + 0x14));
    if !f.jne(0x41) {
        wr::<u32>(alpha, 0, g::<u32>(at + 0x14).read());
    } else {
        f.fld(d(at + 0x18));
        f.fchs();
        f.fcom_m(rd(alpha, 0));
        if !f.jne(0x41) {
            wr::<f32>(alpha, 0, f.fstp());
        } else {
            f.fstp_st(0);
        }
    }
    // 0x522915: dist = cos(...) * distScale + baseDist
    f.fld(d(at));
    f.fld(rd(alpha, 0));
    f.fcomp_m(k(K_0));
    if !f.jne(0x41) {
        f.fld(d(at + 8));
        f.fmul_m(rd(alpha, 0));
        f.fld(k(K_HALF_PI));
        f.fcomp_st(1);
        if !f.jp(0x5) {
            f.fstp_st(0);
            f.fld(k(K_HALF_PI));
        }
        fcos(f);
    } else {
        f.fld(rd(alpha, 0));
        fcos(f);
    }
    f.fmul_m(d(at + 4));
    let front = this.byte_add(0x190);
    f.faddp(1);
    let src = this.byte_add(0x19C);
    // front from alpha / beta (esp = B-4 from the pushed 0 on: B+0x2C..0x34 are the temporaries)
    f.fld(rd(alpha, 0));
    fcos(f);
    f.fld(rd(beta, 0));
    fcos(f);
    f.fmul_st(1);
    f.fchs();
    wr::<f32>(b, 0x2C, f.fstp());
    wr::<u32>(front, 0, rd::<u32>(b, 0x2C));
    f.fld(rd(beta, 0));
    fsin(f);
    f.fmul_st(1);
    f.fchs();
    wr::<f32>(b, 0x30, f.fstp());
    wr::<u32>(front, 4, rd::<u32>(b, 0x30));
    f.fstp_st(0);
    f.fld(rd(alpha, 0));
    fsin(f);
    wr::<f32>(b, 0x34, f.fstp());
    f.fld_st(0);
    wr::<u32>(front, 8, rd::<u32>(b, 0x34));
    f.fmul_m(rd(front, 0));
    f.fld_st(1);
    f.fmul_m(rd(front, 4));
    wr::<f32>(b, 0x54, f.fstp());
    f.fxch(1);
    f.fmul_m(rd(front, 8));
    wr::<f32>(b, 0x58, f.fstp());
    f.fld(rd(b, 0x38));
    f.fsub_st(1);
    wr::<f32>(b, 0x2C, f.fstp());
    wr::<u32>(src, 0, rd::<u32>(b, 0x2C));
    f.fstp_st(0);
    f.fld(rd(b, 0x3C));
    f.fsub_m(rd(b, 0x54));
    wr::<f32>(b, 0x30, f.fstp());
    f.fld(rd(b, 0x40));
    wr::<u32>(src, 4, rd::<u32>(b, 0x30));
    f.fsub_m(rd(b, 0x58));
    wr::<f32>(b, 0x34, f.fstp());
    wr::<u32>(src, 8, rd::<u32>(b, 0x34));
    let tp = b.byte_add(0x38);
    handle_camera_motion_for_ducking_during_aim(the_camera(), ped, src, tp, 0);
    copy3(this.byte_add(0x148), tp);
    set_col_vars_aim_weapon(rd::<i32>(b, 0x18));
    if g::<i32>(G_CAMERA_DIRECTION).read() == 3 {
        camera_generic_mode_special_cases(the_camera(), ped);
        camera_ped_aim_mode_special_cases(the_camera(), ped);
        camera_col_det_and_react(src, tp);
        improve_near_clip(the_camera(), GPtr::NULL, ped, src, tp);
    }
    // 0x522A8A: the lock-on blend of front
    f.fld(d(S_C));
    g::<u8>(CAM_DIRECTLY_BEHIND).write(0);
    f.fcomp_m(k(K_0));
    g::<u8>(CAM_DIRECTLY_IN_FRONT).write(0);
    if !f.jne(0x41) && !rd::<GPtr<u8>>(ped, 0x71C).is_null() {
        f.fld(d(S_C));
        f.fmul_m(k(K_0_5));
        f.fld_st(0);
        f.fld(d(S_LOCK));
        f.fmul_st(1);
        wr::<f32>(b, 0x50, f.fstp());
        f.fld(d(S_LOCK + 4));
        f.fmul_st(1);
        wr::<f32>(b, 0x54, f.fstp());
        f.fmul_m(d(S_LOCK + 8));
        f.fld(k(K_1));
        f.fsub_st(2);
        f.fld(rd(b, 0x38));
        f.fmul_st(1);
        f.fld(rd(b, 0x3C));
        f.fmul_st(2);
        wr::<f32>(b, 0x48, f.fstp());
        f.fxch(1);
        f.fmul_m(rd(b, 0x40));
        wr::<f32>(b, 0x4C, f.fstp());
        f.fadd_m(rd(b, 0x50));
        wr::<f32>(b, 0x5C, f.fstp());
        f.fld(rd(b, 0x48));
        f.fadd_m(rd(b, 0x54));
        wr::<f32>(b, 0x60, f.fstp());
        f.fld(rd(b, 0x4C));
        f.fadd_st(1);
        f.fstp_st(2);
        f.fstp_st(0);
        f.fld(rd(b, 0x5C));
        f.fsub_m(rd(src, 0));
        wr::<f32>(b, 0x2C, f.fstp());
        f.fld(rd(b, 0x60));
        f.fsub_m(rd(src, 4));
        wr::<f32>(b, 0x30, f.fstp());
        f.fsub_m(rd(src, 8));
        wr::<u32>(front, 0, rd::<u32>(b, 0x2C));
        wr::<u32>(front, 4, rd::<u32>(b, 0x30));
        wr::<f32>(b, 0x34, f.fstp());
        wr::<u32>(front, 8, rd::<u32>(b, 0x34));
        crate::camproc::vector_normalise(front);
    }
    get_vectors_ready_for_rw(this);
    // 0x522B69: the ped heading and the quick-aim pitch
    let wflags = rd::<u32>(wi, 0x18);
    if (wflags >> 1) & 1 != 0 && rd::<u32>(ped, 0x46C) & 0x0400_0000 == 0 {
        wr::<u8>(this, 0xA, 0);
        return;
    }
    if rd::<i32>(wi, 0) == 0 || rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        wr::<u8>(this, 0xA, 0);
        return;
    }
    let from_front = |f: &mut Fpu| {
        f.fld(rd(front, 0));
        f.fchs();
        f.fld(rd(this, 0x194));
        fpatan(f);
        f.fsub_m(rd(b, 0x10));
    };
    let have = if rd::<i32>(active_weapon(ped), 0) == 0x29 {
        from_front(f);
        true
    } else {
        let o = rd::<GPtr<u8>>(ped, 0x71C);
        if !o.is_null() {
            let m = rd::<GPtr<u8>>(ped, 0x14);
            let pp = if m.is_null() {
                ped.byte_add(4)
            } else {
                m.byte_add(0x30)
            };
            let r = vector_sub(b.byte_add(0x5C), entity_pos(o), pp);
            f.fld(rd(r, 0));
            f.fchs();
            f.fld(rd(b, 0x60));
            fpatan(f);
            true
        } else if g::<u8>(S_AUTO_AIM).read() != 0 {
            from_front(f);
            true
        } else {
            false
        }
    };
    if have {
        wr::<f32>(b, 0x6C, f.fstp());
        f.fld(rd(b, 0x6C));
        f.fcomp_m(k(K_M100));
        if !f.jne(0x41) {
            f.fld(rd(b, 0x6C));
            let hb = rd::<u32>(b, 0x6C);
            f.fadd_m(k(K_M0_05));
            wr::<f32>(ped, 0x558, f.fst());
            wr::<f32>(ped, 0x55C, f.fstp());
            placeable_set_heading(ped, hb);
            update_rw_matrix(ped);
        }
    }
    let pitch = find_3rd_person_quick_aim_pitch(the_camera());
    let mut f2 = Fpu::new();
    push_st0(&mut f2, pitch);
    let pd = rd::<GPtr<u8>>(g::<GPtr<u8>>(CAMERA_TARGET).read(), 0x480);
    wr::<f32>(pd, 0x54, f2.fstp());
    wr::<u8>(this, 0xA, 0);
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_PROCESS_AIM_WEAPON,
    HOOK_FIND_3RD_PERSON_QUICK_AIM_PITCH_PORT,
    HOOK_FIND_3RD_PERSON_CAM_TARGET_VECTOR,
];

/// Globals of this module (none: the statics are accessed by address).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];

orig! {
    /// `CCam::Get_TwoPlayer_AimVector(CVector&)` (0x513E40)
    pub fn cam_get_two_player_aim_vector(cam: GPtr<u8>, out: GPtr<u8>) = thiscall 0x513E40 => unported;
}

/// The active cam's `tan(FOV * 0.5 * 0.017453292)` (FPTAN, the pushed 1.0 popped): `fld [cam+0xB4];
/// fmul 0.5; fmul deg2rad; fptan; fstp st(0)`.
fn tan_half_fov(f: &mut Fpu, cam: GPtr<u8>) {
    f.fld(rd(cam, 0xB4));
    f.fmul_m(k(K_0_5));
    f.fmul_m(k(K_DEG2RAD));
    f.fptan();
    f.fstp_st(0);
}

/// `CCamera::Find3rdPersonQuickAimPitch()` (0x50AD40, ST0): `-(atan(((0.5 - CHairMultY) * 2 * tan(FOV/2))
/// * (1 / aspect)) + alpha)` of the active cam. FPTAN (sa-math's 64-bit model) stays on the stack; the
/// FPATAN result (f64 model) is rounded by the following `fadd`, so the returned ST0 holds 24 bits.
#[hook(addr = 0x50AD40, conv = thiscall, name = "CCamera::Find3rdPersonQuickAimPitch", expect = "0F B6 41 59 69 C0 38 02 00 00", this = "CCamera")]
pub fn find_3rd_person_quick_aim_pitch_port(this: GPtr<u8>) -> sa_math::x87::X87 {
    let cam = crate::common::active_cam(this);
    let mut f = Fpu::new();
    tan_half_fov(&mut f, cam);
    f.fld(k(K_0_5));
    f.fsub_m(d(CHAIR_MULT_Y));
    f.fadd_st(0);
    f.fmulp(1);
    f.fld(k(K_1));
    f.fdiv_m(d(ASPECT_RATIO));
    f.fmulp(1);
    f.fld1();
    fpatan(&mut f);
    f.fadd_m(rd(cam, 0xAC));
    f.fchs();
    f.pop()
}

/// `CCamera::Find3rdPersonCamTargetVector(float range, CVector muzzle, CVector& outSource,
/// CVector& outTarget)` (0x514970, `ret 0x18`, AL = 1). The entry `jmp 0x408345` (`.text` protection
/// chunk) does the `imul eax, eax, 0x238` of the active cam index. outSource = cam source;
/// outTarget = dir = cam front (mode 0x31: `Get_TwoPlayer_AimVector`), else `front + up * (t * ((0.5 -
/// CHairMultY) * 2 * (1 / aspect))) + (front x up) * (((CHairMultX - 0.5) * 2) * t)` normalised (each
/// sum stored per component; `t = tan(FOV/2)` by FPTAN); then `p = ((m - s).x*d.x + (m - s).z*d.z) +
/// (m - s).y*d.y`, `outSource += d * p` (z through the stored f32 of the argument slot), `outTarget =
/// outSource + d * range`. EAX keeps the upper bytes of the last dword moved (outTarget.x's bits).
#[hook(addr = 0x514970, conv = thiscall, name = "CCamera::Find3rdPersonCamTargetVector", expect = "83 EC 10 0F B6 41 59 90", this = "CCamera")]
pub fn find_3rd_person_cam_target_vector(
    this: GPtr<u8>,
    range: u32,
    mx: u32,
    my: u32,
    mz: u32,
    out_source: GPtr<u8>,
    out_target: GPtr<u8>,
) -> u32 {
    // frame: B+0x0C x-scale, B+0x10..0x18 cross product, B+0x18 temp, B+0x20.. the arguments
    let mut fr = [0u32; 0xE];
    let b = GPtr::<u32>::from_raw(fr.as_mut_ptr()).cast::<u8>();
    wr::<u32>(b, 0x20, range);
    wr::<u32>(b, 0x24, mx);
    wr::<u32>(b, 0x28, my);
    wr::<u32>(b, 0x2C, mz);
    let mut f = Fpu::new();
    let f = &mut f;
    let cam = crate::common::active_cam(this);
    tan_half_fov(f, cam);
    f.fld(d(CHAIR_MULT_X));
    f.fsub_m(k(K_0_5));
    f.fadd_st(0);
    f.fmul_st(1);
    wr::<f32>(b, 0xC, f.fstp());
    f.fld(k(K_0_5));
    f.fsub_m(d(CHAIR_MULT_Y));
    f.fadd_st(0);
    f.fld(k(K_1));
    f.fdiv_m(d(ASPECT_RATIO));
    let src = out_source;
    let dir = out_target;
    copy3(src, cam.byte_add(0x19C));
    let cam = crate::common::active_cam(this);
    copy3(dir, cam.byte_add(0x190));
    let cam = crate::common::active_cam(this);
    f.fmulp(1);
    f.fmulp(1);
    if rd::<i16>(cam, 0xC) == 0x31 {
        f.fstp_st(0);
        cam_get_two_player_aim_vector(cam, dir);
    } else {
        f.fld_st(0);
        f.fmul_m(rd(cam, 0x1B4));
        f.fld_st(1);
        f.fmul_m(rd(cam, 0x1B8));
        f.fxch(2);
        f.fmul_m(rd(cam, 0x1BC));
        wr::<f32>(b, 0x18, f.fstp());
        f.fadd_m(rd(dir, 0));
        wr::<f32>(dir, 0, f.fstp());
        f.fadd_m(rd(dir, 4));
        wr::<f32>(dir, 4, f.fstp());
        f.fld(rd(b, 0x18));
        f.fadd_m(rd(dir, 8));
        wr::<f32>(dir, 8, f.fstp());
        let cam = crate::common::active_cam(this);
        let r = cross_product(b.byte_add(0x10), cam.byte_add(0x190), cam.byte_add(0x1B4));
        f.fld(rd(b, 0xC));
        f.fmul_m(rd(r, 0));
        f.fld(rd(b, 0xC));
        f.fmul_m(rd(r, 4));
        f.fld(rd(b, 0xC));
        f.fmul_m(rd(r, 8));
        wr::<f32>(b, 0x18, f.fstp());
        f.fxch(1);
        f.fadd_m(rd(dir, 0));
        wr::<f32>(dir, 0, f.fstp());
        f.fadd_m(rd(dir, 4));
        wr::<f32>(dir, 4, f.fstp());
        f.fld(rd(b, 0x18));
        f.fadd_m(rd(dir, 8));
        wr::<f32>(dir, 8, f.fstp());
        crate::camproc::vector_normalise(dir);
    }
    // 0x514AC3: project the muzzle onto the aim line
    f.fld(rd(b, 0x24));
    f.fsub_m(rd(src, 0));
    f.fld(rd(b, 0x28));
    f.fsub_m(rd(src, 4));
    f.fld(rd(b, 0x2C));
    f.fsub_m(rd(src, 8));
    f.fxch(2);
    f.fmul_m(rd(dir, 0));
    f.fld_st(2);
    f.fmul_m(rd(dir, 8));
    f.faddp(1);
    f.fxch(1);
    f.fmul_m(rd(dir, 4));
    f.faddp(1);
    f.fxch(1);
    f.fstp_st(0);
    f.fld_st(0);
    f.fmul_m(rd(dir, 0));
    f.fld_st(1);
    f.fmul_m(rd(dir, 4));
    wr::<f32>(b, 0x28, f.fstp());
    f.fxch(1);
    f.fmul_m(rd(dir, 8));
    wr::<f32>(b, 0x2C, f.fstp());
    f.fadd_m(rd(src, 0));
    wr::<f32>(src, 0, f.fst());
    f.fld(rd(b, 0x28));
    f.fadd_m(rd(src, 4));
    wr::<f32>(src, 4, f.fst());
    f.fld(rd(b, 0x2C));
    f.fadd_m(rd(src, 8));
    wr::<f32>(b, 0x30, f.fst());
    wr::<f32>(src, 8, f.fstp());
    // pop edi: [esp+X] = B+X+4 from here
    f.fld(rd(b, 0x20));
    f.fmul_m(rd(dir, 0));
    f.fld(rd(b, 0x20));
    f.fmul_m(rd(dir, 4));
    wr::<f32>(b, 0x14, f.fstp());
    f.fld(rd(b, 0x20));
    f.fmul_m(rd(dir, 8));
    wr::<f32>(b, 0x18, f.fstp());
    f.fxch(2);
    f.fadd_st(2);
    wr::<f32>(b, 0x24, f.fstp());
    let xb = rd::<u32>(b, 0x24);
    wr::<u32>(dir, 0, xb);
    f.fstp_st(1);
    f.fadd_m(rd(b, 0x14));
    wr::<f32>(b, 0x28, f.fstp());
    f.fld(rd(b, 0x30));
    f.fadd_m(rd(b, 0x18));
    wr::<u32>(dir, 4, rd::<u32>(b, 0x28));
    wr::<f32>(b, 0x2C, f.fstp());
    wr::<u32>(dir, 8, rd::<u32>(b, 0x2C));
    (xb & !0xFF) | 1
}
