//! Ambient peds at attractors (batch `std11-ai-1`): `CPedIntelligence::ProcessEventHandler` (0x601BB0), the
//! `CEventAttractor` destructor (0x4AF400), `CTaskComplexUseEffect::CreateFirstSubTask` (0x632460) and the
//! `CTaskComplexUseEffect` destructor (0x632270), `CTaskComplexUseEffect::ControlSubTask` (0x632570) and
//! `CTaskComplexUseEffect::CreateNextSubTask` (0x632420). Ported from the
//! 1.0 US bytes; unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEventHandler::HandleEvents()` (0x4C3F10)
    fn eh_handle_events(eh: GPtr<u8>) = thiscall 0x4C3F10 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CScripted2dEffects::GetIndex(const C2dEffectPedAttractor*)` (0x6F9F60)
    fn scripted_2d_effects_get_index(effect: GPtr<u8>) -> i32 = cdecl 0x6F9F60 => unported;
    /// `GetPedAttractorManager()` (0x5EE190)
    fn get_ped_attractor_manager() -> GPtr<u8> = cdecl 0x5EE190 => unported;
    /// `CPedAttractorManager::RegisterPedWithAttractor(CPed*, C2dEffectPedAttractor*, CEntity*, int32)` (0x5EFCA0)
    fn register_ped_with_attractor(m: GPtr<u8>, ped: GPtr<u8>, fx: GPtr<u8>, entity: GPtr<u8>, move_state: u32) -> GPtr<u8> = thiscall 0x5EFCA0 => unported;
    /// `CPedAttractor::GetTaskForPed(CPed*)` (0x5EC500)
    fn attractor_get_task_for_ped(a: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x5EC500 => unported;
    /// `CTaskSimple::CTaskSimple()` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CPedAttractorManager::DeRegisterPed(CPed*, CPedAttractor*)` (0x5EC850)
    /// `FindPlayerWanted(int32)` (0x56E230)
    fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CWanted::CanCopJoinPursuit(CCopPed*)` (0x562FB0)
    fn wanted_can_cop_join_pursuit(w: GPtr<u8>, cop: GPtr<u8>) -> u8 = thiscall 0x562FB0 => unported;
    /// `CTask::MakeAbortable(CPed*, eAbortPriority, const CEvent*)` (vtable slot 0x18)
    fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable slot 0x24)
    fn task_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CPedAttractorManager::IsPedRegisteredWithEffect`-style lookup (0x5EBF50): the attractor `ped` is registered
    /// with for (fx, entity)
    fn find_attractor_for_ped(m: GPtr<u8>, ped: GPtr<u8>, fx: GPtr<u8>, entity: GPtr<u8>) -> GPtr<u8> = thiscall 0x5EBF50 => unported;
    fn deregister_ped(m: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>) -> u8 = thiscall 0x5EC850 => unported;
    /// `CTaskComplex` base destructor (0x61A3D0)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => crate::mvpb_ai::task_complex_dtor_61a3d0;
}

/// `CPedIntelligence::ProcessEventHandler()` (0x601BB0, thiscall): `m_eventHandler.HandleEvents()` (this + 0x34, tail
/// jump).
#[hook(addr = 0x601BB0, conv = thiscall, name = "CPedIntelligence::ProcessEventHandler", expect = "83 C1 34 E9 58 23 EC FF",
       unverified = "std11-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn intel_process_event_handler(this: GPtr<u8>) {
    eh_handle_events(this.byte_add(0x34));
}

/// `CEventAttractor::~CEventAttractor` (0x4AF400, thiscall): vtable 0x85B1B8; with the entity (+0x18) set,
/// `entity->CleanUpOldReference(&this->entity)`; then the `CEvent` vtable 0x85AB38.
#[hook(addr = 0x4AF400, conv = thiscall, name = "FUN_004af400", expect = "6A FF 68 68 9E 83 00",
       unverified = "std11-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_attractor_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85B1B8);
    let e = rd::<GPtr<u8>>(this, 0x18);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(0x18));
    }
    wr::<u32>(this, 0, 0x85AB38);
}

/// `CTaskComplexUseEffect::CreateFirstSubTask(CPed* ped)` (0x632460, thiscall, `ret 4`): the ped reference (+0x18)
/// changed to `ped`; the ped attractor (+0x14) = 0 when aborting (byte +0x20) or without an entity (+0x10) when the
/// effect (+0xC) is not a scripted one (`GetIndex < 0`), else `GetPedAttractorManager()->RegisterPedWithAttractor(ped,
/// fx, entity, move state +0x1C)`. With an attractor: its `GetTaskForPed(ped)`; else a new 8-byte `CTaskSimpleNone`
/// (vtable 0x859CFC), or 0 when the allocation fails.
#[hook(addr = 0x632460, conv = thiscall, name = "CTaskComplexUseEffect::CreateFirstSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "std11-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_complex_use_effect_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let slot = this.byte_add(0x18);
    let old = rd::<GPtr<u8>>(this, 0x18);
    if !old.is_null() {
        entity_clean_up_old_reference(old, slot);
    }
    wr::<GPtr<u8>>(this, 0x18, ped);
    if !ped.is_null() {
        entity_register_reference(ped, slot);
    }
    let a = if rd::<u8>(this, 0x20) != 0
        || (rd::<u32>(this, 0x10) == 0 && scripted_2d_effects_get_index(rd::<GPtr<u8>>(this, 0xC)) < 0)
    {
        GPtr::NULL
    } else {
        let (fx, entity, ms) = (rd::<GPtr<u8>>(this, 0xC), rd::<GPtr<u8>>(this, 0x10), rd::<u32>(this, 0x1C));
        register_ped_with_attractor(get_ped_attractor_manager(), ped, fx, entity, ms)
    };
    wr::<GPtr<u8>>(this, 0x14, a);
    if !a.is_null() {
        return attractor_get_task_for_ped(a, ped);
    }
    let t = task_new(8);
    if t.is_null() {
        return t;
    }
    task_simple_ctor(t);
    wr::<u32>(t, 0, 0x859CFC);
    t
}

/// `CTaskComplexUseEffect::~CTaskComplexUseEffect` (0x632270, thiscall): vtable 0x86E0D8; with the ped (+0x18) and
/// the ped attractor (+0x14) set, `GetPedAttractorManager()->DeRegisterPed(ped, attractor)`; the entity reference
/// (+0x10) cleaned up; then the `CTaskComplex` base destructor (0x61A3D0).
#[hook(addr = 0x632270, conv = thiscall, name = "CTaskComplexUseEffect::Destructor", expect = "6A FF 68 58 06 84 00",
       unverified = "std11-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_complex_use_effect_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86E0D8);
    let ped = rd::<GPtr<u8>>(this, 0x18);
    if !ped.is_null() {
        let a = rd::<GPtr<u8>>(this, 0x14);
        if !a.is_null() {
            deregister_ped(get_ped_attractor_manager(), ped, a);
        }
    }
    let e = rd::<GPtr<u8>>(this, 0x10);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(0x10));
    }
    task_complex_dtor(this);
}

/// `CTaskComplexUseEffect::ControlSubTask(CPed* ped)` (0x632570, thiscall, `ret 4`): a cop (ped type 6) while the
/// player is wanted (+0x2C) and `CanCopJoinPursuit(ped)`, when the sub-task (+8) aborts (`MakeAbortable(ped, 1,
/// null)`): the ped deregistered from its attractor (+0x14), 0. Otherwise the attractor's `GetTaskForPed(ped)`, or
/// without one the virtual `CreateFirstSubTask(ped)`.
#[hook(addr = 0x632570, conv = thiscall, name = "CTaskComplexUseEffect::ControlSubTask", expect = "56 57 8B 7C 24 0C 83 BF 98 05 00 00 06",
       unverified = "std11-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_complex_use_effect_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(ped, 0x598) == 6
        && rd::<u32>(find_player_wanted(-1), 0x2C) != 0
        && wanted_can_cop_join_pursuit(find_player_wanted(-1), ped) != 0
        && task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0
    {
        deregister_ped(get_ped_attractor_manager(), ped, rd::<GPtr<u8>>(this, 0x14));
        return GPtr::NULL;
    }
    let a = rd::<GPtr<u8>>(this, 0x14);
    if a.is_null() { task_create_first_sub_task(this, ped) } else { attractor_get_task_for_ped(a, ped) }
}

/// `CTaskComplexUseEffect::CreateNextSubTask(CPed* ped)` (0x632420, thiscall, `ret 4`): with a ped attractor (+0x14)
/// that the manager still finds for (ped, fx +0xC, entity +0x10) (0x5EBF50), its `GetTaskForPed(ped)`; else 0.
#[hook(addr = 0x632420, conv = thiscall, name = "CTaskComplexUseEffect::CreateNextSubTask", expect = "8B 41 14 85 C0 56 57",
       unverified = "std11-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn task_complex_use_effect_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let a = rd::<GPtr<u8>>(this, 0x14);
    if a.is_null() {
        return GPtr::NULL;
    }
    let (fx, entity) = (rd::<GPtr<u8>>(this, 0xC), rd::<GPtr<u8>>(this, 0x10));
    if find_attractor_for_ped(get_ped_attractor_manager(), ped, fx, entity) == a {
        attractor_get_task_for_ped(a, ped)
    } else {
        GPtr::NULL
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_INTEL_PROCESS_EVENT_HANDLER,
    HOOK_EVENT_ATTRACTOR_DTOR,
    HOOK_TASK_COMPLEX_USE_EFFECT_CREATE_FIRST_SUB_TASK,
    HOOK_TASK_COMPLEX_USE_EFFECT_DTOR,
    HOOK_TASK_COMPLEX_USE_EFFECT_CONTROL_SUB_TASK,
    HOOK_TASK_COMPLEX_USE_EFFECT_CREATE_NEXT_SUB_TASK,
];
