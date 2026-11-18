//! samp-rs's quad UI ([`samp_overlay::QuadUi`]) on the mod's overlay: its images are overlay images, its quads
//! overlay quads.

use std::collections::BTreeMap;

use sa_mod::{FontText, Image, Overlay, Quad, Rgba};
use sa_samp_api::ui::TextdrawFont;

/// The UI's images, by the ids the UI knows them by.
#[derive(Default)]
pub struct Images {
    next: u32,
    images: BTreeMap<u32, Image>,
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
}
