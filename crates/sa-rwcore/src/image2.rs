//! RW-5 helper B, module `image2`: the RwImage / D3D9 image functions PORT-RW-4 left out (see
//! docs/arch/rw-port.md). Ported from the 1.0 US bytes; `docs/re/specs/RW/RwImage.md`,
//! `RwTexture.md` and `RwD3D9Raster.md` are reading aids.
//!
//! Layouts as in [`crate::image`] (`RwImage`: +0 flags, +4 width, +8 height, +0xC depth, +0x10 stride,
//! +0x14 cpPixels, +0x18 palette) and [`crate::raster`] (`RwRaster`: +4 cpPixels, +8 palette, +0xC
//! width, +0x10 height, +0x14 depth, +0x18 stride, +0x20 cType, +0x21 flags, +0x22 privateFlags,
//! +0x23 cFormat; the D3D9 extension at `raster + [0xB4E9E0]`).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{RwError, g, module, p8, rd, rw_error_set, rw_free, rw_memory_alloc, rw_memory_free, wr};
use crate::raster::D3D9_RASTER_EXT_OFFSET;
use sa_math::x87::X87;
use crate::texture::{TEXTURE_MODULE, TEXTURE_REGISTRY};

fn a32<T: sa_core::mem::GameData>(p: GPtr<T>) -> u32 {
    p.addr() as u32
}

/// E_RW_STRING_TRUNCATION.
const E_RW_TRUNCATED: u32 = 0x8000_001E;

orig! {
    /// `_rwerror(code, a)` (0x8088D0, two arguments)
    fn rw_error2(code: u32, a: u32) -> u32 = cdecl 0x8088D0 => unported;
    /// `_rwerror(code, a, b, c, d)` (0x8088D0, five arguments)
    fn rw_error5(code: u32, a: u32, b: u32, c: u32, d: i32) -> u32 = cdecl 0x8088D0 => unported;
    /// `G->stringFuncs.rwstrncpy(dst, src, n)` (G+0xFC)
    fn rw_strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl table 0xC97B24 0xFC => unported;
    /// `G->stringFuncs.rwstrlen(s)` (G+0x120)
    fn rw_strlen(s: GPtr<u8>) -> u32 = cdecl table 0xC97B24 0x120 => unported;
    /// `_rwPluginRegistryInitObject(reg, obj)` (0x8086E0)
    fn plugin_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => crate::plugin::registry_init_object;
    /// `_rwTextureReadAutoMipmap(name, mask)` (0x7F4AE0)
    fn texture_read_auto_mipmap(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F4AE0 => unported;
    /// The texture module's mipmap name callback (M+0x30)
    fn call_mipmap_name_cb(f: u32, name: GPtr<u8>, mask: GPtr<u8>, level: u32, format: u32) -> u32 = cdecl dyn => unported;
    /// `_rwTextureImageReadAndMask(name, mask, type, &w, &h, &depth, &format)` (0x7F4690)
    fn texture_image_read_and_mask(name: GPtr<u8>, mask: GPtr<u8>, ty: u32, w: GPtr<i32>, h: GPtr<i32>, depth: GPtr<i32>, format: GPtr<u32>) -> GPtr<u8> = cdecl 0x7F4690 => unported;
    /// `RwRasterCreate(w, h, depth, flags)` (0x7FB230)
    fn rw_raster_create(w: i32, h: i32, depth: i32, flags: u32) -> GPtr<u8> = cdecl 0x7FB230 => crate::raster::rw_raster_create;
    /// `RwRasterDestroy(raster)` (0x7FB020)
    fn rw_raster_destroy(r: GPtr<u8>) -> u32 = cdecl 0x7FB020 => crate::raster::rw_raster_destroy;
    /// `RwRasterLock(raster, level, mode)` (0x7FB2D0)
    fn rw_raster_lock(r: GPtr<u8>, level: u32, mode: u32) -> u32 = cdecl 0x7FB2D0 => crate::raster::rw_raster_lock;
    /// `RwRasterUnlock(raster)` (0x7FAEC0)
    fn rw_raster_unlock(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7FAEC0 => crate::raster::rw_raster_unlock;
    /// `RwRasterLockPalette(raster, mode)` (0x7FB0E0)
    fn rw_raster_lock_palette(r: GPtr<u8>, mode: u32) -> u32 = cdecl 0x7FB0E0 => crate::raster::rw_raster_lock_palette;
    /// `RwRasterUnlockPalette(raster)` (0x7FAFF0)
    fn rw_raster_unlock_palette(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7FAFF0 => crate::raster::rw_raster_unlock_palette;
    /// `RwImageDestroy(image)` (0x802740)
    fn rw_image_destroy(img: GPtr<u8>) -> u32 = cdecl 0x802740 => crate::image::rw_image_destroy;
    /// `RwImageGammaCorrect(image)` (0x803E30)
    fn rw_image_gamma_correct(img: GPtr<u8>) -> GPtr<u8> = cdecl 0x803E30 => crate::image::rw_image_gamma_correct;
    /// `RwRasterSetFromImage(raster, image)` (0x804290)
    fn rw_raster_set_from_image(r: GPtr<u8>, img: GPtr<u8>) -> GPtr<u8> = cdecl 0x804290 => crate::image::rw_raster_set_from_image;
    /// `RwTextureSetMaskName(texture, mask)` (0x7F3910)
    fn rw_texture_set_mask_name(t: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3910 => crate::texture::rw_texture_set_mask_name;
    /// `FUN_007f44c0(palette, 0, images, count, bits)`: builds one palette shared by `count` images,
    /// possibly replacing them (0x7F44C0)
    fn images_make_shared_palette(pal: GPtr<u8>, unused: u32, images: GPtr<u32>, count: i32, bits: u32) -> u32 = cdecl 0x7F44C0 => unported;
    /// `FUN_007ff450(image, raster)`: a 32-bit image from the raster pixels (0x7FF450)
    fn d3d9_image_get_raster32(img: GPtr<u8>, r: GPtr<u8>) -> u32 = cdecl 0x7FF450 => unported;
    /// `_rwD3D9ImageFindFormat(image)` (0x7FFFF0)
    fn image_find_format(img: GPtr<u8>) -> u32 = cdecl 0x7FFFF0 => unported;
    /// `RwImageCreateResample(image, w, h)` (0x80CD10)
    fn rw_image_create_resample(img: GPtr<u8>, w: i32, h: i32) -> GPtr<u8> = cdecl 0x80CD10 => crate::image::rw_image_create_resample_port;
    /// `FUN_00800810(d3dFormat)`: the RGBA -> pixel converter for a D3D format (0x800810)
    fn get_rgba_to_pixel_converter(fmt: u32) -> u32 = cdecl 0x800810 => unported;
    /// A converter returned by 0x800810: `u16 conv(const RwRGBA*)`
    fn call_converter(f: u32, rgba: GPtr<u8>) -> u32 = cdecl dyn => unported;
    /// `FUN_00800c80(raster, image)`: 4/8-bit raster from an image (0x800C80)
    fn raster_set_image_pal(r: GPtr<u8>, img: GPtr<u8>) -> u32 = cdecl 0x800C80 => unported;
    /// `FUN_00801390(raster, image)`: 24/32-bit raster from an image (0x801390)
    fn raster_set_image_32(r: GPtr<u8>, img: GPtr<u8>) -> u32 = cdecl 0x801390 => unported;
    /// `_rwerror(code)` (0x8088D0, one argument)
    fn rw_error1(code: u32) -> u32 = cdecl 0x8088D0 => unported;
    /// `_rwPluginRegistryDeInitObject(reg, obj)` (0x808740)
    fn plugin_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => crate::plugin::registry_deinit_object;
    /// `RwImageAllocatePixels(image)` (0x8027A0)
    fn rw_image_allocate_pixels(img: GPtr<u8>) -> GPtr<u8> = cdecl 0x8027A0 => crate::image::rw_image_allocate_pixels;
    /// `FUN_00803790(dst, src)`: image copy for equal depths (0x803790)
    fn image_copy_same_depth(dst: GPtr<u8>, src: GPtr<u8>) -> u32 = cdecl 0x803790 => unported;
    /// `FUN_00803840(dst, src)`: image copy with depth conversion (0x803840)
    fn image_copy_convert(dst: GPtr<u8>, src: GPtr<u8>) -> u32 = cdecl 0x803840 => unported;
    /// `G->stringFuncs.rwstrrchr(s, c)` (G+0x108)
    fn rw_strrchr(s: GPtr<u8>, c: u32) -> GPtr<u8> = cdecl table 0xC97B24 0x108 => unported;
    /// `G->stringFuncs.rwstrcmp(a, b)` (G+0x114)
    fn rw_strcmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl table 0xC97B24 0x114 => unported;
    /// `_rwImageFindFileAndCall(name, mode, cb, data)` (0x8030C0): path search, file open, `cb`
    fn image_find_file_and_call(name: GPtr<u8>, mode: u32, cb: u32, data: GPtr<u8>) -> u32 = cdecl 0x8030C0 => unported;
    /// `RwImageRead(name)` (0x802FD0)
    fn rw_image_read_call(name: GPtr<u8>) -> GPtr<u8> = cdecl 0x802FD0 => rw_image_read;
    /// `RwImageMakeMask(image)` (0x802A20)
    fn rw_image_make_mask(img: GPtr<u8>) -> GPtr<u8> = cdecl 0x802A20 => crate::image::rw_image_make_mask;
    /// `RwImageApplyMask(image, mask)` (0x802AF0)
    fn rw_image_apply_mask_call(img: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl 0x802AF0 => rw_image_apply_mask;
    /// `FUN_0080ca20(src, x0, x1, yfixed, acc)`: accumulates one source row of the 16.16 column span
    /// `[x0, x1]` into four floats (0x80CA20)
    fn resample_accumulate(src: GPtr<u8>, x0: i32, x1: i32, y: i32, acc: GPtr<f32>) -> u32 = cdecl 0x80CA20 => unported;
    /// `_rwerror(code, a, b)` (0x8088D0, three arguments)
    fn rw_error3(code: u32, a: u32, b: u32) -> u32 = cdecl 0x8088D0 => unported;
    /// `RwImageGetPath()` (0x802F60)
    fn rw_image_get_path() -> GPtr<u8> = cdecl 0x802F60 => unported;
    /// `_rwpathisabsolute(name)` (0x80E9B0)
    fn path_is_absolute(name: GPtr<u8>) -> u32 = cdecl 0x80E9B0 => unported;
    /// `G->stringFuncs.rwstrcpy(dst, src)` (G+0xF8)
    fn rw_strcpy(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl table 0xC97B24 0xF8 => unported;
    /// `G->stringFuncs.rwstrcat(dst, src)` (G+0x100)
    fn rw_strcat(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl table 0xC97B24 0x100 => unported;
    /// `G->stringFuncs.rwstrchr(s, c)` (G+0x10C)
    fn rw_strchr(s: GPtr<u8>, c: u32) -> GPtr<u8> = cdecl table 0xC97B24 0x10C => unported;
    /// `G->fileFuncs.rwfexist(name)` (G+0xC4)
    fn rw_fexist(name: GPtr<u8>) -> u32 = cdecl table 0xC97B24 0xC4 => unported;
    /// `RwStreamOpen(type, access, data)` (0x7ECEF0)
    fn rw_stream_open(ty: u32, access: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7ECEF0 => crate::stream::rw_stream_open;
    /// `RwStreamClose(stream, data)` (0x7ECE20)
    fn rw_stream_close(s: GPtr<u8>, data: GPtr<u8>) -> u32 = cdecl 0x7ECE20 => crate::stream::rw_stream_close;
    /// `RwD3D9RasterStreamReadDDS(stream)` (0x81F360)
    fn rw_d3d9_raster_stream_read_dds(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x81F360 => unported;
    /// `RwTextureCreate(raster)` (0x7F37C0)
    fn rw_texture_create(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F37C0 => crate::texture::rw_texture_create;
    /// `RwTextureSetName(texture, name)` (0x7F38A0)
    fn rw_texture_set_name(t: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F38A0 => crate::texture::rw_texture_set_name;
}

/// `RwErrorSet({1, code})` with a code already returned by `_rwerror`.
fn error_set_code(code: u32) {
    let mut e = RwError {
        plugin_id: 1,
        error_code: 0,
    };
    e.error_code = code as i32;
    rw_error_set(GPtr::from_raw(&mut e));
}

/// `rep movsd` of `n >> 2` dwords then `rep movsb` of `n & 3` bytes, forwards.
fn rep_movs(dst: GPtr<u8>, src: GPtr<u8>, n: u32) {
    let words = (n >> 2) as isize;
    for i in 0..words {
        wr::<u32>(dst, i * 4, rd::<u32>(src, i * 4));
    }
    let base = words * 4;
    for i in 0..(n & 3) as isize {
        wr::<u8>(dst, base + i, rd::<u8>(src, base + i));
    }
}

/// `rwstrncpy(dst, src, n)`; when `rwstrlen(src) >= n`: E_RW_STRING_TRUNCATION (src, n, n - 1,
/// `(signed char)src[n - 1]`) and `dst[n - 1] = 0`.
fn strncpy_checked(dst: GPtr<u8>, src: GPtr<u8>, n: u32) {
    rw_strncpy(dst, src, n);
    if rw_strlen(src) >= n {
        let c = rd::<i8>(src, (n - 1) as isize) as i32;
        error_set_code(rw_error5(E_RW_TRUNCATED, a32(src), n, n - 1, c));
        wr::<u8>(dst, (n - 1) as isize, 0);
    }
}

/// `_rwTextureDefaultReadCallBack(name, maskName)` (0x7F40F0), the default `RwTextureRead` read
/// callback. With mipmapping on (texture module +0x1C) it returns `_rwTextureReadAutoMipmap(name,
/// mask)` (0x7F4AE0). Otherwise, in the original's order: `name` and (when non-NULL and non-empty)
/// `mask` are copied into 256-byte stack buffers (`rwstrncpy`; 256+ chars raise E_RW_STRING_TRUNCATION
/// and terminate at [0xFF]; the mask buffer is "" otherwise); the module's mipmap name callback (+0x30),
/// when set, is called as `cb(nameBuf, maskBuf, 0, 4)` (result ignored); `_rwTextureImageReadAndMask
/// (nameBuf, maskBuf, 4, &w, &h, &depth, &format)` (0x7F4690; NULL -> NULL); `RwRasterCreate(w, h,
/// depth, format)` (NULL -> image destroyed, NULL). A palettised raster (cFormat & 0x60) gets a shared
/// palette in a 1 KB stack buffer, `FUN_007f44c0(buf, 0, &image, 1, PAL8 ? 4 : 8)` (as the bytes have
/// it: 4 for PAL8, 8 for PAL4), and `image->palette = buf` (the image may have been replaced through
/// `&image`). Then `RwImageGammaCorrect(image)` (result ignored), `RwRasterSetFromImage` (failure:
/// raster and image destroyed, NULL), the image is destroyed (its palette pointer still points at the
/// stack buffer), and a texture is built inline (texture freelist, hint 0x30006; NULL -> raster
/// destroyed, NULL; the stores of `RwTextureCreate` in this function's order, plugin init) named after
/// the caller's `name` (32-byte copy, truncation error at [0x1F]); a non-NULL `mask` is copied into the
/// mask name the same way (even when empty), a NULL one goes through `RwTextureSetMaskName(texture,
/// (char*)0xC97904)`. Returns the texture.
#[hook(addr = 0x7F40F0, conv = cdecl, name = "_rwTextureDefaultReadCallBack", expect = "A1 24 7B C9 00",
       unverified = "rw5-image-1", route = "split", hazards = "heap-writes,uninit-locals",
       recs = "grove-walk",
       note = "file reads (RwImageRead via 0x7F4690); the 1 KB stack palette may be partly unwritten by 0x7F44C0 before RwRasterSetFromImage reads it (port: zeros)")]
pub fn rw_texture_default_read_callback(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(module(TEXTURE_MODULE), 0x1C) != 0 {
        return texture_read_auto_mipmap(name, mask);
    }
    let mut name_buf = [0u8; 0x100];
    let mut mask_buf = [0u8; 0x100];
    let mut pal_buf = [0u8; 0x400];
    let nb: GPtr<u8> = GPtr::from_raw(&mut name_buf).cast();
    let mb: GPtr<u8> = GPtr::from_raw(&mut mask_buf).cast();
    let pb: GPtr<u8> = GPtr::from_raw(&mut pal_buf).cast();
    strncpy_checked(nb, name, 0x100);
    wr::<u8>(mb, 0, 0);
    if !mask.is_null() && rd::<u8>(mask, 0) != 0 {
        strncpy_checked(mb, mask, 0x100);
    }
    let cb = rd::<u32>(module(TEXTURE_MODULE), 0x30);
    if cb != 0 {
        call_mipmap_name_cb(cb, nb, mb, 0, 4);
    }
    let mut w = 0i32;
    let mut h = 0i32;
    let mut depth = 0i32;
    let mut format = 0u32;
    let img = texture_image_read_and_mask(
        nb,
        mb,
        4,
        GPtr::from_raw(&mut w),
        GPtr::from_raw(&mut h),
        GPtr::from_raw(&mut depth),
        GPtr::from_raw(&mut format),
    );
    if img.is_null() {
        return GPtr::NULL;
    }
    let mut image = a32(img);
    let raster = rw_raster_create(w, h, depth, format);
    if raster.is_null() {
        rw_image_destroy(p8(image));
        return GPtr::NULL;
    }
    let cf = rd::<u8>(raster, 0x23);
    if cf & 0x60 != 0 {
        let bits = if cf & 0x40 != 0 { 4 } else { 8 };
        images_make_shared_palette(pb, 0, GPtr::from_raw(&mut image), 1, bits);
        wr::<u32>(p8(image), 0x18, a32(pb));
    }
    rw_image_gamma_correct(p8(image));
    if rw_raster_set_from_image(raster, p8(image)).is_null() {
        rw_raster_destroy(raster);
        rw_image_destroy(p8(image));
        return GPtr::NULL;
    }
    rw_image_destroy(p8(image));
    let t = rw_memory_alloc(p8(rd::<u32>(module(TEXTURE_MODULE), 8)), 0x30006);
    if t.is_null() {
        rw_raster_destroy(raster);
        return GPtr::NULL;
    }
    wr::<u32>(t, 0x50, 0);
    wr::<u8>(t, 0x51, 0x11);
    wr::<u32>(t, 4, 0);
    wr::<u8>(t, 0x10, 0);
    wr::<u8>(t, 0x30, 0);
    wr::<u32>(t, 0, a32(raster));
    wr::<u32>(t, 0x54, 1);
    wr::<u8>(t, 0x50, 1);
    plugin_init_object(p8(TEXTURE_REGISTRY), t);
    strncpy_checked(t.byte_add(0x10), name, 0x20);
    if mask.is_null() {
        rw_texture_set_mask_name(t, p8(0xC97904));
        return t;
    }
    strncpy_checked(t.byte_add(0x30), mask, 0x20);
    t
}

/// `_rwD3D9ImageGetRaster(image, raster, flags)` (0x7FF270), `stdFunc[6]` (IMAGEGETRASTER). A raster
/// whose D3D9 extension byte +0xA has bits 0xF0 set (compressed) raises `_rwerror(2, 0x8E2600)` and
/// returns 0. Otherwise the raster is locked when not already locked (`privateFlags & 2`;
/// `RwRasterLock(raster, 0, 2)`, result ignored) and its palette when palettised (cFormat & 0x60) and
/// not already palette-locked (`privateFlags & 8`; `RwRasterLockPalette(raster, 2)`, result ignored). A
/// 32-bit image is filled by `FUN_007ff450(image, raster)` (result ignored). An 8-bit image needs an
/// 8-bit raster with a palette format (cFormat & 0xF): the pixels are copied with one block copy of
/// `height * stride` bytes when the image stride equals the raster stride and the raster width equals
/// its stride, else row by row (`width` bytes per row, rows by the two strides); the palette is the
/// raster's (1 KB copy) or, without one, a grey ramp (i, i, i, 0xFF). Other combinations raise
/// `_rwerror(2, 0x8E25B8)` (8-bit image, unsuitable raster) or `_rwerror(2, 0x8E257C)` (other image
/// depths). The locks this function took are released, palette first. Original quirk kept: every path
/// after the compressed check returns 1, the two format errors included.
#[hook(addr = 0x7FF270, conv = cdecl, name = "_rwD3D9ImageGetRaster", expect = "83 EC 14 A1 E0 E9 B4 00",
       unverified = "rw5-image-1", route = "L2", hazards = "heap-writes",
       note = "locks/unlocks the raster (D3D9 LockRect stream); writes the image pixels and palette")]
pub fn d3d9_image_get_raster(img: GPtr<u8>, r: GPtr<u8>, _flags: u32) -> u32 {
    let ext = r.byte_add(g::<i32>(D3D9_RASTER_EXT_OFFSET).read() as isize);
    if rd::<u8>(ext, 0xA) & 0xF0 != 0 {
        error_set_code(rw_error2(2, 0x8E2600));
        return 0;
    }
    let mut pixels_locked = 0u32;
    let mut palette_locked = 0u32;
    if rd::<u8>(r, 0x22) & 2 == 0 {
        rw_raster_lock(r, 0, 2);
        pixels_locked = 1;
    }
    if rd::<u8>(r, 0x23) & 0x60 != 0 && rd::<u8>(r, 0x22) & 8 == 0 {
        rw_raster_lock_palette(r, 2);
        palette_locked = 1;
    }
    let d = rd::<i32>(img, 0xC);
    if d == 0x20 {
        d3d9_image_get_raster32(img, r);
    } else if d == 8 {
        if rd::<i32>(r, 0x14) == 8 && rd::<u8>(r, 0x23) & 0xF != 0 {
            let stride = rd::<i32>(r, 0x18);
            if rd::<i32>(img, 0x10) == stride && rd::<i32>(r, 0xC) == stride {
                let n = rd::<i32>(r, 0x10).wrapping_mul(stride) as u32;
                rep_movs(p8(rd::<u32>(img, 0x14)), p8(rd::<u32>(r, 4)), n);
            } else if rd::<i32>(r, 0x10) > 0 {
                let mut y = 0i32;
                loop {
                    let src = rd::<i32>(r, 0x18).wrapping_mul(y).wrapping_add(rd::<i32>(r, 4));
                    let dst = rd::<i32>(img, 0x10).wrapping_mul(y).wrapping_add(rd::<i32>(img, 0x14));
                    rep_movs(p8(dst as u32), p8(src as u32), rd::<u32>(r, 0xC));
                    y += 1;
                    if y >= rd::<i32>(r, 0x10) {
                        break;
                    }
                }
            }
            let pal = p8(rd::<u32>(r, 8));
            if !pal.is_null() {
                rep_movs(p8(rd::<u32>(img, 0x18)), pal, 0x400);
            } else {
                for i in 0..0x100isize {
                    wr::<u8>(p8(rd::<u32>(img, 0x18)), i * 4, i as u8);
                    wr::<u8>(p8(rd::<u32>(img, 0x18)), i * 4 + 1, i as u8);
                    wr::<u8>(p8(rd::<u32>(img, 0x18)), i * 4 + 2, i as u8);
                    wr::<u8>(p8(rd::<u32>(img, 0x18)), i * 4 + 3, 0xFF);
                }
            }
        } else {
            error_set_code(rw_error2(2, 0x8E25B8));
        }
    } else {
        error_set_code(rw_error2(2, 0x8E257C));
    }
    if palette_locked == 1 {
        rw_raster_unlock_palette(r);
    }
    if pixels_locked == 1 {
        rw_raster_unlock(r);
    }
    1
}
/// Rows `0..raster.height`, columns `0..raster.width` (both re-read every step as the original does):
/// `dst(raster row, u16) = f(src(image row, 4 bytes per pixel))`.
fn convert_rows(r: GPtr<u8>, img: GPtr<u8>, mut f: impl FnMut(GPtr<u8>) -> u16) {
    let mut y = 0i32;
    while y < rd::<i32>(r, 0x10) {
        let mut src = p8(rd::<i32>(img, 0x10).wrapping_mul(y).wrapping_add(rd::<i32>(img, 0x14)) as u32);
        let mut dst = p8(rd::<i32>(r, 0x18).wrapping_mul(y).wrapping_add(rd::<i32>(r, 4)) as u32);
        let mut x = 0i32;
        while x < rd::<i32>(r, 0xC) {
            wr::<u16>(dst, 0, f(src));
            src = src.byte_add(4);
            dst = dst.byte_add(2);
            x += 1;
        }
        y += 1;
    }
}

/// `_rwD3D9RasterSetImage(raster, image, flags)` (0x8001E0), `stdFunc[7]` (RASTERSETIMAGE). A
/// compressed raster (D3D9 extension byte +0xA & 0xF0) raises `_rwerror(2, 0x8E2638)` and returns 0.
/// Then the extension's alpha byte (+8) is set: for cType 0 from the byte at `0x85C675 + 8 *
/// ((_rwD3D9ImageFindFormat(image) >> 8) & 0xF)`, else for D3DFMT_P8 (0x29) `(format & 0xF00) == 0x500`
/// (format = cFormat << 8), else the byte at `0xB4E7E0 + 4 * d3dFormat`; a set flag is also set on the
/// parent raster when the raster is a sub-raster. An image of another size is replaced by
/// `RwImageCreateResample(image, raster.w, raster.h)` (no NULL check, kept). Unless already locked
/// (`privateFlags & 4`) the raster is locked with `RwRasterLock(raster, 0, 5)` (NULL -> return 0; the
/// resampled image leaks, kept); a palette format (format & 0x6000) locks the palette unless
/// `privateFlags & 0x10` (`RwRasterLockPalette(raster, 5)`, NULL -> return 0 with the pixel lock held
/// and the image leaked, kept). By raster depth: 4/8 -> `FUN_00800c80(raster, image)`; 24/32 ->
/// `FUN_00801390(raster, image)`; 64 -> nothing; 16 -> by image depth: 4/8 build a 16-bit lookup of
/// the palette (16 entries for depth 4, 256 for 8) through the converter `FUN_00800810(d3dFormat)` and
/// map each image byte through it; 32 packs RGBA directly for X1R5G5B5 (24), R5G6B5 (23), A1R5G5B5 (25),
/// A4R4G4B4 (26) or calls the converter per pixel for other D3D formats; other image depths do nothing.
/// Other raster depths raise 0x8000000C. Afterwards the locks this function took are released (palette
/// first), a resampled image is destroyed, and it returns 1. Uninitialised read kept as a difference:
/// a 4-bit image whose pixel bytes exceed 15 reads lookup entries the original never wrote (stack
/// garbage); the port's lookup is zero-filled.
#[hook(addr = 0x8001E0, conv = cdecl, name = "_rwD3D9RasterSetImage", expect = "81 EC 20 02 00 00",
       unverified = "rw5-image-1", route = "L2", hazards = "heap-writes,uninit-locals",
       note = "locks/unlocks the raster (D3D9 LockRect stream) and writes its pixels; may create and destroy a resampled image")]
pub fn d3d9_raster_set_image(r: GPtr<u8>, image: GPtr<u8>, _flags: u32) -> u32 {
    let ext = r.byte_add(g::<i32>(D3D9_RASTER_EXT_OFFSET).read() as isize);
    let mut was_locked = 0u32;
    let mut pal_was_locked = 0u32;
    if rd::<u8>(ext, 0xA) & 0xF0 != 0 {
        error_set_code(rw_error2(2, 0x8E2638));
        return 0;
    }
    let format = (rd::<u8>(r, 0x23) as u32) << 8;
    let mut img = image;
    if rd::<u8>(r, 0x20) == 0 {
        let f = image_find_format(img);
        let idx = ((f as i32) >> 8) & 0xF;
        wr::<u8>(ext, 8, rd::<u8>(p8(0x85C675), idx as isize * 8));
    } else {
        let fmt = rd::<u32>(ext, 0x18);
        if fmt == 0x29 {
            wr::<u8>(ext, 8, (format & 0xF00 == 0x500) as u8);
        } else {
            wr::<u8>(ext, 8, rd::<u8>(p8(0xB4E7E0), (fmt as isize).wrapping_mul(4)));
        }
    }
    if rd::<u8>(ext, 8) != 0 {
        let parent = rd::<u32>(r, 0);
        if a32(r) != parent {
            let off = g::<i32>(D3D9_RASTER_EXT_OFFSET).read();
            wr::<u8>(p8(parent), off as isize + 8, 1);
        }
    }
    let rw = rd::<i32>(r, 0xC);
    if rw != rd::<i32>(img, 4) || rd::<i32>(r, 0x10) != rd::<i32>(img, 8) {
        img = rw_image_create_resample(img, rw, rd::<i32>(r, 0x10));
    }
    if rd::<u8>(r, 0x22) & 4 != 0 {
        was_locked = 1;
    } else if rw_raster_lock(r, 0, 5) == 0 {
        return 0;
    }
    let pal_format = format & 0x6000;
    if pal_format != 0 {
        if rd::<u8>(r, 0x22) & 0x10 != 0 {
            pal_was_locked = 1;
        } else if rw_raster_lock_palette(r, 5) == 0 {
            return 0;
        }
    }
    match rd::<i32>(r, 0x14) {
        4 | 8 => {
            raster_set_image_pal(r, img);
        }
        24 | 32 => {
            raster_set_image_32(r, img);
        }
        64 => {}
        16 => {
            let id = rd::<i32>(img, 0xC);
            if id == 4 || id == 8 {
                let conv = get_rgba_to_pixel_converter(rd::<u32>(ext, 0x18));
                let mut lut = [0u16; 0x100];
                let n = if id == 4 { 0x10 } else { 0x100 };
                for (i, e) in lut.iter_mut().enumerate().take(n) {
                    *e = call_converter(conv, p8(rd::<u32>(img, 0x18)).byte_add(i as isize * 4)) as u16;
                }
                let mut y = 0i32;
                while y < rd::<i32>(r, 0x10) {
                    let src = p8(rd::<i32>(img, 0x10).wrapping_mul(y).wrapping_add(rd::<i32>(img, 0x14)) as u32);
                    let dst = p8(rd::<i32>(r, 0x18).wrapping_mul(y).wrapping_add(rd::<i32>(r, 4)) as u32);
                    let mut x = 0i32;
                    while x < rd::<i32>(r, 0xC) {
                        wr::<u16>(dst, x as isize * 2, lut[rd::<u8>(src, x as isize) as usize]);
                        x += 1;
                    }
                    y += 1;
                }
            } else if id == 0x20 {
                let b = |p: GPtr<u8>, k: isize| rd::<u8>(p, k) as u16;
                match rd::<u32>(ext, 0x18) {
                    24 => convert_rows(r, img, |p| ((b(p, 0) & 0xF8) << 7) | ((b(p, 1) & 0xF8) << 2) | (b(p, 2) >> 3)),
                    23 => convert_rows(r, img, |p| ((b(p, 0) & 0xF8) << 8) | ((b(p, 1) & 0xFC) << 3) | (b(p, 2) >> 3)),
                    25 => convert_rows(r, img, |p| {
                        ((b(p, 3) & 0x80) << 8) | ((b(p, 0) & 0xF8) << 7) | ((b(p, 1) & 0xF8) << 2) | (b(p, 2) >> 3)
                    }),
                    26 => convert_rows(r, img, |p| {
                        ((b(p, 3) & 0xF0) << 8) | ((b(p, 0) & 0xF0) << 4) | (b(p, 1) & 0xF0) | (b(p, 2) >> 4)
                    }),
                    fmt => {
                        let conv = get_rgba_to_pixel_converter(fmt);
                        convert_rows(r, img, |p| call_converter(conv, p) as u16);
                    }
                }
            }
        }
        _ => {
            let mut e = RwError {
                plugin_id: 1,
                error_code: 0,
            };
            e.error_code = rw_error1(0x8000_000C) as i32;
            rw_error_set(GPtr::from_raw(&mut e));
        }
    }
    if pal_format != 0 && pal_was_locked == 0 {
        rw_raster_unlock_palette(r);
    }
    if was_locked == 0 {
        rw_raster_unlock(r);
    }
    if img != image {
        rw_image_destroy(img);
    }
    1
}
/// IMAGEMODULE `G + [0xC9A65C]`; +0 is the image freelist.
fn image_free_list() -> GPtr<u8> {
    p8(rd::<u32>(module(crate::image::IMAGE_MODULE), 0))
}

/// The inline `RwImageDestroy` of the originals: owned pixels freed (flags bit 0), plugin
/// destructors, freelist free.
fn image_destroy_inline(img: GPtr<u8>) {
    if rd::<u8>(img, 0) & 1 != 0 {
        rw_free(p8(rd::<u32>(img, 0x14)));
        wr::<u32>(img, 0x14, 0);
        wr::<u32>(img, 0x18, 0);
        wr::<u32>(img, 0, rd::<u32>(img, 0) & !1);
    }
    plugin_deinit_object(p8(crate::image::IMAGE_REGISTRY), img);
    rw_memory_free(image_free_list(), img);
}

/// `flags = (flags & ~2) | (src.flags & 2)` (gamma-corrected bit) as the originals store it twice.
fn copy_gamma_flag(dst: GPtr<u8>, src: GPtr<u8>) {
    let f = rd::<u32>(dst, 0) & !2;
    wr::<u32>(dst, 0, f);
    wr::<u32>(dst, 0, (rd::<u32>(src, 0) & 2) | f);
}

/// `RwImageApplyMask(image, mask)` (0x802AF0). Sizes must match (else error 0x8000000A, NULL). A 4/8-bit
/// image is first made 32-bit in place: a temporary image of its size and depth is created inline
/// (image freelist, hint 0x30018, plugin init; NULL -> NULL) and allocated (failure: inline destroy,
/// NULL), filled from the image (`FUN_00803790(tmp, image)` for equal depths, else `FUN_00803840`), its
/// gamma bit taken from the image; the image's owned pixels are freed, its depth set to 32 and its
/// pixels re-allocated (result ignored); the image is filled back from the temporary (equal depths:
/// palette copy of `4 << depth` bytes when both have one and depth <= 8, then rows of `((depth + 7) >> 3)
/// * width` bytes; unequal, the only reachable case: `FUN_00803840(image, tmp)`), the gamma bit is
/// copied back, and the temporary's pixels are freed unconditionally before its plugin destructors and
/// freelist free. Other image depths than 4/8/32: error 0x80000009, NULL. Then, row by row, the alpha
/// byte of every image pixel becomes the mask's: the mask palette entry's alpha for a 4/8-bit mask, the
/// pixel's alpha for a 32-bit mask (other mask depths change nothing). Returns the image. Original bugs
/// kept: a failing fill (0 from 0x803790/0x803840) makes the gamma-bit store write through NULL (crash).
/// Not reproduced: the original keeps `image.width` and row pointers in its `image` argument slot.
#[hook(addr = 0x802AF0, conv = cdecl, name = "RwImageApplyMask", expect = "83 EC 10 53 8B 5C 24 18",
       unverified = "rw5-image-1", route = "L2", hazards = "heap-writes",
       note = "palettised images are converted to 32 bits in place (pixels freed and re-allocated)")]
pub fn rw_image_apply_mask(img: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> {
    let w = rd::<i32>(img, 4);
    if w != rd::<i32>(mask, 4) || rd::<i32>(img, 8) != rd::<i32>(mask, 8) {
        error_set_code(rw_error1(0x8000_000A));
        return GPtr::NULL;
    }
    let h = rd::<i32>(img, 8);
    let d = rd::<i32>(img, 0xC);
    if d == 4 || d == 8 {
        let tmp = rw_memory_alloc(image_free_list(), 0x30018);
        if tmp.is_null() {
            return GPtr::NULL;
        }
        wr::<i32>(tmp, 4, w);
        wr::<i32>(tmp, 8, h);
        wr::<i32>(tmp, 0xC, d);
        wr::<u32>(tmp, 0x14, 0);
        wr::<u32>(tmp, 0x18, 0);
        wr::<u32>(tmp, 0, 0);
        plugin_init_object(p8(crate::image::IMAGE_REGISTRY), tmp);
        if rw_image_allocate_pixels(tmp).is_null() {
            image_destroy_inline(tmp);
            return GPtr::NULL;
        }
        let ok = if rd::<i32>(tmp, 0xC) == rd::<i32>(img, 0xC) {
            image_copy_same_depth(tmp, img)
        } else {
            image_copy_convert(tmp, img)
        };
        copy_gamma_flag(if ok != 0 { tmp } else { GPtr::NULL }, img);
        if rd::<u8>(img, 0) & 1 != 0 {
            rw_free(p8(rd::<u32>(img, 0x14)));
            wr::<u32>(img, 0x14, 0);
            wr::<u32>(img, 0x18, 0);
            wr::<u32>(img, 0, rd::<u32>(img, 0) & !1);
        }
        wr::<i32>(img, 0xC, 0x20);
        rw_image_allocate_pixels(img);
        let td = rd::<i32>(tmp, 0xC);
        let mut dst = img;
        if rd::<i32>(img, 0xC) == td {
            let ipal = rd::<u32>(img, 0x18);
            let tpal = rd::<u32>(tmp, 0x18);
            if ipal != 0 && tpal != 0 && td <= 8 {
                rep_movs(p8(ipal), p8(tpal), (1u32 << (td as u32 & 31)) << 2);
            }
            let row = ((rd::<i32>(img, 0xC) + 7) >> 3).wrapping_mul(rd::<i32>(img, 4)) as u32;
            let mut src = rd::<u32>(tmp, 0x14);
            let mut out = rd::<u32>(img, 0x14);
            let mut y = 0i32;
            while y < rd::<i32>(img, 8) {
                rep_movs(p8(out), p8(src), row);
                src = src.wrapping_add(rd::<u32>(tmp, 0x10));
                out = out.wrapping_add(rd::<u32>(img, 0x10));
                y += 1;
            }
        } else if image_copy_convert(img, tmp) == 0 {
            dst = GPtr::NULL;
        }
        copy_gamma_flag(dst, tmp);
        rw_free(p8(rd::<u32>(tmp, 0x14)));
        let f = rd::<u32>(tmp, 0) & !1;
        wr::<u32>(tmp, 0x14, 0);
        wr::<u32>(tmp, 0x18, 0);
        wr::<u32>(tmp, 0, f);
        plugin_deinit_object(p8(crate::image::IMAGE_REGISTRY), tmp);
        rw_memory_free(image_free_list(), tmp);
    } else if d != 0x20 {
        error_set_code(rw_error1(0x8000_0009));
        return GPtr::NULL;
    }
    let mpal = rd::<u32>(mask, 0x18);
    let mut irow = rd::<u32>(img, 0x14);
    let mut mrow = rd::<u32>(mask, 0x14);
    let mut y = 0i32;
    while y < rd::<i32>(img, 8) {
        let md = rd::<i32>(mask, 0xC);
        if md == 4 || md == 8 {
            let mut x = 0i32;
            while x < rd::<i32>(img, 4) {
                let idx = rd::<u8>(p8(mrow), x as isize) as isize;
                wr::<u8>(p8(irow), x as isize * 4 + 3, rd::<u8>(p8(mpal), idx * 4 + 3));
                x += 1;
            }
        } else if md == 0x20 {
            let mut x = 0i32;
            while x < rd::<i32>(img, 4) {
                wr::<u8>(p8(irow), x as isize * 4 + 3, rd::<u8>(p8(mrow), x as isize * 4 + 3));
                x += 1;
            }
        }
        mrow = mrow.wrapping_add(rd::<u32>(mask, 0x10));
        irow = irow.wrapping_add(rd::<u32>(img, 0x10));
        y += 1;
    }
    img
}
/// The `{reader, result}` block `RwImageRead` hands to `_rwImageFindFileAndCall`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ReadCbData {
    reader: u32,
    image: u32,
}

// SAFETY: two integers.
#[allow(unsafe_code)]
unsafe impl sa_core::mem::GameData for ReadCbData {}

/// Finds the extension (the last '.' after the last ':', '/' and '\\', each searched with `rwstrrchr`
/// from the previous hit) and its file format in the image module's format list (M+0x21C, next at
/// +0x30; an entry matches when `rwstrcmp` of its lower-case (+0) or upper-case (+0x14) extension
/// with the extension is 0). Returns the entry's reader (+0x28), or 0 when there is no extension, no
/// matching entry or no reader.
fn find_image_reader(name: GPtr<u8>) -> u32 {
    let mut s = name;
    for ch in *b":/\\" {
        let p = rw_strrchr(s, ch as u32);
        if !p.is_null() {
            s = p;
        }
    }
    let ext = rw_strrchr(s, b'.' as u32);
    if ext.is_null() {
        return 0;
    }
    let mut e = p8(rd::<u32>(module(crate::image::IMAGE_MODULE), 0x21C));
    while !e.is_null() {
        if rw_strcmp(e, ext) == 0 || rw_strcmp(e.byte_add(0x14), ext) == 0 {
            return rd::<u32>(e, 0x28);
        }
        e = p8(rd::<u32>(e, 0x30));
    }
    0
}

/// `RwImageRead(name)` (0x802FD0): finds the reader of the file's extension (see
/// [`find_image_reader`]; none -> NULL) and returns the image `_rwImageFindFileAndCall(name, 5,
/// _rwImageReadFileCB (0x8032D0), &{reader, NULL})` (0x8030C0) leaves in the block.
#[hook(addr = 0x802FD0, conv = cdecl, name = "RwImageRead", expect = "A1 24 7B C9 00",
       unverified = "rw5-image-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "opens and reads the image file (through the image path search 0x8030C0)")]
pub fn rw_image_read(name: GPtr<u8>) -> GPtr<u8> {
    let reader = find_image_reader(name);
    if reader == 0 {
        return GPtr::NULL;
    }
    let mut d = ReadCbData { reader, image: 0 };
    let dp: GPtr<ReadCbData> = GPtr::from_raw(&mut d);
    image_find_file_and_call(name, 5, 0x8032D0, dp.cast());
    p8(dp.read().image)
}
/// `RwImageReadMaskedImage(name, maskName)` (0x8035C0): `RwImageRead(name)` (NULL -> NULL); without a
/// mask name (NULL or "") the image is returned. Otherwise `RwImageRead(maskName)` (NULL -> the image is
/// destroyed inline, NULL), `RwImageMakeMask(mask)` and `RwImageApplyMask(image, mask)`; both
/// succeeding destroy the mask and return the image, else image and mask are destroyed (inline
/// `RwImageDestroy`, image first) and NULL is returned.
#[hook(addr = 0x8035C0, conv = cdecl, name = "RwImageReadMaskedImage", expect = "8B 44 24 04 53 56 57",
       unverified = "rw5-image-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "reads the image and mask files")]
pub fn rw_image_read_masked_image(name: GPtr<u8>, mask_name: GPtr<u8>) -> GPtr<u8> {
    let img = rw_image_read_call(name);
    if img.is_null() {
        return GPtr::NULL;
    }
    if mask_name.is_null() || rd::<u8>(mask_name, 0) == 0 {
        return img;
    }
    let mask = rw_image_read_call(mask_name);
    if mask.is_null() {
        image_destroy_inline(img);
        return GPtr::NULL;
    }
    if !rw_image_make_mask(mask).is_null() && !rw_image_apply_mask_call(img, mask).is_null() {
        image_destroy_inline(mask);
        return img;
    }
    image_destroy_inline(img);
    image_destroy_inline(mask);
    GPtr::NULL
}
/// `fistp dword` with RC = chop (the inline `fnstcw; or ah, 0xC; fldcw; fistp` sequences) of a value
/// just stored as f32 and reloaded: truncation toward zero; NaN and values outside the i32 range give
/// the integer indefinite 0x80000000.
fn fistp_chop_f32(v: f32) -> i32 {
    let d = v as f64;
    if v.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&d) {
        i32::MIN
    } else {
        d as i32
    }
}

/// 65536.0 (0x885050).
const F_65536: u32 = 0x4780_0000;
/// 1/65536 (0x859D5C).
const F_INV_65536: u32 = 0x3780_0000;
/// 1.0 (0x858624).
const F_ONE: u32 = 0x3F80_0000;
/// 255.0 (0x859AAC).
const F_255: u32 = 0x437F_0000;
/// 0.5 (0x858B8C).
const F_HALF: u32 = 0x3F00_0000;

/// `RwImageResample(dst, src)` (0x80C600): box-filter resample of a 32-bit `src` into the 32-bit `dst`.
/// `dst.flags |= src.flags & 2`; 16.16 steps `xstep = chop(f32(fild(src.w) / fild(dst.w) * 65536))`,
/// `ystep` likewise from the heights. For each destination row `y` (`yf = y * ystep`) and column `x`
/// (`xf = x * xstep`): `FUN_0080ca20(src, xf, xf + xstep - 1, yfixed, acc)` (0x80CA20) accumulates one
/// source row band into four floats. When the band `[yf, yf + ystep - 1]` stays in one source row the
/// colour is `acc * yspan` (`yspan = f32((ystep - 1) * 2^-16)`); otherwise the first row is weighted by
/// `(next row - yf) * 2^-16`, whole rows in between are added (`acc1[0] + acc0[0]`, then `acc0[i] +
/// acc1[i]`, as the bytes order them) and the last row is weighted by `(yf + ystep - 1 - row) * 2^-16`.
/// The colour is then scaled by `k = 1 / yspan` (r with the register value of k, g/b/a with its stored
/// f32; r and g stay x87 register values until this multiply) and every channel is stored as
/// `chop(f32(c * 255 + 0.5))` (low byte). All x87 operations follow the bytes under PC=24 ([`X87`]).
/// Returns `dst`.
#[hook(addr = 0x80C600, conv = cdecl, name = "RwImageResample", expect = "83 EC 70 8B 44 24 78",
       unverified = "rw5-image-1", route = "L2", hazards = "heap-writes",
       note = "x87 (PC=24) arithmetic without transcendentals; writes the dst pixels")]
pub fn rw_image_resample(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> {
    let dw = rd::<i32>(dst, 4);
    let sw = rd::<i32>(src, 4);
    let sh = rd::<i32>(src, 8);
    let sflags = rd::<u32>(src, 0);
    let dh = rd::<i32>(dst, 8);
    wr::<u32>(dst, 0, rd::<u32>(dst, 0) | (sflags & 2));
    let c65536 = f32::from_bits(F_65536);
    let cinv = f32::from_bits(F_INV_65536);
    let xstep = fistp_chop_f32(X87::from_i32(sw).fdiv(X87::from_i32(dw)).fmul_m(c65536).to_f32());
    let ystep = fistp_chop_f32(X87::from_i32(sh).fdiv(X87::from_i32(dh)).fmul_m(c65536).to_f32());
    let mut yf = 0i32;
    let mut y = 0i32;
    while y < dh {
        let mut out = rd::<i32>(dst, 0x10).wrapping_mul(y).wrapping_add(rd::<i32>(dst, 0x14)) as u32;
        if dw > 0 {
            let yend = ystep.wrapping_add(yf).wrapping_sub(1);
            let yspan = X87::from_i32(yend.wrapping_sub(yf)).fmul_m(cinv).to_f32();
            let y0 = yf >> 16;
            let y1 = yend >> 16;
            let mut xf = 0i32;
            let mut count = dw;
            loop {
                let xend = xstep.wrapping_add(xf).wrapping_sub(1);
                let mut acc0 = [0f32; 4];
                let mut acc1 = [0f32; 4];
                let a0: GPtr<f32> = GPtr::from_raw(&mut acc0).cast();
                let a1: GPtr<f32> = GPtr::from_raw(&mut acc1).cast();
                let (r, g);
                if y0 == y1 {
                    resample_accumulate(src, xf, xend, yf, a0);
                    let w = yspan;
                    let v = |i: isize| a0.byte_add(i * 4).read();
                    r = X87::from_f32(w).fmul_m(v(0));
                    g = X87::from_f32(v(1)).fmul_m(w);
                    a0.byte_add(8).write(X87::from_f32(v(2)).fmul_m(w).to_f32());
                    a0.byte_add(12).write(X87::from_f32(v(3)).fmul_m(w).to_f32());
                } else {
                    let mut row = (yf as u32 & 0xFFFF_0000).wrapping_add(0x1_0000) as i32;
                    resample_accumulate(src, xf, xend, yf, a0);
                    let wf = X87::from_i32(row.wrapping_sub(yf)).fmul_m(cinv);
                    for i in 0..4 {
                        let p = a0.byte_add(i * 4);
                        p.write(X87::from_f32(p.read()).fmul(wf).to_f32());
                    }
                    while row >> 16 != y1 {
                        resample_accumulate(src, xf, xend, row, a1);
                        a0.write(X87::from_f32(a1.read()).fadd_m(a0.read()).to_f32());
                        row = row.wrapping_add(0x1_0000);
                        for i in 1..4 {
                            let p = a0.byte_add(i * 4);
                            p.write(X87::from_f32(p.read()).fadd_m(a1.byte_add(i * 4).read()).to_f32());
                        }
                    }
                    resample_accumulate(src, xf, xend, row, a1);
                    let wl = X87::from_i32(ystep.wrapping_add(yf).wrapping_sub(1).wrapping_sub(row)).fmul_m(cinv);
                    for i in 0..4 {
                        let p = a1.byte_add(i * 4);
                        p.write(X87::from_f32(p.read()).fmul(wl).to_f32());
                    }
                    r = X87::from_f32(a1.read()).fadd_m(a0.read());
                    g = X87::from_f32(a0.byte_add(4).read()).fadd_m(a1.byte_add(4).read());
                    for i in 2..4 {
                        let p = a0.byte_add(i * 4);
                        p.write(X87::from_f32(p.read()).fadd_m(a1.byte_add(i * 4).read()).to_f32());
                    }
                }
                let k = X87::from_f32(f32::from_bits(F_ONE)).fdiv_m(yspan);
                let k32 = k.to_f32();
                a0.write(k.fmul(r).to_f32());
                a0.byte_add(4).write(g.fmul_m(k32).to_f32());
                for i in 2..4 {
                    let p = a0.byte_add(i * 4);
                    p.write(X87::from_f32(p.read()).fmul_m(k32).to_f32());
                }
                for i in 0..4 {
                    let t = X87::from_f32(a0.byte_add(i * 4).read())
                        .fmul_m(f32::from_bits(F_255))
                        .fadd_m(f32::from_bits(F_HALF))
                        .to_f32();
                    wr::<u8>(p8(out), i, fistp_chop_f32(t) as u8);
                }
                xf = xf.wrapping_add(xstep);
                out = out.wrapping_add(4);
                count = count.wrapping_sub(1);
                if count == 0 {
                    break;
                }
            }
        }
        yf = yf.wrapping_add(ystep);
        y += 1;
    }
    dst
}
/// `_rwD3D9DecompressDXT1(width, height, dst, src)` (0x81E720): decodes DXT1 blocks (8 bytes each,
/// `width / 4` per block row and `height / 4` block rows, both signed divisions truncating toward
/// zero) into 32-bit pixels at `dst` (row pitch `width * 4` bytes; colours stored as the bytes R, G, B,
/// A). Per block: c0/c1 from the two RGB565 words (`r << 3`, `g << 2`, `b << 3`, A 0xFF). When the c0
/// word is greater than the c1 word (unsigned): c2 = (2 c0 + c1) / 3, c3 = (c0 + 2 c1) / 3 per channel,
/// both opaque; else c2 = (c0 + c1) / 2 (opaque) and c3 = 0 (transparent black), and any pixel using
/// c3 makes the result 1. Each of the four index bytes (+4..+7) gives one pixel row, two bits per pixel
/// from the low bits up. Returns 1 when a transparent pixel was decoded, else 0.
#[hook(addr = 0x81E720, conv = cdecl, name = "_rwD3D9DecompressDXT1", expect = "83 EC 34 56 8B 74 24 3C",
       unverified = "rw5-image-1", route = "L2", hazards = "heap-writes",
       note = "pure integer decoder (no game state); writes the dst pixel buffer")]
pub fn d3d9_decompress_dxt1(width: i32, height: i32, dst: GPtr<u8>, src: GPtr<u8>) -> u32 {
    let bw = width / 4;
    let bh = height / 4;
    let mut result = 0u32;
    if bh <= 0 {
        return 0;
    }
    let row_pitch = (width as u32).wrapping_shl(4);
    let next_row = (width as u32).wrapping_mul(4).wrapping_sub(16);
    let mut drow = a32(dst);
    let mut srow = a32(src);
    for _ in 0..bh {
        if bw > 0 {
            let mut d = drow;
            let mut s = p8(srow);
            for _ in 0..bw {
                let w0 = rd::<u16>(s, 0) as u32;
                let w1 = rd::<u16>(s, 2) as u32;
                let (r0, g0, b0) = (((w0 >> 11) << 3) & 0xFF, ((w0 >> 5) << 2) & 0xFF, (w0 << 3) & 0xFF);
                let (r1, g1, b1) = (((w1 >> 11) << 3) & 0xFF, ((w1 >> 5) << 2) & 0xFF, (w1 << 3) & 0xFF);
                let rgba = |r: u32, g: u32, b: u32, a: u32| r | (g << 8) | (b << 16) | (a << 24);
                let c0 = rgba(r0, g0, b0, 0xFF);
                let c1 = rgba(r1, g1, b1, 0xFF);
                let four = w0 > w1;
                let (c2, c3) = if four {
                    (
                        rgba((r1 + 2 * r0) / 3, (g1 + 2 * g0) / 3, (b1 + 2 * b0) / 3, 0xFF),
                        rgba((r0 + 2 * r1) / 3, (g0 + 2 * g1) / 3, (b0 + 2 * b1) / 3, 0xFF),
                    )
                } else {
                    (rgba((r1 + r0) / 2, (g1 + g0) / 2, (b1 + b0) / 2, 0xFF), 0)
                };
                let mut transparent = 0u32;
                let mut p = d;
                for row in 0..4isize {
                    let mut idx = rd::<u8>(s, 4 + row) as u32;
                    for _ in 0..4 {
                        let c = match idx & 3 {
                            0 => c0,
                            1 => c1,
                            2 => c2,
                            _ => {
                                if !four {
                                    transparent = 1;
                                }
                                c3
                            }
                        };
                        wr::<u32>(p8(p), 0, c);
                        p = p.wrapping_add(4);
                        idx >>= 2;
                    }
                    p = p.wrapping_add(next_row);
                }
                if !four {
                    result |= transparent;
                }
                d = d.wrapping_add(16);
                s = s.byte_add(8);
            }
        }
        srow = srow.wrapping_add((bw as u32).wrapping_mul(8));
        drow = drow.wrapping_add(row_pitch);
    }
    result
}
/// `RwD3D9DDSTextureRead(name, maskName)` (0x820A10): `maskName` is ignored. The file name is
/// `<dir><name>.dds` (".dds" from 0x8E2B24) in a 256-byte stack buffer: with a relative `name`
/// (`_rwpathisabsolute(name) != 1`) and a non-empty `RwImageGetPath()`, each ';'-separated entry of
/// the path is tried in order (`rwstrchr(entry, ';')` is called twice, the first result unused; the
/// entry is copied with `rep movs`, then `rwstrcpy(buf + len, name)`, `rwstrcat(buf, ".dds")`) until
/// `rwfexist(buf)` succeeds; an empty entry or the end of the list raises `_rwerror(0x16, name,
/// "(null)")` (0x8E240C) and returns NULL. An absolute `name` or an empty path uses `name` alone (one
/// `rwfexist`, same error) and the part after its last '\\' as the texture name. Then `RwStreamOpen(
/// FILESYSTEM, READ, buf)` (NULL -> NULL), `RwD3D9RasterStreamReadDDS(stream)`, `RwStreamClose(stream,
/// NULL)`; no raster -> NULL; `RwTextureCreate(raster)` (NULL -> NULL, raster leaked, kept); the filter
/// mode byte (+0x50) becomes 6 (linear mip linear) for a mipmapped raster (cFormat bit 7), else 2;
/// `RwTextureSetName(texture, name or its base name)`. Original bug not reproduced 1:1: nothing bounds
/// the 256-byte buffer (a longer path entry + name overflows the original's frame into its saved
/// registers and return address; the port's buffer is the same size and overflows its own stack).
#[hook(addr = 0x820A10, conv = cdecl, name = "RwD3D9DDSTextureRead", expect = "81 EC 08 01 00 00",
       unverified = "rw5-image-1", route = "split", hazards = "heap-writes",
       note = "file system probes (rwfexist) and a file stream; creates raster and texture")]
pub fn rw_d3d9_dds_texture_read(name: GPtr<u8>, _mask: GPtr<u8>) -> GPtr<u8> {
    let path = rw_image_get_path();
    let mut buf = [0u8; 0x100];
    let b: GPtr<u8> = GPtr::from_raw(&mut buf).cast();
    let tex_name = if path_is_absolute(name) == 1 || path.is_null() || rd::<u8>(path, 0) == 0 {
        let mut p = a32(name).wrapping_add(rw_strlen(name));
        loop {
            p = p.wrapping_sub(1);
            if p < a32(name) || rd::<u8>(p8(p), 0) == b'\\' {
                break;
            }
        }
        rw_strcpy(b, name);
        rw_strcat(b, p8(0x8E2B24));
        if rw_fexist(b) == 0 {
            return dds_not_found(name);
        }
        p8(p.wrapping_add(1))
    } else {
        let mut entry = path;
        loop {
            if rd::<u8>(entry, 0) == 0 {
                return dds_not_found(name);
            }
            rw_strchr(entry, b';' as u32);
            let sep = rw_strchr(entry, b';' as u32);
            let (len, next) = if sep.is_null() {
                (rw_strlen(entry), GPtr::NULL)
            } else {
                (a32(sep).wrapping_sub(a32(entry)), sep.byte_add(1))
            };
            rep_movs(b, entry, len);
            rw_strcpy(b.byte_add(len as isize), name);
            rw_strcat(b, p8(0x8E2B24));
            if rw_fexist(b) != 0 {
                break;
            }
            if next.is_null() {
                return dds_not_found(name);
            }
            entry = next;
        }
        name
    };
    let s = rw_stream_open(2, 1, b);
    if s.is_null() {
        return GPtr::NULL;
    }
    let raster = rw_d3d9_raster_stream_read_dds(s);
    rw_stream_close(s, GPtr::NULL);
    if raster.is_null() {
        return GPtr::NULL;
    }
    let t = rw_texture_create(raster);
    if t.is_null() {
        return GPtr::NULL;
    }
    wr::<u8>(t, 0x50, if rd::<u8>(raster, 0x23) & 0x80 != 0 { 6 } else { 2 });
    rw_texture_set_name(t, tex_name);
    t
}

/// The not-found tail of `RwD3D9DDSTextureRead`: `RwErrorSet({1, _rwerror(0x16, name, "(null)")})`,
/// NULL.
fn dds_not_found(name: GPtr<u8>) -> GPtr<u8> {
    error_set_code(rw_error3(0x16, a32(name), 0x8E240C));
    GPtr::NULL
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RW_TEXTURE_DEFAULT_READ_CALLBACK,
    HOOK_D3D9_IMAGE_GET_RASTER,
    HOOK_D3D9_RASTER_SET_IMAGE,
    HOOK_RW_IMAGE_APPLY_MASK,
    HOOK_RW_IMAGE_READ,
    HOOK_RW_IMAGE_READ_MASKED_IMAGE,
    HOOK_RW_IMAGE_RESAMPLE,
    HOOK_D3D9_DECOMPRESS_DXT1,
    HOOK_RW_D3D9_DDS_TEXTURE_READ,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
