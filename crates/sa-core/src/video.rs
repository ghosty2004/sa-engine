//! CORE ports: `VideoMode::*` (video mode list of the RenderWare D3D9 subsystem) and `VideoPlayer::*`
//! (DirectShow playback of the intro movies) of gta_sa.exe 1.0 US (see `docs/arch/core-port.md`,
//! fork C1). Imports, COM methods and RenderWare as in [`crate::platform`].

use crate::mem::GPtr;
use crate::platform::{protected_cell_sub, rdata};
use crate::{globals, hook, orig};
use sa_math::x87::X87;

globals! {
    list GLOBALS;
    /// `gVideoModes` (`char**`)
    pub static VIDEO_MODES @ 0xC920D0: u32 = 0;
    /// `gCurrentGpu` (the subsystem the list was built for)
    pub static CURRENT_GPU @ 0x8D6248: u32 = 0xFFFF_FFFF;
    /// `gCurrentVideoMode`
    pub static CURRENT_VIDEO_MODE @ 0x8D6220: i32 = -1;
    /// `GcurSelSS` (selected subsystem = D3D adapter)
    pub static CUR_SEL_SS @ 0xC920F4: u32 = 0;
    /// Free video memory measured at start-up (`s_OSStatus.VRAM.Avail`, PsGlobal area)
    pub static VRAM_AVAIL @ 0xC8CF80: u32 = 0;
    /// `pvGraphBuilder` (IGraphBuilder*)
    pub static GRAPH_BUILDER @ 0xC920D4: u32 = 0;
    /// `pvMediaControl` (IMediaControl*)
    pub static MEDIA_CONTROL @ 0xC920D8: u32 = 0;
    /// `pvMediaEvent` (IMediaEventEx*)
    pub static MEDIA_EVENT @ 0xC920DC: u32 = 0;
    /// `pvVideoWindow` (IVideoWindow*)
    pub static VIDEO_WINDOW @ 0xC920E0: u32 = 0;
    /// `pvMediaSeeking` (IMediaSeeking*)
    pub static MEDIA_SEEKING @ 0xC920E4: u32 = 0;
    /// `gGameState` (app.h; read and advanced by OnGraphNotify)
    pub static GAME_STATE @ 0xC8D4C0: i32 = 0;
    /// Protection constants of `GetVideoModeList`'s entry chunk (0x402012): `.HOODLUM` dword
    pub static PROT_VML_A @ 0x156070C: u32 = 0x009A_9E66;
    /// ... minus this `.text` dword (`clc; sbb`) = &gVideoModes
    pub static PROT_VML_B @ 0x6ECF06: u32 = 0xFFD1_7D96;
}

/// A COM interface pointer held in a global, as the `this` of a `vslot` call.
fn com(p: u32) -> GPtr<u32> {
    GPtr::from_addr(p as usize)
}

/// `RwVideoMode` (rwcore.h): width, height, depth, flags, refRate, format.
pub type RwVideoMode = [i32; 6];

pub mod call {
    use super::*;
    orig! {
        /// `RwEngineGetNumVideoModes` (0x7F2CC0)
        pub fn rw_engine_get_num_video_modes() -> i32 = cdecl 0x7F2CC0 => unported;
        /// `RwEngineGetCurrentSubSystem` (0x7F2C60)
        pub fn rw_engine_get_current_sub_system() -> u32 = cdecl 0x7F2C60 => unported;
        /// `RwEngineGetVideoModeInfo` (0x7F2CF0)
        pub fn rw_engine_get_video_mode_info(info: GPtr<RwVideoMode>, mode: i32) -> GPtr<RwVideoMode> = cdecl 0x7F2CF0 => unported;
        /// `RwD3D9ChangeVideoMode` (0x7F8640)
        pub fn rw_d3d9_change_video_mode(mode: i32) -> i32 = cdecl 0x7F8640 => unported;
        /// `CPostEffects::SetupBackBufferVertex` (0x7043D0)
        pub fn setup_back_buffer_vertex() = cdecl 0x7043D0 => unported;
        /// `CMemoryMgr::Calloc` (0x72F460) as called here: `(num, size, 0)`
        pub fn memory_calloc(num: u32, size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F460 => unported;
        /// `RwEngineInstance->stringFuncs.vecSprintf`-style `rwsprintf` (+0xF0) with 3 values
        pub fn rw_sprintf3(buf: GPtr<u8>, fmt: GPtr<u8>, a: u32, b: u32, c: u32) -> i32 = cdecl table 0xC97B24 0xF0 => unported;
        /// `VideoMode::FreeVideoModeList` (0x745A80)
        pub fn free_video_mode_list() -> i32 = cdecl 0x745A80 => super::free_video_mode_list;
        /// `VideoPlayer::Release` (0x746740)
        pub fn release() = cdecl 0x746740 => super::release;
        /// `VideoPlayer::UpdateWindow` (0x7466D0)
        pub fn update_window() -> i32 = cdecl 0x7466D0 => super::update_window;
        /// D3D9 `Direct3DCreate9` (import thunk 0x807C2B)
        pub fn direct3d_create9(sdk: u32) -> u32 = stdcall 0x807C2B => unported;
        /// `IDirect3D9::GetAdapterModeCount` (+0x18)
        pub fn d3d_get_adapter_mode_count(d3d: GPtr<u32>, adapter: u32, fmt: u32) -> i32 = stdcall vslot 0x18 => unported;
        /// `IDirect3D9::EnumAdapterModes` (+0x1C)
        pub fn d3d_enum_adapter_modes(d3d: GPtr<u32>, adapter: u32, fmt: u32, mode: i32, out: GPtr<[u32; 4]>) -> i32 = stdcall vslot 0x1C => unported;
        /// `IUnknown::QueryInterface` (+0)
        pub fn com_query_interface(obj: GPtr<u32>, iid: GPtr<u8>, out: GPtr<u32>) -> i32 = stdcall vslot 0x00 => unported;
        /// `IUnknown::Release` (+8)
        pub fn com_release(obj: GPtr<u32>) -> u32 = stdcall vslot 0x08 => unported;
        /// `IMediaControl::Run` (+0x1C)
        pub fn media_control_run(obj: GPtr<u32>) -> i32 = stdcall vslot 0x1C => unported;
        /// `IMediaControl::Stop` (+0x24)
        pub fn media_control_stop(obj: GPtr<u32>) -> i32 = stdcall vslot 0x24 => unported;
        /// `IMediaControl::GetState` (+0x28)
        pub fn media_control_get_state(obj: GPtr<u32>, timeout: u32, state: GPtr<i32>) -> i32 = stdcall vslot 0x28 => unported;
        /// `IMediaEventEx::GetEvent` (+0x20)
        pub fn media_event_get_event(obj: GPtr<u32>, code: GPtr<i32>, p1: GPtr<u32>, p2: GPtr<u32>, timeout: u32) -> i32 = stdcall vslot 0x20 => unported;
        /// `IMediaEventEx::FreeEventParams` (+0x30)
        pub fn media_event_free_event_params(obj: GPtr<u32>, code: i32, p1: u32, p2: u32) -> i32 = stdcall vslot 0x30 => unported;
        /// `IMediaEventEx` slot +0x34 (`SetNotifyWindow(hwnd, msg, instance)`)
        pub fn media_event_slot34(obj: GPtr<u32>, a: u32, b: u32, c: u32) -> i32 = stdcall vslot 0x34 => unported;
        /// `IGraphBuilder::RenderFile` (+0x34)
        pub fn graph_render_file(obj: GPtr<u32>, file: GPtr<u16>, list: u32) -> i32 = stdcall vslot 0x34 => unported;
        /// `IVideoWindow::put_WindowStyle` (+0x24)
        pub fn video_window_put_window_style(obj: GPtr<u32>, style: u32) -> i32 = stdcall vslot 0x24 => unported;
        /// `IVideoWindow::put_Owner` (+0x74)
        pub fn video_window_put_owner(obj: GPtr<u32>, owner: u32) -> i32 = stdcall vslot 0x74 => unported;
        /// `IVideoWindow::put_MessageDrain` (+0x7C)
        pub fn video_window_put_message_drain(obj: GPtr<u32>, drain: u32) -> i32 = stdcall vslot 0x7C => unported;
        /// `IVideoWindow::SetWindowPosition` (+0x9C)
        pub fn video_window_set_window_position(obj: GPtr<u32>, l: i32, t: i32, w: i32, h: i32) -> i32 = stdcall vslot 0x9C => unported;
        /// USER32 `GetClientRect` at 0x7466E1
        pub fn get_client_rect(hwnd: u32, r: GPtr<[i32; 4]>) -> i32 = stdcall site 0x7466E1 => unported;
        /// USER32 `SetFocus` at 0x74672C (UpdateWindow)
        pub fn set_focus_update(hwnd: u32) -> u32 = stdcall site 0x74672C => unported;
        /// USER32 `UpdateWindow` at 0x74766E
        pub fn user_update_window(hwnd: u32) -> i32 = stdcall site 0x74766E => unported;
        /// KERNEL32 `MultiByteToWideChar` at 0x74768C
        pub fn multi_byte_to_wide_char(cp: u32, flags: u32, s: GPtr<u8>, n: i32, w: GPtr<u16>, wn: i32) -> i32 = stdcall site 0x74768C => unported;
        /// ole32 `CoInitialize` at 0x747694
        pub fn co_initialize_play(r: u32) -> i32 = stdcall site 0x747694 => unported;
        /// ole32 `CoCreateInstance` at 0x7476B5
        pub fn co_create_instance(clsid: GPtr<u8>, outer: u32, ctx: u32, iid: GPtr<u8>, out: GPtr<u32>) -> i32 = stdcall site 0x7476B5 => unported;
        /// USER32 `SetFocus` at 0x7477B0 (Play)
        pub fn set_focus_play(hwnd: u32) -> u32 = stdcall site 0x7477B0 => unported;
    }
}

fn ps_window() -> u32 {
    crate::platform::RS_PS.get().cast::<u32>().read()
}

/// `VideoMode::FreeVideoModeList` (0x745A80): with a list, `RwFree` each of the
/// `RwEngineGetNumVideoModes()` entries (signed count, list re-read per entry), `RwFree` the list,
/// clear it. Returns TRUE either way.
#[hook(addr = 0x745A80, conv = cdecl, name = "VideoMode::FreeVideoModeList", expect = "57 E8 3A D2 0A 00")]
pub fn free_video_mode_list() -> i32 {
    let n = call::rw_engine_get_num_video_modes();
    if VIDEO_MODES.get() != 0 {
        for i in 0..n.max(0) {
            let e = GPtr::<u32>::from_addr(VIDEO_MODES.get() as usize)
                .wrapping_index(i)
                .read();
            crate::platform::call::rw_free(GPtr::from_addr(e as usize));
        }
        crate::platform::call::rw_free(GPtr::from_addr(VIDEO_MODES.get() as usize));
        VIDEO_MODES.set(0);
    }
    1
}

/// `rsVIDEOMODEEXCLUSIVE` modes that `GetVideoModeList` lists: at least 640x480, aspect
/// `height / width` (x87 `fild`/`fidiv`, PC=24, stored as f32) exactly one of 0.8, 0.75, 0.6, 0.625
/// (`fcomp` + `test ah, 44h`: NaN never matches), and, unless exactly 640x480,
/// `vram - depth*width*height/8 > 0x2D00000` (32-bit wrapping products, signed division by 8,
/// unsigned compare).
pub fn mode_is_listed(m: &RwVideoMode, vram: u32) -> bool {
    let (w, h, depth, flags) = (m[0], m[1], m[2], m[3]);
    if flags & 1 == 0 {
        return false;
    }
    let ratio = X87::from_i32(h).fdiv(X87::from_i32(w)).to_f32();
    if w < 640 || h < 480 {
        return false;
    }
    // 0x858C98 0.8, 0x858F34 0.75, 0x858CC8 0.6, 0x8652DC 0.625
    let ok = [0x3F4C_CCCDu32, 0x3F40_0000, 0x3F19_999A, 0x3F20_0000]
        .iter()
        .any(|&c| ratio == f32::from_bits(c));
    if !ok {
        return false;
    }
    if w == 640 && h == 480 {
        return true;
    }
    let bytes = depth.wrapping_mul(w).wrapping_mul(h) / 8;
    vram.wrapping_sub(bytes as u32) > 0x2D0_0000
}

/// `VideoMode::GetVideoModeList` (0x745AF0, entry `jmp` into the protection chunk 0x402012 that reads
/// `gVideoModes`). An existing list built for the current subsystem is returned as is; one for
/// another subsystem is freed. Then `gCurrentGpu = RwEngineGetCurrentSubSystem()` and a fresh list of
/// `n = RwEngineGetNumVideoModes()` entries (`CMemoryMgr::Calloc(n, 4, 0)`); entry *i* is
/// `Calloc(100, 1, 0)` holding `"%lu X %lu X %lu"` (rwsprintf) for listed modes ([`mode_is_listed`])
/// and 0 otherwise (also stored explicitly). The loop count is signed (gta-reversed: unsigned).
#[hook(addr = 0x745AF0, conv = cdecl, name = "VideoMode::GetVideoModeList", expect = "E9 1D C5 CB FF 83")]
pub fn get_video_mode_list() -> u32 {
    if protected_cell_sub(&PROT_VML_A, &PROT_VML_B, VIDEO_MODES.ptr()) != 0 {
        if CURRENT_GPU.get() == call::rw_engine_get_current_sub_system() {
            return VIDEO_MODES.get();
        }
        call::free_video_mode_list();
    }
    CURRENT_GPU.set(call::rw_engine_get_current_sub_system());
    let n = call::rw_engine_get_num_video_modes();
    VIDEO_MODES.set(call::memory_calloc(n as u32, 4, 0).addr() as u32);
    for i in 0..n.max(0) {
        let mut m: RwVideoMode = [0; 6];
        call::rw_engine_get_video_mode_info(GPtr::from_raw(&raw mut m), i);
        let slot = |v: u32| {
            GPtr::<u32>::from_addr(VIDEO_MODES.get() as usize)
                .wrapping_index(i)
                .write(v)
        };
        if mode_is_listed(&m, VRAM_AVAIL.get()) {
            let s = call::memory_calloc(0x64, 1, 0);
            slot(s.addr() as u32);
            call::rw_sprintf3(s, rdata(0x874AC0), m[0] as u32, m[1] as u32, m[2] as u32);
        } else {
            slot(0);
        }
    }
    VIDEO_MODES.get()
}

/// `VideoMode::SetVideoMode(int32)` (0x745C70): `RwD3D9ChangeVideoMode(mode);
/// CPostEffects::SetupBackBufferVertex()`; RsGlobal.maximumWidth/Height = the camera frame buffer's
/// width/height (`Scene.camera->frameBuffer` at +0x60, raster +0x0C/+0x10).
#[hook(addr = 0x745C70, conv = cdecl, name = "VideoMode::SetVideoMode", expect = "8B 44 24 04 50")]
pub fn set_video_mode(mode: i32) {
    call::rw_d3d9_change_video_mode(mode);
    call::setup_back_buffer_vertex();
    let cam = crate::platform::SCENE_CAMERA.get();
    let fb = |o: isize| {
        let raster = cam.byte_add(0x60).cast::<u32>().read();
        GPtr::<i32>::from_addr(raster as usize).byte_add(o).read()
    };
    crate::platform::RS_MAXIMUM_WIDTH.set(fb(0x0C));
    crate::platform::RS_MAXIMUM_HEIGHT.set(fb(0x10));
}

/// `VideoMode::IsVideoModeExclusive` (0x745CA0): `RwEngineGetVideoModeInfo(&m, gCurrentVideoMode);
/// m.flags & 1` in AL.
#[hook(addr = 0x745CA0, conv = cdecl, name = "VideoMode::IsVideoModeExclusive", expect = "A1 20 62 8D 00")]
pub fn is_video_mode_exclusive() -> u8 {
    let mut m: RwVideoMode = [0; 6];
    call::rw_engine_get_video_mode_info(GPtr::from_raw(&raw mut m), CURRENT_VIDEO_MODE.get());
    (m[3] & 1) as u8
}

/// The D3D format `GetBestRefreshRate` enumerates for a colour depth: 32 → X8R8G8B8 (0x16),
/// 24 → R8G8B8 (0x14), else R5G6B5 (0x17) (`((depth != 24) - 1) & ~2) + 0x17`).
pub fn d3d_format_for_depth(depth: u32) -> u32 {
    if depth == 32 {
        0x16
    } else {
        ((((depth != 24) as u32).wrapping_sub(1)) & 0xFFFF_FFFD).wrapping_add(0x17)
    }
}

/// `GetBestRefreshRate(width, height, depth)` (0x7460A0): over the adapter `GcurSelSS`'s modes of the
/// matching format with exactly this width and height, a refresh rate of 0 returns 0 at once
/// (after `Release`); otherwise the lowest rate >= 60 (unsigned compares) is returned, or -1 if none.
#[hook(addr = 0x7460A0, conv = cdecl, name = "GetBestRefreshRate", expect = "83 EC 10 53 55 56 57")]
pub fn get_best_refresh_rate(width: u32, height: u32, depth: u32) -> u32 {
    let d3d = com(call::direct3d_create9(0x1F));
    let mut best = 0x7FFF_FFFFu32;
    let fmt = d3d_format_for_depth(depth);
    let n = call::d3d_get_adapter_mode_count(d3d, CUR_SEL_SS.get(), fmt);
    let mut i = 0;
    while i < n {
        let mut dm = [0u32; 4];
        call::d3d_enum_adapter_modes(d3d, CUR_SEL_SS.get(), fmt, i, GPtr::from_raw(&raw mut dm));
        if dm[0] == width && dm[1] == height && dm[3] == fmt {
            let rate = dm[2];
            if rate == 0 {
                call::com_release(d3d);
                return 0;
            }
            if rate < best && rate >= 0x3C {
                best = rate;
            }
        }
        i += 1;
    }
    call::com_release(d3d);
    if best == 0x7FFF_FFFF { 0xFFFF_FFFF } else { best }
}

/// `VideoPlayer::WaitState(OAFilterState)` (0x745F90): `pvMediaControl->GetState(10, &s)` until
/// `s == state` (the control pointer re-read each time); returns the last HRESULT.
#[hook(addr = 0x745F90, conv = cdecl, name = "VideoPlayer::WaitState", expect = "56 8B 74 24 08")]
pub fn wait_state(state: i32) -> i32 {
    loop {
        let mut s: i32 = 0;
        let hr = call::media_control_get_state(com(MEDIA_CONTROL.get()), 10, GPtr::from_raw(&raw mut s));
        if s == state {
            return hr;
        }
    }
}

/// `VideoPlayer::OnGraphNotify` (0x745FB0): drains `pvMediaEvent->GetEvent(&code, &p1, &p2, 0)`
/// (while it succeeds); each event is freed (`FreeEventParams(code, p1, p2)`, whose result is
/// returned); on EC_COMPLETE (1) `gGameState` 2 → 3, 4 → 5 and slot 0x34 of the event interface is
/// called with (0, 0, 0). Returns 0 when there was no event.
#[hook(addr = 0x745FB0, conv = cdecl, name = "VideoPlayer::OnGraphNotify", expect = "A1 DC 20 C9 00")]
pub fn on_graph_notify() -> i32 {
    let mut result = 0;
    let (mut code, mut p1, mut p2) = (0i32, 0u32, 0u32);
    let get = |c: &mut i32, a: &mut u32, b: &mut u32| {
        call::media_event_get_event(
            com(MEDIA_EVENT.get()),
            GPtr::from_raw(c),
            GPtr::from_raw(a),
            GPtr::from_raw(b),
            0,
        )
    };
    while get(&mut code, &mut p1, &mut p2) >= 0 {
        result = call::media_event_free_event_params(com(MEDIA_EVENT.get()), code, p1, p2);
        if code == 1 {
            match GAME_STATE.get() {
                2 => GAME_STATE.set(3),
                4 => GAME_STATE.set(5),
                _ => {}
            }
            call::media_event_slot34(com(MEDIA_EVENT.get()), 0, 0, 0);
        }
    }
    result
}

/// `VideoPlayer::UpdateWindow` (0x7466D0): `GetClientRect(window, &r)`;
/// `hr = pvVideoWindow->SetWindowPosition(r.left, r.top, r.right, r.bottom)`; on failure `hr`; else
/// `hr = pvVideoWindow->put_MessageDrain(window); SetFocus(window); hr`.
#[hook(addr = 0x7466D0, conv = cdecl, name = "VideoPlayer::UpdateWindow", expect = "8B 0D 54 70 C1 00")]
pub fn update_window() -> i32 {
    let mut r = [0i32; 4];
    call::get_client_rect(ps_window(), GPtr::from_raw(&raw mut r));
    let hr = call::video_window_set_window_position(com(VIDEO_WINDOW.get()), r[0], r[1], r[2], r[3]);
    if hr < 0 {
        return hr;
    }
    let hr = call::video_window_put_message_drain(com(VIDEO_WINDOW.get()), ps_window());
    call::set_focus_update(ps_window());
    hr
}

/// `VideoPlayer::Release` (0x746740): `Release` and clear, in this order, pvMediaEvent,
/// pvMediaSeeking, pvMediaControl, pvVideoWindow, pvGraphBuilder (each only if set).
#[hook(addr = 0x746740, conv = cdecl, name = "VideoPlayer::Release", expect = "A1 DC 20 C9 00 56")]
pub fn release() {
    for g in [
        &MEDIA_EVENT,
        &MEDIA_SEEKING,
        &MEDIA_CONTROL,
        &VIDEO_WINDOW,
        &GRAPH_BUILDER,
    ] {
        let p = g.get();
        if p != 0 {
            call::com_release(com(p));
        }
        g.set(0);
    }
}

/// `VideoPlayer::Shutdown` (0x7467B0): `if (pvMediaControl) pvMediaControl->Stop(); Release()`.
#[hook(addr = 0x7467B0, conv = cdecl, name = "VideoPlayer::Shutdown", expect = "A1 D8 20 C9 00")]
pub fn shutdown() {
    let c = MEDIA_CONTROL.get();
    if c != 0 {
        call::media_control_stop(com(c));
    }
    call::release();
}

/// `VideoPlayer::Play(int nCmdShow, const char* path)` (0x747660): `UpdateWindow(window)`; the path
/// to UTF-16 (255 chars); `CoInitialize`; `CoCreateInstance(CLSID_FilterGraph 0x873EE8, NULL,
/// CLSCTX_INPROC 3, IID_IGraphBuilder 0x88700C, &pvGraphBuilder)`; `RenderFile(path, NULL)`;
/// QueryInterface for IMediaControl (0x874708), IMediaEventEx (0x8746E8), IMediaSeeking (0x8870DC),
/// IVideoWindow (0x8746B8); `put_Owner(window)`; `put_WindowStyle(WS_CHILD | WS_CLIPSIBLINGS |
/// WS_CLIPCHILDREN 0x46000000)`; `pvMediaEvent->SetNotifyWindow(window, 0x40D, 0)`; `UpdateWindow()`;
/// `pvMediaControl->Run()`; `SetFocus(window)`. Stops at the first failing step (no cleanup; the
/// gta-reversed `FIX_BUGS` blocks are not followed).
#[hook(addr = 0x747660, conv = cdecl, name = "VideoPlayer::Play", expect = "A1 54 70 C1 00 8B 08")]
pub fn play(_cmd_show: i32, path: GPtr<u8>) {
    call::user_update_window(ps_window());
    let mut wide = [0u16; 0x100];
    call::multi_byte_to_wide_char(0, 0, path, -1, GPtr::from_raw(wide.as_mut_ptr()), 0xFF);
    if call::co_initialize_play(0) < 0 {
        return;
    }
    if call::co_create_instance(rdata(0x873EE8), 0, 3, rdata(0x88700C), GRAPH_BUILDER.ptr()) < 0 {
        return;
    }
    if call::graph_render_file(com(GRAPH_BUILDER.get()), GPtr::from_raw(wide.as_mut_ptr()), 0) < 0 {
        return;
    }
    for (iid, out) in [
        (0x874708, &MEDIA_CONTROL),
        (0x8746E8, &MEDIA_EVENT),
        (0x8870DC, &MEDIA_SEEKING),
        (0x8746B8, &VIDEO_WINDOW),
    ] {
        if call::com_query_interface(com(GRAPH_BUILDER.get()), rdata(iid), out.ptr()) < 0 {
            return;
        }
    }
    if call::video_window_put_owner(com(VIDEO_WINDOW.get()), ps_window()) < 0 {
        return;
    }
    if call::video_window_put_window_style(com(VIDEO_WINDOW.get()), 0x4600_0000) < 0 {
        return;
    }
    if call::media_event_slot34(com(MEDIA_EVENT.get()), ps_window(), 0x40D, 0) < 0 {
        return;
    }
    call::update_window();
    if call::media_control_run(com(MEDIA_CONTROL.get())) < 0 {
        return;
    }
    call::set_focus_play(ps_window());
}

mod sel {
    use crate::mem::GPtr;
    crate::orig! {
        /// `RwEngineGetNumSubSystems` (0x7F2C00)
        pub fn rw_engine_get_num_sub_systems() -> i32 = cdecl 0x7F2C00 => unported;
        /// `RwEngineGetSubSystemInfo(RwSubSystemInfo*, index)` (0x7F2C30)
        pub fn rw_engine_get_sub_system_info(info: GPtr<u8>, index: i32) -> GPtr<u8> = cdecl 0x7F2C30 => unported;
        /// `RwEngineSetSubSystem(index)` (0x7F2C90)
        pub fn rw_engine_set_sub_system(index: u32) -> i32 = cdecl 0x7F2C90 => unported;
        /// `RwEngineSetVideoMode(index)` (0x7F2D50)
        pub fn rw_engine_set_video_mode(index: i32) -> i32 = cdecl 0x7F2D50 => unported;
        /// `RwD3D9EngineSetRefreshRate(rate)` (0x7F8580)
        pub fn rw_d3d9_engine_set_refresh_rate(rate: u32) -> i32 = cdecl 0x7F8580 => unported;
        /// `RwD3D9EngineSetMultiSamplingLevels(levels)` (0x7F84F0)
        pub fn rw_d3d9_engine_set_multi_sampling_levels(levels: u32) -> i32 = cdecl 0x7F84F0 => unported;
        /// `GetBestRefreshRate(width, height, depth)` (0x7460A0)
        pub fn get_best_refresh_rate(width: u32, height: u32, depth: u32) -> u32 = cdecl 0x7460A0 => super::get_best_refresh_rate;
        /// `VideoMode::GetVideoModeList()` (0x745AF0)
        pub fn get_video_mode_list() -> u32 = cdecl 0x745AF0 => super::get_video_mode_list;
        /// USER32 `DialogBoxParamA` (IAT 0x8582CC)
        pub fn dialog_box_param_a(instance: u32, template: u32, parent: u32, proc_: u32, param: u32) -> i32 = stdcall cell 0x8582CC => unported;
        /// USER32 `MessageBoxA` (IAT 0x8582F4)
        pub fn message_box_a(hwnd: u32, text: GPtr<u8>, caption: GPtr<u8>, ty: u32) -> i32 = stdcall cell 0x8582F4 => unported;
    }
}

globals! {
    list SELECT_GLOBALS;
    /// `UseDefaultVM`
    pub static USE_DEFAULT_VM @ 0xC920FC: u32 = 0;
    /// `GnumSubSystems`
    pub static NUM_SUB_SYSTEMS @ 0xC920F0: i32 = 0;
    /// `MultipleSubSystems`
    pub static MULTIPLE_SUB_SYSTEMS @ 0xC92118: u32 = 0;
    /// `IsVMNotSelected`
    pub static IS_VM_NOT_SELECTED @ 0x8D6218: u32 = 1;
}

/// `GsubSysInfo[16]` (0x50-byte `RwSubSystemInfo` each).
const SUB_SYS_INFO: usize = 0xC8CFC0;
/// The time-stamp counter `psSelectDevice` stores (high dword, low dword).
const SELECT_TSC_HI: usize = 0xC92104;
const SELECT_TSC_LO: usize = 0xC92108;
/// `FrontEndMenuManager` fields: m_nCurrentScreenItem, m_nPrefsAntialiasing, m_nPrefsVideoMode,
/// m_nDisplayVideoMode.
const MENU_CURRENT_SCREEN_ITEM: usize = 0xBA679C;
const MENU_PREFS_ANTIALIASING: usize = 0xBA6810;
const MENU_PREFS_VIDEO_MODE: usize = 0xBA681C;
const MENU_DISPLAY_VIDEO_MODE: usize = 0xBA6820;

/// `rdtsc` as (high, low); 0 off x86.
fn rdtsc() -> (u32, u32) {
    #[cfg(target_arch = "x86")]
    #[allow(unsafe_code)]
    {
        // SAFETY: reads the time-stamp counter.
        let t = unsafe { core::arch::x86::_rdtsc() };
        ((t >> 32) as u32, t as u32)
    }
    #[cfg(not(target_arch = "x86"))]
    {
        (0, 0)
    }
}

/// `Ps::psSelectDevice` (0x746190), as the 1.0 US bytes differ from gta-reversed: unless
/// `UseDefaultVM`, the subsystem count (0 → FALSE; at most 16), their infos, the current one, and
/// the time-stamp counter stored at 0xC92104/0xC92108. More than one subsystem: the video-mode
/// dialog (`DialogBoxParamA(instance, 0x68, window, 0x745E50, 0)` unless `UseDefaultVM`), set the
/// subsystem. One: set the subsystem; unless `UseDefaultVM`, a listed `m_nDisplayVideoMode` (non-zero)
/// becomes the preferred and current mode, else the default search: from `gCurrentVideoMode` up,
/// the first mode when `IsVMNotSelected` is clear, otherwise the first exclusive 800x600x32 (none →
/// `MessageBoxA` and FALSE). Then the mode's info, `m_nCurrentScreenItem = 0`, set the mode
/// (failure → FALSE); an exclusive mode sets the best refresh rate (when there is one), the
/// maximum size and `ps->fullScreen`; the antialiasing preference becomes the multisampling level.
/// The mode-info local is uninitialised in the original where a query fails (port: zero).
/// Written by STD-3, not yet verified.
#[hook(addr = 0x746190, conv = cdecl, name = "Ps::psSelectDevice", expect = "A1 FC 20 C9 00 83 EC 1C")]
pub fn ps_select_device() -> i32 {
    let ps = crate::platform::RS_PS.get().cast::<u32>();
    let (mut window, mut instance) = (0u32, 0u32);
    if USE_DEFAULT_VM.get() == 0 {
        window = ps.read();
        instance = ps.wrapping_index(1).read();
        let n = sel::rw_engine_get_num_sub_systems();
        NUM_SUB_SYSTEMS.set(n);
        if n == 0 {
            return 0;
        }
        if n > 16 {
            NUM_SUB_SYSTEMS.set(16);
        }
        let mut i = 0;
        while i < NUM_SUB_SYSTEMS.get() {
            sel::rw_engine_get_sub_system_info(GPtr::from_addr(SUB_SYS_INFO + 0x50 * i as usize), i);
            i += 1;
        }
        CUR_SEL_SS.set(call::rw_engine_get_current_sub_system());
        let (hi, lo) = rdtsc();
        GPtr::<u32>::from_addr(SELECT_TSC_HI).write(hi);
        GPtr::<u32>::from_addr(SELECT_TSC_LO).write(lo);
    }
    let mut vmi: RwVideoMode = [0; 6];
    let vmi_p = GPtr::from_raw(&raw mut vmi);
    if NUM_SUB_SYSTEMS.get() > 1 {
        MULTIPLE_SUB_SYSTEMS.set(1);
        if USE_DEFAULT_VM.get() == 0 && sel::dialog_box_param_a(instance, 0x68, window, 0x745E50, 0) == 0 {
            return 0;
        }
        if sel::rw_engine_set_sub_system(CUR_SEL_SS.get()) == 0 {
            return 0;
        }
    } else {
        MULTIPLE_SUB_SYSTEMS.set(0);
        if sel::rw_engine_set_sub_system(CUR_SEL_SS.get()) == 0 {
            return 0;
        }
        if USE_DEFAULT_VM.get() == 0 {
            let list = sel::get_video_mode_list();
            let vm = GPtr::<i32>::from_addr(MENU_DISPLAY_VIDEO_MODE).read();
            let listed = GPtr::<u32>::from_addr(list as usize).wrapping_index(vm).read() != 0;
            if listed && vm != 0 {
                GPtr::<i32>::from_addr(MENU_PREFS_VIDEO_MODE).write(vm);
                CURRENT_VIDEO_MODE.set(vm);
            } else {
                // the default-mode search (0x746362)
                loop {
                    let n = call::rw_engine_get_num_video_modes();
                    let cur = CURRENT_VIDEO_MODE.get();
                    if cur >= n {
                        sel::message_box_a(0, rdata(0x874AD0), rdata(0x86D388), 0);
                        return 0;
                    }
                    call::rw_engine_get_video_mode_info(vmi_p, cur);
                    if IS_VM_NOT_SELECTED.get() == 0
                        || (vmi[0] == 800 && vmi[1] == 600 && vmi[2] == 32 && vmi[3] & 1 != 0)
                    {
                        break;
                    }
                    CURRENT_VIDEO_MODE.set(cur.wrapping_add(1));
                }
            }
        }
    }
    call::rw_engine_get_video_mode_info(vmi_p, CURRENT_VIDEO_MODE.get());
    GPtr::<u32>::from_addr(MENU_CURRENT_SCREEN_ITEM).write(0);
    if sel::rw_engine_set_video_mode(CURRENT_VIDEO_MODE.get()) == 0 {
        return 0;
    }
    if vmi[3] & 1 != 0 {
        let rr = sel::get_best_refresh_rate(vmi[0] as u32, vmi[1] as u32, vmi[2] as u32);
        if rr != 0xFFFF_FFFF {
            sel::rw_d3d9_engine_set_refresh_rate(rr);
        }
        if vmi[3] & 1 != 0 {
            crate::platform::RS_MAXIMUM_WIDTH.set(vmi[0]);
            crate::platform::RS_MAXIMUM_HEIGHT.set(vmi[1]);
            ps.wrapping_index(2).write(1);
        }
    }
    sel::rw_d3d9_engine_set_multi_sampling_levels(GPtr::<u32>::from_addr(MENU_PREFS_ANTIALIASING).read());
    1
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_FREE_VIDEO_MODE_LIST,
    HOOK_GET_VIDEO_MODE_LIST,
    HOOK_SET_VIDEO_MODE,
    HOOK_IS_VIDEO_MODE_EXCLUSIVE,
    HOOK_GET_BEST_REFRESH_RATE,
    HOOK_WAIT_STATE,
    HOOK_ON_GRAPH_NOTIFY,
    HOOK_UPDATE_WINDOW,
    HOOK_RELEASE,
    HOOK_SHUTDOWN,
    HOOK_PLAY,
    HOOK_PS_SELECT_DEVICE,
];

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    #[test]
    fn listed_modes() {
        let m = |w, h, d, f| [w, h, d, f, 60, 0];
        assert!(mode_is_listed(&m(640, 480, 32, 1), 0));
        assert!(!mode_is_listed(&m(640, 480, 32, 0), u32::MAX));
        assert!(mode_is_listed(
            &m(800, 480, 32, 1),
            0x2D0_0000 + 800 * 480 * 4 + 1
        ));
        assert!(!mode_is_listed(&m(800, 480, 32, 1), 0x2D0_0000 + 800 * 480 * 4));
        assert!(mode_is_listed(&m(1280, 1024, 32, 1), 0x1000_0000)); // 0.8
        assert!(!mode_is_listed(&m(1280, 720, 32, 1), u32::MAX)); // 0.5625
        assert!(!mode_is_listed(&m(320, 240, 32, 1), u32::MAX));
        // vram smaller than the frame buffer wraps (unsigned compare)
        assert!(mode_is_listed(&m(1024, 768, 32, 1), 0));
    }

    #[test]
    fn formats() {
        assert_eq!(d3d_format_for_depth(32), 0x16);
        assert_eq!(d3d_format_for_depth(24), 0x14);
        assert_eq!(d3d_format_for_depth(16), 0x17);
    }

    #[test]
    fn protected_list_cell() {
        assert_eq!(PROT_VML_A.get().wrapping_sub(PROT_VML_B.get()), 0xC920D0);
    }
}
