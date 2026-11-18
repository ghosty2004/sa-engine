//! The libpng 1.0.12 entry points gta_sa.exe links statically (with zlib 1.1.3), as far as RW's `RtPNGImageWrite`
//! (0x7CF600) calls them, served by the pure-Rust `png` encoder (MIT OR Apache-2.0).
//!
//! `png_create_write_struct_2` / `png_create_info_struct` hand out zeroed blocks from the C heap as the two structs;
//! the settings are kept here per png struct. Of the png struct's fields only `io_ptr` (+0x54) is written, because
//! the game's write callback (`_rtPNGWriteData` 0x7CF980) reads it. `png_write_end` encodes the image and passes the
//! whole file to that callback in one call (libpng writes it chunk by chunk; the stream receives the same sequence of
//! bytes, except that the compressed data is the `png` crate's and not zlib 1.1.3's).
//!
//! libpng reports an error by `longjmp` to the `setjmp` the caller armed. Here an error (an invalid header, a
//! palette for a non-palette image, `png_error` from the callback) is logged and makes every later call on that
//! struct do nothing, so nothing more is written; the caller goes on to free the structs and close its stream.

use std::collections::HashMap;
use std::sync::Mutex;

use sa_core::mem::GPtr;

sa_core::orig! {
    /// The `png_rw_ptr` write callback `(png_ptr, data, length)` (cdecl), by address.
    fn write_callback(f: u32, png: u32, data: u32, len: u32) = cdecl dyn => unported;
}

/// `png_struct.io_ptr` (libpng 1.0.12, 32-bit).
const IO_PTR: u32 = 0x54;
/// Bytes handed out for each struct: past every field the game's callbacks read.
const STRUCT_BLOCK: u32 = 0x100;

const COLOR_TYPE_PALETTE: u32 = 3;
const COLOR_TYPE_RGB_ALPHA: u32 = 6;

#[derive(Default)]
struct Writer {
    write_fn: u32,
    width: u32,
    height: u32,
    depth: u32,
    color: u32,
    palette: Vec<u8>,
    trns: Vec<u8>,
    rows: Vec<u8>,
    failed: bool,
}

static WRITERS: Mutex<Option<HashMap<u32, Writer>>> = Mutex::new(None);

fn with_writer<R: Default>(png: u32, f: impl FnOnce(&mut Writer) -> R) -> R {
    let mut s = WRITERS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    match s.get_or_insert_with(HashMap::new).get_mut(&png) {
        Some(w) if !w.failed => f(w),
        _ => R::default(),
    }
}

fn fail(png: u32, msg: &str) {
    eprintln!("libpng: error: {msg}");
    let mut s = WRITERS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(w) = s.get_or_insert_with(HashMap::new).get_mut(&png) {
        w.failed = true;
    }
}

fn alloc() -> u32 {
    crate::crt::calloc(1, STRUCT_BLOCK).addr() as u32
}

/// `png_create_write_struct_2(version, error_ptr, error_fn, warn_fn, mem_ptr, malloc_fn, free_fn)` (0x7D03B0).
fn png_create_write_struct_2(_ver: u32, _err: u32, _err_fn: u32, _warn_fn: u32, _mem: u32, _malloc: u32, _free: u32) -> u32 {
    let png = alloc();
    if png != 0 {
        let mut s = WRITERS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        s.get_or_insert_with(HashMap::new).insert(png, Writer::default());
    }
    png
}

/// `png_create_info_struct(png_ptr)` (0x7D2E80).
fn png_create_info_struct(png: u32) -> u32 {
    if png == 0 { 0 } else { alloc() }
}

/// `png_destroy_write_struct(&png_ptr, &info_ptr)` (0x7D0A40): both freed (either pointer may be NULL), the
/// variables cleared.
fn png_destroy_write_struct(png_pp: u32, info_pp: u32) {
    if png_pp != 0 {
        let png = GPtr::<u32>::from_addr(png_pp as usize).read();
        let mut s = WRITERS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        s.get_or_insert_with(HashMap::new).remove(&png);
    }
    for pp in [info_pp, png_pp] {
        if pp == 0 {
            continue;
        }
        let cell = GPtr::<u32>::from_addr(pp as usize);
        let p = cell.read();
        if p != 0 {
            crate::crt::free(GPtr::from_addr(p as usize));
            cell.write(0);
        }
    }
}

/// `png_set_write_fn(png_ptr, io_ptr, write_fn, flush_fn)` (0x7D2C20).
fn png_set_write_fn(png: u32, io: u32, write_fn: u32, _flush: u32) {
    with_writer(png, |w| {
        GPtr::<u32>::from_addr((png + IO_PTR) as usize).write(io);
        w.write_fn = write_fn;
    });
}

/// `png_set_IHDR(png_ptr, info_ptr, width, height, bit_depth, color_type, interlace, compression, filter)`
/// (0x7D1D60): what the encoder supports of libpng's checks (non-zero size, a palette or RGBA image, a bit depth
/// valid for it, no interlacing, the standard compression and filter methods).
#[allow(clippy::too_many_arguments)]
fn png_set_ihdr(png: u32, _info: u32, width: u32, height: u32, depth: u32, color: u32, interlace: u32, compression: u32, filter: u32) {
    let valid = width != 0
        && height != 0
        && match color {
            COLOR_TYPE_PALETTE => matches!(depth, 1 | 2 | 4 | 8),
            COLOR_TYPE_RGB_ALPHA => matches!(depth, 8 | 16),
            _ => false,
        }
        && interlace == 0
        && compression == 0
        && filter == 0;
    if !valid {
        return fail(png, &format!("unsupported IHDR {width}x{height}, depth {depth}, colour type {color}"));
    }
    with_writer(png, |w| {
        (w.width, w.height, w.depth, w.color) = (width, height, depth, color);
    });
}

/// `png_set_PLTE(png_ptr, info_ptr, palette, num_palette)` (0x7D2130): `num_palette` RGB triplets.
fn png_set_plte(png: u32, _info: u32, palette: u32, n: u32) {
    if n == 0 || n > 256 {
        return fail(png, &format!("invalid palette length {n}"));
    }
    with_writer(png, |w| GPtr::<u8>::from_addr(palette as usize).with_bytes(3 * n as usize, |b| w.palette = b.to_vec()));
}

/// `png_set_tRNS(png_ptr, info_ptr, trans, num_trans, trans_values)` (0x7D2630): the palette entries' alpha.
fn png_set_trns(png: u32, _info: u32, trans: u32, n: u32, _values: u32) {
    with_writer(png, |w| GPtr::<u8>::from_addr(trans as usize).with_bytes(n.min(256) as usize, |b| w.trns = b.to_vec()));
}

/// `png_write_info(png_ptr, info_ptr)` (0x7CFFA0): the header is written with the image in `png_write_end`.
fn png_write_info(_png: u32, _info: u32) {}

/// `png_set_packing(png_ptr)` (0x7D1340): rows of a depth below 8 come one pixel per byte, which
/// [`png_write_image`] expects anyway.
fn png_set_packing(_png: u32) {}

/// `png_write_image(png_ptr, row_pointers)` (0x7D06C0): `height` rows, one pixel per byte (palette) or 4 / 8 bytes
/// per pixel (RGBA 8 / 16).
fn png_write_image(png: u32, rows: u32) {
    with_writer(png, |w| {
        let len = row_bytes(w.color, w.depth, w.width);
        for i in 0..w.height {
            let row = GPtr::<u32>::from_addr((rows + 4 * i) as usize).read();
            GPtr::<u8>::from_addr(row as usize).with_bytes(len, |b| w.rows.extend_from_slice(b));
        }
    });
}

/// `png_write_end(png_ptr, info_ptr)` (0x7D0250): the encoded file through the write callback.
fn png_write_end(png: u32, _info: u32) {
    let file = with_writer(png, |w| {
        Some((w.write_fn, encode(w.width, w.height, w.depth, w.color, &w.rows, &w.palette, &w.trns)))
    });
    match file {
        Some((write_fn, Ok(data))) => write_callback(write_fn, png, data.as_ptr() as usize as u32, data.len() as u32),
        Some((_, Err(e))) => fail(png, &e),
        None => {}
    }
}

/// `png_error(png_ptr, message)` (0x7D34F0).
fn png_error(png: u32, msg: u32) {
    let text = if msg == 0 { "(no message)".to_owned() } else { crate::bind::cstr(GPtr::from_addr(msg as usize)) };
    fail(png, &text);
}

fn row_bytes(color: u32, depth: u32, width: u32) -> usize {
    let per_pixel = if color == COLOR_TYPE_PALETTE { 1 } else { 4 * depth as usize / 8 };
    width as usize * per_pixel
}

/// A PNG of `rows` (one byte per pixel for a palette image, packed here to `depth` bits; RGBA samples otherwise,
/// 16-bit ones big-endian as libpng takes them).
pub fn encode(width: u32, height: u32, depth: u32, color: u32, rows: &[u8], palette: &[u8], trns: &[u8]) -> Result<Vec<u8>, String> {
    let (color_type, bit_depth) = match (color, depth) {
        (COLOR_TYPE_PALETTE, 1) => (png::ColorType::Indexed, png::BitDepth::One),
        (COLOR_TYPE_PALETTE, 2) => (png::ColorType::Indexed, png::BitDepth::Two),
        (COLOR_TYPE_PALETTE, 4) => (png::ColorType::Indexed, png::BitDepth::Four),
        (COLOR_TYPE_PALETTE, 8) => (png::ColorType::Indexed, png::BitDepth::Eight),
        (COLOR_TYPE_RGB_ALPHA, 8) => (png::ColorType::Rgba, png::BitDepth::Eight),
        (COLOR_TYPE_RGB_ALPHA, 16) => (png::ColorType::Rgba, png::BitDepth::Sixteen),
        _ => return Err(format!("no image set up (colour type {color}, depth {depth})")),
    };
    let data: Vec<u8> = if color == COLOR_TYPE_PALETTE && depth < 8 {
        let per_byte = 8 / depth as usize;
        rows.chunks(width as usize)
            .flat_map(|row| {
                row.chunks(per_byte).map(move |px| {
                    px.iter().enumerate().fold(0u8, |acc, (i, &v)| {
                        acc | ((v & ((1 << depth) - 1) as u8) << (8 - depth as usize * (i + 1)))
                    })
                })
            })
            .collect()
    } else {
        rows.to_vec()
    };
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, width, height);
    enc.set_color(color_type);
    enc.set_depth(bit_depth);
    if color == COLOR_TYPE_PALETTE {
        enc.set_palette(palette);
        if !trns.is_empty() {
            enc.set_trns(trns);
        }
    }
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(&data).map_err(|e| e.to_string())?;
    w.finish().map_err(|e| e.to_string())?;
    Ok(out)
}

/// `(address, name, native entry)` of the libpng functions.
pub fn entries() -> Vec<(u32, &'static str, usize)> {
    use crate::platform::registry_entry::cdecl_entry as entry;
    vec![
        entry!(0x007D_03B0, "libpng png_create_write_struct_2", png_create_write_struct_2; fn(ver: u32, err: u32, err_fn: u32, warn_fn: u32, mem: u32, malloc: u32, free: u32) -> u32),
        entry!(0x007D_2E80, "libpng png_create_info_struct", png_create_info_struct; fn(png: u32) -> u32),
        entry!(0x007D_0A40, "libpng png_destroy_write_struct", png_destroy_write_struct; fn(png_pp: u32, info_pp: u32) -> ()),
        entry!(0x007D_2C20, "libpng png_set_write_fn", png_set_write_fn; fn(png: u32, io: u32, write_fn: u32, flush: u32) -> ()),
        entry!(0x007D_1D60, "libpng png_set_IHDR", png_set_ihdr; fn(png: u32, info: u32, width: u32, height: u32, depth: u32, color: u32, interlace: u32, compression: u32, filter: u32) -> ()),
        entry!(0x007D_2130, "libpng png_set_PLTE", png_set_plte; fn(png: u32, info: u32, palette: u32, n: u32) -> ()),
        entry!(0x007D_2630, "libpng png_set_tRNS", png_set_trns; fn(png: u32, info: u32, trans: u32, n: u32, values: u32) -> ()),
        entry!(0x007C_FFA0, "libpng png_write_info", png_write_info; fn(png: u32, info: u32) -> ()),
        entry!(0x007D_1340, "libpng png_set_packing", png_set_packing; fn(png: u32) -> ()),
        entry!(0x007D_06C0, "libpng png_write_image", png_write_image; fn(png: u32, rows: u32) -> ()),
        entry!(0x007D_0250, "libpng png_write_end", png_write_end; fn(png: u32, info: u32) -> ()),
        entry!(0x007D_34F0, "libpng png_error", png_error; fn(png: u32, msg: u32) -> ()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The frame info, the pixels, the palette and the tRNS chunk.
    type Decoded = (png::OutputInfo, Vec<u8>, Option<Vec<u8>>, Option<Vec<u8>>);

    fn decode(file: &[u8]) -> Decoded {
        let mut dec = png::Decoder::new(std::io::Cursor::new(file));
        dec.set_transformations(png::Transformations::IDENTITY);
        let mut dec = dec.read_info().expect("header");
        let (plte, trns) = {
            let info = dec.info();
            (info.palette.as_ref().map(|p| p.to_vec()), info.trns.as_ref().map(|t| t.to_vec()))
        };
        let mut buf = vec![0; dec.output_buffer_size().expect("size")];
        let out = dec.next_frame(&mut buf).expect("frame");
        buf.truncate(out.buffer_size());
        (out, buf, plte, trns)
    }

    #[test]
    fn rgba_image_round_trips() {
        let (w, h) = (5u32, 3u32);
        let rows: Vec<u8> = (0..w * h * 4).map(|i| (i * 7) as u8).collect();
        let file = encode(w, h, 8, COLOR_TYPE_RGB_ALPHA, &rows, &[], &[]).expect("encode");
        let (info, px, _, _) = decode(&file);
        assert_eq!((info.width, info.height, info.color_type), (w, h, png::ColorType::Rgba));
        assert_eq!(px, rows);
    }

    #[test]
    fn four_bit_palette_image_is_packed_with_its_alpha() {
        let (w, h) = (3u32, 2u32);
        let rows = [1u8, 15, 0, 7, 8, 2];
        let palette: Vec<u8> = (0..16 * 3).map(|i| i as u8).collect();
        let trns: Vec<u8> = (0..16).map(|i| 255 - i as u8).collect();
        let file = encode(w, h, 4, COLOR_TYPE_PALETTE, &rows, &palette, &trns).expect("encode");
        let (info, px, plte, alpha) = decode(&file);
        assert_eq!((info.bit_depth, info.color_type), (png::BitDepth::Four, png::ColorType::Indexed));
        assert_eq!(px, [0x1F, 0x00, 0x78, 0x20]);
        assert_eq!(plte.as_deref(), Some(palette.as_slice()));
        assert_eq!(alpha.as_deref(), Some(trns.as_slice()));
    }
}
