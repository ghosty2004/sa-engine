//! WRITER-NATIVE (batch `native-28`): `CStoredCar::RestoreCar` (0x447E40) and the hideout garage's
//! `RestoreCarsForThisHideOut` (0x448550, a `.HOODLUM` stub to 0x156D8D0), reached near CJ's garage right after
//! start-in-world. Ported from the 1.0 US bytes; x87 arithmetic follows the original's instruction order.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, xi};

orig! {
    /// `CStreaming::RequestModel(int32, int32 flags)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::RequestVehicleUpgrade(int32, int32 flags)` (0x408C70)
    fn streaming_request_vehicle_upgrade(id: i32, flags: i32) = cdecl 0x408C70 => unported;
    /// `CStreaming::HasVehicleUpgradeLoaded(int32)` (0x407820)
    fn streaming_has_vehicle_upgrade_loaded(id: i32) -> u8 = cdecl 0x407820 => unported;
    /// `CVehicle::operator new(uint32)` (0x6E2D50)
    fn vehicle_new(size: u32) -> GPtr<u8> = cdecl 0x6E2D50 => unported;
    /// `CMonsterTruck(int32 modelIndex, eVehicleCreatedBy)` (0x6C8D60)
    fn monster_truck_ctor(v: GPtr<u8>, mi: u32, by: u32) -> GPtr<u8> = thiscall 0x6C8D60 => unported;
    /// `CQuadBike(int32, eVehicleCreatedBy)` (0x6CE370)
    fn quad_bike_ctor(v: GPtr<u8>, mi: u32, by: u32) -> GPtr<u8> = thiscall 0x6CE370 => unported;
    /// `CHeli(int32, eVehicleCreatedBy)` (0x6C4190)
    fn heli_ctor(v: GPtr<u8>, mi: u32, by: u32) -> GPtr<u8> = thiscall 0x6C4190 => unported;
    /// `CPlane(int32, eVehicleCreatedBy)` (0x6C8E20)
    fn plane_ctor(v: GPtr<u8>, mi: u32, by: u32) -> GPtr<u8> = thiscall 0x6C8E20 => unported;
    /// `CBike(int32, eVehicleCreatedBy)` (0x6BF430)
    fn bike_ctor(v: GPtr<u8>, mi: u32, by: u32) -> GPtr<u8> = thiscall 0x6BF430 => unported;
    /// `CBmx(int32, eVehicleCreatedBy)` (0x6BF820)
    fn bmx_ctor(v: GPtr<u8>, mi: u32, by: u32) -> GPtr<u8> = thiscall 0x6BF820 => unported;
    /// `CTrailer(int32, eVehicleCreatedBy)` (0x6D03A0)
    fn trailer_ctor(v: GPtr<u8>, mi: u32, by: u32) -> GPtr<u8> = thiscall 0x6D03A0 => unported;
    /// `CBoat(int32, eVehicleCreatedBy)` (0x6F2940)
    fn boat_ctor(v: GPtr<u8>, mi: u32, by: u32) -> GPtr<u8> = thiscall 0x6F2940 => unported;
    /// `CAutomobile(int32, eVehicleCreatedBy, bool)` (0x6B0A90)
    fn automobile_ctor(v: GPtr<u8>, mi: u32, by: u32, b: u32) -> GPtr<u8> = thiscall 0x6B0A90 => unported;
    /// `CVehicle::SetupUpgradesAfterLoad()` (0x6E3400)
    fn vehicle_setup_upgrades_after_load(v: GPtr<u8>) = thiscall 0x6E3400 => unported;
    /// `CVehicle::SetRemap(int32)` (0x6D0C00)
    fn vehicle_set_remap(v: GPtr<u8>, i: i32) = thiscall 0x6D0C00 => unported;
    /// `CStoredCar::RestoreCar()` (0x447E40)
    fn stored_car_restore_c(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x447E40 => stored_car_restore;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
}

/// `CVehicle* CStoredCar::RestoreCar()` (0x447E40, SEH frame).
///
/// Requests the model (+0x12, flags 8) and the 15 upgrades (+0x14 words, -1 = none). Null until the model
/// (`ms_aInfoForModel[mi]` state 1) and every upgrade are loaded. The light overrides 0x8A6459 / 0x8A6458 take
/// +0x38 / +0x37. A vehicle of the model's type (model info +0x3C: monster truck, quad, heli, plane, boat, bike
/// (+0x614 |= 0x10), BMX (same), trailer, else automobile), created by mission type 1. Position +0, forward
/// `(a, b, c) * 0.01` from the int8s +0x3C..+0x3E, right `(b, -a, 0)`, up `(0, 0, 1)`; status abandoned, no
/// driver, +0x1D2 = +0x36, handling flags +0x38C = +0xC, +0x428 bit 7 cleared, +0x42A |= 2, +0x4F8 = 1;
/// automobiles take the lock bits +0x4A8 & 7 from +0x39. Bits of +0x10: proofs into +0x40 (0x40000, 0x80000,
/// 0x800000, 0x100000, 0x200000), 0x20 tyres (+0x42E |= 0x10, +0x1BE = 1), 0x40 / 0x80 handling flags 0x20000 /
/// 0x80000. Upgrades copied to +0x43A, `SetupUpgradesAfterLoad`, `SetRemap(+0x3A)`, +0x428 bit 4 cleared, radio
/// +0x48A = +0x3B, colours +0x434..+0x437 = +0x32..+0x35, +0x42F |= 4. The original writes +0x614 of a null bike
/// when the pool is full.
#[hook(addr = 0x447E40, conv = thiscall, name = "CStoredCar::RestoreCar",
       expect = "6A FF 68 93 79 83 00",
       unverified = "native-28", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "creates a vehicle (pools)")]
pub fn stored_car_restore(this: GPtr<u8>) -> GPtr<u8> {
    let mi = u32::from(rd::<u16>(this, 0x12));
    streaming_request_model(mi as i32, 8);
    for k in 0..15 {
        let u = rd::<i16>(this, 0x14 + 2 * k);
        if u != -1 {
            streaming_request_vehicle_upgrade(i32::from(u), 0);
        }
    }
    let mi = u32::from(rd::<u16>(this, 0x12));
    if g::<u8>(0x8E_4CD0 + (mi as usize) * 20).read() != 1 {
        return GPtr::NULL;
    }
    for k in 0..15 {
        let u = rd::<i16>(this, 0x14 + 2 * k);
        if u != -1 && streaming_has_vehicle_upgrade_loaded(i32::from(u)) == 0 {
            return GPtr::NULL;
        }
    }
    g::<u8>(0x8A_6459).write(rd::<u8>(this, 0x38));
    g::<u8>(0x8A_6458).write(rd::<u8>(this, 0x37));
    let mi = u32::from(rd::<u16>(this, 0x12));
    let info = g::<GPtr<u8>>(0xA9_B0C8 + (mi as usize) * 4).read();
    let make = |size: u32, f: &dyn Fn(GPtr<u8>) -> GPtr<u8>| {
        let v = vehicle_new(size);
        if v.is_null() { v } else { f(v) }
    };
    let mi = u32::from(rd::<u16>(this, 0x12));
    let veh = match rd::<u32>(info, 0x3C).wrapping_sub(1) {
        0 => make(0x99C, &|v| monster_truck_ctor(v, mi, 1)),
        1 => make(0x9BC, &|v| quad_bike_ctor(v, mi, 1)),
        2 => make(0xA18, &|v| heli_ctor(v, mi, 1)),
        3 => make(0xA04, &|v| plane_ctor(v, mi, 1)),
        4 => make(0x7E8, &|v| boat_ctor(v, mi, 1)),
        8 | 9 => {
            let v = if rd::<u32>(info, 0x3C) == 9 {
                make(0x814, &|v| bike_ctor(v, mi, 1))
            } else {
                make(0x838, &|v| bmx_ctor(v, mi, 1))
            };
            wr::<u8>(v, 0x614, rd::<u8>(v, 0x614) | 0x10);
            v
        }
        10 => make(0x9F4, &|v| trailer_ctor(v, mi, 1)),
        _ => make(0x988, &|v| automobile_ctor(v, mi, 1, 1)),
    };
    let m = rd::<GPtr<u8>>(veh, 0x14);
    if !m.is_null() {
        wr::<u32>(m, 0x30, rd::<u32>(this, 0));
        wr::<u32>(rd::<GPtr<u8>>(veh, 0x14), 0x34, rd::<u32>(this, 4));
        wr::<u32>(rd::<GPtr<u8>>(veh, 0x14), 0x38, rd::<u32>(this, 8));
    } else {
        wr::<u32>(veh, 4, rd::<u32>(this, 0));
        wr::<u32>(veh, 8, rd::<u32>(this, 4));
        wr::<u32>(veh, 0xC, rd::<u32>(this, 8));
    }
    let k = f32::from_bits(0x3C23_D70A);
    let a = xi(i32::from(rd::<i8>(this, 0x3C))).fmul_m(k);
    let b = xi(i32::from(rd::<i8>(this, 0x3D))).fmul_m(k);
    let c = xi(i32::from(rd::<i8>(this, 0x3E))).fmul_m(k);
    let mat = |o: isize, v: f32| wr::<f32>(rd::<GPtr<u8>>(veh, 0x14), o, v);
    mat(0x10, a.to_f32());
    mat(0x14, b.to_f32());
    mat(0x18, c.to_f32());
    mat(0, b.to_f32());
    mat(4, a.fchs().to_f32());
    wr::<u32>(rd::<GPtr<u8>>(veh, 0x14), 8, 0);
    wr::<u8>(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 0x20);
    wr::<u32>(rd::<GPtr<u8>>(veh, 0x14), 0x20, 0);
    wr::<u32>(rd::<GPtr<u8>>(veh, 0x14), 0x24, 0);
    wr::<u32>(rd::<GPtr<u8>>(veh, 0x14), 0x28, 0x3F80_0000);
    let b42a = rd::<u8>(veh, 0x42A);
    wr::<u32>(veh, 0x460, 0);
    wr::<u8>(veh, 0x1D2, rd::<u8>(this, 0x36));
    wr::<u32>(veh, 0x38C, rd::<u32>(this, 0xC));
    wr::<u8>(veh, 0x428, rd::<u8>(veh, 0x428) & 0x7F);
    wr::<u8>(veh, 0x42A, b42a | 2);
    wr::<u32>(veh, 0x4F8, 1);
    if rd::<u32>(veh, 0x590) == 0 {
        let l = rd::<u8>(veh, 0x4A8);
        wr::<u8>(veh, 0x4A8, (l & !7) | (rd::<u8>(this, 0x39) & 7));
    }
    let fl = rd::<u8>(this, 0x10);
    let or40 = |v: u32| wr::<u32>(veh, 0x40, rd::<u32>(veh, 0x40) | v);
    if fl & 1 != 0 {
        or40(0x40000);
    }
    if fl & 2 != 0 {
        or40(0x80000);
    }
    if fl & 4 != 0 {
        or40(0x80_0000);
    }
    if fl & 8 != 0 {
        or40(0x10_0000);
    }
    if fl & 0x10 != 0 {
        or40(0x20_0000);
    }
    if fl & 0x20 != 0 {
        wr::<u8>(veh, 0x42E, rd::<u8>(veh, 0x42E) | 0x10);
        wr::<u8>(veh, 0x1BE, 1);
    }
    if fl & 0x40 != 0 {
        wr::<u32>(veh, 0x38C, rd::<u32>(veh, 0x38C) | 0x20000);
    }
    if fl & 0x80 != 0 {
        wr::<u32>(veh, 0x38C, rd::<u32>(veh, 0x38C) | 0x80000);
    }
    for k in 0..15 {
        wr::<u16>(veh, 0x43A + 2 * k, rd::<u16>(this, 0x14 + 2 * k));
    }
    vehicle_setup_upgrades_after_load(veh);
    vehicle_set_remap(veh, i32::from(rd::<i8>(this, 0x3A)));
    wr::<u8>(veh, 0x428, rd::<u8>(veh, 0x428) & 0xEF);
    wr::<u8>(veh, 0x48A, rd::<u8>(this, 0x3B));
    wr::<u8>(veh, 0x434, rd::<u8>(this, 0x32));
    wr::<u8>(veh, 0x435, rd::<u8>(this, 0x33));
    wr::<u8>(veh, 0x436, rd::<u8>(this, 0x34));
    wr::<u8>(veh, 0x42F, rd::<u8>(veh, 0x42F) | 4);
    wr::<u8>(veh, 0x437, rd::<u8>(this, 0x35));
    veh
}

/// `bool CGarage::RestoreCarsForThisHideOut(CStoredCar* cars)` (0x448550, the `.HOODLUM` body 0x156D8D0, `ret 4`;
/// `this` unused). For the 4 stored cars (0x40 bytes) with a model (+0x12): `RestoreCar`; a restored vehicle gets
/// +0x42D bit 5 cleared, is added to the world and the slot's model cleared. True when all four slots are empty.
#[hook(addr = 0x448550, conv = thiscall, name = "thunk_FUN_0156d8d0",
       expect = "E9 7B 53 12 01",
       unverified = "native-28", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "the entry is a jmp to the .HOODLUM body 0x156D8D0")]
pub fn restore_cars_for_this_hideout(_this: GPtr<u8>, cars: GPtr<u8>) -> u8 {
    for i in 0..4 {
        let car = cars.byte_add(0x40 * i);
        if rd::<u16>(car, 0x12) == 0 {
            continue;
        }
        let v = stored_car_restore_c(car);
        if v.is_null() {
            continue;
        }
        wr::<u8>(v, 0x42D, rd::<u8>(v, 0x42D) & 0xDF);
        world_add(v);
        wr::<u16>(car, 0x12, 0);
    }
    u8::from((0..4).all(|i| rd::<u16>(cars.byte_add(0x40 * i), 0x12) == 0))
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_STORED_CAR_RESTORE, HOOK_RESTORE_CARS_FOR_THIS_HIDEOUT];
