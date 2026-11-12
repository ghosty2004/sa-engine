//! Two event constructors the real-time walk reaches (batch `std13-ai-1`): `CEventVehicleDamage` (0x4B18D0) and the
//! event with vtable 0x86C6D0 (0x5F6190, catalogue `FUN_005f6190`). Ported from the 1.0 US bytes. Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CEvent::CEvent()` (0x4ABFC0)
    fn event_ctor(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ABFC0 => unported;
}

/// `CEventVehicleDamage::CEventVehicleDamage(CVehicle*, CEntity* attacker, eWeaponType)` (0x4B18D0, thiscall, `ret
/// 0xC`; entry `push -1; jmp 0x407A45`, the protection fragment pushes the SEH handler). The `CEventEditableResponse`
/// part inlined: +4 = 0, +8 = 0, +0xC = 1, +0xE = 200, +0x10 = -1; vtable 0x85B588; +0x14 vehicle, +0x18 attacker
/// (each a registered reference when non-null), +0x1C weapon type.
#[hook(addr = 0x4B18D0, conv = thiscall, name = "CEventVehicleDamage::Constructor", expect = "6A FF E9 6E 61 F5 FF",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "entry `push -1; jmp` into the protection fragment 0x407A45")]
pub fn event_vehicle_damage_ctor(this: GPtr<u8>, vehicle: GPtr<u8>, attacker: GPtr<u8>, weapon: u32) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u8>(this, 8, 0);
    wr::<u8>(this, 0xC, 1);
    wr::<u16>(this, 0xE, 200);
    wr::<u16>(this, 0x10, 0xFFFF);
    wr::<GPtr<u8>>(this, 0x18, attacker);
    wr::<u32>(this, 0, 0x85B588);
    wr::<GPtr<u8>>(this, 0x14, vehicle);
    wr::<u32>(this, 0x1C, weapon);
    if !vehicle.is_null() {
        entity_register_reference(vehicle, this.byte_add(0x14));
    }
    let a = rd::<GPtr<u8>>(this, 0x18);
    if !a.is_null() {
        entity_register_reference(a, this.byte_add(0x18));
    }
    this
}

/// `FUN_005f6190` (0x5F6190, thiscall, `ret 0x10`, SEH frame): `CEvent()`, +0xC = the first argument, +0x10 = the
/// entity (a registered reference when non-null), +0x14 = the third argument's low byte, +0x18 = the fourth; vtable
/// 0x86C6D0.
#[hook(addr = 0x5F6190, conv = thiscall, name = "FUN_005f6190", expect = "6A FF 68 18 DE 83 00",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_5f6190_ctor(this: GPtr<u8>, a: u32, entity: GPtr<u8>, flag: u32, d: u32) -> GPtr<u8> {
    event_ctor(this);
    wr::<u32>(this, 0xC, a);
    wr::<u8>(this, 0x14, flag as u8);
    wr::<u32>(this, 0, 0x86C6D0);
    wr::<GPtr<u8>>(this, 0x10, entity);
    wr::<u32>(this, 0x18, d);
    if !entity.is_null() {
        entity_register_reference(entity, this.byte_add(0x10));
    }
    this
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_VEHICLE_DAMAGE_CTOR, HOOK_EVENT_5F6190_CTOR];
