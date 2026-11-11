//! PORT-RW-4 module `raster`: RwRaster and the D3D9 raster driver (_rwD3D9Raster*), texture formats (see docs/arch/rw-port.md, PORT-RW-4).
//! Ported from the 1.0 US bytes; the specs under `docs/re/specs/RW/` are reading aids.
//!
//! RASTERMODULE `M = G + [0xC980D8]` (0x64): +0x00..+0x24 context stack (`stack[0]` = &dummy), +0x28 i32
//! top index, +0x2C dummy raster (0x34 bytes), +0x60 raster freelist. 0xC980DC open count, 0xC980B4
//! static freelist, registry 0x8E2518, freelist parameters 0x8E2530 (per block) / 0x8E2534 (prealloc).
//!
//! `RwRaster` (0x34 + plugins): +0 parent, +4 cpPixels, +8 palette, +0xC width, +0x10 height, +0x14 depth,
//! +0x18 stride, +0x1C i16 nOffsetX, +0x1E i16 nOffsetY, +0x20 cType, +0x21 cFlags, +0x22 privateFlags,
//! +0x23 cFormat, +0x24 originalPixels, +0x28 originalWidth, +0x2C originalHeight, +0x30 originalStride.
//!
//! The `RwGlobals` standard functions (`G + 0x48 + 4*i`, `RwBool (*)(void* out, void* inOut, RwInt32 n)`)
//! are called through `orig!` `table` entries at their byte offsets.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{error1, g, module, p8, rd, rw_globals, rw_memory_alloc, rw_memory_free, wr};

/// RASTERMODULE offset global.
pub const RASTER_MODULE: usize = 0xC980D8;
/// RASTERMODULE open count.
const RASTER_INSTANCES: usize = 0xC980DC;
/// Static raster freelist struct.
const RASTER_FL_STATIC: u32 = 0xC980B4;
/// Raster plugin registry (`sizeOfStruct` at +0).
pub const RASTER_REGISTRY: u32 = 0x8E2518;
/// Freelist parameters in `.data`.
const RASTER_FL_PER_BLOCK: usize = 0x8E2530;
const RASTER_FL_PREALLOC: usize = 0x8E2534;

orig! {
    /// `_rwPluginRegistryGetPluginOffset(reg, id)` (0x808470)
    fn plugin_get_offset(reg: GPtr<u8>, id: u32) -> i32 = cdecl 0x808470 => crate::plugin::registry_get_plugin_offset;
    /// `_rwPluginRegistryAddPlugin(reg, size, id, ctor, dtor, copy)` (0x8084A0)
    fn plugin_add(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x8084A0 => crate::plugin::registry_add_plugin;
    /// `_rwPluginRegistryInitObject(reg, obj)` (0x8086E0)
    fn plugin_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => crate::plugin::registry_init_object;
    /// `_rwPluginRegistryDeInitObject(reg, obj)` (0x808740)
    fn plugin_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => crate::plugin::registry_deinit_object;
    /// `RwFreeListCreateAndPreallocateSpace(entrySize, entriesPerBlock, alignment, numBlocks, space, hint)` (0x801B70)
    fn freelist_create_prealloc(size: u32, per_block: u32, align: u32, n: u32, space: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x801B70 => unported;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    fn rw_free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => unported;
    /// `_rwResourcesPurge()` (0x807E50)
    fn rw_resources_purge() -> u32 = cdecl 0x807E50 => unported;

    /// `stdFunc[4]` RASTERCREATE (G+0x58)
    fn std_raster_create(out: u32, r: GPtr<u8>, flags: u32) -> u32 = cdecl table 0xC97B24 0x58 => unported;
    /// `stdFunc[5]` RASTERDESTROY (G+0x5C)
    fn std_raster_destroy(out: u32, r: GPtr<u8>, n: u32) -> u32 = cdecl table 0xC97B24 0x5C => unported;
    /// `stdFunc[11]` SETRASTERCONTEXT (G+0x74)
    fn std_set_raster_context(out: u32, r: GPtr<u8>, n: u32) -> u32 = cdecl table 0xC97B24 0x74 => unported;
    /// `stdFunc[12]` RASTERSUBRASTER (G+0x78)
    fn std_raster_sub_raster(r: GPtr<u8>, parent: GPtr<u8>, n: u32) -> u32 = cdecl table 0xC97B24 0x78 => unported;
    /// `stdFunc[13]` RASTERCLEARRECT (G+0x7C)
    fn std_raster_clear_rect(out: u32, rect: GPtr<u8>, pixel: u32) -> u32 = cdecl table 0xC97B24 0x7C => unported;
    /// `stdFunc[14]` RASTERCLEAR (G+0x80)
    fn std_raster_clear(out: u32, inout: u32, pixel: u32) -> u32 = cdecl table 0xC97B24 0x80 => unported;
    /// `stdFunc[15]` RASTERLOCK (G+0x84)
    fn std_raster_lock(out: GPtr<u32>, r: GPtr<u8>, flags: u32) -> u32 = cdecl table 0xC97B24 0x84 => unported;
    /// `stdFunc[16]` RASTERUNLOCK (G+0x88)
    fn std_raster_unlock(out: u32, r: GPtr<u8>, n: u32) -> u32 = cdecl table 0xC97B24 0x88 => unported;
    /// `stdFunc[17]` RASTERRENDER (G+0x8C)
    fn std_raster_render(r: GPtr<u8>, pt: GPtr<u8>, n: u32) -> u32 = cdecl table 0xC97B24 0x8C => unported;
    /// `stdFunc[18]` RASTERRENDERSCALED (G+0x90)
    fn std_raster_render_scaled(r: GPtr<u8>, rect: GPtr<u8>, n: u32) -> u32 = cdecl table 0xC97B24 0x90 => unported;
    /// `stdFunc[19]` RASTERRENDERFAST (G+0x94)
    fn std_raster_render_fast(r: GPtr<u8>, pt: GPtr<u8>, n: u32) -> u32 = cdecl table 0xC97B24 0x94 => unported;
    /// `stdFunc[20]` RASTERSHOWRASTER (loaded from G+0x98 before `_rwResourcesPurge`)
    fn call_show_raster(f: u32, r: GPtr<u8>, dev: u32, flags: u32) -> u32 = cdecl dyn => unported;
    /// `stdFunc[23]` RASTERLOCKPALETTE (G+0xA4)
    fn std_raster_lock_palette(out: GPtr<u32>, r: GPtr<u8>, mode: u32) -> u32 = cdecl table 0xC97B24 0xA4 => unported;
    /// `stdFunc[24]` RASTERUNLOCKPALETTE (G+0xA8)
    fn std_raster_unlock_palette(out: u32, r: GPtr<u8>, n: u32) -> u32 = cdecl table 0xC97B24 0xA8 => unported;
    /// `stdFunc[28]` RASTERGETMIPLEVELS (G+0xB8)
    fn std_raster_get_mip_levels(out: GPtr<u32>, r: GPtr<u8>, n: u32) -> u32 = cdecl table 0xC97B24 0xB8 => unported;
}

fn a32<T: sa_core::mem::GameData>(p: GPtr<T>) -> u32 {
    p.addr() as u32
}

/// `M = G + [0xC980D8]`.
fn rmod() -> GPtr<u8> {
    module(RASTER_MODULE)
}

/// `[M + 0x60]`, the raster freelist.
fn raster_free_list() -> GPtr<u8> {
    p8(rd::<u32>(rmod(), 0x60))
}

/// `RwRasterGetPluginOffset(id)` (0x7FAE40).
#[hook(addr = 0x7FAE40, conv = cdecl, name = "RwRasterGetPluginOffset", expect = "8B 44 24 04 50")]
pub fn rw_raster_get_plugin_offset(id: u32) -> i32 {
    plugin_get_offset(p8(RASTER_REGISTRY), id)
}

/// `RwRasterGetCurrentContext()` (0x7FAE60): `M.stack[M.top]`.
#[hook(addr = 0x7FAE60, conv = cdecl, name = "RwRasterGetCurrentContext", expect = "A1 D8 80 C9 00")]
pub fn rw_raster_get_current_context() -> GPtr<u8> {
    let m = rmod();
    let top = rd::<i32>(m, 0x28);
    p8(rd::<u32>(m, (top as isize).wrapping_mul(4)))
}

/// `RwRasterRenderScaled(raster, rect)` (0x7FAE80): `stdFunc[18](raster, rect, 0)`, result ignored.
#[hook(addr = 0x7FAE80, conv = cdecl, name = "RwRasterRenderScaled", expect = "8B 44 24 08 8B 0D 24 7B C9 00")]
pub fn rw_raster_render_scaled(r: GPtr<u8>, rect: GPtr<u8>) -> GPtr<u8> {
    std_raster_render_scaled(r, rect, 0);
    r
}

/// `RwRasterGetOffset(raster, &x, &y)` (0x7FAEA0): the two i16 offsets (EAX = raster, unused).
#[hook(addr = 0x7FAEA0, conv = cdecl, name = "RwRasterGetOffset", expect = "8B 44 24 04 8B 54 24 08")]
pub fn rw_raster_get_offset(r: GPtr<u8>, x: GPtr<i16>, y: GPtr<i16>) {
    x.write(rd::<i16>(r, 0x1C));
    y.write(rd::<i16>(r, 0x1E));
}

/// `RwRasterUnlock(raster)` (0x7FAEC0): `stdFunc[16](NULL, raster, 0)`, result ignored.
#[hook(addr = 0x7FAEC0, conv = cdecl, name = "RwRasterUnlock", expect = "A1 24 7B C9 00",
       unverified = "rw5-raster", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn rw_raster_unlock(r: GPtr<u8>) -> GPtr<u8> {
    std_raster_unlock(0, r, 0);
    r
}

/// `RwRasterClear(pixel)` (0x7FAEE0): only with a raster context (`M.top > 0`, signed); otherwise
/// error 0x15. Returns the `top > 0` flag.
#[hook(addr = 0x7FAEE0, conv = cdecl, name = "RwRasterClear", expect = "A1 24 7B C9 00")]
pub fn rw_raster_clear(pixel: u32) -> u32 {
    let ok = (rd::<i32>(rmod(), 0x28) > 0) as u32;
    if ok != 0 {
        std_raster_clear(0, 0, pixel);
    } else {
        error1(0x15);
    }
    ok
}

/// The `{x, y}` local the render wrappers pass (the original's local rect: width/height not written,
/// the port leaves them 0).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RectLocal {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

// SAFETY: four integers.
#[allow(unsafe_code)]
unsafe impl sa_core::mem::GameData for RectLocal {}

/// `RwRasterRenderFast(raster, x, y)` (0x7FAF50): `stdFunc[19](raster, &{x, y}, 0)`, result ignored.
#[hook(addr = 0x7FAF50, conv = cdecl, name = "RwRasterRenderFast", expect = "83 EC 10 8B 44 24 18")]
pub fn rw_raster_render_fast(r: GPtr<u8>, x: i32, y: i32) -> GPtr<u8> {
    let mut pt = RectLocal { x, y, ..Default::default() };
    std_raster_render_fast(r, GPtr::from_raw(&mut pt).cast(), 0);
    r
}

/// `RwRasterClearRect(rect, pixel)` (0x7FAF90): `stdFunc[13](NULL, rect, pixel)`; returns 1.
#[hook(addr = 0x7FAF90, conv = cdecl, name = "RwRasterClearRect", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn rw_raster_clear_rect(rect: GPtr<u8>, pixel: u32) -> u32 {
    std_raster_clear_rect(0, rect, pixel);
    1
}

/// `RwRasterRender(raster, x, y)` (0x7FAFB0): `stdFunc[17](raster, &{x, y}, 0)`, result ignored.
#[hook(addr = 0x7FAFB0, conv = cdecl, name = "RwRasterRender", expect = "83 EC 10 8B 44 24 18")]
pub fn rw_raster_render(r: GPtr<u8>, x: i32, y: i32) -> GPtr<u8> {
    let mut pt = RectLocal { x, y, ..Default::default() };
    std_raster_render(r, GPtr::from_raw(&mut pt).cast(), 0);
    r
}

/// `RwRasterUnlockPalette(raster)` (0x7FAFF0): `stdFunc[24](NULL, raster, 0)`, then
/// `privateFlags &= 0xE7`.
#[hook(addr = 0x7FAFF0, conv = cdecl, name = "RwRasterUnlockPalette", expect = "A1 24 7B C9 00")]
pub fn rw_raster_unlock_palette(r: GPtr<u8>) -> GPtr<u8> {
    std_raster_unlock_palette(0, r, 0);
    wr::<u8>(r, 0x22, rd::<u8>(r, 0x22) & 0xE7);
    r
}

/// `RwRasterDestroy(raster)` (0x7FB020): plugin destructors, `stdFunc[5](NULL, raster, 0)`, freelist
/// free; returns 1.
#[hook(addr = 0x7FB020, conv = cdecl, name = "RwRasterDestroy", expect = "56 8B 74 24 08")]
pub fn rw_raster_destroy(r: GPtr<u8>) -> u32 {
    plugin_deinit_object(p8(RASTER_REGISTRY), r);
    std_raster_destroy(0, r, 0);
    rw_memory_free(raster_free_list(), r);
    1
}

/// `RwRasterPushContext(raster)` (0x7FB060): `stdFunc[11](NULL, raster, 0)`; on success
/// `stack[++top] = raster` (no overflow check).
#[hook(addr = 0x7FB060, conv = cdecl, name = "RwRasterPushContext", expect = "A1 24 7B C9 00")]
pub fn rw_raster_push_context(r: GPtr<u8>) -> GPtr<u8> {
    if std_set_raster_context(0, r, 0) == 0 {
        return GPtr::NULL;
    }
    let m = rmod();
    wr::<i32>(m, 0x28, rd::<i32>(m, 0x28).wrapping_add(1));
    let m = rmod();
    let top = rd::<i32>(m, 0x28);
    wr::<u32>(m, (top as isize).wrapping_mul(4), a32(r));
    r
}

/// `RwRasterRegisterPlugin(size, id, ctor, dtor, copy)` (0x7FB0B0).
#[hook(addr = 0x7FB0B0, conv = cdecl, name = "RwRasterRegisterPlugin", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn rw_raster_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 {
    plugin_add(p8(RASTER_REGISTRY), size, id, ctor, dtor, copy)
}

/// `RwRasterLockPalette(raster, mode)` (0x7FB0E0): `stdFunc[23](&out, raster, mode)`; the original's
/// `out` is its own `raster` argument slot (overwritten with the palette pointer). Returns out or NULL.
#[hook(addr = 0x7FB0E0, conv = cdecl, name = "RwRasterLockPalette", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn rw_raster_lock_palette(r: GPtr<u8>, mode: u32) -> u32 {
    let mut out = a32(r);
    let ok = std_raster_lock_palette(GPtr::from_raw(&mut out), r, mode);
    if ok != 0 { out } else { 0 }
}

/// `RwRasterPopContext()` (0x7FB110): `top--` (even when the device call fails, no underflow check),
/// `stdFunc[11](NULL, stack[top - 1], 0)`; returns the popped raster or NULL.
#[hook(addr = 0x7FB110, conv = cdecl, name = "RwRasterPopContext", expect = "8B 0D D8 80 C9 00")]
pub fn rw_raster_pop_context() -> u32 {
    let m = rmod();
    let old = rd::<i32>(m, 0x28);
    wr::<i32>(m, 0x28, old.wrapping_sub(1));
    let m = rmod();
    let popped = rd::<u32>(m, (old as isize).wrapping_mul(4));
    let new = rd::<i32>(m, 0x28);
    let next = rd::<u32>(m, (new as isize).wrapping_mul(4));
    let ok = std_set_raster_context(0, p8(next), 0);
    if ok != 0 { popped } else { 0 }
}

/// `RwRasterGetNumLevels(raster)` (0x7FB160): 1 unless the cFormat mipmap bit (0x80) is set; then
/// `stdFunc[28](&out, raster, 0)` (out = the original's argument slot) gives out, or -1 on failure.
#[hook(addr = 0x7FB160, conv = cdecl, name = "RwRasterGetNumLevels", expect = "8B 4C 24 04 8A 41 23",
       unverified = "rw5-raster", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn rw_raster_get_num_levels(r: GPtr<u8>) -> i32 {
    if (rd::<u8>(r, 0x23) as i8) >= 0 {
        return 1;
    }
    let mut out = a32(r);
    if std_raster_get_mip_levels(GPtr::from_raw(&mut out), r, 0) != 0 {
        out as i32
    } else {
        -1
    }
}

/// `RwRasterShowRaster(raster, dev, flags)` (0x7FB1A0): loads `stdFunc[20]` first, purges the
/// resources arena, then calls it; returns raster or NULL.
#[hook(addr = 0x7FB1A0, conv = cdecl, name = "RwRasterShowRaster", expect = "A1 24 7B C9 00")]
pub fn rw_raster_show_raster(r: GPtr<u8>, dev: u32, flags: u32) -> GPtr<u8> {
    let f = rd::<u32>(rw_globals(), 0x98);
    rw_resources_purge();
    if call_show_raster(f, r, dev, flags) != 0 { r } else { GPtr::NULL }
}

/// `RwRasterSubRaster(raster, parent, rect)` (0x7FB1D0): only for rasters with cFlags bit 0x80
/// (DONTALLOCATE). width/height from rect, offsets = parent offsets + rect x/y (16-bit adds), then
/// `stdFunc[12](raster, parent, 0)`; on success `raster.parent = parent.parent`.
#[hook(addr = 0x7FB1D0, conv = cdecl, name = "RwRasterSubRaster", expect = "56 8B 74 24 08")]
pub fn rw_raster_sub_raster(r: GPtr<u8>, parent: GPtr<u8>, rect: GPtr<u8>) -> GPtr<u8> {
    if (rd::<u8>(r, 0x21) as i8) >= 0 {
        return GPtr::NULL;
    }
    wr::<u32>(r, 0xC, rd::<u32>(rect, 8));
    wr::<u32>(r, 0x10, rd::<u32>(rect, 0xC));
    wr::<u16>(r, 0x1C, rd::<u16>(parent, 0x1C).wrapping_add(rd::<u16>(rect, 0)));
    wr::<u16>(r, 0x1E, rd::<u16>(parent, 0x1E).wrapping_add(rd::<u16>(rect, 4)));
    if std_raster_sub_raster(r, parent, 0) == 0 {
        return GPtr::NULL;
    }
    wr::<u32>(r, 0, rd::<u32>(parent, 0));
    r
}

/// `RwRasterCreate(width, height, depth, flags)` (0x7FB230): freelist alloc (hint 0x30407), header
/// stores in the original order, `stdFunc[4](NULL, raster, flags)`; failure frees the raster and returns
/// NULL; success runs the plugin constructors.
#[hook(addr = 0x7FB230, conv = cdecl, name = "RwRasterCreate", expect = "A1 24 7B C9 00",
       unverified = "rw5-raster", route = "split", hazards = "heap-writes", recs = "grove-walk")]
pub fn rw_raster_create(width: i32, height: i32, depth: i32, flags: u32) -> GPtr<u8> {
    let r = rw_memory_alloc(raster_free_list(), 0x30407);
    if r.is_null() {
        return GPtr::NULL;
    }
    wr::<i32>(r, 0xC, width);
    wr::<u8>(r, 0x22, 0);
    wr::<i32>(r, 0x10, height);
    wr::<i32>(r, 0x14, depth);
    wr::<u8>(r, 0x21, 0);
    wr::<u16>(r, 0x1C, 0);
    wr::<u16>(r, 0x1E, 0);
    wr::<u32>(r, 0, a32(r));
    wr::<u32>(r, 4, 0);
    wr::<u32>(r, 8, 0);
    if std_raster_create(0, r, flags) == 0 {
        rw_memory_free(raster_free_list(), r);
        return GPtr::NULL;
    }
    plugin_init_object(p8(RASTER_REGISTRY), r);
    r
}

/// `RwRasterLock(raster, level, mode)` (0x7FB2D0): `stdFunc[15](&out, raster, ((level & 0xFF) << 8) +
/// mode)` (out = the original's `raster` argument slot); returns out (the pixels) or NULL.
#[hook(addr = 0x7FB2D0, conv = cdecl, name = "RwRasterLock", expect = "8B 44 24 08 8B 54 24 0C",
       unverified = "rw5-raster", route = "split", hazards = "heap-writes", recs = "grove-walk")]
pub fn rw_raster_lock(r: GPtr<u8>, level: u32, mode: u32) -> u32 {
    let flags = ((level & 0xFF) << 8).wrapping_add(mode);
    let mut out = a32(r);
    let ok = std_raster_lock(GPtr::from_raw(&mut out), r, flags);
    if ok != 0 { out } else { 0 }
}

/// `cb_RASTERMODULE_RwEngine_Close(instance, offset, size)` (0x7FB310): destroys the freelist when
/// set and clears the pointer; decrements the open count; returns the instance.
#[hook(addr = 0x7FB310, conv = cdecl, name = "cb_RASTERMODULE_RwEngine_Close", expect = "A1 24 7B C9 00")]
pub fn raster_module_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let fl = raster_free_list();
    if !fl.is_null() {
        rw_free_list_destroy(fl);
        wr::<u32>(rmod(), 0x60, 0);
    }
    let c = g::<u32>(RASTER_INSTANCES);
    c.write(c.read().wrapping_sub(1));
    instance
}

/// `RwRasterSetFreeListCreateParams(blockSize, numBlocksToPrealloc)` (0x7FB350).
#[hook(addr = 0x7FB350, conv = cdecl, name = "RwRasterSetFreeListCreateParams", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn rw_raster_set_free_list_create_params(per_block: u32, prealloc: u32) {
    g::<u32>(RASTER_FL_PER_BLOCK).write(per_block);
    g::<u32>(RASTER_FL_PREALLOC).write(prealloc);
}

/// `cb_RASTERMODULE_RwEngine_Open(instance, offset, size)` (0x7FB370): stores the offset, zeroes the
/// dummy raster (13 dwords at M+0x2C), sets its fields in the original order (width, height, depth = 0,
/// cFlags = 0x80, parent, cpPixels = 0, cType = 0), top = 0, `stack[0] = &dummy`, then creates the
/// freelist (entry size `[0x8E2518]`, alignment 4, static 0xC980B4, hint 0x40407). NULL -> NULL.
#[hook(addr = 0x7FB370, conv = cdecl, name = "cb_RASTERMODULE_RwEngine_Open", expect = "8B 54 24 08 53")]
pub fn raster_module_open(instance: GPtr<u8>, offset: u32, _size: u32) -> GPtr<u8> {
    g::<u32>(RASTER_MODULE).write(offset);
    let m = rw_globals().byte_add(offset as i32 as isize);
    for i in 0..13 {
        wr::<u32>(m, 0x2C + i * 4, 0);
    }
    wr::<u32>(rmod(), 0x38, 0);
    wr::<u32>(rmod(), 0x3C, 0);
    wr::<u32>(rmod(), 0x40, 0);
    wr::<u8>(rmod(), 0x4D, 0x80);
    wr::<u32>(rmod(), 0x30, 0);
    wr::<u32>(rmod(), 0x34, 0);
    wr::<u8>(rmod(), 0x4C, 0);
    wr::<u32>(rmod(), 0x28, 0);
    let m = rmod();
    wr::<u32>(m, 0, a32(m.byte_add(0x2C)));
    let fl = freelist_create_prealloc(
        g::<u32>(RASTER_REGISTRY as usize).read(),
        g::<u32>(RASTER_FL_PER_BLOCK).read(),
        4,
        g::<u32>(RASTER_FL_PREALLOC).read(),
        p8(RASTER_FL_STATIC),
        0x40407,
    );
    wr::<u32>(rmod(), 0x60, a32(fl));
    if rd::<u32>(rmod(), 0x60) == 0 {
        return GPtr::NULL;
    }
    let c = g::<u32>(RASTER_INSTANCES);
    c.write(c.read().wrapping_add(1));
    instance
}

// ---------------------------------------------------------------------------------------------
// D3D9 raster driver (0x4C9A60..0x4CD820)
//
// Extension `E = raster + [0xB4E9E0]` (plugin 0x40C, 0x24 bytes): +0 texture/surface, +4 palette
// (+0x400 palette index), +8 u8 alpha, +9 u8 cube:4/face:4, +0xA u8 autoMipmapGen:4/compressed:4,
// +0xB u8 lockedMipLevel, +0xC lockedSurface, +0x10 D3DLOCKED_RECT {Pitch, pBits}, +0x18 D3DFORMAT,
// +0x1C swapChain, +0x20 window.

/// `_RwD3D9RasterExtOffset`.
pub const D3D9_RASTER_EXT_OFFSET: usize = 0xB4E9E0;
/// Current raster context (`_rwD3D9SetRasterContext`).
const D3D9_RASTER_CONTEXT: usize = 0xB4E9E4;
/// The D3DFORMAT info table `{u8 alpha, u8 depth, u16 rwFormat}[128]`.
const D3D9_PIXEL_FORMAT_INFO: usize = 0xB4E7E0;
/// Driver raster globals cleared by `_rwD3D9RasterClose`.
const D3D9_RASTER_GLOBALS: usize = 0xB4E9E8;
/// Buffer freed by `_rwD3D9RasterClose` (`rwfree`).
const D3D9_RASTER_BUFFER: usize = 0xB4E9F4;
/// Palette freelist.
const D3D9_PALETTE_FREELIST: usize = 0xB4E9F8;
/// Video-memory raster list head and its node freelist.
const VM_RASTER_LIST: usize = 0xB4E9FC;
const VM_RASTER_FREELIST: usize = 0xB4EA00;
/// `_RwDirect3DObject` (IDirect3D9*), `_RwD3DAdapterIndex`, `_RwD3DAdapterType`.
const D3D_OBJECT: usize = 0xC97C20;
const D3D_ADAPTER_INDEX: usize = 0xC97C24;
const D3D_ADAPTER_TYPE: usize = 0x8E2428;
/// `_RwD3DDevice` (IDirect3DDevice9*).
const D3D_DEVICE: usize = 0xC97C28;
/// Default depth-stencil surface and render surface.
const D3D_DEPTH_SURFACE: usize = 0xC97C2C;
const D3D_RENDER_SURFACE: usize = 0xC97C30;
/// Current display-mode format and depth-buffer format.
const D3D_DISPLAY_FORMAT: usize = 0xC9BEF0;
const D3D_DEPTH_FORMAT: usize = 0xC9C068;
/// D3DCAPS9.Caps2 (bit 0x40000000 = CANAUTOGENMIPMAP).
const D3D_CAPS2: usize = 0xC9BF0C;

orig! {
    /// `RwTextureRasterGenerateMipmaps(raster, image)` (0x7F3CD0)
    fn rw_texture_raster_generate_mipmaps(r: GPtr<u8>, image: u32) -> u32 = cdecl 0x7F3CD0 => rw_texture_raster_generate_mipmaps_port;
    /// `RwFreeListCreate(entrySize, entriesPerBlock, alignment, hint)` (0x801980)
    fn rw_free_list_create(size: u32, per_block: u32, align: u32, hint: u32) -> GPtr<u8> = cdecl 0x801980 => unported;
    /// `_rxD3D9VideoMemoryRasterListDestroy()` (0x4CB5E0)
    fn vm_raster_list_destroy_call() = cdecl 0x4CB5E0 => vm_raster_list_destroy;
    /// `G->rwfree(p)` (+0x138) as `_rwD3D9RasterClose` calls it
    fn rw_free_call(p: u32) = cdecl table 0xC97B24 0x138 => unported;
    /// `RwRasterRegisterPlugin(size, id, ctor, dtor, copy)` (0x7FB0B0)
    fn rw_raster_register_plugin_call(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x7FB0B0 => rw_raster_register_plugin;

    /// `IUnknown::Release` (vtable +8)
    pub fn com_release(obj: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;
    /// `IDirect3D9::CheckDeviceFormat(adapter, devType, adapterFmt, usage, rType, checkFmt)` (+0x28)
    fn d3d_check_device_format(d3d: GPtr<u8>, adapter: u32, dev_type: u32, adapter_fmt: u32, usage: u32, rtype: u32, fmt: u32) -> i32 = stdcall vslot 0x28 => unported;
    /// `IDirect3D9::CheckDepthStencilMatch(adapter, devType, adapterFmt, rtFmt, dsFmt)` (+0x30)
    fn d3d_check_depth_stencil_match(d3d: GPtr<u8>, adapter: u32, dev_type: u32, adapter_fmt: u32, rt_fmt: u32, ds_fmt: u32) -> i32 = stdcall vslot 0x30 => unported;
    /// `IDirect3DTexture9::GetSurfaceLevel(level, out)` (+0x48)
    fn tex_get_surface_level(tex: GPtr<u8>, level: u32, out: GPtr<u32>) -> i32 = stdcall vslot 0x48 => unported;
    /// `IDirect3DCubeTexture9::GetCubeMapSurface(face, level, out)` (+0x48)
    fn cube_get_cube_map_surface(tex: GPtr<u8>, face: u32, level: u32, out: GPtr<u32>) -> i32 = stdcall vslot 0x48 => unported;
    /// `IDirect3DBaseTexture9::GetLevelCount()` (+0x34)
    fn tex_get_level_count(tex: GPtr<u8>) -> u32 = stdcall vslot 0x34 => unported;
    /// `IDirect3DSurface9::GetDesc(desc)` (+0x30)
    fn surf_get_desc(surf: GPtr<u8>, desc: GPtr<u8>) -> i32 = stdcall vslot 0x30 => unported;
    /// `IDirect3DSurface9::LockRect(lockedRect, rect, flags)` (+0x34)
    fn surf_lock_rect(surf: GPtr<u8>, locked: GPtr<u8>, rect: GPtr<u8>, flags: u32) -> i32 = stdcall vslot 0x34 => unported;
    /// `IDirect3DSurface9::UnlockRect()` (+0x38)
    fn surf_unlock_rect(surf: GPtr<u8>) -> i32 = stdcall vslot 0x38 => unported;
    /// `IDirect3DDevice9::UpdateSurface(src, srcRect, dst, dstPoint)` (+0x78)
    fn dev_update_surface(dev: GPtr<u8>, src: GPtr<u8>, rect: GPtr<u8>, dst: GPtr<u8>, pt: GPtr<u8>) -> i32 = stdcall vslot 0x78 => unported;
    /// `IDirect3DDevice9::GetRenderTargetData(rt, dst)` (+0x80)
    fn dev_get_render_target_data(dev: GPtr<u8>, rt: GPtr<u8>, dst: GPtr<u8>) -> i32 = stdcall vslot 0x80 => unported;
    /// `IDirect3DDevice9::CreateOffscreenPlainSurface(w, h, fmt, pool, out, shared)` (+0x90)
    fn dev_create_offscreen_plain_surface(dev: GPtr<u8>, w: u32, h: u32, fmt: u32, pool: u32, out: GPtr<u8>, shared: u32) -> i32 = stdcall vslot 0x90 => unported;
    /// `IDirect3DDevice9::SetPaletteEntries(index, entries)` (+0x11C)
    fn dev_set_palette_entries(dev: GPtr<u8>, index: u32, entries: GPtr<u8>) -> i32 = stdcall vslot 0x11C => unported;
}

/// The extension of a raster.
fn ext(r: GPtr<u8>) -> GPtr<u8> {
    r.byte_add(g::<i32>(D3D9_RASTER_EXT_OFFSET).read() as isize)
}

/// The topmost parent (`t = r->parent; while (t != t->parent) t = t->parent`).
fn top_parent(r: GPtr<u8>) -> GPtr<u8> {
    let mut t = p8(rd::<u32>(r, 0));
    loop {
        let n = p8(rd::<u32>(t, 0));
        if n == t {
            return t;
        }
        t = n;
    }
}

fn device() -> GPtr<u8> {
    p8(g::<u32>(D3D_DEVICE).read())
}

fn d3d() -> GPtr<u8> {
    p8(g::<u32>(D3D_OBJECT).read())
}

/// `cb_DEVICEMODULE_RwRaster_Ctor(raster, offset, size)` (0x4C9A60): `E.swapChain = NULL`.
#[hook(addr = 0x4C9A60, conv = cdecl, name = "cb_DEVICEMODULE_RwRaster_Ctor", expect = "8B 44 24 04 8B 0D E0 E9 B4 00")]
pub fn d3d9_raster_ctor(r: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    wr::<u32>(ext(r), 0x1C, 0);
    r
}

/// `cb_DEVICEMODULE_RwRaster_Dtor(raster, offset, size)` (0x4C9A80): releases and clears the swap chain.
#[hook(addr = 0x4C9A80, conv = cdecl, name = "cb_DEVICEMODULE_RwRaster_Dtor", expect = "A1 E0 E9 B4 00")]
pub fn d3d9_raster_dtor(r: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let e = ext(r);
    let sc = p8(rd::<u32>(e, 0x1C));
    if !sc.is_null() {
        com_release(sc);
        wr::<u32>(e, 0x1C, 0);
    }
    r
}

/// The stores `_rwD3D9RasterPluginAttach` makes into the D3DFORMAT info table after zeroing it
/// (address, size, value), in the original order (0x4C9AF3..0x4C9E6F).
const PIXEL_FORMAT_STORES: [(u32, u8, u16); 138] = [
    (0xB4E830, 1, 0x0), (0xB4E831, 1, 0x18), (0xB4E832, 2, 0x0), (0xB4E834, 1, 0x1), (0xB4E835, 1, 0x20),
    (0xB4E836, 2, 0x500), (0xB4E838, 1, 0x0), (0xB4E839, 1, 0x20), (0xB4E83A, 2, 0x600), (0xB4E83C, 1, 0x0),
    (0xB4E83D, 1, 0x10), (0xB4E83E, 2, 0x200), (0xB4E840, 1, 0x0), (0xB4E841, 1, 0x10), (0xB4E842, 2, 0xA00),
    (0xB4E844, 1, 0x1), (0xB4E845, 1, 0x10), (0xB4E846, 2, 0x100), (0xB4E848, 1, 0x1), (0xB4E849, 1, 0x10),
    (0xB4E84A, 2, 0x300), (0xB4E84C, 1, 0x0), (0xB4E84D, 1, 0x8), (0xB4E84E, 2, 0x0), (0xB4E850, 1, 0x1),
    (0xB4E851, 1, 0x8), (0xB4E852, 2, 0x0), (0xB4E854, 1, 0x1), (0xB4E855, 1, 0x10), (0xB4E856, 2, 0x0),
    (0xB4E858, 1, 0x0), (0xB4E859, 1, 0x10), (0xB4E85A, 2, 0x0), (0xB4E85C, 1, 0x1), (0xB4E85D, 1, 0x20),
    (0xB4E85E, 2, 0x0), (0xB4E860, 1, 0x1), (0xB4E861, 1, 0x20), (0xB4E862, 2, 0x0), (0xB4E864, 1, 0x0),
    (0xB4E865, 1, 0x20), (0xB4E866, 2, 0x0), (0xB4E868, 1, 0x0), (0xB4E869, 1, 0x20), (0xB4E86A, 2, 0x0),
    (0xB4E86C, 1, 0x1), (0xB4E86D, 1, 0x20), (0xB4E86E, 2, 0x0), (0xB4E870, 1, 0x1), (0xB4E871, 1, 0x40),
    (0xB4E872, 2, 0x0), (0xB4E880, 1, 0x1), (0xB4E881, 1, 0x10), (0xB4E882, 2, 0x0), (0xB4E884, 1, 0x0),
    (0xB4E885, 1, 0x8), (0xB4E886, 2, 0x2000), (0xB4E8A8, 1, 0x0), (0xB4E8A9, 1, 0x8), (0xB4E8AA, 2, 0x400),
    (0xB4E8AC, 1, 0x1), (0xB4E8AD, 1, 0x10), (0xB4E8AE, 2, 0x0), (0xB4E8B0, 1, 0x1), (0xB4E8B1, 1, 0x8),
    (0xB4E8B2, 2, 0x0), (0xB4E8D0, 1, 0x0), (0xB4E8D1, 1, 0x10), (0xB4E8D2, 2, 0x0), (0xB4E8D4, 1, 0x0),
    (0xB4E8D5, 1, 0x10), (0xB4E8D6, 2, 0x0), (0xB4E8FA, 2, 0x700), (0xB4E906, 2, 0x700), (0xB4E922, 2, 0x700),
    (0xB4E8FE, 2, 0x900), (0xB4E90E, 2, 0x900), (0xB4E916, 2, 0x900), (0xB4E91E, 2, 0x900), (0xB4E92A, 2, 0x900),
    (0xB4E92E, 2, 0x900), (0xB4E8D8, 1, 0x0), (0xB4E8DA, 2, 0x0), (0xB4E8DC, 1, 0x0), (0xB4E8DE, 2, 0x0),
    (0xB4E8E0, 1, 0x0), (0xB4E8E2, 2, 0x0), (0xB4E8EE, 2, 0x0), (0xB4E8F8, 1, 0x0), (0xB4E8FC, 1, 0x0),
    (0xB4E904, 1, 0x0), (0xB4E90C, 1, 0x0), (0xB4E914, 1, 0x0), (0xB4E91C, 1, 0x0), (0xB4E920, 1, 0x0),
    (0xB4E928, 1, 0x0), (0xB4E92C, 1, 0x0), (0xB4E924, 1, 0x0), (0xB4E926, 2, 0x0), (0xB4E998, 1, 0x0),
    (0xB4E99A, 2, 0x0), (0xB4E99C, 1, 0x0), (0xB4E99E, 2, 0x0), (0xB4E9A0, 1, 0x0), (0xB4E9A2, 2, 0x0),
    (0xB4E9A6, 2, 0x0), (0xB4E9A8, 1, 0x0), (0xB4E9AA, 2, 0x0), (0xB4E9AC, 1, 0x0), (0xB4E9AE, 2, 0x0),
    (0xB4E9B2, 2, 0x0), (0xB4E9B4, 1, 0x0), (0xB4E9B6, 2, 0x0), (0xB4E8D9, 1, 0x20), (0xB4E8DD, 1, 0x20),
    (0xB4E8E1, 1, 0x20), (0xB4E8EC, 1, 0x1), (0xB4E8ED, 1, 0x20), (0xB4E8F9, 1, 0x10), (0xB4E8FD, 1, 0x20),
    (0xB4E905, 1, 0x10), (0xB4E90D, 1, 0x20), (0xB4E915, 1, 0x20), (0xB4E91D, 1, 0x20), (0xB4E921, 1, 0x10),
    (0xB4E929, 1, 0x20), (0xB4E92D, 1, 0x20), (0xB4E925, 1, 0x10), (0xB4E999, 1, 0x40), (0xB4E99D, 1, 0x10),
    (0xB4E9A1, 1, 0x20), (0xB4E9A4, 1, 0x1), (0xB4E9A5, 1, 0x40), (0xB4E9A9, 1, 0x20), (0xB4E9AD, 1, 0x40),
    (0xB4E9B0, 1, 0x1), (0xB4E9B1, 1, 0x80), (0xB4E9B5, 1, 0x10),
];

/// `_rwD3D9RasterPluginAttach()` (0x4C9AB0): registers the 0x24-byte extension (id 0x40C), stores the
/// offset (before the sign test), fills the D3DFORMAT info table. Returns 1, or 0 on a negative offset.
#[hook(addr = 0x4C9AB0, conv = cdecl, name = "_rwD3D9RasterPluginAttach", expect = "53 33 DB 53 68 80 9A 4C 00")]
pub fn d3d9_raster_plugin_attach() -> u32 {
    let off = rw_raster_register_plugin_call(0x24, 0x40C, 0x4C9A60, 0x4C9A80, 0);
    g::<i32>(D3D9_RASTER_EXT_OFFSET).write(off);
    if off < 0 {
        return 0;
    }
    for i in 0..0x80 {
        g::<u32>(D3D9_PIXEL_FORMAT_INFO + i * 4).write(0);
    }
    for &(a, size, v) in PIXEL_FORMAT_STORES.iter() {
        if size == 1 {
            g::<u8>(a as usize).write(v as u8);
        } else {
            g::<u16>(a as usize).write(v);
        }
    }
    1
}

/// `_rwD3D9RasterHasAlpha(raster)` (0x4C9EA0): the parent's alpha byte, 0 without a parent.
#[hook(addr = 0x4C9EA0, conv = cdecl, name = "_rwD3D9RasterHasAlpha", expect = "8B 44 24 04 8B 00")]
pub fn d3d9_raster_has_alpha(r: GPtr<u8>) -> u32 {
    let p = p8(rd::<u32>(r, 0));
    if p.is_null() {
        return 0;
    }
    rd::<u8>(ext(p), 8) as u32
}

/// `D3DSURFACE_DESC` (0x20 bytes) as `GetDesc` fills it.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SurfaceDesc {
    format: u32,
    ty: u32,
    usage: u32,
    pool: u32,
    ms_type: u32,
    ms_quality: u32,
    width: u32,
    height: u32,
}

// SAFETY: eight integers.
#[allow(unsafe_code)]
unsafe impl sa_core::mem::GameData for SurfaceDesc {}

/// A `RECT` (or `POINT` in the first two words) local.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RectPt {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

// SAFETY: four integers.
#[allow(unsafe_code)]
unsafe impl sa_core::mem::GameData for RectPt {}

/// `GetSurfaceLevel` / `GetCubeMapSurface` of the top parent's texture, as the lock and unlock paths
/// call them (cube when the low nibble of `E(top)+9` is set, face = high nibble).
fn top_surface(top: GPtr<u8>, level: u32, out: GPtr<u32>) -> i32 {
    let te = ext(top);
    let cube = rd::<u8>(te, 9);
    let tex = p8(rd::<u32>(te, 0));
    if cube & 0xF != 0 {
        cube_get_cube_map_surface(tex, (cube >> 4) as u32, level, out)
    } else {
        tex_get_surface_level(tex, level, out)
    }
}

/// The sub-raster rectangle `{offX, offY, width + offX, height + offY}`.
fn sub_rect(r: GPtr<u8>) -> RectPt {
    let ox = rd::<i16>(r, 0x1C) as i32;
    let oy = rd::<i16>(r, 0x1E) as i32;
    RectPt {
        left: ox,
        top: oy,
        right: rd::<i32>(r, 0xC).wrapping_add(ox),
        bottom: rd::<i32>(r, 0x10).wrapping_add(oy),
    }
}

/// The camera surface of a top raster: its texture field with a swap chain, else the render surface.
fn camera_surface(top: GPtr<u8>) -> u32 {
    let te = ext(top);
    if rd::<u32>(te, 0x1C) != 0 {
        rd::<u32>(te, 0)
    } else {
        g::<u32>(D3D_RENDER_SURFACE).read()
    }
}

/// `_rwD3D9RasterLock(pixelsOut, raster, flags)` (0x4C9F90), `stdFunc[15]`. Level = `flags >> 8`
/// (low byte), lock flags 0x800 (write bit) / 0x8810 (no write bit). Texture types (0, 4) lock a level
/// surface of the top parent's texture into `E.lockedSurface`; camera types (2, 5) copy the render
/// target (or the camera texture level) into a new system-memory surface (`GetRenderTargetData` only
/// with the read bit) and lock that. Sub-rasters lock their rectangle. On success the raster's size
/// becomes the level size (min 1), the original size is kept in +0x28/+0x2C, `*pixelsOut = pBits`.
///
/// Kept: a failed `LockRect` in the texture path leaks the level surface. The original keeps the
/// cType / cube byte / camera surface in its own `raster` argument slot (the port uses locals).
#[hook(addr = 0x4C9F90, conv = cdecl, name = "_rwD3D9RasterLock", expect = "83 EC 44 56 8B 74 24 50")]
pub fn d3d9_raster_lock(out: GPtr<u32>, r: GPtr<u8>, flags: u32) -> u32 {
    if rd::<u32>(r, 4) != 0 {
        return 0;
    }
    let e = ext(r);
    let top = top_parent(r);
    let level = (flags >> 8) as u8;
    let write = flags & 1;
    let lock_flags: u32 = if write != 0 { 0x800 } else { 0x8810 };
    let ctype = rd::<u8>(r, 0x20);
    match ctype & 7 {
        0 | 4 => {
            let locked: GPtr<u32> = e.byte_add(0xC).cast();
            if top_surface(top, level as u32, locked) < 0 {
                return 0;
            }
            let surf = p8(locked.read());
            let hr = if top == r {
                surf_lock_rect(surf, e.byte_add(0x10), GPtr::NULL, lock_flags)
            } else {
                let mut rc = sub_rect(r);
                surf_lock_rect(surf, e.byte_add(0x10), GPtr::from_raw(&mut rc).cast(), lock_flags)
            };
            if hr < 0 {
                return 0;
            }
            if flags & 2 != 0 {
                wr::<u8>(r, 0x22, rd::<u8>(r, 0x22) | 2);
            }
            if write != 0 {
                wr::<u8>(r, 0x22, rd::<u8>(r, 0x22) | 4);
            }
        }
        2 | 5 => {
            let mut slot: u32 = 0;
            if ctype & 7 == 5 {
                if top_surface(top, level as u32, GPtr::from_raw(&mut slot)) < 0 {
                    return 0;
                }
            } else {
                slot = camera_surface(top);
            }
            let mut desc = SurfaceDesc::default();
            surf_get_desc(p8(slot), GPtr::from_raw(&mut desc).cast());
            let mut hr = dev_create_offscreen_plain_surface(
                device(),
                desc.width,
                desc.height,
                desc.format,
                2,
                e.byte_add(0xC),
                0,
            );
            if hr >= 0 {
                let read = flags & 2;
                if read != 0 {
                    dev_get_render_target_data(device(), p8(slot), p8(rd::<u32>(e, 0xC)));
                }
                let locked = p8(rd::<u32>(e, 0xC));
                hr = if top == r {
                    surf_lock_rect(locked, e.byte_add(0x10), GPtr::NULL, lock_flags)
                } else {
                    let mut rc = sub_rect(r);
                    surf_lock_rect(locked, e.byte_add(0x10), GPtr::from_raw(&mut rc).cast(), lock_flags)
                };
                if hr >= 0 {
                    if read != 0 {
                        wr::<u8>(r, 0x22, rd::<u8>(r, 0x22) | 2);
                    }
                    if write != 0 {
                        wr::<u8>(r, 0x22, rd::<u8>(r, 0x22) | 4);
                    }
                } else {
                    com_release(locked);
                }
            }
            if rd::<u8>(r, 0x20) & 7 == 5 {
                com_release(p8(slot));
            }
            if hr < 0 {
                return 0;
            }
        }
        _ => return 0,
    }
    let bits = rd::<u32>(e, 0x14);
    let h = rd::<i32>(r, 0x10);
    wr::<u32>(r, 4, bits);
    let w = rd::<i32>(r, 0xC);
    wr::<i32>(r, 0x28, w);
    let sh = (level & 31) as u32;
    let w2 = w >> sh;
    wr::<i32>(r, 0x2C, h);
    let h2 = h >> sh;
    wr::<i32>(r, 0xC, w2);
    wr::<i32>(r, 0x10, h2);
    if w2 == 0 {
        wr::<i32>(r, 0xC, 1);
    }
    if h2 == 0 {
        wr::<i32>(r, 0x10, 1);
    }
    wr::<u32>(r, 0x18, rd::<u32>(e, 0x10));
    wr::<u8>(e, 0xB, level);
    out.write(rd::<u32>(r, 4));
    1
}

/// `_rwD3D9RasterUnlock(out, raster, flags)` (0x4CA290), `stdFunc[16]`. Texture types: unlock and
/// release the level surface, restore size/stride/pixels; a written level 0 of an automipmap raster
/// without hardware mipmap generation regenerates the mipmaps of the top parent
/// (`RwTextureRasterGenerateMipmaps(top, NULL)`, lockedMipLevel = 0xFF). Camera types: unlock; when
/// written, copy back with `UpdateSurface` (sub-rasters with their rectangle and point; result
/// ignored), release the copy (and the camera texture level), and for camera textures regenerate the
/// mipmaps likewise (no autoMipmapGen test there).
#[hook(addr = 0x4CA290, conv = cdecl, name = "_rwD3D9RasterUnlock", expect = "83 EC 18 53 56")]
pub fn d3d9_raster_unlock(_out: u32, r: GPtr<u8>, _flags: u32) -> u32 {
    if rd::<u32>(r, 4) == 0 {
        return 0;
    }
    let e = ext(r);
    match rd::<u8>(r, 0x20) & 7 {
        0 | 4 => {
            surf_unlock_rect(p8(rd::<u32>(e, 0xC)));
            com_release(p8(rd::<u32>(e, 0xC)));
            wr::<u32>(r, 0xC, rd::<u32>(r, 0x28));
            let pf = rd::<u8>(r, 0x22);
            wr::<u32>(r, 0x10, rd::<u32>(r, 0x2C));
            wr::<u32>(r, 0x18, 0);
            wr::<u32>(r, 4, 0);
            if pf & 4 != 0
                && rd::<u8>(r, 0x23) & 0x10 != 0
                && rd::<u8>(e, 0xB) == 0
                && rd::<u8>(e, 0xA) & 0xF == 0
            {
                wr::<u8>(r, 0x22, pf & 0xF9);
                wr::<u8>(e, 0xB, 0xFF);
                rw_texture_raster_generate_mipmaps(top_parent(r), 0);
                return 1;
            }
            wr::<u8>(r, 0x22, pf & 0xF9);
            1
        }
        2 | 5 => {
            wr::<u32>(r, 0xC, rd::<u32>(r, 0x28));
            wr::<u32>(r, 0x10, rd::<u32>(r, 0x2C));
            wr::<u32>(r, 0x18, 0);
            wr::<u32>(r, 4, 0);
            surf_unlock_rect(p8(rd::<u32>(e, 0xC)));
            if rd::<u8>(r, 0x22) & 4 == 0 {
                com_release(p8(rd::<u32>(e, 0xC)));
                wr::<u8>(r, 0x22, rd::<u8>(r, 0x22) & 0xF9);
                return 1;
            }
            let top = top_parent(r);
            let mut slot: u32 = 0;
            let mut ok = true;
            if rd::<u8>(r, 0x20) & 7 == 5 {
                let level = rd::<u8>(e, 0xB) as u32;
                ok = top_surface(top, level, GPtr::from_raw(&mut slot)) >= 0;
            } else {
                slot = camera_surface(top);
            }
            if ok {
                if top == r {
                    dev_update_surface(device(), p8(rd::<u32>(e, 0xC)), GPtr::NULL, p8(slot), GPtr::NULL);
                } else {
                    let ox = rd::<i16>(r, 0x1C) as i32;
                    let oy = rd::<i16>(r, 0x1E) as i32;
                    let mut rc = RectPt {
                        right: rd::<i32>(r, 0xC).wrapping_add(ox),
                        bottom: rd::<i32>(r, 0x10).wrapping_add(oy),
                        ..Default::default()
                    };
                    let mut pt = RectPt::default();
                    rc.left = ox;
                    pt.left = ox;
                    rc.top = oy;
                    pt.top = oy;
                    dev_update_surface(
                        device(),
                        p8(rd::<u32>(e, 0xC)),
                        GPtr::from_raw(&mut rc).cast(),
                        p8(slot),
                        GPtr::from_raw(&mut pt).cast(),
                    );
                }
                if rd::<u8>(r, 0x20) & 7 == 5 {
                    com_release(p8(slot));
                }
            }
            com_release(p8(rd::<u32>(e, 0xC)));
            wr::<u8>(r, 0x22, rd::<u8>(r, 0x22) & 0xF9);
            if rd::<u8>(r, 0x20) & 7 == 5 && rd::<u8>(r, 0x23) & 0x10 != 0 && rd::<u8>(e, 0xB) == 0 {
                wr::<u8>(e, 0xB, 0xFF);
                rw_texture_raster_generate_mipmaps(top, 0);
            }
            1
        }
        _ => 0,
    }
}

/// The raster types the palette lock/unlock accept: cType & 7 in {0, 4, 5}.
fn palette_type_ok(r: GPtr<u8>) -> bool {
    let t = rd::<u8>(r, 0x20) & 7;
    t == 0 || (t > 3 && t <= 5)
}

/// `_rwD3D9RasterLockPalette(out, raster, flags)` (0x4CA4E0), `stdFunc[23]`: `raster.palette =
/// E.palette`, privateFlags |= 0x18 (write bit) or 8, `*out = palette`.
#[hook(addr = 0x4CA4E0, conv = cdecl, name = "_rwD3D9RasterLockPalette", expect = "8B 44 24 08 33 C9")]
pub fn d3d9_raster_lock_palette(out: GPtr<u32>, r: GPtr<u8>, flags: u32) -> u32 {
    if !palette_type_ok(r) {
        return 0;
    }
    let pal = rd::<u32>(ext(r), 4);
    let pf = rd::<u8>(r, 0x22);
    wr::<u32>(r, 8, pal);
    if flags & 1 != 0 {
        wr::<u8>(r, 0x22, pf | 0x18);
    } else {
        wr::<u8>(r, 0x22, pf | 8);
    }
    out.write(pal);
    1
}

/// `_rwD3D9RasterUnlockPalette(out, raster, flags)` (0x4CA540), `stdFunc[24]`: a written palette
/// (privateFlags 0x10) is uploaded with `SetPaletteEntries(palette[+0x400], palette)`; privateFlags
/// &= 0xE7, `raster.palette = NULL`.
#[hook(addr = 0x4CA540, conv = cdecl, name = "_rwD3D9RasterUnlockPalette", expect = "56 8B 74 24 0C")]
pub fn d3d9_raster_unlock_palette(_out: u32, r: GPtr<u8>, _flags: u32) -> u32 {
    if !palette_type_ok(r) {
        return 0;
    }
    if rd::<u8>(r, 0x22) & 0x10 != 0 {
        let pal = p8(rd::<u32>(ext(r), 4));
        dev_set_palette_entries(device(), rd::<u32>(pal, 0x400), pal);
    }
    wr::<u8>(r, 0x22, rd::<u8>(r, 0x22) & 0xE7);
    wr::<u32>(r, 8, 0);
    1
}

/// `_rwD3D9SetRasterContext(out, raster, flags)` (0x4CB520), `stdFunc[11]`.
#[hook(addr = 0x4CB520, conv = cdecl, name = "_rwD3D9SetRasterContext", expect = "8B 44 24 08 A3 E4 E9 B4 00")]
pub fn d3d9_set_raster_context(_out: u32, r: u32, _flags: u32) -> u32 {
    g::<u32>(D3D9_RASTER_CONTEXT).write(r);
    1
}

/// `_rxD3D9VideoMemoryRasterListAdd(raster)` (0x4CB530): a node `{raster, next}` from the list
/// freelist (hint 0x30411) pushed at the head. No NULL check (kept). EAX (the node) is unused by the
/// three callers.
#[hook(addr = 0x4CB530, conv = cdecl, name = "_rxD3D9VideoMemoryRasterListAdd", expect = "A1 00 EA B4 00")]
pub fn vm_raster_list_add(r: GPtr<u8>) {
    let node = rw_memory_alloc(p8(g::<u32>(VM_RASTER_FREELIST).read()), 0x30411);
    let head = g::<u32>(VM_RASTER_LIST).read();
    wr::<u32>(node, 0, a32(r));
    wr::<u32>(node, 4, head);
    g::<u32>(VM_RASTER_LIST).write(a32(node));
}

/// `_rxD3D9VideoMemoryRasterListDestroy()` (0x4CB5E0): frees every node (head advanced after each
/// free), then destroys the node freelist and clears it.
#[hook(addr = 0x4CB5E0, conv = cdecl, name = "_rxD3D9VideoMemoryRasterListDestroy", expect = "A1 FC E9 B4 00")]
pub fn vm_raster_list_destroy() {
    let mut n = g::<u32>(VM_RASTER_LIST).read();
    while n != 0 {
        let next = rd::<u32>(p8(n), 4);
        rw_memory_free(p8(g::<u32>(VM_RASTER_FREELIST).read()), p8(n));
        g::<u32>(VM_RASTER_LIST).write(next);
        n = next;
    }
    let fl = g::<u32>(VM_RASTER_FREELIST).read();
    if fl != 0 {
        rw_free_list_destroy(p8(fl));
        g::<u32>(VM_RASTER_FREELIST).write(0);
    }
}

/// `_rxD3D9VideoMemoryRasterListRelease()` (0x4CB640): for each listed top-level raster releases its
/// video-memory resources (cType compared as a whole byte): 0/4 only depth-format textures (D3DFMT
/// 0x46..=0x50), 1 unless it is the default depth surface, 5 always (pointers cleared); 2 releases the
/// swap chain without clearing the pointer (kept).
#[hook(addr = 0x4CB640, conv = cdecl, name = "_rxD3D9VideoMemoryRasterListRelease", expect = "8B 0D FC E9 B4 00")]
pub fn vm_raster_list_release() {
    let mut n = g::<u32>(VM_RASTER_LIST).read();
    while n != 0 {
        let r = p8(rd::<u32>(p8(n), 0));
        let next = rd::<u32>(p8(n), 4);
        let e = ext(r);
        if rd::<u32>(r, 0) == a32(r) {
            match rd::<u8>(r, 0x20) {
                0 | 4 => {
                    let t = rd::<u32>(e, 0);
                    let f = rd::<i32>(e, 0x18);
                    if t != 0 && (0x46..=0x50).contains(&f) {
                        com_release(p8(t));
                        wr::<u32>(e, 0, 0);
                    }
                }
                1 => {
                    let t = rd::<u32>(e, 0);
                    if t != 0 && t != g::<u32>(D3D_DEPTH_SURFACE).read() {
                        com_release(p8(t));
                        wr::<u32>(e, 0, 0);
                    }
                }
                2 => {
                    let sc = rd::<u32>(e, 0x1C);
                    if sc != 0 {
                        com_release(p8(sc));
                    }
                }
                5 => {
                    let t = rd::<u32>(e, 0);
                    if t != 0 {
                        com_release(p8(t));
                        wr::<u32>(e, 0, 0);
                    }
                }
                _ => {}
            }
        }
        n = next;
    }
}

/// `_rwD3D9RasterGetMipLevels(out, raster, flags)` (0x4CBCB0), `stdFunc[28]`: for texture types
/// (cType 0, 4, 5 of the top raster, starting at the raster itself) the texture's level count, or
/// without a texture the bit count of max(width, height) (signed compare); other types 1.
#[hook(addr = 0x4CBCB0, conv = cdecl, name = "_rwD3D9RasterGetMipLevels", expect = "8B 44 24 08 8B 08")]
pub fn d3d9_raster_get_mip_levels(out: GPtr<u32>, r: GPtr<u8>, _flags: u32) -> u32 {
    let mut t = r;
    loop {
        let n = p8(rd::<u32>(t, 0));
        if n == t {
            break;
        }
        t = n;
    }
    let c = rd::<u8>(t, 0x20);
    if c != 4 && c != 5 && c != 0 {
        out.write(1);
        return 1;
    }
    let tex = p8(rd::<u32>(ext(t), 0));
    if !tex.is_null() {
        let n = tex_get_level_count(tex);
        out.write(n);
        return 1;
    }
    let w = rd::<i32>(t, 0xC);
    let h = rd::<i32>(t, 0x10);
    let mut m = (if w > h { w } else { h }) as u32;
    let mut n = 0u32;
    while m != 0 {
        m >>= 1;
        n += 1;
    }
    out.write(n);
    1
}

/// `_rwD3D9TextureSetRaster(texture, raster, flags)` (0x4CBD40), `stdFunc[8]`: `texture->raster = raster`.
#[hook(addr = 0x4CBD40, conv = cdecl, name = "_rwD3D9TextureSetRaster", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn d3d9_texture_set_raster(t: GPtr<u8>, r: u32, _flags: u32) -> u32 {
    wr::<u32>(t, 0, r);
    1
}

/// `_rwD3D9RasterSubRaster(raster, parent, flags)` (0x4CBD50), `stdFunc[12]`: copies stride, depth,
/// cType, cFormat from the parent; cpPixels = NULL.
#[hook(addr = 0x4CBD50, conv = cdecl, name = "_rwD3D9RasterSubRaster", expect = "8B 4C 24 08 8B 51 18")]
pub fn d3d9_raster_sub_raster(r: GPtr<u8>, parent: GPtr<u8>, _flags: u32) -> u32 {
    wr::<u32>(r, 0x18, rd::<u32>(parent, 0x18));
    wr::<u32>(r, 0x14, rd::<u32>(parent, 0x14));
    wr::<u8>(r, 0x20, rd::<u8>(parent, 0x20));
    wr::<u8>(r, 0x23, rd::<u8>(parent, 0x23));
    wr::<u32>(r, 4, 0);
    1
}

/// `CheckDeviceFormat(adapter, type, displayFormat, usage, rtype, fmt)`.
fn check_format(usage: u32, rtype: u32, fmt: u32) -> i32 {
    d3d_check_device_format(
        d3d(),
        g::<u32>(D3D_ADAPTER_INDEX).read(),
        g::<u32>(D3D_ADAPTER_TYPE).read(),
        g::<u32>(D3D_DISPLAY_FORMAT).read(),
        usage,
        rtype,
        fmt,
    )
}

/// `CheckDepthStencilMatch(adapter, type, displayFormat, rt, ds)`.
fn check_ds(rt: u32, ds: u32) -> i32 {
    d3d_check_depth_stencil_match(
        d3d(),
        g::<u32>(D3D_ADAPTER_INDEX).read(),
        g::<u32>(D3D_ADAPTER_TYPE).read(),
        g::<u32>(D3D_DISPLAY_FORMAT).read(),
        rt,
        ds,
    )
}

/// `_rwD3D9CheckValidZBufferFormat(fmt)` (0x4CBD80): DEPTHSTENCIL surface format, then a depth-stencil
/// match (display format as render target).
#[hook(addr = 0x4CBD80, conv = cdecl, name = "_rwD3D9CheckValidZBufferFormat", expect = "8B 15 28 24 8E 00")]
pub fn d3d9_check_valid_zbuffer_format(fmt: u32) -> u32 {
    let mut hr = check_format(2, 1, fmt);
    if hr >= 0 {
        hr = check_ds(g::<u32>(D3D_DISPLAY_FORMAT).read(), fmt);
    }
    (hr >= 0) as u32
}

/// `_rwD3D9CheckValidTextureFormat(fmt)` (0x4CBDE0).
#[hook(addr = 0x4CBDE0, conv = cdecl, name = "_rwD3D9CheckValidTextureFormat", expect = "A1 F0 BE C9 00")]
pub fn d3d9_check_valid_texture_format(fmt: u32) -> u32 {
    (check_format(0, 3, fmt) >= 0) as u32
}

/// `_rwD3D9CheckValidCameraTextureFormat(fmt)` (0x4CBE20): RENDERTARGET texture, then a match of fmt
/// with the depth-buffer format.
#[hook(addr = 0x4CBE20, conv = cdecl, name = "_rwD3D9CheckValidCameraTextureFormat", expect = "8B 15 28 24 8E 00")]
pub fn d3d9_check_valid_camera_texture_format(fmt: u32) -> u32 {
    let mut hr = check_format(1, 3, fmt);
    if hr >= 0 {
        hr = check_ds(fmt, g::<u32>(D3D_DEPTH_FORMAT).read());
    }
    (hr >= 0) as u32
}

/// `_rwD3D9CheckValidZBufferTextureFormat(fmt)` (0x4CBE80).
#[hook(addr = 0x4CBE80, conv = cdecl, name = "_rwD3D9CheckValidZBufferTextureFormat", expect = "8B 15 28 24 8E 00")]
pub fn d3d9_check_valid_zbuffer_texture_format(fmt: u32) -> u32 {
    let mut hr = check_format(2, 3, fmt);
    if hr >= 0 {
        hr = check_ds(g::<u32>(D3D_DISPLAY_FORMAT).read(), fmt);
    }
    (hr >= 0) as u32
}

/// `_rwD3D9CheckValidCubeTextureFormat(fmt)` (0x4CBEE0).
#[hook(addr = 0x4CBEE0, conv = cdecl, name = "_rwD3D9CheckValidCubeTextureFormat", expect = "A1 F0 BE C9 00")]
pub fn d3d9_check_valid_cube_texture_format(fmt: u32) -> u32 {
    (check_format(0, 5, fmt) >= 0) as u32
}

/// `_rwD3D9CheckValidCameraCubeTextureFormat(fmt)` (0x4CBF20).
#[hook(addr = 0x4CBF20, conv = cdecl, name = "_rwD3D9CheckValidCameraCubeTextureFormat", expect = "8B 15 28 24 8E 00")]
pub fn d3d9_check_valid_camera_cube_texture_format(fmt: u32) -> u32 {
    let mut hr = check_format(1, 5, fmt);
    if hr >= 0 {
        hr = check_ds(fmt, g::<u32>(D3D_DEPTH_FORMAT).read());
    }
    (hr >= 0) as u32
}

/// `_rwD3D9CheckAutoMipmapGenTextureFormat(fmt)` (0x4CBFD0): with CANAUTOGENMIPMAP,
/// `CheckDeviceFormat(AUTOGENMIPMAP | RENDERTARGET, texture)` == D3D_OK (not just >= 0).
#[hook(addr = 0x4CBFD0, conv = cdecl, name = "_rwD3D9CheckAutoMipmapGenTextureFormat", expect = "F7 05 0C BF C9 00 00 00 00 40")]
pub fn d3d9_check_auto_mipmap_gen_texture_format(fmt: u32) -> u32 {
    if g::<u32>(D3D_CAPS2).read() & 0x4000_0000 == 0 {
        return 0;
    }
    (check_format(0x401, 3, fmt) == 0) as u32
}

/// `_rwD3D9CheckAutoMipmapGenCubeTextureFormat(fmt)` (0x4CC020): AUTOGENMIPMAP, cube texture, == D3D_OK.
#[hook(addr = 0x4CC020, conv = cdecl, name = "_rwD3D9CheckAutoMipmapGenCubeTextureFormat", expect = "F7 05 0C BF C9 00 00 00 00 40")]
pub fn d3d9_check_auto_mipmap_gen_cube_texture_format(fmt: u32) -> u32 {
    if g::<u32>(D3D_CAPS2).read() & 0x4000_0000 == 0 {
        return 0;
    }
    (check_format(0x400, 5, fmt) == 0) as u32
}

/// `_rxD3D9VideoMemoryRasterListCreate()` (0x4CC150): the node freelist (8 bytes, 0x7F per block,
/// alignment 4, hint 0x30411); no result (EAX = the freelist, unused by the caller).
#[hook(addr = 0x4CC150, conv = cdecl, name = "_rxD3D9VideoMemoryRasterListCreate", expect = "68 11 04 03 00")]
pub fn vm_raster_list_create() {
    let fl = rw_free_list_create(8, 0x7F, 4, 0x30411);
    g::<u32>(VM_RASTER_FREELIST).write(a32(fl));
}

/// `_rwD3D9RasterClose()` (0x4CC170): destroys the video-memory list, clears the three driver
/// globals 0xB4E9E8..0xB4E9F0, frees the buffer at 0xB4E9F4 and destroys the palette freelist.
#[hook(addr = 0x4CC170, conv = cdecl, name = "_rwD3D9RasterClose", expect = "56 E8 6A F4 FF FF")]
pub fn d3d9_raster_close() {
    vm_raster_list_destroy_call();
    let buf = g::<u32>(D3D9_RASTER_BUFFER).read();
    g::<u32>(D3D9_RASTER_GLOBALS).write(0);
    g::<u32>(D3D9_RASTER_GLOBALS + 4).write(0);
    g::<u32>(D3D9_RASTER_GLOBALS + 8).write(0);
    if buf != 0 {
        rw_free_call(buf);
        g::<u32>(D3D9_RASTER_BUFFER).write(0);
    }
    let fl = g::<u32>(D3D9_PALETTE_FREELIST).read();
    if fl != 0 {
        rw_free_list_destroy(p8(fl));
        g::<u32>(D3D9_PALETTE_FREELIST).write(0);
    }
}

// ---------------------------------------------------------------------------------------------
// Texture leftovers (mipmap callbacks, stream size/write, chunk info) and the anisotropy plugin.

/// TEXTUREMODULE offset global (module +0x2C mipmap generate callback, +0x30 mipmap name callback).
const TEXTURE_MODULE: usize = 0xC97B4C;
/// Texture plugin registry.
const TEXTURE_REGISTRY: u32 = 0x8E23CC;
/// The anisotropy plugin offset in RwTexture (`RpAnisotGetTextureOffset`'s global).
const ANISOT_TEXTURE_OFFSET: usize = 0xC92478;

orig! {
    /// The mipmap generate callback `M+0x2C (raster, image)` (stored pointer)
    fn call_mipmap_generate_cb(f: u32, r: GPtr<u8>, image: u32) -> u32 = cdecl dyn => unported;
    /// `RwStreamReadInt32(stream, buf, len)` (0x7ED540)
    fn rw_stream_read_int32(s: GPtr<u8>, buf: GPtr<u32>, len: u32) -> GPtr<u8> = cdecl 0x7ED540 => unported;
    /// `RwStreamWriteInt32(stream, buf, len)` (0x7ED460)
    fn rw_stream_write_int32(s: GPtr<u8>, buf: GPtr<u32>, len: u32) -> GPtr<u8> = cdecl 0x7ED460 => unported;
    /// `RwStreamWrite(stream, buf, len)` (0x7ECB30)
    fn rw_stream_write(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> GPtr<u8> = cdecl 0x7ECB30 => unported;
    /// `RwStreamRead(stream, buf, len)` (0x7EC9D0)
    fn rw_stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => crate::stream::rw_stream_read;
    /// `RwStreamFindChunk(stream, type, &length, &version)` (0x7ED2D0)
    fn rw_stream_find_chunk(s: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> u32 = cdecl 0x7ED2D0 => crate::stream::rw_stream_find_chunk;
    /// `RwStreamSkip(stream, offset)` (0x7ECD00)
    fn rw_stream_skip(s: GPtr<u8>, off: u32) -> GPtr<u8> = cdecl 0x7ECD00 => unported;
    /// `_rwStreamWriteVersionedChunkHeader(stream, type, size, version, build)` (0x7ED270)
    fn write_chunk_header(s: GPtr<u8>, ty: u32, size: u32, version: u32, build: u32) -> GPtr<u8> = cdecl 0x7ED270 => unported;
    /// `_rwStringStreamGetSize(string)` (0x80A470)
    fn string_stream_get_size(s: GPtr<u8>) -> u32 = cdecl 0x80A470 => unported;
    /// `_rwStringStreamWrite(string, stream)` (0x80A4A0)
    fn string_stream_write(string: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> = cdecl 0x80A4A0 => unported;
    /// `_rwPluginRegistryGetSize(reg, obj)` (0x808B00)
    fn plugin_get_size(reg: GPtr<u8>, obj: GPtr<u8>) -> u32 = cdecl 0x808B00 => unported;
    /// `_rwPluginRegistryWriteDataChunks(reg, stream, obj)` (0x808B40)
    fn plugin_write_data_chunks(reg: GPtr<u8>, s: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808B40 => unported;
    /// `_rwPluginRegistryAddPlgnStrmlwysCB(reg, id, cb)` (0x808920)
    fn plugin_add_stream_always_cb(reg: GPtr<u8>, id: u32, cb: u32) -> i32 = cdecl 0x808920 => unported;
    /// `RwRasterLock(raster, level, mode)` (0x7FB2D0)
    fn rw_raster_lock_call(r: GPtr<u8>, level: u32, mode: u32) -> u32 = cdecl 0x7FB2D0 => rw_raster_lock;
    /// `RwRasterUnlock(raster)` (0x7FAEC0)
    fn rw_raster_unlock_call(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7FAEC0 => rw_raster_unlock;
    /// `RwRasterLockPalette(raster, mode)` (0x7FB0E0)
    fn rw_raster_lock_palette_call(r: GPtr<u8>, mode: u32) -> u32 = cdecl 0x7FB0E0 => rw_raster_lock_palette;
    /// `RwRasterUnlockPalette(raster)` (0x7FAFF0)
    fn rw_raster_unlock_palette_call(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7FAFF0 => rw_raster_unlock_palette;
    /// `RwRasterGetNumLevels(raster)` (0x7FB160)
    fn rw_raster_get_num_levels_call(r: GPtr<u8>) -> i32 = cdecl 0x7FB160 => rw_raster_get_num_levels;
    /// `_rwD3D9NativeTextureGetSize(&size, texture, 0)` (0x4CD360)
    fn native_texture_get_size_call(out: GPtr<u32>, t: GPtr<u8>, n: u32) -> u32 = cdecl 0x4CD360 => d3d9_native_texture_get_size;

    /// `IDirect3D(Cube)Texture9::GetLevelDesc(level, desc)` (+0x44)
    fn tex_get_level_desc(tex: GPtr<u8>, level: u32, desc: GPtr<u8>) -> i32 = stdcall vslot 0x44 => unported;
    /// `IDirect3DTexture9::LockRect(level, locked, rect, flags)` (+0x4C)
    fn tex_lock_rect(tex: GPtr<u8>, level: u32, locked: GPtr<u8>, rect: u32, flags: u32) -> i32 = stdcall vslot 0x4C => unported;
    /// `IDirect3DCubeTexture9::LockRect(face, level, locked, rect, flags)` (+0x4C)
    fn cube_lock_rect(tex: GPtr<u8>, face: u32, level: u32, locked: GPtr<u8>, rect: u32, flags: u32) -> i32 = stdcall vslot 0x4C => unported;
    /// `IDirect3DTexture9::UnlockRect(level)` (+0x50)
    fn tex_unlock_rect(tex: GPtr<u8>, level: u32) -> i32 = stdcall vslot 0x50 => unported;
    /// `IDirect3DCubeTexture9::UnlockRect(face, level)` (+0x50)
    fn cube_unlock_rect(tex: GPtr<u8>, face: u32, level: u32) -> i32 = stdcall vslot 0x50 => unported;
}

fn tmod() -> GPtr<u8> {
    module(TEXTURE_MODULE)
}

/// `RwTextureValidatePlugins(texture)` (0x7F3C00): returns 1.
#[hook(addr = 0x7F3C00, conv = cdecl, name = "RwTextureValidatePlugins", expect = "B8 01 00 00 00")]
pub fn rw_texture_validate_plugins(_t: u32) -> u32 {
    1
}

/// `RwTextureSetMipmapGenerationCallBack(cb)` (0x7F3C70).
#[hook(addr = 0x7F3C70, conv = cdecl, name = "RwTextureSetMipmapGenerationCallBack", expect = "8B 0D 24 7B C9 00")]
pub fn rw_texture_set_mipmap_generation_callback(cb: u32) -> u32 {
    wr::<u32>(tmod(), 0x2C, cb);
    1
}

/// `RwTextureGetMipmapGenerationCallBack()` (0x7F3C90).
#[hook(addr = 0x7F3C90, conv = cdecl, name = "RwTextureGetMipmapGenerationCallBack", expect = "A1 24 7B C9 00")]
pub fn rw_texture_get_mipmap_generation_callback() -> u32 {
    rd::<u32>(tmod(), 0x2C)
}

/// `RwTextureSetMipmapNameCallBack(cb)` (0x7F3CA0).
#[hook(addr = 0x7F3CA0, conv = cdecl, name = "RwTextureSetMipmapNameCallBack", expect = "8B 0D 24 7B C9 00")]
pub fn rw_texture_set_mipmap_name_callback(cb: u32) -> u32 {
    wr::<u32>(tmod(), 0x30, cb);
    1
}

/// `RwTextureGetMipmapNameCallBack()` (0x7F3CC0).
#[hook(addr = 0x7F3CC0, conv = cdecl, name = "RwTextureGetMipmapNameCallBack", expect = "A1 24 7B C9 00")]
pub fn rw_texture_get_mipmap_name_callback() -> u32 {
    rd::<u32>(tmod(), 0x30)
}

/// `RwTextureRasterGenerateMipmaps(raster, image)` (0x7F3CD0): the generate callback's result as 0/1.
#[hook(addr = 0x7F3CD0, conv = cdecl, name = "RwTextureRasterGenerateMipmaps", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn rw_texture_raster_generate_mipmaps_port(r: GPtr<u8>, image: u32) -> u32 {
    let f = rd::<u32>(tmod(), 0x2C);
    (call_mipmap_generate_cb(f, r, image) != 0) as u32
}

/// `RwTextureSetStreamAlwaysCallBack(pluginID, cb)` (0x804580).
#[hook(addr = 0x804580, conv = cdecl, name = "RwTextureSetStreamAlwaysCallBack", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn rw_texture_set_stream_always_callback(id: u32, cb: u32) -> i32 {
    plugin_add_stream_always_cb(p8(TEXTURE_REGISTRY), id, cb)
}

/// `RwTextureStreamGetSize(texture)` (0x8045A0): `name + 0x1C + mask + 0xC + plugins + 0xC` (string
/// stream sizes of name and mask).
#[hook(addr = 0x8045A0, conv = cdecl, name = "RwTextureStreamGetSize", expect = "56 57 8B 7C 24 0C")]
pub fn rw_texture_stream_get_size(t: GPtr<u8>) -> u32 {
    let a = string_stream_get_size(t.byte_add(0x10)).wrapping_add(0x1C);
    let b = string_stream_get_size(t.byte_add(0x30));
    let s = a.wrapping_add(b).wrapping_add(0xC);
    plugin_get_size(p8(TEXTURE_REGISTRY), t).wrapping_add(s).wrapping_add(0xC)
}

/// `RwTextureStreamWrite(texture, stream)` (0x8045E0): texture chunk header (6, size as
/// `RwTextureStreamGetSize`, 0x36003 / 0xFFFF), struct header (1, 4), the dword
/// `(raster && !(cFormat & AUTOMIPMAP)) << 16 | filterAddressing & 0xFFFF` (the original builds it in
/// its own `texture` argument slot), name and mask strings, plugin chunks. Returns texture or NULL.
#[hook(addr = 0x8045E0, conv = cdecl, name = "RwTextureStreamWrite", expect = "53 55 56 57 8B 7C 24 14")]
pub fn rw_texture_stream_write(t: GPtr<u8>, s: GPtr<u8>) -> GPtr<u8> {
    let name = t.byte_add(0x10);
    let mask = t.byte_add(0x30);
    let a = string_stream_get_size(name).wrapping_add(0x1C);
    let b = string_stream_get_size(mask);
    let a = a.wrapping_add(b).wrapping_add(0xC);
    let size = plugin_get_size(p8(TEXTURE_REGISTRY), t).wrapping_add(a).wrapping_add(0xC);
    if write_chunk_header(s, 6, size, 0x36003, 0xFFFF).is_null() {
        return GPtr::NULL;
    }
    if write_chunk_header(s, 1, 4, 0x36003, 0xFFFF).is_null() {
        return GPtr::NULL;
    }
    let r = p8(rd::<u32>(t, 0));
    let mip = (!r.is_null() && rd::<u8>(r, 0x23) & 0x10 == 0) as u32;
    let mut v = (mip << 16) | (rd::<u32>(t, 0x50) & 0xFFFF);
    if rw_stream_write(s, GPtr::from_raw(&mut v).cast(), 4).is_null() {
        return GPtr::NULL;
    }
    if string_stream_write(name, s).is_null() {
        return GPtr::NULL;
    }
    if string_stream_write(mask, s).is_null() {
        return GPtr::NULL;
    }
    if plugin_write_data_chunks(p8(TEXTURE_REGISTRY), s, t).is_null() {
        return GPtr::NULL;
    }
    t
}

/// `_rwTextureChunkInfoRead(stream, info, &size)` (0x804E60): finds the struct chunk, reads one dword
/// (the original into its zeroed `stream` argument slot), `*size = length + 0xC`, skips `length - 4`,
/// then info = {filter (byte 0), addressU (bits 8..11), addressV (bits 12..15)}. Returns info, or NULL
/// when the chunk is missing or the read is short.
#[hook(addr = 0x804E60, conv = cdecl, name = "_rwTextureChunkInfoRead", expect = "51 56 8B 74 24 0C")]
pub fn rw_texture_chunk_info_read(s: GPtr<u8>, info: GPtr<u8>, size: GPtr<u32>) -> GPtr<u8> {
    let mut len: u32 = 0;
    if rw_stream_find_chunk(s, 1, GPtr::from_raw(&mut len), GPtr::NULL) == 0 {
        return GPtr::NULL;
    }
    let mut v: u32 = 0;
    if rw_stream_read(s, GPtr::from_raw(&mut v).cast(), 4) != 4 {
        return GPtr::NULL;
    }
    size.write(len.wrapping_add(0xC));
    rw_stream_skip(s, len.wrapping_sub(4));
    wr::<u32>(info, 0, v & 0xFF);
    wr::<u32>(info, 4, (v >> 8) & 0xF);
    wr::<u32>(info, 8, (v >> 12) & 0xF);
    info
}

/// `RwTexture` + anisotropy offset.
fn anisot(t: GPtr<u8>) -> GPtr<u8> {
    t.byte_add(g::<i32>(ANISOT_TEXTURE_OFFSET).read() as isize)
}

/// `cb_ANISOTPLUGIN_RwTexture_Ctor(texture, offset, size)` (0x749020): anisotropy byte = 1.
#[hook(addr = 0x749020, conv = cdecl, name = "cb_ANISOTPLUGIN_RwTexture_Ctor", expect = "8B 44 24 04 8B 0D 78 24 C9 00")]
pub fn anisot_ctor(t: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    anisot(t).write(1);
    t
}

/// `cb_ANISOTPLUGIN_RwTexture_Dtor(texture, offset, size)` (0x749030): returns the texture (5 bytes:
/// `mov eax, [esp+4]; ret`).
#[hook(addr = 0x749030, conv = cdecl, name = "cb_ANISOTPLUGIN_RwTexture_Dtor", expect = "8B 44 24 04 C3")]
pub fn anisot_dtor(t: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    t
}

/// `cb_ANISOTPLUGIN_RwTexture_Copy(dst, src, offset, size)` (0x749040): copies the byte.
#[hook(addr = 0x749040, conv = cdecl, name = "cb_ANISOTPLUGIN_RwTexture_Copy", expect = "8B 0D 78 24 C9 00")]
pub fn anisot_copy(dst: GPtr<u8>, src: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let v = anisot(src).read();
    anisot(dst).write(v);
    dst
}

/// `cb_ANISOTPLUGIN_RwTexture_StreamRead(stream, length, texture, offset, size)` (0x749060): only a
/// 4-byte chunk on a texture; reads an int32 (the original into its own `length` argument slot) and
/// stores its low byte. Returns stream or NULL.
#[hook(addr = 0x749060, conv = cdecl, name = "cb_ANISOTPLUGIN_RwTexture_StreamRead", expect = "8B 44 24 08 56")]
pub fn anisot_stream_read(s: GPtr<u8>, len: u32, t: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    if len != 4 || t.is_null() {
        return GPtr::NULL;
    }
    let mut v = len;
    if rw_stream_read_int32(s, GPtr::from_raw(&mut v), 4).is_null() {
        return GPtr::NULL;
    }
    anisot(t).write(v as u8);
    s
}

/// `cb_ANISOTPLUGIN_RwTexture_StreamWrite(stream, length, texture, offset, size)` (0x7490B0): for a
/// texture with a raster writes the byte as an int32 (built in the original's `texture` argument
/// slot). Returns stream or NULL.
#[hook(addr = 0x7490B0, conv = cdecl, name = "cb_ANISOTPLUGIN_RwTexture_StreamWrite", expect = "8B 44 24 0C 85 C0")]
pub fn anisot_stream_write(s: GPtr<u8>, _len: u32, t: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    if t.is_null() || rd::<u32>(t, 0) == 0 {
        return GPtr::NULL;
    }
    let mut v = anisot(t).read() as u32;
    if rw_stream_write_int32(s, GPtr::from_raw(&mut v), 4).is_null() {
        return GPtr::NULL;
    }
    s
}

/// `cb_ANISOTPLUGIN_RwTexture_StreamGetSize(texture, offset, size)` (0x7490F0): 4 for a texture with a
/// raster and an anisotropy other than 1, else 0.
#[hook(addr = 0x7490F0, conv = cdecl, name = "cb_ANISOTPLUGIN_RwTexture_StreamGetSize", expect = "8B 44 24 04 85 C0")]
pub fn anisot_stream_get_size(t: GPtr<u8>, _offset: i32, _size: i32) -> i32 {
    if t.is_null() || rd::<u32>(t, 0) == 0 || anisot(t).read() == 1 {
        return 0;
    }
    4
}

/// A `D3DLOCKED_RECT` + `D3DSURFACE_DESC` pair of locals (pitch at +0, height at +0x24).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct LockDesc {
    pitch: i32,
    bits: u32,
    desc: SurfaceDesc,
}

// SAFETY: plain integers.
#[allow(unsafe_code)]
unsafe impl sa_core::mem::GameData for LockDesc {}

/// `_rwD3D9NativeTextureGetSize(&size, texture, flags)` (0x4CD360): 0x54 without a raster; otherwise
/// 0x64 (+0x80 PAL4 / +0x400 PAL8 palette) plus, per level (1 with hardware mipmap generation, else
/// `GetLevelCount`, from the last level down), `4 + pitch * height` (`pitch * (height >> 2)` for
/// compressed), each level locked read-only (0x8010) to learn the pitch; cube textures lock face 0 and
/// count the level sum 6 times. Results of GetLevelDesc/LockRect are not checked (the original then
/// reads its uninitialised locals; the port's are 0). The original keeps the running size in its
/// `texture` argument slot.
#[hook(addr = 0x4CD360, conv = cdecl, name = "_rwD3D9NativeTextureGetSize", expect = "8B 44 24 08 8B 00")]
pub fn d3d9_native_texture_get_size(out: GPtr<u32>, t: GPtr<u8>, _flags: u32) -> u32 {
    let r = p8(rd::<u32>(t, 0));
    if r.is_null() {
        out.write(0x54);
        return 1;
    }
    let fmt = (rd::<u8>(r, 0x23) as u32) << 8;
    let mut size: u32 = if fmt & 0x4000 != 0 {
        0xE4
    } else if fmt & 0x2000 != 0 {
        0x464
    } else {
        0x64
    };
    let e = ext(r);
    let cube = rd::<u8>(e, 9) & 0xF != 0;
    let auto = rd::<u8>(e, 0xA) & 0xF != 0;
    let tex = p8(rd::<u32>(e, 0));
    let mut l = LockDesc::default();
    let level_size = |l: &LockDesc| -> u32 {
        if rd::<u8>(e, 0xA) & 0xF0 != 0 {
            (l.desc.height >> 2).wrapping_mul(l.pitch as u32)
        } else {
            (l.pitch as u32).wrapping_mul(l.desc.height)
        }
    };
    if cube {
        let mut sum: u32 = 0;
        let mut n = if auto { 1 } else { tex_get_level_count(tex) };
        while n != 0 {
            n -= 1;
            tex_get_level_desc(tex, n, GPtr::from_raw(&mut l.desc).cast());
            cube_lock_rect(tex, 0, n, GPtr::from_raw(&mut l).cast(), 0, 0x8010);
            cube_unlock_rect(tex, 0, n);
            sum = sum.wrapping_add(level_size(&l)).wrapping_add(4);
        }
        size = size.wrapping_add(sum.wrapping_mul(6));
    } else {
        let mut n = if auto { 1 } else { tex_get_level_count(tex) };
        while n != 0 {
            n -= 1;
            tex_get_level_desc(tex, n, GPtr::from_raw(&mut l.desc).cast());
            tex_lock_rect(tex, n, GPtr::from_raw(&mut l).cast(), 0, 0x8010);
            tex_unlock_rect(tex, n);
            size = size.wrapping_add(level_size(&l)).wrapping_add(4);
        }
    }
    out.write(size);
    1
}

/// The native texture header `_rwD3D9NativeTextureWrite` writes (0x48 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeHeader {
    platform: u32,
    filter_addressing: u32,
    name: [u32; 8],
    mask: [u32; 8],
}

// SAFETY: plain integers.
#[allow(unsafe_code)]
unsafe impl sa_core::mem::GameData for NativeHeader {}

/// The raster info block (0x10 bytes).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NativeRasterInfo {
    format: u32,
    d3d_format: u32,
    width: u16,
    height: u16,
    depth: u8,
    num_levels: u8,
    ty: u8,
    flags: u8,
}

// SAFETY: plain integers.
#[allow(unsafe_code)]
unsafe impl sa_core::mem::GameData for NativeRasterInfo {}

/// `_rwD3D9NativeTextureWrite(stream, texture, flags)` (0x4CD4D0): struct chunk header (size from
/// `_rwD3D9NativeTextureGetSize` - 0xC), the 0x48-byte header {9, filter | addressU << 8 | addressV <<
/// 12, name, mask}, the 0x10-byte raster info (flags: 1 alpha, 2 cube, 4 hardware mipmaps, 8
/// compressed), the palette (PAL4 0x80 / PAL8 0x400 bytes, locked for read), then per face (6 for cube,
/// the face nibble of `E+9` reset to 0 and advanced after each face) and level (only level 0 with
/// hardware mipmaps): lock for read, size dword (`pitch * height`, compressed `pitch * (height >> 2)`
/// from `GetLevelDesc`), the pixels in 0x400-byte pieces, unlock. Any failed write returns 0 and leaves
/// a locked raster/palette locked (kept). The original's running byte count is dead and not kept.
#[hook(addr = 0x4CD4D0, conv = cdecl, name = "_rwD3D9NativeTextureWrite", expect = "81 EC 88 00 00 00")]
pub fn d3d9_native_texture_write(s: GPtr<u8>, t: GPtr<u8>, _flags: u32) -> u32 {
    let mut size: u32 = 0;
    native_texture_get_size_call(GPtr::from_raw(&mut size), t, 0);
    if write_chunk_header(s, 1, size.wrapping_sub(0xC), 0x36003, 0xFFFF).is_null() {
        return 0;
    }
    let fa = rd::<u32>(t, 0x50);
    let mut hdr = NativeHeader {
        platform: 9,
        filter_addressing: (((((fa >> 12) & 0xF) << 4) | ((fa >> 8) & 0xF)) << 8) | (fa & 0xFF),
        name: [0; 8],
        mask: [0; 8],
    };
    for i in 0..8 {
        hdr.name[i] = rd::<u32>(t, 0x10 + i as isize * 4);
    }
    for i in 0..8 {
        hdr.mask[i] = rd::<u32>(t, 0x30 + i as isize * 4);
    }
    if rw_stream_write(s, GPtr::from_raw(&mut hdr).cast(), 0x48).is_null() {
        return 0;
    }
    let r = p8(rd::<u32>(t, 0));
    let e = ext(r);
    let mut info = NativeRasterInfo {
        width: rd::<u16>(r, 0xC),
        height: rd::<u16>(r, 0x10),
        format: (rd::<u8>(r, 0x23) as u32) << 8,
        depth: rd::<u8>(r, 0x14),
        ..Default::default()
    };
    info.num_levels = rw_raster_get_num_levels_call(r) as u8;
    info.ty = rd::<u8>(r, 0x20);
    info.flags = 0;
    if rd::<u8>(e, 8) != 0 {
        info.flags = 1;
    }
    if rd::<u8>(e, 9) & 0xF != 0 {
        info.flags |= 2;
    }
    if rd::<u8>(e, 0xA) & 0xF != 0 {
        info.flags |= 4;
    }
    if rd::<u8>(e, 0xA) & 0xF0 != 0 {
        info.flags |= 8;
    }
    info.d3d_format = rd::<u32>(e, 0x18);
    if rw_stream_write(s, GPtr::from_raw(&mut info).cast(), 0x10).is_null() {
        return 0;
    }
    let pal_len = if info.format & 0x4000 != 0 {
        0x80
    } else if info.format & 0x2000 != 0 {
        0x400
    } else {
        0
    };
    if pal_len != 0 {
        let pal = rw_raster_lock_palette_call(r, 2);
        if rw_stream_write(s, p8(pal), pal_len).is_null() {
            return 0;
        }
        rw_raster_unlock_palette_call(r);
    }
    let cube = rd::<u8>(e, 9) & 0xF;
    wr::<u8>(e, 9, cube);
    let mut faces: u32 = if cube != 0 { 6 } else { 1 };
    loop {
        let levels = info.num_levels as i32;
        let mut level: i32 = 0;
        if levels != 0 {
            loop {
                let mut pixels = rw_raster_lock_call(r, level as u32, 2);
                if pixels == 0 {
                    return 0;
                }
                let mut desc = SurfaceDesc::default();
                tex_get_level_desc(p8(rd::<u32>(e, 0)), level as u32, GPtr::from_raw(&mut desc).cast());
                let pitch = rd::<u32>(e, 0x10);
                size = if rd::<u8>(e, 0xA) & 0xF0 != 0 {
                    (desc.height >> 2).wrapping_mul(pitch)
                } else {
                    pitch.wrapping_mul(desc.height)
                };
                if rw_stream_write(s, GPtr::from_raw(&mut size).cast(), 4).is_null() {
                    return 0;
                }
                let mut n = size >> 10;
                while n != 0 {
                    n -= 1;
                    if rw_stream_write(s, p8(pixels), 0x400).is_null() {
                        return 0;
                    }
                    size = size.wrapping_sub(0x400);
                    pixels = pixels.wrapping_add(0x400);
                }
                if size != 0 && rw_stream_write(s, p8(pixels), size).is_null() {
                    return 0;
                }
                rw_raster_unlock_call(r);
                if rd::<u8>(e, 0xA) & 0xF != 0 {
                    break;
                }
                level += 1;
                if level >= info.num_levels as i32 {
                    break;
                }
            }
        }
        let b = rd::<u8>(e, 9);
        let c = b & 0xF;
        if c != 0 {
            wr::<u8>(e, 9, ((b & 0xF0).wrapping_add(0x10) & 0xF0) | c);
        }
        faces -= 1;
        if faces == 0 {
            break;
        }
    }
    1
}

/// `RwD3D9RasterIsCompressed(raster)` (0x820C90): the compressed nibble of `E+0xA` as 0/1.
#[hook(addr = 0x820C90, conv = cdecl, name = "RwD3D9RasterIsCompressed", expect = "8B 44 24 04 8B 0D E0 E9 B4 00")]
pub fn d3d9_raster_is_compressed(r: GPtr<u8>) -> u32 {
    (rd::<u8>(ext(r), 0xA) & 0xF0 != 0) as u32
}

orig! {
    /// `_rwD3D9CheckValidTextureFormat(fmt)` (0x4CBDE0)
    fn check_valid_texture_format_call(fmt: u32) -> u32 = cdecl 0x4CBDE0 => d3d9_check_valid_texture_format;
    /// `_rwD3D9CheckAutoMipmapGenCubeTextureFormat(fmt)` (0x4CC020)
    fn check_auto_mipmap_cube_call(fmt: u32) -> u32 = cdecl 0x4CC020 => d3d9_check_auto_mipmap_gen_cube_texture_format;
    /// `FUN_004cbf80(fmt)`: the AUTOGENMIPMAP texture-format check (0x4CBF80, entered through 5 nops
    /// and a protection stub)
    fn check_auto_mipmap_texture_4cbf80(fmt: u32) -> u32 = cdecl 0x4CBF80 => unported;
    /// `D3DResourceSystem::CreateTexture(w, h, levels, fmt, ext)` (0x730510)
    fn d3d_resource_create_texture(w: u32, h: u32, levels: u32, fmt: u32, ext: GPtr<u8>) -> i32 = cdecl 0x730510 => unported;
    /// `_rwD3D9RasterCreateFromDDSFormat(w, h, d3dFormat, flags)` (0x4CD050)
    fn raster_create_from_dds_format(w: u32, h: u32, fmt: u32, flags: u32) -> GPtr<u8> = cdecl 0x4CD050 => unported;
    /// `RwRasterCreate(w, h, depth, flags)` (0x7FB230)
    fn rw_raster_create_call(w: u32, h: u32, depth: u32, flags: u32) -> GPtr<u8> = cdecl 0x7FB230 => rw_raster_create_u;
    /// `RwRasterDestroy(raster)` (0x7FB020)
    fn rw_raster_destroy_call(r: GPtr<u8>) -> u32 = cdecl 0x7FB020 => rw_raster_destroy;
    /// `RwTextureCreate(raster)` (0x7F37C0)
    fn rw_texture_create(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F37C0 => unported;
    /// `RwTextureSetName(texture, name)` (0x7F38A0)
    fn rw_texture_set_name(t: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F38A0 => unported;
    /// `RwTextureSetMaskName(texture, name)` (0x7F3910)
    fn rw_texture_set_mask_name(t: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3910 => unported;
    /// `IDirect3DDevice9::CreateCubeTexture(edge, levels, usage, fmt, pool, out, shared)` (+0x64)
    fn dev_create_cube_texture(dev: GPtr<u8>, edge: u32, levels: u32, usage: u32, fmt: u32, pool: u32, out: GPtr<u8>, shared: u32) -> i32 = stdcall vslot 0x64 => unported;
}

/// The phase B/C path of `rw_raster_create_call` (unsigned arguments).
#[cfg_attr(sa_phase_a, allow(dead_code))]
fn rw_raster_create_u(w: u32, h: u32, depth: u32, flags: u32) -> GPtr<u8> {
    rw_raster_create(w as i32, h as i32, depth as i32, flags)
}

/// The cube-texture creation shared by the two cube paths of `_rwD3D9NativeTextureRead`: levels check
/// against the mipmap caps (0xC9BF44 & 0x30000), cube nibble, optional hardware mipmap generation,
/// `CreateCubeTexture(width, levels, usage, fmt, MANAGED, &E.texture, NULL)`.
fn native_read_cube(r: GPtr<u8>, e: GPtr<u8>, levels: u32, format: u32, fmt: u32) -> bool {
    if g::<u32>(0xC9BF44).read() & 0x30000 == 0 && levels > 1 {
        return false;
    }
    wr::<u8>(e, 9, (rd::<u8>(e, 9) & 0xF1) | 1);
    if format & 0x9000 == 0x9000 && check_auto_mipmap_cube_call(fmt) != 0 {
        wr::<u8>(e, 0xA, (rd::<u8>(e, 0xA) & 0xF1) | 1);
    }
    let usage = if rd::<u8>(e, 0xA) & 0xF != 0 { 0x400 } else { 0 };
    dev_create_cube_texture(device(), rd::<u32>(r, 0xC), levels, usage, fmt, 1, e, 0) >= 0
}

/// `_rwD3D9NativeTextureRead(stream, &texture, size)` (0x4CD820), `stdFunc[25]`: struct chunk
/// (version 0x34000..=0x36003), the 0x48-byte header (platform 9), the 0x10-byte raster info; creates
/// the raster (compressed: valid format check, `RwRasterCreate(.., type | 0x80 | format)` then a cube
/// texture or `D3DResourceSystem::CreateTexture`; cube: `RwRasterCreate` + cube texture; plain:
/// `RwRasterCreate(type | format)` or, for formats with only the 0x9000 bits,
/// `_rwD3D9RasterCreateFromDDSFormat`), checks the result against the info (hardware mipmaps, format,
/// D3D format, size), reads the palette and per face / level the size dword and the pixels (the raster
/// is locked for write without a NULL check, kept), then creates the texture with the header's filter
/// byte and addressing nibbles, name and mask. Any failure destroys the raster and returns 0.
#[hook(addr = 0x4CD820, conv = cdecl, name = "_rwD3D9NativeTextureRead", expect = "83 EC 70 53 55")]
pub fn d3d9_native_texture_read(s: GPtr<u8>, out: GPtr<u32>, _size: u32) -> u32 {
    let mut ver: u32 = 0;
    let mut len: u32 = 0;
    if rw_stream_find_chunk(s, 1, GPtr::from_raw(&mut len), GPtr::from_raw(&mut ver)) == 0 {
        return 0;
    }
    if !(0x34000..=0x36003).contains(&ver) {
        return 0;
    }
    let mut hdr = NativeHeader {
        platform: 0,
        filter_addressing: 0,
        name: [0; 8],
        mask: [0; 8],
    };
    if rw_stream_read(s, GPtr::from_raw(&mut hdr).cast(), 0x48) != 0x48 || hdr.platform != 9 {
        return 0;
    }
    let mut info = NativeRasterInfo::default();
    if rw_stream_read(s, GPtr::from_raw(&mut info).cast(), 0x10) != 0x10 {
        return 0;
    }
    let format = info.format;
    let fmt = info.d3d_format;
    let (w, h) = (info.width as u32, info.height as u32);
    let ty = info.ty as u32;
    let r: GPtr<u8>;
    let e: GPtr<u8>;
    if info.flags & 8 != 0 {
        if check_valid_texture_format_call(fmt) == 0 {
            return 0;
        }
        r = rw_raster_create_call(w, h, info.depth as u32, ty | 0x80 | format);
        if r.is_null() {
            return 0;
        }
        e = ext(r);
        let levels = if (rd::<u8>(r, 0x23) as i8) < 0 { info.num_levels as u32 } else { 1 };
        if info.flags & 2 != 0 {
            if !native_read_cube(r, e, levels, format, fmt) {
                rw_raster_destroy_call(r);
                return 0;
            }
            wr::<u8>(r, 0x21, rd::<u8>(r, 0x21) & 0x7F);
            let c = (rd::<u8>(e, 9) & 0xF1) | 1;
            wr::<u8>(e, 8, info.flags & 1);
            wr::<u8>(e, 9, c);
        } else {
            if format & 0x9000 == 0x9000 && check_auto_mipmap_texture_4cbf80(fmt) != 0 {
                wr::<u8>(e, 0xA, (rd::<u8>(e, 0xA) & 0xF1) | 1);
            }
            if d3d_resource_create_texture(rd::<u32>(r, 0xC), rd::<u32>(r, 0x10), levels, fmt, e) < 0 {
                rw_raster_destroy_call(r);
                return 0;
            }
            wr::<u8>(r, 0x21, rd::<u8>(r, 0x21) & 0x7F);
            wr::<u8>(e, 8, info.flags & 1);
        }
        let a = (rd::<u8>(e, 0xA) & 0xF) | 0x10;
        wr::<u32>(e, 0x18, fmt);
        wr::<u8>(e, 0xA, a);
    } else if info.flags & 2 != 0 {
        r = rw_raster_create_call(w, h, info.depth as u32, ty | 0x80 | format);
        if r.is_null() {
            return 0;
        }
        e = ext(r);
        let levels = if (rd::<u8>(r, 0x23) as i8) < 0 { info.num_levels as u32 } else { 1 };
        if !native_read_cube(r, e, levels, format, fmt) {
            rw_raster_destroy_call(r);
            return 0;
        }
        wr::<u8>(r, 0x21, rd::<u8>(r, 0x21) & 0x7F);
        wr::<u8>(e, 8, info.flags & 1);
    } else if format & 0xFFFF_6FFF != 0 {
        r = rw_raster_create_call(w, h, info.depth as u32, ty | format);
        if r.is_null() {
            return 0;
        }
        e = ext(r);
        wr::<u8>(e, 0xA, rd::<u8>(e, 0xA) & 0xF);
    } else {
        r = raster_create_from_dds_format(w, h, fmt, (format & 0x9000) | ty);
        if r.is_null() {
            return 0;
        }
        e = ext(r);
    }
    let fail = |r: GPtr<u8>| {
        rw_raster_destroy_call(r);
        0
    };
    if info.flags & 4 != 0 && rd::<u8>(e, 0xA) & 0xF == 0 {
        return fail(r);
    }
    if format != (rd::<u8>(r, 0x23) as u32) << 8
        || fmt != rd::<u32>(e, 0x18)
        || w != rd::<u32>(r, 0xC)
        || h != rd::<u32>(r, 0x10)
    {
        return fail(r);
    }
    let pal_len = if format & 0x4000 != 0 {
        0x80
    } else if format & 0x2000 != 0 {
        0x400
    } else {
        0
    };
    if pal_len != 0 {
        let pal = rw_raster_lock_palette_call(r, 1);
        let n = rw_stream_read(s, p8(pal), pal_len);
        rw_raster_unlock_palette_call(r);
        if n != pal_len {
            rw_raster_destroy_call(r);
            return 0;
        }
    }
    let cf = rd::<u8>(r, 0x23);
    let automip = cf & 0x10;
    wr::<u8>(r, 0x23, !automip & cf);
    let face = rd::<u8>(e, 9) & 0xF;
    let mut faces: u32 = if face != 0 { 6 } else { 1 };
    let levels: u32 = if rd::<u8>(e, 0xA) & 0xF != 0 { 1 } else { info.num_levels as u32 };
    wr::<u8>(e, 9, face);
    loop {
        let mut level: u32 = 0;
        while level < levels {
            let pixels = rw_raster_lock_call(r, level, 1);
            let mut size: u32 = 0;
            if rw_stream_read(s, GPtr::from_raw(&mut size).cast(), 4) != 4 {
                rw_raster_unlock_call(r);
                rw_raster_destroy_call(r);
                return 0;
            }
            if rw_stream_read(s, p8(pixels), size) != size {
                rw_raster_unlock_call(r);
                rw_raster_destroy_call(r);
                return 0;
            }
            rw_raster_unlock_call(r);
            level += 1;
        }
        let b = rd::<u8>(e, 9);
        let c = b & 0xF;
        if c != 0 {
            wr::<u8>(e, 9, ((b & 0xF0).wrapping_add(0x10) & 0xF0) | c);
        }
        faces -= 1;
        if faces == 0 {
            break;
        }
    }
    wr::<u8>(r, 0x23, rd::<u8>(r, 0x23) | automip);
    let t = rw_texture_create(r);
    if t.is_null() {
        rw_raster_destroy_call(r);
        return 0;
    }
    wr::<u8>(t, 0x50, hdr.filter_addressing as u8);
    let hf = hdr.filter_addressing;
    let fa = rd::<u32>(t, 0x50);
    let fa = fa ^ ((((hf as i32 >> 8) << 8) as u32 ^ fa) & 0xF00);
    wr::<u32>(t, 0x50, fa);
    let fa = fa ^ ((((hf as i32 >> 12) << 12) as u32 ^ fa) & 0xF000);
    wr::<u32>(t, 0x50, fa);
    rw_texture_set_name(t, GPtr::from_raw(&mut hdr.name).cast());
    rw_texture_set_mask_name(t, GPtr::from_raw(&mut hdr.mask).cast());
    out.write(a32(t));
    1
}

/// The default find callback's name test: case-insensitive for 'a'..='z' (signed byte compares), the
/// terminating position compared raw.
fn tex_name_matches(tname: GPtr<u8>, name: GPtr<u8>) -> bool {
    let up = |c: u8| {
        if (c as i8) >= 0x61 && (c as i8) <= 0x7A {
            c.wrapping_add(0xE0)
        } else {
            c
        }
    };
    let mut i: isize = 0;
    loop {
        let a = rd::<u8>(tname, i);
        if a == 0 {
            break;
        }
        let b = rd::<u8>(name, i);
        if b == 0 {
            break;
        }
        if up(a) != up(b) {
            return false;
        }
        i += 1;
    }
    rd::<u8>(tname, i) == rd::<u8>(name, i)
}

/// `_rwTextureDefaultFindCallBack(name)` (0x7F53D0): with a current dictionary only that dictionary is
/// searched; otherwise every dictionary in the module list. A match whose texture pointer is NULL
/// (link at 8) abandons the rest of that dictionary (kept). Returns the texture or NULL.
#[hook(addr = 0x7F53D0, conv = cdecl, name = "_rwTextureDefaultFindCallBack", expect = "83 EC 08 A1 24 7B C9 00")]
pub fn rw_texture_default_find_callback(name: GPtr<u8>) -> GPtr<u8> {
    let m = tmod();
    let cur = p8(rd::<u32>(m, 0x10));
    if !cur.is_null() {
        let head = cur.byte_add(8);
        let mut l = p8(rd::<u32>(cur, 8));
        while l != head {
            let tex = l.byte_add(-8);
            let tname = tex.byte_add(0x10);
            if !tname.is_null() && tex_name_matches(tname, name) {
                return tex;
            }
            l = p8(rd::<u32>(l, 0));
        }
        return GPtr::NULL;
    }
    let mut d = p8(rd::<u32>(m, 0));
    while d != m {
        let head = d.byte_add(-8);
        let mut l = p8(rd::<u32>(head, 0));
        while l != head {
            let tex = l.byte_add(-8);
            let tname = tex.byte_add(0x10);
            if !tname.is_null() && tex_name_matches(tname, name) {
                if !tex.is_null() {
                    return tex;
                }
                break;
            }
            l = p8(rd::<u32>(l, 0));
        }
        d = p8(rd::<u32>(d, 0));
    }
    GPtr::NULL
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RW_RASTER_GET_PLUGIN_OFFSET,
    HOOK_RW_RASTER_GET_CURRENT_CONTEXT,
    HOOK_RW_RASTER_RENDER_SCALED,
    HOOK_RW_RASTER_GET_OFFSET,
    HOOK_RW_RASTER_CLEAR,
    HOOK_RW_RASTER_RENDER_FAST,
    HOOK_RW_RASTER_CLEAR_RECT,
    HOOK_RW_RASTER_RENDER,
    HOOK_RW_RASTER_UNLOCK_PALETTE,
    HOOK_RW_RASTER_DESTROY,
    HOOK_RW_RASTER_PUSH_CONTEXT,
    HOOK_RW_RASTER_REGISTER_PLUGIN,
    HOOK_RW_RASTER_LOCK_PALETTE,
    HOOK_RW_RASTER_POP_CONTEXT,
    HOOK_RW_RASTER_SHOW_RASTER,
    HOOK_RW_RASTER_SUB_RASTER,
    HOOK_RASTER_MODULE_CLOSE,
    HOOK_RW_RASTER_SET_FREE_LIST_CREATE_PARAMS,
    HOOK_RASTER_MODULE_OPEN,
    HOOK_D3D9_RASTER_CTOR,
    HOOK_D3D9_RASTER_DTOR,
    HOOK_D3D9_RASTER_PLUGIN_ATTACH,
    HOOK_D3D9_RASTER_HAS_ALPHA,
    HOOK_D3D9_RASTER_LOCK,
    HOOK_D3D9_RASTER_UNLOCK,
    HOOK_D3D9_RASTER_LOCK_PALETTE,
    HOOK_D3D9_RASTER_UNLOCK_PALETTE,
    HOOK_D3D9_SET_RASTER_CONTEXT,
    HOOK_VM_RASTER_LIST_ADD,
    HOOK_VM_RASTER_LIST_DESTROY,
    HOOK_VM_RASTER_LIST_RELEASE,
    HOOK_D3D9_RASTER_GET_MIP_LEVELS,
    HOOK_D3D9_TEXTURE_SET_RASTER,
    HOOK_D3D9_RASTER_SUB_RASTER,
    HOOK_D3D9_CHECK_VALID_ZBUFFER_FORMAT,
    HOOK_D3D9_CHECK_VALID_TEXTURE_FORMAT,
    HOOK_D3D9_CHECK_VALID_CAMERA_TEXTURE_FORMAT,
    HOOK_D3D9_CHECK_VALID_ZBUFFER_TEXTURE_FORMAT,
    HOOK_D3D9_CHECK_VALID_CUBE_TEXTURE_FORMAT,
    HOOK_D3D9_CHECK_VALID_CAMERA_CUBE_TEXTURE_FORMAT,
    HOOK_D3D9_CHECK_AUTO_MIPMAP_GEN_TEXTURE_FORMAT,
    HOOK_D3D9_CHECK_AUTO_MIPMAP_GEN_CUBE_TEXTURE_FORMAT,
    HOOK_VM_RASTER_LIST_CREATE,
    HOOK_D3D9_RASTER_CLOSE,
    HOOK_RW_TEXTURE_VALIDATE_PLUGINS,
    HOOK_RW_TEXTURE_SET_MIPMAP_GENERATION_CALLBACK,
    HOOK_RW_TEXTURE_GET_MIPMAP_GENERATION_CALLBACK,
    HOOK_RW_TEXTURE_SET_MIPMAP_NAME_CALLBACK,
    HOOK_RW_TEXTURE_GET_MIPMAP_NAME_CALLBACK,
    HOOK_RW_TEXTURE_RASTER_GENERATE_MIPMAPS_PORT,
    HOOK_RW_TEXTURE_SET_STREAM_ALWAYS_CALLBACK,
    HOOK_RW_TEXTURE_STREAM_GET_SIZE,
    HOOK_RW_TEXTURE_STREAM_WRITE,
    HOOK_RW_TEXTURE_CHUNK_INFO_READ,
    HOOK_ANISOT_CTOR,
    HOOK_ANISOT_DTOR,
    HOOK_ANISOT_COPY,
    HOOK_ANISOT_STREAM_READ,
    HOOK_ANISOT_STREAM_WRITE,
    HOOK_ANISOT_STREAM_GET_SIZE,
    HOOK_D3D9_NATIVE_TEXTURE_GET_SIZE,
    HOOK_D3D9_NATIVE_TEXTURE_WRITE,
    HOOK_D3D9_RASTER_IS_COMPRESSED,
    HOOK_D3D9_NATIVE_TEXTURE_READ,
    HOOK_RW_TEXTURE_DEFAULT_FIND_CALLBACK,
    HOOK_RW_RASTER_UNLOCK,
    HOOK_RW_RASTER_GET_NUM_LEVELS,
    HOOK_RW_RASTER_CREATE,
    HOOK_RW_RASTER_LOCK,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
