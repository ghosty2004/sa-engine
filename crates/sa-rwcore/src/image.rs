//! PORT-RW-4 module `image`: RwImage and the D3D9 image/raster conversions (see docs/arch/rw-port.md, PORT-RW-4).
//! Ported from the 1.0 US bytes; the specs under `docs/re/specs/RW/` are reading aids.
//!
//! IMAGEMODULE `M = G + [0xC9A65C]`: +0 image freelist, +0xC gamma table (256 bytes), +0x20C gamma.
//! Image registry 0x8E2670.
//!
//! `RwImage` (0x1C + plugins): +0 flags (bit 0 = owns pixels, bit 1 = gamma corrected), +4 width,
//! +8 height, +0xC depth, +0x10 stride, +0x14 cpPixels, +0x18 palette.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{
    RwError, error1, module, p8, rd, rw_error_set, rw_free, rw_malloc, rw_memory_alloc, rw_memory_free, wr,
};

/// IMAGEMODULE offset global.
pub const IMAGE_MODULE: usize = 0xC9A65C;
/// Image plugin registry.
pub const IMAGE_REGISTRY: u32 = 0x8E2670;

orig! {
    /// `_rwPluginRegistryInitObject(reg, obj)` (0x8086E0)
    fn plugin_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => crate::plugin::registry_init_object;
    /// `_rwPluginRegistryDeInitObject(reg, obj)` (0x808740)
    fn plugin_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => crate::plugin::registry_deinit_object;
    /// `_rwerror(code, a)` (0x8088D0, two arguments)
    fn rw_error2(code: u32, a: u32) -> u32 = cdecl 0x8088D0 => unported;
    /// `RwImageRead(name)` (0x802FD0)
    fn rw_image_read(name: GPtr<u8>) -> GPtr<u8> = cdecl 0x802FD0 => unported;
    /// `RwImageReadMaskedImage(name, mask)` (0x8035C0)
    fn rw_image_read_masked_image(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl 0x8035C0 => unported;
    /// `RwImageDestroy(image)` (0x802740)
    fn rw_image_destroy_call(img: GPtr<u8>) -> u32 = cdecl 0x802740 => rw_image_destroy;
    /// `RwImageCreateResample(image, w, h)` (0x80CD10)
    fn rw_image_create_resample(img: GPtr<u8>, w: i32, h: i32) -> GPtr<u8> = cdecl 0x80CD10 => rw_image_create_resample_port;
    /// `RwRasterCreate(w, h, depth, flags)` (0x7FB230)
    fn rw_raster_create_call(w: i32, h: i32, depth: i32, flags: u32) -> GPtr<u8> = cdecl 0x7FB230 => crate::raster::rw_raster_create;
    /// `_rwD3D9RGBToPixel(out, rgba, format)` (0x7FEE20), the recursive DEFAULT case
    fn rgb_to_pixel_call(out: GPtr<u32>, rgba: GPtr<u8>, format: u32) -> u32 = cdecl 0x7FEE20 => d3d9_rgb_to_pixel;
    /// `_rwD3D9ClampRasterSize(&w, &h, flags)` (0x7FED70)
    fn clamp_raster_size(w: GPtr<u8>, h: GPtr<u8>, flags: u32) -> u32 = cdecl 0x7FED70 => unported;
    /// `_rwD3D9ImageFindFormat(image)` (0x7FFFF0)
    fn image_find_format(img: GPtr<u8>) -> u32 = cdecl 0x7FFFF0 => unported;
    /// `FUN_004cc580(raster, format)`: sets type/flags/format and depth of the raster (0x4CC580)
    fn raster_set_format(r: GPtr<u8>, format: u32) -> u32 = cdecl 0x4CC580 => unported;

    /// `stdFunc[6]` IMAGEGETRASTER (G+0x60)
    fn std_image_get_raster(img: GPtr<u8>, r: GPtr<u8>, n: u32) -> u32 = cdecl table 0xC97B24 0x60 => unported;
    /// `stdFunc[7]` RASTERSETIMAGE (G+0x64)
    fn std_raster_set_image(r: GPtr<u8>, img: GPtr<u8>, n: u32) -> u32 = cdecl table 0xC97B24 0x64 => unported;
    /// `stdFunc[9]` IMAGEFINDRASTERFORMAT (G+0x6C)
    fn std_image_find_raster_format(r: GPtr<u8>, img: GPtr<u8>, ty: u32) -> u32 = cdecl table 0xC97B24 0x6C => unported;
}

fn a32<T: sa_core::mem::GameData>(p: GPtr<T>) -> u32 {
    p.addr() as u32
}

/// `M = G + [0xC9A65C]`.
fn imod() -> GPtr<u8> {
    module(IMAGE_MODULE)
}

fn image_free_list() -> GPtr<u8> {
    p8(rd::<u32>(imod(), 0))
}

/// `RwImageCreate(width, height, depth)` (0x8026E0): freelist alloc (hint 0x30018); width, height,
/// depth, cpPixels = palette = flags = 0, plugin constructors.
#[hook(addr = 0x8026E0, conv = cdecl, name = "RwImageCreate", expect = "A1 24 7B C9 00")]
pub fn rw_image_create(w: i32, h: i32, depth: i32) -> GPtr<u8> {
    let img = rw_memory_alloc(image_free_list(), 0x30018);
    if img.is_null() {
        return GPtr::NULL;
    }
    wr::<i32>(img, 4, w);
    wr::<i32>(img, 8, h);
    wr::<i32>(img, 0xC, depth);
    wr::<u32>(img, 0x14, 0);
    wr::<u32>(img, 0x18, 0);
    wr::<u32>(img, 0, 0);
    plugin_init_object(p8(IMAGE_REGISTRY), img);
    img
}

/// `RwImageDestroy(image)` (0x802740): frees owned pixels (flags bit 0), plugin destructors, freelist
/// free; returns 1.
#[hook(addr = 0x802740, conv = cdecl, name = "RwImageDestroy", expect = "56 8B 74 24 08")]
pub fn rw_image_destroy(img: GPtr<u8>) -> u32 {
    if rd::<u8>(img, 0) & 1 != 0 {
        rw_free(p8(rd::<u32>(img, 0x14)));
        wr::<u32>(img, 0x14, 0);
        wr::<u32>(img, 0x18, 0);
        wr::<u32>(img, 0, rd::<u32>(img, 0) & !1);
    }
    plugin_deinit_object(p8(IMAGE_REGISTRY), img);
    rw_memory_free(image_free_list(), img);
    1
}

/// `RwImageAllocatePixels(image)` (0x8027A0): stride = `(((depth + 7) >> 3) * width + 3) & ~3`;
/// one block of `stride * height` (+ `4 << depth` palette bytes for depth 4/8), hint 0x30018. Failure:
/// error 0x80000013 (size), NULL. Sets the palette pointer (after the pixels, or NULL) and flags bit 0.
#[hook(addr = 0x8027A0, conv = cdecl, name = "RwImageAllocatePixels", expect = "83 EC 08 53 55")]
pub fn rw_image_allocate_pixels(img: GPtr<u8>) -> GPtr<u8> {
    let d = rd::<i32>(img, 0xC);
    let (has_pal, pal_size) = if d == 4 || d == 8 {
        (true, (1u32 << (d as u32 & 31)) << 2)
    } else {
        (false, 0)
    };
    let stride = ((((d.wrapping_add(7)) >> 3).wrapping_mul(rd::<i32>(img, 4))).wrapping_add(3) as u32) & !3;
    let pix = (rd::<i32>(img, 8) as u32).wrapping_mul(stride);
    wr::<u32>(img, 0x10, stride);
    let size = pix.wrapping_add(pal_size);
    let p = rw_malloc(size, 0x30018);
    wr::<u32>(img, 0x14, a32(p));
    if p.is_null() {
        let mut e = RwError {
            plugin_id: 1,
            error_code: 0,
        };
        e.error_code = rw_error2(0x8000_0013, size) as i32;
        rw_error_set(GPtr::from_raw(&mut e));
        return GPtr::NULL;
    }
    let pal = if has_pal { a32(p).wrapping_add(pix) } else { 0 };
    wr::<u32>(img, 0x18, pal);
    wr::<u32>(img, 0, rd::<u32>(img, 0) | 1);
    img
}

/// `RwImageFreePixels(image)` (0x802860): frees the pixels unconditionally, clears pixels/palette and
/// flags bit 0.
#[hook(addr = 0x802860, conv = cdecl, name = "RwImageFreePixels", expect = "8B 0D 24 7B C9 00")]
pub fn rw_image_free_pixels(img: GPtr<u8>) -> GPtr<u8> {
    rw_free(p8(rd::<u32>(img, 0x14)));
    wr::<u32>(img, 0x14, 0);
    wr::<u32>(img, 0x18, 0);
    wr::<u32>(img, 0, rd::<u32>(img, 0) & !1);
    img
}

/// alpha = max(r, g, b) of one RGBA entry.
fn mask_entry(p: GPtr<u8>) {
    let mut m = rd::<u8>(p, 0);
    let g1 = rd::<u8>(p, 1);
    if g1 > m {
        m = g1;
    }
    let b = rd::<u8>(p, 2);
    if b > m {
        m = b;
    }
    wr::<u8>(p, 3, m);
}

/// `RwImageMakeMask(image)` (0x802A20): alpha = max(r, g, b) for every palette entry (depth 4/8) or
/// every pixel (depth 32); other depths unchanged.
#[hook(addr = 0x802A20, conv = cdecl, name = "RwImageMakeMask", expect = "55 8B 6C 24 08")]
pub fn rw_image_make_mask(img: GPtr<u8>) -> GPtr<u8> {
    let d = rd::<i32>(img, 0xC);
    if d == 4 || d == 8 {
        let n = 1i32 << (d as u32 & 31);
        let pal = p8(rd::<u32>(img, 0x18));
        for i in 0..n as isize {
            mask_entry(pal.byte_add(i * 4));
        }
    } else if d == 32 {
        let mut row = p8(rd::<u32>(img, 0x14));
        let mut y = 0i32;
        while y < rd::<i32>(img, 8) {
            let mut x = 0i32;
            while x < rd::<i32>(img, 4) {
                mask_entry(row.byte_add(x as isize * 4));
                x += 1;
            }
            row = row.byte_add(rd::<i32>(img, 0x10) as isize);
            y += 1;
        }
    }
    img
}

/// Gamma-corrects r, g, b of one RGBA entry through the module table (alpha rewritten unchanged).
fn gamma_entry(p: GPtr<u8>, t: GPtr<u8>) {
    for k in 0..3 {
        let v = rd::<u8>(p, k);
        wr::<u8>(p, k, rd::<u8>(t, v as isize));
    }
    wr::<u8>(p, 3, rd::<u8>(p, 3));
}

/// `RwImageGammaCorrect(image)` (0x803E30): depth 4/8 correct the palette, depth 32 the pixels (rows by
/// stride, `width` entries each); missing pixels/palette is error 0x80000016, other depths 0x80000008
/// (NULL). Sets flags bit 1.
#[hook(addr = 0x803E30, conv = cdecl, name = "RwImageGammaCorrect", expect = "8B 44 24 04 83 EC 08")]
pub fn rw_image_gamma_correct(img: GPtr<u8>) -> GPtr<u8> {
    let d = rd::<i32>(img, 0xC);
    if d == 4 || d == 8 {
        let pal = p8(rd::<u32>(img, 0x18));
        let n = 1u32 << (d as u32 & 31);
        if pal.is_null() {
            error1(0x8000_0016);
            return GPtr::NULL;
        }
        let t = imod().byte_add(0xC);
        for i in 0..n as isize {
            gamma_entry(pal.byte_add(i * 4), t);
        }
    } else if d == 32 {
        let mut row = p8(rd::<u32>(img, 0x14));
        let w = rd::<u32>(img, 4);
        let h = rd::<i32>(img, 8);
        if row.is_null() {
            error1(0x8000_0016);
            return GPtr::NULL;
        }
        if h > 0 {
            for _ in 0..h {
                let t = imod().byte_add(0xC);
                for x in 0..w as isize {
                    gamma_entry(row.byte_add(x * 4), t);
                }
                row = row.byte_add(rd::<i32>(img, 0x10) as isize);
            }
        }
    } else {
        error1(0x8000_0008);
        return GPtr::NULL;
    }
    wr::<u32>(img, 0, rd::<u32>(img, 0) | 2);
    img
}

/// `RwImageSetFromRaster(image, raster)` (0x804250): `stdFunc[6](image, raster, 0)`; a gamma-corrected
/// raster (privateFlags bit 0) marks the image gamma corrected.
#[hook(addr = 0x804250, conv = cdecl, name = "RwImageSetFromRaster", expect = "A1 24 7B C9 00")]
pub fn rw_image_set_from_raster(img: GPtr<u8>, r: GPtr<u8>) -> GPtr<u8> {
    if std_image_get_raster(img, r, 0) == 0 {
        return GPtr::NULL;
    }
    if rd::<u8>(r, 0x22) & 1 != 0 {
        wr::<u32>(img, 0, rd::<u32>(img, 0) | 2);
    }
    img
}

/// `stdFunc[7](raster, image, 0)` plus the gamma flag transfer, shared by the raster-from-image paths.
fn raster_set_image_flagged(r: GPtr<u8>, img: GPtr<u8>) -> u32 {
    let ok = std_raster_set_image(r, img, 0);
    if ok != 0 && rd::<u8>(img, 0) & 2 != 0 {
        wr::<u8>(r, 0x22, rd::<u8>(r, 0x22) | 1);
    }
    ok
}

/// `RwRasterSetFromImage(raster, image)` (0x804290): `stdFunc[7](raster, image, 0)`; a gamma-corrected
/// image sets raster privateFlags bit 0.
#[hook(addr = 0x804290, conv = cdecl, name = "RwRasterSetFromImage", expect = "A1 24 7B C9 00")]
pub fn rw_raster_set_from_image(r: GPtr<u8>, img: GPtr<u8>) -> GPtr<u8> {
    if raster_set_image_flagged(r, img) == 0 {
        return GPtr::NULL;
    }
    r
}

/// A stack `RwRaster` (0x34 bytes) as the find-format callers pass to the driver.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RasterLocal {
    words: [u32; 13],
}

// SAFETY: plain integers.
#[allow(unsafe_code)]
unsafe impl sa_core::mem::GameData for RasterLocal {}

impl RasterLocal {
    fn base(&mut self) -> GPtr<u8> {
        GPtr::from_raw(self).cast()
    }
}

/// `RwImageFindRasterFormat(image, type, &w, &h, &depth, &format)` (0x8042C0): `stdFunc[9]` on a stack
/// raster, then `format = cFormat << 8 | cType`, w, h, depth (stored in that order). The original's
/// stack raster is uninitialised; only the fields the driver writes are read.
#[hook(addr = 0x8042C0, conv = cdecl, name = "RwImageFindRasterFormat", expect = "8B 44 24 08 8B 15 24 7B C9 00")]
pub fn rw_image_find_raster_format(
    img: GPtr<u8>,
    ty: u32,
    w: GPtr<u32>,
    h: GPtr<u32>,
    depth: GPtr<u32>,
    format: GPtr<u32>,
) -> GPtr<u8> {
    let mut ras = RasterLocal::default();
    let r = ras.base();
    if std_image_find_raster_format(r, img, ty) == 0 {
        return GPtr::NULL;
    }
    format.write(((rd::<u8>(r, 0x23) as u32) << 8) | rd::<u8>(r, 0x20) as u32);
    w.write(rd::<u32>(r, 0xC));
    h.write(rd::<u32>(r, 0x10));
    depth.write(rd::<u32>(r, 0x14));
    img
}

/// `RwRasterRead(filename)` (0x8043F0): reads the image, finds the raster format (type 0), resamples
/// to it when the size differs (no NULL check on the resampled image, kept), creates the raster and
/// sets it from the image. Bug kept differently: when the driver's format search fails the original
/// uses the filename pointer as width, height and format and an uninitialised stack word as depth; the
/// port uses the filename the same way and 0 as depth.
#[hook(addr = 0x8043F0, conv = cdecl, name = "RwRasterRead", expect = "8B 44 24 04 83 EC 38")]
pub fn rw_raster_read(name: GPtr<u8>) -> GPtr<u8> {
    let mut img = rw_image_read(name);
    if img.is_null() {
        return GPtr::NULL;
    }
    let mut ras = RasterLocal::default();
    let r = ras.base();
    let (w, h, depth, format);
    if std_image_find_raster_format(r, img, 0) != 0 {
        format = ((rd::<u8>(r, 0x23) as u32) << 8) | rd::<u8>(r, 0x20) as u32;
        depth = rd::<i32>(r, 0x14);
        w = rd::<i32>(r, 0xC);
        h = rd::<i32>(r, 0x10);
    } else {
        format = a32(name);
        h = a32(name) as i32;
        w = a32(name) as i32;
        depth = 0;
    }
    if w != rd::<i32>(img, 4) || h != rd::<i32>(img, 8) {
        let resampled = rw_image_create_resample(img, w, h);
        rw_image_destroy_call(img);
        img = resampled;
    }
    let raster = rw_raster_create_call(w, h, depth, format);
    if raster.is_null() {
        rw_image_destroy_call(img);
        return GPtr::NULL;
    }
    raster_set_image_flagged(raster, img);
    rw_image_destroy_call(img);
    raster
}

/// `RwRasterReadMaskedRaster(name, mask)` (0x8044E0): reads the masked image, creates a raster of its
/// size (depth 0, flags 0) and sets it from the image.
#[hook(addr = 0x8044E0, conv = cdecl, name = "RwRasterReadMaskedRaster", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn rw_raster_read_masked_raster(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> {
    let img = rw_image_read_masked_image(name, mask);
    if img.is_null() {
        return GPtr::NULL;
    }
    let raster = rw_raster_create_call(rd::<i32>(img, 4), rd::<i32>(img, 8), 0, 0);
    if raster.is_null() {
        rw_image_destroy_call(img);
        return GPtr::NULL;
    }
    raster_set_image_flagged(raster, img);
    rw_image_destroy_call(img);
    raster
}

/// `_rwD3D9RGBToPixel(out, rgba, format)` (0x7FEE20): packs an RGBA byte quadruple into the pixel
/// format `format & 0xF00` (1555, 565, 4444, LUM8 = `((30r + 59g + 11b) / 100 * a) / 255`, 8888, 888 with
/// alpha 0xFF, 555); DEFAULT (0) recurses with 8888 (its result kept in the original's `rgba` argument
/// slot). Others: error 0x8000000D, 0.
#[hook(addr = 0x7FEE20, conv = cdecl, name = "_rwD3D9RGBToPixel", expect = "8B 44 24 0C 83 EC 08")]
pub fn d3d9_rgb_to_pixel(out: GPtr<u32>, c: GPtr<u8>, format: u32) -> u32 {
    let r = |k: isize| rd::<u8>(c, k) as u32;
    let v = match format & 0xF00 {
        0x100 => ((r(3) & 0x80) << 8) | ((r(0) & 0xF8) << 7) | ((r(1) & 0xF8) << 2) | (r(2) >> 3),
        0xA00 => ((r(0) & 0xF8) << 7) | ((r(1) & 0xF8) << 2) | (r(2) >> 3),
        0x200 => ((r(0) & 0xF8) << 8) | ((r(1) & 0xFC) << 3) | (r(2) >> 3),
        0x300 => ((r(3) & 0xF0) << 8) | ((r(0) & 0xF0) << 4) | (r(1) & 0xF0) | (r(2) >> 4),
        0x400 => {
            let lum = ((r(1) * 59 + r(2) * 11 + r(0) * 30) as i32 / 100) as u32;
            lum.wrapping_mul(r(3)) / 255
        }
        0x500 => (r(3) << 24) | (r(0) << 16) | (r(1) << 8) | r(2),
        0x600 => 0xFF00_0000 | (r(0) << 16) | (r(1) << 8) | r(2),
        0 => {
            let mut slot = a32(c);
            if rgb_to_pixel_call(GPtr::from_raw(&mut slot), c, 0x500) == 0 {
                return 0;
            }
            slot
        }
        _ => {
            error1(0x8000_000D);
            return 0;
        }
    };
    out.write(v);
    1
}

/// `_rwD3D9PixelToRGB(rgba, pixel, format)` (0x7FF070): PAL8 (format bit 0x2000) copies the dword;
/// otherwise unpacks `*pixel` (always read as a dword) for 1555, 565, 4444 (alpha 0xFF when the
/// nibble is 0xF), LUM8, 8888, 888, 555, with the original's byte store order. Others (incl.
/// DEFAULT): error 0x8000000D, 0.
#[hook(addr = 0x7FF070, conv = cdecl, name = "_rwD3D9PixelToRGB", expect = "8B 44 24 0C 83 EC 08")]
pub fn d3d9_pixel_to_rgb(c: GPtr<u8>, pixel: GPtr<u32>, format: u32) -> u32 {
    if format & 0x2000 != 0 {
        c.cast::<u32>().write(pixel.read());
        return 1;
    }
    let st = |k: isize, v: u32| wr::<u8>(c, k, v as u8);
    match format & 0xF00 {
        0x300 => {
            let p = pixel.read();
            st(3, if p & 0xF000 == 0xF000 { 0xFF } else { (p >> 8) & 0xF0 });
            st(0, (p >> 4) & 0xF0);
            st(2, p << 4);
            st(1, p & 0xF0);
        }
        0x200 => {
            let p = pixel.read();
            st(3, 0xFF);
            st(0, (p >> 8) & 0xF8);
            st(2, p << 3);
            st(1, (p >> 3) & 0xFC);
        }
        0x100 => {
            let p = pixel.read();
            st(3, if p & 0x8000 != 0 { 0xFF } else { 0 });
            st(0, (p >> 7) & 0xF8);
            st(2, p << 3);
            st(1, (p >> 2) & 0xF8);
        }
        0x400 => {
            let p = pixel.read();
            st(3, 0xFF);
            st(0, p);
            st(1, p);
            st(2, p);
        }
        0xA00 => {
            let p = pixel.read();
            st(3, 0xFF);
            st(0, (p >> 7) & 0xF8);
            st(2, p << 3);
            st(1, (p >> 2) & 0xF8);
        }
        0x600 => {
            let p = pixel.read();
            st(3, 0xFF);
            st(2, p);
            st(0, p >> 16);
            st(1, p >> 8);
        }
        0x500 => {
            let p = pixel.read();
            st(2, p);
            st(0, p >> 16);
            st(3, p >> 24);
            st(1, p >> 8);
        }
        _ => {
            error1(0x8000_000D);
            return 0;
        }
    }
    1
}

/// `_rwD3D9ImageFindRasterFormat(raster, image, flags)` (0x7FFF00), `stdFunc[9]`: raster width/height
/// from the image, depth 0; type 0 and 4/5 clamp the size and pick the image's format
/// (`_rwD3D9ImageFindFormat`), combined with `flags & 0xFFFF6FFF` (type 0) or `flags & 0x9000 | type`
/// (4/5); types 1/2 keep `flags`; then `FUN_004cc580(raster, format)`. Type 3 and > 5: error
/// 0x8000000D, 0.
#[hook(addr = 0x7FFF00, conv = cdecl, name = "_rwD3D9ImageFindRasterFormat", expect = "83 EC 08 53 8B 5C 24 10")]
pub fn d3d9_image_find_raster_format(r: GPtr<u8>, img: GPtr<u8>, flags: u32) -> u32 {
    wr::<u32>(r, 0xC, rd::<u32>(img, 4));
    wr::<u32>(r, 0x14, 0);
    wr::<u32>(r, 0x10, rd::<u32>(img, 8));
    let ty = flags & 7;
    match ty {
        0 => {
            clamp_raster_size(r.byte_add(0xC), r.byte_add(0x10), flags);
            let f = image_find_format(img);
            raster_set_format(r, f | (flags & 0xFFFF_6FFF))
        }
        4 | 5 => {
            clamp_raster_size(r.byte_add(0xC), r.byte_add(0x10), flags);
            let f = image_find_format(img);
            raster_set_format(r, ty | (f | (flags & 0x9000)))
        }
        1 | 2 => raster_set_format(r, flags),
        _ => {
            error1(0x8000_000D);
            0
        }
    }
}

orig! {
    /// `G->stringFuncs.rwstrlen(s)` (G+0x120)
    fn rw_strlen(s: GPtr<u8>) -> u32 = cdecl table 0xC97B24 0x120 => unported;
    /// `G->rwrealloc(p, size, hint)` (G+0x13C)
    fn rw_realloc_call(p: u32, size: u32, hint: u32) -> u32 = cdecl table 0xC97B24 0x13C => unported;
    /// `RwImageCreate(w, h, depth)` (0x8026E0)
    fn rw_image_create_call(w: i32, h: i32, depth: i32) -> GPtr<u8> = cdecl 0x8026E0 => rw_image_create;
    /// `RwImageAllocatePixels(image)` (0x8027A0)
    fn rw_image_allocate_pixels_call(img: GPtr<u8>) -> GPtr<u8> = cdecl 0x8027A0 => rw_image_allocate_pixels;
    /// `RwImageFreePixels(image)` (0x802860)
    fn rw_image_free_pixels_call(img: GPtr<u8>) -> GPtr<u8> = cdecl 0x802860 => rw_image_free_pixels;
    /// `RwImageCopy(dst, src)` (0x803950, `_rwImageCopyResample` in rw-names)
    fn rw_image_copy(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x803950 => unported;
    /// `RwImageResample(dst, src)` (0x80C600)
    fn rw_image_resample(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x80C600 => unported;
}

/// `RwImageSetPath(path)` (0x802EA0): grows the module path buffer (M+4, capacity M+8, `rwrealloc`
/// hint 0x1040406) when `strlen + 1` exceeds the capacity (signed compare; failure: error 0x80000013
/// with the size, NULL), then copies `strlen + 1` bytes. Returns path.
#[hook(addr = 0x802EA0, conv = cdecl, name = "RwImageSetPath", expect = "A1 24 7B C9 00")]
pub fn rw_image_set_path(path: GPtr<u8>) -> GPtr<u8> {
    let n = rw_strlen(path).wrapping_add(1);
    if (n as i32) > rd::<i32>(imod(), 8) {
        let p = rw_realloc_call(rd::<u32>(imod(), 4), n, 0x0104_0406);
        if p == 0 {
            let mut e = RwError {
                plugin_id: 1,
                error_code: 0,
            };
            e.error_code = rw_error2(0x8000_0013, n) as i32;
            rw_error_set(GPtr::from_raw(&mut e));
            return GPtr::NULL;
        }
        wr::<u32>(imod(), 4, p);
        wr::<u32>(imod(), 8, n);
    }
    let dst = p8(rd::<u32>(imod(), 4));
    let words = n >> 2;
    for i in 0..words as isize {
        wr::<u32>(dst, i * 4, rd::<u32>(path, i * 4));
    }
    let base = (words * 4) as isize;
    for i in 0..(n & 3) as isize {
        wr::<u8>(dst, base + i, rd::<u8>(path, base + i));
    }
    path
}

/// `RwImageCreateResample(image, width, height)` (0x80CD10): a new 32-bit image of the target size;
/// 32-bit sources are resampled directly, others through a 32-bit copy (`RwImageCopy`) that is freed
/// afterwards. Every failure frees what was created and returns NULL.
#[hook(addr = 0x80CD10, conv = cdecl, name = "RwImageCreateResample", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn rw_image_create_resample_port(src: GPtr<u8>, w: i32, h: i32) -> GPtr<u8> {
    let dst = rw_image_create_call(w, h, 0x20);
    if dst.is_null() {
        return GPtr::NULL;
    }
    if rw_image_allocate_pixels_call(dst).is_null() {
        rw_image_destroy_call(dst);
        return GPtr::NULL;
    }
    if rd::<i32>(src, 0xC) == 0x20 {
        if !rw_image_resample(dst, src).is_null() {
            return dst;
        }
        rw_image_free_pixels_call(dst);
        rw_image_destroy_call(dst);
        return GPtr::NULL;
    }
    let tmp = rw_image_create_call(rd::<i32>(src, 4), rd::<i32>(src, 8), 0x20);
    if tmp.is_null() {
        rw_image_free_pixels_call(dst);
        rw_image_destroy_call(dst);
        return GPtr::NULL;
    }
    if rw_image_allocate_pixels_call(tmp).is_null() {
        rw_image_destroy_call(tmp);
        rw_image_free_pixels_call(dst);
        rw_image_destroy_call(dst);
        return GPtr::NULL;
    }
    rw_image_copy(tmp, src);
    if rw_image_resample(dst, tmp).is_null() {
        rw_image_free_pixels_call(tmp);
        rw_image_destroy_call(tmp);
        rw_image_free_pixels_call(dst);
        rw_image_destroy_call(dst);
        return GPtr::NULL;
    }
    rw_image_free_pixels_call(tmp);
    rw_image_destroy_call(tmp);
    dst
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RW_IMAGE_CREATE,
    HOOK_RW_IMAGE_DESTROY,
    HOOK_RW_IMAGE_ALLOCATE_PIXELS,
    HOOK_RW_IMAGE_FREE_PIXELS,
    HOOK_RW_IMAGE_MAKE_MASK,
    HOOK_RW_IMAGE_GAMMA_CORRECT,
    HOOK_RW_IMAGE_SET_FROM_RASTER,
    HOOK_RW_RASTER_SET_FROM_IMAGE,
    HOOK_RW_IMAGE_FIND_RASTER_FORMAT,
    HOOK_RW_RASTER_READ,
    HOOK_RW_RASTER_READ_MASKED_RASTER,
    HOOK_D3D9_RGB_TO_PIXEL,
    HOOK_D3D9_PIXEL_TO_RGB,
    HOOK_D3D9_IMAGE_FIND_RASTER_FORMAT,
    HOOK_RW_IMAGE_SET_PATH,
    HOOK_RW_IMAGE_CREATE_RESAMPLE_PORT,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
