//! WRITER-CLIMB (batch `climb-8`): `CGarage::StoreAndRemoveCarsForThisHideOut` (0x449900), reached when the hideout
//! garage door shuts behind CJ in the native Grove Street runs. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, wr};

use crate::bindings::garage_is_point_inside;
use crate::bindings::world_remove;
use crate::bindings::stored_car_store;
orig! {
    /// `CGarage::EntityHasASphereWayOutsideGarage(CEntity*, float)` (0x449050)
    fn garage_entity_has_sphere_way_outside(g: GPtr<u8>, e: GPtr<u8>, margin: f32) -> u8 = thiscall 0x449050 => unported;
    /// `CPlayerInfo::CancelPlayerEnteringCars(CVehicle*)` (0x56E860)
    fn player_info_cancel_entering_cars(p: GPtr<u8>, v: GPtr<u8>) = thiscall 0x56E860 => unported;
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
        if garage_is_point_inside(this, sa_math::vector::CVector::new(f32::from_bits(rd::<u32>(p, 0)), f32::from_bits(rd::<u32>(p, 4)), f32::from_bits(rd::<u32>(p, 8)))) == 0 {
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
/// `CStoredCar::StoreCar(CVehicle* v)` (0x449760, thiscall, `ret 4`; FRONTIER-2 lane C turn 5): model (+0x12) from the
/// vehicle's +0x22; the position (+0 / +4 / +8, each read from the matrix +0x30 or the placement +4); the forward
/// vector bytes +0x3C..+0x3E = `ftol(matrix +0x10 / +0x14 / +0x18 * [0x858628])`; the colours +0x32..+0x35 (vehicle
/// +0x434..), +0x36 (+0x1D2), the handling flags +0xC (+0x38C), +0x37 / +0x38 (+0x438 / +0x439); the flag word +0x10
/// from the physical flags +0x40 (0x40000 -> 1, 0x80000 -> 2, 0x800000 -> 4, 0x100000 -> 8, 0x200000 -> 0x10), the
/// byte +0x42E bit 0x10 (0x20) and the handling flags 0x20000 (0x40) / 0x80000 (0x80); for a vehicle sub-class (+0x590)
/// 0 or 9 the byte +0x39 = `+0x4A8 & 7` (else untouched); the 15 upgrade words +0x43A to +0x14; +0x3A =
/// `GetRemapIndex()`, +0x3B = +0x48A.
#[hook(addr = 0x449760, conv = thiscall, name = "CStoredCar::StoreCar", expect = "53 56 8B 74 24 0C",
       unverified = "frontier2-c11", route = "L2", hazards = "none")]
pub fn store_car(this: GPtr<u8>, v: GPtr<u8>) {
    wr::<u16>(this, 0x12, rd::<u16>(v, 0x22));
    let pos = || {
        let m = rd::<u32>(v, 0x14);
        if m != 0 {
            g::<u8>(m as usize + 0x30)
        } else {
            v.byte_add(4)
        }
    };
    wr::<u32>(this, 0, rd::<u32>(pos(), 0));
    wr::<u32>(this, 4, rd::<u32>(pos(), 4));
    wr::<u32>(this, 8, rd::<u32>(pos(), 8));
    for k in 0..3isize {
        let m: GPtr<u8> = g(rd::<u32>(v, 0x14) as usize);
        let b = super::common::ftol32(super::common::x(rd::<f32>(m, 0x10 + 4 * k)).fmul_m(g::<f32>(0x858628).read()));
        wr::<u8>(this, 0x3C + k, b as u8);
    }
    for k in 0..4isize {
        wr::<u8>(this, 0x32 + k, rd::<u8>(v, 0x434 + k));
    }
    wr::<u8>(this, 0x36, rd::<u8>(v, 0x1D2));
    wr::<u32>(this, 0xC, rd::<u32>(v, 0x38C));
    wr::<u8>(this, 0x37, rd::<u8>(v, 0x438));
    wr::<u8>(this, 0x38, rd::<u8>(v, 0x439));
    wr::<u16>(this, 0x10, 0);
    let flags = |bit: u32| rd::<u32>(v, 0x40) & bit != 0;
    if flags(0x40000) {
        wr::<u16>(this, 0x10, 1);
    }
    let or = |b: u8| wr::<u8>(this, 0x10, rd::<u8>(this, 0x10) | b);
    if flags(0x80000) {
        or(2);
    }
    if flags(0x800000) {
        or(4);
    }
    if flags(0x100000) {
        or(8);
    }
    if flags(0x200000) {
        wr::<u16>(this, 0x10, rd::<u16>(this, 0x10) | 0x10);
    }
    if rd::<u8>(v, 0x42E) & 0x10 != 0 {
        or(0x20);
    }
    if rd::<u32>(v, 0x38C) & 0x20000 != 0 {
        or(0x40);
    }
    if rd::<u32>(v, 0x38C) & 0x80000 != 0 {
        or(0x80);
    }
    let sub = rd::<u32>(v, 0x590);
    if sub == 0 || sub == 9 {
        wr::<u8>(this, 0x39, rd::<u8>(v, 0x4A8) & 7);
    }
    for k in 0..15isize {
        wr::<u16>(this, 0x14 + 2 * k, rd::<u16>(v, 0x43A + 2 * k));
    }
    let remap = crate::bindings::get_remap_index(v);
    wr::<u8>(this, 0x3A, remap as u8);
    wr::<u8>(this, 0x3B, rd::<u8>(v, 0x48A));
}

pub const HOOKS: &[HookReg] = &[HOOK_STORE_AND_REMOVE_CARS_FOR_THIS_HIDE_OUT, HOOK_STORE_CAR];
