//! WRITER-WORLD-2 (batch `world6-1`, SA:MP-executed rows): `CWorld::TriggerExplosionSectorList`, the
//! per-entity part of an explosion (SA:MP's `CreateExplosion` and every rocket / car blast reach it
//! through `CWorld::TriggerExplosion` 0x56B790). Ported from the 1.0 US bytes; gta-reversed as a
//! reading aid. Where they differ the bytes win, notably:
//!
//! - the vehicle response compares `GetSpeed(point) . normal * m_fMass` (gta-reversed drops the mass)
//!   and stores the force factor into the same stack slot as the damage fraction;
//! - the collision normal is the matrix times the *negated* `CColPoint` normal, the plane's
//!   `VehicleDamage` gets the negated normal too;
//! - the ped push-away uses `CTimer::ms_fTimeStep * mass * 0.008` (not the step in ms);
//! - peds get `GetLocalDirection(-impact)`, and the crime report follows both ped damage paths;
//! - the ATM branch (`MI_ATM` hit by the player) subtracts twice the stack slot of the damage
//!   fraction *before* this entity has set it: the value of the previous entity of the list, or an
//!   uninitialised stack word for the first one (modelled as a slot that persists across entities,
//!   starting at 0.0);
//! - a plane missed by `ProcessSphereBox` still gets `VehicleDamage` at its position with the negated
//!   normal of the uninitialised `CColPoint` (the buffer persists across entities here too).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::common::{ftol32, g, rd, wr, x};

use crate::bindings::damage_calc_compute_damage_response;
use crate::bindings::damage_calc_dtor;
use crate::bindings::entity_get_col_model as get_col_model;
use crate::bindings::event_damage_affects_ped;
use crate::bindings::event_damage_ctor;
use crate::bindings::event_damage_dtor;
use crate::bindings::event_group_add;
use crate::bindings::get_local_direction;
use crate::bindings::kob_ctor;
use crate::bindings::kob_dtor;
use crate::bindings::max_f;
use crate::bindings::min_f;
use crate::bindings::multiply3x3 as multiply3x3_mv;
use crate::bindings::multiply3x3_vm;
use crate::bindings::object_damage;
use crate::bindings::physical_add_to_moving_list as add_to_moving_list;
use crate::bindings::report_crime;
use crate::bindings::switch_vehicle_to_real_physics;
use crate::bindings::try_to_explode;
use crate::bindings::vec_mul_assign;
use crate::bindings::vehicle_inflict_damage;
use sa_core::bindings::col_box_set;
use sa_core::bindings::find_player_ped;
use sa_stream::bindings::is_object_glass;
orig! {
    /// `CGlass::WindowRespondsToExplosion(CEntity*, CVector point)` (0x71C1A0)
    fn window_responds_to_explosion(e: GPtr<u8>, point: CVector) = cdecl 0x71C1A0 => unported;
    /// `CEntity::SetIsStatic(bool)` (vtable +0x10)
    fn v_set_is_static(e: GPtr<u8>, s: u32) = thiscall vslot 0x10 => unported;
    /// `Fx_c::TriggerWaterHydrant(CVector&)` (0x4A0D70) on `g_fx` (0xA9AE00)
    fn trigger_water_hydrant(fx: GPtr<u8>, pos: GPtr<u8>) = thiscall 0x4A0D70 => unported;
    /// `CWeapon::GenerateDamageEvent(CPed*, CEntity*, eWeaponType, int32, ePedPieceTypes, int32)` (0x73A530)
    fn generate_damage_event(victim: GPtr<u8>, creator: GPtr<u8>, ty: i32, damage: i32, piece: i32, dir: u32) -> u8 = cdecl 0x73A530 => unported;
    /// `CColSphere::Set(float radius, const CVector& centre, uint8 material, uint8 flags, uint8 lighting)` (0x40FD10)
    fn col_sphere_set(s: GPtr<u8>, radius: f32, center: GPtr<u8>, mat: u32, flags: u32, light: u32) = thiscall 0x40FD10 => unported;
    /// `CCollision::ProcessSphereBox(const CColSphere&, const CColBox&, CColPoint&, float& maxTouchDist)` (0x412130)
    fn process_sphere_box(sp: GPtr<u8>, bx: GPtr<u8>, cp: GPtr<u8>, max_dist: GPtr<u8>) -> u8 = cdecl 0x412130 => unported;
    /// `CPhysical::GetSpeed(CVector point)` (0x542CE0): the result through the hidden pointer
    fn get_speed(e: GPtr<u8>, out: GPtr<CVector>, point: CVector) -> GPtr<CVector> = thiscall 0x542CE0 => crate::physical::forces::get_speed;
    /// `CPhysical::ApplyForce(CVector force, CVector point, bool updateTurnSpeed)` (0x542B50)
    fn apply_force(e: GPtr<u8>, force: CVector, point: CVector, update_turn: u8) = thiscall 0x542B50 => crate::physical::forces::apply_force;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    fn apply_move_force(e: GPtr<u8>, force: CVector) = thiscall 0x5429F0 => crate::physical::forces::apply_move_force;
    /// `CPhysical::ApplyTurnForce(CVector force, CVector point)` (0x542A50)
    fn apply_turn_force(e: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => crate::physical::forces::apply_turn_force;
    /// `CVehicle::VehicleDamage(float, eVehicleCollisionComponent, CEntity*, CVector*, CVector*, eWeaponType)` (vtable +0xE0)
    fn v_vehicle_damage(v: GPtr<u8>, dmg: u32, comp: u32, e: GPtr<u8>, p: GPtr<u8>, d: GPtr<u8>, weapon: u32) = thiscall vslot 0xE0 => unported;
    /// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(CEntity*, float, eWeaponType, ePedPieceTypes, bool)` (0x4AD3F0)
    fn damage_calc_ctor(c: GPtr<u8>, e: GPtr<u8>, dmg: f32, wt: u32, piece: u32, speak: u32) -> GPtr<u8> = thiscall 0x4AD3F0 => unported;
}

/// `MI_FENCE` / `MI_FENCE2` (0x8CD600 / 0x8CD604), `MI_FIRE_HYDRANT` (0x8CD594), `MI_ATM` (0x8CD710).
const MI_FENCE: usize = 0x8C_D600;
const MI_FENCE2: usize = 0x8C_D604;
const MI_FIRE_HYDRANT: usize = 0x8C_D594;
const MI_ATM: usize = 0x8C_D710;
/// `CTimer::ms_fTimeStep`, `CTimer::m_snTimeInMilliseconds`.
const TIME_STEP: usize = 0xB7_CB5C;
const TIME_MS: usize = 0xB7_CB84;
/// `1 / 1400` (0x864F98).
const INV_1400: f32 = f32::from_bits(0x3A3B_3EE7);
/// `0.008` (0x863984).
const K_0_008: f32 = f32::from_bits(0x3C03_126F);
/// `0.2` (0x858CC4).
const F_0_2: f32 = f32::from_bits(0x3E4C_CCCD);
/// `0.1` (0x858B1C), `0.01` (0x858C58).
const F_0_1: f32 = f32::from_bits(0x3DCC_CCCD);
const F_0_01: f32 = f32::from_bits(0x3C23_D70A);

use sa_core::field::ptr_of as p;

use crate::world::common::entity_pos as ent_pos;

/// `fld; fchs; fstp` of an f32.
fn neg(v: f32) -> f32 {
    x(v).fchs().to_f32()
}

/// `GetIsStatic()`: +0x1C bit 2 (`bIsStatic`) or bit 18 (`bIsStaticWaitingForCollision`).
fn is_static(e: GPtr<u8>) -> bool {
    let f = rd::<u32>(e, 0x1C);
    f & 4 != 0 || f & 0x4_0000 != 0
}

/// `min(1, (radius - dist) / radius * 2)` of the static-object path (`fdiv`; NaN gives 1).
fn static_fraction(radius: f32, dist: f32) -> f32 {
    let r = x(radius).fsub_m(dist).fdiv_m(radius);
    let r = r.fadd(r);
    if r.fcom(x(1.0)).test_parity(0x05) { 1.0 } else { r.to_f32() }
}

/// The two ped damage events: a `CPedDamageResponseCalculator(creator, amount, WEAPON_EXPLOSION,
/// PED_PIECE_TORSO, false)` and a `CEventDamage(creator, now, WEAPON_EXPLOSION, PED_PIECE_TORSO,
/// dir, false, +0x46C bit 8 (`bIsTalking`, pushed as `(flags >> 8) & 0xFFFFFF01`))`; the response is
/// computed when the event affects the ped (else marked calculated), the event goes to the ped's
/// event group, then both are destroyed (event first).
fn ped_damage_events(ped: GPtr<u8>, creator: GPtr<u8>, amount: f32, dir: i32) {
    let mut calc = [0u32; 8];
    let mut ev = [0u32; 0x20];
    damage_calc_ctor(p(&mut calc), creator, amount, 0x33, 3, 0);
    let talk = (rd::<u32>(ped, 0x46C) >> 8) & 0xFFFF_FF01;
    event_damage_ctor(p(&mut ev), creator, g::<u32>(TIME_MS).read(), 0x33, 3, dir as u32, 0, talk);
    if event_damage_affects_ped(p(&mut ev), ped) != 0 {
        damage_calc_compute_damage_response(p(&mut calc), ped, p(&mut ev).byte_add(0x38), 1);
    } else {
        wr::<u8>(p(&mut ev), 0x42, 1);
    }
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), p(&mut ev), 0);
    event_damage_dtor(p(&mut ev));
    damage_calc_dtor(p(&mut calc));
}

/// `CWorld::TriggerExplosionSectorList(CPtrListDoubleLink& list, const CVector& point, float
/// radius, float visibleDistance, CEntity* victim, CEntity* creator, bool processVehicleBombTimer,
/// float damage)` (0x567750). For every entity of the list closer than `radius` (`fsqrt` distance,
/// compared unrounded):
///
/// 1. objects `TryToExplode`; explosion-proof entities (+0x40 bit 23) and peds in a vehicle (+0x46C
///    bit 8... bit 0x100) are skipped;
/// 2. static entities: non-objects that use collision become dynamic (`SetIsStatic(false)`,
///    `AddToMovingList`); objects without `bDisableTurnForce` (+0x40 bit 2) that are uprooted
///    (`visibleDistance > info->uprootLimit`, or a fence model) are either glass
///    (`WindowRespondsToExplosion`) or made dynamic, a fire hydrant not yet exploded triggers its
///    water and is flagged exploded (+0x140 bit 6), other objects are flagged unless the object info
///    causes explosions; then the ATM quirk (module docs); still static: `ObjectDamage(fraction *
///    300)` with the static fraction; static or collision-less entities stop here;
/// 3. `fraction = min(1, (radius - dist) * (1/radius) * 2)`, `factor = mass / 1400 * fraction *
///    visibleDistance`, `impact = dir / max(0.01, dist)` with `z` clamped at 0;
/// 4. vehicles: BMX riders get a knock-off event and a damage event (`fraction * damage * 1100`), a
///    `STATUS_SIMPLE` vehicle switches to real physics, `InflictDamage(fraction * damage * 1100)`,
///    the bomb timer becomes `t / 10 + 1`; the sphere (`-dir` into the vehicle's space, radius
///    `dist`) against the model's bounding box gives a contact: the normal (rotated, negated, `z`
///    pushed up), the contact's fraction times 0.2 for the victim times `min(1, mass / 3000)` times
///    `visibleDistance`, reduced when the contact already moves faster, applied with `ApplyForce`
///    unless `bDisableTurnForce`; planes always take `VehicleDamage(1000)`;
/// 5. peds: on a wrecked vehicle a 1000 damage event; otherwise unattached peds are pushed
///    (`min(factor, mass / 4)`, reduced by the current speed, +4 up when standing else
///    `timeStep * mass * 0.008`) and get a `fraction * 250` damage event; a ped creator reports a
///    crime (`CRIME_DAMAGE_COP_CAR` 5 for cops, else 4);
/// 6. objects without +0x40 bits 2 / 7: `z` at least 0.2 (when below 0.1), `factor` reduced by the
///    current speed, move force, turn force scaled by `min(1, turnMass / mass)` at half the bound
///    sphere's radius up; then `ObjectDamage(fraction * 300)`.
#[hook(addr = 0x567750, conv = cdecl, name = "CWorld::TriggerExplosionSectorList<CPtrListDoubleLink<CPhysical*>>",
       expect = "64 A1 00 00 00 00",
       unverified = "world6-1", route = "L2", hazards = "x87-transcendental,uninit-locals,heap-writes",
       recs = "weapon-combat,police-gang",
       note = "events on the stack (knock-off, damage); the ATM path and the plane miss path read stack words this call may not have written (modelled as 0.0 / zero CColPoint on the first entity)")]
#[allow(clippy::too_many_arguments)]
pub fn trigger_explosion_sector_list(list: GPtr<u8>, point: GPtr<u8>, radius: f32, vis: f32, victim: GPtr<u8>,
                                     creator: GPtr<u8>, bomb: u32, damage: f32) {
    let (px, py, pz) = (rd::<f32>(point, 0), rd::<f32>(point, 4), rd::<f32>(point, 8));
    // stack slots that keep their value from one entity to the next
    let mut rel: f32 = 0.0; // [esp+0xC]
    let mut cp = [0u32; 11]; // CColPoint [esp+0xAC]
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        let pos = ent_pos(e);
        let dz = x(rd::<f32>(pos, 8)).fsub_m(pz).to_f32();
        let dy = x(rd::<f32>(pos, 4)).fsub_m(py).to_f32();
        let dxe = x(rd::<f32>(pos, 0)).fsub_m(px);
        let dx = dxe.to_f32();
        let len = dxe.fmul_m(dx).fadd(x(dy).fmul_m(dy)).fadd(x(dz).fmul_m(dz)).fsqrt();
        let dist = len.to_f32();
        if len.fcom(x(radius)).test_parity(0x05) {
            continue;
        }
        if rd::<u8>(e, 0x36) & 7 == 4 {
            try_to_explode(e);
        }
        let f40 = rd::<u32>(e, 0x40);
        if (f40 >> 23) & 1 != 0 {
            continue;
        }
        let ty = rd::<u8>(e, 0x36) & 7;
        if ty == 3 && rd::<u32>(e, 0x46C) & 0x100 != 0 {
            continue;
        }
        if is_static(e) {
            if ty != 4 {
                if rd::<u8>(e, 0x1C) & 1 != 0 {
                    v_set_is_static(e, 0);
                    add_to_moving_list(e);
                }
            } else {
                if ty & (f40 as u8) == 0 {
                    let info = rd::<GPtr<u8>>(e, 0x160);
                    let mi = rd::<i16>(e, 0x22) as i32;
                    let uproot = !x(vis).fcom(x(rd::<f32>(info, 0x14))).test_any(0x41)
                        || mi == g::<u16>(MI_FENCE).read() as i32
                        || mi == g::<u16>(MI_FENCE2).read() as i32;
                    if uproot {
                        if is_object_glass(e) != 0 {
                            window_responds_to_explosion(e, CVector::new(px, py, pz));
                        } else {
                            v_set_is_static(e, 0);
                            add_to_moving_list(e);
                            let mi = rd::<i16>(e, 0x22) as i32;
                            if mi == g::<u16>(MI_FIRE_HYDRANT).read() as i32 && rd::<u8>(e, 0x140) & 0x40 == 0 {
                                trigger_water_hydrant(g(0xA9_AE00), ent_pos(e));
                                wr::<u32>(e, 0x140, rd::<u32>(e, 0x140) | 0x40);
                            } else if rd::<u8>(e, 0x36) & 7 == 4 && rd::<u8>(rd::<GPtr<u8>>(e, 0x160), 0x1F) == 0 {
                                wr::<u32>(e, 0x140, rd::<u32>(e, 0x140) | 0x40);
                            }
                        }
                    }
                }
                let mi = rd::<i16>(e, 0x22) as i32;
                if mi == g::<u16>(MI_ATM).read() as i32 && creator == find_player_ped(-1) {
                    let r = x(rel);
                    wr::<f32>(e, 0x154, r.fadd(r).fsubr_m(rd::<f32>(e, 0x154)).to_f32());
                }
                if is_static(e) {
                    rel = static_fraction(radius, dist);
                    object_damage(e, x(rel).fmul_m(300.0).to_f32(), GPtr::NULL, GPtr::NULL, creator, 0x33);
                }
            }
        }
        if is_static(e) || rd::<u8>(e, 0x1C) & 1 == 0 {
            continue;
        }
        let inv = x(1.0).fdiv_m(radius).to_f32();
        let r = x(radius).fsub_m(dist).fmul_m(inv);
        let r = r.fadd(r);
        rel = if r.fcom(x(1.0)).test_parity(0x05) { 1.0 } else { r.to_f32() };
        let mut factor = x(rd::<f32>(e, 0x8C)).fmul_m(INV_1400).fmul_m(rel).fmul_m(vis).to_f32();
        let d = if x(F_0_01).fcom(x(dist)).test_any(0x41) { x(dist) } else { x(F_0_01) };
        let id = d.fdivr_m(1.0);
        let ze = x(dz).fmul(id);
        let iy = id.fmul_m(dy).to_f32(); // [esp+0x3C], copied to [esp+0x14]
        let ix = id.fmul_m(dx).to_f32();
        let iz = if ze.fcom(x(0.0)).test_parity(0x05) { ze.to_f32() } else { 0.0 };
        let mut imp = [ix, iy, iz]; // [esp+0x10]
        match rd::<u8>(e, 0x36) & 7 {
            2 => {
                if rd::<u32>(e, 0x594) == 10 && !rd::<GPtr<u8>>(e, 0x460).is_null() {
                    let mut ev = [0u32; 0x20];
                    kob_ctor(p(&mut ev), e, e.byte_add(0x44), p(&mut imp), 0, 0, 0x33, 0, 0, 0, 1, 0);
                    let drv = rd::<GPtr<u8>>(e, 0x460);
                    event_group_add(rd::<GPtr<u8>>(drv, 0x47C).byte_add(0x68), p(&mut ev), 0);
                    let mut v2 = [imp[0], imp[1]];
                    let dir = get_local_direction(rd::<GPtr<u8>>(e, 0x460), p(&mut v2));
                    let dmg = ftol32(x(rel).fmul_m(damage).fmul_m(1100.0));
                    generate_damage_event(rd::<GPtr<u8>>(e, 0x460), e, 0x33, dmg, 3, dir as u32);
                    kob_dtor(p(&mut ev));
                }
                if rd::<u8>(e, 0x36) & 0xF8 == 0x10 {
                    switch_vehicle_to_real_physics(e);
                }
                vehicle_inflict_damage(e, creator, 0x33, x(rel).fmul_m(damage).fmul_m(1100.0).to_f32(), 0.0, 0.0, 0.0);
                if bomb as u8 != 0 {
                    let t = rd::<u16>(e, 0x4DE);
                    if t != 0 {
                        wr::<u16>(e, 0x4DE, t / 10 + 1);
                    }
                }
                let mut ndir = [neg(dx), neg(dy), neg(dz)];
                let mut centre = [0f32; 3];
                multiply3x3_vm(p(&mut centre), p(&mut ndir), rd::<GPtr<u8>>(e, 0x14));
                let mut sphere = [0u32; 5];
                col_sphere_set(p(&mut sphere), dist, p(&mut centre), 0, 0, 0xFF);
                let col_max = get_col_model(e);
                let col_min = get_col_model(e);
                let mut bx = [0u32; 7];
                col_box_set(p(&mut bx), col_min, col_max.byte_add(0xC), 0, 0, 0xFF);
                let mut max_dist: f32 = 100_000.0;
                let mut cpos = [0f32; 3]; // [esp+0x1C]
                if process_sphere_box(p(&mut sphere), p(&mut bx), p(&mut cp), p(&mut max_dist)) != 0 {
                    let mut out = [0f32; 3];
                    multiply3x3_mv(p(&mut out), rd::<GPtr<u8>>(e, 0x14), p(&mut cp));
                    cpos = out;
                    let cn = |k: usize| f32::from_bits(cp[4 + k]);
                    let mut nn = [neg(cn(0)), neg(cn(1)), neg(cn(2))];
                    multiply3x3_mv(p(&mut out), rd::<GPtr<u8>>(e, 0x14), p(&mut nn));
                    imp = out; // the normal reuses the impact slot
                    if !x(imp[2]).fcom(x(f32::from_bits(0xBE4C_CCCD))).test_parity(0x05) {
                        imp[2] = f32::from_bits(0xBE4C_CCCD);
                    } else if !x(imp[2]).fcom(x(F_0_2)).test_parity(0x05)
                        && !x(imp[2]).fcom(x(0.0)).test_any(0x41)
                    {
                        imp[2] = x(imp[2]).fadd_m(F_0_2).to_f32();
                    }
                    let vp = ent_pos(e);
                    let ax = x(cpos[0]).fadd_m(rd::<f32>(vp, 0));
                    let ay = x(cpos[1]).fadd_m(rd::<f32>(vp, 4));
                    let az = x(cpos[2]).fadd_m(rd::<f32>(vp, 8)).to_f32();
                    let ddx = ax.fsub_m(px).to_f32();
                    let ddy = ay.fsub_m(py);
                    let ddz = x(az).fsub_m(pz);
                    let l = ddz.fmul(ddz).fadd(ddy.fmul(ddy)).fadd(x(ddx).fmul_m(ddx)).fsqrt();
                    let f = l.fsubr_m(radius).fmul_m(inv);
                    let f = f.fadd(f);
                    let mut f = if f.fcom(x(1.0)).test_parity(0x05) { x(1.0) } else { f };
                    if e == victim {
                        f = f.fmul_m(g::<f32>(0x8C_DF10).read());
                    }
                    let mut mm = x(rd::<f32>(e, 0x8C)).fdiv_m(g::<f32>(0x8C_DF0C).read());
                    if !x(1.0).fcom(mm).test_parity(0x05) {
                        mm = x(1.0);
                    }
                    rel = mm.fmul(f).fmul_m(vis).to_f32();
                    let mut sp = CVector::new(0.0, 0.0, 0.0);
                    let s = get_speed(e, GPtr::from_raw(&mut sp as *mut CVector), CVector::new(cpos[0], cpos[1], cpos[2]));
                    let sb: GPtr<u8> = s.cast();
                    let dot = x(imp[1]).fmul_m(rd::<f32>(sb, 4))
                        .fadd(x(imp[2]).fmul_m(rd::<f32>(sb, 8)))
                        .fadd(x(imp[0]).fmul_m(rd::<f32>(sb, 0)))
                        .fmul_m(rd::<f32>(e, 0x8C));
                    if !dot.fcom(x(rel).fmul_m(3.0)).test_any(0x41) {
                        let v = dot.fsubr_m(rel);
                        rel = if x(0.0).fcom(v).test_any(0x41) { v.to_f32() } else { 0.0 };
                    }
                    for c in imp.iter_mut() {
                        *c = x(*c).fmul_m(rel).to_f32();
                    }
                    if rd::<u8>(e, 0x40) & 4 == 0 {
                        apply_force(e, CVector::new(imp[0], imp[1], imp[2]), CVector::new(cpos[0], cpos[1], cpos[2]), 1);
                    }
                }
                if rd::<u32>(e, 0x594) == 4 {
                    let vp = ent_pos(e);
                    for (k, c) in cpos.iter_mut().enumerate() {
                        *c = x(*c).fadd_m(rd::<f32>(vp, 4 * k as isize)).to_f32();
                    }
                    let cn = |k: usize| f32::from_bits(cp[4 + k]);
                    let mut nn = [neg(cn(0)), neg(cn(1)), neg(cn(2))];
                    v_vehicle_damage(e, 1000f32.to_bits(), 0, creator, p(&mut cpos), p(&mut nn), 0x33);
                }
            }
            3 => {
                let mut v2 = [neg(imp[0]), neg(iy)];
                let dir = get_local_direction(e, p(&mut v2));
                let att = rd::<GPtr<u8>>(e, 0xFC);
                let wrecked = !att.is_null() && rd::<u8>(att, 0x36) & 7 == 2 && rd::<u8>(att, 0x36) & 0xF8 == 0x28;
                if wrecked {
                    ped_damage_events(e, creator, 1000.0, dir);
                } else {
                    if att.is_null() {
                        let mass = rd::<f32>(e, 0x8C);
                        let m4 = x(mass).fmul_m(0.25).to_f32();
                        let mut f = if x(factor).fcom(x(m4)).test_any(0x41) { x(factor) } else { x(m4) };
                        let dot = x(imp[1]).fmul_m(rd::<f32>(e, 0x48))
                            .fadd(x(imp[2]).fmul_m(rd::<f32>(e, 0x4C)))
                            .fadd(x(imp[0]).fmul_m(rd::<f32>(e, 0x44)))
                            .fmul_m(mass);
                        if !dot.fcom(f.fadd(f)).test_any(0x41) {
                            f = X87::from_f64_exact(max_f(0.0, f.fsub(dot).to_f32()));
                        }
                        imp[0] = x(imp[0]).fmul(f).to_f32();
                        imp[1] = x(imp[1]).fmul(f).to_f32();
                        let zf = f.fmul_m(imp[2]);
                        if rd::<u8>(e, 0x46C) & 1 != 0 && rd::<u8>(e, 0x1C) & 1 != 0 {
                            let z = zf.fadd_m(4.0);
                            wr::<u32>(e, 0x46C, rd::<u32>(e, 0x46C) & !1);
                            imp[2] = z.to_f32();
                        } else {
                            imp[2] = x(g::<f32>(TIME_STEP).read())
                                .fmul_m(rd::<f32>(e, 0x8C))
                                .fmul_m(K_0_008)
                                .fadd(zf)
                                .to_f32();
                        }
                        apply_move_force(e, CVector::new(imp[0], imp[1], imp[2]));
                    }
                    ped_damage_events(e, creator, x(rel).fmul_m(250.0).to_f32(), dir);
                }
                if !creator.is_null() && rd::<u8>(creator, 0x36) & 7 == 3 {
                    let crime = if rd::<u32>(e, 0x598) == 6 { 5 } else { 4 };
                    report_crime(crime, e, creator);
                }
            }
            4 => {
                if rd::<u8>(e, 0x40) & 0x84 == 0 {
                    if !x(imp[2]).fcom(x(F_0_1)).test_parity(0x05) {
                        imp[2] = F_0_2;
                    }
                    let mass = rd::<f32>(e, 0x8C);
                    let dot = x(imp[2]).fmul_m(rd::<f32>(e, 0x4C))
                        .fadd(x(iy).fmul_m(rd::<f32>(e, 0x48)))
                        .fadd(x(imp[0]).fmul_m(rd::<f32>(e, 0x44)))
                        .fmul_m(mass);
                    if !dot.fcom(x(factor).fmul_m(4.0)).test_any(0x41) {
                        factor = X87::from_f64_exact(max_f(0.0, x(factor).fsub(dot).to_f32())).to_f32();
                    }
                    vec_mul_assign(p(&mut imp), factor);
                    apply_move_force(e, CVector::new(imp[0], imp[1], imp[2]));
                    let mi = rd::<i16>(e, 0x22) as isize;
                    let info = g::<GPtr<u8>>((0xA9_B0C8 + mi * 4) as usize).read();
                    let col = rd::<GPtr<u8>>(info, 0x14);
                    let half = x(rd::<f32>(col, 0x24)).fmul_m(0.5).to_f32();
                    let ratio = x(rd::<f32>(e, 0x90)).fdiv_m(rd::<f32>(e, 0x8C)).to_f32();
                    let k = X87::from_f64_exact(min_f(1.0, ratio)).to_f32();
                    vec_mul_assign(p(&mut imp), k);
                    apply_turn_force(e, CVector::new(imp[0], imp[1], imp[2]), CVector::new(0.0, 0.0, half));
                }
                object_damage(e, x(rel).fmul_m(300.0).to_f32(), GPtr::NULL, GPtr::NULL, creator, 0x33);
            }
            _ => {}
        }
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_TRIGGER_EXPLOSION_SECTOR_LIST];
