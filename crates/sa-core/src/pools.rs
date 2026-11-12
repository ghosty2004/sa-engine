//! CORE ports: the remaining `CPool<T>` instantiations of gta_sa.exe 1.0 US and `CPools`
//! (see `docs/arch/core-port.md`).
//!
//! Every pool instantiation shares one algorithm (`crate::pool`); the instantiations differ only in
//! the slot size (`sizeof(S)`, the widest type stored), which the original folds into an `imul`/`lea`
//! /`shl` at the end of `New` and at the start of the constructor. Slot sizes below were read from
//! those instructions. Where gta-reversed's `Pool.h` / `Pools.cpp` differ, the original is followed:
//!
//! - constructors (0x550180.. and 0x552AF0, 0x405900): `objects = new[](size * slot)`,
//!   `map = new[](size)`, `owns = 1`, `size`, `first_free = -1`, then per slot `map[i] |= 0x80;
//!   map[i] &= 0x80` (two read-modify-writes). No NOMANSLAND fill of the storage (gr `DoFill`), `+0x11`
//!   is never written, the name argument is unused;
//! - `CPools::ShutDown` frees the arrays only when `size > 0` (then `objects = map = size =
//!   first_free = 0`; gr's Flush sets `-1`, clears the bools and runs always), keeps the
//!   `ms_p*Pool` pointers (gr nulls them) and `printf`s two lines;
//! - `operator delete` of pooled types: `map[idx] |= 0x80; if (idx < first_free) first_free = idx`,
//!   no null check and no DEADLAND fill.

use crate::mem::{GBool, GPtr, GameData, Global};
use crate::pool::{self, ByteMap, CPool, FREE, ObjectSlot, PedSlot, VehicleSlot};
use crate::{globals, hook, orig};

/// Storage of one pool slot of `N` bytes (the original's `sizeof(S)`).
#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct Slot<const N: usize>(pub [u8; N]);
// SAFETY: a byte array.
#[allow(unsafe_code)]
unsafe impl<const N: usize> GameData for Slot<N> {}

/// `CPtrNodeSingleLink` (8)
pub type PtrNodeSingleSlot = Slot<0x08>;
/// `CPtrNodeDoubleLink` (0xC)
pub type PtrNodeDoubleSlot = Slot<0x0C>;
/// `CEntryInfoNode` (0x14)
pub type EntryInfoNodeSlot = Slot<0x14>;
/// `CBuilding` (0x38)
pub type BuildingSlot = Slot<0x38>;
/// `CDummy` (0x38)
pub type DummySlot = Slot<0x38>;
/// `CColModel` (0x30)
pub type ColModelSlot = Slot<0x30>;
/// widest `CTask` (0x80)
pub type TaskSlot = Slot<0x80>;
/// widest `CEvent` (0x44)
pub type EventSlot = Slot<0x44>;
/// `CPointRoute` (0x64)
pub type PointRouteSlot = Slot<0x64>;
/// `CPatrolRoute` (0x1A4)
pub type PatrolRouteSlot = Slot<0x1A4>;
/// `CNodeRoute` (0x24)
pub type NodeRouteSlot = Slot<0x24>;
/// widest `CTaskAllocator` (0x20)
pub type TaskAllocatorSlot = Slot<0x20>;
/// `CPedIntelligence` (0x294)
pub type PedIntelligenceSlot = Slot<0x294>;
/// widest `CPedAttractor` (0xC4)
pub type PedAttractorSlot = Slot<0xC4>;
/// `IplDef` (0x34)
pub type IplDefSlot = Slot<0x34>;
/// `CQuadTreeNode` (0x28)
pub type QuadTreeNodeSlot = Slot<0x28>;

/// A pool pointer global.
pub type PoolPtr<T> = GPtr<CPool<T>>;

globals! {
    list GLOBALS;
    /// `CPools::ms_pPtrNodeSingleLinkPool`
    pub static PTR_NODE_SINGLE_POOL @ 0xB74484: PoolPtr<PtrNodeSingleSlot> = GPtr::NULL;
    /// `CPools::ms_pPtrNodeDoubleLinkPool`
    pub static PTR_NODE_DOUBLE_POOL @ 0xB74488: PoolPtr<PtrNodeDoubleSlot> = GPtr::NULL;
    /// `CPools::ms_pEntryInfoNodePool`
    pub static ENTRY_INFO_NODE_POOL @ 0xB7448C: PoolPtr<EntryInfoNodeSlot> = GPtr::NULL;
    /// `CPools::ms_pBuildingPool`
    pub static BUILDING_POOL @ 0xB74498: PoolPtr<BuildingSlot> = GPtr::NULL;
    /// `CPools::ms_pDummyPool`
    pub static DUMMY_POOL @ 0xB744A0: PoolPtr<DummySlot> = GPtr::NULL;
    /// `CPools::ms_pColModelPool`
    pub static COL_MODEL_POOL @ 0xB744A4: PoolPtr<ColModelSlot> = GPtr::NULL;
    /// `CPools::ms_pTaskPool`
    pub static TASK_POOL @ 0xB744A8: PoolPtr<TaskSlot> = GPtr::NULL;
    /// `CPools::ms_pEventPool`
    pub static EVENT_POOL @ 0xB744AC: PoolPtr<EventSlot> = GPtr::NULL;
    /// `CPools::ms_pPointRoutePool`
    pub static POINT_ROUTE_POOL @ 0xB744B0: PoolPtr<PointRouteSlot> = GPtr::NULL;
    /// `CPools::ms_pPatrolRoutePool`
    pub static PATROL_ROUTE_POOL @ 0xB744B4: PoolPtr<PatrolRouteSlot> = GPtr::NULL;
    /// `CPools::ms_pNodeRoutePool`
    pub static NODE_ROUTE_POOL @ 0xB744B8: PoolPtr<NodeRouteSlot> = GPtr::NULL;
    /// `CPools::ms_pTaskAllocatorPool`
    pub static TASK_ALLOCATOR_POOL @ 0xB744BC: PoolPtr<TaskAllocatorSlot> = GPtr::NULL;
    /// `CPools::ms_pPedIntelligencePool`
    pub static PED_INTELLIGENCE_POOL @ 0xB744C0: PoolPtr<PedIntelligenceSlot> = GPtr::NULL;
    /// `CPools::ms_pPedAttractorPool`
    pub static PED_ATTRACTOR_POOL @ 0xB744C4: PoolPtr<PedAttractorSlot> = GPtr::NULL;
    /// `CCarCtrl::NumRandomCars` (decremented by `LoadVehiclePool` through a protection chunk)
    pub static NUM_RANDOM_CARS @ 0x969094: i32 = 0;
    /// The two cells the protection chunk at 0x403AC1 subtracts to find `NumRandomCars`
    /// (`[0x156079C] - [0x59921E]`, = 0x969094 in the exe image).
    pub static PROT_CELL_A @ 0x156079C: u32 = 0xE760_5D7D;
    pub static PROT_CELL_B @ 0x59921E: u32 = 0xE6C9_CCE9;
    /// `CWorld::Players[0].m_pPed`
    pub static PLAYER0_PED @ 0xB7CD98: GPtr<u8> = GPtr::NULL;
    /// `CModelInfo::ms_modelInfoPtrs`
    pub static MODEL_INFO_PTRS @ 0xA9B0C8: [GPtr<u8>; 20000] = [GPtr::NULL; 20000];
    /// The `CMemoryHeap` constructed by `__init_g_c87bd0` (first dword `m_FirstBlock`).
    pub static MAIN_HEAP @ 0xC87BD0: GPtr<u8> = GPtr::NULL;
}

/// Calls into code that may still be original (CRT allocation, other subsystems, these pools).
pub mod call {
    use super::*;
    orig! {
        /// `operator new[]` (0x821195: `jmp operator new`)
        pub fn operator_new_array(size: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
        /// `operator new` (0x82119A, CRT)
        pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
        /// `operator delete[]` (0x8213AE: `jmp operator delete`)
        pub fn operator_delete_array(p: GPtr<u8>) = cdecl 0x8213AE => unported;
        /// `operator delete` (0x8214BD, CRT)
        pub fn operator_delete(p: GPtr<u8>) = cdecl 0x8214BD => unported;
        /// `printf(fmt)` (0x821982, CRT)
        pub fn printf0(fmt: GPtr<u8>) -> i32 = cdecl 0x821982 => unported;
        /// `printf(fmt, int, double, double, double)` (0x821982, CRT; the doubles are pushed as qwords)
        pub fn printf_i_ddd(fmt: GPtr<u8>, i: i32, x: f64, y: f64, z: f64) -> i32 = cdecl 0x821982 => unported;

        /// `CNodeRoutePool::New` (0x41B710)
        pub fn node_route_pool_new(p: PoolPtr<NodeRouteSlot>) -> GPtr<NodeRouteSlot> = thiscall 0x41B710 => super::node_route_pool_new;
        /// `CEntryInfoNodePool::GetAt` (0x536D10, really `New`)
        pub fn entry_info_node_pool_new(p: PoolPtr<EntryInfoNodeSlot>) -> GPtr<EntryInfoNodeSlot> = thiscall 0x536D10 => super::entry_info_node_pool_new;

        // pool constructors (thiscall, `this, size, name`)
        pub fn ctor_ptr_node_single(p: PoolPtr<PtrNodeSingleSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<PtrNodeSingleSlot> = thiscall 0x550180 => super::ctor_ptr_node_single;
        pub fn ctor_ptr_node_double(p: PoolPtr<PtrNodeDoubleSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<PtrNodeDoubleSlot> = thiscall 0x550250 => super::ctor_ptr_node_double;
        pub fn ctor_entry_info_node(p: PoolPtr<EntryInfoNodeSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<EntryInfoNodeSlot> = thiscall 0x550320 => super::ctor_entry_info_node;
        pub fn ctor_ped(p: PoolPtr<PedSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<PedSlot> = thiscall 0x5503F0 => super::ctor_ped;
        pub fn ctor_vehicle(p: PoolPtr<VehicleSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<VehicleSlot> = thiscall 0x5504C0 => super::ctor_vehicle;
        pub fn ctor_building(p: PoolPtr<BuildingSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<BuildingSlot> = thiscall 0x550570 => super::ctor_building;
        pub fn ctor_object(p: PoolPtr<ObjectSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<ObjectSlot> = thiscall 0x550640 => super::ctor_object;
        pub fn ctor_dummy(p: PoolPtr<DummySlot>, n: i32, name: GPtr<u8>) -> PoolPtr<DummySlot> = thiscall 0x5506F0 => super::ctor_dummy;
        pub fn ctor_col_model(p: PoolPtr<ColModelSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<ColModelSlot> = thiscall 0x5507C0 => super::ctor_col_model;
        pub fn ctor_task(p: PoolPtr<TaskSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<TaskSlot> = thiscall 0x550890 => super::ctor_task;
        pub fn ctor_event(p: PoolPtr<EventSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<EventSlot> = thiscall 0x550960 => super::ctor_event;
        pub fn ctor_point_route(p: PoolPtr<PointRouteSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<PointRouteSlot> = thiscall 0x550A30 => super::ctor_point_route;
        pub fn ctor_patrol_route(p: PoolPtr<PatrolRouteSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<PatrolRouteSlot> = thiscall 0x550B00 => super::ctor_patrol_route;
        pub fn ctor_node_route(p: PoolPtr<NodeRouteSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<NodeRouteSlot> = thiscall 0x550BD0 => super::ctor_node_route;
        pub fn ctor_task_allocator(p: PoolPtr<TaskAllocatorSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<TaskAllocatorSlot> = thiscall 0x550CA0 => super::ctor_task_allocator;
        pub fn ctor_ped_intelligence(p: PoolPtr<PedIntelligenceSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<PedIntelligenceSlot> = thiscall 0x550D70 => super::ctor_ped_intelligence;
        pub fn ctor_ped_attractor(p: PoolPtr<PedAttractorSlot>, n: i32, name: GPtr<u8>) -> PoolPtr<PedAttractorSlot> = thiscall 0x550E40 => super::ctor_ped_attractor;

        // CPools save/load
        pub fn save_ped_pool() -> u8 = cdecl 0x5D4B40 => super::save_ped_pool;
        pub fn save_object_pool() -> u8 = cdecl 0x5D4940 => super::save_object_pool;
        pub fn load_ped_pool() -> u8 = cdecl 0x5D2D70 => super::load_ped_pool;
        pub fn load_object_pool() -> u8 = cdecl 0x5D4A40 => super::load_object_pool;
        pub fn get_ped_ref(p: GPtr<PedSlot>) -> i32 = cdecl 0x54FF60 => crate::pool::get_ped_ref;
        pub fn get_vehicle_ref(p: GPtr<VehicleSlot>) -> i32 = cdecl 0x54FFC0 => crate::pool::get_vehicle_ref;
        pub fn get_object_ref(p: GPtr<ObjectSlot>) -> i32 = cdecl 0x550020 => crate::pool::get_object_ref;
        /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)`
        pub fn save_data(p: GPtr<u8>, size: u32) = cdecl 0x5D1270 => unported;
        /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)`
        pub fn load_data(p: GPtr<u8>, size: u32) = cdecl 0x5D1300 => unported;
        /// `CObjectSaveStructure::Construct(CObject*)`
        pub fn object_save_construct(this: GPtr<u8>, obj: GPtr<u8>) = thiscall 0x5D2160 => unported;
        /// `CObjectSaveStructure::Extract(CObject*)`
        pub fn object_save_extract(this: GPtr<u8>, obj: GPtr<u8>) = thiscall 0x5D2220 => unported;
        /// `CPopulation::ConvertToDummyObject(CObject*)`
        pub fn convert_to_dummy_object(obj: GPtr<u8>) = cdecl 0x614670 => unported;
        /// `CObject::operator new(size)`
        pub fn object_new(size: u32) -> GPtr<u8> = cdecl 0x5A1EE0 => unported;
        /// `CObject::operator new(size, poolRef)`
        pub fn object_new_at(size: u32, pool_ref: i32) -> GPtr<u8> = cdecl 0x5A1EF0 => unported;
        /// `CObject::CObject(int32 modelIndex, bool create)` (the bool pushed as a dword)
        pub fn object_ctor(this: GPtr<u8>, model: i32, create: u32) -> GPtr<u8> = thiscall 0x5A1D70 => unported;
        /// `CWorld::Add(CEntity*)`
        pub fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
        /// `CWorld::Remove(CEntity*)`
        pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
        /// `CProjectileInfo::RemoveIfThisIsAProjectile(CObject*)` (bool in AL)
        pub fn remove_if_projectile(obj: GPtr<u8>) -> u8 = cdecl 0x739A40 => unported;
        /// virtual slot 0: the scalar deleting destructor (`this, flags`)
        pub fn delete_entity(obj: GPtr<u8>, flags: i32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
        /// `CVehicle::Save` (vtable +0x100), `CVehicle::Load` (+0x104)
        pub fn vehicle_save(v: GPtr<u8>) -> u32 = thiscall vslot 0x100 => unported;
        pub fn vehicle_load(v: GPtr<u8>) -> u32 = thiscall vslot 0x104 => unported;
        /// `CPed::Save` (vtable +0x60), `CPed::Load` (+0x64)
        pub fn ped_save(p: GPtr<u8>) -> u32 = thiscall vslot 0x60 => unported;
        pub fn ped_load(p: GPtr<u8>) -> u32 = thiscall vslot 0x64 => unported;
        /// `CStreaming::RequestModel(int32, int32 flags)`
        pub fn request_model(model: i32, flags: i32) = cdecl 0x4087E0 => unported;
        /// `CStreaming::LoadAllRequestedModels(bool)`
        pub fn load_all_requested_models(only_priority: u32) = cdecl 0x40EA10 => unported;
        /// `CVehicle::operator new(size, poolRef)`
        pub fn vehicle_new_at(size: u32, pool_ref: i32) -> GPtr<u8> = cdecl 0x6E2D60 => unported;
        pub fn boat_ctor(this: GPtr<u8>, model: i32, created_by: i32) -> GPtr<u8> = thiscall 0x6F2940 => unported;
        pub fn train_ctor(this: GPtr<u8>, model: i32, created_by: i32) -> GPtr<u8> = thiscall 0x6F6030 => unported;
        pub fn bike_ctor(this: GPtr<u8>, model: i32, created_by: i32) -> GPtr<u8> = thiscall 0x6BF430 => unported;
        pub fn bmx_ctor(this: GPtr<u8>, model: i32, created_by: i32) -> GPtr<u8> = thiscall 0x6BF820 => unported;
        pub fn monster_truck_ctor(this: GPtr<u8>, model: i32, created_by: i32) -> GPtr<u8> = thiscall 0x6C8D60 => unported;
        pub fn quad_bike_ctor(this: GPtr<u8>, model: i32, created_by: i32) -> GPtr<u8> = thiscall 0x6CE370 => unported;
        pub fn heli_ctor(this: GPtr<u8>, model: i32, created_by: i32) -> GPtr<u8> = thiscall 0x6C4190 => unported;
        pub fn plane_ctor(this: GPtr<u8>, model: i32, created_by: i32) -> GPtr<u8> = thiscall 0x6C8E20 => unported;
        pub fn trailer_ctor(this: GPtr<u8>, model: i32, created_by: i32) -> GPtr<u8> = thiscall 0x6D03A0 => unported;
        pub fn automobile_ctor(this: GPtr<u8>, model: i32, created_by: i32, setup_suspension: i32) -> GPtr<u8> = thiscall 0x6B0A90 => unported;
        /// `CCarCtrl::UpdateCarCount(CVehicle*, bool remove)`
        pub fn update_car_count(v: GPtr<u8>, remove: u32) = cdecl 0x424000 => unported;
        /// `CPed::operator new(size, poolRef)`
        pub fn ped_new_at(size: u32, pool_ref: i32) -> GPtr<u8> = cdecl 0x5E4730 => unported;
        /// `CPlayerPed::CPlayerPed(int32 playerId, bool groupCreated)` (both pushed as dwords)
        pub fn player_ped_ctor(this: GPtr<u8>, id: i32, group: u32) -> GPtr<u8> = thiscall 0x60D5B0 => unported;
    }
}

// ---------------------------------------------------------------------------- generic parts ----

fn field<T, U>(pool: GPtr<CPool<T>>, off: usize) -> GPtr<U> {
    pool.byte_add(off as isize).cast()
}

/// Offset of a `CPool` header field (the original's on 32-bit targets, asserted in `crate::pool`).
macro_rules! off {
    ($t:ty, $f:ident) => {
        core::mem::offset_of!(CPool<$t>, $f)
    };
}

/// A field of a game object at byte offset `off`.
fn at<U>(obj: GPtr<u8>, off: isize) -> GPtr<U> {
    obj.byte_add(off).cast()
}

/// The pool constructors: `CPool<T>::CPool(int32 size, const char* name)` as the original compiles
/// it. `alloc` is the allocator the instantiation calls (`new[]` 0x821195, or `new` 0x82119A for the
/// quad-tree node pool).
pub fn pool_construct<T: 'static>(
    pool: GPtr<CPool<T>>,
    size: i32,
    alloc: fn(u32) -> GPtr<u8>,
) -> GPtr<CPool<T>> {
    let slot = core::mem::size_of::<T>() as u32;
    let objects = alloc((size as u32).wrapping_mul(slot));
    field::<T, GPtr<u8>>(pool, off!(T, objects)).write(objects);
    let map = alloc(size as u32);
    field::<T, GPtr<u8>>(pool, off!(T, byte_map)).write(map);
    field::<T, GBool>(pool, off!(T, owns_arrays)).write(GBool::TRUE);
    field::<T, i32>(pool, off!(T, size)).write(size);
    field::<T, i32>(pool, off!(T, first_free)).write(-1);
    // The original re-reads `m_byteMap` from the header for each access; nothing in between writes
    // it, so the map pointer read once is the same.
    let mut map = field::<T, GPtr<u8>>(pool, off!(T, byte_map)).read();
    init_map(&mut map, size);
    pool
}

/// The constructor's map loop: for `i` in `0..size` (signed), `map[i] |= 0x80`, then `map[i] &= 0x80`.
pub fn init_map(map: &mut impl ByteMap, size: i32) {
    let mut i = 0i32;
    while i < size {
        let b = map.get(i);
        map.set(i, b | FREE);
        let b = map.get(i);
        map.set(i, b & FREE);
        i += 1;
    }
}

/// The slot part of the inlined `CPool::Delete`: `map[idx] |= 0x80`, then `first_free` is read
/// (after the map write: a negative index can hit the header) and lowered to `idx` when `idx` is
/// smaller (signed). `first_free` is the accessor of the header field; returns the value written.
pub fn delete_at(
    map: &mut impl ByteMap,
    idx: i32,
    first_free: impl FnOnce(&dyn ByteMap) -> i32,
) -> Option<i32> {
    let b = map.get(idx);
    map.set(idx, b | FREE);
    let ff = first_free(&*map);
    (idx < ff).then_some(idx)
}

/// `CPool<T>::Delete(obj)` as the inlined copies in `operator delete` compile it: index by signed
/// division, `map[idx] |= 0x80`, and `first_free` written only when `idx` is lower (signed).
pub fn pool_delete_exact<T: 'static>(pool: GPtr<CPool<T>>, obj: GPtr<T>) {
    let p = pool.read();
    let idx = obj.index_from(p.objects);
    let mut map = p.byte_map;
    let ff_field = field::<T, i32>(pool, off!(T, first_free));
    if let Some(ff) = delete_at(&mut map, idx, |_| ff_field.read()) {
        ff_field.write(ff);
    }
}

/// One pool of `CPools::ShutDown` (0x5519F0): the arrays are freed and the header cleared only when
/// `size > 0` (and the arrays only when owned); the header is always deleted.
pub fn shutdown_pool<T: 'static>(pool: GPtr<CPool<T>>) {
    if pool.is_null() {
        return;
    }
    if field::<T, i32>(pool, off!(T, size)).read() > 0 {
        if field::<T, u8>(pool, off!(T, owns_arrays)).read() != 0 {
            call::operator_delete_array(field::<T, GPtr<u8>>(pool, off!(T, objects)).read());
            call::operator_delete_array(field::<T, GPtr<u8>>(pool, off!(T, byte_map)).read());
        }
        field::<T, GPtr<u8>>(pool, off!(T, objects)).write(GPtr::NULL);
        field::<T, GPtr<u8>>(pool, off!(T, byte_map)).write(GPtr::NULL);
        field::<T, u32>(pool, off!(T, size)).write(0);
        field::<T, u32>(pool, off!(T, first_free)).write(0);
    }
    call::operator_delete(pool.cast());
}

/// A pool constructor (`this, size, name`).
type PoolCtor<T> = fn(PoolPtr<T>, i32, GPtr<u8>) -> PoolPtr<T>;

/// One step of `CPools::Initialise` (0x550F10): `new(0x14)`, then the constructor when non-null.
fn init_pool<T: 'static>(g: &Global<PoolPtr<T>>, ctor: PoolCtor<T>, size: i32, name: u32) {
    let p: GPtr<CPool<T>> = call::operator_new(0x14).cast();
    let r = if p.is_null() {
        GPtr::NULL
    } else {
        ctor(p, size, GPtr::from_addr(name as usize))
    };
    g.set(r);
}

/// The backwards slot walk of `CheckPoolsEmpty` / `PruneAllReferencesInWorld`: calls `f` with every
/// used, non-null slot from `size - 1` down to 0. The counter is the original's `dec`/`jne` on an
/// unsigned register (a negative size walks on until it wraps).
pub fn for_used_slots_backwards<T: 'static>(pool: GPtr<CPool<T>>, mut f: impl FnMut(GPtr<u8>)) {
    let mut n = field::<T, u32>(pool, off!(T, size)).read();
    while n != 0 {
        let map = field::<T, GPtr<u8>>(pool, off!(T, byte_map)).read();
        let b = map.wrapping_index(n.wrapping_sub(1) as i32).read();
        n = n.wrapping_sub(1);
        if (b as i8) < 0 {
            continue;
        }
        let objects = field::<T, GPtr<u8>>(pool, off!(T, objects)).read();
        let obj = objects.byte_add((n.wrapping_mul(core::mem::size_of::<T>() as u32)) as i32 as isize);
        if obj.is_null() {
            continue;
        }
        f(obj);
    }
}

// --------------------------------------------------------------------- New instantiations ------

/// `CBuildingPool::New` (0x403FA0, slot 0x38).
#[hook(addr = 0x403FA0, conv = thiscall, name = "CBuildingPool::New", expect = "8B 51 08 56 32 C0", this = "CPool<int32>")]
pub fn building_pool_new(p: PoolPtr<BuildingSlot>) -> GPtr<BuildingSlot> {
    pool::pool_new(p)
}

/// `CColModelPool::New` (0x40FB80, slot 0x30: `lea eax,[eax+eax*2]; shl eax,4`).
#[hook(addr = 0x40FB80, conv = thiscall, name = "CColModelPool::New", expect = "8B 51 08 56 32 C0", this = "CPool<int32>")]
pub fn col_model_pool_new(p: PoolPtr<ColModelSlot>) -> GPtr<ColModelSlot> {
    pool::pool_new(p)
}

/// `CPointRoutePool::New` (0x41B5B0, slot 0x64).
#[hook(addr = 0x41B5B0, conv = thiscall, name = "CPointRoutePool::New", expect = "8B 51 08 56 32 C0", this = "CPool<int32>")]
pub fn point_route_pool_new(p: PoolPtr<PointRouteSlot>) -> GPtr<PointRouteSlot> {
    pool::pool_new(p)
}

/// `CPatrolRoutePool::New` (0x41B660, slot 0x1A4).
#[hook(addr = 0x41B660, conv = thiscall, name = "CPatrolRoutePool::New", expect = "8B 51 08 56 32 C0", this = "CPool<int32>")]
pub fn patrol_route_pool_new(p: PoolPtr<PatrolRouteSlot>) -> GPtr<PatrolRouteSlot> {
    pool::pool_new(p)
}

/// `CNodeRoutePool::New` (0x41B710, slot 0x24).
#[hook(addr = 0x41B710, conv = thiscall, name = "CNodeRoutePool::New", expect = "8B 51 08 56 32 C0", this = "CPool<int32>")]
pub fn node_route_pool_new(p: PoolPtr<NodeRouteSlot>) -> GPtr<NodeRouteSlot> {
    pool::pool_new(p)
}

/// `CEntryInfoNodePool::GetAt` (0x536D10): the catalogue name is gta-reversed's, but the bytes are a
/// `CPool::New` with slot 0x14 (the target of `CEntryInfoNode::operator new`).
#[hook(addr = 0x536D10, conv = thiscall, name = "CEntryInfoNodePool::GetAt", expect = "8B 51 08 56 32 C0", this = "CPool<int32>")]
pub fn entry_info_node_pool_new(p: PoolPtr<EntryInfoNodeSlot>) -> GPtr<EntryInfoNodeSlot> {
    pool::pool_new(p)
}

/// `CPtrNodeSingleLinkPool::New` (0x552240, slot 8).
#[hook(addr = 0x552240, conv = thiscall, name = "CPtrNodeSingleLinkPool::New", expect = "8B 51 08 56 32 C0", this = "CPool<int32>")]
pub fn ptr_node_single_pool_new(p: PoolPtr<PtrNodeSingleSlot>) -> GPtr<PtrNodeSingleSlot> {
    pool::pool_new(p)
}

/// `CPtrNodeDoubleLinkPool::New` (0x5522E0, slot 0xC).
#[hook(addr = 0x5522E0, conv = thiscall, name = "CPtrNodeDoubleLinkPool::New", expect = "8B 51 08 56 32 C0", this = "CPool<int32>")]
pub fn ptr_node_double_pool_new(p: PoolPtr<PtrNodeDoubleSlot>) -> GPtr<PtrNodeDoubleSlot> {
    pool::pool_new(p)
}

/// `CPedIntelligencePool::New` (0x605EC0, slot 0x294).
#[hook(addr = 0x605EC0, conv = thiscall, name = "CPedIntelligencePool::New", expect = "8B 51 08 56 32 C0", this = "CPool<int32>")]
pub fn ped_intelligence_pool_new(p: PoolPtr<PedIntelligenceSlot>) -> GPtr<PedIntelligenceSlot> {
    pool::pool_new(p)
}

// ----------------------------------------------------------------- operator new / delete -------

/// `CNodeRoute::operator new` (0x41B860): `mov ecx, ms_pNodeRoutePool; jmp CNodeRoutePool::New`.
/// Not hooked (`ported-unhookable`): the protection bytes after the next stub branch into its first
/// instruction (`jmp short` at 0x41B87A → 0x41B865), so the installer refuses the patch.
pub fn node_route_operator_new(_size: u32) -> GPtr<NodeRouteSlot> {
    call::node_route_pool_new(NODE_ROUTE_POOL.get())
}

/// `CNodeRoute::operator delete` (stub 0x41B870 → .HOODLUM 0x156E0C0): inlined `Delete`, slot 0x24
/// (magic 0x38E38E39, `sar 3`).
#[hook(addr = 0x41B870, conv = cdecl, name = "CNodeRoute::operator delete", expect = "E9 4B 28 15 01")]
pub fn node_route_operator_delete(obj: GPtr<NodeRouteSlot>) {
    pool_delete_exact(NODE_ROUTE_POOL.get(), obj)
}

/// `CEntryInfoNode::operator new` (0x536DC0): `mov ecx, ms_pEntryInfoNodePool; jmp 0x536D10`.
#[hook(addr = 0x536DC0, conv = cdecl, name = "CEntryInfoNode::operator new", expect = "8B 0D 8C 44 B7 00")]
pub fn entry_info_node_operator_new(_size: u32) -> GPtr<EntryInfoNodeSlot> {
    call::entry_info_node_pool_new(ENTRY_INFO_NODE_POOL.get())
}

/// `CEntryInfoNode::operator delete` (0x536DD0): inlined `Delete`, slot 0x14 (magic 0x66666667,
/// `sar 3`).
#[hook(addr = 0x536DD0, conv = cdecl, name = "CEntryInfoNode::operator delete", expect = "8B 4C 24 04 56")]
pub fn entry_info_node_operator_delete(obj: GPtr<EntryInfoNodeSlot>) {
    pool_delete_exact(ENTRY_INFO_NODE_POOL.get(), obj)
}

/// `CEntryInfoList::Flush` (0x536E10, thiscall on the list head): unlinks and frees every node.
/// Node layout: +0x0C previous, +0x10 next. Per node: `next` is read first; if the head is the node
/// the head becomes `next`; `prev->next = node->next`, `node->next->prev = node->prev` (each when
/// non-null, re-read from the node); then the inlined `CEntryInfoNode::operator delete`.
#[hook(addr = 0x536E10, conv = thiscall, name = "CEntryInfoList::Flush", expect = "57 8B 39 85 FF")]
pub fn entry_info_list_flush(list: GPtr<GPtr<u8>>) {
    let mut node = list.read();
    while !node.is_null() {
        let next = at::<GPtr<u8>>(node, 0x10).read();
        if list.read() == node {
            list.write(next);
        }
        let prev = at::<GPtr<u8>>(node, 0x0C).read();
        if !prev.is_null() {
            at::<GPtr<u8>>(prev, 0x10).write(at::<GPtr<u8>>(node, 0x10).read());
        }
        let n = at::<GPtr<u8>>(node, 0x10).read();
        if !n.is_null() {
            at::<GPtr<u8>>(n, 0x0C).write(at::<GPtr<u8>>(node, 0x0C).read());
        }
        pool_delete_exact(ENTRY_INFO_NODE_POOL.get(), node.cast());
        node = next;
    }
}

// ------------------------------------------------------------------------- constructors --------

/// `CPool<CPtrNodeSingleLink>::CPool(size, name)` (0x550180, slot 8).
#[hook(addr = 0x550180, conv = thiscall, name = "FUN_00550180", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_ptr_node_single(
    p: PoolPtr<PtrNodeSingleSlot>,
    size: i32,
    _name: GPtr<u8>,
) -> PoolPtr<PtrNodeSingleSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CPtrNodeDoubleLink>::CPool` (0x550250, slot 0xC).
#[hook(addr = 0x550250, conv = thiscall, name = "FUN_00550250", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_ptr_node_double(
    p: PoolPtr<PtrNodeDoubleSlot>,
    size: i32,
    _name: GPtr<u8>,
) -> PoolPtr<PtrNodeDoubleSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CEntryInfoNode>::CPool` (0x550320, slot 0x14).
#[hook(addr = 0x550320, conv = thiscall, name = "FUN_00550320", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_entry_info_node(
    p: PoolPtr<EntryInfoNodeSlot>,
    size: i32,
    _name: GPtr<u8>,
) -> PoolPtr<EntryInfoNodeSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CPed, CCopPed>::CPool` (0x5503F0, slot 0x7C4).
#[hook(addr = 0x5503F0, conv = thiscall, name = "FUN_005503f0", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_ped(p: PoolPtr<PedSlot>, size: i32, _name: GPtr<u8>) -> PoolPtr<PedSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CVehicle, CHeli>::CPool` (0x5504C0, slot 0xA18).
#[hook(addr = 0x5504C0, conv = thiscall, name = "FUN_005504c0", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_vehicle(p: PoolPtr<VehicleSlot>, size: i32, _name: GPtr<u8>) -> PoolPtr<VehicleSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CBuilding>::CPool` (0x550570, slot 0x38).
#[hook(addr = 0x550570, conv = thiscall, name = "FUN_00550570", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_building(p: PoolPtr<BuildingSlot>, size: i32, _name: GPtr<u8>) -> PoolPtr<BuildingSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CObject, CCutsceneObject>::CPool` (0x550640, slot 0x19C).
#[hook(addr = 0x550640, conv = thiscall, name = "FUN_00550640", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_object(p: PoolPtr<ObjectSlot>, size: i32, _name: GPtr<u8>) -> PoolPtr<ObjectSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CDummy>::CPool` (0x5506F0, slot 0x38).
#[hook(addr = 0x5506F0, conv = thiscall, name = "FUN_005506f0", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_dummy(p: PoolPtr<DummySlot>, size: i32, _name: GPtr<u8>) -> PoolPtr<DummySlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CColModel>::CPool` (0x5507C0, slot 0x30).
#[hook(addr = 0x5507C0, conv = thiscall, name = "FUN_005507c0", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_col_model(p: PoolPtr<ColModelSlot>, size: i32, _name: GPtr<u8>) -> PoolPtr<ColModelSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CTask>::CPool` (0x550890, slot 0x80).
#[hook(addr = 0x550890, conv = thiscall, name = "FUN_00550890", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_task(p: PoolPtr<TaskSlot>, size: i32, _name: GPtr<u8>) -> PoolPtr<TaskSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CEvent>::CPool` (0x550960, slot 0x44).
#[hook(addr = 0x550960, conv = thiscall, name = "FUN_00550960", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_event(p: PoolPtr<EventSlot>, size: i32, _name: GPtr<u8>) -> PoolPtr<EventSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CPointRoute>::CPool` (0x550A30, slot 0x64).
#[hook(addr = 0x550A30, conv = thiscall, name = "FUN_00550a30", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_point_route(p: PoolPtr<PointRouteSlot>, size: i32, _name: GPtr<u8>) -> PoolPtr<PointRouteSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CPatrolRoute>::CPool` (0x550B00, slot 0x1A4).
#[hook(addr = 0x550B00, conv = thiscall, name = "FUN_00550b00", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_patrol_route(
    p: PoolPtr<PatrolRouteSlot>,
    size: i32,
    _name: GPtr<u8>,
) -> PoolPtr<PatrolRouteSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CNodeRoute>::CPool` (0x550BD0, slot 0x24).
#[hook(addr = 0x550BD0, conv = thiscall, name = "FUN_00550bd0", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_node_route(p: PoolPtr<NodeRouteSlot>, size: i32, _name: GPtr<u8>) -> PoolPtr<NodeRouteSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CTaskAllocator>::CPool` (0x550CA0, slot 0x20).
#[hook(addr = 0x550CA0, conv = thiscall, name = "FUN_00550ca0", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_task_allocator(
    p: PoolPtr<TaskAllocatorSlot>,
    size: i32,
    _name: GPtr<u8>,
) -> PoolPtr<TaskAllocatorSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CPedIntelligence>::CPool` (0x550D70, slot 0x294).
#[hook(addr = 0x550D70, conv = thiscall, name = "FUN_00550d70", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_ped_intelligence(
    p: PoolPtr<PedIntelligenceSlot>,
    size: i32,
    _name: GPtr<u8>,
) -> PoolPtr<PedIntelligenceSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CPedAttractor>::CPool` (0x550E40, slot 0xC4).
#[hook(addr = 0x550E40, conv = thiscall, name = "FUN_00550e40", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_ped_attractor(
    p: PoolPtr<PedAttractorSlot>,
    size: i32,
    _name: GPtr<u8>,
) -> PoolPtr<PedAttractorSlot> {
    pool_construct(p, size, call::operator_new_array)
}

/// `CPool<CQuadTreeNode>::CPool` (0x552AF0, slot 0x28; calls `operator new` 0x82119A directly).
#[hook(addr = 0x552AF0, conv = thiscall, name = "FUN_00552af0", expect = "56 57 8B 7C 24 0C", this = "CPool<int32>")]
pub fn ctor_quad_tree_node(
    p: PoolPtr<QuadTreeNodeSlot>,
    size: i32,
    _name: GPtr<u8>,
) -> PoolPtr<QuadTreeNodeSlot> {
    pool_construct(p, size, call::operator_new)
}

/// `CIplDefPool::Constructor` (stub 0x405900 → .HOODLUM 0x1561B20, slot 0x34).
#[hook(addr = 0x405900, conv = thiscall, name = "CIplDefPool::Constructor", expect = "E9 1B C2 15 01", this = "CPool<int32>")]
pub fn ctor_ipl_def(p: PoolPtr<IplDefSlot>, size: i32, _name: GPtr<u8>) -> PoolPtr<IplDefSlot> {
    pool_construct(p, size, call::operator_new_array)
}

// ------------------------------------------------------------------------------- CPools ---------

/// `CPools::Initialise` (0x550F10): the 17 pools in this order, each `new(0x14)` + constructor (null
/// stored when `new` fails). Sizes and name strings (`.rdata`) as pushed by the original.
#[hook(addr = 0x550F10, conv = cdecl, name = "CPools::Initialise", expect = "6A FF 68 0B CC 83 00")]
pub fn initialise() {
    use pool::{OBJECT_POOL, PED_POOL, VEHICLE_POOL};
    init_pool(&PTR_NODE_SINGLE_POOL, call::ctor_ptr_node_single, 70000, 0x863D10);
    init_pool(&PTR_NODE_DOUBLE_POOL, call::ctor_ptr_node_double, 3200, 0x863D00);
    init_pool(&ENTRY_INFO_NODE_POOL, call::ctor_entry_info_node, 500, 0x863CF0);
    init_pool(&PED_POOL, call::ctor_ped, 140, 0x863CE8);
    init_pool(&VEHICLE_POOL, call::ctor_vehicle, 110, 0x863CDC);
    init_pool(&BUILDING_POOL, call::ctor_building, 13000, 0x863CD0);
    init_pool(&OBJECT_POOL, call::ctor_object, 350, 0x863CC8);
    init_pool(&DUMMY_POOL, call::ctor_dummy, 2500, 0x863CC0);
    init_pool(&COL_MODEL_POOL, call::ctor_col_model, 10150, 0x863CB4);
    init_pool(&TASK_POOL, call::ctor_task, 500, 0x863CAC);
    init_pool(&EVENT_POOL, call::ctor_event, 200, 0x863CA4);
    init_pool(&POINT_ROUTE_POOL, call::ctor_point_route, 64, 0x863C98);
    init_pool(&PATROL_ROUTE_POOL, call::ctor_patrol_route, 32, 0x863C8C);
    init_pool(&NODE_ROUTE_POOL, call::ctor_node_route, 64, 0x863C80);
    init_pool(&TASK_ALLOCATOR_POOL, call::ctor_task_allocator, 16, 0x863C70);
    init_pool(&PED_INTELLIGENCE_POOL, call::ctor_ped_intelligence, 140, 0x863C60);
    init_pool(&PED_ATTRACTOR_POOL, call::ctor_ped_attractor, 64, 0x863C50);
}

/// `CPools::ShutDown` (0x5519F0): `printf("Shutdown pool started\n")`, the 17 pools in
/// `Initialise` order through [`shutdown_pool`], `printf("Shutdown pool done\n")`. The `ms_p*Pool`
/// globals keep their (now dangling) values.
#[hook(addr = 0x5519F0, conv = cdecl, name = "CPools::ShutDown", expect = "53 56 57 68 78 3D 86 00")]
pub fn shut_down() {
    use pool::{OBJECT_POOL, PED_POOL, VEHICLE_POOL};
    call::printf0(GPtr::from_addr(0x863D78));
    shutdown_pool(PTR_NODE_SINGLE_POOL.get());
    shutdown_pool(PTR_NODE_DOUBLE_POOL.get());
    shutdown_pool(ENTRY_INFO_NODE_POOL.get());
    shutdown_pool(PED_POOL.get());
    shutdown_pool(VEHICLE_POOL.get());
    shutdown_pool(BUILDING_POOL.get());
    shutdown_pool(OBJECT_POOL.get());
    shutdown_pool(DUMMY_POOL.get());
    shutdown_pool(COL_MODEL_POOL.get());
    shutdown_pool(TASK_POOL.get());
    shutdown_pool(EVENT_POOL.get());
    shutdown_pool(POINT_ROUTE_POOL.get());
    shutdown_pool(PATROL_ROUTE_POOL.get());
    shutdown_pool(NODE_ROUTE_POOL.get());
    shutdown_pool(TASK_ALLOCATOR_POOL.get());
    shutdown_pool(PED_INTELLIGENCE_POOL.get());
    shutdown_pool(PED_ATTRACTOR_POOL.get());
    call::printf0(GPtr::from_addr(0x863D64));
}

/// `CPools::CheckPoolsEmpty` (0x551950): walks the object pool from the last slot down and prints
/// every object of type 5 (`+0x13C`): `printf("Offending object: MI:%d Coors:%f %f %f\n", model,
/// x, y, z)` with the position from the matrix (`+0x14`, at +0x30) or the placement (+4); then
/// `printf("pools have been cleared \n")`. gta-reversed iterates upwards and logs instead.
#[hook(addr = 0x551950, conv = cdecl, name = "CPools::CheckPoolsEmpty", expect = "53 8B 1D 9C 44 B7 00")]
pub fn check_pools_empty() {
    for_used_slots_backwards(pool::OBJECT_POOL.get(), |obj| {
        if at::<u8>(obj, 0x13C).read() != 5 {
            return;
        }
        let m = at::<GPtr<u8>>(obj, 0x14).read();
        let pos = if m.is_null() {
            obj.byte_add(4)
        } else {
            m.byte_add(0x30)
        };
        call::printf_i_ddd(
            GPtr::from_addr(0x863D3C),
            at::<i16>(obj, 0x22).read() as i32,
            at::<f32>(pos, 0).read() as f64,
            at::<f32>(pos, 4).read() as f64,
            at::<f32>(pos, 8).read() as f64,
        );
    });
    call::printf0(GPtr::from_addr(0x863D20));
}

/// `CPools::CheckBuildingAtomics` (0x550170): `or eax, -1; ret`. Four bytes followed by padding: too
/// short for the 5-byte entry jump, so not hooked (state `ported-unhookable`).
pub fn check_building_atomics() -> i32 {
    -1
}

/// `CPools::MakeSureSlotInObjectPoolIsEmpty(slot)` (0x550080). gta-reversed has `NOTSA_UNREACHABLE`
/// (its commented-out body inverts the projectile test); the original:
/// free slot → return; object type 3 (temporary) → `CWorld::Remove`, `delete`; otherwise, unless
/// `CProjectileInfo::RemoveIfThisIsAProjectile` returns true: a new `CObject(model, false)` takes a
/// 0x19C-byte copy of the old one (`rep movsd`, 0x67 dwords) between `CWorld::Remove(old)` and
/// `CWorld::Add(new)`; `old->m_pRwObject (+0x18) = 0`, `delete old`, `new->m_pReferences (+0x24) = 0`.
#[hook(addr = 0x550080, conv = cdecl, name = "CPools::MakeSureSlotInObjectPoolIsEmpty", expect = "64 A1 00 00 00 00")]
pub fn make_sure_slot_in_object_pool_is_empty(slot: i32) {
    let p = pool::OBJECT_POOL.get().read();
    if (p.byte_map.wrapping_index(slot).read() as i8) < 0 {
        return;
    }
    let obj: GPtr<u8> = p.objects.wrapping_index(slot).cast();
    if at::<u8>(obj, 0x13C).read() == 3 {
        call::world_remove(obj);
        call::delete_entity(obj, 1);
        return;
    }
    if call::remove_if_projectile(obj) != 0 {
        return;
    }
    let mem = call::object_new(0x17C);
    let new_obj = if mem.is_null() {
        GPtr::NULL
    } else {
        call::object_ctor(mem, at::<i16>(obj, 0x22).read() as i32, 0)
    };
    call::world_remove(obj);
    for k in 0..0x67 {
        at::<u32>(new_obj, 4 * k).write(at::<u32>(obj, 4 * k).read());
    }
    call::world_add(new_obj);
    at::<u32>(obj, 0x18).write(0);
    call::delete_entity(obj, 1);
    at::<u32>(new_obj, 0x24).write(0);
}

/// `CPools::Save` (0x5D0880): `SavePedPool() && SaveObjectPool()`, returning the callee's AL.
#[hook(addr = 0x5D0880, conv = cdecl, name = "CPools::Save", expect = "E8 BB 42 00 00")]
pub fn save() -> u8 {
    let r = call::save_ped_pool();
    if r == 0 {
        return r;
    }
    call::save_object_pool()
}

/// `CPools::Load` (0x5D0890): `LoadPedPool() && LoadObjectPool()`, returning the callee's AL.
#[hook(addr = 0x5D0890, conv = cdecl, name = "CPools::Load", expect = "E8 DB 24 00 00")]
pub fn load() -> u8 {
    let r = call::load_ped_pool();
    if r == 0 {
        return r;
    }
    call::load_object_pool()
}

fn local<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

/// The "is this vehicle saved" test of `SaveVehiclePool` (+0x4A4 == 2, no non-zero dword among the 8
/// at +0x464 — the scan stops at the first one —, and +0x460 == 0).
fn vehicle_is_saved(v: GPtr<u8>) -> bool {
    if at::<u8>(v, 0x4A4).read() != 2 {
        return false;
    }
    let mut any = false;
    let mut k = 0;
    while k < 8 && !any {
        if at::<u32>(v, 0x464 + 4 * k).read() != 0 {
            any = true;
        }
        k += 1;
    }
    at::<u32>(v, 0x460).read() == 0 && !any
}

/// `CPools::SaveVehiclePool` (0x5D4800; gta-reversed: unreversed). Counts the saved vehicles (used
/// slot, non-null, [`vehicle_is_saved`]; unsigned index loop), saves the count, then per saved
/// vehicle `GetVehicleRef`, the model index (`+0x22`, sign-extended), and the virtual `Save`
/// (vtable +0x100). The second pass re-reads the pool pointer after each save and uses the size
/// read by the first.
#[hook(addr = 0x5D4800, conv = cdecl, name = "CPools::SaveVehiclePool", expect = "83 EC 10 53 55")]
pub fn save_vehicle_pool() -> u8 {
    let pool_ptr = pool::VEHICLE_POOL.get();
    let size = pool_ptr.read().size as u32;
    let mut count: u32 = 0;
    let mut i = 0u32;
    while i < size {
        let p = pool_ptr.read();
        if (p.byte_map.wrapping_index(i as i32).read() as i8) >= 0 {
            let v: GPtr<u8> = p.objects.wrapping_index(i as i32).cast();
            if !v.is_null() && vehicle_is_saved(v) {
                count = count.wrapping_add(1);
            }
        }
        i += 1;
    }
    call::save_data(local(&mut count), 4);
    let mut i = 0u32;
    while i < size {
        let p = pool::VEHICLE_POOL.get().read();
        if (p.byte_map.wrapping_index(i as i32).read() as i8) >= 0 {
            let v: GPtr<u8> = p.objects.wrapping_index(i as i32).cast();
            if !v.is_null() && vehicle_is_saved(v) {
                let mut r = call::get_vehicle_ref(v.cast());
                let mut model = at::<i16>(v, 0x22).read() as i32;
                call::save_data(local(&mut r), 4);
                call::save_data(local(&mut model), 4);
                call::vehicle_save(v);
            }
        }
        i += 1;
    }
    1
}

/// `CPools::SaveObjectPool` (0x5D4940; gta-reversed: unreversed). Objects of type 2 (`+0x13C`):
/// count, then per object ref, model, a `CObjectSaveStructure` (0x30 bytes, `Construct`) preceded by
/// its size (0x30). The count pass skips free slots with `test 0x80`, the save pass with a sign test.
/// The structure is an uninitialised local (entry ESP -0x30) whose bytes `Construct` skips (+0x12..+0x17,
/// +0x19, +0x25..+0x27) reach the save file; SAVE-1 takes them from the footprint snapshot (ADR-001 §5b;
/// docs/arch/save-port.md), the iterations reusing the same local.
#[hook(addr = 0x5D4940, conv = cdecl, name = "CPools::SaveObjectPool", expect = "83 EC 40 8B 15 9C 44 B7 00",
       route = "split", hazards = "uninit-locals", recs = "save-load", footprint = 0x1000)]
pub fn save_object_pool() -> u8 {
    let p0 = pool::OBJECT_POOL.get().read();
    let size = p0.size as u32;
    let mut count: u32 = 0;
    let mut i = 0u32;
    while i < size {
        let p = pool::OBJECT_POOL.get().read();
        if p.byte_map.wrapping_index(i as i32).read() & FREE == 0 {
            let o: GPtr<u8> = p.objects.wrapping_index(i as i32).cast();
            if !o.is_null() && at::<u8>(o, 0x13C).read() == 2 {
                count = count.wrapping_add(1);
            }
        }
        i += 1;
    }
    call::save_data(local(&mut count), 4);
    let mut buf = [0u8; 0x30];
    for (k, b) in buf.iter_mut().enumerate() {
        *b = crate::abi::footprint_residue_u8(k as i32 - 0x30, 0x1000);
    }
    let mut i = 0u32;
    while i < size {
        let p = pool::OBJECT_POOL.get().read();
        if (p.byte_map.wrapping_index(i as i32).read() as i8) >= 0 {
            let o: GPtr<u8> = p.objects.wrapping_index(i as i32).cast();
            if !o.is_null() && at::<u8>(o, 0x13C).read() == 2 {
                let mut r = call::get_object_ref(o.cast());
                let mut model = at::<i16>(o, 0x22).read() as i32;
                call::save_data(local(&mut r), 4);
                call::save_data(local(&mut model), 4);
                let mut n: u32 = 0x30;
                call::object_save_construct(local(&mut buf), o);
                call::save_data(local(&mut n), 4);
                call::save_data(local(&mut buf), 0x30);
            }
        }
        i += 1;
    }
    1
}

/// `CPools::SavePedPool` (0x5D4B40; gta-reversed: unreversed). Peds with bit 0x100 of `+0x46C` clear
/// and `+0x598` (ped type) == 0: count, then ref, model, ped type and the virtual `Save` (+0x60).
#[hook(addr = 0x5D4B40, conv = cdecl, name = "CPools::SavePedPool", expect = "83 EC 10 8B 15 90 44 B7 00")]
pub fn save_ped_pool() -> u8 {
    let size = pool::PED_POOL.get().read().size as u32;
    let saved =
        |ped: GPtr<u8>| at::<u32>(ped, 0x46C).read() & 0x100 == 0 && at::<u32>(ped, 0x598).read() == 0;
    let mut count: u32 = 0;
    let mut i = 0u32;
    while i < size {
        let p = pool::PED_POOL.get().read();
        if p.byte_map.wrapping_index(i as i32).read() & FREE == 0 {
            let ped: GPtr<u8> = p.objects.wrapping_index(i as i32).cast();
            if !ped.is_null() && saved(ped) {
                count = count.wrapping_add(1);
            }
        }
        i += 1;
    }
    call::save_data(local(&mut count), 4);
    let mut i = 0u32;
    while i < size {
        let p = pool::PED_POOL.get().read();
        if (p.byte_map.wrapping_index(i as i32).read() as i8) >= 0 {
            let ped: GPtr<u8> = p.objects.wrapping_index(i as i32).cast();
            if !ped.is_null() && saved(ped) {
                let mut r = call::get_ped_ref(ped.cast());
                let mut model = at::<i16>(ped, 0x22).read() as i32;
                let mut ty = at::<u32>(ped, 0x598).read();
                call::save_data(local(&mut r), 4);
                call::save_data(local(&mut model), 4);
                call::save_data(local(&mut ty), 4);
                call::ped_save(ped);
            }
        }
        i += 1;
    }
    1
}

/// `CPools::LoadObjectPool` (0x5D4A40). Count (pre-set to 0), then per object: ref, model; an object
/// already at the ref's slot (`ref >> 8`, no bounds check, used and non-null) goes through
/// `CPopulation::ConvertToDummyObject`; `new(0x17C, ref) CObject(model, false)`; the save structure's
/// size dword is read and ignored, 0x30 bytes are read and `Extract`ed; `CWorld::Add`.
#[hook(addr = 0x5D4A40, conv = cdecl, name = "CPools::LoadObjectPool", expect = "83 EC 44 57 8D 44 24 04")]
pub fn load_object_pool() -> u8 {
    let mut count: i32 = 0;
    call::load_data(local(&mut count), 4);
    let mut i = 0i32;
    while i < count {
        let (mut r, mut model) = (0i32, 0i32);
        call::load_data(local(&mut r), 4);
        call::load_data(local(&mut model), 4);
        let p = pool::OBJECT_POOL.get().read();
        let idx = r >> 8;
        if (p.byte_map.wrapping_index(idx).read() as i8) >= 0 {
            let old: GPtr<u8> = p.objects.wrapping_index(idx).cast();
            if !old.is_null() {
                call::convert_to_dummy_object(old);
            }
        }
        let mem = call::object_new_at(0x17C, r);
        let obj = if mem.is_null() {
            GPtr::NULL
        } else {
            call::object_ctor(mem, model, 0)
        };
        let mut n: u32 = 0;
        let mut buf = [0u8; 0x30];
        call::load_data(local(&mut n), 4);
        call::load_data(local(&mut buf), 0x30);
        call::object_save_extract(local(&mut buf), obj);
        call::world_add(obj);
        i += 1;
    }
    1
}

/// `CPools::LoadPedPool` (0x5D2D70). Per ped: ref, model (unused), ped type. Type 0: `new(0x7A4, ref)
/// CPlayerPed(0, false)`, `+0x71A = 100` (weapon accuracy), `CWorld::Players[0].m_pPed = ped`. Any
/// other type reuses the previous iteration's ped (null on the first): gta-reversed asserts instead.
/// Then the virtual `Load` (+0x64) and `CWorld::Add`.
#[hook(addr = 0x5D2D70, conv = cdecl, name = "CPools::LoadPedPool", expect = "64 A1 00 00 00 00")]
pub fn load_ped_pool() -> u8 {
    let mut count: i32 = 0;
    call::load_data(local(&mut count), 4);
    let mut ped: GPtr<u8> = GPtr::NULL;
    let mut i = 0i32;
    while i < count {
        let (mut r, mut model, mut ty) = (0i32, 0i32, 0i32);
        call::load_data(local(&mut r), 4);
        call::load_data(local(&mut model), 4);
        call::load_data(local(&mut ty), 4);
        if ty == 0 {
            let mem = call::ped_new_at(0x7A4, r);
            let p = if mem.is_null() {
                GPtr::NULL
            } else {
                call::player_ped_ctor(mem, 0, 0)
            };
            at::<u8>(p, 0x71A).write(100);
            ped = p;
            PLAYER0_PED.set(p);
        }
        call::ped_load(ped);
        call::world_add(ped);
        i += 1;
    }
    1
}

/// `CPools::LoadVehiclePool` (0x5D2A20). Per vehicle: ref, model; `RequestModel(model, 8)`,
/// `LoadAllRequestedModels(false)`; the class by `ms_modelInfoPtrs[model]->m_nVehicleType (+0x3C)`
/// (jump table 0x5D2D40) with `CVehicle::operator new(size, ref)` and the constructor
/// `(model, RANDOM_VEHICLE = 1)` (CAutomobile: `(model, 1, true)`); bikes and BMXs get
/// `+0x614 |= 0x10` (also on a null result, as the original). Then the virtual `Load` (+0x104),
/// `--NumRandomCars` through the protection chunk at 0x403AC1 (address = `[0x156079C] -
/// [0x59921E]`), `CCarCtrl::UpdateCarCount(v, false)`, `CWorld::Add(v)`.
#[hook(addr = 0x5D2A20, conv = cdecl, name = "CPools::LoadVehiclePool", expect = "6A FF 68 9A D6 83 00")]
pub fn load_vehicle_pool() -> u8 {
    let mut count: i32 = 0;
    call::load_data(local(&mut count), 4);
    let mut i = 0i32;
    while i < count {
        let (mut r, mut model) = (0i32, 0i32);
        call::load_data(local(&mut r), 4);
        call::load_data(local(&mut model), 4);
        call::request_model(model, 8);
        call::load_all_requested_models(0);
        let mi = MODEL_INFO_PTRS
            .ptr()
            .cast::<GPtr<u8>>()
            .wrapping_index(model)
            .read();
        let ty = at::<u32>(mi, 0x3C).read().wrapping_sub(1);
        type Ctor = fn(GPtr<u8>, i32, i32) -> GPtr<u8>;
        let make = |size: u32, ctor: Ctor| {
            let m = call::vehicle_new_at(size, r);
            if m.is_null() {
                GPtr::NULL
            } else {
                ctor(m, model, 1)
            }
        };
        let v = match ty {
            0 => make(0x99C, call::monster_truck_ctor),
            1 => make(0x9BC, call::quad_bike_ctor),
            2 => make(0xA18, call::heli_ctor),
            3 => make(0xA04, call::plane_ctor),
            4 => make(0x7E8, call::boat_ctor),
            5 => make(0x6AC, call::train_ctor),
            8 | 9 => {
                let v = if ty == 8 {
                    make(0x814, call::bike_ctor)
                } else {
                    make(0x838, call::bmx_ctor)
                };
                let f = at::<u8>(v, 0x614);
                f.write(f.read() | 0x10);
                v
            }
            10 => make(0x9F4, call::trailer_ctor),
            _ => {
                let m = call::vehicle_new_at(0x988, r);
                if m.is_null() {
                    GPtr::NULL
                } else {
                    call::automobile_ctor(m, model, 1, 1)
                }
            }
        };
        call::vehicle_load(v);
        let cell = PROT_CELL_A.get().wrapping_sub(PROT_CELL_B.get());
        let nrc: GPtr<i32> = if cell == NUM_RANDOM_CARS.addr() {
            NUM_RANDOM_CARS.ptr()
        } else {
            GPtr::from_addr(cell as usize)
        };
        nrc.write(nrc.read().wrapping_sub(1));
        call::update_car_count(v, 0);
        call::world_add(v);
        i += 1;
    }
    1
}

/// `__init_g_c87bd0` (0x855690, static initialiser): `CMemoryHeap::CMemoryHeap()` (0x72E860, not
/// catalogued: `m_FirstBlock = nullptr`) on the global heap at 0xC87BD0. Runs during CRT start-up,
/// before sa-hook attaches, so its hook is never entered in practice.
#[hook(addr = 0x855690, conv = cdecl, name = "__init_g_c87bd0", expect = "B9 D0 7B C8 00")]
pub fn init_g_c87bd0() {
    call_heap::memory_heap_ctor(MAIN_HEAP.ptr().cast());
}

/// Phases B/C `CMemoryHeap::CMemoryHeap()`: `m_FirstBlock = nullptr`.
pub fn memory_heap_ctor_owned(this: GPtr<GPtr<u8>>) -> GPtr<GPtr<u8>> {
    this.write(GPtr::NULL);
    this
}

mod call_heap {
    use super::*;
    orig! {
        /// `CMemoryHeap::CMemoryHeap()` (0x72E860)
        pub fn memory_heap_ctor(this: GPtr<GPtr<u8>>) -> GPtr<GPtr<u8>> = thiscall 0x72E860 => super::memory_heap_ctor_owned;
    }
}

/// 0x44D0E0: a lone `ret` (unhookable).
pub fn ret_44d0e0() {}

/// 0x60F8C0: a lone `ret` (unhookable).
pub fn ret_60f8c0() {}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_BUILDING_POOL_NEW,
    HOOK_COL_MODEL_POOL_NEW,
    HOOK_POINT_ROUTE_POOL_NEW,
    HOOK_PATROL_ROUTE_POOL_NEW,
    HOOK_NODE_ROUTE_POOL_NEW,
    HOOK_ENTRY_INFO_NODE_POOL_NEW,
    HOOK_PTR_NODE_SINGLE_POOL_NEW,
    HOOK_PTR_NODE_DOUBLE_POOL_NEW,
    HOOK_PED_INTELLIGENCE_POOL_NEW,
    HOOK_NODE_ROUTE_OPERATOR_DELETE,
    HOOK_ENTRY_INFO_NODE_OPERATOR_NEW,
    HOOK_ENTRY_INFO_NODE_OPERATOR_DELETE,
    HOOK_ENTRY_INFO_LIST_FLUSH,
    HOOK_CTOR_PTR_NODE_SINGLE,
    HOOK_CTOR_PTR_NODE_DOUBLE,
    HOOK_CTOR_ENTRY_INFO_NODE,
    HOOK_CTOR_PED,
    HOOK_CTOR_VEHICLE,
    HOOK_CTOR_BUILDING,
    HOOK_CTOR_OBJECT,
    HOOK_CTOR_DUMMY,
    HOOK_CTOR_COL_MODEL,
    HOOK_CTOR_TASK,
    HOOK_CTOR_EVENT,
    HOOK_CTOR_POINT_ROUTE,
    HOOK_CTOR_PATROL_ROUTE,
    HOOK_CTOR_NODE_ROUTE,
    HOOK_CTOR_TASK_ALLOCATOR,
    HOOK_CTOR_PED_INTELLIGENCE,
    HOOK_CTOR_PED_ATTRACTOR,
    HOOK_CTOR_QUAD_TREE_NODE,
    HOOK_CTOR_IPL_DEF,
    HOOK_INITIALISE,
    HOOK_SHUT_DOWN,
    HOOK_CHECK_POOLS_EMPTY,
    HOOK_MAKE_SURE_SLOT_IN_OBJECT_POOL_IS_EMPTY,
    HOOK_SAVE,
    HOOK_LOAD,
    HOOK_SAVE_VEHICLE_POOL,
    HOOK_SAVE_OBJECT_POOL,
    HOOK_SAVE_PED_POOL,
    HOOK_LOAD_OBJECT_POOL,
    HOOK_LOAD_PED_POOL,
    HOOK_LOAD_VEHICLE_POOL,
    HOOK_INIT_G_C87BD0,
];

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;
    use std::vec;
    use std::vec::Vec;

    fn fake_alloc(n: u32) -> GPtr<u8> {
        let v: &'static mut [u8] = std::boxed::Box::leak(vec![0x5Au8; n as usize + 1].into_boxed_slice());
        GPtr::from_raw(v.as_mut_ptr())
    }

    #[test]
    fn constructor_marks_every_slot_free_with_counter_zero() {
        let mut hdr = CPool::<NodeRouteSlot> {
            objects: GPtr::NULL,
            byte_map: GPtr::NULL,
            size: 0,
            first_free: 7,
            owns_arrays: GBool::FALSE,
            locked: GBool(0x42),
            _pad: [0; 2],
        };
        let p = GPtr::from_raw(&raw mut hdr);
        assert_eq!(pool_construct(p, 5, fake_alloc), p);
        assert_eq!(
            (hdr.size, hdr.first_free, hdr.owns_arrays, hdr.locked),
            (5, -1, GBool::TRUE, GBool(0x42))
        );
        for i in 0..5 {
            assert_eq!(hdr.byte_map.wrapping_index(i).read(), 0x80);
        }
        // the byte after the map is untouched
        assert_eq!(hdr.byte_map.wrapping_index(5).read(), 0x5A);
        // a new slot after construction: slot 0, counter 1
        let s = pool::pool_new(p);
        assert_eq!(s, hdr.objects);
        assert_eq!(hdr.byte_map.read(), 0x01);
    }

    #[test]
    fn delete_writes_first_free_only_when_lower() {
        let mut storage: Vec<EntryInfoNodeSlot> = vec![Slot([0; 0x14]); 4];
        let mut map = vec![0x01u8, 0x02, 0x03, 0x04];
        let mut hdr = CPool::<EntryInfoNodeSlot> {
            objects: GPtr::from_raw(storage.as_mut_ptr()),
            byte_map: GPtr::from_raw(map.as_mut_ptr()),
            size: 4,
            first_free: 2,
            owns_arrays: GBool::TRUE,
            locked: GBool::FALSE,
            _pad: [0; 2],
        };
        let p = GPtr::from_raw(&raw mut hdr);
        pool_delete_exact(p, GPtr::from_raw(&raw mut storage[3]));
        assert_eq!((map[3], hdr.first_free), (0x84, 2));
        pool_delete_exact(p, GPtr::from_raw(&raw mut storage[1]));
        assert_eq!((map[1], hdr.first_free), (0x82, 1));
    }

    #[test]
    fn backwards_walk_skips_free_slots() {
        let mut storage: Vec<Slot<8>> = vec![Slot([0; 8]); 4];
        let mut map = vec![0x01u8, 0x82, 0x03, 0xFF];
        let mut hdr = CPool::<Slot<8>> {
            objects: GPtr::from_raw(storage.as_mut_ptr()),
            byte_map: GPtr::from_raw(map.as_mut_ptr()),
            size: 4,
            first_free: 0,
            owns_arrays: GBool::TRUE,
            locked: GBool::FALSE,
            _pad: [0; 2],
        };
        let base = storage.as_ptr() as usize;
        let mut seen = Vec::new();
        for_used_slots_backwards(GPtr::from_raw(&raw mut hdr), |o| seen.push((o.addr() - base) / 8));
        assert_eq!(seen, vec![2, 0]);
        let _ = &mut map;
    }
}
