//! Exact integer arithmetic behind [`crate::x87::X87`] values with more than 53 significant bits (the
//! 64-bit results of FYL2X and FPTAN), and the double-double evaluation of those transcendentals.
//!
//! Everything here is integer or IEEE f64 arithmetic with a single defined result (no host float
//! behaviour that differs between targets): `Big` values are exact, double-double steps use only
//! `+ - * /` and `fma`.

/// A finite value `±(m + ε) * 2^e`, where `ε` is 0 when `sticky` is false and strictly between 0 and
/// 1 otherwise (discarded non-zero low bits).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Big {
    pub neg: bool,
    pub e: i32,
    pub m: u128,
    pub sticky: bool,
}

impl Big {
    pub fn zero(neg: bool) -> Big {
        Big {
            neg,
            e: 0,
            m: 0,
            sticky: false,
        }
    }

    /// The exact value of a finite f64.
    pub fn from_f64(v: f64) -> Big {
        let b = v.to_bits();
        let ef = ((b >> 52) & 0x7FF) as i32;
        let frac = b & ((1 << 52) - 1);
        let (m, e) = if ef == 0 {
            (frac, -1074)
        } else {
            (frac | (1 << 52), ef - 1075)
        };
        Big {
            neg: b >> 63 != 0,
            e,
            m: m as u128,
            sticky: false,
        }
    }

    /// The exact value `hi + lo` of two finite f64s with `|lo| <= ulp(hi)` (an X87 or a double-double
    /// pair). The low part keeps its exact bits down to 2^-70 relative to `hi`; anything below that
    /// becomes the sticky bit.
    pub fn from_pair(hi: f64, lo: f64) -> Big {
        let h = Big::from_f64(hi);
        if lo == 0.0 || h.m == 0 {
            return h;
        }
        let l = Big::from_f64(lo);
        // common exponent 70 bits below hi's top bit: hi's 53 bits fit with room to spare
        let top = h.e + 52;
        let e = top - 70;
        let hm = h.m << (h.e - e) as u32;
        let (lm, rem) = if l.e >= e {
            (l.m << (l.e - e) as u32, false)
        } else {
            let s = (e - l.e) as u32;
            if s >= 128 {
                (0, true)
            } else {
                (l.m >> s, l.m & ((1u128 << s) - 1) != 0)
            }
        };
        let m = if l.neg == h.neg {
            hm + lm
        } else {
            // hi - (lm + δ) = (hi - lm - 1) + (1 - δ)
            hm - lm - rem as u128
        };
        Big {
            neg: h.neg,
            e,
            m,
            sticky: rem,
        }
    }

    /// Index of the top set bit of `m` (m != 0).
    fn top(&self) -> i32 {
        127 - self.m.leading_zeros() as i32
    }

    /// Truncated toward zero to `p` significant bits (the sticky part is dropped with them).
    pub fn truncate(self, p: u32) -> Big {
        if self.m == 0 {
            return self;
        }
        let d = self.top() + 1 - p as i32;
        if d <= 0 {
            return Big {
                sticky: false,
                ..self
            };
        }
        Big {
            neg: self.neg,
            e: self.e + d,
            m: self.m >> d as u32,
            sticky: false,
        }
    }

    /// `a * b` for `a` with at most 64 significant bits and `b` with at most 67 (exact up to a sticky
    /// part below 2^-60 relative, enough to round to 64 bits).
    pub fn mul_wide(a: Big, b: Big) -> Big {
        let (an, ae, am) = a.norm64();
        debug_assert!(b.m >> 67 == 0 && !b.sticky);
        let lo = am as u128 * (b.m & u128::from(u64::MAX));
        let hi = am as u128 * (b.m >> 64);
        // (hi << 64) + lo, shifted right by 8 so it fits
        let m = (hi << 56) + (lo >> 8);
        Big {
            neg: an != b.neg,
            e: ae + b.e + 8,
            m,
            sticky: lo & 0xFF != 0,
        }
    }

    /// Normalised so that the top bit of `m` is bit 63 (exact; the value must have at most 64
    /// significant bits and no sticky part, as every X87 value does).
    pub fn norm64(self) -> (bool, i32, u64) {
        debug_assert!(self.m != 0 && !self.sticky);
        let t = self.top();
        let (m, e) = if t > 63 {
            let s = (t - 63) as u32;
            debug_assert!(self.m & ((1u128 << s) - 1) == 0, "more than 64 significant bits");
            (self.m >> s, self.e + s as i32)
        } else {
            (self.m << (63 - t) as u32, self.e - (63 - t))
        };
        (self.neg, e, m as u64)
    }

    /// Rounds to nearest-even with a `p`-bit significand and, when given, no grid finer than
    /// `2^min_e` (subnormal rounding). The result is exact: `m < 2^(p+1)` and no sticky part.
    pub fn round(self, p: u32, min_e: Option<i32>) -> Big {
        if self.m == 0 {
            return Big::zero(self.neg);
        }
        let mut d = self.top() + 1 - p as i32;
        if let Some(me) = min_e {
            d = d.max(me - self.e);
        }
        if d <= 0 {
            // every bit is kept; the callers always produce at least two bits beyond `p` when a
            // sticky part is present
            debug_assert!(!self.sticky, "sticky value with too few bits to round");
            return Big {
                sticky: false,
                ..self
            };
        }
        let d = d as u32;
        let (q, up) = if d >= 128 {
            // the whole value is below 2^(e+d-1) only when d > top+1; at d == 128 the top bit may be
            // the half bit
            let half = d == 128 && self.m >> 127 == 1;
            let above = half && (self.m & !(1u128 << 127) != 0 || self.sticky);
            (0u128, above)
        } else {
            let q = self.m >> d;
            let r = self.m & ((1u128 << d) - 1);
            let half = 1u128 << (d - 1);
            let up = r > half || (r == half && (self.sticky || q & 1 == 1));
            (q, up)
        };
        Big {
            neg: self.neg,
            e: self.e + d as i32,
            m: q + up as u128,
            sticky: false,
        }
    }

    /// The exact f64 value of a rounded `Big` (its significand must fit 53 bits and the exponent the
    /// f64 range).
    pub fn to_f64(self) -> f64 {
        let v = libm::scalbn(self.m as f64, self.e);
        if self.neg { -v } else { v }
    }

    pub fn mul(a: Big, b: Big) -> Big {
        let (an, ae, am) = a.norm64();
        let (bn, be, bm) = b.norm64();
        Big {
            neg: an != bn,
            e: ae + be,
            m: am as u128 * bm as u128,
            sticky: false,
        }
    }

    pub fn div(a: Big, b: Big) -> Big {
        let (an, ae, am) = a.norm64();
        let (bn, be, bm) = b.norm64();
        // 64 + 2 quotient bits at least (enough to round to 64 bits with the sticky remainder)
        let num = (am as u128) << 64;
        let (q1, r1) = (num / bm as u128, num % bm as u128);
        let n2 = r1 << 2;
        let (q2, r) = (n2 / bm as u128, n2 % bm as u128);
        Big {
            neg: an != bn,
            e: ae - 66 - be,
            m: (q1 << 2) | q2,
            sticky: r != 0,
        }
    }

    /// `a + b` for non-zero operands (`-0` results of exact cancellation are `+0`, round to nearest).
    pub fn add(a: Big, b: Big) -> Big {
        let (an, ae, am) = a.norm64();
        let (bn, be, bm) = b.norm64();
        // a = the operand with the larger exponent
        let (an, ae, am, bn, be, bm) = if ae >= be {
            (an, ae, am, bn, be, bm)
        } else {
            (bn, be, bm, an, ae, am)
        };
        let diff = (ae - be) as u32;
        if diff <= 62 {
            let x = (am as i128) << diff;
            let y = bm as i128;
            let s = if an { -x } else { x } + if bn { -y } else { y };
            return Big {
                neg: s < 0,
                e: be,
                m: s.unsigned_abs(),
                sticky: false,
            };
        }
        // b is below a's last bit: a (shifted up 62 bits) plus b's top part, the rest sticky
        let x = (am as u128) << 62;
        let s = diff - 62;
        let (y, rem) = if s >= 64 {
            (0u128, true)
        } else {
            (bm as u128 >> s, (bm as u128) & ((1u128 << s) - 1) != 0)
        };
        let m = if an == bn { x + y } else { x - y - rem as u128 };
        Big {
            neg: an,
            e: ae - 62,
            m,
            sticky: rem,
        }
    }

    /// Square root with at least 63 result bits (enough to round to 24).
    pub fn sqrt(a: Big) -> Big {
        let (_, mut e, m) = a.norm64();
        let mut m = m as u128;
        if e & 1 != 0 {
            m <<= 1;
            e -= 1;
        }
        let n = m << 62;
        let r = n.isqrt();
        Big {
            neg: false,
            e: (e - 62) / 2,
            m: r,
            sticky: r * r != n,
        }
    }
}

// ---- double-double -------------------------------------------------------------------------------

/// An unevaluated sum `h + l` with `|l| <= ulp(h)/2` (about 106 significant bits).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Dd {
    pub h: f64,
    pub l: f64,
}

fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}

fn quick_two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    (s, b - (s - a))
}

impl Dd {
    pub fn new(h: f64) -> Dd {
        Dd { h, l: 0.0 }
    }

    pub fn add(self, o: Dd) -> Dd {
        let (s, e) = two_sum(self.h, o.h);
        let (t, f) = two_sum(self.l, o.l);
        let (s, e) = quick_two_sum(s, e + t);
        let (h, l) = quick_two_sum(s, e + f);
        Dd { h, l }
    }

    pub fn neg(self) -> Dd {
        Dd {
            h: -self.h,
            l: -self.l,
        }
    }

    pub fn sub(self, o: Dd) -> Dd {
        self.add(o.neg())
    }

    pub fn mul(self, o: Dd) -> Dd {
        let p = self.h * o.h;
        let e = libm::fma(self.h, o.h, -p);
        let e = e + (self.h * o.l + self.l * o.h);
        let (h, l) = quick_two_sum(p, e);
        Dd { h, l }
    }

    pub fn div(self, o: Dd) -> Dd {
        let q1 = self.h / o.h;
        let r = self.sub(o.mul(Dd::new(q1)));
        let q2 = r.h / o.h;
        let r = r.sub(o.mul(Dd::new(q2)));
        let q3 = r.h / o.h;
        let (h, l) = quick_two_sum(q1, q2);
        Dd { h, l }.add(Dd::new(q3))
    }

    /// Exact scaling by a power of two (no overflow or underflow in the modelled range).
    pub fn scale(self, n: i32) -> Dd {
        Dd {
            h: libm::scalbn(self.h, n),
            l: libm::scalbn(self.l, n),
        }
    }
}

/// ln 2 as a double-double.
const LN2: Dd = Dd {
    h: f64::from_bits(0x3FE6_2E42_FEFA_39EF),
    l: f64::from_bits(0x3C7A_BC9E_3B39_803F),
};

/// `log2(x)` for a finite positive `x = hi + lo`, to about 104 bits. Exact (an integer) for powers of
/// two.
pub(crate) fn log2_dd(hi: f64, lo: f64) -> Dd {
    let b = Big::from_pair(hi, lo);
    let (_, e, m) = b.norm64(); // x = m * 2^e, m in [2^63, 2^64)
    // x = mm * 2^k with mm in [sqrt(1/2), sqrt(2))
    let mut k = e + 63;
    let mut sc = -(e + 63);
    if m > 0xB504_F333_F9DE_6484 {
        // m / 2^63 > sqrt(2)
        k += 1;
        sc -= 1;
    }
    let mm = Dd { h: hi, l: lo }.scale(sc);
    if mm.h == 1.0 && mm.l == 0.0 {
        return Dd::new(f64::from(k));
    }
    let one = Dd::new(1.0);
    let z = mm.sub(one).div(mm.add(one));
    let z2 = z.mul(z);
    let mut term = z;
    let mut sum = z;
    let mut n = 1.0;
    loop {
        term = term.mul(z2);
        n += 2.0;
        let t = term.div(Dd::new(n));
        sum = sum.add(t);
        if t.h.abs() < sum.h.abs() * 1e-34 {
            break;
        }
    }
    // ln(mm) = 2 atanh(z)
    let ln = sum.scale(1);
    Dd::new(f64::from(k)).add(ln.div(LN2))
}

/// `tan(r)` for `|r| <= 0.8` (a double-double), to about 104 bits, from the sine and cosine series.
pub(crate) fn tan_dd(r: Dd) -> Dd {
    if r.h == 0.0 {
        return r;
    }
    let r2 = r.mul(r);
    // sin = r - r^3/3! + ..., cos = 1 - r^2/2! + ...
    let mut s = r;
    let mut c = Dd::new(1.0);
    let mut ts = r;
    let mut tc = Dd::new(1.0);
    let mut n = 1.0;
    loop {
        tc = tc.mul(r2).div(Dd::new(-(n * (n + 1.0))));
        c = c.add(tc);
        ts = ts.mul(r2).div(Dd::new(-((n + 1.0) * (n + 2.0))));
        s = s.add(ts);
        n += 2.0;
        if ts.h.abs() < s.h.abs() * 1e-34 && tc.h.abs() < 1e-34 {
            break;
        }
    }
    s.div(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_round() {
        // 1 + 2^-24 is a tie: to even (1)
        let b = Big {
            neg: false,
            e: -24,
            m: (1 << 24) + 1,
            sticky: false,
        };
        assert_eq!(b.round(24, None).to_f64(), 1.0);
        // with a sticky part it is above the tie
        let b = Big { sticky: true, ..b };
        assert_eq!(b.round(24, None).to_f64(), 1.0 + 2f64.powi(-23));
    }

    #[test]
    fn pairs() {
        let v = Big::from_pair(1.0, -(2f64.powi(-60)));
        let (_, e, m) = v.norm64();
        assert_eq!((e, m), (-64, u64::MAX - 15));
    }

    #[test]
    fn log2_and_tan() {
        let l = log2_dd(10.0, 0.0);
        assert_eq!(l.h, core::f64::consts::LOG2_10);
        assert!(log2_dd(8.0, 0.0).h == 3.0 && log2_dd(8.0, 0.0).l == 0.0);
        let t = tan_dd(Dd::new(0.5));
        assert_eq!(t.h, libm::tan(0.5));
    }
}
