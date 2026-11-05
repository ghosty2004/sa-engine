//! `CVector` helpers, ported from the 1.0 US machine code.

use crate::consts as k;
use crate::x87::X87;

/// `CVector::Normalise` (0x59C910, thiscall). Returns the vector the original leaves in memory.
///
/// Original: `s = (x*x + y*y) + z*z` (PC=24); when `s > 0` or `s` is NaN, `inv = 1 / sqrt(s)` and each
/// component is replaced by `f32(inv * c)`, in the order x, y, z. Otherwise (s == 0) only `x` is set to
/// 1.0; y and z are left as they were (gta-reversed agrees).
pub fn normalise(v: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = v.map(X87::from_f32);
    let s = x.fmul(x).fadd(y.fmul(y)).fadd(z.fmul(z));
    // fcom 0; test ah,0x41; jp -> root path when > 0 or unordered
    if !s.fcom(X87::zero()).test_parity(0x41) {
        return [f32::from_bits(k::F1), v[1], v[2]];
    }
    // fsqrt; fdivr 1.0; then per component `fld st0; fmul [ecx+i]; fstp [ecx+i]` (memory operand)
    let inv = s.fsqrt().fdivr_m(f32::from_bits(k::F1));
    [
        inv.fmul_m(v[0]).to_f32(),
        inv.fmul_m(v[1]).to_f32(),
        inv.fmul_m(v[2]).to_f32(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_cases() {
        assert_eq!(normalise([3.0, 0.0, 4.0]), [0.6, 0.0, 0.8]);
        assert_eq!(normalise([0.0, 5.0, 0.0]), [0.0, 1.0, 0.0]);
        assert_eq!(normalise([0.0, -0.0, 0.0]), [1.0, -0.0, 0.0]);
        // x87 range: the squares of 1e30 do not overflow.
        let n = normalise([1e30, 0.0, 0.0]);
        assert_eq!(n, [1.0, 0.0, 0.0]);
        // Underflowing squares: 1e-30^2 is an f32 zero but not an x87 zero.
        assert_eq!(normalise([1e-30, 0.0, 0.0]), [1.0, 0.0, 0.0]);
    }
}
