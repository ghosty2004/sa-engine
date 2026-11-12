//! The RW D3D9 driver's raster-to-raster copy helper 0x4CAAE0 (batch `mvpn-rw-1`), the one the
//! generic raster render dispatcher `_rwD3D9RasterRenderGeneric` (0x4CAD90) calls for texture-type
//! destinations. Ported from the 1.0 US bytes (no gta-reversed counterpart). Register convention:
//! source raster in EAX, stack `(rect, scaled)`, caller pops. Not hookable in phase A as a
//! `conv = reg(...)` hook: every path reaches an indirect (D3D vtable) call before its `ret`, so
//! sa-hook's `regconv_check` cannot derive the convention; registered `#[native]` for phase B with
//! the register argument modelled as the first cdecl argument (phase B callers are Rust and call it
//! directly). Not verified.
//!
//! Raster fields: +0 parent, +0xC width, +0x10 height, +0x1C / +0x1E sub-raster offset (i16), +0x20
//! type. Extension (raster + `[0xB4E9E0]`): +0 texture / surface, +9 cube (low nibble; face in the
//! high one), +0x1C swap chain.

use sa_core::abi::NativeReg;
use sa_core::mem::GPtr;
use sa_core::{native, orig};

use crate::common::{g, p8, rd};

/// The raster extension offset.
const EXT_OFFSET: usize = 0xB4E9E0;
/// The current context (destination) raster.
const CONTEXT_RASTER: usize = 0xB4E9E4;
/// `IDirect3DDevice9*`.
const DEVICE: usize = 0xC97C28;
/// The default render surface (camera rasters without a swap chain).
const RENDER_SURFACE: usize = 0xC97C30;
/// `D3DCAPS9.StretchRectFilterCaps` (0x1000100: min/mag point, 0x2000200: min/mag linear).
const STRETCH_FILTER_CAPS: usize = 0xC9BFF4;

orig! {
    /// `IUnknown::Release()` (+8)
    fn release(o: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;
    /// `IDirect3DTexture9::GetSurfaceLevel(level, out)` (+0x48)
    fn tex_get_surface_level(tex: GPtr<u8>, level: u32, out: GPtr<u32>) -> i32 = stdcall vslot 0x48 => unported;
    /// `IDirect3DCubeTexture9::GetCubeMapSurface(face, level, out)` (+0x48)
    fn cube_get_cube_map_surface(tex: GPtr<u8>, face: u32, level: u32, out: GPtr<u32>) -> i32 = stdcall vslot 0x48 => unported;
    /// `IDirect3DDevice9::UpdateSurface(src, srcRect, dst, dstPoint)` (+0x78)
    fn dev_update_surface(dev: GPtr<u8>, src: GPtr<u8>, rect: GPtr<u8>, dst: GPtr<u8>, pt: GPtr<u8>) -> i32 = stdcall vslot 0x78 => unported;
    /// `IDirect3DDevice9::GetRenderTargetData(rt, dst)` (+0x80)
    fn dev_get_render_target_data(dev: GPtr<u8>, rt: GPtr<u8>, dst: GPtr<u8>) -> i32 = stdcall vslot 0x80 => unported;
    /// `IDirect3DDevice9::StretchRect(src, srcRect, dst, dstRect, filter)` (+0x88)
    fn dev_stretch_rect(dev: GPtr<u8>, src: GPtr<u8>, src_rect: GPtr<u8>, dst: GPtr<u8>, dst_rect: GPtr<u8>, filter: u32) -> i32 = stdcall vslot 0x88 => unported;
}

/// A `RECT` (or a `POINT` in the first two words) local.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

// SAFETY: four integers.
#[allow(unsafe_code)]
unsafe impl sa_core::mem::GameData for Rect {}

fn u(a: usize) -> u32 {
    g::<u32>(a).read()
}

/// The top parent: follow +0 until it points at itself (the walk starts at the raster itself).
fn top_of(r: GPtr<u8>) -> GPtr<u8> {
    let mut t = r;
    loop {
        let n = p8(rd::<u32>(t, 0));
        if n == t {
            return t;
        }
        t = n;
    }
}

/// The surface of a top raster: a camera (type 2) uses its texture field with a swap chain, else the
/// render surface (no reference taken); any other type takes level 0 of its texture
/// (`GetCubeMapSurface(face, 0)` when the cube nibble is set, else `GetSurfaceLevel(0)`), the result
/// ignored. The extension offset is re-read on every call, as the original does.
fn surface_of(top: GPtr<u8>, out: &mut u32) {
    let e = top.byte_add(u(EXT_OFFSET) as i32 as isize);
    if rd::<u8>(top, 0x20) == 2 {
        *out = if rd::<u32>(e, 0x1C) != 0 { rd::<u32>(e, 0) } else { u(RENDER_SURFACE) };
        return;
    }
    let cube = rd::<u8>(e, 9);
    let tex = p8(rd::<u32>(e, 0));
    if cube & 0xF != 0 {
        cube_get_cube_map_surface(tex, (cube >> 4) as u32, 0, GPtr::from_raw(out));
    } else {
        tex_get_surface_level(tex, 0, GPtr::from_raw(out));
    }
}

/// `FUN_004caae0(src (EAX), rect, scaled)` (0x4CAAE0; caller pops): copies the source raster into the
/// current context raster `[0xB4E9E4]`. Takes the surfaces of both top parents (see `surface_of`), the
/// source rectangle `{ox, oy, ox + width, oy + height}` of the source sub-raster and the destination
/// rectangle `{dx + rect.x, dy + rect.y, ...}` (dx/dy = the context raster's sub-raster offsets; right
/// / bottom = `rect.w + left` / `rect.h + top` when `scaled`, else the source width / height + left /
/// top). By top types (camera = 2 or 5): neither a camera, or only the destination one →
/// `UpdateSurface(src, &srcRect, dst, &point)` with the point = the source's own offset `{ox, oy}` (the
/// original's, not the destination rectangle); a camera source into a non-camera destination →
/// `GetRenderTargetData(src, dst)`; both cameras → `StretchRect(src, &srcRect, dst, &dstRect, filter)`,
/// filter = linear (2) when scaled and the caps allow it, else point (1) when unscaled and allowed, else
/// none (0). Surfaces of non-type-2 top parents are released (destination first). Returns `hr >= 0`.
///
/// The destination rectangle is computed but only `StretchRect` reads it, as in the original.
#[native(addr = 0x4CAAE0, conv = cdecl, name = "FUN_004caae0")]
pub fn raster_copy_to_context(src: GPtr<u8>, rect: GPtr<u8>, scaled: u32) -> u32 {
    let src_top = top_of(src);
    let mut src_surf: u32 = 0;
    surface_of(src_top, &mut src_surf);
    let dst_top = top_of(p8(u(CONTEXT_RASTER)));
    let mut dst_surf: u32 = 0;
    surface_of(dst_top, &mut dst_surf);
    let dst = p8(u(CONTEXT_RASTER));

    let ox = rd::<i16>(src, 0x1C) as i32;
    let oy = rd::<i16>(src, 0x1E) as i32;
    let mut src_rect = Rect {
        left: ox,
        top: oy,
        right: ox.wrapping_add(rd::<i32>(src, 0xC)),
        bottom: oy.wrapping_add(rd::<i32>(src, 0x10)),
    };
    let left = (rd::<i16>(dst, 0x1C) as i32).wrapping_add(rd::<i32>(rect, 0));
    let top = (rd::<i16>(dst, 0x1E) as i32).wrapping_add(rd::<i32>(rect, 4));
    let (right, bottom) = if scaled != 0 {
        (rd::<i32>(rect, 8).wrapping_add(left), rd::<i32>(rect, 0xC).wrapping_add(top))
    } else {
        (rd::<i32>(src, 0xC).wrapping_add(left), rd::<i32>(src, 0x10).wrapping_add(top))
    };
    let mut dst_rect = Rect { left, top, right, bottom };
    let mut point = Rect { left: ox, top: oy, ..Default::default() };

    let camera = |t: GPtr<u8>| matches!(rd::<u8>(t, 0x20), 2 | 5);
    let dev = p8(u(DEVICE));
    let update = |sr: &mut Rect, pt: &mut Rect| {
        dev_update_surface(dev, p8(src_surf), GPtr::from_raw(sr).cast(), p8(dst_surf), GPtr::from_raw(pt).cast())
    };
    let hr = if !camera(dst_top) {
        if !camera(src_top) {
            update(&mut src_rect, &mut point)
        } else {
            dev_get_render_target_data(dev, p8(src_surf), p8(dst_surf))
        }
    } else if !camera(src_top) {
        update(&mut src_rect, &mut point)
    } else {
        let caps = u(STRETCH_FILTER_CAPS);
        let filter = if scaled != 0 {
            if caps & 0x0200_0200 != 0 { 2 } else { 0 }
        } else if caps & 0x0100_0100 != 0 {
            1
        } else {
            0
        };
        dev_stretch_rect(
            dev,
            p8(src_surf),
            GPtr::from_raw(&mut src_rect).cast(),
            p8(dst_surf),
            GPtr::from_raw(&mut dst_rect).cast(),
            filter,
        )
    };
    if rd::<u8>(dst_top, 0x20) != 2 {
        release(p8(dst_surf));
    }
    if rd::<u8>(src_top, 0x20) != 2 {
        release(p8(src_surf));
    }
    (hr >= 0) as u32
}

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_RASTER_COPY_TO_CONTEXT];
