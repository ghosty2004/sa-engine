//! `CMenuManager` front-end drawing: `DrawStandardMenus`, `DrawBackground`, `Print*`, `DisplaySlider`,
//! `DrawWindowedText` (PORT-HUD-3). Ported from the 1.0 US bytes; gta-reversed
//! `Frontend/MenuManager_Draw.cpp` as reading aid (the bytes win).
//!
//! Many of these functions inline `StretchX` / `StretchY` with their own constants: `W == 640 ? c :
//! (float)(fild W * k)` where `k` is a separate `.rdata` constant (≈ c / 640), not `c * (1/640)` as the
//! out-of-line `StretchX` computes. The ports reproduce each inline form with its constant ([`isx`],
//! [`isy`]); calls to the out-of-line functions stay `orig!` calls (`stretch_x_call` /
//! `stretch_y_call`, ST0 kept at x87 precision).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, rd, wr, x, xi};
use crate::menu_mgr::{stretch_x_call, stretch_y_call};

/// No globals of its own.
pub const GLOBALS: &[GlobalDesc] = &[];

/// `aScreens` (0x8CE008)
const SCREENS: usize = 0x8CE008;
/// `RsGlobal.maximumWidth` / `maximumHeight`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// `TheText` (0xC1B340), `AudioEngine` (0xB6BC90), `HudColour` (0xBAB22C), `g_fx` (0xA9AE00)
const THE_TEXT: usize = 0xC1B340;
const AUDIO_ENGINE: usize = 0xB6BC90;
const HUD_COLOUR: usize = 0xBAB22C;
const G_FX: usize = 0xA9AE00;
/// The save slot states (`CGenericGameStorage::ms_Slots`, 0xC16EBC; 0xC16EB8 = slot `-1`)
const SLOT_STATES: usize = 0xC16EBC;
/// `gString` (0xB71670), `gGxtString` (0xC1B100), `gGxtString2` (0xC1AED8)
const G_STRING: usize = 0xB71670;
const G_GXT_STRING: usize = 0xC1B100;
const G_GXT_STRING2: usize = 0xC1AED8;
/// `CCamera::m_fMouseAccelHorzntl` (0xB6EC1C), `MousePointerStateHelper.m_bVerticalInvert` (0xBA6745),
/// `CVehicle::m_bEnableMouseSteering` / `m_bEnableMouseFlying` (0xC1CC02 / 0xC1CC03)
const MOUSE_ACCEL: usize = 0xB6EC1C;
const INVERT_MOUSE_Y: usize = 0xBA6745;
const MOUSE_STEERING: usize = 0xC1CC02;
const MOUSE_FLYING: usize = 0xC1CC03;
/// The two `.HOODLUM` data dwords the no-CD chunk 0x409B33 combines (`[a] - [b] - 1`, carry set) into the
/// x argument of `GetNumberLines` (0x42700000 = 60.0 with the shipped bytes; read at run time as the
/// original does)
const PROT_X_A: usize = 0x15608E8;
const PROT_X_B: usize = 0x156B63F;
/// `[PROT_X_A] - [PROT_X_B] - 1` with the shipped bytes (0xCA42879E - 0x87D2879D - 1): 60.0.
const PROT_X_VALUE: u32 = 0x4270_0000;

/// GXT keys (`.rdata`)
mod key {
    pub const FESZ_QQ: usize = 0x86553C;
    pub const FEQ_SRW: usize = 0x865534;
    pub const FESZ_QC: usize = 0x86552C;
    pub const FESZ_QO: usize = 0x865524;
    pub const FES_LCG: usize = 0x86551C;
    pub const DOTS: usize = 0x86550C;
    pub const FESZ_CS: usize = 0x865504;
    pub const FEM_SL_D: usize = 0x8654F8;
    pub const FEC_MOU: usize = 0x8654F0;
    pub const FEJ_TIT: usize = 0x8654E8;
    pub const FEL: [usize; 5] = [0x8654E0, 0x8654D8, 0x8654D0, 0x8654C8, 0x8654C0];
    pub const FEM_OFF: usize = 0x8653A4;
    pub const FEM_ON: usize = 0x86539C;
    pub const FED_RDM: usize = 0x8654B0;
    pub const FED_RDB: usize = 0x8654B8;
    pub const FEA_PR: [usize; 3] = [0x865498, 0x8654A0, 0x8654A8];
    pub const FED_FX: [usize; 4] = [0x865490, 0x865488, 0x865480, 0x865478];
    pub const PCT_D: usize = 0x859508;
    pub const FET_SCN: usize = 0x865468;
    pub const FET_CCN: usize = 0x865470;
    pub const FED_RES: usize = 0x86545C;
    pub const FED_AAS: usize = 0x865454;
}

const fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// 1/640 (0x859520), 1/448 (0x859524), 0.0625 (0x858620), 0.5 (0x858B8C), 40.0 (0x858A10), 3.0
/// (0x858B3C), 2^32 (0x858C54), 70.0 (0x858CE0)
const K_INV_640: f32 = f(0x3ACC_CCCD);
const K_INV_448: f32 = f(0x3B12_4925);
const K_1_16: f32 = f(0x3D80_0000);
const K_HALF: f32 = f(0x3F00_0000);
const K_40: f32 = f(0x4220_0000);
const K_3: f32 = f(0x4040_0000);
const K_2_32: f32 = f(0x4F80_0000);
const K_70: f32 = f(0x428C_0000);

orig! {
    /// `CFont::SetBackground(bool, bool)` (0x7195C0)
    pub fn font_set_background(on: u32, outline: u32) = cdecl 0x7195C0 => crate::font::set_background;
    /// `CFont::SetProportional(bool)` (0x7195B0)
    pub fn font_set_proportional(v: u32) = cdecl 0x7195B0 => crate::font::set_proportional;
    /// `CFont::SetOrientation(eFontAlignment)` (0x719610)
    pub fn font_set_orientation(v: u32) = cdecl 0x719610 => crate::font::set_orientation;
    /// `CFont::SetWrapx(float)` (0x7194D0)
    pub fn font_set_wrapx(v: u32) = cdecl 0x7194D0 => crate::font::set_wrapx;
    /// `CFont::SetRightJustifyWrap(float)` (0x7194F0)
    pub fn font_set_right_justify_wrap(v: u32) = cdecl 0x7194F0 => crate::font::set_right_justify_wrap;
    /// `CFont::SetCentreSize(float)` (0x7194E0)
    pub fn font_set_centre_size(v: u32) = cdecl 0x7194E0 => crate::font::set_centre_size;
    /// `CFont::SetFontStyle(eFontStyle)` (0x719490)
    pub fn font_set_font_style(v: u32) = cdecl 0x719490 => crate::font::set_font_style;
    /// `CFont::SetScale(float, float)` (0x719380)
    pub fn font_set_scale(w: u32, h: u32) = cdecl 0x719380 => crate::font::set_scale;
    /// `CFont::SetScaleForCurrentLanguage(float, float)` (0x7193A0)
    pub fn font_set_scale_for_current_language(w: f32, h: u32) = cdecl 0x7193A0 => crate::font::set_scale_for_current_language;
    /// `CFont::SetEdge(int8)` (0x719590)
    pub fn font_set_edge(v: u32) = cdecl 0x719590 => crate::font::set_edge;
    /// `CFont::SetColor(CRGBA)` (0x719430)
    pub fn font_set_color(c: u32) = cdecl 0x719430 => crate::font::set_color;
    /// `CFont::SetDropColor(CRGBA)` (0x719510)
    pub fn font_set_drop_color(c: u32) = cdecl 0x719510 => crate::font::set_drop_color;
    /// `CFont::SetDropShadowPosition(int8)` (0x719570)
    pub fn font_set_drop_shadow_position(v: u32) = cdecl 0x719570 => crate::font::set_drop_shadow_position;
    /// `CFont::PrintString(float, float, const GxtChar*)` (0x71A700)
    pub fn font_print_string(px: f32, py: f32, text: GPtr<u8>) = cdecl 0x71A700 => crate::font_print::print_string;
    /// `CFont::GetNumberLines(float, float, const GxtChar*)` (0x71A5E0; AX)
    pub fn font_get_number_lines(px: u32, py: u32, text: GPtr<u8>) -> u32 = cdecl 0x71A5E0 => get_number_lines_bits;
    /// `CFont::GetStringWidth(const GxtChar*, bool, bool)` (0x71A0E0, ST0)
    pub fn font_get_string_width_st0(text: GPtr<u8>, full: u32, script: u32) -> f64 = cdecl 0x71A0E0 => crate::font_print::get_string_width_call_st0;
    /// `CText::Get(const char*)` (0x6A0050, thiscall)
    pub fn text_get(this: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => crate::text::text_get;
    /// `CRGBA::CRGBA(r, g, b, a)` (0x7170C0, thiscall)
    pub fn rgba_ctor(this: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CHudColours::GetRGB(uint8)` (0x58FEA0, thiscall, hidden return pointer)
    pub fn hud_colours_get_rgb(this: GPtr<u8>, out: GPtr<u8>, colour: u32) -> GPtr<u8> = thiscall 0x58FEA0 => crate::hud::get_rgb;
    /// `AsciiToGxtChar(const char*, GxtChar*)` (0x718600)
    pub fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) = cdecl 0x718600 => unported;
    /// `GxtCharStrlen(const GxtChar*)` (0x718690)
    pub fn gxt_char_strlen(s: GPtr<u8>) -> u32 = cdecl 0x718690 => crate::font_print::gxt_char_strlen;
    /// `GxtCharStrcat(GxtChar*, const GxtChar*)` (0x718630)
    pub fn gxt_char_strcat(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x718630 => crate::font_print::gxt_char_strcat;
    /// CRT `sprintf(buf, fmt, a)` (0x821BB5)
    pub fn crt_sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, a: i32) -> i32 = cdecl 0x821BB5 => unported;
    /// `RwEngineInstance->stringFuncs.vecSprintf` (`RwEngineInstance` +0xF0): `(buf, fmt, a)`
    pub fn rw_sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, a: i32) -> i32 = cdecl table 0xC97B24 0xF0 => unported;
    /// `CMemoryMgr::Calloc` (0x72F460) as `DrawStandardMenus` calls it: `(0x64, 1, 0)`
    pub fn memory_calloc(a: u32, b: u32, c: u32) -> GPtr<u8> = cdecl 0x72F460 => unported;
    /// `CMemoryMgr::Free(void*)` (0x72F430)
    pub fn memory_free(p: GPtr<u8>) = cdecl 0x72F430 => unported;
    /// `CGenericGameStorage::GetNameOfSavedGame(int32)` (0x5D0F40)
    pub fn get_name_of_saved_game(slot: i32) -> GPtr<u8> = cdecl 0x5D0F40 => unported;
    /// `CGenericGameStorage::GetSavedGameDateAndTime(int32)` (0x618D00)
    pub fn get_saved_game_date_and_time(slot: i32) -> GPtr<u8> = cdecl 0x618D00 => unported;
    /// `CAudioEngine::GetRadioStationName(int8)` (0x507000, thiscall)
    pub fn audio_get_radio_station_name(this: GPtr<u8>, station: u32) -> GPtr<u8> = thiscall 0x507000 => unported;
    /// `Fx_c::GetFxQuality()` (0x49EA50, thiscall)
    pub fn fx_get_quality(this: GPtr<u8>) -> u32 = thiscall 0x49EA50 => unported;
    /// `VideoMode::GetVideoModeList()` (0x745AF0)
    pub fn get_video_mode_list() -> GPtr<GPtr<u8>> = cdecl 0x745AF0 => unported;
    /// `CSprite2d::Draw(const CRect&, const CRGBA&)` (0x728350, thiscall)
    pub fn sprite2d_draw_rect(this: GPtr<u8>, rect: GPtr<u8>, colour: GPtr<u8>) = thiscall 0x728350 => unported;
    /// `CMenuManager::PrintStats()` (0x574900, thiscall)
    pub fn print_stats_call(this: GPtr<u8>) = thiscall 0x574900 => print_stats;
    /// `CMenuManager::PrintBriefs()` (0x576320, thiscall)
    pub fn print_briefs_call(this: GPtr<u8>) = thiscall 0x576320 => print_briefs;
    /// `CMenuManager::PrintRadioStationList()` (0x5746F0, thiscall)
    pub fn print_radio_station_list_call(this: GPtr<u8>) = thiscall 0x5746F0 => print_radio_station_list;
    /// `CMenuManager::DrawControllerScreenExtraText(int32)` (0x57D8D0, thiscall)
    pub fn draw_controller_screen_extra_text(this: GPtr<u8>, y: i32) = thiscall 0x57D8D0 => unported;
    /// `CMenuManager::DisplaySlider(float x, float y, float h1, float h2, float length, float value,
    /// int32 spacing)` (0x576860, thiscall)
    pub fn display_slider_call(this: GPtr<u8>, px: f32, py: f32, h1: f32, h2: f32, len: f32, value: f32, spacing: i32) -> i32 = thiscall 0x576860 => display_slider;
    /// `CMenuManager::CheckHover(int32, int32, int32, int32)` (0x57C4F0, thiscall)
    pub fn check_hover_call(this: GPtr<u8>, left: i32, right: i32, top: i32, bottom: i32) -> u8 = thiscall 0x57C4F0 => crate::menu_mgr::check_hover;
    /// `CMenuManager::SetHelperText(int32)` (0x57CD10, thiscall)
    pub fn set_helper_text_call(this: GPtr<u8>, text: u32) = thiscall 0x57CD10 => crate::menu_mgr::set_helper_text;
    /// `CMenuManager::ResetHelperText()` (0x57CD30, thiscall)
    pub fn reset_helper_text_call(this: GPtr<u8>) = thiscall 0x57CD30 => crate::menu_mgr::reset_helper_text;
    /// `CMenuManager::DisplayHelperText(const char*)` (0x57E240, thiscall)
    pub fn display_helper_text_call(this: GPtr<u8>, key: GPtr<u8>) = thiscall 0x57E240 => crate::menu_mgr::display_helper_text;
}

/// The Rust path of [`font_get_number_lines`] with the float arguments as words.
#[allow(dead_code)]
fn get_number_lines_bits(px: u32, py: u32, text: GPtr<u8>) -> u32 {
    crate::font::get_number_lines(f32::from_bits(px), f32::from_bits(py), text)
}

fn b8(p: GPtr<u8>, off: isize) -> u8 {
    rd(p, off)
}

fn local<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(core::ptr::from_mut(v)).cast()
}

fn w() -> i32 {
    g::<i32>(RS_MAX_WIDTH).read()
}

fn h() -> i32 {
    g::<i32>(RS_MAX_HEIGHT).read()
}

/// Inline `StretchX(c)`: `c` when the width is 640, else `(float)(fild W * k)`.
fn isx(c: f32, k: f32) -> f32 {
    if w() == 0x280 { c } else { xi(w()).fmul_m(k).to_f32() }
}

/// Inline `StretchY(c)`: `c` when the height is 448, else `(float)(fild H * k)`.
fn isy(c: f32, k: f32) -> f32 {
    if h() == 0x1C0 { c } else { xi(h()).fmul_m(k).to_f32() }
}

/// Inline stretch of an item coordinate: `fild v`, then `fild H; fmul st, st(1); fmul [1/448]` (Y) /
/// the same with W and 1/640 (X), stored as f32.
fn item_sy(v: i32) -> f32 {
    if h() == 0x1C0 { xi(v).to_f32() } else { xi(h()).fmul(xi(v)).fmul_m(K_INV_448).to_f32() }
}

fn item_sx(v: i32) -> f32 {
    if w() == 0x280 { xi(v).to_f32() } else { xi(w()).fmul(xi(v)).fmul_m(K_INV_640).to_f32() }
}

/// The current screen (`aScreens[(int8)+0x15D]`).
fn cur_screen(this: GPtr<u8>) -> GPtr<u8> {
    let s = i32::from(b8(this, 0x15D) as i8);
    g(SCREENS.wrapping_add(s.wrapping_mul(0xE2) as u32 as usize))
}

/// Item `i` of the current screen (+0 action, +1 name[8], +9 type, +0xA target screen, +0xC x, +0xE y,
/// +0x10 align).
fn item(this: GPtr<u8>, i: i32) -> GPtr<u8> {
    cur_screen(this).byte_add(i.wrapping_mul(0x12) as isize + 0xA)
}

fn rgba(r: u32, gr: u32, b: u32, a: u32) -> u32 {
    let mut tmp = 0u32;
    rd(rgba_ctor(local(&mut tmp), r, gr, b, a), 0)
}

fn hud_rgb(c: u32) -> u32 {
    let mut tmp = 0u32;
    rd(hud_colours_get_rgb(g(HUD_COLOUR), local(&mut tmp), c), 0)
}

fn get(k: usize) -> GPtr<u8> {
    text_get(g(THE_TEXT), g(k))
}

fn on_off(v: u8) -> GPtr<u8> {
    get(if v != 0 { key::FEM_ON } else { key::FEM_OFF })
}

/// `repz cmpsb` over 8 bytes: the item name equals the key (bytes after a NUL compared too).
fn name_is(name: GPtr<u8>, k: usize) -> bool {
    (0..8).all(|i| b8(name, i) == g::<u8>(k + i as usize).read())
}

/// Constants of the inline stretches (`.rdata`)
mod kk {
    use super::f;
    pub const X1_3: f32 = f(0x3B05_1EB8);
    pub const Y2_1: f32 = f(0x3B99_9999);
    pub const Y1_2: f32 = f(0x3B2F_8AF9);
    pub const X0_49: f32 = f(0x3A48_B439);
    pub const Y97: f32 = f(0x3E5D_B6DB);
    pub const X60: f32 = f(0x3DC0_0000);
    pub const Y0_95: f32 = f(0x3B0A_F8AF);
    pub const X0_42: f32 = f(0x3A2C_0832);
    pub const X0_7: f32 = f(0x3A8F_5C29);
    pub const X0_35: f32 = f(0x3A0F_5C29);
    pub const X0_56: f32 = f(0x3A65_6042);
    pub const X3: f32 = f(0x3B99_999A);
    pub const X100: f32 = f(0x3E20_0000);
    pub const Y20: f32 = f(0x3D36_DB6E);
    pub const Y4: f32 = f(0x3C12_4925);
    pub const Y125: f32 = f(0x3E8E_DB6E);
    pub const Y95: f32 = f(0x3E59_2492);
    pub const X500: f32 = f(0x3F48_0000);
    pub const BRIGHT: f32 = f(0x3B2A_AAAB);
    pub const VOL: f32 = f(0x3C80_0000);
    pub const DIST_MIN: f32 = f(0x3F6C_CCCD);
    pub const DIST_SCALE: f32 = f(0x3F92_4925);
    pub const MOUSE_MIN: f32 = f(0x3BA3_D70A);
}

/// The local GXT buffer of `DrawStandardMenus` (the original's is 0x40 bytes at `esp+0xF0`; the port's
/// is larger, so a long video mode name cannot overrun it where the original would overwrite its frame).
const RIGHT_BUF: usize = 0x100;

/// `CMenuManager::DrawStandardMenus(bool drawTitle)` (0x5794A0, thiscall, `ret 4`; part of the title
/// toggle in the `.text` chunk 0x403A5C (`mov ecx, TheText; jmp back`) and the `GetNumberLines` x in
/// the no-CD chunk 0x409B33): the standard front-end page.
///
/// Font setup (no background, proportional, left, wrap `W - 10`, right wrap 10, centre size W);
/// screen 0 `PrintStats`, 2 `PrintBriefs`, 3 with the title `PrintRadioStationList`. The title (when
/// asked, named, and not the loaded map screen 5 / +0x78): font 0, inline scale (1.3, 2.1), edge 1,
/// HUD colours 3 / 5, at (40, 28). Item 0 a title (action 1): wrap `W - 40`, font 1, language scale
/// (0.49, 1.2), edge 2, colours (0x4A, 0x5A, 0x6B) on black; screens 6 / 0x23 / 11 with +0xE9 ask
/// FESZ_QQ / FEQ_SRW / FES_LCG, screen 17 by the selected slot's (+0x15F) state FESZ_QO (0) /
/// FESZ_QC (2), else item 0's text, at (60, 97), then wrap `W - 10` and right wrap 10. Screen 0x26:
/// with +0x1B09 the items are not drawn, `DrawControllerScreenExtraText(-8)`.
///
/// Then the 12 items: font 2, save slots (type 1..8) edge 1 / scale (0.42, 0.95), others scale (0.7,
/// 1.0) / edge 2; black drop colour; colour (0xAC, 0xCB, 0xF1) for the current item with +0x78, else
/// (0x4A, 0x5A, 0x6B); align by +0x10 (1 left, 2 right, else centre). An item at (0, 0), or of type 9,
/// is placed at (320, 130) when it is item 0 or item 1 after a title, else below the previous item (+20
/// for type 9, +30). Titles and unnamed items stop here. Text: slots: x = 80, left aligned, by the
/// slot state: filled: the save name (`"..."` appended when 254 characters or more), the right column
/// its date; corrupted: FESZ_CS; when that or the name is null / empty: `FEM_SL<i>` centred at x 320
/// with the highlight x `_ftol(StretchX(40))` (inline); type 9 (mission pack): with the pack's id byte
/// (`this + i * 0x105 - 0xA9`) its name (`- 0xA8`) through `gGxtString`, action 0xB, else action 0x14,
/// no text and the previous item's y; type 10: FEJ_TIT / FEC_MOU by the control method (+0xD0); else
/// the item's text. The right column by action (switch 0x57B380): toggles ON / OFF (0x18 +0x4C, 0x19
/// +0x44, 0x1A +0x4B, 0x1E +0x51, 0x1F +0x4D, 0x21 +0x4A, 0x23 +0x21, 0x2F..0x34 +0xB1..+0xB6, 0x40
/// +0xC5, 0x41 +0xE8; 0x2B +0xC4 dimmed (0xE, 0x1E, 0x2F) outside the main menu, 0x2E inverted
/// 0xBA6745, 0x3B / 0x3C mouse steering / flying dimmed with the joypad), 0x20 the radio station's
/// name, 0x22 radar mode, 0x24 language, 0x2A FX quality, 0x2C anti-aliasing (`"%d"` of +0xCC - 1
/// through a `Calloc`ed buffer, or OFF), 0x38 the video mode's name cut before `'('` and its trailing
/// blanks, 0x3A control type, 0x3F user track mode. The text prints at the item's stretched position;
/// the right column (font 2, edge 1, right aligned, slots scale (0.35, 0.95), the radio station item 5
/// of screen 3 (0.56, 1.0), else (0.7, 1.0)) at `W - StretchX(40)`. The current item with text,
/// off the map / briefs screens: the highlight x unless set (left `sx - StretchX(40)`, right
/// `StretchX(40) + x * W/640`, centre `sx - StretchX(40) - width * 0.5`, `_ftol`), and with +0x78 off
/// screens 0xD / 0xE / 0x12 the sprite +0xF8 drawn at `{x, sy + StretchX(47), x + StretchX(32), sy -
/// StretchX(5)}` in white. The helper text for the video mode / anti-aliasing items (FET_APP when they
/// differ from the applied value, reset when equal, FET_RSO and the value restored on screens 4 /
/// 0x1B elsewhere); the sliders (0x1B brightness, 0x1C radio volume (y 95), 0x1D SFX volume, 0x3D draw
/// distance, 0x3E mouse sensitivity) through `DisplaySlider` and, for the current item when items are
/// drawn, the mouse states +0x1B00 (left arrow 7 / 0xB / 0xD / 9 / 0xF, right 6 / 0xA / 0xC / 8 / 0xE
/// while the mouse is over the bar, else 0x10); the text line count `* 29` (and 70 after a radio item)
/// accumulates into a y that is only passed to `GetNumberLines`. Screens 0, 3, 4, 0x1A, 0x1B, 0x24,
/// 0x27, 0x28 end with `DisplayHelperText(null)`.
///
/// Differences from gta-reversed (the bytes win): the stretches are the inline forms above; the drop
/// colour of the items is `CRGBA(0, 0, 0, 255)` (gr: HUD colour black); type 9 items are always
/// re-placed (gr: only at (0, 0)); the mission pack test reads the pack's id byte, not its name; a
/// filled slot prints only its name (gr prints the v1.01 `"<i>:"` prefix) and an over-long name gets
/// `"..."` appended without truncation; a corrupted or unnamed slot falls back to `FEM_SL<i>` only when
/// the text is null or empty; the right column scale tests type 1..8 (not gr's `isSlot`); the
/// highlight x is a 16-bit integer (`cmp word`, `_ftol`); the sprite is not drawn on screens 0xD, 0xE,
/// 0x12; the helper-text checks run for every non-title item; the slider mouse states follow the
/// bytes (the radio slider checks y 95..120, the mouse sensitivity slider only x >= StretchX(500));
/// the item y accumulator adds 2^32 to a negative line count (unsigned conversion pattern).
#[hook(addr = 0x5794A0, conv = thiscall, name = "CMenuManager::DrawStandardMenus", expect = "81 EC 20 01 00 00")]
pub fn draw_standard_menus(this: GPtr<u8>, draw_title: u32) {
    let draw_title = draw_title as u8 != 0;
    let mut item_y_pos: f32 = 0.0;
    let mut draw_items = true;
    font_set_background(0, 0);
    font_set_proportional(1);
    font_set_orientation(1);
    font_set_wrapx(xi(w().wrapping_sub(10)).to_f32().to_bits());
    font_set_right_justify_wrap(0x4120_0000);
    font_set_centre_size(xi(w()).to_f32().to_bits());
    match b8(this, 0x15D) as i8 {
        0 => print_stats_call(this),
        2 => print_briefs_call(this),
        3 if draw_title => print_radio_station_list_call(this),
        _ => {}
    }

    // the title
    if draw_title && b8(cur_screen(this), 0) != 0 && !(b8(this, 0x15D) == 5 && b8(this, 0x78) != 0) {
        font_set_orientation(1);
        font_set_font_style(0);
        let sy = isy(f(0x4006_6666), kk::Y2_1);
        let sx = isx(f(0x3FA6_6666), kk::X1_3);
        font_set_scale(sx.to_bits(), sy.to_bits());
        font_set_edge(1);
        font_set_color(hud_rgb(3));
        font_set_drop_color(hud_rgb(5));
        let py = isy(f(0x41E0_0000), K_1_16);
        let px = isx(K_40, K_1_16);
        let text = text_get(g(THE_TEXT), cur_screen(this));
        font_print_string(px, py, text);
    }

    // a title item 0: the page's text
    if b8(item(this, 0), 0) == 1 {
        font_set_wrapx(xi(w().wrapping_sub(0x28)).to_f32().to_bits());
        font_set_font_style(1);
        let sy = isy(f(0x3F99_999A), kk::Y1_2);
        let sx = isx(f(0x3EFA_E147), kk::X0_49);
        font_set_scale_for_current_language(sx, sy.to_bits());
        font_set_orientation(1);
        font_set_edge(2);
        font_set_drop_color(rgba(0, 0, 0, 0xFF));
        font_set_color(rgba(0x4A, 0x5A, 0x6B, 0xFF));
        let scr = b8(this, 0x15D) as i8;
        let ask = |k: usize| if b8(this, 0xE9) != 0 { Some(k) } else { None };
        let k = match scr {
            6 => ask(key::FESZ_QQ),
            11 => ask(key::FES_LCG),
            17 => {
                let slot = i32::from(b8(this, 0x15F) as i8);
                match g::<u32>((SLOT_STATES as u32).wrapping_add((slot as u32).wrapping_mul(4)) as usize).read() {
                    0 => Some(key::FESZ_QO),
                    2 => Some(key::FESZ_QC),
                    _ => None,
                }
            }
            0x23 => ask(key::FEQ_SRW),
            _ => None,
        };
        let text = match k {
            Some(k) => get(k),
            None => text_get(g(THE_TEXT), item(this, 0).byte_add(1)),
        };
        let py = isy(f(0x42C2_0000), kk::Y97);
        let px = isx(f(0x4270_0000), kk::X60);
        font_print_string(px, py, text);
        font_set_wrapx(xi(w().wrapping_sub(10)).to_f32().to_bits());
        font_set_right_justify_wrap(0x4120_0000);
    }

    if b8(this, 0x15D) == 0x26 {
        if b8(this, 0x1B09) != 0 {
            draw_items = false;
        }
        draw_controller_screen_extra_text(this, -8);
    }

    let has_label = b8(item(this, 0), 0) == 1;
    let mut right_buf = [0u8; RIGHT_BUF];
    for i in 0..0xCi32 {
        draw_item(this, i, has_label, draw_items, &mut item_y_pos, &mut right_buf);
    }

    if matches!(b8(this, 0x15D) as i8, 0 | 3 | 4 | 0x1A | 0x1B | 0x24 | 0x27 | 0x28) {
        display_helper_text_call(this, GPtr::NULL);
    }
}

/// One iteration of `DrawStandardMenus`' item loop (0x5798F0..0x57B29F).
fn draw_item(this: GPtr<u8>, i: i32, has_label: bool, draw_items: bool, item_y_pos: &mut f32, right_buf: &mut [u8; RIGHT_BUF]) {
    let ty = b8(item(this, i), 9) as i8;
    let line_height: i16 = if ty == 9 { 0x14 } else { 0x1E };
    let mut right = GPtr::<u8>::NULL;
    let is_slot = (1..=8).contains(&ty);
    font_set_font_style(2);
    if is_slot {
        font_set_edge(1);
        let sy = isy(f(0x3F73_3333), kk::Y0_95);
        let sx = isx(f(0x3ED7_0A3E), kk::X0_42);
        font_set_scale(sx.to_bits(), sy.to_bits());
    } else {
        let sy = isy(1.0, K_INV_448);
        let sx = isx(f(0x3F33_3333), kk::X0_7);
        font_set_scale(sx.to_bits(), sy.to_bits());
        font_set_edge(2);
    }
    font_set_drop_color(rgba(0, 0, 0, 0xFF));
    if i == rd::<i32>(this, 0x54) && b8(this, 0x78) != 0 {
        font_set_color(rgba(0xAC, 0xCB, 0xF1, 0xFF));
    } else {
        font_set_color(rgba(0x4A, 0x5A, 0x6B, 0xFF));
    }
    font_set_orientation(match b8(item(this, i), 0x10) as i8 {
        1 => 1,
        2 => 2,
        _ => 0,
    });

    // position
    let it = item(this, i);
    if (rd::<i16>(it, 0xC) == 0 && rd::<i16>(it, 0xE) == 0) || b8(it, 9) == 9 {
        if i == 0 || (i == 1 && has_label) {
            wr::<i16>(it, 0xC, 0x140);
            wr::<i16>(item(this, i), 0xE, 0x82);
        } else {
            wr(it, 0xC, rd::<i16>(it, 0xC - 0x12));
            let it = item(this, i);
            wr(it, 0xE, rd::<i16>(it, 0xE - 0x12).wrapping_add(line_height));
        }
    }

    let it = item(this, i);
    let action_ptr = it;
    if b8(it, 0) == 1 || b8(it, 1) == 0 {
        return;
    }

    // the item's text
    let ty = b8(it, 9) as i8;
    let mut x_off: i32 = 0;
    let text: GPtr<u8>;
    if (1..=8).contains(&ty) {
        font_set_orientation(1);
        wr::<i16>(it, 0xC, 0x50);
        let state = g::<u32>((SLOT_STATES as u32 - 4).wrapping_add((i as u32).wrapping_mul(4)) as usize).read();
        let mut t = GPtr::<u8>::NULL;
        let mut named = false;
        if state == 0 {
            let slot = i.wrapping_sub(1);
            t = get_name_of_saved_game(slot);
            if gxt_char_strlen(t) >= 0xFE {
                ascii_to_gxt_char(g(key::DOTS), g(G_GXT_STRING2));
                gxt_char_strcat(t, g(G_GXT_STRING2));
            }
            right = get_saved_game_date_and_time(slot);
            named = true;
        } else if state == 2 {
            t = get(key::FESZ_CS);
            named = true;
        }
        if !named || t.is_null() || b8(t, 0) == 0 {
            crt_sprintf1(g(G_STRING), g(key::FEM_SL_D), i);
            font_set_orientation(0);
            wr::<i16>(item(this, i), 0xC, 0x140);
            t = text_get(g(THE_TEXT), g(G_STRING));
            x_off = if w() == 0x280 { ftol32(x(K_40)) } else { ftol32(xi(w()).fmul_m(K_1_16)) };
        }
        text = t;
    } else if ty == 9 {
        let id = b8(this, (i.wrapping_mul(0x105) - 0xA9) as isize);
        if id != 0 {
            ascii_to_gxt_char(this.byte_add((i.wrapping_mul(0x105) - 0xA8) as isize), g(G_GXT_STRING));
            wr::<u8>(item(this, i), 0, 0xB);
            text = g(G_GXT_STRING);
        } else {
            wr::<u8>(action_ptr, 0, 0x14);
            let it = item(this, i);
            wr(it, 0xE, rd::<i16>(it, 0xE - 0x12));
            text = GPtr::NULL;
        }
    } else if ty == 0xA {
        text = get(if b8(this, 0xD0) == 0 { key::FEC_MOU } else { key::FEJ_TIT });
    } else {
        text = text_get(g(THE_TEXT), it.byte_add(1));
    }

    // the right column
    let action = b8(item(this, i), 0);
    let toggle = |off: isize| on_off(b8(this, off));
    let dim = || font_set_color(rgba(0xE, 0x1E, 0x2F, 0xFF));
    match action {
        0x18 => right = toggle(0x4C),
        0x19 => right = toggle(0x44),
        0x1A => right = toggle(0x4B),
        0x1E => right = toggle(0x51),
        0x1F => right = toggle(0x4D),
        0x20 => right = audio_get_radio_station_name(g(AUDIO_ENGINE), u32::from(b8(this, 0x52))),
        0x21 => right = toggle(0x4A),
        0x22 => match rd::<u32>(this, 0x24) {
            0 => right = get(key::FED_RDM),
            1 => right = get(key::FED_RDB),
            2 => right = get(key::FEM_OFF),
            _ => {}
        },
        0x23 => right = toggle(0x21),
        0x24 => {
            let l = i32::from(b8(this, 0x84) as i8) as u32;
            if l <= 4 {
                right = get(key::FEL[l as usize]);
            }
        }
        0x2A => {
            let q = fx_get_quality(g(G_FX));
            if q <= 3 {
                right = get(key::FED_FX[q as usize]);
            }
        }
        0x2B => {
            right = toggle(0xC4);
            if b8(this, 0xE9) == 0 {
                dim();
            }
        }
        0x2C => {
            let buf = memory_calloc(0x64, 1, 0);
            let aa: i32 = rd(this, 0xCC);
            if aa > 1 {
                rw_sprintf1(buf, g(key::PCT_D), aa - 1);
                ascii_to_gxt_char(buf, local(right_buf));
                right = local(right_buf);
                memory_free(buf);
            } else {
                right = get(key::FEM_OFF);
                memory_free(buf);
            }
        }
        0x2E => right = on_off(u8::from(g::<u8>(INVERT_MOUSE_Y).read() == 0)),
        0x2F..=0x34 => right = toggle(0xB1 + isize::from(action - 0x2F)),
        0x38 => {
            let list = get_video_mode_list();
            let mode: i32 = rd(this, 0xD8);
            let name: GPtr<u8> = list.wrapping_index(mode).read();
            ascii_to_gxt_char(name, local(right_buf));
            trim_video_mode(right_buf);
            right = local(right_buf);
        }
        0x3A => match b8(this, 0xD0) as i8 {
            0 => right = get(key::FET_SCN),
            1 => right = get(key::FET_CCN),
            _ => {}
        },
        0x3B | 0x3C => {
            right = on_off(g::<u8>(if action == 0x3B { MOUSE_STEERING } else { MOUSE_FLYING }).read());
            if b8(this, 0xD0) == 1 {
                dim();
            }
        }
        0x3F => {
            if (0..=2).contains(&(b8(this, 0xB0) as i8)) {
                right = get(key::FEA_PR[b8(this, 0xB0) as usize]);
            }
        }
        0x40 => right = toggle(0xC5),
        0x41 => right = toggle(0xE8),
        _ => {}
    }

    // print the text and the right column
    if !text.is_null() {
        let it = item(this, i);
        let py = item_sy(i32::from(rd::<i16>(it, 0xE)));
        let px = item_sx(i32::from(rd::<i16>(it, 0xC)));
        font_print_string(px, py, text);
    }
    if !right.is_null() {
        font_set_font_style(2);
        font_set_edge(1);
        font_set_orientation(2);
        let ty = b8(item(this, i), 9) as i8;
        let (sx, sy) = if (1..=8).contains(&ty) {
            let sy = isy(f(0x3F73_3333), kk::Y0_95);
            (isx(f(0x3EB3_3333), kk::X0_35), sy)
        } else if b8(this, 0x15D) == 3 && i == 5 {
            let sy = isy(1.0, K_INV_448);
            (isx(f(0x3F0F_5C29), kk::X0_56), sy)
        } else {
            let sy = isy(1.0, K_INV_448);
            (isx(f(0x3F33_3333), kk::X0_7), sy)
        };
        font_set_scale(sx.to_bits(), sy.to_bits());
        let py = item_sy(i32::from(rd::<i16>(item(this, i), 0xE)));
        let s40 = if w() == 0x280 { x(K_40) } else { xi(w()).fmul_m(K_1_16) };
        let px = xi(w()).fsub(s40).to_f32();
        font_print_string(px, py, right);
    }

    // the highlight sprite
    if !text.is_null() && i == rd::<i32>(this, 0x54) && b8(this, 0x15D) != 5 && b8(this, 0x15D) != 2 {
        if x_off as i16 == 0 {
            let it = item(this, i);
            let ix = i32::from(rd::<i16>(it, 0xC));
            let sx = || xi(ix).fmul(xi(w()).fmul_m(K_INV_640));
            let v = match b8(it, 0x10) as i8 {
                1 => {
                    let a = sx().to_f32();
                    stretch_x_call(this, K_40).fsubr_m(a)
                }
                2 => stretch_x_call(this, K_40).fadd(sx()),
                _ => {
                    let a = sx().to_f32();
                    let b = stretch_x_call(this, K_40).fsubr_m(a).to_f32();
                    X87::from_f64_exact(font_get_string_width_st0(text, 1, 0)).fmul_m(K_HALF).fsubr_m(b)
                }
            };
            x_off = ftol32(v);
        }
        if b8(this, 0x78) != 0 && !matches!(b8(this, 0x15D), 0xD | 0xE | 0x12) {
            let left = xi(i32::from(x_off as i16)).to_f32();
            let iy = i32::from(rd::<i16>(item(this, i), 0xE));
            let sy = xi(iy).fmul(xi(h()).fmul_m(K_INV_448)).to_f32();
            let mut rect = [0f32; 4];
            rect[0] = left;
            rect[3] = stretch_x_call(this, f(0x40A0_0000)).fsubr_m(sy).to_f32();
            rect[2] = stretch_x_call(this, f(0x4200_0000)).fadd_m(left).to_f32();
            rect[1] = stretch_x_call(this, f(0x423C_0000)).fadd_m(sy).to_f32();
            let mut c = 0u32;
            let cp = rgba_ctor(local(&mut c), 0xFF, 0xFF, 0xFF, 0xFF);
            sprite2d_draw_rect(this.byte_add(0xF8), local(&mut rect), cp);
        }
    }

    // helper text of the video mode / anti-aliasing items
    let cur_name = || item(this, rd(this, 0x54)).byte_add(1);
    if rd::<i32>(this, 0xD8) == rd::<i32>(this, 0xD4) && name_is(cur_name(), key::FED_RES) && rd::<i32>(this, 0x1B18) == 1 {
        reset_helper_text_call(this);
    }
    if rd::<i32>(this, 0xCC) == rd::<i32>(this, 0xC8) && name_is(cur_name(), key::FED_AAS) && rd::<i32>(this, 0x1B18) == 1 {
        reset_helper_text_call(this);
    }
    if rd::<i32>(this, 0xD8) != rd::<i32>(this, 0xD4) && name_is(cur_name(), key::FED_RES) {
        set_helper_text_call(this, 1);
    }
    if rd::<i32>(this, 0xCC) != rd::<i32>(this, 0xC8) && name_is(cur_name(), key::FED_AAS) {
        set_helper_text_call(this, 1);
    }
    let applied: i32 = rd(this, 0xC8);
    if rd::<i32>(this, 0xCC) != applied && !name_is(cur_name(), key::FED_AAS) && matches!(b8(this, 0x15D), 4 | 0x1B) {
        wr(this, 0xCC, applied);
        set_helper_text_call(this, 3);
    }
    let applied: i32 = rd(this, 0xD4);
    if rd::<i32>(this, 0xD8) != applied && !name_is(cur_name(), key::FED_RES) && matches!(b8(this, 0x15D), 4 | 0x1B) {
        wr(this, 0xD8, applied);
        set_helper_text_call(this, 3);
    }

    // sliders
    match b8(item(this, i), 0) {
        0x1B => {
            let value = xi(rd(this, 0x3C)).fmul_m(kk::BRIGHT).to_f32();
            slider(this, i, draw_items, value, SliderKind::Plain(7, 6));
        }
        0x1C => {
            let value = xi(i32::from(b8(this, 0x50) as i8)).fmul_m(kk::VOL).to_f32();
            slider(this, i, draw_items, value, SliderKind::Radio);
        }
        0x1D => {
            let value = xi(i32::from(b8(this, 0x4F) as i8)).fmul_m(kk::VOL).to_f32();
            slider(this, i, draw_items, value, SliderKind::Plain(0xD, 0xC));
        }
        0x3D => {
            let value = x(rd(this, 0x40)).fsub_m(kk::DIST_MIN).fmul_m(kk::DIST_SCALE).to_f32();
            slider(this, i, draw_items, value, SliderKind::Plain(9, 8));
        }
        0x3E => {
            let value = x(g::<f32>(MOUSE_ACCEL).read()).fdiv_m(kk::MOUSE_MIN).to_f32();
            slider(this, i, draw_items, value, SliderKind::Mouse);
        }
        _ => {}
    }

    // the y accumulator
    if !text.is_null() {
        // Both dwords are static (each read only at 0x409B38/0x409B3E): phases B/C use their result.
        let px = if sa_core::PHASE_A {
            g::<u32>(PROT_X_A).read().wrapping_sub(g::<u32>(PROT_X_B).read()).wrapping_sub(1)
        } else {
            PROT_X_VALUE
        };
        let n = font_get_number_lines(px, item_y_pos.to_bits(), text) as u16 as i16;
        let n29 = i32::from(n).wrapping_mul(0x1D);
        let mut v = xi(n29);
        if n29 < 0 {
            v = v.fadd_m(K_2_32);
        }
        *item_y_pos = v.fadd_m(*item_y_pos).to_f32();
    }
    if b8(item(this, i), 0) == 0x20 {
        *item_y_pos = x(*item_y_pos).fadd_m(K_70).to_f32();
    }
}

/// Cuts the GXT video mode name before its first `'('` and the blanks before it (0x57A076..0x57A0C7).
fn trim_video_mode(buf: &mut [u8; RIGHT_BUF]) {
    let mut n = 0usize;
    if buf[0] == 0 {
        return;
    }
    loop {
        if buf[n] == b'(' {
            break;
        }
        let c = buf[n + 1];
        n += 1;
        if c == 0 {
            return;
        }
    }
    while n > 0 && buf[n - 1] == b' ' {
        n -= 1;
    }
    buf[n] = 0;
}

/// The mouse states a slider sets.
#[derive(Clone, Copy)]
enum SliderKind {
    /// Bar at y 125..150: (left arrow, right) states; the right state needs x >= StretchX(500) and y in
    /// the bar.
    Plain(u32, u32),
    /// The radio volume bar at y 95..120 (states 0xB / 0xA).
    Radio,
    /// The mouse sensitivity bar (states 0xF / 0xE, the right state tests x only).
    Mouse,
}

/// One slider of `DrawStandardMenus` (cases of the switch 0x57B3E8): `DisplaySlider(StretchX(500),
/// StretchY(top), StretchY(4), StretchY(20), StretchX(100), value, _ftol(StretchX(3)))` (inline
/// stretches), then for the current item when items are drawn the mouse state +0x1B00 from
/// `CheckHover(0, pos - StretchX(3), top, bottom)` (left arrow) or `CheckHover(pos + StretchX(3),
/// StretchX(W), top, bottom)` (right part), top / bottom = `_ftol(StretchY(125 / 150))` (95 / 120 for the
/// radio volume), else 0x10.
fn slider(this: GPtr<u8>, i: i32, draw_items: bool, value: f32, kind: SliderKind) {
    let radio = matches!(kind, SliderKind::Radio);
    let s3 = if w() == 0x280 { x(K_3) } else { xi(w()).fmul_m(kk::X3) };
    let s100 = isx(f(0x42C8_0000), kk::X100);
    let (s20, s4, stop) = if h() == 0x1C0 {
        (f(0x41A0_0000), f(0x4080_0000), if radio { f(0x42BE_0000) } else { f(0x42FA_0000) })
    } else {
        let hh = xi(h());
        (
            hh.fmul_m(kk::Y20).to_f32(),
            hh.fmul_m(kk::Y4).to_f32(),
            hh.fmul_m(if radio { kk::Y95 } else { kk::Y125 }).to_f32(),
        )
    };
    let s500 = isx(f(0x43FA_0000), kk::X500);
    let spacing = ftol32(s3);
    let pos = display_slider_call(this, s500, stop, s4, s20, s100, value, spacing);
    if i != rd::<i32>(this, 0x54) || !draw_items {
        return;
    }
    let (top_c, bottom_c) = if radio { (f(0x42BE_0000), f(0x42F0_0000)) } else { (f(0x42FA_0000), f(0x4316_0000)) };
    let (left_state, right_state) = match kind {
        SliderKind::Plain(l, r) => (l, r),
        SliderKind::Radio => (0xB, 0xA),
        SliderKind::Mouse => (0xF, 0xE),
    };
    let posf = xi(pos).to_f32();
    let bottom = ftol32(stretch_y_call(this, bottom_c));
    let top = ftol32(stretch_y_call(this, top_c));
    let r = ftol32(stretch_x_call(this, K_3).fsubr_m(posf));
    if check_hover_call(this, 0, r, top, bottom) != 0 {
        wr::<u32>(this, 0x1B00, left_state);
        return;
    }
    let bottom = ftol32(stretch_y_call(this, bottom_c));
    let top = ftol32(stretch_y_call(this, top_c));
    let r = ftol32(stretch_x_call(this, xi(w()).to_f32()));
    let l = ftol32(stretch_x_call(this, K_3).fadd_m(posf));
    let mut keep = false;
    if check_hover_call(this, l, r, top, bottom) != 0 {
        wr::<u32>(this, 0x1B00, right_state);
        let mx = xi(rd(this, 0xBC)).to_f32();
        let x_ok = stretch_x_call(this, f(0x43FA_0000)).fcom(x(mx)).test_any(0x41);
        keep = match kind {
            SliderKind::Mouse => x_ok,
            _ => {
                x_ok && {
                    let my = xi(rd(this, 0xC0)).to_f32();
                    stretch_y_call(this, top_c).fcom(x(my)).test_any(0x41)
                        && stretch_y_call(this, bottom_c).fcom(x(my)).test_parity(0x05)
                }
            }
        };
    }
    if !keep {
        wr::<u32>(this, 0x1B00, 0x10);
    }
}

// ---------------------------------------------------------------------------------------------------
// DrawBackground
// ---------------------------------------------------------------------------------------------------

/// `DrawBackground`'s state (statics of the original): the background sprite byte 0xBA8298, the guard
/// dword 0xBA8294 (bit 0 error time, bit 1 scan time), the controller error start 0xBA8290 and its
/// pending-reset byte 0xBA828C, the user track scan start 0xBA8288, the scan's "restart time" byte
/// 0x8CDFFA, progress direction 0x8CDFFC and position 0x8CE000.
const BG_SPRITE: usize = 0xBA8298;
const BG_GUARD: usize = 0xBA8294;
const BG_ERROR_START: usize = 0xBA8290;
const BG_ERROR_RESET: usize = 0xBA828C;
const BG_SCAN_START: usize = 0xBA8288;
const BG_SCAN_RESTART: usize = 0x8CDFFA;
const BG_PROGRESS_DIR: usize = 0x8CDFFC;
const BG_PROGRESS_POS: usize = 0x8CE000;
/// `CTimer::m_snTimeInMillisecondsPauseMode` (0xB7CB7C), `CTimer::m_FrameCounter` (0xB7CB4C),
/// `Scene.m_pRwCamera` (0xC1703C), `AEUserRadioTrackManager` (0xB6B970) and its scan state (+0x18,
/// 0xB6B988)
const TIME_PAUSE_MODE: usize = 0xB7CB7C;
const FRAME_COUNTER: usize = 0xB7CB4C;
const SCENE_CAMERA: usize = 0xC1703C;
const USER_RADIO_TRACKS: usize = 0xB6B970;
const USER_RADIO_SCAN_STATE: usize = 0xB6B988;
/// `CPad::NewKeyState.esc` / `OldKeyState.esc`, `NewMouseControllerState.lmb`
const KEY_ESC: (usize, usize) = (0xB733A8, 0xB73138);
const MOUSE_LMB: usize = 0xB73418;

/// Screen (0..0x27) -> case of the sprite switch (byte table 0x57C24C), case -> sprite (0x57C22C).
const BG_SPRITE_CASE: [u8; 40] = [
    0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x7, 0x7, 0x7, 0x2, 0x2, 0x7, 0x7, 0x7, 0x7, 0x7, 0x2, 0x7, 0x7, 0x7, 0x7, 0x7, 0x7,
    0x7, 0x7, 0x7, 0x1, 0x4, 0x0, 0x7, 0x7, 0x7, 0x7, 0x1, 0x7, 0x0, 0x6, 0x7, 0x5, 0x6,
];
const BG_SPRITES: [u8; 8] = [0xF, 0xE, 0xD, 0x10, 0x12, 0, 0x11, 0x13];

orig! {
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    pub fn sprite2d_draw_rect_static(rect: GPtr<u8>, colour: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `CMenuManager::PrintMap()` (0x575130, thiscall)
    pub fn print_map(this: GPtr<u8>) = thiscall 0x575130 => crate::menu_map::print_map;
    /// `CMenuManager::DrawControllerSetupScreen()` (0x57F300, thiscall)
    pub fn draw_controller_setup_screen(this: GPtr<u8>) = thiscall 0x57F300 => unported;
    /// `CMenuManager::DrawQuitGameScreen()` (0x57D860, thiscall)
    pub fn draw_quit_game_screen_call(this: GPtr<u8>) = thiscall 0x57D860 => crate::menu_mgr::draw_quit_game_screen;
    /// `CMenuManager::DrawStandardMenus(bool)` (0x5794A0, thiscall)
    pub fn draw_standard_menus_call(this: GPtr<u8>, draw_title: u32) = thiscall 0x5794A0 => draw_standard_menus;
    /// `CMenuManager::SmallMessageScreen(const char*)` (0x574010, thiscall)
    pub fn small_message_screen_call(this: GPtr<u8>, key: GPtr<u8>) = thiscall 0x574010 => crate::menu_mgr::small_message_screen;
    /// `RsCameraBeginUpdate(RwCamera*)` (0x619450)
    pub fn rs_camera_begin_update(cam: u32) -> i32 = cdecl 0x619450 => unported;
    /// 0x727260 (`CSprite2d::SetRecipNearClip` in the catalogue)
    pub fn sprite2d_727260() = cdecl 0x727260 => unported;
    /// 0x727350 (`CSprite2d::InitPerFrame` in the catalogue)
    pub fn sprite2d_727350() = cdecl 0x727350 => unported;
    /// `CFont::InitPerFrame()` (0x719800)
    pub fn font_init_per_frame() = cdecl 0x719800 => crate::font::init_per_frame;
    /// `DefinedState2d()` (0x734750)
    pub fn defined_state_2d() = cdecl 0x734750 => unported;
    /// `CFont::DrawFonts()` (0x71A210)
    pub fn font_draw_fonts() = cdecl 0x71A210 => crate::font_print::draw_fonts;
    /// `CFont::RenderFontBuffer()` (0x719840)
    pub fn font_render_font_buffer() = cdecl 0x719840 => crate::font_print::render_font_buffer;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: u32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::IsEscJustPressed()` (0x572DB0; the callers pass a pad in ECX, unused)
    pub fn is_esc_just_pressed_call() -> u8 = cdecl 0x572DB0 => crate::menu_mgr::is_esc_just_pressed;
    /// `CAEUserRadioTrackManager::ScanUserTracks()` (0x4F4BA0, thiscall)
    pub fn user_radio_scan_user_tracks(this: GPtr<u8>) -> u8 = thiscall 0x4F4BA0 => unported;
    /// `CAEUserRadioTrackManager::Initialise()` (0x4F35B0, thiscall)
    pub fn user_radio_initialise(this: GPtr<u8>) = thiscall 0x4F35B0 => unported;
}

/// The four per-frame calls before a message screen (`RsCameraBeginUpdate` succeeded), a white `CRGBA`
/// built and not used, then `SmallMessageScreen(key)` and `DrawFonts`.
fn bg_message_screen(this: GPtr<u8>, key: usize) {
    sprite2d_727260();
    sprite2d_727350();
    font_init_per_frame();
    defined_state_2d();
    let mut c = 0u32;
    rgba_ctor(local(&mut c), 0xFF, 0xFF, 0xFF, 0xFF);
    small_message_screen_call(this, g(key));
    font_draw_fonts();
}

fn esc_just_pressed() -> bool {
    g::<u16>(KEY_ESC.0).read() != 0 && g::<u16>(KEY_ESC.1).read() == 0
}

fn error_elapsed() -> u32 {
    g::<u32>(TIME_PAUSE_MODE).read().wrapping_sub(g::<u32>(BG_ERROR_START).read())
}

/// `CMenuManager::DrawBackground()` (0x57B750, thiscall): nothing until the textures are loaded
/// (+0x15C).
///
/// The background sprite (0xBA8298) by screen (tables 0x57C24C / 0x57C22C: 0xF screens 0, 0x1C, 0x23;
/// 0xE 1, 0x1A, 0x21; 0xD 2, 9, 0xA, 0x10; 0x10 screen 3; 0x12 4, 0x1B; none 5, 0x26; 0x11 0x24, 0x27;
/// else 0x13) and its rect by sprite: 0x10 `{W/2 - StretchX(128), StretchY(256), W/2 + StretchX(128),
/// 0}` (W/2 integer), 0x13 `{W - StretchX(300), StretchY(200), W, 0}`, the others `{W - StretchX(256),
/// StretchY(256), W, 0}`. A black rect `{-5, H + 5, W + 5, -5}`, the sprite in white. The map screen 5
/// runs `PrintMap` with +0x64..+0x6C saved and restored. Screen 0x26 `DrawControllerSetupScreen`,
/// 0x2B `DrawQuitGameScreen`, else `DrawStandardMenus(true)`.
///
/// Controller error (+0x1AEA): start time 0xBA8290 set once (guard bit 0) and again after a reset
/// (0xBA828C); the message screen FEC_ER3 (1) / FEC_ER2 (2) / FEC_ERI (else) through
/// `SmallMessageScreen` when `RsCameraBeginUpdate` succeeds (with `DrawFonts`), then `DrawFonts`; after
/// 7 s, or escape just pressed after 1 s, +0x1AEA is cleared and the reset byte set. Return.
///
/// User track scan (+0x1AEB): start time 0xBA8288 set once (guard bit 1); FEA_SMP as above (one
/// `DrawFonts`); every frame with frame counter bit 2 the progress position 0x8CE000 moves by -dir
/// (dir 1 from 370 up, -1 from 270 down); a grey (0x32) bar `{StretchX(270), StretchY(250),
/// StretchX(375), StretchY(245)}` and a light (0xE1) block `{pos, StretchY(250), pos + 5,
/// StretchY(245)}` (inline stretches; the block's x as `pos * W * (1/640)`), `DrawFonts`, the helper
/// text reset, the start time restarted when 0x8CDFFA is set. Unless `ScanUserTracks()` returns
/// non-zero or escape was just pressed, return; then (after 3 s or escape) by the scan state 0xB6B988:
/// 2 or 3 -> 0; state 2 without escape: `Initialise` and helper text 5; state 3, or escape: helper text
/// 4; either clears +0x1AEB and sets 0x8CDFFA. `DisplayHelperText(null)`, return.
///
/// Otherwise, with the mouse shown (+0xB8): `RenderFontBuffer`, then on the map screen with the left
/// button down inside `(StretchX(60), W - StretchX(60)) x (StretchY(60), H - StretchY(60))` (`_ftol`
/// bounds, strict) the grab cursor (sprite +0x158), else the arrow (sprite +0x154), see
/// [`draw_pointer`].
///
/// Differences from gta-reversed (the bytes win): the background rect is chosen by the sprite (gr
/// compares the sprite with screen ids); the error / scan messages go through `SmallMessageScreen`
/// with the camera begin-update and per-frame calls (gr: `MessageScreen`), `DrawFonts` instead of
/// `RenderFontBuffer` (twice for the error screen when the camera update succeeds); the scan's end
/// conditions and helper texts follow the bytes above; gr reads the pad before the error branch.
#[hook(addr = 0x57B750, conv = thiscall, name = "CMenuManager::DrawBackground", expect = "83 EC 30 56 8B F1")]
pub fn draw_background(this: GPtr<u8>) {
    if b8(this, 0x15C) == 0 {
        return;
    }
    let scr = i32::from(b8(this, 0x15D) as i8) as u32;
    let sprite = if scr <= 0x27 { BG_SPRITES[BG_SPRITE_CASE[scr as usize] as usize] } else { 0x13 };
    let wi = w();
    g::<u8>(BG_SPRITE).write(sprite);
    let big = f(0x4974_2400);
    let mut rect: [f32; 4] = [big, -big, -big, big];
    if sprite != 0 && (0xD..=0x13).contains(&sprite) {
        rect = match sprite {
            0x10 => {
                let half = xi(wi / 2).to_f32();
                let l = stretch_x_call(this, f(0x4300_0000)).fsubr_m(half).to_f32();
                let r = stretch_x_call(this, f(0x4300_0000)).fadd_m(half).to_f32();
                let b = stretch_y_call(this, f(0x4380_0000)).to_f32();
                [l, b, r, 0.0]
            }
            0x13 => {
                let wf = xi(w()).to_f32();
                let l = stretch_x_call(this, f(0x4396_0000)).fsubr_m(wf).to_f32();
                let b = stretch_y_call(this, f(0x4348_0000)).to_f32();
                [l, b, wf, 0.0]
            }
            _ => {
                let wf = xi(w()).to_f32();
                let l = stretch_x_call(this, f(0x4380_0000)).fsubr_m(wf).to_f32();
                let b = stretch_y_call(this, f(0x4380_0000)).to_f32();
                [l, b, wf, 0.0]
            }
        };
    }
    let mut screen_rect = [f(0xC0A0_0000), xi(h().wrapping_add(5)).to_f32(), xi(wi.wrapping_add(5)).to_f32(), f(0xC0A0_0000)];
    let mut c = 0u32;
    let cp = rgba_ctor(local(&mut c), 0, 0, 0, 0xFF);
    sprite2d_draw_rect_static(local(&mut screen_rect), cp);
    if g::<u8>(BG_SPRITE).read() != 0 {
        let mut c = 0u32;
        let cp = rgba_ctor(local(&mut c), 0xFF, 0xFF, 0xFF, 0xFF);
        let s = isize::from(g::<u8>(BG_SPRITE).read());
        sprite2d_draw_rect(this.byte_add(s * 4 + 0xF8), local(&mut rect), cp);
    }
    if b8(this, 0x15D) == 5 {
        let a: u32 = rd(this, 0x68);
        let b: u32 = rd(this, 0x64);
        let cc: u32 = rd(this, 0x6C);
        print_map(this);
        wr(this, 0x64, b);
        wr(this, 0x68, a);
        wr(this, 0x6C, cc);
    }
    match b8(this, 0x15D) {
        0x26 => draw_controller_setup_screen(this),
        0x2B => draw_quit_game_screen_call(this),
        _ => draw_standard_menus_call(this, 1),
    }

    // controller error
    let err = b8(this, 0x1AEA);
    if err != 0 {
        let guard = g::<u8>(BG_GUARD).read();
        let now = g::<u32>(TIME_PAUSE_MODE).read();
        if guard & 1 == 0 {
            g::<u32>(BG_GUARD).write(g::<u32>(BG_GUARD).read() | 1);
            g::<u32>(BG_ERROR_START).write(now);
        }
        if g::<u8>(BG_ERROR_RESET).read() != 0 {
            g::<u32>(BG_ERROR_START).write(now);
            g::<u8>(BG_ERROR_RESET).write(0);
        }
        let key = match err as i8 {
            1 => 0x86556C,
            2 => 0x865574,
            _ => 0x86557C,
        };
        if rs_camera_begin_update(g::<u32>(SCENE_CAMERA).read()) != 0 {
            bg_message_screen(this, key);
        }
        font_draw_fonts();
        let clear = error_elapsed() > 0x1B58 || {
            get_pad(u32::from(b8(this, 0xEA)));
            esc_just_pressed() && error_elapsed() > 0x3E8
        };
        if clear {
            wr::<u8>(this, 0x1AEA, 0);
            g::<u8>(BG_ERROR_RESET).write(1);
        }
        return;
    }

    // user track scan
    if b8(this, 0x1AEB) != 0 {
        draw_scan_screen(this);
        return;
    }

    // the mouse pointer
    if b8(this, 0xB8) == 0 {
        return;
    }
    font_render_font_buffer();
    let top = ftol32(stretch_y_call(this, f(0x4270_0000)));
    let hf = xi(h()).to_f32();
    let bottom = ftol32(stretch_y_call(this, f(0x4270_0000)).fsubr_m(hf));
    let left = ftol32(stretch_x_call(this, f(0x4270_0000)));
    let wf = xi(w()).to_f32();
    let right = ftol32(stretch_x_call(this, f(0x4270_0000)).fsubr_m(wf));
    if b8(this, 0x15D) == 5 {
        let lmb = g::<u8>(MOUSE_LMB).read();
        get_pad(u32::from(b8(this, 0xEA)));
        if lmb != 0 {
            let my: i32 = rd(this, 0xC0);
            if my > top && my < bottom {
                let mx: i32 = rd(this, 0xBC);
                if mx > left && mx < right {
                    draw_pointer(this, 0x158, mx, my);
                    return;
                }
            }
        }
    }
    let mx: i32 = rd(this, 0xBC);
    let my: i32 = rd(this, 0xC0);
    draw_pointer(this, 0x154, mx, my);
}

/// The pointer sprite at `this + off`: a (100, 100, 100, 50) shadow `{mx + StretchX(6), my +
/// StretchY(21), mx + StretchX(24), my + StretchY(3)}`, then the pointer `{mx, my + StretchY(18), mx +
/// StretchX(18), my}` in white with the position read again.
fn draw_pointer(this: GPtr<u8>, off: isize, mxi: i32, myi: i32) {
    let mx = xi(mxi).to_f32();
    let my = xi(myi).to_f32();
    let mut rect = [0f32; 4];
    rect[0] = stretch_x_call(this, f(0x40C0_0000)).fadd_m(mx).to_f32();
    rect[3] = stretch_y_call(this, K_3).fadd_m(my).to_f32();
    rect[2] = stretch_x_call(this, f(0x41C0_0000)).fadd_m(mx).to_f32();
    rect[1] = stretch_y_call(this, f(0x41A8_0000)).fadd_m(my).to_f32();
    let mut c = 0u32;
    let cp = rgba_ctor(local(&mut c), 0x64, 0x64, 0x64, 0x32);
    let sprite = this.byte_add(off);
    sprite2d_draw_rect(sprite, local(&mut rect), cp);
    let mx = xi(rd(this, 0xBC)).to_f32();
    rect[0] = mx;
    let my = xi(rd(this, 0xC0)).to_f32();
    rect[3] = my;
    rect[2] = stretch_x_call(this, f(0x4190_0000)).fadd_m(mx).to_f32();
    rect[1] = stretch_y_call(this, f(0x4190_0000)).fadd_m(my).to_f32();
    let mut c = 0u32;
    let cp = rgba_ctor(local(&mut c), 0xFF, 0xFF, 0xFF, 0xFF);
    sprite2d_draw_rect(sprite, local(&mut rect), cp);
}

/// The user track scan branch of [`draw_background`] (0x57BC19..0x57BF61).
fn draw_scan_screen(this: GPtr<u8>) {
    if g::<u8>(BG_GUARD).read() & 2 == 0 {
        g::<u32>(BG_GUARD).write(g::<u32>(BG_GUARD).read() | 2);
        g::<u32>(BG_SCAN_START).write(g::<u32>(TIME_PAUSE_MODE).read());
    }
    if rs_camera_begin_update(g::<u32>(SCENE_CAMERA).read()) != 0 {
        bg_message_screen(this, 0x865564);
    }
    if g::<u8>(FRAME_COUNTER).read() & 4 != 0 {
        let p = g::<i32>(BG_PROGRESS_POS).read().wrapping_sub(g::<i32>(BG_PROGRESS_DIR).read());
        g::<i32>(BG_PROGRESS_POS).write(p);
        if p >= 0x172 {
            g::<i32>(BG_PROGRESS_DIR).write(1);
        }
        if p <= 0x10E {
            g::<i32>(BG_PROGRESS_DIR).write(-1);
        }
    }
    let y250 = || if h() == 0x1C0 { x(f(0x437A_0000)) } else { xi(h()).fmul_m(f(0x3F0E_DB6E)) };
    let y245 = || if h() == 0x1C0 { x(f(0x4375_0000)) } else { xi(h()).fmul_m(f(0x3F0C_0000)) };
    let x375 = if w() == 0x280 { x(f(0x43BB_8000)) } else { xi(w()).fmul_m(f(0x3F16_0000)) };
    let x270 = if w() == 0x280 { x(f(0x4387_0000)) } else { xi(w()).fmul_m(f(0x3ED8_0000)) };
    let mut rect = [x270.to_f32(), y250().to_f32(), x375.to_f32(), y245().to_f32()];
    let mut c = 0u32;
    let cp = rgba_ctor(local(&mut c), 0x32, 0x32, 0x32, 0xFF);
    sprite2d_draw_rect_static(local(&mut rect), cp);
    let px = |v: i32| if w() == 0x280 { xi(v) } else { xi(v).fmul(xi(w())).fmul_m(K_INV_640) };
    let mut rect = [
        px(g::<i32>(BG_PROGRESS_POS).read()).to_f32(),
        y250().to_f32(),
        px(g::<i32>(BG_PROGRESS_POS).read().wrapping_add(5)).to_f32(),
        y245().to_f32(),
    ];
    let mut c = 0u32;
    let cp = rgba_ctor(local(&mut c), 0xE1, 0xE1, 0xE1, 0xFF);
    sprite2d_draw_rect_static(local(&mut rect), cp);
    font_draw_fonts();
    reset_helper_text_call(this);
    if g::<u8>(BG_SCAN_RESTART).read() != 0 {
        g::<u32>(BG_SCAN_START).write(g::<u32>(TIME_PAUSE_MODE).read());
        g::<u8>(BG_SCAN_RESTART).write(0);
    }
    if user_radio_scan_user_tracks(g(USER_RADIO_TRACKS)) == 0 {
        get_pad(u32::from(b8(this, 0xEA)));
        if !esc_just_pressed() {
            return;
        }
    }
    if g::<u32>(TIME_PAUSE_MODE).read().wrapping_sub(g::<u32>(BG_SCAN_START).read()) <= 0xBB8 {
        get_pad(u32::from(b8(this, 0xEA)));
        if is_esc_just_pressed_call() == 0 {
            return;
        }
    }
    let mut help = None;
    let st = g::<i32>(USER_RADIO_SCAN_STATE).read();
    if st >= 2 {
        if st <= 3 {
            g::<i32>(USER_RADIO_SCAN_STATE).write(0);
        }
        if st == 2 {
            get_pad(u32::from(b8(this, 0xEA)));
            if is_esc_just_pressed_call() == 0 {
                user_radio_initialise(g(USER_RADIO_TRACKS));
                help = Some(5);
            }
        }
    }
    if help.is_none() {
        let st = g::<i32>(USER_RADIO_SCAN_STATE).read();
        if (2..=3).contains(&st) {
            g::<i32>(USER_RADIO_SCAN_STATE).write(0);
        }
        if st == 3 {
            help = Some(4);
        } else {
            get_pad(u32::from(b8(this, 0xEA)));
            if is_esc_just_pressed_call() != 0 {
                help = Some(4);
            }
        }
    }
    if let Some(hh) = help {
        wr::<u8>(this, 0x1AEB, 0);
        g::<u8>(BG_SCAN_RESTART).write(1);
        set_helper_text_call(this, hh);
    }
    display_helper_text_call(this, GPtr::NULL);
}

// ---------------------------------------------------------------------------------------------------
// DrawWindowedText, DisplaySlider
// ---------------------------------------------------------------------------------------------------

/// 10.0 (0x85862C), 4.0 (0x858B90), 22.0 (0x858F8C), 22/448 (0x865430), 1.4 (0x859994), 1.1 (0x858F14),
/// 16.0 (0x8599D0), 20.0 (0x858BA4), 15.0 (0x858B48), 2.0 (0x858CA0), 2/448 (0x865398), 2/640 (0x865394),
/// 1/32 (0x864E10)
const K_10: f32 = f(0x4120_0000);
const K_4: f32 = f(0x4080_0000);
const K_22: f32 = f(0x41B0_0000);
const K_Y22: f32 = f(0x3D49_2492);
const K_1_4: f32 = f(0x3FB3_3333);
const K_1_1: f32 = f(0x3F8C_CCCD);
const K_16: f32 = f(0x4180_0000);
const K_20: f32 = f(0x41A0_0000);
const K_15: f32 = f(0x4170_0000);
const K_2: f32 = f(0x4000_0000);
const K_Y2: f32 = f(0x3B92_4925);
const K_X2: f32 = f(0x3B4C_CCCD);
const K_1_32: f32 = f(0x3D00_0000);

orig! {
    /// `CFont::GetTextRect(CRect*, float, float, const GxtChar*)` (0x71A620)
    pub fn font_get_text_rect(rect: GPtr<f32>, px: f32, py: f32, text: GPtr<u8>) = cdecl 0x71A620 => crate::font_print::get_text_rect;
}

/// `W * (1/640) * 10` (x87, no 640 test), as `DrawWindowedText` computes its margin.
fn wt_margin() -> X87 {
    xi(w()).fmul_m(K_INV_640).fmul_m(K_10)
}

/// The wrap / right wrap / centre size / font / alignment / scale block `DrawWindowedText` runs twice.
fn wt_layout(xw: f32, x_minus_wrap: f32, wrap: f32, align: u32) {
    font_set_wrapx(wt_margin().fsubr_m(xw).to_f32().to_bits());
    font_set_right_justify_wrap(wt_margin().fadd_m(x_minus_wrap).to_f32().to_bits());
    let m = wt_margin();
    font_set_centre_size(m.fadd(m).fsubr_m(wrap).to_f32().to_bits());
    font_set_font_style(1);
    font_set_orientation(align);
    let sy = isy(1.0, K_INV_448);
    let sx = isx(f(0x3F33_3333), kk::X0_7);
    font_set_scale(sx.to_bits(), sy.to_bits());
}

/// `CMenuManager::DrawWindowedText(float x, float y, float wrap, const char* title, const char*
/// message, eFontAlignment align)` (0x578F50, thiscall, `ret 0x18`; `this` unused).
///
/// With `m = W * (1/640) * 10` (x87, not the inline 640 test): wrap `(x + wrap) - m`, right wrap `m +
/// (x - wrap)`, centre size `wrap - 2m`, font 1, `align`, scale (inline 0.7, 1.0); the message's text
/// rect from `GetTextRect` at (x, y), its left - 4 and +0xC + `StretchY(22)` (inline), drawn in (0, 0,
/// 0, 0xBE). Colour 0xE1 grey on black, edge 2, left, font 0, language scale `(W/640 * 1.1, H/448 *
/// 1.4)`, wrap at the rect's +8. A non-empty title prints at `(left + W/640 * 20, +4 - H/448 * 16)`. A
/// non-empty message: the first block again, HUD colour 3, drop shadow 2, black drop colour, printed at
/// `(x, y + H/448 * 15)`.
///
/// Differences from gta-reversed (the bytes win): the right wrap is `m + (x - wrap)` (gr `StretchX(10)
/// + wrap`); the margins and the second scale are not inline-stretched (`W/640 * k` always). The
/// original keeps `CRGBA` temporaries in its own argument slots (x, wrap); callers do not read them.
#[hook(addr = 0x578F50, conv = thiscall, name = "CMenuManager::DrawWindowedText", expect = "83 EC 1C D9 44 24 20")]
pub fn draw_windowed_text(_this: GPtr<u8>, px: f32, py: f32, wrap: f32, title: GPtr<u8>, message: GPtr<u8>, align: u32) {
    let xw = x(px).fadd_m(wrap).to_f32();
    let big = f(0x4974_2400);
    let mut rect: [f32; 4] = [big, -big, -big, big];
    font_set_wrapx(wt_margin().fsubr_m(xw).to_f32().to_bits());
    let x_minus_wrap = x(px).fsub_m(wrap).to_f32();
    font_set_right_justify_wrap(wt_margin().fadd_m(x_minus_wrap).to_f32().to_bits());
    let m = wt_margin();
    font_set_centre_size(m.fadd(m).fsubr_m(wrap).to_f32().to_bits());
    font_set_font_style(1);
    font_set_orientation(align);
    let sy = isy(1.0, K_INV_448);
    let sx = isx(f(0x3F33_3333), kk::X0_7);
    font_set_scale(sx.to_bits(), sy.to_bits());
    let text = text_get(g(THE_TEXT), message);
    font_get_text_rect(local(&mut rect).cast(), px, py, text);
    rect[0] = x(rect[0]).fsub_m(K_4).to_f32();
    let s22 = if h() == 0x1C0 { x(K_22) } else { xi(h()).fmul_m(K_Y22) };
    rect[3] = s22.fadd_m(rect[3]).to_f32();
    let mut c = 0u32;
    let cp = rgba_ctor(local(&mut c), 0, 0, 0, 0xBE);
    sprite2d_draw_rect_static(local(&mut rect), cp);
    font_set_color(rgba(0xE1, 0xE1, 0xE1, 0xFF));
    font_set_drop_color(rgba(0, 0, 0, 0xFF));
    font_set_edge(2);
    font_set_orientation(1);
    font_set_font_style(0);
    let sy = xi(h()).fmul_m(K_INV_448).fmul_m(K_1_4).to_f32();
    let sx = xi(w()).fmul_m(K_INV_640).fmul_m(K_1_1).to_f32();
    font_set_scale_for_current_language(sx, sy.to_bits());
    font_set_wrapx(rect[2].to_bits());
    if !title.is_null() && b8(title, 0) != 0 {
        let t = text_get(g(THE_TEXT), title);
        let ty = xi(h()).fmul_m(K_INV_448).fmul_m(K_16).fsubr_m(rect[1]).to_f32();
        let tx = xi(w()).fmul_m(K_INV_640).fmul_m(K_20).fadd_m(rect[0]).to_f32();
        font_print_string(tx, ty, t);
    }
    if !message.is_null() && b8(message, 0) != 0 {
        wt_layout(xw, x_minus_wrap, wrap, align);
        font_set_color(hud_rgb(3));
        font_set_drop_shadow_position(2);
        font_set_drop_color(rgba(0, 0, 0, 0xFF));
        let t = text_get(g(THE_TEXT), message);
        let my = xi(h()).fmul_m(K_INV_448).fmul_m(K_15).fadd_m(py).to_f32();
        font_print_string(px, my, t);
    }
}

/// `CMenuManager::DisplaySlider(float x, float y, float h1, float h2, float length, float value, int32
/// spacing)` (0x576860, thiscall, `ret 0x1C`; `this` unused): 16 bars. Bar i (counter c = 16 - i): `bx =
/// x + length * i * (1/16)`, height `(c * h1 + i * h2) * (1/16)`; selected (colour 0xAC, 0xCB, 0xF1, and
/// the result = `_ftol(bx)`) when `i * (1/16) + 1/32 < value`, else (0x4A, 0x5A, 0x6B). With `mh =
/// max(h1, h2)` (h2 unless h1 > h2): a (0, 0, 0, 0xC8) shadow `{bx + StretchX(2), mh + StretchY(2) + y,
/// spacing + StretchX(2) + bx, mh + y - height + StretchY(2)}` (inline stretches) and the bar `{bx, mh
/// + y, bx + spacing, mh + y - height}` (`spacing` as a float). Returns the last selected bar's x (0
/// when none).
///
/// Differences from gta-reversed: none in the result; the original keeps the bar colour in its
/// `spacing` argument slot (the caller does not read it).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x576860, conv = thiscall, name = "CMenuManager::DisplaySlider", expect = "83 EC 40 DB 44 24 5C")]
pub fn display_slider(_this: GPtr<u8>, px: f32, py: f32, h1: f32, h2: f32, len: f32, value: f32, spacing: i32) -> i32 {
    let spacing_f = xi(spacing).to_f32();
    let mut last = 0i32;
    let max_h = || if x(h1).fcom(x(h2)).test_any(0x41) { h2 } else { h1 };
    for i in 0..16i32 {
        let counter = 16 - i;
        let bx = x(len).fmul(xi(i)).fmul_m(K_1_16).fadd_m(px).to_f32();
        let bh = xi(counter).fmul_m(h1).fadd(xi(i).fmul_m(h2)).fmul_m(K_1_16).to_f32();
        let unselected = x(K_1_16).fmul(xi(i)).fadd_m(K_1_32).fcom(x(value)).test_parity(0x05);
        let colour = if unselected {
            rgba(0x4A, 0x5A, 0x6B, 0xFF)
        } else {
            let c = rgba(0xAC, 0xCB, 0xF1, 0xFF);
            last = ftol32(x(bx));
            c
        };
        let mh = max_h();
        let a = if h() == 0x1C0 { x(K_2) } else { xi(h()).fmul_m(K_Y2) };
        let b = if w() == 0x280 { x(K_2) } else { xi(w()).fmul_m(K_X2) };
        let cc = if h() == 0x1C0 { x(K_2) } else { xi(h()).fmul_m(K_Y2) };
        let d = if w() == 0x280 { x(K_2) } else { xi(w()).fmul_m(K_X2) };
        let mut shadow = [0f32; 4];
        shadow[0] = d.fadd_m(bx).to_f32();
        shadow[3] = x(mh).fadd_m(py).fsub_m(bh).fadd(cc).to_f32();
        shadow[2] = x(spacing_f).fadd(b).fadd_m(bx).to_f32();
        shadow[1] = x(mh).fadd(a).fadd_m(py).to_f32();
        let mut sc = 0u32;
        let sp = rgba_ctor(local(&mut sc), 0, 0, 0, 0xC8);
        sprite2d_draw_rect_static(local(&mut shadow), sp);
        let mh = max_h();
        let mut bar = [0f32; 4];
        bar[3] = x(mh).fadd_m(py).fsub_m(bh).to_f32();
        bar[0] = bx;
        bar[2] = x(spacing_f).fadd_m(bx).to_f32();
        bar[1] = x(mh).fadd_m(py).to_f32();
        let mut col = colour;
        sprite2d_draw_rect_static(local(&mut bar), local(&mut col));
    }
    last
}

// ---------------------------------------------------------------------------------------------------
// PrintRadioStationList, PrintBriefs
// ---------------------------------------------------------------------------------------------------

orig! {
    /// `CSprite2d::Draw(float x, float y, float w, float h, const CRGBA&)` (0x7282C0, thiscall)
    pub fn sprite2d_draw_xywh(this: GPtr<u8>, px: f32, py: f32, sw: f32, sh: f32, colour: GPtr<u8>) = thiscall 0x7282C0 => unported;
    /// `CSprite2d::Draw2DPolygon(x1, y1, x2, y2, x3, y3, x4, y4, const CRGBA&)` (0x7285B0)
    pub fn sprite2d_draw_2d_polygon(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, x4: f32, y4: f32, colour: GPtr<u8>) = cdecl 0x7285B0 => unported;
    /// `CFont::SetJustify(bool)` (0x719600)
    pub fn font_set_justify(v: u32) = cdecl 0x719600 => crate::font::set_justify;
    /// `CMessages::InsertNumberInString(text, n1..n6, out)` (0x69DE90)
    pub fn msg_insert_number_in_string(text: GPtr<u8>, n1: i32, n2: i32, n3: i32, n4: i32, n5: i32, n6: i32, out: GPtr<u8>) = cdecl 0x69DE90 => crate::messages::insert_number_in_string;
    /// `CMessages::InsertStringInString(GxtChar*, const GxtChar*)` (0x69E040)
    pub fn msg_insert_string_in_string(target: GPtr<u8>, repl: GPtr<u8>) = cdecl 0x69E040 => crate::messages::insert_string_in_string;
    /// `CMessages::InsertPlayerControlKeysInString(GxtChar*)` (0x69E160)
    pub fn msg_insert_player_control_keys(s: GPtr<u8>) = cdecl 0x69E160 => crate::messages::insert_player_control_keys_in_string;
}

/// `CMenuManager::PrintRadioStationList()` (0x5746F0, thiscall): the radio station sprites 1..12
/// (`this + 0xF8 + 4 i`) except the current one (+0x52, signed), each at x `44 + 47 (i - 1)` (stretched
/// as `W * x * (1/640)`), y inline `StretchY(300)`, size inline (35, 35), colour (255, 255, 255, 30);
/// then the current station when 1..12: x `47 s - 15`, y `StretchY(290)`, size (60, 60), white.
///
/// Differences from gta-reversed: none (inline stretches with their own constants).
#[hook(addr = 0x5746F0, conv = thiscall, name = "CMenuManager::PrintRadioStationList", expect = "83 EC 18 53 55")]
pub fn print_radio_station_list(this: GPtr<u8>) {
    let mut xpos: i32 = 0x2C;
    for i in 1..=12i32 {
        if i32::from(b8(this, 0x52) as i8) != i {
            let sh = isy(f(0x420C_0000), f(0x3DA0_0000));
            let sw = isx(f(0x420C_0000), f(0x3D60_0000));
            let sy = isy(f(0x4396_0000), f(0x3F2B_6DB7));
            let sx = item_sx(xpos);
            let mut c = 0u32;
            let cp = rgba_ctor(local(&mut c), 0xFF, 0xFF, 0xFF, 0x1E);
            sprite2d_draw_xywh(this.byte_add(0xF8 + 4 * i as isize), sx, sy, sw, sh, cp);
        }
        xpos = xpos.wrapping_add(0x2F);
    }
    let s = b8(this, 0x52) as i8;
    if s > 0 && s < 0xD {
        let sh = isy(f(0x4270_0000), f(0x3E09_2492));
        let sw = isx(f(0x4270_0000), f(0x3DC0_0000));
        let sy = isy(f(0x4391_0000), f(0x3F25_B6DB));
        let sx = item_sx(i32::from(s).wrapping_mul(0x2F).wrapping_sub(0xF));
        let mut c = 0u32;
        let cp = rgba_ctor(local(&mut c), 0xFF, 0xFF, 0xFF, 0xFF);
        let s = i32::from(b8(this, 0x52) as i8);
        sprite2d_draw_xywh(this.byte_add(0xF8 + 4 * s as isize), sx, sy, sw, sh, cp);
    }
}

/// `CMessages::PreviousBriefs` (0xC1A570, 20 entries of 0x20: +0 text, +4 six numbers, +0x1C string)
const PREVIOUS_BRIEFS: u32 = 0xC1A570;
/// `PrintBriefs`' blink time 0xBA8278 and "draw arrows" byte 0x8CDFF9
const BRIEF_BLINK_TIME: usize = 0xBA8278;
const BRIEF_DRAW_ARROWS: usize = 0x8CDFF9;

/// `CMenuManager::PrintBriefs()` (0x576320, thiscall): colour 0xE1 grey on black, left, font 1,
/// language scale (inline 0.49, 0.7), wrap inline `StretchX(560)`, drop shadow 1, no justify. The
/// briefs from the selected row (+8, unsigned byte) down to row - 3 (the bound re-read each step;
/// rows below 0 read before the array as the original does): with a text, numbers and string inserted
/// into `gGxtString` (0xC1B100), the control keys, printed at (inline `StretchX(70)`, y stretched as
/// `H * y * (1/448)`), y from 100 in steps of 70. No justify. With +0x78: every 700 ms of pause-mode
/// time (0xBA8278) the arrows byte 0x8CDFF9 toggles; when set, the up arrow (row < 19 and brief row + 1
/// has text) `(50, 100) (55, 110) (45, 110) (50, 100)` and the down arrow (row > 3) `(50, 348) (55, 338)
/// (45, 338) (50, 348)` as `Draw2DPolygon` in 0xE1 grey (inline stretches).
///
/// Differences from gta-reversed (the bytes win): the blink time is the global 0xBA8278 (gr: a
/// field); the down arrow's condition is row > 3; the rows below 0 are not guarded.
#[hook(addr = 0x576320, conv = thiscall, name = "CMenuManager::PrintBriefs", expect = "83 EC 24 53 56")]
pub fn print_briefs(this: GPtr<u8>) {
    font_set_color(rgba(0xE1, 0xE1, 0xE1, 0xFF));
    font_set_drop_color(rgba(0, 0, 0, 0xFF));
    font_set_orientation(1);
    font_set_font_style(1);
    let sy = isy(f(0x3F33_3333), K_INV_640);
    let sx = isx(f(0x3EFA_E147), kk::X0_49);
    font_set_scale_for_current_language(sx, sy.to_bits());
    font_set_wrapx(isx(f(0x440C_0000), f(0x3F60_0000)).to_bits());
    font_set_drop_shadow_position(1);
    font_set_justify(0);
    let mut row = i32::from(b8(this, 8));
    let mut y: f32 = 100.0;
    if row >= row - 3 {
        loop {
            let e: GPtr<u8> = g(PREVIOUS_BRIEFS.wrapping_add((row as u32).wrapping_mul(0x20)) as usize);
            let text: GPtr<u8> = rd(e, 0);
            if !text.is_null() {
                let n = |k: isize| rd::<i32>(e, 4 + 4 * k);
                msg_insert_number_in_string(text, n(0), n(1), n(2), n(3), n(4), n(5), g(G_GXT_STRING));
                msg_insert_string_in_string(g(G_GXT_STRING), rd(e, 0x1C));
                msg_insert_player_control_keys(g(G_GXT_STRING));
                let py = if h() == 0x1C0 { y } else { xi(h()).fmul_m(y).fmul_m(K_INV_448).to_f32() };
                let px = isx(K_70, f(0x3DE0_0000));
                font_print_string(px, py, g(G_GXT_STRING));
                y = x(y).fadd_m(K_70).to_f32();
            }
            row -= 1;
            if row < i32::from(b8(this, 8)) - 3 {
                break;
            }
        }
    }
    font_set_justify(0);
    if b8(this, 0x78) == 0 {
        return;
    }
    let now = g::<u32>(TIME_PAUSE_MODE).read();
    if now.wrapping_sub(g::<u32>(BRIEF_BLINK_TIME).read()) > 0x2BC {
        g::<u32>(BRIEF_BLINK_TIME).write(now);
        let v = g::<u8>(BRIEF_DRAW_ARROWS).read();
        g::<u8>(BRIEF_DRAW_ARROWS).write(u8::from(v == 0));
    }
    if g::<u8>(BRIEF_DRAW_ARROWS).read() == 0 {
        return;
    }
    let sx50 = || isx(f(0x4248_0000), f(0x3DA0_0000));
    let sx45 = || isx(f(0x4234_0000), f(0x3D90_0000));
    let sx55 = || isx(f(0x425C_0000), f(0x3DB0_0000));
    let r = b8(this, 8);
    if r < 0x13 && rd::<u32>(g::<u8>(PREVIOUS_BRIEFS as usize + 0x20 + usize::from(r) * 0x20), 0) != 0 {
        let y100 = isy(f(0x42C8_0000), f(0x3E64_9249));
        let x50 = sx50();
        let y110 = isy(f(0x42DC_0000), f(0x3E7B_6DB7));
        let x45 = sx45();
        let y110b = isy(f(0x42DC_0000), f(0x3E7B_6DB7));
        let x55 = sx55();
        let y100b = isy(f(0x42C8_0000), f(0x3E64_9249));
        let x50b = sx50();
        let mut c = 0u32;
        let cp = rgba_ctor(local(&mut c), 0xE1, 0xE1, 0xE1, 0xFF);
        sprite2d_draw_2d_polygon(x50b, y100b, x55, y110b, x45, y110, x50, y100, cp);
    }
    if b8(this, 8) > 3 {
        let y348 = isy(f(0x43AE_0000), f(0x3F46_DB6E));
        let x50 = sx50();
        let y338 = isy(f(0x43A9_0000), f(0x3F41_2492));
        let x45 = sx45();
        let y338b = isy(f(0x43A9_0000), f(0x3F41_2492));
        let x55 = sx55();
        let y348b = isy(f(0x43AE_0000), f(0x3F46_DB6E));
        let x50b = sx50();
        let mut c = 0u32;
        let cp = rgba_ctor(local(&mut c), 0xE1, 0xE1, 0xE1, 0xFF);
        sprite2d_draw_2d_polygon(x50b, y348b, x55, y338b, x45, y338, x50, y348, cp);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_DRAW_STANDARD_MENUS,
    HOOK_DRAW_BACKGROUND,
    HOOK_DRAW_WINDOWED_TEXT,
    HOOK_DISPLAY_SLIDER,
    HOOK_PRINT_RADIO_STATION_LIST,
    HOOK_PRINT_BRIEFS,
    HOOK_PRINT_STATS,
];

// ---------------------------------------------------------------------------------------------------
// PrintStats
// ---------------------------------------------------------------------------------------------------

/// `PrintStats`' statics: the scroll position 0x8CDFF4 (float), the page it belongs to 0x8CDFF8, the
/// scroll time 0xBA826C; `CStats` current stat id word 0xB794CC
const STATS_SCROLL_POS: usize = 0x8CDFF4;
const STATS_PAGE: usize = 0x8CDFF8;
const STATS_SCROLL_TIME: usize = 0xBA826C;
const STATS_CURRENT_ID: usize = 0xB794CC;

orig! {
    /// `CStats::ConstructStatLine(int32 line, uint8 page)` (0x55A780; returns the line count)
    pub fn stats_construct_stat_line(line: i32, page: u32) -> i32 = cdecl 0x55A780 => unported;
    /// `CStats::GetStatValue(uint16)` (0x558E40, ST0)
    pub fn stats_get_stat_value_st0(id: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CStats::FindCriminalRatingNumber()` (0x559080)
    pub fn stats_find_criminal_rating_number() -> i32 = cdecl 0x559080 => unported;
    /// `CStats::FindCriminalRatingString()` (0x55A210)
    pub fn stats_find_criminal_rating_string() -> GPtr<u8> = cdecl 0x55A210 => unported;
    /// `GxtCharStrcpy(GxtChar*, const GxtChar*)` (0x718660)
    pub fn gxt_char_strcpy(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x718660 => crate::font_print::gxt_char_strcpy;
    /// `CSprite2d::DrawBarChart(x, y, width, height, progress, progressAdd, drawPercentage, drawBlackBorder,
    /// CRGBA colour, CRGBA addColour)` (0x728640)
    pub fn sprite2d_draw_bar_chart(px: f32, py: f32, bw: u32, bh: u32, progress: f32, add: u32, pct: u32, border: u32, colour: u32, add_colour: u32) = cdecl 0x728640 => unported;
}

/// Inline `StretchY(c)` kept on the x87 stack (not stored): `c` or `fild H * k`.
fn isy87(c: f32, k: f32) -> X87 {
    if h() == 0x1C0 { x(c) } else { xi(h()).fmul_m(k) }
}

/// `CMenuManager::PrintStats()` (0x574900, thiscall): the stats page.
///
/// The page = `_ftol(min(item, 8))` (x87; `min` keeps the item only when below 8, the item +0x54 is
/// not written); a page change resets the scroll position 0x8CDFF4 to -120 (page byte 0x8CDFF8). `n =
/// ConstructStatLine(99999, page)`; font 2, scale (inline 0.3, 0.75). Every 40 ms of pause-mode time
/// (0xBA826C) with a positive speed (+4) the position moves by `StretchY(100) / speed` (inline), up
/// with +0 set, down otherwise. Line i: `y = StretchY(54) * i + StretchY(50) - pos` (x87), raised by
/// `(n + 7) * 54` (stretched as `* H * (1/448)`) while below `StretchY(50)`, lowered while above `H -
/// StretchY(50)`; drawn when `StretchY(50) < y < StretchY(360)`: `ConstructStatLine(i, page)`, y -
/// `StretchY(37)` for an empty `gGxtString`, the alpha (0 outside, `(y - S80) * 8` in (S80, S110), 255
/// in [S110, S300], `(S330 - y) * 8` in (S300, S330), at most 255, `_ftol` low byte), black drop
/// colour, edge 1, centred, 0xE1 grey, `gGxtString` at `(StretchX(450), y)`; then with no current stat
/// id (0xB794CC) `gGxtString2` in (0xAC, 0xCB, 0xF1) at `y + StretchY(17)`, else `DrawBarChart(
/// StretchX(400), y + StretchY(17), _ftol(W/640 * 100), _ftol(H/448 * 10), min(value * 0.001 * 100,
/// 1000), 0, 0, 1, colour, (0, 0, 0, 0))`. Finally edge 1, black / grey, font 3, scale (0.6, 0.8),
/// centred: the criminal rating string at `(StretchX(450), StretchY(40))` and its number (`"%d"` through
/// `gString`) at `(StretchX(450), StretchY(57))`. All stretches inline.
///
/// Differences from gta-reversed (the bytes win): the item is not clamped in place; the scroll time
/// is the global 0xBA826C (gr: a field) and the scroll moves only for a positive speed but the time
/// restarts regardless; the alpha has no lower clamp (a negative alpha goes through `_ftol` and its
/// low byte); the fade bounds compare the f32 y after the 37 shift while the visibility test uses the
/// unrounded x87 y.
///
/// Uninitialised stack: the original passes `ConstructStatLine` its page as a dword whose upper three
/// bytes are residue (the callee reads the byte, `movzx eax, byte [esp+8]`), and `GetStatValue` the id
/// with the upper word of the screen width left in EAX (the callee reads the word); the port passes
/// the zero-extended byte / word.
#[hook(addr = 0x574900, conv = thiscall, name = "CMenuManager::PrintStats", expect = "81 EC 8C 00 00 00")]
pub fn print_stats(this: GPtr<u8>) {
    let mut v = xi(rd(this, 0x54));
    if v.fcom(x(f(0x4100_0000))).test_parity(0x05) {
        v = x(f(0x4100_0000));
    }
    let page = ftol32(v) as u8;
    if g::<u8>(STATS_PAGE).read() != page {
        g::<u32>(STATS_SCROLL_POS).write(0xC2F0_0000);
        g::<u8>(STATS_PAGE).write(page);
    }
    let n = stats_construct_stat_line(0x1869F, u32::from(page));
    font_set_font_style(2);
    let sy = isy(f(0x3F40_0000), f(0x3ADB_6DB7));
    let sx = isx(f(0x3E99_999A), f(0x39F5_C290));
    font_set_scale(sx.to_bits(), sy.to_bits());
    let now = g::<u32>(TIME_PAUSE_MODE).read();
    if now.wrapping_sub(g::<u32>(STATS_SCROLL_TIME).read()) > 0x28 {
        let speed: f32 = rd(this, 4);
        if !x(speed).fcom(x(0.0)).test_any(0x41) {
            let d = isy87(f(0x42C8_0000), f(0x3E64_9249)).fdiv_m(speed);
            let pos = g::<f32>(STATS_SCROLL_POS).read();
            let nv = if b8(this, 0) != 0 { d.fadd_m(pos) } else { d.fsubr_m(pos) };
            g::<f32>(STATS_SCROLL_POS).write(nv.to_f32());
        }
        g::<u32>(STATS_SCROLL_TIME).write(now);
    }
    let k50 = f(0x3DE4_9249);
    let k54 = f(0x3DF6_DB6E);
    for i in 0..n {
        let (s54, s50) = if h() == 0x1C0 {
            (x(f(0x4258_0000)), x(f(0x4248_0000)))
        } else {
            let hf = xi(h()).to_f32();
            (xi(h()).fmul_m(k54), x(hf).fmul_m(k50))
        };
        let mut y = s54.fmul(xi(i)).fadd(s50).fsub_m(g::<f32>(STATS_SCROLL_POS).read());
        let step = |hh: Option<f32>| {
            let d = xi(n.wrapping_add(7)).fmul_m(f(0x4258_0000));
            match hh {
                None if h() == 0x1C0 => d,
                None => d.fmul(xi(h())).fmul_m(K_INV_448),
                Some(_) if h() == 0x1C0 => d,
                Some(hf) => d.fmul_m(hf).fmul_m(K_INV_448),
            }
        };
        loop {
            let t = isy87(f(0x4248_0000), k50);
            if y.fcom(t).test_parity(0x05) {
                break;
            }
            y = y.fadd(step(None));
        }
        let hf = xi(h()).to_f32();
        let s50f = || if h() == 0x1C0 { x(f(0x4248_0000)) } else { x(hf).fmul_m(k50) };
        loop {
            let lim = x(hf).fsub(s50f());
            if y.fcom(lim).test_any(0x41) {
                break;
            }
            y = y.fsub(step(Some(hf)));
        }
        let mut yf = y.to_f32();
        if y.fcom(s50f()).test_any(0x41) {
            continue;
        }
        let t360 = if h() == 0x1C0 { x(f(0x43B4_0000)) } else { x(hf).fmul_m(f(0x3F4D_B6DB)) };
        if y.fcom(t360).test_parity(0x05) {
            continue;
        }
        stats_construct_stat_line(i, u32::from(page));
        if g::<u8>(G_GXT_STRING).read() == 0 {
            yf = x(yf).fsub(isy87(f(0x4214_0000), f(0x3DA9_2492))).to_f32();
        }
        let s330 = || isy87(f(0x43A5_0000), f(0x3F3C_9249));
        let s300 = || isy87(f(0x4396_0000), f(0x3F2B_6DB7));
        let s80 = || isy87(f(0x42A0_0000), f(0x3E36_DB6E));
        let s110 = || isy87(f(0x42DC_0000), f(0x3E7B_6DB7));
        let mut alpha = if !x(yf).fcom(s330()).test_parity(0x05) && !x(yf).fcom(s300()).test_any(0x41) {
            s330().fsub_m(yf).fmul_m(f(0x4100_0000))
        } else {
            x(0.0)
        };
        if !x(yf).fcom(s80()).test_any(0x41) && !x(yf).fcom(s110()).test_parity(0x05) {
            alpha = s80().fsubr_m(yf).fmul_m(f(0x4100_0000));
        }
        let full = !x(yf).fcom(s110()).c0() && !x(yf).fcom(s300()).test_parity(0x41);
        if full || !alpha.fcom(x(f(0x437F_0000))).test_any(0x41) {
            alpha = x(f(0x437F_0000));
        }
        let a = u32::from(ftol32(alpha) as u8);
        font_set_drop_color(rgba(0, 0, 0, a));
        font_set_edge(1);
        font_set_orientation(0);
        font_set_color(rgba(0xE1, 0xE1, 0xE1, a));
        let x450 = || isx(f(0x43E1_0000), f(0x3F34_0000));
        font_print_string(x450(), yf, g(G_GXT_STRING));
        let id = g::<u16>(STATS_CURRENT_ID).read();
        if id == 0 {
            font_set_color(rgba(0xAC, 0xCB, 0xF1, a));
            let s17 = isy87(f(0x4188_0000), f(0x3D1B_6DB7));
            let px = x450();
            font_print_string(px, s17.fadd_m(yf).to_f32(), g(G_GXT_STRING2));
        } else {
            let v = X87::from_f64_exact(stats_get_stat_value_st0(u32::from(id)))
                .fmul_m(f(0x3A83_126F))
                .fmul_m(f(0x42C8_0000));
            let value = if v.fcom(x(f(0x447A_0000))).test_parity(0x05) { f(0x447A_0000) } else { v.to_f32() };
            let s17 = isy(f(0x4188_0000), f(0x3D1B_6DB7));
            let s400 = isx(f(0x43C8_0000), f(0x3F20_0000));
            let add = rgba(0, 0, 0, 0);
            let colour = rgba(0xAC, 0xCB, 0xF1, a);
            let bh = ftol32(xi(h()).fmul_m(K_INV_448).fmul_m(K_10)) as u32;
            let bw = ftol32(xi(w()).fmul_m(K_INV_640).fmul_m(f(0x42C8_0000))) as u32;
            let py = x(s17).fadd_m(yf).to_f32();
            sprite2d_draw_bar_chart(s400, py, bw, bh, value, 0, 0, 1, colour, add);
        }
    }
    font_set_edge(1);
    font_set_drop_color(rgba(0, 0, 0, 0xFF));
    font_set_color(rgba(0xE1, 0xE1, 0xE1, 0xFF));
    font_set_font_style(3);
    let sy = isy(f(0x3F4C_CCCD), f(0x3AEA_0EA1));
    let sx = isx(f(0x3F19_999A), f(0x3A75_C290));
    font_set_scale(sx.to_bits(), sy.to_bits());
    font_set_orientation(0);
    let mut rating = [0u8; 0x50];
    crt_sprintf1(g(G_STRING), g(key::PCT_D), stats_find_criminal_rating_number());
    ascii_to_gxt_char(g(G_STRING), local(&mut rating));
    gxt_char_strcpy(g(G_GXT_STRING), stats_find_criminal_rating_string());
    let py = isy(f(0x4220_0000), f(0x3DB6_DB6E));
    let px = isx(f(0x43E1_0000), f(0x3F34_0000));
    font_print_string(px, py, g(G_GXT_STRING));
    let py = isy(f(0x4264_0000), f(0x3E02_4925));
    let px = isx(f(0x43E1_0000), f(0x3F34_0000));
    font_print_string(px, py, local(&mut rating));
}
