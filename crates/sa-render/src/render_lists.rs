//! Unnamed list and pool helpers (written by WRITER-MVP-D, batch `mvpd-unk-2`) on the in-world render path
//! (sorted node lists used by the visibility plugins, a 12-byte-object pool delete used by the car
//! environment-map pipeline). Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::common::{rd, wr, x};

/// Node links of the sorted lists of 0x733910/0x733F40: prev +0xC, next +0x10.
fn unlink(n: GPtr<u8>) {
    let next = rd::<GPtr<u8>>(n, 0x10);
    wr::<u32>(next, 0xC, rd::<u32>(n, 0xC));
    let prev = rd::<GPtr<u8>>(n, 0xC);
    wr::<u32>(prev, 0x10, rd::<u32>(n, 0x10));
}

/// Inserts `n` after `at` in the original's store order.
fn link_after(n: GPtr<u8>, at: GPtr<u8>) {
    wr::<u32>(n, 0x10, rd::<u32>(at, 0x10));
    wr::<u32>(rd::<GPtr<u8>>(at, 0x10), 0xC, n.addr() as u32);
    wr::<u32>(n, 0xC, at.addr() as u32);
    wr::<u32>(at, 0x10, n.addr() as u32);
}

/// 0x733F40 (`FUN_00733f40`, thiscall): moves every node of the used list (head sentinel +4, end
/// sentinel +0x14) to the front of the free list (head sentinel +0x28), one at a time.
#[hook(addr = 0x733F40, conv = thiscall, name = "FUN_00733f40", expect = "8B 41 10 56 8D 71 14",
       unverified = "mvpd-unk-2", route = "L2", hazards = "heap-writes", note = "relinks list nodes (heap)")]
pub fn sorted_list_clear(this: GPtr<u8>) {
    let end = this.byte_add(0x14);
    let free = this.byte_add(0x28);
    loop {
        let n = rd::<GPtr<u8>>(this, 0x10);
        if n == end {
            return;
        }
        unlink(n);
        link_after(n, free);
    }
}

/// 0x733910 (`FUN_00733910`, thiscall, `ret 4`): takes the first free node (free head +0x2C, end
/// +0x3C; none → NULL), copies the 3-dword `data` into it and links it into the used list before the
/// first node whose key (+8) is not below `data`'s (unordered keeps walking). Returns the node.
#[hook(addr = 0x733910, conv = thiscall, name = "FUN_00733910", expect = "8B 51 10 56 8B 74 24 08",
       unverified = "mvpd-unk-2", route = "L2", hazards = "heap-writes", note = "relinks list nodes (heap)")]
pub fn sorted_list_insert(this: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    let end = this.byte_add(0x14);
    let mut at = rd::<GPtr<u8>>(this, 0x10);
    while at != end && x(rd::<f32>(at, 8)).fcom(x(rd::<f32>(data, 8))).c0() {
        at = rd::<GPtr<u8>>(at, 0x10);
    }
    let n = rd::<GPtr<u8>>(this, 0x38);
    if n == this.byte_add(0x3C) {
        return GPtr::NULL;
    }
    for o in [0isize, 4, 8] {
        wr::<u32>(n, o, rd::<u32>(data, o));
    }
    unlink(n);
    link_after(n, rd::<GPtr<u8>>(at, 0xC));
    n
}

/// 0x5D9180 (`FUN_005d9180`, thiscall, `ret 4`, a 12-byte-object pool delete): index = `(ptr - base) / 12`
/// (signed); its flag byte (+4 array) gets bit 7; the first-free hint (+0xC) is lowered to it. Returns
/// the flag byte's address in EAX.
#[hook(addr = 0x5D9180, conv = thiscall, name = "FUN_005d9180", expect = "8B 54 24 04 53",
       unverified = "mvpd-unk-2", route = "L2", hazards = "heap-writes", note = "pool flag bytes (heap)")]
pub fn pool12_delete(this: GPtr<u8>, ptr: u32) -> u32 {
    let idx = (ptr.wrapping_sub(rd::<u32>(this, 0)) as i32) / 12;
    let f = rd::<GPtr<u8>>(this, 4).byte_add(idx as isize);
    f.write(f.read() | 0x80);
    if idx < rd::<i32>(this, 0xC) {
        wr::<i32>(this, 0xC, idx);
    }
    f.addr() as u32
}

pub const HOOKS: &[HookReg] = &[HOOK_SORTED_LIST_CLEAR, HOOK_SORTED_LIST_INSERT, HOOK_POOL12_DELETE];
