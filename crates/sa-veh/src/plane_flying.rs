//! FRONTIER-2 turn 6, lane B (batch `frontier2-b15`): the plane's per-frame flight and render bodies,
//! `CPlane::ProcessFlyingCarStuff` (0x6CB7C0) and `CPlane::PreRender` (0x6C94A0). Ported from the 1.0 US bytes
//! (gta-reversed only calls the original); docs/re/specs/VEH/CPlane.md was a reading aid (where it differs, the bytes
//! win: the Hydra's hover call passes `-leftRight`, not `-upDown`). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::field::v3_ptr as vp;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_core::crt::rand_at;
use sa_math::fpu::{ge_or_nan, gt, le_or_nan, lt};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, rd, wr, x, xi};
use crate::flying_control::{height_above_ceiling, pos_z};
use crate::mvpb_a1::flying_control;
use crate::plane_damage::{get_aeroplane_comp_status, set_aeroplane_comp_status};
use crate::automobile_render::process_panel;
use crate::bike_render::quaternion_set;
use crate::bindings::{
    get_flying_pointer, set_component_atomic_alpha, set_component_rotation, store_shadow_for_vehicle, update_wheel_matrix,
    vehicle_pre_render,
};
use crate::boat_pre_render::do_boat_splashes;
use crate::common::model_info;
use crate::heli_pre_render::{do_heli_dust_effect, v_slot_d0};
use crate::mvpd_v4::ci_asin as asin;
use sa_core::bindings::{
    fx_play, frame_for_all_objects, matrix_attach, matrix_dtor, matrix_mul, matrix_mul_vec, matrix_set_translate_only,
    matrix_update, matrix_update_rw, quat_get, vector_normalise,
};
use sa_core::field::ptr_of as buf;
use sa_stream::bindings::get_wheel_posn;
use sa_world::bindings::{
    entity_get_col_model, fx_add_particle, fx_copy_parent_matrix, fx_create_system as create_fx_system, fx_kill,
    fx_prt_mult_ctor, fx_set_const_time, register_streak, fx_set_local_particles, fx_set_offset_pos, fx_system_set_matrix,
};
use sa_world::world::mvpe_ent1::ci_pow as pow;
use sa_world::world::common::entity_pos;
use sa_world::world::break_object::rand_unit as rand01;
use sa_world::world::stats_travel::speed_sq;
use sa_core::field::gf;
use sa_pop::carctrl3::time_step as ts;

orig! {
    /// `CPlaceable::GetRoll()` (0x420B30; ST0)
    fn placeable_get_roll(p: GPtr<u8>) -> X87 = reg("args = ecx") 0x420B30 => sa_world::world::entity::get_roll;
    /// virtual (vtable +0x114) of `CAutomobile`: `DoHoverSuspensionRatios`
    fn v_slot_114(v: GPtr<u8>) = thiscall vslot 0x114 => unported;
    /// `CVehicleModelInfo::GetOriginalCompPosition(CVector& out, int32 component)` (0x4C7DD0, AL)
    fn get_original_comp_position(mi: GPtr<u8>, out: GPtr<u8>, comp: i32) -> u8 = thiscall 0x4C7DD0 => unported;
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

/// `CPlane::PreRender` constants of `.rdata` (degrees to radians, the control-surface ranges, the gear angles).
mod k {
    pub const DEG: f32 = f32::from_bits(0x3C8E_FA35);
    pub const RUDDER: f32 = 40.0;
    pub const AILERON: f32 = -30.0;
    pub const ELEVATOR: f32 = 25.0;
    pub const GEAR_A: f32 = f32::from_bits(0x3FBD_E44F);
    pub const HALF_PI_NEG: f32 = f32::from_bits(0xBFC9_0FDB);
    pub const GEAR_B: f32 = f32::from_bits(0xBFB2_B8C3);
    pub const GEAR_C: f32 = f32::from_bits(0x4011_361E);
    pub const GEAR_D: f32 = f32::from_bits(0x3FA7_8D36);
    pub const TWO_PI: f32 = f32::from_bits(0x40C9_0FDB);
    pub const HALF_PI: f32 = f32::from_bits(0x3FC9_0FDB);
}

/// The control-surface rotation through the shared `CMatrix` and `CQuaternion` of `CPlane::PreRender`.
struct Surf {
    mi: GPtr<u8>,
    mat: GPtr<u8>,
    q: GPtr<u8>,
    axis: [f32; 3],
}

impl Surf {
    /// `mat.Attach(&frame->modelling, false); mat.UpdateRW()`.
    fn attach(&self, node: GPtr<u8>) {
        matrix_attach(self.mat, node.byte_add(0x10), 0);
        matrix_update_rw(self.mat);
    }
    /// The rotation axis from vehicle-struct dummy `dummy`: `def` set to 1.0 when the dummy is all zero, else the
    /// dummy minus the attached frame's position, normalised.
    fn axis_from(&mut self, dummy: isize, def: usize) {
        let d = rd::<GPtr<u8>>(self.mi, 0x5C).byte_add(dummy * 0xC);
        self.axis = [rd::<f32>(d, 0), rd::<f32>(d, 4), rd::<f32>(d, 8)];
        let zero = |v: f32| !x(v).fcom(x(0.0)).test_parity(0x44);
        if zero(self.axis[0]) && zero(self.axis[1]) && zero(self.axis[2]) {
            self.axis[def] = 1.0;
            return;
        }
        for (i, o) in [(0usize, 0x30isize), (1, 0x34), (2, 0x38)] {
            self.axis[i] = f32_of(x(self.axis[i]).fsub_m(rd::<f32>(self.mat, o)));
        }
        vector_normalise(GPtr::from_raw(&mut self.axis as *mut [f32; 3]));
    }
    /// `q.Set(&axis, angle); q.Get(mat.m_pAttachMatrix)`.
    fn apply(&mut self, angle: f32) {
        quaternion_set(self.q, vp(&mut self.axis), angle);
        quat_get(self.q, rd::<GPtr<u8>>(self.mat, 0x40));
    }
    /// The aileron's stretch with the landing gear out (`s > 1`): `forward.y *= s` when the axis is the default one,
    /// else the frame moved by `(1 - s) * bbox.max.y * 0.03` along y from its original position.
    fn stretch(&self, s: f32, unit: f32, comp: i32, col: GPtr<u8>) {
        if le_or_nan(x(s), x(1.0)) {
            return;
        }
        matrix_update(self.mat);
        if !x(self.axis[0]).fcom(x(unit)).test_parity(0x44) {
            wr::<f32>(self.mat, 0x14, f32_of(x(rd::<f32>(self.mat, 0x14)).fmul_m(s)));
        } else {
            let mut out = [0f32; 3];
            if get_original_comp_position(self.mi, vp(&mut out), comp) != 0 {
                let y = x(1.0).fsub_m(s).fmul_m(rd::<f32>(col, 0x10)).fmul_m(f32::from_bits(0x3CF5_C28F)).fadd_m(out[1]);
                matrix_set_translate_only(self.mat, out[0], f32_of(y), out[2]);
            }
        }
        matrix_update_rw(self.mat);
    }
}

/// `(a.z * b.z + a.y * b.y) + a.x * b.x`, `a` the move speed, `b` a matrix row: the inlined dot products.
fn dot_speed(this: GPtr<u8>, row: GPtr<u8>) -> X87 {
    x(rd::<f32>(this, 0x4C))
        .fmul_m(rd::<f32>(row, 8))
        .fadd(x(rd::<f32>(this, 0x48)).fmul_m(rd::<f32>(row, 4)))
        .fadd(x(rd::<f32>(this, 0x44)).fmul_m(rd::<f32>(row, 0)))
}
/// `clamp(v, -lim, lim)` as the Vortex's tilt code tests it (upper bound first, NaN kept).
fn clamp_tilt(v: X87, lim: f32) -> f32 {
    let first = if gt(v, x(lim)) { x(lim) } else { v };
    let lo = f32_of(x(lim).fchs());
    if gt(x(lo), first) {
        lo
    } else if gt(v, x(lim)) {
        lim
    } else {
        f32_of(v)
    }
}
/// `a > hi -> hi; a < -hi -> -hi` with `-hi` rounded through the f32 slot.
fn clamp_x87(a: X87, hi: f32) -> X87 {
    if gt(a, x(hi)) {
        return x(hi);
    }
    let lo = f32_of(x(hi).fchs());
    if lt(a, x(lo)) { x(lo) } else { a }
}

/// `CPlane::PreRender()` (0x6C94A0, vtable slot 17). See docs/re/specs/VEH/CPlane.md §A-L: the vehicle pre-render, the
/// suspension (Andromada wheel tilt), wheel matrices, the propeller angle, the control surfaces (status 0..3), the
/// propeller frames, the landing gear by model, the model specials (Andromada cargo door, Hydra nozzles / jet thrust /
/// dust, Vortex skirt), the bouncing panels and the move / turn force copies, the shadow, the wing-tip streaks, the
/// smoke ejector (Cropduster, Stuntplane) and the Skimmer's splashes.
#[hook(addr = 0x6C94A0, conv = thiscall, name = "CPlane::PreRender", this = "CPlane", expect = "6A FF 68 AB 85 84 00",
       unverified = "frontier2-b15", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "_CIasin / _CIpow via orig::x87_*; the stack CMatrix / CQuaternion live on the Rust stack; the SEH frame is not reproduced; RegisterStreak's alpha dword carries stack garbage above its low byte in the original (the callee reads the byte)")]
pub fn plane_pre_render(this: GPtr<u8>) {
    let mut mat_b = [0u32; 0x12];
    let mat = buf(&mut mat_b);
    let mut q_b = [0u32; 4];
    let mi = model_info(i32::from(rd::<i16>(this, 0x22)));
    let col = entity_get_col_model(this);
    vehicle_pre_render(this);
    let model = || rd::<u16>(this, 0x22);
    if model() == 0x21B {
        v_slot_114(this);
    }
    // B: suspension
    if rd::<u8>(this, 0x42B) & 1 != 0 {
        v_slot_d0(this);
        for wheel in 0..4i32 {
            let o = 4 * wheel as isize;
            let r = x(rd::<f32>(this, 0x878 + o)).fdiv_m(rd::<f32>(this, 0x888 + o)).fsubr_m(1.0);
            let c = f32_of(x(rd::<f32>(this, 0x7D4 + o)).fsub(r).fdiv(x(1.0).fsub(r)));
            let mut pos = [0f32; 3];
            get_wheel_posn(mi, wheel, vp(&mut pos), 1);
            let mut z = x(pos[2]).fadd_m(rd::<f32>(rd::<GPtr<u8>>(this, 0x384), 0xB8));
            if gt(x(c), x(0.0)) {
                z = z.fsub(x(c).fmul_m(rd::<f32>(this, 0x878 + o)));
            }
            let cur = rd::<f32>(this, 0x838 + o);
            if le_or_nan(z, x(cur)) && !(rd::<u32>(this, 0x40) & 0x400_0000 != 0 && rd::<u32>(this, 0x38C) & 0x2_0000 != 0) {
                z = z.fsub_m(cur).fmul_m(0.75).fadd_m(cur);
            }
            wr::<f32>(this, 0x838 + o, f32_of(z));
            if model() == 0x250 {
                let rot = if lt(x(rd::<f32>(this, 0x7D4 + o)), x(1.0)) {
                    let m = rd::<GPtr<u8>>(this, 0x14);
                    let n = this.byte_add(0x734 + wheel as isize * 0x2C);
                    let d = x(rd::<f32>(m, 0x18))
                        .fmul_m(rd::<f32>(n, 8))
                        .fadd(x(rd::<f32>(n, 0)).fmul_m(rd::<f32>(m, 0x10)))
                        .fadd(x(rd::<f32>(m, 0x14)).fmul_m(rd::<f32>(n, 4)));
                    let dd = if gt(x(-1.0), d) {
                        -1.0
                    } else if !ge_or_nan(x(1.0), d) {
                        1.0
                    } else {
                        f32_of(d)
                    };
                    asin(x(dd)).fchs()
                } else {
                    x(rd::<f32>(this, 0x828 + o)).fmul_m(f32::from_bits(0x3F73_3333))
                };
                wr::<f32>(this, 0x828 + o, f32_of(rot));
            }
        }
    }
    // C: wheel matrices
    update_wheel_matrix(this, 4, 1);
    update_wheel_matrix(this, 7, 1);
    update_wheel_matrix(this, 2, 1);
    update_wheel_matrix(this, 5, if model() == 0x208 { 3 } else { 1 });
    // D: propeller angle
    let a = x(ts()).fmul_m(rd::<f32>(this, 0x9C4)).fadd_m(rd::<f32>(this, 0x9C8));
    wr::<f32>(this, 0x9C8, f32_of(a));
    if gt(a, x(k::TWO_PI)) {
        let mut v = x(rd::<f32>(this, 0x9C8));
        loop {
            v = v.fsub_m(k::TWO_PI);
            if !gt(v, x(k::TWO_PI)) {
                break;
            }
        }
        wr::<f32>(this, 0x9C8, f32_of(v));
    }
    let node = |i: isize| rd::<GPtr<u8>>(this, 0x648 + i * 4);
    let mut s = Surf { mi, mat, q: buf(&mut q_b), axis: [0.0; 3] };
    // E: control surfaces
    if rd::<u8>(this, 0x36) & 0xF8 <= 0x18 {
        if !node(16).is_null() {
            s.attach(node(16));
            s.axis_from(11, 2);
            s.apply(f32_of(x(k::RUDDER).fmul_m(k::DEG).fmul_m(rd::<f32>(this, 0x988))));
            if model() == 0x21B && !node(24).is_null() {
                s.attach(node(24));
                quat_get(s.q, rd::<GPtr<u8>>(mat, 0x40));
            }
        }
        if !node(20).is_null() {
            let mut sc = 1.0f32;
            let mut off = 0.0f32;
            let gear = || x(rd::<f32>(this, 0x9CC)).fabs();
            if gt(x(rd::<f32>(rd::<GPtr<u8>>(this, 0x388), 0x34)), x(1.0))
                && lt(gear(), x(1.0))
                && (!node(21).is_null() || !node(22).is_null())
            {
                sc = f32_of(gear().fsubr_m(1.0).fmul_m(0.5).fadd_m(1.0));
                off = f32_of(gear().fsubr_m(1.0).fmul_m(f32::from_bits(0x3E99_999A)));
            }
            s.attach(node(20));
            s.axis_from(9, 0);
            s.apply(f32_of(x(k::AILERON).fmul_m(k::DEG).fmul_m(rd::<f32>(this, 0x990)).fadd_m(off)));
            s.stretch(sc, 1.0, 20, col);
            if !node(19).is_null() {
                s.attach(node(19));
                s.axis[0] = f32_of(x(s.axis[0]).fmul_m(-1.0));
                s.apply(f32_of(x(k::AILERON).fmul_m(k::DEG).fmul_m(rd::<f32>(this, 0x990)).fsub_m(off)));
                s.stretch(sc, -1.0, 19, col);
            }
        }
        if !node(18).is_null() {
            s.attach(node(18));
            s.axis_from(10, 0);
            s.apply(f32_of(x(k::ELEVATOR).fmul_m(k::DEG).fmul_m(rd::<f32>(this, 0x98C)).fchs()));
            if !node(17).is_null() {
                s.attach(node(17));
                let ang = f32_of(x(k::ELEVATOR).fmul_m(k::DEG).fmul_m(rd::<f32>(this, 0x98C)));
                s.axis[0] = f32_of(x(s.axis[0]).fmul_m(-1.0));
                s.apply(ang);
            }
        }
    }
    // F: propellers
    for (st, mv, sign) in [(12isize, 13isize, 1.0f32), (14, 15, -1.0)] {
        for (n, alpha) in [(st, 0xFFu32), (mv, 0)] {
            let f = node(n);
            if f.is_null() {
                continue;
            }
            let t = x(sign).fmul_m(rd::<f32>(this, 0x9C8));
            let angle = if alpha != 0 { t.fadd(t) } else { t.fchs() };
            set_component_rotation(this, f, 1, f32_of(angle), 1);
            let mut atomic = [0u32; 1];
            frame_for_all_objects(node(n), 0x6A_0750, buf(&mut atomic).addr() as u32);
            if atomic[0] != 0 {
                set_component_atomic_alpha(GPtr::from_addr(atomic[0] as usize), alpha);
            }
        }
    }
    // G: landing gear (axis, misc axis or -1, k gear L / R, k misc A / B)
    let gear_case = match model() {
        476 => Some((1u32, -1i32, -k::GEAR_A, k::GEAR_A, 0.0f32, 0.0f32)),
        519 | 577 => Some((1, 0, -k::GEAR_A, k::GEAR_A, k::GEAR_C, 0.0)),
        520 => Some((0, 0, k::HALF_PI_NEG, k::HALF_PI_NEG, k::GEAR_B, k::GEAR_C)),
        553 => Some((0, -1, k::GEAR_D, k::GEAR_D, 0.0, 0.0)),
        592 => Some((0, 0, k::GEAR_C, k::GEAR_C, -k::GEAR_C, 0.0)),
        _ => None,
    };
    if let Some((axis, misc_axis, kl, kr, ka, kb)) = gear_case {
        let g = || x(rd::<f32>(this, 0x9CC)).fabs();
        set_component_rotation(this, node(21), axis, f32_of(g().fmul(x(kl))), 1);
        set_component_rotation(this, node(22), axis, f32_of(g().fmul_m(kr)), 1);
        if misc_axis > -1 {
            set_component_rotation(this, node(23), misc_axis as u32, f32_of(g().fmul_m(ka)), 1);
            if gt(x(kb), X87::from_f64_exact(0.0)) {
                set_component_rotation(this, node(24), misc_axis as u32, f32_of(g().fmul_m(kb)), 1);
            }
        }
    }
    // H: model specials
    let m = model();
    if m == 0x250 {
        let a = xi(i32::from(rd::<u16>(this, 0x86C))).fmul_m(gf(0x8D_33C0));
        set_component_rotation(this, node(24), 0, f32_of(a), 1);
    } else if m == 0x208 {
        hydra(this);
    } else if m == 0x21B && !node(23).is_null() {
        vortex(this, mi, mat, node(23));
    }
    // I: panels, forces, shadow
    for p in 0..3isize {
        let panel = this.byte_add(0x6AC + p * 0x20);
        let id = rd::<i16>(panel, 0);
        if id <= -1 {
            continue;
        }
        let mf = [rd::<u32>(this, 0x8B4), rd::<u32>(this, 0x8B8), rd::<u32>(this, 0x8BC)];
        let tf = [rd::<u32>(this, 0x8C0), rd::<u32>(this, 0x8C4), rd::<u32>(this, 0x8C8)];
        process_panel(panel, this, node(isize::from(id)), mf[0], mf[1], mf[2], tf[0], tf[1], tf[2], g::<u32>(0x8D_33FC).read(), g::<u32>(0x8D_3400).read());
    }
    for (dst, a, b) in [(0x8B4isize, 0x44isize, 0x5Cisize), (0x8C0, 0x50, 0x68)] {
        let v = [0isize, 4, 8].map(|i| f32_of(x(rd::<f32>(this, a + i)).fadd_m(rd::<f32>(this, b + i))));
        for (i, val) in v.into_iter().enumerate() {
            wr::<f32>(this, dst + i as isize * 4, val);
        }
    }
    let m = model();
    store_shadow_for_vehicle(this, if m == 0x21B { 0 } else if m == 0x241 || m == 0x250 { 5 } else { 3 });
    // J: wing-tip streaks
    if rd::<u8>(this, 0x42B) & 0x40 == 0 && m != 0x21B && m != 0x1D0 {
        let up = rd::<GPtr<u8>>(this, 0x14).byte_add(0x20);
        let mut v = dot_speed(this, up).fmul_m(rd::<f32>(rd::<GPtr<u8>>(this, 0x388), 0x2C)).fmul_m(6400.0).fabs().fsub_m(32.0);
        if gt(x(0.0), v) {
            v = x(0.0);
        }
        let alpha = ftol32(v) as u8;
        let d = rd::<GPtr<u8>>(mi, 0x5C).byte_add(0x90);
        let mut b = [rd::<f32>(d, 0), rd::<f32>(d, 4), rd::<f32>(d, 8)];
        let mut a = [f32_of(x(b[0]).fsub_m(f32::from_bits(0x3DCC_CCCD))), b[1], b[2]];
        for side in 0..2u32 {
            if side == 1 {
                b[0] = f32_of(x(b[0]).fchs());
                a[0] = f32_of(x(a[0]).fchs());
            }
            let mut o1 = [0f32; 3];
            let r1 = matrix_mul_vec(vp(&mut o1), rd::<GPtr<u8>>(this, 0x14), vp(&mut b));
            let l = [rd::<u32>(r1, 0), rd::<u32>(r1, 4), rd::<u32>(r1, 8)];
            let mut o2 = [0f32; 3];
            let r2 = matrix_mul_vec(vp(&mut o2), rd::<GPtr<u8>>(this, 0x14), vp(&mut a));
            let r = [rd::<u32>(r2, 0), rd::<u32>(r2, 4), rd::<u32>(r2, 8)];
            let id = (this.addr() as u32).wrapping_add(side);
            register_streak(id, 0xFF, 0xFF, 0xFF, u32::from(alpha), l[0], l[1], l[2], r[0], r[1], r[2]);
        }
    }
    // K: smoke ejector
    if rd::<u8>(this, 0xA00) != 0 && (m == 0x200 || m == 0x201) {
        let mut mult = [0u32; 7];
        let mp = buf(&mut mult);
        let crop = m == 0x200;
        let mut off = if crop {
            fx_prt_mult_ctor(mp, 1.0, 1.0, 1.0, f32::from_bits(0x3ECC_CCCD), 1.0, 1.0, f32::from_bits(0x3E4C_CCCD));
            [0.0f32, -0.5, -0.5]
        } else {
            fx_prt_mult_ctor(mp, 1.0, 0.0, 0.0, f32::from_bits(0x3ECC_CCCD), 1.0, 1.0, f32::from_bits(0x3E99_999A));
            [0.0f32, -5.0, 0.0]
        };
        let mut out = [0f32; 3];
        let r = matrix_mul_vec(vp(&mut out), rd::<GPtr<u8>>(this, 0x14), vp(&mut off));
        let mut pos = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
        let ten = |o: isize| x(rd::<f32>(this, o)).fmul_m(10.0);
        let z = if crop { ten(0x4C).fsub_m(3.0) } else { ten(0x4C) };
        let mut vel = [f32_of(ten(0x44)), f32_of(ten(0x48)), f32_of(z)];
        let fx = g::<GPtr<u8>>(0xA9_AE20).read();
        fx_add_particle(fx, vp(&mut pos), vp(&mut vel), 0, mp, 0xBF80_0000, f32::from_bits(0x3F99_999A), f32::from_bits(0x3F19_999A), 0);
    }
    // L: skimmer splashes
    if model() == 0x1CC && rd::<u32>(this, 0x40) & 0x100 != 0 {
        do_boat_splashes(this, rd::<f32>(this, 0x950));
    }
    matrix_dtor(mat);
}

/// `CPlane::PreRender`'s Hydra block (0x6C9F26-0x6CA29B): the nozzle angle on both nozzle frames; engine off: the four
/// jet-thrust systems and the dust killed; engine on: dust above the hover switch angle (else killed), the jet-thrust
/// systems created on the vehicle's frame and placed on the nozzles with their offsets and the throttle's time.
fn hydra(this: GPtr<u8>) {
    let ang = xi(i32::from(rd::<u16>(this, 0x86C))).fmul_m(k::HALF_PI).fdiv(xi(i32::from(g::<i16>(0x8D_33C4).read())));
    let a = f32_of(ang);
    set_component_rotation(this, rd::<GPtr<u8>>(this, 0x660), 0, a, 1);
    set_component_rotation(this, rd::<GPtr<u8>>(this, 0x654), 0, a, 1);
    let kill_dust = || {
        let d = rd::<GPtr<u8>>(this, 0x580);
        if !d.is_null() {
            fx_kill(d);
            wr::<u32>(this, 0x580, 0);
            wr::<u32>(this, 0x984, 0);
        }
    };
    if rd::<u8>(this, 0x428) & 0x10 == 0 {
        for j in 0..4isize {
            let f = rd::<GPtr<u8>>(this, 0x9E8 + j * 4);
            if !f.is_null() {
                fx_kill(f);
                wr::<u32>(this, 0x9E8 + j * 4, 0);
            }
        }
        kill_dust();
        return;
    }
    let t = f32_of(x(rd::<f32>(this, 0x994)).fadd_m(1.0).fmul_m(0.5));
    if i32::from(rd::<u16>(this, 0x86C)) >= i32::from(g::<i16>(0x8D_33C8).read()) {
        do_heli_dust_effect(this, t, 2.0);
    } else {
        kill_dust();
    }
    let f6 = rd::<GPtr<u8>>(this, 0x660).byte_add(0x10);
    let f3 = rd::<GPtr<u8>>(this, 0x654).byte_add(0x10);
    let mut v: [f32; 3];
    for j in 0..4isize {
        let rw = rd::<GPtr<u8>>(this, 0x18);
        if rw.is_null() {
            continue;
        }
        let parent = rd::<u32>(rw, 4).wrapping_add(0x10);
        if parent == 0 || rd::<u32>(this, 0x9E8 + j * 4) != 0 {
            continue;
        }
        v = [0.0; 3];
        let f = create_fx_system(GPtr::from_addr(0xA9_AE80), GPtr::from_addr(0x87_1A64), vp(&mut v), GPtr::from_addr(parent as usize), 0);
        wr::<GPtr<u8>>(this, 0x9E8 + j * 4, f);
        if !f.is_null() {
            fx_play(f);
            fx_set_local_particles(rd::<GPtr<u8>>(this, 0x9E8 + j * 4), 1);
            fx_copy_parent_matrix(rd::<GPtr<u8>>(this, 0x9E8 + j * 4));
        }
    }
    let offs: [[u32; 3]; 4] = [
        [0x3F33_3333, 0xBEE6_6666, 0x3D4C_CCCD],
        [0xBF51_EB85, 0xBEE6_6666, 0x3D4C_CCCD],
        [0x3F21_47AE, 0xBEE6_6666, 0x3D8F_5C29],
        [0xBF40_0000, 0xBEE6_6666, 0x3D8F_5C29],
    ];
    for (j, o) in offs.into_iter().enumerate() {
        let slot = 0x9E8 + j as isize * 4;
        if rd::<u32>(this, slot) == 0 {
            continue;
        }
        let frame = if j < 2 { f6 } else { f3 };
        v = o.map(f32::from_bits);
        let mut tmp_b = [0u32; 0x12];
        let tmp = buf(&mut tmp_b);
        matrix_mul(tmp, rd::<GPtr<u8>>(this, 0x14), frame);
        fx_system_set_matrix(rd::<GPtr<u8>>(this, slot), tmp);
        fx_set_offset_pos(rd::<GPtr<u8>>(this, slot), vp(&mut v));
        fx_set_const_time(rd::<GPtr<u8>>(this, slot), 1, t);
        matrix_dtor(tmp);
    }
}

/// `CPlane::PreRender`'s Vortex block (0x6CA2F3-0x6CA633): the skirt frame (node 23) leans with the speed along the
/// forward / right vectors (`up.y` / `up.x`, +-0.3) and eases (`pow(0.9, ts)`) its pitch / roll / height toward the
/// suspension's.
fn vortex(this: GPtr<u8>, mi: GPtr<u8>, mat: GPtr<u8>, n23: GPtr<u8>) {
    let cd = rd::<GPtr<u8>>(entity_get_col_model(this), 0x2C);
    let mut out = [0f32; 3];
    get_wheel_posn(mi, 0, vp(&mut out), 0);
    matrix_attach(mat, n23.byte_add(0x10), 0);
    let mm = rd::<GPtr<u8>>(this, 0x14);
    let lim = gf(0x8D_3414);
    let two = gf(0x8D_3418);
    wr::<f32>(mat, 0x24, clamp_tilt(dot_speed(this, mm.byte_add(0x10)).fmul_m(two), lim));
    wr::<f32>(mat, 0x20, clamp_tilt(dot_speed(this, mm).fmul_m(two), lim));
    let kk = pow(x(gf(0x8D_3410)), ts());
    let one_k = f32_of(x(1.0).fsub(kk));
    let ease = |o: isize, v: X87| wr::<f32>(mat, o, f32_of(x(rd::<f32>(mat, o)).fmul(kk).fadd(x(one_k).fmul(v))));
    let w = |i: isize| rd::<f32>(this, 0x838 + i * 4);
    let lines = rd::<GPtr<u8>>(cd, 0x10);
    let a = x(w(2)).fsub_m(w(3)).fadd(x(w(0)).fsub_m(w(1))).fmul_m(0.5).fdiv(x(rd::<f32>(lines, 4)).fsub_m(rd::<f32>(lines, 0x24)));
    ease(0x18, clamp_x87(a, gf(0x8D_340C)));
    let b = x(w(3)).fsub_m(w(1)).fadd(x(w(2)).fsub_m(w(0))).fmul_m(0.5).fdiv(x(rd::<f32>(lines, 0x60)).fsub_m(rd::<f32>(lines, 0x20)));
    ease(0x08, clamp_x87(b, gf(0x8D_3408)));
    let mut c = x(w(3))
        .fadd_m(w(1))
        .fadd_m(w(2))
        .fadd_m(w(0))
        .fmul_m(0.25)
        .fsub_m(out[2])
        .fdiv(x(rd::<f32>(mi, 0x40)).fmul_m(0.5))
        .fsubr_m(1.0);
    let hi = gf(0x8D_3404);
    if gt(c, x(hi)) {
        c = x(hi);
    }
    ease(0x28, c);
    matrix_update_rw(mat);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PLANE_PROCESS_FLYING_CAR_STUFF, HOOK_PLANE_PRE_RENDER];
