//! What the UI draws in one pass, in order and in screen pixels ([`DrawList`]), and D3DX's `DrawText` layout on the
//! glyph atlas ([`layout`]): lines, `DT_CENTER` / `DT_RIGHT` / `DT_VCENTER` / `DT_BOTTOM`, `DT_EXPANDTABS`,
//! `DT_CALCRECT`. [`crate::QuadUi::flush`] turns the list into the canvas's quads.

use sa_samp_api::ui::{TextdrawFont, UiRect, dt};

use super::font::FontAtlas;

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
    /// One run of atlas text in one colour, pen at `(x, y)` (top of the line box).
    Text {
        /// Left.
        x: f32,
        /// Top of the line box.
        y: f32,
        /// Atlas slot ([`super::font::slot`]).
        slot: usize,
        /// Colour.
        colour: u32,
        /// Bytes (code page 1252; the symbol fonts' bytes).
        text: Vec<u8>,
        /// Outline kind ([`super::font::outline`]).
        outline: u8,
    },
    /// A rectangle of samp's skin texture (`sampgui.png`) scaled into `dst`.
    Skin {
        /// Source rectangle in skin texture pixels.
        src: [f32; 4],
        /// Screen rectangle.
        dst: [f32; 4],
        /// Tint `0xAARRGGBB`.
        colour: u32,
    },
    /// A sprite textdraw (font 4, `txd:name`): `rect` `[left, top, right, bottom]`, tint `CRGBA`.
    Sprite {
        /// The textdraw's text.
        text: Vec<u8>,
        /// Rectangle.
        rect: [f32; 4],
        /// Tint.
        colour: u32,
    },
    /// A text textdraw: the `CFont` state, then `PrintString`.
    GameFont {
        /// The font state and position.
        font: TextdrawFont,
        /// Text bytes.
        text: Vec<u8>,
    },
}

/// The commands of one pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawList {
    /// Commands in draw order.
    pub cmds: Vec<Cmd>,
}

/// One laid-out line: pen position and the byte range of the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// Left.
    pub x: i32,
    /// Top.
    pub y: i32,
    /// Byte range in the text.
    pub range: std::ops::Range<usize>,
}

/// Width of `text` in slot `si`; with `DT_EXPANDTABS` a tab moves to the next multiple of eight average
/// character widths (GDI's default tab stops; the average is the width of `x`).
pub fn width(a: &FontAtlas, si: usize, text: &[u8], flags: u32) -> i32 {
    if flags & dt::EXPANDTABS == 0 || !text.contains(&b'\t') {
        return a.width(si, text);
    }
    let stop = 8 * a.width(si, b"x").max(1);
    let mut w = 0;
    for part in text.split_inclusive(|&b| b == b'\t') {
        w += a.width(si, part);
        if part.last() == Some(&b'\t') {
            w = (w / stop + 1) * stop;
        }
    }
    w
}

/// D3DX `DrawText` layout of `text` in `rect` with `flags`: the lines (empty for nothing to draw) and the
/// extent `DT_CALCRECT` reports. Lines break at `\n` (not with `DT_SINGLELINE`); `DT_VCENTER` / `DT_BOTTOM` place the
/// whole block (D3DX does so also without `DT_SINGLELINE`). Centring uses integer division
/// (`left + (width − textWidth) / 2`).
pub fn layout(a: &FontAtlas, si: usize, text: &[u8], rect: UiRect, flags: u32) -> (Vec<Line>, UiRect) {
    let lh = a.line_height(si);
    let mut ranges = Vec::new();
    if flags & dt::SINGLELINE != 0 {
        ranges.push(0..text.len());
    } else {
        let mut start = 0;
        for (i, &b) in text.iter().enumerate() {
            if b == b'\n' {
                ranges.push(start..i);
                start = i + 1;
            }
        }
        ranges.push(start..text.len());
    }
    let n = ranges.len() as i32;
    let rw = rect.right - rect.left;
    let rh = rect.bottom - rect.top;
    // D3DX centres / bottom-aligns the whole block, with or without `DT_SINGLELINE` (unlike GDI).
    let block = n * lh;
    let top = if flags & dt::VCENTER != 0 {
        rect.top + (rh - block) / 2
    } else if flags & dt::BOTTOM != 0 {
        rect.bottom - block
    } else {
        rect.top
    };
    let mut lines = Vec::with_capacity(ranges.len());
    let mut max_w = 0;
    for (k, mut r) in ranges.into_iter().enumerate() {
        if r.end > r.start && text[r.end - 1] == b'\r' {
            r.end -= 1;
        }
        let w = width(a, si, &text[r.clone()], flags);
        max_w = max_w.max(w);
        let x = if flags & dt::CENTER != 0 {
            rect.left + (rw - w) / 2
        } else if flags & dt::RIGHT != 0 {
            rect.right - w
        } else {
            rect.left
        };
        lines.push(Line { x, y: top + k as i32 * lh, range: r });
    }
    let extent = UiRect::new(rect.left, rect.top, rect.left + max_w, rect.top + n * lh);
    (lines, extent)
}

impl DrawList {
    /// A filled rectangle.
    pub fn rect(&mut self, r: [f32; 4], colour: u32) {
        self.cmds.push(Cmd::Rect { rect: r, colour });
    }

    /// Text laid out by [`layout`], one colour per visible glyph (`glyphs`, blanks have none; `None` = `colour`
    /// for all), drawn run by run with `outline` baked in.
    #[allow(clippy::too_many_arguments)]
    pub fn text(
        &mut self,
        a: &FontAtlas,
        si: usize,
        text: &[u8],
        rect: UiRect,
        flags: u32,
        colour: u32,
        glyphs: Option<&[u32]>,
        outline: u8,
    ) -> UiRect {
        let (lines, extent) = layout(a, si, text, rect, flags);
        let mut next = glyphs.unwrap_or(&[]).iter();
        let mut current = colour;
        for l in lines {
            let mut pen = l.x;
            let mut run: Vec<u8> = Vec::new();
            let mut run_x = pen;
            let mut run_colour = current;
            let flush = |cmds: &mut Vec<Cmd>, run: &mut Vec<u8>, x: i32, c: u32| {
                if run.iter().any(|&b| !matches!(b, b' ' | b'\t' | b'\r')) {
                    cmds.push(Cmd::Text {
                        x: x as f32,
                        y: l.y as f32,
                        slot: si,
                        colour: c,
                        text: std::mem::take(run),
                        outline,
                    });
                }
                run.clear();
            };
            for &b in &text[l.range.clone()] {
                let blank = matches!(b, b' ' | b'\t' | b'\r' | b'\n');
                if !blank && glyphs.is_some() {
                    current = next.next().copied().unwrap_or(current);
                }
                if b == b'\t' && flags & dt::EXPANDTABS != 0 {
                    flush(&mut self.cmds, &mut run, run_x, run_colour);
                    let stop = 8 * a.width(si, b"x").max(1);
                    pen = l.x + ((pen - l.x) / stop + 1) * stop;
                    continue;
                }
                if !blank && current != run_colour {
                    flush(&mut self.cmds, &mut run, run_x, run_colour);
                    run_colour = current;
                }
                if run.is_empty() {
                    run_x = pen;
                }
                run.push(b);
                pen += a.width(si, &[b]);
            }
            flush(&mut self.cmds, &mut run, run_x, run_colour);
        }
        extent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::{Faces, heights};

    fn atlas() -> FontAtlas {
        FontAtlas::build(Faces::default(), heights(800, 0))
    }

    #[test]
    fn layout_centres_and_breaks_lines() {
        let a = atlas();
        let w = a.width(2, b"OK");
        let (l, _) = layout(&a, 2, b"OK", UiRect::new(100, 50, 200, 80), dt::CENTER | dt::VCENTER | dt::SINGLELINE);
        assert_eq!((l[0].x, l[0].y), (100 + (100 - w) / 2, 50 + (30 - 20) / 2));
        // DXUT's button format (`DT_CENTER | DT_VCENTER`, no `DT_SINGLELINE`) centres too.
        let (l2, _) = layout(&a, 2, b"OK", UiRect::new(100, 50, 200, 80), dt::CENTER | dt::VCENTER);
        assert_eq!(l2, l);
        let (l, e) = layout(&a, 1, b"ab\ncde", UiRect::new(10, 20, 10, 20), 0);
        assert_eq!(l.len(), 2);
        assert_eq!((l[1].x, l[1].y), (10, 20 + 12));
        assert_eq!(e, UiRect::new(10, 20, 10 + a.width(1, b"cde"), 20 + 24));
        // A 1-pixel rect centres around its left edge (samp's 3D text painter).
        let (l, _) = layout(&a, 1, b"Shop", UiRect::new(300, 0, 301, 1), dt::CENTER);
        assert_eq!(l[0].x, 300 + (1 - a.width(1, b"Shop")) / 2);
    }

    #[test]
    fn text_splits_colour_runs_and_skips_blanks() {
        let a = atlas();
        let mut d = DrawList::default();
        let glyphs = [0xFFFF_FFFF, 0xFFFF_FFFF, 0xFFFF_0000];
        d.text(&a, 0, b"ab c", UiRect::new(5, 6, 5, 6), 0, 0xFFFF_FFFF, Some(&glyphs), 1);
        let runs: Vec<(f32, u32, Vec<u8>)> = d
            .cmds
            .iter()
            .map(|c| match c {
                Cmd::Text { x, colour, text, .. } => (*x, *colour, text.clone()),
                _ => panic!(),
            })
            .collect();
        assert_eq!(runs[0], (5.0, 0xFFFF_FFFF, b"ab ".to_vec()));
        assert_eq!(runs[1], (5.0 + a.width(0, b"ab ") as f32, 0xFFFF_0000, b"c".to_vec()));
        // Tabs expand to multiples of eight widths of 'x'.
        let stop = 8 * a.width(1, b"x");
        assert_eq!(width(&a, 1, b"a\tb", dt::EXPANDTABS), stop + a.width(1, b"b"));
        assert_eq!(width(&a, 1, b"a\tb", 0), a.width(1, b"ab"));
    }
}
