//! `CTaskComplexLeaveCarAndFlee`'s constructor and `CreateNextSubTask` (batch `carjack-1`): a passenger of a jacked
//! car gets out and runs. Ported from the 1.0 US bytes (gta-reversed as a reading aid; the bytes win). Unverified
//! (docs/verify/HOWTO.md). Fields: +0xC vehicle, +0x10 door, +0x14 flee point, +0x20 flee delay, +0x24 wait for the
//! car to slow down.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskComplex::CTaskComplex()` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskComplexLeaveCarAndFlee::CreateSubTask(int32)` (0x63C0A0, `ret 4`)
    fn leave_car_and_flee_create_sub_task(t: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x63C0A0 => unported;
    /// `CTaskComplexWander::GetWanderTaskByPedType(CPed*)` (0x673D00, cdecl)
    fn wander_get_wander_task_by_ped_type(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x673D00 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0, `ret 0xC`)
    fn task_manager_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: u32, b: u32) = thiscall 0x681AF0 => unported;
}

/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// -2.0 (the flee point's distance behind a boat in bounding radii)
const BEHIND_SCALE: usize = 0x858B18;

/// `CTaskComplexLeaveCarAndFlee::CTaskComplexLeaveCarAndFlee(CVehicle*, const CVector& fleePoint, int32 door, int32
/// delay, bool wait)` (0x63BF90, `ret 0x14`; entry `push -1; jmp` into the protection fragment 0x403A25, SEH frame):
/// `CTaskComplex()`, +0x10 door, vtable 0x86E8AC, +0xC vehicle, +0x14 the point, +0x20 delay, +0x24 the flag's low
/// byte; the vehicle, when set, a registered reference.
#[hook(addr = 0x63BF90, conv = thiscall, name = "CTaskComplexLeaveCarAndFlee::Constructor", expect = "6A FF E9 8E 7A DC FF",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack",
       note = "entry `push -1; jmp` into the protection fragment 0x403A25")]
pub fn leave_car_and_flee_ctor(this: GPtr<u8>, veh: GPtr<u8>, point: GPtr<u8>, door: u32, delay: u32, wait: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0x10, door);
    wr::<u32>(this, 0, 0x86E8AC);
    wr::<GPtr<u8>>(this, 0xC, veh);
    for k in 0..3isize {
        wr::<u32>(this, 0x14 + 4 * k, rd::<u32>(point, 4 * k));
    }
    wr::<u32>(this, 0x20, delay);
    wr::<u8>(this, 0x24, wait as u8);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(0xC));
    }
    this
}

/// `CTaskComplexLeaveCarAndFlee::CreateNextSubTask(CPed*)` (0x642390, `ret 4`). By the sub-task's type: leave car
/// (0x2C0): when the *ped's* vehicle (+0x58C) is a boat (+0x590 == 5) the flee point (+0x14) becomes its position plus
/// its forward vector times `bound radius * -2.0` (x/y products stored as floats, z kept in x87 precision), then
/// sub-task 0x384; otherwise 0x38E. 0x384 → 0x38E. 0x38E → `CreateSubTask(0x516)`, and the ped's primary default
/// task (slot 4) becomes `GetWanderTaskByPedType(ped)` (the intelligence read first). Any other type: NULL.
#[hook(addr = 0x642390, conv = thiscall, name = "CTaskComplexLeaveCarAndFlee::CreateNextSubTask", expect = "83 EC 0C 56 8B F1",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn leave_car_and_flee_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    match v_get_task_type(rd::<GPtr<u8>>(this, 8)) {
        0x2C0 => {
            let v = rd::<GPtr<u8>>(ped, 0x58C);
            if v.is_null() || rd::<u32>(v, 0x590) != 5 {
                return leave_car_and_flee_create_sub_task(this, 0x38E);
            }
            let fwd = rd::<GPtr<u8>>(v, 0x14).byte_add(0x10);
            let (fx, fy, fz) = (rd::<f32>(fwd, 0), rd::<f32>(fwd, 4), rd::<f32>(fwd, 8));
            let mi = g::<u32>(MODEL_INFO_PTRS + rd::<i16>(v, 0x22) as isize as usize * 4).read();
            let col = rd::<GPtr<u8>>(GPtr::from_addr(mi as usize), 0x14);
            let k = x(rd::<f32>(col, 0x24)).fmul_m(g::<f32>(BEHIND_SCALE).read());
            let dx = f32_of(x(fx).fmul(k));
            let dy = f32_of(x(fy).fmul(k));
            let dz = k.fmul_m(fz);
            let m = rd::<GPtr<u8>>(v, 0x14);
            let pos = if m.is_null() { v.byte_add(4) } else { m.byte_add(0x30) };
            wr::<f32>(this, 0x14, f32_of(x(dx).fadd_m(rd::<f32>(pos, 0))));
            wr::<f32>(this, 0x18, f32_of(x(dy).fadd_m(rd::<f32>(pos, 4))));
            wr::<f32>(this, 0x1C, f32_of(dz.fadd_m(rd::<f32>(pos, 8))));
            leave_car_and_flee_create_sub_task(this, 0x384)
        }
        0x384 => leave_car_and_flee_create_sub_task(this, 0x38E),
        0x38E => {
            let r = leave_car_and_flee_create_sub_task(this, 0x516);
            let intel = rd::<GPtr<u8>>(ped, 0x47C);
            let w = wander_get_wander_task_by_ped_type(ped);
            task_manager_set_task(intel.byte_add(4), w, 4, 0);
            r
        }
        _ => GPtr::NULL,
    }
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[HOOK_LEAVE_CAR_AND_FLEE_CTOR, HOOK_LEAVE_CAR_AND_FLEE_CREATE_NEXT_SUB_TASK];
