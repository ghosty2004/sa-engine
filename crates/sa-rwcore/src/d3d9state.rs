//! PORT-RW-4 module `d3d9state`: the D3D9 driver's render-state, texture-stage-state and sampler-state
//! caches, `RwD3D9SetTexture`, materials / surface properties and the RW render-state handlers
//! (`_rwD3D9RWSetRenderState` / `_rwD3D9RWGetRenderState`). See docs/arch/rw-port.md (PORT-RW-4).
//! Ported from the 1.0 US bytes (0x7FC200..0x7FEE20); `docs/re/specs/RW/RwD3D9RenderState.md` as a
//! reading aid. Every D3D9 call is made through the device's vtable exactly as the original makes it
//! (same method, arguments and order), so the D3D9 call stream is unchanged.
//!
//! Cache layout (all `.bss`, accessed raw; other modules use the same cells):
//! - render states: `{u32 value; u32 queued}` x 210 at 0xC991D0 (wanted), `u32` x 210 at 0xC98E88 (last sent),
//!   dirty list `u32` at 0xC98B40, count 0xC9A5EC;
//! - texture-stage states: `{value, queued}` x (8*33) at 0xC980F8 (index stage*33+type), last sent 0xC99860,
//!   dirty list `{stage, type}` at 0xC99C80, count 0xC9A5F0;
//! - sampler states (immediate): `u32` x (8*14) at 0xC98938 (index stage*14+type);
//! - material last sent (0x11 dwords) 0xC98AF8, staging material 0x8E2538, surface key 0xC9A5D8..0xC9A600;
//! - per-stage RW texture state 8 x 0x18 at 0xC9A508 (+0 raster, +4 address U, +8 V, +0xC filter,
//!   +0x10 border colour, +0x14 max anisotropy);
//! - RW mirrors 0xC9A4C0..0xC9A504, 0xC9A5C8..0xC9A5D4, anisotropy plugin offset 0xC9A5E8.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, x};

/// `_RwD3DDevice` (`IDirect3DDevice9*`).
pub const D3D_DEVICE: usize = 0xC97C28;
/// `D3DCAPS9.RasterCaps` of the driver's caps copy.
pub const CAPS_RASTER: usize = 0xC9BF24;
/// `D3DCAPS9.TextureCaps`.
pub const CAPS_TEXTURE: usize = 0xC9BF3C;
/// `D3DCAPS9.MaxTextureWidth` / `MaxTextureHeight`.
pub const CAPS_MAX_TEX_W: usize = 0xC9BF58;
pub const CAPS_MAX_TEX_H: usize = 0xC9BF5C;

const RS: u32 = 0xC991D0;
const RS_DEV: u32 = 0xC98E88;
const RS_LIST: u32 = 0xC98B40;
const RS_COUNT: usize = 0xC9A5EC;
const TSS: u32 = 0xC980F8;
const TSS_DEV: u32 = 0xC99860;
const TSS_LIST: u32 = 0xC99C80;
const TSS_COUNT: usize = 0xC9A5F0;
const SAMP: u32 = 0xC98938;
const MATERIAL: usize = 0xC98AF8;
const MATERIAL_STAGING: usize = 0x8E2538;
/// Per-stage RW texture state (0x18 bytes per stage).
const STAGE: u32 = 0xC9A508;

// RW mirrors.
const M_ZWRITE: usize = 0xC9A4C0;
const M_ZTEST: usize = 0xC9A4C4;
const M_STENCIL_ENABLE: usize = 0xC9A4C8;
const M_STENCIL_FAIL: usize = 0xC9A4CC;
const M_STENCIL_ZFAIL: usize = 0xC9A4D0;
const M_STENCIL_PASS: usize = 0xC9A4D4;
const M_STENCIL_FUNC: usize = 0xC9A4D8;
const M_STENCIL_REF: usize = 0xC9A4DC;
const M_STENCIL_MASK: usize = 0xC9A4E0;
const M_STENCIL_WMASK: usize = 0xC9A4E4;
const M_VERTEX_ALPHA: usize = 0xC9A4E8;
const M_TEXTURE_ALPHA: usize = 0xC9A4EC;
const M_CULL: usize = 0xC9A4F0;
const M_FOG_ENABLE: usize = 0xC9A4F4;
const M_FOG_TYPE: usize = 0xC9A4F8;
const M_FOG_DENSITY: usize = 0xC9A4FC;
const M_FOG_COLOR: usize = 0xC9A500;
const M_SHADE: usize = 0xC9A504;
const M_SRC_BLEND: usize = 0xC9A5C8;
const M_DEST_BLEND: usize = 0xC9A5CC;
const M_ALPHA_FUNC: usize = 0xC9A5D0;
const M_ALPHA_TEST_WANTED: usize = 0xC9A5D4;
const ANISOT_OFFSET: usize = 0xC9A5E8;

// Surface-properties key.
const KEY_FLAGS: usize = 0xC9A5F4;
const KEY_COLOR: usize = 0xC9A5F8;
const KEY_DIFFUSE: usize = 0xC9A5FC;
const KEY_AMBIENT: usize = 0xC9A600;
const KEY_AMB_LIGHT: usize = 0xC9A5D8;
/// `AmbientSaturated` (r, g, b f32), written by the light code.
const AMB_LIGHT: usize = 0x8E2418;
/// `1/255` (f32).
const K_1_255: usize = 0x859A3C;
/// The camera the driver renders with (`+0x84` farPlane, `+0x88` fogPlane).
const CUR_CAMERA: usize = 0xC9BCC0;
/// D3D9 raster extension offset inside RwRaster (`+0` texture, `+4` palette, `+8` u8 alpha).
const RASTER_EXT_OFFSET: usize = 0xB4E9E0;

// Mapping tables (u32, `.rdata`).
const T_STENCIL_OP: u32 = 0x8849A0;
const T_CMP: u32 = 0x8849C4;
const T_SHADE: u32 = 0x8848F0;
const T_CULL: u32 = 0x884990;
const T_BLEND: u32 = 0x88490C;
const T_ADDRESS: u32 = 0x88493C;
const T_FOG: u32 = 0x8848FC;
/// Filter pairs `(min/mag, mip)`.
const T_FILTER: u32 = 0x884950;

orig! {
    /// `IDirect3DDevice9::SetRenderState` (vtable +0xE4)
    pub fn dev_set_render_state(dev: GPtr<u8>, state: u32, value: u32) -> i32 = stdcall vslot 0xE4 => unported;
    /// `IDirect3DDevice9::SetTextureStageState` (+0x10C)
    pub fn dev_set_texture_stage_state(dev: GPtr<u8>, stage: u32, ty: u32, value: u32) -> i32 = stdcall vslot 0x10C => unported;
    /// `IDirect3DDevice9::SetSamplerState` (+0x114)
    pub fn dev_set_sampler_state(dev: GPtr<u8>, stage: u32, ty: u32, value: u32) -> i32 = stdcall vslot 0x114 => unported;
    /// `IDirect3DDevice9::SetMaterial` (+0xC4)
    pub fn dev_set_material(dev: GPtr<u8>, m: GPtr<u8>) -> i32 = stdcall vslot 0xC4 => unported;
    /// `IDirect3DDevice9::SetClipPlane` (+0xDC)
    pub fn dev_set_clip_plane(dev: GPtr<u8>, index: u32, plane: GPtr<u8>) -> i32 = stdcall vslot 0xDC => unported;
    /// `IDirect3DDevice9::SetTexture` (+0x104)
    pub fn dev_set_texture(dev: GPtr<u8>, stage: u32, tex: u32) -> i32 = stdcall vslot 0x104 => unported;
    /// `IDirect3DDevice9::SetCurrentTexturePalette` (+0x124)
    pub fn dev_set_current_texture_palette(dev: GPtr<u8>, index: u32) -> i32 = stdcall vslot 0x124 => unported;

    /// `RwD3D9SetRenderState(state, value)` (0x7FC2D0)
    pub fn set_render_state_call(state: u32, value: u32) -> u32 = cdecl 0x7FC2D0 => set_render_state;
    /// `_rwD3D9SetTextureInternal(raster, stage)` (0x7FDCD0)
    pub fn set_texture_internal_call(raster: GPtr<u8>, stage: u32) -> u32 = cdecl 0x7FDCD0 => set_texture_internal;
    /// `_rwD3D9RenderStateTextureFilter(filter)` (0x7FDB80)
    pub fn rs_texture_filter_call(f: u32) -> u32 = cdecl 0x7FDB80 => rs_texture_filter;
    /// `_rwD3D9RenderStateZTestEnable` (0x7FEA70, really ZWRITEENABLE)
    pub fn rs_zwrite_call(v: u32) -> u32 = cdecl 0x7FEA70 => rs_zwrite_enable;
    /// `_rwD3D9RenderStateZWriteEnable` (0x7FEB70, really ZTESTENABLE)
    pub fn rs_ztest_call(v: u32) -> u32 = cdecl 0x7FEB70 => rs_ztest_enable;
    /// `_rwD3D9RenderStateShadeMode` (0x7FEC70)
    pub fn rs_shade_call(v: u32) -> u32 = cdecl 0x7FEC70 => rs_shade_mode;
    /// `_rwD3D9RenderStateVertexAlphaEnable` (0x7FE0A0)
    pub fn rs_vertex_alpha_call(v: u32) -> u32 = cdecl 0x7FE0A0 => rs_vertex_alpha_enable;
    /// `_rwD3D9RenderStateTextureBorderColor` (0x7FECC0)
    pub fn rs_border_call(v: u32) -> u32 = cdecl 0x7FECC0 => rs_texture_border_color;
    /// `_rwD3D9RenderStateCullMode` (0x7FED00)
    pub fn rs_cull_call(v: u32) -> u32 = cdecl 0x7FED00 => rs_cull_mode;
    /// `_rwD3D9RenderStateStencilEnable` (0x7FE1A0)
    pub fn rs_stencil_enable_call(v: u32) -> u32 = cdecl 0x7FE1A0 => rs_stencil_enable;
    /// `_rwD3D9RenderStateStencilFail` (0x7FE1F0)
    pub fn rs_stencil_fail_call(v: u32) -> u32 = cdecl 0x7FE1F0 => rs_stencil_fail;
    /// `_rwD3D9RenderStateStencilZFail` (0x7FE240)
    pub fn rs_stencil_zfail_call(v: u32) -> u32 = cdecl 0x7FE240 => rs_stencil_zfail;
    /// `_rwD3D9RenderStateStencilPass` (0x7FE290)
    pub fn rs_stencil_pass_call(v: u32) -> u32 = cdecl 0x7FE290 => rs_stencil_pass;
    /// `_rwD3D9RenderStateStencilFunction` (0x7FE2E0)
    pub fn rs_stencil_func_call(v: u32) -> u32 = cdecl 0x7FE2E0 => rs_stencil_function;
    /// `_rwD3D9RenderStateStencilRef` (0x7FE330)
    pub fn rs_stencil_ref_call(v: u32) -> u32 = cdecl 0x7FE330 => rs_stencil_ref;
    /// `_rwD3D9RenderStateStencilMask` (0x7FE380)
    pub fn rs_stencil_mask_call(v: u32) -> u32 = cdecl 0x7FE380 => rs_stencil_mask;
    /// `_rwD3D9RenderStateStencilWriteMask` (0x7FE3D0)
    pub fn rs_stencil_wmask_call(v: u32) -> u32 = cdecl 0x7FE3D0 => rs_stencil_write_mask;
}

/// `_RwD3DDevice`.
pub fn device() -> GPtr<u8> {
    p8(g::<u32>(D3D_DEVICE).read())
}

#[inline]
fn cell(base: u32, index: u32, scale: u32) -> GPtr<u32> {
    g::<u32>(base.wrapping_add(index.wrapping_mul(scale)) as usize)
}

#[inline]
fn gu(addr: usize) -> u32 {
    g::<u32>(addr).read()
}

#[inline]
fn su(addr: usize, v: u32) {
    g::<u32>(addr).write(v)
}

/// `rs[s].value`.
pub fn rs_value(s: u32) -> GPtr<u32> {
    cell(RS, s, 8)
}

fn rs_queued(s: u32) -> GPtr<u32> {
    cell(RS + 4, s, 8)
}

/// `rsList[count++] = s` (the count re-read from memory).
fn rs_append(s: u32) {
    let n = gu(RS_COUNT);
    cell(RS_LIST, n, 4).write(s);
    su(RS_COUNT, n.wrapping_add(1));
}

/// Marks `s` queued (if it is not) without touching its value.
fn rs_mark(s: u32) {
    if rs_queued(s).read() == 0 {
        rs_queued(s).write(1);
        rs_append(s);
    }
}

/// `rs[s].value = v` unconditionally, then queue.
fn rs_store(s: u32, v: u32) {
    rs_value(s).write(v);
    rs_mark(s);
}

/// Compare-and-queue: only when the wanted value changes.
fn rs_set(s: u32, v: u32) {
    if rs_value(s).read() != v {
        rs_store(s, v);
    }
}

fn table(t: u32, i: u32) -> u32 {
    cell(t, i, 4).read()
}

fn samp(stage: u32, ty: u32) -> GPtr<u32> {
    cell(SAMP, stage.wrapping_mul(14).wrapping_add(ty), 4)
}

fn stage_field(stage: u32, off: u32) -> GPtr<u32> {
    g::<u32>(STAGE.wrapping_add(stage.wrapping_mul(0x18)).wrapping_add(off) as usize)
}

/// A direct `SetSamplerState` with the cache cell updated first (no compare).
fn sampler_direct(stage: u32, ty: u32, v: u32) {
    samp(stage, ty).write(v);
    dev_set_sampler_state(device(), stage, ty, v);
}

/// `_rwD3D9RenderStateFlushCache` (0x7FC200): sends every queued render state whose wanted value differs
/// from the last sent one (queue order), then the same for texture-stage states; both counts are re-read
/// on every pass and cleared at the end.
#[hook(addr = 0x7FC200, conv = cdecl, name = "_rwD3D9RenderStateFlushCache", expect = "A1 EC A5 C9 00 53")]
pub fn flush_cache() {
    if gu(RS_COUNT) != 0 {
        let mut i = 0u32;
        loop {
            let s = cell(RS_LIST, i, 4).read();
            let v = rs_value(s).read();
            let dev_v = cell(RS_DEV, s, 4);
            rs_queued(s).write(0);
            if dev_v.read() != v {
                dev_v.write(v);
                dev_set_render_state(device(), s, v);
            }
            i = i.wrapping_add(1);
            if i >= gu(RS_COUNT) {
                break;
            }
        }
        su(RS_COUNT, 0);
    }
    if gu(TSS_COUNT) != 0 {
        let mut i = 0u32;
        loop {
            let stage = cell(TSS_LIST, i, 8).read();
            let ty = cell(TSS_LIST + 4, i, 8).read();
            let idx = stage.wrapping_mul(33).wrapping_add(ty);
            let v = cell(TSS, idx, 8).read();
            let dev_v = cell(TSS_DEV, idx, 4);
            cell(TSS + 4, idx, 8).write(0);
            if dev_v.read() != v {
                dev_v.write(v);
                dev_set_texture_stage_state(device(), stage, ty, v);
            }
            i = i.wrapping_add(1);
            if i >= gu(TSS_COUNT) {
                break;
            }
        }
        su(TSS_COUNT, 0);
    }
}

/// `RwD3D9SetRenderState` (0x7FC2D0): deferred; compare, store, queue; returns `state` (EAX). The original leaves EDX alone and
/// its callers in `_rwD3D9RenderStateOpen` (0x7FCEBE.., 0x7FCFF4..) and `_rwD3D9RWSetRenderState`
/// (0x7FE99F) keep a value in EDX across the call (sa-hook callers test): `preserve = "edx"`.
#[hook(addr = 0x7FC2D0, conv = cdecl, name = "RwD3D9SetRenderState", expect = "8B 44 24 04 8B 4C 24 08", preserve = "edx")]
pub fn set_render_state(state: u32, value: u32) -> u32 {
    rs_set(state, value);
    // EAX: the original loads `state` into EAX first and never changes it (callers such as
    // `_rwD3D9LightsEnable` tail-call it and return that value).
    state
}

/// `RwD3D9GetRenderState` (0x7FC320): the wanted value.
#[hook(addr = 0x7FC320, conv = cdecl, name = "RwD3D9GetRenderState", expect = "8B 44 24 04 8B 54 24 08")]
pub fn get_render_state(state: u32, out: GPtr<u32>) {
    out.write(rs_value(state).read());
}

/// `RwD3D9SetTextureStageState` (0x7FC340): deferred; compare, store, queue `{stage, type}`.
#[hook(addr = 0x7FC340, conv = cdecl, name = "RwD3D9SetTextureStageState", expect = "8B 4C 24 04 8B 54 24 08")]
pub fn set_texture_stage_state(stage: u32, ty: u32, value: u32) {
    let idx = stage.wrapping_mul(33).wrapping_add(ty);
    let v = cell(TSS, idx, 8);
    if v.read() != value {
        v.write(value);
        let q = cell(TSS + 4, idx, 8);
        if q.read() == 0 {
            q.write(1);
            let n = gu(TSS_COUNT);
            cell(TSS_LIST, n, 8).write(stage);
            cell(TSS_LIST + 4, n, 8).write(ty);
            su(TSS_COUNT, n.wrapping_add(1));
        }
    }
}

/// `RwD3D9GetTextureStageState` (0x7FC3A0).
#[hook(addr = 0x7FC3A0, conv = cdecl, name = "RwD3D9GetTextureStageState", expect = "8B 44 24 04 8B 54 24 08")]
pub fn get_texture_stage_state(stage: u32, ty: u32, out: GPtr<u32>) {
    out.write(cell(TSS, stage.wrapping_mul(33).wrapping_add(ty), 8).read());
}

/// `RwD3D9SetSamplerState` (0x7FC3C0): immediate when the cached value changes.
#[hook(addr = 0x7FC3C0, conv = cdecl, name = "RwD3D9SetSamplerState", expect = "8B 4C 24 04 56 8B 74 24 0C")]
pub fn set_sampler_state(stage: u32, ty: u32, value: u32) {
    let c = samp(stage, ty);
    if c.read() != value {
        c.write(value);
        dev_set_sampler_state(device(), stage, ty, value);
    }
}

/// `RwD3D9GetSamplerState` (0x7FC400).
#[hook(addr = 0x7FC400, conv = cdecl, name = "RwD3D9GetSamplerState", expect = "8B 44 24 04 8B 54 24 08")]
pub fn get_sampler_state(stage: u32, ty: u32, out: GPtr<u32>) {
    out.write(samp(stage, ty).read());
}

/// `RwD3D9SetMaterial` (0x7FC430): 1 when the 0x11 dwords equal the cache (integer compare); else copy,
/// clear the surface key's flags word and `SetMaterial(m)`; returns `hr >= 0`.
#[hook(addr = 0x7FC430, conv = cdecl, name = "RwD3D9SetMaterial", expect = "8B 54 24 04 53 B9 F8 8A C9 00")]
pub fn set_material(m: GPtr<u8>) -> u32 {
    let cache = p8(MATERIAL as u32);
    if (0..0x11).all(|i| rd::<u32>(cache, i * 4) == rd::<u32>(m, i * 4)) {
        return 1;
    }
    for i in 0..0x11 {
        crate::common::wr::<u32>(cache, i * 4, rd::<u32>(m, i * 4));
    }
    su(KEY_FLAGS, 0);
    (dev_set_material(device(), m) >= 0) as u32
}

/// `RwD3D9SetClipPlane` (0x7FC4A0): `SetClipPlane(index, plane)`; returns `hr >= 0`.
#[hook(addr = 0x7FC4A0, conv = cdecl, name = "RwD3D9SetClipPlane", expect = "8B 54 24 08 A1 28 7C C9 00")]
pub fn set_clip_plane(index: u32, plane: GPtr<u8>) -> u32 {
    (dev_set_clip_plane(device(), index, plane) >= 0) as u32
}

/// `fild` of a colour byte.
fn fild_u8(b: u8) -> sa_math::x87::X87 {
    sa_math::x87::X87::from_i32(b as i32)
}

/// `RwD3D9SetSurfaceProperties` (0x7FC4D0). Key = (diffuse, ambient, colour dword, flags & 0x48, ambient
/// light rgb), all integer compares. Builds the D3DMATERIAL9 at 0x8E2538 (only the words the original
/// writes), queues AMBIENT / COLORVERTEX / AMBIENTMATERIALSOURCE / EMISSIVEMATERIALSOURCE, copies the
/// material to the cache and calls `SetMaterial(0x8E2538)`; returns `hr >= 0`.
#[hook(addr = 0x7FC4D0, conv = cdecl, name = "RwD3D9SetSurfaceProperties", expect = "8B 4C 24 04 8B 54 24 0C")]
pub fn set_surface_properties(sp: GPtr<u8>, color: GPtr<u8>, flags: u32) -> u32 {
    let f = flags & 0x48;
    let key_same = gu(KEY_DIFFUSE) == rd::<u32>(sp, 8)
        && gu(KEY_AMBIENT) == rd::<u32>(sp, 0)
        && gu(KEY_COLOR) == rd::<u32>(color, 0)
        && gu(KEY_FLAGS) == f
        && gu(KEY_AMB_LIGHT) == gu(AMB_LIGHT)
        && gu(KEY_AMB_LIGHT + 4) == gu(AMB_LIGHT + 4)
        && gu(KEY_AMB_LIGHT + 8) == gu(AMB_LIGHT + 8);
    if key_same {
        return 1;
    }
    su(KEY_DIFFUSE, rd::<u32>(sp, 8));
    su(KEY_AMBIENT, rd::<u32>(sp, 0));
    su(KEY_COLOR, rd::<u32>(color, 0));
    su(KEY_AMB_LIGHT, gu(AMB_LIGHT));
    su(KEY_AMB_LIGHT + 4, gu(AMB_LIGHT + 4));
    su(KEY_FLAGS, f);
    su(KEY_AMB_LIGHT + 8, gu(AMB_LIGHT + 8));
    let mat = |off: usize| g::<f32>(MATERIAL_STAGING + off);
    let amb_light = |i: usize| g::<f32>(AMB_LIGHT + i * 4).read();
    let c = |i: isize| rd::<u8>(color, i);
    if f & 0x40 != 0 && rd::<u32>(color, 0) != 0xFFFF_FFFF {
        let k = g::<f32>(K_1_255).read();
        let s = x(rd::<f32>(sp, 8)).fmul_m(k);
        mat(0).write(fild_u8(c(0)).fmul(s).to_f32());
        mat(4).write(fild_u8(c(1)).fmul(s).to_f32());
        mat(8).write(fild_u8(c(2)).fmul(s).to_f32());
        mat(0xC).write(fild_u8(c(3)).fmul_m(k).to_f32());
        let amb = x(rd::<f32>(sp, 0)).fmul_m(k);
        if f & 8 != 0 {
            let d3dcolor =
                ((c(3) as u32) << 24) | ((c(0) as u32) << 16) | ((c(1) as u32) << 8) | c(2) as u32;
            rs_set(0x8B, d3dcolor);
            rs_set(0x8D, 1);
            rs_set(0x93, 1);
            rs_set(0x94, 0);
            mat(0x10).write(0.0);
            mat(0x14).write(0.0);
            mat(0x18).write(0.0);
            for i in 0..3 {
                let v = fild_u8(c(i as isize)).fmul_m(amb_light(i)).fmul(amb).to_f32();
                mat(0x30 + i * 4).write(v);
            }
        } else {
            rs_set(0x8B, 0xFFFF_FFFF);
            rs_set(0x8D, 0);
            rs_set(0x93, 0);
            rs_set(0x94, 0);
            for i in 0..2 {
                let v = fild_u8(c(i as isize)).fmul_m(amb_light(i)).fmul(amb).to_f32();
                mat(0x10 + i * 4).write(v);
            }
            mat(0x30).write(0.0);
            mat(0x34).write(0.0);
            let b = fild_u8(c(2));
            mat(0x38).write(0.0);
            mat(0x18).write(b.fmul_m(amb_light(2)).fmul(amb).to_f32());
        }
    } else {
        // fld / fst: a signalling NaN is stored quieted.
        let d = x(rd::<f32>(sp, 8)).to_f32();
        mat(0).write(d);
        mat(4).write(d);
        mat(8).write(d);
        g::<u32>(MATERIAL_STAGING + 0xC).write(0x3F80_0000);
        rs_set(0x8B, 0xFFFF_FFFF);
        rs_set(0x93, 0);
        if f & 8 != 0 {
            rs_set(0x8D, 1);
            rs_set(0x94, 1);
        } else {
            rs_set(0x8D, 0);
            rs_set(0x94, 0);
            mat(0x30).write(0.0);
            mat(0x34).write(0.0);
            mat(0x38).write(0.0);
        }
        if rd::<u32>(sp, 0) != 0x3F80_0000 {
            let a = rd::<f32>(sp, 0);
            for i in 0..3 {
                mat(0x10 + i * 4).write(x(amb_light(i)).fmul_m(a).to_f32());
            }
        } else {
            for i in 0..3 {
                g::<u32>(MATERIAL_STAGING + 0x10 + i * 4).write(gu(AMB_LIGHT + i * 4));
            }
        }
    }
    for i in 0..0x11 {
        g::<u32>(MATERIAL + i * 4).write(gu(MATERIAL_STAGING + i * 4));
    }
    (dev_set_material(device(), p8(MATERIAL_STAGING as u32)) >= 0) as u32
}

orig! {
    /// `RwD3D9SetTextureStageState(stage, type, value)` (0x7FC340)
    pub fn set_tss_call(stage: u32, ty: u32, value: u32) = cdecl 0x7FC340 => set_texture_stage_state;
    /// `RwD3D9SetSamplerState(stage, type, value)` (0x7FC3C0)
    pub fn set_sampler_call(stage: u32, ty: u32, value: u32) = cdecl 0x7FC3C0 => set_sampler_state;
    /// `_rwD3D9RenderStateFlushCache()` (0x7FC200, reached by a tail jump)
    pub fn flush_cache_call() = cdecl 0x7FC200 => flush_cache;
}

/// The cache wipe shared by `_rwD3D9RenderStateOpen` and `_rwD3D9RenderStateReset`: last-sent render
/// states -1 except FOGCOLOR, AMBIENT, STENCILMASK, STENCILWRITEMASK (0); wanted = last sent, not queued;
/// texture-stage states -1 (both), samplers -1, material and surface key 0, both lists empty.
fn wipe_caches() {
    for i in 0..0xD2u32 {
        cell(RS_DEV, i, 4).write(0xFFFF_FFFF);
    }
    for s in [0x22u32, 0x8B, 0x3A, 0x3B] {
        cell(RS_DEV, s, 4).write(0);
    }
    for i in 0..0xD2u32 {
        rs_queued(i).write(0);
        rs_value(i).write(cell(RS_DEV, i, 4).read());
    }
    for i in 0..0x108u32 {
        cell(TSS_DEV, i, 4).write(0xFFFF_FFFF);
    }
    su(RS_COUNT, 0);
    for i in 0..0x108u32 {
        cell(TSS, i, 8).write(0xFFFF_FFFF);
        cell(TSS + 4, i, 8).write(0);
    }
    su(TSS_COUNT, 0);
    for i in 0..0x70u32 {
        cell(SAMP, i, 4).write(0xFFFF_FFFF);
    }
    su(KEY_DIFFUSE, 0);
    for i in 0..0x11 {
        su(MATERIAL + i * 4, 0);
    }
    su(KEY_AMBIENT, 0);
    su(KEY_AMB_LIGHT, 0);
    su(KEY_AMB_LIGHT + 4, 0);
    su(KEY_AMB_LIGHT + 8, 0);
    su(KEY_FLAGS, 0);
    su(KEY_COLOR, 0);
}

/// Both table-fog caps (RasterCaps FOGTABLE 0x100 and WFOG 0x100000).
fn table_fog() -> bool {
    let caps = gu(CAPS_RASTER);
    caps & 0x100 != 0 && caps & 0x10_0000 != 0
}

/// The fixed tail of Open / Reset after the per-stage loop and the blend states: stage-0 texture-stage
/// states, the lighting-related render states and DITHERENABLE from RasterCaps bit 0, then the flush.
fn open_tail() {
    set_tss_call(0, 1, 3);
    set_tss_call(0, 3, 0);
    set_tss_call(0, 4, 3);
    set_tss_call(0, 6, 0);
    set_render_state_call(0x91, 0);
    set_render_state_call(0x92, 0);
    set_render_state_call(0x93, 0);
    set_render_state_call(0x1A, gu(CAPS_RASTER) & 1);
    set_render_state_call(0x1D, 0);
    set_render_state_call(0x8E, 0);
    set_render_state_call(0x8B, 0xFFFF_FFFF);
    set_render_state_call(0x8F, 0);
    flush_cache_call();
}

/// `_rwD3D9RenderStateOpen` (0x7FCAC0): wipes the caches and queues / sends the RW defaults (fog off,
/// Gouraud, z test + write, stencil off, per stage: no texture, linear or point filter by
/// TextureFilterCaps, wrap, black border, anisotropy 1; blend SRCALPHA / INVSRCALPHA, alpha func
/// GREATER, cull back), then flushes.
#[hook(addr = 0x7FCAC0, conv = cdecl, name = "_rwD3D9RenderStateOpen", expect = "53 55 56 83 CB FF 57")]
pub fn render_state_open() {
    wipe_caches();
    su(M_FOG_ENABLE, 0);
    su(M_FOG_TYPE, 1);
    su(M_FOG_DENSITY, 0x3F80_0000);
    su(M_FOG_COLOR, 0);
    rs_set(0x1C, 0);
    if table_fog() {
        rs_set(0x23, 3);
        rs_set(0x8C, 0);
    } else {
        rs_set(0x23, 0);
        rs_set(0x8C, 3);
    }
    rs_set(0x22, 0);
    rs_set(9, 2);
    su(M_SHADE, 2);
    rs_set(0x17, 4);
    rs_set(0xE, 1);
    rs_set(7, 1);
    su(M_ZWRITE, 1);
    su(M_ZTEST, 1);
    rs_set(0x34, 0);
    rs_set(0x35, 1);
    rs_set(0x36, 1);
    rs_set(0x37, 1);
    rs_set(0x38, 8);
    rs_set(0x39, 0);
    set_render_state_call(0x3A, 0xFFFF_FFFF);
    set_render_state_call(0x3B, 0xFFFF_FFFF);
    su(M_STENCIL_FUNC, 8);
    su(M_STENCIL_MASK, 0xFFFF_FFFF);
    su(M_STENCIL_WMASK, 0xFFFF_FFFF);
    su(M_STENCIL_ENABLE, 0);
    su(M_STENCIL_FAIL, 1);
    su(M_STENCIL_ZFAIL, 1);
    su(M_STENCIL_PASS, 1);
    su(M_STENCIL_REF, 0);
    for stage in 0..8u32 {
        dev_set_texture(device(), stage, 0);
        stage_field(stage, 0).write(0);
        if gu(0xC9BF40) & 0x200 != 0 {
            if samp(stage, 6).read() != 2 {
                sampler_direct(stage, 6, 2);
            }
            set_sampler_call(stage, 5, 2);
            stage_field(stage, 0xC).write(2);
        } else {
            set_sampler_call(stage, 6, 1);
            set_sampler_call(stage, 5, 1);
            stage_field(stage, 0xC).write(1);
        }
        set_sampler_call(stage, 7, 0);
        set_sampler_call(stage, 1, 1);
        set_sampler_call(stage, 2, 1);
        stage_field(stage, 4).write(1);
        stage_field(stage, 8).write(1);
        set_sampler_call(stage, 4, 0xFF00_0000);
        stage_field(stage, 0x10).write(0xFF00_0000);
        stage_field(stage, 0x14).write(1);
        set_sampler_call(stage, 0xA, 1);
        if stage != 0 {
            set_tss_call(stage, 1, 1);
            set_tss_call(stage, 4, 1);
        }
    }
    set_render_state_call(0x13, 5);
    set_render_state_call(0x14, 6);
    su(M_SRC_BLEND, 5);
    su(M_DEST_BLEND, 6);
    set_render_state_call(0x19, 5);
    su(M_ALPHA_FUNC, 5);
    su(M_ALPHA_TEST_WANTED, 1);
    set_render_state_call(0x18, 0);
    set_render_state_call(0x1B, 0);
    set_render_state_call(0xF, 0);
    su(M_VERTEX_ALPHA, 0);
    su(M_TEXTURE_ALPHA, 0);
    set_render_state_call(0x89, 0);
    set_render_state_call(0x16, 2);
    su(M_CULL, 2);
    open_tail();
}

/// `_rwD3D9RenderStateReset` (0x7FD100, after a device reset): saves the wanted FOGSTART, FOGEND and
/// ALPHAREF, wipes the caches, then re-applies the RW mirrors (fog, shade, z, stencil, every stage's
/// sampler states, blend, alpha test, cull) and flushes.
#[hook(addr = 0x7FD100, conv = cdecl, name = "_rwD3D9RenderStateReset", expect = "83 EC 0C A1 F0 92 C9 00")]
pub fn render_state_reset() {
    let fog_start = rs_value(0x24).read();
    let fog_end = rs_value(0x25).read();
    let alpha_ref = rs_value(0x18).read();
    wipe_caches();
    rs_set(0x26, gu(M_FOG_DENSITY));
    rs_set(0x1C, gu(M_FOG_ENABLE));
    let fog = table(T_FOG, gu(M_FOG_TYPE));
    if table_fog() {
        rs_set(0x23, fog);
        rs_set(0x8C, 0);
    } else {
        rs_set(0x23, 0);
        rs_set(0x8C, table(T_FOG, gu(M_FOG_TYPE)));
    }
    rs_set(0x24, fog_start);
    rs_set(0x25, fog_end);
    rs_set(0x22, gu(M_FOG_COLOR));
    rs_set(9, table(T_SHADE, gu(M_SHADE)));
    let zwrite = gu(M_ZWRITE);
    rs_set(0xE, zwrite);
    if gu(M_ZTEST) != 0 {
        rs_set(0x17, 4);
        rs_set(7, 1);
    } else {
        rs_set(0x17, 8);
        rs_set(7, (zwrite != 0) as u32);
    }
    rs_set(0x34, gu(M_STENCIL_ENABLE));
    rs_set(0x35, table(T_STENCIL_OP, gu(M_STENCIL_FAIL)));
    rs_set(0x36, table(T_STENCIL_OP, gu(M_STENCIL_ZFAIL)));
    rs_set(0x37, table(T_STENCIL_OP, gu(M_STENCIL_PASS)));
    set_render_state_call(0x38, table(T_CMP, gu(M_STENCIL_FUNC)));
    set_render_state_call(0x39, gu(M_STENCIL_REF));
    set_render_state_call(0x3A, gu(M_STENCIL_MASK));
    set_render_state_call(0x3B, gu(M_STENCIL_WMASK));
    for stage in 0..8u32 {
        dev_set_texture(device(), stage, 0);
        stage_field(stage, 0).write(0);
        let f = stage_field(stage, 0xC);
        set_sampler_call(stage, 6, cell(T_FILTER, f.read(), 8).read());
        set_sampler_call(stage, 5, cell(T_FILTER, f.read(), 8).read());
        set_sampler_call(stage, 7, cell(T_FILTER + 4, f.read(), 8).read());
        set_sampler_call(stage, 1, table(T_ADDRESS, stage_field(stage, 4).read()));
        set_sampler_call(stage, 2, table(T_ADDRESS, stage_field(stage, 8).read()));
        set_sampler_call(stage, 4, stage_field(stage, 0x10).read());
        set_sampler_call(stage, 0xA, stage_field(stage, 0x14).read());
        if stage != 0 {
            set_tss_call(stage, 1, 1);
            set_tss_call(stage, 4, 1);
        }
    }
    set_render_state_call(0x13, table(T_BLEND, gu(M_SRC_BLEND)));
    set_render_state_call(0x14, table(T_BLEND, gu(M_DEST_BLEND)));
    set_render_state_call(0x19, table(T_CMP, gu(M_ALPHA_FUNC)));
    set_render_state_call(0x18, alpha_ref);
    if gu(M_VERTEX_ALPHA) != 0 {
        set_render_state_call(0x1B, 1);
        set_render_state_call(0xF, gu(M_ALPHA_TEST_WANTED));
    } else {
        set_render_state_call(0x1B, 0);
        set_render_state_call(0xF, 0);
    }
    su(M_TEXTURE_ALPHA, 0);
    set_render_state_call(0x89, 0);
    set_render_state_call(0x16, table(T_CULL, gu(M_CULL)));
    open_tail();
}

/// `_rwD3D9RWGetRenderState` (0x7FD810): RW state `s` (1..30) from the mirrors; returns 1, or 0 for
/// TEXTUREPERSPECTIVE / FOGTABLE / ALPHAPRIMITIVEBUFFER / unknown states (nothing written) and for a
/// TEXTUREADDRESS whose U and V differ (0 written).
#[hook(addr = 0x7FD810, conv = cdecl, name = "_rwD3D9RWGetRenderState", expect = "8B 44 24 04 48 83 F8 1D")]
pub fn rw_get_render_state(s: u32, out: GPtr<u32>) -> u32 {
    let src = match s {
        1 => STAGE as usize,
        2 => {
            let (u, v) = (gu(STAGE as usize + 4), gu(STAGE as usize + 8));
            if u == v {
                out.write(u);
                return 1;
            }
            out.write(0);
            return 0;
        }
        3 => STAGE as usize + 4,
        4 => STAGE as usize + 8,
        6 => M_ZTEST,
        7 => M_SHADE,
        8 => M_ZWRITE,
        9 => STAGE as usize + 0xC,
        10 => M_SRC_BLEND,
        11 => M_DEST_BLEND,
        12 => M_VERTEX_ALPHA,
        13 => STAGE as usize + 0x10,
        14 => M_FOG_ENABLE,
        15 => M_FOG_COLOR,
        16 => M_FOG_TYPE,
        17 => {
            // fld / fstp dword: a signalling NaN comes out quieted.
            out.cast::<f32>().write(x(g::<f32>(M_FOG_DENSITY).read()).to_f32());
            return 1;
        }
        20 => M_CULL,
        21 => M_STENCIL_ENABLE,
        22 => M_STENCIL_FAIL,
        23 => M_STENCIL_ZFAIL,
        24 => M_STENCIL_PASS,
        25 => M_STENCIL_FUNC,
        26 => M_STENCIL_REF,
        27 => M_STENCIL_MASK,
        28 => M_STENCIL_WMASK,
        29 => M_ALPHA_FUNC,
        30 => rs_value(0x18).addr(),
        _ => return 0,
    };
    out.write(gu(src));
    1
}

/// `FUN_007fdb80` (`_rwD3D9RenderStateTextureFilter`, stage 0): resets a stage-0 anisotropy above 1, then
/// on a filter change sets MIN/MAG directly and MIP only when its cached value differs. Returns 1.
#[hook(addr = 0x7FDB80, conv = cdecl, name = "FUN_007fdb80", expect = "83 3D 1C A5 C9 00 01")]
pub fn rs_texture_filter(f: u32) -> u32 {
    if (gu(STAGE as usize + 0x14) as i32) > 1 {
        su(STAGE as usize + 0x14, 1);
        samp(0, 0xA).write(1);
        dev_set_sampler_state(device(), 0, 0xA, 1);
    }
    if f != gu(STAGE as usize + 0xC) {
        let mip = cell(T_FILTER + 4, f, 8).read();
        let mm = cell(T_FILTER, f, 8).read();
        su(STAGE as usize + 0xC, f);
        sampler_direct(0, 6, mm);
        sampler_direct(0, 5, mm);
        if samp(0, 7).read() != mip {
            sampler_direct(0, 7, mip);
        }
    }
    1
}

/// `_rwD3D9RenderStateSrcBlend` (0x7FDC30).
#[hook(addr = 0x7FDC30, conv = cdecl, name = "_rwD3D9RenderStateSrcBlend", expect = "8B 4C 24 04 A1 C8 A5 C9 00")]
pub fn rs_src_blend(v: u32) -> u32 {
    if v != gu(M_SRC_BLEND) {
        rs_store(0x13, table(T_BLEND, v));
        su(M_SRC_BLEND, v);
    }
    1
}

/// `_rwD3D9RenderStateDestBlend` (0x7FDC80).
#[hook(addr = 0x7FDC80, conv = cdecl, name = "_rwD3D9RenderStateDestBlend", expect = "8B 4C 24 04 A1 CC A5 C9 00")]
pub fn rs_dest_blend(v: u32) -> u32 {
    if v != gu(M_DEST_BLEND) {
        rs_store(0x14, table(T_BLEND, v));
        su(M_DEST_BLEND, v);
    }
    1
}

/// The texture-alpha mirror turning on (stage 0, raster with alpha): with vertex alpha off, ALPHABLEND = 1
/// (stored unconditionally) and ALPHATEST = the alpha-test-wanted flag when it differs.
fn texture_alpha_on() {
    if gu(M_TEXTURE_ALPHA) != 0 {
        return;
    }
    su(M_TEXTURE_ALPHA, 1);
    if gu(M_VERTEX_ALPHA) != 0 {
        return;
    }
    rs_store(0x1B, 1);
    let w = gu(M_ALPHA_TEST_WANTED);
    if rs_value(0xF).read() != w {
        rs_store(0xF, w);
    }
}

/// The texture-alpha mirror turning off: with vertex alpha off, ALPHABLEND = 0 (unconditionally) and
/// ALPHATEST = 0 when it is not 0.
fn texture_alpha_off() {
    if gu(M_TEXTURE_ALPHA) == 0 {
        return;
    }
    su(M_TEXTURE_ALPHA, 0);
    if gu(M_VERTEX_ALPHA) != 0 {
        return;
    }
    rs_store(0x1B, 0);
    if rs_value(0xF).read() != 0 {
        rs_store(0xF, 0);
    }
}

/// `FUN_007fdcd0` (`_rwD3D9SetTextureInternal(raster, stage)`): stage 0 updates the texture-alpha mirror;
/// on a raster change `SetTexture(stage, ext->texture)` (+ `SetCurrentTexturePalette(palette+0x400)` when
/// the raster has a palette) or `SetTexture(stage, NULL)`. Returns 1.
#[hook(addr = 0x7FDCD0, conv = cdecl, name = "FUN_007fdcd0", expect = "8B 54 24 04 55 56 57")]
pub fn set_texture_internal(r: GPtr<u8>, stage: u32) -> u32 {
    let ext = if r.is_null() {
        GPtr::NULL
    } else {
        p8(gu(RASTER_EXT_OFFSET).wrapping_add(r.addr() as u32))
    };
    if stage == 0 {
        if !r.is_null() && rd::<u8>(ext, 8) != 0 {
            texture_alpha_on();
        } else {
            texture_alpha_off();
        }
    }
    let cur = stage_field(stage, 0);
    if cur.read() != r.addr() as u32 {
        cur.write(r.addr() as u32);
        if r.is_null() {
            dev_set_texture(device(), stage, 0);
        } else {
            dev_set_texture(device(), stage, rd::<u32>(ext, 0));
            let pal = rd::<u32>(ext, 4);
            if pal != 0 {
                dev_set_current_texture_palette(device(), rd::<u32>(p8(pal), 0x400));
            }
        }
    }
    1
}

/// `_rwD3D9GetCurrentTexture` (0x7FDE50): the stage's current raster.
#[hook(addr = 0x7FDE50, conv = cdecl, name = "_rwD3D9GetCurrentTexture", expect = "8B 44 24 04 8D 04 40")]
pub fn get_current_texture(stage: u32) -> u32 {
    stage_field(stage, 0).read()
}

/// `_rwD3D9SetMaxAnisotropy` (0x7FDE60): stores the RpAnisot plugin offset.
#[hook(addr = 0x7FDE60, conv = cdecl, name = "_rwD3D9SetMaxAnisotropy", expect = "8B 44 24 04 A3 E8 A5 C9 00")]
pub fn set_max_anisotropy(off: u32) {
    su(ANISOT_OFFSET, off);
}

/// `RwD3D9SetTexture` (0x7FDE70). With a texture: address U / V (direct, on change), anisotropy (always
/// sampler stage 0, original quirk; > 1 forces filter 7), filter (MIN/MAG/MIP direct on change), then
/// `_rwD3D9SetTextureInternal(t->raster, stage)`. Without: stage 0 clears the texture-alpha state, and
/// a stage with a raster gets `SetTexture(stage, NULL)`. Returns 1.
#[hook(addr = 0x7FDE70, conv = cdecl, name = "RwD3D9SetTexture", expect = "53 8B 5C 24 08 33 D2")]
pub fn set_texture(t: GPtr<u8>, stage: u32) -> u32 {
    if t.is_null() {
        if stage == 0 {
            texture_alpha_off();
        }
        let cur = stage_field(stage, 0);
        if cur.read() != 0 {
            cur.write(0);
            dev_set_texture(device(), stage, 0);
        }
        return 1;
    }
    let u = (rd::<u32>(t, 0x50) >> 8) & 0xF;
    if u != stage_field(stage, 4).read() {
        stage_field(stage, 4).write(u);
        sampler_direct(stage, 1, table(T_ADDRESS, u));
    }
    let v = (rd::<u32>(t, 0x50) >> 12) & 0xF;
    if v != stage_field(stage, 8).read() {
        stage_field(stage, 8).write(v);
        sampler_direct(stage, 2, table(T_ADDRESS, v));
    }
    let mut filter = rd::<u32>(t, 0x50) & 0xFF;
    let off = gu(ANISOT_OFFSET);
    if (off as i32) > 0 {
        let an = rd::<u8>(p8(off.wrapping_add(t.addr() as u32)), 0) as u32;
        if stage_field(stage, 0x14).read() != an {
            stage_field(stage, 0x14).write(an);
            samp(0, 0xA).write(an);
            dev_set_sampler_state(device(), 0, 0xA, an);
        }
        if (an as i32) > 1 {
            filter = 7;
        }
    }
    if stage_field(stage, 0xC).read() != filter {
        let mm = cell(T_FILTER, filter, 8).read();
        stage_field(stage, 0xC).write(filter);
        let mip = cell(T_FILTER + 4, filter, 8).read();
        sampler_direct(stage, 6, mm);
        sampler_direct(stage, 5, mm);
        sampler_direct(stage, 7, mip);
    }
    set_texture_internal_call(p8(rd::<u32>(t, 0)), stage);
    1
}

/// `_rwD3D9RenderStateVertexAlphaEnable` (0x7FE0A0): on a change of the mirror, with no texture alpha,
/// ALPHABLEND = v (stored unconditionally) and ALPHATEST = wanted flag / 0 when it differs. Returns 1.
#[hook(addr = 0x7FE0A0, conv = cdecl, name = "_rwD3D9RenderStateVertexAlphaEnable", expect = "8B 4C 24 04 33 D2 3B CA")]
pub fn rs_vertex_alpha_enable(v: u32) -> u32 {
    let m = gu(M_VERTEX_ALPHA);
    if v != 0 {
        if m != 0 {
            return 1;
        }
        su(M_VERTEX_ALPHA, 1);
        if gu(M_TEXTURE_ALPHA) != 0 {
            return 1;
        }
        rs_store(0x1B, 1);
        let w = gu(M_ALPHA_TEST_WANTED);
        if rs_value(0xF).read() != w {
            rs_store(0xF, w);
        }
    } else {
        if m == 0 {
            return 1;
        }
        su(M_VERTEX_ALPHA, 0);
        if gu(M_TEXTURE_ALPHA) != 0 {
            return 1;
        }
        rs_store(0x1B, 0);
        if rs_value(0xF).read() != 0 {
            rs_store(0xF, 0);
        }
    }
    1
}

/// `_rwD3D9RenderStateIsVertexAlphaEnable` (0x7FE190).
#[hook(addr = 0x7FE190, conv = cdecl, name = "_rwD3D9RenderStateIsVertexAlphaEnable", expect = "A1 E8 A4 C9 00 C3")]
pub fn rs_is_vertex_alpha_enable() -> u32 {
    gu(M_VERTEX_ALPHA)
}

/// The stencil / shade / cull handlers: on a mirror change, `rs[state] = mapped value` stored
/// unconditionally and queued, then the mirror. Return 1.
fn mirrored(mirror: usize, state: u32, v: u32, mapped: impl FnOnce(u32) -> u32) -> u32 {
    if v != gu(mirror) {
        rs_store(state, mapped(v));
        su(mirror, v);
    }
    1
}

/// `FUN_007fe1a0` (`_rwD3D9RenderStateStencilEnable`).
#[hook(addr = 0x7FE1A0, conv = cdecl, name = "FUN_007fe1a0", expect = "8B 4C 24 04 A1 C8 A4 C9 00")]
pub fn rs_stencil_enable(v: u32) -> u32 {
    mirrored(M_STENCIL_ENABLE, 0x34, v, |v| v)
}

/// `FUN_007fe1f0` (`_rwD3D9RenderStateStencilFail`).
#[hook(addr = 0x7FE1F0, conv = cdecl, name = "FUN_007fe1f0", expect = "8B 4C 24 04 A1 CC A4 C9 00")]
pub fn rs_stencil_fail(v: u32) -> u32 {
    mirrored(M_STENCIL_FAIL, 0x35, v, |v| table(T_STENCIL_OP, v))
}

/// `FUN_007fe240` (`_rwD3D9RenderStateStencilZFail`).
#[hook(addr = 0x7FE240, conv = cdecl, name = "FUN_007fe240", expect = "8B 4C 24 04 A1 D0 A4 C9 00")]
pub fn rs_stencil_zfail(v: u32) -> u32 {
    mirrored(M_STENCIL_ZFAIL, 0x36, v, |v| table(T_STENCIL_OP, v))
}

/// `FUN_007fe290` (`_rwD3D9RenderStateStencilPass`).
#[hook(addr = 0x7FE290, conv = cdecl, name = "FUN_007fe290", expect = "8B 4C 24 04 A1 D4 A4 C9 00")]
pub fn rs_stencil_pass(v: u32) -> u32 {
    mirrored(M_STENCIL_PASS, 0x37, v, |v| table(T_STENCIL_OP, v))
}

/// `FUN_007fe2e0` (`_rwD3D9RenderStateStencilFunction`).
#[hook(addr = 0x7FE2E0, conv = cdecl, name = "FUN_007fe2e0", expect = "8B 4C 24 04 A1 D8 A4 C9 00")]
pub fn rs_stencil_function(v: u32) -> u32 {
    mirrored(M_STENCIL_FUNC, 0x38, v, |v| table(T_CMP, v))
}

/// `FUN_007fe330` (`_rwD3D9RenderStateStencilRef`).
#[hook(addr = 0x7FE330, conv = cdecl, name = "FUN_007fe330", expect = "8B 4C 24 04 A1 DC A4 C9 00")]
pub fn rs_stencil_ref(v: u32) -> u32 {
    mirrored(M_STENCIL_REF, 0x39, v, |v| v)
}

/// `FUN_007fe380` (`_rwD3D9RenderStateStencilMask`).
#[hook(addr = 0x7FE380, conv = cdecl, name = "FUN_007fe380", expect = "8B 4C 24 04 A1 E0 A4 C9 00")]
pub fn rs_stencil_mask(v: u32) -> u32 {
    mirrored(M_STENCIL_MASK, 0x3A, v, |v| v)
}

/// `FUN_007fe3d0` (`_rwD3D9RenderStateStencilWriteMask`).
#[hook(addr = 0x7FE3D0, conv = cdecl, name = "FUN_007fe3d0", expect = "8B 4C 24 04 A1 E4 A4 C9 00")]
pub fn rs_stencil_write_mask(v: u32) -> u32 {
    mirrored(M_STENCIL_WMASK, 0x3B, v, |v| v)
}

/// TEXTUREADDRESSU of stage 0 on change (direct sampler state 1).
fn address_u0(v: u32) {
    if v != gu(STAGE as usize + 4) {
        let m = table(T_ADDRESS, v);
        su(STAGE as usize + 4, v);
        sampler_direct(0, 1, m);
    }
}

/// TEXTUREADDRESSV of stage 0 on change (direct sampler state 2).
fn address_v0(v: u32) {
    if v != gu(STAGE as usize + 8) {
        su(STAGE as usize + 8, v);
        let m = table(T_ADDRESS, v);
        sampler_direct(0, 2, m);
    }
}

/// `_rwD3D9RWSetRenderState` (0x7FE420): RW render state `s` (1..30, jump table 0x7FE9F8). `param` is the
/// value, or a pointer to the f32 for FOGDENSITY. Returns 0 for unknown states, FOGTABLE and
/// ALPHAPRIMITIVEBUFFER, `param != 0` for TEXTUREPERSPECTIVE, the handler's result or 1 otherwise.
/// (The original reuses its own argument slots as locals for FOGTYPE / FOGDENSITY: not reproduced.)
#[hook(addr = 0x7FE420, conv = cdecl, name = "_rwD3D9RWSetRenderState", expect = "8B 4C 24 04 53 33 DB")]
pub fn rw_set_render_state(s: u32, param: u32) -> u32 {
    match s {
        1 => set_texture_internal_call(p8(param), 0),
        2 => {
            address_u0(param);
            address_v0(param);
            1
        }
        3 => {
            address_u0(param);
            1
        }
        4 => {
            address_v0(param);
            1
        }
        5 => (param != 0) as u32,
        6 => rs_ztest_call(param),
        7 => rs_shade_call(param),
        8 => rs_zwrite_call(param),
        9 => rs_texture_filter_call(param),
        10 => {
            if param != gu(M_SRC_BLEND) {
                rs_store(0x13, table(T_BLEND, param));
                su(M_SRC_BLEND, param);
            }
            1
        }
        11 => {
            if param != gu(M_DEST_BLEND) {
                rs_store(0x14, table(T_BLEND, param));
                su(M_DEST_BLEND, param);
            }
            1
        }
        12 => rs_vertex_alpha_call(param),
        13 => rs_border_call(param),
        14 => {
            let m = gu(M_FOG_ENABLE);
            if param != 0 {
                if m == 0 && gu(CAPS_RASTER) & 0x180 != 0 {
                    rs_store(0x1C, 1);
                    su(M_FOG_ENABLE, 1);
                }
            } else if m != 0 {
                rs_store(0x1C, 0);
                su(M_FOG_ENABLE, 0);
            }
            1
        }
        15 => {
            if param != gu(M_FOG_COLOR) {
                rs_store(0x22, param);
                su(M_FOG_COLOR, param);
            }
            1
        }
        16 => {
            if param != gu(M_FOG_TYPE) {
                let caps = gu(CAPS_RASTER);
                if caps & 0x100 != 0 && caps & 0x10_0000 != 0 {
                    rs_store(0x23, table(T_FOG, param));
                    if rs_value(0x8C).read() != 0 {
                        rs_store(0x8C, 0);
                    }
                } else {
                    if rs_value(0x23).read() != 0 {
                        rs_store(0x23, 0);
                    }
                    rs_store(0x8C, table(T_FOG, param));
                }
                su(M_FOG_TYPE, param);
            }
            if param == 1 {
                let c = p8(gu(CUR_CAMERA));
                let fog_plane = rd::<u32>(c, 0x88);
                let far_plane = rd::<u32>(c, 0x84);
                rs_set(0x24, fog_plane);
                rs_set(0x25, far_plane);
            }
            1
        }
        17 => {
            // x87 compare: unordered counts as different, +0 == -0.
            let d = g::<u32>(param as usize).read();
            if f32::from_bits(d) != f32::from_bits(gu(M_FOG_DENSITY)) {
                rs_store(0x26, d);
                su(M_FOG_DENSITY, d);
            }
            1
        }
        20 => rs_cull_call(param),
        21 => rs_stencil_enable_call(param),
        22 => rs_stencil_fail_call(param),
        23 => rs_stencil_zfail_call(param),
        24 => rs_stencil_pass_call(param),
        25 => rs_stencil_func_call(param),
        26 => rs_stencil_ref_call(param),
        27 => rs_stencil_mask_call(param),
        28 => rs_stencil_wmask_call(param),
        29 => {
            if param != gu(M_ALPHA_FUNC) {
                su(M_ALPHA_FUNC, param);
                let wanted = (param != 8) as u32;
                su(M_ALPHA_TEST_WANTED, wanted);
                if rs_value(0x1B).read() != 0 {
                    set_render_state_call(0xF, wanted);
                }
                rs_store(0x19, table(T_CMP, param));
            }
            1
        }
        30 => {
            set_render_state_call(0x18, param);
            1
        }
        _ => 0,
    }
}

/// `FUN_007fea70` (`_rwD3D9RenderStateZTestEnable` in rw-names; implements ZWRITEENABLE): on a mirror
/// change, ZENABLE follows when the ZTEST mirror is off, ZWRITEENABLE = v (both stored unconditionally).
#[hook(addr = 0x7FEA70, conv = cdecl, name = "FUN_007fea70", expect = "8B 44 24 04 33 C9 3B C1")]
pub fn rs_zwrite_enable(v: u32) -> u32 {
    let on = (v != 0) as u32;
    if (gu(M_ZWRITE) != 0) as u32 == on {
        return 1;
    }
    if gu(M_ZTEST) == 0 {
        rs_store(7, on);
    }
    rs_store(0xE, on);
    su(M_ZWRITE, on);
    1
}

/// `FUN_007feb70` (`_rwD3D9RenderStateZWriteEnable` in rw-names; implements ZTESTENABLE): on a mirror
/// change, ZENABLE follows when the ZWRITE mirror is off, ZFUNC = 4 (LESSEQUAL) / 8 (ALWAYS).
#[hook(addr = 0x7FEB70, conv = cdecl, name = "FUN_007feb70", expect = "8B 44 24 04 33 C9 3B C1")]
pub fn rs_ztest_enable(v: u32) -> u32 {
    let on = (v != 0) as u32;
    if (gu(M_ZTEST) != 0) as u32 == on {
        return 1;
    }
    if gu(M_ZWRITE) == 0 {
        rs_store(7, on);
    }
    rs_store(0x17, if on != 0 { 4 } else { 8 });
    su(M_ZTEST, on);
    1
}

/// `FUN_007fec70` (`_rwD3D9RenderStateShadeMode`).
#[hook(addr = 0x7FEC70, conv = cdecl, name = "FUN_007fec70", expect = "8B 4C 24 04 A1 04 A5 C9 00")]
pub fn rs_shade_mode(v: u32) -> u32 {
    mirrored(M_SHADE, 9, v, |v| table(T_SHADE, v))
}

/// `_rwD3D9RenderStateTextureBorderColor` (0x7FECC0): stage 0, direct sampler state 4 on change.
#[hook(addr = 0x7FECC0, conv = cdecl, name = "_rwD3D9RenderStateTextureBorderColor", expect = "8B 44 24 04 8B 0D 18 A5 C9 00")]
pub fn rs_texture_border_color(c: u32) -> u32 {
    if c != gu(STAGE as usize + 0x10) {
        su(STAGE as usize + 0x10, c);
        sampler_direct(0, 4, c);
    }
    1
}

/// `FUN_007fed00` (`_rwD3D9RenderStateCullMode`).
#[hook(addr = 0x7FED00, conv = cdecl, name = "FUN_007fed00", expect = "8B 4C 24 04 A1 F0 A4 C9 00")]
pub fn rs_cull_mode(v: u32) -> u32 {
    mirrored(M_CULL, 0x16, v, |v| table(T_CULL, v))
}

/// `1 << (n & 31)` with `n` = index of the highest set bit (-1 for 0: gives 0x80000000; a negative value
/// never terminates, as in the original).
fn pow2_floor(v: i32) -> i32 {
    let mut n: i32 = -1;
    let mut t = v;
    while t != 0 {
        n = n.wrapping_add(1);
        t >>= 1;
    }
    1i32.wrapping_shl((n & 31) as u32)
}

/// `FUN_007fed70` (`_rwD3D9ClampRasterSize(w*, h*, flags)`): clamps to the max texture size (signed),
/// rounds down to powers of two as the caps require, and squares (min) with SQUAREONLY.
#[hook(addr = 0x7FED70, conv = cdecl, name = "FUN_007fed70", expect = "8B 54 24 04 A1 58 BF C9 00")]
pub fn clamp_raster_size(w: GPtr<i32>, h: GPtr<i32>, flags: u32) {
    let mw = gu(CAPS_MAX_TEX_W) as i32;
    if w.read() > mw {
        w.write(mw);
    }
    let mh = gu(CAPS_MAX_TEX_H) as i32;
    if h.read() > mh {
        h.write(mh);
    }
    let round = if flags & 7 != 0 && flags & 0x8000 != 0 {
        gu(CAPS_TEXTURE) & 2 != 0
    } else {
        let caps = gu(CAPS_TEXTURE);
        caps & 2 != 0 && caps & 0x100 == 0
    };
    if round {
        w.write(pow2_floor(w.read()));
        h.write(pow2_floor(h.read()));
    }
    if gu(CAPS_TEXTURE) & 0x20 != 0 {
        let (a, b) = (w.read(), h.read());
        if a < b {
            h.write(a);
        } else {
            w.write(b);
        }
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_FLUSH_CACHE,
    HOOK_SET_RENDER_STATE,
    HOOK_GET_RENDER_STATE,
    HOOK_SET_TEXTURE_STAGE_STATE,
    HOOK_GET_TEXTURE_STAGE_STATE,
    HOOK_SET_SAMPLER_STATE,
    HOOK_GET_SAMPLER_STATE,
    HOOK_SET_MATERIAL,
    HOOK_SET_CLIP_PLANE,
    HOOK_SET_SURFACE_PROPERTIES,
    HOOK_RENDER_STATE_OPEN,
    HOOK_RENDER_STATE_RESET,
    HOOK_RW_GET_RENDER_STATE,
    HOOK_RS_TEXTURE_FILTER,
    HOOK_RS_SRC_BLEND,
    HOOK_RS_DEST_BLEND,
    HOOK_SET_TEXTURE_INTERNAL,
    HOOK_GET_CURRENT_TEXTURE,
    HOOK_SET_MAX_ANISOTROPY,
    HOOK_SET_TEXTURE,
    HOOK_RS_VERTEX_ALPHA_ENABLE,
    HOOK_RS_IS_VERTEX_ALPHA_ENABLE,
    HOOK_RS_STENCIL_ENABLE,
    HOOK_RS_STENCIL_FAIL,
    HOOK_RS_STENCIL_ZFAIL,
    HOOK_RS_STENCIL_PASS,
    HOOK_RS_STENCIL_FUNCTION,
    HOOK_RS_STENCIL_REF,
    HOOK_RS_STENCIL_MASK,
    HOOK_RS_STENCIL_WRITE_MASK,
    HOOK_RW_SET_RENDER_STATE,
    HOOK_RS_ZWRITE_ENABLE,
    HOOK_RS_ZTEST_ENABLE,
    HOOK_RS_SHADE_MODE,
    HOOK_RS_TEXTURE_BORDER_COLOR,
    HOOK_RS_CULL_MODE,
    HOOK_CLAMP_RASTER_SIZE,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
