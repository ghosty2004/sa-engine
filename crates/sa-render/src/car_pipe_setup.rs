//! `CCustomCarEnvMapPipeline` material / atomic setup and the `CCarFXRenderer` wrappers (STD-6, batch
//! `std6-render-1`). Ported from the 1.0 US bytes (0x5DA230, 0x5DA560, 0x5DA610, 0x5D5B20..0x5D5B60).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr};

orig! {
    /// The env-map material pool's `New` (0x5D9110, thiscall)
    fn env_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x5D9110 => crate::mvpa_render2::env_map_material_pool_new;
    /// The env-map material pool's `Delete(ptr)` (0x5D9180, thiscall)
    fn env_pool_delete(pool: GPtr<u8>, p: u32) -> u32 = thiscall 0x5D9180 => crate::render_lists::pool12_delete;
    /// `RpMatFXMaterialGetEffects(material)` (0x812140)
    fn matfx_get_effects(m: GPtr<u8>) -> u32 = cdecl 0x812140 => unported;
    /// `CCustomCarEnvMapPipeline::SetFxEnvTexture(material, tex)` (0x5DA230)
    fn set_fx_env_texture_call(m: GPtr<u8>, tex: u32) = cdecl 0x5DA230 => set_fx_env_texture;
    /// `RpGeometryForAllMaterials(geometry, cb, data)` (0x74C790)
    fn geometry_for_all_materials(g: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x74C790 => unported;
    /// `PipelinePlugin::SetPipelineID(atomic, id)` (0x72FC50)
    fn set_pipeline_id(a: GPtr<u8>, id: u32) = cdecl 0x72FC50 => crate::pipe_ids::set_pipeline_id;
    /// `CCustomCarEnvMapPipeline::CustomPipeAtomicSetup(atomic)` (0x5DA610)
    fn custom_pipe_atomic_setup_call(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x5DA610 => custom_pipe_atomic_setup;
    /// `RpClumpForAllAtomics(clump, cb, data)` (0x749B70)
    fn clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// The env-map record duplication `(slot)` (0x5D9570)
    fn env_record_own_call(slot: GPtr<u32>) -> u32 = cdecl 0x5D9570 => env_record_own;
    /// `_rpD3D9AtomicGeometrySetUsageFlags(geometry, flags)` (0x7588B0)
    fn geometry_set_usage_flags(g: GPtr<u8>, flags: u32) = cdecl 0x7588B0 => unported;
    /// `CCustomBuildingDNPipeline::GetExtraVertColourPtr(geometry)` (0x5D6E90)
    fn get_extra_vert_colour_ptr(g: GPtr<u8>) -> u32 = cdecl 0x5D6E90 => crate::pipe_ids::get_extra_vert_colour_ptr;
    /// `CCustomBuildingDNPipeline::CustomPipeAtomicSetup` (0x5D71C0), `CCustomBuildingPipeline::CustomPipeAtomicSetup` (0x5D7E50)
    fn dn_atomic_setup_call(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x5D71C0 => building_dn_atomic_setup;
    fn building_atomic_setup_call(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x5D7E50 => building_atomic_setup;
}

/// Material plugin offsets: env-map data (`[0x8D12C4]`), specular data (`[0x8D12CC]`); MatFX data
/// (`[0xC9AB74]`).
const ENV_OFFSET: usize = 0x8D12C4;
const SPEC_OFFSET: usize = 0x8D12CC;
const MATFX_OFFSET: usize = 0xC9AB74;
/// The shared default env-map record, the env-map material pool, the car pipeline.
const FAKE_ENV: u32 = 0xC02D18;
const ENV_POOL: usize = 0xC02D28;
const CAR_PIPE: usize = 0xC02D24;
/// 1/255 (0x859A3C)
const INV_255: f32 = f32::from_bits(0x3B80_8081);

fn mat_slot(m: GPtr<u8>, off: usize) -> GPtr<u32> {
    m.byte_add(g::<i32>(off).read() as isize).cast()
}

/// `(float)shininess * (1/255) != 0` as the original's `fild; fmul; fcomp; test ah, 0x44` computes it.
fn shininess_nonzero(env: GPtr<u8>) -> bool {
    let v = X87::from_i32(rd::<u8>(env, 4) as i32).fmul_m(INV_255);
    v.to_f64() != 0.0
}

/// `CCustomCarEnvMapPipeline::SetFxEnvTexture(material, texture)` (0x5DA230): a material still on the
/// shared default record (0xC02D18) gets its own pool entry (a copy of the default; none when the pool
/// is full: return). The entry's texture (+8) = `texture`, or with NULL the material's MatFX slot-0
/// word +4; a NULL texture or a zero shininess (+4) returns the entry to the pool and puts the material
/// back on the default record. The resulting record's texture (when any) gets filtering 2 (+0x50) and
/// addressing 0x11 (+0x51).
#[hook(addr = 0x5DA230, conv = cdecl, name = "CCustomCarEnvMapPipeline::SetFxEnvTexture", expect = "A1 C4 12 8D 00 56 57",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn set_fx_env_texture(m: GPtr<u8>, tex: u32) {
    let slot = mat_slot(m, ENV_OFFSET);
    let mut e = slot.read();
    if e == FAKE_ENV {
        let p = env_pool_new(GPtr::from_addr(g::<u32>(ENV_POOL).read() as usize));
        slot.write(p.addr() as u32);
        if p.is_null() {
            return;
        }
        for o in [0, 4, 8] {
            wr::<u32>(p, o, g::<u32>(FAKE_ENV as usize + o as usize).read());
        }
        e = p.addr() as u32;
    }
    if e == 0 {
        return;
    }
    let env = GPtr::<u8>::from_addr(e as usize);
    let mut cur = env;
    if tex == 0 {
        let t = if m.is_null() { 0 } else { rd::<u32>(GPtr::from_addr(mat_slot(m, MATFX_OFFSET).read() as usize), 4) };
        wr::<u32>(env, 8, t);
        if t == 0 || !shininess_nonzero(env) {
            env_pool_delete(GPtr::from_addr(g::<u32>(ENV_POOL).read() as usize), e);
            mat_slot(m, ENV_OFFSET).write(FAKE_ENV);
            cur = GPtr::from_addr(FAKE_ENV as usize);
        }
    } else {
        wr::<u32>(env, 8, tex);
    }
    let t = rd::<u32>(cur, 8);
    if t != 0 {
        let tp = GPtr::<u8>::from_addr(t as usize);
        wr::<u8>(tp, 0x51, 0x11);
        wr::<u8>(tp, 0x50, 2);
    }
}

/// `CCustomCarEnvMapPipeline::CustomPipeMaterialSetup(material, data)` (0x5DA560): +0x10 = 0; an
/// env-map MatFX material gets its env texture (`SetFxEnvTexture(material, NULL)`); then the low 3 bits
/// of +0x10: 1 (2 for a texture whose name starts with 'x') with an env record of non-zero shininess
/// and a texture, | 4 with a specular record of non-zero specularity and a texture. Returns the
/// material.
#[hook(addr = 0x5DA560, conv = cdecl, name = "CCustomCarEnvMapPipeline::CustomPipeMaterialSetup", expect = "56 8B 74 24 08 56 C7 46 10 00 00 00 00",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn custom_pipe_material_setup(m: GPtr<u8>, _data: u32) -> GPtr<u8> {
    wr::<u32>(m, 0x10, 0);
    if matfx_get_effects(m) == 2 {
        set_fx_env_texture_call(m, 0);
    }
    let mut flags = 0u32;
    let env = mat_slot(m, ENV_OFFSET).read();
    if env != 0 {
        let ep = GPtr::<u8>::from_addr(env as usize);
        if shininess_nonzero(ep) && rd::<u32>(ep, 8) != 0 {
            let tex = GPtr::<u8>::from_addr(rd::<u32>(ep, 8) as usize);
            flags = (rd::<u8>(tex, 0x10) == b'x') as u32 + 1;
        }
    }
    let spec = mat_slot(m, SPEC_OFFSET).read();
    if spec != 0 {
        let sp = GPtr::<u8>::from_addr(spec as usize);
        if rd::<f32>(sp, 0) != 0.0 && rd::<u32>(sp, 4) != 0 {
            flags |= 4;
        }
    }
    wr::<u32>(m, 0x10, (rd::<u32>(m, 0x10) & !7) | flags);
    m
}

/// `CCustomCarEnvMapPipeline::CustomPipeAtomicSetup(atomic)` (0x5DA610; the material callback address
/// 0x5DA560 comes from the protection chunk 0x401B0C): every material set up, the atomic's pipeline
/// (+0x6C) = the car pipeline, pipeline id 0x53F2009A. Returns the atomic.
#[hook(addr = 0x5DA610, conv = cdecl, name = "CCustomCarEnvMapPipeline::CustomPipeAtomicSetup", expect = "56 8B 74 24 08 8B 46 18",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn custom_pipe_atomic_setup(atomic: GPtr<u8>) -> GPtr<u8> {
    geometry_for_all_materials(GPtr::from_addr(rd::<u32>(atomic, 0x18) as usize), 0x5DA560, 0);
    wr::<u32>(atomic, 0x6C, g::<u32>(CAR_PIPE).read());
    set_pipeline_id(atomic, 0x53F2_009A);
    atomic
}

/// `CCarFXRenderer::CustomCarPipeAtomicSetup(atomic)` (0x5D5B20): `jmp CustomPipeAtomicSetup`.
#[sa_core::native(addr = 0x5D5B20, conv = cdecl, name = "CCarFXRenderer::CustomCarPipeAtomicSetup")]
pub fn custom_car_pipe_atomic_setup(atomic: GPtr<u8>) -> GPtr<u8> {
    custom_pipe_atomic_setup_call(atomic)
}

/// `CCarFXRenderer::CustomCarPipeClumpSetup(clump)` (0x5D5B40): every atomic set up. Returns the clump.
#[hook(addr = 0x5D5B40, conv = cdecl, name = "CCarFXRenderer::CustomCarPipeClumpSetup", expect = "56 8B 74 24 08 6A 00 68 30 5B 5D 00",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn custom_car_pipe_clump_setup(clump: GPtr<u8>) -> GPtr<u8> {
    clump_for_all_atomics(clump, 0x5D5B30, 0);
    clump
}

/// `CCarFXRenderer::SetCustomFXAtomicRenderPipelinesVMICB(atomic, data)` (0x5D5B60). Returns the atomic.
#[hook(addr = 0x5D5B60, conv = cdecl, name = "CCarFXRenderer::SetCustomFXAtomicRenderPipelinesVMICB", expect = "56 8B 74 24 08 56 E8 A5 4A 00 00",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes")]
pub fn set_custom_fx_atomic_render_pipelines_vmi_cb(atomic: GPtr<u8>, _data: u32) -> GPtr<u8> {
    custom_pipe_atomic_setup_call(atomic);
    atomic
}

/// The env-map record duplication (0x5D9570, catalogued as `CCustomBuildingDNPipeline::SetFxEnvTexture`):
/// a slot still on the shared default record (0xC02D18) gets its own pool entry, a copy of the default
/// (NULL when the pool is full). Returns the slot's record.
#[hook(addr = 0x5D9570, conv = cdecl, name = "CCustomBuildingDNPipeline::SetFxEnvTexture", expect = "56 8B 74 24 08 8B 06 3D 18 2D C0 00",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes")]
pub fn env_record_own(slot: GPtr<u32>) -> u32 {
    let e = slot.read();
    if e != FAKE_ENV {
        return e;
    }
    let p = env_pool_new(GPtr::from_addr(g::<u32>(ENV_POOL).read() as usize));
    slot.write(p.addr() as u32);
    if p.is_null() {
        return 0;
    }
    for o in [0, 4, 8] {
        wr::<u32>(p, o, g::<u32>(FAKE_ENV as usize + o as usize).read());
    }
    p.addr() as u32
}

/// The building pipelines' material setup (0x5D7120 DN, 0x5D7DB0 plain; identical bytes): +0x10 = 0; an
/// env-map MatFX material gets its own env record whose texture is the MatFX slot-0 texture (filtering
/// 2, addressing 0x11); flag 1 with an env record of non-zero shininess and a texture.
fn building_material_setup(m: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(m, 0x10, 0);
    if matfx_get_effects(m) == 2 {
        let e = env_record_own_call(mat_slot(m, ENV_OFFSET));
        if e != 0 {
            let t = rd::<u32>(GPtr::from_addr(mat_slot(m, MATFX_OFFSET).read() as usize), 4);
            wr::<u32>(GPtr::from_addr(e as usize), 8, t);
            if t != 0 {
                wr::<u8>(GPtr::from_addr(t as usize), 0x51, 0x11);
                wr::<u8>(GPtr::from_addr(t as usize), 0x50, 2);
            }
        }
    }
    let env = mat_slot(m, ENV_OFFSET).read();
    let mut flags = 0;
    if env != 0 {
        let ep = GPtr::<u8>::from_addr(env as usize);
        if shininess_nonzero(ep) && rd::<u32>(ep, 8) != 0 {
            flags = 1;
        }
    }
    wr::<u32>(m, 0x10, (rd::<u32>(m, 0x10) & !7) | flags);
    m
}

/// `CCustomBuildingDNPipeline::CustomPipeMaterialSetup(material, data)` (0x5D7120).
#[hook(addr = 0x5D7120, conv = cdecl, name = "CCustomBuildingDNPipeline::CustomPipeMaterialSetup", expect = "56 8B 74 24 08 56 C7 46 10 00 00 00 00 E8 0E B0 23 00",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn building_dn_material_setup(m: GPtr<u8>, _data: u32) -> GPtr<u8> {
    building_material_setup(m)
}

/// `CCustomBuildingPipeline::CustomPipeMaterialSetup(material, data)` (0x5D7DB0).
#[hook(addr = 0x5D7DB0, conv = cdecl, name = "CCustomBuildingPipeline::CustomPipeMaterialSetup", expect = "56 8B 74 24 08 56 C7 46 10 00 00 00 00 E8 7E A3 23 00",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn building_plain_material_setup(m: GPtr<u8>, _data: u32) -> GPtr<u8> {
    building_material_setup(m)
}

/// `CCustomBuildingDNPipeline::CustomPipeAtomicSetup(atomic)` (0x5D71C0): materials set up, geometry
/// usage flag 8, the DN pipeline (+0x6C = `[0xC02C1C]`), pipeline id 0x53F20098. Returns the atomic.
#[hook(addr = 0x5D71C0, conv = cdecl, name = "CCustomBuildingDNPipeline::CustomPipeAtomicSetup", expect = "56 8B 74 24 08 57 8B 7E 18",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn building_dn_atomic_setup(atomic: GPtr<u8>) -> GPtr<u8> {
    let geom = GPtr::<u8>::from_addr(rd::<u32>(atomic, 0x18) as usize);
    geometry_for_all_materials(geom, 0x5D7120, 0);
    geometry_set_usage_flags(geom, 8);
    wr::<u32>(atomic, 0x6C, g::<u32>(0xC02C1C).read());
    set_pipeline_id(atomic, 0x53F2_0098);
    atomic
}

/// `CCustomBuildingPipeline::CustomPipeAtomicSetup(atomic)` (0x5D7E50): materials set up, the building
/// pipeline (`[0xC02C68]`), pipeline id 0x53F2009C. Returns the atomic.
#[hook(addr = 0x5D7E50, conv = cdecl, name = "CCustomBuildingPipeline::CustomPipeAtomicSetup", expect = "56 8B 74 24 08 8B 46 18 6A 00",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn building_atomic_setup(atomic: GPtr<u8>) -> GPtr<u8> {
    geometry_for_all_materials(GPtr::from_addr(rd::<u32>(atomic, 0x18) as usize), 0x5D7DB0, 0);
    wr::<u32>(atomic, 0x6C, g::<u32>(0xC02C68).read());
    set_pipeline_id(atomic, 0x53F2_009C);
    atomic
}

/// `CCustomBuildingRenderer::AtomicSetup(atomic)` (0x5D7F00): the DN pipeline for a geometry with night
/// colours and a non-zero +0x30 word, else the plain building pipeline.
#[hook(addr = 0x5D7F00, conv = cdecl, name = "CCustomBuildingRenderer::AtomicSetup", expect = "56 57 8B 7C 24 0C 8B 77 18",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn building_renderer_atomic_setup(atomic: GPtr<u8>) -> GPtr<u8> {
    let geom = GPtr::<u8>::from_addr(rd::<u32>(atomic, 0x18) as usize);
    if get_extra_vert_colour_ptr(geom) != 0 && rd::<u32>(geom, 0x30) != 0 {
        dn_atomic_setup_call(atomic)
    } else {
        building_atomic_setup_call(atomic)
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_ENV_RECORD_OWN,
    HOOK_BUILDING_DN_MATERIAL_SETUP,
    HOOK_BUILDING_PLAIN_MATERIAL_SETUP,
    HOOK_BUILDING_DN_ATOMIC_SETUP,
    HOOK_BUILDING_ATOMIC_SETUP,
    HOOK_BUILDING_RENDERER_ATOMIC_SETUP,
    HOOK_SET_FX_ENV_TEXTURE,
    HOOK_CUSTOM_PIPE_MATERIAL_SETUP,
    HOOK_CUSTOM_PIPE_ATOMIC_SETUP,
    HOOK_CUSTOM_CAR_PIPE_CLUMP_SETUP,
    HOOK_SET_CUSTOM_FX_ATOMIC_RENDER_PIPELINES_VMI_CB,
];

/// Phase B natives of this module.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[NATIVE_CUSTOM_CAR_PIPE_ATOMIC_SETUP];
