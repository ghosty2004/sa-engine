//! `CMenuSystem` leaves (PORT-HUD): the script menus SA:MP uses. Ported from the 1.0 US bytes;
//! gta-reversed `MenuSystem.cpp` as reading aid. Menu creation, input and drawing stay original in this
//! pass.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

sa_core::globals! {
    list GLOBALS;
    /// `CMenuSystem::MenuNumber[2]` (`tMenuPanel*`)
    pub static MENU_NUMBER @ 0xBA82D8: [u32; 2] = [0; 2];
    /// `CMenuSystem::MenuInUse[2]`
    pub static MENU_IN_USE @ 0xBA82E0: [u8; 2] = [0; 2];
    /// `CMenuSystem::CurrentMenuInUse`
    pub static CURRENT_MENU_IN_USE @ 0xBA82E2: u8 = 0;
    /// `CMenuSystem::num_menus_in_use`
    pub static NUM_MENUS_IN_USE @ 0xBA82E3: u8 = 0;
}

/// `tMenuPanel` offsets used here.
const ROW_TITLE: isize = 0x41; // GxtChar[10] per row
const COLUMN_WIDTH: isize = 0x3FC; // float[] per column
const ROW_SELECTABLE: isize = 0x3D6; // bool[12]
const NUM_ROWS: isize = 0x3F6; // uint8
const SELECTED_ROW: isize = 0x415; // int8

orig! {
    /// `operator delete(void*)` (0x8214BD)
    pub fn operator_delete(p: GPtr<u8>) = cdecl 0x8214BD => unported;
}

/// `MenuNumber[menu]` (index read as the bytes do, `[menu*4 + 0xBA82D8]`).
fn panel(menu: u8) -> GPtr<u8> {
    g::<GPtr<u8>>(0xBA82D8 + usize::from(menu) * 4).read()
}

/// `MenuInUse[i]` (index read as the bytes do; 2 is `CurrentMenuInUse`).
fn in_use(i: usize) -> u8 {
    g::<u8>(0xBA82E0 + i).read()
}

/// `CMenuSystem::SetColumnWidth(uint8 menu, uint8 column, uint16 width)` (0x582050):
/// `fild` of the width into the column's float.
#[hook(addr = 0x582050, conv = cdecl, name = "CMenuSystem::SetColumnWidth", expect = "0F B7 44 24 0C")]
pub fn set_column_width(menu: u8, column: u8, width: u16) {
    wr(
        panel(menu),
        COLUMN_WIDTH + isize::from(column) * 4,
        f32::from(width),
    );
}

/// `CMenuSystem::CheckForSelected(uint8 menu)` (0x5807E0): the selected row while the menu is in use,
/// else -99 (`mov al, 0x9D`; only AL is set).
#[hook(addr = 0x5807E0, conv = cdecl, name = "CMenuSystem::CheckForSelected", expect = "0F B6 44 24 04")]
pub fn check_for_selected(menu: u8) -> u8 {
    if in_use(usize::from(menu)) != 0 {
        rd(panel(menu), SELECTED_ROW)
    } else {
        0x9D
    }
}

/// `CMenuSystem::SwitchOffMenu(uint8 menu)` (0x580750): frees the panel (the pointer is left), marks it
/// unused, one menu fewer, and makes the other menu current when it is in use (index walk as the bytes:
/// 1, then 2 = `CurrentMenuInUse` itself, just cleared).
#[hook(addr = 0x580750, conv = cdecl, name = "CMenuSystem::SwitchOffMenu", expect = "56 E9 41 12 E8 FF")]
pub fn switch_off_menu(menu: u8) {
    let m = usize::from(menu);
    if in_use(m) == 0 {
        return;
    }
    operator_delete(panel(menu));
    g::<u8>(0xBA82E0 + m).write(0);
    NUM_MENUS_IN_USE.set(NUM_MENUS_IN_USE.get().wrapping_sub(1));
    CURRENT_MENU_IN_USE.set(0);
    if in_use(0) != 0 {
        return;
    }
    let mut a: i8 = 0;
    while a < 2 {
        a += 1;
        if in_use(a as usize) != 0 {
            break;
        }
    }
    CURRENT_MENU_IN_USE.set(a as u8);
    if a == 2 {
        CURRENT_MENU_IN_USE.set(0);
    }
}

/// The selected-row walk shared by `ActivateItems` / `ActivateOneItem`: advance while the row is not
/// selectable or has no title and is below the row count; wrap to 0 once and walk again; 0 if still
/// out of range.
fn walk_selection(menu: u8) {
    let walk = || loop {
        let p = panel(menu);
        let r: i8 = rd(p, SELECTED_ROW);
        let ri = isize::from(r);
        if rd::<u8>(p, ROW_SELECTABLE + ri) != 0 && rd::<u8>(p, ROW_TITLE + ri * 10) != 0 {
            break;
        }
        if i32::from(r) >= i32::from(rd::<u8>(p, NUM_ROWS)) {
            break;
        }
        wr(p, SELECTED_ROW, r.wrapping_add(1));
    };
    walk();
    let p = panel(menu);
    if i32::from(rd::<i8>(p, SELECTED_ROW)) >= i32::from(rd::<u8>(p, NUM_ROWS)) {
        wr::<u8>(p, SELECTED_ROW, 0);
        walk();
    }
    let p = panel(menu);
    if i32::from(rd::<i8>(p, SELECTED_ROW)) >= i32::from(rd::<u8>(p, NUM_ROWS)) {
        wr::<u8>(p, SELECTED_ROW, 0);
    }
}

/// `CMenuSystem::ActivateOneItem(uint8 menu, uint8 row, bool active)` (0x581B30)
#[hook(addr = 0x581B30, conv = cdecl, name = "CMenuSystem::ActivateOneItem", expect = "0F B6 44 24 08")]
pub fn activate_one_item(menu: u8, row: u8, active: u8) {
    wr(panel(menu), ROW_SELECTABLE + isize::from(row), active);
    walk_selection(menu);
}

/// `CMenuSystem::ActivateItems(uint8 menu, bool x12)` (0x581990): the 12 row flags, then the walk.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x581990, conv = cdecl, name = "CMenuSystem::ActivateItems", expect = "8A 4C 24 08 56")]
pub fn activate_items(
    menu: u8,
    a0: u8,
    a1: u8,
    a2: u8,
    a3: u8,
    a4: u8,
    a5: u8,
    a6: u8,
    a7: u8,
    a8: u8,
    a9: u8,
    a10: u8,
    a11: u8,
) {
    for (k, v) in [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11]
        .into_iter()
        .enumerate()
    {
        wr(panel(menu), ROW_SELECTABLE + k as isize, v);
    }
    walk_selection(menu);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_SET_COLUMN_WIDTH,
    HOOK_CHECK_FOR_SELECTED,
    HOOK_SWITCH_OFF_MENU,
    HOOK_ACTIVATE_ONE_ITEM,
    HOOK_ACTIVATE_ITEMS,
];
