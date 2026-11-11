//! PORT-CAM: `CCurves` (pure x87 math, cdecl, results in ST0). Ported from the 1.0 US bytes;
//! gta-reversed `Curves.cpp` as a reading aid. Literal x87 sequences on [`Fpu`].

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{Fpu, k, rd};

/// 0.0 (0x858B50), -1.0 (0x858C1C), 1/3 (0x8594EC), 0.7 (0x858CB0), 1/0.7 (0x8594E8), 1.0 (0x858624),
/// 5.0 (0x858C80), 1e-5 (0x858C14), pi (0x858CB8), 2 pi (0x858CBC), 0.5 (0x858B8C), 1/(2 pi) (0x8594F0)
const ZERO: u32 = 0x0000_0000;
const NEG_ONE: u32 = 0xBF80_0000;
const THIRD: u32 = 0x3EAA_AAAA;
const P07: u32 = 0x3F33_3333;
const INV_P07: u32 = 0x3FB6_DB6E;
const ONE: u32 = 0x3F80_0000;
const FIVE: u32 = 0x40A0_0000;
const EPS: u32 = 0x3727_C5AC;
const PI: u32 = 0x4049_0FDB;
const TWO_PI: u32 = 0x40C9_0FDB;
const HALF: u32 = 0x3F00_0000;
const INV_TWO_PI: u32 = 0x3E22_F983;

/// This module's globals (none).
pub const GLOBALS: &[GlobalDesc] = &[];

/// `CCollision::DistToMathematicalLine2D` through its ST0 (sa-world's port returns the x87 value).
#[cfg_attr(sa_phase_a, allow(dead_code))]
fn dist_to_mathematical_line_2d_abi(x: f32, y: f32, dx: f32, dy: f32, px: f32, py: f32) -> f64 {
    sa_world::collision::prim::dist_to_mathematical_line_2d(x, y, dx, dy, px, py).to_f64()
}

orig! {
    /// `CCollision::DistToMathematicalLine2D(x, y, dx, dy, px, py)` (0x412A30, ST0)
    pub fn dist_to_mathematical_line_2d(x: f32, y: f32, dx: f32, dy: f32, px: f32, py: f32) -> f64 = cdecl 0x412A30 => dist_to_mathematical_line_2d_abi;
    /// `CCurves::CalcSpeedVariationInBend` (0x43C660, ST0)
    pub fn calc_speed_variation_in_bend_call(start: GPtr<u8>, end: GPtr<u8>, sdx: f32, sdy: f32, edx: f32, edy: f32) -> f64 = cdecl 0x43C660 => calc_speed_variation_in_bend_abi;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn calc_speed_variation_in_bend_abi(
    start: GPtr<u8>,
    end: GPtr<u8>,
    sdx: f32,
    sdy: f32,
    edx: f32,
    edy: f32,
) -> f64 {
    calc_speed_variation_in_bend(start, end, sdx, sdy, edx, edy).to_f64()
}

/// `fld -1.0; fdiv st, st(2); fmulp st(1), st`: `n * (-1 / st1)` with `n` in ST0, the cross product
/// in ST1.
fn scale_by_neg_inv_cross(f: &mut Fpu) {
    f.fld(k(NEG_ONE));
    f.fdiv_st(2);
    f.fmulp(1);
}

/// `CCurves::DistForLineToCrossOtherLine(lineBase, lineDir, otherBase, otherDir)` (0x43C610, all
/// CVector2D by value): `c = ldx*ody - ldy*odx`; `c == 0` (ordered) gives -1.0, else
/// `((lbx-obx)*ody - (lby-oby)*odx) * (-1/c)` (registers only).
#[hook(addr = 0x43C610, conv = cdecl, name = "CCurves::DistForLineToCrossOtherLine", expect = "D9 44 24 0C D8 4C 24 20")]
#[allow(clippy::too_many_arguments)]
pub fn dist_for_line_to_cross_other_line(
    lbx: f32,
    lby: f32,
    ldx: f32,
    ldy: f32,
    obx: f32,
    oby: f32,
    odx: f32,
    ody: f32,
) -> X87 {
    let mut f = Fpu::new();
    f.fld(ldx);
    f.fmul_m(ody);
    f.fld(ldy);
    f.fmul_m(odx);
    f.fsubp(1);
    f.fcom_m(k(ZERO));
    if !f.jp(0x44) {
        f.fstp_st(0);
        f.fld(k(NEG_ONE));
        return f.pop();
    }
    f.fld(lbx);
    f.fsub_m(obx);
    f.fmul_m(ody);
    f.fld(lby);
    f.fsub_m(oby);
    f.fmul_m(odx);
    f.fsubp(1);
    scale_by_neg_inv_cross(&mut f);
    f.fstp_st(1);
    f.pop()
}

/// `CCurves::CalcCorrectedDist(curr, total, speedVariation, float* interp)` (0x43C880): `total < 1e-5`
/// (ordered) gives 0.0 and `*interp = 0.5`; else `t = curr/total`, `*interp = 0.5 - cos(pi t) * 0.5`
/// (stored f32), result `sin(2 pi t) * (total / (2 pi)) * var + ((1 - 2 var) + 1) * 0.5 * curr`.
#[hook(addr = 0x43C880, conv = cdecl, name = "CCurves::CalcCorrectedDist", expect = "D9 44 24 08 D8 1D 14 8C 85 00")]
pub fn calc_corrected_dist(curr: f32, total: f32, var: f32, interp: GPtr<f32>) -> X87 {
    let mut f = Fpu::new();
    f.fld(total);
    f.fcomp_m(k(EPS));
    if !f.jp(0x05) {
        interp.write(k(HALF));
        f.fld(k(ZERO));
        return f.pop();
    }
    f.fld(curr);
    f.fdiv_m(total);
    f.fld(k(PI));
    f.fmul_st(1);
    let v = f.pop();
    f.push(v.fcos());
    f.fmul_m(k(HALF));
    f.fsubr_m(k(HALF));
    interp.write(f.fstp());
    f.fmul_m(k(TWO_PI));
    let v = f.pop();
    f.push(v.fsin());
    f.fld(total);
    f.fmul_m(k(INV_TWO_PI));
    f.fmulp(1);
    f.fmul_m(var);
    f.fld(var);
    f.fadd_st(0);
    f.fsubr_m(k(ONE));
    f.fadd_m(k(ONE));
    f.fmul_m(k(HALF));
    f.fmul_m(curr);
    f.faddp(1);
    f.pop()
}

/// `CCurves::CalcSpeedVariationInBend(const CVector& start, const CVector& end, CVector2D startDir,
/// CVector2D endDir)` (0x43C660): `d = sd . ed` (stored f32); `d <= 0` gives 1/3; `d <= 0.7` (or NaN)
/// gives `(1 - d * (1/0.7)) * (1/3)`; else `DistToMathematicalLine2D(end, endDir, start) /
/// |start - end|(2D) * (1/3)`.
#[hook(addr = 0x43C660, conv = cdecl, name = "CCurves::CalcSpeedVariationInBend", expect = "D9 44 24 0C D8 4C 24 14")]
pub fn calc_speed_variation_in_bend(
    start: GPtr<u8>,
    end: GPtr<u8>,
    sdx: f32,
    sdy: f32,
    edx: f32,
    edy: f32,
) -> X87 {
    let mut f = Fpu::new();
    f.fld(sdx);
    f.fmul_m(edx);
    f.fld(sdy);
    f.fmul_m(edy);
    f.faddp(1);
    let d = f.fst();
    f.fcomp_m(k(ZERO));
    if !f.jp(0x41) {
        f.fld(k(THIRD));
        return f.pop();
    }
    f.fld(d);
    f.fcomp_m(k(P07));
    if f.jne(0x41) {
        f.fld(d);
        f.fmul_m(k(INV_P07));
        f.fsubr_m(k(ONE));
        f.fmul_m(k(THIRD));
        return f.pop();
    }
    let r = dist_to_mathematical_line_2d(rd(end, 0), rd(end, 4), edx, edy, rd(start, 0), rd(start, 4));
    f.push(X87::from_f64_exact(r));
    f.fld(rd(start, 0));
    f.fsub_m(rd(end, 0));
    f.fld(rd(start, 4));
    f.fsub_m(rd(end, 4));
    hypot_st1_st0(&mut f);
    f.fdivp(1);
    f.fmul_m(k(THIRD));
    f.pop()
}

/// `fld st(1); fmulp st(2); fld st0; fmul st, st1; faddp st(2); fxch st1; fsqrt; fstp st(1)`: with
/// `b` in ST0 and `a` in ST1, leaves `sqrt(a*a + b*b)` in ST0 (the two popped).
fn hypot_st1_st0(f: &mut Fpu) {
    f.fld_st(1);
    f.fmulp(2);
    f.fld_st(0);
    f.fmul_st(1);
    f.faddp(2);
    f.fxch(1);
    f.fsqrt();
    f.fstp_st(1);
}

/// `CCurves::CalcSpeedScaleFactor(start, end, sdx, sdy, edx, edy)` (0x43C710): `v =
/// CalcSpeedVariationInBend(..)`; `d1`, `d2` = the distances along each line to the other one (as
/// `DistForLineToCrossOtherLine`, -1.0 when parallel, stored f32 in the argument slots); both `> 0`:
/// `m = min(d1 < d2 ? d1 : d2, 5)` (5 unless `5 < m` is false), result `2m + (d2 - m) + (d1 - m)`;
/// else `|start - end|(2D) / (1 - v) + 0.0 + 0.0`.
#[hook(addr = 0x43C710, conv = cdecl, name = "CCurves::CalcSpeedScaleFactor", expect = "8B 44 24 18 8B 4C 24 14")]
pub fn calc_speed_scale_factor(
    start: GPtr<u8>,
    end: GPtr<u8>,
    sdx: f32,
    sdy: f32,
    edx: f32,
    edy: f32,
) -> X87 {
    let mut f = Fpu::new();
    let v = calc_speed_variation_in_bend_call(start, end, sdx, sdy, edx, edy);
    f.push(X87::from_f64_exact(v));
    f.fld(sdx);
    f.fmul_m(edy);
    f.fld(sdy);
    f.fmul_m(edx);
    // ST0 = A (sdy*edx), ST1 = B (sdx*edy), ST2 = v
    f.fld_st(1);
    f.fsub_st(1);
    f.fcom_m(k(ZERO));
    let d1 = if !f.jp(0x44) {
        k(NEG_ONE)
    } else {
        f.fld(rd(start, 0));
        f.fsub_m(rd(end, 0));
        f.fmul_m(edy);
        f.fld(rd(start, 4));
        f.fsub_m(rd(end, 4));
        f.fmul_m(edx);
        f.fsubp(1);
        scale_by_neg_inv_cross(&mut f);
        f.fstp()
    };
    f.fstp_st(0);
    f.fchs();
    f.fxch(1);
    f.fchs();
    f.fsubp(1);
    f.fcom_m(k(ZERO));
    let d2 = if !f.jp(0x44) {
        k(NEG_ONE)
    } else {
        f.fld(rd(end, 0));
        f.fsub_m(rd(start, 0));
        f.fmul_m(sdy);
        f.fld(rd(end, 4));
        f.fsub_m(rd(start, 4));
        f.fmul_m(sdx);
        f.fsubp(1);
        scale_by_neg_inv_cross(&mut f);
        f.fstp()
    };
    f.fstp_st(0);
    // ST0 = v
    let pos = |f: &mut Fpu, d: f32| {
        f.fld(d);
        f.fcomp_m(k(ZERO));
        !f.jne(0x41)
    };
    if pos(&mut f, d1) && pos(&mut f, d2) {
        f.fstp_st(0);
        f.fld(d1);
        f.fcomp_m(d2);
        let m = if f.jp(0x05) { d2 } else { d1 };
        f.fld(m);
        f.fld(k(FIVE));
        f.fcomp_st(1);
        if !f.jp(0x05) {
            f.fstp_st(0);
            f.fld(k(FIVE));
        }
        f.fld(d1);
        f.fsub_st(1);
        let r1 = f.fstp();
        f.fld(d2);
        f.fsub_st(1);
        let r2 = f.fstp();
        f.fadd_st(0);
        f.fadd_m(r2);
        f.fadd_m(r1);
        return f.pop();
    }
    f.fld(rd(start, 0));
    f.fsub_m(rd(end, 0));
    f.fld(rd(start, 4));
    f.fsub_m(rd(end, 4));
    hypot_st1_st0(&mut f);
    f.fld(k(ONE));
    f.fsub_st(2);
    f.fdivp(1);
    f.fstp_st(1);
    f.fadd_m(k(ZERO));
    f.fadd_m(k(ZERO));
    f.pop()
}

pub const HOOKS: &[HookReg] = &[
    HOOK_DIST_FOR_LINE_TO_CROSS_OTHER_LINE,
    HOOK_CALC_CORRECTED_DIST,
    HOOK_CALC_SPEED_VARIATION_IN_BEND,
    HOOK_CALC_SPEED_SCALE_FACTOR,
];
