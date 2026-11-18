//! `RtPNGImageWrite`: an `RwImage` to a PNG file through the statically linked libpng 1.0.12, which is
//! third-party code called where it lies in the exe.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{rw_stream_close, rw_stream_open};
use crate::common::{p8, rd, rw_free, rw_malloc, wr};

/// The libpng version string "1.0.12".
const PNG_VERSION: u32 = 0x8D_EDF4;
/// The png_struct callbacks of RtPNG: allocation, release, stream write and (empty) flush.
const PNG_MALLOC_FN: u32 = 0x7C_F940;
const PNG_FREE_FN: u32 = 0x7C_F960;
const PNG_WRITE_FN: u32 = 0x7C_F980;
const PNG_FLUSH_FN: u32 = 0x7C_F9A0;
const PNG_COLOR_TYPE_PALETTE: u32 = 3;
const PNG_COLOR_TYPE_RGB_ALPHA: u32 = 6;

orig! {
    /// libpng `png_create_write_struct_2(version, error_ptr, error_fn, warn_fn, mem_ptr, malloc_fn, free_fn)` (0x7D03B0)
    fn png_create_write_struct_2(ver: u32, err: u32, err_fn: u32, warn_fn: u32, mem: u32, malloc_fn: u32, free_fn: u32) -> u32 = cdecl 0x7D03B0 => unported;
    /// libpng `png_create_info_struct(png)` (0x7D2E80)
    fn png_create_info_struct(png: u32) -> u32 = cdecl 0x7D2E80 => unported;
    /// libpng `png_destroy_write_struct(&png, &info)` (0x7D0A40)
    fn png_destroy_write_struct(png: GPtr<u32>, info: GPtr<u32>) = cdecl 0x7D0A40 => unported;
    /// libpng `png_set_write_fn(png, io_ptr, write_fn, flush_fn)` (0x7D2C20)
    fn png_set_write_fn(png: u32, io: GPtr<u8>, write_fn: u32, flush_fn: u32) = cdecl 0x7D2C20 => unported;
    /// libpng `png_set_IHDR(png, info, width, height, bit_depth, color_type, interlace, compression, filter)` (0x7D1D60)
    fn png_set_ihdr(png: u32, info: u32, w: u32, h: u32, depth: u32, color: u32, interlace: u32, compression: u32, filter: u32) = cdecl 0x7D1D60 => unported;
    /// libpng `png_set_PLTE(png, info, palette, count)` (0x7D2130)
    fn png_set_plte(png: u32, info: u32, palette: GPtr<u8>, n: u32) = cdecl 0x7D2130 => unported;
    /// libpng `png_set_tRNS(png, info, trans, count, trans_values)` (0x7D2630)
    fn png_set_trns(png: u32, info: u32, trans: GPtr<u8>, n: u32, values: u32) = cdecl 0x7D2630 => unported;
    /// libpng `png_write_info(png, info)` (0x7CFFA0)
    fn png_write_info(png: u32, info: u32) = cdecl 0x7CFFA0 => unported;
    /// libpng `png_set_packing(png)` (0x7D1340)
    fn png_set_packing(png: u32) = cdecl 0x7D1340 => unported;
    /// libpng `png_write_image(png, rows)` (0x7D06C0)
    fn png_write_image(png: u32, rows: GPtr<u8>) = cdecl 0x7D06C0 => unported;
    /// libpng `png_write_end(png, info)` (0x7D0250)
    fn png_write_end(png: u32, info: u32) = cdecl 0x7D0250 => unported;
}

/// `RtPNGImageWrite(image, filename)` (0x7CF600): returns the image, or NULL when the file or libpng
/// could not be set up. 4/8-bit images are written palettised (the palette's alpha as tRNS), 32-bit
/// ones as RGBA. The original arms libpng's error longjmp with `setjmp` on the png_struct; a Rust frame
/// cannot be re-entered that way, so a libpng error here jumps to the stale target libpng's own
/// constructor left behind.
#[hook(addr = 0x7CF600, conv = cdecl, name = "RtPNGImageWrite", expect = "55 8B EC 83 EC 28",
       unverified = "frontier2-c31", route = "split", hazards = "heap-writes",
       note = "file writes; no setjmp: a libpng error (out of memory, bad depth) does not return NULL as in the original")]
pub fn rt_png_image_write(img: GPtr<u8>, filename: GPtr<u8>) -> GPtr<u8> {
    if filename.is_null() || img.is_null() {
        return img;
    }
    let stream = rw_stream_open(2, 2, filename);
    if stream.is_null() {
        return GPtr::NULL;
    }
    let mut png = png_create_write_struct_2(PNG_VERSION, 0, 0, 0, 0, PNG_MALLOC_FN, PNG_FREE_FN);
    if png == 0 {
        rw_stream_close(stream, GPtr::NULL);
        return GPtr::NULL;
    }
    let mut info = png_create_info_struct(png);
    if info == 0 {
        rw_stream_close(stream, GPtr::NULL);
        png_destroy_write_struct(GPtr::from_raw(&mut png), GPtr::NULL);
        return GPtr::NULL;
    }
    png_set_write_fn(png, stream, PNG_WRITE_FN, PNG_FLUSH_FN);
    let depth = rd::<u32>(img, 0xC);
    let width = rd::<u32>(img, 4);
    let height = rd::<i32>(img, 8);
    let (bits, color, palette) = match depth {
        4 => (4, PNG_COLOR_TYPE_PALETTE, rd::<u32>(img, 0x18)),
        8 => (8, PNG_COLOR_TYPE_PALETTE, rd::<u32>(img, 0x18)),
        0x20 => (8, PNG_COLOR_TYPE_RGB_ALPHA, 0),
        _ => (0, 0, 0),
    };
    png_set_ihdr(png, info, width, height as u32, bits, color, 0, 0, 0);
    let mut plte = GPtr::<u8>::NULL;
    let mut trns = GPtr::<u8>::NULL;
    if color == PNG_COLOR_TYPE_PALETTE {
        let n = 1i32 << depth;
        plte = rw_malloc((n * 3) as u32, 0x3010D);
        trns = rw_malloc(n as u32, 0x3010D);
        for i in 0..n.max(0) as isize {
            for c in 0..3 {
                wr::<u8>(plte, i * 3 + c, rd::<u8>(p8(palette), i * 4 + c));
            }
            wr::<u8>(trns, i, rd::<u8>(p8(palette), i * 4 + 3));
        }
        png_set_plte(png, info, plte, n as u32);
        png_set_trns(png, info, trns, n as u32, 0);
    }
    png_write_info(png, info);
    png_set_packing(png);
    let pixels = rd::<u32>(img, 0x14);
    let stride = rd::<u32>(img, 0x10);
    let rows = rw_malloc((height as u32) << 2, 0x3010D);
    if rows.is_null() {
        png_destroy_write_struct(GPtr::from_raw(&mut png), GPtr::NULL);
        rw_stream_close(stream, GPtr::NULL);
        return GPtr::NULL;
    }
    let mut row = pixels;
    for i in 0..height.max(0) as isize {
        wr::<u32>(rows, i * 4, row);
        row = row.wrapping_add(stride);
    }
    png_write_image(png, rows);
    png_write_end(png, info);
    if !plte.is_null() {
        rw_free(plte);
        rw_free(trns);
    }
    rw_free(rows);
    png_destroy_write_struct(GPtr::from_raw(&mut png), GPtr::from_raw(&mut info));
    rw_stream_close(stream, GPtr::NULL);
    img
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_RT_PNG_IMAGE_WRITE];
