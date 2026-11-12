//! `RwTexDictionaryFindHashNamedTexture` (0x734E50, batch `attr-8`), needed by the `CHandObject` constructor. The same
//! function is also ported on WRITER-SAMP's branch (batch `samp-render-2`, commit 39986d4c); keep one of the two when
//! both are integrated. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::rd;

orig! {
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => unported;
}

/// `RwTexDictionaryFindHashNamedTexture(RwTexDictionary*, uint32 hash)` (0x734E50, cdecl): walks the dictionary's
/// texture list (head at +8, each texture linked at its +8) and returns the first texture whose name (+0x10) has
/// `CKeyGen::GetUppercaseKey(name) == hash`, else null.
#[hook(addr = 0x734E50, conv = cdecl, name = "RwTexDictionaryFindHashNamedTexture", expect = "53 8B 5C 24 08",
       unverified = "attr-8", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn rw_tex_dictionary_find_hash_named_texture(dict: GPtr<u8>, hash: u32) -> GPtr<u8> {
    let head = dict.byte_add(8);
    let mut node = rd::<GPtr<u8>>(head, 0);
    while node != head {
        let tex = node.byte_add(-8);
        let name = tex.byte_add(0x10);
        if !name.is_null() && get_uppercase_key(name) == hash {
            return tex;
        }
        node = rd::<GPtr<u8>>(node, 0);
    }
    GPtr::NULL
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_RW_TEX_DICTIONARY_FIND_HASH_NAMED_TEXTURE];
