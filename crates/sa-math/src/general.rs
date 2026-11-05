//! `CGeneral` (gta-reversed `General.cpp`), ported from the 1.0 US machine code.
//!
//! gta-reversed gives the intent; the instruction sequences below follow the original bytes, because the
//! original's operation order differs from the C++ in ways that change rounding (e.g. `(-1/y)*x` where
//! gta-reversed writes `-x/y`, `(1/a)*0.5` where it writes `1/(2a)`), and in which operands are reloaded
//! from memory (that decides NaN results, see [`crate::x87`]). Every `fnstsw`/`test ah` branch is
//! reproduced with [`crate::x87::Cmp`] so NaN inputs take the original's paths.
//!
//! The `*_x87` variants return the ST0 register value; the plain variants return what a caller's
//! `fstp dword` stores.

use crate::consts as k;
#[cfg(feature = "mutants")]
use crate::x87::Cmp;
use crate::x87::X87;

/// A constant as a register value (`fld m32`).
fn c(bits: u32) -> X87 {
    X87::constant(bits)
}

/// A constant as a memory operand.
fn m(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `CGeneral::LimitAngle` (0x53CB00), degrees into [-180, 180). Returns `None` exactly when the original
/// never returns: the loop step `a - 360` (or `a + 360`) rounds back to `a` (|a| >= 2^33 under PC=24,
/// and ±inf), so the original spins forever.
pub fn limit_angle_checked(angle: f32) -> Option<X87> {
    let mut st = X87::from_f32(angle);
    // fcom 180; test ah,1; jne -> skip when below or unordered
    if !st.fcom(c(k::F180)).test_any(0x01) {
        loop {
            let next = st.fsub_m(m(k::F360));
            if next.same(st) {
                return None;
            }
            st = next;
            if st.fcom(c(k::F180)).test_any(0x01) {
                break;
            }
        }
    }
    // fcom -180; test ah,5; jp -> done when >= or unordered
    if !st.fcom(c(k::FM180)).test_parity(0x05) {
        loop {
            let next = st.fadd_m(m(k::F360));
            if next.same(st) {
                return None;
            }
            st = next;
            // jnp -> loop while below
            if st.fcom(c(k::FM180)).test_parity(0x05) {
                break;
            }
        }
    }
    Some(st)
}

/// `CGeneral::LimitAngle` (0x53CB00).
///
/// # Panics
/// Where the original never returns (see [`limit_angle_checked`]); divergence ledger entry
/// "LimitAngle hang".
pub fn limit_angle(angle: f32) -> f32 {
    limit_angle_checked(angle)
        .expect("CGeneral::LimitAngle: the original never returns for this input")
        .to_f32()
}

/// `fcom m; test ah,5; jp` taken: ST0 >= m or unordered.
fn ge_or_un(st: X87, bits: u32) -> bool {
    st.fcom(c(bits)).test_parity(0x05)
}

/// 0x53CBC0: `do st += 2PI while (st < -PI)`.
fn add_loop_body(mut st: X87) -> X87 {
    loop {
        st = st.fadd_m(m(k::TWO_PI));
        if ge_or_un(st, k::MINUS_PI) {
            return st;
        }
    }
}

/// 0x53CBB3: `while (st < -PI) st += 2PI`.
fn add_loop(st: X87) -> X87 {
    if ge_or_un(st, k::MINUS_PI) {
        st
    } else {
        add_loop_body(st)
    }
}

/// `CGeneral::LimitRadianAngle` (0x53CB50): clamp to [-25, 25], then wrap into [-PI, PI). ST0 value.
pub fn limit_radian_angle_x87(angle: f32) -> X87 {
    let a = X87::from_f32(angle);
    // fld -25; fcomp st(1): -25 vs a; test ah,0x41; jne -> a >= -25 or unordered
    if !c(k::FM25).fcom(a).test_any(0x41) {
        // st = -25, jmp into the add loop body
        return add_loop_body(c(k::FM25));
    }
    // fld 25; fcomp st(1): 25 vs a; test ah,5; jp -> 25 >= a or unordered
    let mut st = if !c(k::F25).fcom(a).test_parity(0x05) {
        c(k::F25) // jmp into the subtract loop body
    } else {
        // fcom PI; test ah,1; jne -> skip the subtract loop when below or unordered
        if a.fcom(c(k::PI)).test_any(0x01) {
            return add_loop(a);
        }
        a
    };
    loop {
        st = st.fsub_m(m(k::TWO_PI));
        if st.fcom(c(k::PI)).test_any(0x01) {
            break;
        }
    }
    add_loop(st)
}

/// `CGeneral::LimitRadianAngle` (0x53CB50).
pub fn limit_radian_angle(angle: f32) -> f32 {
    limit_radian_angle_x87(angle).to_f32()
}

/// `CGeneral::GetRadianAngleBetweenPoints(float, float, float, float)` (0x53CBE0). ST0 value.
pub fn get_radian_angle_between_points_x87(x1: f32, y1: f32, x2: f32, y2: f32) -> X87 {
    let dx = X87::from_f32(x2).fsub_m(x1);
    let mut dy = X87::from_f32(y2).fsub_m(y1);
    // fcom 0; test ah,0x44; jp -> keep dy unless it is exactly ±0
    if !dy.fcom(X87::zero()).test_parity(0x44) {
        dy = c(k::F0_0001);
    }
    let sdx = dx.fcom(X87::zero());
    let sdy = dy.fcom(X87::zero());
    // fdiv st, st(1); fld1; fpatan
    let t = dx.fdiv(dy).fpatan(X87::one());
    let h = m(k::HALF_PI);
    // test ah,0x41 on dx; jne -> dx <= 0 or unordered
    if !sdx.test_any(0x41) {
        if !sdy.test_any(0x41) {
            t.fsubr_m(h).fadd_m(h) // (C - t) + C
        } else {
            t.fadd_m(h).fsubr_m(h) // C - (t + C)
        }
    } else if !sdy.test_any(0x41) {
        t.fadd_m(h).fsubr_m(m(k::MINUS_HALF_PI)) // -C - (t + C)
    } else {
        t.fsubr_m(h).fsub_m(h) // (C - t) - C
    }
}

/// `CGeneral::GetRadianAngleBetweenPoints(float, float, float, float)` (0x53CBE0).
pub fn get_radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    get_radian_angle_between_points_x87(x1, y1, x2, y2).to_f32()
}

/// `CGeneral::GetATanOfXY` (0x53CC70): angle of (x, y) in [0, 2PI]. ST0 value. One path (x > 0, y > 0,
/// |x| >= |y|) returns the raw FPATAN result, whose 64-bit significand this port only has to 53 bits
/// (see `x87`).
pub fn get_atan_of_xy_x87(x: f32, y: f32) -> X87 {
    let (xv, yv) = (X87::from_f32(x), X87::from_f32(y));
    let zero = X87::zero();
    // test ah,0x44; jp: continue unless x == 0 (and then unless y == 0)
    if !xv.fcom(zero).test_parity(0x44) && !yv.fcom(zero).test_parity(0x44) {
        return c(k::F0);
    }
    let s1 = xv.fcom(zero);
    let ax = if s1.test_parity(0x05) { xv } else { xv.fchs() };
    let s2 = yv.fcom(zero);
    let ay = if s2.test_parity(0x05) { yv } else { yv.fchs() };
    let s3 = ax.fcom(ay);
    let y_pos = !yv.fcom(zero).test_any(0x41);
    let x_pos = !xv.fcom(zero).test_any(0x41);
    let one = X87::one();
    let m1 = c(k::MINUS_ONE);
    if !s3.test_parity(0x05) {
        // |x| < |y|
        if y_pos {
            if x_pos {
                xv.fdiv_m(y).fpatan(one).fsubr_m(m(k::HALF_PI))
            } else {
                m1.fdiv_m(y).fmul_m(x).fpatan(one).fadd_m(m(k::HALF_PI))
            }
        } else if x_pos {
            m1.fdiv_m(y).fmul_m(x).fpatan(one).fadd_m(m(k::THREE_HALF_PI))
        } else {
            xv.fdiv_m(y).fpatan(one).fsubr_m(m(k::THREE_HALF_PI))
        }
    } else if y_pos {
        if x_pos {
            yv.fdiv_m(x).fpatan(one)
        } else {
            m1.fdiv_m(x).fmul_m(y).fpatan(one).fsubr_m(m(k::PI))
        }
    } else if x_pos {
        m1.fdiv_m(x).fmul_m(y).fpatan(one).fsubr_m(m(k::TWO_PI))
    } else {
        yv.fdiv_m(x).fpatan(one).fadd_m(m(k::PI))
    }
}

/// `CGeneral::GetATanOfXY` (0x53CC70).
pub fn get_atan_of_xy(x: f32, y: f32) -> f32 {
    get_atan_of_xy_x87(x, y).to_f32()
}

/// `CGeneral::SolveQuadratic` (0x53CE30): roots of `a*x^2 + b*x + c`. `None` when the discriminant is
/// negative (the original then writes nothing and returns false). Otherwise `(x1, x2)`; the original
/// stores `*x2` first, then `*x1` (that order matters when both pointers alias). A NaN discriminant takes
/// the root path, like the original.
pub fn solve_quadratic(a: f32, b: f32, c_: f32) -> Option<(f32, f32)> {
    let (av, bv) = (X87::from_f32(a), X87::from_f32(b));
    // fld b; fmul b; fld a; fmul c; fmul 4; fsubp
    let d = bv.fmul_m(b).fsub(av.fmul_m(c_).fmul_m(m(k::F4)));
    // fcom 0; test ah,5; jp -> root path when >= 0 or unordered
    if !d.fcom(X87::zero()).test_parity(0x05) {
        return None;
    }
    let s = d.fsqrt();
    // fld 1; fdiv a
    let inv = c(k::F1).fdiv_m(a);
    // fld st1; fsub b; fmul st,st1; fmul 0.5; fstp [x2]
    let x2 = s.fsub_m(b).fmul(inv).fmul_m(m(k::F0_5));
    // fld b; fchs; fsub st,st2; fmul st,st1; fmul 0.5; fstp [x1]
    let x1 = bv.fchs().fsub(s).fmul(inv).fmul_m(m(k::F0_5));
    Some((x1.to_f32(), x2.to_f32()))
}

/// Deliberately faulty variants for the INF-12 mutation smoke test. Test-only (feature `mutants`).
#[cfg(feature = "mutants")]
pub mod mutants {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum SolveQuadraticMutant {
        /// The discriminant in plain f32 arithmetic (a naive port: overflows and flushes where the x87
        /// keeps its extended exponent range).
        F32Discriminant,
        /// The no-root branch also taken for a zero discriminant (`<= 0` instead of `< 0`).
        ZeroDiscriminantIsNoRoot,
        /// The roots stored swapped.
        SwappedRoots,
        /// gta-reversed's `1 / (2a)` instead of the original's `(1/a) * 0.5`. Not a fault: scaling by
        /// 0.5 is exact in the x87 range, so both round identically (the runner finds no difference).
        GtaReversedDenominator,
    }

    /// The injected faults (each must be caught).
    pub const ALL: [SolveQuadraticMutant; 3] = [
        SolveQuadraticMutant::F32Discriminant,
        SolveQuadraticMutant::ZeroDiscriminantIsNoRoot,
        SolveQuadraticMutant::SwappedRoots,
    ];

    /// Equivalent mutants (must *not* produce mismatches).
    pub const EQUIVALENT: [SolveQuadraticMutant; 1] = [SolveQuadraticMutant::GtaReversedDenominator];

    pub fn solve_quadratic(mt: SolveQuadraticMutant, a: f32, b: f32, c_: f32) -> Option<(f32, f32)> {
        use SolveQuadraticMutant::*;
        let (av, bv) = (X87::from_f32(a), X87::from_f32(b));
        let d = if mt == F32Discriminant {
            X87::from_f32(b * b - a * c_ * 4.0)
        } else {
            bv.fmul_m(b).fsub(av.fmul_m(c_).fmul_m(m(k::F4)))
        };
        let cmp = d.fcom(X87::zero());
        let no_root = if mt == ZeroDiscriminantIsNoRoot {
            matches!(cmp, Cmp::Less | Cmp::Equal)
        } else {
            !cmp.test_parity(0x05)
        };
        if no_root {
            return None;
        }
        let s = d.fsqrt();
        let (x1, x2) = if mt == GtaReversedDenominator {
            let denom = c(k::F1).fdiv(c(k::F2).fmul_m(a));
            (bv.fchs().fsub(s).fmul(denom), s.fsub_m(b).fmul(denom))
        } else {
            let inv = c(k::F1).fdiv_m(a);
            (
                bv.fchs().fsub(s).fmul(inv).fmul_m(m(k::F0_5)),
                s.fsub_m(b).fmul(inv).fmul_m(m(k::F0_5)),
            )
        };
        let (x1, x2) = (x1.to_f32(), x2.to_f32());
        Some(if mt == SwappedRoots { (x2, x1) } else { (x1, x2) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    #[test]
    fn limit_angle_cases() {
        assert_eq!(limit_angle(0.0), 0.0);
        assert_eq!(limit_angle(180.0), -180.0);
        assert_eq!(limit_angle(-180.0), -180.0);
        assert_eq!(limit_angle(540.0), -180.0);
        assert_eq!(limit_angle(-190.0), 170.0);
        assert!(limit_angle(f32::NAN).is_nan());
        assert!(limit_angle_checked(f32::INFINITY).is_none());
        assert!(limit_angle_checked(f32::NEG_INFINITY).is_none());
        assert!(limit_angle_checked(1e30).is_none());
        assert!(limit_angle_checked(1e6).is_some());
    }

    #[test]
    fn limit_radian_angle_cases() {
        assert_eq!(limit_radian_angle(0.0), 0.0);
        assert_eq!(limit_radian_angle(PI), PI - 2.0 * PI);
        assert_eq!(limit_radian_angle(-PI), -PI);
        assert_eq!(limit_radian_angle(100.0), limit_radian_angle(25.0));
        assert_eq!(limit_radian_angle(f32::NEG_INFINITY), limit_radian_angle(-25.0));
        assert!(limit_radian_angle(f32::NAN).is_nan());
    }

    #[test]
    fn atan_quadrants() {
        let close = |a: f32, b: f32| (a - b).abs() < 1e-5;
        assert_eq!(get_atan_of_xy(0.0, 0.0), 0.0);
        // y == 0 counts as "not above": 2PI - atan(-0), i.e. 2PI, not 0.
        assert!(close(get_atan_of_xy(1.0, 0.0), 2.0 * PI));
        assert!(close(get_atan_of_xy(1.0, 1.0), PI / 4.0));
        assert!(close(get_atan_of_xy(-1.0, 1.0), 3.0 * PI / 4.0));
        assert!(close(get_atan_of_xy(-1.0, -1.0), 5.0 * PI / 4.0));
        assert!(close(get_atan_of_xy(1.0, -1.0), 7.0 * PI / 4.0));
        assert!(close(get_atan_of_xy(0.5, 2.0), PI / 2.0 - 0.5f32.atan2(2.0)));
    }

    #[test]
    fn radian_angle_between_points() {
        let close = |a: f32, b: f32| (a - b).abs() < 1e-5;
        // Straight "north" (+y) gives PI in this convention.
        assert!(close(
            get_radian_angle_between_points(0.0, 0.0, 0.0, 1.0).abs(),
            PI
        ));
        assert!(get_radian_angle_between_points(0.0, 0.0, 0.0, 0.0).is_finite());
    }

    #[test]
    fn quadratic() {
        assert_eq!(solve_quadratic(1.0, 0.0, -4.0), Some((-2.0, 2.0)));
        assert_eq!(solve_quadratic(1.0, 0.0, 4.0), None);
        let (x1, x2) = solve_quadratic(f32::NAN, 1.0, 1.0).unwrap();
        assert!(x1.is_nan() && x2.is_nan());
    }
}
