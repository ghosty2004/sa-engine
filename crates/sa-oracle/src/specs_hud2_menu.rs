//! L1 specs for PORT-HUD-2 (`sa_hud::menu2`): the `CMenuSystem` panel setters and getters. Both
//! `MenuNumber` slots are patched to point at the scratch panel (region 0, 0x418 bytes, random contents),
//! `MenuInUse` to {1, 0}; menu indices are 0 or 1 in the low byte (random upper bytes: the original reads
//! the byte only). Strings are short and NUL-terminated so every copy stays inside the panel.

use crate::call::Conv::*;
use crate::call::SCRATCH;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_hud::menu2;

fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

fn seed(v: &[u32]) -> u64 {
    let mut h: u64 = 0x3E2_A41;
    for x in v {
        h = (h ^ u64::from(*x)).wrapping_mul(0x100_0000_01B3);
    }
    h
}

const MENU_NUMBER: u32 = 0xBA82D8;
const MENU_IN_USE: u32 = 0xBA82E0;
/// Emulated address of region 0 (the first region starts at scratch + 0x10).
const PANEL: u32 = SCRATCH + 0x10;
const PANEL_WORDS: usize = 0x418 / 4;

fn menu(r: &mut Rng) -> u32 {
    (r.next_u32() & !0xFF) | r.below(2)
}
fn column4(r: &mut Rng) -> u32 {
    (r.next_u32() & !0xFF) | r.below(4)
}
fn row12(r: &mut Rng) -> u32 {
    (r.next_u32() & !0xFF) | r.below(12)
}
fn small(r: &mut Rng) -> u32 {
    (r.next_u32() & !0xFF) | r.below(0x36)
}
fn byte(r: &mut Rng) -> u32 {
    r.next_u32()
}
fn null(_: &mut Rng) -> u32 {
    0
}

fn panel_patch() -> Vec<(u32, Vec<u8>)> {
    let mut p = PANEL.to_le_bytes().to_vec();
    p.extend_from_slice(&PANEL.to_le_bytes());
    vec![(MENU_NUMBER, p), (MENU_IN_USE, vec![1, 0])]
}

fn panel_reads() -> Vec<(u32, u32)> {
    vec![(MENU_NUMBER, MENU_NUMBER + 8), (MENU_IN_USE, MENU_IN_USE + 2)]
}

/// A NUL-terminated string of 0..=24 bytes (no NUL inside) in region `ri`'s words.
fn put_string(v: &mut [u32], lay: &Layout, ri: usize, r: &mut Rng) {
    let len = r.below(25) as usize;
    let mut s: Vec<u8> = (0..len).map(|_| 1 + r.below(0xFF) as u8).collect();
    s.push(0);
    for (wi, slot) in lay.word_slot[ri].iter().enumerate() {
        let Some(k) = slot else { continue };
        let mut w = [0u8; 4];
        for (j, b) in w.iter_mut().enumerate() {
            *b = s.get(4 * wi + j).copied().unwrap_or((r.next_u32() & 0xFF) as u8);
        }
        v[*k] = u32::from_le_bytes(w);
    }
}

fn strings(v: &mut [u32], lay: &Layout) {
    let mut r = Rng::new(seed(v));
    for ri in 1..lay.word_slot.len() {
        put_string(v, lay, ri, &mut r);
    }
}

fn base(key: &'static str, name: &'static str, addr: u32, ret: RetKind) -> B {
    b(key, name, addr, Cdecl, ret).patch(panel_patch()).reads(panel_reads())
}

pub fn all() -> Vec<Spec> {
    vec![
        base("HudMenuCheckForAccept", "CMenuSystem::CheckForAccept", 0x5807C0, Al)
            .regions(vec![raw(PANEL_WORDS)])
            .args(vec![A::I(menu)])
            .rust(|c| {
                let used = [1u8, 0][(c.i(0) & 0xFF) as usize];
                let p = tp(c, 0);
                ExpRet::Al(menu2::check_for_accept_with(used, p))
            }),
        base("HudMenuInsertOneMenuItemWithNumber", "CMenuSystem::InsertOneMenuItemWithNumber", 0x581D70, Void)
            .regions(vec![raw(PANEL_WORDS), raw(8)])
            .args(vec![A::I(menu), A::I(column4), A::I(row12), A::P(1), A::I(byte), A::I(byte)])
            .fix(|v, l| strings(v, l))
            .rust(|c| {
                let (col, row, n1, n2) = (c.i(1), c.i(2), c.i(4) as i32, c.i(5) as i32);
                let (p, k) = (tp(c, 0), tp(c, 1));
                menu2::insert_one_menu_item_with_number_with(p, col, row, k, n1, n2);
                ExpRet::Void
            }),
        base("HudMenuInsertOneMenuItemWithNumberNull", "CMenuSystem::InsertOneMenuItemWithNumber", 0x581D70, Void)
            .regions(vec![raw(PANEL_WORDS)])
            .args(vec![A::I(menu), A::I(column4), A::I(row12), A::I(null), A::I(byte), A::I(byte)])
            .rust(|c| {
                let (col, row, n1, n2) = (c.i(1), c.i(2), c.i(4) as i32, c.i(5) as i32);
                let p = tp(c, 0);
                menu2::insert_one_menu_item_with_number_with(p, col, row, GPtr::from_addr(0), n1, n2);
                ExpRet::Void
            }),
        base("HudMenuSetColumnOrientation", "CMenuSystem::SetColumnOrientation", 0x582080, Void)
            .regions(vec![raw(PANEL_WORDS)])
            .args(vec![A::I(menu), A::I(small), A::I(byte)])
            .rust(|c| {
                let (col, o) = (c.i(1), c.i(2));
                menu2::set_column_orientation_with(tp(c, 0), col, o);
                ExpRet::Void
            }),
        base("HudMenuSetActiveMenuItem", "CMenuSystem::SetActiveMenuItem", 0x5820C0, Void)
            .regions(vec![raw(PANEL_WORDS)])
            .args(vec![A::I(menu), A::I(byte)])
            .rust(|c| {
                let row = c.i(1);
                menu2::set_active_menu_item_with(tp(c, 0), row);
                ExpRet::Void
            }),
        base("HudMenuGetCarColourFromGrid", "CMenuSystem::GetCarColourFromGrid", 0x5822B0, Al)
            .regions(vec![raw(PANEL_WORDS)])
            .args(vec![A::I(menu), A::I(byte)])
            .rust(|c| {
                let i = c.i(1);
                ExpRet::Al(menu2::get_car_colour_from_grid_with(tp(c, 0), i))
            }),
        base("HudMenuHighlightOneItem", "CMenuSystem::HighlightOneItem", 0x581C10, Void)
            .regions(vec![raw(PANEL_WORDS)])
            .args(vec![A::I(menu), A::I(small), A::I(byte)])
            .rust(|c| {
                let (row, h) = (c.i(1), c.i(2));
                menu2::highlight_one_item_with(tp(c, 0), row, h);
                ExpRet::Void
            }),
        base("HudMenuInsertMenu", "CMenuSystem::InsertMenu", 0x581E00, Void)
            .regions(vec![raw(PANEL_WORDS), raw(8), raw(8)])
            .args(vec![
                A::I(menu),
                A::I(column4),
                A::P(1),
                A::P(1),
                A::P(2),
                A::I(null),
                A::P(1),
                A::I(null),
                A::P(2),
                A::P(1),
                A::P(1),
                A::I(null),
                A::P(2),
                A::P(2),
                A::P(1),
            ])
            .fix(|v, l| strings(v, l))
            .rust(|c| {
                let col = c.i(1);
                let (p, a, b2) = (tp(c, 0), tp(c, 1), tp(c, 2));
                let n = GPtr::from_addr(0);
                let rows = [a, b2, n, a, n, b2, a, a, n, b2, b2, a];
                menu2::insert_menu_with(p, col, a, rows);
                ExpRet::Void
            }),
    ]
}
