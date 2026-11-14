//! Destructors of the collision events a car (or the player on foot) hitting peds raises (PEDHIT):
//! `CEventGotKnockedOverByCar` and `CEventPedCollisionWithPlayer` (`CEventVehicleCollision`'s 0x4AC920 is
//! FRAMELIMIT's, in `mvpb_ai`). Ported from the 1.0 US bytes. The SEH frames are not reproduced.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity** ref)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEventPedCollisionWithPed::~CEventPedCollisionWithPed()` (0x4ACA70)
    fn ped_collision_with_ped_dtor(e: GPtr<u8>) = thiscall 0x4ACA70 => unported;
}

/// The shared body: vtable `vt`, `CleanUpOldReference(&+0x14)` when the vehicle (+0x14) is set, `CEvent`'s vtable.
fn vehicle_event_dtor(this: GPtr<u8>, vt: u32) {
    wr::<u32>(this, 0, vt);
    let v = rd::<GPtr<u8>>(this, 0x14);
    if !v.is_null() {
        entity_clean_up_old_reference(v, this.byte_add(0x14));
    }
    wr::<u32>(this, 0, 0x0085_AB38);
}

/// `CEventGotKnockedOverByCar::~CEventGotKnockedOverByCar()` (0x4B1C10, thiscall): vtable 0x85B618.
#[hook(addr = 0x4B1C10, conv = thiscall, name = "FUN_004b1c10",
       expect = "6A FF 68 98 A1 83 00",
       unverified = "pedhit-1", route = "L2", hazards = "heap-writes", recs = "driving",
       note = "SEH frame not reproduced")]
pub fn event_knocked_over_by_car_dtor(this: GPtr<u8>) {
    vehicle_event_dtor(this, 0x0085_B618);
}

/// `CEventPedCollisionWithPlayer::~CEventPedCollisionWithPlayer()` (0x5FED80, thiscall): vtable 0x86C970, then a
/// tail jump into the base destructor `CEventPedCollisionWithPed::~CEventPedCollisionWithPed` 0x4ACA70 (the player
/// running into a ped on foot).
#[hook(addr = 0x5FED80, conv = thiscall, name = "FUN_005fed80",
       expect = "C7 01 70 C9 86 00 E9",
       unverified = "pedhit-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_ped_collision_with_player_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x0086_C970);
    ped_collision_with_ped_dtor(this);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] =
    &[HOOK_EVENT_KNOCKED_OVER_BY_CAR_DTOR, HOOK_EVENT_PED_COLLISION_WITH_PLAYER_DTOR];
