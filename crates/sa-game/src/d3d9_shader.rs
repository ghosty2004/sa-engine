//! Native D3D9 shader objects and the statically linked `D3DXAssembleShader` (docs/arch/gpu.md).
//!
//! * `IDirect3DDevice9::CreateVertexShader`/`CreatePixelShader` copy the token stream out of
//!   game memory (through its end token, skipping comment payloads) and create the shader on the
//!   seam (`D3d9Device::create_*_shader`, translated to WGSL by `sa_rw_wgpu::sm`); the shader
//!   object is a heap object `{vtable, handle}` (`AddRef`, `Release`, `GetFunction`).
//!   `Set`/`Get` of shaders and of float/int/bool constants (both stages) go to the seam; the
//!   constants are mirrored here for the `Get…Constant…` methods.
//! * `D3DXAssembleShader` (0x7652AE, D3DX9 linked into gta_sa.exe; called only by RW's
//!   `_rpD3D9VertexShaderGenerate` 0x75F0B0 on the text it prints) assembles with
//!   `sa_rw_wgpu::sm::asm` into an `ID3DXBuffer` object (`GetBufferPointer`, `GetBufferSize`,
//!   `Release`). Defines and include handlers are not supported (the game passes none).
//!
//! The device only reaches the shader path when the caps report shader support, which
//! [`shaders_enabled`] gates (`SA_GAME_SHADERS=1`; off by default until RW's generator is ported).

// COM method signatures are dictated by D3D9.
#![allow(clippy::too_many_arguments)]

use std::collections::HashMap;
use std::sync::Mutex;

use sa_core::mem::GPtr;
use sa_platform::gpu::{self, D3d9Device};

use crate::d3d9::with_device;

const D3D_OK: i32 = 0;
const D3DERR_INVALIDCALL: i32 = 0x8876_086C_u32 as i32;
const E_FAIL: i32 = 0x8000_4005_u32 as i32;
const E_NOINTERFACE: i32 = 0x8000_4002_u32 as i32;

/// The page holding the vtables of this module's objects.
const PAGE: usize = 0x0003_4000;
/// Method traps: `TRAP + 0x400 * class + 4 * slot`.
const TRAP: u32 = 0x0008_0000;

const SHADER_METHODS: [&str; 5] = ["QueryInterface", "AddRef", "Release", "GetDevice", "GetFunction"];
const BUFFER_METHODS: [&str; 5] = ["QueryInterface", "AddRef", "Release", "GetBufferPointer", "GetBufferSize"];
const CLASSES: [(&str, &[&str]); 3] =
    [("IDirect3DVertexShader9", &SHADER_METHODS), ("IDirect3DPixelShader9", &SHADER_METHODS), ("ID3DXBuffer", &BUFFER_METHODS)];
const VSHADER: usize = 0;
const PSHADER: usize = 1;
const XBUFFER: usize = 2;

fn vtable(class: usize) -> u32 {
    (PAGE + 0x100 * class) as u32
}
fn trap(class: usize, slot: usize) -> u32 {
    TRAP + 0x400 * class as u32 + 4 * slot as u32
}

enum Obj {
    /// A shader: its seam id and token stream (`GetFunction`).
    Shader { id: gpu::ShaderId, tokens: Vec<u32> },
    /// An `ID3DXBuffer`: the bytes live in `data` (stable address while the object lives).
    Buffer { data: Box<[u32]>, size: u32 },
}

struct Store {
    next: u32,
    objs: HashMap<u32, (Obj, u32)>,
    /// The current shader objects (for `GetVertexShader`/`GetPixelShader`).
    cur_vs: u32,
    cur_ps: u32,
    vf: Vec<[f32; 4]>,
    pf: Vec<[f32; 4]>,
    vi: [[i32; 4]; 16],
    pi: [[i32; 4]; 16],
    vb: [bool; 16],
    pb: [bool; 16],
}

static STORE: Mutex<Option<Store>> = Mutex::new(None);

fn with_store<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    let mut s = STORE.lock().unwrap_or_else(|e| e.into_inner());
    f(s.get_or_insert_with(|| Store {
        next: 1,
        objs: HashMap::new(),
        cur_vs: 0,
        cur_ps: 0,
        vf: vec![[0.0; 4]; 256],
        pf: vec![[0.0; 4]; 224],
        vi: [[0; 4]; 16],
        pi: [[0; 4]; 16],
        vb: [false; 16],
        pb: [false; 16],
    }))
}

fn new_object(class: usize, obj: Obj) -> u32 {
    let h = with_store(|s| {
        let h = s.next;
        s.next += 1;
        s.objs.insert(h, (obj, 1));
        h
    });
    let o: &'static mut [u32; 2] = Box::leak(Box::new([vtable(class), h]));
    o.as_ptr() as u32
}
fn handle(obj: u32) -> u32 {
    GPtr::<u32>::from_addr(obj as usize).wrapping_index(1).read()
}

/// Whether the native device reports (and so the game uses) vertex/pixel shaders:
/// `SA_GAME_SHADERS=1`.
pub fn shaders_enabled() -> bool {
    crate::env::var_os("SA_GAME_SHADERS").is_some_and(|v| v == "1")
}

/// Sets the shader fields of the native `D3DCAPS9` at `out` when [`shaders_enabled`]: vs_2_0
/// with 256 constants, ps_2_0, `PixelShader1xMaxValue` 8.0.
pub fn patch_caps9(out: GPtr<u8>) {
    if !shaders_enabled() {
        return;
    }
    for (o, v) in [(0xC4, 0xFFFE_0200u32), (0xC8, 256), (0xCC, 0xFFFF_0200), (0xD0, 8.0f32.to_bits())] {
        out.byte_add(o).cast::<u32>().write(v);
    }
}

/// Maps the vtable page.
#[allow(unsafe_code)]
pub fn map() -> Result<(), String> {
    crate::arena::map_fixed(PAGE, 0x1000).map_err(|e| format!("{}: {e}", format!("cannot map the D3D9 shader object page at {PAGE:#x}")))?;
    for (c, (_, methods)) in CLASSES.iter().enumerate() {
        for i in 0..methods.len() {
            GPtr::<u32>::from_addr(vtable(c) as usize + 4 * i).write(trap(c, i));
        }
    }
    Ok(())
}

/// Every method trap's name.
pub fn names() -> Vec<(u32, &'static str)> {
    let mut v = Vec::new();
    for (c, (class, methods)) in CLASSES.iter().enumerate() {
        for (i, m) in methods.iter().enumerate() {
            let name: &'static str = Box::leak(format!("{class}::{m}").into_boxed_str());
            v.push((trap(c, i), name));
        }
    }
    v
}

// ------------------------------------------------------------------ shaders ------------------

/// The token stream at `code` (through the end token).
fn read_tokens(code: u32) -> Option<Vec<u32>> {
    if code == 0 {
        return None;
    }
    let p = GPtr::<u32>::from_addr(code as usize);
    let n = sa_rw_wgpu::sm::parse::token_length(|i| Some(p.wrapping_index(i as i32).read()))?;
    Some((0..n).map(|i| p.wrapping_index(i as i32).read()).collect())
}

fn create_shader(code: u32, out: GPtr<u32>, pixel: bool) -> i32 {
    let Some(tokens) = read_tokens(code) else { return D3DERR_INVALIDCALL };
    let r = with_device(|d| if pixel { d.create_pixel_shader(&tokens) } else { d.create_vertex_shader(&tokens) });
    match r {
        Some(Ok(id)) => {
            out.write(new_object(if pixel { PSHADER } else { VSHADER }, Obj::Shader { id, tokens }));
            D3D_OK
        }
        Some(Err(e)) => {
            eprintln!("sa-game: Create{}Shader: {e:?}", if pixel { "Pixel" } else { "Vertex" });
            D3DERR_INVALIDCALL
        }
        None => D3DERR_INVALIDCALL,
    }
}

fn shader_id(obj: u32) -> Option<gpu::ShaderId> {
    if obj == 0 {
        return None;
    }
    let h = handle(obj);
    with_store(|s| match s.objs.get(&h) {
        Some((Obj::Shader { id, .. }, _)) => Some(*id),
        _ => None,
    })
}

fn dev_create_vertex_shader(_this: u32, code: u32, out: GPtr<u32>) -> i32 {
    create_shader(code, out, false)
}
fn dev_create_pixel_shader(_this: u32, code: u32, out: GPtr<u32>) -> i32 {
    create_shader(code, out, true)
}
fn dev_set_vertex_shader(_this: u32, sh: u32) -> i32 {
    let id = shader_id(sh);
    if sh != 0 && id.is_none() {
        return D3DERR_INVALIDCALL;
    }
    with_store(|s| s.cur_vs = sh);
    with_device(|d| d.set_vertex_shader(id));
    D3D_OK
}
fn dev_set_pixel_shader(_this: u32, sh: u32) -> i32 {
    let id = shader_id(sh);
    if sh != 0 && id.is_none() {
        return D3DERR_INVALIDCALL;
    }
    with_store(|s| s.cur_ps = sh);
    with_device(|d| d.set_pixel_shader(id));
    D3D_OK
}
fn dev_get_vertex_shader(_this: u32, out: GPtr<u32>) -> i32 {
    let sh = with_store(|s| s.cur_vs);
    if sh != 0 {
        obj_add_ref(sh);
    }
    out.write(sh);
    D3D_OK
}
fn dev_get_pixel_shader(_this: u32, out: GPtr<u32>) -> i32 {
    let sh = with_store(|s| s.cur_ps);
    if sh != 0 {
        obj_add_ref(sh);
    }
    out.write(sh);
    D3D_OK
}

fn read_f(data: u32, n: u32) -> Vec<[f32; 4]> {
    let p = GPtr::<f32>::from_addr(data as usize);
    (0..n as i32).map(|r| [0, 1, 2, 3].map(|c| p.wrapping_index(4 * r + c).read())).collect()
}
fn read_i(data: u32, n: u32) -> Vec<[i32; 4]> {
    let p = GPtr::<i32>::from_addr(data as usize);
    (0..n as i32).map(|r| [0, 1, 2, 3].map(|c| p.wrapping_index(4 * r + c).read())).collect()
}
fn read_b(data: u32, n: u32) -> Vec<bool> {
    let p = GPtr::<u32>::from_addr(data as usize);
    (0..n as i32).map(|r| p.wrapping_index(r).read() != 0).collect()
}
fn store_regs<T: Copy>(regs: &mut [T], start: u32, v: &[T]) -> bool {
    let end = start as usize + v.len();
    if end > regs.len() {
        return false;
    }
    regs[start as usize..end].copy_from_slice(v);
    true
}
fn load_regs<T: Copy>(regs: &[T], start: u32, n: u32, write: impl Fn(usize, T)) -> i32 {
    let end = start as usize + n as usize;
    if end > regs.len() {
        return D3DERR_INVALIDCALL;
    }
    for (k, v) in regs[start as usize..end].iter().enumerate() {
        write(k, *v);
    }
    D3D_OK
}

fn dev_set_vs_const_f(_this: u32, start: u32, data: u32, n: u32) -> i32 {
    let v = read_f(data, n);
    if !with_store(|s| store_regs(&mut s.vf, start, &v)) {
        return D3DERR_INVALIDCALL;
    }
    with_device(|d| d.set_vertex_shader_constant_f(start, &v));
    D3D_OK
}
fn dev_set_ps_const_f(_this: u32, start: u32, data: u32, n: u32) -> i32 {
    let v = read_f(data, n);
    if !with_store(|s| store_regs(&mut s.pf, start, &v)) {
        return D3DERR_INVALIDCALL;
    }
    with_device(|d| d.set_pixel_shader_constant_f(start, &v));
    D3D_OK
}
fn dev_set_vs_const_i(_this: u32, start: u32, data: u32, n: u32) -> i32 {
    let v = read_i(data, n);
    if !with_store(|s| store_regs(&mut s.vi, start, &v)) {
        return D3DERR_INVALIDCALL;
    }
    with_device(|d| d.set_vertex_shader_constant_i(start, &v));
    D3D_OK
}
fn dev_set_ps_const_i(_this: u32, start: u32, data: u32, n: u32) -> i32 {
    let v = read_i(data, n);
    if !with_store(|s| store_regs(&mut s.pi, start, &v)) {
        return D3DERR_INVALIDCALL;
    }
    with_device(|d| d.set_pixel_shader_constant_i(start, &v));
    D3D_OK
}
fn dev_set_vs_const_b(_this: u32, start: u32, data: u32, n: u32) -> i32 {
    let v = read_b(data, n);
    if !with_store(|s| store_regs(&mut s.vb, start, &v)) {
        return D3DERR_INVALIDCALL;
    }
    with_device(|d| d.set_vertex_shader_constant_b(start, &v));
    D3D_OK
}
fn dev_set_ps_const_b(_this: u32, start: u32, data: u32, n: u32) -> i32 {
    let v = read_b(data, n);
    if !with_store(|s| store_regs(&mut s.pb, start, &v)) {
        return D3DERR_INVALIDCALL;
    }
    with_device(|d| d.set_pixel_shader_constant_b(start, &v));
    D3D_OK
}
fn write_f(out: u32) -> impl Fn(usize, [f32; 4]) {
    move |k, v| {
        for (c, x) in v.iter().enumerate() {
            GPtr::<f32>::from_addr(out as usize).wrapping_index((4 * k + c) as i32).write(*x);
        }
    }
}
fn write_i(out: u32) -> impl Fn(usize, [i32; 4]) {
    move |k, v| {
        for (c, x) in v.iter().enumerate() {
            GPtr::<i32>::from_addr(out as usize).wrapping_index((4 * k + c) as i32).write(*x);
        }
    }
}
fn write_b(out: u32) -> impl Fn(usize, bool) {
    move |k, v| GPtr::<u32>::from_addr(out as usize).wrapping_index(k as i32).write(v as u32)
}
fn dev_get_vs_const_f(_this: u32, start: u32, out: u32, n: u32) -> i32 {
    with_store(|s| load_regs(&s.vf, start, n, write_f(out)))
}
fn dev_get_ps_const_f(_this: u32, start: u32, out: u32, n: u32) -> i32 {
    with_store(|s| load_regs(&s.pf, start, n, write_f(out)))
}
fn dev_get_vs_const_i(_this: u32, start: u32, out: u32, n: u32) -> i32 {
    with_store(|s| load_regs(&s.vi, start, n, write_i(out)))
}
fn dev_get_ps_const_i(_this: u32, start: u32, out: u32, n: u32) -> i32 {
    with_store(|s| load_regs(&s.pi, start, n, write_i(out)))
}
fn dev_get_vs_const_b(_this: u32, start: u32, out: u32, n: u32) -> i32 {
    with_store(|s| load_regs(&s.vb, start, n, write_b(out)))
}
fn dev_get_ps_const_b(_this: u32, start: u32, out: u32, n: u32) -> i32 {
    with_store(|s| load_regs(&s.pb, start, n, write_b(out)))
}

// ------------------------------------------------------------------ objects ------------------

fn obj_query_interface(_obj: u32, _iid: u32, out: GPtr<u32>) -> i32 {
    if !out.is_null() {
        out.write(0);
    }
    E_NOINTERFACE
}
fn obj_add_ref(obj: u32) -> u32 {
    let h = handle(obj);
    with_store(|s| match s.objs.get_mut(&h) {
        Some((_, rc)) => {
            *rc += 1;
            *rc
        }
        None => 0,
    })
}
fn obj_release(obj: u32) -> u32 {
    let h = handle(obj);
    let (left, freed) = with_store(|s| {
        let Some((_, rc)) = s.objs.get_mut(&h) else { return (0, None) };
        *rc -= 1;
        if *rc > 0 {
            return (*rc, None);
        }
        if s.cur_vs == obj {
            s.cur_vs = 0;
        }
        if s.cur_ps == obj {
            s.cur_ps = 0;
        }
        (0, s.objs.remove(&h).map(|o| o.0))
    });
    if let Some(Obj::Shader { id, .. }) = freed {
        with_device(|d| d.release_shader(id));
    }
    // the {vtable, handle} block itself stays allocated (a stale pointer reads a dead handle)
    left
}
fn shader_get_device(_obj: u32, out: GPtr<u32>) -> i32 {
    out.write(crate::dx::DEVICE9 as u32);
    D3D_OK
}
/// `GetFunction(pData, pSizeOfData)`: with `pData` NULL only the size.
fn shader_get_function(obj: u32, data: u32, size: GPtr<u32>) -> i32 {
    let h = handle(obj);
    let tokens = with_store(|s| match s.objs.get(&h) {
        Some((Obj::Shader { tokens, .. }, _)) => Some(tokens.clone()),
        _ => None,
    });
    let Some(tokens) = tokens else { return D3DERR_INVALIDCALL };
    if data != 0 {
        if size.read() < 4 * tokens.len() as u32 {
            return D3DERR_INVALIDCALL;
        }
        for (i, t) in tokens.iter().enumerate() {
            GPtr::<u32>::from_addr(data as usize).wrapping_index(i as i32).write(*t);
        }
    }
    size.write(4 * tokens.len() as u32);
    D3D_OK
}
fn buffer_get_pointer(obj: u32) -> u32 {
    let h = handle(obj);
    with_store(|s| match s.objs.get(&h) {
        Some((Obj::Buffer { data, .. }, _)) => data.as_ptr() as u32,
        _ => 0,
    })
}
fn buffer_get_size(obj: u32) -> u32 {
    let h = handle(obj);
    with_store(|s| match s.objs.get(&h) {
        Some((Obj::Buffer { size, .. }, _)) => *size,
        _ => 0,
    })
}

/// A new `ID3DXBuffer` holding `bytes`.
fn new_buffer(bytes: &[u8]) -> u32 {
    let mut words = vec![0u32; bytes.len().div_ceil(4).max(1)];
    for (i, b) in bytes.iter().enumerate() {
        words[i / 4] |= (*b as u32) << (8 * (i % 4));
    }
    new_object(XBUFFER, Obj::Buffer { data: words.into_boxed_slice(), size: bytes.len() as u32 })
}

/// `D3DXAssembleShader(pSrcData, SrcDataLen, pDefines, pInclude, Flags, ppShader, ppErrorMsgs)`.
fn d3dx_assemble_shader(src: u32, len: u32, _defines: u32, _include: u32, _flags: u32, out: GPtr<u32>, errors: GPtr<u32>) -> i32 {
    if src == 0 {
        return D3DERR_INVALIDCALL;
    }
    let p = GPtr::<u8>::from_addr(src as usize);
    let bytes: Vec<u8> = (0..len as i32).map(|i| p.wrapping_index(i).read()).take_while(|&b| b != 0).collect();
    let text = String::from_utf8_lossy(&bytes);
    if !errors.is_null() {
        errors.write(0);
    }
    match sa_rw_wgpu::sm::asm::assemble(&text) {
        Ok(tokens) => {
            if !out.is_null() {
                let b: Vec<u8> = tokens.iter().flat_map(|t| t.to_le_bytes()).collect();
                out.write(new_buffer(&b));
            }
            D3D_OK
        }
        Err(e) => {
            eprintln!("sa-game: D3DXAssembleShader: {e}\n{text}");
            if !errors.is_null() {
                let mut m = format!("(line {}): error: {}", e.line, e.msg).into_bytes();
                m.push(0);
                errors.write(new_buffer(&m));
            }
            E_FAIL
        }
    }
}

macro_rules! entries {
    ($( $addr:expr, $name:literal => $f:path : fn($($a:ident : $t:ty),*) -> $r:ty ;)*) => {
        /// `(address, name, native stdcall entry)` of every implemented method and of
        /// `D3DXAssembleShader`.
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

fn dev(slot: usize) -> u32 {
    crate::dx::trap(3, slot)
}

entries! {
    0x0076_52AE, "D3DXAssembleShader" => d3dx_assemble_shader: fn(src: u32, len: u32, defines: u32, include: u32, flags: u32, out: GPtr<u32>, errors: GPtr<u32>) -> i32;
    dev(91), "IDirect3DDevice9::CreateVertexShader" => dev_create_vertex_shader: fn(this: u32, code: u32, out: GPtr<u32>) -> i32;
    dev(92), "IDirect3DDevice9::SetVertexShader" => dev_set_vertex_shader: fn(this: u32, sh: u32) -> i32;
    dev(93), "IDirect3DDevice9::GetVertexShader" => dev_get_vertex_shader: fn(this: u32, out: GPtr<u32>) -> i32;
    dev(94), "IDirect3DDevice9::SetVertexShaderConstantF" => dev_set_vs_const_f: fn(this: u32, start: u32, data: u32, n: u32) -> i32;
    dev(95), "IDirect3DDevice9::GetVertexShaderConstantF" => dev_get_vs_const_f: fn(this: u32, start: u32, out: u32, n: u32) -> i32;
    dev(96), "IDirect3DDevice9::SetVertexShaderConstantI" => dev_set_vs_const_i: fn(this: u32, start: u32, data: u32, n: u32) -> i32;
    dev(97), "IDirect3DDevice9::GetVertexShaderConstantI" => dev_get_vs_const_i: fn(this: u32, start: u32, out: u32, n: u32) -> i32;
    dev(98), "IDirect3DDevice9::SetVertexShaderConstantB" => dev_set_vs_const_b: fn(this: u32, start: u32, data: u32, n: u32) -> i32;
    dev(99), "IDirect3DDevice9::GetVertexShaderConstantB" => dev_get_vs_const_b: fn(this: u32, start: u32, out: u32, n: u32) -> i32;
    dev(106), "IDirect3DDevice9::CreatePixelShader" => dev_create_pixel_shader: fn(this: u32, code: u32, out: GPtr<u32>) -> i32;
    dev(107), "IDirect3DDevice9::SetPixelShader" => dev_set_pixel_shader: fn(this: u32, sh: u32) -> i32;
    dev(108), "IDirect3DDevice9::GetPixelShader" => dev_get_pixel_shader: fn(this: u32, out: GPtr<u32>) -> i32;
    dev(109), "IDirect3DDevice9::SetPixelShaderConstantF" => dev_set_ps_const_f: fn(this: u32, start: u32, data: u32, n: u32) -> i32;
    dev(110), "IDirect3DDevice9::GetPixelShaderConstantF" => dev_get_ps_const_f: fn(this: u32, start: u32, out: u32, n: u32) -> i32;
    dev(111), "IDirect3DDevice9::SetPixelShaderConstantI" => dev_set_ps_const_i: fn(this: u32, start: u32, data: u32, n: u32) -> i32;
    dev(112), "IDirect3DDevice9::GetPixelShaderConstantI" => dev_get_ps_const_i: fn(this: u32, start: u32, out: u32, n: u32) -> i32;
    dev(113), "IDirect3DDevice9::SetPixelShaderConstantB" => dev_set_ps_const_b: fn(this: u32, start: u32, data: u32, n: u32) -> i32;
    dev(114), "IDirect3DDevice9::GetPixelShaderConstantB" => dev_get_ps_const_b: fn(this: u32, start: u32, out: u32, n: u32) -> i32;
    trap(VSHADER, 0), "IDirect3DVertexShader9::QueryInterface" => obj_query_interface: fn(obj: u32, iid: u32, out: GPtr<u32>) -> i32;
    trap(VSHADER, 1), "IDirect3DVertexShader9::AddRef" => obj_add_ref: fn(obj: u32) -> u32;
    trap(VSHADER, 2), "IDirect3DVertexShader9::Release" => obj_release: fn(obj: u32) -> u32;
    trap(VSHADER, 3), "IDirect3DVertexShader9::GetDevice" => shader_get_device: fn(obj: u32, out: GPtr<u32>) -> i32;
    trap(VSHADER, 4), "IDirect3DVertexShader9::GetFunction" => shader_get_function: fn(obj: u32, data: u32, size: GPtr<u32>) -> i32;
    trap(PSHADER, 0), "IDirect3DPixelShader9::QueryInterface" => obj_query_interface: fn(obj: u32, iid: u32, out: GPtr<u32>) -> i32;
    trap(PSHADER, 1), "IDirect3DPixelShader9::AddRef" => obj_add_ref: fn(obj: u32) -> u32;
    trap(PSHADER, 2), "IDirect3DPixelShader9::Release" => obj_release: fn(obj: u32) -> u32;
    trap(PSHADER, 3), "IDirect3DPixelShader9::GetDevice" => shader_get_device: fn(obj: u32, out: GPtr<u32>) -> i32;
    trap(PSHADER, 4), "IDirect3DPixelShader9::GetFunction" => shader_get_function: fn(obj: u32, data: u32, size: GPtr<u32>) -> i32;
    trap(XBUFFER, 0), "ID3DXBuffer::QueryInterface" => obj_query_interface: fn(obj: u32, iid: u32, out: GPtr<u32>) -> i32;
    trap(XBUFFER, 1), "ID3DXBuffer::AddRef" => obj_add_ref: fn(obj: u32) -> u32;
    trap(XBUFFER, 2), "ID3DXBuffer::Release" => obj_release: fn(obj: u32) -> u32;
    trap(XBUFFER, 3), "ID3DXBuffer::GetBufferPointer" => buffer_get_pointer: fn(obj: u32) -> u32;
    trap(XBUFFER, 4), "ID3DXBuffer::GetBufferSize" => buffer_get_size: fn(obj: u32) -> u32;
}

#[cfg(all(test, target_arch = "x86"))]
mod tests {
    use super::*;

    fn ptr(p: &mut u32) -> GPtr<u32> {
        GPtr::from_addr(p as *mut u32 as usize)
    }

    /// `D3DXAssembleShader` on RW-style text: an `ID3DXBuffer` whose bytes decode to the shader;
    /// a syntax error gives `E_FAIL` and an error buffer. (32-bit: guest pointers are host ones.)
    #[test]
    fn assemble_into_buffer_objects() {
        let src = b"vs_1_1\ndcl_position0 v0\nm4x4 oPos.xyzw, v0, c[0]\n\0";
        let (mut out, mut errs) = (0u32, 0u32);
        let hr = d3dx_assemble_shader(src.as_ptr() as u32, src.len() as u32, 0, 0, 4, ptr(&mut out), ptr(&mut errs));
        assert_eq!((hr, errs), (D3D_OK, 0));
        let (p, n) = (buffer_get_pointer(out), buffer_get_size(out));
        let tokens: Vec<u32> = (0..n as i32 / 4).map(|i| GPtr::<u32>::from_addr(p as usize).wrapping_index(i).read()).collect();
        let sh = sa_rw_wgpu::sm::parse(&tokens).unwrap();
        assert_eq!(sh.to_string(), "vs_1_1\ndcl_position v0\nm4x4 oPos, v0, c0\n");
        assert_eq!(obj_add_ref(out), 2);
        assert_eq!(obj_release(out), 1);
        assert_eq!(obj_release(out), 0);
        assert_eq!(buffer_get_size(out), 0);

        let bad = b"vs_1_1\nfrob r0, v0\n";
        let hr = d3dx_assemble_shader(bad.as_ptr() as u32, bad.len() as u32, 0, 0, 4, ptr(&mut out), ptr(&mut errs));
        assert_eq!(hr, E_FAIL);
        assert!(errs != 0 && buffer_get_size(errs) > 0);
        // the token stream reader stops at the end token, skipping comment payloads
        let stream = [0xFFFE_0101u32, 0x0001_FFFE, 0x0000_FFFF, 0x0000_FFFF, 0xDEAD_BEEF];
        assert_eq!(read_tokens(stream.as_ptr() as u32).map(|t| t.len()), Some(4));
    }
}
