//! STD-5: `CPlaceable::AllocateStaticMatrix` (0x54F4C0) and `CMatrixLinkList::AddToList2` (0x54EA10),
//! ported from the 1.0 US bytes.
//!
//! `CMatrixLink` (0x54): the matrix, +0x48 owner, +0x4C prev, +0x50 next. `gMatrixList` (0xB74288): +0
//! the used list head (0xB74288..), +0xA8 the static list head (0xB74330), +0x150 the free list head
//! (0xB743D8; its next at +0x1A0 = 0xB74428), +0x1A4 the free list end (0xB7442C).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CPlaceable::RemoveMatrix()` (0x54F3B0)
    fn remove_matrix(this: GPtr<u8>) = thiscall 0x54F3B0 => unported;
    /// `CMatrixLinkList::AddToList2()` (0x54EA10)
    fn add_to_list2_call(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x54EA10 => add_to_list2;
}

fn rd(p: GPtr<u8>, o: isize) -> u32 {
    p.byte_add(o).cast::<u32>().read()
}
fn wr(p: GPtr<u8>, o: isize, v: u32) {
    p.byte_add(o).cast::<u32>().write(v)
}
fn p8(a: u32) -> GPtr<u8> {
    GPtr::from_addr(a as usize)
}
fn addr(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// Unlinks `node` and inserts it after `head`.
fn move_after(node: GPtr<u8>, head: GPtr<u8>) {
    wr(p8(rd(node, 0x50)), 0x4C, rd(node, 0x4C));
    wr(p8(rd(node, 0x4C)), 0x50, rd(node, 0x50));
    wr(node, 0x50, rd(head, 0x50));
    wr(p8(rd(head, 0x50)), 0x4C, addr(node));
    wr(node, 0x4C, addr(head));
    wr(head, 0x50, addr(node));
}

/// The static list head.
const STATIC_HEAD: u32 = 0xB74330;

/// `CPlaceable::AllocateStaticMatrix()` (0x54F4C0): an existing matrix link moves to the front of the
/// static list; without one, the first free link does (owner = this, `this->m_matrix` = it), and with
/// no free link the last used link's owner loses its matrix (`RemoveMatrix`) and `AddToList2` provides
/// one.
#[hook(addr = 0x54F4C0, conv = thiscall, name = "CPlaceable::AllocateStaticMatrix", expect = "56 8B F1 8B 46 14 85 C0 74 32",
       unverified = "std5-world-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn allocate_static_matrix(this: GPtr<u8>) {
    let m = rd(this, 0x14);
    if m != 0 {
        move_after(p8(m), p8(STATIC_HEAD));
        return;
    }
    let first_free = GPtr::<u32>::from_addr(0xB74428).read();
    let node = if first_free != 0xB7442C {
        move_after(p8(first_free), p8(STATIC_HEAD));
        p8(first_free)
    } else {
        let last = p8(GPtr::<u32>::from_addr(0xB74328).read());
        remove_matrix(p8(rd(last, 0x48)));
        add_to_list2_call(p8(0xB74288))
    };
    wr(node, 0x48, addr(this));
    wr(this, 0x14, addr(node));
}

/// `CMatrixLinkList::AddToList2()` (0x54EA10): the first free link (`this + 0x1A0`, unless it is the end
/// `this + 0x1A4`: NULL) moves to the front of the static list (`this + 0xA8`) and is returned.
#[hook(addr = 0x54EA10, conv = thiscall, name = "CMatrixLinkList::AddToList2", expect = "8B 81 A0 01 00 00 8D 91 A4 01",
       unverified = "std5-world-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn add_to_list2(this: GPtr<u8>) -> GPtr<u8> {
    let node = rd(this, 0x1A0);
    if node == addr(this) + 0x1A4 {
        return p8(0);
    }
    move_after(p8(node), this.byte_add(0xA8));
    p8(node)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_ALLOCATE_STATIC_MATRIX, HOOK_ADD_TO_LIST2];
