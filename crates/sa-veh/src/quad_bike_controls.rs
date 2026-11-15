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
    /// virtual `CVehicle::ProcessControlInputs(uint8 playerNum)` (vtable +0x60)
    fn v_process_control_inputs(v: GPtr<u8>, player: u32) = thiscall vslot 0x60 => unported;
    /// virtual slot +0x114 of the vehicle (called while cheat 0x969152 is on)
    fn v_slot_114(v: GPtr<u8>) = thiscall vslot 0x114 => unported;
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

/// One lean / steer turn force: `ApplyTurnForce(axis * k, centreOfMass + at)` through the vector helpers the bytes
/// call (`operator+` 0x40FE30 into one temporary, `operator*` 0x40FE90 into another).
fn quad_turn_force(this: GPtr<u8>, axis: GPtr<u8>, k: f32, at: GPtr<u8>) {
    use sa_math::vector::CVector;
    use sa_world::bindings::{physical_apply_turn_force, vector_add, vector_scale};
    let mut p = CVector { x: 0.0, y: 0.0, z: 0.0 };
    let mut f = CVector { x: 0.0, y: 0.0, z: 0.0 };
    vector_add(GPtr::from_raw(&mut p as *mut CVector).cast(), this.byte_add(0xA4), at);
    vector_scale(GPtr::from_raw(&mut f as *mut CVector).cast(), k, axis);
    physical_apply_turn_force(this, f, p);
}

/// `min(|moveSpeed|, 0.1)` as the bytes compute it: the magnitude, and when it is not above 0.1 (or NaN) the
/// magnitude again.
fn quad_speed_capped(this: GPtr<u8>) -> X87 {
    use sa_core::bindings::vector_magnitude;
    let ms = this.byte_add(0x44).cast::<u32>();
    if sa_math::fpu::le_or_nan(X87::from_f64_exact(vector_magnitude(ms)), x(f32::from_bits(0x3DCC_CCCD))) {
        X87::from_f64_exact(vector_magnitude(ms))
    } else {
        x(f32::from_bits(0x3DCC_CCCD))
    }
}

/// `CQuadBike::ProcessAI(uint32& extraHandlingFlags)` (0x6CE460, vtable slot; FRONTIER-2 turn 3 lane B, batch
/// `frontier2-b11`, reached by the user-hit quad). Not player-driven (status != 0): `CAutomobile::ProcessAI`. Else:
/// `GetColModel` (unused), +0x3DB `&= 0xFC`; a vehicle recording without car AI returns false; the centre of mass from
/// the handling data (+0x384 +0x14); without a player driver false. `PruneReferences`; a driver with task 0x329, 0x337
/// or 0x339 or state 0x3F: handbrake (+0x428 bit 5), brake 1, gas 0, false. Otherwise the driver's inputs (vtable
/// +0x60 with its ped type) and pad; the lean (+0x99C) against `dot(turnSpeed, right)`:
/// - lean < 0 and dot < handling +0xC (lean back): F = turnMass * lean * 0.0015 in the air (+0x960 = 0), else, with no
///   brake and no handbrake, handling +0x10 * turnMass * lean * min(|moveSpeed|, 0.1) (0 otherwise);
/// - else lean > 0 and dot > -handling +4 (lean forward): as above with handling +8, the force only while braking;
/// each applies `up * -(timeStep * F)` at `centreOfMass + forward`.
///
/// All four wheels in the air (compression +0x7D4..+0x7E0 exactly 1.0) with a pad: steer s = left/right / 128 (the
/// mouse x * 0.01 clamped to +-1.5 with 3rd-person mouse steering and |s| < 0.05); handbrake: against `dot(turnSpeed,
/// up)` and +-0.03, `right * (timeStep * turnMass * 0.0015 * s)` at `centreOfMass + forward`; no handbrake and not
/// accelerating: against `dot(turnSpeed, forward)`, the same force at `centreOfMass + up`. Then the wheelie angle
/// +0x994 = `p * +0x994 - (1 - p) * handling +0x18 * steer / (lock * pi/180)`, `p = pow(handling +0x1C, timeStep)`;
/// `DoDriveByShootings`; with cheat 0x969152 vtable +0x114. Returns false.
#[hook(addr = 0x6CE460, conv = thiscall, name = "CQuadBike::ProcessAI", this = "CQuadBike", expect = "83 EC 1C 53 55 56 8B F1",
       unverified = "frontier2-b11", route = "L2", hazards = "x87-transcendental,stack-residue",
       note = "pow(handling+0x1C, timeStep) through orig::x87_binary(0x822130); the original keeps the lean force and the steer value in its own flags-pointer argument slot (not modelled)")]
pub fn quad_bike_process_ai(this: GPtr<u8>, flags: GPtr<u8>) -> u8 {
    use crate::common::entity_get_col_model;
    use sa_core::bindings::{prune_references, vector_dot_product};
    use sa_math::fpu::{ge_or_nan, gt, le_or_nan};
    use sa_stream::bindings::ped_is_player;
    use sa_world::bindings::{get_pad_from_player, max_f, min_f, pad_get_accelerate, tm_find_active_task_by_type};
    if rd::<u8>(this, 0x36) & 0xF8 != 0 {
        return crate::trailer_ai::automobile_process_ai(this, flags);
    }
    entity_get_col_model(this);
    wr::<u8>(this, 0x3DB, rd::<u8>(this, 0x3DB) & 0xFC);
    let rec = rd::<i8>(this, 0x424);
    if rec >= 0 && g::<u8>(0x97_D6C0 + rec as usize).read() == 0 {
        return 0;
    }
    let hd = rd::<GPtr<u8>>(this, 0x384);
    for k in 0..3 {
        wr::<u32>(this, 0xA4 + 4 * k, rd::<u32>(hd, 0x14 + 4 * k));
    }
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if driver.is_null() || ped_is_player(driver) == 0 {
        return 0;
    }
    prune_references(this);
    let has = |ty: i32| !tm_find_active_task_by_type(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x460), 0x47C).byte_add(4), ty).is_null();
    if has(0x329) || has(0x337) || has(0x339) || rd::<u32>(rd::<GPtr<u8>>(this, 0x460), 0x530) == 0x3F {
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) | 0x20);
        wr::<u32>(this, 0x4A0, 0x3F80_0000);
        wr::<u32>(this, 0x49C, 0);
        return 0;
    }
    let mut pad = GPtr::<u8>::NULL;
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if !driver.is_null() && ped_is_player(driver) != 0 {
        v_process_control_inputs(this, u32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x460), 0x598)));
        pad = get_pad_from_player(rd::<GPtr<u8>>(this, 0x460));
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let mut force = 0f32;
    let dot = vector_dot_product(this.byte_add(0x50), m);
    let lean = || x(rd::<f32>(this, 0x99C));
    let tm = || rd::<f32>(this, 0x90);
    let lean_force = |off: isize, active: bool| -> f32 {
        if rd::<u8>(this, 0x960) == 0 {
            f32_of(x(tm()).fmul_m(rd::<f32>(this, 0x99C)).fmul_m(f32::from_bits(0x3AC4_9BA6)))
        } else if active {
            let cap = quad_speed_capped(this);
            let hd = rd::<GPtr<u8>>(this, 0x988);
            f32_of(x(rd::<f32>(hd, off)).fmul_m(tm()).fmul_m(rd::<f32>(this, 0x99C)).fmul(cap))
        } else {
            0.0
        }
    };
    let ts = || g::<f32>(0xB7_CB5C).read();
    if !ge_or_nan(lean(), x(0.0)) && !ge_or_nan(dot, x(rd::<f32>(rd::<GPtr<u8>>(this, 0x988), 0xC))) {
        let brake_free = !sa_math::fpu::fcom_parity(x(rd::<f32>(this, 0x4A0)), x(0.0), 0x44) && rd::<u8>(this, 0x428) & 0x20 == 0;
        force = lean_force(0x10, brake_free);
        let k = f32_of(x(ts()).fmul_m(force).fchs());
        quad_turn_force(this, m.byte_add(0x20), k, m.byte_add(0x10));
    } else if !le_or_nan(lean(), x(0.0))
        && !le_or_nan(dot, x(rd::<f32>(rd::<GPtr<u8>>(this, 0x988), 4)).fchs())
    {
        let braking = !le_or_nan(x(rd::<f32>(this, 0x4A0)), x(0.0));
        force = lean_force(8, braking);
        let k = f32_of(x(ts()).fmul_m(force).fchs());
        quad_turn_force(this, m.byte_add(0x20), k, m.byte_add(0x10));
    }
    let _ = force;
    let one = 0x3F80_0000u32;
    if [0x7D4, 0x7D8, 0x7DC, 0x7E0].iter().all(|&o| rd::<u32>(this, o) == one) && !pad.is_null() {
        let mut s = f32_of(xi(i32::from(sa_world::bindings::pad_get_steering_left_right(pad))).fmul_m(f32::from_bits(0x3C00_0000)));
        if g::<u8>(0xB6_EC2E).read() != 0 && !ge_or_nan(x(s).fabs(), x(f32::from_bits(0x3D4C_CCCD))) {
            let v = f32_of(x(g::<f32>(0xB7_3424).read()).fmul_m(f32::from_bits(0x3C23_D70A)));
            let r = f32_of(max_f(-1.5, v));
            s = f32_of(min_f(1.5f32.to_bits(), r.to_bits()));
        }
        let k03 = f32::from_bits(0x3CF5_C28F);
        let turns = |d: X87| {
            if !ge_or_nan(d, x(k03)) && !ge_or_nan(x(s), x(0.0)) {
                true
            } else {
                gt(d, x(k03).fchs()) && gt(x(s), x(0.0))
            }
        };
        let k = || f32_of(x(ts()).fmul_m(tm()).fmul_m(f32::from_bits(0x3AC4_9BA6)).fmul_m(s));
        if rd::<u8>(this, 0x428) & 0x20 != 0 {
            let m = rd::<GPtr<u8>>(this, 0x14);
            if turns(vector_dot_product(this.byte_add(0x50), m.byte_add(0x20))) {
                quad_turn_force(this, m, k(), m.byte_add(0x10));
            }
        } else if pad_get_accelerate(pad) as u16 == 0 {
            let m = rd::<GPtr<u8>>(this, 0x14);
            if turns(vector_dot_product(this.byte_add(0x50), m.byte_add(0x10))) {
                quad_turn_force(this, m, k(), m.byte_add(0x20));
            }
        }
    }
    let hd = rd::<GPtr<u8>>(this, 0x988);
    let p = X87::from_f64_exact(sa_core::orig::x87_binary(0x82_2130, f64::from(rd::<f32>(hd, 0x1C)), f64::from(ts())));
    let a = x(rd::<f32>(hd, 0x18)).fmul_m(rd::<f32>(this, 0x494));
    let lock = x(rd::<f32>(rd::<GPtr<u8>>(this, 0x384), 0xA0)).fmul_m(f32::from_bits(0x3C8E_FA35));
    let q = a.fdiv(lock);
    let r = p.fmul_m(rd::<f32>(this, 0x994)).fsub(x(1.0).fsub(p).fmul(q));
    wr::<f32>(this, 0x994, f32_of(r));
    crate::bindings::do_drive_by_shootings(this);
    if g::<u8>(0x96_9152).read() != 0 {
        v_slot_114(this);
    }
    0
}

/// `CQuadBike::PreRender()` (0x6CEAD0, vtable slot; FRONTIER-2 turn 3 lane B, batch `frontier2-b11`, reached by the
/// user-hit quad): `CAutomobile::PreRender`; the transmission (frame +0x688) from the suspension positions +0x83C /
/// +0x844 and wheel 1's position (`SetTransmissionRotation(..., false)`); with wheel 0's position w the front axles
/// through a stack `CMatrix`: frame +0x684 turned about y by `atan2(+0x838 - w.z, |w.x|)`, frame +0x680 by
/// `-atan2(+0x840 - w.z, |w.x|)` (FPATAN), the handlebars (+0x68C) about z by `0x8D3454 * steer (+0x9A0)`.
#[hook(addr = 0x6CEAD0, conv = thiscall, name = "CQuadBike::PreRender", this = "CQuadBike", expect = "6A FF 68 58 86 84 00",
       unverified = "frontier2-b11", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "FPATAN through sa_math::x87; the CMatrix lives on the Rust stack; writes the frames' RwMatrix; the SEH frame is not reproduced")]
pub fn quad_bike_pre_render(this: GPtr<u8>) {
    use crate::automobile_render::set_transmission_rotation;
    use crate::heli_pre_render::spin;
    use sa_core::bindings::{matrix_set_rotate_y, matrix_set_rotate_z};
    use sa_stream::bindings::get_wheel_posn;
    use sa_world::bindings::matrix_dtor;
    crate::trailer_ai::automobile_pre_render(this);
    let mi = crate::common::model_info(i32::from(rd::<i16>(this, 0x22)));
    let mut mat = [0u32; 0x12];
    let m = sa_core::field::ptr_of(&mut mat);
    let mut w = [0f32; 3];
    let wp = GPtr::from_raw(w.as_mut_ptr()).cast::<u8>();
    get_wheel_posn(mi, 1, wp, 0);
    set_transmission_rotation(this, rd::<GPtr<u8>>(this, 0x688), rd::<f32>(this, 0x83C), rd::<f32>(this, 0x844), w[0], w[1], w[2], 0);
    get_wheel_posn(mi, 0, wp, 0);
    let axle = |susp: isize| x(rd::<f32>(this, susp)).fsub_m(w[2]).fpatan(x(w[0]).fabs());
    let f = rd::<GPtr<u8>>(this, 0x684);
    if !f.is_null() {
        spin(m, f, f32_of(axle(0x838)), matrix_set_rotate_y);
    }
    let f = rd::<GPtr<u8>>(this, 0x680);
    if !f.is_null() {
        spin(m, f, f32_of(axle(0x840).fchs()), matrix_set_rotate_y);
    }
    let f = rd::<GPtr<u8>>(this, 0x68C);
    if !f.is_null() {
        spin(m, f, f32_of(x(g::<f32>(0x8D_3454).read()).fmul_m(rd::<f32>(this, 0x9A0))), matrix_set_rotate_z);
    }
    matrix_dtor(m);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_QUAD_BIKE_PROCESS_CONTROL_INPUTS, HOOK_QUAD_BIKE_PROCESS_CONTROL, HOOK_QUAD_BIKE_PROCESS_AI, HOOK_QUAD_BIKE_PRE_RENDER];
