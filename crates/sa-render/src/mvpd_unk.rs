//! WRITER-MVP-D (batch `mvpd-unk-1`): small unclassified (UNK-game) helpers on the in-world path —
//! two `CMatrix` constructors, `CSimpleTransform::UpdateMatrix`, the colour `lerp`, `GetFirstTexture`
//! and `CloneAtomicToClumpCB`.
//!
//! Ported from the 1.0 US bytes; gta-reversed `Matrix.cpp`, `SimpleTransform.cpp`, `RwHelper.cpp` and
//! `Clump`/`Visibility` helpers as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{rd, wr, x};

orig! {
    /// `CMatrix::Attach(RwMatrix*, bool)` (0x59BD10)
    fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59BD10 => unported;
    /// `CMatrix::SetTranslate(float, float, float)` (0x59AF40)
    fn matrix_set_translate(m: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x59AF40 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020)
    fn matrix_set_rotate_z_only(m: GPtr<u8>, h: u32) = thiscall 0x59B020 => unported;
    /// `RwTexDictionaryForAllTextures(RwTexDictionary*, callback, data)` (0x7F3730)
    fn rw_tex_dictionary_for_all_textures(d: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3730 => unported;
    /// `RpAtomicClone(RpAtomic*)` (0x749E60)
    fn rp_atomic_clone(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x749E60 => unported;
    /// `RpAtomicSetFrame(RpAtomic*, RwFrame*)` (0x74BF20)
    fn rp_atomic_set_frame(a: GPtr<u8>, f: u32) -> GPtr<u8> = cdecl 0x74BF20 => unported;
    /// `RpClumpAddAtomic(RpClump*, RpAtomic*)` (0x74A490)
    fn rp_clump_add_atomic(c: GPtr<u8>, a: GPtr<u8>) -> GPtr<u8> = cdecl 0x74A490 => unported;
}

/// `CMatrix::CMatrix(RwMatrix*, bool owns)` (0x59C050, thiscall, `ret 8`): `m_pAttachMatrix` (+0x40) =
/// NULL, then `Attach(m, owns)`. Returns `this`.
#[hook(addr = 0x59C050, conv = thiscall, name = "CMatrix", expect = "8B 44 24 08 56",
       unverified = "mvpd-unk-1", route = "L2", hazards = "heap-writes",
       note = "the matrix is usually a heap or stack object; Attach copies the RwMatrix")]
pub fn matrix_ctor_rw(this: GPtr<u8>, m: GPtr<u8>, owns: u32) -> GPtr<u8> {
    wr::<u32>(this, 0x40, 0);
    matrix_attach(this, m, owns);
    this
}

/// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0, thiscall, `ret 4`): +0x40 and +0x44 cleared, the 16
/// dwords of the source copied. Returns `this`.
#[hook(addr = 0x59BCF0, conv = thiscall, name = "CMatrix", expect = "8B C1 56 8B 74 24 08",
       unverified = "mvpd-unk-1", route = "L2", hazards = "heap-writes",
       note = "copies into the constructed matrix (often a heap object)")]
pub fn matrix_ctor_copy(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x40, 0);
    wr::<u32>(this, 0x44, 0);
    for i in 0..16isize {
        wr::<u32>(this, i * 4, rd::<u32>(src, i * 4));
    }
    this
}

/// `CSimpleTransform::UpdateMatrix(CMatrix*) const` (0x54F1B0, thiscall, `ret 4`):
/// `m->SetTranslate(pos)` then `m->SetRotateZOnly(heading +0xC)`.
#[hook(addr = 0x54F1B0, conv = thiscall, name = "CSimpleTransform::UpdateMatrix", this = "CSimpleTransform", expect = "56 8B F1 8B 46 08",
       unverified = "mvpd-unk-1", route = "L2", hazards = "heap-writes", note = "writes the entity matrix (heap)")]
pub fn simple_transform_update_matrix(this: GPtr<u8>, m: GPtr<u8>) {
    matrix_set_translate(m, rd::<u32>(this, 0), rd::<u32>(this, 4), rd::<u32>(this, 8));
    matrix_set_rotate_z_only(m, rd::<u32>(this, 0xC));
}

/// `lerp` (0x6E5280, cdecl, the result through a hidden pointer): two 16-byte values by value (three
/// floats each) and `t`; `out = a * (1 - t) + b * t` per component (out +0xC/+0xD bytes cleared first).
/// Returns `out`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x6E5280, conv = cdecl, name = "lerp", expect = "D9 05 24 86 85 00",
       unverified = "mvpd-unk-1", route = "L2", hazards = "none")]
pub fn lerp(out: GPtr<u8>, ax: f32, ay: f32, az: f32, _a3: u32, bx: f32, by: f32, bz: f32, _b3: u32, t: f32) -> GPtr<u8> {
    let omt = X87::one().fsub_m(t);
    wr::<u8>(out, 0xC, 0);
    wr::<u8>(out, 0xD, 0);
    wr::<f32>(out, 0, x(ax).fmul(omt).fadd(x(bx).fmul_m(t)).to_f32());
    wr::<f32>(out, 4, x(ay).fmul(omt).fadd(x(by).fmul_m(t)).to_f32());
    wr::<f32>(out, 8, omt.fmul_m(az).fadd(x(bz).fmul_m(t)).to_f32());
    out
}

/// `GetFirstTexture(RwTexDictionary*)` (0x734940, cdecl): `RwTexDictionaryForAllTextures(dict, 0x734930,
/// &tex)` with tex = NULL; returns tex.
#[hook(addr = 0x734940, conv = cdecl, name = "GetFirstTexture", expect = "51 8B 4C 24 08",
       route = "L2", hazards = "none")]
pub fn get_first_texture(dict: GPtr<u8>) -> u32 {
    let mut tex = 0u32;
    rw_tex_dictionary_for_all_textures(dict, 0x734930, GPtr::from_raw(&mut tex as *mut u32).cast());
    tex
}

/// `CloneAtomicToClumpCB(RpAtomic*, void* clump)` (0x537290, cdecl): `RpAtomicClone(atomic)`, its frame
/// set to the atomic's frame's +0xA0, added to the clump. Returns the atomic.
#[hook(addr = 0x537290, conv = cdecl, name = "CloneAtomicToClumpCB", expect = "56 8B 74 24 08",
       unverified = "mvpd-unk-1", route = "L2", hazards = "heap-writes", note = "clones an RpAtomic (RW heap)")]
pub fn clone_atomic_to_clump_cb(atomic: GPtr<u8>, clump: GPtr<u8>) -> GPtr<u8> {
    let c = rp_atomic_clone(atomic);
    rp_atomic_set_frame(c, rd::<u32>(rd::<GPtr<u8>>(atomic, 4), 0xA0));
    rp_clump_add_atomic(clump, c);
    atomic
}

pub const HOOKS: &[HookReg] = &[
    HOOK_MATRIX_CTOR_RW,
    HOOK_MATRIX_CTOR_COPY,
    HOOK_SIMPLE_TRANSFORM_UPDATE_MATRIX,
    HOOK_LERP,
    HOOK_GET_FIRST_TEXTURE,
    HOOK_CLONE_ATOMIC_TO_CLUMP_CB,
];
