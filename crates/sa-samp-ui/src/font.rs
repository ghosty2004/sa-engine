//! The UI fonts: samp's D3DX "Arial" fonts are replaced by Liberation Sans Bold (metric-compatible with
//! Arial, SIL Open Font License 1.1, committed under `assets/fonts/` with its licence), rasterised once
//! into an 8-bit alpha atlas that the renderer uploads as one texture.
//!
//! Sizes follow samp (samp.dll 0x100c52d0 / 0x100c5360): the main font is 14/16/18/20 px for a video
//! width below 1024 / 1400 / 1600 / above, the small font 2 px less, GUI font 0 is 20 px and GUI font 1
//! 18 px (config `fontsize` 0). Weight 700 (samp's default `fontweight`).

use std::collections::HashMap;

use ab_glyph::{Font as _, FontRef, PxScale, ScaleFont as _, point};

/// The committed font file.
pub const FONT_TTF: &[u8] = include_bytes!("../../../assets/fonts/LiberationSans-Bold.ttf");

/// The fonts the layouts use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontId {
    /// Main font: chat lines.
    Main,
    /// Small font (main − 2): nametags, labels, dialog body, scoreboard.
    Small,
    /// GUI font 0: buttons, edit boxes.
    Gui0,
    /// GUI font 1 (GUI − 2): dialog lists.
    Gui1,
}

impl FontId {
    /// All fonts, in atlas order.
    pub const ALL: [FontId; 4] = [FontId::Main, FontId::Small, FontId::Gui0, FontId::Gui1];

    /// Index in [`FontId::ALL`].
    pub const fn index(self) -> usize {
        match self {
            FontId::Main => 0,
            FontId::Small => 1,
            FontId::Gui0 => 2,
            FontId::Gui1 => 3,
        }
    }
}

/// Main font height for video width `w` (`fontsize` 0).
pub fn main_height(w: u32) -> u32 {
    match w {
        0..1024 => 14,
        1024..1400 => 16,
        1400..1600 => 18,
        _ => 20,
    }
}

/// Heights of [`FontId::ALL`] for video width `w`.
pub fn heights_for_width(w: u32) -> [u32; 4] {
    let m = main_height(w);
    [m, m - 2, 20, 18]
}

/// Text metrics. `text` never contains colour codes (callers strip them first, as samp does).
pub trait Measure {
    /// Width of one line in pixels.
    fn width(&self, font: FontId, text: &str) -> f32;
    /// Line height (the height of "Y", samp's line unit).
    fn line_height(&self, font: FontId) -> f32;
}

/// Fixed-pitch metrics for tests: every character is `char_w` wide.
#[derive(Clone, Copy, Debug)]
pub struct MonoMeasure {
    /// Character width.
    pub char_w: f32,
    /// Line height.
    pub line_h: f32,
}

impl Measure for MonoMeasure {
    fn width(&self, _font: FontId, text: &str) -> f32 {
        self.char_w * text.chars().count() as f32
    }
    fn line_height(&self, _font: FontId) -> f32 {
        self.line_h
    }
}

/// One glyph in the atlas.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Glyph {
    /// Atlas rectangle (pixels): x, y, width, height (0 width = nothing to draw, e.g. a space).
    pub rect: [u16; 4],
    /// Offset of the rectangle from the pen position at the top of the line.
    pub offset: (f32, f32),
    /// Pen advance (whole pixels, as GDI advances).
    pub advance: f32,
}

/// The glyph atlas of the four UI fonts (8-bit coverage, row-major).
pub struct FontAtlas {
    /// Width in pixels (power of two).
    pub width: u32,
    /// Height in pixels (power of two).
    pub height: u32,
    /// Coverage, `width * height` bytes.
    pub alpha: Vec<u8>,
    heights: [u32; 4],
    glyphs: HashMap<(usize, char), Glyph>,
}

/// A rasterised glyph before packing: key, width, height, coverage.
type Bitmap = ((usize, char), u32, u32, Vec<u8>);

/// The characters rasterised: printable ASCII and Latin-1 (what code page 1252 text maps to, mostly).
fn charset() -> impl Iterator<Item = char> {
    (' '..='~').chain('\u{A1}'..='\u{FF}')
}

impl FontAtlas {
    /// Rasterises the four fonts at `heights` (see [`heights_for_width`]).
    pub fn build(heights: [u32; 4]) -> Self {
        let font =
            FontRef::try_from_slice(FONT_TTF).expect("assets/fonts/LiberationSans-Bold.ttf is a valid font");
        const W: u32 = 512;
        let mut glyphs = HashMap::new();
        let mut bitmaps: Vec<Bitmap> = Vec::new();
        for (fi, &h) in heights.iter().enumerate() {
            let scale = PxScale::from(h as f32);
            let scaled = font.as_scaled(scale);
            let ascent = scaled.ascent();
            for ch in charset() {
                let id = font.glyph_id(ch);
                let advance = scaled.h_advance(id).round();
                let mut g = Glyph {
                    advance,
                    ..Glyph::default()
                };
                if let Some(og) = font.outline_glyph(id.with_scale_and_position(scale, point(0.0, ascent))) {
                    let b = og.px_bounds();
                    let (gw, gh) = (b.width() as u32, b.height() as u32);
                    let mut px = vec![0u8; (gw * gh) as usize];
                    og.draw(|x, y, c| {
                        if x < gw && y < gh {
                            px[(y * gw + x) as usize] = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
                        }
                    });
                    g.offset = (b.min.x, b.min.y);
                    bitmaps.push(((fi, ch), gw, gh, px));
                }
                glyphs.insert((fi, ch), g);
            }
        }
        // Shelf packing, 1 px padding.
        let (mut x, mut y, mut row_h) = (1u32, 1u32, 0u32);
        let mut placed = Vec::with_capacity(bitmaps.len());
        for (key, gw, gh, px) in bitmaps {
            if x + gw + 1 > W {
                x = 1;
                y += row_h + 1;
                row_h = 0;
            }
            placed.push((key, x, y, gw, gh, px));
            x += gw + 1;
            row_h = row_h.max(gh);
        }
        let height = (y + row_h + 1).next_power_of_two();
        let mut alpha = vec![0u8; (W * height) as usize];
        for (key, gx, gy, gw, gh, px) in placed {
            for r in 0..gh {
                let d = ((gy + r) * W + gx) as usize;
                alpha[d..d + gw as usize].copy_from_slice(&px[(r * gw) as usize..((r + 1) * gw) as usize]);
            }
            if let Some(g) = glyphs.get_mut(&key) {
                g.rect = [gx as u16, gy as u16, gw as u16, gh as u16];
            }
        }
        Self {
            width: W,
            height,
            alpha,
            heights,
            glyphs,
        }
    }

    /// Heights the atlas was built for.
    pub fn heights(&self) -> [u32; 4] {
        self.heights
    }

    /// The glyph for `ch` (unknown characters draw as `?`).
    pub fn glyph(&self, font: FontId, ch: char) -> Glyph {
        let i = font.index();
        self.glyphs
            .get(&(i, ch))
            .or_else(|| self.glyphs.get(&(i, '?')))
            .copied()
            .unwrap_or_default()
    }

    /// RGBA8 pixels (white, alpha = coverage) for renderers without an alpha-only format.
    pub fn rgba(&self) -> Vec<u8> {
        self.alpha.iter().flat_map(|&a| [255, 255, 255, a]).collect()
    }
}

impl Measure for FontAtlas {
    fn width(&self, font: FontId, text: &str) -> f32 {
        text.chars()
            .map(|c| {
                if c == '\t' {
                    0.0
                } else {
                    self.glyph(font, c).advance
                }
            })
            .sum()
    }
    fn line_height(&self, font: FontId) -> f32 {
        self.heights[font.index()] as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_follow_samp_table() {
        assert_eq!(heights_for_width(800), [14, 12, 20, 18]);
        assert_eq!(heights_for_width(1024), [16, 14, 20, 18]);
        assert_eq!(heights_for_width(1400), [18, 16, 20, 18]);
        assert_eq!(heights_for_width(1920), [20, 18, 20, 18]);
    }

    #[test]
    fn atlas_holds_every_glyph_and_metrics_are_arial_like() {
        let a = FontAtlas::build(heights_for_width(1280));
        assert!(a.width.is_power_of_two() && a.height.is_power_of_two());
        assert!(a.height <= 512, "atlas {}x{}", a.width, a.height);
        // Every printable glyph but the space has pixels inside the atlas.
        for c in ('!'..='~').chain(['é', 'ß']) {
            let g = a.glyph(FontId::Main, c);
            assert!(g.rect[2] > 0 && g.rect[3] > 0, "{c:?}");
            assert!(
                u32::from(g.rect[0] + g.rect[2]) <= a.width && u32::from(g.rect[1] + g.rect[3]) <= a.height
            );
        }
        assert_eq!(a.glyph(FontId::Main, ' ').rect[2], 0);
        // Arial Bold 16 px: "Hello" is about 38 px wide; the small font is narrower.
        let w = a.width(FontId::Main, "Hello");
        assert!((30.0..46.0).contains(&w), "{w}");
        assert!(a.width(FontId::Small, "Hello") < w);
        assert_eq!(a.line_height(FontId::Gui0), 20.0);
        // Unknown characters fall back to '?'.
        assert_eq!(a.glyph(FontId::Main, '\u{4E2D}'), a.glyph(FontId::Main, '?'));
    }
}
