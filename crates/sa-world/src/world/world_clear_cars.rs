//! WRITER-WORLD-2 (batch `world6-5`): `CWorld::ClearCarsFromArea`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{rd, wr, x};

orig! {
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CVehicle::CanBeDeleted()` (0x6D1180)
    fn can_be_deleted(v: GPtr<u8>) -> u8 = thiscall 0x6D1180 => unported;
    /// `CPopulation::RemovePed(CPed*)` (0x610F20)
    fn remove_ped(p: GPtr<u8>) = cdecl 0x610F20 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CVehicle::RemovePassenger(CPed*)` (0x6D1610)
    fn remove_passenger(v: GPtr<u8>, p: GPtr<u8>) = thiscall 0x6D1610 => unported;
    /// `CCarCtrl::IsThisVehicleInteresting(CVehicle*)` (0x423EA0)
    fn is_this_vehicle_interesting(v: GPtr<u8>) -> u8 = cdecl 0x423EA0 => unported;
    /// `CGarages::StoreCarInNearestImpoundingGarage(CVehicle*)` (0x44A3C0)
    fn store_car_in_nearest_impounding_garage(v: GPtr<u8>) = cdecl 0x44A3C0 => unported;
    /// `CCarCtrl::RemoveFromInterestingVehicleList(CVehicle*)` (0x423ED0)
    fn remove_from_interesting_vehicle_list(v: GPtr<u8>) = cdecl 0x423ED0 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// the deleting destructor (vtable +0)
    fn v_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
}

/// `min <= v <= max` (`test ah, 1` against the minimum, `test ah, 0x41; jp` against the maximum).
fn in_range(v: f32, mn: f32, mx: f32) -> bool {
    !x(v).fcom(x(mn)).test_any(0x01) && !x(v).fcom(x(mx)).test_parity(0x41)
}

/// `CWorld::ClearCarsFromArea(float minX, float minY, float minZ, float maxX, float maxY, float
/// maxZ)` (0x566610): every vehicle of the pool (lowest slot first) whose position lies in the box,
/// except the player's vehicle when its +0x590 is 5, mission vehicles (+0x428 bit 3) and vehicles
/// that cannot be deleted: its driver and passengers are removed (`CPopulation::RemovePed`, the
/// driver reference cleaned up and cleared), an interesting vehicle is stored in the nearest
/// impounding garage, it leaves the interesting list and the world and is deleted.
#[hook(addr = 0x566610, conv = cdecl, name = "CWorld::ClearCarsFromArea",
       expect = "83 EC 1C 53 55 57",
       unverified = "world6-5", route = "L2", hazards = "heap-writes")]
pub fn clear_cars_from_area(x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32) {
    let pool = GPtr::<GPtr<u8>>::from_addr(0xB7_4494).read();
    let size = rd::<i32>(pool, 8);
    let mut i = 0;
    while i < size {
        let flag = rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize);
        let veh = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0xA18);
        i += 1;
        if flag < 0 || veh.is_null() {
            continue;
        }
        if veh == rd::<GPtr<u8>>(find_player_ped(-1), 0x584) && rd::<u32>(veh, 0x590) == 5 {
            continue;
        }
        let m = rd::<GPtr<u8>>(veh, 0x14);
        let pos = if m.is_null() { veh.byte_add(4) } else { m.byte_add(0x30) };
        let (px, py, pz) = (rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8));
        if !in_range(px, x0, x1) || !in_range(py, y0, y1) || !in_range(pz, z0, z1) {
            continue;
        }
        if rd::<u8>(veh, 0x428) & 8 != 0 || can_be_deleted(veh) == 0 {
            continue;
        }
        let drv = rd::<GPtr<u8>>(veh, 0x460);
        if !drv.is_null() {
            remove_ped(drv);
            let d = rd::<GPtr<u8>>(veh, 0x460);
            if !d.is_null() {
                clean_up_old_reference(d, veh.byte_add(0x460));
            }
            wr::<u32>(veh, 0x460, 0);
        }
        let mut k = 0;
        while k < i32::from(rd::<u8>(veh, 0x488)) {
            let p = rd::<GPtr<u8>>(veh, 0x464 + 4 * k as isize);
            if !p.is_null() {
                remove_passenger(veh, p);
                remove_ped(p);
            }
            k += 1;
        }
        if is_this_vehicle_interesting(veh) != 0 {
            store_car_in_nearest_impounding_garage(veh);
        }
        remove_from_interesting_vehicle_list(veh);
        world_remove(veh);
        v_delete(veh, 1);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CLEAR_CARS_FROM_AREA];
