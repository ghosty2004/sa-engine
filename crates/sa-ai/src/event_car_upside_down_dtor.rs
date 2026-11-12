//! STD-15 (batch `std15-ai-4`): the `CEventCarUpsideDown` destructor body (0x4B1D50), reached when the player's car
//! rolls over. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571B70)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// `CEventCarUpsideDown::~CEventCarUpsideDown` body (0x4B1D50, thiscall; SEH frame): vtable 0x85B660, the
/// vehicle (+0xC) drops its registered reference to the slot, then the `CEvent` vtable 0x85AB38.
#[hook(addr = 0x4B1D50, conv = thiscall, name = "FUN_004b1d50", expect = "6A FF 68 D8 A1 83 00",
       unverified = "std15-ai-4", route = "L2", hazards = "none", recs = "car-drive")]
pub fn event_car_upside_down_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85B660);
    let v = rd::<GPtr<u8>>(this, 0xC);
    if !v.is_null() {
        entity_clean_up_old_reference(v, this.byte_add(0xC));
    }
    wr::<u32>(this, 0, 0x85AB38);
}

pub const HOOKS: &[HookReg] = &[HOOK_EVENT_CAR_UPSIDE_DOWN_DTOR];

orig! {
    /// `CEventEditableResponse::~CEventEditableResponse` body (0x4AC480, thiscall)
    fn event_editable_response_dtor_body(this: GPtr<u8>) = thiscall 0x4AC480 => unported;
}

/// The destructor body 0x4B83C0 (thiscall; an event raised when the player leaves a car): vtable 0x85B898, then a
/// tail jump to `CEventEditableResponse`'s body 0x4AC480.
#[hook(addr = 0x4B83C0, conv = thiscall, name = "FUN_004b83c0", expect = "C7 01 98 B8 85 00 E9",
       unverified = "std15-ai-4", route = "L2", hazards = "none", recs = "car-enter")]
pub fn event_dtor_body_4b83c0(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85B898);
    event_editable_response_dtor_body(this);
}

pub const HOOKS_2: &[HookReg] = &[HOOK_EVENT_DTOR_BODY_4B83C0];
