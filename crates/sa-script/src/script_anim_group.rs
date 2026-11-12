//! STD-7: `CTheScripts::ScriptAttachAnimGroupToCharModel` (batch `std7-script-1`), ported from the 1.0 US
//! bytes; unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;

orig! {
    /// `CBaseModelInfo::GetAnimFileIndex()` (vtable +0x38)
    fn mi_get_anim_file_index(mi: GPtr<u8>) -> i32 = thiscall vslot 0x38 => unported;
    /// `CBaseModelInfo::SetAnimFile(const char*)` (vtable +0x30)
    fn mi_set_anim_file(mi: GPtr<u8>, name: GPtr<u8>) = thiscall vslot 0x30 => unported;
    /// `CBaseModelInfo::ConvertAnimFileIndex()` (vtable +0x34)
    fn mi_convert_anim_file_index(mi: GPtr<u8>) = thiscall vslot 0x34 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990)
    fn get_animation_block_index(name: GPtr<u8>) -> i32 = cdecl 0x4D3990 => unported;
}

/// `CTheScripts::ScriptAttachAnimGroupToCharModel(int32 model, const char* ifp)` (0x474800, cdecl): the model's
/// anim file index (`GetAnimFileIndex`) equal to `GetAnimationBlockIndex(ifp)` → false; else `SetAnimFile(ifp)`,
/// `ConvertAnimFileIndex()`, true. AL only (the upper EAX bytes are the last callee's).
#[hook(addr = 0x474800, conv = cdecl, name = "CTheScripts::ScriptAttachAnimGroupToCharModel", expect = "8B 44 24 04 53 56 8B 34 85 C8 B0 A9 00",
       unverified = "std7-script-1", route = "L2", hazards = "ax-al-return", recs = "new-game")]
pub fn script_attach_anim_group_to_char_model(model: i32, ifp: GPtr<u8>) -> u8 {
    let mi = GPtr::<GPtr<u8>>::from_addr(MODEL_INFO_PTRS).wrapping_index(model).read();
    let cur = mi_get_anim_file_index(mi);
    if cur == get_animation_block_index(ifp) {
        return 0;
    }
    mi_set_anim_file(mi, ifp);
    mi_convert_anim_file_index(mi);
    1
}

pub const HOOKS: &[HookReg] = &[HOOK_SCRIPT_ATTACH_ANIM_GROUP_TO_CHAR_MODEL];
