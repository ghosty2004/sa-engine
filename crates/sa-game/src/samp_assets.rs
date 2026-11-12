//! SA:MP's own UI files, read at run time from the user's install (never shipped): the GUI skin
//! `sampgui.png` and the cursor `mouse.png` (both in the GTA folder of a SA:MP install), and the UI font
//! "Arial" Bold from the user's system. The engine-drawn UI (`sa-samp-ui`) falls back to flat controls and
//! the committed Liberation Sans Bold without them.
//!
//! Search order: the game folder, then `SA_SAMP_DIR` (a folder holding the SA:MP client files). The font:
//! `SA_SAMP_UI_FONT` (a TTF path), then the usual places of Microsoft's Arial Bold (`arialbd.ttf`) on Linux,
//! in the game's Wine prefix and on Windows.

use std::path::{Path, PathBuf};

use sa_samp_ui::skin::{TEXTURE_H, TEXTURE_W};

/// Decodes a PNG into RGBA8 (`None` for anything else than 8-bit RGB / RGBA / grey).
fn decode_png(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let f = std::fs::File::open(path).ok()?;
    let mut dec = png::Decoder::new(std::io::BufReader::new(f));
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
        png::ColorType::Rgb => px.as_chunks::<3>().0.iter().flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => px.as_chunks::<2>().0.iter().flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some((info.width, info.height, rgba))
}

/// The folders searched for SA:MP's files.
fn samp_dirs(game: &Path) -> Vec<PathBuf> {
    let mut v = vec![game.to_path_buf()];
    if let Some(d) = std::env::var_os("SA_SAMP_DIR") {
        v.push(PathBuf::from(d));
    }
    v
}

fn find(game: &Path, name: &str) -> Option<PathBuf> {
    samp_dirs(game).into_iter().map(|d| d.join(name)).find(|p| p.is_file())
}

/// The combined skin texture ([`sa_samp_ui::skin`]): `sampgui.png` at the top, `mouse.png` at (0, 512).
/// `None` when `sampgui.png` is missing or not 512 × 512; a missing `mouse.png` leaves its area empty.
pub fn skin(game: &Path) -> Option<Vec<u8>> {
    let (w, h, gui) = decode_png(&find(game, "sampgui.png")?)?;
    if (w, h) != (512, 512) {
        return None;
    }
    let mut img = vec![0u8; (TEXTURE_W * TEXTURE_H * 4) as usize];
    img[..gui.len()].copy_from_slice(&gui);
    if let Some((mw, mh, m)) = find(game, "mouse.png").and_then(|p| decode_png(&p)) {
        for y in 0..mh.min(32) {
            for x in 0..mw.min(32) {
                let s = ((y * mw + x) * 4) as usize;
                let d = (((512 + y) * TEXTURE_W + x) * 4) as usize;
                img[d..d + 4].copy_from_slice(&m[s..s + 4]);
            }
        }
    }
    Some(img)
}

/// The user's Arial Bold, if there is one.
pub fn arial_bold(game: &Path) -> Option<(PathBuf, Vec<u8>)> {
    let mut c: Vec<PathBuf> = Vec::new();
    if let Some(p) = std::env::var_os("SA_SAMP_UI_FONT") {
        c.push(PathBuf::from(p));
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        c.push(home.join(".fonts/arialbd.ttf"));
        c.push(home.join(".local/share/fonts/arialbd.ttf"));
    }
    for d in [
        "/usr/share/fonts/TTF/arialbd.ttf",
        "/usr/share/fonts/truetype/msttcorefonts/arialbd.ttf",
        "/usr/share/fonts/truetype/msttcorefonts/Arial_Bold.ttf",
        "/usr/share/fonts/msttcore/arialbd.ttf",
        "/usr/share/fonts/corefonts/arialbd.ttf",
        "C:\\Windows\\Fonts\\arialbd.ttf",
    ] {
        c.push(PathBuf::from(d));
    }
    if let Some(p) = std::env::var_os("WINEPREFIX") {
        c.push(PathBuf::from(p).join("drive_c/windows/Fonts/arialbd.ttf"));
    }
    // A game folder inside a Wine prefix (…/drive_c/…): that prefix's fonts.
    if let Some(drive_c) = game.ancestors().find(|a| a.file_name().is_some_and(|n| n == "drive_c")) {
        c.push(drive_c.join("windows/Fonts/arialbd.ttf"));
    }
    c.into_iter()
        .filter(|p| p.is_file())
        .find_map(|p| std::fs::read(&p).ok().filter(|b| sa_samp_ui::font::parses(b)).map(|b| (p, b)))
}

/// Hands the files to the overlay and reports what was found.
pub fn install(game: &Path, overlay: &mut sa_samp::overlay::Overlay) {
    match skin(game) {
        Some(img) => {
            overlay.set_skin(img);
            println!("sa-game: SA:MP UI skin: sampgui.png + mouse.png from the SA:MP install");
        }
        None => println!(
            "sa-game: SA:MP UI skin: no sampgui.png in the game folder or SA_SAMP_DIR; flat controls"
        ),
    }
    match arial_bold(game) {
        Some((p, ttf)) => {
            println!("sa-game: SA:MP UI font: {}", p.display());
            overlay.set_font(ttf);
        }
        None => println!("sa-game: SA:MP UI font: no Arial Bold found; Liberation Sans Bold"),
    }
}
