//! `FxSystem_c::AttachToBone(CEntity*, ePedBones)` (0x4AA400, batch `std12-render-1`): reached when a ped effect
//! (cigarette smoke, blood) follows a bone. Ported from the 1.0 US bytes (gta-reversed as a reading aid). Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{rd, wr};

orig! {
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    fn get_anim_hierarchy_from_skin_clump(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0), `RpHAnimHierarchyGetMatrixArray` (0x7C5120)
    fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    fn rp_hanim_hierarchy_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
}

/// `FxSystem_c::AttachToBone(CEntity* e, ePedBones bone)` (0x4AA400, thiscall, `ret 8`): the system's parent matrix
/// (+0xC) becomes the bone's matrix in the entity clump's skin hierarchy.
#[hook(addr = 0x4AA400, conv = thiscall, name = "FxSystem_c::AttachToBone", expect = "8B 44 24 04 53 56 8B D9",
       unverified = "std12-render-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn fx_system_attach_to_bone(this: GPtr<u8>, e: GPtr<u8>, bone: i32) {
    let h = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(e, 0x18));
    let i = rp_hanim_id_get_index(h, bone);
    let m = rp_hanim_hierarchy_get_matrix_array(h).byte_add((i << 6) as isize);
    wr::<u32>(this, 0xC, m.addr() as u32);
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_FX_SYSTEM_ATTACH_TO_BONE];
