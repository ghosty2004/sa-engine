//! `TransformVectors` (batch `mvpk-math-1`): transforms an array of vectors by a `CMatrix` through the shared work
//! `RwMatrix`. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;

/// The shared work `RwMatrix*` (0xB72920)
const WORK_RW_MATRIX: usize = 0xB72920;

use sa_core::bindings::matrix_copy_to_rw_matrix;
use sa_core::bindings::rw_matrix_update;
use sa_core::bindings::rw_v3d_transform_vector;

fn work_matrix() -> GPtr<u8> {
    GPtr::<GPtr<u8>>::from_addr(WORK_RW_MATRIX).read()
}

/// `TransformVectors(RwV3d* out, int32 count, const CMatrix& m, const RwV3d* in)` (stub 0x54EEA0, cdecl): `m` is copied
/// into the work `RwMatrix` (`CopyToRwMatrix`, `RwMatrixUpdate`), then each of the `count` vectors (none when zero; the
/// count is decremented to zero, so a negative count runs 2^32 times: original) is `RwV3dTransformVector(out + i, in +
/// i, work)` (the work pointer re-read per vector).
#[hook(addr = 0x54eea0, conv = cdecl, name = "TransformVectors", expect = "E9 D0 94 EB FF 8B 4C 24 0C",
       route = "L2", hazards = "none",
       note = "the entry fragment 0x408375 loads the work matrix pointer through rol([0x15608C8], [0x642511]); the port reads 0xB72920 (assumed no-CD VM value)")]
pub fn transform_vectors(out: GPtr<u8>, count: i32, m: GPtr<u8>, inp: GPtr<u8>) {
    matrix_copy_to_rw_matrix(m, work_matrix());
    rw_matrix_update(work_matrix());
    let mut n = count as u32;
    let (mut o, mut i) = (out, inp);
    while n != 0 {
        rw_v3d_transform_vector(o, i, work_matrix());
        o = o.byte_add(0xC);
        i = i.byte_add(0xC);
        n = n.wrapping_sub(1);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_TRANSFORM_VECTORS];
