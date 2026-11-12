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

/// The current raster context (`RwRaster*`).
const CONTEXT: usize = 0xB4E9E4;

orig! {
    /// `FUN_004caae0` (0x4CAAE0, proposed `_rwD3D9RasterCopy`): EAX = source raster, stack `(rect,
    /// scaled)` (caller pops), result in EAX
    fn raster_copy(src: GPtr<u8>, rect: GPtr<u8>, scaled: u32) -> u32 = reg("args = eax stack stack") 0x4CAAE0 => unported;
    /// `FUN_004ca5a0` (0x4CA5A0, proposed `_rwD3D9RasterRenderQuad`): ESI = source raster, EBX =
    /// scaled flag, stack `(rect, blend)` (caller pops); its EAX is not used
    fn raster_render_quad(src: GPtr<u8>, scaled: u32, rect: GPtr<u8>, blend: u32) = reg("args = esi ebx stack stack") 0x4CA5A0 => unported;
    /// `_rwD3D9RasterRenderGeneric` (0x4CAD90): EAX = source, EDX = rect, ECX = scaled, stack blend
    fn render_generic_call(src: GPtr<u8>, rect: GPtr<u8>, scaled: u32, blend: u32) -> u32 = reg("args = eax edx ecx stack") 0x4CAD90 => raster_render_generic;
    /// `_rwD3D9RasterClearGeneric` (0x4CAEA0): ESI = rect (x, y, w, h), stack colour (caller pops)
    fn clear_generic_call(rect: GPtr<u8>, colour: u32) -> u32 = reg("args = esi stack") 0x4CAEA0 => unported;
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

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_RASTER_RENDER_GENERIC,
    HOOK_RASTER_RENDER,
    HOOK_RASTER_RENDER_FAST,
    HOOK_RASTER_RENDER_SCALED,
    HOOK_RASTER_CLEAR_RECT,
    HOOK_RASTER_CLEAR,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
