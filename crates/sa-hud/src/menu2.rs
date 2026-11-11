//! `CMenuSystem`, second pass (PORT-HUD-2): menu creation, rows, the per-frame `Process` and the small
//! setters/getters of the script menus SA:MP uses. Ported from the 1.0 US bytes; gta-reversed
//! `MenuSystem.cpp` as reading aid. Drawing and input handling (`Display*Menu`, `Input*Menu`) and
//! `FillGridWithCarColours` stay original (`orig!`).
//!
//! `tMenuPanel` (0x418): +0x00 grid colours, +0x40 type (0 standard, 1 grid), +0x41 row texts
//! `GxtChar[4][12][10]`, +0x224 / +0x2E4 row numbers `int32[4][12]` (two per row), +0x3A4 column headers
//! `GxtChar[4][10]`, +0x3CC title, +0x3E2 `bool[12]` row active, +0x3EE column orientation `uint8[4]`,
//! +0x3F2 column flags `uint8[4]`, +0x3F6 row count, +0x3F7 column count, +0x3F8 interactive, +0x3F9,
//! +0x3FC column widths `float[4]`, +0x40C x, +0x410 y, +0x414 background, +0x415 active row,
//! +0x416 accepted row (-99 = none).
//!
//! Byte arguments are taken as the dwords the callers push: where the original passes an argument slot
//! on unchanged (tail jumps, `Process`), the port passes the same dword.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, x};
use crate::menu::{CURRENT_MENU_IN_USE, NUM_MENUS_IN_USE};

/// No globals of its own (`menu.rs` declares the menu tables).
pub const GLOBALS: &[GlobalDesc] = &[];

const MENU_NUMBER: usize = 0xBA82D8;
const MENU_IN_USE: usize = 0xBA82E0;

orig! {
    /// `operator new(uint32)` (0x82119A)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `CMenuSystem::SwitchOffMenu(uint8)` (0x580750)
    pub fn switch_off_menu_call(menu: u32) = cdecl 0x580750 => unported;
    /// `CMenuSystem::InputStandardMenu(uint8)` (0x580800)
    pub fn input_standard_menu(menu: u32) = cdecl 0x580800 => input_standard_menu_port;
    /// `CMenuSystem::InputGridMenu(uint8)` (0x580BD0)
    pub fn input_grid_menu(menu: u32) = cdecl 0x580BD0 => unported;
    /// `CMenuSystem::DisplayStandardMenu(uint8, bool)` (0x580E00)
    pub fn display_standard_menu(menu: u32, bright: u32) = cdecl 0x580E00 => unported;
    /// `CMenuSystem::DisplayGridMenu(uint8, bool)` (0x5816E0)
    pub fn display_grid_menu(menu: u32, bright: u32) = cdecl 0x5816E0 => unported;
    /// `CMenuSystem::Input(uint8)` (0x5825D0)
    pub fn input_call(menu: u32) = cdecl 0x5825D0 => input;
    /// `CMenuSystem::FillGridWithCarColours(uint8)` (0x5820E0)
    pub fn fill_grid_with_car_colours(menu: u32) = cdecl 0x5820E0 => unported;
    /// `CMenuSystem::ActivateItems(uint8 menu, bool x12)` (0x581990)
    #[allow(clippy::too_many_arguments)]
    pub fn activate_items(menu: u32, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32) = cdecl 0x581990 => unported;
}

/// `MenuNumber[i]` (`movzx` index).
fn panel(i: u32) -> GPtr<u8> {
    g::<GPtr<u8>>(MENU_NUMBER + (i as usize & 0xFF) * 4).read()
}

/// `MenuInUse[i]`, indexed as the bytes do (`movzx` or `movsx` of the byte, per site).
fn in_use_u(i: u32) -> u8 {
    g::<u8>(MENU_IN_USE + (i as usize & 0xFF)).read()
}
fn in_use_s(i: u32) -> u8 {
    g::<u8>(MENU_IN_USE.wrapping_add(i as u8 as i8 as isize as usize)).read()
}

/// Unbounded `strcpy` (the inlined byte loop, NUL included).
fn copy_str(dst: GPtr<u8>, src: GPtr<u8>) {
    let mut k = 0isize;
    loop {
        let c = src.byte_add(k).read();
        dst.byte_add(k).write(c);
        if c == 0 {
            break;
        }
        k += 1;
    }
}

/// `src` copied to `dst`, or `dst[0] = 0` for a null `src`.
fn set_text(dst: GPtr<u8>, src: GPtr<u8>) {
    if src.is_null() {
        dst.write(0);
    } else {
        copy_str(dst, src);
    }
}

/// The inlined row count: `NumRows` = 1 + the last row of column 0 with a text (0 when none).
fn recount_rows(p: GPtr<u8>) {
    wr::<u8>(p, 0x3F6, 0);
    let mut d = 0u8;
    for r in 0..12 {
        d = d.wrapping_add(1);
        if rd::<u8>(p, 0x41 + r * 10) != 0 {
            wr::<u8>(p, 0x3F6, rd::<u8>(p, 0x3F6).wrapping_add(d));
            d = 0;
        }
    }
}

/// `CMenuSystem::CheckForAccept(uint8 menu)` (0x5807C0): the accepted row while the menu is in use, else
/// -99 (`mov al, 0x9D`; only AL is set).
#[hook(addr = 0x5807C0, conv = cdecl, name = "CMenuSystem::CheckForAccept", expect = "0F B6 44 24 04")]
pub fn check_for_accept(menu: u32) -> u8 {
    let used = in_use_u(menu);
    check_for_accept_with(used, if used != 0 { panel(menu) } else { GPtr::from_addr(0) })
}

/// [`check_for_accept`] with `MenuInUse[menu]` and the panel as parameters (L1).
pub fn check_for_accept_with(in_use: u8, p: GPtr<u8>) -> u8 {
    if in_use != 0 { rd::<u8>(p, 0x416) } else { 0x9D }
}

/// `CMenuSystem::InsertOneMenuItemWithNumber(uint8 menu, uint8 column, uint8 row, const char* key,
/// int32 n1, int32 n2)` (0x581D70): sets one row text (or clears it) and its two numbers, then recounts
/// the rows.
#[hook(addr = 0x581D70, conv = cdecl, name = "CMenuSystem::InsertOneMenuItemWithNumber", expect = "0F B6 44 24 08")]
pub fn insert_one_menu_item_with_number(menu: u32, column: u32, row: u32, key: GPtr<u8>, n1: i32, n2: i32) {
    insert_one_menu_item_with_number_with(panel(menu), column, row, key, n1, n2);
}

/// [`insert_one_menu_item_with_number`] on a given panel (L1; the original rereads `MenuNumber[menu]`,
/// which nothing changes in between).
pub fn insert_one_menu_item_with_number_with(p: GPtr<u8>, column: u32, row: u32, key: GPtr<u8>, n1: i32, n2: i32) {
    let idx = (row & 0xFF) as isize + (column & 0xFF) as isize * 12;
    set_text(p.byte_add(0x41 + idx * 10), key);
    wr::<i32>(p, 0x224 + idx * 4, n1);
    wr::<i32>(p, 0x2E4 + idx * 4, n2);
    recount_rows(p);
}

/// `CMenuSystem::SetColumnOrientation(uint8 menu, uint8 column, eFontAlignment)` (0x582080).
#[hook(addr = 0x582080, conv = cdecl, name = "CMenuSystem::SetColumnOrientation", expect = "0F B6 44 24 04")]
pub fn set_column_orientation(menu: u32, column: u32, orientation: u32) {
    set_column_orientation_with(panel(menu), column, orientation);
}

/// [`set_column_orientation`] on a given panel (L1).
pub fn set_column_orientation_with(p: GPtr<u8>, column: u32, orientation: u32) {
    wr::<u8>(p, 0x3EE + (column & 0xFF) as isize, orientation as u8);
}

/// `CMenuSystem::SetActiveMenuItem(uint8 menu, uint8 row)` (0x5820C0).
#[hook(addr = 0x5820C0, conv = cdecl, name = "CMenuSystem::SetActiveMenuItem", expect = "0F B6 44 24 04")]
pub fn set_active_menu_item(menu: u32, row: u32) {
    set_active_menu_item_with(panel(menu), row);
}

/// [`set_active_menu_item`] on a given panel (L1).
pub fn set_active_menu_item_with(p: GPtr<u8>, row: u32) {
    wr::<u8>(p, 0x415, row as u8);
}

/// `CMenuSystem::GetCarColourFromGrid(uint8 menu, uint8 index)` (0x5822B0): byte `index` of the panel
/// (the grid colours); only AL is set.
#[hook(addr = 0x5822B0, conv = cdecl, name = "CMenuSystem::GetCarColourFromGrid", expect = "0F B6 44 24 04")]
pub fn get_car_colour_from_grid(menu: u32, index: u32) -> u8 {
    get_car_colour_from_grid_with(panel(menu), index)
}

/// [`get_car_colour_from_grid`] on a given panel (L1).
pub fn get_car_colour_from_grid_with(p: GPtr<u8>, index: u32) -> u8 {
    rd::<u8>(p, (index & 0xFF) as isize)
}

/// `CMenuSystem::Initialise()` (0x5822D0): switches off both menus that are in use, then
/// `CurrentMenuInUse = 0`. Not hookable: the loop at +3 (0x5822D3, `jb` from 0x5822EA) branches into the
/// 5-byte patch; reached through `orig!` only.
pub fn initialise() {
    for i in 0..2u32 {
        if g::<u8>(MENU_IN_USE + i as usize).read() != 0 {
            switch_off_menu_call(i);
        }
    }
    CURRENT_MENU_IN_USE.set(0);
}

/// `CMenuSystem::Input(uint8 menu)` (0x5825D0): `InputStandardMenu` / `InputGridMenu` by the panel type
/// (tail jumps passing the argument dword on).
#[hook(addr = 0x5825D0, conv = cdecl, name = "CMenuSystem::Input", expect = "8B 4C 24 04 0F B6 C1")]
pub fn input(menu: u32) {
    match rd::<u8>(panel(menu), 0x40) {
        0 => input_standard_menu(menu),
        1 => input_grid_menu(menu),
        _ => {}
    }
}

/// `CMenuSystem::CreateNewMenu(eMenuType type, const char* title, float x, float y, float width,
/// int8 columns, bool interactive, bool background, eFontAlignment alignment)` (0x582300): allocates a
/// panel in slot `num_menus_in_use`, marks it in use and fills it per type (grid: width, at most 8
/// columns, rows = columns², `FillGridWithCarColours`; standard: at most 4 columns, title, every item
/// active, per-column width / alignment / flags 0xFF, texts cleared). Returns the slot (AL).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x582300, conv = cdecl, name = "CMenuSystem::CreateNewMenu", expect = "83 EC 08 53 8A 1D E3 82 BA 00")]
pub fn create_new_menu(
    kind: u32,
    title: GPtr<u8>,
    px: u32,
    py: u32,
    width: u32,
    columns: u32,
    interactive: u32,
    background: u32,
    alignment: u32,
) -> u8 {
    let slot = NUM_MENUS_IN_USE.get();
    let p0 = operator_new(0x418);
    let si = usize::from(slot);
    g::<GPtr<u8>>(MENU_NUMBER + si * 4).write(p0);
    g::<u8>(MENU_IN_USE + si).write(1);
    wr::<u8>(p0, 0x40, kind as u8);
    let p = || g::<GPtr<u8>>(MENU_NUMBER + si * 4).read();
    match kind as u8 {
        1 => {
            wr::<u32>(p(), 0x3FC, width);
            wr::<u8>(p(), 0x414, u8::from(background as u8 != 0));
            wr::<u8>(p(), 0x3F8, interactive as u8);
            wr::<u8>(p(), 0x3F9, 1);
            wr::<u8>(p(), 0x3F7, (columns as u8).min(8));
            wr::<u32>(p(), 0x40C, px);
            wr::<u32>(p(), 0x410, py);
            wr::<u8>(p(), 0x416, 0x9D);
            wr::<u8>(p(), 0x415, 0);
            let q = p();
            let c = rd::<u8>(q, 0x3F7);
            wr::<u8>(q, 0x3F6, c.wrapping_mul(c));
            fill_grid_with_car_colours(u32::from(slot));
            NUM_MENUS_IN_USE.set(NUM_MENUS_IN_USE.get().wrapping_add(1));
        }
        0 => {
            wr::<u8>(p(), 0x3F6, 0);
            wr::<u8>(p(), 0x3F7, (columns as u8).min(4));
            wr::<u8>(p(), 0x415, 0);
            wr::<u8>(p(), 0x3F8, interactive as u8);
            wr::<u8>(p(), 0x3F9, 1);
            wr::<u32>(p(), 0x40C, px);
            wr::<u32>(p(), 0x410, py);
            wr::<u8>(p(), 0x414, u8::from(background as u8 != 0));
            set_text(p().byte_add(0x3CC), title);
            wr::<u8>(p(), 0x416, 0x9D);
            activate_items(u32::from(slot), 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1);
            // `fld` / `fstp` of the width (a signalling NaN is stored quiet)
            let w = x(f32::from_bits(width)).to_f32();
            let mut t = 0isize;
            for c in 0..4isize {
                wr::<f32>(p(), 0x3FC + c * 4, w);
                wr::<u8>(p(), 0x3EE + c, alignment as u8);
                wr::<u8>(p(), 0x3F2 + c, 0xFF);
                wr::<u8>(p(), 0x3A4 + c * 10, 0);
                for r in 0..12 {
                    wr::<u8>(p(), 0x41 + t, 0);
                    wr::<u8>(p(), 0x3E2 + r, 0);
                    t += 10;
                }
            }
            NUM_MENUS_IN_USE.set(NUM_MENUS_IN_USE.get().wrapping_add(1));
        }
        _ => NUM_MENUS_IN_USE.set(NUM_MENUS_IN_USE.get().wrapping_add(1)),
    }
    slot
}

/// `CMenuSystem::InsertMenu(uint8 menu, uint8 column, const char* header, const char* rows[12]...)`
/// (0x581E00; the entry jumps to a 0x40157F chunk that loads the menu index): sets the column header and
/// its 12 row texts (null clears), resets both numbers of every row of the column to -1, and recounts the
/// rows when the column is 0.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x581E00, conv = cdecl, name = "CMenuSystem::InsertMenu", expect = "E9 7A F7 E7 FF 8B 54 24 0C")]
pub fn insert_menu(
    menu: u32,
    column: u32,
    header: GPtr<u8>,
    r0: GPtr<u8>,
    r1: GPtr<u8>,
    r2: GPtr<u8>,
    r3: GPtr<u8>,
    r4: GPtr<u8>,
    r5: GPtr<u8>,
    r6: GPtr<u8>,
    r7: GPtr<u8>,
    r8: GPtr<u8>,
    r9: GPtr<u8>,
    r10: GPtr<u8>,
    r11: GPtr<u8>,
) {
    insert_menu_with(panel(menu), column, header, [r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11]);
}

/// [`insert_menu`] on a given panel (L1).
pub fn insert_menu_with(p: GPtr<u8>, column: u32, header: GPtr<u8>, rows: [GPtr<u8>; 12]) {
    let col = (column & 0xFF) as isize;
    set_text(p.byte_add(0x3A4 + col * 10), header);
    for (r, s) in rows.iter().enumerate() {
        set_text(p.byte_add(col * 0x78 + 0x41 + r as isize * 10), *s);
    }
    for r in 0..12isize {
        let o = col * 0x30 + 0x2E4 + r * 4;
        wr::<i32>(p, o - 0xC0, -1);
        wr::<i32>(p, o, -1);
    }
    if column as u8 == 0 {
        recount_rows(p);
    }
}

/// Draws a menu by its panel type (`DisplayStandardMenu` / `DisplayGridMenu`).
fn display(menu: u32, bright: u32) {
    match rd::<u8>(panel(menu), 0x40) {
        0 => display_standard_menu(menu, bright),
        1 => display_grid_menu(menu, bright),
        _ => {}
    }
}

/// `CMenuSystem::Process(int8 menu)` (0x582630): for -99, both other in-use menus are drawn dimmed and
/// the current one bright, and an interactive current menu gets its input (tail jump to `Input*Menu`);
/// otherwise only that menu is drawn bright and, when interactive, gets `Input`. `MenuInUse` is indexed
/// with the sign-extended byte, `MenuNumber` with the zero-extended one, as the bytes do. The menu dwords
/// passed on keep the argument's upper bytes (for -99 the loop index replaces only the low byte of the
/// argument slot, the current menu is passed zero-extended).
#[hook(addr = 0x582630, conv = cdecl, name = "CMenuSystem::Process", expect = "53 8B 5C 24 08")]
pub fn process(menu: u32) {
    if menu as u8 != 0x9D {
        if in_use_s(menu) == 0 {
            return;
        }
        display(menu, 1);
        let q: GPtr<u8> = g::<GPtr<u8>>(MENU_NUMBER.wrapping_add((menu as u8 as i8 as isize * 4) as usize)).read();
        if rd::<u8>(q, 0x3F8) != 0 {
            input_call(menu);
        }
        return;
    }
    let mut slot_arg = menu;
    let mut cur = CURRENT_MENU_IN_USE.get();
    for i in 0..2u32 {
        slot_arg = (slot_arg & !0xFF) | i;
        if i as i32 != i32::from(cur as i8) && g::<u8>(MENU_IN_USE + i as usize).read() != 0 {
            let t = rd::<u8>(g::<GPtr<u8>>(MENU_NUMBER + i as usize * 4).read(), 0x40);
            match t {
                0 => display_standard_menu(slot_arg, 0),
                1 => display_grid_menu(slot_arg, 0),
                _ => {}
            }
            if t <= 1 {
                cur = CURRENT_MENU_IN_USE.get();
            }
        }
    }
    let c = u32::from(cur);
    if in_use_s(c) == 0 {
        return;
    }
    let t = rd::<u8>(panel(c), 0x40);
    match t {
        0 => display_standard_menu(c, 1),
        1 => display_grid_menu(c, 1),
        _ => {}
    }
    if t <= 1 {
        cur = CURRENT_MENU_IN_USE.get();
    }
    let c = u32::from(cur);
    let q: GPtr<u8> = g::<GPtr<u8>>(MENU_NUMBER.wrapping_add((cur as i8 as isize * 4) as usize)).read();
    if rd::<u8>(q, 0x3F8) == 0 {
        return;
    }
    match rd::<u8>(panel(c), 0x40) {
        0 => input_standard_menu(c),
        1 => input_grid_menu(c),
        _ => {}
    }
}

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetAnaloguePadUp()` (0x540950, AL)
    pub fn pad_analogue_up(pad: GPtr<u8>) -> u8 = thiscall 0x540950 => unported;
    /// `CPad::GetAnaloguePadDown()` (0x540980, AL)
    pub fn pad_analogue_down(pad: GPtr<u8>) -> u8 = thiscall 0x540980 => unported;
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float volume, float speed)` (0x506EA0)
    pub fn report_frontend_audio_event(this: GPtr<u8>, event: u32, volume: u32, speed: u32) = thiscall 0x506EA0 => unported;
}

/// `AudioEngine`
const AUDIO_ENGINE: usize = 0xB6BC90;
/// Two byte flags tested before the menu sounds and the absolute pad words (0xB7CB49 / 0xB7CB48:
/// `CPad` / `CTimer` state as gta-reversed names them; read raw here).
const FLAG_A: usize = 0xB7CB49;
const FLAG_B: usize = 0xB7CB48;

fn flags_set() -> bool {
    g::<u8>(FLAG_A).read() != 0 || g::<u8>(FLAG_B).read() != 0
}

fn sound(event: u32) {
    report_frontend_audio_event(g(AUDIO_ENGINE), event, 0, 0x3F80_0000);
}

/// A pad button "just pressed": word `new` of `GetPad(0)` set and word `old` clear.
fn just(newo: isize, oldo: isize) -> bool {
    let p = get_pad(0);
    rd::<u16>(p, newo) != 0 && rd::<u16>(p, oldo) == 0
}

/// The alternative input: with one of the two flags set, `GetPad(0)` is called twice and the absolute
/// words `a` (set) and `b` (clear) are tested.
fn alt(a: usize, b: usize) -> bool {
    if !flags_set() {
        return false;
    }
    get_pad(0);
    if g::<u16>(a).read() == 0 {
        return false;
    }
    get_pad(0);
    g::<u16>(b).read() == 0
}

/// Row `r` (signed byte) is active and has a text.
fn row_ok(p: GPtr<u8>, r: i8) -> bool {
    let r = isize::from(r);
    rd::<u8>(p, 0x3D6 + r) != 0 && rd::<u8>(p, 0x41 + r * 10) != 0
}

/// `CMenuSystem::HighlightOneItem(uint8 menu, uint8 row, bool highlight)` (0x581C10): sets the row's
/// highlight flag (+0x3E2) and marks it active (+0x3D6).
#[hook(addr = 0x581C10, conv = cdecl, name = "CMenuSystem::HighlightOneItem", expect = "0F B6 44 24 04")]
pub fn highlight_one_item(menu: u32, row: u32, highlight: u32) {
    highlight_one_item_with(panel(menu), row, highlight);
}

/// [`highlight_one_item`] on a given panel (L1).
pub fn highlight_one_item_with(p: GPtr<u8>, row: u32, highlight: u32) {
    let r = (row & 0xFF) as isize;
    wr::<u8>(p, 0x3E2 + r, highlight as u8);
    wr::<u8>(p, 0x3D6 + r, 1);
}

/// `CMenuSystem::InputStandardMenu(uint8 menu)` (0x580800): normalises `CurrentMenuInUse` to a menu in
/// use, plays the cross sound, accepts the active row on the accept button (or the alternative input),
/// clears the accepted row when it is not the active one, and with more than one row moves the active row
/// up / down (analogue pad, the d-pad when +0x3F9 is set, or the alternative input) to the next active row
/// with a text, wrapping at both ends. Sounds go to `AudioEngine.ReportFrontendAudioEvent(event, 0, 1)`
/// unless one of the two flags at 0xB7CB48 / 0xB7CB49 is set (the up / down sound always plays).
#[hook(addr = 0x580800, conv = cdecl, name = "CMenuSystem::InputStandardMenu", expect = "A0 E2 82 BA 00")]
pub fn input_standard_menu_port(menu: u32) {
    let cur = |v: u8| CURRENT_MENU_IN_USE.set(v);
    let used = |v: u8| g::<u8>(MENU_IN_USE.wrapping_add(v as i8 as isize as usize)).read() != 0;
    let mut al = CURRENT_MENU_IN_USE.get();
    if (al as i8) < 0 {
        al = 1;
        cur(al);
    }
    if !used(al) {
        while (al as i8) > 0 {
            al = al.wrapping_sub(1);
            if used(al) {
                break;
            }
        }
        cur(al);
    }
    if (al as i8) >= 2 {
        al = 0;
        cur(al);
    }
    if !used(al) {
        while (al as i8) < 2 {
            al = al.wrapping_add(1);
            if used(al) {
                break;
            }
        }
        cur(al);
    }
    if just(0x1E, 0x4E) && !flags_set() {
        sound(2);
    }
    if just(0x20, 0x50) || alt(0xB733EA, 0xB7317A) {
        if !flags_set() {
            sound(1);
        }
        let p = panel(menu);
        let r = rd::<u8>(p, 0x415);
        if rd::<u8>(p, 0x3D6 + isize::from(r as i8)) != 0 {
            wr::<u8>(p, 0x416, r);
        }
    }
    let p = panel(menu);
    if rd::<u8>(p, 0x416) != rd::<u8>(p, 0x415) {
        wr::<u8>(p, 0x416, 0x9D);
    }
    if rd::<u8>(panel(menu), 0x3F6) <= 1 {
        return;
    }
    let dpad = |n: isize, o: isize| rd::<u8>(panel(menu), 0x3F9) != 0 && just(n, o);
    if pad_analogue_up(get_pad(0)) != 0 || dpad(0x10, 0x40) || alt(0xB733B6, 0xB73146) {
        sound(3);
        let mut c = rd::<u8>(panel(menu), 0x415);
        loop {
            c = c.wrapping_sub(1);
            wr::<u8>(panel(menu), 0x415, c);
            c = rd::<u8>(panel(menu), 0x415);
            if row_ok(panel(menu), c as i8) || (c as i8) < 0 {
                break;
            }
        }
    }
    if pad_analogue_down(get_pad(0)) != 0 || dpad(0x12, 0x42) || alt(0xB733B8, 0xB73148) {
        sound(3);
        let p = panel(menu);
        wr::<u8>(p, 0x415, rd::<u8>(p, 0x415).wrapping_add(1));
        loop {
            let p = panel(menu);
            let d = rd::<u8>(p, 0x415);
            if row_ok(p, d as i8) || i32::from(d as i8) >= i32::from(rd::<u8>(p, 0x3F6)) {
                break;
            }
            wr::<u8>(p, 0x415, d.wrapping_add(1));
        }
    }
    if (rd::<u8>(panel(menu), 0x415) as i8) < 0 {
        let mut c = rd::<u8>(panel(menu), 0x3F6);
        loop {
            c = c.wrapping_sub(1);
            wr::<u8>(panel(menu), 0x415, c);
            c = rd::<u8>(panel(menu), 0x415);
            if row_ok(panel(menu), c as i8) || (c as i8) < 0 {
                break;
            }
        }
    }
    let p = panel(menu);
    if i32::from(rd::<u8>(p, 0x415) as i8) >= i32::from(rd::<u8>(p, 0x3F6)) {
        wr::<u8>(p, 0x415, 0);
        loop {
            let p = panel(menu);
            let d = rd::<u8>(p, 0x415);
            if row_ok(p, d as i8) || i32::from(d as i8) >= i32::from(rd::<u8>(p, 0x3F6)) {
                break;
            }
            wr::<u8>(p, 0x415, d.wrapping_add(1));
        }
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_CHECK_FOR_ACCEPT,
    HOOK_INSERT_ONE_MENU_ITEM_WITH_NUMBER,
    HOOK_SET_COLUMN_ORIENTATION,
    HOOK_SET_ACTIVE_MENU_ITEM,
    HOOK_GET_CAR_COLOUR_FROM_GRID,
    HOOK_INPUT,
    HOOK_CREATE_NEW_MENU,
    HOOK_INSERT_MENU,
    HOOK_PROCESS,
    HOOK_HIGHLIGHT_ONE_ITEM,
    HOOK_INPUT_STANDARD_MENU_PORT,
];
