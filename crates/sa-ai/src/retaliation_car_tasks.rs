//! The two car tasks a jacked ped retaliates with (batch `carjack-1`): `CTaskComplexDestroyCar::ControlSubTask` (attack
//! the car CJ drove off in) and `CTaskComplexDragPedFromCar::CreateFirstSubTask` (pull CJ back out). Ported from the
//! 1.0 US bytes (gta-reversed as a reading aid; the bytes win). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::*;

orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CTaskComplexDestroyCar::CreateSubTask(int32, CPed*)` (0x6287A0, `ret 8`)
    fn destroy_car_create_sub_task(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x6287A0 => unported;
    /// `CWeapon::IsTypeMelee()` (0x73B1C0)
    fn weapon_is_type_melee(w: GPtr<u8>) -> u8 = thiscall 0x73B1C0 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::SetCurrentWeapon(int32 slot)` (0x5E61F0, `ret 4`)
    fn ped_set_current_weapon_slot(p: GPtr<u8>, slot: u32) = thiscall 0x5E61F0 => unported;
    /// `CVehicle::IsPassenger(CPed*)` (0x6D1BD0, `ret 4`)
    fn vehicle_is_passenger(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6D1BD0 => unported;
    /// `CVehicle::IsDriver(CPed*)` (0x6D1C40, `ret 4`)
    fn vehicle_is_driver(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6D1C40 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CCarEnterExit::ComputeTargetDoorToExit(CVehicle*, CPed*)` (0x64F110, cdecl)
    fn compute_target_door_to_exit(v: GPtr<u8>, p: GPtr<u8>) -> u32 = cdecl 0x64F110 => unported;
    /// `CTaskComplexEnterCar::CreateFirstSubTask(CPed*)` (0x643A60, `ret 4`)
    fn enter_car_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x643A60 => unported;
    /// `CTaskComplexEnterCar::CreateSubTask(int32, CPed*)` (0x63E040, `ret 8`)
    fn enter_car_create_sub_task(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x63E040 => unported;
}

/// 0.0
const ZERO: usize = 0x858B50;

/// `CTaskComplexDestroyCar::ControlSubTask(CPed*)` (0x6288C0, `ret 4`). No vehicle (+0x10) or its health (+0x4C0) ≤ 0
/// (`fcomp`, NaN continues): NULL. Byte +0xC set: the task's `CreateFirstSubTask`. Melee sub-task (0x3EC) with a
/// non-melee current weapon → 0x3ED; armed (0x3ED) with an empty current slot (ammo +0x5AC) for a non-player: the
/// first slot with ammo becomes current, none → slot 0 and 0x3EC. A switch replaces an urgently aborted sub-task
/// through `CreateSubTask(type, ped)`; otherwise the sub-task stays.
#[hook(addr = 0x6288C0, conv = thiscall, name = "CTaskComplexDestroyCar::ControlSubTask", expect = "53 8B D9 8B 43 10",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn destroy_car_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let v = rd::<GPtr<u8>>(this, 0x10);
    if v.is_null() || matches!(x(rd::<f32>(v, 0x4C0)).fcom(x(g::<f32>(ZERO).read())), Cmp::Less | Cmp::Equal) {
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0xC) != 0 {
        return v_create_first_sub_task(this, ped);
    }
    let slot = isize::from(rd::<i8>(ped, 0x718));
    let next = match v_get_task_type(rd::<GPtr<u8>>(this, 8)) {
        0x3EC => {
            if weapon_is_type_melee(ped.byte_add(0x5A0 + slot * 0x1C)) != 0 {
                return rd::<GPtr<u8>>(this, 8);
            }
            0x3ED
        }
        0x3ED => {
            if rd::<u32>(ped, 0x5AC + slot * 0x1C) != 0 || ped_is_player(ped) != 0 {
                return rd::<GPtr<u8>>(this, 8);
            }
            let mut i = 0u32;
            while i < 13 {
                if rd::<i32>(ped, 0x5AC + i as isize * 0x1C) > 0 {
                    ped_set_current_weapon_slot(ped, i);
                    return rd::<GPtr<u8>>(this, 8);
                }
                i += 1;
            }
            ped_set_current_weapon_slot(ped, 0);
            0x3EC
        }
        _ => return rd::<GPtr<u8>>(this, 8),
    };
    if v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, GPtr::NULL) != 0 {
        return destroy_car_create_sub_task(this, next, ped);
    }
    rd::<GPtr<u8>>(this, 8)
}

/// `CTaskComplexDragPedFromCar::CreateFirstSubTask(CPed*)` (0x643D00, `ret 4`). The ped to drag (+0x50) must sit in its
/// vehicle (+0x58C, +0x46C bit 8) as a passenger or the driver, else `CTaskComplexEnterCar::CreateSubTask(0xC8)`. The
/// car (+0xC) becomes that vehicle (old reference cleaned, new one registered); flag byte +0x10: bit 0 = the dragged
/// ped drives it, bit 1 cleared, bit 2 set. The target door (+0x1C) is `ComputeTargetDoorToExit(car, ped)` unless the
/// handling flags (+0x384 → +0xCC) have bit 0x200 or the dragged ped's vehicle is class 9 / type 2 (then 0); then
/// `CTaskComplexEnterCar::CreateFirstSubTask(ped)`.
#[hook(addr = 0x643D00, conv = thiscall, name = "CTaskComplexDragPedFromCar::CreateFirstSubTask", expect = "56 8B F1 8B 46 50",
       unverified = "carjack-1", route = "L2", hazards = "heap-writes", recs = "carjack")]
pub fn drag_ped_from_car_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let p = rd::<GPtr<u8>>(this, 0x50);
    let ok = !p.is_null()
        && rd::<u32>(p, 0x58C) != 0
        && rd::<u32>(p, 0x46C) & 0x100 != 0
        && (vehicle_is_passenger(rd::<GPtr<u8>>(p, 0x58C), p) != 0 || {
            let p = rd::<GPtr<u8>>(this, 0x50);
            vehicle_is_driver(rd::<GPtr<u8>>(p, 0x58C), p) != 0
        });
    if !ok {
        return enter_car_create_sub_task(this, 0xC8, ped);
    }
    let slot = this.byte_add(0xC);
    let old = rd::<GPtr<u8>>(this, 0xC);
    if !old.is_null() {
        entity_clean_up_old_reference(old, slot);
    }
    let v = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x50), 0x58C);
    wr::<GPtr<u8>>(this, 0xC, v);
    entity_register_reference(v, slot);
    let car = rd::<GPtr<u8>>(this, 0xC);
    let p = rd::<GPtr<u8>>(this, 0x50);
    let mut fl = rd::<u8>(this, 0x10);
    fl = if rd::<GPtr<u8>>(car, 0x460) == p { fl | 1 } else { fl & 0xFE };
    wr::<u8>(this, 0x10, fl);
    wr::<u8>(this, 0x10, (fl & 0xFD) | 4);
    if car.is_null() {
        return enter_car_create_first_sub_task(this, ped);
    }
    let pv = rd::<GPtr<u8>>(p, 0x58C);
    if rd::<u32>(rd::<GPtr<u8>>(car, 0x384), 0xCC) & 0x200 != 0 || rd::<u32>(pv, 0x590) == 9 || rd::<u32>(pv, 0x594) == 2 {
        wr::<u32>(this, 0x1C, 0);
    } else {
        let door = compute_target_door_to_exit(car, p);
        wr::<u32>(this, 0x1C, door);
    }
    enter_car_create_first_sub_task(this, ped)
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[HOOK_DESTROY_CAR_CONTROL_SUB_TASK, HOOK_DRAG_PED_FROM_CAR_CREATE_FIRST_SUB_TASK];
