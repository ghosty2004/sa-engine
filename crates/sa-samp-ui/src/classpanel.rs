//! samp's class-selection panel: the GUI dialog 0x1026eaa4 with the buttons `<<`, `>>` and `Spawn`
//! (samp-rs `ui-rendering.md` §4.2 "GUI dialogs created by samp", samp-rs `Renderer::class_panel`).
//!
//! A 310 × 40 dialog on the scoreboard's background colour, centred horizontally at
//! `y = clientHeight − 40 − 50`; three DXUT buttons of 90 × 30 at (10, 5), (110, 5) and (210, 5) with GUI font 0
//! labels and the usual DXUT hover look. A button fires as DXUT's `CDXUTButton` does: pressed by the left button
//! going down over it, clicked when the button comes up still over it.

use sa_samp_api::ui::ClassButton;

use crate::draw::DrawList;
use crate::font::Measure;
use crate::panels::SB_BACKGROUND;

/// Panel width.
pub const WIDTH: f32 = 310.0;
/// Panel height.
pub const HEIGHT: f32 = 40.0;
/// Gap between the panel's bottom and the client area's bottom.
pub const BOTTOM_GAP: f32 = 50.0;
/// Button size.
pub const BUTTON_W: f32 = 90.0;
/// Button height.
pub const BUTTON_H: f32 = 30.0;
/// Button origins inside the panel, in samp's order `<<`, `>>`, `Spawn`.
pub const BUTTON_POS: [(f32, f32); 3] = [(10.0, 5.0), (110.0, 5.0), (210.0, 5.0)];
/// Button labels.
pub const LABELS: [&str; 3] = ["<<", ">>", "Spawn"];
/// The buttons, in the order of [`BUTTON_POS`].
pub const BUTTONS: [ClassButton; 3] = [ClassButton::Previous, ClassButton::Next, ClassButton::Spawn];

/// The panel rectangle for a `w` × `h` client area (integer centring as samp: `(w − 310) / 2`).
pub fn rect(w: f32, h: f32) -> [f32; 4] {
    let x = ((w - WIDTH) / 2.0).trunc();
    let y = h - HEIGHT - BOTTOM_GAP;
    [x, y, x + WIDTH, y + HEIGHT]
}

/// The three button rectangles (`<<`, `>>`, `Spawn`).
pub fn buttons(w: f32, h: f32) -> [[f32; 4]; 3] {
    let r = rect(w, h);
    BUTTON_POS.map(|(dx, dy)| {
        let (l, t) = (r[0] + dx, r[1] + dy);
        [l, t, l + BUTTON_W, t + BUTTON_H]
    })
}

/// DXUT's `PtInRect`: left / top inside, right / bottom outside.
fn contains(r: &[f32; 4], x: f32, y: f32) -> bool {
    x >= r[0] && x < r[2] && y >= r[1] && y < r[3]
}

/// The index of the button under `(x, y)`.
pub fn hit(w: f32, h: f32, x: f32, y: f32) -> Option<usize> {
    buttons(w, h).iter().position(|r| contains(r, x, y))
}

/// Draws the panel: the background, then each button (red and moved by (−1, −2) under the cursor).
pub fn draw(m: &dyn Measure, w: f32, h: f32, cursor: Option<(f32, f32)>, d: &mut DrawList) {
    d.dialog_background(rect(w, h), SB_BACKGROUND);
    for (r, label) in buttons(w, h).iter().zip(LABELS) {
        let over = cursor.is_some_and(|(x, y)| contains(r, x, y));
        d.button(m, *r, label, over);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Cmd;
    use crate::font::MonoMeasure;
    use crate::skin::{colour, src};

    const M: MonoMeasure = MonoMeasure {
        char_w: 8.0,
        line_h: 18.0,
    };

    #[test]
    fn layout_matches_samp() {
        assert_eq!(rect(800.0, 600.0), [245.0, 510.0, 555.0, 550.0]);
        assert_eq!(rect(1280.0, 720.0), [485.0, 630.0, 795.0, 670.0]);
        // Odd widths: integer division of (w − 310).
        assert_eq!(rect(1365.0, 768.0)[0], 527.0);
        assert_eq!(
            buttons(800.0, 600.0),
            [
                [255.0, 515.0, 345.0, 545.0],
                [355.0, 515.0, 445.0, 545.0],
                [455.0, 515.0, 545.0, 545.0],
            ]
        );
    }

    #[test]
    fn hit_test_edges() {
        assert_eq!(hit(800.0, 600.0, 255.0, 515.0), Some(0));
        assert_eq!(hit(800.0, 600.0, 344.9, 544.9), Some(0));
        assert_eq!(hit(800.0, 600.0, 345.0, 530.0), None);
        assert_eq!(hit(800.0, 600.0, 350.0, 530.0), None);
        assert_eq!(hit(800.0, 600.0, 355.0, 530.0), Some(1));
        assert_eq!(hit(800.0, 600.0, 500.0, 530.0), Some(2));
        assert_eq!(hit(800.0, 600.0, 500.0, 545.0), None);
        assert_eq!(hit(800.0, 600.0, 500.0, 514.0), None);
        assert_eq!(hit(800.0, 600.0, 250.0, 530.0), None);
    }

    #[test]
    fn draw_background_buttons_and_hover() {
        let mut d = DrawList {
            skin: true,
            ..DrawList::default()
        };
        draw(&M, 800.0, 600.0, Some((360.0, 520.0)), &mut d);
        assert_eq!(
            d.cmds[0],
            Cmd::Rect {
                rect: [245.0, 510.0, 555.0, 550.0],
                colour: SB_BACKGROUND
            }
        );
        let skins: Vec<_> = d
            .cmds
            .iter()
            .filter_map(|c| match c {
                Cmd::Skin { src: s, dst, colour } => Some((*s, *dst, *colour)),
                _ => None,
            })
            .collect();
        // Face per button (the fill layer of a button not under the mouse has alpha 0: not drawn).
        assert_eq!(skins.len(), 4);
        assert_eq!(
            skins[0],
            (src::BUTTON, [255.0, 515.0, 345.0, 545.0], colour::BUTTON_TEX)
        );
        // `>>` under the cursor: moved by (−1, −2), red face and red fill.
        assert_eq!(
            skins[1],
            (src::BUTTON, [354.0, 513.0, 444.0, 543.0], colour::BUTTON_TEX_OVER)
        );
        assert_eq!(
            skins[2],
            (src::BUTTON, [354.0, 513.0, 444.0, 543.0], colour::BUTTON_TEX_OVER)
        );
        assert_eq!(skins[3].1, [455.0, 515.0, 545.0, 545.0]);
        let t = d.texts();
        assert_eq!(t.iter().filter(|s| **s == "<<").count(), 2);
        assert_eq!(t.iter().filter(|s| **s == ">>").count(), 2);
        assert_eq!(t.iter().filter(|s| **s == "Spawn").count(), 2);
    }
}
