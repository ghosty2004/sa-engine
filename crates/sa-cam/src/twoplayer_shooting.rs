//! `CCam::Process_Cam_TwoPlayer_InCarAndShooting` (0x519810): player 0's vehicle followed while the
//! other player (the one not driving) aims a screen cursor and fires drive-by shots.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::cammodes::avoid_the_geometry;
use crate::common::{Fpu, THE_CAMERA, g, k, rd, wr};
use crate::followped::{ci_pow, fcos, fpatan, fsin};
use crate::twoplayer::{Frame, PLAYER0_PED, PLAYER1_PED, pos_of};

sa_core::orig! {
    /// `CCam::RotCamIfInFrontCar(const CVector&, float)` (0x50A4F0)
    fn rot_cam_if_in_front_car(c: GPtr<u8>, target: GPtr<u8>, orientation: u32) -> u8 = thiscall 0x50A4F0 => crate::followcar::rot_cam_if_in_front_car;
}

const TIME_STEP: usize = 0xB7CB5C;
const ASPECT_RATIO: usize = 0xC3EFA4;
/// `.data` tunables, read at run time: 0.6, 0.4, 0.98, 1.3, 0.4, 0.03, 0.3.
const HELI_RAISE: usize = 0x8CC53C;
const FOV_SPEED_THRESHOLD: usize = 0x8CC540;
const FOV_DECAY: usize = 0x8CC544;
const RAISE_SCALE: usize = 0x8CC600;
const RAISE_OFFSET: usize = 0x8CC608;
const SHOT_SHAKE: usize = 0x8CCD18;
const RAISE_PITCH: usize = 0x8CCD1C;

const K_HALF_PI: u32 = 0x3FC9_0FDB;
const K_PI: u32 = 0x4049_0FDB;
const K_2PI: u32 = 0x40C9_0FDB;
const K_MINUS_PI: u32 = 0xC049_0FDB;
const K_QUARTER_PI: u32 = 0x3F49_0FDB;
const K_2_OVER_PI: u32 = 0x3F22_F983;
const K_70: u32 = 0x428C_0000;
const K_100: u32 = 0x42C8_0000;
const K_0_0125: u32 = 0x3C4C_CCCD;
const K_X_SENSITIVITY: u32 = 0x3D92_4925;
const K_Y_SENSITIVITY: u32 = 0x3D2F_8AF9;
const K_0_004: u32 = 0x3B83_126F;
const K_120: u32 = 0x42F0_0000;
const K_0_85: u32 = 0x3F59_999A;
const K_0_01: u32 = 0x3C23_D70A;
const K_ONE: u32 = 0x3F80_0000;
const K_0_9: u32 = 0x3F66_6666;
const K_MINUS_0_9: u32 = 0xBF66_6666;
const K_HALF: u32 = 0x3F00_0000;
const K_MINUS_0_4: u32 = 0xBECC_CCCD;
const K_0_002: u32 = 0x3B03_126F;
const K_BASE_PITCH: u32 = 0xBEB2_B8C3;
const K_TWO: u32 = 0x4000_0000;
const K_SPEED_SQ_MIN: u32 = 0x38D1_B717;
const K_5: u32 = 0x40A0_0000;
const K_3: u32 = 0x4040_0000;
const K_0_96: u32 = 0x3F75_C28F;
const K_PITCH_MIN: u32 = 0xBF86_0A92;
const K_PITCH_MAX: u32 = 0x3F06_0A92;
const K_HELI_PITCH_MAX: u32 = 0x3E32_B8C3;
const K_HALF_DEG2RAD: u32 = 0x3C0E_FA35;

/// Frame offsets (from ESP after the four register pushes).
const P: usize = 0x1C;
const ABS_UD: usize = 0x10;
const LEN: usize = 0x14;
const ABS_LR: usize = 0x18;
const EXCESS: usize = 0x28;
const PAD: usize = 0x2C;
const HEADING: usize = 0x30;
const BASE: usize = 0x34;
const WEAPON_INFO: usize = 0x38;
const L: usize = 0x3C;
const TARGET: usize = 0x48;
const OUT_X: usize = 0x4C;
const OUT_Y: usize = 0x50;
const TASK: usize = 0x90;

/// `v` moved towards `goal` by `(goal - v) * (1 - 0.85^ts)`, the step clamped to `ts * 0.01`.
fn pull_towards(this: GPtr<u8>, off: isize, goal: f32) {
    let ts = g::<f32>(TIME_STEP).read();
    let mut f = Fpu::new();
    f.fld(k(K_0_85));
    f.fld(ts);
    ci_pow(&mut f);
    f.fsubr_m(k(K_ONE));
    f.fld(goal);
    f.fsub_m(rd::<f32>(this, off));
    f.fmulp(1);
    f.fld(ts);
    f.fmul_m(k(K_0_01));
    let mut m = f.fstp();
    f.fcom_m(m);
    if !f.jne(0x41) {
        f.pop();
        f.fld(m);
    } else {
        f.fld(m);
        f.fchs();
        m = f.fstp();
        f.fcom_m(m);
        if !f.jp(5) {
            f.pop();
            f.fld(m);
        }
    }
    f.fadd_m(rd::<f32>(this, off));
    wr(this, off, f.fstp());
}

/// `(x*x + y*y) + z*z` of the vehicle's move speed on the x87 stack.
fn push_speed_sq(f: &mut Fpu, v: GPtr<u8>) {
    f.fld(rd::<f32>(v, 0x4C));
    f.fld(rd::<f32>(v, 0x48));
    f.fld(rd::<f32>(v, 0x44));
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
}

/// `CCam::Process_Cam_TwoPlayer_InCarAndShooting()` (0x519810, thiscall, no arguments). Only for a
/// vehicle target: V = player 0's vehicle, the shooter is the player not driving it (pad 1 / player 1 when
/// player 0 drives). The follow-car FOV (speed bonus, decay to 70, clamp 70..100); the shooter's aim
/// sticks move the screen cursor (+0x108, +0x10C), pulled to the nearest target within twice the weapon
/// range; the cursor beyond +-0.9 turns the camera (x: the excess angle, y: alpha), and with idle sticks
/// and no target it re-centres on (0, -0.4). The camera orbits the vehicle at `len * 0.9 .. len + 2`
/// (`len = 0.5 + 2 |col min y|`, raised by the col model's height, a heli by its up vector), pitched by
/// the travel slope (blended with `0.96^ts`, clamped), then avoids the geometry and is rotated by the
/// excess. Firing (pad's car-gun button, a ranged weapon that is ready, the shooter unable to lean out)
/// builds a stack `CTaskSimpleGangDriveBy` with the shot's quadrant relative to the vehicle heading,
/// fires it through 0x627CC0 and shakes the camera.
#[hook(addr = 0x519810, conv = thiscall, name = "CCam::Process_Cam_TwoPlayer_InCarAndShooting", this = "CCam",
       expect = "6A FF 64 A1 00 00 00 00 68 98 C7 83 00",
       unverified = "frontier2-b34", route = "L2", hazards = "x87-transcendental",
       note = "FPATAN headings, FSIN / FCOS of the excess angle, FPTAN of the half FOV")]
pub fn process_two_player_in_car_and_shooting(this: GPtr<u8>) {
    use sa_core::bindings::{cross_product, get_pad, get_weapon_info, vector_sub};
    use sa_world::bindings::{
        aim_weapon_left_right, aim_weapon_up_down, cam_shake_no_pos, entity_get_col_model, gang_drive_by_ctor,
        gang_drive_by_dtor, gang_drive_by_fire, get_vehicle_appearance, pad_get_car_gun_fired, ped_get_weapon_skill,
        placeable_get_heading, vec_mul_assign, vector_add, vector_add_assign, vector_mul_scalar,
        vehicle_can_ped_lean_out, weapon_find_nearest_target_entity_with_screen_coors, weapon_is_type_melee,
        weapon_update,
    };
    use crate::bindings::get_vectors_ready_for_rw;

    let target_entity = rd::<GPtr<u8>>(this, 0x21C);
    if rd::<u8>(target_entity, 0x36) & 7 != 2 {
        return;
    }
    let ts = || g::<f32>(TIME_STEP).read();
    let fov = || rd::<f32>(this, 0xB4);
    let mut fr = Frame([0u32; 0x35]);
    let mut f = Fpu::new();

    let p0 = g::<GPtr<u8>>(PLAYER0_PED).read();
    let veh = rd::<GPtr<u8>>(p0, 0x58C);
    fr.copy_from(P, pos_of(veh));
    let m = rd::<GPtr<u8>>(veh, 0x14);
    if !m.is_null() {
        f.fld(rd::<f32>(m, 0x14));
        f.fld(rd::<f32>(m, 0x10));
        f.fchs();
        f.fxch(1);
        fpatan(&mut f);
    } else {
        f.fld(rd::<f32>(veh, 0x10));
    }
    f.fsub_m(k(K_HALF_PI));
    let heading = f.fstp();
    fr.s(HEADING, heading);
    let (pad, shooter) = if rd::<GPtr<u8>>(veh, 0x460) == p0 {
        (get_pad(1), g::<GPtr<u8>>(PLAYER1_PED).read())
    } else {
        (get_pad(0), g::<GPtr<u8>>(PLAYER0_PED).read())
    };
    fr.u(PAD, pad.addr() as u32);

    // the follow-car FOV
    let sub_type = rd::<i32>(veh, 0x594);
    if sub_type == 0 || sub_type == 9 {
        let m = rd::<GPtr<u8>>(veh, 0x14);
        let forward_speed = |f: &mut Fpu, x_first: bool| {
            f.fld(rd::<f32>(veh, 0x4C));
            f.fmul_m(rd::<f32>(m, 0x18));
            f.fld(rd::<f32>(veh, 0x48));
            f.fmul_m(rd::<f32>(m, 0x14));
            f.faddp(1);
            if x_first {
                f.fld(rd::<f32>(veh, 0x44));
                f.fmul_m(rd::<f32>(m, 0x10));
            } else {
                f.fld(rd::<f32>(m, 0x10));
                f.fmul_m(rd::<f32>(veh, 0x44));
            }
            f.faddp(1);
        };
        forward_speed(&mut f, true);
        f.fcomp_m(g::<f32>(FOV_SPEED_THRESHOLD).read());
        if !f.jne(0x41) {
            forward_speed(&mut f, false);
            f.fsub_m(g::<f32>(FOV_SPEED_THRESHOLD).read());
            f.fmul_m(ts());
            f.fadd_m(fov());
            wr(this, 0xB4, f.fstp());
        }
    }
    f.fld(fov());
    f.fcomp_m(k(K_70));
    if !f.jne(0x41) {
        f.fld(g::<f32>(FOV_DECAY).read());
        f.fld(ts());
        ci_pow(&mut f);
        f.fld(fov());
        f.fsub_m(k(K_70));
        f.fmul_st(1);
        f.fadd_m(k(K_70));
        wr(this, 0xB4, f.fstp());
        f.pop();
    }
    f.fld(fov());
    f.fcomp_m(k(K_100));
    if !f.jne(0x41) {
        wr::<u32>(this, 0xB4, K_100);
    } else {
        f.fld(fov());
        f.fcomp_m(k(K_70));
        if !f.jp(5) {
            wr::<u32>(this, 0xB4, K_70);
        }
    }

    // the screen cursor
    f.fild(i32::from(aim_weapon_left_right(pad, shooter)));
    let lr = f.fstp();
    f.fild(-i32::from(aim_weapon_up_down(pad, shooter)));
    f.fld(lr);
    f.fabs();
    fr.s(ABS_LR, f.fst());
    f.fld(fov());
    f.fmul_m(k(K_0_0125));
    let sens = f.fst();
    f.fmul_m(k(K_X_SENSITIVITY));
    f.fmul_st(1);
    f.fmul_m(ts());
    f.fmul_m(k(K_0_004));
    f.fmul_m(k(K_0_004));
    f.fmul_m(lr);
    f.fadd_m(rd::<f32>(this, 0x108));
    wr(this, 0x108, f.fstp());
    f.pop();
    f.fld_st(0);
    f.fabs();
    fr.s(ABS_UD, f.fst());
    f.fld(sens);
    f.fmul_m(k(K_Y_SENSITIVITY));
    f.fmul_st(1);
    f.fmul_m(ts());
    f.fmul_m(k(K_0_004));
    f.fmul_m(k(K_0_004));
    f.fmul_st(2);
    f.fadd_m(rd::<f32>(this, 0x10C));
    wr(this, 0x10C, f.fstp());
    f.pop();
    f.pop();

    let weapon = |ped: GPtr<u8>| ped.byte_add(0x5A0 + isize::from(rd::<i8>(ped, 0x718)) * 0x1C);
    let info = get_weapon_info(rd::<i32>(weapon(shooter), 0), ped_get_weapon_skill(shooter));
    fr.u(WEAPON_INFO, info.addr() as u32);
    let sp = pos_of(shooter);
    let range = X87::from_f32(rd::<f32>(info, 8));
    let target = weapon_find_nearest_target_entity_with_screen_coors(
        rd::<f32>(this, 0x108),
        rd::<f32>(this, 0x10C),
        range.fadd(range).to_f32(),
        rd::<f32>(sp, 0),
        rd::<f32>(sp, 4),
        rd::<f32>(sp, 8),
        fr.p(OUT_X).cast(),
        fr.p(OUT_Y).cast(),
    );
    fr.u(TARGET, target.addr() as u32);
    if !target.is_null() {
        f.fld(fr.f(ABS_LR));
        f.fcomp_m(k(K_120));
        let lr_small = !f.jp(5);
        f.fld(fr.f(ABS_UD));
        f.fcomp_m(k(K_120));
        if lr_small && !f.jp(5) {
            pull_towards(this, 0x108, fr.f(OUT_X));
            pull_towards(this, 0x10C, fr.f(OUT_Y));
        }
    }

    // the cursor's excess beyond +-0.9
    fr.u(EXCESS, 0);
    f.fld(rd::<f32>(this, 0x108));
    f.fcomp_m(k(K_0_9));
    if !f.jne(0x41) {
        f.fld(rd::<f32>(this, 0x108));
        wr::<u32>(this, 0x108, K_0_9);
        f.fsub_m(k(K_0_9));
        fr.s(EXCESS, f.fstp());
    }
    f.fld(rd::<f32>(this, 0x108));
    f.fcomp_m(k(K_MINUS_0_9));
    if !f.jp(5) {
        f.fld(rd::<f32>(this, 0x108));
        wr::<u32>(this, 0x108, K_MINUS_0_9);
        f.fadd_m(k(K_0_9));
        fr.s(EXCESS, f.fstp());
    }
    let tilt = |f: &mut Fpu, add: bool, clamped: u32| {
        f.fld(rd::<f32>(this, 0x10C));
        if add {
            f.fadd_m(k(K_0_9));
        } else {
            f.fsub_m(k(K_0_9));
        }
        f.fmul_m(ts());
        wr::<u32>(this, 0x10C, clamped);
        f.fmul_m(k(K_HALF));
        f.fsubr_m(rd::<f32>(this, 0xAC));
        wr(this, 0xAC, f.fstp());
    };
    f.fld(rd::<f32>(this, 0x10C));
    f.fcomp_m(k(K_0_9));
    if !f.jne(0x41) {
        tilt(&mut f, false, K_0_9);
    }
    f.fld(rd::<f32>(this, 0x10C));
    f.fcomp_m(k(K_MINUS_0_9));
    if !f.jp(5) {
        tilt(&mut f, true, K_MINUS_0_9);
    }

    // re-centre with idle sticks and no target
    f.fld(fr.f(ABS_LR));
    f.fcomp_m(k(K_ONE));
    let lr_idle = !f.jp(5);
    f.fld(fr.f(ABS_UD));
    f.fcomp_m(k(K_ONE));
    if lr_idle && !f.jp(5) && target.is_null() {
        f.fld(rd::<f32>(this, 0x108));
        f.fld(rd::<f32>(this, 0x10C));
        f.fsub_m(k(K_MINUS_0_4));
        f.fld_st(0);
        f.fmul_st(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fsqrt();
        f.fld(ts());
        f.fmul_m(k(K_0_002));
        let s = f.fstp();
        fr.s(ABS_LR, s);
        f.fcom_m(s);
        if !f.jp(5) {
            f.pop();
            wr::<u32>(this, 0x108, 0);
            wr::<u32>(this, 0x10C, K_MINUS_0_4);
        } else {
            f.fdivr_m(k(K_ONE));
            f.fld(s);
            f.fmul_st(1);
            f.fmul_st(3);
            f.fsubr_m(rd::<f32>(this, 0x108));
            wr(this, 0x108, f.fstp());
            f.fmul_m(s);
            f.fmul_st(1);
            f.fsubr_m(rd::<f32>(this, 0x10C));
            wr(this, 0x10C, f.fstp());
        }
        f.pop();
        f.pop();
    }

    // the orbit distances
    let max_z = rd::<f32>(entity_get_col_model(veh), 0x14);
    fr.s(ABS_LR, max_z);
    f.fld(rd::<f32>(entity_get_col_model(veh), 4));
    f.fabs();
    f.fadd_st(0);
    f.fld(k(K_HALF));
    fr.u(BASE, K_BASE_PITCH);
    f.fadd_st(1);
    fr.s(LEN, f.fstp());
    f.pop();
    if get_vehicle_appearance(veh) == 3 && rd::<u8>(veh, 0x36) & 0xF8 != 0x40 {
        let m = rd::<GPtr<u8>>(veh, 0x14);
        let raise = || g::<f32>(HELI_RAISE).read();
        f.fld(raise());
        f.fmul_m(rd::<f32>(m, 0x28));
        f.fld(raise());
        f.fmul_m(rd::<f32>(m, 0x24));
        f.fld(raise());
        f.fmul_m(rd::<f32>(m, 0x20));
        fr.s(L, f.fstp());
        f.fxch(1);
        f.fmul_m(max_z);
        let dz = f.fstp();
        f.fmul_m(max_z);
        f.fld(fr.f(L));
        f.fmul_m(max_z);
        f.fadd_m(fr.f(P));
        fr.s(P, f.fstp());
        f.fadd_m(fr.f(P + 4));
        fr.s(P + 4, f.fstp());
        f.fld(dz);
        f.fadd_m(fr.f(P + 8));
        fr.s(P + 8, f.fstp());
    } else {
        f.fld(g::<f32>(RAISE_SCALE).read());
        f.fmul_m(max_z);
        f.fsub_m(g::<f32>(RAISE_OFFSET).read());
        f.fcom_m(0.0);
        if !f.jne(0x41) {
            f.fld(fr.f(P + 8));
            f.fadd_st(1);
            fr.s(P + 8, f.fstp());
            f.fld(fr.f(LEN));
            f.fadd_st(1);
            fr.s(LEN, f.fstp());
            f.fld(g::<f32>(RAISE_PITCH).read());
            f.fdiv_m(fr.f(LEN));
            f.fmul_st(1);
            f.fadd_m(fr.f(BASE));
            fr.s(BASE, f.fstp());
        }
        f.pop();
    }
    f.fld(fr.f(LEN));
    f.fmul_m(k(K_0_9));
    let min_d = f.fstp();
    fr.s(ABS_UD, min_d);
    f.fld(k(K_TWO));
    wr(this, 0xCC, min_d);
    f.fadd_m(fr.f(LEN));
    let max_d = f.fstp();
    fr.s(ABS_LR, max_d);
    wr(this, 0xD0, max_d);

    let src = this.byte_add(0x19C);
    f.fld(rd::<f32>(src, 4));
    f.fsub_m(fr.f(P + 4));
    f.fld(rd::<f32>(src, 0));
    f.fsub_m(fr.f(P));
    let dx = f.fst();
    fr.s(L, dx);
    f.fmul_m(dx);
    f.fld_st(1);
    f.fmul_st(2);
    f.faddp(1);
    f.fsqrt();
    wr(this, 0x70, f.fst());
    if !f.st(0).fcom(X87::from_f64_exact(0.002)).test_parity(5) {
        f.pop();
        f.fld(k(K_0_002));
    }
    f.fld(fr.f(P + 4));
    f.fsub_m(rd::<f32>(src, 4));
    f.fld(fr.f(P));
    f.fsub_m(rd::<f32>(src, 0));
    f.fchs();
    f.fxch(1);
    fpatan(&mut f);
    f.fsub_m(k(K_HALF_PI));
    wr(this, 0xBC, f.fstp());
    f.fcom_m(max_d);
    let scale = if !f.jne(0x41) {
        f.fdivr_m(max_d);
        true
    } else {
        f.fcom_m(min_d);
        if f.jp(5) {
            false
        } else {
            f.fdivr_m(min_d);
            true
        }
    };
    if scale {
        f.fld(fr.f(L));
        f.fmul_st(1);
        f.fadd_m(fr.f(P));
        wr(src, 0, f.fstp());
        f.fmul_st(1);
        f.fadd_m(fr.f(P + 4));
        wr(src, 4, f.fstp());
    } else {
        f.pop();
    }
    f.pop();

    // the pitch from the travel slope
    push_speed_sq(&mut f, veh);
    f.fcomp_m(k(K_SPEED_SQ_MIN));
    f.pop();
    f.pop();
    f.pop();
    if !f.jne(0x41) {
        f.fld(rd::<f32>(veh, 0x48));
        f.fld(rd::<f32>(veh, 0x44));
        f.fld_st(0);
        f.fmul_st(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fstp_st(2);
        f.pop();
        f.fsqrt();
        f.fld(rd::<f32>(veh, 0x4C));
        f.fxch(1);
        fpatan(&mut f);
        f.fld(k(if rd::<i32>(veh, 0x594) == 3 { K_3 } else { K_5 }));
        push_speed_sq(&mut f, veh);
        f.fstp_st(3);
        f.pop();
        f.pop();
        f.fsqrt();
        f.fsub_m(k(K_0_01));
        f.fmulp(1);
        f.fld(k(K_ONE));
        f.fcomp_st(1);
        if !f.jp(5) {
            f.pop();
            f.fld(k(K_ONE));
        }
        f.fmulp(1);
        f.fadd_m(fr.f(BASE));
        fr.s(BASE, f.fstp());
        f.fld(k(K_0_96));
        f.fld(ts());
        ci_pow(&mut f);
        f.fld(k(K_ONE));
        f.fsub_st(1);
        f.fmul_m(fr.f(BASE));
        f.fxch(1);
        f.fmul_m(rd::<f32>(this, 0xAC));
        f.faddp(1);
        wr(this, 0xAC, f.fstp());
    }
    let heli = rd::<i32>(veh, 0x594) == 3;
    let hi = k(if heli { K_HELI_PITCH_MAX } else { K_PITCH_MAX });
    f.fld(k(K_PITCH_MIN));
    f.fcom_m(rd::<f32>(this, 0xAC));
    if !f.jne(0x41) {
        wr(this, 0xAC, f.fstp());
    } else {
        f.pop();
        f.fld(rd::<f32>(this, 0xAC));
        f.fcomp_m(hi);
        if !f.jne(0x41) {
            wr(this, 0xAC, hi);
        }
    }
    f.fld(rd::<f32>(this, 0xAC));
    fsin(&mut f);
    f.fmul_m(max_d);
    f.fsubr_m(fr.f(P + 8));
    wr(src, 8, f.fstp());

    rot_cam_if_in_front_car(this, fr.p(P), heading.to_bits());
    let fudge = this.byte_add(0x148);
    for j in 0..3 {
        wr::<u32>(fudge, 4 * j, fr.0[P / 4 + j as usize]);
    }
    fr.copy_from(L, src);
    avoid_the_geometry(GPtr::from_addr(THE_CAMERA), fr.p(L), fudge, src, rd::<u32>(this, 0xB4));

    // rotate the source about the target by the cursor's excess
    f.fld(rd::<f32>(src, 0));
    f.fsub_m(fr.f(P));
    f.fld(rd::<f32>(src, 4));
    f.fsub_m(fr.f(P + 4));
    f.fld(fr.f(EXCESS));
    fsin(&mut f);
    f.fld_st(0);
    f.fld(fr.f(EXCESS));
    fcos(&mut f);
    let c = f.fstp();
    f.fmul_st(2);
    f.fld(c);
    f.fmul_st(4);
    f.faddp(1);
    let nx = f.fstp();
    f.fld(c);
    f.fmul_st(2);
    f.fxch(1);
    f.fmul_st(3);
    f.fsubp(1);
    f.fstp_st(2);
    f.pop();
    f.fld(nx);
    f.fadd_m(fr.f(P));
    wr(src, 0, f.fstp());
    f.fadd_m(fr.f(P + 4));
    wr(src, 4, f.fstp());

    let front = this.byte_add(0x190);
    for j in (0..3).rev() {
        f.fld(fr.f(P + 4 * j));
        f.fsub_m(rd::<f32>(src, 4 * j as isize));
    }
    wr::<u8>(this, 0xA, 0);
    for j in 0..3 {
        fr.s(P + 4 * j, f.fstp());
        wr::<u32>(front, 4 * j as isize, fr.0[P / 4 + j]);
    }
    get_vectors_ready_for_rw(this);

    // the drive-by shot
    if vehicle_can_ped_lean_out(veh, shooter) == 0 {
        weapon_update(weapon(shooter), shooter);
    }
    if pad_get_car_gun_fired(GPtr::from_addr(fr.0[PAD / 4] as usize)) == 0
        || vehicle_can_ped_lean_out(veh, shooter) != 0
        || weapon_is_type_melee(weapon(shooter)) != 0
        || rd::<u32>(weapon(shooter), 4) != 0
    {
        return;
    }
    if !target.is_null() {
        fr.copy_from(P, pos_of(target));
    } else {
        let up = this.byte_add(0x1B4);
        cross_product(fr.p(L), front, up);
        f.fld(fov());
        f.fmul_m(k(K_HALF_DEG2RAD));
        let half = f.fst();
        fr.s(TARGET, half);
        let tan = |v: X87| v.fptan().map_or(X87::from_f64_exact(f64::NAN), |(t, _)| t);
        let tan_h = tan(f.pop()).to_f32();
        fr.s(HEADING, tan_h);
        let tan_v = tan(X87::from_f32(half))
            .fdiv(X87::from_f32(g::<f32>(ASPECT_RATIO).read()))
            .fmul(X87::from_f32(rd::<f32>(this, 0x10C)))
            .to_f32();
        let t1 = vector_mul_scalar(fr.p(0x84), up, tan_v);
        let t2 = vector_mul_scalar(fr.p(0x6C), fr.p(L), rd::<f32>(this, 0x108));
        let t3 = vector_mul_scalar(fr.p(0x78), t2, tan_h);
        let t4 = vector_add(fr.p(0x60), front, t3);
        let t5 = vector_sub(fr.p(0x54), t4, t1);
        for j in 0..3 {
            fr.u(P + 4 * j, rd::<u32>(t5, 4 * j as isize));
        }
        let info = GPtr::<u8>::from_addr(fr.0[WEAPON_INFO / 4] as usize);
        let reach = X87::from_f32(rd::<f32>(info, 8)).fmul(X87::constant(K_3)).to_f32();
        vec_mul_assign(fr.p(P), reach);
        vector_add_assign(fr.p(P), src);
    }
    fr.0[PAD / 4] = (fr.0[PAD / 4] & !0xFF) | u32::from(shooter != rd::<GPtr<u8>>(veh, 0x468));
    let dy = rd::<u32>(vector_sub(fr.p(0x54), fr.p(P), src), 4);
    fr.u(WEAPON_INFO, dy);
    let d = vector_sub(fr.p(0x60), fr.p(P), src);
    f.fld(rd::<f32>(d, 0));
    f.fchs();
    f.fld(fr.f(WEAPON_INFO));
    fpatan(&mut f);
    fr.s(WEAPON_INFO, f.fstp());
    f.push(X87::from_f64_exact(placeable_get_heading(veh)));
    f.fsubr_m(fr.f(WEAPON_INFO));
    f.fcom_m(k(K_PI));
    if !f.jne(0x41) {
        f.fsub_m(k(K_2PI));
    } else {
        f.fcom_m(k(K_MINUS_PI));
        if !f.jp(5) {
            f.fadd_m(k(K_2PI));
        }
    }
    f.fadd_m(k(K_QUARTER_PI));
    f.fcom_m(0.0);
    if !f.jp(5) {
        f.fadd_m(k(K_2PI));
    }
    f.fmul_m(k(K_2_OVER_PI));
    let quadrant = sa_math::fpu::ftol32(f.pop());

    let task = fr.p(TASK);
    gang_drive_by_ctor(task, GPtr::NULL, GPtr::NULL, K_100, 100, 8, fr.0[PAD / 4]);
    let info = get_weapon_info(rd::<i32>(weapon(shooter), 0), ped_get_weapon_skill(shooter));
    wr(task, 0x30, info);
    wr::<u8>(task, 0x14, quadrant as u8);
    gang_drive_by_fire(task, shooter);
    cam_shake_no_pos(GPtr::from_addr(THE_CAMERA), g::<f32>(SHOT_SHAKE).read());
    gang_drive_by_dtor(task);
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_TWO_PLAYER_IN_CAR_AND_SHOOTING];
