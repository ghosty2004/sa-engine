//! What the connect screen draws with: text in Liberation Sans (regular and bold, glyph atlases of samp-overlay built
//! once per UI scale) and shapes from small procedural images, stretched by the overlay's quads: rounded rectangles
//! as nine-slices, soft shadows, gradients, a vignette, icons. Everything is laid out in design units scaled to the
//! overlay ([`Painter::begin`]).

use sa_mod::{Image, Overlay, Quad, Rgba};
use samp_overlay::font::{FONT_REGULAR_TTF, FONT_TTF, Faces, FontAtlas, SLOTS, outline};

/// Text sizes, in design units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    Title,
    Heading,
    Body,
    Small,
    Tiny,
}

const SIZES: [f32; 5] = [34.0, 21.0, 15.0, 13.0, 11.0];
/// The smallest pixel size of each, so that small screens stay legible (the layout gives text room for it).
const MIN_PX: [f32; 5] = [24.0, 16.0, 13.0, 12.0, 10.5];

/// The pixel size of text `slot` at UI scale `scale`.
fn px(slot: usize, scale: f32) -> f32 {
    (SIZES[slot] * scale).max(MIN_PX[slot])
}

impl Size {
    fn slot(self) -> usize {
        self as usize
    }
}

/// Font weights.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    Regular,
    Bold,
}

/// The icons of the icon image, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Lock,
    Star,
    Search,
    Globe,
    Clock,
    Arrow,
    User,
    Refresh,
    Check,
}

const ICONS: usize = 9;
const ICON_PX: u32 = 64;

/// The two weights at one UI scale, and their overlay images once uploaded.
struct Fonts {
    scale: f32,
    regular: FontAtlas,
    bold: FontAtlas,
    images: Option<(Image, Image)>,
}

fn heights(scale: f32) -> [u32; SLOTS] {
    let mut h = [6u32; SLOTS];
    for (i, h) in h.iter_mut().enumerate().take(SIZES.len()) {
        *h = (px(i, scale) * 1.18).round().max(6.0) as u32;
    }
    h
}

impl Fonts {
    fn build(scale: f32) -> Fonts {
        let atlas = |ttf: &'static [u8]| {
            FontAtlas::build(
                Faces {
                    ui: Some(ttf),
                    ..Faces::default()
                },
                heights(scale),
            )
        };
        Fonts {
            scale,
            regular: atlas(FONT_REGULAR_TTF),
            bold: atlas(FONT_TTF),
            images: None,
        }
    }

    fn atlas(&self, w: Weight) -> &FontAtlas {
        match w {
            Weight::Regular => &self.regular,
            Weight::Bold => &self.bold,
        }
    }
}

/// The procedural images.
struct Shapes {
    round: Image,
    shadow: Image,
    gradient: Image,
    vignette: Image,
    disc: Image,
    icons: Image,
    button: Image,
}

/// Which image a quad samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Src {
    Plain,
    Round,
    Shadow,
    Gradient,
    Vignette,
    Disc,
    Icons,
    Button,
    Font(Weight),
}

/// The primary button's image: its size (the button's proportions) and colours from left to right.
const BUTTON: (u32, u32) = (512, 88);
const BUTTON_COLOURS: [u32; 2] = [0x5B8CFF, 0x8B5CF6];

/// The primary button: a rounded rectangle shading from one colour to the other, lighter at the top.
fn button_pixels() -> Vec<u8> {
    let (w, h) = (BUTTON.0 as f32, BUTTON.1 as f32);
    let r = h * 0.26;
    let [a, b] = BUTTON_COLOURS.map(|c| [(c >> 16) as f32, ((c >> 8) & 0xFF) as f32, (c & 0xFF) as f32]);
    let mut px = Vec::with_capacity((BUTTON.0 * BUTTON.1 * 4) as usize);
    for y in 0..BUTTON.1 {
        for x in 0..BUTTON.0 {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let t = fx / w;
            let light = 1.0 + 0.10 * (1.0 - fy / h);
            let alpha =
                (0.5 - rounded_box(fx, fy, (w / 2.0, h / 2.0), (w / 2.0, h / 2.0), r)).clamp(0.0, 1.0);
            for i in 0..3 {
                px.push(((a[i] + (b[i] - a[i]) * t) * light).min(255.0) as u8);
            }
            px.push((alpha * 255.0).round() as u8);
        }
    }
    px
}

/// Edge of a nine-slice image's corner, in texels (the image is twice that).
const ROUND_R: u32 = 64;
const SHADOW_R: u32 = 64;

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// White pixels with alpha `f(x, y)` (pixel centres).
fn mask(w: u32, h: u32, f: impl Fn(f32, f32) -> f32) -> Vec<u8> {
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let a = f(x as f32 + 0.5, y as f32 + 0.5).clamp(0.0, 1.0);
            px.extend_from_slice(&[255, 255, 255, (a * 255.0).round() as u8]);
        }
    }
    px
}

fn len(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

/// Distance to a segment, minus the half width `r`.
fn capsule(px: f32, py: f32, a: (f32, f32), b: (f32, f32), r: f32) -> f32 {
    let (bx, by) = (b.0 - a.0, b.1 - a.1);
    let (qx, qy) = (px - a.0, py - a.1);
    let h = ((qx * bx + qy * by) / (bx * bx + by * by)).clamp(0.0, 1.0);
    len(qx - bx * h, qy - by * h) - r
}

/// Distance to a ring of radius `r` and half width `t`.
fn ring(px: f32, py: f32, c: (f32, f32), r: f32, t: f32) -> f32 {
    (len(px - c.0, py - c.1) - r).abs() - t
}

/// Distance to a rounded box centred at `c`, half size `b`, radius `r`.
fn rounded_box(px: f32, py: f32, c: (f32, f32), b: (f32, f32), r: f32) -> f32 {
    let qx = (px - c.0).abs() - b.0 + r;
    let qy = (py - c.1).abs() - b.1 + r;
    len(qx.max(0.0), qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

/// Signed distance to the polygon `pts`.
fn polygon(px: f32, py: f32, pts: &[(f32, f32)]) -> f32 {
    let mut d = f32::MAX;
    let mut inside = false;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        d = d.min(capsule(px, py, a, b, 0.0));
        if (a.1 > py) != (b.1 > py) && px < (b.0 - a.0) * (py - a.1) / (b.1 - a.1) + a.0 {
            inside = !inside;
        }
    }
    if inside { -d } else { d }
}

/// Signed distance to a five-pointed star (outer radius `ro`, inner `ri`) centred at `c`, point up.
fn star(px: f32, py: f32, c: (f32, f32), ro: f32, ri: f32) -> f32 {
    let pts: Vec<(f32, f32)> = (0..10)
        .map(|i| {
            let a = std::f32::consts::PI * (i as f32) / 5.0 - std::f32::consts::FRAC_PI_2;
            let r = if i % 2 == 0 { ro } else { ri };
            (c.0 + r * a.cos(), c.1 + r * a.sin())
        })
        .collect();
    polygon(px, py, &pts)
}

/// The signed distance of icon `i` at texel (`x`, `y`) of its 64 x 64 cell.
fn icon_sdf(i: usize, x: f32, y: f32) -> f32 {
    let c = (32.0, 32.0);
    match i {
        // lock: a body and a shackle
        0 => {
            let body = rounded_box(x, y, (32.0, 41.0), (17.0, 13.0), 4.0);
            let shackle = ring(x, y, (32.0, 26.0), 10.5, 3.0).max(y - 28.0);
            let legs = capsule(x, y, (21.5, 26.0), (21.5, 32.0), 3.0).min(capsule(
                x,
                y,
                (42.5, 26.0),
                (42.5, 32.0),
                3.0,
            ));
            body.min(shackle).min(legs)
        }
        1 => star(x, y, (32.0, 34.0), 26.0, 11.0) - 1.0,
        2 => ring(x, y, (27.0, 27.0), 15.0, 3.5).min(capsule(x, y, (38.5, 38.5), (52.0, 52.0), 4.0)),
        3 => {
            let outer = ring(x, y, c, 23.0, 3.0);
            let meridian = (len((x - 32.0) / 10.0, (y - 32.0) / 23.0) - 1.0).abs() * 10.0 - 2.5;
            let equator = capsule(x, y, (10.0, 32.0), (54.0, 32.0), 2.5);
            outer
                .min(meridian.max(len(x - 32.0, y - 32.0) - 23.0))
                .min(equator)
        }
        4 => ring(x, y, c, 23.0, 3.0)
            .min(capsule(x, y, c, (32.0, 18.0), 3.0))
            .min(capsule(x, y, c, (43.0, 37.0), 3.0)),
        5 => capsule(x, y, (12.0, 32.0), (50.0, 32.0), 3.5)
            .min(capsule(x, y, (36.0, 18.0), (51.0, 32.0), 3.5))
            .min(capsule(x, y, (36.0, 46.0), (51.0, 32.0), 3.5)),
        6 => {
            let head = len(x - 32.0, y - 22.0) - 10.0;
            let body = rounded_box(x, y, (32.0, 50.0), (19.0, 13.0), 12.0).max(y - 56.0);
            head.min(body)
        }
        7 => {
            // an arc open at the top right, its arrowhead pointing down the right side
            let a = (y - 32.0).atan2(x - 32.0);
            let arc = ring(x, y, c, 17.0, 3.5).max(if a > -1.35 && a < -0.35 { 1.0 } else { -1.0 });
            arc.min(polygon(x, y, &[(40.0, 22.0), (57.0, 22.0), (48.5, 33.0)]))
        }
        _ => {
            capsule(x, y, (16.0, 34.0), (27.0, 45.0), 4.0).min(capsule(x, y, (27.0, 45.0), (49.0, 21.0), 4.0))
        }
    }
}

impl Shapes {
    fn new() -> Option<Shapes> {
        let img = |w: u32, h: u32, px: Vec<u8>| Image::new(w, h, true, &px).ok();
        let s = (ROUND_R * 2) as f32;
        let round = mask(ROUND_R * 2, ROUND_R * 2, |x, y| {
            0.5 - (len(x - s / 2.0, y - s / 2.0) - s / 2.0 + 0.5)
        });
        // a soft falloff from the centre outwards: drawn as a nine-slice around a card it is a blurred shadow
        let t = (SHADOW_R * 2) as f32;
        let shadow = mask(SHADOW_R * 2, SHADOW_R * 2, |x, y| {
            let d = len(x - t / 2.0, y - t / 2.0) / (t / 2.0);
            let a = 1.0 - smoothstep(0.0, 1.0, d);
            a * a
        });
        let gradient = mask(4, 256, |_, y| y / 256.0);
        let vignette = mask(256, 256, |x, y| {
            let d = len((x - 128.0) / 128.0, (y - 128.0) / 128.0);
            smoothstep(0.45, 1.35, d)
        });
        let disc = mask(64, 64, |x, y| 0.5 - (len(x - 32.0, y - 32.0) - 31.0));
        let icons = mask(ICON_PX * ICONS as u32, ICON_PX, |x, y| {
            let i = (x / ICON_PX as f32) as usize;
            0.5 - icon_sdf(i, x - (i as u32 * ICON_PX) as f32, y)
        });
        Some(Shapes {
            round: img(ROUND_R * 2, ROUND_R * 2, round)?,
            shadow: img(SHADOW_R * 2, SHADOW_R * 2, shadow)?,
            gradient: img(4, 256, gradient)?,
            vignette: img(256, 256, vignette)?,
            disc: img(64, 64, disc)?,
            icons: img(ICON_PX * ICONS as u32, ICON_PX, icons)?,
            button: img(BUTTON.0, BUTTON.1, button_pixels())?,
        })
    }
}

/// A rectangle in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, p: (f32, f32)) -> bool {
        p.0 >= self.x && p.0 < self.x + self.w && p.1 >= self.y && p.1 < self.y + self.h
    }

    pub fn inset(&self, dx: f32, dy: f32) -> Rect {
        Rect::new(
            self.x + dx,
            self.y + dy,
            (self.w - 2.0 * dx).max(0.0),
            (self.h - 2.0 * dy).max(0.0),
        )
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
}

/// `0xRRGGBB` with alpha `a` (0..=1).
pub fn rgb(c: u32, a: f32) -> Rgba {
    Rgba((c << 8) | (a.clamp(0.0, 1.0) * 255.0).round() as u32)
}

/// Text as the atlas's code page (Latin-1; anything else `?`).
pub fn latin1(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
        .collect()
}

/// The screen's drawing state: the fonts, the shapes and the quads of the frame, batched per image.
#[derive(Default)]
pub struct Painter {
    fonts: Vec<Fonts>,
    shapes: Option<Shapes>,
    shapes_failed: bool,
    /// The UI scale of this frame (pixels per design unit) and the fonts used for it.
    pub scale: f32,
    current: usize,
    batch: Vec<Quad>,
    batch_src: Option<Src>,
}

impl Painter {
    /// Builds the fonts for the UI scales of overlay sizes `sizes` (rasterising is the expensive part: call it where
    /// the host allows the most work).
    pub fn prepare(&mut self, sizes: &[(u32, u32)]) {
        for &(w, h) in sizes {
            let s = scale_for(w, h);
            if !self.fonts.iter().any(|f| (f.scale - s).abs() < 0.01) {
                self.fonts.push(Fonts::build(s));
            }
        }
    }

    /// Starts a frame on an overlay of `size`: picks the fonts nearest to its scale and uploads what is missing.
    pub fn begin(&mut self, size: (u32, u32)) -> bool {
        self.scale = scale_for(size.0, size.1);
        if self.fonts.is_empty() {
            self.prepare(&[size]);
        }
        let s = self.scale;
        self.current = (0..self.fonts.len())
            .min_by(|&a, &b| {
                (self.fonts[a].scale - s)
                    .abs()
                    .total_cmp(&(self.fonts[b].scale - s).abs())
            })
            .unwrap_or(0);
        let f = &mut self.fonts[self.current];
        if f.images.is_none() {
            let up = |a: &FontAtlas| Image::new(a.width, a.height, true, &a.rgba()).ok();
            f.images = up(&f.regular).zip(up(&f.bold));
            // only the atlases of the scale in use stay uploaded (the images share one budget with samp's UI)
            for (i, other) in self.fonts.iter_mut().enumerate() {
                if i != self.current {
                    other.images = None;
                }
            }
        }
        if self.shapes.is_none() && !self.shapes_failed {
            self.shapes = Shapes::new();
            self.shapes_failed = self.shapes.is_none();
        }
        self.fonts[self.current].images.is_some() && self.shapes.is_some()
    }

    fn push(&mut self, o: &mut Overlay, src: Src, q: Quad) {
        if self.batch_src != Some(src) {
            self.flush(o);
            self.batch_src = Some(src);
        }
        self.batch.push(q);
    }

    /// Draws the batched quads.
    pub fn flush(&mut self, o: &mut Overlay) {
        let Some(src) = self.batch_src.take() else { return };
        if self.batch.is_empty() {
            return;
        }
        let image = match (src, &self.shapes) {
            (Src::Plain, _) => None,
            (Src::Font(w), _) => self.fonts[self.current]
                .images
                .as_ref()
                .map(|(r, b)| if w == Weight::Bold { b } else { r }),
            (_, None) => None,
            (Src::Round, Some(s)) => Some(&s.round),
            (Src::Shadow, Some(s)) => Some(&s.shadow),
            (Src::Gradient, Some(s)) => Some(&s.gradient),
            (Src::Vignette, Some(s)) => Some(&s.vignette),
            (Src::Disc, Some(s)) => Some(&s.disc),
            (Src::Icons, Some(s)) => Some(&s.icons),
            (Src::Button, Some(s)) => Some(&s.button),
        };
        for chunk in self.batch.chunks(4096) {
            let _ = o.quads(image, chunk);
        }
        self.batch.clear();
    }

    /// Cuts what follows to `r` (`None`: no clip).
    pub fn clip(&mut self, o: &mut Overlay, r: Option<Rect>) {
        self.flush(o);
        let _ = o.clip(r.map(|r| [r.x, r.y, r.w, r.h]));
    }

    /// A plain rectangle.
    pub fn rect(&mut self, o: &mut Overlay, r: Rect, c: Rgba) {
        self.push(
            o,
            Src::Plain,
            Quad {
                rect: [r.x, r.y, r.w, r.h],
                uv: [0.0; 4],
                colour: c,
            },
        );
    }

    /// A rectangle shaded from `top` to `bottom`.
    pub fn gradient(&mut self, o: &mut Overlay, r: Rect, top: Rgba, bottom: Rgba) {
        self.rect(o, r, top);
        // the gradient image's alpha runs 0 -> 1 downwards: `bottom` fades in over `top`
        let q = Quad {
            rect: [r.x, r.y, r.w, r.h],
            uv: [0.0, 0.0, 1.0, 1.0],
            colour: bottom,
        };
        self.push(o, Src::Gradient, q);
    }

    /// A rectangle fading from transparent at the top to `c` at the bottom (`up`: the other way).
    pub fn fade(&mut self, o: &mut Overlay, r: Rect, c: Rgba, up: bool) {
        let uv = if up {
            [0.0, 1.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0, 1.0]
        };
        self.push(
            o,
            Src::Gradient,
            Quad {
                rect: [r.x, r.y, r.w, r.h],
                uv,
                colour: c,
            },
        );
    }

    /// Darkens the edges of `r`.
    pub fn vignette(&mut self, o: &mut Overlay, r: Rect, c: Rgba) {
        self.push(
            o,
            Src::Vignette,
            Quad {
                rect: [r.x, r.y, r.w, r.h],
                uv: [0.0, 0.0, 1.0, 1.0],
                colour: c,
            },
        );
    }

    fn nine(&mut self, o: &mut Overlay, src: Src, r: Rect, corner: f32, c: Rgba) {
        let k = corner.min(r.w / 2.0).min(r.h / 2.0).max(0.0);
        if k <= 0.0 {
            return self.rect(o, r, c);
        }
        let xs = [r.x, r.x + k, r.right() - k, r.right()];
        let ys = [r.y, r.y + k, r.bottom() - k, r.bottom()];
        let us = [0.0, 0.5, 0.5, 1.0];
        for j in 0..3 {
            for i in 0..3 {
                let (w, h) = (xs[i + 1] - xs[i], ys[j + 1] - ys[j]);
                if w <= 0.0 || h <= 0.0 {
                    continue;
                }
                let q = Quad {
                    rect: [xs[i], ys[j], w, h],
                    uv: [us[i], us[j], us[i + 1], us[j + 1]],
                    colour: c,
                };
                self.push(o, src, q);
            }
        }
    }

    /// A rectangle with rounded corners of radius `radius` pixels.
    pub fn round(&mut self, o: &mut Overlay, r: Rect, radius: f32, c: Rgba) {
        self.nine(o, Src::Round, r, radius, c);
    }

    /// A rounded rectangle's outline `t` pixels wide, over `fill` (the inside).
    pub fn outlined(&mut self, o: &mut Overlay, r: Rect, radius: f32, t: f32, line: Rgba, fill: Rgba) {
        self.round(o, r, radius, line);
        self.round(o, r.inset(t, t), (radius - t).max(0.0), fill);
    }

    /// A soft shadow `blur` pixels wide around `r` (corner radius `radius`).
    pub fn shadow(&mut self, o: &mut Overlay, r: Rect, radius: f32, blur: f32, c: Rgba) {
        let s = Rect::new(
            r.x - blur,
            r.y - blur + blur * 0.35,
            r.w + 2.0 * blur,
            r.h + 2.0 * blur,
        );
        self.nine(o, Src::Shadow, s, blur + radius, c);
    }

    /// The primary button's face, stretched over `r` (made in the button's proportions) and tinted with `c`.
    pub fn button(&mut self, o: &mut Overlay, r: Rect, c: Rgba) {
        self.push(
            o,
            Src::Button,
            Quad {
                rect: [r.x, r.y, r.w, r.h],
                uv: [0.0, 0.0, 1.0, 1.0],
                colour: c,
            },
        );
    }

    /// A filled circle.
    pub fn disc(&mut self, o: &mut Overlay, cx: f32, cy: f32, radius: f32, c: Rgba) {
        let q = Quad {
            rect: [cx - radius, cy - radius, radius * 2.0, radius * 2.0],
            uv: [0.0, 0.0, 1.0, 1.0],
            colour: c,
        };
        self.push(o, Src::Disc, q);
    }

    /// Icon `i` in a square of `size` pixels at (`x`, `y`).
    pub fn icon(&mut self, o: &mut Overlay, i: Icon, x: f32, y: f32, size: f32, c: Rgba) {
        let n = ICONS as f32;
        let u0 = i as usize as f32 / n;
        let q = Quad {
            rect: [x, y, size, size],
            uv: [u0, 0.0, u0 + 1.0 / n, 1.0],
            colour: c,
        };
        self.push(o, Src::Icons, q);
    }

    /// How much the current atlas's glyphs of `size` are scaled.
    fn k(&self, size: Size) -> f32 {
        px(size.slot(), self.scale) / px(size.slot(), self.fonts[self.current].scale)
    }

    /// The pixel height of a line of `size`.
    pub fn line_height(&self, size: Size) -> f32 {
        let f = &self.fonts[self.current];
        f.regular.line_height(size.slot()) as f32 * self.k(size)
    }

    /// The width of `text` in pixels.
    pub fn measure(&self, size: Size, w: Weight, text: &[u8]) -> f32 {
        let f = &self.fonts[self.current];
        f.atlas(w).width(size.slot(), text) as f32 * self.k(size)
    }

    /// Draws `text` with its line's top at `y`; returns its width.
    #[allow(clippy::too_many_arguments)]
    pub fn text(
        &mut self,
        o: &mut Overlay,
        x: f32,
        y: f32,
        size: Size,
        w: Weight,
        c: Rgba,
        text: &[u8],
    ) -> f32 {
        let f = &self.fonts[self.current];
        let k = self.k(size);
        let atlas = f.atlas(w);
        let (aw, ah) = (atlas.width as f32, atlas.height as f32);
        // whole pixels at the atlas's own scale keep the glyphs sharp
        let snap = |v: f32| if (k - 1.0).abs() < 0.02 { v.round() } else { v };
        let mut pen = 0.0;
        let mut quads = Vec::with_capacity(text.len());
        for &b in text {
            let g = atlas.glyph(size.slot(), outline::NONE, b);
            if g.rect[2] > 0 {
                let [gx, gy, gw, gh] = g.rect.map(f32::from);
                let rect = [
                    snap(x + pen + g.offset.0 * k),
                    snap(y + g.offset.1 * k),
                    gw * k,
                    gh * k,
                ];
                quads.push(Quad {
                    rect,
                    uv: [gx / aw, gy / ah, (gx + gw) / aw, (gy + gh) / ah],
                    colour: c,
                });
            }
            pen += g.advance * k;
        }
        for q in quads {
            self.push(o, Src::Font(w), q);
        }
        pen
    }

    /// Draws `text` cut to `max` pixels with `...`; returns its width.
    #[allow(clippy::too_many_arguments)]
    pub fn text_fit(
        &mut self,
        o: &mut Overlay,
        x: f32,
        y: f32,
        size: Size,
        w: Weight,
        c: Rgba,
        text: &[u8],
        max: f32,
    ) -> f32 {
        if self.measure(size, w, text) <= max {
            return self.text(o, x, y, size, w, c, text);
        }
        let dots = self.measure(size, w, b"...");
        let mut n = text.len();
        while n > 0 && self.measure(size, w, &text[..n]) + dots > max {
            n -= 1;
        }
        let mut t = text[..n].trim_ascii_end().to_vec();
        t.extend_from_slice(b"...");
        self.text(o, x, y, size, w, c, &t)
    }
}

/// Pixels per design unit on an overlay of `w` x `h`: the screen is laid out on 1280 x 760 units, never below
/// 0.6 (800 x 600 stays legible: the card then fills the screen).
pub fn scale_for(w: u32, h: u32) -> f32 {
    let s = (w as f32 / 1280.0).min(h as f32 / 760.0);
    s.max(0.6)
}
