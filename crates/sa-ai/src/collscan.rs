//! `CCollisionEventScanner::ScanForCollisionEvents` (PORT-AI-5, helper A). Ported from the 1.0 US bytes
//! (0x604500-0x6052E3, the tail continues through the island 0x401CE1, which loads `victim->m_pPlayerData` and jumps
//! back to 0x605165); the spec `docs/re/specs/CALLED-ONLY/CCollisionEventScanner.md` (medium confidence) was re-read
//! branch by branch. gta-reversed has only a wrapper. Events go through their original constructors, `CEventGroup::Add`
//! and destructors via `orig!`; the events, the damage calculator and the vectors the original builds on its stack
//! are zeroed Rust locals of the same size (entity references they register point into the Rust stack and are
//! released by their destructors, as in `handler.rs`).
//!
//! `CCollisionEventScanner`: +0 `m_bAlreadyHitByCar` (u8). Victim fields: +0xD8 damage intensity, +0xDC damage
//! entity, +0xE0 impact velocity, +0xEC collision position, +0xF8 u16 piece, +0x8C mass, +0x550 / +0x554 (two floats
//! multiplied with the impact velocity), +0x46C bit 0 bIsStanding.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig;

use crate::common::*;

sa_core::globals! {
    list GLOBALS;
    /// Last "bump" sound event time of the balaclava branch (no gta-reversed name)
    pub static SCAN_LAST_BUMP_SOUND @ 0xC0B1B0: u32 = 0;
    /// Last "moving object" sound event time of the balaclava branch (no gta-reversed name)
    pub static SCAN_LAST_MOVING_SOUND @ 0xC0B1B4: u32 = 0;
    /// Intensity above which the bump sound has range 40 (else 30)
    pub static SCAN_LOUD_INTENSITY @ 0x8D239C: f32 = 3.0;
    /// Minimum intensity of the bump sound
    pub static SCAN_SOUND_MIN_INTENSITY @ 0x8D23A0: f32 = 1.0;
    /// Minimum |turn speed z| of a moving object for its sound
    pub static SCAN_MIN_TURN_SPEED @ 0x8D23A4: f32 = 0.04;
    /// Object damage scale
    pub static SCAN_OBJECT_DAMAGE_SCALE @ 0x8D23A8: f32 = 10.0;
    /// Object damage threshold for players
    pub static SCAN_OBJECT_THRESHOLD_PLAYER @ 0x8D23AC: f32 = 2.0;
    /// Object damage threshold for other peds
    pub static SCAN_OBJECT_THRESHOLD_PED @ 0x8D23B0: f32 = 1.0;
}

/// `CTimer::m_snTimeInMilliseconds` (owned by CTimer).
const TIME_MS: usize = 0xB7CB84;

orig! {
    /// `CTaskManager::GetSimplestActiveTask` (0x6819D0)
    fn tm_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360, cdecl)
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => unported;
    /// `FUN_00406da0` (thiscall on a CVector, ST0): the squared length `(x*x + y*y) + z*z`
    fn vector_squared_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CPed::KillPedWithCar(CVehicle*, float, bool)` (0x5F0360, `ret 0xC`)
    fn ped_kill_ped_with_car(p: GPtr<u8>, veh: GPtr<u8>, impulse: u32, b: u32) = thiscall 0x5F0360 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlaceable::GetHeading` (0x441DB0, ST0)
    fn placeable_get_heading(p: GPtr<u8>) -> f64 = thiscall 0x441DB0 => unported;
    /// `CEntity::GetColModel` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `FUN_004119d0(CVector* out, const CVector* v, float k)` (0x4119D0, cdecl): out = v * (1.0f / k)
    fn vector_div(out: GPtr<u8>, v: GPtr<u8>, k: u32) -> GPtr<u8> = cdecl 0x4119D0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, result in `out`)
    fn matrix_mul_vector(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn general_limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CVector::Normalise` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CVector::DotProduct(const CVector*, const CVector*)` (0x40FDB0, ST0)
    fn vector_dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `FUN_0040fe90(CVector* out, float k, const CVector* v)` (0x40FE90, cdecl): out = k * v
    fn vector_scale(out: GPtr<u8>, k: u32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `CPedGeometryAnalyser::ComputePedHitSide(const CPed&, const CPhysical&)` (0x5F3640, cdecl)
    fn compute_ped_hit_side(ped: GPtr<u8>, other: GPtr<u8>) -> u32 = cdecl 0x5F3640 => unported;
    /// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(CEntity*, float, eWeaponType, ePedPieceTypes,
    /// bool)` (0x4AD3F0, `ret 0x14`)
    fn calc_ctor(c: GPtr<u8>, src: GPtr<u8>, dmg: u32, wt: u32, piece: u32, b: u32) -> GPtr<u8> = thiscall 0x4AD3F0 => unported;
    /// `CPedDamageResponseCalculator::~CPedDamageResponseCalculator` (0x4AD420)
    fn calc_dtor(c: GPtr<u8>) = thiscall 0x4AD420 => unported;
    /// `CPedDamageResponseCalculator::ComputeDamageResponse(CPed*, CPedDamageResponse*, bool)` (0x4B5AC0, `ret 0xC`)
    fn calc_compute_damage_response(c: GPtr<u8>, ped: GPtr<u8>, resp: GPtr<u8>, b: u32) = thiscall 0x4B5AC0 => unported;
    /// `CEventDamage::CEventDamage(CEntity*, uint32 time, eWeaponType, ePedPieceTypes, uint8 direction, bool, bool)`
    /// (0x4AD830, `ret 0x1C`)
    fn damage_ctor(e: GPtr<u8>, src: GPtr<u8>, time: u32, wt: u32, piece: u32, dir: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x4AD830 => unported;
    /// `CEventDamage::AffectsPed(CPed*)` (0x4B35A0, `ret 4`)
    fn damage_affects_ped(e: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4B35A0 => unported;
    /// `CEventDamage::~CEventDamage` (0x4AD960)
    fn damage_dtor(e: GPtr<u8>) = thiscall 0x4AD960 => unported;
    /// `CPedIntelligence::GetMoveStateFromGoToTask` (0x601D70)
    fn intel_get_move_state_from_go_to_task(i: GPtr<u8>) -> u32 = thiscall 0x601D70 => unported;
    /// `CEventVehicleCollision::CEventVehicleCollision(int16 piece, float, CVehicle*, const CVector*, const CVector*,
    /// int8 moveState, int16)` (0x4AC840, `ret 0x1C`)
    fn vehicle_collision_ctor(e: GPtr<u8>, piece: u32, dmg: u32, veh: GPtr<u8>, vel: GPtr<u8>, pos: GPtr<u8>, ms: u32, a: u32) -> GPtr<u8> = thiscall 0x4AC840 => unported;
    /// `CEventVehicleCollision` destructor (0x4AC920)
    fn vehicle_collision_dtor(e: GPtr<u8>) = thiscall 0x4AC920 => unported;
    /// `CEventPlayerCollisionWithPed::CEventPlayerCollisionWithPed(int16, float, CPed*, CVector*, CVector*, int32,
    /// int32)` (0x5FEE40, `ret 0x1C`)
    fn player_collision_with_ped_ctor(e: GPtr<u8>, piece: u32, dmg: u32, ped: GPtr<u8>, vel: GPtr<u8>, pos: GPtr<u8>, ms: u32, oms: u32) -> GPtr<u8> = thiscall 0x5FEE40 => unported;
    /// `CEventPedCollisionWithPlayer::CEventPedCollisionWithPlayer(...)` (0x5FED40, `ret 0x1C`)
    fn ped_collision_with_player_ctor(e: GPtr<u8>, piece: u32, dmg: u32, ped: GPtr<u8>, vel: GPtr<u8>, pos: GPtr<u8>, ms: u32, oms: u32) -> GPtr<u8> = thiscall 0x5FED40 => unported;
    /// `CEventPedCollisionWithPed::CEventPedCollisionWithPed(...)` (0x4AC990, `ret 0x1C`)
    fn ped_collision_with_ped_ctor(e: GPtr<u8>, piece: u32, dmg: u32, ped: GPtr<u8>, vel: GPtr<u8>, pos: GPtr<u8>, ms: u32, oms: u32) -> GPtr<u8> = thiscall 0x4AC990 => unported;
    /// The ped-collision events' base destructor (0x4ACA70)
    fn ped_collision_dtor(e: GPtr<u8>) = thiscall 0x4ACA70 => unported;
    /// `CEventObjectCollision::CEventObjectCollision(int16, float, CObject*, const CVector*, const CVector*, int32)`
    /// (0x4ACCF0, `ret 0x18`)
    fn object_collision_ctor(e: GPtr<u8>, piece: u32, dmg: u32, obj: GPtr<u8>, vel: GPtr<u8>, pos: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x4ACCF0 => unported;
    /// `CEventObjectCollision` destructor (0x4ACDD0)
    fn object_collision_dtor(e: GPtr<u8>) = thiscall 0x4ACDD0 => unported;
    /// `CEventBuildingCollision::CEventBuildingCollision(int16, float, CBuilding*, const CVector*, const CVector*,
    /// int32)` (0x4ACF00, `ret 0x18`)
    fn building_collision_ctor(e: GPtr<u8>, piece: u32, dmg: u32, b: GPtr<u8>, vel: GPtr<u8>, pos: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x4ACF00 => unported;
    /// `CEventBuildingCollision` destructor (0x4ACFF0)
    fn building_collision_dtor(e: GPtr<u8>) = thiscall 0x4ACFF0 => unported;
    /// `CEventAcquaintancePedHate::CEventAcquaintancePedHate(CPed*)` (0x420E70, `ret 4`)
    fn acquaintance_ped_hate_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x420E70 => unported;
    /// The acquaintance events' base destructor (0x4AF890)
    fn acquaintance_dtor(e: GPtr<u8>) = thiscall 0x4AF890 => unported;
    /// `CEventSoundQuiet::CEventSoundQuiet(CEntity*, float, uint32, const CVector*)` (0x5E05B0, `ret 0x10`)
    fn sound_quiet_ctor(e: GPtr<u8>, ent: GPtr<u8>, range: u32, time: u32, pos: GPtr<u8>) -> GPtr<u8> = thiscall 0x5E05B0 => unported;
    /// `CEventSoundQuiet` destructor (0x5DEA00)
    fn sound_quiet_dtor(e: GPtr<u8>) = thiscall 0x5DEA00 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `RwHelper::GetEventGlobalGroup()` (0x4ABA50)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CPed::Say(uint16, uint32, float, bool, bool, bool)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, off: u32, prob: u32, a: u32, b: u32, c: u32) = thiscall 0x5EFFE0 => unported;
    /// `CMentalState::IncrementAnger(int32)` (0x421050, `ret 4`, a `.HOODLUM` stub), called on `m_pIntelligence`
    fn increment_anger(i: GPtr<u8>, n: u32) = thiscall 0x421050 => unported;
    /// `CAcquaintance::GetAcquaintances(int32)` (0x608970, `ret 4`)
    fn acquaintance_get(a: GPtr<u8>, i: i32) -> u32 = thiscall 0x608970 => unported;
    /// `CPedType::GetPedFlag(ePedType)` (0x608830, cdecl)
    fn ped_type_get_ped_flag(t: u32) -> u32 = cdecl 0x608830 => unported;
    /// `CEntity::GetIsStatic` (0x4633E0, a `.HOODLUM` stub)
    fn entity_get_is_static(e: GPtr<u8>) -> u8 = thiscall 0x4633E0 => unported;
    /// `FUN_00420800(float a, float b)` (0x420800, cdecl, ST0): `a > b ? a : b`
    fn float_max(a: u32, b: u32) -> f64 = cdecl 0x420800 => unported;
    /// `CPed::GetLocalDirection(const CVector2D&)` (0x5DEF60, `ret 4`)
    fn ped_get_local_direction(p: GPtr<u8>, dir: GPtr<u8>) -> u32 = thiscall 0x5DEF60 => unported;
    /// `CWeapon::GenerateDamageEvent(CPed*, CEntity*, eWeaponType, int32 damage, ePedPieceTypes, int32 direction)`
    /// (0x73A530, cdecl)
    fn weapon_generate_damage_event(ped: GPtr<u8>, src: GPtr<u8>, wt: u32, dmg: i32, piece: u32, dir: u32) = cdecl 0x73A530 => unported;
    /// `CEntity::GetIsTypePhysical` (0x4DA030)
    fn entity_get_is_type_physical(e: GPtr<u8>) -> u8 = thiscall 0x4DA030 => unported;
    /// `CPedClothesDesc::GetIsWearingBalaclava` (0x5A7950)
    fn clothes_get_is_wearing_balaclava(c: GPtr<u8>) -> u8 = thiscall 0x5A7950 => unported;
}

fn cf(addr: usize) -> X87 {
    x(g::<f32>(addr).read())
}

fn fm(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

fn now() -> u32 {
    g::<u32>(TIME_MS).read()
}

/// The entity's position: `m_matrix->pos` (+0x14 -> +0x30) when it has a matrix, else the placement (+4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

fn fv(b: &[u32], i: usize) -> f32 {
    f32::from_bits(b[i])
}

/// `(int16)` piece: `xor r, r; mov r16, [victim+0xF8]` (zero-extended dword).
fn piece(v: GPtr<u8>) -> u32 {
    rd::<u16>(v, 0xF8) as u32
}

/// The entity type (`+0x36 & 7`).
fn etype(e: GPtr<u8>) -> u8 {
    rd::<u8>(e, 0x36) & 7
}

/// `t = impactVel.x * v550 + impactVel.y * v554` (0x60464A / 0x604F90); `Some(t * mass + d)` when `t < 0`.
fn standing_push(v: GPtr<u8>, d: f32) -> Option<X87> {
    let a = x(rd::<f32>(v, 0xE4)).fmul_m(rd::<f32>(v, 0x554));
    let b = x(rd::<f32>(v, 0xE0)).fmul_m(rd::<f32>(v, 0x550));
    let t = a.fadd(b);
    if t.fcom(cf(0x858B50)).test_parity(5) {
        return None;
    }
    Some(t.fmul_m(rd::<f32>(v, 0x8C)).fadd_m(d))
}

/// The damage path of a ped attached to a vehicle (0x604942 / 0x604A32): hit side, a damage response calculator
/// and a `CEventDamage` (weapon 49, torso) on the stack; the response is computed when the event affects the ped.
/// The event is not added to any group. The last constructor argument is `(flags46C >> 8) & 0xFFFFFF01` (the
/// original pushes the whole dword).
fn attached_damage(v: GPtr<u8>, other: GPtr<u8>) {
    let side = compute_ped_hit_side(v, other);
    let mut calc = [0u32; 5];
    calc_ctor(local(&mut calc), other, rd::<u32>(v, 0xD8), 0x31, 3, 0);
    let in_veh = (rd::<u32>(v, 0x46C) >> 8) & 0xFFFF_FF01;
    let mut ev = [0u32; 0x11];
    damage_ctor(local(&mut ev), other, now(), 0x31, 3, side, 0, in_veh);
    if damage_affects_ped(local(&mut ev), v) != 0 {
        calc_compute_damage_response(local(&mut calc), v, local(&mut ev).byte_add(0x38), 1);
    }
    damage_dtor(local(&mut ev));
    calc_dtor(local(&mut calc));
}

/// Is the victim attached to a vehicle (`m_pAttachedTo` +0xFC of type 2)?
fn attached_to_vehicle(v: GPtr<u8>) -> bool {
    let a = rd::<GPtr<u8>>(v, 0xFC);
    !a.is_null() && etype(a) == 2
}

/// The vehicle-player kill test (0x6046A9-0x604920): the angle between the vehicle's heading and the direction to
/// the victim (from the bounding box centre) decides which velocity component counts: inside the front / back
/// sectors `dir . moveSpeed` (dir = normalised victim - vehicle position), on the sides the vehicle's right axis
/// (negated for a positive angle); a component > 0.1 kills the player with the clamped intensity `d`.
fn vehicle_hits_player(v: GPtr<u8>, other: GPtr<u8>, sp: f32, d: f32) {
    let mut d = d;
    if !x(d).fcom(cf(0x858BA4)).test_any(0x41) {
        d = 20.0;
    }
    let h = f32_of(X87::from_f64_exact(placeable_get_heading(other)));
    let cm = entity_get_col_model(other);
    let bmin = [rd::<f32>(cm, 0), rd::<f32>(cm, 4), rd::<f32>(cm, 8)];
    let cm = entity_get_col_model(other);
    let bmax = [rd::<f32>(cm, 0xC), rd::<f32>(cm, 0x10), rd::<f32>(cm, 0x14)];
    let cz = f32_of(x(bmax[2]).fadd_m(bmin[2]));
    let cy = f32_of(x(bmin[1]).fadd_m(bmax[1]));
    let cx = f32_of(x(bmin[0]).fadd_m(bmax[0]));
    let mut c = [cx.to_bits(), cy.to_bits(), cz.to_bits()];
    let mut half = [0u32; 3];
    vector_div(local(&mut half), local(&mut c), 0x4000_0000);
    let mut cw = [0u32; 3];
    let r = matrix_mul_vector(local(&mut cw), rd::<GPtr<u8>>(other, 0x14), local(&mut half));
    let mut cwc = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
    let mut tmp = [0u32; 3];
    let rel = vector_sub(local(&mut tmp), local(&mut cwc), pos(v));
    let (rx, ry) = (rd::<f32>(rel, 0), rd::<f32>(rel, 4));
    let a = x(rx).fchs().fpatan(x(ry));
    let arg = f32_of(x(h).fsub(a));
    let diff = f32_of(X87::from_f64_exact(general_limit_radian_angle(arg.to_bits())));
    let dyb = x(bmax[1]).fsub_m(bmin[1]);
    let dxb = x(bmax[0]).fsub_m(bmin[0]);
    let box_ang = f32_of(dxb.fpatan(dyb));
    let op = pos(other);
    let mut dir = [0u32; 3];
    vector_sub(local(&mut dir), pos(v), op);
    vector_normalise(local(&mut dir));
    let mv = other.byte_add(0x44);
    let ad = x(f32::from_bits(diff.to_bits() & 0x7FFF_FFFF));
    let front = !ad.fcom(x(box_ang)).test_parity(5) || !ad.fcom(cf(0x858CB8).fsub_m(box_ang)).test_any(0x41);
    let dot = if front {
        x(fv(&dir, 1))
            .fmul_m(rd::<f32>(mv, 4))
            .fadd(x(fv(&dir, 2)).fmul_m(rd::<f32>(mv, 8)))
            .fadd(x(fv(&dir, 0)).fmul_m(rd::<f32>(mv, 0)))
    } else if !x(diff).fcom(cf(0x858B50)).test_any(0x41) {
        let mut s = [0u32; 3];
        let r = vector_scale(local(&mut s), 0xBF80_0000, rd::<GPtr<u8>>(other, 0x14));
        X87::from_f64_exact(vector_dot_product(r, mv))
    } else {
        let right = rd::<GPtr<u8>>(other, 0x14);
        let dot = X87::from_f64_exact(vector_dot_product(right, mv));
        let stored = f32_of(dot);
        if !x(sp).fcom(cf(0x86CD7C)).test_any(0x41) && !dot.fcom(cf(0x858B1C)).test_parity(5) {
            // 0x6048B4: the right . victim-forward test only decides a dead store; the dot product is reloaded
            // from its f32 slot
            let _ = vector_dot_product(right, rd::<GPtr<u8>>(v, 0x14).byte_add(0x10));
            x(stored)
        } else {
            dot
        }
    };
    if !dot.fcom(cf(0x858B1C)).test_any(0x41) {
        ped_kill_ped_with_car(v, other, d.to_bits(), 0);
    }
}

/// Vehicle collision (0x60459D).
fn vehicle_collision(this: GPtr<u8>, v: GPtr<u8>, other: GPtr<u8>, group: GPtr<u8>) {
    if rd::<u8>(this, 0) != 0 {
        return;
    }
    let sp = f32_of(X87::from_f64_exact(vector_squared_magnitude(other.byte_add(0x44))));
    let thr = if rd::<u32>(other, 0x4C4) != 0 { fm(0x86C924) } else { fm(0x86C920) };
    if rd::<i32>(other, 0x594) == 6
        && rd::<u8>(other, 0x40) & 4 != 0
        && (rd::<u32>(v, 0x1C) >> 4) & 1 != 0
        && rd::<u8>(v, 0x46C) & 1 != 0
        && rd::<u32>(v, 0x568) == 0
        && !x(sp).fcom(cf(0x858FC4)).test_any(0x41)
    {
        ped_kill_ped_with_car(v, other, 0x4170_0000, 0);
    }
    if !x(sp).fcom(x(thr)).test_any(0x41) {
        let mut d = rd::<f32>(v, 0xD8);
        if rd::<u8>(v, 0x46C) & 1 != 0
            && let Some(d2) = standing_push(v, d)
        {
            d = if cf(0x858B50).fcom(d2).test_any(0x41) { f32_of(d2) } else { 0.0 };
        }
        if ped_is_player(v) != 0 {
            vehicle_hits_player(v, other, sp, d);
        } else if attached_to_vehicle(v) {
            attached_damage(v, other);
        } else {
            ped_kill_ped_with_car(v, other, rd::<u32>(v, 0xD8), 0);
        }
        return;
    }
    if attached_to_vehicle(v) {
        attached_damage(v, other);
        return;
    }
    if ped_is_player(v) != 0 {
        return;
    }
    let ms = intel_get_move_state_from_go_to_task(rd::<GPtr<u8>>(v, 0x47C));
    let mut ev = [0u32; 0xD];
    vehicle_collision_ctor(local(&mut ev), piece(v), rd::<u32>(v, 0xD8), rd::<GPtr<u8>>(v, 0xDC), v.byte_add(0xE0), v.byte_add(0xEC), ms, 0);
    event_group_add(group, local(&mut ev), 0);
    vehicle_collision_dtor(local(&mut ev));
}

/// Which ped-collision event (0x5FEE40 / 0x5FED40 / 0x4AC990) and the vtable its inlined destructor sets before
/// the base destructor 0x4ACA70 (none for `CEventPedCollisionWithPed`).
#[derive(Clone, Copy)]
enum PedEv {
    PlayerWithPed,
    PedWithPlayer,
    PedWithPed,
}

#[allow(clippy::too_many_arguments)]
fn ped_event(kind: PedEv, group: GPtr<u8>, pc: u32, dmg: u32, ped: GPtr<u8>, vel: GPtr<u8>, cpos: GPtr<u8>, ms: u32, oms: u32) -> [u32; 0xD] {
    let mut ev = [0u32; 0xD];
    let e = local(&mut ev);
    match kind {
        PedEv::PlayerWithPed => player_collision_with_ped_ctor(e, pc, dmg, ped, vel, cpos, ms, oms),
        PedEv::PedWithPlayer => ped_collision_with_player_ctor(e, pc, dmg, ped, vel, cpos, ms, oms),
        PedEv::PedWithPed => ped_collision_with_ped_ctor(e, pc, dmg, ped, vel, cpos, ms, oms),
    };
    event_group_add(group, e, 0);
    ev
}

fn ped_event_dtor(kind: PedEv, ev: &mut [u32; 0xD]) {
    match kind {
        PedEv::PlayerWithPed => ev[0] = 0x86C9B0,
        PedEv::PedWithPlayer => ev[0] = 0x86C970,
        PedEv::PedWithPed => {}
    }
    ped_collision_dtor(local(ev));
}

/// `CEventAcquaintancePedHate(target)` added to `group` when `GetAcquaintances(hater +0x4E0, 4)` has the target's
/// ped-type flag (0x604C7D / 0x604E7B; destructor: vtable 0x858E68, then 0x4AF890).
fn hate_if_hated(hater: GPtr<u8>, target: GPtr<u8>, group_of: impl Fn() -> GPtr<u8>) {
    let a = acquaintance_get(hater.byte_add(0x4E0), 4);
    if a & ped_type_get_ped_flag(rd::<u32>(target, 0x598)) == 0 {
        return;
    }
    let mut ev = [0u32; 6];
    acquaintance_ped_hate_ctor(local(&mut ev), target);
    event_group_add(group_of(), local(&mut ev), 0);
    ev[0] = 0x858E68;
    acquaintance_dtor(local(&mut ev));
}

/// Ped collision (0x604B56) and its mirror on the other ped (0x604D3E).
fn ped_collision(v: GPtr<u8>, other: GPtr<u8>, group: GPtr<u8>, ms: u32) {
    let oms = intel_get_move_state_from_go_to_task(rd::<GPtr<u8>>(other, 0x47C));
    let vel = v.byte_add(0xE0);
    let cpos = v.byte_add(0xEC);
    if ped_is_player(v) != 0 {
        let mut ev = ped_event(PedEv::PlayerWithPed, group, piece(v), rd::<u32>(v, 0xD8), rd::<GPtr<u8>>(v, 0xDC), vel, cpos, ms, oms);
        ped_say(other, 0x1C, 0, 0x3F80_0000, 0, 0, 0);
        ped_event_dtor(PedEv::PlayerWithPed, &mut ev);
    } else if ped_is_player(other) != 0 {
        let mut ev = ped_event(PedEv::PedWithPlayer, group, piece(v), rd::<u32>(v, 0xD8), rd::<GPtr<u8>>(v, 0xDC), vel, cpos, ms, oms);
        increment_anger(rd::<GPtr<u8>>(v, 0x47C), 1);
        ped_say(v, 0x1C, 0, 0x3F80_0000, 0, 0, 0);
        hate_if_hated(v, other, || group);
        ped_event_dtor(PedEv::PedWithPlayer, &mut ev);
    } else {
        let mut ev = ped_event(PedEv::PedWithPed, group, piece(v), rd::<u32>(v, 0xD8), rd::<GPtr<u8>>(v, 0xDC), vel, cpos, ms, oms);
        ped_event_dtor(PedEv::PedWithPed, &mut ev);
    }
    if !rd::<GPtr<u8>>(other, 0xDC).is_null() {
        return;
    }
    let m1 = fm(0x858C1C);
    let mut mv = [0u32; 3];
    for (k, c) in mv.iter_mut().enumerate() {
        *c = f32_of(x(rd::<f32>(vel, 4 * k as isize)).fmul_m(m1)).to_bits();
    }
    let og = || rd::<GPtr<u8>>(other, 0x47C).byte_add(0x68);
    let kind = if ped_is_player(other) != 0 {
        PedEv::PlayerWithPed
    } else if ped_is_player(v) != 0 {
        PedEv::PedWithPlayer
    } else {
        PedEv::PedWithPed
    };
    let mut ev = ped_event(kind, og(), piece(v), rd::<u32>(v, 0xD8), v, local(&mut mv), cpos, ms, oms);
    if let PedEv::PedWithPlayer = kind {
        increment_anger(rd::<GPtr<u8>>(other, 0x47C), 1);
        hate_if_hated(other, v, og);
    }
    ped_event_dtor(kind, &mut ev);
}

/// Object collision (0x604F70): a standing victim pushed hard enough by a non-static object (not its ground
/// object +0x584, not attached to itself) gets `CWeapon::GenerateDamageEvent` (weapon 54, torso, damage
/// `_ftol(d / thr * [0x8D23A8])`, direction from `GetLocalDirection(-impact.xy)`) and +0x128 = the damage entity;
/// else a `CEventObjectCollision`.
fn object_collision(v: GPtr<u8>, other: GPtr<u8>, group: GPtr<u8>, ms: u32) {
    let st = rd::<u8>(v, 0x46C) & 1;
    let mut d = rd::<f32>(v, 0xD8);
    if st != 0
        && entity_get_is_static(other) == 0
        && let Some(d2) = standing_push(v, d)
    {
        d = f32_of(X87::from_f64_exact(float_max(0, f32_of(d2).to_bits())));
    }
    let thr = if rd::<GPtr<u8>>(v, 0x480).is_null() {
        SCAN_OBJECT_THRESHOLD_PED.get()
    } else {
        SCAN_OBJECT_THRESHOLD_PLAYER.get()
    };
    let hit = !x(d).fcom(x(thr)).test_any(0x41)
        && entity_get_is_static(other) == 0
        && st != 0
        && other != rd::<GPtr<u8>>(v, 0x584)
        && {
            let a = rd::<GPtr<u8>>(other, 0xFC);
            a.is_null() || a != other
        };
    if hit {
        let dmg = f32_of(x(d).fdiv_m(thr).fmul_m(SCAN_OBJECT_DAMAGE_SCALE.get()));
        let mut dir2 = [0u32; 2];
        let ix = rd::<f32>(v, 0xE0);
        dir2[0] = f32_of(x(ix).fchs()).to_bits();
        dir2[1] = f32_of(x(rd::<f32>(v, 0xE4)).fchs()).to_bits();
        let ld = ped_get_local_direction(v, local(&mut dir2));
        weapon_generate_damage_event(v, rd::<GPtr<u8>>(v, 0xDC), 0x36, ftol32(x(dmg)), 3, ld);
        wr::<u32>(v, 0x128, rd::<u32>(v, 0xDC));
        return;
    }
    let mut ev = [0u32; 0xC];
    object_collision_ctor(local(&mut ev), piece(v), rd::<u32>(v, 0xD8), other, v.byte_add(0xE0), v.byte_add(0xEC), ms);
    event_group_add(group, local(&mut ev), 0);
    object_collision_dtor(local(&mut ev));
}

/// The balaclava tail (0x605165): a player wearing the balaclava hit by a building or object makes a quiet sound
/// event (range 40 for a fast-turning physical object at most every 2 s, else 40 / 30 by intensity at most every
/// 1 s) in the global event group.
fn balaclava_sound(v: GPtr<u8>) {
    let pd = rd::<GPtr<u8>>(v, 0x480);
    if pd.is_null() {
        return;
    }
    let de = rd::<GPtr<u8>>(v, 0xDC);
    if etype(de) != 1 && etype(de) != 4 {
        return;
    }
    if clothes_get_is_wearing_balaclava(rd::<GPtr<u8>>(pd, 4)) == 0 {
        return;
    }
    let e = rd::<GPtr<u8>>(v, 0xDC);
    let range: u32;
    if entity_get_is_type_physical(e) != 0 && rd::<u8>(e, 0x40) & 0x20 != 0 {
        let tz = x(f32::from_bits(rd::<u32>(e, 0x58) & 0x7FFF_FFFF));
        if tz.fcom(x(SCAN_MIN_TURN_SPEED.get())).test_any(0x41) {
            return;
        }
        let t = now();
        if t <= SCAN_LAST_MOVING_SOUND.get().wrapping_add(2000) {
            return;
        }
        range = 0x4220_0000;
        SCAN_LAST_MOVING_SOUND.set(t);
    } else {
        if x(rd::<f32>(v, 0xD8)).fcom(x(SCAN_SOUND_MIN_INTENSITY.get())).test_any(0x41) {
            return;
        }
        let t = now();
        if t <= SCAN_LAST_BUMP_SOUND.get().wrapping_add(1000) {
            return;
        }
        let r = if !x(rd::<f32>(v, 0xD8)).fcom(x(SCAN_LOUD_INTENSITY.get())).test_any(0x41) {
            cf(0x858A10)
        } else {
            cf(0x858CA4)
        };
        range = f32_of(r).to_bits();
        SCAN_LAST_BUMP_SOUND.set(t);
        if r.fcom(cf(0x858B50)).test_any(0x41) {
            return;
        }
    }
    let mut zero = [0u32; 3];
    let mut ev = [0u32; 0xB];
    sound_quiet_ctor(local(&mut ev), v, range, 0xFFFF_FFFF, local(&mut zero));
    event_group_add(get_event_global_group(), local(&mut ev), 0);
    sound_quiet_dtor(local(&mut ev));
}

/// `CCollisionEventScanner::ScanForCollisionEvents(CPed* victim, CEventGroup* group)` (0x604500, `ret 8`).
///
/// With a damage entity and an intensity that is not 0.0 (NaN passes): the victim's move state (`ms`: the simplest
/// active task's +8 for a go-to task, else 1), then by the damage entity's type: building -> `CEventBuildingCollision`;
/// vehicle -> `vehicle_collision`; ped -> `ped_collision`; object -> `object_collision`; then the balaclava sound.
/// Always clears `m_bAlreadyHitByCar`.
///
/// Ported, not registered (PORT-AI-5): the c-bug split run (ai5v2) found an output mismatch at frame 233 in the
/// entity reference pool (`CReferences::pEmptyList` 0xBA1768 and `aRefs` 0xB9B9A8 differ after the call): the
/// reference register/release order of the stack events differs from the original. Hook attribute was
/// `addr = 0x604500, conv = thiscall, expect = "6A FF 68 9D E9 83 00"`.
///
/// Phase B (STD-7): registered as a native so the native game runs it (there is no original to call); the
/// reference-pool order difference above is a known divergence of the native build (platform.md section 13).
#[sa_core::native(addr = 0x604500, conv = thiscall, name = "CCollisionEventScanner::ScanForCollisionEvents")]
pub fn scan_for_collision_events(this: GPtr<u8>, v: GPtr<u8>, group: GPtr<u8>) {
    if !rd::<GPtr<u8>>(v, 0xDC).is_null() && x(rd::<f32>(v, 0xD8)).fcom(cf(0x858B50)).test_parity(0x44) {
        let task = tm_get_simplest_active_task(rd::<GPtr<u8>>(v, 0x47C).byte_add(4));
        let ms = if !task.is_null() && task_is_go_to_task(task) != 0 {
            rd::<u32>(task, 8)
        } else {
            1
        };
        let other = rd::<GPtr<u8>>(v, 0xDC);
        match etype(other) {
            1 => {
                let mut ev = [0u32; 0xC];
                building_collision_ctor(local(&mut ev), piece(v), rd::<u32>(v, 0xD8), other, v.byte_add(0xE0), v.byte_add(0xEC), ms);
                event_group_add(group, local(&mut ev), 0);
                building_collision_dtor(local(&mut ev));
            }
            2 => vehicle_collision(this, v, other, group),
            3 => ped_collision(v, other, group, ms),
            4 => object_collision(v, other, group, ms),
            _ => {}
        }
        balaclava_sound(v);
    }
    wr::<u8>(this, 0, 0);
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[];
