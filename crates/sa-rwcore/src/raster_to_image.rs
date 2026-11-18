//! Locked D3D9 raster pixels to a 32-bit `RwImage`, and the per-format pixel-to-RGBA converters.
//!
//! `RwRaster`: +0 parent, +4 cpPixels, +8 palette, +0xC width, +0x10 height, +0x14 depth, +0x18 stride,
//! +0x23 cFormat; the D3D format at `parent + [0xB4E9E0] + 0x18`. `RwImage`: +0x10 stride, +0x14 cpPixels.

use sa_core::abi::{HookReg, NativeReg};
use sa_core::bindings::rw_error1;
use sa_core::mem::GPtr;
use sa_core::{hook, native, orig};

use crate::common::{g, p8, rd, wr};
use crate::image2::{call_converter, error_set_code};
use crate::raster::D3D9_RASTER_EXT_OFFSET;
use crate::palquant::{rw_pal_quant_add_image, rw_pal_quant_init, rw_pal_quant_match_image, rw_pal_quant_resolve_palette, rw_pal_quant_term};

/// E_RW_INVRASTERDEPTH.
const E_RW_INVALID_DEPTH: u32 = 0x8000_000C;

orig! {
    /// A pixel-to-RGBA converter selected by [`d3d9_convert_raster_to_image`]: `conv(RwRGBA* dst, pixel)`
    fn call_pixel_to_rgba(f: u32, dst: GPtr<u8>, pixel: u32) = cdecl dyn => unported;
}

/// The D3D format of the raster's top-level parent.
fn parent_d3d_format(r: GPtr<u8>) -> u32 {
    let parent = rd::<u32>(r, 0);
    rd::<u32>(p8(parent.wrapping_add(g::<i32>(D3D9_RASTER_EXT_OFFSET).read() as u32)), 0x18)
}

/// Visits every raster pixel (rows `0..height`, `width` pixels each, both re-read per row; a zero width
/// wraps as the original's `dec / jne` count does) with the matching 4-byte image pixel.
fn for_each_pixel(img: GPtr<u8>, r: GPtr<u8>, src_step: isize, mut f: impl FnMut(GPtr<u8>, GPtr<u8>)) {
    let mut y = 0i32;
    while y < rd::<i32>(r, 0x10) {
        let mut src = p8(rd::<i32>(r, 0x18).wrapping_mul(y).wrapping_add(rd::<i32>(r, 4)) as u32);
        let mut dst = p8(rd::<i32>(img, 0x10).wrapping_mul(y).wrapping_add(rd::<i32>(img, 0x14)) as u32);
        let mut n = rd::<u32>(r, 0xC);
        loop {
            f(src, dst);
            src = src.byte_add(src_step);
            dst = dst.byte_add(4);
            n = n.wrapping_sub(1);
            if n == 0 {
                break;
            }
        }
        y += 1;
    }
}

fn invalid_depth() {
    error_set_code(rw_error1(E_RW_INVALID_DEPTH));
}

fn put_rgba(dst: GPtr<u8>, r: u8, g: u8, b: u8, a: u8) {
    wr::<u8>(dst, 0, r);
    wr::<u8>(dst, 1, g);
    wr::<u8>(dst, 2, b);
    wr::<u8>(dst, 3, a);
}

fn rgb555(dst: GPtr<u8>, v: u32, a: u8) {
    put_rgba(dst, (v >> 7) as u8 & 0xF8, (v >> 2) as u8 & 0xF8, (v as u8) << 3, a);
}

fn rgb565(dst: GPtr<u8>, v: u32) {
    put_rgba(dst, (v >> 8) as u8 & 0xF8, (v >> 3) as u8 & 0xFC, (v as u8) << 3, 0xFF);
}

fn argb4444(dst: GPtr<u8>, v: u32) {
    let a = if v & 0xF000 == 0xF000 { 0xFF } else { (v >> 8) as u8 & 0xF0 };
    put_rgba(dst, (v >> 4) as u8 & 0xF0, v as u8 & 0xF0, (v as u8) << 4, a);
}

/// `_rwD3D9ImageGetRasterBody(image, raster)` (0x7FF450): the pixels of a locked raster into a 32-bit image.
/// Rasters without a cFormat nibble go through [`d3d9_convert_raster_to_image`].
#[hook(addr = 0x7FF450, conv = cdecl, name = "FUN_007ff450", expect = "83 EC 08 53 55",
       unverified = "frontier2-c29", route = "L2", hazards = "heap-writes")]
pub fn d3d9_image_get_raster32(img: GPtr<u8>, r: GPtr<u8>) {
    let nibble = u32::from(rd::<u8>(r, 0x23) & 0xF);
    if nibble == 0 {
        d3d9_convert_raster_to_image(img, r);
        return;
    }
    match rd::<u32>(r, 0x14) {
        4 | 8 => {
            let pal = rd::<u32>(r, 8);
            for_each_pixel(img, r, 1, |s, d| {
                let i = u32::from(rd::<u8>(s, 0));
                let px = if pal != 0 { rd::<u32>(p8(pal), (i * 4) as isize) } else { 0xFF00_0000 | i << 16 | i << 8 | i };
                wr::<u32>(d, 0, px);
            });
        }
        0x10 => match nibble {
            1 => for_each_pixel(img, r, 2, |s, d| {
                let v = u32::from(rd::<u16>(s, 0));
                rgb555(d, v, if v & 0x8000 != 0 { 0xFF } else { 0 });
            }),
            2 => for_each_pixel(img, r, 2, |s, d| rgb565(d, u32::from(rd::<u16>(s, 0)))),
            3 => for_each_pixel(img, r, 2, |s, d| argb4444(d, u32::from(rd::<u16>(s, 0)))),
            0xA => for_each_pixel(img, r, 2, |s, d| rgb555(d, u32::from(rd::<u16>(s, 0)), 0xFF)),
            _ => {}
        },
        0x18 | 0x20 => match parent_d3d_format(r) {
            0x16 => for_each_pixel(img, r, 4, |s, d| {
                let v = rd::<u32>(s, 0);
                put_rgba(d, (v >> 16) as u8, (v >> 8) as u8, v as u8, 0xFF);
            }),
            0x15 => for_each_pixel(img, r, 4, |s, d| {
                let v = rd::<u32>(s, 0);
                put_rgba(d, (v >> 16) as u8, (v >> 8) as u8, v as u8, (v >> 24) as u8);
            }),
            0x23 => for_each_pixel(img, r, 4, |s, d| {
                let v = rd::<u32>(s, 0);
                put_rgba(d, (v >> 22) as u8, (v >> 12) as u8, (v >> 2) as u8, ((v >> 30) as u8).wrapping_mul(0x55));
            }),
            _ => d3d9_convert_raster_to_image(img, r),
        },
        _ => invalid_depth(),
    }
}

/// The converter of a D3D format (0 for formats without one: the original then calls address 0).
fn pixel_to_rgba_converter(d3d_format: u32) -> u32 {
    match d3d_format {
        0x15 => 0x7FFDF0,
        0x16 => 0x7FFD90,
        0x17 => 0x7FFC10,
        0x18 => 0x7FFBE0,
        0x19 => 0x7FFC70,
        0x1A => 0x7FFCB0,
        0x1B => 0x7FFD30,
        0x1C => 0x7FFE70,
        0x1D => 0x7FFD00,
        0x1E => 0x7FFD60,
        0x20 | 0x3F => 0x7FFE20,
        0x21 | 0x3E => 0x7FFDB0,
        0x32 => 0x7FFE50,
        0x33 => 0x7FFEC0,
        0x34 => 0x7FFE90,
        0x3C | 0x75 => 0x7FFDD0,
        0x3D => 0x7FFC40,
        0x51 => 0x7FFEE0,
        _ => 0,
    }
}

/// `_rwD3D9ConvertRasterToImage(image, raster)` (0x7FF8C0): every raster pixel through the converter of
/// the parent's D3D format.
#[hook(addr = 0x7FF8C0, conv = cdecl, name = "FUN_007ff8c0", expect = "8B 0D E0 E9 B4 00",
       unverified = "frontier2-c29", route = "L2", hazards = "heap-writes")]
pub fn d3d9_convert_raster_to_image(img: GPtr<u8>, r: GPtr<u8>) {
    let conv = pixel_to_rgba_converter(parent_d3d_format(r));
    match rd::<u32>(r, 0x14) {
        4 | 8 => for_each_pixel(img, r, 1, |s, d| call_pixel_to_rgba(conv, d, u32::from(rd::<u8>(s, 0)))),
        0x10 => for_each_pixel(img, r, 2, |s, d| call_pixel_to_rgba(conv, d, u32::from(rd::<u16>(s, 0)))),
        0x20 => for_each_pixel(img, r, 4, |s, d| call_pixel_to_rgba(conv, d, rd::<u32>(s, 0))),
        _ => invalid_depth(),
    }
}

/// D3DFMT_X1R5G5B5 to RGBA (0x7FFBE0).
#[native(addr = 0x7FFBE0, conv = cdecl, name = "(missed by ghidra: data-code-immediate)")]
pub fn x1r5g5b5_to_rgba(dst: GPtr<u8>, v: u32) {
    rgb555(dst, v, 0xFF);
}

/// D3DFMT_R5G6B5 to RGBA (0x7FFC10).
#[hook(addr = 0x7FFC10, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 08 8B 4C 24 04",
       unverified = "frontier2-c29", route = "L2", hazards = "heap-writes")]
pub fn r5g6b5_to_rgba(dst: GPtr<u8>, v: u32) {
    rgb565(dst, v);
}

/// D3DFMT_L6V5U5 read as packed colour fields (0x7FFC40).
#[native(addr = 0x7FFC40, conv = cdecl, name = "(missed by ghidra: data-code-immediate)")]
pub fn l6v5u5_to_rgba(dst: GPtr<u8>, v: u32) {
    put_rgba(dst, (v as u8) << 3, (v >> 2) as u8 & 0xF8, (v >> 8) as u8 & 0xFC, 0xFF);
}

/// D3DFMT_A1R5G5B5 to RGBA (0x7FFC70).
#[hook(addr = 0x7FFC70, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 08 8B 4C 24 04",
       unverified = "frontier2-c29", route = "L2", hazards = "heap-writes")]
pub fn a1r5g5b5_to_rgba(dst: GPtr<u8>, v: u32) {
    rgb555(dst, v, if v & 0x8000 != 0 { 0xFF } else { 0 });
}

/// D3DFMT_A4R4G4B4 to RGBA (0x7FFCB0).
#[hook(addr = 0x7FFCB0, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 08 8B C8",
       unverified = "frontier2-c29", route = "L2", hazards = "heap-writes")]
pub fn a4r4g4b4_to_rgba(dst: GPtr<u8>, v: u32) {
    argb4444(dst, v);
}

fn r3g3b2(dst: GPtr<u8>, v: u32, a: u8) {
    put_rgba(dst, v as u8 & 0xE0, (v as u8 & 0xFC) << 3, (v as u8) << 6, a);
}

/// D3DFMT_A8R3G3B2 to RGBA (0x7FFD00).
#[hook(addr = 0x7FFD00, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 08 8B 4C 24 04",
       unverified = "frontier2-c29", route = "L2", hazards = "heap-writes")]
pub fn a8r3g3b2_to_rgba(dst: GPtr<u8>, v: u32) {
    r3g3b2(dst, v, (v >> 8) as u8);
}

/// D3DFMT_R3G3B2 to RGBA (0x7FFD30).
#[hook(addr = 0x7FFD30, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8A 44 24 08 8B 4C 24 04",
       unverified = "frontier2-c29", route = "L2", hazards = "heap-writes")]
pub fn r3g3b2_to_rgba(dst: GPtr<u8>, v: u32) {
    r3g3b2(dst, v, 0xFF);
}

/// D3DFMT_X4R4G4B4 to RGBA (0x7FFD60).
#[hook(addr = 0x7FFD60, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 08 8B 4C 24 04",
       unverified = "frontier2-c29", route = "L2", hazards = "heap-writes")]
pub fn x4r4g4b4_to_rgba(dst: GPtr<u8>, v: u32) {
    put_rgba(dst, (v >> 4) as u8 & 0xF0, v as u8 & 0xF0, (v as u8) << 4, 0xFF);
}

/// D3DFMT_L8 to RGBA (0x7FFE50).
#[hook(addr = 0x7FFE50, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 04 8A 4C 24 08",
       unverified = "frontier2-c29", route = "L2", hazards = "heap-writes")]
pub fn l8_to_rgba(dst: GPtr<u8>, v: u32) {
    let l = v as u8;
    put_rgba(dst, l, l, l, 0xFF);
}

/// D3DFMT_A8 to RGBA (0x7FFE70).
#[hook(addr = 0x7FFE70, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 04 B1 FF 88 08",
       unverified = "frontier2-c29", route = "L2", hazards = "heap-writes")]
pub fn a8_to_rgba(dst: GPtr<u8>, v: u32) {
    put_rgba(dst, 0xFF, 0xFF, 0xFF, v as u8);
}

/// D3DFMT_A4L4 to RGBA (0x7FFE90); the alpha test compares the whole dword.
#[hook(addr = 0x7FFE90, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 4C 24 08 8B 54 24 04",
       unverified = "frontier2-c29", route = "L2", hazards = "heap-writes")]
pub fn a4l4_to_rgba(dst: GPtr<u8>, v: u32) {
    let l = (v as u8) << 4;
    put_rgba(dst, l, l, l, if v >= 0xF0 { 0xFF } else { v as u8 & 0xF0 });
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_D3D9_IMAGE_GET_RASTER32,
    HOOK_D3D9_CONVERT_RASTER_TO_IMAGE,
    HOOK_R5G6B5_TO_RGBA,
    HOOK_A1R5G5B5_TO_RGBA,
    HOOK_A4R4G4B4_TO_RGBA,
    HOOK_A8R3G3B2_TO_RGBA,
    HOOK_R3G3B2_TO_RGBA,
    HOOK_X4R4G4B4_TO_RGBA,
    HOOK_L8_TO_RGBA,
    HOOK_A8_TO_RGBA,
    HOOK_A4L4_TO_RGBA,
    HOOK_D3D9_RASTER_SET_IMAGE_32,
    HOOK_D3D9_RASTER_SET_IMAGE_PAL,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_X1R5G5B5_TO_RGBA, NATIVE_L6V5U5_TO_RGBA];

/// Per D3D format, a dword whose low byte is 0xFF for formats without alpha.
const FORMAT_HAS_ALPHA: usize = 0xB4E7E0;

/// Visits every pixel of a raster row (`width` pixels, rows `0..height`, both re-read) with the image
/// pixel feeding it.
fn for_each_raster_pixel(r: GPtr<u8>, img: GPtr<u8>, src_step: isize, mut f: impl FnMut(GPtr<u8>) -> u32) {
    let mut y = 0i32;
    while y < rd::<i32>(r, 0x10) {
        let mut src = p8(rd::<i32>(img, 0x10).wrapping_mul(y).wrapping_add(rd::<i32>(img, 0x14)) as u32);
        let mut dst = p8(rd::<i32>(r, 0x18).wrapping_mul(y).wrapping_add(rd::<i32>(r, 4)) as u32);
        let mut x = 0i32;
        while x < rd::<i32>(r, 0xC) {
            wr::<u32>(dst, 0, f(src));
            src = src.byte_add(src_step);
            dst = dst.byte_add(4);
            x += 1;
        }
        y += 1;
    }
}

fn xrgb_from_rgba(p: GPtr<u8>) -> u32 {
    0xFF00_0000 | u32::from(rd::<u8>(p, 0)) << 16 | u32::from(rd::<u8>(p, 1)) << 8 | u32::from(rd::<u8>(p, 2))
}

fn argb_from_rgba(p: GPtr<u8>) -> u32 {
    u32::from(rd::<u8>(p, 3)) << 24 | u32::from(rd::<u8>(p, 0)) << 16 | u32::from(rd::<u8>(p, 1)) << 8 | u32::from(rd::<u8>(p, 2))
}

/// The alpha flag of the raster's D3D9 extension (+8), and of its parent's when it has one.
fn set_raster_alpha(r: GPtr<u8>, ext: GPtr<u8>, alpha: bool) {
    wr::<u8>(ext, 8, u8::from(alpha));
    if !alpha {
        return;
    }
    let parent = rd::<u32>(r, 0);
    if parent != r.addr() as u32 {
        wr::<u8>(p8(parent.wrapping_add(g::<i32>(D3D9_RASTER_EXT_OFFSET).read() as u32)), 8, 1);
    }
}

/// `_rwD3D9RasterSetImagePalettized(raster, image)` (0x801390): a 4/8-bit (through its palette) or 32-bit
/// image into a locked 24/32-bit raster, then the raster's alpha flag.
#[hook(addr = 0x801390, conv = cdecl, name = "FUN_00801390", expect = "81 EC 10 04 00 00",
       unverified = "frontier2-c29", route = "L2", hazards = "heap-writes,uninit-locals",
       note = "a 4-bit image's indices above 15 read the never-written part of the 256-entry stack table (port: zeros)")]
pub fn d3d9_raster_set_image_32(r: GPtr<u8>, img: GPtr<u8>) {
    let ext = r.byte_add(g::<i32>(D3D9_RASTER_EXT_OFFSET).read() as isize);
    let depth = rd::<u32>(img, 0xC);
    let nibble = rd::<u8>(r, 0x23) & 0xF;
    let converted = |r: GPtr<u8>| {
        let fmt = rd::<u32>(r.byte_add(g::<i32>(D3D9_RASTER_EXT_OFFSET).read() as isize), 0x18);
        (crate::frontier_b::rgba_to_pixel_converter(fmt), rd::<u8>(p8(FORMAT_HAS_ALPHA as u32), fmt.wrapping_mul(4) as isize) != 0xFF)
    };
    let alpha = match depth {
        4 | 8 => {
            let mut lut = [0u32; 0x100];
            let pal = p8(rd::<u32>(img, 0x18));
            let alpha = match nibble {
                6 => {
                    for (i, e) in lut.iter_mut().enumerate().take(1 << depth) {
                        *e = xrgb_from_rgba(pal.byte_add(i as isize * 4));
                    }
                    false
                }
                5 => {
                    for (i, e) in lut.iter_mut().enumerate().take(1 << depth) {
                        *e = argb_from_rgba(pal.byte_add(i as isize * 4));
                    }
                    true
                }
                _ => {
                    let (conv, alpha) = converted(r);
                    let mut i = 0usize;
                    while i < 1 << rd::<u32>(img, 0xC) {
                        lut[i] = call_converter(conv, p8(rd::<u32>(img, 0x18)).byte_add(i as isize * 4));
                        i += 1;
                    }
                    alpha
                }
            };
            for_each_raster_pixel(r, img, 1, |p| lut[usize::from(rd::<u8>(p, 0))]);
            alpha
        }
        0x20 => match nibble {
            6 => {
                for_each_raster_pixel(r, img, 4, xrgb_from_rgba);
                false
            }
            5 => {
                for_each_raster_pixel(r, img, 4, argb_from_rgba);
                true
            }
            _ => {
                let (conv, alpha) = converted(r);
                for_each_raster_pixel(r, img, 4, |p| call_converter(conv, p));
                alpha
            }
        },
        _ => true,
    };
    set_raster_alpha(r, ext, alpha);
}

/// Luminance of an RGBA colour, `(30 r + 59 g + 11 b) / 100`.
fn luminance(p: GPtr<u8>) -> u32 {
    (u32::from(rd::<u8>(p, 1)) * 0x3B + u32::from(rd::<u8>(p, 2)) * 0xB + u32::from(rd::<u8>(p, 0)) * 0x1E) / 100
}

/// One byte of an 8-bit non-palettised raster (by the D3D format) from an RGBA colour.
fn byte_pixel(d3d_format: u32, p: GPtr<u8>) -> u8 {
    let c = |k: isize| rd::<u8>(p, k);
    match d3d_format {
        0x1B => c(0) & 0xE0 | (c(1) & 0xE0) >> 3 | c(2) >> 6,
        0x34 => (luminance(p) >> 4) as u8 | c(3) & 0xF0,
        0x1C => c(3),
        _ => (luminance(p) * u32::from(c(3)) / 0xFF) as u8,
    }
}

/// Visits every byte of an 8-bit raster with the image pixel feeding it (`src_step` bytes apart).
fn for_each_raster_byte(r: GPtr<u8>, img: GPtr<u8>, src_step: isize, mut f: impl FnMut(GPtr<u8>) -> u8) {
    let mut y = 0i32;
    while y < rd::<i32>(r, 0x10) {
        let mut dst = p8(rd::<i32>(r, 0x18).wrapping_mul(y).wrapping_add(rd::<i32>(r, 4)) as u32);
        let mut src = p8(rd::<i32>(img, 0x10).wrapping_mul(y).wrapping_add(rd::<i32>(img, 0x14)) as u32);
        let mut x = 0i32;
        while x < rd::<i32>(r, 0xC) {
            wr::<u8>(dst, 0, f(src));
            dst = dst.byte_add(1);
            src = src.byte_add(src_step);
            x += 1;
        }
        y += 1;
    }
}

/// `_rwD3D9RasterSetImageMipmaps(raster, image)` (0x800C80): an image into a locked 4/8-bit raster. An
/// 8-bit non-palettised (LUM8-class) raster gets one byte per pixel by its D3D format (R3G3B2, A4L4, A8,
/// else luminance times alpha), through the image palette for 4/8-bit images. A palettised raster takes
/// a 4/8-bit image's indices and (unless the raster's palette is shared) its palette; a 32-bit image is
/// quantised to the raster's depth first (an opaque result clears the raster's alpha flag).
#[hook(addr = 0x800C80, conv = cdecl, name = "FUN_00800c80", expect = "B8 0C 41 00 00",
       unverified = "frontier2-c34", route = "L2", hazards = "heap-writes,uninit-locals",
       note = "a 4-bit image's indices above 15 read the never-written part of the 256-entry stack table (port: zeros); the quantiser is a zeroed local in the port")]
pub fn d3d9_raster_set_image_pal(r: GPtr<u8>, img: GPtr<u8>) {
    let ext = r.byte_add(g::<i32>(D3D9_RASTER_EXT_OFFSET).read() as isize);
    let image_depth = rd::<u32>(img, 0xC);
    let indexed = image_depth == 4 || image_depth == 8;
    if rd::<u8>(r, 0x23) & 0xF == 4 {
        let fmt = rd::<u32>(ext, 0x18);
        if indexed {
            let mut lut = [0u8; 0x100];
            let pal = p8(rd::<u32>(img, 0x18));
            for (i, e) in lut.iter_mut().enumerate().take(1 << image_depth) {
                *e = byte_pixel(fmt, pal.byte_add(i as isize * 4));
            }
            for_each_raster_byte(r, img, 1, |s| lut[usize::from(rd::<u8>(s, 0))]);
        } else {
            for_each_raster_byte(r, img, 4, |s| byte_pixel(fmt, s));
        }
        return;
    }
    if indexed {
        let stride = rd::<i32>(r, 0x18);
        if rd::<i32>(img, 0x10) == stride && rd::<i32>(r, 0xC) == stride {
            let n = rd::<i32>(r, 0x10).wrapping_mul(rd::<i32>(r, 0xC)) as u32;
            crate::image2::rep_movs(p8(rd::<u32>(r, 4)), p8(rd::<u32>(img, 0x14)), n);
        } else {
            let mut y = 0i32;
            while y < rd::<i32>(r, 0x10) {
                let src = rd::<i32>(img, 0x10).wrapping_mul(y).wrapping_add(rd::<i32>(img, 0x14));
                let dst = rd::<i32>(r, 0x18).wrapping_mul(y).wrapping_add(rd::<i32>(r, 4));
                crate::image2::rep_movs(p8(dst as u32), p8(src as u32), rd::<u32>(r, 0xC));
                y += 1;
            }
        }
        let pal = p8(rd::<u32>(r, 8));
        if rd::<u8>(ext, 0xB) == 0 && !pal.is_null() {
            crate::image2::rep_movs(pal, p8(rd::<u32>(img, 0x18)), (1u32 << (image_depth & 0x1F)) << 2);
        }
        return;
    }
    let depth = rd::<u32>(r, 0x14);
    let q_img = crate::bindings::rw_image_create(rd::<i32>(img, 4), rd::<i32>(img, 8), depth as i32);
    if q_img.is_null() {
        return;
    }
    crate::bindings::image_allocate_pixels(q_img);
    let mut pq = [0u8; 0x4008];
    let q = GPtr::<u8>::from_raw(pq.as_mut_ptr());
    if rw_pal_quant_init(q) == 0 {
        return;
    }
    rw_pal_quant_add_image(q, img, 1.0);
    rw_pal_quant_resolve_palette(p8(rd::<u32>(q_img, 0x18)), 1 << (depth & 0x1F), q);
    rw_pal_quant_match_image(p8(rd::<u32>(q_img, 0x14)), rd::<i32>(q_img, 0x10), rd::<u32>(q_img, 0xC), 0, q, img);
    rw_pal_quant_term(q);
    if rd::<u8>(ext, 8) != 0 && crate::frontier_c::d3d9_image_find_format(q_img) & 0xF00 == 0x600 {
        wr::<u8>(r, 0x23, rd::<u8>(r, 0x23) & 0xFE | 6);
        wr::<u8>(ext, 8, 0);
    }
    d3d9_raster_set_image_pal(r, q_img);
    crate::bindings::image_destroy(q_img);
}
