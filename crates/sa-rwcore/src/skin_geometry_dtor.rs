//! The RpSkin geometry extension destructor (batch `std8-rw-1`), ported from the 1.0 US bytes; it replaces
//! sa-game's stand-in, which handled only the case without a skin. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, rw_free, rw_memory_free, wr};

orig! {
    /// `_rpSkinDeinitialize(geometry)` (0x7C8820)
    fn skin_deinitialize(geom: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C8820 => crate::skin::rp_skin_deinitialize;
    /// `_rpSkinSplitDataDestroy(skin)` (0x7C8B10)
    fn skin_split_data_destroy(skin: GPtr<u8>) -> u32 = cdecl 0x7C8B10 => crate::skin::rp_skin_split_data_destroy;
}

/// The skin geometry extension offset global (0xC978A8) and the skin freelist (0xC978B8).
const SKIN_GEOMETRY_OFFSET: usize = 0xC978A8;
const SKIN_FREE_LIST: usize = 0xC978B8;

/// The RpSkin geometry extension destructor (0x7C6A10, cdecl `(object, offset, size)`, returns `object`): with a
/// skin, `_rpSkinDeinitialize(geometry)`, `RwFree` of the skin's bone data (+0x3C) when set,
/// `_rpSkinSplitDataDestroy(skin)`, the skin back to the skin freelist (`RwFreeListFree`) and the extension slot
/// cleared.
#[hook(addr = 0x7C6A10, conv = cdecl, name = "cb_SKINPLUGIN_RpGeometry_Dtor", expect = "A1 A8 78 C9 00 56 57",
       unverified = "std8-rw-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn skin_geometry_dtor(obj: GPtr<u8>, _off: u32, _size: u32) -> GPtr<u8> {
    let off = g::<u32>(SKIN_GEOMETRY_OFFSET).read() as isize;
    let skin = p8(rd::<u32>(obj, off));
    if skin.is_null() {
        return obj;
    }
    skin_deinitialize(obj);
    let bones = p8(rd::<u32>(skin, 0x3C));
    if !bones.is_null() {
        rw_free(bones);
    }
    skin_split_data_destroy(skin);
    rw_memory_free(p8(g::<u32>(SKIN_FREE_LIST).read()), skin);
    wr::<u32>(obj, g::<u32>(SKIN_GEOMETRY_OFFSET).read() as isize, 0);
    obj
}

pub const HOOKS: &[HookReg] = &[HOOK_SKIN_GEOMETRY_DTOR];
