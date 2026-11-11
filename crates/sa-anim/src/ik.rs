//! ANIM: `IKChain_c` and `IKChainManager_c` (PORT-ANIM-INPUT, fork D). Ported from the 1.0 US bytes;
//! gta-reversed `Ragdoll/IKChain.cpp` / `IKChainManager.cpp` as reading aids,
//! `docs/re/specs/ANIM/IKChain_c.deviations.md` and `IKChainManager_c.md`. See
//! `docs/arch/anim-input-port.md`.
//!
//! `IKChain_c` (0x58): +0x00 `ListItem_c`, +0x08 `m_Ped`, +0x0C `m_BonesCount`, +0x10 `m_Bones`
//! (`BoneNode_c**`), +0x14 `m_PivotBoneMatrix`, +0x18 `m_Blend`, +0x1C `m_EffectorBone` (int16),
//! +0x20 `m_EffectorPos`, +0x2C `m_PivotBone` (int16), +0x30 `m_Entity`, +0x34 `m_OffsetBone`,
//! +0x38 `m_OffsetPos`, +0x44 `m_Speed`, +0x48 `m_TargetPos`, +0x54 `m_bUpdateTarget`, +0x55
//! `m_IKSlot`, +0x56 `m_Priority`. `IKChainManager_c` (`g_ikChainMan` 0xC15448): 32 chains, the
//! active list `TList_c` at +0xB00, the free list at +0xB0C.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::{Cmp, X87};

use crate::bones::{
    bone_info, bone_node_man, eh_vector_ctor, limit_flag, tlist_add_item, tlist_ctor, tlist_remove_all,
    tlist_remove_head, tlist_remove_item,
};
use crate::common::{at, g, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `g_ikChainMan`
    pub static G_IK_CHAIN_MAN @ 0xC15448: [u8; 0xB18] = [0; 0xB18];
}

/// `g_ikChainMan` (0xC15448).
pub fn ik_chain_man() -> GPtr<u8> {
    G_IK_CHAIN_MAN.ptr().cast()
}

/// `CWorld::ms_nCurrentScanCode` (owned by WORLD).
const CURRENT_SCAN_CODE: usize = 0xB7CD78;
/// The camera mode word `CanAccept` compares with 7 (`TheCamera` +0x81C).
const CAM_MODE_WORD: usize = 0xB6F858;
/// `TheCamera.m_matrix` / `TheCamera.placement.pos`.
const CAM_MATRIX: usize = 0xB6F03C;
const CAM_PLACEMENT_POS: usize = 0xB6F02C;

const TINY: f32 = f32::from_bits(0x3727_C5AC); // 0x858C14
const ALIGNED: f32 = f32::from_bits(0x3F7F_3B64); // 0x86D2C8 (0.997)
const RAD2DEG: f32 = f32::from_bits(0x4265_2EE0); // 0x859878
const FACING: f32 = f32::from_bits(0x3F73_3333); // 0x858EF0 (0.95)
const BLENDED: f32 = f32::from_bits(0x3F7A_E148); // 0x863E18 (0.98)
const FAR_RANGE: f32 = 999.0; // 0x866BEC

orig! {
    /// `RwV3dTransformVector(out, in, matrix)` (0x7EDDC0)
    pub fn rw_v3d_transform_vector(out: GPtr<f32>, inp: GPtr<f32>, m: GPtr<u8>) -> GPtr<f32> = cdecl 0x7EDDC0 => unported;
    /// `RwV3dTransformPoint(out, in, matrix)` (0x7EDD60)
    pub fn rw_v3d_transform_point(out: GPtr<f32>, inp: GPtr<f32>, m: GPtr<u8>) -> GPtr<f32> = cdecl 0x7EDD60 => unported;
    /// `RwV3dNormalize(out, in)` (0x7ED9B0): the length in ST0
    pub fn rw_v3d_normalize(out: GPtr<f32>, inp: GPtr<f32>) -> f64 = cdecl 0x7ED9B0 => unported;
    /// `RwV3dLength(in)` (0x7EDAC0): ST0
    pub fn rw_v3d_length(inp: GPtr<f32>) -> f64 = cdecl 0x7EDAC0 => unported;
    /// `RtQuatConvertFromMatrix(quat, matrix)` (0x7EB5C0)
    pub fn rt_quat_convert_from_matrix(q: GPtr<f32>, m: GPtr<u8>) -> u32 = cdecl 0x7EB5C0 => unported;
    /// `RtQuatTransformVectors(out, in, n, quat)` (0x7EBBB0)
    pub fn rt_quat_transform_vectors(out: GPtr<f32>, inp: GPtr<f32>, n: i32, q: GPtr<f32>) -> GPtr<f32> = cdecl 0x7EBBB0 => unported;
    /// `RtQuatRotate(quat, axis, angle, combineOp)` (0x7EB7C0)
    pub fn rt_quat_rotate(q: GPtr<f32>, axis: GPtr<f32>, angle: f32, op: i32) -> GPtr<f32> = cdecl 0x7EB7C0 => unported;
    /// CRT `acos(double)` (0x822394, the cdecl entry the `_CIacos` at 0x822380 shares its body with)
    pub fn crt_acos(v: f64) -> f64 = cdecl 0x822394 => unported;
    /// `RwHelper::GetAnimHierarchyFromSkinClump(clump)` (0x734A40)
    pub fn get_anim_hierarchy_from_skin_clump(clump: GPtr<u8>) -> GPtr<u8> = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(hierarchy, id)` (0x7C51A0)
    pub fn rp_hanim_id_get_index(h: GPtr<u8>, id: i32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(hierarchy)` (0x7C5120)
    pub fn rp_hanim_hierarchy_get_matrix_array(h: GPtr<u8>) -> GPtr<u8> = cdecl 0x7C5120 => unported;
    /// `CEntity::RegisterReference(CEntity** ref)` (0x571B70)
    pub fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CPed::GetBonePosition(RwV3d& out, boneTag, bool updateSkin)` (0x5E4280)
    pub fn ped_get_bone_position(ped: GPtr<u8>, out: GPtr<f32>, bone: i32, update: u32) = thiscall 0x5E4280 => unported;
    /// `operator new` (0x821195)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// CRT `free` (0x8213AE)
    pub fn crt_free(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    /// `BoneNodeManager_c::GetBoneNode()` (0x617460)
    pub fn get_bone_node(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x617460 => crate::bones::manager_get_bone_node;
    /// `BoneNodeManager_c::ReturnBoneNode(node)` (0x617470)
    pub fn return_bone_node(this: GPtr<u8>, node: GPtr<u8>) = thiscall 0x617470 => crate::bones::manager_return_bone_node;
    /// `BoneNode_c::Init(tag, frame)` (0x6177B0)
    pub fn bone_init(this: GPtr<u8>, tag: i32, frame: GPtr<u8>) -> u8 = thiscall 0x6177B0 => crate::bones::init;
    /// `BoneNode_c::AddChild(child)` (0x616BD0)
    pub fn bone_add_child(this: GPtr<u8>, child: GPtr<u8>) = thiscall 0x616BD0 => crate::bones::add_child;
    /// `BoneNode_c::GetIdFromBoneTag(tag)` (0x617050)
    pub fn bone_get_id_from_bone_tag(tag: i32) -> i32 = cdecl 0x617050 => crate::bones::get_id_from_bone_tag;
    /// `BoneNode_c::CalcWldMat(parent)` (0x616CD0)
    pub fn bone_calc_wld_mat(this: GPtr<u8>, parent: GPtr<u8>) = thiscall 0x616CD0 => crate::bones::calc_wld_mat;
    /// `BoneNode_c::BlendKeyframe(blend)` (0x616E30)
    pub fn bone_blend_keyframe(this: GPtr<u8>, blend: f32) = thiscall 0x616E30 => crate::bones::blend_keyframe;
    /// `BoneNode_c::Limit(blend)` (0x617650)
    pub fn bone_limit(this: GPtr<u8>, blend: f32) = thiscall 0x617650 => crate::bones::limit;
    /// `BoneNode_c::GetSpeed()` (0x616CB0)
    pub fn bone_get_speed(this: GPtr<u8>) -> f64 = thiscall 0x616CB0 => bone_get_speed_host;
    /// `BoneNode_c::SetSpeed(speed)` (0x616CC0)
    pub fn bone_set_speed(this: GPtr<u8>, speed: u32) = thiscall 0x616CC0 => crate::bones::set_speed;
    /// `BoneNode_c::GetLimits(axis, &min, &max)` (0x616BF0)
    pub fn bone_get_limits(this: GPtr<u8>, axis: i32, min: GPtr<u32>, max: GPtr<u32>) = thiscall 0x616BF0 => crate::bones::get_limits;
    /// `BoneNode_c::SetLimits(axis, min, max)` (0x616C50)
    pub fn bone_set_limits(this: GPtr<u8>, axis: i32, min: u32, max: u32) = thiscall 0x616C50 => crate::bones::set_limits;
    /// `BoneNode_c::ClampLimitsDefault(x, y, z)` (0x617530)
    pub fn bone_clamp_limits_default(this: GPtr<u8>, lx: u32, ly: u32, lz: u32) = thiscall 0x617530 => bone_clamp_default_host;
    /// `BoneNode_c::ClampLimitsCurrent(x, y, z)` (0x6175D0)
    pub fn bone_clamp_limits_current(this: GPtr<u8>, lx: u32, ly: u32, lz: u32) = thiscall 0x6175D0 => bone_clamp_current_host;
    /// `IKChain_c::SetupBones(effector, effectorPos, pivot, frames)` (0x617CA0)
    pub fn call_setup_bones(this: GPtr<u8>, effector: i32, ex: u32, ey: u32, ez: u32, pivot: i32, frames: GPtr<u8>) = thiscall 0x617CA0 => setup_bones;
    /// `IKChain_c::MoveBonesToTarget()` (0x6178B0)
    pub fn call_move_bones_to_target(this: GPtr<u8>) = thiscall 0x6178B0 => move_bones_to_target;
    /// `IKChain_c::Exit()` (0x617870)
    pub fn call_chain_exit(this: GPtr<u8>) = thiscall 0x617870 => exit;
    /// `IKChain_c::Update(deltaTime)` (0x6184B0)
    pub fn call_chain_update(this: GPtr<u8>, dt: f32) = thiscall 0x6184B0 => update;
    /// `IKChain_c::IsFacingTarget()` (0x617E60)
    pub fn call_chain_is_facing_target(this: GPtr<u8>) -> u8 = thiscall 0x617E60 => is_facing_target;
    /// `IKChain_c::Init(...)` (0x618370)
    pub fn call_chain_init(this: GPtr<u8>, name: u32, slot: i32, ped: GPtr<u8>, effector: i32, ex: u32, ey: u32, ez: u32, pivot: i32, entity: GPtr<u8>, offset_bone: i32, ox: u32, oy: u32, oz: u32, speed: u32, priority: i32) -> u8 = thiscall 0x618370 => init;
    /// `IKChainManager_c::Exit()` (0x6180D0)
    pub fn call_manager_exit(this: GPtr<u8>) = thiscall 0x6180D0 => manager_exit;
    /// `IKChainManager_c::CanAccept(ped, range)` (0x618800)
    pub fn call_can_accept(this: GPtr<u8>, ped: GPtr<u8>, range: f32) -> u8 = thiscall 0x618800 => can_accept;
    /// `IKChainManager_c::AbortLookAt(ped, blendTime)` (0x618280)
    pub fn call_abort_look_at(this: GPtr<u8>, ped: GPtr<u8>, time: i32) = thiscall 0x618280 => abort_look_at;
    /// `CTaskManager::GetTaskSecondary(slot)` (0x681810)
    pub fn task_manager_get_task_secondary(tm: GPtr<u8>, slot: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskSimpleIKManager::GetTaskAtSlot(slot)` (0x6339B0)
    pub fn ik_manager_task_get_task_at_slot(t: GPtr<u8>, slot: i32) -> GPtr<u8> = thiscall 0x6339B0 => unported;
    /// `CTaskSimpleIKLookAt::GetLookAtEntity()` (0x634120)
    pub fn look_at_task_get_entity(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x634120 => unported;
    /// `CTaskSimpleIKLookAt::GetLookAtOffset(CVector* out)` (0x634130): returns out
    pub fn look_at_task_get_offset(t: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x634130 => unported;
    /// `CTaskSimpleIKChain::BlendOut(time)` (0x633C40)
    pub fn ik_chain_task_blend_out(t: GPtr<u8>, time: i32) = thiscall 0x633C40 => unported;
    /// `CTaskSimpleIKChain::GetIKChain()` (0x633C70)
    pub fn ik_chain_task_get_ik_chain(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x633C70 => unported;
    /// `CPed::IsAlive()` (0x5E0170)
    pub fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CEntity::GetIsOnScreen()` (0x534540)
    pub fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    pub fn vector_sub(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x40FE60 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0): ST0
    pub fn vector_magnitude_sqr(v: GPtr<f32>) -> f64 = thiscall 0x406DA0 => unported;
    /// `RpAnimBlendClumpGetAssociation(clump, animId)` (0x4D68B0)
    pub fn rp_anim_blend_clump_get_association(clump: GPtr<u8>, anim: i32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CEntity::UpdateRpHAnim()` (0x532B20)
    pub fn entity_update_rp_hanim(e: GPtr<u8>) = thiscall 0x532B20 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    pub fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `IKChainManager_c::CanAcceptLookAt(ped)` (0x6188B0, AL)
    pub fn call_can_accept_look_at(this: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6188B0 => can_accept_look_at;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    pub fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskSimpleIKManager::CTaskSimpleIKManager()` (0x6337F0): returns `this`
    pub fn ik_manager_task_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x6337F0 => unported;
    /// `CTaskManager::SetTaskSecondary(CTask*, slot)` (0x681B60)
    pub fn task_manager_set_task_secondary(tm: GPtr<u8>, task: GPtr<u8>, slot: i32) = thiscall 0x681B60 => unported;
    /// `CTaskSimpleIKManager::AddIKChainTask(CTaskSimpleIKChain*, slot)` (0x633940)
    pub fn ik_manager_task_add_ik_chain_task(t: GPtr<u8>, task: GPtr<u8>, slot: i32) = thiscall 0x633940 => unported;
    /// `CTaskSimpleIKLookAt::UpdateLookAtInfo(purpose, ped, entity, time, offsetBone, CVector offset (by
    /// value), useTorso, speed, blendTime, priority)` (0x634050)
    pub fn look_at_task_update_look_at_info(t: GPtr<u8>, purpose: u32, ped: GPtr<u8>, entity: u32, time: u32, bone: u32, ox: u32, oy: u32, oz: u32, use_torso: u32, speed: u32, blend: u32, priority: u32) = thiscall 0x634050 => unported;
    /// `CTaskSimpleIKLookAt::CTaskSimpleIKLookAt(purpose, entity, time, offsetBone, CVector offset (by
    /// value), useTorso, speed, blendTime, priority)` (0x633E00): returns `this`
    pub fn look_at_task_ctor(t: GPtr<u8>, purpose: u32, entity: u32, time: u32, bone: u32, ox: u32, oy: u32, oz: u32, use_torso: u32, speed: u32, blend: u32, priority: u32) -> GPtr<u8> = thiscall 0x633E00 => unported;
}

/// The byte at 0xC1542C (unnamed in the catalogue): non-zero refuses every unforced look-at.
const NO_LOOK_AT: usize = 0xC1542C;

/// Phase B/C path of [`bone_get_speed`].
pub fn bone_get_speed_host(this: GPtr<u8>) -> f64 {
    crate::bones::get_speed(this).to_f64()
}
/// Phase B/C path of [`bone_clamp_limits_default`] (bool arguments pushed as dwords, read as bytes).
pub fn bone_clamp_default_host(this: GPtr<u8>, lx: u32, ly: u32, lz: u32) {
    crate::bones::clamp_limits_default(this, lx as u8, ly as u8, lz as u8)
}
/// Phase B/C path of [`bone_clamp_limits_current`].
pub fn bone_clamp_current_host(this: GPtr<u8>, lx: u32, ly: u32, lz: u32) {
    crate::bones::clamp_limits_current(this, lx as u8, ly as u8, lz as u8)
}

/// `m_Bones[i]`.
fn bone(this: GPtr<u8>, i: i32) -> GPtr<u8> {
    rd::<GPtr<GPtr<u8>>>(this, 0x10).wrapping_index(i).read()
}

/// The first of `m_Bones` whose tag (+8) equals `tag`, else null (the inlined `GetBoneNodeFromTag`).
fn find_bone(this: GPtr<u8>, tag: i32) -> GPtr<u8> {
    let n = rd::<i32>(this, 0xC);
    let mut i = 0;
    while i < n {
        let b = bone(this, i);
        if rd::<i32>(b, 8) == tag {
            return b;
        }
        i += 1;
    }
    GPtr::NULL
}

/// `IKChain_c::Exit()` (0x617870): every bone back to `g_boneNodeMan`, `free(m_Bones)`, `m_Bones = 0`.
#[hook(addr = 0x617870, conv = thiscall, name = "IKChain_c::Exit", expect = "56 8B F1 8B 46 0C", this = "IKChain_c")]
pub fn exit(this: GPtr<u8>) {
    let mut i = 0;
    while i < rd::<i32>(this, 0xC) {
        return_bone_node(bone_node_man(), bone(this, i));
        i += 1;
    }
    crt_free(rd::<GPtr<u8>>(this, 0x10));
    wr::<u32>(this, 0x10, 0);
}

/// `IKChain_c::SetBlend(blend)` (0x617C10): raw dword store at +0x18.
#[hook(addr = 0x617C10, conv = thiscall, name = "IKChain_c::SetBlend", expect = "8B 44 24 04 89 41 18", this = "IKChain_c")]
pub fn set_blend(this: GPtr<u8>, blend: u32) {
    wr::<u32>(this, 0x18, blend);
}

/// `IKChain_c::SetOffsetBoneTag(tag)` (0x617C20): dword store at +0x34.
#[hook(addr = 0x617C20, conv = thiscall, name = "IKChain_c::SetOffsetBoneTag", expect = "8B 44 24 04 89 41 34", this = "IKChain_c")]
pub fn set_offset_bone_tag(this: GPtr<u8>, tag: i32) {
    wr::<i32>(this, 0x34, tag);
}

/// `IKChain_c::SetOffsetPos(CVector pos)` (0x617C30, by value): +0x38..+0x40.
#[hook(addr = 0x617C30, conv = thiscall, name = "IKChain_c::SetOffsetPos", expect = "8B 44 24 04 8B 54 24 08", this = "IKChain_c")]
pub fn set_offset_pos(this: GPtr<u8>, px: u32, py: u32, pz: u32) {
    wr::<u32>(this, 0x38, px);
    wr::<u32>(this, 0x3C, py);
    wr::<u32>(this, 0x40, pz);
}

/// `IKChain_c::GetPriority()` (0x617C50): `movsx eax, byte [+0x56]`. Not hookable: 5 bytes ending
/// in `ret` (the steal would include the `ret`).
#[sa_core::native(addr = 0x617C50, conv = thiscall, name = "IKChain_c::GetPriority")]
pub fn get_priority(this: GPtr<u8>) -> i32 {
    i32::from(rd::<i8>(this, 0x56))
}

/// `IKChain_c::GetBoneNodeFromTag(tag)` (0x617C60): first bone with that tag, else null.
#[hook(addr = 0x617C60, conv = thiscall, name = "IKChain_c::GetBoneNodeFromTag", expect = "8B 51 0C 53 56", this = "IKChain_c")]
pub fn get_bone_node_from_tag(this: GPtr<u8>, tag: i32) -> GPtr<u8> {
    find_bone(this, tag)
}

/// `IKChain_c::SetupBones(effector, CVector effectorPos (unused), pivot, AnimBlendFrameData* frames)`
/// (0x617CA0): walk the parent chain from the effector up to (not including) the pivot, one bone
/// node each (no stop other than the pivot, no bound on the 32-entry local array: original bugs
/// kept for valid chains), copy into a new `m_Bones` array, then link each bone to its parent bone
/// in the chain. `m_PivotBoneMatrix` = the pivot's matrix in the skin hierarchy.
#[hook(addr = 0x617CA0, conv = thiscall, name = "IKChain_c::SetupBones", expect = "81 EC 80 00 00 00", this = "IKChain_c")]
#[allow(clippy::too_many_arguments)]
pub fn setup_bones(this: GPtr<u8>, effector: i32, _ex: u32, _ey: u32, _ez: u32, pivot: i32, frames: GPtr<u8>) {
    wr::<i32>(this, 0xC, 0);
    let h = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0x18));
    let pi = rp_hanim_id_get_index(h, pivot);
    let m = rp_hanim_hierarchy_get_matrix_array(h);
    wr::<u32>(this, 0x14, m.addr().wrapping_add((pi << 6) as u32 as usize) as u32);
    wr::<u16>(this, 0x2C, pivot as u16);
    let mut local = [GPtr::<u8>::NULL; 32];
    let mut tag = effector;
    while tag != pivot {
        let node = get_bone_node(bone_node_man());
        let idx = rp_hanim_id_get_index(h, tag);
        let kf = rd::<GPtr<u8>>(frames, (idx.wrapping_mul(0x18) + 0x10) as isize);
        bone_init(node, tag, kf);
        let n = rd::<i32>(this, 0xC);
        local[n as usize] = node;
        wr::<i32>(this, 0xC, n + 1);
        let id = bone_get_id_from_bone_tag(tag);
        tag = i32::from(rd::<i16>(bone_info(id), 2));
    }
    let n = rd::<i32>(this, 0xC);
    let arr = operator_new((n as u32).wrapping_shl(2));
    wr::<u32>(this, 0x10, arr.addr() as u32);
    let mut i = 0;
    while i < rd::<i32>(this, 0xC) {
        arr.cast::<GPtr<u8>>().wrapping_index(i).write(local[i as usize]);
        i += 1;
    }
    let mut i = 0;
    while i < rd::<i32>(this, 0xC) {
        let id = bone_get_id_from_bone_tag(rd::<i32>(bone(this, i), 8));
        let parent = i32::from(rd::<i16>(bone_info(id), 2));
        if parent > -1 {
            let p = find_bone(this, parent);
            if !p.is_null() {
                bone_add_child(p, bone(this, i));
            }
        }
        i += 1;
    }
}

/// `IKChain_c::MoveBonesToTarget()` (0x6178B0): refresh the target (entity bone / matrix + offset,
/// or the plain offset) when `m_bUpdateTarget`; then per bone: the effector position (bone 0's world
/// position + its world-rotated `m_EffectorPos`), the unit vectors bone→effector and bone→target
/// (skipped when either length <= 1e-5), and unless their dot >= 0.997 rotate the bone by
/// `acos(dot) * bone speed * m_Speed * 57.29578` degrees about their cross product expressed in the
/// parent's frame (the inverse of `RtQuatConvertFromMatrix(parent)`; the inverse quaternion stays
/// zero when the squared norm is <= 0: uninitialised stack in the original), `Limit(m_Blend)` when
/// the 0x8D2BD2 flag is set, `CalcWldMat`.
#[hook(addr = 0x6178B0, conv = thiscall, name = "IKChain_c::MoveBonesToTarget", expect = "83 EC 7C 53 55", this = "IKChain_c")]
pub fn move_bones_to_target(this: GPtr<u8>) {
    let b0 = bone(this, 0);
    let target = at::<f32>(this, 0x48);
    let tf = |k: isize| rd::<f32>(this, 0x48 + 4 * k);
    if rd::<u8>(this, 0x54) != 0 {
        let ent = rd::<GPtr<u8>>(this, 0x30);
        if ent.is_null() {
            for k in 0..3 {
                wr::<u32>(this, 0x48 + 4 * k, rd::<u32>(this, 0x38 + 4 * k));
            }
        } else {
            let obj = rd::<GPtr<u8>>(ent, 0x18);
            let mat = if obj.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(obj, 4).byte_add(0x10) };
            let tag = rd::<i32>(this, 0x34);
            if tag != -1 {
                ped_get_bone_position(ent, target, tag, 0);
                if !mat.is_null() {
                    let mut t = [0f32; 3];
                    rw_v3d_transform_vector(GPtr::from_raw(t.as_mut_ptr()), at(this, 0x38), mat);
                    for (k, v) in t.iter().enumerate() {
                        let k = k as isize;
                        wr::<f32>(this, 0x48 + 4 * k, x(*v).fadd_m(tf(k)).to_f32());
                    }
                }
            } else if !mat.is_null() {
                rw_v3d_transform_point(target, at(this, 0x38), mat);
            }
        }
    }
    let mut i = 0;
    while i < rd::<i32>(this, 0xC) {
        let b = bone(this, i);
        let bp = [rd::<f32>(b, 0x6C), rd::<f32>(b, 0x70), rd::<f32>(b, 0x74)];
        let p0 = [rd::<f32>(b0, 0x6C), rd::<f32>(b0, 0x70), rd::<f32>(b0, 0x74)];
        let mut t = [0f32; 3];
        rw_v3d_transform_vector(GPtr::from_raw(t.as_mut_ptr()), at(this, 0x20), b0.byte_add(0x3C));
        let ea = x(t[0]).fadd_m(p0[0]);
        let eb = x(t[1]).fadd_m(p0[1]);
        let ec = x(t[2]).fadd_m(p0[2]).to_f32();
        let mut d1 = [ea.fsub_m(bp[0]).to_f32(), eb.fsub_m(bp[1]).to_f32(), x(ec).fsub_m(bp[2]).to_f32()];
        let skip = |len: f64| X87::from_f64_exact(len).fcom(x(TINY)) != Cmp::Greater;
        if skip(rw_v3d_length(GPtr::from_raw(d1.as_mut_ptr()))) {
            i += 1;
            continue;
        }
        let d1p = GPtr::from_raw(d1.as_mut_ptr());
        rw_v3d_normalize(d1p, d1p);
        let mut d2 = [
            x(tf(0)).fsub_m(bp[0]).to_f32(),
            x(tf(1)).fsub_m(bp[1]).to_f32(),
            x(tf(2)).fsub_m(bp[2]).to_f32(),
        ];
        if skip(rw_v3d_length(GPtr::from_raw(d2.as_mut_ptr()))) {
            i += 1;
            continue;
        }
        let d2p = GPtr::from_raw(d2.as_mut_ptr());
        rw_v3d_normalize(d2p, d2p);
        let dot = x(d2[1])
            .fmul_m(d1[1])
            .fadd(x(d2[0]).fmul_m(d1[0]))
            .fadd(x(d2[2]).fmul_m(d1[2]))
            .to_f32();
        // test ah, 5 / jp: >= 0.997 or NaN skips
        if x(dot).fcom(x(ALIGNED)) != Cmp::Less {
            i += 1;
            continue;
        }
        let speed = X87::from_f64_exact(bone_get_speed(b));
        let ang = X87::from_f64_exact(crt_acos(f64::from(dot)));
        let angle = speed.fmul(ang).fmul_m(rd::<f32>(this, 0x44)).fmul_m(RAD2DEG).to_f32();
        let mut axis = [
            x(d2[2]).fmul_m(d1[1]).fsub(x(d2[1]).fmul_m(d1[2])).to_f32(),
            x(d2[0]).fmul_m(d1[2]).fsub(x(d2[2]).fmul_m(d1[0])).to_f32(),
            x(d2[1]).fmul_m(d1[0]).fsub(x(d2[0]).fmul_m(d1[1])).to_f32(),
        ];
        let parent = rd::<GPtr<u8>>(b, 0x2C);
        let pm = if parent.is_null() { rd::<GPtr<u8>>(this, 0x14) } else { parent.byte_add(0x3C) };
        let mut q = [0f32; 4];
        rt_quat_convert_from_matrix(GPtr::from_raw(q.as_mut_ptr()), pm);
        let n = x(q[1])
            .fmul_m(q[1])
            .fadd(x(q[0]).fmul_m(q[0]))
            .fadd(x(q[2]).fmul_m(q[2]))
            .fadd(x(q[3]).fmul_m(q[3]));
        let mut qi = [0f32; 4];
        if n.fcom(x(0.0)) == Cmp::Greater {
            let inv = n.fdivr_m(1.0);
            qi[3] = x(q[3]).fmul(inv).to_f32();
            let ninv = inv.fchs();
            qi[0] = x(q[0]).fmul(ninv).to_f32();
            qi[1] = x(q[1]).fmul(ninv).to_f32();
            qi[2] = x(q[2]).fmul(ninv).to_f32();
        }
        let mut axis2 = [0f32; 3];
        rt_quat_transform_vectors(
            GPtr::from_raw(axis2.as_mut_ptr()),
            GPtr::from_raw(axis.as_mut_ptr()),
            1,
            GPtr::from_raw(qi.as_mut_ptr()),
        );
        rt_quat_rotate(at(b, 0x10), GPtr::from_raw(axis2.as_mut_ptr()), angle, 2);
        if limit_flag() != 0 {
            bone_limit(b, rd::<f32>(this, 0x18));
        }
        let parent = rd::<GPtr<u8>>(b, 0x2C);
        let pm = if parent.is_null() { rd::<GPtr<u8>>(this, 0x14) } else { parent.byte_add(0x3C) };
        bone_calc_wld_mat(b, pm);
        i += 1;
    }
}

/// `IKChain_c::UpdateEntity(entity)` (0x617E00): `m_Entity = entity`; when non-null, tail-jump
/// `entity->RegisterReference(&m_Entity)`.
#[hook(addr = 0x617E00, conv = thiscall, name = "IKChain_c::UpdateEntity", expect = "8D 41 30 8B 4C 24 04", this = "IKChain_c")]
pub fn update_entity(this: GPtr<u8>, entity: GPtr<u8>) {
    wr::<u32>(this, 0x30, entity.addr() as u32);
    if !entity.is_null() {
        register_reference(entity, this.byte_add(0x30));
    }
}

/// `IKChain_c::UpdateOffset(offsetBone, CVector offsetPos)` (0x617E20).
#[hook(addr = 0x617E20, conv = thiscall, name = "IKChain_c::UpdateOffset", expect = "8B 44 24 04 8B 54 24 08", this = "IKChain_c")]
pub fn update_offset(this: GPtr<u8>, tag: i32, px: u32, py: u32, pz: u32) {
    wr::<i32>(this, 0x34, tag);
    wr::<u32>(this, 0x38, px);
    wr::<u32>(this, 0x3C, py);
    wr::<u32>(this, 0x40, pz);
}

/// `IKChain_c::UpdateTarget(bool)` (0x617E50): byte store at +0x54.
#[hook(addr = 0x617E50, conv = thiscall, name = "IKChain_c::UpdateTarget", expect = "8A 44 24 04 88 41 54", this = "IKChain_c")]
pub fn update_target(this: GPtr<u8>, update: u8) {
    wr::<u8>(this, 0x54, update);
}

/// The world position of bone 0 (+0x6C) and the effector direction: `RwV3dTransformVector` of
/// `m_EffectorPos` by bone 0's world matrix, normalised.
fn effector_dir(this: GPtr<u8>) -> ([f32; 3], [f32; 3]) {
    let b0 = bone(this, 0);
    let pos = [rd::<f32>(b0, 0x6C), rd::<f32>(b0, 0x70), rd::<f32>(b0, 0x74)];
    let mut dir = [0f32; 3];
    let dp = GPtr::from_raw(dir.as_mut_ptr());
    rw_v3d_transform_vector(dp, at(this, 0x20), b0.byte_add(0x3C));
    rw_v3d_normalize(dp, dp);
    (pos, dir)
}

/// `IKChain_c::IsFacingTarget()` (0x617E60): `dot(effector dir, normalise(target - bone 0)) >= 0.95`
/// (`test ah, 1`) and `m_Blend > 0.98`.
#[hook(addr = 0x617E60, conv = thiscall, name = "IKChain_c::IsFacingTarget", expect = "83 EC 24 56 8B F1", this = "IKChain_c")]
pub fn is_facing_target(this: GPtr<u8>) -> u8 {
    let (pos, dir) = effector_dir(this);
    let tf = |k: isize| rd::<f32>(this, 0x48 + 4 * k);
    let mut d = [
        x(tf(0)).fsub_m(pos[0]).to_f32(),
        x(tf(1)).fsub_m(pos[1]).to_f32(),
        x(tf(2)).fsub_m(pos[2]).to_f32(),
    ];
    let dp = GPtr::from_raw(d.as_mut_ptr());
    rw_v3d_normalize(dp, dp);
    let dot = x(dir[1]).fmul_m(d[1]).fadd(x(dir[0]).fmul_m(d[0])).fadd(x(dir[2]).fmul_m(d[2]));
    if matches!(dot.fcom(x(FACING)), Cmp::Less | Cmp::Unordered) {
        return 0;
    }
    u8::from(x(rd::<f32>(this, 0x18)).fcom(x(BLENDED)) == Cmp::Greater)
}

/// `IKChain_c::IsAtTarget(float range, float* outDist)` (0x617F30): `|target - bone 0|` (stored to
/// `*outDist` when non-null) <= range, and `m_Blend > 0.98`.
#[hook(addr = 0x617F30, conv = thiscall, name = "IKChain_c::IsAtTarget", expect = "83 EC 18 56 8B F1", this = "IKChain_c")]
pub fn is_at_target(this: GPtr<u8>, range: f32, out: GPtr<f32>) -> u8 {
    let b0 = bone(this, 0);
    let tf = |k: isize| rd::<f32>(this, 0x48 + 4 * k);
    let mut d = [
        x(tf(0)).fsub_m(rd::<f32>(b0, 0x6C)).to_f32(),
        x(tf(1)).fsub_m(rd::<f32>(b0, 0x70)).to_f32(),
        x(tf(2)).fsub_m(rd::<f32>(b0, 0x74)).to_f32(),
    ];
    let len = X87::from_f64_exact(rw_v3d_length(GPtr::from_raw(d.as_mut_ptr())));
    if !out.is_null() {
        out.write(len.to_f32());
    }
    if !matches!(len.fcom(x(range)), Cmp::Less | Cmp::Equal) {
        return 0;
    }
    u8::from(x(rd::<f32>(this, 0x18)).fcom(x(BLENDED)) == Cmp::Greater)
}

/// `IKChain_c::Init(name, slot, ped, effector, CVector effectorPos, pivot, entity, offsetBone,
/// CVector offsetPos, speed, priority)` (0x618370; `name` unused): `m_Ped = ped` first; fails (AL 0)
/// when the ped's frame data flags have bit 5 set or bit 7 clear, when the ped lacks flag
/// 0x20000000 (+0x474), or when the effector's interp-frame translation is exactly zero; otherwise
/// registers the ped reference, `m_Bones = 0`, `SetupBones`, stores the rest and returns 1.
#[hook(addr = 0x618370, conv = thiscall, name = "IKChain_c::Init", expect = "8B 54 24 0C 53", this = "IKChain_c")]
#[allow(clippy::too_many_arguments)]
pub fn init(
    this: GPtr<u8>,
    _name: u32,
    slot: i32,
    ped: GPtr<u8>,
    effector: i32,
    ex: u32,
    ey: u32,
    ez: u32,
    pivot: i32,
    entity: GPtr<u8>,
    offset_bone: i32,
    ox: u32,
    oy: u32,
    oz: u32,
    speed: u32,
    priority: i32,
) -> u8 {
    wr::<u32>(this, 8, ped.addr() as u32);
    let clump = rd::<GPtr<u8>>(ped, 0x18);
    // RpAnimBlendClumpData via the plugin offset (0xB5F878), its frames (+0x10)
    let off = g::<i32>(0xB5F878).read();
    let blend = rd::<GPtr<u8>>(clump, off as isize);
    let frames = rd::<GPtr<u8>>(blend, 0x10);
    let fl = rd::<u8>(frames, 0);
    if fl & 0x20 != 0 || fl & 0x80 == 0 || rd::<u32>(ped, 0x474) & 0x2000_0000 == 0 {
        return 0;
    }
    let h = get_anim_hierarchy_from_skin_clump(clump);
    let idx = rp_hanim_id_get_index(h, effector);
    let kf = rd::<GPtr<u8>>(frames, (idx.wrapping_mul(0x18) + 0x10) as isize);
    let nonzero = |o: isize| x(rd::<f32>(kf, o)).fcom(x(0.0)) != Cmp::Equal;
    if !(nonzero(0x10) || nonzero(0x14) || nonzero(0x18)) {
        return 0;
    }
    register_reference(rd::<GPtr<u8>>(this, 8), this.byte_add(8));
    wr::<u32>(this, 0x10, 0);
    call_setup_bones(this, effector, ex, ey, ez, pivot, frames);
    wr::<u32>(this, 0x20, ex);
    wr::<u32>(this, 0x24, ey);
    wr::<u32>(this, 0x28, ez);
    wr::<u32>(this, 0x18, 0);
    wr::<u16>(this, 0x1C, effector as u16);
    wr::<u32>(this, 0x30, entity.addr() as u32);
    if !entity.is_null() {
        register_reference(entity, this.byte_add(0x30));
    }
    wr::<i32>(this, 0x34, offset_bone);
    wr::<u32>(this, 0x38, ox);
    wr::<u32>(this, 0x3C, oy);
    wr::<u32>(this, 0x40, oz);
    wr::<u32>(this, 0x44, speed);
    wr::<u8>(this, 0x54, 1);
    wr::<u8>(this, 0x55, slot as u8);
    wr::<u8>(this, 0x56, priority as u8);
    1
}

/// `IKChain_c::Update(deltaTime)` (0x6184B0; `deltaTime` unused): refresh `m_PivotBoneMatrix`,
/// `CalcWldMat` of the last bone from it, `MoveBonesToTarget`, `BlendKeyframe(m_Blend)` per bone.
#[hook(addr = 0x6184B0, conv = thiscall, name = "IKChain_c::Update", expect = "53 56 8B F1 8B 46 08", this = "IKChain_c")]
pub fn update(this: GPtr<u8>, _dt: f32) {
    let h = get_anim_hierarchy_from_skin_clump(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 8), 0x18));
    let pi = rp_hanim_id_get_index(h, i32::from(rd::<i16>(this, 0x2C)));
    let m = rp_hanim_hierarchy_get_matrix_array(h);
    let pm = m.byte_add((pi << 6) as isize);
    wr::<u32>(this, 0x14, pm.addr() as u32);
    bone_calc_wld_mat(bone(this, rd::<i32>(this, 0xC) - 1), pm);
    call_move_bones_to_target(this);
    let mut i = 0;
    while i < rd::<i32>(this, 0xC) {
        bone_blend_keyframe(bone(this, i), rd::<f32>(this, 0x18));
        i += 1;
    }
}

/// `IKChain_c::ClampLimits(tag, x, y, z, current)` (0x618520): `ClampLimitsCurrent` / `Default` of
/// the bone with that tag (called with a null `this` when there is none: original bug kept).
#[hook(addr = 0x618520, conv = thiscall, name = "IKChain_c::ClampLimits", expect = "8B 51 0C 53 56", this = "IKChain_c")]
pub fn clamp_limits(this: GPtr<u8>, tag: i32, lx: u32, ly: u32, lz: u32, current: u8) {
    let b = find_bone(this, tag);
    if current != 0 {
        bone_clamp_limits_current(b, lx, ly, lz);
    } else {
        bone_clamp_limits_default(b, lx, ly, lz);
    }
}

/// `IKChain_c::GetLimits(tag, axis, &min, &max)` (0x618590).
#[hook(addr = 0x618590, conv = thiscall, name = "IKChain_c::GetLimits", expect = "8B 51 0C 53 56", this = "IKChain_c")]
pub fn get_limits(this: GPtr<u8>, tag: i32, axis: i32, min: GPtr<u32>, max: GPtr<u32>) {
    bone_get_limits(find_bone(this, tag), axis, min, max);
}

/// `IKChain_c::SetLimits(tag, axis, min, max)` (0x6185E0).
#[hook(addr = 0x6185E0, conv = thiscall, name = "IKChain_c::SetLimits", expect = "8B 51 0C 53 56", this = "IKChain_c")]
pub fn set_limits(this: GPtr<u8>, tag: i32, axis: i32, min: u32, max: u32) {
    bone_set_limits(find_bone(this, tag), axis, min, max);
}

/// `IKChain_c::GetBoneSpeed(tag)` (0x618630): `GetSpeed` of the bone (ST0).
#[hook(addr = 0x618630, conv = thiscall, name = "IKChain_c::GetBoneSpeed", expect = "8B 51 0C 53 56", this = "IKChain_c")]
pub fn get_bone_speed(this: GPtr<u8>, tag: i32) -> X87 {
    X87::from_f64_exact(bone_get_speed(find_bone(this, tag)))
}

/// `IKChain_c::SetBoneSpeed(tag, speed)` (0x618680).
#[hook(addr = 0x618680, conv = thiscall, name = "IKChain_c::SetBoneSpeed", expect = "8B 51 0C 53 56", this = "IKChain_c")]
pub fn set_bone_speed(this: GPtr<u8>, tag: i32, speed: u32) {
    bone_set_speed(find_bone(this, tag), speed);
}

/// `IKChainManager_c::IKChainManager_c` (0x617FC0): the 32 chains (`eh vector constructor
/// iterator`, ctor 0x617850 = `ListItem_c` ctor, dtor 0x617860), both lists' ctors; returns this.
#[hook(addr = 0x617FC0, conv = thiscall, name = "IKChainManager_c::IKChainManager_c", expect = "6A FF 68 61 F0 83 00")]
pub fn manager_constructor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(this, 0x58, 0x20, 0x617850, 0x617860);
    tlist_ctor(this.byte_add(0xB00));
    tlist_ctor(this.byte_add(0xB0C));
    this
}

/// `IKChainManager_c::Init()` (0x6180A0): every chain onto the free list; AL = 1.
#[hook(addr = 0x6180A0, conv = thiscall, name = "IKChainManager_c::Init", expect = "53 56 57 8D 99 0C 0B 00 00")]
pub fn manager_init(this: GPtr<u8>) -> u8 {
    for i in 0..0x20 {
        tlist_add_item(this.byte_add(0xB0C), this.byte_add(i * 0x58));
    }
    1
}

/// `IKChainManager_c::Exit()` (0x6180D0): for every active chain the inlined `IKChain_c::Exit`
/// (bones back, `free(m_Bones)`, null), then `RemoveAll` on both lists.
#[hook(addr = 0x6180D0, conv = thiscall, name = "IKChainManager_c::Exit", expect = "53 55 8B D9 8D AB 00 0B 00 00")]
pub fn manager_exit(this: GPtr<u8>) {
    let mut c = rd::<GPtr<u8>>(this, 0xB00);
    while !c.is_null() {
        let mut i = 0;
        while i < rd::<i32>(c, 0xC) {
            return_bone_node(bone_node_man(), bone(c, i));
            i += 1;
        }
        crt_free(rd::<GPtr<u8>>(c, 0x10));
        wr::<u32>(c, 0x10, 0);
        c = rd::<GPtr<u8>>(c, 4);
    }
    tlist_remove_all(this.byte_add(0xB00));
    tlist_remove_all(this.byte_add(0xB0C));
}

/// `IKChainManager_c::Reset()` (0x618140): `Exit()`, then every chain onto the free list.
#[hook(addr = 0x618140, conv = thiscall, name = "IKChainManager_c::Reset", expect = "53 56 57 8B F1")]
pub fn manager_reset(this: GPtr<u8>) {
    call_manager_exit(this);
    for i in 0..0x20 {
        tlist_add_item(this.byte_add(0xB0C), this.byte_add(i * 0x58));
    }
}

/// `IKChainManager_c::RemoveIKChain(chain)` (0x618170): `chain->Exit()`, off the active list, onto
/// the free list.
#[hook(addr = 0x618170, conv = thiscall, name = "IKChainManager_c::RemoveIKChain", expect = "56 57 8B 7C 24 0C")]
pub fn manager_remove_ik_chain(this: GPtr<u8>, chain: GPtr<u8>) {
    call_chain_exit(chain);
    tlist_remove_item(this.byte_add(0xB00), chain);
    tlist_add_item(this.byte_add(0xB0C), chain);
}

/// The ped's secondary IK task manager (`ped->m_pIntelligence (+0x47C)->m_TaskMgr.GetTaskSecondary(5)`).
fn secondary_ik(ped: GPtr<u8>) -> GPtr<u8> {
    task_manager_get_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 5)
}

/// `IKChainManager_c::IsLooking(ped)` (0x6181A0; the field load sits in a chunk at 0x401F69): a
/// secondary IK task with a task in slot 0 (AL).
#[hook(addr = 0x6181A0, conv = thiscall, name = "IKChainManager_c::IsLooking", expect = "8B 44 24 04 90")]
pub fn is_looking(_this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let t = secondary_ik(ped);
    if t.is_null() {
        return 0;
    }
    u8::from(!ik_manager_task_get_task_at_slot(t, 0).is_null())
}

/// `IKChainManager_c::GetLookAtEntity(ped)` (0x6181D0): the look-at task's entity, else 0.
#[hook(addr = 0x6181D0, conv = thiscall, name = "IKChainManager_c::GetLookAtEntity", expect = "8B 44 24 04 8B 80 7C 04 00 00")]
pub fn get_look_at_entity(_this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let t = secondary_ik(ped);
    if t.is_null() {
        return GPtr::NULL;
    }
    let la = ik_manager_task_get_task_at_slot(t, 0);
    if la.is_null() {
        return GPtr::NULL;
    }
    look_at_task_get_entity(la)
}

/// `IKChainManager_c::GetLookAtOffset(ped, CVector* out)` (0x618210): the look-at task's offset,
/// else (0, 0, 0). EAX = the offset's z dword on the task path, 0 otherwise.
#[hook(addr = 0x618210, conv = thiscall, name = "IKChainManager_c::GetLookAtOffset", expect = "8B 44 24 04 8B 80 7C 04 00 00")]
pub fn get_look_at_offset(_this: GPtr<u8>, ped: GPtr<u8>, out: GPtr<u32>) -> u32 {
    let t = secondary_ik(ped);
    if !t.is_null() {
        let la = ik_manager_task_get_task_at_slot(t, 0);
        if !la.is_null() {
            let mut tmp = [0u32; 3];
            let r = look_at_task_get_offset(la, GPtr::from_raw(tmp.as_mut_ptr()).cast());
            let r = r.cast::<u32>();
            out.write(r.read());
            out.wrapping_index(1).write(r.wrapping_index(1).read());
            let z = r.wrapping_index(2).read();
            out.wrapping_index(2).write(z);
            return z;
        }
    }
    out.wrapping_index(2).write(0);
    out.wrapping_index(1).write(0);
    out.write(0);
    out.addr() as u32
}

/// `IKChainManager_c::AbortLookAt(ped, blendTime)` (0x618280): `BlendOut(blendTime)` on the look-at
/// task (`GetTaskAtSlot` is called even when there is no secondary task: original bug kept).
#[hook(addr = 0x618280, conv = thiscall, name = "IKChainManager_c::AbortLookAt", expect = "8B 44 24 04 8B 80 7C 04 00 00")]
pub fn abort_look_at(_this: GPtr<u8>, ped: GPtr<u8>, time: i32) {
    let la = ik_manager_task_get_task_at_slot(secondary_ik(ped), 0);
    if !la.is_null() {
        ik_chain_task_blend_out(la, time);
    }
}

/// `IKChainManager_c::IsArmPointing(arm, ped)` (0x6182B0): a task in slot `arm + 1` (AL).
#[hook(addr = 0x6182B0, conv = thiscall, name = "IKChainManager_c::IsArmPointing", expect = "8B 44 24 08 8B 80 7C 04 00 00")]
pub fn is_arm_pointing(_this: GPtr<u8>, arm: i32, ped: GPtr<u8>) -> u8 {
    let t = secondary_ik(ped);
    if t.is_null() {
        return 0;
    }
    u8::from(!ik_manager_task_get_task_at_slot(t, arm.wrapping_add(1)).is_null())
}

/// `IKChainManager_c::AbortPointArm(arm, ped, blendTime)` (0x6182F0): `BlendOut` on the point-arm
/// task in slot `arm + 1` (no null check of the secondary task, as in the original).
#[hook(addr = 0x6182F0, conv = thiscall, name = "IKChainManager_c::AbortPointArm", expect = "8B 44 24 08 8B 80 7C 04 00 00")]
pub fn abort_point_arm(_this: GPtr<u8>, arm: i32, ped: GPtr<u8>, time: i32) {
    let p = ik_manager_task_get_task_at_slot(secondary_ik(ped), arm.wrapping_add(1));
    if !p.is_null() {
        ik_chain_task_blend_out(p, time);
    }
}

/// `IKChainManager_c::IsFacingTarget(ped, slot)` (0x618330): `IsFacingTarget` of the chain of the IK
/// task in `slot` (AL), false without one.
#[hook(addr = 0x618330, conv = thiscall, name = "IKChainManager_c::IsFacingTarget", expect = "8B 44 24 04 8B 80 7C 04 00 00")]
pub fn manager_is_facing_target(_this: GPtr<u8>, ped: GPtr<u8>, slot: i32) -> u8 {
    let t = secondary_ik(ped);
    if t.is_null() {
        return 0;
    }
    let k = ik_manager_task_get_task_at_slot(t, slot);
    if k.is_null() {
        return 0;
    }
    call_chain_is_facing_target(ik_chain_task_get_ik_chain(k))
}

/// `IKChainManager_c::Update(deltaTime)` (0x6186D0): for slot 0..3: advance
/// `CWorld::ms_nCurrentScanCode` (wrap: `ClearScanCodes`, 1) — once PER SLOT, the `jl 0x6186E2` loop
/// back goes to the scan-code compare, so the code advances four times per call — then every active
/// chain of that slot: update the ped's `RpHAnim` once per scan code (+0x2C), `chain->Update(deltaTime)`.
#[hook(addr = 0x6186D0, conv = thiscall, name = "IKChainManager_c::Update", expect = "66 A1 78 CD B7 00")]
pub fn manager_update(this: GPtr<u8>, dt: f32) {
    let sc = g::<u16>(CURRENT_SCAN_CODE);
    let mut ax = sc.read();
    for slot in 0..4u32 {
        if ax < 0xFFFF {
            ax += 1;
        } else {
            world_clear_scan_codes();
            ax = 1;
        }
        sc.write(ax);
        let mut c = rd::<GPtr<u8>>(this, 0xB00);
        while !c.is_null() {
            if slot == u32::from(rd::<u8>(c, 0x55)) {
                let ped = rd::<GPtr<u8>>(c, 8);
                if !ped.is_null() && rd::<u16>(ped, 0x2C) != ax {
                    entity_update_rp_hanim(ped);
                    wr::<u16>(rd::<GPtr<u8>>(c, 8), 0x2C, sc.read());
                }
                call_chain_update(c, dt);
                ax = sc.read();
            }
            c = rd::<GPtr<u8>>(c, 4);
        }
    }
}

/// `IKChainManager_c::AddIKChain(...)` (0x618750): a chain from the free list (null when empty);
/// `Init` with the same 15 arguments; on success onto the active list and returned, else back onto
/// the free list and null.
#[hook(addr = 0x618750, conv = thiscall, name = "IKChainManager_c::AddIKChain", expect = "53 56 57 8B F9")]
#[allow(clippy::too_many_arguments)]
pub fn manager_add_ik_chain(
    this: GPtr<u8>,
    name: u32,
    slot: i32,
    ped: GPtr<u8>,
    effector: i32,
    ex: u32,
    ey: u32,
    ez: u32,
    pivot: i32,
    entity: GPtr<u8>,
    offset_bone: i32,
    ox: u32,
    oy: u32,
    oz: u32,
    speed: u32,
    priority: i32,
) -> GPtr<u8> {
    let free = this.byte_add(0xB0C);
    let c = tlist_remove_head(free);
    if c.is_null() {
        return GPtr::NULL;
    }
    if call_chain_init(c, name, slot, ped, effector, ex, ey, ez, pivot, entity, offset_bone, ox, oy, oz, speed, priority) != 0 {
        tlist_add_item(this.byte_add(0xB00), c);
        return c;
    }
    tlist_add_item(free, c);
    GPtr::NULL
}

/// `IKChainManager_c::CanAccept(ped, range)` (0x618800): the ped has an RW object, is alive and on
/// screen, and (camera mode word 0xB6F858 == 7, or range >= 999 / NaN, or
/// `range² >= |ped - camera|²` / NaN).
#[hook(addr = 0x618800, conv = thiscall, name = "IKChainManager_c::CanAccept", expect = "83 EC 0C 56 8B 74 24 14")]
pub fn can_accept(_this: GPtr<u8>, ped: GPtr<u8>, range: f32) -> u8 {
    let obj = rd::<GPtr<u8>>(ped, 0x18);
    if obj.is_null() || rd::<u32>(obj, 4).wrapping_add(0x10) == 0 {
        return 0;
    }
    if ped_is_alive(ped) == 0 || entity_get_is_on_screen(ped) == 0 {
        return 0;
    }
    if g::<u16>(CAM_MODE_WORD).read() == 7 {
        return 1;
    }
    // test ah, 5 / jp: >= or unordered → true
    if !matches!(x(range).fcom(x(FAR_RANGE)), Cmp::Less) {
        return 1;
    }
    let cm = g::<GPtr<u8>>(CAM_MATRIX).read();
    let cam: GPtr<f32> = if cm.is_null() { g(CAM_PLACEMENT_POS) } else { cm.byte_add(0x30).cast() };
    let pm = rd::<GPtr<u8>>(ped, 0x14);
    let pp: GPtr<f32> = if pm.is_null() { ped.byte_add(4).cast() } else { pm.byte_add(0x30).cast() };
    let mut d = [0f32; 3];
    let r = vector_sub(GPtr::from_raw(d.as_mut_ptr()), pp, cam);
    let msq = X87::from_f64_exact(vector_magnitude_sqr(r));
    let r2 = x(range).fmul_m(range);
    u8::from(!matches!(r2.fcom(msq), Cmp::Less))
}

/// `IKChainManager_c::CanAcceptLookAt(ped)` (0x6188B0): `CanAccept(ped, 20.0)`; a ped with flag
/// 0x400000 (+0x474) aborts a running look-at (250 ms) and is refused; flag bit 4 of +0x528 refuses;
/// so do anims 299..302 on the clump. AL.
#[hook(addr = 0x6188B0, conv = thiscall, name = "IKChainManager_c::CanAcceptLookAt", expect = "56 8B 74 24 08")]
pub fn can_accept_look_at(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if call_can_accept(this, ped, 20.0) == 0 {
        return 0;
    }
    if rd::<u32>(ped, 0x474) & 0x40_0000 != 0 {
        let t = secondary_ik(ped);
        if !t.is_null() && !ik_manager_task_get_task_at_slot(t, 0).is_null() {
            call_abort_look_at(this, ped, 0xFA);
        }
        return 0;
    }
    if (rd::<u32>(ped, 0x528) >> 4) & 1 != 0 {
        return 0;
    }
    for anim in 0x12B..=0x12D {
        if !rp_anim_blend_clump_get_association(rd::<GPtr<u8>>(ped, 0x18), anim).is_null() {
            return 0;
        }
    }
    u8::from(rp_anim_blend_clump_get_association(rd::<GPtr<u8>>(ped, 0x18), 0x12E).is_null())
}

/// `IKChainManager_c::LookAt(purpose, ped, entity, time, offsetBone, CVector* offset, useTorso, speed,
/// blendTime, priority, forceLookAt)` (0x618970, thiscall, `ret 0x2C`). Without `forceLookAt` (byte):
/// returns when `CanAcceptLookAt(ped)` fails or the byte at 0xC1542C is set. The ped's secondary task 5
/// (task manager = intelligence +0x47C, +4) is created when missing (`CTask::operator new(0x1C)`,
/// `CTaskSimpleIKManager` constructor when the allocation is non-null, else null; `SetTaskSecondary(t,
/// 5)`) and fetched again (no null check after that). The offset is `*offset` or zero. With a task in
/// IK slot 0 (no type check): returns when `priority` < its signed priority byte +0x59; `useTorso` set
/// and the task's +0x58 clear zeroes the low byte of `useTorso` (the other bytes of the pushed dword
/// stay); `useTorso` clear and +0x58 set → `AbortLookAt(ped, 250)`; else `UpdateLookAtInfo(purpose, ped,
/// entity, time, offsetBone, offset, useTorso, speed, blendTime, priority)`. Without one: `new(0x5C)`,
/// `CTaskSimpleIKLookAt(purpose, entity, time, offsetBone, offset, useTorso, speed, blendTime,
/// priority)` (null when the allocation fails) and `AddIKChainTask(t, 0)`.
///
/// Stack: the original stores both `operator new` results into its own `forceLookAt` argument slot
/// (S+0x2C) and the cleared `useTorso` byte into its slot (S+0x1C); not reproduced. `speed` is passed
/// through as raw bits.
#[hook(addr = 0x618970, conv = thiscall, name = "IKChainManager_c::LookAt", expect = "6A FF 68 B6 F0 83 00",
       unverified = "mvpi-anim-1", route = "L2", hazards = "stack-residue,heap-writes", recs = "grove-walk",
       note = "allocates CTaskSimpleIKManager / CTaskSimpleIKLookAt tasks (task pool)")]
#[allow(clippy::too_many_arguments)]
pub fn look_at(
    this: GPtr<u8>,
    purpose: u32,
    ped: GPtr<u8>,
    entity: u32,
    time: u32,
    bone: u32,
    offset: GPtr<u32>,
    use_torso: u32,
    speed: u32,
    blend: u32,
    priority: i32,
    force: u32,
) {
    if force as u8 == 0 && (call_can_accept_look_at(this, ped) == 0 || g::<u8>(NO_LOOK_AT).read() != 0) {
        return;
    }
    let mut mgr = secondary_ik(ped);
    if mgr.is_null() {
        let mem = task_new(0x1C);
        let t = if mem.is_null() { GPtr::NULL } else { ik_manager_task_ctor(mem) };
        task_manager_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 5);
        mgr = secondary_ik(ped);
    }
    let (ox, oy, oz) = if offset.is_null() {
        (0, 0, 0)
    } else {
        (offset.read(), offset.wrapping_index(1).read(), offset.wrapping_index(2).read())
    };
    let la = ik_manager_task_get_task_at_slot(mgr, 0);
    if !la.is_null() {
        if priority < i32::from(rd::<i8>(la, 0x59)) {
            return;
        }
        let task_torso = rd::<u8>(la, 0x58);
        let mut torso = use_torso;
        if torso as u8 != 0 {
            if task_torso == 0 {
                torso &= 0xFFFF_FF00;
            }
        } else if task_torso != 0 {
            call_abort_look_at(this, ped, 0xFA);
            return;
        }
        look_at_task_update_look_at_info(la, purpose, ped, entity, time, bone, ox, oy, oz, torso, speed, blend, priority as u32);
        return;
    }
    let mem = task_new(0x5C);
    let t = if mem.is_null() {
        GPtr::NULL
    } else {
        look_at_task_ctor(mem, purpose, entity, time, bone, ox, oy, oz, use_torso, speed, blend, priority as u32)
    };
    ik_manager_task_add_ik_chain_task(mgr, t, 0);
}

orig! {
    /// `CTaskSimpleIKPointArm::UpdatePointArmInfo(purpose, entity, offsetBone, CVector offset (by value), speed,
    /// blendTime)` (0x634370, `ret 0x20`)
    pub fn point_arm_task_update_info(t: GPtr<u8>, purpose: u32, entity: u32, bone: u32, ox: u32, oy: u32, oz: u32, speed: u32, blend: u32) = thiscall 0x634370 => unported;
    /// `CTaskSimpleIKPointArm::CTaskSimpleIKPointArm(purpose, arm, entity, offsetBone, CVector offset (by value),
    /// speed, blendTime)` (0x634150, `ret 0x24`): returns `this`
    pub fn point_arm_task_ctor(t: GPtr<u8>, purpose: u32, arm: u32, entity: u32, bone: u32, ox: u32, oy: u32, oz: u32, speed: u32, blend: u32) -> GPtr<u8> = thiscall 0x634150 => unported;
}

/// `IKChainManager_c::PointArm(purpose, arm, ped, entity, offsetBone, CVector* offset, speed, blendTime, range)`
/// (0x618B60, thiscall, `ret 0x24`; the entry is `nop; jmp 0x405A03`, a chunk loading `fs:[0]` for the SEH frame).
/// `CanAccept(ped, range)` false → nothing. The ped's secondary IK manager task (slot 5) is created when missing
/// (`CTask::operator new(0x1C)`, its constructor or NULL, `SetTaskSecondary(task, 5)`, then fetched again). The offset
/// is `*offset` or zero. The task in slot `arm + 1` (any task: no type check) gets `UpdatePointArmInfo(purpose,
/// entity, offsetBone, offset, speed, blendTime)`; without one a new `CTaskSimpleIKPointArm(purpose, arm, entity,
/// offsetBone, offset, speed, blendTime)` (`operator new(0x5C)`, NULL when that fails) goes to
/// `AddIKChainTask(task, arm + 1)`. The original keeps `arm + 1` in its own `range` argument slot. Speed, blend time
/// and range pass through as raw bits.
#[hook(addr = 0x618b60, conv = thiscall, name = "IKChainManager_c::PointArm", expect = "90 E9 9D CE DE FF",
       unverified = "mvpl-anim-1", route = "L2", hazards = "stack-residue,heap-writes", recs = "grove-walk",
       note = "allocates CTaskSimpleIKManager / CTaskSimpleIKPointArm tasks (task pool); the range argument slot holds the IK slot")]
#[allow(clippy::too_many_arguments)]
pub fn point_arm(
    this: GPtr<u8>,
    purpose: u32,
    arm: i32,
    ped: GPtr<u8>,
    entity: u32,
    bone: u32,
    offset: GPtr<u32>,
    speed: u32,
    blend: u32,
    range: u32,
) {
    if call_can_accept(this, ped, f32::from_bits(range)) == 0 {
        return;
    }
    let mut mgr = secondary_ik(ped);
    if mgr.is_null() {
        let mem = task_new(0x1C);
        let t = if mem.is_null() { GPtr::NULL } else { ik_manager_task_ctor(mem) };
        task_manager_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 5);
        mgr = secondary_ik(ped);
    }
    let (ox, oy, oz) = if offset.is_null() {
        (0, 0, 0)
    } else {
        (offset.read(), offset.wrapping_index(1).read(), offset.wrapping_index(2).read())
    };
    let slot = arm.wrapping_add(1);
    let pa = ik_manager_task_get_task_at_slot(mgr, slot);
    if !pa.is_null() {
        point_arm_task_update_info(pa, purpose, entity, bone, ox, oy, oz, speed, blend);
        return;
    }
    let mem = task_new(0x5C);
    let t = if mem.is_null() {
        GPtr::NULL
    } else {
        point_arm_task_ctor(mem, purpose, arm as u32, entity, bone, ox, oy, oz, speed, blend)
    };
    ik_manager_task_add_ik_chain_task(mgr, t, slot);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_POINT_ARM,
    HOOK_LOOK_AT,
    HOOK_EXIT,
    HOOK_SET_BLEND,
    HOOK_SET_OFFSET_BONE_TAG,
    HOOK_SET_OFFSET_POS,
    HOOK_GET_BONE_NODE_FROM_TAG,
    HOOK_SETUP_BONES,
    HOOK_MOVE_BONES_TO_TARGET,
    HOOK_UPDATE_ENTITY,
    HOOK_UPDATE_OFFSET,
    HOOK_UPDATE_TARGET,
    HOOK_IS_FACING_TARGET,
    HOOK_IS_AT_TARGET,
    HOOK_INIT,
    HOOK_UPDATE,
    HOOK_CLAMP_LIMITS,
    HOOK_GET_LIMITS,
    HOOK_SET_LIMITS,
    HOOK_GET_BONE_SPEED,
    HOOK_SET_BONE_SPEED,
    HOOK_MANAGER_CONSTRUCTOR,
    HOOK_MANAGER_INIT,
    HOOK_MANAGER_EXIT,
    HOOK_MANAGER_RESET,
    HOOK_MANAGER_REMOVE_IK_CHAIN,
    HOOK_IS_LOOKING,
    HOOK_GET_LOOK_AT_ENTITY,
    HOOK_GET_LOOK_AT_OFFSET,
    HOOK_ABORT_LOOK_AT,
    HOOK_IS_ARM_POINTING,
    HOOK_ABORT_POINT_ARM,
    HOOK_MANAGER_IS_FACING_TARGET,
    HOOK_MANAGER_UPDATE,
    HOOK_MANAGER_ADD_IK_CHAIN,
    HOOK_CAN_ACCEPT,
    HOOK_CAN_ACCEPT_LOOK_AT,
];
