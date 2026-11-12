//! `CVisibilityPlugins` plugin-data accessors and the clump visibility callbacks (STD-6, batch
//! `std6-render-1`). Ported from the 1.0 US bytes (0x732230..0x733750); gta-reversed
//! `VisibilityPlugins.cpp` as reading aid.
//!
//! Atomic plugin data (`atomic + [0x8D608C]`): +0 model index (i16), +2 flags / id (u16). Frame plugin
//! data (`frame + [0x8D6090]`): the hierarchy id (the clump's `CClumpModelInfo*`). Clump plugin data
//! (`clump + [0x8D6094]`): +0 visibility callback, +4 alpha.

use sa_core::abi::{HookReg, NativeReg};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

orig! {
    /// `RpClumpForAllAtomics(clump, cb, data)` (0x749B70)
    fn clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `CBaseModelInfo::GetModelType` (vtable slot 0x10)
    fn mi_get_model_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
    /// A clump visibility callback `cb(clump)`
    fn call_visibility_cb(f: u32, clump: GPtr<u8>) -> u32 = cdecl dyn => unported;
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `RwV3dTransformPoints(out, in, n, matrix)` (0x7EDD90)
    fn v3d_transform_points(out: GPtr<f32>, inp: GPtr<f32>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `RwCameraFrustumTestSphere(camera, sphere)` (0x7EE2D0)
    fn camera_frustum_test_sphere(cam: GPtr<u8>, sphere: GPtr<f32>) -> u32 = cdecl 0x7EE2D0 => unported;
}

const ATOMIC_OFFSET: usize = 0x8D608C;
const FRAME_OFFSET: usize = 0x8D6090;
const CLUMP_OFFSET: usize = 0x8D6094;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `Scene.m_pRwCamera`
const SCENE_CAMERA: usize = 0xC8804C;
/// `AtomicDefaultRenderCallBack` (0x7491C0)
const ATOMIC_DEFAULT_RENDER_CB: u32 = 0x7491C0;

fn adata(atomic: GPtr<u8>) -> GPtr<u8> {
    atomic.byte_add(g::<i32>(ATOMIC_OFFSET).read() as isize)
}

fn cdata(clump: GPtr<u8>) -> GPtr<u8> {
    clump.byte_add(g::<i32>(CLUMP_OFFSET).read() as isize)
}

fn frame_slot(frame: u32) -> GPtr<u32> {
    g::<u32>(frame.wrapping_add(g::<u32>(FRAME_OFFSET).read()) as usize)
}

/// `CVisibilityPlugins::SetModelInfoIndex(atomic, index)` (0x732230): the i16 at +0.
#[hook(addr = 0x732230, conv = cdecl, name = "CVisibilityPlugins::SetModelInfoIndex", expect = "66 8B 44 24 08 8B 4C 24 04",
       route = "L1-pure", hazards = "none")]
pub fn set_model_info_index(atomic: GPtr<u8>, index: i32) {
    wr::<u16>(adata(atomic), 0, index as u16);
}

/// `CVisibilityPlugins::GetModelInfoIndex(atomic)` (0x732250): the i16 at +0, sign-extended.
#[hook(addr = 0x732250, conv = cdecl, name = "CVisibilityPlugins::GetModelInfoIndex", expect = "8B 44 24 04 8B 0D 8C 60 8D 00 0F BF",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none")]
pub fn get_model_info_index(atomic: GPtr<u8>) -> i32 {
    rd::<i16>(adata(atomic), 0) as i32
}

/// `CVisibilityPlugins::GetModelInfo(atomic)` (0x732260): NULL for index -1, else
/// `ms_modelInfoPtrs[index]`.
#[hook(addr = 0x732260, conv = cdecl, name = "CVisibilityPlugins::GetModelInfo", expect = "8B 44 24 04 8B 0D 8C 60 8D 00 66 8B",
       route = "L1-pure", hazards = "none")]
pub fn get_model_info(atomic: GPtr<u8>) -> u32 {
    let i = rd::<i16>(adata(atomic), 0);
    if i == -1 {
        return 0;
    }
    g::<u32>(MODEL_INFO_PTRS + 4 * i as isize as usize).read()
}

/// `CVisibilityPlugins::SetAtomicId(atomic, id)` (0x732290): the u16 at +2.
#[hook(addr = 0x732290, conv = cdecl, name = "CVisibilityPlugins::SetAtomicId", expect = "66 8B 44 24 08 8B 4C 24 04 8B 15",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none")]
pub fn set_atomic_id(atomic: GPtr<u8>, id: i32) {
    wr::<u16>(adata(atomic), 2, id as u16);
}

/// `CVisibilityPlugins::SetAtomicFlag(atomic, flag)` (0x7322B0): +2 |= flag.
#[hook(addr = 0x7322B0, conv = cdecl, name = "CVisibilityPlugins::SetAtomicFlag", expect = "A1 8C 60 8D 00 8B 4C 24 04",
       route = "L1-pure", hazards = "none")]
pub fn set_atomic_flag(atomic: GPtr<u8>, flag: u32) {
    let d = adata(atomic);
    wr::<u16>(d, 2, rd::<u16>(d, 2) | flag as u16);
}

/// `setClumpForAllAtomicsFlagCB(atomic, data)` (0x7322D0): +2 |= data; returns the atomic (EAX
/// keeps it from the load).
#[hook(addr = 0x7322D0, conv = cdecl, name = "CVisibilityPlugins::setClumpForAllAtomicsFlagCB", expect = "8B 0D 8C 60 8D 00 8B 44 24 04",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none")]
pub fn set_clump_for_all_atomics_flag_cb(atomic: GPtr<u8>, data: u32) -> GPtr<u8> {
    let d = adata(atomic);
    wr::<u16>(d, 2, rd::<u16>(d, 2) | data as u16);
    atomic
}

/// `CVisibilityPlugins::SetClumpForAllAtomicsFlag(clump, flag)` (0x7322F0).
#[hook(addr = 0x7322F0, conv = cdecl, name = "CVisibilityPlugins::SetClumpForAllAtomicsFlag", expect = "8B 44 24 08 8B 4C 24 04 50 68 D0 22 73 00",
       unverified = "std6-render-1", route = "L2", hazards = "none")]
pub fn set_clump_for_all_atomics_flag(clump: GPtr<u8>, flag: u32) {
    clump_for_all_atomics(clump, 0x7322D0, flag);
}

/// `CVisibilityPlugins::ClearAtomicFlag(atomic, flag)` (0x732310): +2 &= ~flag.
#[hook(addr = 0x732310, conv = cdecl, name = "CVisibilityPlugins::ClearAtomicFlag", expect = "8B 54 24 08 A1 8C 60 8D 00",
       route = "L1-pure", hazards = "none")]
pub fn clear_atomic_flag(atomic: GPtr<u8>, flag: u32) {
    let d = adata(atomic);
    wr::<u16>(d, 2, rd::<u16>(d, 2) & !(flag as u16));
}

/// `clearClumpForAllAtomicsFlagCB(atomic, data)` (0x732330): +2 &= ~data; returns the atomic.
#[hook(addr = 0x732330, conv = cdecl, name = "CVisibilityPlugins::clearClumpForAllAtomicsFlagCB", expect = "8B 54 24 08 8B 0D 8C 60 8D 00",
       route = "L1-pure", hazards = "none")]
pub fn clear_clump_for_all_atomics_flag_cb(atomic: GPtr<u8>, data: u32) -> GPtr<u8> {
    let d = adata(atomic);
    wr::<u16>(d, 2, rd::<u16>(d, 2) & !(data as u16));
    atomic
}

/// `CVisibilityPlugins::ClearClumpForAllAtomicsFlag(clump, flag)` (0x732350).
#[hook(addr = 0x732350, conv = cdecl, name = "CVisibilityPlugins::ClearClumpForAllAtomicsFlag", expect = "8B 44 24 08 8B 4C 24 04 50 68 30 23 73 00",
       route = "L2", hazards = "none")]
pub fn clear_clump_for_all_atomics_flag(clump: GPtr<u8>, flag: u32) {
    clump_for_all_atomics(clump, 0x732330, flag);
}

/// `CVisibilityPlugins::GetAtomicId(atomic)` (0x732370): the u16 at +2, zero-extended.
#[hook(addr = 0x732370, conv = cdecl, name = "CVisibilityPlugins::GetAtomicId", expect = "8B 44 24 04 8B 0D 8C 60 8D 00 0F B7",
       route = "L1-pure", hazards = "none")]
pub fn get_atomic_id(atomic: GPtr<u8>) -> u32 {
    rd::<u16>(adata(atomic), 2) as u32
}

/// `CVisibilityPlugins::SetUserValue(atomic, value)` (0x732380): the u16 at +2.
#[hook(addr = 0x732380, conv = cdecl, name = "CVisibilityPlugins::SetUserValue", expect = "66 8B 44 24 08 8B 4C 24 04 8B 15",
       route = "L1-pure", hazards = "none")]
pub fn set_user_value(atomic: GPtr<u8>, value: u32) {
    wr::<u16>(adata(atomic), 2, value as u16);
}

/// `CVisibilityPlugins::GetUserValue(atomic)` (0x7323A0): AX = the u16 at +2; the upper half of EAX
/// keeps the atomic pointer's (`mov eax, [esp+4]` then `mov ax, ...`).
#[hook(addr = 0x7323A0, conv = cdecl, name = "CVisibilityPlugins::GetUserValue", expect = "8B 44 24 04 8B 0D 8C 60 8D 00 66 8B 44",
       route = "L1-pure", hazards = "ax-al-return")]
pub fn get_user_value(atomic: GPtr<u8>) -> u32 {
    (atomic.addr() as u32 & 0xFFFF_0000) | rd::<u16>(adata(atomic), 2) as u32
}

/// `CVisibilityPlugins::SetAlphaCB(material, data)` (0x7323B0): the material colour's alpha (+7) =
/// the low byte of `data`; returns the material.
#[hook(addr = 0x7323B0, conv = cdecl, name = "CVisibilityPlugins::SetAlphaCB", expect = "8B 44 24 04 8A 4C 24 08",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none")]
pub fn set_alpha_cb(material: GPtr<u8>, data: u32) -> GPtr<u8> {
    wr::<u8>(material, 7, data as u8);
    material
}

/// `CVisibilityPlugins::SetAtomicRenderCallback(atomic, cb)` (0x7328A0): `atomic->renderCallBack`
/// (+0x48) = cb, or `AtomicDefaultRenderCallBack` (0x7491C0) for NULL.
#[hook(addr = 0x7328A0, conv = cdecl, name = "CVisibilityPlugins::SetAtomicRenderCallback", expect = "8B 44 24 08 85 C0 75 05",
       route = "L1-pure", hazards = "none")]
pub fn set_atomic_render_callback(atomic: GPtr<u8>, cb: u32) {
    wr::<u32>(atomic, 0x48, if cb == 0 { ATOMIC_DEFAULT_RENDER_CB } else { cb });
}

/// `CVisibilityPlugins::SetFrameHierarchyId(frame, id)` (0x732A00).
#[hook(addr = 0x732A00, conv = cdecl, name = "CVisibilityPlugins::SetFrameHierarchyId", expect = "8B 44 24 08 8B 4C 24 04 8B 15 90 60 8D 00",
       route = "L1-pure", hazards = "none")]
pub fn set_frame_hierarchy_id(frame: GPtr<u8>, id: u32) {
    frame_slot(frame.addr() as u32).write(id);
}

/// `CVisibilityPlugins::GetFrameHierarchyId(frame)` (0x732A20, a protected entry: `jmp 0x4017CE`, the
/// chunk loads the frame offset into ECX and returns at 0x732A2A): the frame's hierarchy id.
#[sa_core::native(addr = 0x732A20, conv = cdecl, name = "CVisibilityPlugins::GetFrameHierarchyId")]
pub fn get_frame_hierarchy_id(frame: GPtr<u8>) -> u32 {
    frame_slot(frame.addr() as u32).read()
}

/// `CVisibilityPlugins::DefaultVisibilityCB(clump)` (0x732A30): 1.
#[hook(addr = 0x732A30, conv = cdecl, name = "CVisibilityPlugins::DefaultVisibilityCB", expect = "B8 01 00 00 00 C3",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none")]
pub fn default_visibility_cb(_clump: GPtr<u8>) -> u32 {
    1
}

/// `CVisibilityPlugins::FrustumSphereCB(clump)` (0x732A40): the clump model info's collision bounding
/// sphere (centre +0x18, radius +0x24 of the `CColModel` at +0x14) transformed by the frame's LTM and
/// tested against `Scene.m_pRwCamera`'s frustum; 1 when not outside.
#[hook(addr = 0x732A40, conv = cdecl, name = "CVisibilityPlugins::FrustumSphereCB", expect = "83 EC 10 8B 44 24 14",
       unverified = "std6-render-1", route = "L2", hazards = "none")]
pub fn frustum_sphere_cb(clump: GPtr<u8>) -> u32 {
    let frame = rd::<u32>(clump, 4);
    let mi = frame_slot(frame).read();
    let col = g::<u32>(mi as usize + 0x14).read() as usize;
    let mut sphere = [
        g::<f32>(col + 0x18).read(),
        g::<f32>(col + 0x1C).read(),
        g::<f32>(col + 0x20).read(),
        g::<f32>(col + 0x24).read(),
    ];
    let p = GPtr::from_raw(sphere.as_mut_ptr());
    let ltm = frame_get_ltm(GPtr::from_addr(frame as usize));
    v3d_transform_points(p, p, 1, ltm);
    (camera_frustum_test_sphere(GPtr::from_addr(g::<u32>(SCENE_CAMERA).read() as usize), p) != 0) as u32
}

/// `CVisibilityPlugins::VehicleVisibilityCB_BigVehicle(clump)` (0x732AB0): `jmp FrustumSphereCB`.
#[sa_core::native(addr = 0x732AB0, conv = cdecl, name = "CVisibilityPlugins::VehicleVisibilityCB_BigVehicle")]
pub fn vehicle_visibility_cb_big_vehicle(clump: GPtr<u8>) -> u32 {
    frustum_sphere_cb(clump)
}

/// `CVisibilityPlugins::GetClumpModelInfo(clump)` (0x732AC0): the hierarchy id of the clump's frame.
#[hook(addr = 0x732AC0, conv = cdecl, name = "CVisibilityPlugins::GetClumpModelInfo", expect = "8B 44 24 04 8B 48 04 8B 15 90 60 8D 00",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none")]
pub fn get_clump_model_info(clump: GPtr<u8>) -> u32 {
    frame_slot(rd::<u32>(clump, 4)).read()
}

/// `CVisibilityPlugins::IsClumpVisible(clump)` (0x732AE0): tail-calls the clump's visibility callback.
#[hook(addr = 0x732AE0, conv = cdecl, name = "CVisibilityPlugins::IsClumpVisible", expect = "8B 44 24 04 89 44 24 04",
       unverified = "std6-render-1", route = "L2", hazards = "none")]
pub fn is_clump_visible(clump: GPtr<u8>) -> u32 {
    call_visibility_cb(rd::<u32>(cdata(clump), 0), clump)
}

/// `CVisibilityPlugins::SetClumpAlpha(clump, alpha)` (0x732B00): clump data +4.
#[hook(addr = 0x732B00, conv = cdecl, name = "CVisibilityPlugins::SetClumpAlpha", expect = "8B 44 24 08 8B 4C 24 04 8B 15 94 60 8D 00",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none")]
pub fn set_clump_alpha(clump: GPtr<u8>, alpha: i32) {
    wr::<i32>(cdata(clump), 4, alpha);
}

/// `CVisibilityPlugins::GetClumpAlpha(clump)` (0x732B20): clump data +4.
#[hook(addr = 0x732B20, conv = cdecl, name = "CVisibilityPlugins::GetClumpAlpha", expect = "8B 44 24 04 8B 0D 94 60 8D 00 8B 44",
       unverified = "std6-render-1", route = "L1-pure", hazards = "none")]
pub fn get_clump_alpha(clump: GPtr<u8>) -> i32 {
    rd::<i32>(cdata(clump), 4)
}

/// `CVisibilityPlugins::SetClumpModelInfo(clump, mi)` (0x733750): the clump frame's hierarchy id =
/// mi; a vehicle model (type 6) gets the visibility callback `VehicleVisibilityCB_BigVehicle`
/// (0x732AB0) for vehicle types 6 / 8 (+0x3C), else `VehicleVisibilityCB` (0x7336F0).
#[hook(addr = 0x733750, conv = cdecl, name = "CVisibilityPlugins::SetClumpModelInfo", expect = "8B 0D 90 60 8D 00 56",
       unverified = "std6-render-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn set_clump_model_info(clump: GPtr<u8>, mi: GPtr<u8>) {
    frame_slot(rd::<u32>(clump, 4)).write(mi.addr() as u32);
    if mi_get_model_type(mi) == 6 {
        let vt = rd::<u32>(mi, 0x3C);
        let cb = if vt == 6 || vt == 8 { 0x732AB0 } else { 0x7336F0 };
        wr::<u32>(cdata(clump), 0, cb);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_SET_MODEL_INFO_INDEX,
    HOOK_GET_MODEL_INFO_INDEX,
    HOOK_GET_MODEL_INFO,
    HOOK_SET_ATOMIC_ID,
    HOOK_SET_ATOMIC_FLAG,
    HOOK_SET_CLUMP_FOR_ALL_ATOMICS_FLAG_CB,
    HOOK_SET_CLUMP_FOR_ALL_ATOMICS_FLAG,
    HOOK_CLEAR_ATOMIC_FLAG,
    HOOK_CLEAR_CLUMP_FOR_ALL_ATOMICS_FLAG_CB,
    HOOK_CLEAR_CLUMP_FOR_ALL_ATOMICS_FLAG,
    HOOK_GET_ATOMIC_ID,
    HOOK_SET_USER_VALUE,
    HOOK_GET_USER_VALUE,
    HOOK_SET_ALPHA_CB,
    HOOK_SET_ATOMIC_RENDER_CALLBACK,
    HOOK_SET_FRAME_HIERARCHY_ID,
    HOOK_DEFAULT_VISIBILITY_CB,
    HOOK_FRUSTUM_SPHERE_CB,
    HOOK_GET_CLUMP_MODEL_INFO,
    HOOK_IS_CLUMP_VISIBLE,
    HOOK_SET_CLUMP_ALPHA,
    HOOK_GET_CLUMP_ALPHA,
    HOOK_SET_CLUMP_MODEL_INFO,
];

/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[NATIVE_GET_FRAME_HIERARCHY_ID, NATIVE_VEHICLE_VISIBILITY_CB_BIG_VEHICLE];
