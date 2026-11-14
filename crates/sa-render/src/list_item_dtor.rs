//! `ListItem_c::~ListItem_c` (batch `mvpm-render-1`): the FX list-item destructor entry, a lone tail `jmp` to the
//! empty base destructor. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::hook;

use sa_core::bindings::list_item_dtor as list_item_base_dtor;

/// `ListItem_c::~ListItem_c()` (0x49EA70, thiscall): `jmp 0x4A8DC0` (ECX passes through unchanged).
#[hook(addr = 0x49ea70, conv = thiscall, name = "ListItem_c::~ListItem_c", expect = "E9 4B A3 00 00 90",
       route = "L2", hazards = "none")]
pub fn list_item_dtor(this: GPtr<u8>) {
    list_item_base_dtor(this);
}

pub const HOOKS: &[HookReg] = &[HOOK_LIST_ITEM_DTOR];
