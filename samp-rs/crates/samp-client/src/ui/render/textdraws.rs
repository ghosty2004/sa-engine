//! Textdraws, drawn through the game's `CFont` right after `Render2dStuff`: text textdraws make samp's `CFont`
//! calls in its order ([`UiBackend::textdraw_text`]); sprite textdraws (font 4, `txd:name`) and model previews
//! (font 5) draw the backend's texture, previews falling back to the text path while they have none.
//! Spec: docs/re/subsystems/ui-rendering.md, "14. Textdraws"; addresses in docs/re/addresses/ui-textdraws.md.

use sa_samp_api::ui::{TextdrawFont, UiBackend};
use samp_ui::textdraw::{self, FontParams, TextDraws};

/// The backend's `CFont` state for samp's parameters.
fn font(p: &FontParams) -> TextdrawFont {
    TextdrawFont {
        scale: p.scale,
        colour: p.colour,
        orientation: p.orientation,
        wrap_x: p.wrap_x,
        centre_size: p.centre_size,
        background: p.background,
        background_colour: p.background_colour,
        proportional: p.proportional,
        drop_colour: p.drop_colour,
        outline: p.outline,
        shadow: p.shadow,
        font: p.font,
        x: p.x,
        y: p.y,
    }
}

/// Draws every present textdraw in id order and stores the hit rectangles.
pub fn draw_all(b: &mut dyn UiBackend, pool: &mut TextDraws) {
    let (w, h) = b.video_size();
    let (ux, uy) = b.textdraw_units();
    let hover = pool.selection.hover_colour;
    let ids = pool.ids();
    let live: Vec<(u16, Vec<u8>)> = pool
        .iter()
        .filter(|(_, t)| t.data.style == textdraw::FONT_SPRITE)
        .map(|(id, t)| (id, t.text.clone()))
        .collect();
    let live: Vec<(u16, &[u8])> = live.iter().map(|(id, t)| (*id, t.as_slice())).collect();
    b.textdraw_sync(&live);
    for id in ids {
        let Some(t) = pool.get_mut(id) else { continue };
        let style = t.data.style;
        if style == textdraw::FONT_SPRITE || style == textdraw::FONT_PREVIEW {
            let r = textdraw::sprite_rect(&t.data, w, h, ux, uy);
            let colour = if t.hovered { hover } else { t.data.letter_colour };
            let drawn = b.textdraw_sprite(id, style, &t.text, t.serial, r, colour);
            // The sprite path stores its hit rect also without a texture; a preview without one takes the text
            // path. Mirrors samp.dll 0x100B30D0.
            if drawn || style == textdraw::FONT_SPRITE {
                t.hit = Some(textdraw::sprite_hit_rect(&t.data, w, h, ux, uy));
                continue;
            }
        }
        if t.text.is_empty() {
            continue;
        }
        let p = textdraw::font_params(&t.data, t.hovered.then_some(hover), w, h, ux, uy);
        b.textdraw_text(&font(&p), &t.text, t.has_keys);
        t.hit = Some(textdraw::hit_rect(&p));
    }
}
