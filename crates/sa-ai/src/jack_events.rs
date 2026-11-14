//! Event constructors and destructors a car-jack reaches (batch `carjack-1`): `CEventPedEnteredMyVehicle` (the
//! jacked driver's event) and the `CEventSeenPanickedPed` destructor (bystanders; the `CEventVehicleCollision`
//! one, 0x4AC920, is FRAMELIMIT's in mvpb_ai.rs). Ported from the 1.0 US bytes.
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
}

/// `CEventPedEnteredMyVehicle::CEventPedEnteredMyVehicle(CPed*, CVehicle*, eTargetDoor)` (0x4AEAC0, catalogue
/// `FUN_004aeac0`, `ret 0xC`, SEH frame; vtable 0x85AFE8, event type 0x11): the `CEventEditableResponse` part inlined
/// (+4 = 0, +8 = 0, +0xC = 1, +0xE = 200, +0x10 = -1), +0x18 vehicle, +0x14 ped, +0x1C door; both registered as
/// references without a NULL check (as the original).
#[hook(addr = 0x4AEAC0, conv = thiscall, name = "FUN_004aeac0", expect = "6A FF 68 C8 9C 83 00",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn event_ped_entered_my_vehicle_ctor(this: GPtr<u8>, ped: GPtr<u8>, veh: GPtr<u8>, door: u32) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u8>(this, 0xC, 1);
    wr::<u16>(this, 0xE, 200);
    wr::<u16>(this, 0x10, 0xFFFF);
    wr::<GPtr<u8>>(this, 0x18, veh);
    wr::<u32>(this, 0, 0x85AFE8);
    wr::<GPtr<u8>>(this, 0x14, ped);
    wr::<u32>(this, 0x1C, door);
    entity_register_reference(ped, this.byte_add(0x14));
    entity_register_reference(rd::<GPtr<u8>>(this, 0x18), this.byte_add(0x18));
    this
}

/// `CEventPedEnteredMyVehicle::~CEventPedEnteredMyVehicle()` (0x4AEB90, catalogue `FUN_004aeb90`, SEH frame): vtable
/// 0x85AFE8, `CleanUpOldReference` of the ped (+0x14) and the vehicle (+0x18) when set, then the `CEvent` vtable
/// 0x85AB38 (its destructor inlined).
#[hook(addr = 0x4AEB90, conv = thiscall, name = "FUN_004aeb90", expect = "6A FF 68 E8 9C 83 00",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn event_ped_entered_my_vehicle_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85AFE8);
    for off in [0x14, 0x18] {
        let e = rd::<GPtr<u8>>(this, off);
        if !e.is_null() {
            entity_clean_up_old_reference(e, this.byte_add(off));
        }
    }
    wr::<u32>(this, 0, 0x85AB38);
}

/// The shape of the one-reference event destructors: vtable `vt`, `CleanUpOldReference` of the entity at +0x14 when
/// set, then the `CEvent` vtable 0x85AB38 (its destructor inlined).
fn one_ref_event_dtor(this: GPtr<u8>, vt: u32) {
    wr::<u32>(this, 0, vt);
    let e = rd::<GPtr<u8>>(this, 0x14);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(0x14));
    }
    wr::<u32>(this, 0, 0x85AB38);
}

/// `CEventSeenPanickedPed::~CEventSeenPanickedPed()` (0x4B2130, catalogue `FUN_004b2130`; entry `push -1; jmp` into
/// the protection fragment 0x40A0CB, SEH frame): [`one_ref_event_dtor`] with vtable 0x85B778 (the ped at +0x14).
#[hook(addr = 0x4B2130, conv = thiscall, name = "FUN_004b2130", expect = "6A FF E9 94 7F F5 FF",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack",
       note = "entry `push -1; jmp` into the protection fragment 0x40A0CB")]
pub fn event_seen_panicked_ped_dtor(this: GPtr<u8>) {
    one_ref_event_dtor(this, 0x85B778);
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_EVENT_PED_ENTERED_MY_VEHICLE_CTOR,
    HOOK_EVENT_PED_ENTERED_MY_VEHICLE_DTOR,
    HOOK_EVENT_SEEN_PANICKED_PED_DTOR,
];
