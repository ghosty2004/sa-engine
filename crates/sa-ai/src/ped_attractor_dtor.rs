//! `CPedAttractor`'s destructor, the complete destructors of nine derived classes (each stores its vtable and jumps
//! to the base destructor; `CPedTriggerScriptAttractor`'s 0x5EE880 is in `gen_dtors.rs`) and `HasEmptySlot` (batch
//! `std13-ai-1`). Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `operator delete(void*)` (0x8214BD)
    fn operator_delete(p: u32) = cdecl 0x8214BD => unported;
    fn destructor_o(this: GPtr<u8>) = thiscall 0x5EC410 => destructor;
}

/// `CPedAttractor::~CPedAttractor()` (0x5EC410, thiscall, SEH frame): vtable 0x86C538; `CleanUpOldReference(+8)` on
/// the entity when there is one; the storage of the pair vector (+0x30), the queue (+0x20) and the attracted list
/// (+0x10) is freed (`operator delete`) and each vector's first / last / end cleared.
#[hook(addr = 0x5EC410, conv = thiscall, name = "CPedAttractor::Destructor", expect = "6A FF 68 B1 DA 83 00",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "EAX on return is the last freed pointer or residue (callers ignore it)")]
pub fn destructor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C538);
    let e = rd::<GPtr<u8>>(this, 8);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(8));
    }
    for v in [0x30, 0x20, 0x10] {
        let p = rd::<u32>(this, v);
        if p != 0 {
            operator_delete(p);
        }
        wr::<u32>(this, v, 0);
        wr::<u32>(this, v + 4, 0);
        wr::<u32>(this, v + 8, 0);
    }
}

/// `FUN_005ee530` (0x5EE530, thiscall): the complete destructor of the attractor class with vtable 0x86C550 (vtable, then
/// `CPedAttractor::~CPedAttractor` as a tail jump).
#[hook(addr = 0x5EE530, conv = thiscall, name = "FUN_005ee530", expect = "C7 01 50 C5 86 00", unverified = "std13-ai-1", route = "L2",
       hazards = "heap-writes", recs = "grove-walk")]
pub fn atm_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C550);
    destructor_o(this);
}

/// `FUN_005ee600` (0x5EE600, thiscall): the complete destructor of the attractor class with vtable 0x86C568 (vtable, then
/// `CPedAttractor::~CPedAttractor` as a tail jump).
#[hook(addr = 0x5EE600, conv = thiscall, name = "FUN_005ee600", expect = "C7 01 68 C5 86 00", unverified = "std13-ai-1", route = "L2",
       hazards = "heap-writes", recs = "grove-walk")]
pub fn seat_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C568);
    destructor_o(this);
}

/// `FUN_005ee6d0` (0x5EE6D0, thiscall): the complete destructor of the attractor class with vtable 0x86C580 (vtable, then
/// `CPedAttractor::~CPedAttractor` as a tail jump).
#[hook(addr = 0x5EE6D0, conv = thiscall, name = "FUN_005ee6d0", expect = "C7 01 80 C5 86 00", unverified = "std13-ai-1", route = "L2",
       hazards = "heap-writes", recs = "grove-walk")]
pub fn stop_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C580);
    destructor_o(this);
}

/// `FUN_005ee7b0` (0x5EE7B0, thiscall): the complete destructor of the attractor class with vtable 0x86C598 (vtable, then
/// `CPedAttractor::~CPedAttractor` as a tail jump).
#[hook(addr = 0x5EE7B0, conv = thiscall, name = "FUN_005ee7b0", expect = "C7 01 98 C5 86 00", unverified = "std13-ai-1", route = "L2",
       hazards = "heap-writes", recs = "grove-walk")]
pub fn pizza_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C598);
    destructor_o(this);
}

/// `FUN_005ee950` (0x5EE950, thiscall): the complete destructor of the attractor class with vtable 0x86C5CC (vtable, then
/// `CPedAttractor::~CPedAttractor` as a tail jump).
#[hook(addr = 0x5EE950, conv = thiscall, name = "FUN_005ee950", expect = "C7 01 CC C5 86 00", unverified = "std13-ai-1", route = "L2",
       hazards = "heap-writes", recs = "grove-walk")]
pub fn shelter_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C5CC);
    destructor_o(this);
}

/// `FUN_005eea20` (0x5EEA20, thiscall): the complete destructor of the attractor class with vtable 0x86C5E4 (vtable, then
/// `CPedAttractor::~CPedAttractor` as a tail jump).
#[hook(addr = 0x5EEA20, conv = thiscall, name = "FUN_005eea20", expect = "C7 01 E4 C5 86 00", unverified = "std13-ai-1", route = "L2",
       hazards = "heap-writes", recs = "grove-walk")]
pub fn look_at_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C5E4);
    destructor_o(this);
}

/// `FUN_005eeaf0` (0x5EEAF0, thiscall): the complete destructor of the attractor class with vtable 0x86C5FC (vtable, then
/// `CPedAttractor::~CPedAttractor` as a tail jump).
#[hook(addr = 0x5EEAF0, conv = thiscall, name = "FUN_005eeaf0", expect = "C7 01 FC C5 86 00", unverified = "std13-ai-1", route = "L2",
       hazards = "heap-writes", recs = "grove-walk")]
pub fn scripted_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C5FC);
    destructor_o(this);
}

/// `FUN_005eebc0` (0x5EEBC0, thiscall): the complete destructor of the attractor class with vtable 0x86C614 (vtable, then
/// `CPedAttractor::~CPedAttractor` as a tail jump).
#[hook(addr = 0x5EEBC0, conv = thiscall, name = "FUN_005eebc0", expect = "C7 01 14 C6 86 00", unverified = "std13-ai-1", route = "L2",
       hazards = "heap-writes", recs = "grove-walk")]
pub fn park_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C614);
    destructor_o(this);
}

/// `FUN_005eec90` (0x5EEC90, thiscall): the complete destructor of the attractor class with vtable 0x86C62C (vtable, then
/// `CPedAttractor::~CPedAttractor` as a tail jump).
#[hook(addr = 0x5EEC90, conv = thiscall, name = "FUN_005eec90", expect = "C7 01 2C C6 86 00", unverified = "std13-ai-1", route = "L2",
       hazards = "heap-writes", recs = "grove-walk")]
pub fn step_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C62C);
    destructor_o(this);
}

/// `CPedAttractor::HasEmptySlot()` (0x5EAF10, thiscall): AL = queue + attracted < maximum (+0x3C, signed).
#[hook(addr = 0x5EAF10, conv = thiscall, name = "CPedAttractor::HasEmptySlot", expect = "8B 41 20 85 C0 75 04",
       unverified = "std13-ai-1", route = "L2", hazards = "ax-al-return", recs = "grove-walk",
       note = "EAX's upper bytes: the registered count's")]
pub fn has_empty_slot(this: GPtr<u8>) -> u8 {
    let n = |first: isize| {
        let f = rd::<u32>(this, first);
        if f == 0 { 0 } else { (rd::<u32>(this, first + 4).wrapping_sub(f) as i32) >> 2 }
    };
    u8::from(n(0x20).wrapping_add(n(0x10)) < rd::<i32>(this, 0x3C))
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_DESTRUCTOR,
    HOOK_ATM_DTOR,
    HOOK_SEAT_DTOR,
    HOOK_STOP_DTOR,
    HOOK_PIZZA_DTOR,
    HOOK_SHELTER_DTOR,
    HOOK_LOOK_AT_DTOR,
    HOOK_SCRIPTED_DTOR,
    HOOK_PARK_DTOR,
    HOOK_STEP_DTOR,
    HOOK_HAS_EMPTY_SLOT,
];
