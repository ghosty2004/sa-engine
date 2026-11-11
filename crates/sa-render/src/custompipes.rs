//! STD-4 helper (native boot, gGameState FRONTEND_LOADING): the creation of GTA's custom RW
//! pipelines that `CGame::InitialiseEssentialsAfterRW` reaches: `CCarFXRenderer::Initialise` ->
//! `CCustomCarEnvMapPipeline::CreatePipe` (its pipeline, the env-map support check and the three
//! material / atomic data pools), and `CCustomBuildingRenderer::Initialise` ->
//! `CCustomBuildingPipeline::CreatePipe` / `CCustomBuildingDNPipeline::CreatePipe`. Ported from the
//! 1.0 US bytes (gta-reversed `Pipelines/` as reading aid; the bytes win).
//!
//! Each `CreateCustomObjPipe` builds a one-node pipeline from the D3D9 atomic all-in-one node
//! definition, finds the node by name and installs the class's reinstance and render callbacks
//! (the instance callback is set to the node's own), and tags the pipeline with its plugin id at
//! +0x2C and +0x30. The render / reinstance callbacks themselves are not ported here (their
//! addresses are stored only).

use sa_core::abi::{HookReg, NativeReg};
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::pool::CPool;
use sa_core::pools::{pool_construct, Slot};
use sa_core::{hook, orig};

use crate::common::{g, rd};

sa_core::globals! {
    list GLOBALS_LIST;
    /// `CCustomCarEnvMapPipeline::ObjPipeline`
    pub static CAR_ENV_MAP_PIPE @ 0xC02D24: u32 = 0;
    /// `CCustomCarEnvMapPipeline::g_GameLight` (`D3DLIGHT9`, 0x68 bytes, cleared by `CreatePipe`)
    pub static CAR_ENV_MAP_STATE @ 0xC02CB0: [u32; 0x1A] = [0; 0x1A];
    /// `CCustomBuildingPipeline::ObjPipeline`
    pub static BUILDING_PIPE @ 0xC02C68: u32 = 0;
    /// `CCustomBuildingDNPipeline::ObjPipeline`
    pub static BUILDING_DN_PIPE @ 0xC02C1C: u32 = 0;
    /// Unnamed flag read by `CCustomBuildingDNPipeline::CreateCustomObjPipe` (0 in the image, never
    /// written by the game; with 0 the DN pipe keeps the node's own reinstance callback)
    pub static BUILDING_DN_FLAG @ 0xC02C20: u32 = 0;
}

/// `CCustomCarEnvMapPipeline::m_gEnvMapPipeMatDataPool` / `AtmDataPool` / `SpecMapPipeMatDataPool`
/// (declared by [`crate::rwplugins`]).
use crate::rwplugins::{ENV_ATM_POOL, ENV_MAT_POOL, SPEC_MAT_POOL};

/// `CustomEnvMapPipeMaterialData` (0xC).
pub type EnvMapMatSlot = Slot<0xC>;
/// `CustomEnvMapPipeAtomicData` (0xC).
pub type EnvMapAtmSlot = Slot<0xC>;
/// `CustomSpecMapPipeMaterialData` (8).
pub type SpecMapMatSlot = Slot<8>;

/// Pipeline plugin ids stored at +0x2C / +0x30.
const ID_CAR_ENV_MAP: u32 = 0x53F2_009A;
const ID_BUILDING: u32 = 0x53F2_009C;
const ID_BUILDING_DN: u32 = 0x53F2_0098;

/// Callbacks installed in the all-in-one node.
const CAR_INSTANCE_CB: u32 = 0x5D8490;
const CAR_RENDER_CB: u32 = 0x5D9900;
const BUILDING_REINSTANCE_CB: u32 = 0x5D77A0;
const BUILDING_RENDER_CB: u32 = 0x5D77D0;
const DN_REINSTANCE_CB: u32 = 0x5D63E0;
const DN_RENDER_CB: u32 = 0x5D6480;

orig! {
    /// `RxPipelineCreate()` (0x8057B0)
    fn rx_pipeline_create() -> u32 = cdecl 0x8057B0 => unported;
    /// `RxPipelineLock(pipe)` (0x806990)
    fn rx_pipeline_lock(p: u32) -> u32 = cdecl 0x806990 => unported;
    /// `RxLockedPipeAddFragment(pipe, firstIndex, nodeDef, NULL)` (0x806BE0, varargs; as called here)
    fn rx_locked_pipe_add_fragment(p: u32, first: u32, def: u32, end: u32) -> u32 = cdecl 0x806BE0 => unported;
    /// `RxLockedPipeUnlock(pipe)` (0x805F40)
    fn rx_locked_pipe_unlock(p: u32) -> u32 = cdecl 0x805F40 => unported;
    /// `_rxPipelineDestroy(pipe)` (0x805820)
    fn rx_pipeline_destroy(p: u32) = cdecl 0x805820 => unported;
    /// `RxPipelineFindNodeByName(pipe, name, start, index)` (0x806B30)
    fn rx_pipeline_find_node_by_name(p: u32, name: u32, start: u32, index: u32) -> u32 = cdecl 0x806B30 => unported;
    /// `RxNodeDefinitionGetD3D9AtomicAllInOne()` (0x7582E0)
    fn node_def_atomic_all_in_one() -> u32 = cdecl 0x7582E0 => unported;
    /// `RxD3D9AllInOneGetInstanceCallBack(node)` (0x757390)
    fn all_in_one_get_instance_cb(node: u32) -> u32 = cdecl 0x757390 => unported;
    /// `RxD3D9AllInOneSetInstanceCallBack(node, cb)` (0x757380)
    fn all_in_one_set_instance_cb(node: u32, cb: u32) = cdecl 0x757380 => unported;
    /// `RxD3D9AllInOneGetReinstanceCallBack(node)` (0x7573B0)
    fn all_in_one_get_reinstance_cb(node: u32) -> u32 = cdecl 0x7573B0 => unported;
    /// `RxD3D9AllInOneSetReinstanceCallBack(node, cb)` (0x7573A0)
    fn all_in_one_set_reinstance_cb(node: u32, cb: u32) = cdecl 0x7573A0 => unported;
    /// `RxD3D9AllInOneSetRenderCallBack(node, cb)` (0x7573E0)
    fn all_in_one_set_render_cb(node: u32, cb: u32) = cdecl 0x7573E0 => unported;
    /// `RwD3D9GetCaps()` (0x7FAD20): the `D3DCAPS9` of the device
    fn rw_d3d9_get_caps() -> u32 = cdecl 0x7FAD20 => unported;
    /// CRT `operator new` (0x82119A)
    fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;

    /// `CCustomCarEnvMapPipeline::CreateCustomObjPipe()` (0x5D9F80)
    fn car_create_custom_obj_pipe_call() -> u32 = cdecl 0x5D9F80 => car_env_map_create_custom_obj_pipe;
    /// `IsEnvironmentMappingSupported()` (0x5D8980)
    fn is_env_mapping_supported_call() -> u32 = cdecl 0x5D8980 => is_environment_mapping_supported;
    /// `CCustomCarEnvMapPipeline::CreatePipe()` (0x5DA020)
    fn car_create_pipe_call() -> u32 = cdecl 0x5DA020 => car_env_map_create_pipe;
    /// `CPool<CustomEnvMapPipeMaterialData>::CPool(size, name)` (0x5D9060)
    fn env_mat_pool_ctor_call(p: GPtr<CPool<EnvMapMatSlot>>, size: i32, name: GPtr<u8>) -> GPtr<CPool<EnvMapMatSlot>> = thiscall 0x5D9060 => env_mat_pool_ctor;
    /// `CPool<CustomEnvMapPipeAtomicData>::CPool(size, name)` (0x5D91B0)
    fn env_atm_pool_ctor_call(p: GPtr<CPool<EnvMapAtmSlot>>, size: i32, name: GPtr<u8>) -> GPtr<CPool<EnvMapAtmSlot>> = thiscall 0x5D91B0 => env_atm_pool_ctor;
    /// `CPool<CustomSpecMapPipeMaterialData>::CPool(size, name)` (0x5D9300)
    fn spec_mat_pool_ctor_call(p: GPtr<CPool<SpecMapMatSlot>>, size: i32, name: GPtr<u8>) -> GPtr<CPool<SpecMapMatSlot>> = thiscall 0x5D9300 => spec_mat_pool_ctor;
    /// `CCustomBuildingPipeline::CreateCustomObjPipe()` (0x5D7AA0)
    fn building_create_custom_obj_pipe_call() -> u32 = cdecl 0x5D7AA0 => building_create_custom_obj_pipe;
    /// `CCustomBuildingDNPipeline::CreateCustomObjPipe()` (0x5D6750)
    fn building_dn_create_custom_obj_pipe_call() -> u32 = cdecl 0x5D6750 => building_dn_create_custom_obj_pipe;
    /// `CCustomBuildingPipeline::CreatePipe()` (0x5D7D90)
    fn building_create_pipe_call() -> u32 = cdecl 0x5D7D90 => building_create_pipe;
    /// `CCustomBuildingDNPipeline::CreatePipe()` (0x5D7100)
    fn building_dn_create_pipe_call() -> u32 = cdecl 0x5D7100 => building_dn_create_pipe;
}

/// The all-in-one node of a pipeline built from definition `def` (found by its name).
fn find_all_in_one(pipe: u32, def: u32) -> u32 {
    rx_pipeline_find_node_by_name(pipe, rd::<u32>(GPtr::from_addr(def as usize), 0), 0, 0)
}

/// Tags a pipeline with its plugin id (+0x2C pluginId, +0x30 pluginData).
fn tag(pipe: u32, id: u32) {
    g::<u32>(pipe as usize + 0x2C).write(id);
    g::<u32>(pipe as usize + 0x30).write(id);
}

// ------------------------------------------------------------------ CCustomCarEnvMapPipeline ----

/// `CCustomCarEnvMapPipeline::CreateCustomObjPipe()` (0x5D9F80): `RxPipelineCreate`; NULL -> NULL.
/// Lock (NULL: the pipeline is destroyed, NULL), then the all-in-one definition is fetched,
/// `AddFragment(lock, NULL, def, NULL)` and its result unlocked (NULL: destroyed, NULL). The node
/// found by the definition's name keeps its instance callback and gets `CustomPipeInstanceCB`
/// (0x5D8490) as reinstance and `CustomPipeRenderCB` (0x5D9900) as render callback; id 0x53F2009A.
#[hook(addr = 0x5D9F80, conv = cdecl, name = "CCustomCarEnvMapPipeline::CreateCustomObjPipe", expect = "53 56 57 E8 28 B8 22 00",
       unverified = "std4-rxpipe-3", route = "L2", hazards = "heap-writes")]
pub fn car_env_map_create_custom_obj_pipe() -> u32 {
    let pipe = rx_pipeline_create();
    if pipe == 0 {
        return 0;
    }
    let lock = rx_pipeline_lock(pipe);
    if lock != 0 {
        let def = node_def_atomic_all_in_one();
        let f = rx_locked_pipe_add_fragment(lock, 0, def, 0);
        if rx_locked_pipe_unlock(f) != 0 {
            let node = find_all_in_one(pipe, def);
            all_in_one_set_instance_cb(node, all_in_one_get_instance_cb(node));
            all_in_one_set_reinstance_cb(node, CAR_INSTANCE_CB);
            all_in_one_set_render_cb(node, CAR_RENDER_CB);
            tag(pipe, ID_CAR_ENV_MAP);
            return pipe;
        }
    }
    rx_pipeline_destroy(pipe);
    0
}

/// `IsEnvironmentMappingSupported()` (0x5D8980), from the device caps: `MaxTextureBlendStages`
/// (+0x94) > 1 (signed), `MaxSimultaneousTextures` (+0x98) >= 2 (unsigned), `TextureOpCaps`
/// (+0x90) bits 13 and 24; then bit 9 of the dword at +0x38 (0 or 1). Else 0.
///
/// Not registered: 0x5D8980 is hooked by sa-render mvpa_render1.rs (verified) (duplicate port left unregistered when STD-5 merged both).
pub fn is_environment_mapping_supported() -> u32 {
    let caps = rw_d3d9_get_caps() as usize;
    if g::<i32>(caps + 0x94).read() <= 1 {
        return 0;
    }
    let two = g::<u32>(caps + 0x98).read() >= 2;
    let op = g::<u32>(caps + 0x90).read();
    if !two || (op >> 13) & 1 == 0 || (op >> 24) & 1 == 0 {
        return 0;
    }
    (g::<u32>(caps + 0x38).read() >> 9) & 1
}

/// `CPool<CustomEnvMapPipeMaterialData>::CPool(size, name)` (0x5D9060, thiscall, `ret 8`): both
/// arrays from `operator new` (slot 0xC).
///
/// Not registered: 0x5D9060 is hooked by sa-render mvpa_render1.rs (verified) (duplicate port left unregistered when STD-5 merged both).
pub fn env_mat_pool_ctor(p: GPtr<CPool<EnvMapMatSlot>>, size: i32, _name: GPtr<u8>) -> GPtr<CPool<EnvMapMatSlot>> {
    pool_construct(p, size, operator_new)
}

/// `CPool<CustomEnvMapPipeAtomicData>::CPool(size, name)` (0x5D91B0, thiscall, `ret 8`): both
/// arrays from `operator new` (slot 0xC).
///
/// Not registered: 0x5D91B0 is hooked by sa-render mvpa_render1.rs (verified) (duplicate port left unregistered when STD-5 merged both).
pub fn env_atm_pool_ctor(p: GPtr<CPool<EnvMapAtmSlot>>, size: i32, _name: GPtr<u8>) -> GPtr<CPool<EnvMapAtmSlot>> {
    pool_construct(p, size, operator_new)
}

/// `CPool<CustomSpecMapPipeMaterialData>::CPool(size, name)` (0x5D9300, thiscall, `ret 8`): both
/// arrays from `operator new` (slot 8).
///
/// Not registered: 0x5D9300 is hooked by sa-render mvpa_render1.rs (verified) (duplicate port left unregistered when STD-5 merged both).
pub fn spec_mat_pool_ctor(p: GPtr<CPool<SpecMapMatSlot>>, size: i32, _name: GPtr<u8>) -> GPtr<CPool<SpecMapMatSlot>> {
    pool_construct(p, size, operator_new)
}

/// `CCustomCarEnvMapPipeline::CreatePipe()` (0x5DA020): the pipeline -> 0xC02D24; NULL or no
/// environment-mapping support -> 0. Else `g_GameLight` (0xC02CB0, 0x1A dwords) is cleared and the three
/// pools are created (`new(0x14)` each; a failed `new` stores NULL): material data (0x1000,
/// "CustomEnvMapPipeMatData" 0x86BEE8), atomic data (0x400, 0x86BECC), specular material data
/// (0x1000, 0x86BEA8). Returns 1. (The exception frame of the `new`/constructor pairs is not
/// modelled: no constructor here can throw.)
#[hook(addr = 0x5DA020, conv = cdecl, name = "CCustomCarEnvMapPipeline::CreatePipe", expect = "6A FF 68 51 D7 83 00",
       unverified = "std4-rxpipe-3", route = "L2", hazards = "heap-writes")]
pub fn car_env_map_create_pipe() -> u32 {
    let p = car_create_custom_obj_pipe_call();
    CAR_ENV_MAP_PIPE.set(p);
    if p == 0 || is_env_mapping_supported_call() == 0 {
        return 0;
    }
    CAR_ENV_MAP_STATE.set([0; 0x1A]);
    let m = operator_new(0x14);
    let m = if m.is_null() { GPtr::NULL } else { env_mat_pool_ctor_call(m.cast(), 0x1000, GPtr::from_addr(0x86BEE8)).cast() };
    ENV_MAT_POOL.set(m);
    let a = operator_new(0x14);
    let a = if a.is_null() { GPtr::NULL } else { env_atm_pool_ctor_call(a.cast(), 0x400, GPtr::from_addr(0x86BECC)).cast() };
    ENV_ATM_POOL.set(a);
    let s = operator_new(0x14);
    let s = if s.is_null() { GPtr::NULL } else { spec_mat_pool_ctor_call(s.cast(), 0x1000, GPtr::from_addr(0x86BEA8)).cast() };
    SPEC_MAT_POOL.set(s);
    1
}

/// `CCarFXRenderer::Initialise()` (0x5D5AC0): `CCustomCarEnvMapPipeline::CreatePipe() != 0`.
#[hook(addr = 0x5D5AC0, conv = cdecl, name = "CCarFXRenderer::Initialise", expect = "E8 5B 45 00 00 F7 D8",
       unverified = "std4-rxpipe-3", route = "L2", hazards = "none")]
pub fn car_fx_renderer_initialise() -> u32 {
    (car_create_pipe_call() != 0) as u32
}

// ------------------------------------------------------------------ CCustomBuilding(DN)Pipeline --

/// `CCustomBuildingPipeline::CreateCustomObjPipe()` (0x5D7AA0): `RxPipelineCreate`, then the
/// all-in-one definition (fetched before the NULL test); Lock, `AddFragment(lock, NULL, def, NULL)`,
/// Unlock(lock), each checked (failure: destroyed, NULL). The node keeps its instance callback and
/// gets `CustomPipeReinstanceCB` (0x5D77A0) / `CustomPipeRenderCB` (0x5D77D0); id 0x53F2009C.
#[hook(addr = 0x5D7AA0, conv = cdecl, name = "CCustomBuildingPipeline::CreateCustomObjPipe", expect = "53 56 57 E8 08 DD 22 00",
       unverified = "std4-rxpipe-3", route = "L2", hazards = "heap-writes")]
pub fn building_create_custom_obj_pipe() -> u32 {
    let pipe = rx_pipeline_create();
    let def = node_def_atomic_all_in_one();
    if pipe == 0 {
        return 0;
    }
    let lock = rx_pipeline_lock(pipe);
    if lock != 0 && rx_locked_pipe_add_fragment(lock, 0, def, 0) != 0 && rx_locked_pipe_unlock(lock) != 0 {
        let node = find_all_in_one(pipe, def);
        all_in_one_set_instance_cb(node, all_in_one_get_instance_cb(node));
        all_in_one_set_reinstance_cb(node, BUILDING_REINSTANCE_CB);
        all_in_one_set_render_cb(node, BUILDING_RENDER_CB);
        tag(pipe, ID_BUILDING);
        return pipe;
    }
    rx_pipeline_destroy(pipe);
    0
}

/// `CCustomBuildingDNPipeline::CreateCustomObjPipe()` (0x5D6750): as the building pipeline; the node
/// keeps its instance callback, and its reinstance callback becomes `CustomPipeReinstanceCB`
/// (0x5D63E0) when `[0xC02C20]` is set, else its own; render callback `CustomPipeRenderCB`
/// (0x5D6480); id 0x53F20098.
#[hook(addr = 0x5D6750, conv = cdecl, name = "CCustomBuildingDNPipeline::CreateCustomObjPipe", expect = "53 56 57 E8 58 F0 22 00",
       unverified = "std4-rxpipe-3", route = "L2", hazards = "heap-writes")]
pub fn building_dn_create_custom_obj_pipe() -> u32 {
    let pipe = rx_pipeline_create();
    let def = node_def_atomic_all_in_one();
    if pipe == 0 {
        return 0;
    }
    let lock = rx_pipeline_lock(pipe);
    if lock != 0 && rx_locked_pipe_add_fragment(lock, 0, def, 0) != 0 && rx_locked_pipe_unlock(lock) != 0 {
        let node = find_all_in_one(pipe, def);
        if BUILDING_DN_FLAG.get() != 0 {
            all_in_one_set_instance_cb(node, all_in_one_get_instance_cb(node));
            all_in_one_set_reinstance_cb(node, DN_REINSTANCE_CB);
        } else {
            all_in_one_set_instance_cb(node, all_in_one_get_instance_cb(node));
            all_in_one_set_reinstance_cb(node, all_in_one_get_reinstance_cb(node));
        }
        all_in_one_set_render_cb(node, DN_RENDER_CB);
        tag(pipe, ID_BUILDING_DN);
        return pipe;
    }
    rx_pipeline_destroy(pipe);
    0
}

/// `CCustomBuildingPipeline::CreatePipe()` (0x5D7D90): the pipeline -> 0xC02C68; returns it != NULL.
#[hook(addr = 0x5D7D90, conv = cdecl, name = "CCustomBuildingPipeline::CreatePipe", expect = "E8 0B FD FF FF 33 C9",
       unverified = "std4-rxpipe-3", route = "L2", hazards = "none")]
pub fn building_create_pipe() -> u32 {
    let p = building_create_custom_obj_pipe_call();
    BUILDING_PIPE.set(p);
    (p != 0) as u32
}

/// `CCustomBuildingDNPipeline::CreatePipe()` (0x5D7100): the pipeline -> 0xC02C1C; returns it != NULL.
#[hook(addr = 0x5D7100, conv = cdecl, name = "CCustomBuildingDNPipeline::CreatePipe", expect = "E8 4B F6 FF FF 33 C9",
       unverified = "std4-rxpipe-3", route = "L2", hazards = "none")]
pub fn building_dn_create_pipe() -> u32 {
    let p = building_dn_create_custom_obj_pipe_call();
    BUILDING_DN_PIPE.set(p);
    (p != 0) as u32
}

/// `CCustomBuildingRenderer::Initialise()` (0x5D7EC0): the building pipeline (0 -> 0), then
/// `CCustomBuildingDNPipeline::CreatePipe() != 0`.
#[hook(addr = 0x5D7EC0, conv = cdecl, name = "CCustomBuildingRenderer::Initialise", expect = "E8 CB FE FF FF 85 C0",
       unverified = "std4-rxpipe-3", route = "L2", hazards = "none")]
pub fn custom_building_renderer_initialise() -> u32 {
    let r = building_create_pipe_call();
    if r == 0 {
        return r;
    }
    (building_dn_create_pipe_call() != 0) as u32
}

/// `CGrassRenderer::Initialise()` (0x5DD6B0, between the two pipeline initialisers in
/// `CGame::InitialiseEssentialsAfterRW`): `mov al, 1; ret` (true in AL, the upper EAX bits are the
/// caller's). `#[native]`: the 3-byte body (then padding) is too short for phase A's 5-byte patch.
#[sa_core::native(addr = 0x5DD6B0, conv = cdecl, name = "CGrassRenderer::Initialise")]
pub fn grass_renderer_initialise() -> bool {
    true
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_CAR_ENV_MAP_CREATE_CUSTOM_OBJ_PIPE,
    HOOK_CAR_ENV_MAP_CREATE_PIPE,
    HOOK_CAR_FX_RENDERER_INITIALISE,
    HOOK_BUILDING_CREATE_CUSTOM_OBJ_PIPE,
    HOOK_BUILDING_DN_CREATE_CUSTOM_OBJ_PIPE,
    HOOK_BUILDING_CREATE_PIPE,
    HOOK_BUILDING_DN_CREATE_PIPE,
    HOOK_CUSTOM_BUILDING_RENDERER_INITIALISE,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_GRASS_RENDERER_INITIALISE];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = GLOBALS_LIST;
