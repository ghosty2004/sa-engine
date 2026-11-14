//! World-anchored drawing: nametag bars and names, chat bubbles, 3D text labels and the `/dl` vehicle labels.

use sa_samp_api::ui::{ColourQuad, UiBackend, UiFont, UiRect, UiSprite, dt};
use samp_ui::colour;
use samp_ui::fonts;
use samp_ui::nametags;

use super::{BubbleDraw, LabelDraw, SPRITE_ALPHABLEND, TagDraw, outlined};

/// Screen point of a tag: `(x, y, z + 0.0475·d + 0.2)`, truncated; `None` when the projected z is above 1.
fn tag_point(b: &dyn UiBackend, t: &TagDraw) -> Option<(i32, i32)> {
    let p = [t.anchor[0], t.anchor[1], t.anchor[2] + t.distance * nametags::RAISE_PER_METRE + nametags::RAISE_BASE];
    let s = b.project(p);
    (s[2] <= 1.0).then_some((s[0] as i32, s[1] as i32))
}

/// Nametag health / armour bars pass (ui-rendering.md, "Bars"). Mirrors samp.dll 0x10074270.
pub fn tag_bars(b: &mut dyn UiBackend, tags: &[TagDraw]) {
    b.bars_begin();
    for t in tags {
        let Some((x, y)) = tag_point(b, t) else { continue };
        for group in nametags::bars(x, y, t.health, t.armour) {
            b.bars_group(&group.map(|q| ColourQuad { x0: q.x0, y0: q.y0, x1: q.x1, y1: q.y1, colour: q.colour }));
        }
    }
    b.bars_end();
}

/// Nametag names pass, including the AFK icon (ui-rendering.md, "Name text"). Mirrors samp.dll 0x10075390.
pub fn tag_names(b: &mut dyn UiBackend, tags: &[TagDraw], nametag_status: bool) {
    // The SAMPAUX3 fonts are created on the first frame with a nametag, like samp.
    let aux = b.aux_fonts(!tags.is_empty());
    b.sprite_begin(UiSprite::Tags, SPRITE_ALPHABLEND);
    for t in tags {
        let Some((x, y)) = tag_point(b, t) else { continue };
        let m = b.measure(UiFont::Small, &t.name, -1, UiRect::default(), dt::CALCRECT, 0xFF00_0000);
        let (w, h) = (m.right - m.left, m.bottom - m.top);
        let r = nametags::name_rect(x, y, w, h);
        let rect = UiRect::new(r[0], r[1], r[2], r[3]);
        outlined(b, UiSprite::Tags, UiFont::Small, UiFont::SmallOutline, &t.name, rect, 0, t.colour, &fonts::OUTLINE_4);
        if aux {
            let flags = dt::CALCRECT | dt::NOCLIP | dt::CENTER | dt::VCENTER;
            let c = b.measure(UiFont::Aux22, b"C", 1, UiRect::default(), flags, 0);
            if t.afk && nametag_status {
                let (cr, er) = nametags::afk_rects(r[0], y, [c.left, c.top, c.right, c.bottom]);
                let flags = dt::CENTER | dt::VCENTER | dt::NOCLIP;
                let mut rc = UiRect::new(cr[0], cr[1], cr[2], cr[3]);
                b.draw_text(UiFont::Aux22, None, b"C", 1, &mut rc, flags, nametags::AFK_BACKDROP);
                let mut re = UiRect::new(er[0], er[1], er[2], er[3]);
                b.draw_text(UiFont::Aux20, None, b"E", 1, &mut re, flags, nametags::AFK_ICON);
            }
        }
    }
    b.sprite_end(UiSprite::Tags);
}

/// The 3D text painter shared by labels and chat bubbles: project, skip when z > 1, truncate to a rect
/// [x, y, x+1, y+1], then the font renderer's small draw with format 0x101: when `outline` is set, four black
/// passes of the stripped text (offsets move only left / top), then the main pass of the raw text through the
/// colour-code wrapper. Mirrors samp.dll 0x1006BC90.
fn text3d(b: &mut dyn UiBackend, pos: [f32; 3], text: &[u8], argb: u32, outline: bool) {
    let p = b.project(pos);
    if p[2] > 1.0 {
        return;
    }
    let (x, y) = (p[0] as i32, p[1] as i32);
    let rect = UiRect::new(x, y, x + 1, y + 1);
    let passes: &[(i32, i32)] = if outline { &fonts::OUTLINE_4 } else { &[] };
    outlined(b, UiSprite::Text3d, UiFont::Small, UiFont::SmallOutline, text, rect, dt::CENTER, argb, passes);
}

/// Chat bubbles (ui-rendering.md, "Chat bubbles"): the 3D text painter with the outline always on and no
/// line-of-sight test. Mirrors samp.dll 0x10066ED0.
pub fn bubbles(b: &mut dyn UiBackend, bubbles: &[BubbleDraw]) {
    b.sprite_begin(UiSprite::Text3d, SPRITE_ALPHABLEND);
    for bb in bubbles {
        text3d(b, bb.anchor, &bb.text, bb.colour, true);
    }
    b.sprite_end(UiSprite::Text3d);
}

/// 3D text labels: one batch on the 3D-text sprite through the 3D text painter. The outline is on only when
/// the label colour's alpha is FF; the text keeps its `{RRGGBB}` codes for the colour wrapper (ui-rendering.md,
/// "Smaller overlays"). Mirrors samp.dll 0x10001350.
pub fn labels(b: &mut dyn UiBackend, labels: &[LabelDraw]) {
    b.sprite_begin(UiSprite::Text3d, SPRITE_ALPHABLEND);
    for l in labels {
        text3d(b, l.pos, &l.text, l.argb, l.argb & 0xFF00_0000 == 0xFF00_0000);
    }
    b.sprite_end(UiSprite::Text3d);
}

/// `/dl` vehicle labels on the 3D-text painter with the outline on: four black outline draws of the stripped
/// text, then the main draw in [`crate::vehlabels::LABEL_ARGB`] through the colour wrapper.
pub fn vehicle_labels(b: &mut dyn UiBackend, rows: &[([f32; 3], String)]) {
    b.sprite_begin(UiSprite::Text3d, SPRITE_ALPHABLEND);
    for (pos, text) in rows {
        let p = b.project(*pos);
        if p[2] > 1.0 {
            continue;
        }
        let (x, y) = (p[0] as i32, p[1] as i32);
        let text = colour::strip_codes(text.as_bytes());
        let n = text.len() as i32;
        for (dx, dy) in fonts::OUTLINE_4 {
            let mut r = UiRect::new(x + dx, y + dy, x + 1, y + 1);
            b.draw_text(UiFont::Small, Some(UiSprite::Text3d), &text, n, &mut r, dt::CENTER | dt::NOCLIP, colour::OUTLINE);
        }
        let mut r = UiRect::new(x, y, x + 1, y + 1);
        let argb = crate::vehlabels::LABEL_ARGB;
        let glyphs: Vec<u32> = colour::glyph_colours(&text, argb).iter().map(|g| g.colour).collect();
        b.draw_text_coloured(UiFont::Small, UiSprite::Text3d, &text, &mut r, dt::CENTER | dt::NOCLIP, argb, &glyphs);
    }
    b.sprite_end(UiSprite::Text3d);
}
