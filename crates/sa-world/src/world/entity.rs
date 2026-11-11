//! CEntity / CPlaceable / CDummy / CBuilding leaves (PORT-WORLD-2, helper A), plus the free helpers
//! they share (`TransformPoint`, `TransformVectors`, `CSimpleTransform::Invert`, `CBoundingBox`,
//! `CPtrListDoubleLink`) and the building / dummy pools' `operator new` / `delete`. Ported from the
//! 1.0 US bytes (gta-reversed `Entity.cpp`, `Placeable.cpp` as reading aids).
//!
//! `CPlaceable` (0x18): +0 vtable, +4 `CSimpleTransform` placement (x, y, z, heading), +0x14
//! `CMatrixLink*` (CMatrix: right +0, forward +0x10, up +0x20, position +0x30; link: +0x48 owner, +0x4C
//! prev, +0x50 next). `CEntity` (0x38): +0x18 `m_pRwObject` (`RwObject`: +4 parent frame), +0x1C flags
//! (bit 0 `bUsesCollision`, bit 2 `bIsStatic`, bit 8 `bIsBIGBuilding`, bit 18 `bIsStaticWaitingForCollision`,
//! ...), +0x22 model index (int16), +0x24 `m_pReferences` (`CReference`: +0 next, +4 `CEntity**`),
//! +0x36 type (bits 0-2) / status (bits 3-7). `CVehicle` +0x48B special col model index (int8).
//! `CPool` (0x14): +0 objects, +4 byte map (bit 7 free, bits 0-6 id), +8 size, +0xC first free.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{ftol32, g, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
}

/// `CModelInfo::ms_modelInfoPtrs`
pub const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CVehicle::m_aSpecialColModel` (`CColModel[4]`, 0x30 each)
const SPECIAL_COL_MODELS: usize = 0xC1CC78;
/// `CReferences::pEmptyList`
pub const REFS_EMPTY_LIST: usize = 0xBA1768;
/// `CPools::ms_pBuildingPool`, `ms_pObjectPool`, `ms_pDummyPool`
pub const BUILDING_POOL: usize = 0xB74498;
pub const OBJECT_POOL: usize = 0xB7449C;
pub const DUMMY_POOL: usize = 0xB744A0;
/// `TheCamera`
const THE_CAMERA: usize = 0xB6F028;
/// `CTimer::ms_fTimeStep`, `CTimer::m_snTimeInMilliseconds`
const TIME_STEP: usize = 0xB7CB5C;
const TIME_MS: usize = 0xB7CB84;
/// `CTimer::ms_fTimeStepNonClipped`
const TIME_STEP_NON_CLIPPED: usize = 0xB7CB58;
/// 0x858B38: 0.02; 0x863980: 0.0061328127 (2pi / 1024)
const F_0_02: f32 = f32::from_bits(0x3CA3_D70A);
const F_CRANE: f32 = f32::from_bits(0x3BC8_F5C3);
/// `CPools::ms_pPedPool`, `ms_pVehiclePool`
const PED_POOL: usize = 0xB74490;
const VEHICLE_POOL: usize = 0xB74494;
/// `gMatrixList`: the free list head node (+0x50 is its `next`, at 0xB74428)
const MATRIX_FREE_HEAD: usize = 0xB743D8;
/// `gMatrixList`: the head node `FreeStaticMatrix` links into (+0x50 at 0xB742D8)
const MATRIX_STATIC_HEAD: usize = 0xB74288;
/// `CEscalators::aEscalators` (`CEscalator[32]`, 0x150 each); +0x78 `m_bExist`, +0xA4 `m_pEntity`
const ESCALATORS: usize = 0xC6E9A8;
const ESCALATOR_SIZE: usize = 0x150;
const NUM_ESCALATORS: usize = 32;
/// 0x858B50: 0.0; 0x858C1C: -1.0
const F0: f32 = 0.0;
const FM1: f32 = -1.0;
/// 0x858B14: 1/32768
const F_1_32768: f32 = 3.051_757_8e-5;

orig! {
    /// `RpAtomicDestroy(RpAtomic*)` (0x749DC0)
    pub fn rp_atomic_destroy(a: GPtr<u8>) -> u32 = cdecl 0x749DC0 => unported;
    /// `RwFrameDestroy(RwFrame*)` (0x7F05A0)
    pub fn rw_frame_destroy(f: GPtr<u8>) -> u32 = cdecl 0x7F05A0 => unported;
    /// `RpClumpDestroy(RpClump*)` (0x74A310)
    pub fn rp_clump_destroy(c: GPtr<u8>) -> u32 = cdecl 0x74A310 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*)` (0x4D15E0)
    pub fn rp_anim_blend_clump_get_first_association(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D15E0 => unported;
    /// `RpAnimBlendClumpUpdateAnimations(RpClump*, float, bool)` (0x4D34F0)
    pub fn rp_anim_blend_clump_update_animations(c: GPtr<u8>, step: f32, visible: u32) = cdecl 0x4D34F0 => unported;
    /// `CMatrix::UpdateMatrix(RwMatrix*)` (0x59AD20)
    pub fn matrix_update_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD20 => unported;
    /// `CPhysical::AddToMovingList()` (0x542800)
    pub fn physical_add_to_moving_list(p: GPtr<u8>) = thiscall 0x542800 => unported;
    /// `CEntity::SetIsStatic(bool)` (vtable slot 0x10)
    pub fn entity_vset_is_static(e: GPtr<u8>, v: u8) = thiscall vslot 0x10 => unported;
    /// `CPtrNodeDoubleLink::AddToList(CPtrListDoubleLink*)` (0x5329A0)
    pub fn double_node_add_to_list(n: GPtr<u8>, l: GPtr<u8>) = thiscall 0x5329A0 => unported;
    /// `CBaseModelInfo::AddRef()` (0x4C4BA0)
    pub fn model_info_add_ref(mi: GPtr<u8>) = thiscall 0x4C4BA0 => unported;
    /// `CStreaming::AddEntity(CEntity*)` (0x409650)
    pub fn streaming_add_entity(e: GPtr<u8>) -> GPtr<u8> = cdecl 0x409650 => unported;
    /// `CEntity::CreateEffects()` (0x533790)
    pub fn entity_create_effects(e: GPtr<u8>) = thiscall 0x533790 => unported;
    /// `CVehicle::IsVehiclePointerValid(CVehicle*)` (0x6E38F0)
    pub fn is_vehicle_pointer_valid(e: GPtr<u8>) -> u8 = cdecl 0x6E38F0 => unported;
    /// `IsPedPointerValid(CPed*)` (0x5E8A30)
    pub fn is_ped_pointer_valid(e: GPtr<u8>) -> u8 = cdecl 0x5E8A30 => unported;
    /// `CObject::IsObjectPointerValid(CObject*)` (0x5A2C20)
    pub fn object_is_object_pointer_valid(e: GPtr<u8>) -> u8 = cdecl 0x5A2C20 => crate::world::object::is_object_pointer_valid;
    /// `CBaseModelInfo::RemoveRef()` (0x4C4BB0)
    pub fn model_info_remove_ref(mi: GPtr<u8>) = thiscall 0x4C4BB0 => unported;
    /// `CStreaming::RemoveEntity(CLink<CEntity*>*)` (0x409710)
    pub fn streaming_remove_entity(l: GPtr<u8>) = cdecl 0x409710 => unported;
    /// `CBaseModelInfo::GetModelType` (vtable slot 0x10)
    pub fn model_info_get_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
    /// `CEntity::DestroyEffects()` (0x533BF0)
    pub fn entity_destroy_effects(e: GPtr<u8>) = thiscall 0x533BF0 => unported;
    /// `RpMaterialUVAnimExists(const RpMaterial*)` (0x7CC530)
    pub fn rp_material_uv_anim_exists(m: GPtr<u8>) -> u32 = cdecl 0x7CC530 => unported;
    /// `RpMaterialUVAnimAddAnimTime(RpMaterial*, float)` (0x7CC4B0)
    pub fn rp_material_uv_anim_add_anim_time(m: GPtr<u8>, t: f32) -> GPtr<u8> = cdecl 0x7CC4B0 => unported;
    /// `RpMaterialUVAnimApplyUpdate(RpMaterial*)` (0x7CC110)
    pub fn rp_material_uv_anim_apply_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7CC110 => unported;
    /// `RwHelper::GetFirstAtomic(RpClump*)` (0x734820)
    pub fn rw_get_first_atomic(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `RpSkinGeometryGetSkin(RpGeometry*)` (0x7C7550)
    pub fn rp_skin_geometry_get_skin(g: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C7550 => unported;
    /// `RwHelper::GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40)
    pub fn rw_get_anim_hierarchy_from_skin_clump(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpHAnimHierarchyUpdateMatrices(RpHAnimHierarchy*)` (0x7C51D0)
    pub fn rp_h_anim_hierarchy_update_matrices(h: GPtr<u8>) -> u32 = cdecl 0x7C51D0 => unported;
    /// `CMatrix::CMatrix(RwMatrix*, bool owner)` (0x59C050)
    pub fn matrix_ctor_attach(m: GPtr<u8>, rw: GPtr<u8>, owner: u32) = thiscall 0x59C050 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    pub fn matrix_update_rw_attached(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix` (0x59ACD0)
    pub fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float, const RwMatrix*)` (0x420C40)
    pub fn camera_is_sphere_visible(cam: GPtr<u8>, c: GPtr<u8>, r: f32, m: GPtr<u8>) -> u8 = thiscall 0x420C40 => unported;
    /// `CRect::StretchToPoint(float, float)` (0x5327F0)
    pub fn rect_stretch_to_point(r: GPtr<u8>, px: f32, py: f32) = thiscall 0x5327F0 => unported;
    /// `CEntity::SetModelIndexNoCreate` (vtable slot 0x18)
    pub fn entity_vset_model_index_no_create(e: GPtr<u8>, m: u32) = thiscall vslot 0x18 => unported;
    /// `CEntity::CreateRwObject` (vtable slot 0x1C)
    pub fn entity_vcreate_rw_object(e: GPtr<u8>) = thiscall vslot 0x1C => unported;
    /// `CGlass::IsObjectGlass(CEntity*)` (0x46A760)
    pub fn is_object_glass(e: GPtr<u8>) -> u8 = cdecl 0x46A760 => unported;
    /// `CTrafficLights::IsMITrafficLight(int32)` (0x49D5A0)
    pub fn is_mi_traffic_light(m: i32) -> u8 = cdecl 0x49D5A0 => unported;
    /// `CBaseModelInfo::AsAtomicModelInfoPtr` (vtable slot 4)
    pub fn model_info_as_atomic(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CTagManager::AddTag(CEntity*)` (0x49CC90)
    pub fn tag_manager_add_tag(e: GPtr<u8>) = cdecl 0x49CC90 => unported;
    /// `operator delete` / CRT `free` (0x8214BD)
    pub fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `eh vector destructor iterator(void*, uint32 size, int32 count, void (__thiscall*)(void*))` (0x821E02, stdcall)
    pub fn eh_vector_destructor_iterator(p: GPtr<u8>, size: u32, count: i32, dtor: u32) = stdcall 0x821E02 => unported;
    /// `RpClumpForAllAtomics(RpClump*, cb, void*)` (0x749B70)
    pub fn rp_clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    pub fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CEntity::GetBoundRect()` (vtable slot 0x24, hidden return pointer)
    pub fn entity_get_bound_rect(e: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CEntity::Add(const CRect&)` (vtable slot 4)
    pub fn entity_add_rect(e: GPtr<u8>, r: GPtr<u8>) = thiscall vslot 0x4 => unported;
    /// `CPtrListSingleLink::Flush()` (0x552400)
    pub fn single_link_flush(l: GPtr<u8>) = thiscall 0x552400 => unported;
    /// `CPtrListDoubleLink::Flush()` (0x552470)
    pub fn double_link_flush(l: GPtr<u8>) = thiscall 0x552470 => unported;
    /// `CEntity::~CEntity` body (0x535E90)
    pub fn entity_destructor(e: GPtr<u8>) = thiscall 0x535E90 => unported;
    /// `CMatrixLinkList::Init(int32)` (0x54F0D0)
    pub fn matrix_link_list_init(l: GPtr<u8>, n: i32) = thiscall 0x54F0D0 => unported;
    /// `CMatrixLinkList::AddToList1()` (0x54E9D0)
    pub fn matrix_link_list_add_to_list1(l: GPtr<u8>) -> GPtr<u8> = thiscall 0x54E9D0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, returns `out`)
    pub fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CPointLights::RemoveLightsAffectingObject()` (0x6FFFE0)
    pub fn remove_lights_affecting_object() = cdecl 0x6FFFE0 => unported;
    /// `Light::SetAmbientColours()` (0x735D30)
    pub fn set_ambient_colours() = cdecl 0x735D30 => unported;
    /// `Light::DeActivateDirectional()` (0x735C70)
    pub fn deactivate_directional() = cdecl 0x735C70 => unported;
    /// `Light::ActivateDirectional()` (0x735C80)
    pub fn activate_directional() = cdecl 0x735C80 => unported;
    /// `CPointLights::GenerateLightsAffectingObject(const CVector*, float*, CEntity*)` (0x6FFBB0, ST0)
    pub fn generate_lights_affecting_object(pos: GPtr<u8>, out: GPtr<u8>, e: GPtr<u8>) -> f64 = cdecl 0x6FFBB0 => unported;
    /// `Light::SetLightColoursForPedsCarsAndObjects(float)` (0x735D90)
    pub fn set_light_colours_for_peds_cars_and_objects(l: f32) = cdecl 0x735D90 => unported;
    /// `CPtrNodeSingleLink::operator new` (0x552380)
    pub fn single_node_new(size: u32) -> GPtr<u8> = cdecl 0x552380 => unported;
    /// `CPtrNodeDoubleLink::operator new` (0x5523C0)
    pub fn double_node_new(size: u32) -> GPtr<u8> = cdecl 0x5523C0 => unported;
    /// `CPtrNodeDoubleLink::operator delete` (0x5523D0)
    pub fn double_node_delete(p: GPtr<u8>) = cdecl 0x5523D0 => unported;
    /// `CEntity::GetIsOnScreen()` (0x534540)
    pub fn get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `RpGeometryForAllMaterials(RpGeometry*, cb, void*)` (0x74C790)
    pub fn rp_geometry_for_all_materials(g: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x74C790 => unported;
    /// `CGlass::AskForObjectToBeRenderedInGlass(CEntity*)` (0x71ACD0)
    pub fn ask_for_object_to_be_rendered_in_glass(e: GPtr<u8>) = cdecl 0x71ACD0 => unported;
    /// `CStreaming::RemoveModel(int32)` (0x4089A0)
    pub fn streaming_remove_model(i: i32) = cdecl 0x4089A0 => unported;
    /// `CEntity::DeleteRwObject` (vtable slot 0x20)
    pub fn entity_delete_rw_object(e: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020)
    pub fn matrix_set_rotate_z_only(m: GPtr<u8>, h: f32) = thiscall 0x59B020 => unported;
    /// `CMatrix::SetRotate(float, float, float)` (0x59B120)
    pub fn matrix_set_rotate(m: GPtr<u8>, ax: f32, ay: f32, az: f32) = thiscall 0x59B120 => unported;
    /// `RwFrameUpdateObjects(RwFrame*)` (0x7F0910)
    pub fn rw_frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => unported;
    /// `CEscalator::SwitchOff` (0x717860)
    pub fn escalator_switch_off(e: GPtr<u8>) = thiscall 0x717860 => crate::world::escalators::switch_off;
    /// `CBuildingPool::New` (0x403FA0, thiscall on the pool)
    pub fn building_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x403FA0 => unported;
    /// `CBaseModelInfo::Get2dEffect(int32)` (0x4C4C70)
    pub fn model_info_get_2d_effect(mi: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x4C4C70 => unported;
    /// `GetPedAttractorManager()` (0x5EE190)
    pub fn get_ped_attractor_manager() -> GPtr<u8> = cdecl 0x5EE190 => unported;
    /// `CPedAttractorManager::HasEmptySlot(const C2dEffect*, const CEntity*)` (0x5EBB00)
    pub fn attractor_has_empty_slot(m: GPtr<u8>, e: GPtr<u8>, ent: GPtr<u8>) -> u8 = thiscall 0x5EBB00 => unported;
}

/// `CPlaceable::GetPosition()`: matrix + 0x30 when there is a matrix, else +4.
pub fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// The `CColModel*` `CEntity::GetColModel` returns (inlined in several callers).
pub fn col_model_of(this: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(this, 0x36) & 7 == 2 {
        let i = rd::<i8>(this, 0x48B);
        if i > -1 {
            return g::<u8>(SPECIAL_COL_MODELS.wrapping_add((i as usize) * 0x30));
        }
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
    rd::<GPtr<u8>>(mi, 0x14)
}

/// Slot index of `p` in `pool` (`(p - objects) / size`, the compiler's magic-number signed division).
pub fn pool_index(pool: GPtr<u8>, p: GPtr<u8>, size: i32) -> i32 {
    (p.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32 / size
}

/// `CPool::Delete` body (inlined in the pools' `operator delete`): marks the slot free and lowers
/// the first-free hint.
pub fn pool_delete(pool: GPtr<u8>, p: GPtr<u8>, size: i32) {
    let i = pool_index(pool, p, size);
    let b = rd::<GPtr<u8>>(pool, 4).wrapping_index(i);
    b.write(b.read() | 0x80);
    if i < rd::<i32>(pool, 0xC) {
        wr::<i32>(pool, 0xC, i);
    }
}

/// `CEntity::AsXxx` identity cast (0x404350, `mov eax, ecx; ret`, 3 bytes). Ported, unhookable.
#[sa_core::native(addr = 0x404350, conv = thiscall, name = "FUN_00404350")]
pub fn as_xxx(this: GPtr<u8>) -> GPtr<u8> {
    this
}

/// `CPlaceable::GetMatrix` (0x41BD80, `mov eax, [ecx+0x14]; ret`, 4 bytes). Ported, unhookable.
#[sa_core::native(addr = 0x41BD80, conv = thiscall, name = "FUN_0041bd80")]
pub fn placeable_get_matrix_raw(this: GPtr<u8>) -> GPtr<u8> {
    rd(this, 0x14)
}

/// `CEntity::GetModelIndex` (stub 0x404420 → 0x1563A40): `movsx eax, word [ecx+0x22]`.
#[hook(addr = 0x404420, conv = thiscall, name = "thunk_FUN_01563a40", expect = "E9 1B F6 15 01", this = "CEntity")]
pub fn get_model_index(this: GPtr<u8>) -> i32 {
    i32::from(rd::<i16>(this, 0x22))
}

/// `CEntity::GetStatus` (stub 0x4043E0 → 0x15700F0): `movzx eax, byte [ecx+0x36]; shr eax, 3`.
#[hook(addr = 0x4043E0, conv = thiscall, name = "thunk_FUN_015700f0", expect = "E9 0B BD 16 01", this = "CEntity")]
pub fn get_status(this: GPtr<u8>) -> u32 {
    u32::from(rd::<u8>(this, 0x36)) >> 3
}

/// `CEntity::TreatAsPlayerForCollisions` (0x541F70): status bits all zero (`STATUS_PLAYER`).
#[hook(addr = 0x541F70, conv = thiscall, name = "CEntity::TreatAsPlayerForCollisions", expect = "8A 41 36 24 F8", this = "CEntity")]
pub fn treat_as_player_for_collisions(this: GPtr<u8>) -> u8 {
    u8::from(rd::<u8>(this, 0x36) & 0xF8 == 0)
}

/// `CEntity::SetIsStatic(bool)` (0x403E20, `ret 4`): flags bit 2.
#[hook(addr = 0x403E20, conv = thiscall, name = "CEntity::SetIsStatic", expect = "0F B6 44 24 04", this = "CEntity")]
pub fn set_is_static(this: GPtr<u8>, v: u8) {
    let f = rd::<u32>(this, 0x1C);
    wr::<u32>(this, 0x1C, f ^ (((u32::from(v) << 2) ^ f) & 4));
}

/// `CPlaceable::SetHeading(float)` (0x43E0C0, `ret 4`): `CMatrix::SetRotateZOnly` (tail jump) with a
/// matrix, else the placement heading.
#[hook(addr = 0x43E0C0, conv = thiscall, name = "CPlaceable::SetHeading", expect = "8B C1 8B 48 14", this = "CPlaceable")]
pub fn set_heading(this: GPtr<u8>, h: f32) {
    let m = rd::<GPtr<u8>>(this, 0x14);
    if !m.is_null() {
        matrix_set_rotate_z_only(m, h);
        return;
    }
    wr::<f32>(this, 0x10, h);
}

/// `CEntity::UpdateRwFrame` (0x532B00): `RwFrameUpdateObjects(parent frame)` when there is an RwObject.
#[hook(addr = 0x532B00, conv = thiscall, name = "CEntity::UpdateRwFrame", expect = "8B 41 18 85 C0", this = "CEntity")]
pub fn update_rw_frame(this: GPtr<u8>) {
    let o = rd::<GPtr<u8>>(this, 0x18);
    if !o.is_null() {
        rw_frame_update_objects(rd(o, 4));
    }
}

/// `CEntity::SetupBigBuilding` (0x533150): clears `bUsesCollision`, sets `bIsBIGBuilding` and bit 16
/// (`bStreamingDontDelete`), and the model info flag 0x20 at +0x12.
#[hook(addr = 0x533150, conv = thiscall, name = "CEntity::SetupBigBuilding", expect = "8B 41 1C 83 E0 FE", this = "CEntity")]
pub fn setup_big_building(this: GPtr<u8>) {
    wr::<u32>(this, 0x1C, (rd::<u32>(this, 0x1C) & !1) | 0x10100);
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
    wr::<u8>(mi, 0x12, rd::<u8>(mi, 0x12) | 0x20);
}

/// `CEntity::GetModellingMatrix` (stub 0x46A2D0 → 0x1565110): parent frame + 0x10, or null.
#[hook(addr = 0x46A2D0, conv = thiscall, name = "CEntity::GetModellingMatrix", expect = "E9 3B AE 0F 01", this = "CEntity")]
pub fn get_modelling_matrix(this: GPtr<u8>) -> GPtr<u8> {
    let o = rd::<GPtr<u8>>(this, 0x18);
    if o.is_null() {
        return GPtr::NULL;
    }
    rd::<GPtr<u8>>(o, 4).byte_add(0x10)
}

/// `CPlaceable::CPlaceable` (0x54F1E0): vtable 0x863C40, placement and matrix zero; returns `this`.
#[hook(addr = 0x54F1E0, conv = thiscall, name = "FUN_0054f1e0", expect = "8B C1 33 C9 C7 00 40 3C 86 00", this = "CPlaceable")]
pub fn placeable_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0x863C40);
    for o in [4, 8, 0xC, 0x10, 0x14] {
        wr::<u32>(this, o, 0);
    }
    this
}

/// `atan2(-m.forward.x, m.forward.y)` (`fld [m+0x10]; fchs; fld [m+0x14]; fpatan`).
pub fn matrix_heading(m: GPtr<u8>) -> X87 {
    x(rd(m, 0x10)).fchs().fpatan(x(rd(m, 0x14)))
}

/// `CPlaceable::GetHeading` (0x441DB0): the matrix heading, else the placement heading (ST0).
/// Not registered: returns the raw FPATAN ST0, modelled at 53 bits (MATH-FPTAN); L2 compares the full ST0.
pub fn get_heading(this: GPtr<u8>) -> X87 {
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() { x(rd(this, 0x10)) } else { matrix_heading(m) }
}

/// `CEntity::GetIsStatic` (stub 0x4633E0 → 0x15619F0): `bIsStatic || bIsStaticWaitingForCollision`.
#[hook(addr = 0x4633E0, conv = thiscall, name = "CEntity::GetIsStatic", expect = "E9 0B E6 0F 01", this = "CEntity")]
pub fn get_is_static(this: GPtr<u8>) -> u32 {
    let f = rd::<u32>(this, 0x1C);
    u32::from(f & 4 != 0 || f & 0x40000 != 0)
}

/// `CEntity::GetIsTypePhysical` (0x4DA030): type 2..4 (vehicle, ped, object).
#[hook(addr = 0x4DA030, conv = thiscall, name = "CEntity::GetIsTypePhysical", expect = "8A 41 36 24 07", this = "CEntity")]
pub fn get_is_type_physical(this: GPtr<u8>) -> u32 {
    let t = rd::<u8>(this, 0x36) & 7;
    u32::from(t > 1 && t < 5)
}

/// `CPlaceable::SetPosn(const CVector&)` (0x4241C0, `ret 4`): the three components are loaded (x87,
/// sNaN quieted) before any store.
#[hook(addr = 0x4241C0, conv = thiscall, name = "CPlaceable::SetPosn-vector", expect = "8B 44 24 04 D9 40 08", this = "CPlaceable")]
pub fn set_posn_vector(this: GPtr<u8>, v: GPtr<f32>) {
    let v = v.cast::<u8>();
    let z = x(rd(v, 8));
    let y = x(rd(v, 4));
    let xx = x(rd(v, 0));
    let d = pos_ptr(this);
    wr::<f32>(d, 0, xx.to_f32());
    wr::<f32>(d, 4, y.to_f32());
    wr::<f32>(d, 8, z.to_f32());
}

/// `CPlaceable::SetPosn(float, float, float)` (stub 0x420B80 → 0x1566E30, `ret 0xC`): dword copies.
#[hook(addr = 0x420B80, conv = thiscall, name = "CPlaceable::SetPosn-xyz", expect = "E9 AB 62 14 01", this = "CPlaceable")]
pub fn set_posn_xyz(this: GPtr<u8>, px: u32, py: u32, pz: u32) {
    let d = pos_ptr(this);
    wr::<u32>(d, 0, px);
    wr::<u32>(d, 4, py);
    wr::<u32>(d, 8, pz);
}

/// `CDummy::operator delete(void*)` (0x5326F0): `CPools::ms_pDummyPool->Delete`.
#[hook(addr = 0x5326F0, conv = cdecl, name = "CDummy::operator delete", expect = "8B 4C 24 04 56")]
pub fn dummy_delete(p: GPtr<u8>) {
    pool_delete(g::<GPtr<u8>>(DUMMY_POOL).read(), p, 0x38);
}

/// `CBuilding::operator delete(void*)` (stub 0x4040A0 → 0x156C060): `ms_pBuildingPool->Delete`.
#[hook(addr = 0x4040A0, conv = cdecl, name = "delete", expect = "E9 BB 7F 16 01")]
pub fn building_delete(p: GPtr<u8>) {
    pool_delete(g::<GPtr<u8>>(BUILDING_POOL).read(), p, 0x38);
}

/// `CBuilding::operator new(uint32)` (0x404090): `ms_pBuildingPool->New()` (tail jump, size ignored).
#[hook(addr = 0x404090, conv = cdecl, name = "new", expect = "8B 0D 98 44 B7 00")]
pub fn building_new(_size: u32) -> GPtr<u8> {
    building_pool_new(g::<GPtr<u8>>(BUILDING_POOL).read())
}

/// `CEntity::GetColModel` (0x535300; the vehicle branch goes through the protection chunk at 0x401644
/// and back): a vehicle's special col model when its index is >= 0, else the model info's.
#[hook(addr = 0x535300, conv = thiscall, name = "CEntity::GetColModel", expect = "8A 41 36 24 07", this = "CEntity")]
pub fn get_col_model(this: GPtr<u8>) -> GPtr<u8> {
    col_model_of(this)
}

/// `CEntity::RemoveEscalatorsForEntity` (0x717900): switches off (and clears `m_bExist` of) every
/// existing escalator whose entity is `this`.
#[hook(addr = 0x717900, conv = thiscall, name = "CEntity::RemoveEscalatorsForEntity", expect = "56 57 8B F9 BE 20 EA C6 00", this = "CEntity")]
pub fn remove_escalators_for_entity(this: GPtr<u8>) {
    for i in 0..NUM_ESCALATORS {
        let e = g::<u8>(ESCALATORS + i * ESCALATOR_SIZE);
        if rd::<u8>(e, 0x78) != 0 && rd::<GPtr<u8>>(e, 0xA4) == this {
            escalator_switch_off(e);
            wr::<u8>(e, 0x78, 0);
        }
    }
}

/// `CEntity::GetDistanceFromCentreOfMassToBaseOfModel` (0x536BE0): `-colModel->bbox.min.z` (ST0).
#[hook(addr = 0x536BE0, conv = thiscall, name = "CEntity::GetDistanceFromCentreOfMassToBaseOfModel", expect = "8A 41 36 24 07", this = "CEntity")]
pub fn get_distance_from_centre_of_mass_to_base_of_model(this: GPtr<u8>) -> X87 {
    x(rd(col_model_of(this), 8)).fchs()
}

/// Unlinks `node` (whose predecessor's `next` field is at `prev`) and pushes it on
/// `CReferences::pEmptyList` with a null `ref`.
fn free_reference(prev: GPtr<u8>, node: GPtr<u8>) -> GPtr<u8> {
    let next = rd::<GPtr<u8>>(node, 0);
    wr::<GPtr<u8>>(prev, 0, next);
    wr::<GPtr<u8>>(node, 0, g::<GPtr<u8>>(REFS_EMPTY_LIST).read());
    g::<GPtr<u8>>(REFS_EMPTY_LIST).write(node);
    wr::<u32>(node, 4, 0);
    next
}

/// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`): frees the first reference node whose
/// `ref` is the argument.
#[hook(addr = 0x571A00, conv = thiscall, name = "CEntity::CleanUpOldReference-", expect = "8B 41 24 83 C1 24", this = "CEntity")]
pub fn clean_up_old_reference(this: GPtr<u8>, r: GPtr<u8>) {
    let mut prev = this.byte_add(0x24);
    let mut n = rd::<GPtr<u8>>(prev, 0);
    while !n.is_null() {
        if rd::<GPtr<u8>>(n, 4) == r {
            free_reference(prev, n);
            return;
        }
        prev = n;
        n = rd(n, 0);
    }
}

/// `CPlaceable::GetRightDirection(CVector* out)` (0x41CC70, `ret 4`): matrix right, else
/// `(cos h, sin h, 0)`. Returns `out`.
#[hook(addr = 0x41CC70, conv = thiscall, name = "CPlaceable::GetRightDirection", expect = "8B 51 14 85 D2", this = "CPlaceable")]
pub fn get_right_direction(this: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> {
    let o = out.cast::<u8>();
    let m = rd::<GPtr<u8>>(this, 0x14);
    if !m.is_null() {
        for k in [0, 4, 8] {
            wr::<u32>(o, k, rd::<u32>(m, k));
        }
        return out;
    }
    let s = x(rd(this, 0x10)).fsin();
    let c = x(rd(this, 0x10)).fcos();
    wr::<u32>(o, 8, 0);
    wr::<f32>(o, 0, c.to_f32());
    wr::<f32>(o, 4, s.to_f32());
    out
}

/// `CPlaceable::GetTopDirection(CVector* out)` (0x41CCB0, `ret 4`): matrix forward (+0x10), else
/// `(-sin h, cos h, 0)`. Returns `out`.
#[hook(addr = 0x41CCB0, conv = thiscall, name = "CPlaceable::GetTopDirection", expect = "8B 41 14 85 C0", this = "CPlaceable")]
pub fn get_top_direction(this: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> {
    let o = out.cast::<u8>();
    let m = rd::<GPtr<u8>>(this, 0x14);
    if !m.is_null() {
        for k in [0, 4, 8] {
            wr::<u32>(o, k, rd::<u32>(m, 0x10 + k));
        }
        return out;
    }
    let c = x(rd(this, 0x10)).fcos();
    let s = x(rd(this, 0x10)).fsin();
    wr::<u32>(o, 8, 0);
    wr::<f32>(o, 0, s.fchs().to_f32());
    wr::<f32>(o, 4, c.to_f32());
    out
}

/// `CPlaceable::GetAtDirection(CVector* out)` (0x50E420, `ret 4`): matrix up (+0x20), else (0, 0, 1).
#[hook(addr = 0x50E420, conv = thiscall, name = "CPlaceable::GetAtDirection", expect = "8B 41 14 85 C0", this = "CPlaceable")]
pub fn get_at_direction(this: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> {
    let o = out.cast::<u8>();
    let m = rd::<GPtr<u8>>(this, 0x14);
    if !m.is_null() {
        for k in [0, 4, 8] {
            wr::<u32>(o, k, rd::<u32>(m, 0x20 + k));
        }
        return out;
    }
    wr::<u32>(o, 0, 0);
    wr::<u32>(o, 4, 0);
    wr::<u32>(o, 8, 0x3F80_0000);
    out
}

/// `TransformPoint(CVector& out, const CSimpleTransform& t, const CVector& in)` (0x54ECE0, cdecl):
/// rotation by `t.heading` plus translation, in the original's load/store order (`out` may alias `in`).
#[hook(addr = 0x54ECE0, conv = cdecl, name = "TransformPoint", expect = "8B 44 24 08 D9 40 0C")]
pub fn transform_point(out: GPtr<f32>, t: GPtr<f32>, v: GPtr<f32>) {
    let (o, t, v) = (out.cast::<u8>(), t.cast::<u8>(), v.cast::<u8>());
    let c = x(rd(t, 0xC)).fcos();
    let s = x(rd(t, 0xC)).fsin();
    let a = c.fmul_m(rd(v, 0));
    let b = s.fmul_m(rd(v, 4));
    wr::<f32>(o, 0, a.fsub(b).fadd_m(rd(t, 0)).to_f32());
    let cy = c.fmul_m(rd(v, 4));
    let sx = s.fmul_m(rd(v, 0));
    wr::<f32>(o, 4, cy.fadd(sx).fadd_m(rd(t, 4)).to_f32());
    wr::<f32>(o, 8, x(rd(t, 8)).fadd_m(rd(v, 8)).to_f32());
}

/// `TransformVectors(CVector* out, int32 n, const CSimpleTransform& t, const CVector* in)` (0x54EE30,
/// cdecl): rotation only, `cos`/`sin` stored as f32 first; z copied; `dec / jne` count loop.
#[hook(addr = 0x54EE30, conv = cdecl, name = "TransformVectors", expect = "51 8B 44 24 10")]
pub fn transform_vectors(out: GPtr<f32>, n: i32, t: GPtr<f32>, v: GPtr<f32>) {
    let t = t.cast::<u8>();
    let c = x(rd(t, 0xC)).fcos().to_f32();
    let s = x(rd(t, 0xC)).fsin().to_f32();
    if n == 0 {
        return;
    }
    let (mut o, mut i) = (out.cast::<u8>(), v.cast::<u8>());
    let mut n = n;
    loop {
        let a = x(c).fmul_m(rd(i, 0));
        n = n.wrapping_sub(1);
        let b = x(s).fmul_m(rd(i, 4));
        wr::<f32>(o, 0, a.fsub(b).to_f32());
        let a = x(s).fmul_m(rd(i, 0));
        let b = x(c).fmul_m(rd(i, 4));
        wr::<f32>(o, 4, a.fadd(b).to_f32());
        wr::<u32>(o, 8, rd::<u32>(i, 8));
        o = o.byte_add(0xC);
        i = i.byte_add(0xC);
        if n == 0 {
            break;
        }
    }
}

/// `CSimpleTransform::Invert(const CSimpleTransform& src)` (0x54EF90, thiscall on the destination,
/// `ret 4`): original store order (`this` may alias `src`).
#[hook(addr = 0x54EF90, conv = thiscall, name = "CSimpleTransform::Invert", expect = "8B 44 24 04 D9 40 0C", this = "CSimpleTransform")]
pub fn simple_transform_invert(this: GPtr<u8>, src: GPtr<u8>) {
    let c = x(rd(src, 0xC)).fcos();
    let s = x(rd(src, 0xC)).fsin();
    let a = c.fmul_m(rd(src, 0)).fchs();
    let b = s.fmul_m(rd(src, 4));
    wr::<f32>(this, 0, a.fsub(b).to_f32());
    let a = s.fmul_m(rd(src, 0));
    let b = c.fmul_m(rd(src, 4));
    wr::<f32>(this, 4, a.fsub(b).to_f32());
    wr::<f32>(this, 8, x(rd(src, 8)).fchs().to_f32());
    wr::<f32>(this, 0xC, x(rd(src, 0xC)).fchs().to_f32());
}

/// `CEntity::PruneReferences` (0x571A90): frees every reference node whose `*ref` is no longer `this`.
#[hook(addr = 0x571A90, conv = thiscall, name = "CEntity::PruneReferences", expect = "8B 41 24 85 C0", this = "CEntity")]
pub fn prune_references(this: GPtr<u8>) {
    let mut prev = this.byte_add(0x24);
    let mut n = rd::<GPtr<u8>>(prev, 0);
    while !n.is_null() {
        if rd::<GPtr<u8>>(rd::<GPtr<u8>>(n, 4), 0) == this {
            prev = n;
            n = rd(n, 0);
        } else {
            n = free_reference(prev, n);
        }
    }
}

/// `CEntity::ResolveReferences` (0x571A40): nulls every `*ref` still pointing at `this`, then nulls the
/// nodes' `ref` and splices the whole list onto `CReferences::pEmptyList`.
#[hook(addr = 0x571A40, conv = thiscall, name = "CEntity::ResolveReferences", expect = "8B 41 24 56 33 F6", this = "CEntity")]
pub fn resolve_references(this: GPtr<u8>) {
    let mut n = rd::<GPtr<u8>>(this, 0x24);
    while !n.is_null() {
        let r = rd::<GPtr<u8>>(n, 4);
        if rd::<GPtr<u8>>(r, 0) == this {
            wr::<u32>(r, 0, 0);
        }
        n = rd(n, 0);
    }
    let head = rd::<GPtr<u8>>(this, 0x24);
    if head.is_null() {
        return;
    }
    let mut last = head;
    wr::<u32>(last, 4, 0);
    while !rd::<GPtr<u8>>(last, 0).is_null() {
        last = rd(last, 0);
        wr::<u32>(last, 4, 0);
    }
    wr::<GPtr<u8>>(last, 0, g::<GPtr<u8>>(REFS_EMPTY_LIST).read());
    g::<GPtr<u8>>(REFS_EMPTY_LIST).write(rd(this, 0x24));
    wr::<u32>(this, 0x24, 0);
}

/// `CPlaceable::SetOrientation(float, float, float)` (0x439A80, `ret 0xC`): with a matrix,
/// `SetRotate(x, y, z)` keeping the position (`saved + m.pos`, x87); else the heading = `z`.
#[hook(addr = 0x439A80, conv = thiscall, name = "CPlaceable::SetOrientation-xyz", expect = "83 EC 0C 56 8B F1", this = "CPlaceable")]
pub fn set_orientation(this: GPtr<u8>, ax: f32, ay: f32, az: f32) {
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        wr::<f32>(this, 0x10, az);
        return;
    }
    let p = [rd::<f32>(m, 0x30), rd::<f32>(m, 0x34), rd::<f32>(m, 0x38)];
    matrix_set_rotate(m, ax, ay, az);
    let m = rd::<GPtr<u8>>(this, 0x14);
    for (k, v) in p.into_iter().enumerate() {
        let o = 0x30 + 4 * k as isize;
        wr::<f32>(m, o, x(v).fadd_m(rd(m, o)).to_f32());
    }
}

/// Moves matrix link `m` from its list to after the head node `head` (whose `next` is at +0x50).
fn relink_matrix(m: GPtr<u8>, head: usize) {
    let prev = rd::<GPtr<u8>>(m, 0x4C);
    let next = rd::<GPtr<u8>>(m, 0x50);
    wr::<GPtr<u8>>(next, 0x4C, prev);
    let next = rd::<GPtr<u8>>(m, 0x50);
    wr::<GPtr<u8>>(rd::<GPtr<u8>>(m, 0x4C), 0x50, next);
    let h = g::<GPtr<u8>>(head + 0x50);
    wr::<GPtr<u8>>(m, 0x50, h.read());
    wr::<GPtr<u8>>(h.read(), 0x4C, m);
    wr::<u32>(m, 0x4C, head as u32);
    h.write(m);
}

/// `CPlaceable::RemoveMatrix` (0x54F3B0): placement = matrix position (y, z through x87) and heading;
/// the link loses its owner and moves to the free list of `gMatrixList`.
#[hook(addr = 0x54F3B0, conv = thiscall, name = "CPlaceable::RemoveMatrix", expect = "83 EC 10 8B 41 14", this = "CPlaceable")]
pub fn remove_matrix(this: GPtr<u8>) {
    let m = rd::<GPtr<u8>>(this, 0x14);
    let py = x(rd(m, 0x34)).to_f32();
    let px = rd::<u32>(m, 0x30);
    let pz = x(rd(m, 0x38)).to_f32();
    let h = matrix_heading(m).to_f32();
    wr::<u32>(this, 4, px);
    wr::<f32>(this, 8, py);
    wr::<f32>(this, 0xC, pz);
    wr::<f32>(this, 0x10, h);
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(m, 0x48, 0);
    relink_matrix(m, MATRIX_FREE_HEAD);
}

/// `CPlaceable::FreeStaticMatrix` (0x54F010): moves the matrix link after the node at 0xB74288.
#[hook(addr = 0x54F010, conv = thiscall, name = "CPlaceable::FreeStaticMatrix", expect = "8B 41 14 8B 48 50", this = "CPlaceable")]
pub fn free_static_matrix(this: GPtr<u8>) {
    relink_matrix(rd(this, 0x14), MATRIX_STATIC_HEAD);
}

/// `CDummyPool::New` (0x532630, thiscall on the pool): `CPool<CDummy>::New`. Scans from `firstFree + 1`
/// with one wrap (at `== size`); marks the slot used and bumps its 7-bit id. Null when full.
#[hook(addr = 0x532630, conv = thiscall, name = "CDummyPool::New", expect = "8B 51 08 56 32 C0", this = "CPool_int32")]
pub fn dummy_pool_new(pool: GPtr<u8>) -> GPtr<u8> {
    pool_new(pool, 0x38)
}

/// `CPool<T>::New` body as compiled in `CDummyPool::New` (slot size `size`).
pub fn pool_new(pool: GPtr<u8>, size: i32) -> GPtr<u8> {
    let mut wrapped = false;
    loop {
        let i = rd::<i32>(pool, 0xC).wrapping_add(1);
        wr::<i32>(pool, 0xC, i);
        if i == rd::<i32>(pool, 8) {
            wr::<i32>(pool, 0xC, 0);
            if wrapped {
                return GPtr::NULL;
            }
            wrapped = true;
        }
        let i = rd::<i32>(pool, 0xC);
        if rd::<GPtr<u8>>(pool, 4).wrapping_index(i).read() & 0x80 != 0 {
            break;
        }
    }
    let i = rd::<i32>(pool, 0xC);
    let b = rd::<GPtr<u8>>(pool, 4).wrapping_index(i);
    b.write(b.read() & 0x7F);
    let v = b.read();
    b.write((v & 0x80) | (v.wrapping_add(1) & 0x7F));
    rd::<GPtr<u8>>(pool, 0).byte_add(rd::<i32>(pool, 0xC).wrapping_mul(size) as isize)
}

/// `CPtrListDoubleLink::CPtrListDoubleLink` (0x563180): head null; returns `this`.
#[hook(addr = 0x563180, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "8B C1 C7 00 00 00 00 00")]
pub fn ptr_list_double_link_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    this
}

/// `CEntity::SetCompAlphaCB(RpMaterial*, void* alpha)` (0x533280, cdecl): material colour alpha (+7)
/// = low byte of the data; returns the material.
#[hook(addr = 0x533280, conv = cdecl, name = "CEntity::SetCompAlphaCB", expect = "8B 44 24 04 8A 4C 24 08")]
pub fn set_comp_alpha_cb(mat: GPtr<u8>, alpha: u32) -> GPtr<u8> {
    wr::<u8>(mat, 7, alpha as u8);
    mat
}

/// `CDummy::IsDummyPointerValid(CDummy*)` (0x532730, cdecl): non-null with a slot index in
/// `0..=size` (original: `jle`, one past the end accepted, the byte map is not checked).
#[hook(addr = 0x532730, conv = cdecl, name = "CDummy::IsDummyPointerValid", expect = "8B 44 24 04 85 C0")]
pub fn is_dummy_pointer_valid(p: GPtr<u8>) -> u8 {
    if p.is_null() {
        return 0;
    }
    let pool = g::<GPtr<u8>>(DUMMY_POOL).read();
    let i = pool_index(pool, p, 0x38);
    u8::from(i >= 0 && i <= rd::<i32>(pool, 8))
}

/// `CBuilding::IsBuildingPointerValid(CBuilding*)` (0x4040E0, cdecl): non-null, slot in range and
/// not free.
#[hook(addr = 0x4040E0, conv = cdecl, name = "CBuilding::IsBuildingPointerValid", expect = "8B 44 24 04 85 C0")]
pub fn is_building_pointer_valid(p: GPtr<u8>) -> u8 {
    if p.is_null() {
        return 0;
    }
    let pool = g::<GPtr<u8>>(BUILDING_POOL).read();
    let i = pool_index(pool, p, 0x38);
    if i < 0 || i >= rd::<i32>(pool, 8) {
        return 0;
    }
    u8::from((rd::<GPtr<u8>>(pool, 4).wrapping_index(i).read() as i8) >= 0)
}

/// `CPlaceable::GetRoll` (0x420B30): `atan2(right.z, ±|right.xy|)` (negated when `up.z < 0`); 0.0
/// without a matrix.
/// Not registered: returns the raw FPATAN ST0, modelled at 53 bits (MATH-FPTAN); L2 compares the full ST0.
pub fn get_roll(this: GPtr<u8>) -> X87 {
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        return x(F0);
    }
    let ry = x(rd(m, 4));
    let rx = x(rd(m, 0));
    let mut xy = rx.fmul(rx).fadd(ry.fmul(ry)).fsqrt();
    // fcomp 0.0; test ah,5; jp: multiply by -1 only when Less
    if !x(rd(m, 0x28)).fcom(x(F0)).test_parity(0x05) {
        xy = xy.fmul_m(FM1);
    }
    x(rd(m, 8)).fpatan(xy)
}

/// `CBoundingBox::IsPointWithin(const CVector&)` (0x49C300, `ret 4`): `min <= p <= max` per axis
/// (NaN → false).
#[hook(addr = 0x49C300, conv = thiscall, name = "CBoundingBox::IsPointWithin", preserve = "ecx", expect = "8B 54 24 04 D9 02", this = "CBoundingBox")]
pub fn bounding_box_is_point_within(this: GPtr<u8>, p: GPtr<f32>) -> u8 {
    let p = p.cast::<u8>();
    for k in [0, 4, 8] {
        if x(rd(p, k)).fcom(x(rd(this, k))).test_any(0x01) {
            return 0;
        }
    }
    for k in [0, 4, 8] {
        if x(rd(p, k)).fcom(x(rd(this, 0xC + k))).test_parity(0x41) {
            return 0;
        }
    }
    1
}

/// Shared by both `IsWithinArea` overloads: orders the corner pair (when `a > b`: `b` slot gets
/// `fstp(a)`, `a` slot the raw `b`).
fn order(a: f32, b: f32) -> (f32, f32) {
    if x(a).fcom(x(b)).test_any(0x41) { (a, b) } else { (b, x(a).to_f32()) }
}

/// `lo <= v <= hi` as `test ah,1 / jne` then `test ah,0x41 / jp`.
fn within(v: f32, lo: f32, hi: f32) -> bool {
    !x(v).fcom(x(lo)).test_any(0x01) && !x(v).fcom(x(hi)).test_parity(0x41)
}

/// `CPlaceable::IsWithinArea(float x1, float y1, float x2, float y2)` (0x54F200, `ret 0x10`).
#[hook(addr = 0x54F200, conv = thiscall, name = "CPlaceable::IsWithinArea-xy", expect = "8B 41 14 83 EC 0C", this = "CPlaceable")]
pub fn is_within_area_xy(this: GPtr<u8>, x1: f32, y1: f32, x2: f32, y2: f32) -> u8 {
    let p = pos_ptr(this);
    let (px, py) = (rd::<f32>(p, 0), rd::<f32>(p, 4));
    let (x1, x2) = order(x1, x2);
    let (y1, y2) = order(y1, y2);
    u8::from(within(px, x1, x2) && within(py, y1, y2))
}

/// `CPlaceable::IsWithinArea(float x1, float y1, float z1, float x2, float y2, float z2)` (0x54F2B0,
/// `ret 0x18`).
#[hook(addr = 0x54F2B0, conv = thiscall, name = "CPlaceable::IsWithinArea-xyz", expect = "8B 41 14 83 EC 0C", this = "CPlaceable")]
pub fn is_within_area_xyz(this: GPtr<u8>, x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32) -> u8 {
    let p = pos_ptr(this);
    let (px, py, pz) = (rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8));
    let (x1, x2) = order(x1, x2);
    let (y1, y2) = order(y1, y2);
    let (z1, z2) = order(z1, z2);
    u8::from(within(px, x1, x2) && within(py, y1, y2) && within(pz, z1, z2))
}

/// `CEntity::TestCollision(bool)` (0x403E70, `xor al,al; ret 4`): false.
#[hook(addr = 0x403E70, conv = thiscall, name = "CEntity::TestCollision", expect = "32 C0 C2 04 00", this = "CEntity")]
pub fn test_collision(_this: GPtr<u8>, _apply: u8) -> u8 {
    0
}

/// `CEntity::SpecialEntityCalcCollisionSteps(bool&, bool&)` (0x403EA0, `mov al,1; ret 8`): true.
#[hook(addr = 0x403EA0, conv = thiscall, name = "CEntity::SpecialEntityCalcCollisionSteps", expect = "B0 01 C2 08 00", this = "CEntity")]
pub fn special_entity_calc_collision_steps(_this: GPtr<u8>, _a: GPtr<u8>, _b: GPtr<u8>) -> u8 {
    1
}

/// `CEntity::DoesNotCollideWithFlyers` (0x532D40): model info flags (+0x12) special type 1 or 2 (bits
/// 11-14 == 0x800 / 0x1000), else flag bit 10.
#[hook(addr = 0x532D40, conv = thiscall, name = "CEntity::DoesNotCollideWithFlyers", expect = "0F BF 41 22 8B 0C 85 C8 B0 A9 00", this = "CEntity")]
pub fn does_not_collide_with_flyers(this: GPtr<u8>) -> u8 {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
    let f = rd::<u16>(mi, 0x12);
    let t = f & 0x7800;
    if t == 0x800 || t == 0x1000 {
        return 1;
    }
    ((f >> 10) & 1) as u8
}

/// `CEntity::GetRandom2dEffect(int32 type, bool mustBeFree)` (0x533410, `ret 8`): collects (up to 32)
/// the model's 2d effects of `type` (with `mustBeFree`, only those whose attractor has an empty slot)
/// and returns one picked with `rand()`; null when none.
#[hook(addr = 0x533410, conv = thiscall, name = "CEntity::GetRandom2dEffect", expect = "81 EC 88 00 00 00", this = "CEntity")]
pub fn get_random_2d_effect(this: GPtr<u8>, ty: i32, must_be_free: u8) -> GPtr<u8> {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
    let mut found = [GPtr::<u8>::NULL; 32];
    let mut n = 0i32;
    let mut i = 0i32;
    if rd::<u8>(mi, 0xD) == 0 {
        return GPtr::NULL;
    }
    loop {
        let e = model_info_get_2d_effect(mi, i);
        if i32::from(rd::<u8>(e, 0xC)) == ty
            && (must_be_free == 0 || attractor_has_empty_slot(get_ped_attractor_manager(), e, this) != 0)
            && n < 32
        {
            found[n as usize] = e;
            n += 1;
        }
        i += 1;
        if i >= i32::from(rd::<u8>(mi, 0xD)) {
            break;
        }
    }
    if n == 0 {
        return GPtr::NULL;
    }
    let r = sa_core::orig::at_site(0x533499, sa_core::crt::rand) & 0xFFFF;
    let k = ftol32(xi(r).fmul_m(F_1_32768).fmul(xi(n)));
    found[k as usize]
}

/// `CBuilding::~CBuilding` (0x404180, `jmp CEntity::~CEntity`; non-HOODLUM jump, expect + 1 byte).
#[hook(addr = 0x404180, conv = thiscall, name = "CBuilding::~CBuilding", expect = "E9 0B 1D 13 00 90", this = "CBuilding")]
pub fn building_dtor(this: GPtr<u8>) {
    entity_destructor(this);
}

/// `CDummy::~CDummy` (0x532560): vtable 0x8638C0, then `CEntity::~CEntity` (tail jump).
#[hook(addr = 0x532560, conv = thiscall, name = "FUN_00532560", expect = "C7 01 C0 38 86 00", this = "CDummy")]
pub fn dummy_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x8638C0);
    entity_destructor(this);
}

/// `CDummyPed::~CDummyPed` (0x5DE300): vtable 0x86C198, then `CDummy::~CDummy`.
#[hook(addr = 0x5DE300, conv = thiscall, name = "FUN_005de300", expect = "C7 01 98 C1 86 00", this = "CDummyPed")]
pub fn dummy_ped_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86C198);
    dummy_dtor(this);
}

/// The scalar deleting destructors' pool release (inlined `operator delete`): returns `this`.
pub fn scalar_delete(this: GPtr<u8>, flags: u32, pool: usize, size: i32) -> GPtr<u8> {
    if flags & 1 != 0 {
        pool_delete(g::<GPtr<u8>>(pool).read(), this, size);
    }
    this
}

/// `CBuilding` scalar deleting destructor (0x404130, `ret 4`).
#[hook(addr = 0x404130, conv = thiscall, name = "CBuilding::`scalar deleting destructor'", expect = "56 57 8B F9 E8 47 00 00 00", this = "CBuilding")]
pub fn building_scalar_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    building_dtor(this);
    scalar_delete(this, flags, BUILDING_POOL, 0x38)
}

/// `CDummy` scalar deleting destructor (0x532770, `ret 4`).
#[hook(addr = 0x532770, conv = thiscall, name = "CDummy::`scalar deleting destructor'", expect = "56 57 8B F9 E8 E7 FD FF FF", this = "CDummy")]
pub fn dummy_scalar_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dummy_dtor(this);
    scalar_delete(this, flags, DUMMY_POOL, 0x38)
}

/// `CEntity::CEntity` (0x532A90): `CPlaceable` base, vtable 0x863928, type/status byte 0x20, flags
/// 0x8000080, model -1, random seed (+0x20) = `rand()`; the other fields zero. Returns `this`.
#[hook(addr = 0x532A90, conv = thiscall, name = "CEntity::Constructor", expect = "53 56 8B F1 E8 47 C7 01 00", this = "CEntity")]
pub fn entity_ctor(this: GPtr<u8>) -> GPtr<u8> {
    placeable_ctor(this);
    wr::<u32>(this, 0, 0x863928);
    wr::<u8>(this, 0x36, 0x20);
    wr::<u32>(this, 0x1C, 0x0800_0080);
    wr::<u16>(this, 0x2C, 0);
    wr::<i16>(this, 0x22, -1);
    wr::<u32>(this, 0x18, 0);
    wr::<u8>(this, 0x2F, 0);
    wr::<u8>(this, 0x2E, 0);
    let r = sa_core::orig::at_site(0x532AC4, sa_core::crt::rand);
    wr::<u16>(this, 0x20, r as u16);
    wr::<u32>(this, 0x24, 0);
    wr::<u32>(this, 0x28, 0);
    wr::<u8>(this, 0x34, 0);
    wr::<u8>(this, 0x35, 0);
    wr::<u32>(this, 0x30, 0);
    this
}

/// `CBuilding::CBuilding` (stub 0x403E00 → 0x156CFD0): `CEntity`, type 1 (`& 0xF9 | 1`),
/// `bUsesCollision`, vtable 0x8585C8. Returns `this`.
#[hook(addr = 0x403E00, conv = thiscall, name = "CBuilding", expect = "E9 CB 91 16 01", this = "CBuilding")]
pub fn building_ctor(this: GPtr<u8>) -> GPtr<u8> {
    entity_ctor(this);
    wr::<u8>(this, 0x36, (rd::<u8>(this, 0x36) & 0xF9) | 1);
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 1);
    wr::<u32>(this, 0, 0x8585C8);
    this
}

/// `CDummy::CDummy` (0x532540): `CEntity`, type bits `& 0xFD | 5`, vtable 0x8638C0. Returns `this`.
#[hook(addr = 0x532540, conv = thiscall, name = "CDummy::CDummy", expect = "56 8B F1 E8 48 05 00 00", this = "CDummy")]
pub fn dummy_ctor(this: GPtr<u8>) -> GPtr<u8> {
    entity_ctor(this);
    wr::<u8>(this, 0x36, (rd::<u8>(this, 0x36) & 0xFD) | 5);
    wr::<u32>(this, 0, 0x8638C0);
    this
}

/// `CDummyPed::CDummyPed` (0x5DE2E0): `CDummy`, vtable 0x86C198. Returns `this`.
#[hook(addr = 0x5DE2E0, conv = thiscall, name = "FUN_005de2e0", expect = "56 8B F1 E8 58 42 F5 FF", this = "CDummyPed")]
pub fn dummy_ped_ctor(this: GPtr<u8>) -> GPtr<u8> {
    dummy_ctor(this);
    wr::<u32>(this, 0, 0x86C198);
    this
}

/// `CPlaceable::~CPlaceable` (0x54F490): vtable 0x863C40; with a matrix, `RemoveMatrix` and
/// `--numMatrices` (0xB74238); the matrix pointer is then set to 0xB74240 (original).
#[hook(addr = 0x54F490, conv = thiscall, name = "FUN_0054f490", expect = "56 8B F1 8B 46 14", this = "CPlaceable")]
pub fn placeable_dtor(this: GPtr<u8>) {
    let m = rd::<GPtr<u8>>(this, 0x14);
    wr::<u32>(this, 0, 0x863C40);
    if !m.is_null() {
        remove_matrix(this);
        let n = g::<i32>(0xB74238);
        n.write(n.read().wrapping_sub(1));
    }
    wr::<u32>(this, 0x14, 0xB74240);
}

/// `CPlaceable::InitMatrixArray` (0x54F3A0): `gMatrixList.Init(900)`.
#[hook(addr = 0x54F3A0, conv = cdecl, name = "CPlaceable::InitMatrixArray", expect = "68 84 03 00 00")]
pub fn init_matrix_array() {
    matrix_link_list_init(g(MATRIX_STATIC_HEAD), 900);
}

/// `CPlaceable::AllocateMatrix` (0x54F560): takes the first free link (moved after the head at
/// 0xB74288) or, with none free (next == tail node 0xB7442C), removes the matrix of the owner of the
/// last used link (0xB74328) and uses `AddToList1`. Returns the matrix (EAX).
#[hook(addr = 0x54F560, conv = thiscall, name = "CPlaceable::AllocateMatrix", expect = "56 8B F1 8B 46 14", this = "CPlaceable")]
pub fn allocate_matrix(this: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(this, 0x14);
    if !m.is_null() {
        return m;
    }
    let f = g::<GPtr<u8>>(MATRIX_FREE_HEAD + 0x50).read();
    let m = if f.addr() != 0xB7442C {
        relink_matrix(f, MATRIX_STATIC_HEAD);
        f
    } else {
        let last = g::<GPtr<u8>>(0xB74328).read();
        remove_matrix(rd(last, 0x48));
        matrix_link_list_add_to_list1(g(MATRIX_STATIC_HEAD))
    };
    wr::<GPtr<u8>>(m, 0x48, this);
    wr::<GPtr<u8>>(this, 0x14, m);
    m
}

/// `CEntity::TransformFromObjectSpace(CVector& out, const CVector& offset)` (0x5334F0, `ret 8`):
/// `out = matrix * offset`, or `TransformPoint(out, placement, offset)`. EAX is what the original
/// leaves: the z bits of the product, or the placement address.
#[hook(addr = 0x5334F0, conv = thiscall, name = "CEntity::TransformFromObjectSpace-ref", expect = "8B 41 14 83 EC 18", this = "CEntity")]
pub fn transform_from_object_space_ref(this: GPtr<u8>, out: GPtr<u8>, off: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        transform_point(out.cast(), this.byte_add(4).cast(), off.cast());
        return this.byte_add(4);
    }
    let mut tmp = [rd::<u32>(off, 0), rd::<u32>(off, 4), rd::<u32>(off, 8)];
    let mut res = [0u32; 3];
    let r = matrix_mul_vec(GPtr::from_raw(res.as_mut_ptr()).cast(), m, GPtr::from_raw(tmp.as_mut_ptr()).cast());
    wr::<u32>(out, 0, rd::<u32>(r, 0));
    wr::<u32>(out, 4, rd::<u32>(r, 4));
    let z = rd::<u32>(r, 8);
    wr::<u32>(out, 8, z);
    GPtr::from_addr(z as usize)
}

/// `CEntity::TransformFromObjectSpace(const CVector& offset)` (0x533560, returns the vector through
/// the hidden pointer `out`, `ret 8`).
#[hook(addr = 0x533560, conv = thiscall, name = "CEntity::TransformFromObjectSpace-ptr", expect = "8B 41 14 83 EC 24", this = "CEntity")]
pub fn transform_from_object_space_ptr(this: GPtr<u8>, out: GPtr<u8>, off: GPtr<u8>) -> GPtr<u8> {
    let mut v = [0u32; 3];
    let vp = GPtr::from_raw(v.as_mut_ptr()).cast::<u8>();
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        transform_point(vp.cast(), this.byte_add(4).cast(), off.cast());
    } else {
        let mut tmp = [rd::<u32>(off, 0), rd::<u32>(off, 4), rd::<u32>(off, 8)];
        let mut res = [0u32; 3];
        let r = matrix_mul_vec(GPtr::from_raw(res.as_mut_ptr()).cast(), m, GPtr::from_raw(tmp.as_mut_ptr()).cast());
        for k in [0, 4, 8] {
            wr::<u32>(vp, k, rd::<u32>(r, k));
        }
    }
    for k in [0, 4, 8] {
        wr::<u32>(out, k, rd::<u32>(vp, k));
    }
    out
}

/// `CEntity::GetBoundCentre(CVector& out)` (0x534290, `ret 4`): `TransformFromObjectSpace(out,
/// modelInfo->colModel->boundSphere.centre)` (model info's col model, not `GetColModel`); EAX as it.
#[hook(addr = 0x534290, conv = thiscall, name = "CEntity::GetBoundCentre-ref", expect = "0F BF 41 22 8B 14 85 C8 B0 A9 00", this = "CEntity")]
pub fn get_bound_centre_ref(this: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
    let c = rd::<GPtr<u8>>(mi, 0x14).byte_add(0x18);
    transform_from_object_space_ref(this, out, c)
}

/// `CEntity::GetBoundCentre()` (0x534250, hidden return pointer, `ret 4`): returns `out`.
#[hook(addr = 0x534250, conv = thiscall, name = "CEntity::GetBoundCentre-vec", expect = "0F BF 41 22 8B 14 85 C8 B0 A9 00", this = "CEntity")]
pub fn get_bound_centre_vec(this: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> {
    let mut v = [0u32; 3];
    let vp = GPtr::from_raw(v.as_mut_ptr()).cast::<u8>();
    get_bound_centre_ref(this, vp);
    for k in [0, 4, 8] {
        wr::<u32>(out, k, rd::<u32>(vp, k));
    }
    out
}

/// `CEntity::RemoveLighting(bool reset)` (0x553370, `ret 4`): when set, ambient colours, directional
/// light off, point lights removed.
#[hook(addr = 0x553370, conv = thiscall, name = "CEntity::RemoveLighting", expect = "8A 44 24 04 84 C0", this = "CEntity")]
pub fn remove_lighting(_this: GPtr<u8>, reset: u8) {
    if reset != 0 {
        set_ambient_colours();
        deactivate_directional();
        remove_lights_affecting_object();
    }
}

/// `CEntity::SetupLighting` (0x553DC0): for `bLightObject` entities, directional light on and
/// `SetLightColoursForPedsCarsAndObjects(GenerateLightsAffectingObject(pos, null, this) * 0.5)`; true.
#[hook(addr = 0x553DC0, conv = thiscall, name = "CEntity::SetupLighting", expect = "56 8B F1 F7 46 1C 00 00 00 10", this = "CEntity")]
pub fn entity_setup_lighting(this: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0x1C) & 0x1000_0000 == 0 {
        return 0;
    }
    activate_directional();
    let v = generate_lights_affecting_object(pos_ptr(this), GPtr::NULL, this);
    set_light_colours_for_peds_cars_and_objects(X87::from_f64_exact(v).fmul_m(0.5).to_f32());
    1
}

/// `CPtrListSingleLink::AddItem(void*)` (0x5335E0, `ret 4`): a new node at the head. A failed
/// allocation writes through null+4 (original). Returns the node.
#[hook(addr = 0x5335E0, conv = thiscall, name = "CPtrListSingleLink::AddItem", expect = "56 6A 08 8B F1")]
pub fn single_link_add_item(this: GPtr<u8>, item: GPtr<u8>) -> GPtr<u8> {
    let n = single_node_new(8);
    if !n.is_null() {
        wr::<GPtr<u8>>(n, 0, item);
    }
    wr::<GPtr<u8>>(n, 4, rd(this, 0));
    wr::<GPtr<u8>>(this, 0, n);
    n
}

/// `CPtrListDoubleLink::AddItem(void*)` (0x533670, `ret 4`): a new node (item, next, prev) at the
/// head. Returns the node.
#[hook(addr = 0x533670, conv = thiscall, name = "CPtrListDoubleLink::AddItem", expect = "56 6A 0C 8B F1")]
pub fn double_link_add_item(this: GPtr<u8>, item: GPtr<u8>) -> GPtr<u8> {
    let n = double_node_new(0xC);
    if !n.is_null() {
        wr::<GPtr<u8>>(n, 0, item);
    }
    link_double_head(this, n);
    n
}

/// Pushes double-link node `n` at the head `*head` (prev null, next = old head, old head's prev = n).
pub fn link_double_head(head: GPtr<u8>, n: GPtr<u8>) {
    wr::<u32>(n, 8, 0);
    wr::<GPtr<u8>>(n, 4, rd(head, 0));
    let h = rd::<GPtr<u8>>(head, 0);
    if !h.is_null() {
        wr::<GPtr<u8>>(h, 8, n);
    }
    wr::<GPtr<u8>>(head, 0, n);
}

/// Unlinks double-link node `n` from the list whose head is `*head` (the head re-pointed only when it
/// is `n`), without freeing it.
pub fn unlink_double(head: GPtr<u8>, n: GPtr<u8>) {
    if rd::<GPtr<u8>>(head, 0) == n {
        wr::<GPtr<u8>>(head, 0, rd(n, 4));
    }
    let p = rd::<GPtr<u8>>(n, 8);
    if !p.is_null() {
        wr::<GPtr<u8>>(p, 4, rd(n, 4));
    }
    let nx = rd::<GPtr<u8>>(n, 4);
    if !nx.is_null() {
        wr::<GPtr<u8>>(nx, 8, rd(n, 8));
    }
}

/// `CPtrListDoubleLink::DeleteItem(void*)` (0x5336B0, `ret 4`): frees the first node holding `item`.
#[hook(addr = 0x5336B0, conv = thiscall, name = "CPtrListDoubleLink::DeleteItem", expect = "8B 11 8B C2 85 C0")]
pub fn double_link_delete_item(this: GPtr<u8>, item: GPtr<u8>) {
    let mut n = rd::<GPtr<u8>>(this, 0);
    while !n.is_null() {
        if rd::<GPtr<u8>>(n, 0) == item {
            unlink_double(this, n);
            double_node_delete(n);
            return;
        }
        n = rd(n, 4);
    }
}

/// `CEntity::IsVisible` (0x536BC0): RwObject and `bIsVisible` (flags bit 7), then `GetIsOnScreen`.
#[hook(addr = 0x536BC0, conv = thiscall, name = "CEntity::IsVisible", expect = "8B 41 18 85 C0", this = "CEntity")]
pub fn is_visible(this: GPtr<u8>) -> u8 {
    if rd::<GPtr<u8>>(this, 0x18).is_null() || (rd::<u8>(this, 0x1C) as i8) >= 0 {
        return 0;
    }
    get_is_on_screen(this)
}

/// `CEntity::SetAtomicAlpha(RpAtomic*, int32 alpha)` (0x533290, cdecl): geometry flag
/// `rpGEOMETRYMODULATEMATERIALCOLOR` and every material's alpha (`SetCompAlphaCB`). Returns the atomic.
#[hook(addr = 0x533290, conv = cdecl, name = "CEntity::SetAtomicAlpha", expect = "56 8B 74 24 08")]
pub fn set_atomic_alpha(atomic: GPtr<u8>, alpha: u32) -> GPtr<u8> {
    let geo = rd::<GPtr<u8>>(atomic, 0x18);
    wr::<u32>(geo, 8, rd::<u32>(geo, 8) | 0x40);
    rp_geometry_for_all_materials(geo, 0x533280, alpha);
    atomic
}

/// `CEntity::PreRenderForGlassWindow` (0x533240): unless the model is special type 5 (0x2800),
/// `CGlass::AskForObjectToBeRenderedInGlass(this)` and `bIsVisible` cleared.
#[hook(addr = 0x533240, conv = thiscall, name = "CEntity::PreRenderForGlassWindow", expect = "56 8B F1 0F BF 46 22", this = "CEntity")]
pub fn pre_render_for_glass_window(this: GPtr<u8>) {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
    if rd::<u16>(mi, 0x12) & 0x7800 == 0x2800 {
        return;
    }
    ask_for_object_to_be_rendered_in_glass(this);
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & 0xFFFF_FF7F);
}

/// `CBuilding::ReplaceWithNewModel(int32)` (0x403EC0, `ret 4`): `DeleteRwObject()`, removes the old
/// model when its ref count (+8) is 0, then stores the new index.
#[hook(addr = 0x403EC0, conv = thiscall, name = "CBuilding::ReplaceWithNewModel", expect = "56 8B F1 8B 06", this = "CBuilding")]
pub fn replace_with_new_model(this: GPtr<u8>, model: i32) {
    entity_delete_rw_object(this);
    let old = i32::from(rd::<i16>(this, 0x22));
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(old).read();
    if rd::<u16>(mi, 8) == 0 {
        streaming_remove_model(old);
    }
    wr::<u16>(this, 0x22, model as u16);
}

/// `CEntity::RegisterReference(CEntity** ref)` (0x571B70, `ret 4`): not for plain buildings (type 1
/// without flags 0x4400000 and with +0x2E clear); nothing when `ref` is already registered. Without a
/// free `CReference`, prunes the references of the used peds, vehicles, then objects (pool slots from
/// the last down) until one is free; none → nothing. The node is pushed at the head of the list.
#[hook(addr = 0x571B70, conv = thiscall, name = "CEntity::RegisterReference-", expect = "55 8B E9 8A 45 36", this = "CEntity")]
pub fn register_reference(this: GPtr<u8>, r: GPtr<u8>) {
    if rd::<u8>(this, 0x36) & 7 == 1 && rd::<u32>(this, 0x1C) & 0x0440_0000 == 0 && rd::<u8>(this, 0x2E) == 0 {
        return;
    }
    let mut n = rd::<GPtr<u8>>(this, 0x24);
    while !n.is_null() {
        if rd::<GPtr<u8>>(n, 4) == r {
            return;
        }
        n = rd(n, 0);
    }
    let empty = g::<GPtr<u8>>(REFS_EMPTY_LIST);
    let mut e = empty.read();
    if e.is_null() {
        'pools: for (pool, size) in [(PED_POOL, 0x7C4u32), (VEHICLE_POOL, 0xA18), (OBJECT_POOL, 0x19C)] {
            let p = g::<GPtr<u8>>(pool).read();
            let mut i = rd::<u32>(p, 8);
            while i != 0 {
                let b = rd::<GPtr<u8>>(p, 4).byte_add(i as isize - 1).read();
                i -= 1;
                if (b as i8) >= 0 {
                    let ent = GPtr::<u8>::from_addr(rd::<u32>(p, 0).wrapping_add(i.wrapping_mul(size)) as usize);
                    if !ent.is_null() {
                        prune_references(ent);
                        e = empty.read();
                        if !e.is_null() {
                            break 'pools;
                        }
                    }
                }
            }
        }
        if e.is_null() {
            return;
        }
    }
    empty.write(rd(e, 0));
    wr::<GPtr<u8>>(e, 0, rd(this, 0x24));
    wr::<GPtr<u8>>(this, 0x24, e);
    wr::<GPtr<u8>>(e, 4, r);
}

/// `CEntity` scalar deleting destructor (0x536C20, `ret 4`): `~CEntity`, then `operator delete`.
#[hook(addr = 0x536C20, conv = thiscall, name = "CEntity::`scalar deleting destructor'", expect = "56 8B F1 E8 68 F2 FF FF", this = "CEntity")]
pub fn entity_scalar_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    entity_destructor(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `CDummyPed` scalar deleting destructor (0x5DE310, `ret 4`): `~CDummyPed`, then the dummy pool.
#[hook(addr = 0x5DE310, conv = thiscall, name = "FUN_005de310", expect = "56 8B F1 E8 E8 FF FF FF", this = "CDummyPed")]
pub fn dummy_ped_scalar_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    dummy_ped_dtor(this);
    if flags & 1 != 0 {
        dummy_delete(this);
    }
    this
}

/// `CPlaceable` scalar deleting destructor (0x54F5D0, catalogue `CPlaceable::~CPlaceable`, `ret 4`):
/// the destructor body inlined, then `operator delete`.
#[hook(addr = 0x54F5D0, conv = thiscall, name = "CPlaceable::~CPlaceable", expect = "56 8B F1 8B 46 14", this = "CPlaceable")]
pub fn placeable_scalar_dtor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    placeable_dtor(this);
    if flags & 1 != 0 {
        crt_free(this);
    }
    this
}

/// `CPlaceable::ShutdownMatrixArray` (0x54EFD0): destroys and frees the matrix link array (0xB74480,
/// `eh vector destructor iterator` with `~CMatrixLink`) and clears the pointer.
#[hook(addr = 0x54EFD0, conv = cdecl, name = "CPlaceable::ShutdownMatrixArray", expect = "A1 80 44 B7 00")]
pub fn shutdown_matrix_array() {
    let p = g::<GPtr<u8>>(0xB74480);
    let a = p.read();
    if !a.is_null() {
        eh_vector_destructor_iterator(a, 0x54, rd(a, -4), 0x54E8E0);
        crt_free(a.byte_add(-4));
    }
    p.write(GPtr::NULL);
}

/// `CEntity::SetRwObjectAlpha(int32)` (0x5332C0, `ret 4`): atomic (type 1) → geometry flag and
/// material alphas; clump (type 2) → `SetAtomicAlpha` for every atomic.
#[hook(addr = 0x5332C0, conv = thiscall, name = "CEntity::SetRwObjectAlpha", expect = "8B 41 18 85 C0", this = "CEntity")]
pub fn set_rw_object_alpha(this: GPtr<u8>, alpha: u32) {
    let o = rd::<GPtr<u8>>(this, 0x18);
    if o.is_null() {
        return;
    }
    match rd::<u8>(o, 0) {
        1 => {
            let geo = rd::<GPtr<u8>>(o, 0x18);
            wr::<u32>(geo, 8, rd::<u32>(geo, 8) | 0x40);
            rp_geometry_for_all_materials(geo, 0x533280, alpha);
        }
        2 => {
            rp_clump_for_all_atomics(o, 0x533290, alpha);
        }
        _ => {}
    }
}

/// `CPlaceable::GetMatrix` (0x411990): allocates the matrix (and fills it from the placement,
/// `CSimpleTransform::UpdateMatrix`) when there is none; returns it.
#[hook(addr = 0x411990, conv = thiscall, name = "CPlaceable::GetMatrix", expect = "56 8B F1 8B 46 14", this = "CPlaceable")]
pub fn get_matrix(this: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(this, 0x14).is_null() {
        allocate_matrix(this);
        simple_transform_update_matrix(this.byte_add(4), rd(this, 0x14));
    }
    rd(this, 0x14)
}

/// `CEntity::Add()` (0x533020): `Add(GetBoundRect())` through the vtable (slots 0x24 and 4).
#[hook(addr = 0x533020, conv = thiscall, name = "CEntity::Add-void", expect = "83 EC 10 56 8B F1", this = "CEntity")]
pub fn add_void(this: GPtr<u8>) {
    let mut rect = [0u32; 4];
    let rp = GPtr::from_raw(rect.as_mut_ptr()).cast::<u8>();
    entity_get_bound_rect(this, rp);
    entity_add_rect(this, rp);
}

/// `CPtrListSingleLink::~CPtrListSingleLink` (0x43E1A0, `jmp Flush`; non-HOODLUM jump, expect + 1).
#[hook(addr = 0x43E1A0, conv = thiscall, name = "thunk_FUN_00552400", expect = "E9 5B 42 11 00 90")]
pub fn single_link_dtor(this: GPtr<u8>) {
    single_link_flush(this);
}

/// `CPtrListDoubleLink::~CPtrListDoubleLink` (0x5630F0, `jmp Flush`; non-HOODLUM jump, expect + 1).
#[hook(addr = 0x5630F0, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "E9 7B F3 FE FF 90")]
pub fn double_link_dtor(this: GPtr<u8>) {
    double_link_flush(this);
}

/// The model info's col model bounding sphere centre (+0x18) and radius (+0x24) (`GetIsTouching`,
/// `GetIsOnScreen` read the model info's, not `GetColModel`).
fn mi_col(this: GPtr<u8>) -> GPtr<u8> {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
    rd(mi, 0x14)
}

/// `CEntity::GetIsTouching(const CVector& pos, float radius)` (0x5344B0, `ret 8`; the model info load
/// goes through the protection chunk at 0x40122A and back): `(r + radius)^2 > |c - pos|^2`, the sum
/// as `(dz^2 + dx^2) + dy^2` (EAX 0/1).
#[hook(addr = 0x5344B0, conv = thiscall, name = "CEntity::GetIsTouching-vec", expect = "83 EC 0C 56 8B F1", this = "CEntity")]
pub fn get_is_touching_vec(this: GPtr<u8>, pos: GPtr<u8>, radius: f32) -> u32 {
    let mut c = [0u32; 3];
    let cp = GPtr::from_raw(c.as_mut_ptr()).cast::<u8>();
    transform_from_object_space_ref(this, cp, mi_col(this).byte_add(0x18));
    let r = x(rd(mi_col(this), 0x24)).fadd_m(radius);
    let dx = x(rd(cp, 0)).fsub_m(rd(pos, 0));
    let dy = x(rd(cp, 4)).fsub_m(rd(pos, 4));
    let dz = x(rd(cp, 8)).fsub_m(rd(pos, 8));
    let s = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    u32::from(!r.fmul(r).fcom(s).test_any(0x41))
}

/// `CEntity::GetIsTouching(CEntity* other)` (0x5343F0, `ret 4`): `(r_other + r_this)^2 > |c - c_other|^2`,
/// the sum as `(dz^2 + dy^2) + dx^2` (EAX 0/1).
#[hook(addr = 0x5343F0, conv = thiscall, name = "CEntity::GetIsTouching-ent", expect = "83 EC 18 56 8B F1", this = "CEntity")]
pub fn get_is_touching_ent(this: GPtr<u8>, other: GPtr<u8>) -> u32 {
    let mut c = [0u32; 3];
    let mut o = [0u32; 3];
    let cp = GPtr::from_raw(c.as_mut_ptr()).cast::<u8>();
    let op = GPtr::from_raw(o.as_mut_ptr()).cast::<u8>();
    transform_from_object_space_ref(this, cp, mi_col(this).byte_add(0x18));
    transform_from_object_space_ref(other, op, mi_col(other).byte_add(0x18));
    let r = x(rd(mi_col(other), 0x24)).fadd_m(rd(mi_col(this), 0x24));
    let dx = x(rd(cp, 0)).fsub_m(rd(op, 0));
    let dy = x(rd(cp, 4)).fsub_m(rd(op, 4));
    let dz = x(rd(cp, 8)).fsub_m(rd(op, 8));
    let s = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    u32::from(!r.fmul(r).fcom(s).test_any(0x41))
}

/// `CEntity::GetIsOnScreen` (0x534540): `TheCamera.IsSphereVisible(centre, radius, mat 0xB6FA74)`, else
/// with the mirror flag (0xB6F998) the second matrix (0xB6FABC).
#[hook(addr = 0x534540, conv = thiscall, name = "CEntity::GetIsOnScreen", expect = "83 EC 10 56 8B F1", this = "CEntity")]
pub fn get_is_on_screen_port(this: GPtr<u8>) -> u8 {
    let mut c = [0u32; 3];
    let cp = GPtr::from_raw(c.as_mut_ptr()).cast::<u8>();
    transform_from_object_space_ref(this, cp, mi_col(this).byte_add(0x18));
    let r = rd::<f32>(mi_col(this), 0x24);
    if camera_is_sphere_visible(g(THE_CAMERA), cp, r, g(0xB6FA74)) != 0 {
        return 1;
    }
    if g::<u8>(0xB6F998).read() == 0 {
        return 0;
    }
    camera_is_sphere_visible(g(THE_CAMERA), cp, r, g(0xB6FABC))
}

/// `CEntity::GetBoundRect()` (0x534120, hidden return pointer, `ret 4`): a `CRect(1e6, -1e6, -1e6,
/// 1e6)` stretched to the four xy corners (min, max, (max.x, min.y), (min.x, max.y) with the
/// original's swap: the max slot gets `fstp(min.x)`) of the col box in world space.
#[hook(addr = 0x534120, conv = thiscall, name = "CEntity::GetBoundRect", expect = "83 EC 34 56 8B F1", this = "CEntity")]
pub fn get_bound_rect(this: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> {
    let col = mi_col(this);
    let mut lo = [rd::<u32>(col, 0), rd::<u32>(col, 4), rd::<u32>(col, 8)];
    let mut hi = [rd::<u32>(col, 0xC), rd::<u32>(col, 0x10), rd::<u32>(col, 0x14)];
    let mut rect = [0x4974_2400u32, 0xC974_2400, 0xC974_2400, 0x4974_2400];
    let mut v = [0u32; 3];
    let lp = GPtr::from_raw(lo.as_mut_ptr()).cast::<u8>();
    let hp = GPtr::from_raw(hi.as_mut_ptr()).cast::<u8>();
    let rp = GPtr::from_raw(rect.as_mut_ptr()).cast::<u8>();
    let vp = GPtr::from_raw(v.as_mut_ptr()).cast::<u8>();
    let corner = |c: GPtr<u8>| {
        transform_from_object_space_ref(this, vp, c);
        rect_stretch_to_point(rp, rd(vp, 0), rd(vp, 4));
    };
    corner(lp);
    corner(hp);
    let min_x = x(rd(lp, 0)).to_f32();
    wr::<u32>(lp, 0, rd::<u32>(hp, 0));
    wr::<f32>(hp, 0, min_x);
    corner(lp);
    corner(hp);
    for k in [0, 4, 8, 0xC] {
        wr::<u32>(out, k, rd::<u32>(rp, k));
    }
    out
}

/// `CEntity::SetModelIndex(uint32)` (0x532AE0, `ret 4`): `SetModelIndexNoCreate` then
/// `CreateRwObject` through the vtable (slots 0x18 and 0x1C).
#[hook(addr = 0x532AE0, conv = thiscall, name = "CEntity::SetModelIndex", expect = "56 8B F1 8B 4C 24 08", this = "CEntity")]
pub fn set_model_index(this: GPtr<u8>, model: u32) {
    entity_vset_model_index_no_create(this, model);
    entity_vcreate_rw_object(this);
}

/// The `ModelIndices` `HasPreRenderEffects` compares against (in its order, after the special types).
const PRE_RENDER_MI_A: [usize; 5] = [0x8CD598, 0x8CD59C, 0x8CD5A0, 0x8CD5A4, 0x8CD5AC];
const PRE_RENDER_MI_B: [usize; 8] = [0x8CD638, 0x8CD6E0, 0x8CD71C, 0x8CD700, 0x8CD740, 0x8CD6FC, 0x8CD718, 0x8CD75C];
const PRE_RENDER_MI_C: [usize; 4] = [0x8CD518, 0x8CD51C, 0x8CD520, 0x8CD524];

/// `CEntity::HasPreRenderEffects` (0x532B70): special model types 1, 2, 9; a list of model indices
/// (and 0x159, 0x156, 0x158); glass; objects with object flag 0x1; traffic lights; four more model
/// indices; any light 2d effect (type 0).
#[hook(addr = 0x532B70, conv = thiscall, name = "CEntity::HasPreRenderEffects", expect = "56 57 8B F9 66 8B 77 22", this = "CEntity")]
pub fn has_pre_render_effects(this: GPtr<u8>) -> u8 {
    let model = rd::<i16>(this, 0x22);
    let m = i32::from(model);
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(m).read();
    let t = rd::<u16>(mi, 0x12) & 0x7800;
    if t == 0x800 || t == 0x1000 || t == 0x4800 {
        return 1;
    }
    let is = |a: usize| m == i32::from(g::<u16>(a).read());
    if PRE_RENDER_MI_A.iter().any(|&a| is(a)) || matches!(model, 0x159 | 0x156 | 0x158) || PRE_RENDER_MI_B.iter().any(|&a| is(a)) {
        return 1;
    }
    if is_object_glass(this) != 0 {
        return 1;
    }
    if rd::<u8>(this, 0x36) & 7 == 4 && rd::<u8>(this, 0x140) & 1 != 0 {
        return 1;
    }
    let m = i32::from(rd::<i16>(this, 0x22));
    if is_mi_traffic_light(m) != 0 {
        return 1;
    }
    let is = |a: usize| m == i32::from(g::<u16>(a).read());
    if PRE_RENDER_MI_C.iter().any(|&a| is(a)) {
        return 1;
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(m).read();
    if rd::<u8>(mi, 0xD) == 0 {
        return 0;
    }
    let mut i = 0i32;
    loop {
        if rd::<u8>(model_info_get_2d_effect(mi, i), 0xC) == 0 {
            return 1;
        }
        i += 1;
        if i >= i32::from(rd::<u8>(mi, 0xD)) {
            return 0;
        }
    }
}

/// `CEntity::SetModelIndexNoCreate(uint32)` (0x533700, `ret 4`): stores the index, flag bit 21 =
/// `HasPreRenderEffects`, bit 14 from model flag 0x2, bit 27 cleared without model flag 0x40; tag
/// atomics (`AsAtomicModelInfoPtr`, no flag 0x80 at +0x13, special type 6) are added to `CTagManager`.
#[hook(addr = 0x533700, conv = thiscall, name = "CEntity::SetModelIndexNoCreate", expect = "8B 44 24 04 56 57", this = "CEntity")]
pub fn set_model_index_no_create(this: GPtr<u8>, model: u32) {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(model as i32).read();
    wr::<u16>(this, 0x22, model as u16);
    let hp = has_pre_render_effects(this);
    let f = rd::<u32>(this, 0x1C);
    let f = f ^ (((u32::from(hp) << 21) ^ f) & 0x20_0000);
    wr::<u32>(this, 0x1C, f);
    if rd::<u8>(mi, 0x12) & 2 != 0 {
        wr::<u32>(this, 0x1C, f | 0x4000);
    }
    if rd::<u8>(mi, 0x12) & 0x40 == 0 {
        wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & 0xF7FF_FFFF);
    }
    if model_info_as_atomic(mi).is_null() {
        return;
    }
    if rd::<u8>(model_info_as_atomic(mi), 0x13) & 0x80 != 0 {
        return;
    }
    if rd::<u16>(model_info_as_atomic(mi), 0x12) & 0x7800 == 0x3000 {
        tag_manager_add_tag(this);
    }
}

/// `CEntity::IsEntityPointerValid(CEntity*)` (0x533310, cdecl): by type (jump table at 0x533360):
/// 0 → false, building/vehicle/ped/object/dummy → that pool's test (tail jumps), 6 → true.
#[hook(addr = 0x533310, conv = cdecl, name = "CEntity::IsEntityPointerValid", expect = "8B 44 24 04 85 C0")]
pub fn is_entity_pointer_valid(e: GPtr<u8>) -> u8 {
    if e.is_null() {
        return 0;
    }
    match rd::<u8>(e, 0x36) & 7 {
        1 => is_building_pointer_valid(e),
        2 => is_vehicle_pointer_valid(e),
        3 => is_ped_pointer_valid(e),
        4 => object_is_object_pointer_valid(e),
        5 => is_dummy_pointer_valid(e),
        6 => 1,
        _ => 0,
    }
}

/// `CEntity::FindTriggerPointCoors(CVector* out, int32 index)` (0x533380, `ret 8`): the world position
/// (`GetMatrix() * effect.pos`) of the first trigger-point 2d effect (type 8) with that index (+0x10);
/// zero when none. Returns `out`.
#[hook(addr = 0x533380, conv = thiscall, name = "CEntity::FindTriggerPointCoors", expect = "51 53 8B D9 0F BF 43 22", this = "CEntity")]
pub fn find_trigger_point_coors(this: GPtr<u8>, out: GPtr<u8>, index: i32) -> GPtr<u8> {
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
    let n = i32::from(rd::<u8>(mi, 0xD));
    let mut i = 0;
    while i < n {
        let e = model_info_get_2d_effect(mi, i);
        if rd::<u8>(e, 0xC) == 8 && rd::<i32>(e, 0x10) == index {
            let m = get_matrix(this);
            matrix_mul_vec(out, m, e);
            return out;
        }
        i += 1;
    }
    wr::<u32>(out, 0, 0);
    wr::<u32>(out, 4, 0);
    wr::<u32>(out, 8, 0);
    out
}

/// `CEntity::DetachFromRwObject` (0x533FB0): with an RwObject: model ref released, streaming entity
/// link (+0x28) removed, animated-UV atomics (model type 5 with flag 0x100 at +0x12, not objects)
/// leave `CWorld::ms_listMovingEntityPtrs` (0xB9ACC8), effects destroyed; then the RwObject is null.
#[hook(addr = 0x533FB0, conv = thiscall, name = "CEntity::DetachFromRwObject", expect = "56 8B F1 8B 46 18", this = "CEntity")]
pub fn detach_from_rw_object(this: GPtr<u8>) {
    if !rd::<GPtr<u8>>(this, 0x18).is_null() {
        let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
        model_info_remove_ref(mi);
        streaming_remove_entity(rd(this, 0x28));
        wr::<u32>(this, 0x28, 0);
        let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
        if model_info_get_type(mi) == 5 && rd::<u8>(mi, 0x13) & 1 != 0 && rd::<u8>(this, 0x36) & 7 != 4 {
            double_link_delete_item(g(0xB9ACC8), this);
        }
        entity_destroy_effects(this);
    }
    wr::<u32>(this, 0x18, 0);
}

/// `CEntity::MaterialUpdateUVAnimCB(RpMaterial*, void*)` (0x532D70, cdecl): advances an animated
/// material by `CTimer::ms_fTimeStep * 0.02` and applies it. Returns the material.
#[hook(addr = 0x532D70, conv = cdecl, name = "CEntity::MaterialUpdateUVAnimCB", expect = "56 8B 74 24 08")]
pub fn material_update_uv_anim_cb(m: GPtr<u8>, _data: u32) -> GPtr<u8> {
    if rp_material_uv_anim_exists(m) != 0 {
        let dt = x(g::<f32>(TIME_STEP).read()).fmul_m(F_0_02).to_f32();
        rp_material_uv_anim_add_anim_time(m, dt);
        rp_material_uv_anim_apply_update(m);
    }
    m
}

/// `CEntity::UpdateRpHAnim` (0x532B20; the flag test runs in the protection chunk at 0x401E1A): for
/// skinned clumps without flag 0x800000 (+0x1C), `RpHAnimHierarchyUpdateMatrices`.
#[hook(addr = 0x532B20, conv = thiscall, name = "CEntity::UpdateRpHAnim", expect = "56 8B F1 8B 46 18", this = "CEntity")]
pub fn update_rp_h_anim(this: GPtr<u8>) {
    let a = rw_get_first_atomic(rd(this, 0x18));
    if a.is_null() || rp_skin_geometry_get_skin(rd(a, 0x18)).is_null() {
        return;
    }
    if rd::<u32>(this, 0x1C) & 0x80_0000 != 0 {
        return;
    }
    rp_h_anim_hierarchy_update_matrices(rw_get_anim_hierarchy_from_skin_clump(rd(this, 0x18)));
}

/// `CEntity::ModifyMatrixForCrane` (0x533170): unless paused (0xB7CB49 / 0xB7CB48), rotates the RW
/// frame about z by `(timeMs & 0x3FF) * 0.0061328127` through a temporary `CMatrix` attached to it.
/// (The SEH frame around the temporary is not built: stack only.)
#[hook(addr = 0x533170, conv = thiscall, name = "CEntity::ModifyMatrixForCrane", expect = "64 A1 00 00 00 00", this = "CEntity")]
pub fn modify_matrix_for_crane(this: GPtr<u8>) {
    if g::<u8>(0xB7CB49).read() != 0 || g::<u8>(0xB7CB48).read() != 0 {
        return;
    }
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if rw.is_null() || rd::<GPtr<u8>>(rw, 4).byte_add(0x10).is_null() {
        return;
    }
    let mut tmp = [0u32; 0x12];
    let tp = GPtr::from_raw(tmp.as_mut_ptr()).cast::<u8>();
    matrix_ctor_attach(tp, rd::<GPtr<u8>>(rw, 4).byte_add(0x10), 0);
    let t = g::<i32>(TIME_MS).read() & 0x3FF;
    let a = xi(t).fmul_m(F_CRANE).to_f32();
    matrix_set_rotate_z_only(tp, a);
    matrix_update_rw_attached(tp);
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if !rw.is_null() {
        rw_frame_update_objects(rd(rw, 4));
    }
    matrix_dtor(tp);
}

/// `CEntity::UpdateAnim` (0x535F00): clears flag 0x800000; for animated clumps, objects of type 4 use
/// the unclipped time step and update always; others refresh the off-screen flag 0x20000 (unless
/// set) and pass its inverse (the whole shifted dword, as the original pushes it) as "visible".
#[hook(addr = 0x535F00, conv = thiscall, name = "CEntity::UpdateAnim", expect = "56 8B F1 8B 56 1C", this = "CEntity")]
pub fn update_anim(this: GPtr<u8>) {
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & 0xFF7F_FFFF);
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if rw.is_null() || rd::<u8>(rw, 0) != 2 || rp_anim_blend_clump_get_first_association(rw).is_null() {
        return;
    }
    let (ts, flag) = if rd::<u8>(this, 0x36) & 7 == 4 && rd::<u8>(this, 0x13C) == 4 {
        (g::<f32>(TIME_STEP_NON_CLIPPED).read(), 1u32)
    } else {
        if rd::<u32>(this, 0x1C) & 0x2_0000 == 0 {
            let off = u32::from(get_is_on_screen(this) == 0);
            let f = rd::<u32>(this, 0x1C);
            wr::<u32>(this, 0x1C, f ^ (((off << 17) ^ f) & 0x2_0000));
        }
        let s = rd::<u32>(this, 0x1C) >> 17;
        (g::<f32>(TIME_STEP).read(), (s ^ 1) & 0xFFFF_FF01)
    };
    let step = x(ts).fmul_m(F_0_02).to_f32();
    rp_anim_blend_clump_update_animations(rd(this, 0x18), step, flag);
}

/// `CEntity::AttachToRwObject(RwObject*, bool updateMatrix)` (0x533ED0, `ret 8`): only for visible
/// entities (flag bit 7). Stores the RwObject; optionally refreshes the matrix from the frame;
/// animated-UV clumps go to the moving list (objects: `AddToMovingList` when not in it, then
/// `SetIsStatic(false)`; others: a new node in `ms_listMovingEntityPtrs`); then model ref, streaming
/// link (+0x28) and effects.
#[hook(addr = 0x533ED0, conv = thiscall, name = "CEntity::AttachToRwObject", expect = "56 8B F1 8A 46 1C", this = "CEntity")]
pub fn attach_to_rw_object(this: GPtr<u8>, rw: GPtr<u8>, update: u8) {
    if (rd::<u8>(this, 0x1C) as i8) >= 0 {
        return;
    }
    wr::<GPtr<u8>>(this, 0x18, rw);
    if rw.is_null() {
        return;
    }
    if update != 0 {
        let m = get_matrix(this);
        matrix_update_matrix(m, rd::<GPtr<u8>>(rd(this, 0x18), 4).byte_add(0x10));
    }
    if rd::<u8>(rd(this, 0x18), 0) == 2 {
        let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
        if rd::<u8>(mi, 0x13) & 1 != 0 {
            if rd::<u8>(this, 0x36) & 7 == 4 {
                if rd::<u32>(this, 0xB4) == 0 {
                    physical_add_to_moving_list(this);
                }
                entity_vset_is_static(this, 0);
            } else {
                let n = double_node_new(0xC);
                if !n.is_null() {
                    wr::<GPtr<u8>>(n, 0, this);
                }
                double_node_add_to_list(n, g(0xB9ACC8));
            }
        }
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
    model_info_add_ref(mi);
    let l = streaming_add_entity(this);
    wr::<GPtr<u8>>(this, 0x28, l);
    entity_create_effects(this);
}

/// `CEntity::DeleteRwObject` (0x534030): destroys the atomic (and its frame) or the clump (skinned
/// clumps first drop their anim through 0x734B90); then as `DetachFromRwObject` (model ref, streaming
/// link, buildings decrement 0xB71804, moving-list node, effects) and finally
/// `RemoveEscalatorsForEntity` (tail jump).
#[hook(addr = 0x534030, conv = thiscall, name = "CEntity::DeleteRwObject", expect = "56 8B F1 57 8B 7E 18", this = "CEntity")]
pub fn delete_rw_object(this: GPtr<u8>) {
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if rw.is_null() {
        return;
    }
    match rd::<u8>(rw, 0) {
        1 => {
            let frame = rd::<GPtr<u8>>(rw, 4);
            rp_atomic_destroy(rw);
            rw_frame_destroy(frame);
        }
        2 => {
            let a = rw_get_first_atomic(rw);
            if !a.is_null() && !rp_skin_geometry_get_skin(rd(a, 0x18)).is_null() {
                rp_clump_for_all_atomics(rw, 0x734B90, 0);
            }
            rp_clump_destroy(rw);
        }
        _ => {}
    }
    wr::<u32>(this, 0x18, 0);
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
    model_info_remove_ref(mi);
    streaming_remove_entity(rd(this, 0x28));
    wr::<u32>(this, 0x28, 0);
    if rd::<u8>(this, 0x36) & 7 == 1 {
        let n = g::<i32>(0xB71804);
        n.write(n.read().wrapping_sub(1));
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(this, 0x22))).read();
    if model_info_get_type(mi) == 5 && rd::<u8>(mi, 0x13) & 1 != 0 && rd::<u8>(this, 0x36) & 7 != 4 {
        double_link_delete_item(g(0xB9ACC8), this);
    }
    entity_destroy_effects(this);
    remove_escalators_for_entity(this);
}

/// The empty `CEntity` virtuals (`ret` / `ret n`, 1-3 bytes): `ProcessControl` 0x403E40,
/// `ProcessCollision` 0x403E50, `ProcessShift` 0x403E60, `Teleport` 0x403E80 (`ret 0x10`),
/// `SpecialEntityPreCollisionStuff` 0x403E90 (`ret 0x18`), `FlagToDestroyWhenNextProcessed` 0x403EB0.
/// Ported, unhookable.
pub fn entity_empty_virtual(_this: GPtr<u8>) {}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_GET_MODEL_INDEX,
    HOOK_GET_STATUS,
    HOOK_TREAT_AS_PLAYER_FOR_COLLISIONS,
    HOOK_SET_IS_STATIC,
    HOOK_SET_HEADING,
    HOOK_UPDATE_RW_FRAME,
    HOOK_SETUP_BIG_BUILDING,
    HOOK_GET_MODELLING_MATRIX,
    HOOK_PLACEABLE_CTOR,
    HOOK_GET_IS_STATIC,
    HOOK_GET_IS_TYPE_PHYSICAL,
    HOOK_SET_POSN_VECTOR,
    HOOK_SET_POSN_XYZ,
    HOOK_DUMMY_DELETE,
    HOOK_BUILDING_DELETE,
    HOOK_BUILDING_NEW,
    HOOK_GET_COL_MODEL,
    HOOK_REMOVE_ESCALATORS_FOR_ENTITY,
    HOOK_GET_DISTANCE_FROM_CENTRE_OF_MASS_TO_BASE_OF_MODEL,
    HOOK_CLEAN_UP_OLD_REFERENCE,
    HOOK_GET_RIGHT_DIRECTION,
    HOOK_GET_TOP_DIRECTION,
    HOOK_GET_AT_DIRECTION,
    HOOK_TRANSFORM_POINT,
    HOOK_TRANSFORM_VECTORS,
    HOOK_SIMPLE_TRANSFORM_INVERT,
    HOOK_PRUNE_REFERENCES,
    HOOK_RESOLVE_REFERENCES,
    HOOK_SET_ORIENTATION,
    HOOK_REMOVE_MATRIX,
    HOOK_FREE_STATIC_MATRIX,
    HOOK_DUMMY_POOL_NEW,
    HOOK_PTR_LIST_DOUBLE_LINK_CTOR,
    HOOK_SET_COMP_ALPHA_CB,
    HOOK_IS_DUMMY_POINTER_VALID,
    HOOK_IS_BUILDING_POINTER_VALID,
    HOOK_BOUNDING_BOX_IS_POINT_WITHIN,
    HOOK_IS_WITHIN_AREA_XY,
    HOOK_IS_WITHIN_AREA_XYZ,
    HOOK_TEST_COLLISION,
    HOOK_SPECIAL_ENTITY_CALC_COLLISION_STEPS,
    HOOK_DOES_NOT_COLLIDE_WITH_FLYERS,
    HOOK_GET_RANDOM_2D_EFFECT,
    HOOK_BUILDING_DTOR,
    HOOK_DUMMY_DTOR,
    HOOK_DUMMY_PED_DTOR,
    HOOK_BUILDING_SCALAR_DTOR,
    HOOK_DUMMY_SCALAR_DTOR,
    HOOK_ENTITY_CTOR,
    HOOK_BUILDING_CTOR,
    HOOK_DUMMY_CTOR,
    HOOK_DUMMY_PED_CTOR,
    HOOK_PLACEABLE_DTOR,
    HOOK_INIT_MATRIX_ARRAY,
    HOOK_ALLOCATE_MATRIX,
    HOOK_TRANSFORM_FROM_OBJECT_SPACE_REF,
    HOOK_TRANSFORM_FROM_OBJECT_SPACE_PTR,
    HOOK_GET_BOUND_CENTRE_REF,
    HOOK_GET_BOUND_CENTRE_VEC,
    HOOK_REMOVE_LIGHTING,
    HOOK_ENTITY_SETUP_LIGHTING,
    HOOK_SINGLE_LINK_ADD_ITEM,
    HOOK_DOUBLE_LINK_ADD_ITEM,
    HOOK_DOUBLE_LINK_DELETE_ITEM,
    HOOK_IS_VISIBLE,
    HOOK_SET_ATOMIC_ALPHA,
    HOOK_PRE_RENDER_FOR_GLASS_WINDOW,
    HOOK_REPLACE_WITH_NEW_MODEL,
    HOOK_REGISTER_REFERENCE,
    HOOK_ENTITY_SCALAR_DTOR,
    HOOK_DUMMY_PED_SCALAR_DTOR,
    HOOK_PLACEABLE_SCALAR_DTOR,
    HOOK_SHUTDOWN_MATRIX_ARRAY,
    HOOK_SET_RW_OBJECT_ALPHA,
    HOOK_GET_MATRIX,
    HOOK_ADD_VOID,
    HOOK_SINGLE_LINK_DTOR,
    HOOK_DOUBLE_LINK_DTOR,
    HOOK_GET_IS_TOUCHING_VEC,
    HOOK_GET_IS_TOUCHING_ENT,
    HOOK_GET_IS_ON_SCREEN_PORT,
    HOOK_GET_BOUND_RECT,
    HOOK_SET_MODEL_INDEX,
    HOOK_HAS_PRE_RENDER_EFFECTS,
    HOOK_SET_MODEL_INDEX_NO_CREATE,
    HOOK_IS_ENTITY_POINTER_VALID,
    HOOK_FIND_TRIGGER_POINT_COORS,
    HOOK_DETACH_FROM_RW_OBJECT,
    HOOK_MATERIAL_UPDATE_UV_ANIM_CB,
    HOOK_UPDATE_RP_H_ANIM,
    HOOK_MODIFY_MATRIX_FOR_CRANE,
    HOOK_UPDATE_ANIM,
    HOOK_ATTACH_TO_RW_OBJECT,
    HOOK_DELETE_RW_OBJECT,
];
