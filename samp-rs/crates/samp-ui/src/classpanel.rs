//! samp's class-selection panel: the GUI dialog 0x1026eaa4 with the buttons `<<`, `>>` and `Spawn`
//! (docs/re/subsystems/ui-rendering.md, "4.2 GUI dialogs created by samp").
//!
//! A 310 × 40 dialog on the scoreboard's background colour, centred horizontally at `y = clientH − 40 − 50`;
//! three DXUT buttons of 90 × 30 at (10, 5), (110, 5) and (210, 5) with GUI font 0 labels. A button fires as
//! `CDXUTButton` does: the left button goes down over it and comes up still over it.

use crate::Rect;

/// Panel size.
pub const WIDTH: i32 = 310;
pub const HEIGHT: i32 = 40;
/// Gap between the panel's bottom and the client area's bottom.
pub const BOTTOM_GAP: i32 = 50;
/// Button size.
pub const BUTTON_W: i32 = 90;
pub const BUTTON_H: i32 = 30;
/// Button x offsets inside the panel (`<<`, `>>`, `Spawn`); all at y 5.
pub const BUTTON_X: [i32; 3] = [10, 110, 210];
pub const BUTTON_Y: i32 = 5;
/// Button labels.
pub const LABELS: [&[u8]; 3] = [b"<<", b">>", b"Spawn"];
/// Background (the scoreboard's).
pub const BACKGROUND: u32 = crate::scoreboard::BACKGROUND;

/// A panel button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassButton {
    /// `<<`: previous class (control id 6).
    Previous,
    /// `>>`: next class (control id 5).
    Next,
    /// `Spawn` (control id 4).
    Spawn,
}

/// The buttons in [`BUTTON_X`] order.
pub const BUTTONS: [ClassButton; 3] = [ClassButton::Previous, ClassButton::Next, ClassButton::Spawn];

/// The panel rectangle in a `client_w × client_h` client area (integer centring as samp).
pub fn rect(client_w: i32, client_h: i32) -> Rect {
    Rect::xywh((client_w - WIDTH) / 2, client_h - HEIGHT - BOTTOM_GAP, WIDTH, HEIGHT)
}

/// The three button rectangles.
pub fn buttons(client_w: i32, client_h: i32) -> [Rect; 3] {
    let r = rect(client_w, client_h);
    BUTTON_X.map(|dx| Rect::xywh(r.left + dx, r.top + BUTTON_Y, BUTTON_W, BUTTON_H))
}

/// The button under `(x, y)` among `rects` (DXUT `PtInRect`).
pub fn hit(rects: &[Rect; 3], x: i32, y: i32) -> Option<ClassButton> {
    rects.iter().position(|r| r.contains(x, y)).map(|i| BUTTONS[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_samp() {
        assert_eq!(rect(800, 600), Rect::new(245, 510, 555, 550));
        assert_eq!(rect(1280, 720), Rect::new(485, 630, 795, 670));
        assert_eq!(rect(1365, 768).left, 527);
        assert_eq!(
            buttons(800, 600),
            [Rect::new(255, 515, 345, 545), Rect::new(355, 515, 445, 545), Rect::new(455, 515, 545, 545)]
        );
    }

    #[test]
    fn hit_test_edges() {
        let b = buttons(800, 600);
        assert_eq!(hit(&b, 255, 515), Some(ClassButton::Previous));
        assert_eq!(hit(&b, 344, 544), Some(ClassButton::Previous));
        assert_eq!(hit(&b, 345, 530), None);
        assert_eq!(hit(&b, 355, 530), Some(ClassButton::Next));
        assert_eq!(hit(&b, 500, 530), Some(ClassButton::Spawn));
        assert_eq!(hit(&b, 500, 545), None);
        assert_eq!(hit(&b, 500, 514), None);
    }
}
