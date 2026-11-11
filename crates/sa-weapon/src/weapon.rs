//! `CWeapon` (PORT-WEAPON-PED): the leaves and mid-size helpers. Ported from the 1.0 US bytes
//! (gta-reversed `Weapon.cpp` and `docs/re/specs/WEAPON/CWeapon*.md` as reading aids).
//!
//! `CWeapon` (0x1C): +0 type, +4 state, +8 ammo in clip, +0xC total ammo, +0x10 time for next shot,
//! +0x14 / +0x15 bytes, +0x18 `FxSystem_c*`.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

pub const WEAPON_SIZE: usize = 0x1C;
const _: () = assert!(WEAPON_SIZE == sa_core::layouts::size::CWeapon);

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CWeapon::ms_PelletTestCol` (`CColModel`)
pub const PELLET_TEST_COL: usize = 0xC8A7DC;

sa_core::globals! {
    list GLOBALS;
    /// `CWeapon::bPhotographHasBeenTaken`
    pub static PHOTOGRAPH_HAS_BEEN_TAKEN @ 0xC8A7C0: u8 = 0;
    /// `CWeapon::ms_bTakePhoto`
    pub static TAKE_PHOTO @ 0xC8A7C1: u8 = 0;
}

orig! {
    /// `CPed::GetWeaponSkill(eWeaponType)` (0x5E3B60, thiscall)
    pub fn ped_get_weapon_skill(ped: GPtr<u8>, ty: i32) -> u8 = thiscall 0x5E3B60 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0, thiscall)
    pub fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CBaseModelInfo::AddRef()` (0x4C4BA0, thiscall)
    pub fn model_add_ref(mi: GPtr<u8>) = thiscall 0x4C4BA0 => unported;
    /// `CBaseModelInfo::RemoveRef()` (0x4C4BB0, thiscall)
    pub fn model_remove_ref(mi: GPtr<u8>) = thiscall 0x4C4BB0 => unported;
    /// `CWeapon::CanBeUsedFor2Player(eWeaponType)` (0x73B240)
    pub fn can_be_used_for_2_player_call(ty: i32) -> u8 = cdecl 0x73B240 => can_be_used_for_2_player;
    /// `CWeaponInfo::Initialise()` (0x5BF750)
    pub fn weapon_info_initialise() = cdecl 0x5BF750 => crate::weapon_info::initialise;
    /// `CShotInfo::Initialise()` (0x739B60)
    pub fn shot_info_initialise() = cdecl 0x739B60 => crate::bullet_shot::shot_initialise;
    /// `CExplosion::Initialise()` (0x736A40)
    pub fn explosion_initialise() = cdecl 0x736A40 => crate::explosion::initialise;
    /// `CProjectileInfo::Initialise()` (0x737B40)
    pub fn projectile_info_initialise() = cdecl 0x737B40 => crate::projectile::initialise;
    /// `CBulletInfo::Initialise()` (0x735FD0)
    pub fn bullet_info_initialise() = cdecl 0x735FD0 => crate::bullet_shot::bullet_initialise;
    /// `CWeaponInfo::Shutdown()` (0x743C50)
    pub fn weapon_info_shutdown() = cdecl 0x743C50 => unported;
    /// `CShotInfo::Shutdown()` (0x739C20)
    pub fn shot_info_shutdown() = cdecl 0x739C20 => unported;
    /// `CExplosion::Shutdown()` (0x7368F0)
    pub fn explosion_shutdown() = cdecl 0x7368F0 => unported;
    /// `CProjectileInfo::Shutdown()` (0x737BC0)
    pub fn projectile_info_shutdown() = cdecl 0x737BC0 => crate::projectile::shutdown;
    /// `CBulletInfo::Shutdown()` (0x736000)
    pub fn bullet_info_shutdown() = cdecl 0x736000 => unported;
    /// `CColModel::RemoveCollisionVolumes()` (0x40F9E0, thiscall)
    pub fn col_model_remove_collision_volumes(cm: GPtr<u8>) = thiscall 0x40F9E0 => unported;
    /// `CShotInfo::Update()` (0x739E60)
    pub fn shot_info_update() = cdecl 0x739E60 => unported;
    /// `CExplosion::Update()` (0x737620)
    pub fn explosion_update() = cdecl 0x737620 => unported;
    /// `CProjectileInfo::Update()` (0x738B20)
    pub fn projectile_info_update() = cdecl 0x738B20 => unported;
    /// `CBulletInfo::Update()` (0x7360D0)
    pub fn bullet_info_update() = cdecl 0x7360D0 => unported;
    /// `CBirds::HandleGunShot(const CVector*, const CVector*)` (0x712E40)
    pub fn birds_handle_gun_shot(a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x712E40 => unported;
    /// `CShadows::GunShotSetsOilOnFire(const CVector*, const CVector*)` (0x707550)
    pub fn shadows_gun_shot_sets_oil_on_fire(a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x707550 => unported;
    /// `CWorld::ProcessLineOfSight(const CVector&, const CVector&, CColPoint&, CEntity*&, bool x8, CStoredCollPoly*, bool, bool)` (0x56BA00)
    pub fn world_process_line_of_sight(a: GPtr<u8>, b: GPtr<u8>, cp: GPtr<u8>, e: GPtr<u8>, bd: u8, vh: u8, pd: u8, ob: u8, du: u8, st: u8, cam: u8, sh: u8) -> u8 = cdecl 0x56BA00 => unported;
    /// `CCollision::DistToLine(const CVector*, const CVector*, const CVector*)` (0x417610, ST0)
    pub fn dist_to_line(a: GPtr<u8>, b: GPtr<u8>, p: GPtr<u8>) -> f64 = cdecl 0x417610 => unported;
    /// `CEntity::GetIsOnScreen()` (0x534540, thiscall)
    pub fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(float r, g, b, a, size, spin, life)` (0x4AB290, thiscall)
    pub fn fx_prt_mult(this: GPtr<u8>, r: f32, g: f32, b: f32, a: f32, size: f32, spin: f32, life: f32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(CVector*, CVector*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    pub fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, t: f32, mult: GPtr<u8>, a: f32, b: f32, c: f32, d: u8) = thiscall 0x4AA440 => unported;
    /// `RwMatrixCreate()` (0x7F2A50)
    pub fn rw_matrix_create() -> GPtr<u8> = cdecl 0x7F2A50 => unported;
    /// `RwMatrixDestroy(RwMatrix*)` (0x7F2A20)
    pub fn rw_matrix_destroy(m: GPtr<u8>) -> i32 = cdecl 0x7F2A20 => unported;
    /// `Fx_c::CreateMatFromVec(RwMatrix*, CVector* origin, CVector* target)` (0x49E950, thiscall)
    pub fn fx_create_mat_from_vec(fx: GPtr<u8>, m: GPtr<u8>, o: GPtr<u8>, t: GPtr<u8>) = thiscall 0x49E950 => unported;
    /// `FxManager_c::CreateFxSystem(const char*, CVector*, RwMatrix*, bool)` (0x4A9BE0, thiscall)
    pub fn create_fx_system(man: GPtr<u8>, name: GPtr<u8>, pos: GPtr<u8>, m: GPtr<u8>, flag: u8) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxSystem_c::CopyParentMatrix()` (0x4AA890, thiscall)
    pub fn fx_copy_parent_matrix(fx: GPtr<u8>) = thiscall 0x4AA890 => unported;
    /// `FxSystem_c::SetMustCreatePrts(bool)` (0x4AAC70, thiscall)
    pub fn fx_set_must_create_prts(fx: GPtr<u8>, b: u8) = thiscall 0x4AAC70 => unported;
    /// `FxSystem_c::SetConstTime(bool, float)` (0x4AA6C0, thiscall)
    pub fn fx_set_const_time(fx: GPtr<u8>, b: u8, t: f32) = thiscall 0x4AA6C0 => unported;
    /// `FxSystem_c::SetMatrix(RwMatrix*)` (0x4AA630, thiscall)
    pub fn fx_set_matrix(fx: GPtr<u8>, m: GPtr<u8>) = thiscall 0x4AA630 => unported;
    /// `CWeapon::FireInstantHitFromCar(CVehicle*, bool left, bool right)` (0x73EC40, thiscall)
    pub fn fire_instant_hit_from_car(this: GPtr<u8>, veh: GPtr<u8>, left: u8, right: u8) -> u8 = thiscall 0x73EC40 => crate::weapon_fire::fire_instant_hit_from_car;
    /// `CAEPedWeaponAudioEntity::AddAudioEvent(int32)` (0x4E69F0, thiscall)
    pub fn ped_weapon_audio_add_event(ae: GPtr<u8>, ev: i32) = thiscall 0x4E69F0 => unported;
    /// `CStats::GetPercentageProgress()` (0x5591E0, ST0)
    pub fn stats_get_percentage_progress() -> f64 = cdecl 0x5591E0 => unported;
    /// `CWeaponInfo::GetWeaponReloadTime()` (0x743D70, thiscall)
    pub fn get_weapon_reload_time_call(info: GPtr<u8>) -> u32 = thiscall 0x743D70 => crate::weapon_info::get_weapon_reload_time;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, uint32 animId)` (0x4D68B0)
    pub fn clump_get_association(clump: GPtr<u8>, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CPedIntelligence::GetTaskUseGun()` (0x600F70, thiscall)
    pub fn intel_get_task_use_gun(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F70 => unported;
    /// `CWeaponInfo::GetCrouchReloadAnimationID()` (0x685700, thiscall)
    pub fn get_crouch_reload_animation_id_call(info: GPtr<u8>) -> u32 = thiscall 0x685700 => crate::weapon_info::get_crouch_reload_animation_id;
    /// `FUN_0073a280` (0x73A280, thiscall): reload loop time A of the info's anim group
    pub fn reload_loop_time_a_call(info: GPtr<u8>, flag: u32) -> u32 = thiscall 0x73A280 => reload_loop_time_a_port;
    /// `FUN_0073a2b0` (0x73A2B0, thiscall): reload loop time B
    pub fn reload_loop_time_b_call(info: GPtr<u8>, flag: u32) -> u32 = thiscall 0x73A2B0 => reload_loop_time_b_port;
    /// `CWeapon::Reload(CPed*)` (0x73AEB0, thiscall)
    pub fn reload_call(this: GPtr<u8>, owner: GPtr<u8>) = thiscall 0x73AEB0 => reload;
    /// `CWeapon::EvaluateTargetForHeatSeekingMissile(...)` (0x73E560, ST0)
    pub fn evaluate_target_call(t: GPtr<u8>, origin: GPtr<u8>, dir: GPtr<u8>, mult: f32, planes: u8, last: GPtr<u8>) -> f64 = cdecl 0x73E560 => evaluate_target_st0;
}

fn model_info(id: i32) -> GPtr<u8> {
    g::<GPtr<u8>>(MODEL_INFO_PTRS.wrapping_add((id as usize).wrapping_mul(4))).read()
}

/// `CWeapon::IsTypeMelee()` (0x73B1C0, thiscall): `GetWeaponInfo(type, 1)->fireType == MELEE`.
#[hook(addr = 0x73B1C0, conv = thiscall, name = "CWeapon::IsTypeMelee", expect = "8B 01 6A 01 50", this = "CWeapon")]
pub fn is_type_melee(this: GPtr<u8>) -> u8 {
    (ru(get_weapon_info(ru(this, 0) as i32, 1), 0) == 0) as u8
}

/// `CWeapon::IsType2Handed()` (0x73B1E0, thiscall): types 31, 30, 27, 25, 34, 37, 33.
#[hook(addr = 0x73B1E0, conv = thiscall, name = "CWeapon::IsType2Handed", expect = "8B 01 83 F8 1F", this = "CWeapon")]
pub fn is_type_2_handed(this: GPtr<u8>) -> u8 {
    matches!(ru(this, 0), 0x1F | 0x1E | 0x1B | 0x19 | 0x22 | 0x25 | 0x21) as u8
}

/// `CWeapon::IsTypeProjectile()` (0x73B210, thiscall): types 16, 39, 17, 18, 21.
#[hook(addr = 0x73B210, conv = thiscall, name = "CWeapon::IsTypeProjectile", expect = "8B 01 83 F8 10", this = "CWeapon")]
pub fn is_type_projectile(this: GPtr<u8>) -> u8 {
    matches!(ru(this, 0), 0x10 | 0x27 | 0x11 | 0x12 | 0x15) as u8
}

/// `CWeapon::CanBeUsedFor2Player(eWeaponType)` (0x73B240, cdecl): false for 9, 34, 35, 46 (jump table
/// 0x73B268), true otherwise.
#[hook(addr = 0x73B240, conv = cdecl, name = "CWeapon::CanBeUsedFor2Player-Static", expect = "8B 44 24 04 83 C0 F7")]
pub fn can_be_used_for_2_player(ty: i32) -> u8 {
    !matches!(ty, 9 | 34 | 35 | 46) as u8
}

/// `CWeapon::HasWeaponAmmoToBeUsed()` (0x73B2A0, thiscall): types 0..5, 8..14 and 46 always; else
/// total ammo != 0 (jump table 0x73B2CC).
#[hook(addr = 0x73B2A0, conv = thiscall, name = "CWeapon::HasWeaponAmmoToBeUsed", expect = "8B 01 83 F8 2E", this = "CWeapon")]
pub fn has_weapon_ammo_to_be_used(this: GPtr<u8>) -> u8 {
    match ru(this, 0) {
        0..=5 | 8..=14 | 46 => 1,
        _ => (ru(this, 0xC) != 0) as u8,
    }
}

/// `CWeapon::CanBeUsedFor2Player()` (0x73DEF0, thiscall).
#[hook(addr = 0x73DEF0, conv = thiscall, name = "CWeapon::CanBeUsedFor2Player-Method", expect = "8B 01 50 E8 48 D3 FF FF", this = "CWeapon")]
pub fn can_be_used_for_2_player_method(this: GPtr<u8>) -> u8 {
    can_be_used_for_2_player_call(ru(this, 0) as i32)
}

/// `CWeapon::TargetWeaponRangeMultiplier(CEntity* victim, CEntity* owner)` (0x73B380, cdecl, ST0):
/// 3.0 when the victim is (or rides, or is attached to) a vehicle that is not a bike (subclass 9);
/// for a player owner 2.0 in camera mode 0x31, 3.0 in mode 0x2D; else 1.0.
#[hook(addr = 0x73B380, conv = cdecl, name = "CWeapon::TargetWeaponRangeMultiplier", expect = "8B 54 24 04 85 D2")]
pub fn target_weapon_range_multiplier(victim: GPtr<u8>, owner: GPtr<u8>) -> X87 {
    let one = X87::from_f32(1.0);
    let three = X87::from_f32(3.0);
    if victim.is_null() || owner.is_null() {
        return one;
    }
    let t = etype(victim);
    if t == 2 && ru(victim, 0x590) != 9 {
        return three;
    }
    if t == 3 {
        let v = rp(victim, 0x58C);
        if !v.is_null() && ru(v, 0x590) != 9 {
            return three;
        }
        let a = rp(victim, 0xFC);
        if !a.is_null() && etype(a) == 2 && ru(a, 0x590) != 9 {
            return three;
        }
    }
    if etype(owner) != 3 || ped_is_player(owner) == 0 {
        return one;
    }
    let cam = usize::from(g::<u8>(0xB6F081).read());
    let mode = g::<u16>(0xB6F1A8 + cam * 0x238).read();
    match mode {
        0x31 => X87::from_f32(2.0),
        0x2D => three,
        _ => one,
    }
}

/// `CWeapon::CWeapon(eWeaponType, int32 ammo)` (0x73B430, thiscall `ret 8`): total ammo clamped to
/// 99999 (signed); the clip is `min(total, info->clipSize)` (unsigned compare) when ammo != 0.
#[hook(addr = 0x73B430, conv = thiscall, name = "CWeapon::Constructor", expect = "8B 44 24 08 53", this = "CWeapon")]
pub fn constructor(this: GPtr<u8>, ty: i32, ammo: i32) -> GPtr<u8> {
    wu(this, 0, ty as u32);
    wu(this, 4, 0);
    let a = if ammo > 99_999 { 99_999 } else { ammo };
    wu(this, 0xC, a as u32);
    wu(this, 8, 0);
    if a != 0 {
        let clip = i32::from(rh(get_weapon_info(ty, 1), 0x20) as i16) as u32;
        let total = ru(this, 0xC);
        wu(this, 8, if total < clip { total } else { clip });
    }
    wu(this, 0x10, 0);
    wb(this, 0x14, 0);
    wu(this, 0x18, 0);
    wb(this, 0x15, 0);
    this
}

/// The clip refill of `Reload` / `Initialise`: `min(total, clipSize)` with the owner's skill.
fn refill(this: GPtr<u8>, owner: GPtr<u8>) {
    let ty = ru(this, 0) as i32;
    let info = if owner.is_null() {
        get_weapon_info(ty, 1)
    } else {
        let s = ped_get_weapon_skill(owner, ty);
        get_weapon_info(ru(this, 0) as i32, s)
    };
    let clip = i32::from(rh(info, 0x20) as i16) as u32;
    let total = ru(this, 0xC);
    wu(this, 8, if total < clip { total } else { clip });
}

/// `CWeapon::Reload(CPed* owner)` (0x73AEB0, thiscall `ret 4`): with ammo left, refill the clip.
#[hook(addr = 0x73AEB0, conv = thiscall, name = "CWeapon::Reload", expect = "56 8B F1 8B 46 0C", this = "CWeapon")]
pub fn reload(this: GPtr<u8>, owner: GPtr<u8>) {
    if ru(this, 0xC) != 0 {
        refill(this, owner);
    }
}

/// `CWeapon::StopWeaponEffect()` (0x73B360, thiscall): kills the fx unless the type is 18 (molotov).
#[hook(addr = 0x73B360, conv = thiscall, name = "CWeapon::StopWeaponEffect", expect = "56 8B F1 8B 4E 18", this = "CWeapon")]
pub fn stop_weapon_effect(this: GPtr<u8>) {
    let fx = rp(this, 0x18);
    if !fx.is_null() && ru(this, 0) != 0x12 {
        fx_kill(fx);
        wu(this, 0x18, 0);
    }
}

/// `CWeapon::Initialise(eWeaponType, int32 ammo, CPed* owner)` (0x73B4A0, thiscall `ret 0xC`): as the
/// constructor with the owner's skill, then `AddRef` on both weapon models; +0x14 untouched.
#[hook(addr = 0x73B4A0, conv = thiscall, name = "CWeapon::Initialise", expect = "8B 44 24 08 8B 54 24 04", this = "CWeapon")]
pub fn initialise(this: GPtr<u8>, ty: i32, ammo: i32, owner: GPtr<u8>) {
    wu(this, 0, ty as u32);
    wu(this, 4, 0);
    let a = if ammo > 99_999 { 99_999 } else { ammo };
    wu(this, 0xC, a as u32);
    wu(this, 8, 0);
    if a != 0 {
        refill(this, owner);
    }
    wu(this, 0x10, 0);
    let m1 = ru(get_weapon_info(ru(this, 0) as i32, 1), 0xC) as i32;
    let m2 = ru(get_weapon_info(ru(this, 0) as i32, 1), 0x10) as i32;
    if m1 != -1 {
        model_add_ref(model_info(m1));
    }
    if m2 != -1 {
        model_add_ref(model_info(m2));
    }
    wu(this, 0x18, 0);
    wb(this, 0x15, 0);
}

/// `CWeapon::Shutdown()` (0x73A380, thiscall): `RemoveRef` on both models, then type, state, ammo,
/// clip and shot time 0.
#[hook(addr = 0x73A380, conv = thiscall, name = "CWeapon::Shutdown", expect = "56 8B F1 8B 06", this = "CWeapon")]
pub fn shutdown(this: GPtr<u8>) {
    let m1 = ru(get_weapon_info(ru(this, 0) as i32, 1), 0xC) as i32;
    if m1 != -1 {
        model_remove_ref(model_info(m1));
    }
    let m2 = ru(get_weapon_info(ru(this, 0) as i32, 1), 0x10) as i32;
    if m2 != -1 {
        model_remove_ref(model_info(m2));
    }
    wu(this, 0, 0);
    wu(this, 4, 0);
    wu(this, 0xC, 0);
    wu(this, 8, 0);
    wu(this, 0x10, 0);
}

/// `CWeapon::InitialiseWeapons()` (0x73A300): the five `Initialise`s, then both photo flags 0.
#[hook(addr = 0x73A300, conv = cdecl, name = "CWeapon::InitialiseWeapons", expect = "E8 4B 54 E8 FF")]
pub fn initialise_weapons() {
    weapon_info_initialise();
    shot_info_initialise();
    explosion_initialise();
    projectile_info_initialise();
    bullet_info_initialise();
    PHOTOGRAPH_HAS_BEEN_TAKEN.set(0);
    TAKE_PHOTO.set(0);
}

/// `CWeapon::ShutdownWeapons()` (0x73A330): the five `Shutdown`s, then
/// `ms_PelletTestCol.RemoveCollisionVolumes()` (tail jump).
#[hook(addr = 0x73A330, conv = cdecl, name = "CWeapon::ShutdownWeapons", expect = "E8 1B 99 00 00")]
pub fn shutdown_weapons() {
    weapon_info_shutdown();
    shot_info_shutdown();
    explosion_shutdown();
    projectile_info_shutdown();
    bullet_info_shutdown();
    col_model_remove_collision_volumes(g(PELLET_TEST_COL));
}

/// `CWeapon::UpdateWeapons()` (0x73A360): shot infos, explosions, projectiles, bullets.
#[hook(addr = 0x73A360, conv = cdecl, name = "CWeapon::UpdateWeapons", expect = "E8 FB FA FF FF")]
pub fn update_weapons() {
    shot_info_update();
    explosion_update();
    projectile_info_update();
    bullet_info_update();
}

/// `CWeapon::ProcessLineOfSight(const CVector& a, const CVector& b, CColPoint&, CEntity*&,
/// eWeaponType, CEntity*, bool buildings, vehicles, peds, objects, dummies, bool, bool camera)`
/// (0x73B300, cdecl, 13 arguments): `CBirds::HandleGunShot`, `CShadows::GunShotSetsOilOnFire`, then
/// `CWorld::ProcessLineOfSight(a, b, cp, e, buildings, vehicles, peds, objects, dummies, false, camera,
/// true)` (argument 11 is ignored; see-through 0 and shoot-through 1 are constants).
#[hook(addr = 0x73B300, conv = cdecl, name = "CWeapon::ProcessLineOfSight", expect = "56 8B 74 24 0C")]
#[allow(clippy::too_many_arguments)]
pub fn process_line_of_sight(a: GPtr<u8>, b: GPtr<u8>, cp: GPtr<u8>, e: GPtr<u8>, _ty: i32, _shooter: GPtr<u8>, bd: u8, vh: u8, pd: u8, ob: u8, du: u8, _a11: u8, cam: u8) -> u8 {
    birds_handle_gun_shot(a, b);
    shadows_gun_shot_sets_oil_on_fire(a, b);
    world_process_line_of_sight(a, b, cp, e, bd, vh, pd, ob, du, 0, cam, 1)
}

/// `CWeapon::EvaluateTargetForHeatSeekingMissile(CEntity* target, const CVector& origin, const
/// CVector& dir, float tolerance, bool planesPriority, CEntity* preferred)` (0x73E560, cdecl, ST0).
/// `d = |targetPos - origin|` (f32), `l = DistToLine(origin, origin + dir*250, targetPos)`; -1.0 unless
/// `l * tolerance < d`; else `sqrt(d)*0.1 + l/d`, quartered for a plane/heli (type 3/4) target with
/// `planesPriority` and again (returning) for the preferred target. The original stores `d` and `l`
/// into its own argument slots 0 and 1 (not reproduced; the callers discard them).
#[hook(addr = 0x73E560, conv = cdecl, name = "CWeapon::EvaluateTargetForHeatSeekingMissile", expect = "83 EC 18 56 8B 74 24 20")]
pub fn evaluate_target_for_heat_seeking_missile(target: GPtr<u8>, origin: GPtr<f32>, dir: GPtr<f32>, tol: f32, planes: u8, last: GPtr<u8>) -> X87 {
    let tp = pos_ptr(target);
    let mut f = Fpu::new();
    f.fld(rf(tp, 0));
    f.fsub_m(origin.read());
    f.fld(rf(tp, 4));
    f.fsub_m(origin.wrapping_index(1).read());
    f.fld(rf(tp, 8));
    f.fsub_m(origin.wrapping_index(2).read());
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fsqrt();
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
    let d = f.fstp();
    let k = cf(0x437A_0000);
    let ex = x(dir.read()).fmul_m(k);
    let ey = x(dir.wrapping_index(1).read()).fmul_m(k);
    let ez = x(dir.wrapping_index(2).read()).fmul_m(k).to_f32();
    let mut end = [
        ex.fadd_m(origin.read()).to_f32(),
        ey.fadd_m(origin.wrapping_index(1).read()).to_f32(),
        x(ez).fadd_m(origin.wrapping_index(2).read()).to_f32(),
    ];
    let mut tpos = rv_bits(pos_ptr(target), 0);
    let l = st0(dist_to_line(origin.cast(), lp(&mut end).cast(), lp(&mut tpos).cast())).to_f32();
    let mut f = Fpu::new();
    f.fld(d);
    f.fsqrt();
    f.fmul_m(cf(0x3DCC_CCCD));
    f.fld(l);
    f.fdiv_m(d);
    f.faddp(1);
    f.fld(l);
    f.fmul_m(tol);
    f.fcomp_m(d);
    if f.jp(5) {
        return X87::from_f32(-1.0);
    }
    let mut r = f.st(0);
    if planes != 0 && etype(target) == 2 && matches!(ru(target, 0x594), 3 | 4) {
        r = r.fmul_m(0.25);
    }
    if !last.is_null() && last == target {
        r = r.fmul_m(0.25);
    }
    r
}

/// Phases B/C path of the `orig!` call (ST0 as f64).
pub fn evaluate_target_st0(t: GPtr<u8>, origin: GPtr<u8>, dir: GPtr<u8>, mult: f32, planes: u8, last: GPtr<u8>) -> f64 {
    evaluate_target_for_heat_seeking_missile(t, origin.cast(), dir.cast(), mult, planes, last).to_f64()
}

/// `CWeapon::PickTargetForHeatSeekingMissile(CVector origin, CVector dir, float tolerance, CEntity*
/// ignore, bool planesPriority, CEntity* preferred)` (0x73F910, cdecl): over the vehicle pool from the
/// last slot down, every used vehicle other than `ignore` with flag +0x42D & 0x40 and health > 0 is
/// evaluated from `origin + dir*5` (the original writes that point over its own `origin` argument);
/// the lowest non-negative score below the previous best (starting at 99999.9) wins.
#[hook(addr = 0x73F910, conv = cdecl, name = "CWeapon::PickTargetForHeatSeekingMissile", expect = "83 EC 14 D9 44 24 24")]
#[allow(clippy::too_many_arguments)]
pub fn pick_target_for_heat_seeking_missile(ox: f32, oy: f32, oz: f32, dx: f32, dy: f32, dz: f32, tol: f32, ignore: GPtr<u8>, planes: u8, last: GPtr<u8>) -> GPtr<u8> {
    let k = cf(0x40A0_0000);
    let mx = x(dx).fmul_m(k);
    let my = x(dy).fmul_m(k);
    let mz = x(dz).fmul_m(k).to_f32();
    let mut origin = [mx.fadd_m(ox).to_f32(), x(oy).fadd(my).to_f32(), x(oz).fadd_m(mz).to_f32()];
    let mut dir = [dx, dy, dz];
    let mut best = f32::from_bits(0x47C3_4FF3);
    let mut target = GPtr::NULL;
    let pool = g::<GPtr<u8>>(0xB74494).read();
    let mut i = ru(pool, 8);
    while i != 0 {
        let flags = rp(pool, 4);
        let slot = rb(flags, (i - 1) as isize);
        i -= 1;
        if (slot as i8) < 0 {
            continue;
        }
        let v = rp(pool, 0).byte_add((i as usize * 0xA18) as isize);
        if v.is_null() || v == ignore || rb(v, 0x42D) & 0x40 == 0 {
            continue;
        }
        let mut f = Fpu::new();
        f.fld(rf(v, 0x4C0));
        f.fcomp_m(0.0);
        if f.jne(0x41) {
            continue;
        }
        let r = st0(evaluate_target_call(v, lp(&mut origin).cast(), lp(&mut dir).cast(), tol, planes, last));
        let mut f = Fpu::new();
        f.push(r);
        f.fcom_m(0.0);
        if f.jne(1) {
            continue;
        }
        f.fcom_m(best);
        if f.jp(5) {
            continue;
        }
        best = f.fstp();
        target = v;
    }
    target
}

/// `CWeapon::AddGunshell(CEntity* creator, CVector* pos, CVector2D* dir, float size)` (0x73A3E0,
/// thiscall `ret 0x10`): for an on-screen creator within 10 m of the camera (`((dz² + dx²) + dy²) <=
/// 100`, NaN passes): velocity `(dir.x*1, dir.y*1, rand()/32767*0.06*20 + 0.4)`, then an extra
/// `GetRandomNumberInRange(-20, 20)` whose result is discarded (original: one more `rand()` per
/// gunshell, CONFIRMED in the spec), a gunshell particle with (0.5, 0.5, 0.5, 1, size, 1, 1) (red
/// (0.6, 0.1, 0.1) for shotgun / spas12).
#[hook(addr = 0x73A3E0, conv = thiscall, name = "CWeapon::AddGunshell", expect = "83 EC 28 56 8B 74 24 30", this = "CWeapon")]
pub fn add_gunshell(this: GPtr<u8>, creator: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<f32>, size: f32) {
    if creator.is_null() || entity_get_is_on_screen(creator) == 0 {
        return;
    }
    let cp = pos_ptr(creator);
    let cam = cam_pos();
    let mut f = Fpu::new();
    f.fld(cam.read());
    f.fsub_m(rf(cp, 0));
    f.fld(cam.wrapping_index(1).read());
    f.fsub_m(rf(cp, 4));
    f.fld(cam.wrapping_index(2).read());
    f.fsub_m(rf(cp, 8));
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fcomp_m(100.0);
    if !f.jne(0x41) {
        return;
    }
    let vx = x(dir.read()).fmul_m(1.0).to_f32();
    let vy = x(dir.wrapping_index(1).read()).fmul_m(1.0).to_f32();
    let r = sa_core::orig::at_site(0x73A47B, rand);
    let vz = xi(r)
        .fmul_m(f32::from_bits(0x3800_0100))
        .fmul_m(f32::from_bits(0x3D75_C28F))
        .fmul_m(20.0)
        .fadd_m(f32::from_bits(0x3ECC_CCCC))
        .to_f32();
    random_int(-20, 20);
    let mut mult = [0u32; 7];
    fx_prt_mult(lp(&mut mult).cast(), 0.5, 0.5, 0.5, 1.0, size, 1.0, 1.0);
    if matches!(ru(this, 0), 0x19 | 0x1B) {
        mult[0] = 0x3F19_999A;
        mult[1] = 0x3DCC_CCCD;
        mult[2] = 0x3DCC_CCCD;
    }
    let mut vel = [vx, vy, vz];
    let shell = g::<GPtr<u8>>(0xA9AE14).read();
    fx_add_particle(shell, pos, lp(&mut vel).cast(), 0.0, lp(&mut mult).cast(), -1.0, f32::from_bits(0x3F99_999A), f32::from_bits(0x3F19_999A), 0);
}

/// `CWeapon::DoWeaponEffect(CVector origin, CVector target)` (0x73E690, thiscall `ret 0x18`): for the
/// flamethrower (37), spray can (41) and extinguisher (42) a matrix from origin to target drives the
/// weapon's fx (created as "flamethrower" / "spraycan" / "extinguisher" at the origin of that matrix,
/// copy-parent, played, must-create, const time 1.0; or the existing one gets the matrix and const
/// time); for other types the fx is killed unless the type is 18. The original passes a stack copy of
/// the name; the port passes the `.rdata` string it copies from (same characters).
#[hook(addr = 0x73E690, conv = thiscall, name = "CWeapon::DoWeaponEffect", expect = "83 EC 2C 56 8B F1", this = "CWeapon")]
#[allow(clippy::too_many_arguments)]
pub fn do_weapon_effect(this: GPtr<u8>, ox: f32, oy: f32, oz: f32, tx: f32, ty: f32, tz: f32) {
    let name = match ru(this, 0) {
        0x25 => 0x872C50,
        0x29 => 0x872C44,
        0x2A => 0x872C34,
        t => {
            let fx = rp(this, 0x18);
            if !fx.is_null() && t != 0x12 {
                fx_kill(fx);
                wu(this, 0x18, 0);
            }
            return;
        }
    };
    let mut origin = [ox, oy, oz];
    let mut target = [tx, ty, tz];
    let m = rw_matrix_create();
    fx_create_mat_from_vec(g(0xA9AE00), m, lp(&mut origin).cast(), lp(&mut target).cast());
    if rp(this, 0x18).is_null() {
        let mut zero = [0u32; 3];
        let fx = create_fx_system(g(0xA9AE80), g(name), lp(&mut zero).cast(), m, 0);
        wp(this, 0x18, fx);
        if !fx.is_null() {
            fx_copy_parent_matrix(fx);
            fx_play(rp(this, 0x18));
            fx_set_must_create_prts(rp(this, 0x18), 1);
            fx_set_const_time(rp(this, 0x18), 1, 1.0);
        }
    } else {
        fx_set_matrix(rp(this, 0x18), m);
        fx_set_const_time(rp(this, 0x18), 1, 1.0);
    }
    rw_matrix_destroy(m);
}

/// `CWeapon::FireFromCar(CVehicle* veh, bool left, bool right)` (0x73FA20, thiscall `ret 0xC`): only
/// in state 0/1 with a loaded clip; after a successful `FireInstantHitFromCar` the driver's weapon
/// audio event 0x91, ammo use (unless the infinite-ammo cheat 0x969178: clip--, and total-- for
/// 0 < total < 25000 when the vehicle is not the player's or the progress is below 100 %), state 1
/// and the next shot time (reload: state 2 and the reload time; else +1000 ms). True unless the
/// state/clip check fails.
#[hook(addr = 0x73FA20, conv = thiscall, name = "CWeapon::FireFromCar", expect = "56 8B F1 8B 46 04", this = "CWeapon")]
pub fn fire_from_car(this: GPtr<u8>, veh: GPtr<u8>, left: u8, right: u8) -> u8 {
    let st = ru(this, 4);
    if (st != 0 && st != 1) || ru(this, 8) as i32 <= 0 {
        return 0;
    }
    if fire_instant_hit_from_car(this, veh, left, right) == 0 {
        return 1;
    }
    let driver = rp(veh, 0x460);
    if !driver.is_null() {
        ped_weapon_audio_add_event(driver.byte_add(0x394), 0x91);
    }
    if g::<u8>(0x969178).read() == 0 {
        let clip = ru(this, 8) as i32;
        if clip > 0 {
            wu(this, 8, (clip - 1) as u32);
        }
        let total = ru(this, 0xC) as i32;
        if total < 25_000 && total > 0 {
            let dec = if rb(veh, 0x36) & 0xF8 != 0 {
                true
            } else {
                let mut f = Fpu::new();
                f.push(st0(stats_get_percentage_progress()));
                f.fcomp_m(100.0);
                !f.jp(5)
            };
            if dec {
                wu(this, 0xC, ru(this, 0xC).wrapping_sub(1));
            }
        }
    }
    wu(this, 4, 1);
    if ru(this, 8) != 0 {
        wu(this, 0x10, time_ms().wrapping_add(1000));
        return 1;
    }
    if ru(this, 0xC) == 0 {
        return 1;
    }
    wu(this, 4, 2);
    let now = time_ms();
    let info = get_weapon_info(ru(this, 0) as i32, 1);
    wu(this, 0x10, get_weapon_reload_time_call(info).wrapping_add(now));
    1
}

/// Phases B/C paths of the reload loop time helpers (the `orig!` calls pass the full pushed dword).
pub fn reload_loop_time_a_port(info: GPtr<u8>, flag: u32) -> u32 {
    crate::weapon_info::reload_loop_time_a(info, flag as u8)
}
/// See [`reload_loop_time_a_port`].
pub fn reload_loop_time_b_port(info: GPtr<u8>, flag: u32) -> u32 {
    crate::weapon_info::reload_loop_time_b(info, flag as u8)
}

/// `CTimer::m_snPreviousTimeInMilliseconds`
fn prev_time_ms() -> u32 {
    sa_core::timer::PREV_TIME_MS.get()
}

/// The ped flag the reload sound helpers get: bit 26 of `CPed` +0x46C (`and 0xFFFFFF01` keeps it).
fn ped_flag26(ped: GPtr<u8>) -> u32 {
    (ru(ped, 0x46C) >> 26) & 0xFFFF_FF01
}

/// Plays the reload sound `ev` when `base + loop` was crossed in this frame (`loop` = AX != 0).
fn reload_sound_by_time(ped: GPtr<u8>, base: u32, lp: u32, ev: i32) {
    let w = lp & 0xFFFF;
    if w == 0 {
        return;
    }
    let t = base.wrapping_add(w);
    if prev_time_ms() < t && time_ms() >= t {
        ped_weapon_audio_add_event(ped.byte_add(0x394), ev);
    }
}

/// Plays the reload sound `ev` when the anim's current time passed `loop * 0.001` in this step.
fn reload_sound_by_anim(ped: GPtr<u8>, assoc: GPtr<u8>, lp: u32, ev: i32) {
    let mut f = Fpu::new();
    f.fild((lp & 0xFFFF) as i32);
    f.fmul_m(f32::from_bits(0x3A83_126F));
    f.fcom_m(rf(assoc, 0x20));
    if f.jp(0x41) {
        return;
    }
    f.fld(rf(assoc, 0x28));
    f.fsubr_m(rf(assoc, 0x20));
    f.fcomp_st(1);
    if !f.jp(5) {
        ped_weapon_audio_add_event(ped.byte_add(0x394), ev);
    }
}

/// `CWeapon::Update(CPed* owner)` (0x73DB40, thiscall `ret 4`): the weapon state machine (jump table
/// 0x73DEDC). Firing (1): the shotgun/spas12 pump sounds 0x92/0x93 at the reload loop times after the
/// shot, then (shot time over) state 0, or 3 for a non-melee weapon without ammo. Reloading (2):
/// with the anim flag (bit 12) and no player override the reload anim (0xE2, else the crouch reload
/// anim) drives the sounds and ends the reload at 90 % (or a gun task without anim ends it now);
/// otherwise the sounds follow the reload time; when the shot time is over `Reload(owner)` and state 0.
/// State 4 → 0. The fx is killed (not for the molotov) except on the firing path.
#[hook(addr = 0x73DB40, conv = thiscall, name = "CWeapon::Update", expect = "53 55 56 57 8B 7C 24 14", this = "CWeapon")]
pub fn update(this: GPtr<u8>, ped: GPtr<u8>) {
    let ty = ru(this, 0) as i32;
    let info = if ped.is_null() {
        get_weapon_info(ty, 1)
    } else {
        let sk = ped_get_weapon_skill(ped, ty);
        get_weapon_info(ru(this, 0) as i32, sk)
    };
    match ru(this, 4) {
        1 => {
            if !ped.is_null() && matches!(ru(this, 0), 0x1B | 0x19) {
                let a = reload_loop_time_a_call(info, ped_flag26(ped));
                reload_sound_by_time(ped, ru(this, 0x10), a, 0x92);
                let b = reload_loop_time_b_call(info, ped_flag26(ped));
                reload_sound_by_time(ped, ru(this, 0x10), b, 0x93);
            }
            if time_ms() <= ru(this, 0x10) {
                return;
            }
            if ru(info, 0) != 0 && ru(this, 0xC) == 0 {
                wu(this, 4, 3);
            } else {
                wu(this, 4, 0);
            }
            return;
        }
        2 => {
            if !ped.is_null() && (ru(this, 0) as i32) < 0x2F {
                update_reload(this, ped, info);
            }
            if time_ms() > ru(this, 0x10) {
                reload_call(this, ped);
                wu(this, 4, 0);
            }
        }
        4 => wu(this, 4, 0),
        _ => {}
    }
    let fx = rp(this, 0x18);
    if !fx.is_null() && ru(this, 0) != 0x12 {
        fx_kill(fx);
        wu(this, 0x18, 0);
    }
}

/// The reloading branch of [`update`] (0x73DC75..0x73DE9C).
fn update_reload(this: GPtr<u8>, ped: GPtr<u8>, info: GPtr<u8>) {
    let anim_reload = (ru(info, 0x18) >> 12) & 1 != 0;
    let player_override = anim_reload && {
        let focus = usize::from(g::<u8>(0xB7CD74).read());
        g::<u8>(0xB7CEE5 + focus * 0x190).read() != 0 && ped_is_player(ped) != 0
    };
    if anim_reload && !player_override {
        let id = if (ru(info, 0x18) >> 12) & 1 != 0 { 0xE2 } else { 0 };
        let mut assoc = clump_get_association(rp(ped, 0x18), id);
        if assoc.is_null() {
            let crouch = get_crouch_reload_animation_id_call(info);
            assoc = clump_get_association(rp(ped, 0x18), crouch);
        }
        let has_gun_task = !intel_get_task_use_gun(rp(ped, 0x47C)).is_null();
        if !assoc.is_null() {
            let a = reload_loop_time_a_call(info, ped_flag26(ped));
            reload_sound_by_anim(ped, assoc, a, 0x92);
            let b = reload_loop_time_b_call(info, ped_flag26(ped));
            reload_sound_by_anim(ped, assoc, b, 0x93);
            let now = time_ms();
            if now > ru(this, 0x10) {
                let hier = rp(assoc, 0x14);
                let mut f = Fpu::new();
                f.fld(rf(hier, 0x10));
                f.fdivr_m(rf(assoc, 0x20));
                f.fcomp_m(f32::from_bits(0x3F66_6666));
                if !f.jp(5) {
                    wu(this, 0x10, now);
                }
            }
            return;
        }
        if has_gun_task {
            let now = time_ms();
            if now > ru(this, 0x10) {
                wu(this, 0x10, now);
            }
            return;
        }
    }
    let rt = crate::weapon::get_weapon_reload_time_call(info);
    let base = ru(this, 0x10).wrapping_sub(rt);
    let a = reload_loop_time_a_call(info, ped_flag26(ped));
    reload_sound_by_time(ped, base, a, 0x92);
    let b = reload_loop_time_b_call(info, ped_flag26(ped));
    reload_sound_by_time(ped, base, b, 0x93);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_IS_TYPE_MELEE,
    HOOK_IS_TYPE_2_HANDED,
    HOOK_IS_TYPE_PROJECTILE,
    HOOK_CAN_BE_USED_FOR_2_PLAYER,
    HOOK_HAS_WEAPON_AMMO_TO_BE_USED,
    HOOK_CAN_BE_USED_FOR_2_PLAYER_METHOD,
    HOOK_TARGET_WEAPON_RANGE_MULTIPLIER,
    HOOK_CONSTRUCTOR,
    HOOK_RELOAD,
    HOOK_STOP_WEAPON_EFFECT,
    HOOK_INITIALISE,
    HOOK_SHUTDOWN,
    HOOK_INITIALISE_WEAPONS,
    HOOK_SHUTDOWN_WEAPONS,
    HOOK_UPDATE_WEAPONS,
    HOOK_PROCESS_LINE_OF_SIGHT,
    HOOK_EVALUATE_TARGET_FOR_HEAT_SEEKING_MISSILE,
    HOOK_PICK_TARGET_FOR_HEAT_SEEKING_MISSILE,
    HOOK_ADD_GUNSHELL,
    HOOK_DO_WEAPON_EFFECT,
    HOOK_FIRE_FROM_CAR,
    HOOK_UPDATE,
];
