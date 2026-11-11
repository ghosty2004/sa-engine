//! PORT-CAM-3 (helper A): `CCam::Process_FollowPedWithMouse` (0x50F970), the on-foot camera with the
//! mouse (`CCam::m_bUseMouse3rdPerson`, the PC default). Ported from the 1.0 US bytes (gta-reversed: not
//! reversed). Same technique as [`crate::followped`]: a frame array at the original's `B`-relative
//! offsets (B = ESP after the four register pushes; B+0x68 is the CColPoint of the line-of-sight tests)
//! and the [`Fpu`] machine. FPTAN uses sa-math's 64-bit model; FPATAN / FSIN / FCOS the f64 models.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::platform::SCENE_CAMERA;

use crate::avoidgeom::process_line_of_sight;
use crate::camcol::{set_near_clip_raw, vector_magnitude};
use crate::cammodes::test_sphere_against_world;
use crate::camproc::{find_player_vehicle, get_pad, get_vectors_ready_for_rw, vector_normalise};
use crate::common::{Fpu, g, k, rd, wr};
use crate::followped::{ci_pow, clip_beta, copy3, d, fcos, fpatan, fsin, pad_clear_mouse_history, push_st0};
use crate::pedonpc::{matrix_set_rotate_z_only, pad_540e80, pad_540f80};
use crate::rocket::{matrix_update_rw_matrix, simple_transform_update_rw_matrix};

const IGNORE_ENTITY: usize = 0xB7CD68;
const TEMP_SPHERE_COL_POINT: usize = 0xB9B250;
const TIME_STEP: usize = 0xB7CB5C;
const ASPECT_RATIO: usize = 0xC3EFA4;
const MOUSE_ACCEL_VERTICAL: usize = 0xB6EC18;
const MOUSE_ACCEL_HORZNTL: usize = 0xB6EC1C;
const MOUSE_X: usize = 0xB73424;
const MOUSE_Y: usize = 0xB73428;
const CAMERA_TARGET: usize = 0xB6F980;
/// `TheCamera` fields: `m_bFading` (+0x51), the fading direction (+0xC30, i16), +0x37 (0xB6F05F, keep
/// beta at +0x9C), `m_bCamDirectlyBehind` / `InFront`, the target orientation (+0x120, 0xB6F148).
const FADING: usize = 0xB6F079;
const FADING_DIRECTION: usize = 0xB6FC58;
const CAMERA_37: usize = 0xB6F05F;
const CAM_DIRECTLY_BEHIND: usize = 0xB6F042;
const CAM_DIRECTLY_IN_FRONT: usize = 0xB6F043;
const CAMERA_ORIENTATION: usize = 0xB6F148;
/// `CDraw::FadeValue` (u8 0xC3EFAB) and an unused byte read (0xBA6793).
const FADE_VALUE: usize = 0xC3EFAB;
const UNUSED_BYTE: usize = 0xBA6793;
/// `.data` tweak values: base distance 1.7 (0x8CC7C0), distance scale 2.0 (0x8CC7C4), alpha distance
/// factor 3.0 (0x8CC7C8), pad scale 0.01 (0x8CC7CC), target height 0.8 (0x8CC7D0), fade threshold 45
/// (u16 0x8CC7D4), the auto pitch -0.22 (0x8CC7D8), the near-clip margin 0.5 (0x8CC38C).
const BASE_DIST: usize = 0x8CC7C0;
const DIST_SCALE: usize = 0x8CC7C4;
const ALPHA_DIST: usize = 0x8CC7C8;
const PAD_SCALE: usize = 0x8CC7CC;
const TARGET_HEIGHT: usize = 0x8CC7D0;
const FADE_THRESHOLD: usize = 0x8CC7D4;
const AUTO_PITCH: usize = 0x8CC7D8;
const NEAR_MARGIN: usize = 0x8CC38C;

const K_1: u32 = 0x3F80_0000; // [0x858624]
const K_0_1: u32 = 0x3DCC_CCCD; // [0x858B1C]
const K_0: u32 = 0; // [0x858B50]
const K_0_5: u32 = 0x3F00_0000; // [0x858B8C]
const K_4: u32 = 0x4080_0000; // [0x858B90]
const K_0_9: u32 = 0x3F66_6666; // [0x858C20]
const K_0_3: u32 = 0x3E99_999A; // [0x858C24]
const K_0_05: u32 = 0x3D4C_CCCD; // [0x858C28]
const K_M0_9: u32 = 0xBF66_6666; // [0x858CAC]
const K_PI: u32 = 0x4049_0FDB; // [0x858CB8]
const K_0_6: u32 = 0x3F19_999A; // [0x858CC8]
const K_0_4: u32 = 0x3ECC_CCCD; // [0x858EE8]
const K_0_35: u32 = 0x3EB3_3333; // [0x858F9C]
const K_HALF_PI: u32 = 0x3FC9_0FDB; // [0x858FE4]
const K_M0_05: u32 = 0xBD4C_CCCD; // [0x85901C]
const K_DEG2RAD: u32 = 0x3C8E_FA35; // [0x8595EC]
const K_QUARTER_PI: u32 = 0x3F49_0FDB; // [0x859AB0]
const K_1_14: u32 = 0x3D92_4925; // [0x859B50]
const K_0_0125: u32 = 0x3C4C_CCCD; // [0x85A5C4]
const K_1_1: u32 = 0x3F8C_CCCD; // [0x862F10]
const K_0_92: u32 = 0x3F6B_851F; // [0x862F14]
const K_ALPHA_MIN: u32 = 0xBFC7_F1E7; // [0x8630F4]
const K_3_70: u32 = 0x3D2F_8AF9; // [0x8631AC]
const K_M2_5: u32 = 0xC020_0000; // [0x8631B0]

/// `(a - b)` per component, `(dz*dz + dy*dy) + dx*dx` in the bytes' register order, `fsqrt`; leaves
/// the length in ST0.
fn dist_zyx(f: &mut Fpu, a: GPtr<u8>, b: GPtr<u8>) {
    f.fld(rd(a, 0));
    f.fsub_m(rd(b, 0));
    f.fld(rd(a, 4));
    f.fsub_m(rd(b, 4));
    f.fld(rd(a, 8));
    f.fsub_m(rd(b, 8));
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
}

/// `CCam::Process_FollowPedWithMouse(const CVector& target, float orientation, float, float)` (0x50F970,
/// thiscall, `ret 0x10`). Order of calls, re-reads and x87 operations as in the bytes:
/// - FOV 70; non-ped targets return; reset: rotating 0, collision checks on, the pad's mouse history
///   cleared, m_bResetStatics = 0; a train vehicle sets the "follow orientation" flag;
/// - T = target + (0, 0, 0.8); with pad byte +0x10E bit 0x20: beta steps to the source's direction (or
///   orientation + pi when looking down), else the mouse (`(mx * -2.5) * accelH * FOV*0.0125`,
///   `(my * 4) * FOV*0.0125 * accelV`) or the look sticks (scaled by ts and 0.01);
/// - after a fade (or bit 0x20) alpha steps (0.05) toward -0.22; ClipBeta; alpha clamped to
///   [-1.5620697, pi/4]; `dist = cos(min(3 * alpha, pi/2) or alpha) * 2 + 1.7`; beta overrides (+0x37,
///   directly behind / in front, train);
/// - source = T - front * dist; fudge = T; a line of sight T -> source moves it to the hit (a ped hit far
///   enough away tests again from the hit point; near clips 0.05 .. 0.9 from the distances);
/// - up to six spheres (`near * tan(FOV/2) * aspect * 1.1`) in front of the source pull the near clip
///   (and the source toward T when it reaches 0.1); `m_fDistance` (+0xC4) follows the distance (snap
///   when nearer, else `pow(0.92, ts)` smoothing that also places the source), the near clip from it;
/// - GetVectorsReadyForRW; while fading in (direction 1, value > 0x80) the target faces the camera.
#[hook(addr = 0x50F970, conv = thiscall, name = "CCam::Process_FollowPedWithMouse", expect = "81 EC B4 00 00 00", this = "CCam")]
pub fn process_follow_ped_with_mouse(
    this: GPtr<u8>,
    target: GPtr<u8>,
    orientation: u32,
    _sv: u32,
    _svw: u32,
) {
    let mut fr = [0u32; 0x34];
    let b = GPtr::<u32>::from_raw(fr.as_mut_ptr()).cast::<u8>();
    wr::<u32>(b, 0xCC, orientation);
    let mut fpu = Fpu::new();
    let f = &mut fpu;
    let t = rd::<GPtr<u8>>(this, 0x21C);
    wr::<u32>(this, 0xB4, 0x428C_0000);
    wr::<u8>(b, 0x1B, 0);
    if rd::<u8>(t, 0x36) & 7 != 3 {
        return;
    }
    if rd::<u8>(this, 0xA) != 0 {
        wr::<u8>(this, 0xB, 0);
        wr::<u8>(this, 0x3, 1);
        pad_clear_mouse_history(get_pad(0));
        wr::<u8>(this, 0xA, 0);
    }
    if !find_player_vehicle(-1, 0).is_null() && rd::<i32>(find_player_vehicle(-1, 0), 0x590) == 6 {
        wr::<u8>(b, 0x1B, 1);
    }
    // T (B+0x28..0x30)
    wr::<u32>(b, 0x30, rd::<u32>(target, 8));
    f.fld(d(TARGET_HEIGHT));
    f.fadd_m(rd(b, 0x30));
    wr::<u32>(b, 0x28, rd::<u32>(target, 0));
    wr::<f32>(b, 0x30, f.fstp());
    wr::<u32>(b, 0x2C, rd::<u32>(target, 4));
    let src = this.byte_add(0x19C);
    let bit20 = || rd::<u8>(get_pad(0), 0x10E) & 0x20 != 0;
    if bit20() {
        for i in 0..3 {
            f.fld(rd(src, 4 * i));
            f.fsub_m(rd(b, 0x28 + 4 * i));
            wr::<f32>(b, 0x1C + 4 * i, f.fstp());
        }
        vector_normalise(b.byte_add(0x1C));
        f.fld(rd(b, 0x24));
        f.fcomp_m(k(K_M0_9));
        if !f.jp(0x5) {
            f.fld(rd(b, 0xCC));
            f.fadd_m(k(K_PI));
        } else {
            f.fld(rd(b, 0x20));
            f.fld(rd(b, 0x1C));
            fpatan(f);
        }
        f.fsub_m(rd(this, 0xBC));
        wr::<u32>(b, 0x10, 0);
        wr::<f32>(b, 0x14, f.fstp());
    } else {
        let _ = get_pad(0);
        wr::<u32>(b, 0x1C, g::<u32>(MOUSE_X).read());
        f.fld(rd(b, 0x1C));
        f.fcomp_m(k(K_0));
        wr::<u32>(b, 0x20, g::<u32>(MOUSE_Y).read());
        let moved = f.jp(0x44) || {
            f.fld(rd(b, 0x20));
            f.fcomp_m(k(K_0));
            f.jp(0x44)
        };
        if moved && rd::<i16>(get_pad(0), 0x10E) == 0 {
            f.fld(rd(b, 0x1C));
            f.fmul_m(k(K_M2_5));
            f.fld(rd(b, 0x20));
            f.fmul_m(k(K_4));
            f.fld(rd(this, 0xB4));
            f.fmul_m(k(K_0_0125));
            f.fld(d(MOUSE_ACCEL_HORZNTL));
            f.fmul_st(1);
            f.fmul_st(3);
            wr::<f32>(b, 0x14, f.fstp());
            f.fmul_st(1);
            f.fmul_m(d(MOUSE_ACCEL_VERTICAL));
            wr::<f32>(b, 0x10, f.fstp());
            f.fstp_st(0);
        } else {
            let lr = i32::from(pad_540e80(get_pad(0))).wrapping_neg();
            wr::<i32>(b, 0x10, lr);
            f.fild(rd(b, 0x10));
            wr::<f32>(b, 0x14, f.fstp());
            let ud = i32::from(pad_540f80(get_pad(0)));
            wr::<i32>(b, 0x10, ud);
            f.fild(rd(b, 0x10));
            f.fld(rd(this, 0xB4));
            f.fmul_m(k(K_0_0125));
            f.fld(k(K_1_14));
            f.fmul_st(1);
            f.fmul_m(d(TIME_STEP));
            f.fmul_m(d(PAD_SCALE));
            f.fmul_m(rd(b, 0x14));
            wr::<f32>(b, 0x14, f.fstp());
            f.fmul_m(k(K_3_70));
            f.fmul_m(d(TIME_STEP));
            f.fmul_st(1);
            f.fmul_m(d(PAD_SCALE));
            wr::<f32>(b, 0x10, f.fstp());
        }
        f.fstp_st(0);
    }
    // 0x50FBB4: the automatic pitch after a fade
    let fading = g::<u8>(FADING).read();
    let fade = g::<u8>(FADE_VALUE).read();
    let auto = (fading != 0
        && g::<i16>(FADING_DIRECTION).read() == 1
        && u16::from(fade) > g::<u16>(FADE_THRESHOLD).read())
        || fade > 0xC8
        || bit20();
    let alpha = this.byte_add(0xAC);
    let beta = this.byte_add(0xBC);
    if auto {
        let p = d(AUTO_PITCH);
        f.fld(p);
        f.fsub_m(k(K_0_05));
        f.fcomp_m(rd(alpha, 0));
        if !f.jne(0x41) {
            f.fld(k(K_0_05));
        } else {
            f.fld(rd(alpha, 0));
            f.fcomp_m(d(AUTO_PITCH));
            f.fld(d(AUTO_PITCH));
            if !f.jp(0x5) {
                f.fsub_m(rd(alpha, 0));
            } else {
                f.fadd_m(k(K_0_05));
                f.fcomp_m(rd(alpha, 0));
                if !f.jp(0x5) {
                    f.fld(k(K_M0_05));
                } else {
                    f.fld(rd(alpha, 0));
                    f.fcomp_m(d(AUTO_PITCH));
                    if !f.jne(0x41) {
                        f.fld(d(AUTO_PITCH));
                        f.fsub_m(rd(alpha, 0));
                    } else {
                        f.fld(k(K_0));
                    }
                }
            }
        }
    } else {
        f.fld(rd(b, 0x10));
    }
    f.fld(rd(b, 0x14));
    f.fadd_m(rd(beta, 0));
    wr::<f32>(beta, 0, f.fstp());
    f.fadd_m(rd(alpha, 0));
    wr::<f32>(alpha, 0, f.fstp());
    clip_beta(this);
    f.fld(rd(alpha, 0));
    f.fcomp_m(k(K_QUARTER_PI));
    if !f.jne(0x41) {
        wr::<u32>(alpha, 0, K_QUARTER_PI);
    } else {
        f.fld(rd(alpha, 0));
        f.fcomp_m(k(K_ALPHA_MIN));
        if !f.jp(0x5) {
            wr::<u32>(alpha, 0, K_ALPHA_MIN);
        }
    }
    // 0x50FCDE: distance
    f.fld(d(BASE_DIST));
    f.fld(rd(alpha, 0));
    f.fcomp_m(k(K_0));
    if !f.jne(0x41) {
        f.fld(d(ALPHA_DIST));
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
    let keep = g::<u8>(CAMERA_37).read();
    f.fmul_m(d(DIST_SCALE));
    f.fadd_st(1);
    wr::<f32>(b, 0x10, f.fstp());
    f.fstp_st(0);
    if keep == 1 {
        wr::<u32>(beta, 0, rd::<u32>(this, 0x9C));
    }
    if g::<u8>(CAM_DIRECTLY_BEHIND).read() == 1 {
        f.fld(d(CAMERA_ORIENTATION));
        f.fadd_m(k(K_PI));
        wr::<f32>(beta, 0, f.fstp());
    }
    if g::<u8>(CAM_DIRECTLY_IN_FRONT).read() == 1 {
        wr::<u32>(beta, 0, g::<u32>(CAMERA_ORIENTATION).read());
    }
    if rd::<u8>(b, 0x1B) != 0 {
        wr::<u32>(beta, 0, rd::<u32>(b, 0xCC));
    }
    source_and_collision(this, b, f);
    sphere_near_clip(this, b, f);
    distance_smoothing(this, b, f);
    // 0x5104F6
    g::<u8>(CAM_DIRECTLY_BEHIND).write(0);
    g::<u8>(CAM_DIRECTLY_IN_FRONT).write(0);
    get_vectors_ready_for_rw(this);
    if g::<u8>(FADING).read() == 0
        || g::<i16>(FADING_DIRECTION).read() != 1
        || g::<u8>(FADE_VALUE).read() <= 0x80
    {
        return;
    }
    let front = this.byte_add(0x190);
    f.fld(rd(front, 0));
    let te = g::<GPtr<u8>>(CAMERA_TARGET).read();
    f.fchs();
    f.fld(rd(this, 0x194));
    fpatan(f);
    wr::<f32>(b, 0x10, f.fst());
    wr::<f32>(te, 0x558, f.fstp());
    let te = g::<GPtr<u8>>(CAMERA_TARGET).read();
    f.fld(rd(b, 0x10));
    wr::<f32>(te, 0x55C, f.fstp());
    let te = g::<GPtr<u8>>(CAMERA_TARGET).read();
    let m = rd::<GPtr<u8>>(te, 0x14);
    if !m.is_null() {
        matrix_set_rotate_z_only(m, rd::<u32>(b, 0x10));
    } else {
        wr::<u32>(te, 0x10, rd::<u32>(b, 0x10));
    }
    let te = g::<GPtr<u8>>(CAMERA_TARGET).read();
    let rw = rd::<GPtr<u8>>(te, 0x18);
    if !rw.is_null() {
        let rwm = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let m = rd::<GPtr<u8>>(te, 0x14);
        if !m.is_null() {
            matrix_update_rw_matrix(m, rwm);
        } else {
            simple_transform_update_rw_matrix(te.byte_add(4), rwm);
        }
    }
}

/// 0x50FD8D: front from alpha / beta, source = T - front * dist (B+0x10), fudge = T, the line of sight
/// T -> source (CColPoint B+0x68, entity B+0x40) and the near clip from the hit distances.
fn source_and_collision(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu) {
    let front = this.byte_add(0x190);
    let src = this.byte_add(0x19C);
    f.fld(rd(this, 0xAC));
    fcos(f);
    wr::<u32>(b, 0x40, 0);
    f.fld(rd(this, 0xBC));
    fcos(f);
    f.fmul_st(1);
    f.fchs();
    wr::<f32>(b, 0x1C, f.fstp());
    wr::<u32>(front, 0, rd::<u32>(b, 0x1C));
    f.fld(rd(this, 0xBC));
    fsin(f);
    f.fmul_st(1);
    f.fchs();
    wr::<f32>(b, 0x20, f.fstp());
    wr::<u32>(front, 4, rd::<u32>(b, 0x20));
    f.fstp_st(0);
    f.fld(rd(this, 0xAC));
    fsin(f);
    wr::<f32>(b, 0x24, f.fstp());
    f.fld(rd(b, 0x10));
    wr::<u32>(front, 8, rd::<u32>(b, 0x24));
    f.fmul_m(rd(front, 0));
    f.fld(rd(b, 0x10));
    f.fmul_m(rd(front, 4));
    f.fld(rd(b, 0x10));
    f.fmul_m(rd(front, 8));
    wr::<f32>(b, 0x58, f.fstp());
    f.fld(rd(b, 0x28));
    f.fsub_st(2);
    wr::<f32>(b, 0x1C, f.fstp());
    wr::<u32>(src, 0, rd::<u32>(b, 0x1C));
    f.fld(rd(b, 0x2C));
    f.fsub_st(1);
    wr::<f32>(b, 0x20, f.fstp());
    f.fstp_st(0);
    wr::<u32>(src, 4, rd::<u32>(b, 0x20));
    f.fstp_st(0);
    f.fld(rd(b, 0x30));
    f.fsub_m(rd(b, 0x58));
    wr::<f32>(b, 0x24, f.fstp());
    wr::<u32>(src, 8, rd::<u32>(b, 0x24));
    copy3(this.byte_add(0x148), b.byte_add(0x28));
    g::<GPtr<u8>>(IGNORE_ENTITY).write(rd::<GPtr<u8>>(this, 0x21C));
    let cp = b.byte_add(0x68);
    let hit_ent = b.byte_add(0x40);
    if process_line_of_sight(b.byte_add(0x28), src, cp, hit_ent, 1, 1, 1, 1, 0, 0, 1, 0) == 0 {
        g::<u32>(IGNORE_ENTITY).write(0);
        return;
    }
    let ent = rd::<GPtr<u8>>(b, 0x40);
    dist_zyx(f, b.byte_add(0x28), cp);
    wr::<f32>(b, 0x14, f.fstp());
    f.fld(rd(b, 0x10));
    f.fsub_m(rd(b, 0x14));
    wr::<f32>(b, 0x10, f.fstp());
    let far_ped = rd::<u8>(ent, 0x36) & 7 == 3 && {
        f.fld(rd(b, 0x10));
        f.fcomp_m(k(K_0_4));
        !f.jne(0x41)
    };
    let near: Option<u32>;
    if far_ped {
        if process_line_of_sight(cp, src, cp, hit_ent, 1, 1, 1, 1, 0, 0, 1, 0) != 0 {
            for i in 0..3 {
                f.fld(rd(b, 0x28 + 4 * i));
                f.fsub_m(rd(cp, 4 * i));
                wr::<f32>(b, 0x34 + 4 * i, f.fstp());
            }
            push_st0(f, vector_magnitude(b.byte_add(0x34)));
            f.fcom_m(k(K_0_6));
            copy3(src, cp);
            if f.jp(0x5) {
                f.fstp_st(0);
                near = None;
            } else {
                f.fsub_m(k(K_0_3));
                f.fld(k(K_0_05));
                f.fcomp_st(1);
                if !f.jne(0x41) {
                    wr::<u32>(b, 0x10, K_0_05);
                    f.fstp_st(0);
                } else {
                    wr::<f32>(b, 0x10, f.fstp());
                }
                near = Some(rd::<u32>(b, 0x10));
            }
        } else {
            f.fld(rd(b, 0x10));
            f.fsub_m(k(K_0_35));
            f.fld(k(K_0_9));
            f.fcomp_st(1);
            if !f.jp(0x5) {
                f.fstp_st(0);
                wr::<u32>(b, 0x10, K_0_9);
            } else {
                wr::<f32>(b, 0x10, f.fstp());
            }
            near = Some(rd::<u32>(b, 0x10));
        }
    } else {
        copy3(src, cp);
        f.fld(rd(b, 0x14));
        f.fcomp_m(k(K_0_6));
        if f.jp(0x5) {
            near = None;
        } else {
            f.fld(rd(b, 0x14));
            f.fsub_m(k(K_0_3));
            f.fld(k(K_0_05));
            f.fcomp_st(1);
            if !f.jne(0x41) {
                f.fstp_st(0);
                wr::<u32>(b, 0x10, K_0_05);
            } else {
                wr::<f32>(b, 0x10, f.fstp());
            }
            near = Some(rd::<u32>(b, 0x10));
        }
    }
    if let Some(n) = near {
        set_near_clip_raw(SCENE_CAMERA.get(), n);
    }
    g::<u32>(IGNORE_ENTITY).write(0);
}

/// 0x510086: the first sphere (`r = near * ((aspect * tan(FOV/2)) * 1.1)`) and, while it touches
/// something, up to five more (`r = ((tan * aspect) * near) * 1.1`) after pulling the near clip from the
/// contact's distance off the view axis (`|c - (c.f) f| / k`).
fn sphere_near_clip(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu) {
    let front = this.byte_add(0x190);
    let src = this.byte_add(0x19C);
    f.fld(rd(this, 0xB4));
    f.fmul_m(k(K_DEG2RAD));
    let _ = g::<u8>(UNUSED_BYTE).read();
    let cam = SCENE_CAMERA.get();
    f.fmul_m(k(K_0_5));
    f.fptan();
    f.fstp_st(0);
    f.fld(d(ASPECT_RATIO));
    f.fmul_st(1);
    f.fmul_m(k(K_1_1));
    wr::<f32>(b, 0x10, f.fstp());
    f.fstp_st(0);
    f.fld(rd(cam, 0x80));
    wr::<f32>(b, 0x14, f.fst());
    f.fmul_m(rd(front, 0));
    f.fld(rd(b, 0x14));
    f.fmul_m(rd(front, 4));
    f.fld(rd(b, 0x14));
    f.fmul_m(rd(front, 8));
    wr::<f32>(b, 0x58, f.fstp());
    f.fxch(1);
    f.fadd_m(rd(src, 0));
    wr::<f32>(b, 0x1C, f.fstp());
    f.fadd_m(rd(src, 4));
    wr::<f32>(b, 0x20, f.fstp());
    f.fld(rd(b, 0x58));
    f.fadd_m(rd(src, 8));
    wr::<f32>(b, 0x24, f.fstp());
    f.fld(rd(b, 0x14));
    f.fmul_m(rd(b, 0x10));
    let r = f.fstp().to_bits();
    let mut hit = test_sphere_against_world(
        rd::<u32>(b, 0x1C),
        rd::<u32>(b, 0x20),
        rd::<u32>(b, 0x24),
        r,
        GPtr::NULL,
        1,
        1,
        0,
        1,
        0,
        0,
    );
    if hit.is_null() {
        return;
    }
    f.fld(k(K_1));
    f.fdiv_m(rd(b, 0x10));
    wr::<f32>(b, 0x1C, f.fstp());
    let mut n = 0;
    let cpp = |i: usize| d(TEMP_SPHERE_COL_POINT + 4 * i);
    loop {
        // 0x510150: the contact's distance off the view axis
        f.fld(cpp(0));
        f.fsub_m(rd(src, 0));
        f.fld(cpp(1));
        f.fsub_m(rd(src, 4));
        f.fld(cpp(2));
        f.fsub_m(rd(src, 8));
        wr::<f32>(b, 0xB4, f.fst());
        f.fmul_m(rd(front, 8));
        f.fld_st(1);
        f.fmul_m(rd(front, 4));
        f.faddp(1);
        f.fld_st(2);
        f.fmul_m(rd(front, 0));
        f.faddp(1);
        f.fld_st(0);
        f.fmul_m(rd(front, 0));
        f.fld_st(1);
        f.fmul_m(rd(front, 4));
        wr::<f32>(b, 0x48, f.fstp());
        f.fxch(1);
        f.fmul_m(rd(front, 8));
        wr::<f32>(b, 0x4C, f.fstp());
        f.fxch(2);
        f.fsub_st(2);
        wr::<f32>(b, 0x50, f.fstp());
        f.fstp_st(1);
        f.fsub_m(rd(b, 0x48));
        f.fld(rd(b, 0xB4));
        f.fsub_m(rd(b, 0x4C));
        f.fld_st(0);
        f.fmul_st(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fld(rd(b, 0x50));
        f.fmul_m(rd(b, 0x50));
        f.faddp(1);
        f.fsqrt();
        f.fstp_st(2);
        f.fstp_st(0);
        f.fmul_m(rd(b, 0x1C));
        wr::<f32>(b, 0x10, f.fstp());
        // near = clamp to [0.1, current]
        f.fld(rd(b, 0x14));
        f.fcomp_m(rd(b, 0x10));
        if f.jp(0x5) {
            f.fld(rd(b, 0x10));
        } else {
            f.fld(rd(b, 0x14));
        }
        f.fld(k(K_0_1));
        f.fcomp_st(1);
        f.fstp_st(0);
        if !f.jne(0x41) {
            wr::<u32>(b, 0x10, K_0_1);
        } else {
            f.fld(rd(b, 0x14));
            f.fcomp_m(rd(b, 0x10));
            if !f.jp(0x5) {
                wr::<u32>(b, 0x10, rd::<u32>(b, 0x14));
            }
        }
        f.fld(rd(b, 0x10));
        f.fcomp_m(rd(b, 0x14));
        if !f.jp(0x5) {
            set_near_clip_raw(SCENE_CAMERA.get(), rd::<u32>(b, 0x10));
        }
        f.fld(rd(b, 0x10));
        f.fcomp_m(k(K_0_1));
        if !f.jp(0x44) {
            // at the minimum: the source moves 30% toward T
            f.fld(rd(b, 0x28));
            f.fsub_m(rd(src, 0));
            f.fld(rd(b, 0x2C));
            f.fsub_m(rd(src, 4));
            f.fld(rd(b, 0x30));
            f.fsub_m(rd(src, 8));
            wr::<f32>(b, 0xC0, f.fstp());
            f.fxch(1);
            f.fmul_m(k(K_0_3));
            wr::<f32>(b, 0xA0, f.fstp());
            f.fmul_m(k(K_0_3));
            f.fld(rd(b, 0xC0));
            f.fmul_m(k(K_0_3));
            f.fld(rd(b, 0xA0));
            f.fadd_m(rd(src, 0));
            wr::<f32>(b, 0x5C, f.fstp());
            f.fxch(1);
            f.fadd_m(rd(src, 4));
            wr::<f32>(b, 0x60, f.fstp());
            f.fadd_m(rd(src, 8));
            wr::<u32>(src, 0, rd::<u32>(b, 0x5C));
            wr::<u32>(src, 4, rd::<u32>(b, 0x60));
            wr::<f32>(b, 0x64, f.fstp());
            wr::<u32>(src, 8, rd::<u32>(b, 0x64));
        }
        // 0x5102C3: the next sphere
        let cam = SCENE_CAMERA.get();
        f.fld(rd(cam, 0x80));
        wr::<f32>(b, 0x14, f.fstp());
        f.fld(rd(this, 0xB4));
        f.fmul_m(k(K_DEG2RAD));
        f.fmul_m(k(K_0_5));
        f.fptan();
        f.fstp_st(0);
        f.fmul_m(d(ASPECT_RATIO));
        f.fmul_m(rd(b, 0x14));
        f.fmul_m(k(K_1_1));
        wr::<f32>(b, 0x10, f.fstp());
        f.fld(rd(b, 0x14));
        let r = rd::<u32>(b, 0x10);
        f.fmul_m(rd(front, 0));
        f.fld(rd(b, 0x14));
        f.fmul_m(rd(front, 4));
        f.fld(rd(b, 0x14));
        f.fmul_m(rd(front, 8));
        wr::<f32>(b, 0x9C, f.fstp());
        f.fxch(1);
        f.fadd_m(rd(src, 0));
        wr::<f32>(b, 0x34, f.fstp());
        f.fadd_m(rd(src, 4));
        wr::<f32>(b, 0x38, f.fstp());
        f.fld(rd(b, 0x9C));
        f.fadd_m(rd(src, 8));
        wr::<f32>(b, 0x3C, f.fstp());
        hit = test_sphere_against_world(
            rd::<u32>(b, 0x34),
            rd::<u32>(b, 0x38),
            rd::<u32>(b, 0x3C),
            r,
            GPtr::NULL,
            1,
            1,
            0,
            1,
            0,
            0,
        );
        n += 1;
        if n > 5 || hit.is_null() {
            return;
        }
    }
}

/// 0x510378: `m_fDistance` (+0xC4) follows `|T - source|` (snap when nearer, else `pow(0.92, ts)`
/// smoothing that also moves the source to that distance), then the near clip `D - 0.5` (at least 0.1)
/// when below the current one.
fn distance_smoothing(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu) {
    let src = this.byte_add(0x19C);
    dist_zyx(f, b.byte_add(0x28), src);
    wr::<f32>(b, 0x10, f.fst());
    f.fcomp_m(rd(this, 0xC4));
    if !f.jp(0x5) {
        wr::<u32>(this, 0xC4, rd::<u32>(b, 0x10));
        return;
    }
    f.fld(k(K_0_92));
    f.fld(d(TIME_STEP));
    ci_pow(f);
    f.fld(k(K_1));
    f.fsub_st(1);
    f.fmul_m(rd(b, 0x10));
    f.fxch(1);
    f.fmul_m(rd(this, 0xC4));
    f.faddp(1);
    wr::<f32>(this, 0xC4, f.fst());
    f.fld(rd(b, 0x10));
    f.fcomp_m(k(K_0_05));
    if !f.jne(0x41) {
        f.fld(rd(src, 0));
        f.fsub_m(rd(b, 0x28));
        f.fld(rd(src, 4));
        f.fsub_m(rd(b, 0x2C));
        f.fld(rd(src, 8));
        f.fsub_m(rd(b, 0x30));
        wr::<f32>(b, 0x9C, f.fstp());
        f.fld_st(2);
        wr::<f32>(b, 0x14, f.fstp());
        f.fxch(1);
        f.fmul_m(rd(b, 0x14));
        wr::<f32>(b, 0xA0, f.fstp());
        f.fmul_m(rd(b, 0x14));
        f.fld(rd(b, 0x9C));
        f.fmul_m(rd(b, 0x14));
        f.fld(k(K_1));
        f.fdiv_m(rd(b, 0x10));
        f.fld(rd(b, 0xA0));
        f.fmul_st(1);
        wr::<f32>(b, 0x44, f.fstp());
        f.fxch(2);
        f.fmul_st(2);
        wr::<f32>(b, 0x48, f.fstp());
        f.fmul_st(1);
        f.fxch(1);
        f.fstp_st(0);
        f.fld(rd(b, 0x44));
        f.fadd_m(rd(b, 0x28));
        wr::<f32>(b, 0x34, f.fstp());
        wr::<u32>(src, 0, rd::<u32>(b, 0x34));
        f.fld(rd(b, 0x48));
        f.fadd_m(rd(b, 0x2C));
        wr::<f32>(b, 0x38, f.fstp());
        wr::<u32>(src, 4, rd::<u32>(b, 0x38));
        f.fadd_m(rd(b, 0x30));
        wr::<f32>(b, 0x3C, f.fstp());
        wr::<u32>(src, 8, rd::<u32>(b, 0x3C));
    }
    f.fsub_m(d(NEAR_MARGIN));
    let cam = SCENE_CAMERA.get();
    f.fld(rd(cam, 0x80));
    f.fld_st(1);
    f.fcompp();
    if f.jp(0x5) {
        f.fstp_st(0);
        return;
    }
    f.fcom_m(k(K_0_1));
    if !f.jne(0x41) {
        wr::<f32>(b, 0x10, f.fstp());
        set_near_clip_raw(cam, rd::<u32>(b, 0x10));
    } else {
        wr::<u32>(b, 0x10, K_0_1);
        f.fstp_st(0);
        set_near_clip_raw(cam, K_0_1);
    }
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_FOLLOW_PED_WITH_MOUSE];

/// Globals of this module (none: the statics are accessed by address).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
