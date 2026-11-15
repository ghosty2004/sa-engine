//! PORT-VEH-4 (batch `veh4-2`): `CQuadBike::ProcessControlInputs`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, jne_mask, jp5, rd, wr, x, xi};

use sa_input::bindings::get_pad as pad_get_pad;
use sa_world::bindings::pad_get_steering_left_right;
use sa_world::bindings::pad_get_steering_up_down;
orig! {
    /// `CAutomobile::ProcessControlInputs(uint8 pedType)` (0x6AD690)
    fn automobile_process_control_inputs(v: GPtr<u8>, ped_type: u32) = thiscall 0x6AD690 => unported;
}

use sa_pop::carctrl3::time_step;
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
            pad_get_pad((id as u32) as i32);
            let mut mouse = x(g::<f32>(0xB7_3424).read()).fcom(x(0.0)).test_parity(0x44);
            if !mouse {
                pad_get_pad((id as u32) as i32);
                mouse = x(g::<f32>(0xB7_3428).read()).fcom(x(0.0)).test_parity(0x44);
            }
            if !mouse
                && !jne_mask(x(rd::<f32>(this, 0x58C)).fabs(), x(0.0), 0x41)
                && mouse_state().read() == 2
                && pad_get_steering_left_right(pad_get_pad((id as u32) as i32)) == 0
                && pad_get_steering_up_down(pad_get_pad((id as u32) as i32)) == 0
            {
                mouse = true;
            }
            if mouse {
                mouse_state().write(2);
                if rd::<u16>(pad_get_pad((id as u32) as i32), 0x2C) == 0 {
                    pad_get_pad((id as u32) as i32);
                    lean(f32_of(x(g::<f32>(0xB7_3428).read()).fmul_m(f32::from_bits(0xBB65_6042)).fadd_m(cur())));
                }
                if !jp5(x(cur()).fabs(), x(f32::from_bits(0x3EB3_3333))) || rd::<u16>(pad_get_pad((id as u32) as i32), 0x2C) != 0 {
                    let p = X87::from_f64_exact(sa_core::orig::x87_binary(
                        0x822130,
                        x(f32::from_bits(0x3F7A_E148)).to_f64(),
                        x(time_step()).to_f64(),
                    ));
                    lean(f32_of(p.fmul_m(cur())));
                }
                break 'clamp;
            }
            if pad_get_steering_up_down(pad_get_pad((id as u32) as i32)) == 0 && mouse_state().read() == 2 {
                break 'clamp;
            }
            let ts = time_step();
            mouse_state().write(0);
            (ts, pad_get_steering_up_down(pad_get_pad((id as u32) as i32)))
        } else {
            let ts = time_step();
            (ts, pad_get_steering_up_down(pad_get_pad((id as u32) as i32)))
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
    if rd::<u16>(pad_get_pad((id as u32) as i32), 0x10E) != 0 {
        wr::<u32>(this, 0x99C, 0);
    }
}

/// `CQuadBike::ProcessControl()` (0x6CDCC0, vtable slot; FRONTIER-2 turn 3 lane B, batch `frontier2-b11`, user-hit on
/// a SA:MP server: every iteration blocked with a quad nearby). Ported from the bytes (gta-reversed calls the
/// original). Only for a player-driven quad (status 0) while the damping flag 0x8D3450 is set; otherwise (and
/// afterwards) `CAutomobile::ProcessControl`. The turn speed in the quad's frame `t` (`FUN_0059C810`); the resistance
/// pair (a, b) and the y weight w (default (0.995 x, 0.5), w 1.0; vecQuadResistance 0x8D3458):
/// - front wheel counts +0x7D4 and +0x7DC both exactly 1.0 (bit compares): b = 0.995 (0x871ABC); when +0x7D8 or +0x7E0
///   is below 1.0 and the forward z is positive, a = res.x - min(|handling.+0x2C - fwd.z| * 0.25, 0.07); otherwise
///   a = 0.995 and w = 1.0 (0x871AC8);
/// - else rear counts +0x7F8 and +0x800 both 1.0: b = 0.995; forward z negative: a = (min(|handling.+0x30 - fwd.z| *
///   0.3, 0.1) + 0.9) * res.x, else a = res.x;
/// - else a = res.x, b = 0.5.
///
/// `q1 = a / (t.x^2 * b + 1)`, `q2 = res.y / (t.y^2 * w + 1)`, both to the power of the time step (`_CIpow`);
/// `d1 = t.x * q1' - t.x`, `d2 = t.y * q2' - t.y`. Turn force `-up * d2 * turnMass` at `right + M * centreOfMass`, then
/// `up * d1 * turnMass` at `forward + M * centreOfMass` (the f32 stores between the products as the bytes do).
#[hook(addr = 0x6CDCC0, conv = thiscall, name = "CQuadBike::ProcessControl", this = "CQuadBike", expect = "83 EC 38 53 56 8B F1",
       unverified = "frontier2-b11", route = "L2", hazards = "x87-transcendental",
       note = "user-hit (SA:MP web build); two pow(x, timeStep) through orig::x87_binary(0x822130); then CAutomobile::ProcessControl")]
pub fn quad_bike_process_control(this: GPtr<u8>) {
    use sa_math::fpu::{ge_or_nan, le_or_nan};
    use sa_math::vector::CVector;
    use sa_world::bindings::{multiply3x3, multiply3x3_vm, physical_apply_turn_force};
    if rd::<u8>(this, 0x36) & 0xF8 == 0 && g::<u8>(0x8D_3450).read() != 0 {
        let res_x = g::<f32>(0x8D_3458).read();
        let res_y = g::<f32>(0x8D_345C).read();
        let mut w = g::<f32>(0x87_1AC4).read();
        let mut t = [0f32; 3];
        multiply3x3_vm(GPtr::from_raw(t.as_mut_ptr()).cast(), this.byte_add(0x50), rd::<GPtr<u8>>(this, 0x14));
        let one = 0x3F80_0000u32;
        let k995 = g::<f32>(0x87_1ABC).read();
        let fwd_z = || x(rd::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x18));
        let (a, b) = if rd::<u32>(this, 0x7D4) == one && rd::<u32>(this, 0x7DC) == one {
            let lifted = !ge_or_nan(x(rd::<f32>(this, 0x7D8)), x(1.0)) || !ge_or_nan(x(rd::<f32>(this, 0x7E0)), x(1.0));
            if lifted && !le_or_nan(fwd_z(), x(0.0)) {
                let hd = rd::<GPtr<u8>>(this, 0x988);
                let v = x(rd::<f32>(hd, 0x2C)).fsub(fwd_z()).fabs().fmul_m(0.25);
                let k = f32::from_bits(0x3D8F_5C29);
                let m = if ge_or_nan(x(k), v) { v } else { x(k) };
                (m.fsubr_m(res_x), x(k995))
            } else {
                w = g::<f32>(0x87_1AC8).read();
                (x(k995), x(k995))
            }
        } else if rd::<u32>(this, 0x7F8) == one && rd::<u32>(this, 0x800) == one {
            if ge_or_nan(fwd_z(), x(0.0)) {
                (x(res_x), x(k995))
            } else {
                let hd = rd::<GPtr<u8>>(this, 0x988);
                let v = x(rd::<f32>(hd, 0x30)).fsub(fwd_z()).fabs().fmul_m(f32::from_bits(0x3E99_999A));
                let k = f32::from_bits(0x3DCC_CCCD);
                let m = if ge_or_nan(x(k), v) { v } else { x(k) };
                (m.fadd_m(f32::from_bits(0x3F66_6666)).fmul_m(res_x), x(k995))
            }
        } else {
            (x(res_x), x(g::<f32>(0x87_1AC0).read()))
        };
        let q1 = f32_of(x(t[0]).fmul_m(t[0]).fmul(b).fadd_m(1.0).fdivr(a));
        let q2 = f32_of(x(t[1]).fmul_m(t[1]).fmul_m(w).fadd_m(1.0).fdivr_m(res_y));
        let ts = time_step();
        let p1 = f32_of(X87::from_f64_exact(sa_core::orig::x87_binary(0x82_2130, f64::from(q1), f64::from(ts))));
        let p2 = X87::from_f64_exact(sa_core::orig::x87_binary(0x82_2130, f64::from(q2), f64::from(ts)));
        let d1 = f32_of(x(t[0]).fmul_m(p1).fsub_m(t[0]));
        let d2 = p2.fmul_m(t[1]).fsub_m(t[1]);
        let m = rd::<GPtr<u8>>(this, 0x14);
        let tm = rd::<f32>(this, 0x90);
        let up = |k: isize| rd::<f32>(m, 0x20 + 4 * k);
        let nz = f32_of(x(up(2)).fmul_m(-1.0));
        let fx = f32_of(x(f32_of(x(up(0)).fmul_m(-1.0).fmul(d2))).fmul_m(tm));
        let fy = f32_of(x(up(1)).fmul_m(-1.0).fmul(d2).fmul_m(tm));
        let fz = f32_of(x(f32_of(x(nz).fmul(d2))).fmul_m(tm));
        let mut out = [0f32; 3];
        let com = this.byte_add(0xA4);
        multiply3x3(GPtr::from_raw(out.as_mut_ptr()).cast(), m, com);
        let pt = CVector {
            x: f32_of(x(out[0]).fadd_m(rd::<f32>(m, 0))),
            y: f32_of(x(out[1]).fadd_m(rd::<f32>(m, 4))),
            z: f32_of(x(out[2]).fadd_m(rd::<f32>(m, 8))),
        };
        physical_apply_turn_force(this, CVector { x: fx, y: fy, z: fz }, pt);
        let m = rd::<GPtr<u8>>(this, 0x14);
        let tm = rd::<f32>(this, 0x90);
        let gz = f32_of(x(d1).fmul_m(rd::<f32>(m, 0x28)));
        let gx = f32_of(x(d1).fmul_m(rd::<f32>(m, 0x20)).fmul_m(tm));
        let gy = f32_of(x(d1).fmul_m(rd::<f32>(m, 0x24)).fmul_m(tm));
        let gz = f32_of(x(gz).fmul_m(tm));
        let mut out = [0f32; 3];
        multiply3x3(GPtr::from_raw(out.as_mut_ptr()).cast(), m, com);
        let pt = CVector {
            x: f32_of(x(rd::<f32>(m, 0x10)).fadd_m(out[0])),
            y: f32_of(x(out[1]).fadd_m(rd::<f32>(m, 0x14))),
            z: f32_of(x(out[2]).fadd_m(rd::<f32>(m, 0x18))),
        };
        physical_apply_turn_force(this, CVector { x: gx, y: gy, z: gz }, pt);
    }
    crate::bindings::automobile_process_control(this);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_QUAD_BIKE_PROCESS_CONTROL_INPUTS, HOOK_QUAD_BIKE_PROCESS_CONTROL];
