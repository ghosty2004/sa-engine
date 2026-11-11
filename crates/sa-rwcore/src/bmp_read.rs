//! RenderWare toolkit `RtBMP`: `RtBMPImageRead` and its row decoder (STD-6, batch `std6-rw-1`).
//! Ported from the 1.0 US bytes (0x7CDF60, 0x7CE5D0).
//!
//! `RwImage` fields used: +4 width, +8 height, +0xC depth, +0x10 stride, +0x14 cpPixels, +0x18 palette.
//! The toolkit's errors carry plugin id 0x19D (`rwID_BMPIMAGEPLUGIN`).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{RwError, p8, rd, rw_error_set, rw_free, rw_malloc};

orig! {
    /// `RwStreamOpen(type, access, data)` (0x7ECEF0)
    fn stream_open(ty: u32, access: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7ECEF0 => crate::stream::rw_stream_open;
    /// `RwStreamRead(stream, buf, len)` (0x7EC9D0)
    fn stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => crate::stream::rw_stream_read;
    /// `RwStreamClose(stream, data)` (0x7ECE20)
    fn stream_close(s: GPtr<u8>, data: GPtr<u8>) -> u32 = cdecl 0x7ECE20 => crate::stream::rw_stream_close;
    /// `RwStreamSkip(stream, offset)` (0x7ECD00)
    fn stream_skip(s: GPtr<u8>, offset: u32) -> GPtr<u8> = cdecl 0x7ECD00 => crate::stream::rw_stream_skip;
    /// `RwImageCreate(w, h, depth)` (0x8026E0)
    fn image_create(w: i32, h: i32, depth: i32) -> GPtr<u8> = cdecl 0x8026E0 => crate::image::rw_image_create;
    /// `RwImageAllocatePixels(image)` (0x8027A0)
    fn image_allocate_pixels(img: GPtr<u8>) -> GPtr<u8> = cdecl 0x8027A0 => crate::image::rw_image_allocate_pixels;
    /// `RwImageDestroy(image)` (0x802740)
    fn image_destroy(img: GPtr<u8>) -> u32 = cdecl 0x802740 => crate::image::rw_image_destroy;
    /// `_rwerror(code)` (0x8088D0, one argument)
    fn rw_error1(code: u32) -> u32 = cdecl 0x8088D0 => unported;
    /// `_rwerror(code, a)` (0x8088D0, two arguments)
    fn rw_error2(code: u32, a: u32) -> u32 = cdecl 0x8088D0 => unported;
    /// `_rtBMPDecodeRow(image, y, src, bpp, rle)` (0x7CE5D0)
    fn decode_row_call(img: GPtr<u8>, y: i32, src: GPtr<u8>, bpp: i32, rle: i32) -> u32 = cdecl 0x7CE5D0 => bmp_decode_row;
}

/// The toolkit's plugin id in its `RwError`s.
const BMP_PLUGIN: i32 = 0x19D;

fn bmp_error(code: u32) {
    let mut e = RwError {
        plugin_id: BMP_PLUGIN,
        error_code: 0,
    };
    e.error_code = rw_error1(code) as i32;
    rw_error_set(GPtr::from_raw(&mut e));
}

fn b(p: GPtr<u8>, i: u32) -> u8 {
    p.byte_add(i as isize).read()
}

fn wb(p: GPtr<u8>, i: u32, v: u8) {
    p.byte_add(i as isize).write(v)
}

fn le16(buf: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([buf[o], buf[o + 1]])
}

fn le32(buf: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([buf[o], buf[o + 1], buf[o + 2], buf[o + 3]])
}

/// The original's 0x400-byte stack buffer (header tail and palette).
const LOCAL: usize = 0x400;

fn read_into(s: GPtr<u8>, buf: &mut [u8; LOCAL], off: usize, len: u32) -> u32 {
    // the original reads into its stack buffer without a bound; a header or palette that does not
    // fit (only malformed files: biSize > 0x404 or a negative bit count) is cut at the buffer end
    let room = (LOCAL - off) as u32;
    let n = len.min(room);
    let got = stream_read(s, GPtr::from_raw(buf[off..].as_mut_ptr()), n);
    if n < len && got == n {
        // the bytes past the buffer are consumed as the original consumes them
        let mut rest = len - n;
        let mut scratch = [0u8; 0x100];
        let mut total = got;
        while rest != 0 {
            let k = rest.min(0x100);
            let r = stream_read(s, GPtr::from_raw(scratch.as_mut_ptr()), k);
            total += r;
            if r != k {
                break;
            }
            rest -= k;
        }
        return total;
    }
    got
}

/// `RtBMPImageRead(filename)` (0x7CDF60): reads a Windows / OS/2 BMP file into a new `RwImage`.
/// Header: `BM`, `bfOffBits`, `biSize` (0xC: core header with 16-bit width / height, no compression;
/// otherwise width, height, bit count, compression, `biClrUsed`). Compression 2 (RLE4) is an error
/// (0x80000009). Image depth 4 for 1 / 4 bpp, 8 for 8 bpp, else 32. For bit counts below 9 the palette
/// (`biClrUsed` entries when in 1..=2^bpp, else 2^bpp; BGR triples for the core header, BGRx quads for
/// a 0x28 header, any other header size with a palette is an error that leaks the image) is stored as
/// RGBA with alpha 0xFF. Then the stream skips to `bfOffBits`, and rows are read bottom-up into a
/// `RwMalloc` buffer of `((w + 7) & ~7) * ((bpp + 7) / 8)` bytes (hint 0x10000): uncompressed (or 24 /
/// 32 bpp) rows of `((bpp * w + 7) / 8 + 3) & ~3` bytes, or RLE8 (escape 0 = end of line, 1 = end of
/// bitmap, 2 (delta) = error, >= 3 = absolute run padded to a word). Each row goes through
/// `_rtBMPDecodeRow`. Every failure closes the stream and returns NULL (after destroying the image
/// where the original does).
#[hook(addr = 0x7CDF60, conv = cdecl, name = "RtBMPImageRead", expect = "81 EC 54 04 00 00",
       unverified = "std6-rw-1", route = "L2", hazards = "heap-writes", recs = "new-game",
       note = "only caller on the boot path: CPlayerSkin::GetSkinTexture (the player skin BMP)")]
pub fn rt_bmp_image_read(name: GPtr<u8>) -> GPtr<u8> {
    let s = stream_open(2, 1, name);
    if s.is_null() {
        return GPtr::NULL;
    }
    let fail = |s: GPtr<u8>| -> GPtr<u8> {
        stream_close(s, GPtr::NULL);
        GPtr::NULL
    };
    let mut magic = [0u8; 4];
    if stream_read(s, GPtr::from_raw(magic.as_mut_ptr()), 4) != 4 {
        return fail(s);
    }
    if u16::from_le_bytes([magic[0], magic[1]]) != 0x4D42 {
        return fail(s);
    }
    // local_400 + 2 .. + 0x10: the size's high half, reserved, bfOffBits, biSize
    let mut buf = [0u8; LOCAL];
    if stream_read(s, GPtr::from_raw(buf[2..].as_mut_ptr()), 0xE) != 0xE {
        return fail(s);
    }
    let bi_size = le32(&buf, 0xC);
    let off_bits = le32(&buf, 8);
    let want = bi_size.wrapping_sub(4) as u32;
    if read_into(s, &mut buf, 0, want) != want {
        return fail(s);
    }
    let mut consumed = bi_size.wrapping_add(0xE);
    let (width, height, bpp, compression, ncolors);
    if bi_size == 0xC {
        width = le16(&buf, 0) as i32;
        height = le16(&buf, 2) as i32;
        bpp = le16(&buf, 6);
        compression = 0;
        ncolors = 1i32.wrapping_shl(bpp as u32 & 0x1F);
    } else {
        width = le32(&buf, 0);
        height = le32(&buf, 4);
        bpp = le16(&buf, 0xA);
        compression = le32(&buf, 0xC);
        if compression == 2 {
            bmp_error(0x8000_0009);
            return fail(s);
        }
        let used = le32(&buf, 0x1C);
        let max = 1i32.wrapping_shl(le32(&buf, 8) as u32 >> 16 & 0x1F);
        ncolors = if used < 1 || max < used { 1i32.wrapping_shl(bpp as u32 & 0x1F) } else { used };
    }
    let bits = bpp as i32;
    let depth = match bits {
        1 | 4 => 4,
        8 => 8,
        _ => 0x20,
    };
    let img = image_create(width, height, depth);
    if img.is_null() {
        return fail(s);
    }
    let destroy_fail = |s: GPtr<u8>| -> GPtr<u8> {
        image_destroy(img);
        stream_close(s, GPtr::NULL);
        GPtr::NULL
    };
    if image_allocate_pixels(img).is_null() {
        return destroy_fail(s);
    }
    if bpp < 9 {
        let pal = p8(rd::<u32>(img, 0x18));
        if bi_size == 0xC {
            let n = ncolors.wrapping_mul(3);
            if read_into(s, &mut buf, 0, n as u32) != n as u32 {
                return fail(s);
            }
            consumed = n.wrapping_add(0x1A);
            for k in 0..ncolors.max(0) as usize {
                let i = (k * 3).min(LOCAL - 3);
                let o = (k * 4) as u32;
                wb(pal, o, buf[i + 2]);
                wb(pal, o + 1, buf[i + 1]);
                wb(pal, o + 2, buf[i]);
                wb(pal, o + 3, 0xFF);
            }
        } else {
            if bi_size != 0x28 {
                // the original leaks the image here
                bmp_error(0x8000_0009);
                return fail(s);
            }
            let n = ncolors.wrapping_mul(4);
            if read_into(s, &mut buf, 0, n as u32) != n as u32 {
                return fail(s);
            }
            consumed = n.wrapping_add(0x36);
            for k in 0..ncolors.max(0) as usize {
                let i = (k * 4).min(LOCAL - 4);
                let o = (k * 4) as u32;
                wb(pal, o, buf[i + 2]);
                wb(pal, o + 1, buf[i + 1]);
                wb(pal, o + 2, buf[i]);
                wb(pal, o + 3, 0xFF);
            }
        }
    }
    if stream_skip(s, off_bits.wrapping_sub(consumed) as u32).is_null() {
        return destroy_fail(s);
    }
    let row_bytes = ((bits.wrapping_mul(width).wrapping_add(7) / 8).wrapping_add(3) as u32) & !3;
    let size = ((width.wrapping_add(7) as u32) & !7).wrapping_mul(((bits + 7) / 8) as u32);
    let row = rw_malloc(size, 0x10000);
    if row.is_null() {
        let mut e = RwError {
            plugin_id: BMP_PLUGIN,
            error_code: 0,
        };
        e.error_code = rw_error2(0x8000_0013, size) as i32;
        rw_error_set(GPtr::from_raw(&mut e));
        return destroy_fail(s);
    }
    let free_fail = |s: GPtr<u8>| -> GPtr<u8> {
        rw_free(row);
        destroy_fail(s)
    };
    if compression == 0 || bits == 0x18 || bits == 0x20 {
        let mut y = height;
        for _ in 0..height.max(0) {
            if stream_read(s, row, row_bytes) != row_bytes {
                return free_fail(s);
            }
            decode_row_call(img, y - 1, row, bits, 0);
            y -= 1;
        }
    } else {
        if compression != 1 {
            bmp_error(0x8000_0009);
            return free_fail(s);
        }
        let mut line = 0i32;
        while line < height {
            let mut x = 0u32;
            loop {
                let mut pair = [0u8; 2];
                if stream_read(s, GPtr::from_raw(pair.as_mut_ptr()), 2) != 2 {
                    return free_fail(s);
                }
                let (count, value) = (pair[0], pair[1]);
                if count != 0 {
                    for i in 0..count as u32 {
                        wb(row, x + i, value);
                    }
                    x += count as u32;
                    continue;
                }
                if value >= 3 {
                    let n = value as u32;
                    if stream_read(s, row.byte_add(x as isize), n) != n {
                        return free_fail(s);
                    }
                    x += n;
                    if value & 1 != 0 {
                        let mut pad = [0u8; 1];
                        if stream_read(s, GPtr::from_raw(pad.as_mut_ptr()), 1) != 1 {
                            return free_fail(s);
                        }
                    }
                    continue;
                }
                if value == 1 {
                    line = height - 1;
                } else if value != 0 {
                    return free_fail(s);
                }
                break;
            }
            decode_row_call(img, height - line - 1, row, bits, 1);
            line += 1;
        }
    }
    rw_free(row);
    stream_close(s, GPtr::NULL);
    img
}

/// `_rtBMPDecodeRow(image, y, src, bpp, rle)` (0x7CE5D0): converts one BMP row into image row `y`
/// by `depth | bpp << 8`: 8/8 copy; 1 bpp into 4/8 (one index byte per pixel, MSB first) or 32
/// (palette); 4 bpp into 4/8 (high nibble first) or 32 (palette); 8 into 32 (palette); 15 / 16 bpp
/// (5-5-5 / 5-6-5, top bits kept) into 32 RGBA; 24 into 32 (BGR swapped to RGB without `rle`, copied
/// as is with it); 32 into 32 (first three bytes copied, alpha 0xFF). Anything else raises
/// 0x80000008 and returns 0; otherwise 1.
#[hook(addr = 0x7CE5D0, conv = cdecl, name = "FUN_007ce5d0", expect = "8B 54 24 04 83 EC 08",
       unverified = "std6-rw-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn bmp_decode_row(img: GPtr<u8>, y: i32, src: GPtr<u8>, bpp: i32, rle: i32) -> u32 {
    let pal = p8(rd::<u32>(img, 0x18));
    let w = rd::<u32>(img, 4);
    let key = rd::<u32>(img, 0xC) | (bpp as u32).wrapping_shl(8);
    let dst = p8(rd::<i32>(img, 0x10).wrapping_mul(y).wrapping_add(rd::<i32>(img, 0x14)) as u32);
    let pal_word = |i: u32| -> u32 { pal.byte_add((i * 4) as isize).cast::<u32>().read() };
    let put32 = |k: u32, v: u32| dst.byte_add((k * 4) as isize).cast::<u32>().write(v);
    match key {
        0x808 => {
            for k in 0..w {
                wb(dst, k, b(src, k));
            }
        }
        0x104 | 0x108 => {
            for k in 0..w {
                wb(dst, k, b(src, k >> 3) >> (7 - (k & 7)) & 1);
            }
        }
        0x120 => {
            for k in 0..w {
                put32(k, pal_word((b(src, k >> 3) >> (7 - (k & 7)) & 1) as u32));
            }
        }
        0x404 | 0x408 => {
            for k in 0..w {
                wb(dst, k, b(src, k >> 1) >> (if k & 1 == 0 { 4 } else { 0 }) & 0xF);
            }
        }
        0x420 => {
            for k in 0..w {
                put32(k, pal_word((b(src, k >> 1) >> (if k & 1 == 0 { 4 } else { 0 }) & 0xF) as u32));
            }
        }
        0x820 => {
            for k in 0..w {
                put32(k, pal_word(b(src, k) as u32));
            }
        }
        0xF20 | 0x1020 => {
            for k in 0..w {
                let v = src.byte_add((k * 2) as isize).cast::<u16>().read() as u32;
                let o = k * 4;
                if key == 0xF20 {
                    wb(dst, o, (v >> 7) as u8 & 0xF8);
                    wb(dst, o + 1, (v >> 2) as u8 & 0xF8);
                } else {
                    wb(dst, o, (v >> 8) as u8 & 0xF8);
                    wb(dst, o + 1, (v >> 3) as u8 & 0xFC);
                }
                wb(dst, o + 2, (v << 3) as u8);
                wb(dst, o + 3, 0xFF);
            }
        }
        0x1820 | 0x2020 => {
            let step = if key == 0x1820 { 3 } else { 4 };
            let swap = key == 0x1820 && rle == 0;
            for k in 0..w {
                let i = k * step;
                let o = k * 4;
                let (c0, c1, c2) = (b(src, i), b(src, i + 1), b(src, i + 2));
                if swap {
                    wb(dst, o, c2);
                    wb(dst, o + 1, c1);
                    wb(dst, o + 2, c0);
                } else {
                    wb(dst, o, c0);
                    wb(dst, o + 1, c1);
                    wb(dst, o + 2, c2);
                }
                wb(dst, o + 3, 0xFF);
            }
        }
        _ => {
            bmp_error(0x8000_0008);
            return 0;
        }
    }
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_RT_BMP_IMAGE_READ, HOOK_BMP_DECODE_ROW];

/// No globals.
pub const GLOBALS: &[GlobalDesc] = &[];
