//! The IJG libjpeg 6b entry points gta_sa.exe links statically (0x5C26B0-0x5D0260), as far as the ported
//! `JPegCompressScreen` (sa-render `jpeg_plugin.rs`, the gallery photo) calls them, served by the pure-Rust
//! `jpeg-encoder` (MIT OR Apache-2.0 AND IJG).
//!
//! The game fills a `jpeg_compress_struct` (0x168 bytes, libjpeg 6b layout) and hands every scanline to
//! `jpeg_write_scanlines`; the rows are collected here (per struct address) and encoded in
//! `jpeg_finish_compress`, which writes the file through the game's `jpeg_destination_mgr` callbacks as libjpeg
//! does: `init_destination` from `jpeg_start_compress`, `empty_output_buffer` whenever the buffer is full,
//! `term_destination` at the end. The struct fields the game reads back (`next_scanline`, `global_state`) are kept
//! as libjpeg keeps them.
//!
//! The settings are libjpeg's defaults (`jpeg_set_defaults`: quality 75, 2x2 chroma subsampling) and the game's
//! `optimize_coding`. The rows are 4 bytes per pixel (gta_sa.exe's libjpeg is built with `RGB_PIXELSIZE` 4, so
//! `input_components` 4 is valid for `JCS_RGB`; the fourth byte is ignored). The encoded bytes differ from libjpeg's
//! (another DCT and entropy coder): the file is a valid JPEG of the same picture.

use std::collections::HashMap;
use std::sync::Mutex;

use jpeg_encoder::{ColorType, Encoder, SamplingFactor};
use sa_core::mem::GPtr;

sa_core::orig! {
    /// A `jpeg_destination_mgr` callback `(j_compress_ptr)` (cdecl), by address: `init_destination`,
    /// `empty_output_buffer` (returns `boolean` in AL) or `term_destination`.
    fn dest_callback(f: u32, cinfo: u32) -> u32 = cdecl dyn => unported;
}

/// `jpeg_compress_struct` offsets (libjpeg 6b, 32-bit).
mod cinfo {
    pub const ERR: u32 = 0x00;
    pub const CLIENT_DATA: u32 = 0x0C;
    pub const GLOBAL_STATE: u32 = 0x14;
    pub const DEST: u32 = 0x18;
    pub const IMAGE_WIDTH: u32 = 0x1C;
    pub const IMAGE_HEIGHT: u32 = 0x20;
    pub const INPUT_COMPONENTS: u32 = 0x24;
    pub const INPUT_GAMMA: u32 = 0x30;
    pub const DATA_PRECISION: u32 = 0x38;
    pub const OPTIMIZE_CODING: u32 = 0xB8;
    pub const NEXT_SCANLINE: u32 = 0xD0;
    pub const SIZE: u32 = 0x168;
}

/// `jpeg_destination_mgr` offsets.
mod dest {
    pub const NEXT_OUTPUT_BYTE: u32 = 0x00;
    pub const FREE_IN_BUFFER: u32 = 0x04;
    pub const INIT_DESTINATION: u32 = 0x08;
    pub const EMPTY_OUTPUT_BUFFER: u32 = 0x0C;
    pub const TERM_DESTINATION: u32 = 0x10;
}

const JPEG_LIB_VERSION: u32 = 62;
const CSTATE_START: u32 = 100;
const CSTATE_SCANNING: u32 = 101;
const DEFAULT_QUALITY: u8 = 75;
const BYTES_PER_PIXEL: usize = 4;

/// The rows of one compression, collected until `jpeg_finish_compress`.
#[derive(Default)]
struct Compress {
    width: u32,
    height: u32,
    rows: Vec<u8>,
}

static STATE: Mutex<Option<HashMap<u32, Compress>>> = Mutex::new(None);

fn with_state<R>(f: impl FnOnce(&mut HashMap<u32, Compress>) -> R) -> R {
    let mut s = STATE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    f(s.get_or_insert_with(HashMap::new))
}

fn rd(base: u32, off: u32) -> u32 {
    GPtr::<u32>::from_addr(base.wrapping_add(off) as usize).read()
}

fn wr(base: u32, off: u32, v: u32) {
    GPtr::<u32>::from_addr(base.wrapping_add(off) as usize).write(v);
}

/// `jpeg_std_error(err)` (0x5CD810): the error manager is returned as is; its methods are never called (no
/// libjpeg code runs that could report an error).
fn jpeg_std_error(err: u32) -> u32 {
    err
}

/// `jpeg_CreateCompress(cinfo, version, structsize)` (0x5CF960): the struct zeroed except `err` and `client_data`,
/// `input_gamma` 1.0, `global_state` CSTATE_START. A version or size other than libjpeg 6b's is not supported.
fn jpeg_create_compress(c: u32, version: u32, size: u32) {
    assert!(
        version == JPEG_LIB_VERSION && size == cinfo::SIZE,
        "libjpeg: jpeg_CreateCompress(version {version}, size {size:#x}) is not libjpeg 6b's"
    );
    let (err, client) = (rd(c, cinfo::ERR), rd(c, cinfo::CLIENT_DATA));
    for off in (0..cinfo::SIZE).step_by(4) {
        wr(c, off, 0);
    }
    wr(c, cinfo::ERR, err);
    wr(c, cinfo::CLIENT_DATA, client);
    GPtr::<f64>::from_addr(c.wrapping_add(cinfo::INPUT_GAMMA) as usize).write(1.0);
    wr(c, cinfo::GLOBAL_STATE, CSTATE_START);
    with_state(|s| s.insert(c, Compress::default()));
}

/// `jpeg_set_defaults(cinfo)` (0x5C7590): 8-bit samples; the quality and sampling defaults are applied when encoding.
fn jpeg_set_defaults(c: u32) {
    wr(c, cinfo::DATA_PRECISION, 8);
}

/// `jpeg_start_compress(cinfo, write_all_tables)` (0x5C26B0): the image size taken, `next_scanline` 0, then the
/// destination's `init_destination`.
fn jpeg_start_compress(c: u32, _write_all_tables: u32) {
    assert_eq!(rd(c, cinfo::INPUT_COMPONENTS) as usize, BYTES_PER_PIXEL, "libjpeg: only 4-byte RGB rows are supported");
    let (width, height) = (rd(c, cinfo::IMAGE_WIDTH), rd(c, cinfo::IMAGE_HEIGHT));
    with_state(|s| {
        s.insert(c, Compress { width, height, rows: Vec::with_capacity(width as usize * height as usize * BYTES_PER_PIXEL) })
    });
    wr(c, cinfo::GLOBAL_STATE, CSTATE_SCANNING);
    wr(c, cinfo::NEXT_SCANLINE, 0);
    let d = rd(c, cinfo::DEST);
    dest_callback(rd(d, dest::INIT_DESTINATION), c);
}

/// `jpeg_write_scanlines(cinfo, rows, n)` (0x5C2730): up to `n` rows (fewer at the image's end); returns how many
/// were taken, `next_scanline` advanced by that.
fn jpeg_write_scanlines(c: u32, rows: u32, n: u32) -> u32 {
    let next = rd(c, cinfo::NEXT_SCANLINE);
    with_state(|s| {
        let Some(st) = s.get_mut(&c) else { return 0 };
        let take = n.min(st.height.saturating_sub(next));
        let len = st.width as usize * BYTES_PER_PIXEL;
        for i in 0..take {
            GPtr::<u8>::from_addr(rd(rows, 4 * i) as usize).with_bytes(len, |row| st.rows.extend_from_slice(row));
        }
        wr(c, cinfo::NEXT_SCANLINE, next + take);
        take
    })
}

/// `jpeg_finish_compress(cinfo)` (0x5CFAC0): the image encoded and written through the destination manager, then
/// `term_destination`; `global_state` back to CSTATE_START.
fn jpeg_finish_compress(c: u32) {
    let optimize = rd(c, cinfo::OPTIMIZE_CODING) != 0;
    let Some(st) = with_state(|s| s.get_mut(&c).map(std::mem::take)) else {
        return;
    };
    let jpeg = encode(st.width, st.height, &st.rows, optimize).unwrap_or_else(|e| panic!("libjpeg: {e}"));
    let d = rd(c, cinfo::DEST);
    let mut left = jpeg.as_slice();
    while !left.is_empty() {
        if rd(d, dest::FREE_IN_BUFFER) == 0 {
            dest_callback(rd(d, dest::EMPTY_OUTPUT_BUFFER), c);
            continue;
        }
        let n = (rd(d, dest::FREE_IN_BUFFER) as usize).min(left.len());
        let out = rd(d, dest::NEXT_OUTPUT_BYTE);
        for (i, &b) in left[..n].iter().enumerate() {
            GPtr::<u8>::from_addr(out as usize + i).write(b);
        }
        wr(d, dest::NEXT_OUTPUT_BYTE, out + n as u32);
        wr(d, dest::FREE_IN_BUFFER, rd(d, dest::FREE_IN_BUFFER) - n as u32);
        left = &left[n..];
    }
    dest_callback(rd(d, dest::TERM_DESTINATION), c);
    wr(c, cinfo::GLOBAL_STATE, CSTATE_START);
}

/// `jpeg_destroy_compress(cinfo)` (0x5CFA30): the collected state dropped, `global_state` 0.
fn jpeg_destroy_compress(c: u32) {
    with_state(|s| s.remove(&c));
    wr(c, cinfo::GLOBAL_STATE, 0);
}

/// `width` x `height` pixels of 4 bytes (R, G, B, unused) as a baseline JPEG with libjpeg's default settings.
pub fn encode(width: u32, height: u32, rgbx: &[u8], optimize: bool) -> Result<Vec<u8>, String> {
    let (w, h) = (u16::try_from(width).map_err(|_| "width")?, u16::try_from(height).map_err(|_| "height")?);
    let mut out = Vec::new();
    let mut enc = Encoder::new(&mut out, DEFAULT_QUALITY);
    enc.set_sampling_factor(SamplingFactor::F_2_2);
    enc.set_optimized_huffman_tables(optimize);
    enc.encode(rgbx, w, h, ColorType::Rgba).map_err(|e| e.to_string())?;
    Ok(out)
}

/// `(address, name, native entry)` of the libjpeg functions.
pub fn entries() -> Vec<(u32, &'static str, usize)> {
    use crate::native::cdecl_entry as entry;
    vec![
        entry!(0x005C_D810, "libjpeg jpeg_std_error", jpeg_std_error; fn(err: u32) -> u32),
        entry!(0x005C_F960, "libjpeg jpeg_CreateCompress", jpeg_create_compress; fn(c: u32, version: u32, size: u32) -> ()),
        entry!(0x005C_7590, "libjpeg jpeg_set_defaults", jpeg_set_defaults; fn(c: u32) -> ()),
        entry!(0x005C_26B0, "libjpeg jpeg_start_compress", jpeg_start_compress; fn(c: u32, all: u32) -> ()),
        entry!(0x005C_2730, "libjpeg jpeg_write_scanlines", jpeg_write_scanlines; fn(c: u32, rows: u32, n: u32) -> u32),
        entry!(0x005C_FAC0, "libjpeg jpeg_finish_compress", jpeg_finish_compress; fn(c: u32) -> ()),
        entry!(0x005C_FA30, "libjpeg jpeg_destroy_compress", jpeg_destroy_compress; fn(c: u32) -> ()),
    ]
}

#[cfg(test)]
mod tests {
    use super::encode;

    #[test]
    fn encoded_picture_decodes_to_the_same_image() {
        let (w, h) = (64u32, 48u32);
        let mut px = Vec::new();
        for y in 0..h {
            for x in 0..w {
                px.extend_from_slice(&[(x * 4) as u8, (y * 5) as u8, 0x80, 0xFF]);
            }
        }
        let jpeg = encode(w, h, &px, true).expect("encode");
        assert_eq!(&jpeg[..2], &[0xFF, 0xD8]);
        assert_eq!(&jpeg[jpeg.len() - 2..], &[0xFF, 0xD9]);
        let mut dec = jpeg_decoder::Decoder::new(jpeg.as_slice());
        let rgb = dec.decode().expect("decode");
        let info = dec.info().expect("info");
        assert_eq!(info.pixel_format, jpeg_decoder::PixelFormat::RGB24);
        assert_eq!((u32::from(info.width), u32::from(info.height)), (w, h));
        assert_eq!(rgb.len(), (w * h * 3) as usize);
        let worst = px
            .chunks(4)
            .zip(rgb.chunks(3))
            .flat_map(|(a, b)| (0..3).map(move |i| (i32::from(a[i]) - i32::from(b[i])).abs()))
            .max()
            .unwrap();
        assert!(worst <= 24, "largest channel error {worst}");
    }
}
