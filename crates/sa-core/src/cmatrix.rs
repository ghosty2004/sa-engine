//! `CMatrix` (MATH) ports reached by the native boot (STD-3). `CMatrix` layout: right +0x00,
//! flags +0x0C, forward +0x10, pad +0x1C, up +0x20, pad +0x2C, pos +0x30, pad +0x3C,
//! `m_pAttachMatrix` +0x40, `m_bOwnsAttachedMatrix` +0x44. Written from the 1.0 US bytes, not
//! yet verified (phase A queue).

use crate::abi::HookReg;
use crate::hook;
use crate::mem::{GPtr, GlobalDesc};

/// `CMatrix::SetScale(float)` (0x59AED0, thiscall): a scale matrix without touching the flag and
/// pad words: right = (s, 0, 0), forward = (0, s, 0), up = (0, 0, s), pos = 0. The float is copied
/// as its bits (integer moves).
#[hook(addr = 0x59AED0, conv = thiscall, name = "CMatrix::SetScale-f", expect = "8B 44 24 04 8B 54 24 04")]
pub fn set_scale(this: GPtr<u8>, s: f32) {
    let w = |o: isize, v: u32| this.byte_add(o).cast::<u32>().write(v);
    let b = s.to_bits();
    w(0x00, b);
    for o in [0x04, 0x08, 0x10] {
        w(o, 0);
    }
    w(0x14, b);
    for o in [0x18, 0x20, 0x24] {
        w(o, 0);
    }
    w(0x28, b);
    for o in [0x30, 0x34, 0x38] {
        w(o, 0);
    }
}

crate::orig! {
    /// `RwMatrixDestroy(RwMatrix*)` (0x7F2A20)
    fn rw_matrix_destroy(m: GPtr<u8>) -> i32 = cdecl 0x7F2A20 => unported;
}

/// `CMatrix::Attach(RwMatrix* m, bool owns)` (0x59BD10, thiscall): an owned previously attached
/// matrix is destroyed (`RwMatrixDestroy`), then `m_pAttachMatrix = m`, `m_bOwnsAttachedMatrix =
/// owns` (zero-extended into the dword), and the 12 vector words right/forward/up/pos copied from
/// `m` (the flag and pad words are not; the original does not `Update`).
#[hook(addr = 0x59BD10, conv = thiscall, name = "CMatrix::Attach", expect = "56 8B F1 8B 46 40")]
pub fn attach(this: GPtr<u8>, m: GPtr<u8>, owns: u8) {
    let r = |p: GPtr<u8>, o: isize| p.byte_add(o).cast::<u32>().read();
    let old = r(this, 0x40);
    if old != 0 && r(this, 0x44) != 0 {
        rw_matrix_destroy(GPtr::from_addr(old as usize));
    }
    this.byte_add(0x40).cast::<u32>().write(m.addr() as u32);
    this.byte_add(0x44).cast::<u32>().write(owns as u32);
    for o in [0x00, 0x04, 0x08, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28, 0x30, 0x34, 0x38] {
        this.byte_add(o).cast::<u32>().write(r(m, o));
    }
}

crate::orig! {
    /// `RwMatrixDestroy(RwMatrix*)` (0x7F2A20)
    fn cm_rw_matrix_destroy(m: GPtr<u8>) -> i32 = cdecl 0x7F2A20 => unported;
}

/// `CMatrix::~CMatrix()` (0x59ACD0, thiscall): when the matrix owns its RW matrix (+0x44) and that
/// pointer (+0x40) is set, `RwMatrixDestroy(+0x40)`.
///
/// Not registered: 0x59ACD0 is hooked by sa-world mvpa_math.rs (duplicate port left unregistered when STD-5 merged both).
pub fn matrix_dtor(this: GPtr<u8>) {
    if this.byte_add(0x44).cast::<u32>().read() != 0 {
        let m = this.byte_add(0x40).cast::<u32>().read();
        if m != 0 {
            cm_rw_matrix_destroy(GPtr::from_addr(m as usize));
        }
    }
}


/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_SET_SCALE, HOOK_ATTACH];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
