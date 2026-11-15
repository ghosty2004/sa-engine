//! FRONTIER-2 turn 2, lane B (batch `frontier2-b5`): core / math functions on the phase B frontier. Ported from the
//! 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_math::x87::X87;

use crate::abi::HookReg;
use crate::bindings::cross_product;
use crate::field::{rd, wr};
use crate::mem::GPtr;
use crate::bindings::matrix_reorthogonalise;
use crate::hook;

/// `CCompressedMatrixNotAligned::DecompressIntoFullMatrix(CMatrix& m)` (0x59B9F0, thiscall, `ret 4`; the L1-verified
/// model is `sa_math::compressed::CCompressedMatrixNotAligned::decompress_into_full_matrix`): right x/y/z and forward
/// x/y/z (+0 / +0x10 of `m`) = `fild(int8 +0xC..+0x11) * [0x859BCC]`, the up vector (+0x20) = `CrossProduct(right,
/// forward)` (0x59C730 into a stack temporary), the position (+0x30) = the dwords +0..+8, then `Reorthogonalise`.
#[hook(addr = 0x59B9F0, conv = thiscall, name = "CCompressedMatrixNotAligned::DecompressIntoFullMatrix",
       expect = "83 EC 1C 56 8B 74 24 24", this = "CCompressedMatrixNotAligned",
       unverified = "frontier2-b5", route = "L2", hazards = "none")]
pub fn decompress_into_full_matrix(this: GPtr<u8>, m: GPtr<u8>) {
    let k = GPtr::<f32>::from_addr(0x85_9BCC).read();
    for (i, o) in [0isize, 4, 8, 0x10, 0x14, 0x18].into_iter().enumerate() {
        let v = X87::from_i32(i32::from(rd::<i8>(this, 0xC + i as isize))).fmul_m(k).to_f32();
        wr::<f32>(m, o, v);
    }
    let mut tmp = [0u32; 3];
    let t = GPtr::from_raw(tmp.as_mut_ptr()).cast::<u8>();
    let r = cross_product(t, m, m.byte_add(0x10));
    for o in [0isize, 4, 8] {
        wr::<u32>(m, 0x20 + o, rd::<u32>(r, o));
    }
    for o in [0isize, 4, 8] {
        wr::<u32>(m, 0x30 + o, rd::<u32>(this, o));
    }
    matrix_reorthogonalise(m);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_DECOMPRESS_INTO_FULL_MATRIX];
