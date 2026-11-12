//! WRITER-NATIVE (batch `native-6`): event `AffectsPed` vslots reached on the native walk
//! (`CEventPotentialWalkIntoObject`, `CEventVehicleDamage`). Ported from the 1.0 US bytes; gta-reversed as a
//! reading aid.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::IsAlive()` (0x5E0170)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn task_manager_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CVehicle::IsPassenger(CPed*)` (0x6D1BD0)
    fn vehicle_is_passenger(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6D1BD0 => unported;
}

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFO_PTRS: usize = 0xA9_B0C8;

/// `CEventPotentialWalkIntoObject::AffectsPed(CPed*)` (0x4B4950, `ret 4`, bool in AL).
///
/// False for the player, a dead ped, no object (+0x14), move state +0x18 = 1, a ped standing on something
/// (+0xFC) or on this object, an object flagged +0x40 bit 5, or a collision box thinner than 0.01 on any axis
/// (`max - min < 0.01`, ordered). False also when the ped's active task is `0x3AB` with this object at +0xC or
/// `0x1FE` with it at +0x1C; else true.
#[hook(addr = 0x4B4950, conv = thiscall, name = "CEventPotentialWalkIntoObject::AffectsPed",
       expect = "56 8B 74 24 08 57 8B F9",
       unverified = "native-6", route = "L2", hazards = "ax-al-return", recs = "grove-walk",
       note = "the false paths leave EAX's upper bytes from the last call or load")]
pub fn walk_into_object_affects_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_player(ped) != 0 || ped_is_alive(ped) == 0 {
        return 0;
    }
    let obj = rd::<GPtr<u8>>(this, 0x14);
    if obj.is_null() || rd::<u32>(this, 0x18) == 1 {
        return 0;
    }
    if !rd::<GPtr<u8>>(ped, 0xFC).is_null() || rd::<GPtr<u8>>(obj, 0xFC) == ped || rd::<u8>(obj, 0x40) & 0x20 != 0 {
        return 0;
    }
    let mi = i32::from(rd::<i16>(obj, 0x22));
    let info = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(mi).read();
    let col = rd::<GPtr<u8>>(info, 0x14);
    let thin = f32::from_bits(0x3C23_D70A);
    for i in 0..3 {
        let d = x(rd::<f32>(col, 0xC + 4 * i)).fsub_m(rd::<f32>(col, 4 * i));
        if !d.fcom(x(thin)).test_parity(0x05) {
            return 0;
        }
    }
    let task = task_manager_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    if !task.is_null() {
        if task_get_type(task) == 0x3AB && rd::<GPtr<u8>>(task, 0xC) == rd::<GPtr<u8>>(this, 0x14) {
            return 0;
        }
        if task_get_type(task) == 0x1FE && rd::<GPtr<u8>>(task, 0x1C) == rd::<GPtr<u8>>(this, 0x14) {
            return 0;
        }
    }
    1
}

/// `CEventVehicleDamage::AffectsPed(CPed*)` (0x4B1A00, `ret 4`, bool in AL).
///
/// False for a dead ped or no attacker (+0x18), and for a ped in a vehicle (+0x46C bit 8) that is a passenger
/// of a vehicle with a driver. Otherwise true when the damaged vehicle (+0x14) is the ped's vehicle (+0x58C).
#[hook(addr = 0x4B1A00, conv = thiscall, name = "CEventVehicleDamage::AffectsPed",
       expect = "56 8B 74 24 08 57 8B F9",
       unverified = "native-6", route = "L2", hazards = "ax-al-return", recs = "grove-walk,driving",
       note = "the first false path leaves EAX's upper bytes from the last call or load; the others set all of EAX")]
pub fn vehicle_damage_affects_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_alive(ped) == 0 || rd::<u32>(this, 0x18) == 0 {
        return 0;
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() && vehicle_is_passenger(v, ped) != 0 && !rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x58C), 0x460).is_null() {
            return 0;
        }
    }
    let v = rd::<GPtr<u8>>(this, 0x14);
    u8::from(!v.is_null() && rd::<GPtr<u8>>(ped, 0x58C) == v)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_WALK_INTO_OBJECT_AFFECTS_PED, HOOK_VEHICLE_DAMAGE_AFFECTS_PED];
