//! The RW D3D9 driver's raster render and clear standard functions (`stdFunc[0x11..0x13]`,
//! `stdFunc[0xD]`, `stdFunc[0xE]`) and the render dispatcher they share, ported from the 1.0 US
//! bytes (`docs/re/specs/RW/RwD3D9Raster.md` 0x4CAD90, `RwD3D9Raster.p3.md`). The internal helpers
//! take register arguments: they are declared with `orig! { ... = reg("...") }` and the dispatcher
//! is hooked with `conv = reg(...)` (HARNESS-5, `sa_core::regconv`).
//!
//! The render destination is the current raster context `[0xB4E9E4]` (`_rwD3D9SetRasterContext`);
//! the dispatcher reads it through a `.HOODLUM` disguised load (0x4CAD98 -> 0x401EE5, which computes
//! the same address), the port reads it directly.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, p8, rd};
use crate::d3d9cam::dev_clear;
use crate::d3d9dev::{set_depth_stencil_surface, set_render_target_surface};
use crate::image::{d3d9_pixel_to_rgb, d3d9_rgb_to_pixel};
use crate::raster::{d3d9_raster_lock, d3d9_raster_unlock, tex_get_level_desc};
use crate::raster_copy::{level0_surface, release, top_of, Rect, DEPTH_SURFACE, DEVICE, EXT_OFFSET, RENDER_SURFACE};
use sa_core::field::gr32 as u;

/// The current raster context (`RwRaster*`).
const CONTEXT: usize = 0xB4E9E4;

orig! {
    /// `FUN_004caae0` (0x4CAAE0, proposed `_rwD3D9RasterCopy`): EAX = source raster, stack `(rect,
    /// scaled)` (caller pops), result in EAX
    fn raster_copy(src: GPtr<u8>, rect: GPtr<u8>, scaled: u32) -> u32 = reg("args = eax stack stack") 0x4CAAE0 => crate::raster_copy::raster_copy_to_context;
    /// `FUN_004ca5a0` (0x4CA5A0, proposed `_rwD3D9RasterRenderQuad`): ESI = source raster, EBX =
    /// scaled flag, stack `(rect, blend)` (caller pops); its EAX is not used
    fn raster_render_quad(src: GPtr<u8>, scaled: u32, rect: GPtr<u8>, blend: u32) = reg("args = esi ebx stack stack") 0x4CA5A0 => crate::raster_copy::raster_render_quad;
    /// `_rwD3D9RasterRenderGeneric` (0x4CAD90): EAX = source, EDX = rect, ECX = scaled, stack blend
    fn render_generic_call(src: GPtr<u8>, rect: GPtr<u8>, scaled: u32, blend: u32) -> u32 = reg("args = eax edx ecx stack") 0x4CAD90 => raster_render_generic;
    /// `_rwD3D9RasterClearGeneric` (0x4CAEA0): ESI = rect (x, y, w, h), stack colour (caller pops)
    fn clear_generic_call(rect: GPtr<u8>, colour: u32) -> u32 = reg("args = esi stack") 0x4CAEA0 => raster_clear_generic;
    /// `IDirect3DDevice9::ColorFill(surface, rect, colour)` (+0x8C)
    fn dev_color_fill(dev: GPtr<u8>, surf: GPtr<u8>, rect: GPtr<u8>, colour: u32) -> i32 = stdcall vslot 0x8C => unported;
}

/// `_rwD3D9RasterRenderGeneric(src (EAX), rect (EDX), scaled (ECX), blend)` (0x4CAD90; caller pops
/// `blend`): NULL source -> 0. By the destination's (`[0xB4E9E4]`) full type byte: normal / texture
/// (0 / 4) copy a source of type 0, 2, 4, 5 (`0x4CAAE0`, 1 when it returns non-zero, else 0),
/// a camera / camera texture (2 / 5) destination draws a source of type 0, 4, 5 as a textured quad
/// (`0x4CA5A0`, then 1) and copies a camera source; a Z-buffer source gives 0, type 3 and types
/// above 5 give 1. Destinations of type 1, 3 or above 5: 0.
#[hook(addr = 0x4CAD90, conv = reg("args = eax edx ecx stack"), name = "FUN_004cad90", expect = "85 C0 53 8B D9",
       route = "L2", hazards = "none", recs = "boot,menus-hud",
       note = "register convention EAX/EDX/ECX + stack (HARNESS-5); the context is read through a .HOODLUM disguised load at 0x4CAD98")]
pub fn raster_render_generic(src: GPtr<u8>, rect: GPtr<u8>, scaled: u32, blend: u32) -> u32 {
    if src.is_null() {
        return 0;
    }
    let dest = p8(g::<u32>(CONTEXT).read());
    let copy = || (raster_copy(src, rect, scaled) != 0) as u32;
    let src_type = rd::<u8>(src, 0x20);
    match rd::<u8>(dest, 0x20) {
        0 | 4 => match src_type {
            0 | 2 | 4 | 5 => copy(),
            1 => 0,
            _ => 1,
        },
        2 | 5 => match src_type {
            0 | 4 | 5 => {
                raster_render_quad(src, scaled, rect, blend);
                1
            }
            2 => copy(),
            1 => 0,
            _ => 1,
        },
        _ => 0,
    }
}

/// `_rwD3D9RasterRender(raster, rect, flags)` (0x4CAE40, `stdFunc[0x11]`): the dispatcher with
/// scaled 0 and blend 1; `flags` is ignored.
#[hook(addr = 0x4CAE40, conv = cdecl, name = "_rwD3D9RasterRender", expect = "8B 54 24 08 8B 44 24 04 6A 01",
       unverified = "regconv-rw-1", route = "L2", hazards = "none", recs = "boot,menus-hud",
       note = "calls the register-convention dispatcher 0x4CAD90")]
pub fn raster_render(raster: GPtr<u8>, rect: GPtr<u8>, _flags: u32) -> u32 {
    render_generic_call(raster, rect, 0, 1)
}

/// `_rwD3D9RasterRenderFast(raster, rect, flags)` (0x4CAE60, `stdFunc[0x13]`): scaled 0, blend 0.
#[hook(addr = 0x4CAE60, conv = cdecl, name = "_rwD3D9RasterRenderFast", expect = "8B 54 24 08 8B 44 24 04 6A 00",
       route = "L2", hazards = "none", recs = "boot,menus-hud",
       note = "calls the register-convention dispatcher 0x4CAD90")]
pub fn raster_render_fast(raster: GPtr<u8>, rect: GPtr<u8>, _flags: u32) -> u32 {
    render_generic_call(raster, rect, 0, 0)
}

/// `_rwD3D9RasterRenderScaled(raster, rect, flags)` (0x4CAE80, `stdFunc[0x12]`): scaled 1, blend 1.
#[hook(addr = 0x4CAE80, conv = cdecl, name = "_rwD3D9RasterRenderScaled", expect = "8B 54 24 08 8B 44 24 04 6A 01",
       unverified = "regconv-rw-1", route = "L2", hazards = "none", recs = "boot,menus-hud",
       note = "calls the register-convention dispatcher 0x4CAD90")]
pub fn raster_render_scaled(raster: GPtr<u8>, rect: GPtr<u8>, _flags: u32) -> u32 {
    render_generic_call(raster, rect, 1, 1)
}

/// `_rwD3D9RasterClearRect(out, rect, colour)` (0x4CB4C0, `stdFunc[0xD]`): `_rwD3D9RasterClearGeneric`
/// with ESI = `rect`; its result.
#[hook(addr = 0x4CB4C0, conv = cdecl, name = "_rwD3D9RasterClearRect", expect = "8B 44 24 0C 56",
       unverified = "regconv-rw-1", route = "L2", hazards = "heap-writes", recs = "boot,menus-hud",
       note = "calls the register-convention 0x4CAEA0 (ESI = rect)")]
pub fn raster_clear_rect(_out: u32, rect: GPtr<u8>, colour: u32) -> u32 {
    clear_generic_call(rect, colour)
}

/// `_rwD3D9RasterClear(out, inout, colour)` (0x4CB4E0, `stdFunc[0xE]`): a local rect `{0, 0,
/// context width, context height}` cleared through `_rwD3D9RasterClearGeneric` (ESI = the rect).
#[hook(addr = 0x4CB4E0, conv = cdecl, name = "_rwD3D9RasterClear", expect = "83 EC 10 33 C0",
       unverified = "regconv-rw-1", route = "L2", hazards = "heap-writes", recs = "boot,menus-hud",
       note = "calls the register-convention 0x4CAEA0 (ESI = a local rect)")]
pub fn raster_clear(_out: u32, _inout: u32, colour: u32) -> u32 {
    let ctx = p8(g::<u32>(CONTEXT).read());
    let mut rect = [0u32, 0, rd::<u32>(ctx, 0xC), rd::<u32>(ctx, 0x10)];
    clear_generic_call(GPtr::from_raw(rect.as_mut_ptr()).cast(), colour)
}

/// The context raster's colour as an 8888 pixel: `_rwD3D9PixelToRGB(&rgba, &colour, cFormat << 8)`, then
/// `_rwD3D9RGBToPixel(&out, &rgba, 0x500)`. (A format PixelToRGB rejects leaves the original's RGBA local
/// unset, stack residue; the port's is 0. Camera rasters always carry a format.)
fn colour_8888(ctx: GPtr<u8>, colour: u32) -> u32 {
    let (mut c, mut rgba, mut out) = (colour, 0u32, 0u32);
    let format = u32::from(rd::<u8>(ctx, 0x23)) << 8;
    d3d9_pixel_to_rgb(GPtr::from_raw(&mut rgba).cast(), GPtr::from_raw(&mut c), format);
    d3d9_rgb_to_pixel(GPtr::from_raw(&mut out), GPtr::from_raw(&mut rgba).cast(), 0x500);
    out
}

/// The device rectangle of `rect` in the context raster: `{ox + x, oy + y, w + left, h + top}` (ox / oy =
/// the sub-raster offsets +0x1C / +0x1E).
fn device_rect(ctx: GPtr<u8>, rect: GPtr<u8>) -> Rect {
    let left = i32::from(rd::<i16>(ctx, 0x1C)).wrapping_add(rd::<i32>(rect, 0));
    let top = i32::from(rd::<i16>(ctx, 0x1E)).wrapping_add(rd::<i32>(rect, 4));
    Rect { left, top, right: rd::<i32>(rect, 8).wrapping_add(left), bottom: rd::<i32>(rect, 0xC).wrapping_add(top) }
}

/// `IDirect3DDevice9::Clear(1, &rect, flags, colour, 1.0, 0)` on the device.
fn clear_rect(r: &mut Rect, flags: u32, colour: u32) {
    dev_clear(p8(g::<u32>(DEVICE).read()), 1, GPtr::from_raw(r).addr() as u32, flags, colour, 1.0, 0);
}

/// `ColorFill(surface, &rect, colour)` of `rect` in the context raster, then the surface released.
fn fill_surface(ctx: GPtr<u8>, rect: GPtr<u8>, surf: u32, colour: u32) {
    let mut r = device_rect(ctx, rect);
    dev_color_fill(p8(g::<u32>(DEVICE).read()), p8(surf), GPtr::from_raw(&mut r).cast(), colour);
    release(p8(surf));
}

/// `_rwD3D9RasterClearGeneric(rect (ESI), colour)` (0x4CAEA0; caller pops `colour`): clears `rect` (x, y, w, h)
/// of the current context raster `[0xB4E9E4]` by its type byte (+0x20):
/// - camera (2): the colour as 8888 (raw at depth 32, else converted from the raster format), the default render
///   and depth surfaces set (`_rwD3D9SetRenderTarget(0, [0xC97C30])`, `_rwD3D9SetDepthStencilSurface([0xC97C2C])`),
///   `Clear(1, &deviceRect, TARGET, colour, 1.0, 0)`; 1.
/// - Z-buffer (1): the top parent's surface (extension +0) as depth-stencil, `Clear(1, &deviceRect, ZBUFFER, 0,
///   1.0, 0)`, the default depth surface back; 1.
/// - camera texture (5): the colour as for a camera, level 0 of the top parent's texture, `ColorFill`; 1.
/// - normal / texture (0 / 4): the direct parent's (+0, one step) texture `GetLevelDesc(0)`; in the default pool
///   the colour converted and `ColorFill` on level 0 of that texture (1). Otherwise `_rwD3D9RasterLock(&pixels,
///   ctx, 1)` (failure: 0) and the rectangle's rows (stride +0x18) filled by depth (+0x14): 32 / 16 with the raw
///   colour (dword / word), 8 with the palette index nearest to its four bytes (top parent's palette, extension
///   +4; first strict minimum of the summed squared byte differences, 0 stops early); `_rwD3D9RasterUnlock(pixels,
///   ctx, 1)`; 1, or 0 for any other depth.
/// - 3 and above 5: 0.
///
/// The rectangle's fields are re-read on every row as in the original.
#[hook(addr = 0x4CAEA0, conv = reg("args = esi stack"), name = "_rwD3D9RasterClearGeneric", expect = "A1 E4 E9 B4 00",
       unverified = "frontier2-c18", route = "L2", hazards = "heap-writes", recs = "boot,menus-hud",
       note = "register convention ESI + stack (HARNESS-5); D3D9 device / texture / surface calls")]
pub fn raster_clear_generic(rect: GPtr<u8>, colour: u32) -> u32 {
    let ctx = p8(g::<u32>(CONTEXT).read());
    let ext_off = || u(EXT_OFFSET) as i32 as isize;
    match rd::<u8>(ctx, 0x20) {
        2 => {
            let c = if rd::<u32>(ctx, 0x14) != 32 { colour_8888(ctx, colour) } else { colour };
            set_render_target_surface(0, u(RENDER_SURFACE));
            set_depth_stencil_surface(u(DEPTH_SURFACE));
            let mut r = device_rect(p8(g::<u32>(CONTEXT).read()), rect);
            clear_rect(&mut r, 1, c);
            1
        }
        1 => {
            let top = top_of(ctx);
            set_depth_stencil_surface(rd::<u32>(top, ext_off()));
            let mut r = device_rect(p8(g::<u32>(CONTEXT).read()), rect);
            clear_rect(&mut r, 2, 0);
            set_depth_stencil_surface(u(DEPTH_SURFACE));
            1
        }
        5 => {
            let c = if rd::<u32>(ctx, 0x14) != 32 { colour_8888(ctx, colour) } else { colour };
            let top = top_of(p8(g::<u32>(CONTEXT).read()));
            let mut surf = 0u32;
            level0_surface(top.byte_add(ext_off()), &mut surf);
            fill_surface(p8(g::<u32>(CONTEXT).read()), rect, surf, c);
            1
        }
        0 | 4 => {
            let e = p8(rd::<u32>(ctx, 0)).byte_add(ext_off());
            let mut desc = [0u32; 8];
            tex_get_level_desc(p8(rd::<u32>(e, 0)), 0, GPtr::from_raw(desc.as_mut_ptr()).cast());
            if desc[3] == 0 {
                let c = colour_8888(p8(g::<u32>(CONTEXT).read()), colour);
                let mut surf = 0u32;
                level0_surface(e, &mut surf);
                fill_surface(p8(g::<u32>(CONTEXT).read()), rect, surf, c);
                return 1;
            }
            clear_locked(rect, colour)
        }
        _ => 0,
    }
}

/// The locked path of [`raster_clear_generic`] (normal / texture rasters outside the default pool).
fn clear_locked(rect: GPtr<u8>, colour: u32) -> u32 {
    let mut pixels = 0u32;
    if d3d9_raster_lock(GPtr::from_raw(&mut pixels), p8(g::<u32>(CONTEXT).read()), 1) == 0 {
        return 0;
    }
    let ctx = p8(g::<u32>(CONTEXT).read());
    let r = |o: isize| rd::<i32>(rect, o);
    // Every row: `pixels + (y0 + row) * stride + x0 * size`, `w` values.
    let fill = |size: u32, put: &dyn Fn(GPtr<u8>)| {
        let mut row = 0i32;
        while row < r(0xC) {
            let line = (r(4).wrapping_add(row) as u32).wrapping_mul(rd::<u32>(ctx, 0x18)).wrapping_add(pixels);
            let mut p = line.wrapping_add((r(0) as u32).wrapping_mul(size));
            let mut x = 0i32;
            while x < r(8) {
                put(p8(p));
                p = p.wrapping_add(size);
                x += 1;
            }
            row += 1;
        }
    };
    let done = match rd::<u32>(ctx, 0x14) {
        32 => {
            fill(4, &|p| p.cast::<u32>().write(colour));
            true
        }
        16 => {
            fill(2, &|p| p.cast::<u16>().write(colour as u16));
            true
        }
        8 => {
            let index = nearest_palette_index(top_of(ctx), colour);
            fill(1, &|p| p.write(index));
            true
        }
        _ => false,
    };
    d3d9_raster_unlock(pixels, p8(g::<u32>(CONTEXT).read()), 1);
    u32::from(done)
}

/// The palette entry (top raster's extension +4, 256 four-byte entries) nearest to the colour's four bytes: the
/// first strict minimum of the summed squared byte differences (wrapping i32 sums compared unsigned); an exact
/// match ends the search.
fn nearest_palette_index(top: GPtr<u8>, colour: u32) -> u8 {
    let pal = p8(rd::<u32>(top, u(EXT_OFFSET) as i32 as isize + 4));
    let c = colour.to_le_bytes();
    let (mut best, mut index) = (u32::MAX, 0u8);
    for i in 0..256isize {
        let d = (0..4isize).fold(0i32, |s, k| {
            let v = i32::from(rd::<u8>(pal, i * 4 + k)) - i32::from(c[k as usize]);
            s.wrapping_add(v.wrapping_mul(v))
        }) as u32;
        if d < best {
            best = d;
            index = i as u8;
            if d == 0 {
                break;
            }
        }
    }
    index
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_RASTER_RENDER_GENERIC,
    HOOK_RASTER_RENDER,
    HOOK_RASTER_RENDER_FAST,
    HOOK_RASTER_RENDER_SCALED,
    HOOK_RASTER_CLEAR_RECT,
    HOOK_RASTER_CLEAR,
    HOOK_RASTER_CLEAR_GENERIC,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
