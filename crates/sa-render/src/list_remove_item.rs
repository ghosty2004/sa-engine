//! `List_c::RemoveItem` (batch `std8-render-1`), ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::common::{rd, wr};

/// `List_c::RemoveItem(ListItem_c* item)` (0x4A8E30, thiscall `ret 4`; `List_c`: +0 last, +4 first, +8 count;
/// `ListItem_c`: +0 next, +4 previous): unlinks `item` (the previous item's next, or the list's first, becomes
/// `item->next`; the next item's previous, or the list's last, becomes `item->prev`) and decrements the count. The
/// item's own links are left as they are.
#[hook(addr = 0x4A8E30, conv = thiscall, name = "FUN_004a8e30", expect = "56 8B 74 24 08 8B 56 04",
       unverified = "std8-render-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn list_remove_item(list: GPtr<u8>, item: GPtr<u8>) {
    let prev = rd::<GPtr<u8>>(item, 4);
    let next = rd::<GPtr<u8>>(item, 0);
    if prev.is_null() {
        wr::<GPtr<u8>>(list, 4, next);
    } else {
        wr::<GPtr<u8>>(prev, 0, next);
    }
    if next.is_null() {
        wr::<GPtr<u8>>(list, 0, rd::<GPtr<u8>>(item, 4));
    } else {
        wr::<GPtr<u8>>(next, 4, prev);
    }
    wr::<u32>(list, 8, rd::<u32>(list, 8).wrapping_sub(1));
}

pub const HOOKS: &[HookReg] = &[HOOK_LIST_REMOVE_ITEM];
