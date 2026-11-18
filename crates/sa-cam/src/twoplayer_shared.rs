//! `CCam::Process_Cam_TwoPlayer` (0x525E50): the shared two-player camera that keeps both players on
//! foot (or both in their own vehicles) in view, or follows the one player in focus.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::common::{Fpu, g, k, rd, wr};
use crate::followped::{ci_pow, fpatan};
use sa_world::world::garage_update::now;
use crate::twoplayer::{PLAYER0_PED, PLAYER1_PED, CUR_CAM_COL_VARS, pos_of, process_line_of_sight, vector_normalise};

sa_core::orig! {
    /// `FUN_005132d0(float heading, CVector* outSource, CVector* outFront, CVector* outTarget)` (0x5132D0)
    fn two_player_vectors(c: GPtr<u8>, heading: u32, src: GPtr<u8>, front: GPtr<u8>, target: GPtr<u8>) = thiscall 0x5132D0 => crate::twoplayer::two_player_vectors;
    /// `FUN_00513220(CVector point)` (0x513220, the vector by value; ECX is never read)
    fn two_player_point_sees_both(x: u32, y: u32, z: u32) -> u8 = stdcall 0x513220 => crate::twoplayer::two_player_point_sees_both;
}

/// `CGameLogic::n2PlayerPedInFocus` (0 / 1: that player alone, 2: both).
const PED_IN_FOCUS: usize = 0x8A5E50;
const TIME_STEP: usize = 0xB7CB5C;
const LAST_TIME_CAMERA_OK: usize = 0xB6EC24;
const LAST_TIME_CAMERA_COLLIDED: usize = 0xB6EC28;
const PRINT_FOCUS_HELP_TIMER: usize = 0x96A8B4;
const PRINT_FOCUS_HELP_COUNTER: usize = 0x96A8B8;
const THE_TEXT: usize = 0xC1B340;
/// "WRN2_2P" in `.rdata`.
const KEY_WRN2_2P: usize = 0x859884;
/// `.data` tunables, read at run time: 0.8, 0.1, 0.1, 0.02, 0.007.
const BETA_SPEED_DECAY: usize = 0x8CC5E0;
const BETA_SPEED_LIMIT: usize = 0x8CC5E4;
const DRIFT_RATE: usize = 0x8CC5E8;
const DRIFT_LIMIT: usize = 0x8CC5EC;
const STEER_RATE: usize = 0x8CC4A0;

const K_0_15: u32 = 0x3E19_999A;
const K_PI: u32 = 0x4049_0FDB;
const K_2PI: u32 = 0x40C9_0FDB;
const K_MINUS_PI: u32 = 0xC049_0FDB;
const K_HALF_PI: u32 = 0x3FC9_0FDB;
const K_0_2: u32 = 0x3E4C_CCCD;
const K_0_1: u32 = 0x3DCC_CCCD;
const K_0_01: u32 = 0x3C23_D70A;
const K_ONE: u32 = 0x3F80_0000;
const K_128: u32 = 0x4300_0000;
const K_MINUS_128: u32 = 0xC300_0000;
const K_0_0125: u32 = 0x3C4C_CCCD;
const K_1_OVER_14: u32 = 0x3D92_4925;
const ALPHA_SHARED: u32 = 0xBF06_0A92;
const FOV_SHARED: u32 = 0x428C_0000;

/// One player in focus: his vehicle through `Process_FollowCar_SA` (the target entity switched to it for
/// the call, then player 0's ped), else his ped through `Process_FollowPed_SA` (player 1's ped as the
/// target for that call only when he is the one in focus).
fn follow_one_player(this: GPtr<u8>, ped_cell: usize) {
    use sa_world::bindings::{cam_follow_car_sa, cam_follow_ped_sa};
    let ped = g::<GPtr<u8>>(ped_cell).read();
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 && !veh.is_null() {
        wr(this, 0x21C, veh);
        cam_follow_car_sa(this, pos_of(veh), 0, 0, 0, 0);
        wr(this, 0x21C, g::<GPtr<u8>>(PLAYER0_PED).read());
    } else if ped_cell == PLAYER1_PED {
        wr(this, 0x21C, ped);
        cam_follow_ped_sa(this, pos_of(g::<GPtr<u8>>(PLAYER1_PED).read()), 0, 0, 0, 0);
        wr(this, 0x21C, g::<GPtr<u8>>(PLAYER0_PED).read());
    } else {
        cam_follow_ped_sa(this, pos_of(ped), 0, 0, 0, 0);
    }
    wr::<u8>(this, 0xA, 0);
}

/// Candidate source / front for `heading` into the frame's locals, and whether both players are visible
/// from that source.
fn try_heading(this: GPtr<u8>, heading: f32, src: &mut [u32; 3], front: &mut [u32; 3]) -> bool {
    let sp = GPtr::<u32>::from_raw(src.as_mut_ptr()).cast::<u8>();
    let fp = GPtr::<u32>::from_raw(front.as_mut_ptr()).cast::<u8>();
    two_player_vectors(this, heading.to_bits(), sp, fp, this.byte_add(0x148));
    two_player_point_sees_both(src[0], src[1], src[2]) != 0
}

/// After a blocked correction: the beta speed (+0xC0) is zeroed when it has the correction's sign.
fn stop_turning_into_the_wall(this: GPtr<u8>, t: f32) {
    let mut f = Fpu::new();
    let bs = || rd::<f32>(this, 0xC0);
    f.fld(t);
    f.fcomp_m(0.0);
    let reset = if !f.jne(0x41) {
        f.fld(bs());
        f.fcomp_m(0.0);
        !f.jne(0x41) || {
            f.fld(t);
            f.fcomp_m(0.0);
            !f.jp(5) && {
                f.fld(bs());
                f.fcomp_m(0.0);
                !f.jp(5)
            }
        }
    } else {
        f.fld(t);
        f.fcomp_m(0.0);
        !f.jp(5) && {
            f.fld(bs());
            f.fcomp_m(0.0);
            !f.jp(5)
        }
    };
    if reset {
        wr::<u32>(this, 0xC0, 0);
    }
    g::<u32>(LAST_TIME_CAMERA_COLLIDED).write(now());
}

/// `cand += 0.15` when `t > 0.01`, `cand -= 0.15` when `t < 0.01`; then, when `|t| > 0.01`, the
/// candidate is tried and a blocked one zeroes `t`.
fn nudge_candidate(this: GPtr<u8>, cand: &mut f32, t: &mut f32, src: &mut [u32; 3], front: &mut [u32; 3]) {
    let mut f = Fpu::new();
    f.fld(*t);
    f.fcomp_m(k(K_0_01));
    if !f.jne(0x41) {
        f.fld(*cand);
        f.fadd_m(k(K_0_15));
        *cand = f.fstp();
    } else {
        f.fld(*t);
        f.fcomp_m(k(K_0_01));
        if !f.jp(5) {
            f.fld(*cand);
            f.fsub_m(k(K_0_15));
            *cand = f.fstp();
        }
    }
    f.fld(*t);
    f.fabs();
    f.fcomp_m(k(K_0_01));
    if f.jne(0x41) {
        return;
    }
    if !try_heading(this, *cand, src, front) {
        stop_turning_into_the_wall(this, *t);
        *t = 0.0;
    }
}

/// `v` wrapped into `(ref - pi, ref + pi]` by one 2 pi step, on the x87 stack.
fn wrap_around(f: &mut Fpu, reference: f32) {
    f.fld(reference);
    f.fadd_m(k(K_PI));
    f.fld_st(1);
    f.fcompp();
    if !f.jne(0x41) {
        f.fsub_m(k(K_2PI));
        return;
    }
    f.fld(reference);
    f.fsub_m(k(K_PI));
    f.fld_st(1);
    f.fcompp();
    if !f.jp(5) {
        f.fadd_m(k(K_2PI));
    }
}

/// `x` clamped to `[-lim, lim]` on the x87 stack (`-lim` rounded through the `newB` slot).
fn clamp_by(f: &mut Fpu, lim: f32, slot: &mut f32) {
    f.fcom_m(lim);
    if !f.jne(0x41) {
        f.pop();
        f.fld(lim);
        return;
    }
    f.fld(lim);
    f.fchs();
    *slot = f.fstp();
    f.fcom_m(*slot);
    if !f.jp(5) {
        f.pop();
        f.fld(*slot);
    }
}

/// `CCam::Process_Cam_TwoPlayer()` (0x525E50, thiscall, no arguments). Either player's focus button
/// (pad 0: 0 <-> 2, pad 1: 1 <-> 2) picks the player in focus; one player in focus is followed alone.
/// Both: alpha -30 degrees; up to 21 headings around beta (beta -/+ k * 0.15) are tried until both players
/// are visible from the candidate source (the times of the last success / collision recorded). The
/// target heading moves at most 0.1 per frame; with no collision for a second it drifts by the players'
/// summed velocity heading and by both players' aim sticks, each nudge tried and dropped when it blocks
/// (the beta speed zeroed when it turns the same way). beta speed = clamped rate blended with `0.8^ts`;
/// beta += ts * beta speed; the camera vectors come from 0x5132D0. With no heading clear for 500 ms the
/// source is pulled in to the first hit and "WRN2_2P" is shown (at most 6 times, a minute apart).
/// FOV 70, up from front.
#[hook(addr = 0x525E50, conv = thiscall, name = "CCam::Process_Cam_TwoPlayer", this = "CCam",
       expect = "83 EC 74 53 55 56 57 33 DB",
       unverified = "frontier2-b34", route = "L2", hazards = "x87-transcendental",
       note = "FPATAN of the summed velocity, _CIpow of the time step")]
pub fn process_two_player(this: GPtr<u8>) {
    use crate::camcontrol::pad_cycle_camera_down;
    use sa_core::bindings::{cross_product, get_pad, hud_set_help_message, text_get, vector_magnitude};
    use sa_world::bindings::aim_weapon_left_right;

    let focus = g::<i32>(PED_IN_FOCUS);
    if pad_cycle_camera_down(get_pad(0)) != 0 {
        focus.write(if focus.read() != 0 { 0 } else { 2 });
    } else if pad_cycle_camera_down(get_pad(1)) != 0 {
        focus.write(if focus.read() == 1 { 2 } else { 1 });
    }
    match focus.read() {
        0 => return follow_one_player(this, PLAYER0_PED),
        1 => return follow_one_player(this, PLAYER1_PED),
        _ => {}
    }

    let ts = || g::<f32>(TIME_STEP).read();
    let beta = || rd::<f32>(this, 0xBC);
    let mut src = [0u32; 3];
    let mut front = [0u32; 3];
    let mut f = Fpu::new();

    wr::<u32>(this, 0xAC, ALPHA_SHARED);
    wr::<u32>(this, 0xB0, 0);
    let mut cand = 0f32;
    let mut i = 0i32;
    while i < 21 {
        let step = (i + 1) / 2;
        f.fild(step);
        f.fmul_m(k(K_0_15));
        if i & 1 != 0 {
            f.fadd_m(beta());
        } else {
            f.fsubr_m(beta());
        }
        cand = f.fstp();
        if try_heading(this, cand, &mut src, &mut front) {
            g::<u32>(LAST_TIME_CAMERA_OK).write(now());
            break;
        }
        i += 1;
    }
    let clear = i == 0;
    if i == 21 {
        cand = beta();
    }
    if i > 0 {
        g::<u32>(LAST_TIME_CAMERA_COLLIDED).write(now());
    }
    if rd::<u8>(this, 0xA) != 0 {
        wr(this, 0xBC, cand);
    }

    f.fld(cand);
    f.fld(beta());
    f.fadd_m(k(K_PI));
    f.fcomp_m(cand);
    if !f.jp(5) {
        f.pop();
        f.fld(cand);
        f.fsub_m(k(K_2PI));
    } else {
        f.fld(beta());
        f.fsub_m(k(K_PI));
        f.fcomp_m(cand);
        if !f.jne(0x41) {
            f.pop();
            f.fld(cand);
            f.fadd_m(k(K_2PI));
        }
    }
    f.fsub_m(beta());
    f.fmul_m(k(K_0_2));
    let mut slot = 0f32;
    clamp_by(&mut f, k(K_0_1), &mut slot);
    f.fadd_m(beta());
    let mut t = 0f32;
    let new_beta = f.fstp();

    let settled = || clear && now() >= g::<u32>(LAST_TIME_CAMERA_COLLIDED).read().wrapping_add(1000);
    if settled() {
        let p0 = g::<GPtr<u8>>(PLAYER0_PED).read();
        let p1 = g::<GPtr<u8>>(PLAYER1_PED).read();
        let mut v = [0f32; 3];
        for (j, c) in v.iter_mut().enumerate() {
            let o = 0x44 + 4 * j as isize;
            f.fld(rd::<f32>(p0, o));
            f.fadd_m(rd::<f32>(p1, o));
            *c = if j < 2 { f.fstp() } else { f.fst() };
        }
        f.fmul_m(v[2]);
        f.fld(v[0]);
        f.fmul_m(v[0]);
        f.faddp(1);
        f.fld(v[1]);
        f.fmul_m(v[1]);
        f.faddp(1);
        f.fcom_m(k(K_0_01));
        if f.jne(0x41) {
            f.pop();
            f.fld(cand);
            f.fsub_m(k(K_0_15));
            cand = f.fstp();
        } else {
            f.fld(g::<f32>(DRIFT_RATE).read());
            f.fmul_m(ts());
            let rate = f.fstp();
            f.fld(g::<f32>(DRIFT_LIMIT).read());
            f.fmul_m(ts());
            let lim = f.fstp();
            f.fld(v[0]);
            f.fchs();
            f.fld(v[1]);
            fpatan(&mut f);
            f.fsub_m(k(K_HALF_PI));
            let mut h = f.fst();
            f.fsub_m(new_beta);
            f.fcom_m(k(K_PI));
            if !f.jne(0x41) {
                f.pop();
                f.fld(h);
                f.fsub_m(k(K_2PI));
                h = f.fstp();
            } else {
                f.fcomp_m(k(K_MINUS_PI));
                if !f.jp(5) {
                    f.fld(h);
                    f.fadd_m(k(K_2PI));
                    h = f.fstp();
                }
            }
            f.fsqrt();
            f.fmul_m(rate);
            f.fcomp_m(k(K_ONE));
            if !f.jne(0x41) {
                f.fld(k(K_ONE));
            } else {
                let vp = GPtr::<f32>::from_raw(v.as_mut_ptr()).cast::<u32>();
                f.push(sa_math::fpu::st0(vector_magnitude(vp)));
                f.fmul_m(rate);
            }
            f.fld(h);
            f.fsub_m(new_beta);
            f.fmul_st(1);
            t = f.fstp();
            f.pop();
            f.fld(t);
            f.fcomp_m(lim);
            if !f.jne(0x41) {
                t = lim;
            } else {
                f.fld(lim);
                f.fchs();
                f.fld(t);
                f.fcomp_st(1);
                if f.jp(5) {
                    f.pop();
                } else {
                    t = f.fstp();
                }
            }
            nudge_candidate(this, &mut cand, &mut t, &mut src, &mut front);
        }
    }

    f.fld(t);
    f.fadd_m(new_beta);
    wrap_around(&mut f, beta());
    f.fld(k(K_ONE));
    f.fcomp_m(ts());
    if f.jne(0x41) {
        f.fld(ts());
    } else {
        f.fld(k(K_ONE));
    }
    f.fxch(1);
    f.fsub_m(beta());
    f.fdiv_st(1);
    let rate = f.fstp();
    f.pop();
    f.fld(g::<f32>(BETA_SPEED_DECAY).read());
    f.fld(ts());
    ci_pow(&mut f);
    let lim = g::<f32>(BETA_SPEED_LIMIT).read();
    let p = f.fstp();

    let mut slot = rate;
    if settled() {
        let a0 = -i32::from(aim_weapon_left_right(get_pad(0), g::<GPtr<u8>>(PLAYER0_PED).read()));
        let p1 = g::<GPtr<u8>>(PLAYER1_PED).read();
        f.fild(a0);
        let a0 = f.fstp();
        let a1 = i32::from(aim_weapon_left_right(get_pad(1), p1));
        f.fild(a1);
        f.fsubr_m(a0);
        f.fcom_m(k(K_128));
        if !f.jne(0x41) {
            f.pop();
            f.fld(k(K_128));
        } else {
            f.fcom_m(k(K_MINUS_128));
            if !f.jp(5) {
                f.pop();
                f.fld(k(K_MINUS_128));
            }
        }
        f.fld_st(0);
        f.fabs();
        f.fld(rd::<f32>(this, 0xB4));
        f.fmul_m(k(K_0_0125));
        f.fmul_m(k(K_1_OVER_14));
        f.fmulp(1);
        f.fmul_m(g::<f32>(STEER_RATE).read());
        f.fmul_m(g::<f32>(STEER_RATE).read());
        f.fmul_st(1);
        t = f.fstp();
        f.pop();
        nudge_candidate(this, &mut cand, &mut t, &mut src, &mut front);
        f.fld(t);
        f.fadd_m(rate);
    } else {
        f.fld(rate);
    }
    clamp_by(&mut f, lim, &mut slot);

    f.fld(k(K_ONE));
    f.fsub_m(p);
    f.fmulp(1);
    f.fld(p);
    f.fmul_m(rd::<f32>(this, 0xC0));
    f.faddp(1);
    wr(this, 0xC0, f.fst());
    f.fmul_m(ts());
    f.fadd_m(beta());
    let b = f.fstp();
    wr(this, 0xBC, b);
    two_player_vectors(this, b.to_bits(), this.byte_add(0x19C), this.byte_add(0x190), this.byte_add(0x148));

    if i == 21 && now().wrapping_sub(g::<u32>(LAST_TIME_CAMERA_OK).read()) > 500 {
        let mut hit = GPtr::<u8>::NULL;
        let mut cp = [0u32; 11];
        g::<u8>(CUR_CAM_COL_VARS).write(5);
        let source = this.byte_add(0x19C);
        if process_line_of_sight(
            this.byte_add(0x148),
            source,
            GPtr::<u32>::from_raw(cp.as_mut_ptr()).cast::<u8>(),
            GPtr::<GPtr<u8>>::from_raw(&mut hit).cast::<u8>(),
            1, 0, 0, 0, 0, 1, 1, 0,
        ) != 0
        {
            for (j, w) in cp[..3].iter().enumerate() {
                wr::<u32>(source, 4 * j as isize, *w);
            }
        }
        if now() > g::<u32>(PRINT_FOCUS_HELP_TIMER).read() && g::<u32>(PRINT_FOCUS_HELP_COUNTER).read() < 6 {
            let text = text_get(GPtr::from_addr(THE_TEXT), GPtr::from_addr(KEY_WRN2_2P));
            hud_set_help_message(text, 0, 0, 0);
            g::<u32>(PRINT_FOCUS_HELP_TIMER).write(now().wrapping_add(60_000));
            let c = g::<u32>(PRINT_FOCUS_HELP_COUNTER);
            c.write(c.read().wrapping_add(1));
        }
    }

    let up = this.byte_add(0x1B4);
    let front = this.byte_add(0x190);
    wr::<u32>(up, 0, 0);
    wr::<u32>(up, 4, 0);
    wr::<u32>(up, 8, K_ONE);
    vector_normalise(up.cast());
    let mut right = [0u32; 3];
    let rp = GPtr::<u32>::from_raw(right.as_mut_ptr()).cast::<u8>();
    cross_product(rp, up, front);
    vector_normalise(rp.cast());
    let mut tmp = [0u32; 3];
    let r = cross_product(GPtr::<u32>::from_raw(tmp.as_mut_ptr()).cast::<u8>(), front, rp);
    for j in 0..3 {
        wr::<u32>(up, 4 * j, rd::<u32>(r, 4 * j));
    }
    vector_normalise(up.cast());
    wr::<u32>(this, 0xB4, FOV_SHARED);
    wr::<u8>(this, 0xA, 0);
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_TWO_PLAYER];
