//! `CPhysical` collision response between two physicals (batch `mvph-phys-1`): the four-argument
//! `ApplyCollision` (0x548680) and `ApplySoftCollision` (0x54A2C0) that `ProcessCollisionSectorList`
//! runs for a pair. Ported from the 1.0 US bytes (SecuROM fragments inlined), x87 sequences through
//! the [`Fpu`] model (PC = 24) in the original's operand order; values the original keeps on the x87
//! stack between stores stay on the model's stack (`X87` values in Rust). gta-reversed
//! `Entity/Physical.cpp` was a reading aid only; where it differs, the bytes win (noted per hook).
//! Unverified (docs/verify/HOWTO.md).
//!
//! `CColPoint` offsets: point +0, normal +0x10, surfaceA +0x20, surfaceB +0x23, depth +0x28.
//! Physical flags (+0x40) used: 1 mass twice as big, 4 disable collision force, 8 collidable,
//! 0x10 disable turn force, 0x20 disable move force, 0x40 infinite mass, 0x2000 don't apply speed,
//! 0x4000000 add moving collision speed. Entity flags (+0x1C): 1 uses collision, 4 / 0x40000 static
//! (and waiting for collision), 0x10 stuck, 0x20 in safe position, 0x1000 has hit wall.
#![allow(clippy::unreadable_literal, clippy::needless_range_loop, clippy::collapsible_if, clippy::needless_late_init)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::attach::{ped_is_player, set_is_static_v};
use super::base::add_to_moving_list;
use super::collide::{apply_collision_call, report_collision, AUDIO_ENGINE};
use super::forces::{apply_force_call, apply_friction_force_call, apply_move_force_call, get_speed_call};
use super::friction::G_FX;
use super::friction_attach::{add_moving_collision_speed_v, scalar_mul_vector, vector_add_assign, vector_div, vector_sq_mag};
use super::shared::*;
use crate::collision::fpu::Fpu;

/// `CWorld::PlayerInFocus` (0xB7CD74, uint8)
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
/// `CWorld::Players[0].m_nHavocCaused` (0xB7CED8; players are 0x190 bytes apart)
const PLAYER_HAVOC_CAUSED: usize = 0xB7CED8;
/// Size of a `CPlayerInfo`.
const PLAYER_INFO_SIZE: usize = 0x190;
/// `ModelIndices::MI_FIRE_HYDRANT` (0x8CD594, int16)
const MI_FIRE_HYDRANT: usize = 0x8CD594;
/// `ModelIndices::MI_PARKINGMETER` (0x8CD548, int16)
const MI_PARKINGMETER: usize = 0x8CD548;
/// `ModelIndices::MI_PARKINGMETER2` (0x8CD54C, int16)
const MI_PARKINGMETER2: usize = 0x8CD54C;
/// `CCheat::m_aCheatsActive[CHEAT_CARS_FLOAT_AWAY_WHEN_HIT]` (0x969166, uint8)
const CHEAT_CARS_FLOAT_AWAY: usize = 0x969166;
/// `STAT_COST_OF_PROPERTY_DAMAGED`
const STAT_PROPERTY_DAMAGED: u32 = 0x7D;
/// The `double` 0.01 at 0x85A308 (`fcomp qword`).
const D_85A308: u64 = 0x3F84_7AE1_47AE_147B;

orig! {
    /// `CGlass::IsObjectGlass(CEntity*)` (0x46A760, cdecl, AL)
    pub fn is_object_glass(e: Ent) -> u8 = cdecl 0x46a760 => unported;
    /// `CGlass::WindowRespondsToCollision(CEntity*, float, CVector speed, CVector point, bool)`
    /// (0x71BC40, cdecl; both vectors by value)
    pub fn window_responds_to_collision(e: Ent, damage: f32, sx: f32, sy: f32, sz: f32, px: f32, py: f32, pz: f32,
        explosion: u32) = cdecl 0x71bc40 => unported;
    /// `CGlass::WindowRespondsToSoftCollision(CEntity*, float)` (0x71AF70, cdecl)
    pub fn window_responds_to_soft_collision(e: Ent, damage: f32) = cdecl 0x71af70 => unported;
    /// `CObject::ObjectDamage(float, const CVector*, const CVector*, CEntity*, eWeaponType)` (0x5A0D90)
    pub fn object_damage(o: Ent, damage: f32, point: Ent, normal: Ent, damager: Ent, weapon: u32)
        = thiscall 0x5a0d90 => unported;
    /// `CPickups::CreateSomeMoney(CVector, int32)` (0x458970, cdecl, the vector by value)
    pub fn create_some_money(x: f32, y: f32, z: f32, amount: i32) = cdecl 0x458970 => unported;
    /// `Fx_c::TriggerWaterHydrant(CVector&)` (0x4A0D70, thiscall on `g_fx`)
    pub fn trigger_water_hydrant(fx: Ent, pos: Ent) = thiscall 0x4a0d70 => unported;
    /// `CStats::IncrementStat(eStats, float)` (0x55C180, cdecl)
    pub fn increment_stat(stat: u32, value: f32) = cdecl 0x55c180 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0, cdecl)
    pub fn find_player_vehicle(id: i32, remote: u32) -> Ent = cdecl 0x56e0d0 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, cdecl, out first)
    pub fn vector_mul(out: Ent, v: Ent, s: f32) -> Ent = cdecl 0x40fec0 => crate::world::mvpa_math::vector_mul_scalar;
    /// `CPhysical::UnsetIsInSafePosition()` (0x548320)
    pub fn unset_is_in_safe_position_call(e: Ent) = thiscall 0x548320 => crate::physical::attach::unset_is_in_safe_position;
}

fn eflags(e: Ent) -> u32 {
    ru(e, EFLAGS)
}

/// Entity flag 0x1000 (`bHasHitWall`).
fn hit_wall(e: Ent) -> bool {
    eflags(e) & 0x1000 != 0
}

/// `AudioEngine.ReportCollision(a, b, surfA, surfB, cp.point, normal, impact, 1.0f, false, false)`.
fn report(a: Ent, b: Ent, surf_a: u8, surf_b: u8, cp: Ent, impact: f32) {
    report_collision(
        GPtr::from_addr(AUDIO_ENGINE),
        a,
        b,
        u32::from(surf_a),
        u32::from(surf_b),
        cp,
        cp.byte_add(0x10),
        impact,
        0x3F80_0000,
        0,
        0,
    );
}

/// `cp.point - e.matrix.pos` (each component `fld; fsub; fstp`; the matrix is not null-checked).
fn point_rel(cp: Ent, e: Ent) -> [f32; 3] {
    let pos = matrix(e).byte_add(0x30);
    let mut f = Fpu::new();
    let mut r = [0f32; 3];
    for (i, r) in r.iter_mut().enumerate() {
        f.fld(rf(cp, 4 * i as isize));
        f.fsub_m(rf(pos, 4 * i as isize));
        *r = f.fstp();
    }
    r
}

/// `GetSpeed(r)` of `e`.
fn speed(e: Ent, r: [f32; 3]) -> [f32; 3] {
    let mut out = CVector::new(0.0, 0.0, 0.0);
    rv(get_speed_call(e, lp(&mut out), cv(r)).cast(), 0)
}

/// `v += e->AddMovingCollisionSpeed(r)` (virtual +0xC4, then `CVector::operator+=` 0x411A00) when `e`
/// has physical flag 0x4000000, is a vehicle and `surface` is 0x41 (car moving component).
fn add_moving_speed(e: Ent, v: &mut [f32; 3], r: [f32; 3], surface: u8) {
    if pflags(e) & 0x400_0000 != 0 && etype(e) == ty::VEHICLE && surface == 0x41 {
        let mut rr = cv(r);
        let mut out = CVector::new(0.0, 0.0, 0.0);
        let m = add_moving_collision_speed_v(e, lp(&mut out).cast(), lp(&mut rr).cast());
        let mut vv = cv(*v);
        vector_add_assign(lp(&mut vv).cast(), m);
        *v = va(vv);
    }
}

/// `(a.y*n.y + a.z*n.z) + a.x*n.x` (the order of the speed dot products of the stored speeds).
fn dot_yzx(a: [f32; 3], n: [f32; 3]) -> f32 {
    let mut f = Fpu::new();
    f.fld(a[1]);
    f.fmul_m(n[1]);
    f.fld(a[2]);
    f.fmul_m(n[2]);
    f.faddp(1);
    f.fld(a[0]);
    f.fmul_m(n[0]);
    f.faddp(1);
    f.fstp()
}

/// `(a.z*n.z + a.y*n.y) + a.x*n.x` (move speed against the normal).
fn dot_zyx(f: &mut Fpu, a: [f32; 3], n: [f32; 3]) {
    f.fld(a[2]);
    f.fmul_m(n[2]);
    f.fld(a[1]);
    f.fmul_m(n[1]);
    f.faddp(1);
    f.fld(a[0]);
    f.fmul_m(n[0]);
    f.faddp(1);
}

/// `a - b` per component (`fld; fsub; fstp`).
fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    let mut f = Fpu::new();
    let mut r = [0f32; 3];
    for i in 0..3 {
        f.fld(a[i]);
        f.fsub_m(b[i]);
        r[i] = f.fstp();
    }
    r
}

/// `1 / (|d x n|² / (factor*turnMass) [+ 1/(factor*mass)])`, the second term only without physical
/// flag 0x20 (bDisableMoveForce) when `check_move` (`ApplySoftCollision` always adds it); the cross product through `CVector::CrossProduct` 0x59C730, its
/// squares summed on the x87 stack (`x² + y²) + z²`), the result stored as f32.
fn pair_mass(e: Ent, d: [f32; 3], n: Ent, factor: f32, check_move: bool) -> f32 {
    let mut dv = cv(d);
    let mut out = CVector::new(0.0, 0.0, 0.0);
    let c = rv(cross_product(lp(&mut out), lp(&mut dv), n.cast()).cast(), 0);
    let mut f = Fpu::new();
    f.fld(c[2]);
    f.fld(c[1]);
    f.fld(c[0]);
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fld(factor);
    f.fmul_m(rf(e, TURN_MASS));
    f.fdivp(1);
    if !check_move || pflags(e) & 0x20 == 0 {
        f.fld(factor);
        f.fmul_m(rf(e, MASS));
        f.fdivr_m(cf(k::K_858624));
        f.faddp(1);
    }
    f.fdivr_m(cf(k::K_858624));
    f.fstp()
}

/// The shared tail of the three general branches: elasticity `el = (e.elasticity + this.elasticity)
/// times 0.5`; `thisV = hitWall(this) ? ms : ms - el*diff`; `entV = hitWall(e) ? ms : ms - (dotE -
/// ms)*el`; `*d1 = (thisV - dotT)*mT`; `*d2 = -((entV - dotE)*mE)`. Returns the model with `*d2`
/// (unrounded) in st0.
#[allow(clippy::too_many_arguments)]
fn elastic(this: Ent, e: Ent, ms: f32, diff: f32, dot_t: f32, dot_e: f32, m_t: f32, m_e: f32, d1: GPtr<f32>,
           d2: GPtr<f32>) -> Fpu {
    let mut f = Fpu::new();
    f.fld(rf(e, ELASTICITY));
    f.fadd_m(rf(this, ELASTICITY));
    f.fmul_m(cf(k::K_858B8C));
    let el = f.fstp();
    if hit_wall(this) {
        f.fld(ms);
    } else {
        f.fld(el);
        f.fmul_m(diff);
        f.fsubr_m(ms);
    }
    if hit_wall(e) {
        f.fld(ms);
    } else {
        f.fld(dot_e);
        f.fsub_m(ms);
        f.fmul_m(el);
        f.fsubr_m(ms);
    }
    f.fxch(1);
    f.fsub_m(dot_t);
    f.fmul_m(m_t);
    d1.write(f.fstp());
    f.fsub_m(dot_e);
    f.fmul_m(m_e);
    f.fchs();
    f.fld_st(0);
    d2.write(f.fstp());
    f
}

/// The two force vectors after [`elastic`]: `n * (*d1 / facT)` and `n * -(d2 / facE)` (d2 the value
/// still on the stack).
fn forces(mut f: Fpu, n: [f32; 3], d1: GPtr<f32>, fac_t: f32, fac_e: f32) -> ([f32; 3], [f32; 3]) {
    let mut t = [0f32; 3];
    let mut e = [0f32; 3];
    f.fld(d1.read());
    f.fdiv_m(fac_t);
    f.fld_st(0);
    f.fmul_m(n[0]);
    t[0] = f.fstp();
    f.fld_st(0);
    f.fmul_m(n[1]);
    t[1] = f.fstp();
    f.fmul_m(n[2]);
    t[2] = f.fstp();
    f.fdiv_m(fac_e);
    f.fchs();
    f.fld_st(0);
    f.fmul_m(n[0]);
    e[0] = f.fstp();
    f.fld_st(0);
    f.fmul_m(n[1]);
    e[1] = f.fstp();
    f.fmul_m(n[2]);
    e[2] = f.fstp();
    (t, e)
}

/// `ms = (mE*dotE + mT*dotT) / (mE + mT)` in the bytes' operand order, stored.
fn mix(m_e: f32, dot_e: f32, m_t: f32, dot_t: f32) -> f32 {
    let mut f = Fpu::new();
    f.fld(m_e);
    f.fmul_m(dot_e);
    f.fld(m_t);
    f.fmul_m(dot_t);
    f.faddp(1);
    f.fld(m_e);
    f.fadd_m(m_t);
    f.fdivp(1);
    f.fstp()
}

/// `dotT - ms` stored; true when it is not below 0 (the `jp` exits returning false).
fn diff_not_negative(dot_t: f32, ms: f32, diff: &mut f32) -> bool {
    let mut f = Fpu::new();
    f.fld(dot_t);
    f.fsub_m(ms);
    *diff = f.fst();
    f.fcomp_m(cf(k::K_858B50));
    f.jp(5)
}

/// `CWorld::Players[PlayerInFocus].m_nHavocCaused += 2` and `IncrementStat(STAT_COST_OF_PROPERTY_DAMAGED,
/// rand() % 30 + 30)` (the `rand` at `site`).
fn property_damage(site: u32) {
    let pif = usize::from(GPtr::<u8>::from_addr(PLAYER_IN_FOCUS).read());
    let havoc = GPtr::<i32>::from_addr(PLAYER_HAVOC_CAUSED + pif * PLAYER_INFO_SIZE);
    havoc.write(havoc.read().wrapping_add(2));
    let r = sa_core::orig::at_site(site, sa_core::crt::rand) % 30 + 30;
    let mut f = Fpu::new();
    f.fild(r);
    increment_stat(STAT_PROPERTY_DAMAGED, f.fstp());
}

/// `CGlass::WindowRespondsToCollision(e, damage, this.moveSpeed, cp.point, false)`.
fn window_collision(this: Ent, e: Ent, damage: f32, cp: Ent) {
    let s = rv(this, MOVE_SPEED);
    let p = rv(cp, 0);
    window_responds_to_collision(e, damage, s[0], s[1], s[2], p[0], p[1], p[2], 0);
}

/// `CPhysical::ApplyCollision(CEntity* entity, CColPoint& cp, float& thisDamage, float& entityDamage)`
/// (0x548680, `ret 0x10`, AL), the collision response of two physicals.
///
/// Mass factors: `facT` (local +0x68) = 10 when the entity has bDisableTurnForce (0x10) and this not
/// bDisableMoveForce (0x20) (`pedStandingOnThis` when the entity is a ped standing on this, +0x568),
/// else 2/1 by this' flag 1. `facE` (+0x40) = 1; with this bDisableTurnForce: a player ped against
/// a vehicle with status 4/5 or this' entity flag 0x1000 gets `1 / (max(mass - 2000, 0) * 0.0002 +
/// 1)` (0x8631F0, 0x85A7D0), else 10 unless the entity has 0x20; a ped standing on the entity (+0x568)
/// sets `thisStandingOnEntity` and 10. Without it: a towing vehicle (+0x4C8) `(towed.mass +
/// mass)/mass`, else 2/1 by the entity's flag 1. `entNoForce` (+0x5B): the entity has 4 without 8,
/// or is attached (+0xFC) without 0x40; it clears `thisStandingOnEntity`. Centres of mass
/// `Multiply3x3(matrix, com)`, zero with 0x40. A vehicle against an object with object flag 0x100
/// (+0x140), or the reverse, zeroes `n.z` in the col point.
///
/// A static entity (entity flags 4 / 0x40000) without `entNoForce`: with this bDisableTurnForce, `s =
/// move . n` (zyx); `s < 0`: a non-object gets `ApplyCollision`-1 (with 4, its result returned) or
/// `SetIsStatic(false)`; an object: both damages `-(s*mass)`, `ApplyCollision`-1 for 4, uproot limit
/// (`m_pObjectInfo` +0x160, +0x14) >= 9999 or damage <= limit; glass: `WindowRespondsToCollision`;
/// else (without 4) `SetIsStatic(false)`, havoc += 2, stat 0x7D += rand()%30+30. Otherwise `r =
/// cp.point - pos`, `v = GetSpeed(r)` (+ `AddMovingCollisionSpeed` for this' surface A), `s = v . n`
/// (yzx); `s < 0`: a non-object as above (always the havoc and stat); an object: collision mass `m`
/// of `(r - com) x n`, `damage = hitWall ? m*s*-1 : -((elasticity+1)*m*s)` (both outputs), multiplier
/// 3 for a vehicle of subtype 9 (+0x594) else 1; with `m_nColDamageEffect` (+0x144) and `mult*damage
/// > 20`: `ObjectDamage(mult*damage, point, n, this, 55)`; an entity that no longer uses collision:
/// `ApplyForce((n*damage) / (2*colDamageMultiplier), r, true)` without this 4, report impact
/// `(colDamageMult/m)*damage*2`, false. Then 4 or limit >= 9999 or (damage <= limit and not (this
/// stuck 0x10 and hit wall)): `WindowRespondsToSoftCollision` for glass and `ApplyCollision`-1.
/// Else glass `WindowRespondsToCollision` / `SetIsStatic(false)`; a fire hydrant (not exploded,
/// +0x140 bit 0x40) `TriggerWaterHydrant(pos)`, a parking meter `CreateSomeMoney(pos, rand()%100)`,
/// an object whose info has no `m_bCausesExplosion` (+0x1F) — these three set bit 0x40; without
/// this 4 and limit > 200: `ApplyForce((0.2*n)*damage, r, true)` (G_8CD7F8). A still-static entity
/// returns false; else `AddToMovingList` unless it has 4.
///
/// Both bDisableTurnForce: `dotT`, `dotE` (zyx move speeds), `ms` = dotT (this 4 or 0x2000, the
/// entity force applied), dotE (entity 4 or 0x2000), `(mT*dotT*4 + mE*dotE)/(mT*4 + mE)` (this'
/// +0x474 sign bit, entity force applied, G_8CD7FC) or `min(dotE, 0)`. `dotT - ms >= 0`: false.
/// Elasticity as in [`elastic`]; `*d1 = (thisV - dotT)*mass`, `ApplyMoveForce(n*d1)` and report
/// unless this 4 / 0x2000; the entity: `*d2 = -((entV - dotE)*mass)`, `ApplyMoveForce(n*-d2)` (after
/// `UnsetIsInSafePosition`) and report unless 4 / 0x2000. True.
///
/// This bDisableTurnForce only: entity `r`, speed (+ moving speed for surface B), the entity's z
/// kept unrounded on the x87 stack, `mT = facT*mass`, `mE` = pair mass of the entity with `facE`,
/// `ms = entNoForce ? dotE : mix`. Forces `n*(d1/facT)` (z clamped at 0, x/y doubled for
/// `thisStandingOnEntity`, `ApplyMoveForce` unless this 4) and `n*-(d2/facE)` (`ApplyForce(.., rE,
/// true)` unless the entity has 4 or `thisStandingOnEntity`). Entity bDisableTurnForce only: this'
/// `r`, speed (+ moving speed: the bytes test the flag set, gta-reversed inverts it), `mT` = pair
/// mass with `facT`, `mE = facE*mass`; this force (z clamped) through `ApplyForce` unless this 4 or
/// `pedStandingOnThis`; the entity force: z < 0 → 0 and, when `|dotT| < 0.01` (double), x/y halved;
/// doubled for `pedStandingOnThis`; `ApplyMoveForce`. Neither: both `r`, speeds and pair masses; a
/// vehicle without hit wall and 4: `n.z < 0.7` scales the force z by 0.3, status 0 scales `r` by
/// 0.8, `bNoMoreCollisionTorque` applies `ApplyFrictionForce(f*-0.3, r)`; the same for the entity
/// with `-n.z`; the cars-float-away cheat clears bApplyGravity (2) of a non-mission vehicle hit by
/// the player's vehicle (either way); `ApplyForce` for both (4 excluded). Every true exit reports
/// `d1/mT` for this and (except the first) `d2/mE` for the entity.
#[hook(addr = 0x548680, conv = thiscall, name = "CPhysical::ApplyCollision-2", expect = "81 EC 90 00 00 00",
       this = "CPhysical", unverified = "mvph-phys-1", route = "L2", hazards = "none", recs = "grove-walk,driving",
       note = "rand at 0x548A50 / 0x548C7F (return 0x548A55 / 0x548C84) and 0x548FB3 (0x548FB8); calls ApplyCollision-1 with values left on the x87 stack (depth only)")]
pub fn apply_collision_2(this: Ent, ent: Ent, cp: Ent, d1: GPtr<f32>, d2: GPtr<f32>) -> u8 {
    let pf_e = pflags(ent);
    let pf_t = pflags(this);
    let mut this_on_ent = false; // +0x1F
    let mut ped_on_this = false; // +0x47
    let mut ent_no_force = false; // +0x5B
    let fac_t: f32; // +0x68
    let mut fac_e: f32 = 1.0; // +0x40
    if pf_e & 0x10 != 0 && pf_t & 0x20 == 0 {
        fac_t = 10.0;
        if etype(ent) == ty::PED && rp(ent, 0x568) == this {
            ped_on_this = true;
        }
    } else {
        fac_t = if pf_t & 1 != 0 { 2.0 } else { 1.0 };
    }
    if pf_t & 0x10 != 0 {
        let player_vs_car = etype(this) == ty::PED && ped_is_player(this) != 0 && etype(ent) == ty::VEHICLE && {
            let st = rb(ent, TYPE) >> 3;
            st == 4 || st == 5 || eflags(this) & 0x1000 != 0
        };
        if player_vs_car {
            let mut f = Fpu::new();
            f.fld(rf(ent, MASS));
            f.fsub_m(cf(k::K_8631F0));
            f.fld(cf(k::K_858B50));
            f.fcomp_st(1);
            if !f.jne(0x41) {
                f.fstp_st(0);
                f.fld(cf(k::K_858B50));
            }
            f.fmul_m(cf(k::K_85A7D0));
            f.fadd_m(cf(k::K_858624));
            f.fdivr_m(cf(k::K_858624));
            fac_e = f.fstp();
        } else if pf_e & 0x20 == 0 {
            fac_e = 10.0;
        }
        if etype(this) == ty::PED && rp(this, 0x568) == ent {
            this_on_ent = true;
            fac_e = 10.0;
        }
    } else if etype(this) == ty::VEHICLE && !rp(this, 0x4C8).is_null() {
        let towed = rp(this, 0x4C8);
        let mut f = Fpu::new();
        f.fld(rf(towed, MASS));
        f.fadd_m(rf(this, MASS));
        f.fdiv_m(rf(this, MASS));
        fac_e = f.fstp();
    } else {
        fac_e = if pf_e & 1 != 0 { 2.0 } else { 1.0 };
    }
    if (pf_e & 4 != 0 && pf_e & 8 == 0) || (!rp(ent, ATTACHED_TO).is_null() && pf_e & 0x40 == 0) {
        ent_no_force = true;
        this_on_ent = false;
    }
    let mut co_t = CVector::new(0.0, 0.0, 0.0);
    let mut com_t = rv(multiply3x3(lp(&mut co_t), matrix(this), this.byte_add(CENTRE_OF_MASS).cast()).cast(), 0);
    let mut co_e = CVector::new(0.0, 0.0, 0.0);
    let mut com_e = rv(multiply3x3(lp(&mut co_e), matrix(ent), ent.byte_add(CENTRE_OF_MASS).cast()).cast(), 0);
    if pflags(this) & 0x40 != 0 {
        com_t = [0.0; 3];
    }
    if pflags(ent) & 0x40 != 0 {
        com_e = [0.0; 3];
    }
    let lamp = |v: Ent, o: Ent| {
        etype(v) == ty::VEHICLE && etype(o) == ty::OBJECT && ru(o, 0x140) & 0x100 != 0
    };
    if lamp(this, ent) || lamp(ent, this) {
        wf(cp, 0x18, 0.0);
    }
    let n_ptr = cp.byte_add(0x10);

    // ---- a static entity ----------------------------------------------------------------------
    if (eflags(ent) & 4 != 0 || eflags(ent) & 0x4_0000 != 0) && !ent_no_force {
        if let Some(v) = static_entity(this, ent, cp, d1, d2, com_t) {
            return v;
        }
    }
    let n = rv(n_ptr, 0);
    let pf_t = pflags(this);
    let pf_e = pflags(ent);

    // ---- both bDisableTurnForce ---------------------------------------------------------------
    if pf_t & 0x10 != 0 && pf_e & 0x10 != 0 {
        let mut f = Fpu::new();
        dot_zyx(&mut f, rv(this, MOVE_SPEED), n);
        let m_t = rf(this, MASS); // +0x28
        let m_e = rf(ent, MASS); // +0x2C
        let mut fe = Fpu::new();
        dot_zyx(&mut fe, rv(ent, MOVE_SPEED), n);
        let dot_e = fe.fstp(); // +0x30
        let mut apply_ent = false;
        let ms: f32; // +0x20
        if pf_t & 4 != 0 || pf_t & 0x2000 != 0 {
            ms = f.fst();
            apply_ent = true;
        } else if pf_e & 4 != 0 || pf_e & 0x2000 != 0 {
            ms = dot_e;
        } else if (ru(this, 0x474) as i32) < 0 {
            f.fld(m_t);
            f.fmul_st(1);
            f.fmul_m(G_8CD7FC.get());
            f.fld(m_e);
            f.fmul_m(dot_e);
            f.faddp(1);
            f.fld(m_t);
            f.fmul_m(G_8CD7FC.get());
            f.fadd_m(m_e);
            f.fdivp(1);
            ms = f.fstp();
            apply_ent = true;
        } else {
            let mut g = Fpu::new();
            g.fld(dot_e);
            g.fcomp_m(cf(k::K_858B50));
            ms = if g.jp(5) { 0.0 } else { dot_e };
        }
        f.fld(ms);
        f.fsubr_st(1);
        let diff = f.fst(); // +0x54
        f.fcomp_m(cf(k::K_858B50));
        if f.jp(5) {
            return 0;
        }
        let mut g = Fpu::new();
        g.fld(rf(ent, ELASTICITY));
        g.fadd_m(rf(this, ELASTICITY));
        g.fmul_m(cf(k::K_858B8C));
        let el = g.fstp(); // +0x24
        if hit_wall(this) {
            f.fld(ms);
        } else {
            f.fld(el);
            f.fmul_m(diff);
            f.fsubr_m(ms);
        }
        f.fsubrp(1);
        f.fmul_m(m_t);
        f.fld_st(0);
        d1.write(f.fstp());
        let mut fv = [0f32; 3];
        f.fld_st(0);
        f.fmul_m(n[0]);
        fv[0] = f.fstp();
        f.fld_st(0);
        f.fmul_m(n[1]);
        fv[1] = f.fstp();
        f.fmul_m(n[2]);
        fv[2] = f.fstp();
        let pf_t = pflags(this);
        if pf_t & 4 == 0 && pf_t & 0x2000 == 0 {
            apply_move_force_call(this, cv(fv));
            let mut g = Fpu::new();
            g.fld(d1.read());
            g.fdiv_m(m_t);
            report(this, ent, rb(cp, 0x20), rb(cp, 0x23), cp, g.fstp());
        }
        if !apply_ent {
            return 1;
        }
        let mut f = Fpu::new();
        if hit_wall(ent) {
            f.fld(ms);
        } else {
            f.fld(dot_e);
            f.fsub_m(ms);
            f.fmul_m(el);
            f.fsubr_m(ms);
        }
        f.fsub_m(dot_e);
        f.fmul_m(m_e);
        f.fchs();
        f.fld_st(0);
        d2.write(f.fstp());
        f.fchs();
        f.fld_st(0);
        f.fmul_m(n[0]);
        fv[0] = f.fstp();
        f.fld_st(0);
        f.fmul_m(n[1]);
        fv[1] = f.fstp();
        f.fmul_m(n[2]);
        fv[2] = f.fstp();
        let pf_e = pflags(ent);
        if pf_e & 4 == 0 && pf_e & 0x2000 == 0 {
            if eflags(ent) & 0x20 != 0 {
                unset_is_in_safe_position_call(ent);
            }
            apply_move_force_call(ent, cv(fv));
            let mut g = Fpu::new();
            g.fld(d2.read());
            g.fdiv_m(m_e);
            report(ent, this, rb(cp, 0x23), rb(cp, 0x20), cp, g.fstp());
        }
        return 1;
    }

    if pf_t & 0x10 != 0 {
        // ---- this bDisableTurnForce only --------------------------------------------------------
        let r_e = point_rel(cp, ent); // +0x70
        let v = speed(ent, r_e); // +0x34
        let mut f = Fpu::new();
        let mut vx = v;
        if pflags(ent) & 0x400_0000 != 0 && etype(ent) == ty::VEHICLE && rb(cp, 0x23) == 0x41 {
            // the sum of x and y is stored, z stays on the x87 stack
            let mut rr = cv(r_e);
            let mut out = CVector::new(0.0, 0.0, 0.0);
            let m = add_moving_collision_speed_v(ent, lp(&mut out).cast(), lp(&mut rr).cast());
            for i in 0..2 {
                f.fld(vx[i]);
                f.fadd_m(rf(m, 4 * i as isize));
                vx[i] = f.fstp();
            }
            f.fld(vx[2]);
            f.fadd_m(rf(m, 8));
        } else {
            f.fld(vx[2]);
        }
        let mut g = Fpu::new();
        dot_zyx(&mut g, rv(this, MOVE_SPEED), n);
        let dot_t = g.fstp(); // +0x6C
        f.fmul_m(n[2]);
        f.fld(vx[1]);
        f.fmul_m(n[1]);
        f.faddp(1);
        f.fld(vx[0]);
        f.fmul_m(n[0]);
        f.faddp(1);
        let dot_e = f.fstp(); // +0x30
        let mut g = Fpu::new();
        g.fld(fac_t);
        g.fmul_m(rf(this, MASS));
        let m_t = g.fstp(); // +0x28
        let d = sub3(r_e, com_e);
        let m_e = pair_mass(ent, d, n_ptr, fac_e, true); // +0x2C
        let ms = if ent_no_force { dot_e } else { mix(m_e, dot_e, m_t, dot_t) };
        let mut diff = 0.0;
        if diff_not_negative(dot_t, ms, &mut diff) {
            return 0;
        }
        let st = elastic(this, ent, ms, diff, dot_t, dot_e, m_t, m_e, d1, d2);
        let (mut tf, ef) = forces(st, n, d1, fac_t, fac_e);
        if pflags(this) & 4 == 0 {
            let mut g = Fpu::new();
            g.fld(tf[2]);
            g.fcomp_m(cf(k::K_858B50));
            if !g.jp(5) {
                tf[2] = 0.0;
            }
            if this_on_ent {
                for c in tf.iter_mut().take(2) {
                    let mut g = Fpu::new();
                    g.fld(*c);
                    g.fadd_st(0);
                    *c = g.fstp();
                }
            }
            apply_move_force_call(this, cv(tf));
        }
        if pflags(ent) & 4 == 0 && !this_on_ent {
            if eflags(ent) & 0x20 != 0 {
                unset_is_in_safe_position_call(ent);
            }
            apply_force_call(ent, cv(ef), cv(r_e), 1);
        }
        report_pair(this, ent, cp, d1, d2, m_t, m_e);
        return 1;
    }

    if pf_e & 0x10 != 0 {
        // ---- entity bDisableTurnForce only ------------------------------------------------------
        let r = point_rel(cp, this); // +0x48
        let mut v = speed(this, r); // +0x5C
        add_moving_speed(this, &mut v, r, rb(cp, 0x20));
        let dot_t = dot_yzx(v, n); // +0x6C
        let mut g = Fpu::new();
        dot_zyx(&mut g, rv(ent, MOVE_SPEED), n);
        let dot_e = g.fstp(); // +0x30
        let d = sub3(r, com_t);
        let m_t = pair_mass(this, d, n_ptr, fac_t, true); // +0x28
        let mut g = Fpu::new();
        g.fld(fac_e);
        g.fmul_m(rf(ent, MASS));
        let m_e = g.fst(); // +0x2C
        g.fmul_m(dot_e);
        g.fld(m_t);
        g.fmul_m(dot_t);
        g.faddp(1);
        g.fld(m_e);
        g.fadd_m(m_t);
        g.fdivp(1);
        let ms = g.fstp();
        let mut diff = 0.0;
        if diff_not_negative(dot_t, ms, &mut diff) {
            return 0;
        }
        let st = elastic(this, ent, ms, diff, dot_t, dot_e, m_t, m_e, d1, d2);
        let (mut tf, ef) = forces(st, n, d1, fac_t, fac_e);
        if pflags(this) & 4 == 0 && !ped_on_this {
            let mut g = Fpu::new();
            g.fld(tf[2]);
            g.fcomp_m(cf(k::K_858B50));
            if !g.jp(5) {
                tf[2] = 0.0;
            }
            apply_force_call(this, cv(tf), cv(r), 1);
        }
        if pflags(ent) & 4 == 0 {
            let mut em = ef;
            let mut g = Fpu::new();
            g.fld(ef[2]);
            g.fcomp_m(cf(k::K_858B50));
            if !g.jp(5) {
                em[2] = 0.0;
                let mut h = Fpu::new();
                h.fld(dot_t);
                h.fabs();
                h.push(X87::from_f64_exact(f64::from_bits(D_85A308)));
                h.fxch(1);
                h.fcompp();
                if !h.jp(5) {
                    em[2] = 0.0;
                    for c in em.iter_mut().take(2) {
                        let mut k2 = Fpu::new();
                        k2.fld(*c);
                        k2.fmul_m(cf(k::K_858B8C));
                        *c = k2.fstp();
                    }
                }
            }
            if ped_on_this {
                for c in em.iter_mut().take(2) {
                    let mut k2 = Fpu::new();
                    k2.fld(*c);
                    k2.fadd_st(0);
                    *c = k2.fstp();
                }
            }
            if eflags(ent) & 0x20 != 0 {
                unset_is_in_safe_position_call(ent);
            }
            apply_move_force_call(ent, cv(em));
        }
        report_pair(this, ent, cp, d1, d2, m_t, m_e);
        return 1;
    }

    // ---- neither ------------------------------------------------------------------------------
    let mut r = point_rel(cp, this); // +0x48
    let mut r_e = point_rel(cp, ent); // +0x70
    let mut v = speed(this, r); // +0x5C
    add_moving_speed(this, &mut v, r, rb(cp, 0x20));
    let mut ve = speed(ent, r_e); // +0x34
    add_moving_speed(ent, &mut ve, r_e, rb(cp, 0x23));
    let dot_t = dot_yzx(v, n); // +0x6C
    let mut g = Fpu::new();
    dot_zyx(&mut g, ve, n);
    let dot_e = g.fstp(); // +0x30
    let m_t = pair_mass(this, sub3(r, com_t), n_ptr, fac_t, true); // +0x28
    let m_e = pair_mass(ent, sub3(r_e, com_e), n_ptr, fac_e, true); // +0x2C
    let ms = mix(m_e, dot_e, m_t, dot_t);
    let mut diff = 0.0;
    if diff_not_negative(dot_t, ms, &mut diff) {
        return 0;
    }
    let st = elastic(this, ent, ms, diff, dot_t, dot_e, m_t, m_e, d1, d2);
    let (mut tf, mut ef) = forces(st, n, d1, fac_t, fac_e);
    let friction = |e: Ent, force: &mut [f32; 3], r: &mut [f32; 3], nz_neg: bool| {
        if etype(e) != ty::VEHICLE || hit_wall(e) || pflags(e) & 4 != 0 {
            return;
        }
        let mut g = Fpu::new();
        g.fld(rf(cp, 0x18));
        if nz_neg {
            g.fmul_m(cf(k::K_858C1C));
        }
        g.fcomp_m(cf(k::K_858CB0));
        if !g.jp(5) {
            g.fld(force[2]);
            g.fmul_m(cf(k::K_858C24));
            force[2] = g.fstp();
        }
        if rb(e, TYPE) & 0xF8 == 0 {
            for c in r.iter_mut() {
                g.fld(*c);
                g.fmul_m(cf(k::K_858C98));
                *c = g.fstp();
            }
        }
        if super::collide::NO_MORE_COLLISION_TORQUE.get() != 0 {
            let mut ff = [0f32; 3];
            for (i, c) in ff.iter_mut().enumerate() {
                g.fld(force[i]);
                g.fmul_m(cf(k::K_858EE4));
                *c = g.fstp();
            }
            apply_friction_force_call(e, cv(ff), cv(*r));
        }
    };
    friction(this, &mut tf, &mut r, false);
    friction(ent, &mut ef, &mut r_e, true);
    if GPtr::<u8>::from_addr(CHEAT_CARS_FLOAT_AWAY).read() != 0 {
        if find_player_vehicle(-1, 0) == this && etype(ent) == ty::VEHICLE && rb(ent, 0x4A4) != 2 {
            wu(ent, PFLAGS, pflags(ent) & 0xFFFF_FFFD);
        }
        if find_player_vehicle(-1, 0) == ent && etype(this) == ty::VEHICLE && rb(this, 0x4A4) != 2 {
            wu(this, PFLAGS, pflags(this) & 0xFFFF_FFFD);
        }
    }
    if pflags(this) & 4 == 0 {
        apply_force_call(this, cv(tf), cv(r), 1);
    }
    if pflags(ent) & 4 == 0 {
        if eflags(ent) & 0x20 != 0 {
            unset_is_in_safe_position_call(ent);
        }
        apply_force_call(ent, cv(ef), cv(r_e), 1);
    }
    report_pair(this, ent, cp, d1, d2, m_t, m_e);
    1
}

/// The two reports of a true exit: `(this, entity, A, B, *d1/mT)` and `(entity, this, B, A, *d2/mE)`.
fn report_pair(this: Ent, ent: Ent, cp: Ent, d1: GPtr<f32>, d2: GPtr<f32>, m_t: f32, m_e: f32) {
    let mut g = Fpu::new();
    g.fld(d1.read());
    g.fdiv_m(m_t);
    report(this, ent, rb(cp, 0x20), rb(cp, 0x23), cp, g.fstp());
    g.fld(d2.read());
    g.fdiv_m(m_e);
    report(ent, this, rb(cp, 0x23), rb(cp, 0x20), cp, g.fstp());
}

/// The static-entity part of [`apply_collision_2`] (0x548911..0x548A93). `Some(al)` where the
/// original returns (false, or `ApplyCollision`-1's result), `None` to go on with the pair response.
fn static_entity(this: Ent, ent: Ent, cp: Ent, d1: GPtr<f32>, d2: GPtr<f32>, com_t: [f32; 3]) -> Option<u8> {
    let n_ptr = cp.byte_add(0x10);
    let n = rv(n_ptr, 0);
    let info = || rp(ent, 0x160);
    if pflags(this) & 0x10 != 0 {
        let mut f = Fpu::new();
        dot_zyx(&mut f, rv(this, MOVE_SPEED), n);
        let s = f.fst(); // +0x24
        f.fcomp_m(cf(k::K_858B50));
        if !f.jp(5) {
            if etype(ent) != ty::OBJECT {
                if pflags(ent) & 4 != 0 {
                    return Some(apply_collision_call(this, ent, cp, d1));
                }
                set_is_static_v(ent, 0);
            } else {
                let mut f = Fpu::new();
                f.fld(s);
                f.fmul_m(rf(this, MASS));
                f.fchs();
                d1.write(f.fst());
                d2.write(f.fstp());
                let limit_hit = pflags(ent) & 4 == 0 && {
                    let lim = rf(info(), 0x14);
                    let mut g = Fpu::new();
                    g.fld(lim);
                    g.fcomp_m(cf(k::K_863C20));
                    !g.jp(5) && {
                        g.fld(d1.read());
                        g.fcomp_m(lim);
                        !g.jne(0x41)
                    }
                };
                if !limit_hit {
                    return Some(apply_collision_call(this, ent, cp, d1));
                }
                if is_object_glass(ent) != 0 {
                    window_collision(this, ent, d1.read(), cp);
                } else if pflags(ent) & 4 == 0 {
                    set_is_static_v(ent, 0);
                    property_damage(0x548A55);
                }
            }
        }
    } else {
        let r = point_rel(cp, this); // +0x48
        let mut v = speed(this, r); // +0x5C
        add_moving_speed(this, &mut v, r, rb(cp, 0x20));
        let mut f = Fpu::new();
        f.fld(v[1]);
        f.fmul_m(n[1]);
        f.fld(v[2]);
        f.fmul_m(n[2]);
        f.faddp(1);
        f.fld(v[0]);
        f.fmul_m(n[0]);
        f.faddp(1);
        let s = f.fst(); // +0x24
        f.fcomp_m(cf(k::K_858B50));
        if !f.jp(5) {
            if etype(ent) != ty::OBJECT {
                if pflags(ent) & 4 != 0 {
                    return Some(apply_collision_call(this, ent, cp, d1));
                }
                set_is_static_v(ent, 0);
                property_damage(0x548C84);
            } else if let Some(v) = object_hit(this, ent, cp, d1, d2, r, s, com_t) {
                return Some(v);
            }
        }
    }
    if eflags(ent) & 4 != 0 || eflags(ent) & 0x4_0000 != 0 {
        return Some(0);
    }
    if pflags(ent) & 4 == 0 {
        add_to_moving_list(ent);
    }
    None
}

/// A moving physical (turn force enabled) hitting a static object (0x548CB9..0x54908C); `None`
/// continues at 0x548A75.
#[allow(clippy::too_many_arguments)]
fn object_hit(this: Ent, ent: Ent, cp: Ent, d1: GPtr<f32>, d2: GPtr<f32>, r: [f32; 3], s: f32, com_t: [f32; 3])
              -> Option<u8> {
    let n_ptr = cp.byte_add(0x10);
    let info = rp(ent, 0x160);
    let d = sub3(r, com_t);
    let mut dv = cv(d);
    let mut out = CVector::new(0.0, 0.0, 0.0);
    let c = cross_product(lp(&mut out), lp(&mut dv), n_ptr.cast());
    let mut f = Fpu::new();
    f.push(X87::from_f64_exact(vector_sq_mag(c.cast())));
    f.fdiv_m(rf(this, TURN_MASS));
    f.fld(cf(k::K_858624));
    f.fdiv_m(rf(this, MASS));
    f.faddp(1);
    f.fdivr_m(cf(k::K_858624));
    let m = f.fstp(); // +0x54
    if hit_wall(this) {
        f.fld(m);
        f.fmul_m(s);
        f.fmul_m(cf(k::K_858C1C));
        d1.write(f.fstp());
    } else {
        f.fld(rf(this, ELASTICITY));
        f.fadd_m(cf(k::K_858624));
        f.fmul_m(m);
        f.fmul_m(s);
        f.fchs();
        d1.write(f.fstp());
    }
    d2.write(d1.read());
    f.fld(cf(k::K_858624));
    if etype(this) == ty::VEHICLE && ru(this, 0x594) == 9 {
        f.fstp_st(0);
        f.fld(cf(k::K_858B3C));
    }
    let mut skip_damage = true;
    if rb(ent, 0x144) != 0 {
        f.fmul_m(d1.read());
        let od = f.fst(); // +0x24
        f.fcomp_m(cf(k::K_858BA4));
        if !f.jne(0x41) {
            object_damage(ent, od, cp, n_ptr, this, 0x37);
            if eflags(ent) & 1 == 0 {
                if pflags(this) & 4 == 0 {
                    let mut g = Fpu::new();
                    g.fld(rf(info, 0x18));
                    g.fadd_st(0);
                    let c2 = g.fstp();
                    let mut t1 = CVector::new(0.0, 0.0, 0.0);
                    let p1 = vector_mul(lp(&mut t1).cast(), n_ptr, d1.read());
                    let mut t2 = CVector::new(0.0, 0.0, 0.0);
                    let p2 = vector_div(lp(&mut t2).cast(), p1, c2);
                    apply_force_call(this, cv(rv(p2, 0)), cv(r), 1);
                }
                let mut g = Fpu::new();
                g.fld(rf(info, 0x18));
                g.fdiv_m(m);
                g.fmul_m(d1.read());
                g.fadd_st(0);
                report(this, ent, rb(cp, 0x20), rb(cp, 0x23), cp, g.fstp());
                return Some(0);
            }
        }
        skip_damage = false;
    }
    if skip_damage {
        f.fstp_st(0);
    }
    // 0x548E9F
    let lim = rf(info, 0x14);
    let soft = pflags(ent) & 4 != 0 || {
        let mut g = Fpu::new();
        g.fld(lim);
        g.fcomp_m(cf(k::K_863C20));
        g.jp(5) || {
            g.fld(d1.read());
            g.fcomp_m(lim);
            g.jne(0x41) && !(eflags(this) & 0x10 != 0 && eflags(this) & 0x1000 != 0)
        }
    };
    if soft {
        if is_object_glass(ent) != 0 {
            window_responds_to_soft_collision(ent, d1.read());
        }
        return Some(apply_collision_call(this, ent, cp, d1));
    }
    if is_object_glass(ent) != 0 {
        window_collision(this, ent, d1.read(), cp);
    } else {
        set_is_static_v(ent, 0);
    }
    let mi = i32::from(rh(ent, MODEL_INDEX) as i16);
    let exploded = ru(ent, 0x140) & 0x40 != 0;
    let pos = || {
        let m = matrix(ent);
        if m.is_null() {
            ent.byte_add(PLACEMENT_POS)
        } else {
            m.byte_add(0x30)
        }
    };
    let mut set_exploded = false;
    if mi == i32::from(GPtr::<u16>::from_addr(MI_FIRE_HYDRANT).read()) && !exploded {
        trigger_water_hydrant(GPtr::from_addr(G_FX), pos());
        set_exploded = true;
    } else if (mi == i32::from(GPtr::<u16>::from_addr(MI_PARKINGMETER).read())
        || mi == i32::from(GPtr::<u16>::from_addr(MI_PARKINGMETER2).read()))
        && !exploded
    {
        let p = pos();
        let amount = sa_core::orig::at_site(0x548FB8, sa_core::crt::rand) % 100;
        let v = rv(p, 0);
        create_some_money(v[0], v[1], v[2], amount);
        set_exploded = true;
    } else if etype(ent) == ty::OBJECT && rb(info, 0x1F) == 0 {
        set_exploded = true;
    }
    if set_exploded {
        wu(ent, 0x140, ru(ent, 0x140) | 0x40);
    }
    if pflags(this) & 4 == 0 {
        let mut g = Fpu::new();
        g.fld(rf(info, 0x14));
        g.fcomp_m(cf(k::K_858A48));
        if !g.jne(0x41) {
            let mut t1 = CVector::new(0.0, 0.0, 0.0);
            let p1 = scalar_mul_vector(lp(&mut t1).cast(), G_8CD7F8.get(), n_ptr);
            let mut t2 = CVector::new(0.0, 0.0, 0.0);
            let p2 = vector_mul(lp(&mut t2).cast(), p1, d1.read());
            apply_force_call(this, cv(rv(p2, 0)), cv(r), 1);
        }
    }
    None
}

orig! {
    /// `CPhysical::ApplySoftCollision(CEntity*, CColPoint&, float&)` (0x543890, AL)
    pub fn apply_soft_collision_1(e: Ent, entity: Ent, cp: Ent, damage: GPtr<f32>) -> u8
        = thiscall 0x543890 => crate::physical::collide::apply_soft_collision;
    /// virtual `CBaseModelInfo::AsAtomicModelInfoPtr()` (vtable +4, EAX)
    pub fn model_info_as_atomic_v(mi: Ent) -> Ent = thiscall vslot 0x4 => unported;
    /// `CPhysical::ApplySpeed()` (0x547B80)
    pub fn apply_speed_call(e: Ent) = thiscall 0x547b80 => crate::physical::attach::apply_speed_port;
    /// `CVector::operator*=(float)` (0x40FEF0)
    pub fn vector_scale_assign(v: Ent, s: f32) = thiscall 0x40fef0 => unported;
}

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFO_PTRS: usize = 0xA9B0C8;


/// `CPhysical::ApplySoftCollision(CPhysical* entity, CColPoint& cp, float& thisDamage, float&
/// entityDamage)` (0x54A2C0, `ret 0x10`, AL), the soft response of two physicals (no shared
/// mass factor for the both-turn-disabled case, softer object handling).
///
/// Mass factors: `facT` (+0x58) = 10 when the entity has bDisableTurnForce and this neither 0x20 nor
/// 0x40 (`pedOnThis` when the entity's +0x568 is this, no type test), else 2/1 by this' flag 1.
/// `facE` (+0x6C) with this bDisableTurnForce and the entity neither 0x20 nor 0x40: the player /
/// vehicle formula of `ApplyCollision`-2 or 10, then `thisOnEntity` for a ped whose +0x568 is the
/// entity; else 2/1 by the entity's flag 1. `entNoForce` (+0x47): the entity has 4 without 8 (no
/// attachment test); it clears `thisOnEntity`. Centres of mass as in `ApplyCollision`-2.
///
/// A static entity without `entNoForce`: this bDisableTurnForce returns `ApplySoftCollision`-1. Else
/// `r`, `v = GetSpeed(r)` (no moving speed), `s = v . n` (yzx); `s < 0`: a non-object without 4 gets
/// `SetIsStatic(false)`, havoc and the stat; an object: mass `m`, damage as `ApplyCollision`-2 (no
/// vehicle multiplier); `m_nColDamageEffect` and damage > 20: `ObjectDamage`, and an entity no longer
/// using collision gets (without this 4) `ApplyForce((0.2*n)*damage, r, true)` and the report
/// `damage/m*0.2`, false. Damage <= uproot limit and this not stuck (0x10), or the entity has 4:
/// soft — glass `WindowRespondsToSoftCollision`, `f = n*damage` (vehicles with `n.z < 0.7`: `f.z *=
/// 0.3`), `ApplyForce(f, r, !(vehicle && bNoMoreCollisionTorque))` without this 4, report `d/m`,
/// true. Otherwise glass `WindowRespondsToCollision` / `SetIsStatic(false)` (without 4), hydrant,
/// parking meters as in `ApplyCollision`-2, and an object whose model info's virtual
/// `AsAtomicModelInfoPtr` (vtable +4) is null is marked (+0x140 bit 0x40, no limit-200 force). A
/// still-static entity returns false, else `AddToMovingList` without 4.
///
/// Both bDisableTurnForce: `dotT` on the stack, `ms` = `(mE*dotE + (mT*dotT)*2)/(mT*2 + mE)` for a
/// ped in state 11 (+0x530; the entity response follows) else `min(dotE, 0)`; `dotT - ms >= 0`
/// false; `*d1`, `ApplyMoveForce(n*d1)` without this 4, the report always; the entity (state 11
/// only): `dotE - ms >= 0` returns true, `*d2 = -((entV - dotE)*mE)`, `ApplyMoveForce(n*-d2)` without
/// its 4 (`operator*` 0x40FEC0), report. True. The other three branches follow `ApplyCollision`-2
/// with the full collision mass (no 0x20 test) and: this-only: no `UnsetIsInSafePosition`; entity
/// only: no hit-wall test, this' `ApplyForce` unless 4 / `pedOnThis`, the entity's `ApplyMoveForce`
/// without `UnsetIsInSafePosition`; neither: no 4 test before the vehicle adjustments, no cheat, and
/// an entity in a safe position runs `ApplySpeed` with its speeds negated around it (then clears
/// 0x20) before its `ApplyForce`.
#[hook(addr = 0x54a2c0, conv = thiscall, name = "CPhysical::ApplySoftCollision-2", expect = "90 E9 89 F4 EB FF",
       this = "CPhysical", unverified = "mvph-phys-1", route = "L2", hazards = "entry-protection-read",
       recs = "grove-walk,driving",
       note = "entry `nop; jmp` into a no-CD fragment (0x40974F); rand at return 0x54A5C7 and 0x54A95E")]
pub fn apply_soft_collision_2(this: Ent, ent: Ent, cp: Ent, d1: GPtr<f32>, d2: GPtr<f32>) -> u8 {
    let pf_e = pflags(ent);
    let pf_t = pflags(this);
    let mut this_on_ent = false; // +0x2F
    let mut ped_on_this = false; // +0x23
    let mut ent_no_force = false; // +0x47
    let fac_t: f32; // +0x58
    let fac_e: f32; // +0x6C
    if pf_e & 0x10 != 0 && pf_t & 0x60 == 0 {
        fac_t = 10.0;
        if rp(ent, 0x568) == this {
            ped_on_this = true;
        }
    } else {
        fac_t = if pf_t & 1 != 0 { 2.0 } else { 1.0 };
    }
    if pf_t & 0x10 != 0 && pf_e & 0x60 == 0 {
        let player_vs_car = etype(this) == ty::PED && ped_is_player(this) != 0 && etype(ent) == ty::VEHICLE && {
            let st = rb(ent, TYPE) >> 3;
            st == 4 || st == 5 || eflags(this) & 0x1000 != 0
        };
        if player_vs_car {
            let mut f = Fpu::new();
            f.fld(rf(ent, MASS));
            f.fsub_m(cf(k::K_8631F0));
            f.fld(cf(k::K_858B50));
            f.fcomp_st(1);
            if !f.jne(0x41) {
                f.fstp_st(0);
                f.fld(cf(k::K_858B50));
            }
            f.fmul_m(cf(k::K_85A7D0));
            f.fadd_m(cf(k::K_858624));
            f.fdivr_m(cf(k::K_858624));
            fac_e = f.fstp();
        } else {
            fac_e = 10.0;
        }
        if etype(this) == ty::PED && rp(this, 0x568) == ent {
            this_on_ent = true;
        }
    } else {
        fac_e = if pf_e & 1 != 0 { 2.0 } else { 1.0 };
    }
    if pf_e & 4 != 0 && pf_e & 8 == 0 {
        ent_no_force = true;
        this_on_ent = false;
    }
    let mut co_t = CVector::new(0.0, 0.0, 0.0);
    let mut com_t = rv(multiply3x3(lp(&mut co_t), matrix(this), this.byte_add(CENTRE_OF_MASS).cast()).cast(), 0);
    let mut co_e = CVector::new(0.0, 0.0, 0.0);
    let mut com_e = rv(multiply3x3(lp(&mut co_e), matrix(ent), ent.byte_add(CENTRE_OF_MASS).cast()).cast(), 0);
    if pflags(this) & 0x40 != 0 {
        com_t = [0.0; 3];
    }
    if pflags(ent) & 0x40 != 0 {
        com_e = [0.0; 3];
    }
    let n_ptr = cp.byte_add(0x10);

    if (eflags(ent) & 4 != 0 || eflags(ent) & 0x4_0000 != 0) && !ent_no_force {
        if pflags(this) & 0x10 != 0 {
            return apply_soft_collision_1(this, ent, cp, d1);
        }
        if let Some(v) = soft_static(this, ent, cp, d1, d2, com_t) {
            return v;
        }
    }
    let n = rv(n_ptr, 0);
    let pf_t = pflags(this);
    let pf_e = pflags(ent);

    if pf_t & 0x10 != 0 && pf_e & 0x10 != 0 {
        let mut f = Fpu::new();
        dot_zyx(&mut f, rv(this, MOVE_SPEED), n);
        let m_t = rf(this, MASS); // +0x24
        let m_e = rf(ent, MASS); // +0x28
        let mut fe = Fpu::new();
        dot_zyx(&mut fe, rv(ent, MOVE_SPEED), n);
        let dot_e = fe.fstp(); // +0x34
        let state11 = ru(this, 0x530) == 0xB; // +0x23
        let ms: f32; // +0x1C
        if state11 {
            f.fld(m_e);
            f.fmul_m(dot_e);
            f.fld(m_t);
            f.fmul_st(2);
            f.fadd_st(0);
            f.faddp(1);
            f.fld(m_t);
            f.fadd_st(0);
            f.fadd_m(m_e);
            f.fdivp(1);
            ms = f.fstp();
        } else {
            let mut g = Fpu::new();
            g.fld(dot_e);
            g.fcomp_m(cf(k::K_858B50));
            ms = if g.jp(5) { 0.0 } else { dot_e };
        }
        f.fld(ms);
        f.fsubr_st(1);
        let diff = f.fst(); // +0x48
        f.fcomp_m(cf(k::K_858B50));
        if f.jp(5) {
            return 0;
        }
        let mut g = Fpu::new();
        g.fld(rf(ent, ELASTICITY));
        g.fadd_m(rf(this, ELASTICITY));
        g.fmul_m(cf(k::K_858B8C));
        let el = g.fstp(); // +0x30
        if hit_wall(this) {
            f.fld(ms);
        } else {
            f.fld(diff);
            f.fmul_m(el);
            f.fsubr_m(ms);
        }
        f.fsubrp(1);
        f.fmul_m(m_t);
        f.fld_st(0);
        d1.write(f.fstp());
        let mut fv = [0f32; 3];
        f.fld_st(0);
        f.fmul_m(n[0]);
        fv[0] = f.fstp();
        f.fld_st(0);
        f.fmul_m(n[1]);
        fv[1] = f.fstp();
        f.fmul_m(n[2]);
        fv[2] = f.fstp();
        if pflags(this) & 4 == 0 {
            apply_move_force_call(this, cv(fv));
        }
        let mut g = Fpu::new();
        g.fld(d1.read());
        g.fdiv_m(m_t);
        report(this, ent, rb(cp, 0x20), rb(cp, 0x23), cp, g.fstp());
        if !state11 {
            return 1;
        }
        let mut f = Fpu::new();
        f.fld(dot_e);
        f.fsub_m(ms);
        let ediff = f.fst(); // +0x58
        f.fcomp_m(cf(k::K_858B50));
        if f.jp(5) {
            return 1;
        }
        if hit_wall(ent) {
            f.fld(ms);
        } else {
            f.fld(ediff);
            f.fmul_m(el);
            f.fsubr_m(ms);
        }
        f.fsub_m(dot_e);
        f.fmul_m(m_e);
        f.fchs();
        f.fld_st(0);
        d2.write(f.fstp());
        f.fchs();
        let s = f.fstp();
        let mut t = CVector::new(0.0, 0.0, 0.0);
        let p = vector_mul(lp(&mut t).cast(), n_ptr, s);
        if pflags(ent) & 4 == 0 {
            apply_move_force_call(ent, cv(rv(p, 0)));
        }
        let mut g = Fpu::new();
        g.fld(d2.read());
        g.fdiv_m(m_e);
        report(ent, this, rb(cp, 0x23), rb(cp, 0x20), cp, g.fstp());
        return 1;
    }

    let (m_t, m_e);
    if pf_t & 0x10 != 0 {
        // ---- this bDisableTurnForce only --------------------------------------------------------
        let r_e = point_rel(cp, ent); // +0x70
        let v = speed(ent, r_e); // +0x4C
        let mut f = Fpu::new();
        let mut vx = v;
        if pflags(ent) & 0x400_0000 != 0 && etype(ent) == ty::VEHICLE && rb(cp, 0x23) == 0x41 {
            let mut rr = cv(r_e);
            let mut out = CVector::new(0.0, 0.0, 0.0);
            let m = add_moving_collision_speed_v(ent, lp(&mut out).cast(), lp(&mut rr).cast());
            for i in 0..2 {
                f.fld(vx[i]);
                f.fadd_m(rf(m, 4 * i as isize));
                vx[i] = f.fstp();
            }
            f.fld(vx[2]);
            f.fadd_m(rf(m, 8));
        } else {
            f.fld(vx[2]);
        }
        let mut g = Fpu::new();
        dot_zyx(&mut g, rv(this, MOVE_SPEED), n);
        let dot_t = g.fstp(); // +0x68
        f.fmul_m(n[2]);
        f.fld(vx[1]);
        f.fmul_m(n[1]);
        f.faddp(1);
        f.fld(vx[0]);
        f.fmul_m(n[0]);
        f.faddp(1);
        let dot_e = f.fstp(); // +0x34
        let mut g = Fpu::new();
        g.fld(fac_t);
        g.fmul_m(rf(this, MASS));
        m_t = g.fstp(); // +0x24
        m_e = pair_mass(ent, sub3(r_e, com_e), n_ptr, fac_e, false); // +0x28
        let ms = if ent_no_force { dot_e } else { mix(m_e, dot_e, m_t, dot_t) };
        let mut diff = 0.0;
        if diff_not_negative(dot_t, ms, &mut diff) {
            return 0;
        }
        let st = elastic(this, ent, ms, diff, dot_t, dot_e, m_t, m_e, d1, d2);
        let (mut tf, ef) = forces(st, n, d1, fac_t, fac_e);
        if pflags(this) & 4 == 0 {
            let mut g = Fpu::new();
            g.fld(tf[2]);
            g.fcomp_m(cf(k::K_858B50));
            if !g.jp(5) {
                tf[2] = 0.0;
            }
            if this_on_ent {
                for c in tf.iter_mut().take(2) {
                    let mut g = Fpu::new();
                    g.fld(*c);
                    g.fadd_st(0);
                    *c = g.fstp();
                }
            }
            apply_move_force_call(this, cv(tf));
        }
        if pflags(ent) & 4 == 0 && !this_on_ent {
            apply_force_call(ent, cv(ef), cv(r_e), 1);
        }
    } else if pf_e & 0x10 != 0 {
        // ---- entity bDisableTurnForce only ------------------------------------------------------
        let r = point_rel(cp, this); // +0x5C
        let mut v = speed(this, r); // +0x38
        add_moving_speed(this, &mut v, r, rb(cp, 0x20));
        let dot_t = dot_yzx(v, n); // +0x68
        let mut g = Fpu::new();
        dot_zyx(&mut g, rv(ent, MOVE_SPEED), n);
        let dot_e = g.fstp(); // +0x34
        m_t = pair_mass(this, sub3(r, com_t), n_ptr, fac_t, false); // +0x24
        let mut g = Fpu::new();
        g.fld(fac_e);
        g.fmul_m(rf(ent, MASS));
        m_e = g.fst(); // +0x28
        g.fmul_m(dot_e);
        g.fld(m_t);
        g.fmul_m(dot_t);
        g.faddp(1);
        g.fld(m_e);
        g.fadd_m(m_t);
        g.fdivp(1);
        let ms = g.fstp();
        let mut diff = 0.0;
        if diff_not_negative(dot_t, ms, &mut diff) {
            return 0;
        }
        let st = elastic(this, ent, ms, diff, dot_t, dot_e, m_t, m_e, d1, d2);
        let (mut tf, ef) = forces(st, n, d1, fac_t, fac_e);
        if pflags(this) & 4 == 0 && !ped_on_this {
            let mut g = Fpu::new();
            g.fld(tf[2]);
            g.fcomp_m(cf(k::K_858B50));
            if !g.jp(5) {
                tf[2] = 0.0;
            }
            apply_force_call(this, cv(tf), cv(r), 1);
        }
        if pflags(ent) & 4 == 0 {
            let mut em = ef;
            let mut g = Fpu::new();
            g.fld(ef[2]);
            g.fcomp_m(cf(k::K_858B50));
            if !g.jp(5) {
                em[2] = 0.0;
                let mut h = Fpu::new();
                h.fld(dot_t);
                h.fabs();
                h.push(X87::from_f64_exact(f64::from_bits(D_85A308)));
                h.fxch(1);
                h.fcompp();
                if !h.jp(5) {
                    em[2] = 0.0;
                    for c in em.iter_mut().take(2) {
                        let mut k2 = Fpu::new();
                        k2.fld(*c);
                        k2.fmul_m(cf(k::K_858B8C));
                        *c = k2.fstp();
                    }
                }
            }
            if ped_on_this {
                for c in em.iter_mut().take(2) {
                    let mut k2 = Fpu::new();
                    k2.fld(*c);
                    k2.fadd_st(0);
                    *c = k2.fstp();
                }
            }
            apply_move_force_call(ent, cv(em));
        }
    } else {
        // ---- neither ----------------------------------------------------------------------------
        let mut r = point_rel(cp, this); // +0x5C
        let mut r_e = point_rel(cp, ent); // +0x70
        let mut v = speed(this, r); // +0x38
        add_moving_speed(this, &mut v, r, rb(cp, 0x20));
        let mut ve = speed(ent, r_e); // +0x4C
        add_moving_speed(ent, &mut ve, r_e, rb(cp, 0x23));
        let dot_t = dot_yzx(v, n); // +0x68
        let mut g = Fpu::new();
        dot_zyx(&mut g, ve, n);
        let dot_e = g.fstp(); // +0x34
        m_t = pair_mass(this, sub3(r, com_t), n_ptr, fac_t, false); // +0x24
        m_e = pair_mass(ent, sub3(r_e, com_e), n_ptr, fac_e, false); // +0x28
        let ms = mix(m_e, dot_e, m_t, dot_t);
        let mut diff = 0.0;
        if diff_not_negative(dot_t, ms, &mut diff) {
            return 0;
        }
        let st = elastic(this, ent, ms, diff, dot_t, dot_e, m_t, m_e, d1, d2);
        let (mut tf, mut ef) = forces(st, n, d1, fac_t, fac_e);
        let adjust = |e: Ent, force: &mut [f32; 3], r: &mut [f32; 3], nz_neg: bool| {
            if etype(e) != ty::VEHICLE || hit_wall(e) {
                return;
            }
            let mut g = Fpu::new();
            g.fld(rf(cp, 0x18));
            if nz_neg {
                g.fmul_m(cf(k::K_858C1C));
            }
            g.fcomp_m(cf(k::K_858CB0));
            if !g.jp(5) {
                g.fld(force[2]);
                g.fmul_m(cf(k::K_858C24));
                force[2] = g.fstp();
            }
            if rb(e, TYPE) & 0xF8 == 0 {
                for c in r.iter_mut() {
                    g.fld(*c);
                    g.fmul_m(cf(k::K_858C98));
                    *c = g.fstp();
                }
            }
            if super::collide::NO_MORE_COLLISION_TORQUE.get() != 0 {
                let mut ff = [0f32; 3];
                for (i, c) in ff.iter_mut().enumerate() {
                    g.fld(force[i]);
                    g.fmul_m(cf(k::K_858EE4));
                    *c = g.fstp();
                }
                apply_friction_force_call(e, cv(ff), cv(*r));
            }
        };
        adjust(this, &mut tf, &mut r, false);
        adjust(ent, &mut ef, &mut r_e, true);
        if pflags(this) & 4 == 0 {
            apply_force_call(this, cv(tf), cv(r), 1);
        }
        if pflags(ent) & 4 == 0 {
            if eflags(ent) & 0x20 != 0 {
                vector_scale_assign(ent.byte_add(MOVE_SPEED), cf(k::K_858C1C));
                vector_scale_assign(ent.byte_add(TURN_SPEED), cf(k::K_858C1C));
                apply_speed_call(ent);
                vector_scale_assign(ent.byte_add(MOVE_SPEED), cf(k::K_858C1C));
                vector_scale_assign(ent.byte_add(TURN_SPEED), cf(k::K_858C1C));
                wu(ent, EFLAGS, eflags(ent) & 0xFFFF_FFDF);
            }
            apply_force_call(ent, cv(ef), cv(r_e), 1);
        }
    }
    report_pair(this, ent, cp, d1, d2, m_t, m_e);
    1
}

/// The static-entity part of [`apply_soft_collision_2`] with turn force (0x54A4ED..0x54A88A): `Some`
/// where the original returns, `None` to go on.
fn soft_static(this: Ent, ent: Ent, cp: Ent, d1: GPtr<f32>, d2: GPtr<f32>, com_t: [f32; 3]) -> Option<u8> {
    let n_ptr = cp.byte_add(0x10);
    let r = point_rel(cp, this); // +0x5C
    let v = speed(this, r); // +0x38
    let s = dot_yzx(v, rv(n_ptr, 0)); // +0x48
    let mut f = Fpu::new();
    f.fld(s);
    f.fcomp_m(cf(k::K_858B50));
    if !f.jp(5) {
        if etype(ent) != ty::OBJECT {
            if pflags(ent) & 4 == 0 {
                set_is_static_v(ent, 0);
                property_damage(0x54A5C7);
            }
        } else if let Some(v) = soft_object_hit(this, ent, cp, d1, d2, r, s, com_t) {
            return Some(v);
        }
    }
    if eflags(ent) & 4 != 0 || eflags(ent) & 0x4_0000 != 0 {
        return Some(0);
    }
    if pflags(ent) & 4 == 0 {
        add_to_moving_list(ent);
    }
    None
}

/// A physical hitting a static object softly (0x54A5EC..0x54AAB4); `None` continues at 0x54A86C.
#[allow(clippy::too_many_arguments)]
fn soft_object_hit(this: Ent, ent: Ent, cp: Ent, d1: GPtr<f32>, d2: GPtr<f32>, r: [f32; 3], s: f32,
                   com_t: [f32; 3]) -> Option<u8> {
    let n_ptr = cp.byte_add(0x10);
    let d = sub3(r, com_t);
    let mut dv = cv(d);
    let mut out = CVector::new(0.0, 0.0, 0.0);
    let c = cross_product(lp(&mut out), lp(&mut dv), n_ptr.cast());
    let mut f = Fpu::new();
    f.push(X87::from_f64_exact(vector_sq_mag(c.cast())));
    f.fdiv_m(rf(this, TURN_MASS));
    f.fld(cf(k::K_858624));
    f.fdiv_m(rf(this, MASS));
    f.faddp(1);
    f.fdivr_m(cf(k::K_858624));
    let m = f.fstp(); // +0x30
    if hit_wall(this) {
        f.fld(m);
        f.fmul_m(s);
        f.fmul_m(cf(k::K_858C1C));
        d1.write(f.fstp());
    } else {
        f.fld(rf(this, ELASTICITY));
        f.fadd_m(cf(k::K_858624));
        f.fmul_m(m);
        f.fmul_m(s);
        f.fchs();
        d1.write(f.fstp());
    }
    d2.write(d1.read());
    if rb(ent, 0x144) != 0 {
        f.fld(d1.read());
        f.fcomp_m(cf(k::K_858BA4));
        if !f.jne(0x41) {
            object_damage(ent, d1.read(), cp, n_ptr, this, 0x37);
            if eflags(ent) & 1 == 0 {
                if pflags(this) & 4 == 0 {
                    let mut t1 = CVector::new(0.0, 0.0, 0.0);
                    let p1 = scalar_mul_vector(lp(&mut t1).cast(), f32::from_bits(0x3E4C_CCCD), n_ptr);
                    let mut t2 = CVector::new(0.0, 0.0, 0.0);
                    let p2 = vector_mul(lp(&mut t2).cast(), p1, d1.read());
                    apply_force_call(this, cv(rv(p2, 0)), cv(r), 1);
                }
                let mut g = Fpu::new();
                g.fld(d1.read());
                g.fdiv_m(m);
                g.fmul_m(cf(k::K_858CC4));
                report(this, ent, rb(cp, 0x20), rb(cp, 0x23), cp, g.fstp());
                return Some(0);
            }
        }
    }
    // 0x54A79E
    let info = rp(ent, 0x160);
    let mut g = Fpu::new();
    g.fld(d1.read());
    g.fcomp_m(rf(info, 0x14));
    let soft = (g.jne(0x41) && eflags(this) & 0x10 == 0) || pflags(ent) & 4 != 0;
    if soft {
        if is_object_glass(ent) != 0 {
            window_responds_to_soft_collision(ent, d1.read());
        }
        let mut t = CVector::new(0.0, 0.0, 0.0);
        let mut fv = rv(vector_mul(lp(&mut t).cast(), n_ptr, d1.read()), 0);
        let vehicle = etype(this) == ty::VEHICLE;
        if vehicle {
            let mut g = Fpu::new();
            g.fld(rf(cp, 0x18));
            g.fcomp_m(cf(k::K_858CB0));
            if !g.jp(5) {
                g.fld(fv[2]);
                g.fmul_m(cf(k::K_858C24));
                fv[2] = g.fstp();
            }
        }
        if pflags(this) & 4 == 0 {
            let turn = !(vehicle && super::collide::NO_MORE_COLLISION_TORQUE.get() != 0) as u8;
            apply_force_call(this, cv(fv), cv(r), turn);
        }
        let mut g = Fpu::new();
        g.fld(d1.read());
        g.fdiv_m(m);
        report(this, ent, rb(cp, 0x20), rb(cp, 0x23), cp, g.fstp());
        return Some(1);
    }
    if is_object_glass(ent) != 0 {
        window_collision(this, ent, d1.read(), cp);
    } else if pflags(ent) & 4 == 0 {
        set_is_static_v(ent, 0);
    }
    let mi = i32::from(rh(ent, MODEL_INDEX) as i16);
    let exploded = ru(ent, 0x140) & 0x40 != 0;
    let pos = || {
        let m = matrix(ent);
        if m.is_null() {
            ent.byte_add(PLACEMENT_POS)
        } else {
            m.byte_add(0x30)
        }
    };
    let set_exploded = if mi == i32::from(GPtr::<u16>::from_addr(MI_FIRE_HYDRANT).read()) && !exploded {
        trigger_water_hydrant(GPtr::from_addr(G_FX), pos());
        true
    } else if (mi == i32::from(GPtr::<u16>::from_addr(MI_PARKINGMETER).read())
        || mi == i32::from(GPtr::<u16>::from_addr(MI_PARKINGMETER2).read()))
        && !exploded
    {
        let p = pos();
        let amount = sa_core::orig::at_site(0x54A95E, sa_core::crt::rand) % 100;
        let v = rv(p, 0);
        create_some_money(v[0], v[1], v[2], amount);
        true
    } else if etype(ent) == ty::OBJECT {
        let info_ptr = GPtr::<Ent>::from_addr(MODEL_INFO_PTRS + 4 * mi as usize).read();
        model_info_as_atomic_v(info_ptr).is_null()
    } else {
        false
    };
    if set_exploded {
        wu(ent, 0x140, ru(ent, 0x140) | 0x40);
    }
    None
}

pub const HOOKS: &[HookReg] = &[HOOK_APPLY_COLLISION_2, HOOK_APPLY_SOFT_COLLISION_2];
