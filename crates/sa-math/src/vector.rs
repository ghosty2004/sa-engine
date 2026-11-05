//! `CVector` / `CVector2D`, ported from the 1.0 US machine code.
//!
//! Every function follows the original instruction sequence: which operand is loaded (`fld`) and which
//! is a memory operand (`fmul dword` etc., see [`crate::x87`] for why that matters for NaNs), the order
//! of the additions, and which intermediates are stored as f32 on the stack. Functions that return a
//! float return the ST0 register value ([`X87`]); `.to_f32()` is what a caller's `fstp dword` stores.

use crate::consts as k;
use crate::matrix::CMatrix;
use crate::x87::X87;

/// `CVector` (12 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// `CVector2D` (8 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CVector2D {
    pub x: f32,
    pub y: f32,
}

const _: () = {
    assert!(core::mem::size_of::<CVector>() == 0xC);
    assert!(core::mem::offset_of!(CVector, y) == 4);
    assert!(core::mem::offset_of!(CVector, z) == 8);
    assert!(core::mem::size_of::<CVector2D>() == 8);
    assert!(core::mem::offset_of!(CVector2D, y) == 4);
};

fn ld(v: f32) -> X87 {
    X87::from_f32(v)
}

/// `fld a; fmul b` (b a memory operand).
fn mul(a: f32, b: f32) -> X87 {
    ld(a).fmul_m(b)
}

impl CVector {
    pub const fn new(x: f32, y: f32, z: f32) -> CVector {
        CVector { x, y, z }
    }

    /// `CVector::Normalise` (0x59C910, thiscall). See [`normalise`].
    pub fn normalise(&mut self) {
        let [x, y, z] = normalise([self.x, self.y, self.z]);
        *self = CVector { x, y, z };
    }

    /// `CVector::Magnitude` (0x4082C0, thiscall): `sqrt((x*x + y*y) + z*z)`, all register operands.
    pub fn magnitude_x87(&self) -> X87 {
        let (x, y, z) = (ld(self.x), ld(self.y), ld(self.z));
        x.fmul(x).fadd(y.fmul(y)).fadd(z.fmul(z)).fsqrt()
    }

    /// `CVector::Magnitude2D` (0x406D50, thiscall): `sqrt(x*x + y*y)`.
    pub fn magnitude_2d_x87(&self) -> X87 {
        let (x, y) = (ld(self.x), ld(self.y));
        x.fmul(x).fadd(y.fmul(y)).fsqrt()
    }

    /// `CVector::NormaliseAndMag` (0x59C970, thiscall). Normalises like [`normalise`] and returns
    /// `1 / (1 / sqrt(s))` (the original divides the reciprocal again, it does not keep the root); when
    /// `s == 0` it sets only `x = 1.0` and returns 1.0.
    pub fn normalise_and_mag_x87(&mut self) -> X87 {
        let (x, y, z) = (ld(self.x), ld(self.y), ld(self.z));
        let s = x.fmul(x).fadd(y.fmul(y)).fadd(z.fmul(z));
        if !s.fcom(X87::zero()).test_parity(0x41) {
            self.x = f32::from_bits(k::F1);
            return X87::constant(k::F1);
        }
        let inv = s.fsqrt().fdivr_m(f32::from_bits(k::F1));
        self.x = inv.fmul_m(self.x).to_f32();
        self.y = inv.fmul_m(self.y).to_f32();
        self.z = inv.fmul_m(self.z).to_f32();
        inv.fdivr_m(f32::from_bits(k::F1))
    }

    /// `CVector::Sum(a, b)` (0x40FDD0, thiscall): `this = a + b`, one component at a time
    /// (`fld a.i; fadd b.i; fstp this.i`), so aliasing `this` with `a` or `b` gives the same result.
    pub fn sum(a: &CVector, b: &CVector) -> CVector {
        CVector {
            x: ld(a.x).fadd_m(b.x).to_f32(),
            y: ld(a.y).fadd_m(b.y).to_f32(),
            z: ld(a.z).fadd_m(b.z).to_f32(),
        }
    }

    /// `CVector::Difference(a, b)` (0x40FE00, thiscall): `this = a - b` (per component, alias-safe).
    pub fn difference(a: &CVector, b: &CVector) -> CVector {
        CVector {
            x: ld(a.x).fsub_m(b.x).to_f32(),
            y: ld(a.y).fsub_m(b.y).to_f32(),
            z: ld(a.z).fsub_m(b.z).to_f32(),
        }
    }

    /// `CVector::Cross_OG(a, b)` (0x70F890, thiscall): `this = a x b`. The original stores each
    /// component before it loads the next one's operands, so for `this` aliasing `a` or `b` use
    /// [`CVector::cross_og_aliased`].
    pub fn cross_og(a: &CVector, b: &CVector) -> CVector {
        let mut out = CVector::default();
        let (a, b) = (*a, *b);
        cross_og_seq(&mut out, |o, i| match i {
            Operand::A => a,
            Operand::B => b,
            Operand::Out => *o,
        });
        out
    }

    /// `Cross_OG` with `this` aliasing the operands as given (`a_is_this`, `b_is_this`).
    pub fn cross_og_aliased(this: &mut CVector, a: &CVector, b: &CVector, a_is_this: bool, b_is_this: bool) {
        let (a, b) = (*a, *b);
        cross_og_seq(this, |o, i| match i {
            Operand::A if a_is_this => *o,
            Operand::B if b_is_this => *o,
            Operand::A => a,
            Operand::B => b,
            Operand::Out => *o,
        });
    }

    /// `CVector::FromMultiply(m, v)` (0x59C670, thiscall): `this = m * v` (point: with translation).
    /// Stores x before loading y's operands: for `this` aliasing `v` use
    /// [`CVector::from_multiply_in_place`].
    pub fn from_multiply(m: &CMatrix, v: &CVector) -> CVector {
        let mut out = *v;
        from_multiply_seq(m, &mut out, *v, false, true);
        out
    }

    /// `FromMultiply` with `this == &v` (the original's store/load interleaving).
    pub fn from_multiply_in_place(&mut self, m: &CMatrix) {
        let v = *self;
        from_multiply_seq(m, self, v, true, true);
    }

    /// `CVector::FromMultiply3x3(m, v)` (0x59C6D0, thiscall): `this = m * v` without translation.
    /// For `this` aliasing `v` use [`CVector::from_multiply_3x3_in_place`].
    pub fn from_multiply_3x3(m: &CMatrix, v: &CVector) -> CVector {
        let mut out = *v;
        from_multiply_seq(m, &mut out, *v, false, false);
        out
    }

    /// `FromMultiply3x3` with `this == &v`.
    pub fn from_multiply_3x3_in_place(&mut self, m: &CMatrix) {
        let v = *self;
        from_multiply_seq(m, self, v, true, false);
    }
}

#[derive(Clone, Copy)]
enum Operand {
    A,
    B,
    Out,
}

/// The `Cross_OG` instruction sequence; `get` reads an operand at the moment the original loads it.
fn cross_og_seq(out: &mut CVector, get: impl Fn(&CVector, Operand) -> CVector) {
    use Operand::*;
    // fld b.z; fmul a.y; fld a.z; fmul b.y; fsubp; fstp x
    let x = mul(get(out, B).z, get(out, A).y).fsub(mul(get(out, A).z, get(out, B).y));
    out.x = x.to_f32();
    // fld a.z; fmul b.x; fld a.x; fmul b.z; fsubp; fstp y
    let y = mul(get(out, A).z, get(out, B).x).fsub(mul(get(out, A).x, get(out, B).z));
    out.y = y.to_f32();
    // fld a.x; fmul b.y; fld b.x; fmul a.y; fsubp; fstp z
    let z = mul(get(out, A).x, get(out, B).y).fsub(mul(get(out, B).x, get(out, A).y));
    out.z = z.to_f32();
    let _ = Out;
}

/// FromMultiply / FromMultiply3x3: per component `((m.up.i*v.z + m.r1*v.a) + m.r2*v.b) [+ m.pos.i]`.
fn from_multiply_seq(m: &CMatrix, out: &mut CVector, v0: CVector, alias: bool, pos: bool) {
    let v = |o: &CVector| if alias { *o } else { v0 };
    // x: fld up.x; fmul v.z; fld fwd.x; fmul v.y; faddp; fld right.x; fmul v.x; faddp [; fadd pos.x]
    let vv = v(out);
    let mut x = mul(m.up.x, vv.z)
        .fadd(mul(m.forward.x, vv.y))
        .fadd(mul(m.right.x, vv.x));
    if pos {
        x = x.fadd_m(m.pos.x);
    }
    out.x = x.to_f32();
    // y: fld up.y; fmul v.z; fld right.y; fmul v.x; faddp; fld fwd.y; fmul v.y; faddp [; fadd pos.y]
    let vv = v(out);
    let mut y = mul(m.up.y, vv.z)
        .fadd(mul(m.right.y, vv.x))
        .fadd(mul(m.forward.y, vv.y));
    if pos {
        y = y.fadd_m(m.pos.y);
    }
    out.y = y.to_f32();
    let vv = v(out);
    let mut z = mul(m.up.z, vv.z)
        .fadd(mul(m.right.z, vv.x))
        .fadd(mul(m.forward.z, vv.y));
    if pos {
        z = z.fadd_m(m.pos.z);
    }
    out.z = z.to_f32();
}

/// `CVector::DotProduct(const RwV3d*, const RwV3d*)` (0x40FDB0, cdecl): `(a.z*b.z + a.y*b.y) + a.x*b.x`.
pub fn dot_product_x87(a: &CVector, b: &CVector) -> X87 {
    mul(a.z, b.z).fadd(mul(a.y, b.y)).fadd(mul(a.x, b.x))
}

/// `CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730, cdecl, returns `out`):
/// `a x b`; every operand is loaded before `out` is written (alias-safe).
pub fn cross_product(a: &CVector, b: &CVector) -> CVector {
    CVector {
        x: mul(b.z, a.y).fsub(mul(a.z, b.y)).to_f32(),
        y: mul(a.z, b.x).fsub(mul(b.z, a.x)).to_f32(),
        z: mul(a.x, b.y).fsub(mul(b.x, a.y)).to_f32(),
    }
}

/// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, hidden return pointer): the same sums
/// as [`CVector::from_multiply`], but every operand is loaded before the result is stored (alias-safe).
pub fn matrix_mul_vector(m: &CMatrix, v: &CVector) -> CVector {
    let mut out = *v;
    from_multiply_seq(m, &mut out, *v, false, true);
    out
}

impl CVector2D {
    /// `CVector2D::Magnitude` (0x420860, thiscall): `sqrt(x*x + y*y)`.
    pub fn magnitude_x87(&self) -> X87 {
        let (x, y) = (ld(self.x), ld(self.y));
        x.fmul(x).fadd(y.fmul(y)).fsqrt()
    }

    /// `CVector2D::Normalise` (0x44E480, thiscall): as `CVector::Normalise` in 2D; `s == 0` sets only
    /// `x = 1.0`.
    pub fn normalise(&mut self) {
        let (x, y) = (ld(self.x), ld(self.y));
        let s = x.fmul(x).fadd(y.fmul(y));
        if !s.fcom(X87::zero()).test_parity(0x41) {
            self.x = f32::from_bits(k::F1);
            return;
        }
        let inv = s.fsqrt().fdivr_m(f32::from_bits(k::F1));
        self.x = inv.fmul_m(self.x).to_f32();
        self.y = inv.fmul_m(self.y).to_f32();
    }

    /// `CVector2D::operator=` (0x43E110, thiscall, returns `this`): a raw two-word copy.
    pub fn assign(&mut self, o: &CVector2D) {
        *self = *o;
    }
}

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

    #[test]
    fn products() {
        let a = CVector::new(1.0, 0.0, 0.0);
        let b = CVector::new(0.0, 1.0, 0.0);
        assert_eq!(cross_product(&a, &b), CVector::new(0.0, 0.0, 1.0));
        assert_eq!(CVector::cross_og(&a, &b), CVector::new(0.0, 0.0, 1.0));
        assert_eq!(dot_product_x87(&a, &a).to_f32(), 1.0);
        assert_eq!(CVector::new(3.0, 4.0, 0.0).magnitude_x87().to_f32(), 5.0);
        let mut v = CVector::new(0.0, 3.0, 4.0);
        assert_eq!(v.normalise_and_mag_x87().to_f32(), 5.0);
        assert_eq!(v, CVector::new(0.0, 0.6, 0.8));
    }
}
