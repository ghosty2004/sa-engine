//! Gang tasks: CTaskComplexGangLeader, CTaskComplexGangFollower, CTaskGangHassle*, CTaskComplexSignalAtPed (PORT-AI-6). Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AI` only as reading aids.
//!
//! Also `CTaskComplexLeaveCarAndWander`. Every call to another catalogued function goes through `orig!` at
//! the original address (this module's own functions included); tasks and events come from the original
//! pools and keep their original vtables; objects the original builds on its stack are zeroed Rust locals.
//! Every `rand()` names its original return address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `g_ikChainMan`
const IK_CHAIN_MAN: usize = 0xC15448;
/// `g_fxMan`
const FX_MAN: usize = 0xA9AE80;
/// `"gangs"` (anim block name)
const STR_GANGS: u32 = 0x86F92C;
/// `CAnimManager::ms_aAnimBlocks` +0x10 (`bLoaded`), stride 0x20.
const ANIM_BLOCK_LOADED: u32 = 0xB5D4B0;
/// `ModelIndices::MI_GANG_DRINK` (u16)
const MI_GANG_DRINK: usize = 0x8CD728;
/// `ModelIndices::MI_GANG_SMOKE` (u16)
const MI_GANG_SMOKE: usize = 0x8CD72C;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: u32 = 0xA9B0C8;

/// An f32 of the image (`.rdata` constant or `.data` tunable), read when the original reads it.
fn c(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// Entity position: matrix +0x30, else the placement at +4.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// A Rust local passed where the original passes the address of a stack object.
fn slot<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

/// `(float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * k` (fild, fmul, fmul) then `_ftol`.
fn rand_scaled(site: u32, k_addr: usize) -> i32 {
    let r = at_site(site, rand) & 0xFFFF;
    ftol32(xi(r).fmul_m(c(0x858B14)).fmul_m(c(k_addr)))
}

/// `fstp dword` of an f64 an original callee returned in ST0.
fn st0(v: f64) -> X87 {
    X87::from_f64_exact(v)
}

/// The inlined `CTaskTimer::IsOutOfTime` tail of a timer at `this + off` (start, interval, started +8,
/// reset +9) once `started` was tested: a pending reset restarts it at now; true when `now >= start +
/// interval` (unsigned, `cmp; sbb; inc`).
fn timer_due(this: GPtr<u8>, off: isize) -> bool {
    if rd::<u8>(this, off + 9) != 0 {
        wr::<u32>(this, off, g::<u32>(TIME_MS).read());
        wr::<u8>(this, off + 9, 0);
    }
    let end = rd::<u32>(this, off + 4).wrapping_add(rd::<u32>(this, off));
    g::<u32>(TIME_MS).read() >= end
}

/// The weapon type of the ped's current weapon slot (`m_aWeapons[m_nActiveWeaponSlot].m_eWeaponType`; the slot
/// byte +0x718 is sign-extended).
fn current_weapon_type(p: GPtr<u8>) -> u32 {
    let s = isize::from(rd::<i8>(p, 0x718));
    rd::<u32>(p, 0x5A0 + s * 0x1C)
}

orig! {
    // ---- virtual slots ----
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CreateNextSubTask(CPed*)` (vtable +0x20)
    fn v_create_next_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x20 => unported;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
    /// `CTaskComplexGangLeader::ScanForStuff(CPed*)` (vtable +0x2C)
    fn v_scan_for_stuff(t: GPtr<u8>, ped: GPtr<u8>) = thiscall vslot 0x2C => unported;
    /// `CTaskComplexWander::GetWanderType()` (vtable +0x2C)
    fn v_wander_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x2C => unported;

    // ---- this module's functions and their neighbours (called through the original address) ----
    /// `CTaskComplexGangLeader::ShouldLoadGangAnims()` (0x65E7F0, static)
    fn should_load_gang_anims() -> u8 = cdecl 0x65E7F0 => unported;
    /// `CTaskComplexGangLeader::GetRandomGangAmbientAnim(CPed*, CEntity*)` (0x65E730, cdecl)
    fn get_random_gang_ambient_anim(ped: GPtr<u8>, held: GPtr<u8>) -> i32 = cdecl 0x65E730 => unported;
    /// `CTaskComplexGangLeader::TryToPassObject(CPed*, CPedGroup*)` (0x65EA50, cdecl)
    fn try_to_pass_object_orig(ped: GPtr<u8>, group: GPtr<u8>) -> GPtr<u8> = cdecl 0x65EA50 => unported;
    /// `CTaskComplexGangFollower::CalculateOffsetPosition(CVector*)` (0x65ED40, `ret 4`)
    fn gang_follower_calculate_offset_position(t: GPtr<u8>, out: GPtr<u8>) = thiscall 0x65ED40 => unported;
    /// `CTaskGangHassleVehicle::CalcTargetOffset()` (0x6641A0)
    fn gang_hassle_vehicle_calc_target_offset_orig(t: GPtr<u8>) = thiscall 0x6641A0 => unported;
    /// `CTaskComplexLeaveCarAndWander::Constructor(CVehicle*, int32, int32, bool)` (0x63BDD0, `ret 0x10`)
    fn leave_car_and_wander_ctor_orig(t: GPtr<u8>, v: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x63BDD0 => unported;
    /// `CTaskComplexLeaveCarAndWander::CreateSubTask(int32)` (0x63BEC0, `ret 4`)
    fn leave_car_and_wander_create_sub_task_orig(t: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x63BEC0 => unported;

    // ---- task bases and constructors ----
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskComplex::~CTaskComplex` (0x61A3D0)
    fn task_complex_dtor(t: GPtr<u8>) = thiscall 0x61A3D0 => unported;
    /// `CTaskSimpleStandStill::Constructor(int32 time, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, time: i32, a: u32, b: u32, f: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskSimpleRunAnim::Constructor(AssocGroupId, AnimationId, float, bool)` (0x61A8B0, `ret 0x10`)
    fn run_anim_ctor(t: GPtr<u8>, group: u32, anim: i32, delta: u32, b: u32) -> GPtr<u8> = thiscall 0x61A8B0 => unported;
    /// `CTaskComplexPlayHandSignalAnim::Constructor(AnimationId, float)` (0x61B2B0, `ret 8`)
    fn play_hand_signal_anim_ctor(t: GPtr<u8>, anim: i32, delta: u32) -> GPtr<u8> = thiscall 0x61B2B0 => unported;
    /// `CTaskComplexPassObject::Constructor(CPed*, bool)` (0x6611B0, `ret 8`)
    fn pass_object_ctor(t: GPtr<u8>, ped: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6611B0 => unported;
    /// `CTaskComplexBeInGroup::Constructor(int32 groupId, bool)` (0x632E50, `ret 8`)
    fn be_in_group_ctor(t: GPtr<u8>, id: i32, b: u32) -> GPtr<u8> = thiscall 0x632E50 => unported;
    /// `CTaskComplexWanderGang` constructor `(int32 moveState, uint8 dir, int32 time, bool, float)` (0x66F5C0, `ret 0x14`)
    fn wander_gang_ctor(t: GPtr<u8>, ms: i32, dir: i32, time: i32, b: u32, f: u32) -> GPtr<u8> = thiscall 0x66F5C0 => unported;
    /// `CTaskComplexSignalAtPed::Constructor(CPed*, int32, bool)` (0x660A30, `ret 0xC`)
    fn signal_at_ped_ctor_orig(t: GPtr<u8>, ped: GPtr<u8>, a: i32, b: u32) -> GPtr<u8> = thiscall 0x660A30 => unported;
    /// `CTaskComplexTurnToFaceEntityOrCoord::Constructor(CEntity*, float, float)` (0x66B890, `ret 0xC`)
    fn turn_to_face_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x66B890 => unported;
    /// `CTaskSimplePause::Constructor(int32 time)` (0x48E750, `ret 4`)
    fn pause_ctor(t: GPtr<u8>, time: i32) -> GPtr<u8> = thiscall 0x48E750 => unported;
    /// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::Constructor(CEntity*, int32, int32, float, float,
    /// float, bool, bool)` (0x661DC0, `ret 0x20`)
    fn seek_entity_xy_offset_ctor(t: GPtr<u8>, e: GPtr<u8>, a: i32, b: i32, r: u32, d1: u32, d2: u32, c1: u32, c2: u32) -> GPtr<u8> = thiscall 0x661DC0 => unported;
    /// `CEntitySeekPosCalculatorXYOffset::CEntitySeekPosCalculatorXYOffset(const CVector&)` (0x65DE60, `ret 4`)
    fn seek_pos_calc_xy_offset_ctor(t: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x65DE60 => unported;
    /// `CTaskComplexSmartFleeEntity::Constructor(CEntity*, bool, float, int32, int32, float)` (0x65C430, `ret 0x18`)
    fn smart_flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, f: u32) -> GPtr<u8> = thiscall 0x65C430 => unported;
    /// `CTaskComplexTrackEntity::Constructor(CEntity*, CVector (by value), bool, int32, float, float, bool)`
    /// (0x65F3B0, `ret 0x24`)
    fn track_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, vx: u32, vy: u32, vz: u32, a: u32, b: i32, f1: u32, f2: u32, c: u32) -> GPtr<u8> = thiscall 0x65F3B0 => unported;
    /// `CTaskComplexLeaveCar` constructor `(CVehicle*, int32, int32, bool, bool)` (0x63B8C0, `ret 0x14`)
    fn leave_car_ctor(t: GPtr<u8>, v: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;

    // ---- events ----
    /// `CEventLeanOnVehicle::Constructor(CVehicle*, int32 time)` (0x65DAF0, `ret 8`)
    fn event_lean_on_vehicle_ctor(e: GPtr<u8>, v: GPtr<u8>, time: i32) -> GPtr<u8> = thiscall 0x65DAF0 => unported;
    /// `CEventGroupEvent::CEventGroupEvent(CPed*, CEvent*)` (0x4ADFD0, `ret 8`)
    fn event_group_event_ctor(e: GPtr<u8>, ped: GPtr<u8>, ev: GPtr<u8>) -> GPtr<u8> = thiscall 0x4ADFD0 => unported;
    /// `CEventGroupEvent::~CEventGroupEvent` (0x4AE070)
    fn event_group_event_dtor(e: GPtr<u8>) = thiscall 0x4AE070 => unported;
    /// `CEventScriptCommand::Constructor(int32 primaryTaskIndex, CTask*, bool)` (0x4B0A00, `ret 0xC`)
    fn event_script_command_ctor(e: GPtr<u8>, idx: i32, t: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand` complete destructor (0x4B0A50)
    fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CEventCreatePartnerTask` constructor `(int32 type, CPed* partner, bool leader, float)` (0x5F6190, `ret 0x10`)
    fn event_create_partner_task_ctor(e: GPtr<u8>, ty: i32, partner: GPtr<u8>, lead: u32, f: u32) -> GPtr<u8> = thiscall 0x5F6190 => unported;
    /// `CEventCreatePartnerTask` complete destructor (0x5F6200)
    fn event_create_partner_task_dtor(e: GPtr<u8>) = thiscall 0x5F6200 => unported;
    /// `CEventPassObject::Constructor(CPed* giver, bool)` (0x65DC70, `ret 8`)
    fn event_pass_object_ctor(e: GPtr<u8>, giver: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x65DC70 => unported;
    /// `CEventPassObject` complete destructor (0x65DCF0)
    fn event_pass_object_dtor(e: GPtr<u8>) = thiscall 0x65DCF0 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(eg: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470, `ret 4`)
    fn group_intelligence_add_event(gi: GPtr<u8>, e: GPtr<u8>) = thiscall 0x5F7470 => unported;
    /// `CPedGroupIntelligence::SetDefaultTaskAllocatorType(int32)` (0x5FBB70, `ret 4` through 0x5FB280)
    fn group_intelligence_set_default_task_allocator_type(gi: GPtr<u8>, ty: i32) = thiscall 0x5FBB70 => unported;

    // ---- groups ----
    /// `CPedGroupMembership::GetMember(int32)` (0x5F69B0, `ret 4`)
    fn membership_get_member(m: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x5F69B0 => unported;
    /// `CPedGroupMembership::GetLeader()` (0x5F69A0)
    fn membership_get_leader(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F69A0 => unported;
    /// `CPedGroupMembership::IsMember(const CPed*)` (0x5F6A10, `ret 4`)
    fn membership_is_member(m: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x5F6A10 => unported;
    /// `CPedGroupMembership::CountMembers()` (0x5F6A50)
    fn membership_count_members(m: GPtr<u8>) -> i32 = thiscall 0x5F6A50 => unported;
    /// `CPedGroupMembership::CountMembersExcludingLeader()` (0x5F6AA0)
    fn membership_count_members_excluding_leader(m: GPtr<u8>) -> i32 = thiscall 0x5F6AA0 => unported;
    /// `CPedGroupMembership::AddFollower(CPed*)` (0x5F8020, `ret 4`)
    fn membership_add_follower(m: GPtr<u8>, p: GPtr<u8>) = thiscall 0x5F8020 => unported;
    /// `CPedGroupMembership::RemoveMember(CPed*)` (0x5FB210, `ret 4`)
    fn membership_remove_member(m: GPtr<u8>, p: GPtr<u8>) = thiscall 0x5FB210 => unported;
    /// `CPedGroup::Process()` (0x5FC7E0)
    fn ped_group_process(g: GPtr<u8>) = thiscall 0x5FC7E0 => unported;
    /// `CPedGroup::GetClosestGroupPed(CPed*, float* outDistSq)` (0x5FACD0, `ret 8`)
    fn ped_group_get_closest_group_ped(g: GPtr<u8>, p: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x5FACD0 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80, cdecl)
    fn ped_groups_get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroups::GetGroupId(CPedGroup*)` (0x5F7EE0, cdecl)
    fn ped_groups_get_group_id(g: GPtr<u8>) -> i32 = cdecl 0x5F7EE0 => unported;

    // ---- peds, tasks, IK, anims ----
    /// `CPed::Say(uint16, uint32, float, uint8, uint8, uint8)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, delay: u32, prob: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::IsPlayingHandSignal()` (0x5E0460)
    fn ped_is_playing_hand_signal(p: GPtr<u8>) -> u8 = thiscall 0x5E0460 => unported;
    /// `CPed::GetEntityThatThisPedIsHolding()` (0x5E02E0)
    fn ped_get_held_entity(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x5E02E0 => unported;
    /// `CPed::DropEntityThatThisPedIsHolding(bool)` (0x5E0360, `ret 4`)
    fn ped_drop_held_entity(p: GPtr<u8>, b: u32) = thiscall 0x5E0360 => unported;
    /// `CPed::SetMoveAnimSpeed(CAnimBlendAssociation*)` (0x5DEC10, `ret 4`)
    fn ped_set_move_anim_speed(p: GPtr<u8>, a: GPtr<u8>) = thiscall 0x5DEC10 => unported;
    /// `CEntity::IsVisible()` (0x536BC0)
    fn entity_is_visible(e: GPtr<u8>) -> u8 = thiscall 0x536BC0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CPedIntelligence::GetMoveStateFromGoToTask()` (0x601D70)
    fn intel_get_move_state_from_goto_task(i: GPtr<u8>) -> i32 = thiscall 0x601D70 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn tm_get_task_secondary(tm: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskManager::SetTask(CTask*, int32, bool)` (0x681AF0, `ret 0xC`)
    fn tm_set_task(tm: GPtr<u8>, t: GPtr<u8>, i: i32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CTaskManager::SetTaskSecondary(CTask*, int32)` (0x681B60, `ret 8`)
    fn tm_set_task_secondary(tm: GPtr<u8>, t: GPtr<u8>, i: i32) = thiscall 0x681B60 => unported;
    /// `CTaskComplexWander::GetWanderTaskByPedType(CPed*)` (0x673D00, cdecl)
    fn wander_task_by_ped_type(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x673D00 => unported;
    /// `CTaskComplexWander::GetDistSqOfClosestPathNodeToPed(CPed*)` (0x669FF0, `ret 4`, float in ST0)
    fn wander_dist_sq_of_closest_path_node(t: GPtr<u8>, p: GPtr<u8>) -> f64 = thiscall 0x669FF0 => unported;
    /// `CTaskComplexWanderGang::CanJoinGang()` (0x669FB0; catalogue `sub_669fb0`)
    fn wander_gang_can_join_gang(t: GPtr<u8>) -> u8 = thiscall 0x669FB0 => unported;
    /// `CTaskSimpleHoldEntity::PlayAnim(AnimationId, AssocGroupId)` (0x691720, `ret 8`)
    fn hold_entity_play_anim(t: GPtr<u8>, anim: i32, group: u32) = thiscall 0x691720 => unported;
    /// `CTaskSimpleGoToPoint::UpdatePoint(const CVector&, float, bool)` (0x645700, `ret 0xC`)
    fn goto_point_update_point(t: GPtr<u8>, p: GPtr<u8>, r: u32, b: u32) = thiscall 0x645700 => unported;
    /// `CTaskTimer::IsOutOfTime()` (0x420E30)
    fn task_timer_is_out_of_time(t: GPtr<u8>) -> u8 = thiscall 0x420E30 => unported;
    /// `IKChainManager_c::LookAt(name, CPed*, CEntity*, int32 time, int32 bone, RwV3d*, bool, float, int32, int32, bool)`
    /// (0x618970, `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: u32, b: u32, speed: u32, blend: i32, prio: i32, b2: u32) = thiscall 0x618970 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::AbortLookAt(CPed*, int32 blendOutTime)` (0x618280, `ret 8`)
    fn ik_abort_look_at(m: GPtr<u8>, ped: GPtr<u8>, t: i32) = thiscall 0x618280 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0, cdecl)
    fn clump_get_association(clump: GPtr<u8>, anim: i32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990)
    fn anim_get_block_index(name: u32) -> i32 = cdecl 0x4D3990 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0)
    fn anim_add_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    fn anim_remove_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CStreaming::RequestModel(int32 id, int32 flags)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `FxManager_c::CreateFxSystem(const char*, RwV3d*, RwMatrix*, bool)` (0x4A9BE0, `ret 0x10`)
    fn fx_create_fx_system(m: GPtr<u8>, name: u32, pos: GPtr<u8>, mat: u32, b: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxSystem_c::AttachToBone(CEntity*, int32)` (0x4AA400, `ret 8`)
    fn fx_attach_to_bone(fx: GPtr<u8>, e: GPtr<u8>, bone: i32) = thiscall 0x4AA400 => unported;
    /// `FxSystem_c::PlayAndKill()` (0x4AA3D0)
    fn fx_play_and_kill(fx: GPtr<u8>) = thiscall 0x4AA3D0 => unported;

    // ---- world, vehicles, maths ----
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn world_find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CVehicle::GetSpareHasslePosId()` (0x6D3AE0)
    fn vehicle_get_spare_hassle_pos_id(v: GPtr<u8>) -> i32 = thiscall 0x6D3AE0 => unported;
    /// `CVehicle::SetHasslePosId(int32, bool)` (0x6D3B30, `ret 8`)
    fn vehicle_set_hassle_pos_id(v: GPtr<u8>, id: i32, b: u32) = thiscall 0x6D3B30 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn general_random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, float in ST0)
    fn general_radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, float in ST0)
    fn general_limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `operator+(const CVector&, const CVector&)` (0x40FE30, cdecl, returns through `out`)
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x40FE30 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, cdecl, returns through `out`)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x40FE60 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, cdecl, returns through `out`)
    fn vector_scale(out: GPtr<u8>, v: GPtr<u8>, f: u32) = cdecl 0x40FEC0 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00, `ret 4`)
    fn vector_add_assign(v: GPtr<u8>, o: GPtr<u8>) = thiscall 0x411A00 => unported;
    /// `CVector::SquaredMagnitude()` (0x406DA0, float in ST0)
    fn vector_squared_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `DotProduct(const CVector&, const CVector&)` (0x40FDB0, cdecl, float in ST0)
    fn vector_dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `std::max<float>(float, float)` (0x420800, cdecl; the loaded argument in ST0)
    fn float_max(a: u32, b: u32) -> f64 = cdecl 0x420800 => unported;
    /// `approxEqual(float, float, float)` (0x4EEA80, cdecl, EAX)
    fn approx_equal(a: u32, b: u32, eps: u32) -> u32 = cdecl 0x4EEA80 => unported;
}

/// The "gangs" anim block handling shared by the gang tasks (flag byte at `this + off`): without the flag, when
/// `ShouldLoadGangAnims()`: add a reference if the block is loaded (and set the flag), else request it; with the
/// flag, when `!ShouldLoadGangAnims()`: drop the reference and clear the flag.
fn gang_anims_stream(this: GPtr<u8>, off: isize) {
    if rd::<u8>(this, off) == 0 {
        if should_load_gang_anims() != 0 {
            let idx = anim_get_block_index(STR_GANGS);
            if g::<u8>((idx as u32).wrapping_shl(5).wrapping_add(ANIM_BLOCK_LOADED) as usize).read() == 1 {
                anim_add_block_ref(idx);
                wr::<u8>(this, off, 1);
            } else {
                streaming_request_model(idx.wrapping_add(0x63E7), 8);
            }
        }
    } else if should_load_gang_anims() == 0 {
        anim_remove_block_ref(anim_get_block_index(STR_GANGS));
        wr::<u8>(this, off, 0);
    }
}

/// The smoke-exhale effect (timer expired): with a clump (+0x18) and a non-null frame matrix (`[clump+4] +
/// 0x10`), `g_fxMan.CreateFxSystem([0x86FBF0], &(0, 0.1, 0), matrix, false)`, attached to bone 5 and played.
/// Returns false when the clump or matrix check failed (the caller then keeps its timer flag).
fn smoke_exhale(ped: GPtr<u8>) -> bool {
    let clump = rd::<GPtr<u8>>(ped, 0x18);
    if clump.is_null() {
        return false;
    }
    let mat = rd::<u32>(clump, 4).wrapping_add(0x10);
    if mat == 0 {
        return false;
    }
    let mut pos = [0u32, 0x3DCC_CCCD, 0];
    let fx = fx_create_fx_system(g(FX_MAN), 0x86FBF0, slot(&mut pos), mat, 0);
    if !fx.is_null() {
        fx_attach_to_bone(fx, ped, 5);
        fx_play_and_kill(fx);
    }
    true
}

/// The four partial gang anims of a holding ped (0x12B DRNKBR_PRTL, 0x12C SMKCIG_PRTL, 0x12D DRNKBR_PRTL_F,
/// 0x12E SMKCIG_PRTL_F), the clump re-read for each lookup.
fn held_anims(ped: GPtr<u8>) -> [GPtr<u8>; 4] {
    let a0 = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), 0x12B);
    let a1 = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), 0x12C);
    let a2 = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), 0x12D);
    let a3 = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), 0x12E);
    [a0, a1, a2, a3]
}

/// One of the partial anims plays: stop looking (`AbortLookAt(ped, 250)` if looking), set ped +0x474 bit 22,
/// and start the exhale timer at `this + toff` (now, 2700, started) when it is not started and a smoking anim
/// (0x12C, else 0x12E) is before 0.5 [0x858B8C] (`test ah, 5; jnp`).
fn held_anims_playing(this: GPtr<u8>, ped: GPtr<u8>, a: &[GPtr<u8>; 4], toff: isize) {
    if ik_is_looking(g(IK_CHAIN_MAN), ped) != 0 {
        ik_abort_look_at(g(IK_CHAIN_MAN), ped, 0xFA);
    }
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) | 0x40_0000);
    let early = |p: GPtr<u8>| !x(rd::<f32>(p, 0x20)).fcom(x(c(0x858B8C))).test_parity(5);
    let start = (!a[1].is_null() && early(a[1])) || (!a[3].is_null() && early(a[3]));
    if start && rd::<u8>(this, toff + 8) == 0 {
        wr::<u32>(this, toff, g::<u32>(TIME_MS).read());
        wr::<u32>(this, toff + 4, 0xA8C);
        wr::<u8>(this, toff + 8, 1);
    }
}

/// The hold-anim branch both gang tasks take when `GetRandomNumberInRange(0, 500) != 200`:
/// `GetRandomNumberInRange(0, 100) == 50` and a 0x133 (`CTaskSimpleHoldEntity`) active ->
/// `PlayAnim(GetRandomGangAmbientAnim(ped, held), 0x34)`.
fn play_random_hold_anim(ped: GPtr<u8>, held: GPtr<u8>) {
    if general_random_int_in_range(0, 0x64) == 0x32 {
        let h = tm_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x133);
        if !h.is_null() {
            let anim = get_random_gang_ambient_anim(ped, held);
            hold_entity_play_anim(h, anim, 0x34);
        }
    }
}

/// The pass-object tail of both gang tasks: a `CEventPassObject(ped, false)` (a stack object in the original)
/// added to `to`'s event group, then a new `CTaskComplexPassObject(to, true)` (null when the pool is full).
fn pass_object_to(ped: GPtr<u8>, to: GPtr<u8>) -> GPtr<u8> {
    let mut ev = [0u32; 5];
    let evp = slot(&mut ev);
    event_pass_object_ctor(evp, ped, 0);
    event_group_add(rd::<GPtr<u8>>(to, 0x47C).byte_add(0x68), evp, 0);
    let mem = task_new(0x30);
    let t = if mem.is_null() {
        GPtr::NULL
    } else {
        pass_object_ctor(mem, to, 1)
    };
    event_pass_object_dtor(evp);
    t
}

/// The look-at started by both gang tasks: `LookAt(name, ped, target, _ftol((float)time), 5, null, true, 0.15f,
/// 500, 3, false)` (the time goes through a float stack slot in the original).
fn gang_look_at(name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32) {
    let t = ftol32(x(f32_of(xi(time))));
    ik_look_at(g(IK_CHAIN_MAN), name, ped, target, t, 5, 0, 1, 0x3E19_999A, 0x1F4, 3, 0);
}

// ======================================================================================================
// CTaskComplexGangLeader
// ======================================================================================================

/// `CTaskComplexGangLeader::Destructor` (0x65DF30). vtable 0x86F8FC; with the anims referenced (+0x34) the
/// "gangs" block reference is dropped and the flag cleared; then the `CTaskComplex` destructor.
#[hook(addr = 0x65DF30, conv = thiscall, name = "CTaskComplexGangLeader::Destructor", expect = "6A FF 68 58 43 84 00")]
pub fn gang_leader_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86F8FC);
    if rd::<u8>(this, 0x34) != 0 {
        anim_remove_block_ref(anim_get_block_index(STR_GANGS));
        wr::<u8>(this, 0x34, 0);
    }
    task_complex_dtor(this);
}

/// `CTaskComplexGangLeader::ScanForStuff(CPed*)` (0x65E200, `ret 4`). With the scan timer (+0x1C) started and
/// not yet due: nothing. Else `p = _ftol((rand() & 0xFFFF) * 3.0517578e-05f * 100.0f)` (`rand()` 0x65E265).
///
/// p < 5: for each of the 16 vehicles of the ped's vehicle scanner (intel +0xE0) that is an automobile (+0x594 ==
/// 0) with status 4 (+0x36 & 0xF8 == 0x20), no gang leaning on it (+0x42E bit 0), not a mission vehicle (+0x4A4 !=
/// 2), squared distance < 300 [0x858FD8] and |dz| < 5 [0x858C80]: `GetRandomNumberInRange(10000, 25000)`, then
/// `AddEvent(CEventGroupEvent(ped, new CEventLeanOnVehicle(veh, time)))` on the group intelligence and the timer
/// restarted at 60000. The original does not stop after the first vehicle (no `break`; kept).
///
/// p == 20: the first of the 16 scanned peds (intel +0x130) created by the game (+0x484 == 1), of the ped's type,
/// not in this group, not in a vehicle, not in the player's group, and whose wander test passes. The wander test
/// looks at the *leader's* 0x390 task (`FindActiveTaskByType` on `ped`, not the scanned ped; kept): a gang wander
/// (`GetWanderType() == 4`) that cannot join fails. For that ped: without a group and with fewer than 7 followers,
/// a `CEventScriptCommand(3, new CTaskComplexBeInGroup(groupId, false), false)` added to *its* event group,
/// `AddFollower`, `CPedGroup::Process()`. Then the closest member (squared distance, start 1e9) with 16 <= d <= 100
/// gets a partner pair: `GetRandomNumberInRange(0, 7)`, `CEventCreatePartnerTask(type, member, true, 0.5f)` to the
/// scanned ped and `(type, scanned, false, 0.5f)` to the member. The timer restarts at 10000.
#[hook(addr = 0x65E200, conv = thiscall, name = "CTaskComplexGangLeader::ScanForStuff", expect = "6A FF 68 D6 43 84 00")]
pub fn gang_leader_scan_for_stuff(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(this, 0x24) != 0 && !timer_due(this, 0x1C) {
        return;
    }
    let p = rand_scaled(0x65E265, 0x858628);
    if p < 5 {
        let intel = rd::<GPtr<u8>>(ped, 0x47C);
        for i in 0..16isize {
            let v = rd::<GPtr<u8>>(intel, 0xE0 + i * 4);
            if v.is_null()
                || rd::<i32>(v, 0x594) != 0
                || rd::<u8>(v, 0x36) & 0xF8 != 0x20
                || rd::<u8>(v, 0x42E) & 1 != 0
                || rd::<u8>(v, 0x4A4) == 2
            {
                continue;
            }
            let pp = ent_pos(ped);
            let vp = ent_pos(v);
            let dz = x(rd::<f32>(vp, 8)).fsub_m(rd::<f32>(pp, 8));
            let dy = x(rd::<f32>(vp, 4)).fsub_m(rd::<f32>(pp, 4));
            let dx = x(rd::<f32>(vp, 0)).fsub_m(rd::<f32>(pp, 0));
            let sq = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz));
            if sq.fcom(x(c(0x858FD8))).test_parity(5) {
                continue;
            }
            let vp = ent_pos(v);
            let pp = ent_pos(ped);
            let dz = x(rd::<f32>(vp, 8)).fsub_m(rd::<f32>(pp, 8)).fabs();
            if dz.fcom(x(c(0x858C80))).test_parity(5) {
                continue;
            }
            let time = general_random_int_in_range(0x2710, 0x61A8);
            let mem = event_new(0x14);
            let ev = if mem.is_null() {
                GPtr::NULL
            } else {
                event_lean_on_vehicle_ctor(mem, v, time)
            };
            let mut ge = [0u32; 6];
            let gp = slot(&mut ge);
            event_group_event_ctor(gp, ped, ev);
            group_intelligence_add_event(rd::<GPtr<u8>>(this, 0xC).byte_add(0x30), gp);
            wr::<u32>(this, 0x1C, g::<u32>(TIME_MS).read());
            wr::<u32>(this, 0x20, 0xEA60);
            wr::<u8>(this, 0x24, 1);
            event_group_event_dtor(gp);
        }
        return;
    }
    if p != 0x14 {
        return;
    }
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    for i in 0..16isize {
        let s = rd::<GPtr<u8>>(intel, 0x130 + i * 4);
        if s.is_null() || rd::<u8>(s, 0x484) != 1 || rd::<i32>(ped, 0x598) != rd::<i32>(s, 0x598) {
            continue;
        }
        let sg = ped_groups_get_peds_group(s);
        if sg == rd::<GPtr<u8>>(this, 0xC) || rd::<u32>(s, 0x46C) & 0x100 != 0 {
            continue;
        }
        let player = world_find_player_ped(-1);
        let gid = rd::<u32>(rd::<GPtr<u8>>(player, 0x480), 0x38);
        let pm = GPtr::<u8>::from_addr(gid.wrapping_mul(0x2D4).wrapping_add(0xC09928) as usize);
        if membership_is_member(pm, s) != 0 {
            continue;
        }
        let w = tm_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x390);
        if !w.is_null() && v_wander_type(w) == 4 && wander_gang_can_join_gang(w) == 0 {
            continue;
        }
        if sg.is_null() {
            let id = ped_groups_get_group_id(rd::<GPtr<u8>>(this, 0xC));
            if membership_count_members_excluding_leader(rd::<GPtr<u8>>(this, 0xC).byte_add(8)) < 7 {
                let mem = task_new(0x28);
                let t = if mem.is_null() {
                    GPtr::NULL
                } else {
                    be_in_group_ctor(mem, id, 0)
                };
                let mut sc = [0u32; 6];
                let scp = slot(&mut sc);
                event_script_command_ctor(scp, 3, t, 0);
                event_group_add(rd::<GPtr<u8>>(s, 0x47C).byte_add(0x68), scp, 0);
                membership_add_follower(rd::<GPtr<u8>>(this, 0xC).byte_add(8), s);
                ped_group_process(rd::<GPtr<u8>>(this, 0xC));
                event_script_command_dtor(scp);
            }
        }
        let membership = rd::<GPtr<u8>>(this, 0xC).byte_add(8);
        let mut best = f32::from_bits(0x4E6E_6B28);
        let mut closest = GPtr::<u8>::NULL;
        for j in 0..8 {
            let m = membership_get_member(membership, j);
            if m.is_null() || m == s {
                continue;
            }
            let mp = ent_pos(m);
            let sp = ent_pos(s);
            let mut d = [0u32; 3];
            let dp = slot(&mut d);
            vector_sub(dp, sp, mp);
            let sq = st0(vector_squared_magnitude(dp));
            if !sq.fcom(x(best)).test_parity(5) {
                best = f32_of(sq);
                closest = m;
            }
        }
        if !x(best).fcom(x(c(0x858628))).test_parity(0x41) && !x(best).fcom(x(c(0x8599D0))).test_any(1) {
            let ty = general_random_int_in_range(0, 7);
            let mut e1 = [0u32; 7];
            let e1p = slot(&mut e1);
            event_create_partner_task_ctor(e1p, ty, closest, 1, 0x3F00_0000);
            event_group_add(rd::<GPtr<u8>>(s, 0x47C).byte_add(0x68), e1p, 0);
            let mut e2 = [0u32; 7];
            let e2p = slot(&mut e2);
            event_create_partner_task_ctor(e2p, ty, s, 0, 0x3F00_0000);
            event_group_add(rd::<GPtr<u8>>(closest, 0x47C).byte_add(0x68), e2p, 0);
            event_create_partner_task_dtor(e2p);
            event_create_partner_task_dtor(e1p);
        }
        wr::<u32>(this, 0x1C, g::<u32>(TIME_MS).read());
        wr::<u32>(this, 0x20, 0x2710);
        wr::<u8>(this, 0x24, 1);
        return;
    }
}

/// `CTaskComplexGangLeader::TryToPassObject(CPed*, CPedGroup*)` (0x65EA50, cdecl). The group's closest ped to
/// `ped` when its squared distance is below 4.0 [0x858B90] and `ped` (not the result) is not a player; else null.
/// The distance is a stack local the callee writes (the original's slot holds the pushed ECX; zeroed here).
#[hook(addr = 0x65EA50, conv = cdecl, name = "CTaskComplexGangLeader::TryToPassObject", expect = "51 8B 4C 24 0C")]
pub fn gang_leader_try_to_pass_object(ped: GPtr<u8>, group: GPtr<u8>) -> GPtr<u8> {
    let mut d = 0u32;
    let r = ped_group_get_closest_group_ped(group, ped, slot(&mut d));
    if r.is_null() || x(f32::from_bits(d)).fcom(x(c(0x858B90))).test_parity(5) || ped_is_player(ped) != 0 {
        return GPtr::NULL;
    }
    r
}

/// `CTaskComplexGangLeader::ControlSubTask(CPed*)` (0x662370, `ret 4`). Clears ped +0x474 bit 22; "gangs" anim
/// block (+0x34). `GetTaskType` is called twice: a 0x390 sub-task with the wander timer (+0x10) started and due
/// and the closest path node nearer than 2.0 [0x858CA0] (squared) -> `SetDefaultTaskAllocatorType(5)` and a new
/// `CTaskSimpleStandStill(500, false, false, 8.0f)`. Exhale timer (+0x28) started and due -> smoke effect (and
/// +0x30 = 0 when it was made). `ScanForStuff` (virtual). Invisible ped -> sub-task. Not looking and
/// `_ftol((rand() & 0xFFFF) * 3.0517578e-05f * 100.0f) > 95` (`rand()` 0x66258B): look at member
/// `GetRandomNumberInRange(0, 8)` (not the ped) for `GetRandomNumberInRange(3000, 5000)` ms. Without anims
/// (+0x34) or with move state >= 6 -> sub-task. Nothing held: no hand signal and no secondary task 4 ->
/// `r = GetRandomNumberInRange(0, 500)`: 50 < r < 56 a `CTaskSimpleRunAnim(0x34, 0x117 + _ftol((rand() & 0xFFFF) *
/// 3.0517578e-05f * 8.0f), 4.0f, false)` (`rand()` 0x662956, after the allocation), r == 100 a
/// `CTaskComplexPlayHandSignalAnim(-1, 4.0f)`, as secondary task 4 (null when the pool is full). Holding: partial
/// anims (exhale timer), `Say(0x17 drink / 0xC8 smoke, 0, 0.2f)`; no 0x4BB task: `GetRandomNumberInRange(0, 500)
/// != 200` -> hold anim; else, with no partial anim, `TryToPassObject` to an unarmed ped holding nothing:
/// `to->Say(0x18 / 0xC9, 0, 1.0f)` and the pass-object event and task (returned).
#[hook(addr = 0x662370, conv = thiscall, name = "CTaskComplexGangLeader::ControlSubTask", expect = "6A FF 64 A1 00 00 00 00")]
pub fn gang_leader_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & 0xFFBF_FFFF);
    gang_anims_stream(this, 0x34);
    v_task_type(sub());
    if v_task_type(sub()) == 0x390
        && rd::<u8>(this, 0x18) != 0
        && timer_due(this, 0x10)
        && !st0(wander_dist_sq_of_closest_path_node(sub(), ped))
            .fcom(x(c(0x858CA0)))
            .test_parity(5)
    {
        group_intelligence_set_default_task_allocator_type(rd::<GPtr<u8>>(this, 0xC).byte_add(0x30), 5);
        let mem = task_new(0x20);
        if mem.is_null() {
            return GPtr::NULL;
        }
        return stand_still_ctor(mem, 0x1F4, 0, 0, 0x4100_0000);
    }
    if rd::<u8>(this, 0x30) != 0 && timer_due(this, 0x28) && smoke_exhale(ped) {
        wr::<u8>(this, 0x30, 0);
    }
    v_scan_for_stuff(this, ped);
    if entity_is_visible(ped) == 0 {
        return sub();
    }
    if ik_is_looking(g(IK_CHAIN_MAN), ped) == 0 && rand_scaled(0x66258B, 0x858628) > 0x5F {
        let time = general_random_int_in_range(0xBB8, 0x1388);
        let idx = general_random_int_in_range(0, 8);
        let m = membership_get_member(rd::<GPtr<u8>>(this, 0xC).byte_add(8), idx);
        if !m.is_null() && m != ped {
            gang_look_at(0x86FBE0, ped, m, time);
        }
    }
    if rd::<u8>(this, 0x34) == 0 || rd::<i32>(ped, 0x534) >= 6 {
        return sub();
    }
    let held = ped_get_held_entity(ped);
    if held.is_null() {
        if ped_is_playing_hand_signal(ped) != 0
            || !tm_get_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 4).is_null()
        {
            return sub();
        }
        let r = general_random_int_in_range(0, 0x1F4);
        let t = if r > 0x32 && r < 0x38 {
            let mem = task_new(0x20);
            if mem.is_null() {
                GPtr::NULL
            } else {
                let i = rand_scaled(0x662956, 0x859000);
                run_anim_ctor(mem, 0x34, i.wrapping_add(0x117), 0x4080_0000, 0)
            }
        } else if r == 0x64 {
            let mem = task_new(0x18);
            if mem.is_null() {
                GPtr::NULL
            } else {
                play_hand_signal_anim_ctor(mem, -1, 0x4080_0000)
            }
        } else {
            return sub();
        };
        tm_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4);
        return sub();
    }
    let a = held_anims(ped);
    let any = a.iter().any(|p| !p.is_null());
    if any {
        held_anims_playing(this, ped, &a, 0x28);
    }
    let mi = i32::from(rd::<i16>(held, 0x22));
    if mi == i32::from(g::<u16>(MI_GANG_DRINK).read()) {
        ped_say(ped, 0x17, 0, 0x3E4C_CCCD, 0, 0, 0);
    } else if mi == i32::from(g::<u16>(MI_GANG_SMOKE).read()) {
        ped_say(ped, 0xC8, 0, 0x3E4C_CCCD, 0, 0, 0);
    }
    if !tm_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x4BB).is_null() {
        return sub();
    }
    if general_random_int_in_range(0, 0x1F4) != 0xC8 {
        play_random_hold_anim(ped, held);
        return sub();
    }
    if any {
        return sub();
    }
    let to = try_to_pass_object_orig(ped, rd::<GPtr<u8>>(this, 0xC));
    if to.is_null() || !ped_get_held_entity(to).is_null() || current_weapon_type(to) != 0 {
        return sub();
    }
    let mi = i32::from(rd::<i16>(held, 0x22));
    if mi == i32::from(g::<u16>(MI_GANG_DRINK).read()) {
        ped_say(to, 0x18, 0, 0x3F80_0000, 0, 0, 0);
    } else if mi == i32::from(g::<u16>(MI_GANG_SMOKE).read()) {
        ped_say(to, 0xC9, 0, 0x3F80_0000, 0, 0, 0);
    }
    pass_object_to(ped, to)
}

// ======================================================================================================
// CTaskComplexGangFollower
// ======================================================================================================

/// `Say` ids of the follower's hand-signal chatter by `GetRandomNumberInRange(0, 10)` (jump tables 0x6634A0 /
/// 0x6634BC with index bytes 0,0,0,1,1,1,1,1,2,3; both identical).
const FOLLOWER_SAY: [u32; 10] = [0x2D, 0x2D, 0x2D, 0xA6, 0xA6, 0xA6, 0xA6, 0xA6, 0x18, 0xC9];

/// `GetRandomNumberInRange(0, 10)`, then `ped->Say(id, 0, 1.0f)` by [`FOLLOWER_SAY`] (`cmp eax, 9; ja`).
fn follower_chatter(ped: GPtr<u8>) {
    let r = general_random_int_in_range(0, 0xA) as u32;
    if r <= 9 {
        ped_say(ped, FOLLOWER_SAY[r as usize], 0, 0x3F80_0000, 0, 0, 0);
    }
}

/// `CTaskComplexGangFollower::ControlSubTask(CPed*)` (0x662A10, `ret 4`). Clears ped +0x474 bit 22. No leader:
/// `MakeAbortable(ped, 1, null)` true -> ped +0x478 bit 16 cleared, null; else the sub-task. A 0x2C5 sub-task ->
/// sub-task. Then (idle anim 0 of the clump, leader running = `GetMoveStateFromGoToTask` in {4, 6, 7},
/// `CalculateOffsetPosition(+0x20)`, d = leader - ped flattened (f32), d2 = dy² + dx²):
///
/// Sub-task 0x38B with +0x3D bit 2 (`v = intel +0x274 > 8` read first): the offset is copied to the seek task
/// (+0x44) and its +0x20 = 2.0f; with an active 0x384 and no 0x38A: target = leader position + offset, delta =
/// target - ped (z = 0); when the leader runs and the ped is behind the target along the leader's forward axis
/// (`ped·fwd + -(target·fwd) < 0`): within `(1 + r)²` (r = 0x384 +0x18) the target moves 2 forward and the idle
/// anim speed becomes `max(0.85f, speed - 0.0125f)`, beyond it `SetMoveAnimSpeed` and the speed is set back to
/// the old one (approxEqual 0.013f) or the old ± 0.0125f towards the new one; either sets the "speed" flag
/// (only with an idle anim). `UpdatePoint(target, 0.5f [0x86FC78], false)` on the 0x384. Then with v and d2 < 64.0
/// [0x86F82C] and `MakeAbortable(ped, 1, null)`: +0x20 = (dx, dy, 0) and a new `CTaskSimpleStandStill(500, false,
/// false, 8.0f)`.
///
/// Then ped +0x478 bit 16 = the speed flag; "gangs" anim block (+0x3D bit 0); a group without +0x2D0 and +0x3D
/// bit 3 clear, more than 3 members and `GetRandomNumberInRange(0, 2000) == 500` sets +0x3D bit 1; exhale timer
/// (+0x40, `CTaskTimer::IsOutOfTime`); invisible -> sub-task; not looking and `GetRandomNumberInRange(0, 100) >
/// 95`: look at member `GetRandomNumberInRange(0, 8)` (the leader instead of the ped itself). Without anims or
/// move state >= 6 -> sub-task. Holding: as the leader (exhale timer +0x40), but on the pass the speech is
/// `GetRandomNumberInRange(0, 500) < 250 ? to->Say(0x18 / 0xC9, 0, 1.0f) : ped->Say(0x17 / 0xC8, 1500, 1.0f)`.
/// Not holding: as the leader's hand signals; after the RunAnim, walking peds (+0x534 == 1) chatter; the hand
/// signal anim is created only for walking peds, then chatter.
#[hook(addr = 0x662A10, conv = thiscall, name = "CTaskComplexGangFollower::ControlSubTask", expect = "64 A1 00 00 00 00")]
pub fn gang_follower_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & 0xFFBF_FFFF);
    if rd::<GPtr<u8>>(this, 0x10).is_null() {
        if v_make_abortable(sub(), ped, 1, GPtr::NULL) != 0 {
            wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) & 0xFFFE_FFFF);
            return GPtr::NULL;
        }
        return sub();
    }
    if v_task_type(sub()) == 0x2C5 {
        return sub();
    }
    let idle = clump_get_association(rd::<GPtr<u8>>(ped, 0x18), 0);
    let ms = intel_get_move_state_from_goto_task(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), 0x47C));
    let running = ms == 4 || ms == 6 || ms == 7;
    let off = this.byte_add(0x20);
    gang_follower_calculate_offset_position(this, off);
    let pp = ent_pos(ped);
    let lp = ent_pos(rd::<GPtr<u8>>(this, 0x10));
    let dy = f32_of(x(rd::<f32>(lp, 4)).fsub_m(rd::<f32>(pp, 4)));
    let dx = f32_of(x(rd::<f32>(lp, 0)).fsub_m(rd::<f32>(pp, 0)));
    let d2 = f32_of(x(dy).fmul_m(dy).fadd(x(dx).fmul_m(dx)));
    let mut flag = false;
    if v_task_type(sub()) == 0x38B {
        let v274 = rd::<i32>(rd::<GPtr<u8>>(ped, 0x47C), 0x274) > 8;
        if rd::<u8>(this, 0x3D) & 4 != 0 {
            let s = sub();
            wr::<u32>(s, 0x44, rd::<u32>(off, 0));
            wr::<u32>(s, 0x48, rd::<u32>(off, 4));
            wr::<u32>(s, 0x4C, rd::<u32>(off, 8));
            wr::<u32>(s, 0x20, 0x4000_0000);
            let t384 = tm_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x384);
            let t38a = tm_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x38A);
            if !t384.is_null() && t38a.is_null() {
                let mut target = [0u32; 3];
                let tp = slot(&mut target);
                vector_add(tp, ent_pos(rd::<GPtr<u8>>(this, 0x10)), off);
                let mut delta = [0u32; 3];
                let dp = slot(&mut delta);
                vector_sub(dp, tp, ent_pos(ped));
                wr::<u32>(dp, 8, 0);
                if running {
                    let lm = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), 0x14);
                    let t = |i: isize| rd::<f32>(tp, i * 4);
                    let neg = f32_of(
                        x(t(1))
                            .fmul_m(rd::<f32>(lm, 0x14))
                            .fadd(x(t(2)).fmul_m(rd::<f32>(lm, 0x18)))
                            .fadd(x(t(0)).fmul_m(rd::<f32>(lm, 0x10)))
                            .fchs(),
                    );
                    let along = st0(vector_dot_product(ent_pos(ped), lm.byte_add(0x10))).fadd_m(neg);
                    if !along.fcom(x(c(0x858B50))).test_parity(5) {
                        let m = st0(vector_squared_magnitude(dp));
                        let r = x(rd::<f32>(t384, 0x18));
                        let lim = x(c(0x858624)).fadd(r).fmul(r.fadd_m(c(0x858624)));
                        if !m.fcom(lim).test_parity(0x41) {
                            let lm = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), 0x14);
                            let mut fwd = [0u32; 3];
                            let fp = slot(&mut fwd);
                            vector_scale(fp, lm.byte_add(0x10), 0x4000_0000);
                            vector_add_assign(tp, fp);
                            if !idle.is_null() {
                                let s = f32_of(x(rd::<f32>(idle, 0x24)).fsub_m(c(0x85A5C4)));
                                let v = st0(float_max(0x3F59_999A, s.to_bits()));
                                wr::<f32>(idle, 0x24, f32_of(v));
                                flag = true;
                            }
                        } else if !idle.is_null() {
                            let old = rd::<f32>(idle, 0x24);
                            ped_set_move_anim_speed(ped, idle);
                            let new = rd::<f32>(idle, 0x24);
                            if approx_equal(old.to_bits(), new.to_bits(), 0x3C54_FDF4) & 0xFF != 0 {
                                wr::<f32>(idle, 0x24, old);
                            } else if x(old).fcom(x(new)).test_any(0x41) {
                                wr::<f32>(idle, 0x24, f32_of(x(old).fadd_m(c(0x85A5C4))));
                            } else {
                                wr::<f32>(idle, 0x24, f32_of(x(old).fsub_m(c(0x85A5C4))));
                            }
                            flag = true;
                        }
                    }
                }
                goto_point_update_point(t384, tp, g::<u32>(0x86FC78).read(), 0);
            }
            if v274
                && !x(d2).fcom(x(c(0x86F82C))).test_parity(5)
                && v_make_abortable(sub(), ped, 1, GPtr::NULL) != 0
            {
                wr::<f32>(off, 0, dx);
                wr::<f32>(off, 4, dy);
                wr::<u32>(off, 8, 0);
                let mem = task_new(0x20);
                if mem.is_null() {
                    return GPtr::NULL;
                }
                return stand_still_ctor(mem, 0x1F4, 0, 0, 0x4100_0000);
            }
        }
    }
    let f = rd::<u32>(ped, 0x478);
    wr::<u32>(ped, 0x478, (f & !0x1_0000) | (u32::from(flag) << 16));
    if rd::<u8>(this, 0x3D) & 1 == 0 {
        if should_load_gang_anims() != 0 {
            let idx = anim_get_block_index(STR_GANGS);
            if g::<u8>((idx as u32).wrapping_shl(5).wrapping_add(ANIM_BLOCK_LOADED) as usize).read() == 1 {
                anim_add_block_ref(idx);
                wr::<u8>(this, 0x3D, rd::<u8>(this, 0x3D) | 1);
            } else {
                streaming_request_model(idx.wrapping_add(0x63E7), 8);
            }
        }
    } else if should_load_gang_anims() == 0 {
        anim_remove_block_ref(anim_get_block_index(STR_GANGS));
        wr::<u8>(this, 0x3D, rd::<u8>(this, 0x3D) & 0xFE);
    }
    let grp = rd::<GPtr<u8>>(this, 0xC);
    if rd::<u8>(grp, 0x2D0) == 0
        && rd::<u8>(this, 0x3D) & 8 == 0
        && membership_count_members(grp.byte_add(8)) > 3
        && general_random_int_in_range(0, 0x7D0) == 0x1F4
    {
        wr::<u8>(this, 0x3D, rd::<u8>(this, 0x3D) | 2);
    }
    if rd::<u8>(this, 0x48) != 0 && task_timer_is_out_of_time(this.byte_add(0x40)) != 0 && smoke_exhale(ped) {
        wr::<u8>(this, 0x48, 0);
    }
    if entity_is_visible(ped) == 0 {
        return sub();
    }
    if ik_is_looking(g(IK_CHAIN_MAN), ped) == 0 && general_random_int_in_range(0, 0x64) > 0x5F {
        let time = general_random_int_in_range(0xBB8, 0x1388);
        let idx = ftol32(xi(general_random_int_in_range(0, 8)));
        let mut m = membership_get_member(rd::<GPtr<u8>>(this, 0xC).byte_add(8), idx);
        if m == ped {
            m = rd::<GPtr<u8>>(this, 0x10);
        }
        if !m.is_null() {
            gang_look_at(0x86FBF8, ped, m, time);
        }
    }
    if rd::<u8>(this, 0x3D) & 1 == 0 || rd::<i32>(ped, 0x534) >= 6 {
        return sub();
    }
    let held = ped_get_held_entity(ped);
    if held.is_null() {
        if ped_is_playing_hand_signal(ped) != 0
            || !tm_get_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 4).is_null()
        {
            return sub();
        }
        let r = general_random_int_in_range(0, 0x1F4);
        if r > 0x32 && r < 0x38 {
            let mem = task_new(0x20);
            let t = if mem.is_null() {
                GPtr::NULL
            } else {
                let i = rand_scaled(0x66333C, 0x859000);
                run_anim_ctor(mem, 0x34, i.wrapping_add(0x117), 0x4080_0000, 0)
            };
            tm_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4);
            if rd::<i32>(ped, 0x534) == 1 {
                follower_chatter(ped);
            }
        } else if r == 0x64 && rd::<i32>(ped, 0x534) == 1 {
            let mem = task_new(0x18);
            let t = if mem.is_null() {
                GPtr::NULL
            } else {
                play_hand_signal_anim_ctor(mem, -1, 0x4080_0000)
            };
            tm_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4);
            follower_chatter(ped);
        }
        return sub();
    }
    let a = held_anims(ped);
    let any = a.iter().any(|p| !p.is_null());
    if any {
        held_anims_playing(this, ped, &a, 0x40);
    }
    if !tm_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x4BB).is_null() {
        return sub();
    }
    if general_random_int_in_range(0, 0x1F4) != 0xC8 {
        play_random_hold_anim(ped, held);
        return sub();
    }
    if any {
        return sub();
    }
    let to = try_to_pass_object_orig(ped, rd::<GPtr<u8>>(this, 0xC));
    if to.is_null() || !ped_get_held_entity(to).is_null() || current_weapon_type(to) != 0 {
        return sub();
    }
    let mi = i32::from(rd::<i16>(held, 0x22));
    let ids = if mi == i32::from(g::<u16>(MI_GANG_DRINK).read()) {
        Some((0x18, 0x17))
    } else if mi == i32::from(g::<u16>(MI_GANG_SMOKE).read()) {
        Some((0xC9, 0xC8))
    } else {
        None
    };
    if let Some((req, give)) = ids {
        if general_random_int_in_range(0, 0x1F4) < 0xFA {
            ped_say(to, req, 0, 0x3F80_0000, 0, 0, 0);
        } else {
            ped_say(ped, give, 0x5DC, 0x3F80_0000, 0, 0, 0);
        }
    }
    pass_object_to(ped, to)
}

/// `CTaskComplexGangFollower::CreateNextSubTask(CPed*)` (0x665E00, `ret 4`). No leader: ped +0x478 bit 16
/// cleared, null. `v = intel +0x274 > [0x86C938]` (signed). +0x3D bit 1 after a 0x4BA (signal at ped): leave the
/// group (`RemoveMember(ped)`), primary task 4 = a new `CTaskComplexWanderGang(4, GetRandomNumberInRange(0, 8),
/// 30000, true, [0x86FCB0])` (the random draw only after a successful allocation; null when the pool is full), ped
/// +0x478 bit 16 cleared, null. v, or a 0x38B sub-task while the leader's move state < 4: a player leader makes
/// the ped `Say(0x5A, 0, 0.3f)`; a new `CTaskSimpleStandStill(500, false, false, 8.0f)`. After a 0xCB / 0x1AA
/// (each a new `GetTaskType` call): +0x3D bit 1 -> `CTaskComplexSignalAtPed(leader, -1, false)`; else
/// `GetRandomNumberInRange(0, 30) == 20` and member `GetRandomNumberInRange(0, CountMembers())` (the leader
/// instead of the ped itself) -> `CTaskComplexTurnToFaceEntityOrCoord(member, [0x86FC7C], [0x86FC80])`; else
/// `CTaskSimplePause(50)`. After a 0x2C5: the virtual `CreateFirstSubTask`. Else with +0x3D bit 2:
/// `CTaskComplexSeekEntity<XYOffset>(leader, 50000, 1000, [0x86FC2C] * 0.5f, [0x86F830], [0x86FC30], false, false)`
/// whose offset (+0x44) gets `CEntitySeekPosCalculatorXYOffset(+0x20)`'s vector, +0x54 |= 8, +0x50 = 7 (written
/// through the result also when the allocation failed, as the original does); otherwise the stand-still task.
#[hook(addr = 0x665E00, conv = thiscall, name = "CTaskComplexGangFollower::CreateNextSubTask", expect = "64 A1 00 00 00 00")]
pub fn gang_follower_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    if rd::<GPtr<u8>>(this, 0x10).is_null() {
        wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) & 0xFFFE_FFFF);
        return GPtr::NULL;
    }
    let limit = g::<i32>(0x86C938).read();
    let v = rd::<i32>(rd::<GPtr<u8>>(ped, 0x47C), 0x274) > limit;
    if rd::<u8>(this, 0x3D) & 2 != 0 && v_task_type(sub()) == 0x4BA {
        membership_remove_member(rd::<GPtr<u8>>(this, 0xC).byte_add(8), ped);
        let mem = task_new(0x38);
        let t = if mem.is_null() {
            GPtr::NULL
        } else {
            let f = g::<u32>(0x86FCB0).read();
            let dir = general_random_int_in_range(0, 8);
            wander_gang_ctor(mem, 4, dir, 0x7530, 1, f)
        };
        tm_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4, 0);
        wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) & 0xFFFE_FFFF);
        return GPtr::NULL;
    }
    let stand_still = |say: bool| {
        if say {
            let l = rd::<GPtr<u8>>(this, 0x10);
            if !l.is_null() && ped_is_player(l) != 0 {
                ped_say(ped, 0x5A, 0, 0x3E99_999A, 0, 0, 0);
            }
        }
        let mem = task_new(0x20);
        if mem.is_null() {
            return GPtr::NULL;
        }
        stand_still_ctor(mem, 0x1F4, 0, 0, 0x4100_0000)
    };
    if v || (v_task_type(sub()) == 0x38B && rd::<i32>(rd::<GPtr<u8>>(this, 0x10), 0x534) < 4) {
        return stand_still(true);
    }
    if v_task_type(sub()) == 0xCB || v_task_type(sub()) == 0x1AA {
        if rd::<u8>(this, 0x3D) & 2 != 0 {
            let mem = task_new(0x18);
            if mem.is_null() {
                return GPtr::NULL;
            }
            return signal_at_ped_ctor_orig(mem, rd::<GPtr<u8>>(this, 0x10), -1, 0);
        }
        if general_random_int_in_range(0, 0x1E) == 0x14 {
            let n = membership_count_members(rd::<GPtr<u8>>(this, 0xC).byte_add(8));
            let i = general_random_int_in_range(0, n);
            let mut m = membership_get_member(rd::<GPtr<u8>>(this, 0xC).byte_add(8), i);
            if m == ped {
                m = membership_get_leader(rd::<GPtr<u8>>(this, 0xC).byte_add(8));
            }
            if !m.is_null() {
                let mem = task_new(0x28);
                if mem.is_null() {
                    return GPtr::NULL;
                }
                let b = g::<u32>(0x86FC80).read();
                let a = g::<u32>(0x86FC7C).read();
                return turn_to_face_entity_ctor(mem, m, a, b);
            }
        }
        let mem = task_new(0x18);
        if mem.is_null() {
            return GPtr::NULL;
        }
        return pause_ctor(mem, 0x32);
    }
    if v_task_type(sub()) == 0x2C5 {
        return v_create_first_sub_task(this, ped);
    }
    if rd::<u8>(this, 0x3D) & 4 == 0 {
        return stand_still(false);
    }
    let mem = task_new(0x58);
    let t = if mem.is_null() {
        GPtr::NULL
    } else {
        let c30 = g::<u32>(0x86FC30).read();
        let r = f32_of(x(c(0x86FC2C)).fmul_m(c(0x858B8C)));
        let c830 = g::<u32>(0x86F830).read();
        seek_entity_xy_offset_ctor(mem, rd::<GPtr<u8>>(this, 0x10), 0xC350, 0x3E8, r.to_bits(), c830, c30, 0, 0)
    };
    let mut calc = [0u32; 4];
    seek_pos_calc_xy_offset_ctor(slot(&mut calc), this.byte_add(0x20));
    wr::<u32>(t, 0x44, calc[1]);
    wr::<u32>(t, 0x48, calc[2]);
    wr::<u32>(t, 0x4C, calc[3]);
    wr::<u8>(t, 0x54, rd::<u8>(t, 0x54) | 8);
    wr::<u32>(t, 0x50, 7);
    t
}

// ======================================================================================================
// CTaskGangHassleVehicle
// ======================================================================================================

/// `CTaskGangHassleVehicle::Constructor(CVehicle*, int32 (unused), uint8, float, float)` (0x65FAC0, `ret 0x14`).
/// `CTaskComplex` base, vtable 0x86F9D4, +0x38 = +0x3C = 0, bytes +0x40 = +0x41 = 0, +0x18 = the byte, +0x1C and
/// +0x20 = the floats, +0x10 = vehicle, +0x14 = -1 (the second argument is not stored), +0x30 = 0, +0x34 = 0; the
/// vehicle reference is registered when non-null.
#[hook(addr = 0x65FAC0, conv = thiscall, name = "CTaskGangHassleVehicle::Constructor", expect = "6A FF 68 D8 44 84 00")]
pub fn gang_hassle_vehicle_ctor(this: GPtr<u8>, veh: GPtr<u8>, _a: u32, b: u32, f1: u32, f2: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0, 0x86F9D4);
    wr::<u32>(this, 0x38, 0);
    wr::<u32>(this, 0x3C, 0);
    wr::<u8>(this, 0x40, 0);
    wr::<u8>(this, 0x41, 0);
    wr::<u8>(this, 0x18, b as u8);
    wr::<u32>(this, 0x1C, f1);
    wr::<GPtr<u8>>(this, 0x10, veh);
    wr::<i32>(this, 0x14, -1);
    wr::<u32>(this, 0x20, f2);
    wr::<u8>(this, 0x30, 0);
    wr::<u32>(this, 0x34, 0);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(0x10));
    }
    this
}

/// `CTaskGangHassleVehicle::Destructor` (0x65FB60; island 0x4039EE pushes the SEH handler, body at 0x65FB67).
/// vtable 0x86F9D4; with a vehicle (+0x10): its hassle position (+0x14) is released when > -1
/// (`SetHasslePosId(id, false)`) and the reference cleaned up; the +0x34 reference cleaned up; the "gangs"
/// block reference (+0x30) dropped; then the `CTaskComplex` destructor.
///
/// Not hooked (INF-20): protection code reads the patched entry bytes (`adc ecx, [0x65FB62]` at 0x401A26).
pub fn gang_hassle_vehicle_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86F9D4);
    let v = rd::<GPtr<u8>>(this, 0x10);
    if !v.is_null() {
        let id = rd::<i32>(this, 0x14);
        if id > -1 {
            vehicle_set_hassle_pos_id(v, id, 0);
        }
        let v = rd::<GPtr<u8>>(this, 0x10);
        if !v.is_null() {
            entity_clean_up_old_reference(v, this.byte_add(0x10));
        }
    }
    let p = rd::<GPtr<u8>>(this, 0x34);
    if !p.is_null() {
        entity_clean_up_old_reference(p, this.byte_add(0x34));
    }
    if rd::<u8>(this, 0x30) != 0 {
        anim_remove_block_ref(anim_get_block_index(STR_GANGS));
        wr::<u8>(this, 0x30, 0);
    }
    task_complex_dtor(this);
}

/// `CTaskGangHassleVehicle::CreateNextSubTask(CPed*)` (0x65FC80, `ret 4`). Null without a vehicle or after a
/// 0x38F. Vehicle health (+0x4C0) below 250.0 [0x859F80] (`test ah, 5; jp`): a new
/// `CTaskComplexSmartFleeEntity(vehicle, false, 30.0f, [0x86F674], [0x86F678], [0xC18CF0])`. Else null after a
/// 0x4BC or 0x39D, otherwise a new `CTaskComplexTrackEntity(vehicle, +0x24 (by value), true, -1, 10.0f, 40.0f,
/// true)`.
#[hook(addr = 0x65FC80, conv = thiscall, name = "CTaskGangHassleVehicle::CreateNextSubTask", expect = "6A FF 68 46 45 84 00")]
pub fn gang_hassle_vehicle_create_next_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    if rd::<GPtr<u8>>(this, 0x10).is_null() || (!sub().is_null() && v_task_type(sub()) == 0x38F) {
        return GPtr::NULL;
    }
    let health = rd::<f32>(rd::<GPtr<u8>>(this, 0x10), 0x4C0);
    if !x(health).fcom(x(c(0x859F80))).test_parity(5) {
        let mem = task_new(0x40);
        if mem.is_null() {
            return GPtr::NULL;
        }
        let f = g::<u32>(0xC18CF0).read();
        let b = g::<u32>(0x86F678).read();
        let a = g::<u32>(0x86F674).read();
        return smart_flee_entity_ctor(mem, rd::<GPtr<u8>>(this, 0x10), 0, 0x41F0_0000, a, b, f);
    }
    if (!sub().is_null() && v_task_type(sub()) == 0x4BC) || (!sub().is_null() && v_task_type(sub()) == 0x39D) {
        return GPtr::NULL;
    }
    let mem = task_new(0x58);
    if mem.is_null() {
        return GPtr::NULL;
    }
    track_entity_ctor(
        mem,
        rd::<GPtr<u8>>(this, 0x10),
        rd::<u32>(this, 0x24),
        rd::<u32>(this, 0x28),
        rd::<u32>(this, 0x2C),
        1,
        -1,
        0x4120_0000,
        0x4220_0000,
        1,
    )
}

/// `CTaskGangHassleVehicle::GetTargetHeading(CPed*)` (0x65FDD0, `ret 4`, float in ST0). From the vehicle's
/// matrix (no null check): hassle position (+0x14) 0 / 2 -> the right vector, 1 / 3 -> minus the right vector,
/// 4 -> the forward vector, 5 -> minus the forward vector, any other -> the right vector; returns
/// `LimitRadianAngle((float)GetRadianAngleBetweenPoints(v.x, v.y, 0, 0))`.
#[hook(addr = 0x65FDD0, conv = thiscall, name = "CTaskGangHassleVehicle::GetTargetHeading", expect = "83 EC 1C 8B 41 10")]
pub fn gang_hassle_vehicle_get_target_heading(this: GPtr<u8>, _ped: GPtr<u8>) -> X87 {
    let m = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x10), 0x14);
    let neg = |v: f32| f32_of(x(v).fchs());
    let (rx, ry) = (rd::<f32>(m, 0), rd::<f32>(m, 4));
    let (fx, fy) = (rd::<f32>(m, 0x10), rd::<f32>(m, 0x14));
    let (vx, vy) = match rd::<i32>(this, 0x14) {
        1 | 3 => (neg(rx), neg(ry)),
        4 => (fx, fy),
        5 => (neg(fx), neg(fy)),
        _ => (rx, ry),
    };
    let a = f32_of(st0(general_radian_angle_between_points(vx.to_bits(), vy.to_bits(), 0, 0)));
    st0(general_limit_radian_angle(a.to_bits()))
}

/// `CTaskGangHassleVehicle::CalcTargetOffset()` (0x6641A0). +0x24..+0x2C = 0, then from the vehicle model's
/// collision bounding box (min, max) and the distance +0x20 by hassle position (+0x14): 0 -> (min.x - d,
/// max.y * 0.5), 1 -> (max.x + d, max.y * 0.5), 2 -> (min.x - d, min.y * 0.5), 3 -> (max.x + d, min.y * 0.5),
/// 4 -> y = min.y - d, 5 -> y = max.y + d; other positions keep zero.
#[hook(addr = 0x6641A0, conv = thiscall, name = "CTaskGangHassleVehicle::CalcTargetOffset", expect = "83 EC 18 8B 41 10")]
pub fn gang_hassle_vehicle_calc_target_offset(this: GPtr<u8>) {
    let v = rd::<GPtr<u8>>(this, 0x10);
    wr::<u32>(this, 0x2C, 0);
    wr::<u32>(this, 0x28, 0);
    wr::<u32>(this, 0x24, 0);
    let mi = (rd::<i16>(v, 0x22) as i32 as u32).wrapping_mul(4).wrapping_add(MODEL_INFO_PTRS);
    let col = rd::<GPtr<u8>>(g::<GPtr<u8>>(mi as usize).read(), 0x14);
    let (min_x, min_y) = (rd::<f32>(col, 0), rd::<f32>(col, 4));
    let (max_x, max_y) = (rd::<f32>(col, 0xC), rd::<f32>(col, 0x10));
    let d = rd::<f32>(this, 0x20);
    let half = |v: f32| f32_of(x(v).fmul_m(c(0x858B8C)));
    match rd::<i32>(this, 0x14) {
        0 => {
            wr::<f32>(this, 0x24, f32_of(x(min_x).fsub_m(d)));
            wr::<f32>(this, 0x28, half(max_y));
        }
        1 => {
            wr::<f32>(this, 0x24, f32_of(x(max_x).fadd_m(d)));
            wr::<f32>(this, 0x28, half(max_y));
        }
        2 => {
            wr::<f32>(this, 0x24, f32_of(x(min_x).fsub_m(d)));
            wr::<f32>(this, 0x28, half(min_y));
        }
        3 => {
            wr::<f32>(this, 0x24, f32_of(x(max_x).fadd_m(d)));
            wr::<f32>(this, 0x28, half(min_y));
        }
        4 => wr::<f32>(this, 0x28, f32_of(x(min_y).fsub_m(d))),
        5 => wr::<f32>(this, 0x28, f32_of(x(max_y).fadd_m(d))),
        _ => {}
    }
}

/// `CTaskGangHassleVehicle::CreateFirstSubTask(CPed*)` (0x664BA0, `ret 4`). Null without a vehicle. +0x34 = the
/// vehicle's driver (+0x460; stored without cleaning up an old reference, registered when non-null). Null when
/// the collision box is wider than 4.0 [0x858B90] or longer than 8.0 [0x859000] (`test ah, 0x41; je`) or no
/// hassle position is free (`GetSpareHasslePosId` -> +0x14 == -1); else the position is taken
/// (`SetHasslePosId(id, true)`), `CalcTargetOffset()`, +0x31 = 0, `ped->DropEntityThatThisPedIsHolding(true)`, the
/// timer +0x38 = now (read before the draw), +0x3C = `GetRandomNumberInRange(150000, 250000)`, +0x40 = 1. A ped in
/// a vehicle gets a new `CTaskComplexLeaveCar(vehicle, 0, 0, true, false)`; else the virtual `CreateNextSubTask`.
#[hook(addr = 0x664BA0, conv = thiscall, name = "CTaskGangHassleVehicle::CreateFirstSubTask", expect = "64 A1 00 00 00 00")]
pub fn gang_hassle_vehicle_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let v = rd::<GPtr<u8>>(this, 0x10);
    if v.is_null() {
        return GPtr::NULL;
    }
    let drv = rd::<GPtr<u8>>(v, 0x460);
    wr::<GPtr<u8>>(this, 0x34, drv);
    if !drv.is_null() {
        entity_register_reference(drv, this.byte_add(0x34));
    }
    let v = rd::<GPtr<u8>>(this, 0x10);
    let mi = (rd::<i16>(v, 0x22) as i32 as u32).wrapping_mul(4).wrapping_add(MODEL_INFO_PTRS);
    let col = rd::<GPtr<u8>>(g::<GPtr<u8>>(mi as usize).read(), 0x14);
    let (min_x, min_y) = (rd::<f32>(col, 0), rd::<f32>(col, 4));
    let (max_x, max_y) = (rd::<f32>(col, 0xC), rd::<f32>(col, 0x10));
    if !x(max_x).fsub_m(min_x).fcom(x(c(0x858B90))).test_any(0x41)
        || !x(max_y).fsub_m(min_y).fcom(x(c(0x859000))).test_any(0x41)
    {
        return GPtr::NULL;
    }
    let id = vehicle_get_spare_hassle_pos_id(v);
    wr::<i32>(this, 0x14, id);
    if id == -1 {
        return GPtr::NULL;
    }
    vehicle_set_hassle_pos_id(rd::<GPtr<u8>>(this, 0x10), id, 1);
    gang_hassle_vehicle_calc_target_offset_orig(this);
    wr::<u8>(this, 0x31, 0);
    ped_drop_held_entity(ped, 1);
    let now = g::<u32>(TIME_MS).read();
    let t = general_random_int_in_range(0x249F0, 0x3D090);
    wr::<u32>(this, 0x38, now);
    wr::<i32>(this, 0x3C, t);
    wr::<u8>(this, 0x40, 1);
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 && !rd::<GPtr<u8>>(ped, 0x58C).is_null() {
        let mem = task_new(0x34);
        if mem.is_null() {
            return GPtr::NULL;
        }
        return leave_car_ctor(mem, rd::<GPtr<u8>>(ped, 0x58C), 0, 0, 1, 0);
    }
    v_create_next_sub_task(this, ped)
}

// ======================================================================================================
// CTaskGangHasslePed / CTaskComplexSignalAtPed
// ======================================================================================================

/// `CTaskGangHasslePed::Destructor` (0x65FF60). vtable 0x86FA00; the target reference (+0xC) cleaned up; the
/// "gangs" block reference (+0x1C) dropped; then the `CTaskComplex` destructor.
#[hook(addr = 0x65FF60, conv = thiscall, name = "CTaskGangHasslePed::Destructor", expect = "6A FF 68 78 45 84 00")]
pub fn gang_hassle_ped_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86FA00);
    let t = rd::<GPtr<u8>>(this, 0xC);
    if !t.is_null() {
        entity_clean_up_old_reference(t, this.byte_add(0xC));
    }
    if rd::<u8>(this, 0x1C) != 0 {
        anim_remove_block_ref(anim_get_block_index(STR_GANGS));
        wr::<u8>(this, 0x1C, 0);
    }
    task_complex_dtor(this);
}

/// `CTaskComplexSignalAtPed::Constructor(CPed*, int32 (unused), bool)` (0x660A30, `ret 0xC`; island 0x407415
/// pushes the SEH handler, body at 0x660A37). `CTaskComplex` base, vtable 0x86FA8C, +0xC = target, +0x10 = 0 (the
/// second argument is not stored), +0x14 = the bool, +0x15 = 0; the target reference is registered when
/// non-null.
#[hook(addr = 0x660A30, conv = thiscall, name = "CTaskComplexSignalAtPed::Constructor", expect = "6A FF E9 DE 69 DA FF")]
pub fn signal_at_ped_ctor(this: GPtr<u8>, target: GPtr<u8>, _a: i32, b: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0, 0x86FA8C);
    wr::<GPtr<u8>>(this, 0xC, target);
    wr::<u32>(this, 0x10, 0);
    wr::<u8>(this, 0x14, b as u8);
    wr::<u8>(this, 0x15, 0);
    if !target.is_null() {
        entity_register_reference(target, this.byte_add(0xC));
    }
    this
}

/// `CTaskComplexSignalAtPed::Destructor` (0x660AB0). vtable 0x86FA8C; the "gangs" block reference (+0x15)
/// dropped; the target reference (+0xC) cleaned up; then the `CTaskComplex` destructor.
#[hook(addr = 0x660AB0, conv = thiscall, name = "CTaskComplexSignalAtPed::Destructor", expect = "6A FF 68 E8 46 84 00")]
pub fn signal_at_ped_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86FA8C);
    if rd::<u8>(this, 0x15) != 0 {
        anim_remove_block_ref(anim_get_block_index(STR_GANGS));
        wr::<u8>(this, 0x15, 0);
    }
    let t = rd::<GPtr<u8>>(this, 0xC);
    if !t.is_null() {
        entity_clean_up_old_reference(t, this.byte_add(0xC));
    }
    task_complex_dtor(this);
}

// ======================================================================================================
// CTaskComplexLeaveCarAndWander
// ======================================================================================================

/// `CTaskComplexLeaveCarAndWander::Constructor(CVehicle*, int32, int32, bool)` (0x63BDD0, `ret 0x10`).
/// `CTaskComplex` base, +0x10 / +0x14 = the ints, vtable 0x86E880, +0xC = vehicle, +0x18 = the bool; the vehicle
/// reference is registered when non-null.
#[hook(addr = 0x63BDD0, conv = thiscall, name = "CTaskComplexLeaveCarAndWander::Constructor", expect = "6A FF 68 F8 16 84 00")]
pub fn leave_car_and_wander_ctor(this: GPtr<u8>, veh: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0x10, a);
    wr::<u32>(this, 0x14, b);
    wr::<u32>(this, 0, 0x86E880);
    wr::<GPtr<u8>>(this, 0xC, veh);
    wr::<u8>(this, 0x18, c as u8);
    if !veh.is_null() {
        entity_register_reference(veh, this.byte_add(0xC));
    }
    this
}

/// `CTaskComplexLeaveCarAndWander::GetTaskType` (0x63BE40): 0x2C3.
#[hook(addr = 0x63BE40, conv = thiscall, name = "CTaskComplexLeaveCarAndWander::GetTaskType", expect = "B8 C3 02 00 00")]
pub fn leave_car_and_wander_get_task_type(_this: GPtr<u8>) -> u32 {
    0x2C3
}

/// `CTaskComplexLeaveCarAndWander::Destructor` (0x63BE50). vtable 0x86E880; the vehicle reference (+0xC)
/// cleaned up; then the `CTaskComplex` destructor.
#[hook(addr = 0x63BE50, conv = thiscall, name = "CTaskComplexLeaveCarAndWander::Destructor", expect = "6A FF 68 18 17 84 00")]
pub fn leave_car_and_wander_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86E880);
    let v = rd::<GPtr<u8>>(this, 0xC);
    if !v.is_null() {
        entity_clean_up_old_reference(v, this.byte_add(0xC));
    }
    task_complex_dtor(this);
}

/// `CTaskComplexLeaveCarAndWander::ControlSubTask(CPed*)` (0x63BEB0, `ret 4`): the sub-task.
#[hook(addr = 0x63BEB0, conv = thiscall, name = "CTaskComplexLeaveCarAndWander::ControlSubTask", expect = "8B 41 08 C2 04 00")]
pub fn leave_car_and_wander_control_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 8)
}

/// `CTaskComplexLeaveCarAndWander::CreateSubTask(int32 taskType)` (0x63BEC0, `ret 4`). 0x2C0: a new
/// `CTaskComplexLeaveCar(+0xC, +0x10, +0x14, +0x18 (zero-extended), false)`; 0xC8: an inlined
/// `CTaskSimpleLeaveGroup` (8 bytes: `CTaskSimple`, vtable 0x859CFC); any other type (0x516 included): null.
#[hook(addr = 0x63BEC0, conv = thiscall, name = "CTaskComplexLeaveCarAndWander::CreateSubTask", expect = "64 A1 00 00 00 00")]
pub fn leave_car_and_wander_create_sub_task(this: GPtr<u8>, ty: i32) -> GPtr<u8> {
    match ty {
        0xC8 => {
            let mem = task_new(8);
            if mem.is_null() {
                return GPtr::NULL;
            }
            task_simple_ctor(mem);
            wr::<u32>(mem, 0, 0x859CFC);
            mem
        }
        0x2C0 => {
            let mem = task_new(0x34);
            if mem.is_null() {
                return GPtr::NULL;
            }
            leave_car_ctor(
                mem,
                rd::<GPtr<u8>>(this, 0xC),
                rd::<u32>(this, 0x10),
                rd::<u32>(this, 0x14),
                u32::from(rd::<u8>(this, 0x18)),
                0,
            )
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexLeaveCarAndWander::Clone` (0x63DAD0). A new task through the original constructor with +0xC,
/// +0x10, +0x14 and +0x18 (zero-extended); null when the pool is full.
#[hook(addr = 0x63DAD0, conv = thiscall, name = "CTaskComplexLeaveCarAndWander::Clone", expect = "6A FF 68 1B 1B 84 00")]
pub fn leave_car_and_wander_clone(this: GPtr<u8>) -> GPtr<u8> {
    let mem = task_new(0x1C);
    if mem.is_null() {
        return GPtr::NULL;
    }
    leave_car_and_wander_ctor_orig(
        mem,
        rd::<GPtr<u8>>(this, 0xC),
        rd::<u32>(this, 0x10),
        rd::<u32>(this, 0x14),
        u32::from(rd::<u8>(this, 0x18)),
    )
}

/// `CTaskComplexLeaveCarAndWander::CreateNextSubTask(CPed*)` (0x6422C0, `ret 4`). After a 0xC8 or 0x2C0:
/// `CreateSubTask(0x516)` (always null), and unless the ped's primary task 4 (intel +0x14) is a 0x390, primary
/// task 4 = `GetWanderTaskByPedType(ped)` (`SetTask(.., 4, false)`); returns that `CreateSubTask` result. Else null.
#[hook(addr = 0x6422C0, conv = thiscall, name = "CTaskComplexLeaveCarAndWander::CreateNextSubTask", expect = "53 56 8B F1 8B 4E 08")]
pub fn leave_car_and_wander_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let ty = v_task_type(rd::<GPtr<u8>>(this, 8));
    if ty != 0xC8 && ty != 0x2C0 {
        return GPtr::NULL;
    }
    let r = leave_car_and_wander_create_sub_task_orig(this, 0x516);
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let t = rd::<GPtr<u8>>(intel, 0x14);
    if !t.is_null() && v_task_type(t) == 0x390 {
        return r;
    }
    let w = wander_task_by_ped_type(ped);
    tm_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), w, 4, 0);
    r
}

/// `CTaskComplexLeaveCarAndWander::CreateFirstSubTask(CPed*)` (0x642340, `ret 4`): tail call of
/// `CreateSubTask(ped in a vehicle (+0x46C bit 8) ? 0x2C0 : 0xC8)` (the original overwrites its argument slot).
#[hook(addr = 0x642340, conv = thiscall, name = "CTaskComplexLeaveCarAndWander::CreateFirstSubTask", expect = "8B 44 24 04 8B 90 6C 04 00 00")]
pub fn leave_car_and_wander_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let ty = if rd::<u32>(ped, 0x46C) & 0x100 != 0 { 0x2C0 } else { 0xC8 };
    leave_car_and_wander_create_sub_task_orig(this, ty)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_GANG_LEADER_DTOR,
    HOOK_GANG_LEADER_SCAN_FOR_STUFF,
    HOOK_GANG_LEADER_TRY_TO_PASS_OBJECT,
    HOOK_GANG_LEADER_CONTROL_SUB_TASK,
    HOOK_GANG_FOLLOWER_CONTROL_SUB_TASK,
    HOOK_GANG_FOLLOWER_CREATE_NEXT_SUB_TASK,
    HOOK_GANG_HASSLE_VEHICLE_CTOR,
    HOOK_GANG_HASSLE_VEHICLE_CREATE_NEXT_SUB_TASK,
    HOOK_GANG_HASSLE_VEHICLE_GET_TARGET_HEADING,
    HOOK_GANG_HASSLE_VEHICLE_CALC_TARGET_OFFSET,
    HOOK_GANG_HASSLE_VEHICLE_CREATE_FIRST_SUB_TASK,
    HOOK_GANG_HASSLE_PED_DTOR,
    HOOK_SIGNAL_AT_PED_CTOR,
    HOOK_SIGNAL_AT_PED_DTOR,
    HOOK_LEAVE_CAR_AND_WANDER_CTOR,
    HOOK_LEAVE_CAR_AND_WANDER_GET_TASK_TYPE,
    HOOK_LEAVE_CAR_AND_WANDER_DTOR,
    HOOK_LEAVE_CAR_AND_WANDER_CONTROL_SUB_TASK,
    HOOK_LEAVE_CAR_AND_WANDER_CREATE_SUB_TASK,
    HOOK_LEAVE_CAR_AND_WANDER_CLONE,
    HOOK_LEAVE_CAR_AND_WANDER_CREATE_NEXT_SUB_TASK,
    HOOK_LEAVE_CAR_AND_WANDER_CREATE_FIRST_SUB_TASK,
];
