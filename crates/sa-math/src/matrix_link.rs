//! `CMatrixLink` / `CMatrixLinkList` (the matrix pool), ported from the 1.0 US machine code.
//!
//! These functions only move pointers. They are ported on a 32-bit memory model ([`Mem32`]) with the
//! original's addresses and its exact load/store order, so they behave like the original for any
//! pointer graph (aliasing and corrupt lists included).

/// Dword memory in the game's address space.
pub trait Mem32 {
    fn rd(&self, addr: u32) -> u32;
    fn wr(&mut self, addr: u32, v: u32);
}

/// `CMatrixLink`: a `CMatrix` (0x48) + owner (0x48) + prev (0x4C) + next (0x50) = 0x54 bytes.
pub const LINK_SIZE: u32 = 0x54;
pub const LINK_OWNER: u32 = 0x48;
pub const LINK_PREV: u32 = 0x4C;
pub const LINK_NEXT: u32 = 0x50;

/// `CMatrixLinkList` layout (0x1FC bytes): six sentinel links and the object array.
pub const LIST_HEAD: u32 = 0x0;
pub const LIST_TAIL: u32 = 0x54;
pub const LIST_ALLOC_HEAD: u32 = 0xA8;
pub const LIST_ALLOC_TAIL: u32 = 0xFC;
pub const LIST_FREE_HEAD: u32 = 0x150;
pub const LIST_FREE_TAIL: u32 = 0x1A4;
pub const LIST_OBJECTS: u32 = 0x1F8;
pub const LIST_SIZE: u32 = 0x1FC;

/// `CMatrixLink::Insert(CMatrixLink* link)` (0x54E8F0): insert `link` after `this`.
pub fn insert(m: &mut impl Mem32, this: u32, link: u32) {
    let next = m.rd(this + LINK_NEXT);
    m.wr(link + LINK_NEXT, next);
    let next = m.rd(this + LINK_NEXT);
    m.wr(next + LINK_PREV, link);
    m.wr(link + LINK_PREV, this);
    m.wr(this + LINK_NEXT, link);
}

/// `CMatrixLink::Remove()` (0x54E910).
pub fn remove(m: &mut impl Mem32, this: u32) {
    let next = m.rd(this + LINK_NEXT);
    let prev = m.rd(this + LINK_PREV);
    m.wr(next + LINK_PREV, prev);
    let prev = m.rd(this + LINK_PREV);
    let next = m.rd(this + LINK_NEXT);
    m.wr(prev + LINK_NEXT, next);
}

fn add_to(m: &mut impl Mem32, list: u32, head: u32) -> u32 {
    let l = m.rd(list + LIST_FREE_HEAD + LINK_NEXT);
    if l == list + LIST_FREE_TAIL {
        return 0;
    }
    remove(m, l);
    insert(m, list + head, l);
    l
}

/// `CMatrixLinkList::AddToList1()` (0x54E9D0): the first free link moved after the head of list 1;
/// returns it, or 0 when the free list is empty.
pub fn add_to_list1(m: &mut impl Mem32, list: u32) -> u32 {
    add_to(m, list, LIST_HEAD)
}

/// `CMatrixLinkList::AddToList2()` (0x54EA10): as `AddToList1` for list 2.
pub fn add_to_list2(m: &mut impl Mem32, list: u32) -> u32 {
    add_to(m, list, LIST_ALLOC_HEAD)
}

fn move_to(m: &mut impl Mem32, list: u32, head: u32, l: u32) {
    remove(m, l);
    insert(m, list + head, l);
}

/// `CMatrixLinkList::MoveToList1(CMatrixLink*)` (0x54EA60).
pub fn move_to_list1(m: &mut impl Mem32, list: u32, l: u32) {
    move_to(m, list, LIST_HEAD, l)
}

/// `CMatrixLinkList::MoveToList2(CMatrixLink*)` (0x54EA90).
pub fn move_to_list2(m: &mut impl Mem32, list: u32, l: u32) {
    move_to(m, list, LIST_ALLOC_HEAD, l)
}

/// `CMatrixLinkList::MoveToFreeList(CMatrixLink*)` (0x54EAD0).
pub fn move_to_free_list(m: &mut impl Mem32, list: u32, l: u32) {
    move_to(m, list, LIST_FREE_HEAD, l)
}

fn count(m: &impl Mem32, list: u32, head: u32, tail: u32) -> u32 {
    let mut p = m.rd(list + head + LINK_NEXT);
    let mut n = 0u32;
    while p != list + tail {
        p = m.rd(p + LINK_NEXT);
        n = n.wrapping_add(1);
    }
    n
}

/// `CMatrixLinkList::GetNumFree()` (0x54EB10).
pub fn get_num_free(m: &impl Mem32, list: u32) -> u32 {
    count(m, list, LIST_FREE_HEAD, LIST_FREE_TAIL)
}

/// `CMatrixLinkList::GetNumUsed1()` (0x54EB30).
pub fn get_num_used1(m: &impl Mem32, list: u32) -> u32 {
    count(m, list, LIST_HEAD, LIST_TAIL)
}

/// `CMatrixLinkList::GetNumUsed2()` (0x54EB50).
pub fn get_num_used2(m: &impl Mem32, list: u32) -> u32 {
    count(m, list, LIST_ALLOC_HEAD, LIST_ALLOC_TAIL)
}

/// `CMatrixLinkList::Init(int count)` (0x54F0D0). `new_array(bytes)` is `operator new` (0x82119A); the
/// array gets the count prefix and each link the `CMatrixLink` constructor (0x54F0C0: attach fields
/// 0), in increasing order. Then the six sentinels are linked and every object is inserted after the
/// free head, from the last to the first. A null allocation leaves `m_pObjects` 0 and the original
/// still runs the insert loop on `0 + offset` (as this port does).
pub fn init(m: &mut impl Mem32, list: u32, count: u32, new_array: impl FnOnce(u32) -> u32) {
    let p = new_array(count.wrapping_mul(LINK_SIZE).wrapping_add(4));
    let mut objects = 0;
    if p != 0 {
        m.wr(p, count);
        objects = p + 4;
        for i in 0..count {
            let o = objects + i * LINK_SIZE;
            m.wr(o + 0x40, 0);
            m.wr(o + 0x44, 0);
        }
    }
    m.wr(list + LIST_HEAD + LINK_NEXT, list + LIST_TAIL);
    m.wr(list + LIST_FREE_HEAD + LINK_NEXT, list + LIST_FREE_TAIL);
    m.wr(list + LIST_OBJECTS, objects);
    m.wr(list + LIST_TAIL + LINK_PREV, list + LIST_HEAD);
    m.wr(list + LIST_ALLOC_HEAD + LINK_NEXT, list + LIST_ALLOC_TAIL);
    m.wr(list + LIST_ALLOC_TAIL + LINK_PREV, list + LIST_ALLOC_HEAD);
    m.wr(list + LIST_FREE_TAIL + LINK_PREV, list + LIST_FREE_HEAD);
    let mut off = count.wrapping_mul(LINK_SIZE);
    for _ in 0..count {
        off = off.wrapping_sub(LINK_SIZE);
        let o = m.rd(list + LIST_OBJECTS).wrapping_add(off);
        insert(m, list + LIST_FREE_HEAD, o);
    }
}

/// `CMatrixLinkList::Shutdown()` (0x54E990): when allocated, the destructors (`CMatrix::~CMatrix`,
/// 0x59ACD0, on each object from the last to the first, passed in as `destroy_link(addr)`), then
/// `free(objects - 4)`; `m_pObjects = 0` in any case.
pub fn shutdown(m: &mut impl Mem32, list: u32, mut destroy_link: impl FnMut(u32), free: impl FnOnce(u32)) {
    let objects = m.rd(list + LIST_OBJECTS);
    if objects != 0 {
        let n = m.rd(objects - 4);
        for i in (0..n).rev() {
            destroy_link(objects + i * LINK_SIZE);
        }
        free(objects - 4);
    }
    m.wr(list + LIST_OBJECTS, 0);
}
