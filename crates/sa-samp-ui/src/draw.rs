//! The draw list: what the UI draws this frame, in order, in screen pixels. The engine renderer
//! (`sa-samp::overlay`) turns it into `CSprite2d` rectangles, RW Im2D glyph quads and `CFont` prints.

use sa_samp_api::hud::FontStyle;

use crate::colour::colour_runs;
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
        /// Outline kind (`font::outline`: none, samp's 1-pixel four-direction or 2-pixel eight-offset
        /// black outline; the renderer's outlined glyphs).
        outline: u8,
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
    /// A rectangle of samp's skin texture (`sampgui.png`, `mouse.png`; [`crate::skin`]) scaled into `dst`.
    Skin {
        /// Source rectangle in skin texture pixels `[left, top, right, bottom]`.
        src: [f32; 4],
        /// Screen rectangle.
        dst: [f32; 4],
        /// Tint `0xAARRGGBB`.
        colour: u32,
    },
    /// A TXD texture (sprite textdraws) through `CSprite2d::Draw`.
    Sprite {
        /// Texture source.
        source: crate::textdraw::SpriteSource,
        /// `[left, top, right, bottom]`.
        rect: [f32; 4],
        /// Tint `0xAARRGGBB`.
        colour: u32,
    },
}

/// The outline kind of an outline offset list (empty, [`OUTLINE_4`], [`OUTLINE_8`]).
pub fn outline_kind(offsets: &[(i32, i32)]) -> u8 {
    use crate::font::outline;
    match offsets.len() {
        0 => outline::NONE,
        8 => outline::TWO,
        _ => outline::ONE,
    }
}

/// The commands of one frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawList {
    /// Commands in draw order.
    pub cmds: Vec<Cmd>,
    /// Whether samp's skin texture is available ([`Cmd::Skin`]); otherwise GUI controls are flat rectangles.
    pub skin: bool,
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
                outline: crate::font::outline::NONE,
            });
        }
    }

    /// Text with `{RRGGBB}` codes, drawn run by run; with a non-empty `outline` every run is outlined in
    /// black (samp's outlined font draws the stripped text in black at the outline offsets, then in colour;
    /// the renderer bakes the one-pixel outline into its glyphs). Returns the stripped width.
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
        let mut pen = x;
        for run in colour_runs(text, base) {
            if !run.text.is_empty() {
                self.cmds.push(Cmd::Text {
                    x: pen,
                    y,
                    font,
                    colour: run.colour,
                    text: run.text.clone(),
                    outline: outline_kind(outline),
                });
            }
            pen += m.width(font, &run.text);
        }
        pen - x
    }

    /// Multi-line text drawn as one D3DX `DrawText` call: `\n` starts a line `line_height` lower, and a
    /// colour set by `{RRGGBB}` carries over the line break.
    #[allow(clippy::too_many_arguments)]
    pub fn coloured_lines(
        &mut self,
        m: &dyn Measure,
        x: f32,
        y: f32,
        font: FontId,
        text: &str,
        base: u32,
        outline: &[(i32, i32)],
    ) {
        let lh = m.line_height(font);
        let (mut pen, mut line) = (x, 0.0f32);
        for run in colour_runs(text, base) {
            for (i, part) in run.text.split('\n').enumerate() {
                if i > 0 {
                    pen = x;
                    line += 1.0;
                }
                let part = part.trim_end_matches('\r');
                if !part.is_empty() {
                    self.cmds.push(Cmd::Text {
                        x: pen,
                        y: y + line * lh,
                        font,
                        colour: run.colour,
                        text: part.into(),
                        outline: outline_kind(outline),
                    });
                }
                pen += m.width(font, part);
            }
        }
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
    fn coloured_text_draws_outlined_runs() {
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
        let run = |x: f32, colour: u32, text: &str, outline: u8| Cmd::Text {
            x,
            y: 20.0,
            font: FontId::Main,
            colour,
            text: text.into(),
            outline,
        };
        assert_eq!(
            d.cmds,
            vec![run(10.0, 0xFFFF_FFFF, "ab", 1), run(24.0, 0xFFFF_0000, "cd", 1)]
        );
        let mut d = DrawList::default();
        d.coloured(&m, 0.0, 20.0, FontId::Main, "x", 0xFF12_3456, &[]);
        assert_eq!(d.cmds, vec![run(0.0, 0xFF12_3456, "x", 0)]);
    }
}
