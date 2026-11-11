//! PORT-WEAPON-2 module `weapon_impact` (see `docs/arch/weapon-ped-port.md`, pass 2): `CWeapon`'s damage
//! event and bullet impact. Ported from the 1.0 US bytes (gta-reversed `Weapon.cpp` and
//! `docs/re/specs/WEAPON/CWeapon.deviations.md` as reading aids; the EXE wins).
//!
//! Event and calculator objects the original builds on its stack are built in Rust locals through the
//! original constructors/destructors (`orig!`), in the original order. The SEH frames (`fs:[0]`, the
//! state byte) are not reproduced (stack-only).

use sa_core::abi::HookReg;
use sa_core::mem::GlobalDesc;
use sa_core::{hook, orig};

use crate::common::*;

/// No module globals of its own (the WEAPON arrays are declared by their owning modules).
pub const GLOBALS: &[GlobalDesc] = &[];

/// `CPed::m_fHealth`
const PED_HEALTH: isize = 0x540;
/// `CPed::m_pIntelligence`
const PED_INTEL: isize = 0x47C;
/// `CPed` flag dword 0x46C (bit 8 = `bInVehicle`)
const PED_FLAGS: isize = 0x46C;
/// `CEntity::m_pRwClump`
const CLUMP: isize = 0x18;

/// `sizeof(CPedDamageResponseCalculator)`
const CALC_SIZE: usize = 0x14;
/// `sizeof(CEventDamage)`
const EVENT_DAMAGE_SIZE: usize = 0x44;

orig! {
    /// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(CEntity*, float, eWeaponType, ePedPieceTypes, bool)` (0x4AD3F0, thiscall)
    pub fn calc_ctor(this: GPtr<u8>, creator: GPtr<u8>, damage: f32, ty: i32, piece: i32, b: u8) -> GPtr<u8> = thiscall 0x4AD3F0 => unported;
    /// `CPedDamageResponseCalculator::~CPedDamageResponseCalculator()` (0x4AD420, thiscall)
    pub fn calc_dtor(this: GPtr<u8>) = thiscall 0x4AD420 => unported;
    /// `CPedDamageResponseCalculator::ComputeDamageResponse(CPed*, CPedDamageResponse&, bool speak)` (0x4B5AC0, thiscall)
    pub fn calc_compute_damage_response(this: GPtr<u8>, ped: GPtr<u8>, resp: GPtr<u8>, speak: u8) = thiscall 0x4B5AC0 => unported;
    /// `CEventDamage::CEventDamage(CEntity* source, uint32 time, eWeaponType, ePedPieceTypes, uint8 dir, bool, bool inVehicle)`
    /// (0x4AD830, thiscall); the last word is pushed as `(flags >> 8) & 0xFFFFFF01`
    pub fn event_damage_ctor(this: GPtr<u8>, source: GPtr<u8>, time: u32, ty: i32, piece: i32, dir: u32, a: u32, in_veh: u32) -> GPtr<u8> = thiscall 0x4AD830 => unported;
    /// `CEventDamage::~CEventDamage()` (non-virtual body, 0x4AD960, thiscall)
    pub fn event_damage_dtor(this: GPtr<u8>) = thiscall 0x4AD960 => unported;
    /// `CEventDamage::AffectsPed(CPed*)` (0x4B35A0, thiscall)
    pub fn event_damage_affects_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4B35A0 => unported;
    /// `CEventDamage::ComputeDeathAnim(CPed*, bool)` (0x4B3A60, thiscall)
    pub fn event_damage_compute_death_anim(this: GPtr<u8>, ped: GPtr<u8>, b: u8) = thiscall 0x4B3A60 => unported;
    /// `CEventDamage::ComputeDamageAnim(CPed*, bool)` (0x4B3FC0, thiscall)
    pub fn event_damage_compute_damage_anim(this: GPtr<u8>, ped: GPtr<u8>, b: u8) = thiscall 0x4B3FC0 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, thiscall)
    pub fn event_group_add(this: GPtr<u8>, ev: GPtr<u8>, b: u8) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CLocalisation::Blood()` (0x56D230)
    pub fn localisation_blood() -> u8 = cdecl 0x56D230 => unported;
    /// `CLocalisation::KickingWhenDown()` (0x56D270)
    pub fn localisation_kicking_when_down() -> u8 = cdecl 0x56D270 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0, thiscall)
    pub fn task_manager_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720, thiscall)
    pub fn task_manager_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10, thiscall)
    pub fn task_get_type(task: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*, uint32 flags)` (0x4D6A70)
    pub fn clump_get_first_association_flags(clump: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x4D6A70 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, uint32 animId)` (0x4D68B0)
    pub fn clump_get_association(clump: GPtr<u8>, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    pub fn blend_animation(clump: GPtr<u8>, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimManager::AddAnimation(RpClump*, AssocGroupId, AnimationId)` (0x4D3AA0)
    pub fn add_animation(clump: GPtr<u8>, group: u32, id: u32) -> GPtr<u8> = cdecl 0x4D3AA0 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80, thiscall)
    pub fn assoc_set_current_time(assoc: GPtr<u8>, t: f32) = thiscall 0x4CEA80 => unported;
    /// `CPed::IsAlive()` (0x5E0170, thiscall)
    pub fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `CCrime::ReportCrime(eCrimeType, CEntity* victim, CPed* criminal)` (0x532010)
    pub fn report_crime(ty: i32, victim: GPtr<u8>, criminal: GPtr<u8>) = cdecl 0x532010 => unported;
    /// `CWeapon::GenerateDamageEvent(...)` (0x73A530)
    pub fn generate_damage_event_call(victim: GPtr<u8>, creator: GPtr<u8>, ty: i32, damage: i32, piece: i32, dir: u32) -> u8 = cdecl 0x73A530 => generate_damage_event;
    /// `CPed::GetWeaponSkill()` (current weapon, 0x5E6580, thiscall)
    pub fn ped_get_weapon_skill_current(ped: GPtr<u8>) -> u8 = thiscall 0x5E6580 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0, thiscall)
    pub fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::GetLocalDirection(const CVector2D&)` (0x5DEF60, thiscall)
    pub fn ped_get_local_direction(ped: GPtr<u8>, v: GPtr<u8>) -> u32 = thiscall 0x5DEF60 => unported;
    /// `CPedGroups::AreInSameGroup(const CPed*, const CPed*)` (0x5F7F40)
    pub fn are_in_same_group(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x5F7F40 => unported;
    /// `CBulletTraces::AddTrace(const CVector* from, const CVector* to, eWeaponType, CEntity*)` (0x726AF0)
    pub fn bullet_traces_add_trace(from: GPtr<u8>, to: GPtr<u8>, ty: i32, e: GPtr<u8>) = cdecl 0x726AF0 => unported;
    /// `CStats::GetStatValue(eStats)` (0x558E40, ST0)
    pub fn stats_get_stat_value(stat: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CStats::IncrementStat(eStats, float)` (0x55C180)
    pub fn stats_increment_stat(stat: u32, v: f32) = cdecl 0x55C180 => unported;
    /// `CStats::UpdateStatsWhenWeaponHit(eWeaponType)` (0x55CEB0)
    pub fn stats_update_when_weapon_hit(ty: i32) = cdecl 0x55CEB0 => unported;
    /// `CVehicle::BurstTyre(uint8 component, bool)` (vtable slot 0xB0, thiscall); the component dword is
    /// pushed with the stale upper bytes of the stack slot it was stored in
    pub fn vehicle_burst_tyre(veh: GPtr<u8>, component: u32, b: u8) -> u8 = thiscall vslot 0xB0 => unported;
    /// `CEntity::SetIsStatic(bool)` (vtable slot 0x10, thiscall)
    pub fn entity_set_is_static(e: GPtr<u8>, b: u8) = thiscall vslot 0x10 => unported;
    /// `CVector::Normalise()` (0x59C910, thiscall)
    pub fn normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CVector::Magnitude()` (0x4082C0, thiscall, ST0)
    pub fn magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, returns into `out`)
    pub fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `operator*(float, const CVector&)` (0x40FE90, returns into `out`)
    pub fn vector_scale(out: GPtr<u8>, f: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `std::min<float>(a, b)` (0x404330, ST0: `a < b ? a : b`)
    pub fn min_f32(a: f32, b: f32) -> f64 = cdecl 0x404330 => unported;
    /// `tColLighting::GetCurrentLighting(float scale)` (0x59F0C0, the lighting byte passed by value, ST0)
    pub fn get_current_lighting(lighting: u32, scale: f32) -> f64 = cdecl 0x59F0C0 => unported;
    /// `CAudioEngine::ReportBulletHit(CEntity*, eSurfaceType, const CVector&, float angle)` (0x506EC0, thiscall)
    pub fn audio_report_bullet_hit(ae: GPtr<u8>, e: GPtr<u8>, surface: u32, pos: GPtr<u8>, angle: f32) = thiscall 0x506EC0 => unported;
    /// `Fx_c::AddBlood(CVector&, CVector&, int32, float lighting)` (0x49EB00, thiscall); lighting pushed as the raw word
    pub fn fx_add_blood(fx: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>, amount: u32, lighting: u32) = thiscall 0x49EB00 => unported;
    /// `Fx_c::AddBulletImpact(CVector&, CVector&, int32 surface, int32 amount, float lighting)` (0x49F3D0, thiscall)
    pub fn fx_add_bullet_impact(fx: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>, surface: u32, amount: u32, lighting: f32) = thiscall 0x49F3D0 => unported;
    /// `Fx_c::AddTyreBurst(CVector&, CVector&)` (0x49F300, thiscall)
    pub fn fx_add_tyre_burst(fx: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>) = thiscall 0x49F300 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float)` (0x420D40, thiscall)
    pub fn camera_is_sphere_visible(cam: GPtr<u8>, c: GPtr<u8>, r: f32) -> u8 = thiscall 0x420D40 => unported;
    /// `CGlass::WasGlassHitByBullet(CEntity*, CVector)` (0x71C0D0)
    pub fn glass_was_hit_by_bullet(e: GPtr<u8>, x: u32, y: u32, z: u32) = cdecl 0x71C0D0 => unported;
    /// `CObject::ObjectDamage(float, const CVector*, const CVector*, CEntity*, eWeaponType)` (0x5A0D90, thiscall)
    pub fn object_damage(obj: GPtr<u8>, dmg: f32, pos: GPtr<u8>, dir: GPtr<u8>, damager: GPtr<u8>, ty: i32) = thiscall 0x5A0D90 => unported;
    /// `CEntity::GetIsStatic()` (0x4633E0, thiscall)
    pub fn entity_get_is_static(e: GPtr<u8>) -> u8 = thiscall 0x4633E0 => unported;
    /// `CPhysical::AddToMovingList()` (0x542800, thiscall)
    pub fn physical_add_to_moving_list(e: GPtr<u8>) = thiscall 0x542800 => unported;
    /// `CPhysical::ApplyForce(CVector force, CVector point, bool)` (0x542B50, thiscall); raw words
    pub fn physical_apply_force(e: GPtr<u8>, fx: u32, fy: u32, fz: u32, px: u32, py: u32, pz: u32, b: u8) = thiscall 0x542B50 => unported;
    /// `CVehicle::InflictDamage(CEntity*, eWeaponType, float, CVector)` (0x6D7C90, thiscall); raw position words
    pub fn vehicle_inflict_damage(veh: GPtr<u8>, e: GPtr<u8>, ty: i32, dmg: f32, x: u32, y: u32, z: u32) = thiscall 0x6D7C90 => unported;
    /// `CEventVehicleDamageWeapon::CEventVehicleDamageWeapon(CVehicle*, CEntity*, eWeaponType)` (0x61C2E0, thiscall)
    pub fn event_vehicle_damage_weapon_ctor(ev: GPtr<u8>, veh: GPtr<u8>, e: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x61C2E0 => unported;
    /// `CEventVehicleDamage::~CEventVehicleDamage()` (base body, 0x4B1990, thiscall)
    pub fn event_vehicle_damage_dtor(ev: GPtr<u8>) = thiscall 0x4B1990 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn pad_get_pad(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::StartShake_Distance(int16 time, uint8 freq, float x, y, z)` (0x53F9A0, thiscall); raw words
    pub fn pad_start_shake_distance(pad: GPtr<u8>, time: u32, freq: u32, x: u32, y: u32, z: u32) = thiscall 0x53F9A0 => unported;
}

/// CRT `_CIasin` (0x821E70): ST0 in, ST0 out (`sa_core::orig::x87_unary`; unported in phases B/C).
fn ci_asin(v: X87) -> X87 {
    st0(sa_core::orig::x87_unary(0x821E70, v.to_f64()))
}

/// `Fx_c g_fx`
const G_FX: usize = 0xA9AE00;
/// `CAudioEngine AudioEngine`
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `CCamera TheCamera`
const THE_CAMERA: usize = 0xB6F028;
/// `CVehicle` passengers (`CPed* [8]`), driver at 0x460, count byte at 0x488
const VEH_DRIVER: isize = 0x460;
const VEH_PASSENGERS: isize = 0x464;
const VEH_NUM_PASSENGERS: isize = 0x488;

/// `ped->m_fHealth` compared with 0.0 (`fld; fcomp [0x858B50]; fnstsw ax`).
fn health_cmp(ped: GPtr<u8>) -> Fpu {
    let mut f = Fpu::new();
    f.fld(rf(ped, PED_HEALTH));
    f.fcomp_m(0.0);
    f
}

/// `CWeapon::GenerateDamageEvent(CPed* victim, CEntity* creator, eWeaponType type, int32 damage,
/// ePedPieceTypes piece, uint8 direction)` (0x73A530, cdecl, AL result).
///
/// Builds a `CPedDamageResponseCalculator(creator, (float)damage, type, piece, false)` and a
/// `CEventDamage(creator, now, type, piece, direction, false, victim.bInVehicle)` on the stack, then:
/// - a victim with `health <= 0` (C0 or C3 alone), `Blood() && KickingWhenDown()` and a simplest active
///   task of type 0xDA: blend anim 0x27 (when an association with flag 0x800 exists) or 0x24 (group 0,
///   delta 8.0); on success `SetCurrentTime(0)` and `flags = (flags & 0xFFF7) | 1`; result true;
/// - a dead victim (`!IsAlive`): result true, nothing else;
/// - `!AffectsPed(victim)`: result false;
/// - else: `ReportCrime(2, victim, creator)` when the creator is player 0's ped; `ComputeDamageResponse(victim,
///   event.response, true)`; for melee (see below) and a victim not in a vehicle the death (`health <= 0`)
///   or damage anim; anim ids 0x1C..0x1F: the association (added when missing) gets blend 0, delta and
///   speed and time 0; 0xBF nothing; 0xC0 result false; others `BlendAnimation(group, id, delta)` (its
///   result used without a null check) gets the speed; ids other than 0xBF/0xC0 set event bit 0x400.
///   Event bit 0x1000 = creator is a ped whose active task is 0x403, or the type is 0x17. The event
///   goes to the victim's event group unless the victim is in a vehicle, alive (`health > 0`) and its
///   active task is 0x3FE.
///
/// Kept original (CWeapon.deviations.md): `GetWeaponInfo(type, 1)->m_nWeaponFire` is read for every
/// type (gr with fix-bugs skips it for non-weapon types); melee = `fire == 0 || (type == 0x36 && creator
/// is an object)`, then `!bInVehicle`. The upper EAX bits of the result are the calculator destructor's
/// (callers read AL); here they are 0.
#[hook(addr = 0x73A530, conv = cdecl, name = "CWeapon::GenerateDamageEvent", expect = "6A FF 68 10 8E 84 00")]
pub fn generate_damage_event(victim: GPtr<u8>, creator: GPtr<u8>, ty: i32, damage: i32, piece: i32, dir: u32) -> u8 {
    let mut result: u8 = 1;
    let mut calc = [0u32; CALC_SIZE / 4];
    let mut ev = [0u32; EVENT_DAMAGE_SIZE / 4];
    let calc_p: GPtr<u8> = lp(&mut calc).cast();
    let ev_p: GPtr<u8> = lp(&mut ev).cast();
    calc_ctor(calc_p, creator, xi(damage).to_f32(), ty, piece, 0);
    let in_veh = (ru(victim, PED_FLAGS) >> 8) & 0xFFFF_FF01;
    event_damage_ctor(ev_p, creator, time_ms(), ty, piece, dir, 0, in_veh);

    let tm = |ped: GPtr<u8>| rp(ped, PED_INTEL).byte_add(4);
    'body: {
        if !health_cmp(victim).jp(0x41)
            && localisation_blood() != 0
            && localisation_kicking_when_down() != 0
            && !task_manager_get_simplest_active_task(tm(victim)).is_null()
            && task_get_type(task_manager_get_simplest_active_task(tm(victim))) == 0xDA
        {
            let id = if clump_get_first_association_flags(rp(victim, CLUMP), 0x800).is_null() { 0x24 } else { 0x27 };
            let a = blend_animation(rp(victim, CLUMP), 0, id, 0x4100_0000);
            if !a.is_null() {
                assoc_set_current_time(a, 0.0);
                wh(a, 0x2E, (rh(a, 0x2E) & 0xFFF7) | 1);
            }
            break 'body;
        }
        if ped_is_alive(victim) == 0 {
            break 'body;
        }
        if event_damage_affects_ped(ev_p, victim) == 0 {
            event_damage_dtor(ev_p);
            calc_dtor(calc_p);
            return 0;
        }
        if creator == find_player_ped(-1) {
            report_crime(2, victim, creator);
        }
        calc_compute_damage_response(calc_p, victim, ev_p.byte_add(0x38), 1);
        let fire = ru(get_weapon_info(ty, 1), 0);
        let melee = fire == 0 || (ty == 0x36 && !creator.is_null() && etype(creator) == 4);
        if melee && ru(victim, PED_FLAGS) & 0x100 == 0 {
            if health_cmp(victim).jp(0x41) {
                event_damage_compute_damage_anim(ev_p, victim, 1);
            } else {
                event_damage_compute_death_anim(ev_p, victim, 1);
            }
            let id = ru(ev_p, 0x2C);
            if (0x1C..=0x1F).contains(&(id as i32)) {
                let mut a = clump_get_association(rp(victim, CLUMP), id);
                if a.is_null() {
                    a = add_animation(rp(victim, CLUMP), ru(ev_p, 0x28), ru(ev_p, 0x2C));
                }
                wu(a, 0x18, 0);
                wu(a, 0x1C, ru(ev_p, 0x30));
                wu(a, 0x24, ru(ev_p, 0x34));
                assoc_set_current_time(a, 0.0);
                wb(ev_p, 0x25, rb(ev_p, 0x25) | 4);
            } else if id == 0xBF {
            } else if id == 0xC0 {
                result = 0;
            } else {
                let a = blend_animation(rp(victim, CLUMP), ru(ev_p, 0x28), id, ru(ev_p, 0x30));
                wu(a, 0x24, ru(ev_p, 0x34));
                wb(ev_p, 0x25, rb(ev_p, 0x25) | 4);
            }
        }
        let mut bl = false;
        if !creator.is_null() && etype(creator) == 3 {
            let t = task_manager_get_active_task(tm(creator));
            bl = (!t.is_null() && task_get_type(t) == 0x403) || ty == 0x17;
        }
        let al = rb(ev_p, 0x25);
        wb(ev_p, 0x25, (al & !0x10) | (u8::from(bl) << 4));
        if ru(victim, PED_FLAGS) & 0x100 != 0
            && !health_cmp(victim).jne(0x41)
            && !task_manager_get_active_task(tm(victim)).is_null()
            && task_get_type(task_manager_get_active_task(tm(victim))) == 0x3FE
        {
            break 'body;
        }
        event_group_add(rp(victim, PED_INTEL).byte_add(0x68), ev_p, 0);
    }
    event_damage_dtor(ev_p);
    calc_dtor(calc_p);
    result
}

/// `(dir . normal)` of the bullet against the hit normal and, when negative, the audio report: `dir =
/// normalise(end - start)` (each component stored as f32), `dot = (dir.z*n.z + dir.y*n.y) + dir.x*n.x`
/// in x87, compared unrounded with 0 (C0 alone), the angle `asin(-(float)dot) * 57.295776f`.
fn report_hit_angle(victim: GPtr<u8>, cp: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) {
    let mut dir = [
        x(rf(end, 0)).fsub_m(rf(start, 0)).to_f32(),
        x(rf(end, 4)).fsub_m(rf(start, 4)).to_f32(),
        x(rf(end, 8)).fsub_m(rf(start, 8)).to_f32(),
    ];
    normalise(lp(&mut dir).cast());
    let dot = x(dir[2])
        .fmul_m(rf(cp, 0x18))
        .fadd(x(dir[1]).fmul_m(rf(cp, 0x14)))
        .fadd(x(dir[0]).fmul_m(rf(cp, 0x10)));
    let mut f = Fpu::new();
    f.push(dot);
    f.fcomp_m(0.0);
    if f.jp(5) {
        return;
    }
    let angle = ci_asin(x(dot.to_f32()).fchs()).fmul_m(cf(0x4265_2EE0)).to_f32();
    audio_report_bullet_hit(g(AUDIO_ENGINE), victim, u32::from(rb(cp, 0x23)), cp, angle);
}

/// `if (TheCamera.IsSphereVisible(cp.pos, 1.0)) g_fx.AddBulletImpact(cp.pos, cp.normal, cp.surfaceB,
/// amount, GetCurrentLighting(cp.lightingB, 0.5))`.
fn bullet_impact_fx(cp: GPtr<u8>, amount: u8) {
    if camera_is_sphere_visible(g(THE_CAMERA), cp, 1.0) == 0 {
        return;
    }
    let l = st0(get_current_lighting(u32::from(rb(cp, 0x25)), 0.5)).to_f32();
    fx_add_bullet_impact(g(G_FX), cp, cp.byte_add(0x10), u32::from(rb(cp, 0x23)), u32::from(amount), l);
}

/// `ApplyForce(normal * f, cp.pos - victim.pos, true)` through the original vector helpers (0x40FE60,
/// 0x40FE90) in the original call order.
fn push_at_hit(victim: GPtr<u8>, cp: GPtr<u8>, f: f32) {
    let mut pt = [0f32; 3];
    let ptp = vector_sub(lp(&mut pt).cast(), cp, pos_ptr(victim));
    let mut fv = [0f32; 3];
    let fvp = vector_scale(lp(&mut fv).cast(), f, cp.byte_add(0x10));
    let a = rv_bits(ptp, 0);
    let b = rv_bits(fvp, 0);
    physical_apply_force(victim, b[0], b[1], b[2], a[0], a[1], a[2], 1);
}

/// A `CEventVehicleDamageWeapon(veh, firedBy, type)` on the stack, added to `ped`'s event group, then
/// destroyed as the original does (vtable 0x86D640 stored, base destructor 0x4B1990).
fn vehicle_damage_event(ped: GPtr<u8>, veh: GPtr<u8>, fired_by: GPtr<u8>, ty: i32) {
    let mut ev = [0u32; 8];
    let evp: GPtr<u8> = lp(&mut ev).cast();
    event_vehicle_damage_weapon_ctor(evp, veh, fired_by, ty);
    event_group_add(rp(ped, PED_INTEL).byte_add(0x68), evp, 0);
    wu(evp, 0, 0x86D640);
    event_vehicle_damage_dtor(evp);
}

/// Replaces the low byte of a stack word (`mov byte ptr [slot], r8`).
fn set_low(slot: &mut u32, b: u8) {
    *slot = (*slot & 0xFFFF_FF00) | u32::from(b);
}

/// `x87 compare of [p+off] with 0.0; test ah, 0x41; jne` not taken: the value is > 0.
fn gt0(p: GPtr<u8>, off: isize) -> bool {
    let mut f = Fpu::new();
    f.fld(rf(p, off));
    f.fcomp_m(0.0);
    !f.jne(0x41)
}

/// `CWeapon::DoBulletImpact(CEntity* firedBy, CEntity* victim, const CVector* start, const CVector* end,
/// const CColPoint* cp, int32 incrementalHit)` (0x73B550, thiscall, `ret 0x18`). Follows the bytes:
///
/// - info = `GetWeaponInfo(type, 1)`, or with the ped's current skill for a ped shooter; a player shooter
///   (`firedByPlayer`) reports crime 1 (before the victim null test);
/// - no victim: `AddTrace(start, end, type, firedBy)`, then the shake;
/// - else `AddTrace(incrementalHit ? start + (cp.pos - start)*0.4f : start, cp.pos, type, firedBy)`
///   (x87: dz, dy unrounded and dx rounded before the products, each product stored as f32);
/// - stats (kept original, the `#else` form of gr's FIX_BUGS block): entered iff `(firedByPlayer &&
///   victim != firedBy) || (firedBy->status bits 0xF8) == 0` (firedBy read without a null check); victim
///   type 3/2/4: `IncrementStat(0x80, 1)` unless `GetStatValue(0x7E) < GetStatValue(0x80)`; types
///   0x16..0x20 with a ped shooter: target = `firedBy->+0x71C` or the victim; hit counts when (ped)
///   health > 0, (vehicle) health > 0 and status != 5, (object) health > 0, damage effect != 0 and
///   info->colDamageMultiplier >= 0.5; vehicles then need `bCanBeDamaged` and not `bBulletProof`, and
///   on wheel pieces 13..16 a successful `BurstTyre(piece, true)` (called during the test, as gr); a ped
///   target in the shooter's group never counts; then `UpdateStatsWhenWeaponHit(type)`;
/// - ped victim: firedBy dereferenced without a null check; same ped type (not 4/5) and a non-player
///   shooter, or `victim == firedBy`: return at once (no shake). Else for `incrementalHit <= 0`: the hit
///   angle report, damage `info->damage` (times `-incrementalHit` when negative), 150 for a player
///   within 1.0 of the victim without `bNoCriticalHits` and not types 0x19/0x1B, then
///   `GenerateDamageEvent(victim, firedBy, type, damage, cp.pieceB, GetLocalDirection(start - pos))`
///   decides the blood (else blood = true); player crime 4; with `Blood()` and blood:
///   `AddBlood(cp.pos, cp.normal, piece 9 ? (health < 0 ? 0x20 : 0x10) : (incrementalHit ? 4 : 8),
///   victim->+0x12C)`;
/// - other victims: `WasGlassHitByBullet(victim, cp.pos)`; impact amount `incrementalHit ? 2 : 8`;
///   objects: fx; damage effect >= 200: `ObjectDamage(gunBreakMode 1: 151, 2: 151*smashMult, else 50)`;
///   else (no `bDisableCollisionForce`, colDamageMultiplier < 99.9): un-static (uproot <= 0), push
///   with `normal * f` (f = -2, or -0.2 with flags 0xA0, times 0.2 when incremental) at `cp.pos - pos`,
///   and then ALSO `ObjectDamage(50)` (original; gr calls ObjectDamage only for effect >= 200);
///   vehicles: wheel pieces 13..16 burst the tyre, tyre-burst fx, and with a ped shooter a
///   `CEventVehicleDamageWeapon` to the driver and every passenger slot below the count; else
///   `InflictDamage(firedBy, type, damage (doubled for a player in camera mode 0x31), cp.pos)`, fx and,
///   unless the load monitor level >= 2, `ApplyForce(normal * (min(1, mass*0.001) * f), cp.pos - pos)`
///   (f = -20 for 24/38, -4 for 25/27, else -10); buildings: fx and the player's last building model;
///   then for `incrementalHit <= 0` the hit angle report;
/// - shake: a ped shooter of ped type 0/1 shakes its pad by the distance to player 0
///   (`StartShake_Distance(240, 128, p3.x, p2.y, p1.z)` over three `FindPlayerPed` calls).
///
/// Stack-only, not reproduced: the original keeps temporaries in its own argument slots (firedBy, victim,
/// start, end, incrementalHit) and the `CVector2D` for `GetLocalDirection` over its `firedByPlayer`
/// local (not read afterwards on that path). The words pushed from argument slots (the `BurstTyre`
/// component) are reproduced with their stale upper bytes. The SEH frame is not built.
#[hook(addr = 0x73B550, conv = thiscall, name = "CWeapon::DoBulletImpact", expect = "6A FF 68 50 8E 84 00", this = "CWeapon")]
pub fn do_bullet_impact(this: GPtr<u8>, fired_by: GPtr<u8>, victim: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>, cp: GPtr<u8>, inc_hit: i32) {
    // the firedBy / victim argument slots, reused by the original as temporaries
    let mut s4 = fired_by.addr() as u32;
    let mut s8 = victim.addr() as u32;
    let ty = || ru(this, 0) as i32;
    let mut wi = get_weapon_info(ty(), 1);
    if !fired_by.is_null() && etype(fired_by) == 3 {
        let skill = ped_get_weapon_skill_current(fired_by);
        wi = get_weapon_info(ty(), skill);
    }
    let mut fired_by_player = GPtr::<u8>::NULL;
    if !fired_by.is_null() && etype(fired_by) == 3 && ped_is_player(fired_by) != 0 {
        fired_by_player = fired_by;
        report_crime(1, victim, fired_by);
    }
    'shake: {
        if victim.is_null() {
            bullet_traces_add_trace(start, end, ty(), fired_by);
            break 'shake;
        }
        let mut from = rv(start, 0);
        if inc_hit != 0 {
            let k = cf(0x3ECC_CCCD);
            let mut f = Fpu::new();
            f.fld(rf(cp, 8));
            f.fsub_m(rf(start, 8));
            f.fld(rf(cp, 4));
            f.fsub_m(rf(start, 4));
            f.fld(rf(cp, 0));
            f.fsub_m(rf(start, 0));
            let dx = f.fstp();
            f.fxch(1);
            f.fmul_m(k);
            let pz = f.fstp();
            s4 = pz.to_bits();
            f.fmul_m(k);
            let py = f.fstp();
            s8 = py.to_bits();
            let px = x(dx).fmul_m(k).to_f32();
            let z = x(pz).fadd_m(rf(start, 8)).to_f32();
            let y = x(py).fadd_m(rf(start, 4)).to_f32();
            let xx = x(px).fadd_m(rf(start, 0)).to_f32();
            from = [xx, y, z];
        }
        bullet_traces_add_trace(lp(&mut from).cast(), cp, ty(), fired_by);

        if (!fired_by_player.is_null() && victim != fired_by) || rb(fired_by, 0x36) & 0xF8 == 0 {
            if matches!(etype(victim), 2..=4) {
                let a = st0(stats_get_stat_value(0x80)).to_f32();
                s4 = a.to_bits();
                let mut f = Fpu::new();
                f.push(st0(stats_get_stat_value(0x7E)));
                f.fcomp_m(a);
                if f.ah() & 1 == 0 {
                    stats_increment_stat(0x80, 1.0);
                }
            }
            'stats: {
                if !(0x16..=0x20).contains(&ty()) {
                    break 'stats;
                }
                set_low(&mut s4, 0);
                if etype(fired_by) != 3 {
                    break 'stats;
                }
                let mut t = rp(fired_by, 0x71C);
                if t.is_null() {
                    t = victim;
                }
                let cl = rb(t, 0x36);
                let dl = cl & 7;
                let counts = (dl == 3 && gt0(t, PED_HEALTH))
                    || (dl == 2 && gt0(t, 0x4C0) && cl & 0xF8 != 0x28)
                    || (dl == 4 && gt0(t, 0x154) && rb(t, 0x144) != 0 && {
                        let mut f = Fpu::new();
                        f.fld(rf(rp(t, 0x160), 0x18));
                        f.fcomp_m(0.5);
                        f.ah() & 1 == 0
                    });
                if counts {
                    set_low(&mut s4, 1);
                }
                if dl == 2 {
                    if rb(t, 0x42A) & 0x20 == 0 || ru(t, 0x40) & 0x40000 != 0 {
                        set_low(&mut s4, 0);
                    }
                    let piece = rb(cp, 0x24);
                    if (0xD..=0x10).contains(&piece) {
                        set_low(&mut s8, piece);
                        if vehicle_burst_tyre(t, s8, 1) == 0 {
                            set_low(&mut s4, 0);
                        }
                    }
                }
                if etype(t) == 3 && are_in_same_group(t, fired_by) != 0 {
                    break 'stats;
                }
                if s4 as u8 != 0 {
                    stats_update_when_weapon_hit(ty());
                }
            }
        }

        if etype(victim) == 3 {
            let mut same = GPtr::<u8>::NULL;
            if etype(fired_by) == 3 {
                same = fired_by;
                let pt = ru(fired_by, 0x598);
                if pt == ru(victim, 0x598) && pt != 4 && pt != 5 && ped_is_player(fired_by) == 0 {
                    return;
                }
            }
            if victim == same {
                return;
            }
            set_low(&mut s4, 1);
            if inc_hit <= 0 {
                report_hit_angle(victim, cp, start, end);
                let p = rv(pos_ptr(victim), 0);
                let mut v2 = [x(rf(start, 0)).fsub_m(p[0]).to_f32(), x(rf(start, 4)).fsub_m(p[1]).to_f32()];
                let dir = ped_get_local_direction(victim, lp(&mut v2).cast());
                let mut dmg = i32::from(rh(wi, 0x22) as i16);
                if inc_hit < 0 {
                    dmg = dmg.wrapping_mul(inc_hit).wrapping_neg();
                }
                if etype(fired_by) == 3 && ped_is_player(fired_by) != 0 {
                    let mut d = [0f32; 3];
                    let dp = vector_sub(lp(&mut d).cast(), pos_ptr(victim), start);
                    let mut f = Fpu::new();
                    f.push(st0(magnitude(dp)));
                    f.fcomp_m(1.0);
                    if !f.jp(5) && ru(victim, 0x470) & 0x1000 == 0 && ty() != 0x19 && ty() != 0x1B {
                        dmg = 0x96;
                    }
                }
                let r = generate_damage_event_call(victim, fired_by, ty(), dmg, i32::from(rb(cp, 0x24)), dir);
                set_low(&mut s4, r);
            }
            if etype(fired_by) == 3 && ped_is_player(fired_by) != 0 {
                report_crime(4, victim, fired_by);
            }
            if localisation_blood() != 0 && s4 as u8 != 0 {
                let mut amount: u8 = if inc_hit == 0 { 8 } else { 4 };
                if rb(cp, 0x24) == 9 {
                    amount = 0x10;
                    let mut f = Fpu::new();
                    f.fld(rf(victim, PED_HEALTH));
                    f.fcomp_m(0.0);
                    if !f.jp(5) {
                        amount = 0x20;
                    }
                }
                fx_add_blood(g(G_FX), cp, cp.byte_add(0x10), u32::from(amount), ru(victim, 0x12C));
            }
            break 'shake;
        }

        let c = rv_bits(cp, 0);
        glass_was_hit_by_bullet(victim, c[0], c[1], c[2]);
        set_low(&mut s4, if inc_hit != 0 { 2 } else { 8 });
        match etype(victim) {
            4 => {
                bullet_impact_fx(cp, s4 as u8);
                if rb(victim, 0x144) >= 0xC8 {
                    let oi = rp(victim, 0x160);
                    let dmg = match ru(oi, 0x48) {
                        1 => 151.0,
                        2 => x(rf(oi, 0x34)).fmul_m(151.0).to_f32(),
                        _ => 50.0,
                    };
                    object_damage(victim, dmg, cp, cp.byte_add(0x10), fired_by, ty());
                } else {
                    'force: {
                        if rb(victim, 0x40) & 4 != 0 {
                            break 'force;
                        }
                        let oi = rp(victim, 0x160);
                        let mut f = Fpu::new();
                        f.fld(rf(oi, 0x18));
                        f.fcomp_m(cf(0x42C7_CCCD));
                        if f.jp(5) {
                            break 'force;
                        }
                        if entity_get_is_static(victim) != 0 {
                            let mut f = Fpu::new();
                            f.fld(rf(oi, 0x14));
                            f.fcomp_m(0.0);
                            if !f.jp(0x41) {
                                entity_set_is_static(victim, 0);
                                physical_add_to_moving_list(victim);
                            }
                        }
                        if entity_get_is_static(victim) != 0 {
                            break 'force;
                        }
                        s4 = if rb(victim, 0x40) & 0xA0 != 0 { 0xBE4C_CCCD } else { 0xC000_0000 };
                        if inc_hit != 0 {
                            s4 = x(f32::from_bits(s4)).fmul_m(cf(0x3E4C_CCCD)).to_f32().to_bits();
                        }
                        push_at_hit(victim, cp, f32::from_bits(s4));
                    }
                    object_damage(victim, 50.0, cp, cp.byte_add(0x10), fired_by, ty());
                }
            }
            2 => {
                let piece = rb(cp, 0x24);
                if (0xD..=0x10).contains(&piece) {
                    set_low(&mut s4, piece);
                    vehicle_burst_tyre(victim, s4, 1);
                    fx_add_tyre_burst(g(G_FX), cp, cp.byte_add(0x10));
                    if !fired_by.is_null() && etype(fired_by) == 3 {
                        let driver = rp(victim, VEH_DRIVER);
                        if !driver.is_null() {
                            vehicle_damage_event(driver, victim, fired_by, ty());
                        }
                        s4 = 0;
                        if rb(victim, VEH_NUM_PASSENGERS) != 0 {
                            let mut slot = victim.byte_add(VEH_PASSENGERS);
                            loop {
                                let p = rp(slot, 0);
                                if !p.is_null() {
                                    vehicle_damage_event(p, victim, fired_by, ty());
                                }
                                s4 = s4.wrapping_add(1);
                                slot = slot.byte_add(4);
                                if (s4 as i32) >= i32::from(rb(victim, VEH_NUM_PASSENGERS)) {
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    s8 = xi(i32::from(rh(wi, 0x22) as i16)).to_f32().to_bits();
                    if !fired_by_player.is_null() {
                        let cam = usize::from(g::<u8>(0xB6F081).read());
                        if g::<u16>(0xB6F1A8 + cam * 0x238).read() == 0x31 {
                            let d = x(f32::from_bits(s8));
                            s8 = d.fadd(d).to_f32().to_bits();
                        }
                    }
                    vehicle_inflict_damage(victim, fired_by, ty(), f32::from_bits(s8), c[0], c[1], c[2]);
                    bullet_impact_fx(cp, s4 as u8);
                    // jump table 0x73C1D0 / 0x73C1DC over type - 0x18
                    s4 = match ty() {
                        24 | 38 => 0xC1A0_0000,
                        25 | 27 => 0xC080_0000,
                        _ => 0xC120_0000,
                    };
                    if g::<i32>(0xB7298C).read() < 2 {
                        let m = x(rf(victim, 0x8C)).fmul_m(cf(0x3A83_126F)).to_f32();
                        s4 = st0(min_f32(1.0, m)).fmul_m(f32::from_bits(s4)).to_f32().to_bits();
                        push_at_hit(victim, cp, f32::from_bits(s4));
                    }
                }
            }
            1 => {
                bullet_impact_fx(cp, s4 as u8);
                if !fired_by_player.is_null() {
                    let pd = rp(fired_by_player, 0x480);
                    if !pd.is_null() {
                        wu(pd, 0x9C, i32::from(rh(victim, 0x22) as i16) as u32);
                    }
                }
            }
            _ => {}
        }
        // 0x73C06C: the protection chunk 0x401705 reloads incrementalHit and jumps back to 0x73C073
        if inc_hit <= 0 {
            report_hit_angle(victim, cp, start, end);
        }
    }
    let pt = if fired_by.is_null() || etype(fired_by) != 3 { 2 } else { ru(fired_by, 0x598) };
    if pt != 0 && pt != 1 {
        return;
    }
    let p1 = pos_ptr(find_player_ped(-1));
    let p2 = pos_ptr(find_player_ped(-1));
    let p3 = pos_ptr(find_player_ped(-1));
    let (z, y, xx) = (ru(p1, 8), ru(p2, 4), ru(p3, 0));
    let pad = pad_get_pad(pt as i32);
    pad_start_shake_distance(pad, 0xF0, 0x80, xx, y, z);
}

pub const HOOKS: &[HookReg] = &[HOOK_GENERATE_DAMAGE_EVENT, HOOK_DO_BULLET_IMPACT];
