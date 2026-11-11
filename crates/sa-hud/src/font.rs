//! `CFont` (PORT-HUD): the text state setters, per-frame reset, character metrics and sub-font mapping,
//! and `CFontChar::Set`. Ported from the 1.0 US bytes; gta-reversed `Font.cpp` / `FontSpecifiers.h` as
//! reading aids. Text layout and rendering (`ProcessCurrentString`, `PrintString`, `RenderFontBuffer`)
//! stay original in this pass.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
/// `FrontEndMenuManager.m_nPrefsLanguage` (0xBA67CC, sa-core)
use sa_core::platform::PREFS_LANGUAGE;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `s_RenderEndPtr` (the font buffer write pointer)
    pub static RENDER_END_PTR @ 0xC716A8: u32 = 0;
    /// `s_FontData[2]` (0xD2 each: 0xD1 proportional widths, then the unproportional width)
    pub static FONT_DATA @ 0xC718B0: [u8; 2 * 0xD2] = [0; 2 * 0xD2];
    /// `CFont::PS2Symbol`
    pub static PS2_SYMBOL @ 0xC71A54: u8 = 0;
    /// `CFont::m_NewLine`
    pub static NEW_LINE @ 0xC71A55: u8 = 0;
    /// `CFont::m_Details` (`CFontDetails`, 0x40)
    pub static DETAILS @ 0xC71A60: [u8; 0x40] = [0; 0x40];
    /// `CFont::RenderState` (`CFontChar`, 0x30)
    pub static RENDER_STATE @ 0xC71AA0: [u8; 0x30] = [0; 0x30];
}

/// `s_RenderFontBuffer` (the start of the font buffer)
pub const RENDER_FONT_BUFFER: u32 = 0xC716B0;

/// `CFontDetails` field offsets (from the setters' stores).
pub mod det {
    pub const COLOR: isize = 0x00;
    pub const SCALE_X: isize = 0x04;
    pub const SCALE_Y: isize = 0x08;
    pub const SLANT: isize = 0x0C;
    pub const SLANT_REF_X: isize = 0x10;
    pub const SLANT_REF_Y: isize = 0x14;
    pub const JUSTIFY: isize = 0x18;
    pub const CENTRE: isize = 0x19;
    pub const RIGHT_JUSTIFY: isize = 0x1A;
    pub const BACKGROUND: isize = 0x1B;
    pub const BACKGROUND_OUTLINE: isize = 0x1C;
    pub const PROPORTIONAL: isize = 0x1D;
    pub const ALPHA_FADE: isize = 0x20;
    pub const BACKGROUND_COLOR: isize = 0x24;
    pub const WRAP_X: isize = 0x28;
    pub const CENTRE_SIZE: isize = 0x2C;
    pub const RIGHT_JUSTIFY_WRAP: isize = 0x30;
    pub const FONT_TEXTURE: isize = 0x34;
    pub const FONT_STYLE: isize = 0x35;
    pub const DROP_AMOUNT: isize = 0x36;
    pub const DROP_COLOR: isize = 0x37;
    pub const EDGE_AMOUNT: isize = 0x3B;
    pub const EDGE_SPACE: isize = 0x3C;
}

/// `CFontChar` (RenderState) offsets read here.
const RS_PROP_ON: isize = 0x2A;
const RS_FONT_TEXTURE: isize = 0x2C;

fn details() -> GPtr<u8> {
    DETAILS.ptr().cast()
}

fn d_rd<T: sa_core::mem::GameData>(off: isize) -> T {
    rd(details(), off)
}

fn d_wr<T: sa_core::mem::GameData>(off: isize, v: T) {
    wr(details(), off, v)
}

orig! {
    /// `CSprite::InitSpriteBuffer()` (0x70CFB0)
    pub fn init_sprite_buffer() = cdecl 0x70CFB0 => unported;
    /// `CFont::FindSubFontCharacter(uint8 c, uint8 style)` (0x7192C0)
    pub fn find_sub_font_character_call(c: u32, style: u32) -> u8 = cdecl 0x7192C0 => find_sub_font_character_abi;
    /// `CFont::ProcessCurrentString(bool print, float x, float y, const GxtChar* text)` (0x71A220)
    pub fn process_current_string(print: u32, x: f32, y: f32, text: GPtr<u8>) -> u32 = cdecl 0x71A220 => unported;
}

pub fn find_sub_font_character_abi(c: u32, style: u32) -> u8 {
    find_sub_font_character(c as u8, style as u8)
}

/// 255.0 (0x859AAC)
const K_255: f32 = 255.0;
/// 1/255 (0x859A3C)
const K_INV_255: u32 = 0x3B80_8081;

/// `fcomp [255.0]` + `test ah, 5` + `jp` taken: alpha fade >= 255 (or unordered).
fn fade_is_full(fade: f32) -> bool {
    x(fade).fcom(x(K_255)).test_parity(0x05)
}

/// `fild alpha; fmul [m_Details.AlphaFade]; fmul [1/255]; _ftol` (low byte).
fn faded_alpha(alpha: u8, fade: f32) -> u8 {
    ftol32(
        xi(i32::from(alpha))
            .fmul_m(fade)
            .fmul_m(f32::from_bits(K_INV_255)),
    ) as u8
}

/// `CFont::SetScale(float w, float h)` (0x719380): stores both words unchanged.
#[hook(addr = 0x719380, conv = cdecl, name = "CFont::SetScale", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn set_scale(w: u32, h: u32) {
    d_wr(det::SCALE_X, w);
    d_wr(det::SCALE_Y, h);
}

/// `CFont::SetScaleForCurrentLanguage(float w, float h)` (0x7193A0): languages 1..4 (French, German,
/// Italian, Spanish; one jump table target) scale the width by 0.8 (0x858C98); the width always goes
/// through `fld`/`fstp`, the height is copied as a word.
#[hook(addr = 0x7193A0, conv = cdecl, name = "CFont::SetScaleForCurrentLanguage", expect = "0F BE 05 CC 67 BA 00")]
pub fn set_scale_for_current_language(w: f32, h: u32) {
    let lang = i32::from(PREFS_LANGUAGE.get() as i8);
    let v = if (lang.wrapping_sub(1) as u32) <= 3 {
        x(w).fmul_m(f32::from_bits(0x3F4C_CCCD))
    } else {
        x(w)
    };
    d_wr(det::SCALE_X, v.to_f32());
    d_wr(det::SCALE_Y, h);
}

/// `CFont::SetSlantRefPoint(float x, float y)` (0x719400)
#[hook(addr = 0x719400, conv = cdecl, name = "CFont::SetSlantRefPoint", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn set_slant_ref_point(px: u32, py: u32) {
    d_wr(det::SLANT_REF_X, px);
    d_wr(det::SLANT_REF_Y, py);
}

/// `CFont::SetSlant(float)` (0x719420)
#[hook(addr = 0x719420, conv = cdecl, name = "CFont::SetSlant", expect = "8B 44 24 04 A3 6C 1A C7 00")]
pub fn set_slant(v: u32) {
    d_wr(det::SLANT, v);
}

/// `CFont::SetColor(CRGBA)` (0x719430): stores the colour; below full alpha fade the alpha byte becomes
/// `_ftol(a * AlphaFade * (1/255))`.
#[hook(addr = 0x719430, conv = cdecl, name = "CFont::SetColor", expect = "D9 05 80 1A C7 00")]
pub fn set_color(c: u32) {
    let fade: f32 = d_rd(det::ALPHA_FADE);
    d_wr(det::COLOR, c);
    if !fade_is_full(fade) {
        d_wr(det::COLOR + 3, faded_alpha((c >> 24) as u8, fade));
    }
}

/// `CFont::SetFontStyle(eFontStyle)` (0x719490): 2 -> texture 0 / style 2, 3 -> texture 1 / style 1,
/// otherwise texture = the style byte, style 0.
#[hook(addr = 0x719490, conv = cdecl, name = "CFont::SetFontStyle", expect = "8A 4C 24 04 0F B6 C1")]
pub fn set_font_style(style: u32) {
    let s = style as u8;
    match s {
        2 => {
            d_wr::<u8>(det::FONT_TEXTURE, 0);
            d_wr::<u8>(det::FONT_STYLE, 2);
        }
        3 => {
            d_wr::<u8>(det::FONT_TEXTURE, 1);
            d_wr::<u8>(det::FONT_STYLE, 1);
        }
        _ => {
            d_wr::<u8>(det::FONT_TEXTURE, s);
            d_wr::<u8>(det::FONT_STYLE, 0);
        }
    }
}

/// `CFont::SetWrapx(float)` (0x7194D0)
#[hook(addr = 0x7194D0, conv = cdecl, name = "CFont::SetWrapx", expect = "8B 44 24 04 A3 88 1A C7 00")]
pub fn set_wrapx(v: u32) {
    d_wr(det::WRAP_X, v);
}

/// `CFont::SetCentreSize(float)` (0x7194E0)
#[hook(addr = 0x7194E0, conv = cdecl, name = "CFont::SetCentreSize", expect = "8B 44 24 04 A3 8C 1A C7 00")]
pub fn set_centre_size(v: u32) {
    d_wr(det::CENTRE_SIZE, v);
}

/// `CFont::SetRightJustifyWrap(float)` (0x7194F0)
#[hook(addr = 0x7194F0, conv = cdecl, name = "CFont::SetRightJustifyWrap", expect = "8B 44 24 04 A3 90 1A C7 00")]
pub fn set_right_justify_wrap(v: u32) {
    d_wr(det::RIGHT_JUSTIFY_WRAP, v);
}

/// `CFont::SetAlphaFade(float)` (0x719500)
#[hook(addr = 0x719500, conv = cdecl, name = "CFont::SetAlphaFade", expect = "8B 44 24 04 A3 80 1A C7 00")]
pub fn set_alpha_fade(v: u32) {
    d_wr(det::ALPHA_FADE, v);
}

/// `CFont::SetDropColor(CRGBA)` (0x719510): stores the colour; below full alpha fade the drop alpha
/// becomes `_ftol(Color.a * AlphaFade * (1/255))` (the text colour's alpha, as the bytes read 0xC71A63).
#[hook(addr = 0x719510, conv = cdecl, name = "CFont::SetDropColor", expect = "8B 44 24 04 D9 05 80 1A C7 00")]
pub fn set_drop_color(c: u32) {
    let fade: f32 = d_rd(det::ALPHA_FADE);
    d_wr(det::DROP_COLOR, c);
    if !fade_is_full(fade) {
        let a: u8 = d_rd(det::COLOR + 3);
        d_wr(det::DROP_COLOR + 3, faded_alpha(a, fade));
    }
}

/// `CFont::SetDropShadowPosition(int8)` (0x719570): edge off, drop amount.
#[hook(addr = 0x719570, conv = cdecl, name = "CFont::SetDropShadowPosition", expect = "32 C0 A2 9B 1A C7 00")]
pub fn set_drop_shadow_position(v: u32) {
    d_wr::<u8>(det::EDGE_AMOUNT, 0);
    d_wr::<u8>(det::EDGE_SPACE, 0);
    d_wr(det::DROP_AMOUNT, v as u8);
}

/// `CFont::SetEdge(int8)` (0x719590): drop off, edge amount and space.
#[hook(addr = 0x719590, conv = cdecl, name = "CFont::SetEdge", expect = "8A 44 24 04 C6 05 96 1A C7 00 00")]
pub fn set_edge(v: u32) {
    d_wr::<u8>(det::DROP_AMOUNT, 0);
    d_wr(det::EDGE_AMOUNT, v as u8);
    d_wr(det::EDGE_SPACE, v as u8);
}

/// `CFont::SetProportional(bool)` (0x7195B0)
#[hook(addr = 0x7195B0, conv = cdecl, name = "CFont::SetProportional", expect = "8A 44 24 04 A2 7D 1A C7 00")]
pub fn set_proportional(v: u32) {
    d_wr(det::PROPORTIONAL, v as u8);
}

/// `CFont::SetBackground(bool, bool)` (0x7195C0)
#[hook(addr = 0x7195C0, conv = cdecl, name = "CFont::SetBackground", expect = "8A 44 24 04 8A 4C 24 08")]
pub fn set_background(on: u32, outline: u32) {
    d_wr(det::BACKGROUND, on as u8);
    d_wr(det::BACKGROUND_OUTLINE, outline as u8);
}

/// `CFont::SetBackgroundColor(CRGBA)` (0x7195E0)
#[hook(addr = 0x7195E0, conv = cdecl, name = "CFont::SetBackgroundColor", expect = "8B 44 24 04 A2 84 1A C7 00")]
pub fn set_background_color(c: u32) {
    d_wr(det::BACKGROUND_COLOR, c);
}

/// `CFont::SetJustify(bool)` (0x719600). Not hooked: `CHud::DrawHelpText` pushes ECX at 0x58BAC5 after
/// `SetOrientation` / `SetJustify` (a stack-slot reservation that sa-hook's callers test cannot prove
/// through the following x87 branches), so a hook would need `preserve = "ecx"`, and a preserving hook
/// needs an L1 spec comparing ECX, which cannot run a function that writes globals.
pub fn set_justify(v: u32) {
    d_wr(det::JUSTIFY, v as u8);
}

/// `CFont::SetOrientation(eFontAlignment)` (0x719610): 0 centre, 1 left, 2 right; other values store
/// nothing. Not hooked, as `SetJustify`; besides, the original leaves ECX = 0, which `preserve` cannot model.
pub fn set_orientation(v: u32) {
    match v as u8 {
        0 => {
            d_wr::<u8>(det::CENTRE, 1);
            d_wr::<u8>(det::RIGHT_JUSTIFY, 0);
        }
        1 => {
            d_wr::<u8>(det::CENTRE, 0);
            d_wr::<u8>(det::RIGHT_JUSTIFY, 0);
        }
        2 => {
            d_wr::<u8>(det::CENTRE, 0);
            d_wr::<u8>(det::RIGHT_JUSTIFY, 1);
        }
        _ => {}
    }
}

/// `CFont::InitPerFrame()` (0x719800): edge/drop off, new line and PS2 symbol cleared, render-state
/// texture -1, the buffer pointer back to the start, then `CSprite::InitSpriteBuffer` (tail jump).
#[hook(addr = 0x719800, conv = cdecl, name = "CFont::InitPerFrame", expect = "32 C0 A2 9B 1A C7 00")]
pub fn init_per_frame() {
    d_wr::<u8>(det::EDGE_AMOUNT, 0);
    d_wr::<u8>(det::EDGE_SPACE, 0);
    d_wr::<u8>(det::DROP_AMOUNT, 0);
    NEW_LINE.set(0);
    PS2_SYMBOL.set(0);
    wr::<u16>(RENDER_STATE.ptr().cast(), RS_FONT_TEXTURE, 0xFFFF);
    RENDER_END_PTR.set(RENDER_FONT_BUFFER);
    init_sprite_buffer();
}

/// `s_FontData[font]`: proportional width of `c`, or the unproportional width (+0xD1).
fn font_width(font: i32, c: Option<u8>) -> u8 {
    let base = FONT_DATA
        .ptr()
        .cast::<u8>()
        .byte_add(font.wrapping_mul(0xD2) as isize);
    match c {
        Some(c) => rd(base, isize::from(c)),
        None => rd(base, 0xD1),
    }
}

/// `CFont::GetLetterIdPropValue(uint8)` (0x718770): '?' (0x3F) maps to 0; the width of the render
/// state's font (proportional when its prop flag is 1), as `fild` (ST0).
#[hook(addr = 0x718770, conv = cdecl, name = "CFont::GetLetterIdPropValue-", expect = "8A 44 24 04 3C 3F")]
pub fn get_letter_id_prop_value(c: u32) -> X87 {
    let mut ch = c as u8;
    if ch == 0x3F {
        ch = 0;
    }
    let rs: GPtr<u8> = RENDER_STATE.ptr().cast();
    let font = i32::from(rd::<i16>(rs, RS_FONT_TEXTURE));
    let prop = rd::<u8>(rs, RS_PROP_ON) == 1;
    xi(i32::from(font_width(font, prop.then_some(ch))))
}

/// `CFont::FindSubFontCharacter(uint8 c, uint8 style)` (0x7192C0): the glyph of `c` in the sub-font
/// (style 1 adds the symbol remaps first). Only AL is set (the rest of EAX is the caller's).
#[hook(addr = 0x7192C0, conv = cdecl, name = "CFont::FindSubFontCharacter", expect = "80 7C 24 08 01", preserve = "ecx,edx")]
pub fn find_sub_font_character(c: u8, style: u8) -> u8 {
    if style == 1 {
        match c {
            0x1A => return 0x9A,
            0x08..=0x09 => return c.wrapping_add(0x56),
            0x04 => return 0x5D,
            0x07 => return 0xCE,
            0x0E => return 0xCF,
            0x01 => return 0xD0,
            _ => {}
        }
    }
    match c {
        0x8F => 0xCD,
        0x1F => 0x5B,
        0x06 => 0x0A,
        0x3E => 0x20,
        0x10..=0x19 => c.wrapping_add(0x80),
        0x21..=0x3A => c.wrapping_add(0x7A),
        0x41..=0x5A => c.wrapping_add(0x5A),
        0x60..=0x76 => c.wrapping_add(0x55),
        0x77..=0x8C => c.wrapping_add(0x3E),
        0x8D..=0x8E => 0xCC,
        _ => c,
    }
}

/// `CFont::GetCharacterSize(uint8)` (0x719750): '?' maps to 0; with a font style the sub-font glyph,
/// otherwise 0x91 -> 0x40 and anything above 0x9B -> 0; then `(width + (int8)EdgeAmount) * Scale.x`
/// (`fild`, `fiadd`, `fmul`; ST0).
#[hook(addr = 0x719750, conv = cdecl, name = "CFont::GetCharacterSize", expect = "8A 44 24 04 3C 3F")]
pub fn get_character_size(c: u32) -> X87 {
    let mut w = c;
    if w as u8 == 0x3F {
        w &= !0xFF;
    }
    let style: u8 = d_rd(det::FONT_STYLE);
    let ch = if style != 0 {
        find_sub_font_character_call(w, u32::from(style))
    } else {
        match w as u8 {
            0x91 => 0x40,
            v if v > 0x9B => 0,
            v => v,
        }
    };
    let font = i32::from(d_rd::<u8>(det::FONT_TEXTURE));
    let prop = d_rd::<u8>(det::PROPORTIONAL) != 0;
    let edge = i32::from(d_rd::<i8>(det::EDGE_AMOUNT));
    xi(i32::from(font_width(font, prop.then_some(ch))))
        .fadd(xi(edge))
        .fmul_m(d_rd(det::SCALE_X))
}

/// `CFont::GetNumberLines(float x, float y, const GxtChar*)` (0x71A5E0):
/// `ProcessCurrentString(false, x, y, text)`.
#[hook(addr = 0x71A5E0, conv = cdecl, name = "CFont::GetNumberLines", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn get_number_lines(px: f32, py: f32, text: GPtr<u8>) -> u32 {
    process_current_string(0, px, py, text)
}

/// `CFontChar::Set(const CFontChar&)` (0x718E50, thiscall): copies the fields (0x00..0x2A, 0x2C..0x2E;
/// the padding bytes 0x2B and 0x2F are left), returns `this` in EAX.
#[hook(addr = 0x718E50, conv = thiscall, name = "CFontChar::Set", expect = "8B C1 8B 4C 24 04", this = "CFontChar")]
pub fn font_char_set(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    let mut o = 0;
    while o < 0x14 {
        wr(this, o, rd::<u32>(src, o));
        o += 4;
    }
    for o in 0x14..0x18 {
        wr(this, o, rd::<u8>(src, o));
    }
    let mut o = 0x18;
    while o < 0x28 {
        wr(this, o, rd::<u32>(src, o));
        o += 4;
    }
    for o in 0x28..0x2B {
        wr(this, o, rd::<u8>(src, o));
    }
    wr(this, 0x2C, rd::<u16>(src, 0x2C));
    wr(this, 0x2E, rd::<u8>(src, 0x2E));
    this
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_SET_SCALE,
    HOOK_SET_SCALE_FOR_CURRENT_LANGUAGE,
    HOOK_SET_SLANT_REF_POINT,
    HOOK_SET_SLANT,
    HOOK_SET_COLOR,
    HOOK_SET_FONT_STYLE,
    HOOK_SET_WRAPX,
    HOOK_SET_CENTRE_SIZE,
    HOOK_SET_RIGHT_JUSTIFY_WRAP,
    HOOK_SET_ALPHA_FADE,
    HOOK_SET_DROP_COLOR,
    HOOK_SET_DROP_SHADOW_POSITION,
    HOOK_SET_EDGE,
    HOOK_SET_PROPORTIONAL,
    HOOK_SET_BACKGROUND,
    HOOK_SET_BACKGROUND_COLOR,
    HOOK_INIT_PER_FRAME,
    HOOK_GET_LETTER_ID_PROP_VALUE,
    HOOK_FIND_SUB_FONT_CHARACTER,
    HOOK_GET_CHARACTER_SIZE,
    HOOK_GET_NUMBER_LINES,
    HOOK_FONT_CHAR_SET,
];
