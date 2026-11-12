//! Native DirectX / COM objects other than DirectInput ([`crate::com`]): the DLL probing of
//! `psInitialize`'s DirectX check (`LoadLibraryA`/`GetProcAddress`/`FreeLibrary`),
//! `IDirectDraw7` (`Ps::GetVideoMemInfo`), `IDirectSound` (`Ps::CheckDirectSound`), and the
//! `IDirect3D9` / `IDirect3DDevice9` objects the RW D3D9 driver code reaches.
//!
//! Same scheme as [`crate::com`]: the objects live on a page sa-game maps at [`PAGE`], their
//! vtables hold trap addresses, and every implemented method is an entry of `orig!`'s address
//! registry; an unimplemented method fails loudly as `IDirect3DDevice9::<method>`.
//!
//! Platform values chosen here are not measured on the phase A setup (docs/arch/platform.md §8,
//! "Platform values"): every probed DLL exists (DirectX 9 present), 256 MiB of video memory
//! (all of it available), `DirectSoundCreate` succeeds.

use sa_core::mem::GPtr;

use crate::bind::{cstr, with};

/// The page holding the objects and vtables.
pub const PAGE: usize = 0x0003_1000;
/// Method trap addresses: `TRAP + 0x400 * class + 4 * slot`.
const TRAP: u32 = 0x0005_0000;
/// Video memory `IDirectDraw7::GetAvailableVidMem` reports (total and free).
const VIDEO_MEMORY: u32 = 256 << 20;

/// COM classes: name, object address, vtable address, method names (vtable order).
struct Class {
    name: &'static str,
    object: usize,
    vtable: usize,
    methods: &'static [&'static str],
}

const DD7_METHODS: [&str; 30] = [
    "QueryInterface", "AddRef", "Release", "Compact", "CreateClipper", "CreatePalette",
    "CreateSurface", "DuplicateSurface", "EnumDisplayModes", "EnumSurfaces", "FlipToGDISurface",
    "GetCaps", "GetDisplayMode", "GetFourCCCodes", "GetGDISurface", "GetMonitorFrequency",
    "GetScanLine", "GetVerticalBlankStatus", "Initialize", "RestoreDisplayMode",
    "SetCooperativeLevel", "SetDisplayMode", "WaitForVerticalBlank", "GetAvailableVidMem",
    "GetSurfaceFromDC", "RestoreAllSurfaces", "TestCooperativeLevel", "GetDeviceIdentifier",
    "StartModeTest", "EvaluateMode",
];
const DS_METHODS: [&str; 11] = [
    "QueryInterface", "AddRef", "Release", "CreateSoundBuffer", "GetCaps", "DuplicateSoundBuffer",
    "SetCooperativeLevel", "Compact", "GetSpeakerConfig", "SetSpeakerConfig", "Initialize",
];

const D3D9_METHODS: [&str; 17] = [
    "QueryInterface", "AddRef", "Release", "RegisterSoftwareDevice", "GetAdapterCount",
    "GetAdapterIdentifier", "GetAdapterModeCount", "EnumAdapterModes", "GetAdapterDisplayMode",
    "CheckDeviceType", "CheckDeviceFormat", "CheckDeviceMultiSampleType", "CheckDepthStencilMatch",
    "CheckDeviceFormatConversion", "GetDeviceCaps", "GetAdapterMonitor", "CreateDevice",
];
const DEVICE9_METHODS: [&str; 119] = [
    "QueryInterface", "AddRef", "Release", "TestCooperativeLevel", "GetAvailableTextureMem",
    "EvictManagedResources", "GetDirect3D", "GetDeviceCaps", "GetDisplayMode",
    "GetCreationParameters", "SetCursorProperties", "SetCursorPosition", "ShowCursor",
    "CreateAdditionalSwapChain", "GetSwapChain", "GetNumberOfSwapChains", "Reset", "Present",
    "GetBackBuffer", "GetRasterStatus", "SetDialogBoxMode", "SetGammaRamp", "GetGammaRamp",
    "CreateTexture", "CreateVolumeTexture", "CreateCubeTexture", "CreateVertexBuffer",
    "CreateIndexBuffer", "CreateRenderTarget", "CreateDepthStencilSurface", "UpdateSurface",
    "UpdateTexture", "GetRenderTargetData", "GetFrontBufferData", "StretchRect", "ColorFill",
    "CreateOffscreenPlainSurface", "SetRenderTarget", "GetRenderTarget", "SetDepthStencilSurface",
    "GetDepthStencilSurface", "BeginScene", "EndScene", "Clear", "SetTransform", "GetTransform",
    "MultiplyTransform", "SetViewport", "GetViewport", "SetMaterial", "GetMaterial", "SetLight",
    "GetLight", "LightEnable", "GetLightEnable", "SetClipPlane", "GetClipPlane", "SetRenderState",
    "GetRenderState", "CreateStateBlock", "BeginStateBlock", "EndStateBlock", "SetClipStatus",
    "GetClipStatus", "GetTexture", "SetTexture", "GetTextureStageState", "SetTextureStageState",
    "GetSamplerState", "SetSamplerState", "ValidateDevice", "SetPaletteEntries",
    "GetPaletteEntries", "SetCurrentTexturePalette", "GetCurrentTexturePalette", "SetScissorRect",
    "GetScissorRect", "SetSoftwareVertexProcessing", "GetSoftwareVertexProcessing",
    "SetNPatchMode", "GetNPatchMode", "DrawPrimitive", "DrawIndexedPrimitive", "DrawPrimitiveUP",
    "DrawIndexedPrimitiveUP", "ProcessVertices", "CreateVertexDeclaration",
    "SetVertexDeclaration", "GetVertexDeclaration", "SetFVF", "GetFVF", "CreateVertexShader",
    "SetVertexShader", "GetVertexShader", "SetVertexShaderConstantF", "GetVertexShaderConstantF",
    "SetVertexShaderConstantI", "GetVertexShaderConstantI", "SetVertexShaderConstantB",
    "GetVertexShaderConstantB", "SetStreamSource", "GetStreamSource", "SetStreamSourceFreq",
    "GetStreamSourceFreq", "SetIndices", "GetIndices", "CreatePixelShader", "SetPixelShader",
    "GetPixelShader", "SetPixelShaderConstantF", "GetPixelShaderConstantF",
    "SetPixelShaderConstantI", "GetPixelShaderConstantI", "SetPixelShaderConstantB",
    "GetPixelShaderConstantB", "DrawRectPatch", "DrawTriPatch", "DeletePatch", "CreateQuery",
];

const DD7: usize = PAGE + 0x10;
const DS: usize = PAGE + 0x20;
/// The `IDirect3D9` object `Direct3DCreate9` returns.
pub const D3D9: usize = PAGE + 0x30;
/// The `IDirect3DDevice9` object `CreateDevice` returns (methods in [`crate::d3d9`]).
pub const DEVICE9: usize = PAGE + 0x40;
const CLASSES: [Class; 4] = [
    Class { name: "IDirectDraw7", object: DD7, vtable: PAGE + 0x100, methods: &DD7_METHODS },
    Class { name: "IDirectSound", object: DS, vtable: PAGE + 0x200, methods: &DS_METHODS },
    Class { name: "IDirect3D9", object: D3D9, vtable: PAGE + 0x300, methods: &D3D9_METHODS },
    Class { name: "IDirect3DDevice9", object: DEVICE9, vtable: PAGE + 0x400, methods: &DEVICE9_METHODS },
];

/// The display modes the native adapter reports for a format, `(width, height, refresh)`: the
/// usual 4:3/16:9 sizes at 60 Hz in X8R8G8B8 (0x16) and R5G6B5 (0x17), none in A2R10G10B10.
/// Not measured (docs/arch/platform.md §8, platform values).
pub const ADAPTER_MODES: [(u32, u32, u32); 7] =
    [(640, 480, 60), (800, 600, 60), (1024, 768, 60), (1280, 720, 60), (1280, 1024, 60), (1600, 900, 60), (1920, 1080, 60)];
/// The desktop mode (`GetAdapterDisplayMode`): width, height, refresh, format.
pub const DESKTOP_MODE: [u32; 4] = [1920, 1080, 60, 0x16];

fn modes_for(format: u32) -> &'static [(u32, u32, u32)] {
    if format == 0x16 || format == 0x17 { &ADAPTER_MODES } else { &[] }
}

/// `D3DCAPS9` (0x130 bytes) of the native device, as `(offset, value)`: a hardware T&L device
/// without programmable shaders (the wgpu seam runs fixed function only, platform.md §4.1), DXT,
/// 2048² textures, anisotropy 16, 8 lights, 8 stages / 4 simultaneous textures (the seam honours
/// stages 0–1). Not measured.
pub const CAPS9: [(usize, u32); 34] = [
    (0x00, 1),            // DeviceType HAL
    (0x08, 0x0002_0000),  // Caps: READ_SCANLINE
    (0x0C, 0x6002_0000),  // Caps2: FULLSCREENGAMMA | CANAUTOGENMIPMAP | DYNAMICTEXTURES
    (0x10, 0x0000_03A0),  // Caps3
    (0x14, 0x8000_000F),  // PresentationIntervals
    (0x1C, 0x001B_BEF0),  // DevCaps (incl. HWTRANSFORMANDLIGHT 0x10000)
    (0x20, 0x000E_CEF2),  // PrimitiveMiscCaps
    (0x24, 0x0737_A1B1),  // RasterCaps (incl. ANISOTROPY 0x20000)
    (0x28, 0xFF),         // ZCmpCaps
    (0x2C, 0x3FFF),       // SrcBlendCaps
    (0x30, 0x3FFF),       // DestBlendCaps
    (0x34, 0xFF),         // AlphaCmpCaps
    (0x38, 0x0004_2208),  // ShadeCaps
    (0x3C, 0x0001_EC45),  // TextureCaps
    (0x40, 0x0703_0700),  // TextureFilterCaps
    (0x4C, 0x3F),         // TextureAddressCaps
    (0x58, 2048),         // MaxTextureWidth
    (0x5C, 2048),         // MaxTextureHeight
    (0x64, 8192),         // MaxTextureRepeat
    (0x68, 2048),         // MaxTextureAspectRatio
    (0x6C, 16),           // MaxAnisotropy
    (0x70, 0x4B18_967F),  // MaxVertexW 1e10
    (0x88, 0x01FF),       // StencilCaps
    (0x8C, 0x0018_0008),  // FVFCaps: 8 texcoords
    (0x90, 0x03FF_FFFF),  // TextureOpCaps
    (0x94, 8),            // MaxTextureBlendStages
    (0x98, 4),            // MaxSimultaneousTextures
    (0x9C, 0x13B),        // VertexProcessingCaps
    (0xA0, 8),            // MaxActiveLights
    (0xA4, 6),            // MaxUserClipPlanes
    (0xB4, 0x00FF_FFFF),  // MaxPrimitiveCount
    (0xB8, 0x00FF_FFFF),  // MaxVertexIndex
    (0xC4, 0xFFFE_0000),  // VertexShaderVersion: none
    (0xCC, 0xFFFF_0000),  // PixelShaderVersion: none
];
/// Fields of `D3DCAPS9` set after the table (MaxStreams, MaxStreamStride, NumSimultaneousRTs,
/// MaxPointSize 64.0).
pub const CAPS9_MORE: [(usize, u32); 4] = [(0xBC, 16), (0xC0, 0xFF), (0xF0, 1), (0xB0, 0x4280_0000)];

/// Writes the native `D3DCAPS9` to `out` (0x130 bytes).
pub fn write_caps9(out: GPtr<u8>) {
    for i in 0..0x130 / 4 {
        out.cast::<u32>().wrapping_index(i).write(0);
    }
    for (o, v) in CAPS9.iter().chain(CAPS9_MORE.iter()) {
        out.byte_add(*o as isize).cast::<u32>().write(*v);
    }
    crate::d3d9_shader::patch_caps9(out);
}

/// The trap address of method `slot` of class `class` (0 DirectDraw7, 1 DirectSound, 2 Direct3D9,
/// 3 Direct3DDevice9).
pub fn trap(class: usize, slot: usize) -> u32 {
    TRAP + 0x400 * class as u32 + 4 * slot as u32
}

/// Trap of `DirectDrawCreateEx` as `GetProcAddress` returns it.
const TRAP_DD_CREATE_EX: u32 = TRAP + 0x3F00;

/// Maps the page and writes the objects' vtable pointers and the vtables.
#[allow(unsafe_code)]
pub fn map() -> Result<(), String> {
    crate::arena::map_fixed(PAGE, 0x1000).map_err(|e| format!("{}: {e}", format!("cannot map the DirectX object page at {PAGE:#x}")))?;
    let w = |a: usize, v: u32| GPtr::<u32>::from_addr(a).write(v);
    for (c, cl) in CLASSES.iter().enumerate() {
        w(cl.object, cl.vtable as u32);
        for i in 0..cl.methods.len() {
            w(cl.vtable + 4 * i, trap(c, i));
        }
    }
    Ok(())
}

/// Every method trap's name.
pub fn names() -> Vec<(u32, &'static str)> {
    let mut v = Vec::new();
    for (c, cl) in CLASSES.iter().enumerate() {
        for (i, m) in cl.methods.iter().enumerate() {
            v.push((trap(c, i), &*Box::leak(format!("{}::{m}", cl.name).into_boxed_str())));
        }
    }
    v.push((TRAP_DD_CREATE_EX, "ddraw.dll!DirectDrawCreateEx (GetProcAddress)"));
    v
}

macro_rules! entries {
    ($( $addr:expr, $name:literal => $f:path : fn($($a:ident : $t:ty),*) -> $r:ty ;)*) => {
        /// `(address, name, native stdcall entry)` of every implemented method and entry point.
        pub fn entries() -> Vec<(u32, &'static str, usize)> {
            let mut v: Vec<(u32, &'static str, usize)> = Vec::new();
            $({
                #[cfg(target_arch = "x86")]
                extern "stdcall-unwind" fn w($($a: $t),*) -> $r {
                    $f($($a),*)
                }
                #[cfg(not(target_arch = "x86"))]
                extern "C-unwind" fn w($($a: $t),*) -> $r {
                    $f($($a),*)
                }
                v.push(($addr, $name, {
                    #[cfg(target_arch = "wasm32")]
                    let e = sa_core::wabi_wrap!(w; $($a)*);
                    #[cfg(not(target_arch = "wasm32"))]
                    let e = w as *const () as usize;
                    e
                }));
            })*
            v
        }
    };
}

entries! {
    trap(0, 1), "IDirectDraw7::AddRef" => add_ref: fn(this: u32) -> u32;
    trap(0, 2), "IDirectDraw7::Release" => release: fn(this: u32) -> u32;
    trap(0, 23), "IDirectDraw7::GetAvailableVidMem" => dd_get_available_vid_mem: fn(this: u32, caps: GPtr<u32>, total: GPtr<u32>, free: GPtr<u32>) -> i32;
    trap(1, 1), "IDirectSound::AddRef" => add_ref: fn(this: u32) -> u32;
    trap(1, 2), "IDirectSound::Release" => release: fn(this: u32) -> u32;
    trap(1, 4), "IDirectSound::GetCaps" => ds_get_caps: fn(this: u32, caps: GPtr<u32>) -> i32;
    TRAP_DD_CREATE_EX, "ddraw.dll!DirectDrawCreateEx (GetProcAddress)" => direct_draw_create_ex: fn(guid: u32, out: GPtr<u32>, iid: u32, outer: u32) -> i32;
    0x0081_805C, "DirectDrawCreateEx (ddraw delay-load thunk)" => direct_draw_create_ex: fn(guid: u32, out: GPtr<u32>, iid: u32, outer: u32) -> i32;
    0x0081_803B, "DirectSoundCreate (dsound delay-load thunk)" => direct_sound_create: fn(guid: u32, out: GPtr<u32>, outer: u32) -> i32;
    0x0080_7C2B, "Direct3DCreate9 (d3d9 delay-load thunk)" => direct3d_create9: fn(sdk: u32) -> u32;
    trap(2, 1), "IDirect3D9::AddRef" => add_ref: fn(this: u32) -> u32;
    trap(2, 2), "IDirect3D9::Release" => release: fn(this: u32) -> u32;
    trap(2, 4), "IDirect3D9::GetAdapterCount" => d3d_get_adapter_count: fn(this: u32) -> u32;
    trap(2, 6), "IDirect3D9::GetAdapterModeCount" => d3d_get_adapter_mode_count: fn(this: u32, adapter: u32, format: u32) -> u32;
    trap(2, 7), "IDirect3D9::EnumAdapterModes" => d3d_enum_adapter_modes: fn(this: u32, adapter: u32, format: u32, mode: u32, out: GPtr<u32>) -> i32;
    trap(2, 8), "IDirect3D9::GetAdapterDisplayMode" => d3d_get_adapter_display_mode: fn(this: u32, adapter: u32, out: GPtr<u32>) -> i32;
    trap(2, 10), "IDirect3D9::CheckDeviceFormat" => d3d_check_device_format: fn(this: u32, adapter: u32, ty: u32, adapter_format: u32, usage: u32, rtype: u32, format: u32) -> i32;
    trap(2, 11), "IDirect3D9::CheckDeviceMultiSampleType" => d3d_check_multisample: fn(this: u32, adapter: u32, ty: u32, format: u32, windowed: u32, ms: u32, quality: GPtr<u32>) -> i32;
    trap(2, 14), "IDirect3D9::GetDeviceCaps" => d3d_get_device_caps: fn(this: u32, adapter: u32, ty: u32, caps: GPtr<u8>) -> i32;
    trap(2, 5), "IDirect3D9::GetAdapterIdentifier" => d3d_get_adapter_identifier: fn(this: u32, adapter: u32, flags: u32, out: GPtr<u8>) -> i32;
    trap(2, 9), "IDirect3D9::CheckDeviceType" => d3d_check_device_type: fn(this: u32, adapter: u32, ty: u32, adapter_format: u32, bb_format: u32, windowed: u32) -> i32;
    trap(2, 12), "IDirect3D9::CheckDepthStencilMatch" => d3d_check_depth_stencil_match: fn(this: u32, adapter: u32, ty: u32, adapter_format: u32, rt_format: u32, ds_format: u32) -> i32;
    trap(2, 16), "IDirect3D9::CreateDevice" => d3d_create_device: fn(this: u32, adapter: u32, ty: u32, hwnd: u32, flags: u32, pp: GPtr<u32>, out: GPtr<u32>) -> i32;
    trap(3, 1), "IDirect3DDevice9::AddRef" => add_ref: fn(this: u32) -> u32;
    trap(3, 2), "IDirect3DDevice9::Release" => release: fn(this: u32) -> u32;
    trap(3, 7), "IDirect3DDevice9::GetDeviceCaps" => dev_get_device_caps: fn(this: u32, caps: GPtr<u8>) -> i32;
}


fn direct3d_create9(_sdk: u32) -> u32 {
    with("Direct3DCreate9", |_| ());
    D3D9 as u32
}
fn d3d_get_adapter_count(_this: u32) -> u32 {
    1
}
fn d3d_get_adapter_mode_count(_this: u32, adapter: u32, format: u32) -> u32 {
    if adapter != 0 { 0 } else { modes_for(format).len() as u32 }
}
/// `EnumAdapterModes` → `D3DDISPLAYMODE {Width, Height, RefreshRate, Format}`.
fn d3d_enum_adapter_modes(_this: u32, adapter: u32, format: u32, mode: u32, out: GPtr<u32>) -> i32 {
    match modes_for(format).get(mode as usize) {
        Some(&(w, h, r)) if adapter == 0 => {
            for (i, v) in [w, h, r, format].iter().enumerate() {
                out.wrapping_index(i as i32).write(*v);
            }
            0
        }
        _ => 0x8876_086Cu32 as i32, // D3DERR_INVALIDCALL
    }
}
fn d3d_get_adapter_display_mode(_this: u32, _adapter: u32, out: GPtr<u32>) -> i32 {
    for (i, v) in DESKTOP_MODE.iter().enumerate() {
        out.wrapping_index(i as i32).write(*v);
    }
    0
}
/// No multisampling (`D3DERR_NOTAVAILABLE`): the seam renders single-sampled.
fn d3d_check_multisample(_this: u32, _a: u32, _t: u32, _f: u32, _w: u32, ms: u32, _q: GPtr<u32>) -> i32 {
    if ms == 0 { 0 } else { 0x8876_086Au32 as i32 }
}
/// `CheckDeviceFormat`: the formats the seam converts (sa_rw_wgpu: every uncompressed RW format,
/// DXT1–5, A8/L8/A8L8) and its depth formats; P8, V8U8 and the rest are `D3DERR_NOTAVAILABLE`.
fn d3d_check_device_format(_this: u32, _a: u32, _t: u32, _af: u32, usage: u32, _rtype: u32, format: u32) -> i32 {
    let dxt = (u32::from_le_bytes(*b"DXT1")..=u32::from_le_bytes(*b"DXT5")).contains(&format)
        && format & 0xFF == b'D' as u32;
    let ok = if usage & 2 != 0 {
        matches!(format, 71 | 75 | 77 | 80)
    } else {
        matches!(format, 21..=26 | 28 | 50 | 51) || dxt
    };
    if ok { 0 } else { 0x8876_086Au32 as i32 }
}
fn d3d_get_device_caps(_this: u32, _adapter: u32, _ty: u32, caps: GPtr<u8>) -> i32 {
    write_caps9(caps);
    0
}
/// The adapter description `GetAdapterIdentifier` reports (RW's sub-system name). Not measured
/// (platform value, docs/arch/platform.md §8).
pub const ADAPTER_DESCRIPTION: &str = "sa-engine wgpu";

/// `GetAdapterIdentifier(adapter, flags, D3DADAPTER_IDENTIFIER9*)`: Driver "sa-engine", Description
/// [`ADAPTER_DESCRIPTION`], everything else 0 (0x44C bytes).
fn d3d_get_adapter_identifier(_this: u32, adapter: u32, _flags: u32, out: GPtr<u8>) -> i32 {
    if adapter != 0 {
        return 0x8876_086Cu32 as i32;
    }
    for i in 0..0x44C {
        out.byte_add(i).write(0);
    }
    for (base, text) in [(0, "sa-engine"), (0x200, ADAPTER_DESCRIPTION)] {
        for (i, b) in text.bytes().enumerate() {
            out.byte_add((base + i) as isize).write(b);
        }
    }
    0
}
/// `CheckDeviceType(adapter, type, displayFormat, backBufferFormat, windowed)`: the display formats
/// the adapter lists (X8R8G8B8 with an X8R8G8B8 or A8R8G8B8 back buffer, R5G6B5 with R5G6B5) on
/// adapter 0; else `D3DERR_NOTAVAILABLE`. The seam's back buffer is RGBA8 whatever the format.
fn d3d_check_device_type(_this: u32, adapter: u32, _ty: u32, af: u32, bf: u32, _windowed: u32) -> i32 {
    let ok = adapter == 0 && matches!((af, bf), (0x16, 0x15 | 0x16) | (0x17, 0x17));
    if ok { 0 } else { 0x8876_086Au32 as i32 }
}
/// `CheckDepthStencilMatch(adapter, type, displayFormat, rtFormat, dsFormat)`: every depth format
/// `CheckDeviceFormat` accepts matches every render-target format (the seam pairs any colour target
/// with its Depth24PlusStencil8).
fn d3d_check_depth_stencil_match(_this: u32, adapter: u32, _ty: u32, _af: u32, _rf: u32, df: u32) -> i32 {
    if adapter == 0 && matches!(df, 71 | 75 | 77 | 80) { 0 } else { 0x8876_086Au32 as i32 }
}
/// `CreateDevice(adapter, type, hwnd, flags, D3DPRESENT_PARAMETERS*, out)`: the native device on a
/// headless `WgpuDevice` of the parameters' back-buffer size (0, a windowed device: the window's
/// client size; the parameters are not written back), the swap chain reporting the parameters'
/// back-buffer and auto depth-stencil formats. Behaviour flags are not modelled (the seam is
/// single-threaded fixed function; the x87 control word is the game's own, `sa_core::abi::game_fpu`).
/// No wgpu adapter: `D3DERR_NOTAVAILABLE`.
fn d3d_create_device(_this: u32, adapter: u32, _ty: u32, hwnd: u32, _flags: u32, pp: GPtr<u32>, out: GPtr<u32>) -> i32 {
    if adapter != 0 {
        return 0x8876_086Cu32 as i32;
    }
    let (mut w, mut h) = (pp.read(), pp.wrapping_index(1).read());
    if w == 0 || h == 0 {
        let mut rc = [0i32; 4];
        crate::bind::get_window_rect(hwnd, GPtr::from_raw(&mut rc));
        if w == 0 {
            w = rc[2].max(1) as u32;
        }
        if h == 0 {
            h = rc[3].max(1) as u32;
        }
    }
    let color = pp.wrapping_index(2).read();
    let depth = pp.wrapping_index(10).read();
    with("IDirect3D9::CreateDevice", |_| ());
    if !crate::d3d9::create_device(w, h, color, depth) {
        return 0x8876_086Au32 as i32;
    }
    out.write(DEVICE9 as u32);
    0
}
fn dev_get_device_caps(_this: u32, caps: GPtr<u8>) -> i32 {
    write_caps9(caps);
    0
}

fn add_ref(_this: u32) -> u32 {
    1
}
fn release(_this: u32) -> u32 {
    0
}
fn dd_get_available_vid_mem(_this: u32, _caps: GPtr<u32>, total: GPtr<u32>, free: GPtr<u32>) -> i32 {
    with("IDirectDraw7::GetAvailableVidMem", |_| ());
    if !total.is_null() {
        total.write(VIDEO_MEMORY);
    }
    if !free.is_null() {
        free.write(VIDEO_MEMORY);
    }
    0
}
/// `IDirectSound::GetCaps(DSCAPS*)`: `DS_OK` with the caller's `dwSize` kept and the other fields
/// zero (the game only tests the result).
fn ds_get_caps(_this: u32, caps: GPtr<u32>) -> i32 {
    with("IDirectSound::GetCaps", |_| ());
    for i in 1..24 {
        caps.wrapping_index(i).write(0);
    }
    0
}
fn direct_draw_create_ex(_guid: u32, out: GPtr<u32>, _iid: u32, _outer: u32) -> i32 {
    with("DirectDrawCreateEx", |_| ());
    out.write(DD7 as u32);
    0
}
fn direct_sound_create(_guid: u32, out: GPtr<u32>, _outer: u32) -> i32 {
    with("DirectSoundCreate", |_| ());
    out.write(DS as u32);
    0
}

/// DLLs `LoadLibraryA` "loads" (lower case), each with a made-up module handle.
const LIBRARIES: [&str; 4] = ["ddraw.dll", "d3d8.dll", "dpnhpast.dll", "d3d9.dll"];
const MODULE_BASE: u32 = 0x7000_0000;

/// `LoadLibraryA`: a handle for the DirectX DLLs `psInitialize` probes, `NULL` otherwise.
pub fn load_library_a(name: GPtr<u8>) -> u32 {
    with("LoadLibraryA", |_| ());
    let n = cstr(name).to_ascii_lowercase();
    LIBRARIES
        .iter()
        .position(|l| *l == n)
        .map_or(0, |i| MODULE_BASE + 0x10_0000 * i as u32)
}
/// `GetProcAddress`: `DirectDrawCreateEx` of ddraw.dll; `NULL` otherwise.
pub fn get_proc_address(module: u32, name: GPtr<u8>) -> u32 {
    with("GetProcAddress", |_| ());
    if module == MODULE_BASE && cstr(name) == "DirectDrawCreateEx" {
        TRAP_DD_CREATE_EX
    } else {
        0
    }
}
pub fn free_library(_module: u32) -> i32 {
    with("FreeLibrary", |_| ());
    1
}
/// `CoCreateInstance`: no COM server is available natively (DirectShow's filter graph for the
/// intro movies included): `REGDB_E_CLASSNOTREG`. Phase B plays no movies (platform.md section 9).
pub fn co_create_instance(_clsid: u32, _outer: u32, _ctx: u32, _iid: u32, out: GPtr<u32>) -> i32 {
    with("CoCreateInstance", |_| ());
    out.write(0);
    0x8004_0154u32 as i32
}
/// `CoInitialize(NULL)`: `S_OK`.
pub fn co_initialize(_reserved: u32) -> i32 {
    with("CoInitialize", |_| ());
    0
}
