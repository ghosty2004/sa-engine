//! FRONTIER-2 turn 6, lane B (batch `frontier2-b15`): the plane's per-frame flight and render bodies,
//! `CPlane::ProcessFlyingCarStuff` (0x6CB7C0) and `CPlane::PreRender` (0x6C94A0). Ported from the 1.0 US bytes
//! (gta-reversed only calls the original); docs/re/specs/VEH/CPlane.md was a reading aid (where it differs, the bytes
//! win: the Hydra's hover call passes `-leftRight`, not `-upDown`). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::field::v3_ptr as vp;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_core::crt::rand_at;
use sa_math::fpu::{gt, le_or_nan, lt};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, rd, wr, x, xi};
use crate::flying_control::{height_above_ceiling, pos_z};
use crate::mvpb_a1::flying_control;
use crate::plane_damage::{get_aeroplane_comp_status, set_aeroplane_comp_status};
use crate::bindings::get_flying_pointer;
use sa_core::bindings::matrix_mul_vec;
use sa_world::bindings::{fx_add_particle, fx_prt_mult_ctor};
use sa_world::world::common::entity_pos;
use sa_world::world::break_object::rand_unit as rand01;
use sa_world::world::stats_travel::speed_sq;
use sa_core::field::gf;
use sa_pop::carctrl3::time_step as ts;

orig! {
    /// `CPlaceable::GetRoll()` (0x420B30; ST0)
    fn placeable_get_roll(p: GPtr<u8>) -> X87 = reg("args = ecx") 0x420B30 => sa_world::world::entity::get_roll;
}

/// `fild dword` of a value the original then fixes up with `+ 4294967296.0` when negative: the u32 as a float.
fn xu(v: u32) -> X87 {
    X87::from_f64_exact(f64::from(v))
}
/// `((hi - lo) * r) + lo` with `lo` and `hi` as the f32 the original keeps in its locals.
fn lerp(hi: f32, lo: f32, r: X87) -> X87 {
    x(hi).fsub_m(lo).fmul(r).fadd_m(lo)
}

/// The damaged-surface step of one control value (rudder +0x988, elevators +0x98C, ailerons +0x990): the value is
/// damped by `status * 0.2` and jittered by `R(hi) * status^2 * ts * SF`; the matching bouncing panel's y offset
/// jitters by `R(hi2)` likewise and the copy passed to `FlyingControl` moves by `10 * panel.rot.y` (added for comp 19).
struct Surface {
    field: isize,
    hi: usize,
    hi2: usize,
    ten: usize,
    /// `fild` of the status rounded through the f32 slot first (rudder) or kept extended (elevators, ailerons)
    rounded_damp: bool,
    site: u32,
    site2: u32,
}

#[allow(clippy::too_many_arguments)]
fn surface(this: GPtr<u8>, s: &Surface, st: i32, comp: u32, pi: i32, sf: f32, copy: &mut f32) {
    let stf = xi(st);
    let damp = if s.rounded_damp {
        x(1.0).fsub_m(f32_of(stf.fmul_m(f32::from_bits(0x3E4C_CCCD))))
    } else {
        stf.fmul_m(f32::from_bits(0x3E4C_CCCD)).fsubr_m(1.0)
    };
    wr::<f32>(this, s.field, f32_of(damp.fmul_m(rd::<f32>(this, s.field))));
    let hi = gf(s.hi);
    let lo = f32_of(x(hi).fchs());
    let r = rand01(s.site);
    let st2 = f32_of(xi(st.wrapping_mul(st)));
    let v = lerp(hi, lo, r).fmul_m(st2).fmul_m(ts()).fmul_m(sf).fadd_m(rd::<f32>(this, s.field));
    wr::<f32>(this, s.field, f32_of(v));
    *copy = f32_of(v);
    if pi < 0 {
        return;
    }
    let panel = this.byte_add(pi as isize * 0x20);
    let hi2 = gf(s.hi2);
    let lo2 = f32_of(x(hi2).fchs());
    let r2 = rand01(s.site2);
    let p = lerp(hi2, lo2, r2).fmul_m(st2).fmul_m(ts()).fmul_m(sf).fadd_m(rd::<f32>(panel, 0x6C4));
    wr::<f32>(panel, 0x6C4, f32_of(p));
    let t = x(gf(s.ten)).fmul_m(rd::<f32>(panel, 0x6B8));
    *copy = if comp == 0x13 { f32_of(t.fadd_m(*copy)) } else { f32_of(t.fsubr_m(*copy)) };
}

const RUDDER: Surface =
    Surface { field: 0x988, hi: 0x8D_3440, hi2: 0x8D_343C, ten: 0x8D_3438, rounded_damp: true, site: 0x6C_BAF9, site2: 0x6C_BB67 };
const ELEVATOR: Surface =
    Surface { field: 0x98C, hi: 0x8D_3434, hi2: 0x8D_3430, ten: 0x8D_342C, rounded_damp: false, site: 0x6C_BC12, site2: 0x6C_BC80 };
const AILERON: Surface =
    Surface { field: 0x990, hi: 0x8D_3428, hi2: 0x8D_3424, ten: 0x8D_3420, rounded_damp: false, site: 0x6C_BD37, site2: 0x6C_BDA5 };

/// One smoke puff from damaged component `comp` (0x6CBE20-0x6CC0D4): not while drowning; below speed factor 0.3 only
/// one frame in eight; beyond 80 m of the camera only for the player's plane; at the component frame's position, with
/// the move speed * 12.5 jittered by 0.9..1.1 per axis, a random life and a position jitter of `R(smoke)`.
fn smoke_puff(this: GPtr<u8>, comp: u32, smoke: f32, sf: f32) {
    if rd::<u8>(this, 0x42B) & 0x40 != 0 {
        return;
    }
    if le_or_nan(x(sf), x(f32::from_bits(0x3E99_999A))) && rand_at(0x6C_BE43) & 7 != 0 {
        return;
    }
    let cam = entity_pos(GPtr::from_addr(0xB6_F028));
    let vp_ = entity_pos(this);
    let dx = x(rd::<f32>(vp_, 0)).fsub_m(rd::<f32>(cam, 0));
    let dy = x(rd::<f32>(vp_, 4)).fsub_m(rd::<f32>(cam, 4));
    let dz = x(rd::<f32>(vp_, 8)).fsub_m(rd::<f32>(cam, 8));
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if !lt(d2, x(6400.0)) && rd::<u8>(this, 0x36) & 0xF8 != 0 {
        return;
    }
    let node = rd::<GPtr<u8>>(this, 0x648 + comp as isize * 4);
    if node.is_null() {
        return;
    }
    let mut np = [rd::<f32>(node, 0x40), rd::<f32>(node, 0x44), rd::<f32>(node, 0x48)];
    let mut tmp = [0f32; 3];
    let r = matrix_mul_vec(vp(&mut tmp), rd::<GPtr<u8>>(this, 0x14), vp(&mut np));
    let mut p = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
    let mut mult = [0u32; 7];
    let mp: GPtr<u8> = GPtr::from_raw(mult.as_mut_ptr()).cast();
    fx_prt_mult_ctor(mp, 0.0, 0.0, 0.0, f32::from_bits(0x3E4C_CCCD), 1.0, 1.0, smoke);
    let q = f32::from_bits(0x3E80_0000);
    let k50 = 50.0f32;
    let mut vel = [
        f32_of(x(rd::<f32>(this, 0x44)).fmul_m(q).fmul_m(k50)),
        f32_of(x(rd::<f32>(this, 0x48)).fmul_m(q).fmul_m(k50)),
        f32_of(x(f32_of(x(rd::<f32>(this, 0x4C)).fmul_m(q))).fmul_m(k50)),
    ];
    for (i, site) in [0x6C_BF80u32, 0x6C_BFA7, 0x6C_BFCE].into_iter().enumerate() {
        let j = rand01(site).fmul_m(f32::from_bits(0x3E4C_CCD0)).fadd_m(f32::from_bits(0x3F66_6666));
        vel[i] = f32_of(j.fmul_m(vel[i]));
    }
    wr::<f32>(mp, 0x18, f32_of(rand01(0x6C_BFF5)));
    let lo = f32_of(x(smoke).fchs());
    for (i, site) in [0x6C_C01Du32, 0x6C_C050, 0x6C_C083].into_iter().enumerate() {
        let r = rand01(site);
        p[i] = f32_of(lerp(smoke, lo, r).fadd_m(p[i]));
    }
    let fx = g::<GPtr<u8>>(0xA9_AE20).read();
    fx_add_particle(fx, vp(&mut p), vp(&mut vel), 0, mp, 0xBF80_0000, f32::from_bits(0x3F99_999A), f32::from_bits(0x3F19_999A), 0);
}

/// `CPlane::ProcessFlyingCarStuff()` (0x6CB7C0, vtable +0x110). Nothing for a time step `<= 0`. The damage wave
/// counter +0x9D0 advances by a random 0.25..1.75 (`PLANE_DAMAGE_WAVE_COUNTER_VAR`) times the frame's milliseconds;
/// the speed factor is `min(|moveSpeed| * 3, 1)`. A player plane under 250 health loses both static propellers and
/// skews its control copies by the roll. Except for the RC Baron (464) every damaged component 12..24 with a frame
/// then disturbs its control (propellers: the throttle by a sine of the wave; rudder / elevators / ailerons: see
/// [`surface`]) and may puff smoke ([`smoke_puff`]). Not driven (status other than 0 / 3 / 8): the propeller runs down
/// (the engine is switched off when it stops). Driven: the propeller speed +0x9C4 approaches the target of the throttle
/// (cut near the ceiling for the player / remote plane, which also pushes the throttle down), stops when drowning; then
/// `FlyingControl` (RC Baron model 1; the Hydra in hover with the Hunter's flying handling: model 6; else model 3)
/// while the engine runs and the plane turns its propeller or moves.
#[hook(addr = 0x6CB7C0, conv = thiscall, name = "CPlane::ProcessFlyingCarStuff", this = "CPlane", expect = "D9 05 5C CB B7 00 81 EC 98 00 00 00",
       unverified = "frontier2-b15", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "FSIN of the damage wave, FSQRT of the speed; rand at_site 0x6CB805 .. 0x6CC083 (16 sites); adds smoke particles")]
pub fn plane_process_flying_car_stuff(this: GPtr<u8>) {
    if le_or_nan(x(ts()), x(0.0)) && !x(ts()).is_nan() {
        return;
    }
    // damage wave
    let var = gf(0x8D_33E0);
    let hi = f32_of(x(var).fadd(X87::from_f64_exact(1.0)));
    let lo = f32_of(x(1.0).fsub_m(var));
    let r = rand01(0x6C_B805);
    let v = lerp(hi, lo, r);
    let n = ftol32(x(ts()).fmul_m(f32::from_bits(0x3CA3_D70A)).fmul_m(1000.0));
    let add = ftol32(v.fmul(xu(n as u32)));
    wr::<i32>(this, 0x9D0, rd::<i32>(this, 0x9D0).wrapping_add(add));
    // speed factor
    let m3 = speed_sq(this).fsqrt().fmul_m(3.0);
    let sf = if gt(m3, x(1.0)) { 1.0 } else { f32_of(speed_sq(this).fsqrt().fmul_m(3.0)) };
    let mut c_skid = rd::<f32>(this, 0x988);
    let mut c_ud = rd::<f32>(this, 0x98C);
    let mut c_lr = rd::<f32>(this, 0x990);
    if lt(x(rd::<f32>(this, 0x4C0)), x(250.0)) && rd::<u8>(this, 0x36) & 0xF8 == 0 {
        let dm = this.byte_add(0x5A0);
        set_aeroplane_comp_status(dm, 0xC, 2);
        set_aeroplane_comp_status(dm, 0xD, 2);
        c_skid = f32_of(x(c_skid).fadd_m(0.5));
        if lt(placeable_get_roll(this).fabs(), x(f32::from_bits(0x4016_CBE4))) {
            c_lr = f32_of(x(c_lr).fadd_m(0.75));
        }
        if gt(placeable_get_roll(this).fabs(), x(f32::from_bits(0x3FC9_0FDB))) {
            c_ud = f32_of(x(c_ud).fadd_m(0.5));
        }
    }
    if rd::<u16>(this, 0x22) != 0x1D0 {
        for comp in 12u32..25 {
            let st = get_aeroplane_comp_status(this.byte_add(0x5A0), comp) as i32;
            if rd::<u32>(this, 0x648 + comp as isize * 4) == 0 || st <= 0 {
                continue;
            }
            let pi = (0..4).find(|&i| i32::from(rd::<i16>(this, 0x6AC + i * 0x20)) == comp as i32).map_or(-1, |i| i as i32);
            let smoke: X87 = match comp {
                12..=15 => {
                    if gt(x(rd::<f32>(this, 0x994)), x(0.0)) {
                        let k = gf(0x8D_344C);
                        let ahi = f32_of(x(k).fadd_m(1.0));
                        let alo = f32_of(x(1.0).fsub_m(k));
                        let r = rand01(0x6C_BA2D);
                        let stf = xi(st);
                        let ang = xu(rd::<u32>(this, 0x9D0)).fmul_m(f32::from_bits(0x40C9_0FDB)).fdiv(xu(g::<u32>(0x8D_3448).read()));
                        let s = ang.fsin().fsub_m(1.0);
                        let a = lerp(ahi, alo, r).fmul(s).fmul(stf).fmul(stf).fmul_m(ts()).fmul_m(gf(0x8D_3444));
                        wr::<f32>(this, 0x994, f32_of(a.fadd_m(rd::<f32>(this, 0x994))));
                    }
                    xi(st).fmul_m(0.5)
                }
                16 => {
                    surface(this, &RUDDER, st, comp, pi, sf, &mut c_skid);
                    x(f32_of(xi(st).fmul_m(f32::from_bits(0x3E4C_CCCD))))
                }
                17 | 18 => {
                    surface(this, &ELEVATOR, st, comp, pi, sf, &mut c_ud);
                    x(f32_of(xi(st))).fmul_m(f32::from_bits(0x3E19_999A))
                }
                19 | 20 => {
                    surface(this, &AILERON, st, comp, pi, sf, &mut c_lr);
                    x(f32_of(xi(st))).fmul_m(0.25)
                }
                _ => continue,
            };
            if le_or_nan(smoke, x(0.0)) {
                continue;
            }
            smoke_puff(this, comp, f32_of(smoke), sf);
        }
    }
    let status = rd::<u8>(this, 0x36) >> 3;
    if status != 0 && status != 8 && status != 3 {
        let thr = f32_of(x(ts()).fmul_m(f32::from_bits(0x3A83_126F)));
        if le_or_nan(x(rd::<f32>(this, 0x9C4)), x(thr)) {
            wr::<u32>(this, 0x9C4, 0);
            wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xEF);
            return;
        }
        let dec = if le_or_nan(x(rd::<f32>(this, 0x9C4)), x(gf(0x8D_33D0))) {
            x(thr)
        } else {
            x(ts()).fmul_m(f32::from_bits(0x3B44_9BA6))
        };
        wr::<u8>(this, 0xB8, 0);
        wr::<f32>(this, 0x9C4, f32_of(x(rd::<f32>(this, 0x9C4)).fsub(dec)));
        return;
    }
    let std = gf(0x8D_33D0);
    let mut target = std;
    let accel = x(rd::<f32>(this, 0x994));
    if gt(accel, x(0.0)) {
        target = f32_of(x(gf(0x8D_33D4)).fsub_m(std).fmul_m(rd::<f32>(this, 0x994)).fadd_m(std));
    } else if lt(accel, x(0.0)) {
        target = f32_of(x(std).fsub_m(gf(0x8D_33CC)).fmul_m(rd::<f32>(this, 0x994)).fadd_m(std));
    }
    if status == 0 || status == 8 {
        let fm = if rd::<u16>(this, 0x22) != 0x1D0 { 3 } else { 1 };
        if gt(height_above_ceiling(this, pos_z(this), fm), x(0.0)) {
            let mut h = height_above_ceiling(this, pos_z(this), fm).fmul_m(f32::from_bits(0x3CA3_D70A));
            if !le_or_nan(x(0.0), h) {
                h = x(0.0);
            }
            target = f32_of(h.fmul_m(target));
            let mut a = height_above_ceiling(this, pos_z(this), fm).fmul_m(f32::from_bits(0x3D23_D70A)).fsubr_m(rd::<f32>(this, 0x994));
            if !le_or_nan(x(-1.0), a) {
                a = x(-1.0);
            }
            wr::<f32>(this, 0x994, f32_of(a));
        }
    }
    let prop = x(target).fsub_m(rd::<f32>(this, 0x9C4)).fmul_m(ts()).fmul_m(gf(0x8D_33D8)).fadd_m(rd::<f32>(this, 0x9C4));
    wr::<f32>(this, 0x9C4, f32_of(prop));
    let drowning = rd::<u8>(this, 0x42B) & 0x40 != 0;
    if drowning {
        wr::<u32>(this, 0x9C4, 0);
        wr::<u32>(this, 0x994, 0);
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xEF);
    }
    let model = rd::<u16>(this, 0x22);
    let engine_on = || rd::<u8>(this, 0x428) & 0x10 != 0;
    if model == 0x1D0 {
        if !drowning && engine_on() {
            flying_control(this, 1, c_skid, c_ud, c_lr, rd::<f32>(this, 0x994));
        }
        return;
    }
    if !engine_on() {
        return;
    }
    if le_or_nan(x(rd::<f32>(this, 0x9C4)), x(gf(0x8D_33CC))) && le_or_nan(speed_sq(this), X87::from_f64_exact(0.05)) {
        return;
    }
    if model == 0x208
        && rd::<u8>(this, 0x36) & 0xF8 == 0
        && i32::from(rd::<u16>(this, 0x86C)) >= i32::from(g::<i16>(0x8D_33C8).read())
    {
        let saved = rd::<u32>(this, 0x388);
        let hunter = g::<GPtr<u8>>(0xA9_B76C).read();
        let fh = get_flying_pointer(GPtr::from_addr(0xC2_B9C8), u32::from(rd::<u8>(hunter, 0x4A)));
        wr::<GPtr<u8>>(this, 0x388, fh);
        let up = gt(x(rd::<f32>(this, 0x994)), X87::from_f64_exact(0.0));
        if up || (rd::<u8>(this, 0x960) < 4 && rd::<u32>(this, 0x40) & 0x800_0000 == 0) {
            let a = f32_of(x(gf(0x8D_341C)).fmul_m(rd::<f32>(this, 0x994)));
            flying_control(this, 6, c_skid, c_ud, f32_of(x(c_lr).fchs()), a);
        }
        wr::<u32>(this, 0x388, saved);
        return;
    }
    flying_control(this, 3, c_skid, c_ud, c_lr, rd::<f32>(this, 0x994));
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PLANE_PROCESS_FLYING_CAR_STUFF];
