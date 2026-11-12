//! STD-4: the RW D3D9 driver's raster creation and destruction: `_rwD3D9RasterCreate` (0x4CCE60)
//! with its register-argument helpers (format check 0x4CC5C0, texture 0x4CB7C0, camera texture
//! 0x4CB9C0, Z buffer 0x4CCD70), `_rwD3D9RasterDestroy` (0x4CBB00), the video-memory list removal
//! (0x4CB560) and the palette index release (0x4CB730), and the protected auto-mipmap format check
//! (0x4CBF80). Ported from the 1.0 US bytes; they replace sa-game's `rw_standin.rs` stand-ins.
//!
//! Register arguments: the helpers are hooked with `conv = reg(...)` (HARNESS-5, `sa_core::regconv`)
//! and called directly by the ports here; `thunk_FUN_0040a288` (0x4CB730, an entry that is a
//! `nop; jmp` into a protection stub) stays a phase B `#[native]` (its register argument modelled as
//! a cdecl argument).
//!
//! Raster fields: +0 parent, +4 pixels, +8 palette, +0xC width, +0x10 height, +0x14 depth, +0x18
//! stride, +0x20 type, +0x21 flags, +0x23 format byte, +0x28 / +0x2C original size. Extension
//! (raster + `[0xB4E9E0]`): +0 texture / surface, +4 palette, +8 alpha, +9 cube (low nibble; face in
//! the high one), +0xA auto-mipmap, +0xB locked level, +0xC locked surface, +0x18 `D3DFORMAT`.

use sa_core::abi::{HookReg, NativeReg};
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, rw_malloc, rw_memory_alloc, rw_memory_free, rw_realloc, wr};

const EXT_OFFSET: usize = 0xB4E9E0;
/// Free palette indices: count, capacity, next new index, array of u16.
const PAL_FREE_COUNT: usize = 0xB4E9E8;
const PAL_FREE_CAP: usize = 0xB4E9EC;
const PAL_NEXT: usize = 0xB4E9F0;
const PAL_FREE: usize = 0xB4E9F4;
const PAL_FREELIST: usize = 0xB4E9F8;
/// Video-memory raster list head and its node freelist.
const VM_LIST: usize = 0xB4E9FC;
const VM_FREELIST: usize = 0xB4EA00;
/// RW format → `D3DFORMAT` (u32), depth (u8 +4), alpha (u8 +5), 8 bytes per format index.
const FORMAT_TABLE: usize = 0x85C670;
/// `D3DFORMAT` → RW format word (+2 of 4-byte entries).
const D3D_TO_RW: usize = 0xB4E7E0;
const WINDOW: usize = 0xC97C1C;
const D3D9: usize = 0xC97C20;
const ADAPTER: usize = 0xC97C24;
const DEVICE: usize = 0xC97C28;
const DEPTH_SURFACE: usize = 0xC97C2C;
const MODE_FORMAT: usize = 0xC9BEF0;
const MODE_DEPTH: usize = 0xC9BEF4;
const Z_DEPTH: usize = 0xC9BEFC;
const CAPS2: usize = 0xC9BF0C;
/// `D3DCAPS9.TextureCaps` low byte (bit 7: ALPHAPALETTE).
const TEXTURE_CAPS: usize = 0xC9BF3C;
const DEV_TYPE: usize = 0x8E2428;
const PP: usize = 0xC9C040;
const PP_FORMAT: usize = 0xC9C048;
const PP_AUTO_DEPTH_FORMAT: usize = 0xC9C068;
const MODE: usize = 0xC9BEE4;
const PP_MS: usize = 0xC9C050;
const PP_MS_QUALITY: usize = 0xC9C054;
const D3DERR_OUTOFVIDEOMEMORY: i32 = 0x8876_017Cu32 as i32;
const HINT: u32 = 0x30411;

orig! {
    /// `IUnknown::Release()` (+8)
    fn release(o: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;
    /// `IDirect3D9::CheckDeviceFormat(adapter, type, adapterFormat, usage, rtype, format)` (+0x28)
    fn d3d_check_device_format(d: GPtr<u8>, adapter: u32, ty: u32, af: u32, usage: u32, rtype: u32, f: u32) -> i32 = stdcall vslot 0x28 => unported;
    /// `IDirect3DDevice9::EvictManagedResources()` (+0x14)
    fn dev_evict_managed_resources(d: GPtr<u8>) -> i32 = stdcall vslot 0x14 => unported;
    /// `IDirect3DDevice9::CreateTexture(w, h, levels, usage, format, pool, out, shared)` (+0x5C)
    fn dev_create_texture(d: GPtr<u8>, w: u32, h: u32, levels: u32, usage: u32, f: u32, pool: u32, out: GPtr<u8>, shared: u32) -> i32 = stdcall vslot 0x5C => unported;
    /// `IDirect3DDevice9::CreateCubeTexture(edge, levels, usage, format, pool, out, shared)` (+0x64)
    fn dev_create_cube_texture(d: GPtr<u8>, edge: u32, levels: u32, usage: u32, f: u32, pool: u32, out: GPtr<u8>, shared: u32) -> i32 = stdcall vslot 0x64 => unported;
    /// `IDirect3DDevice9::CreateDepthStencilSurface(w, h, format, ms, quality, discard, out, shared)` (+0x74)
    fn dev_create_depth_stencil_surface(d: GPtr<u8>, w: u32, h: u32, f: u32, ms: u32, q: u32, discard: u32, out: GPtr<u8>, shared: u32) -> i32 = stdcall vslot 0x74 => unported;

    /// USER32 `GetClientRect(hwnd, rect)` (IAT 0x858304)
    fn get_client_rect(hwnd: u32, rect: GPtr<i32>) -> i32 = stdcall cell 0x858304 => unported;
    /// `_rwD3D9CheckValidTextureFormat(fmt)` (0x4CBDE0)
    fn valid_texture(f: u32) -> u32 = cdecl 0x4CBDE0 => crate::raster::d3d9_check_valid_texture_format;
    /// `_rwD3D9CheckValidZBufferFormat(fmt)` (0x4CBD80)
    fn valid_zbuffer(f: u32) -> u32 = cdecl 0x4CBD80 => crate::raster::d3d9_check_valid_zbuffer_format;
    /// `_rwD3D9CheckValidCameraTextureFormat(fmt)` (0x4CBE20)
    fn valid_camera_texture(f: u32) -> u32 = cdecl 0x4CBE20 => crate::raster::d3d9_check_valid_camera_texture_format;
    /// `_rwD3D9CheckValidZBufferTextureFormat(fmt)` (0x4CBE80)
    fn valid_zbuffer_texture(f: u32) -> u32 = cdecl 0x4CBE80 => crate::raster::d3d9_check_valid_zbuffer_texture_format;
    /// `_rwD3D9CheckAutoMipmapGenTextureFormat(fmt)` (0x4CBFD0)
    fn auto_mipmap_rt(f: u32) -> u32 = cdecl 0x4CBFD0 => crate::raster::d3d9_check_auto_mipmap_gen_texture_format;
    /// `thunk_FUN_004cbf8a(fmt)` (0x4CBF80)
    fn auto_mipmap(f: u32) -> u32 = cdecl 0x4CBF80 => check_auto_mipmap_texture_format;
    /// `FUN_007fed70(&width, &height, flags)`: the raster size clamp
    fn clamp_raster_size(w: GPtr<i32>, h: GPtr<i32>, flags: u32) = cdecl 0x7FED70 => crate::d3d9state::clamp_raster_size;
    /// `_rxD3D9VideoMemoryRasterListAdd(raster)` (0x4CB530)
    fn vm_list_add(r: GPtr<u8>) = cdecl 0x4CB530 => crate::raster::vm_raster_list_add;
    /// `_rwFreeListCreate(entrySize, entriesPerBlock, alignment, hint)` (0x801980)
    fn freelist_create(size: u32, per_block: u32, align: u32, hint: u32) -> GPtr<u8> = cdecl 0x801980 => unported;
    /// `D3DResourceSystem::CreateTexture(w, h, levels, format, out)` (0x730510)
    fn resource_create_texture(w: u32, h: u32, levels: u32, f: u32, out: GPtr<u8>) -> i32 = cdecl 0x730510 => unported;
    /// `D3DResourceSystem::DestroyTexture(texture)` (0x730B70)
    fn resource_destroy_texture(t: GPtr<u8>) = cdecl 0x730B70 => unported;
    /// `_rwD3D9GetCurrentTexture(stage)` (0x7FDE50)
    fn get_current_texture(stage: u32) -> u32 = cdecl 0x7FDE50 => crate::d3d9state::get_current_texture;
    /// `FUN_007fdcd0(raster, stage)`: the stage's texture set
    fn set_texture_internal(r: GPtr<u8>, stage: u32) -> u32 = cdecl 0x7FDCD0 => crate::d3d9state::set_texture_internal;
}

fn u(a: usize) -> u32 {
    g::<u32>(a).read()
}
fn device() -> GPtr<u8> {
    p8(u(DEVICE))
}
fn ext_of(r: GPtr<u8>) -> GPtr<u8> {
    r.byte_add(u(EXT_OFFSET) as i32 as isize)
}
/// `D3DFORMAT`, depth and alpha of RW format index `(fmt >> 8) & 0xF`.
fn table(fmt: u32) -> (u32, u32, u8) {
    let e = p8((FORMAT_TABLE + 8 * ((fmt >> 8) & 0xF) as usize) as u32);
    (rd::<u32>(e, 0), rd::<u8>(e, 4) as u32, rd::<u8>(e, 5))
}
fn auto_mip_bit(e: GPtr<u8>) {
    wr::<u8>(e, 0xA, (rd::<u8>(e, 0xA) & 0xF1) | 1);
}
/// `ext+0xA & 0xF ? AUTOGENMIPMAP : 0`.
fn autogen_usage(e: GPtr<u8>) -> u32 {
    if rd::<u8>(e, 0xA) & 0xF != 0 { 0x400 } else { 0 }
}
/// Pushes `{raster, next}` on the video-memory list (the inlined `_rxD3D9VideoMemoryRasterListAdd`).
fn vm_list_push(r: GPtr<u8>) {
    let n = rw_memory_alloc(p8(u(VM_FREELIST)), HINT);
    wr::<u32>(n, 0, r.addr() as u32);
    wr::<u32>(n, 4, u(VM_LIST));
    g::<u32>(VM_LIST).write(n.addr() as u32);
}

/// `thunk_FUN_004cbf8a(fmt)` (0x4CBF80, its body behind a protection jump): with CANAUTOGENMIPMAP
/// (caps2 bit 30), `CheckDeviceFormat(adapter, type, display format, AUTOGENMIPMAP, TEXTURE, fmt)`
/// == D3D_OK; else 0.
#[hook(addr = 0x4CBF80, conv = cdecl, name = "thunk_FUN_004cbf8a", expect = "90 90 90 90 90",
       unverified = "std4-d3d9-2", route = "L2", hazards = "none", recs = "boot")]
pub fn check_auto_mipmap_texture_format(f: u32) -> u32 {
    if u(CAPS2) & 0x4000_0000 == 0 {
        return 0;
    }
    (d3d_check_device_format(p8(u(D3D9)), u(ADAPTER), u(DEV_TYPE), u(MODE_FORMAT), 0x400, 3, f) == 0) as u32
}

/// `FUN_004cc5c0` (0x4CC5C0; EAX = flags, EDI = raster): the format check. Sets the type (+0x20) and
/// flags (+0x21) bytes, picks or validates the format and the depth, and stores the format byte
/// (+0x23, bits 8..15 of the result). Normal / texture rasters: a default format is 8888 (the mode
/// deeper than 16 bits and A8R8G8B8 valid), else 4444 (A4R4G4B4 valid), else 1555; palettised ones
/// need P8 (`PAL8` without a format: 8888 with ALPHAPALETTE caps, else 888, depth 8; `PAL4` fails);
/// an explicit format must pass the texture (or, for depth formats, the Z-texture) check; with
/// MIPMAP | AUTOMIPMAP and a format the device can auto-generate (0x4CBF80) the extension's
/// auto-mipmap bit is set. Z: D16 / D32 by the Z depth, an explicit one must match it. Camera: the
/// back buffer's RW format (an explicit one must equal it), the mode depth. Camera texture: 565 at
/// 16 bits, else 888 when valid (else 565); an explicit one must pass the camera-texture check;
/// auto-mipmap through 0x4CBFD0. Other types: 0.
#[hook(addr = 0x4CC5C0, conv = reg("args = eax edi"), name = "FUN_004cc5c0", expect = "53 8B D8 8A C3",
       unverified = "regconv-rw-1", route = "L2", hazards = "heap-writes", recs = "boot",
       note = "register convention, EAX = flags, EDI = raster -> EAX (HARNESS-5; was a phase B #[native])")]
pub fn check_raster_format(flags: u32, r: GPtr<u8>) -> u32 {
    let ty = flags & 7;
    wr::<u8>(r, 0x20, ty as u8);
    let mut fmt = flags & 0xFF00;
    wr::<u8>(r, 0x21, (flags & 0xF8) as u8);
    let depth = |d: u32| wr::<u32>(r, 0x14, d);
    let done = |fmt: u32| {
        wr::<u8>(r, 0x23, (fmt >> 8) as u8);
        1
    };
    match ty {
        0 | 4 => {
            let sel = fmt & 0xF00;
            if sel == 0 {
                if fmt & 0x2000 != 0 {
                    if valid_texture(0x29) == 0 {
                        return 0;
                    }
                    fmt |= if g::<u8>(TEXTURE_CAPS).read() & 0x80 != 0 { 0x500 } else { 0x600 };
                    depth(8);
                } else if (u(MODE_DEPTH) as i32) > 16 && valid_texture(0x15) != 0 {
                    fmt |= 0x500;
                    depth(32);
                } else if valid_texture(0x1A) != 0 {
                    fmt |= 0x300;
                    depth(16);
                } else {
                    fmt |= 0x100;
                    depth(16);
                }
            } else {
                if fmt & 0x4000 != 0 {
                    return 0;
                }
                if fmt & 0x2000 != 0 {
                    if valid_texture(0x29) == 0 {
                        return 0;
                    }
                    if !(sel == 0x600 || (sel == 0x500 && g::<u8>(TEXTURE_CAPS).read() & 0x80 != 0)) {
                        return 0;
                    }
                    depth(8);
                } else {
                    let (d, dd, _) = table(fmt);
                    let ok = if (0x46..=0x50).contains(&(d as i32)) { valid_zbuffer_texture(d) } else { valid_texture(d) };
                    if ok == 0 {
                        return 0;
                    }
                    depth(dd);
                }
            }
            if fmt & 0x2000 == 0 && flags & 0x9000 == 0x9000 && auto_mipmap(table(fmt).0) != 0 {
                auto_mip_bit(ext_of(r));
            }
            done(fmt)
        }
        1 => {
            let sel = fmt & 0xF00;
            let z = u(Z_DEPTH);
            if sel == 0 {
                if z == 16 {
                    depth(16);
                    return done(0x700);
                }
                if z == 32 {
                    depth(32);
                    return done(0x900);
                }
                return done(fmt);
            }
            if sel == 0x700 && z == 16 {
                depth(16);
                return done(fmt);
            }
            if sel == 0x900 && z == 32 {
                depth(32);
                return done(fmt);
            }
            0
        }
        2 => {
            let rw = g::<u16>(D3D_TO_RW + 4 * u(PP_FORMAT) as usize + 2).read() as u32;
            if fmt & 0xF00 != 0 && rw != fmt {
                return 0;
            }
            depth(u(MODE_DEPTH));
            done(rw)
        }
        5 => {
            if fmt & 0xF00 == 0 {
                if u(MODE_DEPTH) != 16 && valid_camera_texture(0x16) != 0 {
                    fmt |= 0x600;
                    depth(32);
                } else {
                    fmt |= 0x200;
                    depth(16);
                }
            } else {
                let (d, dd, _) = table(fmt);
                if valid_camera_texture(d) == 0 {
                    return 0;
                }
                depth(dd);
            }
            if flags & 0x9000 == 0x9000 && auto_mipmap_rt(table(fmt).0) != 0 {
                auto_mip_bit(ext_of(r));
            }
            done(fmt)
        }
        _ => 0,
    }
}

/// `FUN_004cb7c0(raster)` (0x4CB7C0; EDI = extension): a normal / texture raster's texture. The
/// format comes from the format byte; palettised (8888 / 888 only) rasters get a 0x404-byte palette
/// from their freelist (created on first use: 64 per block, alignment 4) with a free or new palette
/// index at +0x400, and P8. The alpha flag from the format table, the size clamped (0x7FED70), then a
/// cube (`CreateCubeTexture(width, levels, AUTOGENMIPMAP?, format, MANAGED)`), a depth texture
/// (`CreateTexture(w, h, 1, DEPTHSTENCIL, format, DEFAULT)` + the video-memory list) or
/// `D3DResourceSystem::CreateTexture(w, h, levels, format)`; levels = 1 without MIPMAP, else 0. A
/// failure frees the palette (index released) and returns 0; a palettised success joins the
/// video-memory list.
#[hook(addr = 0x4CB7C0, conv = reg("args = stack edi"), name = "FUN_004cb7c0", expect = "8B 4C 24 04 33 C0",
       unverified = "regconv-rw-1", route = "L2", hazards = "heap-writes", recs = "boot",
       note = "register convention, raster on the stack (caller pops), EDI = extension (HARNESS-5; was a phase B #[native])")]
pub fn create_texture_raster(r: GPtr<u8>, e: GPtr<u8>) -> u32 {
    let fmt = (rd::<u8>(r, 0x23) as u32) << 8;
    if fmt & 0x6000 == 0 {
        wr::<u32>(e, 0x18, table(fmt).0);
    } else {
        let sel = fmt & 0xF00;
        if sel != 0x600 && sel != 0x500 {
            return 0;
        }
        let mut fl = u(PAL_FREELIST);
        if fl == 0 {
            fl = freelist_create(0x404, 0x40, 4, HINT).addr() as u32;
            g::<u32>(PAL_FREELIST).write(fl);
        }
        let pal = rw_memory_alloc(p8(fl), HINT);
        wr::<u32>(e, 4, pal.addr() as u32);
        if !pal.is_null() {
            let n = u(PAL_FREE_COUNT);
            let idx = if n != 0 {
                g::<u32>(PAL_FREE_COUNT).write(n - 1);
                g::<u16>(u(PAL_FREE) as usize + 2 * (n - 1) as usize).read() as u32
            } else {
                let v = u(PAL_NEXT);
                g::<u32>(PAL_NEXT).write(v.wrapping_add(1));
                v
            };
            wr::<u32>(pal, 0x400, idx);
        }
        wr::<u32>(e, 0x18, 0x29);
    }
    wr::<u8>(e, 8, table(fmt).2);
    let w = r.byte_add(0xC).cast::<i32>();
    let h = r.byte_add(0x10).cast::<i32>();
    clamp_raster_size(w, h, rd::<u8>(r, 0x20) as u32 | fmt);
    let levels = !(fmt >> 15) & 1;
    let d3dfmt = rd::<u32>(e, 0x18);
    let hr = if rd::<u8>(e, 9) & 0xF != 0 {
        dev_create_cube_texture(device(), w.read() as u32, levels, autogen_usage(e), d3dfmt, 1, e, 0)
    } else if (0x46..=0x50).contains(&(d3dfmt as i32)) {
        let hr = dev_create_texture(device(), w.read() as u32, h.read() as u32, 1, 2, d3dfmt, 0, e, 0);
        if hr >= 0 {
            vm_list_add(r);
        }
        hr
    } else {
        resource_create_texture(w.read() as u32, h.read() as u32, levels, d3dfmt, e)
    };
    if hr < 0 {
        if rd::<u32>(e, 0x18) == 0x29 && rd::<u32>(e, 4) != 0 {
            palette_index_free(rd::<u32>(p8(rd::<u32>(e, 4)), 0x400));
            rw_memory_free(p8(u(PAL_FREELIST)), p8(rd::<u32>(e, 4)));
            wr::<u32>(e, 4, 0);
        }
        return 0;
    }
    if rd::<u32>(e, 4) != 0 {
        vm_list_push(r);
    }
    1
}

/// `FUN_004cb9c0(raster)` (0x4CB9C0; ESI = extension): a camera texture. Fails for palettised
/// formats; format and alpha from the format table, the size clamped, `EvictManagedResources`, then
/// `CreateCubeTexture` / `CreateTexture` with RENDERTARGET (| AUTOGENMIPMAP) in the DEFAULT pool.
/// Out of video memory fails; another failure retries in the back buffer's format without
/// RENDERTARGET (the extension keeps the first format) and on success takes that format's RW
/// format byte and the mode depth.
#[hook(addr = 0x4CB9C0, conv = reg("args = stack esi"), name = "FUN_004cb9c0", expect = "8B 4C 24 04 33 C0",
       unverified = "regconv-rw-1", route = "L2", hazards = "heap-writes", recs = "boot",
       note = "register convention, raster on the stack (caller pops), ESI = extension (HARNESS-5; was a phase B #[native])")]
pub fn create_camera_texture_raster(r: GPtr<u8>, e: GPtr<u8>) -> u32 {
    let fmt = (rd::<u8>(r, 0x23) as u32) << 8;
    if fmt & 0x6000 != 0 {
        return 0;
    }
    let (d, _, alpha) = table(fmt);
    wr::<u32>(e, 0x18, d);
    wr::<u8>(e, 8, alpha);
    let w = r.byte_add(0xC).cast::<i32>();
    let h = r.byte_add(0x10).cast::<i32>();
    clamp_raster_size(w, h, rd::<u8>(r, 0x20) as u32 | fmt);
    dev_evict_managed_resources(device());
    let levels = !(fmt >> 15) & 1;
    let create = |usage: u32, f: u32| {
        if rd::<u8>(e, 9) & 0xF != 0 {
            dev_create_cube_texture(device(), w.read() as u32, levels, usage, f, 0, e, 0)
        } else {
            dev_create_texture(device(), w.read() as u32, h.read() as u32, levels, usage, f, 0, e, 0)
        }
    };
    let hr = create(autogen_usage(e) | 1, rd::<u32>(e, 0x18));
    if hr == D3DERR_OUTOFVIDEOMEMORY {
        return 0;
    }
    if hr >= 0 {
        return 1;
    }
    if create(autogen_usage(e), u(PP_FORMAT)) < 0 {
        return 0;
    }
    wr::<u8>(r, 0x23, g::<u8>(D3D_TO_RW + 4 * u(PP_FORMAT) as usize + 3).read());
    wr::<u32>(r, 0x14, u(MODE_DEPTH));
    1
}

/// `FUN_004ccd70()` (0x4CCD70; EDI = raster, ESI = extension): a Z raster. The format by the Z
/// depth: 16 bits D15S1 when valid else D16; 32 bits D24S8, D24X4S4, D32 when valid else D24X8;
/// alpha 0. A raster of the window's client size uses the device's depth surface (0xC97C2C, no
/// reference added); otherwise `EvictManagedResources` and `CreateDepthStencilSurface(w, h, format,
/// the present parameters' multisampling, no discard)`. 0 on failure.
#[hook(addr = 0x4CCD70, conv = reg("args = edi esi"), name = "FUN_004ccd70", expect = "A1 FC BE C9 00",
       unverified = "regconv-rw-1", route = "L2", hazards = "heap-writes", recs = "boot",
       note = "register convention, EDI = raster, ESI = extension (HARNESS-5; was a phase B #[native])")]
pub fn create_zbuffer_raster(r: GPtr<u8>, e: GPtr<u8>) -> u32 {
    match u(Z_DEPTH) {
        16 => wr::<u32>(e, 0x18, if valid_zbuffer(0x49) != 0 { 0x49 } else { 0x50 }),
        32 => {
            let f = if valid_zbuffer(0x4B) != 0 {
                0x4B
            } else if valid_zbuffer(0x4F) != 0 {
                0x4F
            } else if valid_zbuffer(0x47) != 0 {
                0x47
            } else {
                0x4D
            };
            wr::<u32>(e, 0x18, f);
        }
        _ => {}
    }
    wr::<u8>(e, 8, 0);
    let mut rc = [0i32; 4];
    get_client_rect(u(WINDOW), GPtr::from_raw(rc.as_mut_ptr()));
    if rc[2] == rd::<i32>(r, 0xC) && rc[3] == rd::<i32>(r, 0x10) {
        wr::<u32>(e, 0, u(DEPTH_SURFACE));
        return 1;
    }
    dev_evict_managed_resources(device());
    let hr = dev_create_depth_stencil_surface(
        device(),
        rd::<u32>(r, 0xC),
        rd::<u32>(r, 0x10),
        rd::<u32>(e, 0x18),
        u(PP_MS),
        u(PP_MS_QUALITY),
        0,
        e,
        0,
    );
    (hr != D3DERR_OUTOFVIDEOMEMORY && hr >= 0) as u32
}

/// `_rwD3D9RasterCreate(out, raster, flags)` (0x4CCE60, `stdFunc[4]`): clears pixels / palette and
/// the extension (locked level 0xFF), the format check (0x4CC5C0; failure 0). An empty size makes a
/// don't-allocate raster (stride 0, depth 16 when unset; a camera raster joins the video-memory
/// list). Otherwise by type: normal / texture 0x4CB7C0, Z 0x4CCD70, camera texture 0x4CB9C0 (both
/// joining the video-memory list), none of them for don't-allocate rasters; a camera raster checks
/// the size against the window's client area (unless don't-allocate), takes the back buffer's depth
/// and format, becomes don't-allocate with its original size set and joins the list.
#[hook(addr = 0x4CCE60, conv = cdecl, name = "_rwD3D9RasterCreate", expect = "A1 E0 E9 B4 00 83 EC 10",
       route = "L2", hazards = "heap-writes", recs = "boot")]
pub fn raster_create(_out: u32, r: GPtr<u8>, flags: u32) -> u32 {
    let e = ext_of(r);
    wr::<u32>(r, 4, 0);
    wr::<u32>(r, 8, 0);
    wr::<u8>(r, 0x20, (flags & 7) as u8);
    wr::<u8>(r, 0x21, (flags & 0xF8) as u8);
    wr::<u32>(e, 0, 0);
    wr::<u32>(e, 4, 0);
    wr::<u8>(e, 8, 0);
    wr::<u8>(e, 0xA, 0);
    wr::<u8>(e, 9, 0);
    wr::<u8>(e, 0xB, 0xFF);
    wr::<u32>(e, 0xC, 0);
    wr::<u32>(e, 0x18, 0);
    if check_raster_format(flags, r) == 0 {
        return 0;
    }
    let dont_alloc = |r: GPtr<u8>| rd::<u8>(r, 0x21) & 0x80 != 0;
    if rd::<u32>(r, 0xC) == 0 || rd::<u32>(r, 0x10) == 0 {
        wr::<u8>(r, 0x21, 0x80);
        wr::<u32>(r, 0x18, 0);
        if rd::<u32>(r, 0x14) == 0 {
            wr::<u32>(r, 0x14, 0x10);
        }
        if rd::<u8>(r, 0x20) == 2 {
            vm_list_push(r);
        }
        return 1;
    }
    match rd::<u8>(r, 0x20) {
        0 | 4 => {
            if dont_alloc(r) {
                return 1;
            }
            (create_texture_raster(r, e) != 0) as u32
        }
        5 => {
            if dont_alloc(r) {
                return 1;
            }
            if create_camera_texture_raster(r, e) == 0 {
                return 0;
            }
            vm_list_add(r);
            1
        }
        1 => {
            if dont_alloc(r) {
                return 1;
            }
            if create_zbuffer_raster(r, e) == 0 {
                return 0;
            }
            vm_list_add(r);
            1
        }
        2 => {
            let mut rc = [0i32; 4];
            get_client_rect(u(WINDOW), GPtr::from_raw(rc.as_mut_ptr()));
            if !dont_alloc(r) && (rc[2] < rd::<i32>(r, 0xC) || rc[3] < rd::<i32>(r, 0x10)) {
                return 0;
            }
            let pf = u(PP_FORMAT);
            wr::<u32>(r, 0x14, match pf {
                0x15 | 0x16 | 0x23 => 0x20,
                0x17..=0x19 => 0x10,
                _ => 0,
            });
            wr::<u8>(r, 0x21, 0x80);
            wr::<u32>(r, 0x18, 0);
            wr::<u32>(r, 4, 0);
            wr::<u32>(r, 0x28, rd::<u32>(r, 0xC));
            wr::<u32>(r, 0x2C, rd::<u32>(r, 0x10));
            wr::<u32>(e, 0x18, u(PP_FORMAT));
            vm_list_add(r);
            1
        }
        _ => 1,
    }
}

/// `_rxD3D9VideoMemoryRasterListRemove()` (0x4CB560; ESI = raster): unlinks the raster's node from
/// the video-memory list and frees it (nothing when absent).
#[hook(addr = 0x4CB560, conv = reg("args = esi"), name = "_rxD3D9VideoMemoryRasterListRemove", expect = "8B 15 FC E9 B4 00",
       unverified = "regconv-rw-1", route = "L2", hazards = "heap-writes", recs = "boot",
       note = "register convention, ESI = raster; mov edi, edi at 0x4CB56E is an alignment no-op (HARNESS-5; was a phase B #[native])")]
pub fn vm_list_remove(r: GPtr<u8>) {
    let head = u(VM_LIST);
    let mut prev: u32 = 0;
    let mut n = head;
    while n != 0 {
        if rd::<u32>(p8(n), 0) == r.addr() as u32 {
            break;
        }
        prev = n;
        n = rd::<u32>(p8(n), 4);
    }
    if n == 0 {
        return;
    }
    if prev != 0 {
        wr::<u32>(p8(prev), 4, rd::<u32>(p8(n), 4));
    } else if head == n {
        g::<u32>(VM_LIST).write(rd::<u32>(p8(n), 4));
    }
    rw_memory_free(p8(u(VM_FREELIST)), p8(n));
}

/// `thunk_FUN_0040a288()` (0x4CB730; ESI = palette index; the protection stub at 0x40A288 loads the
/// free count 0xB4E9E8 into ECX and continues at 0x4CB736): the index is pushed on the free-index
/// array, which grows by 0x200 entries (`rwmalloc` / `rwrealloc`, hint 0x1030411) when full.
#[sa_core::native(addr = 0x4CB730, conv = cdecl, name = "thunk_FUN_0040a288")]
pub fn palette_index_free(idx: u32) {
    let n = u(PAL_FREE_COUNT);
    if n < u(PAL_FREE_CAP) {
        g::<u16>(u(PAL_FREE) as usize + 2 * n as usize).write(idx as u16);
        g::<u32>(PAL_FREE_COUNT).write(n + 1);
        return;
    }
    let cap = u(PAL_FREE_CAP).wrapping_add(0x200);
    g::<u32>(PAL_FREE_CAP).write(cap);
    let p = if u(PAL_FREE) == 0 {
        rw_malloc(cap.wrapping_mul(2), 0x103_0411)
    } else {
        rw_realloc(p8(u(PAL_FREE)), cap.wrapping_mul(2), 0x103_0411)
    };
    let n = u(PAL_FREE_COUNT);
    p.byte_add(2 * n as isize).cast::<u16>().write(idx as u16);
    g::<u32>(PAL_FREE).write(p.addr() as u32);
    g::<u32>(PAL_FREE_COUNT).write(n + 1);
}

/// `_rwD3D9RasterDestroy(out, raster, n)` (0x4CBB00, `stdFunc[5]`): unbinds the raster from every
/// stage 0..7 it is the current texture of. A sub-raster: a camera one leaves the video-memory list;
/// 1. A top-level raster by type: normal (texture: the same) frees its palette (leaving the list,
/// index released) or leaves the list for a depth format, then releases its texture
/// (`D3DResourceSystem::DestroyTexture` for textures); camera texture leaves the list and releases;
/// Z leaves the list and releases a surface other than the device's depth surface; camera leaves
/// the list; don't-allocate rasters do nothing. Type 3 and above 5 return 0, the rest 1.
#[hook(addr = 0x4CBB00, conv = cdecl, name = "_rwD3D9RasterDestroy", expect = "A1 E0 E9 B4 00 53 56",
       route = "L2", hazards = "heap-writes", recs = "boot")]
pub fn raster_destroy(_out: u32, r: GPtr<u8>, _n: u32) -> u32 {
    let e = ext_of(r);
    for stage in 0..8 {
        if get_current_texture(stage) == r.addr() as u32 {
            set_texture_internal(GPtr::NULL, stage);
        }
    }
    let ty = rd::<u8>(r, 0x20);
    if rd::<u32>(r, 0) != r.addr() as u32 {
        if ty == 2 {
            vm_list_remove(r);
        }
        return 1;
    }
    let dont_alloc = rd::<u8>(r, 0x21) & 0x80 != 0;
    match ty {
        0 | 4 => {
            if dont_alloc {
                return 1;
            }
            if rd::<u32>(e, 4) != 0 {
                vm_list_remove(r);
                palette_index_free(rd::<u32>(p8(rd::<u32>(e, 4)), 0x400));
                rw_memory_free(p8(u(PAL_FREELIST)), p8(rd::<u32>(e, 4)));
                wr::<u32>(e, 4, 0);
            } else if (0x46..=0x50).contains(&(rd::<u32>(e, 0x18) as i32)) {
                vm_list_remove(r);
            }
            let t = rd::<u32>(e, 0);
            if t != 0 {
                if ty == 0 {
                    release(p8(t));
                } else {
                    resource_destroy_texture(p8(t));
                }
            }
            1
        }
        5 => {
            if dont_alloc {
                return 1;
            }
            vm_list_remove(r);
            let t = rd::<u32>(e, 0);
            if t != 0 {
                release(p8(t));
            }
            1
        }
        1 => {
            if dont_alloc {
                return 1;
            }
            vm_list_remove(r);
            let t = rd::<u32>(e, 0);
            if t != u(DEPTH_SURFACE) && t != 0 {
                release(p8(t));
            }
            1
        }
        2 => {
            vm_list_remove(r);
            1
        }
        _ => 0,
    }
}

orig! {
    /// `IDirect3DDevice9::CreateAdditionalSwapChain(params, out)` (+0x34)
    fn dev_create_additional_swap_chain(d: GPtr<u8>, pp: GPtr<u32>, out: GPtr<u8>) -> i32 = stdcall vslot 0x34 => unported;
    /// `IDirect3DDevice9::SetPaletteEntries(palette, entries)` (+0x11C)
    fn dev_set_palette_entries(d: GPtr<u8>, index: u32, entries: GPtr<u8>) -> i32 = stdcall vslot 0x11C => unported;
    /// `IDirect3DSwapChain9::GetBackBuffer(index, type, out)` (+0x14)
    fn swap_chain_get_back_buffer(sc: GPtr<u8>, index: u32, ty: u32, out: GPtr<u8>) -> i32 = stdcall vslot 0x14 => unported;
}

orig! {
    /// `FUN_004cc0c0` (0x4CC0C0, proposed `_rwD3D9CheckValidFormatForRasterType`): EAX = cube flag,
    /// ECX = raster type, EDX = `D3DFORMAT`; non-zero when the device takes the format for the type
    fn check_format_for_type(cube: u32, ty: u32, f: u32) -> u32 = reg("args = eax ecx edx") 0x4CC0C0 => unported;
    /// `RwImageCreate(width, height, depth)` (0x8026E0)
    fn image_create(w: i32, h: i32, depth: i32) -> GPtr<u8> = cdecl 0x8026E0 => crate::image::rw_image_create;
    /// `RwImageAllocatePixels(image)` (0x8027A0)
    fn image_allocate_pixels(img: GPtr<u8>) -> GPtr<u8> = cdecl 0x8027A0 => crate::image::rw_image_allocate_pixels;
    /// `RwImageDestroy(image)` (0x802740)
    fn image_destroy(img: GPtr<u8>) -> u32 = cdecl 0x802740 => crate::image::rw_image_destroy;
    /// `_rwD3D9ImageGetRaster(image, raster, flags)` (0x7FF270)
    fn image_get_raster(img: GPtr<u8>, r: GPtr<u8>, flags: u32) -> u32 = cdecl 0x7FF270 => crate::image2::d3d9_image_get_raster;
    /// `_rwD3D9RasterSetImage(raster, image, flags)` (0x8001E0)
    fn raster_set_image(r: GPtr<u8>, img: GPtr<u8>, flags: u32) -> u32 = cdecl 0x8001E0 => crate::image2::d3d9_raster_set_image;
    /// `_rwD3D9RasterDestroy(out, raster, n)` (0x4CBB00)
    fn raster_destroy_call(out: u32, r: GPtr<u8>, n: u32) -> u32 = cdecl 0x4CBB00 => raster_destroy;
    /// `_rwD3D9RasterCreate(out, raster, flags)` (0x4CCE60)
    fn raster_create_call(out: u32, r: GPtr<u8>, flags: u32) -> u32 = cdecl 0x4CCE60 => raster_create;
}

/// `_rwD3D9RasterConvertToNonPalettized(raster)` (0x4CD250): the raster's top-level pixels go into a
/// new 32-bit image (`RwImageCreate(width, height, 32)`, NULL -> nothing; `RwImageAllocatePixels`,
/// `_rwD3D9ImageGetRaster(image, raster->parent, 0)`), then the raster is destroyed and re-created
/// as `(type & 7) | format | MIPMAP | AUTOMIPMAP` (0x9000; the create's result is ignored) and gets
/// the image back (`_rwD3D9RasterSetImage`), and the image is destroyed. Format (checked with
/// 0x4CC0C0 for the extension's cube flag and the type): with the extension's alpha byte, 8888
/// (0x500) when the mode is deeper than 16 bits and A8R8G8B8 (0x15) is valid, else 4444 (0x300)
/// when A4R4G4B4 (0x1A) is valid, else 1555 (0x100); without alpha, 888 (0x600) when deeper than
/// 16 bits and X8R8G8B8 (0x16) is valid, else 565 (0x200).
#[hook(addr = 0x4CD250, conv = cdecl, name = "_rwD3D9RasterConvertToNonPalettized", expect = "53 57 8B 7C 24 0C",
       unverified = "regconv-rw-1", route = "L2", hazards = "heap-writes", recs = "boot",
       note = "calls the register-convention format check 0x4CC0C0 (EAX/ECX/EDX)")]
pub fn raster_convert_to_non_palettized(r: GPtr<u8>) {
    let img = image_create(rd::<i32>(r, 0xC), rd::<i32>(r, 0x10), 0x20);
    if img.is_null() {
        return;
    }
    let e = ext_of(r);
    image_allocate_pixels(img);
    image_get_raster(img, p8(rd::<u32>(r, 0)), 0);
    let cube = || u32::from(rd::<u8>(e, 9) & 0xF);
    let ty = || u32::from(rd::<u8>(r, 0x20));
    let deep = u(MODE_DEPTH) as i32 > 16;
    let fmt = if rd::<u8>(e, 8) != 0 {
        if deep && check_format_for_type(cube(), ty(), 0x15) != 0 {
            0x500
        } else if check_format_for_type(cube(), ty(), 0x1A) != 0 {
            0x300
        } else {
            0x100
        }
    } else if deep {
        if check_format_for_type(cube(), ty(), 0x16) != 0 { 0x600 } else { 0x200 }
    } else {
        0x200
    };
    raster_destroy_call(0, r, 0);
    raster_create_call(0, r, (u32::from(rd::<u8>(r, 0x20)) & 7) | fmt | 0x9000);
    raster_set_image(r, img, 0);
    image_destroy(img);
}

/// `_rxD3D9VideoMemoryRasterListRestore()` (0x4CC970; the list head 0xB4E9FC is loaded behind a
/// protection jump): after a device reset every top-level raster of the video-memory list is
/// re-created. Normal / texture: a palettised one re-sends its palette (`SetPaletteEntries`), a
/// depth-format one gets a new depth texture. Camera texture without texture: as created
/// (render target, retry in the back-buffer format without RENDERTARGET whose RW format byte,
/// depth, D3D format and alpha from 0xB4E7E0 it then takes). Z: without surface the format by the
/// Z depth and a new depth-stencil surface (size at least 1x1), else the window's client size, the
/// device's depth surface and the auto depth-stencil format. Camera: a swap-chain one gets a new
/// swap chain (copy of the present parameters, the window's client size (0 for a non-empty
/// side, 1 otherwise), no auto depth-stencil) and its back buffer (released once); otherwise its
/// size is rescaled from the engine mode to the back buffer and it takes the back buffer's format
/// data. Returns 1 unless an HRESULT seen after a raster was negative (the last one is carried
/// over to later rasters).
#[hook(addr = 0x4CC970, conv = cdecl, name = "_rxD3D9VideoMemoryRasterListRestore", expect = "83 EC 60 E9 3D 90 F3 FF",
       route = "L2", hazards = "heap-writes", recs = "boot",
       note = "entry jumps into a protection stub (0x4059B5) that loads the list head")]
pub fn vm_list_restore() -> u32 {
    let mut ok: u32 = 1;
    let mut hr: i32 = 0;
    let mut n = u(VM_LIST);
    while n != 0 {
        let r = p8(rd::<u32>(p8(n), 0));
        let next = rd::<u32>(p8(n), 4);
        let e = ext_of(r);
        let fmt_rw = |r: GPtr<u8>| (rd::<u8>(r, 0x23) as u32) << 8;
        let pp_fmt = || u(PP_FORMAT) as usize;
        if rd::<u32>(r, 0) == r.addr() as u32 {
            match rd::<u8>(r, 0x20) {
                0 | 4 => {
                    if fmt_rw(r) & 0x6000 != 0 {
                        let pal = rd::<u32>(e, 4);
                        if pal != 0 {
                            hr = dev_set_palette_entries(device(), rd::<u32>(p8(pal), 0x400), p8(pal));
                        }
                    } else {
                        let d = rd::<u32>(e, 0x18);
                        if (0x46..=0x50).contains(&(d as i32)) {
                            hr = dev_create_texture(device(), rd::<u32>(r, 0xC), rd::<u32>(r, 0x10), 1, 2, d, 0, e, 0);
                        }
                    }
                }
                5 => {
                    if rd::<u32>(e, 0) == 0 {
                        let levels = !(fmt_rw(r) >> 15) & 1;
                        let create = |usage: u32, f: u32| {
                            if rd::<u8>(e, 9) & 0xF != 0 {
                                dev_create_cube_texture(device(), rd::<u32>(r, 0xC), levels, usage, f, 0, e, 0)
                            } else {
                                dev_create_texture(device(), rd::<u32>(r, 0xC), rd::<u32>(r, 0x10), levels, usage, f, 0, e, 0)
                            }
                        };
                        hr = create(autogen_usage(e) | 1, rd::<u32>(e, 0x18));
                        if hr >= 0 {
                            n = next;
                            continue;
                        }
                        if hr != D3DERR_OUTOFVIDEOMEMORY {
                            hr = create(autogen_usage(e), u(PP_FORMAT));
                            wr::<u8>(r, 0x23, g::<u8>(D3D_TO_RW + 4 * pp_fmt() + 3).read());
                            wr::<u32>(r, 0x14, g::<u8>(D3D_TO_RW + 4 * pp_fmt() + 1).read() as u32);
                            wr::<u32>(e, 0x18, u(PP_FORMAT));
                            wr::<u8>(e, 8, g::<u8>(D3D_TO_RW + 4 * pp_fmt()).read());
                        }
                    }
                }
                1 => {
                    if rd::<u32>(e, 0) == 0 {
                        match u(Z_DEPTH) {
                            16 => wr::<u32>(e, 0x18, if valid_zbuffer(0x49) != 0 { 0x49 } else { 0x50 }),
                            32 => {
                                let f = if valid_zbuffer(0x4B) != 0 {
                                    0x4B
                                } else if valid_zbuffer(0x4F) != 0 {
                                    0x4F
                                } else if valid_zbuffer(0x47) != 0 {
                                    0x47
                                } else {
                                    0x4D
                                };
                                wr::<u32>(e, 0x18, f);
                            }
                            _ => {}
                        }
                        let h = rd::<i32>(r, 0x10).max(1) as u32;
                        let w = rd::<i32>(r, 0xC).max(1) as u32;
                        hr = dev_create_depth_stencil_surface(device(), w, h, rd::<u32>(e, 0x18), u(PP_MS), u(PP_MS_QUALITY), 0, e, 0);
                    } else {
                        let mut rc = [0i32; 4];
                        get_client_rect(u(WINDOW), GPtr::from_raw(rc.as_mut_ptr()));
                        wr::<i32>(r, 0xC, rc[2]);
                        wr::<i32>(r, 0x10, rc[3]);
                        wr::<u32>(e, 0, u(DEPTH_SURFACE));
                        wr::<u32>(e, 0x18, u(PP_AUTO_DEPTH_FORMAT));
                    }
                }
                2 => {
                    if rd::<u32>(e, 0x1C) != 0 {
                        let mut pp = [0u32; 14];
                        for (k, v) in pp.iter_mut().enumerate() {
                            *v = u(PP + 4 * k);
                        }
                        let hwnd = rd::<u32>(e, 0x20);
                        pp[7] = hwnd;
                        let mut rc = [0i32; 4];
                        get_client_rect(hwnd, GPtr::from_raw(rc.as_mut_ptr()));
                        wr::<u32>(e, 0x1C, 0);
                        pp[0] = (rc[2] == 0) as u32;
                        pp[1] = (rc[3] == 0) as u32;
                        pp[9] = 0;
                        if dev_create_additional_swap_chain(device(), GPtr::from_raw(pp.as_mut_ptr()), e.byte_add(0x1C)) == 0 {
                            hr = swap_chain_get_back_buffer(p8(rd::<u32>(e, 0x1C)), 0, 0, e);
                            release(p8(rd::<u32>(e, 0)));
                            wr::<u32>(e, 0x18, pp[2]);
                        }
                    } else {
                        let mw = u(MODE);
                        if mw != 0 && rd::<u32>(r, 0xC) != u(PP) {
                            wr::<u32>(r, 0xC, rd::<u32>(r, 0xC).wrapping_mul(u(PP)) / mw);
                        }
                        let mh = u(MODE + 4);
                        if mh != 0 && rd::<u32>(r, 0x10) != u(PP + 4) {
                            wr::<u32>(r, 0x10, rd::<u32>(r, 0x10).wrapping_mul(u(PP + 4)) / mh);
                        }
                        wr::<u32>(r, 0x14, g::<u8>(D3D_TO_RW + 4 * pp_fmt() + 1).read() as u32);
                        wr::<u8>(r, 0x23, g::<u8>(D3D_TO_RW + 4 * pp_fmt() + 3).read());
                        wr::<u32>(e, 0x18, u(PP_FORMAT));
                        wr::<u8>(e, 8, g::<u8>(D3D_TO_RW + 4 * pp_fmt()).read());
                    }
                }
                _ => {}
            }
        }
        if hr < 0 {
            ok = 0;
        }
        n = next;
    }
    ok
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RASTER_CONVERT_TO_NON_PALETTIZED,
    HOOK_CHECK_RASTER_FORMAT,
    HOOK_CREATE_TEXTURE_RASTER,
    HOOK_CREATE_CAMERA_TEXTURE_RASTER,
    HOOK_CREATE_ZBUFFER_RASTER,
    HOOK_VM_LIST_REMOVE,
    HOOK_CHECK_AUTO_MIPMAP_TEXTURE_FORMAT,
    HOOK_RASTER_CREATE,
    HOOK_RASTER_DESTROY,
    HOOK_VM_LIST_RESTORE,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[
    NATIVE_PALETTE_INDEX_FREE,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
