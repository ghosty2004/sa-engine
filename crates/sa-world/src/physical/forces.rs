//! `CPhysical` forces, speeds, gravity, air resistance and the damage record (PORT-PHYS), ported
//! instruction by instruction from the 1.0 US bytes with the [`Fpu`] stack machine (PC = 24).
//!
//! The force functions share two shapes: the *move part* (`k = 1.0f / m_fMass` kept in a register,
//! `v.x*k` added directly, `v.y*k` and `v.z*k` through f32 temporaries) and the *turn part* (the
//! centre of mass rotated by `Multiply3x3` unless `bInfiniteMass`, `r = point - com` stored as f32,
//! `c = CrossProduct(r, force)`, `k = 1.0f / turnMass`, same add pattern).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::shared::*;
use crate::collision::fpu::Fpu;

orig! {
    /// `CCullZones::DoExtraAirResistanceForPlayer()` (0x72DDD0, AL)
    pub fn do_extra_air_resistance_for_player() -> u8 = cdecl 0x72DDD0 => unported;
}

sa_core::globals! {
    list GLOBALS;
    /// 0x8D34A8 (2.5; extra air resistance factor)
    pub static G_8D34A8 @ 0x8D34A8: f32 = 2.5;
}

/// The move part: `dst += v * (1.0f / m_fMass)` (k in a register; x added directly, y and z through
/// f32 temporaries).
fn move_part(this: Ent, v: [f32; 3], dst: isize) {
    let mut f = Fpu::new();
    f.fld(rf(this, MASS));
    f.fdivr_m(cf(k::K_858624));
    f.fld(v[0]);
    f.fmul_st(1);
    f.fld(v[1]);
    f.fmul_st(2);
    let y = f.fstp();
    f.fxch(1);
    f.fmul_m(v[2]);
    let z = f.fstp();
    f.fadd_m(rf(this, dst));
    wf(this, dst, f.fstp());
    f.fld(y);
    f.fadd_m(rf(this, dst + 4));
    wf(this, dst + 4, f.fstp());
    f.fld(z);
    f.fadd_m(rf(this, dst + 8));
    wf(this, dst + 8, f.fstp());
}

/// The turn part (see the module docs). `adjust_infinite`: with `bInfiniteMass` the turn mass
/// becomes `((com.z*mass)*com.z)*0.5f + turnMass` (ApplyForce, ApplyFrictionForce,
/// ApplyFrictionTurnForce; not ApplyTurnForce). `force` is the by-value argument (its z is zeroed
/// with `bDisableMoveForce`, as is the point's z).
fn turn_part(this: Ent, force: [f32; 3], point: [f32; 3], adjust_infinite: bool, dst: isize) {
    let flags = pflags(this);
    let mut turn_mass = rf(this, TURN_MASS);
    let mut com = CVector::new(0.0, 0.0, 0.0);
    if flags & 0x40 != 0 {
        if adjust_infinite {
            let mut f = Fpu::new();
            f.fld(rf(this, CENTRE_OF_MASS + 8));
            f.fld_st(0);
            f.fmul_m(rf(this, MASS));
            f.fmul_st(1);
            f.fmul_m(cf(k::K_858B8C));
            f.fadd_m(turn_mass);
            turn_mass = f.fstp();
        }
    } else {
        let mut out = CVector::new(0.0, 0.0, 0.0);
        let r = multiply3x3(lp(&mut out), matrix(this), this.byte_add(CENTRE_OF_MASS).cast());
        com = cv(rv(r.cast(), 0));
    }
    let mut force = cv(force);
    let mut f = Fpu::new();
    if pflags(this) & 0x20 != 0 {
        f.fld(cf(k::K_858B50));
        force.z = 0.0;
    } else {
        f.fld(point[2]);
    }
    f.fld(point[0]);
    f.fsub_m(com.x);
    let rx = f.fstp();
    f.fld(point[1]);
    f.fsub_m(com.y);
    let ry = f.fstp();
    f.fsub_m(com.z);
    let rz = f.fstp();
    let mut r = CVector::new(rx, ry, rz);
    let mut out = CVector::new(0.0, 0.0, 0.0);
    let c = cross_product(lp(&mut out), lp(&mut r), lp(&mut force));
    let c = rv(c.cast(), 0);
    f.fld(cf(k::K_858624));
    f.fdiv_m(turn_mass);
    f.fld_st(0);
    f.fmul_m(c[0]);
    f.fld_st(1);
    f.fmul_m(c[1]);
    let ty = f.fstp();
    f.fxch(1);
    f.fmul_m(c[2]);
    let tz = f.fstp();
    f.fadd_m(rf(this, dst));
    wf(this, dst, f.fstp());
    f.fld(ty);
    f.fadd_m(rf(this, dst + 4));
    wf(this, dst + 4, f.fstp());
    f.fld(tz);
    f.fadd_m(rf(this, dst + 8));
    wf(this, dst + 8, f.fstp());
}

/// `CPhysical::ApplyMoveForce(CVector force)` (0x5429F0, `ret 0xC`): nothing with
/// `bDisableMoveForce`/`bInfiniteMass`; z = 0 with `bDisableZ`; `m_vecMoveSpeed += force / mass`.
#[hook(addr = 0x5429F0, conv = thiscall, name = "CPhysical::ApplyMoveForce-vec", expect = "8B 41 40 A8 60", this = "CPhysical", preserve = "edx")]
pub fn apply_move_force(this: Ent, force: CVector) {
    let fl = pflags(this);
    if fl & 0x60 != 0 {
        return;
    }
    let z = if fl & 0x80 != 0 { cf(k::K_858B50) } else { force.z };
    move_part(this, [force.x, force.y, z], MOVE_SPEED);
}

/// `CPhysical::ApplyFrictionMoveForce(CVector)` (0x5430A0, `ret 0xC`): as ApplyMoveForce into
/// `m_vecFrictionMoveSpeed`.
#[hook(addr = 0x5430A0, conv = thiscall, name = "CPhysical::ApplyFrictionMoveForce", expect = "8B 41 40 A8 60", this = "CPhysical")]
pub fn apply_friction_move_force(this: Ent, force: CVector) {
    let fl = pflags(this);
    if fl & 0x60 != 0 {
        return;
    }
    let z = if fl & 0x80 != 0 { cf(k::K_858B50) } else { force.z };
    move_part(this, [force.x, force.y, z], FRICTION_MOVE);
}

/// `CPhysical::ApplyTurnForce(CVector force, CVector point)` (0x542A50, `ret 0x18`): nothing with
/// `bDisableTurnForce`; the turn part into `m_vecTurnSpeed` (no infinite-mass turn mass).
#[hook(addr = 0x542A50, conv = thiscall, name = "CPhysical::ApplyTurnForce", expect = "83 EC 18 56 8B F1", this = "CPhysical")]
pub fn apply_turn_force(this: Ent, force: CVector, point: CVector) {
    if pflags(this) & 0x10 != 0 {
        return;
    }
    turn_part(this, va(force), va(point), false, TURN_SPEED);
}

/// `CPhysical::ApplyFrictionTurnForce(CVector force, CVector point)` (0x543100, `ret 0x18`; spec
/// CPhysical.md): the turn part into `m_vecFrictionTurnSpeed`, infinite-mass turn mass.
#[hook(addr = 0x543100, conv = thiscall, name = "CPhysical::ApplyFrictionTurnForce", expect = "83 EC 1C 56 8B F1", this = "CPhysical")]
pub fn apply_friction_turn_force(this: Ent, force: CVector, point: CVector) {
    if pflags(this) & 0x10 != 0 {
        return;
    }
    turn_part(this, va(force), va(point), true, FRICTION_TURN);
}

/// `CPhysical::ApplyForce(CVector force, CVector point, bool updateTurnSpeed)` (0x542B50, `ret
/// 0x1C`): the move part (z of the local copy zeroed with `bDisableZ`) unless
/// `bDisableMoveForce`/`bInfiniteMass`; the turn part (on the argument force) when not
/// `bDisableTurnForce` and `updateTurnSpeed`.
#[hook(addr = 0x542B50, conv = thiscall, name = "CPhysical::ApplyForce", expect = "83 EC 18 8B 44 24 1C", this = "CPhysical")]
pub fn apply_force(this: Ent, force: CVector, point: CVector, update_turn: u8) {
    let fl = pflags(this);
    let z = if fl & 0x80 != 0 { 0.0 } else { force.z };
    if fl & 0x60 == 0 {
        move_part(this, [force.x, force.y, z], MOVE_SPEED);
    }
    if pflags(this) & 0x10 != 0 || update_turn == 0 {
        return;
    }
    turn_part(this, va(force), va(point), true, TURN_SPEED);
}

/// `CPhysical::ApplyFrictionForce(CVector force, CVector point)` (0x543220, `ret 0x18`): as
/// ApplyForce into the friction speeds, the turn part always (unless `bDisableTurnForce`).
#[hook(addr = 0x543220, conv = thiscall, name = "CPhysical::ApplyFrictionForce", expect = "83 EC 1C 8B 44 24 20", this = "CPhysical")]
pub fn apply_friction_force(this: Ent, force: CVector, point: CVector) {
    let fl = pflags(this);
    let z = if fl & 0x80 != 0 { 0.0 } else { force.z };
    if fl & 0x60 == 0 {
        move_part(this, [force.x, force.y, z], FRICTION_MOVE);
    }
    if pflags(this) & 0x10 != 0 {
        return;
    }
    turn_part(this, va(force), va(point), true, FRICTION_TURN);
}

/// `CPhysical::GetSpeed(CVector point)` (0x542CE0, `CVector* (CVector* out, CVector point)`, `ret
/// 0x10`): `moveSpeed + frictionMove + (turnSpeed + frictionTurn) x (point - com)` with the
/// original's sum order (x: stored `move.x + c.x`, y: stored `c.y + move.y`, z in registers).
#[hook(addr = 0x542CE0, conv = thiscall, name = "CPhysical::GetSpeed", expect = "83 EC 24 56 8B F1", this = "CPhysical")]
pub fn get_speed(this: Ent, out: GPtr<CVector>, point: CVector) -> GPtr<CVector> {
    let mut com = CVector::new(0.0, 0.0, 0.0);
    if pflags(this) & 0x40 == 0 {
        let mut o = CVector::new(0.0, 0.0, 0.0);
        let r = multiply3x3(lp(&mut o), matrix(this), this.byte_add(CENTRE_OF_MASS).cast());
        com = cv(rv(r.cast(), 0));
    }
    let mut f = Fpu::new();
    f.fld(point.x);
    f.fsub_m(com.x);
    let rx = f.fstp();
    f.fld(point.y);
    f.fsub_m(com.y);
    let ry = f.fstp();
    f.fld(point.z);
    f.fsub_m(com.z);
    let rz = f.fstp();
    f.fld(rf(this, TURN_SPEED));
    f.fadd_m(rf(this, FRICTION_TURN));
    let wx = f.fstp();
    f.fld(rf(this, TURN_SPEED + 4));
    f.fadd_m(rf(this, FRICTION_TURN + 4));
    let wy = f.fstp();
    f.fld(rf(this, TURN_SPEED + 8));
    f.fadd_m(rf(this, FRICTION_TURN + 8));
    let wz = f.fstp();
    let mut w = CVector::new(wx, wy, wz);
    let mut r = CVector::new(rx, ry, rz);
    let mut o = CVector::new(0.0, 0.0, 0.0);
    let c = rv(cross_product(lp(&mut o), lp(&mut w), lp(&mut r)).cast(), 0);
    f.fld(rf(this, MOVE_SPEED));
    f.fadd_m(c[0]);
    let sx = f.fstp();
    f.fld(c[1]);
    f.fadd_m(rf(this, MOVE_SPEED + 4));
    let sy = f.fstp();
    f.fld(c[2]);
    f.fadd_m(rf(this, MOVE_SPEED + 8));
    f.fadd_m(rf(this, FRICTION_MOVE + 8));
    f.fld(sy);
    f.fadd_m(rf(this, FRICTION_MOVE + 4));
    f.fld(sx);
    f.fadd_m(rf(this, FRICTION_MOVE));
    let o = out.cast::<u8>();
    wf(o, 0, f.fstp());
    wf(o, 4, f.fstp());
    wf(o, 8, f.fstp());
    out
}

/// `CPhysical::ApplyMoveSpeed()` (0x542DD0; ECX preserved: ApplySpeed calls ApplyTurnSpeed right after
/// it without reloading `this`, sites 0x547D96/0x547EDD/0x5480B0/0x548248): with `bDontApplySpeed` (0x2000) or
/// `bDisableMoveForce` the move speed is zeroed (z, y, x); otherwise `pos += timeStep * moveSpeed`
/// (the matrix pointer re-read per component, no null check).
#[hook(addr = 0x542DD0, conv = thiscall, name = "CPhysical::ApplyMoveSpeed", expect = "8B 41 40 F6 C4 20", this = "CPhysical", preserve = "ecx")]
pub fn apply_move_speed(this: Ent) {
    let fl = pflags(this);
    if fl & 0x2000 != 0 || fl & 0x20 != 0 {
        wu(this, MOVE_SPEED + 8, 0);
        wu(this, MOVE_SPEED + 4, 0);
        wu(this, MOVE_SPEED, 0);
        return;
    }
    for c in 0..3 {
        let mut f = Fpu::new();
        f.fld(sa_core::timer::TIME_STEP.get());
        let m = matrix(this);
        f.fmul_m(rf(this, MOVE_SPEED + 4 * c));
        f.fadd_m(rf(m, 0x30 + 4 * c));
        wf(m, 0x30 + 4 * c, f.fstp());
    }
}

/// `CPhysical::ApplyTurnSpeed()` (0x542E20): with `bDontApplySpeed` the turn speed is zeroed (z, y,
/// x). Otherwise `a = timeStep * turnSpeed` (f32); right += a x right, forward += a x forward, up
/// += a x up (each CrossProduct on the updated matrix); then, unless `bDisableMoveForce` or
/// `bInfiniteMass`, `pos += a x Multiply3x3(matrix, -com)`.
#[hook(addr = 0x542E20, conv = thiscall, name = "CPhysical::ApplyTurnSpeed", expect = "83 EC 3C 56 8B F1", this = "CPhysical")]
pub fn apply_turn_speed(this: Ent) {
    if pflags(this) & 0x2000 != 0 {
        wu(this, TURN_SPEED + 8, 0);
        wu(this, TURN_SPEED + 4, 0);
        wu(this, TURN_SPEED, 0);
        return;
    }
    let ts = sa_core::timer::TIME_STEP.get();
    let mut a = [0f32; 3];
    for (c, a) in a.iter_mut().enumerate() {
        let mut f = Fpu::new();
        f.fld(ts);
        f.fmul_m(rf(this, TURN_SPEED + 4 * c as isize));
        *a = f.fstp();
    }
    let mut av = cv(a);
    // right (+0), forward (+0x10), up (+0x20)
    for row in [0isize, 0x10, 0x20] {
        let m = matrix(this);
        let mut o = CVector::new(0.0, 0.0, 0.0);
        let c = rv(cross_product(lp(&mut o), lp(&mut av), m.byte_add(row).cast()).cast(), 0);
        for (i, ci) in c.iter().enumerate() {
            let m = matrix(this);
            let off = row + 4 * i as isize;
            let mut f = Fpu::new();
            f.fld(*ci);
            f.fadd_m(rf(m, off));
            wf(m, off, f.fstp());
        }
    }
    if pflags(this) & 0x60 != 0 {
        return;
    }
    let mut f = Fpu::new();
    let mut n = [0f32; 3];
    for (i, n) in n.iter_mut().enumerate() {
        f.fld(rf(this, CENTRE_OF_MASS + 4 * i as isize));
        f.fchs();
        *n = f.fstp();
    }
    let mut nv = cv(n);
    let mut o = CVector::new(0.0, 0.0, 0.0);
    let r = multiply3x3(lp(&mut o), matrix(this), lp(&mut nv));
    let mut o2 = CVector::new(0.0, 0.0, 0.0);
    let c = rv(cross_product(lp(&mut o2), lp(&mut av), r).cast(), 0);
    for (i, ci) in c.iter().enumerate() {
        let m = matrix(this);
        let off = 0x30 + 4 * i as isize;
        let mut f = Fpu::new();
        f.fld(*ci);
        f.fadd_m(rf(m, off));
        wf(m, off, f.fstp());
    }
}

/// `CPhysical::ApplyGravity()` (0x542FE0): with `bApplyGravity` and not `bDisableMoveForce`: an
/// infinite-mass physical gets `ApplyForce((0, 0, timeStep*mass*-0.008f), Multiply3x3(matrix,
/// com), true)`; otherwise, when it uses collision (entity flag 1), `moveSpeed.z -= timeStep *
/// 0.008f`.
#[hook(addr = 0x542FE0, conv = thiscall, name = "CPhysical::ApplyGravity", expect = "83 EC 18 56 8B F1", this = "CPhysical")]
pub fn apply_gravity(this: Ent) {
    let fl = pflags(this);
    if fl & 2 == 0 || fl & 0x20 != 0 {
        return;
    }
    if fl & 0x40 != 0 {
        let mut f = Fpu::new();
        f.fld(sa_core::timer::TIME_STEP.get());
        f.fmul_m(rf(this, MASS));
        f.fmul_m(cf(k::K_863BFC));
        let z = f.fstp();
        let mut o = CVector::new(0.0, 0.0, 0.0);
        let p = rv(multiply3x3(lp(&mut o), matrix(this), this.byte_add(CENTRE_OF_MASS).cast()).cast(), 0);
        apply_force_call(this, CVector::new(0.0, 0.0, z), cv(p), 1);
        return;
    }
    if ru(this, EFLAGS) & 1 == 0 {
        return;
    }
    let mut f = Fpu::new();
    f.fld(sa_core::timer::TIME_STEP.get());
    f.fmul_m(cf(k::K_863984));
    f.fsubr_m(rf(this, MOVE_SPEED + 8));
    wf(this, MOVE_SPEED + 8, f.fstp());
}

orig! {
    /// `CPhysical::ApplyForce(CVector, CVector, bool)` (0x542B50)
    pub fn apply_force_call(e: Ent, force: CVector, point: CVector, update_turn: u8)
        = thiscall 0x542B50 => apply_force;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    pub fn apply_move_force_call(e: Ent, force: CVector) = thiscall 0x5429F0 => apply_move_force;
    /// `CPhysical::ApplyTurnForce(CVector, CVector)` (0x542A50)
    pub fn apply_turn_force_call(e: Ent, force: CVector, point: CVector) = thiscall 0x542A50 => apply_turn_force;
    /// `CPhysical::ApplyFrictionForce(CVector, CVector)` (0x543220)
    pub fn apply_friction_force_call(e: Ent, force: CVector, point: CVector) = thiscall 0x543220 => apply_friction_force;
    /// `CPhysical::GetSpeed(CVector)` (0x542CE0)
    pub fn get_speed_call(e: Ent, out: GPtr<CVector>, point: CVector) -> GPtr<CVector> = thiscall 0x542CE0 => get_speed;
}

/// `CPhysical::ApplyAirResistance()` (0x544C40). With `m_fAirResistance > 0.1f` and not a vehicle:
/// move and turn speeds `*= pow(airResistance, timeStep)`. Otherwise `r = |moveSpeed| *
/// airResistance` (times 2.5 for a vehicle of type 0 or 9 when
/// `CCullZones::DoExtraAirResistanceForPlayer`), move speed `*= pow(1 - r, timeStep)`, turn speed
/// `*= 0.99f`. `pow` is the CRT's `_CIpow` (0x822130), called as the original does.
#[hook(addr = 0x544C40, conv = thiscall, name = "CPhysical::ApplyAirResistance", expect = "51 56 8B F1 D9 86 98 00 00 00", this = "CPhysical")]
pub fn apply_air_resistance(this: Ent) {
    let mut f = Fpu::new();
    f.fld(rf(this, AIR_RESISTANCE));
    f.fcomp_m(cf(k::K_858B1C));
    if !f.jne(0x41) && etype(this) != ty::VEHICLE {
        let p = ci_pow(X87::from_f32(rf(this, AIR_RESISTANCE)), X87::from_f32(sa_core::timer::TIME_STEP.get()));
        f.push(p);
        for off in [MOVE_SPEED, MOVE_SPEED + 4, MOVE_SPEED + 8, TURN_SPEED, TURN_SPEED + 4] {
            f.fld_st(0);
            f.fmul_m(rf(this, off));
            wf(this, off, f.fstp());
        }
        f.fmul_m(rf(this, TURN_SPEED + 8));
        wf(this, TURN_SPEED + 8, f.fstp());
        return;
    }
    let mut f = Fpu::new();
    f.fld(rf(this, MOVE_SPEED + 8));
    f.fld(rf(this, MOVE_SPEED + 4));
    f.fld(rf(this, MOVE_SPEED));
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
    f.fmul_m(rf(this, AIR_RESISTANCE));
    let mut r = f.fstp();
    if do_extra_air_resistance_for_player() != 0 && etype(this) == ty::VEHICLE {
        let vt = ru(this, 0x594);
        if vt == 0 || vt == 9 {
            f.fld(G_8D34A8.get());
            f.fmul_m(r);
            r = f.fstp();
        }
    }
    f.fld(cf(k::K_858624));
    f.fsub_m(r);
    let base = f.pop();
    let p = ci_pow(base, X87::from_f32(sa_core::timer::TIME_STEP.get()));
    f.push(p);
    f.fld_st(0);
    f.fmul_m(rf(this, MOVE_SPEED));
    wf(this, MOVE_SPEED, f.fstp());
    f.fld_st(0);
    f.fmul_m(rf(this, MOVE_SPEED + 4));
    wf(this, MOVE_SPEED + 4, f.fstp());
    f.fmul_m(rf(this, MOVE_SPEED + 8));
    wf(this, MOVE_SPEED + 8, f.fstp());
    for off in [TURN_SPEED, TURN_SPEED + 4, TURN_SPEED + 8] {
        f.fld(rf(this, off));
        f.fmul_m(cf(k::K_862CD0));
        wf(this, off, f.fstp());
    }
}

/// `CPhysical::SetDamagedPieceRecord(float intensity, CEntity* entity, CColPoint& cp, float
/// distMult)` (0x5428C0, `ret 0x10`). When `intensity > m_fDamageIntensity` (NaN false): intensity,
/// piece type (`cp.pieceA`, +0x21), damage entity (reference moved), impact position (`cp.point`)
/// and velocity (`distMult * cp.normal`); a glass-like surface (0x41: `cp.surfaceB` for an object
/// this, else `cp.surfaceA` for an object entity) sets object flag 0x80000 at +0x140. Always: an
/// object with `bDisableZ` hit by the model `0x8CD778` gets +0x148 = 0x35 if it was 0xFF, else 0x32.
#[hook(addr = 0x5428C0, conv = thiscall, name = "CPhysical::SetDamagedPieceRecord", expect = "D9 44 24 04 83 EC 0C", this = "CPhysical")]
pub fn set_damaged_piece_record(this: Ent, intensity: u32, entity: Ent, cp: Ent, dist_mult: f32) {
    let mut f = Fpu::new();
    f.fld(f32::from_bits(intensity));
    f.fcomp_m(rf(this, DAMAGE_INTENSITY));
    if !f.jne(0x41) {
        wu(this, DAMAGE_INTENSITY, intensity);
        wh(this, PIECE_TYPE, rb(cp, 0x21) as u16);
        let slot: GPtr<Ent> = this.byte_add(DAMAGE_ENTITY).cast();
        let old = slot.read();
        if !old.is_null() {
            clean_up_old_reference(old, slot);
        }
        slot.write(entity);
        register_reference(entity, slot);
        wv_bits(this, LAST_IMPACT_POS, rv_bits(cp, 0));
        let mut v = [0f32; 3];
        for (i, v) in v.iter_mut().enumerate() {
            f.fld(dist_mult);
            f.fmul_m(rf(cp, 0x10 + 4 * i as isize));
            *v = f.fstp();
        }
        wv(this, LAST_IMPACT_VELOCITY, v);
        if etype(this) == ty::OBJECT && rb(cp, 0x23) == 0x41 {
            wu(this, 0x140, ru(this, 0x140) | 0x80000);
        } else if etype(entity) == ty::OBJECT && rb(cp, 0x20) == 0x41 {
            wu(entity, 0x140, ru(entity, 0x140) | 0x80000);
        }
    }
    if rb(this, PFLAGS) & 0x80 != 0 {
        let model = rh(entity, MODEL_INDEX) as i16 as i32;
        let special = sa_stream::modelinfo::model_indices::MODEL_INDICES_B
            .ptr()
            .cast::<u16>()
            .wrapping_index(0x1C)
            .read();
        if model == special as i32 && etype(this) == ty::OBJECT {
            let b = rb(this, 0x148);
            wb(this, 0x148, if b == 0xFF { 0x35 } else { 0x32 });
        }
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_APPLY_MOVE_FORCE,
    HOOK_APPLY_FRICTION_MOVE_FORCE,
    HOOK_APPLY_TURN_FORCE,
    HOOK_APPLY_FRICTION_TURN_FORCE,
    HOOK_APPLY_FORCE,
    HOOK_APPLY_FRICTION_FORCE,
    HOOK_GET_SPEED,
    HOOK_APPLY_MOVE_SPEED,
    HOOK_APPLY_TURN_SPEED,
    HOOK_APPLY_GRAVITY,
    HOOK_APPLY_AIR_RESISTANCE,
    HOOK_SET_DAMAGED_PIECE_RECORD,
];
