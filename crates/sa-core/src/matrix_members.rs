//! STD-7: remaining `CMatrix` members (`Detach`, `Scale`, `operator*=`), ported from the 1.0 US bytes.
//! Unverified. Layout: right +0, forward +0x10, up +0x20, position +0x30, `m_pAttachMatrix` +0x40,
//! `m_bOwnsAttachedMatrix` +0x44.

use sa_math::x87::X87;

use crate::abi::HookReg;
use crate::mem::GPtr;
use crate::{hook, orig};

orig! {
    /// `RwMatrixDestroy(RwMatrix*)` (0x7F2A20)
    fn mm_rw_matrix_destroy(m: GPtr<u8>) -> i32 = cdecl 0x7F2A20 => unported;
    /// `operator*(CMatrix* out, const CMatrix& a, const CMatrix& b)` (0x59BE30, cdecl, returns out)
    fn mm_matrix_mul(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59BE30 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    fn mm_matrix_assign(this: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn mm_matrix_dtor(this: GPtr<u8>) = thiscall 0x59ACD0 => unported;
}

/// `CMatrix::Detach()` (0x59ACF0, thiscall): when `m_pAttachMatrix` and `m_bOwnsAttachedMatrix` are both
/// non-zero, `RwMatrixDestroy(m_pAttachMatrix)`; then `m_pAttachMatrix = 0` (the flag is kept).
#[hook(addr = 0x59ACF0, conv = thiscall, name = "CMatrix::Detach", expect = "56 8B F1 8B 46 40 85 C0",
       unverified = "std7-math-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn detach(this: GPtr<u8>) {
    let m = this.byte_add(0x40).cast::<u32>().read();
    if m != 0 && this.byte_add(0x44).cast::<u32>().read() != 0 {
        mm_rw_matrix_destroy(GPtr::from_addr(m as usize));
    }
    this.byte_add(0x40).cast::<u32>().write(0);
}

/// `CMatrix::Scale(float s)` (0x459350, thiscall, `ret 4`): right, forward and up (x, y, z each) multiplied
/// by `s` (each product rounded to f32). The entry `lea eax, [ecx + 8]; jmp 0x406A51` continues in a
/// protection chunk in game `.text` (`mov ecx, 3; jmp 0x459358`), which sa-hook cannot patch: a phase B
/// native.
#[crate::native(addr = 0x459350, conv = thiscall, name = "CMatrix::Scale")]
pub fn scale(this: GPtr<u8>, s: f32) {
    for row in 0..3isize {
        for c in 0..3isize {
            let p = this.byte_add(row * 0x10 + c * 4).cast::<f32>();
            p.write(X87::from_f32(s).fmul_m(p.read()).to_f32());
        }
    }
}

/// `CMatrix::operator*=(const CMatrix& m)` (stub 0x411A80 → 0x156FF00, thiscall, `ret 4`): a stack
/// temporary = `*this * m` (`operator*` 0x59BE30), `*this = tmp` (`operator=`), the temporary destroyed
/// (`~CMatrix`); returns `this`. The temporary's attach words come from `operator*`.
#[hook(addr = 0x411A80, conv = thiscall, name = "CMatrix::operator*=", expect = "E9 7B E4 15 01",
       route = "L2", hazards = "uninit-locals", recs = "grove-walk")]
pub fn mul_assign(this: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> {
    let mut tmp = [0u32; 0x48 / 4];
    let t = GPtr::from_raw(tmp.as_mut_ptr()).cast::<u8>();
    let r = mm_matrix_mul(t, this, m);
    mm_matrix_assign(this, r);
    mm_matrix_dtor(t);
    this
}

/// `CCompressedMatrixNotAligned::CompressFromFullMatrix(CMatrix& m)` (0x59BAD0, thiscall, `ret 4`): right x/y/z
/// and forward x/y/z each `_ftol(v * [0x86532C])` (x87 product, low byte) into +0xC..+0x11; the position (dword
/// copies) into +0..+8.
#[hook(addr = 0x59BAD0, conv = thiscall, name = "CCompressedMatrixNotAligned::CompressFromFullMatrix", expect = "56 57 8B 7C 24 0C D9 07",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn compress_from_full_matrix(this: GPtr<u8>, m: GPtr<u8>) {
    let k = GPtr::<f32>::from_addr(0x86532C).read();
    for (i, o) in [0isize, 4, 8, 0x10, 0x14, 0x18].into_iter().enumerate() {
        let v = X87::from_f32(m.byte_add(o).cast::<f32>().read()).fmul_m(k).ftol() as u8;
        this.byte_add(0xC + i as isize).write(v);
    }
    for o in [0isize, 4, 8] {
        this.byte_add(o).cast::<u32>().write(m.byte_add(0x30 + o).cast::<u32>().read());
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_DETACH, HOOK_MUL_ASSIGN, HOOK_COMPRESS_FROM_FULL_MATRIX];

