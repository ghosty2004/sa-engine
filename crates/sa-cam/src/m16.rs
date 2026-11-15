//! FRONTIER-2 (batch `frontier2-4`): `CCam::Process_M16_1stPerson` (0x5105C0), the first-person weapon aim camera
//! (modes 7 sniper, 0x2E camera, and the other first-person aim modes, on foot or in a drive-by from a vehicle).
//! Ported from the 1.0 US bytes (gta-reversed does not reverse it). Same technique as [`crate::rocket`]: a frame
//! array at the original's `B`-relative offsets (B = ESP after the four register pushes; the argument slots `B+0x60`
//! and up are reused as scratch) and the [`Fpu`] machine.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::platform::SCENE_CAMERA;
use sa_core::{hook, orig};

use crate::camcol::set_near_clip_raw;
use sa_world::collision::sphere_cast::ensure_matrix as inline_matrix;
use crate::cammodes::ped_get_bone_position;
use crate::camproc::{get_pad, get_vectors_ready_for_rw};
use crate::common::{Fpu, g, k, rd, wr};
use crate::followped::{ci_asin, ci_pow, clip_beta, copy3, d, fcos, fpatan, fsin, ped_is_player, push_st0};
use crate::pedonpc::{pad_sniper_zoom_in, pad_sniper_zoom_out};
use crate::rocket::{
    entity_update_rp_h_anim, entity_update_rw_frame, get_is_line_of_sight_clear, matrix_update_rw_matrix,
    pad_look_around_left_right, pad_look_around_up_down, simple_transform_update_rw_matrix,
};
use crate::bindings::{cc_well_buffer_me, set_heli_orientation};
use sa_world::bindings::{placeable_get_heading, placeable_get_matrix, ped_position_attached_ped};

orig! {
    /// `CControllerConfigManager::GetMouseButtonAssociatedWithAction(e_ControllerAction)` (0x52F580, `ret 4`)
    fn mouse_button_for_action(ccm: GPtr<u8>, action: i32) -> i32 = thiscall 0x52F580 => unported;
}

/// `ControlsManager` (0xB70198); the mouse wheel (0xB73420) and the mouse x / y deltas (0xB73424 / 0xB73428);
/// `CCamera::m_bUseMouse3rdPerson` (0xB6EC2E) and the mouse accelerations (0xB6EC18 vertical, 0xB6EC1C horizontal).
const CONTROLS_MANAGER: usize = 0xB70198;
const MOUSE_WHEEL: usize = 0xB73420;
const MOUSE_X: usize = 0xB73424;
const MOUSE_Y: usize = 0xB73428;
const USE_MOUSE_3RD_PERSON: usize = 0xB6EC2E;
const MOUSE_ACCEL_VERTICAL: usize = 0xB6EC18;
const MOUSE_ACCEL_HORZNTL: usize = 0xB6EC1C;
const TIME_STEP: usize = 0xB7CB5C;
const TIME_MS: usize = 0xB7CB84;
const CAMERA_TARGET: usize = 0xB6F980;
/// The drive-by heading the reset takes (0xA4449C) and the heading still to turn (0xA44498).
const DRIVEBY_HEADING: usize = 0xA4449C;
const DRIVEBY_TURN: usize = 0xA44498;
/// Statics reset with the camera: the target FOV of the zoom (0xB6FFE8), 0xB6FFEC, 0xB6FFF0 and the near-clip
/// flag (u8 0xB6FFF4).
const S_TARGET_FOV: usize = 0xB6FFE8;
const S_FFEC: usize = 0xB6FFEC;
const S_FFF0: usize = 0xB6FFF0;
const S_NEAR_CLIP_SET: usize = 0xB6FFF4;
/// The camera shake (`CCam` +0x118 / +0x11C amplitudes, +0x120 start time) tuning: period 800 ms (0x8CC474),
/// duration 600 ms (0x8CC478), decay 0.95 (0x8CC47C), scale 0.1 (0x8CC480).
const SHAKE_PERIOD: usize = 0x8CC474;
const SHAKE_DURATION: usize = 0x8CC478;
const SHAKE_DECAY: usize = 0x8CC47C;
const SHAKE_SCALE: usize = 0x8CC480;
/// The pad smoothing bases 0.8 / 0.5 (0x8CCC98 / 0x8CCC94), the alpha limit 1.2 (0x8CCC90), the ducking source
/// offsets 0.5 / 0.18 (0x8CC7BC / 0x8CC7B8).
const SMOOTH_SLOW: usize = 0x8CCC98;
const SMOOTH_FAST: usize = 0x8CCC94;
const ALPHA_LIMIT: usize = 0x8CCC90;
const DUCK_FWD: usize = 0x8CC7BC;
const DUCK_RIGHT: usize = 0x8CC7B8;

const K_0: u32 = 0; // [0x858B50]
const K_1: u32 = 0x3F80_0000; // [0x858624]
const K_M1: u32 = 0xBF80_0000; // [0x858C1C]
const K_0_1: u32 = 0x3DCC_CCCD; // [0x858B1C]
const K_0_3: u32 = 0x3E99_999A; // [0x858C24]
const K_0_4: u32 = 0x3ECC_CCCD; // [0x858EE8]
const K_0_5: u32 = 0x3F00_0000; // [0x858B8C]
const K_2: u32 = 0x4000_0000; // [0x858CA0]
const K_3: u32 = 0x4040_0000; // [0x858B3C]
const K_M3: u32 = 0xC040_0000; // [0x859024]
const K_7: u32 = 0x40E0_0000; // [0x862F50]
const K_15: u32 = 0x4170_0000; // [0x858B48]
const K_70: u32 = 0x428C_0000; // [0x858CE0]
const K_255: u32 = 0x437F_0000; // [0x859AAC]
const K_10000: u32 = 0x461C_4000; // [0x859AA4]
const K_0_0001: u32 = 0x38D1_B717; // [0x858FC4]
const K_0_0125: u32 = 0x3C4C_CCCD; // [0x85A5C4]
const K_0_04: u32 = 0x3D23_D70A; // [0x858CEC]
const K_1_128: u32 = 0x3C00_0000; // [0x858B88]
const K_DB_UD: u32 = 0x3D0C_6F2D; // 0.034285713 [0x8631C0]
const K_UD_SCALE: u32 = 0x383A_69DC; // [0x8630FC]
const K_1_14: u32 = 0x3D92_4925; // [0x859B50]
const K_LR_SCALE: u32 = 0x3D6A_0EA1; // [0x863100]
const K_2_32: u32 = 0x4F80_0000; // 4294967296 [0x858C54]
const K_PI: u32 = 0x4049_0FDB; // [0x858CB8]
const K_M_PI: u32 = 0xC049_0FDB; // [0x858CC0]
const K_2PI: u32 = 0x40C9_0FDB; // [0x858CBC]
const K_HALF_PI: u32 = 0x3FC9_0FDB; // [0x858FE4]
const K_0_05: u32 = 0x3D4C_CCCD; // [0x862F48]
const K_0_75: u32 = 0x3F40_0000; // [0x862F44]
const K_HELI: u32 = 0x3DCC_CCCD; // 0.1 [0x862F4C]
const K_ALPHA_MAX: u32 = 0x3F86_0A92; // [0x8630F8]
const K_ALPHA_MIN: u32 = 0xBFBF_0243; // [0x8631BC]
const K_LOS_BETA: u32 = 0x3F1C_61AA; // 0.61086524 [0x8631B8]
const K_LOS_ALPHA: u32 = 0x3EB2_B8C3; // 0.34906587 [0x8631B4]
const K_NEAR_FOV: u32 = 0x3E19_999A; // 0.15 [0x862F54]

/// `fld` of a frame / object dword.
fn fl(f: &mut Fpu, p: GPtr<u8>, off: isize) {
    f.fld(rd(p, off));
}

/// `fstp dword [p+off]`.
fn st(f: &mut Fpu, p: GPtr<u8>, off: isize) {
    wr::<f32>(p, off, f.fstp());
}

/// The ped's RW matrix (from its matrix or simple transform), frame and animation (0x511157 / 0x51159C).
fn update_ped_rw(t: GPtr<u8>) {
    let rw = rd::<GPtr<u8>>(t, 0x18);
    if !rw.is_null() {
        let rwm = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let m = rd::<GPtr<u8>>(t, 0x14);
        if !m.is_null() {
            matrix_update_rw_matrix(m, rwm);
        } else {
            simple_transform_update_rw_matrix(t.byte_add(4), rwm);
        }
    }
}

/// `CCam::Process_M16_1stPerson(const CVector& target, float orientation, float, float)` (0x5105C0, thiscall,
/// `ret 0x10`). A non-ped target does nothing. Order of calls, re-reads and x87 operations as in the bytes:
/// - drive-by = the target is a player attached to an entity (+0xFC);
/// - reset: beta from the drive-by heading (0xA4449C) or the ped's heading - pi/2 (unless the 3rd-person mouse is
///   on, the ped has no target entity and it is no drive-by: then neither beta nor alpha), alpha 0, beta2 = heading -
///   pi/2, statics cleared, collision checks on, FOV speed / shake angles 0, the target FOV = the FOV;
/// - modes 7 / 0x2E: the zoom ([`zoom`]); other modes FOV 70;
/// - a drive-by turns beta by the pending heading (0xA44498) ([`driveby_turn`]);
/// - the look input ([`look_input`]): mouse (`dB = mx * -3 * FOV*0.0125 * accelH`, `dA = my * 3 * FOV*0.0125 *
///   accelV`; zero while the pad's +0x10E / +0x117 are set or the time step is not positive) or pad (drive-by:
///   `|LR/128| * (FOV*0.0125 * 0.04) * ts * LR/128` and the up/down analogue; on foot the squared stick
///   formulas), smoothed with `pow(0.8 or 0.5, ts)` into +0xC0 / +0xB0; `ClipBeta`;
/// - the shake (+0x120): `cos(2pi (now - start) / 800) * 0.1` times the amplitudes, which decay by `0.95^ts`;
///   over after 600 ms;
/// - source and limits: drive-by ([`driveby_source`]) or on foot ([`on_foot_source`]); alpha within +-1.2;
/// - front from alpha / beta; the line-of-sight checks set the near clip ([`near_clip`]); `GetVectorsReadyForRW`;
///   the camera target's rotations = `atan2(-front.x, front.y)`.
#[hook(addr = 0x5105C0, conv = thiscall, name = "CCam::Process_M16_1stPerson", expect = "83 EC 4C 56 8B F1", this = "CCam",
       unverified = "frontier2-4", route = "L3-only", hazards = "x87-transcendental",
       note = "fsin / fcos / fpatan / _CIasin / _CIpow on the sa-math models; the frame lives in a Rust array")]
pub fn process_m16_1st_person(this: GPtr<u8>, target: GPtr<u8>, _o: u32, _sv: u32, _svw: u32) {
    let mut fr = [0u32; 0x24];
    let b = GPtr::<u32>::from_raw(fr.as_mut_ptr()).cast::<u8>();
    let mut fpu = Fpu::new();
    let f = &mut fpu;
    let t = rd::<GPtr<u8>>(this, 0x21C);
    if rd::<u8>(t, 0x36) & 7 != 3 {
        return;
    }
    wr::<u32>(b, 0x24, K_ALPHA_MAX);
    wr::<u32>(b, 0x28, 0x3FBF_0243);
    wr::<u32>(b, 0x14, K_0_1);
    wr::<u32>(b, 0x18, 0x3E99_999A);
    wr::<u8>(b, 0x13, 0);
    let mut bl = 0u8;
    if ped_is_player(t) != 0 && !rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x21C), 0xFC).is_null() {
        wr::<u8>(b, 0x13, 1);
        bl = 1;
    }
    copy3(b.byte_add(0x38), target);
    if rd::<u8>(this, 0xA) != 0 {
        let mouse_no_target =
            g::<u8>(USE_MOUSE_3RD_PERSON).read() != 0 && rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x21C), 0x71C).is_null();
        let set_alpha = if mouse_no_target {
            if bl != 0 {
                wr::<u32>(this, 0xBC, g::<u32>(DRIVEBY_HEADING).read());
                true
            } else {
                false
            }
        } else {
            if bl != 0 {
                wr::<u32>(this, 0xBC, g::<u32>(DRIVEBY_HEADING).read());
            } else {
                fl(f, rd::<GPtr<u8>>(this, 0x21C), 0x558);
                f.fsub_m(k(K_HALF_PI));
                st(f, this, 0xBC);
            }
            true
        };
        if set_alpha {
            wr::<u32>(this, 0xAC, 0);
        }
        fl(f, rd::<GPtr<u8>>(this, 0x21C), 0x558);
        let fov = rd::<u32>(this, 0xB4);
        f.fsub_m(k(K_HALF_PI));
        wr::<u8>(this, 0xA, 0);
        g::<u8>(S_NEAR_CLIP_SET).write(0);
        g::<u32>(S_FFF0).write(0);
        st(f, this, 0xA8);
        g::<u32>(S_FFEC).write(0);
        wr::<u8>(this, 3, 1);
        wr::<u32>(this, 0xB8, 0);
        g::<u32>(S_TARGET_FOV).write(fov);
        wr::<u32>(this, 0xB0, 0);
        wr::<u32>(this, 0xC0, 0);
    }
    let mode = rd::<i16>(this, 0xC);
    if mode == 7 || mode == 0x2E {
        zoom(this, f);
        bl = rd::<u8>(b, 0x13);
    } else {
        wr::<u32>(this, 0xB4, K_70);
    }
    if bl != 0 {
        driveby_turn(this, f);
    }
    look_input(this, b, f, bl);
    shake(this, f);
    let t = rd::<GPtr<u8>>(this, 0x21C);
    if rd::<u32>(t, 0x46C) & 0x400_0000 != 0 {
        wr::<u32>(b, 0x18, 0x3F4C_CCCD);
    }
    if rd::<u8>(b, 0x13) != 0 {
        driveby_source(this, b, f, t);
    } else {
        on_foot_source(this, b, f, t);
    }
    // 0x51176E: alpha within +-1.2
    f.fld(d(ALPHA_LIMIT));
    f.fchs();
    f.fcom_m(rd(this, 0xAC));
    if !f.jne(0x41) {
        st(f, this, 0xAC);
    } else {
        f.fstp_st(0);
        fl(f, this, 0xAC);
        f.fcomp_m(d(ALPHA_LIMIT));
        if !f.jne(0x41) {
            wr::<u32>(this, 0xAC, g::<u32>(ALPHA_LIMIT).read());
        }
    }
    // 0x5117AB: front from alpha / beta, the aim point 3 ahead (B+0x38), the source 0.4 ahead
    let front = this.byte_add(0x190);
    let src = this.byte_add(0x19C);
    fl(f, this, 0xAC);
    fcos(f);
    fl(f, this, 0xBC);
    fcos(f);
    f.fmul_st(1);
    f.fchs();
    st(f, b, 0x2C);
    wr::<u32>(front, 0, rd::<u32>(b, 0x2C));
    fl(f, this, 0xBC);
    fsin(f);
    f.fmul_st(1);
    f.fchs();
    st(f, b, 0x30);
    wr::<u32>(front, 4, rd::<u32>(b, 0x30));
    f.fstp_st(0);
    fl(f, this, 0xAC);
    fsin(f);
    st(f, b, 0x34);
    wr::<u32>(front, 8, rd::<u32>(b, 0x34));
    scaled_front(f, front, K_3, b);
    f.fxch(1);
    f.fadd_m(rd(src, 0));
    st(f, b, 0x2C);
    wr::<u32>(b, 0x38, rd::<u32>(b, 0x2C));
    f.fadd_m(rd(src, 4));
    st(f, b, 0x30);
    fl(f, b, 0x58);
    wr::<u32>(b, 0x3C, rd::<u32>(b, 0x30));
    f.fadd_m(rd(src, 8));
    st(f, b, 0x34);
    wr::<u32>(b, 0x40, rd::<u32>(b, 0x34));
    add_scaled_front(f, front, src, b, K_0_4);
    near_clip(this, b, f);
    // 0x511AC1: the source back by 0.4 of the front
    scaled_front(f, front, K_0_4, b);
    fl(f, src, 0);
    f.fsub_st(2);
    st(f, src, 0);
    fl(f, src, 4);
    f.fsub_st(1);
    st(f, src, 4);
    f.fstp_st(0);
    f.fstp_st(0);
    fl(f, src, 8);
    f.fsub_m(rd(b, 0x58));
    st(f, src, 8);
    get_vectors_ready_for_rw(this);
    fl(f, front, 0);
    f.fchs();
    fl(f, this, 0x194);
    fpatan(f);
    f.fld_st(0);
    st(f, g::<GPtr<u8>>(CAMERA_TARGET).read(), 0x558);
    st(f, g::<GPtr<u8>>(CAMERA_TARGET).read(), 0x55C);
}

/// `fld front[i]; fmul k` for x, y, z, then `fstp [B+0x58]` of z: the stack holds `y*k` on top of `x*k`.
fn scaled_front(f: &mut Fpu, front: GPtr<u8>, kk: u32, b: GPtr<u8>) {
    fl(f, front, 0);
    f.fmul_m(k(kk));
    fl(f, front, 4);
    f.fmul_m(k(kk));
    fl(f, front, 8);
    f.fmul_m(k(kk));
    st(f, b, 0x58);
}

/// `src += front * kk` (each component through the stack, z via B+0x58).
fn add_scaled_front(f: &mut Fpu, front: GPtr<u8>, src: GPtr<u8>, b: GPtr<u8>, kk: u32) {
    scaled_front(f, front, kk, b);
    f.fxch(1);
    f.fadd_m(rd(src, 0));
    st(f, src, 0);
    f.fadd_m(rd(src, 4));
    st(f, src, 4);
    fl(f, b, 0x58);
    f.fadd_m(rd(src, 8));
    st(f, src, 8);
}

/// Modes 7 / 0x2E (0x51071C): the mouse wheel zooms when it is bound to zoom in / out (actions 8 / 9 with buttons 4
/// / 5): the target FOV (0xB6FFE8) times / divided by `(|wheel| * 7 + 10000) * 0.0001`; otherwise the pad's sniper
/// zoom out / in scales the FOV by `(ts * 255 + 10000) * 0.0001` (target = FOV), else the FOV follows the target
/// (`WellBufferMe(target, &FOV, &speed, 0.5, 0.25, false)` when more than 0.5 apart). FOV and target within [15 (3
/// for the camera, 0x2E), 70]; the cross-hair colour statics.
fn zoom(this: GPtr<u8>, f: &mut Fpu) {
    let zoom_in_button = mouse_button_for_action(GPtr::from_addr(CONTROLS_MANAGER), 8);
    let zoom_out_button = mouse_button_for_action(GPtr::from_addr(CONTROLS_MANAGER), 9);
    let wheel = |f: &mut Fpu| {
        let w = g::<u32>(MOUSE_WHEEL).read();
        let _ = get_pad(0);
        f.fld(f32::from_bits(w));
    };
    let wheel_pos = |f: &mut Fpu| {
        wheel(f);
        f.fcomp_m(k(K_0));
        !f.jne(0x41)
    };
    let wheel_neg = |f: &mut Fpu| {
        wheel(f);
        f.fcomp_m(k(K_0));
        !f.jp(0x5)
    };
    let wheel_step = |f: &mut Fpu| {
        wheel(f);
        f.fabs();
        f.fmul_m(k(K_7));
        f.fadd_m(k(K_10000));
    };
    let mut used_wheel = false;
    if (wheel_pos(f) && zoom_out_button == 4) || (wheel_neg(f) && zoom_out_button == 5) {
        wheel_step(f);
        f.fmul_m(d(S_TARGET_FOV));
        f.fmul_m(k(K_0_0001));
        wr::<f32>(GPtr::from_addr(S_TARGET_FOV), 0, f.fstp());
        used_wheel = true;
    } else if (wheel_pos(f) && zoom_in_button == 4) || (wheel_neg(f) && zoom_in_button == 5) {
        wheel_step(f);
        f.fmul_m(k(K_0_0001));
        f.fdivr_m(d(S_TARGET_FOV));
        wr::<f32>(GPtr::from_addr(S_TARGET_FOV), 0, f.fstp());
        used_wheel = true;
    }
    let fov = this.byte_add(0xB4);
    let step = |f: &mut Fpu| {
        f.fld(d(TIME_STEP));
        f.fmul_m(k(K_255));
        f.fadd_m(k(K_10000));
    };
    let mut reset_speed = true;
    if pad_sniper_zoom_out(get_pad(0)) != 0 && !used_wheel {
        step(f);
        f.fmul_m(rd(fov, 0));
        f.fmul_m(k(K_0_0001));
        st(f, fov, 0);
        g::<u32>(S_TARGET_FOV).write(rd::<u32>(fov, 0));
    } else if pad_sniper_zoom_in(get_pad(0)) != 0 && !used_wheel {
        step(f);
        f.fmul_m(k(K_0_0001));
        f.fdivr_m(rd(fov, 0));
        st(f, fov, 0);
        g::<u32>(S_TARGET_FOV).write(rd::<u32>(fov, 0));
    } else {
        f.fld(d(S_TARGET_FOV));
        f.fsub_m(rd(fov, 0));
        f.fabs();
        f.fcomp_m(k(K_0_5));
        if !f.jne(0x41) {
            cc_well_buffer_me(
                d(S_TARGET_FOV),
                fov.cast(),
                this.byte_add(0xB8).cast(),
                f32::from_bits(0x3F00_0000),
                f32::from_bits(0x3E80_0000),
                0,
            );
            reset_speed = false;
        }
    }
    if reset_speed {
        wr::<u32>(this, 0xB8, 0);
    }
    // 0x51089B: the upper limit 70 for the FOV and the target
    f.fld(rd(fov, 0));
    f.fcomp_m(k(K_70));
    if !f.jne(0x41) {
        wr::<u32>(fov, 0, K_70);
    }
    f.fld(d(S_TARGET_FOV));
    f.fcomp_m(k(K_70));
    if !f.jne(0x41) {
        g::<u32>(S_TARGET_FOV).write(K_70);
    } else {
        let low = if rd::<i16>(this, 0xC) == 0x2E { K_3 } else { K_15 };
        f.fld(rd(fov, 0));
        f.fcomp_m(k(low));
        if !f.jp(0x5) {
            wr::<u32>(fov, 0, low);
        }
        f.fld(d(S_TARGET_FOV));
        f.fcomp_m(k(low));
        if !f.jp(0x5) {
            g::<u32>(S_TARGET_FOV).write(low);
        }
    }
    g::<u32>(0xB6F0B4).write(0xB4);
    g::<u32>(0xB6F0B0).write(0xFF);
    g::<u32>(0xB6F0AC).write(0xB4);
    g::<u32>(0xB6F0B8).write(1);
    g::<u32>(0xB6F0C8).write(0x78);
}

/// 0x510A0C: with a pending drive-by turn (0xA44498 > 0.0), `D = beta - heading` (+2pi when negative) and `2pi - D`;
/// when either is below the turn, beta = the heading and the turn ends; otherwise beta moves by the turn towards the
/// heading (down when `D <= 2pi - D`).
fn driveby_turn(this: GPtr<u8>, f: &mut Fpu) {
    // fcomp qword 0.0; test ah, 0x41; jne: only a positive turn
    f.fld(d(DRIVEBY_TURN));
    f.fcomp_m(0.0);
    if f.jne(0x41) {
        return;
    }
    fl(f, this, 0xBC);
    f.fsub_m(d(DRIVEBY_HEADING));
    f.fcom_m(k(K_0));
    if !f.jp(0x5) {
        f.fadd_m(k(K_2PI));
    }
    f.fld(k(K_2PI));
    f.fsub_st(1);
    let rest = f.fstp();
    f.fcom_m(d(DRIVEBY_TURN));
    let done = !f.jp(0x5) || {
        f.fld(rest);
        f.fcomp_m(d(DRIVEBY_TURN));
        !f.jp(0x5)
    };
    if done {
        wr::<u32>(this, 0xBC, g::<u32>(DRIVEBY_HEADING).read());
        f.fstp_st(0);
        g::<u32>(DRIVEBY_TURN).write(0);
        return;
    }
    f.fcomp_m(rest);
    if !f.jne(0x41) {
        f.fld(d(DRIVEBY_TURN));
        f.fadd_m(rd(this, 0xBC));
    } else {
        fl(f, this, 0xBC);
        f.fsub_m(d(DRIVEBY_TURN));
    }
    st(f, this, 0xBC);
}

/// 0x510AB6: the look input, leaving `[dA, dB]` applied to alpha (+0xAC) and beta (+0xBC), then `ClipBeta`.
fn look_input(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu, bl: u8) {
    let _ = get_pad(0);
    wr::<u32>(b, 0x2C, g::<u32>(MOUSE_X).read());
    f.fld(rd(b, 0x2C));
    f.fcomp_m(k(K_0));
    wr::<u32>(b, 0x30, g::<u32>(MOUSE_Y).read());
    let mouse = f.jp(0x44) || {
        f.fld(rd(b, 0x30));
        f.fcomp_m(k(K_0));
        f.jp(0x44)
    };
    let fov = this.byte_add(0xB4);
    if mouse {
        fl(f, b, 0x2C);
        f.fmul_m(k(K_M3));
        st(f, b, 0x60);
        fl(f, b, 0x30);
        f.fmul_m(k(K_3));
        st(f, b, 0x20);
        let still = rd::<i16>(get_pad(0), 0x10E) != 0
            || rd::<u8>(get_pad(0), 0x117) != 0
            || {
                f.fld(d(TIME_STEP));
                f.fcomp_m(k(K_0));
                !f.jp(0x41)
            };
        if still {
            f.fld(k(K_0));
            wr::<u32>(b, 0x20, 0);
        } else {
            fl(f, b, 0x60);
        }
        fl(f, fov, 0);
        f.fmul_m(k(K_0_0125));
        wr::<f32>(b, 0x60, f.fst());
        f.fmul_m(d(MOUSE_ACCEL_HORZNTL));
        f.fmulp(1);
        fl(f, b, 0x60);
        f.fmul_m(d(MOUSE_ACCEL_VERTICAL));
        wr::<u32>(this, 0xB0, 0);
        wr::<u32>(this, 0xC0, 0);
        f.fmul_m(rd(b, 0x20));
    } else {
        let t = rd::<GPtr<u8>>(this, 0x21C);
        let lr = i32::from(pad_look_around_left_right(get_pad(0), t)).wrapping_neg();
        let t = rd::<GPtr<u8>>(this, 0x21C);
        f.fild(lr);
        st(f, b, 0x60);
        let ud = i32::from(pad_look_around_up_down(get_pad(0), t));
        wr::<i32>(b, 0x2C, ud);
        f.fild(ud);
        if bl != 0 {
            // 0x510B4F: [ud]
            fl(f, b, 0x60);
            f.fmul_m(k(K_1_128));
            fl(f, fov, 0);
            f.fmul_m(k(K_0_0125));
            f.fld_st(1);
            f.fabs();
            f.fld_st(1);
            f.fmul_m(k(K_0_04));
            f.fmulp(1);
            f.fmul_m(d(TIME_STEP));
            f.fmul_st(2);
            st(f, b, 0x20);
            f.fld_st(2);
            f.fmul_m(k(K_1_128));
            f.fld_st(0);
            f.fabs();
            f.fxch(2);
            f.fmul_m(k(K_DB_UD));
            f.fmulp(2);
            f.fxch(1);
            f.fmul_m(d(TIME_STEP));
            f.fmul_st(1);
            st(f, b, 0x2C);
            f.fstp_st(0);
        } else {
            // 0x510C41: [ud]
            f.fld(k(K_1));
            wr::<u32>(b, 0x20, K_1);
            fl(f, b, 0x60);
            f.fcomp_m(k(K_0));
            if !f.jp(0x5) {
                wr::<u32>(b, 0x20, K_M1);
            }
            f.fld_st(1);
            f.fcomp_m(k(K_0));
            if !f.jp(0x5) {
                f.fstp_st(0);
                f.fld(k(K_M1));
            }
            fl(f, fov, 0);
            f.fmul_m(k(K_0_0125));
            fl(f, b, 0x60);
            f.fmul_m(rd(b, 0x60));
            f.fmul_m(k(K_0_0001));
            f.fld_st(1);
            f.fmul_m(k(K_LR_SCALE));
            f.fmulp(1);
            f.fmul_m(d(TIME_STEP));
            f.fmul_m(rd(b, 0x20));
            st(f, b, 0x20);
            f.fld_st(2);
            f.fmul_st(3);
            f.fmul_m(k(K_UD_SCALE));
            f.fxch(1);
            f.fmul_m(k(K_1_14));
            f.fmulp(1);
            f.fmul_m(d(TIME_STEP));
            f.fmul_st(1);
            st(f, b, 0x2C);
        }
        // 0x510CCD: [x, ud]
        wr::<u32>(b, 0x1C, g::<u32>(SMOOTH_SLOW).read());
        f.fstp_st(0);
        fl(f, b, 0x60);
        f.fabs();
        f.fcomp_m(k(K_2));
        if !f.jp(0x5) {
            f.fabs();
            f.fcomp_m(k(K_2));
            if !f.jp(0x5) {
                wr::<u32>(b, 0x1C, g::<u32>(SMOOTH_FAST).read());
            }
        } else {
            f.fstp_st(0);
        }
        fl(f, b, 0x1C);
        f.fld(d(TIME_STEP));
        ci_pow(f);
        st(f, b, 0x1C);
        f.fld(k(K_1));
        f.fsub_m(rd(b, 0x1C));
        wr::<f32>(b, 0x60, f.fst());
        f.fmul_m(rd(b, 0x20));
        fl(f, b, 0x1C);
        f.fmul_m(rd(this, 0xC0));
        f.faddp(1);
        wr::<f32>(this, 0xC0, f.fst());
        fl(f, b, 0x1C);
        f.fmul_m(rd(this, 0xB0));
        fl(f, b, 0x60);
        f.fmul_m(rd(b, 0x2C));
        f.faddp(1);
        wr::<f32>(b, 0x60, f.fst());
        st(f, this, 0xB0);
        fl(f, b, 0x60);
    }
    // 0x510D62: [dA, dB]
    f.fxch(1);
    f.fadd_m(rd(this, 0xBC));
    st(f, this, 0xBC);
    f.fadd_m(rd(this, 0xAC));
    st(f, this, 0xAC);
    clip_beta(this);
}

/// 0x510D83: the camera shake while +0x120 (start time) is positive.
fn shake(this: GPtr<u8>, f: &mut Fpu) {
    let start = rd::<i32>(this, 0x120);
    if start <= 0 {
        return;
    }
    let dt = g::<i32>(TIME_MS).read().wrapping_sub(start);
    f.fild(dt);
    if dt < 0 {
        f.fadd_m(k(K_2_32));
    }
    let v = f.pop().fdiv(sa_math::x87::X87::from_i32(g::<i32>(SHAKE_PERIOD).read()));
    f.push(v);
    f.fmul_m(k(K_2PI));
    fcos(f);
    f.fld(d(SHAKE_SCALE));
    f.fmul_st(1);
    f.fmul_m(rd(this, 0x118));
    f.fadd_m(rd(this, 0xBC));
    st(f, this, 0xBC);
    f.fmul_m(rd(this, 0x11C));
    f.fmul_m(d(SHAKE_SCALE));
    f.fadd_m(rd(this, 0xAC));
    st(f, this, 0xAC);
    for off in [0x118, 0x11C] {
        f.fld(d(SHAKE_DECAY));
        f.fld(d(TIME_STEP));
        ci_pow(f);
        f.fmul_m(rd(this, off));
        st(f, this, off);
    }
    let end = (start as u32).wrapping_add(g::<u32>(SHAKE_DURATION).read());
    if g::<u32>(TIME_MS).read() > end {
        wr::<u32>(this, 0x120, 0);
    }
}

/// 0x510E64: the drive-by pitch and heading from the attached vehicle (+0xFC) by the seat / attach type (+0x778):
/// 1 → `asin(clamp(-right.z, -1, 1))`, heading; 2 → `asin(clamp(-forward.z))`, heading + pi/2; 3 → `asin(clamp(right.z))`,
/// heading - pi; others → `asin(clamp(forward.z))`, `atan2(-forward.x, forward.y)` (or the simple heading) - pi/2.
/// Then the ped is positioned on the vehicle and the source is bone 5 (on a bike with the head offsets cleared and
/// the attach limits +0x780 for both alpha bounds) moved along the ped's up / back; beta follows the heading beyond
/// the seat's limit (+0x77C, 0.75 of it free, 0.05 * ts catch-up, a heli turns with it), alpha stays within the
/// limits (B+0x24 / B+0x28).
fn driveby_source(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu, t: GPtr<u8>) {
    let ty = rd::<u16>(t, 0x778);
    let veh = || rd::<GPtr<u8>>(t, 0xFC);
    match ty {
        1..=3 => {
            let (off, neg) = match ty {
                1 => (8, true),
                2 => (0x18, true),
                _ => (8, false),
            };
            // the bytes test `> 1` first, then `-1 > v`, each with a fresh GetMatrix
            clamped_ordered(f, b, off, neg, veh);
            fl(f, b, 0x60);
            ci_asin(f);
            st(f, b, 0x1C);
            push_st0(f, placeable_get_heading(veh()));
            match ty {
                2 => f.fadd_m(k(K_HALF_PI)),
                3 => f.fsub_m(k(K_PI)),
                _ => {}
            }
        }
        _ => {
            let m = |_: ()| inline_matrix(veh());
            fl(f, m(()), 0x18);
            f.fcomp_m(k(K_1));
            let v = if !f.jne(0x41) {
                K_1
            } else {
                fl(f, m(()), 0x18);
                f.fld(k(K_M1));
                f.fcomp_st(1);
                f.fstp_st(0);
                if !f.jne(0x41) {
                    K_M1
                } else {
                    let _ = m(());
                    fl(f, placeable_get_matrix(veh()), 0x18);
                    f.fstp().to_bits()
                }
            };
            wr::<u32>(b, 0x60, v);
            fl(f, b, 0x60);
            ci_asin(f);
            let v = veh();
            let mm = rd::<GPtr<u8>>(v, 0x14);
            st(f, b, 0x1C);
            if !mm.is_null() {
                fl(f, mm, 0x10);
                f.fchs();
                fl(f, mm, 0x14);
                fpatan(f);
            } else {
                fl(f, v, 0x10);
            }
            f.fsub_m(k(K_HALF_PI));
        }
    }
    st(f, b, 0x20);
    ped_position_attached_ped(t);
    update_ped_rw(t);
    entity_update_rw_frame(t);
    entity_update_rp_h_anim(t);
    wr::<u32>(b, 0x2C, 0);
    wr::<u32>(b, 0x30, 0);
    wr::<u32>(b, 0x34, 0);
    let v = rd::<GPtr<u8>>(t, 0xFC);
    if rd::<u8>(v, 0x36) & 7 == 2 && rd::<i32>(v, 0x590) == 9 {
        wr::<u32>(b, 0x14, 0);
        wr::<u32>(b, 0x18, 0);
        ped_get_bone_position(rd::<GPtr<u8>>(this, 0x21C), b.byte_add(0x2C), 5, 1);
        let lim = rd::<u32>(t, 0x780);
        wr::<u32>(b, 0x24, lim);
        wr::<u32>(b, 0x28, lim);
    } else {
        ped_get_bone_position(rd::<GPtr<u8>>(this, 0x21C), b.byte_add(0x2C), 5, 1);
    }
    let src = this.byte_add(0x19C);
    copy3(src, b.byte_add(0x2C));
    // src += B14 * up; src -= B18 * forward
    let m = inline_matrix(rd::<GPtr<u8>>(this, 0x21C));
    fl(f, b, 0x14);
    f.fmul_m(rd(m, 0x20));
    fl(f, b, 0x14);
    f.fmul_m(rd(m, 0x24));
    fl(f, b, 0x14);
    f.fmul_m(rd(m, 0x28));
    st(f, b, 0x58);
    f.fxch(1);
    f.fadd_m(rd(src, 0));
    st(f, src, 0);
    f.fadd_m(rd(src, 4));
    st(f, src, 4);
    fl(f, b, 0x58);
    f.fadd_m(rd(src, 8));
    st(f, src, 8);
    let m = inline_matrix(rd::<GPtr<u8>>(this, 0x21C));
    fl(f, b, 0x18);
    f.fmul_m(rd(m, 0x10));
    fl(f, b, 0x18);
    f.fmul_m(rd(m, 0x14));
    fl(f, b, 0x18);
    f.fmul_m(rd(m, 0x18));
    st(f, b, 0x58);
    fl(f, src, 0);
    f.fsub_st(2);
    st(f, src, 0);
    fl(f, src, 4);
    f.fsub_st(1);
    st(f, src, 4);
    f.fstp_st(0);
    f.fstp_st(0);
    fl(f, src, 8);
    f.fsub_m(rd(b, 0x58));
    st(f, src, 8);
    driveby_beta(this, b, f, t);
    driveby_alpha(this, b, f);
}

/// The three-way clamp of the drive-by cases 1..3 (`> 1` → 1, `-1 > v` → -1, else v), each test with a fresh
/// `GetMatrix` of the vehicle, into B+0x60.
fn clamped_ordered(f: &mut Fpu, b: GPtr<u8>, off: isize, neg: bool, veh: impl Fn() -> GPtr<u8>) {
    let load = |f: &mut Fpu| {
        fl(f, placeable_get_matrix(veh()), off);
        if neg {
            f.fchs();
        }
    };
    load(f);
    f.fcomp_m(k(K_1));
    if !f.jne(0x41) {
        // greater than 1: the bytes go on to the `<= 1` test, which fails, and take 1.0
        load(f);
        f.fcomp_m(k(K_1));
        let v = if !f.jne(0x41) {
            K_1
        } else {
            load(f);
            f.fstp().to_bits()
        };
        wr::<u32>(b, 0x60, v);
        return;
    }
    load(f);
    f.fld(k(K_M1));
    f.fcomp_st(1);
    f.fstp_st(0);
    if !f.jne(0x41) {
        wr::<u32>(b, 0x60, K_M1);
        return;
    }
    load(f);
    f.fcomp_m(k(K_1));
    let v = if !f.jne(0x41) {
        K_1
    } else {
        load(f);
        f.fstp().to_bits()
    };
    wr::<u32>(b, 0x60, v);
}

/// 0x5112D6: beta towards the vehicle's heading (B+0x20) beyond 0.75 of the seat limit (+0x77C).
fn driveby_beta(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu, t: GPtr<u8>) {
    f.fld(d(TIME_STEP));
    f.fmul_m(k(K_0_05));
    st(f, b, 0x60);
    f.fld(k(K_0_75));
    f.fmul_m(rd(t, 0x77C));
    f.fld(k(K_1));
    f.fsub_m(k(K_0_75));
    f.fmul_m(rd(t, 0x77C));
    st(f, b, 0x14);
    // [lim]
    fl(f, b, 0x20);
    f.fld_st(0);
    f.fsub_m(rd(this, 0xBC));
    f.fcom_m(k(K_PI));
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fsub_m(k(K_2PI));
    } else {
        f.fcomp_m(k(K_M_PI));
        if !f.jp(0x5) {
            f.fadd_m(k(K_2PI));
        }
    }
    let v = rd::<GPtr<u8>>(t, 0xFC);
    let heli = if rd::<u8>(v, 0x36) & 7 == 2 && rd::<u32>(v, 0x594) == 3 { v } else { GPtr::NULL };
    f.fsub_m(rd(this, 0xBC));
    // [D, lim]
    f.fcom_st(1);
    let excess = if !f.jne(0x41) {
        f.fsub_st(1);
        true
    } else {
        f.fld_st(1);
        f.fchs();
        f.fld_st(1);
        f.fcompp();
        if !f.jp(0x5) {
            f.fadd_st(1);
            true
        } else {
            f.fstp_st(0);
            f.fstp_st(0);
            false
        }
    };
    if !excess {
        return;
    }
    st(f, b, 0x18);
    f.fstp_st(0);
    fl(f, b, 0x18);
    f.fabs();
    fl(f, b, 0x60);
    f.fmul_st(1);
    st(f, b, 0x60);
    fl(f, b, 0x14);
    f.fadd_m(rd(b, 0x60));
    f.fld_st(1);
    f.fcompp();
    if !f.jne(0x41) {
        f.fsub_m(rd(b, 0x14));
        st(f, b, 0x60);
    } else {
        f.fstp_st(0);
    }
    if !heli.is_null() {
        f.fld(rd(heli, 0x954));
        f.fcomp_m(k(K_0));
        if !f.jne(0x41) {
            f.fld(d(TIME_STEP));
            f.fmul_m(rd(b, 0x18));
            f.fmul_m(k(K_HELI));
            f.fsubr_m(rd(heli, 0x954));
            let o = f.fstp();
            set_heli_orientation(heli, o.to_bits());
        }
    }
    fl(f, b, 0x18);
    f.fcomp_m(k(K_0));
    if !f.jp(0x44) {
        return;
    }
    fl(f, b, 0x18);
    f.fcomp_m(k(K_0));
    if !f.jp(0x5) {
        fl(f, this, 0xBC);
        f.fsub_m(rd(b, 0x60));
    } else {
        fl(f, b, 0x60);
        f.fadd_m(rd(this, 0xBC));
    }
    st(f, this, 0xBC);
}

/// 0x511439: alpha towards the vehicle pitch (B+0x1C) beyond 0.75 of the limits B+0x24 (above) / B+0x28 (below),
/// catching up by `ts * 0.05` of the excess beyond the free 0.25.
fn driveby_alpha(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu) {
    f.fld(k(K_0_75));
    f.fmul_m(rd(b, 0x24));
    st(f, b, 0x60);
    f.fld(k(K_1));
    f.fsub_m(k(K_0_75));
    fl(f, b, 0x24);
    f.fmul_st(1);
    st(f, b, 0x14);
    // [1 - 0.75]
    fl(f, b, 0x60);
    f.fadd_m(rd(b, 0x1C));
    f.fcomp_m(rd(this, 0xAC));
    if !f.jp(0x5) {
        fl(f, b, 0x1C);
        f.fsub_m(rd(this, 0xAC));
        fl(f, b, 0x60);
        f.fchs();
        f.fld_st(1);
        f.fcompp();
        if !f.jp(0x5) {
            f.fadd_m(rd(b, 0x60));
            f.fabs();
            st(f, b, 0x60);
            f.fld(d(TIME_STEP));
            f.fmul_m(k(K_0_05));
            f.fmul_m(rd(b, 0x60));
            fl(f, b, 0x14);
            f.fadd_st(1);
            f.fcomp_m(rd(b, 0x60));
            if !f.jp(0x5) {
                f.fstp_st(0);
                fl(f, b, 0x60);
                f.fsub_m(rd(b, 0x14));
            }
            fl(f, this, 0xAC);
            f.fsub_st(1);
            st(f, this, 0xAC);
        }
        f.fstp_st(0);
    }
    // 0x5114D2: [1 - 0.75]
    f.fld(k(K_0_75));
    f.fmul_m(rd(b, 0x28));
    st(f, b, 0x60);
    f.fmul_m(rd(b, 0x28));
    st(f, b, 0x14);
    fl(f, b, 0x1C);
    f.fsub_m(rd(b, 0x60));
    f.fcomp_m(rd(this, 0xAC));
    if f.jne(0x41) {
        return;
    }
    fl(f, b, 0x1C);
    f.fsub_m(rd(this, 0xAC));
    f.fcom_m(rd(b, 0x60));
    if f.jne(0x41) {
        f.fstp_st(0);
        return;
    }
    f.fsub_m(rd(b, 0x60));
    f.fabs();
    st(f, b, 0x60);
    f.fld(d(TIME_STEP));
    f.fmul_m(k(K_0_05));
    f.fmul_m(rd(b, 0x60));
    fl(f, b, 0x14);
    f.fadd_st(1);
    f.fcomp_m(rd(b, 0x60));
    if !f.jp(0x5) {
        f.fstp_st(0);
        fl(f, b, 0x60);
        f.fsub_m(rd(b, 0x14));
    }
    f.fadd_m(rd(this, 0xAC));
    st(f, this, 0xAC);
}

/// 0x511560: on foot: alpha clamped to [-1.4922565, 1.0471976]; the ped's RW matrix / frame / animation; the
/// source = bone 5 + 0.1 up, then back along the forward (and right, ducking: 0.5 / 0.18) vector.
fn on_foot_source(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu, t: GPtr<u8>) {
    fl(f, this, 0xAC);
    f.fcomp_m(k(K_ALPHA_MAX));
    if !f.jne(0x41) {
        wr::<u32>(this, 0xAC, 0x3F86_0A92);
    } else {
        fl(f, this, 0xAC);
        f.fcomp_m(k(K_ALPHA_MIN));
        if !f.jp(0x5) {
            wr::<u32>(this, 0xAC, 0xBFBF_0243);
        }
    }
    update_ped_rw(t);
    entity_update_rw_frame(rd::<GPtr<u8>>(this, 0x21C));
    entity_update_rp_h_anim(rd::<GPtr<u8>>(this, 0x21C));
    wr::<u32>(b, 0x2C, 0);
    wr::<u32>(b, 0x30, 0);
    wr::<u32>(b, 0x34, 0);
    ped_get_bone_position(rd::<GPtr<u8>>(this, 0x21C), b.byte_add(0x2C), 5, 1);
    let src = this.byte_add(0x19C);
    copy3(src, b.byte_add(0x2C));
    fl(f, this, 0x1A4);
    f.fadd_m(k(K_0_1));
    st(f, this, 0x1A4);
    let ped = rd::<GPtr<u8>>(this, 0x21C);
    let back = |f: &mut Fpu, kv: f32, moff: isize, soff: isize| {
        let m = inline_matrix(rd::<GPtr<u8>>(this, 0x21C));
        f.fld(kv);
        f.fmul_m(rd(m, moff));
        f.fsubr_m(rd(src, soff));
        st(f, src, soff);
    };
    if rd::<u32>(ped, 0x46C) & 0x400_0000 != 0 {
        back(f, d(DUCK_FWD), 0x10, 0);
        back(f, d(DUCK_FWD), 0x14, 4);
        back(f, d(DUCK_RIGHT), 0, 0);
        back(f, d(DUCK_RIGHT), 4, 4);
    } else {
        back(f, rd(b, 0x18), 0x10, 0);
        back(f, rd(b, 0x18), 0x14, 4);
    }
}

/// 0x511883: with collision checks on (+3 == 1), the aim point (B+0x38) and two side points (beta +- 0.61086524,
/// alpha - 0.34906587, 3 out) must see the source, else the near clip is 0.3 and the flag (0xB6FFF4) set; all clear
/// clears the flag. Without the checks a set flag keeps the clip. Then the camera mode 0x2E sets the near clip to
/// `((15 - min(FOV, 15)) * 0.15 + 1) * 0.3`.
fn near_clip(this: GPtr<u8>, b: GPtr<u8>, f: &mut Fpu) {
    let src = this.byte_add(0x19C);
    let blocked = || {
        set_near_clip_raw(SCENE_CAMERA.get(), f32::from_bits(K_0_3));
        g::<u8>(S_NEAR_CLIP_SET).write(1);
    };
    if rd::<u8>(this, 3) == 1 {
        if get_is_line_of_sight_clear(b.byte_add(0x38), src, 1, 1, 0, 1, 0, 1, 1) == 0 {
            blocked();
            return;
        }
        for sign in [true, false] {
            fl(f, this, 0xBC);
            if sign {
                f.fadd_m(k(K_LOS_BETA));
            } else {
                f.fsub_m(k(K_LOS_BETA));
            }
            fl(f, this, 0xAC);
            f.fsub_m(k(K_LOS_ALPHA));
            f.fld_st(0);
            fcos(f);
            f.fld_st(2);
            fcos(f);
            f.fmul_st(1);
            st(f, b, 0x44);
            f.fxch(2);
            fsin(f);
            f.fmul_st(2);
            st(f, b, 0x48);
            f.fstp_st(1);
            fsin(f);
            fl(f, b, 0x44);
            f.fmul_m(k(K_3));
            fl(f, b, 0x48);
            f.fmul_m(k(K_3));
            st(f, b, 0x30);
            f.fxch(1);
            f.fmul_m(k(K_3));
            st(f, b, 0x34);
            f.fadd_m(rd(src, 0));
            st(f, b, 0x44);
            wr::<u32>(b, 0x2C, rd::<u32>(b, 0x44));
            fl(f, b, 0x30);
            f.fadd_m(rd(src, 4));
            st(f, b, 0x48);
            fl(f, b, 0x34);
            wr::<u32>(b, 0x30, rd::<u32>(b, 0x48));
            f.fadd_m(rd(src, 8));
            st(f, b, 0x4C);
            wr::<u32>(b, 0x34, rd::<u32>(b, 0x4C));
            if get_is_line_of_sight_clear(b.byte_add(0x2C), src, 1, 1, 0, 1, 0, 1, 1) == 0 {
                blocked();
                return;
            }
        }
        g::<u8>(S_NEAR_CLIP_SET).write(0);
    } else if g::<u8>(S_NEAR_CLIP_SET).read() != 0 {
        return;
    }
    if rd::<i16>(this, 0xC) != 0x2E {
        return;
    }
    f.fld(k(K_15));
    f.fcomp_m(rd(this, 0xB4));
    if !f.jp(0x5) {
        f.fld(k(K_15));
    } else {
        fl(f, this, 0xB4);
    }
    f.fld(k(K_15));
    f.fsub_st(1);
    f.fmul_m(k(K_NEAR_FOV));
    f.fadd_m(k(K_1));
    f.fmul_m(k(K_0_3));
    let near = f.fstp();
    f.fstp_st(0);
    set_near_clip_raw(SCENE_CAMERA.get(), near);
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_M16_1ST_PERSON];
