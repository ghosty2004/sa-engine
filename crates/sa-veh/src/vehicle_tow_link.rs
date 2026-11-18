//! FRONTIER-2 turn 10, lane B (batch `frontier2-b22`): the tow link forces, `CVehicle::UpdateTrailerLink`
//! (0x6DFC50) and `CVehicle::UpdateTractorLink` (0x6E0050). Ported from the 1.0 US bytes; gta-reversed's shared
//! `UpdateVehicleLinkImpl` is a reading aid only (it multiplies by `max(1, timeStep)` where the bytes divide, and
//! its trailer path breaks the link in cases the bytes merely return from).

use sa_core::abi::HookReg;
use sa_core::bindings::{cross_product, vector_dot_product, vector_normalise};
use sa_core::field::{gf, gr16u, ptr_of as vp};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::{le_or_nan, lt};
use sa_math::vector::CVector;
use sa_math::x87::X87;
use sa_world::world::common::entity_pos;

use crate::common::{f32_of, rd, wr, x};
use crate::mvpd_v1::read_vec;

use sa_world::bindings::apply_force;
use sa_world::bindings::max_f;
use sa_world::bindings::multiply3x3;
use sa_world::bindings::physical_get_speed as get_speed;
use sa_world::bindings::vector_div;
use sa_world::bindings::vector_magnitude_sqr;
use sa_world::bindings::vector_mul_scalar;
use sa_world::bindings::vector_scale;
use sa_world::bindings::vector_sub_assign;
use sa_math::fpu::{ftol32, ge_or_nan, xi};
use sa_pop::bindings::{fabsf, get_car_gun_up_down};
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::{find_objects_in_range, get_pad_from_player};

orig! {
    /// `CVehicle::GetTowHitchPos(CVector&, bool, CVehicle*)` (vtable +0xEC)
    fn v_get_tow_hitch_pos(v: GPtr<u8>, out: GPtr<u8>, b: u32, other: GPtr<u8>) -> u8 = thiscall vslot 0xEC => unported;
    /// `CVehicle::GetTowBarPos(CVector&, bool, CVehicle*)` (vtable +0xF0)
    fn v_get_tow_bar_pos(v: GPtr<u8>, out: GPtr<u8>, b: u32, other: GPtr<u8>) -> u8 = thiscall vslot 0xF0 => unported;
    /// `CVehicle::SetTowLink(CVehicle*, bool)` (vtable +0xF4)
    fn v_set_tow_link(v: GPtr<u8>, other: GPtr<u8>, b: u32) -> u8 = thiscall vslot 0xF4 => unported;
    /// `CVehicle::BreakTowLink()` (vtable +0xF8)
    fn v_break_tow_link(v: GPtr<u8>) -> u8 = thiscall vslot 0xF8 => unported;
}

/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7_CB5C;
/// `TOWTRUCK_HOIST_DOWN_LIMIT` (u16)
const HOIST_DOWN_LIMIT: u32 = 0x8D_313C;
/// `TOWTRUCK_HOIST_UP_LIMIT` (u16)
const HOIST_UP_LIMIT: u32 = 0x8D_3140;
/// 10.0 (0x85862C), 0.0 (0x858B50), 2.0 (0x871090), 6.0 (0x87108C), 0.5 (0x871094), 1.0 (0x871098).
const K_DEAD_ZONE: usize = 0x85_862C;
const K_ZERO: usize = 0x85_8B50;
const K_DOWN_SPEED: usize = 0x87_1090;
const K_UP_SPEED: usize = 0x87_108C;
const K_HITCH_2D: usize = 0x87_1094;
const K_HITCH_Z: usize = 0x87_1098;

/// `a - b` per component, each an f32 store.
fn sub3(a: CVector, b: CVector) -> CVector {
    CVector::new(f32_of(x(a.x).fsub_m(b.x)), f32_of(x(a.y).fsub_m(b.y)), f32_of(x(a.z).fsub_m(b.z)))
}

/// `GetSpeed(point)` of `e` into a temporary, copied out.
fn speed_at(e: GPtr<u8>, point: CVector) -> CVector {
    let mut out = CVector::default();
    read_vec(get_speed(e, GPtr::from_raw(&mut out), point).cast())
}

/// A tow truck / tractor (models 525 / 531) whose hoist (+0x86C) is above `TOWTRUCK_HOIST_DOWN_LIMIT - 100`.
fn hoist_up(v: GPtr<u8>) -> bool {
    let m = rd::<u16>(v, 0x22);
    (m == 0x20D || m == 0x213) && i32::from(rd::<u16>(v, 0x86C)) > i32::from(gr16u(HOIST_DOWN_LIMIT)) - 0x64
}

/// `!applyFull && applyDist`: `sd = (k * d) / max(1.0, timeStep)` through the temporaries the bytes use (`d` is
/// overwritten with the result).
fn dist_speed(d: &mut CVector, k: f32) -> CVector {
    let m = f32_of(max_f(1.0, gf(TIME_STEP)));
    let mut tmp = CVector::default();
    let t = vector_scale(vp(&mut tmp), k, vp(d));
    read_vec(vector_div(vp(d), t, m))
}

/// A baggage trailer `v` (type +0x594 11 with +0x9EC = -1000.0): `sd -= (sd . up) * up` with the dot product
/// summed z, y, x.
fn drop_up_component(v: GPtr<u8>, sd: &mut CVector) {
    let up = rd::<GPtr<u8>>(v, 0x14).byte_add(0x20);
    let dot = f32_of(
        x(sd.z).fmul_m(rd::<f32>(up, 8)).fadd(x(sd.y).fmul_m(rd::<f32>(up, 4))).fadd(x(sd.x).fmul_m(rd::<f32>(up, 0))),
    );
    let mut tmp = CVector::default();
    let t = vector_scale(vp(&mut tmp), dot, up);
    vector_sub_assign(vp(sd), t);
}

/// The point relative to the centre of mass, `point - matrix * centreOfMass (+0xA4)`, crossed with the normalised
/// speed difference: `(r x n)` into a temporary.
fn arm_cross(this: GPtr<u8>, point: CVector, sd: CVector, cross: &mut CVector) -> GPtr<u8> {
    let mut com = CVector::default();
    let c = read_vec(multiply3x3(vp(&mut com), rd::<GPtr<u8>>(this, 0x14), this.byte_add(0xA4)));
    let mut n = sd;
    vector_normalise(vp(&mut n).cast());
    let mut r = sub3(point, c);
    cross_product(vp(cross), vp(&mut r), vp(&mut n))
}

/// `1 / (1 / mass + q)` with `q = |r x n|^2 / turnMass`, in x87 order.
fn link_mass(this: GPtr<u8>, cross_sq: X87) -> X87 {
    let q = cross_sq.fdiv_m(rd::<f32>(this, 0x90));
    let inv = x(1.0).fdiv_m(rd::<f32>(this, 0x8C));
    q.fadd(inv).fdivr_m(1.0)
}

/// `CVehicle::UpdateTrailerLink(bool applyFullVelocityAtHookUp, bool applyDistToSpeed)` (0x6DFC50, thiscall,
/// `ret 8`; the entry fragment 0x406886 loads the towing vehicle +0x4C4 into ECX).
///
/// The link breaks (`BreakTowLink`, vtable +0xF8) without a towing vehicle, outside status 10 / 11 (towed, simple
/// towed), when either end has no position (`GetTowHitchPos(.., true, towing)`, the towing vehicle's
/// `GetTowBarPos(.., true, this)`), when the link `d = bar - hitch` is longer than `max(timeStep, 0.7)` (the
/// squared length summed z (unrounded times rounded), y, x and stored), when the forward vectors' dot product is
/// below -0.3 or the up vectors' below 0. A tow truck / tractor with its hoist up returns without a force. Else,
/// with both ends relative to their vehicles, `sd = towing.GetSpeed(bar) - GetSpeed(hitch)`, replaced with
/// `(0.3 * d) / max(1, timeStep)` for `!applyFull && applyDist`, its up component dropped for a baggage trailer,
/// and `ApplyForce(sd * m, hitch, true)` with `m = 1 / (1 / mass + |r x n|^2 / turnMass)` stored as an f32.
#[hook(addr = 0x6DFC50, conv = thiscall, name = "CVehicle::UpdateTrailerLink", this = "CVehicle",
       expect = "83 EC 70 53 56 8B F1 90 E9", unverified = "frontier2-b22", route = "L2", hazards = "none",
       note = "entry chunk 0x406886 (`mov ecx, [esi+0x4C4]`)")]
#[allow(clippy::too_many_lines)]
pub fn update_trailer_link(this: GPtr<u8>, apply_full: u32, apply_dist: u32) {
    let towing = rd::<GPtr<u8>>(this, 0x4C4);
    let mut bar = CVector::default();
    let mut hitch = CVector::default();
    let st = rd::<u8>(this, 0x36) >> 3;
    if towing.is_null()
        || (st != 10 && st != 11)
        || v_get_tow_hitch_pos(this, vp(&mut hitch), 1, towing) == 0
        || v_get_tow_bar_pos(rd::<GPtr<u8>>(this, 0x4C4), vp(&mut bar), 1, this) == 0
    {
        v_break_tow_link(this);
        return;
    }
    let mut d = CVector::new(f32_of(x(bar.x).fsub_m(hitch.x)), f32_of(x(bar.y).fsub_m(hitch.y)), 0.0);
    let dz = x(bar.z).fsub_m(hitch.z);
    d.z = f32_of(dz);
    let sq = f32_of(dz.fmul_m(d.z).fadd(x(d.y).fmul_m(d.y)).fadd(x(d.x).fmul_m(d.x)));
    let ts = gf(TIME_STEP);
    let seven = gf(0x85_8CB0);
    let t = if le_or_nan(x(ts), x(seven)) { x(seven) } else { x(ts) };
    // fld 1.0; fmul st(2); fcompp against the root: `test ah, 5; jnp` breaks when 1.0 * t < |d|
    if lt(x(gf(0x87_1DF0)).fmul(t), x(sq).fsqrt()) {
        v_break_tow_link(this);
        return;
    }
    let tw = rd::<GPtr<u8>>(this, 0x4C4);
    let tm = rd::<GPtr<u8>>(tw, 0x14);
    let m = rd::<GPtr<u8>>(this, 0x14);
    let fwd = x(rd::<f32>(tm, 0x18))
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(x(rd::<f32>(tm, 0x14)).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(x(rd::<f32>(tm, 0x10)).fmul_m(rd::<f32>(m, 0x10)));
    if lt(fwd, x(gf(0x85_8EE4))) || lt(vector_dot_product(tm.byte_add(0x20), m.byte_add(0x20)), x(0.0)) {
        v_break_tow_link(this);
        return;
    }
    if hoist_up(tw) {
        return;
    }
    vector_sub_assign(vp(&mut hitch), entity_pos(this));
    vector_sub_assign(vp(&mut bar), entity_pos(tw));
    let s1 = speed_at(this, hitch);
    let s2 = speed_at(rd::<GPtr<u8>>(this, 0x4C4), bar);
    let mut sd = sub3(s2, s1);
    if apply_full as u8 == 0 && apply_dist as u8 != 0 {
        sd = dist_speed(&mut d, gf(0x87_1DF4));
    }
    if rd::<u32>(this, 0x594) == 0xB && crate::vehicle2::field_9ec_is_minus_1000(this) != 0 {
        drop_up_component(this, &mut sd);
    }
    let mut cross = CVector::default();
    let c = arm_cross(this, hitch, sd, &mut cross);
    let k = f32_of(link_mass(this, vector_magnitude_sqr(c)));
    let mut out = CVector::default();
    let f = read_vec(vector_mul_scalar(vp(&mut out), vp(&mut sd), k));
    apply_force(this, f, hitch, 1);
}

/// `CVehicle::UpdateTractorLink(bool applyFullVelocityAtHookUp, bool applyDistToSpeed)` (0x6E0050, thiscall,
/// `ret 8`).
///
/// Nothing without a towed vehicle (+0x4C8) or when either end has no position (the towed vehicle's
/// `GetTowHitchPos(.., true, this)`, `GetTowBarPos(.., true, towed)`), or for a tow truck / tractor with its hoist
/// up (no link break here). Else, with `d = hitch - bar` and both ends relative to their vehicles, `sd =
/// towed.GetSpeed(hitch) - GetSpeed(bar)`; unless `applyFull` it is scaled by `(1 - mass / (towed mass + mass)) *
/// 0.5` and, with `applyDist`, replaced with `(0.1 * d) / max(1, timeStep)`; a baggage trailer's up component is
/// dropped; `ApplyForce(sd * m, bar, true)` with `m = 1 / (1 / mass + |r x n|^2 / turnMass)` kept in x87 (the
/// squared magnitude summed x, y, z inline), and `m_nFakePhysics` (+0xB8) = 0.
#[hook(addr = 0x6E0050, conv = thiscall, name = "CVehicle::UpdateTractorLink", this = "CVehicle",
       expect = "83 EC 6C 56 8B F1", unverified = "frontier2-b22", route = "L2", hazards = "none")]
pub fn update_tractor_link(this: GPtr<u8>, apply_full: u32, apply_dist: u32) {
    let towed = rd::<GPtr<u8>>(this, 0x4C8);
    if towed.is_null() {
        return;
    }
    let mut bar = CVector::default();
    let mut hitch = CVector::default();
    if v_get_tow_hitch_pos(towed, vp(&mut hitch), 1, this) == 0
        || v_get_tow_bar_pos(this, vp(&mut bar), 1, rd::<GPtr<u8>>(this, 0x4C8)) == 0
    {
        return;
    }
    let mut d = sub3(hitch, bar);
    if hoist_up(this) {
        return;
    }
    let tw = rd::<GPtr<u8>>(this, 0x4C8);
    hitch = sub3(hitch, read_vec(entity_pos(tw)));
    bar = sub3(bar, read_vec(entity_pos(this)));
    let s1 = speed_at(tw, hitch);
    let s2 = speed_at(this, bar);
    let mut sd = sub3(s1, s2);
    if apply_full as u8 == 0 {
        let k = x(rd::<f32>(rd::<GPtr<u8>>(this, 0x4C8), 0x8C))
            .fadd_m(rd::<f32>(this, 0x8C))
            .fdivr_m(rd::<f32>(this, 0x8C))
            .fsubr_m(1.0)
            .fmul_m(gf(0x87_1DF8));
        sd = CVector::new(f32_of(x(sd.x).fmul(k)), f32_of(x(sd.y).fmul(k)), f32_of(x(sd.z).fmul(k)));
        if apply_dist as u8 != 0 {
            sd = dist_speed(&mut d, gf(0x87_1DFC));
        }
    }
    let tw = rd::<GPtr<u8>>(this, 0x4C8);
    if rd::<u32>(tw, 0x594) == 0xB && rd::<u32>(tw, 0x9EC) == 0xC47A_0000 {
        drop_up_component(tw, &mut sd);
    }
    let mut cross = CVector::default();
    let c = read_vec(arm_cross(this, bar, sd, &mut cross));
    let (cx, cy, cz) = (x(c.x), x(c.y), x(c.z));
    let k = link_mass(this, cx.fmul(cx).fadd(cy.fmul(cy)).fadd(cz.fmul(cz)));
    let f = CVector::new(f32_of(x(sd.x).fmul(k)), f32_of(x(sd.y).fmul(k)), f32_of(x(sd.z).fmul(k)));
    apply_force(this, f, bar, 1);
    wr::<u8>(this, 0xB8, 0);
}

/// This module's registrations.
/// `CAutomobile::TowTruckControl()` (0x6A40F0, thiscall, plain `ret`).
///
/// Only for a player-status car (+0x36 & 0xF8 == 0) with a player driver and his pad. The pad's car gun up / down
/// (`GetCarGunUpDown`, read again for every use) beyond 10 moves the hoist angle (+0x86C, u16) by
/// `(i16)_ftol2(v * speed * timeStep)` (speed 2.0 up for v > 0, 6.0 down): up it becomes `max(towing ?
/// TOWTRUCK_HOIST_UP_LIMIT : 0, angle - step)` (signed ints), down, while below `TOWTRUCK_HOIST_DOWN_LIMIT`, `min(limit,
/// angle - step)`. Fully down with nothing towed, the tow bar position (vtable +0xF0, false) looks for a vehicle among
/// the 16 nearest (`FindObjectsInRange(bar, 10, 2D, .., vehicles)`) whose hitch (vtable +0xEC, true) is unlocked
/// (+0x428 bit 3), within 0.5 in 2D (`sqrt`) and 1.0 in height (`fabsf`) of the bar: it gets `SetTowLink(this,
/// false)` (vtable +0xF4) and the hoist angle drops by 100.
#[hook(addr = 0x6A40F0, conv = thiscall, name = "CAutomobile::TowTruckControl", this = "CAutomobile",
       expect = "83 EC 6C 56 8B F1", unverified = "frontier2-b25", route = "L2", hazards = "none",
       note = "the tow link of the hitched vehicle")]
pub fn tow_truck_control(this: GPtr<u8>) {
    if rd::<u8>(this, 0x36) & 0xF8 != 0 {
        return;
    }
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if driver.is_null() || ped_is_player(driver) == 0 {
        return;
    }
    let pad = get_pad_from_player(rd::<GPtr<u8>>(this, 0x460));
    if pad.is_null() {
        return;
    }
    let ud = || xi(i32::from(get_car_gun_up_down(pad)));
    if !le_or_nan(ud().fabs(), x(gf(K_DEAD_ZONE))) {
        let ts = gf(TIME_STEP);
        let step = |k: usize| ftol32(ud().fmul_m(gf(k)).fmul_m(ts)) as i16 as i32;
        let angle = || i32::from(rd::<u16>(this, 0x86C));
        if !le_or_nan(ud(), x(gf(K_ZERO))) {
            let floor = if rd::<u32>(this, 0x4C8) != 0 { i32::from(gr16u(HOIST_UP_LIMIT)) } else { 0 };
            let a = if floor > angle() - step(K_DOWN_SPEED) { floor } else { angle() - step(K_DOWN_SPEED) };
            wr::<u16>(this, 0x86C, a as u16);
        } else if rd::<u16>(this, 0x86C) < gr16u(HOIST_DOWN_LIMIT) {
            let limit = i32::from(gr16u(HOIST_DOWN_LIMIT));
            let a = if limit < angle() - step(K_UP_SPEED) { limit } else { angle() - step(K_UP_SPEED) };
            wr::<u16>(this, 0x86C, a as u16);
        }
    }
    if rd::<u16>(this, 0x86C) != gr16u(HOIST_DOWN_LIMIT) || rd::<u32>(this, 0x4C8) != 0 {
        return;
    }
    let mut bar = CVector::default();
    let mut hitch = CVector::default();
    if v_get_tow_bar_pos(this, vp(&mut bar), 0, this) == 0 {
        return;
    }
    let mut count: i16 = 0;
    let mut list = [0u32; 16];
    find_objects_in_range(
        vp(&mut bar),
        10.0,
        1,
        GPtr::from_raw(&mut count as *mut i16).cast(),
        16,
        GPtr::from_raw(list.as_mut_ptr()).cast(),
        0,
        1,
        0,
        0,
        0,
    );
    let mut i: i16 = 0;
    while i < count {
        let c: GPtr<u8> = GPtr::from_addr(list[i as usize] as usize);
        i = i.wrapping_add(1);
        if c == this || v_get_tow_hitch_pos(c, vp(&mut hitch), 1, this) == 0 || rd::<u8>(c, 0x428) & 8 != 0 {
            continue;
        }
        let dx = x(hitch.x).fsub_m(bar.x);
        let dy = x(hitch.y).fsub_m(bar.y);
        let dz = f32_of(x(hitch.z).fsub_m(bar.z));
        if ge_or_nan(dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt(), x(gf(K_HITCH_2D))) {
            continue;
        }
        if !ge_or_nan(X87::from_f64_exact(fabsf(dz)), x(gf(K_HITCH_Z))) {
            v_set_tow_link(c, this, 0);
            wr::<u16>(this, 0x86C, rd::<u16>(this, 0x86C).wrapping_add(0xFF9C));
            return;
        }
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_UPDATE_TRAILER_LINK, HOOK_UPDATE_TRACTOR_LINK, HOOK_TOW_TRUCK_CONTROL];
