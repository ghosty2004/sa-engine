//! PORT-VEH-4 (batch `veh4-4`): `CVehicle::FindTyreNearestPoint`, `CAutomobile::GetMovingCollisionOffset`
//! and `CVehicle::CanVehicleBeDamaged`. Ported from the 1.0 US bytes; gta-reversed as a reading aid
//! (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use crate::common::{f32_of, rd, wr, x, xi};

orig! {
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(n: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(n: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CPhysical::CanPhysicalBeDamaged(eWeaponType, bool*)` (0x5448B0, AL)
    fn physical_can_be_damaged(p: GPtr<u8>, w: u32, out: GPtr<u8>) -> u8 = thiscall 0x5448B0 => unported;
}

/// `CVehicle::FindTyreNearestPoint(float x, float y)` (0x6D7BC0): the wheel quadrant of the point
/// relative to the vehicle (forward and right components, z ignored as `axis.z * 0.0`): a bike gives
/// 0 (front) or 2 (rear); a car 0/1 (front left/right) or 2/3 (rear left/right).
///
/// Kept as in the original: the position falls back to the placement without a matrix, but the axes
/// are read from the (null) matrix.
#[hook(addr = 0x6D7BC0, conv = thiscall, name = "CVehicle::FindTyreNearestPoint", this = "CVehicle",
       expect = "8B 41 14 83 EC 0C",
       unverified = "veh4-4", route = "L2", hazards = "none",
       note = "stores the right component into its own argument slot")]
pub fn vehicle_find_tyre_nearest_point(this: GPtr<u8>, px: f32, py: f32) -> i32 {
    let m = rd::<GPtr<u8>>(this, 0x14);
    let p = if m.is_null() { this.byte_add(4) } else { m.byte_add(0x30) };
    let dx = f32_of(x(px).fsub_m(rd::<f32>(p, 0)));
    let dy = f32_of(x(py).fsub_m(rd::<f32>(p, 4)));
    let fwd = x(dy)
        .fmul_m(rd::<f32>(m, 0x14))
        .fadd(x(dx).fmul_m(rd::<f32>(m, 0x10)))
        .fadd(x(rd::<f32>(m, 0x18)).fmul_m(0.0));
    let right = f32_of(
        x(dy).fmul_m(rd::<f32>(m, 4)).fadd(x(dx).fmul_m(rd::<f32>(m, 0))).fadd(x(rd::<f32>(m, 8)).fmul_m(0.0)),
    );
    let front = fwd.fcom(x(0.0)) == Cmp::Greater;
    let rgt = x(right).fcom(x(0.0)) == Cmp::Greater;
    if rd::<u32>(this, 0x590) == 9 {
        return if front { 0 } else { 2 };
    }
    match (front, rgt) {
        (true, true) => 1,
        (true, false) => 0,
        (false, true) => 3,
        (false, false) => 2,
    }
}

/// `CAutomobile::GetMovingCollisionOffset()` (0x6A2150): the moving part's collision offset
/// `(u16)+0x86C * k` for the Dumper (406, with its +0x6A0 part), Packer (443), Dozer (486, +0x698),
/// Andromada (592, +0x6A8) and Forklift (530), else 0 (also when +0x86C is 0). ST0.
#[hook(addr = 0x6A2150, conv = thiscall, name = "CAutomobile::GetMovingCollisionOffset", this = "CAutomobile",
       expect = "51 66 8B 91 6C 08 00 00",
       unverified = "veh4-4", route = "L2", hazards = "none")]
pub fn automobile_get_moving_collision_offset(this: GPtr<u8>) -> X87 {
    let v = rd::<u16>(this, 0x86C);
    if v == 0 {
        return x(0.0);
    }
    let model = rd::<i16>(this, 0x22);
    let k = |bits: u32| xi(i32::from(v)).fmul_m(f32::from_bits(bits));
    if model == 0x196 && rd::<u32>(this, 0x6A0) != 0 {
        return k(0x3951_B717);
    }
    if model == 0x1BB {
        return k(0xB8D1_B717);
    }
    if model == 0x1E6 && rd::<u32>(this, 0x698) != 0 {
        return k(0x3951_B717);
    }
    if model == 0x250 && rd::<u32>(this, 0x6A8) != 0 {
        return k(0x391D_4952);
    }
    if model == 0x212 {
        return k(0x3A1D_4952);
    }
    x(0.0)
}

/// `CVehicle::CanVehicleBeDamaged(CEntity* damager, eWeaponType weapon, bool* fireOrExplosion)`
/// (0x6D1280): false unless damageable (+0x42A bit 5); a moving vehicle flagged +0x40 bit 22 only by
/// the player's ped or vehicle; never by the player's ped when it targets this vehicle (+0xFC); then
/// `CanPhysicalBeDamaged` (clearing `*fireOrExplosion` first), except that a player-controlled vehicle
/// below 250 health is not damaged further by fire/explosions.
#[hook(addr = 0x6D1280, conv = thiscall, name = "CVehicle::CanVehicleBeDamaged", this = "CVehicle",
       expect = "56 8B F1 F6 86 2A 04 00 00 20",
       unverified = "veh4-4", route = "L2", hazards = "none")]
pub fn vehicle_can_vehicle_be_damaged(this: GPtr<u8>, damager: GPtr<u8>, weapon: u32, out: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x42A) & 0x20 == 0 {
        return 0;
    }
    if rd::<u8>(this, 0x36) & 0xF8 != 0
        && rd::<u32>(this, 0x40) & 0x40_0000 != 0
        && damager != find_player_ped(-1)
        && damager != find_player_vehicle(-1, 0)
    {
        return 0;
    }
    if !find_player_ped(-1).is_null()
        && damager == find_player_ped(-1)
        && rd::<GPtr<u8>>(find_player_ped(-1), 0xFC) == this
    {
        return 0;
    }
    wr::<u8>(out, 0, 0);
    if physical_can_be_damaged(this, weapon, out) == 0 {
        return 0;
    }
    if rd::<u8>(out, 0) != 0
        && rd::<u8>(this, 0x36) & 0xF8 == 0
        && x(rd::<f32>(this, 0x4C0)).fcom(x(250.0)) == Cmp::Less
    {
        return 0;
    }
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_VEHICLE_FIND_TYRE_NEAREST_POINT,
    HOOK_AUTOMOBILE_GET_MOVING_COLLISION_OFFSET,
    HOOK_VEHICLE_CAN_VEHICLE_BE_DAMAGED,
];
