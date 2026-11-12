//! PORT-VEH-3 (batch `veh3-2`, SA:MP-executed rows first): removing a vehicle upgrade and the
//! atomic callback that destroys an upgrade atomic. Ported from the 1.0 US bytes; gta-reversed as a
//! reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{model_info, rd, wr};

orig! {
    /// `CVehicle::ClearVehicleUpgradeFlags(int32, int32)` (0x6D3210)
    fn clear_vehicle_upgrade_flags(v: GPtr<u8>, unused: u32, comp: u32) -> u8 = thiscall 0x6D3210 => crate::vehicle2::clear_vehicle_upgrade_flags;
    /// `CLinkedUpgradeList::FindOtherUpgrade(int16)` (0x4C74D0) on `CVehicleModelInfo::ms_linkedUpgrades` (0xB4E6D8): int16 in AX
    fn find_other_upgrade(list: GPtr<u8>, model: i32) -> i16 = thiscall 0x4C74D0 => unported;
    /// `CVehicle::RemoveReplacementUpgrade(int32)` (0x6D39E0)
    fn remove_replacement_upgrade(v: GPtr<u8>, node: u32) = thiscall 0x6D39E0 => crate::vehicle2::remove_replacement_upgrade;
    /// `RpClumpForAllAtomics(RpClump*, callback, void*)` (0x749B70)
    fn rp_clump_for_all_atomics(clump: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `CVisibilityPlugins::GetAtomicId(RpAtomic*)` (0x732370)
    fn vis_get_atomic_id(atomic: GPtr<u8>) -> u32 = cdecl 0x732370 => unported;
    /// `CVisibilityPlugins::GetModelInfo(RpAtomic*)` (0x732260)
    fn vis_get_model_info(atomic: GPtr<u8>) -> GPtr<u8> = cdecl 0x732260 => unported;
    /// `RpClumpRemoveAtomic(RpClump*, RpAtomic*)` (0x74A4C0)
    fn rp_clump_remove_atomic(clump: GPtr<u8>, atomic: GPtr<u8>) -> GPtr<u8> = cdecl 0x74A4C0 => unported;
    /// `RpAtomicDestroy(RpAtomic*)` (0x749DC0)
    fn rp_atomic_destroy(atomic: GPtr<u8>) -> i32 = cdecl 0x749DC0 => unported;
    /// `RwFrameDestroy(RwFrame*)` (0x7F05A0)
    fn rw_frame_destroy(f: GPtr<u8>) -> i32 = cdecl 0x7F05A0 => unported;
    /// `CBaseModelInfo::RemoveRef()` (0x4C4BB0)
    fn model_info_remove_ref(mi: GPtr<u8>) = thiscall 0x4C4BB0 => unported;
}

/// `CVehicle::RemoveUpgradeCB` (0x6D3300).
const REMOVE_UPGRADE_CB: u32 = 0x006D_3300;

/// The upgrade model's (component type, replacement flag): bits 10..14 and bit 8 of the model
/// info's flags word (+0x12).
fn upgrade_kind(mi: GPtr<u8>) -> (u32, bool) {
    let w = u32::from(rd::<u16>(mi, 0x12));
    ((w >> 10) & 0x1F, w & 0x100 != 0)
}

/// Remove the parts of an upgrade model: a replacement restores the node (`RemoveReplacementUpgrade`
/// with the component type), anything else destroys its atomics (`RemoveUpgradeCB`).
fn remove_parts(this: GPtr<u8>, t: u32, replacement: bool) -> bool {
    if replacement {
        remove_replacement_upgrade(this, t);
        true
    } else {
        rp_clump_for_all_atomics(rd::<GPtr<u8>>(this, 0x18), REMOVE_UPGRADE_CB, t);
        false
    }
}

/// `CVehicle::RemoveVehicleUpgrade(int32 model)` (0x6DF930): nothing when
/// `ClearVehicleUpgradeFlags(model, type)` handles it; else the upgrade's parts and those of its
/// linked upgrade (`FindOtherUpgrade`, -1 for none) are removed (a type-2 replacement also resets
/// +0x458 to 1.0 and restores the nodes 5, 4 and 7), and every upgrade slot (+0x43A, 15 x int16)
/// holding the model becomes -1.
#[hook(addr = 0x6DF930, conv = thiscall, name = "CVehicle::RemoveVehicleUpgrade", this = "CVehicle",
       expect = "55 8B 6C 24 08",
       unverified = "veh3-2", route = "L2", hazards = "heap-writes",
       note = "destroys RW atomics/frames of the clump")]
pub fn remove_vehicle_upgrade(this: GPtr<u8>, model: i32) {
    let mi = model_info(model);
    let (t, _) = upgrade_kind(mi);
    if clear_vehicle_upgrade_flags(this, model as u32, t) != 0 {
        return;
    }
    let other = i32::from(find_other_upgrade(GPtr::from_addr(0xB4_E6D8), model));
    let (t, repl) = upgrade_kind(mi);
    if remove_parts(this, t, repl) && t == 2 {
        wr::<u32>(this, 0x458, 0x3F80_0000);
        remove_replacement_upgrade(this, 5);
        remove_replacement_upgrade(this, 4);
        remove_replacement_upgrade(this, 7);
    }
    if other != -1 {
        let (t2, repl2) = upgrade_kind(model_info(other));
        remove_parts(this, t2, repl2);
    }
    for k in 0..15isize {
        let o = 0x43A + 2 * k;
        if i32::from(rd::<i16>(this, o)) == model {
            wr::<u16>(this, o, 0xFFFF);
        }
    }
}

/// `CVehicle::RemoveUpgradeCB(RpAtomic*, void* type)` (0x6D3300): an upgrade atomic (atomic id flag
/// 0x800) whose model's component type (flags bits 10..14) equals `type` and which is not a
/// replacement (bit 8) is removed from its clump and destroyed with its frame, and its model's
/// reference dropped. Returns the atomic.
#[hook(addr = 0x6D3300, conv = cdecl, name = "CVehicle::RemoveUpgradeCB",
       expect = "56 8B 74 24 08",
       unverified = "veh3-2", route = "L2", hazards = "heap-writes",
       note = "destroys RW atomics/frames")]
pub fn remove_upgrade_cb(atomic: GPtr<u8>, ty: u32) -> GPtr<u8> {
    if vis_get_atomic_id(atomic) & 0x800 == 0 {
        return atomic;
    }
    let (t, repl) = upgrade_kind(vis_get_model_info(atomic));
    if ty != t || repl {
        return atomic;
    }
    let mi = vis_get_model_info(atomic);
    let frame = rd::<GPtr<u8>>(atomic, 4);
    rp_clump_remove_atomic(rd::<GPtr<u8>>(atomic, 0x3C), atomic);
    rp_atomic_destroy(atomic);
    rw_frame_destroy(frame);
    if !mi.is_null() {
        model_info_remove_ref(mi);
    }
    atomic
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_REMOVE_VEHICLE_UPGRADE, HOOK_REMOVE_UPGRADE_CB];
