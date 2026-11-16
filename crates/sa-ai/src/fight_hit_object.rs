//! WRITER-NATIVE (batch `native-34`): 0x61D400, `CTaskSimpleFight`'s hit on an object (unnamed), reached when a
//! punch lands on street furniture, and its vehicle twin 0x61D0B0 (FRONTIER-2 lane C, batch `frontier2-c13`). Both
//! share the audio / punch-impact tail. Ported from the 1.0 US bytes.

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

use sa_world::bindings::object_damage;
use sa_world::bindings::ped_weapon_audio_add_audio_event as ped_weapon_audio_event;
use sa_world::bindings::physical_add_to_moving_list;
use crate::bindings::fight_strike_damage_c;
use sa_world::bindings::apply_force as physical_apply_force;
use sa_world::bindings::ped_audio_add_audio_event as ped_audio_event;
use crate::bindings::fx_add_punch_impact;
use sa_core::bindings::{get_anim_hierarchy_from_skin_clump, rp_hanim_hierarchy_get_matrix_array,
                        rp_hanim_id_get_index};
use sa_world::bindings::{event_vehicle_damage_dtor, fx_add_sparks, report_crime};
orig! {
    /// `CEntity::SetIsStatic(bool)` (vtable slot 0x10)
    fn entity_set_is_static(e: GPtr<u8>, b: u32) = thiscall vslot 0x10 => unported;
    /// `CVehicle::VehicleDamage(float, uint16 component, CEntity* damager, CVector* pos, CVector* dir, eWeaponType)`
    /// (vtable slot 0xE0)
    fn v_vehicle_damage(v: GPtr<u8>, dmg: u32, comp: u32, e: GPtr<u8>, p: GPtr<u8>, d: GPtr<u8>, weapon: u32) = thiscall vslot 0xE0 => unported;
    /// `CEventVehicleDamage::CEventVehicleDamage(CVehicle*, CEntity* attacker, eWeaponType)` (0x4B18D0)
    fn event_vehicle_damage_ctor(e: GPtr<u8>, veh: GPtr<u8>, attacker: GPtr<u8>, weapon: u32) -> GPtr<u8> = thiscall 0x4B18D0 => crate::event_vehicle_damage_ctor::event_vehicle_damage_ctor;
}

/// `g_fx` (Fx_c, 0xA9AE00).
const G_FX: usize = 0xA9_AE00;

/// The weapon type of the ped's current slot (`[ped + 0x5A0 + (int8)[ped + 0x718] * 0x1C]`).
fn current_weapon(ped: GPtr<u8>) -> u32 {
    let slot = i32::from(rd::<i8>(ped, 0x718));
    rd::<u32>(ped, (0x5A0 + slot * 0x1C) as isize)
}

/// The tail both hits share: the chainsaw (weapon 9) audio event 0x9B, the move's hit sound (`[0xC16F0C + (+0x24 *
/// 0x22 + +0x25) * 4]`) on the ped audio with the hit entity and the surface, and a punch impact at `pos` along
/// `normal`.
fn hit_tail(this: GPtr<u8>, ped: GPtr<u8>, hit: GPtr<u8>, surface: u32, pos: GPtr<u8>, normal: GPtr<u8>) {
    if current_weapon(ped) == 9 {
        ped_weapon_audio_event(ped.byte_add(0x394), 0x9B);
    }
    let idx = i32::from(rd::<i8>(this, 0x24)) * 0x22 + i32::from(rd::<i8>(this, 0x25));
    let sfx = g::<u32>((0xC1_6F0Cu32).wrapping_add((idx as u32).wrapping_mul(4)) as usize).read();
    ped_audio_event(ped.byte_add(0x138), sfx, 0.0, 1.0, hit, surface, 0, 0);
    fx_add_punch_impact(GPtr::from_addr(G_FX), pos, normal, 4);
}

/// `CTaskSimpleFight::<hit object>(CPed* ped, CObject* obj, CVector* pos, CVector* normal, int16, uint8 surface)`
/// (0x61D400, `ret 0x18`).
///
/// `dmg` = the strike damage. An object with health byte +0x144 below 200, not flagged +0x40 bit 2 and with
/// mass-like info +0x160 +0x18 below [0x86D6B0] (ordered): a fixed one (+0x1C bit 2 / 18) whose info +0x14 is not
/// positive is made dynamic and added to the moving list; a free one gets `ApplyForce(normal * k, pos -
/// position, true)` with `k` = -0.1 for +0x40 bit 7, else -0.5. Then `ObjectDamage(dmg * 10, pos, &normal, ped,
/// weapon)`, the chainsaw (weapon 9) audio event 0x9B, the move's hit sound (`[0xC16F0C + (+0x24 * 0x22 +
/// +0x25) * 4]`) on the ped audio with the surface, and a punch impact. The original keeps `dmg` in the normal
/// argument slot.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x61D400, conv = thiscall, name = "FUN_0061d400",
       expect = "83 EC 24 8B 44 24 34",
       unverified = "native-34", route = "L2", hazards = "heap-writes,stack-residue", recs = "weapon-combat",
       note = "the damage lives in the normal argument slot")]
pub fn fight_hit_object(this: GPtr<u8>, ped: GPtr<u8>, obj: GPtr<u8>, pos: GPtr<u8>, normal: GPtr<u8>, _piece: u32, surface: u32) {
    let mut n = [rd::<u32>(normal, 0), rd::<u32>(normal, 4), rd::<u32>(normal, 8)];
    let np: GPtr<u8> = GPtr::from_raw(n.as_mut_ptr()).cast();
    let dmg = X87::from_f64_exact(fight_strike_damage_c(this, ped).to_f64()).to_f32();
    if rd::<u8>(obj, 0x144) < 0xC8
        && rd::<u8>(obj, 0x40) & 4 == 0
        && !x(rd::<f32>(rd::<GPtr<u8>>(obj, 0x160), 0x18)).fcom(x(f32::from_bits(0x42C7_CCCD))).test_parity(0x05)
    {
        let fl = rd::<u32>(obj, 0x1C);
        if (fl & 4 != 0 || fl & 0x40000 != 0) && !x(rd::<f32>(rd::<GPtr<u8>>(obj, 0x160), 0x14)).fcom(x(0.0)).test_parity(0x41) {
            entity_set_is_static(obj, 0);
            physical_add_to_moving_list(obj);
        }
        let fl = rd::<u32>(obj, 0x1C);
        if fl & 4 == 0 && fl & 0x40000 == 0 {
            let k = if rd::<i8>(obj, 0x40) < 0 { f32::from_bits(0xBDCC_CCCD) } else { -0.5 };
            let m = rd::<GPtr<u8>>(obj, 0x14);
            let op = if m.is_null() { obj.byte_add(4) } else { m.byte_add(0x30) };
            let d = |o: isize| x(rd::<f32>(pos, o)).fsub_m(rd::<f32>(op, o)).to_f32().to_bits();
            let (dx, dy, dz) = (d(0), d(4), d(8));
            let f = |o: isize| x(rd::<f32>(np, o)).fmul_m(k).to_f32().to_bits();
            physical_apply_force(obj, sa_math::vector::CVector::new(f32::from_bits(f(0)), f32::from_bits(f(4)), f32::from_bits(f(8))), sa_math::vector::CVector::new(f32::from_bits(dx), f32::from_bits(dy), f32::from_bits(dz)), 1);
        }
    }
    object_damage(obj, x(dmg).fmul_m(10.0).to_f32(), pos, np, ped, current_weapon(ped));
    hit_tail(this, ped, obj, surface, pos, np);
}

/// A stack `CEventVehicleDamageWeapon`: `CEventVehicleDamage(veh, ped, 5)` (0x4B18D0) with the vtable 0x86D640 stored,
/// added to `to`'s event group, then destroyed as the original does (vtable stored again, destructor 0x4B1990).
fn hit_car_event(to: GPtr<u8>, veh: GPtr<u8>, ped: GPtr<u8>) {
    let mut ev = [0u32; 8];
    let ep: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    event_vehicle_damage_ctor(ep, veh, ped, 5);
    wr::<u32>(ep, 0, 0x86_D640);
    event_group_add(rd::<GPtr<u8>>(to, 0x47C).byte_add(0x68), ep, 0);
    wr::<u32>(ep, 0, 0x86_D640);
    event_vehicle_damage_dtor(ep);
}

/// `CTaskSimpleFight::<hit vehicle>(CPed* ped, CVehicle* veh, CVector* pos, CVector* normal, int16 component, uint8
/// surface)` (0x61D0B0, `ret 0x18`).
///
/// The health +0x4C0 is read first, then `dmg` = the strike damage (0x61C740, stored as a float). A chainsaw (weapon
/// 9) adds sparks at `pos` along the ped's bone 0x18 matrix `at` vector (force 5.0, 0x20 sparks, zero across, type 1,
/// spread 0.3, life 1.0) and damages with `handling+4 * dmg * 0.00075`; any other weapon with `handling+4 * dmg *
/// 0.01` (vtable +0xE0, the weapon type passed). Then `ReportCrime(0x15, veh, ped)`, a `CEventVehicleDamageWeapon(veh,
/// ped, 5)` for the driver and every non-null passenger (count +0x488 re-read). When the health dropped (ordered
/// `<`), +0x508 = the weapon byte read at the start and +0x50C = the ped (reference registered). The shared tail
/// follows with the vehicle as the hit entity.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x61D0B0, conv = thiscall, name = "FUN_0061d0b0",
       expect = "6A FF 68 80 F3 83 00",
       unverified = "frontier2-c13", route = "L2", hazards = "heap-writes", recs = "weapon-combat",
       note = "the original keeps the damage and the passenger counter in its argument slots")]
pub fn fight_hit_car(this: GPtr<u8>, ped: GPtr<u8>, veh: GPtr<u8>, pos: GPtr<u8>, normal: GPtr<u8>, component: u32, surface: u32) {
    let health = rd::<f32>(veh, 0x4C0);
    let dmg = X87::from_f64_exact(fight_strike_damage_c(this, ped).to_f64()).to_f32();
    let weapon = current_weapon(ped);
    let handling = rd::<GPtr<u8>>(veh, 0x384);
    if weapon == 9 {
        let hier = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(ped, 0x18));
        let idx = rp_hanim_id_get_index(hier, 0x18);
        let at = rp_hanim_hierarchy_get_matrix_array(hier).byte_add(idx.wrapping_shl(6) as isize).byte_add(0x20);
        let mut dir = [rd::<u32>(at, 0), rd::<u32>(at, 4), rd::<u32>(at, 8)];
        let dp: GPtr<u8> = GPtr::from_raw(dir.as_mut_ptr()).cast();
        let zero = sa_math::vector::CVector::new(0.0, 0.0, 0.0);
        fx_add_sparks(GPtr::from_addr(G_FX), pos, dp, 5.0, 0x20, zero, 1, f32::from_bits(0x3E99_999A), 1.0);
        let f = x(rd::<f32>(handling, 4)).fmul_m(dmg).fmul_m(sa_core::field::gf(0x86_D6AC)).to_f32();
        v_vehicle_damage(veh, f.to_bits(), component, ped, pos, normal, 9);
    } else {
        let f = x(rd::<f32>(handling, 4)).fmul_m(dmg).fmul_m(sa_core::field::gf(0x85_8C58)).to_f32();
        v_vehicle_damage(veh, f.to_bits(), component, ped, pos, normal, weapon);
    }
    report_crime(0x15, veh, ped);
    let drv = rd::<GPtr<u8>>(veh, 0x460);
    if !drv.is_null() {
        hit_car_event(drv, veh, ped);
    }
    let mut i = 0u32;
    while i < u32::from(rd::<u8>(veh, 0x488)) {
        let p = rd::<GPtr<u8>>(veh, 0x464 + 4 * i as isize);
        if !p.is_null() {
            hit_car_event(p, veh, ped);
        }
        i += 1;
    }
    if !x(rd::<f32>(veh, 0x4C0)).fcom(x(health)).test_parity(0x05) {
        wr::<u8>(veh, 0x508, weapon as u8);
        let slot = veh.byte_add(0x50C);
        wr::<GPtr<u8>>(slot, 0, ped);
        entity_register_reference(ped, slot);
    }
    hit_tail(this, ped, veh, surface, pos, normal);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FIGHT_HIT_OBJECT, HOOK_FIGHT_HIT_CAR];
