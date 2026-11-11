//! PORT-RW-4 module `d3d9dev`: the RW D3D9 device wrappers: FVF / vertex declaration / shader / stream /
//! index caches, draw calls, transform and light caches, vertex declarations, shaders and constants,
//! render targets, the frustum tests, the engine multisampling / refresh-rate settings and the device
//! cache reset (see docs/arch/rw-port.md, PORT-RW-4). Ported from the 1.0 US bytes (0x7F5EF0..0x7FAE30);
//! `docs/re/specs/RW/RwD3D9Device*.md` as a reading aid. D3D9 calls go through the COM vtables exactly
//! as the original makes them.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, p8, rd, rw_free, rw_malloc, rw_memory_alloc, rw_memory_free, rw_realloc, wr, x};
use crate::d3d9state::device;

/// `IDirect3D9*` of the driver.
const D3D9: usize = 0xC97C20;
const ADAPTER: usize = 0xC97C24;
const DEVICE_TYPE: usize = 0x8E2428;
/// Display-mode format of the adapter (`D3DPRESENT_PARAMETERS.BackBufferFormat` copy).
const ADAPTER_FORMAT: usize = 0xC9BEF0;
const PRESENT_WINDOWED: usize = 0xC9BEF8;
/// Caps copy (`RwD3D9GetCaps`).
const CAPS: u32 = 0xC9BF00;

const LAST_FVF: usize = 0x8E2440;
const LAST_DECL: usize = 0x8E2444;
const LAST_VS: usize = 0x8E2448;
const LAST_PS: usize = 0x8E244C;
const LAST_IB: usize = 0x8E2450;
/// Stream sources 4 x `{vb, offset, stride, -}`.
const STREAMS: u32 = 0xC97BD8;
/// Render-target cache (4) and depth-stencil cache.
const RENDER_TARGETS: u32 = 0xC98090;
const DEPTH_STENCIL: usize = 0xC9808C;
const STENCIL_CLEAR: usize = 0xC97C44;
/// Transform cache: one freelist block pointer per D3DTRANSFORMSTATETYPE (0x104 slots).
const TRANSFORMS: u32 = 0xC97C70;
const TRANSFORM_FL: usize = 0xC98080;
/// The cached world matrix (block from the transform freelist) and "world is identity".
const WORLD_CACHE: usize = 0xC98070;
const WORLD_IDENTITY: usize = 0xC97C68;
/// The identity D3DMATRIX (`.rdata`).
const IDENTITY: u32 = 0x8847A0;
/// D3DMATRIX staging for `RwD3D9SetTransformWorld`.
const WORLD_STAGING: u32 = 0x8E24D0;
/// Light cache: count, array of 0x6C-byte entries (D3DLIGHT9 + enabled flag at +0x68).
const LIGHT_COUNT: usize = 0xC98084;
const LIGHTS: usize = 0xC98088;
/// Vertex-declaration cache: count, capacity, array of `{decl, numElements, elements*}`.
const DECL_COUNT: usize = 0xC97C58;
const DECL_CAPACITY: usize = 0xC97C5C;
const DECLS: usize = 0xC97C60;
const RESTORE_CALLBACK: usize = 0xC980B0;
const MULTITHREAD_SAFE: usize = 0xC97C4C;
const SOFTWARE_VP: usize = 0xC97C50;
const MAX_MULTISAMPLE: usize = 0x8E242C;
const MULTISAMPLE: usize = 0x8E2430;
const MULTISAMPLE_QUALITY_SCALE: usize = 0x8E2434;
const MULTISAMPLE_QUALITY: usize = 0x8E2438;
const REFRESH_RATE: usize = 0x8E243C;
/// The three adapter formats `RwD3D9EngineSetRefreshRate` enumerates.
const MODE_FORMATS: u32 = 0x884788;
/// D3D9 raster extension offset inside RwRaster.
const RASTER_EXT_OFFSET: usize = 0xB4E9E0;

orig! {
    /// `IDirect3DDevice9::SetFVF` (+0x164)
    pub fn dev_set_fvf(dev: GPtr<u8>, fvf: u32) -> i32 = stdcall vslot 0x164 => unported;
    /// `IDirect3DDevice9::SetVertexDeclaration` (+0x15C)
    pub fn dev_set_vertex_declaration(dev: GPtr<u8>, decl: u32) -> i32 = stdcall vslot 0x15C => unported;
    /// `IDirect3DDevice9::CreateVertexDeclaration` (+0x158)
    pub fn dev_create_vertex_declaration(dev: GPtr<u8>, elements: u32, out: GPtr<u32>) -> i32 = stdcall vslot 0x158 => unported;
    /// `IDirect3DDevice9::SetVertexShader` (+0x170)
    pub fn dev_set_vertex_shader(dev: GPtr<u8>, vs: u32) -> i32 = stdcall vslot 0x170 => unported;
    /// `IDirect3DDevice9::CreateVertexShader` (+0x16C)
    pub fn dev_create_vertex_shader(dev: GPtr<u8>, func: u32, out: u32) -> i32 = stdcall vslot 0x16C => unported;
    /// `IDirect3DDevice9::SetVertexShaderConstantF` (+0x178)
    pub fn dev_set_vs_constant(dev: GPtr<u8>, reg: u32, data: u32, count: u32) -> i32 = stdcall vslot 0x178 => unported;
    /// `IDirect3DDevice9::CreatePixelShader` (+0x1A8)
    pub fn dev_create_pixel_shader(dev: GPtr<u8>, func: u32, out: u32) -> i32 = stdcall vslot 0x1A8 => unported;
    /// `IDirect3DDevice9::SetPixelShader` (+0x1AC)
    pub fn dev_set_pixel_shader(dev: GPtr<u8>, ps: u32) -> i32 = stdcall vslot 0x1AC => unported;
    /// `IDirect3DDevice9::SetPixelShaderConstantF` (+0x1B4)
    pub fn dev_set_ps_constant(dev: GPtr<u8>, reg: u32, data: u32, count: u32) -> i32 = stdcall vslot 0x1B4 => unported;
    /// `IDirect3DDevice9::SetStreamSource` (+0x190)
    pub fn dev_set_stream_source(dev: GPtr<u8>, stream: u32, vb: u32, offset: u32, stride: u32) -> i32 = stdcall vslot 0x190 => unported;
    /// `IDirect3DDevice9::SetIndices` (+0x1A0)
    pub fn dev_set_indices(dev: GPtr<u8>, ib: u32) -> i32 = stdcall vslot 0x1A0 => unported;
    /// `IDirect3DDevice9::DrawPrimitive` (+0x144)
    pub fn dev_draw_primitive(dev: GPtr<u8>, prim: u32, start: u32, count: u32) -> i32 = stdcall vslot 0x144 => unported;
    /// `IDirect3DDevice9::DrawIndexedPrimitive` (+0x148)
    pub fn dev_draw_indexed_primitive(dev: GPtr<u8>, prim: u32, base: u32, min: u32, num: u32, start: u32, count: u32) -> i32 = stdcall vslot 0x148 => unported;
    /// `IDirect3DDevice9::DrawPrimitiveUP` (+0x14C)
    pub fn dev_draw_primitive_up(dev: GPtr<u8>, prim: u32, count: u32, data: u32, stride: u32) -> i32 = stdcall vslot 0x14C => unported;
    /// `IDirect3DDevice9::DrawIndexedPrimitiveUP` (+0x150)
    pub fn dev_draw_indexed_primitive_up(dev: GPtr<u8>, prim: u32, min: u32, num: u32, count: u32, indices: u32, fmt: u32, data: u32, stride: u32) -> i32 = stdcall vslot 0x150 => unported;
    /// `IDirect3DDevice9::SetTransform` (+0xB0)
    pub fn dev_set_transform(dev: GPtr<u8>, state: u32, m: u32) -> i32 = stdcall vslot 0xB0 => unported;
    /// `IDirect3DDevice9::SetLight` (+0xCC)
    pub fn dev_set_light(dev: GPtr<u8>, index: u32, light: u32) -> i32 = stdcall vslot 0xCC => unported;
    /// `IDirect3DDevice9::LightEnable` (+0xD4)
    pub fn dev_light_enable(dev: GPtr<u8>, index: u32, enable: u32) -> i32 = stdcall vslot 0xD4 => unported;
    /// `IDirect3DDevice9::SetRenderTarget` (+0x94)
    pub fn dev_set_render_target(dev: GPtr<u8>, index: u32, surf: u32) -> i32 = stdcall vslot 0x94 => unported;
    /// `IDirect3DDevice9::SetDepthStencilSurface` (+0x9C)
    pub fn dev_set_depth_stencil_surface(dev: GPtr<u8>, surf: u32) -> i32 = stdcall vslot 0x9C => unported;
    /// `IDirect3DTexture9::GetSurfaceLevel` (+0x48)
    pub fn tex_get_surface_level(tex: GPtr<u8>, level: u32, out: GPtr<u32>) -> i32 = stdcall vslot 0x48 => unported;
    /// `IDirect3DCubeTexture9::GetCubeMapSurface` (+0x48)
    pub fn cube_get_cube_map_surface(tex: GPtr<u8>, face: u32, level: u32, out: GPtr<u32>) -> i32 = stdcall vslot 0x48 => unported;
    /// `IUnknown::AddRef` (+4)
    pub fn com_add_ref(obj: GPtr<u8>) -> u32 = stdcall vslot 0x4 => unported;
    /// `IUnknown::Release` (+8)
    pub fn com_release(obj: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;
    /// `IDirect3D9::GetAdapterModeCount` (+0x18)
    pub fn d3d_get_adapter_mode_count(d3d: GPtr<u8>, adapter: u32, fmt: u32) -> u32 = stdcall vslot 0x18 => unported;
    /// `IDirect3D9::EnumAdapterModes` (+0x1C)
    pub fn d3d_enum_adapter_modes(d3d: GPtr<u8>, adapter: u32, fmt: u32, mode: u32, out: GPtr<u8>) -> i32 = stdcall vslot 0x1C => unported;
    /// `IDirect3D9::CheckDeviceFormat` (+0x28)
    pub fn d3d_check_device_format(d3d: GPtr<u8>, adapter: u32, dev_type: u32, adapter_fmt: u32, usage: u32, rtype: u32, fmt: u32) -> i32 = stdcall vslot 0x28 => unported;
    /// `IDirect3D9::CheckDeviceMultiSampleType` (+0x2C)
    pub fn d3d_check_multisample(d3d: GPtr<u8>, adapter: u32, dev_type: u32, fmt: u32, windowed: u32, ty: u32, quality: u32) -> i32 = stdcall vslot 0x2C => unported;

    /// `_rwD3D9RenderStateFlushCache()` (0x7FC200)
    pub fn flush_cache_call() = cdecl 0x7FC200 => crate::d3d9state::flush_cache;
    /// `_rwD3D9LightCompare(a, b)` (0x7FA8B0)
    pub fn light_compare_call(a: GPtr<u8>, b: GPtr<u8>) -> u32 = cdecl 0x7FA8B0 => light_compare;
    /// `_rwFreeListCreate(entrySize, entriesPerBlock, alignment, hint)` (0x801980)
    pub fn free_list_create(size: u32, per_block: u32, align: u32, hint: u32) -> GPtr<u8> = cdecl 0x801980 => unported;
    /// `RwFreeListPurge(fl)` (0x801E00)
    pub fn free_list_purge(fl: GPtr<u8>) -> u32 = cdecl 0x801E00 => unported;
}

#[inline]
fn gu(addr: usize) -> u32 {
    g::<u32>(addr).read()
}

#[inline]
fn su(addr: usize, v: u32) {
    g::<u32>(addr).write(v)
}

#[inline]
fn at(base: u32, off: u32) -> GPtr<u32> {
    g::<u32>(base.wrapping_add(off) as usize)
}

fn d3d9() -> GPtr<u8> {
    p8(gu(D3D9))
}

/// Copies `n` dwords.
fn copy_dwords(dst: GPtr<u8>, src: GPtr<u8>, n: isize) {
    for i in 0..n {
        wr::<u32>(dst, i * 4, rd::<u32>(src, i * 4));
    }
}

fn dwords_equal(a: GPtr<u8>, b: GPtr<u8>, n: isize) -> bool {
    (0..n).all(|i| rd::<u32>(a, i * 4) == rd::<u32>(b, i * 4))
}

/// `_rwD3D9SetDepthStencilSurface` (0x7F5EF0): `SetDepthStencilSurface(surf)` when it changes.
#[hook(addr = 0x7F5EF0, conv = cdecl, name = "_rwD3D9SetDepthStencilSurface", expect = "8B 44 24 04 8B 0D 8C 80 C9 00")]
pub fn set_depth_stencil_surface(surf: u32) {
    if gu(DEPTH_STENCIL) != surf {
        su(DEPTH_STENCIL, surf);
        dev_set_depth_stencil_surface(device(), surf);
    }
}

/// `_rwD3D9SetRenderTarget` (0x7F5F20): `SetRenderTarget(index, surf)` when it changes; `hr >= 0`.
#[hook(addr = 0x7F5F20, conv = cdecl, name = "_rwD3D9SetRenderTarget", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn set_render_target_surface(index: u32, surf: u32) -> u32 {
    let c = at(RENDER_TARGETS, index.wrapping_mul(4));
    if c.read() == surf {
        return 1;
    }
    c.write(surf);
    (dev_set_render_target(device(), index, surf) >= 0) as u32
}

/// `RwD3D9EngineGetMaxMultiSamplingLevels` (0x7F84E0).
#[hook(addr = 0x7F84E0, conv = cdecl, name = "RwD3D9EngineGetMaxMultiSamplingLevels", expect = "A1 2C 24 8E 00 C3")]
pub fn engine_get_max_multisampling_levels() -> u32 {
    gu(MAX_MULTISAMPLE)
}

/// `RwD3D9EngineSetMultiSamplingLevels` (0x7F84F0): clamps to the maximum; below 2: 1 (returns 1);
/// else steps the level down while `CheckDeviceMultiSampleType` fails (to 2 at least) and returns the
/// quality `scale * requested / max` (u32 arithmetic) it also stores.
#[hook(addr = 0x7F84F0, conv = cdecl, name = "RwD3D9EngineSetMultiSamplingLevels", expect = "A1 2C 24 8E 00 57")]
pub fn engine_set_multisampling_levels(levels: u32) -> u32 {
    let max = gu(MAX_MULTISAMPLE);
    let lv = if levels > max { max } else { levels };
    su(MULTISAMPLE, lv);
    if lv < 2 {
        su(MULTISAMPLE, 1);
        su(MULTISAMPLE_QUALITY, 1);
        return 1;
    }
    loop {
        let windowed = !gu(PRESENT_WINDOWED) & 1;
        let hr = d3d_check_multisample(
            d3d9(),
            gu(ADAPTER),
            gu(DEVICE_TYPE),
            gu(ADAPTER_FORMAT),
            windowed,
            gu(MULTISAMPLE),
            0,
        );
        if hr >= 0 {
            break;
        }
        let n = gu(MULTISAMPLE).wrapping_sub(1);
        su(MULTISAMPLE, n);
        if n < 2 {
            break;
        }
    }
    let q = gu(MULTISAMPLE_QUALITY_SCALE).wrapping_mul(lv) / gu(MAX_MULTISAMPLE);
    su(MULTISAMPLE_QUALITY, q);
    q
}

/// `RwD3D9EngineSetRefreshRate` (0x7F8580): stores the rate; when non-zero, enumerates the modes of the
/// three formats at 0x884788 (from the last mode down) for an exact match, else keeps the highest rate
/// below it, and stores that when it differs.
#[hook(addr = 0x7F8580, conv = cdecl, name = "RwD3D9EngineSetRefreshRate", expect = "8B 44 24 04 83 EC 10")]
pub fn engine_set_refresh_rate(rate: u32) {
    su(REFRESH_RATE, rate);
    if rate == 0 {
        return;
    }
    let mut best = 0u32;
    let mut mode = [0u32; 4];
    for f in 0..3 {
        let fmt = at(MODE_FORMATS, f * 4).read();
        let mut n = d3d_get_adapter_mode_count(d3d9(), gu(ADAPTER), fmt);
        while n != 0 {
            n -= 1;
            d3d_enum_adapter_modes(d3d9(), gu(ADAPTER), fmt, n, GPtr::from_raw(mode.as_mut_ptr()).cast());
            let rr = mode[2];
            let want = gu(REFRESH_RATE);
            if rr == want {
                best = want;
                break;
            }
            if rr < want && best < rr {
                best = rr;
            }
        }
    }
    if gu(REFRESH_RATE) != best {
        su(REFRESH_RATE, best);
    }
}

/// `RwD3D9EngineSetMultiThreadSafe` (0x7F8620).
#[hook(addr = 0x7F8620, conv = cdecl, name = "RwD3D9EngineSetMultiThreadSafe", expect = "8B 44 24 04 A3 4C 7C C9 00")]
pub fn engine_set_multithread_safe(v: u32) {
    su(MULTITHREAD_SAFE, v);
}

/// `RwD3D9EngineSetSoftwareVertexProcessing` (0x7F8630).
#[hook(addr = 0x7F8630, conv = cdecl, name = "RwD3D9EngineSetSoftwareVertexProcessing", expect = "8B 44 24 04 A3 50 7C C9 00")]
pub fn engine_set_software_vertex_processing(v: u32) {
    su(SOFTWARE_VP, v);
}

/// `_rwDeviceGetHandle` (0x7F9C20): the RwDevice at 0x8E2498.
#[hook(addr = 0x7F9C20, conv = cdecl, name = "_rwDeviceGetHandle", expect = "B8 98 24 8E 00 C3")]
pub fn device_get_handle() -> u32 {
    0x8E_2498
}

/// `RwD3D9DeviceSupportsDXTTexture` (0x7F9C30): 1 when any of DXT1..DXT5 passes `CheckDeviceFormat` as a
/// texture (tried in order, stopping at the first success).
#[hook(addr = 0x7F9C30, conv = cdecl, name = "RwD3D9DeviceSupportsDXTTexture", expect = "8B 15 F0 BE C9 00")]
pub fn device_supports_dxt_texture() -> u32 {
    for fourcc in [0x3154_5844u32, 0x3254_5844, 0x3354_5844, 0x3454_5844, 0x3554_5844] {
        let hr = d3d_check_device_format(
            d3d9(),
            gu(ADAPTER),
            gu(DEVICE_TYPE),
            gu(ADAPTER_FORMAT),
            0,
            3,
            fourcc,
        );
        if hr >= 0 {
            return 1;
        }
    }
    0
}

/// `RwD3D9SetStencilClear` (0x7F9D30).
#[hook(addr = 0x7F9D30, conv = cdecl, name = "RwD3D9SetStencilClear", expect = "8B 44 24 04 A3 44 7C C9 00")]
pub fn set_stencil_clear(v: u32) {
    su(STENCIL_CLEAR, v);
}

/// `RwD3D9GetStencilClear` (0x7F9D40).
#[hook(addr = 0x7F9D40, conv = cdecl, name = "RwD3D9GetStencilClear", expect = "A1 44 7C C9 00 C3")]
pub fn get_stencil_clear() -> u32 {
    gu(STENCIL_CLEAR)
}

/// `RwD3D9GetCurrentD3DDevice` (0x7F9D50; the catalogue name, the body resets the device caches):
/// render-target / depth caches 0, FVF / declaration / shader / index caches -1, streams `{-1, 0, 0}`,
/// world not identity; the transform freelist blocks freed and the list purged (or the list created and
/// the slots cleared), a fresh identity world-matrix block, light count 0 and the light array freed.
/// Returns `_RwD3DDevice`.
#[hook(addr = 0x7F9D50, conv = cdecl, name = "RwD3D9GetCurrentD3DDevice", expect = "33 C0 53 A3 90 80 C9 00")]
pub fn device_caches_reset() -> u32 {
    for i in 0..4u32 {
        at(RENDER_TARGETS, i * 4).write(0);
    }
    su(DEPTH_STENCIL, 0);
    for c in [LAST_FVF, LAST_DECL, LAST_VS, LAST_PS, LAST_IB] {
        su(c, 0xFFFF_FFFF);
    }
    for i in 0..4u32 {
        at(STREAMS, i * 0x10).write(0xFFFF_FFFF);
        at(STREAMS, i * 0x10 + 4).write(0);
        at(STREAMS, i * 0x10 + 8).write(0);
    }
    su(WORLD_IDENTITY, 0);
    if gu(TRANSFORM_FL) != 0 {
        for i in 0..0x104u32 {
            let slot = at(TRANSFORMS, i * 4);
            if slot.read() != 0 {
                rw_memory_free(p8(gu(TRANSFORM_FL)), p8(slot.read()));
                slot.write(0);
            }
        }
        free_list_purge(p8(gu(TRANSFORM_FL)));
    } else {
        su(TRANSFORM_FL, free_list_create(0x40, 0xF, 0x10, 0x30411).addr() as u32);
        for i in 0..0x104u32 {
            at(TRANSFORMS, i * 4).write(0);
        }
    }
    let w = rw_memory_alloc(p8(gu(TRANSFORM_FL)), 0x30411);
    su(WORLD_CACHE, w.addr() as u32);
    copy_dwords(w, p8(IDENTITY), 0x10);
    su(LIGHT_COUNT, 0);
    if gu(LIGHTS) != 0 {
        rw_free(p8(gu(LIGHTS)));
        su(LIGHTS, 0);
    }
    gu(crate::d3d9state::D3D_DEVICE)
}

/// `RwD3D9GetCurrentD3DRenderTarget` (0x7F9E80).
#[hook(addr = 0x7F9E80, conv = cdecl, name = "RwD3D9GetCurrentD3DRenderTarget", expect = "8B 44 24 04 8B 04 85 90 80 C9 00")]
pub fn get_current_d3d_render_target(index: u32) -> u32 {
    at(RENDER_TARGETS, index.wrapping_mul(4)).read()
}

/// `RwD3D9SetRenderTarget` (0x7F9E90): level 0 of the raster's parent texture (a cube face when the
/// extension's byte +9 has a low nibble: face = high nibble) as render target `index` when it changes
/// (`hr >= 0`, else 1); the surface reference is released afterwards.
#[hook(addr = 0x7F9E90, conv = cdecl, name = "RwD3D9SetRenderTarget", expect = "51 8B 4C 24 0C 33 C0")]
pub fn set_render_target(index: u32, raster: GPtr<u8>) -> u32 {
    let mut surf: u32 = 0;
    if !raster.is_null() {
        let ext = p8(rd::<u32>(raster, 0).wrapping_add(gu(RASTER_EXT_OFFSET)));
        let cube = rd::<u8>(ext, 9);
        let tex = p8(rd::<u32>(ext, 0));
        if cube & 0xF != 0 {
            cube_get_cube_map_surface(tex, (cube >> 4) as u32, 0, GPtr::from_raw(&mut surf));
        } else {
            tex_get_surface_level(tex, 0, GPtr::from_raw(&mut surf));
        }
    }
    let c = at(RENDER_TARGETS, index.wrapping_mul(4));
    let ok = if c.read() != surf {
        c.write(surf);
        let hr = dev_set_render_target(device(), index, surf);
        (hr >= 0) as u32
    } else {
        1
    };
    if surf != 0 {
        com_release(p8(surf));
    }
    ok
}

/// `_rwD3D9SetFVF` (0x7F9F30): on a change, `SetFVF`; the FVF cache becomes the FVF on success, -1 on
/// failure, and the declaration cache -1.
#[hook(addr = 0x7F9F30, conv = cdecl, name = "_rwD3D9SetFVF", expect = "A1 40 24 8E 00 56")]
pub fn set_fvf(fvf: u32) {
    if gu(LAST_FVF) == fvf {
        return;
    }
    let hr = dev_set_fvf(device(), fvf);
    su(LAST_FVF, 0xFFFF_FFFF);
    if hr >= 0 {
        su(LAST_FVF, fvf);
    }
    su(LAST_DECL, 0xFFFF_FFFF);
}

/// `_rwD3D9SetVertexDeclaration` (0x7F9F70): as `_rwD3D9SetFVF` with the roles swapped.
#[hook(addr = 0x7F9F70, conv = cdecl, name = "_rwD3D9SetVertexDeclaration", expect = "A1 44 24 8E 00 56")]
pub fn set_vertex_declaration(decl: u32) {
    if gu(LAST_DECL) == decl {
        return;
    }
    let hr = dev_set_vertex_declaration(device(), decl);
    su(LAST_DECL, 0xFFFF_FFFF);
    if hr >= 0 {
        su(LAST_DECL, decl);
    }
    su(LAST_FVF, 0xFFFF_FFFF);
}

/// `_rwD3D9SetVertexShader` (0x7F9FB0).
#[hook(addr = 0x7F9FB0, conv = cdecl, name = "_rwD3D9SetVertexShader", expect = "A1 48 24 8E 00 56")]
pub fn set_vertex_shader(vs: u32) {
    if gu(LAST_VS) == vs {
        return;
    }
    let hr = dev_set_vertex_shader(device(), vs);
    su(LAST_VS, 0xFFFF_FFFF);
    if hr >= 0 {
        su(LAST_VS, vs);
    }
}

/// `_rwD3D9SetPixelShader` (0x7F9FF0).
#[hook(addr = 0x7F9FF0, conv = cdecl, name = "_rwD3D9SetPixelShader", expect = "A1 4C 24 8E 00 56")]
pub fn set_pixel_shader(ps: u32) {
    if gu(LAST_PS) == ps {
        return;
    }
    let hr = dev_set_pixel_shader(device(), ps);
    su(LAST_PS, 0xFFFF_FFFF);
    if hr >= 0 {
        su(LAST_PS, ps);
    }
}

/// `RwD3D9SetStreamSource` (0x7FA030): `SetStreamSource` when the cached `{vb, offset, stride}` differs.
#[hook(addr = 0x7FA030, conv = cdecl, name = "RwD3D9SetStreamSource", expect = "53 56 57 8B 7C 24 10")]
pub fn set_stream_source(stream: u32, vb: u32, offset: u32, stride: u32) {
    let e = stream.wrapping_shl(4);
    if at(STREAMS, e).read() == vb && at(STREAMS, e + 4).read() == offset && at(STREAMS, e + 8).read() == stride {
        return;
    }
    at(STREAMS, e).write(vb);
    at(STREAMS, e + 4).write(offset);
    at(STREAMS, e + 8).write(stride);
    dev_set_stream_source(device(), stream, vb, offset, stride);
}

/// `_rwD3D9SetStreams` (0x7FA090): streams 0 and 1 from an array of `{vb, offset, stride, -}`; without
/// `use_offsets` the offset used (and compared with the cache) is 0. A NULL vb unbinds a bound stream.
#[hook(addr = 0x7FA090, conv = cdecl, name = "_rwD3D9SetStreams", expect = "8B 44 24 08 53 55 56")]
pub fn set_streams(streams: GPtr<u8>, use_offsets: u32) {
    for i in 0..2u32 {
        let s = streams.byte_add(i as isize * 0x10);
        let c = i * 0x10;
        let vb = rd::<u32>(s, 0);
        if vb == 0 {
            if at(STREAMS, c).read() != 0 {
                at(STREAMS, c).write(0);
                at(STREAMS, c + 4).write(0);
                at(STREAMS, c + 8).write(0);
                dev_set_stream_source(device(), i, 0, 0, 0);
            }
            continue;
        }
        let offset = if use_offsets != 0 { rd::<u32>(s, 4) } else { 0 };
        if at(STREAMS, c).read() == vb
            && at(STREAMS, c + 4).read() == offset
            && at(STREAMS, c + 8).read() == rd::<u32>(s, 8)
        {
            continue;
        }
        at(STREAMS, c).write(vb);
        at(STREAMS, c + 4).write(offset);
        at(STREAMS, c + 8).write(rd::<u32>(s, 8));
        dev_set_stream_source(device(), i, vb, offset, rd::<u32>(s, 8));
    }
}

/// `_rwD3D9SetIndices` (0x7FA1C0).
#[hook(addr = 0x7FA1C0, conv = cdecl, name = "_rwD3D9SetIndices", expect = "8B 44 24 04 8B 0D 50 24 8E 00")]
pub fn set_indices(ib: u32) {
    if gu(LAST_IB) != ib {
        su(LAST_IB, ib);
        dev_set_indices(device(), ib);
    }
}

/// Unbinds stream 0 and the index buffer before a `*UP` draw.
fn unbind_for_up() {
    if at(STREAMS, 0).read() != 0 || at(STREAMS, 4).read() != 0 || at(STREAMS, 8).read() != 0 {
        at(STREAMS, 0).write(0);
        at(STREAMS, 4).write(0);
        at(STREAMS, 8).write(0);
        dev_set_stream_source(device(), 0, 0, 0, 0);
    }
    if gu(LAST_IB) != 0 {
        su(LAST_IB, 0);
        dev_set_indices(device(), 0);
    }
}

/// `_rwD3D9DrawIndexedPrimitiveUP` (0x7FA1F0): unbind, flush, `DrawIndexedPrimitiveUP(.., INDEX16, ..)`.
#[hook(addr = 0x7FA1F0, conv = cdecl, name = "_rwD3D9DrawIndexedPrimitiveUP", expect = "A1 D8 7B C9 00 56")]
pub fn draw_indexed_primitive_up(prim: u32, min: u32, num: u32, count: u32, indices: u32, data: u32, stride: u32) -> i32 {
    unbind_for_up();
    flush_cache_call();
    dev_draw_indexed_primitive_up(device(), prim, min, num, count, indices, 0x65, data, stride)
}

/// `_rwD3D9DrawPrimitiveUP` (0x7FA290): unbind, flush, `DrawPrimitiveUP`.
#[hook(addr = 0x7FA290, conv = cdecl, name = "_rwD3D9DrawPrimitiveUP", expect = "A1 D8 7B C9 00 56")]
pub fn draw_primitive_up(prim: u32, count: u32, data: u32, stride: u32) -> i32 {
    unbind_for_up();
    flush_cache_call();
    dev_draw_primitive_up(device(), prim, count, data, stride)
}

/// `_rwD3D9DrawIndexedPrimitive` (0x7FA320): flush, `DrawIndexedPrimitive`.
#[hook(addr = 0x7FA320, conv = cdecl, name = "_rwD3D9DrawIndexedPrimitive", expect = "E8 DB 1E 00 00")]
pub fn draw_indexed_primitive(prim: u32, base: u32, min: u32, num: u32, start: u32, count: u32) -> i32 {
    flush_cache_call();
    dev_draw_indexed_primitive(device(), prim, base, min, num, start, count)
}

/// `_rwD3D9DrawPrimitive` (0x7FA360): flush, `DrawPrimitive`.
#[hook(addr = 0x7FA360, conv = cdecl, name = "_rwD3D9DrawPrimitive", expect = "E8 9B 1E 00 00")]
pub fn draw_primitive(prim: u32, start: u32, count: u32) -> i32 {
    flush_cache_call();
    dev_draw_primitive(device(), prim, start, count)
}

/// The transform-cache block of `state`, allocated from the transform freelist when missing.
fn transform_slot(state: u32) -> GPtr<u8> {
    let slot = at(TRANSFORMS, state.wrapping_mul(4));
    if slot.read() == 0 {
        let p = rw_memory_alloc(p8(gu(TRANSFORM_FL)), 0x30411);
        slot.write(p.addr() as u32);
    }
    p8(slot.read())
}

/// `RwD3D9SetTransform` (0x7FA390): 1 when the cached matrix equals `m` (identity for NULL; integer
/// compare); else copy, `SetTransform(state, m or identity)`, the WORLD flag follows; `hr >= 0`.
#[hook(addr = 0x7FA390, conv = cdecl, name = "RwD3D9SetTransform", expect = "55 8B EC 51 56 8B 75 0C")]
pub fn set_transform(state: u32, m: GPtr<u8>) -> u32 {
    let src = if m.is_null() { p8(IDENTITY) } else { m };
    let cached = at(TRANSFORMS, state.wrapping_mul(4)).read();
    if cached != 0 && dwords_equal(p8(cached), src, 0x10) {
        return 1;
    }
    copy_dwords(transform_slot(state), src, 0x10);
    let hr = dev_set_transform(device(), state, src.addr() as u32);
    if state == 0x100 {
        su(WORLD_IDENTITY, m.is_null() as u32);
    }
    (hr >= 0) as u32
}

/// `RwD3D9GetTransform` (0x7FA4F0): the cached matrix (identity when none) into `out` (if not NULL).
#[hook(addr = 0x7FA4F0, conv = cdecl, name = "RwD3D9GetTransform", expect = "55 8B EC 56 57 8B 7D 0C")]
pub fn get_transform(state: u32, out: GPtr<u8>) {
    if out.is_null() {
        return;
    }
    let mut src = at(TRANSFORMS, state.wrapping_mul(4)).read();
    if src == 0 {
        src = IDENTITY;
    }
    copy_dwords(out, p8(src), 0x10);
}

/// `RwD3D9SetTransformWorld` (0x7FA520): an RwMatrix without the internal-identity flag is laid out as a
/// D3DMATRIX at 0x8E24D0 (rows right/up/at/pos; the w column keeps its static values) and sent when it
/// differs from the cached world (or the world was identity); identity is sent once. `hr >= 0` or 1.
#[hook(addr = 0x7FA520, conv = cdecl, name = "RwD3D9SetTransformWorld", expect = "55 8B EC 8B 45 08")]
pub fn set_transform_world(m: GPtr<u8>) -> u32 {
    let cache = p8(gu(WORLD_CACHE));
    if !m.is_null() && rd::<u32>(m, 0xC) & 0x2_0000 == 0 {
        let st = p8(WORLD_STAGING);
        for row in 0..4isize {
            for c in 0..3isize {
                wr::<u32>(st, row * 0x10 + c * 4, rd::<u32>(m, row * 0x10 + c * 4));
            }
        }
        if gu(WORLD_IDENTITY) == 0 && dwords_equal(cache, st, 0x10) {
            return 1;
        }
        copy_dwords(cache, st, 0x10);
        let hr = dev_set_transform(device(), 0x100, WORLD_STAGING);
        su(WORLD_IDENTITY, 0);
        return (hr >= 0) as u32;
    }
    if gu(WORLD_IDENTITY) != 0 {
        return 1;
    }
    copy_dwords(cache, p8(IDENTITY), 0x10);
    let hr = dev_set_transform(device(), 0x100, IDENTITY);
    su(WORLD_IDENTITY, 1);
    (hr >= 0) as u32
}

fn light_entry(index: u32) -> GPtr<u8> {
    p8(gu(LIGHTS).wrapping_add(index.wrapping_mul(0x6C)))
}

/// `RwD3D9SetLight` (0x7FA660): an index past the count grows the array (`rwrealloc` / `rwmalloc`, hint
/// 0x1030411, new entries disabled) and always sends; else 1 when `_rwD3D9LightCompare` finds it
/// unchanged. Copies the D3DLIGHT9 and calls `SetLight(index, light)`; `hr >= 0`.
#[hook(addr = 0x7FA660, conv = cdecl, name = "RwD3D9SetLight", expect = "53 8B 5C 24 08 55")]
pub fn set_light(index: i32, light: GPtr<u8>) -> u32 {
    let old = gu(LIGHT_COUNT) as i32;
    if index >= old {
        let n = index.wrapping_add(1);
        su(LIGHT_COUNT, n as u32);
        let size = (n as u32).wrapping_mul(0x6C);
        let p = if gu(LIGHTS) != 0 {
            rw_realloc(p8(gu(LIGHTS)), size, 0x103_0411)
        } else {
            rw_malloc(size, 0x103_0411)
        };
        su(LIGHTS, p.addr() as u32);
        let mut i = old;
        while i < gu(LIGHT_COUNT) as i32 {
            wr::<u32>(light_entry(i as u32), 0x68, 0);
            i += 1;
        }
    } else if light_compare_call(light, light_entry(index as u32)) != 0 {
        return 1;
    }
    copy_dwords(light_entry(index as u32), light, 0x1A);
    (dev_set_light(device(), index as u32, light.addr() as u32) >= 0) as u32
}

/// `RwD3D9GetLight` (0x7FA820): the cached D3DLIGHT9, or zeros past the count.
#[hook(addr = 0x7FA820, conv = cdecl, name = "RwD3D9GetLight", expect = "8B 44 24 04 8B 0D 84 80 C9 00")]
pub fn get_light(index: i32, out: GPtr<u8>) {
    if index < gu(LIGHT_COUNT) as i32 {
        copy_dwords(out, light_entry(index as u32), 0x1A);
    } else {
        for i in 0..0x1A {
            wr::<u32>(out, i * 4, 0);
        }
    }
}

/// `RwD3D9EnableLight` (0x7FA860): `LightEnable` when the cached flag differs (`hr >= 0`), else 1.
#[hook(addr = 0x7FA860, conv = cdecl, name = "RwD3D9EnableLight", expect = "8B 4C 24 04 A1 84 80 C9 00")]
pub fn enable_light(index: i32, enable: u32) -> u32 {
    if index >= gu(LIGHT_COUNT) as i32 {
        return 1;
    }
    let e = light_entry(index as u32);
    if rd::<u32>(e, 0x68) == enable {
        return 1;
    }
    wr::<u32>(e, 0x68, enable);
    (dev_light_enable(device(), index as u32, enable) >= 0) as u32
}

/// `FUN_007fa8b0` (`_rwD3D9LightCompare(a, b)`): 1 when type and diffuse rgb match and, by type, POINT:
/// position + range, SPOT: position, direction, range, theta, phi, DIRECTIONAL: direction (integer
/// compares); other types compare equal. Else 0.
#[hook(addr = 0x7FA8B0, conv = cdecl, name = "FUN_007fa8b0", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn light_compare(a: GPtr<u8>, b: GPtr<u8>) -> u32 {
    let eq = |o: isize| rd::<u32>(a, o) == rd::<u32>(b, o);
    if !(eq(0) && eq(4) && eq(8) && eq(0xC)) {
        return 0;
    }
    let fields: &[isize] = match rd::<u32>(a, 0) {
        1 => &[0x34, 0x38, 0x3C, 0x4C],
        2 => &[0x34, 0x38, 0x3C, 0x40, 0x44, 0x48, 0x4C, 0x60, 0x64],
        3 => &[0x40, 0x44, 0x48],
        _ => &[],
    };
    fields.iter().all(|&o| eq(o)) as u32
}

fn decl_entry(i: u32) -> GPtr<u8> {
    p8(gu(DECLS).wrapping_add(i.wrapping_mul(0xC)))
}

/// `RwD3D9CreateVertexDeclaration` (0x7FAA30): looks the element list (D3DDECL_END included) up in the
/// declaration cache: a hit AddRefs (or re-creates a released) declaration; a miss grows the cache by 16
/// when full, creates the declaration and keeps a copy of the elements (`rwmalloc`, unchecked). Returns
/// `*out != NULL`.
#[hook(addr = 0x7FAA30, conv = cdecl, name = "RwD3D9CreateVertexDeclaration", expect = "53 55 8B 6C 24 0C")]
pub fn create_vertex_declaration(elements: GPtr<u8>, out: GPtr<u32>) -> u32 {
    let mut n: u32 = 0;
    while rd::<u8>(elements, n as isize * 8 + 4) != 0x11 {
        n += 1;
    }
    n += 1;
    let count = gu(DECL_COUNT);
    for i in 0..count {
        let e = decl_entry(i);
        if rd::<u32>(e, 4) != n {
            continue;
        }
        let copy = p8(rd::<u32>(e, 8));
        if !dwords_equal(copy, elements, (n * 2) as isize) {
            continue;
        }
        let d = rd::<u32>(e, 0);
        if d != 0 {
            com_add_ref(p8(d));
        } else {
            dev_create_vertex_declaration(device(), rd::<u32>(e, 8), e.cast());
        }
        let d = rd::<u32>(decl_entry(i), 0);
        out.write(d);
        return (d != 0) as u32;
    }
    if count >= gu(DECL_CAPACITY) {
        let cap = gu(DECL_CAPACITY).wrapping_add(0x10);
        su(DECL_CAPACITY, cap);
        let size = cap.wrapping_mul(0xC);
        let p = if gu(DECLS) != 0 {
            rw_realloc(p8(gu(DECLS)), size, 0x104_0411)
        } else {
            rw_malloc(size, 0x104_0411)
        };
        su(DECLS, p.addr() as u32);
    }
    if dev_create_vertex_declaration(device(), elements.addr() as u32, out) < 0 {
        out.write(0);
        return 0;
    }
    wr::<u32>(decl_entry(gu(DECL_COUNT)), 0, out.read());
    wr::<u32>(decl_entry(gu(DECL_COUNT)), 4, n);
    let copy = rw_malloc(n * 8, 0x4_0411);
    wr::<u32>(decl_entry(gu(DECL_COUNT)), 8, copy.addr() as u32);
    let dst = p8(rd::<u32>(decl_entry(gu(DECL_COUNT)), 8));
    copy_dwords(dst, elements, (n * 2) as isize);
    su(DECL_COUNT, gu(DECL_COUNT).wrapping_add(1));
    (out.read() != 0) as u32
}

/// `RwD3D9DeleteVertexDeclaration` (0x7FAC10): releases a cached declaration and clears its slot when the
/// reference count reaches 0 (the element copy stays cached).
#[hook(addr = 0x7FAC10, conv = cdecl, name = "RwD3D9DeleteVertexDeclaration", expect = "8B 15 58 7C C9 00")]
pub fn delete_vertex_declaration(decl: u32) {
    for i in 0..gu(DECL_COUNT) {
        if rd::<u32>(decl_entry(i), 0) == decl {
            let e = decl_entry(i);
            if com_release(p8(rd::<u32>(e, 0))) == 0 {
                wr::<u32>(decl_entry(i), 0, 0);
            }
            return;
        }
    }
}

/// `RwD3D9CreateVertexShader` (0x7FAC60): on success the vertex-shader cache is invalidated; `hr >= 0`.
#[hook(addr = 0x7FAC60, conv = cdecl, name = "RwD3D9CreateVertexShader", expect = "8B 54 24 08 A1 28 7C C9 00")]
pub fn create_vertex_shader(func: u32, out: u32) -> u32 {
    let hr = dev_create_vertex_shader(device(), func, out);
    if hr >= 0 {
        su(LAST_VS, 0xFFFF_FFFF);
    }
    (hr >= 0) as u32
}

/// `RwD3D9DeleteVertexShader` (0x7FAC90): `Release` (its result returned).
#[hook(addr = 0x7FAC90, conv = cdecl, name = "RwD3D9DeleteVertexShader", expect = "8B 44 24 04 50 8B 08")]
pub fn delete_vertex_shader(vs: GPtr<u8>) -> u32 {
    com_release(vs)
}

/// `_rwD3D9SetVertexShaderConstant` (0x7FACA0): `SetVertexShaderConstantF` (HRESULT returned).
#[hook(addr = 0x7FACA0, conv = cdecl, name = "_rwD3D9SetVertexShaderConstant", expect = "8B 54 24 0C A1 28 7C C9 00")]
pub fn set_vertex_shader_constant(reg: u32, data: u32, count: u32) -> i32 {
    dev_set_vs_constant(device(), reg, data, count)
}

/// `RwD3D9CreatePixelShader` (0x7FACC0): `hr >= 0`.
#[hook(addr = 0x7FACC0, conv = cdecl, name = "RwD3D9CreatePixelShader", expect = "8B 54 24 08 A1 28 7C C9 00")]
pub fn create_pixel_shader(func: u32, out: u32) -> u32 {
    (dev_create_pixel_shader(device(), func, out) >= 0) as u32
}

/// `RwD3D9DeletePixelShader` (0x7FACF0): `Release`.
#[hook(addr = 0x7FACF0, conv = cdecl, name = "RwD3D9DeletePixelShader", expect = "8B 44 24 04 50 8B 08")]
pub fn delete_pixel_shader(ps: GPtr<u8>) -> u32 {
    com_release(ps)
}

/// `_rwD3D9SetPixelShaderConstant` (0x7FAD00): `SetPixelShaderConstantF` (HRESULT returned).
#[hook(addr = 0x7FAD00, conv = cdecl, name = "_rwD3D9SetPixelShaderConstant", expect = "8B 54 24 0C A1 28 7C C9 00")]
pub fn set_pixel_shader_constant(reg: u32, data: u32, count: u32) -> i32 {
    dev_set_ps_constant(device(), reg, data, count)
}

/// `RwD3D9GetCaps` (0x7FAD20): the D3DCAPS9 copy at 0xC9BF00.
#[hook(addr = 0x7FAD20, conv = cdecl, name = "RwD3D9GetCaps", expect = "B8 00 BF C9 00 C3")]
pub fn get_caps() -> u32 {
    CAPS
}

/// `fcomp` then `and eax, 0x4100; je`: the plane test fails (return 0) when `v > o` (no C3/C0).
fn above(v: X87, o: X87) -> bool {
    !v.fcom(o).test_any(0x41)
}

/// `RwD3D9CameraIsSphereFullyInsideFrustum` (0x7FAD30): for the 6 frustum planes at camera +0x94
/// (stride 0x14), `((n.y*c.y + n.x*c.x) + n.z*c.z) - d` must not be above `-radius` (x87, PC 24;
/// unordered passes); else 0.
/// (The original stores `-radius` over its camera argument slot: not reproduced.)
#[hook(addr = 0x7FAD30, conv = cdecl, name = "RwD3D9CameraIsSphereFullyInsideFrustum", expect = "8B 54 24 08 8B 44 24 04")]
pub fn camera_is_sphere_fully_inside_frustum(camera: GPtr<u8>, sphere: GPtr<u8>) -> u32 {
    let neg_r = x(x(rd::<f32>(sphere, 0xC)).fchs().to_f32());
    for i in 0..6isize {
        let p = camera.byte_add(0x94 + i * 0x14);
        let a = x(rd::<f32>(p, 4)).fmul_m(rd::<f32>(sphere, 4));
        let b = x(rd::<f32>(p, 0)).fmul_m(rd::<f32>(sphere, 0));
        let s = a.fadd(b);
        let c = x(rd::<f32>(p, 8)).fmul_m(rd::<f32>(sphere, 8));
        let v = s.fadd(c).fsub_m(rd::<f32>(p, 0xC));
        if above(v, neg_r) {
            return 0;
        }
    }
    1
}

/// `RwD3D9CameraIsBBoxFullyInsideFrustum` (0x7FAD90): for each plane, the box corner chosen by the
/// plane's index bytes (+0x10..+0x12: 0 = inf, 1 = sup) must not lie beyond the plane:
/// `((n.y*Y + X*n.x) + n.z*Z)` above the plane distance returns 0.
#[hook(addr = 0x7FAD90, conv = cdecl, name = "RwD3D9CameraIsBBoxFullyInsideFrustum", expect = "8B 44 24 04 8B 54 24 08")]
pub fn camera_is_bbox_fully_inside_frustum(camera: GPtr<u8>, bbox: GPtr<u8>) -> u32 {
    for i in 0..6isize {
        let p = camera.byte_add(0x94 + i * 0x14);
        let corner = |k: isize| {
            let sel = rd::<u8>(p, 0x10 + k) as isize;
            rd::<f32>(bbox.byte_add(-sel * 12), 0xC + k * 4)
        };
        let (cx, cy, cz) = (corner(0), corner(1), corner(2));
        let a = x(rd::<f32>(p, 4)).fmul(x(cy));
        let b = x(cx).fmul_m(rd::<f32>(p, 0));
        let s = a.fadd(b);
        let c = x(rd::<f32>(p, 8)).fmul(x(cz));
        let v = s.fadd(c);
        if above(v, x(rd::<f32>(p, 0xC))) {
            return 0;
        }
    }
    1
}

/// `_rwD3D9DeviceSetRestoreCallback` (0x7FAE20).
#[hook(addr = 0x7FAE20, conv = cdecl, name = "_rwD3D9DeviceSetRestoreCallback", expect = "8B 44 24 04 A3 B0 80 C9 00")]
pub fn device_set_restore_callback(cb: u32) {
    su(RESTORE_CALLBACK, cb);
}

/// `_rwD3D9DeviceGetRestoreCallback` (0x7FAE30).
#[hook(addr = 0x7FAE30, conv = cdecl, name = "_rwD3D9DeviceGetRestoreCallback", expect = "A1 B0 80 C9 00 C3")]
pub fn device_get_restore_callback() -> u32 {
    gu(RESTORE_CALLBACK)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_SET_DEPTH_STENCIL_SURFACE,
    HOOK_SET_RENDER_TARGET_SURFACE,
    HOOK_ENGINE_GET_MAX_MULTISAMPLING_LEVELS,
    HOOK_ENGINE_SET_MULTISAMPLING_LEVELS,
    HOOK_ENGINE_SET_REFRESH_RATE,
    HOOK_ENGINE_SET_MULTITHREAD_SAFE,
    HOOK_ENGINE_SET_SOFTWARE_VERTEX_PROCESSING,
    HOOK_DEVICE_GET_HANDLE,
    HOOK_DEVICE_SUPPORTS_DXT_TEXTURE,
    HOOK_SET_STENCIL_CLEAR,
    HOOK_GET_STENCIL_CLEAR,
    HOOK_DEVICE_CACHES_RESET,
    HOOK_GET_CURRENT_D3D_RENDER_TARGET,
    HOOK_SET_RENDER_TARGET,
    HOOK_SET_FVF,
    HOOK_SET_VERTEX_DECLARATION,
    HOOK_SET_VERTEX_SHADER,
    HOOK_SET_PIXEL_SHADER,
    HOOK_SET_STREAM_SOURCE,
    HOOK_SET_STREAMS,
    HOOK_SET_INDICES,
    HOOK_DRAW_INDEXED_PRIMITIVE_UP,
    HOOK_DRAW_PRIMITIVE_UP,
    HOOK_DRAW_INDEXED_PRIMITIVE,
    HOOK_DRAW_PRIMITIVE,
    HOOK_SET_TRANSFORM,
    HOOK_GET_TRANSFORM,
    HOOK_SET_TRANSFORM_WORLD,
    HOOK_SET_LIGHT,
    HOOK_GET_LIGHT,
    HOOK_ENABLE_LIGHT,
    HOOK_LIGHT_COMPARE,
    HOOK_CREATE_VERTEX_DECLARATION,
    HOOK_DELETE_VERTEX_DECLARATION,
    HOOK_CREATE_VERTEX_SHADER,
    HOOK_DELETE_VERTEX_SHADER,
    HOOK_SET_VERTEX_SHADER_CONSTANT,
    HOOK_CREATE_PIXEL_SHADER,
    HOOK_DELETE_PIXEL_SHADER,
    HOOK_SET_PIXEL_SHADER_CONSTANT,
    HOOK_GET_CAPS,
    HOOK_CAMERA_IS_SPHERE_FULLY_INSIDE_FRUSTUM,
    HOOK_CAMERA_IS_BBOX_FULLY_INSIDE_FRUSTUM,
    HOOK_DEVICE_SET_RESTORE_CALLBACK,
    HOOK_DEVICE_GET_RESTORE_CALLBACK,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
