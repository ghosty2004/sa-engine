//! WRITER-CLIMB (batch `climb-8`): `CGarage::StoreAndRemoveCarsForThisHideOut` (0x449900), reached when the hideout
//! garage door shuts behind CJ in the native Grove Street runs. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr};

orig! {
    /// `CGarage::IsPointInsideGarage(CVector)` (0x448740, `ret 0xC`)
    fn garage_is_point_inside(g: GPtr<u8>, x: u32, y: u32, z: u32) -> u8 = thiscall 0x448740 => unported;
    /// `CGarage::EntityHasASphereWayOutsideGarage(CEntity*, float)` (0x449050)
    fn garage_entity_has_sphere_way_outside(g: GPtr<u8>, e: GPtr<u8>, margin: f32) -> u8 = thiscall 0x449050 => unported;
    /// `CStoredCar::StoreCar(CVehicle*)` (0x449760)
    fn stored_car_store(s: GPtr<u8>, v: GPtr<u8>) = thiscall 0x449760 => unported;
    /// `CPlayerInfo::CancelPlayerEnteringCars(CVehicle*)` (0x56E860)
    fn player_info_cancel_entering_cars(p: GPtr<u8>, v: GPtr<u8>) = thiscall 0x56E860 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// the vehicle's scalar deleting destructor (vtable +0x00)
    fn entity_scalar_deleting_destructor(e: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x00 => unported;
}

/// `CPools::ms_pVehiclePool` (0xB74494): slots +0, flags +4, size +8; 0xA18-byte vehicles.
const VEHICLE_POOL: usize = 0xB7_4494;

/// Clears stored-car slots `from..4` (model word +0x12 of the 0x40-byte `CStoredCar`s).
fn clear_from(stored: GPtr<u8>, from: i32) {
    for k in from..4 {
        wr::<u16>(stored, k as isize * 0x40 + 0x12, 0);
    }
}

/// `void CGarage::StoreAndRemoveCarsForThisHideOut(CStoredCar* stored, int32 maxSlot)` (0x449900, `ret 8`).
///
/// The four slots are cleared. Over the vehicle pool from the last slot down, every used vehicle whose position
/// `IsPointInsideGarage` and that is not a mission vehicle (+0x4A4 != 2) is stored in the next slot while fewer
/// than `min(maxSlot, 4)` are stored and no sphere is way outside (margin 1), then taken from the focused player's
/// entering (`CancelPlayerEnteringCars`), removed from the world and deleted. The slots left are cleared again.
#[hook(addr = 0x449900, conv = thiscall, name = "CGarage::StoreAndRemoveCarsForThisHideOut",
       expect = "83 EC 0C 8B 54 24 10 53",
       unverified = "climb-8", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn store_and_remove_cars_for_this_hide_out(this: GPtr<u8>, stored: GPtr<u8>, max_slot: i32) {
    clear_from(stored, 0);
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let mut n = 0i32;
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
            continue;
        }
        let v: GPtr<u8> = GPtr::from_addr(rd::<u32>(pool, 0).wrapping_add((i as u32).wrapping_mul(0xA18)) as usize);
        if v.is_null() {
            continue;
        }
        let m = rd::<GPtr<u8>>(v, 0x14);
        let p = if m.is_null() { v.byte_add(4) } else { m.byte_add(0x30) };
        if garage_is_point_inside(this, rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)) == 0 {
            continue;
        }
        if rd::<u8>(v, 0x4A4) == 2 {
            continue;
        }
        if n < max_slot.min(4) && garage_entity_has_sphere_way_outside(this, v, 1.0) == 0 {
            stored_car_store(stored.byte_add(n as isize * 0x40), v);
            n += 1;
        }
        let focus = u32::from(g::<u8>(0xB7_CD74).read());
        player_info_cancel_entering_cars(GPtr::from_addr((focus * 0x190 + 0xB7_CD98) as usize), v);
        world_remove(v);
        entity_scalar_deleting_destructor(v, 1);
    }
    if n < 4 {
        clear_from(stored, n);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_STORE_AND_REMOVE_CARS_FOR_THIS_HIDE_OUT];
