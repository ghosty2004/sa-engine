//! PORT-VEH-4 (batch `veh4-3`): `CPlane::ProcessControlInputs`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).
//!
//! The body's protection detour (`nop; jmp 0x406AF1` at 0x6CB4F3) is the relocated
//! `cmp word [esi + 0x22], 0x208` and jumps straight back to 0x6CB4F9.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use crate::common::{f32_of, ftol32, g, jne_mask, jp5, rd, wr, x, xi};

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetSteeringLeftRight()` (0x53FB80): int16 in AX
    fn pad_get_steering_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FB80 => unported;
    /// `CPad::GetSteeringUpDown()` (0x53FBD0): int16 in AX
    fn pad_get_steering_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FBD0 => unported;
    /// `CPad::GetCarGunUpDown()` (0x53FC10): int16 in AX
    fn pad_get_car_gun_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FC10 => unported;
    /// `CPad::GetLookLeft()` (0x53FDD0, AL)
    fn pad_get_look_left(pad: GPtr<u8>) -> u8 = thiscall 0x53FDD0 => unported;
    /// `CPad::GetLookRight()` (0x53FE10, AL)
    fn pad_get_look_right(pad: GPtr<u8>) -> u8 = thiscall 0x53FE10 => unported;
    /// `CPad::GetHandBrake()` (0x540040): int16 in AX
    fn pad_get_hand_brake(pad: GPtr<u8>) -> i16 = thiscall 0x540040 => unported;
    /// `CPad::GetAccelerate()` (0x5403F0): int16 in AX
    fn pad_get_accelerate(pad: GPtr<u8>) -> i16 = thiscall 0x5403F0 => unported;
    /// `CPad::GetBrake()` (0x540080): int16 in AX
    fn pad_get_brake(pad: GPtr<u8>) -> i16 = thiscall 0x540080 => unported;
    /// `CPlane::SetGearUp()` (0x6CAC20)
    fn plane_set_gear_up(p: GPtr<u8>) = thiscall 0x6CAC20 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(n: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPlayerPed::KeepAreaAroundPlayerClear()` (0x60C1E0)
    fn keep_area_around_player_clear(p: GPtr<u8>) = thiscall 0x60C1E0 => unported;
}

/// `CTimer::ms_fTimeStep`.
fn ts() -> f32 {
    g::<f32>(0xB7_CB5C).read()
}

/// `pow(0.99, timeStep)` (`_CIpow`).
fn decay() -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_binary(
        0x822130,
        x(g::<f32>(0x87_1910).read()).to_f64(),
        x(ts()).to_f64(),
    ))
}

/// `v + (target - v) * step * 0.2` for the control at `o`.
fn ease(this: GPtr<u8>, o: isize, target: X87, step: f32) {
    let v = rd::<f32>(this, o);
    wr::<f32>(this, o, f32_of(target.fsub_m(v).fmul_m(step).fmul_m(f32::from_bits(0x3E4C_CCCD)).fadd_m(v)));
}

/// The [-1, 1] clamp of the original (`v > -1 && v >= 1` gives 1, `v <= -1` or NaN gives -1).
fn clamp1(v: X87) -> f32 {
    if !jne_mask(v, x(-1.0), 0x41) && jp5(v, x(1.0)) {
        1.0
    } else if jne_mask(v, x(-1.0), 0x41) {
        -1.0
    } else {
        f32_of(v)
    }
}

/// `sqrt((x*x + y*y) + z*z)` of the move speed.
fn speed(this: GPtr<u8>) -> X87 {
    let (mx, my, mz) = (x(rd::<f32>(this, 0x44)), x(rd::<f32>(this, 0x48)), x(rd::<f32>(this, 0x4C)));
    mx.fmul(mx).fadd(my.fmul(my)).fadd(mz.fmul(mz)).fsqrt()
}

/// `CPlane::ProcessControlInputs(uint8 pedType)` (0x6CADD0).
///
/// Pitch (+0x990) and yaw (+0x98C) ease toward the stick (`leftright / 128`, `updown / 128`, by
/// `timeStep * 0.2`) or, in mouse flying (0xB6EC2E and 0xC1CC03, the mouse moved or the stick centred
/// while in mouse mode), add the mouse movement times 0.0015 (while the pad's +0x2C word is 0) and decay by
/// `pow(0.99, timeStep)` below 0.15; both clamped to [-1, 1]. Roll (+0x988) eases toward the stick
/// (Vortex, 539, clamped) or toward the look buttons (right 1, left -1), the latter scaled by the time
/// step twice (the second factor is the local the stick path stored the time step in). Steering
/// (+0x494) is `-(maxSteer * deg2rad * roll * max(1 - forwardSpeed, 0.1))`, x1.5 with the Vortex's
/// handbrake (roll too). Throttle (+0x994) `(accel - brake) / 255`. The landing gear (+0x9CC) is toggled
/// by a fresh left-shoulder press with all four suspensions extended (planes whose flying handling
/// +0x30 is below 1, not the RC Baron) and otherwise animates (retracting from 0 to 1 at 0.02 per
/// step: `SetGearUp` at 1; extending from -1 to 0, then the wheels back and the air resistance from
/// the handling). The Hydra's thrust angle (+0x86C, previous at +0x86E) moves by the gun up/down
/// (`ftol(updown * timeStep * 25 / 128)`, floored at 0, capped at 0x8D33C4). Handbrake and gas are
/// cleared; the brake comes from the brake button while moving forward (`/510` above 0.35 forward
/// speed, else `/255`), else 0.5 when nearly stopped with both pedals under 10, else 0. While the
/// player's controls are disabled: handbrake on, full brake, the area cleared and the speed capped at
/// 0.28.
#[hook(addr = 0x6CADD0, conv = thiscall, name = "CPlane::ProcessControlInputs", this = "CPlane",
       expect = "83 EC 14 53 55 56",
       unverified = "veh4-3", route = "L2", hazards = "x87-transcendental,uninit-locals",
       note = "pow(0.99, timeStep) through orig::x87_binary(0x822130); FSQRT of the speed; in mouse flying the look-button roll multiplies by a local the original never wrote on that path (modelled as 0.0, the SPLIT_STACKFILL value)")]
pub fn plane_process_control_inputs(this: GPtr<u8>, ped_type: u32) {
    let m = rd::<GPtr<u8>>(this, 0x14);
    let fwd = f32_of(
        x(rd::<f32>(this, 0x4C))
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(x(rd::<f32>(this, 0x48)).fmul_m(rd::<f32>(m, 0x14)))
            .fadd(x(rd::<f32>(this, 0x44)).fmul_m(rd::<f32>(m, 0x10))),
    );
    let id = i32::from(ped_type as u8);
    let state = g::<i32>(0xC1_CC04);
    let mut step_local = 0.0f32;
    let stick = |this: GPtr<u8>, step_local: &mut f32| {
        *step_local = ts();
        state.write(0);
        let lr = pad_get_steering_left_right(pad_get_pad(id));
        ease(this, 0x990, xi(i32::from(lr)).fmul_m(f32::from_bits(0x3C00_0000)), *step_local);
        *step_local = ts();
        let ud = pad_get_steering_up_down(pad_get_pad(id));
        ease(this, 0x98C, xi(i32::from(ud)).fmul_m(f32::from_bits(0x3C00_0000)), *step_local);
    };
    if g::<u8>(0xB6_EC2E).read() != 0 && g::<u8>(0xC1_CC03).read() != 0 {
        pad_get_pad(id);
        let mut mouse = x(g::<f32>(0xB7_3424).read()).fcom(x(0.0)).test_parity(0x44);
        if !mouse {
            pad_get_pad(id);
            mouse = x(g::<f32>(0xB7_3428).read()).fcom(x(0.0)).test_parity(0x44);
        }
        if !mouse {
            let moving = !jne_mask(x(rd::<f32>(this, 0x990)).fabs(), x(0.0), 0x41)
                || !jne_mask(x(rd::<f32>(this, 0x98C)).fabs(), x(0.0), 0x41);
            if moving
                && state.read() == 2
                && pad_get_steering_left_right(pad_get_pad(id)) == 0
                && pad_get_steering_up_down(pad_get_pad(id)) == 0
            {
                mouse = true;
            }
        }
        if mouse {
            state.write(2);
            if rd::<u16>(pad_get_pad(id), 0x2C) == 0 {
                pad_get_pad(id);
                let k = g::<f32>(0x87_1908).read();
                wr::<f32>(this, 0x990, f32_of(x(g::<f32>(0xB7_3424).read()).fmul_m(k).fadd_m(rd::<f32>(this, 0x990))));
                pad_get_pad(id);
                wr::<f32>(this, 0x98C, f32_of(x(g::<f32>(0xB7_3428).read()).fmul_m(k).fadd_m(rd::<f32>(this, 0x98C))));
            }
            let lim = x(g::<f32>(0x87_190C).read());
            if !jp5(x(rd::<f32>(this, 0x990)).fabs(), lim) {
                wr::<f32>(this, 0x990, f32_of(decay().fmul_m(rd::<f32>(this, 0x990))));
            }
            if !jp5(x(rd::<f32>(this, 0x98C)).fabs(), lim) {
                wr::<f32>(this, 0x98C, f32_of(decay().fmul_m(rd::<f32>(this, 0x98C))));
            }
        } else if pad_get_steering_left_right(pad_get_pad(id)) != 0
            || pad_get_steering_up_down(pad_get_pad(id)) != 0
            || state.read() != 2
        {
            stick(this, &mut step_local);
        }
    } else {
        stick(this, &mut step_local);
    }
    wr::<f32>(this, 0x990, clamp1(x(rd::<f32>(this, 0x990))));
    wr::<f32>(this, 0x98C, clamp1(x(rd::<f32>(this, 0x98C))));
    let vortex = rd::<i16>(this, 0x22) == 0x21B;
    let roll = rd::<f32>(this, 0x988);
    let eased = |target: X87, extra: f32, step_local: f32| {
        target
            .fsub_m(roll)
            .fmul_m(extra)
            .fmul_m(step_local)
            .fmul_m(f32::from_bits(0x3E4C_CCCD))
            .fadd_m(roll)
    };
    let r = if vortex {
        step_local = ts();
        let lr = pad_get_steering_left_right(pad_get_pad(id));
        let v = xi(i32::from(lr)).fmul_m(f32::from_bits(0x3C00_0000)).fsub_m(roll).fmul_m(step_local);
        let v = v.fmul_m(f32::from_bits(0x3E4C_CCCD)).fadd_m(roll);
        wr::<f32>(this, 0x988, f32_of(v));
        clamp1(v)
    } else {
        let right = f32_of(xi(i32::from(pad_get_look_right(pad_get_pad(id)))));
        let v = if pad_get_look_left(pad_get_pad(id)) != 0 { eased(x(-1.0), ts(), step_local) } else { eased(x(right), ts(), step_local) };
        wr::<f32>(this, 0x988, f32_of(v));
        clamp1(v)
    };
    wr::<f32>(this, 0x988, r);
    let mut t = x(1.0).fsub_m(fwd);
    if !jne_mask(x(f32::from_bits(0x3DCC_CCCD)), t, 0x41) {
        t = x(f32::from_bits(0x3DCC_CCCD));
    }
    let h = rd::<GPtr<u8>>(this, 0x384);
    let steer = x(rd::<f32>(h, 0xA0)).fmul_m(f32::from_bits(0x3C8E_FA35)).fmul_m(rd::<f32>(this, 0x988)).fmul(t).fchs();
    wr::<f32>(this, 0x494, f32_of(steer));
    if vortex && pad_get_hand_brake(pad_get_pad(id)) != 0 {
        wr::<f32>(this, 0x494, f32_of(x(rd::<f32>(this, 0x494)).fmul_m(1.5)));
        wr::<f32>(this, 0x988, f32_of(x(rd::<f32>(this, 0x988)).fmul_m(1.5)));
    }
    let acc = i32::from(pad_get_accelerate(pad_get_pad(id)));
    let brk = i32::from(pad_get_brake(pad_get_pad(id)));
    wr::<f32>(this, 0x994, f32_of(xi(acc - brk).fmul_m(f32::from_bits(0x3B80_8081))));
    gear(this, id);
    if rd::<i16>(this, 0x22) == 0x208 {
        thrust(this, id);
    }
    wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xDF);
    wr::<u32>(this, 0x49C, 0);
    let mut braked = false;
    if !jne_mask(x(fwd), x(0.0), 0x41) && pad_get_brake(pad_get_pad(id)) != 0 {
        let b = i32::from(pad_get_brake(pad_get_pad(id)));
        let k = if !jne_mask(x(fwd), x(f32::from_bits(0x3EB3_3333)), 0x41) { 0x3B00_8081 } else { 0x3B80_8081 };
        wr::<f32>(this, 0x4A0, f32_of(xi(b).fmul_m(f32::from_bits(k))));
        braked = true;
    }
    if !braked {
        let slow = speed(this).fcom(x(f32::from_bits(0x3DCC_CCCD))) == Cmp::Less;
        let idle = slow && pad_get_brake(pad_get_pad(id)) < 10 && pad_get_accelerate(pad_get_pad(id)) < 10;
        wr::<u32>(this, 0x4A0, if idle { 0x3F00_0000 } else { 0 });
    }
    if rd::<u16>(pad_get_pad(id), 0x10E) != 0 {
        wr::<u32>(this, 0x4A0, 0x3F80_0000);
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) | 0x20);
        wr::<u32>(this, 0x49C, 0);
        keep_area_around_player_clear(find_player_ped(-1));
        let s = speed(this);
        let cap = g::<f32>(0x87_1254).read();
        if !jne_mask(s, x(cap), 0x41) {
            let f = s.fdivr_m(cap);
            let nx = f32_of(f.fmul_m(rd::<f32>(this, 0x44)));
            let ny = f32_of(f.fmul_m(rd::<f32>(this, 0x48)));
            let nz = f32_of(f.fmul_m(rd::<f32>(this, 0x4C)));
            wr::<f32>(this, 0x44, nx);
            wr::<f32>(this, 0x48, ny);
            wr::<f32>(this, 0x4C, nz);
        }
    }
}

/// The landing gear (0x6CB321..0x6CB4F3).
fn gear(this: GPtr<u8>, id: i32) {
    let pad = pad_get_pad(id);
    let one = 0x3F80_0000u32;
    let rate = || x(ts()).fmul_m(f32::from_bits(0x3CA3_D70A));
    let set_wheels = |v: u8| {
        for o in 0x5A5isize..=0x5A8 {
            wr::<u8>(this, o, v);
        }
    };
    let toggle = rd::<u16>(pad, 0x26) != 0
        && rd::<u16>(pad, 0x56) == 0
        && [0x7D4isize, 0x7D8, 0x7DC, 0x7E0].iter().all(|&o| rd::<u32>(this, o) == one)
        && !jp5(x(rd::<f32>(rd::<GPtr<u8>>(this, 0x388), 0x30)), x(1.0))
        && rd::<i16>(this, 0x22) != 0x1D0;
    let g9 = rd::<f32>(this, 0x9CC);
    if toggle {
        if !x(g9).fcom(x(0.0)).test_parity(0x44) {
            set_wheels(2);
            wr::<f32>(this, 0x9CC, f32_of(rate().fadd_m(rd::<f32>(this, 0x9CC))));
        } else if rd::<u32>(this, 0x9CC) == one {
            wr::<f32>(this, 0x9CC, f32_of(rate().fsub_m(1.0)));
        } else {
            wr::<f32>(this, 0x9CC, f32_of(x(g9).fmul_m(-1.0)));
        }
        return;
    }
    if x(g9).fcom(x(0.0)) == Cmp::Less {
        let v = rate().fadd_m(rd::<f32>(this, 0x9CC));
        wr::<f32>(this, 0x9CC, f32_of(v));
        if !v.fcom(x(0.0)).c0() {
            let h = rd::<GPtr<u8>>(this, 0x384);
            wr::<u32>(this, 0x9CC, 0);
            let air = x(rd::<f32>(h, 0x10)).fdiv_m(1000.0);
            set_wheels(0);
            wr::<f32>(this, 0x98, f32_of(air.fmul_m(0.5)));
        }
    } else if !jne_mask(x(g9), x(0.0), 0x41) && !jp5(x(rd::<f32>(this, 0x9CC)), x(1.0)) {
        let v = rate().fadd_m(rd::<f32>(this, 0x9CC));
        wr::<f32>(this, 0x9CC, f32_of(v));
        if !v.fcom(x(1.0)).c0() {
            plane_set_gear_up(this);
        }
    }
}

/// The Hydra's thrust angle (0x6CB4FF..0x6CB5EA).
fn thrust(this: GPtr<u8>, id: i32) {
    if !jne_mask(xi(i32::from(pad_get_car_gun_up_down(pad_get_pad(id)))).fabs(), x(10.0), 0x41) {
        wr::<u16>(this, 0x86E, rd::<u16>(this, 0x86C));
        let delta = || {
            let ud = pad_get_car_gun_up_down(pad_get_pad(id));
            let d = ftol32(xi(i32::from(ud)).fmul_m(ts()).fmul_m(g::<f32>(0x8D_33DC).read()).fmul_m(f32::from_bits(0x3C00_0000)));
            i32::from(d as i16) + i32::from(rd::<u16>(this, 0x86C))
        };
        let v = if delta() < 0 { 0 } else { delta() };
        wr::<u16>(this, 0x86C, v as u16);
        let cap = g::<i16>(0x8D_33C4).read();
        if i32::from(v as u16) > i32::from(cap) {
            wr::<i16>(this, 0x86C, cap);
        }
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PLANE_PROCESS_CONTROL_INPUTS];
