//! x87 arithmetic as `gta_sa.exe` 1.0 US runs it.
//!
//! The game runs with the x87 control word 0x007F: all exceptions masked, round to nearest, and
//! **precision control = 24 bits** (set by D3D9, measured in-game by INF-3). Under PC=24 every
//! FADD/FSUB/FMUL/FDIV/FSQRT result is rounded to a 24-bit significand, but the exponent keeps the full
//! 15-bit extended range. So x87 code is *not* f32 code: `3e38 + 3e38` is a finite register value, and a
//! result that would be an f32 subnormal keeps all 24 bits. Plain `f32` arithmetic gets both wrong.
//!
//! [`X87`] models one register value as an `f64`:
//! - finite values hold at most 24 significant bits (an f64 holds them exactly);
//! - each arithmetic op is computed in f64 and then rounded to 24 bits. The x87 rounds the *exact*
//!   result once; to do the same for operands wider than 24 bits (an FPATAN result), the f64 result's
//!   exact error (TwoSum, or `fma` for *, / and sqrt) breaks the ties the f64 rounding created. For
//!   24-bit operands this is not even needed (53 >= 2*24 + 2, Figueroa 1995);
//! - the f64 exponent range (±1022) covers every value reachable from f32 inputs through a few ops
//!   (f32 products/quotients stay within ±2^300). Values outside it are not modelled;
//! - NaNs are handled explicitly by bit manipulation (never by host float arithmetic, whose NaN results
//!   are platform dependent, e.g. on wasm). The rules, measured on the real CPU (`sa-oracle` NaN matrix,
//!   see `docs/re/l1-harness.md`):
//!   - `fld m32` of a signalling NaN quiets it and keeps sign and payload;
//!   - two quiet NaN operands: the one with the larger significand; equal significands: the positive one;
//!   - one NaN operand: that NaN (quieted);
//!   - a *memory* operand that is a signalling NaN loses to a quiet NaN in the register; against a
//!     non-NaN it gives itself, quieted. This is why the API has separate memory-operand forms
//!     (`fadd_m`, `fsub_m`, ...): the original code reloads its float arguments from the stack, so a
//!     signalling-NaN argument behaves differently as a register and as a memory operand;
//!   - invalid operations (inf - inf, 0 * inf, 0/0, inf/inf, sqrt of a negative) give the x87
//!     "indefinite" (negative quiet NaN, `0xFFC00000` when stored as f32).
//!
//! Transcendentals (FPATAN) are *not* affected by PC on the real CPU and return a 64-bit significand.
//! [`X87::fpatan`] computes them in f64 (53 bits, via `libm`, deterministic on every target). That is a
//! documented approximation: the next PC=24 op rounds the value to 24 bits, and the f64/64-bit
//! difference changes that rounding only when the exact value lies within 2^-29 ulp of a rounding
//! boundary. A function that *returns* a raw FPATAN result leaves 64 bits in ST0, which this type cannot
//! reproduce (see `GetATanOfXY` in `docs/re/l1-harness.md`).

use core::cmp::Ordering;

/// f64 bits of the x87 indefinite QNaN (sign 1, quiet bit only).
const INDEFINITE: u64 = 0xFFF8_0000_0000_0000;
const QUIET: u64 = 1 << 51;
const FRAC: u64 = (1 << 52) - 1;

/// One x87 register value under precision control 24 (see the module docs).
#[derive(Clone, Copy, Debug)]
pub struct X87(f64);

/// Result of an x87 compare (FCOM/FCOMP): condition codes C3/C2/C0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmp {
    /// C3=C2=C0=0
    Greater,
    /// C0=1
    Less,
    /// C3=1
    Equal,
    /// C3=C2=C0=1
    Unordered,
}

impl Cmp {
    /// `C0` (the "below" bit): set for Less and Unordered.
    pub fn c0(self) -> bool {
        matches!(self, Cmp::Less | Cmp::Unordered)
    }
    /// `C2`: set only for Unordered.
    pub fn c2(self) -> bool {
        self == Cmp::Unordered
    }
    /// `C3`: set for Equal and Unordered.
    pub fn c3(self) -> bool {
        matches!(self, Cmp::Equal | Cmp::Unordered)
    }
    /// The bits of `AH` after `fnstsw ax` (C0 = 0x01, C2 = 0x04, C3 = 0x40).
    pub fn ah(self) -> u8 {
        (self.c0() as u8) | ((self.c2() as u8) << 2) | ((self.c3() as u8) << 6)
    }
    /// `test ah, mask` then `jp`: x86 PF is set when the low byte of the result has an even number of
    /// set bits. MSVC uses it with masks 0x05 / 0x41 / 0x44 to branch on two condition bits at once.
    pub fn test_parity(self, mask: u8) -> bool {
        (self.ah() & mask).count_ones().is_multiple_of(2)
    }
    /// `test ah, mask` then `jne` / `je`: true when any masked bit is set (the `jne` branch).
    pub fn test_any(self, mask: u8) -> bool {
        self.ah() & mask != 0
    }
}

/// Rounds a finite f64 to a 24-bit significand, round-to-nearest-even, without any exponent limit
/// (the x87 PC=24 rounding). f64 subnormals are outside the modelled range and are returned unchanged.
pub fn round24(v: f64) -> f64 {
    let bits = v.to_bits();
    let exp = (bits >> 52) & 0x7FF;
    if exp == 0 || exp == 0x7FF {
        return v;
    }
    const DROP: u32 = 52 - 23;
    let mask = (1u64 << DROP) - 1;
    let half = 1u64 << (DROP - 1);
    let low = bits & mask;
    let mut base = bits & !mask;
    if low > half || (low == half && (base >> DROP) & 1 == 1) {
        base += 1 << DROP; // a carry into the exponent is the correct result
    }
    f64::from_bits(base)
}

/// Rounds the exact value `s + e` to 24 bits, where `s` is the f64-rounded result of an operation and
/// `e` its exact error (or any value with the error's sign that is non-zero exactly when the error is).
/// The f64 rounding only loses information when `s` lands exactly on a 24-bit tie; the error then
/// decides the direction, as the x87 (which rounds the exact result once) does.
fn round24_exact(s: f64, e: f64) -> f64 {
    let bits = s.to_bits();
    let exp = (bits >> 52) & 0x7FF;
    if e == 0.0 || exp == 0 || exp == 0x7FF {
        return round24(s);
    }
    const DROP: u32 = 52 - 23;
    let mask = (1u64 << DROP) - 1;
    if bits & mask != 1 << (DROP - 1) {
        return round24(s);
    }
    let base = bits & !mask;
    // tie in s: away from zero when the exact value is larger in magnitude
    if (e > 0.0) == (s > 0.0) {
        f64::from_bits(base + (1 << DROP))
    } else {
        f64::from_bits(base)
    }
}

/// Exact error of `a + b` (Knuth's TwoSum).
fn two_sum_err(a: f64, b: f64, s: f64) -> f64 {
    let bb = s - a;
    let aa = s - bb;
    (a - aa) + (b - bb)
}

/// True for a signalling NaN f32 (quiet bit clear).
pub fn is_snan_f32(v: f32) -> bool {
    let b = v.to_bits();
    b & 0x7F80_0000 == 0x7F80_0000 && b & 0x7F_FFFF != 0 && b & 0x40_0000 == 0
}

/// The second operand of an arithmetic instruction.
#[derive(Clone, Copy, Debug)]
enum Src {
    Reg(X87),
    /// `m32` operand (keeps its signalling-ness, see the module docs)
    Mem(f32),
}

#[derive(Clone, Copy)]
enum Op {
    Add,
    Sub,
    SubR,
    Mul,
    Div,
    DivR,
}

impl X87 {
    /// `fld dword`: exact for every f32; a signalling NaN is quieted (payload kept).
    pub fn from_f32(v: f32) -> X87 {
        let b = v.to_bits();
        if v.is_nan() {
            let sign = ((b >> 31) as u64) << 63;
            let frac = ((b & 0x7F_FFFF) as u64) << 29;
            return X87(f64::from_bits(sign | (0x7FF << 52) | frac | QUIET));
        }
        X87(f64::from(v))
    }

    /// A constant loaded from the exe's `.rdata` (always finite).
    pub fn constant(bits: u32) -> X87 {
        X87::from_f32(f32::from_bits(bits))
    }

    /// `fld1`
    pub fn one() -> X87 {
        X87(1.0)
    }

    /// `fldz`
    pub fn zero() -> X87 {
        X87(0.0)
    }

    /// `fst dword`: round to f32 (nearest-even, f32 subnormals and overflow per IEEE); NaN keeps sign
    /// and the top 23 significand bits.
    pub fn to_f32(self) -> f32 {
        let b = self.0.to_bits();
        if self.0.is_nan() {
            let sign = ((b >> 63) as u32) << 31;
            let frac = ((b & FRAC) >> 29) as u32;
            return f32::from_bits(sign | 0x7F80_0000 | frac);
        }
        self.0 as f32
    }

    /// The register value as f64 (exact; NaN payload in the f64 layout).
    pub fn to_f64(self) -> f64 {
        self.0
    }

    /// The 80-bit extended encoding of this value (what `fstp tbyte` writes).
    pub fn to_ext(self) -> [u8; 10] {
        let b = self.0.to_bits();
        let sign = ((b >> 63) as u16) << 15;
        let exp = ((b >> 52) & 0x7FF) as u16;
        let frac = b & FRAC;
        let (e, m): (u16, u64) = if exp == 0x7FF {
            (0x7FFF, (1 << 63) | (frac << 11))
        } else if exp == 0 {
            if frac != 0 {
                // f64 subnormal: outside the modelled range, but encode it exactly anyway.
                let lz = frac.leading_zeros() - 11; // shift to put the top bit at bit 52
                let m = frac << (lz + 11);
                (16383 - 1022 - lz as u16, m)
            } else {
                (0, 0)
            }
        } else {
            (exp + (16383 - 1023), (1 << 63) | (frac << 11))
        };
        let mut out = [0u8; 10];
        out[..8].copy_from_slice(&m.to_le_bytes());
        out[8..].copy_from_slice(&(sign | e).to_le_bytes());
        out
    }

    pub fn is_nan(self) -> bool {
        self.0.is_nan()
    }

    fn quiet(self) -> X87 {
        X87(f64::from_bits(self.0.to_bits() | QUIET))
    }

    /// Two quiet NaNs: larger significand, ties to the positive one.
    fn larger_nan(a: X87, b: X87) -> X87 {
        let (ba, bb) = (a.0.to_bits(), b.0.to_bits());
        let (fa, fb) = (ba & FRAC, bb & FRAC);
        if fa > fb || (fa == fb && ba >> 63 == 0) {
            a
        } else {
            b
        }
    }

    /// The NaN result of `self op src`, if any operand is a NaN.
    fn nan_of(self, src: Src) -> Option<X87> {
        let (b, b_snan) = match src {
            Src::Reg(x) => (x, false),
            Src::Mem(m) => (X87::from_f32(m), is_snan_f32(m)),
        };
        match (self.is_nan(), b.is_nan()) {
            (false, false) => None,
            (true, false) => Some(self.quiet()),
            (false, true) => Some(b),
            (true, true) if b_snan => Some(self.quiet()),
            (true, true) => Some(X87::larger_nan(self.quiet(), b)),
        }
    }

    fn indefinite() -> X87 {
        X87(f64::from_bits(INDEFINITE))
    }

    fn arith(self, op: Op, src: Src) -> X87 {
        if let Some(n) = self.nan_of(src) {
            return n;
        }
        let b = match src {
            Src::Reg(x) => x.0,
            Src::Mem(m) => f64::from(m),
        };
        let (x, y) = match op {
            Op::SubR | Op::DivR => (b, self.0),
            _ => (self.0, b),
        };
        let finite = x.is_finite() && y.is_finite();
        let (r, e) = match op {
            Op::Add | Op::Sub | Op::SubR => {
                let y = if matches!(op, Op::Add) { y } else { -y };
                if x.is_infinite() && y.is_infinite() && x.is_sign_negative() != y.is_sign_negative() {
                    return X87::indefinite();
                }
                let s = x + y;
                (
                    s,
                    if finite && s.is_finite() {
                        two_sum_err(x, y, s)
                    } else {
                        0.0
                    },
                )
            }
            Op::Mul => {
                if (x == 0.0 && y.is_infinite()) || (x.is_infinite() && y == 0.0) {
                    return X87::indefinite();
                }
                let p = x * y;
                (
                    p,
                    if finite && p.is_finite() {
                        libm::fma(x, y, -p)
                    } else {
                        0.0
                    },
                )
            }
            Op::Div | Op::DivR => {
                if (x == 0.0 && y == 0.0) || (x.is_infinite() && y.is_infinite()) {
                    return X87::indefinite();
                }
                let q = x / y;
                // rem = x - q*y exactly; the true quotient is q + rem/y (only its sign is needed)
                let rem = if finite && q.is_finite() && y != 0.0 {
                    libm::fma(-q, y, x)
                } else {
                    0.0
                };
                let e = if y > 0.0 { rem } else { -rem };
                (q, e)
            }
        };
        X87(round24_exact(r, e))
    }

    /// FADD with a register operand (PC=24).
    pub fn fadd(self, o: X87) -> X87 {
        self.arith(Op::Add, Src::Reg(o))
    }
    /// FSUB: `self - o`, register operand.
    pub fn fsub(self, o: X87) -> X87 {
        self.arith(Op::Sub, Src::Reg(o))
    }
    /// FSUBR: `o - self`, register operand.
    pub fn fsubr(self, o: X87) -> X87 {
        self.arith(Op::SubR, Src::Reg(o))
    }
    /// FMUL, register operand.
    pub fn fmul(self, o: X87) -> X87 {
        self.arith(Op::Mul, Src::Reg(o))
    }
    /// FDIV: `self / o`, register operand. x/0 is ±inf (masked ZE).
    pub fn fdiv(self, o: X87) -> X87 {
        self.arith(Op::Div, Src::Reg(o))
    }
    /// FDIVR: `o / self`, register operand.
    pub fn fdivr(self, o: X87) -> X87 {
        self.arith(Op::DivR, Src::Reg(o))
    }

    /// `fadd m32`
    pub fn fadd_m(self, m: f32) -> X87 {
        self.arith(Op::Add, Src::Mem(m))
    }
    /// `fsub m32`: `self - m`
    pub fn fsub_m(self, m: f32) -> X87 {
        self.arith(Op::Sub, Src::Mem(m))
    }
    /// `fsubr m32`: `m - self`
    pub fn fsubr_m(self, m: f32) -> X87 {
        self.arith(Op::SubR, Src::Mem(m))
    }
    /// `fmul m32`
    pub fn fmul_m(self, m: f32) -> X87 {
        self.arith(Op::Mul, Src::Mem(m))
    }
    /// `fdiv m32`: `self / m`
    pub fn fdiv_m(self, m: f32) -> X87 {
        self.arith(Op::Div, Src::Mem(m))
    }
    /// `fdivr m32`: `m / self`
    pub fn fdivr_m(self, m: f32) -> X87 {
        self.arith(Op::DivR, Src::Mem(m))
    }

    /// FSQRT (PC=24): sqrt(-0) = -0, negative non-zero gives indefinite.
    pub fn fsqrt(self) -> X87 {
        if self.is_nan() {
            return self.quiet();
        }
        if self.0 < 0.0 {
            return X87::indefinite();
        }
        let s = libm::sqrt(self.0);
        let e = if s.is_finite() {
            libm::fma(-s, s, self.0)
        } else {
            0.0
        };
        X87(round24_exact(s, e))
    }

    /// FCHS (also flips the sign of a NaN).
    pub fn fchs(self) -> X87 {
        X87(f64::from_bits(self.0.to_bits() ^ (1 << 63)))
    }

    /// FPATAN with ST1 = `y` (self) and ST0 = `x`: atan2(y, x), *not* rounded to 24 bits (PC does not
    /// apply). Computed in f64, see the module docs for the precision caveat.
    pub fn fpatan(self, x: X87) -> X87 {
        if let Some(n) = self.nan_of(Src::Reg(x)) {
            return n;
        }
        X87(libm::atan2(self.0, x.0))
    }

    /// FCOM/FCOMP: compare `self` (ST0) with `o`.
    pub fn fcom(self, o: X87) -> Cmp {
        match self.0.partial_cmp(&o.0) {
            None => Cmp::Unordered,
            Some(Ordering::Less) => Cmp::Less,
            Some(Ordering::Equal) => Cmp::Equal,
            Some(Ordering::Greater) => Cmp::Greater,
        }
    }

    /// Bit-exact identity (NaN payloads included).
    pub fn same(self, o: X87) -> bool {
        self.0.to_bits() == o.0.to_bits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round24_basics() {
        assert_eq!(round24(1.0), 1.0);
        // 1 + 2^-24 is a tie between 1 and 1 + 2^-23: rounds to even (1).
        assert_eq!(round24(1.0 + 2f64.powi(-24)), 1.0);
        assert_eq!(round24(1.0 + 3.0 * 2f64.powi(-24)), 1.0 + 2.0 * 2f64.powi(-23));
        // No exponent limit: 2^200 * (1 + 2^-30) rounds to 2^200, no overflow, no f32 clamp.
        assert_eq!(round24(2f64.powi(200) * (1.0 + 2f64.powi(-30))), 2f64.powi(200));
        // Carry into the exponent.
        assert_eq!(round24(2.0 - 2f64.powi(-30)), 2.0);
    }

    #[test]
    fn pc24_keeps_extended_range() {
        let big = X87::from_f32(3e38);
        let s = big.fadd(big);
        assert!(s.to_f64().is_finite());
        assert_eq!(s.to_f32(), f32::INFINITY);
        let tiny = X87::from_f32(1e-30).fmul(X87::from_f32(1e-20));
        assert!(tiny.to_f64() > 0.0);
    }

    #[test]
    fn nans() {
        let f = f32::from_bits;
        let snan = f(0x7F80_0001);
        let q = X87::from_f32(snan);
        assert_eq!(q.to_f32().to_bits(), 0x7FC0_0001);
        let ind = X87::from_f32(0.0).fdiv(X87::from_f32(0.0));
        assert_eq!(ind.to_f32().to_bits(), 0xFFC0_0000);
        assert_eq!(X87::from_f32(-1.0).fsqrt().to_f32().to_bits(), 0xFFC0_0000);
        assert_eq!(X87::from_f32(-0.0).fsqrt().to_f32().to_bits(), 0x8000_0000);
        let a = X87::from_f32(f(0x7FC0_0005));
        let b = X87::from_f32(f(0xFFC0_0009));
        assert_eq!(a.fadd(b).to_f32().to_bits(), 0xFFC0_0009);
        assert_eq!(b.fadd(a).to_f32().to_bits(), 0xFFC0_0009);
        // equal significands: the positive one
        let (p, n) = (X87::from_f32(f(0x7FC0_0000)), X87::from_f32(f(0xFFC0_0000)));
        assert_eq!(n.fsub(p).to_f32().to_bits(), 0x7FC0_0000);
        assert_eq!(p.fsub(n).to_f32().to_bits(), 0x7FC0_0000);
        // a signalling-NaN memory operand loses to a quiet NaN register
        assert_eq!(p.fdiv_m(f(0x7F80_0001)).to_f32().to_bits(), 0x7FC0_0000);
        assert_eq!(X87::one().fdiv_m(f(0x7F80_0001)).to_f32().to_bits(), 0x7FC0_0001);
    }

    #[test]
    fn ext_encoding() {
        assert_eq!(
            X87::from_f32(1.0).to_ext(),
            [0, 0, 0, 0, 0, 0, 0, 0x80, 0xFF, 0x3F]
        );
        assert_eq!(X87::from_f32(-0.0).to_ext(), [0, 0, 0, 0, 0, 0, 0, 0, 0, 0x80]);
        assert_eq!(
            X87::from_f32(f32::INFINITY).to_ext(),
            [0, 0, 0, 0, 0, 0, 0, 0x80, 0xFF, 0x7F]
        );
        let ind = X87::indefinite().to_ext();
        assert_eq!(ind, [0, 0, 0, 0, 0, 0, 0, 0xC0, 0xFF, 0xFF]);
    }

    #[test]
    fn parity_tests() {
        // `test ah,5; jp` = "ST0 >= operand or unordered".
        assert!(Cmp::Greater.test_parity(5));
        assert!(Cmp::Equal.test_parity(5));
        assert!(!Cmp::Less.test_parity(5));
        assert!(Cmp::Unordered.test_parity(5));
        // `test ah,0x44; jp` = "not equal (ordered) or unordered".
        assert!(!Cmp::Equal.test_parity(0x44));
        assert!(Cmp::Less.test_parity(0x44));
        assert!(Cmp::Unordered.test_parity(0x44));
    }
}
