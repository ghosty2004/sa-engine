//! samp-rs's UI drawn as textured quads, for games that offer a 2D overlay of images instead of Direct3D 9 and
//! D3DX: [`QuadUi`] implements `sa_samp_api::ui::UiBackend` on a [`Canvas`]. samp's D3DX fonts become one glyph
//! atlas ([`font`]), `DrawText` its layout on the atlas ([`draw`]), the DXUT skin and the cursor an image of the
//! user's `sampgui.png` and `mouse.png` ([`skin_image`]; without them flat rectangles of the same colours and the
//! platform's cursor). The client's pass collects the draws; the host
//! calls [`QuadUi::flush`] once per frame to turn them into the canvas's quads.
//!
//! Text textdraws go to the game's font where the canvas offers it ([`Canvas::game_text`]), else the UI font stands
//! in at the textdraw's letter size. Sprite textdraws are the canvas's game textures ([`Canvas::game_sprite`]);
//! model previews take the text path.

pub mod draw;
pub mod font;

use sa_samp_api::ui::{ColourQuad, TextdrawFont, UiBackend, UiFont, UiMetrics, UiRect, UiSprite, dt};

use self::draw::{Cmd, DrawList};
use self::font::{Faces, FontAtlas, MASK_BYTE, outline, slot};

/// A quad of an image: the screen rectangle `[x, y, w, h]` in pixels, the image's texture coordinates
/// `[u0, v0, u1, v1]` and a tint `0xAARRGGBB`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    /// Left, top, width, height.
    pub rect: [f32; 4],
    /// Texture coordinates (0..=1).
    pub uv: [f32; 4],
    /// Tint `0xAARRGGBB` (multiplies the image).
    pub colour: u32,
}

/// Where [`QuadUi`] draws: a 2D overlay of images and quads over the game's frame.
pub trait Canvas {
    /// The overlay's size in pixels.
    fn size(&self) -> (i32, i32);
    /// Uploads a `width` x `height` RGBA8 image (rows from the top, straight alpha): its id, `None` when the canvas
    /// refuses it. `smooth`: sampled with linear filtering over a box-filtered mip chain (D3DX's textures, scaled
    /// skin controls), else nearest texels (the glyphs, drawn 1:1).
    fn upload(&mut self, width: u32, height: u32, smooth: bool, rgba: &[u8]) -> Option<u32>;
    /// Deletes image `id`.
    fn release(&mut self, id: u32);
    /// Draws `quads` of image `image` (`None`: plain colour), in order.
    fn quads(&mut self, image: Option<u32>, quads: &[Quad]);
    /// Prints a text textdraw with the game's own font (`CFont`); false when the canvas cannot, and the UI font
    /// stands in.
    fn game_text(&mut self, font: &TextdrawFont, text: &[u8]) -> bool {
        let _ = (font, text);
        false
    }
    /// Draws a sprite textdraw's game texture (`text` is `txd:name`, [`samp_ui::textdraw::sprite_source`]) into
    /// `rect` (`[left, top, right, bottom]`) tinted with `colour` (`CRGBA`); false when the canvas has no such
    /// texture (the textdraw then takes the text path, as samp does).
    fn game_sprite(&mut self, text: &[u8], rect: [f32; 4], colour: u32) -> bool {
        let _ = (text, rect, colour);
        false
    }
}

/// The size of the skin image: `sampgui.png` (512 x 512) at the top, `mouse.png` (32 x 32, hotspot 0, 0) at
/// (0, 512).
pub const SKIN_SIZE: (u32, u32) = (512, 1024);
/// Where `mouse.png` sits in the skin image.
const CURSOR_SRC: [f32; 4] = [0.0, 512.0, 32.0, 544.0];

/// The skin image ([`SKIN_SIZE`]) of `sampgui.png` and `mouse.png` decoded to RGBA8 (width, height, pixels):
/// `None` when `sampgui.png` is not 512 x 512; without `mouse.png` its area stays empty.
pub fn skin_image(gui: (u32, u32, &[u8]), mouse: Option<(u32, u32, &[u8])>) -> Option<Vec<u8>> {
    let (w, h, px) = gui;
    if (w, h) != (512, 512) || px.len() != (w * h * 4) as usize {
        return None;
    }
    let mut img = vec![0u8; (SKIN_SIZE.0 * SKIN_SIZE.1 * 4) as usize];
    img[..px.len()].copy_from_slice(px);
    if let Some((mw, mh, m)) = mouse.filter(|(mw, mh, m)| m.len() == (mw * mh * 4) as usize) {
        for y in 0..mh.min(32) {
            for x in 0..mw.min(32) {
                let s = ((y * mw + x) * 4) as usize;
                let d = (((512 + y) * SKIN_SIZE.0 + x) * 4) as usize;
                img[d..d + 4].copy_from_slice(&m[s..s + 4]);
            }
        }
    }
    Some(img)
}

/// What a glyph-atlas or skin image is on the canvas.
#[derive(Debug, Default)]
enum Upload {
    #[default]
    Pending,
    Done(u32),
    Failed,
}

/// A world point's place on the screen with the game's camera, for nametags and labels: `(x, y)` in pixels, `None`
/// off screen or behind the camera.
pub type Projector = Box<dyn Fn([f32; 3]) -> Option<(f32, f32)>>;

/// samp's UI backend over a [`Canvas`].
#[derive(Default)]
pub struct QuadUi {
    atlas: Option<FontAtlas>,
    atlas_image: Upload,
    /// `sampgui.png` as RGBA8 ([`SKIN_SIZE`]).
    skin: Option<Vec<u8>>,
    skin_image: Upload,
    /// The UI font file the user has (Arial Bold), `None` = the committed Liberation Sans Bold.
    font_file: Option<Vec<u8>>,
    /// `gtaweap3.ttf` (death window icons).
    weapon_file: Option<Vec<u8>>,
    /// `sampaux3.ttf` (AFK icon).
    aux_file: Option<Vec<u8>>,
    list: DrawList,
    /// The canvas size at the last flush.
    screen: (i32, i32),
    cursor: (i32, i32),
    /// samp's input lock and cursor as the client set them: keyboard, mouse, cursor shown.
    input: (bool, bool, bool),
    projector: Option<Projector>,
    /// Images the next flush deletes (the atlas was rebuilt).
    stale: Vec<u32>,
    /// Sprite textdraw texts and whether the canvas had their texture.
    sprites: std::collections::HashMap<Vec<u8>, bool>,
}

impl std::fmt::Debug for QuadUi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuadUi").field("atlas_image", &self.atlas_image).field("screen", &self.screen).finish()
    }
}

/// Bytes of UTF-16 text in the atlas's code page (Latin-1; DXUT's password mask at [`MASK_BYTE`]).
fn narrow(text: &[u16]) -> Vec<u8> {
    text.iter()
        .map(|&c| match c {
            0x25CF => MASK_BYTE,
            0..=0xFF => c as u8,
            _ => b'?',
        })
        .collect()
}

/// `0xAARRGGBB` of a `CRGBA` as stored (R, G, B, A in memory order, read as a little-endian word).
fn crgba(c: u32) -> u32 {
    let [r, g, b, a] = c.to_le_bytes();
    u32::from_be_bytes([a, r, g, b])
}

impl QuadUi {
    /// A backend whose screen is `size` until the first flush.
    pub fn new(size: (i32, i32)) -> Self {
        Self { screen: size, ..Self::default() }
    }

    /// samp's skin ([`skin_image`]); ignored when the size does not match.
    pub fn set_skin(&mut self, rgba: Vec<u8>) {
        if rgba.len() == (SKIN_SIZE.0 * SKIN_SIZE.1 * 4) as usize {
            if let Upload::Done(id) = self.skin_image {
                self.stale.push(id);
            }
            self.skin = Some(rgba);
            self.skin_image = Upload::Pending;
        }
    }

    /// Uses `ttf` (the user's Arial Bold, as samp's D3DX "Arial") for the UI fonts. Ignored when it does not parse.
    pub fn set_font(&mut self, ttf: Vec<u8>) {
        if font::parses(&ttf) {
            self.font_file = Some(ttf);
            self.drop_atlas();
        }
    }

    /// The symbol fonts samp draws with: `gtaweap3.ttf` (death window) and `sampaux3.ttf` (AFK icon).
    pub fn set_symbol_fonts(&mut self, weapon: Option<Vec<u8>>, aux: Option<Vec<u8>>) {
        self.weapon_file = weapon.filter(|t| font::parses(t));
        self.aux_file = aux.filter(|t| font::parses(t));
        self.drop_atlas();
    }

    /// How world points reach the screen (nametags, labels); without one they are not drawn.
    pub fn set_projector(&mut self, p: Projector) {
        self.projector = Some(p);
    }

    /// The mouse position (client pixels).
    pub fn set_cursor(&mut self, x: i32, y: i32) {
        self.cursor = (x, y);
    }

    /// samp's input lock as the client set it: (keyboard, mouse, cursor shown).
    pub fn input_state(&self) -> (bool, bool, bool) {
        self.input
    }

    /// The draws collected since the last flush.
    pub fn list(&self) -> &DrawList {
        &self.list
    }

    fn drop_atlas(&mut self) {
        self.atlas = None;
        if let Upload::Done(id) = std::mem::take(&mut self.atlas_image) {
            self.stale.push(id);
        }
    }

    /// Builds the atlas for video width `w` and `/fontsize` `fs` when the sizes changed.
    fn ensure_atlas(&mut self, w: i32, fs: i32) -> bool {
        let heights = font::heights(w, fs);
        if self.atlas.as_ref().is_some_and(|a| a.heights() == heights) {
            return false;
        }
        self.drop_atlas();
        let faces =
            Faces { ui: self.font_file.as_deref(), weapon: self.weapon_file.as_deref(), aux: self.aux_file.as_deref() };
        self.atlas = Some(FontAtlas::build(faces, heights));
        true
    }

    /// Uploads what is missing and draws what the pass collected, then forgets it. Nothing is drawn when `hide`
    /// (samp draws no UI while the game's menu is up).
    pub fn flush(&mut self, canvas: &mut dyn Canvas, hide: bool) {
        self.screen = canvas.size();
        for id in self.stale.drain(..) {
            canvas.release(id);
        }
        let list = std::mem::take(&mut self.list);
        if hide {
            return;
        }
        if self.atlas.is_none() {
            self.ensure_atlas(self.screen.0, 0);
        }
        let Some(atlas) = self.atlas.as_ref() else { return };
        if let Upload::Pending = self.atlas_image {
            self.atlas_image =
                canvas.upload(atlas.width, atlas.height, false, &atlas.rgba()).map_or(Upload::Failed, Upload::Done);
        }
        if let (Upload::Pending, Some(skin)) = (&self.skin_image, &self.skin) {
            self.skin_image = canvas.upload(SKIN_SIZE.0, SKIN_SIZE.1, true, skin).map_or(Upload::Failed, Upload::Done);
        }
        let glyphs = match self.atlas_image {
            Upload::Done(id) => Some(id),
            _ => None,
        };
        let skin = match self.skin_image {
            Upload::Done(id) => Some(id),
            _ => None,
        };
        let mut batch = Batch { canvas, image: None, quads: Vec::new() };
        let skin_quad = |src: &[f32; 4], dst: [f32; 4], colour: u32| {
            let (w, h) = (SKIN_SIZE.0 as f32, SKIN_SIZE.1 as f32);
            let uv = [src[0] / w, src[1] / h, src[2] / w, src[3] / h];
            Quad { rect: [dst[0], dst[1], dst[2] - dst[0], dst[3] - dst[1]], uv, colour }
        };
        for cmd in &list.cmds {
            match cmd {
                Cmd::Rect { rect, colour } => batch.rect(*rect, *colour),
                Cmd::Text { x, y, slot, colour, text, outline } => {
                    if let Some(id) = glyphs {
                        text_quads(&mut batch, atlas, id, (*x, *y), *slot, *outline, text, *colour, 1.0, 1.0);
                    }
                }
                Cmd::Skin { src, dst, colour } => match skin {
                    Some(id) => batch.push(Some(id), skin_quad(src, *dst, *colour)),
                    None => batch.rect(*dst, *colour),
                },
                Cmd::Sprite { text, rect, colour } => {
                    let ok = batch.canvas.game_sprite(text, *rect, *colour);
                    self.sprites.insert(text.clone(), ok);
                }
                Cmd::GameFont { font, text } => {
                    if batch.canvas.game_text(font, text) {
                        continue;
                    }
                    if let Some(id) = glyphs {
                        textdraw(&mut batch, atlas, id, font, text);
                    }
                }
            }
        }
        if let (true, Some(id)) = (self.input.2, skin) {
            let (x, y) = (self.cursor.0 as f32, self.cursor.1 as f32);
            batch.push(Some(id), skin_quad(&CURSOR_SRC, [x, y, x + 32.0, y + 32.0], 0xFFFF_FFFF));
        }
        batch.finish();
    }

    /// Whether the UI draws its own cursor (samp's `mouse.png`) while the cursor shows; else the host should show
    /// the platform's.
    pub fn draws_cursor(&self) -> bool {
        self.skin.is_some() && !matches!(self.skin_image, Upload::Failed)
    }
}

/// Consecutive quads of one image, drawn in one canvas call.
struct Batch<'a> {
    canvas: &'a mut dyn Canvas,
    image: Option<u32>,
    quads: Vec<Quad>,
}

impl Batch<'_> {
    fn push(&mut self, image: Option<u32>, q: Quad) {
        if image != self.image {
            self.finish();
            self.image = image;
        }
        self.quads.push(q);
    }

    fn rect(&mut self, r: [f32; 4], colour: u32) {
        self.push(None, Quad { rect: [r[0], r[1], r[2] - r[0], r[3] - r[1]], uv: [0.0, 0.0, 1.0, 1.0], colour });
    }

    fn finish(&mut self) {
        if !self.quads.is_empty() {
            self.canvas.quads(self.image, &self.quads);
            self.quads.clear();
        }
    }
}

/// The glyph quads of `text` in slot `si` with its pen at `pen` (top of the line), the glyphs scaled by `sx` / `sy`.
#[allow(clippy::too_many_arguments)]
fn text_quads(
    b: &mut Batch,
    a: &FontAtlas,
    image: u32,
    pen: (f32, f32),
    si: usize,
    kind: u8,
    text: &[u8],
    colour: u32,
    sx: f32,
    sy: f32,
) {
    let (aw, ah) = (a.width as f32, a.height as f32);
    // whole pixels: the glyphs are sampled 1:1 (nearest)
    let (mut x, top) = (pen.0.round(), pen.1.round());
    for &ch in text {
        let g = a.glyph(si, kind, ch);
        let [gx, gy, gw, gh] = g.rect.map(f32::from);
        if gw > 0.0 && gh > 0.0 {
            let rect = [x + (g.offset.0 * sx).round(), top + (g.offset.1 * sy).round(), gw * sx, gh * sy];
            b.push(Some(image), Quad { rect, uv: [gx / aw, gy / ah, (gx + gw) / aw, (gy + gh) / ah], colour });
        }
        x += g.advance * sx;
    }
}

/// The game font's line height at `CFont` scale 1 in pixels: `GetHeight` is `scale.y * 32 / 2 + 2 * scale.y`.
const GAME_LINE: f32 = 18.0;

/// A text textdraw in the UI font: the main font scaled to the textdraw's letter height, its width by the letter
/// width (0.5 of the height is the game font's usual proportion), with its box, shadow or outline, alignment and
/// wrap width; `~n~` breaks the line, other `~x~` codes are dropped.
fn textdraw(b: &mut Batch, a: &FontAtlas, image: u32, f: &TextdrawFont, text: &[u8]) {
    let si = slot(UiFont::Main);
    let line = f.scale.1 * GAME_LINE;
    if line <= 0.0 {
        return;
    }
    let sy = line / a.line_height(si) as f32;
    let sx = sy * (f.scale.0 / f.scale.1 / 0.5).clamp(0.2, 4.0);
    let mut lines: Vec<Vec<u8>> = vec![Vec::new()];
    let mut i = 0;
    while i < text.len() {
        if text[i] == b'~'
            && let Some(end) = text[i + 1..].iter().position(|&c| c == b'~')
        {
            if &text[i + 1..i + 1 + end] == b"n" {
                lines.push(Vec::new());
            }
            i += end + 2;
            continue;
        }
        lines.last_mut().expect("one line").push(text[i]);
        i += 1;
    }
    let width = |l: &[u8]| a.width(si, l) as f32 * sx;
    let widest = lines.iter().map(|l| width(l)).fold(0.0, f32::max);
    let left_of = |w: f32| match f.orientation {
        0 => f.x - w / 2.0,
        2 => f.x - w,
        _ => f.x,
    };
    if f.background {
        let (l, r) = match f.orientation {
            0 => (f.x - f.centre_size / 2.0, f.x + f.centre_size / 2.0),
            2 => (f.wrap_x.min(f.x - widest), f.x),
            _ => (f.x, f.wrap_x.max(f.x + widest)),
        };
        b.rect([l, f.y, r, f.y + line * lines.len() as f32], crgba(f.background_colour));
    }
    let colour = crgba(f.colour);
    let drop = crgba(f.drop_colour);
    for (k, l) in lines.iter().enumerate() {
        let pen = (left_of(width(l)), f.y + k as f32 * line);
        if f.outline > 0 {
            let o = f32::from(f.outline);
            for (dx, dy) in [(-o, 0.0), (o, 0.0), (0.0, -o), (0.0, o)] {
                text_quads(b, a, image, (pen.0 + dx, pen.1 + dy), si, outline::NONE, l, drop, sx, sy);
            }
        } else if f.shadow > 0 {
            let s = f32::from(f.shadow);
            text_quads(b, a, image, (pen.0 + s, pen.1 + s), si, outline::NONE, l, drop, sx, sy);
        }
        text_quads(b, a, image, pen, si, outline::NONE, l, colour, sx, sy);
    }
}

impl UiBackend for QuadUi {
    fn video_size(&self) -> (i32, i32) {
        self.screen
    }

    fn client_size(&self) -> (i32, i32) {
        self.screen
    }

    fn cursor_pos(&self) -> (i32, i32) {
        self.cursor
    }

    fn fonts(&mut self, _face: &[u8], _weight: u32, size: i32, force: bool) -> bool {
        if force {
            self.drop_atlas();
        }
        self.ensure_atlas(self.screen.0, size)
    }

    fn metrics(&self) -> UiMetrics {
        let Some(a) = self.atlas.as_ref() else { return UiMetrics::default() };
        let (icon_w, icon_h) = if a.has(slot(UiFont::DeathBackdrop)) {
            let si = slot(UiFont::DeathBackdrop);
            (a.width(si, b"G"), a.line_height(si))
        } else {
            // Without GTAWEAPON3: its aux + 12 cell and the glyph's measured width (31 px at 24).
            let h = a.line_height(slot(UiFont::DeathNames)) + 12;
            (((1.3 * h as f32).round()) as i32, h)
        };
        UiMetrics {
            main_h: a.line_height(0),
            small_h: a.line_height(1),
            gui0_h: a.line_height(2),
            gui1_h: a.line_height(3),
            nick_w: a.width(slot(UiFont::DeathNames), samp_ui::deathwindow::NICK_SAMPLE),
            icon_w,
            icon_h,
            timestamp_w: a.width(0, samp_ui::chat::TIMESTAMP_SAMPLE),
        }
    }

    fn measure(&self, f: UiFont, text: &[u8], count: i32, rect: UiRect, flags: u32, _colour: u32) -> UiRect {
        let Some(a) = self.atlas.as_ref() else { return rect };
        let text = if count < 0 { text } else { &text[..(count as usize).min(text.len())] };
        draw::layout(a, slot(f), text, rect, flags).1
    }

    fn sprite_begin(&mut self, _sprite: UiSprite, _flags: u32) {}

    fn sprite_end(&mut self, _sprite: UiSprite) {}

    fn draw_text(
        &mut self,
        f: UiFont,
        _sprite: Option<UiSprite>,
        text: &[u8],
        count: i32,
        rect: &mut UiRect,
        flags: u32,
        colour: u32,
    ) -> i32 {
        let text = if count < 0 { text } else { &text[..(count as usize).min(text.len())] };
        let Some(a) = self.atlas.as_ref() else { return 0 };
        if flags & dt::CALCRECT != 0 {
            *rect = draw::layout(a, slot(f), text, *rect, flags).1;
            return rect.bottom - rect.top;
        }
        let e = self.list.text(a, slot(f), text, *rect, flags, colour, None, outline::NONE);
        e.bottom - e.top
    }

    fn draw_text_coloured(
        &mut self,
        f: UiFont,
        _sprite: UiSprite,
        text: &[u8],
        rect: &mut UiRect,
        flags: u32,
        colour: u32,
        glyphs: &[u32],
    ) {
        let Some(a) = self.atlas.as_ref() else { return };
        self.list.text(a, slot(f), text, *rect, flags, colour, Some(glyphs), outline::NONE);
    }

    /// The outline passes are the atlas's pre-outlined glyphs: one draw instead of five (or nine).
    fn draw_text_outlined(
        &mut self,
        _sprite: UiSprite,
        f: UiFont,
        _outline_font: UiFont,
        stripped: &[u8],
        rect: UiRect,
        flags: u32,
        colour: u32,
        glyphs: &[u32],
        offsets: &[(i32, i32)],
    ) {
        let Some(a) = self.atlas.as_ref() else { return };
        let kind = match offsets.len() {
            0 => outline::NONE,
            8 => outline::TWO,
            _ => outline::ONE,
        };
        self.list.text(a, slot(f), stripped, rect, flags | dt::NOCLIP, colour, Some(glyphs), kind);
    }

    fn dialog_begin(&mut self, rect: UiRect, background: u32) {
        if background >> 24 != 0 {
            self.list.rect([rect.left, rect.top, rect.right, rect.bottom].map(|v| v as f32), background);
        }
    }

    fn dialog_end(&mut self) {}

    fn gui_sprite(&mut self, src: UiRect, dst: UiRect, colour: u32) {
        if colour >> 24 == 0 {
            return;
        }
        let dst = [dst.left, dst.top, dst.right, dst.bottom].map(|v| v as f32);
        let src = [src.left, src.top, src.right, src.bottom].map(|v| v as f32);
        self.list.cmds.push(Cmd::Skin { src, dst, colour });
    }

    fn gui_text(&mut self, f: UiFont, text: &[u8], rect: UiRect, format: u32, colour: u32) {
        if colour >> 24 == 0 {
            return;
        }
        let stripped = samp_ui::colour::strip_codes(text);
        let glyphs: Vec<u32> = samp_ui::colour::glyph_colours(text, colour).iter().map(|g| g.colour).collect();
        let Some(a) = self.atlas.as_ref() else { return };
        self.list.text(a, slot(f), &stripped, rect, format, colour, Some(&glyphs), outline::NONE);
    }

    fn gui_text_wide(&mut self, f: UiFont, text: &[u16], count: i32, rect: UiRect, format: u32, colour: u32) {
        if colour >> 24 == 0 {
            return;
        }
        let text = if count < 0 { text } else { &text[..(count as usize).min(text.len())] };
        let bytes = narrow(text);
        let Some(a) = self.atlas.as_ref() else { return };
        self.list.text(a, slot(f), &bytes, rect, format, colour, None, outline::NONE);
    }

    fn gui_rect(&mut self, rect: UiRect, colour: u32) {
        self.list.rect([rect.left, rect.top, rect.right, rect.bottom].map(|v| v as f32), colour);
    }

    fn text_width_wide(&self, f: UiFont, text: &[u16]) -> i32 {
        self.atlas.as_ref().map_or(0, |a| a.width(slot(f), &narrow(text)))
    }

    fn to_wide(&self, text: &[u8]) -> Vec<u16> {
        text.iter().map(|&b| u16::from(b)).collect()
    }

    fn bars_begin(&mut self) {}

    fn bars_group(&mut self, quads: &[ColourQuad; 3]) {
        for q in quads {
            self.list.rect([q.x0, q.y0, q.x1, q.y1], q.colour);
        }
    }

    fn bars_end(&mut self) {}

    fn project(&self, p: [f32; 3]) -> [f32; 3] {
        match self.projector.as_ref().and_then(|f| f(p)) {
            Some((x, y)) => [x, y, 0.5],
            None => [0.0, 0.0, 2.0],
        }
    }

    fn aux_fonts(&mut self, _create: bool) -> bool {
        self.atlas.as_ref().is_some_and(|a| a.has(slot(UiFont::Aux20)) && a.has(slot(UiFont::Aux22)))
    }

    fn chat_blit(&mut self) -> bool {
        false
    }

    fn textdraw_units(&self) -> (f32, f32) {
        (1.0 / 640.0, 1.0 / 448.0)
    }

    fn textdraw_text(&mut self, f: &TextdrawFont, text: &[u8], _has_keys: bool) {
        self.list.cmds.push(Cmd::GameFont { font: *f, text: text.to_vec() });
    }

    /// The texture is the canvas's, known at the flush: a textdraw is drawn as a sprite until the canvas reported
    /// that it has no such texture.
    fn textdraw_sprite(&mut self, _id: u16, style: u8, text: &[u8], _serial: u64, rect: [f32; 4], colour: u32) -> bool {
        if style != samp_ui::textdraw::FONT_SPRITE || self.sprites.get(text) == Some(&false) {
            return false;
        }
        self.list.cmds.push(Cmd::Sprite { text: text.to_vec(), rect, colour });
        true
    }

    fn textdraw_sync(&mut self, live: &[(u16, &[u8])]) {
        self.sprites.retain(|t, _| live.iter().any(|(_, l)| *l == t.as_slice()));
    }

    fn set_input_state(&mut self, keyboard: bool, mouse: bool, cursor: bool) {
        self.input = (keyboard, mouse, cursor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A canvas that records what it was asked.
    #[derive(Default)]
    struct Recorder {
        uploads: Vec<(u32, u32)>,
        released: Vec<u32>,
        calls: Vec<(Option<u32>, Vec<Quad>)>,
    }

    impl Canvas for Recorder {
        fn size(&self) -> (i32, i32) {
            (800, 600)
        }
        fn upload(&mut self, w: u32, h: u32, _smooth: bool, rgba: &[u8]) -> Option<u32> {
            assert_eq!(rgba.len(), (w * h * 4) as usize);
            self.uploads.push((w, h));
            Some(self.uploads.len() as u32)
        }
        fn release(&mut self, id: u32) {
            self.released.push(id);
        }
        fn quads(&mut self, image: Option<u32>, quads: &[Quad]) {
            self.calls.push((image, quads.to_vec()));
        }
    }

    fn backend() -> QuadUi {
        let mut o = QuadUi::new((800, 600));
        assert!(o.fonts(b"Arial", 700, 0, false));
        assert!(!o.fonts(b"Arial", 700, 0, false));
        o
    }

    #[test]
    fn metrics_follow_the_atlas() {
        let m = backend().metrics();
        assert_eq!((m.main_h, m.small_h, m.gui0_h, m.gui1_h), (14, 12, 20, 18));
        assert_eq!((m.icon_w, m.icon_h), (31, 24));
        assert!(m.timestamp_w > 50 && m.nick_w > 150);
    }

    #[test]
    fn dxut_calls_become_rects_and_centred_glyphs() {
        let mut o = backend();
        o.dialog_begin(UiRect::new(245, 510, 555, 550), 0x960A_0A0A);
        o.gui_sprite(UiRect::new(0, 0, 136, 54), UiRect::new(255, 515, 345, 545), 0xC8FF_FFFF);
        o.gui_sprite(UiRect::new(0, 0, 136, 54), UiRect::new(255, 515, 345, 545), 0x00FF_FFFF);
        o.gui_text(UiFont::Gui0, b"<<", UiRect::new(255, 515, 345, 545), 0x5, 0xFFC8_C8C8);
        o.dialog_end();
        let w = o.atlas.as_ref().unwrap().width(2, b"<<");
        let cmds = o.list().cmds.clone();
        assert_eq!(cmds[0], Cmd::Rect { rect: [245.0, 510.0, 555.0, 550.0], colour: 0x960A_0A0A });
        assert!(matches!(cmds[1], Cmd::Skin { dst: [255.0, 515.0, 345.0, 545.0], colour: 0xC8FF_FFFF, .. }));
        assert_eq!(cmds.len(), 3);
        let Cmd::Text { x, y, slot, .. } = &cmds[2] else { panic!() };
        assert_eq!((*x, *y, *slot), ((255 + (90 - w) / 2) as f32, 520.0, 2));
        // without a skin the control is a flat rectangle; the label two glyph quads of the atlas image
        let mut c = Recorder::default();
        o.flush(&mut c, false);
        assert_eq!(c.uploads.len(), 1);
        assert_eq!(c.calls.len(), 2);
        assert_eq!((c.calls[0].0, c.calls[0].1.len()), (None, 2));
        assert_eq!((c.calls[1].0, c.calls[1].1.len()), (Some(1), 2));
        assert!(o.list().cmds.is_empty());
        // the atlas is uploaded once; a font size change replaces it
        o.gui_rect(UiRect::new(0, 0, 1, 1), 0xFF00_0000);
        o.flush(&mut c, false);
        assert_eq!(c.uploads.len(), 1);
        assert!(o.fonts(b"Arial", 700, 2, false));
        o.flush(&mut c, false);
        assert_eq!(c.released, [1]);
        assert_eq!(c.uploads.len(), 2);
    }

    #[test]
    fn outlined_text_is_one_run_of_pre_outlined_glyphs() {
        let mut o = backend();
        let glyphs = [0xFFFF_FFFF; 5];
        o.draw_text_outlined(
            UiSprite::Chat,
            UiFont::Main,
            UiFont::MainOutline,
            b"hello",
            UiRect::new(45, 10, 550, 110),
            dt::SINGLELINE,
            0xFFFF_FFFF,
            &glyphs,
            &samp_ui::fonts::OUTLINE_4,
        );
        assert_eq!(o.list().cmds.len(), 1);
        assert!(matches!(&o.list().cmds[0], Cmd::Text { x: 45.0, y: 10.0, outline: 1, .. }));
        assert_eq!(narrow(&[0x25CF, u16::from(b'a'), 0x4E2D]), vec![MASK_BYTE, b'a', b'?']);
    }

    #[test]
    fn textdraws_scale_align_and_break_lines() {
        let mut o = backend();
        let f = TextdrawFont {
            scale: (0.5, 1.0),
            colour: 0xFF00_00FF,
            orientation: 0,
            centre_size: 100.0,
            background: true,
            background_colour: 0x8000_0000,
            x: 400.0,
            y: 50.0,
            ..TextdrawFont::default()
        };
        o.textdraw_text(&f, b"AB~n~C~r~D", false);
        let mut c = Recorder::default();
        o.flush(&mut c, false);
        // the box spans the centre size over two lines of 18 px
        assert_eq!(c.calls[0].0, None);
        assert_eq!(c.calls[0].1[0].rect, [350.0, 50.0, 100.0, 36.0]);
        // A, B on the first line, C, D on the second; red (CRGBA 0xFF0000FF in memory order R, G, B, A)
        let glyphs = &c.calls[1].1;
        assert_eq!(glyphs.len(), 4);
        assert!(glyphs.iter().all(|q| q.colour == 0xFFFF_0000));
        assert!(glyphs[2].rect[1] > glyphs[0].rect[1] + 10.0);
    }
}
