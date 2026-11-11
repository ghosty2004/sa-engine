//! PORT-RW-4 module `matfx`: RpMatFX and its D3D9 pipeline (see docs/arch/rw-port.md, PORT-RW-4).
//! Ported from the 1.0 US bytes; the specs under `docs/re/specs/RW/` are reading aids.
//!
//! Plugin offsets: material 0xC9AB74 (`MatFXData*`), atomic 0xC9AB84 and world sector 0xC9AB88 (`RwBool`
//! effects enabled). `MatFXData` (0x34, freelist `[0xC9AB80]`, hint 0x30120): two 0x18-byte effect slots
//! (+0, +0x18; slot type at slot+0x14) and the effect flags at +0x30. Slot payloads: BUMPMAP +0 frame, +4
//! bumped texture, +8 bump texture, +0xC -coefficient, +0x10 1/width; ENVMAP +0 frame, +4 texture, +8
//! coefficient, +0xC frame-buffer alpha; DUAL +0 texture, +4 src blend, +8 dst blend; UVTRANSFORM +0/+4
//! matrices. Pipelines: atomic 0xC9AB8C, sector 0xC9AB90; saved instance callbacks 0xC9ABC4 / 0xC9ABC8;
//! caps flags 0xC9ABAC..0xC9ABC0, pixel shaders 0xC9AB94..0xC9ABA8.
//!
//! Slot searches are inlined in most originals (slot 0 then 1 by type, NULL when absent, and the callers
//! then dereference NULL: kept). The originals keep their byte loop counter in an argument slot (the
//! material's) or a local, and `_rpMatFXD3D9AtomicRenderCallback` stores its lighting state and its
//! "no lighting" flag over its `object` / `type` argument slots; neither is reproduced (caller stack).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, p8, rd, rw_memory_alloc, wr, x};

/// Material plugin offset.
const MATFX_MATERIAL_OFFSET: usize = 0xC9AB74;
/// Atomic plugin offset.
const MATFX_ATOMIC_OFFSET: usize = 0xC9AB84;
/// World-sector plugin offset.
const MATFX_SECTOR_OFFSET: usize = 0xC9AB88;
/// MatFXData freelist.
const MATFX_FREELIST: usize = 0xC9AB80;
/// Freelist create parameters.
const FL_PER_BLOCK: usize = 0x8E29B8;
const FL_PREALLOC: usize = 0x8E29BC;
/// Pipelines.
const ATOMIC_PIPE: usize = 0xC9AB8C;
const SECTOR_PIPE: usize = 0xC9AB90;
/// Saved default instance callbacks.
const ATOMIC_INSTANCE_CB: usize = 0xC9ABC4;
const SECTOR_INSTANCE_CB: usize = 0xC9ABC8;
/// Pixel shaders usable.
const PS_AVAILABLE: usize = 0xC9ABC0;
/// The six pixel shader slots and their bytecode.
const PS_SLOTS: [(usize, u32); 6] = [
    (0xC9AB94, 0x8852F0),
    (0xC9AB98, 0x885350),
    (0xC9AB9C, 0x8853A0),
    (0xC9ABA0, 0x8853E0),
    (0xC9ABA4, 0x885468),
    (0xC9ABA8, 0x885518),
];
/// rwID_MATERIALEFFECTSPLUGIN
const MATFX_ID: u32 = 0x120;
/// 1.0f (0x858624).
const ONE: f32 = 1.0;

orig! {
    /// `RwEngineRegisterPlugin(size, id, open, close)` (0x7F2BB0)
    fn rw_engine_register_plugin(size: i32, id: u32, open: u32, close: u32) -> i32 = cdecl 0x7F2BB0 => unported;
    /// `RpMaterialRegisterPlugin(size, id, ctor, dtor, copy)` (0x74DBF0)
    fn rp_material_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74DBF0 => unported;
    /// `RpMaterialRegisterPluginStream(id, read, write, getSize)` (0x74DC20)
    fn rp_material_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x74DC20 => unported;
    /// `RpAtomicRegisterPlugin(size, id, ctor, dtor, copy)` (0x74BDA0)
    fn rp_atomic_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74BDA0 => unported;
    /// `RpAtomicRegisterPluginStream(id, read, write, getSize)` (0x74BE00)
    fn rp_atomic_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x74BE00 => unported;
    /// `RpWorldSectorRegisterPlugin(size, id, ctor, dtor, copy)` (0x761C90)
    fn rp_world_sector_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x761C90 => unported;
    /// `RpWorldSectorRegisterPluginStream(id, read, write, getSize)` (0x761CC0)
    fn rp_world_sector_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x761CC0 => unported;
    /// `RwTextureDestroy(texture)` (0x7F3820)
    fn rw_texture_destroy(t: GPtr<u8>) -> u32 = cdecl 0x7F3820 => unported;
    /// `RwTexDictionaryGetCurrent()` (0x7F3A90)
    fn rw_tex_dictionary_get_current() -> GPtr<u8> = cdecl 0x7F3A90 => unported;
    /// `RwTexDictionaryFindNamedTexture(dict, name)` (0x7F39F0)
    fn rw_tex_dictionary_find_named_texture(d: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F39F0 => unported;
    /// `RwTexDictionaryAddTexture(dict, texture)` (0x7F3980)
    fn rw_tex_dictionary_add_texture(d: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3980 => unported;
    /// Bumped-texture builder (0x816290): (baseTexture, bumpTexture)
    fn matfx_create_bumped_texture(base: GPtr<u8>, bump: GPtr<u8>) -> GPtr<u8> = cdecl 0x816290 => unported;
    /// Dual-texture render-state setup stub (0x816280): (slot, state), returns 1
    fn matfx_setup_dual_state(slot: GPtr<u8>, state: u32) -> u32 = cdecl 0x816280 => unported;
    /// `RxPipelineCreate()` (0x8057B0)
    fn rx_pipeline_create() -> GPtr<u8> = cdecl 0x8057B0 => unported;
    /// `RxPipelineLock(pipeline)` (0x806990)
    fn rx_pipeline_lock(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x806990 => unported;
    /// `RxLockedPipeAddFragment(pipeline, firstIndex, nodeDef, NULL)` (0x806BE0, varargs; as called here)
    fn rx_locked_pipe_add_fragment(p: GPtr<u8>, first: u32, node: GPtr<u8>, end: u32) -> GPtr<u8> = cdecl 0x806BE0 => unported;
    /// `RxLockedPipeUnlock(pipeline)` (0x805F40)
    fn rx_locked_pipe_unlock(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x805F40 => unported;
    /// `_rxPipelineDestroy(pipeline)` (0x805820)
    fn rx_pipeline_destroy(p: GPtr<u8>) = cdecl 0x805820 => unported;
    /// `RxPipelineFindNodeByName(pipeline, name, start, index)` (0x806B30)
    fn rx_pipeline_find_node_by_name(p: GPtr<u8>, name: u32, start: u32, index: u32) -> GPtr<u8> = cdecl 0x806B30 => unported;
    /// `RxNodeDefinitionGetD3D9AtomicAllInOne()` (0x7582E0)
    fn node_def_d3d9_atomic_all_in_one() -> GPtr<u8> = cdecl 0x7582E0 => unported;
    /// `RxNodeDefinitionGetD3D9WorldSectorAllInOne()` (0x75E9F0)
    fn node_def_d3d9_sector_all_in_one() -> GPtr<u8> = cdecl 0x75E9F0 => unported;
    /// `RxD3D9AllInOneGetInstanceCallBack(node)` (0x757390)
    fn all_in_one_get_instance_cb(n: GPtr<u8>) -> u32 = cdecl 0x757390 => unported;
    /// `RxD3D9AllInOneSetInstanceCallBack(node, cb)` (0x757380)
    fn all_in_one_set_instance_cb(n: GPtr<u8>, cb: u32) = cdecl 0x757380 => unported;
    /// `RxD3D9AllInOneSetRenderCallBack(node, cb)` (0x7573E0)
    fn all_in_one_set_render_cb(n: GPtr<u8>, cb: u32) = cdecl 0x7573E0 => unported;
    /// `RwD3D9GetCaps()` (0x7FAD20)
    fn rw_d3d9_get_caps() -> GPtr<u8> = cdecl 0x7FAD20 => unported;
    /// `RwD3D9CreatePixelShader(function, &shader)` (0x7FACC0)
    fn rw_d3d9_create_pixel_shader(code: u32, out: u32) -> i32 = cdecl 0x7FACC0 => unported;
    /// `RwD3D9DeletePixelShader(shader)` (0x7FACF0)
    fn rw_d3d9_delete_pixel_shader(s: u32) = cdecl 0x7FACF0 => unported;
    /// Atomic geometry usage flags get (0x7588D0) / set (0x7588B0)
    fn atomic_geometry_get_usage_flags(geom: GPtr<u8>) -> u32 = cdecl 0x7588D0 => unported;
    fn atomic_geometry_set_usage_flags(geom: GPtr<u8>, flags: u32) = cdecl 0x7588B0 => unported;
    /// World-sector usage flags get (0x758900) / set (0x7588E0)
    fn sector_get_usage_flags(s: GPtr<u8>) -> u32 = cdecl 0x758900 => unported;
    fn sector_set_usage_flags(s: GPtr<u8>, flags: u32) = cdecl 0x7588E0 => unported;
    /// A stored instance callback `(object, header, reinstance)`
    fn call_instance_cb(f: u32, object: GPtr<u8>, header: GPtr<u8>, reinstance: u32) -> u32 = cdecl dyn => unported;
    /// `_rwD3D9EnableClippingIfNeeded(object, type)` (0x756D90)
    fn d3d9_enable_clipping_if_needed(object: GPtr<u8>, ty: u32) = cdecl 0x756D90 => unported;
    /// `RwD3D9GetRenderState(state, &value)` (0x7FC320)
    fn rw_d3d9_get_render_state(state: u32, out: GPtr<u32>) = cdecl 0x7FC320 => unported;
    /// `RwD3D9SetRenderState(state, value)` (0x7FC2D0)
    fn rw_d3d9_set_render_state(state: u32, value: u32) -> i32 = cdecl 0x7FC2D0 => unported;
    /// `RwD3D9SetTextureStageState(stage, type, value)` (0x7FC340)
    fn rw_d3d9_set_texture_stage_state(stage: u32, ty: u32, value: u32) -> i32 = cdecl 0x7FC340 => unported;
    /// `RwD3D9SetTexture(texture, stage)` (0x7FDE70)
    fn rw_d3d9_set_texture(t: u32, stage: u32) -> i32 = cdecl 0x7FDE70 => unported;
    /// `IDirect3DDevice9::SetIndices(ib)` (vtable +0x1A0)
    fn device_set_indices(dev: GPtr<u8>, ib: u32) -> i32 = stdcall vslot 0x1A0 => unported;
    /// `IDirect3DDevice9::SetVertexDeclaration(decl)` (vtable +0x15C)
    fn device_set_vertex_declaration(dev: GPtr<u8>, decl: u32) -> i32 = stdcall vslot 0x15C => unported;
    /// `_rwD3D9SetStreams(streams, useOffsets)` (0x7FA090)
    fn d3d9_set_streams(streams: GPtr<u8>, offsets: u32) = cdecl 0x7FA090 => unported;
    /// `_rwD3D9RenderStateVertexAlphaEnable(enable)` (0x7FE0A0)
    fn d3d9_vertex_alpha_enable(enable: u32) = cdecl 0x7FE0A0 => unported;
    /// `RwD3D9SetSurfaceProperties(surfaceProps, color, flags)` (0x7FC4D0)
    fn rw_d3d9_set_surface_properties(sp: GPtr<u8>, color: GPtr<u8>, flags: u32) -> i32 = cdecl 0x7FC4D0 => unported;
    /// MatFX D3D9 effect renderers: no-lighting (0x812C90), default (0x812AC0), dual (0x812D40), env
    /// (0x813A10), bump / bump-env (0x814460)
    fn matfx_render_black(header: GPtr<u8>, inst: GPtr<u8>) = cdecl 0x812C90 => unported;
    fn matfx_render_default(header: GPtr<u8>, inst: GPtr<u8>, flags: u32, tex: u32) = cdecl 0x812AC0 => unported;
    fn matfx_render_dual(header: GPtr<u8>, inst: GPtr<u8>, flags: u32, tex: u32, dual: u32) = cdecl 0x812D40 => unported;
    fn matfx_render_env(header: GPtr<u8>, inst: GPtr<u8>, flags: u32, bump: u32, tex: u32, env: u32) = cdecl 0x813A10 => unported;
    fn matfx_render_bump(header: GPtr<u8>, inst: GPtr<u8>, flags: u32, tex: u32, bumped: u32, env: u32) = cdecl 0x814460 => unported;

    /// `_rpMatFXMaterialDataDestroyEffects(data)` (0x810CA0)
    fn destroy_effects_call(d: GPtr<u8>) -> GPtr<u8> = cdecl 0x810CA0 => rp_matfx_material_data_destroy_effects;
    /// `_rpMatFXAtomicSetPipeline(atomic)` (0x816260)
    fn atomic_set_pipeline_call(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x816260 => rp_matfx_atomic_set_pipeline;
    /// `_rpMatFXWorldSectorSetPipeline(sector)` (0x816270)
    fn sector_set_pipeline_call(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x816270 => rp_matfx_world_sector_set_pipeline;
    /// `RpMatFXMaterialSetBumpMapTexture(material, texture)` (0x812160)
    fn set_bump_map_texture_call(m: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> = cdecl 0x812160 => rp_matfx_material_set_bump_map_texture;
}

fn ext_of(m: GPtr<u8>) -> GPtr<u8> {
    p8(rd::<u32>(m, g::<i32>(MATFX_MATERIAL_OFFSET).read() as isize))
}

/// The inlined effect search: slot 0, then slot 1, whose type (+0x14) is `ty`; NULL when absent.
fn find(m: GPtr<u8>, ty: u32) -> GPtr<u8> {
    let d = ext_of(m);
    for k in 0..2isize {
        if rd::<u32>(d, k * 0x18 + 0x14) == ty {
            return d.byte_add(k * 0x18);
        }
    }
    GPtr::NULL
}

/// `refCount++` of a texture (+0x54).
fn add_ref(t: GPtr<u8>) {
    wr::<u32>(t, 0x54, rd::<u32>(t, 0x54).wrapping_add(1));
}

/// `RpMatFXMaterialDataSetFreeListCreateParams(blockSize, numBlocksToPrealloc)` (0x810700). EAX =
/// blockSize.
#[hook(addr = 0x810700, conv = cdecl, name = "RpMatFXMaterialDataSetFreeListCreateParams", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn rp_matfx_material_data_set_free_list_create_params(block: u32, prealloc: u32) -> u32 {
    g::<u32>(FL_PER_BLOCK).write(block);
    g::<u32>(FL_PREALLOC).write(prealloc);
    block
}

/// `RpMatFXPluginAttach()` (0x810AA0): engine, material (+stream), atomic (+stream) and world-sector
/// (+stream) plugins; any negative result returns 0; the last one's `>= 0` is the result.
#[hook(addr = 0x810AA0, conv = cdecl, name = "RpMatFXPluginAttach", expect = "68 B0 0B 81 00")]
pub fn rp_matfx_plugin_attach() -> u32 {
    if rw_engine_register_plugin(0, MATFX_ID, 0x810BF0, 0x810BB0) < 0 {
        return 0;
    }
    let m = rp_material_register_plugin(4, MATFX_ID, 0x810C40, 0x810C60, 0x810D20);
    g::<i32>(MATFX_MATERIAL_OFFSET).write(m);
    if m < 0 {
        return 0;
    }
    if rp_material_register_plugin_stream(MATFX_ID, 0x811410, 0x8111F0, 0x811970) < 0 {
        return 0;
    }
    let a = rp_atomic_register_plugin(4, MATFX_ID, 0x811A00, 0x811A20, 0x811A40);
    g::<i32>(MATFX_ATOMIC_OFFSET).write(a);
    if a < 0 {
        return 0;
    }
    if rp_atomic_register_plugin_stream(MATFX_ID, 0x811A90, 0x811A60, 0x811AE0) < 0 {
        return 0;
    }
    let s = rp_world_sector_register_plugin(4, MATFX_ID, 0x811B00, 0x811B20, 0x811B40);
    g::<i32>(MATFX_SECTOR_OFFSET).write(s);
    if s < 0 {
        return 0;
    }
    (rp_world_sector_register_plugin_stream(MATFX_ID, 0x811B90, 0x811B60, 0x811BE0) >= 0) as u32
}

/// `_rpMatFXMaterialDataDestroyEffects(data)` (0x810CA0): per slot by type: BUMPMAP destroys the bumped
/// (+4) and bump (+8) textures, ENVMAP its texture (+4), DUAL its texture (+0), UVTRANSFORM clears +0/+4;
/// then the 0x34 bytes are zeroed. Returns data.
#[hook(addr = 0x810CA0, conv = cdecl, name = "_rpMatFXMaterialDataDestroyEffects", expect = "53 55 8B 6C 24 0C")]
pub fn rp_matfx_material_data_destroy_effects(d: GPtr<u8>) -> GPtr<u8> {
    for k in 0..2isize {
        let s = d.byte_add(k * 0x18);
        let destroy = |o: isize| {
            let t = rd::<u32>(s, o);
            if t != 0 {
                rw_texture_destroy(p8(t));
            }
        };
        match rd::<u32>(s, 0x14) {
            1 => {
                destroy(4);
                destroy(8);
            }
            2 => destroy(4),
            4 => destroy(0),
            5 => {
                wr::<u32>(s, 0, 0);
                wr::<u32>(s, 4, 0);
            }
            _ => {}
        }
    }
    for k in 0..13 {
        wr::<u32>(d, k * 4, 0);
    }
    d
}

/// `_rpMatFXMaterialFindEffect(material, type)` (0x811150): the slot of that type, or NULL.
#[hook(addr = 0x811150, conv = cdecl, name = "_rpMatFXMaterialFindEffect", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_find_effect(m: GPtr<u8>, ty: u32) -> GPtr<u8> {
    find(m, ty)
}

/// `RpMatFXAtomicEnableEffects(atomic)` (0x811C00): sets the MatFX pipeline once (failure -> NULL) and
/// the enabled flag. Returns atomic.
#[hook(addr = 0x811C00, conv = cdecl, name = "RpMatFXAtomicEnableEffects", expect = "A1 84 AB C9 00")]
pub fn rp_matfx_atomic_enable_effects(a: GPtr<u8>) -> GPtr<u8> {
    let f = a.byte_add(g::<i32>(MATFX_ATOMIC_OFFSET).read() as isize);
    if rd::<u32>(f, 0) == 0 {
        if atomic_set_pipeline_call(a).is_null() {
            return GPtr::NULL;
        }
        wr::<u32>(f, 0, 1);
    }
    a
}

/// `RpMatFXAtomicQueryEffects(atomic)` (0x811C30).
#[hook(addr = 0x811C30, conv = cdecl, name = "RpMatFXAtomicQueryEffects", expect = "8B 44 24 04 8B 0D 84 AB C9 00")]
pub fn rp_matfx_atomic_query_effects(a: GPtr<u8>) -> u32 {
    rd::<u32>(a, g::<i32>(MATFX_ATOMIC_OFFSET).read() as isize)
}

/// `RpMatFXWorldSectorEnableEffects(sector)` (0x811C40): as the atomic variant. Returns sector.
#[hook(addr = 0x811C40, conv = cdecl, name = "RpMatFXWorldSectorEnableEffects", expect = "A1 88 AB C9 00")]
pub fn rp_matfx_world_sector_enable_effects(s: GPtr<u8>) -> GPtr<u8> {
    let f = s.byte_add(g::<i32>(MATFX_SECTOR_OFFSET).read() as isize);
    if rd::<u32>(f, 0) == 0 {
        if sector_set_pipeline_call(s).is_null() {
            return GPtr::NULL;
        }
        wr::<u32>(f, 0, 1);
    }
    s
}

/// `RpMatFXWorldSectorQueryEffects(sector)` (0x811C70).
#[hook(addr = 0x811C70, conv = cdecl, name = "RpMatFXWorldSectorQueryEffects", expect = "8B 44 24 04 8B 0D 88 AB C9 00")]
pub fn rp_matfx_world_sector_query_effects(s: GPtr<u8>) -> u32 {
    rd::<u32>(s, g::<i32>(MATFX_SECTOR_OFFSET).read() as isize)
}

/// The DUAL slot defaults of `RpMatFXMaterialSetEffects`: dst blend 6, src blend 5, then the dual
/// state setup for render states 0xA and 0xB.
fn dual_defaults(m: GPtr<u8>) {
    let s = find(m, 4);
    wr::<u32>(s, 8, 6);
    wr::<u32>(s, 4, 5);
    matfx_setup_dual_state(s, 0xA);
    matfx_setup_dual_state(s, 0xB);
}

/// `RpMatFXMaterialSetEffects(material, flags)` (0x811C80): the data is allocated (zeroed) when missing
/// (failure -> NULL); flags 0, or different non-zero old flags, destroy the effects; flags stored; slot
/// types per flags (1: bump; 2: env; 3: bump + env; 4: dual + defaults; 5: uv; 6: uv + dual + defaults).
/// Returns material.
#[hook(addr = 0x811C80, conv = cdecl, name = "RpMatFXMaterialSetEffects", expect = "A1 74 AB C9 00")]
pub fn rp_matfx_material_set_effects(m: GPtr<u8>, flags: u32) -> GPtr<u8> {
    let off = g::<i32>(MATFX_MATERIAL_OFFSET).read() as isize;
    let mut d = p8(rd::<u32>(m, off));
    if d.is_null() {
        d = rw_memory_alloc(p8(g::<u32>(MATFX_FREELIST).read()), 0x30120);
        if d.is_null() {
            return GPtr::NULL;
        }
        for k in 0..13 {
            wr::<u32>(d, k * 4, 0);
        }
        wr::<u32>(m, g::<i32>(MATFX_MATERIAL_OFFSET).read() as isize, d.addr() as u32);
    }
    if flags == 0 {
        destroy_effects_call(d);
    } else {
        let old = rd::<u32>(d, 0x30);
        if old != 0 && old != flags {
            destroy_effects_call(d);
        }
    }
    wr::<u32>(d, 0x30, flags);
    match flags {
        1 => wr::<u32>(d, 0x14, 1),
        2 => wr::<u32>(d, 0x14, 2),
        3 => {
            wr::<u32>(d, 0x14, 1);
            wr::<u32>(d, 0x2C, 2);
        }
        4 => {
            wr::<u32>(d, 0x14, 4);
            dual_defaults(m);
        }
        5 => wr::<u32>(d, 0x14, 5),
        6 => {
            wr::<u32>(d, 0x14, 5);
            wr::<u32>(d, 0x2C, 4);
            dual_defaults(m);
        }
        _ => {}
    }
    m
}

/// `RpMatFXMaterialSetupBumpMap(material, texture, frame, coefficient)` (0x811E00): the bump texture,
/// then frame and -coefficient (x87 `fchs`) on the BUMPMAP slot. NULL material -> 0 (after the frame
/// store through it). Returns material.
#[hook(addr = 0x811E00, conv = cdecl, name = "RpMatFXMaterialSetupBumpMap", expect = "8B 44 24 08 53")]
pub fn rp_matfx_material_setup_bump_map(m: GPtr<u8>, t: GPtr<u8>, frame: u32, coef: f32) -> GPtr<u8> {
    if set_bump_map_texture_call(m, t).is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(find(m, 1), 0, frame);
    if m.is_null() {
        return GPtr::NULL;
    }
    wr::<f32>(find(m, 1), 0xC, f32_of(x(coef).fchs()));
    m
}

/// `RpMatFXMaterialSetupEnvMap(material, texture, frame, useFrameBufferAlpha, coefficient)` (0x811ED0):
/// texture (ref added, old destroyed), then (NULL material -> 0) frame, alpha, coefficient. Returns
/// material.
#[hook(addr = 0x811ED0, conv = cdecl, name = "RpMatFXMaterialSetupEnvMap", expect = "51 A1 74 AB C9 00")]
pub fn rp_matfx_material_setup_env_map(m: GPtr<u8>, t: GPtr<u8>, frame: u32, fb_alpha: u32, coef: u32) -> GPtr<u8> {
    let s = find(m, 2);
    add_ref(t);
    let old = rd::<u32>(s, 4);
    if old != 0 {
        rw_texture_destroy(p8(old));
        wr::<u32>(s, 4, 0);
    }
    wr::<u32>(s, 4, t.addr() as u32);
    if m.is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(find(m, 2), 0, frame);
    wr::<u32>(find(m, 2), 0xC, fb_alpha);
    wr::<u32>(find(m, 2), 8, coef);
    m
}

/// `RpMatFXMaterialSetupDualTexture(material, texture, srcBlend, dstBlend)` (0x812040): texture (ref
/// added, old destroyed, states 2 and 1), then (NULL material -> 0) blend modes and states 0xA / 0xB.
#[hook(addr = 0x812040, conv = cdecl, name = "RpMatFXMaterialSetupDualTexture", expect = "51 A1 74 AB C9 00")]
pub fn rp_matfx_material_setup_dual_texture(m: GPtr<u8>, t: GPtr<u8>, src: u32, dst: u32) -> GPtr<u8> {
    let s = find(m, 4);
    add_ref(t);
    let old = rd::<u32>(s, 0);
    if old != 0 {
        rw_texture_destroy(p8(old));
        wr::<u32>(s, 0, 0);
    }
    wr::<u32>(s, 0, t.addr() as u32);
    matfx_setup_dual_state(s, 2);
    matfx_setup_dual_state(s, 1);
    if m.is_null() {
        return GPtr::NULL;
    }
    let s = find(m, 4);
    wr::<u32>(s, 4, src);
    wr::<u32>(s, 8, dst);
    matfx_setup_dual_state(s, 0xA);
    matfx_setup_dual_state(s, 0xB);
    m
}

/// `RpMatFXMaterialGetEffects(material)` (0x812140): the flags, or 0 without data.
#[hook(addr = 0x812140, conv = cdecl, name = "RpMatFXMaterialGetEffects", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_get_effects(m: GPtr<u8>) -> u32 {
    let d = ext_of(m);
    if d.is_null() {
        0
    } else {
        rd::<u32>(d, 0x30)
    }
}

/// `RpMatFXMaterialSetBumpMapTexture(material, texture)` (0x812160): old bump (+8) and bumped (+4,
/// then 1/width 0) textures destroyed. With a texture: stored (ref added); if its raster or the
/// material texture's raster has width 0 -> return material; the bumped name interleaves the
/// characters of the base (or the bump texture's own) name and the bump name (up to 30, 32-byte
/// buffer); found in the current dictionary (ref added) or built by 0x816290 (NULL -> 0) and added to
/// the dictionary. Without a texture the material texture is used. +0x10 = 1 / (float)raster width.
#[hook(addr = 0x812160, conv = cdecl, name = "RpMatFXMaterialSetBumpMapTexture", expect = "83 EC 28 A1 74 AB C9 00")]
pub fn rp_matfx_material_set_bump_map_texture(m: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> {
    let s = find(m, 1);
    let old = rd::<u32>(s, 8);
    if old != 0 {
        rw_texture_destroy(p8(old));
        wr::<u32>(s, 8, 0);
    }
    let old = rd::<u32>(s, 4);
    if old != 0 {
        rw_texture_destroy(p8(old));
        wr::<u32>(s, 4, 0);
        wr::<u32>(s, 0x10, 0);
    }
    let tex = if t.is_null() {
        p8(rd::<u32>(m, 0))
    } else {
        wr::<u32>(s, 8, t.addr() as u32);
        add_ref(t);
        let tw0 = rd::<u32>(p8(rd::<u32>(t, 0)), 0xC) == 0;
        let base = p8(rd::<u32>(m, 0));
        if tw0 {
            return m;
        }
        if !base.is_null() && rd::<u32>(p8(rd::<u32>(base, 0)), 0xC) == 0 {
            return m;
        }
        let mut name = [0u8; 32];
        let np: GPtr<u8> = GPtr::from_raw(name.as_mut_ptr());
        let mut ptrs = [if base.is_null() { t.byte_add(0x10) } else { base.byte_add(0x10) }, t.byte_add(0x10)];
        let mut n = 0i32;
        let mut out = 0isize;
        loop {
            if rd::<u8>(ptrs[0], 0) == 0 && rd::<u8>(ptrs[1], 0) == 0 {
                break;
            }
            for p in ptrs.iter_mut() {
                let c = rd::<u8>(*p, 0);
                if c != 0 {
                    wr::<u8>(np, out, c);
                    out += 1;
                    *p = p.byte_add(1);
                    n += 1;
                }
            }
            if n >= 0x1E {
                break;
            }
        }
        wr::<u8>(np, out, 0);
        let dict = rw_tex_dictionary_get_current();
        wr::<u32>(s, 4, 0);
        if !dict.is_null() {
            let f = rw_tex_dictionary_find_named_texture(dict, np);
            wr::<u32>(s, 4, f.addr() as u32);
        }
        let found = rd::<u32>(s, 4);
        if found != 0 {
            add_ref(p8(found));
        } else {
            let b = matfx_create_bumped_texture(base, t);
            wr::<u32>(s, 4, b.addr() as u32);
            if b.is_null() {
                return GPtr::NULL;
            }
            if !dict.is_null() {
                rw_tex_dictionary_add_texture(dict, b);
            }
        }
        p8(rd::<u32>(s, 4))
    };
    let w = rd::<i32>(p8(rd::<u32>(tex, 0)), 0xC);
    wr::<f32>(s, 0x10, f32_of(X87::from_i32(w).fdivr_m(ONE)));
    m
}

/// `RpMatFXMaterialSetBumpMapFrame(material, frame)` (0x812320). EAX = material.
#[hook(addr = 0x812320, conv = cdecl, name = "RpMatFXMaterialSetBumpMapFrame", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_set_bump_map_frame(m: GPtr<u8>, frame: u32) -> GPtr<u8> {
    wr::<u32>(find(m, 1), 0, frame);
    m
}

/// `RpMatFXMaterialSetBumpMapCoefficient(material, coef)` (0x812380): stores -coef (x87). EAX = material.
#[hook(addr = 0x812380, conv = cdecl, name = "RpMatFXMaterialSetBumpMapCoefficient", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_set_bump_map_coefficient(m: GPtr<u8>, coef: f32) -> GPtr<u8> {
    wr::<f32>(find(m, 1), 0xC, f32_of(x(coef).fchs()));
    m
}

/// `RpMatFXMaterialGetBumpMapBumpedTexture(material)` (0x8123E0): slot +4.
#[hook(addr = 0x8123E0, conv = cdecl, name = "RpMatFXMaterialGetBumpMapBumpedTexture", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_get_bump_map_bumped_texture(m: GPtr<u8>) -> u32 {
    rd::<u32>(find(m, 1), 4)
}

/// `RpMatFXMaterialGetBumpMapTexture(material)` (0x812430): slot +8.
#[hook(addr = 0x812430, conv = cdecl, name = "RpMatFXMaterialGetBumpMapTexture", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_get_bump_map_texture(m: GPtr<u8>) -> u32 {
    rd::<u32>(find(m, 1), 8)
}

/// `RpMatFXMaterialGetBumpMapFrame(material)` (0x812480): slot +0.
#[hook(addr = 0x812480, conv = cdecl, name = "RpMatFXMaterialGetBumpMapFrame", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_get_bump_map_frame(m: GPtr<u8>) -> u32 {
    rd::<u32>(find(m, 1), 0)
}

/// `RpMatFXMaterialGetBumpMapCoefficient(material)` (0x8124D0): -slot+0xC in ST0.
#[hook(addr = 0x8124D0, conv = cdecl, name = "RpMatFXMaterialGetBumpMapCoefficient", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_get_bump_map_coefficient(m: GPtr<u8>) -> X87 {
    x(rd::<f32>(find(m, 1), 0xC)).fchs()
}

/// `RpMatFXMaterialSetEnvMapTexture(material, texture)` (0x812530): ref added, old destroyed, stored.
#[hook(addr = 0x812530, conv = cdecl, name = "RpMatFXMaterialSetEnvMapTexture", expect = "A1 74 AB C9 00")]
pub fn rp_matfx_material_set_env_map_texture(m: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> {
    let s = find(m, 2);
    add_ref(t);
    let old = rd::<u32>(s, 4);
    if old != 0 {
        rw_texture_destroy(p8(old));
        wr::<u32>(s, 4, 0);
    }
    wr::<u32>(s, 4, t.addr() as u32);
    m
}

/// `RpMatFXMaterialSetEnvMapFrame(material, frame)` (0x8125B0).
#[hook(addr = 0x8125B0, conv = cdecl, name = "RpMatFXMaterialSetEnvMapFrame", expect = "A1 74 AB C9 00")]
pub fn rp_matfx_material_set_env_map_frame(m: GPtr<u8>, frame: u32) -> GPtr<u8> {
    wr::<u32>(find(m, 2), 0, frame);
    m
}

/// `RpMatFXMaterialSetEnvMapFrameBufferAlpha(material, useFBAlpha)` (0x812610).
#[hook(addr = 0x812610, conv = cdecl, name = "RpMatFXMaterialSetEnvMapFrameBufferAlpha", expect = "A1 74 AB C9 00")]
pub fn rp_matfx_material_set_env_map_frame_buffer_alpha(m: GPtr<u8>, a: u32) -> GPtr<u8> {
    wr::<u32>(find(m, 2), 0xC, a);
    m
}

/// `RpMatFXMaterialSetEnvMapCoefficient(material, coef)` (0x812680): raw dword store.
#[hook(addr = 0x812680, conv = cdecl, name = "RpMatFXMaterialSetEnvMapCoefficient", expect = "A1 74 AB C9 00")]
pub fn rp_matfx_material_set_env_map_coefficient(m: GPtr<u8>, coef: u32) -> GPtr<u8> {
    wr::<u32>(find(m, 2), 8, coef);
    m
}

/// `RpMatFXMaterialGetEnvMapTexture(material)` (0x8126F0): slot +4.
#[hook(addr = 0x8126F0, conv = cdecl, name = "RpMatFXMaterialGetEnvMapTexture", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_get_env_map_texture(m: GPtr<u8>) -> u32 {
    rd::<u32>(find(m, 2), 4)
}

/// `RpMatFXMaterialGetEnvMapFrame(material)` (0x812740): slot +0.
#[hook(addr = 0x812740, conv = cdecl, name = "RpMatFXMaterialGetEnvMapFrame", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_get_env_map_frame(m: GPtr<u8>) -> u32 {
    rd::<u32>(find(m, 2), 0)
}

/// `RpMatFXMaterialGetEnvMapFrameBufferAlpha(material)` (0x812790): slot +0xC.
#[hook(addr = 0x812790, conv = cdecl, name = "RpMatFXMaterialGetEnvMapFrameBufferAlpha", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_get_env_map_frame_buffer_alpha(m: GPtr<u8>) -> u32 {
    rd::<u32>(find(m, 2), 0xC)
}

/// `RpMatFXMaterialGetEnvMapCoefficient(material)` (0x8127E0): slot +8 in ST0.
#[hook(addr = 0x8127E0, conv = cdecl, name = "RpMatFXMaterialGetEnvMapCoefficient", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_get_env_map_coefficient(m: GPtr<u8>) -> X87 {
    x(rd::<f32>(find(m, 2), 8))
}

/// `RpMatFXMaterialSetDualTexture(material, texture)` (0x812830): ref added, old destroyed, stored,
/// states 2 and 1.
#[hook(addr = 0x812830, conv = cdecl, name = "RpMatFXMaterialSetDualTexture", expect = "A1 74 AB C9 00")]
pub fn rp_matfx_material_set_dual_texture(m: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> {
    let s = find(m, 4);
    add_ref(t);
    let old = rd::<u32>(s, 0);
    if old != 0 {
        rw_texture_destroy(p8(old));
        wr::<u32>(s, 0, 0);
    }
    wr::<u32>(s, 0, t.addr() as u32);
    matfx_setup_dual_state(s, 2);
    matfx_setup_dual_state(s, 1);
    m
}

/// `RpMatFXMaterialSetDualBlendModes(material, src, dst)` (0x8128C0): stored, states 0xA / 0xB.
#[hook(addr = 0x8128C0, conv = cdecl, name = "RpMatFXMaterialSetDualBlendModes", expect = "A1 74 AB C9 00")]
pub fn rp_matfx_material_set_dual_blend_modes(m: GPtr<u8>, src: u32, dst: u32) -> GPtr<u8> {
    let s = find(m, 4);
    wr::<u32>(s, 4, src);
    wr::<u32>(s, 8, dst);
    matfx_setup_dual_state(s, 0xA);
    matfx_setup_dual_state(s, 0xB);
    m
}

/// `RpMatFXMaterialGetDualTexture(material)` (0x812930): slot +0.
#[hook(addr = 0x812930, conv = cdecl, name = "RpMatFXMaterialGetDualTexture", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_get_dual_texture(m: GPtr<u8>) -> u32 {
    rd::<u32>(find(m, 4), 0)
}

/// `RpMatFXMaterialGetDualBlendModes(material, &src, &dst)` (0x812980). EAX = material.
#[hook(addr = 0x812980, conv = cdecl, name = "RpMatFXMaterialGetDualBlendModes", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_get_dual_blend_modes(m: GPtr<u8>, src: GPtr<u32>, dst: GPtr<u32>) -> GPtr<u8> {
    let s = find(m, 4);
    src.write(rd::<u32>(s, 4));
    dst.write(rd::<u32>(s, 8));
    m
}

/// `RpMatFXMaterialSetUVTransformMatrices(material, base, dual)` (0x8129E0). EAX = material.
#[hook(addr = 0x8129E0, conv = cdecl, name = "RpMatFXMaterialSetUVTransformMatrices", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_set_uv_transform_matrices(m: GPtr<u8>, base: u32, dual: u32) -> GPtr<u8> {
    let s = find(m, 5);
    wr::<u32>(s, 0, base);
    wr::<u32>(s, 4, dual);
    m
}

/// `RpMatFXMaterialGetUVTransformMatrices(material, &base, &dual)` (0x812A50): each output written when
/// non-NULL. EAX = material.
#[hook(addr = 0x812A50, conv = cdecl, name = "RpMatFXMaterialGetUVTransformMatrices", expect = "8B 44 24 04 8B 0D 74 AB C9 00")]
pub fn rp_matfx_material_get_uv_transform_matrices(m: GPtr<u8>, base: GPtr<u32>, dual: GPtr<u32>) -> GPtr<u8> {
    let s = find(m, 5);
    if !base.is_null() {
        base.write(rd::<u32>(s, 0));
    }
    if !dual.is_null() {
        dual.write(rd::<u32>(s, 4));
    }
    m
}

/// `_rpMatFXD3D9AtomicRenderCallback(header, object, type, flags)` (0x815C20): clipping, lighting state;
/// unlit without MODULATEMATERIALCOLOR renders black (texture 0, TEXTUREFACTOR 0xFF000000, stage 0
/// SELECTARG2 of TFACTOR for colour and alpha); index buffer / streams / vertex declaration through the
/// driver caches; per mesh: vertex alpha (instance alpha or material alpha != 0xFF), then the black
/// renderer, or (lit: surface properties) the effect renderer chosen by the material's flags.
#[hook(addr = 0x815C20, conv = cdecl, name = "_rpMatFXD3D9AtomicRenderCallback", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn rp_matfx_d3d9_atomic_render_callback(header: GPtr<u8>, object: GPtr<u8>, ty: u32, flags: u32) {
    d3d9_enable_clipping_if_needed(object, ty & 0xFF);
    let mut lighting = 0u32;
    rw_d3d9_get_render_state(0x89, GPtr::from_raw(&mut lighting));
    let black = lighting == 0 && flags & 8 == 0;
    if black {
        rw_d3d9_set_texture(0, 0);
        rw_d3d9_set_render_state(0x3C, 0xFF00_0000);
        rw_d3d9_set_texture_stage_state(0, 1, 3);
        rw_d3d9_set_texture_stage_state(0, 3, 3);
        rw_d3d9_set_texture_stage_state(0, 4, 3);
        rw_d3d9_set_texture_stage_state(0, 6, 3);
    }
    let h = header.byte_add(0x18);
    let ib = rd::<u32>(h, 8);
    if ib != 0 && g::<u32>(0x8E2450).read() != ib {
        g::<u32>(0x8E2450).write(ib);
        device_set_indices(p8(g::<u32>(0xC97C28).read()), rd::<u32>(h, 8));
    }
    d3d9_set_streams(h.byte_add(0x10), rd::<u32>(h, 0x30));
    let decl = rd::<u32>(h, 0x34);
    if g::<u32>(0x8E2444).read() != decl {
        g::<u32>(0x8E2444).write(decl);
        g::<u32>(0x8E2440).write(0xFFFF_FFFF);
        device_set_vertex_declaration(p8(g::<u32>(0xC97C28).read()), rd::<u32>(h, 0x34));
    }
    let mut n = rd::<u32>(h, 4);
    if n == 0 {
        return;
    }
    let mut inst = h.byte_add(0x40);
    loop {
        let va = rd::<u32>(inst, 0xC) != 0 || rd::<u8>(p8(rd::<u32>(inst, 8)), 7) != 0xFF;
        d3d9_vertex_alpha_enable(va as u32);
        if black {
            matfx_render_black(h, inst);
        } else {
            let mat = p8(rd::<u32>(inst, 8));
            if lighting != 0 {
                rw_d3d9_set_surface_properties(mat.byte_add(0xC), mat.byte_add(4), flags);
            }
            let mat = p8(rd::<u32>(inst, 8));
            let d = ext_of(mat);
            let fx = if d.is_null() { 0 } else { rd::<u32>(d, 0x30) };
            let tex = rd::<u32>(mat, 0);
            match fx {
                1 => matfx_render_bump(h, inst, flags, tex, rd::<u32>(d, 4), 0),
                3 => matfx_render_bump(h, inst, flags, tex, rd::<u32>(d, 4), rd::<u32>(d, 0x1C)),
                2 => matfx_render_env(h, inst, flags, 0, tex, rd::<u32>(d, 4)),
                4 => matfx_render_dual(h, inst, flags, tex, rd::<u32>(d, 0)),
                6 => matfx_render_dual(h, inst, flags, tex, rd::<u32>(d, 0x18)),
                _ => matfx_render_default(h, inst, flags, tex),
            }
        }
        inst = inst.byte_add(0x24);
        n = n.wrapping_sub(1);
        if n == 0 {
            break;
        }
    }
}

/// Creates one MatFX all-in-one pipeline (pluginId 0x120) around `node_def`; NULL on failure (a lock
/// failure destroys it).
fn create_pipe(node_def: fn() -> GPtr<u8>) -> GPtr<u8> {
    let p = rx_pipeline_create();
    if p.is_null() {
        return GPtr::NULL;
    }
    wr::<u32>(p, 0x2C, MATFX_ID);
    let l = rx_pipeline_lock(p);
    if l.is_null() {
        rx_pipeline_destroy(p);
        return GPtr::NULL;
    }
    let nd = node_def();
    let f = rx_locked_pipe_add_fragment(l, 0, nd, 0);
    rx_locked_pipe_unlock(f);
    p
}

/// `_rpMatFXPipelinesCreate()` (0x815E60): atomic and sector pipelines (stored even when NULL), their
/// all-in-one nodes found by name get the MatFX instance callbacks (the defaults saved) and the MatFX
/// render callback; caps flags; with pixel shader 1.1 the six pixel shaders. Returns 1.
#[hook(addr = 0x815E60, conv = cdecl, name = "_rpMatFXPipelinesCreate", expect = "53 56 57 E8 48 F9 FE FF")]
pub fn rp_matfx_pipelines_create() -> u32 {
    let p = create_pipe(node_def_d3d9_atomic_all_in_one);
    g::<u32>(ATOMIC_PIPE).write(p.addr() as u32);
    let name = rd::<u32>(node_def_d3d9_atomic_all_in_one(), 0);
    let node = rx_pipeline_find_node_by_name(p8(g::<u32>(ATOMIC_PIPE).read()), name, 0, 0);
    g::<u32>(ATOMIC_INSTANCE_CB).write(all_in_one_get_instance_cb(node));
    all_in_one_set_instance_cb(node, 0x8160A0);
    all_in_one_set_render_cb(node, 0x815C20);

    let p = create_pipe(node_def_d3d9_sector_all_in_one);
    g::<u32>(SECTOR_PIPE).write(p.addr() as u32);
    let name = rd::<u32>(node_def_d3d9_sector_all_in_one(), 0);
    let node = rx_pipeline_find_node_by_name(p8(g::<u32>(SECTOR_PIPE).read()), name, 0, 0);
    g::<u32>(SECTOR_INSTANCE_CB).write(all_in_one_get_instance_cb(node));
    all_in_one_set_instance_cb(node, 0x816120);
    all_in_one_set_render_cb(node, 0x815C20);

    let caps = rw_d3d9_get_caps();
    let stages = rd::<u32>(caps, 0x94);
    g::<u32>(0xC9ABBC).write(stages);
    if stages >= 2 {
        g::<u32>(0xC9ABAC).write((rd::<u32>(caps, 0x98) >= 2) as u32);
        g::<u32>(0xC9ABB0).write(rd::<u32>(caps, 0x90) & 0x0100_0000);
        g::<u32>(0xC9ABB4).write(rd::<u32>(caps, 0x90) & 0x10);
    } else {
        for a in [0xC9ABACusize, 0xC9ABB0, 0xC9ABB4, 0xC9ABB8, 0x8E29C0, 0x8E29C4] {
            g::<u32>(a).write(0);
        }
    }
    if rd::<u32>(caps, 0xCC) & 0xFFFF >= 0x101 {
        g::<u32>(PS_AVAILABLE).write(1);
        for (slot, code) in PS_SLOTS {
            rw_d3d9_create_pixel_shader(code, slot as u32);
        }
    } else {
        g::<u32>(PS_AVAILABLE).write(0);
        for (slot, _) in PS_SLOTS {
            g::<u32>(slot).write(0);
        }
    }
    1
}

/// `_rpMatFXAtomicInstanceCallback(atomic, header, reinstance)` (0x8160A0): on a first instance, every
/// mesh whose material has BUMPMAP or BUMPENVMAP (with pixel shaders) adds usage flag 0x10000 to the
/// atomic's geometry; then the saved default callback.
#[hook(addr = 0x8160A0, conv = cdecl, name = "_rpMatFXAtomicInstanceCallback", expect = "8B 44 24 0C 53")]
pub fn rp_matfx_atomic_instance_callback(a: GPtr<u8>, header: GPtr<u8>, reinstance: u32) -> u32 {
    if reinstance == 0 {
        let mut n = rd::<u32>(header, 4);
        let mut e = header.byte_add(0x48);
        loop {
            let d = ext_of(p8(rd::<u32>(e, 0)));
            if !d.is_null() {
                let f = rd::<u32>(d, 0x30);
                if (f == 1 || f == 3) && g::<u32>(PS_AVAILABLE).read() != 0 {
                    let geom = p8(rd::<u32>(a, 0x18));
                    let u = atomic_geometry_get_usage_flags(geom);
                    atomic_geometry_set_usage_flags(geom, u | 0x10000);
                }
            }
            e = e.byte_add(0x24);
            n = n.wrapping_sub(1);
            if n == 0 {
                break;
            }
        }
    }
    call_instance_cb(g::<u32>(ATOMIC_INSTANCE_CB).read(), a, header, reinstance)
}

/// `_rpMatFXWorldSectorInstanceCallback(sector, header, reinstance)` (0x816120): as the atomic variant
/// on the sector's usage flags, without the reinstance test.
#[hook(addr = 0x816120, conv = cdecl, name = "_rpMatFXWorldSectorInstanceCallback", expect = "53 55 8B 6C 24 10")]
pub fn rp_matfx_world_sector_instance_callback(s: GPtr<u8>, header: GPtr<u8>, reinstance: u32) -> u32 {
    let mut n = rd::<u32>(header, 4);
    let mut e = header.byte_add(0x48);
    loop {
        let d = ext_of(p8(rd::<u32>(e, 0)));
        if !d.is_null() {
            let f = rd::<u32>(d, 0x30);
            if (f == 1 || f == 3) && g::<u32>(PS_AVAILABLE).read() != 0 {
                let u = sector_get_usage_flags(s);
                sector_set_usage_flags(s, u | 0x10000);
            }
        }
        e = e.byte_add(0x24);
        n = n.wrapping_sub(1);
        if n == 0 {
            break;
        }
    }
    call_instance_cb(g::<u32>(SECTOR_INSTANCE_CB).read(), s, header, reinstance)
}

/// `_rpMatFXPipelinesDestroy()` (0x816190): both pipelines and the six pixel shaders released and
/// cleared. Returns 1.
#[hook(addr = 0x816190, conv = cdecl, name = "_rpMatFXPipelinesDestroy", expect = "A1 8C AB C9 00")]
pub fn rp_matfx_pipelines_destroy() -> u32 {
    for a in [ATOMIC_PIPE, SECTOR_PIPE] {
        let p = g::<u32>(a).read();
        if p != 0 {
            rx_pipeline_destroy(p8(p));
            g::<u32>(a).write(0);
        }
    }
    for (slot, _) in PS_SLOTS {
        let s = g::<u32>(slot).read();
        if s != 0 {
            rw_d3d9_delete_pixel_shader(s);
            g::<u32>(slot).write(0);
        }
    }
    1
}

/// `_rpMatFXAtomicSetPipeline(atomic)` (0x816260). EAX = atomic.
#[hook(addr = 0x816260, conv = cdecl, name = "_rpMatFXAtomicSetPipeline", expect = "8B 44 24 04 8B 0D 8C AB C9 00")]
pub fn rp_matfx_atomic_set_pipeline(a: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(a, 0x6C, g::<u32>(ATOMIC_PIPE).read());
    a
}

/// `_rpMatFXWorldSectorSetPipeline(sector)` (0x816270). EAX = sector.
#[hook(addr = 0x816270, conv = cdecl, name = "_rpMatFXWorldSectorSetPipeline", expect = "8B 44 24 04 8B 0D 90 AB C9 00")]
pub fn rp_matfx_world_sector_set_pipeline(s: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(s, 0x7C, g::<u32>(SECTOR_PIPE).read());
    s
}

/// `RpMatFXGetD3D9Pipeline(type)` (0x8162F0): 1 atomic, 2 sector, else NULL.
#[hook(addr = 0x8162F0, conv = cdecl, name = "RpMatFXGetD3D9Pipeline", expect = "8B 44 24 04 48")]
pub fn rp_matfx_get_d3d9_pipeline(ty: u32) -> u32 {
    match ty {
        1 => g::<u32>(ATOMIC_PIPE).read(),
        2 => g::<u32>(SECTOR_PIPE).read(),
        _ => 0,
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RP_MATFX_MATERIAL_DATA_SET_FREE_LIST_CREATE_PARAMS,
    HOOK_RP_MATFX_PLUGIN_ATTACH,
    HOOK_RP_MATFX_MATERIAL_DATA_DESTROY_EFFECTS,
    HOOK_RP_MATFX_MATERIAL_FIND_EFFECT,
    HOOK_RP_MATFX_ATOMIC_ENABLE_EFFECTS,
    HOOK_RP_MATFX_ATOMIC_QUERY_EFFECTS,
    HOOK_RP_MATFX_WORLD_SECTOR_ENABLE_EFFECTS,
    HOOK_RP_MATFX_WORLD_SECTOR_QUERY_EFFECTS,
    HOOK_RP_MATFX_MATERIAL_SET_EFFECTS,
    HOOK_RP_MATFX_MATERIAL_SETUP_BUMP_MAP,
    HOOK_RP_MATFX_MATERIAL_SETUP_ENV_MAP,
    HOOK_RP_MATFX_MATERIAL_SETUP_DUAL_TEXTURE,
    HOOK_RP_MATFX_MATERIAL_GET_EFFECTS,
    HOOK_RP_MATFX_MATERIAL_SET_BUMP_MAP_TEXTURE,
    HOOK_RP_MATFX_MATERIAL_SET_BUMP_MAP_FRAME,
    HOOK_RP_MATFX_MATERIAL_SET_BUMP_MAP_COEFFICIENT,
    HOOK_RP_MATFX_MATERIAL_GET_BUMP_MAP_BUMPED_TEXTURE,
    HOOK_RP_MATFX_MATERIAL_GET_BUMP_MAP_TEXTURE,
    HOOK_RP_MATFX_MATERIAL_GET_BUMP_MAP_FRAME,
    HOOK_RP_MATFX_MATERIAL_GET_BUMP_MAP_COEFFICIENT,
    HOOK_RP_MATFX_MATERIAL_SET_ENV_MAP_TEXTURE,
    HOOK_RP_MATFX_MATERIAL_SET_ENV_MAP_FRAME,
    HOOK_RP_MATFX_MATERIAL_SET_ENV_MAP_FRAME_BUFFER_ALPHA,
    HOOK_RP_MATFX_MATERIAL_SET_ENV_MAP_COEFFICIENT,
    HOOK_RP_MATFX_MATERIAL_GET_ENV_MAP_TEXTURE,
    HOOK_RP_MATFX_MATERIAL_GET_ENV_MAP_FRAME,
    HOOK_RP_MATFX_MATERIAL_GET_ENV_MAP_FRAME_BUFFER_ALPHA,
    HOOK_RP_MATFX_MATERIAL_GET_ENV_MAP_COEFFICIENT,
    HOOK_RP_MATFX_MATERIAL_SET_DUAL_TEXTURE,
    HOOK_RP_MATFX_MATERIAL_SET_DUAL_BLEND_MODES,
    HOOK_RP_MATFX_MATERIAL_GET_DUAL_TEXTURE,
    HOOK_RP_MATFX_MATERIAL_GET_DUAL_BLEND_MODES,
    HOOK_RP_MATFX_MATERIAL_SET_UV_TRANSFORM_MATRICES,
    HOOK_RP_MATFX_MATERIAL_GET_UV_TRANSFORM_MATRICES,
    HOOK_RP_MATFX_D3D9_ATOMIC_RENDER_CALLBACK,
    HOOK_RP_MATFX_PIPELINES_CREATE,
    HOOK_RP_MATFX_ATOMIC_INSTANCE_CALLBACK,
    HOOK_RP_MATFX_WORLD_SECTOR_INSTANCE_CALLBACK,
    HOOK_RP_MATFX_PIPELINES_DESTROY,
    HOOK_RP_MATFX_ATOMIC_SET_PIPELINE,
    HOOK_RP_MATFX_WORLD_SECTOR_SET_PIPELINE,
    HOOK_RP_MATFX_GET_D3D9_PIPELINE,
];

/// Globals owned by this module (MatFX state is accessed raw).
pub const GLOBALS: &[GlobalDesc] = &[];
