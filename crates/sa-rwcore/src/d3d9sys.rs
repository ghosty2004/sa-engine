//! STD-4: the RW D3D9 driver's device system function `_rwD3D9System` (0x7F5F70) with every request
//! and its helpers (mode list, standard functions, present parameters, multisample probe, device
//! caches, device stop), the CPU feature probes, the RW x87 processor functions and the x87 matrix
//! multiply FINALIZESTART installs. Ported from the 1.0 US bytes; they replace sa-game's temporary
//! `rw_standin.rs` stand-ins (docs/arch/platform.md §9.4).
//!
//! Driver globals (all raw addresses, as d3d9cam.rs): window 0xC97C1C, `IDirect3D9*` 0xC97C20,
//! adapter 0xC97C24, device 0xC97C28, depth / back-buffer surfaces 0xC97C2C / 0xC97C30, stencil
//! present 0xC97C3C, started 0xC97C38, mode count / list 0xC97C40 / 0xC97C48 (0x14-byte entries
//! `{width, height, refresh, format, flags}`), current mode 0xC97C18, desktop mode 0xC97BC8 (4
//! dwords), engine mode 0xC9BEE4 (width, height, refresh, format, depth +0x10, flags +0x14), Z depth
//! 0xC9BEFC, total adapter modes 0xC9BEE0, `D3DCAPS9` 0xC9BF00, device type 0x8E2428, multisample
//! levels 0x8E242C..0x8E2438, `D3DPRESENT_PARAMETERS` 0xC9C040.

use sa_core::abi::{HookReg, NativeReg, game_fpu};
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{f32_of, g, p8, rd, rw_free, rw_malloc, rw_memory_alloc, rw_memory_free, rw_realloc, wr, x};

const MODE_CUR: usize = 0xC97C18;
const DESKTOP: usize = 0xC97BC8;
const WINDOW: usize = 0xC97C1C;
const D3D9: usize = 0xC97C20;
const ADAPTER: usize = 0xC97C24;
const DEVICE: usize = 0xC97C28;
const DEPTH_SURFACE: usize = 0xC97C2C;
const RENDER_SURFACE: usize = 0xC97C30;
const STARTED: usize = 0xC97C38;
const STENCIL: usize = 0xC97C3C;
const NUM_MODES: usize = 0xC97C40;
const MODES: usize = 0xC97C48;
const MULTITHREAD: usize = 0xC97C4C;
const SOFTWARE_VP: usize = 0xC97C50;
const TOTAL_MODES: usize = 0xC9BEE0;
/// Engine mode: width, height, refresh, format, depth (+0x10), flags (+0x14).
const MODE: usize = 0xC9BEE4;
const MODE_DEPTH: usize = 0xC9BEF4;
const MODE_FLAGS: usize = 0xC9BEF8;
const Z_DEPTH: usize = 0xC9BEFC;
const CAPS: usize = 0xC9BF00;
const DEV_TYPE: usize = 0x8E2428;
/// Multisample levels: maskable (+0), +4, non-maskable quality + 1 (+8), +0xC.
const MS: usize = 0x8E242C;
const MAX_REFRESH: usize = 0x8E243C;
/// `D3DPRESENT_PARAMETERS`.
const PP: usize = 0xC9C040;
/// The three adapter formats the mode list enumerates (R5G6B5, X8R8G8B8, A2R10G10B10).
const MODE_FORMATS: usize = 0x884788;
/// Identity matrix the transform cache's world entry starts as.
const IDENTITY: usize = 0x8847A0;
/// CPU feature flags MMX, SSE, SSE2, 3DNow!.
const CPU_MMX: usize = 0xC980A0;
const CPU_SSE: usize = 0xC980A4;
const CPU_SSE2: usize = 0xC980A8;
const CPU_3DNOW: usize = 0xC980AC;
/// `_rwProcessorInitialize`'s control word cell and the saved control word.
const FPU_CW: usize = 0xCAF000;
const FPU_CW_SAVED: usize = 0xCAF004;
const MODE_HINT: u32 = 0x104_0411;

orig! {
    /// `Direct3DCreate9(sdk)` (d3d9 delay-load thunk 0x807C2B)
    fn direct3d_create9(sdk: u32) -> u32 = stdcall 0x807C2B => unported;
    /// `IUnknown::Release()` (+8)
    fn release(o: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;
    /// `IDirect3D9::GetAdapterCount()` (+0x10)
    fn d3d_get_adapter_count(d: GPtr<u8>) -> u32 = stdcall vslot 0x10 => unported;
    /// `IDirect3D9::GetAdapterIdentifier(adapter, flags, ident)` (+0x14)
    fn d3d_get_adapter_identifier(d: GPtr<u8>, adapter: u32, flags: u32, out: GPtr<u8>) -> i32 = stdcall vslot 0x14 => unported;
    /// `IDirect3D9::GetAdapterModeCount(adapter, format)` (+0x18)
    fn d3d_get_adapter_mode_count(d: GPtr<u8>, adapter: u32, format: u32) -> u32 = stdcall vslot 0x18 => unported;
    /// `IDirect3D9::EnumAdapterModes(adapter, format, mode, out)` (+0x1C)
    fn d3d_enum_adapter_modes(d: GPtr<u8>, adapter: u32, format: u32, mode: u32, out: u32) -> i32 = stdcall vslot 0x1C => unported;
    /// `IDirect3D9::GetAdapterDisplayMode(adapter, out)` (+0x20)
    fn d3d_get_adapter_display_mode(d: GPtr<u8>, adapter: u32, out: u32) -> i32 = stdcall vslot 0x20 => unported;
    /// `IDirect3D9::CheckDeviceType(adapter, type, adapterFormat, backBufferFormat, windowed)` (+0x24)
    fn d3d_check_device_type(d: GPtr<u8>, adapter: u32, ty: u32, af: u32, bf: u32, windowed: u32) -> i32 = stdcall vslot 0x24 => unported;
    /// `IDirect3D9::CheckDeviceFormat(adapter, type, adapterFormat, usage, rtype, format)` (+0x28)
    fn d3d_check_device_format(d: GPtr<u8>, adapter: u32, ty: u32, af: u32, usage: u32, rtype: u32, f: u32) -> i32 = stdcall vslot 0x28 => unported;
    /// `IDirect3D9::CheckDeviceMultiSampleType(adapter, type, format, windowed, ms, quality)` (+0x2C)
    fn d3d_check_multisample(d: GPtr<u8>, adapter: u32, ty: u32, f: u32, windowed: u32, ms: u32, q: u32) -> i32 = stdcall vslot 0x2C => unported;
    /// `IDirect3D9::CheckDepthStencilMatch(adapter, type, adapterFormat, rtFormat, dsFormat)` (+0x30)
    fn d3d_check_depth_stencil_match(d: GPtr<u8>, adapter: u32, ty: u32, af: u32, rf: u32, df: u32) -> i32 = stdcall vslot 0x30 => unported;
    /// `IDirect3D9::GetDeviceCaps(adapter, type, caps)` (+0x38)
    fn d3d_get_device_caps(d: GPtr<u8>, adapter: u32, ty: u32, caps: u32) -> i32 = stdcall vslot 0x38 => unported;
    /// `IDirect3D9::CreateDevice(adapter, type, hwnd, flags, params, out)` (+0x40)
    fn d3d_create_device(d: GPtr<u8>, adapter: u32, ty: u32, hwnd: u32, flags: u32, pp: u32, out: u32) -> i32 = stdcall vslot 0x40 => unported;

    /// `IDirect3DDevice9::GetAvailableTextureMem()` (+0x10)
    fn dev_get_available_texture_mem(d: GPtr<u8>) -> u32 = stdcall vslot 0x10 => unported;
    /// `IDirect3DDevice9::GetRenderTarget(index, out)` (+0x98)
    fn dev_get_render_target(d: GPtr<u8>, index: u32, out: u32) -> i32 = stdcall vslot 0x98 => unported;
    /// `IDirect3DDevice9::GetDepthStencilSurface(out)` (+0xA0)
    fn dev_get_depth_stencil_surface(d: GPtr<u8>, out: u32) -> i32 = stdcall vslot 0xA0 => unported;
    /// `IDirect3DDevice9::SetVertexDeclaration(decl)` (+0x15C)
    fn dev_set_vertex_declaration(d: GPtr<u8>, decl: u32) -> i32 = stdcall vslot 0x15C => unported;
    /// `IDirect3DDevice9::SetVertexShader(shader)` (+0x170)
    fn dev_set_vertex_shader(d: GPtr<u8>, s: u32) -> i32 = stdcall vslot 0x170 => unported;
    /// `IDirect3DDevice9::SetStreamSource(stream, vb, offset, stride)` (+0x190)
    fn dev_set_stream_source(d: GPtr<u8>, stream: u32, vb: u32, offset: u32, stride: u32) -> i32 = stdcall vslot 0x190 => unported;
    /// `IDirect3DDevice9::SetIndices(ib)` (+0x1A0)
    fn dev_set_indices(d: GPtr<u8>, ib: u32) -> i32 = stdcall vslot 0x1A0 => unported;
    /// `IDirect3DDevice9::SetPixelShader(shader)` (+0x1AC)
    fn dev_set_pixel_shader(d: GPtr<u8>, s: u32) -> i32 = stdcall vslot 0x1AC => unported;
    /// `IDirect3DDevice9::SetTexture(stage, tex)` (+0x104)
    fn dev_set_texture(d: GPtr<u8>, stage: u32, tex: u32) -> i32 = stdcall vslot 0x104 => unported;

    /// `G->stringFuncs.rwstrcpy(dst, src)` (`RwGlobals` +0xF8)
    fn rw_strcpy(dst: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = cdecl table 0xC97B24 0xF8 => unported;
    /// USER32 `ShowWindow(hwnd, cmd)` (IAT 0x858278)
    fn show_window(hwnd: u32, cmd: i32) -> i32 = stdcall cell 0x858278 => unported;
    /// USER32 `GetWindowRect(hwnd, rect)` (IAT 0x8582F0)
    fn get_window_rect(hwnd: u32, rect: GPtr<i32>) -> i32 = stdcall cell 0x8582F0 => unported;
    /// USER32 `GetClientRect(hwnd, rect)` (IAT 0x858304)
    fn get_client_rect(hwnd: u32, rect: GPtr<i32>) -> i32 = stdcall cell 0x858304 => unported;

    /// `_rwMatrixSetMultFn(fn)` (0x7F12C0)
    fn matrix_set_mult_fn(f: u32) -> u32 = cdecl 0x7F12C0 => crate::matrix::rw_matrix_set_mult_fn;
    /// `_rwVectorSetMultFn(point, points, vector, vectors)` (0x7ED5F0)
    fn vector_set_mult_fn(p: u32, ps: u32, v: u32, vs: u32) -> u32 = cdecl 0x7ED5F0 => unported;
    /// `_rwFreeListCreate(entrySize, entriesPerBlock, alignment, hint)` (0x801980)
    fn freelist_create(size: u32, per_block: u32, align: u32, hint: u32) -> GPtr<u8> = cdecl 0x801980 => unported;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    fn freelist_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => unported;
    /// `RwFreeListPurge(fl)` (0x801E00)
    fn freelist_purge(fl: GPtr<u8>) -> i32 = cdecl 0x801E00 => unported;
    /// `_rxD3D9VideoMemoryRasterListCreate()` (0x4CC150)
    fn video_memory_raster_list_create() -> u32 = cdecl 0x4CC150 => unported;
    /// `_rwD3D9RasterClose()` (0x4CC170)
    fn raster_close() -> u32 = cdecl 0x4CC170 => unported;
    /// `_rwD3D9Im2DRenderOpen()` (0x7FB480)
    fn im2d_render_open() -> u32 = cdecl 0x7FB480 => unported;
    /// `_rwD3D9Im2DRenderClose()` (0x7FB5F0)
    fn im2d_render_close() -> u32 = cdecl 0x7FB5F0 => unported;
    /// `_rwD3D9RenderStateOpen()` (0x7FCAC0)
    fn render_state_open() -> u32 = cdecl 0x7FCAC0 => unported;
    /// `_rwD3D9VertexBufferManagerOpen()` (0x7F5CB0)
    fn vertex_buffer_manager_open() -> u32 = cdecl 0x7F5CB0 => unported;
    /// `_rwD3D9VertexBufferManagerClose()` (0x7F5D20)
    fn vertex_buffer_manager_close() -> u32 = cdecl 0x7F5D20 => unported;

    /// `FUN_007f7540()`: the mode list
    fn build_mode_list_call() = cdecl 0x7F7540 => build_mode_list;
    /// `_rwD3D9SetStandardFunctions(table, -, count)` (0x7F72E0)
    fn set_standard_functions_call(out: GPtr<u32>, inout: u32, n: i32) -> u32 = cdecl 0x7F72E0 => set_standard_functions;
    /// `D3D9SetPresentParameters(mode, flags, adapterFormat)` (0x7F6CB0)
    fn set_present_parameters_call(mode: u32, flags: u32, format: u32) = cdecl 0x7F6CB0 => set_present_parameters;
    /// `FUN_007f6bf0()`: the multisample probe
    fn multisample_probe_call() = cdecl 0x7F6BF0 => multisample_probe;
    /// `FUN_007f6b00()`: the declaration / FVF / shader caches invalidated
    fn reset_caches_call() = cdecl 0x7F6B00 => reset_caches;
    /// `FUN_007f6b40()`: the transform cache reset
    fn reset_transform_cache_call() = cdecl 0x7F6B40 => reset_transform_cache;
    /// `FUN_007f7100()`: the device stop
    fn device_stop_call() -> u32 = cdecl 0x7F7100 => device_stop;
    /// `_rwIntelMMXsupported()` (0x80DBE0)
    fn mmx_call() -> u32 = cdecl 0x80DBE0 => intel_mmx_supported;
    /// `_rwIntelSSEsupported()` (0x80DC40)
    fn sse_call() -> u32 = cdecl 0x80DC40 => intel_sse_supported;
    /// `_rwIntelSSE2supported()` (0x80DCA0)
    fn sse2_call() -> u32 = cdecl 0x80DCA0 => intel_sse2_supported;
    /// `_rwAMD3DNowSupported()` (0x80DD00)
    fn amd_3dnow_call() -> u32 = cdecl 0x80DD00 => amd_3dnow_supported;
    /// `_rwProcessorInitialize()` (0x857404)
    fn processor_initialize_call() = cdecl 0x857404 => processor_initialize;
    /// `_rwProcessorRelease()` (0x85742B)
    fn processor_release_call() = cdecl 0x85742B => processor_release;
}

fn u(a: usize) -> u32 {
    g::<u32>(a).read()
}
fn set(a: usize, v: u32) {
    g::<u32>(a).write(v)
}
fn d3d9() -> GPtr<u8> {
    p8(u(D3D9))
}
fn device() -> GPtr<u8> {
    p8(u(DEVICE))
}

/// Colour depth of a `D3DFORMAT` as the driver's switches map it (0x7F6A84 etc.): X8R8G8B8 /
/// A8R8G8B8 / A2R10G10B10 32, R5G6B5 / X1R5G5B5 / A1R5G5B5 16, else 0.
pub fn format_depth(f: u32) -> u32 {
    match f {
        0x15 | 0x16 | 0x23 => 32,
        0x17..=0x19 => 16,
        _ => 0,
    }
}

/// `D3DFORMAT` → RW raster format (GETMODEINFO's switch 0x7F6AC8): A8R8G8B8 / A2R10G10B10 8888,
/// X8R8G8B8 888, R5G6B5 565, X1R5G5B5 555, A1R5G5B5 1555, else 0.
fn format_raster(f: u32) -> u32 {
    match f {
        0x15 | 0x23 => 0x500,
        0x16 => 0x600,
        0x17 => 0x200,
        0x18 => 0xA00,
        0x19 => 0x100,
        _ => 0,
    }
}

/// The multisample probe (0x7F6BF0's body, inlined by USEMODE and SETSUBSYSTEM): the four levels
/// reset to 1; maskable levels 2..16 of the mode format (0xC9BEF0) with `windowed = !(flags & 1)`
/// (0xC9BEF8), each success storing its level; then the non-maskable type (1) with a quality out:
/// on success quality + 1 goes to 0x8E2434 and raises 0x8E242C when larger.
fn ms_probe() {
    let windowed = !u(MODE_FLAGS) & 1;
    for a in [MS, MS + 4, MS + 8, MS + 0xC] {
        set(a, 1);
    }
    for level in 2..=16u32 {
        if d3d_check_multisample(d3d9(), u(ADAPTER), u(DEV_TYPE), u(MODE + 0xC), windowed, level, 0) >= 0 {
            set(MS, level);
        }
    }
    // the quality local is the level counter's slot: 17 after the loop
    let mut q: u32 = 17;
    if d3d_check_multisample(d3d9(), u(ADAPTER), u(DEV_TYPE), u(MODE + 0xC), windowed, 1, GPtr::from_raw(&mut q).addr() as u32) >= 0 {
        let n = q.wrapping_add(1);
        set(MS + 8, n);
        if n > u(MS) {
            set(MS, n);
        }
    }
}

/// `FUN_007f6bf0()` (0x7F6BF0): the multisample probe ([`ms_probe`]).
#[hook(addr = 0x7F6BF0, conv = cdecl, name = "FUN_007f6bf0", expect = "51 56 8B 35 F8 BE C9 00",
       route = "L2", hazards = "none", recs = "boot")]
pub fn multisample_probe() {
    ms_probe();
}

/// `FUN_007f7540()` (0x7F7540): the mode list. `[0xC97C48]` is (re)allocated for `total + 1`
/// entries (`rwrealloc`/`rwmalloc`, hint 0x1040411); entry 0 is the adapter's display mode, kept
/// (flags 0, count 1) when its format is one of 0x15..0x19, 0x23, else the count is 0. Then for each
/// format of 0x884788 every adapter mode is enumerated into the next free entry; one equal in
/// width, height and format to an entry 1.. only raises that entry's refresh rate (unsigned max),
/// otherwise it is kept with flags 1 (exclusive). Finally the list shrinks to the count when smaller
/// than `total + 1`.
#[hook(addr = 0x7F7540, conv = cdecl, name = "FUN_007f7540", expect = "8B 0D 48 7C C9 00 83 EC 0C",
       route = "L2", hazards = "heap-writes", recs = "boot")]
pub fn build_mode_list() {
    let total = u(TOTAL_MODES);
    let size = total.wrapping_mul(5).wrapping_add(5).wrapping_mul(4);
    let list = if u(MODES) != 0 {
        rw_realloc(p8(u(MODES)), size, MODE_HINT)
    } else {
        rw_malloc(size, MODE_HINT)
    };
    set(MODES, list.addr() as u32);
    d3d_get_adapter_display_mode(d3d9(), u(ADAPTER), list.addr() as u32);
    let e0 = p8(u(MODES));
    if matches!(rd::<u32>(e0, 0xC), 0x15..=0x19 | 0x23) {
        wr::<u32>(e0, 0x10, 0);
        set(NUM_MODES, 1);
    } else {
        set(NUM_MODES, 0);
    }
    for fi in 0..3 {
        let format = u(MODE_FORMATS + 4 * fi);
        let count = d3d_get_adapter_mode_count(d3d9(), u(ADAPTER), format) as i32;
        let mut i: i32 = 0;
        while i < count {
            let n = u(NUM_MODES);
            let base = u(MODES);
            d3d_enum_adapter_modes(d3d9(), u(ADAPTER), format, i as u32, base.wrapping_add(n.wrapping_mul(0x14)));
            let n = u(NUM_MODES) as i32;
            let base = p8(u(MODES));
            let new = base.byte_add(0x14 * n as isize);
            let mut found = None;
            if n > 1 {
                for k in 1..n {
                    let e = base.byte_add(0x14 * k as isize);
                    if rd::<u32>(e, 0) == rd::<u32>(new, 0)
                        && rd::<u32>(e, 4) == rd::<u32>(new, 4)
                        && rd::<u32>(e, 0xC) == rd::<u32>(new, 0xC)
                    {
                        found = Some(e);
                        break;
                    }
                }
            }
            match found {
                None => {
                    wr::<u32>(new, 0x10, 1);
                    set(NUM_MODES, n as u32 + 1);
                }
                Some(e) => {
                    let r = rd::<u32>(new, 8);
                    if rd::<u32>(e, 8) < r {
                        wr::<u32>(e, 8, r);
                    }
                }
            }
            i += 1;
        }
    }
    let n = u(NUM_MODES) as i32;
    if n < u(TOTAL_MODES).wrapping_add(1) as i32 {
        let l = rw_realloc(p8(u(MODES)), (n as u32).wrapping_mul(0x14), MODE_HINT);
        set(MODES, l.addr() as u32);
    }
}

/// The driver's standard functions `(rwStandardFunc index, function)` (0x7F72E0's table, in its
/// order).
const STANDARDS: [(u32, u32); 27] = [
    (1, 0x7F8F20), (0xA, 0x7F98D0), (0x15, 0x7F7730), (0x14, 0x7F99B0), (2, 0x7FEE20),
    (3, 0x7FF070), (7, 0x8001E0), (6, 0x7FF270), (5, 0x4CBB00), (4, 0x4CCE60), (9, 0x7FFF00),
    (8, 0x4CBD40), (0xF, 0x4C9F90), (0x10, 0x4CA290), (0x17, 0x4CA4E0), (0x18, 0x4CA540),
    (0xE, 0x4CB4E0), (0xD, 0x4CB4C0), (0x11, 0x4CAE40), (0x12, 0x4CAE80), (0x13, 0x4CAE60),
    (0xB, 0x4CB520), (0xC, 0x4CBD50), (0x19, 0x4CD360), (0x1B, 0x4CD4D0), (0x1A, 0x4CD820),
    (0x1C, 0x4CBCB0),
];
/// `_rwD3D9StandardFuncNotImplemented` (0x7F7530).
const NOT_IMPLEMENTED: u32 = 0x7F7530;

/// `_rwD3D9SetStandardFunctions(table, -, count)` (0x7F72E0): `count > 0` fills the table with
/// 0x7F7530; `count > 1` then stores the 27 driver functions, last table entry first. Returns 1.
#[hook(addr = 0x7F72E0, conv = cdecl, name = "_rwD3D9SetStandardFunctions", expect = "81 EC DC 00 00 00 53",
       route = "L2", hazards = "none", recs = "boot")]
pub fn set_standard_functions(out: GPtr<u32>, _inout: u32, n: i32) -> u32 {
    for i in 0..n.max(0) {
        out.wrapping_index(i).write(NOT_IMPLEMENTED);
    }
    for &(idx, f) in STANDARDS.iter().rev() {
        if n > 1 {
            out.wrapping_index(idx as i32).write(f);
        }
    }
    1
}

/// `_rwD3D9StandardFuncNotImplemented` (0x7F7530): returns 0. Not hookable (3 bytes, padding
/// follows).
#[sa_core::native(addr = 0x7F7530, conv = cdecl, name = "_rwD3D9StandardFuncNotImplemented")]
pub fn standard_not_implemented(_out: u32, _inout: u32, _n: i32) -> u32 {
    0
}

/// `D3D9SetPresentParameters(mode, flags, adapterFormat)` (0x7F6CB0). Exclusive modes (`flags & 1`):
/// windowed 0, one back buffer, the mode's refresh capped at 0x8E243C, immediate presentation,
/// the mode's size, FLIP, the mode's format. Windowed: windowed 1, one back buffer, refresh 0,
/// immediate, size 0 unless the window rectangle's width / height is 0 (then 1), COPY, the adapter
/// format. The back-buffer format is A8R8G8B8 when the format is X8R8G8B8 and `CheckDeviceType`
/// accepts that, else the format. Auto depth-stencil on: a 32-bit back buffer tries D24S8, D24X4S4
/// (stencil), D32, D24X8 (no stencil) with `CheckDeviceFormat(DEPTHSTENCIL, SURFACE)` and
/// `CheckDepthStencilMatch`, falling back to the 16-bit list D15S1 (stencil), D16 (no stencil); the
/// Z depth (0xC9BEFC) and the stencil flag (0xC97C3C) follow. Multisampling from the probe's levels
/// (DISCARD swap effect when used).
#[hook(addr = 0x7F6CB0, conv = cdecl, name = "WindowedModeHooks::RenderWare::D3D9SetPresentParameters", expect = "8A 44 24 08 83 EC 10",
       route = "L2", hazards = "none", recs = "boot")]
pub fn set_present_parameters(mode: u32, flags: u32, format: u32) {
    let m = p8(mode);
    let fmt;
    if flags & 1 != 0 {
        fmt = rd::<u32>(m, 0xC);
        set(PP + 0x20, 0);
        set(PP + 0xC, 1);
        let r = rd::<u32>(m, 8);
        let max = u(MAX_REFRESH);
        set(PP + 0x30, r);
        if r >= max {
            set(PP + 0x30, max);
        }
        set(PP + 0x34, 0x8000_0000);
        set(PP, rd::<u32>(m, 0));
        set(PP + 4, rd::<u32>(m, 4));
        set(PP + 0x18, 2);
    } else {
        let mut rc = [0i32; 4];
        set(PP + 0x20, 1);
        set(PP + 0xC, 1);
        set(PP + 0x30, 0);
        set(PP + 0x34, 0x8000_0000);
        get_window_rect(u(WINDOW), GPtr::from_raw(rc.as_mut_ptr()));
        set(PP + 0x18, 3);
        set(PP, (rc[2].wrapping_sub(rc[0]) == 0) as u32);
        set(PP + 4, (rc[3].wrapping_sub(rc[1]) == 0) as u32);
        fmt = format;
    }
    let bb = if fmt == 0x16 {
        let hr = d3d_check_device_type(d3d9(), u(ADAPTER), u(DEV_TYPE), fmt, 0x15, u(PP + 0x20));
        if hr < 0 { 0x16 } else { 0x15 }
    } else {
        fmt
    };
    set(PP + 8, bb);
    set(PP + 0x24, 1);
    set(Z_DEPTH, format_depth(bb));
    let fits = |ds: u32| {
        d3d_check_device_format(d3d9(), u(ADAPTER), u(DEV_TYPE), fmt, 2, 1, ds) == 0
            && d3d_check_depth_stencil_match(d3d9(), u(ADAPTER), u(DEV_TYPE), fmt, u(PP + 8), ds) == 0
    };
    if u(Z_DEPTH) == 32 {
        if fits(0x4B) {
            set(PP + 0x28, 0x4B);
            set(STENCIL, 1);
        } else if fits(0x4F) {
            set(PP + 0x28, 0x4F);
            set(STENCIL, 1);
        } else if fits(0x47) {
            set(PP + 0x28, 0x47);
            set(STENCIL, 0);
        } else if fits(0x4D) {
            set(PP + 0x28, 0x4D);
            set(STENCIL, 0);
        } else {
            set(Z_DEPTH, 16);
        }
    }
    if u(Z_DEPTH) == 16 {
        if fits(0x49) {
            set(PP + 0x28, 0x49);
            set(STENCIL, 1);
        } else {
            set(PP + 0x28, 0x50);
            set(STENCIL, 0);
        }
    }
    set(PP + 0x10, 0);
    set(PP + 0x14, 0);
    if u(MS + 4) <= 1 && u(MS + 0xC) <= 1 {
        return;
    }
    let mut q: u32 = 0;
    let qp = GPtr::from_raw(&mut q).addr() as u32;
    let hr = d3d_check_multisample(d3d9(), u(ADAPTER), u(DEV_TYPE), u(PP + 8), u(PP + 0x20), u(MS + 4), qp);
    if hr >= 0 && q != 0 {
        set(PP + 0x10, u(MS + 4));
        set(PP + 0x14, q - 1);
        if u(PP + 0x10) != 0 {
            set(PP + 0x18, 1);
        }
        return;
    }
    q = 0;
    let hr = d3d_check_multisample(d3d9(), u(ADAPTER), u(DEV_TYPE), u(PP + 8), u(PP + 0x20), 1, qp);
    if hr < 0 || q == 0 {
        if u(PP + 0x10) != 0 {
            set(PP + 0x18, 1);
        }
        return;
    }
    let cap = q - 1;
    let mut lv = u(MS + 0xC).wrapping_sub(2);
    if lv > cap {
        lv = cap;
    }
    set(PP + 0x10, 1);
    set(PP + 0x18, 1);
    set(PP + 0x14, lv);
}

/// `FUN_007f6b00()` (0x7F6B00): the FVF / declaration / shader / index caches (0x8E2440..0x8E2450)
/// and the four stream caches (0xC97BD8, 0x10 apart: buffer -1, offset 0, stride 0) invalidated.
#[hook(addr = 0x7F6B00, conv = cdecl, name = "FUN_007f6b00", expect = "83 C9 FF 33 D2",
       route = "L2", hazards = "none", recs = "boot")]
pub fn reset_caches() {
    for a in [0x8E2440, 0x8E2444, 0x8E2448, 0x8E244C, 0x8E2450] {
        set(a, 0xFFFF_FFFF);
    }
    for i in 0..4 {
        set(0xC97BD8 + 0x10 * i, 0xFFFF_FFFF);
        set(0xC97BDC + 0x10 * i, 0);
        set(0xC97BE0 + 0x10 * i, 0);
    }
}

/// `FUN_007f6b40()` (0x7F6B40): the transform cache. Clears 0xC97C68; with a freelist (0xC98080)
/// every cached matrix (0xC97C70, 0x104 entries) is freed and the list purged, else the freelist is
/// created (0x40-byte entries, 15 per block, alignment 16) and the table zeroed; then the world
/// matrix (0xC98070) is a new entry holding the identity of 0x8847A0.
#[hook(addr = 0x7F6B40, conv = cdecl, name = "FUN_007f6b40", expect = "8B 0D 80 80 C9 00 56 57",
       route = "L2", hazards = "heap-writes", recs = "boot")]
pub fn reset_transform_cache() {
    set(0xC97C68, 0);
    if u(0xC98080) != 0 {
        for i in 0..0x104 {
            let c = u(0xC97C70 + 4 * i);
            if c != 0 {
                rw_memory_free(p8(u(0xC98080)), p8(c));
                set(0xC97C70 + 4 * i, 0);
            }
        }
        freelist_purge(p8(u(0xC98080)));
    } else {
        let fl = freelist_create(0x40, 0xF, 0x10, 0x30411);
        set(0xC98080, fl.addr() as u32);
        for i in 0..0x104 {
            set(0xC97C70 + 4 * i, 0);
        }
    }
    let world = rw_memory_alloc(p8(u(0xC98080)), 0x30411);
    set(0xC98070, world.addr() as u32);
    for i in 0..16 {
        wr::<u32>(world, 4 * i, u(IDENTITY + 4 * i as usize));
    }
}

/// `FUN_007f7100()` (0x7F7100): the device stop (STOP request). Clears "started", frees the mode
/// list, restores the x87 control word (`_rwProcessorRelease`), clears FTZ (MXCSR bit 15) when SSE
/// is present, frees the cached transforms and destroys their freelist, frees the light cache,
/// unbinds textures 0..7, indices, streams 0..3, pixel shader, declaration and vertex shader,
/// releases every cached vertex declaration (0xC97C60, 0xC97C58 entries of 0xC bytes: `Release`
/// until it returns 0) and frees its element copy, frees the table, closes the raster, vertex-buffer
/// and Im2D sub-systems and releases the device. Returns 1.
#[hook(addr = 0x7F7100, conv = cdecl, name = "FUN_007f7100", expect = "51 A1 48 7C C9 00 53",
       unverified = "std4-d3d9-1", route = "L3-only", hazards = "heap-writes", recs = "boot",
       note = "runs at shutdown only; releases the device")]
pub fn device_stop() -> u32 {
    set(STARTED, 0);
    if u(MODES) != 0 {
        rw_free(p8(u(MODES)));
        set(MODES, 0);
        set(NUM_MODES, 0);
    }
    processor_release_call();
    if u(CPU_SSE) != 0 {
        game_fpu::ldmxcsr(game_fpu::stmxcsr() & 0xFFFF_7FFF);
    }
    for i in 0..0x104 {
        let c = u(0xC97C70 + 4 * i);
        if c != 0 {
            rw_memory_free(p8(u(0xC98080)), p8(c));
            set(0xC97C70 + 4 * i, 0);
        }
    }
    freelist_destroy(p8(u(0xC98080)));
    set(0xC98080, 0);
    set(0xC98084, 0);
    if u(0xC98088) != 0 {
        rw_free(p8(u(0xC98088)));
        set(0xC98088, 0);
    }
    for s in 0..8 {
        dev_set_texture(device(), s, 0);
    }
    dev_set_indices(device(), 0);
    for s in 0..4 {
        dev_set_stream_source(device(), s, 0, 0, 0);
    }
    dev_set_pixel_shader(device(), 0);
    dev_set_vertex_declaration(device(), 0);
    dev_set_vertex_shader(device(), 0);
    let mut i: u32 = 0;
    while i < u(0xC97C58) {
        let e = 0xC * i as usize;
        if u(u(0xC97C60) as usize + e) != 0 {
            while release(p8(u(u(0xC97C60) as usize + e))) != 0 {}
        }
        let elems = u(u(0xC97C60) as usize + e + 8);
        if elems != 0 {
            rw_free(p8(elems));
        }
        i += 1;
    }
    if u(0xC97C60) != 0 {
        rw_free(p8(u(0xC97C60)));
        set(0xC97C60, 0);
    }
    set(0xC97C58, 0);
    set(0xC97C5C, 0);
    raster_close();
    vertex_buffer_manager_close();
    im2d_render_close();
    release(device());
    set(DEVICE, 0);
    1
}

orig! {
    /// `IDirect3DDevice9::SetRenderTarget(index, surface)` (+0x94)
    fn dev_set_render_target(d: GPtr<u8>, index: u32, s: u32) -> i32 = stdcall vslot 0x94 => unported;
    /// `IDirect3DDevice9::SetDepthStencilSurface(surface)` (+0x9C)
    fn dev_set_depth_stencil_surface(d: GPtr<u8>, s: u32) -> i32 = stdcall vslot 0x9C => unported;
    /// `_rwD3D9Im3DRenderClose()` (0x80DFB0)
    fn im3d_render_close() -> u32 = cdecl 0x80DFB0 => unported;
    /// `_rwD3D9DynamicVertexBufferRelease()` (0x7F5840)
    fn dynamic_vertex_buffer_release() = cdecl 0x7F5840 => unported;
    /// `_rxD3D9VideoMemoryRasterListRelease()` (0x4CB640)
    fn video_memory_raster_list_release() = cdecl 0x4CB640 => unported;
    /// `RwFreeListPurgeAllFreeLists()` (0x801F90)
    fn freelist_purge_all() -> i32 = cdecl 0x801F90 => unported;
}

/// `D3D9DeviceReleaseVideoMemory()` (0x7F7F70): before a device reset. The FVF / declaration /
/// shader / stream caches invalidated and the transform cache reset (0x7F6B00's and 0x7F6B40's
/// bodies, inlined), textures 0..7, indices, streams 0..3, pixel shader, declaration and vertex
/// shader unbound, render target 0 back to the back buffer and targets 1..3 cleared (each through
/// the render-target cache 0xC98090..), the depth surface back to the device's, then Im2D and Im3D
/// closed, the dynamic vertex buffers and the video-memory rasters released and every freelist
/// purged. Returns 1.
#[hook(addr = 0x7F7F70, conv = cdecl, name = "D3D9DeviceReleaseVideoMemory", expect = "53 83 C9 FF 33 DB 56",
       route = "L2", hazards = "heap-writes", recs = "boot")]
pub fn release_video_memory() -> u32 {
    reset_caches();
    reset_transform_cache();
    for s in 0..8 {
        dev_set_texture(device(), s, 0);
    }
    dev_set_indices(device(), 0);
    for s in 0..4 {
        dev_set_stream_source(device(), s, 0, 0, 0);
    }
    dev_set_pixel_shader(device(), 0);
    dev_set_vertex_declaration(device(), 0);
    dev_set_vertex_shader(device(), 0);
    let rt = u(RENDER_SURFACE);
    if u(0xC98090) != rt {
        set(0xC98090, rt);
        dev_set_render_target(device(), 0, rt);
    }
    for i in 1..4u32 {
        let a = 0xC98090 + 4 * i as usize;
        if u(a) != 0 {
            set(a, 0);
            dev_set_render_target(device(), i, 0);
        }
    }
    let ds = u(DEPTH_SURFACE);
    if u(0xC9808C) != ds {
        set(0xC9808C, ds);
        dev_set_depth_stencil_surface(device(), ds);
    }
    im2d_render_close();
    im3d_render_close();
    dynamic_vertex_buffer_release();
    video_memory_raster_list_release();
    freelist_purge_all();
    1
}

/// Sums the adapter's mode counts of the three list formats into 0xC9BEE0.
fn count_modes() {
    set(TOTAL_MODES, 0);
    for fi in 0..3 {
        let n = d3d_get_adapter_mode_count(d3d9(), u(ADAPTER), u(MODE_FORMATS + 4 * fi));
        set(TOTAL_MODES, u(TOTAL_MODES).wrapping_add(n));
    }
}

/// `_rwD3D9System(request, out, inout, n)` (0x7F5F70), the D3D9 driver's device system function.
///
/// * 0 OPEN (`inout` = `{hwnd}`): `Direct3DCreate9(31)`, the sub-system name "D3D9", the first
///   adapter (from 0) whose `GetDeviceCaps(HAL)` succeeds (none: the object released, 0), the mode
///   total of the three list formats, the adapter's display mode as engine mode (depth from the
///   format, flags 0) and desktop mode, mode 0, the multisample probe, the CPU feature flags.
/// * 1 CLOSE: frees the mode list, releases the `IDirect3D9`.
/// * 2 START: present parameters from the engine mode (0x7F6CB0 with the desktop format),
///   `GetDeviceCaps`, behaviour flags (MULTITHREADED for 0xC97C4C; SOFTWARE for 0xC97C50, which
///   also clears the caps' HW T&L bit and 0xC9BFEC; else HARDWARE (+PUREDEVICE) when the caps
///   have hardware T&L, else SOFTWARE), `CreateDevice` (failure: not started, 0), the client size as
///   engine size, then the swap-chain surfaces (each released once), the render-target caches, the
///   caches 0x7F6B00 / 0x7F6B40, the light cache, the video-memory raster list, Im2D, render
///   states and the vertex-buffer manager; started.
/// * 3 STOP (0x7F7100), 4 REGISTER (the device block of 0x8E2498, 14 dwords, to `out`; `inout`
///   to 0xC9BCC4), 5 GETNUMMODES, 6 GETMODEINFO, 7 USEMODE (before START only), 8 FOCUS
///   (fullscreen: `ShowWindow(SW_RESTORE / SW_HIDE)`), 0xA GETMODE, 0xB STANDARDS (0x7F72E0),
///   0xC GETTEXMEMSIZE, 0xD GETNUMSUBSYSTEMS, 0xE GETSUBSYSTEMINFO (the adapter description),
///   0xF GETCURRENTSUBSYSTEM, 0x10 SETSUBSYSTEM, 0x11 FINALIZESTART (the RW processor set-up, FTZ
///   with SSE, the x87 matrix multiply, the SSE2 / SSE / x87 vector transforms), 0x13
///   GETMAXTEXTURESIZE, 0x16 (`*(u16*)out = 2`). Every other request returns 0.
#[hook(addr = 0x7F5F70, conv = cdecl, name = "D3D9System", expect = "8B 44 24 04 81 EC 70 04 00 00",
       unverified = "std4-d3d9-1", route = "L3-only", hazards = "heap-writes", recs = "boot",
       note = "device creation and window calls (OS effects); boot-time only")]
pub fn d3d9_system(request: u32, out: GPtr<u8>, inout: GPtr<u8>, n: i32) -> u32 {
    let o = |i: i32| out.cast::<u32>().wrapping_index(i);
    match request {
        0 => {
            set(WINDOW, inout.cast::<u32>().read());
            let d = direct3d_create9(0x1F);
            set(D3D9, d);
            if d == 0 {
                return 0;
            }
            rw_strcpy(p8(0xC9BCE0), p8(0x8E2510));
            let count = d3d_get_adapter_count(d3d9());
            set(ADAPTER, 0);
            set(DEV_TYPE, 1);
            let mut ok = false;
            if count > 0 {
                loop {
                    if d3d_get_device_caps(d3d9(), u(ADAPTER), u(DEV_TYPE), CAPS as u32) >= 0 {
                        ok = u(ADAPTER) < count;
                        break;
                    }
                    set(ADAPTER, u(ADAPTER).wrapping_add(1));
                    if u(ADAPTER) >= count {
                        break;
                    }
                }
            }
            if !ok {
                release(d3d9());
                set(D3D9, 0);
                return 0;
            }
            count_modes();
            d3d_get_adapter_display_mode(d3d9(), u(ADAPTER), MODE as u32);
            let f = u(MODE + 0xC);
            set(MODE_DEPTH, format_depth(f));
            set(MODE_FLAGS, 0);
            set(DESKTOP, u(MODE));
            set(DESKTOP + 4, u(MODE + 4));
            set(DESKTOP + 8, u(MODE + 8));
            set(DESKTOP + 0xC, f);
            set(MODE_CUR, 0);
            multisample_probe_call();
            set(CPU_MMX, mmx_call());
            set(CPU_SSE, sse_call());
            set(CPU_SSE2, sse2_call());
            set(CPU_3DNOW, amd_3dnow_call());
            1
        }
        1 => {
            if u(MODES) != 0 {
                rw_free(p8(u(MODES)));
                set(MODES, 0);
                set(NUM_MODES, 0);
            }
            if u(D3D9) != 0 {
                release(d3d9());
                set(D3D9, 0);
            }
            1
        }
        2 => {
            let mut dm = [0u32; 4];
            d3d_get_adapter_display_mode(d3d9(), u(ADAPTER), GPtr::from_raw(dm.as_mut_ptr()).addr() as u32);
            for i in 0..14 {
                set(PP + 4 * i, 0);
            }
            set_present_parameters_call(MODE as u32, u(MODE_FLAGS), dm[3]);
            set(PP + 0x1C, u(WINDOW));
            d3d_get_device_caps(d3d9(), u(ADAPTER), u(DEV_TYPE), CAPS as u32);
            set(PP + 0x2C, 0);
            let mut flags: u32 = if u(MULTITHREAD) != 0 { 4 } else { 0 };
            let dev_caps = u(CAPS + 0x1C);
            if u(SOFTWARE_VP) != 0 {
                flags |= 0x20;
                set(CAPS + 0x1C, dev_caps & 0xFFFE_FFFF);
                set(CAPS + 0xEC, 0);
            } else if dev_caps & 0x1_0000 != 0 {
                flags |= if dev_caps & 0x10_0000 != 0 { 0x50 } else { 0x40 };
            } else {
                flags |= 0x20;
            }
            let hr = d3d_create_device(d3d9(), u(ADAPTER), u(DEV_TYPE), u(WINDOW), flags, PP as u32, DEVICE as u32);
            if hr < 0 {
                set(STARTED, 0);
                return 0;
            }
            if u(WINDOW) != 0 {
                let mut rc = [0i32; 4];
                get_client_rect(u(WINDOW), GPtr::from_raw(rc.as_mut_ptr()));
                set(MODE, rc[2] as u32);
                set(MODE + 4, rc[3] as u32);
            }
            dev_get_render_target(device(), 0, RENDER_SURFACE as u32);
            release(p8(u(RENDER_SURFACE)));
            dev_get_depth_stencil_surface(device(), DEPTH_SURFACE as u32);
            release(p8(u(DEPTH_SURFACE)));
            set(0xC9808C, 0);
            for a in [0xC98090, 0xC98094, 0xC98098, 0xC9809C] {
                set(a, 0);
            }
            reset_caches_call();
            reset_transform_cache_call();
            let lights = u(0xC98088);
            set(0xC98084, 0);
            if lights != 0 {
                rw_free(p8(lights));
                set(0xC98088, 0);
            }
            video_memory_raster_list_create();
            im2d_render_open();
            render_state_open();
            vertex_buffer_manager_open();
            set(STARTED, 1);
            1
        }
        3 => device_stop_call(),
        4 => {
            for i in 0..14 {
                o(i).write(u(0x8E2498 + 4 * i as usize));
            }
            set(0xC9BCC4, inout.addr() as u32);
            1
        }
        5 => {
            if u(MODES) == 0 {
                build_mode_list_call();
            }
            o(0).write(u(NUM_MODES));
            1
        }
        6 => {
            if u(MODES) == 0 {
                build_mode_list_call();
            }
            if n < 0 || n >= u(NUM_MODES) as i32 {
                return 0;
            }
            let e = 0x14 * n as usize;
            o(0).write(u(u(MODES) as usize + e));
            o(1).write(u(u(MODES) as usize + e + 4));
            o(2).write(format_depth(u(u(MODES) as usize + e + 0xC)));
            o(3).write(u(u(MODES) as usize + e + 0x10));
            o(4).write(u(u(MODES) as usize + e + 8));
            o(5).write(format_raster(u(u(MODES) as usize + e + 0xC)));
            1
        }
        7 => {
            if u(STARTED) != 0 || n < 0 || n >= u(NUM_MODES) as i32 {
                return 0;
            }
            set(MODE_CUR, n as u32);
            let e = u(MODES) as usize + 0x14 * n as usize;
            for i in 0..4 {
                set(MODE + 4 * i, u(e + 4 * i));
            }
            set(MODE_FLAGS, u(e + 0x10));
            set(MODE_DEPTH, format_depth(u(e + 0xC)));
            ms_probe();
            1
        }
        8 => {
            if u(DEVICE) == 0 {
                return 0;
            }
            if u(PP + 0x20) != 0 {
                return 1;
            }
            show_window(u(WINDOW), if n != 0 { 9 } else { 0 });
            1
        }
        0xA => {
            o(0).write(u(MODE_CUR));
            1
        }
        0xB => set_standard_functions_call(out.cast(), inout.addr() as u32, n),
        0xC => {
            o(0).write(dev_get_available_texture_mem(device()));
            1
        }
        0xD => {
            o(0).write(d3d_get_adapter_count(d3d9()));
            1
        }
        0xE => {
            let mut ident = [0u8; 0x44C];
            let ip = GPtr::from_raw(ident.as_mut_ptr());
            d3d_get_adapter_identifier(d3d9(), n as u32, 0, ip);
            rw_strcpy(out, ip.byte_add(0x200));
            1
        }
        0xF => {
            o(0).write(u(ADAPTER));
            1
        }
        0x10 => {
            set(ADAPTER, n as u32);
            count_modes();
            d3d_get_adapter_display_mode(d3d9(), u(ADAPTER), MODE as u32);
            build_mode_list_call();
            ms_probe();
            1
        }
        0x11 => {
            processor_initialize_call();
            if u(CPU_SSE) != 0 {
                game_fpu::ldmxcsr(game_fpu::stmxcsr() | 0x8000);
            }
            matrix_set_mult_fn(X87_MATRIX_MULTIPLY);
            if u(CPU_SSE) != 0 {
                if u(CPU_SSE2) != 0 && u(CPU_3DNOW) == 0 {
                    vector_set_mult_fn(0x80D570, 0x80D440, 0x80D600, 0x80D4E0);
                } else {
                    vector_set_mult_fn(0x80D280, 0x80D070, 0x80D370, 0x80D180);
                }
            } else {
                vector_set_mult_fn(0x8570EF, 0x857000, 0x857160, 0x85707F);
            }
            1
        }
        0x13 => {
            let (w, h) = (u(CAPS + 0x58), u(CAPS + 0x5C));
            o(0).write(if w >= h { w } else { h });
            1
        }
        0x16 => {
            out.cast::<u16>().write(2);
            1
        }
        _ => 0,
    }
}

// ------------------------------------------------------------------ CPU and x87 ----------------

/// `cpuid(1).edx` when CPUID is available (the originals' EFLAGS.ID test).
fn cpuid1_edx() -> Option<u32> {
    sa_core::abi::game_fpu::cpuid(1).map(|r| r[3])
}

/// `_rwIntelMMXsupported()` (0x80DBE0): CPUID.1:EDX bit 23 (0 without CPUID).
#[hook(addr = 0x80DBE0, conv = cdecl, name = "_rwIntelMMXsupported", expect = "55 8B EC 51 33 C9 53",
       route = "L2", hazards = "none", recs = "boot")]
pub fn intel_mmx_supported() -> u32 {
    cpuid1_edx().map_or(0, |d| (d >> 23) & 1)
}

/// `_rwIntelSSEsupported()` (0x80DC40): CPUID.1:EDX bits 25 or 26 (0 without CPUID).
#[hook(addr = 0x80DC40, conv = cdecl, name = "_rwIntelSSEsupported", expect = "55 8B EC 51 33 C9 53",
       route = "L2", hazards = "none", recs = "boot")]
pub fn intel_sse_supported() -> u32 {
    cpuid1_edx().map_or(0, |d| (d & 0x600_0000 != 0) as u32)
}

/// `_rwIntelSSE2supported()` (0x80DCA0): CPUID.1:EDX bit 26 (0 without CPUID).
#[hook(addr = 0x80DCA0, conv = cdecl, name = "_rwIntelSSE2supported", expect = "55 8B EC 51 33 C9 53",
       route = "L2", hazards = "none", recs = "boot")]
pub fn intel_sse2_supported() -> u32 {
    cpuid1_edx().map_or(0, |d| (d >> 26) & 1)
}

/// `_rwAMD3DNowSupported()` (0x80DD00): CPUID.80000001h:EDX bit 31 when the maximum extended leaf is
/// above 0x80000000 (0 without CPUID).
#[hook(addr = 0x80DD00, conv = cdecl, name = "_rwAMD3DNowSupported", expect = "55 8B EC 51 53 56 33 F6",
       route = "L2", hazards = "none", recs = "boot")]
pub fn amd_3dnow_supported() -> u32 {
    match sa_core::abi::game_fpu::cpuid(0x8000_0000) {
        Some(r) if r[0] > 0x8000_0000 => sa_core::abi::game_fpu::cpuid(0x8000_0001)
            .map_or(0, |e| (e[3] & 0x8000_0000 != 0) as u32),
        _ => 0,
    }
}

/// `_rwProcessorInitialize()` (0x857404): `fnstcw` into the low word of 0xCAF000, the whole dword
/// saved to 0xCAF004, then `(dword & ~0x300) | 0x3F` (24-bit precision, exceptions masked) stored and
/// loaded with `fldcw`. EAX is preserved.
#[hook(addr = 0x857404, conv = cdecl, name = "_rwProcessorInitialize", expect = "50 9B D9 3D 00 F0 CA 00",
       unverified = "std4-d3d9-1", route = "L2", hazards = "none", recs = "boot",
       note = "writes the x87 control word")]
pub fn processor_initialize() {
    g::<u16>(FPU_CW).write(game_fpu::fnstcw());
    let cw = u(FPU_CW);
    set(FPU_CW_SAVED, cw);
    let cw = (cw & 0xFFFF_FCFF) | 0x3F;
    set(FPU_CW, cw);
    game_fpu::fldcw(cw as u16);
}

/// `_rwProcessorRelease()` (0x85742B): `fldcw word [0xCAF004]`.
#[hook(addr = 0x85742B, conv = cdecl, name = "_rwProcessorRelease", expect = "D9 2D 04 F0 CA 00 C3",
       unverified = "std4-d3d9-1", route = "L2", hazards = "none", recs = "boot",
       note = "writes the x87 control word")]
pub fn processor_release() {
    game_fpu::fldcw(g::<u16>(FPU_CW_SAVED).read());
}

/// `_rwProcessorForceSinglePrecision()` (0x857432): `fnstcw` into the low word of 0xCAF000, the
/// dword `& ~0x300 | 0x3F`, `fldcw` of the low word.
#[hook(addr = 0x857432, conv = cdecl, name = "_rwProcessorForceSinglePrecision", expect = "9B D9 3D 00 F0 CA 00",
       unverified = "std4-d3d9-1", route = "L2", hazards = "none", recs = "boot",
       note = "writes the x87 control word")]
pub fn processor_force_single_precision() {
    g::<u16>(FPU_CW).write(game_fpu::fnstcw());
    set(FPU_CW, (u(FPU_CW) & 0xFFFF_FCFF) | 0x3F);
    game_fpu::fldcw(g::<u16>(FPU_CW).read());
}

/// `_rwX87MatrixMultiply` (0x8572A0).
pub const X87_MATRIX_MULTIPLY: u32 = 0x8572A0;

/// `_rwX87MatrixMultiply(dst, a, b)` (0x8572A0): `dst = a * b` for the 3x3 part and the position
/// (`dst.pos = a.pos * b + b.pos`); `dst` flags (+0xC) and the pads are not written. Each element is
/// the x87 sum `(a.x*b[0][c] + a.y*b[1][c]) + a.z*b[2][c]` (+ `b.pos[c]` for the position row).
/// The loads and stores follow the original's order (a row's three components are loaded before the
/// previous row's last element is stored; `b` is read per element), so aliasing behaves the same.
/// EAX, EBX and ECX are preserved.
#[hook(addr = 0x8572A0, conv = cdecl, name = "_rwX87MatrixMultiply", expect = "55 8B EC 50 53 51",
       unverified = "std4-d3d9-1", route = "L2", hazards = "none", recs = "boot")]
pub fn x87_matrix_multiply(dst: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) {
    let ld = |p: GPtr<u8>, o: isize| x(rd::<f32>(p, o));
    let st = |o: isize, v: sa_math::x87::X87| wr::<f32>(dst, o, f32_of(v));
    let (mut a0, mut a1, mut a2) = (ld(a, 0), ld(a, 4), ld(a, 8));
    for row in 0..4isize {
        let r = 0x10 * row;
        let pos = row == 3;
        let mut s0 = ld(b, 0).fmul(a0).fadd(ld(b, 0x10).fmul(a1)).fadd(ld(b, 0x20).fmul(a2));
        if pos {
            s0 = s0.fadd(ld(b, 0x30));
        }
        let p1 = ld(b, 4).fmul(a0);
        st(r, s0);
        let mut s1 = p1.fadd(ld(b, 0x14).fmul(a1)).fadd(ld(b, 0x24).fmul(a2));
        if pos {
            s1 = s1.fadd(ld(b, 0x34));
        }
        let p2 = a0.fmul(ld(b, 8));
        st(r + 4, s1);
        let mut s2 = p2.fadd(a1.fmul(ld(b, 0x18))).fadd(a2.fmul(ld(b, 0x28)));
        if pos {
            s2 = s2.fadd(ld(b, 0x38));
        } else {
            let n = r + 0x10;
            a2 = ld(a, n + 8);
            a1 = ld(a, n + 4);
            a0 = ld(a, n);
        }
        st(r + 8, s2);
    }
}

/// `FUN_00760cf0()` (0x760CF0, `_rpD3D9VertexShaderCacheReset`): the five cached shader keys
/// (0xC926F8..0xC92708) -1, 0xC94BF8 0, the 0x100-byte descriptor at 0xC94AF8 zeroed then given its
/// constants (+4, +8, +0x18, +0x1C, +0x24 = 2; +0xC, +0x28 = 4; +0x2C = 6; +0x104 = 0xC926F8). By the
/// caps' vertex shader version (low word of 0xC9BFC4) 0xC94C00 becomes: 3.x 0x19; 2.x 0x19 when caps
/// +0xFC is positive, else 0x14; 1.x 0xA (and caps +0xEC's bits 1..5 cleared); those three then store
/// `min(caps +0xC8, 0x100)` at 0xC94C04; without vertex shaders 0 (and nothing else).
#[hook(addr = 0x760CF0, conv = cdecl, name = "FUN_00760cf0", expect = "83 C8 FF 57 A3 F8 26 C9 00",
       route = "L2", hazards = "none", recs = "boot")]
pub fn vertex_shader_cache_reset() {
    for a in [0xC926F8, 0xC926FC, 0xC92700, 0xC92704] {
        set(a, 0xFFFF_FFFF);
    }
    set(0xC94BF8, 0);
    set(0xC92708, 0xFFFF_FFFF);
    for i in 0..0x40 {
        set(0xC94AF8 + 4 * i, 0);
    }
    for a in [0xC94AFC, 0xC94B00, 0xC94B10, 0xC94B14, 0xC94B1C] {
        set(a, 2);
    }
    let vs = u(CAPS + 0xC4) & 0xFFFF;
    set(0xC94BFC, 0xC926F8);
    set(0xC94B04, 4);
    set(0xC94B20, 4);
    set(0xC94B24, 6);
    if vs >= 0x300 {
        set(0xC94C00, 0x19);
    } else if vs >= 0x200 {
        set(0xC94C00, if (u(CAPS + 0xFC) as i32) <= 0 { 0x14 } else { 0x19 });
    } else if vs >= 0x100 {
        set(0xC94C00, 0xA);
        g::<u8>(CAPS + 0xEC).write(g::<u8>(CAPS + 0xEC).read() & 0xC1);
    } else {
        set(0xC94C00, 0);
        return;
    }
    let n = u(CAPS + 0xC8);
    set(0xC94C04, if n >= 0x100 { 0x100 } else { n });
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_VERTEX_SHADER_CACHE_RESET,
    HOOK_MULTISAMPLE_PROBE,
    HOOK_BUILD_MODE_LIST,
    HOOK_SET_STANDARD_FUNCTIONS,
    HOOK_SET_PRESENT_PARAMETERS,
    HOOK_RESET_CACHES,
    HOOK_RESET_TRANSFORM_CACHE,
    HOOK_DEVICE_STOP,
    HOOK_RELEASE_VIDEO_MEMORY,
    HOOK_D3D9_SYSTEM,
    HOOK_INTEL_MMX_SUPPORTED,
    HOOK_INTEL_SSE_SUPPORTED,
    HOOK_INTEL_SSE2_SUPPORTED,
    HOOK_AMD_3DNOW_SUPPORTED,
    HOOK_PROCESSOR_INITIALIZE,
    HOOK_PROCESSOR_RELEASE,
    HOOK_PROCESSOR_FORCE_SINGLE_PRECISION,
    HOOK_X87_MATRIX_MULTIPLY,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_STANDARD_NOT_IMPLEMENTED];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
