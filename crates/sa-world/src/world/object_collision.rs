//! `CObject::SpecialEntityPreCollisionStuff` (batch `mvpk-world-2`): which object-vs-entity collisions are skipped or
//! softened; `CObject::SpecialEntityCalcCollisionSteps` (batch `mvpk-world-3`): the collision sub-step count. Ported from the 1.0 US bytes (gta-reversed `Object.cpp` as a reading aid; the bytes win). Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::x87::X87;

use super::common::{g, rd, wr, x};

orig! {
    /// `CEntity::GetIsStatic()` (0x4633E0)
    fn entity_get_is_static(e: GPtr<u8>) -> u8 = thiscall 0x4633E0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CMatrix::operator*(CVector* out, const CMatrix&, const CVector&)` (0x59C890; rotation plus translation)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `Invert(const CMatrix& src, CMatrix& dst)` (0x59B920)
    fn matrix_invert(src: GPtr<u8>, dst: GPtr<u8>) -> GPtr<u8> = cdecl 0x59B920 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0, `ret 4`)
    fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
}

/// 0.66 (0x85BA50): the "at" z below which an object flagged +0x140 bit 8 lets vehicles and peds through
const K_TIPPED_AT_Z: usize = 0x85BA50;
/// 10.0 (0x85862C): the mass ratio that makes another object unable to move this one
const K_MASS_RATIO: usize = 0x85862C;
/// 1.0 (0x858624), 0.0 (0x858B50)
const K_ONE: usize = 0x858624;
const K_ZERO: usize = 0x858B50;

fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}
fn ty(e: GPtr<u8>) -> u8 {
    rd::<u8>(e, 0x36) & 7
}
fn uses_collision_recording(e: GPtr<u8>) -> bool {
    (rd::<u32>(e, 0x1C) >> 4) & 1 != 0
}
fn set(p: GPtr<u8>) {
    p.write(1);
}
/// An entity's position (matrix +0x30, else the placement +4).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CObject::SpecialEntityPreCollisionStuff(CPhysical* other, bool ignoreStuck, bool& disabled, bool& ignored, bool&
/// unableToMove, bool& stuck)` (0x59FEE0, thiscall, `ret 0x18`). x87 compares.
///
/// Touching each other (+0x128 either way) → `ignored`, done. Attached (+0xFC) to the other, the other to this, or both to
/// the same entity → `disabled`. Infinite mass (+0x40 bit 7): without bit 1 and the other not infinite (+0x40 byte sign)
/// → `disabled`; else `ignoreStuck` → `disabled`, else either "uses collision recording" (+0x1C bit 4) → `stuck`. Flags
/// +0x40 & 0x60 → `ignoreStuck` or this +0x1C bit 4 → `disabled`; else the other's +0x1C bit 4 → (its bit 12 ?
/// `unableToMove` : `stuck`). Otherwise:
///
/// - Object flag +0x140 bit 8 with "at" z (+0x28) below 0.66 or bit 4 of +0x1C, against a vehicle or ped → `ignored`;
///   a vehicle also becomes +0x128; a ped returns at once.
/// - Against a vehicle: models 0x176..0x17D → `disabled`; not type 3 (+0x13C), not +0x140 bit 6 and static → nothing;
///   vehicle models 0x196, 0x1E6, 0x212 → either +0x1C bit 4 → `stuck`; damage (+0x144) below 20 and the col model's
///   bounding-box max components all below 1.0 → `p = M * (max - min)` (with translation); `p.z` below the vehicle's z →
///   `ignored` and +0x128 = vehicle; else the vehicle's inverse matrix (stack `CMatrix`) times p, its z below 0 → the
///   same.
/// - Model 0x156 against a ped standing higher (matrix z) → `disabled`.
/// - Against an object whose info (+0x160) +0x14 is above 0, not attached, not (+0x40 bit 2 without bit 3), and whose
///   mass (+0x8C) * 10 is above this mass → `unableToMove`.
///
/// Then, unless `ignored`: `ignoreStuck` or this +0x1C bit 4 → `stuck`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x59fee0, conv = thiscall, name = "CObject::SpecialEntityPreCollisionStuff", expect = "64 A1 00 00 00 00",
       this = "CObject", route = "L2", hazards = "none", recs = "grove-walk",
       note = "the CMatrix temporary lives on the Rust stack")]
pub fn object_special_entity_pre_collision_stuff(
    this: GPtr<u8>,
    other: GPtr<u8>,
    ignore_stuck: u8,
    disabled: GPtr<u8>,
    ignored: GPtr<u8>,
    unable: GPtr<u8>,
    stuck: GPtr<u8>,
) {
    if rd::<GPtr<u8>>(this, 0x128) == other || rd::<GPtr<u8>>(other, 0x128) == this {
        set(ignored);
        return;
    }
    if !pre_collision_body(this, other, ignore_stuck, disabled, ignored, unable, stuck) {
        return;
    }
    if ignored.read() != 0 {
        return;
    }
    if ignore_stuck != 0 || uses_collision_recording(this) {
        set(stuck);
    }
}

/// The body before the common tail; false when the original returns without the tail (the ped case).
fn pre_collision_body(this: GPtr<u8>, other: GPtr<u8>, ignore_stuck: u8, disabled: GPtr<u8>, ignored: GPtr<u8>, unable: GPtr<u8>, stuck: GPtr<u8>) -> bool {
    let att = rd::<GPtr<u8>>(this, 0xFC);
    let oatt = rd::<GPtr<u8>>(other, 0xFC);
    if att == other || oatt == this || (!att.is_null() && att == oatt) {
        set(disabled);
        return true;
    }
    let fl = rd::<u32>(this, 0x40);
    let infinite = (fl >> 7) & 1 != 0;
    if infinite && fl & 2 == 0 && (rd::<u8>(other, 0x40) as i8) >= 0 {
        set(disabled);
        return true;
    }
    if infinite {
        if ignore_stuck != 0 {
            set(disabled);
        } else if uses_collision_recording(this) || uses_collision_recording(other) {
            set(stuck);
        }
        return true;
    }
    if fl & 0x60 != 0 {
        if ignore_stuck != 0 || uses_collision_recording(this) {
            set(disabled);
        } else {
            let of = rd::<u32>(other, 0x1C);
            if (of >> 4) & 1 != 0 {
                if (of >> 0xC) & 1 == 0 {
                    set(stuck);
                } else {
                    set(unable);
                }
            }
        }
        return true;
    }
    let oflags = rd::<u32>(this, 0x140);
    if oflags & 0x100 != 0 {
        let low = !x(rd::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x28)).fcom(x(k(K_TIPPED_AT_Z))).test_parity(5);
        if (low || uses_collision_recording(this)) && matches!(ty(other), 2 | 3) {
            set(ignored);
            if ty(other) == 2 {
                wr::<GPtr<u8>>(this, 0x128, other);
                return true;
            }
            return false;
        }
    }
    let t = ty(other);
    if t == 2 {
        let model = rd::<u16>(this, 0x22);
        if (0x176..=0x17D).contains(&model) {
            set(disabled);
            return true;
        }
        if rd::<u8>(this, 0x13C) != 3 && oflags & 0x40 == 0 && entity_get_is_static(this) != 0 {
            return true;
        }
        if matches!(rd::<u16>(other, 0x22), 0x196 | 0x1E6 | 0x212) {
            if uses_collision_recording(this) || uses_collision_recording(other) {
                set(stuck);
            }
            return true;
        }
        if rd::<u8>(this, 0x144) >= 0x14 {
            return true;
        }
        let col = entity_get_col_model(this);
        let one = x(k(K_ONE));
        for o in [0xCisize, 0x10, 0x14] {
            if x(rd::<f32>(col, o)).fcom(one).test_parity(5) {
                return true;
            }
        }
        let mut cm = [0u32; 0x12];
        let cm_p: GPtr<u8> = GPtr::from_raw(cm.as_mut_ptr()).cast();
        let mut d = [0u32; 3];
        let d_p: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
        vector_sub(d_p, col.byte_add(0xC), col);
        let mut out = [0u32; 3];
        let r = matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr()).cast(), rd::<GPtr<u8>>(this, 0x14), d_p);
        let mut p = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
        let p_p: GPtr<u8> = GPtr::from_raw(p.as_mut_ptr()).cast();
        let om = rd::<GPtr<u8>>(other, 0x14);
        let below = !x(f32::from_bits(p[2])).fcom(x(rd::<f32>(entity_pos(other), 8))).test_parity(5);
        let hit = below || {
            let inv = matrix_invert(om, cm_p);
            matrix_assign(cm_p, inv);
            let mut o2 = [0u32; 3];
            let r2 = matrix_mul_vec(GPtr::from_raw(o2.as_mut_ptr()).cast(), cm_p, p_p);
            !x(rd::<f32>(r2, 8)).fcom(x(k(K_ZERO))).test_parity(5)
        };
        if hit {
            set(ignored);
            wr::<GPtr<u8>>(this, 0x128, other);
        }
        matrix_dtor(cm_p);
        return true;
    }
    if rd::<u16>(this, 0x22) == 0x156 && t == 3 {
        let mz = rd::<f32>(rd::<GPtr<u8>>(this, 0x14), 0x38);
        let oz = rd::<f32>(rd::<GPtr<u8>>(other, 0x14), 0x38);
        if !x(mz).fcom(x(oz)).test_parity(5) {
            set(disabled);
            return true;
        }
    }
    if t == 4 {
        let info = rd::<GPtr<u8>>(other, 0x160);
        if x(rd::<f32>(info, 0x14)).fcom(x(k(K_ZERO))).test_any(0x41) {
            return true;
        }
        if !oatt.is_null() {
            return true;
        }
        let of = rd::<u32>(other, 0x40);
        if of & 4 != 0 && of & 8 == 0 {
            return true;
        }
        if !x(rd::<f32>(other, 0x8C)).fmul_m(k(K_MASS_RATIO)).fcom(x(rd::<f32>(this, 0x8C))).test_any(0x41) {
            set(unable);
        }
    }
    true
}

orig! {
    /// `CPhysical::GetSpeed(CVector* out, CVector point)` (0x542CE0, `ret 0x10`)
    fn physical_get_speed(p: GPtr<u8>, out: GPtr<u8>, x: u32, y: u32, z: u32) -> GPtr<u8> = thiscall 0x542CE0 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix&, const CVector&)` (0x59C790)
    fn multiply_3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CVector::SquaredMagnitude()` (0x406DA0, ST0)
    fn vector_squared_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// The `ceilf`-style wrapper 0x538AD0 (`ceil((double) f)`, ST0)
    fn ceil_of_float(v: f32) -> f64 = cdecl 0x538AD0 => unported;
    /// CRT `ceil(double)` (0x823820, ST0)
    fn crt_ceil(v: f64) -> f64 = cdecl 0x823820 => unported;
}

/// `CTimer::ms_fTimeStep` (0xB7CB5C), `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const TIMER_TIME_STEP: usize = 0xB7CB5C;
const MODEL_INFOS: usize = 0xA9B0C8;

fn step_count(v: X87) -> u8 {
    v.ftol() as u32 as u8
}

/// `CObject::SpecialEntityCalcCollisionSteps(bool&, bool&)` (0x5A02E0, thiscall, `ret 8`; the arguments are unused). x87
/// (FSQRT); returns the step count in AL (EAX = the `_ftol` result, the port returns AL zero-extended). With the time
/// step `ts`:
///
/// - Infinite mass (+0x40 bit 7) or object info (+0x160) +0x1D == 5: `m2 = (x² + y² + z²) * ts * ts` of the move speed
///   (+0x44); below `r²` (the model's col sphere radius +0x24) → 1; else `_ftol(ceil(sqrt(f32 m2) / r))`.
/// - Flag +0x40 bit 5: `e` = the larger of the col box max x / y, replaced by `min(min.x, min.y)` when its negation is
///   larger (original); `|e * turn speed z (+0x58)|` not above 0.1 → 1, else `_ftol(ceil(|..| * 10))`.
/// - Bit 6: the turn speed at the col box min point (`GetSpeed(Multiply3x3(M, min))`), `|v|² * ts * ts` below 0.0225 → 1,
///   else `_ftol(ceilf(f32(sqrt * 6.6667)))`.
/// - Type 3 (+0x13C) without +0x140 bit 13 → 1.
/// - Info +0x1D == 1: the speeds at `M * (0, 0, max.z)` and `M * (0, 0, min.z)` (no translation); the larger squared
///   (f32 of the first; ties take the second) `* ts * ts` below 0.09 → 1, else `ceilf(f32(sqrt * 3.3333))`.
/// - Not +0x140 bit 13 and info +0x1D neither 2 nor 4: `|move speed|² * ts * ts` below 0.09 → 1, else `ceilf(f32(sqrt *
///   3.3333))`.
/// - Else `s = f32(0.5 * min(dx, dy, dz))` of the col box (dy and dz as f32 when chosen); `|move speed|² * ts * ts` below
///   `s²` → 1, else `ceilf(f32(sqrt / s))`.
#[hook(addr = 0x5a02e0, conv = thiscall, name = "CObject::SpecialEntityCalcCollisionSteps", expect = "83 EC 28 53 56",
       this = "CObject", route = "L2", hazards = "x87-transcendental,ax-al-return", recs = "grove-walk",
       note = "AL = _ftol(ceil(..)); EAX upper bytes are the _ftol result's")]
pub fn object_special_entity_calc_collision_steps(this: GPtr<u8>, _a: GPtr<u8>, _b: GPtr<u8>) -> u8 {
    let ts = || g::<f32>(TIMER_TIME_STEP).read();
    let fl = rd::<u32>(this, 0x40);
    let info_kind = || rd::<u8>(rd::<GPtr<u8>>(this, 0x160), 0x1D);
    let col_of_model = || {
        let mi = GPtr::<GPtr<u8>>::from_addr(MODEL_INFOS + 4 * (rd::<i16>(this, 0x22) as isize as usize)).read();
        rd::<GPtr<u8>>(mi, 0x14)
    };
    if (fl as u8 as i8) < 0 || info_kind() == 5 {
        let (sx, sy, sz) = (rd::<f32>(this, 0x44), rd::<f32>(this, 0x48), rd::<f32>(this, 0x4C));
        let m2 = x(sx).fmul_m(sx).fadd(x(sy).fmul_m(sy)).fadd(x(sz).fmul_m(sz)).fmul_m(ts()).fmul_m(ts());
        let m2f = m2.to_f32();
        let col = col_of_model();
        let r = x(rd::<f32>(col, 0x24));
        if m2.fcom(r.fmul(r)).c0() {
            return 1;
        }
        let q = x(m2f).fsqrt().fdiv(x(rd::<f32>(col_of_model(), 0x24)));
        return step_count(X87::from_f64_exact(crt_ceil(q.to_f64())));
    }
    if fl & 0x20 != 0 {
        let mx = rd::<f32>(entity_get_col_model(this), 0xC);
        let my = rd::<f32>(entity_get_col_model(this), 0x10);
        let mut e = if x(my).fcom(x(mx)).test_parity(5) {
            rd::<f32>(entity_get_col_model(this), 0x10)
        } else {
            rd::<f32>(entity_get_col_model(this), 0xC)
        };
        let nx = rd::<f32>(entity_get_col_model(this), 0);
        let ny = rd::<f32>(entity_get_col_model(this), 4);
        let v = if x(ny).fcom(x(nx)).test_any(0x41) {
            rd::<f32>(entity_get_col_model(this), 4)
        } else {
            rd::<f32>(entity_get_col_model(this), 0)
        };
        if !x(v).fchs().fcom(x(e)).test_any(0x41) {
            e = v;
        }
        let a = x(e).fmul_m(rd::<f32>(this, 0x58)).fabs();
        if a.fcom(x(k(0x858B1C))).test_any(0x41) {
            return 1;
        }
        return step_count(X87::from_f64_exact(crt_ceil(a.fmul_m(k(K_MASS_RATIO)).to_f64())));
    }
    if fl & 0x40 != 0 {
        let col = entity_get_col_model(this);
        let mut out = [0u32; 3];
        let r = multiply_3x3(GPtr::from_raw(out.as_mut_ptr()).cast(), rd::<GPtr<u8>>(this, 0x14), col);
        let p = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
        let mut sp = [0u32; 3];
        let s = physical_get_speed(this, GPtr::from_raw(sp.as_mut_ptr()).cast(), p[0], p[1], p[2]);
        let v = [rd::<f32>(s, 0), rd::<f32>(s, 4), rd::<f32>(s, 8)];
        let m2 = x(v[2]).fmul_m(v[2]).fadd(x(v[1]).fmul_m(v[1])).fadd(x(v[0]).fmul_m(v[0])).fmul_m(ts()).fmul_m(ts());
        if m2.fcom(x(k(0x866FC0))).c0() {
            return 1;
        }
        return step_count(X87::from_f64_exact(ceil_of_float(m2.fsqrt().fmul_m(k(0x866FBC)).to_f32())));
    }
    if rd::<u8>(this, 0x13C) == 3 && rd::<u32>(this, 0x140) & 0x2000 == 0 {
        return 1;
    }
    let ms = this.byte_add(0x44);
    let info_kind = info_kind();
    if info_kind == 1 {
        let col = col_of_model();
        let mut a = [0u32, 0, rd::<u32>(col, 0x14)];
        let mut b = [0u32, 0, rd::<u32>(col, 8)];
        let mut o1 = [0u32; 3];
        let r1 = multiply_3x3(GPtr::from_raw(o1.as_mut_ptr()).cast(), rd::<GPtr<u8>>(this, 0x14), GPtr::from_raw(a.as_mut_ptr()).cast());
        a = [rd::<u32>(r1, 0), rd::<u32>(r1, 4), rd::<u32>(r1, 8)];
        let mut o2 = [0u32; 3];
        let r2 = multiply_3x3(GPtr::from_raw(o2.as_mut_ptr()).cast(), rd::<GPtr<u8>>(this, 0x14), GPtr::from_raw(b.as_mut_ptr()).cast());
        b = [rd::<u32>(r2, 0), rd::<u32>(r2, 4), rd::<u32>(r2, 8)];
        let mut out = [0u32; 3];
        let s1 = physical_get_speed(this, GPtr::from_raw(out.as_mut_ptr()).cast(), a[0], a[1], a[2]);
        let mut va = [rd::<u32>(s1, 0), rd::<u32>(s1, 4), rd::<u32>(s1, 8)];
        let s2 = physical_get_speed(this, GPtr::from_raw(out.as_mut_ptr()).cast(), b[0], b[1], b[2]);
        let mut vb = [rd::<u32>(s2, 0), rd::<u32>(s2, 4), rd::<u32>(s2, 8)];
        let pa: GPtr<u8> = GPtr::from_raw(va.as_mut_ptr()).cast();
        let pb: GPtr<u8> = GPtr::from_raw(vb.as_mut_ptr()).cast();
        let ma = X87::from_f64_exact(vector_squared_magnitude(pa)).to_f32();
        let mb = X87::from_f64_exact(vector_squared_magnitude(pb));
        let pick = if !mb.fcom(x(ma)).test_parity(5) { pa } else { pb };
        let m2 = X87::from_f64_exact(vector_squared_magnitude(pick)).fmul_m(ts()).fmul_m(ts());
        if m2.fcom(x(k(0x85B324))).c0() {
            return 1;
        }
        return step_count(X87::from_f64_exact(ceil_of_float(m2.fsqrt().fmul_m(k(0x859F60)).to_f32())));
    }
    if rd::<u32>(this, 0x140) & 0x2000 == 0 && info_kind != 2 && info_kind != 4 {
        let m2 = X87::from_f64_exact(vector_squared_magnitude(ms)).fmul_m(ts()).fmul_m(ts());
        if m2.fcom(x(k(0x85B324))).c0() {
            return 1;
        }
        return step_count(X87::from_f64_exact(ceil_of_float(m2.fsqrt().fmul_m(k(0x859F60)).to_f32())));
    }
    let col = entity_get_col_model(this);
    let dx = x(rd::<f32>(col, 0xC)).fsub_m(rd::<f32>(col, 0));
    let dy = x(rd::<f32>(col, 0x10)).fsub_m(rd::<f32>(col, 4));
    let mut m = if dy.fcom(dx).test_parity(5) { dx } else { x(dy.to_f32()) };
    let dz = x(rd::<f32>(col, 0x14)).fsub_m(rd::<f32>(col, 8));
    if !dz.fcom(m).test_parity(5) {
        m = x(dz.to_f32());
    }
    let s = x(k(0x8D0A28)).fmul(m).to_f32();
    let m2 = X87::from_f64_exact(vector_squared_magnitude(ms)).fmul_m(ts()).fmul_m(ts());
    if m2.fcom(x(s).fmul_m(s)).c0() {
        return 1;
    }
    step_count(X87::from_f64_exact(ceil_of_float(m2.fsqrt().fdiv_m(s).to_f32())))
}

pub const HOOKS: &[HookReg] = &[HOOK_OBJECT_SPECIAL_ENTITY_PRE_COLLISION_STUFF, HOOK_OBJECT_SPECIAL_ENTITY_CALC_COLLISION_STEPS];
