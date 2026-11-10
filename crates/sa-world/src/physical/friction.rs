//! `CPhysical::ApplyFriction(float, CColPoint&)` and `ApplyFriction(CPhysical*, float, CColPoint&)`
//! (PORT-PHYS), ported instruction by instruction from the 1.0 US bytes with the [`Fpu`] stack
//! machine (PC = 24). gta-reversed `Entity/Physical.cpp` was a reading aid; its spark spread
//! `GetRandomNumberInRange(-0.2f, 0.2f)` differs from the bytes' `t*0.4f - 0.2f` (gr-dev
//! Physical.cpp:1755, keep-original).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::forces::{apply_friction_force_call, get_speed_call};
use super::shared::*;
use crate::collision::fpu::Fpu;

orig! {
    /// `SurfaceInfos_c::GetFrictionEffect(uint32 surface)` (0x55E650, thiscall on `g_surfaceInfos`)
    pub fn get_friction_effect(si: Ent, surface: u32) -> i32 = thiscall 0x55E650 => unported;
    /// `CVector::DotProduct(const CVector*, const CVector*)` (0x40FDB0, ST0)
    pub fn dot_product(a: Ent, b: Ent) -> f64 = cdecl 0x40FDB0 => unported;
    /// `operator*(CVector* out, float s, const CVector& v)` (0x40FE90, cdecl): `out = s*v`
    pub fn scale_vector(out: GPtr<CVector>, s: f32, v: GPtr<CVector>) -> GPtr<CVector> = cdecl 0x40FE90 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00, thiscall)
    pub fn vector_add_assign(v: GPtr<CVector>, o: GPtr<CVector>) -> GPtr<CVector> = thiscall 0x411A00 => unported;
    /// `CVector::Normalise()` (0x59C910)
    pub fn normalise(v: GPtr<CVector>) = thiscall 0x59C910 => crate::collision::data::normalise_port;
    /// `Fx_c::AddSparks(CVector& origin, CVector& dir, float force, int32 amount, CVector across,
    /// eSparkType, float spread, float life)` (0x49F040, thiscall on `g_fx` 0xA9AE00; every argument a
    /// dword)
    pub fn fx_add_sparks(fx: Ent, origin: Ent, dir: Ent, force: u32, amount: u32, ax: u32, ay: u32, az: u32,
        kind: u32, spread: u32, life: u32) = thiscall 0x49F040 => unported;
}

/// `g_surfaceInfos` (0xB79538)
pub const SURFACE_INFOS: usize = 0xB79538;
/// `g_fx` (0xA9AE00)
pub const G_FX: usize = 0xA9AE00;

/// The collision mass of `c` (registers), times `x` (memory operand), negated: `-(m * x)`, left in
/// st0 (ApplyFriction's `fmul speed; fchs`).
fn neg_mass_times(f: &mut Fpu, this: Ent, c: [f32; 3], x: f32) {
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
    f.fmul_m(x);
    f.fchs();
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
}

/// `CPhysical::ApplyFriction(float friction, CColPoint& cp)` (0x5454C0, `ret 8`, AL).
///
/// Nothing (false) with `bDisableCollisionForce`. `bDisableTurnForce`: the tangential move speed
/// `u = ms - (ms . n) n` (registers), `speed = |u|` (f32); unless `speed > 0` false; `v =
/// max(-speed, -(timeStep/mass*friction))`; `frictionMove.xy += (u/speed)*v` (z untouched).
/// Otherwise the same with the point speed `GetSpeed(cp.point - pos)` (u.y stored as f32), `X =
/// max(-(m*speed), -friction)` with the collision mass m of `(r - com') x dir`, `ApplyFrictionForce(dir
/// * X, r)`; above speed 0.1 on a surface with a friction effect (and the other surface's effect 1,
/// or a vehicle; a type-10 vehicle with +0x460 only when `|n . right| >= 0.8667f`) eight sparks
/// along `normalise(n x moveSpeed) * (rand()/32767*0.4f - 0.2f)` from the point.
#[hook(addr = 0x5454C0, conv = thiscall, name = "CPhysical::ApplyFriction-self", expect = "83 EC 54 53 55", this = "CPhysical")]
pub fn apply_friction(this: Ent, friction: f32, cp: Ent) -> u8 {
    let fl = pflags(this);
    if fl & 4 != 0 {
        return 0;
    }
    let n = cp.byte_add(0x10);
    let mut f = Fpu::new();
    if fl & 0x10 != 0 {
        f.fld(rf(this, MOVE_SPEED + 8));
        f.fmul_m(rf(n, 8));
        f.fld(rf(this, MOVE_SPEED + 4));
        f.fmul_m(rf(n, 4));
        f.faddp(1);
        f.fld(rf(this, MOVE_SPEED));
        f.fmul_m(rf(n, 0));
        f.faddp(1);
        f.fld_st(0);
        f.fmul_m(rf(n, 0));
        f.fld_st(1);
        f.fmul_m(rf(n, 4));
        let ty = f.fstp();
        f.fxch(1);
        f.fmul_m(rf(n, 8));
        let tz = f.fstp();
        f.fsubr_m(rf(this, MOVE_SPEED));
        f.fld(rf(this, MOVE_SPEED + 4));
        f.fsub_m(ty);
        f.fld(rf(this, MOVE_SPEED + 8));
        f.fsub_m(tz);
        f.fld_st(0);
        f.fmulp(1);
        f.fld_st(1);
        f.fmul_st(2);
        f.faddp(1);
        f.fld_st(2);
        f.fmul_st(3);
        f.faddp(1);
        f.fsqrt();
        let speed = f.fst();
        f.fcomp_m(cf(k::K_858B50));
        if f.jne(0x41) {
            return 0;
        }
        f.fld(cf(k::K_858624));
        f.fdiv_m(speed);
        f.fld_st(0);
        f.fmul_st(3);
        let dx = f.fstp();
        f.fxch(1);
        f.fmul_st(1);
        f.fstp_st(2);
        f.fstp_st(0);
        f.fld(speed);
        f.fchs();
        f.fld(sa_core::timer::TIME_STEP.get());
        f.fdiv_m(rf(this, MASS));
        f.fmul_m(friction);
        f.fchs();
        let fr = f.fstp();
        f.fcom_m(fr);
        if !f.jp(5) {
            f.fstp_st(0);
            f.fld(fr);
        }
        f.fld(dx);
        f.fmul_st(1);
        let ax = f.fstp();
        f.fxch(1);
        f.fmul_st(1);
        f.fxch(1);
        f.fstp_st(0);
        f.fld(ax);
        f.fadd_m(rf(this, FRICTION_MOVE));
        wf(this, FRICTION_MOVE, f.fstp());
        f.fadd_m(rf(this, FRICTION_MOVE + 4));
        wf(this, FRICTION_MOVE + 4, f.fstp());
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
    f.fld(v[1]);
    f.fmul_m(nl[1]);
    f.fld(v[2]);
    f.fmul_m(nl[2]);
    f.faddp(1);
    f.fld(v[0]);
    f.fmul_m(nl[0]);
    f.faddp(1);
    f.fld_st(0);
    f.fmul_m(nl[0]);
    f.fld_st(1);
    f.fmul_m(nl[1]);
    let ty = f.fstp();
    f.fxch(1);
    f.fmul_m(nl[2]);
    let tz = f.fstp();
    f.fsubr_m(v[0]);
    f.fld(v[1]);
    f.fsub_m(ty);
    let uy = f.fstp();
    f.fld(v[2]);
    f.fsub_m(tz);
    f.fld_st(0);
    f.fmul_st(1);
    f.fld(uy);
    f.fmul_m(uy);
    f.faddp(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fsqrt();
    let speed = f.fst();
    f.fcomp_m(cf(k::K_858B50));
    if f.jne(0x41) {
        return 0;
    }
    // st0 = uz, st1 = ux
    f.fld(cf(k::K_858624));
    f.fdiv_m(speed);
    f.fld_st(0);
    f.fmul_st(3);
    let wx = f.fstp();
    f.fld(uy);
    f.fmul_st(1);
    let wy = f.fstp();
    f.fxch(1);
    f.fmul_st(1);
    let wz = f.fstp();
    f.fstp_st(0);
    f.fstp_st(0);
    let mut w = CVector::new(wx, wy, wz);
    let mut co = CVector::new(0.0, 0.0, 0.0);
    let com = rv(multiply3x3(lp(&mut co), matrix(this), this.byte_add(CENTRE_OF_MASS).cast()).cast(), 0);
    let mut d = [0f32; 3];
    for (i, d) in d.iter_mut().enumerate() {
        f.fld(r[i]);
        f.fsub_m(com[i]);
        *d = f.fstp();
    }
    let mut dv = cv(d);
    let mut xo = CVector::new(0.0, 0.0, 0.0);
    let c = rv(cross_product(lp(&mut xo), lp(&mut dv), lp(&mut w)).cast(), 0);
    neg_mass_times(&mut f, this, c, speed);
    f.fld(friction);
    f.fchs();
    let nf = f.fstp();
    f.fcom_m(nf);
    if !f.jp(5) {
        f.fstp_st(0);
        f.fld(nf);
    }
    let mut fv = [0f32; 3];
    for (i, wi) in [w.x, w.y, w.z].iter().enumerate() {
        f.fld(*wi);
        f.fmul_st(1);
        fv[i] = f.fstp();
    }
    f.fstp_st(0);
    apply_friction_force_call(this, cv(fv), cv(r));
    f.fld(speed);
    f.fcomp_m(cf(k::K_858B1C));
    if f.jne(0x41) {
        return 1;
    }
    let si: Ent = GPtr::from_addr(SURFACE_INFOS);
    if get_friction_effect(si, rb(cp, 0x23) as u32) == 0 {
        return 1;
    }
    if get_friction_effect(si, rb(cp, 0x20) as u32) != 1 && etype(this) != ty::VEHICLE {
        return 1;
    }
    if etype(this) == ty::VEHICLE && ru(this, 0x594) == 10 && ru(this, 0x460) != 0 {
        f.push(X87::from_f64_exact(dot_product(n, matrix(this))));
        f.fabs();
        f.fcomp_m(cf(k::K_863C14));
        if !f.jp(5) {
            return 1;
        }
    }
    f.fld(speed);
    f.fmul_m(cf(k::K_858C84));
    let s4 = f.fstp();
    let mut across = CVector::new(0.0, 0.0, 0.0);
    scale_vector(lp(&mut across), s4, lp(&mut w));
    f.fld(speed);
    f.fmul_m(cf(k::K_86321C));
    let force = f.fstp();
    let mut dir = w;
    let mut tn = CVector::new(0.0, 0.0, 0.0);
    let tnp = scale_vector(lp(&mut tn), cf(k::K_858B1C), n.cast());
    vector_add_assign(lp(&mut dir), tnp);
    let mut cr = CVector::new(0.0, 0.0, 0.0);
    cross_product(lp(&mut cr), n.cast(), this.byte_add(MOVE_SPEED).cast());
    normalise(lp(&mut cr));
    let across = [across.x.to_bits(), across.y.to_bits(), across.z.to_bits()];
    for _ in 0..8 {
        let rnd = sa_core::orig::at_site(0x5458C5, rand);
        f.fild(rnd);
        f.fmul_m(cf(k::K_858C7C));
        f.fmul_m(cf(k::K_858EE8));
        f.fsub_m(cf(k::K_858CC4));
        f.fld(cr.x);
        f.fmul_st(1);
        f.fld(cr.y);
        f.fmul_st(2);
        let oy = f.fstp();
        f.fld(cr.z);
        f.fmul_st(2);
        let oz = f.fstp();
        f.fadd_m(rf(cp, 0));
        let ox = f.fstp();
        f.fstp_st(0);
        f.fld(oy);
        f.fadd_m(rf(cp, 4));
        let oy = f.fstp();
        f.fld(oz);
        f.fadd_m(rf(cp, 8));
        let oz = f.fstp();
        let mut origin = CVector::new(ox, oy, oz);
        fx_add_sparks(
            GPtr::from_addr(G_FX),
            lp(&mut origin).cast(),
            lp(&mut dir).cast(),
            force.to_bits(),
            1,
            across[0],
            across[1],
            across[2],
            0,
            0x3DCC_CCCD,
            0x3F80_0000,
        );
    }
    1
}

pub const HOOKS: &[HookReg] = &[HOOK_APPLY_FRICTION];
