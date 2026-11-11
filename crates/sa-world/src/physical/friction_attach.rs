//! `CPhysical` collision response, friction and attachment (batch `mvpf-phys-1`): `ApplyCollisionAlt`,
//! the friction between two physicals (`ApplyFriction`-other) and
//! `PositionAttachedEntity`. Ported from the 1.0 US bytes
//! (SecuROM fragments inlined), x87 sequences through the [`Fpu`] model (PC = 24) in the original's
//! operand order; values the original keeps on the x87 stack stay on the model's stack. gta-reversed
//! `Entity/Physical.cpp` was a reading aid only. Unverified (docs/verify/HOWTO.md).
//!
//! `CColPoint` offsets: point +0, normal +0x10, surfaceA +0x20, pieceA +0x21, surfaceB +0x23,
//! pieceB +0x24, depth +0x28.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::attach::{
    allocate_matrix, get_matrix, magnitude_sqr, matrix_assign, matrix_copy_ctor, matrix_dtor, matrix_mul_assign,
    matrix_mul_point, matrix_reset_orientation, matrix_update_matrix, rw_frame_get_ltm, set_is_static_v,
    simple_transform_update_matrix, LocalMatrix,
};
use super::collide::{report_collision, AUDIO_ENGINE};
use super::friction::dot_product;
use super::forces::{apply_force_call, apply_friction_force_call, apply_move_force_call, get_speed_call};
use super::shared::*;
use crate::collision::fpu::Fpu;

orig! {
    /// `CPhysical::ApplySoftCollision(CEntity*, CColPoint&, float&)` (0x543890, AL)
    pub fn apply_soft_collision_1_call(e: Ent, entity: Ent, cp: Ent, damage: GPtr<f32>) -> u8
        = thiscall 0x543890 => crate::physical::collide::apply_soft_collision;
    /// virtual `CVehicle::AddMovingCollisionSpeed(CVector&)` (vtable +0xC4, hidden out first)
    pub fn add_moving_collision_speed_v(e: Ent, out: Ent, point: Ent) -> Ent = thiscall vslot 0xC4 => unported;
    /// `operator/(const CVector&, float)` (0x4119D0, cdecl, out first)
    pub fn vector_div(out: Ent, v: Ent, s: f32) -> Ent = cdecl 0x4119d0 => crate::world::mvpa_math::vector_div_scalar;
    /// `CVector::operator+=(const CVector&)` (0x411A00, thiscall)
    pub fn vector_add_assign(v: Ent, o: Ent) -> Ent = thiscall 0x411a00 => crate::world::mvpa_math::vector_add_assign;
    /// `CVector::SquaredMagnitude()` (0x406DA0, thiscall, ST0; the port returns an X87, so unported here)
    pub fn vector_sq_mag(v: Ent) -> f64 = thiscall 0x406da0 => unported;
    /// `operator+(const CVector&, const CVector&)` (0x40FE30, cdecl, out first)
    pub fn vector_add(out: Ent, a: Ent, b: Ent) -> Ent = cdecl 0x40fe30 => crate::world::mvpa_math::vector_add;
    /// `VectorSub(CVector*, const CVector*, const CVector*)` (0x40FE60, cdecl)
    pub fn vector_sub(out: Ent, a: Ent, b: Ent) -> Ent = cdecl 0x40fe60 => crate::world::mvpa_math::vector_sub;
    /// `operator*(float, const CVector&)` (0x40FE90, cdecl, out first)
    pub fn scalar_mul_vector(out: Ent, s: f32, v: Ent) -> Ent = cdecl 0x40fe90 => crate::world::mvpa_math::scalar_mul_vector;
    /// `CMatrix::Rotate(const CVector&)` (0x541E90, thiscall; no catalogue name)
    pub fn matrix_rotate_vec(m: Ent, v: Ent) = thiscall 0x541e90 => unported;
    /// `CMatrix::operator*(const CMatrix&, const CMatrix&)` (0x59BE30, cdecl, out first)
    pub fn matrix_mul_mat(out: Ent, a: Ent, b: Ent) -> Ent = cdecl 0x59be30 => unported;
    /// `CMatrix::SetTranslateOnly(float, float, float)` (0x59AF80, thiscall)
    pub fn matrix_set_translate_only(m: Ent, x: f32, y: f32, z: f32) = thiscall 0x59af80 => unported;
    /// `CPlaceable::SetMatrix(const CMatrix&)` (0x54F610, thiscall)
    pub fn placeable_set_matrix(e: Ent, m: Ent) = thiscall 0x54f610 => crate::world::mvpa2::placeable_set_matrix;
    /// `CBike::CalculateLeanMatrix()` (0x6B7150, thiscall; sa-veh's port is outside this crate)
    pub fn bike_calculate_lean_matrix(b: Ent) = thiscall 0x6b7150 => unported;
    /// `CPhysical::DettachAutoAttachedEntity()` (0x5446A0)
    pub fn dettach_auto_attached_entity_call(e: Ent) = thiscall 0x5446a0 => crate::physical::attach::dettach_auto_attached_entity;
    /// `CPhysical::DettachEntityFromEntity(float, float, float, bool)` (0x5442F0)
    pub fn dettach_entity_from_entity_call(e: Ent, x: u32, y: u32, z: f32, apply_turn: u8)
        = thiscall 0x5442f0 => crate::physical::attach::dettach_entity_from_entity;
    /// `CGeneral::GetRandomNumberInRange<float>(float, float)` (0x41BD90, cdecl, ST0)
    pub fn random_float(min: f32, max: f32) -> f64 = cdecl 0x41bd90 => unported;
    /// `CPhysical::ApplyFrictionMoveForce(CVector)` (0x5430A0)
    pub fn apply_friction_move_force_call(e: Ent, force: CVector)
        = thiscall 0x5430a0 => crate::physical::forces::apply_friction_move_force;
}

/// `fpatan` on the model: `st1 = atan2(st1, st0)`, pop.
fn fpatan(f: &mut Fpu) {
    let x = f.pop();
    let y = f.pop();
    f.push(y.fpatan(x));
}

/// The heading the original inlines from `GetHeading()`: `atan2(-m.forward.x, m.forward.y)` with a
/// matrix (`fld fy; fld fx; fchs; fxch; fpatan`), else the placement heading (+0x10).
fn push_heading(f: &mut Fpu, e: Ent) {
    let m = matrix(e);
    if m.is_null() {
        f.fld(rf(e, 0x10));
    } else {
        f.fld(rf(m, 0x14));
        f.fld(rf(m, 0x10));
        f.fchs();
        f.fxch(1);
        fpatan(f);
    }
}

/// `AudioEngine.ReportCollision(this, entity, cp.surfaceA, cp.surfaceB, cp, normal, impact, impact2,
/// false, false)`.
fn report2(this: Ent, entity: Ent, cp: Ent, normal: Ent, impact: f32, impact2: u32) {
    report_collision(
        GPtr::from_addr(AUDIO_ENGINE),
        this,
        entity,
        rb(cp, 0x20) as u32,
        rb(cp, 0x23) as u32,
        cp,
        normal,
        impact,
        impact2,
        0,
        0,
    );
}

/// `1 / ((((c.x*c.x) + c.y*c.y) + c.z*c.z) / turnMass + 1/mass)` from `c` = the cross product the
/// callers just computed (x87 registers, the result stored as f32).
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

/// `|move.x| < limit && |move.y| < limit && |move.z| < limit + limit` (the three `fabs` compares of
/// `ApplyCollisionAlt`).
fn alt_slow(this: Ent, limit: f32) -> bool {
    let mut f = Fpu::new();
    f.fld(rf(this, MOVE_SPEED));
    f.fabs();
    f.fcomp_m(limit);
    if f.jp(5) {
        return false;
    }
    f.fld(rf(this, MOVE_SPEED + 4));
    f.fabs();
    f.fcomp_m(limit);
    if f.jp(5) {
        return false;
    }
    f.fld(rf(this, MOVE_SPEED + 8));
    f.fabs();
    f.fld(limit);
    f.fadd_st(0);
    f.fcompp();
    !f.jne(0x41)
}

/// `CPhysical::ApplyCollisionAlt(CEntity* entity, CColPoint& cp, float& damage, CVector& moveSpeed,
/// CVector& turnSpeed)` (0x544D50, `ret 0x14`, AL).
///
/// An attached-to vehicle or object (type 2 or 4) first gets `ApplySoftCollision`-1(entity, cp, d)
/// with `d` = the `cp` argument slot zeroed (result ignored). `bDisableTurnForce` (flag 0x10):
/// `s = (move.z*n.z + move.y*n.y) + move.x*n.x`; unless `s < 0` false; `damage = -(s*mass)`,
/// `ApplyMoveForce(damage*n)`, report impact `damage/mass`, true.
///
/// Otherwise `r = cp.point - pos`, `v = GetSpeed(r)` (+ the vehicle's virtual
/// `AddMovingCollisionSpeed(r)` with physical flag 0x4000000 and surfaceA 0x41; x and y stored, z kept
/// on the x87 stack), `s = (v.z*n.z + n.x*v.x) + n.y*v.y` (stored into the `entity` argument slot);
/// unless `s < 0` false. `d = r - Multiply3x3(matrix, com)` (com zero with flag 0x40), `m` = the
/// collision mass of `d x n`, `limit = timeStep * 0.008` (stored into the `cp` argument slot), scaled
/// by 1.3 for an object (class 1), and for a vehicle without physical flag 0x100: 1.7 for a bike (type
/// 9) with status 4 or 5 (class 3), 1.5 for a boat (type 5, class 4), 1.4 when `up.z < -0.3` (class
/// 2). Impact2 = 1.0. Class 1: when entity flag 8 is clear and the move speed is under the limit (`|x|,
/// |y| < limit`, `|z| < 2 limit`): `damage = -0.98*m*s`, impact2 = 0, and the code falls through to the
/// generic damage below, which overwrites it (original quirk). Classes 2/3/4 under the limit: `damage
/// = c*m*s` (c = -0.95) and impact2 = 0, done. Else a boat hit with surfaceB 0x2B or `n.z < 0.5`:
/// `damage = -((2*elasticity + 1)*m*s)`; else `damage = -((elasticity + 1)*m*s)`.
///
/// `f = n*damage`. With any of physical flags 0xE0: `ApplyForce(f, r, true)`. Otherwise `dm =
/// f/mass`; a vehicle with entity flag 0x1000 whose `|move|² > 0.1` or whose collider is neither a
/// building nor has physical flag 4, or a non-vehicle: `moveSpeed += dm`; other vehicles: `moveSpeed
/// += dm*1.2`; vehicles then scale `f` by 0.8. `turnSpeed += CrossProduct(r - Multiply3x3(matrix,
/// com), f) / turnMass` (com not zeroed here). Report impact `damage/m` with impact2, true.
#[hook(addr = 0x544d50, conv = thiscall, name = "CPhysical::ApplyCollisionAlt", expect = "83 EC 68 53 8B 5C 24 74",
       this = "CPhysical", unverified = "mvpf-phys-1", route = "L2", hazards = "stack-residue",
       recs = "grove-walk,driving",
       note = "writes 0 and then timeStep*0.008 (limit) into its own cp argument slot and s into its entity argument slot")]
pub fn apply_collision_alt(this: Ent, entity: Ent, cp: Ent, damage: GPtr<f32>, out_move: Ent, out_turn: Ent) -> u8 {
    // the `cp` argument slot (zeroed, the attached-to entity's damage, later the speed limit)
    let mut cp_slot: f32 = 0.0;
    let at = rp(this, ATTACHED_TO);
    if !at.is_null() {
        let t = etype(at);
        if t > 1 && t < 5 && t != ty::PED {
            apply_soft_collision_1_call(at, entity, cp, lp(&mut cp_slot));
        }
    }
    let n = cp.byte_add(0x10);
    let mut f = Fpu::new();
    if rb(this, PFLAGS) & 0x10 != 0 {
        f.fld(rf(this, MOVE_SPEED + 8));
        f.fmul_m(rf(n, 8));
        f.fld(rf(this, MOVE_SPEED + 4));
        f.fmul_m(rf(n, 4));
        f.faddp(1);
        f.fld(rf(this, MOVE_SPEED));
        f.fmul_m(rf(n, 0));
        f.faddp(1);
        f.fcom_m(cf(k::K_858B50));
        if f.jp(5) {
            return 0;
        }
        f.fmul_m(rf(this, MASS));
        f.fchs();
        damage.write(f.fst());
        let mut v = [0f32; 3];
        f.fld_st(0);
        f.fmul_m(rf(n, 0));
        v[0] = f.fstp();
        f.fld_st(0);
        f.fmul_m(rf(n, 4));
        v[1] = f.fstp();
        f.fmul_m(rf(n, 8));
        v[2] = f.fstp();
        apply_move_force_call(this, cv(v));
        f.fld(damage.read());
        f.fdiv_m(rf(this, MASS));
        let impact = f.fstp();
        report2(this, entity, cp, n, impact, 0x3F80_0000);
        return 1;
    }
    let m0 = matrix(this);
    let pos = m0.byte_add(0x30);
    f.fld(rf(cp, 8));
    f.fsub_m(rf(pos, 8));
    f.fld(rf(cp, 4));
    f.fsub_m(rf(pos, 4));
    f.fld(rf(cp, 0));
    f.fsub_m(rf(pos, 0));
    let mut r = [0f32; 3];
    r[0] = f.fstp();
    r[1] = f.fstp();
    r[2] = f.fstp();
    let mut vo = CVector::new(0.0, 0.0, 0.0);
    get_speed_call(this, lp(&mut vo), cv(r));
    let mut v = va(vo);
    if ru(this, PFLAGS) & 0x0400_0000 != 0 && etype(this) == ty::VEHICLE && rb(cp, 0x20) == 0x41 {
        let mut mo = [0f32; 3];
        let mut rr = r;
        let mc = add_moving_collision_speed_v(this, lp(&mut mo).cast(), lp(&mut rr).cast());
        f.fld(v[0]);
        f.fadd_m(rf(mc, 0));
        v[0] = f.fstp();
        f.fld(v[1]);
        f.fadd_m(rf(mc, 4));
        v[1] = f.fstp();
        f.fld(v[2]);
        f.fadd_m(rf(mc, 8));
    } else {
        f.fld(v[2]);
    }
    let nl = rv(n, 0);
    f.fld(nl[0]);
    f.fmul_m(v[0]);
    f.fxch(1);
    f.fmul_m(nl[2]);
    f.faddp(1);
    f.fld(nl[1]);
    f.fmul_m(v[1]);
    f.faddp(1);
    // stored into the `entity` argument slot
    let s = f.fst();
    f.fcomp_m(cf(k::K_858B50));
    if f.jp(5) {
        return 0;
    }
    let mut co = CVector::new(0.0, 0.0, 0.0);
    multiply3x3(lp(&mut co), matrix(this), this.byte_add(CENTRE_OF_MASS).cast());
    let mut com = va(co);
    if rb(this, PFLAGS) & 0x40 != 0 {
        com = [0.0; 3];
    }
    let mut d = [0f32; 3];
    for i in 0..3 {
        f.fld(r[i]);
        f.fsub_m(com[i]);
        d[i] = f.fstp();
    }
    let mut dv = cv(d);
    let mut nv = cv(nl);
    let mut xo = CVector::new(0.0, 0.0, 0.0);
    let c = rv(cross_product(lp(&mut xo), lp(&mut dv), lp(&mut nv)).cast(), 0);
    let m = collision_mass(this, c);
    let et = rb(this, TYPE);
    let t = et & 7;
    f.fld(sa_core::timer::TIME_STEP.get());
    f.fmul_m(cf(k::K_863984));
    cp_slot = f.fstp();
    let mut class: u16 = 0;
    let mut scale: Option<u32> = None;
    if t == ty::OBJECT {
        class = 1;
        scale = Some(k::K_863B70);
    } else if t == ty::VEHICLE && ru(this, PFLAGS) & 0x100 == 0 {
        let vt = ru(this, 0x590);
        let st = et >> 3;
        if vt == 9 && (st == 4 || st == 5) {
            class = 3;
            scale = Some(k::K_863B80);
        } else if vt == 5 {
            class = 4;
            scale = Some(k::K_863B88);
        } else {
            f.fld(rf(matrix(this), 0x28));
            f.fcomp_m(cf(k::K_858EE4));
            if !f.jp(5) {
                class = 2;
                scale = Some(k::K_863B78);
            }
        }
    }
    if let Some(sc) = scale {
        f.fld(cf(sc));
        f.fmul_m(cp_slot);
        cp_slot = f.fstp();
    }
    let limit = cp_slot;
    let mut impact2: u32 = 0x3F80_0000;
    // `Some(c)`: the class 2/3/4 store `c*m*s` (no fchs), skipping the generic damage
    let mut special: Option<u32> = None;
    match class {
        1 => {
            if (ru(this, EFLAGS) >> 3) & 1 == 0 && alt_slow(this, limit) {
                f.fld(cf(k::K_863B74));
                impact2 = 0;
                f.fmul_m(m);
                f.fmul_m(s);
                damage.write(f.fstp());
            }
        }
        3 if alt_slow(this, limit) => special = Some(k::K_863B84),
        2 if alt_slow(this, limit) => special = Some(k::K_863B7C),
        4 if alt_slow(this, limit) => special = Some(k::K_863B8C),
        _ => {}
    }
    if let Some(c) = special {
        f.fld(cf(c));
        f.fmul_m(m);
        impact2 = 0;
        f.fmul_m(s);
    } else {
        let boat = etype(this) == ty::VEHICLE && ru(this, 0x590) == 5 && {
            if rb(cp, 0x23) == 0x2B {
                true
            } else {
                f.fld(nl[2]);
                f.fcomp_m(cf(k::K_858B8C));
                !f.jp(5)
            }
        };
        f.fld(rf(this, ELASTICITY));
        if boat {
            f.fadd_st(0);
        }
        f.fadd_m(cf(k::K_858624));
        f.fmul_m(m);
        f.fmul_m(s);
        f.fchs();
    }
    damage.write(f.fstp());
    let pf = rb(this, PFLAGS);
    f.fld(damage.read());
    let mut fv = [0f32; 3];
    f.fld(nl[0]);
    f.fmul_st(1);
    fv[0] = f.fstp();
    f.fld(nl[1]);
    f.fmul_st(1);
    fv[1] = f.fstp();
    f.fmul_m(nl[2]);
    fv[2] = f.fstp();
    if pf & 0xE0 != 0 {
        apply_force_call(this, cv(fv), cv(r), 1);
    } else {
        let mut dm = [0f32; 3];
        vector_div(lp(&mut dm).cast(), lp(&mut fv).cast(), rf(this, MASS));
        if etype(this) == ty::VEHICLE {
            let plain = if (ru(this, EFLAGS) >> 12) & 1 != 0 {
                f.push(X87::from_f64_exact(vector_sq_mag(this.byte_add(MOVE_SPEED))));
                f.fcomp_m(cf(k::K_858B1C));
                !f.jne(0x41) || (etype(entity) != ty::BUILDING && rb(entity, PFLAGS) & 4 == 0)
            } else {
                false
            };
            if plain {
                vector_add_assign(out_move, lp(&mut dm).cast());
            } else {
                let mut sc = [0f32; 3];
                for i in 0..3 {
                    f.fld(dm[i]);
                    f.fmul_m(cf(k::K_858F08));
                    sc[i] = f.fstp();
                }
                vector_add_assign(out_move, lp(&mut sc).cast());
            }
            for x in fv.iter_mut() {
                f.fld(*x);
                f.fmul_m(cf(k::K_858C98));
                *x = f.fstp();
            }
        } else {
            vector_add_assign(out_move, lp(&mut dm).cast());
        }
        let mut c2 = CVector::new(0.0, 0.0, 0.0);
        multiply3x3(lp(&mut c2), matrix(this), this.byte_add(CENTRE_OF_MASS).cast());
        let c2 = va(c2);
        let mut d2 = [0f32; 3];
        for i in 0..3 {
            f.fld(r[i]);
            f.fsub_m(c2[i]);
            d2[i] = f.fstp();
        }
        let mut d2v = cv(d2);
        let mut fvv = cv(fv);
        let mut xo2 = CVector::new(0.0, 0.0, 0.0);
        let cr = cross_product(lp(&mut xo2), lp(&mut d2v), lp(&mut fvv));
        let mut dt = [0f32; 3];
        vector_div(lp(&mut dt).cast(), cr.cast(), rf(this, TURN_MASS));
        vector_add_assign(out_turn, lp(&mut dt).cast());
    }
    f.fld(damage.read());
    f.fdiv_m(m);
    let impact = f.fstp();
    report2(this, entity, cp, n, impact, impact2);
    1
}


/// Where `PositionAttachedEntity`'s detach tests lead.
enum PaeNext {
    /// straight to the destructors (0x547B1F)
    End,
    /// the velocity / force update (0x547740)
    Normal,
    /// `DettachAutoAttachedEntity` and the throw-off forces (0x5475E6)
    Throw,
}

/// The common detach tests of a dumper / forklift load (0x547537): from the attached-to vehicle's
/// damage intensity, its matrix (+0x14, read directly) up.z / forward.z, last impact velocity and move
/// speed.
fn pae_load_tests(f: &mut Fpu, at: Ent) -> PaeNext {
    f.fld(rf(at, DAMAGE_INTENSITY));
    f.fcomp_m(cf(k::K_858B50));
    if !f.jne(0x41) {
        f.fld(rf(matrix(at), 0x28));
        f.fcomp_m(cf(k::K_858B1C));
        if !f.jp(5) {
            return PaeNext::Throw;
        }
    }
    f.fld(rf(at, DAMAGE_INTENSITY));
    f.fcomp_m(cf(k::K_858624));
    if !f.jne(0x41) {
        f.fld(rf(matrix(at), 0x18));
        f.fcomp_m(cf(k::K_863C18));
        if !f.jne(0x41) {
            return PaeNext::Throw;
        }
    }
    f.fld(rf(at, DAMAGE_INTENSITY));
    f.fcomp_m(cf(k::K_863B90));
    if f.jne(0x41) {
        return PaeNext::Normal;
    }
    f.push(X87::from_f64_exact(dot_product(at.byte_add(LAST_IMPACT_VELOCITY), matrix(at).byte_add(0x20))));
    f.fcomp_m(cf(k::K_858CB0));
    if f.jp(5) {
        return PaeNext::Normal;
    }
    f.push(X87::from_f64_exact(magnitude_sqr(at.byte_add(MOVE_SPEED))));
    f.fld(cf(k::K_863B94));
    f.fmul_m(cf(k::K_863B94));
    f.fcompp();
    if f.jne(0x41) {
        return PaeNext::Normal;
    }
    PaeNext::Throw
}

/// `CPhysical::PositionAttachedEntity()` (0x546FF0, `ret`; SEH frame, three local `CMatrix`es).
///
/// Nothing without an attached-to entity (+0xFC). The attached-to entity gets a matrix when it has
/// none (`AllocateMatrix`, placement `UpdateMatrix`); `B` = a copy of its matrix, for a bike (vehicle
/// type 9) its lean matrix (+0x5CC) after `CalculateLeanMatrix`, for this object on a forklift (model
/// 0x212) with a forks frame (+0x698) the frame's LTM (`UpdateMatrix`). With a zero attach quaternion
/// x/y/z (+0x118..): physical flag 0x40: `A = this.matrix`; else `C` = identity rotated by the attach
/// rotation (+0x10C), `A = B * C` (`operator=`, `*=`); `A.pos = B.pos + Multiply3x3(B, offset)`.
/// Otherwise the quaternion becomes an RwMatrix (`s = 2/((x²+y²)+z²+w²)`, pos 0, flags 3) taken by
/// `A.UpdateMatrix`, `A = B * A`, `A.SetTranslateOnly(B * offset)`. `SetMatrix(A)`; an object: with
/// entity flag 4 or 0x40000 `SetIsStatic(false)`, physical flag 0x2000000, fake physics 0.
///
/// Detach tests on a vehicle (this an object or a vehicle): dumper (0x196) / forklift (0x212) loads
/// fall off (`DettachAutoAttachedEntity`, then unless physical flag 4 a random sideways `ApplyMoveForce`
/// of `mass*0.01*(r*right + r*forward)` and with a moving component `mass*0.02*up`) on component angle
/// changes, building impacts, a tilted or heavily damaged vehicle; a vehicle towed by another one is
/// detached (`DettachEntityFromEntity(0, 0, 1, false)`) when `|right.z|` or `|forward.z|` exceeds
/// 0.707. Model 0x16B on a non-colliding object: auto-detach and `bUsesCollision`.
///
/// Then with physical flag 0x40: `v = (pos - prev) / max(timeStep, 1)` (prev at +0x10C, the delta
/// capped to length 2 when its square exceeds 4), `move = v`, `f = (v - oldMove) * -mass`,
/// `ApplyForce(f, Multiply3x3(matrix, com), true)`; a colliding non-fixed attached vehicle/object gets
/// `ApplyForce(-f, (com' + pos) - itsPos, true)`, and the heading difference (wrapped to ±pi, clamped
/// to ±G_8CD7E0) adds `diff * G_8CD7DC * mass` to turnSpeed.z. Without it: `move =
/// attached.GetSpeed(pos - itsPos)`, `turn = attached.turn`. Destroys B, C, A.
#[hook(addr = 0x546ff0, conv = thiscall, name = "CPhysical::PositionAttachedEntity", expect = "90 E9 E2 F7 EB FF",
       this = "CPhysical", unverified = "mvpf-phys-1", route = "L2", hazards = "x87-transcendental,entry-protection-read",
       recs = "grove-walk,driving",
       note = "fpatan/fsqrt through sa_math::x87; the RwMatrix local's pad words stay unwritten as in the original")]
pub fn position_attached_entity(this: Ent) {
    let at = rp(this, ATTACHED_TO);
    if at.is_null() {
        return;
    }
    let mut ma: LocalMatrix = [0; 18];
    let mut mb: LocalMatrix = [0; 18];
    let mut mc: LocalMatrix = [0; 18];
    let pa = lp(&mut ma).cast::<u8>();
    let pb = lp(&mut mb).cast::<u8>();
    let mcp = lp(&mut mc);
    pae_body(this, at, pa, pb, mcp);
    matrix_dtor(pb);
    matrix_dtor(mcp.cast());
    matrix_dtor(pa);
}

fn pae_body(this: Ent, at: Ent, pa: Ent, pb: Ent, mcp: GPtr<LocalMatrix>) {
    let pc = mcp.cast::<u8>();
    if matrix(at).is_null() {
        allocate_matrix(at);
        simple_transform_update_matrix(at.byte_add(PLACEMENT_POS), matrix(at));
    }
    matrix_copy_ctor(pb, matrix(at));
    let at = rp(this, ATTACHED_TO);
    if etype(at) == ty::VEHICLE && ru(at, 0x590) == 9 {
        bike_calculate_lean_matrix(at);
        matrix_assign(pb, rp(this, ATTACHED_TO).byte_add(0x5CC));
    } else if etype(this) == ty::OBJECT && rh(at, MODEL_INDEX) == 0x212 && !rp(at, 0x698).is_null() {
        let ltm = rw_frame_get_ltm(rp(at, 0x698));
        matrix_update_matrix(pb, ltm);
    }
    let mut f = Fpu::new();
    let mut quat = false;
    for off in [0x118, 0x11C, 0x120] {
        f.fld(rf(this, off));
        f.fcomp_m(cf(k::K_858B50));
        if f.jp(0x44) {
            quat = true;
            break;
        }
    }
    if !quat {
        if rb(this, PFLAGS) & 0x40 != 0 {
            matrix_assign(pa, matrix(this));
        } else {
            matrix_reset_orientation(mcp);
            matrix_rotate_vec(pc, this.byte_add(ATTACH_ROTATION));
            matrix_assign(pa, pb);
            matrix_mul_assign(pa, pc);
        }
        let mut oo = CVector::new(0.0, 0.0, 0.0);
        let o = rv(multiply3x3(lp(&mut oo), pb, this.byte_add(ATTACH_OFFSET).cast()).cast(), 0);
        let bpos = rv(pb, 0x30);
        f.fld(bpos[2]);
        f.fadd_m(o[2]);
        f.fld(bpos[1]);
        f.fadd_m(o[1]);
        f.fld(bpos[0]);
        f.fadd_m(o[0]);
        wf(pa, 0x30, f.fstp());
        wf(pa, 0x34, f.fstp());
        wf(pa, 0x38, f.fstp());
    } else {
        let q = this.byte_add(ATTACH_QUAT);
        let (qx, qy, qz, qw) = (rf(q, 0), rf(q, 4), rf(q, 8), rf(q, 0xC));
        f.fld(qy);
        f.fld(qx);
        f.fld(qz);
        f.fld(qw);
        f.fld_st(2);
        f.fmul_st(3);
        f.fld_st(4);
        f.fmul_st(5);
        f.faddp(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fld_st(1);
        f.fmul_st(2);
        f.faddp(1);
        f.fdivr_m(cf(k::K_858CA0));
        f.fstp_st(4);
        f.fstp_st(0);
        f.fstp_st(0);
        f.fstp_st(0);
        f.fld_st(0);
        f.fmul_m(qx);
        f.fld_st(1);
        f.fmul_m(qy);
        let ys = f.fstp();
        f.fxch(1);
        f.fmul_m(qz);
        f.fld_st(1);
        f.fmul_m(qw);
        let wx = f.fstp();
        f.fld(ys);
        f.fmul_m(qw);
        let wy = f.fstp();
        f.fld_st(0);
        f.fmul_m(qw);
        let wz = f.fstp();
        f.fld_st(1);
        f.fmul_m(qx);
        let xx = f.fstp();
        f.fld(ys);
        f.fmul_m(qy);
        let yy = f.fstp();
        f.fld_st(0);
        f.fmul_m(qz);
        let zz = f.fstp();
        f.fmul_m(qy);
        let yz = f.fstp();
        f.fmul_m(qz);
        f.fld(ys);
        f.fmul_m(qx);
        // RwMatrix local: right +0, flags +0xC, up +0x10, at +0x20, pos +0x30 (pads not written)
        let mut rw = [0u32; 16];
        f.fld(zz);
        f.fadd_m(yy);
        f.fsubr_m(cf(k::K_858624));
        rw[0] = f.fstp().to_bits();
        f.fld(wz);
        f.fadd_st(1);
        rw[1] = f.fstp().to_bits();
        f.fld_st(1);
        f.fsub_m(wy);
        rw[2] = f.fstp().to_bits();
        f.fsub_m(wz);
        rw[4] = f.fstp().to_bits();
        f.fld(zz);
        f.fadd_m(xx);
        f.fsubr_m(cf(k::K_858624));
        rw[12] = 0;
        rw[13] = 0;
        rw[5] = f.fstp().to_bits();
        rw[14] = 0;
        f.fld(yz);
        rw[3] = 3;
        f.fadd_m(wx);
        rw[6] = f.fstp().to_bits();
        f.fadd_m(wy);
        rw[8] = f.fstp().to_bits();
        f.fld(yz);
        f.fsub_m(wx);
        rw[9] = f.fstp().to_bits();
        f.fld(yy);
        f.fadd_m(xx);
        f.fsubr_m(cf(k::K_858624));
        rw[10] = f.fstp().to_bits();
        matrix_update_matrix(pa, lp(&mut rw).cast());
        let mut tmp: LocalMatrix = [0; 18];
        let pt = lp(&mut tmp).cast::<u8>();
        let r = matrix_mul_mat(pt, pb, pa);
        matrix_assign(pa, r);
        matrix_dtor(pt);
        let mut oo = CVector::new(0.0, 0.0, 0.0);
        let o = rv(matrix_mul_point(lp(&mut oo), pb, this.byte_add(ATTACH_OFFSET).cast()).cast(), 0);
        matrix_set_translate_only(pa, o[0], o[1], o[2]);
    }
    placeable_set_matrix(this, pa);
    if etype(this) == ty::OBJECT {
        let ef = ru(this, EFLAGS);
        if ef & 4 != 0 || ef & 0x40000 != 0 {
            set_is_static_v(this, 0);
        }
        wu(this, PFLAGS, ru(this, PFLAGS) | 0x0200_0000);
        wb(this, FAKE_PHYSICS, 0);
    }
    let at = rp(this, ATTACHED_TO);
    let next = if etype(at) == ty::VEHICLE {
        let tt = etype(this);
        if tt != ty::OBJECT && tt != ty::VEHICLE {
            PaeNext::Normal
        } else {
            let model = rh(at, MODEL_INDEX);
            if model == 0x196 {
                let ax = rh(at, 0x86C);
                if ax != 0 && ax != rh(at, 0x86E) {
                    PaeNext::Throw
                } else {
                    f.fld(rf(this, DAMAGE_INTENSITY));
                    f.fcomp_m(cf(k::K_858B50));
                    let de = rp(this, DAMAGE_ENTITY);
                    if !f.jne(0x41) && !de.is_null() && etype(de) == ty::BUILDING {
                        PaeNext::Throw
                    } else {
                        pae_load_tests(&mut f, at)
                    }
                }
            } else if model == 0x212 {
                if rh(at, 0x86C) == 0 && rh(at, 0x86E) != 0 {
                    PaeNext::Throw
                } else {
                    f.fld(rf(this, DAMAGE_INTENSITY));
                    f.fld(cf(k::K_863B98));
                    f.fmul_m(rf(this, MASS));
                    f.fxch(1);
                    f.fcompp();
                    let de = rp(this, DAMAGE_ENTITY);
                    if !f.jne(0x41) && !de.is_null() && etype(de) == ty::BUILDING {
                        PaeNext::Throw
                    } else {
                        pae_load_tests(&mut f, at)
                    }
                }
            } else if tt != ty::VEHICLE {
                PaeNext::Normal
            } else {
                f.fld(rf(get_matrix(at), 8));
                f.fabs();
                f.fcomp_m(cf(k::K_863C1C));
                let tilted = if f.jne(0x41) {
                    f.fld(rf(get_matrix(rp(this, ATTACHED_TO)), 0x18));
                    f.fabs();
                    f.fcomp_m(cf(k::K_863C1C));
                    !f.jne(0x41)
                } else {
                    true
                };
                if tilted {
                    dettach_entity_from_entity_call(this, 0, 0, 1.0, 0);
                    PaeNext::End
                } else {
                    PaeNext::Normal
                }
            }
        }
    } else if rh(this, MODEL_INDEX) == 0x16B && etype(at) == ty::OBJECT && rb(at, EFLAGS) & 1 == 0 {
        dettach_auto_attached_entity_call(this);
        wu(this, EFLAGS, ru(this, EFLAGS) | 1);
        PaeNext::End
    } else {
        PaeNext::Normal
    };
    match next {
        PaeNext::End => {}
        PaeNext::Throw => pae_throw(&mut f, this, at),
        PaeNext::Normal => pae_normal(&mut f, this),
    }
}

/// 0x5475E6: `DettachAutoAttachedEntity`, then (unless physical flag 4) the throw-off move forces.
fn pae_throw(f: &mut Fpu, this: Ent, at: Ent) {
    dettach_auto_attached_entity_call(this);
    if rb(this, PFLAGS) & 4 != 0 {
        return;
    }
    f.push(X87::from_f64_exact(random_float(-1.0, 1.0)));
    let r = f.fstp();
    let m = get_matrix(at);
    f.fld(r);
    f.fmul_m(rf(m, 8));
    let az = f.fstp();
    f.fld(r);
    f.fmul_m(rf(m, 4));
    let ay = f.fstp();
    f.fld(r);
    f.fmul_m(rf(m, 0));
    let ax = f.fstp();
    let m = get_matrix(at);
    f.fld(r);
    f.fmul_m(rf(m, 0x18));
    f.fld(r);
    f.fmul_m(rf(m, 0x14));
    f.fld(r);
    f.fmul_m(rf(m, 0x10));
    let t = f.fstp();
    f.fxch(1);
    f.fadd_m(az);
    let bz = f.fstp();
    f.fadd_m(ay);
    let by = f.fstp();
    f.fld(t);
    f.fadd_m(ax);
    let bx = f.fstp();
    f.fld(rf(this, MASS));
    f.fmul_m(cf(k::K_858C58));
    let kk = f.fstp();
    f.fld(bz);
    f.fmul_m(kk);
    f.fld(by);
    f.fmul_m(kk);
    f.fld(bx);
    f.fmul_m(kk);
    let fx = f.fstp();
    let fy = f.fstp();
    let fz = f.fstp();
    apply_move_force_call(this, cv([fx, fy, fz]));
    if rh(at, 0x86C) > rh(at, 0x86E) {
        let m = get_matrix(at);
        f.fld(rf(this, MASS));
        f.fmul_m(cf(k::K_858B38));
        let s = f.fstp();
        let mut o = [0f32; 3];
        let res = scalar_mul_vector(lp(&mut o).cast(), s, m.byte_add(0x20));
        apply_move_force_call(this, cv(rv(res, 0)));
    }
}

/// 0x547740: the velocity / force update after the detach tests.
fn pae_normal(f: &mut Fpu, this: Ent) {
    if rb(this, PFLAGS) & 0x40 == 0 {
        let at = rp(this, ATTACHED_TO);
        let t = etype(at);
        if t != ty::VEHICLE && t != ty::OBJECT {
            return;
        }
        let apos = pos_ptr(at);
        let tpos = pos_ptr(this);
        f.fld(rf(tpos, 8));
        f.fsub_m(rf(apos, 8));
        f.fld(rf(tpos, 4));
        f.fsub_m(rf(apos, 4));
        f.fld(rf(tpos, 0));
        f.fsub_m(rf(apos, 0));
        let rx = f.fstp();
        let ry = f.fstp();
        let rz = f.fstp();
        let mut o = CVector::new(0.0, 0.0, 0.0);
        let res = get_speed_call(at, lp(&mut o), cv([rx, ry, rz]));
        wv_bits(this, MOVE_SPEED, rv_bits(res.cast(), 0));
        let at = rp(this, ATTACHED_TO);
        wv_bits(this, TURN_SPEED, rv_bits(at, TURN_SPEED));
        return;
    }
    let m = matrix(this);
    let pos = pos_ptr(this);
    f.fld(rf(pos, 8));
    f.fsub_m(rf(this, ATTACH_ROTATION + 8));
    let dz = f.fstp();
    f.fld(rf(pos, 4));
    f.fsub_m(rf(this, ATTACH_ROTATION + 4));
    let dy = f.fstp();
    f.fld(rf(pos, 0));
    f.fsub_m(rf(this, ATTACH_ROTATION));
    f.fld(dy);
    f.fld(dz);
    // +0x10C keeps the previous position in this mode
    wv_bits(this, ATTACH_ROTATION, rv_bits(pos_ptr(this), 0));
    f.fld_st(2);
    f.fmul_st(3);
    f.fld(dy);
    f.fmul_m(dy);
    f.faddp(1);
    f.fld(dz);
    f.fmul_m(dz);
    f.faddp(1);
    let len2 = f.fst();
    f.fcomp_m(cf(k::K_858B90));
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fstp_st(0);
        f.fld(len2);
        f.fsqrt();
        f.fdivr_m(cf(k::K_858CA0));
        let sc = f.fst();
        f.fmulp(1);
        f.fld(sc);
        f.fmul_m(dy);
        f.fld(sc);
        f.fmul_m(dz);
    }
    let ts = sa_core::timer::TIME_STEP.get();
    f.fld(cf(k::K_858624));
    f.fcomp_m(ts);
    if f.jne(0x41) {
        f.fld(ts);
    } else {
        f.fld(cf(k::K_858624));
    }
    f.fld(cf(k::K_858624));
    f.fdiv_st(1);
    let inv = f.fstp();
    f.fstp_st(0);
    f.fmul_m(inv);
    let vz = f.fstp();
    f.fmul_m(inv);
    let vy = f.fstp();
    f.fld(inv);
    f.fmul_st(1);
    let vx = f.fstp();
    f.fstp_st(0);
    f.fld(vz);
    f.fsub_m(rf(this, MOVE_SPEED + 8));
    f.fld(vy);
    f.fsub_m(rf(this, MOVE_SPEED + 4));
    f.fld(vx);
    f.fsub_m(rf(this, MOVE_SPEED));
    let dvx = f.fstp();
    let dvy = f.fstp();
    let dvz = f.fstp();
    f.fld(rf(this, MASS));
    f.fchs();
    f.fld(dvx);
    f.fmul_st(1);
    wf(this, MOVE_SPEED, vx);
    wf(this, MOVE_SPEED + 4, vy);
    let fx = f.fstp();
    f.fld(dvy);
    f.fmul_st(1);
    wf(this, MOVE_SPEED + 8, vz);
    let fy = f.fstp();
    f.fld(dvz);
    f.fmul_st(1);
    let fz = f.fstp();
    f.fstp_st(0);
    let mut co = [0f32; 3];
    multiply3x3(lp(&mut co).cast(), m, this.byte_add(CENTRE_OF_MASS).cast());
    apply_force_call(this, cv([fx, fy, fz]), cv(co), 1);
    let at = rp(this, ATTACHED_TO);
    let t = etype(at);
    if t != ty::VEHICLE && t != ty::OBJECT {
        return;
    }
    if rb(at, EFLAGS) & 1 != 0 && rb(at, PFLAGS) & 4 == 0 {
        let apos = pos_ptr(at);
        let tpos = pos_ptr(this);
        f.fld(fz);
        f.fchs();
        f.fld(fy);
        f.fchs();
        f.fld(fx);
        f.fchs();
        let nfx = f.fstp();
        let nfy = f.fstp();
        let nfz = f.fstp();
        let mut s1 = [0f32; 3];
        let a = vector_add(lp(&mut s1).cast(), lp(&mut co).cast(), tpos);
        let mut p = [0f32; 3];
        let pr = vector_sub(lp(&mut p).cast(), a, apos);
        apply_force_call(at, cv([nfx, nfy, nfz]), cv(rv(pr, 0)), 1);
    }
    push_heading(f, rp(this, ATTACHED_TO));
    push_heading(f, this);
    f.fsubp(1);
    f.fcom_m(cf(k::K_858CB8));
    if !f.jne(0x41) {
        f.fsub_m(cf(k::K_858CBC));
    } else {
        f.fcom_m(cf(k::K_858CC0));
        if !f.jp(5) {
            f.fadd_m(cf(k::K_858CBC));
        }
    }
    let lim = G_8CD7E0.get();
    f.fcom_m(lim);
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fld(lim);
    } else {
        f.fld(lim);
        f.fchs();
        let nl = f.fstp();
        f.fcom_m(nl);
        if !f.jp(5) {
            f.fstp_st(0);
            f.fld(nl);
        }
    }
    f.fmul_m(G_8CD7DC.get());
    f.fmul_m(rf(this, MASS));
    f.fadd_m(rf(this, TURN_SPEED + 8));
    wf(this, TURN_SPEED + 8, f.fstp());
}

/// The collision mass of `ApplyFriction`-other kept on the x87 stack (`1/((c.x²+c.y²)+c.z²)/turnMass
/// + 1/mass)`, `fstp st(3)` and two pops leave it in st0 unrounded).
fn push_collision_mass(f: &mut Fpu, e: Ent, c: [f32; 3]) {
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
    f.fdiv_m(rf(e, TURN_MASS));
    f.fld(cf(k::K_858624));
    f.fdiv_m(rf(e, MASS));
    f.faddp(1);
    f.fdivr_m(cf(k::K_858624));
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
}

/// `p - pos` of `e` (`m_matrix->pos`, read through the matrix without a null check), each difference
/// stored as f32.
fn rel_point(f: &mut Fpu, p: Ent, e: Ent) -> [f32; 3] {
    let pos = matrix(e).byte_add(0x30);
    let mut r = [0f32; 3];
    for (i, r) in r.iter_mut().enumerate() {
        f.fld(rf(p, 4 * i as isize));
        f.fsub_m(rf(pos, 4 * i as isize));
        *r = f.fstp();
    }
    r
}

/// The tangential parts of `ApplyFriction`-other's cases A/B/C: with `s1` (st1) and `s2` (st0) on the
/// stack, `a = va - s1*n` (x, y stored from `fsub st(i)`, z through the stored `s1*n.z`) and `b = vb -
/// s2*n` (x, y through the stored `s2*n.x/y`, z through st(3)) left on the stack as bz, by, bx,
/// s2*n.z. Returns `a`.
fn fric_tangents(f: &mut Fpu, n: [f32; 3], va: [f32; 3], vb: [f32; 3]) -> [f32; 3] {
    f.fld_st(1);
    f.fmul_m(n[0]);
    f.fld_st(2);
    f.fmul_m(n[1]);
    f.fxch(3);
    f.fmul_m(n[2]);
    let s1nz = f.fstp();
    f.fld(va[0]);
    f.fsub_st(1);
    let ax = f.fstp();
    f.fld(va[1]);
    f.fsub_st(3);
    let ay = f.fstp();
    f.fstp_st(2);
    f.fstp_st(1);
    f.fld(va[2]);
    f.fsub_m(s1nz);
    let az = f.fstp();
    f.fld_st(0);
    f.fmul_m(n[0]);
    let t0 = f.fstp();
    f.fld_st(0);
    f.fmul_m(n[1]);
    let t1 = f.fstp();
    f.fmul_m(n[2]);
    f.fld(vb[0]);
    f.fsub_m(t0);
    f.fld(vb[1]);
    f.fsub_m(t1);
    f.fld(vb[2]);
    f.fsub_st(3);
    [ax, ay, az]
}

/// `|b|` from bz, by, bx, s2*n.z on the stack in the A/C order (`(bx² + bz²) + by²`), the stack
/// emptied; the f32 store.
fn fric_lb_ac(f: &mut Fpu) -> f32 {
    f.fld_st(2);
    f.fmul_st(3);
    f.fld_st(1);
    f.fmul_st(2);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fsqrt();
    f.fstp_st(4);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fstp()
}

/// `imp1 = max(imp1, -lim)` (`fld imp1; fcomp st(1)`, the f32 `-lim` stored over `imp1`) with `-lim`
/// on st0, then `st0 = min(st0, lim)` for the second impulse.
fn fric_clamp(f: &mut Fpu, imp1: &mut f32, lim: f32) {
    f.fld(*imp1);
    f.fcomp_st(1);
    if f.jp(5) {
        f.fstp_st(0);
    } else {
        *imp1 = f.fstp();
    }
    f.fcom_m(lim);
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fld(lim);
    }
}

/// `ua * imp1` (f32 `imp1`) and `ua * st0` (the unrounded second impulse, popped).
fn fric_forces(f: &mut Fpu, ua: [f32; 3], imp1: f32) -> ([f32; 3], [f32; 3]) {
    let mut f1 = [0f32; 3];
    for (i, v) in f1.iter_mut().enumerate() {
        f.fld(ua[i]);
        f.fmul_m(imp1);
        *v = f.fstp();
    }
    let mut f2 = [0f32; 3];
    for (i, v) in f2.iter_mut().enumerate() {
        f.fld(ua[i]);
        f.fmul_st(1);
        *v = f.fstp();
    }
    f.fstp_st(0);
    (f1, f2)
}

/// `CPhysical::ApplyFriction(CPhysical* other, float adhesiveLimit, CColPoint& cp)` (0x545980,
/// `ret 0xC`, AL).
///
/// Four cases by `bDisableTurnForce` (physical flag 0x10) of this / other. Each splits both contact
/// velocities into the normal part (`s*n`) and the tangential rest `a` (this) / `b` (other), `la =
/// |a|`, `lb = |b|`, `ua = a/la` (this' tangent direction is used for both forces, original quirk),
/// the common tangential speed `v = (m_b*lb + m_a*la)/(m_b + m_a)` with the masses or collision
/// masses; `la - v <= 0` returns false. `imp1 = (v - la)*m_a` is raised to at least `-lim`, `imp2 =
/// (v - lb)*m_b`.
/// - both flagged: move speeds, the masses; `lim = timeStep*limit`, the `max` test done twice, `imp2`
///   unclamped; `ApplyFrictionMoveForce(ua*imp1)` on this and `(ua*imp2)` on other.
/// - this only: other must not be a vehicle (else false); other's `GetSpeed(cp - pos)`, other's
///   collision mass (unrounded) of `(r - com) x ua`; `lim = timeStep*limit` stored into the `limit`
///   argument slot; `imp2 <= lim`; this `ApplyFrictionMoveForce`, other (unless physical flag 4)
///   `ApplyFrictionForce(ua*imp2, r)`.
/// - other only: the mirror (this not a vehicle); this' collision mass; this (unless physical flag
///   4) `ApplyFrictionForce(ua*imp1, r)`, other `ApplyFrictionMoveForce(ua*imp2)`.
/// - neither: both `GetSpeed`s and collision masses (this' stored as f32); the `limit` slot is
///   scaled by G_8CD7C4 when `|s_this| < G_8CD7CC*G_8CD7C8`, and `lim = limit` (no timeStep); each
///   side without physical flag 4 gets `ApplyFrictionForce` at its own `r`.
#[hook(addr = 0x545980, conv = thiscall, name = "CPhysical::ApplyFriction-other", expect = "83 EC 7C 53 55 56 8B F1",
       this = "CPhysical", unverified = "mvpf-phys-1", route = "L2", hazards = "stack-residue,x87-transcendental",
       recs = "grove-walk,driving",
       note = "writes timeStep*limit (or limit*G_8CD7C4) into its own limit argument slot; fsqrt results feed the impulses")]
pub fn apply_friction_other(this: Ent, other: Ent, limit: f32, cp: Ent) -> u8 {
    // the `limit` argument slot (rewritten by cases B/C/D)
    let mut limit = limit;
    let n = rv(cp, 0x10);
    let mut f = Fpu::new();
    let ts = sa_core::timer::TIME_STEP.get();
    let tf = pflags(this) & 0x10 != 0;
    let of = rb(other, PFLAGS) & 0x10 != 0;
    let m1 = rf(this, MASS);
    if tf && of {
        let tm = rv(this, MOVE_SPEED);
        let om = rv(other, MOVE_SPEED);
        f.fld(tm[2]);
        f.fmul_m(n[2]);
        f.fld(tm[1]);
        f.fmul_m(n[1]);
        f.faddp(1);
        f.fld(tm[0]);
        f.fmul_m(n[0]);
        f.faddp(1);
        f.fld(om[2]);
        f.fmul_m(n[2]);
        f.fld(om[1]);
        f.fmul_m(n[1]);
        f.faddp(1);
        f.fld(om[0]);
        f.fmul_m(n[0]);
        f.faddp(1);
        let a = fric_tangents(&mut f, n, tm, om);
        f.fld(a[0]);
        f.fmul_m(a[0]);
        f.fld(a[2]);
        f.fmul_m(a[2]);
        f.faddp(1);
        f.fld(a[1]);
        f.fmul_m(a[1]);
        f.faddp(1);
        f.fsqrt();
        let la = f.fstp();
        let lb = fric_lb_ac(&mut f);
        let m2 = rf(other, MASS);
        f.fld(cf(k::K_858624));
        f.fdiv_m(la);
        f.fld(a[0]);
        f.fmul_st(1);
        let uax = f.fstp();
        f.fld(a[1]);
        f.fmul_st(1);
        let uay = f.fstp();
        f.fmul_m(a[2]);
        f.fld(m1);
        f.fld(m2);
        f.fmul_m(lb);
        f.fld_st(1);
        f.fmul_m(la);
        f.faddp(1);
        f.fld(m2);
        f.fadd_st(2);
        f.fdivp(1);
        let v = f.fstp();
        f.fld(la);
        f.fsub_m(v);
        f.fcomp_m(cf(k::K_858B50));
        if f.jne(0x41) {
            return 0;
        }
        f.fld(v);
        f.fsub_m(la);
        f.fmulp(1);
        f.fld(v);
        f.fsub_m(lb);
        f.fmul_m(m2);
        let imp2 = f.fstp();
        f.fld(ts);
        f.fmul_m(limit);
        f.fchs();
        let lim = f.fstp();
        for _ in 0..2 {
            f.fcom_m(lim);
            if !f.jp(5) {
                f.fstp_st(0);
                f.fld(lim);
            }
        }
        let mut f1 = [0f32; 3];
        f.fld(uax);
        f.fmul_st(1);
        f1[0] = f.fstp();
        f.fld(uay);
        f.fmul_st(1);
        f1[1] = f.fstp();
        f.fld_st(1);
        f.fmul_st(1);
        f1[2] = f.fstp();
        f.fstp_st(0);
        let mut f2 = [0f32; 3];
        f.fld(uax);
        f.fmul_m(imp2);
        f2[0] = f.fstp();
        f.fld(uay);
        f.fmul_m(imp2);
        f2[1] = f.fstp();
        f.fmul_m(imp2);
        f2[2] = f.fstp();
        apply_friction_move_force_call(this, cv(f1));
        apply_friction_move_force_call(other, cv(f2));
        return 1;
    }
    if tf {
        // this flagged, other not
        if etype(other) == ty::VEHICLE {
            return 0;
        }
        let r = rel_point(&mut f, cp, other);
        let mut vo = CVector::new(0.0, 0.0, 0.0);
        get_speed_call(other, lp(&mut vo), cv(r));
        let vo = va(vo);
        let tm = rv(this, MOVE_SPEED);
        f.fld(n[2]);
        f.fmul_m(tm[2]);
        f.fld(n[1]);
        f.fmul_m(tm[1]);
        f.faddp(1);
        f.fld(tm[0]);
        f.fmul_m(n[0]);
        f.faddp(1);
        f.fld(vo[1]);
        f.fmul_m(n[1]);
        f.fld(vo[2]);
        f.fmul_m(n[2]);
        f.faddp(1);
        f.fld(vo[0]);
        f.fmul_m(n[0]);
        f.faddp(1);
        let a = fric_tangents(&mut f, n, tm, vo);
        f.fld(a[2]);
        f.fmul_m(a[2]);
        f.fld(a[1]);
        f.fmul_m(a[1]);
        f.faddp(1);
        f.fld(a[0]);
        f.fmul_m(a[0]);
        f.faddp(1);
        f.fsqrt();
        let la = f.fstp();
        f.fld_st(0);
        f.fmul_st(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fld_st(3);
        f.fmul_st(4);
        f.faddp(1);
        f.fsqrt();
        f.fstp_st(4);
        f.fstp_st(0);
        f.fstp_st(0);
        f.fstp_st(0);
        let lb = f.fstp();
        f.fld(cf(k::K_858624));
        f.fdiv_m(la);
        let mut ua = [0f32; 3];
        for (i, u) in ua.iter_mut().enumerate() {
            f.fld(a[i]);
            f.fmul_st(1);
            *u = f.fstp();
        }
        f.fstp_st(0);
        let mut co = [0f32; 3];
        multiply3x3(lp(&mut co).cast(), matrix(other), other.byte_add(CENTRE_OF_MASS).cast());
        let mut d = [0f32; 3];
        for (i, d) in d.iter_mut().enumerate() {
            f.fld(r[i]);
            f.fsub_m(co[i]);
            *d = f.fstp();
        }
        let mut uav = cv(ua);
        let mut dv = cv(d);
        let mut xo = CVector::new(0.0, 0.0, 0.0);
        let c = rv(cross_product(lp(&mut xo), lp(&mut dv), lp(&mut uav)).cast(), 0);
        push_collision_mass(&mut f, other, c);
        f.fld(lb);
        f.fmul_st(1);
        f.fld(m1);
        f.fmul_m(la);
        f.faddp(1);
        f.fld_st(1);
        f.fadd_m(m1);
        f.fdivp(1);
        f.fld(la);
        f.fsub_st(1);
        f.fcomp_m(cf(k::K_858B50));
        if f.jne(0x41) {
            return 0;
        }
        f.fld(la);
        f.fsubr_st(1);
        f.fmul_m(m1);
        let mut imp1 = f.fstp();
        f.fsub_m(lb);
        f.fmulp(1);
        f.fld(ts);
        f.fmul_m(limit);
        limit = f.fst();
        f.fchs();
        fric_clamp(&mut f, &mut imp1, limit);
        let (f1, f2) = fric_forces(&mut f, ua, imp1);
        apply_friction_move_force_call(this, cv(f1));
        if rb(other, PFLAGS) & 4 == 0 {
            apply_friction_force_call(other, cv(f2), cv(r));
        }
        return 1;
    }
    if of {
        // other flagged, this not
        if etype(this) == ty::VEHICLE {
            return 0;
        }
        let r = rel_point(&mut f, cp, this);
        let mut vt = CVector::new(0.0, 0.0, 0.0);
        get_speed_call(this, lp(&mut vt), cv(r));
        let vt = va(vt);
        let om = rv(other, MOVE_SPEED);
        f.fld(vt[1]);
        f.fmul_m(n[1]);
        f.fld(vt[2]);
        f.fmul_m(n[2]);
        f.faddp(1);
        f.fld(vt[0]);
        f.fmul_m(n[0]);
        f.faddp(1);
        f.fld(om[2]);
        f.fmul_m(n[2]);
        f.fld(om[1]);
        f.fmul_m(n[1]);
        f.faddp(1);
        f.fld(n[0]);
        f.fmul_m(om[0]);
        f.faddp(1);
        let a = fric_tangents(&mut f, n, vt, om);
        f.fld(a[0]);
        f.fmul_m(a[0]);
        f.fld(a[2]);
        f.fmul_m(a[2]);
        f.faddp(1);
        f.fld(a[1]);
        f.fmul_m(a[1]);
        f.faddp(1);
        f.fsqrt();
        let la = f.fstp();
        let lb = fric_lb_ac(&mut f);
        f.fld(cf(k::K_858624));
        f.fdiv_m(la);
        let mut ua = [0f32; 3];
        for (i, u) in ua.iter_mut().enumerate() {
            f.fld(a[i]);
            f.fmul_st(1);
            *u = f.fstp();
        }
        f.fstp_st(0);
        let mut co = [0f32; 3];
        multiply3x3(lp(&mut co).cast(), matrix(this), this.byte_add(CENTRE_OF_MASS).cast());
        let mut d = [0f32; 3];
        for (i, d) in d.iter_mut().enumerate() {
            f.fld(r[i]);
            f.fsub_m(co[i]);
            *d = f.fstp();
        }
        let mut uav = cv(ua);
        let mut dv = cv(d);
        let mut xo = CVector::new(0.0, 0.0, 0.0);
        let c = rv(cross_product(lp(&mut xo), lp(&mut dv), lp(&mut uav)).cast(), 0);
        push_collision_mass(&mut f, this, c);
        let m2 = rf(other, MASS);
        f.fld(m2);
        f.fmul_m(lb);
        f.fld_st(1);
        f.fmul_m(la);
        f.faddp(1);
        f.fld(m2);
        f.fadd_st(2);
        f.fdivp(1);
        let v = f.fstp();
        f.fld(la);
        f.fsub_m(v);
        f.fcomp_m(cf(k::K_858B50));
        if f.jne(0x41) {
            return 0;
        }
        f.fld(v);
        f.fsub_m(la);
        f.fmul_st(1);
        let mut imp1 = f.fstp();
        f.fstp_st(0);
        f.fld(v);
        f.fsub_m(lb);
        f.fmul_m(m2);
        f.fld(ts);
        f.fmul_m(limit);
        limit = f.fst();
        f.fchs();
        fric_clamp(&mut f, &mut imp1, limit);
        let (f1, f2) = fric_forces(&mut f, ua, imp1);
        if rb(this, PFLAGS) & 4 == 0 {
            apply_friction_force_call(this, cv(f1), cv(r));
        }
        apply_friction_move_force_call(other, cv(f2));
        return 1;
    }
    // neither flagged
    let rt = rel_point(&mut f, cp, this);
    let ro = rel_point(&mut f, cp, other);
    let mut vto = CVector::new(0.0, 0.0, 0.0);
    get_speed_call(this, lp(&mut vto), cv(rt));
    let vt = va(vto);
    let mut voo = CVector::new(0.0, 0.0, 0.0);
    get_speed_call(other, lp(&mut voo), cv(ro));
    let vo = va(voo);
    f.fld(vt[1]);
    f.fmul_m(n[1]);
    f.fld(vt[2]);
    f.fmul_m(n[2]);
    f.faddp(1);
    f.fld(vt[0]);
    f.fmul_m(n[0]);
    f.faddp(1);
    f.fld(vo[1]);
    f.fmul_m(n[1]);
    f.fld(vo[2]);
    f.fmul_m(n[2]);
    f.faddp(1);
    f.fld(vo[0]);
    f.fmul_m(n[0]);
    f.faddp(1);
    let s2 = f.fstp();
    f.fld_st(0);
    f.fabs();
    f.fld(G_8CD7CC.get());
    f.fmul_m(G_8CD7C8.get());
    f.fcompp();
    if !f.jne(0x41) {
        f.fld(G_8CD7C4.get());
        f.fmul_m(limit);
        limit = f.fstp();
    }
    // a = vt - s1*n: x stored, y and z kept on the stack
    f.fld_st(0);
    f.fmul_m(n[0]);
    f.fld_st(1);
    f.fmul_m(n[1]);
    f.fxch(2);
    f.fmul_m(n[2]);
    let s1nz = f.fstp();
    f.fld(vt[0]);
    f.fsub_st(1);
    let ax = f.fstp();
    f.fld(vt[1]);
    f.fsub_st(2);
    f.fxch(2);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fld(vt[2]);
    f.fsub_m(s1nz);
    // b = vo - s2*n: x stored, y and z kept
    f.fld(s2);
    f.fmul_m(n[0]);
    f.fld(s2);
    f.fmul_m(n[1]);
    f.fld(s2);
    f.fmul_m(n[2]);
    let s2nz = f.fstp();
    f.fld(vo[0]);
    f.fsub_st(2);
    let bx = f.fstp();
    f.fld(vo[1]);
    f.fsub_st(1);
    f.fstp_st(2);
    f.fstp_st(0);
    f.fld(vo[2]);
    f.fsub_m(s2nz);
    // stack: bz, by, az, ay
    f.fld(ax);
    f.fmul_m(ax);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fld_st(4);
    f.fmul_st(5);
    f.faddp(1);
    f.fsqrt();
    let la = f.fstp();
    f.fld(bx);
    f.fmul_m(bx);
    f.fld_st(1);
    f.fmul_st(2);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fsqrt();
    f.fstp_st(2);
    f.fstp_st(0);
    let lb = f.fstp();
    // stack: az, ay
    f.fld(cf(k::K_858624));
    f.fdiv_m(la);
    let mut ua = [0f32; 3];
    f.fld(ax);
    f.fmul_st(1);
    ua[0] = f.fstp();
    f.fxch(2);
    f.fmul_st(2);
    ua[1] = f.fstp();
    f.fmul_st(1);
    ua[2] = f.fstp();
    f.fstp_st(0);
    let mut ct = [0f32; 3];
    multiply3x3(lp(&mut ct).cast(), matrix(this), this.byte_add(CENTRE_OF_MASS).cast());
    let mut d = [0f32; 3];
    for (i, d) in d.iter_mut().enumerate() {
        f.fld(rt[i]);
        f.fsub_m(ct[i]);
        *d = f.fstp();
    }
    let mut uav = cv(ua);
    let mut dv = cv(d);
    let mut xo = CVector::new(0.0, 0.0, 0.0);
    let c = rv(cross_product(lp(&mut xo), lp(&mut dv), lp(&mut uav)).cast(), 0);
    push_collision_mass(&mut f, this, c);
    let mt = f.fstp();
    let mut co2 = [0f32; 3];
    multiply3x3(lp(&mut co2).cast(), matrix(other), other.byte_add(CENTRE_OF_MASS).cast());
    let mut d2 = [0f32; 3];
    for (i, d) in d2.iter_mut().enumerate() {
        f.fld(ro[i]);
        f.fsub_m(co2[i]);
        *d = f.fstp();
    }
    let mut d2v = cv(d2);
    let mut xo2 = CVector::new(0.0, 0.0, 0.0);
    let c2 = rv(cross_product(lp(&mut xo2), lp(&mut d2v), lp(&mut uav)).cast(), 0);
    push_collision_mass(&mut f, other, c2);
    f.fld(lb);
    f.fmul_st(1);
    f.fld(mt);
    f.fmul_m(la);
    f.faddp(1);
    f.fld_st(1);
    f.fadd_m(mt);
    f.fdivp(1);
    f.fld(la);
    f.fsub_st(1);
    f.fcomp_m(cf(k::K_858B50));
    if f.jne(0x41) {
        return 0;
    }
    f.fld(la);
    f.fsubr_st(1);
    f.fmul_m(mt);
    let mut imp1 = f.fstp();
    f.fsub_m(lb);
    f.fmulp(1);
    f.fld(limit);
    f.fchs();
    fric_clamp(&mut f, &mut imp1, limit);
    let (f1, f2) = fric_forces(&mut f, ua, imp1);
    if rb(this, PFLAGS) & 4 == 0 {
        apply_friction_force_call(this, cv(f1), cv(rt));
    }
    if rb(other, PFLAGS) & 4 == 0 {
        apply_friction_force_call(other, cv(f2), cv(ro));
    }
    1
}

pub const HOOKS: &[HookReg] = &[HOOK_APPLY_COLLISION_ALT, HOOK_POSITION_ATTACHED_ENTITY, HOOK_APPLY_FRICTION_OTHER];
