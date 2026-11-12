//! PORT-VEH-4 (batch `veh4-3`): `CBike::ProcessControlInputs`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use crate::common::{f32_of, g, jne_mask, rd, wr, x, xi};

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetExitVehicle()` (0x5400D0, AL)
    fn pad_get_exit_vehicle(pad: GPtr<u8>) -> u8 = thiscall 0x5400D0 => unported;
    /// `CPad::GetHandBrake()` (0x540040): int16 in AX
    fn pad_get_hand_brake(pad: GPtr<u8>) -> i16 = thiscall 0x540040 => unported;
    /// `CPad::GetSteeringLeftRight()` (0x53FB80): int16 in AX
    fn pad_get_steering_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FB80 => unported;
    /// `CPad::GetSteeringUpDown()` (0x53FBD0): int16 in AX
    fn pad_get_steering_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FBD0 => unported;
    /// `CPad::GetAccelerate()` (0x5403F0): int16 in AX
    fn pad_get_accelerate(pad: GPtr<u8>) -> i16 = thiscall 0x5403F0 => unported;
    /// `CPad::GetBrake()` (0x540080): int16 in AX
    fn pad_get_brake(pad: GPtr<u8>) -> i16 = thiscall 0x540080 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(n: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPlayerPed::KeepAreaAroundPlayerClear()` (0x60C1E0)
    fn keep_area_around_player_clear(p: GPtr<u8>) = thiscall 0x60C1E0 => unported;
}

/// `CTimer::ms_fTimeStep`.
fn ts() -> f32 {
    g::<f32>(0xB7_CB5C).read()
}

/// `pow(0.98, timeStep)` (`_CIpow`).
fn decay() -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_binary(
        0x822130,
        x(g::<f32>(0x87_130C).read()).to_f64(),
        x(ts()).to_f64(),
    ))
}

/// The [-1, 1] clamp of the original (`v <= -1` or NaN gives -1, `v >= 1` gives 1).
fn clamp1(v: f32) -> f32 {
    if jne_mask(x(v), x(-1.0), 0x41) {
        -1.0
    } else if x(v).fcom(x(1.0)) == Cmp::Less {
        v
    } else {
        1.0
    }
}

/// `v + (-input / 128 - v) * step * k` for the control at `o`.
fn ease(this: GPtr<u8>, o: isize, input: i16, step: f32, k: f32) {
    let v = rd::<f32>(this, o);
    let t = xi(-i32::from(input)).fmul_m(f32::from_bits(0x3C00_0000));
    wr::<f32>(this, o, f32_of(t.fsub_m(v).fmul_m(step).fmul_m(k).fadd_m(v)));
}

/// `CBike::ProcessControlInputs(uint8 pedType)` (0x6BE310).
///
/// The handbrake (+0x428 bit 5) is forced on by the exit button, else follows the handbrake button.
/// Steering (+0x58C) and lean (+0x650) ease toward `-leftright / 128` and `-updown / 128` (by
/// `timeStep * 0.2`), or in mouse steering (0xB6EC2E and 0xC1CC02; the mouse moved, or still moving in
/// mouse mode with the stick centred) take the mouse movement times -0.0035 (while the pad's +0x2C word is
/// 0) and decay by `pow(0.98, timeStep)` below 0.35 or while that word is set; both clamped to [-1, 1].
/// Nearly stopped (`|forward speed| < 0.01`) with both pedals above 150, except on a BMX (+0x594 == 10):
/// both applied (+0x7A8 = 1); otherwise forward motion turns a positive pedal into gas and a negative one
/// into brake, backward motion the reverse. Steer angle (+0x494) `maxSteer * deg2rad * steer * |steer|`
/// (the squared value at 0xC1C804) unless the ped type (+0x424) has a 0x97D6C0 entry of 0. Flag +0x429
/// bit 4: a timed full-gas / no-brake pattern and a +-0.08/-0.03 steer drift. Disabled controls (unless
/// 0x96A8A0 is 2): handbrake, full brake, no gas, the area cleared, speed capped at 0.28.
#[hook(addr = 0x6BE310, conv = thiscall, name = "CBike::ProcessControlInputs", this = "CBike",
       expect = "83 EC 14 53 55 56 8B F1",
       unverified = "veh4-3", route = "L2", hazards = "x87-transcendental",
       recs = "driving",
       note = "pow(0.98, timeStep) through orig::x87_binary(0x822130), FSQRT of the speed; the pad pointer is re-fetched per use, as the original")]
pub fn bike_process_control_inputs(this: GPtr<u8>, ped_type: u32) {
    let id = i32::from(ped_type as u8);
    let m = rd::<GPtr<u8>>(this, 0x14);
    let fwd = f32_of(
        x(rd::<f32>(this, 0x4C))
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(x(rd::<f32>(this, 0x48)).fmul_m(rd::<f32>(m, 0x14)))
            .fadd(x(rd::<f32>(this, 0x44)).fmul_m(rd::<f32>(m, 0x10))),
    );
    if pad_get_exit_vehicle(pad_get_pad(id)) != 0 {
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) | 0x20);
    } else {
        let hb = pad_get_hand_brake(pad_get_pad(id)) != 0;
        let f = rd::<u8>(this, 0x428);
        wr::<u8>(this, 0x428, (f & !0x20) | if hb { 0x20 } else { 0 });
    }
    let k02 = f32::from_bits(0x3E4C_CCCD);
    let stick = |this: GPtr<u8>| {
        let t = ts();
        let lr = pad_get_steering_left_right(pad_get_pad(id));
        ease(this, 0x58C, lr, t, g::<f32>(0x87_1300).read());
        let t = ts();
        let ud = pad_get_steering_up_down(pad_get_pad(id));
        ease(this, 0x650, ud, t, k02);
    };
    let state = g::<i32>(0xC1_CC04);
    if g::<u8>(0xB6_EC2E).read() != 0 && g::<u8>(0xC1_CC02).read() != 0 {
        pad_get_pad(id);
        let mut mouse = x(g::<f32>(0xB7_3424).read()).fcom(x(0.0)).test_parity(0x44);
        if !mouse {
            pad_get_pad(id);
            mouse = x(g::<f32>(0xB7_3428).read()).fcom(x(0.0)).test_parity(0x44);
        }
        if !mouse
            && !jne_mask(x(rd::<f32>(this, 0x58C)).fabs(), x(0.0), 0x41)
            && state.read() == 2
            && pad_get_steering_left_right(pad_get_pad(id)) == 0
            && pad_get_steering_up_down(pad_get_pad(id)) == 0
        {
            mouse = true;
        }
        if mouse {
            state.write(2);
            if rd::<u16>(pad_get_pad(id), 0x2C) == 0 {
                pad_get_pad(id);
                let k = g::<f32>(0x87_1304).read();
                wr::<f32>(this, 0x58C, f32_of(x(g::<f32>(0xB7_3424).read()).fmul_m(k).fadd_m(rd::<f32>(this, 0x58C))));
                pad_get_pad(id);
                wr::<f32>(this, 0x650, f32_of(x(g::<f32>(0xB7_3428).read()).fmul_m(k).fadd_m(rd::<f32>(this, 0x650))));
            }
            let lim = x(g::<f32>(0x87_1308).read());
            for o in [0x58Cisize, 0x650] {
                if x(rd::<f32>(this, o)).fabs().fcom(lim) == Cmp::Less || rd::<u16>(pad_get_pad(id), 0x2C) != 0 {
                    wr::<f32>(this, o, f32_of(decay().fmul_m(rd::<f32>(this, o))));
                }
            }
        } else if pad_get_steering_left_right(pad_get_pad(id)) != 0
            || pad_get_steering_up_down(pad_get_pad(id)) != 0
            || state.read() != 2
        {
            state.write(0);
            stick(this);
        }
    } else {
        stick(this);
    }
    wr::<f32>(this, 0x58C, clamp1(rd::<f32>(this, 0x58C)));
    wr::<f32>(this, 0x650, clamp1(rd::<f32>(this, 0x650)));
    let acc = i32::from(pad_get_accelerate(pad_get_pad(id)));
    let brk = i32::from(pad_get_brake(pad_get_pad(id)));
    let pedal = xi(acc - brk).fmul_m(f32::from_bits(0x3B80_8081));
    let pedal_f = f32_of(pedal);
    let mut a = x(fwd);
    if a.fcom(x(0.0)) == Cmp::Less {
        a = a.fchs();
    }
    if a.fcom(x(f32::from_bits(0x3C23_D70A))) == Cmp::Less {
        let over = |v: i16| !jne_mask(xi(i32::from(v)), x(150.0), 0x41);
        if over(pad_get_accelerate(pad_get_pad(id)))
            && over(pad_get_brake(pad_get_pad(id)))
            && rd::<u32>(this, 0x594) != 10
        {
            let g1 = i32::from(pad_get_accelerate(pad_get_pad(id)));
            wr::<f32>(this, 0x49C, f32_of(xi(g1).fmul_m(f32::from_bits(0x3B80_8081))));
            let b1 = i32::from(pad_get_brake(pad_get_pad(id)));
            wr::<u16>(this, 0x7A8, 1);
            wr::<f32>(this, 0x4A0, f32_of(xi(b1).fmul_m(f32::from_bits(0x3B80_8081))));
        } else {
            wr::<f32>(this, 0x49C, pedal_f);
            wr::<u32>(this, 0x4A0, 0);
        }
    } else {
        let fwd_neg = x(fwd).fcom(x(0.0)).c0();
        let ped_neg = pedal.fcom(x(0.0)).c0();
        if !fwd_neg {
            if !ped_neg {
                wr::<f32>(this, 0x49C, f32_of(pedal));
                wr::<u32>(this, 0x4A0, 0);
            } else {
                wr::<u32>(this, 0x49C, 0);
                wr::<f32>(this, 0x4A0, f32_of(pedal.fchs()));
            }
        } else if !ped_neg {
            wr::<f32>(this, 0x4A0, f32_of(pedal));
            wr::<u32>(this, 0x49C, 0);
        } else {
            wr::<f32>(this, 0x49C, f32_of(pedal));
            wr::<u32>(this, 0x4A0, 0);
        }
    }
    let s = x(rd::<f32>(this, 0x58C));
    let mut sq = s.fmul(s);
    if s.fcom(x(0.0)).c0() {
        sq = sq.fchs();
    }
    g::<f32>(0xC1_C804).write(f32_of(sq));
    let pt = rd::<i8>(this, 0x424);
    if pt < 0 || g::<u8>((0x97_D6C0i64 + i64::from(pt)) as usize).read() != 0 {
        let h = rd::<GPtr<u8>>(this, 0x384);
        let a = x(rd::<f32>(h, 0xA0)).fmul_m(f32::from_bits(0x3C8E_FA35)).fmul_m(g::<f32>(0xC1_C804).read());
        wr::<f32>(this, 0x494, f32_of(a));
    }
    if rd::<u8>(this, 0x429) & 0x10 != 0 {
        let now = || g::<u32>(0xB7_CB84).read();
        if now() & 0x3C00 < 0x3000 {
            wr::<u32>(this, 0x49C, 0x3F80_0000);
        }
        if ((now() >> 10).wrapping_add(6) & 0xF) < 12 {
            wr::<u32>(this, 0x4A0, 0);
        }
        let a = x(rd::<f32>(this, 0x494));
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xDF);
        let a = if now() & 0x800 != 0 { a.fadd_m(f32::from_bits(0x3DA3_D70A)) } else { a.fsub_m(f32::from_bits(0x3CF5_C28F)) };
        wr::<f32>(this, 0x494, f32_of(a));
    }
    if rd::<u16>(pad_get_pad(0), 0x10E) != 0 && g::<i32>(0x96_A8A0).read() != 2 {
        wr::<u32>(this, 0x4A0, 0x3F80_0000);
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) | 0x20);
        wr::<u32>(this, 0x49C, 0);
        keep_area_around_player_clear(find_player_ped(-1));
        let (mx, my, mz) = (x(rd::<f32>(this, 0x44)), x(rd::<f32>(this, 0x48)), x(rd::<f32>(this, 0x4C)));
        let speed = mx.fmul(mx).fadd(my.fmul(my)).fadd(mz.fmul(mz)).fsqrt();
        let cap = g::<f32>(0x87_1254).read();
        if !jne_mask(speed, x(cap), 0x41) {
            let f = speed.fdivr_m(cap);
            let nx = f32_of(f.fmul_m(rd::<f32>(this, 0x44)));
            let ny = f32_of(f.fmul_m(rd::<f32>(this, 0x48)));
            let nz = f32_of(f.fmul_m(rd::<f32>(this, 0x4C)));
            wr::<f32>(this, 0x44, nx);
            wr::<f32>(this, 0x48, ny);
            wr::<f32>(this, 0x4C, nz);
        }
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_BIKE_PROCESS_CONTROL_INPUTS];
