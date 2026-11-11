//! PORT-RW-5 module `d3d9cam`: D3D9 camera begin / end update and the driver's system entry (see
//! docs/arch/rw-port.md, PORT-RW-5). Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, p8, rd};
use crate::d3d9state::device;

/// The camera being updated (set by begin update, cleared by end update).
const CURRENT_CAMERA: usize = 0xC9BCC0;
/// `D3D9RasterExtOffset`: the raster plugin's extension offset.
const RASTER_EXT_OFFSET: usize = 0xB4E9E0;
/// The back-buffer surface StretchRect copies from.
const RENDER_SURFACE: usize = 0xC97C30;
/// Set when a camera texture must receive the frame at end update.
const NEED_COPY: usize = 0xC97C34;
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
       unverified = "rw5-cam-1", route = "L2", hazards = "uninit-locals", recs = "grove-walk")]
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

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CAMERA_END_UPDATE, HOOK_CHANGE_VIDEO_MODE];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
