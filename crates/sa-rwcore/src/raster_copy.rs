//! The RW D3D9 driver's raster-to-raster copy helper 0x4CAAE0 (batch `mvpn-rw-1`), the one the
//! generic raster render dispatcher `_rwD3D9RasterRenderGeneric` (0x4CAD90) calls for texture-type
//! destinations, and its textured-quad twin 0x4CA5A0 for camera destinations (FRONTIER-2 lane C, batch
//! `frontier2-c13`, a native for the same reason). Ported from the 1.0 US bytes (no gta-reversed counterpart). Register convention:
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
use crate::d3d9cam::dev_set_viewport;
use crate::d3d9dev::{set_depth_stencil_surface, set_render_target, set_render_target_surface};
use crate::d3d9state::{
    get_render_state, rs_dest_blend, rs_src_blend, rs_vertex_alpha_enable, rw_get_render_state, rw_set_render_state,
    set_render_state, set_texture_internal,
};
use crate::im2d::im2d_render_primitive;
use sa_math::fpu::xi;

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
/// The default depth-stencil surface.
const DEPTH_SURFACE: usize = 0xC97C2C;
/// The quad 0x4CA5A0 draws: four `{x, y, z, rhw, colour, u, v}` vertices (stride 0x1C).
const QUAD: usize = 0xB4E770;

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
    /// `_rwD3D9TestState()` (0x7F8300): the device's cooperative level (resets a lost device); 0 = not renderable
    fn test_state() -> u32 = cdecl 0x7F8300 => unported;
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

use sa_core::field::gr32 as u;

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
        let filter = stretch_filter(scaled);
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

/// The `StretchRect` filter: linear (2) when scaled and the caps allow it, else point (1) when unscaled and allowed, else
/// none (0).
fn stretch_filter(scaled: u32) -> u32 {
    let caps = u(STRETCH_FILTER_CAPS);
    if scaled != 0 {
        if caps & 0x0200_0200 != 0 { 2 } else { 0 }
    } else if caps & 0x0100_0100 != 0 {
        1
    } else {
        0
    }
}

/// One dword of the quad (vertex `v`, byte `off` in it).
fn quad(v: usize, off: usize, bits: u32) {
    g::<u32>(QUAD + v * 0x1C + off).write(bits);
}

/// `fild a; fidiv b; fstp dword`: a texture coordinate.
fn uv(a: i32, b: i32) -> u32 {
    xi(a).fdiv(xi(b)).to_f32().to_bits()
}

/// `FUN_004ca5a0(src (ESI), scaled (EBX), rect, blend)` (0x4CA5A0, proposed `_rwD3D9RasterRenderQuad`; caller pops):
/// draws the source raster into the current context raster `[0xB4E9E4]` as a textured quad. A non-camera context
/// becomes the render target (`RwD3D9SetRenderTarget(0, ctx)`); a camera one, or a failed set, renders into the default
/// surface. Depth-stencil off, the viewport = the context's sub-raster `{x, y, w, h, 0.0, 1.0}`. When
/// `_rwD3D9TestState` passes: the quad at 0xB4E770 (screen corners from `rect`, its width / height when `scaled`, else
/// the source's; z 0, rhw 1, colour -1; u / v = the source's sub-raster corners over its top parent's width / height,
/// x87 `fidiv`), the RW states 0xA / 0xB / 0xC / 0xE / 9 / 3 / 4 / 1 and the D3D states 7 / 0x16 saved, the top
/// parent's texture on stage 0, vertex alpha (plus blend 5 / 6) when `blend` and its alpha byte (ext +8) are set,
/// states 0xE = 0, 9 = scaled ? 2 : 1, 2 = 3, D3D 7 = 0, 0x16 = 1, `_rwD3D9Im2DRenderPrimitive(4, quad, 4)`, then
/// every state restored (the texture through the saved state 1). A camera context ends there. With the render target
/// set, the default surface and depth surface are restored; otherwise the drawn region is copied back: level 0 of the
/// top parent's texture (its cube face when cube), `StretchRect(default surface, context rect, that surface, source
/// rect, filter)`, the surface released.
#[native(addr = 0x4CA5A0, conv = cdecl, name = "FUN_004ca5a0")]
pub fn raster_render_quad(src: GPtr<u8>, scaled: u32, rect: GPtr<u8>, blend: u32) {
    let top = top_of(src);
    let e = top.byte_add(u(EXT_OFFSET) as i32 as isize);
    let mut target = 0;
    let ctx = p8(u(CONTEXT_RASTER));
    if rd::<u8>(ctx, 0x20) != 2 {
        target = set_render_target(0, ctx);
    }
    if target == 0 {
        set_render_target_surface(0, u(RENDER_SURFACE));
    }
    set_depth_stencil_surface(0);
    let ctx = p8(u(CONTEXT_RASTER));
    let mut vp = [
        rd::<i16>(ctx, 0x1C) as i32 as u32,
        rd::<i16>(ctx, 0x1E) as i32 as u32,
        rd::<u32>(ctx, 0xC),
        rd::<u32>(ctx, 0x10),
        0,
        0x3F80_0000,
    ];
    dev_set_viewport(p8(u(DEVICE)), GPtr::from_raw(vp.as_mut_ptr()));
    if test_state() != 0 {
        let r = |o: isize| rd::<i32>(rect, o);
        let f = |v: i32| (v as f32).to_bits();
        let (w, h) = if scaled != 0 { (r(8), r(0xC)) } else { (rd::<i32>(src, 0xC), rd::<i32>(src, 0x10)) };
        quad(0, 0, f(r(0)));
        quad(0, 4, f(r(4)));
        quad(1, 0, f(r(0)));
        quad(1, 4, f(r(4).wrapping_add(h)));
        quad(2, 0, f(r(0).wrapping_add(w)));
        quad(2, 4, f(r(4)));
        quad(3, 0, f(r(0).wrapping_add(w)));
        quad(3, 4, f(h.wrapping_add(r(4))));
        let (ox, oy) = (i32::from(rd::<i16>(src, 0x1C)), i32::from(rd::<i16>(src, 0x1E)));
        let (sw, sh) = (rd::<i32>(src, 0xC), rd::<i32>(src, 0x10));
        let (tw, th) = (rd::<i32>(top, 0xC), rd::<i32>(top, 0x10));
        for v in 0..4 {
            quad(v, 8, 0);
            quad(v, 0xC, 0x3F80_0000);
            quad(v, 0x10, 0xFFFF_FFFF);
            let x = if v >= 2 { ox.wrapping_add(sw) } else { ox };
            let y = if v & 1 == 1 { oy.wrapping_add(sh) } else { oy };
            quad(v, 0x14, uv(x, tw));
            quad(v, 0x18, uv(y, th));
        }
        const RW_STATES: [u32; 8] = [0xA, 0xB, 0xC, 0xE, 9, 3, 4, 1];
        let mut saved = [0u32; 8];
        for (s, out) in RW_STATES.iter().zip(saved.iter_mut()) {
            rw_get_render_state(*s, GPtr::from_raw(out));
        }
        let (mut d7, mut d16) = (0u32, 0u32);
        get_render_state(7, GPtr::from_raw(&mut d7));
        get_render_state(0x16, GPtr::from_raw(&mut d16));
        set_texture_internal(top, 0);
        if blend != 0 && rd::<u8>(e, 8) != 0 {
            rs_vertex_alpha_enable(1);
            rs_src_blend(5);
            rs_dest_blend(6);
        } else {
            rs_vertex_alpha_enable(0);
        }
        rw_set_render_state(0xE, 0);
        rw_set_render_state(9, u32::from(scaled != 0) + 1);
        rw_set_render_state(2, 3);
        set_render_state(7, 0);
        set_render_state(0x16, 1);
        im2d_render_primitive(4, GPtr::from_addr(QUAD), 4);
        set_render_state(7, d7);
        set_render_state(0x16, d16);
        for (s, v) in RW_STATES[..5].iter().zip(saved) {
            rw_set_render_state(*s, v);
        }
        set_texture_internal(p8(saved[7]), 0);
        rw_set_render_state(3, saved[5]);
        rw_set_render_state(4, saved[6]);
    }
    let ctx = p8(u(CONTEXT_RASTER));
    if rd::<u8>(ctx, 0x20) == 2 {
        return;
    }
    if target != 0 {
        set_render_target_surface(0, u(RENDER_SURFACE));
        set_depth_stencil_surface(u(DEPTH_SURFACE));
        return;
    }
    let mut surf: u32 = 0;
    let cube = rd::<u8>(e, 9);
    let tex = p8(rd::<u32>(e, 0));
    if cube & 0xF != 0 {
        cube_get_cube_map_surface(tex, u32::from(cube) >> 4, 0, GPtr::from_raw(&mut surf));
    } else {
        tex_get_surface_level(tex, 0, GPtr::from_raw(&mut surf));
    }
    let ctx = p8(u(CONTEXT_RASTER));
    let (cx, cy) = (i32::from(rd::<i16>(ctx, 0x1C)), i32::from(rd::<i16>(ctx, 0x1E)));
    let mut ctx_rect = Rect { left: cx, top: cy, right: rd::<i32>(ctx, 0xC).wrapping_add(cx), bottom: rd::<i32>(ctx, 0x10).wrapping_add(cy) };
    let (ox, oy) = (i32::from(rd::<i16>(src, 0x1C)), i32::from(rd::<i16>(src, 0x1E)));
    let mut src_rect = Rect { left: ox, top: oy, right: rd::<i32>(src, 0xC).wrapping_add(ox), bottom: rd::<i32>(src, 0x10).wrapping_add(oy) };
    let filter = stretch_filter(scaled);
    dev_stretch_rect(
        p8(u(DEVICE)),
        p8(u(RENDER_SURFACE)),
        GPtr::from_raw(&mut ctx_rect).cast(),
        p8(surf),
        GPtr::from_raw(&mut src_rect).cast(),
        filter,
    );
    release(p8(surf));
}

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_RASTER_COPY_TO_CONTEXT, NATIVE_RASTER_RENDER_QUAD];
