//! WRITER-NATIVE (batch `native-25`): the `CEventVehicleCollision` constructor (0x4AC840), raised when CJ runs
//! into a car (collision scan). Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
}

/// `CEventVehicleCollision::CEventVehicleCollision(int16 pieceType, float damageIntensity, CVehicle* vehicle, const
/// CVector& collisionImpactVelocity, const CVector& collisionPos, int8 moveState, int16 evadeType)` (0x4AC840,
/// `ret 0x1C`, SEH frame): `CEvent` fields +4 = 0, +8 = 0; +0xC = piece type (word), +0xE = evade type (word),
/// +0x10 = damage intensity, vtable 0x85AC28, +0x14 = vehicle (registered reference), +0x18 = impact velocity,
/// +0x24 = position, +0x30 = move state, +0x31 = 0. Returns `this`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x4AC840, conv = thiscall, name = "CEventVehicleCollision::Constructor",
       expect = "6A FF 68 88 99 83 00",
       unverified = "native-25", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "raised from the ped collision scan when the player runs into a car")]
pub fn event_vehicle_collision_ctor(
    this: GPtr<u8>,
    piece: u32,
    damage: u32,
    veh: GPtr<u8>,
    velocity: GPtr<u8>,
    position: GPtr<u8>,
    move_state: u32,
    evade: u32,
) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u16>(this, 0xC, piece as u16);
    wr::<u16>(this, 0xE, evade as u16);
    wr::<u32>(this, 0x10, damage);
    wr::<u32>(this, 0, 0x85_AC28);
    wr::<GPtr<u8>>(this, 0x14, veh);
    for o in [0, 4, 8] {
        wr::<u32>(this, 0x18 + o, rd::<u32>(velocity, o));
    }
    for o in [0, 4, 8] {
        wr::<u32>(this, 0x24 + o, rd::<u32>(position, o));
    }
    wr::<u8>(this, 0x30, move_state as u8);
    wr::<u8>(this, 0x31, 0);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(0x14));
    }
    this
}

/// The event destructor body 0x4AE270 (SEH frame, no `delete`) that frees the vehicle-collision events on the
/// collision path: vtable 0x85AE88, `CleanUpOldReference(&+0x14)` with a vehicle, then the `CEvent` vtable
/// 0x85AB38.
#[hook(addr = 0x4AE270, conv = thiscall, name = "FUN_004ae270",
       expect = "6A FF 68 A8 9B 83 00",
       unverified = "native-25", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_vehicle_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85_AE88);
    let v = rd::<GPtr<u8>>(this, 0x14);
    if !v.is_null() {
        entity_clean_up_old_reference(v, this.byte_add(0x14));
    }
    wr::<u32>(this, 0, 0x85_AB38);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_VEHICLE_COLLISION_CTOR, HOOK_EVENT_VEHICLE_DTOR];
