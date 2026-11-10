//! `cBuoyancy` (PORT-PHYS), ported instruction by instruction from the 1.0 US bytes with the [`Fpu`]
//! stack machine (PC = 24); gta-reversed `Buoyancy.cpp` was a reading aid.
//!
//! Layout (gr `Buoyancy.h`, 0xD0): pos +0, entity matrix +0xC (0x48), water level +0x64, buoyancy
//! constant +0x6C, bounding max +0x70, bounding min +0x7C, checked points +0x88, in water +0x98,
//! centre offset +0x9C, normalised centre offset +0xA8, flip +0xB8, processing boat +0xBA, immersion
//! +0xBC, turn point +0xC0.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::shared::*;
use crate::collision::fpu::Fpu;

/// A `cBuoyancy*`.
pub type Buoy = GPtr<u8>;

const MATRIX_COPY: isize = 0xC;
const WATER_LEVEL: isize = 0x64;
const BUOYANCY_CONST: isize = 0x6C;
const BB_MAX: isize = 0x70;
const BB_MIN: isize = 0x7C;
const NUM_CHECKED: isize = 0x88;
const IN_WATER: isize = 0x98;
const NORM_CENTRE: isize = 0xA8;
const FLIP: isize = 0xB8;
const PROCESSING_BOAT: isize = 0xBA;
const IMMERSION: isize = 0xBC;
const TURN_POINT: isize = 0xC0;

orig! {
    /// `CWaterLevel::GetWaterLevel(float x, float y, float z, float* out, bool waves, CVector*
    /// normal)` (0x6EB690, cdecl, AL)
    pub fn get_water_level(x: f32, y: f32, z: f32, out: GPtr<f32>, waves: u32, normal: Ent) -> u8
        = cdecl 0x6EB690 => unported;
    /// `cBuoyancy::PreCalcSetup(CPhysical*, float)` (0x6C2B90)
    pub fn pre_calc_setup(b: Buoy, e: Ent, buoyancy: f32) = thiscall 0x6C2B90 => unported;
    /// `cBuoyancy::SimpleCalcBuoyancy(CPhysical*)` (0x6C3B00)
    pub fn simple_calc_buoyancy(b: Buoy, e: Ent) = thiscall 0x6C3B00 => unported;
    /// `cBuoyancy::AddSplashParticles(CPhysical*, CVector, CVector, CVector, uint8)` (0x6C34E0)
    pub fn add_splash_particles(b: Buoy, e: Ent, from: CVector, to: CVector, dir: CVector, reduce: u32)
        = thiscall 0x6C34E0 => unported;
    /// `cBuoyancy::CalcBuoyancyForce(CPhysical*, CVector*, CVector*)` (0x6C2750)
    pub fn calc_buoyancy_force_call(b: Buoy, e: Ent, turn: GPtr<CVector>, force: GPtr<CVector>) -> u8
        = thiscall 0x6C2750 => calc_buoyancy_force;
    /// `CPlaceable::GetTopDirection()` (0x41CCB0, hidden return pointer)
    pub fn get_top_direction(e: Ent, out: GPtr<CVector>) -> GPtr<CVector> = thiscall 0x41CCB0 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    pub fn matrix_assign(dst: Ent, src: Ent) = thiscall 0x59BBC0 => unported;
}

sa_core::globals! {
    list GLOBALS;
    /// `cBuoyancy::calcStruct` (`CBuoyancyCalcStruct`, 0x1C bytes; +0x18 is the static-init guard
    /// dword of the function-local static)
    pub static CALC_STRUCT @ 0xC1C858: [u32; 7] = [0; 7];
    /// `cBuoyancy::fPointVolMultiplier`
    pub static POINT_VOL_MULTIPLIER @ 0x8D32C8: f32 = 1.0;
}

fn cs(i: isize) -> GPtr<f32> {
    CALC_STRUCT.ptr().cast::<f32>().wrapping_index(i as i32)
}

/// `cBuoyancy::cBuoyancy()` (0x6C2740, returns this): the embedded matrix's attach pointer and owns
/// flag (+0x4C, +0x50) are zeroed.
#[hook(addr = 0x6C2740, conv = thiscall, name = "cBuoyancy::cBuoyancy", expect = "8B C1 33 C9 89 48 4C")]
pub fn constructor(this: Buoy) -> Buoy {
    wu(this, 0x4C, 0);
    wu(this, 0x50, 0);
    this
}

/// `cBuoyancy::CalcBuoyancyForce(CPhysical*, CVector* turn, CVector* force)` (0x6C2750, `ret 0xC`,
/// AL): false unless in water; `turn = Multiply3x3(matrix, turnPoint)`, `force = (0, 0, immersion *
/// buoyancy * timeStep)`; when `mass * moveSpeed.z > 4 * force.z` the z becomes `max(z - mass *
/// moveSpeed.z, 0)`.
#[hook(addr = 0x6C2750, conv = thiscall, name = "cBuoyancy::CalcBuoyancyForce", expect = "83 EC 0C 56 8B F1")]
pub fn calc_buoyancy_force(this: Buoy, e: Ent, turn: GPtr<CVector>, force: GPtr<CVector>) -> u8 {
    if rb(this, IN_WATER) == 0 {
        return 0;
    }
    let mut o = CVector::new(0.0, 0.0, 0.0);
    let t = rv_bits(multiply3x3(lp(&mut o), this.byte_add(MATRIX_COPY), this.byte_add(TURN_POINT).cast()).cast(), 0);
    wv_bits(turn.cast(), 0, t);
    let fo: Ent = force.cast();
    wu(fo, 0, 0);
    wu(fo, 4, 0);
    let mut f = Fpu::new();
    f.fld(rf(this, IMMERSION));
    f.fmul_m(rf(this, BUOYANCY_CONST));
    f.fmul_m(sa_core::timer::TIME_STEP.get());
    wf(fo, 8, f.fst());
    f.fld(rf(e, MASS));
    f.fmul_m(rf(e, MOVE_SPEED + 8));
    f.fld_st(1);
    f.fmul_m(cf(k::K_858B90));
    f.fld_st(1);
    f.fcompp();
    if f.jne(0x41) {
        return 1;
    }
    f.fsubp(1);
    f.fld(cf(k::K_858B50));
    f.fcomp_st(1);
    if !f.jne(0x41) {
        f.fstp_st(0);
        f.fld(cf(k::K_858B50));
    }
    wf(fo, 8, f.fstp());
    1
}

/// The common part of FindWaterLevel / FindWaterLevelNorm: `p = pos + Multiply3x3(matrix, offset)`,
/// `GetWaterLevel(p.x, p.y, pos.z, &offset.z, true, normal)`, `offset.z = level - (w.z +
/// initial.z)`, clamped to the bounding box z range (state 2 above max, 0 below min, else 1).
fn find_water_level_common(this: Buoy, initial: Ent, offset: Ent, state: GPtr<u32>, normal: Ent) {
    state.write(1);
    let mut o = CVector::new(0.0, 0.0, 0.0);
    let w = rv(multiply3x3(lp(&mut o), this.byte_add(MATRIX_COPY), offset.cast()).cast(), 0);
    let mut f = Fpu::new();
    f.fld(w[0]);
    f.fadd_m(rf(this, 0));
    let px = f.fstp();
    f.fld(w[1]);
    f.fadd_m(rf(this, 4));
    let py = f.fstp();
    let oz: GPtr<f32> = offset.byte_add(8).cast();
    get_water_level(px, py, rf(this, 8), oz, 1, normal);
    f.fld(w[2]);
    f.fadd_m(rf(initial, 8));
    f.fsubr_m(oz.read());
    oz.write(f.fst());
    f.fcom_m(rf(this, BB_MAX + 8));
    if !f.jne(0x41) {
        wu(offset, 8, ru(this, BB_MAX + 8));
        state.write(2);
        return;
    }
    f.fcomp_m(rf(this, BB_MIN + 8));
    if !f.jp(5) {
        wu(offset, 8, ru(this, BB_MIN + 8));
        state.write(0);
    }
}

/// `cBuoyancy::FindWaterLevel(const CVector& initial, CVector* offset, tWaterLevel* state)`
/// (0x6C2810, `ret 0xC`).
#[hook(addr = 0x6C2810, conv = thiscall, name = "cBuoyancy::FindWaterLevel", expect = "83 EC 0C 53 8B 5C 24 1C")]
pub fn find_water_level(this: Buoy, initial: Ent, offset: Ent, state: GPtr<u32>) {
    find_water_level_common(this, initial, offset, state, GPtr::NULL)
}

/// `cBuoyancy::FindWaterLevelNorm(const CVector& initial, CVector* offset, tWaterLevel* state,
/// CVector* normal)` (0x6C28C0, `ret 0x10`).
#[hook(addr = 0x6C28C0, conv = thiscall, name = "cBuoyancy::FindWaterLevelNorm", expect = "83 EC 0C 53 8B 5C 24 1C")]
pub fn find_water_level_norm(this: Buoy, initial: Ent, offset: Ent, state: GPtr<u32>, normal: Ent) {
    find_water_level_common(this, initial, offset, state, normal)
}

/// `cBuoyancy::SimpleSumBuoyancyData(CVector* point, tWaterLevel)` (0x6C2970, `ret 8`, ST0): the
/// point's contribution `|p.z - min.z| - (1 - fPointVolMultiplier)` (squared after scaling for
/// boats) is added to the immersion; the turn point becomes the running average with the
/// contribution-weighted offset (flipped when asked); the count grows by one. 0 when the
/// contribution is negative. Works in the static `calcStruct` (0xC1C858).
#[hook(addr = 0x6C2970, conv = thiscall, name = "cBuoyancy::SimpleSumBuoyancyData", expect = "A0 70 C8 C1 00")]
pub fn simple_sum_buoyancy_data(this: Buoy, p: Ent, _state: u32) -> X87 {
    let guard = CALC_STRUCT.ptr().cast::<u32>().wrapping_index(6);
    if guard.read() & 1 == 0 {
        guard.write(guard.read() | 1);
    }
    let mut f = Fpu::new();
    f.fld(rf(p, 8));
    f.fsub_m(rf(this, BB_MIN + 8));
    f.fabs();
    f.fld(cf(k::K_858624));
    f.fsub_m(POINT_VOL_MULTIPLIER.get());
    f.fsubp(1);
    cs(5).write(f.fst());
    f.fcomp_m(cf(k::K_858B50));
    if !f.jp(5) {
        return X87::from_f32(cf(k::K_858B50));
    }
    if rb(this, PROCESSING_BOAT) != 0 {
        f.fld(POINT_VOL_MULTIPLIER.get());
        f.fmul_m(cs(5).read());
        f.fld_st(0);
        f.fmul_st(1);
        cs(5).write(f.fstp());
        f.fstp_st(0);
    }
    f.fld(cs(5).read());
    f.fadd_m(rf(this, IMMERSION));
    wf(this, IMMERSION, f.fstp());
    f.fld(rf(p, 8));
    f.fadd_m(rf(this, BB_MIN + 8));
    f.fmul_m(rf(this, NORM_CENTRE + 8));
    f.fmul_m(cf(k::K_858B8C));
    cs(4).write(f.fstp());
    f.fld(rf(this, NORM_CENTRE));
    f.fmul_m(rf(p, 0));
    cs(2).write(f.fstp());
    f.fld(rf(this, NORM_CENTRE + 4));
    f.fmul_m(rf(p, 4));
    cs(3).write(f.fstp());
    if rb(this, FLIP) != 0 {
        for i in 2..5 {
            f.fld(cs(i).read());
            f.fmul_m(cf(k::K_858C1C));
            let v = f.fstp();
            cs(i).write(v);
        }
    }
    f.fld(cf(k::K_858624));
    f.fdiv_m(rf(this, NUM_CHECKED));
    cs(1).write(f.fstp());
    f.fld(cf(k::K_858624));
    f.fsub_m(cs(1).read());
    cs(0).write(f.fstp());
    let g4 = cs(1).read();
    let g14 = cs(5).read();
    f.fld(g4);
    f.fmul_m(cs(2).read());
    f.fld(g4);
    f.fmul_m(cs(3).read());
    f.fld(g4);
    f.fmul_m(cs(4).read());
    let c = f.fstp();
    f.fld(g14);
    f.fmul_st(2);
    let ax = f.fstp();
    f.fmul_m(g14);
    f.fstp_st(1);
    f.fld(c);
    f.fmul_m(g14);
    let g0 = cs(0).read();
    for i in 0..3 {
        f.fld(g0);
        f.fmul_m(rf(this, TURN_POINT + 4 * i));
    }
    wb(this, IN_WATER, 1);
    let tz = f.fstp();
    f.fxch(1);
    f.fadd_m(ax);
    wf(this, TURN_POINT, f.fstp());
    f.fadd_st(2);
    wf(this, TURN_POINT + 4, f.fstp());
    f.fld(tz);
    f.fadd_st(1);
    wf(this, TURN_POINT + 8, f.fstp());
    f.fstp_st(0);
    f.fstp_st(0);
    f.fld(rf(this, NUM_CHECKED));
    f.fadd_m(cf(k::K_858624));
    wf(this, NUM_CHECKED, f.fstp());
    X87::from_f32(cs(5).read())
}

/// `cBuoyancy::ProcessBuoyancy(CPhysical* e, float buoyancy, CVector* turn, CVector* force)`
/// (0x6C3EF0, `ret 0x10`, AL). No water level at the position (`bTouchingWater` asks for waves):
/// false. Copies the entity matrix, `PreCalcSetup`; a ped's immersion is `((level - z) + 1) *
/// 0.5263158f` clamped to [0, 1] (0 leaves the water), with splashes from below the surface while
/// partly immersed; others `SimpleCalcBuoyancy`. Then `CalcBuoyancyForce`; true when it returned
/// true or a boat is processed.
#[hook(addr = 0x6C3EF0, conv = thiscall, name = "cBuoyancy::ProcessBuoyancy", expect = "83 EC 24 53 55")]
pub fn process_buoyancy(this: Buoy, e: Ent, buoyancy: f32, turn: GPtr<CVector>, force: GPtr<CVector>) -> u8 {
    let pos = rv(pos_ptr(e), 0);
    let touching = (ru(e, PFLAGS) >> 27) & 1;
    if get_water_level(pos[0], pos[1], pos[2], this.byte_add(WATER_LEVEL).cast(), touching, GPtr::NULL) == 0 {
        return 0;
    }
    matrix_assign(this.byte_add(MATRIX_COPY), matrix(e));
    pre_calc_setup(this, e, buoyancy);
    if etype(e) == ty::PED {
        get_col_model(e);
        let mut f = Fpu::new();
        f.fld(rf(this, WATER_LEVEL));
        f.fsub_m(rf(pos_ptr(e), 8));
        wb(this, IN_WATER, 1);
        f.fsub_m(cf(k::K_858C1C));
        f.fmul_m(cf(k::K_871664));
        wf(this, IMMERSION, f.fstp());
        f.fld(rf(this, IMMERSION));
        f.fcomp_m(cf(k::K_858624));
        if !f.jne(0x41) {
            wu(this, IMMERSION, 0x3F80_0000);
        } else {
            f.fld(rf(this, IMMERSION));
            f.fcomp_m(cf(k::K_858B50));
            if !f.jp(5) {
                wu(this, IMMERSION, 0);
                wb(this, IN_WATER, 0);
            }
        }
        f.fld(rf(this, IMMERSION));
        f.fcomp_m(cf(k::K_858B50));
        wv_bits(this, TURN_POINT, [0; 3]);
        if !f.jne(0x41) {
            f.fld(rf(this, IMMERSION));
            f.fcomp_m(cf(k::K_858624));
            if !f.jp(5) {
                f.fld(rf(this, WATER_LEVEL));
                f.fsub_m(rf(pos_ptr(e), 8));
                let d = f.fstp();
                let mut o = CVector::new(0.0, 0.0, 0.0);
                let top = rv(get_top_direction(e, lp(&mut o)).cast(), 0);
                let mut dir = [0f32; 3];
                for (i, t) in top.iter().enumerate() {
                    f.fld(*t);
                    f.fchs();
                    dir[i] = f.fstp();
                }
                let v = CVector::new(0.0, 0.0, d);
                add_splash_particles(this, e, v, v, cv(dir), 1);
            }
        }
    } else {
        simple_calc_buoyancy(this, e);
    }
    let r = calc_buoyancy_force_call(this, e, turn, force);
    if rb(this, PROCESSING_BOAT) != 0 {
        return 1;
    }
    (r != 0) as u8
}

pub const HOOKS: &[HookReg] = &[
    HOOK_CONSTRUCTOR,
    HOOK_CALC_BUOYANCY_FORCE,
    HOOK_FIND_WATER_LEVEL,
    HOOK_FIND_WATER_LEVEL_NORM,
    HOOK_SIMPLE_SUM_BUOYANCY_DATA,
    HOOK_PROCESS_BUOYANCY,
];

/// `field *= k` (`fld; fmul; fstp`).
fn scale_f(b: Buoy, off: isize, kbits: u32) {
    let mut f = Fpu::new();
    f.fld(rf(b, off));
    f.fmul_m(cf(kbits));
    wf(b, off, f.fstp());
}

/// `field -= k`.
fn sub_f(b: Buoy, off: isize, kbits: u32) {
    let mut f = Fpu::new();
    f.fld(rf(b, off));
    f.fsub_m(cf(kbits));
    wf(b, off, f.fstp());
}

/// The boat bounding-box tweak index per model `0x1BE + i` (bytes at 0x6C2F8C; jump table 0x6C2F68).
const BOAT_CASE: [u8; 0x96] = {
    let mut t = [8u8; 0x96];
    t[0] = 0;
    t[6] = 1;
    t[7] = 2;
    t[8] = 3;
    t[0x1A] = 4;
    t[0x1B] = 5;
    t[0x26] = 6;
    t[0x2F] = 2;
    t[0x95] = 7;
    t
};

/// `cBuoyancy::PreCalcSetup(CPhysical* e, float buoyancy)` (0x6C2B90, `ret 8`): boat flag (vehicle
/// subtype 5), bounding box from the col model with the per-model tweaks of the bytes (boats by the
/// jump table, models 0x1A1 and 0x1CC, vehicle type 3), the centre offset (half extents) and its
/// normalisation by the largest component, count 1, turn point / in-water / immersion reset,
/// position, `waterLevel += m_fUnkn2`, initial position (0, 0, pos.z), buoyancy constant.
#[hook(addr = 0x6C2B90, conv = thiscall, name = "cBuoyancy::PreCalcSetup", expect = "83 EC 10 53 55")]
pub fn pre_calc_setup_port(this: Buoy, e: Ent, buoyancy: u32) {
    let boat = etype(e) == ty::VEHICLE && ru(e, 0x590) == 5;
    wb(this, PROCESSING_BOAT, boat as u8);
    let col = get_col_model(e);
    wv_bits(this, BB_MIN, rv_bits(col, 0));
    wv_bits(this, BB_MAX, rv_bits(col, 0xC));
    let (max_y, min_x, min_y, min_z, max_x, max_z) = (BB_MAX + 4, BB_MIN, BB_MIN + 4, BB_MIN + 8, BB_MAX, BB_MAX + 8);
    if boat {
        let i = (rh(e, MODEL_INDEX) as i16 as i32).wrapping_sub(0x1BE) as u32;
        let case = if i <= 0x95 { BOAT_CASE[i as usize] } else { 8 };
        match case {
            0 => {
                scale_f(this, max_y, k::K_858C20);
                scale_f(this, min_y, k::K_858C20);
            }
            1 => {
                scale_f(this, max_y, k::K_8595F0);
                scale_f(this, min_y, k::K_871660);
            }
            2 => scale_f(this, min_y, k::K_858C20),
            3 => {
                scale_f(this, max_y, k::K_859918);
                scale_f(this, min_y, k::K_87165C);
                sub_f(this, min_z, k::K_858CC4);
            }
            4 => {
                scale_f(this, max_y, k::K_858F14);
                scale_f(this, min_y, k::K_858C20);
                sub_f(this, min_z, k::K_858C24);
            }
            5 => {
                scale_f(this, max_y, k::K_859918);
                scale_f(this, min_y, k::K_858C20);
                sub_f(this, min_z, k::K_858CC4);
            }
            6 => {
                scale_f(this, max_y, k::K_858F14);
                scale_f(this, min_y, k::K_858C20);
            }
            7 => {
                scale_f(this, max_y, k::K_8595F0);
                scale_f(this, min_y, k::K_858C98);
                sub_f(this, min_z, k::K_858B1C);
            }
            _ => {
                scale_f(this, max_y, k::K_87124C);
                scale_f(this, min_y, k::K_858C20);
            }
        }
    } else {
        let model = rh(e, MODEL_INDEX);
        let mut f = Fpu::new();
        if model == 0x1A1 {
            scale_f(this, max_y, k::K_863264);
            scale_f(this, min_y, k::K_858EE8);
        } else if model == 0x1CC {
            f.fld(rf(this, max_y));
            f.fmul_m(cf(k::K_871658));
            wf(this, min_y, f.fstp());
            scale_f(this, max_y, k::K_859994);
            scale_f(this, max_x, k::K_858C24);
            scale_f(this, min_x, k::K_858C24);
            sub_f(this, min_z, k::K_858B1C);
        } else if etype(e) == ty::VEHICLE && ru(e, 0x594) == 3 {
            f.fld(rf(this, max_y));
            f.fchs();
            wf(this, min_y, f.fstp());
            f.fld(rf(this, min_z));
            f.fmul_m(cf(k::K_871654));
            wf(this, max_z, f.fstp());
            scale_f(this, min_z, k::K_862E1C);
        }
    }
    let mut f = Fpu::new();
    f.fld(rf(this, max_x));
    f.fsub_m(rf(this, min_x));
    f.fmul_m(cf(k::K_858B8C));
    let cx = f.fstp();
    f.fld(rf(this, max_y));
    f.fsub_m(rf(this, min_y));
    wf(this, 0x9C, cx);
    f.fmul_m(cf(k::K_858B8C));
    wf(this, 0xA0, f.fst());
    f.fld(rf(this, max_z));
    f.fsub_m(rf(this, min_z));
    f.fmul_m(cf(k::K_858B8C));
    let cz = f.fstp();
    wf(this, 0xA4, cz);
    let one = 0x3F80_0000u32;
    f.fld(cz);
    f.fcomp_m(cx);
    let z_gt_x = !f.jne(0x41);
    let mut z_max = false;
    if z_gt_x {
        f.fld(cz);
        f.fcomp_st(1);
        z_max = !f.jne(0x41);
    }
    if z_max {
        f.fld(cf(k::K_858624));
        f.fdiv_m(cz);
        wu(this, NORM_CENTRE + 8, one);
        f.fld(cx);
        f.fmul_st(1);
        wf(this, NORM_CENTRE, f.fstp());
        f.fmul_st(1);
        wf(this, NORM_CENTRE + 4, f.fstp());
        f.fstp_st(0);
    } else {
        f.fcom_m(cx);
        let mut y_max = false;
        if !f.jne(0x41) {
            f.fcom_m(cz);
            y_max = !f.jne(0x41);
        }
        if y_max {
            f.fdivr_m(cf(k::K_858624));
            wu(this, NORM_CENTRE + 4, one);
            f.fld(cx);
            f.fmul_st(1);
            wf(this, NORM_CENTRE, f.fstp());
            f.fmul_m(cz);
            wf(this, NORM_CENTRE + 8, f.fstp());
        } else {
            f.fld(cf(k::K_858624));
            f.fdiv_m(cx);
            wu(this, NORM_CENTRE, one);
            f.fld_st(0);
            f.fmul_st(2);
            wf(this, NORM_CENTRE + 4, f.fstp());
            f.fmul_m(cz);
            wf(this, NORM_CENTRE + 8, f.fstp());
            f.fstp_st(0);
        }
    }
    wu(this, NUM_CHECKED, one);
    wv_bits(this, TURN_POINT, [0; 3]);
    wb(this, IN_WATER, 0);
    wu(this, IMMERSION, 0);
    let p = rv_bits(pos_ptr(e), 0);
    f.fld(rf(this, 0x68));
    f.fadd_m(rf(this, WATER_LEVEL));
    wv_bits(this, 0, p);
    wf(this, WATER_LEVEL, f.fstp());
    wu(this, 0x58, 0);
    wu(this, 0x5C, 0);
    wu(this, 0x60, ru(this, 8));
    wu(this, BUOYANCY_CONST, buoyancy);
}

pub const HOOKS_2: &[HookReg] = &[HOOK_PRE_CALC_SETUP_PORT];

orig! {
    /// `cBuoyancy::FindWaterLevel(const CVector&, CVector*, tWaterLevel*)` (0x6C2810)
    pub fn find_water_level_call(b: Buoy, initial: Ent, offset: Ent, state: GPtr<u32>) = thiscall 0x6C2810 => find_water_level;
    /// `cBuoyancy::SimpleSumBuoyancyData(CVector*, tWaterLevel)` (0x6C2970, ST0)
    pub fn simple_sum_buoyancy_data_call(b: Buoy, p: Ent, state: u32) -> f64 = thiscall 0x6C2970 => simple_sum_port;
}

pub fn simple_sum_port(b: Buoy, p: Ent, state: u32) -> f64 {
    simple_sum_buoyancy_data(b, p, state).to_f64()
}

/// `cBuoyancy::SimpleCalcBuoyancy(CPhysical* e)` (0x6C3B00, `ret 4`): a 3x3 grid of points from the
/// bounding box minimum in centre-offset steps (x outer, y inner); each gets `FindWaterLevel` (with
/// `fPointVolMultiplier = 1`) and, when touching water, `SimpleSumBuoyancyData` (the grid keeps the
/// point; otherwise only z = min.z). Immersion `/= 9 * height`; splashes along the four edges where
/// both neighbouring points lie between the mid height and max.z.
#[hook(addr = 0x6C3B00, conv = thiscall, name = "cBuoyancy::SimpleCalcBuoyancy", expect = "81 EC A0 00 00 00")]
pub fn simple_calc_buoyancy_port(this: Buoy, e: Ent) {
    let mut grid = [[0u32; 3]; 9];
    let mut x = rf(this, BB_MIN);
    let mut f = Fpu::new();
    for ix in 0..3 {
        f.fld(rf(this, BB_MIN + 4));
        let mut y = f.fst();
        for iy in 0..3 {
            let yv = f.fstp();
            let g = &mut grid[ix * 3 + iy];
            *g = [0; 3];
            let mut p = [x.to_bits(), yv.to_bits(), 0u32];
            let mut state: u32 = 0;
            find_water_level_call(this, this.byte_add(0x58), lp(&mut p).cast(), lp(&mut state));
            POINT_VOL_MULTIPLIER.set(1.0);
            if state != 0 {
                simple_sum_buoyancy_data_call(this, lp(&mut p).cast(), state);
                *g = p;
            } else {
                g[2] = ru(this, BB_MIN + 8);
            }
            f.fld(y);
            f.fadd_m(rf(this, 0xA0));
            y = f.fst();
        }
        f.fstp_st(0);
        f.fld(x);
        f.fadd_m(rf(this, 0x9C));
        x = f.fstp();
    }
    f.fld(rf(this, BB_MAX + 8));
    f.fsub_m(rf(this, BB_MIN + 8));
    f.fld(cf(k::K_85EED4));
    f.fmul_st(1);
    f.fdivr_m(rf(this, IMMERSION));
    wf(this, IMMERSION, f.fstp());
    f.fmul_m(cf(k::K_858B8C));
    f.fadd_m(rf(this, BB_MIN + 8));
    let mid = f.fstp();
    let max_z = rf(this, BB_MAX + 8);
    let between = |f: &mut Fpu, z: u32| -> bool {
        f.fld(f32::from_bits(z));
        f.fcomp_m(mid);
        if f.jne(0x41) {
            return false;
        }
        f.fld(f32::from_bits(z));
        f.fcomp_m(max_z);
        !f.jp(5)
    };
    let v = |g: [u32; 3]| CVector::new(f32::from_bits(g[0]), f32::from_bits(g[1]), f32::from_bits(g[2]));
    for j in 0..2 {
        let (a, b) = (grid[3 * j], grid[3 * j + 3]);
        if between(&mut f, a[2]) && between(&mut f, b[2]) {
            add_splash_particles(this, e, v(a), v(b), CVector::new(0.0, -1.0, 0.0), 0);
        }
        let (c, d) = (grid[3 * j + 2], grid[3 * j + 5]);
        if between(&mut f, c[2]) && between(&mut f, d[2]) {
            add_splash_particles(this, e, v(c), v(d), CVector::new(0.0, 1.0, 0.0), 0);
        }
    }
    for j in 0..2 {
        let (a, b) = (grid[j], grid[j + 1]);
        if between(&mut f, a[2]) && between(&mut f, b[2]) {
            add_splash_particles(this, e, v(a), v(b), CVector::new(-1.0, 0.0, 0.0), 0);
        }
        let (c, d) = (grid[6 + j], grid[7 + j]);
        if between(&mut f, c[2]) && between(&mut f, d[2]) {
            add_splash_particles(this, e, v(c), v(d), CVector::new(1.0, 0.0, 0.0), 0);
        }
    }
}

pub const HOOKS_3: &[HookReg] = &[HOOK_SIMPLE_CALC_BUOYANCY_PORT];
