//! `CCarEnterExit::IsPlayerToQuitCarEnter` (0x64F240, batch `std14-veh-1`; unreversed in gta-reversed). Ported from
//! the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{jp5, jne_mask, rd, x, xi};

orig! {
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn get_pad_from_player(ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPad::MeleeAttackJustDown(bool)` (0x540390)
    fn pad_melee_attack_just_down(pad: GPtr<u8>, b: u32) -> u8 = thiscall 0x540390 => unported;
    /// `CPad::GetPedWalkUpDown()` (0x53FD30)
    fn pad_get_ped_walk_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FD30 => unported;
    /// `CPad::GetPedWalkLeftRight()` (0x53FC90)
    fn pad_get_ped_walk_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FC90 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(x1, y1, x2, y2)` (0x53CBE0, ST0)
    fn radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
}

fn kf(a: usize) -> f32 {
    GPtr::<f32>::from_addr(a).read()
}

/// `CCarEnterExit::IsPlayerToQuitCarEnter(const CPed*, const CVehicle*, int startTime, CTask* subTask)` (0x64F240,
/// cdecl, AL).
///
/// h = the ped's current rotation; for sub-tasks 0xCB, 0xD0, 0x2C0, 0x321..0x325, 0x327, 0x328, 0x32C, 0x330,
/// 0x331, 0x334, 0x33D, h = the vehicle heading -/+ pi/2 (by the side of the ped: the dot of (ped - vehicle) with the
/// vehicle's right vector), + pi when upside down, wrapped into [-pi, pi] (stored as f32 after each step). 0x2C0,
/// 0x322..0x324, 0x331 and 0x334 quit on `MeleeAttackJustDown` (else the stick test); the others only after 500 ms
/// from `startTime`. A burning vehicle always quits; disabled player controls (pad +0x10E) never. The stick test:
/// angle = `GetRadianAngleBetweenPoints(0, 0, -leftRight, upDown)` - the camera heading (0xB6F178), wrapped
/// around h; quit when the stick is past 0.75 (|stick| / 128) and more than pi/4 away from h.
#[hook(addr = 0x64F240, conv = cdecl, name = "CCarEnterExit::IsPlayerToQuitCarEnter", expect = "83 EC 08 53 55 56 8B 74 24 18",
       unverified = "std14-veh-1", route = "L2", hazards = "ax-al-return", recs = "enter-car",
       note = "the heading lives in the ped argument's stack slot in the original")]
pub fn is_player_to_quit_car_enter(ped: GPtr<u8>, veh: GPtr<u8>, start: u32, task: GPtr<u8>) -> u8 {
    let pad = get_pad_from_player(ped);
    let mut h = rd::<f32>(ped, 0x558);
    let mut melee = false;
    if !task.is_null() {
        let ty = v_task_type(task);
        let side = match ty {
            0x2C0 | 0x322..=0x324 | 0x331 | 0x334 => {
                melee = true;
                true
            }
            0xCB | 0xD0 | 0x321 | 0x325 | 0x327 | 0x328 | 0x32C | 0x330 | 0x33D => true,
            _ => false,
        };
        if side {
            h = side_heading(ped, veh);
        }
    }
    if rd::<u32>(veh, 0x490) != 0 {
        return 1;
    }
    if rd::<u16>(pad, 0x10E) != 0 {
        return 0;
    }
    if melee {
        if pad_melee_attack_just_down(pad, 0) != 0 {
            return 1;
        }
    } else {
        let t = GPtr::<u32>::from_addr(0xB7CB84).read().wrapping_sub(start) as i32;
        let mut tv = xi(t);
        if t < 0 {
            tv = tv.fadd_m(kf(0x858C54));
        }
        if jne_mask(tv, x(kf(0x8D2ED8)), 0x41) {
            return 0;
        }
    }
    let a = f32::from(pad_get_ped_walk_up_down(pad));
    let b = f32::from(pad_get_ped_walk_left_right(pad));
    let nb = x(b).fchs().to_f32();
    let ang = X87::from_f64_exact(radian_angle_between_points(0, 0, nb.to_bits(), a.to_bits())).fsub_m(kf(0xB6F178));
    let mag = x(a).fmul_m(a).fadd(x(b).fmul_m(b)).fsqrt().fmul_m(kf(0x858B88));
    let (pi, two_pi) = (kf(0x858CB8), kf(0x858CBC));
    let mut ang = ang;
    if !jne_mask(ang, x(h).fadd_m(pi), 0x41) {
        ang = ang.fsub_m(two_pi);
    } else if !jp5(ang, x(h).fsub_m(pi)) {
        ang = ang.fadd_m(two_pi);
    }
    if jne_mask(mag, x(kf(0x858F34)), 0x41) {
        return 0;
    }
    u8::from(!jne_mask(ang.fsub_m(h).fabs(), x(kf(0x859AB0)), 0x41))
}

/// The heading beside the vehicle (0x64F2A5..0x64F3DB).
fn side_heading(ped: GPtr<u8>, veh: GPtr<u8>) -> f32 {
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let vp = if m.is_null() { veh.byte_add(4) } else { m.byte_add(0x30) };
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    let pp = if pm.is_null() { ped.byte_add(4) } else { pm.byte_add(0x30) };
    let d = |o: isize| x(rd::<f32>(pp, o)).fsub_m(rd::<f32>(vp, o));
    let dot = d(8).fmul_m(rd::<f32>(m, 8)).fadd(d(4).fmul_m(rd::<f32>(m, 4))).fadd(d(0).fmul_m(rd::<f32>(m, 0)));
    let heading = if m.is_null() {
        x(rd::<f32>(veh, 0x10))
    } else {
        x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)))
    };
    let half_pi = kf(0x858FE4);
    let (pi, two_pi, neg_pi) = (kf(0x858CB8), kf(0x858CBC), kf(0x858CC0));
    let mut h = if !jne_mask(dot, x(0.0), 0x41) { heading.fadd_m(half_pi) } else { heading.fsub_m(half_pi) }.to_f32();
    if !jp5(x(rd::<f32>(m, 0x28)), x(0.0)) {
        let s = x(h).fadd_m(pi);
        h = s.to_f32();
        if !jne_mask(s, x(pi), 0x41) {
            h = x(h).fsub_m(two_pi).to_f32();
        }
    }
    if !jne_mask(x(h), x(pi), 0x41) {
        h = x(h).fsub_m(two_pi).to_f32();
    } else if !jp5(x(h), x(neg_pi)) {
        h = x(h).fadd_m(two_pi).to_f32();
    }
    h
}

pub const HOOKS: &[HookReg] = &[HOOK_IS_PLAYER_TO_QUIT_CAR_ENTER];
