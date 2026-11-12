//! WRITER-WORLD (batch `world5-2`): small WORLD rows: `FurnitureGroup_c::Exit`,
//! `CHandObject::Render` and `CPickups::DetonateMinesHitByGunShot`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::rd;

orig! {
    /// `List_c::RemoveAll()` (0x4A8EB0)
    fn list_remove_all(l: GPtr<u8>) = thiscall 0x4A8EB0 => unported;
    /// `RwHelper::GetFirstAtomic(RpClump*)` (0x734820)
    fn get_first_atomic(clump: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `RpMaterialSetTexture(RpMaterial*, RwTexture*)` (0x74DBC0)
    fn rp_material_set_texture(m: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> = cdecl 0x74DBC0 => unported;
    /// `CObject::Render()` (0x59F180)
    fn object_render(o: GPtr<u8>) = thiscall 0x59F180 => unported;
    /// `CPickup::ProcessGunShot(CVector* start, CVector* end)` (0x4588B0)
    fn pickup_process_gun_shot(p: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = thiscall 0x4588B0 => unported;
}

/// `FurnitureGroup_c::Exit()` (0x5910B0): empties every sub-group's furniture list (sub-group +0xC;
/// the sub-groups are chained by +4 from the list head at +0), then the sub-group list itself
/// (tail jump).
#[hook(addr = 0x5910B0, conv = thiscall, name = "FurnitureGroup_c::Exit", this = "FurnitureGroup_c",
       expect = "56 57 8B F9 8B 37",
       unverified = "world5-2", route = "L2", hazards = "heap-writes",
       note = "unlinks list items (heap)")]
pub fn furniture_group_exit(this: GPtr<u8>) {
    let mut sg = rd::<GPtr<u8>>(this, 0);
    while !sg.is_null() {
        list_remove_all(sg.byte_add(0xC));
        sg = rd::<GPtr<u8>>(sg, 4);
    }
    list_remove_all(this);
}

/// `CPickups::DetonateMinesHitByGunShot(CVector* start, CVector* end)` (0x4590C0): every pickup of
/// `aPickUps` (0x9788C0..0x97D640, 0x20 bytes each) of type 12 (mine) processes the shot.
#[hook(addr = 0x4590C0, conv = cdecl, name = "CPickups::DetonateMinesHitByGunShot",
       expect = "55 8B 6C 24 08",
       unverified = "world5-2", route = "L2", hazards = "heap-writes",
       note = "a hit mine explodes (explosion list, pickup removal)")]
pub fn detonate_mines_hit_by_gun_shot(start: GPtr<u8>, end: GPtr<u8>) {
    let mut p = 0x97_88C0usize;
    while p < 0x97_D640 {
        let pk = GPtr::<u8>::from_addr(p);
        if rd::<u8>(pk, 0x1C) == 0xC {
            pickup_process_gun_shot(pk, start, end);
        }
        p += 0x20;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_FURNITURE_GROUP_EXIT,
    HOOK_DETONATE_MINES_HIT_BY_GUN_SHOT,
];
