//! PORT-VEH-4 (batch `veh4-4`): `CVehicle::GetPlaneGunsRateOfFire`, `CVehicle::GetPlaneOrdnanceRateOfFire`
//! and `CVehicle::GetUpgrade`. Ported from the 1.0 US bytes; gta-reversed as a reading aid (the bytes
//! win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, rd, x, xi};

orig! {
    /// `RpClumpForAllAtomics(RpClump*, RpAtomicCallBack, void*)` (0x749B70)
    fn clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `CVisibilityPlugins::GetAtomicId(RpAtomic*)` (0x732250)
    fn get_atomic_id(a: GPtr<u8>) -> i32 = cdecl 0x732250 => unported;
}

/// The weapon-upgrade fire-rate factor: `level / 16` of the plane (+0x9D8, type 4) or heli (+0xA10,
/// type 3) weapon level, else 1.
fn level_factor(this: GPtr<u8>) -> X87 {
    let k = f32::from_bits(0x3D80_0000);
    match rd::<u32>(this, 0x594) {
        4 => xi(i32::from(rd::<u8>(this, 0x9D8))).fmul_m(k),
        3 => xi(i32::from(rd::<u8>(this, 0xA10))).fmul_m(k),
        _ => x(1.0),
    }
}

/// `CVehicle::GetPlaneGunsRateOfFire()` (0x6D40E0): `ftol(rate / factor)` with the rate by model
/// (447/464 40, 476 80, 520 17, 548 100, 576 45, else 60) and the weapon-level factor.
#[hook(addr = 0x6D40E0, conv = thiscall, name = "CVehicle::GetPlaneGunsRateOfFire", this = "CVehicle",
       expect = "51 8B 81 94 05 00 00",
       unverified = "veh4-4", route = "L2", hazards = "none")]
pub fn vehicle_get_plane_guns_rate_of_fire(this: GPtr<u8>) -> i32 {
    let f = level_factor(this);
    let rate = match rd::<i16>(this, 0x22) {
        447 | 464 => 40,
        476 => 80,
        520 => 17,
        548 => 100,
        576 => 45,
        _ => 60,
    };
    ftol32(xi(rate).fdiv(f))
}

/// `CVehicle::GetPlaneOrdnanceRateOfFire(eOrdnanceType)` (0x6D4590): `ftol(rate / factor)` with the rate
/// by model (425 500, 447/476 1000, 520 500 for ordnance 1 else 1000, else 350).
#[hook(addr = 0x6D4590, conv = thiscall, name = "CVehicle::GetPlaneOrdnanceRateOfFire", this = "CVehicle",
       expect = "51 8B 81 94 05 00 00",
       unverified = "veh4-4", route = "L2", hazards = "none",
       note = "the original builds the rate in its argument slot")]
pub fn vehicle_get_plane_ordnance_rate_of_fire(this: GPtr<u8>, ordnance: u32) -> i32 {
    let f = level_factor(this);
    let rate = match rd::<i16>(this, 0x22) {
        425 => 500,
        447 | 476 => 1000,
        520 => {
            if ordnance == 1 {
                500
            } else {
                1000
            }
        }
        _ => 350,
    };
    ftol32(xi(rate).fdiv(f))
}

/// `CVehicle::GetUpgrade(int32 slot)` (0x6D3650): the model id of the clump atomic in that upgrade slot
/// (`FindUpgradeCB` 0x6D3370), else the special upgrades by flag: nitro (slot 15, +0x38C bit 19, model
/// 0x8CD760), hydraulics (slot 16, +0x38C bit 17, 0x8CD76C), stereo (slot 17, +0x42E bit 4, 0x8CD770);
/// -1 otherwise.
#[hook(addr = 0x6D3650, conv = thiscall, name = "CVehicle::GetUpgrade", this = "CVehicle",
       expect = "83 EC 08 56 57",
       unverified = "veh4-4", route = "L2", hazards = "none")]
pub fn vehicle_get_upgrade(this: GPtr<u8>, slot: i32) -> i32 {
    let mut data = [slot as u32, 0];
    clump_for_all_atomics(rd::<GPtr<u8>>(this, 0x18), 0x6D_3370, GPtr::from_raw(&mut data as *mut [u32; 2]).cast());
    if data[1] != 0 {
        return get_atomic_id(GPtr::from_addr(data[1] as usize));
    }
    let model = |a: usize| i32::from(g::<u16>(a).read());
    match slot {
        15 if rd::<u32>(this, 0x38C) & 0x8_0000 != 0 => model(0x8C_D760),
        16 if rd::<u32>(this, 0x38C) & 0x2_0000 != 0 => model(0x8C_D76C),
        17 if rd::<u8>(this, 0x42E) & 0x10 != 0 => model(0x8C_D770),
        _ => -1,
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_VEHICLE_GET_PLANE_GUNS_RATE_OF_FIRE,
    HOOK_VEHICLE_GET_PLANE_ORDNANCE_RATE_OF_FIRE,
    HOOK_VEHICLE_GET_UPGRADE,
];
