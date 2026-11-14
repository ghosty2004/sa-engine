//! Font-renderer text: the small-font draws and the overlays (audio stream title, F5 network statistics).

use sa_samp_api::ui::{UiBackend, UiFont, UiRect, UiSprite, dt};
use samp_ui::colour;
use samp_ui::fonts;

use super::{SPRITE_ALPHABLEND, outlined};

/// Font-renderer small-font draw without the outline passes, in its own sprite Begin/End.
/// Mirrors samp.dll 0x1006B4E0 with the outline off.
fn small_text_plain(b: &mut dyn UiBackend, text: &[u8], rect: UiRect, flags: u32, colour: u32) {
    b.sprite_begin(UiSprite::Text, SPRITE_ALPHABLEND);
    let stripped = colour::strip_codes(text);
    let glyphs: Vec<u32> = colour::glyph_colours(text, colour).iter().map(|g| g.colour).collect();
    let mut r = rect;
    b.draw_text_coloured(UiFont::Small, UiSprite::Text, &stripped, &mut r, flags | dt::NOCLIP, colour, &glyphs);
    b.sprite_end(UiSprite::Text);
}

/// Audio stream title: small font at (15, H − 20)–(W, H + 10) (ui-rendering.md, "Smaller overlays").
/// Mirrors samp.dll 0x10066AB0.
pub fn audio_title(b: &mut dyn UiBackend, text: &[u8]) {
    let (w, h) = b.video_size();
    let rect = UiRect::new(15, h - 20, w, h + 10);
    small_text_plain(b, text, rect, crate::audio::TITLE_FLAGS, crate::audio::TITLE_COLOUR);
}

/// Font-renderer small-font draw with the 4-way outline, in its own sprite Begin/End.
/// Mirrors samp.dll 0x1006B4E0.
pub fn small_text(b: &mut dyn UiBackend, text: &[u8], rect: UiRect, flags: u32, colour: u32) {
    b.sprite_begin(UiSprite::Text, SPRITE_ALPHABLEND);
    outlined(b, UiSprite::Text, UiFont::Small, UiFont::SmallOutline, text, rect, flags, colour, &fonts::OUTLINE_4);
    b.sprite_end(UiSprite::Text);
}

/// F5 overlay: the whole text and "Y" measured (`DT_CALCRECT`, no sprite), then the title
/// `Client Network Stats` at (10, 10)–(W − 150, 10 + h) in 0xFF8888EE and every `\n`-terminated line below
/// it in white, each through the font renderer with its 4-way outline and its own sprite Begin/End.
/// Mirrors samp.dll 0x10060D70.
pub fn net_stats(b: &mut dyn UiBackend, text: &[u8]) {
    let _ = b.measure(UiFont::Main, &colour::strip_codes(text), -1, UiRect::default(), dt::CALCRECT, 0xFF00_0000);
    let y = b.measure(UiFont::Main, b"Y", -1, UiRect::default(), dt::CALCRECT, 0xFF00_0000);
    let h = y.bottom - y.top;
    let right = b.video_size().0 - 150;
    let line = |b: &mut dyn UiBackend, t: &[u8], top: i32, c: u32| {
        b.sprite_begin(UiSprite::Text, SPRITE_ALPHABLEND);
        let rect = UiRect::new(10, top, right, top + h);
        outlined(b, UiSprite::Text, UiFont::Main, UiFont::MainOutline, t, rect, 0, c, &fonts::OUTLINE_4);
        b.sprite_end(UiSprite::Text);
    };
    line(b, b"Client Network Stats", 10, 0xFF88_88EE);
    let mut top = 10 + h;
    let mut rest = text;
    while let Some(i) = rest.iter().position(|&c| c == b'\n') {
        line(b, &rest[..i], top, 0xFFFF_FFFF);
        top += h;
        rest = &rest[i + 1..];
    }
}
