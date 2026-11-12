//! PORT-VEH-4 (batch `veh4-2`): `CQuadBike::ProcessControlInputs`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, jne_mask, jp5, rd, wr, x, xi};

orig! {
    /// `CAutomobile::ProcessControlInputs(uint8 pedType)` (0x6AD690)
    fn automobile_process_control_inputs(v: GPtr<u8>, ped_type: u32) = thiscall 0x6AD690 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetSteeringLeftRight()` (0x53FB80): int16 in AX
    fn pad_get_steering_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FB80 => unported;
    /// `CPad::GetSteeringUpDown()` (0x53FBD0): int16 in AX
    fn pad_get_steering_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FBD0 => unported;
}

/// `CTimer::ms_fTimeStep` (0xB7CB5C).
fn time_step() -> f32 {
    g::<f32>(0xB7_CB5C).read()
}

/// The mouse-steering state (0xC1CC04): 2 while the mouse drives the lean.
fn mouse_state() -> GPtr<i32> {
    g::<i32>(0xC1_CC04)
}

/// `CQuadBike::ProcessControlInputs(uint8 pedType)` (0x6CE020): the automobile inputs, then the lean
/// control +0x99C. With mouse steering enabled (0xB6EC2E and 0xC1CC02) the mouse (Y movement at
/// 0xB73428 times -0.0035, while the pad's +0x2C word is 0) drives it when the mouse moved (either axis
/// non-zero), or when the steering (+0x58C) is non-zero in mouse mode with no stick input but up/down;
/// then the lean decays by `pow(0.98, timeStep)` unless it is at least 0.35 in magnitude and the pad's
/// +0x2C word is 0. Otherwise (and without mouse steering) the stick's up/down (negated) moves it:
/// `lean += (-updown / 128 - lean) * timeStep * 0.2` (skipped when the stick is centred in mouse mode,
/// which is then left). The result is clamped to [-1, 1] (NaN becomes -1) and zeroed while the
/// player's controls are disabled (pad +0x10E).
#[hook(addr = 0x6CE020, conv = thiscall, name = "CQuadBike::ProcessControlInputs", this = "CQuadBike",
       expect = "51 53 8B 5C 24 0C",
       unverified = "veh4-2", route = "L2", hazards = "x87-transcendental",
       recs = "driving",
       note = "pow(0.98, timeStep) through orig::x87_binary(0x822130); the pad pointer is re-fetched per use, as the original")]
pub fn quad_bike_process_control_inputs(this: GPtr<u8>, ped_type: u32) {
    automobile_process_control_inputs(this, ped_type);
    let id = i32::from(ped_type as u8);
    let lean = |v: f32| wr::<f32>(this, 0x99C, v);
    let cur = || rd::<f32>(this, 0x99C);
    'clamp: {
        let stick = if g::<u8>(0xB6_EC2E).read() != 0 && g::<u8>(0xC1_CC02).read() != 0 {
            pad_get_pad(id);
            let mut mouse = x(g::<f32>(0xB7_3424).read()).fcom(x(0.0)).test_parity(0x44);
            if !mouse {
                pad_get_pad(id);
                mouse = x(g::<f32>(0xB7_3428).read()).fcom(x(0.0)).test_parity(0x44);
            }
            if !mouse
                && !jne_mask(x(rd::<f32>(this, 0x58C)).fabs(), x(0.0), 0x41)
                && mouse_state().read() == 2
                && pad_get_steering_left_right(pad_get_pad(id)) == 0
                && pad_get_steering_up_down(pad_get_pad(id)) == 0
            {
                mouse = true;
            }
            if mouse {
                mouse_state().write(2);
                if rd::<u16>(pad_get_pad(id), 0x2C) == 0 {
                    pad_get_pad(id);
                    lean(f32_of(x(g::<f32>(0xB7_3428).read()).fmul_m(f32::from_bits(0xBB65_6042)).fadd_m(cur())));
                }
                if !jp5(x(cur()).fabs(), x(f32::from_bits(0x3EB3_3333))) || rd::<u16>(pad_get_pad(id), 0x2C) != 0 {
                    let p = X87::from_f64_exact(sa_core::orig::x87_binary(
                        0x822130,
                        x(f32::from_bits(0x3F7A_E148)).to_f64(),
                        x(time_step()).to_f64(),
                    ));
                    lean(f32_of(p.fmul_m(cur())));
                }
                break 'clamp;
            }
            if pad_get_steering_up_down(pad_get_pad(id)) == 0 && mouse_state().read() == 2 {
                break 'clamp;
            }
            let ts = time_step();
            mouse_state().write(0);
            (ts, pad_get_steering_up_down(pad_get_pad(id)))
        } else {
            let ts = time_step();
            (ts, pad_get_steering_up_down(pad_get_pad(id)))
        };
        let (ts, ud) = stick;
        let v = xi(-i32::from(ud))
            .fmul_m(f32::from_bits(0x3C00_0000))
            .fsub_m(cur())
            .fmul_m(ts)
            .fmul_m(f32::from_bits(0x3E4C_CCCD))
            .fadd_m(cur());
        lean(f32_of(v));
    }
    let v = cur();
    let out = if !jne_mask(x(v), x(-1.0), 0x41) && jp5(x(v), x(1.0)) {
        1.0
    } else if jne_mask(x(v), x(-1.0), 0x41) {
        -1.0
    } else {
        v
    };
    lean(out);
    if rd::<u16>(pad_get_pad(id), 0x10E) != 0 {
        wr::<u32>(this, 0x99C, 0);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_QUAD_BIKE_PROCESS_CONTROL_INPUTS];
