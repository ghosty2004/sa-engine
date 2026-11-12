//! WRITER-NATIVE (batch `native-10`): `CTaskComplexRoadRage::CreateFirstSubTask` (0x62E170) and
//! `CreateSubTask` (0x629080), reached when a driver gets out to fight after a car collision on the native
//! walk. Ported from the 1.0 US bytes; gta-reversed (`TaskComplexRoadRage.cpp`) as a reading aid.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPed::GiveDelayedWeapon(eWeaponType, uint32 ammo)` (0x5E89B0)
    fn ped_give_delayed_weapon(p: GPtr<u8>, weapon: u32, ammo: u32) = thiscall 0x5E89B0 => unported;
    /// `CPed::SetCurrentWeapon(eWeaponType)` (0x5E6280)
    fn ped_set_current_weapon(p: GPtr<u8>, weapon: u32) = thiscall 0x5E6280 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn general_random_int_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CInterestingEvents::Add(type, CEntity*)` (0x602590) on `g_InterestingEvents` (0xC0B058)
    fn interesting_events_add(ie: GPtr<u8>, ty: u32, e: GPtr<u8>) = thiscall 0x602590 => unported;
    /// `CTaskComplexRoadRage::CreateSubTask(int32, CPed*)` (0x629080)
    fn road_rage_create_sub_task_c(t: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x629080 => road_rage_create_sub_task;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new_c(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => crate::mvpa_ai1::task_operator_new;
    /// `CTaskComplexKillPedOnFoot(CPed*, int32 time, uint32 flags, int32 delay, int32 chance, int8 level)` (0x620E30)
    #[allow(clippy::too_many_arguments)]
    fn kill_ped_on_foot_ctor(t: GPtr<u8>, target: GPtr<u8>, time: i32, flags: u32, delay: i32, chance: i32, level: u32) -> GPtr<u8> = thiscall 0x620E30 => unported;
    /// `CTaskComplexDestroyCar(CVehicle*, uint32, uint32, uint32)` (0x621C00)
    fn destroy_car_ctor(t: GPtr<u8>, veh: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x621C00 => unported;
    /// `CTaskComplexTurnToFaceEntityOrCoord(CEntity*, float, float)` (0x66B890)
    fn turn_to_face_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x66B890 => unported;
    /// `CTaskComplexLeaveCar(CVehicle*, int32 door, int32 delay, bool sensible, bool forceGetOut)` (0x63B8C0)
    fn leave_car_ctor(t: GPtr<u8>, veh: GPtr<u8>, door: i32, delay: i32, sensible: u32, force: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    /// `CTaskSimpleShakeFist()` (0x690B80)
    fn shake_fist_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x690B80 => unported;
    /// `CTaskComplexEnterCarAsDriver(CVehicle*)` (0x6402F0)
    fn enter_car_as_driver_ctor(t: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x6402F0 => unported;
    /// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorStandard>(CEntity*, int32, int32, float, float, float, bool,
    /// bool)` (0x46AC10)
    #[allow(clippy::too_many_arguments)]
    fn seek_entity_standard_ctor(t: GPtr<u8>, e: GPtr<u8>, time: i32, interval: i32, radius: f32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x46AC10 => unported;
}

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFO_PTRS: usize = 0xA9_B0C8;

/// `CTaskComplexRoadRage::CreateFirstSubTask(CPed*)` (0x62E170, `ret 4`).
///
/// Null without a target (+0xC). Null too unless the ped sits in a vehicle (+0x58C, in-vehicle flag +0x46C bit
/// 8) and the target is not a cop (+0x598 = 6). Gang peds (types 7..16) get a delayed weapon 0x16 with 2000
/// shots and hold it; others, with `rand(0, 100) < 50`, get weapon 5 (`rand <= 33`), 6 (`> 66`) or 2 with one
/// shot. `InterestingEvents.Add(0x12, ped)`, then `CreateSubTask(0x2C0)` (leave the car) when the target is in a
/// vehicle, else `CreateSubTask(0x3E8)`.
#[hook(addr = 0x62E170, conv = thiscall, name = "CTaskComplexRoadRage::CreateFirstSubTask",
       expect = "53 8B D9 8B 43 0C 85 C0",
       unverified = "native-10", route = "L2", hazards = "heap-writes", recs = "grove-walk,car-ai")]
pub fn road_rage_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let target = rd::<GPtr<u8>>(this, 0xC);
    if target.is_null() {
        return target;
    }
    if rd::<u32>(ped, 0x58C) == 0 || rd::<u32>(ped, 0x46C) & 0x100 == 0 || rd::<u32>(target, 0x598) == 6 {
        return GPtr::NULL;
    }
    let ty = rd::<i32>(ped, 0x598);
    if (7..=0x10).contains(&ty) {
        ped_give_delayed_weapon(ped, 0x16, 0x7D0);
        ped_set_current_weapon(ped, 0x16);
    } else if general_random_int_in_range(0, 0x64) < 0x32 {
        let r = general_random_int_in_range(0, 0x64);
        let w = if r <= 0x21 {
            5
        } else if r > 0x42 {
            6
        } else {
            2
        };
        ped_give_delayed_weapon(ped, w, 1);
        ped_set_current_weapon(ped, w);
    }
    interesting_events_add(GPtr::from_addr(0xC0_B058), 0x12, ped);
    let t = rd::<GPtr<u8>>(this, 0xC);
    let sub = if rd::<u32>(t, 0x46C) & 0x100 != 0 { 0x2C0 } else { 0x3E8 };
    road_rage_create_sub_task_c(this, sub, ped)
}

/// `CTaskComplexRoadRage::CreateSubTask(int32 taskType, CPed*)` (0x629080, `ret 8`, SEH frame).
///
/// Null without a target (+0xC). 0x2BD: `CTaskComplexEnterCarAsDriver(ped vehicle)`; 0x12E: `CTaskSimpleShakeFist`;
/// 0x2C0: `CTaskComplexLeaveCar(ped vehicle, 0, 0, true, false)`; 0x38B: with a target vehicle a
/// `CTaskComplexSeekEntityStandard(target vehicle, 20000, 1000, bound radius + 1, 2, 2, true, true)`, else as
/// 0x3E8; 0x398: `CTaskComplexTurnToFaceEntityOrCoord(target, [0x86FC7C], [0x86FC80])`; 0x3E8:
/// `CTaskComplexKillPedOnFoot(target, -1, 0, 0, 0, 1)`; 0x3EB: `CTaskComplexDestroyCar(target vehicle, 0, 0, 0)`
/// with a target vehicle, else as 0x3E8. Anything else null; null when the task pool is full.
#[hook(addr = 0x629080, conv = thiscall, name = "CTaskComplexRoadRage::CreateSubTask",
       expect = "64 A1 00 00 00 00 6A FF",
       unverified = "native-10", route = "L2", hazards = "heap-writes", recs = "grove-walk,car-ai",
       note = "the 0x38B case keeps the bound radius in the ped argument slot")]
pub fn road_rage_create_sub_task(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    let target = rd::<GPtr<u8>>(this, 0xC);
    if target.is_null() {
        return GPtr::NULL;
    }
    let kill = |this: GPtr<u8>| {
        let t = task_new_c(0x38);
        if t.is_null() {
            return GPtr::NULL;
        }
        kill_ped_on_foot_ctor(t, rd::<GPtr<u8>>(this, 0xC), -1, 0, 0, 0, 1)
    };
    match ty {
        0x2BD => {
            let t = task_new_c(0x50);
            if t.is_null() {
                return GPtr::NULL;
            }
            enter_car_as_driver_ctor(t, rd::<GPtr<u8>>(ped, 0x58C))
        }
        0x12E => {
            let t = task_new_c(0x10);
            if t.is_null() {
                return GPtr::NULL;
            }
            shake_fist_ctor(t)
        }
        0x2C0 => {
            let t = task_new_c(0x34);
            if t.is_null() {
                return GPtr::NULL;
            }
            leave_car_ctor(t, rd::<GPtr<u8>>(ped, 0x58C), 0, 0, 1, 0)
        }
        0x38B => {
            let v = rd::<GPtr<u8>>(target, 0x58C);
            if v.is_null() {
                return kill(this);
            }
            let info = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(i32::from(rd::<i16>(v, 0x22))).read();
            let radius = rd::<f32>(rd::<GPtr<u8>>(info, 0x14), 0x24);
            let t = task_new_c(0x4C);
            if t.is_null() {
                return GPtr::NULL;
            }
            let r = x(radius).fadd_m(1.0).to_f32();
            let c30 = g::<u32>(0x85_9E30).read();
            let c34 = g::<u32>(0x85_9E34).read();
            seek_entity_standard_ctor(t, rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x58C), 0x4E20, 0x3E8, r, c30, c34, 1, 1)
        }
        0x398 => {
            let t = task_new_c(0x28);
            if t.is_null() {
                return GPtr::NULL;
            }
            turn_to_face_entity_ctor(t, rd::<GPtr<u8>>(this, 0xC), g::<u32>(0x86_FC7C).read(), g::<u32>(0x86_FC80).read())
        }
        0x3E8 => kill(this),
        0x3EB => {
            if rd::<u32>(target, 0x58C) == 0 {
                return kill(this);
            }
            let t = task_new_c(0x20);
            if t.is_null() {
                return GPtr::NULL;
            }
            destroy_car_ctor(t, rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x58C), 0, 0, 0)
        }
        _ => GPtr::NULL,
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_ROAD_RAGE_CREATE_FIRST_SUB_TASK, HOOK_ROAD_RAGE_CREATE_SUB_TASK];
