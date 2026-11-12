//! WRITER-NATIVE (batch `native-11`): `CTaskComplexRoadRage::ControlSubTask` (0x62E3F0) and the
//! `CTaskComplexLeaveAnyCar` constructor (0x421150, a `.HOODLUM` stub), reached by the road-rage task on the
//! native walk. Ported from the 1.0 US bytes (PORT-AI-8's generated bodies, ai8g-1 / ai8g-2, as a check).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable slot 0x18)
    fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskComplexRoadRage::CreateSubTask(int32, CPed*)` (0x629080)
    fn road_rage_create_sub_task(t: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x629080 => crate::road_rage_sub_tasks::road_rage_create_sub_task;
    /// `CTaskComplex::CTaskComplex()` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
}

/// The entity's position (`m_matrix` +0x30, else the placement position +4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CTaskComplexRoadRage::ControlSubTask(CPed*)` (0x62E3F0, `ret 4`).
///
/// Without a target (+0xC): `MakeAbortable(ped, 0, null)` on the sub-task, which is returned. Kill-on-foot
/// (0x3E8) with the ped in a vehicle: `CreateSubTask(0x2BD)` (back into the car) once the target is more than
/// sqrt(400) away (`(dz^2 + dy^2) + dx^2 > 400`, ordered). Destroy-car (0x3EB): `CreateSubTask(0x3E8)` once the
/// target is out of its vehicle; else with the ped in a vehicle `CreateSubTask(0x2BD)` beyond the same
/// distance (`(dx^2 + dz^2) + dy^2`). Otherwise the sub-task.
#[hook(addr = 0x62E3F0, conv = thiscall, name = "CTaskComplexRoadRage::ControlSubTask",
       expect = "53 56 8B F1 8B 46 0C",
       unverified = "native-11", route = "L2", hazards = "none", recs = "grove-walk,car-ai")]
pub fn road_rage_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(this, 0xC) == 0 {
        task_make_abortable(sub, ped, 0, GPtr::NULL);
        return sub;
    }
    let far = x(f32::from_bits(0x43C8_0000));
    match task_get_type(sub) {
        0x3E8 => {
            if rd::<u32>(ped, 0x58C) == 0 {
                return sub;
            }
            let p = pos(ped);
            let t = pos(rd::<GPtr<u8>>(this, 0xC));
            let dx = x(rd::<f32>(t, 0)).fsub_m(rd::<f32>(p, 0));
            let dy = x(rd::<f32>(t, 4)).fsub_m(rd::<f32>(p, 4));
            let dz = x(rd::<f32>(t, 8)).fsub_m(rd::<f32>(p, 8));
            let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
            if d.fcom(far).test_any(0x41) {
                sub
            } else {
                road_rage_create_sub_task(this, 0x2BD, ped)
            }
        }
        0x3EB => {
            let target = rd::<GPtr<u8>>(this, 0xC);
            if rd::<u32>(target, 0x46C) & 0x100 == 0 {
                return road_rage_create_sub_task(this, 0x3E8, ped);
            }
            if rd::<u32>(ped, 0x58C) == 0 {
                return sub;
            }
            let p = pos(ped);
            let t = pos(target);
            let dx = x(rd::<f32>(t, 0)).fsub_m(rd::<f32>(p, 0));
            let dy = x(rd::<f32>(t, 4)).fsub_m(rd::<f32>(p, 4));
            let dz = x(rd::<f32>(t, 8)).fsub_m(rd::<f32>(p, 8));
            let d = dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy));
            if d.fcom(far).test_any(0x41) {
                sub
            } else {
                road_rage_create_sub_task(this, 0x2BD, ped)
            }
        }
        _ => sub,
    }
}

/// `CTaskComplexLeaveAnyCar::CTaskComplexLeaveAnyCar(int32 delay, bool sensible, bool forceGetOut)` (0x421150,
/// `jmp` to the `.HOODLUM` body 0x15667C0, `ret 0xC`): `CTaskComplex` base, +0xC = delay, vtable 0x858EB0, +0x10 /
/// +0x11 = the two bytes. Returns `this`.
#[hook(addr = 0x421150, conv = thiscall, name = "CTaskComplexLeaveAnyCar::Constructor",
       expect = "E9 6B 56 14 01",
       unverified = "native-11", route = "L2", hazards = "heap-writes", recs = "grove-walk,car-ai")]
pub fn leave_any_car_ctor(this: GPtr<u8>, delay: u32, sensible: u32, force: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0xC, delay);
    wr::<u32>(this, 0, 0x85_8EB0);
    wr::<u8>(this, 0x10, sensible as u8);
    wr::<u8>(this, 0x11, force as u8);
    this
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_ROAD_RAGE_CONTROL_SUB_TASK, HOOK_LEAVE_ANY_CAR_CTOR];
