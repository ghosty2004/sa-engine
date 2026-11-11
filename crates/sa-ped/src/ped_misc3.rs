//! Small PED rows of PORT-PED-3 (agent): ped weapon audio entity, speech wrappers, drunk blur, walk anim speed, aim
//! flag, destroy flag. Ported from the 1.0 US bytes (addresses cited); specs in docs/re/specs/PED/, gta-reversed as a
//! reading aid (original bugs kept).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::*;

/// `CMBlur::Drunkness` (f32)
const MBLUR_DRUNKNESS: usize = 0xC73C58;

orig! {
    /// `CAEWeaponAudioEntity::Initialise()` (0x503450)
    fn weapon_audio_initialise(a: GPtr<u8>) = thiscall 0x503450 => unported;
    /// `CAEWeaponAudioEntity::PlayFlameThrowerIdleGasLoop(CPhysical*)` (0x503870)
    fn weapon_audio_play_flame_thrower_idle_gas_loop(a: GPtr<u8>, p: GPtr<u8>) = thiscall 0x503870 => unported;
    /// `CAEWeaponAudioEntity::StopFlameThrowerIdleGasLoop()` (0x5034E0)
    fn weapon_audio_stop_flame_thrower_idle_gas_loop(a: GPtr<u8>) = thiscall 0x5034E0 => unported;
    /// `CPedIntelligence::GetTaskSwim()` (0x601070)
    fn get_task_swim(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `CAEPedSpeechAudioEntity` virtual slot 1 (vtable +4), the script speech entry
    fn speech_v1(s: GPtr<u8>, id: u32, a: u32, b: u32, c: u32, d: u32) -> u32 = thiscall vslot 0x4 => unported;
    /// `CAEPedSpeechAudioEntity::AddSayEvent(int32, int16, uint32, float, uint8, uint8, uint8)` (0x4E6550)
    fn speech_add_say_event(s: GPtr<u8>, ev: u32, ctx: u32, off: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x4E6550 => unported;
    /// `CAnimManager::GetAnimAssociation(AssocGroupId, AnimationId)` (0x4D3A60)
    fn anim_get_anim_association(group: i32, id: i32) -> GPtr<u8> = cdecl 0x4D3A60 => unported;
    /// `CAnimManager::UncompressAnimation(CAnimBlendHierarchy*)` (0x4D41C0)
    fn anim_uncompress_animation(h: GPtr<u8>) = cdecl 0x4D41C0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CPed::GetWeaponSkill(eWeaponType)` (0x5E3B60; EAX pushed whole)
    fn ped_get_weapon_skill_type(p: GPtr<u8>, ty: i32) -> i32 = thiscall 0x5E3B60 => unported;
    /// `CVehicle::RemovePassenger(CPed*)` (0x6D1610)
    fn vehicle_remove_passenger(v: GPtr<u8>, p: GPtr<u8>) = thiscall 0x6D1610 => unported;
    /// `CVehicle::IsVehiclePointerValid(CVehicle*)` (0x6E38F0, AL)
    fn vehicle_is_pointer_valid(v: GPtr<u8>) -> u8 = cdecl 0x6E38F0 => unported;
    /// `CCoverPoint::ReleaseCoverPointForPed(CPed*)` (0x698EF0)
    fn cover_point_release_for_ped(c: GPtr<u8>, p: GPtr<u8>) = thiscall 0x698EF0 => unported;
    /// `CRadar::ClearBlipForEntity(eBlipType, int32 handle)` (0x587C60)
    fn radar_clear_blip_for_entity(ty: i32, handle: i32) = cdecl 0x587C60 => unported;
    /// `CPed::GiveDelayedWeapon(eWeaponType, uint32)` (0x5E89B0)
    fn ped_give_delayed_weapon(p: GPtr<u8>, ty: i32, ammo: i32) = thiscall 0x5E89B0 => unported;
    /// `CPed::SetCurrentWeapon(int32 slot)` (0x5E61F0)
    fn ped_set_current_weapon_slot(p: GPtr<u8>, slot: i32) = thiscall 0x5E61F0 => unported;
    /// `CPickups::CreatePickupCoorsCloseToCoors(float, float, float, float*, float*, float*)` (0x458A80)
    fn pickups_create_pickup_coors_close_to_coors(x: u32, y: u32, z: u32, ox: GPtr<u8>, oy: GPtr<u8>, oz: GPtr<u8>) = cdecl 0x458A80 => unported;
    /// `CPed::CreateDeadPedPickupCoors(float*, float*, float*)` (0x459180)
    fn create_dead_ped_pickup_coors_call(p: GPtr<u8>, x: GPtr<u8>, y: GPtr<u8>, z: GPtr<u8>) = thiscall 0x459180 => unported;
    /// `CWeapon::IsTypeMelee()` (0x73B1C0, AL)
    fn weapon_is_type_melee(w: GPtr<u8>) -> u8 = thiscall 0x73B1C0 => unported;
    /// `CPickups::TryToMerge_WeaponType(CVector, eWeaponType, ePickupType, uint32, bool)` (0x4555A0, AL)
    fn pickups_try_to_merge_weapon_type(x: u32, y: u32, z: u32, ty: i32, pt: i32, ammo: i32, b: u32) -> u8 = cdecl 0x4555A0 => unported;
    /// `CPickups::GenerateNewOne_WeaponType(CVector, eWeaponType, ePickupType, uint32, bool, char*)` (0x457380)
    fn pickups_generate_new_one_weapon_type(x: u32, y: u32, z: u32, ty: i32, pt: i32, ammo: i32, b: u32, msg: u32) -> i32 = cdecl 0x457380 => unported;
    /// `CPed::ClearWeapons()` (0x5E6320)
    fn ped_clear_weapons(p: GPtr<u8>) = thiscall 0x5E6320 => unported;
}

/// `CAEPedWeaponAudioEntity::Initialise(CPed*)` (0x4E6AA0): `+0xA4 = ped`, `CAEWeaponAudioEntity::Initialise()`,
/// `+0xA0 = 1`.
#[hook(addr = 0x4E6AA0, conv = thiscall, name = "CAEPedWeaponAudioEntity::Initialise", expect = "8B 44 24 04 56")]
pub fn ped_weapon_audio_initialise(this: GPtr<u8>, ped: GPtr<u8>) {
    wr::<GPtr<u8>>(this, 0xA4, ped);
    weapon_audio_initialise(this);
    wr::<u8>(this, 0xA0, 1);
}

/// `CAEPedWeaponAudioEntity::Service()` (0x4E6AE0): a ped on foot, not swimming, holding the flamethrower (0x25)
/// plays the idle gas loop; every other case (no ped too) stops it (tail jump).
#[hook(addr = 0x4E6AE0, conv = thiscall, name = "CAEPedWeaponAudioEntity::Service", expect = "56 8B F1 8B 86 A4 00 00 00")]
pub fn ped_weapon_audio_service(this: GPtr<u8>) {
    let ped = rd::<GPtr<u8>>(this, 0xA4);
    if !ped.is_null() && rd::<u32>(ped, 0x46C) & 0x100 == 0 && get_task_swim(rd::<GPtr<u8>>(ped, 0x47C)).is_null() {
        let ped = rd::<GPtr<u8>>(this, 0xA4);
        let slot = i32::from(rd::<i8>(ped, 0x718));
        let w = ped.byte_add((slot * 0x1C + 0x5A0) as isize);
        if !w.is_null() && rd::<i32>(w, 0) == 0x25 {
            weapon_audio_play_flame_thrower_idle_gas_loop(this, ped);
            return;
        }
    }
    weapon_audio_stop_flame_thrower_idle_gas_loop(this);
}

/// `CPed::SayScript(int32, uint8, uint8, uint8)` (0x5EFFB0): `m_pedSpeech (+0x294)` virtual slot 1 with event 0x35 and
/// the four argument dwords as pushed; returns its EAX.
#[hook(addr = 0x5EFFB0, conv = thiscall, name = "CPed::SayScript", expect = "8B 54 24 10 8B 81 94 02 00 00", this = "CPed")]
pub fn say_script(this: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) -> u32 {
    speech_v1(this.byte_add(0x294), 0x35, a, b, c, d)
}

/// `CPed::Say(uint16 ctx, uint32 offset, float vol, uint8, uint8, uint8)` (0x5EFFE0): ctx word 0 returns the argument
/// dword with its low word set to 0xFFFF (`or ax, 0xFFFF`); else `AddSayEvent(0x34, ctx dword as pushed, ...)` on
/// `m_pedSpeech (+0x294)`, returning its EAX.
#[hook(addr = 0x5EFFE0, conv = thiscall, name = "CPed::Say", expect = "8B 44 24 04 66 85 C0", this = "CPed")]
pub fn say(this: GPtr<u8>, ctx: u32, off: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 {
    if ctx as u16 == 0 {
        return ctx | 0xFFFF;
    }
    speech_add_say_event(this.byte_add(0x294), 0x34, ctx, off, vol, a, b, c)
}

/// `CMBlur::SetDrunkBlur(float)` (0x71D730): `Drunkness = v >= 1 or NaN ? 1.0f : v > 0 ? v : 0`.
#[hook(addr = 0x71D730, conv = cdecl, name = "CMBlur::SetDrunkBlur", expect = "D9 44 24 04 D8 1D 24 86 85 00")]
pub fn mblur_set_drunk_blur(v: f32) {
    // fcomp 1.0f; test ah, 5; jnp: only Less goes on
    if x(v).fcom(x(1.0)) != Cmp::Less {
        g::<u32>(MBLUR_DRUNKNESS).write(0x3F80_0000);
    } else if x(v).fcom(X87::zero()) == Cmp::Greater {
        g::<f32>(MBLUR_DRUNKNESS).write(v);
    } else {
        g::<u32>(MBLUR_DRUNKNESS).write(0);
    }
}

/// `CPed::GetWalkAnimSpeed()` (0x5E04B0, ST0): the WALK association of the ped's anim group, `UncompressAnimation` of
/// its hierarchy, then `(last frame +0x18 - first frame +0x18) / totalTime` of the first sequence (frame stride 0x20
/// when bit 1 of the sequence flags is set, else 0x14; the bytes read +0x18 of a 0x14 frame too), 0.0f without
/// frames.
#[hook(addr = 0x5E04B0, conv = thiscall, name = "CPed::GetWalkAnimSpeed", expect = "8B 81 D4 04 00 00", this = "CPed")]
pub fn get_walk_anim_speed(this: GPtr<u8>) -> X87 {
    let a = anim_get_anim_association(rd::<i32>(this, 0x4D4), 0);
    let h = rd::<GPtr<u8>>(a, 0x10);
    let seq = rd::<GPtr<u8>>(h, 4);
    anim_uncompress_animation(h);
    let n = rd::<i16>(seq, 6);
    if n <= 0 {
        return X87::zero();
    }
    let frames = rd::<GPtr<u8>>(seq, 8);
    let stride = if (rd::<u8>(seq, 4) >> 1) & 1 != 0 { 0x20 } else { 0x14 };
    let last = frames.byte_add(((i32::from(n) - 1) * stride) as isize);
    x(rd::<f32>(last, 0x18)).fsub_m(rd::<f32>(frames, 0x18)).fdiv_m(rd::<f32>(h, 0x10))
}

/// `CPed::SetAimFlag(float heading)` (0x5E8830): F1 bit 4 set / bit 5 cleared, look direction = heading, look time 0,
/// look entity (+0x738) released and cleared, the IK "aim with arm" bit (+0x528 bit 2) cleared for ducking peds and
/// then set from bit 1 of the active weapon info's flags (skill from `GetWeaponSkill(type)`).
#[hook(addr = 0x5E8830, conv = thiscall, name = "CPed::SetAimFlag-Heading", expect = "56 8B F1 8B 86 6C 04 00 00", this = "CPed")]
pub fn set_aim_flag_heading(this: GPtr<u8>, heading: u32) {
    let f1 = rd::<u32>(this, 0x46C);
    wr::<u32>(this, 0x73C, heading);
    let slot = this.byte_add(0x738);
    let e = rd::<GPtr<u8>>(slot, 0);
    wr::<u32>(this, 0x46C, (f1 & 0xFFFF_FFDF) | 0x10);
    wr::<u32>(this, 0x748, 0);
    if !e.is_null() {
        clean_up_old_reference(e, slot);
    }
    if rd::<u32>(this, 0x46C) & 0x0400_0000 != 0 {
        wr::<u32>(this, 0x528, rd::<u32>(this, 0x528) & 0xFFFF_FFFB);
    }
    wr::<u32>(slot, 0, 0);
    let ty = rd::<i32>(this, (i32::from(rd::<i8>(this, 0x718)) * 0x1C + 0x5A0) as isize);
    let skill = ped_get_weapon_skill_type(this, ty);
    let info = get_weapon_info(ty, skill);
    let ik = rd::<u32>(this, 0x528);
    if (rd::<u32>(info, 0x18) >> 1) & 1 != 0 {
        wr::<u32>(this, 0x528, ik | 4);
    } else {
        wr::<u32>(this, 0x528, ik & 0xFFFF_FFFB);
    }
}

/// `CPed::FlagToDestroyWhenNextProcessed()` (0x5E7B70): entity flag 0x800; for a ped in a vehicle: leave the driver
/// seat (player drivers mark the vehicle abandoned unless its status is 5) or `RemovePassenger`, clear F1 bit 8,
/// release the vehicle reference when `IsVehiclePointerValid`, clear +0x58C; mission peds (created-by 2) release their
/// cover point and their radar blip (F3 bit 13, handle `index << 8 | flag byte`) and become dead (state 55); others
/// get state 0.
#[hook(addr = 0x5E7B70, conv = thiscall, name = "CPed::FlagToDestroyWhenNextProcessed", expect = "56 8B F1 8B 4E 1C", this = "CPed")]
pub fn flag_to_destroy_when_next_processed(this: GPtr<u8>) {
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x800);
    if rd::<u32>(this, 0x46C) & 0x100 == 0 {
        return;
    }
    let vslot = this.byte_add(0x58C);
    let veh = rd::<GPtr<u8>>(vslot, 0);
    if veh.is_null() {
        return;
    }
    let drv = rd::<GPtr<u8>>(veh, 0x460);
    if drv == this {
        if !drv.is_null() {
            clean_up_old_reference(drv, veh.byte_add(0x460));
        }
        let veh = rd::<GPtr<u8>>(vslot, 0);
        wr::<u32>(veh, 0x460, 0);
        let ty = rd::<i32>(this, 0x598);
        if ty == 0 || ty == 1 {
            let veh = rd::<GPtr<u8>>(vslot, 0);
            if rd::<u8>(veh, 0x36) & 0xF8 != 0x28 {
                wr::<u8>(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 0x20);
            }
        }
    } else {
        vehicle_remove_passenger(veh, this);
    }
    wr::<u32>(this, 0x46C, rd::<u32>(this, 0x46C) & 0xFFFF_FEFF);
    if vehicle_is_pointer_valid(rd::<GPtr<u8>>(vslot, 0)) != 0 {
        let v = rd::<GPtr<u8>>(vslot, 0);
        if !v.is_null() {
            clean_up_old_reference(v, vslot);
        }
    }
    wr::<u32>(vslot, 0, 0);
    if rd::<u8>(this, 0x484) != 2 {
        wr::<u32>(this, 0x530, 0);
        return;
    }
    let cp = rd::<GPtr<u8>>(this, 0x788);
    if !cp.is_null() {
        cover_point_release_for_ped(cp, this);
        wr::<u32>(this, 0x788, 0);
    }
    if rd::<u32>(this, 0x474) & 0x2000 != 0 {
        let pool = g::<GPtr<u8>>(PED_POOL).read();
        let base = rd::<GPtr<u8>>(pool, 0);
        let idx = ((this.addr() as u32).wrapping_sub(base.addr() as u32) as i32) / 0x7C4;
        let flag = i32::from(rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize).read());
        radar_clear_blip_for_entity(2, (idx << 8) + flag);
    }
    wr::<u32>(this, 0x530, 0x37);
}

/// `GiveDelayedWeapon(type, 50)` then `SetCurrentWeapon(GetWeaponInfo(type, 1)->m_nSlot)` (the pair of 0x5E8B16..).
fn give_and_select(this: GPtr<u8>, ty: i32, ammo: i32) {
    ped_give_delayed_weapon(this, ty, ammo);
    ped_set_current_weapon_slot(this, rd::<i32>(get_weapon_info(ty, 1), 0x14));
}

/// `CPed::GiveWeaponAtStartOfFight()` (0x5E8AB0): not for mission peds or peds holding a weapon; by ped type
/// (jump tables 0x5E8BC4 / 0x5E8BB8 for types 7..22): gangs 7..16 get the pistol (0x16) when `(seed & 0x3FF) < 400`;
/// types 17, 20, 22 first the knife (4) when `< 200`, then the pistol when `< 400`; each only while the delayed weapon
/// (+0x710) is 0x37 (none; re-read before each); types 18, 19, 21 and the rest nothing. Ammo 50.
#[hook(addr = 0x5E8AB0, conv = thiscall, name = "CPed::GiveWeaponAtStartOfFight", expect = "56 8B F1 80 BE 84 04 00 00 02", this = "CPed")]
pub fn give_weapon_at_start_of_fight(this: GPtr<u8>) {
    if rd::<u8>(this, 0x484) == 2 {
        return;
    }
    let slot = i32::from(rd::<i8>(this, 0x718));
    if rd::<i32>(this, (slot * 0x1C + 0x5A0) as isize) != 0 {
        return;
    }
    let idx = rd::<i32>(this, 0x598).wrapping_sub(7) as u32;
    if idx > 0xF {
        return;
    }
    const CASES: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 2, 1, 2, 1];
    let seed = || u32::from(rd::<u16>(this, 0x20)) & 0x3FF;
    let case = CASES[idx as usize];
    if case == 2 {
        return;
    }
    if case == 1 && seed() < 0xC8 && rd::<i32>(this, 0x710) == 0x37 {
        give_and_select(this, 4, 0x32);
    }
    if seed() < 0x190 && rd::<i32>(this, 0x710) == 0x37 {
        give_and_select(this, 0x16, 0x32);
    }
}

/// `CPed::GiveWeaponWhenJoiningGang()` (0x5E8BE0): unarmed peds without a delayed weapon get, with 200 rounds, the
/// weapon 0x1E when the byte 0x96917D is set, else 0x23 when 0x96917E is set, else the pistol (0x16), and select it.
#[hook(addr = 0x5E8BE0, conv = thiscall, name = "CPed::GiveWeaponWhenJoiningGang", expect = "56 8B F1 0F BE 86 18 07 00 00", this = "CPed")]
pub fn give_weapon_when_joining_gang(this: GPtr<u8>) {
    let slot = i32::from(rd::<i8>(this, 0x718));
    if rd::<i32>(this, (slot * 0x1C + 0x5A0) as isize) != 0 || rd::<i32>(this, 0x710) != 0x37 {
        return;
    }
    let ty = if g::<u8>(0x96917D).read() != 0 {
        0x1E
    } else if g::<u8>(0x96917E).read() != 0 {
        0x23
    } else {
        0x16
    };
    give_and_select(this, ty, 0xC8);
}

/// `CPed::CreateDeadPedPickupCoors(float* x, float* y, float* z)` (stub 0x459180, body 0x1568940):
/// `CPickups::CreatePickupCoorsCloseToCoors(pos.x, pos.y, pos.z, x, y, z)`.
#[hook(addr = 0x459180, conv = thiscall, name = "CPed::CreateDeadPedPickupCoors-", expect = "E9 BB F7 10 01", this = "CPed")]
pub fn create_dead_ped_pickup_coors(this: GPtr<u8>, px: GPtr<u8>, py: GPtr<u8>, pz: GPtr<u8>) {
    let p = ent_pos(this);
    pickups_create_pickup_coors_close_to_coors(
        p.read().to_bits(),
        p.wrapping_index(1).read().to_bits(),
        p.wrapping_index(2).read().to_bits(),
        px,
        py,
        pz,
    );
}

/// `CPed::CreateDeadPedWeaponPickups()` (0x4591D0): not in a vehicle and not with F3 bit 9; for each of the 13
/// weapon slots holding a weapon other than 0 / 0x28 with ammo (or a melee weapon): coordinates from
/// `CreateDeadPedPickupCoors`, z + 0.3f, ammo `min(ammo, AmmoForWeapon_OnStreet[type] / 2)` (word table 0x8A5F50);
/// merged into an existing pickup when `TryToMerge_WeaponType(pos, type, 4, ammo, 0)`, else a new one of pickup
/// type 0x16 (F4 bit 19) or 4. Then `ClearWeapons`.
#[hook(addr = 0x4591D0, conv = thiscall, name = "CPed::CreateDeadPedWeaponPickups", expect = "83 EC 14 53 55", this = "CPed")]
pub fn create_dead_ped_weapon_pickups(this: GPtr<u8>) {
    if rd::<u32>(this, 0x46C) & 0x100 != 0 || rd::<u32>(this, 0x474) & 0x200 != 0 {
        return;
    }
    let ammo_on_street = g::<u16>(0x8A5F50);
    for i in 0..13 {
        let w = this.byte_add(0x5A0 + i * 0x1C);
        let ty = rd::<i32>(w, 0);
        if ty == 0 || ty == 0x28 {
            continue;
        }
        if rd::<i32>(w, 0xC) == 0 && weapon_is_type_melee(w) == 0 {
            continue;
        }
        let mut c = [0f32; 3];
        let cp: GPtr<f32> = lp(&mut c).cast();
        create_dead_ped_pickup_coors_call(this, cp.cast(), cp.wrapping_index(1).cast(), cp.wrapping_index(2).cast());
        c[2] = f32_of(x(c[2]).fadd_m(f32::from_bits(0x3E99_999A)));
        let ty = rd::<i32>(w, 0);
        let ammo = |ty: i32| {
            let half = i32::from(ammo_on_street.wrapping_index(ty).read()) >> 1;
            let a = rd::<i32>(w, 0xC);
            if a >= half { half } else { a }
        };
        let (cx, cy, cz) = (c[0].to_bits(), c[1].to_bits(), c[2].to_bits());
        if pickups_try_to_merge_weapon_type(cx, cy, cz, ty, 4, ammo(ty), 0) != 0 {
            continue;
        }
        let ty = rd::<i32>(w, 0);
        let pt = if rd::<u32>(this, 0x478) & 0x80000 != 0 { 0x16 } else { 4 };
        pickups_generate_new_one_weapon_type(cx, cy, cz, ty, pt, ammo(ty), 0, 0);
    }
    ped_clear_weapons(this);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_PED_WEAPON_AUDIO_INITIALISE,
    HOOK_PED_WEAPON_AUDIO_SERVICE,
    HOOK_SAY_SCRIPT,
    HOOK_SAY,
    HOOK_MBLUR_SET_DRUNK_BLUR,
    HOOK_GET_WALK_ANIM_SPEED,
    HOOK_SET_AIM_FLAG_HEADING,
    HOOK_FLAG_TO_DESTROY_WHEN_NEXT_PROCESSED,
    HOOK_GIVE_WEAPON_AT_START_OF_FIGHT,
    HOOK_GIVE_WEAPON_WHEN_JOINING_GANG,
    HOOK_CREATE_DEAD_PED_PICKUP_COORS,
    HOOK_CREATE_DEAD_PED_WEAPON_PICKUPS,
];
