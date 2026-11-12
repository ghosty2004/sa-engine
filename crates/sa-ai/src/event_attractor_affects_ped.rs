//! `CEventAttractor::AffectsPed` (0x4AF4B0, batch `std11-ai-1`): whether an attractor event reaches an ambient ped.
//! Ported from the 1.0 US bytes (gta-reversed as a reading aid; where it differs, the bytes are followed). Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPed::IsAlive()` (0x5E0170)
    fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CEvent::GetEventType()` (vtable slot 4)
    fn event_get_type(e: GPtr<u8>) -> u32 = thiscall vslot 0x4 => unported;
    /// `CEventAttractor::IsEffectActive(CEntity*, const C2dEffectPedAttractor*)` (0x4AF460, cdecl)
    fn is_effect_active(entity: GPtr<u8>, fx: GPtr<u8>) -> u8 = cdecl 0x4AF460 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0)
    fn intel_find_task_by_type(i: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `FindPlayerWanted(int32)` (0x56E230)
    fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CPopulation::PedMICanBeCreatedAtThisAttractor(int32, const char*)` (0x6110E0)
    fn ped_mi_can_be_created_at_this_attractor(mi: i32, name: GPtr<u8>) -> u8 = cdecl 0x6110E0 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CWeather::IsRainy()` (0x4ABF50)
    fn weather_is_rainy() -> u8 = cdecl 0x4ABF50 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CIKChainManager_c::IsLooking(CPed*)` (0x6181A0)
    fn ik_is_looking(this: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    fn placeable_get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, result through the first argument)
    fn matrix_mul_vector(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CIKChainManager_c::LookAt(const char*, CPed*, CEntity*, int32, ePedBones, CVector*, bool, float, int32, int32, bool)` (0x618970)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: u32, time: i32, bone: i32, pos: GPtr<u8>, b: u32, speed: u32, blend: i32, prio: i32, b2: u32) = thiscall 0x618970 => unported;
}

/// `g_ikChainMan`
const IK_CHAIN_MAN: usize = 0xC15448;

/// `CEventAttractor::AffectsPed(CPed*)` (0x4AF4B0, thiscall, `ret 4`). Not alive → 0. For an `EVENT_ATTRACTOR` (0x1B):
/// no entity (+0x18) or an inactive effect → 0. A `TASK_COMPLEX_BE_IN_COUPLE` (0x4BF) → 0. A cop (ped type 6) with an
/// `EVENT_ATTRACTOR`: wanted level (+0x2C) set, the effect not a script trigger (type 5) or the model not allowed at the
/// attractor (+0x38 name) → 0. An active task of type 0xE9 (`USE_EFFECT`) without `avoid_looking` (+0x1C) → 0; a
/// shelter (type 4) when it is not rainy → 0. `avoid_looking` → 1. A random 0..100 not below the effect's +0x35 → 0;
/// a second not below +0x36 → 1. Else, when the ped is not already looking, a look-at of 2000..4000 ms at the effect's
/// world position ("CEventAttractor", speed 0.25, blend 500, priority 3); returns 0 (also after the look-at).
///
/// Returns AL; EAX's upper bytes are callee residue in the original (not reproduced).
#[hook(addr = 0x4AF4B0, conv = thiscall, name = "CEventAttractor::AffectsPed", expect = "83 EC 0C 53 8B 5C 24 14",
       unverified = "std11-ai-1", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn event_attractor_affects_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if ped_is_alive(ped) == 0 {
        return 0;
    }
    if event_get_type(this) == 0x1B && rd::<u32>(this, 0x18) == 0 {
        return 0;
    }
    if event_get_type(this) == 0x1B && is_effect_active(rd::<GPtr<u8>>(this, 0x18), rd::<GPtr<u8>>(this, 0x14)) == 0 {
        return 0;
    }
    if !intel_find_task_by_type(rd::<GPtr<u8>>(ped, 0x47C), 0x4BF).is_null() {
        return 0;
    }
    if rd::<u32>(ped, 0x598) == 6 && event_get_type(this) == 0x1B {
        if rd::<u32>(find_player_wanted(-1), 0x2C) != 0 {
            return 0;
        }
        let fx = rd::<GPtr<u8>>(this, 0x14);
        if rd::<u8>(fx, 0x34) != 5 {
            return 0;
        }
        if ped_mi_can_be_created_at_this_attractor(i32::from(rd::<i16>(ped, 0x22)), fx.byte_add(0x38)) == 0 {
            return 0;
        }
    }
    let active = tm_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    if !active.is_null() && rd::<u8>(this, 0x1C) == 0 && task_get_type(active) == 0xE9 {
        return 0;
    }
    if rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 0x34) == 4 && weather_is_rainy() == 0 {
        return 0;
    }
    if rd::<u8>(this, 0x1C) != 0 {
        return 1;
    }
    if random_in_range(0, 100) >= i32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 0x35)) {
        return 0;
    }
    if random_in_range(0, 100) >= i32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 0x36)) {
        return 1;
    }
    let ik = GPtr::<u8>::from_addr(IK_CHAIN_MAN);
    if ik_is_looking(ik, ped) != 0 {
        return 0;
    }
    let time = random_in_range(2000, 4000);
    let fx = rd::<GPtr<u8>>(this, 0x14);
    let m = placeable_get_matrix(rd::<GPtr<u8>>(this, 0x18));
    let mut point = [0f32; 3];
    let pp: GPtr<u8> = GPtr::from_raw(point.as_mut_ptr()).cast();
    matrix_mul_vector(pp, m, fx);
    ik_look_at(ik, 0x85B1FC, ped, 0, time, -1, pp, 0, 0x3E80_0000, 500, 3, 0);
    0
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_ATTRACTOR_AFFECTS_PED];
