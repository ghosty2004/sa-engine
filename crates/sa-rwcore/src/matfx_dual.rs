//! The MatFX D3D9 dual-texture render of one mesh.
//!
//! The DUAL effect slot: +0 texture, +4 source blend, +8 destination blend. With UV transforms (material
//! effects 6) slot 0 holds the base (+0) and dual (+4) matrices and the dual slot is slot 1.

use sa_core::abi::HookReg;
use sa_core::input::rw_render_state_set;
use sa_core::mem::GPtr;
use sa_core::hook;

use crate::bindings::{flush_cache, vertex_alpha_enable};
use crate::common::{g, p8, rd};
use crate::d3d9rast::raster_convert_to_non_palettized;
use crate::d3d9state::{
    device, get_render_state, rs_dest_blend, rs_is_vertex_alpha_enable, rs_src_blend, set_render_state, set_texture,
    set_texture_stage_state as tss,
};
use crate::frontier_b2::{flush_draw_mesh, matfx_d3d9_draw_mesh};
use crate::matfx::{matfx_default_render, set_uv_transform, MATFX_MATERIAL_OFFSET};
use crate::matfx_env::{dev_validate_device, rw_render_state_get};

/// The device can blend the dual texture in a second texture stage.
const DUAL_IN_STAGES: usize = 0xC9_ABAC;
/// Texture stages the device offers.
const MAX_TEXTURE_STAGES: usize = 0xC9_ABBC;
/// TEXTUREFACTOR-modulated second stages work.
const MODULATE_TFACTOR: usize = 0xC9_ABB0;
/// The SRCCOLOR / SRCCOLOR... (9, 3) stage setup works; its one-time validation is still due.
const ADDSIGNED_OK: usize = 0xC9_ABB4;
const ADDSIGNED_VALIDATE: usize = 0x8E_29C0;
const D3DTSS_TEXTURETRANSFORMFLAGS: u32 = 0x18;
const D3DTSS_TEXCOORDINDEX: u32 = 0xB;

fn stage_states(states: &[(u32, u32, u32)]) {
    for &(stage, ty, v) in states {
        tss(stage, ty, v);
    }
}

const BASE_TEXTURED: [(u32, u32, u32); 6] = [(0, 1, 4), (0, 2, 2), (0, 3, 0), (0, 4, 4), (0, 5, 2), (0, 6, 0)];
const BASE_UNTEXTURED: [(u32, u32, u32); 4] = [(0, 1, 3), (0, 3, 0), (0, 4, 3), (0, 6, 0)];
/// Stage 0 passes the base texture through when the device has three stages.
const STAGE0_TEXTURE: [(u32, u32, u32); 4] = [(0, 1, 2), (0, 2, 2), (0, 4, 2), (0, 5, 2)];
/// Stage 2 modulates with the diffuse colour.
const STAGE2_DIFFUSE: [(u32, u32, u32); 6] = [(2, 1, 4), (2, 2, 0), (2, 3, 1), (2, 4, 4), (2, 5, 0), (2, 6, 1)];

/// The dual texture in texture stages for the blend modes the device can express in one draw; `None`
/// when the mesh needs the blended second draw.
fn dual_in_stages(tex: GPtr<u8>, dual: GPtr<u8>, src: u32, dst: u32) -> Option<()> {
    let three_stages = || g::<u32>(MAX_TEXTURE_STAGES).read() == 3;
    match (src, dst) {
        (9, 1) | (1, 3) => {
            if tex.is_null() {
                set_texture(dual, 0);
                stage_states(&BASE_TEXTURED);
            } else {
                set_texture(dual, 1);
                if three_stages() {
                    stage_states(&STAGE0_TEXTURE);
                    stage_states(&[(1, 1, 4), (1, 2, 2), (1, 3, 1), (1, 4, 3), (1, 6, 1)]);
                    stage_states(&STAGE2_DIFFUSE);
                } else {
                    stage_states(&[(1, 1, 4), (1, 2, 2), (1, 3, 1), (1, 4, 3), (1, 6, 1)]);
                }
            }
        }
        (5, 6) => {
            if tex.is_null() {
                set_render_state(0x3C, u32::MAX);
                set_texture(dual, 0);
                stage_states(&[(0, 1, 0xD), (0, 2, 2), (0, 3, 3), (0, 1, 0xD), (0, 2, 2), (0, 3, 3)]);
                stage_states(&[(1, 1, 4), (1, 2, 0), (1, 3, 1), (1, 4, 4), (1, 5, 0), (1, 6, 1)]);
            } else {
                if g::<u32>(MAX_TEXTURE_STAGES).read() < 3 {
                    return None;
                }
                set_texture(dual, 1);
                stage_states(&STAGE0_TEXTURE);
                stage_states(&[(1, 1, 0xD), (1, 2, 2), (1, 3, 1), (1, 4, 0xD), (1, 5, 2), (1, 6, 1)]);
                stage_states(&STAGE2_DIFFUSE);
            }
        }
        (9, 3) => {
            if g::<u32>(ADDSIGNED_OK).read() == 0 {
                return None;
            }
            if tex.is_null() {
                set_texture(dual, 0);
                stage_states(&[(0, 1, 5), (0, 4, 3), (0, 6, 0)]);
            } else {
                set_texture(dual, 1);
                stage_states(&[(1, 1, 5), (1, 2, 2), (1, 3, 1)]);
                if three_stages() {
                    stage_states(&STAGE0_TEXTURE);
                    stage_states(&[(1, 4, 3), (1, 6, 1)]);
                    stage_states(&STAGE2_DIFFUSE);
                } else {
                    stage_states(&[(1, 4, 3), (1, 6, 1)]);
                }
            }
            if g::<u32>(ADDSIGNED_VALIDATE).read() != 0 {
                let mut passes = 0u32;
                flush_cache();
                let hr = dev_validate_device(device(), GPtr::from_raw(&mut passes));
                let valid = hr >= 0 && passes == 1;
                if !valid {
                    g::<u32>(ADDSIGNED_OK).write(0);
                    set_texture(tex, 0);
                    stage_states(&BASE_TEXTURED);
                    stage_states(&[(1, 1, 1), (1, 4, 1), (2, 1, 1), (2, 4, 1)]);
                    set_texture(GPtr::NULL, 1);
                }
                g::<u32>(ADDSIGNED_VALIDATE).write(0);
                if !valid {
                    return None;
                }
            }
        }
        (2, 2) => {
            if tex.is_null() {
                set_texture(dual, 0);
                stage_states(&[(0, 4, 3), (0, 6, 0), (1, 1, 7), (1, 2, 0), (1, 3, 1), (1, 4, 3), (1, 6, 1)]);
            } else {
                if g::<u32>(MODULATE_TFACTOR).read() == 0 {
                    return None;
                }
                set_texture(dual, 1);
                stage_states(&[(1, 1, 0x19), (1, 0x1A, 1), (1, 2, 2), (1, 3, 0), (1, 4, 3), (1, 6, 1)]);
            }
        }
        (1, 5) => {
            if tex.is_null() {
                set_texture(dual, 0);
                stage_states(&[(0, 1, 4), (0, 2, 0x22), (0, 3, 0), (0, 4, 3), (0, 6, 0)]);
            } else {
                set_texture(dual, 1);
                if three_stages() {
                    stage_states(&STAGE0_TEXTURE);
                    stage_states(&[(1, 1, 4), (1, 2, 0x22), (1, 3, 1), (1, 4, 3), (1, 6, 1)]);
                    stage_states(&STAGE2_DIFFUSE);
                } else {
                    stage_states(&[(1, 1, 4), (1, 2, 0x22), (1, 3, 1), (1, 4, 3), (1, 6, 1)]);
                }
            }
        }
        _ => return None,
    }
    Some(())
}

/// The dual texture drawn again over the mesh with the slot's blend modes (z writes off; fog towards
/// black for ONE / ONE-style blends, white for the colour-modulating ones).
fn blend_dual_pass(header: GPtr<u8>, mesh: GPtr<u8>, flags: u32, dual: GPtr<u8>, slot: GPtr<u8>, uv: GPtr<u8>) {
    set_texture(dual, 0);
    if !uv.is_null() {
        set_uv_transform(0, p8(rd::<u32>(uv, 4)));
    }
    if rs_is_vertex_alpha_enable() == 0 {
        vertex_alpha_enable(1);
    }
    let (src, dst) = (rd::<u32>(slot, 4), rd::<u32>(slot, 8));
    if matches!((src, dst), (9, 1) | (1, 3) | (9, 3)) {
        set_render_state(0xF, 0);
    }
    let (mut old_src, mut old_dst) = (0u32, 0u32);
    rw_render_state_get(10, GPtr::from_raw(&mut old_src));
    rw_render_state_get(11, GPtr::from_raw(&mut old_dst));
    rs_src_blend(rd::<u32>(slot, 4));
    rs_dest_blend(rd::<u32>(slot, 8));
    let (mut zwrite, mut fog, mut fog_colour) = (0u32, 0u32, 0u32);
    get_render_state(0xE, GPtr::from_raw(&mut zwrite));
    set_render_state(0xE, 0);
    get_render_state(0x1C, GPtr::from_raw(&mut fog));
    if fog != 0 {
        get_render_state(0x22, GPtr::from_raw(&mut fog_colour));
        let dst = rd::<u32>(slot, 8);
        if dst == 2 {
            set_render_state(0x22, 0);
        } else if rd::<u32>(slot, 4) == 9 || dst == 3 {
            set_render_state(0x22, u32::MAX);
        }
    }
    let projected = flags & 0x80 != 0;
    if projected {
        tss(0, D3DTSS_TEXCOORDINDEX, 1);
    }
    flush_draw_mesh(header, mesh);
    set_render_state(0xE, zwrite);
    if fog != 0 {
        set_render_state(0x22, fog_colour);
    }
    if projected {
        tss(0, D3DTSS_TEXCOORDINDEX, 0);
    }
    vertex_alpha_enable(0);
    rw_render_state_set(10, old_src);
    rw_render_state_set(11, old_dst);
}

/// `_rpMatFXD3D9RenderDual(header, mesh, flags, texture, dualTexture)` (0x812D40): without a dual raster
/// the default render. Otherwise the base texture (stage 0, its UV transform), then the dual texture in
/// further texture stages when the device can express the slot's blend modes in one draw, else drawn
/// again blended over the first draw.
#[hook(addr = 0x812D40, conv = cdecl, name = "FUN_00812d40", expect = "83 EC 10 55 8B 6C 24 28",
       unverified = "frontier2-c32", route = "L2", hazards = "heap-writes",
       note = "keeps the UV-transform data, the validation pass count and render states in its own argument slots (not reproduced)")]
pub fn matfx_render_dual(header: GPtr<u8>, mesh: GPtr<u8>, flags: u32, tex: GPtr<u8>, dual: GPtr<u8>) {
    if dual.is_null() || rd::<u32>(dual, 0) == 0 {
        matfx_default_render(header, mesh, flags, tex.addr() as u32);
        return;
    }
    let fx = p8(rd::<u32>(p8(rd::<u32>(mesh, 8)), g::<i32>(MATFX_MATERIAL_OFFSET).read() as isize));
    let (uv, slot) = if rd::<u32>(fx, 0x30) == 6 { (fx, fx.byte_add(0x18)) } else { (GPtr::NULL, fx) };
    if flags as u8 & 0x84 != 0 {
        set_texture(tex, 0);
        stage_states(&BASE_TEXTURED);
    } else {
        set_texture(GPtr::NULL, 0);
        stage_states(&BASE_UNTEXTURED);
    }
    if !uv.is_null() {
        set_uv_transform(0, p8(rd::<u32>(uv, 0)));
    }
    if !tex.is_null() {
        let dual_raster = p8(rd::<u32>(dual, 0));
        if rd::<u8>(p8(rd::<u32>(tex, 0)), 0x23) & 0x20 != 0 && rd::<u8>(dual_raster, 0x23) & 0x20 != 0 && tex != dual {
            raster_convert_to_non_palettized(dual_raster);
        }
    }
    let in_stages = g::<u32>(DUAL_IN_STAGES).read() != 0
        && dual_in_stages(tex, dual, rd::<u32>(slot, 4), rd::<u32>(slot, 8)).is_some();
    if in_stages {
        if flags as i8 >= 0 {
            tss(1, D3DTSS_TEXCOORDINDEX, 0);
        }
        if !uv.is_null() {
            set_uv_transform(1, p8(rd::<u32>(uv, 4)));
        }
    }
    matfx_d3d9_draw_mesh(header, mesh);
    if in_stages {
        stage_states(&[(0, 1, 4), (0, 2, 2), (0, 3, 0)]);
        set_texture(GPtr::NULL, 1);
        stage_states(&[(2, 1, 1), (2, 4, 1)]);
        if flags as i8 >= 0 {
            tss(1, D3DTSS_TEXCOORDINDEX, 1);
        }
    } else {
        blend_dual_pass(header, mesh, flags, dual, slot, uv);
    }
    if !uv.is_null() {
        tss(0, D3DTSS_TEXTURETRANSFORMFLAGS, 0);
        tss(1, D3DTSS_TEXTURETRANSFORMFLAGS, 0);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_MATFX_RENDER_DUAL];
