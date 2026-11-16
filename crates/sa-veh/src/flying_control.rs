//! FRONTIER-2 turn 5, lane B (batch `frontier2-b14`): `CVehicle::FlyingControl` (0x6D85F0), the `roadblocks`
//! blocker (the police heli) and the flight model of every plane, heli, autogyro and RC vehicle. Ported from the 1.0 US
//! bytes (the body continues at 0x6D85F6 behind the `nop; jmp 0x403CE8` protection entry, which does the
//! `sub esp, 0x250`); gta-reversed's reading was used as an aid only (its operand orders and several constants
//! differ). The body is split into the original's per-flight-model blocks of its jump table (0x6DAEE4 / 0x6DAEF4):
//! model 0 [`dodo`], 1 / 3 / 4 / 5 [`plane`], 2 / 6 / 8 [`heli`], 7 none, then the shared turn / move resistance
//! tail [`resistance`]. Unverified (docs/verify/HOWTO.md).
//!
//! Stack locals of the original are named by their offset from ESP after the four register pushes (`E+0x40`, …)
//! where a value lives in a slot that more than one block reads.

use sa_core::abi::HookReg;
use sa_core::field::v3_ptr as vp;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use crate::bike_control::mat;
use sa_math::fpu::{fbits as c, st0 as q, fcom_ne, fcom_parity, ge_or_nan, gt, le_or_nan, lt};
use sa_math::x87::X87;

use crate::bindings::get_car_gun_left_right;
use crate::mvpd_v4::ci_asin as asin;
use sa_world::physical::shared::{cv, rv};
use sa_world::world::mvpe_ent1::ci_pow as pow;
use crate::common::{f32_of, ftol32, g, rd, wr, x, xi};
use sa_core::bindings::{cross_product, vector_dot_product, vector_magnitude, vector_normalise};
use sa_pop::bindings::{get_car_gun_up_down, pad_get_look_left, pad_get_look_right};
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::{
    entity_get_col_model, get_pad_from_player, max_f, min_f, multiply3x3, multiply3x3_vm, pad_get_accelerate,
    pad_get_brake, pad_get_steering_left_right, pad_get_steering_up_down, physical_apply_move_force_5429f0,
    physical_apply_turn_force, physical_get_speed, vec_mul_assign, vector_add, vector_add_assign, vector_magnitude_sqr,
    vector_mul_scalar, vector_scale, vector_sub,
};

orig! {
    /// `CVehicle::HeightAboveCeiling(float height, eFlightModel)` (0x6D2600; ST0)
    fn height_above_ceiling(v: GPtr<u8>, h: f32, model: u32) -> X87 = reg("args = ecx stack stack; pop") 0x6D2600 => crate::vehicle2::height_above_ceiling;
}

/// The "take the pad's value" marker of the control arguments (0x871F8C, -9999.99f).
const MARKER: f32 = f32::from_bits(0xC61C_3FF6);
/// `CTimer::ms_fTimeStep`.
const TIME_STEP: usize = 0xB7_CB5C;

fn ts() -> f32 {
    g::<f32>(TIME_STEP).read()
}
/// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0).
fn mv(v: GPtr<u8>, f: [f32; 3]) {
    physical_apply_move_force_5429f0(v, cv(f));
}
/// `CPhysical::ApplyTurnForce(CVector force, CVector point)` (0x542A50).
fn turn(v: GPtr<u8>, f: [f32; 3], p: [f32; 3]) {
    physical_apply_turn_force(v, cv(f), cv(p));
}
/// `operator*(float, const CVector&)` (0x40FE90) into a temporary.
fn scale(k: f32, v: GPtr<u8>) -> [f32; 3] {
    let mut o = [0f32; 3];
    let r = vector_scale(vp(&mut o), k, v);
    rv(r, 0)
}
/// `operator*(const CVector&, float)` (0x40FEC0) of a local.
fn mul_l(mut a: [f32; 3], s: f32) -> [f32; 3] {
    let mut o = [0f32; 3];
    let r = vector_mul_scalar(vp(&mut o), vp(&mut a), s);
    rv(r, 0)
}
/// `CVector::operator*=(float)` (0x40FEF0) of a local.
fn mul_assign(mut a: [f32; 3], s: f32) -> [f32; 3] {
    vec_mul_assign(vp(&mut a), s);
    a
}
/// `operator+(const CVector&, const CVector&)` (0x40FE30) of two locals.
fn add_l(mut a: [f32; 3], mut b: [f32; 3]) -> [f32; 3] {
    let mut o = [0f32; 3];
    let r = vector_add(vp(&mut o), vp(&mut a), vp(&mut b));
    rv(r, 0)
}
/// `CVector::Magnitude()` (0x4082C0) of a local.
fn mag(a: &mut [f32; 3]) -> X87 {
    X87::from_f64_exact(vector_magnitude(GPtr::from_raw(a.as_mut_ptr()).cast()))
}
/// `CVector::SquaredMagnitude()` (0x406DA0) of a local.
fn sq_mag(a: &mut [f32; 3]) -> X87 {
    vector_magnitude_sqr(vp(a))
}
/// `CPhysical::GetSpeed(CVector point)` (0x542CE0) into a temporary.
fn speed_at(v: GPtr<u8>, p: [f32; 3]) -> [f32; 3] {
    let mut o = [0f32; 3];
    let r = physical_get_speed(v, GPtr::from_raw(o.as_mut_ptr()).cast(), cv(p));
    rv(r.cast(), 0)
}
/// `_CIacos` (0x822380).
fn acos(v: X87) -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_unary(0x82_2380, v.to_f64()))
}
/// `GetPosition().z`: the matrix's when the entity has one, else the placement's.
fn pos_z(v: GPtr<u8>) -> f32 {
    let m = mat(v);
    if m.is_null() { rd::<f32>(v, 0xC) } else { rd::<f32>(m, 0x38) }
}
/// `((a.z * b.z) + (a.y * b.y)) + (a.x * b.x)` with `b` in memory, as the inlined dot products load it.
fn dot_zyx(a: [f32; 3], b: GPtr<u8>, o: isize) -> X87 {
    x(a[2]).fmul_m(rd::<f32>(b, o + 8)).fadd(x(a[1]).fmul_m(rd::<f32>(b, o + 4))).fadd(x(a[0]).fmul_m(rd::<f32>(b, o)))
}
/// A pad stick value (`movsx`) as `fild` loads it.
fn stick(v: i16) -> X87 {
    xi(i32::from(v))
}

/// The values the per-model blocks share: the driver's pad (`ebx` / `E+0x14`), the air-relative speed (`E+0x40`) and
/// the centre of mass in world orientation (`E+0x50`), and the four control arguments (their stack slots, which the
/// original rewrites).
struct Fc {
    this: GPtr<u8>,
    pad: GPtr<u8>,
    v: [f32; 3],
    com: [f32; 3],
    model: u32,
    skid: f32,
    ud: f32,
    lr: f32,
    accel: f32,
}

/// `CVehicle::FlyingControl(eFlightModel model, float skid, float upDown, float leftRight, float accel)` (0x6D85F0).
/// Without flying handling (+0x388) or with a time step `<= 0` nothing. The pad is the player driver's (status 0). The
/// air-relative speed is the move speed minus the wind (`CWeather::WindDir` 0xC813E0 times the handling's wind
/// multiplier +0x38; no wind for mission vehicles, +0x4A4 == 2); the centre of mass +0xA4 is turned into world
/// orientation. Then the model's block and the resistance tail; models above 8 run only the tail.
#[hook(addr = 0x6D85F0, conv = thiscall, name = "CVehicle::FlyingControl", this = "CVehicle", expect = "90 E9 F2 B6 D2 FF 53 55 56",
       unverified = "frontier2-b14", route = "L2", hazards = "x87-transcendental",
       note = "entry `nop; jmp 0x403CE8` (the frame's sub esp, 0x250 in a protection fragment); _CIpow / _CIasin / _CIacos through orig::x87_*; FPATAN / FCOS / FSIN / FSQRT via X87; ftol of the stall time")]
pub fn vehicle_flying_control(this: GPtr<u8>, model: u32, skid: f32, ud: f32, lr: f32, accel: f32) {
    let fh = rd::<GPtr<u8>>(this, 0x388);
    if fh.is_null() || !fcom_parity(x(ts()), x(0.0), 0x41) {
        return;
    }
    let mut pad = GPtr::<u8>::NULL;
    if rd::<u8>(this, 0x36) & 0xF8 == 0 {
        let driver = rd::<GPtr<u8>>(this, 0x460);
        if !driver.is_null() && ped_is_player(driver) != 0 {
            pad = get_pad_from_player(rd::<GPtr<u8>>(this, 0x460));
        }
    }
    let (mut wy, mut wz) = (0f32, 0f32);
    let wx = if rd::<u8>(this, 0x4A4) == 2 {
        x(0.0)
    } else {
        let m = x(rd::<f32>(fh, 0x38));
        let a = f32_of(x(g::<f32>(0xC8_13E0).read()).fmul(m));
        let b = f32_of(x(g::<f32>(0xC8_13E4).read()).fmul(m));
        let cz = f32_of(m.fmul_m(g::<f32>(0xC8_13E8).read()));
        wy = f32_of(x(b).fchs());
        wz = f32_of(x(cz).fchs());
        x(a).fchs()
    };
    let v = [
        f32_of(wx.fadd_m(rd::<f32>(this, 0x44))),
        f32_of(x(wy).fadd_m(rd::<f32>(this, 0x48))),
        f32_of(x(wz).fadd_m(rd::<f32>(this, 0x4C))),
    ];
    let mut com = [0f32; 3];
    let r = multiply3x3(vp(&mut com), mat(this), this.byte_add(0xA4));
    let com = rv(r, 0);
    let mut fc = Fc { this, pad, v, com, model, skid, ud, lr, accel };
    match model {
        0 => dodo(&mut fc),
        1 | 3 | 4 | 5 => plane(&mut fc),
        2 | 6 | 8 => heli(&mut fc),
        _ => {}
    }
    resistance(&fc);
}

/// Model 0 (0x6D8715, the clipped-wing Dodo): yaw from the steer angle +0x494 and the side slip, a side force, pitch
/// from the pad and the vertical slip, and lift `(upFraction * 3.5 + 0.5) * mass * timeStep * scale * 0.05`, held to
/// `mass * timeStep * 0.0072` above the ceiling (HeightAboveCeiling with model 0) for a player / status 8 vehicle;
/// `turnSpeed.y *= 0.9f ^ timeStep`.
fn dodo(fc: &mut Fc) {
    let this = fc.this;
    let m = mat(this);
    let side = f32_of(dot_zyx(fc.v, m, 0).fmul_m(c(0xBF80_0000)));
    let side_frac = f32_of(mag(&mut fc.v).fdivr_m(side));
    let f = rv(m, 0x10);
    let fws = f32_of(x(f[1]).fmul_m(fc.v[1]).fadd(x(f[0]).fmul_m(fc.v[0])).fadd(x(f[2]).fmul_m(fc.v[2])));
    let scale_v = f32_of(sq_mag(&mut fc.v).fmul_m(fws).fmul_m(fws));
    let mass = rd::<f32>(this, 0x8C);
    let turn_mass = rd::<f32>(this, 0x90);
    let yaw = x(rd::<f32>(this, 0x494))
        .fmul(q(0.001))
        .fadd(x(side_frac).fmul(q(0.003)))
        .fmul_m(ts())
        .fmul_m(scale_v)
        .fmul_m(turn_mass);
    let r1 = mul_assign(rv(m, 0), f32_of(yaw));
    let fwd4 = [f32_of(x(f[0]).fmul_m(-4.0)), f32_of(x(f[1]).fmul_m(-4.0)), f32_of(x(f[2]).fmul_m(-4.0))];
    turn(this, r1, fwd4);
    let k = x(ts()).fmul_m(mass).fmul_m(scale_v).fmul_m(side_frac).fmul_m(c(0x3E4C_CCCD));
    let m = mat(this);
    let r2 = mul_assign(rv(m, 0), f32_of(k));
    mv(this, r2);
    let up2 = scale(2.0, mat(this).byte_add(0x20));
    turn(this, r2, up2);
    let m = mat(this);
    let up_speed = f32_of(dot_zyx(fc.v, m, 0x20).fmul_m(c(0xBF80_0000)));
    let up_frac = f32_of(mag(&mut fc.v).fdivr_m(up_speed));
    let steer = if fc.pad.is_null() { x(0.0) } else { stick(pad_get_steering_up_down(fc.pad)).fmul_m(c(0xBC00_0000)) };
    let m = mat(this);
    let pitch = steer
        .fmul_m(c(0x3A83_126F))
        .fadd(x(up_frac).fmul_m(c(0x3B03_126F)))
        .fmul_m(ts())
        .fmul_m(scale_v)
        .fmul_m(turn_mass);
    let u1 = mul_assign(rv(m, 0x20), f32_of(pitch));
    turn(this, u1, fwd4);
    let mut lift = f32_of(
        x(up_frac).fmul(q(3.5)).fadd(q(0.5)).fmul_m(mass).fmul_m(ts()).fmul_m(scale_v).fmul(q(0.05)),
    );
    let status = rd::<u8>(this, 0x36) >> 3;
    if (status == 0 || status == 8)
        && lt(x(ts()).fmul_m(mass).fmul_m(c(0x3C03_126F)), x(lift))
        && gt(height_above_ceiling(this, pos_z(this), 0), x(0.0))
    {
        lift = f32_of(x(mass).fmul_m(ts()).fmul(q(f64::from_bits(0x3F7D_7DBF_6000_0000))));
    }
    let m = mat(this);
    let lu = scale(lift, m.byte_add(0x20));
    let up2 = scale(2.0, m.byte_add(0x20));
    let p = add_l(up2, fc.com);
    mv(this, lu);
    turn(this, lu, p);
    let p = pow(q(f64::from_bits(0x3FEC_CCCC_C000_0000)), ts());
    wr::<f32>(this, 0x54, f32_of(p.fmul_m(rd::<f32>(this, 0x54))));
}

/// `a + b` componentwise, each `fld a; fadd b; fstp` (the inlined point sums).
fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [f32_of(x(a[0]).fadd_m(b[0])), f32_of(x(a[1]).fadd_m(b[1])), f32_of(x(a[2]).fadd_m(b[2]))]
}

/// Models 1 / 3 / 4 / 5 (0x6D8B1D: RC plane, plane, low-power plane, glider). The marker arguments take the pad's
/// stick (the car-gun up / down overriding the pitch beyond 1) and accelerate - brake; the stick vector is stretched
/// to the square by `1 / cos` of its angle off the nearest axis; then thrust (on the ground or on water, braking only;
/// in the air reduced by the handling's fall-off; kept in the accel argument's slot), the Hydra's nozzle direction (no
/// automobile check in the bytes), side slip, yaw, roll, the roll stabiliser toward the horizon, pitch, the stall
/// counter (planes above 20 degrees) and lift (HeightAboveCeiling evaluated up to three times).
fn plane(fc: &mut Fc) {
    let this = fc.this;
    let pad = fc.pad;
    let one128 = c(0x3C00_0000);
    if !fcom_ne(x(fc.skid), x(MARKER)) {
        fc.skid = 0.0;
        if !pad.is_null() {
            fc.skid = f32_of(stick(pad_get_steering_left_right(pad)).fmul_m(one128));
        }
    }
    if !fcom_ne(x(fc.ud), x(MARKER)) {
        fc.ud = 0.0;
        if !pad.is_null() {
            fc.ud = f32_of(stick(pad_get_steering_up_down(pad)).fmul_m(one128));
            if gt(stick(get_car_gun_up_down(pad)).fabs(), x(1.0)) {
                let g2 = i32::from(get_car_gun_up_down(pad)).wrapping_neg();
                fc.ud = f32_of(xi(g2).fmul_m(one128));
            }
        }
    }
    let a = x(f32_of(x(fc.ud).fpatan(x(fc.skid))));
    let (pi4, pi2, pi, pi34) = (c(0x3F49_0FDB), c(0x3FC9_0FDB), c(0x4049_0FDB), c(0x4016_CBE4));
    let (npi4, npi34) = (c(0xBF49_0FDB), c(0xC016_CBE4));
    let arg = if gt(a, x(npi4)) && !fcom_parity(a, x(pi4), 0x41) {
        Some(a)
    } else if gt(a, x(pi4)) && !fcom_parity(a, x(pi34), 0x41) {
        Some(a.fsub_m(pi2))
    } else if gt(a, x(pi34)) {
        Some(a.fsub_m(pi))
    } else if !fcom_parity(a, x(npi34), 0x41) {
        Some(a.fadd_m(pi))
    } else if !fcom_parity(a, x(npi4), 0x05) {
        Some(a.fadd_m(pi2))
    } else {
        None
    };
    let mult = match arg {
        Some(t) => t.fcos().fdivr_m(1.0),
        None => x(1.0),
    };
    fc.skid = f32_of(x(fc.skid).fmul(mult));
    fc.ud = f32_of(mult.fmul_m(fc.ud).fchs());
    let f = rv(mat(this), 0x10);
    let bb_min_y = x(rd::<f32>(entity_get_col_model(this), 4));
    let tail = [f32_of(x(f[0]).fmul(bb_min_y)), f32_of(x(f[1]).fmul(bb_min_y)), f32_of(x(f[2]).fmul(bb_min_y))];
    let d = x(f[1]).fmul_m(fc.v[1]).fadd(x(f[0]).fmul_m(fc.v[0])).fadd(x(f[2]).fmul_m(fc.v[2]));
    let fws = f32_of(d);
    let fws2 = f32_of(d.fmul_m(fws));
    let mass = rd::<f32>(this, 0x8C);
    let turn_mass = rd::<f32>(this, 0x90);
    if fc.model == 1 {
        let a = mul_l([0.0, 0.0, c(0x3B83_126F)], mass);
        mv(this, mul_l(a, ts()));
    }
    if !fcom_ne(x(fc.accel), x(MARKER)) {
        fc.accel = 0.0;
        if !pad.is_null() {
            let acc = i32::from(pad_get_accelerate(pad) as u16 as i16);
            let brk = i32::from(pad_get_brake(pad) as u16 as i16);
            fc.accel = f32_of(xi(acc.wrapping_sub(brk)).fmul_m(c(0x3B80_8081)));
        }
    }
    let fh = rd::<GPtr<u8>>(this, 0x388);
    let id = rd::<u16>(this, 0x22);
    // `true`: the ground / water block at 0x6D8E6F; `false`: the air block at 0x6D8EFE
    let ground = rd::<u32>(this, 0x590) == 0
        && (id == 0x21B
            || [0x7D4, 0x7D8, 0x7DC, 0x7E0].iter().any(|&o| rd::<u32>(this, o) != 0x3F80_0000)
            || (id == 0x1CC && rd::<u32>(this, 0x40) & 0x100 != 0 && le_or_nan(x(fws), x(c(0x3E4C_CCCD)))))
        && le_or_nan(x(fc.accel), x(0.0));
    if ground {
        let braking = lt(x(fc.accel), x(0.0)) && {
            let mut fw = rv(mat(this), 0x10);
            lt(vector_dot_product(vp(&mut fw), this.byte_add(0x44)), x(c(0x3CA3_D70A)))
        };
        if braking {
            let t = x(fws).fmul_m(c(0x40F8_51EC)).fsubr_m(fc.accel);
            let t = if ge_or_nan(x(0.0), t) { t } else { x(0.0) };
            fc.accel = f32_of(t.fmul_m(rd::<f32>(fh, 4)));
        } else {
            fc.accel = 0.0;
        }
    } else {
        let t = if id == 0x21B {
            x(fc.accel).fmul_m(rd::<f32>(fh, 4))
        } else {
            x(fc.accel).fadd_m(1.0).fmul_m(rd::<f32>(fh, 4)).fmul_m(0.5)
        };
        fc.accel = f32_of(t);
        let fo = rd::<f32>(fh, 8);
        if gt(x(fws), x(0.0)) && lt(x(fo), x(1.0)) {
            let s = if x(fo).fcom(x(0.0)).c0() {
                let d = x(fws).fadd_m(fo);
                d.fmul(d).fmul_m(3.0)
            } else {
                let d = x(fws).fsub_m(fo);
                d.fmul(d).fmul_m(c(0x3F26_6666))
            };
            let s = s.fsubr_m(1.0);
            let s = if le_or_nan(x(0.0), s) { s } else { x(0.0) };
            fc.accel = f32_of(s.fmul_m(fc.accel));
        }
    }
    if fc.model == 4 {
        fc.accel = f32_of(x(fc.accel).fmul_m(c(0x3E99_999A)));
    } else if fc.model == 5 {
        fc.accel = f32_of(x(fc.accel).fmul_m(c(0x3DCC_CCCD)));
    }
    let thrust = fc.accel;
    let m = mat(this);
    let mut td = rv(m, 0x10);
    if id == 0x208 {
        let limit = i32::from(g::<i16>(0x8D_33C4).read());
        let ang = xi(i32::from(rd::<u16>(this, 0x86C))).fmul_m(pi2).fdiv(xi(limit));
        let cs = f32_of(ang.fcos());
        let sn = f32_of(ang.fsin());
        let su = scale(sn, m.byte_add(0x20));
        let cf = scale(cs, m.byte_add(0x10));
        td = add_l(cf, su);
    }
    // the thrust force with the operand order and roundings of 0x6D9084
    let k = c(0x3C03_126F);
    let a0 = x(td[0]).fmul_m(k);
    let b0 = x(td[1]).fmul_m(k);
    let c0 = f32_of(x(td[2]).fmul_m(k));
    let ax = f32_of(a0.fmul_m(thrust));
    let bx = b0.fmul_m(thrust);
    let cx = x(c0).fmul_m(thrust);
    let axm = f32_of(x(ax).fmul(x(mass)));
    let bxm = f32_of(bx.fmul(x(mass)));
    let cxm = cx.fmul(x(mass));
    mv(this, [f32_of(x(axm).fmul_m(ts())), f32_of(x(bxm).fmul_m(ts())), f32_of(cxm.fmul_m(ts()))]);
    // side slip (0x6D9129): right * (side * (|side| * sideSlip)) * mass * timeStep
    let m = mat(this);
    let side = dot_zyx(fc.v, m, 0).fmul_m(c(0xBF80_0000));
    let r = rv(m, 0);
    let k = side.fmul(side.fabs().fmul_m(rd::<f32>(fh, 0x14)));
    let rxm = x(r[0]).fmul(k).fmul_m(mass);
    let ry = f32_of(x(r[1]).fmul(k));
    let rz = f32_of(x(r[2]).fmul(k));
    let rym = x(ry).fmul_m(mass);
    let rzm = f32_of(x(rz).fmul_m(mass));
    mv(this, [f32_of(rxm.fmul_m(ts())), f32_of(rym.fmul_m(ts())), f32_of(x(rzm).fmul_m(ts()))]);
    // yaw (0x6D91EF)
    let m = mat(this);
    let o = speed_at(this, tail);
    let tside = dot_zyx(o, m, 0).fmul_m(c(0xBF80_0000));
    let ys = rd::<f32>(fh, 0x10);
    let (adj, yaw_acc) = if rd::<u16>(this, 0x22) == 0x21B {
        let hi = if le_or_nan(x(fws), x(thrust)) { x(thrust) } else { x(fws) };
        let adj = if fcom_parity(x(fws), x(0.0), 0x41) {
            hi
        } else if ge_or_nan(x(fws), x(thrust)) {
            x(thrust)
        } else {
            x(fws)
        };
        (adj, tside.fabs().fmul_m(ys).fmul(tside))
    } else {
        (x(fws), tside.fabs().fmul_m(ys).fmul(tside))
    };
    let yaw = yaw_acc.fadd(adj.fmul_m(rd::<f32>(fh, 0xC)).fmul_m(fc.skid)).fmul_m(ts()).fmul_m(turn_mass);
    let rr = mul_assign(rv(m, 0), f32_of(yaw));
    turn(this, rr, add3(fc.com, tail));
    // roll (0x6D934D)
    let m = mat(this);
    let st = x(ts()).fmul_m(rd::<f32>(fh, 0x18)).fmul_m(fws);
    let st = if fcom_ne(x(fc.lr), x(MARKER)) { st.fmul_m(fc.lr) } else { st.fmul_m(fc.skid) };
    let rr = mul_assign(rv(m, 0), f32_of(st.fmul_m(turn_mass)));
    turn(this, rr, add3(fc.com, rv(m, 0x20)));
    // roll stabiliser (0x6D9417)
    let m = mat(this);
    let mut fw = rv(m, 0x10);
    let mut zup = [0.0, 0.0, 1.0f32];
    let mut out = [0f32; 3];
    let h = cross_product(vp(&mut out), vp(&mut fw), vp(&mut zup));
    let mut h = rv(h, 0);
    let m = mat(this);
    if !gt(x(rd::<f32>(m, 0x28)), x(0.0)) {
        h = h.map(|e| f32_of(x(e).fmul_m(c(0xBF80_0000))));
    }
    let side_sign = if le_or_nan(x(rd::<f32>(m, 8)), x(0.0)) { x(1.0) } else { x(c(0xBF80_0000)) };
    let fz = x(rd::<f32>(m, 0x18)).fabs().fsubr_m(1.0);
    let dot = dot_zyx(h, m, 0);
    let t = fz.fmul(dot.fsubr_m(1.0).fmul(side_sign));
    let p = add3(fc.com, rv(m, 0x20));
    let k = t.fmul_m(rd::<f32>(fh, 0x1C)).fmul_m(ts()).fmul_m(turn_mass).fmul_m(0.5);
    let f = scale(f32_of(k), m);
    turn(this, f, p);
    // pitch (0x6D9572)
    let m = mat(this);
    let u = rv(m, 0x20);
    let o = speed_at(this, tail);
    let tup = dot_zyx(o, m, 0x20).fmul_m(c(0xBF80_0000));
    let k = tup
        .fmul(tup.fabs().fmul_m(rd::<f32>(fh, 0x24)))
        .fadd(x(fws).fmul_m(rd::<f32>(fh, 0x20)).fmul_m(fc.ud))
        .fmul_m(ts())
        .fmul_m(turn_mass);
    let fu = [f32_of(x(u[0]).fmul(k)), f32_of(x(u[1]).fmul(k)), f32_of(x(u[2]).fmul(k))];
    turn(this, fu, add3(fc.com, tail));
    // lift (0x6D966F)
    let m = mat(this);
    let tz = f32_of(dot_zyx(fc.v, m, 0x20));
    let mg = f32_of(mag(&mut fc.v));
    let r = max_f(c(0x3C23_D70A), mg).fdivr_m(tz);
    let r = min_f(0x3F80_0000, f32_of(r).to_bits());
    let r = max_f(c(0xBF80_0000), f32_of(r));
    let attack = asin(r).fmul_m(c(0xBF80_0000));
    let kind = rd::<u32>(this, 0x594);
    if kind == 4 && gt(attack, x(c(0x3EB2_B8C3))) {
        let add = ftol32(x(ts()).fmul_m(c(0x3CA3_D70A)).fmul_m(1000.0));
        wr::<u32>(this, 0x9A0, rd::<u32>(this, 0x9A0).wrapping_add(add as u32));
    }
    let mut fl = x(rd::<f32>(fh, 0x28));
    if fc.model == 1 {
        let lim = f32_of(x(g::<f32>(0x8D_3670).read()).fmul_m(c(0x3C03_126F)));
        if gt(x(fws2).fmul(fl), x(lim)) {
            fl = x(lim).fdiv_m(fws2);
        }
    } else if kind == 4 && lt(x(rd::<f32>(this, 0x9CC)), x(1.0)) {
        fl = fl.fmul_m(rd::<f32>(fh, 0x34));
    }
    let lift = attack.fmul_m(rd::<f32>(fh, 0x2C)).fadd(fl).fmul_m(mass).fmul_m(ts()).fmul_m(fws2);
    let mut lift_f = f32_of(lift);
    let inst = x(ts()).fmul_m(mass).fmul_m(c(0x3C03_126F));
    if gt(lift, inst) && gt(height_above_ceiling(this, pos_z(this), fc.model), x(0.0)) {
        let h = height_above_ceiling(this, pos_z(this), fc.model).fmul_m(c(0x3CA3_D70A)).fsubr_m(1.0);
        let k = if ge_or_nan(h, x(0.0)) {
            height_above_ceiling(this, pos_z(this), fc.model).fmul_m(c(0x3CA3_D70A)).fsubr_m(1.0)
        } else {
            x(0.0)
        };
        lift_f = f32_of(k.fmul_m(mass).fmul_m(ts()).fmul_m(c(0x3C03_126F)));
    }
    mv(this, scale(lift_f, mat(this).byte_add(0x20)));
}

/// The Hydra's thrust direction (0x6D8FFF / 0x6D9C10): the nozzle angle +0x86C over its limit 0x8D33C4 times pi / 2;
/// `cos * forward + sin * up` through the original's vector operators.
fn hydra_nozzle_dir(this: GPtr<u8>, m: GPtr<u8>) -> [f32; 3] {
    let limit = i32::from(g::<i16>(0x8D_33C4).read());
    let ang = xi(i32::from(rd::<u16>(this, 0x86C))).fmul_m(c(0x3FC9_0FDB)).fdiv(xi(limit));
    let cs = f32_of(ang.fcos());
    let sn = f32_of(ang.fsin());
    let su = scale(sn, m.byte_add(0x20));
    let cf = scale(cs, m.byte_add(0x10));
    add_l(cf, su)
}

/// `(a * p) * m * timeStep`, `(b * p) * m * timeStep`, `(c * p) * m * timeStep` with the stores of the original's
/// interleaved x87 code (0x6D9084, 0x6DA4DA, 0x6DA84B): x rounds after `* p` and after `* m`, y after `* m`, z before
/// `* p`; each component rounds after `* timeStep`.
fn chain3(a: X87, b: X87, cz: X87, p: f32, m: f32) -> [f32; 3] {
    let ap = f32_of(a.fmul_m(p));
    let bp = b.fmul_m(p);
    let cp = x(f32_of(cz)).fmul_m(p);
    let apm = f32_of(x(ap).fmul(x(m)));
    let bpm = f32_of(bp.fmul(x(m)));
    let cpm = cp.fmul(x(m));
    [f32_of(x(apm).fmul_m(ts())), f32_of(x(bpm).fmul_m(ts())), f32_of(cpm.fmul_m(ts()))]
}

/// `right * k * mass * timeStep` with the stores of 0x6D9129 / 0x6DA6CC: x unrounded until the end, y rounded after
/// `* k`, z after `* k` and `* mass`.
fn side_force(r: [f32; 3], k: X87, mass: f32) -> [f32; 3] {
    let rxm = x(r[0]).fmul(k).fmul_m(mass);
    let ry = f32_of(x(r[1]).fmul(k));
    let rz = f32_of(x(r[2]).fmul(k));
    let rym = x(ry).fmul_m(mass);
    let rzm = f32_of(x(rz).fmul_m(mass));
    [f32_of(rxm.fmul_m(ts())), f32_of(rym.fmul_m(ts())), f32_of(x(rzm).fmul_m(ts()))]
}

/// The level-seeking tilt of 0x6D9F5A / 0x6DA06D: `dot(w, axis)` clamped to `[-formLift, formLift]` with the
/// original's compare chain (the dot product recomputed for every use).
fn clamp_tilt(w: [f32; 3], m: GPtr<u8>, o: isize, fl: f32) -> f32 {
    let nfl = x(fl).fchs();
    let dot = || dot_zyx(w, m, o);
    let xv = if le_or_nan(nfl, dot()) { dot() } else { nfl };
    if !le_or_nan(xv, x(fl)) {
        fl
    } else if gt(nfl, dot()) {
        f32_of(nfl)
    } else {
        f32_of(dot())
    }
}

/// Models 2 / 6 / 8 (0x6D986E: RC heli, heli, autogyro). The move speed is damped by `moveRes ^ timeStep`. The
/// autogyro (8): a pusher thrust along forward without the 0.008 scale, and the rotor thrust along `up - tilt *
/// forward` normalised, the automobile's rotor speed +0x84C spun by the air through the rotor and clamped to
/// [0.08, 0.4]. The others: thrust along up (with the minimum-tilt flag +0x42B bit 4 the up vector's angles are
/// quadrupled; the Hydra's nozzle otherwise), a hover force `(0.5 - moveSpeed.z)` up, the rotor thrust reduced by the
/// speed along it and above the ceiling. Then the level-seeking roll / pitch (toward the wind-tilted up; upside down:
/// a fixed recovery), the pad (the heli-controls cheat 0xC1C970 swaps the axes), pitch / roll / side slip / yaw.
fn heli(fc: &mut Fc) {
    let this = fc.this;
    let pad = fc.pad;
    let fh = rd::<GPtr<u8>>(this, 0x388);
    let pw = pow(x(rd::<f32>(fh, 0x3C)), ts());
    for o in [0x44, 0x48, 0x4C] {
        wr::<f32>(this, o, f32_of(pw.fmul_m(rd::<f32>(this, o))));
    }
    let mass = rd::<f32>(this, 0x8C);
    let one128 = c(0x3C00_0000);
    let one255 = c(0x3B80_8081);
    let force = if fc.model == 8 {
        let m = mat(this);
        let fws = f32_of(dot_zyx(fc.v, m, 0x10));
        let a = if fcom_ne(x(fc.accel), x(MARKER)) {
            x(fc.accel)
        } else if pad.is_null() {
            x(0.0)
        } else {
            let acc = i32::from(pad_get_accelerate(pad) as u16 as i16);
            let brk = i32::from(pad_get_brake(pad) as u16 as i16);
            xi(acc.wrapping_sub(brk)).fmul_m(one255)
        };
        let fh = rd::<GPtr<u8>>(this, 0x388);
        let p = if gt(x(fws), x(0.0)) || gt(a, x(0.0)) {
            f32_of(a.fsub(x(fws).fmul_m(rd::<f32>(fh, 8))).fmul_m(rd::<f32>(fh, 4)))
        } else {
            let t = a.fsub(x(fws).fmul_m(rd::<f32>(fh, 8)).fmul_m(8.0));
            let t = if ge_or_nan(x(0.0), t) { t } else { x(0.0) };
            f32_of(t.fmul_m(rd::<f32>(fh, 4)))
        };
        let push = mul_l(mul_l(mul_l(rv(mat(this), 0x10), p), mass), ts());
        mv(this, push);
        let m = mat(this);
        let tf = scale(g::<f32>(0x8D_34E8).read(), m.byte_add(0x10));
        let mut d = [0f32; 3];
        let mut tf = tf;
        let mut up = rv(m, 0x20);
        let r = vector_sub(vp(&mut d), vp(&mut up), vp(&mut tf));
        let mut d = rv(r, 0);
        vector_normalise(GPtr::from_raw(d.as_mut_ptr()).cast());
        let mut rp = f32_of(x(d[0]).fmul_m(fc.v[0]).fadd(x(d[2]).fmul_m(fc.v[2])).fadd(x(d[1]).fmul_m(fc.v[1])));
        let mrs = if rd::<u32>(this, 0x590) != 0 {
            x(c(0x3E61_47AE))
        } else {
            let nl = x(g::<f32>(0x8D_34D8).read()).fchs();
            let xv = if le_or_nan(nl, x(rp)) { x(rp) } else { nl };
            if !ge_or_nan(x(0.0), xv) {
                rp = 0.0;
            } else if !le_or_nan(nl, x(rp)) {
                rp = f32_of(nl);
            }
            let s = f32_of(x(rp).fmul_m(ts()).fmul_m(g::<f32>(0x8D_34D4).read()).fsubr_m(rd::<f32>(this, 0x84C)));
            wr::<f32>(this, 0x84C, s);
            let v2 = pow(x(g::<f32>(0x8D_34DC).read()), ts()).fmul_m(s);
            wr::<f32>(this, 0x84C, f32_of(v2));
            if !le_or_nan(v2, x(c(0x3ECC_CCCD))) {
                wr::<u32>(this, 0x84C, 0x3ECC_CCCD);
            } else if !ge_or_nan(v2, x(c(0x3DA3_D70A))) {
                wr::<u32>(this, 0x84C, 0x3DA3_D70A);
            }
            x(rd::<f32>(this, 0x84C))
        };
        let k8 = c(0x3C03_126F);
        let d8 = [f32_of(x(d[0]).fmul_m(k8)), f32_of(x(d[1]).fmul_m(k8)), f32_of(x(d[2]).fmul_m(k8))];
        let s = f32_of(mrs.fmul_m(g::<f32>(0x8D_34E0).read()).fsub(x(rp).fmul_m(g::<f32>(0x8D_34E4).read())));
        mul_l(mul_l(mul_l(d8, s), mass), ts())
    } else {
        let m = mat(this);
        let mut td = rv(m, 0x20);
        if rd::<u8>(this, 0x42B) & 0x10 != 0 {
            td[0] = f32_of(asin(x(td[0])).fmul_m(4.0).fsin());
            td[1] = f32_of(asin(x(td[1])).fmul_m(4.0).fsin());
            td[2] = f32_of(acos(x(td[2])).fmul_m(4.0).fcos());
        } else if rd::<u16>(this, 0x22) == 0x208 {
            td = hydra_nozzle_dir(this, m);
        }
        let mut tas = f32_of(x(fc.v[2]).fmul_m(td[2]).fadd(x(fc.v[1]).fmul_m(td[1])).fadd(x(fc.v[0]).fmul_m(td[0])));
        if gt(x(tas), x(0.0)) {
            tas = f32_of(x(tas).fadd(x(tas)));
        }
        if !fcom_ne(x(fc.accel), x(MARKER)) {
            fc.accel = 0.0;
            if !pad.is_null() {
                let acc = i32::from(pad_get_accelerate(pad) as u16 as i16);
                let brk = i32::from(pad_get_brake(pad) as u16 as i16);
                fc.accel = f32_of(xi(acc.wrapping_sub(brk)).fmul_m(one255));
                if gt(stick(get_car_gun_up_down(pad)).fabs(), x(1.0)) {
                    fc.accel = f32_of(stick(get_car_gun_up_down(pad)).fmul_m(one128));
                }
            }
        }
        let fh = rd::<GPtr<u8>>(this, 0x388);
        let rta = f32_of(x(fc.accel).fmul_m(rd::<f32>(fh, 4)).fadd_m(c(0x3EE6_6666)));
        let s = f32_of(x(0.5).fsub_m(rd::<f32>(this, 0x4C)));
        let hover = mul_l(mul_l(mul_l(mul_l([0.0, 0.0, 1.0], s), c(0x3C03_126F)), mass), ts());
        mv(this, hover);
        let fh = rd::<GPtr<u8>>(this, 0x388);
        let mut rta = f32_of(x(tas).fmul_m(rd::<f32>(fh, 8)).fsubr_m(rta));
        if gt(height_above_ceiling(this, pos_z(this), fc.model), x(0.0)) {
            rta = f32_of(height_above_ceiling(this, pos_z(this), fc.model).fadd_m(10.0).fdivr_m(rta).fmul_m(10.0));
        }
        let k8 = c(0x3C03_126F);
        let d8 = [f32_of(x(td[0]).fmul_m(k8)), f32_of(x(td[1]).fmul_m(k8)), f32_of(x(td[2]).fmul_m(k8))];
        mul_l(mul_l(mul_l(d8, rta), mass), ts())
    };
    mv(this, force);
    let tm = rd::<f32>(this, 0x90);
    // level-seeking roll and pitch (0x6D9EF9)
    let m = mat(this);
    let fh = rd::<GPtr<u8>>(this, 0x388);
    if gt(x(rd::<f32>(m, 0x28)), x(0.0)) {
        let mut w = [0.0, 0.0, 1.0f32];
        let mut ws = scale(rd::<f32>(fh, 0x38), GPtr::from_addr(0xC8_13E0));
        vector_add_assign(vp(&mut w), vp(&mut ws));
        vector_normalise(GPtr::from_raw(w.as_mut_ptr()).cast());
        let fh = rd::<GPtr<u8>>(this, 0x388);
        let m = mat(this);
        let tilt = clamp_tilt(w, m, 0, rd::<f32>(fh, 0x28));
        let p = add_l(rv(m, 0), fc.com);
        let k = f32_of(x(tilt).fchs().fmul_m(rd::<f32>(fh, 0x2C)).fmul_m(ts()).fmul_m(tm));
        turn(this, scale(k, m.byte_add(0x20)), p);
        let fh = rd::<GPtr<u8>>(this, 0x388);
        let m = mat(this);
        let tilt = clamp_tilt(w, m, 0x10, rd::<f32>(fh, 0x28));
        let p = add_l(rv(m, 0x10), fc.com);
        let k = f32_of(x(tilt).fchs().fmul_m(rd::<f32>(fh, 0x2C)).fmul_m(ts()).fmul_m(tm));
        turn(this, scale(k, m.byte_add(0x20)), p);
    } else {
        let fl = rd::<f32>(fh, 0x28);
        let tilt = if ge_or_nan(x(rd::<f32>(m, 8)), x(0.0)) { f32_of(x(fl).fchs()) } else { fl };
        let p = add_l(rv(m, 0), fc.com);
        let k = f32_of(x(tilt).fmul_m(rd::<f32>(fh, 0x2C)).fmul_m(ts()).fmul_m(tm));
        turn(this, scale(k, m.byte_add(0x20)), p);
        let m = mat(this);
        let fh = rd::<GPtr<u8>>(this, 0x388);
        let fl = rd::<f32>(fh, 0x28);
        let tilt = if ge_or_nan(x(rd::<f32>(m, 0x18)), x(0.0)) { f32_of(x(fl).fchs()) } else { fl };
        let p = add_l(rv(m, 0x10), fc.com);
        let k = f32_of(x(tilt).fmul_m(rd::<f32>(fh, 0x2C)).fmul_m(ts()).fmul_m(tm));
        turn(this, scale(k, m.byte_add(0x20)), p);
    }
    // the pad (0x6DA274)
    let cheat = g::<u8>(0xC1_C970).read() != 0;
    if !fcom_ne(x(fc.ud), x(MARKER)) {
        fc.ud = 0.0;
        if !pad.is_null() {
            fc.ud = f32_of(stick(pad_get_steering_up_down(pad)).fmul_m(one128));
        }
    }
    if !cheat {
        if !fcom_ne(x(fc.lr), x(MARKER)) {
            fc.lr = 0.0;
            if !pad.is_null() {
                let s = i32::from(pad_get_steering_left_right(pad)).wrapping_neg();
                fc.lr = f32_of(xi(s).fmul_m(one128));
            }
        }
        if !fcom_ne(x(fc.skid), x(MARKER)) {
            fc.skid = 0.0;
            if !pad.is_null() {
                fc.skid = f32_of(xi(i32::from(pad_get_look_right(pad))));
                if pad_get_look_left(pad) != 0 {
                    fc.skid = -1.0;
                }
                let gun = || stick(get_car_gun_left_right(pad) as u16 as i16);
                if gt(gun().fabs(), x(1.0)) {
                    fc.skid = f32_of(gun().fmul_m(one128));
                }
            }
        }
    } else {
        if !fcom_ne(x(fc.lr), x(MARKER)) {
            fc.lr = 0.0;
            if !pad.is_null() {
                fc.lr = f32_of(xi(i32::from(pad_get_look_left(pad))));
            }
        }
        if !fcom_ne(x(fc.skid), x(MARKER)) {
            fc.skid = 0.0;
            if !pad.is_null() {
                if pad_get_look_right(pad) != 0 {
                    fc.lr = -1.0;
                }
                fc.skid = f32_of(stick(pad_get_steering_left_right(pad)).fmul_m(one128));
            }
        }
    }
    if rd::<u8>(this, 0x42B) & 0x10 != 0 {
        let fh = rd::<GPtr<u8>>(this, 0x388);
        let (p, al) = (rd::<f32>(fh, 0x20), rd::<f32>(fh, 0x2C));
        let t = asin(x(p).fdiv_m(al)).fmul_m(0.25).fsin().fdiv_m(p).fmul_m(al);
        fc.ud = f32_of(x(fc.ud).fmul(t));
        fc.lr = f32_of(t.fmul_m(fc.lr));
    }
    // pitch and roll (0x6DA4AE)
    let m = mat(this);
    let fh = rd::<GPtr<u8>>(this, 0x388);
    let p = add3(fc.com, rv(m, 0x10));
    let u = |k: f32| (x(k).fmul_m(rd::<f32>(m, 0x20)), x(k).fmul_m(rd::<f32>(m, 0x24)), x(k).fmul_m(rd::<f32>(m, 0x28)));
    let (a, b, cz) = u(fc.ud);
    turn(this, chain3(a, b, cz, rd::<f32>(fh, 0x20), tm), p);
    let m = mat(this);
    let fh = rd::<GPtr<u8>>(this, 0x388);
    let p = add3(fc.com, rv(m, 0));
    let (a, b, cz) = (x(fc.lr).fmul_m(rd::<f32>(m, 0x20)), x(fc.lr).fmul_m(rd::<f32>(m, 0x24)), x(fc.lr).fmul_m(rd::<f32>(m, 0x28)));
    turn(this, chain3(a, b, cz, rd::<f32>(fh, 0x18), tm), p);
    // side slip and yaw (0x6DA679)
    let m = mat(this);
    let fh = rd::<GPtr<u8>>(this, 0x388);
    let r = rv(m, 0);
    let side = f32_of(dot_zyx(fc.v, m, 0).fmul_m(c(0xBF80_0000)));
    let abs = f32_of(x(side).fabs());
    let k = x(abs).fmul_m(rd::<f32>(fh, 0x14)).fmul_m(side);
    mv(this, side_force(r, k, mass));
    let m = mat(this);
    let fh = rd::<GPtr<u8>>(this, 0x388);
    let r = rv(m, 0);
    let k = x(abs)
        .fmul_m(rd::<f32>(fh, 0x10))
        .fmul_m(side)
        .fadd(x(fc.skid).fmul_m(rd::<f32>(fh, 0xC)))
        .fmul_m(ts())
        .fmul_m(tm);
    let f = [f32_of(x(r[0]).fmul(k)), f32_of(x(r[1]).fmul(k)), f32_of(x(r[2]).fmul(k))];
    let nfz = f32_of(x(rd::<f32>(m, 0x18)).fchs());
    let p = [
        f32_of(x(rd::<f32>(m, 0x10)).fchs().fadd_m(fc.com[0])),
        f32_of(x(rd::<f32>(m, 0x14)).fchs().fadd_m(fc.com[1])),
        f32_of(x(nfz).fadd_m(fc.com[2])),
    ];
    turn(this, f, p);
    let m = mat(this);
    let fh = rd::<GPtr<u8>>(this, 0x388);
    let p = add3(fc.com, rv(m, 0));
    let (a, b, cz) = (x(fc.skid).fmul_m(rd::<f32>(m, 0x10)), x(fc.skid).fmul_m(rd::<f32>(m, 0x14)), x(fc.skid).fmul_m(rd::<f32>(m, 0x18)));
    turn(this, chain3(a, b, cz, rd::<f32>(fh, 0xC), tm), p);
}

/// The resistance tail (0x6DA900): the turn speed in the vehicle's frame is damped per axis by `turnRes ^ timeStep`
/// raised to a speed-dependent exponent (planes `2 |v|`, RC plane `6 |v|`, heli / autogyro `|v| + 1`, else 1); an axis
/// with a speed resistance > 0 instead gets a counter torque from the damped difference (times the exponent) about
/// forward / up / right. Then the move speed is held to 1.5 and the turn speed to 0.2.
fn resistance(fc: &Fc) {
    let this = fc.this;
    let fh = rd::<GPtr<u8>>(this, 0x388);
    let mut dd = [0f32; 3];
    let tr = [0x40, 0x44, 0x48].map(|o| f32_of(pow(x(rd::<f32>(fh, o)), ts())));
    let mut lt = [0f32; 3];
    let r = multiply3x3_vm(vp(&mut lt), this.byte_add(0x50), mat(this));
    let mut lt = rv(r, 0);
    let ms = this.byte_add(0x44);
    let exp = match fc.model {
        3..=5 => {
            let (mx, my, mz) = (x(rd::<f32>(ms, 0)), x(rd::<f32>(ms, 4)), x(rd::<f32>(ms, 8)));
            let s = mx.fmul(mx).fadd(my.fmul(my)).fadd(mz.fmul(mz)).fsqrt();
            f32_of(s.fadd(s))
        }
        1 => f32_of(X87::from_f64_exact(vector_magnitude(ms.cast())).fmul_m(6.0)),
        6 | 8 => f32_of(X87::from_f64_exact(vector_magnitude(ms.cast())).fadd_m(1.0)),
        _ => 1.0,
    };
    let fh = rd::<GPtr<u8>>(this, 0x388);
    for i in 0..3 {
        let sr = rd::<f32>(fh, 0x4C + 4 * i as isize);
        if gt(x(sr), x(0.0)) {
            let b = x(lt[i]).fmul_m(lt[i]).fmul_m(sr).fadd_m(1.0).fdivr_m(tr[i]);
            dd[i] = f32_of(pow(b, ts()).fmul_m(lt[i]).fsub_m(lt[i]));
        } else {
            lt[i] = f32_of(pow(x(tr[i]), exp).fmul_m(lt[i]));
        }
    }
    let mut out = [0f32; 3];
    let r = multiply3x3(vp(&mut out), mat(this), vp(&mut lt));
    let ts3 = rv(r, 0);
    for (k, v) in ts3.iter().enumerate() {
        wr::<f32>(this, 0x50 + 4 * k as isize, *v);
    }
    let dd = dd.map(|d| f32_of(x(d).fmul_m(exp)));
    for (i, ax, pt) in [(0usize, 0x10isize, 0x20isize), (1, 0x20, 0), (2, 0, 0x10)] {
        if fcom_ne(x(dd[i]), x(0.0)) {
            let m = mat(this);
            let n1 = c(0xBF80_0000);
            let a = x(rd::<f32>(m, ax)).fmul_m(n1);
            let b = x(rd::<f32>(m, ax + 4)).fmul_m(n1);
            let cz = f32_of(x(rd::<f32>(m, ax + 8)).fmul_m(n1));
            let da = f32_of(x(dd[i]).fmul(a));
            let db = x(dd[i]).fmul(b);
            let dc = x(dd[i]).fmul_m(cz);
            let tm = x(rd::<f32>(this, 0x90));
            let f = [f32_of(x(da).fmul(tm)), f32_of(db.fmul(tm)), f32_of(dc.fmul(tm))];
            turn(this, f, add3(fc.com, rv(m, pt)));
        }
    }
    clamp_speed(this.byte_add(0x44), c(0x4010_0000), c(0x3FC0_0000));
    clamp_speed(this.byte_add(0x50), c(0x3D23_D70A), c(0x3E4C_CCCD));
}

/// `if (|v|² > lim2) v *= k / |v|` with the original's stores (0x6DADBA / 0x6DAE4A).
fn clamp_speed(v: GPtr<u8>, lim2: f32, k: f32) {
    let (vx, vy, vz) = (x(rd::<f32>(v, 0)), x(rd::<f32>(v, 4)), x(rd::<f32>(v, 8)));
    let sq = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz));
    let sqf = f32_of(sq);
    if gt(sq, x(lim2)) {
        let a = x(rd::<f32>(v, 0)).fmul_m(k);
        let b = x(rd::<f32>(v, 4)).fmul_m(k);
        let cz = f32_of(x(rd::<f32>(v, 8)).fmul_m(k));
        let inv = x(sqf).fsqrt().fdivr_m(1.0);
        let invf = f32_of(inv);
        wr::<f32>(v, 0, f32_of(inv.fmul(a)));
        wr::<f32>(v, 4, f32_of(b.fmul_m(invf)));
        wr::<f32>(v, 8, f32_of(x(cz).fmul_m(invf)));
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_FLYING_CONTROL];
