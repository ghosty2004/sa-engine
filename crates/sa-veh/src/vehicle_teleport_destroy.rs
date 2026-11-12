//! PORT-VEH-4 (batch `veh4-4`): `CBoat::Teleport`, `CBike::Teleport` and
//! `CVehicle::DestroyVehicleAndDriverAndPassengers`. Ported from the 1.0 US bytes; gta-reversed as a
//! reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{rd, wr};

orig! {
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CPlaceable::SetOrientation(float, float, float)` (0x439A80)
    fn placeable_set_orientation(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x439A80 => unported;
    /// `CBike::ResetSuspension()` (0x6B6740)
    fn bike_reset_suspension(b: GPtr<u8>) = thiscall 0x6B6740 => unported;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390, AL)
    fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CDarkel::RegisterKillByPlayer(const CPed*, eWeaponType, bool headShot, int32 player)` (0x43DCD0)
    fn darkel_register_kill_by_player(ped: GPtr<u8>, w: u32, head: u32, n: u32) = cdecl 0x43DCD0 => unported;
    /// virtual (vtable +0x54) of `CPed`, removing it
    fn v_ped_slot_54(p: GPtr<u8>) = thiscall vslot 0x54 => unported;
    /// virtual deleting destructor (vtable +0)
    fn v_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
}

/// The shared `Teleport` body: out of the world, the position set (matrix or placement), the
/// orientation reset when asked, move and turn speeds cleared, `extra` (the bike's suspension reset),
/// back into the world.
fn teleport(this: GPtr<u8>, px: u32, py: u32, pz: u32, reset: u32, extra: impl FnOnce(GPtr<u8>)) {
    world_remove(this);
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        wr::<u32>(this, 4, px);
        wr::<u32>(this, 8, py);
        wr::<u32>(this, 0xC, pz);
    } else {
        wr::<u32>(m, 0x30, px);
        wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x34, py);
        wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x38, pz);
    }
    if reset as u8 != 0 {
        placeable_set_orientation(this, 0, 0, 0);
    }
    for o in [0x44isize, 0x48, 0x4C, 0x50, 0x54, 0x58] {
        wr::<u32>(this, o, 0);
    }
    extra(this);
    world_add(this);
}

/// `CBoat::Teleport(CVector, bool resetRotation)` (0x6F20E0).
#[hook(addr = 0x6F20E0, conv = thiscall, name = "CBoat::Teleport", this = "CBoat",
       expect = "56 8B F1 56 E8 97 11 E7 FF",
       unverified = "veh4-4", route = "L2", hazards = "heap-writes",
       note = "re-adds the boat to the world (sector lists)")]
pub fn boat_teleport(this: GPtr<u8>, px: u32, py: u32, pz: u32, reset: u32) {
    teleport(this, px, py, pz, reset, |_| {});
}

/// `CBike::Teleport(CVector, bool resetRotation)` (0x6BCFC0): as the boat's, with the suspension reset.
#[hook(addr = 0x6BCFC0, conv = thiscall, name = "CBike::Teleport", this = "CBike",
       expect = "56 8B F1 56 E8 B7 62 EA FF",
       unverified = "veh4-4", route = "L2", hazards = "heap-writes",
       note = "re-adds the bike to the world (sector lists)")]
pub fn bike_teleport(this: GPtr<u8>, px: u32, py: u32, pz: u32, reset: u32) {
    teleport(this, px, py, pz, reset, bike_reset_suspension);
}

/// Removes an occupant: a kill by the player (weapon 0x37) unless a co-op game is on, then the ped's
/// slot-0x54 virtual.
fn remove_occupant(ped: GPtr<u8>) {
    if is_coop_game_going_on() == 0 {
        darkel_register_kill_by_player(ped, 0x37, 0, 0);
    }
    v_ped_slot_54(ped);
}

/// `CVehicle::DestroyVehicleAndDriverAndPassengers(CVehicle*)` (0x6D2250): the driver and each passenger
/// (+0x464, count +0x488 re-read per step) are removed, the vehicle taken out of the world and
/// deleted.
#[hook(addr = 0x6D2250, conv = cdecl, name = "CVehicle::DestroyVehicleAndDriverAndPassengers",
       expect = "53 57 8B 7C 24 0C",
       unverified = "veh4-4", route = "L3-only", hazards = "heap-writes",
       note = "deletes the vehicle and its peds (pools)")]
pub fn vehicle_destroy_vehicle_and_driver_and_passengers(veh: GPtr<u8>) {
    if !rd::<GPtr<u8>>(veh, 0x460).is_null() {
        remove_occupant(rd::<GPtr<u8>>(veh, 0x460));
    }
    let mut i = 0isize;
    while i < isize::from(rd::<u8>(veh, 0x488)) {
        let p = rd::<GPtr<u8>>(veh, 0x464 + 4 * i);
        if !p.is_null() {
            remove_occupant(p);
        }
        i += 1;
    }
    world_remove(veh);
    v_delete(veh, 1);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_BOAT_TELEPORT,
    HOOK_BIKE_TELEPORT,
    HOOK_VEHICLE_DESTROY_VEHICLE_AND_DRIVER_AND_PASSENGERS,
];
