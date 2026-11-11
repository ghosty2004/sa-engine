//! `CFont` text layout and printing (PORT-HUD-2): string walking, token parsing, line breaking and the
//! x87 width / position arithmetic, plus the `GxtChar` string helpers. Ported from the 1.0 US bytes;
//! gta-reversed `Font.cpp` / `GxtChar.cpp` and `docs/re/specs/HUD/CFont.deviations.md` as reading aids.
//! Rendering stays original: `CFont::PrintString` (ranged), `RenderFontBuffer`, `CSprite2d` and the
//! menu window are `orig!` calls.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, sprintf1, wr, x, xi};
use crate::font::{det, DETAILS, NEW_LINE, PS2_SYMBOL, RENDER_STATE};

/// This module's globals (none of its own: `CFont::m_Details`, `PS2Symbol`, `m_NewLine` and the render
/// state are `crate::font`'s; the buffers below are reached by address).
pub const GLOBALS: &[GlobalDesc] = &[];

/// `gString` (0xB71670, sa-stream's `G_STRING`): `ProcessCurrentString` prints the pending token into it.
const G_STRING: usize = 0xB71670;
/// "~%c~" (0x872930)
const FMT_TOKEN: usize = 0x872930;
/// `HudColour` (0xBAB22C)
const HUD_COLOUR: usize = 0xBAB22C;
/// `FrontEndMenuManager` (0xBA6748)
const FRONT_END_MENU_MANAGER: usize = 0xBA6748;
/// `CTheScripts::IntroTextLines` (0xA913E8, `tScriptText`, 0x44 each)
const INTRO_TEXT_LINES: usize = 0xA913E8;
/// `CTheScripts::NumberOfIntroTextLinesThisFrame` (0xA44B68, u16)
const NUM_INTRO_TEXT_LINES: usize = 0xA44B68;
/// `s_FontData` (0xC718B0, `crate::font::FONT_DATA`), indexed here with any texture byte as the original.
const FONT_DATA: usize = 0xC718B0;

/// 0.5 (0x858B8C)
const K_HALF: f32 = f32::from_bits(0x3F00_0000);
/// 4.0 (0x858B90)
const K_FOUR: f32 = f32::from_bits(0x4080_0000);
/// 32.0 (0x85950C)
const K_32: f32 = f32::from_bits(0x4200_0000);
/// 1.0 (0x858624)
const K_ONE: f32 = f32::from_bits(0x3F80_0000);
/// 0.0 (0x858B50)
const K_ZERO: f32 = f32::from_bits(0);
/// 1.5 (0x858CE8)
const K_1_5: f32 = f32::from_bits(0x3FC0_0000);
/// 255.0 (immediate 0x437F0000)
const K_255_BITS: u32 = 0x437F_0000;
/// 1000000.0 (immediate 0x49742400)
const K_1E6_BITS: u32 = 0x4974_2400;
/// -1000000.0 (immediate 0xC9742400)
const K_M1E6_BITS: u32 = 0xC974_2400;

orig! {
    /// `CMessages::GetStringLength(const GxtChar*)` (0x69DB50)
    pub fn get_string_length_call(s: GPtr<u8>) -> u32 = cdecl 0x69DB50 => crate::messages::get_string_length;
    /// `CMessages::InsertPlayerControlKeysInString(GxtChar*)` (0x69E160)
    pub fn insert_player_control_keys_call(s: GPtr<u8>) = cdecl 0x69E160 => crate::messages::insert_player_control_keys_in_string;
    /// `CFont::GetCharacterSize(uint8)` (0x719750)
    pub fn get_character_size_call_st0(c: u32) -> f64 = cdecl 0x719750 => get_character_size_call_st0_rust;
    /// `CFont::FindSubFontCharacter(uint8 c, uint8 style)` (0x7192C0)
    pub fn find_sub_font_character_call(c: u32, style: u32) -> u8 = cdecl 0x7192C0 => crate::font::find_sub_font_character_abi;
    /// `CFont::SetColor(CRGBA)` (0x719430)
    pub fn set_color_call(c: u32) = cdecl 0x719430 => crate::font::set_color;
    /// `CFont::GetScriptLetterSize(uint8)` (0x719670)
    pub fn get_script_letter_size_call_st0(c: u32) -> f64 = cdecl 0x719670 => get_script_letter_size_call_st0_rust;
    /// `CFont::GetStringWidth(const GxtChar*, bool full, bool scriptText)` (0x71A0E0)
    pub fn get_string_width_call_st0(s: GPtr<u8>, full: u32, script: u32) -> f64 = cdecl 0x71A0E0 => get_string_width_call_st0_rust;
    /// `CFont::ParseToken(const GxtChar*, CRGBA&, bool isBlip, char* out)` (0x718F00)
    pub fn parse_token_call(s: GPtr<u8>, color: GPtr<u8>, is_blip: u32, out: GPtr<u8>) -> GPtr<u8> = cdecl 0x718F00 => parse_token;
    /// `CFont::PrintString(float x, float y, const GxtChar* start, const GxtChar* end, float spacing)`
    /// (0x719B40)
    pub fn print_string_ranged_call(px: f32, py: f32, start: GPtr<u8>, end: GPtr<u8>, spacing: f32) = cdecl 0x719B40 => print_string_ranged;
    /// `CFont::ProcessCurrentString(bool print, float x, float y, const GxtChar*)` (0x71A220)
    pub fn process_current_string_call(print: u32, px: f32, py: f32, text: GPtr<u8>) -> u16 = cdecl 0x71A220 => process_current_string_ax;
    /// `CFont::GetTextRect(CRect*, float x, float y, const GxtChar*)` (0x71A620)
    pub fn get_text_rect_call(rect: GPtr<f32>, px: f32, py: f32, text: GPtr<u8>) = cdecl 0x71A620 => get_text_rect;
    /// `CFont::PrintString(float x, float y, const GxtChar*)` (0x71A700)
    pub fn print_string_call(px: f32, py: f32, text: GPtr<u8>) = cdecl 0x71A700 => print_string;
    /// `CFont::RenderFontBuffer()` (0x719840)
    pub fn render_font_buffer_call() = cdecl 0x719840 => render_font_buffer;
    /// `CMenuManager::DrawWindow(const CRect&, const char* key, uint8 colour, CRGBA back, bool unused,
    /// bool background)` (0x573EE0, thiscall)
    pub fn draw_window(this: GPtr<u8>, rect: GPtr<f32>, key: u32, colour: u32, back: u32, unused: u32, bg: u32) = thiscall 0x573EE0 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    pub fn sprite_draw_rect(rect: GPtr<f32>, colour: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `CHudColours::GetRGBA(CRGBA* out, uint8 colour, uint8 alpha)` (0x58FEE0, thiscall)
    pub fn hud_get_rgba(this: GPtr<u8>, out: GPtr<u8>, colour: u32, alpha: u32) -> GPtr<u8> = thiscall 0x58FEE0 => crate::hud::get_rgba;
    /// `CRGBA::operator=(const CRGBA&)` (stub 0x463370, thiscall)
    pub fn rgba_assign(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x463370 => unported;
    /// `CRGBA::CRGBA(uint8 r, uint8 g, uint8 b, uint8 a)` (0x7170C0, thiscall)
    pub fn rgba_ctor(this: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `std::min<float>(a, b)` instance (0x404330): ST0 = `a` when `a < b`, else `b`.
    pub fn min_f_st0(a: f32, b: f32) -> f64 = cdecl 0x404330 => unported;
}

fn det_ptr() -> GPtr<u8> {
    DETAILS.ptr().cast()
}

fn d8(off: isize) -> u8 {
    rd(det_ptr(), off)
}

fn df(off: isize) -> f32 {
    rd(det_ptr(), off)
}

fn b(p: GPtr<u8>, i: isize) -> u8 {
    rd(p, i)
}

/// `ScaleY * 32 * 0.5 + (ScaleY + ScaleY)` (the line height, `fld; fmul; fmul; fld; fadd st, st;
/// faddp`).
fn line_height() -> X87 {
    let s = df(det::SCALE_Y);
    let a = x(s).fmul_m(K_32).fmul_m(K_HALF);
    let b = x(s).fadd(x(s));
    a.fadd(b)
}

// ---------------------------------------------------------------------------------------------------
// GxtChar helpers
// ---------------------------------------------------------------------------------------------------

/// `GxtCharStrcat(GxtChar* dst, const GxtChar* src)` (0x718630): appends `src` at `dst`'s terminator;
/// returns `dst`.
#[hook(addr = 0x718630, conv = cdecl, name = "GxtCharStrcat", expect = "8B 44 24 04 80 38 00")]
pub fn gxt_char_strcat(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    let mut d = dst;
    while b(d, 0) != 0 {
        d = d.byte_add(1);
    }
    let mut s = src;
    loop {
        let c = b(s, 0);
        if c == 0 {
            break;
        }
        wr(d, 0, c);
        d = d.byte_add(1);
        s = s.byte_add(1);
    }
    wr::<u8>(d, 0, 0);
    dst
}

/// `GxtCharStrcpy(GxtChar* dst, const GxtChar* src)` (0x718660): returns `dst`.
#[hook(addr = 0x718660, conv = cdecl, name = "GxtCharStrcpy", expect = "8B 44 24 04 56 8B 74 24 0C")]
pub fn gxt_char_strcpy(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    let mut i = 0isize;
    loop {
        let c = b(src, i);
        if c == 0 {
            break;
        }
        wr(dst, i, c);
        i += 1;
    }
    wr::<u8>(dst, i, 0);
    dst
}

/// `GxtCharStrlen(const GxtChar*)` (0x718690, catalogue `FUN_00718690`): the length (EAX).
#[hook(addr = 0x718690, conv = cdecl, name = "FUN_00718690", expect = "8B 4C 24 04 8A 11 33 C0")]
pub fn gxt_char_strlen(s: GPtr<u8>) -> u32 {
    let mut n = 0u32;
    while b(s, n as isize) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// `CFont::GetNextSpace(const GxtChar*)` (0x718ED0): the first ' ', NUL or '~' (a leading ' ' is
/// returned as is; '~' is only tested after the first character).
#[hook(addr = 0x718ED0, conv = cdecl, name = "CFont::GetNextSpace", expect = "8B 44 24 04 8A 08 80 F9 20")]
pub fn get_next_space(s: GPtr<u8>) -> GPtr<u8> {
    next_space(s)
}

fn next_space(mut s: GPtr<u8>) -> GPtr<u8> {
    let mut c = b(s, 0);
    if c == b' ' {
        return s;
    }
    loop {
        if c == 0 || c == b'~' {
            return s;
        }
        c = b(s, 1);
        s = s.byte_add(1);
        if c == b' ' {
            return s;
        }
    }
}

/// Copy buffer of [`strip_tokens`]: the original's is 0x100 bytes of its stack with an unbounded copy;
/// the port's is larger and zero-filled (an unterminated token in the original scans its stack).
const STRIP_BUF: usize = 0x800;

/// `FUN_00719240(GxtChar* s)` (0x719240, a `GxtChar` helper of `CStats::BuildStatLine` /
/// `ConstructStatLine`): copies `s` to a stack buffer, then writes back every character outside
/// `~...~` tokens (the closing '~' is searched without a NUL test) and terminates.
#[hook(addr = 0x719240, conv = cdecl, name = "FUN_00719240", expect = "81 EC 00 01 00 00")]
pub fn strip_tokens(s: GPtr<u8>) {
    let mut buf = [0u8; STRIP_BUF];
    let mut i = 0usize;
    loop {
        let c = b(s, i as isize);
        if c == 0 || i + 1 >= STRIP_BUF {
            break;
        }
        buf[i] = c;
        i += 1;
    }
    buf[i] = 0;
    let mut out = 0isize;
    let mut p = 0usize;
    let mut c = buf[0];
    while c != 0 {
        if c == b'~' {
            loop {
                p += 1;
                if p >= STRIP_BUF - 1 || buf[p] == b'~' {
                    break;
                }
            }
        } else {
            wr(s, out, c);
            out += 1;
        }
        p += 1;
        if p >= STRIP_BUF {
            break;
        }
        c = buf[p];
    }
    wr::<u8>(s, out, 0);
}

// ---------------------------------------------------------------------------------------------------
// Tokens and widths
// ---------------------------------------------------------------------------------------------------

/// The 62-entry index table of `ParseToken` (0x7191F4, characters '<'..'y'), 24 = no action.
const TOKEN_INDEX: [u8; 62] = [
    0, 24, 1, 24, 24, 2, 3, 4, 5, 24, 24, 6, 7, 24, 8, 9, 24, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    21, 22, 24, 24, 24, 24, 24, 24, 24, 2, 3, 4, 5, 24, 24, 6, 7, 24, 8, 9, 23, 10, 11, 12, 13, 14, 15,
    16, 17, 18, 19, 20, 21, 22,
];

/// What one jump-table target (0x719190) does.
enum TokenAction {
    /// `PS2Symbol = n`
    Symbol(u8),
    /// `CHudColours::GetRGBA(colour, color.a)` into the colour
    Colour(u32),
    /// 'H': each of r, g, b becomes `_ftol(min((float)(c * 1.5), 255.0))`
    Lighten,
    /// 'N': `m_NewLine = 1`
    NewLine,
    None,
}

fn token_action(slot: u8) -> TokenAction {
    use TokenAction::*;
    match slot {
        0 => Symbol(3),
        1 => Symbol(4),
        2 => Symbol(0xB),
        3 => Colour(2),
        4 => Symbol(0xE),
        5 => Symbol(2),
        6 => Colour(1),
        7 => Lighten,
        8 => Symbol(0xC),
        9 => Symbol(9),
        10 => Symbol(0xA),
        11 => NewLine,
        12 => Symbol(6),
        13 => Colour(7),
        14 => Symbol(7),
        15 => Colour(0),
        16 => Colour(4),
        17 => Symbol(8),
        18 => Symbol(1),
        19 => Symbol(0xD),
        20 => Colour(4),
        21 => Symbol(5),
        22 => Colour(0xB),
        23 => Colour(5),
        _ => None,
    }
}

/// `CFont::ParseToken(const GxtChar* text, CRGBA& color, bool isBlip, char* out)` (0x718F00): `text`
/// points at the opening '~'. The token character `text[1]` - '<' indexes the byte table 0x7191F4
/// (part of the dispatch sits in the `.text` chunk 0x401D91): button symbols set `PS2Symbol`, 'N'/'n'
/// sets `m_NewLine`, colour tokens (unless `isBlip`) set `color` and then (also with `isBlip`) store
/// the token character into `*out` when `out` is set. Returns one past the closing '~' (searched from
/// `text + 1` without a NUL test).
#[hook(addr = 0x718F00, conv = cdecl, name = "CFont::ParseToken", expect = "83 EC 20 57 8B 7C 24 28")]
pub fn parse_token(text: GPtr<u8>, color: GPtr<u8>, is_blip: u32, out: GPtr<u8>) -> GPtr<u8> {
    let mut p = text.byte_add(1);
    let idx = u32::from(b(p, 0)).wrapping_sub(0x3C);
    if idx <= 0x3D {
        match token_action(TOKEN_INDEX[idx as usize]) {
            TokenAction::Symbol(n) => PS2_SYMBOL.set(n),
            TokenAction::NewLine => NEW_LINE.set(1),
            TokenAction::None => {}
            TokenAction::Colour(c) => {
                if is_blip as u8 == 0 {
                    let mut tmp = [0u8; 4];
                    let r = hud_get_rgba(
                        g(HUD_COLOUR),
                        GPtr::from_raw(tmp.as_mut_ptr()),
                        c,
                        u32::from(b(color, 3)),
                    );
                    rgba_assign(color, r);
                }
                token_out(p, out);
            }
            TokenAction::Lighten => {
                if is_blip as u8 == 0 {
                    let lighten = |v: u8| -> u32 {
                        let a = xi(i32::from(v)).fmul_m(K_1_5).to_f32();
                        crate::common::ftol32(min_f(a, f32::from_bits(K_255_BITS))) as u32
                    };
                    let r = lighten(b(color, 0));
                    let gr = lighten(b(color, 1));
                    let bl = lighten(b(color, 2));
                    let mut tmp = [0u8; 4];
                    let c = rgba_ctor(GPtr::from_raw(tmp.as_mut_ptr()), r, gr, bl, u32::from(b(color, 3)));
                    rgba_assign(color, c);
                }
                token_out(p, out);
            }
        }
    }
    while b(p, 0) != b'~' {
        p = p.byte_add(1);
    }
    if b(p, 0) == 0 {
        p.byte_add(2)
    } else {
        p.byte_add(1)
    }
}

fn token_out(p: GPtr<u8>, out: GPtr<u8>) {
    if !out.is_null() {
        wr(out, 0, b(p, 0));
    }
}

/// `CFont::GetScriptLetterSize(uint8)` (0x719670): `GetCharacterSize` with the current intro text
/// line's (`IntroTextLines[NumberOfIntroTextLinesThisFrame]`) scale x (+0), proportional flag (+0x1C),
/// edge (+0x22, int8) and font (+0x28): fonts 2 / 3 use texture 0 / 1 and the sub-font glyph (style 2
/// / 1), other fonts use the font's low byte as texture with 0x91 -> 0x40 and > 0x9B -> 0. ST0 =
/// `ScaleX * (width + edge)`.
#[hook(addr = 0x719670, conv = cdecl, name = "CFont::GetScriptLetterSize", expect = "8A 54 24 04 83 EC 08")]
pub fn get_script_letter_size(c: u32) -> X87 {
    let mut arg = c;
    if arg as u8 == 0x3F {
        arg &= !0xFF;
    }
    let ch = arg as u8;
    let n = u32::from(g::<u16>(NUM_INTRO_TEXT_LINES).read());
    let line: GPtr<u8> = g(INTRO_TEXT_LINES + (n * 0x44) as usize);
    let prop: u8 = rd(line, 0x1C);
    let font: u32 = rd(line, 0x28);
    let scale: f32 = rd(line, 0);
    let edge: i8 = rd(line, 0x22);
    let (tex, glyph) = match font {
        2 => (0u8, find_sub_font_character_call(arg, 2)),
        3 => (1u8, find_sub_font_character_call(arg, 1)),
        _ => {
            let gl = match ch {
                0x91 => 0x40,
                v if v > 0x9B => 0,
                v => v,
            };
            (font as u8, gl)
        }
    };
    let base = FONT_DATA + usize::from(tex) * 0xD2;
    let w = if prop != 0 {
        g::<u8>(base + usize::from(glyph)).read()
    } else {
        g::<u8>(base + 0xD1).read()
    };
    x(scale).fmul(xi(i32::from(w)).fadd(xi(i32::from(edge))))
}

/// The copy buffer of `GetStringWidth`: 0x190 bytes on the original's stack (399 characters are
/// copied, `InsertPlayerControlKeysInString` may grow them past it); the port's is larger.
const WIDTH_BUF: usize = 0x800;

/// `CFont::GetStringWidth(const GxtChar*, bool full, bool scriptText)` (entry 0x71A0E0 = `nop; jmp`
/// into the `.text` chunk 0x4016B9, which allocates the frame and jumps back to 0x71A0E6): copies at
/// most 399 characters (`GetStringLength`), inserts the control keys, then sums the character sizes
/// (`GetCharacterSize`, or `GetScriptLetterSize` for script text, of `c - 0x20`) up to the end, or
/// without `full` up to the first space or the second token / a token after a character. ST0.
///
/// Protection code reads the patched entry bytes as data (`mov ecx, [0x71A0E2]` at 0x40641C): sa-hook's
/// keycells `READERS` redirects that read to a cell holding the original dword (PORT-HUD-3), so the
/// entry is hookable.
#[hook(addr = 0x71A0E0, conv = cdecl, name = "CFont::GetStringWidth", expect = "90 E9 D3 75 CE FF")]
pub fn get_string_width(text: GPtr<u8>, full: u32, script: u32) -> X87 {
    let full = full as u8 != 0;
    let script = script as u8 != 0;
    let mut buf = [0u8; WIDTH_BUF];
    let len = get_string_length_call(text) as u16 as i32;
    let n = if len > 0x18F { 0x18F } else { len };
    let mut i = 0;
    while i < n {
        buf[i as usize] = b(text, i as isize);
        i += 1;
    }
    buf[i as usize] = 0;
    let s = GPtr::from_raw(buf.as_mut_ptr());
    insert_player_control_keys_call(s);
    let mut width = 0f32;
    let mut tag = false;
    let mut any = false;
    let mut p = s;
    loop {
        let c = b(p, 0);
        if (c == b' ' && !full) || c == 0 {
            break;
        }
        if c == b'~' {
            if !full && (tag || any) {
                break;
            }
            loop {
                p = p.byte_add(1);
                if b(p, 0) == b'~' {
                    break;
                }
            }
            p = p.byte_add(1);
            if any || b(p, 0) == b'~' {
                tag = true;
            }
            continue;
        }
        if !full && c == b' ' && tag {
            break;
        }
        let arg = u32::from(c.wrapping_sub(0x20));
        let w = if script {
            get_script_letter_size_call(arg)
        } else {
            get_character_size_call(arg)
        };
        width = w.fadd_m(width).to_f32();
        p = p.byte_add(1);
        any = true;
    }
    x(width)
}

// ---------------------------------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------------------------------

/// The token-restart buffer of `ProcessCurrentString` (0x100 bytes on the original's stack, filled by
/// unbounded copies); the port's is larger.
const LINE_BUF: usize = 0x800;

/// `CFont::ProcessCurrentString(bool print, float x, float y, const GxtChar* text)` (0x71A220): breaks
/// the text into lines at word boundaries against the wrap limit (centre size, `x - RightJustifyWrap`
/// or wrap x), positions each line (centre, right: `x - (width - GetCharacterSize(0))`, justify
/// spacing), prints it with the ranged `PrintString` when `print`, and returns the line count (AX).
/// A colour token at a line start (`ParseToken` with `isBlip`) is carried to the next line by printing
/// "~%c~" into `gString` and restarting from a stack copy. With `print` the saved text colour is set
/// back at the end.
///
/// Return kind AX (`sa_core::abi::Ax`, PORT-HUD-3): the original ends with `mov ax, [esp+0xC]`, so
/// EAX[31:16] is whatever its last full-EAX write left: the EAX residue of `RenderFontBuffer` (print),
/// `PrintString` (ranged), `GetStringWidth`, `ParseToken`, `sprintf`, a stack address (`lea eax`) or a
/// float's bits, depending on the path. That is not derivable from the inputs (it depends on callees'
/// residues, some of them Rust ports); the port returns 0 there (divergence ledger, hud-port.md
/// PORT-HUD-3). Every caller reads AX only: `GetNumberLines` / `ProcessStringToDisplay` pass EAX on and
/// their callers (0x5741FF, 0x57B24B, 0x58B7EA `movsx r32, ax`; 0x58C795 `cmp ax, 1`), `GetTextRect`
/// (0x71A6B4 `movzx eax, al`) and the `PrintString`s (discard it).
#[hook(addr = 0x71A220, conv = cdecl, name = "CFont::ProcessCurrentString", expect = "81 EC 2C 01 00 00")]
pub fn process_current_string(print: u32, px: f32, py: f32, text: GPtr<u8>) -> sa_core::abi::Ax {
    let print = print as u8 != 0;
    let mut buf = [0u8; LINE_BUF];
    let bufp = GPtr::from_raw(buf.as_mut_ptr());
    let mut color_tmp = [0u8; 4];
    let color_tmp = GPtr::from_raw(color_tmp.as_mut_ptr());
    let mut token = [0u8; 1];
    let token_p: GPtr<u8> = GPtr::from_raw(token.as_mut_ptr());

    let saved_color: u32 = rd(det_ptr(), det::COLOR);
    let mut lines: u32 = 0;
    let mut spaces: u32 = 0;
    let mut line_w: f32 = 0.0;
    let mut first = true;
    let mut width: f32 = if d8(det::CENTRE) == 0 && d8(det::RIGHT_JUSTIFY) == 0 {
        px
    } else {
        0.0
    };
    let mut y_line = py;
    let mut start = text;
    let mut cur = text;

    if b(cur, 0) != 0 {
        loop {
            PS2_SYMBOL.set(0);
            let str_w = get_string_width_call(cur, 0, 0).to_f32();
            if b(cur, 0) == b'~' {
                cur = parse_token_call(cur, color_tmp, 1, token_p);
            }
            let mut centre = d8(det::CENTRE) != 0;
            let limit = if centre {
                x(df(det::CENTRE_SIZE))
            } else if d8(det::RIGHT_JUSTIFY) != 0 {
                x(px).fsub_m(df(det::RIGHT_JUSTIFY_WRAP))
            } else {
                x(df(det::WRAP_X))
            };
            let sum = x(str_w).fadd_m(width);
            let sum_f = sum.to_f32();
            let fits = sum.fcom(limit).test_any(0x41);
            let nl = NEW_LINE.get() != 0;
            if (fits || first) && !nl {
                // 0x71A4E3: the word fits, add it to the line.
                width = sum_f;
                cur = next_space(cur);
                let c = b(cur, 0);
                if c == 0 {
                    let x_line = if centre {
                        x(width).fmul_m(K_HALF).fsubr_m(px).to_f32()
                    } else if d8(det::RIGHT_JUSTIFY) == 0 {
                        px
                    } else {
                        x(px).fsub_m(width).to_f32()
                    };
                    lines = lines.wrapping_add(1);
                    if print {
                        print_string_ranged_call(x_line, y_line, start, cur, K_ZERO);
                    }
                } else {
                    if !first {
                        spaces = spaces.wrapping_add(1);
                    }
                    if c != b'~' {
                        width = get_character_size_call(0).fadd_m(width).to_f32();
                        cur = cur.byte_add(1);
                    }
                    line_w = width;
                    first = false;
                }
            } else {
                // 0x71A326: flush the line before this word.
                let mut spacing = 0f32;
                if PS2_SYMBOL.get() != 0 {
                    cur = cur.byte_add(-3);
                }
                let mut end = if nl { cur.byte_add(-3) } else { cur };
                if d8(det::JUSTIFY) != 0 && !centre {
                    spacing = x(df(det::WRAP_X))
                        .fsub_m(line_w)
                        .fdiv(xi(i32::from(spaces as u16 as i16)))
                        .to_f32();
                }
                let x_line;
                if centre {
                    x_line = x(width).fmul_m(K_HALF).fsubr_m(px).to_f32();
                } else if d8(det::RIGHT_JUSTIFY) == 0 {
                    x_line = px;
                } else {
                    x_line = get_character_size_call(0)
                        .fsubr_m(width)
                        .fsubr_m(px)
                        .to_f32();
                    centre = d8(det::CENTRE) != 0;
                }
                lines = lines.wrapping_add(1);
                if print {
                    print_string_ranged_call(x_line, y_line, start, end, spacing);
                    centre = d8(det::CENTRE) != 0;
                }
                let tc = b(token_p, 0);
                if tc != 0 {
                    sprintf1(g(G_STRING), g(FMT_TOKEN), i32::from(tc));
                    // strcpy(buf, gString)
                    let gs: GPtr<u8> = g(G_STRING);
                    let mut i = 0isize;
                    loop {
                        let c = b(gs, i);
                        if c == 0 {
                            break;
                        }
                        wr(bufp, i, c);
                        i += 1;
                    }
                    wr::<u8>(bufp, i, 0);
                    if NEW_LINE.get() != 0 {
                        end = end.byte_add(3);
                    }
                    // strcat(buf, end)
                    let mut d = 0isize;
                    while b(bufp, d) != 0 {
                        d += 1;
                    }
                    let mut s = end;
                    loop {
                        let c = b(s, 0);
                        if c == 0 {
                            break;
                        }
                        wr(bufp, d, c);
                        d += 1;
                        s = s.byte_add(1);
                    }
                    centre = d8(det::CENTRE) != 0;
                    wr::<u8>(bufp, d, 0);
                    cur = bufp;
                    wr::<u8>(token_p, 0, 0);
                }
                let h = line_height();
                NEW_LINE.set(0);
                y_line = h.fadd_m(y_line).to_f32();
                width = if !centre && d8(det::RIGHT_JUSTIFY) == 0 {
                    px
                } else {
                    0.0
                };
                start = cur;
                spaces = 0;
                line_w = 0.0;
                first = true;
            }
            if PS2_SYMBOL.get() != 0 {
                PS2_SYMBOL.set(0);
            }
            if b(cur, 0) == 0 {
                break;
            }
        }
    }
    if print {
        set_color_call(saved_color);
    }
    sa_core::abi::Ax(lines as u16)
}

/// [`process_current_string`]'s AX (the Rust path of `process_current_string_call`).
#[allow(dead_code)]
fn process_current_string_ax(print: u32, px: f32, py: f32, text: GPtr<u8>) -> u16 {
    process_current_string(print, px, py, text).0
}

/// `CFont::ProcessStringToDisplay(float x, float y, const GxtChar*)` (0x71A600):
/// `ProcessCurrentString(true, x, y, text)` (returns its AX in EAX).
#[hook(addr = 0x71A600, conv = cdecl, name = "CFont::ProcessStringToDisplay", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn process_string_to_display(px: f32, py: f32, text: GPtr<u8>) -> u16 {
    process_current_string_call(1, px, py, text)
}

/// `CFont::GetTextRect(CRect* rect, float x, float y, const GxtChar*)` (0x71A620): the line count from
/// `ProcessCurrentString(false, ..)` (its low byte); left/right from the centre size (`x -/+ (size *
/// 0.5 + 4)`), the right-justify wrap (`left = RightJustifyWrap - 4`, `right = x + 4`) or `x - 4` /
/// `WrapX + 4`; `top = y - 4`, `bottom = lineHeight * lines + y + 4`. Stores +0, +8, +4, +0xC.
#[hook(addr = 0x71A620, conv = cdecl, name = "CFont::GetTextRect", expect = "8B 44 24 10 8B 4C 24 0C")]
pub fn get_text_rect(rect: GPtr<f32>, px: f32, py: f32, text: GPtr<u8>) {
    let lines = process_current_string_call(0, px, py, text) as u8;
    let r: GPtr<u8> = rect.cast();
    if d8(det::CENTRE) != 0 {
        let half = || x(df(det::CENTRE_SIZE)).fmul_m(K_HALF).fadd_m(K_FOUR);
        wr(r, 0, half().fsubr_m(px).to_f32());
        wr(r, 8, half().fadd_m(px).to_f32());
    } else if d8(det::RIGHT_JUSTIFY) != 0 {
        wr(r, 0, x(df(det::RIGHT_JUSTIFY_WRAP)).fsub_m(K_FOUR).to_f32());
        wr(r, 8, x(px).fadd_m(K_FOUR).to_f32());
    } else {
        wr(r, 0, x(px).fsub_m(K_FOUR).to_f32());
        wr(r, 8, x(df(det::WRAP_X)).fadd_m(K_FOUR).to_f32());
    }
    wr(r, 4, x(py).fsub_m(K_FOUR).to_f32());
    let bottom = line_height()
        .fmul(xi(i32::from(lines)))
        .fadd_m(py)
        .fadd_m(K_FOUR);
    wr(r, 0xC, bottom.to_f32());
}

/// `CFont::PrintString(float x, float y, const GxtChar*)` (0x71A700): nothing for "" or a text starting
/// with '*'. With a background: the render state's r, g, b (not alpha) from the text colour, the text
/// rect (`GetTextRect` from {1e6, -1e6, -1e6, 1e6}), then either the menu window (enlarged by 1 on
/// every side: top, bottom, left, right) or `CSprite2d::DrawRect`, and the background flag cleared.
/// Then `ProcessCurrentString(true, ..)`.
#[hook(addr = 0x71A700, conv = cdecl, name = "CFont::PrintString-null-terminated", expect = "83 EC 10 53 8B 5C 24 20")]
pub fn print_string(px: f32, py: f32, text: GPtr<u8>) {
    let c0 = b(text, 0);
    if c0 == b'*' || c0 == 0 {
        return;
    }
    if d8(det::BACKGROUND) != 0 {
        let rs: GPtr<u8> = RENDER_STATE.ptr().cast();
        wr(rs, 0x14, d8(det::COLOR));
        wr(rs, 0x15, d8(det::COLOR + 1));
        wr(rs, 0x16, d8(det::COLOR + 2));
        let mut rect = [
            f32::from_bits(K_1E6_BITS),
            f32::from_bits(K_M1E6_BITS),
            f32::from_bits(K_M1E6_BITS),
            f32::from_bits(K_1E6_BITS),
        ];
        let rp = GPtr::from_raw(rect.as_mut_ptr());
        get_text_rect_call(rp, px, py, text);
        if d8(det::BACKGROUND_OUTLINE) != 0 {
            let back: u32 = rd(det_ptr(), det::BACKGROUND_COLOR);
            let r: GPtr<u8> = rp.cast();
            wr(r, 4, x(rd(r, 4)).fsub_m(K_ONE).to_f32());
            wr(r, 0xC, x(rd(r, 0xC)).fadd_m(K_ONE).to_f32());
            wr(r, 0, x(rd(r, 0)).fsub_m(K_ONE).to_f32());
            wr(r, 8, x(rd(r, 8)).fadd_m(K_ONE).to_f32());
            draw_window(g(FRONT_END_MENU_MANAGER), rp, 0, 0, back, 0, 1);
        } else {
            sprite_draw_rect(rp, det_ptr().byte_add(det::BACKGROUND_COLOR));
        }
        wr::<u8>(det_ptr(), det::BACKGROUND, 0);
    }
    process_current_string_call(1, px, py, text);
}

/// `CFont::PrintStringFromBottom(float x, float y, const GxtChar*)` (0x71A820): moves `y` up by
/// `lineHeight * lines` (`ProcessCurrentString(false, ..)`, AX sign-extended) and, unless the slant is
/// 0, by `(SlantRefX - x) * Slant + SlantRefY`; then `PrintString`.
#[hook(addr = 0x71A820, conv = cdecl, name = "CFont::PrintStringFromBottom", expect = "51 8B 44 24 0C 56")]
pub fn print_string_from_bottom(px: f32, py: f32, text: GPtr<u8>) {
    let lines = process_current_string_call(0, px, py, text) as i16;
    let mut y = line_height()
        .fmul(xi(i32::from(lines)))
        .fsubr_m(py)
        .to_f32();
    let slant = df(det::SLANT);
    if x(slant).fcom(x(K_ZERO)).test_parity(0x44) {
        y = x(df(det::SLANT_REF_X))
            .fsub_m(px)
            .fmul_m(slant)
            .fadd_m(df(det::SLANT_REF_Y))
            .fsubr_m(y)
            .to_f32();
    }
    print_string_call(px, y, text);
}

/// `CFont::DrawFonts()` (0x71A210): `jmp CFont::RenderFontBuffer`.
#[hook(addr = 0x71A210, conv = cdecl, name = "CFont::DrawFonts", expect = "E9 2B F6 FF FF 90 90")]
pub fn draw_fonts() {
    render_font_buffer_call();
}

// ---------------------------------------------------------------------------------------------------
// Printing into the font buffer
// ---------------------------------------------------------------------------------------------------

/// `RsGlobal.maximumWidth` (0xC17044, int)
const RS_MAX_WIDTH: usize = 0xC17044;
/// `RsGlobal.maximumHeight` (0xC17048, int)
const RS_MAX_HEIGHT: usize = 0xC17048;
/// 1/640 (0x859520)
const K_INV_640: f32 = f32::from_bits(0x3ACC_CCCD);
/// 1/448 (0x859524)
const K_INV_448: f32 = f32::from_bits(0x3B12_4925);
/// The font buffer's flush limit (0xC7187C): a range that would pass it renders the buffer first.
const FONT_BUFFER_LIMIT: u32 = 0xC7187C;
/// `CFontDetails` +0x1E: set while the drop shadow / edge copies print (they keep the drop colour).
const DET_IS_SHADOW: isize = 0x1E;
/// `CFontChar` (render state) +0x28, passed to `ParseToken` as `isBlip`.
const RS_CONTAIN_IMAGES: isize = 0x28;
/// `CFontChar` (render state) +0x2C, the font texture (u16).
const RS_FONT_TEXTURE: isize = 0x2C;

/// Drop colour into the text colour (b, r, g, a in the original's store order), shadow flag on.
fn use_drop_color() {
    let d = det_ptr();
    wr(d, det::COLOR + 2, d8(det::DROP_COLOR + 2));
    wr(d, det::COLOR, d8(det::DROP_COLOR));
    wr::<u8>(d, DET_IS_SHADOW, 1);
    wr(d, det::COLOR + 1, d8(det::DROP_COLOR + 1));
    wr(d, det::COLOR + 3, d8(det::DROP_COLOR + 3));
}

/// `maxWidth * (1/640) * n`
fn dx_of(n: X87) -> X87 {
    xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(K_INV_640).fmul(n)
}

/// `maxHeight * (1/448) * n`
fn dy_of(n: X87) -> X87 {
    xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(K_INV_448).fmul(n)
}

/// `SlantRef += (dx(n), dy(n))` when the slant is not 0.
fn shift_slant_ref(slant_set: bool, n: i32) {
    if !slant_set {
        return;
    }
    let d = det_ptr();
    wr(d, det::SLANT_REF_X, dx_of(xi(n)).fadd_m(df(det::SLANT_REF_X)).to_f32());
    wr(d, det::SLANT_REF_Y, dy_of(xi(n)).fadd_m(df(det::SLANT_REF_Y)).to_f32());
}

/// Colour bytes r, g (then the amount), b, a back from the saved word, shadow flag off.
fn restore_after_shadow(color: u32, amount_off: isize, amount: u8) {
    let d = det_ptr();
    wr(d, det::COLOR, color as u8);
    wr(d, det::COLOR + 1, (color >> 8) as u8);
    wr(d, amount_off, amount);
    wr(d, det::COLOR + 2, (color >> 16) as u8);
    wr(d, det::COLOR + 3, (color >> 24) as u8);
    wr::<u8>(d, DET_IS_SHADOW, 0);
}

/// `CFont::PrintString(float x, float y, const GxtChar* start, const GxtChar* end, float spacing)`
/// (0x719B40): renders the buffer when the font texture changes; with a drop shadow (else with an
/// edge) prints the range once (shadow: `+dx, +dy`) / eight times (edge: `+-dx +-dy`, `+-dx`, `+-dy`)
/// in the drop colour, `dx = maxWidth/640 * n`, `dy = maxHeight/448 * n`, with the amount cleared
/// meanwhile and the slant reference shifted by `(dx, dy)` (and left shifted), then restores the colour
/// word read after the render. Then appends a `CFontChar` header (x, y, scale, colour, spacing, slant,
/// slant ref, shadow flag, style, proportional, texture, edge space; bytes 0..3, 0x2B, 0x2F left) and the
/// characters to the font buffer (rendering it first when `end - start` would pass 0xC7187C),
/// NUL-terminated and 4-aligned. A token goes through `ParseToken` into a copy of the text colour;
/// outside the shadow pass and without a button symbol its r, g, b become the text colour and the saved
/// colour, and the alpha to restore is reread; the token bytes are copied too. At the end (outside the
/// shadow pass) the text colour gets the saved r, g, b and alpha back.
#[hook(addr = 0x719B40, conv = cdecl, name = "CFont::PrintString-ranged", expect = "83 EC 08 0F B6 05 94 1A C7 00")]
pub fn print_string_ranged(px: f32, py: f32, start: GPtr<u8>, end: GPtr<u8>, spacing: f32) {
    let d = det_ptr();
    let rs: GPtr<u8> = RENDER_STATE.ptr().cast();
    let mut saved: u32 = rd(d, det::COLOR);
    let mut color: u32 = saved;
    if rd::<u16>(rs, RS_FONT_TEXTURE) != u16::from(d8(det::FONT_TEXTURE)) {
        render_font_buffer_call();
        color = rd(d, det::COLOR);
        wr(rs, RS_FONT_TEXTURE, u16::from(d8(det::FONT_TEXTURE)));
    }
    let drop = i32::from(d8(det::DROP_AMOUNT) as i8);
    if xi(drop).fcom(x(K_ZERO)).test_parity(0x44) {
        let slant_set = x(df(det::SLANT)).fcom(x(K_ZERO)).test_parity(0x44);
        let amount = drop as i16;
        use_drop_color();
        wr::<u8>(d, det::DROP_AMOUNT, 0);
        shift_slant_ref(slant_set, i32::from(amount));
        let n = xi(i32::from(amount));
        let yy = dy_of(n).fadd_m(py).to_f32();
        let xx = dx_of(n).fadd_m(px).to_f32();
        print_string_ranged_call(xx, yy, start, end, spacing);
        restore_after_shadow(color, det::DROP_AMOUNT, amount as u8);
    } else {
        // 0x719CA3: no drop shadow; the edge.
        let edge = i32::from(d8(det::EDGE_AMOUNT) as i8);
        if xi(edge).fcom(x(K_ZERO)).test_parity(0x44) {
            let slant_set = x(df(det::SLANT)).fcom(x(K_ZERO)).test_parity(0x44);
            let amount = edge as i16;
            use_drop_color();
            wr::<u8>(d, det::EDGE_AMOUNT, 0);
            shift_slant_ref(slant_set, i32::from(amount));
            // `fild; fstp [arg]`: the amount as a float, then `fmul` by that memory operand.
            let ef = xi(i32::from(amount)).to_f32();
            let ddx = || dx_of(x(ef));
            let ddy = || dy_of(x(ef));
            let pr = |xx: X87, yy: X87| {
                print_string_ranged_call(xx.to_f32(), yy.to_f32(), start, end, spacing)
            };
            pr(ddx().fadd_m(px), ddy().fsubr_m(py));
            pr(ddx().fsubr_m(px), ddy().fsubr_m(py));
            pr(ddx().fadd_m(px), ddy().fadd_m(py));
            pr(ddx().fsubr_m(px), ddy().fadd_m(py));
            pr(ddx().fadd_m(px), x(py));
            pr(ddx().fsubr_m(px), x(py));
            pr(x(px), ddy().fadd_m(py));
            pr(x(px), ddy().fsubr_m(py));
            restore_after_shadow(color, det::EDGE_AMOUNT, amount as u8);
        }
    }

    // 0x719F30: the CFontChar header.
    let span = (end.addr() as u32).wrapping_sub(start.addr() as u32);
    let mut out = crate::font::RENDER_END_PTR.get();
    if out >= FONT_BUFFER_LIMIT.wrapping_sub(span) {
        render_font_buffer_call();
        out = crate::font::RENDER_END_PTR.get();
    }
    let h: GPtr<u8> = g(out as usize);
    wr(h, 4, px);
    wr(h, 8, py);
    wr(h, 0xC, rd::<u32>(d, det::SCALE_X));
    wr(h, 0x10, rd::<u32>(d, det::SCALE_Y));
    for k in 0..4 {
        wr(h, 0x14 + k, d8(det::COLOR + k));
    }
    wr(h, 0x18, spacing);
    wr(h, 0x1C, rd::<u32>(d, det::SLANT));
    wr(h, 0x20, rd::<u32>(d, det::SLANT_REF_X));
    let mut alpha = (saved >> 24) as u8;
    wr(h, 0x24, rd::<u32>(d, det::SLANT_REF_Y));
    wr(h, 0x29, d8(det::FONT_STYLE));
    wr(h, 0x2A, d8(det::PROPORTIONAL));
    wr(h, 0x2C, u16::from(d8(det::FONT_TEXTURE)));
    wr(h, 0x28, d8(DET_IS_SHADOW));
    wr(h, 0x2E, d8(det::EDGE_SPACE));
    out = out.wrapping_add(0x30);
    crate::font::RENDER_END_PTR.set(out);

    let mut p = start;
    let mut tmp = [0u8; 4];
    let tmp_p: GPtr<u8> = GPtr::from_raw(tmp.as_mut_ptr());
    while (p.addr() as u32) < (end.addr() as u32) {
        let c = b(p, 0);
        if c != b'~' {
            wr(g::<u8>(out as usize), 0, c);
            out = out.wrapping_add(1);
            p = p.byte_add(1);
            crate::font::RENDER_END_PTR.set(out);
            continue;
        }
        wr(tmp_p, 0, rd::<u32>(d, det::COLOR));
        let is_blip = u32::from(rd::<u8>(rs, RS_CONTAIN_IMAGES));
        let next = parse_token_call(p, tmp_p, is_blip, GPtr::NULL);
        if d8(DET_IS_SHADOW) == 0 && PS2_SYMBOL.get() == 0 {
            let c: u32 = rd(tmp_p, 0);
            alpha = d8(det::COLOR + 3);
            wr(d, det::COLOR, c as u8);
            wr(d, det::COLOR + 1, (c >> 8) as u8);
            wr(d, det::COLOR + 2, (c >> 16) as u8);
            saved = (saved & 0xFF00_0000) | (c & 0x00FF_FFFF);
        }
        out = crate::font::RENDER_END_PTR.get();
        PS2_SYMBOL.set(0);
        if p != next {
            while p != next {
                wr(g::<u8>(out as usize), 0, b(p, 0));
                out = out.wrapping_add(1);
                p = p.byte_add(1);
            }
            crate::font::RENDER_END_PTR.set(out);
        }
    }
    wr::<u8>(g(out as usize), 0, 0);
    out = out.wrapping_add(1);
    crate::font::RENDER_END_PTR.set(out);
    while out & 3 != 0 {
        out = out.wrapping_add(1);
    }
    crate::font::RENDER_END_PTR.set(out);
    if d8(DET_IS_SHADOW) == 0 {
        wr(d, det::COLOR, saved as u8);
        wr(d, det::COLOR + 1, (saved >> 8) as u8);
        wr(d, det::COLOR + 2, (saved >> 16) as u8);
        wr(d, det::COLOR + 3, alpha);
    }
}

// ---------------------------------------------------------------------------------------------------
// Rendering the font buffer
// ---------------------------------------------------------------------------------------------------

/// `CFont::Sprite` (0xC71AD0, CSprite2d[2])
const FONT_SPRITES: usize = 0xC71AD0;
/// `CFont::ButtonSprite` (0xC71AD8, CSprite2d[15])
const BUTTON_SPRITES: usize = 0xC71AD8;
/// The first character after the buffer's first `CFontChar` header (0xC716B0 + 0x30).
const FONT_BUFFER_TEXT: u32 = 0xC716E0;
/// 17.0 (0x85F100), the button advance per unit of ScaleY
const K_17: f32 = f32::from_bits(0x4188_0000);
/// `CFontChar` (render state) +0x2A, proportional
const RS_PROP_ON: isize = 0x2A;

orig! {
    /// `CSprite2d::SetRenderState()` (0x727B30, thiscall)
    pub fn sprite2d_set_render_state(this: GPtr<u8>) = thiscall 0x727B30 => unported;
    /// `RwRenderStateSet(state, value)` (`RwEngineInstance` +0x20)
    pub fn rw_render_state_set(state: i32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CFontChar::Set(const CFontChar&)` (0x718E50, thiscall)
    pub fn font_char_set_call(this: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x718E50 => crate::font::font_char_set;
    /// `CFont::PrintChar(float x, float y, uint8 c)` (0x718A10)
    pub fn print_char_call(px: f32, py: f32, c: u32) = cdecl 0x718A10 => print_char;
    /// `CSprite::FlushSpriteBuffer()` (0x70CF20)
    pub fn flush_sprite_buffer() = cdecl 0x70CF20 => unported;
    /// `CSprite2d::RenderVertexBuffer()` (0x7273D0)
    pub fn render_vertex_buffer() = cdecl 0x7273D0 => unported;
}

fn rs_f(off: isize) -> f32 {
    rd(RENDER_STATE.ptr().cast::<u8>(), off)
}

fn rs8(off: isize) -> u8 {
    rd(RENDER_STATE.ptr().cast::<u8>(), off)
}

fn font_sprite(tex: i16) -> GPtr<u8> {
    g(FONT_SPRITES.wrapping_add((i32::from(tex) * 4) as u32 as usize))
}

/// `CFont::RenderFontBuffer()` (0x719840): nothing for an empty buffer. Otherwise the render state
/// from the first header (font sprite render state, `RwRenderStateSet(12, 1)`), then per character: a
/// NUL (4-aligned) starts the next header, leading tokens go through `ParseToken` into the local colour
/// (copied to the render state unless it holds images) until a button symbol, the glyph is the sub-font
/// character (with a style) or `c - 0x20` (0x91 -> 0x40, > 0x9B -> 0), y is slanted when the slant is set,
/// `PrintChar` draws it (not for a button with images), and x advances by the button width
/// (`ScaleY * k + edge`), by `(width + edge) * ScaleX`, and by the spacing after a 0 glyph. A button
/// sets the font sprite's render state again. At the end the sprite and vertex buffers are flushed and
/// the buffer is reset.
#[hook(addr = 0x719840, conv = cdecl, name = "CFont::RenderFontBuffer", expect = "A1 A8 16 C7 00 83 EC 18")]
pub fn render_font_buffer() {
    if crate::font::RENDER_END_PTR.get() == crate::font::RENDER_FONT_BUFFER {
        return;
    }
    let rs: GPtr<u8> = RENDER_STATE.ptr().cast();
    sprite2d_set_render_state(font_sprite(rd(rs, RS_FONT_TEXTURE)));
    rw_render_state_set(0xC, 1);
    font_char_set_call(rs, g(crate::font::RENDER_FONT_BUFFER as usize));
    let mut color = [0u8; 4];
    let cp: GPtr<u8> = GPtr::from_raw(color.as_mut_ptr());
    let load_state = || {
        for k in 0..4 {
            wr(cp, k, rs8(0x14 + k));
        }
        (rs_f(4), rs_f(8))
    };
    let (mut px, mut py) = load_state();
    let mut p = FONT_BUFFER_TEXT;
    let mut end = crate::font::RENDER_END_PTR.get();
    if end > p {
        loop {
            if g::<u8>(p as usize).read() == 0 {
                p = p.wrapping_add(1);
                while p & 3 != 0 {
                    p = p.wrapping_add(1);
                }
                if p >= end {
                    break;
                }
                font_char_set_call(rs, g(p as usize));
                (px, py) = load_state();
                p = p.wrapping_add(0x30);
            }
            PS2_SYMBOL.set(0);
            let mut sym = 0u8;
            while g::<u8>(p as usize).read() == b'~' && sym == 0 {
                let next = parse_token_call(g(p as usize), cp, u32::from(rs8(RS_CONTAIN_IMAGES)), GPtr::NULL);
                p = next.addr() as u32;
                if rs8(RS_CONTAIN_IMAGES) == 0 {
                    for k in 0..4 {
                        wr(rs, 0x14 + k, rd::<u8>(cp, k));
                    }
                }
                sym = PS2_SYMBOL.get();
            }
            let c = g::<u8>(p as usize).read().wrapping_sub(0x20);
            let style = rs8(0x29);
            // The glyph dword passed on: only its low byte is written (the rest is stale stack).
            let glyph = if style != 0 {
                find_sub_font_character_call(u32::from(c), u32::from(style))
            } else {
                match c {
                    0x91 => 0x40,
                    v if v > 0x9B => 0,
                    v => v,
                }
            };
            let mut cl = glyph;
            let slant = rs_f(0x1C);
            if x(slant).fcom(x(K_ZERO)).test_parity(0x44) {
                py = x(rs_f(0x20))
                    .fsub_m(px)
                    .fmul_m(slant)
                    .fadd_m(rs_f(0x24))
                    .to_f32();
            }
            if sym == 0 || rs8(RS_CONTAIN_IMAGES) == 0 {
                print_char_call(px, py, u32::from(glyph));
                sym = PS2_SYMBOL.get();
                cl = glyph;
            }
            let edge = i32::from(rs8(0x2E) as i8);
            if sym != 0 {
                px = x(rs_f(0x10))
                    .fmul_m(K_17)
                    .fadd(xi(edge))
                    .fadd_m(px)
                    .to_f32();
            } else {
                let ch = if cl == 0x3F { 0 } else { cl };
                let tex = i32::from(rd::<i16>(rs, RS_FONT_TEXTURE));
                let base = (FONT_DATA as u32).wrapping_add((tex.wrapping_mul(0xD2)) as u32);
                let w = if rs8(RS_PROP_ON) == 1 {
                    g::<u8>(base.wrapping_add(u32::from(ch)) as usize).read()
                } else {
                    g::<u8>(base.wrapping_add(0xD1) as usize).read()
                };
                px = xi(edge)
                    .fadd(xi(i32::from(w)))
                    .fmul_m(rs_f(0xC))
                    .fadd_m(px)
                    .to_f32();
            }
            if cl == 0 {
                px = x(rs_f(0x18)).fadd_m(px).to_f32();
            }
            let at_end = g::<u8>(p as usize).read() == 0;
            if !at_end && sym == 0 {
                p = p.wrapping_add(1);
            } else if sym != 0 {
                PS2_SYMBOL.set(0);
                sprite2d_set_render_state(font_sprite(rd(rs, RS_FONT_TEXTURE)));
            }
            end = crate::font::RENDER_END_PTR.get();
            if p >= end {
                break;
            }
        }
    }
    flush_sprite_buffer();
    render_vertex_buffer();
    crate::font::RENDER_END_PTR.set(crate::font::RENDER_FONT_BUFFER);
}

orig! {
    /// `CSprite2d::Draw(const CRect&, const CRGBA&)` (0x728350, thiscall)
    pub fn sprite2d_draw(this: GPtr<u8>, rect: GPtr<u8>, colour: GPtr<u8>) = thiscall 0x728350 => unported;
    /// `CSprite2d::AddToBuffer(const CRect&, const CRGBA&, u1, v1, u2, v2, u3, v3, u4, v4)` (0x728200)
    pub fn sprite2d_add_to_buffer(rect: GPtr<u8>, colour: GPtr<u8>, u1: f32, v1: f32, u2: f32, v2: f32, u3: f32, v3: f32, u4: f32, v4: f32) = cdecl 0x728200 => unported;
    /// `CFont::GetLetterIdPropValue(uint8)` (0x718770)
    pub fn get_letter_id_prop_value_call_st0(c: u32) -> f64 = cdecl 0x718770 => get_letter_id_prop_value_call_st0_rust;
}

/// The `.rdata` constants of `PrintChar`.
mod pc {
    /// 19.0 (0x865348)
    pub const K_19: f32 = f32::from_bits(0x4198_0000);
    /// 1/32 (0x864E10)
    pub const K_1_32: f32 = f32::from_bits(0x3D00_0000);
    /// 1/16 (0x858620)
    pub const K_1_16: f32 = f32::from_bits(0x3D80_0000);
    /// 0.0001 (0x858FC4)
    pub const K_0_0001: f32 = f32::from_bits(0x38D1_B717);
    /// 0.078125 (0x8652C8, and the double at 0x872928)
    pub const K_0_078125: f32 = f32::from_bits(0x3DA0_0000);
    /// 0.001 (0x858CDC)
    pub const K_0_001: f32 = f32::from_bits(0x3A83_126F);
    /// 0.0021 (0x872920)
    pub const K_0_0021: f32 = f32::from_bits(0x3B09_A027);
    /// 0.015 (0x859F00)
    pub const K_0_015: f32 = f32::from_bits(0x3C75_C28F);
    /// 0.016 (0x863C00)
    pub const K_0_016: f32 = f32::from_bits(0x3C83_126F);
    /// 40.0 (0x858A10)
    pub const K_40: f32 = f32::from_bits(0x4220_0000);
    /// 0.01 (0x858C58)
    pub const K_0_01: f32 = f32::from_bits(0x3C23_D70A);
    /// 0.009 (0x859048)
    pub const K_0_009: f32 = f32::from_bits(0x3C13_74BC);
    /// 0.0121 (0x87291C)
    pub const K_0_0121: f32 = f32::from_bits(0x3C46_3F14);
    /// 0.00055 (0x8717B0)
    pub const K_0_00055: f32 = f32::from_bits(0x3A10_2DE0);
}

/// `fcomp` + `test ah, 5` + `jnp` taken: `a < b` (ordered).
fn below(a: X87, b: X87) -> bool {
    !a.fcom(b).test_parity(0x05)
}

/// `CFont::PrintChar(float x, float y, uint8 c)` (0x718A10): nothing off screen (`y < 0`,
/// `maxHeight < y`, `x < 0`, `maxWidth < x`). A button symbol draws `ButtonSprite[PS2Symbol]` at
/// {x, 19*sy + y, 17*sy + x, 2*sy + y} in white with the render-state alpha. A character: 0 / '?' are not
/// drawn (after the prop-value call), style 1 maps 0xD0 to 0; the 16x16 atlas cell (u = col/16) goes to
/// `CSprite2d::AddToBuffer` with the render-state colour: fonts 0 / 1 use 1/12.8 rows (v = row *
/// 0.078125) and fixed insets, 40 * sy rows below 0xC0 and a slant variant; other fonts 1/16 rows and
/// `32 * sx * propValue` widths.
#[hook(addr = 0x718A10, conv = cdecl, name = "CFont::PrintChar", expect = "D9 44 24 08 83 EC 20")]
pub fn print_char(px: f32, py: f32, c: u32) {
    use pc::*;
    let max_w = xi(g::<i32>(RS_MAX_WIDTH).read());
    let max_h = xi(g::<i32>(RS_MAX_HEIGHT).read());
    if below(x(py), x(K_ZERO)) || below(max_h, x(py)) || below(x(px), x(K_ZERO)) || below(max_w, x(px)) {
        return;
    }
    let colour: GPtr<u8> = RENDER_STATE.ptr().cast::<u8>().byte_add(0x14);
    let sx = rs_f(0xC);
    let sy = rs_f(0x10);
    let mut rect = [0f32; 4];
    let rp: GPtr<u8> = GPtr::from_raw(rect.as_mut_ptr()).cast();
    let sym = PS2_SYMBOL.get();
    if sym != 0 {
        wr(rp, 0, px);
        wr(rp, 0xC, x(sy).fadd(x(sy)).fadd_m(py).to_f32());
        wr(rp, 8, x(sy).fmul_m(K_17).fadd_m(px).to_f32());
        wr(rp, 4, x(sy).fmul_m(K_19).fadd_m(py).to_f32());
        let mut col = [0u8; 4];
        let cp = rgba_ctor(GPtr::from_raw(col.as_mut_ptr()), 0xFF, 0xFF, 0xFF, u32::from(rs8(0x17)));
        sprite2d_draw(g(BUTTON_SPRITES + usize::from(sym) * 4), rp, cp);
        return;
    }
    let mut arg = c;
    let mut ch = arg as u8;
    let mut hidden = false;
    if ch == 0 || ch == 0x3F {
        ch = 0;
        hidden = true;
        arg &= !0xFF;
    }
    let pv = get_letter_id_prop_value_call(arg).fmul_m(K_1_32).to_f32();
    if rs8(0x29) == 1 && ch == 0xD0 {
        ch = 0;
    }
    let row = xi(i32::from(ch >> 4));
    let u = xi(i32::from(ch & 0xF)).fmul_m(K_1_16).to_f32();
    let tex = rd::<u16>(RENDER_STATE.ptr().cast::<u8>(), RS_FONT_TEXTURE);
    let add = |u1: f32, v1: f32, u2: f32, v2: f32, u3: f32, v3: f32, u4: f32, v4: f32| {
        sprite2d_add_to_buffer(rp, colour, u1, v1, u2, v2, u3, v3, u4, v4)
    };
    if tex != 0 && tex != 1 {
        let v = row.fmul_m(K_1_16).to_f32();
        if hidden {
            return;
        }
        wr(rp, 0xC, py);
        wr(rp, 0, px);
        wr(rp, 8, x(sx).fmul_m(K_32).fmul_m(pv).fadd_m(px).to_f32());
        wr(rp, 4, x(sy).fmul_m(K_32).fmul_m(K_HALF).fadd_m(py).to_f32());
        let vb = x(v).fadd_m(K_1_16).to_f32();
        let u2 = x(pv).fmul_m(K_1_16).fadd_m(u).to_f32();
        let vb2 = x(vb).fsub_m(K_0_0001).to_f32();
        let u22 = x(u2).fsub_m(K_0_0001).to_f32();
        add(u, v, u2, v, u, vb, u22, vb2);
        return;
    }
    let v = row.fmul_m(K_0_078125).to_f32();
    if hidden {
        return;
    }
    let slant = rs_f(0x1C);
    wr(rp, 0, px);
    if !x(slant).fcom(x(K_ZERO)).test_parity(0x44) {
        wr(rp, 0xC, py);
        wr(rp, 8, x(sx).fmul_m(K_32).fadd_m(px).to_f32());
        if ch >= 0xC0 {
            wr(rp, 4, x(sy).fmul_m(K_32).fmul_m(K_HALF).fadd_m(py).to_f32());
            let vb0 = x(v).fadd_m(K_0_078125);
            let u2 = x(u).fadd_m(K_1_16).fsub_m(K_0_001).to_f32();
            let v1 = x(v).fadd_m(K_0_0021).to_f32();
            let b4 = vb0.fsub(x(K_0_015)).to_f32();
            let a3 = vb0.fsub_m(K_0_016).to_f32();
            add(u, v1, u2, v1, u, a3, u2, b4);
        } else {
            wr(rp, 4, x(sy).fmul_m(K_40).fmul_m(K_HALF).fadd_m(py).to_f32());
            let vb = x(v).fadd_m(K_0_078125).fsub_m(K_0_0021).to_f32();
            let u2 = x(u).fadd_m(K_1_16).fsub_m(K_0_001).to_f32();
            let v1 = x(v).fadd_m(K_0_0021).to_f32();
            add(u, v1, u2, v1, u, vb, u2, vb);
        }
    } else {
        wr(rp, 0xC, x(py).fadd_m(K_0_015).to_f32());
        wr(rp, 8, x(sx).fmul_m(K_32).fadd_m(px).to_f32());
        wr(
            rp,
            4,
            x(sy).fmul_m(K_40).fmul_m(K_HALF).fadd_m(py).fadd_m(K_0_015).to_f32(),
        );
        let vb0 = x(v).fadd_m(K_0_078125);
        let u2 = x(u).fadd_m(K_1_16).fsub_m(K_0_001).to_f32();
        let c4 = vb0.fsub(x(K_0_0021)).fadd_m(K_0_01).to_f32();
        let d3 = vb0.fsub_m(K_0_009).to_f32();
        let v2 = x(v).fadd_m(K_0_0121).to_f32();
        let v1 = x(v).fadd_m(K_0_00055).to_f32();
        add(u, v1, u2, v2, u, d3, u2, c4);
    }
}

// ---------------------------------------------------------------------------------------------------
// Start-up / shut-down
// ---------------------------------------------------------------------------------------------------

orig! {
    /// `CTxdStore::AddTxdSlot(const char*)` (0x731C80)
    pub fn add_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731C80 => sa_stream::txdstore::add_txd_slot;
    /// `CTxdStore::LoadTxd(int32, const char* filename)` (0x7320B0)
    pub fn load_txd(slot: i32, name: GPtr<u8>) -> u8 = cdecl 0x7320B0 => sa_stream::txdstore::load_txd_filename;
    /// `CTxdStore::AddRef(int32)` (0x731A00)
    pub fn txd_add_ref(slot: i32) = cdecl 0x731A00 => sa_stream::txdstore::add_ref;
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    pub fn push_current_txd() = cdecl 0x7316A0 => sa_stream::txdstore::push_current_txd;
    /// `CTxdStore::SetCurrentTxd(int32)` (0x7319C0)
    pub fn set_current_txd(slot: i32) = cdecl 0x7319C0 => sa_stream::txdstore::set_current_txd;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    pub fn pop_current_txd() = cdecl 0x7316B0 => sa_stream::txdstore::pop_current_txd;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    pub fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => sa_stream::txdstore::find_txd_slot_name;
    /// `CTxdStore::RemoveTxdSlot(int32)` (0x731CD0)
    pub fn remove_txd_slot(slot: i32) = cdecl 0x731CD0 => sa_stream::txdstore::remove_txd_slot;
    /// `CSprite2d::SetTexture(const char* name, const char* mask)` (0x7272B0, thiscall)
    pub fn sprite2d_set_texture(this: GPtr<u8>, name: GPtr<u8>, mask: GPtr<u8>) = thiscall 0x7272B0 => unported;
    /// `CSprite2d::Delete()` (0x727240, thiscall)
    pub fn sprite2d_delete(this: GPtr<u8>) = thiscall 0x727240 => unported;
    /// `CFont::LoadFontValues()` (0x7187C0)
    pub fn load_font_values_call() = cdecl 0x7187C0 => load_font_values;
    /// `CFont::SetScale(float, float)` (0x719380)
    pub fn set_scale_call(w: u32, h: u32) = cdecl 0x719380 => crate::font::set_scale;
    /// `CFont::SetSlantRefPoint(float, float)` (0x719400)
    pub fn set_slant_ref_point_call(px: u32, py: u32) = cdecl 0x719400 => crate::font::set_slant_ref_point;
    /// `CFont::SetSlant(float)` (0x719420)
    pub fn set_slant_call(v: u32) = cdecl 0x719420 => crate::font::set_slant;
    /// `CFont::SetOrientation(eFontAlignment)` (0x719610)
    pub fn set_orientation_call(v: u32) = cdecl 0x719610 => crate::font::set_orientation;
    /// `CFont::SetJustify(bool)` (0x719600)
    pub fn set_justify_call(v: u32) = cdecl 0x719600 => crate::font::set_justify;
    /// `CFont::SetWrapx(float)` (0x7194D0)
    pub fn set_wrapx_call(v: u32) = cdecl 0x7194D0 => crate::font::set_wrapx;
    /// `CFont::SetCentreSize(float)` (0x7194E0)
    pub fn set_centre_size_call(v: u32) = cdecl 0x7194E0 => crate::font::set_centre_size;
    /// `CFont::SetBackground(bool, bool)` (0x7195C0)
    pub fn set_background_call(on: u32, outline: u32) = cdecl 0x7195C0 => crate::font::set_background;
    /// `CFont::SetBackgroundColor(CRGBA)` (0x7195E0)
    pub fn set_background_color_call(c: u32) = cdecl 0x7195E0 => crate::font::set_background_color;
    /// `CFont::SetProportional(bool)` (0x7195B0)
    pub fn set_proportional_call(v: u32) = cdecl 0x7195B0 => crate::font::set_proportional;
    /// `CFont::SetFontStyle(eFontStyle)` (0x719490)
    pub fn set_font_style_call(v: u32) = cdecl 0x719490 => crate::font::set_font_style;
    /// `CFont::SetRightJustifyWrap(float)` (0x7194F0)
    pub fn set_right_justify_wrap_call(v: u32) = cdecl 0x7194F0 => crate::font::set_right_justify_wrap;
    /// `CFont::SetAlphaFade(float)` (0x719500)
    pub fn set_alpha_fade_call(v: u32) = cdecl 0x719500 => crate::font::set_alpha_fade;
    /// `CFont::SetDropShadowPosition(int8)` (0x719570)
    pub fn set_drop_shadow_position_call(v: u32) = cdecl 0x719570 => crate::font::set_drop_shadow_position;
    /// `CFileMgr::SetDir(const char*)` (0x5387D0)
    pub fn file_set_dir(dir: GPtr<u8>) -> i32 = cdecl 0x5387D0 => sa_core::filemgr::set_dir;
    /// `CFileMgr::OpenFile(const char* path, const char* mode)` (0x538900)
    pub fn file_open(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => sa_core::filemgr::open_file;
    /// `CFileMgr::CloseFile(FILE*)` (0x5389D0)
    pub fn file_close(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => sa_core::filemgr::close_file;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    pub fn load_line(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x536F80 => unported;
    /// CRT `sscanf(s, fmt, p)` (0x8220AD)
    pub fn sscanf1(s: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `sscanf(s, fmt, p0..p7)` (0x8220AD)
    pub fn sscanf8(s: GPtr<u8>, fmt: GPtr<u8>, a0: GPtr<u8>, a1: GPtr<u8>, a2: GPtr<u8>, a3: GPtr<u8>, a4: GPtr<u8>, a5: GPtr<u8>, a6: GPtr<u8>, a7: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
}

/// "fonts" (0x86A65C)
const S_FONTS: usize = 0x86A65C;
/// "ps2btns" (0x86A620)
const S_PS2BTNS: usize = 0x86A620;

/// `CFont::Initialise()` (0x5BA690): the "fonts" txd (`models\fonts.txd`) and its two sprites,
/// `LoadFontValues`, the default text state (scale 1, slant ref (maxWidth, 0), slant 0, white with alpha
/// 0, orientation left, no justify, wrap and centre size maxWidth, no background, background colour
/// 0x80808080, proportional, style 0, right-justify wrap 0, alpha fade 255, no drop shadow), then the
/// "ps2btns" txd and `ButtonSprite[1..4]`; the current txd is pushed / popped around both.
#[hook(addr = 0x5BA690, conv = cdecl, name = "CFont::Initialise", expect = "51 56 68 5C A6 86 00")]
pub fn initialise() {
    let slot = add_txd_slot(g(S_FONTS));
    load_txd(slot, g(0x86A648));
    txd_add_ref(slot);
    push_current_txd();
    set_current_txd(slot);
    sprite2d_set_texture(g(FONT_SPRITES), g(0x86A638), g(0x86A640));
    sprite2d_set_texture(g(FONT_SPRITES + 4), g(0x86A628), g(0x86A630));
    load_font_values_call();
    set_scale_call(0x3F80_0000, 0x3F80_0000);
    let width = || xi(g::<i32>(RS_MAX_WIDTH).read()).to_f32().to_bits();
    set_slant_ref_point_call(width(), 0);
    set_slant_call(0);
    let mut c = [0u8; 4];
    let cp: GPtr<u8> = GPtr::from_raw(c.as_mut_ptr());
    set_color_call(rd(rgba_ctor(cp, 0xFF, 0xFF, 0xFF, 0), 0));
    set_orientation_call(1);
    set_justify_call(0);
    set_wrapx_call(width());
    set_centre_size_call(width());
    set_background_call(0, 0);
    set_background_color_call(rd(rgba_ctor(cp, 0x80, 0x80, 0x80, 0x80), 0));
    set_proportional_call(1);
    set_font_style_call(0);
    set_right_justify_wrap_call(0);
    set_alpha_fade_call(K_255_BITS);
    set_drop_shadow_position_call(0);
    pop_current_txd();
    let slot = add_txd_slot(g(S_PS2BTNS));
    load_txd(slot, g(0x86A60C));
    txd_add_ref(slot);
    push_current_txd();
    set_current_txd(slot);
    sprite2d_set_texture(g(BUTTON_SPRITES + 4), g(0x86A604), g(0x86A608));
    sprite2d_set_texture(g(BUTTON_SPRITES + 8), g(0x86A5F4), g(0x86A5FC));
    sprite2d_set_texture(g(BUTTON_SPRITES + 0xC), g(0x86A5E4), g(0x86A5EC));
    sprite2d_set_texture(g(BUTTON_SPRITES + 0x10), g(0x86A5D4), g(0x86A5DC));
    pop_current_txd();
}

/// `CFont::Shutdown()` (0x7189B0): deletes `Sprite[0..1]`, removes the "fonts" slot (unchecked
/// `FindTxdSlot`), deletes `ButtonSprite[1..4]` and removes the "ps2btns" slot.
#[hook(addr = 0x7189B0, conv = cdecl, name = "CFont::Shutdown", expect = "56 57 B9 D0 1A C7 00")]
pub fn shutdown() {
    sprite2d_delete(g(FONT_SPRITES));
    sprite2d_delete(g(FONT_SPRITES + 4));
    remove_txd_slot(find_txd_slot(g(S_FONTS)));
    for k in 0..4 {
        sprite2d_delete(g(BUTTON_SPRITES + 4 + k * 4));
    }
    remove_txd_slot(find_txd_slot(g(S_PS2BTNS)));
}

/// The `sscanf("%s")` target of `LoadFontValues` (0x20 bytes on the original's stack, unbounded).
const FONT_WORD_BUF: usize = 0x200;

/// `memcmp(word, lit, n) == 0` over `n` bytes of the literal (incl. its NUL), as `repe cmpsb`.
fn word_is(word: GPtr<u8>, lit: usize, n: isize) -> bool {
    (0..n).all(|i| b(word, i) == g::<u8>(lit + i as usize).read())
}

/// `CFont::LoadFontValues()` (0x7187C0): reads `DATA\FONTS.DAT` line by line (`#` and empty lines
/// skipped): `[TOTAL_FONTS]` (value read, not stored), `[FONT_ID]` (the current font, low byte),
/// `[REPLACEMENT_SPACE_CHAR]` (+0xD0), `[PROP]` (26 lines of 8 `%d`, low bytes into widths 0..0xCF)
/// and `[UNPROP]` (+0xD1) of `s_FontData[font]`. The `%d` slot is the first of the eight `[PROP]` ints
/// (one stack array, uninitialised in the original, zeroed here).
#[hook(addr = 0x7187C0, conv = cdecl, name = "CFont::LoadFontValues", expect = "83 EC 40 53 55 68 54 8B 85 00")]
pub fn load_font_values() {
    let mut font: u8 = 0;
    file_set_dir(g(0x858B54));
    let f = file_open(g(0x87290C), g(0x859C6C));
    let mut ints = [0i32; 8];
    let ip: GPtr<u8> = GPtr::from_raw(ints.as_mut_ptr()).cast();
    let mut word = [0u8; FONT_WORD_BUF];
    let wp: GPtr<u8> = GPtr::from_raw(word.as_mut_ptr());
    let fmt_d: GPtr<u8> = g(0x859508);
    let read_int = |f: GPtr<u8>| -> u8 {
        let l = load_line(f);
        sscanf1(l, fmt_d, ip);
        rd::<u8>(ip, 0)
    };
    let mut line = load_line(f);
    while !line.is_null() {
        let c = b(line, 0);
        if c != b'#' && c != 0 {
            sscanf1(line, g(0x85C714), wp);
            let base = FONT_DATA + usize::from(font) * 0xD2;
            if word_is(wp, 0x8728FC, 0xE) {
                read_int(f);
            } else if word_is(wp, 0x8728F0, 0xA) {
                font = read_int(f);
            } else if word_is(wp, 0x8728D4, 0x19) {
                let v = read_int(f);
                g::<u8>(base + 0xD0).write(v);
            } else if word_is(wp, 0x8728CC, 0x7) {
                let mut out = base;
                for _ in 0..0x1A {
                    let l = load_line(f);
                    let a = |k: isize| ip.byte_add(k * 4);
                    sscanf8(l, g(0x8728AC), a(0), a(1), a(2), a(3), a(4), a(5), a(6), a(7));
                    for k in 0..8 {
                        g::<u8>(out).write(rd::<u8>(ip, k * 4));
                        out += 1;
                    }
                }
            } else if word_is(wp, 0x8728A0, 0x9) {
                let v = read_int(f);
                g::<u8>(base + 0xD1).write(v);
            }
        }
        line = load_line(f);
    }
    file_close(f);
}

/// Every `#[native]` of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[];

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_GXT_CHAR_STRCAT,
    HOOK_GET_STRING_WIDTH,
    HOOK_GXT_CHAR_STRCPY,
    HOOK_GXT_CHAR_STRLEN,
    HOOK_GET_NEXT_SPACE,
    HOOK_STRIP_TOKENS,
    HOOK_PARSE_TOKEN,
    HOOK_GET_SCRIPT_LETTER_SIZE,
    HOOK_PROCESS_CURRENT_STRING,
    HOOK_PROCESS_STRING_TO_DISPLAY,
    HOOK_GET_TEXT_RECT,
    HOOK_PRINT_STRING,
    HOOK_PRINT_STRING_FROM_BOTTOM,
    HOOK_DRAW_FONTS,
    HOOK_PRINT_STRING_RANGED,
    HOOK_RENDER_FONT_BUFFER,
    HOOK_PRINT_CHAR,
    HOOK_INITIALISE,
    HOOK_SHUTDOWN,
    HOOK_LOAD_FONT_VALUES,
];

/// The Rust path of [`get_character_size_call`] as an ST0 value.
#[allow(dead_code)]
fn get_character_size_call_st0_rust(c: u32) -> f64 {
    crate::font::get_character_size(c).to_f64()
}

/// [`get_character_size_call_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
pub fn get_character_size_call(c: u32) -> X87 {
    X87::from_f64_exact(get_character_size_call_st0(c))
}

/// The Rust path of [`get_script_letter_size_call`] as an ST0 value.
#[allow(dead_code)]
fn get_script_letter_size_call_st0_rust(c: u32) -> f64 {
    get_script_letter_size(c).to_f64()
}

/// [`get_script_letter_size_call_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
pub fn get_script_letter_size_call(c: u32) -> X87 {
    X87::from_f64_exact(get_script_letter_size_call_st0(c))
}

/// The Rust path of [`get_string_width_call`] as an ST0 value.
#[allow(dead_code)]
fn get_string_width_call_st0_rust(s: GPtr<u8>, full: u32, script: u32) -> f64 {
    get_string_width(s, full, script).to_f64()
}

/// [`get_string_width_call_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
pub fn get_string_width_call(s: GPtr<u8>, full: u32, script: u32) -> X87 {
    X87::from_f64_exact(get_string_width_call_st0(s, full, script))
}

/// [`min_f_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
pub fn min_f(a: f32, b: f32) -> X87 {
    X87::from_f64_exact(min_f_st0(a, b))
}

/// The Rust path of [`get_letter_id_prop_value_call`] as an ST0 value.
#[allow(dead_code)]
fn get_letter_id_prop_value_call_st0_rust(c: u32) -> f64 {
    crate::font::get_letter_id_prop_value(c).to_f64()
}

/// [`get_letter_id_prop_value_call_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
pub fn get_letter_id_prop_value_call(c: u32) -> X87 {
    X87::from_f64_exact(get_letter_id_prop_value_call_st0(c))
}
