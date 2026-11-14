//! `CPedIntelligence::FindRespectedFriendInInformRange()` (0x600CF0), the complete destructor 0x420F30 of an event
//! class (vtable 0x858E68) and `CEventPotentialWalkIntoObject`'s destructor (0x4AE670) (batch `std13-ai-1`). Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::mem::GPtr;
use sa_core::abi::HookReg;
use sa_core::hook;

use crate::common::*;

use sa_world::bindings::entity_clean_up_old_reference;
use sa_world::bindings::event_acquaintance_ped_dtor as event_base_dtor;
use sa_world::bindings::get_ped_flag as ped_type_flag;
use crate::bindings::acquaintance_get;

use sa_world::world::common::entity_pos as pos_of;

/// `CPedIntelligence::FindRespectedFriendInInformRange()` (0x600CF0, thiscall). mask = the ped's respect
/// acquaintances (`GetAcquaintances(0)` of ped +0x4E0); over the scanned peds (+0x130, count +0xC4, at most 16) the
/// first with `GetPedFlag(its type +0x598) & mask` and a squared distance below the inform range (+0xC8) squared
/// (x87: `range * range > dx*dx + dz*dz + dy*dy`, the sum in that order) gives AL = 1; else AL = 0.
#[hook(addr = 0x600CF0, conv = thiscall, name = "CPedIntelligence::FindRespectedFriendInInformRange",
       expect = "53 55 56 57 8B F9 8B 0F", unverified = "std13-ai-1", route = "L2", hazards = "ax-al-return",
       recs = "grove-walk", note = "EAX's upper bytes: the FPU status word on the hit path")]
pub fn find_respected_friend_in_inform_range(this: GPtr<u8>) -> u8 {
    let ped = rd::<GPtr<u8>>(this, 0);
    let mask = acquaintance_get(GPtr::<u8>::addr(ped.byte_add(0x4E0)) as u32, 0);
    let mut i = 0i32;
    loop {
        if i >= rd::<i32>(this, 0xC4) {
            return 0;
        }
        let e = rd::<GPtr<u8>>(this, 0x130 + 4 * i as isize);
        i += 1;
        if !e.is_null() && mask & ped_type_flag(rd::<u32>(e, 0x598) as i32) != 0 {
            let b = pos_of(e);
            let a = pos_of(rd::<GPtr<u8>>(this, 0));
            let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0));
            let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4));
            let dz = x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(b, 8));
            let r = x(rd::<f32>(this, 0xC8));
            let s = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
            let c = r.fmul(r).fcom(s);
            if !c.test_any(0x41) {
                return 1;
            }
        }
        if i >= 16 {
            return 0;
        }
    }
}

/// `FUN_00420f30` (0x420F30, thiscall): the complete destructor of the event class with vtable 0x858E68 (vtable,
/// then the base destructor 0x4AF890 as a tail jump).
#[hook(addr = 0x420F30, conv = thiscall, name = "FUN_00420f30", expect = "C7 01 68 8E 85 00", unverified = "std13-ai-1",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_dtor_420f30(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x858E68);
    event_base_dtor(this.cast::<u32>());
}

/// `CEventPotentialWalkIntoObject::~CEventPotentialWalkIntoObject()` (0x4AE670, thiscall, SEH frame; catalogue
/// `FUN_004ae670`): vtable 0x85AF18, `CleanUpOldReference(+0x14)` on the object when there is one, then the `CEvent`
/// vtable 0x85AB38 (the base destructor inlined).
#[hook(addr = 0x4AE670, conv = thiscall, name = "FUN_004ae670", expect = "6A FF 68 28 9C 83 00",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn walk_into_object_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85AF18);
    let e = rd::<GPtr<u8>>(this, 0x14);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(0x14));
    }
    wr::<u32>(this, 0, 0x85AB38);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] =
    &[HOOK_FIND_RESPECTED_FRIEND_IN_INFORM_RANGE, HOOK_EVENT_DTOR_420F30, HOOK_WALK_INTO_OBJECT_DTOR];
