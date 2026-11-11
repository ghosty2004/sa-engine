//! `CCredits` rendering (PORT-HUD-3): `Render`, `RenderCredits`, `PrintCreditText`, `PrintCreditSpace`.
//! Ported from the 1.0 US bytes; gta-reversed `Credits.cpp` as reading aid (the bytes win).
//!
//! `RenderCredits` is 29 KB of straight-line code: 599 `PrintCreditText` calls and 108 inlined
//! `PrintCreditSpace`s, decoded from the exe into [`crate::credits_table::CREDITS`] by
//! `tools/gen/hud_credits.py` (which fails on any instruction outside the two shapes).
//!
//! Differences from gta-reversed (the bytes win): `PrintCreditText` advances the line offset by
//! `_ftol(float(offset) + scaleY * 20)` (gta-reversed: `offset += (uint32)scaleY * 20`), and the
//! inlined spaces by `_ftol(float(offset) + C)` with the constant `C = size * 25` (37.5, 25 or 12.5);
//! offsets and the scroll time are converted as unsigned (`fild` + 2^32 when negative). The text is
//! printed at the screen centre `width / 2` (integer division) and `width / 2 - 1`.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, x, xi};
use crate::credits_table::CREDITS;

/// No globals of its own (`CCredits::bCreditsGoing` / `CreditsStartTime` are declared in `hud.rs`).
pub const GLOBALS: &[GlobalDesc] = &[];

/// One step of `RenderCredits`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreditOp {
    /// `PrintCreditText(scale, scale, TheText.Get(key), &offset, scroll, hi)`: the key's address in the
    /// exe, the scale's f32 bits
    Text { key: u32, scale: u32, hi: bool },
    /// the inlined `PrintCreditSpace`: `offset = _ftol(float(offset) + add)` (`add`: f32 bits)
    Space { add: u32 },
}

/// `CCredits::bCreditsGoing`, `CCredits::CreditsStartTime`
const CREDITS_GOING: usize = 0xC6E97C;
const CREDITS_START_TIME: usize = 0xC6E978;
/// `FrontEndMenuManager.m_bMenuActive`
const MENU_ACTIVE: usize = 0xBA67A4;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `RsGlobal.maximumWidth`, `maximumHeight`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// `TheText`, `TheCamera`, `TheCamera.m_bWideScreenOn`
const THE_TEXT: usize = 0xC1B340;
const THE_CAMERA: usize = 0xB6F028;
const WIDE_SCREEN_ON: usize = 0xB6F065;

/// f32 constants of the exe (bits): 2^32 (0x858C54), 1/23 (0x8684DC), 0.92 (0x8684D8), 20 (0x858BA4),
/// 1 (0x858624), 25 (0x858FE8), -10 (0x859004)
const K_2_32: u32 = 0x4F80_0000;
const K_SCROLL: u32 = 0x3D32_1643;
const K_CENTRE: u32 = 0x3F6B_851F;
const K_20: u32 = 0x41A0_0000;
const K_1: u32 = 0x3F80_0000;
const K_25: u32 = 0x41C8_0000;
const K_END: u32 = 0xC120_0000;

fn k(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `fild dword` of an unsigned value: `+ 2^32` when the dword is negative (`test; jge; fadd`).
fn fild_u32(v: u32) -> X87 {
    let r = xi(v as i32);
    if (v as i32) < 0 { r.fadd_m(k(K_2_32)) } else { r }
}

orig! {
    /// `App::DefinedState2d()` (0x734750)
    fn defined_state_2d() = cdecl 0x734750 => unported;
    /// `CFont::SetBackground(bool, bool)` (0x7195C0)
    fn font_set_background(on: u32, outline: u32) = cdecl 0x7195C0 => crate::font::set_background;
    /// `CFont::SetCentreSize(float)` (0x7194E0)
    fn font_set_centre_size(v: u32) = cdecl 0x7194E0 => crate::font::set_centre_size;
    /// `CFont::SetOrientation(eFontAlignment)` (0x719610)
    fn font_set_orientation(v: u32) = cdecl 0x719610 => crate::font::set_orientation;
    /// `CFont::SetProportional(bool)` (0x7195B0)
    fn font_set_proportional(v: u32) = cdecl 0x7195B0 => crate::font::set_proportional;
    /// `CFont::SetFontStyle(eFontStyle)` (0x719490)
    fn font_set_font_style(v: u32) = cdecl 0x719490 => crate::font::set_font_style;
    /// `CFont::SetEdge(int8)` (0x719590)
    fn font_set_edge(v: u32) = cdecl 0x719590 => crate::font::set_edge;
    /// `CFont::SetDropColor(CRGBA)` (0x719510)
    fn font_set_drop_color(c: u32) = cdecl 0x719510 => crate::font::set_drop_color;
    /// `CFont::SetColor(CRGBA)` (0x719430)
    fn font_set_color(c: u32) = cdecl 0x719430 => crate::font::set_color;
    /// `CFont::SetScale(float, float)` (0x719380)
    fn font_set_scale(sx: u32, sy: u32) = cdecl 0x719380 => crate::font::set_scale;
    /// `CFont::PrintString(float x, float y, const GxtChar*)` (0x71A700)
    fn font_print_string(px: f32, py: f32, text: GPtr<u8>) = cdecl 0x71A700 => crate::font_print::print_string;
    /// `CFont::DrawFonts()` (0x71A210)
    fn font_draw_fonts() = cdecl 0x71A210 => crate::font_print::draw_fonts;
    /// `CRGBA::CRGBA(uint8, uint8, uint8, uint8)` (0x7170C0)
    fn rgba_ctor(this: GPtr<u8>, r: u32, gg: u32, bb: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CText::Get(const char*)` (0x6A0050)
    fn text_get(this: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => crate::text::text_get;
    /// `CCamera::DrawBordersForWideScreen()` (0x514860)
    fn camera_draw_borders_for_wide_screen(this: GPtr<u8>) = thiscall 0x514860 => unported;
    /// `CCredits::RenderCredits()` (0x5A87F0)
    fn render_credits_call() = cdecl 0x5A87F0 => render_credits;
    /// `CCredits::PrintCreditText(float, float, const GxtChar*, uint32&, float, bool)` (0x5A8660)
    fn print_credit_text_call(sx: f32, sy: f32, text: GPtr<u8>, offset: GPtr<u32>, scroll: f32, hi: u32) = cdecl 0x5A8660 => print_credit_text;
}

/// A CRGBA built by the original constructor in a local, as a dword.
fn rgba(r: u32, gg: u32, bb: u32, a: u32) -> u32 {
    let mut c = [0u8; 4];
    let p = rgba_ctor(GPtr::from_raw(c.as_mut_ptr()), r, gg, bb, a);
    p.cast::<u32>().read()
}

/// `CCredits::Render()` (0x53D5B0): `RenderCredits` (a tail jump) while the credits run and the menu is
/// not active.
#[hook(addr = 0x53D5B0, conv = cdecl, name = "CCredits::Render", expect = "A0 7C E9 C6 00")]
pub fn render() {
    if g::<u8>(CREDITS_GOING).read() != 0 && g::<u8>(MENU_ACTIVE).read() == 0 {
        render_credits_call();
    }
}

/// `CCredits::PrintCreditText(float scaleX, float scaleY, const GxtChar* text, uint32& offset, float
/// scroll, bool highlighted)` (0x5A8660): `pos = float(offset) + height + 20 - scroll` (x87, `offset`
/// unsigned); when `20 < pos` (the register value) and `pos < height - 20` (the stored f32): scale,
/// black text at `(width / 2, pos)`, then white (highlighted) or (210, 210, 210, 210) at
/// `(width / 2 - 1, pos - 1)`. Always `offset = _ftol(scaleY * 20 + float(offset))`.
///
/// The original keeps `pos` and the CRGBAs in its own argument slots (`offset` pointer and `scroll`):
/// only the caller's outgoing argument area differs (no caller reads it back).
#[hook(addr = 0x5A8660, conv = cdecl, name = "CCredits::PrintCreditText", expect = "DB 05 48 70 C1 00")]
pub fn print_credit_text(sx: f32, sy: f32, text: GPtr<u8>, offset: GPtr<u32>, scroll: f32, hi: u32) {
    let h = xi(g::<i32>(RS_MAX_HEIGHT).read());
    let p = fild_u32(offset.read()).fadd(h).fadd_m(k(K_20)).fsub_m(scroll);
    let pos = f32_of(p);
    // `fcomp 20; test ah, 0x41; jne skip`, then `fld 20; fsubr st, st(1)` (height - 20), `fld pos;
    // fcompp; test ah, 5; jp skip`
    if !p.fcom(x(k(K_20))).test_any(0x41) && !x(pos).fcom(h.fsub_m(k(K_20))).test_parity(0x05) {
        font_set_scale(sx.to_bits(), sy.to_bits());
        font_set_color(rgba(0, 0, 0, 0xFF));
        let half = g::<i32>(RS_MAX_WIDTH).read() / 2;
        font_print_string(f32_of(xi(half)), pos, text);
        let c = if hi as u8 != 0 { rgba(0xFF, 0xFF, 0xFF, 0xFF) } else { rgba(0xD2, 0xD2, 0xD2, 0xD2) };
        font_set_color(c);
        let y = f32_of(x(pos).fsub_m(k(K_1)));
        let half = g::<i32>(RS_MAX_WIDTH).read() / 2;
        font_print_string(f32_of(xi(half - 1)), y, text);
    }
    let v = x(sy).fmul_m(k(K_20)).fadd(fild_u32(offset.read()));
    offset.write(ftol32(v) as u32);
}

/// `CCredits::PrintCreditSpace(float size, uint32& offset)` (0x5A87C0): `offset = _ftol(size * 25 +
/// float(offset))` (unsigned offset). `RenderCredits` inlines it.
#[hook(addr = 0x5A87C0, conv = cdecl, name = "CCredits::PrintCreditSpace", expect = "D9 44 24 04 56")]
pub fn print_credit_space(size: f32, offset: GPtr<u32>) {
    let v = x(size).fmul_m(k(K_25)).fadd(fild_u32(offset.read()));
    offset.write(ftol32(v) as u32);
}

/// `CCredits::RenderCredits()` (0x5A87F0): 2D state; scroll = `float(now - start) / 23` (unsigned
/// difference, times the f32 1/23); font: no background, centre size `width * 0.92`, centred,
/// proportional, style 1, no edge, drop colour (0, 0, 0, 255); then the [`CREDITS`] sequence;
/// `DrawFonts`, the wide-screen borders; credits stop once `float(offset + height) - scroll < -10`.
#[hook(addr = 0x5A87F0, conv = cdecl, name = "CCredits::RenderCredits", expect = "83 EC 0C 56 E8 57 BF 18 00")]
pub fn render_credits() {
    defined_state_2d();
    let d = g::<u32>(TIME_MS).read().wrapping_sub(g::<u32>(CREDITS_START_TIME).read());
    let scroll = f32_of(fild_u32(d).fmul_m(k(K_SCROLL)));
    let mut offset = [0u32; 1];
    let op = GPtr::from_raw(offset.as_mut_ptr());
    font_set_background(0, 0);
    font_set_centre_size(f32_of(xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(k(K_CENTRE))).to_bits());
    font_set_orientation(0);
    font_set_proportional(1);
    font_set_font_style(1);
    font_set_edge(0);
    font_set_drop_color(rgba(0, 0, 0, 0xFF));
    for s in CREDITS {
        match s {
            CreditOp::Text { key, scale, hi } => {
                let t = text_get(g(THE_TEXT), g(key as usize));
                let sc = k(scale);
                print_credit_text_call(sc, sc, t, op, scroll, u32::from(hi));
            }
            CreditOp::Space { add } => {
                op.write(ftol32(fild_u32(op.read()).fadd_m(k(add))) as u32);
            }
        }
    }
    font_draw_fonts();
    if g::<u8>(WIDE_SCREEN_ON).read() != 0 {
        camera_draw_borders_for_wide_screen(g(THE_CAMERA));
    }
    let end = (op.read() as i32).wrapping_add(g::<i32>(RS_MAX_HEIGHT).read()) as u32;
    // `fcomp -10; test ah, 5; jp skip`
    if !fild_u32(end).fsub_m(scroll).fcom(x(k(K_END))).test_parity(0x05) {
        g::<u8>(CREDITS_GOING).write(0);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER, HOOK_PRINT_CREDIT_TEXT, HOOK_PRINT_CREDIT_SPACE, HOOK_RENDER_CREDITS];
