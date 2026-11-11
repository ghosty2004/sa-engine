//! PORT-RW-5 module `d3d9cam`: D3D9 camera begin / end update and the driver's system entry (see
//! docs/arch/rw-port.md, PORT-RW-5). Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, wr};
use crate::d3d9state::device;

/// The camera being updated (set by begin update, cleared by end update).
const CURRENT_CAMERA: usize = 0xC9BCC0;
/// `D3D9RasterExtOffset`: the raster plugin's extension offset.
const RASTER_EXT_OFFSET: usize = 0xB4E9E0;
/// The back-buffer surface StretchRect copies from.
const RENDER_SURFACE: usize = 0xC97C30;
/// Set when a camera texture must receive the frame at end update.
const NEED_COPY: usize = 0xC97C34;
orig! {
    /// `IDirect3D9::CheckDeviceMultiSampleType(adapter, type, format, windowed, samples, quality*)` (+0x2C)
    fn d3d_check_multisample(d3d: GPtr<u8>, adapter: u32, dev_type: u32, format: u32, windowed: u32, samples: u32, quality: u32) -> i32
        = stdcall vslot 0x2C => unported;
}

/// The multisampling globals: maximum level, quality scale, current level, current quality.
const MS_MAX: usize = 0x8E242C;
const MS_QUALITY_SCALE: usize = 0x8E2434;
const MS_LEVELS: usize = 0x8E2430;
const MS_QUALITY: usize = 0x8E2438;

/// `RwD3D9ChangeMultiSamplingLevels(int32 levels)` (0x7F8A90, cdecl). Nothing (1) before the mode list
/// exists or when the level is unchanged. Saves the present parameters and the level / quality, clamps the
/// level to the maximum (unsigned); below 2 the level and the quality become 1; else it steps the level down
/// while `CheckDeviceMultiSampleType(adapter, devType, format, windowed = !(mode flags & 1), level)` fails
/// (stopping below 2 without forcing 1), and the quality becomes `scale * clamped / max` (unsigned). Then
/// `D3D9DeviceReleaseVideoMemory`, `TestCooperativeLevel`, the present parameters for the current mode and
/// `Reset`: on success the post-reset sequence and the restore callback, 1 when the sequence succeeded; a
/// failed `Reset` or sequence (after releasing video memory again) restores the saved parameters, level and
/// quality and resets again: failure → 0, else the post-reset sequence and the callback → 1.
#[hook(addr = 0x7F8A90, conv = cdecl, name = "RwD3D9ChangeMultiSamplingLevels", expect = "A1 38 7C C9 00 83 EC 38",
       unverified = "std4-d3d9-4", route = "L3-only", hazards = "none", recs = "menus-hud",
       note = "device reset (OS effects); not double-runnable")]
pub fn change_multisampling_levels(levels: u32) -> u32 {
    if g::<u32>(VIDEO_MODES_READY).read() == 0 || g::<u32>(MS_LEVELS).read() == levels {
        return 1;
    }
    let mut saved = [0u32; 0xE];
    for (k, v) in saved.iter_mut().enumerate() {
        *v = g::<u32>(PRESENT_PARAMS as usize + k * 4).read();
    }
    let old_levels = g::<u32>(MS_LEVELS).read();
    let old_quality = g::<u32>(MS_QUALITY).read();
    let max = g::<u32>(MS_MAX).read();
    let clamped = if levels > max { max } else { levels };
    g::<u32>(MS_LEVELS).write(clamped);
    let quality = if clamped < 2 {
        g::<u32>(MS_LEVELS).write(1);
        1
    } else {
        loop {
            let windowed = !g::<u32>(0xC9BEF8).read() & 1;
            let d3d = p8(g::<u32>(0xC97C20).read());
            let hr = d3d_check_multisample(d3d, g::<u32>(0xC97C24).read(), g::<u32>(0x8E2428).read(),
                g::<u32>(0xC9BEF0).read(), windowed, g::<u32>(MS_LEVELS).read(), 0);
            if hr >= 0 {
                break;
            }
            let l = g::<u32>(MS_LEVELS).read().wrapping_sub(1);
            g::<u32>(MS_LEVELS).write(l);
            if l < 2 {
                break;
            }
        }
        g::<u32>(MS_QUALITY_SCALE).read().wrapping_mul(clamped) / max
    };
    g::<u32>(MS_QUALITY).write(quality);
    release_video_memory();
    dev_test_cooperative_level(device());
    set_present_parameters(MODE_WIDTH as u32, g::<u32>(0xC9BEF8).read(), g::<u32>(ADAPTER_CREATE_FLAGS).read());
    if dev_reset(device(), PRESENT_PARAMS) >= 0 {
        let ok = after_reset();
        rcb();
        if ok {
            return 1;
        }
        release_video_memory();
    }
    for (k, v) in saved.iter().enumerate() {
        g::<u32>(PRESENT_PARAMS as usize + k * 4).write(*v);
    }
    g::<u32>(MS_LEVELS).write(old_levels);
    g::<u32>(MS_QUALITY).write(old_quality);
    if dev_reset(device(), PRESENT_PARAMS) < 0 {
        return 0;
    }
    after_reset();
    rcb();
    1
}

/// D3D9 device caps word (texture filter caps; 0x1000100 = StretchRect point filtering).
const STRETCH_CAPS: usize = 0xC9BFF4;

orig! {
    /// `IDirect3DTexture9::GetSurfaceLevel(level, surface)` (+0x48)
    fn tex_get_surface_level(tex: GPtr<u8>, level: u32, out: GPtr<u32>) -> i32 = stdcall vslot 0x48 => unported;
    /// `IDirect3DCubeTexture9::GetCubeMapSurface(face, level, surface)` (+0x48)
    fn cube_get_cube_map_surface(tex: GPtr<u8>, face: u32, level: u32, out: GPtr<u32>) -> i32 = stdcall vslot 0x48 => unported;
    /// `IDirect3DDevice9::StretchRect(src, srcRect, dst, dstRect, filter)` (+0x88)
    fn dev_stretch_rect(dev: GPtr<u8>, src: u32, src_rect: GPtr<u8>, dst: u32, dst_rect: GPtr<u8>, filter: u32) -> i32 = stdcall vslot 0x88 => unported;
    /// `IUnknown::Release()` (+8)
    fn release(o: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;
}

/// `_rwD3D9CameraEndUpdate(out, camera, flags)` (0x7F98D0; reads none of its arguments). For the current
/// camera (0xC9BCC0) whose frame buffer's parent raster has no swap chain (extension +0x1C), is a camera
/// texture (type byte +0x20 == 5) and has a pending copy (0xC97C34): the texture's level-0 surface
/// (`GetCubeMapSurface(face = ext byte +9 >> 4, 0)` when the low nibble of that byte is set, else
/// `GetSurfaceLevel(0)`), the rectangle `{x, y, x + width, y + height}` of the frame buffer (offsets i16
/// +0x1C / +0x1E, size +0xC / +0x10) and `StretchRect(0xC97C30, &rect, surface, &rect, POINT when
/// 0xC9BFF4 & 0x1000100 else NONE)`, the surface released and the pending flag cleared. Always clears
/// the current camera and returns 1. A failed surface query leaves the original's surface local
/// uninitialised (port: 0).
#[hook(addr = 0x7F98D0, conv = cdecl, name = "_rwD3D9CameraEndUpdate", expect = "A1 C0 BC C9 00",
       route = "L2", hazards = "uninit-locals", recs = "grove-walk")]
pub fn camera_end_update(_out: u32, _camera: u32, _flags: u32) -> u32 {
    let cam = p8(g::<u32>(CURRENT_CAMERA).read());
    let fb = p8(rd::<u32>(cam, 0x60));
    let parent = p8(rd::<u32>(fb, 0));
    let ext = parent.byte_add(g::<u32>(RASTER_EXT_OFFSET).read() as i32 as isize);
    if rd::<u32>(ext, 0x1C) == 0 && rd::<u8>(parent, 0x20) == 5 && g::<u32>(NEED_COPY).read() != 0 {
        let face = rd::<u8>(ext, 9);
        let tex = p8(rd::<u32>(ext, 0));
        let mut surf: u32 = 0;
        if face & 0xF != 0 {
            cube_get_cube_map_surface(tex, (face >> 4) as u32, 0, GPtr::from_raw(&mut surf));
        } else {
            tex_get_surface_level(tex, 0, GPtr::from_raw(&mut surf));
        }
        let left = rd::<i16>(fb, 0x1C) as i32;
        let top = rd::<i16>(fb, 0x1E) as i32;
        let mut rect = [
            left as u32,
            top as u32,
            rd::<u32>(fb, 0xC).wrapping_add(left as u32),
            rd::<u32>(fb, 0x10).wrapping_add(top as u32),
        ];
        let filter = (g::<u32>(STRETCH_CAPS).read() & 0x100_0100 != 0) as u32;
        let rp: GPtr<u8> = GPtr::from_raw(rect.as_mut_ptr()).cast();
        dev_stretch_rect(device(), g::<u32>(RENDER_SURFACE).read(), rp, surf, rp, filter);
        release(p8(surf));
        g::<u32>(NEED_COPY).write(0);
    }
    g::<u32>(CURRENT_CAMERA).write(0);
    1
}

/// D3D9 present parameters (0x38 bytes; +0 back-buffer width, +4 height, +0x20 Windowed).
const PRESENT_PARAMS: u32 = 0xC9C040;
const VIDEO_MODES_READY: usize = 0xC97C38;
const NUM_VIDEO_MODES: usize = 0xC97C40;
/// Video modes, 0x14 each: width, height, refresh, format, flags.
const VIDEO_MODES: usize = 0xC97C48;
const CURRENT_MODE: usize = 0xC97C18;
const WINDOW: usize = 0xC97C1C;
const DEPTH_SURFACE: u32 = 0xC97C2C;
const ADAPTER_CREATE_FLAGS: usize = 0xC97BD4;
/// Light cache: count, then the array of 0x6C (D3DLIGHT9 + enable dword at +0x68).
const NUM_LIGHTS: usize = 0xC98084;
const LIGHTS: usize = 0xC98088;
/// Device restore callback.
const RESTORE_CB: usize = 0xC980B0;
/// Engine mode globals: width, height, refresh, format, depth, flags.
const MODE_WIDTH: usize = 0xC9BEE4;

orig! {
    /// `IDirect3DDevice9::TestCooperativeLevel()` (+0xC)
    fn dev_test_cooperative_level(dev: GPtr<u8>) -> i32 = stdcall vslot 0xC => unported;
    /// `IDirect3DDevice9::Reset(params)` (+0x40)
    fn dev_reset(dev: GPtr<u8>, params: u32) -> i32 = stdcall vslot 0x40 => unported;
    /// `IDirect3DDevice9::GetRenderTarget(index, out)` (+0x98)
    fn dev_get_render_target(dev: GPtr<u8>, index: u32, out: u32) -> i32 = stdcall vslot 0x98 => unported;
    /// `IDirect3DDevice9::GetDepthStencilSurface(out)` (+0xA0)
    fn dev_get_depth_stencil_surface(dev: GPtr<u8>, out: u32) -> i32 = stdcall vslot 0xA0 => unported;
    /// `IDirect3DDevice9::SetLight(index, light)` (+0xCC)
    fn dev_set_light(dev: GPtr<u8>, index: u32, light: u32) -> i32 = stdcall vslot 0xCC => unported;
    /// `IDirect3DDevice9::LightEnable(index, enable)` (+0xD4)
    fn dev_light_enable(dev: GPtr<u8>, index: u32, enable: u32) -> i32 = stdcall vslot 0xD4 => unported;

    /// Releases the default-pool driver objects before a reset (0x7F7F70)
    fn release_video_memory() = cdecl 0x7F7F70 => unported;
    /// Fills the present parameters from a video mode `(mode, flags, createFlags)` (0x7F6CB0)
    fn set_present_parameters(mode: u32, flags: u32, create_flags: u32) = cdecl 0x7F6CB0 => unported;
    /// Re-creates the VM rasters after a reset (0x4CC970)
    fn restore_video_memory_rasters() -> u32 = cdecl 0x4CC970 => unported;
    /// `_rwD3D9DynamicVertexBufferRestore()` (0x7F58D0)
    fn dynamic_vertex_buffer_restore() -> u32 = cdecl 0x7F58D0 => crate::d3d9vb::dynamic_vertex_buffer_restore;
    /// `_rwD3D9RenderStateReset()` (0x7FD100)
    fn render_state_reset() = cdecl 0x7FD100 => unported;
    /// `_rwD3D9Im2DRenderOpen()` (0x7FB480)
    fn im2d_render_open() -> u32 = cdecl 0x7FB480 => unported;
    /// `_rwD3D9Im3DRenderOpen()` (0x80E020)
    fn im3d_render_open() -> u32 = cdecl 0x80E020 => unported;
    /// The device restore callback through its pointer
    fn call_restore_cb(f: u32) = cdecl dyn => unported;

    /// USER32 `GetWindowLongA(hwnd, index)` (IAT 0x858260)
    fn get_window_long(hwnd: u32, index: i32) -> u32 = stdcall cell 0x858260 => unported;
    /// USER32 `GetMenu(hwnd)` (IAT 0x858264)
    fn get_menu(hwnd: u32) -> u32 = stdcall cell 0x858264 => unported;
    /// USER32 `AdjustWindowRectEx(rect, style, menu, exStyle)` (IAT 0x858268)
    fn adjust_window_rect_ex(rect: GPtr<u8>, style: u32, menu: u32, ex_style: u32) -> u32 = stdcall cell 0x858268 => unported;
    /// USER32 `SetWindowLongA(hwnd, index, value)` (IAT 0x858274)
    fn set_window_long(hwnd: u32, index: i32, value: u32) -> u32 = stdcall cell 0x858274 => unported;
    /// USER32 `SetWindowPos(hwnd, after, x, y, cx, cy, flags)` (IAT 0x8582E0)
    fn set_window_pos(hwnd: u32, after: u32, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> u32 = stdcall cell 0x8582E0 => unported;
    /// USER32 `GetClientRect(hwnd, rect)` (IAT 0x858304)
    fn get_client_rect(hwnd: u32, rect: GPtr<u8>) -> u32 = stdcall cell 0x858304 => unported;
}

/// The device reset sequence shared by both attempts of [`change_video_mode`] after a successful
/// `Reset`: re-fetch (and release) the render target and depth surface, then the VM rasters and dynamic
/// buffers, render states, the cached lights (`SetLight` + `LightEnable` each; the last HRESULT < 0
/// stops), Im2D and Im3D. True when all succeeded.
fn after_reset() -> bool {
    dev_get_render_target(device(), 0, RENDER_SURFACE as u32);
    release(p8(g::<u32>(RENDER_SURFACE).read()));
    dev_get_depth_stencil_surface(device(), DEPTH_SURFACE);
    release(p8(g::<u32>(DEPTH_SURFACE as usize).read()));
    if restore_video_memory_rasters() == 0 || dynamic_vertex_buffer_restore() == 0 {
        return false;
    }
    render_state_reset();
    let n = g::<i32>(NUM_LIGHTS).read();
    if n > 0 {
        let mut i: i32 = 0;
        let mut off: u32 = 0;
        let mut hr: i32;
        loop {
            let base = g::<u32>(LIGHTS).read();
            dev_set_light(device(), i as u32, base.wrapping_add(off));
            let base = g::<u32>(LIGHTS).read();
            hr = dev_light_enable(device(), i as u32, rd::<u32>(p8(base.wrapping_add(off)), 0x68));
            i += 1;
            off = off.wrapping_add(0x6C);
            if i >= g::<i32>(NUM_LIGHTS).read() {
                break;
            }
        }
        if hr < 0 {
            return false;
        }
    }
    im2d_render_open() != 0 && im3d_render_open() != 0
}

/// `AdjustWindowRectEx` of `rect` for the window's style, menu and extended style, then
/// `SetWindowPos(hwnd, NULL, left, bottom, right - left, bottom - top, flags)` (the original passes the
/// bottom as `y`; both callers' flags include SWP_NOMOVE, so it is ignored).
fn fit_window(rect: &mut [i32; 4], flags: u32) {
    let hwnd = g::<u32>(WINDOW).read();
    let ex = get_window_long(hwnd, -20);
    let menu = (get_menu(g::<u32>(WINDOW).read()) != 0) as u32;
    let style = get_window_long(g::<u32>(WINDOW).read(), -16);
    adjust_window_rect_ex(GPtr::from_raw(rect.as_mut_ptr()).cast(), style, menu, ex);
    let [l, t, r, b] = *rect;
    set_window_pos(g::<u32>(WINDOW).read(), 0, l, b, r.wrapping_sub(l), b.wrapping_sub(t), flags);
}

/// `RwD3D9ChangeVideoMode(mode)` (0x7F8640). Returns 1 without doing anything before the video modes
/// are set up (0xC97C38), for a mode outside `[0, numModes)` or the current one. Otherwise the present
/// parameters are saved, the default-pool objects released (0x7F7F70), `TestCooperativeLevel`, the
/// parameters set from the mode (0x7F6CB0 with the mode's flags and the create flags) and the device
/// `Reset`; on success the post-reset sequence, then the restore callback (0xC980B0) when set. When
/// the reset or the sequence failed: (only after a failed sequence: release again) the saved
/// parameters are put back and the device reset again (failure returns 0), the same sequence (result
/// ignored) and the restore callback; returns 1. On success: the current mode, the engine mode
/// globals (width, height, refresh, format, flags; depth 32 for formats 0x15 / 0x16 / 0x23, 16 for
/// 0x17-0x19, else 0) and the window: fullscreen sizes it to the mode (`AdjustWindowRectEx`,
/// `SetWindowPos` flags 6), windowed sets WS_OVERLAPPEDWINDOW | WS_VISIBLE, sizes it to the back buffer
/// (flags 0x26) and stores the client size as the engine width / height. Returns 1.
#[hook(addr = 0x7F8640, conv = cdecl, name = "RwD3D9ChangeVideoMode", expect = "A1 38 7C C9 00",
       unverified = "rw5-cam-1", route = "L3-only", hazards = "none", recs = "menus-hud",
       note = "device reset and window changes (OS effects); not double-runnable")]
pub fn change_video_mode(mode: i32) -> u32 {
    if g::<u32>(VIDEO_MODES_READY).read() == 0
        || mode < 0
        || mode >= g::<i32>(NUM_VIDEO_MODES).read()
        || g::<i32>(CURRENT_MODE).read() == mode
    {
        return 1;
    }
    let mut saved = [0u32; 14];
    for (k, v) in saved.iter_mut().enumerate() {
        *v = g::<u32>(PRESENT_PARAMS as usize + k * 4).read();
    }
    release_video_memory();
    dev_test_cooperative_level(device());
    let m = (mode as u32).wrapping_mul(0x14);
    let modes = g::<u32>(VIDEO_MODES).read();
    set_present_parameters(
        modes.wrapping_add(m),
        rd::<u32>(p8(modes.wrapping_add(m)), 0x10),
        g::<u32>(ADAPTER_CREATE_FLAGS).read(),
    );
    let mut ok = false;
    let reset_ok = dev_reset(device(), PRESENT_PARAMS) >= 0;
    if reset_ok {
        ok = after_reset();
        let cb = g::<u32>(RESTORE_CB).read();
        if cb != 0 {
            call_restore_cb(cb);
        }
    }
    if !ok {
        if reset_ok {
            release_video_memory();
        }
        for (k, v) in saved.iter().enumerate() {
            g::<u32>(PRESENT_PARAMS as usize + k * 4).write(*v);
        }
        if dev_reset(device(), PRESENT_PARAMS) < 0 {
            return 0;
        }
        after_reset();
        let cb = g::<u32>(RESTORE_CB).read();
        if cb != 0 {
            call_restore_cb(cb);
        }
        return 1;
    }
    g::<i32>(CURRENT_MODE).write(mode);
    let e = p8(g::<u32>(VIDEO_MODES).read().wrapping_add(m));
    let (w, h) = (rd::<u32>(e, 0), rd::<u32>(e, 4));
    g::<u32>(MODE_WIDTH).write(w);
    g::<u32>(MODE_WIDTH + 4).write(h);
    g::<u32>(MODE_WIDTH + 8).write(rd::<u32>(e, 8));
    g::<u32>(MODE_WIDTH + 0xC).write(rd::<u32>(e, 0xC));
    g::<u32>(MODE_WIDTH + 0x14).write(rd::<u32>(e, 0x10));
    let depth = match rd::<u32>(e, 0xC) {
        0x15 | 0x16 | 0x23 => 32,
        0x17..=0x19 => 16,
        _ => 0,
    };
    g::<u32>(MODE_WIDTH + 0x10).write(depth);
    if g::<u32>(PRESENT_PARAMS as usize + 0x20).read() == 0 {
        let mut r = [0, 0, w as i32, h as i32];
        fit_window(&mut r, 6);
    } else {
        set_window_long(g::<u32>(WINDOW).read(), -16, 0x10CF_0000);
        let mut r = [
            0,
            0,
            g::<i32>(PRESENT_PARAMS as usize).read(),
            g::<i32>(PRESENT_PARAMS as usize + 4).read(),
        ];
        fit_window(&mut r, 0x26);
        get_client_rect(g::<u32>(WINDOW).read(), GPtr::from_raw(r.as_mut_ptr()).cast());
        g::<i32>(MODE_WIDTH).write(r[2]);
        g::<i32>(MODE_WIDTH + 4).write(r[3]);
    }
    1
}

// ------------------------------------------------------------------ STD-4 -------------------------
// `_rwD3D9CameraBeginUpdate`, `_rwD3D9CameraClear`, `_rwD3D9RasterShowRaster` (they replace sa-game's
// `rw_standin.rs` stand-ins).

const RASTER_CONTEXT_RT: usize = 0xC98090;
const RASTER_CONTEXT_DS: usize = 0xC9808C;
/// The driver's "inside BeginScene" flag.
const IN_SCENE: usize = 0xC97C54;
const STENCIL_PRESENT: usize = 0xC97C3C;
const CLEAR_STENCIL: usize = 0xC97C44;
/// RW clear flags → `D3DCLEAR` (8 dwords).
const CLEAR_FLAGS: usize = 0x8847E0;
const D3D9_OBJECT: usize = 0xC97C20;
const ADAPTER: usize = 0xC97C24;
/// View matrix (0xC9BC80) and projection matrix (0x8E2458).
const VIEW: usize = 0xC9BC80;
const PROJ: usize = 0x8E2458;
const D3DERR_DEVICELOST: i32 = 0x8876_0868u32 as i32;
const D3DERR_DEVICENOTRESET: i32 = 0x8876_0869u32 as i32;
const D3DERR_INVALIDCALL: i32 = 0x8876_086Cu32 as i32;

orig! {
    /// `IDirect3DDevice9::SetRenderTarget(index, surface)` (+0x94)
    fn dev_set_render_target(dev: GPtr<u8>, index: u32, surface: u32) -> i32 = stdcall vslot 0x94 => unported;
    /// `IDirect3DDevice9::SetDepthStencilSurface(surface)` (+0x9C)
    fn dev_set_depth_stencil_surface(dev: GPtr<u8>, surface: u32) -> i32 = stdcall vslot 0x9C => unported;
    /// `IDirect3DDevice9::BeginScene()` (+0xA4)
    fn dev_begin_scene(dev: GPtr<u8>) -> i32 = stdcall vslot 0xA4 => unported;
    /// `IDirect3DDevice9::EndScene()` (+0xA8)
    fn dev_end_scene(dev: GPtr<u8>) -> i32 = stdcall vslot 0xA8 => unported;
    /// `IDirect3DDevice9::Clear(count, rects, flags, color, z, stencil)` (+0xAC)
    fn dev_clear(dev: GPtr<u8>, count: u32, rects: u32, flags: u32, color: u32, z: f32, stencil: u32) -> i32 = stdcall vslot 0xAC => unported;
    /// `IDirect3DDevice9::SetViewport(viewport)` (+0xBC)
    fn dev_set_viewport(dev: GPtr<u8>, vp: GPtr<u32>) -> i32 = stdcall vslot 0xBC => unported;
    /// `IDirect3DDevice9::Present(src, dst, hwnd, dirty)` (+0x44)
    fn dev_present(dev: GPtr<u8>, src: u32, dst: u32, hwnd: u32, dirty: u32) -> i32 = stdcall vslot 0x44 => unported;
    /// `IDirect3DSwapChain9::Present(src, dst, hwnd, dirty, flags)` (+0xC)
    fn swap_chain_present(sc: GPtr<u8>, src: u32, dst: u32, hwnd: u32, dirty: u32, flags: u32) -> i32 = stdcall vslot 0xC => unported;
    /// `IDirect3DSurface9::GetDesc(desc)` (+0x30)
    fn surf_get_desc(s: GPtr<u8>, desc: GPtr<u32>) -> i32 = stdcall vslot 0x30 => unported;
    /// `IDirect3D9::GetAdapterDisplayMode(adapter, out)` (+0x20)
    fn d3d_get_adapter_display_mode(d: GPtr<u8>, adapter: u32, out: GPtr<u32>) -> i32 = stdcall vslot 0x20 => unported;

    /// USER32 `IsIconic(hwnd)` (IAT 0x85825C)
    fn is_iconic(hwnd: u32) -> u32 = stdcall cell 0x85825C => unported;
    /// `_rwProcessorForceSinglePrecision()` (0x857432)
    fn force_single_precision() = cdecl 0x857432 => crate::d3d9sys::processor_force_single_precision;
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => crate::frame::get_ltm;
    /// `RwMatrixInvert(dst, src)` (0x7F2070)
    fn matrix_invert(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F2070 => crate::matrix::rw_matrix_invert;
    /// `RwD3D9SetTransform(state, matrix)` (0x7FA390)
    fn set_transform(state: u32, m: GPtr<u8>) -> u32 = cdecl 0x7FA390 => crate::d3d9dev::set_transform;
    /// `RwRasterDestroy(raster)` (0x7FB020)
    fn raster_destroy(r: GPtr<u8>) -> u32 = cdecl 0x7FB020 => crate::raster::rw_raster_destroy;
    /// `RwRasterCreate(width, height, depth, flags)` (0x7FB230)
    fn raster_create(w: i32, h: i32, depth: i32, flags: u32) -> GPtr<u8> = cdecl 0x7FB230 => crate::raster::rw_raster_create;
    /// `RwD3D9CameraAttachWindow(camera, hwnd)` (0x7F8D70)
    fn camera_attach_window(camera: GPtr<u8>, hwnd: u32) -> u32 = cdecl 0x7F8D70 => unported;
    /// `FUN_007f5800()`: the dynamic vertex buffers' frame advance
    fn dynamic_vertex_buffer_frame() = cdecl 0x7F5800 => crate::d3d9vb::dynamic_vertex_buffer_manager_frame;
}

fn rcb() {
    let cb = g::<u32>(RESTORE_CB).read();
    if cb != 0 {
        call_restore_cb(cb);
    }
}
fn ext_of(r: GPtr<u8>) -> GPtr<u8> {
    r.byte_add(g::<u32>(RASTER_EXT_OFFSET).read() as i32 as isize)
}
fn set_rt(s: u32) {
    if g::<u32>(RASTER_CONTEXT_RT).read() != s {
        g::<u32>(RASTER_CONTEXT_RT).write(s);
        dev_set_render_target(device(), 0, s);
    }
}
fn set_ds(s: u32) {
    if g::<u32>(RASTER_CONTEXT_DS).read() != s {
        g::<u32>(RASTER_CONTEXT_DS).write(s);
        dev_set_depth_stencil_surface(device(), s);
    }
}
fn viewport(x: u32, y: u32, w: u32, h: u32) {
    let mut vp = [x, y, w, h, 0, 0x3F80_0000];
    dev_set_viewport(device(), GPtr::from_raw(vp.as_mut_ptr()));
}
/// The level-0 surface of a camera texture's extension (cube face from the high nibble of +9).
fn texture_surface(e: GPtr<u8>) -> u32 {
    let mut s: u32 = 0;
    let cube = rd::<u8>(e, 9);
    let tex = p8(rd::<u32>(e, 0));
    if cube & 0xF != 0 {
        cube_get_cube_map_surface(tex, (cube >> 4) as u32, 0, GPtr::from_raw(&mut s));
    } else {
        tex_get_surface_level(tex, 0, GPtr::from_raw(&mut s));
    }
    s
}

/// A swap-chain camera's Z raster when the window's client size `(w, h)` equals the frame buffer's
/// original size: a top-level Z raster of another size is destroyed and re-created at that size
/// (`RwRasterCreate(w, h, 0, ZBUFFER)`, kept when it succeeds). Returns the camera's Z raster
/// (the new one, or NULL when the re-creation failed).
fn swap_chain_zbuffer(camera: GPtr<u8>, w: i32, h: i32) -> u32 {
    let z = rd::<u32>(camera, 0x64);
    if z == 0 || rd::<u32>(p8(z), 0) != z || (w == rd::<i32>(p8(z), 0xC) && h == rd::<i32>(p8(z), 0x10)) {
        return z;
    }
    wr::<u32>(camera, 0x64, 0);
    raster_destroy(p8(z));
    let nz = raster_create(w, h, 0, 1);
    if !nz.is_null() {
        wr::<u32>(camera, 0x64, nz.addr() as u32);
    }
    nz.addr() as u32
}

/// The swap-chain branch shared by begin update and clear: the client size of the extension's
/// window (+0x20; empty: `None`), the Z raster re-created at that size or the window attached
/// (`RwD3D9CameraAttachWindow`, when the frame buffer is its own parent), the swap chain's surface
/// and the Z raster's surface (or none) as render target / depth. Returns the frame buffer.
fn swap_chain_targets(camera: GPtr<u8>) -> Option<GPtr<u8>> {
    let mut fb = p8(rd::<u32>(camera, 0x60));
    let top = p8(rd::<u32>(fb, 0));
    let mut e = ext_of(top);
    let mut rc = [0i32; 4];
    get_client_rect(rd::<u32>(e, 0x20), GPtr::from_raw(rc.as_mut_ptr()).cast());
    if rc[2] == 0 || rc[3] == 0 {
        return None;
    }
    let z = if rc[2] == rd::<i32>(top, 0x28) && rc[3] == rd::<i32>(top, 0x2C) {
        swap_chain_zbuffer(camera, rc[2], rc[3])
    } else {
        if fb == top {
            camera_attach_window(camera, rd::<u32>(e, 0x20));
            fb = p8(rd::<u32>(camera, 0x60));
            e = ext_of(p8(rd::<u32>(fb, 0)));
        }
        rd::<u32>(camera, 0x64)
    };
    set_rt(rd::<u32>(e, 0));
    if z != 0 {
        set_ds(rd::<u32>(ext_of(p8(rd::<u32>(p8(z), 0))), 0));
    } else if g::<u32>(RASTER_CONTEXT_DS).read() != 0 {
        g::<u32>(RASTER_CONTEXT_DS).write(0);
        dev_set_depth_stencil_surface(device(), 0);
    }
    Some(fb)
}

/// The main-window resize shared by begin update and clear (client size `rc` differs from the
/// engine mode): default-pool objects released, a windowed device takes the client size (when
/// `TestCooperativeLevel` succeeds), `Reset` and the post-reset sequence (+ restore callback); on
/// success the client size becomes the engine size. Otherwise (released again after a failed
/// sequence) a fullscreen device fails; a windowed one is reset at the engine size (failure:
/// false), the window resized to it (maximised style removed, `AdjustWindowRectEx`,
/// `SetWindowPos` flags 6) and the post-reset sequence run again (+ callback).
fn resize_device(rc: &[i32; 4]) -> bool {
    release_video_memory();
    if dev_test_cooperative_level(device()) >= 0 && g::<u32>(PRESENT_PARAMS as usize + 0x20).read() != 0 {
        g::<i32>(PRESENT_PARAMS as usize).write(rc[2]);
        g::<i32>(PRESENT_PARAMS as usize + 4).write(rc[3]);
    }
    if dev_reset(device(), PRESENT_PARAMS) >= 0 {
        let ok = after_reset();
        rcb();
        if ok {
            g::<i32>(MODE_WIDTH).write(rc[2]);
            g::<i32>(MODE_WIDTH + 4).write(rc[3]);
            return true;
        }
        release_video_memory();
    }
    if g::<u32>(PRESENT_PARAMS as usize + 0x20).read() == 0 {
        return false;
    }
    g::<u32>(PRESENT_PARAMS as usize + 4).write(g::<u32>(MODE_WIDTH + 4).read());
    g::<u32>(PRESENT_PARAMS as usize).write(g::<u32>(MODE_WIDTH).read());
    if dev_reset(device(), PRESENT_PARAMS) < 0 {
        return false;
    }
    let mut r = [
        rc[0],
        rc[1],
        g::<i32>(MODE_WIDTH).read().wrapping_add(rc[0]),
        g::<i32>(MODE_WIDTH + 4).read().wrapping_add(rc[1]),
    ];
    if get_window_long(g::<u32>(WINDOW).read(), -16) & 0x100_0000 != 0 {
        let s = get_window_long(g::<u32>(WINDOW).read(), -16);
        set_window_long(g::<u32>(WINDOW).read(), -16, s & 0xFEFF_FFFF);
    }
    fit_window(&mut r, 6);
    after_reset();
    rcb();
    true
}

/// The main window's client rectangle for begin update / clear: `None` when empty or minimised;
/// a size other than the engine mode's resizes the device first ([`resize_device`]; `None` when
/// that fails).
fn main_window_ready() -> Option<()> {
    let mut rc = [0i32; 4];
    get_client_rect(g::<u32>(WINDOW).read(), GPtr::from_raw(rc.as_mut_ptr()).cast());
    if rc[2] == 0 || rc[3] == 0 || is_iconic(g::<u32>(WINDOW).read()) != 0 {
        return None;
    }
    if (rc[2] != g::<i32>(MODE_WIDTH).read() || rc[3] != g::<i32>(MODE_WIDTH + 4).read()) && !resize_device(&rc) {
        return None;
    }
    Some(())
}

/// `_rwD3D9CameraBeginUpdate(out, camera, flags)` (0x7F8F20, `stdFunc[1]`). x87 single precision
/// (0x857432); the current camera (0xC9BCC0); the view matrix = inverse of the frame's LTM with the
/// first column negated and the last column (0, 0, 0, 1), set as D3DTS_VIEW; the projection (view
/// window, view offset, near / far; parallel: z scale 1/(far-near), w 1; perspective: far/(far-near),
/// w from z) as D3DTS_PROJECTION (x87 single-precision arithmetic). Then the targets: a swap-chain
/// frame buffer (extension +0x1C) its swap chain's surface and the camera's Z raster; the main
/// window (client area empty or minimised: 0; another size: device resize) a camera texture's level
/// surface when it was created as a render target (with the Z raster's surface, or a level-0
/// surface of a non-Z raster), else the back buffer with the copy-at-end-update flag 0xC97C34; any
/// other raster the back buffer and depth surface. The viewport is the frame buffer's sub-rectangle.
/// `TestCooperativeLevel`: DEVICENOTRESET resets the device (an INVALIDCALL reset of a windowed
/// device retries with the desktop format) and runs the post-reset sequence; a failure clears the
/// in-scene flag and returns 0. Otherwise `BeginScene` unless already in a scene; returns the
/// in-scene flag.
#[hook(addr = 0x7F8F20, conv = cdecl, name = "WindowedModeHooks::RenderWare::_rwD3D9CameraBeginUpdate", expect = "83 EC 50 53 55 56 57",
       unverified = "std4-d3d9-2", route = "L2", hazards = "none", recs = "boot,menus-hud",
       note = "x87 control word; device reset / window paths need OS effects (L3)")]
pub fn camera_begin_update(_out: u32, camera: GPtr<u8>, _flags: u32) -> u32 {
    use sa_math::x87::X87;
    let ld = |a: usize| X87::from_f32(g::<f32>(a).read());
    let st = |a: usize, v: X87| g::<f32>(a).write(v.to_f32());
    let cf = |o: isize| X87::from_f32(rd::<f32>(camera, o));
    force_single_precision();
    g::<u32>(CURRENT_CAMERA).write(camera.addr() as u32);
    let ltm = frame_get_ltm(p8(rd::<u32>(camera, 4)));
    matrix_invert(p8(VIEW as u32), ltm);
    for o in [0, 0x10, 0x20, 0x30] {
        st(VIEW + o, ld(VIEW + o).fchs());
    }
    g::<u32>(VIEW + 0xC).write(0);
    g::<u32>(VIEW + 0x1C).write(0);
    g::<u32>(VIEW + 0x2C).write(0);
    g::<u32>(VIEW + 0x3C).write(0x3F80_0000);
    set_transform(2, p8(VIEW as u32));
    g::<u32>(PROJ).write(rd::<u32>(camera, 0x70));
    g::<u32>(PROJ + 0x14).write(rd::<u32>(camera, 0x74));
    st(PROJ + 0x20, cf(0x78).fmul_m(rd::<f32>(camera, 0x70)));
    st(PROJ + 0x24, cf(0x7C).fmul_m(rd::<f32>(camera, 0x74)));
    st(PROJ + 0x30, cf(0x78).fmul_m(rd::<f32>(camera, 0x70)).fchs());
    st(PROJ + 0x34, cf(0x7C).fmul_m(rd::<f32>(camera, 0x74)).fchs());
    let d = cf(0x84).fsub_m(rd::<f32>(camera, 0x80));
    let zs = if rd::<u32>(camera, 0x14) == 2 {
        g::<u32>(PROJ + 0x2C).write(0);
        g::<u32>(PROJ + 0x3C).write(0x3F80_0000);
        d.fdivr_m(1.0)
    } else {
        g::<u32>(PROJ + 0x2C).write(0x3F80_0000);
        g::<u32>(PROJ + 0x3C).write(0);
        d.fdivr_m(rd::<f32>(camera, 0x84))
    };
    st(PROJ + 0x28, zs);
    st(PROJ + 0x38, ld(PROJ + 0x28).fmul_m(rd::<f32>(camera, 0x80)).fchs());
    set_transform(3, p8(PROJ as u32));
    g::<u32>(0xC97C64).write(0);

    let fb0 = p8(rd::<u32>(camera, 0x60));
    let top = p8(rd::<u32>(fb0, 0));
    let fb = if rd::<u32>(ext_of(top), 0x1C) != 0 {
        match swap_chain_targets(camera) {
            Some(fb) => fb,
            None => return 0,
        }
    } else {
        if main_window_ready().is_none() {
            return 0;
        }
        if rd::<u8>(top, 0x20) == 5 {
            let e = ext_of(top);
            let surf = texture_surface(e);
            let mut desc = [0u32; 8];
            surf_get_desc(p8(surf), GPtr::from_raw(desc.as_mut_ptr()));
            if desc[2] & 1 != 0 {
                set_rt(surf);
                let z = rd::<u32>(camera, 0x64);
                if z != 0 {
                    let zp = p8(rd::<u32>(p8(z), 0));
                    let zt = rd::<u32>(ext_of(zp), 0);
                    if rd::<u8>(zp, 0x20) & 7 == 1 {
                        set_ds(zt);
                    } else {
                        let mut zs: u32 = 0;
                        tex_get_surface_level(p8(zt), 0, GPtr::from_raw(&mut zs));
                        set_ds(zs);
                        release(p8(zs));
                    }
                } else if g::<u32>(RASTER_CONTEXT_DS).read() != 0 {
                    g::<u32>(RASTER_CONTEXT_DS).write(0);
                    dev_set_depth_stencil_surface(device(), 0);
                }
            } else {
                g::<u32>(NEED_COPY).write(1);
                set_rt(g::<u32>(RENDER_SURFACE).read());
                set_ds(g::<u32>(DEPTH_SURFACE as usize).read());
            }
            release(p8(surf));
        } else {
            set_rt(g::<u32>(RENDER_SURFACE).read());
            set_ds(g::<u32>(DEPTH_SURFACE as usize).read());
        }
        fb0
    };
    viewport(
        rd::<i16>(fb, 0x1C) as i32 as u32,
        rd::<i16>(fb, 0x1E) as i32 as u32,
        rd::<u32>(fb, 0xC),
        rd::<u32>(fb, 0x10),
    );
    let mut hr = dev_test_cooperative_level(device());
    if hr == D3DERR_DEVICENOTRESET {
        release_video_memory();
        hr = dev_reset(device(), PRESENT_PARAMS);
        if hr == D3DERR_INVALIDCALL {
            if g::<u32>(PRESENT_PARAMS as usize + 0x20).read() == 0 {
                g::<u32>(IN_SCENE).write(0);
                return 0;
            }
            let mut dm = [0u32; 4];
            d3d_get_adapter_display_mode(p8(g::<u32>(D3D9_OBJECT).read()), g::<u32>(ADAPTER).read(), GPtr::from_raw(dm.as_mut_ptr()));
            g::<u32>(MODE_WIDTH + 0xC).write(dm[3]);
            g::<u32>(PRESENT_PARAMS as usize + 8).write(dm[3]);
            g::<u32>(MODE_WIDTH + 0x10).write(crate::d3d9sys::format_depth(dm[3]));
            hr = dev_reset(device(), PRESENT_PARAMS);
        }
        if hr < 0 {
            g::<u32>(IN_SCENE).write(0);
            return 0;
        }
        after_reset();
        rcb();
    }
    if hr < 0 {
        g::<u32>(IN_SCENE).write(0);
        return 0;
    }
    if g::<u32>(IN_SCENE).read() == 0 && dev_begin_scene(device()) >= 0 {
        g::<u32>(IN_SCENE).write(1);
    }
    g::<u32>(IN_SCENE).read()
}

/// `_rwD3D9CameraClear(camera, RwRGBA*, flags)` (0x7F7730, `stdFunc[0x15]`): the colour as D3DCOLOR
/// (0 without rwCAMERACLEARIMAGE), targets as in begin update (swap chain: its surface and the Z
/// raster, viewport at 0, 0; main window: empty / minimised client area 0, other size resizes the
/// device), the frame buffer's viewport, then `Clear(0, NULL, flags, colour, 1.0, [0xC97C44])`:
/// swap chain and camera texture with the flag table 0x8847E0 (a camera texture without Z raster
/// drops Z / stencil); a camera texture that is not a render target is cleared on the back buffer
/// and that rectangle copied into the texture (`StretchRect`, point filter when the caps allow);
/// other rasters IMAGE / Z as requested and stencil with Z or STENCIL when the depth format has a
/// stencil. Returns 1.
#[hook(addr = 0x7F7730, conv = cdecl, name = "WindowedModeHooks::RenderWare::_rwD3D9CameraClear", expect = "83 EC 68 8B 44 24 74",
       unverified = "std4-d3d9-2", route = "L2", hazards = "none", recs = "boot,menus-hud",
       note = "device reset / window paths need OS effects (L3)")]
pub fn camera_clear(camera: GPtr<u8>, color: GPtr<u8>, flags: u32) -> u32 {
    let mut c: u32 = 0;
    let image = flags & 1;
    if image != 0 {
        c = ((rd::<u8>(color, 3) as u32) << 24)
            | ((rd::<u8>(color, 0) as u32) << 16)
            | ((rd::<u8>(color, 1) as u32) << 8)
            | rd::<u8>(color, 2) as u32;
    }
    let stencil = || g::<u32>(CLEAR_STENCIL).read();
    let table = |f: u32| g::<u32>(CLEAR_FLAGS + 4 * f as usize).read();
    let fb0 = p8(rd::<u32>(camera, 0x60));
    let top = p8(rd::<u32>(fb0, 0));
    if rd::<u32>(ext_of(top), 0x1C) != 0 {
        let Some(fb) = swap_chain_targets(camera) else { return 0 };
        viewport(0, 0, rd::<u32>(fb, 0xC), rd::<u32>(fb, 0x10));
        dev_clear(device(), 0, 0, table(flags), c, 1.0, stencil());
        return 1;
    }
    if main_window_ready().is_none() {
        return 0;
    }
    let fb = fb0;
    let vp = || {
        viewport(
            rd::<i16>(fb, 0x1C) as i32 as u32,
            rd::<i16>(fb, 0x1E) as i32 as u32,
            rd::<u32>(fb, 0xC),
            rd::<u32>(fb, 0x10),
        )
    };
    if rd::<u8>(top, 0x20) == 5 {
        let e = ext_of(top);
        let surf = texture_surface(e);
        let mut desc = [0u32; 8];
        surf_get_desc(p8(surf), GPtr::from_raw(desc.as_mut_ptr()));
        let status: i32 = if desc[2] & 1 != 0 {
            set_rt(surf);
            let z = rd::<u32>(camera, 0x64);
            if z != 0 {
                let zp = p8(rd::<u32>(p8(z), 0));
                let zt = rd::<u32>(ext_of(zp), 0);
                if rd::<u8>(zp, 0x20) & 7 == 1 {
                    set_ds(zt);
                } else {
                    let mut zs: u32 = 0;
                    tex_get_surface_level(p8(zt), 0, GPtr::from_raw(&mut zs));
                    set_ds(zs);
                    release(p8(zs));
                }
            } else if g::<u32>(RASTER_CONTEXT_DS).read() != 0 {
                g::<u32>(RASTER_CONTEXT_DS).write(0);
                dev_set_depth_stencil_surface(device(), 0);
            }
            0
        } else {
            0x8000_4005u32 as i32
        };
        vp();
        let mut cfl = table(flags);
        if cfl & 6 != 0 && rd::<u32>(camera, 0x64) == 0 {
            cfl &= !6;
        }
        dev_clear(device(), 0, 0, cfl, c, 1.0, stencil());
        if status < 0 {
            let x = rd::<i16>(fb, 0x1C) as i32;
            let y = rd::<i16>(fb, 0x1E) as i32;
            let mut rect = [
                x as u32,
                y as u32,
                rd::<u32>(fb, 0xC).wrapping_add(x as u32),
                rd::<u32>(fb, 0x10).wrapping_add(y as u32),
            ];
            let filter = (g::<u32>(STRETCH_CAPS).read() & 0x100_0100 != 0) as u32;
            let rp: GPtr<u8> = GPtr::from_raw(rect.as_mut_ptr()).cast();
            dev_stretch_rect(device(), g::<u32>(RENDER_SURFACE).read(), rp, surf, rp, filter);
        }
        release(p8(surf));
        return 1;
    }
    set_rt(g::<u32>(RENDER_SURFACE).read());
    set_ds(g::<u32>(DEPTH_SURFACE as usize).read());
    vp();
    let mut cfl = (image != 0) as u32;
    if flags & 2 != 0 {
        cfl |= 2;
    }
    if (flags & 2 != 0 || flags & 4 != 0) && g::<u32>(STENCIL_PRESENT).read() != 0 {
        cfl |= 4;
    }
    dev_clear(device(), 0, 0, cfl, c, 1.0, stencil());
    1
}

/// `_rwD3D9RasterShowRaster(raster, -, flags)` (0x7F99B0, `stdFunc[0x14]`): ends an open scene;
/// a change of the vsync request (rwRASTERFLIPWAITVSYNC vs. the presentation interval, ONE /
/// IMMEDIATE) resets the device with the new interval and runs the post-reset sequence; then
/// `Present` on the raster's swap chain (extension +0x1C) or the device. DEVICELOST: when
/// `TestCooperativeLevel` says DEVICENOTRESET, reset and post-reset sequence. The dynamic vertex
/// buffers advance (0x7F5800). Returns `hr >= 0` of the last of Present / TestCooperativeLevel.
#[hook(addr = 0x7F99B0, conv = cdecl, name = "_rwD3D9RasterShowRaster", expect = "A1 54 7C C9 00 85 C0",
       unverified = "std4-d3d9-2", route = "L3-only", hazards = "none", recs = "boot,menus-hud",
       note = "presents the frame (OS effect)")]
pub fn raster_show_raster(raster: GPtr<u8>, _dev: u32, flags: u32) -> u32 {
    if g::<u32>(IN_SCENE).read() != 0 {
        dev_end_scene(device());
        g::<u32>(IN_SCENE).write(0);
    }
    let interval = PRESENT_PARAMS as usize + 0x34;
    let change = if flags & 1 != 0 {
        if g::<u32>(interval).read() == 0x8000_0000 {
            g::<u32>(interval).write(1);
            true
        } else {
            false
        }
    } else if g::<u32>(interval).read() == 1 {
        g::<u32>(interval).write(0x8000_0000);
        true
    } else {
        false
    };
    if change {
        release_video_memory();
        dev_reset(device(), PRESENT_PARAMS);
        after_reset();
        rcb();
    }
    let sc = rd::<u32>(ext_of(p8(rd::<u32>(raster, 0))), 0x1C);
    let mut hr = if sc != 0 {
        swap_chain_present(p8(sc), 0, 0, 0, 0, 0)
    } else {
        dev_present(device(), 0, 0, 0, 0)
    };
    if hr == D3DERR_DEVICELOST {
        hr = dev_test_cooperative_level(device());
        if hr == D3DERR_DEVICENOTRESET {
            release_video_memory();
            dev_reset(device(), PRESENT_PARAMS);
            after_reset();
            rcb();
        }
    }
    dynamic_vertex_buffer_frame();
    (hr >= 0) as u32
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_CHANGE_MULTISAMPLING_LEVELS,
    HOOK_CAMERA_END_UPDATE,
    HOOK_CHANGE_VIDEO_MODE,
    HOOK_CAMERA_BEGIN_UPDATE,
    HOOK_CAMERA_CLEAR,
    HOOK_RASTER_SHOW_RASTER,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
