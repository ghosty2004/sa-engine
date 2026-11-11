//! PORT-RENDER: `CRenderer` (cdecl statics): render lists, LOD lists, sector scanning, visibility.
//! Ported from the 1.0 US bytes (0x5531C0..0x555DB0); gta-reversed `Renderer.cpp` and
//! `docs/re/specs/RENDER/CRenderer*.md` as reading aids.
//!
//! Entity fields used (CEntity): +0x18 `m_pRwObject`, +0x1C flags (bit 7 `m_bIsVisible`, bit 14
//! `m_bDrawLast`, bit 15 `m_bDistanceFade`, bit 20 `m_bUnderwater`, bit 25 `m_bIsBIGBuilding`-style
//! "LOD has visible children" bit 0x2000000, bit 27 `m_bBackfaceCulled`), +0x22 model index (int16),
//! +0x2C scan code (u16), +0x2F area (u8), +0x30 `m_pLod`, +0x34 `m_nNumLodChildren` (u8), +0x35
//! `m_nNumLodChildrenRendered` (int8), +0x36 type in bits 0..2.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::x87::X87;

use crate::common::{g, jne, jp, k, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `CRenderer::ms_nNoOfVisibleSuperLods`
    pub static NO_OF_VISIBLE_SUPER_LODS @ 0xB76838: i32 = 0;
    /// `CRenderer::ms_nNoOfInVisibleEntities`
    pub static NO_OF_INVISIBLE_ENTITIES @ 0xB7683C: i32 = 0;
    /// `CRenderer::ms_nNoOfVisibleLods`
    pub static NO_OF_VISIBLE_LODS @ 0xB76840: i32 = 0;
    /// `CRenderer::ms_nNoOfVisibleEntities`
    pub static NO_OF_VISIBLE_ENTITIES @ 0xB76844: i32 = 0;
    /// `gpOutEntitiesForGetObjectsInFrustum` (CEntity**)
    pub static OUT_ENTITIES @ 0xB76854: u32 = 0;
    /// `CRenderer::ms_pLodRenderList` (tRenderListEntry*)
    pub static LOD_RENDER_LIST @ 0xB745D0: u32 = 0;
    /// `CRenderer::ms_pLodDontRenderList` (tRenderListEntry*)
    pub static LOD_DONT_RENDER_LIST @ 0xB745CC: u32 = 0;
}

/// `CRenderer::ms_aVisibleLodPtrs` (CEntity*[1000]).
pub const VISIBLE_LOD_PTRS: usize = 0xB748F8;
/// `CRenderer::ms_aVisibleEntityPtrs` (CEntity*[1000]).
pub const VISIBLE_ENTITY_PTRS: usize = 0xB75898;
/// `CRenderer::ms_aVisibleSuperLodPtrs` (CEntity*[50]).
pub const VISIBLE_SUPER_LOD_PTRS: usize = 0xB74830;
/// `CRenderer::ms_aInVisibleEntityPtrs` (CEntity*[150]).
pub const INVISIBLE_ENTITY_PTRS: usize = 0xB745D8;
/// `PC_Scratch` (0xC8E0C8): the five scan lists (`CPtrList*`) of `SetupScanLists` live in its first 0x14
/// bytes, the LOD render list at +0x18, the LOD don't-render list at +0x2000.
pub const SCAN_LISTS: usize = 0xC8E0C8;
/// `CRenderer::GetLodRenderListBase()` result.
pub const LOD_RENDER_LIST_BASE: u32 = 0xC8E0E0;
/// `CRenderer::GetLodDontRenderListBase()` result.
pub const LOD_DONT_RENDER_LIST_BASE: u32 = 0xC900C8;
/// `CModelInfo::ms_modelInfoPtrs` (CBaseModelInfo*[20000]).
pub const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CWorld::ms_aRepeatSectors` (16x16 `CRepeatSector`, 0xC bytes).
pub const REPEAT_SECTORS: usize = 0xB992B8;
/// `CWorld::ms_nCurrentScanCode` (u16).
pub const CURRENT_SCAN_CODE: usize = 0xB7CD78;
/// `CGame::currArea` (int32).
pub const CURR_AREA: usize = 0xB72914;
/// `RwEngineInstance` (RwGlobals*).
pub const RW_ENGINE_INSTANCE: usize = 0xC97B24;

orig! {
    /// `RwRenderStateSet` through `RwEngineInstance->dOpenDevice.fpRenderStateSet` (+0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `DeActivateDirectional()` (0x735C70)
    pub fn de_activate_directional() = cdecl 0x735C70 => unported;
    /// `SetAmbientColours()` (0x735D30)
    pub fn set_ambient_colours() = cdecl 0x735D30 => unported;
    /// `CVisibilityPlugins::RenderFadingEntities()` (0x733F10)
    pub fn vp_render_fading_entities() = cdecl 0x733F10 => unported;
    /// `CVisibilityPlugins::RenderFadingUnderwaterEntities()` (0x7337D0)
    pub fn vp_render_fading_underwater_entities() = cdecl 0x7337D0 => unported;
    /// `CPostEffects::IsVisionFXActive()` (0x7034F0)
    pub fn is_vision_fx_active() -> u8 = cdecl 0x7034F0 => unported;
    /// `CPostEffects::FilterFX_StoreAndSetDayNightBalance()` (0x7034B0)
    pub fn filter_fx_store_and_set_day_night_balance() = cdecl 0x7034B0 => unported;
    /// `CPostEffects::FilterFX_RestoreDayNightBalance()` (0x7034D0)
    pub fn filter_fx_restore_day_night_balance() = cdecl 0x7034D0 => unported;
    /// `CPostEffects::NightVisionSetLights()` (0x7012E0)
    pub fn night_vision_set_lights() = cdecl 0x7012E0 => unported;
    /// `CPostEffects::InfraredVisionSetLightsForDefaultObjects()` (0x701430)
    pub fn infrared_vision_set_lights_for_default_objects() = cdecl 0x701430 => unported;
    /// `CEntity::Render()` (vtable +0x48)
    pub fn entity_render(e: GPtr<u8>) = thiscall vslot 0x48 => unported;
    /// `CEntity::SetupLighting()` (vtable +0x4C)
    pub fn entity_setup_lighting(e: GPtr<u8>) -> u8 = thiscall vslot 0x4C => unported;
    /// `CEntity::RemoveLighting(bool)` (vtable +0x50); the argument dword is passed whole
    pub fn entity_remove_lighting(e: GPtr<u8>, lit: u32) = thiscall vslot 0x50 => unported;
    /// `CEntity::PreRender()` (vtable +0x44)
    pub fn entity_pre_render(e: GPtr<u8>) = thiscall vslot 0x44 => unported;
    /// `CVisibilityPlugins::SetupVehicleVariables(RpClump*)` (0x733160)
    pub fn vp_setup_vehicle_variables(clump: GPtr<u8>) = cdecl 0x733160 => unported;
    /// `CVisibilityPlugins::InitAlphaAtomicList()` (0x734530)
    pub fn vp_init_alpha_atomic_list() = cdecl 0x734530 => unported;
    /// `CVisibilityPlugins::RenderAlphaAtomics()` (0x733E90)
    pub fn vp_render_alpha_atomics() = cdecl 0x733E90 => unported;
    /// `CVehicle::RenderDriverAndPassengers()` (0x6D3D60)
    pub fn vehicle_render_driver_and_passengers(v: GPtr<u8>) = thiscall 0x6D3D60 => unported;
    /// `CVehicle::SetupRender()` (0x6D64F0)
    pub fn vehicle_setup_render(v: GPtr<u8>) = thiscall 0x6D64F0 => unported;
    /// `CVehicle::ResetAfterRender()` (0x6D0E20)
    pub fn vehicle_reset_after_render(v: GPtr<u8>) = thiscall 0x6D0E20 => unported;
    /// `CPointLights::RemoveLightsAffectingObject()` (0x6FFFE0)
    pub fn remove_lights_affecting_object() = cdecl 0x6FFFE0 => unported;
    /// `CVisibilityPlugins::InsertEntityIntoSortedList(CEntity*, float)` (0x734570); the float as its dword
    pub fn vp_insert_entity_into_sorted_list(e: GPtr<u8>, dist: u32) -> u8 = cdecl 0x734570 => unported;
    /// `CVisibilityPlugins::InsertEntityIntoUnderwaterEntities(CEntity*, float)` (0x733D90)
    pub fn vp_insert_entity_into_underwater_entities(e: GPtr<u8>, dist: u32) -> u8 = cdecl 0x733D90 => unported;
    /// `CWorld::GetSector(int32, int32)` (0x407260)
    pub fn get_sector(x: i32, y: i32) -> GPtr<u8> = cdecl 0x407260 => unported;
    /// `CEntity::IsVisible()` (0x536BC0)
    pub fn entity_is_visible(e: GPtr<u8>) -> u8 = thiscall 0x536BC0 => unported;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    pub fn request_model(model: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CRenderer::AddEntityToRenderList(CEntity*, float)` (0x5534B0); the float as its dword
    pub fn call_add_entity_to_render_list(e: GPtr<u8>, dist: u32) = cdecl 0x5534B0 => unported;
    /// `CRenderer::SetupScanLists(int32, int32)` (0x553540)
    pub fn call_setup_scan_lists(x: i32, y: i32) = cdecl 0x553540 => setup_scan_lists;
    /// `CRenderer::RenderOneNonRoad(CEntity*)` (0x553260)
    pub fn render_one_non_road_call(e: GPtr<u8>) = cdecl 0x553260 => render_one_non_road;
    /// `CHeli::SpecialHeliPreRender()` (0x6C4750)
    pub fn special_heli_pre_render() = cdecl 0x6C4750 => unported;
    /// `CShadows::RenderExtraPlayerShadows()` (0x707FA0)
    pub fn render_extra_player_shadows() = cdecl 0x707FA0 => unported;
    /// `CVisibilityPlugins::GetClumpAlpha(RpClump*)` (0x732B20)
    pub fn vp_get_clump_alpha(clump: GPtr<u8>) -> i32 = cdecl 0x732B20 => unported;
    /// `CCamera::GetLookDirection()` (0x50AE90)
    pub fn get_look_direction(cam: GPtr<u8>) -> i32 = thiscall 0x50AE90 => unported;
    /// `RwCameraEndUpdate(RwCamera*)` (0x7EE180)
    pub fn rw_camera_end_update(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE180 => unported;
    /// `RwCameraBeginUpdate(RwCamera*)` (0x7EE190)
    pub fn rw_camera_begin_update(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EE190 => unported;
    /// `CCarFXRenderer::SetFxEnvMapLightMult(float)` (0x5D5BA0); the float as its dword
    pub fn set_fx_env_map_light_mult(m: u32) = cdecl 0x5D5BA0 => unported;
    /// `WorldReplaceNormalLightsWithScorched(RpWorld*, float)` (0x7357E0); the float as its dword
    pub fn world_replace_normal_lights_with_scorched(world: GPtr<u8>, l: u32) = cdecl 0x7357E0 => unported;
    /// `CPointLights::GenerateLightsAffectingObject(const CVector*, float*, CEntity*)` (0x6FFBB0, ST0)
    pub fn generate_lights_affecting_object(pos: GPtr<f32>, bright: GPtr<f32>, e: GPtr<u8>) -> f64 = cdecl 0x6FFBB0 => unported;
    /// `CPhysical::GetLightingFromCol(bool)` (0x5447B0, ST0); the dword the original pushes
    pub fn get_lighting_from_col(e: GPtr<u8>, interior: u32) -> f64 = thiscall 0x5447B0 => unported;
    /// `SetLightColoursForPedsCarsAndObjects(float)` (0x735D90); the float as its dword
    pub fn set_light_colours_for_peds_cars_and_objects(m: u32) = cdecl 0x735D90 => unported;
    /// `CRenderer::AddToLodRenderList(CEntity*, float)` (0x553710); the float as its dword
    pub fn add_to_lod_render_list_call(e: GPtr<u8>, dist: u32) = cdecl 0x553710 => unported;
    /// `CEntity::CreateRwObject()` (vtable +0x1C)
    pub fn entity_create_rw_object(e: GPtr<u8>) = thiscall vslot 0x1C => unported;
    /// `CEntity::DeleteRwObject()` (vtable +0x20)
    pub fn entity_delete_rw_object(e: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// `CEntity::GetIsOnScreen()` (0x534540)
    pub fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `CEntity::IsEntityOccluded()` (0x71FAE0)
    pub fn entity_is_entity_occluded(e: GPtr<u8>) -> u8 = thiscall 0x71FAE0 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, result through the hidden pointer)
    pub fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    pub fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CBaseModelInfo::AsAtomicModelInfoPtr()` (vtable +0x4)
    pub fn mi_as_atomic_model_info_ptr(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CBaseModelInfo::GetModelType()` (vtable +0x10, AL)
    pub fn mi_get_model_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
    /// `CBaseModelInfo::GetTimeInfo()` (vtable +0x14)
    pub fn mi_get_time_info(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x14 => unported;
    /// `CClock::GetIsTimeInRange(uint8, uint8)` (0x52CEE0); the zero-extended dwords
    pub fn get_is_time_in_range(on: u32, off: u32) -> u8 = cdecl 0x52CEE0 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// The protection thunk 0x407000 (`thunk_FUN_01562960`, thiscall on the entity, AL)
    pub fn fun_407000(e: GPtr<u8>) -> u8 = thiscall 0x407000 => unported;
    /// `CRenderer::SetupMapEntityVisibility(CEntity*, CBaseModelInfo*, float, bool)` (0x553F60)
    pub fn call_setup_map_entity_visibility(e: GPtr<u8>, mi: GPtr<u8>, dist: u32, t: u8) -> i32 = cdecl 0x553F60 => setup_map_entity_visibility_b;
    /// `CRenderer::SetupEntityVisibility(CEntity*, float&)` (0x554230)
    pub fn call_setup_entity_visibility(e: GPtr<u8>, out: GPtr<f32>) -> i32 = cdecl 0x554230 => setup_entity_visibility;
    /// `CRenderer::SetupBigBuildingVisibility(CEntity*, float&)` (0x554650)
    pub fn call_setup_big_building_visibility(e: GPtr<u8>, out: GPtr<f32>) -> i32 = cdecl 0x554650 => setup_big_building_visibility;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0); the float as its dword
    pub fn limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CRenderer::ShouldModelBeStreamed(CEntity*, const CVector&, float)` (0x554EB0); the float as its dword
    pub fn call_should_model_be_streamed(e: GPtr<u8>, p: GPtr<f32>, far: u32) -> u8 = cdecl 0x554EB0 => should_model_be_streamed_b;
    /// `CRenderer::ScanPtrList_RequestModels<...>(CPtrList&)` (0x555680)
    pub fn call_scan_ptr_list_request_models(list: GPtr<u8>) = cdecl 0x555680 => scan_ptr_list_request_models;
    /// `CCullZones::FindTunnelAttributesForCoors(CVector)` (0x72D9F0; the vector by value = 3 dwords)
    pub fn find_tunnel_attributes_for_coors(x: u32, y: u32, z: u32) -> u32 = cdecl 0x72D9F0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CCamera::CalculateGroundHeight(eGroundHeightType)` (0x514B80, ST0)
    pub fn calculate_ground_height(cam: GPtr<u8>, kind: i32) -> f64 = thiscall 0x514B80 => unported;
    /// `CMirrors::BeforeConstructRenderList()` (0x726DF0)
    pub fn mirrors_before_construct_render_list() = cdecl 0x726DF0 => unported;
    /// `COcclusion::ProcessBeforeRendering()` (0x7201C0)
    pub fn occlusion_process_before_rendering() = cdecl 0x7201C0 => unported;
    /// `CRenderer::ScanWorld()` (0x554FE0)
    pub fn scan_world() = cdecl 0x554FE0 => scan_world_port;
    /// `CVisibilityPlugins::InitAlphaEntityList()` (0x734540)
    pub fn vp_init_alpha_entity_list() = cdecl 0x734540 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    pub fn clear_scan_codes() = cdecl 0x563470 => unported;
    /// `RwV3dTransformPoints(RwV3d* out, const RwV3d* in, int32 n, const RwMatrix*)` (0x7EDD90)
    pub fn rw_v3d_transform_points(out: GPtr<f32>, inp: GPtr<f32>, n: i32, m: GPtr<u8>) -> GPtr<f32> = cdecl 0x7EDD90 => unported;
    /// `CWorldScan::ScanWorld(CVector2D*, int32, void (*)(int32, int32))` (0x72CAE0); the callback address
    pub fn world_scan(points: GPtr<f32>, n: i32, f: u32) = cdecl 0x72CAE0 => unported;
    /// `CRenderer::ProcessLodRenderLists()` (0x553770)
    pub fn call_process_lod_render_lists() = cdecl 0x553770 => process_lod_render_lists;
    /// `CStreaming::StartRenderEntities()` (0x4096C0)
    pub fn start_render_entities() = cdecl 0x4096C0 => unported;
}

/// Phase B/C path of `call_should_model_be_streamed`.
pub fn should_model_be_streamed_b(e: GPtr<u8>, p: GPtr<f32>, far: u32) -> u8 {
    should_model_be_streamed(e, p, f32::from_bits(far))
}

/// Phase B/C path of `call_setup_map_entity_visibility` (the float argument as its dword).
pub fn setup_map_entity_visibility_b(e: GPtr<u8>, mi: GPtr<u8>, dist: u32, t: u8) -> i32 {
    setup_map_entity_visibility(e, mi, f32::from_bits(dist), t)
}

/// Entity type (`+0x36 & 7`).
#[inline]
fn etype(e: GPtr<u8>) -> u8 {
    rd::<u8>(e, 0x36) & 7
}

/// `CModelInfo::ms_modelInfoPtrs[e->m_nModelIndex]` (movsx of the int16 index).
#[inline]
fn model_info_of(e: GPtr<u8>) -> GPtr<u8> {
    let idx = i32::from(rd::<i16>(e, 0x22));
    g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(idx).read()
}

/// `CRenderer::Init()` (0x5531C0): an empty function (`ret`). Not hooked: the entry is 1 byte.
pub fn init() {}

/// `CRenderer::Shutdown()` (0x5531D0): an empty function (`ret`). Not hooked: the entry is 1 byte.
pub fn shutdown() {}

/// `CRenderer::RenderFadingInEntities()` (0x5531E0).
#[hook(addr = 0x5531E0, conv = cdecl, name = "CRenderer::RenderFadingInEntities", expect = "A1 24 7B C9 00")]
pub fn render_fading_in_entities() {
    rw_render_state_set(0xE, 1); // rwRENDERSTATEFOGENABLE
    rw_render_state_set(0xC, 1); // rwRENDERSTATEVERTEXALPHAENABLE
    rw_render_state_set(0x14, 2); // rwRENDERSTATECULLMODE = CULLBACK
    de_activate_directional();
    set_ambient_colours();
    vp_render_fading_entities();
}

/// `CRenderer::RenderFadingInUnderwaterEntities()` (0x553220).
#[hook(addr = 0x553220, conv = cdecl, name = "CRenderer::RenderFadingInUnderwaterEntities", expect = "E8 4B 2A 1E 00")]
pub fn render_fading_in_underwater_entities() {
    de_activate_directional();
    set_ambient_colours();
    vp_render_fading_underwater_entities();
}

/// `CRenderer::RenderOneRoad(CEntity*)` (0x553230).
#[hook(addr = 0x553230, conv = cdecl, name = "CRenderer::RenderOneRoad", expect = "E8 BB 02 1B 00")]
pub fn render_one_road(e: GPtr<u8>) {
    if is_vision_fx_active() != 0 {
        filter_fx_store_and_set_day_night_balance();
        entity_render(e);
        filter_fx_restore_day_night_balance();
    } else {
        entity_render(e);
    }
}

/// `CRenderer::RenderOneNonRoad(CEntity*)` (0x553260). The `SetupLighting` result is stored into the low
/// byte of the argument slot, and `RemoveLighting` gets that whole dword (entity pointer's upper bytes).
#[hook(addr = 0x553260, conv = cdecl, name = "CRenderer::RenderOneNonRoad", expect = "56 8B 74 24 08")]
pub fn render_one_non_road(e: GPtr<u8>) {
    if etype(e) == 3 && rd::<i32>(e, 0x530) == 0x32 {
        return;
    }
    let lit = entity_setup_lighting(e);
    let slot = (e.addr() as u32 & 0xFFFF_FF00) | u32::from(lit);
    if etype(e) == 2 {
        vp_setup_vehicle_variables(rd::<GPtr<u8>>(e, 0x18));
        vp_init_alpha_atomic_list();
        vehicle_render_driver_and_passengers(e);
        vehicle_setup_render(e);
    } else if rd::<u32>(e, 0x1C) & 0x800_0000 == 0 {
        rw_render_state_set(0x14, 1); // CULLNONE
    }
    if is_vision_fx_active() != 0 {
        if rd::<u8>(g(0xC402B8), 0) != 0 {
            night_vision_set_lights();
        }
        if rd::<u8>(g(0xC402B9), 0) != 0 {
            infrared_vision_set_lights_for_default_objects();
        }
        filter_fx_store_and_set_day_night_balance();
        entity_render(e);
        filter_fx_restore_day_night_balance();
    } else {
        entity_render(e);
    }
    if etype(e) == 2 {
        wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) | 0x2000);
        vp_render_alpha_atomics();
        wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) & 0xFFFF_DFFF);
        vehicle_reset_after_render(e);
        entity_remove_lighting(e, slot);
    } else {
        if rd::<u32>(e, 0x1C) & 0x800_0000 == 0 {
            rw_render_state_set(0x14, 2); // CULLBACK
        }
        entity_remove_lighting(e, slot);
    }
}

/// `CRenderer::RemoveVehiclePedLights(CPhysical*)` (0x553390): tail jump unless `bRenderScorched`.
#[hook(addr = 0x553390, conv = cdecl, name = "CRenderer::RemoveVehiclePedLights", expect = "8B 44 24 04 F7 40 40 00 00 00 20")]
pub fn remove_vehicle_ped_lights(e: GPtr<u8>) {
    if rd::<u32>(e, 0x40) & 0x2000_0000 == 0 {
        remove_lights_affecting_object();
    }
}

/// `CRenderer::AddEntityToRenderList(CEntity*, float)` (0x5534B0). The model-info lookup runs in the
/// protection chunk 0x406B11 (`eax = ms_modelInfoPtrs[eax]`); inlined.
#[hook(addr = 0x5534B0, conv = cdecl, name = "CRenderer::AddEntityToRenderList", expect = "56 8B 74 24 08")]
pub fn add_entity_to_render_list(e: GPtr<u8>, dist: u32) {
    let mi = model_info_of(e);
    wr::<u8>(mi, 0x12, rd::<u8>(mi, 0x12) & 0xFE);
    let flags = rd::<u32>(e, 0x1C);
    if flags & 0x8000 != 0 {
        if vp_insert_entity_into_sorted_list(e, dist) != 0 {
            return;
        }
    } else if flags & 0x4000 != 0 && vp_insert_entity_into_sorted_list(e, dist) != 0 {
        wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) & 0xFFFF_7FFF);
        return;
    }
    if rd::<u8>(e, 0x34) > 0 && rd::<u32>(e, 0x1C) & 0x10_0000 == 0 {
        let n = NO_OF_VISIBLE_LODS.get();
        g::<GPtr<u8>>(VISIBLE_LOD_PTRS).wrapping_index(n).write(e);
        NO_OF_VISIBLE_LODS.set(n.wrapping_add(1));
    } else {
        let n = NO_OF_VISIBLE_ENTITIES.get();
        g::<GPtr<u8>>(VISIBLE_ENTITY_PTRS).wrapping_index(n).write(e);
        NO_OF_VISIBLE_ENTITIES.set(n.wrapping_add(1));
    }
}

/// `CRenderer::SetupScanLists(int32 x, int32 y)` (0x553540): the five scan lists in `PC_Scratch`
/// (sector buildings, repeat-sector vehicles / peds / objects, sector dummies), or 0 for the sector lists
/// outside the 120x120 grid.
#[hook(addr = 0x553540, conv = cdecl, name = "CRenderer::SetupScanLists", expect = "8B 54 24 08 8B 4C 24 04")]
pub fn setup_scan_lists(x: i32, y: i32) {
    let rep = REPEAT_SECTORS as u32 + ((((y & 0xF) << 4) + (x & 0xF)) as u32).wrapping_mul(12);
    let lists = g::<u32>(SCAN_LISTS);
    let (s0, s4) = if x < 0 || y < 0 || x >= 120 || y >= 120 {
        (0u32, 0u32)
    } else {
        let s = get_sector(x, y).addr() as u32;
        (s, s.wrapping_add(4))
    };
    lists.wrapping_index(2).write(rep);
    lists.wrapping_index(0).write(s0);
    lists.wrapping_index(3).write(rep.wrapping_add(4));
    lists.wrapping_index(1).write(rep.wrapping_add(8));
    lists.wrapping_index(4).write(s4);
}

/// The scan loop shared by `ScanSectorList_ListModels` / `_ListModelsVisible`: every entity of the five
/// lists not yet seen in this scan (scan code), in the current area or area 13.
fn scan_lists_for_models(visible_only: bool) {
    let lists = g::<u32>(SCAN_LISTS);
    for i in 0..5 {
        let list = lists.wrapping_index(i).read();
        if list == 0 {
            continue;
        }
        let mut node = GPtr::<u8>::from_addr(rd::<u32>(GPtr::from_addr(list as usize), 0) as usize);
        while !node.is_null() {
            let e = rd::<GPtr<u8>>(node, 0);
            let code = rd::<u16>(g(CURRENT_SCAN_CODE), 0);
            node = rd::<GPtr<u8>>(node, 4);
            if rd::<u16>(e, 0x2C) == code {
                continue;
            }
            wr::<u16>(e, 0x2C, code);
            let area = rd::<u8>(e, 0x2F);
            if i32::from(area) != rd::<i32>(g(CURR_AREA), 0) && area != 13 {
                continue;
            }
            if visible_only && entity_is_visible(e) == 0 {
                continue;
            }
            let out = OUT_ENTITIES.get();
            GPtr::<GPtr<u8>>::from_addr(out as usize).write(e);
            OUT_ENTITIES.set(out.wrapping_add(4));
        }
    }
}

/// `CRenderer::ScanSectorList_ListModels(int32 x, int32 y)` (0x5535D0). The original keeps the output
/// pointer in a register and stores it once at the end; nothing it calls reads it.
#[hook(addr = 0x5535D0, conv = cdecl, name = "CRenderer::ScanSectorList_ListModels", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn scan_sector_list_list_models(x: i32, y: i32) {
    call_setup_scan_lists(x, y);
    scan_lists_for_models(false);
}

/// `CRenderer::ScanSectorList_ListModelsVisible(int32 x, int32 y)` (0x553650).
#[hook(addr = 0x553650, conv = cdecl, name = "CRenderer::ScanSectorList_ListModelsVisible", expect = "8B 44 24 08 8B 4C 24 04")]
pub fn scan_sector_list_list_models_visible(x: i32, y: i32) {
    call_setup_scan_lists(x, y);
    scan_lists_for_models(true);
}

/// `CRenderer::GetLodRenderListBase()` (0x5536D0): `&PC_Scratch[0x18]`.
#[hook(addr = 0x5536D0, conv = cdecl, name = "CRenderer::GetLodRenderListBase", expect = "B8 E0 E0 C8 00")]
pub fn get_lod_render_list_base() -> u32 {
    LOD_RENDER_LIST_BASE
}

/// `CRenderer::GetLodDontRenderListBase()` (0x5536E0): `&PC_Scratch[0x2000]`.
#[hook(addr = 0x5536E0, conv = cdecl, name = "CRenderer::GetLodDontRenderListBase", expect = "B8 C8 00 C9 00")]
pub fn get_lod_dont_render_list_base() -> u32 {
    LOD_DONT_RENDER_LIST_BASE
}

/// `CRenderer::ResetLodRenderLists()` (0x5536F0).
#[hook(addr = 0x5536F0, conv = cdecl, name = "CRenderer::ResetLodRenderLists", expect = "C7 05 D0 45 B7 00 E0 E0 C8 00")]
pub fn reset_lod_render_lists() {
    LOD_RENDER_LIST.set(LOD_RENDER_LIST_BASE);
    LOD_DONT_RENDER_LIST.set(LOD_DONT_RENDER_LIST_BASE);
}

/// `{ entity, distance }` appended at `*list`, which then advances by 8. The float goes through
/// `fld`/`fstp dword` (a signalling NaN is quieted).
fn add_to_list(list: &sa_core::mem::Global<u32>, e: GPtr<u8>, dist: f32) {
    let p = GPtr::<u8>::from_addr(list.get() as usize);
    wr::<GPtr<u8>>(p, 0, e);
    let q = GPtr::<u8>::from_addr(list.get() as usize);
    wr::<f32>(q, 4, quiet(dist));
    list.set(list.get().wrapping_add(8));
}

/// `fld dword` + `fstp dword`: a signalling NaN comes out quiet.
#[inline]
pub fn quiet(v: f32) -> f32 {
    if v.is_nan() {
        f32::from_bits(v.to_bits() | 0x40_0000)
    } else {
        v
    }
}

/// `CRenderer::AddToLodRenderList(CEntity*, float)` (0x553710).
#[hook(addr = 0x553710, conv = cdecl, name = "CRenderer::AddToLodRenderList", expect = "A1 D0 45 B7 00")]
pub fn add_to_lod_render_list(e: GPtr<u8>, dist: f32) {
    add_to_list(&LOD_RENDER_LIST, e, dist);
}

/// `CRenderer::AddToLodDontRenderList(CEntity*, float)` (0x553740).
#[hook(addr = 0x553740, conv = cdecl, name = "CRenderer::AddToLodDontRenderList", expect = "A1 CC 45 B7 00")]
pub fn add_to_lod_dont_render_list(e: GPtr<u8>, dist: f32) {
    add_to_list(&LOD_DONT_RENDER_LIST, e, dist);
}

/// `CRenderer::ProcessLodRenderLists()` (0x553770). Every loop re-reads `ms_pLodRenderList` as its end.
#[hook(addr = 0x553770, conv = cdecl, name = "CRenderer::ProcessLodRenderLists", expect = "51 53 55 56 57")]
pub fn process_lod_render_lists() {
    let base = LOD_RENDER_LIST_BASE;
    let slot = |p: u32| GPtr::<u8>::from_addr(p as usize);
    // 1: drop the LODs that are not visible.
    let mut p = base;
    while p != LOD_RENDER_LIST.get() {
        let e = rd::<GPtr<u8>>(slot(p), 0);
        if !e.is_null() && rd::<u8>(e, 0x1C) & 0x80 == 0 {
            wr::<u8>(e, 0x35, 0);
            wr::<u32>(slot(p), 0, 0);
        }
        p = p.wrapping_add(8);
    }
    // 2: repeat until nothing changes.
    loop {
        let mut changed = false;
        let mut p = base;
        if LOD_RENDER_LIST.get() == base {
            break;
        }
        loop {
            let e = rd::<GPtr<u8>>(slot(p), 0);
            if !e.is_null() {
                let children = rd::<u8>(e, 0x34);
                if children > 0 && i32::from(rd::<i8>(e, 0x35)) == i32::from(children) {
                    wr::<u8>(e, 0x35, 0);
                    wr::<u32>(slot(p), 0, 0);
                    changed = true;
                } else {
                    let lod = rd::<GPtr<u8>>(e, 0x30);
                    if !lod.is_null() {
                        let mi = model_info_of(e);
                        if rd::<u8>(mi, 0xC) < 0xFF
                            && rd::<u8>(lod, 0x35) != 0x80
                            && rd::<u32>(lod, 0x1C) & 0x200_0000 != 0
                        {
                            wr::<u8>(lod, 0x35, 0);
                        }
                        if rd::<u32>(e, 0x18) == 0 {
                            if rd::<u32>(lod, 0x1C) & 0x200_0000 != 0 {
                                wr::<u8>(lod, 0x35, 0x80);
                            }
                            wr::<u32>(slot(p), 0, 0);
                            wr::<u8>(e, 0x35, 0);
                            request_model(i32::from(rd::<i16>(e, 0x22)), 0);
                        }
                    }
                }
            }
            p = p.wrapping_add(8);
            if p == LOD_RENDER_LIST.get() {
                break;
            }
        }
        if !changed {
            break;
        }
    }
    // 3: LODs with rendered children are not drawn themselves.
    let mut p = base;
    while p != LOD_RENDER_LIST.get() {
        let e = rd::<GPtr<u8>>(slot(p), 0);
        if !e.is_null() && rd::<i8>(e, 0x35) > 0 {
            wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) & 0xFDFF_FFFF);
            wr::<u8>(e, 0x35, 0);
            wr::<u32>(slot(p), 0, 0);
        }
        p = p.wrapping_add(8);
    }
    // 4: what is left goes to the render list.
    let mut p = base;
    while p != LOD_RENDER_LIST.get() {
        let e = rd::<GPtr<u8>>(slot(p), 0);
        if !e.is_null() {
            let c = rd::<u8>(e, 0x35);
            if c == 0x80 || c == 0 {
                let mi = model_info_of(e);
                let mut f = rd::<u32>(e, 0x1C) | 0x200_0000;
                wr::<u32>(e, 0x1C, f);
                if rd::<u8>(mi, 0xC) != 0xFF {
                    f |= 0x8000;
                    wr::<u32>(e, 0x1C, f);
                }
                call_add_entity_to_render_list(e, rd::<u32>(slot(p), 4));
            }
            wr::<u8>(e, 0x35, 0);
        }
        p = p.wrapping_add(8);
    }
}

/// `CRenderer::PreRender()` (0x553910): `PreRender` of every listed entity (each loop re-reads its
/// counter), of the alpha-list entries drawn by `CVisibilityPlugins::RenderEntity` (clearing their
/// `m_bOffscreen` bit 17 first), then `CHeli::SpecialHeliPreRender` and a tail jump to
/// `CShadows::RenderExtraPlayerShadows`.
#[hook(addr = 0x553910, conv = cdecl, name = "CRenderer::PreRender", expect = "A1 40 68 B7 00")]
pub fn pre_render() {
    for (count, arr) in [
        (&NO_OF_VISIBLE_LODS, VISIBLE_LOD_PTRS),
        (&NO_OF_VISIBLE_ENTITIES, VISIBLE_ENTITY_PTRS),
        (&NO_OF_VISIBLE_SUPER_LODS, VISIBLE_SUPER_LOD_PTRS),
        (&NO_OF_INVISIBLE_ENTITIES, INVISIBLE_ENTITY_PTRS),
    ] {
        let mut i = 0i32;
        while i < count.get() {
            entity_pre_render(g::<GPtr<u8>>(arr).wrapping_index(i).read());
            i += 1;
        }
    }
    for (head, tail) in [(0xC88130usize, 0xC88134u32), (0xC88188, 0xC8818C)] {
        let mut link = rd::<u32>(g(head), 0);
        while link != tail {
            let l = GPtr::<u8>::from_addr(link as usize);
            if rd::<u32>(l, 4) == 0x732B40 {
                let e = rd::<GPtr<u8>>(l, 0);
                wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) & 0xFFFD_FFFF);
                entity_pre_render(rd::<GPtr<u8>>(l, 0));
            }
            link = rd::<u32>(l, 0x10);
        }
    }
    special_heli_pre_render();
    render_extra_player_shadows();
}

/// `CRenderer::RenderRoads()` (0x553A10): the road buildings of the visible list (`RenderOneRoad`
/// inlined).
#[hook(addr = 0x553A10, conv = cdecl, name = "CRenderer::RenderRoads", expect = "A1 24 7B C9 00")]
pub fn render_roads() {
    rw_render_state_set(0xE, 1);
    rw_render_state_set(0xC, 1);
    rw_render_state_set(0x14, 2);
    de_activate_directional();
    set_ambient_colours();
    let mut i = 0i32;
    while i < NO_OF_VISIBLE_ENTITIES.get() {
        let e = g::<GPtr<u8>>(VISIBLE_ENTITY_PTRS).wrapping_index(i).read();
        if etype(e) == 1 && rd::<u8>(model_info_of(e), 0x13) & 1 != 0 {
            if is_vision_fx_active() != 0 {
                filter_fx_store_and_set_day_night_balance();
                entity_render(e);
                filter_fx_restore_day_night_balance();
            } else {
                entity_render(e);
            }
        }
        i += 1;
    }
}

/// `CRenderer::ms_vecCameraPosition` (CVector).
pub const CAMERA_POSITION: usize = 0xB76870;
/// `Scene.m_pRwCamera` (RwCamera*).
pub const SCENE_CAMERA: usize = 0xC1703C;

/// `sqrt` of the squared camera-to-entity components in the original's summation order:
/// `(dz*dz + dy*dy) + dx*dx` for the sorted list, `(dz*dz + dx*dx) + dy*dy` for the underwater list.
fn camera_distance(e: GPtr<u8>, underwater: bool) -> u32 {
    let m = rd::<GPtr<u8>>(e, 0x14);
    let pos = if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    let cam = g::<u8>(CAMERA_POSITION);
    let dx = x(rd::<f32>(cam, 0)).fsub_m(rd::<f32>(pos, 0));
    let dy = x(rd::<f32>(cam, 4)).fsub_m(rd::<f32>(pos, 4));
    let dz = x(rd::<f32>(cam, 8)).fsub_m(rd::<f32>(pos, 8));
    let zz = dz.fmul(dz);
    let sum = if underwater {
        let s = zz.fadd(dx.fmul(dx));
        s.fadd(dy.fmul(dy))
    } else {
        let s = zz.fadd(dy.fmul(dy));
        s.fadd(dx.fmul(dx))
    };
    sum.fsqrt().to_f32().to_bits()
}

/// `CRenderer::RenderEverythingBarRoads()` (0x553AA0). A translucent ped goes to the sorted alpha list
/// (gta-reversed renders it at once; the bytes insert it). The LODs are drawn with the camera's
/// z-shift moved by the `.data` float 0x8CD814 (-100.0).
#[hook(addr = 0x553AA0, conv = cdecl, name = "CRenderer::RenderEverythingBarRoads", expect = "51 A1 24 7B C9 00")]
pub fn render_everything_bar_roads() {
    rw_render_state_set(0xE, 1);
    rw_render_state_set(0xC, 1);
    rw_render_state_set(0x14, 2);
    if rd::<i32>(g(CURR_AREA), 0) == 0 {
        rw_render_state_set(0x1E, 0x8C); // rwRENDERSTATEALPHATESTFUNCTIONREF
    }
    let mut i = 0i32;
    while i < NO_OF_VISIBLE_ENTITIES.get() {
        let e = g::<GPtr<u8>>(VISIBLE_ENTITY_PTRS).wrapping_index(i).read();
        i += 1;
        let t = etype(e);
        if t == 1 && rd::<u8>(model_info_of(e), 0x13) & 1 != 0 {
            continue;
        }
        if t != 2 && (t != 3 || vp_get_clump_alpha(rd::<GPtr<u8>>(e, 0x18)) == 0xFF) {
            render_one_non_road_call(e);
            continue;
        }
        let underwater = if etype(e) != 2 {
            false
        } else if rd::<i32>(e, 0x590) != 5 {
            rd::<u32>(e, 0x40) & 0x800_0000 != 0
        } else {
            let cam = g::<u8>(crate::common::THE_CAMERA);
            let active = u32::from(rd::<u8>(cam, 0x59));
            let mode = rd::<u16>(g(0xB6F1A8 + (active * 0x238) as usize), 0);
            // Wheel cam, or first person not looking left / right: sorted list.
            let sorted =
                mode == 0xE || (mode == 0x10 && get_look_direction(cam) != 3 && get_look_direction(cam) != 0);
            !sorted && vp_get_clump_alpha(rd::<GPtr<u8>>(e, 0x18)) == 0xFF
        };
        let inserted = if underwater {
            vp_insert_entity_into_underwater_entities(e, camera_distance(e, true))
        } else {
            vp_insert_entity_into_sorted_list(e, camera_distance(e, false))
        };
        if inserted == 0 {
            render_one_non_road_call(e);
        }
    }
    let old = rd::<f32>(rd::<GPtr<u8>>(g(SCENE_CAMERA), 0), 0x90);
    rw_camera_end_update(rd::<GPtr<u8>>(g(SCENE_CAMERA), 0));
    let cam = rd::<GPtr<u8>>(g(SCENE_CAMERA), 0);
    let z = x(rd::<f32>(g(0x8CD814), 0)).fadd_m(rd::<f32>(cam, 0x90));
    wr::<f32>(cam, 0x90, z.to_f32());
    rw_camera_begin_update(rd::<GPtr<u8>>(g(SCENE_CAMERA), 0));
    let mut i = 0i32;
    while i < NO_OF_VISIBLE_LODS.get() {
        render_one_non_road_call(g::<GPtr<u8>>(VISIBLE_LOD_PTRS).wrapping_index(i).read());
        i += 1;
    }
    rw_camera_end_update(rd::<GPtr<u8>>(g(SCENE_CAMERA), 0));
    wr::<f32>(rd::<GPtr<u8>>(g(SCENE_CAMERA), 0), 0x90, quiet(old));
    rw_camera_begin_update(rd::<GPtr<u8>>(g(SCENE_CAMERA), 0));
}

/// `CRenderer::RenderFirstPersonVehicle()` (0x553D00). The alpha reference drops to 80 while the player
/// holds weapon type 28 (`m_aWeapons[m_nActiveWeaponSlot].m_eWeaponType`, +0x5A0 + slot * 0x1C).
#[hook(addr = 0x553D00, conv = cdecl, name = "CRenderer::RenderFirstPersonVehicle", expect = "A1 D4 45 B7 00")]
pub fn render_first_person_vehicle() {
    if rd::<u32>(g(FIRST_PERSON_VEHICLE), 0) == 0 {
        return;
    }
    let p = rd::<GPtr<u8>>(g(0xB7CD98), 0);
    let slot = i32::from(rd::<i8>(p, 0x718));
    let wt = rd::<i32>(p.byte_add(slot.wrapping_mul(0x1C) as isize), 0x5A0);
    let alpha_ref = wt == 0x1C;
    if alpha_ref {
        rw_render_state_set(0x1E, 0x50);
    }
    rw_render_state_set(0xE, 1);
    rw_render_state_set(0x8, 1);
    rw_render_state_set(0x6, 1);
    rw_render_state_set(0xC, 1);
    rw_render_state_set(0xA, 5);
    rw_render_state_set(0xB, 6);
    render_one_non_road_call(rd::<GPtr<u8>>(g(FIRST_PERSON_VEHICLE), 0));
    rw_render_state_set(0xE, 0);
    if alpha_ref {
        rw_render_state_set(0x1E, 0);
    }
}

/// `CRenderer::m_pFirstPersonVehicle` (CVehicle*).
pub const FIRST_PERSON_VEHICLE: usize = 0xB745D4;

/// `ST0` result of an original call (exact in f64 at PC=24) as an x87 value.
#[inline]
fn st0(v: f64) -> X87 {
    X87::from_f64_exact(v)
}

/// `CRenderer::SetupLightingForEntity(CPhysical*)` (0x553E40). The entry jumps into the protection chunk
/// 0x401FC0, which pushes `[0x1560700] + [0x4E9CA6]` = 0x3F800000 (1.0f) as the argument of
/// `CCarFXRenderer::SetFxEnvMapLightMult`; inlined. The original stores the light product into the
/// caller's argument slot (`fstp [esp+0x18]`) before passing it on; the slot is dead in every caller.
/// Not hooked: protection code reads the entry's jump displacement as data (`mov edi, [0x553E45]` at
/// 0x403C1B); its callers reach it through `orig!`.
pub fn setup_lighting_for_entity(e: GPtr<u8>) -> u8 {
    set_fx_env_map_light_mult(0x3F80_0000);
    if rd::<u32>(e, 0x40) & 0x2000_0000 != 0 {
        world_replace_normal_lights_with_scorched(rd::<GPtr<u8>>(g(0xC17038), 0), 0x3E38_51EC);
        return 0;
    }
    let pos = entity_pos(e);
    let mut v = [rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8)];
    let bright = e.byte_add(0x130);
    wr::<u32>(bright, 0, 0);
    let l = st0(generate_lights_affecting_object(
        GPtr::from_raw(v.as_mut_ptr()).cast(),
        bright.cast(),
        e,
    ))
    .to_f32();
    // 0x858624 (1.0) - 0x863E08 (0.05), stored as f32
    let d = x(k(0x3F80_0000)).fsub_m(k(0x3D4C_CCCD)).to_f32();
    let c = st0(get_lighting_from_col(e, 1));
    let mult = c.fmul_m(d).fadd_m(k(0x3D4C_CCCD)).fmul_m(l).to_f32().to_bits();
    set_fx_env_map_light_mult(mult);
    set_light_colours_for_peds_cars_and_objects(mult);
    1
}

/// `fcom` + `fnstsw` + `test ah, 5` + `jnp` taken: exactly one of C0/C2 set (`a < b`, ordered).
#[inline]
fn jnp5(a: X87, b: X87) -> bool {
    !a.fcom(b).test_parity(5)
}

/// Position of an entity: `m_matrix ? &m_matrix->pos : &m_placement.pos`.
#[inline]
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// The "not on screen or occluded" exit of the visibility functions: the model info's alpha is reset
/// to 255 unless bit 0 of `+0x12` is set, then that bit is cleared.
fn mark_model_offscreen(mi: GPtr<u8>) {
    let w = rd::<u16>(mi, 0x12);
    if w & 1 == 0 {
        wr::<u8>(mi, 0xC, 0xFF);
    }
    wr::<u16>(mi, 0x12, w & 0xFFFE);
}

/// `CRenderer::SetupMapEntityVisibility(CEntity*, CBaseModelInfo*, float dist, bool timeInRange)`
/// (0x553F60). Returns 0 invisible, 1 visible, 2 off screen, 3 streaming (eRendererVisibility). The
/// original keeps the fade and draw distances in the argument slots of `entity` / `mi` (f32 stores).
#[hook(addr = 0x553F60, conv = cdecl, name = "CRenderer::SetupMapEntityVisibility", expect = "55 56 8B 74 24 0C")]
pub fn setup_map_entity_visibility(e: GPtr<u8>, mi: GPtr<u8>, dist: f32, time_in_range: u8) -> i32 {
    let lod = rd::<GPtr<u8>>(e, 0x30);
    let col = rd::<GPtr<u8>>(mi, 0x14);
    let a = x(rd::<f32>(col, 0x24)).fadd_m(rd::<f32>(g(0xB76848), 0));
    let mi_rw = rd::<u32>(mi, 0x1C);
    let mut fade = k(0x41A0_0000); // 20.0
    let b = x(rd::<f32>(g(0xB6F118), 0)).fmul_m(rd::<f32>(mi, 0x18));
    let mut draw = if jp(b, a, 5) { a.to_f32() } else { b.to_f32() };
    let flags = rd::<u32>(e, 0x1C);
    if flags & 0x8000_0000 == 0 {
        if rd::<u8>(g(0xB745C0), 0) == 0 && flags & 0x4000_0000 != 0 {
            return 0;
        }
        if rd::<u8>(g(0xB745C1), 0) == 0 && flags & 0x4000_0000 == 0 {
            return 0;
        }
    }
    if lod.is_null() {
        let mut v = x(rd::<f32>(mi, 0x18));
        if !jnp5(v, x(draw)) {
            v = x(draw);
        }
        if !jne(v, x(k(0x4316_0000)), 0x41) {
            // v > 150.0: fade = v * 0.0666667 + 10.0
            fade = v.fmul_m(k(0x3D88_8889)).fadd_m(k(0x4120_0000)).to_f32();
        }
        if flags & 0x100 != 0 {
            draw = x(rd::<f32>(g(0x8CD804), 0)).fmul_m(draw).to_f32();
        }
    }
    // `t < draw` as `fcomp [draw]` + `test ah, 5` + `jp` not taken.
    let near = |t: X87| !jp(t, x(draw), 5);
    let faded = x(fade).fadd_m(dist).fsub_m(k(0x41A0_0000));
    if mi_rw == 0 {
        if !lod.is_null() && rd::<u8>(lod, 0x34) > 1 && near(faded) {
            add_to_lod_render_list_call(e, dist.to_bits());
            return 3;
        }
    } else if near(faded) {
        if rd::<u32>(e, 0x18) == 0 {
            entity_create_rw_object(e);
            if rd::<u32>(e, 0x18) == 0 {
                return 0;
            }
        }
        if rd::<u8>(e, 0x1C) & 0x80 == 0 {
            return 0;
        }
        if entity_get_is_on_screen(e) == 0 || entity_is_entity_occluded(e) != 0 {
            mark_model_offscreen(mi);
            return 2;
        }
        let mut f = rd::<u32>(e, 0x1C);
        if rd::<u8>(mi, 0xC) != 0xFF {
            f |= 0x8000;
        } else {
            f &= 0xFFFF_7FFF;
        }
        wr::<u32>(e, 0x1C, f);
        if lod.is_null() {
            return 1;
        }
        if rd::<u8>(mi, 0xC) == 0xFF {
            wr::<u8>(lod, 0x35, rd::<u8>(lod, 0x35).wrapping_add(1));
        }
        if rd::<u8>(lod, 0x34) > 1 {
            add_to_lod_render_list_call(e, dist.to_bits());
            return 0;
        }
        return 1;
    }
    // 0x55412D
    if flags & 0x8_0000 != 0 {
        return 0;
    }
    if mi_rw != 0 && near(x(dist).fsub_m(k(0x41A0_0000))) {
        if rd::<u32>(e, 0x18) == 0 {
            entity_create_rw_object(e);
            if rd::<u32>(e, 0x18) == 0 {
                return 0;
            }
        }
        if rd::<u8>(e, 0x1C) & 0x80 == 0 {
            return 0;
        }
        if entity_get_is_on_screen(e) == 0 || entity_is_entity_occluded(e) != 0 {
            mark_model_offscreen(mi);
            return 0;
        }
        wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) | 0x8000);
        let lod2 = rd::<GPtr<u8>>(e, 0x30);
        if !lod2.is_null() && rd::<u8>(lod2, 0x34) > 1 {
            add_to_lod_render_list_call(e, dist.to_bits());
        } else {
            call_add_entity_to_render_list(e, dist.to_bits());
        }
        return 0;
    }
    // 0x5541E6: within 50.0 of the draw distance, visible, time in range: ask for the model.
    if !near(x(dist).fsub_m(k(0x4248_0000))) {
        return 0;
    }
    if time_in_range == 0 || flags & 0x80 == 0 {
        return 0;
    }
    if rd::<u32>(e, 0x18) == 0 {
        entity_create_rw_object(e);
    }
    3
}

/// `|pos - ms_vecCameraPosition|` through `operator-` (0x40FE60) into a stack temporary and
/// `CVector::Magnitude` (0x4082C0), stored as f32.
fn distance_to_camera_call(pos: GPtr<u8>) -> f32 {
    let mut tmp = [0u32; 3];
    let t = vector_sub(GPtr::from_raw(tmp.as_mut_ptr()).cast(), pos, g(CAMERA_POSITION));
    st0(vector_magnitude(t)).to_f32()
}

/// `CRenderer::SetupEntityVisibility(CEntity*, float& outDistance)` (0x554230).
#[hook(addr = 0x554230, conv = cdecl, name = "CRenderer::SetupEntityVisibility", expect = "83 EC 14 56 8B 74 24 1C")]
pub fn setup_entity_visibility(e: GPtr<u8>, out: GPtr<f32>) -> i32 {
    let mi = model_info_of(e);
    let atomic = mi_as_atomic_model_info_ptr(mi);
    let mut time_in_range = 1u8;
    if etype(e) == 2 {
        let flags = rd::<u32>(e, 0x1C);
        if flags & 0x8000_0000 == 0 {
            let day = rd::<u8>(g(0xB745C0), 0) == 0 && flags & 0x4000_0000 != 0;
            if day || (rd::<u8>(g(0xB745C1), 0) == 0 && flags & 0x4000_0000 == 0) {
                return 0;
            }
        }
    }
    if atomic.is_null() {
        if mi_get_model_type(mi) != 5 && mi_get_model_type(mi) != 4 {
            return setup_entity_visibility_dynamic(e);
        }
    } else if mi_get_model_type(mi) == 3 {
        let ti = mi_get_time_info(mi);
        let other = i32::from(rd::<i16>(ti, 2));
        let off = u32::from(rd::<u8>(ti, 1));
        let on = u32::from(rd::<u8>(ti, 0));
        let other_loaded =
            |o: i32| rd::<u32>(g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(o).read(), 0x1C) != 0;
        if get_is_time_in_range(on, off) == 0 {
            if other == -1 || other_loaded(other) {
                entity_delete_rw_object(e);
                return 0;
            }
            time_in_range = 0;
        } else if other != -1 && other_loaded(other) {
            wr::<u8>(mi, 0xC, 0xFF);
        }
    } else {
        let flags = rd::<u32>(e, 0x1C);
        if flags & 0x8_0000 != 0 {
            if rd::<u32>(e, 0x18) == 0 {
                return 0;
            }
            if flags & 0x80 == 0 && (rd::<u32>(g(0xC7C724), 0) == 0 || rd::<u16>(e, 0x22) != 0) {
                return 0;
            }
            if entity_get_is_on_screen(e) == 0 || entity_is_entity_occluded(e) != 0 {
                return 2;
            }
            if rd::<u32>(e, 0x1C) & 0x4000 == 0 {
                return 1;
            }
            let d = distance_to_camera_call(entity_pos(e));
            vp_insert_entity_into_sorted_list(e, d.to_bits());
            wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) & 0xFFFF_7FFF);
            return 0;
        }
    }
    // 0x554476
    let area = rd::<u8>(e, 0x2F);
    if i32::from(area) != rd::<i32>(g(CURR_AREA), 0) && area != 13 {
        return 0;
    }
    let lod = rd::<GPtr<u8>>(e, 0x30);
    let pos = if lod.is_null() {
        entity_pos(e)
    } else {
        entity_pos(lod)
    };
    let cam = g::<u8>(CAMERA_POSITION);
    let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(cam, 0));
    let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(cam, 4));
    let dz = x(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(cam, 8));
    let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt().to_f32();
    out.write(quiet(d));
    if !jne(x(d), x(k(0x4396_0000)), 0x41) {
        // d > 300.0
        let l = x(rd::<f32>(g(0xB6F118), 0)).fmul_m(rd::<f32>(mi, 0x18)).to_f32();
        if !jp(x(k(0x4396_0000)), x(l), 5) && !jne(x(l).fadd_m(k(0x41A0_0000)), x(d), 0x41) {
            out.write(x(l).fsub_m(k(0x4396_0000)).fadd_m(d).to_f32());
        }
    }
    call_setup_map_entity_visibility(e, mi, out.read().to_bits(), time_in_range)
}

/// The dynamic-model path of `SetupEntityVisibility` (0x5542B5..0x554430): vehicles, peds and objects
/// without an atomic / clump-weapon model; the player's vehicle in first-person views is set aside as
/// `m_pFirstPersonVehicle`.
fn setup_entity_visibility_dynamic(e: GPtr<u8>) -> i32 {
    if find_player_vehicle(-1, 0) == e && rd::<u8>(g(0xB6EC20), 0) != 0 && rd::<u8>(g(0xA43088), 0) != 1 {
        let active = u32::from(rd::<u8>(g(0xB6F081), 0));
        let dir = rd::<i32>(g(0xB6F1C4 + (active * 0x238) as usize), 0);
        let leaning = rd::<i32>(find_player_vehicle(-1, 0), 0x590) == 9
            && rd::<u8>(find_player_vehicle(-1, 0), 0x614) & 0x80 != 0;
        if !leaning {
            let mid = rd::<u16>(e, 0x22);
            if dir == 3 || mid == 0x1B0 || mid == 0x1B5 || rd::<u8>(g(0xB6F04A), 0) != 0 {
                return 2;
            }
            let set = if dir != 0 {
                true
            } else if rd::<u32>(rd::<GPtr<u8>>(e, 0x384), 0xCC) & 0x4000 != 0 {
                return 2;
            } else if rd::<i32>(e, 0x590) != 5 {
                true
            } else {
                matches!(mid, 0x1C5 | 0x1C6 | 0x1AE | 0x1CC)
            };
            if set {
                wr::<GPtr<u8>>(g(FIRST_PERSON_VEHICLE), 0, e);
                return 2;
            }
        }
    }
    if rd::<u32>(e, 0x18) == 0 {
        return 0;
    }
    if rd::<u8>(e, 0x1C) & 0x80 == 0 && (rd::<u32>(g(0xC7C724), 0) == 0 || rd::<u16>(e, 0x22) != 0) {
        return 0;
    }
    if fun_407000(e) == 0 && etype(e) == 2 {
        return 0;
    }
    if entity_get_is_on_screen(e) == 0 || entity_is_entity_occluded(e) != 0 {
        return 2;
    }
    let f = rd::<u32>(e, 0x1C);
    if f & 0x4000 == 0 {
        return 1;
    }
    wr::<u32>(e, 0x1C, f & 0xFFFF_7FFF);
    let d = distance_to_camera_call(entity_pos(e));
    call_add_entity_to_render_list(e, d.to_bits());
    0
}

/// `sqrt((dz*dz + dy*dy) + dx*dx)` of `pos - ms_vecCameraPosition` (operands as the visibility
/// functions load them: the position minus the camera).
fn pos_minus_camera_distance(pos: GPtr<u8>) -> f32 {
    let cam = g::<u8>(CAMERA_POSITION);
    let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(cam, 0));
    let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(cam, 4));
    let dz = x(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(cam, 8));
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt().to_f32()
}

/// `CRenderer::SetupBigBuildingVisibility(CEntity*, float& outDistance)` (0x554650). The time-in-range
/// flag lives in the low byte of the `entity` argument slot, and that whole dword is what
/// `SetupMapEntityVisibility` receives; the callee reads only the byte, so the port passes the byte (its
/// hook declares the argument as `u8`, compared as one byte by L2).
#[hook(addr = 0x554650, conv = cdecl, name = "CRenderer::SetupBigBuildingVisibility", expect = "83 EC 10 8B 15 14 29 B7 00")]
pub fn setup_big_building_visibility(e: GPtr<u8>, out: GPtr<f32>) -> i32 {
    let mi = model_info_of(e);
    let mut time_in_range = 1u8;
    let area = rd::<u8>(e, 0x2F);
    if i32::from(area) != rd::<i32>(g(CURR_AREA), 0) && area != 13 {
        return 0;
    }
    if mi_get_model_type(mi) == 3 {
        let ti = mi_get_time_info(mi);
        let other = i32::from(rd::<i16>(ti, 2));
        let off = u32::from(rd::<u8>(ti, 1));
        let on = u32::from(rd::<u8>(ti, 0));
        let other_loaded =
            |o: i32| rd::<u32>(g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(o).read(), 0x1C) != 0;
        if get_is_time_in_range(on, off) == 0 {
            if other == -1 || other_loaded(other) {
                entity_delete_rw_object(e);
                return 0;
            }
            time_in_range = 0;
        } else if other != -1 && other_loaded(other) {
            wr::<u8>(mi, 0xC, 0xFF);
        }
    } else if mi_get_model_type(mi) == 6 {
        if entity_is_visible(e) == 0 || entity_is_entity_occluded(e) != 0 {
            return 0;
        }
        return 1;
    }
    let lod = rd::<GPtr<u8>>(e, 0x30);
    let pos = if lod.is_null() {
        entity_pos(e)
    } else {
        entity_pos(lod)
    };
    let d = pos_minus_camera_distance(pos);
    out.write(quiet(d));
    if rd::<i8>(e, 0x35) > 0 {
        if !lod.is_null() {
            wr::<u8>(lod, 0x35, rd::<u8>(lod, 0x35).wrapping_add(1));
        }
        if rd::<u8>(e, 0x34) > 1 {
            add_to_list(&LOD_RENDER_LIST, e, out.read());
        } else {
            wr::<u8>(e, 0x35, 0);
        }
        if rd::<u32>(e, 0x18) == 0 {
            entity_create_rw_object(e);
        }
        return 3;
    }
    let r = call_setup_map_entity_visibility(e, mi, d.to_bits(), time_in_range);
    if r == 1 && rd::<u8>(e, 0x34) > 1 {
        if !lod.is_null() && rd::<u8>(mi, 0xC) == 0xFF {
            wr::<u8>(lod, 0x35, rd::<u8>(lod, 0x35).wrapping_add(1));
        }
        add_to_lod_render_list_call(e, out.read().to_bits());
        return 0;
    }
    r
}

/// `CGeneral::LimitRadianAngle(float)` of `atan2(-dx, dy) - ms_fCameraHeading` (FPATAN in sa-math's f64
/// model, the difference rounded at PC=24 and stored as f32), made absolute (`fabs`).
fn heading_offset(dx: X87, dy: X87) -> X87 {
    let a = dx.fchs().fpatan(dy).fsub_m(rd::<f32>(g(0xB7684C), 0)).to_f32();
    let l = limit_radian_angle(a.to_bits());
    X87::from_f64_exact(l.abs())
}

/// `CRenderer::ScanSectorList(int32 x, int32 y)` (0x554840): sets up the visibility of every entity of
/// the sector's five scan lists; models of entities in front of the camera (or within 100 units) are
/// requested.
#[hook(addr = 0x554840, conv = cdecl, name = "CRenderer::ScanSectorList", expect = "83 EC 0C 53 55 56")]
pub fn scan_sector_list(sx: i32, sy: i32) {
    let dx = xi(sx.wrapping_sub(60))
        .fmul_m(k(0x4248_0000))
        .fadd_m(k(0x41C8_0000))
        .fsub_m(rd::<f32>(g(CAMERA_POSITION), 0));
    let dy = xi(sy.wrapping_sub(60))
        .fmul_m(k(0x4248_0000))
        .fadd_m(k(0x41C8_0000))
        .fsub_m(rd::<f32>(g(CAMERA_POSITION), 4));
    let sum = dy.fmul(dy).fadd(dx.fmul(dx));
    // The distance local lives for the whole call: a visibility result of 1 that did not write it
    // passes the previous entity's distance (the first one is uninitialised stack in the original).
    let mut dist = [0f32; 1];
    let near = if jp(sum, x(k(0x461C_4000)), 5) {
        // >= 10000.0: in front of the camera?
        !jp(heading_offset(dx, dy), x(k(0x3EB8_51EC)), 5)
    } else {
        true
    };
    call_setup_scan_lists(sx, sy);
    let lists = g::<u32>(SCAN_LISTS);
    for i in 0..5 {
        let list = lists.wrapping_index(i).read();
        if list == 0 {
            continue;
        }
        let mut node = rd::<GPtr<u8>>(GPtr::from_addr(list as usize), 0);
        while !node.is_null() {
            let e = rd::<GPtr<u8>>(node, 0);
            let code = rd::<u16>(g(CURRENT_SCAN_CODE), 0);
            node = rd::<GPtr<u8>>(node, 4);
            if rd::<u16>(e, 0x2C) == code {
                continue;
            }
            wr::<u16>(e, 0x2C, code);
            wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) & 0xFFFD_FFFF);
            let r = call_setup_entity_visibility(e, GPtr::from_raw(dist.as_mut_ptr()));
            match r {
                0 => {
                    if etype(e) != 4 {
                        continue;
                    }
                    let ami = mi_as_atomic_model_info_ptr(model_info_of(e));
                    if ami.is_null() {
                        continue;
                    }
                    let f = rd::<u16>(ami, 0x12) & 0x7800;
                    if f != 0x2000 && f != 0x2800 {
                        continue;
                    }
                    add_invisible_if_near(e);
                }
                1 => call_add_entity_to_render_list(e, dist[0].to_bits()),
                2 => add_invisible_if_near(e),
                3 => {
                    if rd::<u8>(g(0x9654B0), 0) != 0
                        || entity_get_is_on_screen(e) == 0
                        || rd::<u8>(g(0xB76851), 0) != 0
                    {
                        continue;
                    }
                    let mid = i32::from(rd::<i16>(e, 0x22));
                    let loaded =
                        rd::<u8>(g::<u8>(0x8E4CD0).byte_add((mid.wrapping_mul(20)) as isize), 0) == 1;
                    let request_now = if loaded {
                        near
                    } else if entity_is_entity_occluded(e) == 0 && near {
                        wr::<u8>(g(0xB76850), 0, 1);
                        true
                    } else {
                        false
                    };
                    if request_now || rd::<u8>(g(0xB76850), 0) == 0 || rd::<i32>(g(0x8E4CB8), 0) < 1 {
                        request_model(mid, 0);
                    }
                }
                _ => {}
            }
        }
    }
}

/// `ScanSectorList`'s cases 0 / 2 (0x554A1F): mark the entity off screen (`m_bOffscreen`, bit 17) and,
/// for entities with bit 21 set within 30 (200 for flagged vehicles) units of the camera on x and y,
/// append it to `ms_aInVisibleEntityPtrs` (at most 149).
fn add_invisible_if_near(e: GPtr<u8>) {
    let f = rd::<u32>(e, 0x1C) | 0x2_0000;
    wr::<u32>(e, 0x1C, f);
    if f & 0x20_0000 == 0 {
        return;
    }
    let pos = entity_pos(e);
    let dx = x(rd::<f32>(g(CAMERA_POSITION), 0)).fsub_m(rd::<f32>(pos, 0));
    let mut lim = k(0x41F0_0000); // 30.0
    if etype(e) == 2 && rd::<u8>(e, 0x42D) & 1 != 0 {
        lim = k(0x4348_0000); // 200.0
    }
    let neg = x(lim).fchs().to_f32();
    let inside = |d: X87| !jne(d, x(neg), 0x41) && !jp(d, x(lim), 5);
    if !inside(dx) {
        return;
    }
    let dy = x(rd::<f32>(g(CAMERA_POSITION), 4)).fsub_m(rd::<f32>(pos, 4));
    if !inside(dy) {
        return;
    }
    let n = NO_OF_INVISIBLE_ENTITIES.get();
    if n < 0x95 {
        g::<GPtr<u8>>(INVISIBLE_ENTITY_PTRS).wrapping_index(n).write(e);
        NO_OF_INVISIBLE_ENTITIES.set(n + 1);
    }
}

/// `CRenderer::ScanBigBuildingList(int32 x, int32 y)` (0x554B10): the LOD list of a 30x30 big-building
/// sector (`CWorld::ms_aLodPtrLists` 0xB99EB8).
#[hook(addr = 0x554B10, conv = cdecl, name = "CRenderer::ScanBigBuildingList", expect = "8B 4C 24 04 85 C9")]
pub fn scan_big_building_list(sx: i32, sy: i32) {
    if sx < 0 || sy < 0 || sx >= 30 || sy >= 30 {
        return;
    }
    let mut node = rd::<GPtr<u8>>(g(0xB99EB8), ((sy * 30 + sx) * 4) as isize);
    let dx = xi(sx - 15)
        .fmul_m(k(0x4348_0000))
        .fadd_m(k(0x42C8_0000))
        .fsub_m(rd::<f32>(g(CAMERA_POSITION), 0));
    let dy = xi(sy - 15)
        .fmul_m(k(0x4348_0000))
        .fadd_m(k(0x42C8_0000))
        .fsub_m(rd::<f32>(g(CAMERA_POSITION), 4));
    let sum = dy.fmul(dy).fadd(dx.fmul(dx));
    // The distance local lives for the whole call: a visibility result of 1 that did not write it
    // passes the previous entity's distance (the first one is uninitialised stack in the original).
    let mut dist = [0f32; 1];
    let near = if jp(sum, x(k(0x479C_4000)), 5) {
        // >= 80000.0: `test ah, 0x41` + `jp` on |angle| vs 0.7
        !jp(heading_offset(dx, dy), x(k(0x3F33_3333)), 0x41)
    } else {
        true
    };
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        let code = rd::<u16>(g(CURRENT_SCAN_CODE), 0);
        if rd::<u16>(e, 0x2C) != code {
            wr::<u16>(e, 0x2C, code);
            match call_setup_big_building_visibility(e, GPtr::from_raw(dist.as_mut_ptr())) {
                1 => {
                    let d = x(dist[0]).fadd_m(k(0x3C23_D70A)).to_f32();
                    call_add_entity_to_render_list(e, d.to_bits());
                    wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) & 0xFFFD_FFFF);
                }
                3 if rd::<u8>(g(0x9654B0), 0) == 0 && near => {
                    request_model(i32::from(rd::<i16>(e, 0x22)), 0);
                }
                _ => {}
            }
        }
        node = rd::<GPtr<u8>>(node, 4);
    }
}

/// `CRenderer::ShouldModelBeStreamed(CEntity*, const CVector& point, float farClip)` (0x554EB0): in the
/// current area (or 13), time in range, within `bound radius + farClip` and within the scaled LOD
/// distance. The distance stays unrounded on the x87 stack.
#[hook(addr = 0x554EB0, conv = cdecl, name = "CRenderer::ShouldModelBeStreamed", expect = "8B 15 14 29 B7 00")]
pub fn should_model_be_streamed(e: GPtr<u8>, point: GPtr<f32>, far_clip: f32) -> u8 {
    let area = rd::<u8>(e, 0x2F);
    if i32::from(area) != rd::<i32>(g(CURR_AREA), 0) && area != 13 {
        return 0;
    }
    let mi = model_info_of(e);
    let ti = mi_get_time_info(mi);
    if !ti.is_null() {
        let off = u32::from(rd::<u8>(ti, 1));
        let on = u32::from(rd::<u8>(ti, 0));
        if get_is_time_in_range(on, off) == 0 {
            return 0;
        }
    }
    let pos = entity_pos(e);
    let pt = point.cast::<u8>();
    let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(pt, 0));
    let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(pt, 4));
    let dz = x(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(pt, 8));
    let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    let col = rd::<GPtr<u8>>(model_info_of(e), 0x14);
    let r = x(rd::<f32>(col, 0x24)).fadd_m(far_clip);
    if !jne(d, r, 0x41) {
        return 0;
    }
    let l = x(rd::<f32>(g(0xB6F118), 0)).fmul_m(rd::<f32>(mi, 0x18));
    u8::from(!jne(l, d, 0x41))
}

/// `CRenderer::ScanPtrList_RequestModels<CPtrListSingleLink<CPhysical*>>(CPtrList&)` (0x555680). The
/// scan-code load runs in the protection chunk 0x401E0F (`ax = ms_nCurrentScanCode`); inlined.
#[hook(addr = 0x555680, conv = cdecl, name = "CRenderer::ScanPtrList_RequestModels<CPtrListSingleLink<CPhysical*>>", expect = "8B 44 24 04 57 8B 38")]
pub fn scan_ptr_list_request_models(list: GPtr<u8>) {
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        let code = rd::<u16>(g(CURRENT_SCAN_CODE), 0);
        node = rd::<GPtr<u8>>(node, 4);
        if rd::<u16>(e, 0x2C) == code {
            continue;
        }
        wr::<u16>(e, 0x2C, code);
        if call_should_model_be_streamed(e, g(CAMERA_POSITION), rd::<u32>(g(0xB76848), 0)) != 0 {
            request_model(i32::from(rd::<i16>(e, 0x22)), rd::<i32>(g(0xB745C4), 0));
        }
    }
}

/// `CRenderer::ScanSectorList_RequestModels(int32 x, int32 y)` (0x555900): the sector's building and
/// dummy lists and the repeat sector's object list.
#[hook(addr = 0x555900, conv = cdecl, name = "CRenderer::ScanSectorList_RequestModels", expect = "57 8B 7C 24 08")]
pub fn scan_sector_list_request_models(sx: i32, sy: i32) {
    if sx < 0 || sy < 0 || sx >= 120 || sy >= 120 {
        return;
    }
    let s = get_sector(sx, sy);
    call_scan_ptr_list_request_models(s);
    call_scan_ptr_list_request_models(s.byte_add(4));
    let idx = (((sy & 0xF) << 4) + (sx & 0xF)) as u32;
    call_scan_ptr_list_request_models(GPtr::from_addr((0xB992C0u32 + idx * 12) as usize));
}

/// `TheCamera`'s position: `m_matrix ? &m_matrix->pos : &m_placement.pos` (0xB6F03C / 0xB6F02C).
fn the_camera_pos() -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(g(0xB6F03C), 0);
    if m.is_null() {
        g(0xB6F02C)
    } else {
        m.byte_add(0x30)
    }
}

/// `CRenderer::ConstructRenderList()` (0x5556E0): tunnel flags, the LOD distance multiplier (high
/// camera above the player), mirrors and occluders, the camera position / heading / far plane, then
/// `ScanWorld`, `ProcessLodRenderLists` and a tail jump to `CStreaming::StartRenderEntities`.
#[hook(addr = 0x5556E0, conv = cdecl, name = "CRenderer::ConstructRenderList", expect = "51 A1 3C F0 B6 00")]
pub fn construct_render_list() {
    let cp = the_camera_pos();
    let attr = find_tunnel_attributes_for_coors(rd::<u32>(cp, 0), rd::<u32>(cp, 4), rd::<u32>(cp, 8));
    wr::<u8>(g(0xB745C0), 0, u8::from(attr & 0x880 != 0));
    let c1 = attr & 0x80 != 0 || attr & 0x800 == 0;
    wr::<u8>(g(0xB745C1), 0, u8::from(c1));
    wr::<u32>(g(0x8CD804), 0, 0x3F80_0000);
    wr::<u8>(g(0xB76851), 0, 0);
    let p = find_player_ped(-1);
    let mut mult: Option<X87> = None;
    if !p.is_null() && rd::<u8>(p, 0x2F) == 0 {
        let g1 = st0(calculate_ground_height(g(crate::common::THE_CAMERA), 0)).to_f32();
        let g2 = st0(calculate_ground_height(g(crate::common::THE_CAMERA), 2));
        let pp = entity_pos(p);
        if !jne(x(rd::<f32>(pp, 8)).fsub_m(g1), x(k(0x4248_0000)), 0x41) {
            let pp2 = entity_pos(p);
            let t = x(rd::<f32>(pp2, 8)).fsub(g2);
            if !jne(t, x(k(0x4120_0000)), 0x41) && !find_player_vehicle(-1, 0).is_null() {
                wr::<u8>(g(0xB76851), 0, 1);
            }
        }
    }
    let z = x(rd::<f32>(the_camera_pos(), 8));
    let lo = rd::<f32>(g(0x8CD808), 0);
    if !jne(z, x(lo), 0x41) {
        let mut t = z.fsub_m(lo).fdiv(x(rd::<f32>(g(0x8CD80C), 0)).fsub_m(lo));
        if !jne(t, x(k(0x3F80_0000)), 0x41) {
            t = x(k(0x3F80_0000));
        }
        let s = x(rd::<f32>(g(0x8CD810), 0)).fsub_m(k(0x3F80_0000));
        mult = Some(t.fmul(s).fadd_m(k(0x3F80_0000)));
    }
    let mult = mult.unwrap_or_else(|| x(rd::<f32>(g(0x8CD804), 0)));
    let v = x(rd::<f32>(g(0xB7C548), 0)).fmul(mult).to_f32();
    wr::<f32>(g(0x8CD804), 0, v);
    mirrors_before_construct_render_list();
    occlusion_process_before_rendering();
    NO_OF_VISIBLE_ENTITIES.set(0);
    NO_OF_VISIBLE_LODS.set(0);
    NO_OF_INVISIBLE_ENTITIES.set(0);
    let m = rd::<GPtr<u8>>(g(0xB6F03C), 0);
    let cp = the_camera_pos();
    let cam = g::<u32>(CAMERA_POSITION);
    cam.write(rd::<u32>(cp, 0));
    cam.wrapping_index(1).write(rd::<u32>(cp, 4));
    cam.wrapping_index(2).write(rd::<u32>(cp, 8));
    let heading = if m.is_null() {
        x(rd::<f32>(g(0xB6F038), 0))
    } else {
        x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)))
    };
    wr::<f32>(g(0xB7684C), 0, heading.to_f32());
    let far = rd::<f32>(rd::<GPtr<u8>>(g(0xB6F97C), 0), 0x84);
    LOD_RENDER_LIST.set(LOD_RENDER_LIST_BASE);
    wr::<f32>(g(0xB76848), 0, quiet(far));
    LOD_DONT_RENDER_LIST.set(LOD_DONT_RENDER_LIST_BASE);
    scan_world();
    call_process_lod_render_lists();
    start_render_entities();
}

/// `CRenderer::ScanWorld()` (0x554FE0): the camera-space frustum corners (apex, four far corners, and
/// eight points at 300 units, the second set with x/y scaled by 0.2) transformed by the camera frame's
/// matrix in one `RwV3dTransformPoints` call, then two `CWorldScan::ScanWorld` passes: the 120x120
/// sectors (`ScanSectorList`) over the 300-unit polygon and the 30x30 LOD sectors
/// (`ScanBigBuildingList`) over the far-plane polygon. Also sets `ms_bCameraIsStill`-style byte 0xB745C8
/// (moved < 4 units and turned < acos(0.98)) and the previous camera position / front.
#[hook(addr = 0x554FE0, conv = cdecl, name = "CRenderer::ScanWorld", expect = "A1 94 68 B7 00")]
pub fn scan_world_port() {
    // Guards of two function-local statics with trivial constructors.
    let mut guard = rd::<u32>(g(0xB76894), 0);
    if guard & 1 == 0 {
        guard |= 1;
        wr::<u32>(g(0xB76894), 0, guard);
    }
    if guard & 2 == 0 {
        guard |= 2;
        wr::<u32>(g(0xB76894), 0, guard);
    }
    let rwcam = rd::<GPtr<u8>>(g(0xB6F97C), 0);
    let far = x(rd::<f32>(rwcam, 0x84));
    let far32 = far.to_f32();
    let vwx = rd::<f32>(rwcam, 0x68);
    let vwy = rd::<f32>(rwcam, 0x6C);
    // 13 CVectors: v[0..5] the frustum (apex + far corners), v[5..13] the 300-unit points.
    let mut v = [0f32; 39];
    let fx = far.fmul_m(vwx);
    let fy = far.fmul_m(vwy);
    v[3] = fx.fchs().to_f32();
    v[4] = fy.to_f32();
    v[5] = far32;
    v[6] = fx.to_f32();
    v[7] = fy.to_f32();
    v[8] = far32;
    v[9] = fx.to_f32();
    v[10] = fy.fchs().to_f32();
    v[11] = far32;
    v[12] = fx.fchs().to_f32();
    v[13] = fy.fchs().to_f32();
    v[14] = far32;
    let matrix = rd::<GPtr<u8>>(rwcam, 4).byte_add(0x10);
    wr::<u32>(g(FIRST_PERSON_VEHICLE), 0, 0);
    vp_init_alpha_entity_list();
    let code = rd::<u16>(g(CURRENT_SCAN_CODE), 0);
    if code < 0xFFFF {
        wr::<u16>(g(CURRENT_SCAN_CODE), 0, code + 1);
    } else {
        clear_scan_codes();
        wr::<u16>(g(CURRENT_SCAN_CODE), 0, 1);
    }
    // Camera still?
    let cp = the_camera_pos();
    let dx = x(rd::<f32>(cp, 0)).fsub_m(rd::<f32>(g(0xB76888), 0));
    let dy = x(rd::<f32>(cp, 4)).fsub_m(rd::<f32>(g(0xB7688C), 0));
    let dz = x(rd::<f32>(cp, 8)).fsub_m(rd::<f32>(g(0xB76890), 0));
    let moved = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    let still = if jp(moved, x(k(0x4180_0000)), 5) {
        false
    } else {
        let dot = x(rd::<f32>(g(0xB6F9B4), 0))
            .fmul_m(rd::<f32>(g(0xB76884), 0))
            .fadd(x(rd::<f32>(g(0xB6F9AC), 0)).fmul_m(rd::<f32>(g(0xB7687C), 0)))
            .fadd(x(rd::<f32>(g(0xB6F9B0), 0)).fmul_m(rd::<f32>(g(0xB76880), 0)));
        !jne(dot, x(k(0x3F7A_E148)), 0x41)
    };
    wr::<u8>(g(0xB745C8), 0, u8::from(still));
    let cp = the_camera_pos();
    for i in 0..3 {
        wr::<u32>(g(0xB76888), 4 * i, rd::<u32>(cp, 4 * i));
        wr::<u32>(g(0xB7687C), 4 * i, rd::<u32>(g(0xB6F9AC), 4 * i));
    }
    // The 300-unit points: corner * 300 / far.
    let k300 = k(0x4396_0000);
    let t = x(k(0x3F80_0000)).fdiv_m(far32);
    let s1 = [v[3], v[4], v[5]].map(|c| x(x(c).fmul_m(k300).to_f32()).fmul(t).to_f32());
    let t32 = t.to_f32();
    let corner = |a: usize| {
        let (p0, p1) = (x(v[a]).fmul_m(k300), x(v[a + 1]).fmul_m(k300));
        let p2 = x(v[a + 2]).fmul_m(k300).to_f32();
        [
            p0.fmul_m(t32).to_f32(),
            p1.fmul_m(t32).to_f32(),
            x(p2).fmul_m(t32).to_f32(),
        ]
    };
    let s2 = corner(6);
    let s3 = corner(9);
    // The last corner keeps only x on the stack (y and z go through `fstp dword`).
    let s4 = {
        let a = x(v[12]).fmul_m(k300);
        let b = x(v[13]).fmul_m(k300).to_f32();
        let c = x(v[14]).fmul_m(k300).to_f32();
        [
            a.fmul_m(t32).to_f32(),
            x(b).fmul_m(t32).to_f32(),
            x(c).fmul_m(t32).to_f32(),
        ]
    };
    let k02 = k(0x3E4C_CCCD);
    let scaled = |s: [f32; 3]| [x(s[0]).fmul_m(k02).to_f32(), x(s[1]).fmul_m(k02).to_f32(), s[2]];
    for (i, s) in [s1, s2, scaled(s1), scaled(s2), s3, s4, scaled(s3), scaled(s4)]
        .iter()
        .enumerate()
    {
        v[15 + 3 * i..18 + 3 * i].copy_from_slice(s);
    }
    let vp = GPtr::<f32>::from_raw(v.as_mut_ptr());
    rw_v3d_transform_points(vp, vp, 13, matrix);
    // Sector polygon (120x120 grid of 50 units): apex and the four 300-unit points.
    let to_sector = |c: f32| x(c).fmul_m(k(0x3CA3_D70A)).fadd_m(k(0x4270_0000)).to_f32();
    let mut pts = [0f32; 10];
    for (i, idx) in [0usize, 5, 6, 9, 10].iter().enumerate() {
        pts[2 * i] = to_sector(v[3 * idx]);
        pts[2 * i + 1] = to_sector(v[3 * idx + 1]);
    }
    wr::<u8>(g(0xB76850), 0, 0);
    world_scan(GPtr::from_raw(pts.as_mut_ptr()), 5, 0x554840);
    // LOD polygon (30x30 grid of 200 units): apex and the four far corners.
    let to_lod = |c: f32| x(c).fmul_m(k(0x3BA3_D70A)).fadd_m(k(0x4170_0000)).to_f32();
    for i in 0..5 {
        pts[2 * i] = to_lod(v[3 * i]);
        pts[2 * i + 1] = to_lod(v[3 * i + 1]);
    }
    world_scan(GPtr::from_raw(pts.as_mut_ptr()), 5, 0x554B10);
}

/// `CRenderer::GetObjectsInFrustum(CEntity** out, float farPlane, RwMatrix* transform)` (0x554C60): the
/// entities of the sectors under the triangle (apex, two far corners: the top ones, or the bottom ones
/// when `transform->at.z > 0`) are written to `out`; with no transform the camera frame's matrix is
/// used and only visible entities are listed. Returns the count (`sar` of the byte difference).
#[hook(addr = 0x554C60, conv = cdecl, name = "CRenderer::GetObjectsInFrustum", expect = "81 EC BC 00 00 00")]
pub fn get_objects_in_frustum(out: GPtr<u8>, far_plane: f32, transform: GPtr<u8>) -> i32 {
    let rwcam = rd::<GPtr<u8>>(g(0xB6F97C), 0);
    let far = x(far_plane);
    let fx = far.fmul_m(rd::<f32>(rwcam, 0x68));
    let fy = far.fmul_m(rd::<f32>(rwcam, 0x6C));
    let far32 = far_plane;
    // 13 CVectors (only the first five are set; the rest stay zero, as the `rep stos` leaves them).
    let mut v = [0f32; 39];
    v[3..6].copy_from_slice(&[fx.fchs().to_f32(), fy.to_f32(), far32]);
    v[6..9].copy_from_slice(&[fx.to_f32(), fy.to_f32(), far32]);
    v[9..12].copy_from_slice(&[fx.to_f32(), fy.fchs().to_f32(), far32]);
    v[12..15].copy_from_slice(&[fx.fchs().to_f32(), fy.fchs().to_f32(), far32]);
    let code = rd::<u16>(g(CURRENT_SCAN_CODE), 0);
    if code < 0xFFFF {
        wr::<u16>(g(CURRENT_SCAN_CODE), 0, code + 1);
    } else {
        clear_scan_codes();
        wr::<u16>(g(CURRENT_SCAN_CODE), 0, 1);
    }
    let m = if transform.is_null() {
        rd::<GPtr<u8>>(rd::<GPtr<u8>>(g(0xB6F97C), 0), 4).byte_add(0x10)
    } else {
        transform
    };
    let cp = the_camera_pos();
    for i in 0..3 {
        wr::<u32>(g(CAMERA_POSITION), 4 * i, rd::<u32>(cp, 4 * i));
    }
    wr::<u32>(g(0xB76848), 0, 0x4396_0000); // 300.0
    if !jne(x(rd::<f32>(m, 0x28)), x(k(0)), 0x41) {
        // Looking down: the bottom corners (v1 := v4, v2 := v3).
        let (v3, v4) = ([v[9], v[10], v[11]], [v[12], v[13], v[14]]);
        v[3..6].copy_from_slice(&v4);
        v[6..9].copy_from_slice(&v3);
    }
    let vp = GPtr::<f32>::from_raw(v.as_mut_ptr());
    rw_v3d_transform_points(vp, vp, 13, m);
    let to_sector = |c: f32| x(c).fmul_m(k(0x3CA3_D70A)).fadd_m(k(0x4270_0000)).to_f32();
    let mut pts = [0f32; 6];
    for i in 0..3 {
        pts[2 * i] = to_sector(v[3 * i]);
        pts[2 * i + 1] = to_sector(v[3 * i + 1]);
    }
    OUT_ENTITIES.set(out.addr() as u32);
    let f = if transform.is_null() { 0x553650 } else { 0x5535D0 };
    world_scan(GPtr::from_raw(pts.as_mut_ptr()), 3, f);
    (OUT_ENTITIES.get().wrapping_sub(out.addr() as u32) as i32) >> 2
}

/// `CRenderer::RequestObjectsInFrustum(RwMatrix* transform, int32 streamingFlags)` (0x555960): the
/// models of the sectors under the triangle (apex and two corners at 300 units: the top ones, or the
/// bottom ones when `transform->at.z > 0` or NaN) are requested with `streamingFlags`. The camera
/// position becomes the transform's position.
#[hook(addr = 0x555960, conv = cdecl, name = "CRenderer::RequestObjectsInFrustum", expect = "81 EC D8 00 00 00")]
pub fn request_objects_in_frustum(transform: GPtr<u8>, flags: i32) {
    let rwcam = rd::<GPtr<u8>>(g(0xB6F97C), 0);
    let far32 = x(rd::<f32>(rwcam, 0x84)).to_f32();
    let far = x(far32);
    let fx = far.fmul_m(rd::<f32>(rwcam, 0x68));
    let fy = far.fmul_m(rd::<f32>(rwcam, 0x6C));
    let mut v = [0f32; 39];
    v[3..6].copy_from_slice(&[fx.fchs().to_f32(), fy.to_f32(), far32]);
    v[6..9].copy_from_slice(&[fx.to_f32(), fy.to_f32(), far32]);
    v[9..12].copy_from_slice(&[fx.to_f32(), fy.fchs().to_f32(), far32]);
    v[12..15].copy_from_slice(&[fx.fchs().to_f32(), fy.fchs().to_f32(), far32]);
    let code = rd::<u16>(g(CURRENT_SCAN_CODE), 0);
    let mut rwcam = rwcam;
    if code < 0xFFFF {
        wr::<u16>(g(CURRENT_SCAN_CODE), 0, code + 1);
    } else {
        clear_scan_codes();
        rwcam = rd::<GPtr<u8>>(g(0xB6F97C), 0);
        wr::<u16>(g(CURRENT_SCAN_CODE), 0, 1);
    }
    let m = if transform.is_null() {
        rd::<GPtr<u8>>(rwcam, 4).byte_add(0x10)
    } else {
        transform
    };
    for i in 0..3 {
        wr::<u32>(g(CAMERA_POSITION), 4 * i, rd::<u32>(m, 0x30 + 4 * i));
    }
    wr::<u32>(g(0xB76848), 0, 0x4396_0000); // 300.0
    wr::<i32>(g(0xB745C4), 0, flags);
    let k300 = k(0x4396_0000);
    // `test ah, 0x41` + `jp`: at.z > 0 (or NaN) takes the bottom corners.
    let (first, second) = if jp(x(rd::<f32>(m, 0x28)), x(k(0)), 0x41) {
        (12, 9)
    } else {
        (3, 6)
    };
    let t = x(k(0x3F80_0000)).fdiv_m(far32);
    for c in 0..3 {
        let a = x(v[first + c]).fmul_m(k300).to_f32();
        v[15 + c] = x(a).fmul(t).to_f32();
    }
    let t32 = t.to_f32();
    let p = x(v[second]).fmul_m(k300);
    let q = x(v[second + 1]).fmul_m(k300).to_f32();
    let r = x(v[second + 2]).fmul_m(k300).to_f32();
    v[18] = p.fmul_m(t32).to_f32();
    v[19] = x(q).fmul_m(t32).to_f32();
    v[20] = x(r).fmul_m(t32).to_f32();
    let vp = GPtr::<f32>::from_raw(v.as_mut_ptr());
    rw_v3d_transform_points(vp, vp, 13, m);
    let to_sector = |c: f32| x(c).fmul_m(k(0x3CA3_D70A)).fadd_m(k(0x4270_0000)).to_f32();
    let mut pts = [0f32; 6];
    for (i, idx) in [0usize, 5, 6].iter().enumerate() {
        pts[2 * i] = to_sector(v[3 * idx]);
        pts[2 * i + 1] = to_sector(v[3 * idx + 1]);
    }
    world_scan(GPtr::from_raw(pts.as_mut_ptr()), 3, 0x555900);
}

orig! {
    /// `RwMatrixRotate(RwMatrix*, const RwV3d* axis, float angle, RwOpCombineType)` (0x7F1FD0); the
    /// angle as a raw dword
    pub fn rw_matrix_rotate(m: GPtr<f32>, axis: GPtr<f32>, angle: u32, combine: u32) -> GPtr<f32> = cdecl 0x7F1FD0 => unported;
    /// `RwMatrixTranslate(RwMatrix*, const RwV3d*, RwOpCombineType)` (0x7F2450)
    pub fn rw_matrix_translate(m: GPtr<f32>, v: GPtr<u8>, combine: u32) -> GPtr<f32> = cdecl 0x7F2450 => unported;
    /// `CRenderer::RequestObjectsInFrustum(RwMatrix*, int32)` (0x555960)
    pub fn call_request_objects_in_frustum(m: GPtr<f32>, flags: i32) = cdecl 0x555960 => request_objects_in_frustum_f32;
}

/// `request_objects_in_frustum` with an `f32` matrix pointer (the phase B/C path of the `orig!` call).
pub fn request_objects_in_frustum_f32(m: GPtr<f32>, flags: i32) {
    request_objects_in_frustum(m.cast(), flags);
}

/// `CRenderer::RequestObjectsInDirection(const CVector& posn, float angle)` (0x555CB0): an identity
/// stack RwMatrix rotated 90 degrees about x (`rwCOMBINEREPLACE`), then by `angle * 57.295776 + 180`
/// degrees about z (`rwCOMBINEPOSTCONCAT`), translated to `posn` (post-concat), and passed to
/// `RequestObjectsInFrustum` with the constant streaming flags 0x20 (a third argument, if any caller
/// passes one, is not read). The original's matrix flags word is uninitialised stack ORed with 0x20003
/// and its three pad words are uninitialised; the port uses 0x20003 and zero pads (stack only; see
/// render-port.md).
#[hook(addr = 0x555CB0, conv = cdecl, name = "CRenderer::RequestObjectsInDirection", expect = "83 EC 58 8B 4C 24 24")]
pub fn request_objects_in_direction(posn: GPtr<u8>, angle: f32) {
    let one = 1.0f32;
    let mut axes = [one, 0.0, 0.0, 0.0, 0.0, one];
    let mut m = [
        one,
        0.0,
        0.0,
        f32::from_bits(0x20003),
        0.0,
        one,
        0.0,
        0.0,
        0.0,
        0.0,
        one,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ];
    let mp = GPtr::<f32>::from_raw(m.as_mut_ptr());
    let ap = GPtr::<f32>::from_raw(axes.as_mut_ptr());
    rw_matrix_rotate(mp, ap, 0x42B4_0000, 0);
    let deg = x(angle).fmul_m(k(0x4265_2EE0)).fadd_m(k(0x4334_0000)).to_f32();
    rw_matrix_rotate(mp, ap.wrapping_index(3), deg.to_bits(), 2);
    rw_matrix_translate(mp, posn, 2);
    call_request_objects_in_frustum(mp, 0x20);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_RENDER_FADING_IN_ENTITIES,
    HOOK_RENDER_FADING_IN_UNDERWATER_ENTITIES,
    HOOK_RENDER_ONE_ROAD,
    HOOK_RENDER_ONE_NON_ROAD,
    HOOK_REMOVE_VEHICLE_PED_LIGHTS,
    HOOK_ADD_ENTITY_TO_RENDER_LIST,
    HOOK_SETUP_SCAN_LISTS,
    HOOK_SCAN_SECTOR_LIST_LIST_MODELS,
    HOOK_SCAN_SECTOR_LIST_LIST_MODELS_VISIBLE,
    HOOK_GET_LOD_RENDER_LIST_BASE,
    HOOK_GET_LOD_DONT_RENDER_LIST_BASE,
    HOOK_RESET_LOD_RENDER_LISTS,
    HOOK_ADD_TO_LOD_RENDER_LIST,
    HOOK_ADD_TO_LOD_DONT_RENDER_LIST,
    HOOK_PROCESS_LOD_RENDER_LISTS,
    HOOK_PRE_RENDER,
    HOOK_RENDER_ROADS,
    HOOK_RENDER_EVERYTHING_BAR_ROADS,
    HOOK_RENDER_FIRST_PERSON_VEHICLE,
    HOOK_SETUP_MAP_ENTITY_VISIBILITY,
    HOOK_SETUP_ENTITY_VISIBILITY,
    HOOK_SETUP_BIG_BUILDING_VISIBILITY,
    HOOK_SCAN_SECTOR_LIST,
    HOOK_SCAN_BIG_BUILDING_LIST,
    HOOK_SHOULD_MODEL_BE_STREAMED,
    HOOK_SCAN_PTR_LIST_REQUEST_MODELS,
    HOOK_SCAN_SECTOR_LIST_REQUEST_MODELS,
    HOOK_CONSTRUCT_RENDER_LIST,
    HOOK_SCAN_WORLD_PORT,
    HOOK_GET_OBJECTS_IN_FRUSTUM,
    HOOK_REQUEST_OBJECTS_IN_FRUSTUM,
    HOOK_REQUEST_OBJECTS_IN_DIRECTION,
];
