//! `CPhysical` collision responses (PORT-PHYS): `ApplyCollision`, `ApplySoftCollision`, the spring
//! collisions and dampening, `ApplyScriptCollision`. Ported instruction by instruction from the 1.0 US
//! bytes with the [`Fpu`] stack machine (PC = 24).
//!
//! `CColPoint` offsets: point +0, normal +0x10, surfaceA +0x20, pieceA +0x21, lightingA +0x22,
//! surfaceB +0x23, pieceB +0x24, lightingB +0x25, depth +0x28.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;

use super::forces::{apply_force_call, apply_move_force_call, get_speed_call};
use super::shared::*;
use crate::collision::fpu::Fpu;

orig! {
    /// `CAudioEngine::ReportCollision(CEntity*, CEntity*, surfA, surfB, const CVector& pos, const
    /// CVector* normal, float impact1, float impact2, bool, bool)` (0x506EB0, thiscall on
    /// `AudioEngine` 0xB6BC90; every argument a dword)
    pub fn report_collision(ae: Ent, e1: Ent, e2: Ent, surf_a: u32, surf_b: u32, pos: Ent, normal: Ent,
        impact1: f32, impact2: u32, b1: u32, b2: u32) = thiscall 0x506EB0 => unported;
}

sa_core::globals! {
    list GLOBALS;
    /// `CWorld::bNoMoreCollisionTorque`
    pub static NO_MORE_COLLISION_TORQUE @ 0xB7CD72: u8 = 0;
}

/// `AudioEngine` (0xB6BC90)
pub const AUDIO_ENGINE: usize = 0xB6BC90;

/// `AudioEngine.ReportCollision(this, entity, cp.surfaceA, cp.surfaceB, cp.point, normal, impact,
/// 1.0f, false, false)` as the ApplyCollision tails push it.
fn report(this: Ent, entity: Ent, cp: Ent, normal: Ent, impact: f32) {
    report_collision(
        GPtr::from_addr(AUDIO_ENGINE),
        this,
        entity,
        rb(cp, 0x20) as u32,
        rb(cp, 0x23) as u32,
        cp,
        normal,
        impact,
        0x3F80_0000,
        0,
        0,
    );
}

/// `CPhysical::ApplyCollision(CEntity* entity, CColPoint& cp, float& damageIntensity)` (0x5435C0,
/// `ret 0xC`, AL).
///
/// `bDisableTurnForce`: `s = (move.z*n.z + move.y*n.y) + n.x*move.x` (registers); unless `s < 0`
/// false; `damage = -(s*mass)`, `ApplyMoveForce(damage*n)`, impact `damage/mass`.
/// Otherwise `r = cp.point - pos` (f32), `v = GetSpeed(r)`, `s = (n.y*v.y + n.x*v.x) + v.z*n.z`;
/// unless `s < 0` false; `d = r - Multiply3x3(matrix, com)`, `c = CrossProduct(d, n)`,
/// `m = 1 / (((c.x² + c.y²) + c.z²)/turnMass + 1/mass)` (f32), `damage = -((elasticity + 1)*m*s)`,
/// `f = n*damage` (vehicles with `n.z < 0.7f`: `f.z *= 0.3f`), `ApplyForce(f, r, !vehicle ||
/// !bNoMoreCollisionTorque)` unless `bDisableCollisionForce`; impact `damage/m`. Both report the
/// collision to the audio engine and return true.
#[hook(addr = 0x5435C0, conv = thiscall, name = "CPhysical::ApplyCollision-1", expect = "83 EC 40 53 55", this = "CPhysical")]
pub fn apply_collision(this: Ent, entity: Ent, cp: Ent, damage: GPtr<f32>) -> u8 {
    let n = cp.byte_add(0x10);
    let mut f = Fpu::new();
    if pflags(this) & 0x10 != 0 {
        f.fld(rf(this, MOVE_SPEED + 8));
        f.fmul_m(rf(n, 8));
        f.fld(rf(this, MOVE_SPEED + 4));
        f.fmul_m(rf(n, 4));
        f.faddp(1);
        f.fld(rf(n, 0));
        f.fmul_m(rf(this, MOVE_SPEED));
        f.faddp(1);
        f.fcom_m(cf(k::K_858B50));
        if f.jp(5) {
            return 0;
        }
        f.fmul_m(rf(this, MASS));
        f.fchs();
        damage.write(f.fst());
        let mut v = [0f32; 3];
        for (i, v) in v.iter_mut().enumerate().take(2) {
            f.fld_st(0);
            f.fmul_m(rf(n, 4 * i as isize));
            *v = f.fstp();
        }
        f.fmul_m(rf(n, 8));
        v[2] = f.fstp();
        apply_move_force_call(this, cv(v));
        f.fld(damage.read());
        f.fdiv_m(rf(this, MASS));
        let impact = f.fstp();
        report(this, entity, cp, n, impact);
        return 1;
    }
    let pos = matrix(this).byte_add(0x30);
    let mut r = [0f32; 3];
    for (i, r) in r.iter_mut().enumerate() {
        f.fld(rf(cp, 4 * i as isize));
        f.fsub_m(rf(pos, 4 * i as isize));
        *r = f.fstp();
    }
    let mut vo = CVector::new(0.0, 0.0, 0.0);
    let v = rv(get_speed_call(this, lp(&mut vo), cv(r)).cast(), 0);
    let nl = rv(n, 0);
    f.fld(nl[1]);
    f.fmul_m(v[1]);
    f.fld(nl[0]);
    f.fmul_m(v[0]);
    f.faddp(1);
    f.fld(v[2]);
    f.fmul_m(nl[2]);
    f.faddp(1);
    let s = f.fst();
    f.fcomp_m(cf(k::K_858B50));
    if f.jp(5) {
        return 0;
    }
    let mut co = CVector::new(0.0, 0.0, 0.0);
    let com = rv(multiply3x3(lp(&mut co), matrix(this), this.byte_add(CENTRE_OF_MASS).cast()).cast(), 0);
    let mut d = [0f32; 3];
    for (i, d) in d.iter_mut().enumerate() {
        f.fld(r[i]);
        f.fsub_m(com[i]);
        *d = f.fstp();
    }
    let mut dv = cv(d);
    let mut nv = cv(nl);
    let mut xo = CVector::new(0.0, 0.0, 0.0);
    let c = rv(cross_product(lp(&mut xo), lp(&mut dv), lp(&mut nv)).cast(), 0);
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
    f.fdiv_m(rf(this, TURN_MASS));
    f.fld(cf(k::K_858624));
    f.fdiv_m(rf(this, MASS));
    f.faddp(1);
    f.fdivr_m(cf(k::K_858624));
    let m = f.fstp();
    f.fstp_st(0);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fld(rf(this, ELASTICITY));
    f.fadd_m(cf(k::K_858624));
    f.fmul_m(m);
    f.fmul_m(s);
    f.fchs();
    damage.write(f.fst());
    let vehicle = etype(this) == ty::VEHICLE;
    let mut fv = [0f32; 3];
    f.fld(nl[0]);
    f.fmul_st(1);
    fv[0] = f.fstp();
    f.fld(nl[1]);
    f.fmul_st(1);
    fv[1] = f.fstp();
    f.fmul_m(nl[2]);
    fv[2] = f.fstp();
    if vehicle {
        f.fld(nl[2]);
        f.fcomp_m(cf(k::K_858CB0));
        if !f.jp(5) {
            f.fld(fv[2]);
            f.fmul_m(cf(k::K_858C24));
            fv[2] = f.fstp();
        }
    }
    if pflags(this) & 4 == 0 {
        let turn = !(vehicle && NO_MORE_COLLISION_TORQUE.get() != 0) as u8;
        apply_force_call(this, cv(fv), cv(r), turn);
    }
    f.fld(damage.read());
    f.fdiv_m(m);
    let impact = f.fstp();
    report(this, entity, cp, n, impact);
    1
}

orig! {
    /// `CPhysical::ApplyCollision(CEntity*, CColPoint&, float&)` (0x5435C0)
    pub fn apply_collision_call(e: Ent, entity: Ent, cp: Ent, damage: GPtr<f32>) -> u8 = thiscall 0x5435C0 => apply_collision;
    /// `CVector::Normalise()` (0x59C910, thiscall)
    pub fn normalise(v: GPtr<CVector>) = thiscall 0x59C910 => crate::collision::data::normalise_port;
}

/// `1 / ((((c.x*c.x) + c.y*c.y) + c.z*c.z) / turnMass + 1/mass)` as the collision functions compute
/// it from `c = CrossProduct(d, n)` (registers, the result stored as f32).
fn collision_mass(this: Ent, c: [f32; 3]) -> f32 {
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
    f.fdiv_m(rf(this, TURN_MASS));
    f.fld(cf(k::K_858624));
    f.fdiv_m(rf(this, MASS));
    f.faddp(1);
    f.fdivr_m(cf(k::K_858624));
    f.fstp()
}

/// `CPhysical::ApplySoftCollision(CEntity* entity, CColPoint& cp, float& damageIntensity)`
/// (0x543890, `ret 0xC`, AL).
///
/// With `bDisableTurnForce` first `ApplyCollision(entity, cp, damage)` (result ignored, the function
/// goes on). `r = cp.point - pos`, `v = GetSpeed(r)`, `n = cp.normal` (local), speed factor
/// `SOFTCOL_SPEED_MULT`. A vehicle of type 1 (bike): `dp = n . up` (f32); `dp < -0.9f` returns false;
/// `dp < 0`: `n -= dp*up`, `Normalise(n)`; `dp > 0.5f`: factor `SOFTCOL_SPEED_MULT2`. Then `s = n . v`,
/// `d = r - com'` (com' = 0 for `bInfiniteMass`), `m` = the collision mass of `d x n`.
/// An automobile (vehicle type 0) hit on a wheel piece (13..=16): `damage = depth * timeStep *
/// DEPTH_MULT * m * 0.008f` minus `CARLINE_SPEED_MULT*m*s` when `s < 0`; `n -= (0.9f*(n . right)) *
/// right`. Others: the depth capped at `SOFTCOL_DEPTH_MIN`, `damage -= m*s*factor` when `s < 0`,
/// else damage 0 and false. Then a zero damage returns false; else `ApplyForce(n*damage, r, true)`,
/// a negative damage is negated, true.
#[hook(addr = 0x543890, conv = thiscall, name = "CPhysical::ApplySoftCollision-1", expect = "83 EC 3C 53 8B 5C 24 4C", this = "CPhysical")]
pub fn apply_soft_collision(this: Ent, entity: Ent, cp: Ent, damage: GPtr<f32>) -> u8 {
    if pflags(this) & 0x10 != 0 {
        apply_collision_call(this, entity, cp, damage);
    }
    let mut f = Fpu::new();
    let pos = matrix(this).byte_add(0x30);
    let mut r = [0f32; 3];
    for (i, r) in r.iter_mut().enumerate() {
        f.fld(rf(cp, 4 * i as isize));
        f.fsub_m(rf(pos, 4 * i as isize));
        *r = f.fstp();
    }
    let mut vo = CVector::new(0.0, 0.0, 0.0);
    let v = rv(get_speed_call(this, lp(&mut vo), cv(r)).cast(), 0);
    let mut nv = cv(rv(cp, 0x10));
    let mut factor = SOFTCOL_SPEED_MULT.get();
    if etype(this) == ty::VEHICLE && ru(this, 0x594) == 1 {
        let m = matrix(this);
        f.fld(nv.y);
        f.fmul_m(rf(m, 0x24));
        f.fld(nv.z);
        f.fmul_m(rf(m, 0x28));
        f.faddp(1);
        f.fld(nv.x);
        f.fmul_m(rf(m, 0x20));
        f.faddp(1);
        let dp = f.fst();
        f.fcomp_m(cf(k::K_858CAC));
        if !f.jp(5) {
            return 0;
        }
        f.fld(dp);
        f.fcomp_m(cf(k::K_858B50));
        f.fld(dp);
        if !f.jp(5) {
            f.fmul_m(rf(m, 0x20));
            f.fld(dp);
            f.fmul_m(rf(m, 0x24));
            f.fld(dp);
            f.fmul_m(rf(m, 0x28));
            let tz = f.fstp();
            f.fld(nv.x);
            f.fsub_st(2);
            nv.x = f.fstp();
            f.fld(nv.y);
            f.fsub_st(1);
            nv.y = f.fstp();
            f.fstp_st(0);
            f.fstp_st(0);
            f.fld(nv.z);
            f.fsub_m(tz);
            nv.z = f.fstp();
            normalise(lp(&mut nv));
        } else {
            f.fcomp_m(cf(k::K_858B8C));
            if !f.jne(0x41) {
                factor = SOFTCOL_SPEED_MULT2.get();
            }
        }
    }
    f.fld(v[1]);
    f.fmul_m(nv.y);
    f.fld(v[2]);
    f.fmul_m(nv.z);
    f.faddp(1);
    f.fld(v[0]);
    f.fmul_m(nv.x);
    f.faddp(1);
    let s = f.fstp();
    let mut co = CVector::new(0.0, 0.0, 0.0);
    let mut com = rv(multiply3x3(lp(&mut co), matrix(this), this.byte_add(CENTRE_OF_MASS).cast()).cast(), 0);
    if rb(this, PFLAGS) & 0x40 != 0 {
        com = [0.0; 3];
    }
    let mut d = [0f32; 3];
    for (i, d) in d.iter_mut().enumerate() {
        f.fld(r[i]);
        f.fsub_m(com[i]);
        *d = f.fstp();
    }
    let mut dv = cv(d);
    let mut xo = CVector::new(0.0, 0.0, 0.0);
    let c = rv(cross_product(lp(&mut xo), lp(&mut dv), lp(&mut nv)).cast(), 0);
    let m = collision_mass(this, c);
    let piece = rb(cp, 0x21);
    if etype(this) == ty::VEHICLE && ru(this, 0x594) == 0 && (0xD..=0x10).contains(&piece) {
        f.fld(rf(cp, 0x28));
        f.fmul_m(sa_core::timer::TIME_STEP.get());
        f.fmul_m(SOFTCOL_DEPTH_MULT.get());
        f.fmul_m(m);
        f.fmul_m(cf(k::K_863984));
        damage.write(f.fst());
        f.fld(s);
        f.fcomp_m(cf(k::K_858B50));
        if !f.jp(5) {
            f.fld(SOFTCOL_CARLINE_SPEED_MULT.get());
            f.fmul_m(m);
            f.fmul_m(s);
            f.fsubr_st(1);
            damage.write(f.fstp());
        }
        let mx = matrix(this);
        f.fstp_st(0);
        f.fld(nv.y);
        f.fmul_m(rf(mx, 4));
        f.fld(nv.z);
        f.fmul_m(rf(mx, 8));
        f.faddp(1);
        f.fld(nv.x);
        f.fmul_m(rf(mx, 0));
        f.faddp(1);
        f.fmul_m(cf(k::K_858C20));
        f.fld_st(0);
        f.fmul_m(rf(mx, 0));
        f.fld_st(1);
        f.fmul_m(rf(mx, 4));
        let ty_ = f.fstp();
        f.fxch(1);
        f.fmul_m(rf(mx, 8));
        let tz = f.fstp();
        f.fld(nv.x);
        f.fsub_st(1);
        nv.x = f.fstp();
        f.fstp_st(0);
        f.fld(nv.y);
        f.fsub_m(ty_);
        nv.y = f.fstp();
        f.fld(nv.z);
        f.fsub_m(tz);
        nv.z = f.fstp();
    } else {
        let min = SOFTCOL_DEPTH_MIN.get();
        f.fld(min);
        f.fcomp_m(rf(cp, 0x28));
        if !f.jp(5) {
            f.fld(min);
        } else {
            f.fld(rf(cp, 0x28));
        }
        f.fmul_m(sa_core::timer::TIME_STEP.get());
        f.fmul_m(SOFTCOL_DEPTH_MULT.get());
        f.fmul_m(m);
        f.fmul_m(cf(k::K_863984));
        damage.write(f.fst());
        f.fld(s);
        f.fcomp_m(cf(k::K_858B50));
        if f.jp(5) {
            f.fstp_st(0);
            damage.write(0.0);
            return 0;
        }
        f.fld(m);
        f.fmul_m(s);
        f.fmul_m(factor);
        f.fsubr_st(1);
        damage.write(f.fstp());
        f.fstp_st(0);
    }
    f.fld(damage.read());
    f.fcomp_m(cf(k::K_858B50));
    if !f.jp(0x44) {
        return 0;
    }
    f.fld(damage.read());
    let mut fv = [0f32; 3];
    for (i, n) in [nv.x, nv.y, nv.z].iter().enumerate() {
        f.fld(*n);
        f.fmul_st(1);
        fv[i] = f.fstp();
    }
    f.fstp_st(0);
    apply_force_call(this, cv(fv), cv(r), 1);
    f.fld(damage.read());
    f.fcomp_m(cf(k::K_858B50));
    if !f.jp(5) {
        f.fld(damage.read());
        f.fmul_m(cf(k::K_858C1C));
        damage.write(f.fstp());
    }
    1
}

pub const HOOKS: &[HookReg] = &[HOOK_APPLY_COLLISION, HOOK_APPLY_SOFT_COLLISION];
