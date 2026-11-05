//! `CPedModelInfo` (vtable 0x85BDC0, size 0x44; `m_pHitColModel` at +0x34, voice range at
//! +0x3E/+0x40, current voice at +0x42).
//!
//! Ported from the 1.0 US bytes; gta-reversed `Models/PedModelInfo.cpp` is a reading aid. Kept from
//! the original:
//! - the hit col model has 12 spheres from `m_pColNodeInfos` (0x8A630C, 0x1C bytes each: bone id,
//!   piece, centre, radius); centres are transformed with `CGame::m_pWorkingMatrix1/2`;
//!   `CreateHitColModelSkinned` does not null-check `new CColModel` (the original then crashes in
//!   `AllocateData`; the port passes the null pointer on as well);
//! - `AnimatePedColModelSkinned` copies the working matrix for every sphere and for the root
//!   (bone 3) and sets the bounding box to centre ± 1.2 (x87 PC=24 add/sub, stored as f32);
//!   `...World` uses the bone matrices directly;
//! - `~CPedModelInfo` deletes the hit col model without clearing the pointer (CPedModelInfo.md);
//!   the C++ EH frame it sets up has no observable effect and is not reproduced.

use sa_core::abi::HookReg;
use sa_core::light::{RwMatrix, RwV3d};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::atomic::scalar_delete;
use super::base::{col_model_destructor, col_model_operator_delete, f, MiPtr, VTBL_BASE_MODEL_INFO};
use super::weapon::rp_clump_for_all_atomics;
use crate::shared::mi;

/// `CPedModelInfo` vtable.
pub const VTBL_PED_MODEL_INFO: u32 = 0x85BDC0;
/// +0x34 `m_pHitColModel`
pub const HIT_COL_MODEL: isize = 0x34;
/// `CPedModelInfo::m_pPedIds` (0x8A6268), the frame name/id table `SetClump` passes to `SetFrameIds`
pub const PED_IDS: u32 = 0x8A6268;
/// `CClumpModelInfo::SetAtomicRendererCB` (0x4C5280)
pub const SET_ATOMIC_RENDERER_CB: u32 = 0x4C5280;
/// `CVisibilityPlugins::RenderPedCB` (0x7335B0)
pub const RENDER_PED_CB: u32 = 0x7335B0;
/// Number of entries of `m_pColNodeInfos`.
pub const NUM_COL_NODES: usize = 12;

sa_core::globals! {
    list GLOBALS;
    /// `CPedModelInfo::m_pColNodeInfos` (12 × 0x1C: bone id, piece, centre x/y/z, radius, pad; the
    /// last pad dword is not declared: it holds `CVehicleModelInfo::ms_compsToUse` 0x8A6458)
    pub static COL_NODE_INFOS @ 0x8A630C: [u32; 83] = [
        5, 9, 0x3D4C_CCCD, 0, 0, 0x3E19_999A, 0,
        3, 3, 0x3E4C_CCCD, 0, 0, 0x3E4C_CCCD, 0,
        3, 3, 0, 0, 0, 0x3E4C_CCCD, 0,
        2, 4, 0xBDCC_CCCD, 0, 0, 0x3E4C_CCCD, 0,
        0x20, 5, 0x3D75_C28F, 0, 0, 0x3E0F_5C29, 0,
        0x16, 6, 0x3D75_C28F, 0, 0, 0x3E0F_5C29, 0,
        0x21, 5, 0x3D4C_CCCD, 0, 0, 0x3E0F_5C29, 0,
        0x17, 6, 0x3D4C_CCCD, 0, 0, 0x3E0F_5C29, 0,
        0x2A, 7, 0xBDCC_CCCD, 0, 0, 0x3E38_51EC, 0,
        0x34, 8, 0xBDCC_CCCD, 0, 0, 0x3E38_51EC, 0,
        0x2B, 7, 0xBE38_51EC, 0, 0, 0x3E23_D70A, 0,
        0x35, 8, 0xBE38_51EC, 0, 0, 0x3E23_D70A,
    ];
    // foreign (owner: CGame)
    /// `CGame::m_pWorkingMatrix1`
    pub static WORKING_MATRIX_1 @ 0xB72920: GPtr<RwMatrix> = GPtr::NULL;
    /// `CGame::m_pWorkingMatrix2`
    pub static WORKING_MATRIX_2 @ 0xB72924: GPtr<RwMatrix> = GPtr::NULL;
}

orig! {
    /// `CClumpModelInfo::SetClump` (0x4C4F70)
    pub fn clump_set_clump(this: MiPtr, clump: GPtr<u8>) = thiscall 0x4C4F70 => super::clump::set_clump;
    /// `CClumpModelInfo::SetFrameIds` (0x4C5460)
    pub fn clump_set_frame_ids(this: MiPtr, ids: GPtr<u8>) = thiscall 0x4C5460 => super::clump::set_frame_ids;
    /// `CClumpModelInfo::DeleteRwObject` (0x4C4E70)
    pub fn clump_delete_rw_object(this: MiPtr) = thiscall 0x4C4E70 => super::clump::delete_rw_object;
    /// `CBaseModelInfo::CBaseModelInfo` (0x4C4A60)
    pub fn base_model_info_constructor(this: MiPtr) -> MiPtr = thiscall 0x4C4A60 => unported;
    /// `CPedModelInfo::CreateHitColModelSkinned` (0x4C6D90)
    pub fn create_hit_col_model_skinned_call(this: MiPtr, clump: GPtr<u8>) = thiscall 0x4C6D90 => create_hit_col_model_skinned;
    /// `CPedModelInfo::~CPedModelInfo` (0x4C62F0)
    pub fn ped_destructor_call(this: MiPtr) = thiscall 0x4C62F0 => destructor;
    /// `RwHelper::GetAnimHierarchyFromSkinClump` (0x734A40)
    pub fn get_anim_hierarchy_from_skin_clump(clump: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RwHelper::GetAnimHierarchyFromClump` (0x734B10)
    pub fn get_anim_hierarchy_from_clump(clump: GPtr<u8>) -> GPtr<u8> = cdecl 0x734B10 => unported;
    /// `CColModel::operator new` (0x40FC30)
    pub fn col_model_operator_new(size: u32) -> GPtr<u8> = cdecl 0x40FC30 => unported;
    /// `CColModel::CColModel` (0x40FB60)
    pub fn col_model_constructor(col: GPtr<u8>) -> GPtr<u8> = thiscall 0x40FB60 => unported;
    /// `CColModel::AllocateData(int, int, int, int, int, bool)` (0x40F870)
    pub fn col_model_allocate_data(col: GPtr<u8>, spheres: i32, boxes: i32, lines: i32, verts: i32, tris: i32, disks: u32) = thiscall 0x40F870 => unported;
    /// `CSphere::Set(float, const CVector&)` (0x40FCF0)
    pub fn sphere_set(sphere: GPtr<u8>, radius: f32, centre: GPtr<RwV3d>) = thiscall 0x40FCF0 => unported;
    /// `CBox::Set(const CVector&, const CVector&)` (0x40EDE0)
    pub fn box_set(b: GPtr<u8>, min: GPtr<RwV3d>, max: GPtr<RwV3d>) = thiscall 0x40EDE0 => unported;
    /// `RwMatrixInvert` (0x7F2070)
    pub fn rw_matrix_invert(dst: GPtr<RwMatrix>, src: GPtr<RwMatrix>) -> GPtr<RwMatrix> = cdecl 0x7F2070 => unported;
    /// `RwMatrixTransform` (0x7F25A0)
    pub fn rw_matrix_transform(m: GPtr<RwMatrix>, t: GPtr<RwMatrix>, op: i32) -> GPtr<RwMatrix> = cdecl 0x7F25A0 => unported;
    /// `RwV3dTransformPoints` (0x7EDD90)
    pub fn rw_v3d_transform_points(out: GPtr<RwV3d>, inp: GPtr<RwV3d>, n: i32, m: GPtr<RwMatrix>) -> GPtr<RwV3d> = cdecl 0x7EDD90 => unported;
    /// `RpHAnimIDGetIndex` (0x7C51A0)
    pub fn rp_hanim_id_get_index(hier: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray` (0x7C5120)
    pub fn rp_hanim_hierarchy_get_matrix_array(hier: GPtr<u8>) -> GPtr<RwMatrix> = cdecl 0x7C5120 => unported;
}

fn hit_col(this: MiPtr) -> GPtr<u8> {
    f::<GPtr<u8>>(this, HIT_COL_MODEL).read()
}

/// Node info `k`: (bone id, piece byte, centre, radius).
fn node(k: usize) -> (i32, u8, RwV3d, u32) {
    let e = COL_NODE_INFOS.ptr().cast::<u32>().wrapping_index(7 * k as i32);
    let w = |i: i32| e.wrapping_index(i).read();
    (
        w(0) as i32,
        w(1) as u8,
        RwV3d { x: f32::from_bits(w(2)), y: f32::from_bits(w(3)), z: f32::from_bits(w(4)) },
        w(5),
    )
}

/// `&hier.matrices[RpHAnimIDGetIndex(hier, id)]` (the original's `shl 6; add`).
fn bone_matrix(hier: GPtr<u8>, id: i32) -> GPtr<RwMatrix> {
    let idx = rp_hanim_id_get_index(hier, id);
    let mats = rp_hanim_hierarchy_get_matrix_array(hier);
    mats.cast::<u8>().byte_add(((idx as u32) << 6) as i32 as isize).cast()
}

/// `RwFrameGetMatrix(RpClumpGetFrame(clump))`: `clump->frame + 0x10`.
fn clump_frame_matrix(clump: GPtr<u8>) -> GPtr<RwMatrix> {
    clump.byte_add(4).cast::<GPtr<u8>>().read().byte_add(0x10).cast()
}

/// `vec = point; WM2 = WM1; WM2 = bone * WM2 (pre-concat); vec = WM2 * vec`.
fn transform_via_working(hier: GPtr<u8>, id: i32, v: &mut RwV3d) {
    let (wm1, wm2) = (WORKING_MATRIX_1.get(), WORKING_MATRIX_2.get());
    wm2.write(wm1.read());
    let m = bone_matrix(hier, id);
    rw_matrix_transform(wm2, m, 1);
    let p = GPtr::from_raw(v as *mut RwV3d);
    rw_v3d_transform_points(p, p, 1, wm2);
}

/// `CPedModelInfo::GetModelType` (0x4C57C0): AL = 7 (unhookable).
pub fn get_model_type(_this: MiPtr) -> u8 {
    7
}

/// `CPedModelInfo::AddXtraAtomics` (0x4C6D40): `ret 4` (unhookable).
pub fn add_xtra_atomics(_this: MiPtr, _clump: GPtr<u8>) {}

/// `CPedModelInfo::SetFaceTexture` (0x4C6D50): `ret 4` (unhookable).
pub fn set_face_texture(_this: MiPtr, _tex: GPtr<u8>) {}

/// `CPedModelInfo::IncrementVoice` (0x4C7300): with a valid range (`min, max >= 0`, signed 16-bit),
/// `++voice` and wrap to `min` when it leaves `[min, max]`; otherwise `voice = -1`.
#[hook(addr = 0x4C7300, conv = thiscall, name = "CPedModelInfo::IncrementVoice", expect = "56 66 8B 71 3E", this = "CPedModelInfo")]
pub fn increment_voice(this: MiPtr) {
    let min = f::<i16>(this, 0x3E).read();
    let max = f::<i16>(this, 0x40).read();
    let cur = f::<i16>(this, 0x42);
    if min < 0 || max < 0 {
        cur.write(-1);
        return;
    }
    cur.write(cur.read().wrapping_add(1));
    let v = cur.read();
    if v > max || v < min {
        cur.write(min);
    }
}

/// `CPedModelInfo::CPedModelInfo` (0x4C57A0): base ctor, vptr 0x85BDC0, `m_pHitColModel = 0`.
#[hook(addr = 0x4C57A0, conv = thiscall, name = "CPedModelInfo::CPedModelInfo", expect = "56 8B F1 E8 B8 F2 FF FF", this = "CPedModelInfo")]
pub fn constructor(this: MiPtr) -> MiPtr {
    base_model_info_constructor(this);
    f::<u32>(this, mi::VTABLE).write(VTBL_PED_MODEL_INFO);
    f::<u32>(this, HIT_COL_MODEL).write(0);
    this
}

fn delete_hit_col(this: MiPtr) {
    let col = hit_col(this);
    if !col.is_null() {
        col_model_destructor(col);
        col_model_operator_delete(col);
    }
}

/// `CPedModelInfo::~CPedModelInfo` (0x4C62F0).
#[hook(addr = 0x4C62F0, conv = thiscall, name = "CPedModelInfo::~CPedModelInfo", expect = "6A FF 68 28 BC 83 00", this = "CPedModelInfo")]
pub fn destructor(this: MiPtr) {
    f::<u32>(this, mi::VTABLE).write(VTBL_PED_MODEL_INFO);
    delete_hit_col(this);
    f::<u32>(this, mi::VTABLE).write(VTBL_BASE_MODEL_INFO);
}

/// `CPedModelInfo`'s scalar deleting destructor (0x4C6350).
#[hook(addr = 0x4C6350, conv = thiscall, name = "CPedModelInfo::vfunc_0", expect = "56 8B F1 E8 98 FF FF FF", this = "CPedModelInfo")]
pub fn scalar_deleting_destructor(this: MiPtr, flags: u32) -> MiPtr {
    scalar_delete(this, flags, ped_destructor_call)
}

/// `CPedModelInfo::DeleteRwObject` (0x4C6C50): clump part, then the hit col model (cleared).
#[hook(addr = 0x4C6C50, conv = thiscall, name = "CPedModelInfo::DeleteRwObject", expect = "56 57 8B F1 E8 17 E2 FF FF", this = "CPedModelInfo")]
pub fn delete_rw_object(this: MiPtr) {
    clump_delete_rw_object(this);
    delete_hit_col(this);
    f::<u32>(this, HIT_COL_MODEL).write(0);
}

/// `CPedModelInfo::SetClump(RpClump*)` (0x4C7340, `ret 4`).
#[hook(addr = 0x4C7340, conv = thiscall, name = "CPedModelInfo::SetClump", expect = "56 57 8B 7C 24 0C", this = "CPedModelInfo")]
pub fn set_clump(this: MiPtr, clump: GPtr<u8>) {
    clump_set_clump(this, clump);
    clump_set_frame_ids(this, GPtr::from_addr(PED_IDS as usize));
    if hit_col(this).is_null() {
        create_hit_col_model_skinned_call(this, clump);
    }
    let rw = f::<GPtr<u8>>(this, mi::RW_OBJECT).read();
    rp_clump_for_all_atomics(rw, SET_ATOMIC_RENDERER_CB, RENDER_PED_CB);
    get_anim_hierarchy_from_clump(clump);
}

/// `CPedModelInfo::CreateHitColModelSkinned(RpClump*)` (0x4C6D90, `ret 4`; entry `push -1; jmp`
/// into a protection chunk that pushes the EH handler).
#[hook(addr = 0x4C6D90, conv = thiscall, name = "CPedModelInfo::CreateHitColModelSkinned", expect = "6A FF E9 96 CA F3 FF", this = "CPedModelInfo")]
pub fn create_hit_col_model_skinned(this: MiPtr, clump: GPtr<u8>) {
    let hier = get_anim_hierarchy_from_skin_clump(clump);
    let mem = col_model_operator_new(0x30);
    let cm = if mem.is_null() { GPtr::NULL } else { col_model_constructor(mem) };
    col_model_allocate_data(cm, NUM_COL_NODES as i32, 0, 0, 0, 0, 0);
    rw_matrix_invert(WORKING_MATRIX_1.get(), clump_frame_matrix(clump));
    let mut sphere = cm.byte_add(0x2C).cast::<GPtr<u8>>().read().byte_add(8).cast::<GPtr<u8>>().read();
    for k in 0..NUM_COL_NODES {
        let (bone, piece, centre, radius) = node(k);
        let mut v = centre;
        transform_via_working(hier, bone, &mut v);
        sphere.cast::<RwV3d>().write(v);
        sphere.byte_add(0x0C).cast::<u32>().write(radius);
        sphere.byte_add(0x10).write(0x3E);
        sphere.byte_add(0x11).write(piece);
        sphere = sphere.byte_add(0x14);
    }
    let mut zero = RwV3d::default();
    sphere_set(cm.byte_add(0x18), 1.5, GPtr::from_raw(&mut zero));
    let mut lo = RwV3d { x: -0.5, y: -0.5, z: -1.2 };
    let mut hi = RwV3d { x: 0.5, y: 0.5, z: 1.2 };
    box_set(cm, GPtr::from_raw(&mut lo), GPtr::from_raw(&mut hi));
    cm.byte_add(0x28).write(0);
    f::<GPtr<u8>>(this, HIT_COL_MODEL).write(cm);
}

/// `centre ± 1.2` as the original's x87 add/sub (PC=24) stored to f32.
pub fn box_around(c: RwV3d) -> (RwV3d, RwV3d) {
    let k = f32::from_bits(0x3F99_999A);
    let a = |v: f32| X87::from_f32(v).fadd_m(k).to_f32();
    let s = |v: f32| X87::from_f32(v).fsub_m(k).to_f32();
    (RwV3d { x: s(c.x), y: s(c.y), z: s(c.z) }, RwV3d { x: a(c.x), y: a(c.y), z: a(c.z) })
}

fn set_bounds(this: MiPtr, centre: RwV3d) {
    let mut c = centre;
    sphere_set(hit_col(this).byte_add(0x18), 1.5, GPtr::from_raw(&mut c));
    let (mut lo, mut hi) = box_around(c);
    box_set(hit_col(this), GPtr::from_raw(&mut lo), GPtr::from_raw(&mut hi));
}

/// `CPedModelInfo::AnimatePedColModelSkinned(RpClump*)` (0x4C6F70, `ret 4`): returns the hit col
/// model (created on first use, then returned without animating).
#[hook(addr = 0x4C6F70, conv = thiscall, name = "CPedModelInfo::AnimatePedColModelSkinned", expect = "83 EC 34 56 8B F1", this = "CPedModelInfo")]
pub fn animate_ped_col_model_skinned(this: MiPtr, clump: GPtr<u8>) -> GPtr<u8> {
    if hit_col(this).is_null() {
        create_hit_col_model_skinned_call(this, clump);
        return hit_col(this);
    }
    let col_data = hit_col(this).byte_add(0x2C).cast::<GPtr<u8>>().read();
    let hier = get_anim_hierarchy_from_skin_clump(clump);
    rw_matrix_invert(WORKING_MATRIX_1.get(), clump_frame_matrix(clump));
    let mut off = 0isize;
    for k in 0..NUM_COL_NODES {
        let (bone, _, centre, _) = node(k);
        let mut v = centre;
        transform_via_working(hier, bone, &mut v);
        let spheres = col_data.byte_add(8).cast::<GPtr<u8>>().read();
        spheres.byte_add(off).cast::<RwV3d>().write(v);
        off += 0x14;
    }
    let mut root = RwV3d::default();
    transform_via_working(hier, 3, &mut root);
    set_bounds(this, root);
    hit_col(this)
}

/// `CPedModelInfo::AnimatePedColModelSkinnedWorld(RpClump*)` (0x4C7170, `ret 4`): as above with
/// the bone matrices directly (world space); creates the col model first when missing and then
/// animates it too.
#[hook(addr = 0x4C7170, conv = thiscall, name = "CPedModelInfo::AnimatePedColModelSkinnedWorld", expect = "83 EC 28 53 55", this = "CPedModelInfo")]
pub fn animate_ped_col_model_skinned_world(this: MiPtr, clump: GPtr<u8>) -> GPtr<u8> {
    if hit_col(this).is_null() {
        create_hit_col_model_skinned_call(this, clump);
    }
    let col_data = hit_col(this).byte_add(0x2C).cast::<GPtr<u8>>().read();
    let hier = get_anim_hierarchy_from_skin_clump(clump);
    let mut off = 0isize;
    for k in 0..NUM_COL_NODES {
        let (bone, _, centre, _) = node(k);
        let mut v = centre;
        let m = bone_matrix(hier, bone);
        let p = GPtr::from_raw(&mut v as *mut RwV3d);
        rw_v3d_transform_points(p, p, 1, m);
        let spheres = col_data.byte_add(8).cast::<GPtr<u8>>().read();
        spheres.byte_add(off).cast::<RwV3d>().write(v);
        off += 0x14;
    }
    let mut root = RwV3d::default();
    let m = bone_matrix(hier, 3);
    let p = GPtr::from_raw(&mut root as *mut RwV3d);
    rw_v3d_transform_points(p, p, 1, m);
    set_bounds(this, root);
    hit_col(this)
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[HookReg] = &[
    HOOK_INCREMENT_VOICE,
    HOOK_CONSTRUCTOR,
    HOOK_DESTRUCTOR,
    HOOK_SCALAR_DELETING_DESTRUCTOR,
    HOOK_DELETE_RW_OBJECT,
    HOOK_SET_CLUMP,
    HOOK_CREATE_HIT_COL_MODEL_SKINNED,
    HOOK_ANIMATE_PED_COL_MODEL_SKINNED,
    HOOK_ANIMATE_PED_COL_MODEL_SKINNED_WORLD,
];
