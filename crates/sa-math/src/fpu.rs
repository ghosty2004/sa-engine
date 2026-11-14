//! x87 load/store shorthands shared by every port crate (ADR-003 §3.2: one definition, in the lowest
//! layer that owns the concept). Each is one x87 instruction of the original, expressed with [`X87`];
//! ports import them (`use sa_math::fpu::x;`, or `as` a local name) instead of defining their own.

use crate::x87::X87;

/// `fld dword`: an f32 onto the x87 stack (exact).
#[inline]
pub fn x(v: f32) -> X87 {
    X87::from_f32(v)
}

/// `fild dword`: an int32 onto the x87 stack (exact).
#[inline]
pub fn xi(v: i32) -> X87 {
    X87::from_i32(v)
}

/// `fld dword [const]` of an `.rdata` constant given by its f32 bits.
#[inline]
pub fn xc(bits: u32) -> X87 {
    X87::constant(bits)
}

/// An ST0 value returned by an `orig!` call (declared `f64`), back as an [`X87`] (exact).
#[inline]
pub fn st0(v: f64) -> X87 {
    X87::from_f64_exact(v)
}

/// `fstp dword`: the f32 a store of ST0 writes.
#[inline]
pub fn f32_of(v: X87) -> f32 {
    v.to_f32()
}

/// `_ftol` (0x821B40) of ST0, low 32 bits (EAX).
#[inline]
pub fn ftol32(v: X87) -> i32 {
    v.ftol() as u32 as i32
}

/// An f32 given by its bits (an `.rdata` constant as the original stores it).
#[inline]
pub const fn fbits(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `fabs`: clears the sign bit (NaN payload kept).
#[inline]
pub fn fabs(v: X87) -> X87 {
    X87::from_f64_exact(v.to_f64().abs())
}

/// CRT `floor` on a double (exact).
pub fn floor_f64(v: f64) -> f64 {
    if v.is_nan() || v.is_infinite() || v.abs() >= 4_503_599_627_370_496.0 {
        return v;
    }
    let t = v as i64 as f64;
    if t > v { t - 1.0 } else { t }
}

/// `fcom b; fnstsw ax; test ah, mask; jne`: taken when any of the masked condition bits is set.
#[inline]
pub fn fcom_any(a: X87, b: X87, mask: u8) -> bool {
    a.fcom(b).test_any(mask)
}

/// `fcom b; fnstsw ax; test ah, mask; jp`: taken when the masked condition bits have even parity.
#[inline]
pub fn fcom_parity(a: X87, b: X87, mask: u8) -> bool {
    a.fcom(b).test_parity(mask)
}

/// `test ah, 41h; jne` after `fcom`: `a <= b`, or unordered (C0 or C3 set).
#[inline]
pub fn le_or_nan(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}

/// `test ah, 41h; je` after `fcom`: `a > b` (ordered).
#[inline]
pub fn gt(a: X87, b: X87) -> bool {
    !a.fcom(b).test_any(0x41)
}

/// `test ah, 5; jp` after `fcom`: `a >= b`, or unordered (C0 and C2 equal).
#[inline]
pub fn ge_or_nan(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(5)
}

/// `test ah, 5; jnp` after `fcom`: `a < b` (ordered).
#[inline]
pub fn lt(a: X87, b: X87) -> bool {
    !a.fcom(b).test_parity(5)
}

pub fn fcom_jnp(a: X87, b: X87, m: u8) -> bool {
    !a.fcom(b).test_parity(m)
}


pub fn fcom_ne(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x44)
}


pub fn hypot_x87(ax: X87, ay: X87) -> X87 {
    ax.fmul(ax).fadd(ay.fmul(ay)).fsqrt()
}


pub fn fstp_bits(v: X87) -> u32 {
    v.to_f32().to_bits()
}


pub fn fld_fst(v: f32) -> f32 {
    if v.is_nan() {
        f32::from_bits(v.to_bits() | 0x40_0000)
    } else {
        v
    }
}


pub fn fcom_c0(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorthands_are_the_x87_operations() {
        assert_eq!(x(1.5).to_f32(), 1.5);
        assert_eq!(xi(-3).to_f32(), -3.0);
        assert_eq!(xc(0x3F80_0000).to_f32(), 1.0);
        assert_eq!(st0(0.25).to_f32(), 0.25);
        assert_eq!(f32_of(x(2.0)), 2.0);
        assert_eq!(ftol32(x(-2.75)), -2);
        assert_eq!(fbits(0x4000_0000), 2.0);
        assert_eq!(fabs(x(-2.5)).to_f32(), 2.5);
        assert_eq!((floor_f64(-1.5), floor_f64(2.0), floor_f64(2.7)), (-2.0, 2.0, 2.0));
        let (one, two, nan) = (x(1.0), x(2.0), x(f32::NAN));
        assert!(le_or_nan(one, two) && le_or_nan(one, one) && le_or_nan(nan, one) && !le_or_nan(two, one));
        assert!(gt(two, one) && !gt(one, one) && !gt(nan, one));
        assert!(ge_or_nan(two, one) && ge_or_nan(one, one) && ge_or_nan(nan, one) && !ge_or_nan(one, two));
        assert!(lt(one, two) && !lt(one, one) && !lt(nan, one));
        assert_eq!(fcom_any(one, two, 0x41), le_or_nan(one, two));
        assert_eq!(fcom_parity(one, two, 5), ge_or_nan(one, two));
    }
}
