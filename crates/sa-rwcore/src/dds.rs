//! Reading a DDS file into a D3D9 raster: the formats the device takes are read straight into the
//! raster's levels; every other one is decoded into a 32-bit image (DXT1 / DXT3 / DXT5 decompressed
//! here) and set on a raster of the closest 16 / 32-bit format.
//!
//! DDS header (0x7C bytes after the magic): +4 flags, +8 height, +0xC width, +0x10 pitch or linear
//! size, +0x18 mipmap count, +0x4C pixel-format flags, +0x50 FourCC, +0x54 bit count, +0x58 / +0x60 /
//! +0x64 R / B / A masks, +0x68 caps, +0x6C caps2. `RwRaster`: +0xC width, +0x10 height, +0x14 depth,
//! +0x21 private flags, +0x23 cFormat. Extension (raster + `[0xB4E9E0]`): +0 texture, +8 alpha, +9 cube
//! face (high nibble) / cube flag, +0xA compressed (high nibble), +0x10 locked pitch, +0x18 D3D format.
//! `RwImage`: +4 width, +8 height, +0x10 stride, +0x14 pixels.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{
    clamp_raster_size, image_allocate_pixels, image_destroy, raster_set_from_image, rw_image_create,
    rw_image_create_resample, rw_raster_create, rw_raster_lock, rw_raster_unlock, rw_stream_skip, stream_read,
};
use crate::common::{p8, rd, rw_free, rw_malloc, wr};
use crate::d3d9rast::{dev_create_cube_texture, dev_create_texture};
use crate::d3d9state::device;
use crate::image2::d3d9_decompress_dxt1;
use crate::raster::{
    d3d9_check_valid_cube_texture_format as cube_format_ok, d3d9_check_valid_texture_format as format_ok, ext,
    raster_create_from_dds_format, rw_raster_get_num_levels, tex_get_level_desc,
};

const DDS_MAGIC: u32 = 0x2053_4444;
const DXT1: u32 = 0x3154_5844;
const DXT2: u32 = 0x3254_5844;
const DXT3: u32 = 0x3354_5844;
const DXT4: u32 = 0x3454_5844;
const DXT5: u32 = 0x3554_5844;
/// `D3DFMT_A16B16G16R16`, read as 8 bits per channel.
const FOURCC_A16B16G16R16: u32 = 0x24;
/// The 4-byte D3D format table: +0 alpha, +1 depth, +3 cFormat bits.
const FORMAT_TABLE: u32 = 0xB4_E7E0;
/// The device caps' cube-map mipmap bits.
const DEVICE_CAPS: usize = 0xC9_BF44;
/// Every colour channel of a 32-bit pixel, alpha cleared.
const RGB_MASK: u32 = 0x00FF_FFFF;

orig! {
    /// `_rwD3D9DecompressDXT5Alpha(dst, block, width, rgbMask)` (0x81F0C0)
    fn decompress_dxt5_alpha(dst: GPtr<u8>, src: GPtr<u8>, width: i32, rgb_mask: u32) -> u32 = cdecl 0x81F0C0 => d3d9_decompress_dxt5_alpha;
    /// `_rwD3D9DecompressDXT3(width, height, dst, src)` (0x81EA70)
    fn decompress_dxt3(width: i32, height: i32, dst: GPtr<u8>, src: GPtr<u8>) -> u32 = cdecl 0x81EA70 => d3d9_decompress_dxt3;
    /// `_rwD3D9DecompressDXT5(width, height, dst, src)` (0x81EDC0)
    fn decompress_dxt5(width: i32, height: i32, dst: GPtr<u8>, src: GPtr<u8>) -> u32 = cdecl 0x81EDC0 => d3d9_decompress_dxt5;
}

fn is_dxt(fourcc: u32) -> bool {
    matches!(fourcc, DXT1 | DXT2 | DXT3 | DXT4 | DXT5)
}

fn table_byte(fmt: u32, field: u32) -> u8 {
    rd::<u8>(p8(FORMAT_TABLE.wrapping_add(fmt.wrapping_mul(4)).wrapping_add(field)), 0)
}

/// The colour half of a DXT3 / DXT5 block (`c` = block + 8) into its 4x4 pixels at `dst`. Unlike
/// DXT1, the three-colour mode's fourth colour is (0, 255, 255) with alpha 0; the alpha byte of
/// every colour is overwritten by the alpha half afterwards anyway.
fn decode_colour_block(c: GPtr<u8>, dst: u32, width: i32) {
    let w0 = u32::from(rd::<u16>(c, 0));
    let w1 = u32::from(rd::<u16>(c, 2));
    let (r0, g0, b0) = (((w0 >> 11) << 3) & 0xFF, ((w0 >> 5) << 2) & 0xFF, (w0 << 3) & 0xFF);
    let (r1, g1, b1) = (((w1 >> 11) << 3) & 0xFF, ((w1 >> 5) << 2) & 0xFF, (w1 << 3) & 0xFF);
    let rgba = |r: u32, g: u32, b: u32, a: u32| r | (g << 8) | (b << 16) | (a << 24);
    let c0 = rgba(r0, g0, b0, 0xFF);
    let c1 = rgba(r1, g1, b1, 0xFF);
    let (c2, c3) = if w0 > w1 {
        (
            rgba((r1 + 2 * r0) / 3, (g1 + 2 * g0) / 3, (b1 + 2 * b0) / 3, 0xFF),
            rgba((r0 + 2 * r1) / 3, (g0 + 2 * g1) / 3, (b0 + 2 * b1) / 3, 0xFF),
        )
    } else {
        (rgba((r1 + r0) / 2, (g1 + g0) / 2, (b1 + b0) / 2, 0xFF), rgba(0, 0xFF, 0xFF, 0))
    };
    let next_row = (width as u32).wrapping_mul(4).wrapping_sub(16);
    let mut p = dst;
    for row in 0..4isize {
        let mut idx = u32::from(rd::<u8>(c, 4 + row));
        for _ in 0..4 {
            let colour = match idx & 3 {
                0 => c0,
                1 => c1,
                2 => c2,
                _ => c3,
            };
            wr::<u32>(p8(p), 0, colour);
            p = p.wrapping_add(4);
            idx >>= 2;
        }
        p = p.wrapping_add(next_row);
    }
}

/// Walks the `width / 4` x `height / 4` blocks (signed divisions) of 16 bytes, calling `block(src,
/// dst)` with the block and its top-left pixel (row pitch `width * 4` bytes). Returns what the
/// originals leave in EAX: the block-row count when it is not positive, else 0.
fn for_each_block16(width: i32, height: i32, dst: GPtr<u8>, src: GPtr<u8>, mut block: impl FnMut(GPtr<u8>, u32)) -> u32 {
    let bw = width / 4;
    let bh = height / 4;
    if bh <= 0 {
        return bh as u32;
    }
    let mut drow = dst.addr() as u32;
    let mut srow = src.addr() as u32;
    for _ in 0..bh {
        if bw > 0 {
            let mut d = drow;
            let mut s = srow;
            for _ in 0..bw {
                block(p8(s), d);
                d = d.wrapping_add(16);
                s = s.wrapping_add(16);
            }
        }
        srow = srow.wrapping_add((bw as u32).wrapping_shl(4));
        drow = drow.wrapping_add((width as u32).wrapping_shl(4));
    }
    0
}

/// `_rwD3D9DecompressDXT3(width, height, dst, src)` (0x81EA70): DXT3 blocks (explicit 4-bit alpha at
/// +0, colour at +8) into 32-bit R, G, B, A pixels; each alpha nibble is widened as `a | a << 4`.
#[hook(addr = 0x81EA70, conv = cdecl, name = "FUN_0081ea70", expect = "83 EC 40 56 8B 74 24 48",
       unverified = "frontier2-c35", route = "L2", hazards = "heap-writes",
       note = "keeps colours / the alpha byte in its height, dst and src argument slots (not reproduced)")]
pub fn d3d9_decompress_dxt3(width: i32, height: i32, dst: GPtr<u8>, src: GPtr<u8>) -> u32 {
    let next_row = (width as u32).wrapping_mul(4).wrapping_sub(16);
    for_each_block16(width, height, dst, src, |b, d| {
        decode_colour_block(b.byte_add(8), d, width);
        let mut p = d;
        for row in 0..4isize {
            let mut bits = rd::<u16>(b, 2 * row);
            for _ in 0..4 {
                let nibble = bits as u8 & 0xF;
                let a = u32::from(nibble | nibble << 4) << 24;
                wr::<u32>(p8(p), 0, (rd::<u32>(p8(p), 0) & RGB_MASK) | a);
                p = p.wrapping_add(4);
                bits >>= 4;
            }
            p = p.wrapping_add(next_row);
        }
    })
}

/// `_rwD3D9DecompressDXT5(width, height, dst, src)` (0x81EDC0): DXT5 blocks, the colour half as
/// DXT3's, the interpolated alpha through 0x81F0C0.
#[hook(addr = 0x81EDC0, conv = cdecl, name = "FUN_0081edc0", expect = "83 EC 3C 56 8B 74 24 44",
       unverified = "frontier2-c35", route = "L2", hazards = "heap-writes",
       note = "keeps colours / the block pointer in its height, dst and src argument slots (not reproduced)")]
pub fn d3d9_decompress_dxt5(width: i32, height: i32, dst: GPtr<u8>, src: GPtr<u8>) -> u32 {
    for_each_block16(width, height, dst, src, |b, d| {
        decode_colour_block(b.byte_add(8), d, width);
        decompress_dxt5_alpha(p8(d), b, width, RGB_MASK);
    })
}

/// `_rwD3D9DecompressDXT5Alpha(dst, block, width, rgbMask)` (0x81F0C0): the eight alphas from a0 / a1
/// (a0 > a1: six interpolated in sevenths; else four in fifths, then 0 and 255), sixteen 3-bit
/// indices from the bytes +2..+7, each pixel `(pixel & rgbMask) | alpha << 24`. Returns the pointer
/// past the block's last row.
#[hook(addr = 0x81F0C0, conv = cdecl, name = "FUN_0081f0c0", expect = "83 EC 30 53 8B 5C 24 3C",
       unverified = "frontier2-c35", route = "L2", hazards = "heap-writes",
       note = "keeps the row step in its block argument slot (not reproduced)")]
pub fn d3d9_decompress_dxt5_alpha(dst: GPtr<u8>, src: GPtr<u8>, width: i32, rgb_mask: u32) -> u32 {
    let a0 = i32::from(rd::<u8>(src, 0));
    let a1 = i32::from(rd::<u8>(src, 1));
    let mut alpha = [a0, a1, 0, 0, 0, 0, 0, 0];
    if a0 > a1 {
        for (i, a) in alpha[2..].iter_mut().enumerate() {
            let k = i as i32 + 1;
            *a = ((7 - k) * a0 + k * a1) / 7;
        }
    } else {
        for (i, a) in alpha[2..6].iter_mut().enumerate() {
            let k = i as i32 + 1;
            *a = ((5 - k) * a0 + k * a1) / 5;
        }
        alpha[7] = 0xFF;
    }
    let lo = rd::<u32>(src, 2);
    let hi = rd::<u32>(src, 5);
    let index = |i: u32| if i < 8 { (lo >> (3 * i)) & 7 } else { (hi >> (3 * (i - 8))) & 7 };
    let next_row = (width as u32).wrapping_mul(4).wrapping_sub(16);
    let mut p = dst.addr() as u32;
    for row in 0..4u32 {
        for col in 0..4u32 {
            let a = alpha[index(row * 4 + col) as usize] as u32 & 0xFF;
            wr::<u32>(p8(p), 0, (rd::<u32>(p8(p), 0) & rgb_mask) | a << 24);
            p = p.wrapping_add(4);
        }
        p = p.wrapping_add(next_row);
    }
    p
}

/// Bytes per row of an uncompressed pixel format: by bit count, else 4 for an 0xFF0000 red mask and 2
/// for any other.
fn row_bytes(bits: u32, rmask: u32, width: u32) -> u32 {
    match bits {
        8 => width,
        0x10 => width.wrapping_mul(2),
        0x18 => width.wrapping_mul(3),
        0x20 => width.wrapping_mul(4),
        0x40 => width.wrapping_mul(8),
        _ if rmask == 0xFF_0000 => width.wrapping_mul(4),
        _ => width.wrapping_mul(2),
    }
}

/// The raster flags of a format the device may lack: `preferred` when `fmt` is a valid texture
/// format, else 0x300 (4444) when A4R4G4B4 is, else 0x100 (1555).
fn fallback_flags(preferred: Option<(u32, u32)>) -> u32 {
    match preferred {
        Some((fmt, flags)) if format_ok(fmt) != 0 => flags,
        _ if format_ok(0x1A) != 0 => 0x300,
        _ => 0x100,
    }
}

/// Every pixel of the `width` x `height` file data at `src` (rows `row` bytes apart, `step` bytes per
/// pixel) converted by `convert` into the 32-bit image.
fn convert_pixels(img: GPtr<u8>, src: u32, width: u32, height: u32, row: u32, step: u32, convert: impl Fn(GPtr<u8>) -> [u8; 4]) {
    let mut srow = src;
    for y in 0..height {
        let mut d = rd::<u32>(img, 0x10).wrapping_mul(y).wrapping_add(rd::<u32>(img, 0x14));
        for x in 0..width {
            let px = convert(p8(srow.wrapping_add(x.wrapping_mul(step))));
            wr::<u32>(p8(d), 0, u32::from_le_bytes(px));
            d = d.wrapping_add(4);
        }
        srow = srow.wrapping_add(row);
    }
}

/// The 10-bit channels at bit 22 / 12 / 2 (truncated to their low 8 bits) in `order`, the 2-bit alpha
/// times 0x55.
fn ten_bit(v: u32, order: [u32; 3]) -> [u8; 4] {
    [(v >> order[0]) as u8, (v >> order[1]) as u8, (v >> order[2]) as u8, ((v >> 30) as u8).wrapping_mul(0x55)]
}

/// The file data decoded into a 32-bit image and set on a new raster (resampled when the raster's
/// size differs). `row_residue` is the row step of an A16B16G16R16 file that states its pitch: the
/// original never computes one for it and reads an unwritten local.
fn read_through_image(stream: GPtr<u8>, h: GPtr<u8>, mip: u32, row_residue: u32) -> GPtr<u8> {
    let width = rd::<u32>(h, 0xC);
    let height = rd::<u32>(h, 8);
    let pitch = rd::<u32>(h, 0x10);
    let pf = rd::<u32>(h, 0x4C);
    let fourcc = rd::<u32>(h, 0x50);
    let bits = rd::<u32>(h, 0x54);
    let rmask = rd::<u32>(h, 0x58);
    let mut row = row_residue;
    let size = if pf == 4 {
        if pitch != 0 {
            pitch
        } else if (DXT1..=DXT5).contains(&fourcc) {
            let shift = match fourcc {
                DXT1 => 3,
                DXT2 | DXT3 | DXT4 | DXT5 => 4,
                _ => 0,
            };
            (width >> 2).max(1).wrapping_mul((height >> 2).max(1)) << shift
        } else {
            row = u32::from(table_byte(fourcc, 1) >> 3).wrapping_mul(width);
            row.wrapping_mul(height)
        }
    } else if rd::<u32>(h, 4) & 0x8_0000 != 0 {
        row = row_bytes(bits, rmask, width);
        pitch
    } else {
        row = if pitch != 0 { pitch } else { row_bytes(bits, rmask, width) };
        row.wrapping_mul(height)
    };
    let buf = rw_malloc(size, 0x1_0018);
    if stream_read(stream, buf, size) != size {
        rw_free(buf);
        return GPtr::NULL;
    }
    let img = rw_image_create(width as i32, height as i32, 32);
    if img.is_null() {
        rw_free(buf);
        return GPtr::NULL;
    }
    image_allocate_pixels(img);
    let src = buf.addr() as u32;
    let pixels = p8(rd::<u32>(img, 0x14));
    let convert = |step: u32, f: &dyn Fn(GPtr<u8>) -> [u8; 4]| convert_pixels(img, src, width, height, row, step, f);
    let mut flags = 0u32;
    match pf {
        4 => match fourcc {
            DXT2 | DXT3 => {
                flags = fallback_flags(None);
                decompress_dxt3(width as i32, height as i32, pixels, buf);
            }
            DXT4 | DXT5 => {
                flags = fallback_flags(None);
                decompress_dxt5(width as i32, height as i32, pixels, buf);
            }
            DXT1 => {
                flags = if d3d9_decompress_dxt1(width as i32, height as i32, pixels, buf) != 0 { 0x100 } else { 0x200 };
            }
            FOURCC_A16B16G16R16 => {
                flags = fallback_flags(Some((0x15, 0x500)));
                convert(8, &|s| [rd(s, 1), rd(s, 3), rd(s, 5), rd(s, 7)]);
            }
            _ => {}
        },
        0x40 => {
            if bits == 0x18 {
                flags = if format_ok(0x16) != 0 { 0x600 } else { 0x200 };
                convert(3, &|s| [rd(s, 2), rd(s, 1), rd(s, 0), 0xFF]);
            } else if rmask == 0xFF_0000 {
                flags = if format_ok(0x16) != 0 { 0x600 } else { 0x200 };
                convert(4, &|s| {
                    let v = rd::<u32>(s, 0);
                    [(v >> 16) as u8, (v >> 8) as u8, v as u8, 0xFF]
                });
            } else if rmask == 0xF800 {
                flags = 0x200;
                convert(2, &|s| {
                    let v = rd::<u16>(s, 0);
                    [(v >> 8) as u8 & 0xF8, (v >> 3) as u8 & 0xFC, (v as u8) << 3, 0xFF]
                });
            } else if rmask == 0xE0 {
                flags = 0x200;
                convert(1, &|s| {
                    let b = rd::<u8>(s, 0);
                    [b & 0xE0, (b & 0xFC) << 3, b << 6, 0xFF]
                });
            }
        }
        0x41 => {
            if rmask == 0xFF_0000 {
                flags = fallback_flags(Some((0x15, 0x500)));
                convert(4, &|s| {
                    let v = rd::<u32>(s, 0);
                    [(v >> 16) as u8, (v >> 8) as u8, v as u8, (v >> 24) as u8]
                });
            } else if rmask == 0x7C00 {
                flags = 0x100;
                convert(2, &|s| {
                    let v = rd::<u16>(s, 0);
                    let a = if v & 0x8000 != 0 { 0xFF } else { 0 };
                    [(v >> 7) as u8 & 0xF8, (v >> 2) as u8 & 0xF8, (v as u8) << 3, a]
                });
            } else if rmask == 0xF00 {
                flags = fallback_flags(None);
                convert(2, &|s| {
                    let v = rd::<u16>(s, 0);
                    [(v >> 4) as u8 & 0xF0, v as u8 & 0xF0, (v as u8) << 4, (v >> 8) as u8 & 0xF0]
                });
            } else if rmask == 0x3FF {
                flags = fallback_flags(Some((0x15, 0x500)));
                convert(4, &|s| ten_bit(rd(s, 0), [22, 12, 2]));
            } else if rmask == 0xE0 {
                flags = 0x300;
                convert(2, &|s| {
                    let b = rd::<u8>(s, 0);
                    [b & 0xE0, (b & 0xFC) << 3, b << 6, rd(s, 1)]
                });
            } else if rd::<u32>(h, 0x60) == 0x3FF {
                flags = fallback_flags(Some((0x15, 0x500)));
                convert(4, &|s| ten_bit(rd(s, 0), [2, 12, 22]));
            }
        }
        _ => {}
    }
    rw_free(buf);
    let mip_flags = if mip != 0 { mip | 0x1000 } else { 0 };
    let r = rw_raster_create(width as i32, height as i32, 0, flags | mip_flags | 4);
    if r.is_null() {
        image_destroy(img);
        return GPtr::NULL;
    }
    let mut image = img;
    if rd::<u32>(img, 4) != rd::<u32>(r, 0xC) || rd::<u32>(img, 8) != rd::<u32>(r, 0x10) {
        image = rw_image_create_resample(img, rd(r, 0xC), rd(r, 0x10));
        image_destroy(img);
    }
    raster_set_from_image(r, image);
    image_destroy(image);
    r
}

/// How the header's format is read.
enum Plan {
    /// Straight into this raster's levels.
    Native(GPtr<u8>),
    /// Through a 32-bit image.
    Decode,
    Fail,
}

/// `(width, height)` after the device's size limits for raster flags `flags`.
fn clamped_size(h: GPtr<u8>, flags: u32) -> (u32, u32) {
    let mut w = rd::<i32>(h, 0xC);
    let mut ht = rd::<i32>(h, 8);
    clamp_raster_size(GPtr::from_raw(&mut w), GPtr::from_raw(&mut ht), flags);
    (w as u32, ht as u32)
}

/// A device texture of D3D format `fmt` through `_rwD3D9RasterCreateFromDDSFormat` when the device
/// keeps the file's size, else the image path.
fn plain_texture(h: GPtr<u8>, fmt: u32, mip: u32) -> Plan {
    let (w, ht) = clamped_size(h, mip | 4);
    if rd::<u32>(h, 0xC) != w || rd::<u32>(h, 8) != ht {
        return Plan::Decode;
    }
    let r = raster_create_from_dds_format(w, ht, fmt, mip | 4);
    if r.is_null() {
        Plan::Fail
    } else {
        Plan::Native(r)
    }
}

/// The cube-map levels the file holds: its mipmap count when it has one and the device mipmaps cube
/// maps, else 1.
fn cube_levels(h: GPtr<u8>) -> u32 {
    if crate::common::g::<u32>(DEVICE_CAPS).read() & 0x3_0000 != 0 && rd::<u32>(h, 4) & 0x2_0000 != 0 {
        rd::<u32>(h, 0x18)
    } else {
        1
    }
}

/// An uncompressed cube map of D3D format `fmt` (raster flags `base`), only at the file's own size.
fn cube_texture(h: GPtr<u8>, fmt: u32, base: u32, mip: u32) -> Plan {
    let flags = base | mip;
    let (w, ht) = clamped_size(h, flags | 4);
    if rd::<u32>(h, 0xC) != w || rd::<u32>(h, 8) != ht {
        return Plan::Fail;
    }
    let r = rw_raster_create(w as i32, ht as i32, 0, flags | 0x84);
    if r.is_null() {
        return Plan::Fail;
    }
    wr::<u8>(r, 0x21, rd::<u8>(r, 0x21) & 0x7F);
    let e = ext(r);
    wr::<u8>(e, 9, (rd::<u8>(e, 9) & 0xF1) | 1);
    if dev_create_cube_texture(device(), rd(r, 0xC), cube_levels(h), 0, fmt, 1, e, 0) < 0 {
        return Plan::Fail;
    }
    Plan::Native(r)
}

/// A DXTn / other FourCC texture the device takes as it is, with the raster's format set by hand.
fn fourcc_texture(h: GPtr<u8>, cube: bool, mip: u32) -> Plan {
    let fourcc = rd::<u32>(h, 0x50);
    if cube {
        if cube_format_ok(fourcc) == 0 {
            return Plan::Fail;
        }
    } else if format_ok(fourcc) == 0 {
        return Plan::Decode;
    }
    let r = rw_raster_create(rd(h, 0xC), rd(h, 8), 0, mip | 0x84);
    if r.is_null() {
        return Plan::Fail;
    }
    wr::<u8>(r, 0x21, rd::<u8>(r, 0x21) & 0x7F);
    let e = ext(r);
    wr::<u32>(e, 0x18, fourcc);
    let compressed = if (DXT1 as i32..=DXT5 as i32).contains(&(fourcc as i32)) { 0x10 } else { 0 };
    wr::<u8>(e, 0xA, (rd::<u8>(e, 0xA) & 0xF) | compressed);
    let hr = if cube {
        wr::<u8>(e, 9, (rd::<u8>(e, 9) & 0xF1) | 1);
        dev_create_cube_texture(device(), rd(r, 0xC), cube_levels(h), 0, rd(e, 0x18), 1, e, 0)
    } else {
        let levels = if rd::<u32>(h, 4) & 0x2_0000 != 0 { rd::<u32>(h, 0x18) } else { 1 };
        dev_create_texture(device(), rd(r, 0xC), rd(r, 0x10), levels, 0, rd(e, 0x18), 1, e, 0)
    };
    if hr < 0 {
        Plan::Fail
    } else {
        Plan::Native(r)
    }
}

/// The D3D format for an uncompressed pixel format, or how to read it otherwise.
fn plan(h: GPtr<u8>, cube: bool, mip: u32) -> Plan {
    let pf = rd::<u32>(h, 0x4C);
    let bits = rd::<u32>(h, 0x54);
    let rmask = rd::<u32>(h, 0x58);
    let bmask = rd::<u32>(h, 0x60);
    let amask = rd::<u32>(h, 0x64);
    // A format the device lacks: no cube map, else decoded.
    let checked = |fmt: u32| {
        if format_ok(fmt) != 0 {
            plain_texture(h, fmt, mip)
        } else {
            Plan::Decode
        }
    };
    let cube_checked = |fmt: u32, base: u32| {
        if cube_format_ok(fmt) != 0 {
            cube_texture(h, fmt, base, mip)
        } else {
            Plan::Fail
        }
    };
    // A format the device lacks fails.
    let required = |fmt: u32| {
        if format_ok(fmt) != 0 {
            plain_texture(h, fmt, mip)
        } else {
            Plan::Fail
        }
    };
    match pf {
        4 => fourcc_texture(h, cube, mip),
        0x40 if cube => match (bits, rmask) {
            (0x18, _) => cube_checked(0x14, 0x600),
            (_, 0xFF_0000) => cube_checked(0x16, 0x600),
            (_, 0xF800) => cube_checked(0x17, 0x200),
            _ => Plan::Fail,
        },
        0x40 => match (bits, rmask) {
            (0x18, _) if cube_format_ok(0x14) != 0 => plain_texture(h, 0x14, mip),
            (0x18, _) => Plan::Decode,
            (_, 0xFF_0000) => checked(0x16),
            (_, 0xF800) => plain_texture(h, 0x17, mip),
            (_, 0xE0) => checked(0x1B),
            _ => Plan::Fail,
        },
        0x41 if cube => match rmask {
            0xFF_0000 => cube_checked(0x16, 0x600),
            0x7C00 => cube_checked(0x19, 0x100),
            0xF00 => cube_checked(0x1A, 0x300),
            _ => Plan::Fail,
        },
        0x41 => match rmask {
            0xFF_0000 => checked(0x15),
            0x7C00 => plain_texture(h, 0x19, mip),
            0xF00 => checked(0x1A),
            0x3FF => checked(0x23),
            _ if bmask == 0x3FF => checked(0x1F),
            0xE0 => checked(0x1D),
            _ => Plan::Fail,
        },
        _ if cube => Plan::Fail,
        _ => match (bits, rmask) {
            (0x10, 0xFFFF) => required(0x51),
            (0x10, 0xFF) if amask == 0xFF00 => required(0x33),
            (8, 0xFF) => required(0x32),
            (8, _) if amask == 0xFF => required(0x1C),
            _ => Plan::Fail,
        },
    }
}

/// The face a cube map's next levels belong to, from the remaining caps2 face bits (each test also
/// matches the cube-map bit, which the first face clears), with those bits cleared; `None` when no
/// face is left.
fn next_face(caps2: u32) -> Option<(u8, u32)> {
    let ah = (caps2 >> 8) as u8;
    let face = (0u8..6).find(|&f| ah & (2 | 4 << f) != 0)?;
    let rest = if face == 5 { 0 } else { caps2 & !(u32::from(2u8 | 4 << face) << 8) };
    Some((face, rest))
}

/// Skips a level the raster does not have: its bytes from the raster's size halved `level` times.
fn skip_level(stream: GPtr<u8>, r: GPtr<u8>, level: i32) {
    let e = ext(r);
    let w = ((rd::<i32>(r, 0xC) >> (level & 31)) as u32).max(1);
    let ht = ((rd::<i32>(r, 0x10) >> (level & 31)) as u32).max(1);
    let bytes = if rd::<u8>(e, 0xA) & 0xF0 == 0 {
        ((rd::<i32>(r, 0x14) >> 3) as u32).wrapping_mul(ht).wrapping_mul(w)
    } else {
        let shift = 3 + u32::from(rd::<u32>(e, 0x18) != DXT1);
        ((ht >> 2).max(1).wrapping_mul((w >> 2).max(1))) << shift
    };
    rw_stream_skip(stream, bytes);
}

/// Whether a DXT1 raster uses the transparent colour anywhere: a three-colour block (c0 <= c1) with
/// a pixel of index 3. Rasters narrower / lower than 4 pixels check only their own columns / rows.
fn dxt1_has_alpha(r: GPtr<u8>) -> bool {
    let levels = rw_raster_get_num_levels(r);
    for level in 0..levels {
        let mut p = rw_raster_lock(r, level as u32, 2);
        let w = rd::<i32>(r, 0xC);
        let (mut blocks, cols) = if w < 4 { (1i32, w) } else { (w / 4, 4) };
        let ht = rd::<i32>(r, 0x10);
        let rows = if ht < 4 {
            ht
        } else {
            blocks = (ht / 4).wrapping_mul(blocks);
            4
        };
        loop {
            let b = p8(p);
            if rd::<u16>(b, 0) <= rd::<u16>(b, 2) {
                for row in 0..rows {
                    let mut idx = rd::<u8>(b, 4 + row as isize);
                    for _ in 0..cols {
                        if idx & 3 == 3 {
                            rw_raster_unlock(r);
                            return true;
                        }
                        idx >>= 2;
                    }
                }
            }
            p = p.wrapping_add(8);
            blocks = blocks.wrapping_sub(1);
            if blocks == 0 {
                break;
            }
        }
        rw_raster_unlock(r);
    }
    false
}

/// `RwD3D9RasterStreamReadDDS(stream)` (0x81F360): reads the magic and header, then either creates
/// the device texture (2D or, with the complex + cube caps, a cube map; the format from the FourCC or
/// the masks) and reads every level the file holds into it (per cube face in caps2 order; levels past
/// the raster's own are skipped), or decodes the data into a 32-bit image. Finally the alpha flag and
/// the cFormat bits: from the alpha bits / mask, for DXT1 by scanning for transparent pixels, else
/// from the format table. NULL on any failure (rasters created by then are not freed, a level read
/// that comes up short leaves its raster locked).
#[hook(addr = 0x81F360, conv = cdecl, name = "RwD3D9RasterStreamReadDDS", expect = "81 EC E0 00 00 00 8D 84 24 BC 00 00 00",
       peek = 0xD8, unverified = "frontier2-c35", route = "L2", hazards = "heap-writes,stack-residue",
       note = "an A16B16G16R16 file with a pitch reads its row step from an unwritten local (stack residue); the surface description too when GetLevelDesc fails")]
pub fn raster_stream_read_dds(stream: GPtr<u8>) -> GPtr<u8> {
    let residue = |off: i32| sa_core::abi::peek_stack(0x81F360, off).unwrap_or(0);
    let mut magic = 0u32;
    if stream_read(stream, GPtr::from_raw(&mut magic).cast(), 4) != 4 || magic != DDS_MAGIC {
        return GPtr::NULL;
    }
    let mut header = [0u8; 0x7C];
    let h: GPtr<u8> = GPtr::from_raw(&mut header).cast();
    if stream_read(stream, h, 0x7C) != 0x7C {
        return GPtr::NULL;
    }
    let flags = rd::<u32>(h, 4);
    if flags & 0x1007 == 0 {
        return GPtr::NULL;
    }
    let mip = (flags >> 2) & 0x8000;
    let cube = rd::<u8>(h, 0x68) & 8 != 0 && rd::<u32>(h, 0x6C) & 0xFE00 != 0;
    let r = match plan(h, cube, mip) {
        Plan::Native(r) => r,
        Plan::Decode => return read_through_image(stream, h, mip, residue(-0xD8)),
        Plan::Fail => return GPtr::NULL,
    };
    let e = ext(r);
    let have = rw_raster_get_num_levels(r);
    let count = if mip == 0 { 1 } else { rd::<i32>(h, 0x18) };
    let pf = rd::<u32>(h, 0x4C);
    let fourcc = rd::<u32>(h, 0x50);
    let mut desc: [u32; 8] = core::array::from_fn(|i| residue(-0x20 + 4 * i as i32));
    let mut caps2 = rd::<u32>(h, 0x6C);
    loop {
        if cube {
            let Some((face, rest)) = next_face(caps2) else { break };
            wr::<u8>(e, 9, (rd::<u8>(e, 9) & 0xF) | face << 4);
            caps2 = rest;
        }
        for level in 0..count {
            if level >= have {
                skip_level(stream, r, level);
                continue;
            }
            let pixels = rw_raster_lock(r, level as u32, 1);
            tex_get_level_desc(p8(rd::<u32>(e, 0)), level as u32, GPtr::from_raw(&mut desc).cast());
            let rows = if pf == 4 && is_dxt(fourcc) { desc[7] >> 2 } else { desc[7] };
            let bytes = rows.wrapping_mul(rd::<u32>(e, 0x10));
            if stream_read(stream, p8(pixels), bytes) != bytes {
                return GPtr::NULL;
            }
            rw_raster_unlock(r);
        }
        if !cube || caps2 == 0 {
            break;
        }
    }
    if pf != 4 {
        wr::<u8>(e, 8, u8::from(pf & 1 != 0 || rd::<u32>(h, 0x64) != 0));
        return r;
    }
    let fmt = rd::<u32>(e, 0x18);
    if fmt == DXT1 {
        let alpha = dxt1_has_alpha(r);
        wr::<u8>(e, 8, u8::from(alpha));
        if alpha {
            wr::<u32>(r, 0x14, 0x10);
            wr::<u8>(r, 0x23, (rd::<u8>(r, 0x23) & 0xF1) | 1);
        } else {
            wr::<u8>(r, 0x23, (rd::<u8>(r, 0x23) & 0xF2) | 2);
        }
    } else if is_dxt(fmt) {
        wr::<u8>(e, 8, 1);
        wr::<u32>(r, 0x14, 0x10);
        wr::<u8>(r, 0x23, (rd::<u8>(r, 0x23) & 0xF3) | 3);
    } else {
        wr::<u8>(e, 8, table_byte(fmt, 0));
        wr::<u32>(r, 0x14, u32::from(table_byte(fmt, 1)));
        wr::<u8>(r, 0x23, (rd::<u8>(r, 0x23) & 0xF0) | table_byte(fmt, 3));
    }
    r
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_D3D9_DECOMPRESS_DXT3,
    HOOK_D3D9_DECOMPRESS_DXT5,
    HOOK_D3D9_DECOMPRESS_DXT5_ALPHA,
    HOOK_RASTER_STREAM_READ_DDS,
];
