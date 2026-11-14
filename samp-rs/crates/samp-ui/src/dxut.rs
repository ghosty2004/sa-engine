//! The renderer-independent part of samp's statically linked DXUT GUI: the skin (`sampgui.png`) element
//! rectangles and colours, the format flags of its elements and the geometry of its controls (caption bar,
//! button with its hover look, scrollbar, edit box). Every UI backend draws the controls from these.
//! Spec: docs/re/subsystems/ui-rendering.md, "4. GUI layer (DXUT-derived)", "4.1 Resource manager and skin".

use crate::Rect;

/// `sampgui.png` element rects.
pub mod skin {
    use crate::Rect;
    const fn r(l: i32, t: i32, rr: i32, b: i32) -> Rect {
        Rect::new(l, t, rr, b)
    }
    /// Edit box: text area, then the 8 frame pieces in DXUT element order (TL, T, TR, L, R, BL, B, BR).
    pub const EDIT: [Rect; 9] = [
        r(14, 90, 241, 113),
        r(8, 82, 14, 90),
        r(14, 82, 241, 90),
        r(241, 82, 246, 90),
        r(8, 90, 14, 113),
        r(241, 90, 246, 113),
        r(8, 113, 14, 121),
        r(14, 113, 241, 121),
        r(241, 113, 246, 121),
    ];
    pub const SCROLL_TRACK: Rect = r(243, 144, 265, 155);
    pub const SCROLL_UP: Rect = r(243, 124, 265, 144);
    pub const SCROLL_DOWN: Rect = r(243, 155, 265, 176);
    pub const SCROLL_THUMB: Rect = r(266, 123, 286, 167);
    pub const CAPTION: Rect = r(17, 269, 241, 287);
    pub const BUTTON: Rect = r(0, 0, 136, 54);
    pub const LIST: Rect = r(13, 124, 241, 265);
    pub const LIST_SELECTION: Rect = r(17, 269, 241, 287);
}

/// Colours of samp's skin.
pub mod colours {
    pub const CAPTION_TEX: u32 = 0xC80A_0A0A;
    pub const CAPTION_TEXT: u32 = 0xFFD2_D2D2;
    pub const BUTTON_TEX: u32 = 0xC8FF_FFFF;
    pub const BUTTON_TEX_OVER: u32 = 0xFFB9_2228;
    pub const BUTTON_TEXT: u32 = 0xFFC8_C8C8;
    pub const BUTTON_TEXT_OVER: u32 = 0xFFFF_FFFF;
    /// Fill layer: transparent unless hovered / focused.
    pub const BUTTON_FILL: u32 = 0x00FF_FFFF;
    pub const BUTTON_FILL_OVER: u32 = 0xFFB9_2228;
    pub const LIST_TEX: u32 = 0xC8FF_FFFF;
    pub const WHITE: u32 = 0xFFFF_FFFF;
    pub const THUMB: u32 = 0xFFB9_2228;
    /// Thumb tint while nothing scrolls (disabled state).
    pub const THUMB_DISABLED: u32 = 0xFF95_B0D0;
    pub const EDIT_TEXT: u32 = 0xFFFF_FFFF;
    pub const EDIT_FRAME: u32 = 0xF005_0505;
    pub const EDIT_BORDER: u32 = 0xFFFF_FFFF;
    pub const CARET: u32 = 0xFF96_9696;
    pub const SELECTION_BG: u32 = 0xFFB9_2228;
    pub const SELECTED_TEXT: u32 = 0xFF0A_0A0F;
}

/// Format flags of the elements (D3DX `DT_*`).
pub mod fmt {
    pub const CAPTION: u32 = 0x4;
    pub const BUTTON: u32 = 0x5;
    pub const LIST: u32 = 0x40;
    pub const EDIT: u32 = 0x4;
}

/// Edit box border and spacing (DXUT `m_nBorder`, `m_nSpacing`).
pub const EDIT_BORDER: i32 = 5;
pub const EDIT_SPACING: i32 = 4;

/// The caption bar of a dialog at `rect` and the rectangle its title is drawn in (left + 5).
pub fn caption_rects(rect: Rect, caption_h: i32) -> (Rect, Rect) {
    let bar = Rect::new(rect.left, rect.top, rect.right, rect.top + caption_h);
    (bar, Rect { left: bar.left + 5, ..bar })
}

/// How `CDXUTButton::Render` draws a button: the rectangle (moved by (−1, −2) under the mouse), the face tint,
/// the text colour and the fill-layer tint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonLook {
    pub rect: Rect,
    pub face: u32,
    pub text: u32,
    pub fill: u32,
}

/// The look of a button at `rect`.
pub fn button_look(rect: Rect, mouse_over: bool) -> ButtonLook {
    if mouse_over {
        ButtonLook {
            rect: Rect::new(rect.left - 1, rect.top - 2, rect.right - 1, rect.bottom - 2),
            face: colours::BUTTON_TEX_OVER,
            text: colours::BUTTON_TEXT_OVER,
            fill: colours::BUTTON_FILL_OVER,
        }
    } else {
        ButtonLook { rect, face: colours::BUTTON_TEX, text: colours::BUTTON_TEXT, fill: colours::BUTTON_FILL }
    }
}

/// DXUT scrollbar state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollBar {
    pub rect: Rect,
    pub start: i32,
    pub end: i32,
    pub pos: i32,
    pub page: i32,
}

/// The parts of a scrollbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollParts {
    pub up: Rect,
    pub down: Rect,
    pub track: Rect,
    pub thumb: Rect,
    /// The content scrolls (the thumb is enabled).
    pub scrolls: bool,
}

impl ScrollBar {
    /// Up, down, track and thumb rects (`CDXUTScrollBar::UpdateThumbRect`); the thumb has no height when
    /// nothing scrolls.
    pub fn parts(&self) -> ScrollParts {
        let b = self.rect;
        let w = b.right - b.left;
        let up = Rect { bottom: b.top + w, ..b };
        let down = Rect { top: b.bottom - w, ..b };
        let track = Rect { top: up.bottom, bottom: down.top, ..b };
        let th = track.bottom - track.top;
        if self.end - self.start > self.page {
            let thumb_h = (th * self.page / (self.end - self.start)).max(8);
            let max_pos = self.end - self.start - self.page;
            let top = track.top + (self.pos - self.start) * (th - thumb_h) / max_pos;
            ScrollParts { up, down, track, thumb: Rect { top, bottom: top + thumb_h, ..b }, scrolls: true }
        } else {
            ScrollParts { up, down, track, thumb: Rect { top: track.top, bottom: track.top, ..b }, scrolls: false }
        }
    }
}

/// `CDXUTEditBox` geometry for a box at `rect`: the nine skin destination rects (text area, then the frame
/// pieces in [`skin::EDIT`] order) and the text area (inset by the border and the spacing).
pub fn edit_rects(rect: Rect) -> ([Rect; 9], Rect) {
    let b = EDIT_BORDER;
    let t = Rect::new(rect.left + b, rect.top + b, rect.right - b, rect.bottom - b);
    let dst = [
        t,
        Rect::new(rect.left, rect.top, t.left, t.top),
        Rect::new(t.left, rect.top, t.right, t.top),
        Rect::new(t.right, rect.top, rect.right, t.top),
        Rect::new(rect.left, t.top, t.left, t.bottom),
        Rect::new(t.right, t.top, rect.right, t.bottom),
        Rect::new(rect.left, t.bottom, t.left, rect.bottom),
        Rect::new(t.left, t.bottom, t.right, rect.bottom),
        Rect::new(t.right, t.bottom, rect.right, rect.bottom),
    ];
    let s = EDIT_SPACING;
    (dst, Rect::new(t.left + s, t.top + s, t.right - s, t.bottom - s))
}

/// Width of an edit box's text area (what `PlaceCaret` scrolls against).
pub fn edit_text_width(rect: Rect) -> i32 {
    rect.width() - 2 * (EDIT_BORDER + EDIT_SPACING)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_hover_moves_and_reddens() {
        let r = Rect::new(10, 10, 110, 40);
        assert_eq!(button_look(r, false).rect, r);
        assert_eq!(button_look(r, false).fill >> 24, 0);
        let o = button_look(r, true);
        assert_eq!((o.rect, o.face, o.fill), (Rect::new(9, 8, 109, 38), 0xFFB9_2228, 0xFFB9_2228));
    }

    #[test]
    fn scrollbar_thumb() {
        let sb = |end, pos| ScrollBar { rect: Rect::new(0, 0, 16, 216), start: 0, end, pos, page: 10 };
        let p = sb(5, 0).parts();
        assert!(!p.scrolls);
        assert_eq!((p.up.bottom, p.down.top, p.thumb.top, p.thumb.bottom), (16, 200, 16, 16));
        let p = sb(20, 10).parts();
        assert_eq!((p.thumb.top, p.thumb.bottom), (108, 200));
        // Never shorter than 8.
        let p = sb(1000, 0).parts();
        assert_eq!(p.thumb.height(), 8);
    }

    #[test]
    fn edit_box_frame_and_area() {
        let (dst, area) = edit_rects(Rect::new(0, 0, 100, 44));
        assert_eq!(dst[0], Rect::new(5, 5, 95, 39));
        assert_eq!(dst[8], Rect::new(95, 39, 100, 44));
        assert_eq!(area, Rect::new(9, 9, 91, 35));
        assert_eq!(edit_text_width(Rect::new(0, 0, 100, 44)), 82);
        let (bar, title) = caption_rects(Rect::new(10, 20, 300, 400), 18);
        assert_eq!((bar, title.left), (Rect::new(10, 20, 300, 38), 15));
    }
}
