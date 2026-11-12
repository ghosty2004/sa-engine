//! Native `IDirect3DDevice9` methods and D3D9 resource objects on the render seam
//! (`sa_platform::gpu::D3d9Device`): the device `IDirect3D9::CreateDevice` ([`crate::dx`]) creates,
//! a headless `WgpuDevice` of the present parameters' back-buffer size.
//!
//! The ported RW D3D9 driver (PORT-RW-4, sa-rwcore `d3d9*.rs`, `raster.rs`, `im2d.rs`) calls the
//! device through `orig!` `vslot` declarations exactly as the original does; in phase B those
//! slots of the device object ([`crate::dx::DEVICE9`]) are traps whose methods are implemented
//! here by translating each call to the seam: the D3D9 call stream reaches `WgpuDevice` unchanged.
//! Resources (textures, surfaces, vertex/index buffers, vertex declarations) are heap objects
//! `{vtable, handle}` whose handle indexes [`RESOURCES`]; their methods are traps too. Buffers and
//! texture levels keep a CPU copy: `Lock` hands out the copy, `Unlock` uploads the locked bytes.
//! Shaders (`CreateVertexShader`/`CreatePixelShader`, `Set`/`Get`, constants) live in
//! [`crate::d3d9_shader`].

// COM method signatures are dictated by D3D9.
#![allow(clippy::too_many_arguments)]

use std::collections::HashMap;
use std::sync::Mutex;

use sa_core::mem::GPtr;
use sa_platform::gpu::{self, D3d9Device, PrimitiveType};
use sa_rw_wgpu::d3d9::WgpuDevice;

use crate::bind::with;

/// The device `IDirect3D9::CreateDevice` created.
static DEVICE: Mutex<Option<WgpuDevice>> = Mutex::new(None);

/// Runs `f` on the device (`None` before `CreateDevice`).
pub fn with_device<R>(f: impl FnOnce(&mut WgpuDevice) -> R) -> Option<R> {
    let mut d = DEVICE.lock().unwrap_or_else(|e| e.into_inner());
    d.as_mut().map(f)
}

/// The native window's swap chain (`--window`): `CreateDevice` creates the seam device on the window's GPU device
/// and every `Present` draws the back buffer into the window surface and presents it.
pub struct WindowTarget {
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

static WINDOW: Mutex<Option<WindowTarget>> = Mutex::new(None);

/// Installs the window's swap chain (before the boot creates the device).
pub fn set_window(t: WindowTarget) {
    *WINDOW.lock().unwrap_or_else(|e| e.into_inner()) = Some(t);
}

/// The window's client area changed: the surface is reconfigured (the back buffer keeps its size and is stretched).
pub fn resize_window(w: u32, h: u32) {
    if let Some(t) = WINDOW.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        t.config.width = w.max(1);
        t.config.height = h.max(1);
        t.surface.configure(&t.device, &t.config);
    }
}

/// Draws the presented back buffer into the window surface and presents it.
fn present_to_window(d: &mut WgpuDevice) {
    let mut w = WINDOW.lock().unwrap_or_else(|e| e.into_inner());
    let Some(t) = w.as_mut() else { return };
    let st = match t.surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
        _ => {
            // outdated / lost: reconfigured, this frame is not shown
            t.surface.configure(&t.device, &t.config);
            return;
        }
    };
    let view = st.texture.create_view(&Default::default());
    d.blit_to(&view, t.config.format);
    st.present();
}

/// Wall-clock times of the in-game `Present` calls (`Game::Idle` frames), for the frame-pacing report.
static FRAME_TIMES: Mutex<Vec<std::time::Instant>> = Mutex::new(Vec::new());

/// The in-game present times so far.
pub fn frame_times() -> Vec<std::time::Instant> {
    FRAME_TIMES.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Creates the seam device (a `WgpuDevice` of `w` x `h`: on the window's GPU device with `--window`, otherwise
/// headless) whose swap chain reports the back-buffer format `color` and the depth-stencil format `depth`; false
/// when no adapter exists.
pub fn create_device(w: u32, h: u32, color: u32, depth: u32) -> bool {
    *SWAP_FORMATS.lock().unwrap_or_else(|e| e.into_inner()) = (color, depth);
    let window = WINDOW
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map(|t| (t.device.clone(), t.queue.clone()));
    let dev = match window {
        Some((device, queue)) => Some(WgpuDevice::new(device, queue, w, h)),
        None => WgpuDevice::new_headless(w, h),
    };
    let ok = dev.is_some();
    *DEVICE.lock().unwrap_or_else(|e| e.into_inner()) = dev;
    *SWAP_SURFACES.lock().unwrap_or_else(|e| e.into_inner()) = None;
    *RENDER_TARGET.lock().unwrap_or_else(|e| e.into_inner()) = (0, 0);
    ok
}

/// Where `Present` writes each frame (`--present-dir`), and how many so far.
static PRESENT_DIR: Mutex<Option<(std::path::PathBuf, u32)>> = Mutex::new(None);

/// `--present-dir DIR` (created when missing): every presented frame is written to `DIR/present-NNNN.png` (at most 200;
/// `SA_GAME_PRESENT_SKIP=K` skips the first K, `SA_GAME_PRESENT_EVERY=N` keeps every N-th after that;
/// every 32nd frame not kept overwrites `DIR/present-last.png`; `SA_GAME_PRESENT_GAME=1` counts only the in-game
/// frames).
pub fn set_present_dir(dir: std::path::PathBuf) {
    // created here: `File::create` below fails silently in a missing directory
    let _ = std::fs::create_dir_all(&dir);
    *PRESENT_DIR.lock().unwrap_or_else(|e| e.into_inner()) = Some((dir, 0));
}

/// Set by the main loop while `rsIDLE` (`Game::Idle`, the in-game frame) runs.
pub static IN_GAME_FRAME: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `Present` on the seam device, and the frame to `--present-dir`.
fn present_frame() -> i32 {
    let in_game = IN_GAME_FRAME.load(std::sync::atomic::Ordering::SeqCst);
    let read_back = PRESENT_DIR.lock().unwrap_or_else(|e| e.into_inner()).is_some();
    let r = with_device(|d| {
        let r = d.present();
        present_to_window(d);
        (r, if read_back { d.read_render_target(None).ok() } else { None })
    });
    if in_game {
        FRAME_TIMES.lock().unwrap_or_else(|e| e.into_inner()).push(std::time::Instant::now());
    }
    // `SA_GAME_PRESENT_GAME=1`: only the frames presented inside `Game::Idle` (the in-game frames) are counted and
    // written; the loading screens are not.
    if std::env::var_os("SA_GAME_PRESENT_GAME").is_some() && !in_game {
        with("Present", |_| ());
        return match r {
            Some((Ok(()), _)) => D3D_OK,
            Some((Err(e), _)) => e.hresult() as i32,
            None => D3DERR_INVALIDCALL,
        };
    }
    let mut pd = PRESENT_DIR.lock().unwrap_or_else(|e| e.into_inner());
    // `SA_GAME_PRESENT_SKIP=K`: the first K presented frames are not written.
    static SEEN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let seen = SEEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let skip: u32 = std::env::var("SA_GAME_PRESENT_SKIP").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let every: u32 = std::env::var("SA_GAME_PRESENT_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(1).max(1);
    if let (Some((dir, n)), Some((_, Some((w, h, rgba))))) = (pd.as_mut(), &r) {
        let keep = *n < 200 && seen >= skip && (seen - skip).is_multiple_of(every);
        let path = if keep {
            *n += 1;
            Some(dir.join(format!("present-{:04}.png", *n - 1)))
        } else if seen.is_multiple_of(32) {
            Some(dir.join("present-last.png"))
        } else {
            None
        };
        if let Some(Ok(f)) = path.map(std::fs::File::create) {
            let mut enc = png::Encoder::new(std::io::BufWriter::new(f), *w, *h);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let _ = enc.write_header().and_then(|mut wr| wr.write_image_data(rgba));
        }
    }
    drop(pd);
    with("Present", |_| ());
    match r {
        Some((Ok(()), _)) => D3D_OK,
        Some((Err(e), _)) => e.hresult() as i32,
        None => D3DERR_INVALIDCALL,
    }
}

/// The page holding the resource vtables.
const PAGE: usize = 0x0003_2000;
/// Resource method traps: `TRAP + 0x400 * class + 4 * slot`.
const TRAP: u32 = 0x0006_0000;

const TEXTURE_METHODS: [&str; 22] = [
    "QueryInterface", "AddRef", "Release", "GetDevice", "SetPrivateData", "GetPrivateData",
    "FreePrivateData", "SetPriority", "GetPriority", "PreLoad", "GetType", "SetLOD", "GetLOD",
    "GetLevelCount", "SetAutoGenFilterType", "GetAutoGenFilterType", "GenerateMipSubLevels",
    "GetLevelDesc", "GetSurfaceLevel", "LockRect", "UnlockRect", "AddDirtyRect",
];
const SURFACE_METHODS: [&str; 17] = [
    "QueryInterface", "AddRef", "Release", "GetDevice", "SetPrivateData", "GetPrivateData",
    "FreePrivateData", "SetPriority", "GetPriority", "PreLoad", "GetType", "GetContainer",
    "GetDesc", "LockRect", "UnlockRect", "GetDC", "ReleaseDC",
];
const BUFFER_METHODS: [&str; 14] = [
    "QueryInterface", "AddRef", "Release", "GetDevice", "SetPrivateData", "GetPrivateData",
    "FreePrivateData", "SetPriority", "GetPriority", "PreLoad", "GetType", "Lock", "Unlock",
    "GetDesc",
];
const DECL_METHODS: [&str; 5] = ["QueryInterface", "AddRef", "Release", "GetDevice", "GetDeclaration"];

struct Class {
    name: &'static str,
    methods: &'static [&'static str],
}
const CLASSES: [Class; 4] = [
    Class { name: "IDirect3DTexture9", methods: &TEXTURE_METHODS },
    Class { name: "IDirect3DSurface9", methods: &SURFACE_METHODS },
    Class { name: "IDirect3DBuffer9", methods: &BUFFER_METHODS },
    Class { name: "IDirect3DVertexDeclaration9", methods: &DECL_METHODS },
];
const TEXTURE: usize = 0;
const SURFACE: usize = 1;
const BUFFER: usize = 2;
const DECL: usize = 3;

fn vtable(class: usize) -> u32 {
    (PAGE + 0x100 * class) as u32
}
fn trap(class: usize, slot: usize) -> u32 {
    TRAP + 0x400 * class as u32 + 4 * slot as u32
}

/// A D3D9 resource behind an object.
enum Res {
    Texture { id: gpu::TextureId, w: u32, h: u32, levels: u32, format: u32, usage: u32, pool: u32, locked: HashMap<u32, Vec<u8>> },
    /// A surface: a texture level (`tex`), a stand-alone render target or depth-stencil surface on
    /// its own seam texture (`own`), an offscreen plain surface in system memory (`sys`, never
    /// empty), or one of the swap chain's surfaces (none of these: back or depth buffer).
    Surface { tex: Option<u32>, level: u32, w: u32, h: u32, format: u32, usage: u32, pool: u32, own: Option<gpu::TextureId>, sys: Vec<u8> },
    VertexBuffer { id: gpu::VertexBufferId, data: Vec<u8>, lock: (u32, u32) },
    IndexBuffer { id: gpu::IndexBufferId, data: Vec<u8>, format: u32, lock: (u32, u32) },
    Decl { id: gpu::VertexDeclId, elems: Vec<[u8; 8]> },
}

struct Store {
    next: u32,
    res: HashMap<u32, (Res, u32)>,
}

static RESOURCES: Mutex<Option<Store>> = Mutex::new(None);

fn with_store<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    let mut s = RESOURCES.lock().unwrap_or_else(|e| e.into_inner());
    f(s.get_or_insert_with(|| Store { next: 1, res: HashMap::new() }))
}

/// A new object `{vtable, handle}` for `res` (reference count 1).
fn new_object(class: usize, res: Res) -> u32 {
    let h = with_store(|s| {
        let h = s.next;
        s.next += 1;
        s.res.insert(h, (res, 1));
        h
    });
    let obj: &'static mut [u32; 2] = Box::leak(Box::new([vtable(class), h]));
    obj.as_ptr() as u32
}
fn handle(obj: u32) -> u32 {
    GPtr::<u32>::from_addr(obj as usize).wrapping_index(1).read()
}
fn with_res<R>(obj: u32, f: impl FnOnce(&mut Res) -> R) -> Option<R> {
    let h = handle(obj);
    with_store(|s| s.res.get_mut(&h).map(|(r, _)| f(r)))
}

/// Maps the vtable page.
#[allow(unsafe_code)]
pub fn map() -> Result<(), String> {
    // SAFETY: an anonymous private mapping at a fixed low address that MAP_FIXED_NOREPLACE refuses
    // to place over anything already mapped; never unmapped.
    let p = unsafe {
        libc::mmap(
            PAGE as *mut libc::c_void,
            0x1000,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_FIXED_NOREPLACE,
            -1,
            0,
        )
    };
    if p as usize != PAGE {
        return Err(format!("cannot map the D3D9 resource page at {PAGE:#x}"));
    }
    for (c, cl) in CLASSES.iter().enumerate() {
        for i in 0..cl.methods.len() {
            GPtr::<u32>::from_addr(vtable(c) as usize + 4 * i).write(trap(c, i));
        }
    }
    Ok(())
}

/// Every resource method trap's name.
pub fn names() -> Vec<(u32, &'static str)> {
    let mut v = Vec::new();
    for (c, cl) in CLASSES.iter().enumerate() {
        for (i, m) in cl.methods.iter().enumerate() {
            v.push((trap(c, i), &*Box::leak(format!("{}::{m}", cl.name).into_boxed_str())));
        }
    }
    v
}

const D3D_OK: i32 = 0;
const D3DERR_INVALIDCALL: i32 = 0x8876_086Cu32 as i32;
const D3DERR_NOTAVAILABLE: i32 = 0x8876_086Au32 as i32;

fn rd32(p: GPtr<u8>, i: isize) -> u32 {
    p.byte_add(4 * i).cast::<u32>().read()
}
fn rdf(p: GPtr<u8>, i: isize) -> f32 {
    f32::from_bits(rd32(p, i))
}
fn bytes(p: GPtr<u8>, n: usize) -> Vec<u8> {
    (0..n as isize).map(|i| p.byte_add(i).read()).collect()
}
fn prim(t: u32) -> Option<PrimitiveType> {
    PrimitiveType::from_d3d(t)
}
fn hr<T>(r: Option<gpu::D3dResult<T>>) -> i32 {
    match r {
        Some(Ok(_)) => D3D_OK,
        _ => D3DERR_INVALIDCALL,
    }
}

// ------------------------------------------------------------------ device -------------------

/// The back-buffer and depth surfaces (`GetRenderTarget(0)` / `GetBackBuffer`,
/// `GetDepthStencilSurface`), created on first use.
static SWAP_SURFACES: Mutex<Option<(u32, u32)>> = Mutex::new(None);
/// Their formats: the present parameters' back-buffer and auto depth-stencil formats.
static SWAP_FORMATS: Mutex<(u32, u32)> = Mutex::new((gpu::fmt::X8R8G8B8, gpu::fmt::D24S8));

fn swap_surfaces() -> (u32, u32) {
    let mut s = SWAP_SURFACES.lock().unwrap_or_else(|e| e.into_inner());
    *s.get_or_insert_with(|| {
        let (w, h) = with_device(|d| d.back_buffer_size()).unwrap_or((0, 0));
        let mk = |format: u32, usage: u32| {
            new_object(SURFACE, Res::Surface { tex: None, level: 0, w, h, format, usage, pool: 0, own: None, sys: Vec::new() })
        };
        let (c, z) = *SWAP_FORMATS.lock().unwrap_or_else(|e| e.into_inner());
        (mk(c, 1), mk(z, 2))
    })
}

fn dev_test_cooperative_level(_this: u32) -> i32 {
    D3D_OK
}
fn dev_get_available_texture_mem(_this: u32) -> u32 {
    256 << 20
}
/// The gamma ramp the device reports and accepts (3 x 256 words). The seam has no gamma: the ramp
/// is stored and returned, not applied (identity on start).
static GAMMA_RAMP: Mutex<Option<[u16; 768]>> = Mutex::new(None);
fn dev_set_gamma_ramp(_this: u32, _swap: u32, _flags: u32, ramp: GPtr<u16>) {
    let mut r = [0u16; 768];
    for (i, v) in r.iter_mut().enumerate() {
        *v = ramp.wrapping_index(i as i32).read();
    }
    *GAMMA_RAMP.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
}
fn dev_get_gamma_ramp(_this: u32, _swap: u32, ramp: GPtr<u16>) {
    let r = GAMMA_RAMP
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or_else(|| std::array::from_fn(|i| ((i % 256) * 257) as u16));
    for (i, v) in r.iter().enumerate() {
        ramp.wrapping_index(i as i32).write(*v);
    }
}
fn dev_present(_this: u32, _src: u32, _dst: u32, _hwnd: u32, _dirty: u32) -> i32 {
    dump_memory();
    present_frame()
}

/// `SA_GAME_DUMP=ADDR:LEN[,ADDR:LEN...]` (exploration only): every `Present` prints those game-memory
/// ranges as dwords, with their f32 reading.
fn dump_memory() {
    let Some(spec) = std::env::var_os("SA_GAME_DUMP") else { return };
    for part in spec.to_string_lossy().split(',') {
        let mut it = part.split(':');
        let parse = |v: Option<&str>| v.and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok());
        // `BASE>OFF>OFF...`: each `>` reads the dword at the address so far and adds the offset
        let Some(path) = it.next() else { continue };
        let mut hops = path.split('>');
        let Some(mut a) = parse(hops.next()) else { continue };
        let mut ok = true;
        for h in hops {
            let Some(off) = parse(Some(h)) else { ok = false; break };
            if a < 0x1000 {
                ok = false;
                break;
            }
            a = GPtr::<u32>::from_addr(a as usize).read().wrapping_add(off);
        }
        let Some(n) = parse(it.next()) else { continue };
        if !ok || a < 0x1000 {
            eprintln!("sa-game: dump {path}: null");
            continue;
        }
        let mut line = format!("sa-game: dump {path} = {a:#x}:");
        for k in 0..n.div_ceil(4) {
            let v = GPtr::<u32>::from_addr((a + 4 * k) as usize).read();
            line += &format!(" {v:08x}({})", f32::from_bits(v));
        }
        eprintln!("{line}");
    }
}
/// `EvictManagedResources`: nothing to evict (the seam keeps every resource); `D3D_OK`.
fn dev_evict_managed_resources(_this: u32) -> i32 {
    D3D_OK
}
/// `Reset(D3DPRESENT_PARAMETERS*)`: the back buffer resized to the parameters' size (0 keeps the
/// current size: a windowed device takes its window's client area, which covers the back buffer
/// here); the swap chain's surfaces follow.
fn dev_reset(_this: u32, pp: GPtr<u32>) -> i32 {
    let (cw, ch) = with_device(|d| d.back_buffer_size()).unwrap_or((0, 0));
    let w = match pp.read() {
        0 => cw,
        v => v,
    };
    let h = match pp.wrapping_index(1).read() {
        0 => ch,
        v => v,
    };
    // the same size keeps the back buffer (its contents are undefined after a D3D9 Reset)
    let r = with_device(|d| if (w, h) == d.back_buffer_size() { Ok(()) } else { d.reset(w, h) });
    let swap = *SWAP_SURFACES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((c, z)) = swap {
        for o in [c, z] {
            with_res(o, |r| {
                if let Res::Surface { w: sw, h: sh, .. } = r {
                    *sw = w;
                    *sh = h;
                }
            });
        }
    }
    hr(r)
}
/// `CreateCubeTexture`: the seam has no cube textures (platform.md §4.1): its refusal as the
/// HRESULT (`D3DERR_NOTAVAILABLE`).
fn dev_create_cube_texture(_this: u32, edge: u32, levels: u32, usage: u32, format: u32, _pool: u32, _out: GPtr<u32>, _shared: u32) -> i32 {
    let desc = gpu::TextureDesc {
        width: edge,
        height: edge,
        levels,
        format,
        render_target: usage & 1 != 0,
        depth_stencil: usage & 2 != 0,
        dynamic: usage & 0x200 != 0,
        cube: true,
    };
    match with_device(|d| d.create_texture(&desc)) {
        Some(Ok(id)) => {
            // a backend with cube textures: no cube object (faces, GetCubeMapSurface) yet
            with_device(|d| d.release_texture(id));
            D3DERR_NOTAVAILABLE
        }
        Some(Err(e)) => e.hresult() as i32,
        None => D3DERR_INVALIDCALL,
    }
}
/// A stand-alone surface on its own seam texture (render target or depth-stencil).
fn own_surface(w: u32, h: u32, format: u32, usage: u32, out: GPtr<u32>) -> i32 {
    let desc = gpu::TextureDesc {
        width: w,
        height: h,
        levels: 1,
        format,
        render_target: usage & 1 != 0,
        depth_stencil: usage & 2 != 0,
        dynamic: false,
        cube: false,
    };
    match with_device(|d| d.create_texture(&desc)) {
        Some(Ok(id)) => {
            out.write(new_object(SURFACE, Res::Surface { tex: None, level: 0, w, h, format, usage, pool: 0, own: Some(id), sys: Vec::new() }));
            D3D_OK
        }
        Some(Err(e)) => e.hresult() as i32,
        None => D3DERR_INVALIDCALL,
    }
}
/// `CreateRenderTarget(w, h, format, ms, quality, lockable, out, shared)`: a render-target surface
/// (single-sampled: the seam has no multisampling; not lockable).
fn dev_create_render_target(_this: u32, w: u32, h: u32, format: u32, _ms: u32, _q: u32, _lockable: u32, out: GPtr<u32>, _shared: u32) -> i32 {
    own_surface(w, h, format, 1, out)
}
/// `CreateDepthStencilSurface(w, h, format, ms, quality, discard, out, shared)`: a depth-stencil
/// surface (the seam's Depth24PlusStencil8 whatever the D3D depth format).
fn dev_create_depth_stencil_surface(_this: u32, w: u32, h: u32, format: u32, _ms: u32, _q: u32, _discard: u32, out: GPtr<u32>, _shared: u32) -> i32 {
    own_surface(w, h, format, 2, out)
}
/// Row pitch of a system-memory surface (block rows for DXT).
fn sys_pitch(format: u32, w: u32) -> u32 {
    match gpu::fmt::block_bytes(format) {
        Some(b) => w.div_ceil(4) * b,
        None => w * gpu::fmt::bytes_per_pixel(format).unwrap_or(4),
    }
}
/// `CreateOffscreenPlainSurface(w, h, format, pool, out, shared)`: a system-memory surface (CPU
/// bytes, locked in place), the source of `UpdateSurface` and the destination of
/// `GetRenderTargetData`.
fn dev_create_offscreen_plain_surface(_this: u32, w: u32, h: u32, format: u32, pool: u32, out: GPtr<u32>, _shared: u32) -> i32 {
    let block = gpu::fmt::block_bytes(format).is_some();
    if w == 0 || h == 0 || (gpu::fmt::bytes_per_pixel(format).is_none() && !block) {
        return D3DERR_INVALIDCALL;
    }
    let rows = if block { h.div_ceil(4) } else { h };
    let sys = vec![0u8; (sys_pitch(format, w) * rows) as usize];
    out.write(new_object(SURFACE, Res::Surface { tex: None, level: 0, w, h, format, usage: 0, pool, own: None, sys }));
    D3D_OK
}
/// A surface's seam render target: `Some(None)` the back buffer, `Some(Some(id))` a render-target
/// texture (level 0 of a render-target texture, or an own render-target surface), `None` not a
/// render target.
fn render_target_of(obj: u32) -> Option<Option<gpu::TextureId>> {
    let swap = SWAP_SURFACES.lock().unwrap_or_else(|e| e.into_inner()).map(|s| s.0);
    if swap == Some(obj) {
        return Some(None);
    }
    let usage = with_res(obj, |r| match r {
        Res::Surface { usage, .. } => *usage,
        _ => 0,
    })?;
    if usage & 1 == 0 {
        return None;
    }
    surface_texture(obj).map(Some)
}
/// A `RECT*` (left, top, right, bottom) as (x, y, w, h); NULL = `None`.
fn rect_of(p: GPtr<i32>) -> Option<(u32, u32, u32, u32)> {
    if p.is_null() {
        return None;
    }
    let [l, t, r, b] = [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read(), p.wrapping_index(3).read()];
    Some((l as u32, t as u32, r.wrapping_sub(l) as u32, b.wrapping_sub(t) as u32))
}
/// `StretchRect(src, srcRect, dst, dstRect, filter)` between render targets (back buffer,
/// render-target surfaces, render-target texture levels) on the seam; D3DTEXF_LINEAR (2) filters,
/// the other filters take the nearest texel.
fn dev_stretch_rect(_this: u32, src: u32, src_rect: GPtr<i32>, dst: u32, dst_rect: GPtr<i32>, filter: u32) -> i32 {
    let (Some(s), Some(d)) = (render_target_of(src), render_target_of(dst)) else { return D3DERR_INVALIDCALL };
    hr(with_device(|dev| dev.stretch_rect(s, rect_of(src_rect), d, rect_of(dst_rect), filter == 2)))
}
/// RGBA8 pixels in `format` (A8R8G8B8 / X8R8G8B8: B, G, R, A bytes; R5G6B5; X1R5G5B5 / A1R5G5B5);
/// `None` for other formats.
fn rgba_to(format: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let c = |v: u8, bits: u32| (v as u16) >> (8 - bits);
    match format {
        0x15 | 0x16 => Some(
            rgba.chunks(4)
                .flat_map(|p| [p[2], p[1], p[0], if format == 0x15 { p[3] } else { 0xFF }])
                .collect(),
        ),
        0x17 => Some(
            rgba.chunks(4)
                .flat_map(|p| ((c(p[0], 5) << 11) | (c(p[1], 6) << 5) | c(p[2], 5)).to_le_bytes())
                .collect(),
        ),
        0x18 | 0x19 => Some(
            rgba.chunks(4)
                .flat_map(|p| {
                    let a = if format == 0x19 { c(p[3], 1) << 15 } else { 0x8000 };
                    (a | (c(p[0], 5) << 10) | (c(p[1], 5) << 5) | c(p[2], 5)).to_le_bytes()
                })
                .collect(),
        ),
        _ => None,
    }
}
/// `GetRenderTargetData(renderTarget, sysmemSurface)`: the render target read back into the
/// system-memory surface of the same size, converted to its format.
fn dev_get_render_target_data(_this: u32, rt: u32, dst: u32) -> i32 {
    let Some(src) = render_target_of(rt) else { return D3DERR_INVALIDCALL };
    let Some(Ok((w, h, rgba))) = with_device(|d| d.read_render_target(src)) else { return D3DERR_INVALIDCALL };
    let r = with_res(dst, |r| match r {
        Res::Surface { sys, w: dw, h: dh, format, .. } if !sys.is_empty() && *dw == w && *dh == h => {
            let px = rgba_to(*format, &rgba)?;
            sys[..px.len()].copy_from_slice(&px);
            Some(())
        }
        _ => None,
    })
    .flatten();
    if r.is_some() { D3D_OK } else { D3DERR_INVALIDCALL }
}
/// `UpdateSurface(sysmemSrc, srcRect, dst, dstPoint)`: the source rectangle (NULL: the whole
/// surface) of a system-memory surface uploaded into a texture level of the same format at the
/// point (NULL: the rectangle's origin).
fn dev_update_surface(_this: u32, src: u32, src_rect: GPtr<i32>, dst: u32, pt: GPtr<i32>) -> i32 {
    let s = with_res(src, |r| match r {
        Res::Surface { sys, w, h, format, .. } if !sys.is_empty() => Some((sys.clone(), *w, *h, *format)),
        _ => None,
    })
    .flatten();
    let Some((data, w, h, format)) = s else { return D3DERR_INVALIDCALL };
    let (x, y, rw, rh) = rect_of(src_rect).unwrap_or((0, 0, w, h));
    let (dx, dy) = if pt.is_null() { (x, y) } else { (pt.read() as u32, pt.wrapping_index(1).read() as u32) };
    let d = with_res(dst, |r| match r {
        Res::Surface { tex: Some(t), level, format: f, .. } => Some((*t, *level, *f)),
        _ => None,
    })
    .flatten();
    let Some((tex, level, dformat)) = d else { return D3DERR_INVALIDCALL };
    if dformat != format {
        return D3DERR_INVALIDCALL;
    }
    let id = with_res(tex, |r| match r {
        Res::Texture { id, .. } => Some(*id),
        _ => None,
    })
    .flatten();
    let Some(id) = id else { return D3DERR_INVALIDCALL };
    let pitch = sys_pitch(format, w);
    let off = match gpu::fmt::block_bytes(format) {
        Some(b) => (y / 4) * pitch + (x / 4) * b,
        None => y * pitch + x * gpu::fmt::bytes_per_pixel(format).unwrap_or(4),
    } as usize;
    if off >= data.len() {
        return D3DERR_INVALIDCALL;
    }
    hr(with_device(|d| d.update_texture(id, 0, level, Some((dx, dy, rw, rh)), &data[off..], pitch)))
}
fn dev_get_back_buffer(_this: u32, _swap: u32, _index: u32, _ty: u32, out: GPtr<u32>) -> i32 {
    out.write(swap_surfaces().0);
    D3D_OK
}
fn dev_create_texture(_this: u32, w: u32, h: u32, levels: u32, usage: u32, format: u32, pool: u32, out: GPtr<u32>, _shared: u32) -> i32 {
    let desc = gpu::TextureDesc {
        width: w,
        height: h,
        levels,
        format,
        render_target: usage & 1 != 0,
        depth_stencil: usage & 2 != 0,
        dynamic: usage & 0x200 != 0,
        cube: false,
    };
    match with_device(|d| d.create_texture(&desc)) {
        Some(Ok(id)) => {
            let n = if levels == 0 { 32 - (w.max(h).max(1)).leading_zeros() } else { levels };
            out.write(new_object(TEXTURE, Res::Texture { id, w, h, levels: n, format, usage, pool, locked: HashMap::new() }));
            D3D_OK
        }
        r => {
            if std::env::var_os("SA_GAME_D3D_TRACE").is_some() {
                eprintln!("sa-game: CreateTexture({w}x{h}, levels {levels}, usage {usage:#x}, format {format:#x}) failed: {:?}", r.map(|e| e.err()));
            }
            D3DERR_INVALIDCALL
        }
    }
}
fn dev_create_vertex_buffer(_this: u32, len: u32, usage: u32, _fvf: u32, _pool: u32, out: GPtr<u32>, _shared: u32) -> i32 {
    match with_device(|d| d.create_vertex_buffer(len, usage & 0x200 != 0)) {
        Some(Ok(id)) => {
            out.write(new_object(BUFFER, Res::VertexBuffer { id, data: vec![0; len as usize], lock: (0, 0) }));
            D3D_OK
        }
        _ => D3DERR_INVALIDCALL,
    }
}
fn dev_create_index_buffer(_this: u32, len: u32, usage: u32, format: u32, _pool: u32, out: GPtr<u32>, _shared: u32) -> i32 {
    match with_device(|d| d.create_index_buffer(len, format, usage & 0x200 != 0)) {
        Some(Ok(id)) => {
            out.write(new_object(BUFFER, Res::IndexBuffer { id, data: vec![0; len as usize], format, lock: (0, 0) }));
            D3D_OK
        }
        _ => D3DERR_INVALIDCALL,
    }
}
/// The seam texture of a render-target surface object (`None` = the back buffer / depth buffer).
fn surface_texture(obj: u32) -> Option<gpu::TextureId> {
    if obj == 0 {
        return None;
    }
    let (tex, own) = with_res(obj, |r| match r {
        Res::Surface { tex, own, .. } => (*tex, *own),
        _ => (None, None),
    })?;
    if own.is_some() {
        return own;
    }
    let tex = tex?;
    with_res(tex, |r| match r {
        Res::Texture { id, .. } => Some(*id),
        _ => None,
    })
    .flatten()
}
static RENDER_TARGET: Mutex<(u32, u32)> = Mutex::new((0, 0));
fn dev_set_render_target(_this: u32, index: u32, surface: u32) -> i32 {
    if index != 0 {
        return D3DERR_INVALIDCALL;
    }
    let mut rt = RENDER_TARGET.lock().unwrap_or_else(|e| e.into_inner());
    rt.0 = surface;
    let (c, d) = (surface_texture(rt.0), surface_texture(rt.1));
    with_device(|dev| dev.set_render_target(c, d));
    D3D_OK
}
fn dev_get_render_target(_this: u32, index: u32, out: GPtr<u32>) -> i32 {
    if index != 0 {
        return D3DERR_INVALIDCALL;
    }
    let s = RENDER_TARGET.lock().unwrap_or_else(|e| e.into_inner()).0;
    out.write(if s == 0 { swap_surfaces().0 } else { s });
    D3D_OK
}
fn dev_set_depth_stencil_surface(_this: u32, surface: u32) -> i32 {
    let mut rt = RENDER_TARGET.lock().unwrap_or_else(|e| e.into_inner());
    rt.1 = surface;
    let (c, d) = (surface_texture(rt.0), surface_texture(rt.1));
    with_device(|dev| dev.set_render_target(c, d));
    D3D_OK
}
fn dev_get_depth_stencil_surface(_this: u32, out: GPtr<u32>) -> i32 {
    let s = RENDER_TARGET.lock().unwrap_or_else(|e| e.into_inner()).1;
    out.write(if s == 0 { swap_surfaces().1 } else { s });
    D3D_OK
}
fn dev_begin_scene(_this: u32) -> i32 {
    hr(with_device(|d| d.begin_scene()))
}
fn dev_end_scene(_this: u32) -> i32 {
    hr(with_device(|d| d.end_scene()))
}
/// `Clear(count, rects, flags, color, z, stencil)`: rectangles are not modelled (the whole
/// viewport is cleared; RW clears whole cameras).
fn dev_clear(_this: u32, _count: u32, _rects: u32, flags: u32, color: u32, z: f32, stencil: u32) -> i32 {
    with_device(|d| d.clear(flags, color, z, stencil));
    D3D_OK
}
fn dev_set_transform(_this: u32, state: u32, m: GPtr<u8>) -> i32 {
    let mut mm = [[0f32; 4]; 4];
    for (r, row) in mm.iter_mut().enumerate() {
        for (c, v) in row.iter_mut().enumerate() {
            *v = rdf(m, (4 * r + c) as isize);
        }
    }
    with_device(|d| d.set_transform(state, &mm));
    D3D_OK
}
fn dev_set_viewport(_this: u32, vp: GPtr<u8>) -> i32 {
    let v = gpu::Viewport {
        x: rd32(vp, 0),
        y: rd32(vp, 1),
        width: rd32(vp, 2),
        height: rd32(vp, 3),
        min_z: rdf(vp, 4),
        max_z: rdf(vp, 5),
    };
    with_device(|d| d.set_viewport(&v));
    D3D_OK
}
fn color4(p: GPtr<u8>, i: isize) -> [f32; 4] {
    [rdf(p, i), rdf(p, i + 1), rdf(p, i + 2), rdf(p, i + 3)]
}
fn dev_set_material(_this: u32, m: GPtr<u8>) -> i32 {
    let mat = gpu::Material {
        diffuse: color4(m, 0),
        ambient: color4(m, 4),
        specular: color4(m, 8),
        emissive: color4(m, 12),
        power: rdf(m, 16),
    };
    with_device(|d| d.set_material(&mat));
    D3D_OK
}
fn dev_set_light(_this: u32, index: u32, l: GPtr<u8>) -> i32 {
    let light = gpu::Light {
        kind: rd32(l, 0),
        diffuse: color4(l, 1),
        specular: color4(l, 5),
        ambient: color4(l, 9),
        position: [rdf(l, 13), rdf(l, 14), rdf(l, 15)],
        direction: [rdf(l, 16), rdf(l, 17), rdf(l, 18)],
        range: rdf(l, 19),
        falloff: rdf(l, 20),
        attenuation: [rdf(l, 21), rdf(l, 22), rdf(l, 23)],
        theta: rdf(l, 24),
        phi: rdf(l, 25),
    };
    with_device(|d| d.set_light(index, &light));
    D3D_OK
}
fn dev_light_enable(_this: u32, index: u32, on: u32) -> i32 {
    with_device(|d| d.light_enable(index, on != 0));
    D3D_OK
}
/// `SetClipPlane`: not modelled by the seam (platform.md §4.1); accepted.
fn dev_set_clip_plane(_this: u32, _index: u32, _plane: u32) -> i32 {
    D3D_OK
}
fn dev_set_render_state(_this: u32, state: u32, value: u32) -> i32 {
    with_device(|d| d.set_render_state(state, value));
    D3D_OK
}
fn dev_get_render_state(_this: u32, state: u32, out: GPtr<u32>) -> i32 {
    out.write(with_device(|d| d.render_state(state)).unwrap_or(0));
    D3D_OK
}
fn dev_set_texture(_this: u32, stage: u32, tex: u32) -> i32 {
    let id = if tex == 0 {
        None
    } else {
        with_res(tex, |r| match r {
            Res::Texture { id, .. } => Some(*id),
            _ => None,
        })
        .flatten()
    };
    with_device(|d| d.set_texture(stage, id));
    D3D_OK
}
fn dev_set_texture_stage_state(_this: u32, stage: u32, ty: u32, value: u32) -> i32 {
    with_device(|d| d.set_texture_stage_state(stage, ty, value));
    D3D_OK
}
fn dev_set_sampler_state(_this: u32, sampler: u32, ty: u32, value: u32) -> i32 {
    with_device(|d| d.set_sampler_state(sampler, ty, value));
    D3D_OK
}
/// Palettes (`SetPaletteEntries`, `SetCurrentTexturePalette`): P8 textures are not available on
/// the seam; accepted.
fn dev_palette_ok(_this: u32, _a: u32, _b: u32) -> i32 {
    D3D_OK
}
fn dev_set_current_texture_palette(_this: u32, _a: u32) -> i32 {
    D3D_OK
}
fn dev_draw_primitive(_this: u32, ty: u32, start: u32, count: u32) -> i32 {
    let Some(p) = prim(ty) else { return D3DERR_INVALIDCALL };
    hr(with_device(|d| d.draw_primitive(p, start, count)))
}
fn dev_draw_indexed_primitive(_this: u32, ty: u32, base: i32, min: u32, num: u32, start: u32, count: u32) -> i32 {
    let Some(p) = prim(ty) else { return D3DERR_INVALIDCALL };
    hr(with_device(|d| d.draw_indexed_primitive(p, base, min, num, start, count)))
}
fn dev_draw_primitive_up(_this: u32, ty: u32, count: u32, data: GPtr<u8>, stride: u32) -> i32 {
    let Some(p) = prim(ty) else { return D3DERR_INVALIDCALL };
    let v = bytes(data, (p.vertex_count(count) * stride) as usize);
    hr(with_device(|d| d.draw_primitive_up(p, count, &v, stride)))
}
#[allow(clippy::too_many_arguments)]
fn dev_draw_indexed_primitive_up(_this: u32, ty: u32, min: u32, num: u32, count: u32, idx: GPtr<u8>, format: u32, data: GPtr<u8>, stride: u32) -> i32 {
    let Some(p) = prim(ty) else { return D3DERR_INVALIDCALL };
    let isz = if format == gpu::fmt::INDEX32 { 4 } else { 2 };
    let ib = bytes(idx, (p.vertex_count(count) * isz) as usize);
    let vb = bytes(data, ((min + num) * stride) as usize);
    hr(with_device(|d| d.draw_indexed_primitive_up(p, min, num, count, &ib, format, &vb, stride)))
}
fn dev_create_vertex_declaration(_this: u32, elems: GPtr<u8>, out: GPtr<u32>) -> i32 {
    let mut v = Vec::new();
    let mut raw = Vec::new();
    for i in 0..64 {
        let e = elems.byte_add(8 * i);
        let stream = e.cast::<u16>().read();
        if stream == 0xFF {
            break;
        }
        raw.push(core::array::from_fn(|k| e.byte_add(k as isize).read()));
        v.push(gpu::VertexElement {
            stream,
            offset: e.byte_add(2).cast::<u16>().read(),
            decl_type: e.byte_add(4).read(),
            method: e.byte_add(5).read(),
            usage: e.byte_add(6).read(),
            usage_index: e.byte_add(7).read(),
        });
    }
    match with_device(|d| d.create_vertex_declaration(&v)) {
        Some(Ok(id)) => {
            out.write(new_object(DECL, Res::Decl { id, elems: raw }));
            D3D_OK
        }
        _ => D3DERR_INVALIDCALL,
    }
}
/// `IDirect3DVertexDeclaration9::GetDeclaration(D3DVERTEXELEMENT9* elems, UINT* num)`: the elements given at creation
/// plus `D3DDECL_END()`; `*num` = their count including the end marker (only the count when `elems` is NULL).
fn decl_get_declaration(obj: u32, elems: GPtr<u8>, num: GPtr<u32>) -> i32 {
    let Some(raw) = with_res(obj, |r| match r {
        Res::Decl { elems, .. } => Some(elems.clone()),
        _ => None,
    })
    .flatten() else {
        return D3DERR_INVALIDCALL;
    };
    if !num.is_null() {
        num.write(raw.len() as u32 + 1);
    }
    if !elems.is_null() {
        let end: [u8; 8] = [0xFF, 0, 0, 0, 0x11, 0, 0, 0];
        for (i, e) in raw.iter().chain(core::iter::once(&end)).enumerate() {
            for (k, b) in e.iter().enumerate() {
                elems.byte_add((8 * i + k) as isize).write(*b);
            }
        }
    }
    D3D_OK
}
fn dev_set_vertex_declaration(_this: u32, decl: u32) -> i32 {
    let id = if decl == 0 {
        None
    } else {
        with_res(decl, |r| match r {
            Res::Decl { id, .. } => Some(*id),
            _ => None,
        })
        .flatten()
    };
    with_device(|d| d.set_vertex_declaration(id));
    D3D_OK
}
fn dev_set_fvf(_this: u32, fvf: u32) -> i32 {
    with_device(|d| d.set_fvf(fvf));
    D3D_OK
}
fn dev_set_stream_source(_this: u32, stream: u32, vb: u32, offset: u32, stride: u32) -> i32 {
    let id = if vb == 0 {
        None
    } else {
        with_res(vb, |r| match r {
            Res::VertexBuffer { id, .. } => Some(*id),
            _ => None,
        })
        .flatten()
    };
    with_device(|d| d.set_stream_source(stream, id, offset, stride));
    D3D_OK
}
fn dev_set_indices(_this: u32, ib: u32) -> i32 {
    let id = if ib == 0 {
        None
    } else {
        with_res(ib, |r| match r {
            Res::IndexBuffer { id, .. } => Some(*id),
            _ => None,
        })
        .flatten()
    };
    with_device(|d| d.set_indices(id));
    D3D_OK
}

// ------------------------------------------------------------------ resources ----------------

fn res_add_ref(obj: u32) -> u32 {
    let h = handle(obj);
    with_store(|s| {
        s.res.get_mut(&h).map_or(0, |e| {
            e.1 += 1;
            e.1
        })
    })
}
/// `Release`: at 0 references the seam resource is released (the object block is leaked).
fn res_release(obj: u32) -> u32 {
    let h = handle(obj);
    let gone = with_store(|s| {
        let n = s.res.get_mut(&h).map_or(0, |e| {
            e.1 = e.1.saturating_sub(1);
            e.1
        });
        // the swap chain's surfaces live as long as the device (D3D keeps its own reference)
        let swap = matches!(s.res.get(&h), Some((Res::Surface { tex: None, own: None, sys, .. }, _)) if sys.is_empty());
        if n == 0 && !swap { s.res.remove(&h).map(|e| e.0) } else { None }
    });
    match gone {
        Some(Res::Texture { id, .. }) | Some(Res::Surface { own: Some(id), .. }) => {
            with_device(|d| d.release_texture(id));
        }
        Some(Res::VertexBuffer { id, .. }) => {
            with_device(|d| d.release_vertex_buffer(id));
        }
        Some(Res::IndexBuffer { id, .. }) => {
            with_device(|d| d.release_index_buffer(id));
        }
        Some(Res::Decl { id, .. }) => {
            with_device(|d| d.release_vertex_declaration(id));
        }
        _ => {}
    }
    0
}
fn level_size(format: u32, w: u32, h: u32, level: u32) -> (u32, u32, u32, u32) {
    let (lw, lh) = ((w >> level).max(1), (h >> level).max(1));
    let pitch = match gpu::fmt::block_bytes(format) {
        Some(b) => lw.div_ceil(4) * b,
        None => lw * gpu::fmt::bytes_per_pixel(format).unwrap_or(4),
    };
    let rows = if gpu::fmt::block_bytes(format).is_some() { lh.div_ceil(4) } else { lh };
    (lw, lh, pitch, pitch * rows)
}
fn tex_get_level_count(obj: u32) -> u32 {
    with_res(obj, |r| match r {
        Res::Texture { levels, .. } => *levels,
        _ => 0,
    })
    .unwrap_or(0)
}
/// `D3DSURFACE_DESC {Format, Type, Usage, Pool, MultiSampleType, MultiSampleQuality, Width, Height}`.
fn write_desc(out: GPtr<u8>, format: u32, ty: u32, usage: u32, pool: u32, w: u32, h: u32) {
    for (i, v) in [format, ty, usage, pool, 0, 0, w, h].iter().enumerate() {
        out.byte_add(4 * i as isize).cast::<u32>().write(*v);
    }
}
fn tex_get_level_desc(obj: u32, level: u32, out: GPtr<u8>) -> i32 {
    let d = with_res(obj, |r| match r {
        Res::Texture { w, h, format, usage, pool, .. } => Some((*w, *h, *format, *usage, *pool)),
        _ => None,
    })
    .flatten();
    match d {
        Some((w, h, f, usage, pool)) => {
            let (lw, lh, _, _) = level_size(f, w, h, level);
            write_desc(out, f, 1, usage, pool, lw, lh);
            D3D_OK
        }
        None => D3DERR_INVALIDCALL,
    }
}
fn tex_get_surface_level(obj: u32, level: u32, out: GPtr<u32>) -> i32 {
    let d = with_res(obj, |r| match r {
        Res::Texture { w, h, format, usage, pool, .. } => Some((*w, *h, *format, *usage, *pool)),
        _ => None,
    })
    .flatten();
    let Some((w, h, f, usage, pool)) = d else { return D3DERR_INVALIDCALL };
    let (lw, lh, _, _) = level_size(f, w, h, level);
    res_add_ref(obj);
    out.write(new_object(SURFACE, Res::Surface { tex: Some(obj), level, w: lw, h: lh, format: f, usage, pool, own: None, sys: Vec::new() }));
    D3D_OK
}
/// `LockRect(level, D3DLOCKED_RECT*, rect, flags)`: the whole level's CPU copy (a sub-rectangle
/// lock points into it).
fn tex_lock_rect(obj: u32, level: u32, out: GPtr<u8>, rect: GPtr<u32>, _flags: u32) -> i32 {
    let r = with_res(obj, |r| match r {
        Res::Texture { w, h, format, locked, .. } => {
            let (_, _, pitch, size) = level_size(*format, *w, *h, level);
            let buf = locked.entry(level).or_insert_with(|| vec![0u8; size as usize]);
            Some((buf.as_mut_ptr() as u32, pitch, *format))
        }
        _ => None,
    })
    .flatten();
    let Some((p, pitch, format)) = r else { return D3DERR_INVALIDCALL };
    let mut off = 0;
    if !rect.is_null() {
        let (x, y) = (rect.read(), rect.wrapping_index(1).read());
        off = match gpu::fmt::block_bytes(format) {
            Some(b) => (y / 4) * pitch + (x / 4) * b,
            None => y * pitch + x * gpu::fmt::bytes_per_pixel(format).unwrap_or(4),
        };
    }
    out.cast::<u32>().write(pitch);
    out.byte_add(4).cast::<u32>().write(p + off);
    D3D_OK
}
fn tex_unlock_rect(obj: u32, level: u32) -> i32 {
    let r = with_res(obj, |r| match r {
        Res::Texture { id, w, h, format, locked, .. } => {
            let (_, _, pitch, _) = level_size(*format, *w, *h, level);
            // the level's CPU copy stays: a later read lock (RwRasterLock for reading, the clothes
            // builder's texture blending) sees the uploaded pixels, as a managed D3D9 texture's
            locked.get(&level).map(|data| (*id, pitch, data.clone()))
        }
        _ => None,
    })
    .flatten();
    match r {
        Some((id, pitch, data)) => hr(with_device(|d| d.update_texture(id, 0, level, None, &data, pitch))),
        None => D3DERR_INVALIDCALL,
    }
}
fn surf_get_desc(obj: u32, out: GPtr<u8>) -> i32 {
    let d = with_res(obj, |r| match r {
        Res::Surface { w, h, format, usage, pool, .. } => Some((*w, *h, *format, *usage, *pool)),
        _ => None,
    })
    .flatten();
    match d {
        Some((w, h, f, usage, pool)) => {
            write_desc(out, f, 1, usage, pool, w, h);
            D3D_OK
        }
        None => D3DERR_INVALIDCALL,
    }
}
fn surf_lock_rect(obj: u32, out: GPtr<u8>, rect: GPtr<u32>, flags: u32) -> i32 {
    let sys = with_res(obj, |r| match r {
        Res::Surface { sys, w, format, .. } if !sys.is_empty() => Some((sys.as_mut_ptr() as u32, sys_pitch(*format, *w), *format)),
        _ => None,
    })
    .flatten();
    if let Some((p, pitch, format)) = sys {
        let mut off = 0;
        if !rect.is_null() {
            off = rect.wrapping_index(1).read() * pitch + rect.read() * gpu::fmt::bytes_per_pixel(format).unwrap_or(4);
        }
        out.cast::<u32>().write(pitch);
        out.byte_add(4).cast::<u32>().write(p + off);
        return D3D_OK;
    }
    let t = with_res(obj, |r| match r {
        Res::Surface { tex, level, .. } => Some((*tex, *level)),
        _ => None,
    })
    .flatten();
    match t {
        Some((Some(tex), level)) => tex_lock_rect(tex, level, out, rect, flags),
        _ => D3DERR_INVALIDCALL,
    }
}
fn surf_unlock_rect(obj: u32) -> i32 {
    let t = with_res(obj, |r| match r {
        Res::Surface { tex, level, sys, .. } => Some((*tex, *level, !sys.is_empty())),
        _ => None,
    })
    .flatten();
    match t {
        Some((_, _, true)) => D3D_OK,
        Some((Some(tex), level, false)) => tex_unlock_rect(tex, level),
        _ => D3DERR_INVALIDCALL,
    }
}
/// `Lock(offset, size, void**, flags)` of a vertex or index buffer (size 0 = to the end).
fn buf_lock(obj: u32, offset: u32, size: u32, out: GPtr<u32>, _flags: u32) -> i32 {
    let p = with_res(obj, |r| match r {
        Res::VertexBuffer { data, lock, .. } | Res::IndexBuffer { data, lock, .. } => {
            let len = data.len() as u32;
            let size = if size == 0 { len.saturating_sub(offset) } else { size };
            *lock = (offset.min(len), size.min(len.saturating_sub(offset)));
            Some(data.as_mut_ptr() as u32 + lock.0)
        }
        _ => None,
    })
    .flatten();
    match p {
        Some(p) => {
            out.write(p);
            D3D_OK
        }
        None => D3DERR_INVALIDCALL,
    }
}
fn buf_unlock(obj: u32) -> i32 {
    enum Up {
        V(gpu::VertexBufferId, u32, Vec<u8>),
        I(gpu::IndexBufferId, u32, Vec<u8>),
    }
    let up = with_res(obj, |r| match r {
        Res::VertexBuffer { id, data, lock } => {
            Some(Up::V(*id, lock.0, data[lock.0 as usize..(lock.0 + lock.1) as usize].to_vec()))
        }
        Res::IndexBuffer { id, data, lock, .. } => {
            Some(Up::I(*id, lock.0, data[lock.0 as usize..(lock.0 + lock.1) as usize].to_vec()))
        }
        _ => None,
    })
    .flatten();
    match up {
        Some(Up::V(id, o, d)) => hr(with_device(|dev| dev.update_vertex_buffer(id, o, &d))),
        Some(Up::I(id, o, d)) => hr(with_device(|dev| dev.update_index_buffer(id, o, &d))),
        None => D3DERR_INVALIDCALL,
    }
}
/// `D3DVERTEXBUFFER_DESC`/`D3DINDEXBUFFER_DESC` `{Format, Type, Usage, Pool, Size, (FVF)}`.
fn buf_get_desc(obj: u32, out: GPtr<u32>) -> i32 {
    let d = with_res(obj, |r| match r {
        Res::VertexBuffer { data, .. } => Some((0x64u32, 3u32, data.len() as u32)),
        Res::IndexBuffer { data, format, .. } => Some((*format, 4, data.len() as u32)),
        _ => None,
    })
    .flatten();
    match d {
        Some((f, t, n)) => {
            for (i, v) in [f, t, 0, 0, n].iter().enumerate() {
                out.wrapping_index(i as i32).write(*v);
            }
            D3D_OK
        }
        None => D3DERR_INVALIDCALL,
    }
}

macro_rules! entries {
    ($( $addr:expr, $name:literal => $f:path : fn($($a:ident : $t:ty),*) -> $r:ty ;)*) => {
        /// `(address, name, native stdcall entry)` of every implemented method.
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
                v.push(($addr, $name, w as *const () as usize));
            })*
            v
        }
    };
}

fn dev(slot: usize) -> u32 {
    crate::dx::trap(3, slot)
}

entries! {
    dev(3), "IDirect3DDevice9::TestCooperativeLevel" => dev_test_cooperative_level: fn(this: u32) -> i32;
    dev(4), "IDirect3DDevice9::GetAvailableTextureMem" => dev_get_available_texture_mem: fn(this: u32) -> u32;
    dev(5), "IDirect3DDevice9::EvictManagedResources" => dev_evict_managed_resources: fn(this: u32) -> i32;
    dev(16), "IDirect3DDevice9::Reset" => dev_reset: fn(this: u32, pp: GPtr<u32>) -> i32;
    dev(17), "IDirect3DDevice9::Present" => dev_present: fn(this: u32, src: u32, dst: u32, hwnd: u32, dirty: u32) -> i32;
    dev(25), "IDirect3DDevice9::CreateCubeTexture" => dev_create_cube_texture: fn(this: u32, edge: u32, levels: u32, usage: u32, format: u32, pool: u32, out: GPtr<u32>, shared: u32) -> i32;
    dev(28), "IDirect3DDevice9::CreateRenderTarget" => dev_create_render_target: fn(this: u32, w: u32, h: u32, format: u32, ms: u32, q: u32, lockable: u32, out: GPtr<u32>, shared: u32) -> i32;
    dev(29), "IDirect3DDevice9::CreateDepthStencilSurface" => dev_create_depth_stencil_surface: fn(this: u32, w: u32, h: u32, format: u32, ms: u32, q: u32, discard: u32, out: GPtr<u32>, shared: u32) -> i32;
    dev(30), "IDirect3DDevice9::UpdateSurface" => dev_update_surface: fn(this: u32, src: u32, src_rect: GPtr<i32>, dst: u32, pt: GPtr<i32>) -> i32;
    dev(32), "IDirect3DDevice9::GetRenderTargetData" => dev_get_render_target_data: fn(this: u32, rt: u32, dst: u32) -> i32;
    dev(34), "IDirect3DDevice9::StretchRect" => dev_stretch_rect: fn(this: u32, src: u32, src_rect: GPtr<i32>, dst: u32, dst_rect: GPtr<i32>, filter: u32) -> i32;
    dev(36), "IDirect3DDevice9::CreateOffscreenPlainSurface" => dev_create_offscreen_plain_surface: fn(this: u32, w: u32, h: u32, format: u32, pool: u32, out: GPtr<u32>, shared: u32) -> i32;
    dev(18), "IDirect3DDevice9::GetBackBuffer" => dev_get_back_buffer: fn(this: u32, swap: u32, index: u32, ty: u32, out: GPtr<u32>) -> i32;
    dev(21), "IDirect3DDevice9::SetGammaRamp" => dev_set_gamma_ramp: fn(this: u32, swap: u32, flags: u32, ramp: GPtr<u16>) -> ();
    dev(22), "IDirect3DDevice9::GetGammaRamp" => dev_get_gamma_ramp: fn(this: u32, swap: u32, ramp: GPtr<u16>) -> ();
    dev(23), "IDirect3DDevice9::CreateTexture" => dev_create_texture: fn(this: u32, w: u32, h: u32, levels: u32, usage: u32, format: u32, pool: u32, out: GPtr<u32>, shared: u32) -> i32;
    dev(26), "IDirect3DDevice9::CreateVertexBuffer" => dev_create_vertex_buffer: fn(this: u32, len: u32, usage: u32, fvf: u32, pool: u32, out: GPtr<u32>, shared: u32) -> i32;
    dev(27), "IDirect3DDevice9::CreateIndexBuffer" => dev_create_index_buffer: fn(this: u32, len: u32, usage: u32, format: u32, pool: u32, out: GPtr<u32>, shared: u32) -> i32;
    dev(37), "IDirect3DDevice9::SetRenderTarget" => dev_set_render_target: fn(this: u32, index: u32, surface: u32) -> i32;
    dev(38), "IDirect3DDevice9::GetRenderTarget" => dev_get_render_target: fn(this: u32, index: u32, out: GPtr<u32>) -> i32;
    dev(39), "IDirect3DDevice9::SetDepthStencilSurface" => dev_set_depth_stencil_surface: fn(this: u32, surface: u32) -> i32;
    dev(40), "IDirect3DDevice9::GetDepthStencilSurface" => dev_get_depth_stencil_surface: fn(this: u32, out: GPtr<u32>) -> i32;
    dev(41), "IDirect3DDevice9::BeginScene" => dev_begin_scene: fn(this: u32) -> i32;
    dev(42), "IDirect3DDevice9::EndScene" => dev_end_scene: fn(this: u32) -> i32;
    dev(43), "IDirect3DDevice9::Clear" => dev_clear: fn(this: u32, count: u32, rects: u32, flags: u32, color: u32, z: f32, stencil: u32) -> i32;
    dev(44), "IDirect3DDevice9::SetTransform" => dev_set_transform: fn(this: u32, state: u32, m: GPtr<u8>) -> i32;
    dev(47), "IDirect3DDevice9::SetViewport" => dev_set_viewport: fn(this: u32, vp: GPtr<u8>) -> i32;
    dev(49), "IDirect3DDevice9::SetMaterial" => dev_set_material: fn(this: u32, m: GPtr<u8>) -> i32;
    dev(51), "IDirect3DDevice9::SetLight" => dev_set_light: fn(this: u32, index: u32, l: GPtr<u8>) -> i32;
    dev(53), "IDirect3DDevice9::LightEnable" => dev_light_enable: fn(this: u32, index: u32, on: u32) -> i32;
    dev(55), "IDirect3DDevice9::SetClipPlane" => dev_set_clip_plane: fn(this: u32, index: u32, plane: u32) -> i32;
    dev(57), "IDirect3DDevice9::SetRenderState" => dev_set_render_state: fn(this: u32, state: u32, value: u32) -> i32;
    dev(58), "IDirect3DDevice9::GetRenderState" => dev_get_render_state: fn(this: u32, state: u32, out: GPtr<u32>) -> i32;
    dev(65), "IDirect3DDevice9::SetTexture" => dev_set_texture: fn(this: u32, stage: u32, tex: u32) -> i32;
    dev(67), "IDirect3DDevice9::SetTextureStageState" => dev_set_texture_stage_state: fn(this: u32, stage: u32, ty: u32, value: u32) -> i32;
    dev(69), "IDirect3DDevice9::SetSamplerState" => dev_set_sampler_state: fn(this: u32, sampler: u32, ty: u32, value: u32) -> i32;
    dev(71), "IDirect3DDevice9::SetPaletteEntries" => dev_palette_ok: fn(this: u32, a: u32, b: u32) -> i32;
    dev(73), "IDirect3DDevice9::SetCurrentTexturePalette" => dev_set_current_texture_palette: fn(this: u32, a: u32) -> i32;
    dev(81), "IDirect3DDevice9::DrawPrimitive" => dev_draw_primitive: fn(this: u32, ty: u32, start: u32, count: u32) -> i32;
    dev(82), "IDirect3DDevice9::DrawIndexedPrimitive" => dev_draw_indexed_primitive: fn(this: u32, ty: u32, base: i32, min: u32, num: u32, start: u32, count: u32) -> i32;
    dev(83), "IDirect3DDevice9::DrawPrimitiveUP" => dev_draw_primitive_up: fn(this: u32, ty: u32, count: u32, data: GPtr<u8>, stride: u32) -> i32;
    dev(84), "IDirect3DDevice9::DrawIndexedPrimitiveUP" => dev_draw_indexed_primitive_up: fn(this: u32, ty: u32, min: u32, num: u32, count: u32, idx: GPtr<u8>, format: u32, data: GPtr<u8>, stride: u32) -> i32;
    dev(86), "IDirect3DDevice9::CreateVertexDeclaration" => dev_create_vertex_declaration: fn(this: u32, elems: GPtr<u8>, out: GPtr<u32>) -> i32;
    dev(87), "IDirect3DDevice9::SetVertexDeclaration" => dev_set_vertex_declaration: fn(this: u32, decl: u32) -> i32;
    dev(89), "IDirect3DDevice9::SetFVF" => dev_set_fvf: fn(this: u32, fvf: u32) -> i32;
    dev(100), "IDirect3DDevice9::SetStreamSource" => dev_set_stream_source: fn(this: u32, stream: u32, vb: u32, offset: u32, stride: u32) -> i32;
    dev(104), "IDirect3DDevice9::SetIndices" => dev_set_indices: fn(this: u32, ib: u32) -> i32;
    trap(TEXTURE, 1), "IDirect3DTexture9::AddRef" => res_add_ref: fn(obj: u32) -> u32;
    trap(TEXTURE, 2), "IDirect3DTexture9::Release" => res_release: fn(obj: u32) -> u32;
    trap(TEXTURE, 13), "IDirect3DTexture9::GetLevelCount" => tex_get_level_count: fn(obj: u32) -> u32;
    trap(TEXTURE, 17), "IDirect3DTexture9::GetLevelDesc" => tex_get_level_desc: fn(obj: u32, level: u32, out: GPtr<u8>) -> i32;
    trap(TEXTURE, 18), "IDirect3DTexture9::GetSurfaceLevel" => tex_get_surface_level: fn(obj: u32, level: u32, out: GPtr<u32>) -> i32;
    trap(TEXTURE, 19), "IDirect3DTexture9::LockRect" => tex_lock_rect: fn(obj: u32, level: u32, out: GPtr<u8>, rect: GPtr<u32>, flags: u32) -> i32;
    trap(TEXTURE, 20), "IDirect3DTexture9::UnlockRect" => tex_unlock_rect: fn(obj: u32, level: u32) -> i32;
    trap(SURFACE, 1), "IDirect3DSurface9::AddRef" => res_add_ref: fn(obj: u32) -> u32;
    trap(SURFACE, 2), "IDirect3DSurface9::Release" => res_release: fn(obj: u32) -> u32;
    trap(SURFACE, 12), "IDirect3DSurface9::GetDesc" => surf_get_desc: fn(obj: u32, out: GPtr<u8>) -> i32;
    trap(SURFACE, 13), "IDirect3DSurface9::LockRect" => surf_lock_rect: fn(obj: u32, out: GPtr<u8>, rect: GPtr<u32>, flags: u32) -> i32;
    trap(SURFACE, 14), "IDirect3DSurface9::UnlockRect" => surf_unlock_rect: fn(obj: u32) -> i32;
    trap(BUFFER, 1), "IDirect3DBuffer9::AddRef" => res_add_ref: fn(obj: u32) -> u32;
    trap(BUFFER, 2), "IDirect3DBuffer9::Release" => res_release: fn(obj: u32) -> u32;
    trap(BUFFER, 11), "IDirect3DBuffer9::Lock" => buf_lock: fn(obj: u32, offset: u32, size: u32, out: GPtr<u32>, flags: u32) -> i32;
    trap(BUFFER, 12), "IDirect3DBuffer9::Unlock" => buf_unlock: fn(obj: u32) -> i32;
    trap(BUFFER, 13), "IDirect3DBuffer9::GetDesc" => buf_get_desc: fn(obj: u32, out: GPtr<u32>) -> i32;
    trap(DECL, 1), "IDirect3DVertexDeclaration9::AddRef" => res_add_ref: fn(obj: u32) -> u32;
    trap(DECL, 2), "IDirect3DVertexDeclaration9::Release" => res_release: fn(obj: u32) -> u32;
    trap(DECL, 4), "IDirect3DVertexDeclaration9::GetDeclaration" => decl_get_declaration: fn(obj: u32, elems: GPtr<u8>, num: GPtr<u32>) -> i32;
}
