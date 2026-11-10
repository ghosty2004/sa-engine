//! `CPhysical` spring collisions and dampening (PORT-PHYS), ported instruction by instruction from
//! the 1.0 US bytes with the [`Fpu`] stack machine (PC = 24).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::vector::CVector;

use super::forces::{apply_force_call, get_speed_call};
use super::shared::*;
use crate::collision::fpu::Fpu;

/// `t = timeStep < 3.0f ? timeStep : 3.0f` (NaN gives 3.0f), pushed.
pub fn push_capped_time_step(f: &mut Fpu) {
    let ts = sa_core::timer::TIME_STEP.get();
    f.fld(ts);
    f.fcomp_m(cf(k::K_858B3C));
    if !f.jp(5) {
        f.fld(ts);
    } else {
        f.fld(cf(k::K_858B3C));
    }
}

/// `ApplyForce(force, *point, true)` with the point copied as dwords.
fn apply_force_at(this: Ent, force: [f32; 3], point: Ent) {
    apply_force_call(this, cv(force), cv(rv(point, 0)), 1);
}

/// `st0 * dir` into three f32 (the last product pops st0).
fn scale_pop(f: &mut Fpu, dir: Ent) -> [f32; 3] {
    let mut v = [0f32; 3];
    f.fld_st(0);
    f.fmul_m(rf(dir, 0));
    v[0] = f.fstp();
    f.fld_st(0);
    f.fmul_m(rf(dir, 4));
    v[1] = f.fstp();
    f.fmul_m(rf(dir, 8));
    v[2] = f.fstp();
    v
}

/// `(a.y*b.y + a.z*b.z) + a.x*b.x` with the operand order of the bytes (`first` = the `fld`
/// operand of each product).
fn dot_yzx(f: &mut Fpu, a: [f32; 3], b: [f32; 3], x_first_b: bool) {
    f.fld(a[1]);
    f.fmul_m(b[1]);
    f.fld(a[2]);
    f.fmul_m(b[2]);
    f.faddp(1);
    if x_first_b {
        f.fld(b[0]);
        f.fmul_m(a[0]);
    } else {
        f.fld(a[0]);
        f.fmul_m(b[0]);
    }
    f.faddp(1);
}

/// `CPhysical::ApplySpringCollision(float level, CVector& dir, CVector& point, float springLength,
/// float bias, float& springForce)` (0x543C90, `ret 0x18`, AL = 1). `c = 1 - springLength`; unless
/// `c > 0` (or NaN) nothing; `F = ((((c*mass)*level)*0.016f)*t)*bias` (t = capped time step),
/// `springForce = F`, `ApplyForce(-F*dir, point, true)` (F in a register).
#[hook(addr = 0x543C90, conv = thiscall, name = "CPhysical::ApplySpringCollision", expect = "D9 05 24 86 85 00", this = "CPhysical")]
pub fn apply_spring_collision(this: Ent, level: f32, dir: Ent, point: Ent, len: f32, bias: f32, out: GPtr<f32>) -> u8 {
    let mut f = Fpu::new();
    f.fld(cf(k::K_858624));
    f.fsub_m(len);
    f.fcom_m(cf(k::K_858B50));
    if !f.jp(0x41) {
        return 1;
    }
    push_capped_time_step(&mut f);
    f.fxch(1);
    f.fmul_m(rf(this, MASS));
    f.fmul_m(level);
    f.fmul_m(cf(k::K_863C00));
    f.fmul_st(1);
    f.fmul_m(bias);
    f.fstp_st(1);
    out.write(f.fst());
    f.fchs();
    let v = scale_pop(&mut f, dir);
    apply_force_at(this, v, point);
    1
}

/// `CPhysical::ApplySpringCollisionAlt(float level, CVector& dir, CVector& point, float
/// springLength, float bias, CVector& normal, float& springForce)` (0x543D60, `ret 0x1C`, AL = 1).
/// As ApplySpringCollision, but `normal` is negated in place when `dir . normal > 0`, `F = ((c *
/// (t*mass)) * level * bias) * 0.016f`, `springForce = F` (times 0.75f with
/// `bMakeMassTwiceAsBig`), and the force is `springForce * normal` (the stored f32).
#[hook(addr = 0x543D60, conv = thiscall, name = "CPhysical::ApplySpringCollisionAlt", expect = "D9 05 24 86 85 00", this = "CPhysical")]
#[allow(clippy::too_many_arguments)]
pub fn apply_spring_collision_alt(
    this: Ent,
    level: f32,
    dir: Ent,
    point: Ent,
    len: f32,
    bias: f32,
    normal: Ent,
    out: GPtr<f32>,
) -> u8 {
    let mut f = Fpu::new();
    f.fld(cf(k::K_858624));
    f.fsub_m(len);
    f.fcom_m(cf(k::K_858B50));
    if !f.jp(0x41) {
        return 1;
    }
    dot_yzx(&mut f, rv(dir, 0), rv(normal, 0), false);
    f.fcomp_m(cf(k::K_858B50));
    if !f.jne(0x41) {
        for off in [0, 4, 8] {
            f.fld(rf(normal, off));
            f.fmul_m(cf(k::K_858C1C));
            wf(normal, off, f.fstp());
        }
    }
    push_capped_time_step(&mut f);
    f.fmul_m(rf(this, MASS));
    f.fmulp(1);
    f.fmul_m(level);
    f.fmul_m(bias);
    f.fmul_m(cf(k::K_863C00));
    out.write(f.fst());
    if rb(this, PFLAGS) & 1 != 0 {
        f.fmul_m(cf(k::K_858F34));
        out.write(f.fstp());
    } else {
        f.fstp_st(0);
    }
    f.fld(out.read());
    let v = scale_pop(&mut f, normal);
    apply_force_at(this, v, point);
    1
}

/// The collision mass of `c` times `imp` (registers): `(1 / ((((c.x*c.x) + c.y*c.y) + c.z*c.z) /
/// turnMass + 1/mass)) * imp`, left in st0.
fn mass_times(f: &mut Fpu, this: Ent, c: [f32; 3], imp: f32) {
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
    f.fmul_m(imp);
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
}

/// `CPhysical::ApplySpringDampening(float damping, float limit, CVector& dir, CVector& point,
/// CVector& speed)` (0x543E90, `ret 0x14`, AL = 1). `s1 = speed . dir`, `s2 = GetSpeed(point) . dir`
/// (f32 each), `x = t*damping` (doubled with `bMakeMassTwiceAsBig`) clamped to
/// `[-DAMPING_LIMIT_IN_FRAME, DAMPING_LIMIT_IN_FRAME]`, `imp = -(x*s1)` (f32); the overshoot rules of
/// the bytes (imp and imp + s2 of opposite sign: `imp = -s2` or 0); `imp *= collision mass`
/// (register), capped above at `|limit| * DAMPING_LIMIT_OF_SPRING_FORCE`; `ApplyForce(imp*dir,
/// point, true)`.
#[hook(addr = 0x543E90, conv = thiscall, name = "CPhysical::ApplySpringDampening", expect = "83 EC 28 8B 44 24 3C", this = "CPhysical")]
pub fn apply_spring_dampening(this: Ent, damping: f32, limit: f32, dir: Ent, point: Ent, speed: Ent) -> u8 {
    let mut f = Fpu::new();
    dot_yzx(&mut f, rv(speed, 0), rv(dir, 0), true);
    let s1 = f.fstp();
    let mut vo = CVector::new(0.0, 0.0, 0.0);
    let v = rv(get_speed_call(this, lp(&mut vo), cv(rv(point, 0))).cast(), 0);
    dot_yzx(&mut f, v, rv(dir, 0), true);
    let s2 = f.fstp();
    push_capped_time_step(&mut f);
    f.fmul_m(damping);
    if rb(this, PFLAGS) & 1 != 0 {
        f.fld_st(0);
        f.faddp(1);
    }
    let lim = DAMPING_LIMIT_IN_FRAME.get();
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
    f.fmul_m(s1);
    f.fchs();
    let mut imp = f.fst();
    let zero = cf(k::K_858B50);
    f.fcomp_m(zero);
    // 0 = keep, 1 = imp = 0 (0x543FA4), 2 = imp = -s2 (0x5440EE)
    // 0x5440AB: imp < 0 and imp + s2 < 0: s2 > 0 gives -s2, else 0
    let tail = |f: &mut Fpu| -> i32 {
        f.fld(imp);
        f.fcomp_m(zero);
        if f.jp(5) {
            return 0;
        }
        f.fld(imp);
        f.fadd_m(s2);
        f.fcomp_m(zero);
        if f.jp(5) {
            return 0;
        }
        f.fld(s2);
        f.fcomp_m(zero);
        if f.jne(0x41) { 1 } else { 2 }
    };
    let action = if f.jne(0x41) {
        tail(&mut f)
    } else {
        f.fld(imp);
        f.fadd_m(s2);
        f.fcomp_m(zero);
        if f.jne(0x41) {
            tail(&mut f)
        } else {
            f.fld(s2);
            f.fcomp_m(zero);
            if !f.jp(5) { 2 } else { 1 }
        }
    };
    match action {
        1 => imp = 0.0,
        2 => {
            f.fld(s2);
            f.fchs();
            imp = f.fstp();
        }
        _ => {}
    }
    let mut co = CVector::new(0.0, 0.0, 0.0);
    let com = rv(multiply3x3(lp(&mut co), matrix(this), this.byte_add(CENTRE_OF_MASS).cast()).cast(), 0);
    let mut d = [0f32; 3];
    for (i, d) in d.iter_mut().enumerate() {
        f.fld(rf(point, 4 * i as isize));
        f.fsub_m(com[i]);
        *d = f.fstp();
    }
    let mut dv = cv(d);
    let mut xo = CVector::new(0.0, 0.0, 0.0);
    let c = rv(cross_product(lp(&mut xo), lp(&mut dv), dir.cast()).cast(), 0);
    mass_times(&mut f, this, c, imp);
    f.fld(limit);
    f.fabs();
    f.fmul_m(DAMPING_LIMIT_OF_SPRING_FORCE.get());
    let cap = f.fstp();
    f.fcom_m(cap);
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fld(cap);
    }
    let v = scale_pop(&mut f, dir);
    apply_force_at(this, v, point);
    1
}

/// `CPhysical::ApplySpringDampeningOld(float damping, float limit, CVector& dir, CVector& point,
/// CVector& speed)` (0x544100, `ret 0x14`, AL = 1; spec CPhysical.md; `limit` unused).
#[hook(addr = 0x544100, conv = thiscall, name = "CPhysical::ApplySpringDampeningOld", expect = "83 EC 0C 8B 44 24 20", this = "CPhysical")]
pub fn apply_spring_dampening_old(this: Ent, damping: f32, _limit: f32, dir: Ent, point: Ent, speed: Ent) -> u8 {
    let mut f = Fpu::new();
    dot_yzx(&mut f, rv(speed, 0), rv(dir, 0), true);
    let s1 = f.fstp();
    let mut vo = CVector::new(0.0, 0.0, 0.0);
    let v = rv(get_speed_call(this, lp(&mut vo), cv(rv(point, 0))).cast(), 0);
    dot_yzx(&mut f, v, rv(dir, 0), false);
    let s2 = f.fst();
    f.fadd_m(s1);
    f.fmul_m(cf(k::K_858B8C));
    push_capped_time_step(&mut f);
    f.fmul_m(rf(this, MASS));
    f.fmulp(1);
    f.fmul_m(damping);
    f.fmul_m(cf(k::K_863C04));
    if rb(this, PFLAGS) & 1 != 0 {
        f.fld_st(0);
        f.faddp(1);
    }
    f.fld(rf(point, 8));
    f.fld(rf(point, 4));
    f.fld(rf(point, 0));
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fadd_m(cf(k::K_858624));
    f.fmul_m(rf(this, MASS));
    f.fadd_st(0);
    f.fdivr_m(rf(this, TURN_MASS));
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fcom_m(cf(k::K_858624));
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fld(cf(k::K_858624));
    }
    f.fld(s2);
    f.fmul_m(rf(this, MASS));
    f.fdivr_st(2);
    f.fcom_m(cf(k::K_858B50));
    if !f.jp(5) {
        f.fchs();
    }
    f.fcom_st(1);
    if !f.jne(0x41) {
        f.fxch(1);
        f.fdiv_st(1);
        f.fmulp(2);
    } else {
        f.fstp_st(0);
    }
    f.fstp_st(0);
    let v = scale_pop(&mut f, dir);
    apply_force_at(this, v, point);
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_APPLY_SPRING_COLLISION,
    HOOK_APPLY_SPRING_COLLISION_ALT,
    HOOK_APPLY_SPRING_DAMPENING,
    HOOK_APPLY_SPRING_DAMPENING_OLD,
];
