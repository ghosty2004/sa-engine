//! STD-5: the plain-copy `CMatrix` members (no arithmetic), ported from the 1.0 US bytes. `CMatrix`
//! (0x48): right +0, forward +0x10, up +0x20, position +0x30, +0x40 the attached `RwMatrix`, +0x44 owns it.

use crate::abi::{HookReg, NativeReg};
use crate::mem::GPtr;
use crate::{hook, orig};

orig! {
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0)
    fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18A0 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70)
    fn update_rw_matrix_call(this: GPtr<u8>, rw: GPtr<u8>) -> GPtr<u8> = thiscall 0x59AD70 => update_rw_matrix;
}

fn w(p: GPtr<u8>, o: isize, v: u32) {
    p.byte_add(o).cast::<u32>().write(v)
}
fn r(p: GPtr<u8>, o: isize) -> u32 {
    p.byte_add(o).cast::<u32>().read()
}
/// The twelve vector words (right, forward, up, position; not the +0xC/+0x1C/+0x2C/+0x3C words).
const VEC_WORDS: [isize; 12] = [0, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28, 0x30, 0x34, 0x38];
const ONE: u32 = 0x3F80_0000;

/// `CMatrix::SetTranslate(float x, float y, float z)` (0x59AF40): identity rotation, position (x, y, z).
#[hook(addr = 0x59AF40, conv = thiscall, name = "CMatrix::SetTranslate", expect = "33 C0 BA 00 00 80 3F 89 41 04",
       unverified = "std5-math-2", route = "L2", hazards = "none", recs = "new-game")]
pub fn set_translate(this: GPtr<u8>, x: u32, y: u32, z: u32) {
    for o in [4, 8, 0x10, 0x18, 0x20, 0x24] {
        w(this, o, 0);
    }
    w(this, 0, ONE);
    w(this, 0x14, ONE);
    w(this, 0x28, ONE);
    w(this, 0x30, x);
    w(this, 0x34, y);
    w(this, 0x38, z);
}

/// `CMatrix::SetUnity()` (0x59AE70, protected: `xor eax, eax; jmp 0x401BBB` loads 1.0 into EDX): the
/// identity with position 0.
#[crate::native(addr = 0x59AE70, conv = thiscall, name = "CMatrix::SetUnity")]
pub fn set_unity(this: GPtr<u8>) {
    for o in VEC_WORDS {
        w(this, o, 0);
    }
    w(this, 0, ONE);
    w(this, 0x14, ONE);
    w(this, 0x28, ONE);
}

/// `CMatrix::SetScale(float x, float y, float z)` (0x59AF00): diagonal (x, y, z), position 0.
#[hook(addr = 0x59AF00, conv = thiscall, name = "CMatrix::SetScale-fff", expect = "8B 44 24 04 8B 54 24 08 89 01",
       unverified = "std5-math-2", route = "L2", hazards = "none", recs = "new-game")]
pub fn set_scale(this: GPtr<u8>, x: u32, y: u32, z: u32) {
    for o in VEC_WORDS {
        w(this, o, 0);
    }
    w(this, 0, x);
    w(this, 0x14, y);
    w(this, 0x28, z);
}

/// `CMatrix::CopyOnlyMatrix(const CMatrix&)` (0x59ADD0): the first 0x40 bytes.
#[hook(addr = 0x59ADD0, conv = thiscall, name = "CMatrix::CopyOnlyMatrix", expect = "56 8B 74 24 08 57 8B F9 B9 10",
       unverified = "std5-math-2", route = "L2", hazards = "none", recs = "new-game")]
pub fn copy_only_matrix(this: GPtr<u8>, src: GPtr<u8>) {
    for k in 0..0x10isize {
        w(this, 4 * k, r(src, 4 * k));
    }
}

/// `CMatrix::operator=(const CMatrix&)` (0x59BBC0, protected chunk 0x405FCA): the first 0x40 bytes,
/// then the attached `RwMatrix` (when there is one) updated; returns `this`.
#[crate::native(addr = 0x59BBC0, conv = thiscall, name = "CMatrix::operator=")]
pub fn assign(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    let attached = r(this, 0x40);
    for k in 0..0x10isize {
        w(this, 4 * k, r(src, 4 * k));
    }
    if attached != 0 {
        update_rw_matrix_call(this, GPtr::from_addr(attached as usize));
    }
    this
}

/// `CMatrix::CopyToRwMatrix(RwMatrix*)` (0x59B8B0): the four vectors, then `RwMatrixUpdate`; returns
/// the `RwMatrix`.
#[hook(addr = 0x59B8B0, conv = thiscall, name = "CMatrix::CopyToRwMatrix", expect = "8B C1 56 8B 74 24 08 57 8B 38",
       unverified = "std5-math-2", route = "L2", hazards = "none", recs = "new-game")]
pub fn copy_to_rw_matrix(this: GPtr<u8>, rw: GPtr<u8>) -> GPtr<u8> {
    for o in VEC_WORDS {
        w(rw, o, r(this, o));
    }
    rw_matrix_update(rw);
    rw
}

/// `CMatrix::UpdateMatrix(RwMatrix*)` (0x59AD20): the four vectors from the `RwMatrix`.
#[hook(addr = 0x59AD20, conv = thiscall, name = "CMatrix::UpdateMatrix", expect = "8B 44 24 04 8B 10 89 11 8B 50",
       unverified = "std5-math-2", route = "L2", hazards = "none", recs = "new-game")]
pub fn update_matrix(this: GPtr<u8>, rw: GPtr<u8>) {
    for o in VEC_WORDS {
        w(this, o, r(rw, o));
    }
}

/// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70): the four vectors into the `RwMatrix`, then
/// `RwMatrixUpdate` (its result returned).
#[hook(addr = 0x59AD70, conv = thiscall, name = "CMatrix::UpdateRwMatrix", expect = "8B 11 8B 44 24 04 89 10 8B 51",
       unverified = "std5-math-2", route = "L2", hazards = "none", recs = "new-game")]
pub fn update_rw_matrix(this: GPtr<u8>, rw: GPtr<u8>) -> GPtr<u8> {
    for o in VEC_WORDS {
        w(rw, o, r(this, o));
    }
    rw_matrix_update(rw)
}

/// `CMatrix::Update()` (0x59BB60): the four vectors from the attached `RwMatrix`.
#[hook(addr = 0x59BB60, conv = thiscall, name = "CMatrix::Update", expect = "8B 41 40 8B 10 89 11 8B 50 04",
       unverified = "std5-math-2", route = "L2", hazards = "none", recs = "new-game")]
pub fn update(this: GPtr<u8>) {
    let rw = GPtr::<u8>::from_addr(r(this, 0x40) as usize);
    for o in VEC_WORDS {
        w(this, o, r(rw, o));
    }
}

/// `CMatrix::UpdateRW()` (0x59BBB0): `UpdateRwMatrix(attached)` when there is one.
#[hook(addr = 0x59BBB0, conv = thiscall, name = "CMatrix::UpdateRW", expect = "8B 41 40 85 C0 74 06 50 E8",
       unverified = "std5-math-2", route = "L2", hazards = "none", recs = "new-game")]
pub fn update_rw(this: GPtr<u8>) {
    let a = r(this, 0x40);
    if a != 0 {
        update_rw_matrix_call(this, GPtr::from_addr(a as usize));
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_SET_TRANSLATE,
    HOOK_SET_SCALE,
    HOOK_COPY_ONLY_MATRIX,
    HOOK_COPY_TO_RW_MATRIX,
    HOOK_UPDATE_MATRIX,
    HOOK_UPDATE_RW_MATRIX,
    HOOK_UPDATE,
    HOOK_UPDATE_RW,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_SET_UNITY, NATIVE_ASSIGN];
