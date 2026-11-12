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

pub const HOOKS: &[HookReg] = &[HOOK_SORTED_LIST_CLEAR, HOOK_SORTED_LIST_INSERT, HOOK_POOL12_DELETE, HOOK_INSERT_ENTITY_INTO_SORTED_LIST];

sa_core::orig! {
    /// 0x733910 (the sorted-list insert above, thiscall)
    fn call_sorted_list_insert(this: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> = thiscall 0x733910 => sorted_list_insert;
}

/// `CVisibilityPlugins::m_alphaReallyDrawLastList` (0xC881D0), `m_alphaUnderwaterEntityList` (0xC88178),
/// `m_alphaEntityList` (0xC88120)
const ALPHA_REALLY_DRAW_LAST_LIST: usize = 0xC881D0;
const ALPHA_UNDERWATER_ENTITY_LIST: usize = 0xC88178;
const ALPHA_ENTITY_LIST: usize = 0xC88120;
/// `CVisibilityPlugins::RenderEntity` (0x732B40), the render callback stored with each entry
const RENDER_ENTITY: u32 = 0x732B40;
/// `MI_GRASSHOUSE` (int16 model index at 0x8CD6F4)
const MI_GRASSHOUSE: usize = 0x8CD6F4;

/// `CVisibilityPlugins::InsertEntityIntoSortedList(CEntity* e, float dist)` (0x734570, cdecl, AL; the entry jumps
/// through the protection chunk 0x401D56, which loads `MI_GRASSHOUSE`): the entry `{e, RenderEntity, dist}`; the
/// grass house model first goes into the really-draw-last list (a free node: true); an underwater entity (+0x1C bit
/// 20) into the underwater list, any other into the alpha entity list; true when a node was free.
#[hook(addr = 0x734570, conv = cdecl, name = "CVisibilityPlugins::InsertEntityIntoSortedList", expect = "83 EC 18 8B 4C 24 20",
       unverified = "std9-render-1", route = "L2", hazards = "heap-writes,ax-al-return", recs = "grove-walk",
       note = "AL only: EAX's upper bytes are the node pointer's in the original")]
pub fn insert_entity_into_sorted_list(e: GPtr<u8>, dist: u32) -> u8 {
    let mut item = [e.addr() as u32, RENDER_ENTITY, dist];
    let model = i32::from(rd::<i16>(e, 0x22));
    if model == i32::from(GPtr::<u16>::from_addr(MI_GRASSHOUSE).read()) {
        let mut last = [e.addr() as u32, RENDER_ENTITY, dist];
        if !call_sorted_list_insert(GPtr::from_addr(ALPHA_REALLY_DRAW_LAST_LIST), GPtr::from_raw(last.as_mut_ptr()).cast()).is_null() {
            return 1;
        }
    }
    let list = if rd::<u32>(e, 0x1C) & 0x10_0000 != 0 { ALPHA_UNDERWATER_ENTITY_LIST } else { ALPHA_ENTITY_LIST };
    u8::from(!call_sorted_list_insert(GPtr::from_addr(list), GPtr::from_raw(item.as_mut_ptr()).cast()).is_null())
}
