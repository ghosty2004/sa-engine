//! Hand-ported `CEventHandler::Compute*Response` functions (PORT-AI-2). Ported from the 1.0 US bytes (the exe
//! wins over gta-reversed and the specs). Every call to another catalogued function goes through `orig!` at the
//! original address; tasks come from the original task pool (`common::task_new`), stack objects are Rust locals.
//!
//! `CEventHandler` (from the bytes): +0 the ped, +8 / +0xC the history's current / ... event pointers read as
//! "current event" (+0xC first, else +8), +0x20 the physical response task, +0x24 the event response task.
//! `CEventDamage`: +0xC `m_bAddToEventGroup` (u8), +0x14 source entity, +0x1C weapon type, +0x20, +0x24 u8 ped
//! piece, +0x25 flags (bit 1 knock-down, bit 2 anim done), +0x28 anim group, +0x2C anim id, +0x30 blend delta,
//! +0x34 anim speed, +0x38 / +0x3C two floats added for the hit task time, +0x40 u8.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEvent::GetEventType` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CEvent::GetSourceEntity` (vtable +0x28)
    fn v_event_source(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x28 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPed::IsAlive` (0x5E0170)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CEventDamage::IsSameEventForAI(CEventDamage*)` (0x4ADBF0, `ret 4`)
    fn damage_same_event_for_ai(e: GPtr<u8>, o: GPtr<u8>) -> u8 = thiscall 0x4ADBF0 => unported;
    /// `FUN_004abca0` (CEventDamage, `ret 0`)
    fn damage_4abca0(e: GPtr<u8>) -> u8 = thiscall 0x4ABCA0 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTaskManager::GetActiveTask` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CEventHandler::ComputePersonalityResponseToDamage(CEventDamage*, CPed*)` (0x4BF9B0, `ret 8`)
    fn compute_personality_response_to_damage(h: GPtr<u8>, e: GPtr<u8>, src: GPtr<u8>) = thiscall 0x4BF9B0 => unported;
    /// `CEventHandler::ComputeKnockOffBikeResponse(CEvent*, CTask*, CTask*)` (0x4B9FF0, `ret 0xC`)
    fn compute_knock_off_bike_response(h: GPtr<u8>, e: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = thiscall 0x4B9FF0 => unported;
    /// `CEventDamage::ProcessDamage(CPed*)` (0x4B3A20, `ret 4`)
    fn damage_process_damage(e: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x4B3A20 => unported;
    /// `CEventDamage::ComputeDamageAnim(CPed*, bool)` (0x4B3FC0, `ret 8`)
    fn damage_compute_damage_anim(e: GPtr<u8>, ped: GPtr<u8>, b: u32) = thiscall 0x4B3FC0 => unported;
    /// `CEventDamage::ComputeDeathAnim(CPed*, bool)` (0x4B3A60, `ret 8`)
    fn damage_compute_death_anim(e: GPtr<u8>, ped: GPtr<u8>, b: u32) = thiscall 0x4B3A60 => unported;
    /// `CEventDamage::WasHeadShot(bool* out)` (0x4ADD60, `ret 4`)
    fn damage_was_head_shot(e: GPtr<u8>, out: GPtr<u8>) = thiscall 0x4ADD60 => unported;
    /// `CInterestingEvents::Add(int32 type, CEntity*)` (0x602590, `ret 8`, on g_InterestingEvents 0xC0B058)
    fn interesting_events_add(ie: GPtr<u8>, ty: i32, e: GPtr<u8>) = thiscall 0x602590 => unported;
    /// `CPed::SetPedState(int32)` (0x5E4500, `ret 4`)
    fn ped_set_ped_state(p: GPtr<u8>, s: i32) = thiscall 0x5E4500 => unported;
    /// `CPedIntelligence::GetTaskUseGun` (0x600F70)
    fn intel_get_task_use_gun(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F70 => unported;
    /// `CPedIntelligence::AddTaskPhysResponse(CTask*, bool)` (0x600DC0, `ret 8`)
    fn intel_add_task_phys_response(i: GPtr<u8>, t: GPtr<u8>, b: u32) = thiscall 0x600DC0 => unported;
    /// `CTaskSimpleStealthKill::Constructor(bool, CPed*, AssocGroupId)` (0x6225F0, `ret 0xC`)
    fn stealth_kill_ctor(t: GPtr<u8>, b: u32, src: GPtr<u8>, g: u32) -> GPtr<u8> = thiscall 0x6225F0 => unported;
    /// `CTaskComplexFallAndGetUp::Constructor(animId, group, int32 time)` (0x6786C0, `ret 0xC`)
    fn fall_and_get_up_ctor(t: GPtr<u8>, anim: u32, group: u32, time: i32) -> GPtr<u8> = thiscall 0x6786C0 => unported;
    /// `CTaskSimpleBeHit::Constructor(CPed*, int32, uint8, int32)` (0x620780, `ret 0x10`)
    fn be_hit_ctor(t: GPtr<u8>, src: u32, a: u32, piece: u32, time: i32) -> GPtr<u8> = thiscall 0x620780 => unported;
    /// `CTaskSimpleChoking::Constructor(CPed*, bool)` (0x6202C0, `ret 8`)
    fn choking_ctor(t: GPtr<u8>, src: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6202C0 => unported;
    /// `CTaskSimpleChoking::UpdateChoke(CPed*, CPed*, bool)` (0x620660, `ret 0xC`)
    fn choking_update_choke(t: GPtr<u8>, ped: GPtr<u8>, src: GPtr<u8>, b: u32) = thiscall 0x620660 => unported;
    /// `CTaskComplexDie::Constructor(weapon, group, anim, blendDelta, speed, stealth, falling, fallDir,
    /// overRailing)` (0x630040, `ret 0x24`)
    fn die_ctor(t: GPtr<u8>, wt: u32, group: u32, anim: u32, delta: u32, speed: u32, stealth: u32, falling: u32, dir: u32, rail: u32) -> GPtr<u8> = thiscall 0x630040 => unported;
    /// `CTaskComplexFallToDeath::CalcFall(CPed*, int32*, bool*)` (0x6796C0, static)
    fn fall_to_death_calc_fall(ped: GPtr<u8>, dir: GPtr<u8>, rail: GPtr<u8>) -> u8 = cdecl 0x6796C0 => unported;
    /// `CEventHandler::RegisterKill(CPed*, CEntity*, eWeaponType, bool)` (0x4B9340, static)
    fn register_kill(ped: GPtr<u8>, src: GPtr<u8>, wt: u32, head: u32) = cdecl 0x4B9340 => unported;
    /// `CAnimManager::GetAnimAssociation(group, id)` (0x4D3A60)
    fn anim_get_anim_association(group: u32, id: u32) -> GPtr<u8> = cdecl 0x4D3A60 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, uint32 animId)` (0x4D68B0)
    fn clump_get_association(clump: u32, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*, uint32 flags)` (0x4D6A70)
    fn clump_get_first_association_flags(clump: u32, flags: u32) -> GPtr<u8> = cdecl 0x4D6A70 => unported;
    /// `CAnimManager::AddAnimation(RpClump*, group, id)` (0x4D3AA0)
    fn anim_add_animation(clump: u32, group: u32, id: u32) -> GPtr<u8> = cdecl 0x4D3AA0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, group, id, float delta)` (0x4D4610)
    fn anim_blend_animation(clump: u32, group: u32, id: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80, `ret 4`)
    fn anim_set_current_time(a: GPtr<u8>, t: u32) = thiscall 0x4CEA80 => unported;
    /// `CGeneral::GetRandomNumberInRange<float>(min, max)` (0x41BD90; draws its own rand, ST0)
    fn general_random_float_in_range(min: u32, max: u32) -> f64 = cdecl 0x41BD90 => unported;
    /// `Fx_c::AddBlood(CVector* pos, CVector* dir, int32 count, float light)` (0x49EB00, `ret 0x10`)
    fn fx_add_blood(fx: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>, n: i32, light: u32) = thiscall 0x49EB00 => unported;
    /// `CEventGotKnockedOverByCar::CEventGotKnockedOverByCar(CVehicle*)` (0x4B1B60, `ret 4`)
    fn knocked_over_by_car_ctor(e: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B1B60 => unported;
    /// `CEventGotKnockedOverByCar` destructor (0x4B1C10)
    fn knocked_over_by_car_dtor(e: GPtr<u8>) = thiscall 0x4B1C10 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// Event / task scalar deleting destructor (vtable +0)
    fn v_delete(e: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CEvent::Clone` (vtable +0x10)
    fn v_event_clone(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x10 => unported;
    /// `CEvent::TakesPriorityOver(const CEvent&)` (vtable +0x2C)
    fn v_event_takes_priority_over(e: GPtr<u8>, o: GPtr<u8>) -> u8 = thiscall vslot 0x2C => unported;
    /// `CEventHandler::IsTemporaryEvent(const CEvent&)` (0x4BC370, static)
    fn handler_is_temporary_event(e: GPtr<u8>) -> u32 = cdecl 0x4BC370 => unported;
    /// `CEventHandlerHistory::StoreActiveEvent` (0x4B8BF0)
    fn history_store_active_event(h: GPtr<u8>) = thiscall 0x4B8BF0 => unported;
    /// `CEventKnockOffBike::CEventKnockOffBike()` (0x4AFC70, default)
    fn kob_ctor_default(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AFC70 => unported;
    /// `CEventKnockOffBike::CEventKnockOffBike(veh, CVector* speed, CVector* impact, float damage, float, uint8 weapon,
    /// uint8 direction, int32, CPed*, bool, bool)` (0x4AFCF0, `ret 0x2C`)
    fn kob_ctor(e: GPtr<u8>, veh: GPtr<u8>, speed: GPtr<u8>, impact: GPtr<u8>, dmg: u32, a4: u32, weapon: u32, dir: u32, a7: u32, a8: u32, a9: u32, a10: u32) -> GPtr<u8> = thiscall 0x4AFCF0 => unported;
    /// `CEventKnockOffBike::operator=` (0x4B4AA0, `ret 4`)
    fn kob_assign(e: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B4AA0 => unported;
    /// `CEventKnockOffBike` destructor (0x4AFE70)
    fn kob_dtor(e: GPtr<u8>) = thiscall 0x4AFE70 => unported;
    /// `CEventKnockOffBike::SetPedSafePosition(CPed*)` (0x4B4AC0, `ret 4`)
    fn kob_set_ped_safe_position(e: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4B4AC0 => unported;
    /// `CEventKnockOffBike::SetPedOutCar(CPed*)` (0x4AFF60, `ret 4`)
    fn kob_set_ped_out_car(e: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x4AFF60 => unported;
    /// `CEventKnockOffBike::CalcForcesAndAnims(CPed*)` (0x4B0020, `ret 4`; returns the anim id)
    fn kob_calc_forces_and_anims(e: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x4B0020 => unported;
    /// `CEventDamage::CEventDamage(CEntity*, uint32 time, weapon, piece, direction, bool, bool)` (0x4AD830, `ret 0x1C`)
    fn damage_ctor(e: GPtr<u8>, src: u32, time: u32, wt: u32, piece: u32, dir: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x4AD830 => unported;
    /// `CEventDamage` destructor (0x4AD960)
    fn damage_dtor(e: GPtr<u8>) = thiscall 0x4AD960 => unported;
    /// `CEventDamage::AffectsPed(CPed*)` (0x4B35A0, `ret 4`)
    fn damage_affects_ped(e: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4B35A0 => unported;
    /// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(CEntity*, float, weapon, piece, bool)` (0x4AD3F0, `ret 0x14`)
    fn calc_ctor(c: GPtr<u8>, src: u32, dmg: u32, wt: u32, piece: u32, b: u32) -> GPtr<u8> = thiscall 0x4AD3F0 => unported;
    /// `CPedDamageResponseCalculator::ComputeDamageResponse(CPed*, CPedDamageResponse*, bool)` (0x4B5AC0, `ret 0xC`)
    fn calc_compute_damage_response(c: GPtr<u8>, ped: GPtr<u8>, resp: GPtr<u8>, b: u32) = thiscall 0x4B5AC0 => unported;
    /// `CPedDamageResponseCalculator` destructor (0x4AD420)
    fn calc_dtor(c: GPtr<u8>) = thiscall 0x4AD420 => unported;
    /// `operator*(CVector* out, float, const CVector&)` (0x40FE90, cdecl)
    fn vector_scale(out: GPtr<u8>, f: u32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `CTaskComplexInAirAndLand::CTaskComplexInAirAndLand(bool, bool)` (0x678C80, `ret 8`)
    fn in_air_and_land_ctor(t: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x678C80 => unported;
    /// `CPedGroups::GetPedsGroup(CPed*)` (0x5F7E80, cdecl)
    fn ped_groups_get_peds_group(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::IsLeader(const CPed*)` (0x5F69C0, `ret 4`)
    fn membership_is_leader(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5F69C0 => unported;
    /// `CPedGroupMembership::GetLeader` (0x5F69A0)
    fn membership_get_leader(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F69A0 => unported;
    /// `CEventGroup::GetEventOfType(int32)` (0x4AB650, `ret 4`)
    fn event_group_get_event_of_type(g: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x4AB650 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10, `ret 4`)
    fn sequence_add_task(t: GPtr<u8>, task: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskComplexEnterCarAsPassenger::CTaskComplexEnterCarAsPassenger(CVehicle*, int32 seat, bool)` (0x640340, `ret 0xC`)
    fn enter_car_as_passenger_ctor(t: GPtr<u8>, veh: u32, seat: u32, b: u32) -> GPtr<u8> = thiscall 0x640340 => unported;
    /// `CEventLeaderExitedCarAsDriver::CEventLeaderExitedCarAsDriver` (0x4B8300)
    fn leader_exited_car_ctor(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B8300 => unported;
    /// `CEventGroupEvent::CEventGroupEvent(CPed*, CEvent*)` (0x4ADFD0, `ret 8`)
    fn group_event_ctor(e: GPtr<u8>, ped: GPtr<u8>, ev: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ADFD0 => unported;
    /// `CEventGroupEvent::~CEventGroupEvent` (0x4AE070)
    fn group_event_dtor(e: GPtr<u8>) = thiscall 0x4AE070 => unported;
    /// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470, `ret 4`)
    fn pgi_add_event(i: GPtr<u8>, e: GPtr<u8>) = thiscall 0x5F7470 => unported;
    /// `CEventDraggedOutCar::CEventDraggedOutCar(CVehicle*, CPed*, bool)` (0x4AD250, `ret 0xC`)
    fn dragged_out_car_ctor(e: GPtr<u8>, veh: GPtr<u8>, ped: u32, b: u32) -> GPtr<u8> = thiscall 0x4AD250 => unported;
    /// `CEventDraggedOutCar::~CEventDraggedOutCar` (0x4AD330)
    fn dragged_out_car_dtor(e: GPtr<u8>) = thiscall 0x4AD330 => unported;
}

/// `g_InterestingEvents`
const INTERESTING_EVENTS: usize = 0xC0B058;
/// `g_fx` (`Fx_c`)
const G_FX: usize = 0xA9AE00;

fn ped_type(e: GPtr<u8>) -> u8 {
    rd::<u8>(e, 0x36) & 7
}

/// The inlined "is this the same damage event as the handler's current one" test: the current event (+0xC,
/// else +8; each read twice as the original does) has type 9 (EVENT_DAMAGE) and `IsSameEventForAI` agrees.
fn same_as_current(this: GPtr<u8>, ev: GPtr<u8>) -> bool {
    if rd::<GPtr<u8>>(this, 0xC).is_null() && rd::<GPtr<u8>>(this, 8).is_null() {
        return false;
    }
    let cur = {
        let c = rd::<GPtr<u8>>(this, 0xC);
        if c.is_null() { rd::<GPtr<u8>>(this, 8) } else { c }
    };
    if v_event_type(cur) != 9 {
        return false;
    }
    let cur = {
        let c = rd::<GPtr<u8>>(this, 0xC);
        if c.is_null() { rd::<GPtr<u8>>(this, 8) } else { c }
    };
    damage_same_event_for_ai(ev, cur) != 0
}

/// `CEventHandler::ComputeDamageResponse(CEventDamage*, CTask* active, CTask* simplest, CTask*)` (0x4C0170,
/// `ret 0x10`; the fourth argument is unused). Nothing for a dead ped. Read from the bytes (labels are the
/// original addresses):
/// - `m_bAddToEventGroup` clear: unless the event is the current one (type 9 + `IsSameEventForAI`), a
///   `TASK_COMPLEX_REACT_TO_ATTACK` (0x3EE) is active, the ped is a player, or the source (+0x14) is not a ped:
///   `ComputePersonalityResponseToDamage(event, source)`.
/// - Else `ProcessDamage(ped)`; weapons 0x31/0x32 add interesting event 0xF. On a bike-like vehicle (+0x590 == 9
///   or +0x594 == 2): knock-down flag or `FUN_004abca0` → `ComputeKnockOffBikeResponse`, else the personality
///   path with the +0x14 source (0x4C0236).
/// - Health (+0x540) below [0x858624] (or flag +0x40 with +0xC, or in a vehicle with the knock-down flag): the
///   death path (0x4C07D0): bike knock-off as above; for the fall anims 0xBF/0xC0 the anim is re-chosen from the
///   simplest task (0xCF / 0xCD) with blood for weapon 0x36; `ComputeDeathAnim`; the fall-to-death check;
///   `CTaskComplexDie` as event response task (+0x24); interesting event 0x1C; physical response cleared unless a
///   choking task (0x107) holds it for weapons 0x29/0x2A/0x11; `WasHeadShot`; `RegisterKill`.
/// - Alive: stealth kill (0x403 active on the attacker) → `CTaskSimpleStealthKill` + ped state 0x38; else
///   damage anim, choking (0x29/0x2A/0x11 without a pistol-whip gun task), fall-and-get-up (knock-down flag),
///   partial anims (flag bit 10 of the anim's static association) or `CTaskSimpleBeHit` (+0x20), then the
///   personality path through the virtual `GetSourceEntity` (0x4C062D). Weapons >= 0x2F (0x4C06D7): fall-and-get-up
///   for 0x31/0x32/0x36 outside vehicles and a `CEventGotKnockedOverByCar` for the vehicle or the driving ped.
///
/// Bool arguments the original pushes as dwords with uninitialised upper bytes (`CTaskSimpleChoking` 0x4C0615,
/// `RegisterKill` 0x4C0BA3) are passed zero-extended (the callees read the low byte; `RegisterKill` reads the
/// dword at 0x4B9379 and passes it on).
#[hook(addr = 0x4C0170, conv = thiscall, name = "CEventHandler::ComputeDamageResponse", expect = "64 A1 00 00 00 00")]
pub fn event_handler_compute_damage_response(
    this: GPtr<u8>,
    ev: GPtr<u8>,
    active: GPtr<u8>,
    simplest: GPtr<u8>,
    _unused: GPtr<u8>,
) {
    let ped = rd::<GPtr<u8>>(this, 0);
    if ped_is_alive(ped) == 0 {
        return;
    }
    let src = rd::<GPtr<u8>>(ev, 0x14);
    let wt = rd::<i32>(ev, 0x1C);
    let intel = |p: GPtr<u8>| rd::<GPtr<u8>>(p, 0x47C);
    // 0x4C0236: the personality response through the +0x14 source
    let personality_a = || {
        if !src.is_null() && ped_type(src) == 3 {
            compute_personality_response_to_damage(this, ev, src);
        }
    };
    if rd::<u8>(ev, 0xC) == 0 {
        let same = same_as_current(this, ev);
        if tm_find_active_task_by_type(intel(ped).byte_add(4), 0x3EE).is_null()
            && ped_is_player(ped) == 0
            && !same
        {
            personality_a();
        }
        return;
    }
    damage_process_damage(ev, ped);
    let mut falling = [0u32; 1]; // local 0x1C (byte)
    let mut rail = [0u32; 1]; // local 0x14 (byte)
    let mut dir = [0u32; 1]; // local 0x18
    if wt == 0x31 || wt == 0x32 {
        interesting_events_add(g(INTERESTING_EVENTS), 0xF, ped);
    }
    let on_bike = |p: GPtr<u8>| {
        if rd::<u32>(p, 0x46C) & 0x100 == 0 {
            return false;
        }
        let v = rd::<GPtr<u8>>(p, 0x58C);
        !v.is_null() && (rd::<i32>(v, 0x590) == 9 || rd::<i32>(v, 0x594) == 2)
    };
    let in_veh = (rd::<u32>(ped, 0x46C) >> 8) & 1 != 0;
    if on_bike(ped) {
        if rd::<u8>(ev, 0x25) & 2 == 0 && damage_4abca0(ev) == 0 {
            personality_a();
            return;
        }
        compute_knock_off_bike_response(this, ev, active, simplest);
        return;
    }
    let dead = (rd::<u8>(ev, 0x40) != 0 && rd::<u8>(ev, 0xC) != 0)
        || !x(rd::<f32>(ped, 0x540))
            .fcom(x(g::<f32>(0x858624).read()))
            .test_parity(5)
        || (in_veh && rd::<u8>(ev, 0x25) & 2 != 0);
    if dead {
        // 0x4C07D0
        if on_bike(ped) {
            compute_knock_off_bike_response(this, ev, active, simplest);
            return;
        }
        let aid = rd::<i32>(ev, 0x2C);
        if aid == 0xBF || aid == 0xC0 {
            death_anim_from_simplest(ped, ev, simplest);
            let aid = rd::<i32>(ev, 0x2C);
            if aid == 0xBF || aid == 0xC0 {
                damage_compute_death_anim(ev, ped, 0);
            }
            let w = rd::<i32>(ev, 0x1C);
            let fall_weapon = (0x16..=0x21).contains(&w) || w == 0x26;
            if ped_is_player(ped) == 0
                && !src.is_null()
                && fall_weapon
                && ped_type(src) == 3
                && (ped_is_player(src) != 0 || rd::<u8>(src, 0x484) == 2)
            {
                let f = fall_to_death_calc_fall(
                    ped,
                    GPtr::from_raw(&mut dir as *mut [u32; 1]).cast(),
                    GPtr::from_raw(&mut rail as *mut [u32; 1]).cast(),
                );
                falling[0] = f as u32;
            }
        }
        // 0x4C0AA2
        let mut stealth = 0u32;
        if !active.is_null() && v_task_type(active) == 0x403 {
            stealth = (rd::<u8>(active, 8) == 0) as u32;
        }
        let t = task_new(0x28);
        let t = if t.is_null() {
            t
        } else {
            die_ctor(
                t,
                wt as u32,
                rd::<u32>(ev, 0x28),
                rd::<u32>(ev, 0x2C),
                rd::<u32>(ev, 0x30),
                rd::<u32>(ev, 0x34),
                stealth,
                falling[0],
                dir[0],
                rail[0],
            )
        };
        wr::<GPtr<u8>>(this, 0x24, t);
        interesting_events_add(g(INTERESTING_EVENTS), 0x1C, ped);
        let rt = rd::<GPtr<u8>>(intel(ped), 4);
        if !rt.is_null() && !(v_task_type(rt) == 0x107 && (wt == 0x29 || wt == 0x2A || wt == 0x11)) {
            intel_add_task_phys_response(intel(ped), GPtr::NULL, 0);
        }
        let mut head = [0u32; 1];
        damage_was_head_shot(ev, GPtr::from_raw(&mut head as *mut [u32; 1]).cast());
        if !src.is_null() {
            register_kill(ped, src, wt as u32, head[0]);
        }
        return;
    }
    if src.is_null() {
        return;
    }
    if wt >= 0x2F {
        // 0x4C06D7
        if in_veh {
            return;
        }
        let aid = rd::<i32>(ev, 0x2C);
        if aid == 0xBF || aid == 0xC0 || !(wt == 0x31 || wt == 0x32 || wt == 0x36) {
            return;
        }
        let t = task_new(0x18);
        let t = if t.is_null() {
            t
        } else {
            let time = if rd::<u32>(ped, 0x480) != 0 { 500 } else { 1000 };
            fall_and_get_up_ctor(t, rd::<u32>(ev, 0x2C), rd::<u32>(ev, 0x28), time)
        };
        wr::<GPtr<u8>>(this, 0x20, t);
        let veh = match ped_type(src) {
            2 => src,
            3 if rd::<u32>(src, 0x46C) & 0x100 != 0 => rd::<GPtr<u8>>(src, 0x58C),
            _ => return,
        };
        if veh.is_null() {
            return;
        }
        let mut e = [0u32; 8];
        let ep: GPtr<u8> = GPtr::from_raw(&mut e as *mut [u32; 8]).cast();
        knocked_over_by_car_ctor(ep, veh);
        event_group_add(intel(ped).byte_add(0x68), ep, 0);
        knocked_over_by_car_dtor(ep);
        return;
    }
    if !in_veh && alive_response(this, ped, ev, src, wt) {
        return;
    }
    // 0x4C062D
    let same = same_as_current(this, ev);
    if !tm_find_active_task_by_type(intel(ped).byte_add(4), 0x3EE).is_null()
        || ped_is_player(ped) != 0
        || same
    {
        return;
    }
    if v_event_source(ev).is_null() || ped_type(v_event_source(ev)) != 3 {
        return;
    }
    let s = v_event_source(ev);
    compute_personality_response_to_damage(this, ev, s);
}

/// 0x4C0814..0x4C0A1C: the fall anims 0xBF/0xC0 replaced from the simplest task: a 0xCF task (with blood x10 for
/// weapon 0x36 when +0x3C + +0x38 > [0x858BA4]) copies its running anim (+0x14: blend amount > [0x858B8C], delta at
/// least [0x858B50], current time < total) into the event's group/id, else (its +8 set) picks 0x27/0x24; a 0xCD task
/// with its anim (+0xC) less than [0x858B8C] through picks 0x27 (anim 0x73) / 0x24, else (+8 clear) 0x27/0x24 by
/// the clump's first association with flag 0x800. Every pick sets group 0, delta 4.0, speed 1.0.
fn death_anim_from_simplest(ped: GPtr<u8>, ev: GPtr<u8>, t: GPtr<u8>) {
    if t.is_null() {
        return;
    }
    let pick_by_flags = || {
        if clump_get_first_association_flags(rd::<u32>(ped, 0x18), 0x800).is_null() {
            0x24
        } else {
            0x27
        }
    };
    if v_task_type(t) == 0xCF {
        if rd::<i32>(ev, 0x1C) == 0x36
            && !x(rd::<f32>(ev, 0x3C))
                .fadd_m(rd::<f32>(ev, 0x38))
                .fcom(x(g::<f32>(0x858BA4).read()))
                .test_any(0x41)
        {
            let mut dirv = [0u32, 0, 0x4000_0000];
            for _ in 0..10 {
                let m = rd::<GPtr<u8>>(ped, 0x14);
                let p = if m.is_null() {
                    ped.byte_add(4)
                } else {
                    m.byte_add(0x30)
                };
                let mut pos = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
                let r = X87::from_f64_exact(general_random_float_in_range(0xBF00_0000, 0x3F00_0000));
                pos[0] = f32_of(r.fadd_m(f32::from_bits(pos[0]))).to_bits();
                let r = X87::from_f64_exact(general_random_float_in_range(0xBF00_0000, 0x3F00_0000));
                pos[1] = f32_of(r.fadd_m(f32::from_bits(pos[1]))).to_bits();
                fx_add_blood(
                    g(G_FX),
                    GPtr::from_raw(&mut pos as *mut [u32; 3]).cast(),
                    GPtr::from_raw(&mut dirv as *mut [u32; 3]).cast(),
                    10,
                    rd::<u32>(ped, 0x12C),
                );
            }
        }
        let a = rd::<GPtr<u8>>(t, 0x14);
        if !a.is_null()
            && !x(rd::<f32>(a, 0x18))
                .fcom(x(g::<f32>(0x858B8C).read()))
                .test_any(0x41)
            && !x(rd::<f32>(a, 0x1C))
                .fcom(x(g::<f32>(0x858B50).read()))
                .test_any(0x01)
            && !x(rd::<f32>(a, 0x20))
                .fcom(x(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10)))
                .test_parity(5)
        {
            wr::<i32>(ev, 0x28, rd::<i16>(a, 0xE) as i32);
            wr::<i32>(ev, 0x2C, rd::<i16>(a, 0x2C) as i32);
            wr::<u32>(ev, 0x30, 0x4080_0000);
            wr::<u32>(ev, 0x34, 0x3F80_0000);
            return;
        }
        if rd::<u8>(t, 8) == 0 {
            return;
        }
        let id = pick_by_flags();
        wr::<u32>(ev, 0x34, 0x3F80_0000);
        wr::<u32>(ev, 0x30, 0x4080_0000);
        wr::<u32>(ev, 0x28, 0);
        wr::<u32>(ev, 0x2C, id);
        return;
    }
    if v_task_type(t) != 0xCD {
        return;
    }
    let a = rd::<GPtr<u8>>(t, 0xC);
    if !a.is_null() {
        let r = x(rd::<f32>(a, 0x20)).fdiv(x(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10)));
        if !r.fcom(x(g::<f32>(0x858B8C).read())).test_parity(5) {
            wr::<u32>(ev, 0x34, 0x3F80_0000);
            wr::<u32>(ev, 0x30, 0x4080_0000);
            wr::<u32>(ev, 0x28, 0);
            wr::<u32>(ev, 0x2C, if rd::<u16>(a, 0x2C) == 0x73 { 0x27 } else { 0x24 });
            return;
        }
    }
    if rd::<u8>(t, 8) != 0 {
        return;
    }
    let id = pick_by_flags();
    wr::<u32>(ev, 0x28, 0);
    wr::<u32>(ev, 0x2C, id);
    wr::<u32>(ev, 0x30, 0x4080_0000);
    wr::<u32>(ev, 0x34, 0x3F80_0000);
}

/// 0x4C0338..0x4C062D: the alive, on-foot response (weapon < 0x2F). True when the original leaves the function
/// (the stealth-kill path, 0x4C03D1); every other path continues at 0x4C062D.
fn alive_response(this: GPtr<u8>, ped: GPtr<u8>, ev: GPtr<u8>, src: GPtr<u8>, wt: i32) -> bool {
    let intel = |p: GPtr<u8>| rd::<GPtr<u8>>(p, 0x47C);
    // EBP / local 0x74: the source when it is a ped, else null
    let src_ped = if ped_type(src) == 3 { src } else { GPtr::NULL };
    if !src_ped.is_null()
        && !tm_get_active_task(intel(src).byte_add(4)).is_null()
        && v_task_type(tm_get_active_task(intel(src).byte_add(4))) == 0x403
    {
        let t = task_new(0x20);
        let t = if t.is_null() {
            t
        } else {
            let grp = rd::<u32>(tm_get_active_task(intel(src).byte_add(4)), 0x10);
            stealth_kill_ctor(t, 0, src, grp)
        };
        wr::<GPtr<u8>>(this, 0x20, t);
        ped_set_ped_state(ped, 0x38);
        return true;
    }
    // 0x4C03D6
    if rd::<i32>(ev, 0x2C) == 0xBF {
        damage_compute_damage_anim(ev, ped, 0);
    }
    let choke = (wt == 0x29 || wt == 0x2A || wt == 0x11)
        && (intel_get_task_use_gun(intel(src_ped)).is_null()
            || rd::<u8>(intel_get_task_use_gun(intel(src_ped)), 0x10) != 5);
    if choke {
        // 0x4C05B9
        let rt = rd::<GPtr<u8>>(intel(ped), 4);
        if !rt.is_null() && v_task_type(rt) == 0x107 {
            let rt = rd::<GPtr<u8>>(intel(ped), 4);
            choking_update_choke(rt, ped, src_ped, (wt == 0x11) as u32);
            return false;
        }
        let t = task_new(0x1C);
        let t = if t.is_null() {
            t
        } else {
            choking_ctor(t, src_ped, (wt == 0x11) as u32)
        };
        wr::<GPtr<u8>>(this, 0x20, t);
        return false;
    }
    // 0x4C042B
    if rd::<u8>(ev, 0x25) & 2 != 0 {
        let t = task_new(0x18);
        let t = if t.is_null() {
            t
        } else {
            let v = xi(rd::<u8>(ped, 0x719) as i32)
                .fmul_m(g::<f32>(0x859038).read())
                .fdivr_m(g::<f32>(0x858C4C).read());
            fall_and_get_up_ctor(t, rd::<u32>(ev, 0x2C), rd::<u32>(ev, 0x28), ftol32(v))
        };
        wr::<GPtr<u8>>(this, 0x20, t);
        return false;
    }
    let aid = rd::<i32>(ev, 0x2C);
    if aid == 0xBF || aid == 0xC0 {
        return false;
    }
    let def = anim_get_anim_association(rd::<u32>(ev, 0x28), aid as u32);
    if (rd::<u16>(def, 0xA) as u32 >> 10) & 1 != 0 {
        if rd::<u8>(ev, 0x25) & 4 != 0 {
            return false;
        }
        let aid = rd::<i32>(ev, 0x2C);
        let clump = rd::<u32>(ped, 0x18);
        if (0x1C..=0x1F).contains(&aid) {
            let mut a = clump_get_association(clump, aid as u32);
            if a.is_null() {
                a = anim_add_animation(rd::<u32>(ped, 0x18), rd::<u32>(ev, 0x28), rd::<u32>(ev, 0x2C));
            }
            wr::<u32>(a, 0x18, 0);
            wr::<u32>(a, 0x1C, rd::<u32>(ev, 0x30));
            wr::<u32>(a, 0x24, rd::<u32>(ev, 0x34));
            anim_set_current_time(a, 0);
        } else {
            let a = anim_blend_animation(clump, rd::<u32>(ev, 0x28), aid as u32, rd::<u32>(ev, 0x30));
            wr::<u32>(a, 0x24, rd::<u32>(ev, 0x34));
            anim_set_current_time(a, 0);
        }
        wr::<u8>(ev, 0x25, rd::<u8>(ev, 0x25) | 4);
        return false;
    }
    let t = task_new(0x28);
    let t = if t.is_null() {
        t
    } else {
        let v = x(rd::<f32>(ev, 0x3C)).fadd_m(rd::<f32>(ev, 0x38));
        let piece = rd::<u8>(ev, 0x24) as u32;
        be_hit_ctor(t, src_ped.addr() as u32, rd::<u32>(ev, 0x20), piece, ftol32(v))
    };
    wr::<GPtr<u8>>(this, 0x20, t);
    wr::<u32>(t, 0x10, rd::<u32>(ev, 0x2C));
    wr::<u32>(t, 0x14, rd::<u32>(ev, 0x28));
    wr::<u8>(t, 0xD, (rd::<u8>(ev, 0x25) >> 2) & 1);
    false
}

/// The knock-off direction vector of 0x4BA11D / 0x4BA27B (`local 0x10`): direction 0 = the ped's forward axis
/// times -1.0 [0x858C1C], 1 = its right axis, 2 = its forward axis, other = `operator*(-1.0, right)` (0x40FE90).
fn kob_direction(m: GPtr<u8>, dir: u32) -> [u32; 3] {
    let mut out = [0u32; 3];
    match dir {
        0 => {
            let k = g::<f32>(0x858C1C).read();
            for (i, o) in out.iter_mut().enumerate() {
                *o = f32_of(x(rd::<f32>(m, 0x10 + i as isize * 4)).fmul_m(k)).to_bits();
            }
        }
        1 | 2 => {
            let base = if dir == 1 { 0 } else { 0x10 };
            for (i, o) in out.iter_mut().enumerate() {
                *o = rd::<u32>(m, base + i as isize * 4);
            }
        }
        _ => {
            let mut tmp = [0u32; 3];
            let r = vector_scale(GPtr::from_raw(&mut tmp as *mut [u32; 3]).cast(), 0xBF80_0000, m);
            for (i, o) in out.iter_mut().enumerate() {
                *o = rd::<u32>(r, i as isize * 4);
            }
        }
    }
    out
}

/// The vehicle's move speed (+0x44..+0x4C) times 0.75 [0x858F34] (`local 0x34`).
fn kob_speed(veh: GPtr<u8>) -> [u32; 3] {
    let k = g::<f32>(0x858F34).read();
    let mut v = [0u32; 3];
    for (i, o) in v.iter_mut().enumerate() {
        *o = f32_of(x(rd::<f32>(veh, 0x44 + i as isize * 4)).fmul_m(k)).to_bits();
    }
    v
}

/// Builds the stack `CEventKnockOffBike` (0x4AFCF0: vehicle, speed, direction vector, 50.0 damage, 0, weapon,
/// direction, 0, 0, 0, 0) and assigns it to `evp` (0x4B4AA0), then destroys the temporary.
fn kob_build(this: GPtr<u8>, evp: GPtr<u8>, mut speed: [u32; 3], mut dirv: [u32; 3], weapon: u32, dir: u32) {
    let mut evt2 = [0u32; 0x14];
    let e2: GPtr<u8> = GPtr::from_raw(&mut evt2 as *mut [u32; 0x14]).cast();
    let veh = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x58C);
    let r = kob_ctor(
        e2,
        veh,
        GPtr::from_raw(&mut speed as *mut [u32; 3]).cast(),
        GPtr::from_raw(&mut dirv as *mut [u32; 3]).cast(),
        0x4248_0000,
        0,
        weapon,
        dir,
        0,
        0,
        0,
        0,
    );
    kob_assign(evp, r);
    kob_dtor(e2);
}

/// `CEventHandler::ComputeKnockOffBikeResponse(CEvent*, CTask*, CTask*)` (0x4B9FF0, `ret 0xC`; the entry continues
/// through the island 0x40749C, which pushes the SEH handler; the two task arguments are unused). Read from the
/// bytes (`GetEventType` is asked again for every compare, as the original does):
/// - A knock-off event (type 8) is used as is, after a stack `CEventDamage(+0x38, now, 0x36, 3, 2, true, false)`
///   whose `AffectsPed` gates `CPedDamageResponseCalculator(+0x38, +0x28, 0x36, 3, true)::ComputeDamageResponse(ped,
///   &damage.response (+0x38), true)`.
/// - A damage event (type 9), or types 0x23 / 0x4F, build a `CEventKnockOffBike` from the ped's vehicle speed x
///   0.75, a direction vector (the damage event's +0x24, or `(rand() & 0xFFFF) * 2^-15 * 4.0` truncated for the
///   others, rand return address 0x4BA257), 50.0 damage and weapon +0x1C (types 0x23/0x4F: 0x33), assigned to the
///   local default-constructed one. Any other type makes the original read +0x38 of a null event.
/// - With a vehicle (+0x38) holding the ped in its driver or passenger slots (+0x460..+0x46C): interesting event
///   0x10; `SetPedSafePosition` failing clears +0x1C bit 0 of an in-vehicle ped and stops; else `SetPedOutCar`,
///   `CalcForcesAndAnims`. Weapon 0x35 clears ped flags +0x46C bits 0/1, sets bit 9 and +0x40 bits 8/27 and makes
///   an in-air (health > 0) or die task; otherwise a dead ped (health <= 0) gets a die task (fall anim 0xBF re-chosen
///   by +0x34: 0x6A / 0x19 / 0x19 / 0x1B), leaves the vehicle flag and stops after the bike flag; a living one with a
///   weapon gets fall-and-get-up plus the group reactions (a follower whose leader drives re-enters the car through a
///   sequence, a leader informs its group). A dragger (+0x30) adds a `CEventDraggedOutCar` and sets bike (+0x590 ==
///   9) flag +0x614 bit 4; without one the flag is cleared.
#[hook(addr = 0x4B9FF0, conv = thiscall, name = "CEventHandler::ComputeKnockOffBikeResponse", expect = "6A FF E9 A5 D4 F4 FF")]
pub fn event_handler_compute_knock_off_bike_response(
    this: GPtr<u8>,
    ev: GPtr<u8>,
    _a: GPtr<u8>,
    _b: GPtr<u8>,
) {
    let mut evt = [0u32; 0x14];
    let evp: GPtr<u8> = GPtr::from_raw(&mut evt as *mut [u32; 0x14]).cast();
    kob_ctor_default(evp);
    let kob = if v_event_type(ev) == 8 {
        let mut dmg = [0u32; 0x14];
        let dp: GPtr<u8> = GPtr::from_raw(&mut dmg as *mut [u32; 0x14]).cast();
        damage_ctor(
            dp,
            rd::<u32>(ev, 0x38),
            g::<u32>(0xB7CB84).read(),
            0x36,
            3,
            2,
            1,
            0,
        );
        if damage_affects_ped(dp, rd::<GPtr<u8>>(this, 0)) != 0 {
            let mut calc = [0u32; 0x10];
            let cp: GPtr<u8> = GPtr::from_raw(&mut calc as *mut [u32; 0x10]).cast();
            calc_ctor(cp, rd::<u32>(ev, 0x38), rd::<u32>(ev, 0x28), 0x36, 3, 1);
            calc_compute_damage_response(cp, rd::<GPtr<u8>>(this, 0), dp.byte_add(0x38), 1);
            calc_dtor(cp);
        }
        damage_dtor(dp);
        ev
    } else if v_event_type(ev) == 9 {
        let ped = rd::<GPtr<u8>>(this, 0);
        let speed = kob_speed(rd::<GPtr<u8>>(ped, 0x58C));
        let d = rd::<u8>(ev, 0x24) as u32;
        let dirv = kob_direction(rd::<GPtr<u8>>(ped, 0x14), d);
        kob_build(this, evp, speed, dirv, rd::<u32>(ev, 0x1C), d);
        evp
    } else if v_event_type(ev) == 0x23 || v_event_type(ev) == 0x4F {
        let speed = kob_speed(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x58C));
        let r = sa_core::orig::at_site(0x4BA257, rand) & 0xFFFF;
        let f = xi(r)
            .fmul_m(g::<f32>(0x858B14).read())
            .fmul_m(g::<f32>(0x858B90).read());
        let d = ftol32(f) as u32;
        let dirv = kob_direction(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x14), d);
        kob_build(this, evp, speed, dirv, 0x33, d);
        evp
    } else {
        GPtr::NULL
    };
    kob_respond(this, kob);
    kob_dtor(evp);
}

/// 0x4BA364..0x4BA776 of `ComputeKnockOffBikeResponse` with the knock-off event `kob` (EBP).
fn kob_respond(this: GPtr<u8>, kob: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(kob, 0x38);
    let time = rd::<u32>(kob, 0x2C);
    if veh.is_null() {
        return;
    }
    let ped = rd::<GPtr<u8>>(this, 0);
    if !(0..4).any(|i| rd::<GPtr<u8>>(veh, 0x460 + i * 4) == ped) {
        return;
    }
    interesting_events_add(g(INTERESTING_EVENTS), 0x10, ped);
    if kob_set_ped_safe_position(kob, rd::<GPtr<u8>>(this, 0)) == 0 {
        let p = rd::<GPtr<u8>>(this, 0);
        if rd::<u32>(p, 0x46C) & 0x100 != 0 {
            wr::<u32>(p, 0x1C, rd::<u32>(p, 0x1C) & !1);
        }
        return;
    }
    kob_set_ped_out_car(kob, rd::<GPtr<u8>>(this, 0));
    let mut anim = kob_calc_forces_and_anims(kob, rd::<GPtr<u8>>(this, 0));
    let weapon = rd::<u8>(kob, 0x36);
    let bike_flag = |set: bool| {
        if rd::<i32>(veh, 0x590) == 9 {
            let b = rd::<u8>(veh, 0x614);
            wr::<u8>(veh, 0x614, if set { b | 0x10 } else { b & 0xEF });
        }
    };
    let health_cmp = |p: GPtr<u8>| x(rd::<f32>(p, 0x540)).fcom(x(g::<f32>(0x858B50).read()));
    if weapon == 0x35 {
        let p = rd::<GPtr<u8>>(this, 0);
        wr::<u32>(p, 0x46C, rd::<u32>(p, 0x46C) & !1);
        wr::<u32>(p, 0x46C, rd::<u32>(p, 0x46C) & !2);
        wr::<u32>(p, 0x46C, rd::<u32>(p, 0x46C) | 0x200);
        wr::<u32>(p, 0x40, rd::<u32>(p, 0x40) | 0x100);
        wr::<u32>(p, 0x40, rd::<u32>(p, 0x40) | 0x800_0000);
        let t = if !health_cmp(rd::<GPtr<u8>>(this, 0)).test_any(0x41) {
            let t = task_new(0x10);
            if t.is_null() {
                t
            } else {
                in_air_and_land_ctor(t, 0, 0)
            }
        } else {
            let t = task_new(0x28);
            if t.is_null() {
                t
            } else {
                die_ctor(
                    t,
                    rd::<i8>(kob, 0x36) as i32 as u32,
                    0,
                    anim,
                    0x4080_0000,
                    0,
                    0,
                    0,
                    0,
                    0,
                )
            }
        };
        wr::<GPtr<u8>>(this, 0x24, t);
    } else if !health_cmp(rd::<GPtr<u8>>(this, 0)).test_parity(0x41) {
        // 0x4BA4CB: health <= 0
        if anim == 0xBF {
            match rd::<i8>(kob, 0x34) as i32 as u32 {
                0 => anim = 0x6A,
                1 | 2 => anim = 0x19,
                3 => anim = 0x1B,
                _ => {}
            }
        }
        let t = task_new(0x28);
        let t = if t.is_null() {
            t
        } else {
            die_ctor(
                t,
                rd::<i8>(kob, 0x36) as i32 as u32,
                0,
                anim,
                0x4080_0000,
                0,
                0,
                0,
                0,
                0,
            )
        };
        wr::<GPtr<u8>>(this, 0x24, t);
        let p = rd::<GPtr<u8>>(this, 0);
        wr::<u32>(p, 0x46C, rd::<u32>(p, 0x46C) & !1);
        bike_flag(false);
        return;
    } else if weapon != 0 {
        // 0x4BA55F: alive
        let t = task_new(0x18);
        let t = if t.is_null() {
            t
        } else {
            fall_and_get_up_ctor(t, anim, 0, time as i32)
        };
        wr::<GPtr<u8>>(this, 0x24, t);
        let group = ped_groups_get_peds_group(rd::<GPtr<u8>>(this, 0));
        if !group.is_null() {
            let ms = group.byte_add(8);
            if membership_is_leader(ms, rd::<GPtr<u8>>(this, 0)) == 0 && rd::<u8>(kob, 0x35) & 1 == 0 {
                let leader = membership_get_leader(ms);
                if !leader.is_null()
                    && rd::<u32>(leader, 0x46C) & 0x100 != 0
                    && event_group_get_event_of_type(rd::<GPtr<u8>>(leader, 0x47C).byte_add(0x68), 8)
                        .is_null()
                {
                    let s = task_new(0x40);
                    let s = if s.is_null() { s } else { sequence_ctor(s) };
                    sequence_add_task(s, rd::<GPtr<u8>>(this, 0x24));
                    let e = task_new(0x50);
                    let e = if e.is_null() {
                        e
                    } else {
                        enter_car_as_passenger_ctor(e, veh.addr() as u32, 0, 1)
                    };
                    sequence_add_task(s, e);
                    wr::<GPtr<u8>>(this, 0x24, s);
                }
            } else if membership_is_leader(ms, rd::<GPtr<u8>>(this, 0)) != 0 {
                let e = event_new(0x14);
                let e = if e.is_null() { e } else { leader_exited_car_ctor(e) };
                let mut ge = [0u32; 0x10];
                let gp: GPtr<u8> = GPtr::from_raw(&mut ge as *mut [u32; 0x10]).cast();
                group_event_ctor(gp, rd::<GPtr<u8>>(this, 0), e);
                pgi_add_event(group.byte_add(0x30), gp);
                group_event_dtor(gp);
            }
        }
        let p = rd::<GPtr<u8>>(this, 0);
        wr::<u32>(p, 0x46C, rd::<u32>(p, 0x46C) & !1);
    }
    // 0x4BA701
    let dragger = rd::<u32>(kob, 0x30);
    if dragger == 0 {
        bike_flag(false);
        return;
    }
    let mut dr = [0u32; 0x10];
    let dp: GPtr<u8> = GPtr::from_raw(&mut dr as *mut [u32; 0x10]).cast();
    dragged_out_car_ctor(dp, veh, dragger, (rd::<u8>(kob, 0x35) & 1) as u32);
    event_group_add(
        rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x47C).byte_add(0x68),
        dp,
        1,
    );
    bike_flag(true);
    dragged_out_car_dtor(dp);
}

/// `CEventHandlerHistory::GetCurrentEvent` (0x4B8CA0): the temp event (+8) if set, else the non-temp one (+4).
#[hook(addr = 0x4B8CA0, conv = thiscall, name = "CEventHandlerHistory::GetCurrentEvent", expect = "8B 41 08 85 C0")]
pub fn history_get_current_event(this: GPtr<u8>) -> GPtr<u8> {
    let e = rd::<GPtr<u8>>(this, 8);
    if e.is_null() { rd::<GPtr<u8>>(this, 4) } else { e }
}

/// `CEventHandler::IsTemporaryEvent(const CEvent&)` (0x4BC370, static cdecl): 1 (EAX) for the event types 1..6,
/// 0x21, 0x38, 0x3C, 0x2C, 0xD, 0x2E, 0x43, 0x5D, 0xC, 0x4A, 0x3B, else 0.
#[hook(addr = 0x4BC370, conv = cdecl, name = "CEventHandler::IsTemporaryEvent", expect = "8B 4C 24 04 8B 01")]
pub fn handler_is_temporary_event_port(e: GPtr<u8>) -> u32 {
    matches!(
        v_event_type(e),
        1..=6 | 0x21 | 0x38 | 0x3C | 0x2C | 0xD | 0x2E | 0x43 | 0x5D | 0xC | 0x4A | 0x3B
    ) as u32
}

/// `CEventHandlerHistory::RecordCurrentEvent(CPed*, const CEvent&)` (0x4BC4B0, `ret 8`; the ped is unused).
/// Script commands (type 0x20) are not recorded. A non-temporary event deletes the old non-temp event (+4),
/// stores its clone there, then deletes and clears +0xC (stored) and +8 (temp). A temporary one stores the
/// non-temp event (`StoreActiveEvent`, +4 cleared) when set, deletes the old temp event and stores its clone at +8.
#[hook(addr = 0x4BC4B0, conv = thiscall, name = "CEventHandlerHistory::RecordCurrentEvent", expect = "56 57 8B 7C 24 10")]
pub fn history_record_current_event(this: GPtr<u8>, _ped: GPtr<u8>, ev: GPtr<u8>) {
    if v_event_type(ev) == 0x20 {
        return;
    }
    if handler_is_temporary_event(ev) & 0xFF == 0 {
        let old = rd::<GPtr<u8>>(this, 4);
        if !old.is_null() {
            v_delete(old, 1);
        }
        let c = v_event_clone(ev);
        wr::<GPtr<u8>>(this, 4, c);
        let s = rd::<GPtr<u8>>(this, 0xC);
        if !s.is_null() {
            v_delete(s, 1);
        }
        wr::<u32>(this, 0xC, 0);
        let t = rd::<GPtr<u8>>(this, 8);
        if !t.is_null() {
            v_delete(t, 1);
        }
        wr::<u32>(this, 8, 0);
        return;
    }
    if !rd::<GPtr<u8>>(this, 4).is_null() {
        history_store_active_event(this);
        wr::<u32>(this, 4, 0);
    }
    let t = rd::<GPtr<u8>>(this, 8);
    if !t.is_null() {
        v_delete(t, 1);
    }
    let c = v_event_clone(ev);
    wr::<GPtr<u8>>(this, 8, c);
}

/// `CEventHandlerHistory::ClearAllEvents` (0x4BC550): deletes and clears the temp (+8) then the non-temp (+4)
/// event.
#[hook(addr = 0x4BC550, conv = thiscall, name = "CEventHandlerHistory::ClearAllEvents", expect = "56 8B F1 8B 4E 08")]
pub fn history_clear_all_events(this: GPtr<u8>) {
    let t = rd::<GPtr<u8>>(this, 8);
    if !t.is_null() {
        v_delete(t, 1);
    }
    wr::<u32>(this, 8, 0);
    let n = rd::<GPtr<u8>>(this, 4);
    if !n.is_null() {
        v_delete(n, 1);
    }
    wr::<u32>(this, 4, 0);
}

/// `CEventHandlerHistory::TakesPriorityOverCurrentEvent(const CEvent&)` (0x4BC580, `ret 4`): with a non-temp
/// event (+4), the event's `TakesPriorityOver(+4)` (tail call); else with a temp event (+8): a temporary event
/// compares with +8 only, another one must take priority over +8 and over the stored event (+0xC) when set;
/// true without any current event.
#[hook(addr = 0x4BC580, conv = thiscall, name = "CEventHandlerHistory::TakesPriorityOverCurrentEvent", expect = "56 8B F1 8B 46 04")]
pub fn history_takes_priority_over_current_event(this: GPtr<u8>, ev: GPtr<u8>) -> u8 {
    let n = rd::<GPtr<u8>>(this, 4);
    if !n.is_null() {
        return v_event_takes_priority_over(ev, n);
    }
    if rd::<GPtr<u8>>(this, 8).is_null() {
        return 1;
    }
    if handler_is_temporary_event(ev) & 0xFF != 0 {
        return v_event_takes_priority_over(ev, rd::<GPtr<u8>>(this, 8));
    }
    if v_event_takes_priority_over(ev, rd::<GPtr<u8>>(this, 8)) == 0 {
        return 0;
    }
    let s = rd::<GPtr<u8>>(this, 0xC);
    if !s.is_null() && v_event_takes_priority_over(ev, s) == 0 {
        return 0;
    }
    1
}

/// The shared body of `CEventHandler::FlushImmediately` (0x4C3820) and the destructor (0x4C3EC0): deletes and
/// clears the history's stored (+0xC) and temp (+8) events, then the physical response task (+0x10) when set.
fn handler_flush(this: GPtr<u8>) {
    let s = rd::<GPtr<u8>>(this, 0xC);
    if !s.is_null() {
        v_delete(s, 1);
    }
    wr::<u32>(this, 0xC, 0);
    let t = rd::<GPtr<u8>>(this, 8);
    if !t.is_null() {
        v_delete(t, 1);
    }
    wr::<u32>(this, 8, 0);
    let r = rd::<GPtr<u8>>(this, 0x10);
    if !r.is_null() {
        v_delete(r, 1);
        wr::<u32>(this, 0x10, 0);
    }
}

/// `CEventHandler::FlushImmediately` (0x4C3820).
#[hook(addr = 0x4C3820, conv = thiscall, name = "CEventHandler::FlushImmediately", expect = "56 8B F1 8B 4E 0C")]
pub fn handler_flush_immediately(this: GPtr<u8>) {
    handler_flush(this);
}

/// `CEventHandler::~CEventHandler` (0x4C3EC0): same body as `FlushImmediately`.
#[hook(addr = 0x4C3EC0, conv = thiscall, name = "CEventHandler::Destructor", expect = "56 8B F1 8B 4E 0C")]
pub fn handler_dtor(this: GPtr<u8>) {
    handler_flush(this);
}

/// `CEventHandler::CEventHandler(CPed*)` (0x4C3E80, `ret 4`): +0 = ped; dwords +4..+0x18 and +0x20..+0x30 and the
/// bytes +0x1C/+0x1D zeroed; returns this.
#[hook(addr = 0x4C3E80, conv = thiscall, name = "CEventHandler::Constructor", expect = "8B C1 8B 4C 24 04")]
pub fn handler_ctor(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    wr::<GPtr<u8>>(this, 0, ped);
    for o in [4, 8, 0xC, 0x10, 0x14, 0x18] {
        wr::<u32>(this, o, 0);
    }
    wr::<u8>(this, 0x1C, 0);
    wr::<u8>(this, 0x1D, 0);
    for o in [0x20, 0x24, 0x28, 0x2C, 0x30] {
        wr::<u32>(this, o, 0);
    }
    this
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_EVENT_HANDLER_COMPUTE_DAMAGE_RESPONSE,
    HOOK_EVENT_HANDLER_COMPUTE_KNOCK_OFF_BIKE_RESPONSE,
    HOOK_HISTORY_GET_CURRENT_EVENT,
    HOOK_HANDLER_IS_TEMPORARY_EVENT_PORT,
    HOOK_HISTORY_RECORD_CURRENT_EVENT,
    HOOK_HISTORY_CLEAR_ALL_EVENTS,
    HOOK_HISTORY_TAKES_PRIORITY_OVER_CURRENT_EVENT,
    HOOK_HANDLER_FLUSH_IMMEDIATELY,
    HOOK_HANDLER_DTOR,
    HOOK_HANDLER_CTOR,
];
