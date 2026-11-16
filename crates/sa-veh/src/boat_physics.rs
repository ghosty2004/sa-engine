//! FRONTIER-2 turn 7, lane B (batch `frontier2-b16`): `CVehicle::ProcessBoatControl` (0x6DBCE0), the `boat`
//! scenario's blocker, and `CVehicle::DoBoatSplashes` (0x6DD130). Ported from the 1.0 US bytes; gta-reversed's
//! reading was an aid only. Where they differ the bytes win: the drowning flag (+0x42B bit 6) stays set while the
//! hull is upside down (gta-reversed clears it at once), the Skimmer's take-off timer resets only once the time is
//! strictly past it, the thrust depth is clamped by comparing the depth (not its square) with 1, a negative
//! post-collision z force becomes 0 (not `max`), and the wave strength is `(int16)_ftol(...)`, not `floorf`.
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::bindings::{
    cross_product, matrix_mul_vec, random_float_in_range, vector_dot_product as dot, vector_magnitude_sqr,
};
use sa_core::field::{ptr_of as buf, rd3f as rd3, v3_ptr as vp};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::{ge_or_nan, gt, le_or_nan, lt};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::bindings::pad_get_hand_brake;
use crate::common::{entity_get_col_model as col_model, f32_of, ftol32, g, rd, wr, x};
use sa_pop::carctrl3::{time_ms, time_step as ts};
use sa_world::bindings::{
    audio_add_event, fx_add_particle, fx_prt_mult_ctor, get_pad_from_player, get_water_level, multiply3x3, multiply3x3_vm,
    physical_apply_move_force_5429f0 as apply_move_force, physical_apply_turn_force as apply_turn_force,
    vec_mul_assign, vector_add_assign, vector_mul_scalar, vector_scale, vector_sub, vector_sub_assign,
};
use sa_stream::bindings::{entity_is_visible, ped_is_player};
use sa_world::physical::shared::{ci_pow, cv};
use sa_world::world::common::entity_pos;

orig! {
    /// `cBuoyancy::ProcessBuoyancyBoat(CVehicle*, float, CVector*, CVector*, bool)` (0x6C3030)
    fn process_buoyancy_boat(b: GPtr<u8>, v: GPtr<u8>, buoyancy: f32, turn: GPtr<u8>, force: GPtr<u8>, under: u32) -> u8
        = thiscall 0x6C3030 => sa_world::physical::buoyancy_boat::process_buoyancy_boat;
    /// `CBoat::AddWakePoint(CVector)` (0x6F2550)
    fn add_wake_point(b: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x6F2550 => crate::boat2::add_wake_point;
    /// `CVehicle::ApplyBoatWaterResistance(tBoatHandlingData*, float)` (0x6D2740)
    fn apply_boat_water_resistance(v: GPtr<u8>, h: GPtr<u8>, depth: f32) = thiscall 0x6D2740
        => crate::vehicle2::apply_boat_water_resistance;
}

/// `mod_Buoyancy` (0xC1C890) and its `m_fEntityWaterImmersion` (+0xBC).
const MOD_BUOYANCY: usize = 0xC1_C890;
const MODEL_SKIMMER: u16 = 0x1CC;

/// `k * v` per component (f32 stores), the original's inlined scale of a matrix row.
fn scaled(k: X87, v: GPtr<u8>) -> [f32; 3] {
    [f32_of(k.fmul_m(rd::<f32>(v, 0))), f32_of(k.fmul_m(rd::<f32>(v, 4))), f32_of(k.fmul_m(rd::<f32>(v, 8)))]
}

/// `(m.z*v.z + m.y*v.y) + v.x*m.x`, the original's inlined dot product of the move speed with a matrix row.
fn move_dot(this: GPtr<u8>, row: GPtr<u8>) -> X87 {
    x(rd::<f32>(this, 0x4C)).fmul_m(rd::<f32>(row, 8))
        .fadd(x(rd::<f32>(this, 0x48)).fmul_m(rd::<f32>(row, 4)))
        .fadd(x(rd::<f32>(row, 0)).fmul_m(rd::<f32>(this, 0x44)))
}

/// `CVector * float` (0x40FEC0) into `out`, read back.
fn mul_s(out: &mut [f32; 3], v: GPtr<u8>, s: f32) -> [f32; 3] {
    rd3(vector_mul_scalar(vp(out), v, s))
}

/// `CVehicle::ProcessBoatControl(tBoatHandlingData* h, float* lastDepth, bool collidedWithWorld, bool
/// postCollision)` (0x6DBCE0, thiscall `ret 0x10`).
///
/// `mod_Buoyancy.ProcessBuoyancyBoat(this, +0xA0, &turnPoint, &force, collided)`; false: not in water (+0x40 bit 8,
/// a boat's +0x5AC bit 0). In water when `force.z > ts * mass * 0.0008`; upside down (up.z < -0.6) and nearly still
/// (|move.x|, |move.y| < 0.05) the hull drowns (+0x42B bit 6) and the driver touches water: a player loses breath,
/// another ped gets the drowning damage event. The buoyancy force (x 0.03 for an upside-down slow Skimmer) moves
/// the hull and, after a world collision, turns it (x 0.4). The Skimmer's take-off: a nose-down dive (or a running
/// timer) pitches it up, brakes it along its nose and runs the 300 ms timer. The aquaplane lift `|v|^2 * aqForce *
/// ts * force.z * 0.5` (by the throttle; capped at `ts * aqLimit * mass * 0.008`) pushes along up at the aquaplane
/// offset. Then, upright enough, the wake point, the traction from the forward speed, the thrust point below the
/// stern and its depth in the water: the steered thrust (x depth, throttle, 40, engine acceleration, mass), the
/// side traction, and when slowing down the steering drag. Then the keel's side force (suspension bias), the
/// handbrake drag, the water resistance, the turn resistance by `pow(turnRes, ts)`, the wave hits (audio event 0x63,
/// a lift and a brake along the nose); the force is kept for the boat (+0x630) or as the automobile's +0x950.
#[hook(addr = 0x6DBCE0, conv = thiscall, name = "CVehicle::ProcessBoatControl", this = "CVehicle",
       expect = "6A FF 68 E3 87 84 00 64 A1 00 00 00 00",
       unverified = "frontier2-b16", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "FSIN / FCOS of the steering, FSQRT, _CIpow via orig::x87_binary; the damage event and the calculator live on the Rust stack; the SEH frame is not reproduced; the bool arguments are tested on their low byte")]
pub fn process_boat_control(this: GPtr<u8>, h: GPtr<u8>, last_depth: GPtr<u8>, collided: u32, post: u32) {
    let collided = collided & 0xFF != 0;
    let post = post & 0xFF != 0;
    let boat = if rd::<u32>(this, 0x594) == 5 { this } else { GPtr::NULL };
    let mut turn = [0f32; 3];
    let mut force = [0f32; 3];
    let ok = process_buoyancy_boat(GPtr::from_addr(MOD_BUOYANCY), this, rd::<f32>(this, 0xA0), vp(&mut turn),
        vp(&mut force), u32::from(collided));
    let mut on_water = false;
    if ok != 0 {
        on_water = control(this, h, last_depth, collided, post, boat, turn, force);
    } else {
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & !0x100);
    }
    if !boat.is_null() {
        wr::<u8>(boat, 0x5AC, (rd::<u8>(boat, 0x5AC) & !1) | u8::from(on_water));
    }
}

/// The in-water body of [`process_boat_control`]; returns the "on water" flag.
#[allow(clippy::too_many_arguments)]
fn control(this: GPtr<u8>, h: GPtr<u8>, last_depth: GPtr<u8>, collided: bool, post: bool, boat: GPtr<u8>,
    turn: [f32; 3], force: [f32; 3]) -> bool {
    let mat = || rd::<GPtr<u8>>(this, 0x14);
    let mass = || rd::<f32>(this, 0x8C);
    let model = || rd::<u16>(this, 0x22);
    let up_z = || x(rd::<f32>(mat(), 0x28));
    let mut on_water;
    // in water, drowning
    if lt(x(ts()).fmul_m(mass()).fmul_m(f32::from_bits(0x3A51_B718)), x(force[2])) {
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x100);
        on_water = true;
        let still = |o: isize| lt(x(rd::<f32>(this, o)).fabs(), X87::from_f64_exact(0.05));
        if lt(up_z(), x(-0.6)) && still(0x44) && still(0x48) {
            wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) | 0x40);
            let driver = rd::<GPtr<u8>>(this, 0x460);
            if !driver.is_null() {
                wr::<u32>(driver, 0x40, rd::<u32>(driver, 0x40) | 0x800_0000);
                crate::mvpd_v5::drown_occupant(this, driver);
            }
        } else {
            wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) & 0xBF);
        }
    } else {
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & !0x100);
        on_water = false;
        wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) & 0xBF);
    }
    let imm = g::<f32>(MOD_BUOYANCY + 0xBC).read();
    let mut force = force;
    let fp = vp(&mut force);
    let slow = |lim: f32| lt(x(rd::<f32>(this, 0x44)).fabs(), x(lim)) && lt(x(rd::<f32>(this, 0x48)).fabs(), x(lim));
    let used = if model() == MODEL_SKIMMER && lt(up_z(), x(-0.5)) && slow(0.2) {
        let mut t = [0f32; 3];
        rd3(vector_scale(vp(&mut t), 0.03, fp))
    } else {
        force
    };
    apply_move_force(this, cv(used));
    if collided {
        let k = |v: f32| f32_of(x(v).fmul_m(0.4));
        apply_turn_force(this, CVector::new(k(force[0]), k(force[1]), k(force[2])), cv(turn));
    }
    // the Skimmer's take-off
    if model() == MODEL_SKIMMER {
        let steer2 = || x(rd::<f32>(this, 0x498));
        let go = if steer2().fcom(x(0.0)).test_parity(0x44) {
            true
        } else if lt(x(rd::<f32>(mat(), 0x18)), x(-0.5)) && gt(up_z(), x(-0.5))
            && lt(x(rd::<f32>(this, 0x4C)), x(-0.15))
        {
            let m = x(ts()).fmul_m(mass());
            lt(x(f32::from_bits(0x38A7_C5AD)).fmul(m), x(force[2]))
                && lt(x(force[2]), x(0.4).fmul(m).fmul_m(0.008))
        } else {
            false
        };
        if go {
            let m = mat();
            let k = x(rd::<f32>(m, 0x18)).fmul_m(rd::<f32>(this, 0x90)).fmul_m(ts())
                .fmul_m(f32::from_bits(0xB932_4207)).fmul_m(force[2]);
            apply_turn_force(this, cv(scaled(k, m.byte_add(0x10))), cv(rd3(m.byte_add(0x20))));
            on_water = false;
            let s = move_dot(this, m.byte_add(0x10)).fmul_m(-0.5).fmul_m(mass());
            apply_move_force(this, cv(scaled(s, m.byte_add(0x10))));
            let now = X87::from_f64_exact(f64::from(time_ms()));
            if !steer2().fcom(x(0.0)).test_parity(0x44) {
                wr::<f32>(this, 0x498, f32_of(now.fadd_m(300.0)));
            } else if !le_or_nan(now, steer2()) {
                wr::<u32>(this, 0x498, 0);
            }
        }
    }
    // the aquaplane lift
    if !post && on_water && gt(up_z(), x(0.0)) {
        let (vx, vy, vz) = (x(rd::<f32>(this, 0x44)), x(rd::<f32>(this, 0x48)), x(rd::<f32>(this, 0x4C)));
        let f = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fmul_m(rd::<f32>(h, 0x10)).fmul_m(ts())
            .fmul_m(force[2]).fmul_m(0.5);
        let gas = x(rd::<f32>(this, 0x49C));
        let f = if model() == MODEL_SKIMMER {
            f.fmul(gas.fadd_m(1.0))
        } else if le_or_nan(gas, x(0.05)) {
            x(0.0)
        } else {
            f.fmul_m(rd::<f32>(this, 0x49C))
        };
        let lim = f32_of(x(ts()).fmul_m(rd::<f32>(h, 0x14)).fmul_m(mass()).fmul_m(0.008));
        let f = if le_or_nan(f, x(lim)) { f } else { x(lim) };
        let m = mat();
        let lift = scaled(f, m.byte_add(0x20));
        apply_move_force(this, cv(lift));
        let o = x(rd::<f32>(h, 0x18));
        let ofy = f32_of(o.fmul_m(rd::<f32>(m, 0x14)));
        let ofz = f32_of(o.fmul_m(rd::<f32>(m, 0x18)));
        let ofx = o.fmul_m(rd::<f32>(m, 0x10));
        let point = [f32_of(x(turn[0]).fsub(ofx)), f32_of(x(turn[1]).fsub_m(ofy)), f32_of(x(turn[2]).fsub_m(ofz))];
        apply_turn_force(this, cv(lift), cv(point));
    }
    let player = rd::<u8>(this, 0x36) & 0xF8 == 0;
    let mut pad = GPtr::<u8>::NULL;
    if player {
        let driver = rd::<GPtr<u8>>(this, 0x460);
        if !driver.is_null() && ped_is_player(driver) != 0 {
            pad = get_pad_from_player(driver);
        }
    }
    if gt(up_z(), x(-0.6)) {
        thrust(this, h, post, boat, on_water, player, pad);
    }
    let hd = || rd::<GPtr<u8>>(this, 0x384);
    // the keel's side force
    if x(rd::<f32>(hd(), 0xC0)).fcom(x(0.0)).test_parity(0x44) {
        let mut zax = [0.0, 0.0, 1.0];
        let mut r = [0f32; 3];
        cross_product(vp(&mut r), mat().byte_add(0x10), vp(&mut zax));
        let m = x(r[1]).fmul_m(rd::<f32>(this, 0x48)).fadd(x(r[2]).fmul_m(rd::<f32>(this, 0x4C)))
            .fadd(x(r[0]).fmul_m(rd::<f32>(this, 0x44)))
            .fmul_m(rd::<f32>(hd(), 0xC0)).fmul_m(ts()).fmul_m(imm).fmul_m(mass()).fmul_m(0.1);
        let ny = f32_of(x(r[1]).fchs().fmul_m(0.3));
        let rx3 = x(r[0]).fmul_m(0.3);
        let rx = f32_of(x(ny).fadd_m(r[0]));
        let ry = f32_of(rx3.fadd_m(r[1]));
        let m = m.fchs();
        apply_move_force(this, CVector::new(f32_of(x(rx).fmul(m)), f32_of(x(ry).fmul(m)), f32_of(x(r[2]).fmul(m))));
    }
    // the handbrake drag
    if player && pad_get_hand_brake(pad) != 0 {
        let m = mat();
        let d = move_dot(this, m.byte_add(0x10));
        if gt(d, x(0.0)) {
            let s = d.fmul_m(rd::<f32>(hd(), 0xBC)).fmul_m(ts()).fmul_m(imm).fmul_m(mass()).fmul_m(-0.1);
            apply_move_force(this, cv(scaled(s, m.byte_add(0x10))));
        }
    }
    if on_water && !post && !collided {
        apply_boat_water_resistance(this, h, imm);
    }
    // the turn resistance
    if (model() != MODEL_SKIMMER || !x(rd::<f32>(this, 0x498)).fcom(x(0.0)).test_parity(0x44)) && !collided {
        let p = |o: isize| f32_of(ci_pow(x(rd::<f32>(h, o)), x(ts())));
        let tr = [p(0x30), p(0x34), p(0x38)];
        let tsp = this.byte_add(0x50);
        let mut t = [0f32; 3];
        let r = rd3(multiply3x3_vm(vp(&mut t), tsp, mat()));
        for (i, v) in r.into_iter().enumerate() {
            wr::<f32>(tsp, 4 * i as isize, v);
        }
        let sx = x(rd::<f32>(tsp, 0));
        let k = f32_of(sx.fmul(sx).fmul_m(1000.0).fadd_m(1.0).fdivr_m(tr[0]).fmul_m(rd::<f32>(tsp, 0))
            .fsub_m(rd::<f32>(tsp, 0)));
        wr::<f32>(tsp, 4, f32_of(x(tr[1]).fmul_m(rd::<f32>(tsp, 4))));
        wr::<f32>(tsp, 8, f32_of(x(tr[2]).fmul_m(rd::<f32>(tsp, 8))));
        let r = rd3(multiply3x3(vp(&mut t), mat(), tsp));
        for (i, v) in r.into_iter().enumerate() {
            wr::<f32>(tsp, 4 * i as isize, v);
        }
        let k = x(k).fmul_m(rd::<f32>(this, 0x90));
        let m = mat();
        let tf = scaled(k, m.byte_add(0x20));
        let com = rd3(multiply3x3(vp(&mut t), m, this.byte_add(0xA4)));
        let point = [
            f32_of(x(rd::<f32>(m, 0x10)).fadd_m(com[0])),
            f32_of(x(rd::<f32>(m, 0x14)).fadd_m(com[1])),
            f32_of(x(rd::<f32>(m, 0x18)).fadd_m(com[2])),
        ];
        apply_turn_force(this, cv(tf), cv(point));
    }
    // the waves
    let wave = ftol32(x(imm).fsub_m(rd::<f32>(last_depth, 0)).fmul_m(10000.0)) as i16;
    if !post && on_water && gt(up_z(), x(0.0)) {
        let wf = f32_of(X87::from_i32(i32::from(wave)));
        let vol = x(wf).fmul_m(rd::<f32>(h, 0x1C));
        if gt(vol, x(200.0)) {
            audio_add_event(this.byte_add(0x138), 0x63, f32_of(vol));
        }
        if wave > 200 {
            let mut w = [0f32; 3];
            let hd = hd();
            w[2] = f32_of(vector_magnitude_sqr(this.byte_add(0x44)).fmul_m(wf).fmul_m(0.001));
            if gt(x(w[2]).fadd_m(rd::<f32>(this, 0x4C)), x(rd::<f32>(hd, 0x94))) {
                w[2] = f32_of(x(rd::<f32>(hd, 0x94)).fsub_m(rd::<f32>(this, 0x4C)));
            }
            if lt(x(w[2]), x(0.0)) {
                w[2] = 0.0;
            }
            let fwd = mat().byte_add(0x10);
            let d = f32_of(dot(this.byte_add(0x44), fwd));
            let s = f32_of(x(wf).fmul_m(rd::<f32>(hd, 0x98)).fmul_m(f32::from_bits(0xBC23_D70A)));
            let (mut t1, mut t2) = ([0f32; 3], [0f32; 3]);
            let sf = vector_scale(vp(&mut t1), s, fwd);
            let b = vector_mul_scalar(vp(&mut t2), sf, d);
            vector_add_assign(vp(&mut w), b);
            vec_mul_assign(vp(&mut w), mass());
            apply_move_force(this, cv(w));
            apply_turn_force(this, cv(w), cv(turn));
        }
    }
    if !boat.is_null() {
        for (i, v) in force.into_iter().enumerate() {
            wr::<f32>(boat, 0x630 + 4 * i as isize, v);
        }
    } else if rd::<u32>(this, 0x590) == 0 {
        wr::<f32>(this, 0x950, f32_of(cv(force).magnitude_x87()));
    }
    wr::<f32>(last_depth, 0, imm);
    on_water
}

/// The thrust block of [`control`] (0x6DC3B5..0x6DCB9A): wake point, traction, the thrust point's depth, the
/// steered thrust, the side traction and the steering drag.
#[allow(clippy::too_many_arguments)]
fn thrust(this: GPtr<u8>, h: GPtr<u8>, post: bool, boat: GPtr<u8>, on_water: bool, player: bool, pad: GPtr<u8>) {
    let mat = || rd::<GPtr<u8>>(this, 0x14);
    let gas = || x(rd::<f32>(this, 0x49C));
    let hd = || rd::<GPtr<u8>>(this, 0x384);
    let mut speed = 1.0f32;
    if le_or_nan(gas().fabs(), x(0.05)) {
        let v = CVector::new(rd::<f32>(this, 0x44), rd::<f32>(this, 0x48), 0.0).magnitude_2d_x87();
        speed = f32_of(v);
        if le_or_nan(v, x(0.01)) {
            return;
        }
    }
    if !boat.is_null() && on_water && gt(x(speed), X87::from_f64_exact(0.05)) {
        let p = entity_pos(this);
        let (px, py, pz) = (rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
        col_model(this);
        add_wake_point(boat, px, py, pz);
    }
    let mut traction = 1.0f32;
    if player {
        let t = f32_of(move_dot(this, mat().byte_add(0x10)).fmul_m(rd::<f32>(hd(), 0xA8)));
        let t = if pad_get_hand_brake(pad) != 0 { x(t).fmul_m(0.5) } else { x(t) };
        traction = f32_of(x(1.0).fsub(t));
        if lt(x(traction), x(0.0)) {
            traction = 0.0;
        } else if gt(x(traction), x(1.0)) {
            traction = 1.0;
        }
    }
    let cm = col_model(this);
    let mut tp_in = [0.0, f32_of(x(rd::<f32>(cm, 4)).fmul_m(rd::<f32>(h, 4))), f32_of(x(rd::<f32>(cm, 8)).fmul_m(rd::<f32>(h, 8)))];
    let mut t = [0f32; 3];
    let mut tp = rd3(multiply3x3(vp(&mut t), mat(), vp(&mut tp_in)));
    let p = entity_pos(this);
    let w = [
        f32_of(x(tp[0]).fadd_m(rd::<f32>(p, 0))),
        f32_of(x(tp[1]).fadd_m(rd::<f32>(p, 4))),
        f32_of(x(tp[2]).fadd_m(rd::<f32>(p, 8))),
    ];
    let a = x(traction).fmul_m(rd::<f32>(this, 0x494)).fchs();
    let sin = f32_of(a.fsin());
    let cos = f32_of(a.fcos());
    let mut level = 0f32;
    get_water_level(w[0], w[1], w[2], buf(&mut level), 1, GPtr::NULL);
    if ge_or_nan(x(w[2]).fsub_m(0.5), x(level)) {
        if !boat.is_null() {
            wr::<u8>(boat, 0x5AC, rd::<u8>(boat, 0x5AC) & 0xFD);
        }
        return;
    }
    let d = x(level).fsub_m(w[2]).fadd_m(0.5);
    let depth = if le_or_nan(d, x(1.0)) { f32_of(d.fmul(d)) } else { 1.0 };
    if !boat.is_null() {
        wr::<u8>(boat, 0x5AC, rd::<u8>(boat, 0x5AC) | 2);
    }
    let g = gas().fabs();
    let mut slowing = false;
    let (mut t1, mut t2, mut t3) = ([0f32; 3], [0f32; 3], [0f32; 3]);
    if le_or_nan(g, x(0.01)) || rd::<u16>(this, 0x22) == MODEL_SKIMMER {
        slowing = true;
    } else {
        if lt(g, x(0.5)) {
            slowing = true;
        }
        let st = x(rd::<f32>(this, 0x494));
        let st = if lt(st, x(0.0)) { st.fchs() } else { st };
        let mut steer = [f32_of(x(sin).fchs()), cos, f32_of(st.fmul_m(-1.0))];
        let mut f = multiply3x3(vp(&mut t1), mat(), vp(&mut steer));
        f = vector_scale(vp(&mut t2), depth, f);
        f = vector_mul_scalar(vp(&mut t3), f, rd::<f32>(this, 0x49C));
        f = vector_mul_scalar(vp(&mut t1), f, 40.0);
        f = vector_mul_scalar(vp(&mut t2), f, rd::<f32>(hd(), 0x7C));
        f = vector_mul_scalar(vp(&mut t3), f, rd::<f32>(this, 0x8C));
        let mut fv = rd3(f);
        if gt(x(fv[2]), x(0.2)) {
            let k = x(1.2).fsub_m(fv[2]);
            fv[2] = f32_of(k.fmul(k).fadd_m(0.2));
        }
        if post {
            if lt(gas(), x(0.0)) {
                fv[0] = f32_of(x(fv[0]).fmul_m(5.0));
                fv[1] = f32_of(x(fv[1]).fmul_m(5.0));
            }
            if lt(x(fv[2]), x(0.0)) {
                fv[2] = 0.0;
            }
            apply_move_force(this, cv(mul_s(&mut t1, vp(&mut fv), ts())));
        } else {
            apply_move_force(this, cv(mul_s(&mut t1, vp(&mut fv), ts())));
            let m = mat();
            let a = vector_scale(vp(&mut t1), rd::<f32>(h, 0xC), m.byte_add(0x20));
            let point = rd3(vector_sub(vp(&mut t2), vp(&mut tp), a));
            let ft = mul_s(&mut t3, vp(&mut fv), ts());
            apply_turn_force(this, cv(ft), cv(point));
            let m = mat();
            let s = f32_of(x(fv[1]).fmul_m(rd::<f32>(m, 4)).fadd(x(fv[2]).fmul_m(rd::<f32>(m, 8)))
                .fadd(x(fv[0]).fmul_m(rd::<f32>(m, 0))).fmul_m(rd::<f32>(hd(), 0x28)).fchs());
            let side = vector_scale(vp(&mut t1), s, m);
            let side = mul_s(&mut t2, side, ts());
            apply_turn_force(this, cv(side), cv(rd3(m.byte_add(0x20))));
        }
        if gt(gas(), x(0.0)) && rd::<u8>(this, 0x36) & 0xF8 == 0 {
            // the stern point, transformed and dropped (gta-reversed: "this code does nothing")
            let cm = col_model(this);
            let mut v = [0.0, rd::<f32>(cm, 4), 0.0];
            multiply3x3(vp(&mut t1), mat(), vp(&mut v));
        }
    }
    if post || !slowing {
        return;
    }
    // the steering drag
    let mut tl = f32_of(dot(this.byte_add(0x44), mat().byte_add(0x10)).fmul_m(rd::<f32>(hd(), 0xA4)));
    let lim = x(rd::<f32>(this, 0x90)).fmul_m(0.01);
    if !le_or_nan(x(tl), lim) {
        tl = f32_of(lim);
    }
    let g = gas().fabs();
    if !le_or_nan(g, x(0.01)) {
        let k = x(0.55).fsub(g).fmul_m(tl);
        tl = f32_of(if player { k.fmul_m(2.6) } else { k.fmul_m(5.0) });
    }
    let flip = (lt(gas(), x(0.0)) && gt(x(tl), x(0.0))) || (gt(gas(), x(0.0)) && lt(x(tl), x(0.0)));
    if flip {
        tl = f32_of(x(tl).fmul_m(-1.0));
    }
    let m = mat();
    let mut v = [f32_of(x(sin).fchs()), 0.0, 0.0];
    let a = vector_mul_scalar(vp(&mut t1), vp(&mut v), tl);
    let b = multiply3x3(vp(&mut t2), m, a);
    let mut drag = rd3(vector_scale(vp(&mut t3), depth, b));
    apply_move_force(this, cv(mul_s(&mut t1, vp(&mut drag), ts())));
    apply_turn_force(this, cv(mul_s(&mut t1, vp(&mut drag), ts())), cv(tp));
    let step = if le_or_nan(x(0.01), x(ts())) { x(ts()) } else { x(0.01) };
    let s = f32_of(step.fdiv_m(traction).fmul_m(tl).fmul_m(sin).fmul_m(-0.75));
    let m = mat();
    let r = rd3(vector_scale(vp(&mut t1), s, m));
    apply_turn_force(this, cv(r), cv(rd3(m.byte_add(0x20))));
}

/// One splash particle of [`do_boat_splashes`]: velocity `((-forward * rand(0.8, 1.2)) -/+ right * rand(0.3, 0.7) +
/// up * rand(0.8, 1.2)) * k` (each step one of the original's CVector operator calls), emitted at `pos`.
fn splash(this: GPtr<u8>, pos: &mut [f32; 3], k: f32, side_sub: bool, mult: GPtr<u8>) {
    let m = rd::<GPtr<u8>>(this, 0x14);
    let mut base = [
        f32_of(x(rd::<f32>(m, 0x10)).fchs()),
        f32_of(x(rd::<f32>(m, 0x14)).fchs()),
        f32_of(x(rd::<f32>(m, 0x18)).fchs()),
    ];
    let mut t = [0f32; 3];
    let r = f32_of(random_float_in_range(0.8, 1.2));
    let mut vel = mul_s(&mut t, vp(&mut base), r);
    let r = f32_of(random_float_in_range(0.3, 0.7));
    let side = vector_mul_scalar(vp(&mut t), rd::<GPtr<u8>>(this, 0x14), r);
    if side_sub {
        vector_sub_assign(vp(&mut vel), side);
    } else {
        vector_add_assign(vp(&mut vel), side);
    }
    let r = f32_of(random_float_in_range(0.8, 1.2));
    let up = vector_mul_scalar(vp(&mut t), rd::<GPtr<u8>>(this, 0x14).byte_add(0x20), r);
    vector_add_assign(vp(&mut vel), up);
    vec_mul_assign(vp(&mut vel), k);
    fx_add_particle(g::<GPtr<u8>>(0xA9_AE04).read(), vp(pos), vp(&mut vel), 0, mult, 0xBF80_0000, 1.2, 0.6, 0);
}

/// `CVehicle::DoBoatSplashes(float damping)` (0x6DD130, thiscall `ret 4`).
///
/// Nothing when `|move|^2 <= 0.0025`, the hull is not upright, the camera looks forward in first person, the vehicle
/// is invisible, a cruising (mission 1) vehicle on a frame with bit 1 set, or the camera is 80 m away (2D) or more.
/// Strength `v = sqrt(|move|^2) * 0.075 * damping` (x3 capped at 0.5 for the Skimmer, else capped at 1); none up to
/// 0.15. `s = v * 0.75` (doubled, capped at 1, for a cruising vehicle); alpha = `min((uint8)_ftol(s * 128), 64)`,
/// faded by `(80 - d) * (1/30)` beyond 50 m; size `min((s * 10 + 0.75) * 0.1, 1)`, life `min(rand(0.8, 1.2) * (2s +
/// 0.3) * 0.2, 1)`. Two `g_fx` boat splashes (0xA9AE04) at the hull's sides (`x = bbox.min.x / max.x * 0.7`, `y =
/// bbox.max.y / 2`, `z = 0`; the Skimmer's multipliers 0x8D3678 / 0x8D3674), both thrown back along `-forward`
/// (gta-reversed reads `-right.y, -right.z` there), the left one against `right`, the right one along it.
#[hook(addr = 0x6DD130, conv = thiscall, name = "CVehicle::DoBoatSplashes", this = "CVehicle",
       expect = "81 EC 80 00 00 00 56 8B F1",
       unverified = "frontier2-b16", route = "L2", hazards = "x87-transcendental",
       note = "FSQRT of the speed; the particle multiplier lives on the Rust stack; the three GetRandomNumberInRange calls per splash write their argument slots in the original")]
pub fn do_boat_splashes(this: GPtr<u8>, damping: f32) {
    let (vx, vy, vz) = (x(rd::<f32>(this, 0x44)), x(rd::<f32>(this, 0x48)), x(rd::<f32>(this, 0x4C)));
    let sq_x = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz));
    let sq = f32_of(sq_x);
    if le_or_nan(sq_x, x(f32::from_bits(0x3B23_D70B))) || le_or_nan(x(rd::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x28)), x(0.0)) {
        return;
    }
    if crate::automobile_render::get_looking_forward_first_person(GPtr::from_addr(0xB6_F028)) != 0
        || entity_is_visible(this) == 0
    {
        return;
    }
    let cruise = rd::<u8>(this, 0x3BA) == 1;
    if cruise && g::<u8>(0xB7_CB4C).read() & 2 != 0 {
        return;
    }
    let mut d = [0f32; 3];
    vector_sub(vp(&mut d), entity_pos(this), entity_pos(GPtr::from_addr(0xB6_F028)));
    d[2] = 0.0;
    let dist_x = cv(d).magnitude_x87();
    let dist = f32_of(dist_x);
    if ge_or_nan(dist_x, x(80.0)) {
        return;
    }
    let skimmer = rd::<u16>(this, 0x22) == MODEL_SKIMMER;
    let mut v = x(sq).fsqrt().fmul_m(0.075).fmul_m(damping);
    if skimmer {
        v = v.fmul_m(3.0);
        if !le_or_nan(v, x(0.5)) {
            v = x(0.5);
        }
    } else if !le_or_nan(v, x(1.0)) {
        v = x(1.0);
    }
    if le_or_nan(v, x(0.15)) {
        return;
    }
    let mut s = f32_of(v.fmul_m(0.75));
    if cruise {
        let t = x(s).fadd(x(s));
        s = if ge_or_nan(t, x(1.0)) { 1.0 } else { f32_of(t) };
    }
    let a = ftol32(x(s).fmul_m(128.0)) as u8;
    let mut alpha = a.min(0x40);
    let k12 = x(s).fmul_m(10.0);
    let k = f32_of(k12);
    let size = f32_of(k12.fadd_m(0.75));
    let r = random_float_in_range(0.8, 1.2);
    let life = f32_of(r.fmul(x(s).fadd(x(s)).fadd_m(0.3)));
    if !le_or_nan(x(dist), x(50.0)) {
        alpha = ftol32(x(80.0).fsub_m(dist).fmul_m(f32::from_bits(0x3D08_8889)).fmul(X87::from_i32(i32::from(alpha))))
            as u8;
    }
    let mut mult_b = [0u32; 7];
    let mult = buf(&mut mult_b);
    fx_prt_mult_ctor(mult, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0);
    let clamp1 = |v: X87| if ge_or_nan(v, x(1.0)) { 1.0 } else { f32_of(v) };
    wr::<f32>(mult, 0xC, clamp1(X87::from_i32(i32::from(alpha)).fmul_m(f32::from_bits(0x3B80_8081))));
    wr::<f32>(mult, 0x10, clamp1(x(size).fmul_m(0.1)));
    wr::<f32>(mult, 0x18, clamp1(x(life).fmul_m(0.2)));
    let cm = col_model(this);
    let (min_x, min_z) = (rd::<f32>(cm, 0), rd::<f32>(cm, 8));
    let cm = col_model(this);
    let max_x = rd::<f32>(cm, 0xC);
    let max_y = rd::<f32>(cm, 0x10);
    let (xm, zm) = if skimmer { (g::<f32>(0x8D_3678).read(), x(g::<f32>(0x8D_3674).read())) } else { (0.7, x(0.0)) };
    let half_y = f32_of(x(max_y).fmul_m(0.5));
    let z = f32_of(zm.fmul_m(min_z));
    let mut t = [0f32; 3];
    let mut v0 = [f32_of(x(min_x).fmul_m(xm)), half_y, z];
    let mut p0 = rd3(matrix_mul_vec(vp(&mut t), rd::<GPtr<u8>>(this, 0x14), vp(&mut v0)));
    splash(this, &mut p0, k, true, mult);
    let mut v1 = [f32_of(x(max_x).fmul_m(xm)), half_y, z];
    let mut p1 = rd3(matrix_mul_vec(vp(&mut t), rd::<GPtr<u8>>(this, 0x14), vp(&mut v1)));
    splash(this, &mut p1, k, false, mult);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_BOAT_CONTROL, HOOK_DO_BOAT_SPLASHES];
