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

/// The glyph atlas of the four UI fonts. Every glyph is there twice: plain, and with samp's one-pixel
/// black outline baked in (the four-direction outline of the outlined font: black text at the four offsets,
/// then the text), so an outlined glyph is one quad instead of five. Pixels are white with the glyph's
/// coverage as alpha; outline pixels are black. The vertex colour multiplies both.
pub struct FontAtlas {
    /// Width in pixels (power of two).
    pub width: u32,
    /// Height in pixels (power of two).
    pub height: u32,
    /// Coverage (alpha), `width * height` bytes.
    pub alpha: Vec<u8>,
    /// Brightness (0 = outline black, 255 = glyph white), `width * height` bytes.
    pub lum: Vec<u8>,
    heights: [u32; 4],
    glyphs: HashMap<(usize, bool, char), Glyph>,
}

/// A rasterised glyph before packing: key, width, height, alpha and brightness planes.
type Bitmap = ((usize, bool, char), u32, u32, Vec<u8>, Vec<u8>);

/// The characters rasterised: printable ASCII and Latin-1 (what code page 1252 text maps to, mostly).
fn charset() -> impl Iterator<Item = char> {
    (' '..='~').chain('\u{A1}'..='\u{FF}')
}

/// The outlined variant of a `w × h` coverage bitmap: one pixel larger on every side; black at the four
/// offsets below the glyph (alpha `o`), the glyph over it (alpha `f`): `A = f + o·(1 − f)`, brightness `f / A`.
fn outlined(w: u32, h: u32, cov: &[u8]) -> (u32, u32, Vec<u8>, Vec<u8>) {
    let (ow, oh) = (w + 2, h + 2);
    let at = |x: i32, y: i32| -> f32 {
        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
            0.0
        } else {
            f32::from(cov[(y as u32 * w + x as u32) as usize]) / 255.0
        }
    };
    let mut alpha = vec![0u8; (ow * oh) as usize];
    let mut lum = vec![0u8; (ow * oh) as usize];
    for y in 0..oh as i32 {
        for x in 0..ow as i32 {
            let (gx, gy) = (x - 1, y - 1);
            let f = at(gx, gy);
            let o = [(0, -1), (0, 1), (-1, 0), (1, 0)]
                .iter()
                .map(|(dx, dy)| at(gx - dx, gy - dy))
                .fold(0.0, f32::max);
            let a = f + o * (1.0 - f);
            let i = (y as u32 * ow + x as u32) as usize;
            alpha[i] = (a * 255.0).round() as u8;
            lum[i] = if a > 0.0 {
                (f / a * 255.0).round() as u8
            } else {
                255
            };
        }
    }
    (ow, oh, alpha, lum)
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
                let mut go = g;
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
                    go.offset = (b.min.x - 1.0, b.min.y - 1.0);
                    let (ow, oh, oa, ol) = outlined(gw, gh, &px);
                    bitmaps.push(((fi, true, ch), ow, oh, oa, ol));
                    let lum = vec![255u8; px.len()];
                    bitmaps.push(((fi, false, ch), gw, gh, px, lum));
                }
                glyphs.insert((fi, false, ch), g);
                glyphs.insert((fi, true, ch), go);
            }
        }
        // Shelf packing, 1 px padding.
        let (mut x, mut y, mut row_h) = (1u32, 1u32, 0u32);
        let mut placed = Vec::with_capacity(bitmaps.len());
        for (key, gw, gh, a, l) in bitmaps {
            if x + gw + 1 > W {
                x = 1;
                y += row_h + 1;
                row_h = 0;
            }
            placed.push((key, x, y, gw, gh, a, l));
            x += gw + 1;
            row_h = row_h.max(gh);
        }
        let height = (y + row_h + 1).next_power_of_two();
        let mut alpha = vec![0u8; (W * height) as usize];
        let mut lum = vec![255u8; (W * height) as usize];
        for (key, gx, gy, gw, gh, a, l) in placed {
            for r in 0..gh {
                let d = ((gy + r) * W + gx) as usize;
                let s = (r * gw) as usize..((r + 1) * gw) as usize;
                alpha[d..d + gw as usize].copy_from_slice(&a[s.clone()]);
                lum[d..d + gw as usize].copy_from_slice(&l[s]);
            }
            if let Some(g) = glyphs.get_mut(&key) {
                g.rect = [gx as u16, gy as u16, gw as u16, gh as u16];
            }
        }
        Self {
            width: W,
            height,
            alpha,
            lum,
            heights,
            glyphs,
        }
    }

    /// Heights the atlas was built for.
    pub fn heights(&self) -> [u32; 4] {
        self.heights
    }

    /// The plain glyph for `ch` (unknown characters draw as `?`).
    pub fn glyph(&self, font: FontId, ch: char) -> Glyph {
        self.glyph_variant(font, false, ch)
    }

    /// The plain or outlined glyph for `ch`.
    pub fn glyph_variant(&self, font: FontId, outline: bool, ch: char) -> Glyph {
        let i = font.index();
        self.glyphs
            .get(&(i, outline, ch))
            .or_else(|| self.glyphs.get(&(i, outline, '?')))
            .copied()
            .unwrap_or_default()
    }

    /// RGBA8 pixels (brightness as grey, coverage as alpha).
    pub fn rgba(&self) -> Vec<u8> {
        self.alpha
            .iter()
            .zip(&self.lum)
            .flat_map(|(&a, &l)| [l, l, l, a])
            .collect()
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
        assert!(a.height <= 1024, "atlas {}x{}", a.width, a.height);
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
        // The outlined variant is one pixel larger on every side, black around a white core.
        let (p, o) = (
            a.glyph(FontId::Main, 'H'),
            a.glyph_variant(FontId::Main, true, 'H'),
        );
        assert_eq!((o.rect[2], o.rect[3]), (p.rect[2] + 2, p.rect[3] + 2));
        assert_eq!(o.offset, (p.offset.0 - 1.0, p.offset.1 - 1.0));
        assert_eq!(o.advance, p.advance);
        let corner = |g: Glyph, dx: u16, dy: u16| {
            let i = (g.rect[1] + dy) as usize * a.width as usize + (g.rect[0] + dx) as usize;
            (a.alpha[i], a.lum[i])
        };
        // 'H' has a vertical stem at its left edge: the pixel left of it is outline (black, opaque).
        let mid = o.rect[3] / 2;
        let row: Vec<(u8, u8)> = (0..o.rect[2]).map(|x| corner(o, x, mid)).collect();
        let first = row.iter().find(|(a, _)| *a > 200).unwrap();
        assert!(first.1 < 100, "{row:?}");
        assert!(row.iter().any(|&(a, l)| a == 255 && l == 255), "{row:?}");
        // Unknown characters fall back to '?'.
        assert_eq!(a.glyph(FontId::Main, '\u{4E2D}'), a.glyph(FontId::Main, '?'));
    }
}
