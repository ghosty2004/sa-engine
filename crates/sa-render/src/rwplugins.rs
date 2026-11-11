//! PORT-RENDER (boot): the game-side RenderWare plugins `App::PluginAttach` registers (node names,
//! `CVisibilityPlugins`, `RpAnimBlend`, the building / car pipelines, breakables, collision, 2d effects,
//! `PipelinePlugin`), their constructor / destructor / copy callbacks and the short stream callbacks,
//! plus `CVisibilityPlugins::Initialise`. Ported from the 1.0 US bytes; gta-reversed as reading aid.
//!
//! RW plugin callbacks are cdecl: constructor / destructor `(object, offset, size) -> object`, copy
//! `(dst, src, offset, size) -> dst`, stream read / write `(stream, length, object, offset, size) ->
//! stream`, get-size `(object, offset, size) -> int32`. Where the original uses the offset global
//! instead of the `offset` argument (or the argument instead of the global), so does the port; a
//! global the original reloads after a call or store is reloaded here too.

use sa_core::abi::{HookReg, NativeReg};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{rd, wr};

sa_core::globals! {
    list GLOBALS;
    /// `NodeNamePlugin` offset in RwFrame (`gPluginOffset`, .bss)
    pub static NODE_NAME_OFFSET @ 0xC87C5C: i32 = 0;
    /// `PipelinePlugin` offset in RpAtomic (.data -1)
    pub static PIPELINE_OFFSET @ 0x8D6080: i32 = -1;
    /// `CVisibilityPlugins::ms_atomicPluginOffset` (.data -1)
    pub static VIS_ATOMIC_OFFSET @ 0x8D608C: i32 = -1;
    /// `CVisibilityPlugins::ms_framePluginOffset` (.data -1)
    pub static VIS_FRAME_OFFSET @ 0x8D6090: i32 = -1;
    /// `CVisibilityPlugins::ms_clumpPluginOffset` (.data -1)
    pub static VIS_CLUMP_OFFSET @ 0x8D6094: i32 = -1;
    /// `CCustomBuildingDNPipeline::ms_extraVertColourPluginOffset` (.data -1)
    pub static EXTRA_VERT_COLOUR_OFFSET @ 0x8D12BC: i32 = -1;
    /// `CCustomCarEnvMapPipeline::ms_envMapPluginOffset` (.data -1)
    pub static ENV_MAP_OFFSET @ 0x8D12C4: i32 = -1;
    /// `CCustomCarEnvMapPipeline::ms_envMapAtmPluginOffset` (.data -1)
    pub static ENV_MAP_ATM_OFFSET @ 0x8D12C8: i32 = -1;
    /// `CCustomCarEnvMapPipeline::ms_specularMapPluginOffset` (.data -1)
    pub static SPEC_MAP_OFFSET @ 0x8D12CC: i32 = -1;
    /// `CCustomCarEnvMapPipeline::fakeEnvMapPipeMatData` (0xC bytes, .bss): two scale pairs (bytes),
    /// shininess byte, pad, `renderFrameCounter` word, texture
    pub static FAKE_ENV_MAP_MAT_DATA @ 0xC02D18: [u8; 0xC] = [0; 0xC];
    /// `CCustomCarEnvMapPipeline::m_gEnvMapPipeMatDataPool` (CPool*, entries 0xC)
    pub static ENV_MAT_POOL @ 0xC02D28: GPtr<u8> = GPtr::NULL;
    /// `CCustomCarEnvMapPipeline::m_gEnvMapPipeAtmDataPool` (CPool*, entries 0xC)
    pub static ENV_ATM_POOL @ 0xC02D2C: GPtr<u8> = GPtr::NULL;
    /// `CCustomCarEnvMapPipeline::m_gSpecMapPipeMatDataPool` (CPool*, entries 8)
    pub static SPEC_MAT_POOL @ 0xC02D30: GPtr<u8> = GPtr::NULL;
    /// `BreakablePlugin` offset in RpGeometry (`g_BreakablePlugin`, .bss)
    pub static BREAKABLE_OFFSET @ 0xBB4238: i32 = 0;
    /// `CCollisionPlugin::ms_offset` (RpClump, .bss)
    pub static COLLISION_OFFSET @ 0x9689DC: i32 = 0;
    /// `RpAnimBlend` clump data offset (`ClumpOffset`, .bss)
    pub static ANIM_BLEND_CLUMP_OFFSET @ 0xB5F878: i32 = 0;
    /// `CVisibilityPlugins` sorted lists (`m_alphaList` 0xC88070 … `ms_weaponPedsForPC` 0xC88224,
    /// `CLinkList` = used head / tail, free head / tail links, `links` pointer)
    pub static VIS_LISTS @ 0xC88070: [u8; 0x1E8] = [0; 0x1E8];
}

/// `C2dEffect` plugin offset in RpGeometry (`g2dEffectPluginOffset`, declared by sa-stream).
use sa_stream::modelinfo::base::G2D_EFFECT_PLUGIN_OFFSET;

// plugin ids (rwID_* of Rockstar's vendor 0x253F2xx)
const ID_VIS_ATOMIC: u32 = 0x253F200;
const ID_VIS_CLUMP: u32 = 0x253F201;
const ID_VIS_FRAME: u32 = 0x253F202;
const ID_PIPELINE: u32 = 0x253F2F3;
const ID_ENV_ATM: u32 = 0x253F2F4;
const ID_SPEC_MAT: u32 = 0x253F2F6;
const ID_2D_EFFECT: u32 = 0x253F2F8;
const ID_EXTRA_VERT_COLOUR: u32 = 0x253F2F9;
const ID_COLLISION: u32 = 0x253F2FA;
const ID_ANIM_BLEND: u32 = 0x253F2FB;
const ID_ENV_MAT: u32 = 0x253F2FC;
const ID_BREAKABLE: u32 = 0x253F2FD;
const ID_NODE_NAME: u32 = 0x253F2FE;

/// `CVisibilityPlugins::DefaultVisibilityCB` (0x732A30), the clump constructor's callback.
const DEFAULT_VISIBILITY_CB: u32 = 0x732A30;

orig! {
    /// `RwFrameRegisterPlugin(size, id, ctor, dtor, copy)` (0x7F1260)
    pub fn rw_frame_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x7F1260 => unported;
    /// `RwFrameRegisterPluginStream(id, read, write, getSize)` (0x807580)
    pub fn rw_frame_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x807580 => unported;
    /// `RpAtomicRegisterPlugin` (0x74BDA0)
    pub fn rp_atomic_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74BDA0 => unported;
    /// `RpAtomicRegisterPluginStream` (0x74BE00)
    pub fn rp_atomic_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x74BE00 => unported;
    /// `RpClumpRegisterPlugin` (0x74BDD0)
    pub fn rp_clump_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74BDD0 => unported;
    /// `RpClumpRegisterPluginStream` (0x74BE70)
    pub fn rp_clump_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x74BE70 => unported;
    /// `RpGeometryRegisterPlugin` (0x74CD70)
    pub fn rp_geometry_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74CD70 => unported;
    /// `RpGeometryRegisterPluginStream` (0x74CDA0)
    pub fn rp_geometry_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x74CDA0 => unported;
    /// `RpMaterialRegisterPlugin` (0x74DBF0)
    pub fn rp_material_register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x74DBF0 => unported;
    /// `RpMaterialRegisterPluginStream` (0x74DC20)
    pub fn rp_material_register_plugin_stream(id: u32, read: u32, write: u32, size: u32) -> i32 = cdecl 0x74DC20 => unported;
    /// `RtAnimRegisterInterpolationScheme(RtAnimInterpolatorInfo*)` (0x7CCD40)
    pub fn rt_anim_register_interpolation_scheme(info: GPtr<u8>) -> i32 = cdecl 0x7CCD40 => unported;
    /// `RwStreamRead(stream, buffer, length)` (0x7EC9D0)
    pub fn rw_stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => unported;
    /// `RwStreamWrite(stream, buffer, length)` (0x7ECB30)
    pub fn rw_stream_write(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> GPtr<u8> = cdecl 0x7ECB30 => unported;
    /// `rwstrlen` through `RwEngineInstance` +0x120
    pub fn rw_strlen(s: GPtr<u8>) -> i32 = cdecl table 0xC97B24 0x120 => unported;
    /// CRT `strncpy` (0x821F40)
    pub fn strncpy(d: GPtr<u8>, s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `operator new` (0x821195, the `new[]` jump)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// `operator delete` (0x8214BD)
    pub fn operator_delete(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CMemoryMgr::Malloc(size, hint)` (0x72F420; the callers push a second 0)
    pub fn memory_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `CMemoryMgr::Free(p)` (0x72F430)
    pub fn memory_free(p: GPtr<u8>) = cdecl 0x72F430 => unported;
    /// `RwTextureDestroy` (0x7F3820)
    pub fn rw_texture_destroy(t: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
    /// `RpAtomicSetFrame(atomic, frame)` (0x74BF20)
    pub fn rp_atomic_set_frame(a: GPtr<u8>, f: GPtr<u8>) -> GPtr<u8> = cdecl 0x74BF20 => unported;
    /// `RwFrameDestroy` (0x7F05A0)
    pub fn rw_frame_destroy(f: GPtr<u8>) -> i32 = cdecl 0x7F05A0 => unported;
    /// `RpAtomicDestroy` (0x749DC0)
    pub fn rp_atomic_destroy(a: GPtr<u8>) -> i32 = cdecl 0x749DC0 => unported;
    /// `RpAnimBlendClumpRemoveAllAssociations(clump)` (0x4D6C00)
    pub fn anim_blend_clump_remove_all_associations(c: GPtr<u8>) = cdecl 0x4D6C00 => unported;
    /// `CAnimBlendClumpData::~CAnimBlendClumpData()` (0x4CF100)
    pub fn anim_blend_clump_data_dtor(this: GPtr<u8>) = thiscall 0x4CF100 => unported;
    /// `CPool<CustomEnvMapPipeMaterialDataStruct>::New()` (0x5D9110)
    pub fn env_mat_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x5D9110 => unported;
    /// `CPool<CustomEnvMapPipeAtomicDataStruct>::New()` (0x5D9260)
    pub fn env_atm_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x5D9260 => unported;
    /// `CPool<CustomSpecMapPipeMaterialDataStruct>::New()` (0x5D93B0)
    pub fn spec_mat_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x5D93B0 => unported;
}

/// `obj + off` (the original's `[reg + reg]` addressing, 32-bit wrapping).
fn at_off(obj: GPtr<u8>, off: i32) -> GPtr<u8> {
    obj.byte_add(off as isize)
}

// ------------------------------------------------------------------------------ attaches ---------

/// `NodeNamePlugin::NodeNamePluginAttach` (0x72FAB0): RwFrame plugin (0x18 bytes) and its stream
/// callbacks (result ignored); returns `offset != -1` (offset reloaded).
#[hook(addr = 0x72FAB0, conv = cdecl, name = "NodeNamePlugin::NodeNamePluginAttach", expect = "68 F0 F9 72 00")]
pub fn node_name_plugin_attach() -> i32 {
    let off = rw_frame_register_plugin(0x18, ID_NODE_NAME, 0x72F9C0, 0x72F9E0, 0x72F9F0);
    NODE_NAME_OFFSET.set(off);
    rw_frame_register_plugin_stream(ID_NODE_NAME, 0x72FA50, 0x72FA20, 0x72FA80);
    (NODE_NAME_OFFSET.get() != -1) as i32
}

/// `CVisibilityPlugins::PluginAttach` (0x732E30): atomic (4 bytes), frame (4) and clump (8) plugins.
/// Only the atomic and clump offsets are checked (the frame offset is not, as in the original).
#[hook(addr = 0x732E30, conv = cdecl, name = "CVisibilityPlugins::PluginAttach", expect = "68 70 21 73 00")]
pub fn visibility_plugins_attach() -> i32 {
    let a = rp_atomic_register_plugin(4, ID_VIS_ATOMIC, 0x732150, 0x7321A0, 0x732170);
    VIS_ATOMIC_OFFSET.set(a);
    let f = rw_frame_register_plugin(4, ID_VIS_FRAME, 0x7321B0, 0x7321F0, 0x7321D0);
    VIS_FRAME_OFFSET.set(f);
    let c = rp_clump_register_plugin(8, ID_VIS_CLUMP, 0x732E10, 0x732220, 0x732200);
    VIS_CLUMP_OFFSET.set(c);
    if VIS_ATOMIC_OFFSET.get() == -1 || c == -1 {
        return 0;
    }
    1
}

/// `RpAnimBlend::RpAnimBlendPluginAttach` (0x4D6150): clump plugin (4 bytes); on success registers
/// the interpolation scheme from a stack `RtAnimInterpolatorInfo` (type id, interp key frame size 0x1C,
/// anim key frame size 0x24, apply 0x4D5FA0, blend 0x7C60C0, interpolate 0x4D60C0, add 0x7C6720,
/// mul-recip 0x7C65C0, stream read 0x7C64C0, write 0x7C6540, get-size 0x7C65B0, custom data size 0).
#[hook(addr = 0x4D6150, conv = cdecl, name = "RpAnimBlend::RpAnimBlendPluginAttach", expect = "83 EC 30 68 90 5F 4D 00")]
pub fn rp_anim_blend_plugin_attach() -> i32 {
    let off = rp_clump_register_plugin(4, ID_ANIM_BLEND, 0x4D5F40, 0x4D6110, 0x4D5F90);
    ANIM_BLEND_CLUMP_OFFSET.set(off);
    if off < 0 {
        return 0;
    }
    let mut info: [u32; 12] = [
        ID_ANIM_BLEND,
        0x1C,
        0x24,
        0x4D5FA0,
        0x7C60C0,
        0x4D60C0,
        0x7C6720,
        0x7C65C0,
        0x7C64C0,
        0x7C6540,
        0x7C65B0,
        0,
    ];
    rt_anim_register_interpolation_scheme(GPtr::from_raw(info.as_mut_ptr()).cast());
    1
}

/// Shared tail of the geometry / atomic attaches: stream registration; a negative result resets the
/// offset to -1; returns `offset != -1` with the offset reloaded.
fn stream_tail(r: i32, offset: &sa_core::mem::Global<i32>) -> i32 {
    if r < 0 {
        offset.set(-1);
        return 0;
    }
    (offset.get() != -1) as i32
}

/// `CCustomBuildingDNPipeline::ExtraVertColourPluginAttach` (0x5D72E0): offset = -1, geometry
/// plugin (0xC bytes, no copy), then its stream callbacks.
#[hook(addr = 0x5D72E0, conv = cdecl, name = "CCustomBuildingDNPipeline::ExtraVertColourPluginAttach", expect = "6A 00 68 30 6D 5D 00")]
pub fn extra_vert_colour_plugin_attach() -> i32 {
    EXTRA_VERT_COLOUR_OFFSET.set(-1);
    let off = rp_geometry_register_plugin(0xC, ID_EXTRA_VERT_COLOUR, 0x5D6D10, 0x5D6D30, 0);
    EXTRA_VERT_COLOUR_OFFSET.set(off);
    if off == -1 {
        return 0;
    }
    let r = rp_geometry_register_plugin_stream(ID_EXTRA_VERT_COLOUR, 0x5D6DE0, 0x5D6D80, 0x5D6DC0);
    stream_tail(r, &EXTRA_VERT_COLOUR_OFFSET)
}

orig! {
    /// `CCustomBuildingDNPipeline::ExtraVertColourPluginAttach` (0x5D72E0)
    pub fn extra_vert_colour_plugin_attach_call() -> i32 = cdecl 0x5D72E0 => extra_vert_colour_plugin_attach;
    /// `CCustomCarEnvMapPipeline::RegisterPlugin` (0x5DA450)
    pub fn env_map_register_plugin_call() -> i32 = cdecl 0x5DA450 => env_map_register_plugin;
}

/// `CCustomBuildingRenderer::PluginAttach` (0x5D7EF0): `ExtraVertColourPluginAttach() != 0`
/// (`neg; sbb; neg`).
#[hook(addr = 0x5D7EF0, conv = cdecl, name = "CCustomBuildingRenderer::PluginAttach", expect = "E8 EB F3 FF FF")]
pub fn custom_building_renderer_plugin_attach() -> i32 {
    (extra_vert_colour_plugin_attach_call() != 0) as i32
}

/// `CCustomCarEnvMapPipeline::RegisterPlugin` (0x5DA450): env-map material plugin + stream, the
/// default material data (`fakeEnvMapPipeMatData`: scales 8, 8, 8, 8, shininess 0xFF, texture NULL,
/// frame counter 0, stored in that order), env-map atomic plugin, specular material plugin + stream.
/// A failed stream registration resets that offset to -1.
#[hook(addr = 0x5DA450, conv = cdecl, name = "CCustomCarEnvMapPipeline::RegisterPlugin", expect = "68 00 96 5D 00")]
pub fn env_map_register_plugin() -> i32 {
    let m = rp_material_register_plugin(4, ID_ENV_MAT, 0x5D8BD0, 0x5D95B0, 0x5D9600);
    ENV_MAP_OFFSET.set(m);
    if m < 0 {
        return 0;
    }
    if rp_material_register_plugin_stream(ID_ENV_MAT, 0x5D9660, 0x5D8CD0, 0x5D8D10) < 0 {
        ENV_MAP_OFFSET.set(-1);
        return 0;
    }
    let fake = FAKE_ENV_MAP_MAT_DATA.ptr().cast::<u8>();
    for i in 0..4 {
        wr::<u8>(fake, i, 8);
    }
    wr::<u8>(fake, 4, 0xFF);
    wr::<u32>(fake, 8, 0);
    wr::<u16>(fake, 6, 0);
    let a = rp_atomic_register_plugin(4, ID_ENV_ATM, 0x5D8D30, 0x5D9730, 0x5D9780);
    ENV_MAP_ATM_OFFSET.set(a);
    if a < 0 {
        return 0;
    }
    let s = rp_material_register_plugin(4, ID_SPEC_MAT, 0x5D8D40, 0x5D97D0, 0x5D9830);
    SPEC_MAP_OFFSET.set(s);
    if s < 0 {
        return 0;
    }
    if rp_material_register_plugin_stream(ID_SPEC_MAT, 0x5D9880, 0x5D8D60, 0x5D8DD0) < 0 {
        SPEC_MAP_OFFSET.set(-1);
        return 0;
    }
    1
}

/// `CCarFXRenderer::RegisterPlugins` (0x5D5B00): `CCustomCarEnvMapPipeline::RegisterPlugin() != 0`.
#[hook(addr = 0x5D5B00, conv = cdecl, name = "CCarFXRenderer::RegisterPlugins", expect = "E8 4B 49 00 00")]
pub fn car_fx_renderer_register_plugins() -> i32 {
    (env_map_register_plugin_call() != 0) as i32
}

/// `BreakablePlugin::BreakablePluginAttach` (0x59D100): geometry plugin (4 bytes, no copy) + stream.
#[hook(addr = 0x59D100, conv = cdecl, name = "BreakablePlugin::BreakablePluginAttach", expect = "6A 00 68 90 CD 59 00")]
pub fn breakable_plugin_attach() -> i32 {
    let off = rp_geometry_register_plugin(4, ID_BREAKABLE, 0x59CD70, 0x59CD90, 0);
    BREAKABLE_OFFSET.set(off);
    if off == -1 {
        return 0;
    }
    let r = rp_geometry_register_plugin_stream(ID_BREAKABLE, 0x59CEC0, 0x59CDE0, 0x59D0F0);
    stream_tail(r, &BREAKABLE_OFFSET)
}

/// `CCollisionPlugin::PluginAttach` (0x41B310, `.HOODLUM` stub → 0x15671D0): clump plugin of size 0
/// and its stream callbacks; both results ignored, returns true (AL).
#[hook(addr = 0x41B310, conv = cdecl, name = "CCollisionPlugin::PluginAttach", expect = "E9 BB BE 14 01")]
pub fn collision_plugin_attach() -> u8 {
    let off = rp_clump_register_plugin(0, ID_COLLISION, 0x41B1A0, 0x41B1C0, 0x41B1B0);
    COLLISION_OFFSET.set(off);
    rp_clump_register_plugin_stream(ID_COLLISION, 0x41B1D0, 0x41B2F0, 0x41B300);
    1
}

/// `C2dEffect::PluginAttach` (0x6FA970): geometry plugin (4 bytes) + stream (result ignored);
/// returns `offset != -1` (reloaded; `setne al` over a zeroed EAX).
#[hook(addr = 0x6FA970, conv = cdecl, name = "C2dEffect::PluginAttach", expect = "68 B0 9F 6F 00")]
pub fn effect_2d_plugin_attach() -> u8 {
    let off = rp_geometry_register_plugin(4, ID_2D_EFFECT, 0x6F9F90, 0x6FA880, 0x6F9FB0);
    G2D_EFFECT_PLUGIN_OFFSET.set(off as u32);
    rp_geometry_register_plugin_stream(ID_2D_EFFECT, 0x6F9FD0, 0x6FA620, 0x6FA630);
    (G2D_EFFECT_PLUGIN_OFFSET.get() != u32::MAX) as u8
}

/// `PipelinePlugin::PipelinePluginAttach` (0x72FBD0): atomic plugin (4 bytes, no destructor) and its
/// stream (read + get-size, no write).
#[hook(addr = 0x72FBD0, conv = cdecl, name = "PipelinePlugin::PipelinePluginAttach", expect = "68 70 FB 72 00")]
pub fn pipeline_plugin_attach() -> i32 {
    let off = rp_atomic_register_plugin(4, ID_PIPELINE, 0x72FB50, 0, 0x72FB70);
    PIPELINE_OFFSET.set(off);
    if off == -1 {
        return 0;
    }
    let r = rp_atomic_register_plugin_stream(ID_PIPELINE, 0x72FB90, 0, 0x72FBC0);
    stream_tail(r, &PIPELINE_OFFSET)
}

// ------------------------------------------------------------------- NodeName callbacks ---------

/// `cb_RSNODENAME_RwFrame_Ctor` (0x72F9C0): name[0] = 0 when the offset is positive.
#[hook(addr = 0x72F9C0, conv = cdecl, name = "cb_RSNODENAME_RwFrame_Ctor", expect = "8B 0D 5C 7C C8 00")]
pub fn node_name_ctor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let off = NODE_NAME_OFFSET.get();
    if off > 0 {
        wr::<u8>(at_off(obj, off), 0, 0);
    }
    obj
}

/// `cb_RSNODENAME_RwFrame_Dtor` (0x72F9E0): returns the frame.
#[hook(addr = 0x72F9E0, conv = cdecl, name = "cb_RSNODENAME_RwFrame_Dtor", expect = "8B 44 24 04 C3")]
pub fn node_name_dtor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    obj
}

/// `cb_RSNODENAME_RwFrame_Copy` (0x72F9F0): `strncpy(dst name, src name, 23)`.
#[hook(addr = 0x72F9F0, conv = cdecl, name = "cb_RSNODENAME_RwFrame_Copy", expect = "A1 5C 7C C8 00")]
pub fn node_name_copy(dst: GPtr<u8>, src: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let off = NODE_NAME_OFFSET.get();
    strncpy(at_off(dst, off), at_off(src, off), 0x17);
    dst
}

/// `cb_RSNODENAME_RwFrame_StreamRead` (0x72FA50): reads `length` bytes into the name and terminates
/// it at `name[length]` (no bound check, as in the original).
#[hook(addr = 0x72FA50, conv = cdecl, name = "cb_RSNODENAME_RwFrame_StreamRead", expect = "8B 44 24 0C 8B 0D 5C 7C C8 00")]
pub fn node_name_stream_read(stream: GPtr<u8>, len: i32, obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let name = at_off(obj, NODE_NAME_OFFSET.get());
    rw_stream_read(stream, name, len as u32);
    wr::<u8>(name, len as isize, 0);
    stream
}

/// `cb_RSNODENAME_RwFrame_StreamWrite` (0x72FA20): writes `length` bytes of the name.
#[hook(addr = 0x72FA20, conv = cdecl, name = "cb_RSNODENAME_RwFrame_StreamWrite", expect = "8B 44 24 08 8B 4C 24 0C")]
pub fn node_name_stream_write(stream: GPtr<u8>, len: i32, obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    rw_stream_write(stream, at_off(obj, NODE_NAME_OFFSET.get()), len as u32);
    stream
}

/// `cb_RSNODENAME_RwFrame_StreamGetSize` (0x72FA80): `rwstrlen(name)`, 0 when `obj + offset` is NULL.
#[hook(addr = 0x72FA80, conv = cdecl, name = "cb_RSNODENAME_RwFrame_StreamGetSize", expect = "8B 44 24 04 8B 0D 5C 7C C8 00")]
pub fn node_name_stream_get_size(obj: GPtr<u8>, _off: i32, _size: i32) -> i32 {
    let name = at_off(obj, NODE_NAME_OFFSET.get());
    if name.is_null() {
        return 0;
    }
    rw_strlen(name)
}

// ------------------------------------------------------------------- Pipeline callbacks ---------

/// `cb_RSPIPELINE_RpAtomic_Ctor` (0x72FB50): pipeline id = 0 when the offset is positive.
#[hook(addr = 0x72FB50, conv = cdecl, name = "cb_RSPIPELINE_RpAtomic_Ctor", expect = "8B 0D 80 60 8D 00")]
pub fn pipeline_ctor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let off = PIPELINE_OFFSET.get();
    if off > 0 {
        wr::<u32>(at_off(obj, off), 0, 0);
    }
    obj
}

/// `cb_RSPIPELINE_RpAtomic_Copy` (0x72FB70): copies the pipeline id dword.
#[hook(addr = 0x72FB70, conv = cdecl, name = "cb_RSPIPELINE_RpAtomic_Copy", expect = "8B 0D 80 60 8D 00")]
pub fn pipeline_copy(dst: GPtr<u8>, src: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let off = PIPELINE_OFFSET.get();
    let v = rd::<u32>(at_off(src, off), 0);
    wr::<u32>(at_off(dst, off), 0, v);
    dst
}

/// `cb_RSPIPELINE_RpAtomic_StreamRead` (0x72FB90): reads `length` bytes into the pipeline id.
#[hook(addr = 0x72FB90, conv = cdecl, name = "cb_RSPIPELINE_RpAtomic_StreamRead", expect = "8B 44 24 08 8B 4C 24 0C")]
pub fn pipeline_stream_read(stream: GPtr<u8>, len: i32, obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    rw_stream_read(stream, at_off(obj, PIPELINE_OFFSET.get()), len as u32);
    stream
}

/// `cb_RSPIPELINE_RpAtomic_StreamGetSize` (0x72FBC0): 4.
#[hook(addr = 0x72FBC0, conv = cdecl, name = "cb_RSPIPELINE_RpAtomic_StreamGetSize", expect = "B8 04 00 00 00")]
pub fn pipeline_stream_get_size(_obj: GPtr<u8>, _off: i32, _size: i32) -> i32 {
    4
}

// -------------------------------------------------------------- CVisibilityPlugins callbacks ----

/// `CVisibilityPlugins::AtomicConstructor` (0x732150): model index word = -1, flags word = 0 (the
/// offset is reloaded between the two stores).
#[hook(addr = 0x732150, conv = cdecl, name = "CVisibilityPlugins::AtomicConstructor", expect = "8B 44 24 04 8B 0D 8C 60 8D 00")]
pub fn vis_atomic_ctor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    wr::<u16>(at_off(obj, VIS_ATOMIC_OFFSET.get()), 0, 0xFFFF);
    wr::<u16>(at_off(obj, VIS_ATOMIC_OFFSET.get()), 2, 0);
    obj
}

/// `CVisibilityPlugins::AtomicDestructor` (0x7321A0): returns the atomic.
#[hook(addr = 0x7321A0, conv = cdecl, name = "CVisibilityPlugins::AtomicDestructor", expect = "8B 44 24 04 C3")]
pub fn vis_atomic_dtor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    obj
}

/// `CVisibilityPlugins::AtomicCopyConstructor` (0x732170): copies both words (offset reloaded).
#[hook(addr = 0x732170, conv = cdecl, name = "CVisibilityPlugins::AtomicCopyConstructor", expect = "8B 0D 8C 60 8D 00")]
pub fn vis_atomic_copy(dst: GPtr<u8>, src: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let off = VIS_ATOMIC_OFFSET.get();
    let v = rd::<u16>(at_off(src, off), 0);
    wr::<u16>(at_off(dst, off), 0, v);
    let off = VIS_ATOMIC_OFFSET.get();
    let v = rd::<u16>(at_off(src, off), 2);
    wr::<u16>(at_off(dst, off), 2, v);
    dst
}

/// `CVisibilityPlugins::FrameConstructor` (0x7321B0): hierarchy id = 0.
#[hook(addr = 0x7321B0, conv = cdecl, name = "CVisibilityPlugins::FrameConstructor", expect = "8B 44 24 04 8B 0D 90 60 8D 00")]
pub fn vis_frame_ctor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    wr::<u32>(at_off(obj, VIS_FRAME_OFFSET.get()), 0, 0);
    obj
}

/// `CVisibilityPlugins::FrameDestructor` (0x7321F0): returns the frame.
#[hook(addr = 0x7321F0, conv = cdecl, name = "CVisibilityPlugins::FrameDestructor", expect = "8B 44 24 04 C3")]
pub fn vis_frame_dtor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    obj
}

/// `CVisibilityPlugins::FrameCopyConstructor` (0x7321D0): copies the dword.
#[hook(addr = 0x7321D0, conv = cdecl, name = "CVisibilityPlugins::FrameCopyConstructor", expect = "8B 0D 90 60 8D 00")]
pub fn vis_frame_copy(dst: GPtr<u8>, src: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let off = VIS_FRAME_OFFSET.get();
    let v = rd::<u32>(at_off(src, off), 0);
    wr::<u32>(at_off(dst, off), 0, v);
    dst
}

/// `CVisibilityPlugins::ClumpConstructor` (0x732E10): visibility callback = `DefaultVisibilityCB`
/// (0x732A30), alpha = 0xFF (offset reloaded between the stores).
#[hook(addr = 0x732E10, conv = cdecl, name = "CVisibilityPlugins::ClumpConstructor", expect = "8B 44 24 04 8B 0D 94 60 8D 00")]
pub fn vis_clump_ctor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    wr::<u32>(at_off(obj, VIS_CLUMP_OFFSET.get()), 0, DEFAULT_VISIBILITY_CB);
    wr::<u32>(at_off(obj, VIS_CLUMP_OFFSET.get()), 4, 0xFF);
    obj
}

/// `CVisibilityPlugins::ClumpDestructor` (0x732220): returns the clump.
#[hook(addr = 0x732220, conv = cdecl, name = "CVisibilityPlugins::ClumpDestructor", expect = "8B 44 24 04 C3")]
pub fn vis_clump_dtor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    obj
}

/// `CVisibilityPlugins::ClumpCopyConstructor` (0x732200): copies the callback dword only.
#[hook(addr = 0x732200, conv = cdecl, name = "CVisibilityPlugins::ClumpCopyConstructor", expect = "8B 0D 94 60 8D 00")]
pub fn vis_clump_copy(dst: GPtr<u8>, src: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let off = VIS_CLUMP_OFFSET.get();
    let v = rd::<u32>(at_off(src, off), 0);
    wr::<u32>(at_off(dst, off), 0, v);
    dst
}

/// One inlined `CLinkList<T>::Init(n)` of `CVisibilityPlugins::Initialise` (links of `link` bytes:
/// data, then `prev` at `link - 8`, `next` at `link - 4`). `list` is the list's address: used head
/// +0, used tail +L, free head +2L, free tail +3L, `links` pointer +4L. The links are pushed after the
/// free head from the last to the first, in the original's store order.
fn link_list_init(list: GPtr<u8>, n: u32, link: isize) {
    let prev = link - 8;
    let next = link - 4;
    let bytes = n * link as u32;
    let used_head = list;
    let used_tail = list.byte_add(link);
    let free_head = list.byte_add(2 * link);
    let free_tail = list.byte_add(3 * link);
    let links = operator_new(bytes);
    wr::<GPtr<u8>>(list, 4 * link, links);
    wr::<GPtr<u8>>(used_head, next, used_tail);
    wr::<GPtr<u8>>(used_tail, prev, used_head);
    wr::<GPtr<u8>>(free_head, next, free_tail);
    wr::<GPtr<u8>>(free_tail, prev, free_head);
    let mut cur = free_tail;
    let mut i = bytes;
    loop {
        i -= link as u32;
        let l = rd::<GPtr<u8>>(list, 4 * link).byte_add(i as isize);
        wr::<GPtr<u8>>(l, next, cur);
        let first = rd::<GPtr<u8>>(free_head, next);
        wr::<GPtr<u8>>(first, prev, l);
        wr::<GPtr<u8>>(l, prev, free_head);
        cur = l;
        wr::<GPtr<u8>>(free_head, next, l);
        if i == 0 {
            break;
        }
    }
}

/// `CVisibilityPlugins::Initialise` (0x733A20): the five alpha lists (`m_alphaList` 20,
/// `m_alphaBoxList` 20, `m_alphaEntityList` 200, `m_alphaUnderwaterEntityList` 100,
/// `m_alphaReallyDrawLastList` 50 links of 0x14 bytes) with used head distance 0 and used tail
/// distance 1e8 (0x4CBEBC20) stored after each list's links, then `ms_weaponPedsForPC` (100 links of
/// 0xC bytes, no sort keys).
#[hook(addr = 0x733A20, conv = cdecl, name = "CVisibilityPlugins::Initialise", expect = "56 68 90 01 00 00")]
pub fn visibility_plugins_initialise() {
    let base = VIS_LISTS.ptr().cast::<u8>();
    for (off, n) in [(0usize, 20u32), (0x58, 20), (0xB0, 200), (0x108, 100), (0x160, 50)] {
        let list = base.byte_add(off as isize);
        link_list_init(list, n, 0x14);
        wr::<u32>(list, 8, 0);
        wr::<u32>(list, 0x14 + 8, 0x4CBE_BC20);
    }
    link_list_init(base.byte_add(0x1B4), 100, 0xC);
}

// ------------------------------------------------------------------- RpAnimBlend callbacks ------

/// `RpAnimBlend::ClumpAnimConstruct` (0x4D5F40): clump data pointer = NULL at the **argument**
/// offset.
#[hook(addr = 0x4D5F40, conv = cdecl, name = "RpAnimBlend::ClumpAnimConstruct", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn clump_anim_construct(obj: GPtr<u8>, off: i32, _size: i32) -> GPtr<u8> {
    wr::<u32>(at_off(obj, off), 0, 0);
    obj
}

/// `RpAnimBlend::ClumpAnimDestruct` (0x4D6110): when the clump has blend data, remove all
/// associations, destroy and delete the data, clear the pointer (offset reloaded).
#[hook(addr = 0x4D6110, conv = cdecl, name = "RpAnimBlend::ClumpAnimDestruct", expect = "A1 78 F8 B5 00")]
pub fn clump_anim_destruct(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let data = rd::<GPtr<u8>>(at_off(obj, ANIM_BLEND_CLUMP_OFFSET.get()), 0);
    if !data.is_null() {
        anim_blend_clump_remove_all_associations(obj);
        anim_blend_clump_data_dtor(data);
        operator_delete(data);
        wr::<u32>(at_off(obj, ANIM_BLEND_CLUMP_OFFSET.get()), 0, 0);
    }
    obj
}

/// `RpAnimBlend::ClumpAnimCopy` (0x4D5F90): `xor eax, eax; ret` (returns NULL). Not hookable (3 bytes).
#[sa_core::native(addr = 0x4D5F90, conv = cdecl, name = "RpAnimBlend::ClumpAnimCopy")]
pub fn clump_anim_copy(_dst: GPtr<u8>, _src: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    GPtr::NULL
}

/// `RpAnimBlend::RpAnimBlendKeyFrameInterpolate` (0x4D60C0): the output key frame becomes the
/// identity (quaternion w = 1.0 at +0xC first, then x, y, z and the translation zeroed); returns it.
#[hook(addr = 0x4D60C0, conv = cdecl, name = "RpAnimBlend::RpAnimBlendKeyFrameInterpolate", expect = "8B 44 24 04 33 C9")]
pub fn key_frame_interpolate(out: GPtr<u8>, _in1: GPtr<u8>, _in2: GPtr<u8>, _time: u32, _custom: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(out, 0xC, 0x3F80_0000);
    for o in [0, 4, 8, 0x10, 0x14, 0x18] {
        wr::<u32>(out, o, 0);
    }
    out
}

// --------------------------------------------------- CCustomBuildingDNPipeline callbacks -------

/// `CCustomBuildingDNPipeline::pluginExtraVertColourConstructorCB` (0x5D6D10): zeroes the three
/// dwords when the offset is positive.
#[hook(addr = 0x5D6D10, conv = cdecl, name = "CCustomBuildingDNPipeline::pluginExtraVertColourConstructorCB", expect = "8B 0D BC 12 8D 00")]
pub fn extra_vert_colour_ctor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let off = EXTRA_VERT_COLOUR_OFFSET.get();
    if off > 0 {
        let p = at_off(obj, off);
        wr::<u32>(p, 0, 0);
        wr::<u32>(p, 4, 0);
        wr::<u32>(p, 8, 0);
    }
    obj
}

/// `CCustomBuildingDNPipeline::pluginExtraVertColourDestructorCB` (0x5D6D30): frees (`CMemoryMgr::Free`)
/// and clears the night and day colour arrays (offset reloaded around every access; no offset check).
#[hook(addr = 0x5D6D30, conv = cdecl, name = "CCustomBuildingDNPipeline::pluginExtraVertColourDestructorCB", expect = "A1 BC 12 8D 00")]
pub fn extra_vert_colour_dtor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    for o in [0isize, 4] {
        let p = rd::<GPtr<u8>>(at_off(obj, EXTRA_VERT_COLOUR_OFFSET.get()), o);
        if !p.is_null() {
            memory_free(p);
            wr::<u32>(at_off(obj, EXTRA_VERT_COLOUR_OFFSET.get()), o, 0);
        }
    }
    obj
}

/// `CCustomBuildingDNPipeline::pluginExtraVertColourStreamReadCB` (0x5D6DE0). Reads a 4-byte flag
/// (the original reads it into its `object` argument slot, so a short read leaves the geometry
/// pointer there, i.e. non-zero; the port starts the flag with that value). When set: two arrays of
/// `numVertices * 4` bytes (`CMemoryMgr::Malloc(n, 0)`), dn parameter 1.0, the night colours are read
/// into the first; then, when the geometry has pre-lit colours (+0x30) and `numVertices > 0`, they are
/// copied into the second array byte by byte (r, b, g, a order).
#[hook(addr = 0x5D6DE0, conv = cdecl, name = "CCustomBuildingDNPipeline::pluginExtraVertColourStreamReadCB", expect = "A1 BC 12 8D 00")]
pub fn extra_vert_colour_stream_read(stream: GPtr<u8>, _len: i32, geom: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let data = at_off(geom, EXTRA_VERT_COLOUR_OFFSET.get());
    let mut flag: u32 = geom.addr() as u32;
    rw_stream_read(stream, GPtr::from_raw(&mut flag as *mut u32).cast(), 4);
    if flag == 0 {
        return stream;
    }
    let n = rd::<u32>(geom, 0x14) << 2;
    let night = memory_malloc(n, 0);
    wr::<GPtr<u8>>(data, 0, night);
    let n = rd::<u32>(geom, 0x14) << 2;
    let day = memory_malloc(n, 0);
    let night = rd::<GPtr<u8>>(data, 0);
    wr::<GPtr<u8>>(data, 4, day);
    wr::<u32>(data, 8, 0x3F80_0000);
    let n = rd::<u32>(geom, 0x14) << 2;
    rw_stream_read(stream, night, n);
    let src = rd::<GPtr<u8>>(geom, 0x30);
    let count = rd::<i32>(geom, 0x14);
    if src.is_null() {
        return stream;
    }
    let dst = rd::<GPtr<u8>>(at_off(geom, EXTRA_VERT_COLOUR_OFFSET.get()), 4);
    if count <= 0 {
        return stream;
    }
    for i in 0..count as isize {
        let s = src.byte_add(i * 4);
        let d = dst.byte_add(i * 4);
        wr::<u8>(d, 0, rd::<u8>(s, 0));
        wr::<u8>(d, 2, rd::<u8>(s, 2));
        wr::<u8>(d, 1, rd::<u8>(s, 1));
        wr::<u8>(d, 3, rd::<u8>(s, 3));
    }
    stream
}

/// `CCustomBuildingDNPipeline::pluginExtraVertColourStreamWriteCB` (0x5D6D80). Original bug kept:
/// the flag written is the first 4 bytes **at** the night-colour array (not a flag), and when that
/// dword is non-zero `numVertices * 4` bytes are written from the address it holds.
#[hook(addr = 0x5D6D80, conv = cdecl, name = "CCustomBuildingDNPipeline::pluginExtraVertColourStreamWriteCB", expect = "A1 BC 12 8D 00")]
pub fn extra_vert_colour_stream_write(stream: GPtr<u8>, _len: i32, geom: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let night = rd::<GPtr<u8>>(at_off(geom, EXTRA_VERT_COLOUR_OFFSET.get()), 0);
    rw_stream_write(stream, night, 4);
    let p = rd::<GPtr<u8>>(night, 0);
    if !p.is_null() {
        rw_stream_write(stream, p, rd::<u32>(geom, 0x14) << 2);
    }
    stream
}

/// `CCustomBuildingDNPipeline::pluginExtraVertColourStreamGetSizeCB` (0x5D6DC0): 0 for a NULL
/// geometry, else `numVertices * 8 + 4`.
#[hook(addr = 0x5D6DC0, conv = cdecl, name = "CCustomBuildingDNPipeline::pluginExtraVertColourStreamGetSizeCB", expect = "8B 44 24 04 85 C0")]
pub fn extra_vert_colour_stream_get_size(geom: GPtr<u8>, _off: i32, _size: i32) -> i32 {
    if geom.is_null() {
        return 0;
    }
    rd::<i32>(geom, 0x14).wrapping_mul(8).wrapping_add(4)
}

// ----------------------------------------------------- CCustomCarEnvMapPipeline callbacks -------

/// The inlined `CPool::Delete` of the env-map destructors: `index = (p - objects) / size` (signed;
/// `imul`-by-reciprocal truncation for 0xC, `sar` for 8), `byteMap[index] |= 0x80`, `firstFree =
/// min(firstFree, index)`. CPool: +0 objects, +4 byte map, +0xC first free.
fn pool_delete(pool: GPtr<u8>, p: GPtr<u8>, shift8: bool) {
    let diff = (p.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32;
    let idx = if shift8 { diff >> 3 } else { diff / 12 };
    let flag = rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize);
    wr::<u8>(flag, 0, rd::<u8>(flag, 0) | 0x80);
    if idx < rd::<i32>(pool, 0xC) {
        wr::<i32>(pool, 0xC, idx);
    }
}

/// `rep movsd` + `rep movsb` of `size` bytes.
fn copy_bytes(dst: GPtr<u8>, src: GPtr<u8>, size: u32) {
    let words = size >> 2;
    for i in 0..words as isize {
        wr::<u32>(dst, i * 4, rd::<u32>(src, i * 4));
    }
    let base = (words * 4) as isize;
    for i in 0..(size & 3) as isize {
        wr::<u8>(dst, base + i, rd::<u8>(src, base + i));
    }
}

/// `CCustomCarEnvMapPipeline::pluginEnvMatConstructorCB` (0x5D8BD0): data = `&fakeEnvMapPipeMatData`
/// at the argument offset.
// Not hooked: protection code reads its entry bytes as data (0xF403D3 tests 0x5D8BD3).
#[sa_core::native(addr = 0x5D8BD0, conv = cdecl, name = "CCustomCarEnvMapPipeline::pluginEnvMatConstructorCB")]
pub fn env_mat_ctor(obj: GPtr<u8>, off: i32, _size: i32) -> GPtr<u8> {
    wr::<u32>(at_off(obj, off), 0, FAKE_ENV_MAP_MAT_DATA.addr());
    obj
}

/// `CCustomCarEnvMapPipeline::pluginEnvMatDestructorCB` (0x5D95B0): returns non-default data to its
/// pool and clears the pointer (argument offset).
#[hook(addr = 0x5D95B0, conv = cdecl, name = "CCustomCarEnvMapPipeline::pluginEnvMatDestructorCB", expect = "56 8B 74 24 08")]
pub fn env_mat_dtor(obj: GPtr<u8>, off: i32, _size: i32) -> GPtr<u8> {
    let p = rd::<GPtr<u8>>(at_off(obj, off), 0);
    if p.addr() as u32 != FAKE_ENV_MAP_MAT_DATA.addr() && !p.is_null() {
        pool_delete(ENV_MAT_POOL.get(), p, false);
        wr::<u32>(at_off(obj, off), 0, 0);
    }
    obj
}

/// `CCustomCarEnvMapPipeline::pluginEnvMatCopyConstructorCB` (0x5D9600): with source data, the
/// destination gets a pool entry when it has none (pool exhausted: the default data, return) and then
/// **`size` bytes** (the plugin size, 4) of the source data are copied into it (original behaviour;
/// a destination still pointing at the default data is overwritten in place).
#[hook(addr = 0x5D9600, conv = cdecl, name = "CCustomCarEnvMapPipeline::pluginEnvMatCopyConstructorCB", expect = "8B 44 24 08 53")]
pub fn env_mat_copy(dst: GPtr<u8>, src: GPtr<u8>, off: i32, size: u32) -> GPtr<u8> {
    let s = rd::<GPtr<u8>>(at_off(src, off), 0);
    let mut d = rd::<GPtr<u8>>(at_off(dst, off), 0);
    if s.is_null() {
        return dst;
    }
    if d.is_null() {
        d = env_mat_pool_new(ENV_MAT_POOL.get());
        wr::<GPtr<u8>>(at_off(dst, off), 0, d);
        if d.is_null() {
            wr::<u32>(at_off(dst, off), 0, FAKE_ENV_MAP_MAT_DATA.addr());
            return dst;
        }
    }
    copy_bytes(d, s, size);
    dst
}

/// `CCustomCarEnvMapPipeline::pluginEnvMatStreamGetSizeCB` (0x5D8D10): `obj ? 0x18 : 0`.
#[hook(addr = 0x5D8D10, conv = cdecl, name = "CCustomCarEnvMapPipeline::pluginEnvMatStreamGetSizeCB", expect = "8B 44 24 04 F7 D8")]
pub fn env_mat_stream_get_size(obj: GPtr<u8>, _off: i32, _size: i32) -> i32 {
    if obj.is_null() {
        0
    } else {
        0x18
    }
}

/// `CCustomCarEnvMapPipeline::pluginEnvAtmConstructorCB` (0x5D8D30): data = NULL (argument offset).
#[hook(addr = 0x5D8D30, conv = cdecl, name = "CCustomCarEnvMapPipeline::pluginEnvAtmConstructorCB", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn env_atm_ctor(obj: GPtr<u8>, off: i32, _size: i32) -> GPtr<u8> {
    wr::<u32>(at_off(obj, off), 0, 0);
    obj
}

/// `CCustomCarEnvMapPipeline::pluginEnvAtmDestructorCB` (0x5D9730): returns the data to its pool and
/// clears the pointer (argument offset).
#[hook(addr = 0x5D9730, conv = cdecl, name = "CCustomCarEnvMapPipeline::pluginEnvAtmDestructorCB", expect = "56 8B 74 24 08")]
pub fn env_atm_dtor(obj: GPtr<u8>, off: i32, _size: i32) -> GPtr<u8> {
    let p = rd::<GPtr<u8>>(at_off(obj, off), 0);
    if !p.is_null() {
        pool_delete(ENV_ATM_POOL.get(), p, false);
        wr::<u32>(at_off(obj, off), 0, 0);
    }
    obj
}

/// `CCustomCarEnvMapPipeline::pluginEnvAtmCopyConstructorCB` (0x5D9780): like the material copy
/// (`size` bytes), without the default-data fallback.
#[hook(addr = 0x5D9780, conv = cdecl, name = "CCustomCarEnvMapPipeline::pluginEnvAtmCopyConstructorCB", expect = "8B 44 24 08 53")]
pub fn env_atm_copy(dst: GPtr<u8>, src: GPtr<u8>, off: i32, size: u32) -> GPtr<u8> {
    let s = rd::<GPtr<u8>>(at_off(src, off), 0);
    let mut d = rd::<GPtr<u8>>(at_off(dst, off), 0);
    if s.is_null() {
        return dst;
    }
    if d.is_null() {
        d = env_atm_pool_new(ENV_ATM_POOL.get());
        wr::<GPtr<u8>>(at_off(dst, off), 0, d);
        if d.is_null() {
            return dst;
        }
    }
    copy_bytes(d, s, size);
    dst
}

/// `CCustomCarEnvMapPipeline::pluginSpecMatConstructorCB` (0x5D8D40): data = NULL (global offset).
#[hook(addr = 0x5D8D40, conv = cdecl, name = "CCustomCarEnvMapPipeline::pluginSpecMatConstructorCB", expect = "8B 44 24 04 8B 0D CC 12 8D 00")]
pub fn spec_mat_ctor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    wr::<u32>(at_off(obj, SPEC_MAP_OFFSET.get()), 0, 0);
    obj
}

/// `CCustomCarEnvMapPipeline::pluginSpecMatDestructorCB` (0x5D97D0): destroys the texture (+4,
/// cleared), returns the 8-byte entry to its pool (`sar 3`), clears the pointer.
#[hook(addr = 0x5D97D0, conv = cdecl, name = "CCustomCarEnvMapPipeline::pluginSpecMatDestructorCB", expect = "A1 CC 12 8D 00")]
pub fn spec_mat_dtor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let slot = at_off(obj, SPEC_MAP_OFFSET.get());
    let p = rd::<GPtr<u8>>(slot, 0);
    if !p.is_null() {
        let tex = rd::<GPtr<u8>>(p, 4);
        if !tex.is_null() {
            rw_texture_destroy(tex);
            wr::<u32>(p, 4, 0);
        }
        pool_delete(SPEC_MAT_POOL.get(), p, true);
        wr::<u32>(slot, 0, 0);
    }
    obj
}

/// `CCustomCarEnvMapPipeline::pluginSpecMatCopyConstructorCB` (0x5D9830): with source data, the
/// destination gets a pool entry when it has none (none left: return); copies the specularity dword
/// and the texture, whose ref count (+0x54) is incremented.
#[hook(addr = 0x5D9830, conv = cdecl, name = "CCustomCarEnvMapPipeline::pluginSpecMatCopyConstructorCB", expect = "A1 CC 12 8D 00")]
pub fn spec_mat_copy(dst: GPtr<u8>, src: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let off = SPEC_MAP_OFFSET.get();
    let s = rd::<GPtr<u8>>(at_off(src, off), 0);
    let slot = at_off(dst, off);
    let mut d = rd::<GPtr<u8>>(slot, 0);
    if s.is_null() {
        return dst;
    }
    if d.is_null() {
        d = spec_mat_pool_new(SPEC_MAT_POOL.get());
        wr::<GPtr<u8>>(slot, 0, d);
        if d.is_null() {
            return dst;
        }
    }
    wr::<u32>(d, 0, rd::<u32>(s, 0));
    let tex = rd::<GPtr<u8>>(s, 4);
    wr::<GPtr<u8>>(d, 4, tex);
    if !tex.is_null() {
        wr::<i32>(tex, 0x54, rd::<i32>(tex, 0x54).wrapping_add(1));
    }
    dst
}

/// `CCustomCarEnvMapPipeline::pluginSpecMatStreamGetSizeCB` (0x5D8DD0): -1 when the plugin offset is
/// -1, else 0x1C.
#[hook(addr = 0x5D8DD0, conv = cdecl, name = "CCustomCarEnvMapPipeline::pluginSpecMatStreamGetSizeCB", expect = "8B 0D CC 12 8D 00")]
pub fn spec_mat_stream_get_size(_obj: GPtr<u8>, _off: i32, _size: i32) -> i32 {
    if SPEC_MAP_OFFSET.get() == -1 {
        -1
    } else {
        0x1C
    }
}

// --------------------------------------------------------------- BreakablePlugin callbacks ------

/// `BreakablePlugin::BreakableConstructor` (0x59CD70): data = NULL when the offset is positive.
#[hook(addr = 0x59CD70, conv = cdecl, name = "BreakablePlugin::BreakableConstructor", expect = "8B 0D 38 42 BB 00")]
pub fn breakable_ctor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let off = BREAKABLE_OFFSET.get();
    if off > 0 {
        wr::<u32>(at_off(obj, off), 0, 0);
    }
    obj
}

/// `BreakablePlugin::BreakableDestructor` (0x59CD90): the data pointer is read before the offset is
/// tested (as in the original); with a positive offset and data, destroys every non-NULL texture of the
/// `+0x20` (word) count at `+0x24` and `operator delete`s the data (the pointer is not cleared).
#[hook(addr = 0x59CD90, conv = cdecl, name = "BreakablePlugin::BreakableDestructor", expect = "A1 38 42 BB 00")]
pub fn breakable_dtor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let off = BREAKABLE_OFFSET.get();
    let data = rd::<GPtr<u8>>(at_off(obj, off), 0);
    if off > 0 && !data.is_null() {
        let mut i: i32 = 0;
        if rd::<u16>(data, 0x20) > 0 {
            loop {
                let tex = rd::<GPtr<u8>>(rd::<GPtr<u8>>(data, 0x24), i as isize * 4);
                if !tex.is_null() {
                    rw_texture_destroy(tex);
                }
                i += 1;
                if i >= i32::from(rd::<u16>(data, 0x20)) {
                    break;
                }
            }
        }
        operator_delete(data);
    }
    obj
}

/// `BreakablePlugin::BreakableStreamGetSize` (0x59D0F0): 0x34.
#[hook(addr = 0x59D0F0, conv = cdecl, name = "BreakablePlugin::BreakableStreamGetSize", expect = "B8 34 00 00 00")]
pub fn breakable_stream_get_size(_obj: GPtr<u8>, _off: i32, _size: i32) -> i32 {
    0x34
}

// --------------------------------------------------------------- CCollisionPlugin callbacks -----

/// `cb_RSCOLLISION_RpClump_Ctor` (0x41B1A0): returns the clump.
#[hook(addr = 0x41B1A0, conv = cdecl, name = "cb_RSCOLLISION_RpClump_Ctor", expect = "8B 44 24 04 C3")]
pub fn collision_ctor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    obj
}

/// `cb_RSCOLLISION_RpClump_Copy` (0x41B1B0): returns the destination.
#[hook(addr = 0x41B1B0, conv = cdecl, name = "cb_RSCOLLISION_RpClump_Copy", expect = "8B 44 24 04 C3")]
pub fn collision_copy(dst: GPtr<u8>, _src: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    dst
}

/// `cb_RSCOLLISION_RpClump_Dtor` (0x41B1C0): returns the clump.
#[hook(addr = 0x41B1C0, conv = cdecl, name = "cb_RSCOLLISION_RpClump_Dtor", expect = "8B 44 24 04 C3")]
pub fn collision_dtor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    obj
}

/// `cb_RSCOLLISION_RpClump_StreamWrite` (0x41B2F0): writes nothing, returns the stream.
#[hook(addr = 0x41B2F0, conv = cdecl, name = "cb_RSCOLLISION_RpClump_StreamWrite", expect = "8B 44 24 04 C3")]
pub fn collision_stream_write(stream: GPtr<u8>, _len: i32, _obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    stream
}

/// `cb_RSCOLLISION_RpClump_StreamGetSize` (0x41B300): -1 (`or eax, -1; ret`, 4 bytes: not hookable).
#[sa_core::native(addr = 0x41B300, conv = cdecl, name = "cb_RSCOLLISION_RpClump_StreamGetSize")]
pub fn collision_stream_get_size(_obj: GPtr<u8>, _off: i32, _size: i32) -> i32 {
    -1
}

// -------------------------------------------------------------------- C2dEffect callbacks -------

/// `C2dEffect::t2dEffectPluginConstructor` (0x6F9F90): data = NULL.
#[hook(addr = 0x6F9F90, conv = cdecl, name = "C2dEffect::t2dEffectPluginConstructor", expect = "8B 44 24 04 8B 0D E0 A1 C3 00")]
pub fn effect_2d_ctor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    wr::<u32>(at_off(obj, G2D_EFFECT_PLUGIN_OFFSET.get() as i32), 0, 0);
    obj
}

/// `C2dEffect::t2dEffectPluginCopyConstructor` (0x6F9FB0): the destination's data = NULL (nothing
/// is copied; the returned value is the destination).
#[hook(addr = 0x6F9FB0, conv = cdecl, name = "C2dEffect::t2dEffectPluginCopyConstructor", expect = "8B 44 24 04 8B 0D E0 A1 C3 00")]
pub fn effect_2d_copy(dst: GPtr<u8>, _src: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    wr::<u32>(at_off(dst, G2D_EFFECT_PLUGIN_OFFSET.get() as i32), 0, 0);
    dst
}

/// `C2dEffect::t2dEffectPluginDestructor` (0x6FA880). Data: count dword, then 0x40-byte effects from
/// +4 (type byte at +0xC). Light (0): corona texture (+0x30) and shadow texture (+0x34) destroyed and
/// cleared. Road sign (7): text (+0x28) `CMemoryMgr::Free`d and cleared; the atomic (+0x2C), when set:
/// its frame (+4) detached and destroyed when present, the atomic destroyed, the pointer cleared. The
/// data pointer and the count are reloaded every iteration; at the end the data is freed (the pointer
/// is not cleared).
#[hook(addr = 0x6FA880, conv = cdecl, name = "C2dEffect::t2dEffectPluginDestructor", expect = "51 53 55 56 33 ED")]
pub fn effect_2d_dtor(obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    let mut i: u32 = 0;
    loop {
        let off = G2D_EFFECT_PLUGIN_OFFSET.get() as i32;
        let data = rd::<GPtr<u8>>(at_off(obj, off), 0);
        if data.is_null() || i >= rd::<u32>(data, 0) {
            let data = rd::<GPtr<u8>>(at_off(obj, off), 0);
            if !data.is_null() {
                memory_free(data);
            }
            return obj;
        }
        let e = data.byte_add(4 + i as isize * 0x40);
        match rd::<u8>(e, 0xC) {
            0 => {
                let t = rd::<GPtr<u8>>(e, 0x30);
                if !t.is_null() {
                    rw_texture_destroy(t);
                    wr::<u32>(e, 0x30, 0);
                }
                let t = rd::<GPtr<u8>>(e, 0x34);
                if !t.is_null() {
                    rw_texture_destroy(t);
                    wr::<u32>(e, 0x34, 0);
                }
            }
            7 => {
                let text = rd::<GPtr<u8>>(e, 0x28);
                if !text.is_null() {
                    memory_free(text);
                    wr::<u32>(e, 0x28, 0);
                }
                let atomic = rd::<GPtr<u8>>(e, 0x2C);
                if !atomic.is_null() {
                    let frame = rd::<GPtr<u8>>(atomic, 4);
                    if !frame.is_null() {
                        rp_atomic_set_frame(atomic, GPtr::NULL);
                        rw_frame_destroy(frame);
                    }
                    rp_atomic_destroy(atomic);
                    wr::<u32>(e, 0x2C, 0);
                }
            }
            _ => {}
        }
        i += 1;
    }
}

/// `C2dEffect::Rwt2dEffectPluginDataChunkWriteCallBack` (0x6FA620): writes nothing, returns the stream.
#[hook(addr = 0x6FA620, conv = cdecl, name = "C2dEffect::Rwt2dEffectPluginDataChunkWriteCallBack", expect = "8B 44 24 04 C3")]
pub fn effect_2d_stream_write(stream: GPtr<u8>, _len: i32, _obj: GPtr<u8>, _off: i32, _size: i32) -> GPtr<u8> {
    stream
}

/// `C2dEffect::Rwt2dEffectPluginDataChunkGetSizeCallBack` (0x6FA630): -1 (4 bytes: not hookable).
#[sa_core::native(addr = 0x6FA630, conv = cdecl, name = "C2dEffect::Rwt2dEffectPluginDataChunkGetSizeCallBack")]
pub fn effect_2d_stream_get_size(_obj: GPtr<u8>, _off: i32, _size: i32) -> i32 {
    -1
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[HookReg] = &[
    HOOK_NODE_NAME_PLUGIN_ATTACH,
    HOOK_VISIBILITY_PLUGINS_ATTACH,
    HOOK_RP_ANIM_BLEND_PLUGIN_ATTACH,
    HOOK_EXTRA_VERT_COLOUR_PLUGIN_ATTACH,
    HOOK_CUSTOM_BUILDING_RENDERER_PLUGIN_ATTACH,
    HOOK_ENV_MAP_REGISTER_PLUGIN,
    HOOK_CAR_FX_RENDERER_REGISTER_PLUGINS,
    HOOK_BREAKABLE_PLUGIN_ATTACH,
    HOOK_COLLISION_PLUGIN_ATTACH,
    HOOK_EFFECT_2D_PLUGIN_ATTACH,
    HOOK_PIPELINE_PLUGIN_ATTACH,
    HOOK_NODE_NAME_CTOR,
    HOOK_NODE_NAME_DTOR,
    HOOK_NODE_NAME_COPY,
    HOOK_NODE_NAME_STREAM_READ,
    HOOK_NODE_NAME_STREAM_WRITE,
    HOOK_NODE_NAME_STREAM_GET_SIZE,
    HOOK_PIPELINE_CTOR,
    HOOK_PIPELINE_COPY,
    HOOK_PIPELINE_STREAM_READ,
    HOOK_PIPELINE_STREAM_GET_SIZE,
    HOOK_VIS_ATOMIC_CTOR,
    HOOK_VIS_ATOMIC_DTOR,
    HOOK_VIS_ATOMIC_COPY,
    HOOK_VIS_FRAME_CTOR,
    HOOK_VIS_FRAME_DTOR,
    HOOK_VIS_FRAME_COPY,
    HOOK_VIS_CLUMP_CTOR,
    HOOK_VIS_CLUMP_DTOR,
    HOOK_VIS_CLUMP_COPY,
    HOOK_VISIBILITY_PLUGINS_INITIALISE,
    HOOK_CLUMP_ANIM_CONSTRUCT,
    HOOK_CLUMP_ANIM_DESTRUCT,
    HOOK_KEY_FRAME_INTERPOLATE,
    HOOK_EXTRA_VERT_COLOUR_CTOR,
    HOOK_EXTRA_VERT_COLOUR_DTOR,
    HOOK_EXTRA_VERT_COLOUR_STREAM_READ,
    HOOK_EXTRA_VERT_COLOUR_STREAM_WRITE,
    HOOK_EXTRA_VERT_COLOUR_STREAM_GET_SIZE,
    HOOK_ENV_MAT_DTOR,
    HOOK_ENV_MAT_COPY,
    HOOK_ENV_MAT_STREAM_GET_SIZE,
    HOOK_ENV_ATM_CTOR,
    HOOK_ENV_ATM_DTOR,
    HOOK_ENV_ATM_COPY,
    HOOK_SPEC_MAT_CTOR,
    HOOK_SPEC_MAT_DTOR,
    HOOK_SPEC_MAT_COPY,
    HOOK_SPEC_MAT_STREAM_GET_SIZE,
    HOOK_BREAKABLE_CTOR,
    HOOK_BREAKABLE_DTOR,
    HOOK_BREAKABLE_STREAM_GET_SIZE,
    HOOK_COLLISION_CTOR,
    HOOK_COLLISION_COPY,
    HOOK_COLLISION_DTOR,
    HOOK_COLLISION_STREAM_WRITE,
    HOOK_EFFECT_2D_CTOR,
    HOOK_EFFECT_2D_COPY,
    HOOK_EFFECT_2D_DTOR,
    HOOK_EFFECT_2D_STREAM_WRITE,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[
    NATIVE_ENV_MAT_CTOR,
    NATIVE_CLUMP_ANIM_COPY,
    NATIVE_COLLISION_STREAM_GET_SIZE,
    NATIVE_EFFECT_2D_STREAM_GET_SIZE,
];
