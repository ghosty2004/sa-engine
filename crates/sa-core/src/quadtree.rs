//! CORE ports: `CQuadTreeNode<T>` (0x5525A0–0x552E4B) and its pool set-up (see
//! `docs/arch/core-port.md`).
//!
//! Layout (one instantiation, `CQuadTreeNode<void*>`, 0x28 bytes, pool slot 0x28): +0x00 `CRect`
//! (x0, y0, x1, y1 as stored: the original compares +0x04 with "top" and +0x0C with "bottom"),
//! +0x10 the item list head (`CPtrNodeSingleLink*`: item at +0, next at +4), +0x14 the four children,
//! +0x24 the level (0 = leaf). Every float op is x87 under PC=24 ([`X87`]); the sector bounds are
//! computed as `(a + b) * 0.5` (constant 0x858B8C) and either stored as f32 or kept in ST0, exactly
//! as each function does. Where gta-reversed's `QuadTreeNode.h` differs, the original is followed:
//! `DeleteItem(item, rect)` calls `InSector` for every sector without a null check on the child and
//! recurses with the rect (gr: null check first, then the rect-less `DeleteItem`); both `DeleteItem`s
//! call `CPtrListSingleLink::DeleteItem` only after finding the item in the list; `AddItem` on a leaf
//! inlines the list insertion (`new` 0x552380, head insertion).

use crate::mem::GPtr;
use crate::{globals, hook, orig};
use sa_math::x87::X87;

/// 0x858B8C: 0.5
const HALF: u32 = 0x3F00_0000;

globals! {
    list GLOBALS;
    /// `CQuadTreeNode::ms_pQuadTreeNodePool`
    pub static QUAD_TREE_NODE_POOL @ 0xB745BC: GPtr<u8> = GPtr::NULL;
}

pub mod call {
    use super::*;
    orig! {
        /// `CPtrListSingleLink::AddItem(void*)` (thiscall)
        pub fn list_add_item(list: GPtr<u8>, item: GPtr<u8>) -> GPtr<u8> = thiscall 0x5335E0 => unported;
        /// `CPtrListSingleLink::DeleteItem(void*)` (thiscall)
        pub fn list_delete_item(list: GPtr<u8>, item: GPtr<u8>) = thiscall 0x533610 => unported;
        /// `CPtrNodeSingleLink::operator new` (0x552380: `mov ecx, ms_pPtrNodeSingleLinkPool; jmp New`)
        pub fn ptr_node_single_new(size: u32) -> GPtr<u8> = cdecl 0x552380 => unported;
        /// The quad-tree node pool's `New` (0x552B50, slot 0x28)
        pub fn quad_tree_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x552B50 => unported;
        /// `operator new` (0x82119A)
        pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
        /// The quad-tree node pool constructor (0x552AF0)
        pub fn pool_ctor(p: GPtr<u8>, size: i32, name: GPtr<u8>) -> GPtr<u8> = thiscall 0x552AF0 => unported;
        pub fn find_sector_vec(node: GPtr<u8>, v: GPtr<u8>) -> i32 = thiscall 0x552640 => super::find_sector_vec;
        pub fn in_sector(node: GPtr<u8>, r: GPtr<u8>, sector: i32) -> bool = thiscall 0x5526A0 => super::in_sector;
        pub fn get_all(node: GPtr<u8>, list: GPtr<u8>) = thiscall 0x552870 => super::get_all;
        pub fn get_all_matching_rect(node: GPtr<u8>, r: GPtr<u8>, list: GPtr<u8>) = thiscall 0x5528C0 => super::get_all_matching_rect;
        pub fn for_all_matching_rect(node: GPtr<u8>, r: GPtr<u8>, cb: u32) = thiscall 0x552980 => super::for_all_matching_rect;
        pub fn delete_item(node: GPtr<u8>, item: GPtr<u8>) = thiscall 0x552A40 => super::delete_item;
        pub fn delete_item_rect(node: GPtr<u8>, item: GPtr<u8>, r: GPtr<u8>) = thiscall 0x552A90 => super::delete_item_rect;
        pub fn add_item(node: GPtr<u8>, item: GPtr<u8>, r: GPtr<u8>) = thiscall 0x552CD0 => super::add_item;
        /// a `void (*)(const CRect&/CVector2D&, T)` callback (cdecl)
        pub fn callback(f: u32, at: GPtr<u8>, item: GPtr<u8>) = cdecl dyn => unported;
    }
}

fn at<U>(p: GPtr<u8>, off: isize) -> GPtr<U> {
    p.byte_add(off).cast()
}

fn rect_at(p: GPtr<u8>) -> [f32; 4] {
    core::array::from_fn(|k| at::<f32>(p, 4 * k as isize).read())
}

/// `(a + b) * 0.5` with `a` loaded first and `b` added from memory (`fld a; fadd b; fmul half`).
fn mid_m(a: f32, b: f32) -> X87 {
    X87::from_f32(a).fadd_m(b).fmul_m(f32::from_bits(HALF))
}

/// `(b + a) * 0.5` in the register form (`fld a; fld b; fadd st, st(1); fmul half`).
fn mid_r(a: f32, b: f32) -> X87 {
    X87::from_f32(b)
        .fadd(X87::from_f32(a))
        .fmul_m(f32::from_bits(HALF))
}

/// `CQuadTreeNode::FindSector(const CRect&)` (0x5525A0) on the node rect `n`, level, query rect `r`.
/// `mx = f32((n.x1 + n.x0) * 0.5)`, `my = f32((n.y1 + n.y0) * 0.5)`; level 0 → -1.
/// `r.y0 < my` → cl = 0; else `r.y1 > my` → cl = 1, otherwise -1. Then `r.x1 < mx` → (cl ? 0 : 2);
/// else `r.x0 > mx` → (cl ? 1 : 3), otherwise -1. (NaN compares take the "not less"/"not greater"
/// branches of `test ah,5; jp` / `test ah,41h; jne`.)
pub fn find_sector_rect_of(n: [f32; 4], level: u32, r: [f32; 4]) -> i32 {
    let mx = X87::from_f32(n[2])
        .fadd_m(n[0])
        .fmul_m(f32::from_bits(HALF))
        .to_f32();
    let my = X87::from_f32(n[3])
        .fadd_m(n[1])
        .fmul_m(f32::from_bits(HALF))
        .to_f32();
    if level == 0 {
        return -1;
    }
    let c = X87::from_f32(r[1]).fcom(X87::from_f32(my));
    let cl = if !c.test_parity(5) {
        false
    } else {
        let c = X87::from_f32(r[3]).fcom(X87::from_f32(my));
        if c.test_any(0x41) {
            return -1;
        }
        true
    };
    let c = X87::from_f32(r[2]).fcom(X87::from_f32(mx));
    if !c.test_parity(5) {
        return if cl { 0 } else { 2 };
    }
    let c = X87::from_f32(r[0]).fcom(X87::from_f32(mx));
    if c.test_any(0x41) {
        return -1;
    }
    if cl { 1 } else { 3 }
}

/// `CQuadTreeNode::FindSector(const CVector2D&)` (0x552640): the mid values stay in ST0/ST1
/// (unrounded to f32). `my <= v.y` (or NaN) → cl = 1; then `mx <= v.x` (or NaN) → (cl ? 1 : 3),
/// else (cl ? 0 : 2). Level 0 → -1.
pub fn find_sector_vec_of(n: [f32; 4], level: u32, v: [f32; 2]) -> i32 {
    let mx = mid_m(n[2], n[0]);
    if level == 0 {
        return -1;
    }
    let my = mid_m(n[3], n[1]);
    let cl = my.fcom(X87::from_f32(v[1])).test_any(0x41);
    if mx.fcom(X87::from_f32(v[0])).test_any(0x41) {
        if cl { 1 } else { 3 }
    } else if cl {
        0
    } else {
        2
    }
}

/// `CQuadTreeNode::InSector(const CRect&, int32 sector)` (0x5526A0). The sector's bounds: case 0
/// stores `x1 = mx, y1 = my` (mx from the register form), case 1 keeps `x0 := mx` in ST0 only and
/// stores `y1 = my`, case 2 stores `x1 = mx, y0 = my`, case 3 keeps `x0 := mx` in ST0 and stores
/// `y0 = my`; any other sector value compares the whole node rect. Then
/// `x0 <= r.x1 (or NaN) && !(x1 < r.x0) && y1 <= r.y0 (or NaN) && !(y0 < r.y1)`.
pub fn in_sector_of(n: [f32; 4], level: u32, r: [f32; 4], sector: i32) -> bool {
    if level == 0 {
        return false;
    }
    let mut l = n;
    let half = f32::from_bits(HALF);
    let my_m = || X87::from_f32(n[3]).fadd_m(n[1]).fmul_m(half).to_f32();
    let x0: X87 = match sector as u32 {
        0 => {
            l[2] = mid_r(n[0], n[2]).to_f32();
            l[3] = my_m();
            X87::from_f32(n[0])
        }
        1 => {
            let mx = mid_m(n[0], n[2]);
            l[3] = my_m();
            mx
        }
        2 => {
            l[2] = mid_r(n[0], n[2]).to_f32();
            l[1] = my_m();
            X87::from_f32(n[0])
        }
        3 => {
            let mx = mid_m(n[0], n[2]);
            l[1] = my_m();
            mx
        }
        _ => X87::from_f32(n[0]),
    };
    let f = X87::from_f32;
    if !x0.fcom(f(r[2])).test_any(0x41) {
        return false;
    }
    if !f(l[2]).fcom(f(r[0])).test_parity(5) {
        return false;
    }
    if !f(l[3]).fcom(f(r[1])).test_any(0x41) {
        return false;
    }
    if !f(l[1]).fcom(f(r[3])).test_parity(5) {
        return false;
    }
    true
}

/// The child rect `AddItem` builds for `sector` (jump table 0x552E50): mids as `fld a; fadd b`
/// (x0 + x1, y1 + y0) stored as f32; sector 0: x1, y1; 1: x0, y1; 2: x1, y0; 3: x0, y0.
pub fn child_rect_of(n: [f32; 4], sector: i32) -> [f32; 4] {
    let mut l = n;
    let mx = || mid_m(n[0], n[2]).to_f32();
    let my = || mid_m(n[3], n[1]).to_f32();
    match sector {
        0 => {
            l[2] = mx();
            l[3] = my();
        }
        1 => {
            l[0] = mx();
            l[3] = my();
        }
        2 => {
            l[2] = mx();
            l[1] = my();
        }
        3 => {
            l[0] = mx();
            l[1] = my();
        }
        _ => {}
    }
    l
}

fn level(node: GPtr<u8>) -> u32 {
    at::<u32>(node, 0x24).read()
}

fn child(node: GPtr<u8>, k: i32) -> GPtr<u8> {
    at::<GPtr<u8>>(node, 0x14 + 4 * k as isize).read()
}

/// Calls `f(item)` for every item of the node's list (`[node]` then `[node+4]`).
fn for_items(node: GPtr<u8>, mut f: impl FnMut(GPtr<u8>)) {
    let mut n = at::<GPtr<u8>>(node, 0x10).read();
    while !n.is_null() {
        f(at::<GPtr<u8>>(n, 0).read());
        n = at::<GPtr<u8>>(n, 4).read();
    }
}

/// `CQuadTreeNode::FindSector(const CRect&)` (0x5525A0, thiscall, `ret 4`).
#[hook(addr = 0x5525A0, conv = thiscall, name = "CQuadTreeNode<T>::FindSector-rect", expect = "83 EC 08 D9 41 08")]
pub fn find_sector_rect(node: GPtr<u8>, r: GPtr<u8>) -> i32 {
    find_sector_rect_of(rect_at(node), level(node), rect_at(r))
}

/// `CQuadTreeNode::FindSector(const CVector2D&)` (0x552640, thiscall, `ret 4`).
#[hook(addr = 0x552640, conv = thiscall, name = "CQuadTreeNode<T>::FindSector-vec", expect = "D9 41 08 8B 41 24")]
pub fn find_sector_vec(node: GPtr<u8>, v: GPtr<u8>) -> i32 {
    let n = rect_at(node);
    let lv = level(node);
    if lv == 0 {
        return -1;
    }
    find_sector_vec_of(n, lv, [at::<f32>(v, 0).read(), at::<f32>(v, 4).read()])
}

/// `CQuadTreeNode::InSector(const CRect&, int32)` (0x5526A0, thiscall, `ret 8`, bool in AL).
#[hook(addr = 0x5526A0, conv = thiscall, name = "CQuadTreeNode<T>::InSector", expect = "83 EC 10 8B C1")]
pub fn in_sector(node: GPtr<u8>, r: GPtr<u8>, sector: i32) -> bool {
    let n = rect_at(node);
    let lv = level(node);
    if lv == 0 {
        return false;
    }
    in_sector_of(n, lv, rect_at(r), sector)
}

/// `CQuadTreeNode::GetAll(list)` (0x552870, thiscall, `ret 4`): items into `list`, then each
/// non-null child recursively (re-read per sector).
#[hook(addr = 0x552870, conv = thiscall, name = "CQuadTreeNode<T>::GetAll", expect = "53 8B 5C 24 08")]
pub fn get_all(node: GPtr<u8>, list: GPtr<u8>) {
    for_items(node, |it| {
        call::list_add_item(list, it);
    });
    for k in 0..4 {
        let c = child(node, k);
        if !c.is_null() {
            call::get_all(c, list);
        }
    }
}

/// `CQuadTreeNode::GetAllMatching(const CRect&, list)` (0x5528C0, `ret 8`): items, then each
/// non-null child whose sector `InSector` accepts.
#[hook(addr = 0x5528C0, conv = thiscall, name = "CQuadTreeNode<T>::GetAllMatching-rect", expect = "51 53 55 8B D9")]
pub fn get_all_matching_rect(node: GPtr<u8>, r: GPtr<u8>, list: GPtr<u8>) {
    for_items(node, |it| {
        call::list_add_item(list, it);
    });
    for k in 0..4 {
        let c = child(node, k);
        if !c.is_null() && call::in_sector(node, r, k) {
            call::get_all_matching_rect(c, r, list);
        }
    }
}

/// `CQuadTreeNode::GetAllMatching(const CVector2D&, list)` (0x552930, `ret 8`): iterative descent
/// through `FindSector(vec)`.
#[hook(addr = 0x552930, conv = thiscall, name = "CQuadTreeNode<T>::GetAllMatching-vec", expect = "53 8B 5C 24 0C")]
pub fn get_all_matching_vec(node: GPtr<u8>, v: GPtr<u8>, list: GPtr<u8>) {
    let mut n = node;
    loop {
        for_items(n, |it| {
            call::list_add_item(list, it);
        });
        let s = call::find_sector_vec(n, v);
        if s == -1 {
            return;
        }
        let c = child(n, s);
        if c.is_null() {
            return;
        }
        n = c;
    }
}

/// `CQuadTreeNode::ForAllMatching(const CRect&, callback)` (0x552980, `ret 8`): `callback(rect,
/// item)` (cdecl) per item, then the accepted non-null children.
#[hook(addr = 0x552980, conv = thiscall, name = "CQuadTreeNode<T>::ForAllMatching-rect", expect = "51 53 8B 5C 24 0C")]
pub fn for_all_matching_rect(node: GPtr<u8>, r: GPtr<u8>, cb: u32) {
    for_items(node, |it| call::callback(cb, r, it));
    for k in 0..4 {
        let c = child(node, k);
        if !c.is_null() && call::in_sector(node, r, k) {
            call::for_all_matching_rect(c, r, cb);
        }
    }
}

/// `CQuadTreeNode::ForAllMatching(const CVector2D&, callback)` (0x5529F0, `ret 8`): iterative.
#[hook(addr = 0x5529F0, conv = thiscall, name = "CQuadTreeNode<T>::ForAllMatching-vec", expect = "53 8B 5C 24 08")]
pub fn for_all_matching_vec(node: GPtr<u8>, v: GPtr<u8>, cb: u32) {
    let mut n = node;
    loop {
        for_items(n, |it| call::callback(cb, v, it));
        let s = call::find_sector_vec(n, v);
        if s == -1 {
            return;
        }
        let c = child(n, s);
        if c.is_null() {
            return;
        }
        n = c;
    }
}

/// The list part of both `DeleteItem`s: `CPtrListSingleLink::DeleteItem` only when the item is in the
/// list.
fn delete_from_list(node: GPtr<u8>, item: GPtr<u8>) {
    let mut n = at::<GPtr<u8>>(node, 0x10).read();
    while !n.is_null() {
        if at::<GPtr<u8>>(n, 0).read() == item {
            call::list_delete_item(node.byte_add(0x10), item);
            return;
        }
        n = at::<GPtr<u8>>(n, 4).read();
    }
}

/// `CQuadTreeNode::DeleteItem(T)` (0x552A40, `ret 4`).
#[hook(addr = 0x552A40, conv = thiscall, name = "CQuadTreeNode<T>::DeleteItem-", expect = "53 8B 5C 24 08")]
pub fn delete_item(node: GPtr<u8>, item: GPtr<u8>) {
    delete_from_list(node, item);
    for k in 0..4 {
        let c = child(node, k);
        if !c.is_null() {
            call::delete_item(c, item);
        }
    }
}

/// `CQuadTreeNode::DeleteItem(T, const CRect&)` (0x552A90, `ret 8`): `InSector` for every sector,
/// then `child->DeleteItem(item, rect)` without a null check.
#[hook(addr = 0x552A90, conv = thiscall, name = "CQuadTreeNode<T>::DeleteItem-rect", expect = "53 55 8B E9 8B 45 10")]
pub fn delete_item_rect(node: GPtr<u8>, item: GPtr<u8>, r: GPtr<u8>) {
    delete_from_list(node, item);
    for k in 0..4 {
        if call::in_sector(node, r, k) {
            call::delete_item_rect(child(node, k), item, r);
        }
    }
}

/// `CQuadTreeNode::InitPool` (0x552C00): when `ms_pQuadTreeNodePool` is null,
/// `new(0x14)` + `CPool(400, "QuadTreeNodes")` (null stored when `new` fails).
#[hook(addr = 0x552C00, conv = cdecl, name = "CQuadTreeNode<T>::InitPool", expect = "6A FF 68 4B CC 83 00")]
pub fn init_pool() {
    if !QUAD_TREE_NODE_POOL.get().is_null() {
        return;
    }
    let p = call::operator_new(0x14);
    let r = if p.is_null() {
        GPtr::NULL
    } else {
        call::pool_ctor(p, 400, GPtr::from_addr(0x863DEC))
    };
    QUAD_TREE_NODE_POOL.set(r);
}

/// `CQuadTreeNode::AddItem(T, const CRect&)` (0x552CD0, `ret 8`). Leaf: a new list node (0x552380)
/// with the item inserted at the head (a null node is written through, as the original). Otherwise per
/// accepted sector: a missing child is taken from the node pool (0x552B50) and set up with
/// [`child_rect_of`], empty list, `level - 1` (read from the parent), no children (null stored when the
/// pool is full); then `child->AddItem` (re-read, no null check).
#[hook(addr = 0x552CD0, conv = thiscall, name = "CQuadTreeNode<T>::AddItem", expect = "83 EC 10 56 8B F1")]
pub fn add_item(node: GPtr<u8>, item: GPtr<u8>, r: GPtr<u8>) {
    if level(node) == 0 {
        let ln = call::ptr_node_single_new(8);
        if !ln.is_null() {
            at::<GPtr<u8>>(ln, 0).write(item);
        }
        at::<GPtr<u8>>(ln, 4).write(at::<GPtr<u8>>(node, 0x10).read());
        at::<GPtr<u8>>(node, 0x10).write(ln);
        return;
    }
    for k in 0..4 {
        if !call::in_sector(node, r, k) {
            continue;
        }
        if child(node, k).is_null() {
            let rect = child_rect_of(rect_at(node), k);
            let c = call::quad_tree_pool_new(QUAD_TREE_NODE_POOL.get());
            if !c.is_null() {
                let lv = level(node);
                for (j, v) in rect.iter().enumerate() {
                    at::<f32>(c, 4 * j as isize).write(*v);
                }
                at::<u32>(c, 0x10).write(0);
                at::<u32>(c, 0x24).write(lv.wrapping_sub(1));
                for j in 0..4 {
                    at::<u32>(c, 0x14 + 4 * j).write(0);
                }
            }
            at::<GPtr<u8>>(node, 0x14 + 4 * k as isize).write(c);
        }
        call::add_item(child(node, k), item, r);
    }
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_FIND_SECTOR_RECT,
    HOOK_FIND_SECTOR_VEC,
    HOOK_IN_SECTOR,
    HOOK_GET_ALL,
    HOOK_GET_ALL_MATCHING_RECT,
    HOOK_GET_ALL_MATCHING_VEC,
    HOOK_FOR_ALL_MATCHING_RECT,
    HOOK_FOR_ALL_MATCHING_VEC,
    HOOK_DELETE_ITEM,
    HOOK_DELETE_ITEM_RECT,
    HOOK_INIT_POOL,
    HOOK_ADD_ITEM,
];

#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    const N: [f32; 4] = [0.0, 0.0, 100.0, 100.0];

    #[test]
    fn sectors_of_a_point() {
        // mid (50, 50); cl = (my <= y)
        assert_eq!(find_sector_vec_of(N, 1, [10.0, 10.0]), 2);
        assert_eq!(find_sector_vec_of(N, 1, [10.0, 60.0]), 0);
        assert_eq!(find_sector_vec_of(N, 1, [60.0, 10.0]), 3);
        assert_eq!(find_sector_vec_of(N, 1, [60.0, 60.0]), 1);
        assert_eq!(find_sector_vec_of(N, 0, [60.0, 60.0]), -1);
        // NaN coordinates take the `<=` side
        assert_eq!(find_sector_vec_of(N, 1, [f32::NAN, f32::NAN]), 1);
    }

    #[test]
    fn sectors_of_a_rect() {
        assert_eq!(find_sector_rect_of(N, 1, [1.0, 1.0, 2.0, 2.0]), 2);
        assert_eq!(find_sector_rect_of(N, 1, [60.0, 1.0, 70.0, 2.0]), 3);
        assert_eq!(find_sector_rect_of(N, 1, [1.0, 60.0, 2.0, 70.0]), 0);
        assert_eq!(find_sector_rect_of(N, 1, [60.0, 60.0, 70.0, 70.0]), 1);
        // straddling the middle
        assert_eq!(find_sector_rect_of(N, 1, [40.0, 1.0, 60.0, 2.0]), -1);
        assert_eq!(find_sector_rect_of(N, 0, [1.0, 1.0, 2.0, 2.0]), -1);
    }

    #[test]
    fn child_rects_cover_the_quadrants() {
        assert_eq!(child_rect_of(N, 0), [0.0, 0.0, 50.0, 50.0]);
        assert_eq!(child_rect_of(N, 1), [50.0, 0.0, 100.0, 50.0]);
        assert_eq!(child_rect_of(N, 2), [0.0, 50.0, 50.0, 100.0]);
        assert_eq!(child_rect_of(N, 3), [50.0, 50.0, 100.0, 100.0]);
        // a rect inside quadrant 0's bounds as InSector reads them (x0 <= r.x1, x1 >= r.x0, y1 <= r.y0, y0 >= r.y1)
        assert!(in_sector_of(N, 1, [10.0, 60.0, 20.0, -5.0], 0));
        assert!(!in_sector_of(N, 0, [10.0, 60.0, 20.0, -5.0], 0));
    }
}
