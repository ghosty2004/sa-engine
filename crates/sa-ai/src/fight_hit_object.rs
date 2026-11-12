//! WRITER-NATIVE (batch `native-34`): 0x61D400, `CTaskSimpleFight`'s hit on an object (unnamed), reached when a
//! punch lands on street furniture. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// The fight strike damage (0x61C740, ST0)
    fn fight_strike_damage_c(t: GPtr<u8>, ped: GPtr<u8>) -> f64 = thiscall 0x61C740 => unported;
    /// `CEntity::SetIsStatic(bool)` (vtable slot 0x10)
    fn entity_set_is_static(e: GPtr<u8>, b: u32) = thiscall vslot 0x10 => unported;
    /// `CPhysical::AddToMovingList()` (0x542800)
    fn physical_add_to_moving_list(p: GPtr<u8>) = thiscall 0x542800 => unported;
    /// `CPhysical::ApplyForce(CVector force, CVector point, bool)` (0x542B50)
    #[allow(clippy::too_many_arguments)]
    fn physical_apply_force(p: GPtr<u8>, fx: u32, fy: u32, fz: u32, px: u32, py: u32, pz: u32, b: u32) = thiscall 0x542B50 => unported;
    /// `CObject::ObjectDamage(float, CVector* origin, CVector* dir, CEntity* damager, eWeaponType)` (0x5A0D90)
    fn object_damage(o: GPtr<u8>, dmg: f32, origin: GPtr<u8>, dir: GPtr<u8>, damager: GPtr<u8>, weapon: u32) = thiscall 0x5A0D90 => unported;
    /// `CAEPedWeaponAudioEntity::AddAudioEvent(eAudioEvents)` (0x4E69F0)
    fn ped_weapon_audio_event(a: GPtr<u8>, ev: u32) = thiscall 0x4E69F0 => unported;
    /// `CAEPedAudioEntity::AddAudioEvent(eAudioEvents, float volume, float speed, CPhysical*, uint8 surface, int32,
    /// uint32)` (0x4E2BB0)
    #[allow(clippy::too_many_arguments)]
    fn ped_audio_event(a: GPtr<u8>, ev: u32, vol: f32, speed: f32, p: GPtr<u8>, surface: u32, b: u32, c: u32) = thiscall 0x4E2BB0 => unported;
    /// `Fx_c::AddPunchImpact(const CVector&, const CVector&, int32)` (0x49F670) on `g_fx` (0xA9AE00)
    fn fx_add_punch_impact(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, n: i32) = thiscall 0x49F670 => unported;
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
    let dmg = X87::from_f64_exact(fight_strike_damage_c(this, ped)).to_f32();
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
            physical_apply_force(obj, f(0), f(4), f(8), dx, dy, dz, 1);
        }
    }
    let slot = i32::from(rd::<i8>(ped, 0x718));
    let weapon = rd::<u32>(ped, (0x5A0 + slot * 0x1C) as isize);
    object_damage(obj, x(dmg).fmul_m(10.0).to_f32(), pos, np, ped, weapon);
    let slot = i32::from(rd::<i8>(ped, 0x718));
    if rd::<u32>(ped, (0x5A0 + slot * 0x1C) as isize) == 9 {
        ped_weapon_audio_event(ped.byte_add(0x394), 0x9B);
    }
    let idx = i32::from(rd::<i8>(this, 0x24)) * 0x22 + i32::from(rd::<i8>(this, 0x25));
    let sfx = g::<u32>((0xC1_6F0Cu32).wrapping_add((idx as u32).wrapping_mul(4)) as usize).read();
    ped_audio_event(ped.byte_add(0x138), sfx, 0.0, 1.0, obj, surface, 0, 0);
    fx_add_punch_impact(GPtr::from_addr(0xA9_AE00), pos, np, 4);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FIGHT_HIT_OBJECT];
