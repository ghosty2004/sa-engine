//! `CCustomBuildingDNPipeline::CustomPipeRenderCB` and the two instance-data draw helpers it calls (batch
//! `std9-render-4`): ported from the 1.0 US bytes (the render callback is not reversed in gta-reversed). Unverified
//! (docs/verify/HOWTO.md).
//!
//! `RxD3D9ResEntryHeader` / `RxD3D9InstanceData` layouts as in sa-rwcore's `d3d9_mesh_instance.rs`.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{f32_of, ftol32, g, rd, xi};

orig! {
    /// `_rwD3D9EnableClippingIfNeeded(object, type)` (0x756D90)
    fn enable_clipping_if_needed(object: GPtr<u8>, ty: u32) = cdecl 0x756D90 => unported;
    /// `RwD3D9GetRenderState(state, void* value)` (0x7FC320)
    fn get_render_state(state: u32, value: GPtr<u32>) = cdecl 0x7FC320 => unported;
    /// `RwD3D9SetRenderState(state, value)` (0x7FC2D0)
    fn set_render_state(state: u32, value: u32) -> u32 = cdecl 0x7FC2D0 => unported;
    /// `RwD3D9SetTextureStageState(stage, type, value)` (0x7FC340)
    fn set_tss(stage: u32, ty: u32, value: u32) = cdecl 0x7FC340 => unported;
    /// `RwD3D9SetTexture(texture, stage)` (0x7FDE70)
    fn set_texture(t: u32, stage: u32) -> u32 = cdecl 0x7FDE70 => unported;
    /// `_rwD3D9SetIndices(ib)` (0x7FA1C0)
    fn set_indices(ib: u32) = cdecl 0x7FA1C0 => unported;
    /// `_rwD3D9SetStreams(streams, useOffsets)` (0x7FA090)
    fn set_streams(streams: GPtr<u8>, use_offsets: u32) = cdecl 0x7FA090 => unported;
    /// `_rwD3D9SetVertexDeclaration(decl)` (0x7F9F70)
    fn set_vertex_declaration(decl: u32) = cdecl 0x7F9F70 => unported;
    /// `G->dOpenDevice.fpRenderStateSet(state, value)` (`[G] + 0x20`)
    fn rw_render_state_set(state: u32, value: u32) -> u32 = cdecl table 0xC97B24 0x20 => unported;
    /// `_rwD3D9RenderStateVertexAlphaEnable(enable)` (0x7FE0A0)
    fn vertex_alpha_enable(on: u32) = cdecl 0x7FE0A0 => unported;
    /// `RwD3D9SetSurfaceProperties(surfaceProps, color, flags)` (0x7FC4D0)
    fn set_surface_properties(surf: GPtr<u8>, color: GPtr<u8>, flags: u32) -> u32 = cdecl 0x7FC4D0 => unported;
    /// `_rwD3D9SetPixelShader(shader)` (0x7F9FF0) / `_rwD3D9SetVertexShader(shader)` (0x7F9FB0)
    fn set_pixel_shader(s: u32) = cdecl 0x7F9FF0 => unported;
    fn set_vertex_shader(s: u32) = cdecl 0x7F9FB0 => unported;
    /// `_rwD3D9DrawIndexedPrimitive(type, baseIndex, minIndex, numVertices, startIndex, primCount)` (0x7FA320)
    fn draw_indexed_primitive(ty: u32, base: u32, min: u32, nv: u32, start: u32, count: u32) = cdecl 0x7FA320 => unported;
    /// `_rwD3D9DrawPrimitive(type, startVertex, primCount)` (0x7FA360)
    fn draw_primitive(ty: u32, start: u32, count: u32) = cdecl 0x7FA360 => unported;
    /// `RxD3D9InstanceDataRender(header, inst)` (0x5DA640)
    fn instance_data_render_call(h: GPtr<u8>, inst: GPtr<u8>) = cdecl 0x5DA640 => instance_data_render;
    /// `RxD3D9InstanceDataRenderLighting(header, inst, flags, texture)` (0x5DA6A0)
    fn instance_data_render_lighting_call(h: GPtr<u8>, inst: GPtr<u8>, flags: u32, tex: u32) = cdecl 0x5DA6A0 => instance_data_render_lighting;
}

/// The DN material plugin offset (`[0x8D12C4]`) and the texture-transform matrix cells it fills.
const DN_PLUGIN_OFFSET: usize = 0x8D12C4;
const UV_MATRIX_U: usize = 0xC02C28;
const UV_MATRIX_V: usize = 0xC02C3C;
const UV_MATRIX_ONE_A: usize = 0xC02C64;
const UV_MATRIX_ONE_B: usize = 0xC02C50;

/// The shared draw: pixel shader 0, the instance's vertex shader (+0x10), then an indexed draw (`primType` +0xC,
/// base +0x14, min 0, vertices +0x18, start +0x1C, primitives +0x20) when the header has an index buffer (+8), else a
/// plain draw (base +0x14, primitives +0x20).
fn draw(h: GPtr<u8>, inst: GPtr<u8>) {
    set_pixel_shader(0);
    set_vertex_shader(rd::<u32>(inst, 0x10));
    let prim = rd::<u32>(h, 0xC);
    if rd::<u32>(h, 8) != 0 {
        draw_indexed_primitive(prim, rd(inst, 0x14), 0, rd(inst, 0x18), rd(inst, 0x1C), rd(inst, 0x20));
    } else {
        draw_primitive(prim, rd(inst, 0x14), rd(inst, 0x20));
    }
}

/// `RxD3D9InstanceDataRender(header, inst)` (0x5DA640, cdecl): the shared draw.
#[hook(addr = 0x5DA640, conv = cdecl, name = "RxD3D9InstanceDataRender", expect = "56 6A 00 E8 A8 F9 21 00",
       unverified = "std9-render-4", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn instance_data_render(h: GPtr<u8>, inst: GPtr<u8>) {
    draw(h, inst);
}

/// `RxD3D9InstanceDataRenderLighting(header, inst, flags, texture)` (0x5DA6A0, cdecl): with flags 0x84 (textured)
/// the texture in stage 0 and stage-0 states (COLOROP MODULATE, COLORARG1 TEXTURE, COLORARG2 DIFFUSE, ALPHAOP MODULATE,
/// ALPHAARG1 TEXTURE, ALPHAARG2 DIFFUSE); otherwise no texture and SELECTARG2 of DIFFUSE for both; then the shared
/// draw.
#[hook(addr = 0x5DA6A0, conv = cdecl, name = "RxD3D9InstanceDataRenderLighting", expect = "F6 44 24 0C 84 6A 00 74",
       unverified = "std9-render-4", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn instance_data_render_lighting(h: GPtr<u8>, inst: GPtr<u8>, flags: u32, tex: u32) {
    if flags & 0x84 != 0 {
        set_texture(tex, 0);
        for (t, v) in [(1, 4), (2, 2), (3, 0), (4, 4), (5, 2), (6, 0)] {
            set_tss(0, t, v);
        }
    } else {
        set_texture(0, 0);
        for (t, v) in [(1, 3), (3, 0), (4, 3), (6, 0)] {
            set_tss(0, t, v);
        }
    }
    draw(h, inst);
}

/// `CCustomBuildingDNPipeline::CustomPipeRenderCB(RwResEntry*, void* object, RwUInt8 type, RwUInt32 flags)`
/// (0x5D6480, cdecl). Clipping as needed; with lighting off and no prelight (flags bit 3) the texture factor black
/// is the colour (no texture, SELECTARG2 TFACTOR for colour and alpha) and the meshes are drawn without lighting
/// setup. Indices, streams, vertex declaration; per mesh: stages 1 and 2 disabled; a material whose surface byte
/// +0x10 has bit 0 gets the DN plugin's second texture in stage 1 (texture address wrap, the UV offsets `u, v *
/// 0.125` into the transform matrix, the factor `min(ftol(a / 255 * 254), 255)` grey with full alpha and the stage-1
/// blend states); vertex alpha from the instance (+0xC) or the material alpha; then
/// `RxD3D9InstanceDataRenderLighting` (with `RwD3D9SetSurfaceProperties` when lit) or `RxD3D9InstanceDataRender`.
/// Stage 1 is reset at the end.
#[hook(addr = 0x5D6480, conv = cdecl, name = "CCustomBuildingDNPipeline::CustomPipeRenderCB", expect = "51 0F B6 44 24 10 8B 4C",
       unverified = "std9-render-4", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn building_dn_render_cb(entry: GPtr<u8>, object: GPtr<u8>, ty: u32, flags: u32) {
    enable_clipping_if_needed(object, ty & 0xFF);
    let mut lighting = 0u32;
    get_render_state(0x89, GPtr::from_raw(&raw mut lighting));
    let unlit = lighting == 0 && flags & 8 == 0;
    if unlit {
        set_texture(0, 0);
        set_render_state(0x3C, 0xFF00_0000);
        for t in [1, 3, 4, 6] {
            set_tss(0, t, 3);
        }
    }
    let h = entry.byte_add(0x18);
    let ib = rd::<u32>(h, 8);
    if ib != 0 {
        set_indices(ib);
    }
    set_streams(h.byte_add(0x10), rd::<u32>(h, 0x30));
    set_vertex_declaration(rd::<u32>(h, 0x34));
    let n = rd::<i32>(h, 4);
    let mut inst = h.byte_add(0x40);
    for _ in 0..n.max(0) {
        let mat = rd::<GPtr<u8>>(inst, 8);
        let dual = rd::<u8>(mat, 0x10) & 1 != 0;
        let plug = rd::<GPtr<u8>>(mat, g::<u32>(DN_PLUGIN_OFFSET).read() as isize);
        set_tss(1, 1, 1);
        set_tss(2, 1, 1);
        if dual {
            rw_render_state_set(2, 1);
            let k = g::<f32>(0x858C48).read();
            g::<f32>(UV_MATRIX_U).write(f32_of(xi(i32::from(rd::<i8>(plug, 0))).fmul_m(k)));
            g::<u32>(UV_MATRIX_ONE_A).write(0x3F80_0000);
            g::<u32>(UV_MATRIX_ONE_B).write(0x3F80_0000);
            g::<f32>(UV_MATRIX_V).write(f32_of(xi(i32::from(rd::<i8>(plug, 1))).fmul_m(k)));
            set_texture(rd::<u32>(plug, 8), 1);
            let a = xi(i32::from(rd::<u8>(plug, 4))).fmul_m(g::<f32>(0x859A3C).read()).fmul_m(g::<f32>(0x86BE90).read());
            let mut v = ftol32(a);
            if v > 0xFF {
                v = 0xFF;
            }
            let a = (v as u32) & 0xFF;
            set_render_state(0x3C, (((a | 0xFFFF_FF00) << 8) | a) << 8 | a);
            for (t, val) in [(1, 0x19), (0x1A, 1), (2, 2), (3, 0), (4, 3), (5, 2), (6, 1), (0xB, 0x1_0001), (0x18, 0)] {
                set_tss(1, t, val);
            }
            set_tss(2, 1, 1);
        }
        let alpha = rd::<u32>(inst, 0xC) != 0 || rd::<u8>(rd::<GPtr<u8>>(inst, 8), 7) != 0xFF;
        vertex_alpha_enable(u32::from(alpha));
        if !unlit {
            let mat = rd::<GPtr<u8>>(inst, 8);
            if lighting != 0 {
                set_surface_properties(mat.byte_add(0xC), mat.byte_add(4), flags);
            }
            instance_data_render_lighting_call(h, inst, flags, rd::<u32>(rd::<GPtr<u8>>(inst, 8), 0));
        } else {
            instance_data_render_call(h, inst);
        }
        inst = inst.byte_add(0x24);
    }
    for (t, v) in [(1, 1), (4, 1), (0xB, 1), (0x18, 0)] {
        set_tss(1, t, v);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_INSTANCE_DATA_RENDER, HOOK_INSTANCE_DATA_RENDER_LIGHTING, HOOK_BUILDING_DN_RENDER_CB];
