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

/// The complete destructors: `mov dword [ecx], VTABLE; jmp CPedAttractor::~CPedAttractor`.
macro_rules! derived_dtor {
    ($fn:ident, $addr:literal, $name:literal, $vt:literal, $expect:literal) => {
        #[doc = concat!("`", $name, "` (", stringify!($addr), ", thiscall): vtable ", stringify!($vt),
                        ", then `CPedAttractor::~CPedAttractor` (tail jump).")]
        #[hook(addr = $addr, conv = thiscall, name = $name, expect = $expect, unverified = "std13-ai-1", route = "L2",
               hazards = "heap-writes", recs = "grove-walk")]
        pub fn $fn(this: GPtr<u8>) {
            wr::<u32>(this, 0, $vt);
            destructor_o(this);
        }
    };
}

derived_dtor!(atm_dtor, 0x5EE530, "FUN_005ee530", 0x86C550, "C7 01 50 C5 86 00");
derived_dtor!(seat_dtor, 0x5EE600, "FUN_005ee600", 0x86C568, "C7 01 68 C5 86 00");
derived_dtor!(stop_dtor, 0x5EE6D0, "FUN_005ee6d0", 0x86C580, "C7 01 80 C5 86 00");
derived_dtor!(pizza_dtor, 0x5EE7B0, "FUN_005ee7b0", 0x86C598, "C7 01 98 C5 86 00");
derived_dtor!(shelter_dtor, 0x5EE950, "FUN_005ee950", 0x86C5CC, "C7 01 CC C5 86 00");
derived_dtor!(look_at_dtor, 0x5EEA20, "FUN_005eea20", 0x86C5E4, "C7 01 E4 C5 86 00");
derived_dtor!(scripted_dtor, 0x5EEAF0, "FUN_005eeaf0", 0x86C5FC, "C7 01 FC C5 86 00");
derived_dtor!(park_dtor, 0x5EEBC0, "FUN_005eebc0", 0x86C614, "C7 01 14 C6 86 00");
derived_dtor!(step_dtor, 0x5EEC90, "FUN_005eec90", 0x86C62C, "C7 01 2C C6 86 00");

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
