//! WRITER-WORLD-2 (batch `world6-3`): `CGarage::IsGarageEmpty` and
//! `CGarage::EntityHasASphereWayOutsideGarage`, `CGarage::RemoveCarsBlockingDoorNotInside`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;

use super::common::{rd, x};

orig! {
    /// `CWorld::FindObjectsIntersectingCube(const CVector& min, const CVector& max, int16* count,
    /// int16 maxCount, CEntity** out, bool x5)` (0x568DD0)
    fn find_objects_intersecting_cube(mn: GPtr<u8>, mx: GPtr<u8>, count: GPtr<u8>, max: u32, out: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32) = cdecl 0x568DD0 => unported;
    /// `CGarage::IsEntityTouching3D(CEntity*)` (0x448EE0)
    fn is_entity_touching_3d(gr: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x448EE0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    fn update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CGarage::IsPointInsideGarage(CVector, float margin)` (0x4487D0)
    fn is_point_inside(gr: GPtr<u8>, p: CVector, margin: f32) -> u8 = thiscall 0x4487D0 => unported;
    /// `CGarage::IsPointInsideGarage(CVector)` (0x448740)
    fn is_point_inside_no_margin(gr: GPtr<u8>, p: CVector) -> u8 = thiscall 0x448740 => unported;
    /// `CVehicle::CanBeDeleted()` (0x6D1180)
    fn can_be_deleted(v: GPtr<u8>) -> u8 = thiscall 0x6D1180 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// the deleting destructor (vtable +0)
    fn v_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
}

/// `CGarage::RemoveCarsBlockingDoorNotInside()` (0x449690): the first vehicle of the pool (highest
/// slot first) that touches the garage in 3D with its position outside it, is not a mission
/// vehicle (+0x428 bit 3) and can be deleted, is removed from the world and deleted.
#[hook(addr = 0x449690, conv = thiscall, name = "CGarage::RemoveCarsBlockingDoorNotInside", this = "CGarage",
       expect = "51 53 55 8B 2D 94 44 B7 00",
       unverified = "world6-3", route = "L2", hazards = "heap-writes")]
pub fn remove_cars_blocking_door_not_inside(this: GPtr<u8>) {
    let pool = GPtr::<GPtr<u8>>::from_addr(0xB7_4494).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        let flag = rd::<i8>(rd::<GPtr<u8>>(pool, 4), (i - 1) as isize);
        i -= 1;
        if flag < 0 {
            continue;
        }
        let veh = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0xA18);
        if veh.is_null() || is_entity_touching_3d(this, veh) == 0 {
            continue;
        }
        let m = rd::<GPtr<u8>>(veh, 0x14);
        let pos = if m.is_null() { veh.byte_add(4) } else { m.byte_add(0x30) };
        let p = CVector::new(rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8));
        if is_point_inside_no_margin(this, p) == 0 && rd::<u8>(veh, 0x428) & 8 == 0 && can_be_deleted(veh) != 0 {
            world_remove(veh);
            v_delete(veh, 1);
            return;
        }
    }
}

/// `CGarage::EntityHasASphereWayOutsideGarage(CEntity* e, float margin)` (0x449050): true when some
/// collision sphere centre of `e` (through its matrix, allocated from the placement when missing)
/// lies outside the garage by more than `margin + radius`.
#[hook(addr = 0x449050, conv = thiscall, name = "CGarage::EntityHasASphereWayOutsideGarage", this = "CGarage",
       expect = "83 EC 10 53 8B 5C 24 18",
       unverified = "world6-3", route = "L2", hazards = "heap-writes",
       note = "may allocate the entity's matrix")]
pub fn entity_has_a_sphere_way_outside_garage(this: GPtr<u8>, e: GPtr<u8>, margin: f32) -> u8 {
    let data = rd::<GPtr<u8>>(get_col_model(e), 0x2C);
    let mut i = 0;
    while i < i32::from(rd::<i16>(data, 0)) {
        if rd::<GPtr<u8>>(e, 0x14).is_null() {
            allocate_matrix(e);
            update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
        }
        let sphere = rd::<GPtr<u8>>(data, 8).byte_add(i as isize * 0x14);
        let mut out = [0f32; 3];
        let r = matrix_mul_vec(GPtr::from_raw(&mut out as *mut [f32; 3]).cast(), rd::<GPtr<u8>>(e, 0x14), sphere);
        let m = x(margin).fadd_m(rd::<f32>(rd::<GPtr<u8>>(data, 8).byte_add(i as isize * 0x14), 0xC)).to_f32();
        let p = CVector::new(rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8));
        if is_point_inside(this, p, m) == 0 {
            return 1;
        }
        i += 1;
    }
    0
}

/// `CGarage::IsGarageEmpty()` (0x44A9C0): the vehicles and peds (at most 16) whose boxes meet the
/// garage cube (+0x28 / +0x30 / +0x08 to +0x2C / +0x34 / +0x1C); false as soon as one touches the
/// garage in 3D, else true.
#[hook(addr = 0x44A9C0, conv = thiscall, name = "CGarage::IsGarageEmpty", this = "CGarage",
       expect = "83 EC 5C 56 57",
       unverified = "world6-3", route = "L2", hazards = "uninit-locals",
       note = "the found count is an uninitialised stack word until FindObjectsIntersectingCube writes it (0 here)")]
pub fn is_garage_empty(this: GPtr<u8>) -> u8 {
    let mut mn = [
        rd::<u32>(this, 0x28),
        x(rd::<f32>(this, 0x30)).to_f32().to_bits(),
        x(rd::<f32>(this, 0x08)).to_f32().to_bits(),
    ];
    let mut mx = [
        rd::<u32>(this, 0x2C),
        x(rd::<f32>(this, 0x34)).to_f32().to_bits(),
        x(rd::<f32>(this, 0x1C)).to_f32().to_bits(),
    ];
    let mut count: u32 = 0;
    let mut out = [GPtr::<u8>::NULL; 16];
    let p = |v: &mut [u32; 3]| GPtr::from_raw(v as *mut [u32; 3]).cast::<u8>();
    find_objects_intersecting_cube(p(&mut mn), p(&mut mx), GPtr::from_raw(&mut count as *mut u32).cast(), 0x10,
                                   GPtr::from_raw(&mut out as *mut [GPtr<u8>; 16]).cast(), 0, 1, 1, 0, 0);
    let n = count as u16 as i16;
    let mut i: i16 = 0;
    while i < n {
        if is_entity_touching_3d(this, out[i as usize]) != 0 {
            return 0;
        }
        i += 1;
    }
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_IS_GARAGE_EMPTY, HOOK_ENTITY_HAS_A_SPHERE_WAY_OUTSIDE_GARAGE, HOOK_REMOVE_CARS_BLOCKING_DOOR_NOT_INSIDE];
