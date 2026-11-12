//! The controller setup screens of `CMenuManager` (PORT-MENU): `DrawControllerSetupScreen` 0x57F300,
//! `DrawControllerBound` 0x57E6E0, `DrawControllerScreenExtraText` 0x57D8D0 and `RedefineScreenUserInput`
//! 0x57EF50. Ported from the 1.0 US bytes; gta-reversed `Frontend/MenuManager_Draw.cpp` /
//! `MenuManager_Input.cpp` as reading aids (the bytes win). Fonts, sprites, text, pad and
//! `CControllerConfigManager` calls stay `orig!`.
//!
//! `this` fields (offsets from the bytes): +0x30 system menu (i8), +0x38 the key code a redefinition
//! writes, +0x54 current item, +0x90 the selected row, +0xB7 the controls page (0 on foot, 1 vehicle),
//! +0xB8 display the mouse, +0xBC / +0xC0 mouse position, +0xD0 control method (0 = mouse + keys),
//! +0xF0 the pointer the redefinition writes through, +0x1AE8 show the combo text, +0x1AE9 "a binding
//! was found" (cleared on an unbound row), +0x1AEA controller error, +0x1AF0 / +0x1AF2 / +0x1AF3 / +0x1AF4
//! the up / down repeat latches, +0x1AF8 / +0x1AFC old mouse position, +0x1B00 mouse-in-bounds state,
//! +0x1B04 current mouse option, +0x1B08 just opened the redefine window, +0x1B09 editing a control,
//! +0x1B0A delete all bindings of the next define, +0x1B0B, +0x1B0C the action being edited, +0x1B14
//! the redefine helper is shown, +0x1B15 the backspace latch.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use crate::common::{f32_of, ftol32, g, rd, wr, x, xi};
use crate::menu_input::get_string_width_st0;
use crate::menu_mgr::{
    audio_report_frontend_event, font_print_string, font_set_color, font_set_edge, font_set_font_style, font_set_orientation,
    font_set_wrapx, get_pad, rgba_ctor, sprite_draw_rect, stretch_x_call, stretch_y_call, text_get,
};

/// No globals of its own (the blink states below are addressed directly, as the bytes do).
pub const GLOBALS: &[GlobalDesc] = &[];

/// `TheText`, `ControlsManager`, `HudColour`, `AudioEngine`
const THE_TEXT: usize = 0xC1B340;
const CONTROLS_MANAGER: usize = 0xB70198;
const HUD_COLOUR: usize = 0xBAB22C;
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `RsGlobal.maximumWidth` / `maximumHeight`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// `CTimer::m_snTimeInMillisecondsPauseMode`
const TIME_PAUSE_MODE: usize = 0xB7CB7C;
/// `DrawControllerBound`'s blink: the state byte (0xBA82A4) and its last toggle time (0xBA82A8)
const BOUND_BLINK: usize = 0xBA82A4;
const BOUND_BLINK_TIME: usize = 0xBA82A8;
/// `DrawControllerScreenExtraText`'s blink: state (0xBA8299) and time (0xBA829C)
const EXTRA_BLINK: usize = 0xBA8299;
const EXTRA_BLINK_TIME: usize = 0xBA829C;
/// `RedefineScreenUserInput`'s up / down repeat time (0xBA82AC)
const REPEAT_TIME: usize = 0xBA82AC;
/// `CPad::NewKeyState` / `OldKeyState` words: enter, return, backspace, up, down, escape
const KEY_ENTER: (usize, usize) = (0xB733CC, 0xB7315C);
const KEY_RETURN: (usize, usize) = (0xB733EA, 0xB7317A);
const KEY_BACK: (usize, usize) = (0xB733E4, 0xB73174);
const KEY_UP: usize = 0xB733B6;
const KEY_DOWN: usize = 0xB733B8;
const KEY_ESC: (usize, usize) = (0xB733A8, 0xB73138);
/// `CPad::NewMouseControllerState` / `OldMouseControllerState` bytes: left button, wheel up, wheel down
const MOUSE_LMB: (usize, usize) = (0xB73418, 0xB7342C);
const MOUSE_WHEEL_UP: (usize, usize) = (0xB7341B, 0xB7342F);
const MOUSE_WHEEL_DOWN: (usize, usize) = (0xB7341C, 0xB73430);
/// The on-foot / in-vehicle row -> action index tables (dwords 0x865598 / 0x865608), copied from the exe
const FOOT_ROWS: [u32; 28] = [0, 2, 3, 12, 13, 11, 10, 4, 5, 6, 7, 14, 15, 16, 22, 23, 24, 26, 27, 28, 29, 34, 36, 37, 38, 39, 42, 1];
const VEHICLE_ROWS: [u32; 25] = [0, 1, 4, 5, 6, 7, 8, 9, 16, 10, 17, 18, 19, 20, 21, 22, 25, 34, 35, 36, 37, 30, 31, 32, 33];

// .rdata strings
/// `"FEC_CMP"`, `"FEC_UNB"`, `"FEC_QUE"`, `"FET_RIG"`, `"FET_CIG"`, `"FET_EIG"`
const S_FEC_CMP: usize = 0x866430;
const S_FEC_UNB: usize = 0x866428;
const S_FEC_QUE: usize = 0x865700;
const S_FET_RIG: usize = 0x866420;
const S_FET_CIG: usize = 0x866418;
const S_FET_EIG: usize = 0x866410;
/// `"FET_SCN"` (mouse + keys) / `"FET_CCN"` (joypad), `"FET_CFT"` (on foot) / `"FET_CCR"` (vehicle), `"FEDS_TB"`
const S_FET_SCN: usize = 0x865468;
const S_FET_CCN: usize = 0x865470;
const S_FET_CFT: usize = 0x866440;
const S_FET_CCR: usize = 0x866448;
const S_FEDS_TB: usize = 0x866438;

/// Float immediates
const F_270: f32 = f32::from_bits(0x4387_0000);
const F_240: f32 = f32::from_bits(0x4370_0000);
const F_260: f32 = f32::from_bits(0x4382_0000);
const F_75: f32 = f32::from_bits(0x4296_0000);
const F_100: f32 = f32::from_bits(0x42C8_0000);
const F_20: f32 = f32::from_bits(0x41A0_0000);
const F_0_3: f32 = f32::from_bits(0x3E99_999A);
const F_0_6: f32 = f32::from_bits(0x3F19_999A);
const F_0_4: f32 = f32::from_bits(0x3ECC_CCCD);
const F_0_9: f32 = f32::from_bits(0x3F66_6666);
const F_1_7: f32 = f32::from_bits(0x3FD9_999A);
const F_0_7: f32 = f32::from_bits(0x3F33_3333);
const F_1: f32 = f32::from_bits(0x3F80_0000);
const F_11: f32 = f32::from_bits(0x4130_0000);
const F_48: f32 = f32::from_bits(0x4240_0000);
const F_50: f32 = f32::from_bits(0x4248_0000);
const F_40: f32 = f32::from_bits(0x4220_0000);
const F_600: f32 = f32::from_bits(0x4416_0000);
const F_35: f32 = f32::from_bits(0x420C_0000);
const F_15: f32 = f32::from_bits(0x4170_0000);
const F_33: f32 = f32::from_bits(0x4204_0000);
const F_10: f32 = f32::from_bits(0x4120_0000);
const F_38: f32 = f32::from_bits(0x4218_0000);

orig! {
    /// `CControllerConfigManager::GetControllerSettingTextWithOrderNumber(eControllerAction, eContSetOrder)`
    /// (0x531E20, thiscall)
    fn ccm_setting_text(this: GPtr<u8>, action: i32, order: u32) -> GPtr<u8> = thiscall 0x531E20 => unported;
    /// `CControllerConfigManager::GetButtonComboText(eControllerAction)` (0x52F560, thiscall)
    fn ccm_button_combo_text(this: GPtr<u8>, action: i32) -> GPtr<u8> = thiscall 0x52F560 => unported;
    /// `CPad::GetAnaloguePadUp()` / `GetAnaloguePadDown()` (0x540950 / 0x540980; ECX = the pad, unread)
    fn pad_analogue_up() -> u8 = cdecl 0x540950 => unported;
    fn pad_analogue_down() -> u8 = cdecl 0x540980 => unported;
    /// `CHudColours::GetRGB(out, colour)` (0x58FEA0, thiscall, hidden return)
    fn hud_colours_get_rgb(this: GPtr<u8>, out: GPtr<u8>, colour: u32) -> GPtr<u8> = thiscall 0x58FEA0 => crate::hud::get_rgb;
    /// `CFont::SetScale(float w, float h)` (0x719380)
    fn font_set_scale(w: f32, h: f32) = cdecl 0x719380 => font_set_scale_rust;
    /// `CMenuManager::DisplayHelperText(const char*)` (0x57E240, thiscall)
    fn display_helper_text_call(this: GPtr<u8>, key: GPtr<u8>) = thiscall 0x57E240 => crate::menu_mgr::display_helper_text;
    /// `CMenuManager::DrawControllerBound(uint16 y, bool onlyCheck)` (0x57E6E0, thiscall)
    fn draw_controller_bound_call(this: GPtr<u8>, y: u32, only_check: u32) = thiscall 0x57E6E0 => draw_controller_bound;
}

/// The Rust path of [`font_set_scale`] (phases B/C).
#[allow(dead_code)]
fn font_set_scale_rust(w: f32, h: f32) {
    crate::font::set_scale(w.to_bits(), h.to_bits());
}

fn b8(p: GPtr<u8>, off: isize) -> u8 {
    rd(p, off)
}

fn w() -> i32 {
    g::<i32>(RS_MAX_WIDTH).read()
}

fn h() -> i32 {
    g::<i32>(RS_MAX_HEIGHT).read()
}

fn now() -> u32 {
    g::<u32>(TIME_PAUSE_MODE).read()
}

fn text(key: usize) -> GPtr<u8> {
    text_get(g(THE_TEXT), g(key))
}

/// `CRGBA(r, g, b, a)` on a stack temporary, the dword `SetColor` / `DrawRect` get.
fn rgba(r: u32, gg: u32, bb: u32, a: u32) -> u32 {
    let mut c = [0u8; 4];
    rd::<u32>(rgba_ctor(GPtr::from_raw(c.as_mut_ptr()), r, gg, bb, a), 0)
}

fn set_color(r: u32, gg: u32, bb: u32, a: u32) {
    font_set_color(rgba(r, gg, bb, a));
}

/// `SetScale(StretchX(sx), StretchY(sy))` (StretchY is called first, as the pushes order it).
fn set_scale(this: GPtr<u8>, sx: f32, sy: f32) {
    let hh = f32_of(stretch_y_call(this, sy));
    let ww = f32_of(stretch_x_call(this, sx));
    font_set_scale(ww, hh);
}

/// `SetWrapx(StretchX(100) + W)` (`fiadd`).
fn set_wrap(this: GPtr<u8>) {
    font_set_wrapx(f32_of(stretch_x_call(this, F_100).fadd(xi(w()))).to_bits());
}

/// A key word / mouse byte just pressed: new set, old clear.
fn word_just(k: (usize, usize)) -> bool {
    g::<u16>(k.0).read() != 0 && g::<u16>(k.1).read() == 0
}

fn byte_just(k: (usize, usize)) -> bool {
    g::<u8>(k.0).read() != 0 && g::<u8>(k.1).read() == 0
}

fn below(c: Cmp) -> bool {
    !c.test_parity(0x05)
}

fn above(c: Cmp) -> bool {
    !c.test_any(0x41)
}

/// The number of rows of the current page (+0xB7): on foot 22 (mouse + keys) or 28, in a vehicle 25; any
/// other page: `other` (each caller's own fallback, see there).
fn row_count(this: GPtr<u8>, other: u8) -> u8 {
    match b8(this, 0xB7) {
        0 => {
            if b8(this, 0xD0) != 0 {
                0x1C
            } else {
                0x16
            }
        }
        1 => 0x19,
        _ => other,
    }
}

/// The row height of the current page: on foot 15 (mouse + keys) or 11, in a vehicle 13; any other page:
/// `other`.
fn row_height(this: GPtr<u8>, other: u8) -> u8 {
    match b8(this, 0xB7) {
        0 => {
            if b8(this, 0xD0) == 0 {
                0xF
            } else {
                0xB
            }
        }
        1 => 0xD,
        _ => other,
    }
}

/// The action `DrawControllerBound` shows on row `row` (the two jump tables 0x57EEA4 / 0x57EDF8 over
/// the row tables): -1 none, -2 "combo", else the `eControllerAction`. `None` keeps the previous row's
/// action (pages other than 0 / 1).
fn bound_action(this: GPtr<u8>, row: usize) -> Option<i32> {
    let d0 = b8(this, 0xD0) != 0;
    match b8(this, 0xB7) {
        0 => Some(match FOOT_ROWS[row] {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            4 => 4,
            5 => 5,
            6 => 6,
            7 => 7,
            10 => 0x2F,
            11 => 0x30,
            12 => 0x31,
            13 => 0x32,
            14 => 8,
            15 => 9,
            16 => 0xA,
            22 => 0xB,
            23 => 0xC,
            24 => 0xD,
            26 => 0x2D,
            27 => 0xF,
            28 => 0x10,
            29 => 0x11,
            34 => 0xE,
            36 => {
                if d0 {
                    0x20
                } else {
                    -1
                }
            }
            37 => {
                if d0 {
                    0x21
                } else {
                    -1
                }
            }
            38 => 0x33,
            39 => 0x34,
            _ => 0x2C, // 42
        }),
        1 => Some(match VEHICLE_ROWS[row] {
            0 => 0x12,
            1 => 0x13,
            4 => 0x18,
            5 => 0x19,
            6 => 0x14,
            7 => 0x15,
            8 => 0x16,
            9 => 0x17,
            10 => 0x2F,
            16 => 0xA,
            17 => 0x1A,
            18 => 0x1B,
            19 => 0x1C,
            20 => 0x1D,
            21 => 0x1E,
            22 => 0xB,
            25 => 0x1F,
            30 => 0x26,
            31 => 0x27,
            32 => 0x29,
            33 => 0x28,
            34 => 0x24,
            35 => 0x25,
            36 => 0x22,
            _ => 0x23, // 37
        }),
        _ => None,
    }
}

/// `CMenuManager::DrawControllerBound(uint16 y, bool onlyCheck)` (0x57E6E0, thiscall, `ret 8`): the bound
/// keys of every row of the current page from x = `StretchX(270)`, at y = `ftol(StretchY(rowHeight * row +
/// y))` (the first row `ftol(StretchY(y))`).
///
/// Per row: white text; the selected row (+0x90) is highlighted (unless `onlyCheck`) with the rect
/// `{StretchX(260), StretchY(y0 + 10), W - StretchX(20), StretchY(y0)}`, `y0 = (i16)(rowHeight * sel + y + 1)`
/// (16-bit products), colour (0xAC, 0xCB, 0xF1, 0xFF). Font: left, scale (0.3, 0.6), style 1, wrap
/// `StretchX(100) + W`. Unless the action is -1: the orders 1..4 (stopped early while +0x1B0B is set on the
/// selected row) print `GetControllerSettingTextWithOrderNumber(action, order)` and step x by
/// `ftol(StretchX(75) + x)` (`fiadd`); then -2 prints `FEC_CMP` in black, and a row without any binding clears
/// +0x1AE9 and prints `FEC_UNB` in (200, 50, 50) (not on the selected row while editing). On the selected
/// row: actions -1 / -2 show the helper `FET_EIG`; others set +0x1B0C, and while editing (+0x1B09) blink
/// `FEC_QUE` in black (150 ms, 0xBA82A4 / 0xBA82A8; the text is fetched even when not printed) and show
/// `FET_RIG` (+0x1B0A clear) or `FET_CIG` with +0x1B14 = 1; not editing: `FET_CIG`, +0x1B14 = 0, +0x1B0A = 0.
/// `onlyCheck` suppresses every `PrintString` and the highlight, nothing else.
///
/// Uninitialised stack: on a page other than 0 / 1 the original's row height is a stack byte it never
/// wrote and its row count is the `onlyCheck` byte; the port uses row height 0 there (not reachable:
/// +0xB7 is only ever 0 or 1).
#[hook(addr = 0x57E6E0, conv = thiscall, name = "CMenuManager::DrawControllerBound", expect = "83 EC 54 53 55",
       unverified = "menu-3", route = "split", hazards = "heap-writes",
       recs = "menus-map-controls",
       note = "controller setup screen; draws through CFont / CSprite2d")]
pub fn draw_controller_bound(this: GPtr<u8>, y: u32, only_check: u32) {
    let only = only_check as u8 != 0;
    let height = row_height(this, 0);
    let pos_y = (y & 0xFFFF) as i32;
    let pos_yf = f32_of(xi(pos_y));
    let mut row_y = ftol32(stretch_y_call(this, pos_yf));
    let rows = row_count(this, only_check as u8);
    stretch_y_call(this, pos_yf);
    let rows = i32::from(rows);
    if rows <= 0 {
        return;
    }
    let mut action: i32 = 0;
    let mut row = 0i32;
    while row < rows {
        let mut px = ftol32(stretch_x_call(this, F_270));
        set_color(255, 255, 255, 255);
        let mut found = 0;
        if let Some(a) = bound_action(this, row as usize) {
            action = a;
        }
        let sel: i32 = rd(this, 0x90);
        if sel == row && !only {
            let prod = u16::from(height).wrapping_mul(sel as u16);
            let y0 = i32::from(prod.wrapping_add(y as u16).wrapping_add(1) as i16);
            let bottom = f32_of(stretch_y_call(this, f32_of(xi(y0 + 10))));
            let right = f32_of(stretch_x_call(this, F_20).fsubr_m(f32_of(xi(w()))));
            let top = f32_of(stretch_y_call(this, f32_of(xi(y0))));
            let left = f32_of(stretch_x_call(this, F_260));
            let mut rect = [left, bottom, right, top];
            let mut col = [0u8; 4];
            let cp = rgba_ctor(GPtr::from_raw(col.as_mut_ptr()), 0xAC, 0xCB, 0xF1, 0xFF);
            sprite_draw_rect(GPtr::<f32>::from_raw(rect.as_mut_ptr()).cast(), cp);
            set_color(255, 255, 255, 255);
        }
        font_set_orientation(1);
        set_scale(this, F_0_3, F_0_6);
        font_set_font_style(1);
        set_wrap(this);
        if action != -1 {
            for order in 1..5u32 {
                if b8(this, 0x1B0B) != 0 && rd::<i32>(this, 0x90) == row {
                    break;
                }
                let t = ccm_setting_text(g(CONTROLS_MANAGER), action, order);
                if !t.is_null() {
                    found += 1;
                    if !only {
                        font_print_string(f32_of(xi(px)), f32_of(xi(row_y)), t);
                    }
                    px = ftol32(stretch_x_call(this, F_75).fadd(xi(px)));
                }
            }
            if action == -2 {
                set_color(0, 0, 0, 255);
                if !only {
                    font_print_string(f32_of(xi(px)), f32_of(xi(row_y)), text(S_FEC_CMP));
                }
            } else if found == 0 {
                wr::<u8>(this, 0x1AE9, 0);
                let colour = if rd::<i32>(this, 0x90) != row {
                    true
                } else {
                    b8(this, 0x1B09) == 0
                };
                if colour {
                    set_color(200, 50, 50, 255);
                    if !only {
                        font_print_string(f32_of(xi(px)), f32_of(xi(row_y)), text(S_FEC_UNB));
                    }
                }
            }
        }
        if rd::<i32>(this, 0x90) == row {
            if action == -1 || action == -2 {
                display_helper_text_call(this, g(S_FET_EIG));
            } else {
                wr::<i32>(this, 0x1B0C, action);
                if b8(this, 0x1B09) != 0 {
                    let t = now();
                    if t.wrapping_sub(g::<u32>(BOUND_BLINK_TIME).read()) > 150 {
                        g::<u8>(BOUND_BLINK).write(u8::from(g::<u8>(BOUND_BLINK).read() == 0));
                        g::<u32>(BOUND_BLINK_TIME).write(t);
                    }
                    if g::<u8>(BOUND_BLINK).read() != 0 {
                        set_color(0, 0, 0, 255);
                        let q = text(S_FEC_QUE);
                        if !only {
                            font_print_string(f32_of(xi(px)), f32_of(xi(row_y)), q);
                        }
                    }
                    let key = if b8(this, 0x1B0A) == 0 { S_FET_RIG } else { S_FET_CIG };
                    display_helper_text_call(this, g(key));
                    wr::<u8>(this, 0x1B14, 1);
                } else {
                    display_helper_text_call(this, g(S_FET_CIG));
                    wr::<u8>(this, 0x1B14, 0);
                    wr::<u8>(this, 0x1B0A, 0);
                }
            }
        }
        row += 1;
        let next = i32::from(height).wrapping_mul(row).wrapping_add(pos_y);
        row_y = ftol32(stretch_y_call(this, f32_of(xi(next))));
    }
}

/// `CMenuManager::DrawControllerScreenExtraText(int32 y)` (0x57D8D0, thiscall, `ret 4`): per row of the
/// current page, the bound keys from x = `ftol(StretchX(240))` at `StretchY(y)`, x stepping by
/// `ftol(StretchX(75) + x)` (x is kept as the stored float of the printed position); on the current item
/// (+0x54) while editing (+0x1B09) `FEC_QUE` blinks in black (150 ms, 0xBA8299 / 0xBA829C) at the
/// **unstretched** `(float)y`, then the colour (0x4A, 0x5A, 0x6B); y += row height. Then with +0x1AE8:
/// `GetButtonComboText(+0x54)` in (200, 50, 50) at (x, `StretchY(y + 10)`).
///
/// Differences from gta-reversed (the bytes win): the blink row compares with the current item +0x54
/// (gr: the list selection), the blink y is not stretched, the combo y is the y after the last row + 10.
///
/// Uninitialised stack: on a page other than 0 / 1 (not reachable) the original uses the low byte of `y`
/// as row height and has no rows; its combo x is then a stack dword it never wrote, which the port
/// replaces by 0.
#[hook(addr = 0x57D8D0, conv = thiscall, name = "CMenuManager::DrawControllerScreenExtraText", expect = "83 EC 14 8B 44 24 18",
       unverified = "menu-3", route = "split", hazards = "heap-writes",
       recs = "menus-map-controls",
       note = "controller screen text; draws through CFont")]
pub fn draw_controller_screen_extra_text(this: GPtr<u8>, y: i32) {
    let mut y = y;
    let height = i32::from(row_height(this, y as u8));
    let rows = u32::from(row_count(this, 0));
    let mut px_bits: u32 = 0;
    let mut row = 0u32;
    while row < rows {
        let mut px = ftol32(stretch_x_call(this, F_240));
        for order in 1..5u32 {
            let t = ccm_setting_text(g(CONTROLS_MANAGER), row as i32, order);
            if !t.is_null() {
                let xf = f32_of(xi(px));
                let yy = f32_of(stretch_y_call(this, f32_of(xi(y))));
                font_print_string(xf, yy, t);
                px = ftol32(stretch_x_call(this, F_75).fadd_m(xf));
            }
        }
        if row == rd::<u32>(this, 0x54) && b8(this, 0x1B09) != 0 {
            let t = now();
            if t.wrapping_sub(g::<u32>(EXTRA_BLINK_TIME).read()) > 150 {
                g::<u8>(EXTRA_BLINK).write(u8::from(g::<u8>(EXTRA_BLINK).read() == 0));
                g::<u32>(EXTRA_BLINK_TIME).write(t);
            }
            if g::<u8>(EXTRA_BLINK).read() != 0 {
                set_color(0, 0, 0, 255);
                let q = text(S_FEC_QUE);
                font_print_string(f32_of(xi(px)), f32_of(xi(y)), q);
                set_color(0x4A, 0x5A, 0x6B, 0xFF);
            }
        }
        y = y.wrapping_add(height);
        px_bits = px as u32;
        row += 1;
    }
    if b8(this, 0x1AE8) != 0 {
        let t = ccm_button_combo_text(g(CONTROLS_MANAGER), rd(this, 0x54));
        if !t.is_null() {
            set_color(200, 50, 50, 255);
            let yy = f32_of(stretch_y_call(this, f32_of(xi(y.wrapping_add(10)))));
            font_print_string(f32_of(xi(px_bits as i32)), yy, t);
        }
    }
}

/// `CMenuManager::RedefineScreenUserInput(bool* accept, bool* back)` (0x57EF50, thiscall, `ret 8`): the
/// input of the redefine-controls screen (0x26). Nothing while a control is being edited (+0x1B09).
///
/// 1. Rows: on screen 0x26 the page's row count (22 / 28 / 25), else the low byte of the `accept`
///    pointer (as the bytes; only screen 0x26 calls it).
/// 2. Accept: enter or return just pressed with no system menu (+0x30 < 0), or the pad's cross just
///    pressed (`GetPad(0)`): mouse hidden, `*accept = 1`.
/// 3. Backspace just pressed on screen 0x26 with the latch +0x1B15 clear: +0x1B00 = 0x10, editing
///    (+0x1B09), just opened (+0x1B08) and delete-all (+0x1B0A) set, +0xF0 = `this + 0x38`; otherwise
///    the latch is cleared.
/// 4. Every 200 ms of pause-mode time (0xBA82AC) the repeat latches +0x1AF0..+0x1AF4 are cleared.
/// 5. Up (key, analogue, d-pad just pressed: mouse hidden; wheel up just pressed: mouse shown): once per
///    latch (+0x1AF2), the selection +0x90 moves up, wrapping from 0 to rows - 1; no up: latch cleared.
///    Down likewise (+0x1AF3, wheel down), wrapping from rows - 1 to 0.
/// 6. Back: escape or triangle just pressed: mouse hidden, `*back = 1`; the left button just pressed with
///    +0x1B00 == 3 (the back button) sets back too; with +0x1B00 == 4: event 1 and +0x1B00 = 5.
/// 7. On back: with +0x1AE9 set the page flips (+0xB7 = !+0xB7) and `DrawControllerBound(0x45, true)`
///    rechecks; when that cleared +0x1AE9, the controller error +0x1AEA = 1 + (page != 0); with +0x1AE9
///    clear at the start, +0x1AEA = 3 (the original's EBX, loaded with 3 in step 6).
///
/// Differences from gta-reversed (the bytes win): the row count of step 1 off screen 0x26; the latch
/// clear of step 4 also clears +0x1AF0 / +0x1AF1; step 7's error values.
#[hook(addr = 0x57EF50, conv = thiscall, name = "CMenuManager::RedefineScreenUserInput", expect = "56 8B F1 8A 86 09 1B 00 00",
       unverified = "menu-3", route = "L2", hazards = "none",
       recs = "menus-map-controls",
       note = "redefine-controls screen input")]
pub fn redefine_screen_user_input(this: GPtr<u8>, accept: GPtr<u8>, back: GPtr<u8>) {
    if b8(this, 0x1B09) != 0 {
        return;
    }
    let rows = if b8(this, 0x15D) == 0x26 { row_count(this, accept.addr() as u8) } else { accept.addr() as u8 };
    get_pad(0);
    let keys = word_just(KEY_ENTER) || word_just(KEY_RETURN);
    let accepted = (keys && (b8(this, 0x30) as i8) < 0) || {
        let p = get_pad(0);
        rd::<u16>(p, 0x20) != 0 && rd::<u16>(p, 0x50) == 0
    };
    if accepted {
        wr::<u8>(this, 0xB8, 0);
        wr::<u8>(accept, 0, 1);
    }
    get_pad(0);
    if word_just(KEY_BACK) && b8(this, 0x15D) == 0x26 && b8(this, 0x1B15) == 0 {
        wr::<u32>(this, 0x1B00, 0x10);
        wr::<u8>(this, 0x1B09, 1);
        wr::<u8>(this, 0x1B08, 1);
        wr::<u8>(this, 0x1B0A, 1);
        wr::<u32>(this, 0xF0, this.byte_add(0x38).addr() as u32);
    } else {
        wr::<u8>(this, 0x1B15, 0);
    }
    if now().wrapping_sub(g::<u32>(REPEAT_TIME).read()) > 200 {
        wr::<u32>(this, 0x1AF0, 0);
        wr::<u8>(this, 0x1AF4, 0);
        g::<u32>(REPEAT_TIME).write(now());
    }
    // up
    if step_input(this, KEY_UP, pad_analogue_up, 0x10, MOUSE_WHEEL_UP) {
        if b8(this, 0x1AF2) == 0 {
            let sel: i32 = rd(this, 0x90);
            wr::<u8>(this, 0x1AF2, 1);
            g::<u32>(REPEAT_TIME).write(now());
            if sel == 0 {
                wr::<i32>(this, 0x90, i32::from(rows) - 1);
            } else if sel > 0 {
                wr::<i32>(this, 0x90, sel - 1);
            }
        }
    } else {
        wr::<u8>(this, 0x1AF2, 0);
    }
    // down
    if step_input(this, KEY_DOWN, pad_analogue_down, 0x12, MOUSE_WHEEL_DOWN) {
        if b8(this, 0x1AF3) == 0 {
            wr::<u8>(this, 0x1AF3, 1);
            g::<u32>(REPEAT_TIME).write(now());
            let last = i32::from(rows) - 1;
            let sel: i32 = rd(this, 0x90);
            if sel == last {
                wr::<i32>(this, 0x90, 0);
            } else if sel < last {
                wr::<i32>(this, 0x90, sel + 1);
            }
        }
    } else {
        wr::<u8>(this, 0x1AF3, 0);
    }
    // back
    get_pad(0);
    let esc = word_just(KEY_ESC) || {
        let p = get_pad(0);
        rd::<u16>(p, 0x1E) != 0 && rd::<u16>(p, 0x4E) == 0
    };
    if esc {
        wr::<u8>(this, 0xB8, 0);
        wr::<u8>(back, 0, 1);
    }
    get_pad(0);
    if byte_just(MOUSE_LMB) && rd::<i32>(this, 0x1B00) == 3 {
        wr::<u8>(back, 0, 1);
    }
    get_pad(0);
    if byte_just(MOUSE_LMB) && rd::<i32>(this, 0x1B00) == 4 {
        audio_report_frontend_event(g(AUDIO_ENGINE), 1, 0, F_1.to_bits());
        wr::<u32>(this, 0x1B00, 5);
    }
    if b8(back, 0) == 0 {
        return;
    }
    if b8(this, 0x1AE9) != 0 {
        wr::<u8>(this, 0xB7, u8::from(b8(this, 0xB7) == 0));
        draw_controller_bound_call(this, 0x45, 1);
        if b8(this, 0x1AE9) == 0 {
            wr::<u8>(this, 0x1AEA, u8::from(b8(this, 0xB7) != 0) + 1);
        }
    } else {
        wr::<u8>(this, 0x1AEA, 3);
    }
}

/// One direction of step 5 of [`redefine_screen_user_input`]: the key word held, the analogue test, the
/// d-pad just pressed (mouse hidden), or the wheel just pressed (mouse shown). `GetPad(0)` before each.
fn step_input(this: GPtr<u8>, key: usize, analogue: fn() -> u8, dpad: isize, wheel: (usize, usize)) -> bool {
    get_pad(0);
    let hide = g::<u16>(key).read() != 0
        || {
            get_pad(0);
            analogue() != 0
        }
        || {
            let p = get_pad(0);
            rd::<u16>(p, dpad) != 0 && rd::<u16>(p, dpad + 0x30) == 0
        };
    if hide {
        wr::<u8>(this, 0xB8, 0);
        return true;
    }
    get_pad(0);
    if byte_just(wheel) {
        wr::<u8>(this, 0xB8, 1);
        return true;
    }
    false
}

/// The GXT keys `DrawControllerSetupScreen` fetches, in call order, with the slot of its 44-entry local
/// array each result goes to (slots 40, 41 and 43 are never read). Index 4 / 5 / 6 of the on-foot list
/// are replaced on the vehicle page.
const SETUP_KEYS: [(usize, usize); 41] = [
    (0, 0x8665A8),
    (1, 0x8665A0),
    (2, 0x866598),
    (3, 0x866590),
    (4, 0x866588),
    (5, 0x866580),
    (10, 0x866578),
    (6, 0x866558),
    (7, 0x866550),
    (8, 0x866548),
    (9, 0x866540),
    (11, 0x866538),
    (12, 0x866530),
    (13, 0x866528),
    (14, 0x866520),
    (15, 0x866518),
    (16, 0x866510),
    (17, 0x866508),
    (18, 0x866500),
    (19, 0x8664F8),
    (20, 0x8664F0),
    (21, 0x8664E8),
    (22, 0x8664E0),
    (23, 0x8664D8),
    (24, 0x8664D0),
    (25, 0x8664C8),
    (26, 0x8664C0),
    (27, 0x8664B8),
    (28, 0x8664B0),
    (29, 0x8664A8),
    (30, 0x8664A0),
    (31, 0x866498),
    (32, 0x866490),
    (33, 0x866488),
    (34, 0x866480),
    (35, 0x866478),
    (36, 0x866470),
    (37, 0x866468),
    (39, 0x866460),
    (38, 0x866458),
    (42, 0x866450),
];
/// The vehicle page's keys for list entries 4..6: `FEC_ACC`, `FEC_BRA`, `FEC_TSK`
const SETUP_VEHICLE_KEYS: [usize; 3] = [0x866570, 0x866568, 0x866560];

/// `CMenuManager::DrawControllerSetupScreen()` (0x57F300, thiscall): the redefine-controls screen.
///
/// 1. The 41 action names (`TheText.Get`, in the bytes' order).
/// 2. Font style 0, scale (0.9, 1.7), no edge, `HudColour.GetRGB(3)`; right-aligned at (W -
///    StretchX(48), StretchY(11)) `FET_CCN` (joypad, +0xD0 set) or `FET_SCN`; left-aligned at
///    (StretchX(48), StretchY(11)) `FET_CFT` (on foot) or `FET_CCR` (vehicle); the panel `{StretchX(20),
///    H - StretchY(50), W - StretchX(20), StretchY(50)}` in (0x31, 0x65, 0x94, 0x64).
/// 3. Per row: when not editing, the mouse inside `StretchX(20) < x < StretchX(600)` and the row's
///    `StretchY(rowHeight * row + 69)` < y < `StretchY(rowHeight * (row + 1) + 69)`: +0x1B04 = row, the
///    selection +0x90 = row when the mouse moved, with +0x1B00 == 5 on the selected row editing starts
///    (+0x1B09, +0x1B08, +0xF0 = `this + 0x38`), and +0x1B00 = 0x10 (not when +0x1B00 == 5 off the selected
///    row). Then (0x4A, 0x5A, 0x6B), scale (0.4, 0.6), style 2, wrap `StretchX(100) + W`, the row's name at
///    (StretchX(40), StretchY(rowHeight * row + 69)).
/// 4. `DrawControllerBound(69, false)`. When not editing: scale (0.7, 1.0); the back button
///    (`FEDS_TB`; `StretchX(15) < x < StretchX(35) + StretchX(GetStringWidth)`, `H - StretchY(33) < y < H -
///    StretchY(10)`) sets +0x1B00 = 3, else the list area (`StretchX(20) < x < StretchX(600)`,
///    `StretchY(48) < y < H - StretchY(33)`) 4, else 0x10.
/// 5. `FEDS_TB` at (StretchX(33), H - StretchY(38)): style 2, scale (0.7, 1.0), left, no edge, (0x4A, 0x5A,
///    0x6B).
///
/// Uninitialised stack: on a page other than 0 / 1 (not reachable) the original's row height is a byte
/// it never wrote and its row count the bits of a float local; the port uses 0 rows there.
#[hook(addr = 0x57F300, conv = thiscall, name = "CMenuManager::DrawControllerSetupScreen", expect = "81 EC D8 00 00 00",
       unverified = "menu-3", route = "split", hazards = "heap-writes",
       recs = "menus-map-controls",
       note = "controller setup screen; draws through CFont / CSprite2d")]
pub fn draw_controller_setup_screen(this: GPtr<u8>) {
    let vehicle = b8(this, 0xB7) != 0;
    let mut keys = [GPtr::<u8>::NULL; 44];
    for (k, &(slot, key)) in SETUP_KEYS.iter().enumerate() {
        let key = if vehicle && (4..7).contains(&k) { SETUP_VEHICLE_KEYS[k - 4] } else { key };
        keys[slot] = text(key);
    }

    font_set_font_style(0);
    set_scale(this, F_0_9, F_1_7);
    font_set_edge(0);
    let mut c = [0u8; 4];
    let cp = hud_colours_get_rgb(g(HUD_COLOUR), GPtr::from_raw(c.as_mut_ptr()), 3);
    font_set_color(rd::<u32>(cp, 0));
    font_set_orientation(2);
    let wf = f32_of(xi(w()));
    let method = text(if b8(this, 0xD0) != 0 { S_FET_CCN } else { S_FET_SCN });
    let ty = f32_of(stretch_y_call(this, F_11));
    let tx = f32_of(stretch_x_call(this, F_48).fsubr_m(wf));
    font_print_string(tx, ty, method);
    font_set_orientation(1);
    let mut height: u8 = 0;
    match b8(this, 0xB7) {
        0 => {
            let t = text(S_FET_CFT);
            let ty = f32_of(stretch_y_call(this, F_11));
            let tx = f32_of(stretch_x_call(this, F_48));
            font_print_string(tx, ty, t);
            height = row_height(this, 0);
        }
        1 => {
            let t = text(S_FET_CCR);
            let ty = f32_of(stretch_y_call(this, F_11));
            let tx = f32_of(stretch_x_call(this, F_48));
            font_print_string(tx, ty, t);
            height = 0xD;
        }
        _ => {}
    }
    let hf = f32_of(xi(h()));
    let bottom = f32_of(stretch_y_call(this, F_50).fsubr_m(hf));
    let right = f32_of(stretch_x_call(this, F_20).fsubr_m(f32_of(xi(w()))));
    let top = f32_of(stretch_y_call(this, F_50));
    let left = f32_of(stretch_x_call(this, F_20));
    let mut rect = [left, bottom, right, top];
    let mut col = [0u8; 4];
    let colp = rgba_ctor(GPtr::from_raw(col.as_mut_ptr()), 0x31, 0x65, 0x94, 0x64);
    sprite_draw_rect(GPtr::<f32>::from_raw(rect.as_mut_ptr()).cast(), colp);

    let rows = i32::from(row_count(this, 0));
    let mut row: u8 = 0;
    while i32::from(row) < rows {
        let r = u32::from(row);
        if b8(this, 0x1B09) == 0 {
            row_mouse(this, r, u32::from(height));
        }
        set_color(0x4A, 0x5A, 0x6B, 0xFF);
        set_scale(this, F_0_4, F_0_6);
        font_set_font_style(2);
        set_wrap(this);
        let list: Option<u32> = match b8(this, 0xB7) {
            0 => Some(FOOT_ROWS[usize::from(row)]),
            1 => Some(VEHICLE_ROWS[usize::from(row)]),
            _ => None,
        };
        if let Some(v) = list {
            let yv = f32_of(xi((u32::from(height) * r + 0x45) as i32));
            let t = keys[v as usize];
            let yy = f32_of(stretch_y_call(this, yv));
            let xx = f32_of(stretch_x_call(this, F_40));
            font_print_string(xx, yy, t);
        }
        row = row.wrapping_add(1);
    }

    draw_controller_bound_call(this, 0x45, 0);
    if b8(this, 0x1B09) == 0 {
        set_scale(this, F_0_7, F_1);
        back_button_bounds(this);
    }
    font_set_font_style(2);
    set_scale(this, F_0_7, F_1);
    font_set_orientation(1);
    font_set_edge(0);
    set_color(0x4A, 0x5A, 0x6B, 0xFF);
    let hf = f32_of(xi(h()));
    let t = text(S_FEDS_TB);
    let yy = f32_of(stretch_y_call(this, F_38).fsubr_m(hf));
    let xx = f32_of(stretch_x_call(this, F_33));
    font_print_string(xx, yy, t);
}

/// Step 3's mouse test of one row (0x57F8C2..0x57F9DE).
fn row_mouse(this: GPtr<u8>, row: u32, height: u32) {
    let mx = || f32_of(xi(rd(this, 0xBC)));
    let my = || f32_of(xi(rd(this, 0xC0)));
    if !below(stretch_x_call(this, F_20).fcom(x(mx()))) || !above(stretch_x_call(this, F_600).fcom(x(mx()))) {
        return;
    }
    let top = f32_of(xi((height.wrapping_mul(row).wrapping_add(0x45)) as i32));
    let myv = my();
    if !below(stretch_y_call(this, top).fcom(x(myv))) {
        return;
    }
    let myv = my();
    let bot = f32_of(xi(((row + 1).wrapping_mul(height).wrapping_add(0x45)) as i32));
    if !above(stretch_y_call(this, bot).fcom(x(myv))) {
        return;
    }
    wr::<u32>(this, 0x1B04, row);
    if rd::<i32>(this, 0x1AF8) != rd::<i32>(this, 0xBC) || rd::<i32>(this, 0x1AFC) != rd::<i32>(this, 0xC0) {
        wr::<u32>(this, 0x90, row);
    }
    if rd::<i32>(this, 0x1B00) == 5 {
        if row != rd::<u32>(this, 0x90) {
            return;
        }
        wr::<u8>(this, 0x1B09, 1);
        wr::<u8>(this, 0x1B08, 1);
        wr::<u32>(this, 0xF0, this.byte_add(0x38).addr() as u32);
    }
    wr::<u32>(this, 0x1B00, 0x10);
}

/// Step 4's mouse areas (0x57FB31..0x57FCB8).
fn back_button_bounds(this: GPtr<u8>) {
    let mx = || f32_of(xi(rd(this, 0xBC)));
    let my = || f32_of(xi(rd(this, 0xC0)));
    let hf = || f32_of(xi(h()));
    let mxv = mx();
    let width = f32_of(X87::from_f64_exact(get_string_width_st0(text(S_FEDS_TB), 1, 0)));
    let sw = f32_of(stretch_x_call(this, width));
    let on_back = above(stretch_x_call(this, F_35).fadd_m(sw).fcom(x(mxv)))
        && below(stretch_x_call(this, F_15).fcom(x(mx())))
        && {
            let hv = hf();
            let myv = my();
            below(stretch_y_call(this, F_33).fsubr_m(hv).fcom(x(myv)))
        }
        && {
            let hv = hf();
            let myv = my();
            above(stretch_y_call(this, F_10).fsubr_m(hv).fcom(x(myv)))
        };
    if on_back {
        wr::<u32>(this, 0x1B00, 3);
        return;
    }
    let in_list = below(stretch_x_call(this, F_20).fcom(x(mx())))
        && above(stretch_x_call(this, F_600).fcom(x(mx())))
        && below(stretch_y_call(this, F_48).fcom(x(my())))
        && {
            let hv = hf();
            let myv = my();
            above(stretch_y_call(this, F_33).fsubr_m(hv).fcom(x(myv)))
        };
    wr::<u32>(this, 0x1B00, if in_list { 4 } else { 0x10 });
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_DRAW_CONTROLLER_BOUND,
    HOOK_DRAW_CONTROLLER_SCREEN_EXTRA_TEXT,
    HOOK_REDEFINE_SCREEN_USER_INPUT,
    HOOK_DRAW_CONTROLLER_SETUP_SCREEN,
];
