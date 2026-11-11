//! CObject / CDummyObject / CHandObject / CCutsceneObject / CObjectData / pool new-delete leaves
//! (PORT-WORLD-2, helper A). Ported from the 1.0 US bytes (gta-reversed `Object.cpp`,
//! `CutsceneObject.cpp` as reading aids).
//!
//! `CObject` (pool slot 0x19C, the size of `CCutsceneObject`): `CEntity` +0x1C flags (bit 8
//! `bIsBIGBuilding`, bit 28 `bLightObject`), `CPhysical` +0x40 physical flags (bit 5, bit 29
//! `bRenderScorched`, bit 30), +0xB0 the entry-info list `IsObjectPointerValid` tests, +0x13C object
//! type (`eObjectType`), +0x140 object flags (bit 9 `bCanBeTargetted`), +0x14A ref model index,
//! +0x158 door start angle, +0x16A remap txd, +0x16C remap texture.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{g, rd, wr, x};
use super::entity::{
    deactivate_directional, matrix_heading, pool_index, remove_lights_affecting_object, set_ambient_colours,
    MODEL_INFO_PTRS, OBJECT_POOL,
};

sa_core::globals! {
    list GLOBALS;
}

/// `CObject` pool slot size
const OBJECT_SLOT: i32 = 0x19C;
/// `Scene.m_pRpWorld`
const SCENE_WORLD: usize = 0xC17038;
/// 0x859948: -1000.0
const FM1000: f32 = -1000.0;
/// 0x858B50: 0.0
const F0: f32 = 0.0;
/// 0x8595F0: 1.25; 0x858F34: 0.75; 0x858C20: 0.9
const F1_25: f32 = 1.25;
const F0_75: f32 = 0.75;
const F0_9: f32 = f32::from_bits(0x3F66_6666);
/// `CRopes::ms_aRopes` (`CRope[8]`, 0x328 each): +0x318 carried entity, +0x31C height
const ROPES: usize = 0xB768B8;
/// `CWorld::ms_listObjectsWithControlCode` (`CPtrListDoubleLink` head)
const CONTROL_CODE_LIST: usize = 0xB9ACCC;
/// `CObjectData::ms_aObjectInfo` (0x50 each; slot 0 is the default)
const OBJECT_INFO: usize = 0xBB4A90;
/// 0x863C08: 99999.0; 0x858C98: 0.8
const F99999: f32 = 99999.0;
const F0_8: f32 = f32::from_bits(0x3F4C_CCCD);
/// 0x858CC4: 0.2
const F0_2: f32 = f32::from_bits(0x3E4C_CCCD);
/// `ClumpOffset`: the anim blend clump plugin's offset in `RpClump`
const CLUMP_ANIM_OFFSET: usize = 0xB5F878;
/// `ModelIndices::MI_FIRE_HYDRANT`
const MI_FIRE_HYDRANT: usize = 0x8CD594;

orig! {
    /// `CStreaming::SetMissionDoesntRequireModel(int32)` (0x409C90)
    pub fn set_mission_doesnt_require_model(m: i32) = cdecl 0x409C90 => unported;
    /// `CTheScripts::RemoveFromWaitingForScriptBrainArray(CEntity*, int16)` (0x46ABC0)
    pub fn remove_from_waiting_for_script_brain_array(e: GPtr<u8>, i: u32) = cdecl 0x46ABC0 => unported;
    /// `CColStore::RemoveRef(int32)` (0x4107D0)
    pub fn col_store_remove_ref(i: i32) = cdecl 0x4107D0 => unported;
    /// `CRadar::ClearBlipForEntity(eBlipType, int32 handle)` (0x587C60)
    pub fn radar_clear_blip_for_entity(ty: i32, h: i32) = cdecl 0x587C60 => unported;
    /// `CBaseModelInfo::RemoveRef()` (0x4C4BB0)
    pub fn model_info_remove_ref(mi: GPtr<u8>) = thiscall 0x4C4BB0 => unported;
    /// `CTxdStore::RemoveRef(int32)` (0x731A30)
    pub fn txd_store_remove_ref(i: i32) = cdecl 0x731A30 => unported;
    /// `CObject::RemoveFromControlCodeList()` (0x59F450)
    pub fn remove_from_control_code_list_call(o: GPtr<u8>) = thiscall 0x59F450 => remove_from_control_code_list;
    /// `CPathFind::SetLinksBridgeLights(float xMin, float xMax, float yMin, float yMax, bool)` (0x44D960)
    pub fn path_set_links_bridge_lights(pf: GPtr<u8>, x0: f32, x1: f32, y0: f32, y1: f32, b: u32) = thiscall 0x44D960 => unported;
    /// `CFire::Extinguish()` (0x5393F0)
    pub fn fire_extinguish(f: GPtr<u8>) = thiscall 0x5393F0 => unported;
    /// `CPhysical::~CPhysical()` (0x542450): EAX of `CEntity::~CEntity`
    pub fn physical_destructor(p: GPtr<u8>) -> u32 = thiscall 0x542450 => crate::physical::base::destructor;
    /// `CColStore::AddRef(int32)` (0x4107A0)
    pub fn col_store_add_ref(i: i32) = cdecl 0x4107A0 => unported;
    /// `CBaseModelInfo::SwaysInWind()` (0x4212C0)
    pub fn model_info_sways_in_wind(mi: GPtr<u8>) -> u32 = thiscall 0x4212C0 => unported;
    /// `CGarages::IsModelIndexADoor(int32)` (0x448AF0)
    pub fn garages_is_model_index_a_door(m: i32) -> u8 = cdecl 0x448AF0 => unported;
    /// `CPlaceable::SetMatrix(const CMatrix&)` (0x54F610)
    pub fn placeable_set_matrix(p: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F610 => unported;
    /// `CCustomBuildingRenderer::IsCBPCPipelineAttached(RpAtomic*)` (0x5D7F40)
    pub fn is_cbpc_pipeline_attached(a: GPtr<u8>) -> u32 = cdecl 0x5D7F40 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120)
    pub fn rp_h_anim_hierarchy_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    pub fn matrix_assign(m: GPtr<u8>, o: GPtr<u8>) = thiscall 0x59BBC0 => unported;
    /// `CPhysical::RemoveAndAdd()` (0x542560)
    pub fn physical_remove_and_add(p: GPtr<u8>) = thiscall 0x542560 => unported;
    /// `CObject::CObject(CDummyObject*)` (0x5A1DF0)
    pub fn object_ctor_dummy(o: GPtr<u8>, d: GPtr<u8>) -> GPtr<u8> = thiscall 0x5A1DF0 => unported;
    /// `WaterCreatureManager_c::TryToFreeUpWaterCreatures(int32)` (0x6E40E0)
    pub fn water_creatures_try_to_free_up(m: GPtr<u8>, n: i32) = thiscall 0x6E40E0 => unported;
    /// `CRopes::FindPickupHeight(CEntity*)` (0x556760, ST0)
    pub fn ropes_find_pickup_height(e: GPtr<u8>) -> f64 = cdecl 0x556760 => unported;
    /// `CIplStore::IncludeEntity(int32, CEntity*)` (0x404C90)
    pub fn ipl_store_include_entity(i: i32, e: GPtr<u8>) = cdecl 0x404C90 => unported;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    pub fn key_gen_get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => unported;
    /// `RpAnimBlendClumpInit(RpClump*)` (0x4D6720)
    pub fn rp_anim_blend_clump_init(c: GPtr<u8>) = cdecl 0x4D6720 => unported;
    /// `CObjectSaveStructure::Construct(CObject*)` (0x5D2160)
    pub fn save_structure_construct(s: GPtr<u8>, o: GPtr<u8>) = thiscall 0x5D2160 => unported;
    /// `CObjectSaveStructure::Extract(CObject*)` (0x5D2220)
    pub fn save_structure_extract(s: GPtr<u8>, o: GPtr<u8>) = thiscall 0x5D2220 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    pub fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CEntity::AttachToRwObject(RwObject*, bool)` (0x533ED0)
    pub fn entity_attach_to_rw_object(e: GPtr<u8>, rw: GPtr<u8>, update: u32) = thiscall 0x533ED0 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// scalar deleting destructor (vtable slot 0)
    pub fn entity_vdelete(e: GPtr<u8>, flag: i32) = thiscall vslot 0x0 => unported;
    /// `CEntity::SetIsStatic(bool)` (vtable slot 0x10)
    pub fn entity_vset_is_static(e: GPtr<u8>, v: u8) = thiscall vslot 0x10 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40)
    pub fn simple_transform_update_rw_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CMatrix::UpdateRW(RwMatrix*)` (0x59AD70)
    pub fn matrix_update_rw(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x7)` (0x5674E0)
    pub fn world_process_vertical_line(p: GPtr<u8>, z: f32, cp: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, f: u32, h: u32, i: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `CPhysical::CPhysical` (0x542260)
    pub fn physical_ctor(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x542260 => unported;
    /// `CObject::Init()` (0x59F840)
    pub fn object_init(o: GPtr<u8>) = thiscall 0x59F840 => unported;
    /// `0x5A2CB0` (unnamed, thiscall, `ret 0xC`): `SetObjectData`'s centre-of-mass setter `(0, 0, z)`
    pub fn object_set_centre_of_mass(o: GPtr<u8>, cx: u32, cy: u32, cz: f32) = thiscall 0x5A2CB0 => unported;
    /// `CObject::Explode()` (0x5A1340)
    pub fn object_explode(o: GPtr<u8>) = thiscall 0x5A1340 => unported;
    /// `CObject::~CObject` (0x59F660)
    pub fn object_dtor(o: GPtr<u8>) = thiscall 0x59F660 => unported;
    /// `CRopes::FindRope(uint32 id)` (0x556000)
    pub fn find_rope(id: u32) -> i32 = cdecl 0x556000 => unported;
    /// `CRope::ReleasePickedUpObject()` (0x556030)
    pub fn rope_release_picked_up_object(r: GPtr<u8>) = thiscall 0x556030 => unported;
    /// `CObject::ResetDoorAngle()` (0x59F4B0)
    pub fn object_reset_door_angle(o: GPtr<u8>) = thiscall 0x59F4B0 => unported;
    /// `CPool<CObject>::CreateAtRef(int32 handle)` (0x59F610)
    pub fn object_pool_create_at_ref(pool: GPtr<u8>, handle: i32) = thiscall 0x59F610 => unported;
    /// `CBaseModelInfo::AsAtomicModelInfoPtr` (vtable slot 4)
    pub fn model_info_as_atomic(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CEntity::CreateRwObject()` (0x533D30)
    pub fn entity_create_rw_object(e: GPtr<u8>) = thiscall 0x533D30 => unported;
    /// `CObjectPool::New` (0x5A1C20, thiscall on the pool)
    pub fn object_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x5A1C20 => unported;
    /// `Light::WorldReplaceNormalLightsWithScorched(RpWorld*, float)` (0x7357E0)
    pub fn world_replace_normal_lights_with_scorched(w: GPtr<u8>, l: f32) = cdecl 0x7357E0 => unported;
    /// `Light::ActivateDirectional()` (0x735C80)
    pub fn activate_directional() = cdecl 0x735C80 => unported;
    /// `CRenderer::SetupLightingForEntity(CEntity*)` (0x553E40)
    pub fn setup_lighting_for_entity(e: GPtr<u8>) -> u8 = cdecl 0x553E40 => unported;
    /// `CBaseModelInfo::AddRef` (0x4C4BA0)
    pub fn model_info_add_ref(mi: GPtr<u8>) = thiscall 0x4C4BA0 => unported;
    /// `CTxdStore::AddRef(int32)` (0x731A00)
    pub fn txd_store_add_ref(i: i32) = cdecl 0x731A00 => unported;
    /// `CVector::CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730)
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
}

/// `CObject::operator new(uint32)` (0x5A1EE0): `ms_pObjectPool->New()` (tail jump, size ignored).
#[hook(addr = 0x5A1EE0, conv = cdecl, name = "new", expect = "8B 0D 9C 44 B7 00")]
pub fn object_new(_size: u32) -> GPtr<u8> {
    object_pool_new(g::<GPtr<u8>>(OBJECT_POOL).read())
}

/// `CObject::operator delete(void*)` (0x5A1F20): `ms_pObjectPool->Delete`.
#[hook(addr = 0x5A1F20, conv = cdecl, name = "delete", expect = "8B 4C 24 04 56")]
pub fn object_delete(p: GPtr<u8>) {
    super::entity::pool_delete(g::<GPtr<u8>>(OBJECT_POOL).read(), p, OBJECT_SLOT);
}

/// `CObject::CanBeDeleted` (0x59F120): false for object types 2, 4, 5, 6 (jump table at 0x59F13C).
#[hook(addr = 0x59F120, conv = thiscall, name = "CObject::CanBeDeleted", expect = "0F B6 81 3C 01 00 00", this = "CObject")]
pub fn can_be_deleted(this: GPtr<u8>) -> u8 {
    u8::from(!matches!(rd::<u8>(this, 0x13C), 2 | 4 | 5 | 6))
}

/// `CObject::SetIsStatic(bool)` (0x5A0760, `ret 4`): `bIsStatic`; when cleared on an object with
/// physical flag bit 5 whose door start angle is below -1000, the angle becomes the current heading.
/// Always clears physical flag bit 30.
#[hook(addr = 0x5A0760, conv = thiscall, name = "CObject::SetIsStatic", expect = "8A 44 24 04 0F B6 D0", this = "CObject")]
pub fn set_is_static(this: GPtr<u8>, v: u8) {
    let f = rd::<u32>(this, 0x1C);
    wr::<u32>(this, 0x1C, f ^ (((u32::from(v) << 2) ^ f) & 4));
    // fcomp -1000; test ah,5; jp: only Less goes on
    if v == 0 && rd::<u8>(this, 0x40) & 0x20 != 0 && !x(rd(this, 0x158)).fcom(x(FM1000)).test_parity(0x05) {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let h = if m.is_null() { x(rd(this, 0x10)) } else { matrix_heading(m) };
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & 0xBFFF_FFFF);
        wr::<f32>(this, 0x158, h.to_f32());
        return;
    }
    wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) & 0xBFFF_FFFF);
}

/// `CObject::SetupLighting` (0x554FA0): scorched objects replace the world lights (0.18) and return
/// true; `bLightObject` ones activate the directional light and return
/// `CRenderer::SetupLightingForEntity`; else false.
#[hook(addr = 0x554FA0, conv = thiscall, name = "CObject::SetupLighting", expect = "56 8B F1 F7 46 40 00 00 00 20", this = "CObject")]
pub fn setup_lighting(this: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0x40) & 0x2000_0000 != 0 {
        world_replace_normal_lights_with_scorched(g::<GPtr<u8>>(SCENE_WORLD).read(), f32::from_bits(0x3E38_51EC));
        return 1;
    }
    if rd::<u32>(this, 0x1C) & 0x1000_0000 != 0 {
        activate_directional();
        return setup_lighting_for_entity(this);
    }
    0
}

/// `CCutsceneObject::SetupLighting` (0x553F40): `ActivateDirectional()` then
/// `CRenderer::SetupLightingForEntity(this)`.
#[hook(addr = 0x553F40, conv = thiscall, name = "CCutsceneObject::SetupLighting", expect = "56 8B F1 E8 38 1D 1E 00", this = "CCutsceneObject")]
pub fn cutscene_setup_lighting(this: GPtr<u8>) -> u8 {
    activate_directional();
    setup_lighting_for_entity(this)
}

/// `CObject::RefModelInfo(int32)` (0x59F330, `ret 4`): stores the index (int16) and adds a reference
/// to that model info.
#[hook(addr = 0x59F330, conv = thiscall, name = "CObject::RefModelInfo", expect = "8B 44 24 04 66 89 81 4A 01 00 00", this = "CObject")]
pub fn ref_model_info(this: GPtr<u8>, idx: i32) {
    wr::<u16>(this, 0x14A, idx as u16);
    model_info_add_ref(g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(idx).read());
}

/// `CObject::SetRemapTexture(RwTexture*, int16 txd)` (0x59F350, `ret 8`): the whole argument dword is
/// compared with -1 (original) before `CTxdStore::AddRef`.
#[hook(addr = 0x59F350, conv = thiscall, name = "CObject::SetRemapTexture", expect = "8B 44 24 04 89 81 6C 01 00 00", this = "CObject")]
pub fn set_remap_texture(this: GPtr<u8>, tex: GPtr<u8>, txd: i32) {
    wr::<GPtr<u8>>(this, 0x16C, tex);
    wr::<u16>(this, 0x16A, txd as u16);
    if txd != -1 {
        txd_store_add_ref(txd);
    }
}

/// `CObject::CanBeTargetted` (0x59F320): object flag bit 9.
#[hook(addr = 0x59F320, conv = thiscall, name = "CObject::CanBeTargetted", expect = "8B 81 40 01 00 00", this = "CObject")]
pub fn can_be_targetted(this: GPtr<u8>) -> u8 {
    ((rd::<u32>(this, 0x140) >> 9) & 1) as u8
}

/// `CObject::SetObjectTargettable(bool)` (0x59F300, `ret 4`): object flag bit 9.
#[hook(addr = 0x59F300, conv = thiscall, name = "CObject::SetObjectTargettable", expect = "0F B6 44 24 04", this = "CObject")]
pub fn set_object_targettable(this: GPtr<u8>, v: u8) {
    let f = rd::<u32>(this, 0x140);
    wr::<u32>(this, 0x140, f ^ (((u32::from(v) << 9) ^ f) & 0x200));
}

/// `CObject::IsObjectPointerValid_NotInWorld(CObject*)` (0x5A2B90, cdecl): slot in range and used
/// (no null test).
#[hook(addr = 0x5A2B90, conv = cdecl, name = "CObject::IsObjectPointerValid_NotInWorld", expect = "8B 4C 24 04 56")]
pub fn is_object_pointer_valid_not_in_world(p: GPtr<u8>) -> u8 {
    let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
    let i = pool_index(pool, p, OBJECT_SLOT);
    if i < 0 || i >= rd::<i32>(pool, 8) {
        return 0;
    }
    !(rd::<GPtr<u8>>(pool, 4).wrapping_index(i).read() >> 7) & 1
}

/// `CObject::IsObjectPointerValid(CObject*)` (0x5A2C20, cdecl): slot in range and used, and
/// `bIsBIGBuilding` or a non-null +0xB0.
#[hook(addr = 0x5A2C20, conv = cdecl, name = "CObject::IsObjectPointerValid", expect = "56 8B 35 9C 44 B7 00")]
pub fn is_object_pointer_valid(p: GPtr<u8>) -> u8 {
    let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
    let i = pool_index(pool, p, OBJECT_SLOT);
    if i < 0 || i >= rd::<i32>(pool, 8) || (rd::<GPtr<u8>>(pool, 4).wrapping_index(i).read() as i8) < 0 {
        return 0;
    }
    if rd::<u32>(p, 0x1C) & 0x100 != 0 {
        return 1;
    }
    u8::from(rd::<u32>(p, 0xB0) != 0)
}

/// `CObject::SetMatrixForTrainCrossing(CMatrix*, float angle)` (0x59F200, cdecl): `r = (0,0,1) x
/// right`; forward = `r * cos + (0*sin, 0*sin, sin)` (x87 order and f32 temporaries kept); up =
/// `right x forward`.
#[hook(addr = 0x59F200, conv = cdecl, name = "CObject::SetMatrixForTrainCrossing", expect = "83 EC 3C 56 8B 74 24 44")]
pub fn set_matrix_for_train_crossing(m: GPtr<u8>, angle: f32) {
    let mut up = [0.0f32, 0.0, 1.0];
    let mut r = [0.0f32; 3];
    let mut v = [0.0f32; 3];
    let mut w = [0.0f32; 3];
    let upp = GPtr::from_raw(up.as_mut_ptr()).cast::<u8>();
    let rp = GPtr::from_raw(r.as_mut_ptr()).cast::<u8>();
    let vp = GPtr::from_raw(v.as_mut_ptr()).cast::<u8>();
    let wp = GPtr::from_raw(w.as_mut_ptr()).cast::<u8>();
    cross_product(rp, upp, m);
    let c = x(angle).fcos();
    let rx = x(rd(rp, 0)).fmul(c).to_f32();
    let ry = x(rd(rp, 4)).fmul(c).to_f32();
    let crz = c.fmul_m(rd(rp, 8));
    let s = x(angle).fsin();
    let zs = x(F0).fmul(s);
    let zs32 = zs.to_f32();
    let s32 = s.to_f32();
    wr::<f32>(vp, 0, x(zs32).fadd_m(rx).to_f32());
    wr::<f32>(vp, 4, zs.fadd_m(ry).to_f32());
    wr::<f32>(vp, 8, x(s32).fadd(crz).to_f32());
    cross_product(wp, m, vp);
    for k in [0, 4, 8] {
        wr::<u32>(m, 0x20 + k, rd::<u32>(wp, k));
    }
    for k in [0, 4, 8] {
        wr::<u32>(m, 0x10 + k, rd::<u32>(vp, k));
    }
}

/// `CObject::SetRelatedDummy(CDummyObject*)` (0x59F160, `ret 4`): stores it (+0x170) and tail-calls
/// `dummy->RegisterReference(&m_pRelatedDummy)`.
#[hook(addr = 0x59F160, conv = thiscall, name = "CObject::SetRelatedDummy", expect = "8D 81 70 01 00 00", this = "CObject")]
pub fn set_related_dummy(this: GPtr<u8>, dummy: GPtr<u8>) {
    wr::<GPtr<u8>>(this, 0x170, dummy);
    super::entity::register_reference(dummy, this.byte_add(0x170));
}

/// `CObject::RemoveLighting(bool reset)` (0x553E10, `ret 4`): when set, point lights removed (unless
/// scorched), ambient colours, directional light off.
#[hook(addr = 0x553E10, conv = thiscall, name = "CObject::RemoveLighting", expect = "8A 44 24 04 84 C0", this = "CObject")]
pub fn remove_lighting(this: GPtr<u8>, reset: u8) {
    if reset == 0 {
        return;
    }
    if rd::<u32>(this, 0x40) & 0x2000_0000 == 0 {
        remove_lights_affecting_object();
    }
    set_ambient_colours();
    deactivate_directional();
}

/// `CCutsceneObject::RemoveLighting(bool)` (0x5533F0, `ret 4`; the argument is not tested): point
/// lights removed (unless scorched), ambient colours, directional light off.
#[hook(addr = 0x5533F0, conv = thiscall, name = "CCutsceneObject::RemoveLighting", expect = "F7 41 40 00 00 00 20", this = "CCutsceneObject")]
pub fn cutscene_remove_lighting(this: GPtr<u8>, _reset: u8) {
    if rd::<u32>(this, 0x40) & 0x2000_0000 == 0 {
        remove_lights_affecting_object();
    }
    set_ambient_colours();
    deactivate_directional();
}

/// `CObject::TryToExplode` (0x59F2D0): when the object info (+0x160) causes an explosion (+0x1F) and
/// `bHasExploded` (flag 0x40 at +0x140) is clear: sets it, `Explode()`, true.
#[hook(addr = 0x59F2D0, conv = thiscall, name = "CObject::TryToExplode", expect = "8B 81 60 01 00 00", this = "CObject")]
pub fn try_to_explode(this: GPtr<u8>) -> u8 {
    if rd::<u8>(rd::<GPtr<u8>>(this, 0x160), 0x1F) == 0 {
        return 0;
    }
    let f = rd::<u32>(this, 0x140);
    if f & 0x40 != 0 {
        return 0;
    }
    wr::<u32>(this, 0x140, f | 0x40);
    object_explode(this);
    1
}

/// `CObject::AddToControlCodeList` (0x59F400): a new double-link node for `this` (+0x138) at the head
/// of `CWorld::ms_listObjectsWithControlCode` (0xB9ACCC). Returns the node.
#[hook(addr = 0x59F400, conv = thiscall, name = "CObject::AddToControlCodeList", expect = "56 6A 0C 8B F1", this = "CObject")]
pub fn add_to_control_code_list(this: GPtr<u8>) -> GPtr<u8> {
    let n = super::entity::double_node_new(0xC);
    if !n.is_null() {
        wr::<GPtr<u8>>(n, 0, this);
    }
    wr::<GPtr<u8>>(this, 0x138, n);
    super::entity::link_double_head(g(CONTROL_CODE_LIST), n);
    n
}

/// `CObject::RemoveFromControlCodeList` (0x59F450): unlinks and frees the node (+0x138), then clears it.
#[hook(addr = 0x59F450, conv = thiscall, name = "CObject::RemoveFromControlCodeList", expect = "56 8B F1 8B 86 38 01 00 00", this = "CObject")]
pub fn remove_from_control_code_list(this: GPtr<u8>) {
    let n = rd::<GPtr<u8>>(this, 0x138);
    if n.is_null() {
        return;
    }
    super::entity::unlink_double(g(CONTROL_CODE_LIST), n);
    super::entity::double_node_delete(rd(this, 0x138));
    wr::<u32>(this, 0x138, 0);
}

/// `CObject` scalar deleting destructor (0x5A2BD0, `ret 4`): `~CObject`, then the object pool slot.
#[hook(addr = 0x5A2BD0, conv = thiscall, name = "CObject::`scalar deleting destructor'", expect = "56 57 8B F9 E8 87 CA FF FF", this = "CObject")]
pub fn object_scalar_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    object_dtor(this);
    super::entity::scalar_delete(this, flags, OBJECT_POOL, OBJECT_SLOT)
}

/// `CHandObject::~CHandObject` (0x59F000, `jmp ~CObject`; non-HOODLUM jump, expect + 1 byte).
#[hook(addr = 0x59F000, conv = thiscall, name = "thunk_FUN_0059f660", expect = "E9 5B 06 00 00 90", this = "CHandObject")]
pub fn hand_object_dtor(this: GPtr<u8>) {
    object_dtor(this);
}

/// `CHandObject` scalar deleting destructor (0x59EFE0, `ret 4`).
#[hook(addr = 0x59EFE0, conv = thiscall, name = "CHandObject::`scalar deleting destructor'", expect = "56 8B F1 E8 18 00 00 00", this = "CHandObject")]
pub fn hand_object_scalar_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    hand_object_dtor(this);
    if flags & 1 != 0 {
        object_delete(this);
    }
    this
}

/// `CDummyObject::~CDummyObject` (0x59EAB0): vtable 0x866E78, then `CDummy::~CDummy`.
#[hook(addr = 0x59EAB0, conv = thiscall, name = "CDummyObject::~CDummyObject", expect = "C7 01 78 6E 86 00", this = "CDummyObject")]
pub fn dummy_object_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x866E78);
    super::entity::dummy_dtor(this);
}

/// `CDummyObject` scalar deleting destructor (0x59EB50, `ret 4`).
#[hook(addr = 0x59EB50, conv = thiscall, name = "CDummyObject::`scalar deleting destructor'", expect = "56 8B F1 E8 58 FF FF FF", this = "CDummyObject")]
pub fn dummy_object_scalar_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dummy_object_dtor(this);
    if flags & 1 != 0 {
        super::entity::dummy_delete(this);
    }
    this
}

/// The `CRopes::ms_aRopes` slot of `FindRope(this)` (no -1 test, original).
fn rope_of(this: GPtr<u8>) -> GPtr<u8> {
    let i = find_rope(this.addr() as u32);
    g::<u8>(ROPES.wrapping_add(i.wrapping_mul(0x328) as u32 as usize))
}

/// `CObject::GetRopeHeight` (0x59F380): the rope's +0x31C height (ST0).
#[hook(addr = 0x59F380, conv = thiscall, name = "CObject::GetRopeHeight", expect = "51 E8 7A 6C FB FF", this = "CObject")]
pub fn get_rope_height(this: GPtr<u8>) -> X87 {
    x(rd(rope_of(this), 0x31C))
}

/// `CObject::SetRopeHeight(float)` (0x59F3A0, `ret 4`).
#[hook(addr = 0x59F3A0, conv = thiscall, name = "CObject::SetRopeHeight", expect = "51 E8 5A 6C FB FF", this = "CObject")]
pub fn set_rope_height(this: GPtr<u8>, h: u32) {
    wr::<u32>(rope_of(this), 0x31C, h);
}

/// `CObject::GetObjectCarriedWithRope` (0x59F3C0): the rope's +0x318 entity.
#[hook(addr = 0x59F3C0, conv = thiscall, name = "CObject::GetObjectCarriedWithRope", expect = "51 E8 3A 6C FB FF", this = "CObject")]
pub fn get_object_carried_with_rope(this: GPtr<u8>) -> GPtr<u8> {
    rd(rope_of(this), 0x318)
}

/// `CObject::ReleaseObjectCarriedWithRope` (0x59F3E0): `CRope::ReleasePickedUpObject` (tail jump).
#[hook(addr = 0x59F3E0, conv = thiscall, name = "CObject::ReleaseObjectCarriedWithRope", expect = "51 E8 1A 6C FB FF", this = "CObject")]
pub fn release_object_carried_with_rope(this: GPtr<u8>) {
    rope_release_picked_up_object(rope_of(this));
}

/// `CObject::LockDoor` (0x59F5C0): physical flags |= 0xC, object flag 0x8000 cleared, then
/// `ResetDoorAngle` (tail jump).
#[hook(addr = 0x59F5C0, conv = thiscall, name = "CObject::LockDoor", expect = "8B 51 40 8B 81 40 01 00 00", this = "CObject")]
pub fn lock_door(this: GPtr<u8>) {
    let f = rd::<u32>(this, 0x40) | 0xC;
    let o = rd::<u32>(this, 0x140) & 0xFFFF_7FFF;
    wr::<u32>(this, 0x40, f);
    wr::<u32>(this, 0x140, o);
    object_reset_door_angle(this);
}

/// `CObject::operator new(uint32, int32 handle)` (0x5A1EF0): the slot `handle >> 8` of the object
/// pool, claimed with the pool's `CreateAtRef(handle)` (0x59F610). Returns the slot.
#[hook(addr = 0x5A1EF0, conv = cdecl, name = "new", expect = "8B 44 24 08 8B 0D 9C 44 B7 00")]
pub fn object_new_at(_size: u32, handle: i32) -> GPtr<u8> {
    let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
    let slot = rd::<GPtr<u8>>(pool, 0).byte_add((handle >> 8).wrapping_mul(OBJECT_SLOT) as isize);
    object_pool_create_at_ref(pool, handle);
    slot
}

/// `CObject::CanBeUsedToTakeCoverBehind` (0x5A1B60): not for object type 2; always for the fire
/// hydrant; not for atomic models of special type 11 (0x5800); else when the col model height is in
/// (0.75, 1.25) and the matrix up.z > 0.9 (no matrix null test, original).
#[hook(addr = 0x5A1B60, conv = thiscall, name = "CObject::CanBeUsedToTakeCoverBehind", expect = "57 8B F9 80 BF 3C 01 00 00 02", this = "CObject")]
pub fn can_be_used_to_take_cover_behind(this: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x13C) == 2 {
        return 0;
    }
    let model = i32::from(rd::<i16>(this, 0x22));
    if model == i32::from(g::<u16>(MI_FIRE_HYDRANT).read()) {
        return 1;
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(model).read();
    if !model_info_as_atomic(mi).is_null() {
        let a = model_info_as_atomic(mi);
        if rd::<u16>(a, 0x12) & 0x7800 == 0x5800 {
            return 0;
        }
    }
    let col = rd::<GPtr<u8>>(g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(model).read(), 0x14);
    let h = x(rd(col, 0x14)).fsub_m(rd(col, 8));
    // fcom 1.25; test ah,5; jp: only Less goes on
    if h.fcom(x(F1_25)).test_parity(0x05) {
        return 0;
    }
    // fcomp 0.75; test ah,0x41; jne: only Greater goes on
    if h.fcom(x(F0_75)).test_any(0x41) {
        return 0;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    if x(rd(m, 0x28)).fcom(x(F0_9)).test_any(0x41) {
        return 0;
    }
    1
}

/// `CObject::CreateRwObject` (0x59F110, `jmp CEntity::CreateRwObject`; non-HOODLUM jump, expect + 1
/// byte).
#[hook(addr = 0x59F110, conv = thiscall, name = "CObject::CreateRwObject", expect = "E9 1B 4C F9 FF 90", this = "CObject")]
pub fn object_create_rw_object(this: GPtr<u8>) {
    entity_create_rw_object(this);
}

/// The used, non-null object pool slots in index order over the pool and size read at entry (the
/// originals keep both in locals), with `f` deciding per object.
fn for_each_object(mut f: impl FnMut(GPtr<u8>)) {
    let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
    let n = rd::<i32>(pool, 8);
    let mut i = 0;
    while i < n {
        if (rd::<GPtr<u8>>(pool, 4).wrapping_index(i).read() as i8) >= 0 {
            let o = GPtr::<u8>::from_addr(rd::<u32>(pool, 0).wrapping_add((i as u32).wrapping_mul(OBJECT_SLOT as u32)) as usize);
            if !o.is_null() {
                f(o);
            }
        }
        i += 1;
    }
}

/// `CWorld::Remove` then the scalar deleting destructor (vtable slot 0, flag 1).
fn remove_and_delete(o: GPtr<u8>) {
    world_remove(o);
    entity_vdelete(o, 1);
}

/// `CObject::DeleteAllTempObjects` (0x5A18B0): removes and deletes every temp object (type 3).
#[hook(addr = 0x5A18B0, conv = cdecl, name = "CObject::DeleteAllTempObjects", expect = "51 55 8B 2D 9C 44 B7 00")]
pub fn delete_all_temp_objects() {
    for_each_object(|o| {
        if rd::<u8>(o, 0x13C) == 3 {
            remove_and_delete(o);
        }
    });
}

/// `CObject::DeleteAllMissionObjects` (0x5A1910): removes and deletes every mission object (types 2, 6).
#[hook(addr = 0x5A1910, conv = cdecl, name = "CObject::DeleteAllMissionObjects", expect = "51 53 8B 1D 9C 44 B7 00")]
pub fn delete_all_mission_objects() {
    for_each_object(|o| {
        if matches!(rd::<u8>(o, 0x13C), 2 | 6) {
            remove_and_delete(o);
        }
    });
}

/// `CObject::DeleteAllTempObjectsInArea(CVector point, float radius)` (0x5A1980, cdecl): temp objects
/// with `r^2 > (dz^2 + dy^2) + dx^2`.
#[hook(addr = 0x5A1980, conv = cdecl, name = "CObject::DeleteAllTempObjectsInArea", expect = "83 EC 10 A1 9C 44 B7 00")]
pub fn delete_all_temp_objects_in_area(px: f32, py: f32, pz: f32, radius: f32) {
    for_each_object(|o| {
        if rd::<u8>(o, 0x13C) != 3 {
            return;
        }
        let p = super::entity::pos_ptr(o);
        let dz = x(pz).fsub_m(rd(p, 8));
        let dy = x(py).fsub_m(rd(p, 4));
        let dx = x(px).fsub_m(rd(p, 0));
        let s = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        if !x(radius).fmul_m(radius).fcom(s).test_any(0x41) {
            remove_and_delete(o);
        }
    });
}

/// `CObject::TryToFreeUpTempObjects(int32 n)` (0x5A1840): from the last slot down, removes and
/// deletes up to `n` temp objects that are not visible (`CEntity::IsVisible`).
#[hook(addr = 0x5A1840, conv = cdecl, name = "CObject::TryToFreeUpTempObjects", expect = "51 A1 9C 44 B7 00")]
pub fn try_to_free_up_temp_objects(n: i32) {
    let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
    let mut i = rd::<i32>(pool, 8);
    if i == 0 {
        return;
    }
    let mut n = n;
    loop {
        i -= 1;
        if n <= 0 {
            return;
        }
        if (rd::<GPtr<u8>>(pool, 4).wrapping_index(i).read() as i8) >= 0 {
            let o = GPtr::<u8>::from_addr(rd::<u32>(pool, 0).wrapping_add((i as u32).wrapping_mul(OBJECT_SLOT as u32)) as usize);
            if !o.is_null() && rd::<u8>(o, 0x13C) == 3 && super::entity::is_visible(o) == 0 {
                remove_and_delete(o);
                n -= 1;
            }
        }
        if i == 0 {
            return;
        }
    }
}

/// `CObject::ResetDoorAngle` (0x59F4B0): for door objects (physical flag 0x20) whose start angle is
/// not below-or-equal -1000 (NaN included): heading = start angle, `SetIsStatic(true)` (vtable 0x10),
/// move/turn speeds and their friction vectors zero, RW matrix and frame updated.
#[hook(addr = 0x59F4B0, conv = thiscall, name = "CObject::ResetDoorAngle", expect = "83 EC 10 56 8B F1", this = "CObject")]
pub fn reset_door_angle(this: GPtr<u8>) {
    if rd::<u8>(this, 0x40) & 0x20 == 0 {
        return;
    }
    // fcomp -1000; test ah,0x41; jnp: Less / Equal leave
    if !x(rd(this, 0x158)).fcom(x(FM1000)).test_parity(0x41) {
        return;
    }
    let a = rd::<f32>(this, 0x158);
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        wr::<f32>(this, 0x10, a);
    } else {
        super::entity::matrix_set_rotate_z_only(m, a);
    }
    entity_vset_is_static(this, 1);
    for o in [0x44, 0x48, 0x4C, 0x5C, 0x60, 0x64, 0x50, 0x54, 0x58, 0x68, 0x6C, 0x70] {
        wr::<u32>(this, o, 0);
    }
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if !rw.is_null() {
        let mat = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let m = rd::<GPtr<u8>>(this, 0x14);
        if m.is_null() {
            simple_transform_update_rw_matrix(this.byte_add(4), mat);
        } else {
            matrix_update_rw(m, mat);
        }
    }
    super::entity::update_rw_frame(this);
}

/// `CObject::GetLightingFromCollisionBelow` (0x59FD00): `CWorld::ProcessVerticalLine(pos, -1000, ...,
/// buildings + dummies)`; on a hit the col point's lighting byte (+0x25) goes to +0x149.
#[hook(addr = 0x59FD00, conv = thiscall, name = "CObject::GetLightingFromCollisionBelow", expect = "83 EC 30 56 8B F1", this = "CObject")]
pub fn get_lighting_from_collision_below(this: GPtr<u8>) {
    let mut cp = [0u32; 11];
    let mut ent = [0u32; 1];
    let cpp = GPtr::from_raw(cp.as_mut_ptr()).cast::<u8>();
    let entp = GPtr::from_raw(ent.as_mut_ptr()).cast::<u8>();
    if world_process_vertical_line(super::entity::pos_ptr(this), FM1000, cpp, entp, 1, 0, 0, 0, 1, 0, 0) != 0 {
        wr::<u8>(this, 0x149, rd::<u8>(cpp, 0x25));
    }
}

/// `CObject::CObject` (0x5A1D10): `CPhysical` base, vtable 0x866F60, related dummy null, `Init()`, type
/// 0. Returns `this`. (The SEH frame around the base constructor is not built: stack only.)
#[hook(addr = 0x5A1D10, conv = thiscall, name = "CObject", expect = "6A FF 68 A8 D2 83 00", this = "CObject")]
pub fn object_ctor(this: GPtr<u8>) -> GPtr<u8> {
    physical_ctor(this);
    wr::<u32>(this, 0, 0x866F60);
    wr::<u32>(this, 0x170, 0);
    object_init(this);
    wr::<u8>(this, 0x13C, 0);
    this
}

/// `CObjectData::SetObjectData(int32 modelIndex, CObject& obj)` (0x5A2D00, cdecl): the object info of
/// the model (+0x10, 0x50 each at 0xBB4A90; -1 → the default with mass 99999) and the physical flags
/// that follow from it (fstp copies, original order).
#[hook(addr = 0x5A2D00, conv = cdecl, name = "CObjectData::SetObjectData", expect = "53 56 8B 74 24 0C")]
pub fn set_object_data(model: i32, obj: GPtr<u8>) {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(model).read();
    let idx = rd::<i16>(mi, 0x10);
    if idx == -1 {
        wr::<u32>(obj, 0x8C, 0x47C3_4F80);
        wr::<u32>(obj, 0x90, 0x47C3_4F80);
        let f = (rd::<u32>(obj, 0x40) & !2) | 0x80_000C;
        wr::<u32>(obj, 0x160, OBJECT_INFO as u32);
        wr::<u32>(obj, 0x40, f);
        return;
    }
    let d = g::<u8>(OBJECT_INFO.wrapping_add((i32::from(idx) * 0x50) as usize));
    wr::<GPtr<u8>>(obj, 0x160, d);
    wr::<u32>(obj, 0x8C, rd::<u32>(d, 0));
    wr::<f32>(obj, 0x90, x(rd(d, 4)).to_f32());
    wr::<f32>(obj, 0x98, x(rd(d, 8)).to_f32());
    wr::<f32>(obj, 0x9C, x(rd(d, 0xC)).to_f32());
    wr::<f32>(obj, 0xA0, x(rd(d, 0x10)).to_f32());
    wr::<u8>(obj, 0x144, rd::<u8>(d, 0x1C));
    // fcomp 99999; test ah,1; jne: Less / NaN skip
    if !x(rd(d, 0)).fcom(x(F99999)).test_any(0x01) {
        let f = (rd::<u32>(obj, 0x40) & !2) | 0xC;
        wr::<u32>(obj, 0x40, f);
        if rd::<u8>(d, 0x1C) == 0 {
            wr::<u32>(obj, 0x40, f | 0x80_0000);
        }
    }
    match rd::<u8>(d, 0x1D) {
        6 => {
            let f = rd::<u32>(obj, 0x40) | 0x20;
            let o = rd::<u32>(obj, 0x140) & 0xFFFF_3FFF;
            wr::<u32>(obj, 0x40, f);
            wr::<u32>(obj, 0x140, o);
        }
        7 => {
            let o = (rd::<u32>(obj, 0x140) & 0xFFFF_7FFF) | 0x4000;
            let f = rd::<u32>(obj, 0x40) | 0x2C;
            wr::<u32>(obj, 0x40, f);
            wr::<u32>(obj, 0x140, o);
        }
        8 => {
            wr::<u32>(obj, 0x40, rd::<u32>(obj, 0x40) | 0x40);
            let col = rd::<GPtr<u8>>(g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(model).read(), 0x14);
            let z = x(rd(col, 8)).fmul_m(F0_8).to_f32();
            object_set_centre_of_mass(obj, 0, 0, z);
        }
        9 => {
            wr::<u32>(obj, 0x40, (rd::<u32>(obj, 0x40) & !2) | 0x80);
        }
        _ => {}
    }
}

/// `CObject::Save()` (0x5D2830): a `CObjectSaveStructure` (0x30) written after its size. True.
#[hook(addr = 0x5D2830, conv = thiscall, name = "CObject::Save", expect = "83 EC 34 51 8D 4C 24 08", this = "CObject")]
pub fn save(this: GPtr<u8>) -> u8 {
    let mut size = [0x30u32];
    let mut s = [0u32; 0xC];
    let sp = GPtr::from_raw(s.as_mut_ptr()).cast::<u8>();
    save_structure_construct(sp, this);
    super::common::save_data_to_work_buffer(GPtr::from_raw(size.as_mut_ptr()).cast(), 4);
    super::common::save_data_to_work_buffer(sp, 0x30);
    1
}

/// `CObject::Load()` (0x5D2870): the size and the `CObjectSaveStructure`, then `Extract(this)`. True.
#[hook(addr = 0x5D2870, conv = thiscall, name = "CObject::Load", expect = "83 EC 34 56 8D 44 24 04", this = "CObject")]
pub fn load(this: GPtr<u8>) -> u8 {
    let mut size = [0u32];
    let mut s = [0u32; 0xC];
    let sp = GPtr::from_raw(s.as_mut_ptr()).cast::<u8>();
    super::common::load_data_from_work_buffer(GPtr::from_raw(size.as_mut_ptr()).cast(), 4);
    super::common::load_data_from_work_buffer(sp, 0x30);
    save_structure_extract(sp, this);
    1
}

/// `CObject::Teleport(CVector pos, bool)` (0x5A17B0, `ret 0x10`): out of the world, new position
/// (dword copies), RW matrix and frame updated, back into the world.
#[hook(addr = 0x5A17B0, conv = thiscall, name = "CObject::Teleport", expect = "56 8B F1 56 E8 C7 1A FC FF", this = "CObject")]
pub fn teleport(this: GPtr<u8>, px: u32, py: u32, pz: u32, _reset: u32) {
    world_remove(this);
    let d = super::entity::pos_ptr(this);
    wr::<u32>(d, 0, px);
    wr::<u32>(d, 4, py);
    wr::<u32>(d, 8, pz);
    update_rw_matrix_from(this);
    super::entity::update_rw_frame(this);
    world_add(this);
}

/// The RW frame's modelling matrix from the matrix (`CMatrix::UpdateRW`) or the placement
/// (`CSimpleTransform::UpdateRwMatrix`), when there is an RwObject.
fn update_rw_matrix_from(this: GPtr<u8>) {
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if rw.is_null() {
        return;
    }
    let mat = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        simple_transform_update_rw_matrix(this.byte_add(4), mat);
    } else {
        matrix_update_rw(m, mat);
    }
}

/// `CDummyObject::UpdateFromObject(CObject*)` (0x59EB70, `ret 4`): takes over the object's RwObject
/// (`bUsesCollision | bIsVisible`), detaches it from the object; for objects in an interior whose
/// interior slot (+0x2E, pool 0x8E3FB0, 0x34 each) has +0x2F set, copies the flags 0x200, 0x80 and 1;
/// moves the IPL dword (+0x30).
#[hook(addr = 0x59EB70, conv = thiscall, name = "CDummyObject::UpdateFromObject", expect = "56 8B F1 8B 4E 1C", this = "CDummyObject")]
pub fn update_from_object(this: GPtr<u8>, obj: GPtr<u8>) {
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x81);
    wr::<u32>(obj, 0x1C, rd::<u32>(obj, 0x1C) | 0x2000);
    entity_attach_to_rw_object(this, rd(obj, 0x18), 0);
    wr::<u32>(obj, 0x1C, rd::<u32>(obj, 0x1C) & 0xFFFF_DFFF);
    update_rw_matrix_from(this);
    super::entity::detach_from_rw_object(obj);
    let area = rd::<u8>(obj, 0x2E);
    if area != 0 {
        let pool = g::<GPtr<u8>>(0x8E3FB0).read();
        let slot = if (rd::<GPtr<u8>>(pool, 4).wrapping_index(i32::from(area)).read() as i8) < 0 {
            GPtr::<u8>::NULL
        } else {
            rd::<GPtr<u8>>(pool, 0).byte_add(isize::from(area) * 0x34)
        };
        if rd::<u8>(slot, 0x2F) != 0 {
            let mut t = rd::<u32>(this, 0x1C);
            for bit in [0x200u32, 0x80, 1] {
                t ^= (rd::<u32>(obj, 0x1C) ^ t) & bit;
                wr::<u32>(this, 0x1C, t);
            }
        }
    }
    wr::<u32>(this, 0x30, rd::<u32>(obj, 0x30));
    wr::<u32>(obj, 0x30, 0);
}

/// `CObject::CObject(int32 model, bool create)` (0x5A1D70, `ret 8`): `CPhysical` base, vtable
/// 0x866F60, related dummy null, `SetModelIndex` or `SetModelIndexNoCreate`, `Init()`. Returns `this`.
/// (The SEH frame is not built: stack only.)
#[hook(addr = 0x5A1D70, conv = thiscall, name = "CObject", expect = "6A FF 68 C8 D2 83 00", this = "CObject")]
pub fn object_ctor_model(this: GPtr<u8>, model: u32, create: u8) -> GPtr<u8> {
    physical_ctor(this);
    wr::<u32>(this, 0, 0x866F60);
    wr::<u32>(this, 0x170, 0);
    if create != 0 {
        super::entity::set_model_index(this, model);
    } else {
        super::entity::set_model_index_no_create(this, model);
    }
    object_init(this);
    this
}

/// `CObject::Create(CDummyObject*)` (0x5A2070, cdecl): a pool slot (with the pool's +0x11 flag set
/// around the first try) constructed from the dummy; when full, frees up 5 temp objects and 5 water
/// creatures and tries once more. Null on failure. (The SEH frame is not built: stack only.)
#[hook(addr = 0x5A2070, conv = cdecl, name = "CObject::Create-dummy", expect = "6A FF 68 46 D3 83 00", this = "CObject")]
pub fn create_from_dummy(dummy: GPtr<u8>) -> GPtr<u8> {
    wr::<u8>(g::<GPtr<u8>>(OBJECT_POOL).read(), 0x11, 1);
    let p = object_pool_new(g::<GPtr<u8>>(OBJECT_POOL).read());
    let o = if p.is_null() { GPtr::NULL } else { object_ctor_dummy(p, dummy) };
    wr::<u8>(g::<GPtr<u8>>(OBJECT_POOL).read(), 0x11, 0);
    if !o.is_null() {
        return o;
    }
    try_to_free_up_temp_objects(5);
    water_creatures_try_to_free_up(g(0xC1DF30), 5);
    let p = object_pool_new(g::<GPtr<u8>>(OBJECT_POOL).read());
    if p.is_null() {
        return GPtr::NULL;
    }
    object_ctor_dummy(p, dummy)
}

/// `CObject::GrabObjectToCarryWithRope(CPhysical*)` (0x5A1AB0, `ret 4`): the rope of `this` releases
/// its load, takes `obj` (registered reference), and its attached entity (+0x314) is moved to
/// `obj.matrix * (0, 0, FindPickupHeight(obj))` (x87 loads before the stores) and loses
/// `bUsesCollision`.
#[hook(addr = 0x5A1AB0, conv = thiscall, name = "CObject::GrabObjectToCarryWithRope", expect = "83 EC 18 53 56 57", this = "CObject")]
pub fn grab_object_to_carry_with_rope(this: GPtr<u8>, obj: GPtr<u8>) {
    let rope = rope_of(this);
    rope_release_picked_up_object(rope);
    wr::<GPtr<u8>>(rope, 0x318, obj);
    super::entity::register_reference(obj, rope.byte_add(0x318));
    let h = X87::from_f64_exact(ropes_find_pickup_height(obj)).to_f32();
    let mut v = [0u32, 0, h.to_bits()];
    let mut r = [0u32; 3];
    let res = super::entity::matrix_mul_vec(
        GPtr::from_raw(r.as_mut_ptr()).cast(),
        rd(rd::<GPtr<u8>>(rope, 0x318), 0x14),
        GPtr::from_raw(v.as_mut_ptr()).cast(),
    );
    let z = x(rd(res, 8));
    let y = x(rd(res, 4));
    let xx = x(rd(res, 0));
    let e = rd::<GPtr<u8>>(rope, 0x314);
    let d = super::entity::pos_ptr(e);
    wr::<f32>(d, 0, xx.to_f32());
    wr::<f32>(d, 4, y.to_f32());
    wr::<f32>(d, 8, z.to_f32());
    let e = rd::<GPtr<u8>>(rope, 0x314);
    wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) & !1);
}

/// `CDummyObject::CDummyObject(CObject*)` (0x59EA20, `ret 4`): `CDummy` base, vtable 0x866E78, the
/// object's model (no create), its RwObject taken over (`AttachToRwObject(.., true)`) and detached
/// from it, area/IPL bytes (+0x2E, +0x2F) copied, `CIplStore::IncludeEntity(area, this)`. Returns
/// `this`. (The SEH frame is not built: stack only.)
#[hook(addr = 0x59EA20, conv = thiscall, name = "CDummyObject::CDummyObject", expect = "6A FF 68 A8 D1 83 00", this = "CDummyObject")]
pub fn dummy_object_ctor(this: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> {
    super::entity::dummy_ctor(this);
    wr::<u32>(this, 0, 0x866E78);
    super::entity::set_model_index_no_create(this, i32::from(rd::<i16>(obj, 0x22)) as u32);
    let rw = rd::<GPtr<u8>>(obj, 0x18);
    if !rw.is_null() {
        super::entity::attach_to_rw_object(this, rw, 1);
    }
    super::entity::detach_from_rw_object(obj);
    let area = rd::<u8>(obj, 0x2E);
    wr::<u8>(this, 0x2E, area);
    wr::<u8>(this, 0x2F, rd::<u8>(obj, 0x2F));
    ipl_store_include_entity(i32::from(area), this);
    this
}

/// `CCutsceneObject::SetupCarPipeAtomicsForClump(uint32 model, RpClump*)` (0x5B1AB0, cdecl): the six
/// car-model keys (0x8D0F68, through 0x53CF30) are computed once (0xBC4058); a clump of one of those
/// models gets the car pipe on every atomic (callback 0x5B1A90).
#[hook(addr = 0x5B1AB0, conv = cdecl, name = "CCutsceneObject::SetupCarPipeAtomicsForClump", expect = "A0 58 40 BC 00", this = "CCutsceneObject")]
pub fn setup_car_pipe_atomics_for_clump(model: u32, clump: GPtr<u8>) {
    if g::<u8>(0xBC4058).read() == 0 {
        g::<u8>(0xBC4058).write(1);
        for i in 0..6usize {
            let k = key_gen_get_uppercase_key(g::<GPtr<u8>>(0x8D0F68 + i * 4).read());
            g::<u32>(0xBC4040 + i * 4).write(k);
        }
    }
    let key = rd::<u32>(g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(model as i32).read(), 4);
    for i in 0..6usize {
        if g::<u32>(0xBC4040 + i * 4).read() == key {
            super::entity::rp_clump_for_all_atomics(clump, 0x5B1A90, 0);
            return;
        }
    }
}

/// `CCutsceneObject::SetModelIndex(uint32)` (0x5B1B20, `ret 4`): `CEntity::SetModelIndex`; clumps get
/// their anim blend data (velocity pointer = +0x44, first frame flag 0x10) and the car pipe; the
/// model's +0xC byte becomes 0xFF.
#[hook(addr = 0x5B1B20, conv = thiscall, name = "CCutsceneObject::SetModelIndex", expect = "56 57 8B 7C 24 0C", this = "CCutsceneObject")]
pub fn cutscene_set_model_index(this: GPtr<u8>, model: u32) {
    super::entity::set_model_index(this, model);
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if rd::<u8>(rw, 0) == 2 {
        rp_anim_blend_clump_init(rw);
        let off = g::<u32>(CLUMP_ANIM_OFFSET).read();
        let data = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x18), off as isize);
        wr::<GPtr<u8>>(data, 0xC, this.byte_add(0x44));
        let data = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x18), g::<u32>(CLUMP_ANIM_OFFSET).read() as isize);
        let fr = rd::<GPtr<u8>>(data, 0x10);
        wr::<u8>(fr, 0, rd::<u8>(fr, 0) | 0x10);
        setup_car_pipe_atomics_for_clump(model, rd(this, 0x18));
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(model as i32).read();
    wr::<u8>(mi, 0xC, 0xFF);
}

/// `CObject::CObject(model, false)` as inlined in `CObject::Create(int32, bool)`.
fn object_ctor_inline(p: GPtr<u8>, model: u32) -> GPtr<u8> {
    physical_ctor(p);
    wr::<u32>(p, 0, 0x866F60);
    wr::<u32>(p, 0x170, 0);
    super::entity::set_model_index_no_create(p, model);
    object_init(p);
    p
}

/// `CObject::Create(int32 model, bool)` (0x5A1F60, cdecl; the bool is not used): a pool slot (pool
/// flag +0x11 set around the first try) constructed without RwObject; when full, frees up 5 temp
/// objects and 5 water creatures and tries once more. Null on failure. (SEH frame not built.)
#[hook(addr = 0x5A1F60, conv = cdecl, name = "CObject::Create-intbool", expect = "6A FF 68 26 D3 83 00")]
pub fn create_from_model(model: u32, _create: u8) -> GPtr<u8> {
    wr::<u8>(g::<GPtr<u8>>(OBJECT_POOL).read(), 0x11, 1);
    let p = object_pool_new(g::<GPtr<u8>>(OBJECT_POOL).read());
    let o = if p.is_null() { GPtr::NULL } else { object_ctor_inline(p, model) };
    wr::<u8>(g::<GPtr<u8>>(OBJECT_POOL).read(), 0x11, 0);
    if !o.is_null() {
        return o;
    }
    try_to_free_up_temp_objects(5);
    water_creatures_try_to_free_up(g(0xC1DF30), 5);
    let p = object_pool_new(g::<GPtr<u8>>(OBJECT_POOL).read());
    if p.is_null() {
        return GPtr::NULL;
    }
    object_ctor_inline(p, model)
}

/// The `ModelIndices` `CObject::Init` tests for object flag 0x100 and for the control-code list.
const INIT_MI_FLAG_100: [usize; 8] = [0x8CD4FC, 0x8CD614, 0x8CD518, 0x8CD51C, 0x8CD520, 0x8CD524, 0x8CD504, 0x8CD500];
const INIT_MI_CONTROL: [usize; 7] = [0x8CD6AC, 0x8CD6B0, 0x8CD6B4, 0x8CD6B8, 0x8CD6BC, 0x8CD6C0, 0x8CD6C4];

/// `CObject::Init` (0x59F840): type 4, object info default, `SetIsStatic(true)` (vtable 0x10), object
/// flags (kept bits 0xFC040000, then 0x40000), object data of the model (col store ref and flag
/// 0x10000 for col models with +0x29 bit 0; a centre-of-mass z for special types 1/2 without physical
/// flag 4), door col data reset, many defaults, the model-specific flags and the control-code list.
#[hook(addr = 0x59F840, conv = thiscall, name = "CObject::Init", expect = "53 56 8B F1 8A 46 36", this = "CObject")]
pub fn object_init_port(this: GPtr<u8>) {
    wr::<u8>(this, 0x36, (rd::<u8>(this, 0x36) & 0xFC) | 4);
    wr::<u32>(this, 0x160, OBJECT_INFO as u32);
    wr::<u8>(this, 0x144, 0);
    wr::<u8>(this, 0x145, 0);
    wr::<u8>(this, 0x13C, 1);
    entity_vset_is_static(this, 1);
    let f = (rd::<u32>(this, 0x140) & 0xFC04_0000) | 0x4_0000;
    let model = rd::<i16>(this, 0x22);
    if model == -1 {
        wr::<u32>(this, 0x140, f & 0xFFFE_FFFF);
    } else {
        wr::<u32>(this, 0x140, f);
        set_object_data(i32::from(model), this);
        let m = i32::from(rd::<i16>(this, 0x22));
        let col = rd::<GPtr<u8>>(g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(m).read(), 0x14);
        if rd::<u8>(col, 0x29) & 1 != 0 {
            col_store_add_ref(i32::from(rd::<u8>(col, 0x28)));
            wr::<u32>(this, 0x140, rd::<u32>(this, 0x140) | 0x1_0000);
        }
        let m = i32::from(rd::<i16>(this, 0x22));
        let a = model_info_as_atomic(g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(m).read());
        if !a.is_null() {
            let t = rd::<u16>(a, 0x12) & 0x7800;
            if (t == 0x800 || t == 0x1000) && rd::<u8>(this, 0x40) & 4 == 0 {
                let v = x(rd(col, 0x14)).fsub_m(rd(col, 8)).fmul_m(F0_2).fadd_m(rd(col, 8));
                wr::<f32>(this, 0xAC, v.to_f32());
            }
        }
    }
    if rd::<u8>(this, 0x40) & 0x20 != 0 {
        let d = rd::<GPtr<u8>>(super::entity::col_model_of(this), 0x2C);
        if !d.is_null() {
            wr::<u16>(d, 0, 0);
        }
    }
    let model = rd::<i16>(this, 0x22);
    wr::<u32>(this, 0x154, 0x447A_0000);
    wr::<u32>(this, 0x158, 0xC47A_4000);
    wr::<u32>(this, 0x150, 0);
    wr::<u8>(this, 0x13D, 0);
    wr::<u16>(this, 0x13E, 0);
    wr::<u8>(this, 0x14D, 0);
    wr::<u8>(this, 0x14C, 0);
    wr::<i16>(this, 0x14A, -1);
    wr::<u8>(this, 0x147, 0xFF);
    wr::<u8>(this, 0x148, 0xFF);
    wr::<u32>(this, 0x164, 0);
    let m = i32::from(model);
    if m == i32::from(g::<u16>(0x8CD608).read()) {
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x800_0000);
    }
    if model != -1 && super::entity::model_info_get_type(g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(m).read()) == 4 {
        wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x1000_0000);
    }
    let model = rd::<i16>(this, 0x22);
    let m = i32::from(model);
    let is = |a: usize| m == i32::from(g::<u16>(a).read());
    let set = INIT_MI_FLAG_100.iter().any(|&a| is(a))
        || (model != -1 && {
            let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(m).read();
            !model_info_as_atomic(mi).is_null() && {
                let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
                model_info_sways_in_wind(model_info_as_atomic(mi)) != 0
            }
        });
    let f = if set { rd::<u32>(this, 0x140) | 0x100 } else { rd::<u32>(this, 0x140) & 0xFFFF_FEFF };
    let pf = rd::<u32>(this, 0x40) & 0xFDFF_FFFF;
    wr::<u32>(this, 0x140, f);
    wr::<u32>(this, 0x140, f & 0xFFFF_FDFF);
    wr::<u8>(this, 0x2F, 0xD);
    wr::<i16>(this, 0x16A, -1);
    wr::<u32>(this, 0x16C, 0);
    wr::<u32>(this, 0x40, pf);
    wr::<u32>(this, 0x138, 0);
    let m = i32::from(rd::<i16>(this, 0x22));
    let is = |a: usize| m == i32::from(g::<u16>(a).read());
    if INIT_MI_CONTROL.iter().any(|&a| is(a)) || garages_is_model_index_a_door(m) != 0 {
        add_to_control_code_list(this);
    }
    wr::<u32>(this, 0x174, 0);
    wr::<u32>(this, 0x15C, 0x3F80_0000);
    wr::<u8>(this, 0x149, 0x48);
    wr::<i16>(this, 0x168, -1);
}

/// `CObject::CObject(CDummyObject*)` (0x5A1DF0, `ret 4`): `CPhysical` base, vtable 0x866F60, the
/// dummy's model (no create); its RwObject taken over (`AttachToRwObject(.., true)`) or its matrix
/// copied (`SetMatrix(dummy.GetMatrix())`); dummy detached; `Init()`; area/IPL bytes and flag 0x200
/// copied; flag 0x10000000 when the atomic has no CBPC pipeline. Returns `this`. (SEH frame not built;
/// the related-dummy field +0x170 is not written here, original.)
#[hook(addr = 0x5A1DF0, conv = thiscall, name = "CObject", expect = "6A FF 68 E8 D2 83 00", this = "CObject")]
pub fn object_ctor_from_dummy(this: GPtr<u8>, dummy: GPtr<u8>) -> GPtr<u8> {
    physical_ctor(this);
    wr::<u32>(this, 0, 0x866F60);
    super::entity::set_model_index_no_create(this, i32::from(rd::<i16>(dummy, 0x22)) as u32);
    let rw = rd::<GPtr<u8>>(dummy, 0x18);
    if !rw.is_null() {
        super::entity::attach_to_rw_object(this, rw, 1);
    } else {
        let m = super::entity::get_matrix(dummy);
        placeable_set_matrix(this, m);
    }
    super::entity::detach_from_rw_object(dummy);
    object_init_port(this);
    wr::<u8>(this, 0x2E, rd::<u8>(dummy, 0x2E));
    wr::<u8>(this, 0x2F, rd::<u8>(dummy, 0x2F));
    let rw = rd::<GPtr<u8>>(this, 0x18);
    let t = rd::<u32>(this, 0x1C);
    wr::<u32>(this, 0x1C, t ^ ((rd::<u32>(dummy, 0x1C) ^ t) & 0x200));
    if !rw.is_null() {
        let a = if rd::<u8>(rw, 0) == 1 { rw } else { super::entity::rw_get_first_atomic(rw) };
        if is_cbpc_pipeline_attached(a) == 0 {
            wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x1000_0000);
        }
    }
    this
}

/// `CHandObject::ProcessControl` (0x59EC40): the matrix follows the ped's bone (+0x180) matrix of its
/// skin hierarchy (temporary `CMatrix` attached to it, assigned); then flag 0x20 and
/// `CPhysical::RemoveAndAdd`. (SEH frame not built.)
#[hook(addr = 0x59EC40, conv = thiscall, name = "CHandObject::ProcessControl", expect = "6A FF 68 C8 D1 83 00", this = "CHandObject")]
pub fn hand_object_process_control(this: GPtr<u8>) {
    let ped = rd::<GPtr<u8>>(this, 0x17C);
    let h = super::entity::rw_get_anim_hierarchy_from_skin_clump(rd(ped, 0x18));
    let mats = rp_h_anim_hierarchy_get_matrix_array(h);
    let bone = mats.byte_add((rd::<u32>(this, 0x180) << 6) as i32 as isize);
    let mut tmp = [0u32; 0x12];
    let tp = GPtr::from_raw(tmp.as_mut_ptr()).cast::<u8>();
    super::entity::matrix_ctor_attach(tp, bone, 0);
    matrix_assign(rd(this, 0x14), tp);
    super::entity::matrix_dtor(tp);
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x20);
    physical_remove_and_add(this);
}

/// Registrations of this module.
/// `CObject` vtable (0x866F60).
const OBJECT_VTABLE: u32 = 0x866F60;
/// `CTheScripts::ScriptsForBrains` (0xA90CF0, 0x14-byte brains: streamed script index int16 +0)
const SCRIPTS_FOR_BRAINS: usize = 0xA90CF0;
/// `CObject::nNoTempObjects` (int16)
const NO_TEMP_OBJECTS: usize = 0xBB4A70;
/// `ModelIndices` entry 0x8CD6B8: the bridge whose removal relights the bridge path links
const MI_BRIDGE_LIGHTS: usize = 0x8CD6B8;
/// `ThePaths` (`CPathFind`)
const THE_PATHS: usize = 0x96F050;
/// 0x858CCC: the half extent of the bridge-lights rectangle
const BRIDGE_HALF: usize = 0x858CCC;

/// `CObject::~CObject()` (0x59F660, thiscall; catalogue name `CObject`, `FUN_0059f660`): vtable
/// 0x866F60. A script-brain object (flags 0x300000, +0x140): `SetMissionDoesntRequireModel(0x6676 +
/// ScriptsForBrains[+0x168].scriptIndex)`, flags &= !0x300000 (re-read after the call),
/// `RemoveFromWaitingForScriptBrainArray(this, +0x168)`, +0x168 = -1. Flag byte +0x142 bit 0:
/// `CColStore::RemoveRef(colModel->colSlot)` (model info +0x14, byte +0x28). `CRadar::ClearBlipForEntity(3,
/// pool handle)`; ref model +0x14A ≠ -1 → its model info's `RemoveRef`; remap txd +0x16A ≠ -1 →
/// `CTxdStore::RemoveRef`; a temporary object (+0x13C = 3) decrements `nNoTempObjects` when above 0;
/// `RemoveFromControlCodeList`. The bridge model (`[0x8CD6B8]`) relights the path links in the rectangle
/// of half extent `[0x858CCC]` around the position of the entity at +0x170 (matrix position or placement;
/// x87 `fadd`/`fsub`, stored as f32): `ThePaths.SetLinksBridgeLights(x - h, x + h, y - h, y + h, false)`.
/// A fire (+0x164) is extinguished. Then `CPhysical::~CPhysical()`, whose EAX is returned.
#[hook(addr = 0x59f660, conv = thiscall, name = "CObject", expect = "6A FF 68 28 D2 83 00",
       unverified = "mvpi-world-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "the CObject destructor: releases model/txd/col refs, frees the physical's lists")]
pub fn object_destructor(this: GPtr<u8>) -> u32 {
    wr::<u32>(this, 0, OBJECT_VTABLE);
    if rd::<u32>(this, 0x140) & 0x30_0000 != 0 {
        let b = i32::from(rd::<i16>(this, 0x168));
        let slot = g::<i16>((SCRIPTS_FOR_BRAINS as u32).wrapping_add((b as u32).wrapping_mul(0x14)) as usize).read();
        set_mission_doesnt_require_model(i32::from(slot) + 0x6676);
        let f = rd::<u32>(this, 0x140) & 0xFFCF_FFFF;
        let i = u32::from(rd::<u16>(this, 0x168));
        wr::<u32>(this, 0x140, f);
        remove_from_waiting_for_script_brain_array(this, i);
        wr::<u16>(this, 0x168, 0xFFFF);
    }
    if rd::<u8>(this, 0x142) & 1 != 0 {
        let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).byte_add(isize::from(rd::<i16>(this, 0x22)) * 4).read();
        let col = rd::<GPtr<u8>>(mi, 0x14);
        col_store_remove_ref(i32::from(rd::<u8>(col, 0x28)));
    }
    let pool = g::<GPtr<u8>>(OBJECT_POOL).read();
    let i = pool_index(pool, this, OBJECT_SLOT);
    let flags = rd::<u32>(pool, 4);
    let h = i32::from(GPtr::<u8>::from_addr(flags.wrapping_add(i as u32) as usize).read()).wrapping_add(i << 8);
    radar_clear_blip_for_entity(3, h);
    let r = rd::<i16>(this, 0x14A);
    if r != -1 {
        model_info_remove_ref(g::<GPtr<u8>>(MODEL_INFO_PTRS).byte_add(isize::from(r) * 4).read());
    }
    let t = rd::<i16>(this, 0x16A);
    if t != -1 {
        txd_store_remove_ref(i32::from(t));
    }
    if rd::<u8>(this, 0x13C) == 3 {
        let n = g::<u16>(NO_TEMP_OBJECTS);
        if n.read() > 0 {
            n.write(n.read() - 1);
        }
    }
    remove_from_control_code_list_call(this);
    if i32::from(rd::<i16>(this, 0x22)) == i32::from(g::<u16>(MI_BRIDGE_LIGHTS).read()) {
        let e = rd::<GPtr<u8>>(this, 0x170);
        let m = rd::<GPtr<u8>>(e, 0x14);
        let p: GPtr<f32> = if m.is_null() { e.byte_add(4).cast() } else { m.byte_add(0x30).cast() };
        let (px, py) = (p.read(), p.wrapping_index(1).read());
        let h = g::<f32>(BRIDGE_HALF).read();
        let y1 = x(py).fadd_m(h).to_f32();
        let y0 = x(py).fsub_m(h).to_f32();
        let x1 = x(px).fadd_m(h).to_f32();
        let x0 = x(px).fsub_m(h).to_f32();
        path_set_links_bridge_lights(GPtr::from_addr(THE_PATHS), x0, x1, y0, y1, 0);
    }
    let fire = rd::<GPtr<u8>>(this, 0x164);
    if !fire.is_null() {
        fire_extinguish(fire);
    }
    physical_destructor(this)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_OBJECT_DESTRUCTOR,
    HOOK_OBJECT_NEW,
    HOOK_OBJECT_DELETE,
    HOOK_CAN_BE_DELETED,
    HOOK_SET_IS_STATIC,
    HOOK_SETUP_LIGHTING,
    HOOK_CUTSCENE_SETUP_LIGHTING,
    HOOK_REF_MODEL_INFO,
    HOOK_SET_REMAP_TEXTURE,
    HOOK_CAN_BE_TARGETTED,
    HOOK_SET_OBJECT_TARGETTABLE,
    HOOK_IS_OBJECT_POINTER_VALID_NOT_IN_WORLD,
    HOOK_IS_OBJECT_POINTER_VALID,
    HOOK_SET_MATRIX_FOR_TRAIN_CROSSING,
    HOOK_SET_RELATED_DUMMY,
    HOOK_REMOVE_LIGHTING,
    HOOK_CUTSCENE_REMOVE_LIGHTING,
    HOOK_TRY_TO_EXPLODE,
    HOOK_ADD_TO_CONTROL_CODE_LIST,
    HOOK_REMOVE_FROM_CONTROL_CODE_LIST,
    HOOK_OBJECT_SCALAR_DTOR,
    HOOK_HAND_OBJECT_DTOR,
    HOOK_HAND_OBJECT_SCALAR_DTOR,
    HOOK_DUMMY_OBJECT_DTOR,
    HOOK_DUMMY_OBJECT_SCALAR_DTOR,
    HOOK_GET_ROPE_HEIGHT,
    HOOK_SET_ROPE_HEIGHT,
    HOOK_GET_OBJECT_CARRIED_WITH_ROPE,
    HOOK_RELEASE_OBJECT_CARRIED_WITH_ROPE,
    HOOK_LOCK_DOOR,
    HOOK_OBJECT_NEW_AT,
    HOOK_CAN_BE_USED_TO_TAKE_COVER_BEHIND,
    HOOK_OBJECT_CREATE_RW_OBJECT,
    HOOK_DELETE_ALL_TEMP_OBJECTS,
    HOOK_DELETE_ALL_MISSION_OBJECTS,
    HOOK_DELETE_ALL_TEMP_OBJECTS_IN_AREA,
    HOOK_TRY_TO_FREE_UP_TEMP_OBJECTS,
    HOOK_RESET_DOOR_ANGLE,
    HOOK_GET_LIGHTING_FROM_COLLISION_BELOW,
    HOOK_OBJECT_CTOR,
    HOOK_SET_OBJECT_DATA,
    HOOK_SAVE,
    HOOK_LOAD,
    HOOK_TELEPORT,
    HOOK_UPDATE_FROM_OBJECT,
    HOOK_OBJECT_CTOR_MODEL,
    HOOK_CREATE_FROM_DUMMY,
    HOOK_GRAB_OBJECT_TO_CARRY_WITH_ROPE,
    HOOK_DUMMY_OBJECT_CTOR,
    HOOK_SETUP_CAR_PIPE_ATOMICS_FOR_CLUMP,
    HOOK_CUTSCENE_SET_MODEL_INDEX,
    HOOK_CREATE_FROM_MODEL,
    HOOK_OBJECT_INIT_PORT,
    HOOK_OBJECT_CTOR_FROM_DUMMY,
    HOOK_HAND_OBJECT_PROCESS_CONTROL,
];
