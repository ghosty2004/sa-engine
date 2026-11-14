//! Two small functions the longer Grove Street walk reaches (batch `attr-6`): `CTaskSimpleDoHandSignal::Clone`
//! (0x6608C0) and the `CEventVehicleOnFire` destructor (0x4B1170, catalogue `FUN_004b1170`). Ported from the 1.0 US
//! bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

use sa_core::bindings::entity_clean_up_old_reference;
use sa_world::bindings::task_simple_ctor;

/// `CTaskSimpleDoHandSignal::Clone()` (0x6608C0, thiscall, SEH frame): a new 0x0C-byte task (vtable 0x86FA68, +8 = 0),
/// then +8 = the source's byte +8; the copy writes through a null result too when the pool is full, as in the
/// original.
#[hook(addr = 0x6608C0, conv = thiscall, name = "CTaskSimpleDoHandSignal::Clone", expect = "6A FF 68 8B 46 84 00",
       unverified = "attr-6", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn hand_signal_clone(this: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0xC);
    if !t.is_null() {
        task_simple_ctor(t);
        wr::<u32>(t, 0, 0x86FA68);
        wr::<u8>(t, 8, 0);
    }
    wr::<u8>(t, 8, rd::<u8>(this, 8));
    t
}

/// `CEventVehicleOnFire::~CEventVehicleOnFire()` (0x4B1170, thiscall, SEH frame): vtable 0x85B440,
/// `CleanUpOldReference` on the vehicle (+0x14) when set, then the `CEvent` vtable 0x85AB38.
#[hook(addr = 0x4B1170, conv = thiscall, name = "FUN_004b1170", expect = "6A FF 68 98 A0 83 00",
       unverified = "attr-6", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_vehicle_on_fire_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85B440);
    let v = rd::<GPtr<u8>>(this, 0x14);
    if !v.is_null() {
        entity_clean_up_old_reference(v, GPtr::<u8>::cast::<GPtr<u8>>(this.byte_add(0x14)));
    }
    wr::<u32>(this, 0, 0x85AB38);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_HAND_SIGNAL_CLONE, HOOK_EVENT_VEHICLE_ON_FIRE_DTOR];
