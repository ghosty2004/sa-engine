//! GUI panels: the scoreboard, dialogs and the death window.

use sa_samp_api::ui::{UiBackend, UiFont, UiRect, UiSprite, dt};
use samp_ui::colour;
use samp_ui::deathwindow::{self, DeathWindow};
use samp_ui::dialog::{self, Dialog, DialogLayout, Style};
use samp_ui::fonts;
use samp_ui::scoreboard;

use super::gui::{self, EditView};
use super::text::small_text;
use super::{SPRITE_ALPHABLEND, ScoreboardDraw, ui_rect};

/// Scoreboard: DXUT dialog with the list box, then the texts (ui-rendering.md, "Scoreboard").
pub fn scoreboard(b: &mut dyn UiBackend, sb: &ScoreboardDraw) {
    let (w, h) = b.video_size();
    let small_h = b.metrics().small_h;
    let l = scoreboard::layout(w, h, small_h, 0);
    b.dialog_begin(ui_rect(l.rect), scoreboard::BACKGROUND);
    let cells: Vec<(u32, Vec<Vec<u8>>)> = sb.rows.iter().map(|r| (r.colour, r.cells.to_vec())).collect();
    gui::list_box(b, l.list, &cells, 4, &l.column_widths, sb.selected, sb.scroll);
    b.dialog_end();
    let flags = dt::SINGLELINE;
    let text_rect = ui_rect(l.server_name);
    small_text(b, sb.server_name, text_rect, flags, scoreboard::TITLE_COLOUR);
    // The player-count sample is measured here, after the server name, as samp does.
    let r = b.measure(UiFont::Small, scoreboard::PLAYERS_SAMPLE, -1, UiRect::default(), dt::CALCRECT, 0xFF00_0000);
    let l = scoreboard::layout(w, h, small_h, r.right - r.left);
    small_text(b, &sb.players, UiRect { left: l.players_x, ..text_rect }, flags, scoreboard::TITLE_COLOUR);
    for (hdr, (hx, top)) in scoreboard::HEADERS.iter().zip(l.headers) {
        let r = UiRect::new(hx, top, hx + 46, l.list.top);
        small_text(b, hdr, r, flags, scoreboard::HEADER_COLOUR);
    }
}

/// Dialog: DXUT dialog (caption, buttons, list / edit box focused last), then the body or the style-5
/// headers through the font renderer (ui-rendering.md, "Dialogs", "Per-frame draw").
pub fn dialog(b: &mut dyn UiBackend, d: &Dialog, l: &DialogLayout, cursor: (i32, i32)) {
    b.dialog_begin(ui_rect(l.rect), dialog::BACKGROUND);
    if l.caption_h > 0 {
        gui::caption(b, l.rect, l.caption_h, &d.title);
    }
    let over = |r: samp_ui::Rect| r.contains(cursor.0, cursor.1);
    gui::button(b, &d.button1, l.button1, over(l.button1));
    if let Some(b2) = l.button2 {
        gui::button(b, &d.button2, b2, over(b2));
    }
    if let Some(list) = l.list {
        let rows: Vec<(u32, Vec<Vec<u8>>)> = d.rows().iter().map(|r| (dialog::ROW_COLOUR, r.clone())).collect();
        let cols = if d.style == Style::List { 1 } else { d.columns() };
        gui::list_box(b, list, &rows, cols, &l.columns, d.selected(), d.top_row());
    }
    if let Some(e) = l.edit {
        let wide = d.edit.display_wide(|t| b.to_wide(t));
        let fv = samp_ui::edit::place_caret(0, d.edit.caret(), wide.len(), samp_ui::dxut::edit_text_width(e), &|cp| {
            b.text_width_wide(UiFont::Gui0, &wide[..cp])
        });
        let view = EditView {
            text: &wide,
            caret: d.edit.caret(),
            anchor: d.edit.anchor(),
            first_visible: fv,
            focused: true,
            insert_mode: d.edit.insert_mode(),
        };
        gui::edit_box(b, e, &view);
    }
    b.dialog_end();
    if let Some(body) = &l.body {
        let r = UiRect::new(body.x, body.y, l.rect.right, l.rect.bottom);
        small_text(b, &body.text, r, dt::EXPANDTABS, body.colour);
    }
    for hdr in &l.headers {
        let r = UiRect::new(hdr.x, hdr.y, l.rect.right, l.rect.bottom);
        small_text(b, &hdr.text, r, dt::SINGLELINE, hdr.colour);
    }
}

/// Death window: 5 slots from the origin, newest at the bottom (ui-rendering.md, "Death window").
/// Mirrors samp.dll 0x1006A2E0.
pub fn death_window(b: &mut dyn UiBackend, dw: &DeathWindow) {
    let m = b.metrics();
    let (w, h) = b.video_size();
    let (x, mut y) = deathwindow::origin(w, h, m.nick_w, m.icon_w);
    b.sprite_begin(UiSprite::Death, SPRITE_ALPHABLEND);
    let rows: Vec<_> = dw.rows().collect();
    let empty = deathwindow::ROWS - rows.len().min(deathwindow::ROWS);
    y += empty as i32 * (m.icon_h + 5);
    for r in rows {
        let (left_text, left_colour, right) = if r.killer.is_empty() {
            (&r.victim, r.victim_colour, None)
        } else {
            (&r.killer, r.killer_colour, Some((&r.victim, r.victim_colour)))
        };
        if left_text.is_empty() {
            y += m.icon_h + 5;
            continue;
        }
        let mut mr = UiRect::default();
        let mc = if r.killer.is_empty() { 0xFF00_0000 } else { 0xFFFF_FFFF };
        b.draw_text(UiFont::DeathNames, None, left_text, -1, &mut mr, dt::CALCRECT, mc);
        let lw = mr.right - mr.left;
        death_name(b, left_text, UiRect::new(x + m.nick_w - lw, y, x + 60, y + 30), left_colour);
        let il = x + m.nick_w + 3;
        death_icon(b, UiRect::new(il, y, il + deathwindow::ICON_BOX_W, y + 30), r.reason);
        if let Some((vt, vc)) = right {
            let vl = il + m.icon_w;
            death_name(b, vt, UiRect::new(vl, y, vl + 35, y + 30), vc);
        }
        y += m.icon_h + 5;
    }
    b.sprite_end(UiSprite::Death);
}

/// Death window name with the 4-way outline in the same font. Mirrors samp.dll 0x10069C40.
fn death_name(b: &mut dyn UiBackend, text: &[u8], rect: UiRect, colour: u32) {
    let s = Some(UiSprite::Death);
    for &(dx, dy) in &fonts::OUTLINE_4 {
        let mut r = UiRect { left: rect.left + dx, top: rect.top + dy, ..rect };
        b.draw_text(UiFont::DeathNames, s, text, -1, &mut r, dt::NOCLIP, colour::OUTLINE);
    }
    let mut r = rect;
    b.draw_text(UiFont::DeathNames, s, text, -1, &mut r, dt::NOCLIP, colour);
}

/// Death window icon box: backdrop `G`, its measure into the rect, the weapon glyph.
/// Mirrors samp.dll 0x10069D40.
fn death_icon(b: &mut dyn UiBackend, rect: UiRect, reason: u8) {
    let s = Some(UiSprite::Death);
    let g = [deathwindow::BACKDROP_GLYPH];
    let mut r = UiRect { top: rect.top - 5, ..rect };
    b.draw_text(UiFont::DeathBackdrop, s, &g, -1, &mut r, dt::NOCLIP, deathwindow::BACKDROP_COLOUR);
    b.draw_text(UiFont::DeathBackdrop, s, &g, -1, &mut r, dt::CALCRECT | dt::NOCLIP, deathwindow::BACKDROP_COLOUR);
    let g = [deathwindow::glyph(reason)];
    let c = deathwindow::glyph_colour(reason);
    b.draw_text(UiFont::DeathIcon, s, &g, -1, &mut r, dt::CENTER | dt::VCENTER | dt::NOCLIP, c);
}
