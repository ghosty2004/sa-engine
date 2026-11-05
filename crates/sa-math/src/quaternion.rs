//! `CQuaternion`, ported from the 1.0 US machine code (conventions as in `vector.rs`).

use crate::consts as k;
use crate::matrix::RwMatrix;
use crate::vector::CVector;
use crate::x87::X87;

/// `CQuaternion` (16 bytes): imaginary part x, y, z, real part w.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CQuaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

const _: () = {
    assert!(core::mem::size_of::<CQuaternion>() == 0x10);
    assert!(core::mem::offset_of!(CQuaternion, w) == 0xC);
};

fn ld(v: f32) -> X87 {
    X87::from_f32(v)
}

fn mul(a: f32, b: f32) -> X87 {
    ld(a).fmul_m(b)
}

impl CQuaternion {
    /// `CQuaternion::Normalise` (0x4D1610): `s = ((x*x + y*y) + z*z) + w*w`; `s == 0` sets only
    /// `w = 1.0`; otherwise (NaN included, `test ah,44h; jp`) every component times `1 / sqrt(s)`.
    pub fn normalise(&mut self) {
        let (x, y, z, w) = (ld(self.x), ld(self.y), ld(self.z), ld(self.w));
        let s = x.fmul(x).fadd(y.fmul(y)).fadd(z.fmul(z)).fadd(w.fmul(w));
        if !s.fcom(X87::zero()).test_parity(0x44) {
            self.w = f32::from_bits(k::F1);
            return;
        }
        let inv = s.fsqrt().fdivr_m(f32::from_bits(k::F1));
        self.x = inv.fmul_m(self.x).to_f32();
        self.y = inv.fmul_m(self.y).to_f32();
        self.z = inv.fmul_m(self.z).to_f32();
        self.w = inv.fmul_m(self.w).to_f32();
    }

    /// `CQuaternion::Get(RwMatrix*)` (0x59C080): the rotation matrix of the quaternion into the 3x3 part
    /// of `out` (position, flags and padding untouched).
    pub fn get(&self, out: &mut RwMatrix) {
        let one = f32::from_bits(k::F1);
        let x2 = ld(self.x).fdouble();
        let y2 = ld(self.y).fdouble();
        let z2 = ld(self.z).fdouble().to_f32();
        let xx = x2.fmul_m(self.x).to_f32();
        let xy = y2.fmul_m(self.x).to_f32();
        let xz = mul(z2, self.x).to_f32();
        let yy = y2.fmul_m(self.y).to_f32();
        let yz = mul(z2, self.y).to_f32();
        let zz = mul(z2, self.z).to_f32();
        let xw = x2.fmul_m(self.w).to_f32();
        let yw = y2.fmul_m(self.w);
        let zw = mul(z2, self.w);
        out.right = CVector::new(
            ld(zz).fadd_m(yy).fsubr_m(one).to_f32(),
            zw.fadd_m(xy).to_f32(),
            ld(xz).fsub(yw).to_f32(),
        );
        out.forward = CVector::new(
            ld(xy).fsub(zw).to_f32(),
            ld(zz).fadd_m(xx).fsubr_m(one).to_f32(),
            ld(xw).fadd_m(yz).to_f32(),
        );
        out.up = CVector::new(
            yw.fadd_m(xz).to_f32(),
            ld(yz).fsub_m(xw).to_f32(),
            ld(yy).fadd_m(xx).fsubr_m(one).to_f32(),
        );
    }
}
