//! The DXUT controls samp's GUI dialogs draw (caption, button, scrollbar, edit box, list box), from samp-ui's
//! skin rects and control geometry. Spec: docs/re/subsystems/ui-rendering.md, "4. GUI layer (DXUT-derived)".

use sa_samp_api::ui::{UiBackend, UiFont, UiRect};
use samp_ui::dialog;
use samp_ui::dxut::{self, ScrollBar, colours, fmt, skin};

use super::ui_rect;

/// Caption bar and text (`CDXUTDialog::OnRender` with a caption).
pub fn caption(b: &mut dyn UiBackend, rect: samp_ui::Rect, caption_h: i32, title: &[u8]) {
    let (bar, text) = dxut::caption_rects(rect, caption_h);
    b.gui_sprite(ui_rect(skin::CAPTION), ui_rect(bar), colours::CAPTION_TEX);
    b.gui_text(UiFont::Gui1, title, ui_rect(text), fmt::CAPTION, colours::CAPTION_TEXT);
}

/// `CDXUTButton::Render` (face layer + text, fill layer + text).
pub fn button(b: &mut dyn UiBackend, label: &[u8], rect: samp_ui::Rect, mouse_over: bool) {
    let l = dxut::button_look(rect, mouse_over);
    let r = ui_rect(l.rect);
    b.gui_sprite(ui_rect(skin::BUTTON), r, l.face);
    b.gui_text(UiFont::Gui0, label, r, fmt::BUTTON, l.text);
    b.gui_sprite(ui_rect(skin::BUTTON), r, l.fill);
    b.gui_text(UiFont::Gui0, label, r, fmt::BUTTON, l.text);
}

/// `CDXUTScrollBar::Render`: track, up, down, thumb.
pub fn scrollbar(b: &mut dyn UiBackend, sb: &ScrollBar) {
    let p = sb.parts();
    b.gui_sprite(ui_rect(skin::SCROLL_TRACK), ui_rect(p.track), colours::WHITE);
    b.gui_sprite(ui_rect(skin::SCROLL_UP), ui_rect(p.up), colours::WHITE);
    b.gui_sprite(ui_rect(skin::SCROLL_DOWN), ui_rect(p.down), colours::WHITE);
    let c = if p.scrolls { colours::THUMB } else { colours::THUMB_DISABLED };
    b.gui_sprite(ui_rect(skin::SCROLL_THUMB), ui_rect(p.thumb), c);
}

/// The edit box's text as drawn plus caret / selection state.
pub struct EditView<'a> {
    pub text: &'a [u16],
    pub caret: usize,
    pub anchor: usize,
    pub first_visible: usize,
    pub focused: bool,
    pub insert_mode: bool,
}

/// `CDXUTEditBox::Render` for the box `rect` (border 5, spacing 4), GUI font 0.
pub fn edit_box(b: &mut dyn UiBackend, rect: samp_ui::Rect, e: &EditView) {
    let (dst, area) = dxut::edit_rects(rect);
    for (i, (src, d)) in skin::EDIT.iter().zip(dst).enumerate() {
        let c = if i == 0 { colours::EDIT_FRAME } else { colours::EDIT_BORDER };
        b.gui_sprite(ui_rect(*src), ui_rect(d), c);
    }
    let font = UiFont::Gui0;
    let x_of = |b: &dyn UiBackend, cp: usize| b.text_width_wide(font, &e.text[..cp.min(e.text.len())]);
    let x_first = x_of(b, e.first_visible);
    let caret_x = x_of(b, e.caret);
    let mut sel_rect = None;
    if e.caret != e.anchor {
        let sel_x = x_of(b, e.anchor);
        let (lo, hi) = if caret_x <= sel_x { (caret_x, sel_x) } else { (sel_x, caret_x) };
        let mut r = UiRect::new(lo + area.left - x_first, area.top, hi + area.left - x_first, area.bottom);
        r.left = r.left.max(area.left);
        r.right = r.right.min(area.right);
        if r.right > r.left {
            b.gui_rect(r, colours::SELECTION_BG);
        }
        sel_rect = Some(r);
    }
    let first = e.first_visible.min(e.text.len());
    b.gui_text_wide(font, &e.text[first..], -1, ui_rect(area), fmt::EDIT, colours::EDIT_TEXT);
    if let Some(r) = sel_rect {
        let lo = e.caret.min(e.anchor).max(first);
        let hi = e.caret.max(e.anchor);
        if hi > lo {
            b.gui_text_wide(font, &e.text[lo..hi], (hi - lo) as i32, r, fmt::EDIT, colours::SELECTED_TEXT);
        }
    }
    if e.focused {
        // The caret does not blink (same as samp).
        let cx = area.left - x_first + caret_x;
        let mut r = UiRect::new(cx - 1, area.top, cx + 1, area.bottom);
        if !e.insert_mode {
            r.right = area.left - x_first + x_of(b, e.caret + 1);
        }
        b.gui_rect(r, colours::CARET);
    }
}

/// List box (DXUT `CDXUTListBox::Render` with samp's tablist columns): background, selection bar + rows from
/// the scroll position, then the scrollbar. Each row is `(text colour, cells)`; `columns` cells are drawn at the
/// column offsets.
pub fn list_box(
    b: &mut dyn UiBackend,
    list: samp_ui::Rect,
    rows: &[(u32, Vec<Vec<u8>>)],
    columns: usize,
    widths: &[i32],
    selected: Option<usize>,
    top_row: usize,
) {
    b.gui_sprite(ui_rect(skin::LIST), ui_rect(list), colours::LIST_TEX);
    let sel = dialog::list_selection_rect(&list);
    let text = dialog::list_text_rect(&list);
    let row_h = b.metrics().gui1_h;
    let page = if row_h > 0 { (text.height() / row_h).max(1) } else { 1 };
    if !rows.is_empty() {
        let mut top = text.top;
        for (i, (colour, cells)) in rows.iter().enumerate().skip(top_row) {
            if top + row_h > text.bottom {
                break;
            }
            if Some(i) == selected {
                let r = UiRect::new(sel.left, top, sel.right, top + row_h);
                b.gui_sprite(ui_rect(skin::LIST_SELECTION), r, dialog::SELECTION_COLOUR);
            }
            // DXUT draws the selected row with the selection element (white text).
            let c = if Some(i) == selected { dialog::ROW_COLOUR } else { *colour };
            let mut x = text.left;
            for k in 0..columns.max(1) {
                let cell = cells.get(k).map(Vec::as_slice).unwrap_or(b"");
                b.gui_text(UiFont::Gui1, cell, UiRect::new(x, top, text.right, top + row_h), fmt::LIST, c);
                x += widths.get(k).copied().unwrap_or(0);
            }
            top += row_h;
        }
    }
    let sb = ScrollBar {
        rect: samp_ui::Rect::new(list.right - dialog::LIST_SCROLLBAR_W, list.top, list.right, list.bottom),
        start: 0,
        end: rows.len() as i32,
        pos: top_row as i32,
        page,
    };
    scrollbar(b, &sb);
}
