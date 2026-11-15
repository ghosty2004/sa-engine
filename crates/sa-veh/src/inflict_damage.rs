//! FRONTIER-2 (batch `frontier2-1`): `CVehicle::InflictDamage` (0x6D7C90), the frontier's most-called direct callee
//! (17 callers), reached by the crossing scenario. Ported from the 1.0 US bytes; gta-reversed differs, the bytes are
//! followed: the tyre-burst chance table has no entry for the sawn-off and SPAS (0), a bike rider's chance is capped
//! at 1 only when it is above 1, and the copbike exception applies to non-bike riders.

use sa_core::abi::HookReg;
use sa_core::bindings::{
    entity_register_reference, find_player_ped, find_player_vehicle, matrix_mul_vec, stats_increment_stat,
    vector_magnitude,
};
use sa_core::crt::rand_at;
use sa_core::field::gf;
use sa_core::{hook, orig};
use sa_math::fpu::{le_or_nan, lt};
use sa_math::x87::X87;
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::{
    event_group_add, event_vehicle_damage_dtor, event_vehicle_damage_weapon_ctor, find_tyre_nearest_point,
    get_percentage_progress, get_vehicle_appearance, min_f, player_set_wanted_level_no_drop, report_crime,
    set_engine_status, vector_ne,
};

use crate::bindings::can_vehicle_be_damaged;
use crate::common::*;
use sa_core::mem::GPtr;

orig! {
    /// virtual `CVehicle::BurstTyre(uint8 piece, bool)` (vtable +0xB0)
    fn v_burst_tyre(v: GPtr<u8>, piece: u32, b: u32) -> u8 = thiscall vslot 0xB0 => unported;
    /// virtual `CVehicle::BlowUpCar(CEntity*, bool)` (vtable +0xA4)
    fn v_blow_up_car(v: GPtr<u8>, damager: GPtr<u8>, b: u32) = thiscall vslot 0xA4 => unported;
}

/// The player info of the current player (`CWorld::Players[CWorld::PlayerInFocus]`, 0x190 each).
fn player_info() -> GPtr<u8> {
    g::<u8>(0xB7_CD98 + 0x190 * usize::from(g::<u8>(0xB7_CD74).read()))
}

/// `CVehicle::InflictDamage(CEntity* damager, eWeaponType weapon, float intensity, CVector coords)` (0x6D7C90,
/// `ret 0x18`; its SEH handler push runs in the no-CD chunk 0x404CDC).
///
/// See the module docs and gta-reversed's reading. The bytes' order: `CanVehicleBeDamaged`; the player's vehicle
/// takes half the intensity at 100% progress; above 10 against the player (ped or vehicle) on a non-wrecked vehicle
/// the havoc (+2) and chase value (+1) grow and stat 0x7D by `rand() % 20 + 5`; a non-player ped shooting an
/// automobile / bike may burst the tyre nearest to `coords` (`rand() & 0x7F < chance`); a player hitting a weak petrol
/// tank within 0.25 of the gas cap makes the intensity `min(health, 1100)`; helis and planes (not RC) take 0.4 of
/// non-explosive damage. With health left: the weapon (+0x508) and damager (+0x50C, referenced); a fatal hit zeroes
/// the health, reports the crime (0xC heli / 0x10 plane / 0x14) for the player and either arms the bomb timer
/// (explosion: `(rand() & 0x7FF) + 1000`, +0x4E0 = damager) or blows the vehicle up; otherwise the health drops, the
/// occupants of a player / simple / physics vehicle get `CEventVehicleDamageWeapon` from a ped damager, and an
/// automobile crossing 250 gets engine status 225 and +0x928 = damager. A law enforcer hit by the player gives wanted
/// level 1 (no drop).
#[hook(addr = 0x6D7C90, conv = thiscall, name = "CVehicle::InflictDamage", expect = "6A FF E9 45 D0 D2 FF",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes,stack-residue,entry-protection-read",
       note = "rand at_site 0x6D7D68/0x6D7E3F/0x6D824B; the original reuses its damager / coords argument slots as temporaries; the SEH frame is not reproduced")]
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn vehicle_inflict_damage(this: GPtr<u8>, damager: GPtr<u8>, weapon: i32, intensity: f32, cx: f32, cy: f32, cz: f32) {
    let mut damageable: u8 = 0;
    if can_vehicle_be_damaged(this, damager, weapon as u32, GPtr::from_raw(&mut damageable)) == 0 {
        return;
    }
    let mut intensity = intensity;
    if rd::<u8>(this, 0x36) & 0xF8 == 0 && !get_percentage_progress().fcom(x(gf(0x85_8628))).test_any(0x01) {
        intensity = x(intensity).fmul_m(gf(0x85_8B8C)).to_f32();
    }
    if !le_or_nan(x(intensity), x(gf(0x85_862C)))
        && (damager == find_player_ped(-1) || damager == find_player_vehicle(-1, 0))
        && rd::<u8>(this, 0x36) & 0xF8 != 0x28
    {
        let pi = player_info();
        wr(pi, 0x140, rd::<u32>(pi, 0x140).wrapping_add(2));
        wr(pi, 0x148, x(rd::<f32>(pi, 0x148)).fadd_m(gf(0x85_8624)).to_f32());
        let r = rand_at(0x6D_7D68) % 20 + 5;
        stats_increment_stat(0x7D, xi(r).to_f32());
    }
    if !damager.is_null() && rd::<u8>(damager, 0x36) & 7 == 3 && matches!(rd::<u32>(this, 0x590), 0 | 9) {
        let mut chance: i32 = match weapon {
            22 | 23 | 25 | 28 | 29 | 32 | 52 => 5,
            24 => 0x40,
            30 | 31 => 10,
            _ => 0,
        };
        if ped_is_player(damager) != 0 {
            chance = 0;
        }
        let dv = rd::<GPtr<u8>>(damager, 0x58C);
        let go = if !dv.is_null() && rd::<u32>(dv, 0x594) == 9 {
            if chance > 1 {
                chance = 1;
            }
            chance != 0
        } else {
            let drv = rd::<GPtr<u8>>(this, 0x460);
            !(rd::<u16>(this, 0x22) == 0x20B && !drv.is_null() && rd::<u32>(drv, 0x598) == 6) && chance != 0
        };
        if go && (rd::<u8>(this, 0x42B) as i8) >= 0 && (rand_at(0x6D_7E3F) & 0x7F) < chance {
            if rd::<u32>(this, 0x590) == 9 {
                let t = find_tyre_nearest_point(this, cx, cy);
                v_burst_tyre(this, (t as u8).wrapping_add(0xD) as u32, 0);
            } else if get_vehicle_appearance(this) == 1 {
                let t = find_tyre_nearest_point(this, cx, cy);
                v_burst_tyre(this, (t as u8).wrapping_add(0xD) as u32, 1);
            }
        }
    }
    if rd::<u8>(this, 0x42E) & 0x40 != 0
        && damageable != 0
        && !damager.is_null()
        && rd::<u8>(damager, 0x36) & 7 == 3
        && ped_is_player(damager) != 0
    {
        let mi = g::<GPtr<u8>>(0xA9_B0C8 + 4 * rd::<i16>(this, 0x22) as usize).read();
        let cap = rd::<GPtr<u8>>(mi, 0x5C).byte_add(0x60);
        let mut gas = [rd::<u32>(cap, 0), rd::<u32>(cap, 4), rd::<u32>(cap, 8)];
        let mut zero = [0u32; 3];
        let gp: GPtr<u8> = GPtr::from_raw(gas.as_mut_ptr()).cast();
        if vector_ne(gp, GPtr::from_raw(zero.as_mut_ptr()).cast()) != 0 {
            let mut w = [0u32; 3];
            let o = matrix_mul_vec(GPtr::from_raw(w.as_mut_ptr()).cast(), rd::<GPtr<u8>>(this, 0x14), gp);
            let d = [
                x(cx).fsub_m(rd::<f32>(o, 0)).to_f32().to_bits(),
                x(cy).fsub_m(rd::<f32>(o, 4)).to_f32().to_bits(),
                x(cz).fsub_m(rd::<f32>(o, 8)).to_f32().to_bits(),
            ];
            let mut d = d;
            let m = vector_magnitude(GPtr::from_raw(d.as_mut_ptr()));
            if lt(X87::from_f64_exact(m), x(gf(0x85_8C84))) {
                intensity = min_f(rd::<u32>(this, 0x4C0), 0x4489_8000).to_f32();
            }
        }
    }
    let sub = rd::<u32>(this, 0x594);
    if (sub == 3 || sub == 4) && (rd::<u8>(this, 0x42C) as i8) >= 0 && !matches!(weapon, 0x33 | 0x10 | 0x13 | 0x14) {
        intensity = x(intensity).fmul_m(gf(0x85_8EE8)).to_f32();
    }
    if !le_or_nan(x(rd::<f32>(this, 0x4C0)), x(gf(0x85_8B50))) {
        wr(this, 0x508, weapon as u8);
        if !damager.is_null() {
            let slot = this.byte_add(0x50C);
            wr(slot, 0, damager);
            entity_register_reference(damager, slot);
        }
        if !le_or_nan(x(rd::<f32>(this, 0x4C0)), x(intensity)) {
            let old = rd::<f32>(this, 0x4C0);
            wr(this, 0x4C0, x(old).fsub_m(intensity).to_f32());
            let status = u32::from(rd::<u8>(this, 0x36)) >> 3;
            if matches!(status, 0 | 2 | 3) && !damager.is_null() && rd::<u8>(damager, 0x36) & 7 == 3 {
                let notify = |ped: GPtr<u8>| {
                    let mut ev = [0u32; 0x10];
                    let ep: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
                    event_vehicle_damage_weapon_ctor(ep, this, damager, weapon);
                    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ep, 0);
                    wr::<u32>(ep, 0, 0x86_D640);
                    event_vehicle_damage_dtor(ep);
                };
                let drv = rd::<GPtr<u8>>(this, 0x460);
                if !drv.is_null() {
                    notify(drv);
                }
                let mut i = 0usize;
                while i < usize::from(rd::<u8>(this, 0x488)) {
                    let p = rd::<GPtr<u8>>(this, 0x464 + 4 * i as isize);
                    if !p.is_null() {
                        notify(p);
                    }
                    i += 1;
                }
            }
            let k = x(gf(0x85_9F80));
            if !x(old).fcom(k).test_any(0x01) && lt(x(rd::<f32>(this, 0x4C0)), k) && rd::<u32>(this, 0x590) == 0 {
                set_engine_status(this.byte_add(0x5A0), 0xE1);
                let slot = this.byte_add(0x928);
                wr(slot, 0, damager);
                if !damager.is_null() {
                    entity_register_reference(damager, slot);
                }
            }
        } else {
            wr::<u32>(this, 0x4C0, 0);
            if damager == find_player_ped(-1) {
                let rc = (rd::<u8>(this, 0x42C) as i8) < 0;
                let crime = match rd::<u32>(this, 0x594) {
                    3 if !rc => 0xC,
                    4 if !rc => 0x10,
                    _ => 0x14,
                };
                report_crime(crime, this, damager);
            }
            if weapon == 0x33 {
                let t = (rand_at(0x6D_824B) & 0x7FF) + 0x3E8;
                wr(this, 0x4DE, t as u16);
                let slot = this.byte_add(0x4E0);
                wr(slot, 0, damager);
                if !damager.is_null() {
                    entity_register_reference(damager, slot);
                }
            } else {
                v_blow_up_car(this, damager, 0);
            }
        }
    }
    if rd::<u8>(this, 0x428) & 1 != 0 && damager == find_player_ped(-1) {
        player_set_wanted_level_no_drop(find_player_ped(-1), 1);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_INFLICT_DAMAGE];
