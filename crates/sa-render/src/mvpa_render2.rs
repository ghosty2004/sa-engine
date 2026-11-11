//! MVP-A writer batch `mvpa-render-2`: shadow cameras / real-time shadows, the env-map pipe's pool
//! allocators, FX manager/emitter helpers and small render callbacks. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid. Every hook is unverified.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{Fpu, g, k, rd, wr};

orig! {
    /// `strncpy` (0x821F40)
    pub fn crt_strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// CRT `free` (0x8214BD)
    pub fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `Light::RemoveExtraDirectionalLights(RpWorld*)` (0x7359E0)
    pub fn remove_extra_directional_lights(world: GPtr<u8>) = cdecl 0x7359E0 => unported;
    /// `FxPrim_c::~FxPrim_c()` (0x4A9F40)
    pub fn fx_prim_destructor(this: GPtr<u8>) = thiscall 0x4A9F40 => unported;
    /// `FxEmitter_c::~FxEmitter_c()` (0x4A2BB0)
    pub fn fx_emitter_destructor_call(this: GPtr<u8>) = thiscall 0x4A2BB0 => fx_emitter_destructor;
    /// `FxEmitter_c::CreateParticles(float currTime, float timeStep)` (0x4A41E0)
    pub fn fx_emitter_create_particles(this: GPtr<u8>, a: u32, b: u32) = thiscall 0x4A41E0 => unported;
    /// `CCustomCarEnvMapPipeline::CustomPipeAtomicSetup(RpAtomic*)` (0x5DA610)
    pub fn custom_pipe_atomic_setup(atomic: GPtr<u8>) -> GPtr<u8> = cdecl 0x5DA610 => unported;
    /// `_rwObjectHasFrameSetFrame(void* object, RwFrame*)` (0x804EF0)
    pub fn rw_object_has_frame_set_frame(obj: GPtr<u8>, frame: GPtr<u8>) = cdecl 0x804EF0 => unported;
    /// `RwFrameDestroy(RwFrame*)` (0x7F05A0)
    pub fn rw_frame_destroy(frame: GPtr<u8>) -> i32 = cdecl 0x7F05A0 => unported;
    /// `RwFrameCreate()` (0x7F0410)
    pub fn rw_frame_create() -> GPtr<u8> = cdecl 0x7F0410 => unported;
    /// `RwFrameUpdateObjects(RwFrame*)` (0x7F0910)
    pub fn rw_frame_update_objects(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => unported;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0)
    pub fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18A0 => unported;
    /// `RwRasterDestroy(RwRaster*)` (0x7FB020)
    pub fn rw_raster_destroy(r: GPtr<u8>) -> i32 = cdecl 0x7FB020 => unported;
    /// `RwRasterCreate(w, h, depth, flags)` (0x7FB230)
    pub fn rw_raster_create(w: i32, h: i32, depth: i32, flags: i32) -> GPtr<u8> = cdecl 0x7FB230 => unported;
    /// `RwTextureSetRaster(RwTexture*, RwRaster*)` (0x7F35D0)
    pub fn rw_texture_set_raster(t: GPtr<u8>, r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F35D0 => unported;
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    pub fn rw_texture_destroy(t: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
    /// `RwTextureCreate(RwRaster*)` (0x7F37C0)
    pub fn rw_texture_create(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F37C0 => unported;
    /// `RwCameraDestroy(RwCamera*)` (0x7EE4B0)
    pub fn rw_camera_destroy(c: GPtr<u8>) -> i32 = cdecl 0x7EE4B0 => unported;
    /// `RwCameraCreate()` (0x7EE4F0)
    pub fn rw_camera_create() -> GPtr<u8> = cdecl 0x7EE4F0 => unported;
    /// `RwCameraSetProjection(RwCamera*, RwCameraProjection)` (0x7EE3A0)
    pub fn rw_camera_set_projection(c: GPtr<u8>, p: u32) -> GPtr<u8> = cdecl 0x7EE3A0 => unported;
    /// `RwCameraSetFarClipPlane(RwCamera*, float)` (0x7EE2A0)
    pub fn rw_camera_set_far_clip_plane(c: GPtr<u8>, f: u32) -> GPtr<u8> = cdecl 0x7EE2A0 => unported;
    /// `RwCameraSetNearClipPlane(RwCamera*, float)` (0x7EE1D0)
    pub fn rw_camera_set_near_clip_plane(c: GPtr<u8>, f: u32) -> GPtr<u8> = cdecl 0x7EE1D0 => unported;
    /// `RwCameraSetViewWindow(RwCamera*, const RwV2d*)` (0x7EE410)
    pub fn rw_camera_set_view_window(c: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE410 => unported;
    /// `RpLightCreate(RwInt32 type)` (0x752110)
    pub fn rp_light_create(ty: i32) -> GPtr<u8> = cdecl 0x752110 => unported;
    /// `RpLightDestroy(RpLight*)` (0x7520D0)
    pub fn rp_light_destroy(l: GPtr<u8>) -> i32 = cdecl 0x7520D0 => unported;
    /// `RpLightSetColor(RpLight*, const RwRGBAReal*)` (0x751A90)
    pub fn rp_light_set_color(l: GPtr<u8>, c: GPtr<u8>) -> GPtr<u8> = cdecl 0x751A90 => unported;
    /// `CShadowCamera::Destroy()` (0x705400)
    pub fn shadow_camera_destroy_call(this: GPtr<u8>) = thiscall 0x705400 => shadow_camera_destroy;
    /// `CShadowCamera::Create(int rasterSizePower)` (0x705B60)
    pub fn shadow_camera_create_call(this: GPtr<u8>, n: u32) -> GPtr<u8> = thiscall 0x705B60 => shadow_camera_create;
    /// `CShadowCamera::SetLight(RpLight*)` (0x705520)
    pub fn shadow_camera_set_light_call(this: GPtr<u8>, l: GPtr<u8>) -> GPtr<u8> = thiscall 0x705520 => shadow_camera_set_light;
    /// `CRealTimeShadow::Destroy()` (0x705990)
    pub fn real_time_shadow_destroy_call(this: GPtr<u8>) = thiscall 0x705990 => real_time_shadow_destroy;
    /// `Fx_c::GetFxQuality()` (0x49EA50, on g_fx 0xA9AE00)
    pub fn fx_get_fx_quality(this: GPtr<u8>) -> i32 = thiscall 0x49EA50 => unported;
    /// `CRealTimeShadowManager::GetRealTimeShadow(CPhysical*)` (0x706970)
    pub fn get_real_time_shadow(this: GPtr<u8>, e: GPtr<u8>) = thiscall 0x706970 => unported;
    /// `List_c::AddItem(ListItem_c*)` (0x4A8DF0)
    pub fn list_add_item(l: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8DF0 => unported;
    /// `List_c::RemoveItem(ListItem_c*)` (0x4A8E30)
    pub fn list_remove_item(l: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8E30 => unported;
    /// `FxSystem_c::Exit()` (0x4AA840)
    pub fn fx_system_exit(this: GPtr<u8>) = thiscall 0x4AA840 => unported;
    /// `FxSystem_c::~FxSystem_c()` (0x4AA260)
    pub fn fx_system_destructor(this: GPtr<u8>) = thiscall 0x4AA260 => unported;
    /// `CCustomRoadsignMgr::CreateRoadsignAtomicA(...)` (0x6FEDA0), 9 cdecl arguments
    pub fn create_roadsign_atomic_a(a0: u32, a1: u32, a2: u32, l1: GPtr<u8>, l2: GPtr<u8>, l3: GPtr<u8>, l4: GPtr<u8>,
        a7: u32, a8: u32) -> GPtr<u8> = cdecl 0x6FEDA0 => unported;
}

/// The env-map pipe's `CPool<T>::New()` (thiscall, objects +0, flags +4, size +8, firstFree +0xC):
/// `firstFree` advances (stored each step) to the next slot whose flag has bit 7 (free) set; reaching
/// `size` (read once at entry) the first time restarts at 0, the second time stores 0 and returns NULL.
/// The found flag loses bit 7, then its low 7 bits (the id) are incremented in place
/// (`b ^ (((b + 1) ^ b) & 0x7F)`, two read-modify-writes as in the original); returns
/// `objects + firstFree * elem`.
fn env_map_pool_new(this: GPtr<u8>, elem: i32) -> GPtr<u8> {
    let size = rd::<i32>(this, 8);
    let mut wrapped = false;
    loop {
        let i = rd::<i32>(this, 0xC).wrapping_add(1);
        wr::<i32>(this, 0xC, i);
        if i == size {
            wr::<i32>(this, 0xC, 0);
            if wrapped {
                return GPtr::NULL;
            }
            wrapped = true;
        }
        let i = rd::<i32>(this, 0xC);
        if rd::<GPtr<u8>>(this, 4).byte_add(i as isize).read() & 0x80 != 0 {
            break;
        }
    }
    let f = rd::<GPtr<u8>>(this, 4).byte_add(rd::<i32>(this, 0xC) as isize);
    f.write(f.read() & 0x7F);
    let f = rd::<GPtr<u8>>(this, 4).byte_add(rd::<i32>(this, 0xC) as isize);
    let b = f.read();
    f.write(b ^ ((b.wrapping_add(1) ^ b) & 0x7F));
    let i = rd::<i32>(this, 0xC);
    rd::<GPtr<u8>>(this, 0).byte_add(i.wrapping_mul(elem) as isize)
}

/// `CPool<CustomEnvMapPipeMaterialData>::New()` (0x5D9110): 12-byte objects; see [`env_map_pool_new`].
/// The wrap step runs through the protection chunk 0x401F94 (stores firstFree = 0).
#[hook(addr = 0x5D9110, conv = thiscall, name = "FUN_005d9110", expect = "8B 51 08 56 32 C0",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the pool's flag array (heap)")]
pub fn env_map_material_pool_new(this: GPtr<u8>) -> GPtr<u8> {
    env_map_pool_new(this, 12)
}

/// `CPool<CustomEnvMapPipeAtomicData>::New()` (0x5D9260): 12-byte objects; see [`env_map_pool_new`].
#[hook(addr = 0x5D9260, conv = thiscall, name = "FUN_005d9260", expect = "8B 51 08 56 32 C0",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the pool's flag array (heap)")]
pub fn env_map_atomic_pool_new(this: GPtr<u8>) -> GPtr<u8> {
    env_map_pool_new(this, 12)
}

/// `CPool<CustomSpecMapPipeMaterialData>::New()` (0x5D93B0): 8-byte objects; see [`env_map_pool_new`].
#[hook(addr = 0x5D93B0, conv = thiscall, name = "FUN_005d93b0", expect = "8B 51 08 56 32 C0",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the pool's flag array (heap)")]
pub fn spec_map_material_pool_new(this: GPtr<u8>) -> GPtr<u8> {
    env_map_pool_new(this, 8)
}

/// `cb_RSNODENAME_RwFrame_Copy(void* dst, const void* src, int offset, int size)` (0x72F9F0, cdecl,
/// the node-name frame plugin's copy callback): `strncpy(dst + gPluginOffset, src + gPluginOffset,
/// 23)` with the plugin offset at 0xC87C5C; returns dst.
#[hook(addr = 0x72F9F0, conv = cdecl, name = "cb_RSNODENAME_RwFrame_Copy", expect = "A1 5C 7C C8 00",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the frame plugin data (heap)")]
pub fn node_name_frame_copy(dst: GPtr<u8>, src: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let off = g::<i32>(0xC87C5C).read() as isize;
    crt_strncpy(dst.byte_add(off), src.byte_add(off), 0x17);
    dst
}

/// `CPointLights::RemoveLightsAffectingObject()` (0x6FFFE0, cdecl): `RemoveExtraDirectionalLights
/// (Scene.world)`; the entry jumps into the protection chunk 0x401C11, which decodes the address
/// 0xC17038 (`Scene.world`) and loads it.
#[hook(addr = 0x6FFFE0, conv = cdecl, name = "CPointLights::RemoveLightsAffectingObject", expect = "E9 2C 1C D0 FF 50",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn point_lights_remove_lights_affecting_object() {
    remove_extra_directional_lights(g::<GPtr<u8>>(0xC17038).read());
}

/// `FxEmitter_c::~FxEmitter_c()` (0x4A2BB0, thiscall): a tail jump to `FxPrim_c::~FxPrim_c()`.
#[hook(addr = 0x4A2BB0, conv = thiscall, name = "FxEmitter_c::~FxEmitter_c", expect = "E9 8B 73 00 00 90",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn fx_emitter_destructor(this: GPtr<u8>) {
    fx_prim_destructor(this);
}

/// `FxEmitter_c::`scalar deleting destructor'(u8 flags)` (0x4A2B90, thiscall, `ret 4`): the destructor,
/// `free(this)` when bit 0 of the flags is set; returns this.
#[hook(addr = 0x4A2B90, conv = thiscall, name = "FxEmitter_c::`scalar deleting destructor'", expect = "56 8B F1 E8 18 00 00 00",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "frees the emitter (heap)")]
pub fn fx_emitter_scalar_deleting_destructor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    fx_emitter_destructor_call(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `FxEmitter_c::Update(float currTime, float timeStep)` (0x4A4460, thiscall, `ret 8`): when the
/// emitter is active (+0xC byte) and its blueprint (+8) has no flag 8 at byte +0x62, a tail jump to
/// `CreateParticles(currTime, timeStep)`.
#[hook(addr = 0x4A4460, conv = thiscall, name = "FxEmitter_c::Update_Reversed", expect = "8A 41 0C 84 C0 74 0E",
       unverified = "mvpa-render-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn fx_emitter_update(this: GPtr<u8>, curr_time: u32, time_step: u32) {
    if rd::<u8>(this, 0xC) == 0 {
        return;
    }
    let bp = rd::<GPtr<u8>>(this, 8);
    if rd::<u8>(bp, 0x62) & 8 != 0 {
        return;
    }
    fx_emitter_create_particles(this, curr_time, time_step);
}

/// `CMBlur::ClearDrunkBlur()` (0x71D780): `Drunkness (0xB7CB64) = 1.0f` (in the protection chunk
/// 0x401190, which returns to 0x71D78A) and `0xC73C58 = 0`. The entry is five NOPs.
#[hook(addr = 0x71D780, conv = cdecl, name = "CMBlur::ClearDrunkBlur", expect = "90 90 90 90 90 E9 06 3A CE FF",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn mblur_clear_drunk_blur() {
    g::<u32>(0xB7CB64).write(0x3F80_0000);
    g::<u32>(0xC73C58).write(0);
}

/// `CRealTimeShadowManager::CRealTimeShadowManager()` (0x706770, thiscall, run by a static
/// initialiser): +0x44..+0x50 = 0, bytes +0/+1 = 0, the 16 shadow pointers (+4) = NULL; returns this.
#[hook(addr = 0x706770, conv = thiscall, name = "FUN_00706770", expect = "8B D1 33 C0 89 42 44",
       route = "split", hazards = "none", recs = "grove-walk",
       note = "static constructor of g_realTimeShadowMan (runs once at startup)")]
pub fn real_time_shadow_manager_ctor(this: GPtr<u8>) -> GPtr<u8> {
    for o in [0x44, 0x48, 0x4C, 0x50] {
        wr::<u32>(this, o, 0);
    }
    wr::<u8>(this, 0, 0);
    wr::<u8>(this, 1, 0);
    let mut i = 0;
    while i < 16 {
        wr::<u32>(this, 4 + i * 4, 0);
        i += 1;
    }
    this
}

/// The car env-map pipe's atomic setup callback (0x5D5B30, cdecl, reached through a data reference):
/// returns `CCustomCarEnvMapPipeline::CustomPipeAtomicSetup(atomic)`.
#[hook(addr = 0x5D5B30, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 04 50 E8 D6 4A 00 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn car_env_map_atomic_setup_cb(atomic: GPtr<u8>) -> GPtr<u8> {
    custom_pipe_atomic_setup(atomic)
}

/// `CShadowCamera::Destroy()` (0x705400, thiscall): with a camera (+0): its frame (camera+4) detached
/// (`_rwObjectHasFrameSetFrame(cam, NULL)`) and destroyed; the camera's raster (+0x60) cleared and
/// destroyed; the texture (+4) gets a NULL raster, is destroyed and +4 = NULL; `RwCameraDestroy`;
/// +0 = NULL.
#[hook(addr = 0x705400, conv = thiscall, name = "CShadowCamera::Destroy", expect = "56 8B F1 8B 06 85 C0",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "destroys RW camera/frame/raster/texture objects")]
pub fn shadow_camera_destroy(this: GPtr<u8>) {
    let cam = rd::<GPtr<u8>>(this, 0);
    if cam.is_null() {
        return;
    }
    let frame = rd::<GPtr<u8>>(cam, 4);
    if !frame.is_null() {
        rw_object_has_frame_set_frame(cam, GPtr::NULL);
        rw_frame_destroy(frame);
    }
    let cam = rd::<GPtr<u8>>(this, 0);
    let raster = rd::<GPtr<u8>>(cam, 0x60);
    if !raster.is_null() {
        wr::<u32>(cam, 0x60, 0);
        rw_raster_destroy(raster);
    }
    let tex = rd::<GPtr<u8>>(this, 4);
    if !tex.is_null() {
        rw_texture_set_raster(tex, GPtr::NULL);
        rw_texture_destroy(rd::<GPtr<u8>>(this, 4));
        wr::<u32>(this, 4, 0);
    }
    rw_camera_destroy(rd::<GPtr<u8>>(this, 0));
    wr::<u32>(this, 0, 0);
}

/// `CRealTimeShadow::Destroy()` (0x705990, thiscall): both shadow cameras (+8, +0x14) `Destroy()`;
/// `+0 = NULL` (stored in the protection chunk 0x401CD6), `+0x24 = -1`; with a light (+0x28): its frame
/// detached and destroyed, `RpLightDestroy`, +0x28 = NULL.
#[hook(addr = 0x705990, conv = thiscall, name = "CRealTimeShadow::Destroy", expect = "56 8B F1 8D 4E 08",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "destroys RW objects")]
pub fn real_time_shadow_destroy(this: GPtr<u8>) {
    shadow_camera_destroy_call(this.byte_add(8));
    shadow_camera_destroy_call(this.byte_add(0x14));
    let light = rd::<GPtr<u8>>(this, 0x28);
    wr::<u32>(this, 0, 0);
    wr::<i32>(this, 0x24, -1);
    if light.is_null() {
        return;
    }
    let frame = rd::<GPtr<u8>>(light, 4);
    rw_object_has_frame_set_frame(light, GPtr::NULL);
    rw_frame_destroy(frame);
    rp_light_destroy(rd::<GPtr<u8>>(this, 0x28));
    wr::<u32>(this, 0x28, 0);
}

/// `CShadowCamera::SetLight(RpLight*)` (0x705520, thiscall, `ret 4`): copies right, up and at (3 dwords
/// each, not pos) of the light's frame modelling matrix (frame +0x10) into the camera's frame
/// modelling matrix, `RwMatrixUpdate`, `RwFrameUpdateObjects`; returns the camera (+0).
#[hook(addr = 0x705520, conv = thiscall, name = "CShadowCamera::SetLight", expect = "8B 54 24 04 53 55 56",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the camera frame matrix (heap)")]
pub fn shadow_camera_set_light(this: GPtr<u8>, light: GPtr<u8>) -> GPtr<u8> {
    let frame = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 4);
    let src = rd::<GPtr<u8>>(light, 4).byte_add(0x10);
    let dst = frame.byte_add(0x10);
    for o in [0, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28] {
        wr::<u32>(dst, o, rd::<u32>(src, o));
    }
    rw_matrix_update(dst);
    rw_frame_update_objects(frame);
    rd::<GPtr<u8>>(this, 0)
}

/// `CShadowCamera::SetFrustum(float size)` (0x7054C0, thiscall, `ret 4`): far clip `size + size`, near
/// clip `size * 0.001f` (x87, stored as f32), view window (size, size) from a stack `RwV2d`; returns
/// the camera (+0).
#[hook(addr = 0x7054C0, conv = thiscall, name = "CShadowCamera::SetFrustum", expect = "83 EC 08 D9 44 24 0C",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the RW camera (heap)")]
pub fn shadow_camera_set_frustum(this: GPtr<u8>, size: u32) -> GPtr<u8> {
    let s = f32::from_bits(size);
    let mut f = Fpu::new();
    f.fld(s);
    f.fadd_st(0);
    let far = f.fstp();
    rw_camera_set_far_clip_plane(rd::<GPtr<u8>>(this, 0), far.to_bits());
    f.fld(s);
    f.fmul_m(k(0x3A83_126F));
    let near = f.fstp();
    rw_camera_set_near_clip_plane(rd::<GPtr<u8>>(this, 0), near.to_bits());
    let mut v = [size, size];
    rw_camera_set_view_window(rd::<GPtr<u8>>(this, 0), GPtr::from_raw(v.as_mut_ptr()).cast());
    rd::<GPtr<u8>>(this, 0)
}

/// `CRealTimeShadowManager::DoShadowThisFrame(CPhysical*)` (0x706BA0, thiscall, `ret 4`): `onlyPed =
/// entity type (+0x36 & 7) == 3 && entity +0x598 == 0`; when `g_fx.GetFxQuality() == 3`, or `== 2`
/// (queried again) with onlyPed: an existing shadow (+0x134) gets byte +4 = 1, otherwise
/// `GetRealTimeShadow(entity)`.
#[hook(addr = 0x706BA0, conv = thiscall, name = "CRealTimeShadowManager::DoShadowThisFrame", expect = "53 56 8B 74 24 0C",
       route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the entity's shadow object (heap)")]
pub fn real_time_shadow_manager_do_shadow_this_frame(this: GPtr<u8>, e: GPtr<u8>) {
    let ped = rd::<u8>(e, 0x36) & 7 == 3 && rd::<u32>(e, 0x598) == 0;
    let fx = GPtr::<u8>::from_addr(0xA9AE00);
    if fx_get_fx_quality(fx) != 3 && (fx_get_fx_quality(fx) != 2 || !ped) {
        return;
    }
    let shadow = rd::<GPtr<u8>>(e, 0x134);
    if !shadow.is_null() {
        wr::<u8>(shadow, 4, 1);
        return;
    }
    get_real_time_shadow(this, e);
}

/// `FxManager_c::DestroyFxSystem(FxSystem_c*)` (0x4A9810, thiscall, `ret 4`, ECX preserved by a
/// push/pop): for every prim of the system's blueprint (+8: count i8 at +0x1B re-read per iteration,
/// prims at +0x1C) the particles of its list (+0x20, next at +4 read first) owned by this system (+0x28)
/// are moved to the free particle list (0xA9AE9C); the system is removed from the manager's list
/// (+0xC), `Exit()`, destroyed and freed.
#[hook(addr = 0x4A9810, conv = thiscall, name = "FxManager_c::DestroyFxSystem", expect = "51 53 57 8B 7C 24 10",
       preserve = "ecx", unverified = "mvpa-render-2", route = "L2", hazards = "heap-writes,preserved-regs",
       recs = "grove-walk", note = "frees the FX system; the original keeps ECX (push ecx .. pop ecx)")]
pub fn fx_manager_destroy_fx_system(this: GPtr<u8>, sys: GPtr<u8>) {
    let bp = rd::<GPtr<u8>>(sys, 8);
    if rd::<i8>(bp, 0x1B) > 0 {
        let mut i: i32 = 0;
        loop {
            let prims = rd::<GPtr<GPtr<u8>>>(rd::<GPtr<u8>>(sys, 8), 0x1C);
            let prim = prims.wrapping_index(i).read();
            let mut p = rd::<GPtr<u8>>(prim, 0x20);
            while !p.is_null() {
                let owner = rd::<GPtr<u8>>(p, 0x28);
                let next = rd::<GPtr<u8>>(p, 4);
                if owner == sys {
                    let prims = rd::<GPtr<GPtr<u8>>>(rd::<GPtr<u8>>(sys, 8), 0x1C);
                    list_remove_item(prims.wrapping_index(i).read().byte_add(0x20), p);
                    list_add_item(GPtr::from_addr(0xA9AE9C), p);
                }
                p = next;
            }
            i += 1;
            if i >= i32::from(rd::<i8>(rd::<GPtr<u8>>(sys, 8), 0x1B)) {
                break;
            }
        }
    }
    list_remove_item(this.byte_add(0xC), sys);
    fx_system_exit(sys);
    fx_system_destructor(sys);
    crt_free(sys);
}

/// `CCustomRoadsignMgr::CreateRoadsignAtomic(a0, a1, a2, line1, line2, line3, line4, numLetters,
/// palette)` (0x6FF2D0, cdecl): every NULL line is replaced by a 16-byte stack string `" "` (a space
/// and 15 zero bytes); returns `CreateRoadsignAtomicA` with the same arguments.
#[hook(addr = 0x6FF2D0, conv = cdecl, name = "CCustomRoadsignMgr::CreateRoadsignAtomic", expect = "83 EC 10 33 C0",
       route = "L2", hazards = "none", recs = "grove-walk")]
#[allow(clippy::too_many_arguments)]
pub fn custom_roadsign_create_roadsign_atomic(
    a0: u32,
    a1: u32,
    a2: u32,
    l1: GPtr<u8>,
    l2: GPtr<u8>,
    l3: GPtr<u8>,
    l4: GPtr<u8>,
    a7: u32,
    a8: u32,
) -> GPtr<u8> {
    let mut blank = [0u8; 16];
    blank[0] = 0x20;
    let b = GPtr::from_raw(blank.as_mut_ptr());
    let pick = |l: GPtr<u8>| if l.is_null() { b } else { l };
    create_roadsign_atomic_a(a0, a1, a2, pick(l1), pick(l2), pick(l3), pick(l4), a7, a8)
}

/// `CShadowCamera::Create(int rasterSizePower)` (0x705B60, thiscall, `ret 4`): `size = 1 << n`;
/// camera = `RwCameraCreate()` (+0), a new frame attached; a `size x size` camera raster (depth 0,
/// type 5) stored at camera+0x60 and wrapped in a texture (+4, addressing 0x33, filter 2); parallel
/// projection; returns the camera. Any failure: `Destroy()`, NULL.
#[hook(addr = 0x705B60, conv = thiscall, name = "CShadowCamera::Create", expect = "56 57 8B F1 8B 4C 24 0C",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "creates RW camera/frame/raster/texture objects")]
pub fn shadow_camera_create(this: GPtr<u8>, n: u32) -> GPtr<u8> {
    let size = 1i32 << (n & 31);
    let ok = 'b: {
        let cam = rw_camera_create();
        wr::<GPtr<u8>>(this, 0, cam);
        if cam.is_null() {
            break 'b false;
        }
        let frame = rw_frame_create();
        rw_object_has_frame_set_frame(rd::<GPtr<u8>>(this, 0), frame);
        if rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 4).is_null() {
            break 'b false;
        }
        let raster = rw_raster_create(size, size, 0, 5);
        if raster.is_null() {
            break 'b false;
        }
        wr::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x60, raster);
        let tex = rw_texture_create(raster);
        wr::<GPtr<u8>>(this, 4, tex);
        if tex.is_null() {
            break 'b false;
        }
        wr::<u8>(tex, 0x51, 0x33);
        wr::<u8>(rd::<GPtr<u8>>(this, 4), 0x50, 2);
        rw_camera_set_projection(rd::<GPtr<u8>>(this, 0), 2);
        true
    };
    if ok {
        return rd::<GPtr<u8>>(this, 0);
    }
    shadow_camera_destroy_call(this);
    GPtr::NULL
}

/// `CRealTimeShadow::Create(bool isBlurred, int blurPasses, bool drawMoreBlur)` (0x706460, thiscall,
/// `ret 0xC`): light (+0x28) = `RpLightCreate(1)` (NULL → AL = 0, EAX 0); colour (0.8, 0.8, 0.8, 0)
/// from a stack `RwRGBAReal`; a new frame attached; camera (+8) `Create(7)`; +0x1C blurPasses, +0x10
/// isBlurred, +0x20 drawMoreBlur; when blurred the blur camera (+0x14) `Create(6)`; either camera failing
/// → `Destroy()`, AL = 0; else `SetLight` on the camera, AL = 1. The only caller ignores the result.
#[hook(addr = 0x706460, conv = thiscall, name = "CRealTimeShadow::Create", expect = "83 EC 10 56 6A 01",
       route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "creates RW light/camera objects")]
pub fn real_time_shadow_create(this: GPtr<u8>, blurred: u32, passes: u32, more_blur: u32) -> u8 {
    let light = rp_light_create(1);
    wr::<GPtr<u8>>(this, 0x28, light);
    if light.is_null() {
        return 0;
    }
    let mut col = [0x3F4C_CCCDu32, 0x3F4C_CCCD, 0x3F4C_CCCD, 0];
    rp_light_set_color(light, GPtr::from_raw(col.as_mut_ptr()).cast());
    let frame = rw_frame_create();
    rw_object_has_frame_set_frame(rd::<GPtr<u8>>(this, 0x28), frame);
    let ok = 'b: {
        if shadow_camera_create_call(this.byte_add(8), 7).is_null() {
            break 'b false;
        }
        wr::<u32>(this, 0x1C, passes);
        wr::<u8>(this, 0x10, blurred as u8);
        wr::<u8>(this, 0x20, more_blur as u8);
        if blurred as u8 != 0 && shadow_camera_create_call(this.byte_add(0x14), 6).is_null() {
            break 'b false;
        }
        true
    };
    if !ok {
        real_time_shadow_destroy_call(this);
        return 0;
    }
    shadow_camera_set_light_call(this.byte_add(8), rd::<GPtr<u8>>(this, 0x28));
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_ENV_MAP_MATERIAL_POOL_NEW,
    HOOK_ENV_MAP_ATOMIC_POOL_NEW,
    HOOK_SPEC_MAP_MATERIAL_POOL_NEW,
    HOOK_NODE_NAME_FRAME_COPY,
    HOOK_POINT_LIGHTS_REMOVE_LIGHTS_AFFECTING_OBJECT,
    HOOK_FX_EMITTER_DESTRUCTOR,
    HOOK_FX_EMITTER_SCALAR_DELETING_DESTRUCTOR,
    HOOK_FX_EMITTER_UPDATE,
    HOOK_MBLUR_CLEAR_DRUNK_BLUR,
    HOOK_REAL_TIME_SHADOW_MANAGER_CTOR,
    HOOK_CAR_ENV_MAP_ATOMIC_SETUP_CB,
    HOOK_SHADOW_CAMERA_DESTROY,
    HOOK_REAL_TIME_SHADOW_DESTROY,
    HOOK_SHADOW_CAMERA_SET_LIGHT,
    HOOK_SHADOW_CAMERA_SET_FRUSTUM,
    HOOK_REAL_TIME_SHADOW_MANAGER_DO_SHADOW_THIS_FRAME,
    HOOK_FX_MANAGER_DESTROY_FX_SYSTEM,
    HOOK_CUSTOM_ROADSIGN_CREATE_ROADSIGN_ATOMIC,
    HOOK_SHADOW_CAMERA_CREATE,
    HOOK_REAL_TIME_SHADOW_CREATE,
];
