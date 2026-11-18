//! samp-rs's quad UI ([`samp_overlay::QuadUi`]) on the mod's overlay: its images are overlay images, its quads
//! overlay quads.

use std::collections::BTreeMap;

use sa_mod::{FontText, GameTexture, Image, Overlay, Quad, Rgba};
use sa_samp_api::ui::TextdrawFont;

/// The UI's images, by the ids the UI knows them by, and the game textures of sprite textdraws, by their text
/// (`None`: the game has no such texture).
#[derive(Default)]
pub struct Images {
    next: u32,
    images: BTreeMap<u32, Image>,
    textures: BTreeMap<Vec<u8>, Option<GameTexture>>,
}

/// The game texture of a sprite textdraw's `txd:name`, found (or its dictionary loaded) as samp does: `Err(())`
/// while the game has not looked yet, `Ok(None)` when there is none.
fn find_texture(text: &[u8]) -> Result<Option<GameTexture>, ()> {
    let Some(src) = samp_ui::textdraw::sprite_source(text) else { return Ok(None) };
    let s = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    let file = src.path().map(|p| s(&p));
    match GameTexture::find(&s(&src.slot), file.as_deref(), &s(&src.texture)) {
        Ok(t) => Ok(Some(t)),
        Err(sa_mod::Error::Again) => Err(()),
        Err(_) => Ok(None),
    }
}

/// The overlay of the frame being drawn, with the UI's images.
pub struct ModCanvas<'a> {
    pub overlay: &'a mut Overlay,
    pub images: &'a mut Images,
}

/// `0xAARRGGBB` as the overlay's `0xRRGGBBAA`.
fn rgba(argb: u32) -> Rgba {
    Rgba(argb.rotate_left(8))
}

impl samp_overlay::Canvas for ModCanvas<'_> {
    fn size(&self) -> (i32, i32) {
        let (w, h) = self.overlay.size();
        (w as i32, h as i32)
    }

    fn upload(&mut self, width: u32, height: u32, smooth: bool, rgba: &[u8]) -> Option<u32> {
        match Image::new(width, height, smooth, rgba) {
            Ok(img) => {
                self.images.next += 1;
                self.images.images.insert(self.images.next, img);
                Some(self.images.next)
            }
            Err(e) => {
                sa_mod::log(
                    sa_mod::level::WARN,
                    &format!("samp: UI image {width}x{height}: {e:?}"),
                );
                None
            }
        }
    }

    fn release(&mut self, id: u32) {
        self.images.images.remove(&id);
    }

    fn quads(&mut self, image: Option<u32>, quads: &[samp_overlay::Quad]) {
        let image = match image {
            Some(id) => match self.images.images.get(&id) {
                Some(i) => Some(i),
                None => return,
            },
            None => None,
        };
        let quads: Vec<Quad> = quads
            .iter()
            .map(|q| Quad {
                rect: q.rect,
                uv: q.uv,
                colour: rgba(q.colour),
            })
            .collect();
        let _ = self.overlay.quads(image, &quads);
    }

    fn game_text(&mut self, f: &TextdrawFont, text: &[u8]) -> bool {
        // the textdraw's colours are CRGBA words (R, G, B, A in memory order)
        let rgba = |c: u32| c.swap_bytes();
        let t = FontText {
            x: f.x,
            y: f.y,
            scale: f.scale,
            colour: rgba(f.colour),
            orientation: f.orientation,
            wrap_x: f.wrap_x,
            centre_size: f.centre_size,
            background: f.background,
            background_colour: rgba(f.background_colour),
            proportional: f.proportional,
            drop_colour: rgba(f.drop_colour),
            edge: f.outline,
            shadow: f.shadow,
            style: f.font,
        };
        self.overlay.game_text(&t, text).is_ok()
    }

    fn game_sprite(&mut self, text: &[u8], rect: [f32; 4], colour: u32) -> bool {
        let tex = match self.images.textures.get(text) {
            Some(t) => *t,
            None => match find_texture(text) {
                Ok(t) => *self.images.textures.entry(text.to_vec()).or_insert(t),
                // the game looks it up in its next frame
                Err(()) => return true,
            },
        };
        let Some(tex) = tex else { return false };
        let [l, t, r, b] = rect;
        // the tint is a CRGBA word (R, G, B, A in memory order)
        self.overlay.game_sprite(tex, [l, t, r - l, b - t], Rgba(colour.swap_bytes())).is_ok()
    }
}
