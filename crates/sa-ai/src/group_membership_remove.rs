//! `CPedGroupMembership::RemoveMember(CPed*)` (0x5FB210, batch `std12-ai-1`), reached when an ambient group member
//! leaves its group while CJ walks around Grove Street. Ported from the 1.0 US bytes. Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPedGroupMembership::RemoveMember(int32 memIdx)` (0x5F80D0)
    fn remove_member_by_idx(m: GPtr<u8>, idx: i32) = thiscall 0x5F80D0 => unported;
}

/// `CPedGroupMembership::RemoveMember(CPed* ped)` (0x5FB210, thiscall, `ret 4`): the index of `ped` among the
/// eight member slots (+4); when found, a tail jump to `RemoveMember(index)` with the argument slot replaced by the
/// index. Not found: nothing (EAX = 8, unread by the callers).
#[hook(addr = 0x5FB210, conv = thiscall, name = "CPedGroupMembership::RemoveMember-ByPed", expect = "56 8B 74 24 08 33 C0",
       unverified = "std12-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn remove_member(this: GPtr<u8>, ped: GPtr<u8>) {
    if let Some(i) = (0..8).find(|&i| rd::<GPtr<u8>>(this, 4 + 4 * i) == ped) {
        remove_member_by_idx(this, i as i32);
    }
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_REMOVE_MEMBER];
