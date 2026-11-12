//! PORT-VEH-3 (batch `veh3-3`, SA:MP-executed rows): `CCarEnterExit::IsPlayerToQuitCarEnter`, the
//! test that lets the player abort getting into a car. Ported from the 1.0 US bytes; gta-reversed as
//! a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, x, xi};

orig! {
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// virtual `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPad::MeleeAttackJustDown(bool)` (0x540390)
    fn pad_melee_attack_just_down(pad: GPtr<u8>, b: u32) -> u8 = thiscall 0x540390 => unported;
    /// `CPad::GetPedWalkUpDown()` (0x53FD30): int16 in AX
    fn pad_get_ped_walk_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FD30 => unported;
    /// `CPad::GetPedWalkLeftRight()` (0x53FC90): int16 in AX
    fn pad_get_ped_walk_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FC90 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float, float, float, float)` (0x53CBE0), ST0
    fn get_radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CBE0 => unported;
}

const PI: u32 = 0x4049_0FDB;
const TWO_PI: u32 = 0x40C9_0FDB;
const HALF_PI: u32 = 0x3FC9_0FDB;

fn f(b: u32) -> f32 {
    f32::from_bits(b)
}

/// The task kinds the test applies to: `Some(melee)` (melee = the attack button quits instead of the
/// stick timeout), `None` for any other task.
fn kind(t: i32) -> Option<bool> {
    match t {
        203 | 208 | 0x321 | 0x325 | 0x327 | 0x328 | 0x32C | 0x330 | 0x33D => Some(false),
        704 | 0x322..=0x324 | 0x331 | 0x334 => Some(true),
        _ => None,
    }
}

/// `CCarEnterExit::IsPlayerToQuitCarEnter(CPed* ped, CVehicle* veh, int32 startTime, CTask* task)`
/// (0x64F240): true when the vehicle is gone wrecked/locked (+0x490), else (player controls enabled)
/// for the enter/open/align tasks: the melee button (melee kinds), or after 500 ms the walk stick
/// pushed (|stick| / 128 > 0.75) more than 45 degrees (camera-relative) away from the direction to
/// the car side the ped is on (the car heading ± 90 degrees, +180 upside down, wrapped to ±pi; the
/// ped's +0x558 for other tasks).
#[hook(addr = 0x64F240, conv = cdecl, name = "CCarEnterExit::IsPlayerToQuitCarEnter",
       expect = "83 EC 08 53 55",
       unverified = "veh3-3", route = "L2", hazards = "x87-transcendental",
       note = "fpatan/fsqrt; the original keeps the angle and the stick values in its argument slots")]
pub fn is_player_to_quit_car_enter(ped: GPtr<u8>, veh: GPtr<u8>, start: u32, task: GPtr<u8>) -> u8 {
    let pad = get_pad_from_player(ped);
    let mut a = rd::<f32>(ped, 0x558);
    let mut melee = false;
    let k = if task.is_null() { None } else { kind(v_task_type(task)) };
    {
        if let Some(m) = k {
            melee = m;
            let mm = rd::<GPtr<u8>>(veh, 0x14);
            let vp = if mm.is_null() { veh.byte_add(4) } else { mm.byte_add(0x30) };
            let pm = rd::<GPtr<u8>>(ped, 0x14);
            let pp = if pm.is_null() { ped.byte_add(4) } else { pm.byte_add(0x30) };
            let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(vp, 0));
            let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(vp, 4));
            let dz = x(rd::<f32>(pp, 8)).fsub_m(rd::<f32>(vp, 8));
            let dot = dz.fmul_m(rd::<f32>(mm, 8)).fadd(dy.fmul_m(rd::<f32>(mm, 4))).fadd(dx.fmul_m(rd::<f32>(mm, 0)));
            let heading = if mm.is_null() {
                x(rd::<f32>(veh, 0x10))
            } else {
                x(rd::<f32>(mm, 0x10)).fchs().fpatan(x(rd::<f32>(mm, 0x14)))
            };
            a = if !dot.fcom(x(0.0)).test_any(0x41) {
                heading.fadd_m(f(HALF_PI)).to_f32()
            } else {
                heading.fsub_m(f(HALF_PI)).to_f32()
            };
            // up.z < 0: + pi, and one 2pi reduction when above pi; then (both cases) one more
            // reduction above pi, which skips the low wrap
            let mut wrap_low = true;
            if !x(rd::<f32>(mm, 0x28)).fcom(x(0.0)).test_parity(0x05) {
                let v = x(a).fadd_m(f(PI));
                a = v.to_f32();
                if !v.fcom(x(f(PI))).test_any(0x41) {
                    a = x(a).fsub_m(f(TWO_PI)).to_f32();
                    if !x(a).fcom(x(f(PI))).test_any(0x41) {
                        a = x(a).fsub_m(f(TWO_PI)).to_f32();
                        wrap_low = false;
                    }
                }
            } else if !x(a).fcom(x(f(PI))).test_any(0x41) {
                a = x(a).fsub_m(f(TWO_PI)).to_f32();
                wrap_low = false;
            }
            if wrap_low && !x(a).fcom(x(f(0xC049_0FDB))).test_parity(0x05) {
                // jp not taken: below -pi
                a = x(a).fadd_m(f(TWO_PI)).to_f32();
            }
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
        let dt = g::<u32>(0xB7_CB84).read().wrapping_sub(start);
        let mut df = xi(dt as i32);
        if (dt as i32) < 0 {
            df = df.fadd_m(4_294_967_296.0);
        }
        if df.fcom(x(500.0)).test_any(0x41) {
            return 0;
        }
    }
    let ud = xi(i32::from(pad_get_ped_walk_up_down(pad))).to_f32();
    let lr = xi(i32::from(pad_get_ped_walk_left_right(pad))).to_f32();
    let mut ang = X87::from_f64_exact(get_radian_angle_between_points(0.0, 0.0, x(lr).fchs().to_f32(), ud))
        .fsub_m(g::<f32>(0xB6_F178).read());
    let mag = x(ud).fmul_m(ud).fadd(x(lr).fmul_m(lr)).fsqrt().fmul_m(0.0078125);
    if !ang.fcom(x(a).fadd_m(f(PI))).test_any(0x41) {
        ang = ang.fsub_m(f(TWO_PI));
    } else if !ang.fcom(x(a).fsub_m(f(PI))).test_parity(0x05) {
        ang = ang.fadd_m(f(TWO_PI));
    }
    if mag.fcom(x(0.75)).test_any(0x41) {
        return 0;
    }
    u8::from(!ang.fsub_m(a).fabs().fcom(x(f(0x3F49_0FDB))).test_any(0x41))
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_IS_PLAYER_TO_QUIT_CAR_ENTER];
