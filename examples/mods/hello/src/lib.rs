//! The hello mod (MOD-2 M0): a text line on a dark box in the bottom right corner of every frame.
//! Build: `examples/mods/build.sh` (docs/mods/getting-started.md).

use sa_mod::{Mod, Overlay, Rgba};

/// Glyph cell size in pixels (the host font is 8x8: 2 screen pixels per font pixel).
const SIZE: f32 = 16.0;
/// Distance from the screen edges.
const MARGIN: f32 = 8.0;
/// Padding inside the box.
const PAD: f32 = 6.0;

#[derive(Default)]
struct Hello;

impl Mod for Hello {
    fn init(&mut self) -> Result<(), String> {
        sa_mod::info("hello: loaded");
        Ok(())
    }

    fn overlay(&mut self, frame: u32, o: &mut Overlay) {
        let text = format!("Hello from a wasm mod! frame {frame}");
        let (w, h) = o.size();
        // the font has fixed cells: the width is known before drawing
        let tw = text.chars().count() as f32 * SIZE;
        let (x, y) = (w as f32 - MARGIN - PAD - tw, h as f32 - MARGIN - PAD - SIZE);
        let _ = o.rect(x - PAD, y - PAD, tw + 2.0 * PAD, SIZE + 2.0 * PAD, Rgba::new(0, 0, 0, 0xA0));
        let _ = o.text(x, y, SIZE, Rgba::new(0xFF, 0xE0, 0x40, 0xFF), &text);
    }
}

sa_mod::entry!(Hello);
