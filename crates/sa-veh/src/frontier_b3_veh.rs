//! FRONTIER-2 turn 3, lane B (batch `frontier2-b10`): the aircraft and boat `BlowUpCar` virtuals. Ported from the
//! 1.0 US bytes; gta-reversed as a reading aid only (the bytes win). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::automobile_blow_up::{clump_set_flags_7322f0, damage_fuck_car_completely, kill_peds_in_vehicle};
use crate::automobile_flying_component::{object_ref_model_info, v_set_is_static, v_set_model_index_no_create};
use crate::bindings::car_set_panel_damage as set_bumper_damage;
use crate::bindings::spawn_flying_component;
use crate::common::{f32_of, g, rd, wr, x, xi};
use sa_core::bindings::{object_ctor, object_new, rp_atomic_set_frame, vector_normalise};
use sa_pop::bindings::darkel_register_car_blown_up;
use sa_rwcore::bindings::{frame_for_all_objects, frame_get_ltm, rp_atomic_clone, rw_frame_create};
use sa_stream::bindings::set_atomic_render_callback;
use sa_world::bindings::start_fire_on as start_fire;
use sa_world::bindings::vehicle_change_law_enforcer_state as change_law_enforcer_state;
use sa_world::bindings::{
    add_explosion, cam_shake, entity_attach_to_rw_object, find_player_ped, find_player_vehicle, set_door_damage,
    stats_increment_stat, vector_add_assign, world_add,
};
use sa_world::world::common::entity_pos;

/// The common start of the aircraft `BlowUpCar`s: a player kill (the damager is the player ped or the player's
/// vehicle) gives the focus player +20 to its +0x140 counter, +10.0 to its +0x148 float and stat 0x7D
/// `rand() % 6000 + 4000` (rand at `site`).
fn player_blow_up_bonus(damager: GPtr<u8>, site: u32) {
    if damager == find_player_ped(-1) || damager == find_player_vehicle(-1, 0) {
        let pi = 0xB7_CD98 + 0x190 * usize::from(g::<u8>(0xB7_CD74).read());
        let money = g::<i32>(pi + 0x140);
        money.write(money.read().wrapping_add(20));
        let f = g::<f32>(pi + 0x148);
        f.write(f32_of(x(f.read()).fadd_m(10.0)));
        let r = sa_core::orig::at_site(site, sa_core::crt::rand);
        stats_increment_stat(0x7D, f32_of(xi(r % 6000 + 4000)));
    }
}

/// The player-driven aircraft's stop: flags +0x1C `&= !0x81`, move and turn speed zero.
fn stop_physical(this: GPtr<u8>) {
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & 0xFFFF_FF7E);
    for o in [0x4C, 0x48, 0x44, 0x58, 0x54, 0x50] {
        wr::<u32>(this, o, 0);
    }
}

/// Wrecked: status 5 (+0x36 high bits), flag +0x40 bit 29, blow-up time now, the clump's atomic flags 0x4000, the
/// damage manager wrecked.
fn set_wrecked(this: GPtr<u8>) {
    wr::<u8>(this, 0x36, (rd::<u8>(this, 0x36) & 7) | 0x28);
    wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x2000_0000);
    wr::<u32>(this, 0x4D8, g::<u32>(0xB7_CB84).read());
    clump_set_flags_7322f0(rd::<GPtr<u8>>(this, 0x18), 0x4000);
    damage_fuck_car_completely(this.byte_add(0x5A0), 0);
}

/// The body damage: both bumpers (5, 6), the six doors, the bonnet (node 5) flies off and the bonnet frame's
/// (+0x65C) atomic is hidden; `check_frame`: the plane skips the atomic search without a frame, the heli does not.
fn damage_body(this: GPtr<u8>, check_frame: bool) {
    set_bumper_damage(this, 5, 0);
    set_bumper_damage(this, 6, 0);
    for door in 0..6 {
        set_door_damage(this, door, 0);
    }
    spawn_flying_component(this, 5, 1);
    let frame = rd::<GPtr<u8>>(this, 0x65C);
    if check_frame && frame.is_null() {
        return;
    }
    let mut atomic = [0u32; 1];
    frame_for_all_objects(frame, 0x6A_0750, GPtr::from_raw(atomic.as_mut_ptr()).addr() as u32);
    if atomic[0] != 0 {
        wr::<u8>(GPtr::from_addr(atomic[0] as usize), 2, 0);
    }
}

/// The tail of the aircraft `BlowUpCar`s: lights (+0x4A8 `&= 0xF8`), health, bomb timer cleared, a 0.4 camera shake,
/// occupants killed, engine / lights / siren flags cleared (+0x4A8, +0x428, +0x42D, +0x868; the plane also +0xA00),
/// the siren counters 0x9690A8 / 0x9690AC decremented, law enforcer state off, a fire (0.8, 7 s) by the damager, the
/// Darkel kill, and an explosion (type `ty`, no lifetime, sound, shake -1, never hidden) at the position.
fn blow_up_tail(this: GPtr<u8>, damager: GPtr<u8>, plane: bool, ty: i32) {
    wr::<u8>(this, 0x4A8, rd::<u8>(this, 0x4A8) & 0xF8);
    wr::<u32>(this, 0x4C0, 0);
    wr::<u16>(this, 0x4DE, 0);
    let p = entity_pos(this);
    cam_shake(GPtr::from_addr(0xB6_F028), f32::from_bits(0x3ECC_CCCD), rd(p, 0), rd(p, 4), rd(p, 8));
    kill_peds_in_vehicle(this);
    wr::<u8>(this, 0x4A8, rd::<u8>(this, 0x4A8) & 0xE7);
    wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xAF);
    if plane {
        wr::<u8>(this, 0xA00, 0);
    }
    wr::<u8>(this, 0x42D, rd::<u8>(this, 0x42D) & 0x7F);
    wr::<u8>(this, 0x868, rd::<u8>(this, 0x868) & 0xFE);
    if rd::<u8>(this, 0x428) & 2 != 0 {
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xFD);
        let c = g::<i32>(0x96_90A8);
        c.write(c.read().wrapping_sub(1));
    }
    if rd::<u8>(this, 0x428) & 4 != 0 {
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xFB);
        let c = g::<i32>(0x96_90AC);
        c.write(c.read().wrapping_sub(1));
    }
    change_law_enforcer_state(this, 0);
    start_fire(GPtr::from_addr(0xB7_1F80), this, damager, f32::from_bits(0x3F4C_CCCD), 1, 7000, 0);
    darkel_register_car_blown_up(this, 0);
    let p = entity_pos(this);
    add_explosion(this, damager, ty, rd(p, 0), rd(p, 4), rd(p, 8), 0, 1, f32::from_bits(0xBF80_0000), 0);
}

/// `CPlane::BlowUpCar(CEntity* damager, bool hideExplosion)` (0x6CCCF0, vtable slot; `ret 8`). Only when the plane can
/// be damaged (+0x42A bit 5). A plane not driven by the player (status != 0) that is not already crashing (+0x3BA !=
/// 0x39) and is not the RC model 464 only starts crashing (+0x3BA = 0x39, health 0). Otherwise: the player bonus (rand
/// at 0x6CCD7D), a player-driven plane flags its nine occupants (+0x474 |= 2) and stops; wrecked; models other than
/// 464 get the body damage (the atomic search only with a bonnet frame); the tail with +0xA00 cleared and explosion
/// type 12 for model 464, else 7. The hide flag is never used.
#[hook(addr = 0x6CCCF0, conv = thiscall, name = "CPlane::BlowUpCar", this = "CPlane", expect = "53 55 56 8B F1 F6 86 2A 04 00 00 20",
       unverified = "frontier2-b10", route = "L2", hazards = "heap-writes,stack-residue",
       note = "rand at_site 0x6CCD82; the original keeps the stat value and the bonnet atomic in its own argument slots (not modelled); spawns the bonnet, starts a fire and an explosion")]
pub fn plane_blow_up_car(this: GPtr<u8>, damager: GPtr<u8>, _hide: u32) {
    if rd::<u8>(this, 0x42A) & 0x20 == 0 {
        return;
    }
    if rd::<u8>(this, 0x36) & 0xF8 != 0 && rd::<u8>(this, 0x3BA) != 0x39 && rd::<i16>(this, 0x22) != 0x1D0 {
        wr::<u8>(this, 0x3BA, 0x39);
        wr::<u32>(this, 0x4C0, 0);
        return;
    }
    player_blow_up_bonus(damager, 0x6C_CD82);
    if rd::<u8>(this, 0x36) & 0xF8 == 0 {
        for o in (0x460..=0x480).step_by(4) {
            let p = rd::<GPtr<u8>>(this, o);
            if !p.is_null() {
                wr::<u32>(p, 0x474, rd::<u32>(p, 0x474) | 2);
            }
        }
        stop_physical(this);
    }
    set_wrecked(this);
    if rd::<i16>(this, 0x22) != 0x1D0 {
        damage_body(this, true);
    }
    let ty = if rd::<i16>(this, 0x22) == 0x1D0 { 0xC } else { 7 };
    blow_up_tail(this, damager, true, ty);
}

/// `CHeli::BlowUpCar(CEntity* damager, bool hideExplosion)` (0x6C6D30, vtable slot; `ret 8`). Only when the heli can
/// be damaged. A heli not driven by the player that is not already crashing (+0x3BA != 0x3A) and is not one of the
/// RC models 465 / 501 only starts crashing (+0x3BA = 0x3A, health 0). Otherwise: the player bonus (rand at
/// 0x6C6DC3), the news chopper (488) clears 0xB7CB8C, a player-driven heli stops; wrecked; models other than 465 /
/// 501 get the body damage (the atomic search without a frame check); the tail and explosion type 12 for the RC
/// models, else 7. The hide flag is never used.
#[hook(addr = 0x6C6D30, conv = thiscall, name = "CHeli::BlowUpCar", this = "CHeli", expect = "53 56 8B F1 F6 86 2A 04 00 00 20",
       unverified = "frontier2-b10", route = "L2", hazards = "heap-writes,stack-residue",
       note = "rand at_site 0x6C6DC8; the original keeps the stat value and the bonnet atomic in its own argument slots (not modelled); spawns the bonnet, starts a fire and an explosion")]
pub fn heli_blow_up_car(this: GPtr<u8>, damager: GPtr<u8>, _hide: u32) {
    if rd::<u8>(this, 0x42A) & 0x20 == 0 {
        return;
    }
    let rc = |m: i16| m == 0x1D1 || m == 0x1F5;
    if rd::<u8>(this, 0x36) & 0xF8 != 0 && rd::<u8>(this, 0x3BA) != 0x3A && !rc(rd::<i16>(this, 0x22)) {
        wr::<u8>(this, 0x3BA, 0x3A);
        wr::<u32>(this, 0x4C0, 0);
        return;
    }
    player_blow_up_bonus(damager, 0x6C_6DC8);
    if rd::<i16>(this, 0x22) == 0x1E8 {
        g::<u8>(0xB7_CB8C).write(0);
    }
    if rd::<u8>(this, 0x36) & 0xF8 == 0 {
        stop_physical(this);
    }
    set_wrecked(this);
    if !rc(rd::<i16>(this, 0x22)) {
        damage_body(this, false);
    }
    let ty = if rc(rd::<i16>(this, 0x22)) { 0xC } else { 7 };
    blow_up_tail(this, damager, false, ty);
}

/// `CBoat::BlowUpCar(CEntity* damager, bool hideExplosion)` (0x6F21B0, vtable slot; `ret 8`). Only when the boat can
/// be damaged: `ms.z += 0.13`, status 5, flag +0x40 bit 29, clump flags 0x4000, health and bomb timer cleared, a 0.4
/// camera shake, occupants killed, +0x4A8 `&= 0xE7`, +0x428 `&= 0xAF`, law enforcer state off, an explosion (type 6,
/// no lifetime, sound, shake -1, the hide flag dword) and the Darkel kill. With a moving-part frame (+0x5B4) holding a
/// visible atomic (`GetBoatAtomicObjectCB`): a new `CObject` (model 0x17B, the boat's model info referenced) gets a
/// clone of the atomic on a new frame at the part's LTM (no render callback), is not streamed out (+0x1C bit 19),
/// mass 10, turn mass 25, air resistance 0.99, elasticity 0.1, buoyancy 0.1067, temporary (+0x13C = 3, the temporary
/// count +1), not static, +0x140 bit 0 cleared, removed in 20 s; it takes the boat's speed (z 0.3 when the boat is
/// upright, else 0), turn speed (0.5, 2 y, 2 z), moves by the normalised offset from the boat (plus the boat's up axis
/// when upright), is added to the world, and the boat's part atomic is hidden.
#[hook(addr = 0x6F21B0, conv = thiscall, name = "CBoat::BlowUpCar", this = "CBoat", expect = "6A FF 64 A1 00 00 00 00 68 5B 89 84 00",
       unverified = "frontier2-b10", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the original keeps the atomic, frame and clone in its own argument slots (not modelled); creates a CObject, an RW frame and an atomic clone, adds it to the world")]
pub fn boat_blow_up_car(this: GPtr<u8>, damager: GPtr<u8>, hide: u32) {
    if rd::<u8>(this, 0x42A) & 0x20 == 0 {
        return;
    }
    let z = f32_of(x(rd::<f32>(this, 0x4C)).fadd_m(f32::from_bits(0x3E05_1EB8)));
    let st = (rd::<u8>(this, 0x36) & 7) | 0x28;
    let fl = rd::<u32>(this, 0x40) | 0x2000_0000;
    wr::<f32>(this, 0x4C, z);
    wr::<u8>(this, 0x36, st);
    wr::<u32>(this, 0x40, fl);
    clump_set_flags_7322f0(rd::<GPtr<u8>>(this, 0x18), 0x4000);
    wr::<u32>(this, 0x4C0, 0);
    wr::<u16>(this, 0x4DE, 0);
    let p = entity_pos(this);
    cam_shake(GPtr::from_addr(0xB6_F028), f32::from_bits(0x3ECC_CCCD), rd(p, 0), rd(p, 4), rd(p, 8));
    kill_peds_in_vehicle(this);
    wr::<u8>(this, 0x4A8, rd::<u8>(this, 0x4A8) & 0xE7);
    wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xAF);
    change_law_enforcer_state(this, 0);
    let p = entity_pos(this);
    add_explosion(this, damager, 6, rd(p, 0), rd(p, 4), rd(p, 8), 0, 1, f32::from_bits(0xBF80_0000), hide as u8);
    darkel_register_car_blown_up(this, 0);
    let part = rd::<GPtr<u8>>(this, 0x5B4);
    if part.is_null() {
        return;
    }
    let mut atomic = [0u32; 1];
    frame_for_all_objects(part, 0x6F_00D0, GPtr::from_raw(atomic.as_mut_ptr()).addr() as u32);
    if atomic[0] == 0 {
        return;
    }
    let mem = object_new(0x17C);
    let obj = if mem.is_null() { mem } else { object_ctor(mem) };
    if obj.is_null() {
        return;
    }
    v_set_model_index_no_create(obj, 0x17B);
    object_ref_model_info(obj, i32::from(rd::<i16>(this, 0x22)));
    let ltm = frame_get_ltm(rd::<GPtr<u8>>(this, 0x5B4));
    let frame = rw_frame_create();
    let clone = rp_atomic_clone(GPtr::from_addr(atomic[0] as usize));
    for k in 0..16 {
        wr::<u32>(frame, 0x10 + 4 * k, rd::<u32>(ltm, 4 * k));
    }
    rp_atomic_set_frame(clone, frame);
    set_atomic_render_callback(clone, 0);
    entity_attach_to_rw_object(obj, clone, 1);
    wr::<u32>(obj, 0x1C, rd::<u32>(obj, 0x1C) | 0x8_0000);
    wr::<u32>(obj, 0x8C, 0x4120_0000);
    wr::<u32>(obj, 0x90, 0x41C8_0000);
    wr::<u32>(obj, 0x98, 0x3F7D_70A4);
    wr::<u32>(obj, 0x9C, 0x3DCC_CCCD);
    wr::<u32>(obj, 0xA0, 0x3DDA_740F);
    let n = g::<u16>(0xBB_4A70);
    n.write(n.read().wrapping_add(1));
    wr::<u8>(obj, 0x13C, 3);
    v_set_is_static(obj, 0);
    wr::<u32>(obj, 0x140, rd::<u32>(obj, 0x140) & 0xFFFF_FFFE);
    wr::<u32>(obj, 0x150, g::<u32>(0xB7_CB84).read().wrapping_add(20_000));
    for k in 0..3 {
        wr::<u32>(obj, 0x44 + 4 * k, rd::<u32>(this, 0x44 + 4 * k));
    }
    let up_z = || x(rd::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x28));
    let upright = || sa_math::fpu::gt(up_z(), x(0.0));
    wr::<u32>(obj, 0x4C, if upright() { 0x3E99_999A } else { 0 });
    let t = [
        f32_of(x(rd::<f32>(this, 0x50)).fdouble()),
        f32_of(x(rd::<f32>(this, 0x54)).fdouble()),
        f32_of(x(rd::<f32>(this, 0x58)).fdouble()),
    ];
    wr::<f32>(obj, 0x50, t[0]);
    wr::<f32>(obj, 0x54, t[1]);
    wr::<f32>(obj, 0x58, t[2]);
    wr::<u32>(obj, 0x50, 0x3F00_0000);
    let bp = entity_pos(this);
    let op = entity_pos(obj);
    let mut d = [0f32; 3];
    d[2] = f32_of(x(rd::<f32>(op, 8)).fsub_m(rd::<f32>(bp, 8)));
    d[1] = f32_of(x(rd::<f32>(op, 4)).fsub_m(rd::<f32>(bp, 4)));
    d[0] = f32_of(x(rd::<f32>(op, 0)).fsub_m(rd::<f32>(bp, 0)));
    let dp = GPtr::from_raw(d.as_mut_ptr());
    vector_normalise(dp.cast());
    if upright() {
        vector_add_assign(dp.cast(), rd::<GPtr<u8>>(this, 0x14).byte_add(0x20));
    }
    let m = rd::<GPtr<u8>>(obj, 0x14);
    wr::<f32>(m, 0x30, f32_of(x(d[0]).fadd_m(rd::<f32>(m, 0x30))));
    wr::<f32>(m, 0x34, f32_of(x(d[1]).fadd_m(rd::<f32>(m, 0x34))));
    wr::<f32>(m, 0x38, f32_of(x(d[2]).fadd_m(rd::<f32>(m, 0x38))));
    world_add(obj);
    let mut atomic = [0u32; 1];
    frame_for_all_objects(rd::<GPtr<u8>>(this, 0x5B4), 0x6F_00D0, GPtr::from_raw(atomic.as_mut_ptr()).addr() as u32);
    if atomic[0] != 0 {
        wr::<u8>(GPtr::from_addr(atomic[0] as usize), 2, 0);
    }
}

/// `CHeli::GenerateHeli(CPed* target, bool newsHeli)` (0x6C6520, cdecl; FRONTIER-2 turn 3 lane B). A new `CHeli`
/// (news: model 488, else 497; created by 4). Its spot: 250 from the target at angle `(rand() & 255) * 0.02453`
/// (the cosine of the unrounded product, the sine of the stored f32); outside [-3000, 3000] on x or y the angle turns
/// by pi and the spot is taken again from the target. z = target z + 50, except within 350 (2D) of (-2322, -1653),
/// where the heli goes straight above the target at `max(target z + 200, 560)` (NaN: 560). The height is at least the
/// ground there + 20 (`FindGroundZForCoord`). Physics status (3), +0x428 bit 3; news: mission 0x29 against the player
/// ped, cruise speed 0x23, else mission 0x28, speed 0x46; target +0x41C = `target` either way; search-light / follow
/// distances +0x9A8 / +0x9B0 = 30 / 27 (news) or 20 / 12; +0x84C = 0.165; the matrix turned to `angle + pi` about z;
/// added to the world with a random driver (`SetUpDriver(-1, 0, 0)`). The free helicopter slot scan of the bytes
/// (0xC1C964, two entries) has no effect and is not repeated.
#[hook(addr = 0x6C6520, conv = cdecl, name = "CHeli::GenerateHeli", expect = "64 A1 00 00 00 00 6A FF 68 B6 84 84 00",
       unverified = "frontier2-b10", route = "L2", hazards = "heap-writes,x87-transcendental,stack-residue",
       note = "rand at_site 0x6C65D0; FSIN / FCOS through sa_math::x87; the original keeps the angle in its own argument slots (not modelled); creates a heli and its driver")]
pub fn heli_generate_heli(target: GPtr<u8>, news: u32) -> GPtr<u8> {
    use sa_core::bindings::{heli_ctor, matrix_set_translate};
    use sa_pop::bindings::set_up_driver;
    use sa_world::bindings::{find_ground_z_for_coord, vehicle_new};
    let news = news as u8 != 0;
    let mem = vehicle_new(0xA18);
    let heli = if mem.is_null() { mem } else { heli_ctor(mem, if news { 0x1E8 } else { 0x1F1 }, 4) };
    let tp = entity_pos(target);
    let mut pos = [rd::<f32>(tp, 0), rd::<f32>(tp, 4), rd::<f32>(tp, 8)];
    let r = sa_core::orig::at_site(0x6C_65D0, sa_core::crt::rand) & 0xFF;
    let prod = xi(r).fmul_m(f32::from_bits(0x3CC8_F5C3));
    let mut angle = f32_of(prod);
    pos[0] = f32_of(prod.fcos().fmul_m(250.0).fadd_m(pos[0]));
    pos[1] = f32_of(x(angle).fsin().fmul_m(250.0).fadd_m(pos[1]));
    let lo = x(-3000.0);
    let hi = x(3000.0);
    let out = |v: f32| !sa_math::fpu::ge_or_nan(x(v), lo) || sa_math::fpu::gt(x(v), hi);
    if out(pos[0]) || out(pos[1]) {
        angle = f32_of(x(angle).fadd_m(f32::from_bits(0x4049_0FDB)));
        let tp = entity_pos(target);
        pos = [rd::<f32>(tp, 0), rd::<f32>(tp, 4), rd::<f32>(tp, 8)];
        pos[0] = f32_of(x(angle).fcos().fmul_m(250.0).fadd_m(pos[0]));
        pos[1] = f32_of(x(angle).fsin().fmul_m(250.0).fadd_m(pos[1]));
    }
    let mut z = f32_of(x(pos[2]).fadd_m(50.0));
    let dx = x(pos[0]).fsub_m(-2322.0);
    let dy = x(pos[1]).fsub_m(-1653.0);
    let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
    let (px, py) = if sa_math::fpu::ge_or_nan(d, x(350.0)) {
        (pos[0], pos[1])
    } else {
        let tp = entity_pos(target);
        let up = x(rd::<f32>(tp, 8)).fadd_m(200.0);
        z = if sa_math::fpu::gt(up, x(560.0)) { f32_of(up) } else { 560.0 };
        (rd::<f32>(tp, 0), rd::<f32>(tp, 4))
    };
    let ground = sa_math::fpu::st0(find_ground_z_for_coord(px, py)).fadd_m(20.0);
    let zf = if sa_math::fpu::le_or_nan(x(z), ground) { f32_of(ground) } else { z };
    matrix_set_translate(rd::<GPtr<u8>>(heli, 0x14), px, py, zf);
    wr::<u8>(heli, 0x428, rd::<u8>(heli, 0x428) | 8);
    wr::<u8>(heli, 0x36, (rd::<u8>(heli, 0x36) & 7) | 0x18);
    if news {
        wr::<u8>(heli, 0x3BA, 0x29);
        wr::<GPtr<u8>>(heli, 0x41C, find_player_ped(-1));
        wr::<u8>(heli, 0x3D0, 0x23);
    } else {
        wr::<u8>(heli, 0x3BA, 0x28);
        wr::<u8>(heli, 0x3D0, 0x46);
    }
    wr::<GPtr<u8>>(heli, 0x41C, target);
    wr::<u32>(heli, 0x9A8, if news { 0x41F0_0000 } else { 0x41A0_0000 });
    wr::<u32>(heli, 0x9B0, if news { 0x41D8_0000 } else { 0x4140_0000 });
    let a2 = f32_of(x(angle).fadd_m(f32::from_bits(0x4049_0FDB)));
    wr::<u32>(heli, 0x84C, 0x3E28_F5C3);
    let m = rd::<GPtr<u8>>(heli, 0x14);
    for o in [8, 0x18, 0x20, 0x24] {
        wr::<u32>(m, o, 0);
    }
    wr::<u32>(m, 0x28, 0x3F80_0000);
    let sin = x(a2).fsin();
    let cos = x(a2).fcos();
    wr::<f32>(m, 0, f32_of(sin));
    wr::<f32>(m, 4, f32_of(cos.fchs()));
    wr::<f32>(m, 0x10, f32_of(cos));
    wr::<f32>(m, 0x14, f32_of(sin));
    world_add(heli);
    set_up_driver(heli, -1, 0, 0);
    heli
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PLANE_BLOW_UP_CAR, HOOK_HELI_BLOW_UP_CAR, HOOK_BOAT_BLOW_UP_CAR, HOOK_HELI_GENERATE_HELI];
