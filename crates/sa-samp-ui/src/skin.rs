//! samp's DXUT GUI look: controls drawn from its skin texture `sampgui.png` (512 × 512) with samp's element
//! rectangles and colours (samp.dll 0x1008d7e0; samp-rs `ui-rendering.md` §4.1), and the cursor from
//! `mouse.png` (32 × 32, hotspot 0, 0). Both files belong to the user's SA:MP install and are loaded at run
//! time by the host (never committed); the renderer holds them in one 512 × 1024 texture: `sampgui.png` at
//! the top, `mouse.png` at (0, 512). Without them ([`DrawList::skin`] false) the controls are drawn with flat
//! rectangles of the same colours.

use crate::colour::strip_codes;
use crate::draw::{Cmd, DrawList};
use crate::font::{FontId, Measure};

/// Width of the combined skin texture.
pub const TEXTURE_W: u32 = 512;
/// Height of the combined skin texture.
pub const TEXTURE_H: u32 = 1024;
/// Where `mouse.png` sits in the combined texture.
pub const CURSOR_SRC: [f32; 4] = [0.0, 512.0, 32.0, 544.0];

/// `sampgui.png` element rectangles `[left, top, right, bottom]`.
pub mod src {
    /// Caption bar.
    pub const CAPTION: [f32; 4] = [17.0, 269.0, 241.0, 287.0];
    /// Button face and fill.
    pub const BUTTON: [f32; 4] = [0.0, 0.0, 136.0, 54.0];
    /// List box.
    pub const LIST: [f32; 4] = [13.0, 124.0, 241.0, 265.0];
    /// List selection.
    pub const LIST_SELECTION: [f32; 4] = [17.0, 269.0, 241.0, 287.0];
    /// Scrollbar track.
    pub const SCROLL_TRACK: [f32; 4] = [243.0, 144.0, 265.0, 155.0];
    /// Scrollbar up arrow.
    pub const SCROLL_UP: [f32; 4] = [243.0, 124.0, 265.0, 144.0];
    /// Scrollbar down arrow.
    pub const SCROLL_DOWN: [f32; 4] = [243.0, 155.0, 265.0, 176.0];
    /// Scrollbar thumb.
    pub const SCROLL_THUMB: [f32; 4] = [266.0, 123.0, 286.0, 167.0];
    /// Edit box: text area, then the frame pieces TL, T, TR, L, R, BL, B, BR.
    pub const EDIT: [[f32; 4]; 9] = [
        [14.0, 90.0, 241.0, 113.0],
        [8.0, 82.0, 14.0, 90.0],
        [14.0, 82.0, 241.0, 90.0],
        [241.0, 82.0, 246.0, 90.0],
        [8.0, 90.0, 14.0, 113.0],
        [241.0, 90.0, 246.0, 113.0],
        [8.0, 113.0, 14.0, 121.0],
        [14.0, 113.0, 241.0, 121.0],
        [241.0, 113.0, 246.0, 121.0],
    ];
}

/// samp's skin colours (`0xAARRGGBB`).
pub mod colour {
    /// Caption texture tint.
    pub const CAPTION_TEX: u32 = 0xC80A_0A0A;
    /// Caption text.
    pub const CAPTION_TEXT: u32 = 0xFFD2_D2D2;
    /// Button face tint.
    pub const BUTTON_TEX: u32 = 0xC8FF_FFFF;
    /// Button face tint under the mouse.
    pub const BUTTON_TEX_OVER: u32 = 0xFFB9_2228;
    /// Button text.
    pub const BUTTON_TEXT: u32 = 0xFFC8_C8C8;
    /// Button text under the mouse.
    pub const BUTTON_TEXT_OVER: u32 = 0xFFFF_FFFF;
    /// List box tint.
    pub const LIST_TEX: u32 = 0xC8FF_FFFF;
    /// Scrollbar parts.
    pub const WHITE: u32 = 0xFFFF_FFFF;
    /// Scrollbar thumb.
    pub const THUMB: u32 = 0xFFB9_2228;
    /// Edit box text.
    pub const EDIT_TEXT: u32 = 0xFFFF_FFFF;
    /// Edit box text area.
    pub const EDIT_FRAME: u32 = 0xF005_0505;
    /// Edit box border.
    pub const EDIT_BORDER: u32 = 0xFFFF_FFFF;
    /// Caret.
    pub const CARET: u32 = 0xFF96_9696;
    /// List selection.
    pub const SELECTION: u32 = 0xFFB9_2228;
}

/// List box inner border (DXUT `m_nBorder`).
pub const LIST_BORDER: f32 = 6.0;
/// List box text margin (DXUT `m_nMargin`).
pub const LIST_MARGIN: f32 = 5.0;
/// List box / dialog scrollbar width.
pub const SCROLLBAR_W: f32 = 16.0;
/// Edit box border and spacing (DXUT `m_nBorder`, `m_nSpacing`).
pub const EDIT_BORDER: f32 = 5.0;
/// Edit box spacing.
pub const EDIT_SPACING: f32 = 4.0;

impl DrawList {
    /// `CDXUTDialog::DrawSprite`: the skin rectangle `src` scaled into `dst`, tinted (alpha 0 draws nothing).
    pub fn skin(&mut self, src: [f32; 4], dst: [f32; 4], colour: u32) {
        if colour >> 24 == 0 {
            return;
        }
        if self.skin {
            self.cmds.push(Cmd::Skin { src, dst, colour });
        } else {
            self.rect(dst[0], dst[1], dst[2], dst[3], colour);
        }
    }

    /// DXUT text in `r` with `DT_CENTER` / `DT_VCENTER` as asked, `{RRGGBB}` codes colouring (GUI fonts are
    /// samp's colour-code wrapper fonts).
    #[allow(clippy::too_many_arguments)]
    pub fn gui_text(
        &mut self,
        m: &dyn Measure,
        r: [f32; 4],
        font: FontId,
        text: &str,
        colour: u32,
        centre: bool,
        vcentre: bool,
    ) {
        if colour >> 24 == 0 {
            return;
        }
        let w = m.width(font, &strip_codes(text));
        let x = if centre {
            (r[0] + ((r[2] - r[0]) - w) / 2.0).trunc()
        } else {
            r[0]
        };
        let y = if vcentre {
            (r[1] + ((r[3] - r[1]) - m.line_height(font)) / 2.0).trunc()
        } else {
            r[1]
        };
        self.coloured(m, x, y, font, text, colour, &[]);
    }

    /// The dialog background (flat, all four corners one colour).
    pub fn dialog_background(&mut self, r: [f32; 4], colour: u32) {
        self.rect(r[0], r[1], r[2], r[3], colour);
    }

    /// `CDXUTDialog` caption: the bar from the skin, the title in GUI font 1 at `left + 5`, vertically centred.
    pub fn caption(&mut self, m: &dyn Measure, r: [f32; 4], h: f32, title: &str) {
        let bar = [r[0], r[1], r[2], r[1] + h];
        self.skin(src::CAPTION, bar, colour::CAPTION_TEX);
        self.gui_text(
            m,
            [bar[0] + 5.0, bar[1], bar[2], bar[3]],
            FontId::Gui1,
            title,
            colour::CAPTION_TEXT,
            false,
            true,
        );
    }

    /// `CDXUTButton::Render`: face and label, then the fill layer and the label again; under the mouse the
    /// button moves by (−1, −2) and turns red.
    pub fn button(&mut self, m: &dyn Measure, r: [f32; 4], label: &str, over: bool) {
        let (rc, tex, text, fill) = if over {
            (
                [r[0] - 1.0, r[1] - 2.0, r[2] - 1.0, r[3] - 2.0],
                colour::BUTTON_TEX_OVER,
                colour::BUTTON_TEXT_OVER,
                colour::BUTTON_TEX_OVER,
            )
        } else {
            (r, colour::BUTTON_TEX, colour::BUTTON_TEXT, 0x00FF_FFFF)
        };
        self.skin(src::BUTTON, rc, tex);
        self.gui_text(m, rc, FontId::Gui0, label, text, true, true);
        self.skin(src::BUTTON, rc, fill);
        self.gui_text(m, rc, FontId::Gui0, label, text, true, true);
    }

    /// `CDXUTScrollBar::Render` in `r`: track, arrows, thumb (`thumb` = its top and bottom when the content
    /// scrolls; otherwise the thumb has no height).
    pub fn scrollbar(&mut self, r: [f32; 4], thumb: Option<(f32, f32)>) {
        let w = r[2] - r[0];
        let up = [r[0], r[1], r[2], r[1] + w];
        let down = [r[0], r[3] - w, r[2], r[3]];
        self.skin(src::SCROLL_TRACK, [r[0], up[3], r[2], down[1]], colour::WHITE);
        self.skin(src::SCROLL_UP, up, colour::WHITE);
        self.skin(src::SCROLL_DOWN, down, colour::WHITE);
        if let Some((t, b)) = thumb
            && b > t
        {
            self.skin(src::SCROLL_THUMB, [r[0], t, r[2], b], colour::THUMB);
        }
    }

    /// DXUT scrollbar thumb for `r`: `(top, bottom)` when `count > page`.
    pub fn scroll_thumb(r: [f32; 4], count: usize, page: usize, pos: usize) -> Option<(f32, f32)> {
        if count <= page {
            return None;
        }
        let w = r[2] - r[0];
        let (tt, tb) = (r[1] + w, r[3] - w);
        let th = tb - tt;
        let thumb_h = (th * page as f32 / count as f32).trunc().max(8.0);
        let max_pos = (count - page) as f32;
        let top = tt + (pos.min(count - page) as f32 * (th - thumb_h) / max_pos).trunc();
        Some((top, top + thumb_h))
    }

    /// `CDXUTEditBox::Render` frame: the text area and the eight border pieces. Returns the text rectangle
    /// (`rect` inset by the border and the spacing).
    pub fn edit_frame(&mut self, r: [f32; 4]) -> [f32; 4] {
        let b = EDIT_BORDER;
        let t = [r[0] + b, r[1] + b, r[2] - b, r[3] - b];
        let dst = [
            t,
            [r[0], r[1], t[0], t[1]],
            [t[0], r[1], t[2], t[1]],
            [t[2], r[1], r[2], t[1]],
            [r[0], t[1], t[0], t[3]],
            [t[2], t[1], r[2], t[3]],
            [r[0], t[3], t[0], r[3]],
            [t[0], t[3], t[2], r[3]],
            [t[2], t[3], r[2], r[3]],
        ];
        if self.skin {
            for (i, (s, d)) in src::EDIT.iter().zip(dst).enumerate() {
                self.skin(
                    *s,
                    d,
                    if i == 0 {
                        colour::EDIT_FRAME
                    } else {
                        colour::EDIT_BORDER
                    },
                );
            }
        } else {
            self.rect(r[0], r[1], r[2], r[3], colour::EDIT_FRAME);
            self.frame(r[0] + 2.0, r[1] + 2.0, r[2] - 2.0, r[3] - 2.0, colour::EDIT_BORDER);
        }
        let s = EDIT_SPACING;
        [t[0] + s, t[1] + s, t[2] - s, t[3] - s]
    }

    /// The edit box text (left, vertically centred, white) and the caret (2 px, grey, not blinking), `shift`
    /// pixels scrolled out on the left; `before` is the text before the caret.
    pub fn edit_text(&mut self, m: &dyn Measure, area: [f32; 4], shown: &str, before_w: f32) {
        let font = FontId::Gui0;
        let y = (area[1] + ((area[3] - area[1]) - m.line_height(font)) / 2.0).trunc();
        self.text(area[0], y, font, colour::EDIT_TEXT, shown);
        let cx = area[0] + before_w;
        self.rect(cx - 1.0, area[1], cx + 1.0, area[3], colour::CARET);
    }

    /// The mouse cursor at `(x, y)`: `mouse.png` when the skin is loaded, else a drawn arrow.
    pub fn cursor(&mut self, x: f32, y: f32) {
        if self.skin {
            self.cmds.push(Cmd::Skin {
                src: CURSOR_SRC,
                dst: [x, y, x + 32.0, y + 32.0],
                colour: 0xFFFF_FFFF,
            });
        } else {
            crate::panels::draw_cursor(x, y, self);
        }
    }

    /// `CDXUTListBox::Render` with samp's tab columns: the box, the rows from `top` (selected row on the
    /// selection bar), then the scrollbar. Rows are `(colour, cells)`; `columns` cells per row at the column
    /// offsets `widths`. Returns the number of rows that fit.
    #[allow(clippy::too_many_arguments)]
    pub fn list_box(
        &mut self,
        m: &dyn Measure,
        r: [f32; 4],
        rows: &[(u32, Vec<String>)],
        columns: usize,
        widths: &[f32],
        selected: Option<usize>,
        top: usize,
    ) -> usize {
        self.skin(src::LIST, r, colour::LIST_TEX);
        let sel = list_selection_rect(r);
        let text = [sel[0] + LIST_MARGIN, sel[1], sel[2] - LIST_MARGIN, sel[3]];
        let row_h = m.line_height(FontId::Gui1);
        let page = ((text[3] - text[1]) / row_h).floor().max(1.0) as usize;
        let mut y = text[1];
        for (i, (c, cells)) in rows.iter().enumerate().skip(top) {
            if y + row_h > text[3] {
                break;
            }
            if Some(i) == selected {
                self.skin(
                    src::LIST_SELECTION,
                    [sel[0], y, sel[2], y + row_h],
                    colour::SELECTION,
                );
            }
            let colour = if Some(i) == selected { 0xFFFF_FFFF } else { *c };
            let mut x = text[0];
            for k in 0..columns.max(1) {
                if let Some(cell) = cells.get(k) {
                    self.gui_text(m, [x, y, text[2], y + row_h], FontId::Gui1, cell, colour, false, false);
                }
                x += widths.get(k).copied().unwrap_or(0.0);
            }
            y += row_h;
        }
        let sb = [r[2] - SCROLLBAR_W, r[1], r[2], r[3]];
        self.scrollbar(sb, Self::scroll_thumb(sb, rows.len(), page, top));
        page
    }
}

/// The list box selection area: inside the border, left of the scrollbar.
pub fn list_selection_rect(r: [f32; 4]) -> [f32; 4] {
    [
        r[0] + LIST_BORDER,
        r[1] + LIST_BORDER,
        r[2] - SCROLLBAR_W - LIST_BORDER,
        r[3] - LIST_BORDER,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::MonoMeasure;

    const M: MonoMeasure = MonoMeasure {
        char_w: 8.0,
        line_h: 18.0,
    };

    #[test]
    fn skin_commands_and_flat_fallback() {
        let mut d = DrawList {
            skin: true,
            ..DrawList::default()
        };
        d.button(&M, [10.0, 10.0, 110.0, 40.0], "OK", false);
        // Face (tinted), label, fill (alpha 0: skipped), label again.
        assert_eq!(
            d.cmds[0],
            Cmd::Skin {
                src: src::BUTTON,
                dst: [10.0, 10.0, 110.0, 40.0],
                colour: colour::BUTTON_TEX
            }
        );
        assert_eq!(d.texts(), ["OK", "OK"]);
        let mut over = DrawList {
            skin: true,
            ..DrawList::default()
        };
        over.button(&M, [10.0, 10.0, 110.0, 40.0], "OK", true);
        assert!(matches!(over.cmds[0], Cmd::Skin { dst: [9.0, 8.0, 109.0, 38.0], .. }));
        let mut flat = DrawList::default();
        flat.skin(src::LIST, [0.0, 0.0, 5.0, 5.0], 0xFF00_0000);
        assert!(matches!(flat.cmds[0], Cmd::Rect { .. }));
        let area = DrawList::default().edit_frame([0.0, 0.0, 100.0, 44.0]);
        assert_eq!(area, [9.0, 9.0, 91.0, 35.0]);
        assert_eq!(DrawList::scroll_thumb([0.0, 0.0, 16.0, 216.0], 5, 10, 0), None);
        assert_eq!(
            DrawList::scroll_thumb([0.0, 0.0, 16.0, 216.0], 20, 10, 10),
            Some((108.0, 200.0))
        );
    }
}
