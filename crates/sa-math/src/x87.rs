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
//! Transcendentals (FPATAN, FSIN, FCOS, FSINCOS, FYL2X, FPTAN) are *not* affected by PC on the real
//! CPU and return a 64-bit significand. Such "wide" values are held exactly as an f64 pair, and
//! arithmetic on them is exact integer arithmetic rounded once (see [`X87`]). The models reproduce the
//! CPU's microcode structure where it was measurable (tables rounded to 67 bits, 67-bit truncated
//! intermediates); measured against the CPU's full 80-bit results (`sa-oracle`, l1-harness.md):
//! FPATAN 99.9995%, FSIN/FCOS ~99.5%, FYL2X ~99.4%, FPTAN ~95.9%, the rest 1 ulp of 64 bits off;
//! every stored f32 matched.

use crate::x87_wide::{self, Big, Dd};

/// The CPU's 80-bit constants (round to nearest), as (significand, biased exponent).
const LG2_EXT: (u64, u16) = (0x9A20_9A84_FBCF_F799, 0x3FFD);
const LN2_EXT: (u64, u16) = (0xB172_17F7_D1CF_79AC, 0x3FFE);
const L2E_EXT: (u64, u16) = (0xB8AA_3B29_5C17_F0BC, 0x3FFF);
const L2T_EXT: (u64, u16) = (0xD49A_784B_CD1B_8AFE, 0x4000);
const PI_EXT: (u64, u16) = (0xC90F_DAA2_2168_C235, 0x4000);
use core::cmp::Ordering;

/// f64 bits of the x87 indefinite QNaN (sign 1, quiet bit only).
const INDEFINITE: u64 = 0xFFF8_0000_0000_0000;
const QUIET: u64 = 1 << 51;
const FRAC: u64 = (1 << 52) - 1;

/// One x87 register value under precision control 24 (see the module docs).
///
/// The value is `.0 + .1` exactly: `.0` is the value rounded to 53 bits and `.1` the (exact) rest,
/// which is non-zero only for the 64-bit results of FYL2X and FPTAN ("wide" values). Arithmetic on a
/// wide value is computed exactly in integers (`x87_wide`), then rounded as the x87 rounds.
#[derive(Clone, Copy, Debug)]
pub struct X87(f64, f64);

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

/// The CPU's pi for argument reduction (FSIN/FCOS/FPTAN): 66 bits, `P = PI66 * 2^-64`.
const PI66: u128 = 0x3_243F_6A88_85A3_08D3;

/// FSIN/FCOS's table: `(sin(c), cos(c))` rounded to nearest at 67 bits for the centres
/// `c = 2^b * (1 + (2j+1)/8)` of the quarters of the binades [0.25, 0.5) and [0.5, 1), as
/// ((significand, exponent), (significand, exponent)). Fitted against the CPU (see [`X87::fcos`]).
const SINCOS_TAB: [((u128, i32), (u128, i32)); 8] = [
    ((0x4_70DF_5931_AE1D_9460, -68), (0x7_AF88_53DD_BBE9_EFD0, -67)),
    ((0x5_646F_27E8_BD65_CBE4, -68), (0x7_882F_D26B_35B0_3D34, -67)),
    ((0x6_529A_FA7D_51B1_2963, -68), (0x7_594F_C1CF_900F_E89E, -67)),
    ((0x7_3A74_B8F5_2947_B682, -68), (0x7_2316_FE33_86A1_0D5A, -67)),
    ((0x4_4434_312D_A70E_DD92, -67), (0x6_C474_1058_A931_88EF, -67)),
    ((0x5_13AC_E073_CE1A_AC13, -67), (0x6_2EC4_1697_7240_1864, -67)),
    ((0x5_CEDD_A037_A95D_F6EE, -67), (0x5_8061_49BD_58F7_D46D, -67)),
    ((0x6_72DA_A6EF_3992_B586, -67), (0x4_BC04_4C99_0839_0C72, -67)),
];

/// `(sin r, cos r)` for an exact `0 < r <= P/4` as the CPU computes them before the final rounding
/// (see [`X87::fcos`]): directly for `r < 0.25`, else from [`SINCOS_TAB`] at the centre `c` of r's
/// quarter-binade, `sin(c + t) = S cos t + C sin t`, `cos(c + t) = C cos t - S sin t`. Evaluated in
/// double-double (~100 bits), returned unrounded.
fn sincos_table(r: Big) -> (Big, Big) {
    let top = r.e + r.top();
    if top < -2 {
        let (s, c) = x87_wide::sincos_dd(r.to_dd());
        return (Big::from_pair(s.h, s.l), Big::from_pair(c.h, c.l));
    }
    // r in [0.25, 1): its quarter j and centre
    let b = top; // -2 or -1
    let j = ((r.m << r.m.leading_zeros()) >> 125) & 3; // the two bits after the leading one
    let centre = Big::c(8 + 2 * j + 1, b - 3);
    let t = Big::add_exact(r, centre.neg());
    let ((sm, se), (cm, ce)) = SINCOS_TAB[(4 * (b + 2) + j as i32) as usize];
    let (st, ct) = x87_wide::sincos_dd(t.to_dd());
    let (s, c) = (Big::c(sm, se).to_dd(), Big::c(cm, ce).to_dd());
    let sv = s.mul(ct).add(c.mul(st));
    let cv = c.mul(ct).sub(s.mul(st));
    (Big::from_pair(sv.h, sv.l), Big::from_pair(cv.h, cv.l))
}

/// pi/2, pi/4 and 3pi/4 as FPATAN returns them for infinite operands (measured; round to nearest).
const PIO2_EXT: (u64, u16) = (0xC90F_DAA2_2168_C235, 0x3FFF);
const PIO4_EXT: (u64, u16) = (0xC90F_DAA2_2168_C235, 0x3FFE);
const PI3O4_EXT: (u64, u16) = (0x96CB_E3F9_990E_91A8, 0x4000);

/// FPATAN's pi: rounded to nearest at 67 bits, as (significand, exponent).
const PI67: (u128, i32) = (0x6_487E_D511_0B46_11A6, -65);

/// FPATAN's table `atan(k/32)` for k = 2..=32, rounded to nearest at 67 bits (significand, exponent).
/// Fitted against the CPU (see [`X87::fpatan`]); the values are the correctly rounded constants.
const ATAN_TAB: [(u128, i32); 31] = [
    (0x7_FD56_EDCB_3F7A_71B6, -71),
    (0x5_FB86_0980_BC43_A305, -70),
    (0x7_F56E_A6AB_0BDB_7196, -70),
    (0x4_F5BB_BA31_989B_161A, -69),
    (0x5_EE5E_D2F3_96C0_89A4, -69),
    (0x6_E435_D4A4_9828_8118, -69),
    (0x7_D6DD_7E4B_2037_58AB, -69),
    (0x4_62FD_68C2_FC5E_0986, -68),
    (0x4_D89D_CDC1_FAF2_F34E, -68),
    (0x5_4C2B_6654_7352_76D5, -68),
    (0x5_BD86_5079_37BC_239C, -68),
    (0x6_2C93_4E52_86C9_5B6D, -68),
    (0x6_993B_B0F3_08FF_2DB2, -68),
    (0x7_036D_3253_B27B_E33E, -68),
    (0x7_6B19_C158_6ED3_DA2B, -68),
    (0x7_D037_42D5_0505_F2E3, -68),
    (0x4_195F_A536_CC33_F152, -67),
    (0x4_4957_66FE_F4AA_3DA8, -67),
    (0x4_7802_EAF7_BFAC_FCDB, -67),
    (0x4_A563_964C_238C_37B1, -67),
    (0x4_D17C_0733_8DEE_D102, -67),
    (0x4_FC4F_EE27_A5BD_0F68, -67),
    (0x5_25E3_E8C9_A7B8_4921, -67),
    (0x5_4E3D_5EE2_4187_AE45, -67),
    (0x5_7562_61C5_A6C6_0401, -67),
    (0x5_9B59_8E48_F821_B48B, -67),
    (0x5_C029_F15E_118C_F39E, -67),
    (0x5_E3DA_EF57_4C57_9407, -67),
    (0x6_0674_2DC5_6293_3204, -67),
    (0x6_27FD_7FD5_FC7D_EAA4, -67),
    (0x6_487E_D511_0B46_11A6, -67),
];

/// `atan(t)` for an exact `|t| <= 0.05`: `t` plus the double-double correction (exact up to a sticky
/// part far below the 67th bit), or `t` itself for `|t| < 2^-40`; truncated to 67 bits when `trunc`.
fn atan_67(t: Big, trunc: bool) -> Big {
    if t.m == 0 {
        return t;
    }
    // below 2^-40 the CPU returns t itself (measured: the -t^3/3 correction decides ties of the
    // 64-bit rounding for |t| >= 2^-40 and never below)
    let a = if t.e + t.top() < -40 {
        t
    } else {
        let c = x87_wide::atan_minus_t_dd(t.to_dd());
        Big::add_exact(t, Big::from_pair(c.h, c.l))
    };
    if trunc { a.truncate(67) } else { a }
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
    /// A narrow value (at most 53 significant bits).
    const fn n(v: f64) -> X87 {
        X87(v, 0.0)
    }

    /// True for a value with more than 53 significant bits (see the type docs).
    pub fn is_wide(self) -> bool {
        self.1 != 0.0
    }

    /// The exact value of a finite register value.
    fn big(self) -> Big {
        Big::from_pair(self.0, self.1)
    }

    /// A register value from an exact `Big` with at most 64 significant bits (non-zero).
    fn from_big_exact(b: Big) -> X87 {
        let (neg, e, m) = b.norm64();
        // hi = m rounded to 53 bits (nearest-even), lo = the exact rest (at most 11 bits)
        let hi = Big {
            neg,
            e,
            m: m as u128,
            sticky: false,
        }
        .round(53, None);
        let hv = hi.to_f64();
        let rest = (m as i128) - ((hi.m as i128) << (hi.e - e) as u32);
        let lv = libm::scalbn(rest as f64, e);
        // a value that fits 53 bits is narrow: lo is +0.0 (never -0.0)
        X87(hv, if neg && rest != 0 { -lv } else { lv })
    }

    /// A finite value rounded to nearest-even with a 64-bit significand (a transcendental result).
    fn from_big64(b: Big) -> X87 {
        if b.m == 0 {
            return X87::n(if b.neg { -0.0 } else { 0.0 });
        }
        X87::from_big_exact(b.round(64, None))
    }

    /// `fld tbyte` of a finite or special 80-bit value (unnormals and pseudo-denormals are not
    /// modelled). Values outside the f64 exponent range are not modelled either.
    pub fn from_ext(b: [u8; 10]) -> X87 {
        let m = u64::from_le_bytes(b[..8].try_into().unwrap());
        let se = u16::from_le_bytes([b[8], b[9]]);
        let neg = se >> 15 != 0;
        let e = i32::from(se & 0x7FFF);
        let sign = (neg as u64) << 63;
        if e == 0x7FFF {
            // infinity, or a NaN with the top 52 fraction bits (the f64 layout keeps those)
            let frac = (m << 1) >> 12;
            return X87::n(f64::from_bits(sign | (0x7FF << 52) | frac));
        }
        if m == 0 {
            return X87::n(f64::from_bits(sign));
        }
        X87::from_big_exact(Big {
            neg,
            e: e.max(1) - 16383 - 63,
            m: m as u128,
            sticky: false,
        })
    }

    /// `fld dword`: exact for every f32; a signalling NaN is quieted (payload kept).
    pub fn from_f32(v: f32) -> X87 {
        let b = v.to_bits();
        if v.is_nan() {
            let sign = ((b >> 31) as u64) << 63;
            let frac = ((b & 0x7F_FFFF) as u64) << 29;
            return X87::n(f64::from_bits(sign | (0x7FF << 52) | frac | QUIET));
        }
        X87::n(f64::from(v))
    }

    /// A constant loaded from the exe's `.rdata` (always finite).
    pub fn constant(bits: u32) -> X87 {
        X87::from_f32(f32::from_bits(bits))
    }

    /// `fld1`
    pub fn one() -> X87 {
        X87::n(1.0)
    }

    /// `fldz`
    pub fn zero() -> X87 {
        X87::n(0.0)
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
        if self.is_wide() {
            // one rounding of the exact value (f32 subnormals: grid 2^-149; 2^128 becomes inf)
            return self.big().round(24, Some(-149)).to_f64() as f32;
        }
        self.0 as f32
    }

    /// The register value as f64 (exact for narrow values, rounded to 53 bits for wide ones; NaN
    /// payload in the f64 layout).
    pub fn to_f64(self) -> f64 {
        self.0
    }

    /// The 80-bit extended encoding of this value (what `fstp tbyte` writes).
    pub fn to_ext(self) -> [u8; 10] {
        if self.is_wide() {
            let (neg, e, m) = self.big().norm64();
            let mut out = [0u8; 10];
            out[..8].copy_from_slice(&m.to_le_bytes());
            let se = ((neg as u16) << 15) | (e + 63 + 16383) as u16;
            out[8..].copy_from_slice(&se.to_le_bytes());
            return out;
        }
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
        X87::n(f64::from_bits(self.0.to_bits() | QUIET))
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
        X87::n(f64::from_bits(INDEFINITE))
    }

    fn arith(self, op: Op, src: Src) -> X87 {
        if let Some(n) = self.nan_of(src) {
            return n;
        }
        let bx = match src {
            Src::Reg(x) => x,
            Src::Mem(m) => X87::from_f32(m),
        };
        if (self.is_wide() || bx.is_wide())
            && let Some(v) = self.arith_wide(op, bx)
        {
            return v;
        }
        let b = bx.0;
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
        X87::n(round24_exact(r, e))
    }

    /// `arith` with a wide operand: exact, then rounded to 24 bits. `None` for the zero and infinity
    /// operand cases, whose results do not depend on the low part (the narrow path handles them).
    fn arith_wide(self, op: Op, b: X87) -> Option<X87> {
        let (x, y) = match op {
            Op::SubR | Op::DivR => (b, self),
            _ => (self, b),
        };
        let special = |v: X87| v.0 == 0.0 || v.0.is_infinite();
        let r = match op {
            Op::Add | Op::Sub | Op::SubR => {
                let y = if matches!(op, Op::Add) { y } else { y.fchs() };
                if x.0.is_infinite() || y.0.is_infinite() {
                    return None;
                }
                if y.0 == 0.0 {
                    x.big()
                } else if x.0 == 0.0 {
                    y.big()
                } else {
                    Big::add(x.big(), y.big())
                }
            }
            Op::Mul => {
                if special(x) || special(y) {
                    return None;
                }
                Big::mul(x.big(), y.big())
            }
            Op::Div | Op::DivR => {
                if special(x) || special(y) {
                    return None;
                }
                Big::div(x.big(), y.big())
            }
        };
        Some(X87::n(r.round(24, None).to_f64()))
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
        if self.is_wide() {
            return X87::n(Big::sqrt(self.big()).round(24, None).to_f64());
        }
        let s = libm::sqrt(self.0);
        let e = if s.is_finite() {
            libm::fma(-s, s, self.0)
        } else {
            0.0
        };
        X87::n(round24_exact(s, e))
    }

    /// FCHS (also flips the sign of a NaN).
    pub fn fchs(self) -> X87 {
        X87(
            f64::from_bits(self.0.to_bits() ^ (1 << 63)),
            if self.1 != 0.0 { -self.1 } else { 0.0 },
        )
    }

    /// FPATAN with ST1 = `y` (self) and ST0 = `x`: atan2(y, x), *not* rounded to 24 bits (PC does not
    /// apply): a 64-bit result.
    ///
    /// Model of the CPU's microcode (fitted with `sa-oracle`'s native backend on an Intel i7-14700F,
    /// see `docs/re/l1-harness.md`; all steps are exact integer arithmetic except `atan(t) - t`,
    /// evaluated in double-double to ~100 bits):
    /// - `q = min(|y|, |x|) / max(|y|, |x|)` (exact), `k = round(32 q)`;
    /// - `k < 2`: `t = q` truncated to 67 bits, `a = atan(t)`;
    /// - `k >= 2`: `t = (q - k/32) / (1 + q k/32)` truncated to 67 bits,
    ///   `a = A[k] + trunc67(atan(t))` with the table `A[k] = atan(k/32)` rounded to nearest at 67 bits;
    /// - `|y| <= |x|`, `x > 0`: the result is `a` rounded to nearest at 64 bits; otherwise `a` is
    ///   truncated to 67 bits and the result is `P/2 - a` (`|y| > |x|`, `x > 0`), `P/2 + a`
    ///   (`|y| > |x|`, `x < 0`) or `P - a` (`|y| <= |x|`, `x < 0`) rounded to nearest at 64 bits, `P` =
    ///   pi rounded to nearest at 67 bits; the sign is y's.
    ///
    /// Special operands as the SDM table: NaNs as the arithmetic ops, `y = ±0` → ±0 (`x > 0` or +0) or
    /// ±pi (`x < 0` or -0), `x = ±0` → ±pi/2, `y = ±inf` → ±pi/2 (finite x), ±pi/4 (`x = +inf`), ±3pi/4
    /// (`x = -inf`), finite y with `x = +inf` → ±0, `x = -inf` → ±pi.
    pub fn fpatan(self, x: X87) -> X87 {
        if let Some(n) = self.nan_of(Src::Reg(x)) {
            return n;
        }
        let (yv, xv) = (self.0, x.0);
        let xneg = xv.is_sign_negative();
        let sgn = |v: X87| if yv.is_sign_negative() { v.fchs() } else { v };
        let zero = X87::n(0.0);
        if yv == 0.0 {
            return sgn(if xneg { X87::pi() } else { zero });
        }
        if yv.is_infinite() {
            return sgn(if !xv.is_infinite() {
                X87::ext_const(PIO2_EXT)
            } else if xneg {
                X87::ext_const(PI3O4_EXT)
            } else {
                X87::ext_const(PIO4_EXT)
            });
        }
        if xv.is_infinite() {
            return sgn(if xneg { X87::pi() } else { zero });
        }
        if xv == 0.0 {
            return sgn(X87::ext_const(PIO2_EXT));
        }
        let (ay, ax) = (self.fabs().big(), x.fabs().big());
        let swap = Big::cmp_abs(ay, ax) == Ordering::Greater;
        let (n, d) = if swap { (ax, ay) } else { (ay, ax) };
        let q = Big::div_floor(n, d, 72);
        // k = round(32 q), q <= 1
        let sh = -(q.e + 5);
        let k = if sh <= 0 {
            32
        } else if sh > 127 {
            0
        } else {
            (((q.m >> (sh - 1) as u32) + 1) >> 1) as u32
        };
        let a = if k < 2 {
            atan_67(q.truncate(67), false)
        } else {
            // t = (q - c) / (1 + q c) = (32 n - k d) / (32 d + k n), c = k/32 (exact sums)
            let n32 = Big { e: n.e + 5, ..n };
            let d32 = Big { e: d.e + 5, ..d };
            let kb = Big::c(u128::from(k), 0);
            let num = Big::add_exact(n32, Big::mul(d, kb).neg());
            let den = Big::add_exact(d32, Big::mul(n, kb));
            let at = if num.m == 0 {
                Big::zero(false)
            } else {
                let t = Big::div_floor(num, den, 72).truncate(67);
                atan_67(t, true)
            };
            let (m, e) = ATAN_TAB[k as usize - 2];
            Big::add_exact(Big::c(m, e), at)
        };
        let r = if !swap && !xneg {
            a
        } else {
            let a = a.truncate(67);
            let (pm, pe) = PI67;
            let base = Big::c(pm, if swap { pe - 1 } else { pe });
            Big::add_exact(base, if swap && xneg { a } else { a.neg() })
        };
        sgn(X87::from_big64(r))
    }

    /// A register value from an f64 that the x87 holds exactly (e.g. `fld qword` of an integral double).
    pub fn from_f64_exact(v: f64) -> X87 {
        X87::n(v)
    }

    /// `fild dword`: exact (an i32 always fits the 64-bit significand, and an f64).
    pub fn from_i32(v: i32) -> X87 {
        X87::n(f64::from(v))
    }

    /// `fadd st, st` (doubling: exact unless it overflows the modelled range).
    pub fn fdouble(self) -> X87 {
        self.fadd(self)
    }

    fn ext_const((m, e): (u64, u16)) -> X87 {
        let mut b = [0u8; 10];
        b[..8].copy_from_slice(&m.to_le_bytes());
        b[8..].copy_from_slice(&e.to_le_bytes());
        X87::from_ext(b)
    }
    /// `fldlg2` (log10 2, 64 bits, round to nearest).
    pub fn lg2() -> X87 {
        X87::ext_const(LG2_EXT)
    }
    /// `fldln2` (ln 2).
    pub fn ln2() -> X87 {
        X87::ext_const(LN2_EXT)
    }
    /// `fldl2e` (log2 e).
    pub fn l2e() -> X87 {
        X87::ext_const(L2E_EXT)
    }
    /// `fldl2t` (log2 10).
    pub fn l2t() -> X87 {
        X87::ext_const(L2T_EXT)
    }
    /// `fldpi`.
    pub fn pi() -> X87 {
        X87::ext_const(PI_EXT)
    }

    /// `fabs` (clears the sign, NaNs included).
    pub fn fabs(self) -> X87 {
        if self.0.is_sign_negative() {
            self.fchs()
        } else {
            self
        }
    }

    /// FYL2X with ST1 = `y` (self) and ST0 = `x`: `y * log2(x)`, not affected by PC (64-bit result).
    ///
    /// Model of the CPU (measured on an Intel i7-14700F by `sa-oracle`, see `docs/re/l1-harness.md`):
    /// `log2(x)` is evaluated to ~104 bits, **truncated (toward zero) to 67 significant bits**, then
    /// multiplied exactly by `y` and rounded to nearest with a 64-bit significand. It matches the CPU's
    /// full 80-bit result in about 99.3% of cases; the rest differ by 1 ulp of the 64-bit significand
    /// (see the ledger). Special operands follow the SDM table: NaN rules as the arithmetic ops, `x < 0`
    /// (incl. -inf) → indefinite, `x = ±0` → ∓inf·sign(y) (`y = 0`: indefinite), `x = +inf` → ±inf by
    /// the sign of y (`y = 0`: indefinite), `x = 1` → ±0 by the sign of y (`y = ±inf`: indefinite).
    pub fn fyl2x(self, x: X87) -> X87 {
        let y = self;
        if let Some(n) = y.nan_of(Src::Reg(x)) {
            return n;
        }
        let (xv, yv) = (x.0, y.0);
        let yneg = yv.is_sign_negative();
        let inf = |neg: bool| X87::n(if neg { f64::NEG_INFINITY } else { f64::INFINITY });
        let zero = |neg: bool| X87::n(if neg { -0.0 } else { 0.0 });
        if xv < 0.0 {
            return X87::indefinite();
        }
        if xv == 0.0 {
            return if yv == 0.0 { X87::indefinite() } else { inf(!yneg) };
        }
        if xv.is_infinite() {
            return if yv == 0.0 { X87::indefinite() } else { inf(yneg) };
        }
        if xv == 1.0 && x.1 == 0.0 {
            return if yv.is_infinite() {
                X87::indefinite()
            } else {
                zero(yneg)
            };
        }
        let l = x87_wide::log2_dd(x.0, x.1);
        let lneg = l.h < 0.0;
        if yv.is_infinite() {
            return inf(yneg != lneg);
        }
        if yv == 0.0 {
            return zero(yneg != lneg);
        }
        let l67 = Big::from_pair(l.h, l.l).truncate(67);
        X87::from_big64(Big::mul_wide(y.big(), l67))
    }

    /// FPTAN: returns `(ST1, ST0)` after the instruction: `tan(x)` (not affected by PC, 64-bit result)
    /// and the pushed 1.0; for a NaN or infinite operand both are the NaN result (measured: the CPU
    /// pushes the NaN, not 1.0). `None` when `|x| >= 2^63`: the CPU sets C2 and leaves the operand
    /// unchanged (nothing pushed).
    ///
    /// Model: the argument is reduced against the CPU's 66-bit pi exactly as for [`X87::fcos`]
    /// (`k = round(x / (P/2))`, `r = x - k*P/2`), `tan(r)` (k even) or `-1/tan(r)` (k odd) is evaluated
    /// to ~104 bits and rounded to nearest with a 64-bit significand. It matches the CPU's full 80-bit
    /// result in about 95% of cases; the rest differ by 1 ulp of the 64-bit significand, always
    /// where the exact value lies within ~0.1 ulp of a rounding boundary (see the ledger). NaN → the
    /// quieted NaN, ±inf → the indefinite, ±0 → ±0.
    pub fn fptan(self) -> Option<(X87, X87)> {
        let x = self.0;
        if x.is_nan() {
            return Some((self.quiet(), self.quiet()));
        }
        if x.is_infinite() {
            return Some((X87::indefinite(), X87::indefinite()));
        }
        Some((self.ftan()?, X87::one()))
    }

    /// The FPTAN result for a finite operand (`None` when `|x| >= 2^63`).
    fn ftan(self) -> Option<X87> {
        let x = self.0;
        if x == 0.0 {
            return Some(self);
        }
        let neg = x < 0.0;
        let a = self.fabs();
        let (_, e, m) = a.big().norm64();
        if e >= 0 {
            return None; // |x| >= 2^63
        }
        let (odd, r) = if a.0 < 0.78 {
            (false, Dd { h: a.0, l: a.1 })
        } else {
            // x / (P/2) = m * 2^(e+65) / PI66, 1 <= e + 65 <= 64
            let n = u128::from(m) << (e + 65) as u32;
            let mut k = n / PI66;
            let rem = n % PI66;
            let (neg_r, rem) = if 2 * rem > PI66 {
                k += 1;
                (true, PI66 - rem)
            } else {
                (false, rem)
            };
            let h = rem as f64;
            let l = (rem as i128 - h as i128) as f64;
            let r = Dd { h, l }.scale(-65);
            (k & 1 == 1, if neg_r { r.neg() } else { r })
        };
        let t = x87_wide::tan_dd(r);
        let t = if odd { Dd::new(-1.0).div(t) } else { t };
        let t = if neg { t.neg() } else { t };
        if t.h == 0.0 {
            return Some(X87::n(t.h));
        }
        Some(X87::from_big64(Big::from_pair(t.h, t.l)))
    }

    /// FSINCOS: `(sin, cos)` = (ST1, ST0) after the instruction, each as [`X87::fsin`] /
    /// [`X87::fcos`] (measured: the same values); a NaN or infinite operand gives the NaN result in
    /// both. `None` when `|x| >= 2^63`: the CPU sets C2 and leaves the operand (nothing pushed).
    pub fn fsincos(self) -> Option<(X87, X87)> {
        let x = self.0;
        if x.is_finite() && x.abs() >= 9_223_372_036_854_775_808.0 {
            return None;
        }
        Some((self.fsin(), self.fcos()))
    }

    /// FSIN (not affected by PC). See [`X87::fcos`] for the model.
    pub fn fsin(self) -> X87 {
        self.trig(false)
    }

    /// FCOS (not affected by PC): a 64-bit result. Model of the CPU (fitted with `sa-oracle`'s native
    /// backend, see `docs/re/l1-harness.md`):
    /// - NaN → the quieted NaN; ±inf → the indefinite; ±0 → 1 (FSIN: ±0);
    /// - |x| >= 2^63 → the operand unchanged (the CPU sets C2 and does nothing);
    /// - otherwise the argument is reduced against the CPU's 66-bit approximation of pi,
    ///   `P = 0x3.243F6A8885A308D3`, by quadrants: `k = round(x / (P/2))`, `r = x - k*P/2` computed
    ///   exactly (so for large arguments the result follows the CPU's pi, not the true one);
    /// - `|r| < 0.25`: `sin r` / `cos r`; else from a table at the centre `c` of r's quarter-binade
    ///   (`c = 2^b (1 + (2j+1)/8)`) holding `sin c` and `cos c` rounded to nearest at 67 bits:
    ///   `S cos t + C sin t` / `C cos t - S sin t`, `t = |r| - c`; evaluated to ~100 bits, rotated by
    ///   the quadrant and rounded to nearest at 64 bits.
    ///
    /// The table values are exact (their rounding errors are visible in the CPU's results); the CPU's
    /// internal rounding of the polynomial terms is not modelled: about 0.5% of results differ from
    /// the CPU by 1 ulp of the 64-bit significand (every stored f32 matched; ledger in l1-harness.md).
    pub fn fcos(self) -> X87 {
        self.trig(true)
    }

    fn trig(self, cos: bool) -> X87 {
        let x = self.0;
        if x.is_nan() {
            return self.quiet();
        }
        if x.is_infinite() {
            return X87::indefinite();
        }
        if x == 0.0 {
            return if cos { X87::one() } else { self };
        }
        let a = self.fabs();
        let (_, e, m) = a.big().norm64();
        if e >= 0 {
            return self; // |x| >= 2^63
        }
        // r = |x| - k*P/2 (exact), q = k mod 4
        let (q, r) = if a.0 < 0.78 {
            (0, a.big())
        } else {
            // |x| / (P/2) = m * 2^(e+65) / PI66, 1 <= e + 65 <= 64
            let n = u128::from(m) << (e + 65) as u32;
            let mut k = n / PI66;
            let rem = n % PI66;
            // PI66 is odd: no tie
            let r = if 2 * rem > PI66 {
                k += 1;
                Big::c(PI66 - rem, -65).neg()
            } else {
                Big::c(rem, -65)
            };
            ((k & 3) as u32, r)
        };
        let ar = Big { neg: false, ..r };
        let (sr, cr) = sincos_table(ar);
        let sr = if r.neg { sr.neg() } else { sr };
        // sin(r + q*pi/2) and cos(r + q*pi/2)
        let v = match (cos, q) {
            (false, 0) | (true, 3) => sr,
            (false, 1) | (true, 0) => cr,
            (false, 2) | (true, 1) => sr.neg(),
            _ => cr.neg(),
        };
        // sin is odd, cos is even
        let v = if !cos && x.is_sign_negative() { v.neg() } else { v };
        X87::from_big64(v)
    }

    /// MSVC `_ftol2` (0x821B40), which the game calls for every float-to-int conversion: truncation of
    /// ST0 (the operand is consumed) to a 64-bit integer, returned in EDX:EAX. It rounds with FISTP
    /// (round to nearest) and corrects by the sign of the f32-stored difference. NaN, ±inf and values out
    /// of the i64 range give the "integer indefinite" `0x8000_0000_0000_0000`.
    pub fn ftol(self) -> u64 {
        let v = self.0;
        // fistp qword (RC = nearest)
        let r: u64 = if self.is_wide() {
            // the exact value rounded to an integer (nearest-even), then the fistp range check
            let b = self.big().round(128, Some(0));
            let mag = if b.e >= 64 { u128::MAX } else { b.m << b.e as u32 };
            if (!b.neg && mag >= 1 << 63) || (b.neg && mag > 1 << 63) {
                0x8000_0000_0000_0000
            } else if b.neg {
                (mag as u64).wrapping_neg()
            } else {
                mag as u64
            }
        } else if v.is_nan() || v.abs() >= 9_223_372_036_854_775_808.0 {
            0x8000_0000_0000_0000 // also the exact encoding of -2^63
        } else {
            libm::rint(v) as i64 as u64
        };
        let (lo, hi) = (r as u32, (r >> 32) as u32);
        let sign_bits = self.to_f32().to_bits(); // fst dword [esp+18h]
        if lo == 0 && hi & 0x7FFF_FFFF == 0 {
            return r;
        }
        // fsubp: v - r (PC=24), stored as f32
        let ri = r as i64;
        let rx = if ri == 0 {
            X87::zero()
        } else {
            X87::from_big_exact(Big {
                neg: ri < 0,
                e: 0,
                m: u128::from(ri.unsigned_abs()),
                sticky: false,
            })
        };
        let d = self.fsub(rx).to_f32().to_bits();
        let (mut lo, mut hi) = (lo, hi);
        if (sign_bits as i32) < 0 {
            // xor ecx, 80000000h; add ecx, 7FFFFFFFh; adc eax, 0; adc edx, 0
            let (_, carry) = (d ^ 0x8000_0000).overflowing_add(0x7FFF_FFFF);
            let (l, c2) = lo.overflowing_add(carry as u32);
            lo = l;
            hi = hi.wrapping_add(c2 as u32);
        } else {
            // add ecx, 7FFFFFFFh; sbb eax, 0; sbb edx, 0
            let (_, carry) = d.overflowing_add(0x7FFF_FFFF);
            let (l, b2) = lo.overflowing_sub(carry as u32);
            lo = l;
            hi = hi.wrapping_sub(b2 as u32);
        }
        (u64::from(hi) << 32) | u64::from(lo)
    }

    /// FCOM/FCOMP: compare `self` (ST0) with `o`.
    pub fn fcom(self, o: X87) -> Cmp {
        let ord = match self.0.partial_cmp(&o.0) {
            Some(Ordering::Equal) => self.1.partial_cmp(&o.1),
            r => r,
        };
        match ord {
            None => Cmp::Unordered,
            Some(Ordering::Less) => Cmp::Less,
            Some(Ordering::Equal) => Cmp::Equal,
            Some(Ordering::Greater) => Cmp::Greater,
        }
    }

    /// Bit-exact identity (NaN payloads included).
    pub fn same(self, o: X87) -> bool {
        self.0.to_bits() == o.0.to_bits() && self.1.to_bits() == o.1.to_bits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 80-bit results of FPATAN measured on the CPU (sa-oracle `x87_trig`).
    #[test]
    fn fpatan_cpu_values() {
        let ext = |h: &str| {
            let se = u16::from_str_radix(&h[..4], 16).unwrap();
            let m = u64::from_str_radix(&h[4..], 16).unwrap();
            let mut b = [0u8; 10];
            b[..8].copy_from_slice(&m.to_le_bytes());
            b[8..].copy_from_slice(&se.to_le_bytes());
            b
        };
        let at = |y: u64, x: u64| {
            X87::from_f64_exact(f64::from_bits(y)).fpatan(X87::from_f64_exact(f64::from_bits(x))).to_ext()
        };
        // |y| == |x|, table k = 32; quadrants
        assert_eq!(at(0x3FF0_0000_0000_0000, 0x3FF0_0000_0000_0000), ext("3ffec90fdaa22168c235"));
        assert_eq!(at(0x3FF0_0000_0000_0000, 0xBFF0_0000_0000_0000), ext("400096cbe3f9990e91a8"));
        // |y| > |x|, x < 0: P/2 + trunc67(a)
        assert_eq!(at(0x4004_0000_0000_0000, 0xBFF0_0000_0000_0000), ext("3ffff9c44978f06441a5"));
        assert_eq!(at(0x3FF0_0000_0000_0000, 0x4004_0000_0000_0000), ext("3ffdc2d1bb5b3bedfdc0"));
        // t < 2^-40: atan(t) = t, a 64-bit tie goes to even
        assert_eq!(at(0x3AE8_DECB_8000_0000, 0x4338_44D4_C000_0000), ext("3f7a832c0b177b2599d2"));
        // specials
        assert_eq!(at(0x8000_0000_0000_0000, 0x8000_0000_0000_0000), ext("c000c90fdaa22168c235"));
        assert_eq!(at(0x7FF0_0000_0000_0000, 0xFFF0_0000_0000_0000), ext("400096cbe3f9990e91a8"));
        assert_eq!(at(0x3FF0_0000_0000_0000, 0x0000_0000_0000_0000), ext("3fffc90fdaa22168c235"));
    }

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

    #[test]
    fn ftol_truncates() {
        let t = |v: f32| X87::from_f32(v).ftol() as i64;
        assert_eq!(t(2.5), 2);
        assert_eq!(t(-2.5), -2);
        assert_eq!(t(0.49), 0);
        assert_eq!(t(-0.6), 0);
        assert_eq!(t(3.0), 3);
        assert_eq!(t(-1e10), -10_000_000_000);
        assert_eq!(X87::from_f32(f32::NAN).ftol(), 0x8000_0000_0000_0000);
        assert_eq!(X87::from_f32(1e30).ftol(), 0x8000_0000_0000_0000);
    }

    #[test]
    fn wide_values() {
        let lg2 = X87::lg2();
        assert!(lg2.is_wide());
        assert_eq!(
            lg2.to_ext(),
            [0x99, 0xF7, 0xCF, 0xFB, 0x84, 0x9A, 0x20, 0x9A, 0xFD, 0x3F]
        );
        assert_eq!(X87::from_ext(lg2.to_ext()).to_ext(), lg2.to_ext());
        assert_eq!(lg2.to_f32().to_bits(), 0x3E9A_209B);
        // log10(10) = lg2 * log2(10): 1.0 rounded to f32, and within an ulp of 1 at 64 bits
        let l = lg2.fyl2x(X87::from_f32(10.0));
        assert_eq!(l.to_f32(), 1.0);
        assert_eq!(l.fmul_m(20.0).to_f32(), 20.0);
        // log2 of powers of two is exact
        assert_eq!(X87::one().fyl2x(X87::from_f32(8.0)).to_f32(), 3.0);
        assert!(!X87::one().fyl2x(X87::from_f32(8.0)).is_wide());
        // special operands
        let bits = |v: X87| v.to_f32().to_bits();
        assert_eq!(bits(X87::one().fyl2x(X87::from_f32(-1.0))), 0xFFC0_0000);
        assert_eq!(bits(X87::one().fyl2x(X87::zero())), 0xFF80_0000);
        assert_eq!(bits(X87::one().fchs().fyl2x(X87::zero())), 0x7F80_0000);
        assert_eq!(bits(X87::one().fyl2x(X87::one())), 0);
        assert_eq!(bits(X87::one().fchs().fyl2x(X87::one())), 0x8000_0000);
        // fptan
        let (t, one) = X87::from_f32(0.5).fptan().unwrap();
        assert!(one.same(X87::one()));
        assert_eq!(t.to_f32(), libm::tanf(0.5));
        assert!(X87::from_f32(1e19).fptan().is_none());
        assert_eq!(bits(X87::from_f32(-0.0).fptan().unwrap().0), 0x8000_0000);
        let (a, b) = X87::from_f32(f32::INFINITY).fptan().unwrap();
        assert_eq!((bits(a), bits(b)), (0xFFC0_0000, 0xFFC0_0000));
        // wide arithmetic: compare, negate, sign of exact cancellation
        assert_eq!(
            t.fcom(X87::from_f64_exact(t.to_f64())),
            if t.1 > 0.0 { Cmp::Greater } else { Cmp::Less }
        );
        assert_eq!(bits(t.fsub(t)), 0);
        assert_eq!(t.fchs().to_f32(), -t.to_f32());
        assert_eq!(t.fabs().fchs().fabs().to_ext(), t.to_ext());
    }

    #[test]
    fn trig_model() {
        let s = |v: f32| X87::from_f32(v).fsin().to_f32();
        assert_eq!(s(0.0), 0.0);
        assert_eq!(s(-0.0).to_bits(), 0x8000_0000);
        assert!((s(1.0) - 0.841_470_96).abs() < 1e-7);
        assert!((X87::from_f32(3.0).fcos().to_f32() + 0.989_992_5).abs() < 1e-6);
        // |x| >= 2^63: unchanged
        assert_eq!(s(1e30), 1e30);
        assert_eq!(
            X87::from_f32(f32::INFINITY).fsin().to_f32().to_bits(),
            0xFFC0_0000
        );
        // reduction against the 66-bit pi: sin(k * P66) for a large multiple differs from libm's sin
        let big = 1.0e17f32;
        let m = X87::from_f32(big).fsin().to_f32();
        assert!(m.abs() <= 1.0);
    }
}
