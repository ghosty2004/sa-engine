//! STREAM rows outside the big classes (specs in `docs/re/specs/STREAM/{CPool,CDummy,CDummyObject,
//! CAnimatedBuilding,CCollisionData,CPathFind}.md`, all from the 1.0 US bytes):
//! - the `CPool<T>` instantiations the stores use: `CPool<TxdDef>` ctor/New, `CPool<ColDef>` ctor,
//!   `CPool<CVehicleModelInfo::CVehicleStructure>` ctor (the generic algorithm is sa-core's
//!   `pool::pool_new` / `pools::pool_construct`, L1-verified there for other slot sizes; here re-run
//!   for these addresses);
//! - `CDummy::operator new`, the `CAnimatedBuilding` / `CDummyObject` constructors;
//! - `CCollisionData::GetNoOfShadowVertices` (proposed name, 0x537510);
//! - two empty `CPathFind` leftovers (`ret 0x44`).

use sa_core::mem::GPtr;
use sa_core::pool::{self, CPool};
use sa_core::pools::{self as core_pools, DUMMY_POOL, DummySlot, Slot};
use sa_core::{hook, orig};

use crate::shared::{ColDef, TxdDef, field};

sa_core::globals! {
    list GLOBALS;
}

/// `CVehicleModelInfo::CVehicleStructure` storage (0x314 bytes).
pub type VehicleStructureSlot = Slot<0x314>;

orig! {
    /// CRT `operator new` (0x82119A)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CRT `operator new[]` (0x821195: `jmp operator new`)
    pub fn operator_new_array(size: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// `CDummyPool::New()` (0x532630)
    pub fn dummy_pool_new(p: GPtr<CPool<DummySlot>>) -> GPtr<DummySlot> = thiscall 0x532630 => unported;
    /// `CBuilding::CBuilding()` (0x403E00)
    pub fn building_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x403E00 => unported;
    /// `CDummy::CDummy()` (0x532540)
    pub fn dummy_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x532540 => unported;
}

/// `CPool<TxdDef>::CPool(size, name)` (0x731AD0, thiscall, `ret 8`): both arrays from `operator new`.
#[hook(addr = 0x731AD0, conv = thiscall, name = "FUN_00731ad0", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn txd_pool_ctor(p: GPtr<CPool<TxdDef>>, size: i32, _name: GPtr<u8>) -> GPtr<CPool<TxdDef>> {
    core_pools::pool_construct(p, size, operator_new)
}

/// `CPool<TxdDef>::New()` (0x731B80, thiscall).
#[hook(addr = 0x731B80, conv = thiscall, name = "FUN_00731b80", expect = "8B 51 08 56 32 C0", this = "CPool<int32>")]
pub fn txd_pool_new(p: GPtr<CPool<TxdDef>>) -> GPtr<TxdDef> {
    pool::pool_new(p)
}

/// `CPool<ColDef>::CPool(size, name)` (stub 0x410F80 → .HOODLUM 0x15611F0, thiscall, `ret 8`): both
/// arrays from `operator new[]`.
#[hook(addr = 0x410F80, conv = thiscall, name = "thunk_FUN_015611f0", expect = "E9 6B 02 15 01", this = "CPool<int32>")]
pub fn col_pool_ctor(p: GPtr<CPool<ColDef>>, size: i32, _name: GPtr<u8>) -> GPtr<CPool<ColDef>> {
    core_pools::pool_construct(p, size, operator_new_array)
}

/// `CPool<CVehicleStructure>::CPool(size, name)` (0x5B8340, thiscall, `ret 8`): `operator new[]`.
#[hook(addr = 0x5B8340, conv = thiscall, name = "FUN_005b8340", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn vehicle_structure_pool_ctor(
    p: GPtr<CPool<VehicleStructureSlot>>,
    size: i32,
    _name: GPtr<u8>,
) -> GPtr<CPool<VehicleStructureSlot>> {
    core_pools::pool_construct(p, size, operator_new_array)
}

/// `CDummy::operator new(size)` (0x5326E0, cdecl): `CPools::ms_pDummyPool->New()` (tail jump; the
/// size is ignored, the pool pointer not null-checked).
#[hook(addr = 0x5326E0, conv = cdecl, name = "CDummy::operator new", expect = "8B 0D A0 44 B7 00")]
pub fn dummy_operator_new(_size: u32) -> GPtr<DummySlot> {
    dummy_pool_new(DUMMY_POOL.get())
}

/// `CCollisionData::GetNoOfShadowVertices()` (0x537510, thiscall; proposed name): 1 + the largest
/// shadow-triangle vertex index (unsigned compares), 0 for no triangles; a count negative as int32
/// skips the loop and returns 1.
#[hook(addr = 0x537510, conv = thiscall, name = "FUN_00537510", expect = "8B 51 20 33 C0")]
pub fn get_no_of_shadow_vertices(cd: GPtr<u8>) -> u32 {
    let n = field::<i32, _>(cd, 0x20).read();
    if n == 0 {
        return 0;
    }
    let mut r: u32 = 0;
    if n > 0 {
        let mut t = field::<GPtr<u16>, _>(cd, 0x2C).read();
        for _ in 0..n {
            for k in 0..3 {
                let v = t.wrapping_index(k).read() as u32;
                if v > r {
                    r = v;
                }
            }
            t = t.byte_add(8);
        }
    }
    r.wrapping_add(1)
}

/// `CAnimatedBuilding::CAnimatedBuilding()` (0x537A10, thiscall): `CBuilding::CBuilding`, then the
/// vtable 0x8639B0.
#[hook(addr = 0x537A10, conv = thiscall, name = "CAnimatedBuilding", expect = "56 8B F1 E8 E8 C3 EC FF", this = "CAnimatedBuilding")]
pub fn animated_building_ctor(this: GPtr<u8>) -> GPtr<u8> {
    building_ctor(this);
    field::<u32, _>(this, 0).write(0x8639B0);
    this
}

/// `CDummyObject::CDummyObject()` (0x59EA00, thiscall): `CDummy::CDummy`, then the vtable 0x866E78.
#[hook(addr = 0x59EA00, conv = thiscall, name = "CDummyObject::CDummyObject", expect = "56 8B F1 E8 38 3B F9 FF", this = "CDummyObject")]
pub fn dummy_object_ctor(this: GPtr<u8>) -> GPtr<u8> {
    dummy_ctor(this);
    field::<u32, _>(this, 0).write(0x866E78);
    this
}

/// `CPathFind::StoreNodeInfoCar` (0x44D2C0, thiscall, 17 stack words; proposed name): empty
/// (`ret 0x44`). Not hookable (3 bytes).
pub fn store_node_info_car(_this: GPtr<u8>, _args: [u32; 17]) {}

/// `CPathFind::StoreDetachedNodeInfoCar` (0x44D2F0; proposed name): empty (`ret 0x44`). Not hookable.
pub fn store_detached_node_info_car(_this: GPtr<u8>, _args: [u32; 17]) {}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[sa_core::abi::HookReg] = &[
    HOOK_TXD_POOL_CTOR,
    HOOK_TXD_POOL_NEW,
    HOOK_COL_POOL_CTOR,
    HOOK_VEHICLE_STRUCTURE_POOL_CTOR,
    HOOK_DUMMY_OPERATOR_NEW,
    HOOK_GET_NO_OF_SHADOW_VERTICES,
    HOOK_ANIMATED_BUILDING_CTOR,
    HOOK_DUMMY_OBJECT_CTOR,
];

#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    #[test]
    fn shadow_vertices() {
        let mut tris: [u16; 8] = [3, 9, 2, 0, 1, 0xFFFF, 0, 0];
        let mut cd = [0u8; 0x30];
        cd[0x20..0x24].copy_from_slice(&2i32.to_le_bytes());
        let p = GPtr::<u16>::from_raw(tris.as_mut_ptr());
        let cdp = GPtr::<u8>::from_raw(cd.as_mut_ptr());
        field::<GPtr<u16>, _>(cdp, 0x2C).write(p);
        assert_eq!(get_no_of_shadow_vertices(cdp), 0x10000);
        cd[0x20..0x24].copy_from_slice(&(-1i32).to_le_bytes());
        assert_eq!(get_no_of_shadow_vertices(cdp), 1);
    }
}
