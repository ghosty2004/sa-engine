//! PORT-WEAPON-2 module `weapon_fire` (see `docs/arch/weapon-ped-port.md`, pass 2): the `CWeapon::Fire*`
//! family, ported from the 1.0 US bytes (`docs/re/specs/WEAPON/CWeapon.md` and `CWeapon.deviations.md`
//! as reading aids; gta-reversed `Weapon.cpp` only where it agrees with the bytes).
//!
//! Conventions as in pass 1: x87 arithmetic through [`X87`] / [`Fpu`] in the original instruction
//! order, `rand()` calls made directly by a port attributed to the original return address with
//! [`sa_core::orig::at_site`], every callee through `orig!` with its original address. SEH frames
//! (`fs:[0]`) of the originals are not reproduced (stack-only, invisible to callers).

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;
use sa_core::{hook, orig};

use crate::common::*;
use crate::weapon::{birds_handle_gun_shot, ped_is_player, shadows_gun_shot_sets_oil_on_fire, world_process_line_of_sight};

/// `CColPoint` (0x2C bytes).
const COL_POINT_SIZE: usize = 0x2C;
const _: () = assert!(COL_POINT_SIZE == sa_core::layouts::size::CColPoint);

/// Phases B/C path of [`target_weapon_range_multiplier_call`] (ST0 as f64).
pub fn target_weapon_range_multiplier_st0(victim: GPtr<u8>, owner: GPtr<u8>) -> f64 {
    crate::weapon::target_weapon_range_multiplier(victim, owner).to_f64()
}

/// No module globals of its own (the WEAPON arrays are declared by their owning modules).
pub const GLOBALS: &[GlobalDesc] = &[];

/// `CTimer::m_snTimeInMilliseconds` as a fresh read (the original re-reads it at every use).
#[inline]
fn now() -> u32 {
    time_ms()
}

/// The entity position the original selects with `m ? m + 0x30 : e + 4`.
#[inline]
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rp(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// "Ensure matrix": `if (!e->m_matrix) { e->AllocateMatrix(); e->m_placement.UpdateMatrix(e->m_matrix); }`.
fn ensure_matrix(e: GPtr<u8>) {
    if rp(e, 0x14).is_null() {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rp(e, 0x14));
    }
}

orig! {
    /// `CPlaceable::AllocateMatrix()` (0x54F560, thiscall)
    pub fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, thiscall)
    pub fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl(out, mat, vec), returns out)
    pub fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, cdecl(out, a, b), returns out)
    pub fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::Magnitude()` (0x4082C0, thiscall, ST0)
    pub fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `CTaskManager::GetActiveTask()` (0x681720, thiscall)
    pub fn task_manager_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10, thiscall)
    pub fn task_get_type(task: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CWorld::UseDetonator(CEntity*)` (0x5660B0)
    pub fn world_use_detonator(e: GPtr<u8>) = cdecl 0x5660B0 => unported;
    /// `CWeapon::TakePhotograph(CEntity* owner, CVector* point)` (0x73C1F0, thiscall)
    pub fn take_photograph(this: GPtr<u8>, owner: GPtr<u8>, point: GPtr<u8>) -> u8 = thiscall 0x73C1F0 => unported;
    /// `CAEPedWeaponAudioEntity::AddAudioEvent(int32)` (0x4E69F0, thiscall)
    pub fn ped_weapon_audio_add_event(ae: GPtr<u8>, ev: i32) = thiscall 0x4E69F0 => unported;
    /// `CPed::Say(uint16 phrase, uint32 offset, float volume, bool, bool, bool)` (0x5EFFE0, thiscall)
    pub fn ped_say(ped: GPtr<u8>, phrase: u32, offset: u32, vol: f32, a: u32, b: u32, c: u32) = thiscall 0x5EFFE0 => unported;
    /// `CPed::GiveWeapon(eWeaponType, uint32 ammo, bool)` (0x5E6080, thiscall)
    pub fn ped_give_weapon(ped: GPtr<u8>, ty: i32, ammo: u32, b: u32) -> i32 = thiscall 0x5E6080 => unported;
    /// `CPed::GetWeaponSlot(eWeaponType)` (0x5DF200, thiscall)
    pub fn ped_get_weapon_slot(ped: GPtr<u8>, ty: i32) -> i32 = thiscall 0x5DF200 => unported;
    /// `CPed::SetCurrentWeapon(eWeaponType)` (0x5E6280, thiscall)
    pub fn ped_set_current_weapon(ped: GPtr<u8>, ty: i32) = thiscall 0x5E6280 => unported;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    pub fn stats_increment_stat(stat: u32, v: f32) = cdecl 0x55C180 => unported;
    /// `CStats::GetPercentageProgress()` (0x5591E0, ST0)
    pub fn stats_get_percentage_progress() -> f64 = cdecl 0x5591E0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::Clear(bool enablePlayerControls, bool resetPhase)` (0x541A70, thiscall)
    pub fn pad_clear(pad: GPtr<u8>, a: u32, b: u32) = thiscall 0x541A70 => unported;
    /// `CWeaponInfo::GetWeaponReloadTime()` (0x743D70, thiscall)
    pub fn get_weapon_reload_time_call(info: GPtr<u8>) -> u32 = thiscall 0x743D70 => crate::weapon_info::get_weapon_reload_time;
    /// `CPed::GetWeaponSkill(eWeaponType)` (0x5E3B60, thiscall)
    pub fn ped_get_weapon_skill(ped: GPtr<u8>, ty: i32) -> u8 = thiscall 0x5E3B60 => unported;

    /// `CWeapon::FireInstantHit(CEntity*, CVector* origin, CVector* muzzle, CEntity* targetEnt, CVector* target,
    /// CVector* originOverride, bool forceAim, bool doMuzzleFx)` (0x73FB10, thiscall)
    pub fn fire_instant_hit_call(this: GPtr<u8>, firer: GPtr<u8>, origin: GPtr<u8>, muzzle: GPtr<u8>, target_ent: GPtr<u8>, target: GPtr<u8>, alt: GPtr<u8>, force_aim: u32, muzzle_fx: u32) -> u8 = thiscall 0x73FB10 => unported;
    /// `CWeapon::FireProjectile(CEntity*, CVector* origin, CEntity* targetEnt, CVector* target, float force)` (0x741360, thiscall)
    pub fn fire_projectile_call(this: GPtr<u8>, firer: GPtr<u8>, origin: GPtr<u8>, target_ent: GPtr<u8>, target: GPtr<u8>, force: f32) -> u8 = thiscall 0x741360 => unported;
    /// `CWeapon::FireSniper(CPed*, CEntity* targetEnt, CVector* target)` (0x73AAC0, thiscall)
    pub fn fire_sniper_call(this: GPtr<u8>, firer: GPtr<u8>, target_ent: GPtr<u8>, target: GPtr<u8>) -> u8 = thiscall 0x73AAC0 => fire_sniper;
    /// `CWeapon::FireAreaEffect(CEntity*, CVector* origin, CEntity* targetEnt, CVector* target)` (0x73E800, thiscall)
    pub fn fire_area_effect_call(this: GPtr<u8>, firer: GPtr<u8>, origin: GPtr<u8>, target_ent: GPtr<u8>, target: GPtr<u8>) -> u8 = thiscall 0x73E800 => unported;
    /// `CBulletInfo::AddBullet(CEntity* creator, eWeaponType, CVector pos, CVector velocity)` (0x736010)
    pub fn add_bullet_call(creator: GPtr<u8>, ty: i32, px: f32, py: f32, pz: f32, vx: f32, vy: f32, vz: f32) -> u8 = cdecl 0x736010 => crate::bullet_shot::add_bullet;
    /// `CVector::Normalise()` (0x59C910, thiscall)
    pub fn normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CPad::StartShake_Distance(int16 time, uint8 freq, float x, float y, float z)` (0x53F9A0, thiscall)
    pub fn pad_start_shake_distance(pad: GPtr<u8>, time: u32, freq: u32, x: f32, y: f32, z: f32) = thiscall 0x53F9A0 => unported;
    /// `CCamera::CamShakeNoPos(CCamera*, float)` (0x50A970, cdecl)
    pub fn cam_shake_no_pos(cam: GPtr<u8>, strength: f32) = cdecl 0x50A970 => unported;
    /// `CCrime::ReportCrime(eCrimeType, CEntity* victim, CPed* commitedBy)` (0x532010)
    pub fn report_crime(ty: i32, victim: GPtr<u8>, by: GPtr<u8>) = cdecl 0x532010 => unported;
    /// `CEventGunShot::CEventGunShot(CEntity*, CVector start, CVector end, bool silenced)` (0x4AC610, thiscall `ret 0x20`)
    pub fn event_gun_shot_ctor(ev: GPtr<u8>, e: GPtr<u8>, sx: f32, sy: f32, sz: f32, ex: f32, ey: f32, ez: f32, silenced: u32) -> GPtr<u8> = thiscall 0x4AC610 => unported;
    /// `CEventGunShotWhizzedBy::CEventGunShotWhizzedBy(CEntity*, const CVector&, const CVector&, bool)` (0x4B1FD0, thiscall)
    pub fn event_whizzed_by_ctor(ev: GPtr<u8>, e: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>, silenced: u32) -> GPtr<u8> = thiscall 0x4B1FD0 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50)
    pub fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, thiscall)
    pub fn event_group_add(group: GPtr<u8>, ev: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CInterestingEvents::Add(EType, CEntity*)` (0x602590, thiscall on `g_InterestingEvents` 0xC0B058)
    pub fn interesting_events_add(ie: GPtr<u8>, ty: i32, e: GPtr<u8>) = thiscall 0x602590 => unported;
    /// `CEventGunShotWhizzedBy::~CEventGunShotWhizzedBy()` (0x4B2070, thiscall)
    pub fn event_whizzed_by_dtor(ev: GPtr<u8>) = thiscall 0x4B2070 => unported;
    /// `CEventGunShot::~CEventGunShot()` (0x4AC720, thiscall)
    pub fn event_gun_shot_dtor(ev: GPtr<u8>) = thiscall 0x4AC720 => unported;
    /// `CWeapon::CheckForShootingVehicleOccupant(CEntity**, CColPoint*, eWeaponType, const CVector*, const CVector*)` (0x73F480)
    pub fn check_for_shooting_vehicle_occupant_call(e: GPtr<u8>, cp: GPtr<u8>, ty: i32, origin: GPtr<u8>, target: GPtr<u8>) -> u8 = cdecl 0x73F480 => check_for_shooting_vehicle_occupant;
    /// `CColPoint::operator=(const CColPoint&)` (0x40FC80, thiscall; a stub into `.HOODLUM`)
    pub fn col_point_assign(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x40FC80 => unported;
    /// `CColLine::CColLine(const CVector& start, const CVector& end)` (0x40EF50, thiscall)
    pub fn col_line_ctor(this: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) -> GPtr<u8> = thiscall 0x40EF50 => unported;
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    pub fn get_anim_hierarchy_from_skin_clump(clump: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0)
    pub fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120)
    pub fn rp_hanim_hierarchy_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `RwV3dTransformPoints(RwV3d* out, const RwV3d* in, int32 n, const RwMatrix*)` (0x7EDD90)
    pub fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `CColSphere::Set(float radius, const CVector& center, uint8 material, uint8 piece, uint8 light)` (0x40FD10, thiscall)
    pub fn col_sphere_set(this: GPtr<u8>, radius: f32, center: GPtr<u8>, material: u32, piece: u32, light: u32) = thiscall 0x40FD10 => unported;
    /// `CCollision::ProcessLineSphere(const CColLine&, const CColSphere&, CColPoint&, float& depth)` (0x412AA0)
    pub fn process_line_sphere(line: GPtr<u8>, sphere: GPtr<u8>, cp: GPtr<u8>, depth: GPtr<u8>) -> u8 = cdecl 0x412AA0 => unported;
    /// `CEntity::GetColModel()` (0x535300, thiscall)
    pub fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CMatrix::Invert(const CMatrix& in, CMatrix& out)` (0x59B920, returns out)
    pub fn matrix_invert(inp: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = cdecl 0x59B920 => unported;
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0, thiscall)
    pub fn matrix_copy_ctor(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BCF0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0, thiscall)
    pub fn matrix_dtor(this: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CCollision::CalculateTrianglePlanes(CColModel*)` (0x418580)
    pub fn calculate_triangle_planes(cm: GPtr<u8>) = cdecl 0x418580 => unported;
    /// `SurfaceInfos_c::IsGlass(uint32)` (0x55E790, thiscall on `g_surfaceInfos` 0xB79538; the caller tests AL)
    pub fn surface_is_glass(si: GPtr<u8>, id: u32) -> u32 = thiscall 0x55E790 => unported;
    /// `CCollision::TestLineTriangle(const CColLine&, const CompressedVector*, const CColTriangle&,
    /// const CColTrianglePlane&)` (0x413AC0)
    pub fn test_line_triangle(line: GPtr<u8>, verts: GPtr<u8>, tri: GPtr<u8>, plane: GPtr<u8>) -> u8 = cdecl 0x413AC0 => sa_world::collision::mvpd_coll::test_line_triangle;
    /// `CDamageManager::ProgressPanelDamage(ePanels)` (0x6C23C0, thiscall)
    pub fn progress_panel_damage(dm: GPtr<u8>, panel: u32) -> u8 = thiscall 0x6C23C0 => unported;
    /// `CDamageManager::GetPanelStatus(ePanels)` (0x6C2180, thiscall)
    pub fn get_panel_status(dm: GPtr<u8>, panel: u32) -> i32 = thiscall 0x6C2180 => unported;
    /// `CAutomobile::SetPanelDamage(ePanels, bool)` (0x6B1480, thiscall)
    pub fn automobile_set_panel_damage(veh: GPtr<u8>, panel: u32, b: u32) = thiscall 0x6B1480 => unported;
    /// `CWeapon::DoBulletImpact(CEntity* owner, CEntity* victim, const CVector* start, const CVector* end, const CColPoint*, int32)` (0x73B550, thiscall)
    pub fn do_bullet_impact(this: GPtr<u8>, owner: GPtr<u8>, victim: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>, cp: GPtr<u8>, inc: i32) = thiscall 0x73B550 => crate::weapon_impact::do_bullet_impact;
    /// `CWeapon::TargetWeaponRangeMultiplier(CEntity* victim, CEntity* owner)` (0x73B380, ST0)
    pub fn target_weapon_range_multiplier_call(victim: GPtr<u8>, owner: GPtr<u8>) -> f64 = cdecl 0x73B380 => target_weapon_range_multiplier_st0;
    /// `CPad::StartShake(int16 time, uint8 freq, uint32 delay)` (0x53F920, thiscall)
    pub fn pad_start_shake(pad: GPtr<u8>, time: u32, freq: u32, delay: u32) = thiscall 0x53F920 => unported;
    /// `CPointLights::AddLight(uint8 type, CVector pos, CVector dir, float radius, float r, float g, float b,
    /// uint8 fogType, bool extraShadows, CEntity* attachedTo)` (0x7000E0)
    pub fn point_lights_add_light(ty: u32, px: f32, py: f32, pz: f32, dx: f32, dy: f32, dz: f32, radius: f32, r: f32, g: f32, b: f32, fog: u32, shadows: u32, attached: GPtr<u8>) = cdecl 0x7000E0 => unported;
    /// `CWorld::ResetLineTestOptions()` (0x5631C0)
    pub fn world_reset_line_test_options() = cdecl 0x5631C0 => unported;
    /// `CWeapon::FireM16_1stPerson(CPed*)` (0x741C00, thiscall)
    pub fn fire_m16_1st_person_call(this: GPtr<u8>, firer: GPtr<u8>) -> u8 = thiscall 0x741C00 => fire_m16_1st_person;
}

/// `CWeapon::Fire(CEntity* firedBy, CVector* startPosn, CVector* shotOrigin, CEntity* targetEnt, CVector*
/// targetPosn, CVector* altPosn)` (0x742300, thiscall `ret 0x18`): the dispatcher behind every on-foot
/// shot (SA:MP syncs its arguments).
///
/// 1. `info = GetWeaponInfo(type, 1)`; for a ped shooter `GetWeaponInfo(type, firedBy->GetWeaponSkill(type))`.
/// 2. Without `startPosn` both positions become `firedBy->matrix * (0, 0, 0.6)` (ensure matrix first).
/// 3. With the byte +0x14 set, `startPosn` is moved by `0.15 * (-sin a, cos a)` with
///    `a = heading * 57.295776f * 0.017453216f` (x87, unrounded; heading = `atan2(-fwd.x, fwd.y)` or the
///    placement heading).
/// 4. State must be 0 or 1; an empty clip is refilled from the total (`min`, unsigned) or the call fails.
/// 5. Type switch (jump tables 0x742A30 / 0x742A0C): projectiles, instant-hit (M16 first person and
///    sniper camera variants), rocket launchers (a ped closer than 8 m to its target only fires as the
///    player), area effects, detonator, camera. **Kept original behaviour** (deviation from
///    gta-reversed's `NOTSA_UNREACHABLE`): any other type "fires" iff the low byte of the `targetPosn`
///    pointer is non-zero (`mov bl, [esp+0x60]` at 0x742794) and then runs the ammo/state code.
/// 6. After a shot: ped shooter flag +0x46C |= 0x10000 (not for the camera), weapon audio event 0x91, the
///    player's taunt `Say(0xB6, 200, 1.0)` at a ped target (not for the silenced pistol); the detonator
///    swap for satchels; shot stats 0x7E/0x7F; ammo use (unless the infinite-ammo cheat 0x969178);
///    state 1 and the next shot time (reload time, quartered for the player with the fast-reload flag;
///    the anim loop `ftol((end - start) * -900.0f)` before now; camera +1100 ms).
/// 7. Unarmed (0) and the baseball bat (5) always return true.
#[hook(addr = 0x742300, conv = thiscall, name = "CWeapon::Fire", expect = "83 EC 3C 53 56", this = "CWeapon")]
#[allow(clippy::too_many_arguments)]
pub fn fire(this: GPtr<u8>, fired_by: GPtr<u8>, start: GPtr<u8>, shot_origin: GPtr<u8>, target_ent: GPtr<u8>, target: GPtr<u8>, alt: GPtr<u8>) -> u8 {
    let mut finished_anim = true; // [esp+0xF] (byte), "set the next shot from the anim loop"
    let mut local: [u32; 3] = [0, 0, 0x3F19_999A];
    let mut info = get_weapon_info(ru(this, 0) as i32, 1);
    let mut ped_owner = GPtr::<u8>::NULL;
    if !fired_by.is_null() && rb(fired_by, 0x36) & 7 == 3 {
        ped_owner = fired_by;
        let sk = ped_get_weapon_skill(fired_by, ru(this, 0) as i32);
        info = get_weapon_info(ru(this, 0) as i32, sk);
    }
    let (start, muzzle) = if start.is_null() {
        ensure_matrix(fired_by);
        let mut tmp = [0u32; 3];
        let r = matrix_mul_vec(lp(&mut tmp).cast(), rp(fired_by, 0x14), lp(&mut local).cast());
        local = [ru(r, 0), ru(r, 4), ru(r, 8)];
        let p: GPtr<u8> = lp(&mut local).cast();
        (p, p)
    } else {
        (start, shot_origin)
    };
    if rb(this, 0x14) != 0 {
        let m = rp(fired_by, 0x14);
        let h = if m.is_null() { x(rf(fired_by, 0x10)) } else { x(rf(m, 0x10)).fchs().fpatan(x(rf(m, 0x14))) };
        let a = h.fmul_m(cf(0x4265_2EE0)).fmul_m(cf(0x3C8E_FA0C));
        let k = cf(0x3E19_999A);
        wf(start, 0, a.fsin().fmul_m(k).fsubr_m(rf(start, 0)).to_f32());
        wf(start, 4, a.fcos().fmul_m(k).fadd_m(rf(start, 4)).to_f32());
    }
    let st = ru(this, 4);
    if st != 0 && st != 1 {
        return 0;
    }
    if (ru(this, 8) as i32) <= 0 {
        if (ru(this, 0xC) as i32) <= 0 || st == 2 {
            return 0;
        }
        let clip = i32::from(rh(get_weapon_info(ru(this, 0) as i32, 1), 0x20) as i16) as u32;
        let total = ru(this, 0xC);
        wu(this, 8, if total < clip { total } else { clip });
    }
    let ty = ru(this, 0);
    let cam_weapon_mode = || g::<u16>(0xB6F858).read();
    let fired: u8;
    match ty {
        0x19..=0x1B => {
            fired = fire_instant_hit_call(this, fired_by, start, muzzle, target_ent, target, alt, 0, 1);
            finished_anim = true;
        }
        0x22 => {
            fired = if !ped_owner.is_null() && ru(ped_owner, 0x598) == 0 && cam_weapon_mode() == 7 {
                fire_sniper_call(this, fired_by, target_ent, target)
            } else {
                fire_instant_hit_call(this, fired_by, start, muzzle, target_ent, target, GPtr::NULL, 0, 1)
            };
        }
        0x16..=0x18 | 0x1C..=0x21 | 0x26 => {
            if !ped_owner.is_null() && ru(ped_owner, 0x598) == 0 && matches!(cam_weapon_mode(), 0x22 | 0x2D) {
                fired = fire_m16_1st_person_call(this, fired_by);
                finished_anim = true;
            } else {
                fired = fire_instant_hit_call(this, fired_by, start, muzzle, target_ent, target, alt, 0, 1);
                if !ped_owner.is_null() {
                    if ru(ped_owner, 0x46C) & 0x100 == 0 {
                        finished_anim = false;
                    } else {
                        let tm = rp(ped_owner, 0x47C).byte_add(4);
                        if !task_manager_get_active_task(tm).is_null() {
                            let t = task_manager_get_active_task(rp(ped_owner, 0x47C).byte_add(4));
                            if task_get_type(t) == 0x3FE {
                                finished_anim = false;
                            }
                        }
                    }
                }
            }
        }
        0x23 | 0x24 => {
            if rb(fired_by, 0x36) & 7 == 3 && (!target_ent.is_null() || !target.is_null()) {
                let mut tmp = [0u32; 3];
                let r = if !target_ent.is_null() {
                    vector_sub(lp(&mut tmp).cast(), ent_pos(target_ent), ent_pos(fired_by))
                } else {
                    vector_sub(lp(&mut tmp).cast(), target, ent_pos(fired_by))
                };
                let mut d = [ru(r, 0), ru(r, 4), ru(r, 8)];
                let mut f = Fpu::new();
                f.push(st0(vector_magnitude(lp(&mut d).cast())));
                f.fcomp_m(8.0);
                if f.jne(0x41) && ped_is_player(fired_by) == 0 {
                    return fire_result(this, 0);
                }
            }
            fired = fire_projectile_call(this, fired_by, muzzle, target_ent, target, 0.0);
        }
        0x10..=0x12 | 0x27 => {
            fired = if !target.is_null() {
                let mut d = [0u32; 3];
                vector_sub(lp(&mut d).cast(), target, ent_pos(fired_by));
                let mut f = Fpu::new();
                f.push(st0(vector_magnitude(lp(&mut d).cast())));
                f.fsub_m(10.0);
                f.fmul_m(cf(0x3DCC_CCCD));
                let mut force = f.fst();
                f.fcomp_m(cf(0x3E4C_CCCD));
                if !f.jp(5) {
                    force = cf(0x3E4C_CCCD);
                } else {
                    let mut f = Fpu::new();
                    f.fld(force);
                    f.fcomp_m(1.0);
                    if !f.jne(0x41) {
                        force = 1.0;
                    }
                }
                fire_projectile_call(this, fired_by, muzzle, target_ent, target, force)
            } else if fired_by == find_player_ped(-1) {
                let force = x(rf(rp(fired_by, 0x480), 0x2C)).fmul_m(cf(0x3D19_999A)).to_f32();
                fire_projectile_call(this, fired_by, muzzle, target_ent, GPtr::NULL, force)
            } else {
                fire_projectile_call(this, fired_by, muzzle, target_ent, GPtr::NULL, cf(0x3E99_999A))
            };
        }
        0x25 | 0x29 | 0x2A => {
            fired = fire_area_effect_call(this, fired_by, muzzle, target_ent, target);
            finished_anim = false;
        }
        0x28 => {
            world_use_detonator(fired_by);
            wu(this, 0xC, 1);
            wu(this, 8, 1);
            return after_shot(this, fired_by, target_ent, info, 1, finished_anim);
        }
        0x2B => {
            fired = take_photograph(this, fired_by, muzzle);
        }
        // 0..0xF, 0x13..0x15, > 0x2B: the original takes the low byte of `targetPosn` as the result.
        _ => {
            fired = target.addr() as u8;
        }
    }
    if fired == 0 {
        return fire_result(this, fired);
    }
    after_shot(this, fired_by, target_ent, info, fired, finished_anim)
}

/// The exit at 0x7429F2: unarmed (0) and the bat (5) report true, every other type `fired`.
fn fire_result(this: GPtr<u8>, fired: u8) -> u8 {
    let ty = ru(this, 0);
    if ty == 0 || ty == 5 { 1 } else { fired }
}

/// `CWeapon::Fire` from 0x7427A0 (a shot was fired).
fn after_shot(this: GPtr<u8>, fired_by: GPtr<u8>, target_ent: GPtr<u8>, info: GPtr<u8>, fired: u8, finished_anim: bool) -> u8 {
    let mut by_player = false; // the reused argument byte [esp+0x60]
    if rb(fired_by, 0x36) & 7 == 3 {
        if ru(this, 0) != 0x2B {
            wu(fired_by, 0x46C, ru(fired_by, 0x46C) | 0x10000);
            if ped_is_player(fired_by) != 0 {
                by_player = true;
            }
        }
        ped_weapon_audio_add_event(fired_by.byte_add(0x394), 0x91);
        if by_player && !target_ent.is_null() && rb(target_ent, 0x36) & 7 == 3 && ru(this, 0) != 0x17 {
            ped_say(fired_by, 0xB6, 200, 1.0, 0, 0, 0);
        }
    }
    if ru(this, 0) == 0x27 {
        ped_give_weapon(fired_by, 0x28, 1, 1);
        let s = ped_get_weapon_slot(fired_by, 0x27);
        if ru(fired_by, (s.wrapping_mul(0x1C) as isize).wrapping_add(0x5AC)) as i32 <= 1 {
            let s2 = ped_get_weapon_slot(fired_by, 0x28);
            wu(fired_by, (s2.wrapping_mul(0x1C) as isize).wrapping_add(0x5A4), 0);
            ped_set_current_weapon(fired_by, 0x28);
        }
    }
    if by_player {
        // Byte table 0x742A58 / jump table 0x742A4C over type - 0x10 (0..0x18).
        let stat = match ru(this, 0) {
            0x10 | 0x12 | 0x13 | 0x23 | 0x24 | 0x27 | 0x28 => Some(0x7F),
            0x16..=0x22 | 0x26 => Some(0x7E),
            _ => None,
        };
        if let Some(s) = stat {
            stats_increment_stat(s, 1.0);
        }
    }
    if g::<u8>(0x969178).read() == 0 {
        let clip = ru(this, 8) as i32;
        if clip > 0 {
            wu(this, 8, (clip - 1) as u32);
        }
        let total = ru(this, 0xC) as i32;
        if total > 0 {
            let dec = if !by_player {
                total < 25_000
            } else {
                let mut f = Fpu::new();
                f.push(st0(stats_get_percentage_progress()));
                f.fcomp_m(100.0);
                // `test ah, 5; jnp 0x7428E6`: below 100 % decrements; else only the detonator.
                !f.jp(5) || ru(this, 0) == 0x28
            };
            if dec {
                wu(this, 0xC, ru(this, 0xC).wrapping_sub(1));
            }
        }
    }
    wu(this, 4, 1);
    if ru(this, 8) == 0 {
        if ru(this, 0xC) == 0 {
            let cam = usize::from(g::<u8>(0xB6F081).read());
            if g::<u16>(0xB6F1A8 + cam * 0x238).read() == 0x2E {
                pad_clear(get_pad(0), 0, 1);
            }
            return 1;
        }
        wu(this, 4, 2);
        let t = now();
        wu(this, 0x10, get_weapon_reload_time_call(info).wrapping_add(t));
        if fired_by != find_player_ped(-1) {
            return 1;
        }
        let focus = usize::from(g::<u8>(0xB7CD74).read());
        if g::<u8>(0xB7CEE5 + focus * 0x190).read() == 0 {
            return 1;
        }
        let t = now();
        let rt = get_weapon_reload_time_call(info) as i32;
        wu(this, 0x10, (rt / 4).wrapping_add(t as i32) as u32);
        return 1;
    }
    if finished_anim {
        if ru(this, 0) == 0x2B {
            wu(this, 0x10, now().wrapping_add(0x44C));
        } else {
            let v = x(rf(info, 0x44)).fsub_m(rf(info, 0x40)).fmul_m(cf(0xC461_0000));
            wu(this, 0x10, now().wrapping_sub(ftol32(v) as u32));
        }
    } else {
        wu(this, 0x10, now());
    }
    fire_result(this, fired)
}

/// `CEventGunShot` / `CEventGunShotWhizzedBy` (0x34 bytes each, built on the caller's stack).
const EVENT_SIZE: usize = 0x34;
const _: () = assert!(EVENT_SIZE == sa_core::layouts::size::CEventGunShot);
const _: () = assert!(EVENT_SIZE == sa_core::layouts::size::CEventGunShotWhizzedBy);

/// Active camera's `m_aCams[n]` base offset (`m_nActiveCam` 0xB6F081, stride 0x238).
fn active_cam_off() -> usize {
    usize::from(g::<u8>(0xB6F081).read()) * 0x238
}

/// `S + D*40` as the original builds it twice in `FireSniper` (x87: `z = D.z*k` kept, `y = D.y*k` and
/// `x = D.x*k` stored f32, then `z + S.z`, `S.y + y`, `x + S.x`).
fn sniper_end(s: [f32; 3], d: [f32; 3]) -> [f32; 3] {
    let k = cf(0x4220_0000);
    let z = x(d[2]).fmul_m(k);
    let y = x(d[1]).fmul_m(k).to_f32();
    let xx = x(d[0]).fmul_m(k).to_f32();
    let ez = z.fadd_m(s[2]).to_f32();
    let ey = x(s[1]).fadd_m(y).to_f32();
    let ex = x(xx).fadd_m(s[0]).to_f32();
    [ex, ey, ez]
}

/// `CWeapon::FireSniper(CPed* shooter, CEntity* victim, CVector* target)` (0x73AAC0, thiscall `ret 0xC`;
/// `victim` and `target` are not read). For the player only in camera modes 0x22, 7, 0x2E, 8, 0x33,
/// 0x2A, 0x27, 0x28, 0x34 (else false). `GetWeaponInfo(type, 1)` is called and its result discarded.
/// With the active camera's front F and source S: when `(F.z*0.145f - F.y*0.9894f) + F.x*0.0f > 0.997f`
/// the counter 0x8D4B60 steps `(n + 1) & 7`. `D = normalise(F) * 16` (stored over D),
/// `AddBullet(shooter, type, S, D)`; for ped types 0/1 the pad of that ped type shakes
/// (`StartShake_Distance(240, 128, player pos)`, three `FindPlayerPed` calls) and `CamShakeNoPos(0.2)`;
/// crime 1 is reported (ped: by itself; vehicle with a driver: by the driver). Then a `CEventGunShot`
/// from S to `S + D*40` (silenced for types 0x17 and 0x11), D normalised again, a
/// `CEventGunShotWhizzedBy` from S to `S + D*40` (same flag), `g_InterestingEvents.Add(0x16)`; true.
/// The SEH frame (fs:[0], state byte) is not reproduced; the events live in a Rust stack buffer.
#[hook(addr = 0x73AAC0, conv = thiscall, name = "CWeapon::FireSniper", expect = "6A FF 64 A1 00 00 00 00", this = "CWeapon")]
pub fn fire_sniper(this: GPtr<u8>, shooter: GPtr<u8>, _victim: GPtr<u8>, _target: GPtr<u8>) -> u8 {
    if find_player_ped(-1) == shooter {
        let mode = g::<u16>(0xB6F1A8 + active_cam_off()).read();
        if !matches!(mode, 0x22 | 7 | 0x2E | 8 | 0x33 | 0x2A | 0x27 | 0x28 | 0x34) {
            return 0;
        }
    }
    get_weapon_info(ru(this, 0) as i32, 1);
    let c = active_cam_off();
    let front = 0xB6F32C + c;
    let v = x(g::<f32>(front + 8).read())
        .fmul_m(cf(0x3E14_7AE1))
        .fsub(x(g::<f32>(front + 4).read()).fmul_m(cf(0x3F7D_4952)));
    let mut f = Fpu::new();
    f.push(v.fadd(x(g::<f32>(front).read()).fmul_m(0.0)));
    f.fcomp_m(cf(0x3F7F_3B64));
    let s: [u32; 3] = g::<[u32; 3]>(0xB6F338 + c).read();
    let mut d: [u32; 3] = g::<[u32; 3]>(front).read();
    if !f.jne(0x41) {
        let n = g::<u32>(0x8D4B60);
        n.write(n.read().wrapping_add(1) & 7);
    }
    normalise(lp(&mut d).cast());
    let k16 = cf(0x4180_0000);
    for c in d.iter_mut() {
        *c = x(f32::from_bits(*c)).fmul_m(k16).to_f32().to_bits();
    }
    let sf = s.map(f32::from_bits);
    let df = d.map(f32::from_bits);
    add_bullet_call(shooter, ru(this, 0) as i32, sf[0], sf[1], sf[2], df[0], df[1], df[2]);
    let pt = ru(shooter, 0x598);
    if pt == 0 || pt == 1 {
        let a = ent_pos(find_player_ped(-1));
        let b = ent_pos(find_player_ped(-1));
        let c = ent_pos(find_player_ped(-1));
        let (pz, py, px) = (rf(a, 8), rf(b, 4), rf(c, 0));
        pad_start_shake_distance(get_pad(ru(shooter, 0x598) as i32), 0xF0, 0x80, px, py, pz);
        cam_shake_no_pos(g(0xB6F028), cf(0x3E4C_CCCD));
    }
    match etype(shooter) {
        3 => report_crime(1, shooter, shooter),
        2 => {
            let drv = rp(shooter, 0x460);
            if !drv.is_null() {
                report_crime(1, shooter, drv);
            }
        }
        _ => {}
    }
    let silenced = || u32::from(matches!(ru(this, 0), 0x17 | 0x11));
    let e = sniper_end(sf, df);
    let mut shot = [0u32; EVENT_SIZE / 4];
    event_gun_shot_ctor(lp(&mut shot).cast(), shooter, sf[0], sf[1], sf[2], e[0], e[1], e[2], silenced());
    event_group_add(get_event_global_group(), lp(&mut shot).cast(), 0);
    normalise(lp(&mut d).cast());
    let mut s2 = s;
    let mut e2 = sniper_end(sf, d.map(f32::from_bits));
    let mut whizz = [0u32; EVENT_SIZE / 4];
    event_whizzed_by_ctor(lp(&mut whizz).cast(), shooter, lp(&mut s2).cast(), lp(&mut e2).cast(), silenced());
    event_group_add(get_event_global_group(), lp(&mut whizz).cast(), 0);
    interesting_events_add(g(0xC0B058), 0x16, shooter);
    event_whizzed_by_dtor(lp(&mut whizz).cast());
    event_gun_shot_dtor(lp(&mut shot).cast());
    1
}

/// `CWeapon::FireM16_1stPerson(CPed* shooter)` (0x741C00, thiscall `ret 4`): only in active camera modes
/// 0x22, 7, 0x2E, 8, 0x33, 0x2A, 0x27, 0x28, 0x34, 0x2D (else false). `E = S + (F*3)*range` from the
/// active camera's source S and front F (x87: `F.x*3*R` and `F.z*3` stored f32, `F.y*3*R` kept);
/// `bIncludeDeadPeds`, `bIncludeCarTyres`, `bIncludeBikers` = 1 and `pIgnoreEntity = shooter` around
/// `HandleGunShot`, `GunShotSetsOilOnFire` and `ProcessLineOfSight(S, E, cp, hit, 1,1,1,1,1, 0, 0, 1)`; on a
/// hit `CheckForShootingVehicleOccupant`. A hit farther (2D, `sqrt(dy*dy + dx*dx)`) than
/// `TargetWeaponRangeMultiplier(hit, shooter) * range` is dropped. `DoBulletImpact(shooter, hit, S, E, cp,
/// 0)`. For ped types 0/1: camera recoil with `k` = 0.00015 (type 30), 0.0003 (31), else 0.0002, times 0.3
/// when the player ducks or is attached; **kept original** (gta-reversed rows Weapon.cpp:1633/1634,
/// CONFIRMED deviations): `cam+0xBC += (float)((rand() & 0x7F) - 64) * k`, then `cam+0xAC` with a second
/// `rand()` (gr uses `GetRandomNumberInRange(-64, 64)`). Pad shake: `n = ftol((loopEnd - loopStart) * 900)`,
/// `v = clamp((20.0 - n) * 0.0125, 0.0, 1.0)` (f64 constants), `freq = (u8)ftol(v * 80.0 + 130.0)`,
/// `time = ftol(timeStep * 20000.0f / freq)`, `GetPad(pedType)->StartShake(time, freq', 0)` where `freq'`
/// is the dword whose low byte is `freq` and whose upper bytes are those of the second recoil value
/// (`(rand() & 0x7F) - 64`, the stack slot the original reuses). The original writes `range` and `k` into
/// its own argument slot (not reproduced, stack-only). True.
#[hook(addr = 0x741C00, conv = thiscall, name = "CWeapon::FireM16_1stPerson", expect = "0F B6 05 81 F0 B6 00", this = "CWeapon")]
pub fn fire_m16_1st_person(this: GPtr<u8>, shooter: GPtr<u8>) -> u8 {
    let mode = g::<u16>(0xB6F1A8 + active_cam_off()).read();
    if !matches!(mode, 0x22 | 7 | 0x2E | 8 | 0x33 | 0x2A | 0x27 | 0x28 | 0x34 | 0x2D) {
        return 0;
    }
    let info = get_weapon_info(ru(this, 0) as i32, 1);
    let c = active_cam_off();
    let mut s: [u32; 3] = g::<[u32; 3]>(0xB6F338 + c).read();
    let range = rf(info, 8);
    let k3 = cf(0x4040_0000);
    let front = 0xB6F32C + c;
    let fx3 = x(g::<f32>(front).read()).fmul_m(k3);
    let fy3 = x(g::<f32>(front + 4).read()).fmul_m(k3);
    let fz3 = x(g::<f32>(front + 8).read()).fmul_m(k3).to_f32();
    let r = x(range);
    let fx3r = fx3.fmul(r).to_f32();
    let fy3r = fy3.fmul(r);
    let fz3r = x(fz3).fmul(r).to_f32();
    let sf = s.map(f32::from_bits);
    let ex = x(fx3r).fadd_m(sf[0]).to_f32();
    let ey = x(sf[1]).fadd(fy3r).to_f32();
    let ez = x(sf[2]).fadd_m(fz3r).to_f32();
    let mut e = [ex, ey, ez];
    g::<u8>(0xB7CD71).write(1); // CWorld::bIncludeDeadPeds
    g::<u8>(0xB7CD70).write(1); // CWorld::bIncludeCarTyres
    g::<u8>(0xB7CD6F).write(1); // CWorld::bIncludeBikers
    g::<GPtr<u8>>(0xB7CD68).write(shooter); // CWorld::pIgnoreEntity
    let mut cp = [0u32; COL_POINT_SIZE / 4];
    let mut hit = GPtr::<u8>::NULL;
    let (sp, ep): (GPtr<u8>, GPtr<u8>) = (lp(&mut s).cast(), lp(&mut e).cast());
    birds_handle_gun_shot(sp, ep);
    shadows_gun_shot_sets_oil_on_fire(sp, ep);
    if world_process_line_of_sight(sp, ep, lp(&mut cp).cast(), lp(&mut hit).cast(), 1, 1, 1, 1, 1, 0, 0, 1) != 0 {
        check_for_shooting_vehicle_occupant_call(lp(&mut hit).cast(), lp(&mut cp).cast(), ru(this, 0) as i32, sp, ep);
    }
    g::<u8>(0xB7CD71).write(0);
    g::<u8>(0xB7CD70).write(0);
    g::<u8>(0xB7CD6F).write(0);
    g::<u32>(0xB7CD68).write(0);
    if !hit.is_null() {
        let cpf = |i: usize| f32::from_bits(cp[i]);
        let dx = x(cpf(0)).fsub_m(f32::from_bits(s[0]));
        let dy = x(cpf(1)).fsub_m(f32::from_bits(s[1]));
        let dist = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt().to_f32();
        let mut f = Fpu::new();
        f.push(st0(target_weapon_range_multiplier_call(hit, shooter)));
        f.fmul_m(range);
        f.fcomp_m(dist);
        if !f.jp(5) {
            hit = GPtr::NULL;
        }
    }
    do_bullet_impact(this, shooter, hit, sp, ep, lp(&mut cp).cast(), 0);
    let pt = ru(shooter, 0x598);
    if pt != 0 && pt != 1 {
        return 1;
    }
    let mut k = match ru(this, 0) {
        0x1E => cf(0x391D_4952),
        0x1F => cf(0x399D_4952),
        _ => cf(0x3951_B717),
    };
    if ru(find_player_ped(-1), 0x46C) & 0x0400_0000 != 0 || !rp(find_player_ped(-1), 0xFC).is_null() {
        k = x(k).fmul_m(cf(0x3E99_999A)).to_f32();
    }
    let r1 = sa_core::orig::at_site(0x741EAE, rand);
    let v1 = (r1 & 0x7F) - 0x40;
    let a = g::<f32>(0xB6F258 + active_cam_off());
    a.write(xi(v1).fmul_m(k).fadd_m(a.read()).to_f32());
    let r2 = sa_core::orig::at_site(0x741EDC, rand);
    let v2 = (r2 & 0x7F) - 0x40;
    let b = g::<f32>(0xB6F248 + active_cam_off());
    b.write(xi(v2).fmul_m(k).fadd_m(b.read()).to_f32());
    let n = ftol32(x(rf(info, 0x44)).fsub_m(rf(info, 0x40)).fmul_m(cf(0x4461_0000)));
    let mut v = X87::from_f64_exact(20.0).fsub(xi(n)).fmul(X87::from_f64_exact(0.0125));
    let (lo, hi) = (X87::from_f64_exact(0.0), X87::from_f64_exact(1.0));
    // 0x741F2A..0x741F6D: `fld lo; fcomp st1` (`ah & 0x41`, je), then `fld hi; fcomp st1` (`ah & 5`, jp),
    // then the `lo` test again at 0x741F56.
    if !lo.fcom(v).test_any(0x41) {
        v = lo;
    } else if !hi.fcom(v).test_parity(5) {
        v = hi;
    } else if !lo.fcom(v).test_any(0x41) {
        v = lo;
    }
    let freq = ftol32(v.fmul(X87::from_f64_exact(80.0)).fadd(X87::from_f64_exact(130.0))) as u8;
    let t = ftol32(x(time_step()).fmul_m(cf(0x469C_4000)).fdiv(X87::from_i32(i32::from(freq))));
    // The original stores AL into the low byte of [esp+0x14], which still holds v2, and pushes the dword.
    let freq_dword = (v2 as u32 & 0xFFFF_FF00) | u32::from(freq);
    pad_start_shake(get_pad(ru(shooter, 0x598) as i32), t as u32, freq_dword, 0);
    1
}

/// `CWeapon::FireInstantHitFromCar2(CVector start, CVector end, CVehicle* veh, CEntity* owner)` (0x73CBA0,
/// thiscall `ret 0x20`, void). `GetWeaponInfo(type, 1)` (discarded); `ReportCrime(1, FindPlayerPed(-1),
/// FindPlayerPed(-1))` (the player, whoever fires; two calls, the second result is the victim argument);
/// a `CEventGunShot` from `owner ? owner : veh` (silenced for 0x17 / 0x11) added to the global group;
/// `g_InterestingEvents.Add(0x16, owner)`; `AddLight(0, start, (0,0,0), 3.0, 0.25, 0.22, 0.0, 0, 0, null)`;
/// `bIncludeBikers = 1`, `pIgnoreEntity = veh`; `HandleGunShot`, `GunShotSetsOilOnFire`,
/// `ProcessLineOfSight(start, end, cp, hit, 1,1,1,1,1, 0, 0, 1)`, `ResetLineTestOptions()`;
/// `DoBulletImpact(owner, hit, start, end, cp, 0)`; `~CEventGunShot`. The original passes the silenced
/// flag as ECX with only CL set (upper bytes left by `ReportCrime`); the port pushes 0/1. The vectors are
/// the original's argument slots; the port passes copies of them (same values). SEH frame not reproduced.
#[hook(addr = 0x73CBA0, conv = thiscall, name = "CWeapon::FireInstantHitFromCar2", expect = "6A FF 68 68 8E 84 00", this = "CWeapon")]
#[allow(clippy::too_many_arguments)]
pub fn fire_instant_hit_from_car2(this: GPtr<u8>, sx: f32, sy: f32, sz: f32, ex: f32, ey: f32, ez: f32, veh: GPtr<u8>, owner: GPtr<u8>) {
    get_weapon_info(ru(this, 0) as i32, 1);
    let mut hit = GPtr::<u8>::NULL;
    let p1 = find_player_ped(-1);
    let p2 = find_player_ped(-1);
    report_crime(1, p2, p1);
    let silenced = u32::from(matches!(ru(this, 0), 0x17 | 0x11));
    let src = if owner.is_null() { veh } else { owner };
    let mut shot = [0u32; EVENT_SIZE / 4];
    event_gun_shot_ctor(lp(&mut shot).cast(), src, sx, sy, sz, ex, ey, ez, silenced);
    event_group_add(get_event_global_group(), lp(&mut shot).cast(), 0);
    interesting_events_add(g(0xC0B058), 0x16, owner);
    point_lights_add_light(0, sx, sy, sz, 0.0, 0.0, 0.0, 3.0, 0.25, cf(0x3E61_47AE), 0.0, 0, 0, GPtr::NULL);
    g::<u8>(0xB7CD6F).write(1); // CWorld::bIncludeBikers
    g::<GPtr<u8>>(0xB7CD68).write(veh); // CWorld::pIgnoreEntity
    let mut start = [sx, sy, sz];
    let mut end = [ex, ey, ez];
    let (sp, ep): (GPtr<u8>, GPtr<u8>) = (lp(&mut start).cast(), lp(&mut end).cast());
    let mut cp = [0u32; COL_POINT_SIZE / 4];
    birds_handle_gun_shot(sp, ep);
    shadows_gun_shot_sets_oil_on_fire(sp, ep);
    world_process_line_of_sight(sp, ep, lp(&mut cp).cast(), lp(&mut hit).cast(), 1, 1, 1, 1, 1, 0, 0, 1);
    world_reset_line_test_options();
    do_bullet_impact(this, owner, hit, sp, ep, lp(&mut cp).cast(), 0);
    event_gun_shot_dtor(lp(&mut shot).cast());
}

const COL_LINE_SIZE: usize = 0x20;
const COL_SPHERE_SIZE: usize = 0x14;
const MATRIX_SIZE: usize = 0x48;
const _: () = assert!(COL_LINE_SIZE == sa_core::layouts::size::CColLine);
const _: () = assert!(COL_SPHERE_SIZE == sa_core::layouts::size::CColSphere);
const _: () = assert!(MATRIX_SIZE == sa_core::layouts::size::CMatrix);

/// One occupant test of [`check_for_shooting_vehicle_occupant`] (0x73F50C..0x73F5DC, repeated at
/// 0x73F5FB..0x73F6CC): the head bone (`m_apBones[2]` +0x490 -> +0x14 node id) of a ped with flag
/// `+0x470 & 0x4000000`, (0,0,0) transformed by its hierarchy matrix; sphere `(P.x, P.y, P.z + 0.1f)`,
/// radius 0.2, material 0, piece 9, light 0xFF; true when `ProcessLineSphere` hits (closer than `depth`).
fn occupant_hit(ped: GPtr<u8>, line: GPtr<u8>, cp: GPtr<u8>, depth: GPtr<u8>) -> bool {
    if ped.is_null() || ru(ped, 0x470) & 0x0400_0000 == 0 {
        return false;
    }
    let mut p = [0u32; 3];
    let h = get_anim_hierarchy_from_skin_clump(rp(ped, 0x18));
    let idx = rp_hanim_id_get_index(h, ru(rp(ped, 0x490), 0x14) as i32);
    let m = rp_hanim_hierarchy_get_matrix_array(h).byte_add((idx << 6) as isize);
    rw_v3d_transform_points(lp(&mut p).cast(), lp(&mut p).cast(), 1, m);
    let z = x(f32::from_bits(p[2])).fadd_m(cf(0x3DCC_CCCD)).to_f32().to_bits();
    let mut center = [p[0], p[1], z];
    let mut sphere = [0u32; COL_SPHERE_SIZE / 4];
    col_sphere_set(lp(&mut sphere).cast(), cf(0x3E4C_CCCD), lp(&mut center).cast(), 0, 9, 0xFF);
    process_line_sphere(line, lp(&mut sphere).cast(), cp, depth) != 0
}

/// `CWeapon::CheckForShootingVehicleOccupant(CEntity** pEntity, CColPoint* cp, eWeaponType (unused),
/// const CVector* origin, const CVector* target)` (0x73F480, cdecl; entry `nop; jmp 0x4019CE`, a `.text`
/// stub that loads `fs:[0]` for the SEH frame, which is not reproduced).
///
/// For a vehicle `*pEntity` (else nothing): `saved = *cp`, `line = CColLine(*origin, *target)` (copied
/// out of a temporary as the original does), depth 1.0; the driver (+0x460) and then the 8 passenger
/// slots are tested (shared depth: a later occupant only hits when closer; `*pEntity` = the last occupant
/// hit). For the automobile class (+0x590 == 0), with `d = target - origin`: when
/// `(d.y*F.y + d.z*F.z) + d.x*F.x < 0` (forward; d.x, d.y, d.z kept in x87) and either
/// `(d.y*U.y + d.z*U.z) + d.x*U.x <= 0` (up; d.x, d.y as stored f32, d.z kept) or flag +0x429 & 4: the
/// line is moved into the vehicle's space (`Invert`, then the copy constructor on its own result, as the
/// original does), triangle planes calculated, and the first glass triangle the line crosses (`i16`
/// loop) progresses panel 4 (`ProgressPanelDamage`; at status 2 once more; then `SetPanelDamage(4,
/// true)` when the first progress succeeded). Without an occupant hit: `*pEntity = veh`, `*cp = saved`.
/// AL: 1 on a hit; otherwise the low byte of what the original leaves in EAX (the `pEntity` argument for
/// a non-vehicle, `CColPoint::operator=`'s result after the restore). The callers ignore it.
// Not registered (INF-20): the no-CD protection reads the stub's jump offset as data
// (`mov ecx, [0x73F482]` at 0x1567062, `.HOODLUM`, followed by `rol edi, cl`). catalog/code-data-reads.csv
// lists that reader as a linear-sweep decode outside a recognised protection chunk (chunk = 0), and sa-hook's
// `readers_match_the_exe_and_the_table` test accepts keycells readers only inside chunks: registering needs
// the catalogue's chunk detection (or the test) extended first. Registration would be `#[hook(addr = 0x73F480,
// conv = cdecl, name = "CWeapon::CheckForShootingVehicleOccupant", expect = "90 E9 48 25 CC FF")]`.
pub fn check_for_shooting_vehicle_occupant(pe: GPtr<u8>, cp: GPtr<u8>, _ty: i32, origin: GPtr<u8>, target: GPtr<u8>) -> u8 {
    let veh = pe.cast::<GPtr<u8>>().read();
    if etype(veh) != 2 {
        return pe.addr() as u8;
    }
    let mut saved = [0u32; COL_POINT_SIZE / 4];
    col_point_assign(lp(&mut saved).cast(), cp);
    let mut depth: f32 = 1.0;
    let mut hit = false;
    let mut tmp = [0u32; COL_LINE_SIZE / 4];
    let r = col_line_ctor(lp(&mut tmp).cast(), origin, target);
    let mut line = [0u32; COL_LINE_SIZE / 4];
    for (i, w) in line.iter_mut().enumerate() {
        *w = ru(r, 4 * i as isize);
    }
    let lptr: GPtr<u8> = lp(&mut line).cast();
    let dptr: GPtr<u8> = lp(&mut depth).cast();
    for k in 0..9isize {
        let slot = veh.byte_add(0x460 + 4 * k);
        if occupant_hit(rp(slot, 0), lptr, cp, dptr) {
            wp(pe, 0, rp(slot, 0));
            hit = true;
        }
    }
    if ru(veh, 0x590) == 0 {
        windscreen(veh, origin, target, lptr);
    }
    if hit {
        return 1;
    }
    wp(pe, 0, veh);
    col_point_assign(cp, lp(&mut saved).cast()).addr() as u8
}

/// The windscreen part of [`check_for_shooting_vehicle_occupant`] (0x73F6F1..0x73F8C2).
fn windscreen(veh: GPtr<u8>, origin: GPtr<u8>, target: GPtr<u8>, line: GPtr<u8>) {
    let m = rp(veh, 0x14);
    let dz = x(rf(target, 8)).fsub_m(rf(origin, 8));
    let dy = x(rf(target, 4)).fsub_m(rf(origin, 4));
    let dy32 = dy.to_f32();
    let dx = x(rf(target, 0)).fsub_m(rf(origin, 0));
    let dx32 = dx.to_f32();
    let mut f = Fpu::new();
    f.push(dy.fmul_m(rf(m, 0x14)).fadd(dz.fmul_m(rf(m, 0x18))).fadd(dx.fmul_m(rf(m, 0x10))));
    f.fcomp_m(0.0);
    if f.jp(5) {
        return;
    }
    let mut f = Fpu::new();
    f.push(x(dy32).fmul_m(rf(m, 0x24)).fadd(dz.fmul_m(rf(m, 0x28))).fadd(x(dx32).fmul_m(rf(m, 0x20))));
    f.fcomp_m(0.0);
    // `test ah, 0x41; jnp`: below or equal continues; above (and NaN) needs the flag.
    if f.jp(0x41) && rb(veh, 0x429) & 4 == 0 {
        return;
    }
    let cm = entity_get_col_model(veh);
    let data = rp(cm, 0x2C);
    if (rh(data, 4) as i16) <= 0 {
        return;
    }
    let mut inv = [0u32; MATRIX_SIZE / 4];
    let r = matrix_invert(rp(veh, 0x14), lp(&mut inv).cast());
    matrix_copy_ctor(lp(&mut inv).cast(), r);
    let invp: GPtr<u8> = lp(&mut inv).cast();
    for off in [0isize, 0x10] {
        let mut out = [0u32; 3];
        let r = matrix_mul_vec(lp(&mut out).cast(), invp, line.byte_add(off));
        let v = [ru(r, 0), ru(r, 4), ru(r, 8)];
        for (i, w) in v.iter().enumerate() {
            wu(line, off + 4 * i as isize, *w);
        }
    }
    calculate_triangle_planes(cm);
    if (rh(data, 4) as i16) > 0 {
        let mut i: u32 = 0;
        loop {
            let t = isize::from(i as u16 as i16);
            let tri = rp(data, 0x18).byte_add(t * 8);
            if surface_is_glass(g(0xB79538), u32::from(rb(tri, 6))) as u8 != 0
                && test_line_triangle(line, rp(data, 0x14), tri, rp(data, 0x1C).byte_add(t * 10)) != 0
            {
                let dm = veh.byte_add(0x5A0);
                if progress_panel_damage(dm, 4) != 0 {
                    if get_panel_status(dm, 4) == 2 {
                        progress_panel_damage(dm, 4);
                    }
                    automobile_set_panel_damage(veh, 4, 1);
                }
                break;
            }
            i = i.wrapping_add(1);
            if (i as u16 as i16) >= (rh(data, 4) as i16) {
                break;
            }
        }
    }
    matrix_dtor(invp);
}

orig! {
    /// `CWeapon::DoDriveByAutoAiming(CEntity* owner, CVehicle*, CVector* start, CVector* end, bool canAimVehicles)` (0x73D720)
    pub fn do_drive_by_auto_aiming(owner: GPtr<u8>, veh: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>, aim_vehicles: u32) = cdecl 0x73D720 => unported;
    /// `CWeapon::FireInstantHitFromCar2(CVector, CVector, CVehicle*, CEntity*)` (0x73CBA0, thiscall)
    pub fn fire_instant_hit_from_car2_call(this: GPtr<u8>, sx: f32, sy: f32, sz: f32, ex: f32, ey: f32, ez: f32, veh: GPtr<u8>, owner: GPtr<u8>) = thiscall 0x73CBA0 => fire_instant_hit_from_car2;
}

/// `S += veh->m_vecMoveSpeed * CTimer::ms_fTimeStep` as `FireInstantHitFromCar` does it: the three products
/// first (x, y kept in x87, z stored f32), then `x + S.x`, `S.y + y`, `z + S.z`.
fn add_move_step(veh: GPtr<u8>, s: &mut [f32; 3], y_reg: bool) {
    let mx = x(time_step()).fmul_m(rf(veh, 0x44));
    let my = x(time_step()).fmul_m(rf(veh, 0x48));
    let mz = x(time_step()).fmul_m(rf(veh, 0x4C)).to_f32();
    s[0] = mx.fadd_m(s[0]).to_f32();
    // The bone paths load S.y and add the register (`fld; fadd st(1)`), the seat paths add S.y from memory.
    s[1] = if y_reg { x(s[1]).fadd(my) } else { my.fadd_m(s[1]) }.to_f32();
    s[2] = x(mz).fadd_m(s[2]).to_f32();
}

/// The driver's right-hand bone transform (`m_apBones[6]` +0x4A0 -> +0x14 node id) of `s`, in place
/// (the driver is not null-checked, as in the original's non-bike path).
fn driver_bone_transform(veh: GPtr<u8>, s: &mut [f32; 3]) {
    let h = get_anim_hierarchy_from_skin_clump(rp(rp(veh, 0x460), 0x18));
    let idx = rp_hanim_id_get_index(h, ru(rp(rp(veh, 0x460), 0x4A0), 0x14) as i32);
    let m = rp_hanim_hierarchy_get_matrix_array(h).byte_add((idx << 6) as isize);
    let p: GPtr<u8> = lp(s).cast();
    rw_v3d_transform_points(p, p, 1, m);
}

/// The end point of the bone paths: `S - right*R` (left), else `S + v*R` with `v` = right or forward
/// (x87: `R*v.x` kept, `R*v.y` and `R*v.z` stored f32).
fn bone_path_end(veh: GPtr<u8>, s: [f32; 3], range: f32, left: u8, right: u8) -> [f32; 3] {
    let m = rp(veh, 0x14);
    let r = x(range);
    if left != 0 {
        let rx = r.fmul_m(rf(m, 0));
        let ry = r.fmul_m(rf(m, 4)).to_f32();
        let rz = r.fmul_m(rf(m, 8)).to_f32();
        [x(s[0]).fsub(rx).to_f32(), x(s[1]).fsub_m(ry).to_f32(), x(s[2]).fsub_m(rz).to_f32()]
    } else {
        let v = if right != 0 { m } else { m.byte_add(0x10) };
        let vx = r.fmul_m(rf(v, 0));
        let vy = r.fmul_m(rf(v, 4)).to_f32();
        let vz = r.fmul_m(rf(v, 8)).to_f32();
        [vx.fadd_m(s[0]).to_f32(), x(vy).fadd_m(s[1]).to_f32(), x(vz).fadd_m(s[2]).to_f32()]
    }
}

/// `M * l` (full transform with position, `operator*(CMatrix, CVector)` 0x59C890) of the vehicle matrix.
fn veh_mul(veh: GPtr<u8>, mut l: [f32; 3]) -> [f32; 3] {
    let mut out = [0u32; 3];
    let r = matrix_mul_vec(lp(&mut out).cast(), rp(veh, 0x14), lp(&mut l).cast());
    [rf(r, 0), rf(r, 4), rf(r, 8)]
}

/// `(float)(r & 0xFF) * 0.01f - 1.28f` (x87, unrounded).
fn drive_by_jitter(r: i32) -> X87 {
    xi(r & 0xFF).fmul_m(cf(0x3C23_D70A)).fsub_m(cf(0x3FA3_D70A))
}

/// `CWeapon::FireInstantHitFromCar(CVehicle* veh, bool left, bool right)` (0x73EC40, thiscall `ret 0xC`;
/// the drive-by shot behind `FireFromCar`, always true). `wi = GetWeaponInfo(type, 1)`; `mi` = the
/// vehicle's model info, seat `P = (mi+0x3C == 5) ? mi+0x5C struct : struct + 0x30`.
/// - Not a bike class (+0x590 != 9): `S = wi->fireOffset` (with `right`: `(x*1.8, y*1.8, f32(z*1.8) -
///   0.1)`), driver hand-bone transform (driver not null-checked), `S += mv*ts`, `E` from [`bone_path_end`].
/// - Bike class with a driver: the same without the scaling.
/// - Bike class without a driver (seat geometry, `cm = GetColModel()`, `r = rand() & 0xFF`): left
///   (`rand` before `GetColModel`): `S = M*(-cm.max.x - 0.25, r*0.001 + (P.y - 0.05), P.z + 0.63)`,
///   `E = M*(-R, P.y, P.z + 0.6)`; right (same order): `S = M*(cm.max.x + 0.25, r*0.001 + (P.y - 0.18),
///   P.z + 0.52)`, `E = M*(R, P.y, P.z + 0.5)`; neither (`GetColModel` first): `S = M*(r*0.001 - 0.4,
///   (cm.max.y + P.y) + 0.2, P.z + 0.55)`, `E = M*(0, R, P.z + 0.5)`; each `S += mv*ts`.
/// - Then `r1, r2, r3 = rand()`: `E.x += j(r2)`, `E.y += j(r1)`, `E.z += j(r3)` with
///   `j(r) = (r & 0xFF)*0.01 - 1.28`; `DoDriveByAutoAiming(FindPlayerPed(-1), veh, &S, &E, false)` (the
///   player ped whoever fires, original behaviour); `FireInstantHitFromCar2(S, E, veh, veh->m_pDriver)`.
/// The original writes seat pointers and `rand` values into its own argument slots (not reproduced,
/// stack-only). gta-reversed has no C++ for it (`plugin::CallMethodAndReturn`).
#[hook(addr = 0x73EC40, conv = thiscall, name = "CWeapon::FireInstantHitFromCar", expect = "83 EC 38 8B 01", this = "CWeapon")]
pub fn fire_instant_hit_from_car(this: GPtr<u8>, veh: GPtr<u8>, left: u8, right: u8) -> u8 {
    let wi = get_weapon_info(ru(this, 0) as i32, 1);
    let mi = g::<GPtr<u8>>(0xA9B0C8usize.wrapping_add((i32::from(rh(veh, 0x22) as i16) as usize).wrapping_mul(4))).read();
    let seat = || {
        let st = rp(mi, 0x5C);
        if ru(mi, 0x3C) == 5 { st } else { st.byte_add(0x30) }
    };
    let mut s: [f32; 3];
    let mut e: [f32; 3];
    if ru(veh, 0x590) != 9 {
        let o = rv(wi, 0x24);
        s = if right != 0 {
            let k = cf(0x3FE6_6666);
            let z = x(o[2]).fmul_m(k).to_f32();
            [x(o[0]).fmul_m(k).to_f32(), x(o[1]).fmul_m(k).to_f32(), x(z).fsub_m(cf(0x3DCC_CCCD)).to_f32()]
        } else {
            o
        };
        driver_bone_transform(veh, &mut s);
        add_move_step(veh, &mut s, true);
        e = bone_path_end(veh, s, rf(wi, 8), left, right);
    } else if !rp(veh, 0x460).is_null() {
        s = rv(wi, 0x24);
        driver_bone_transform(veh, &mut s);
        add_move_step(veh, &mut s, true);
        e = bone_path_end(veh, s, rf(wi, 8), left, right);
    } else if left != 0 {
        let p = seat();
        let r = sa_core::orig::at_site(0x73EE1E, rand);
        let cm = entity_get_col_model(veh);
        let lz = x(rf(p, 8)).fadd_m(cf(0x3F21_47AE)).to_f32();
        let ly = x(rf(p, 4)).fsub_m(cf(0x3D4C_CCCD)).fadd(xi(r & 0xFF).fmul_m(cf(0x3A83_126F))).to_f32();
        let lx = x(rf(cm, 0xC)).fchs().fsub_m(0.25).to_f32();
        s = veh_mul(veh, [lx, ly, lz]);
        add_move_step(veh, &mut s, false);
        let p = seat();
        let ez = x(rf(p, 8)).fadd_m(cf(0x3F19_999A)).to_f32();
        e = veh_mul(veh, [x(rf(wi, 8)).fchs().to_f32(), x(rf(p, 4)).to_f32(), ez]);
    } else if right != 0 {
        let p = seat();
        let r = sa_core::orig::at_site(0x73EF33, rand);
        let cm = entity_get_col_model(veh);
        let lz = x(rf(p, 8)).fadd_m(cf(0x3F05_1EB8)).to_f32();
        let ly = x(rf(p, 4)).fsub_m(cf(0x3E38_51EC)).fadd(xi(r & 0xFF).fmul_m(cf(0x3A83_126F))).to_f32();
        let lx = x(rf(cm, 0xC)).fadd_m(0.25).to_f32();
        s = veh_mul(veh, [lx, ly, lz]);
        add_move_step(veh, &mut s, false);
        let p = seat();
        let ez = x(rf(p, 8)).fadd_m(0.5).to_f32();
        e = veh_mul(veh, [x(rf(wi, 8)).to_f32(), x(rf(p, 4)).to_f32(), ez]);
    } else {
        let p = seat();
        let cm = entity_get_col_model(veh);
        let r = sa_core::orig::at_site(0x73F053, rand);
        let lz = x(rf(p, 8)).fadd_m(cf(0x3F0C_CCCD)).to_f32();
        let ly = x(rf(cm, 0x10)).fadd_m(rf(p, 4)).fadd_m(cf(0x3E4C_CCCD)).to_f32();
        let lx = xi(r & 0xFF).fmul_m(cf(0x3A83_126F)).fsub_m(cf(0x3ECC_CCCD)).to_f32();
        s = veh_mul(veh, [lx, ly, lz]);
        add_move_step(veh, &mut s, false);
        let p = seat();
        let ez = x(rf(p, 8)).fadd_m(0.5).to_f32();
        e = veh_mul(veh, [0.0, x(rf(wi, 8)).to_f32(), ez]);
    }
    let r1 = sa_core::orig::at_site(0x73F304, rand);
    let r2 = sa_core::orig::at_site(0x73F30B, rand);
    let ja = drive_by_jitter(r2).to_f32();
    let jb = drive_by_jitter(r1).to_f32();
    let r3 = sa_core::orig::at_site(0x73F34B, rand);
    let jc = drive_by_jitter(r3);
    e[0] = x(ja).fadd_m(e[0]).to_f32();
    e[1] = x(e[1]).fadd_m(jb).to_f32();
    e[2] = x(e[2]).fadd(jc).to_f32();
    let player = find_player_ped(-1);
    do_drive_by_auto_aiming(player, veh, lp(&mut s).cast(), lp(&mut e).cast(), 0);
    let drv = rp(veh, 0x460);
    fire_instant_hit_from_car2_call(this, s[0], s[1], s[2], e[0], e[1], e[2], veh, drv);
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_FIRE,
    HOOK_FIRE_SNIPER,
    HOOK_FIRE_M16_1ST_PERSON,
    HOOK_FIRE_INSTANT_HIT_FROM_CAR2,
    HOOK_FIRE_INSTANT_HIT_FROM_CAR,
];
