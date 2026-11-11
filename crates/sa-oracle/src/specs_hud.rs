//! L1 specs for PORT-HUD (`sa_hud`, see `docs/arch/hud-port.md`): fonts, HUD and menu leaves
//! (`CFont::FindSubFontCharacter`).

use crate::call::Conv::*;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::abi::{PRESERVE_ECX, PRESERVE_EDX};
use sa_hud::font;

/// A character byte (upper bytes random: the original reads only the low byte).
fn g_char(r: &mut Rng) -> u32 {
    (r.next_u32() & !0xFF) | (r.next_u32() & 0xFF)
}

/// A style byte: 0, 1 or 2 mostly (upper bytes random).
fn g_style(r: &mut Rng) -> u32 {
    let lo = if r.chance(80) {
        r.below(3)
    } else {
        r.next_u32() & 0xFF
    };
    (r.next_u32() & !0xFF) | lo
}

pub fn all() -> Vec<Spec> {
    vec![
        b(
            "HudFindSubFontCharacter",
            "CFont::FindSubFontCharacter",
            0x7192C0,
            Cdecl,
            Al,
        )
        .args(vec![A::I(g_char), A::I(g_style)])
        .preserve(PRESERVE_ECX | PRESERVE_EDX)
        .rust(|c| ExpRet::Al(font::find_sub_font_character(c.i(0) as u8, c.i(1) as u8))),
    ]
}
