//! The MatFX D3D9 environment-map render of one mesh and its env texture / texture-transform setup.
//!
//! A MatFX effect slot (`material + [0xC9AB74]`, 0x18 bytes per pass): +0 frame, +4 texture, +8
//! coefficient, +0xC useFrameBufferAlpha.

use sa_core::abi::HookReg;
use sa_core::input::rw_render_state_set;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{flush_cache, frame_get_ltm, rw_matrix_invert, rw_matrix_multiply, vertex_alpha_enable};
use crate::common::{g, p8, rd};
use crate::generated::generated_leaves::g_fun_004c9f70_4c9f70 as raster_is_cube_map;
use crate::d3d9dev::{dev_set_pixel_shader, dev_set_ps_constant, dev_set_vertex_shader};
use crate::d3d9inst::fist;
use crate::d3d9rast::raster_convert_to_non_palettized;
use crate::d3d9state::{
    device, get_render_state, rs_dest_blend, rs_is_vertex_alpha_enable, rs_src_blend, set_render_state, set_texture,
    set_texture_stage_state as tss,
};
use crate::frontier_b2::flush_draw_mesh;
use crate::matfx::{matfx_default_render, MATFX_MATERIAL_OFFSET};
use crate::raster::d3d9_raster_has_alpha;

/// The vertex / pixel shaders last set on the device.
const LAST_VERTEX_SHADER: usize = 0x8E_2448;
const LAST_PIXEL_SHADER: usize = 0x8E_244C;
/// The env-map pixel shaders: over the base texture, and alone.
const PS_ENV_TEXTURED: usize = 0xC9_AB98;
const PS_ENV: usize = 0xC9_AB9C;
/// Forces the two-stage env blend.
const FORCE_TWO_STAGE: usize = 0xC9_ABB0;
/// The device validated the two-stage TFACTOR-less blend in one pass; the validation is still due.
const TWO_STAGE_VALID: usize = 0xC9_ABB8;
const TWO_STAGE_VALIDATE: usize = 0x8E_29C4;
/// The texture-space scale / bias applied after the camera-relative env transform.
const ENV_TEXTURE_MATRIX: usize = 0x88_5608;
const D3DTS_TEXTURE0: u32 = 0x10;
const D3DTSS_TEXTURETRANSFORMFLAGS: u32 = 0x18;
const D3DTSS_TEXCOORDINDEX: u32 = 0xB;
const D3DRS_TEXTUREFACTOR: u32 = 0x3C;

orig! {
    /// `IDirect3DDevice9::ValidateDevice(&passes)` (+0x118)
    pub(crate) fn dev_validate_device(dev: GPtr<u8>, passes: GPtr<u32>) -> i32 = stdcall vslot 0x118 => unported;
    /// `RwRenderStateGet(state, &value)` (G+0x24)
    pub(crate) fn rw_render_state_get(state: u32, out: GPtr<u32>) -> u32 = cdecl table 0xC97B24 0x24 => unported;
}

fn set_pixel_shader(ps: u32) {
    if g::<u32>(LAST_PIXEL_SHADER).read() != ps {
        g::<u32>(LAST_PIXEL_SHADER).write(ps);
        dev_set_pixel_shader(device(), ps);
    }
}

fn stage_states(stage: u32, states: &[(u32, u32)]) {
    for &(ty, v) in states {
        tss(stage, ty, v);
    }
}

/// The env stage's texture, its coordinate source and transform reset after the draw.
fn finish(stage: u32) {
    set_texture(GPtr::NULL, stage);
    tss(stage, D3DTSS_TEXTURETRANSFORMFLAGS, 0);
    tss(stage, D3DTSS_TEXCOORDINDEX, stage);
}

fn dwords(m: &mut [u32; 16]) -> GPtr<u8> {
    GPtr::from_raw(m.as_mut_ptr()).cast()
}

/// `_rpMatFXD3D9SetEnvMapTexture(texture, stage, frame)` (0x814270): the env texture on `stage` with
/// camera-space reflection coordinates through the camera's axes (cube maps), or camera-space normals
/// through `frame` relative to the camera (identity scale / bias without a frame). The frame path's
/// matrices keep the never-written pads of their stack locals, which the transform cache copies.
#[hook(addr = 0x814270, conv = cdecl, name = "FUN_00814270", expect = "81 EC C0 00 00 00 53 56", peek = 0xC0,
       unverified = "frontier2-c32", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the matrix pads (stack residue) reach RwD3D9SetTransform's cache; called from Rust they start at 0")]
pub fn matfx_set_env_map_texture(tex: GPtr<u8>, stage: u32, frame: GPtr<u8>) -> u32 {
    let residue = |off: i32| sa_core::abi::peek_stack(0x814270, off).unwrap_or(0);
    let local = |base: i32| core::array::from_fn::<u32, 16, _>(|i| residue(base + 4 * i as i32));
    set_texture(tex, stage);
    let camera_frame = || p8(rd::<u32>(p8(rd::<u32>(p8(g::<u32>(0xC9_7B24).read()), 0)), 4));
    if raster_is_cube_map(rd::<u32>(tex, 0)) != 0 {
        tss(stage, D3DTSS_TEXTURETRANSFORMFLAGS, 3);
        tss(stage, D3DTSS_TEXCOORDINDEX, 0x3_0000);
        let cam = frame_get_ltm(camera_frame());
        let mut m = [0u32; 16];
        for i in 0..3 {
            m[i] = (-rd::<f32>(cam, 4 * i as isize)).to_bits();
            m[4 + i] = rd::<u32>(cam, 0x10 + 4 * i as isize);
            m[8 + i] = rd::<u32>(cam, 0x20 + 4 * i as isize);
        }
        m[15] = 1.0f32.to_bits();
        crate::d3d9dev::set_transform(stage + D3DTS_TEXTURE0, dwords(&mut m));
        return 1;
    }
    tss(stage, D3DTSS_TEXTURETRANSFORMFLAGS, 2);
    tss(stage, D3DTSS_TEXCOORDINDEX, 0x1_0000);
    if frame.is_null() {
        crate::d3d9dev::set_transform(stage + D3DTS_TEXTURE0, p8(ENV_TEXTURE_MATRIX as u32));
        return 1;
    }
    let cam = frame_get_ltm(camera_frame());
    let env = frame_get_ltm(frame);
    let (mut m, mut inv, mut out) = (local(-0xC0), local(-0x80), local(-0x40));
    rw_matrix_invert(dwords(&mut inv), env);
    rw_matrix_multiply(dwords(&mut m), dwords(&mut inv), cam);
    for v in &mut m[..3] {
        *v = (-f32::from_bits(*v)).to_bits();
    }
    m[3] = 0;
    m[12] = 0;
    m[13] = 0;
    m[14] = 0;
    rw_matrix_multiply(dwords(&mut out), dwords(&mut m), p8(ENV_TEXTURE_MATRIX as u32));
    crate::d3d9dev::set_transform(stage + D3DTS_TEXTURE0, dwords(&mut out));
    1
}

/// The env pass blended over what is drawn (SRCALPHA / ONE, the shininess as TEXTUREFACTOR unless
/// full), lighting, z writes and fog off for the draw; every state restored afterwards.
fn blend_env_pass(header: GPtr<u8>, mesh: GPtr<u8>, env: GPtr<u8>, frame: GPtr<u8>, shininess: u32) {
    matfx_set_env_map_texture(env, 0, frame);
    if rs_is_vertex_alpha_enable() == 0 {
        vertex_alpha_enable(1);
    }
    let (mut src, mut dst) = (0u32, 0u32);
    rw_render_state_get(10, GPtr::from_raw(&mut src));
    rw_render_state_get(11, GPtr::from_raw(&mut dst));
    rs_src_blend(5);
    rs_dest_blend(2);
    let factor = shininess.wrapping_mul(0x0101_0101);
    if factor != u32::MAX {
        set_render_state(D3DRS_TEXTUREFACTOR, factor);
        stage_states(0, &[(1, 4), (2, 2), (3, 3), (4, 2), (5, 2), (6, 3)]);
    } else {
        stage_states(0, &[(1, 2), (2, 2), (4, 2), (5, 2)]);
    }
    let (mut lighting, mut zwrite, mut fog, mut fog_colour) = (0u32, 0u32, 0u32, 0u32);
    get_render_state(0x89, GPtr::from_raw(&mut lighting));
    get_render_state(0xE, GPtr::from_raw(&mut zwrite));
    get_render_state(0x1C, GPtr::from_raw(&mut fog));
    set_render_state(0x89, 0);
    set_render_state(0xE, 0);
    if fog != 0 {
        get_render_state(0x22, GPtr::from_raw(&mut fog_colour));
        set_render_state(0x22, 0);
    }
    flush_draw_mesh(header, mesh);
    if fog != 0 {
        set_render_state(0x22, fog_colour);
    }
    set_render_state(0xE, zwrite);
    set_render_state(0x89, lighting);
    if factor != u32::MAX {
        stage_states(0, &[(3, 0), (6, 0)]);
    } else {
        stage_states(0, &[(1, 4), (4, 4)]);
    }
    vertex_alpha_enable(0);
    rw_render_state_set(10, src);
    rw_render_state_set(11, dst);
    tss(0, D3DTSS_TEXTURETRANSFORMFLAGS, 0);
    tss(0, D3DTSS_TEXCOORDINDEX, 0);
}

/// `_rpMatFXD3D9RenderEnvMap(header, mesh, flags, pass, texture, envTexture)` (0x813A10): a zero
/// shininess (coefficient * 255) or a missing env raster falls back to the default render (pass 0
/// only). Pass 0 draws the base texture and puts the env map on top in one draw where it can (the env
/// pixel shaders for an env map with alpha, or a second texture stage), else blends a second draw;
/// later passes only blend.
#[hook(addr = 0x813A10, conv = cdecl, name = "FUN_00813a10", expect = "83 EC 10 8B 15 74 AB C9 00",
       unverified = "frontier2-c32", route = "L2", hazards = "heap-writes",
       note = "keeps its flags, the blend states and render states in its own argument slots (not reproduced)")]
pub fn matfx_render_env(header: GPtr<u8>, mesh: GPtr<u8>, flags: u32, pass: u32, tex: GPtr<u8>, env: GPtr<u8>) {
    let material = p8(rd::<u32>(mesh, 8));
    let fx = p8(rd::<u32>(material, g::<i32>(MATFX_MATERIAL_OFFSET).read() as isize));
    let slot = fx.byte_add(pass.wrapping_mul(0x18) as isize);
    let shininess = fist(rd::<f32>(slot, 8) * 255.0) as u32 & 0xFF;
    if shininess == 0 || env.is_null() || rd::<u32>(env, 0) == 0 {
        if pass == 0 {
            matfx_default_render(header, mesh, flags, tex.addr() as u32);
        }
        return;
    }
    let vs = rd::<u32>(mesh, 0x10);
    if g::<u32>(LAST_VERTEX_SHADER).read() != vs {
        g::<u32>(LAST_VERTEX_SHADER).write(vs);
        dev_set_vertex_shader(device(), rd(mesh, 0x10));
    }
    let frame = || p8(rd::<u32>(slot, 0));
    if pass == 0 {
        let mut textured = false;
        let mut two_stage = false;
        if flags as u8 & 0x84 != 0 && !tex.is_null() {
            set_texture(tex, 0);
            textured = true;
            let env_raster = p8(rd::<u32>(env, 0));
            if rd::<u8>(p8(rd::<u32>(tex, 0)), 0x23) & 0x20 != 0 && rd::<u8>(env_raster, 0x23) & 0x20 != 0 && tex != env {
                raster_convert_to_non_palettized(env_raster);
            }
        } else {
            set_texture(GPtr::NULL, 0);
        }
        let opaque = rd::<u32>(slot, 0xC) != 0
            || (rd::<u32>(mesh, 0xC) == 0
                && rd::<u8>(p8(rd::<u32>(mesh, 8)), 7) == 0xFF
                && !(textured && d3d9_raster_has_alpha(tex) != 0));
        if opaque {
            if d3d9_raster_has_alpha(env) != 0 {
                if g::<u32>(PS_ENV_TEXTURED).read() != 0 {
                    let c = rd::<u32>(slot, 8);
                    let mut consts = [c; 4];
                    dev_set_ps_constant(device(), 0, GPtr::from_raw(consts.as_mut_ptr()).addr() as u32, 1);
                    let stage = u32::from(textured);
                    matfx_set_env_map_texture(env, stage, frame());
                    set_pixel_shader(g::<u32>(if textured { PS_ENV_TEXTURED } else { PS_ENV }).read());
                    flush_draw_mesh(header, mesh);
                    finish(stage);
                    return;
                }
            } else if g::<u32>(FORCE_TWO_STAGE).read() != 0
                || (shininess == 0xFF
                    && (!textured || g::<u32>(TWO_STAGE_VALID).read() != 0 || g::<u32>(TWO_STAGE_VALIDATE).read() != 0))
            {
                two_stage = true;
            }
        }
        set_pixel_shader(0);
        if textured {
            stage_states(0, &[(1, 4), (2, 2), (3, 0), (4, 4), (5, 2), (6, 0)]);
        } else {
            stage_states(0, &[(1, 3), (3, 0), (4, 3), (6, 0)]);
        }
        if two_stage {
            if shininess == 0xFF && !textured {
                matfx_set_env_map_texture(env, 0, frame());
                tss(0, 1, 7);
                flush_draw_mesh(header, mesh);
                finish(0);
                return;
            }
            matfx_set_env_map_texture(env, 1, frame());
            if shininess != 0xFF {
                set_render_state(D3DRS_TEXTUREFACTOR, shininess.wrapping_mul(0x0101_0101));
                stage_states(1, &[(1, 0x19), (0x1A, 1), (2, 2), (3, 3)]);
            } else {
                stage_states(1, &[(1, 7), (2, 2), (3, 1)]);
                if g::<u32>(TWO_STAGE_VALIDATE).read() != 0 {
                    let mut passes = 0u32;
                    flush_cache();
                    let hr = dev_validate_device(device(), GPtr::from_raw(&mut passes));
                    g::<u32>(TWO_STAGE_VALID).write(u32::from(hr >= 0 && passes == 1));
                    g::<u32>(TWO_STAGE_VALIDATE).write(0);
                }
            }
            flush_draw_mesh(header, mesh);
            finish(1);
            return;
        }
        flush_draw_mesh(header, mesh);
    }
    blend_env_pass(header, mesh, env, frame(), shininess);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_MATFX_SET_ENV_MAP_TEXTURE, HOOK_MATFX_RENDER_ENV];
