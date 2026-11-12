//! The draw list: what the UI draws this frame, in order, in screen pixels. The engine renderer
//! (`sa-samp::overlay`) turns it into `CSprite2d` rectangles, RW Im2D glyph quads and `CFont` prints.

use sa_samp_api::hud::FontStyle;

use crate::colour::{colour_runs, strip_codes};
use crate::font::{FontId, Measure};

/// Black, every outline's colour.
pub const OUTLINE: u32 = 0xFF00_0000;
/// Four-direction outline (small texts).
pub const OUTLINE_4: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];
/// Eight-offset outline of chat lines above 1280 px.
pub const OUTLINE_8: [(i32, i32); 8] = [
    (0, -2),
    (-1, -1),
    (1, -1),
    (0, 2),
    (-1, 1),
    (1, 1),
    (-2, 0),
    (2, 0),
];

/// One draw command.
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    /// A filled rectangle `[left, top, right, bottom]`, `0xAARRGGBB`.
    Rect {
        /// Rectangle.
        rect: [f32; 4],
        /// Colour.
        colour: u32,
    },
    /// A filled triangle (the cursor).
    Triangle {
        /// Corners.
        pts: [(f32, f32); 3],
        /// Colour.
        colour: u32,
    },
    /// One line of UI-font text, top-left at `(x, y)`, no colour codes.
    Text {
        /// Left.
        x: f32,
        /// Top of the line box.
        y: f32,
        /// Font.
        font: FontId,
        /// Colour.
        colour: u32,
        /// Text.
        text: String,
    },
    /// Text in a game font through `CFont` (textdraws): the complete font state, then `PrintString`.
    GameFont {
        /// Font state.
        style: FontStyle,
        /// `PrintString` position (screen pixels).
        x: f32,
        /// Position.
        y: f32,
        /// Text (GTA codes such as `~r~` are the game font's).
        text: String,
    },
}

/// The commands of one frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawList {
    /// Commands in draw order.
    pub cmds: Vec<Cmd>,
}

impl DrawList {
    /// A filled rectangle.
    pub fn rect(&mut self, l: f32, t: f32, r: f32, b: f32, colour: u32) {
        self.cmds.push(Cmd::Rect {
            rect: [l, t, r, b],
            colour,
        });
    }

    /// A rectangle outline of width 1.
    pub fn frame(&mut self, l: f32, t: f32, r: f32, b: f32, colour: u32) {
        self.rect(l, t, r, t + 1.0, colour);
        self.rect(l, b - 1.0, r, b, colour);
        self.rect(l, t, l + 1.0, b, colour);
        self.rect(r - 1.0, t, r, b, colour);
    }

    /// Plain text (no codes, no outline).
    pub fn text(&mut self, x: f32, y: f32, font: FontId, colour: u32, text: &str) {
        if !text.is_empty() {
            self.cmds.push(Cmd::Text {
                x,
                y,
                font,
                colour,
                text: text.into(),
            });
        }
    }

    /// Text with `{RRGGBB}` codes, drawn run by run, each run first in black at the outline offsets (samp's
    /// outlined font: the stripped text in black, then in colour). Returns the stripped width.
    #[allow(clippy::too_many_arguments)]
    pub fn coloured(
        &mut self,
        m: &dyn Measure,
        x: f32,
        y: f32,
        font: FontId,
        text: &str,
        base: u32,
        outline: &[(i32, i32)],
    ) -> f32 {
        let stripped = strip_codes(text);
        if !outline.is_empty() {
            for &(dx, dy) in outline {
                // The outline alpha follows the base colour's alpha (fading labels fade their outline too).
                let a = base & 0xFF00_0000;
                self.text(
                    x + dx as f32,
                    y + dy as f32,
                    font,
                    a | (OUTLINE & 0xFF_FFFF),
                    &stripped,
                );
            }
        }
        let mut pen = x;
        for run in colour_runs(text, base) {
            self.text(pen, y, font, run.colour, &run.text);
            pen += m.width(font, &run.text);
        }
        pen - x
    }

    /// Appends another list.
    pub fn extend(&mut self, other: DrawList) {
        self.cmds.extend(other.cmds);
    }

    /// The texts of all [`Cmd::Text`] commands, for tests and logs.
    pub fn texts(&self) -> Vec<&str> {
        self.cmds
            .iter()
            .filter_map(|c| match c {
                Cmd::Text { text, .. } | Cmd::GameFont { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::MonoMeasure;

    #[test]
    fn coloured_text_draws_outline_then_runs() {
        let m = MonoMeasure {
            char_w: 7.0,
            line_h: 16.0,
        };
        let mut d = DrawList::default();
        let w = d.coloured(
            &m,
            10.0,
            20.0,
            FontId::Main,
            "ab{FF0000}cd",
            0xFFFF_FFFF,
            &OUTLINE_4,
        );
        assert_eq!(w, 28.0);
        assert_eq!(d.cmds.len(), 6);
        assert_eq!(
            d.cmds[0],
            Cmd::Text {
                x: 10.0,
                y: 19.0,
                font: FontId::Main,
                colour: 0xFF00_0000,
                text: "abcd".into()
            }
        );
        assert_eq!(
            d.cmds[5],
            Cmd::Text {
                x: 24.0,
                y: 20.0,
                font: FontId::Main,
                colour: 0xFFFF_0000,
                text: "cd".into()
            }
        );
    }
}
