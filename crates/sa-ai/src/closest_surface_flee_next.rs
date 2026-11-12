//! WRITER-NATIVE (batch `native-26`): `CPedGeometryAnalyser::ComputeClosestSurfacePoint(const CPed&, CEntity&,
//! CVector&)` (0x5F3B70) and `CTaskComplexSmartFleeEntity::CreateNextSubTask` (0x65C680), reached when peds walk
//! round / flee from the player on the native and web builds. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached(float z, CEntity&, CVector* corners)`
    /// (0x5F1FA0, cdecl)
    fn bbox_corners_uncached(z: f32, e: GPtr<u8>, corners: GPtr<u8>) = cdecl 0x5F1FA0 => unported;
    /// `CPedGeometryAnalyser::ComputeClosestSurfacePoint(const CVector&, const CVector corners[4], CVector&)`
    /// (0x5F2C10, cdecl, AL)
    fn closest_surface_point_rect(pos: GPtr<u8>, corners: GPtr<u8>, out: GPtr<u8>) -> u32 = cdecl 0x5F2C10 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskComplexSmartFleeEntity::CreateSubTask(int32)` (0x65C530)
    fn smart_flee_entity_create_sub_task(t: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x65C530 => unported;
}

/// `CPedGeometryAnalyser::ComputeClosestSurfacePoint(const CPed& ped, CEntity& entity, CVector& out)` (0x5F3B70,
/// cdecl, AL): the entity's bounding-box corners at the ped's height (`ComputeEntityBoundingBoxCornersUncached(ped
/// z, entity, corners)`), then `ComputeClosestSurfacePoint(ped position, corners, out)`; returns its EAX.
#[hook(addr = 0x5F3B70, conv = cdecl, name = "CPedGeometryAnalyser::ComputeClosestSurfacePoint-ped",
       expect = "8B 4C 24 04 8B 41 14",
       unverified = "native-26", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn compute_closest_surface_point_ped(ped: GPtr<u8>, e: GPtr<u8>, out: GPtr<u8>) -> u32 {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let p = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let mut corners = [0f32; 12];
    let cp: GPtr<u8> = GPtr::from_raw(corners.as_mut_ptr()).cast();
    bbox_corners_uncached(x(rd::<f32>(p, 8)).to_f32(), e, cp);
    closest_surface_point_rect(p, cp, out)
}

/// `CTaskComplexSmartFleeEntity::CreateNextSubTask(CPed*)` (0x65C680, `ret 4`). The sub-task's type is read twice
/// (the first result unused, as in the original). With a flee target (+0xC): after 0xCB `CreateSubTask(0x38E)`;
/// otherwise (and without a target after 0xCB too) 0xCB / 0x38E finish with `CreateSubTask(0x516)`; anything else
/// null.
#[hook(addr = 0x65C680, conv = thiscall, name = "CTaskComplexSmartFleeEntity::CreateNextSubTask",
       expect = "56 8B F1 8B 46 0C",
       unverified = "native-26", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn smart_flee_entity_create_next_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    let has_target = rd::<u32>(this, 0xC) != 0;
    task_get_type(rd::<GPtr<u8>>(this, 8));
    let ty = task_get_type(rd::<GPtr<u8>>(this, 8));
    if has_target && ty == 0xCB {
        return smart_flee_entity_create_sub_task(this, 0x38E);
    }
    if ty == 0xCB || ty == 0x38E {
        return smart_flee_entity_create_sub_task(this, 0x516);
    }
    GPtr::NULL
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_COMPUTE_CLOSEST_SURFACE_POINT_PED, HOOK_SMART_FLEE_ENTITY_CREATE_NEXT_SUB_TASK];
