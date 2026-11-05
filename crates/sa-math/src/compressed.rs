//! `CCompressedMatrixNotAligned`, ported from the 1.0 US machine code.

use crate::consts as k;
use crate::matrix::CMatrix;
use crate::vector::{self, CVector};
use crate::x87::X87;

/// `CCompressedMatrixNotAligned` (20 bytes): position, then right and forward as int8 triples scaled
/// by 127.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CCompressedMatrixNotAligned {
    pub pos: CVector,
    pub right: [i8; 3],
    pub forward: [i8; 3],
    pub pad: [u8; 2],
}

const _: () = {
    use core::mem::{offset_of, size_of};
    assert!(size_of::<CCompressedMatrixNotAligned>() == 0x14);
    assert!(offset_of!(CCompressedMatrixNotAligned, right) == 0xC);
    assert!(offset_of!(CCompressedMatrixNotAligned, forward) == 0xF);
};

impl CCompressedMatrixNotAligned {
    /// `DecompressIntoFullMatrix(CMatrix&)` (0x59B9F0): `right`/`forward` = `fild(int8) * (1/127)`,
    /// `up = CrossProduct(right, forward)`, `pos` (raw), then `Reorthogonalise`. Flags, padding and the
    /// attach fields of `out` are untouched.
    pub fn decompress_into_full_matrix(&self, out: &mut CMatrix) {
        let c = |v: i8| {
            X87::from_i32(i32::from(v))
                .fmul_m(f32::from_bits(k::DECOMPRESS_SCALE))
                .to_f32()
        };
        out.right = CVector::new(c(self.right[0]), c(self.right[1]), c(self.right[2]));
        out.forward = CVector::new(c(self.forward[0]), c(self.forward[1]), c(self.forward[2]));
        out.up = vector::cross_product(&out.right, &out.forward);
        out.pos = self.pos;
        out.reorthogonalise();
    }

    /// `CompressFromFullMatrix(CMatrix&)` (0x59BAD0): each of the six components times 127 (`fld m;
    /// fmul 127`), truncated by `_ftol2` (low byte kept), then `pos` (raw).
    pub fn compress_from_full_matrix(&mut self, m: &CMatrix) {
        let c = |v: f32| X87::from_f32(v).fmul_m(f32::from_bits(k::F127)).ftol() as u8 as i8;
        self.right = [c(m.right.x), c(m.right.y), c(m.right.z)];
        self.forward = [c(m.forward.x), c(m.forward.y), c(m.forward.z)];
        self.pos = m.pos;
    }
}
