//! RenderWare iteration callbacks that store the object they are given (batch `std8-render-1`), ported from the 1.0 US bytes. Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

/// `GetFirstTextureCallback(RwTexture* tex, void* data)` (0x734930, cdecl): `*(RwTexture**)data = tex`; returns
/// null (stops the `RwTexDictionaryForAllTextures` walk at the first texture).
#[hook(addr = 0x734930, conv = cdecl, name = "GetFirstTextureCallback", expect = "8B 44 24 08 8B 4C 24 04 89 08",
       unverified = "std8-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn get_first_texture_callback(tex: GPtr<u8>, data: GPtr<GPtr<u8>>) -> GPtr<u8> {
    data.write(tex);
    GPtr::NULL
}

/// `MarkerAtomicCB(RpAtomic* atomic, void* data)` (0x722220, cdecl): `*(RpAtomic**)data = atomic`; returns `atomic`
/// (the walk continues).
#[hook(addr = 0x722220, conv = cdecl, name = "MarkerAtomicCB", expect = "8B 44 24 04 8B 4C 24 08 89 01",
       unverified = "std8-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn marker_atomic_cb(atomic: GPtr<u8>, data: GPtr<GPtr<u8>>) -> GPtr<u8> {
    data.write(atomic);
    atomic
}

pub const HOOKS: &[HookReg] = &[HOOK_GET_FIRST_TEXTURE_CALLBACK, HOOK_MARKER_ATOMIC_CB];
