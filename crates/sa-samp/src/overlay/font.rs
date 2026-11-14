//! The engine backend's fonts: samp's D3DX fonts rasterised once into an 8-bit atlas the renderer uploads as one
//! texture (docs/samp/overlay.md §2.1).
//!
//! The UI fonts ("Arial" bold in samp) use the user's Arial Bold when the host found one, else the committed
//! Liberation Sans Bold (metric-compatible, SIL Open Font License 1.1, `assets/fonts/`). The death window's
//! GTAWEAPON3 fonts and the AFK icon's SAMPAUX3 fonts are the loose `gtaweap3.ttf` / `sampaux3.ttf` of the user's
//! GTA / SA:MP install, read at run time (never shipped); without them those glyphs are not drawn. Sizes are
//! samp's (`samp_ui::fonts`): main / small by video width, GUI 20 / 18, aux 12 / 14, all ±2 px per `/fontsize`
//! step; the icon fonts aux + 8 / aux + 12, SAMPAUX3 20 / 22.

use ab_glyph::{Font as _, FontRef, PxScale, ScaleFont as _, point};
use sa_samp_api::ui::UiFont;

/// The committed UI font.
pub const FONT_TTF: &[u8] = include_bytes!("../../../../assets/fonts/LiberationSans-Bold.ttf");

/// Whether `ttf` is a TrueType / OpenType font the atlas can use.
pub fn parses(ttf: &[u8]) -> bool {
    FontRef::try_from_slice(ttf).is_ok()
}

/// Atlas slots: main, small, GUI 0, GUI 1, death-window names, GTAWEAPON3 icon, GTAWEAPON3 backdrop, SAMPAUX3 20,
/// SAMPAUX3 22.
pub const SLOTS: usize = 9;

/// The atlas slot of a backend font (the outline twins share their font's glyphs).
pub const fn slot(f: UiFont) -> usize {
    match f {
        UiFont::Main | UiFont::MainOutline => 0,
        UiFont::Small | UiFont::SmallOutline => 1,
        UiFont::Gui0 => 2,
        UiFont::Gui1 => 3,
        UiFont::DeathNames => 4,
        UiFont::DeathIcon => 5,
        UiFont::DeathBackdrop => 6,
        UiFont::Aux20 => 7,
        UiFont::Aux22 => 8,
    }
}

/// Cell heights of the slots for video width `w` and `/fontsize` `fs` (samp.dll 0x100c52d0 / 0x100c5320 /
/// 0x100c5360), never below 6 px.
pub fn heights(w: i32, fs: i32) -> [u32; SLOTS] {
    use samp_ui::fonts::{aux_height, gui_height, main_height, small_height};
    let fs = fs.clamp(-3, 5);
    let aux = aux_height(w, fs);
    let gui = gui_height(fs);
    [main_height(w, fs), small_height(w, fs), gui, gui - 2, aux, aux + 8, aux + 12, 20, 22].map(|h| h.max(6) as u32)
}

/// Outline kinds of the atlas glyphs.
pub mod outline {
    /// No outline.
    pub const NONE: u8 = 0;
    /// samp's four-direction one-pixel outline.
    pub const ONE: u8 = 1;
    /// samp's eight-offset two-pixel outline (chat lines above 1280 px; main font only).
    pub const TWO: u8 = 2;
}

/// Four-direction outline (samp_ui::fonts::OUTLINE_4).
const OUTLINE_4: [(i32, i32); 4] = samp_ui::fonts::OUTLINE_4;
/// Eight-offset outline (samp_ui::fonts::OUTLINE_8).
const OUTLINE_8: [(i32, i32); 8] = samp_ui::fonts::OUTLINE_8;

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

/// The TrueType files the atlas is built from.
#[derive(Clone, Copy, Debug, Default)]
pub struct Faces<'a> {
    /// The UI font (`None`: the committed Liberation Sans Bold).
    pub ui: Option<&'a [u8]>,
    /// `gtaweap3.ttf`.
    pub weapon: Option<&'a [u8]>,
    /// `sampaux3.ttf`.
    pub aux: Option<&'a [u8]>,
}

/// The glyph atlas. Every UI glyph is there plain and with samp's one-pixel black outline baked in (the
/// four-direction outline of the outlined font: black text at the four offsets, then the text), the main font
/// also with the two-pixel eight-offset outline, so an outlined glyph is one quad instead of five or nine. Pixels
/// are white with the glyph's coverage as alpha; outline pixels are black. The vertex colour multiplies both.
pub struct FontAtlas {
    /// Width in pixels (power of two).
    pub width: u32,
    /// Height in pixels (power of two).
    pub height: u32,
    /// Coverage (alpha), `width * height` bytes.
    pub alpha: Vec<u8>,
    /// Brightness (0 = outline black, 255 = glyph white), `width * height` bytes.
    pub lum: Vec<u8>,
    heights: [u32; SLOTS],
    /// Per slot and outline kind, the glyph of every byte (code page 1252 / the symbol font's byte).
    glyphs: Vec<[[Option<Glyph>; 256]; 3]>,
    /// Slots whose font file was there.
    present: [bool; SLOTS],
}

/// An outline variant: kind, offsets, padding.
type OutlineKind = (u8, &'static [(i32, i32)], u32);

/// A rasterised glyph before packing: key (slot, outline, byte), width, height, alpha and brightness planes.
type Bitmap = ((usize, u8, u8), u32, u32, Vec<u8>, Vec<u8>);

/// The outlined variant of a `w × h` coverage bitmap: `pad` pixels larger on every side; black at the
/// outline `offsets` below the glyph (alpha `o`), the glyph over it (alpha `f`): `A = f + o·(1 − f)`,
/// brightness `f / A`.
fn outlined(w: u32, h: u32, cov: &[u8], offsets: &[(i32, i32)], pad: u32) -> (u32, u32, Vec<u8>, Vec<u8>) {
    let (ow, oh) = (w + 2 * pad, h + 2 * pad);
    let p = pad as i32;
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
            let (gx, gy) = (x - p, y - p);
            let f = at(gx, gy);
            let o = offsets.iter().map(|(dx, dy)| at(gx - dx, gy - dy)).fold(0.0, f32::max);
            let a = f + o * (1.0 - f);
            let i = (y as u32 * ow + x as u32) as usize;
            alpha[i] = (a * 255.0).round() as u8;
            lum[i] = if a > 0.0 { (f / a * 255.0).round() as u8 } else { 255 };
        }
    }
    (ow, oh, alpha, lum)
}

/// The glyph of `code` in a font's Windows symbol cmap (platform 3, encoding 0, format 4), where symbol fonts
/// such as GTAWEAPON3 and SAMPAUX3 map their bytes (at U+F000 + byte; GDI looks them up there for a
/// `SYMBOL_CHARSET` font). `None` when the font has no such subtable or the code is not mapped.
pub fn symbol_glyph(ttf: &[u8], code: u16) -> Option<u16> {
    let u16_at = |o: usize| -> Option<u16> { Some(u16::from_be_bytes(ttf.get(o..o + 2)?.try_into().ok()?)) };
    let u32_at = |o: usize| -> Option<u32> { Some(u32::from_be_bytes(ttf.get(o..o + 4)?.try_into().ok()?)) };
    let tables = usize::from(u16_at(4)?);
    let cmap = (0..tables).find_map(|i| {
        let rec = 12 + 16 * i;
        (ttf.get(rec..rec + 4)? == b"cmap").then(|| u32_at(rec + 8)).flatten()
    })? as usize;
    let subtables = usize::from(u16_at(cmap + 2)?);
    let sub = (0..subtables).find_map(|i| {
        let rec = cmap + 4 + 8 * i;
        (u16_at(rec)? == 3 && u16_at(rec + 2)? == 0).then(|| u32_at(rec + 4)).flatten()
    })? as usize
        + cmap;
    if u16_at(sub)? != 4 {
        return None;
    }
    let segs = usize::from(u16_at(sub + 6)? / 2);
    let ends = sub + 14;
    let starts = ends + 2 * segs + 2;
    let deltas = starts + 2 * segs;
    let offsets = deltas + 2 * segs;
    for i in 0..segs {
        let (end, start) = (u16_at(ends + 2 * i)?, u16_at(starts + 2 * i)?);
        if code < start || code > end {
            continue;
        }
        let delta = u16_at(deltas + 2 * i)?;
        let ro = u16_at(offsets + 2 * i)?;
        let g = if ro == 0 {
            code.wrapping_add(delta)
        } else {
            let at = offsets + 2 * i + usize::from(ro) + 2 * usize::from(code - start);
            match u16_at(at)? {
                0 => 0,
                g => g.wrapping_add(delta),
            }
        };
        return (g != 0).then_some(g);
    }
    None
}

/// The byte the UI fonts hold DXUT's password mask U+25CF `●` at (DEL, never drawn as text).
pub const MASK_BYTE: u8 = 0x7F;

/// The bytes rasterised for a UI font: printable ASCII and Latin-1 (what code page 1252 text maps to, mostly),
/// and the password mask at [`MASK_BYTE`].
fn ui_bytes() -> impl Iterator<Item = u8> {
    (b' '..=b'~').chain([MASK_BYTE]).chain(0xA1..=0xFF)
}

impl FontAtlas {
    /// Rasterises the slots at `heights` (see [`heights`]) from `faces`.
    pub fn build(faces: Faces, heights: [u32; SLOTS]) -> Self {
        let ui = faces
            .ui
            .and_then(|t| FontRef::try_from_slice(t).ok())
            .unwrap_or_else(|| FontRef::try_from_slice(FONT_TTF).expect("the committed UI font parses"));
        let weapon = faces.weapon.and_then(|t| FontRef::try_from_slice(t).ok());
        let aux = faces.aux.and_then(|t| FontRef::try_from_slice(t).ok());
        const W: u32 = 512;
        let mut glyphs = vec![[[None; 256]; 3]; SLOTS];
        let mut present = [false; SLOTS];
        let mut bitmaps: Vec<Bitmap> = Vec::new();
        for (si, &h) in heights.iter().enumerate() {
            let (font, symbol, file) = match si {
                0..=4 => (Some(&ui), false, None),
                5 | 6 => (weapon.as_ref(), true, faces.weapon),
                _ => (aux.as_ref(), true, faces.aux),
            };
            let Some(font) = font else { continue };
            present[si] = true;
            // GDI under Wine (what samp's D3DX font uses): a cell height `h` gives a whole em size
            // `trunc(h · unitsPerEm / (ascent − descent))` (14 px → 12 ppem for Arial / Liberation Sans:
            // measured, the reference chat line widths match only this), and an integer baseline.
            let units = font.units_per_em().unwrap_or(2048.0);
            let cell = font.height_unscaled();
            let ppem = (h as f32 * units / cell).floor().max(1.0);
            let scale = PxScale::from(ppem * cell / units);
            let scaled = font.as_scaled(scale);
            let ascent = scaled.ascent().round();
            let kinds: &[OutlineKind] = match si {
                0 => &[(outline::ONE, &OUTLINE_4, 1), (outline::TWO, &OUTLINE_8, 2)],
                1..=4 => &[(outline::ONE, &OUTLINE_4, 1)],
                _ => &[],
            };
            let bytes: Vec<u8> = if symbol { (b' '..=b'~').collect() } else { ui_bytes().collect() };
            for b in bytes {
                // Symbol fonts map their bytes at U+F000 + byte in their Windows symbol cmap (some at the byte).
                let ch = if !symbol && b == MASK_BYTE { '\u{25CF}' } else { char::from(b) };
                let mut id = font.glyph_id(ch);
                if symbol && let Some(ttf) = file {
                    match symbol_glyph(ttf, 0xF000 + u16::from(b)).or_else(|| symbol_glyph(ttf, u16::from(b))) {
                        Some(g) => id = ab_glyph::GlyphId(g),
                        None => continue,
                    }
                }
                let advance = scaled.h_advance(id).round();
                let mut g = Glyph { advance, ..Glyph::default() };
                let mut variants = Vec::new();
                if let Some(og) = font.outline_glyph(id.with_scale_and_position(scale, point(0.0, ascent))) {
                    let bb = og.px_bounds();
                    let (gw, gh) = (bb.width() as u32, bb.height() as u32);
                    let mut px = vec![0u8; (gw * gh) as usize];
                    og.draw(|x, y, c| {
                        if x < gw && y < gh {
                            px[(y * gw + x) as usize] = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
                        }
                    });
                    g.offset = (bb.min.x, bb.min.y);
                    for &(kind, offsets, pad) in kinds {
                        let (ow, oh, oa, ol) = outlined(gw, gh, &px, offsets, pad);
                        bitmaps.push(((si, kind, b), ow, oh, oa, ol));
                        variants.push((kind, pad as f32));
                    }
                    let lum = vec![255u8; px.len()];
                    bitmaps.push(((si, outline::NONE, b), gw, gh, px, lum));
                } else {
                    variants.extend(kinds.iter().map(|k| (k.0, k.2 as f32)));
                }
                glyphs[si][usize::from(outline::NONE)][usize::from(b)] = Some(g);
                for (kind, pad) in variants {
                    let mut go = g;
                    go.offset = (g.offset.0 - pad, g.offset.1 - pad);
                    glyphs[si][usize::from(kind)][usize::from(b)] = Some(go);
                }
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
        for ((si, kind, b), gx, gy, gw, gh, a, l) in placed {
            for r in 0..gh {
                let d = ((gy + r) * W + gx) as usize;
                let s = (r * gw) as usize..((r + 1) * gw) as usize;
                alpha[d..d + gw as usize].copy_from_slice(&a[s.clone()]);
                lum[d..d + gw as usize].copy_from_slice(&l[s]);
            }
            if let Some(g) = &mut glyphs[si][usize::from(kind)][usize::from(b)] {
                g.rect = [gx as u16, gy as u16, gw as u16, gh as u16];
            }
        }
        Self { width: W, height, alpha, lum, heights, glyphs, present }
    }

    /// Heights the atlas was built for.
    pub fn heights(&self) -> [u32; SLOTS] {
        self.heights
    }

    /// Whether slot `si`'s font file was there.
    pub fn has(&self, si: usize) -> bool {
        self.present[si]
    }

    /// The cell height of slot `si`.
    pub fn line_height(&self, si: usize) -> i32 {
        self.heights[si] as i32
    }

    /// The glyph for byte `b` of slot `si` with outline kind `kind` (the two-pixel outline falls back to the
    /// one-pixel one outside the main font, outlines to the plain glyph in the symbol fonts); unknown bytes of a
    /// UI font draw as `?`.
    pub fn glyph(&self, si: usize, kind: u8, b: u8) -> Glyph {
        let kind = if kind == outline::TWO && si != 0 { outline::ONE } else { kind };
        let kind = if si > 4 { outline::NONE } else { kind };
        let set = &self.glyphs[si][usize::from(kind)];
        set[usize::from(b)].or(if si <= 4 { set[usize::from(b'?')] } else { None }).unwrap_or_default()
    }

    /// Width of `text` in slot `si` (tabs have no width here; `DT_EXPANDTABS` is the layout's).
    pub fn width(&self, si: usize, text: &[u8]) -> i32 {
        text.iter().map(|&b| if b == b'\t' { 0.0 } else { self.glyph(si, outline::NONE, b).advance }).sum::<f32>() as i32
    }

    /// RGBA8 pixels (brightness as grey, coverage as alpha).
    pub fn rgba(&self) -> Vec<u8> {
        self.alpha.iter().zip(&self.lum).flat_map(|(&a, &l)| [l, l, l, a]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_follow_samp_table() {
        assert_eq!(heights(800, 0), [14, 12, 20, 18, 12, 20, 24, 20, 22]);
        assert_eq!(heights(1024, 0)[..5], [16, 14, 20, 18, 14]);
        assert_eq!(heights(1920, 0)[..4], [20, 18, 20, 18]);
        assert_eq!(heights(1280, 2)[..5], [20, 18, 24, 22, 18]);
        assert_eq!(heights(800, -9)[..4], [8, 6, 14, 12]);
        assert_eq!(slot(UiFont::SmallOutline), slot(UiFont::Small));
    }

    #[test]
    fn symbol_cmap_lookup() {
        // Liberation Sans has no symbol cmap.
        assert_eq!(symbol_glyph(FONT_TTF, 0xF047), None);
        assert_eq!(symbol_glyph(b"not a font", 0xF047), None);
        // A user's gtaweap3.ttf (never committed): `SA_TEST_GTAWEAP3=path`.
        if let Some(ttf) = std::env::var_os("SA_TEST_GTAWEAP3").and_then(|p| std::fs::read(p).ok()) {
            let g = symbol_glyph(&ttf, 0xF047).expect("G");
            assert_ne!(Some(g), symbol_glyph(&ttf, 0xF033));
            let a = FontAtlas::build(Faces { weapon: Some(&ttf), ..Faces::default() }, heights(800, 0));
            // The backdrop `G` is a filled square about as wide as tall.
            let g = a.glyph(6, outline::NONE, b'G');
            assert!(a.has(6) && g.rect[2] > 15 && g.rect[3] > 15, "{g:?}");
        }
    }

    #[test]
    fn atlas_holds_every_glyph_and_metrics_are_arial_like() {
        let a = FontAtlas::build(Faces::default(), heights(1280, 0));
        assert!(a.width.is_power_of_two() && a.height.is_power_of_two());
        assert!(a.height <= 1024, "atlas {}x{}", a.width, a.height);
        for c in (b'!'..=b'~').chain([0xE9, 0xDF]) {
            let g = a.glyph(0, outline::NONE, c);
            assert!(g.rect[2] > 0 && g.rect[3] > 0, "{c:?}");
            assert!(u32::from(g.rect[0] + g.rect[2]) <= a.width && u32::from(g.rect[1] + g.rect[3]) <= a.height);
        }
        assert_eq!(a.glyph(0, outline::NONE, b' ').rect[2], 0);
        // Arial Bold 16 px: "Hello" is about 38 px wide; the small font is narrower.
        let w = a.width(0, b"Hello");
        assert!((30..46).contains(&w), "{w}");
        assert!(a.width(1, b"Hello") < w);
        assert_eq!(a.line_height(2), 20);
        let (p, o) = (a.glyph(0, outline::NONE, b'H'), a.glyph(0, outline::ONE, b'H'));
        assert_eq!((o.rect[2], o.rect[3]), (p.rect[2] + 2, p.rect[3] + 2));
        assert_eq!(o.offset, (p.offset.0 - 1.0, p.offset.1 - 1.0));
        assert_eq!(o.advance, p.advance);
        let two = a.glyph(0, outline::TWO, b'H');
        assert_eq!((two.rect[2], two.offset.0), (p.rect[2] + 4, p.offset.0 - 2.0));
        assert_eq!(a.glyph(1, outline::TWO, b'H'), a.glyph(1, outline::ONE, b'H'));
        // 'H' has a vertical stem at its left edge: the pixel left of it is outline (black, opaque).
        let corner = |g: Glyph, dx: u16, dy: u16| {
            let i = (g.rect[1] + dy) as usize * a.width as usize + (g.rect[0] + dx) as usize;
            (a.alpha[i], a.lum[i])
        };
        let mid = o.rect[3] / 2;
        let row: Vec<(u8, u8)> = (0..o.rect[2]).map(|x| corner(o, x, mid)).collect();
        let first = row.iter().find(|(a, _)| *a > 200).unwrap();
        assert!(first.1 < 100, "{row:?}");
        assert!(row.iter().any(|&(a, l)| a == 255 && l == 255), "{row:?}");
        // Unknown bytes fall back to '?'; the symbol slots are empty without their files.
        assert_eq!(a.glyph(0, outline::NONE, 0x81), a.glyph(0, outline::NONE, b'?'));
        assert!(!a.has(5) && a.glyph(5, outline::NONE, b'G') == Glyph::default());
    }
}
