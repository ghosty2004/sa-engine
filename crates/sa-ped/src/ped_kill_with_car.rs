//! `CPed::KillPedWithCar` (PEDHIT): a vehicle hits a ped. Ported from the 1.0 US bytes (0x5F0360..0x5F1309,
//! spec `docs/re/specs/PED/CPed.md`); gta-reversed only wraps the original (`.Reversed = false`).
//!
//! Frame of the original (offsets from ESP after the prologue's `push edi`): `d` (ped - car) at +0x1C, hit zone
//! byte +0x13, the strong path's direction +0x30, weapon +0x50, anim +0x60, anim speed +0x70, the `CEventDamage`
//! at +0x80 (0x44 bytes, its `CPedDamageResponse` at +0x38) and the `CPedDamageResponseCalculator` at +0xC4.
//! The vector helpers (0x40FE30 / 0x40FE60 / 0x40FE90 / 0x40FEC0 / 0x406D70 / 0x411A00) are called as the
//! original calls them; the inline x87 sums keep their order.

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `.data` 0x8D22A0..0x8D22AC: the car force coefficients (mass cap 1600, point scale 0.25, -0.5, bikes -0.75)
const MASS_CAP: usize = 0x8D22A0;
const POINT_SCALE: usize = 0x8D22A4;
const FORCE_SCALE: usize = 0x8D22A8;
const FORCE_SCALE_BIKE: usize = 0x8D22AC;

/// `WEAPON_RAMMEDBYCAR` / `WEAPON_RUNOVERBYCAR`
const RAMMED: u32 = 49;
const RUN_OVER: u32 = 50;

use sa_input::bindings::get_pad;
use sa_core::bindings::tm_get_simplest_active_task;
use sa_core::bindings::vector_dot_product as dot_product;
use sa_core::bindings::vector_magnitude;
use sa_core::bindings::vector_normalise;
use sa_world::bindings::apply_force as physical_apply_force;
use crate::bindings::damage_calc_compute_damage_response;
use crate::bindings::damage_calc_ctor;
use sa_world::bindings::damage_calc_dtor;
use sa_world::bindings::entity_get_col_model;
use sa_world::bindings::event_damage_affects_ped;
use sa_world::bindings::event_damage_ctor;
use sa_world::bindings::event_damage_dtor;
use sa_world::bindings::event_group_add;
use sa_world::bindings::min_f;
use sa_world::bindings::pad_start_shake;
use sa_world::bindings::physical_apply_turn_force;
use sa_world::bindings::report_crime;
use sa_world::bindings::tm_find_active_task_by_type;
use sa_world::bindings::vector_add;
use sa_world::bindings::vector_add_assign;
use sa_world::bindings::vector_magnitude_sqr;
use sa_world::bindings::vector_mul_scalar;
use sa_world::bindings::vector_scale as scalar_mul_vector;
use sa_world::bindings::vector_sub;
use sa_world::bindings::vector_sub_assign;
use crate::bindings::ped_is_player;
use sa_world::bindings::anim_blend_animation as blend_animation;
use sa_world::bindings::audio_add_event as add_audio_event;
use crate::bindings::get_local_direction as ped_get_local_direction;
use sa_world::bindings::ped_say;
use sa_pop::bindings::automobile_remove_bonnet_in_ped_collision as remove_bonnet_in_ped_collision;
use sa_pop::bindings::localisation_knock_down_peds as knock_down_peds;
orig! {
    /// `CTask::GetTaskType` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
}

/// A local `CVector` the original passes by address.
type V3 = [f32; 3];

fn vp(v: &mut V3) -> GPtr<u8> {
    lp(v).cast()
}

/// `(a.y*b.y + a.z*b.z) + a.x*b.x` as the original's inline sums compute it (`order` = the component order).
fn dot_in(a: V3, b: GPtr<u8>, order: [usize; 3]) -> X87 {
    let c = |i: usize| x(a[i]).fmul_m(rd::<f32>(b, 4 * i as isize));
    c(order[0]).fadd(c(order[1])).fadd(c(order[2]))
}

/// `ST0 = |car.moveSpeed|` (0x4082C0).
fn speed(car: GPtr<u8>) -> X87 {
    X87::from_f64_exact(vector_magnitude(GPtr::<u8>::cast::<u32>(car.byte_add(0x44))))
}

/// Where the strong path goes after its impact geometry (block 4 of the spec).
enum Hit {
    /// the runover entries 0x5F0B5F / 0x5F0B6A (hit zone 2) and 0x5F0B6F (hit zone kept)
    RunOver { zone2: bool },
    /// 0x5F0BAF: no throw, only the 0x77 audio event
    Plain,
    /// 0x5F071C: thrown onto the bonnet / over the roof
    Throw,
}

/// `CPed::KillPedWithCar(CVehicle* car, float damageIntensity, bool playDeadAnim)` (0x5F0360, `ret 0xC`).
///
/// Gates (falling / dying peds only get the collision ignore, dead peds nothing; boats, planes, the player's
/// contact vehicle; a destroy-car-melee task against a slow car), then two paths: the **strong** one (intensity
/// > 12, not the player: pad shake for the player's car, the bonnet / roof throw with its lift, the runover, the
/// side hit; a 1000-damage event with a knock-down anim, crime report) and the **weak** one (a 30 / 150 damage
/// bump, the ped pushed with the car). Both end with a force on the car (`ApplyForce` on the vehicle).
#[hook(addr = 0x5F0360, conv = thiscall, name = "CPed::KillPedWithCar", this = "CPed",
       expect = "6A FF 64 A1 00 00 00 00 68 00 DE 83 00",
       unverified = "pedhit-1", route = "L2", hazards = "heap-writes,uninit-locals", recs = "driving",
       note = "StartShake's frequency dword carries stale upper stack bytes in the original (the callee reads the low byte); the CEventDamage local's unwritten bytes are 0 here; the SEH frame is not reproduced")]
pub fn ped_kill_ped_with_car(this: GPtr<u8>, car: GPtr<u8>, intensity: f32, play_dead_anim: u32) {
    let intel = || rd::<GPtr<u8>>(this, 0x47C);
    // block 1: gates
    if !tm_get_simplest_active_task(intel().byte_add(4)).is_null() {
        let t = tm_get_simplest_active_task(intel().byte_add(4));
        match task_get_type(t) {
            // TASK_SIMPLE_FALL, TASK_SIMPLE_DIE
            0xCF | 0xD4 => {
                if rd::<u32>(this, 0x128) == 0 || rd::<u8>(car, 0x36) & 0xF8 == 0 {
                    wr::<GPtr<u8>>(this, 0x128, car);
                }
                return;
            }
            // TASK_SIMPLE_DEAD
            0xDA => return,
            _ => {}
        }
    }
    let contact = rd::<GPtr<u8>>(this, 0x584);
    if !contact.is_null() && rd::<u8>(contact, 0x36) & 7 == 2 {
        if rd::<i32>(contact, 0x590) == 5 || ped_is_player(this) != 0 {
            return;
        }
    } else {
        let v = rd::<GPtr<u8>>(this, 0x58C);
        if !v.is_null() && v == car && (rd::<i32>(car, 0x590) == 5 || rd::<i32>(car, 0x594) == 4) {
            return;
        }
    }
    let t = tm_find_active_task_by_type(intel().byte_add(4), 0x3EC);
    if !t.is_null() && rd::<GPtr<u8>>(t, 0x10) == car {
        let mut mv: V3 = vec3(car.byte_add(0x44).cast());
        let s = X87::from_f64_exact(vector_magnitude_sqr(vp(&mut mv)).to_f64());
        if !s.fcom(x(0.0225)).test_parity(5) {
            return;
        }
    }
    // block 2: d = ped - car (z, y, x)
    let cp = vec3(ent_pos(car));
    let pp = vec3(ent_pos(this));
    let mut d: V3 = [0.0; 3];
    for i in [2, 1, 0] {
        d[i] = f32_of(x(pp[i]).fsub_m(cp[i]));
    }
    let strong = !x(intensity).fcom(x(12.0)).test_any(0x41) && ped_is_player(this) == 0;
    let mut calc = [0u32; 5];
    let calc_p: GPtr<u8> = GPtr::from_raw(&mut calc as *mut [u32; 5]).cast();
    let mut ev = [0u32; 0x11];
    let ev_p: GPtr<u8> = GPtr::from_raw(&mut ev as *mut [u32; 0x11]).cast();
    let moving = car.byte_add(0x44);
    if strong {
        // block 3
        let mut zone: u8 = 0;
        let rnd = (at_site(0x5F04F0, rand) & 3) as u8;
        let mut weapon = RAMMED;
        if car == find_player_vehicle(-1, 0) {
            let inv = f32_of(x(1.0).fdiv_m(rd::<f32>(car, 0x8C)));
            let v = speed(car).fmul_m(inv).fmul_m(200_000.0).fadd_m(80.0);
            let v = if !v.fcom(x(250.0)).test_any(0x41) {
                x(250.0)
            } else {
                speed(car).fmul_m(inv).fmul_m(200_000.0).fadd_m(80.0)
            };
            let s = ftol32(v) as u8;
            let q = 40000 / i32::from(s);
            pad_start_shake(get_pad(0), q as u32, u32::from(s), 0);
        }
        wr::<u32>(this, 0x46C, rd::<u32>(this, 0x46C) & !1);
        let mut back = [f32_of(x(rd::<f32>(car, 0x44)).fchs()), f32_of(x(rd::<f32>(car, 0x48)).fchs())];
        let mut dir = ped_get_local_direction(this, lp(&mut back).cast());
        // block 4
        let cm = entity_get_col_model(car);
        let m = rd::<GPtr<u8>>(car, 0x14);
        let max_x = rd::<f32>(cm, 0xC);
        let side = f32_of(dot_in(d, m, [2, 1, 0]));
        let up = f32_of(dot_in(d, m.byte_add(0x20), [2, 1, 0]));
        let hit = if rd::<i32>(car, 0x594) == 6 {
            Hit::RunOver { zone2: true }
        } else if !X87::from_f64_exact(dot_product(moving, m.byte_add(0x10)).to_f64()).fcom(x(0.0)).test_parity(5) {
            Hit::Plain
        } else {
            let a = if x(side).fcom(x(0.0)).test_parity(5) { x(side) } else { x(side).fchs() };
            if !a.fcom(x(max_x).fmul_m(0.99)).test_any(0x41) {
                zone = if x(side).fcom(x(0.0)).test_any(0x41) { 3 } else { 4 };
                let fwd = f32_of(dot_in(d, m.byte_add(0x10), [2, 1, 0]));
                let lim = x(rd::<f32>(entity_get_col_model(car), 0x10)).fmul_m(0.85);
                if !lim.fcom(x(fwd).fabs()).test_any(0x41) { Hit::RunOver { zone2: false } } else { Hit::Plain }
            } else if (rnd != 0 && (x(up).fcom(x(0.1)).test_any(0x41) || rnd <= 1))
                || rd::<u8>(rd::<GPtr<u8>>(car, 0x384), 0xCC) & 8 != 0
            {
                // a low or random-miss front hit, or a big vehicle (handling model flag 8): run over
                Hit::RunOver { zone2: true }
            } else {
                Hit::Throw
            }
        };
        match hit {
            Hit::Throw => {
                zone = 1;
                let max_y = rd::<f32>(entity_get_col_model(car), 0x10);
                let min_y = rd::<f32>(entity_get_col_model(car), 0x4);
                let max_z = rd::<f32>(entity_get_col_model(car), 0x14);
                let m = rd::<GPtr<u8>>(car, 0x14);
                let fwd_z = rd::<f32>(m, 0x18);
                let pos = if m.is_null() { car.byte_add(4) } else { m.byte_add(0x30) };
                let (mut t1, mut t2, mut s, mut top): (V3, V3, V3, V3) = ([0.0; 3], [0.0; 3], [0.0; 3], [0.0; 3]);
                let mut top_z;
                let len;
                if !x(fwd_z).fcom(x(-0.2)).test_parity(5) {
                    scalar_mul_vector(vp(&mut t1), min_y, m.byte_add(0x10));
                    scalar_mul_vector(vp(&mut t2), max_z, m.byte_add(0x20));
                    vector_add(vp(&mut s), pos, vp(&mut t2));
                    vector_add(vp(&mut top), vp(&mut s), vp(&mut t1));
                    top_z = top[2];
                    len = f32_of(x(max_y).fsub_m(min_y));
                } else if !x(fwd_z).fcom(x(0.1)).test_any(0x41) {
                    scalar_mul_vector(vp(&mut t1), max_y, m.byte_add(0x10));
                    scalar_mul_vector(vp(&mut t2), max_z, m.byte_add(0x20));
                    vector_add(vp(&mut s), pos, vp(&mut t2));
                    vector_add(vp(&mut top), vp(&mut s), vp(&mut t1));
                    top_z = top[2];
                    len = max_y;
                    // the ped is lifted towards the bonnet top, through m_matrix without a null check (0x5F089C)
                    let pm = rd::<GPtr<u8>>(this, 0x14);
                    let pz = || ent_pos(this).wrapping_index(2).read();
                    if !x(top_z).fsub_m(pz()).fcom(x(0.0)).test_any(0x41) {
                        let z = x(top_z).fsub_m(pz()).fmul_m(0.5).fadd_m(rd::<f32>(pm, 0x38));
                        wr::<f32>(pm, 0x38, f32_of(z));
                        top_z = f32_of(x(top_z).fsub_m(pz()).fmul_m(0.25).fadd_m(top_z));
                    }
                } else {
                    scalar_mul_vector(vp(&mut t2), max_z, m.byte_add(0x20));
                    vector_add(vp(&mut top), pos, vp(&mut t2));
                    top_z = top[2];
                    len = rd::<f32>(entity_get_col_model(car), 0x10);
                }
                // 0x5F0917: the lift that carries the ped over the top in the time the car needs for `len`
                let time = speed(car).fdivr_m(len);
                let pz = ent_pos(this).wrapping_index(2).read();
                let lift = f32_of(x(top_z).fsub_m(pz).fdiv(time));
                let r = at_site(0x5F0945, rand) & 0xFF;
                let lift = f32_of(
                    xi(r).fmul(X87::from_f64_exact(0.002)).fadd(X87::from_f64_exact(1.5)).fmul_m(lift),
                );
                let mut n: V3 = vec3(moving.cast());
                vector_normalise(GPtr::<u8>::cast::<[f32;3]>(vp(&mut n)));
                let mut w: V3 = [0.0; 3];
                scalar_mul_vector(vp(&mut w), f32_of(x(lift).fmul_m(0.2)), vp(&mut n));
                w[2] = f32_of(x(w[2]).fadd_m(lift));
                for (i, c) in w.iter().enumerate() {
                    wr::<f32>(this, 0x44 + 4 * i as isize, *c);
                }
                dir += 2;
                if dir > 3 {
                    dir -= 4;
                }
                if rd::<i32>(car, 0x590) == 0 {
                    let bonnet = remove_bonnet_in_ped_collision(car);
                    if !bonnet.is_null() {
                        let right_side = at_site(0x5F0A0F, rand) & 1 != 0;
                        let (mut a, mut b, mut c, mut out): (V3, V3, V3, V3) = ([0.0; 3], [0.0; 3], [0.0; 3], [0.0; 3]);
                        scalar_mul_vector(vp(&mut a), 0.5, rd::<GPtr<u8>>(car, 0x14).byte_add(0x20));
                        scalar_mul_vector(vp(&mut b), 0.1, rd::<GPtr<u8>>(car, 0x14));
                        if right_side {
                            vector_add(vp(&mut c), this.byte_add(0x44), vp(&mut b));
                        } else {
                            vector_sub(vp(&mut c), this.byte_add(0x44), vp(&mut b));
                        }
                        vector_add(vp(&mut out), vp(&mut c), vp(&mut a));
                        for (i, v) in out.iter().enumerate() {
                            wr::<f32>(bonnet, 0x44 + 4 * i as isize, *v);
                        }
                        let m = rd::<GPtr<u8>>(car, 0x14);
                        let mut f: V3 = [0.0; 3];
                        scalar_mul_vector(vp(&mut f), 10.0, m.byte_add(0x20));
                        let p: V3 = vec3(m.byte_add(0x10).cast());
                        physical_apply_turn_force(bonnet, sa_math::vector::CVector::new(f32::from_bits(f[0].to_bits()), f32::from_bits(f[1].to_bits()), f32::from_bits(f[2].to_bits())), sa_math::vector::CVector::new(f32::from_bits(p[0].to_bits()), f32::from_bits(p[1].to_bits()), f32::from_bits(p[2].to_bits())));
                    }
                }
                vector_sub(vp(&mut d), ent_pos(this).cast(), ent_pos(car).cast());
                if knock_down_peds() != 0 {
                    add_audio_event(car.byte_add(0x138), 0x7A, f32::from_bits(0));
                }
            }
            Hit::RunOver { zone2 } => {
                if zone2 {
                    zone = 2;
                }
                weapon = RUN_OVER;
                let mut o: V3 = [0.0; 3];
                vector_mul_scalar(vp(&mut o), moving, 0.9);
                for (i, v) in o.iter().enumerate() {
                    wr::<f32>(this, 0x44 + 4 * i as isize, *v);
                }
                wr::<f32>(this, 0x4C, 0.0);
                if dir == 1 || dir == 3 {
                    dir = 2;
                }
                if knock_down_peds() != 0 {
                    add_audio_event(car.byte_add(0x138), 0x77, f32::from_bits(0));
                }
            }
            Hit::Plain => {
                if knock_down_peds() != 0 {
                    add_audio_event(car.byte_add(0x138), 0x77, f32::from_bits(0));
                }
            }
        }
        // block 5: crime and the damage event
        let driver = rd::<GPtr<u8>>(car, 0x460);
        if !driver.is_null() {
            report_crime(if rd::<i32>(this, 0x598) == 6 { 11 } else { 10 }, this, driver);
        }
        damage_calc_ctor(calc_p, car.addr() as u32, f32::from_bits(1000.0f32.to_bits()).to_bits(), weapon, 3, 0);
        let in_vehicle = (rd::<u32>(this, 0x46C) >> 8) & 0xFFFF_FF01;
        event_damage_ctor(ev_p, car, g::<u32>(TIME_MS).read(), weapon, 3, dir as u32, 0, in_vehicle);
        let rnd2 = at_site(0x5F0C4B, rand) & 3;
        let side_anim = |left: i32, right: i32, other: i32| match zone {
            3 if rnd2 > 1 => left,
            4 if rnd2 > 1 => right,
            _ => other,
        };
        let anim = match dir as u32 {
            0 => side_anim(25, 27, 24),
            1 => 22,
            2 => side_anim(22, 23, 26),
            3 => 23,
            _ => 0xBF,
        };
        if event_damage_affects_ped(ev_p, this) != 0 {
            let mut anim_speed = 1.0f32;
            let blend = if weapon == RAMMED {
                f32_of(speed(car).fmul_m(8.0).fadd_m(4.0))
            } else {
                let b = f32_of(speed(car).fmul_m(12.0).fadd_m(4.0));
                anim_speed = f32_of(speed(car).fmul_m(16.0).fadd(X87::from_f64_exact(1.0)));
                b
            };
            damage_calc_compute_damage_response(calc_p, this, ev_p.byte_add(0x38), 1);
            wr::<u32>(ev_p, 0x28, 0);
            wr::<i32>(ev_p, 0x2C, anim);
            wr::<f32>(ev_p, 0x30, blend);
            wr::<f32>(ev_p, 0x34, anim_speed);
            if play_dead_anim as u8 != 0 {
                let a = blend_animation(rd::<GPtr<u8>>(this, 0x18), 0, anim as u32, f32::from_bits(blend.to_bits()));
                wr::<f32>(a, 0x24, anim_speed);
                wr::<u8>(ev_p, 0x25, rd::<u8>(ev_p, 0x25) | 4);
            }
            event_group_add(intel().byte_add(0x68), ev_p, 0);
            if rd::<u32>(this, 0x128) == 0 {
                wr::<GPtr<u8>>(this, 0x128, car);
            }
            let f = rd::<u32>(this, 0x470);
            wr::<u32>(this, 0x470, if zone == 1 { f | 0x10 } else { f & !0x10 });
            wr::<u8>(intel(), 0x260, 1);
        } else {
            wr::<u32>(this, 0x470, rd::<u32>(this, 0x470) & !0x10);
        }
        ped_say(this, 0x156, 0, f32::from_bits(1.0f32.to_bits()), 0, 0, 0);
    } else {
        // block 6: the weak path
        let low = !x(rd::<f32>(this, 0xE8)).fcom(x(-0.8)).test_parity(5) && !x(intensity).fcom(x(3.0)).test_any(0x41);
        if !low {
            if x(intensity).fcom(x(6.0)).test_any(0x41) {
                return;
            }
            if ped_is_player(this) != 0 && x(intensity).fcom(x(10.0)).test_any(0x41) {
                return;
            }
        }
        wr::<u32>(this, 0x46C, rd::<u32>(this, 0x46C) & !1);
        let mut back = [f32_of(x(rd::<f32>(car, 0x44)).fchs()), f32_of(x(rd::<f32>(car, 0x48)).fchs())];
        let dir = ped_get_local_direction(this, lp(&mut back).cast());
        let mut dmg = 30.0f32;
        if ped_is_player(this) != 0 && rd::<i32>(car, 0x594) == 6 {
            dmg = 150.0;
        }
        damage_calc_ctor(calc_p, car.addr() as u32, f32::from_bits(dmg.to_bits()).to_bits(), RAMMED, 3, 0);
        let in_vehicle = (rd::<u32>(this, 0x46C) >> 8) & 0xFFFF_FF01;
        event_damage_ctor(ev_p, car, g::<u32>(TIME_MS).read(), RAMMED, 3, dir as u32, 0, in_vehicle);
        let anim = dir.wrapping_add(24);
        if event_damage_affects_ped(ev_p, this) != 0 {
            damage_calc_compute_damage_response(calc_p, this, ev_p.byte_add(0x38), 1);
            wr::<u32>(ev_p, 0x28, 0);
            wr::<i32>(ev_p, 0x2C, anim);
            wr::<f32>(ev_p, 0x30, 8.0);
            wr::<f32>(ev_p, 0x34, 1.0);
            if play_dead_anim as u8 != 0 {
                let a = blend_animation(rd::<GPtr<u8>>(this, 0x18), 0, anim as u32, f32::from_bits(8.0f32.to_bits()));
                wr::<f32>(a, 0x24, 1.0);
                wr::<u8>(ev_p, 0x25, rd::<u8>(ev_p, 0x25) | 4);
            }
            event_group_add(intel().byte_add(0x68), ev_p, 0);
            let keep = ped_is_player(this) != 0
                && rd::<u32>(this, 0x1C) & 0x1000 == 0
                && rd::<i32>(car, 0x594) != 6
                && x(rd::<f32>(this, 0xE8)).fcom(x(-0.8)).test_parity(5);
            if !keep {
                wr::<GPtr<u8>>(this, 0x128, car);
            }
            wr::<u8>(intel(), 0x260, 1);
        }
        wr::<u32>(this, 0x470, rd::<u32>(this, 0x470) & !0x10);
        let pv = this.byte_add(0x44);
        let flags = rd::<u32>(this, 0x1C);
        if rd::<i32>(car, 0x594) == 6 {
            if flags & 0x1000 != 0 {
                for i in 0..3 {
                    wr::<f32>(pv, 4 * i, 0.0);
                }
            } else {
                let mut n: V3 = vec3(moving.cast());
                vector_normalise(GPtr::<u8>::cast::<[f32;3]>(vp(&mut n)));
                let k = f32_of(dot_in(n, pv, [1, 2, 0]));
                let mut o: V3 = [0.0; 3];
                vector_mul_scalar(vp(&mut o), vp(&mut n), k);
                vector_sub_assign(pv, vp(&mut o));
                vector_mul_scalar(vp(&mut o), moving, 0.3);
                vector_add_assign(pv, vp(&mut o));
            }
        } else if flags & 0x1000 == 0 {
            let mut o: V3 = [0.0; 3];
            vector_mul_scalar(vp(&mut o), moving, 0.75);
            for (i, v) in o.iter().enumerate() {
                wr::<f32>(pv, 4 * i as isize, *v);
            }
        }
        wr::<f32>(this, 0x4C, 0.0);
        if knock_down_peds() != 0 {
            add_audio_event(car.byte_add(0x138), 0x79, f32::from_bits(0));
        }
        ped_say(this, 0x159, 0, f32::from_bits(1.0f32.to_bits()), 0, 0, 0);
    }
    event_damage_dtor(ev_p);
    damage_calc_dtor(calc_p);
    // block 7: the force on the car, along the car's plane
    let up = rd::<GPtr<u8>>(car, 0x14).byte_add(0x20);
    let k = f32_of(dot_in(d, up, [1, 2, 0]));
    let mut t: V3 = [0.0; 3];
    scalar_mul_vector(vp(&mut t), k, up);
    for i in 0..3 {
        d[i] = f32_of(x(d[i]).fsub_m(t[i]));
    }
    let mut n = d;
    vector_normalise(GPtr::<u8>::cast::<[f32;3]>(vp(&mut n)));
    let s = dot_in(n, moving, [1, 2, 0]);
    if rd::<u8>(this, 0x470) & 0x10 == 0 {
        n[2] = f32_of(x(n[2]).fsub_m(0.2));
    }
    let scale = g::<f32>(if rd::<i32>(car, 0x590) == 9 { FORCE_SCALE_BIKE } else { FORCE_SCALE }).read();
    let s = f32_of(s.fmul_m(scale));
    let q = g::<f32>(POINT_SCALE).read();
    let p = [f32_of(x(d[0]).fmul_m(q)), f32_of(x(d[1]).fmul_m(q)), f32_of(x(d[2]).fmul_m(q))];
    let mass = rd::<f32>(car, 0x8C);
    let cap = g::<f32>(MASS_CAP).read();
    let a = f32_of(X87::from_f64_exact(min_f(1.0f32.to_bits(), f32_of(x(mass).fdiv_m(cap)).to_bits()).to_f64()));
    let k2 = X87::from_f64_exact(min_f(mass.to_bits(), cap.to_bits()).to_f64()).fmul_m(a).fmul_m(s);
    let f = [f32_of(x(n[0]).fmul(k2)), f32_of(x(n[1]).fmul(k2)), f32_of(x(n[2]).fmul(k2))];
    physical_apply_force(car, sa_math::vector::CVector::new(f32::from_bits(f[0].to_bits()), f32::from_bits(f[1].to_bits()), f32::from_bits(f[2].to_bits())), sa_math::vector::CVector::new(f32::from_bits(p[0].to_bits()), f32::from_bits(p[1].to_bits()), f32::from_bits(p[2].to_bits())), 1);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PED_KILL_PED_WITH_CAR];
