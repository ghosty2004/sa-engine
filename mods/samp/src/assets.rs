//! SA:MP's own UI files, which the user copies from their SA:MP install into the mod's folder (never shipped): the
//! GUI skin `sampgui.png`, the cursor `mouse.png`, the symbol fonts `gtaweap3.ttf` (GTA's) and `sampaux3.ttf`, and
//! optionally `arialbd.ttf` (samp's UI font; without it the committed Liberation Sans Bold). Without the skin the
//! controls are flat rectangles and the cursor the platform's.

use sa_mod::files;
use samp_overlay::QuadUi;

/// Decodes a PNG into RGBA8 (`None` for anything but 8-bit RGB / RGBA / grey).
fn decode_png(data: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(data));
    dec.set_transformations(png::Transformations::EXPAND);
    let mut r = dec.read_info().ok()?;
    let mut buf = vec![0u8; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    if info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let px = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px.chunks_exact(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => px
            .chunks_exact(2)
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some((info.width, info.height, rgba))
}

fn file(name: &str) -> Option<Vec<u8>> {
    files::read(name).ok().flatten()
}

/// Hands the files the mod's folder holds to the UI and logs what it found.
pub fn install(ui: &mut QuadUi) {
    let gui = file("sampgui.png").and_then(|d| decode_png(&d));
    let mouse = file("mouse.png").and_then(|d| decode_png(&d));
    let skin = gui.as_ref().and_then(|(w, h, px)| {
        samp_overlay::skin_image(
            (*w, *h, px),
            mouse.as_ref().map(|(w, h, px)| (*w, *h, px.as_slice())),
        )
    });
    let found = |b: bool| if b { "found" } else { "missing" };
    sa_mod::info(&format!(
        "samp: UI files in the mod's folder: sampgui.png {}, mouse.png {}",
        found(skin.is_some()),
        found(mouse.is_some())
    ));
    if let Some(s) = skin {
        ui.set_skin(s);
    }
    let (weapon, aux) = (file("gtaweap3.ttf"), file("sampaux3.ttf"));
    sa_mod::info(&format!(
        "samp: gtaweap3.ttf {}, sampaux3.ttf {}",
        found(weapon.is_some()),
        found(aux.is_some())
    ));
    ui.set_symbol_fonts(weapon, aux);
    if let Some(f) = file("arialbd.ttf") {
        ui.set_font(f);
    }
}
