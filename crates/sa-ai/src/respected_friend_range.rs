//! `CPedIntelligence::FindRespectedFriendInInformRange()` (0x600CF0) and the complete destructor 0x420F30 of an event
//! class (vtable 0x858E68) (batch `std13-ai-1`). Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CAcquaintance::GetAcquaintances(int type)` (0x608970, `ret 4`)
    fn acquaintance_get(a: GPtr<u8>, t: u32) -> u32 = thiscall 0x608970 => unported;
    /// `CPedType::GetPedFlag(ePedType)` (0x608830)
    fn ped_type_flag(t: u32) -> u32 = cdecl 0x608830 => unported;
    /// the base event destructor (0x4AF890)
    fn event_base_dtor(e: GPtr<u8>) = thiscall 0x4AF890 => unported;
}

/// The position of an entity: its matrix's (+0x30) or its placeable's (+4).
fn pos_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CPedIntelligence::FindRespectedFriendInInformRange()` (0x600CF0, thiscall). mask = the ped's respect
/// acquaintances (`GetAcquaintances(0)` of ped +0x4E0); over the scanned peds (+0x130, count +0xC4, at most 16) the
/// first with `GetPedFlag(its type +0x598) & mask` and a squared distance below the inform range (+0xC8) squared
/// (x87: `range * range > dx*dx + dz*dz + dy*dy`, the sum in that order) gives AL = 1; else AL = 0.
#[hook(addr = 0x600CF0, conv = thiscall, name = "CPedIntelligence::FindRespectedFriendInInformRange",
       expect = "53 55 56 57 8B F9 8B 0F", unverified = "std13-ai-1", route = "L2", hazards = "ax-al-return",
       recs = "grove-walk", note = "EAX's upper bytes: the FPU status word on the hit path")]
pub fn find_respected_friend_in_inform_range(this: GPtr<u8>) -> u8 {
    let ped = rd::<GPtr<u8>>(this, 0);
    let mask = acquaintance_get(ped.byte_add(0x4E0), 0);
    let mut i = 0i32;
    loop {
        if i >= rd::<i32>(this, 0xC4) {
            return 0;
        }
        let e = rd::<GPtr<u8>>(this, 0x130 + 4 * i as isize);
        i += 1;
        if !e.is_null() && mask & ped_type_flag(rd::<u32>(e, 0x598)) != 0 {
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
    event_base_dtor(this);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_FIND_RESPECTED_FRIEND_IN_INFORM_RANGE, HOOK_EVENT_DTOR_420F30];
