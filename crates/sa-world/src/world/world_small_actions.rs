//! WRITER-WORLD-2 (batch `world6-3`): small `CWorld` actions: the remote-bomb detonator, setting the
//! world on fire around a point, repositioning the dynamic dummies, removing an entity instead of
//! processing it, the building overlap test over all sectors and the anim-viewer update. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;

use super::common::{g, rd, wr};

use crate::bindings::entity_update_anim as update_anim;
use crate::bindings::population_remove_ped as remove_ped;
use crate::bindings::remove_from_moving_list;
use crate::bindings::simple_transform_update_rw_matrix as transform_update_rw;
use sa_core::bindings::entity_register_reference as register_reference;
use crate::bindings::entity_update_rw_frame as update_rw_frame;
use crate::bindings::find_player_ped;
use sa_core::bindings::matrix_update_rw_matrix as matrix_update_rw;
use crate::bindings::get_num_fires_in_range;
use crate::bindings::fire_manager_start_fire as start_fire;
use crate::bindings::world_test_sphere_against_world as test_sphere_against_world;
orig! {
    /// `CProjectileInfo::RemoveDetonatorProjectiles()` (0x738860)
    fn remove_detonator_projectiles() = cdecl 0x738860 => unported;
    /// `CWorld::RepositionOneObject(CEntity*)` (0x569850)
    fn reposition_one_object(e: GPtr<u8>) = cdecl 0x569850 => unported;
    /// `CEntity::Remove()` (vtable +0xC)
    fn v_remove(e: GPtr<u8>) = thiscall vslot 0xC => unported;
    /// the deleting destructor (vtable +0)
    fn v_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CWorld::TestForBuildingsOnTopOfEachOther(CPtrList&)` (0x563950)
    fn test_list_for_buildings_on_top(list: GPtr<u8>) = cdecl 0x563950 => unported;
}

/// `CWorld::RemoveEntityInsteadOfProcessingIt(CEntity* e)` (0x563A10): a ped that is the player is
/// only removed from the world (and from the moving list when its type is 2..4); any other ped goes
/// through `CPopulation::RemovePed`; any other entity is removed (moving list too for types 2..4)
/// and deleted.
#[hook(addr = 0x563A10, conv = cdecl, name = "CWorld::RemoveEntityInsteadOfProcessingIt",
       expect = "56 8B 74 24 08",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn remove_entity_instead_of_processing_it(e: GPtr<u8>) {
    let moving = |e: GPtr<u8>| {
        let t = rd::<u8>(e, 0x36) & 7;
        t > 1 && t < 5
    };
    if rd::<u8>(e, 0x36) & 7 != 3 {
        v_remove(e);
        if moving(e) {
            remove_from_moving_list(e);
        }
        v_delete(e, 1);
    } else if find_player_ped(-1) == e {
        v_remove(e);
        if moving(e) {
            remove_from_moving_list(e);
        }
    } else {
        remove_ped(e);
    }
}

/// `CWorld::TestForBuildingsOnTopOfEachOther()` (0x5664A0): the overlap test on the building and
/// the dummy list of every sector (120 x 120, 0xB7D0B8 + (x * 120 + y) * 8, the indices clamped to
/// 0..119).
#[hook(addr = 0x5664A0, conv = cdecl, name = "CWorld::TestForBuildingsOnTopOfEachOther-Void",
       expect = "53 55 56 57 33 ED",
       unverified = "world6-3", route = "L2", hazards = "none")]
pub fn test_for_buildings_on_top_of_each_other() {
    for sx in 0..0x78usize {
        for sy in 0..0x78usize {
            let base = 0xB7_D0B8 + (sx.min(0x77) * 0x78 + sy.min(0x77)) * 8;
            test_list_for_buildings_on_top(GPtr::from_addr(base));
            test_list_for_buildings_on_top(GPtr::from_addr(base + 4));
        }
    }
}

/// `CWorld::ProcessForAnimViewer()` (0x5633D0): every entity of the moving list (0xB9ACC8) without
/// +0x1C bit 11: `UpdateAnim`, then (with an RW object) its frame's modelling matrix refreshed from
/// the entity's matrix or placement, and `UpdateRwFrame`.
#[hook(addr = 0x5633D0, conv = cdecl, name = "CWorld::ProcessForAnimViewer",
       expect = "57 8B 3D C8 AC B9 00",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn process_for_anim_viewer() {
    let mut node = g::<GPtr<u8>>(0xB9_ACC8).read();
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        if rd::<u32>(e, 0x1C) & 0x800 != 0 {
            continue;
        }
        update_anim(e);
        let rw = rd::<GPtr<u8>>(e, 0x18);
        if !rw.is_null() {
            let m = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
            let mat = rd::<GPtr<u8>>(e, 0x14);
            if mat.is_null() {
                transform_update_rw(e.byte_add(4), m);
            } else {
                matrix_update_rw(mat, m);
            }
        }
        update_rw_frame(e);
    }
}

/// `CPools::ms_pVehiclePool` (0xB74494), `CPools::ms_pDummyPool` (0xB744A0), `gFireManager` (0xB71F80).
const VEHICLE_POOL: usize = 0xB7_4494;
const DUMMY_POOL: usize = 0xB7_44A0;
const FIRE_MANAGER: usize = 0xB7_1F80;

/// Every used slot of a pool (highest first): `+0` objects, `+4` flags (negative = free), `+8` size.
fn each_slot(pool_ptr: usize, stride: isize, mut f: impl FnMut(GPtr<u8>)) {
    let pool = g::<GPtr<u8>>(pool_ptr).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        let flag = rd::<i8>(rd::<GPtr<u8>>(pool, 4), (i - 1) as isize);
        i -= 1;
        if flag < 0 {
            continue;
        }
        let e = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * stride);
        if !e.is_null() {
            f(e);
        }
    }
}

/// `CWorld::UseDetonator(CEntity* creator)` (0x5660B0): every vehicle carrying a remote bomb (+0x4A8
/// low 3 bits == 3) planted by `creator` (+0x4CC) gets its bomb armed: type cleared, timer 500 (+0x4DE),
/// detonated-by = creator (+0x4E0, a registered reference). Then tail-calls
/// `CProjectileInfo::RemoveDetonatorProjectiles`.
#[hook(addr = 0x5660B0, conv = cdecl, name = "CWorld::UseDetonator",
       expect = "55 8B 2D 94 44 B7 00",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn use_detonator(creator: GPtr<u8>) {
    each_slot(VEHICLE_POOL, 0xA18, |veh| {
        let b = rd::<u8>(veh, 0x4A8);
        if b & 7 == 3 && rd::<GPtr<u8>>(veh, 0x4CC) == creator {
            wr::<u8>(veh, 0x4A8, b & 0xF8);
            wr::<u16>(veh, 0x4DE, 500);
            wr::<GPtr<u8>>(veh, 0x4E0, creator);
            if !creator.is_null() {
                register_reference(creator, veh.byte_add(0x4E0));
            }
        }
    });
    remove_detonator_projectiles();
}

/// `CWorld::SetWorldOnFire(float x, float y, float z, float radius, CEntity*)` (0x56B910): when a
/// building or object touches the sphere and no fire burns within 2 of the point, a fire starts
/// there (size 0.8, 7000 ms, 100 generations, no creator: the last argument is unused).
#[hook(addr = 0x56B910, conv = cdecl, name = "CWorld::SetWorldOnFire",
       expect = "83 EC 0C 8B 44 24 10",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn set_world_on_fire(px: f32, py: f32, pz: f32, radius: f32, _creator: GPtr<u8>) {
    let c = CVector::new(px, py, pz);
    if test_sphere_against_world(c, radius, 0, 1, 0, 0, 1, 0, 0).is_null() {
        return;
    }
    let mut pos = [px, py, pz];
    if get_num_fires_in_range(g(FIRE_MANAGER), GPtr::from_raw(&mut pos as *mut [f32; 3]).cast(), 2.0) != 0 {
        return;
    }
    start_fire(g(FIRE_MANAGER), c, f32::from_bits(0x3F4C_CCCD), 1, 0, 0x1B58, 0x64, 1);
}

/// `CWorld::RepositionCertainDynamicObjects()` (0x56B9C0): `RepositionOneObject` for every dummy of
/// the dummy pool (highest slot first).
#[hook(addr = 0x56B9C0, conv = cdecl, name = "CWorld::RepositionCertainDynamicObjects",
       expect = "53 8B 1D A0 44 B7 00",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn reposition_certain_dynamic_objects() {
    each_slot(DUMMY_POOL, 0x38, reposition_one_object);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_USE_DETONATOR,
    HOOK_SET_WORLD_ON_FIRE,
    HOOK_REPOSITION_CERTAIN_DYNAMIC_OBJECTS,
    HOOK_REMOVE_ENTITY_INSTEAD_OF_PROCESSING_IT,
    HOOK_TEST_FOR_BUILDINGS_ON_TOP_OF_EACH_OTHER,
    HOOK_PROCESS_FOR_ANIM_VIEWER,
];
