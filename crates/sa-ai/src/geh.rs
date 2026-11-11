//! `CGroupEventHandler::Compute*Response` and the `CEventHandler` partner responses (PORT-AI-6). Ported from
//! the 1.0 US bytes; gta-reversed and `docs/re/specs/AI` only as reading aids (where they differ, the bytes
//! win: see the per-function notes).
//!
//! Every `CGroupEventHandler` function is `cdecl` and returns a `CTaskAllocator*` (EAX). Layouts used:
//! `CPedGroup`: +0 a ped pointer (written by `ComputeWalkAlongsideResponse`), +0xC the 7 followers, +0x28 the
//! leader (so +0xC..+0x28 = the 8 members), +0x30 its `CPedGroupIntelligence` (+0xC primary `CPedTaskPair[8]`,
//! +0x1EC default pairs, stride 0x14: +0 ped, +4 task, +8 slot; +0x29C task sequence id), +0x2D0 mission-group
//! byte. Events: +0xE the (editable) response task id (i16), +0x14 the source / threat entity.
//!
//! Task objects the original builds on its stack (the response task, the unused `CTaskSimpleNone` around every
//! `CPedGroupIntelligence::SetTask`, sequences) are zeroed Rust locals here; their constructors and destructors
//! are the original ones (entity references registered with a Rust stack address are released by the same
//! destructor). Writes the originals make into their own argument slots (`ComputeResponseNewGangMember`,
//! `ComputeResponseGather`, `ComputeWalkAlongsideResponse`, `ComputeMemberResponses`) and the tail jumps that
//! rewrite the caller's argument slots (`ComputeResponsePedThreat` -> `ComputeDrivebyResponse`,
//! `ComputeResponseDraggedOutCar` -> `ComputeMemberResponses` / `ComputeResponseLeaderExitedCar`) are not
//! reproduced: no caller reads those slots after the call.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

type P = GPtr<u8>;

orig! {
    /// `CTask::operator delete`-style virtual destructor (vtable +0, `ret 4`)
    fn v_delete(o: P, flags: u32) -> P = thiscall vslot 0x0 => unported;
    /// `CTask::Clone` (vtable +4)
    fn v_clone(t: P) -> P = thiscall vslot 0x4 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_10(t: P) -> i32 = thiscall vslot 0x10 => unported;
    /// `CEvent::Clone` (vtable +0x10)
    fn v_event_clone(e: P) -> P = thiscall vslot 0x10 => unported;
    /// `CEvent` vtable +0x24 (bool: the response task can be recomputed for each member)
    fn v_event_24(e: P) -> u8 = thiscall vslot 0x24 => unported;
    /// `CEvent::GetSourceEntity` (vtable +0x28)
    fn v_event_source(e: P) -> P = thiscall vslot 0x28 => unported;
    /// `CEvent::GetEventType` (vtable +4)
    fn v_event_type(e: P) -> i32 = thiscall vslot 0x4 => unported;

    /// `CPedGroupIntelligence::SetTask(CPed*, const CTask&, CPedTaskPair*, int32 slot, bool)` (0x5F7540, `ret 0x14`)
    fn pgi_set_task(i: P, ped: P, task: P, pairs: P, slot: i32, flag: u32) = thiscall 0x5F7540 => unported;
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: P) -> P = thiscall 0x61A390 => unported;
    /// `CTaskSimple::~CTaskSimple` (0x61A3A0)
    fn task_simple_dtor(t: P) = thiscall 0x61A3A0 => unported;
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: P) -> P = thiscall 0x61A3B0 => unported;
    /// `CTaskComplex::~CTaskComplex` (0x61A3D0)
    fn task_complex_dtor(t: P) = thiscall 0x61A3D0 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence` (0x632BD0)
    fn seq_ctor(t: P) -> P = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10)
    fn seq_add(t: P, task: P) = thiscall 0x632D10 => unported;
    /// `CTaskComplexSequence::~CTaskComplexSequence` (0x6389F0)
    fn seq_dtor(t: P) = thiscall 0x6389F0 => unported;
    /// `CTaskAllocator::operator new(uint32)` (0x69D980)
    fn alloc_new(size: u32) -> P = cdecl 0x69D980 => unported;
    /// `CTaskAllocatorKillThreatsDriveby::CTaskAllocatorKillThreatsDriveby(CPed*)` (0x69CAF0)
    fn alloc_driveby_ctor(a: P, threat: P) -> P = thiscall 0x69CAF0 => unported;
    /// `CTaskAllocatorKillThreatsBasic::CTaskAllocatorKillThreatsBasic(CPed*)` (0x69C710)
    fn alloc_kill_basic_ctor(a: P, threat: P) -> P = thiscall 0x69C710 => unported;
    /// `CTaskAllocatorPlayerCommandAttack::CTaskAllocatorPlayerCommandAttack(CPed*, int32 groupId, int32)` (0x69C480)
    fn alloc_cmd_attack_ctor(a: P, target: P, group_id: i32, x: u32) -> P = thiscall 0x69C480 => unported;
    /// `CWeapon::IsTypeMelee` (0x73B1C0)
    fn weapon_is_melee(w: P) -> u8 = thiscall 0x73B1C0 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: P) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::IsAlive` (0x5E0170)
    fn ped_is_alive(p: P) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0)
    fn intel_find_task(i: P, ty: i32) -> P = thiscall 0x600EE0 => unported;
    /// `CPedIntelligence::IsFriendlyWith(const CPed&)` (0x601BC0)
    fn intel_is_friendly_with(i: P, p: P) -> u8 = thiscall 0x601BC0 => unported;
    /// `CPedIntelligence::Respects(CPed*)` (0x601C90)
    fn intel_respects(i: P, p: P) -> u8 = thiscall 0x601C90 => unported;
    /// `CPedGroupMembership::IsMember(const CPed*)` (0x5F6A10)
    fn membership_is_member(m: P, p: P) -> u8 = thiscall 0x5F6A10 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn groups_get_peds_group(p: P) -> P = cdecl 0x5F7E80 => unported;
    /// `CPedGroups::GetGroupId(CPedGroup*)` (0x5F7EE0)
    fn groups_get_group_id(g: P) -> i32 = cdecl 0x5F7EE0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> P = cdecl 0x56E210 => unported;
    /// `CVehicle::GetSpareHasslePosId` (0x6D3AE0)
    fn veh_spare_hassle_pos(v: P) -> i32 = thiscall 0x6D3AE0 => unported;
    /// `FxSystem_c::GetPlayStatus` (0x4AA900)
    fn fx_get_play_status(f: P) -> i32 = thiscall 0x4AA900 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: P, slot: P) = thiscall 0x571A00 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CEventEditableResponse::ComputeResponseTaskType(CPed*, bool)` (0x4B56C0, `ret 8`)
    fn editable_compute_response_task_type(e: P, ped: P, b: u32) = thiscall 0x4B56C0 => unported;
    /// `CEventHandler::ComputeEventResponseTask(CPed*, CEvent*)` (0x4C4220)
    fn handler_compute_event_response_task(ped: P, e: P) -> P = cdecl 0x4C4220 => unported;

    // Task constructors / destructors (stack objects and pool tasks).
    /// `CTaskComplexLeaveCar::CTaskComplexLeaveCar(CVehicle*, int32 door, int32 delay, bool, bool)` (0x63B8C0)
    fn leave_car_ctor(t: P, veh: P, door: i32, delay: i32, a: u32, b: u32) -> P = thiscall 0x63B8C0 => unported;
    /// `CTaskComplexLeaveCar::~CTaskComplexLeaveCar` (0x63B970)
    fn leave_car_dtor(t: P) = thiscall 0x63B970 => unported;
    /// `CTaskComplexLeaveCarAsPassengerWait::CTaskComplexLeaveCarAsPassengerWait(CVehicle*)` (0x63B430)
    fn leave_car_pw_ctor(t: P, veh: P) -> P = thiscall 0x63B430 => unported;
    /// `CTaskComplexLeaveCarAsPassengerWait::~...` (0x63B4A0)
    fn leave_car_pw_dtor(t: P) = thiscall 0x63B4A0 => unported;
    /// `CTaskComplexSmartFleeEntity::CTaskComplexSmartFleeEntity(CEntity*, bool, float, int32, int32, float)` (0x65C430)
    fn flee_entity_ctor(t: P, e: P, b: u32, safe: u32, time: u32, a: u32, c: u32) -> P = thiscall 0x65C430 => unported;
    /// `CTaskComplexSmartFleeEntity::~...` (0x65C4D0)
    fn flee_entity_dtor(t: P) = thiscall 0x65C4D0 => unported;
    /// `CTaskComplexEnterCarAsPassengerWait::CTaskComplexEnterCarAsPassengerWait(CVehicle*, CPed*, bool, int32)` (0x63B320)
    fn enter_car_pw_ctor(t: P, veh: P, wait_for: P, front: u32, x: i32) -> P = thiscall 0x63B320 => unported;
    /// `CTaskComplexEnterCarAsPassengerWait::~...` (0x63B3C0)
    fn enter_car_pw_dtor(t: P) = thiscall 0x63B3C0 => unported;
    /// `CTaskComplexUseSequence::CTaskComplexUseSequence(int32)` (0x635450)
    fn use_sequence_ctor(t: P, id: i32) -> P = thiscall 0x635450 => unported;
    /// `CTaskSimpleCarDrive::CTaskSimpleCarDrive(CVehicle*, CTaskUtilityLineUpPedWithCar*, bool)` (0x63C340)
    fn car_drive_ctor(t: P, veh: P, line_up: P, b: u32) -> P = thiscall 0x63C340 => unported;
    /// `CTaskSimpleWaitUntilAreaCodesMatch::CTaskSimpleWaitUntilAreaCodesMatch(CPed*)` (0x635540)
    fn wait_area_ctor(t: P, ped: P) -> P = thiscall 0x635540 => unported;
    /// `CTaskSimpleWaitUntilLeaderAreaCodesMatch::ProcessPed(CPed*)` (0x6356C0)
    fn wait_leader_area_process(t: P, ped: P) -> u8 = thiscall 0x6356C0 => unported;
    /// `CTaskSimpleWaitUntilAreaCodesMatch::~...` (0x6355E0)
    fn wait_area_dtor(t: P) = thiscall 0x6355E0 => unported;
    /// `CTaskComplexGangJoinRespond::CTaskComplexGangJoinRespond(bool)` (0x6616F0)
    fn gang_join_respond_ctor(t: P, b: u32) -> P = thiscall 0x6616F0 => unported;
    /// `CTaskSimpleGoToPoint::CTaskSimpleGoToPoint(int32 moveState, const CVector&, float, bool, bool)` (0x667CD0)
    fn go_to_point_ctor(t: P, ms: i32, pos: P, r: u32, a: u32, b: u32) -> P = thiscall 0x667CD0 => unported;
    /// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorStandard>::CTaskComplexSeekEntity(...)` (0x46AC10)
    fn seek_entity_ctor(t: P, e: P, time: i32, scan: i32, r: u32, a: u32, b: u32, c: u32, d: u32) -> P = thiscall 0x46AC10 => unported;
    /// `CTaskGoToVehicleAndLean::CTaskGoToVehicleAndLean(CVehicle*, int32)` (0x660E60)
    fn lean_ctor(t: P, veh: P, dur: u32) -> P = thiscall 0x660E60 => unported;
    /// `CTaskGoToVehicleAndLean::~...` (0x660EE0)
    fn lean_dtor(t: P) = thiscall 0x660EE0 => unported;
    /// `CTaskGangHassleVehicle::CTaskGangHassleVehicle(CVehicle*, int32, uint8, float, float)` (0x65FAC0)
    fn hassle_veh_ctor(t: P, veh: P, a: i32, b: u32, c: u32, d: u32) -> P = thiscall 0x65FAC0 => unported;
    /// `CTaskGangHassleVehicle::~...` (0x65FB60)
    fn hassle_veh_dtor(t: P) = thiscall 0x65FB60 => unported;
    /// `CTaskGangHasslePed::CTaskGangHasslePed(CPed*, int32, int32, int32)` (0x65FED0)
    fn hassle_ped_ctor(t: P, ped: P, a: i32, b: i32, c: i32) -> P = thiscall 0x65FED0 => unported;
    /// `CTaskGangHasslePed::~...` (0x65FF60)
    fn hassle_ped_dtor(t: P) = thiscall 0x65FF60 => unported;
    /// `CTaskComplexSignalAtPed::CTaskComplexSignalAtPed(CPed*, int32, bool)` (0x660A30)
    fn signal_ctor(t: P, ped: P, a: i32, b: u32) -> P = thiscall 0x660A30 => unported;
    /// `CTaskComplexSignalAtPed::~...` (0x660AB0)
    fn signal_dtor(t: P) = thiscall 0x660AB0 => unported;
    /// `CTaskComplexWalkAlongsidePed::CTaskComplexWalkAlongsidePed(CPed*, float)` (0x683240)
    fn walk_alongside_ctor(t: P, ped: P, r: u32) -> P = thiscall 0x683240 => unported;
    /// `CTaskComplexWalkAlongsidePed::~...` (0x6832C0)
    fn walk_alongside_dtor(t: P) = thiscall 0x6832C0 => unported;
    /// `CTaskComplexKillPedOnFoot::CTaskComplexKillPedOnFoot(CPed*, int32, uint32, uint32, uint32, int8)` (0x620E30)
    fn kill_on_foot_ctor(t: P, ped: P, a: i32, b: u32, c: u32, d: u32, e: u32) -> P = thiscall 0x620E30 => unported;
    /// `CTaskComplexKillPedOnFoot::~...` (0x620EF0)
    fn kill_on_foot_dtor(t: P) = thiscall 0x620EF0 => unported;

    /// `CTaskComplexPartnerChat::CTaskComplexPartnerChat(const char*, CPed*, bool, float, int32, bool, int32,
    /// CVector)` (0x684290, `ret 0x28`)
    fn partner_chat_ctor(t: P, tag: u32, partner: P, lead: u32, dist: u32, a: i32, b: u32, c: i32, vx: u32, vy: u32, vz: u32) -> P = thiscall 0x684290 => unported;
    /// `CTaskComplexPartnerDeal::CTaskComplexPartnerDeal(const char*, CPed*, bool, float, CVector)` (0x684190)
    fn partner_deal_ctor(t: P, tag: u32, partner: P, lead: u32, dist: u32, vx: u32, vy: u32, vz: u32) -> P = thiscall 0x684190 => unported;
    /// `CTaskComplexPartnerGreet::CTaskComplexPartnerGreet(const char*, CPed*, bool, float, int32, CVector)` (0x684210)
    fn partner_greet_ctor(t: P, tag: u32, partner: P, lead: u32, dist: u32, kind: i32, vx: u32, vy: u32, vz: u32) -> P = thiscall 0x684210 => unported;

    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    fn task_manager_find_active(tm: P, ty: i32) -> P = thiscall 0x681740 => unported;
    /// `VectorSub(CVector* out, const CVector*, const CVector*)` (0x40FE60)
    fn vector_sub(out: P, a: P, b: P) -> P = cdecl 0x40FE60 => unported;
    /// `CEventCreatePartnerTask::CEventCreatePartnerTask(int32, CPed*, bool, float)` (0x5F6190)
    fn create_partner_event_ctor(e: P, ty: i32, partner: P, lead: u32, dist: u32) -> P = thiscall 0x5F6190 => unported;
    /// `CEventCreatePartnerTask::~CEventCreatePartnerTask` (0x5F6200)
    fn create_partner_event_dtor(e: P) = thiscall 0x5F6200 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: P, e: P, b: u32) -> P = thiscall 0x4AB420 => unported;
    /// `IKChainManager_c::LookAt(const char*, CPed*, CEntity*, int32, int32 bone, RwV3d*, bool, float, int32, int32,
    /// bool)` (0x618970)
    fn ik_look_at(m: P, tag: u32, ped: P, target: P, time: i32, bone: i32, pos: u32, b: u32, speed: u32, blend: i32, prio: i32, c: u32) = thiscall 0x618970 => unported;
    /// `CTaskComplexPartnerDeal::~CTaskComplexPartnerDeal` (0x684200)
    fn partner_deal_dtor(t: P) = thiscall 0x684200 => unported;
    /// `CTaskComplexPartnerGreet::~CTaskComplexPartnerGreet` (0x684280)
    fn partner_greet_dtor(t: P) = thiscall 0x684280 => unported;
    /// `CTaskComplexStareAtPed::CTaskComplexStareAtPed(CPedGroup*, CPed*, int32)` (0x660270)
    fn stare_ctor(t: P, pg: P, ped: P, time: i32) -> P = thiscall 0x660270 => unported;
    /// `CTaskComplexStareAtPed::~CTaskComplexStareAtPed` (0x660310)
    fn stare_dtor(t: P) = thiscall 0x660310 => unported;

    // CGroupEventHandler callees (ported here; called at their original addresses).
    fn geh_driveby(pg: P, threat: P, orig: P) -> P = cdecl 0x5F7A00 => unported;
    fn geh_is_kill_task_appropriate(pg: P, threat: P) -> u8 = cdecl 0x5F7A60 => unported;
    fn geh_leader_entered_car(e: P, pg: P, orig: P) -> P = cdecl 0x5F8900 => unported;
    fn geh_leader_exited_car(e: P, pg: P, orig: P) -> P = cdecl 0x5F90A0 => unported;
    fn geh_leader_quit_entering_car(e: P, pg: P, orig: P) -> P = cdecl 0x5F9530 => unported;
    fn geh_leader_enter_exit(e: P, pg: P, orig: P) -> P = cdecl 0x5F9710 => unported;
    fn geh_new_gang_member(e: P, pg: P, orig: P) -> P = cdecl 0x5F9840 => unported;
    fn geh_gather(e: P, pg: P, orig: P) -> P = cdecl 0x5F99F0 => unported;
    fn geh_lean_on_vehicle(e: P, pg: P, orig: P) -> P = cdecl 0x5F9B20 => unported;
    fn geh_stare(pg: P, ped: P, orig: P, timeout: i32, bias: i32) -> P = cdecl 0x5F9BD0 => unported;
    fn geh_hassle_threat(pg: P, threat: P, orig: P, b: u32) -> P = cdecl 0x5F9D50 => unported;
    fn geh_hassle_sexy_ped(pg: P, ped: P, orig: P) -> P = cdecl 0x5FA020 => unported;
    fn geh_flee_ped(pg: P, threat: P, orig: P, b: u32) -> P = cdecl 0x5FA130 => unported;
    fn geh_do_deal(pg: P, ped: P, orig: P) -> P = cdecl 0x5FA290 => unported;
    fn geh_greet(pg: P, ped: P, orig: P) -> P = cdecl 0x5FA550 => unported;
    fn geh_hand_signal(pg: P, ped: P, orig: P) -> P = cdecl 0x5FA820 => unported;
    fn geh_member_responses(e: P, pg: P, orig: P) -> P = cdecl 0x5FAA50 => unported;
    fn geh_ped_friend(e: P, pg: P, orig: P) -> P = cdecl 0x5FB2D0 => unported;
    fn geh_sexy_ped(e: P, pg: P, orig: P) -> P = cdecl 0x5FB390 => unported;
    fn geh_player_command(e: P, pg: P, orig: P) -> P = cdecl 0x5FB470 => unported;
    fn geh_danger(e: P, pg: P, orig: P) -> P = cdecl 0x5FB540 => unported;
    fn geh_kill_threats_basic(pg: P, threat: P, orig: P, b: u32) -> P = cdecl 0x5FB590 => unported;
    fn geh_kill_player_basic(pg: P, threat: P, orig: P, b: u32) -> P = cdecl 0x5FB670 => unported;
    fn geh_ped_threat(e: P, pg: P, orig: P) -> P = cdecl 0x5FBB90 => unported;
    fn geh_seen_cop(e: P, pg: P, orig: P) -> P = cdecl 0x5FBCB0 => unported;
    fn geh_gun_aimed_at(e: P, pg: P, orig: P) -> P = cdecl 0x5FBD10 => unported;
    fn geh_shot_fired(e: P, pg: P, orig: P) -> P = cdecl 0x5FBDF0 => unported;
    fn geh_dragged_out_car(e: P, pg: P, orig: P) -> P = cdecl 0x5FBE70 => unported;
    fn geh_damage(e: P, pg: P, orig: P) -> P = cdecl 0x5FBF50 => unported;
    fn geh_vehicle_damage(e: P, pg: P, orig: P) -> P = cdecl 0x5FC070 => unported;
}

/// `CTaskSimpleNone` vtable (the unused stack task around every `SetTask`).
const VT_TASK_SIMPLE_NONE: u32 = 0x859CFC;
/// `CTaskComplexLeaveAnyCar` vtable.
const VT_LEAVE_ANY_CAR: u32 = 0x858EB0;

fn p(b: P, off: isize) -> P {
    rd::<P>(b, off)
}

/// A zeroed buffer for a task object the original keeps on its stack (every task fits a 0x80-byte pool slot;
/// 0x100 bytes leave room for the larger stack objects).
struct Local([u32; 0x40]);

impl Local {
    fn new() -> Self {
        Local([0; 0x40])
    }
    fn ptr(&mut self) -> P {
        GPtr::from_raw(&mut self.0 as *mut [u32; 0x40]).cast()
    }
}

/// `CPedGroupIntelligence` of a group.
fn intel_of(pg: P) -> P {
    pg.byte_add(0x30)
}

/// `CEntity::GetPosition`: the matrix position when there is a matrix, else the placement.
fn pos_of(e: P) -> P {
    let m = p(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `(a - b).SquaredMagnitude()` as the originals evaluate it: z, y, x differences, then
/// `(dx*dx + dy*dy) + dz*dz`.
fn dist_sq(a: P, b: P) -> X87 {
    let dz = x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(b, 8));
    let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4));
    let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0));
    dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz))
}

/// `fcomp m32; fnstsw; test ah, 0x41; je out`: true when `v <= m` (or unordered).
fn le_m(v: X87, m: usize) -> bool {
    v.fcom(x(g::<f32>(m).read())).test_any(0x41)
}

/// The response task `task` for `ped` through `CPedGroupIntelligence::SetTask(ped, task, primary pairs, -1,
/// false)`, with the unused stack `CTaskSimpleNone` constructed before and destroyed after it, as every caller
/// here does.
fn set_response(intel: P, ped: P, task: P) {
    let mut none = Local::new();
    let n = none.ptr();
    task_simple_ctor(n);
    wr::<u32>(n, 0, VT_TASK_SIMPLE_NONE);
    pgi_set_task(intel, ped, task, intel.byte_add(0xC), -1, 0);
    wr::<u32>(n, 0, VT_TASK_SIMPLE_NONE);
    task_simple_dtor(n);
}

/// The member slots `pg+0xC + 4k`, k < n (n = 7: followers, 8: followers and leader).
fn member(pg: P, k: isize) -> P {
    p(pg, 0xC + 4 * k)
}

/// The ped's current weapon: `ped+0x5A0 + 0x1C * (int8)ped+0x718`.
fn active_weapon(ped: P) -> P {
    let slot = rd::<i8>(ped, 0x718) as isize;
    ped.byte_add(0x5A0 + slot * 0x1C)
}

/// `ped+0x36 & 7 == 3` (`ENTITY_TYPE_PED`).
fn is_ped(e: P) -> bool {
    rd::<u8>(e, 0x36) & 7 == 3
}

/// `CGroupEventHandler::ComputeDrivebyResponse(CPedGroup*, CPed* threat, CPed*)` (0x5F7A00): a new
/// `CTaskAllocatorKillThreatsDriveby(threat)` (null when the allocation fails).
#[hook(addr = 0x5F7A00, conv = cdecl, name = "CGroupEventHandler::ComputeDrivebyResponse", expect = "6A FF 68 EB DF 83 00")]
pub fn geh_compute_driveby_response(_pg: P, threat: P, _orig: P) -> P {
    let a = alloc_new(0x14);
    if a.is_null() {
        return GPtr::NULL;
    }
    alloc_driveby_ctor(a, threat)
}

/// `CGroupEventHandler::IsKillTaskAppropriate(CPedGroup*, CPed* threat)` (0x5F7A60): true for a mission group
/// (byte +0x2D0 == 1), a threat with a melee weapon, or any of the 8 member slots holding a ped whose weapon is
/// not melee (null slots are skipped). AL only.
#[hook(addr = 0x5F7A60, conv = cdecl, name = "CGroupEventHandler::IsKillTaskAppropriate", expect = "56 8B 74 24 08")]
pub fn geh_is_kill_task_appropriate_port(pg: P, threat: P) -> u8 {
    if rd::<u8>(pg, 0x2D0) == 1 || weapon_is_melee(active_weapon(threat)) != 0 {
        return 1;
    }
    for k in 0..8 {
        let m = member(pg, k);
        if !m.is_null() && weapon_is_melee(active_weapon(m)) == 0 {
            return 1;
        }
    }
    0
}

/// Index of `ped` in the 8 pairs at `pairs` (stride 0x14), as the inlined searches find it.
fn find_pair(pairs: P, ped: P) -> Option<P> {
    (0..8isize).map(|k| pairs.byte_add(k * 0x14)).find(|&pr| p(pr, 0) == ped)
}

/// The inlined `CPedGroupIntelligence::SetTask` of `ComputeResponseLeaderEnteredCar`: the pair of `ped` gets a
/// clone of `t` unless it already holds a task of the same type (the old task is deleted after the store).
/// `pair_type_first`: which `GetTaskType` the original calls first (the primary-pair copies call the pair's,
/// the default-pair copy calls `t`'s).
fn inline_set(pairs: P, ped: P, t: P, pair_type_first: bool) {
    let Some(pr) = find_pair(pairs, ped) else {
        return;
    };
    let cur = p(pr, 4);
    if !cur.is_null() {
        let (a, b) = if pair_type_first {
            let a = v_10(cur);
            (a, v_10(t))
        } else {
            let b = v_10(t);
            (v_10(cur), b)
        };
        if a != b {
            let old = p(pr, 4);
            let c = v_clone(t);
            wr::<P>(pr, 4, c);
            wr::<i32>(pr, 8, -1);
            if !old.is_null() {
                v_delete(old, 1);
            }
        }
    } else {
        let c = v_clone(t);
        wr::<P>(pr, 4, c);
        wr::<i32>(pr, 8, -1);
    }
}

/// The front-seat flag of `ComputeResponseLeaderEnteredCar` (inlined twice): set when exactly one follower slot
/// is used and the vehicle is neither a bike (type 9) nor a quad (subtype 2), cleared again when the front
/// passenger seat (+0x464) holds another ped that is a follower, the leader or has `bDontDragMeOutCar`.
fn front_seat_flag(pg: P, veh: P, m: P) -> u32 {
    let n = (0..7).filter(|&k| !member(pg, k).is_null()).count();
    if n != 1 || rd::<u32>(veh, 0x590) == 9 || rd::<u32>(veh, 0x594) == 2 {
        return 0;
    }
    let d = p(veh, 0x464);
    if d.is_null() || d == m {
        return 1;
    }
    if (0..7).any(|k| member(pg, k) == d) || p(pg, 0x28) == d || rd::<u32>(d, 0x46C) & 0x2000_0000 != 0 {
        return 0;
    }
    1
}

/// A pool task built by `ctor` on a fresh `CTask::operator new(size)` slot, null when the allocation fails.
fn new_task(size: u32, ctor: impl FnOnce(P) -> P) -> P {
    let t = task_new(size);
    if t.is_null() {
        GPtr::NULL
    } else {
        ctor(t)
    }
}

/// `seq.AddTask(new CTaskSimpleCarDrive(veh, null, true))`, then a clone of the ped's default task (when its
/// default pair holds one) — the default-task sequence of `ComputeResponseLeaderEnteredCar`.
fn add_drive_and_default(seq: P, intel: P, ped: P, veh: P) {
    let t = new_task(0x60, |t| car_drive_ctor(t, veh, GPtr::NULL, 1));
    seq_add(seq, t);
    if let Some(pr) = find_pair(intel.byte_add(0x1EC), ped) {
        let d = p(pr, 4);
        if !d.is_null() {
            let c = v_clone(d);
            seq_add(seq, c);
        }
    }
}

/// `CGroupEventHandler::ComputeResponseLeaderEnteredCar(const CEvent&, CPedGroup*, CPed*)` (0x5F8900; gta-reversed
/// does not reverse it). Mission followers (`m_nCreatedBy == 2`) first, then the others; each gets
/// `CTaskComplexEnterCarAsPassengerWait(veh, waitFor, front, 6)` (in a sequence with `CTaskComplexUseSequence(seqId)`
/// for task 0x5E9 with a valid script sequence) while seats are left, else `CTaskComplexLeaveAnyCar`; followers
/// without a group drive task (and finally the leader, unless it is the player) get the drive task plus their old
/// default task as default. Always returns 0.
#[hook(addr = 0x5F8900, conv = cdecl, name = "CGroupEventHandler::ComputeResponseLeaderEnteredCar", expect = "6A FF 68 2D E1 83 00")]
pub fn geh_compute_response_leader_entered_car(e: P, pg: P, _orig: P) -> P {
    let task_id = rd::<i16>(e, 0xE) as i32;
    let intel = intel_of(pg);
    let seq_id = rd::<i32>(intel, 0x29C);
    let leader = p(pg, 0x28);
    if leader.is_null() {
        return GPtr::NULL;
    }
    let veh = p(e, 0x14);
    if veh.is_null() {
        return GPtr::NULL;
    }
    let max_pass = rd::<u8>(veh, 0x488) as i32;
    let mut list = [GPtr::<u8>::NULL; 7];
    let mut n = 0;
    for k in 0..7 {
        let m = member(pg, k);
        if !m.is_null() && rd::<u8>(m, 0x484) == 2 {
            list[n] = m;
            n += 1;
        }
    }
    for k in 0..7 {
        let m = member(pg, k);
        if !m.is_null() && rd::<u8>(m, 0x484) != 2 {
            list[n] = m;
            n += 1;
        }
    }
    let mut wait_for = leader;
    let mut assigned = 0i32;
    for &m in &list {
        if m.is_null() {
            continue;
        }
        if assigned >= max_pass {
            // No seat left: CTaskComplexLeaveAnyCar(0, true, false) on the stack.
            let mut lt = Local::new();
            let t = lt.ptr();
            task_complex_ctor(t);
            wr::<u32>(t, 0, VT_LEAVE_ANY_CAR);
            wr::<u32>(t, 0xC, 0);
            wr::<u8>(t, 0x10, 1);
            wr::<u8>(t, 0x11, 0);
            let mut none = Local::new();
            let nn = none.ptr();
            task_simple_ctor(nn);
            wr::<u32>(nn, 0, VT_TASK_SIMPLE_NONE);
            inline_set(intel.byte_add(0xC), m, t, true);
            wr::<u32>(nn, 0, VT_TASK_SIMPLE_NONE);
            task_simple_dtor(nn);
            wr::<u32>(t, 0, VT_LEAVE_ANY_CAR);
            task_complex_dtor(t);
            continue;
        }
        let seq_ok = task_id == 0x5E9
            && (0..0x40).contains(&seq_id)
            && g::<u32>(0xC17900 + (seq_id as usize) * 0x40).read() != 0;
        if seq_ok {
            let front = front_seat_flag(pg, veh, m);
            let mut ls = Local::new();
            let seq = ls.ptr();
            seq_ctor(seq);
            let t = new_task(0x24, |t| enter_car_pw_ctor(t, veh, wait_for, front, 6));
            seq_add(seq, t);
            let t = new_task(0x1C, |t| use_sequence_ctor(t, seq_id));
            seq_add(seq, t);
            set_response(intel, m, seq);
            seq_dtor(seq);
        } else {
            let front = front_seat_flag(pg, veh, m);
            let mut lt = Local::new();
            let t = lt.ptr();
            enter_car_pw_ctor(t, veh, wait_for, front, 6);
            let mut none = Local::new();
            let nn = none.ptr();
            task_simple_ctor(nn);
            wr::<u32>(nn, 0, VT_TASK_SIMPLE_NONE);
            inline_set(intel.byte_add(0xC), m, t, true);
            wr::<u32>(nn, 0, VT_TASK_SIMPLE_NONE);
            task_simple_dtor(nn);
            let f = rd::<u32>(m, 0x474);
            if f & 0x200_0000 == 0 {
                wr::<u32>(m, 0x474, f | 0x200_0000);
                let mut ls = Local::new();
                let seq = ls.ptr();
                seq_ctor(seq);
                add_drive_and_default(seq, intel, m, veh);
                inline_set(intel.byte_add(0x1EC), m, seq, false);
                seq_dtor(seq);
            }
            enter_car_pw_dtor(t);
        }
        assigned += 1;
        wait_for = m;
    }
    if ped_is_player(leader) == 0 {
        let f = rd::<u32>(leader, 0x474);
        if f & 0x200_0000 == 0 {
            wr::<u32>(leader, 0x474, f | 0x200_0000);
            let mut ls = Local::new();
            let seq = ls.ptr();
            seq_ctor(seq);
            add_drive_and_default(seq, intel, leader, veh);
            pgi_set_task(intel, leader, seq, intel.byte_add(0x1EC), -1, 0);
            seq_dtor(seq);
        }
    }
    GPtr::NULL
}

/// The vehicle of `CPedIntelligence::FindTaskByType(0x2CE)` (`EnterCarAsPassengerWait`, +0xC), else of
/// `FindTaskByType(0x2BC)` (`EnterCarAsPassenger`, +0xC); null when neither gives one.
fn entering_vehicle(m: P) -> P {
    let i = p(m, 0x47C);
    let t = intel_find_task(i, 0x2CE);
    if !t.is_null() {
        let v = p(t, 0xC);
        if !v.is_null() {
            return v;
        }
    }
    let t = intel_find_task(i, 0x2BC);
    if t.is_null() {
        GPtr::NULL
    } else {
        p(t, 0xC)
    }
}

/// `_ftol((float)(rand() & 0xFFFF) * 3.0517578e-05f * [k])` with the rand drawn at return address `site`.
fn rand_scaled(site: u32, k: usize) -> i32 {
    let r = at_site(site, rand) & 0xFFFF;
    ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(k).read()))
}

/// `new CTaskComplexSmartFleeEntity(veh, false, 15.0f, [0x86F674], [0x86F678], [0xC18CF0])` (the vehicle flee of
/// `ComputeResponseLeaderExitedCar`).
fn new_flee_vehicle(veh: P) -> P {
    new_task(0x40, |t| {
        let a = g::<u32>(0xC18CF0).read();
        let b = g::<u32>(0x86F678).read();
        let c = g::<u32>(0x86F674).read();
        flee_entity_ctor(t, veh, 0, 0x4170_0000, c, b, a)
    })
}

/// `CGroupEventHandler::ComputeResponseLeaderExitedCar(const CEvent&, CPedGroup*, CPed*)` (0x5F90A0). For follower
/// k (delay base 500k): its vehicle when it sits in the leader's vehicle (the leader is not null-checked), else the
/// vehicle of its enter-car task. Bus-like anim groups (handling +0xDE == 0xF / 0x10) get
/// `CTaskComplexLeaveCarAsPassengerWait`, others `CTaskComplexLeaveCar`; when the vehicle's fire particle
/// (+0x57C) exists and is not playing (`GetPlayStatus` == 0), a sequence with a flee from the vehicle instead.
/// gta-reversed skips followers already in the leader's vehicle; the bytes handle them. Always returns 0.
#[hook(addr = 0x5F90A0, conv = cdecl, name = "CGroupEventHandler::ComputeResponseLeaderExitedCar", expect = "6A FF 68 CA E1 83 00")]
pub fn geh_compute_response_leader_exited_car(_e: P, pg: P, _orig: P) -> P {
    let intel = intel_of(pg);
    for k in 0..7isize {
        let base = (500 * k) as i32;
        let m = member(pg, k);
        if m.is_null() {
            continue;
        }
        let mv = p(m, 0x58C);
        let veh = if !mv.is_null() && rd::<u32>(m, 0x46C) & 0x100 != 0 && mv == p(p(pg, 0x28), 0x58C) {
            mv
        } else {
            entering_vehicle(m)
        };
        if veh.is_null() {
            continue;
        }
        let group = rd::<u8>(p(veh, 0x384), 0xDE) as i32 + 0x58;
        let fire = || {
            let f = p(veh, 0x57C);
            !f.is_null() && fx_get_play_status(f) == 0
        };
        if group == 0x67 || group == 0x68 {
            if fire() {
                let mut ls = Local::new();
                let seq = ls.ptr();
                seq_ctor(seq);
                let t = new_task(0x10, |t| leave_car_pw_ctor(t, veh));
                seq_add(seq, t);
                let t = new_flee_vehicle(veh);
                seq_add(seq, t);
                set_response(intel, m, seq);
                seq_dtor(seq);
            } else {
                let mut lt = Local::new();
                let t = lt.ptr();
                leave_car_pw_ctor(t, veh);
                set_response(intel, m, t);
                leave_car_pw_dtor(t);
            }
        } else if fire() {
            let r = random_in_range(-250, 250);
            let delay = r.wrapping_add(base).wrapping_add(500);
            let mut ls = Local::new();
            let seq = ls.ptr();
            seq_ctor(seq);
            let t = new_task(0x34, |t| leave_car_ctor(t, veh, 0, delay, 0, 0));
            seq_add(seq, t);
            let t = new_flee_vehicle(veh);
            seq_add(seq, t);
            set_response(intel, m, seq);
            seq_dtor(seq);
        } else {
            let d = rand_scaled(0x5F92AF, 0x85AB78);
            let mut lt = Local::new();
            let t = lt.ptr();
            leave_car_ctor(t, veh, 0, base.wrapping_sub(d).wrapping_add(250), 0, 0);
            set_response(intel, m, t);
            leave_car_dtor(t);
        }
    }
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeResponsLeaderQuitEnteringCar(const CEvent&, CPedGroup*, CPed*)` (0x5F9530). Follower k
/// in a vehicle: `CTaskComplexLeaveCar(veh, 0, 500k, true, false)`; otherwise the vehicle of its enter-car task:
/// `CTaskComplexLeaveCar(veh, 0, 500k - d + 250, false, false)`, d = `_ftol(rand * 2^-15 * -500.0f)`. Returns 0.
#[hook(addr = 0x5F9530, conv = cdecl, name = "CGroupEventHandler::ComputeResponsLeaderQuitEnteringCar", expect = "6A FF 68 03 E2 83 00")]
pub fn geh_compute_respons_leader_quit_entering_car(_e: P, pg: P, _orig: P) -> P {
    let intel = intel_of(pg);
    for k in 0..7isize {
        let base = (500 * k) as i32;
        let m = member(pg, k);
        if m.is_null() {
            continue;
        }
        let mv = p(m, 0x58C);
        if rd::<u32>(m, 0x46C) & 0x100 != 0 && !mv.is_null() {
            let mut lt = Local::new();
            let t = lt.ptr();
            leave_car_ctor(t, mv, 0, base, 1, 0);
            set_response(intel, m, t);
            leave_car_dtor(t);
            continue;
        }
        let veh = entering_vehicle(m);
        if veh.is_null() {
            continue;
        }
        let d = rand_scaled(0x5F962D, 0x85AB78);
        let mut lt = Local::new();
        let t = lt.ptr();
        leave_car_ctor(t, veh, 0, base.wrapping_sub(d).wrapping_add(250), 0, 0);
        set_response(intel, m, t);
        leave_car_dtor(t);
    }
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeResponseLeaderEnterExit(const CEvent&, CPedGroup*, CPed*)` (0x5F9710). With a leader,
/// every follower not in a vehicle gets a stack `CTaskSimpleWaitUntilLeaderAreaCodesMatch(leader)` (base
/// constructor, vtable 0x86C7E8, +0x24 = +0x34 = 0); when its `ProcessPed(follower)` already finishes, the follower's
/// flag +0x1C |= 1 instead of the task. Returns 0.
#[hook(addr = 0x5F9710, conv = cdecl, name = "CGroupEventHandler::ComputeResponseLeaderEnterExit", expect = "55 8B EC 83 E4 F8")]
pub fn geh_compute_response_leader_enter_exit(_e: P, pg: P, _orig: P) -> P {
    let intel = intel_of(pg);
    let leader = p(pg, 0x28);
    if leader.is_null() {
        return GPtr::NULL;
    }
    for k in 0..7 {
        let m = member(pg, k);
        if m.is_null() || rd::<u32>(m, 0x46C) & 0x100 != 0 {
            continue;
        }
        let mut lt = Local::new();
        let t = lt.ptr();
        wait_area_ctor(t, leader);
        wr::<u32>(t, 0, 0x86C7E8);
        wr::<u8>(t, 0x24, 0);
        wr::<u8>(t, 0x34, 0);
        if wait_leader_area_process(t, m) != 0 {
            wr::<u32>(m, 0x1C, rd::<u32>(m, 0x1C) | 1);
        } else {
            set_response(intel, m, t);
        }
        wr::<u32>(t, 0, 0x86C7E8);
        wait_area_dtor(t);
    }
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeResponseNewGangMember(const CEventNewGangMember&, CPedGroup*, CPed*)` (0x5F9840). When
/// the new member (event +0xC) is in the group: a sequence `CTaskComplexLeaveAnyCar` (pool; +0xC = 0, +0x10 = 0,
/// +0x11 = 1), `CTaskComplexGangJoinRespond(true)` and, when the leader is the player,
/// `CTaskSimpleGoToPoint(6, player position, 3.0f, true, false)`. Returns 0.
#[hook(addr = 0x5F9840, conv = cdecl, name = "CGroupEventHandler::ComputeResponseNewGangMember", expect = "6A FF 68 61 E2 83 00")]
pub fn geh_compute_response_new_gang_member(e: P, pg: P, _orig: P) -> P {
    let ped = p(e, 0xC);
    let intel = intel_of(pg);
    if ped.is_null() {
        return GPtr::NULL;
    }
    if !(0..7).any(|k| member(pg, k) == ped) && p(pg, 0x28) != ped {
        return GPtr::NULL;
    }
    let mut ls = Local::new();
    let seq = ls.ptr();
    seq_ctor(seq);
    let t = new_task(0x14, |t| {
        task_complex_ctor(t);
        wr::<u32>(t, 0, VT_LEAVE_ANY_CAR);
        wr::<u32>(t, 0xC, 0);
        wr::<u8>(t, 0x10, 0);
        wr::<u8>(t, 0x11, 1);
        t
    });
    seq_add(seq, t);
    let t = new_task(0x14, |t| gang_join_respond_ctor(t, 1));
    seq_add(seq, t);
    if p(pg, 0x28) == find_player_ped(0) {
        let t = new_task(0x30, |t| go_to_point_ctor(t, 6, pos_of(find_player_ped(0)), 0x4040_0000, 1, 0));
        seq_add(seq, t);
    }
    set_response(intel, ped, seq);
    seq_dtor(seq);
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeResponseGather(const CEvent&, CPedGroup*, CPed* originator)` (0x5F99F0): every
/// follower gets `CTaskComplexSeekEntity<CEntitySeekPosCalculatorStandard>(originator, 50000, 1000, 7.5f,
/// [0x859E30], [0x859E34], true, true)`; the stack task's destructor is inlined (vtable 0x859DF8, reference of
/// +0xC cleaned, calculator vtable 0x859DC4 at +0x40, `CTaskComplex` destructor). Returns 0.
#[hook(addr = 0x5F99F0, conv = cdecl, name = "CGroupEventHandler::ComputeResponseGather", expect = "6A FF 68 90 E2 83 00")]
pub fn geh_compute_response_gather(_e: P, pg: P, orig: P) -> P {
    let intel = intel_of(pg);
    for k in 0..7 {
        let m = member(pg, k);
        if m.is_null() {
            continue;
        }
        let a = g::<u32>(0x859E34).read();
        let b = g::<u32>(0x859E30).read();
        let mut lt = Local::new();
        let t = lt.ptr();
        seek_entity_ctor(t, orig, 50000, 1000, 0x40F0_0000, b, a, 1, 1);
        set_response(intel, m, t);
        wr::<u32>(t, 0, 0x859DF8);
        let ent = p(t, 0xC);
        if !ent.is_null() {
            entity_clean_up_old_reference(ent, t.byte_add(0xC));
        }
        wr::<u32>(t, 0x40, 0x859DC4);
        task_complex_dtor(t);
    }
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeLeanOnVehicleResponse(const CEventLeanOnVehicle&, CPedGroup*, CPed*)` (0x5F9B20): the
/// leader gets `CTaskGoToVehicleAndLean(event+0xC, event+0x10)`. Returns 0.
#[hook(addr = 0x5F9B20, conv = cdecl, name = "CGroupEventHandler::ComputeLeanOnVehicleResponse", expect = "64 A1 00 00 00 00")]
pub fn geh_compute_lean_on_vehicle_response(e: P, pg: P, _orig: P) -> P {
    let leader = p(pg, 0x28);
    let intel = intel_of(pg);
    if leader.is_null() {
        return GPtr::NULL;
    }
    let mut lt = Local::new();
    let t = lt.ptr();
    lean_ctor(t, p(e, 0xC), rd::<u32>(e, 0x10));
    set_response(intel, leader, t);
    lean_dtor(t);
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeStareResponse(CPedGroup*, CPed* stareAt, CPed*, int32 timeout, int32 bias)` (0x5F9BD0):
/// within the squared distance [0x859A44] of the leader, every member gets `CTaskComplexStareAtPed(pg, stareAt,
/// timeout - bias + _ftol((bias - -bias) * (rand * 2^-15)))` (one rand per member, return address 0x5F9C85; the span
/// is the left operand of the product). Returns 0.
#[hook(addr = 0x5F9BD0, conv = cdecl, name = "CGroupEventHandler::ComputeStareResponse", expect = "64 A1 00 00 00 00")]
pub fn geh_compute_stare_response(pg: P, stare_at: P, _orig: P, timeout: i32, bias: i32) -> P {
    if stare_at.is_null() {
        return GPtr::NULL;
    }
    let intel = intel_of(pg);
    let leader = p(pg, 0x28);
    if leader.is_null() || !le_m(dist_sq(pos_of(stare_at), pos_of(leader)), 0x859A44) {
        return GPtr::NULL;
    }
    for k in 0..8 {
        let m = member(pg, k);
        if m.is_null() {
            continue;
        }
        let neg = bias.wrapping_neg();
        let r = at_site(0x5F9C85, rand) & 0xFFFF;
        let scaled = xi(r).fmul_m(g::<f32>(0x858B14).read());
        let span = xi(bias.wrapping_sub(neg));
        let time = ftol32(span.fmul(scaled)).wrapping_add(neg).wrapping_add(timeout);
        let mut lt = Local::new();
        let t = lt.ptr();
        stare_ctor(t, pg, stare_at, time);
        set_response(intel, m, t);
        stare_dtor(t);
    }
    GPtr::NULL
}

/// The follower closest to `ped` for `ComputeDoDealResponse` / `ComputeGreetResponse`: `VectorSub(ped - member)`,
/// `z*z + y*y + x*x` of the stored f32 vector, strictly smaller than the stored f32 minimum (from 1e6); the
/// response needs a closest member with a minimum not below [0x8599D0] and not above [0x858628].
fn closest_partner(pg: P, ped: P) -> P {
    let mut best = GPtr::<u8>::NULL;
    let mut min = f32::from_bits(0x4974_23F0);
    for k in 0..7 {
        let m = member(pg, k);
        if m.is_null() {
            continue;
        }
        let mut v = [0u32; 3];
        let vp: P = GPtr::from_raw(&mut v as *mut [u32; 3]).cast();
        vector_sub(vp, pos_of(ped), pos_of(m));
        let (vx, vy, vz) = (rd::<f32>(vp, 0), rd::<f32>(vp, 4), rd::<f32>(vp, 8));
        let d = x(vz).fmul_m(vz).fadd(x(vy).fmul_m(vy)).fadd(x(vx).fmul_m(vx));
        if d.fcom(x(min)) == sa_math::x87::Cmp::Less {
            min = f32_of(d);
            best = m;
        }
    }
    if best.is_null()
        || x(min).fcom(x(g::<f32>(0x8599D0).read())) == sa_math::x87::Cmp::Less
        || !le_m(x(min), 0x858628)
    {
        return GPtr::NULL;
    }
    best
}

/// The common gate of `ComputeDoDealResponse` / `ComputeGreetResponse`: the ped has an active task 0x390, the leader
/// exists and stands still (+0x534 == 1), and the ped's +0x598 is neither 0 nor 1.
fn partner_gate(pg: P, ped: P) -> bool {
    if task_manager_find_active(p(ped, 0x47C).byte_add(4), 0x390).is_null() {
        return false;
    }
    let leader = p(pg, 0x28);
    if leader.is_null() || rd::<i32>(leader, 0x534) != 1 {
        return false;
    }
    let s = rd::<u32>(ped, 0x598);
    s != 0 && s != 1
}

/// After the partner task: a stack `CEventCreatePartnerTask(ty, closest, false, 0.0f)` added to the ped's event group
/// (intelligence +0x68), every other member looks at the closest one (`GetRandomNumberInRange(3000, 6000)` ms,
/// `g_ikChainMan.LookAt(tag, member, closest, time, 5, null, true, 0.25f, 500, 3, false)`), `pg+0 = ped`.
fn partner_finish(pg: P, ped: P, closest: P, ty: i32, tag: u32, ev: P) {
    create_partner_event_ctor(ev, ty, closest, 0, 0);
    event_group_add(p(ped, 0x47C).byte_add(0x68), ev, 0);
    for k in 0..8 {
        let m = member(pg, k);
        if m.is_null() || m == closest {
            continue;
        }
        let time = random_in_range(3000, 6000);
        ik_look_at(GPtr::from_addr(0xC15448), tag, m, closest, time, 5, 0, 1, 0x3E80_0000, 500, 3, 0);
    }
    wr::<P>(pg, 0, ped);
    create_partner_event_dtor(ev);
}

/// `CGroupEventHandler::ComputeDoDealResponse(CPedGroup*, CPed* dealWith, CPed*)` (0x5FA290): the closest follower gets
/// `CTaskComplexPartnerDeal(tag 0x86C828, dealWith, true, 0.0f, CVector{})`; then the partner event (type 0), the
/// look-ats and `pg+0 = dealWith`. No rand before the look-ats (gta-reversed draws one). Returns 0.
#[hook(addr = 0x5FA290, conv = cdecl, name = "CGroupEventHandler::ComputeDoDealResponse", expect = "64 A1 00 00 00 00")]
pub fn geh_compute_do_deal_response(pg: P, deal_with: P, _orig: P) -> P {
    if deal_with.is_null() || !partner_gate(pg, deal_with) {
        return GPtr::NULL;
    }
    let intel = intel_of(pg);
    let closest = closest_partner(pg, deal_with);
    if closest.is_null() {
        return GPtr::NULL;
    }
    let mut lt = Local::new();
    let t = lt.ptr();
    partner_deal_ctor(t, 0x86C828, deal_with, 1, 0, 0, 0, 0);
    set_response(intel, closest, t);
    let mut le = Local::new();
    partner_finish(pg, deal_with, closest, 0, 0x86C828, le.ptr());
    partner_deal_dtor(t);
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeGreetResponse(CPedGroup*, CPed* toGreet, CPed*)` (0x5FA550): `r =
/// GetRandomNumberInRange(0, 6)`; the closest follower gets `CTaskComplexPartnerGreet(tag 0x86C838, toGreet, true,
/// 0.0f, r, CVector{})`; then the partner event (type r + 1), the look-ats and `pg+0 = toGreet`. Returns 0.
#[hook(addr = 0x5FA550, conv = cdecl, name = "CGroupEventHandler::ComputeGreetResponse", expect = "64 A1 00 00 00 00")]
pub fn geh_compute_greet_response(pg: P, to_greet: P, _orig: P) -> P {
    if to_greet.is_null() || !partner_gate(pg, to_greet) {
        return GPtr::NULL;
    }
    let intel = intel_of(pg);
    let closest = closest_partner(pg, to_greet);
    if closest.is_null() {
        return GPtr::NULL;
    }
    let r = random_in_range(0, 6);
    let mut lt = Local::new();
    let t = lt.ptr();
    partner_greet_ctor(t, 0x86C838, to_greet, 1, 0, r, 0, 0, 0);
    set_response(intel, closest, t);
    let mut le = Local::new();
    partner_finish(pg, to_greet, closest, r.wrapping_add(1), 0x86C838, le.ptr());
    partner_greet_dtor(t);
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeResponseSexyPed(const CEventSexyPed&, CPedGroup*, CPed*)` (0x5FB390): nothing for a
/// member, the player's group or a player-group member; task 0x5E0 -> `ComputeStareResponse(pg, sexyPed, originator,
/// GetRandomNumberInRange(3000, 5000), 1000)`, 0x5E4 -> `ComputeHassleSexyPedResponse`, else 0.
#[hook(addr = 0x5FB390, conv = cdecl, name = "CGroupEventHandler::ComputeResponseSexyPed", expect = "8B 44 24 04 53")]
pub fn geh_compute_response_sexy_ped(e: P, pg: P, orig: P) -> P {
    let task_id = rd::<i16>(e, 0xE) as i32;
    let ped = p(e, 0x14);
    if ped.is_null() {
        return GPtr::NULL;
    }
    if (0..7).any(|k| member(pg, k) == ped) || p(pg, 0x28) == ped {
        return GPtr::NULL;
    }
    let player_group = |off: usize| {
        let gi = rd::<u32>(p(find_player_ped(-1), 0x480), 0x38) as usize;
        GPtr::from_addr(gi.wrapping_mul(0x2D4).wrapping_add(off))
    };
    if pg == player_group(0xC09920) || membership_is_member(player_group(0xC09928), ped) != 0 {
        return GPtr::NULL;
    }
    match task_id {
        0x5E0 => {
            let time = random_in_range(3000, 5000);
            geh_stare(pg, ped, orig, time, 1000)
        }
        0x5E4 => geh_hassle_sexy_ped(pg, ped, orig),
        _ => GPtr::NULL,
    }
}

/// `CGroupEventHandler::ComputeHassleThreatResponse(CPedGroup*, CPed* threat, CPed*, bool)` (0x5F9D50). Only within
/// the squared distance [0x86C824] of the leader. A threat in an automobile (subtype 0): unless the flag is set,
/// each member gets `CTaskGangHassleVehicle(veh, -1, 0, 0.25f, 0.65f)` while the vehicle has a spare hassle
/// position; otherwise each member gets `CTaskGangHasslePed(threat, flag ? 2 : 1, 12000, 20000)`. Returns 0.
#[hook(addr = 0x5F9D50, conv = cdecl, name = "CGroupEventHandler::ComputeHassleThreatResponse", expect = "64 A1 00 00 00 00")]
pub fn geh_compute_hassle_threat_response(pg: P, threat: P, _orig: P, flag: u32) -> P {
    if threat.is_null() {
        return GPtr::NULL;
    }
    let intel = intel_of(pg);
    let leader = p(pg, 0x28);
    if leader.is_null() {
        return GPtr::NULL;
    }
    if !le_m(dist_sq(pos_of(threat), pos_of(leader)), 0x86C824) {
        return GPtr::NULL;
    }
    let flag = flag as u8;
    let tv = p(threat, 0x58C);
    if rd::<u32>(threat, 0x46C) & 0x100 != 0 && !tv.is_null() && rd::<u32>(tv, 0x594) == 0 {
        if flag != 0 {
            return GPtr::NULL;
        }
        for k in 0..8 {
            let m = member(pg, k);
            if m.is_null() || veh_spare_hassle_pos(p(threat, 0x58C)) == -1 {
                continue;
            }
            let mut lt = Local::new();
            let t = lt.ptr();
            hassle_veh_ctor(t, p(threat, 0x58C), -1, 0, 0x3E80_0000, 0x3F26_6666);
            set_response(intel, m, t);
            hassle_veh_dtor(t);
        }
    } else {
        for k in 0..8 {
            let m = member(pg, k);
            if m.is_null() {
                continue;
            }
            let mut lt = Local::new();
            let t = lt.ptr();
            hassle_ped_ctor(t, threat, if flag != 0 { 2 } else { 1 }, 12000, 20000);
            set_response(intel, m, t);
            hassle_ped_dtor(t);
        }
    }
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeHassleSexyPedResponse(CPedGroup*, CPed* sexyPed, CPed*)` (0x5FA020): one rand per
/// follower slot (null slots included, return address 0x5FA06A); when `_ftol(r * 2^-15 * [0x858628]) > 75` and the
/// slot holds a ped, it gets `CTaskGangHasslePed(sexyPed, 0, 8000, 12000)`. Returns 0.
#[hook(addr = 0x5FA020, conv = cdecl, name = "CGroupEventHandler::ComputeHassleSexyPedResponse", expect = "64 A1 00 00 00 00")]
pub fn geh_compute_hassle_sexy_ped_response(pg: P, sexy: P, _orig: P) -> P {
    if sexy.is_null() {
        return GPtr::NULL;
    }
    let intel = intel_of(pg);
    for k in 0..7 {
        let v = rand_scaled(0x5FA06A, 0x858628);
        if v <= 0x4B {
            continue;
        }
        let m = member(pg, k);
        if m.is_null() {
            continue;
        }
        let mut lt = Local::new();
        let t = lt.ptr();
        hassle_ped_ctor(t, sexy, 0, 8000, 12000);
        set_response(intel, m, t);
        hassle_ped_dtor(t);
    }
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeFleePedResponse(CPedGroup*, CPed* threat, CPed*, bool)` (0x5FA130; the entry continues
/// through the island 0x402061). Within the squared distance [0x859A44] of the leader, each member draws a rand
/// (unused, return address 0x5FA1E5) and gets `CTaskComplexSmartFleeEntity(threat, false, 60.0f, 10000,
/// [0x86F678], [0xC18CF0])`. The flag is unused. Returns 0.
#[hook(addr = 0x5FA130, conv = cdecl, name = "CGroupEventHandler::ComputeFleePedResponse", expect = "55 8B EC 83 E4 F8")]
pub fn geh_compute_flee_ped_response(pg: P, threat: P, _orig: P, _flag: u32) -> P {
    if threat.is_null() {
        return GPtr::NULL;
    }
    let leader = p(pg, 0x28);
    let intel = intel_of(pg);
    if leader.is_null() {
        return GPtr::NULL;
    }
    if !le_m(dist_sq(pos_of(threat), pos_of(leader)), 0x859A44) {
        return GPtr::NULL;
    }
    for k in 0..8 {
        let m = member(pg, k);
        if m.is_null() {
            continue;
        }
        at_site(0x5FA1E5, rand);
        let a = g::<u32>(0xC18CF0).read();
        let b = g::<u32>(0x86F678).read();
        let mut lt = Local::new();
        let t = lt.ptr();
        flee_entity_ctor(t, threat, 0, 0x4270_0000, 10000, b, a);
        set_response(intel, m, t);
        flee_entity_dtor(t);
    }
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeHandSignalResponse(CPedGroup*, CPed*, CPed*)` (0x5FA820): every member that is not
/// the player gets `CTaskComplexSignalAtPed(ped, -1, false)`. Returns 0.
#[hook(addr = 0x5FA820, conv = cdecl, name = "CGroupEventHandler::ComputeHandSignalResponse", expect = "55 8B EC 83 E4 F8")]
pub fn geh_compute_hand_signal_response(pg: P, ped: P, _orig: P) -> P {
    if ped.is_null() {
        return GPtr::NULL;
    }
    let intel = intel_of(pg);
    for k in 0..8 {
        let m = member(pg, k);
        if m.is_null() || ped_is_player(m) != 0 {
            continue;
        }
        let mut lt = Local::new();
        let t = lt.ptr();
        signal_ctor(t, ped, -1, 0);
        set_response(intel, m, t);
        signal_dtor(t);
    }
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeWalkAlongsideResponse(CPedGroup*, CPed*, CPed*)` (0x5FA910; unused by the game): the
/// follower closest to `ped` (strictly smaller squared distance, compared against the stored f32 minimum, starting
/// at 1e6) gets `CTaskComplexWalkAlongsidePed(ped, 10.0f)` and `pg+0 = ped`. Returns 0.
#[hook(addr = 0x5FA910, conv = cdecl, name = "CGroupEventHandler::ComputeWalkAlongsideResponse", expect = "64 A1 00 00 00 00")]
pub fn geh_compute_walk_alongside_response(pg: P, ped: P, _ped2: P) -> P {
    if ped.is_null() {
        return GPtr::NULL;
    }
    let intel = intel_of(pg);
    let mut best = GPtr::<u8>::NULL;
    let mut min = f32::from_bits(0x4974_23F0);
    for k in 0..7 {
        let m = member(pg, k);
        if m.is_null() {
            continue;
        }
        let d = dist_sq(pos_of(ped), pos_of(m));
        if d.fcom(x(min)) == sa_math::x87::Cmp::Less {
            min = f32_of(d);
            best = m;
        }
    }
    if best.is_null() {
        return GPtr::NULL;
    }
    let mut lt = Local::new();
    let t = lt.ptr();
    walk_alongside_ctor(t, ped, 0x4120_0000);
    set_response(intel, best, t);
    wr::<P>(pg, 0, ped);
    walk_alongside_dtor(t);
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeMemberResponses(const CEvent&, CPedGroup*, CPed*)` (0x5FAA50; entry `push -1; jmp
/// 0x401AE9`). The event is cloned; every alive member: when the clone's +0x24 virtual is true, its task id
/// becomes 200 and `ComputeResponseTaskType(member, true)` runs; the member's response task
/// (`CEventHandler::ComputeEventResponseTask(member, clone)`) is set and deleted. The clone is deleted. Returns 0.
#[hook(addr = 0x5FAA50, conv = cdecl, name = "CGroupEventHandler::ComputeMemberResponses", expect = "6A FF E9 92 70 E0 FF")]
pub fn geh_compute_member_responses(e: P, pg: P, _orig: P) -> P {
    let intel = intel_of(pg);
    let clone = v_event_clone(e);
    for k in 0..8 {
        let m = member(pg, k);
        if m.is_null() || ped_is_alive(m) == 0 {
            continue;
        }
        if v_event_24(clone) != 0 {
            wr::<i16>(clone, 0xE, 0xC8);
            editable_compute_response_task_type(clone, m, 1);
        }
        let t = handler_compute_event_response_task(m, clone);
        if t.is_null() {
            continue;
        }
        set_response(intel, m, t);
        v_delete(t, 1);
    }
    if !clone.is_null() {
        v_delete(clone, 1);
    }
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeResponsePedFriend(const CEventAcquaintancePed&, CPedGroup*, CPed*)` (0x5FB2D0). Nothing
/// for the player's group (`CPedGroups::ms_groups[player->+0x480->+0x38]`), a player-group member, this group's
/// +0 ped or a member; task 0x5E2 -> `ComputeDoDealResponse`, 0x5E3 -> `ComputeGreetResponse`, else 0.
#[hook(addr = 0x5FB2D0, conv = cdecl, name = "CGroupEventHandler::ComputeResponsePedFriend", expect = "8B 44 24 04 53")]
pub fn geh_compute_response_ped_friend(e: P, pg: P, orig: P) -> P {
    let task_id = rd::<i16>(e, 0xE) as i32;
    let ped = p(e, 0x14);
    if ped.is_null() {
        return GPtr::NULL;
    }
    let player_group = |off: usize| {
        let gi = rd::<u32>(p(find_player_ped(-1), 0x480), 0x38) as usize;
        GPtr::from_addr(gi.wrapping_mul(0x2D4).wrapping_add(off))
    };
    if pg == player_group(0xC09920)
        || membership_is_member(player_group(0xC09928), ped) != 0
        || p(pg, 0) == ped
        || membership_is_member(pg.byte_add(8), ped) != 0
    {
        return GPtr::NULL;
    }
    match task_id {
        0x5E2 => geh_do_deal(pg, ped, orig),
        0x5E3 => geh_greet(pg, ped, orig),
        _ => GPtr::NULL,
    }
}

/// `CGroupEventHandler::ComputeResponsePlayerCommand(const CEventPlayerCommandToGroup&, CPedGroup*, CPed*)` (0x5FB470):
/// command 0 with a target: `new CTaskAllocatorPlayerCommandAttack(target, groupId or -1, target+0x598)`; command 2:
/// `ComputeResponseGather`; else 0.
#[hook(addr = 0x5FB470, conv = cdecl, name = "CGroupEventHandler::ComputeResponsePlayerCommand", expect = "64 A1 00 00 00 00")]
pub fn geh_compute_response_player_command(e: P, pg: P, orig: P) -> P {
    let cmd = rd::<i32>(e, 0x14);
    let target = p(e, 0x18);
    match cmd {
        0 => {
            if target.is_null() {
                return GPtr::NULL;
            }
            let mut id = -1;
            let g = groups_get_peds_group(target);
            if !g.is_null() {
                id = groups_get_group_id(g);
            }
            let a = alloc_new(0x20);
            if a.is_null() {
                return GPtr::NULL;
            }
            alloc_cmd_attack_ctor(a, target, id, rd::<u32>(target, 0x598))
        }
        2 => geh_gather(e, pg, orig),
        _ => GPtr::NULL,
    }
}

/// `CGroupEventHandler::ComputeResponseDanger(const CEventDanger&, CPedGroup*, CPed*)` (0x5FB540): a ped source with
/// task 0x5E1 -> `ComputeFleePedResponse(pg, source, originator, false)`, else 0.
#[hook(addr = 0x5FB540, conv = cdecl, name = "CGroupEventHandler::ComputeResponseDanger", expect = "56 8B 74 24 08")]
pub fn geh_compute_response_danger(e: P, pg: P, orig: P) -> P {
    let src = v_event_source(e);
    if src.is_null() || !is_ped(src) || rd::<i16>(e, 0xE) != 0x5E1 {
        return GPtr::NULL;
    }
    geh_flee_ped(pg, src, orig, 0)
}

/// `CGroupEventHandler::ComputeKillThreatsBasicResponse(CPedGroup*, CPed* threat, CPed*, bool)` (0x5FB590): when a kill
/// task is not appropriate, `ComputeFleePedResponse(pg, threat, originator, false)`; else a new
/// `CTaskAllocatorKillThreatsBasic(threat)`, re-tabled as `...BasicRandomGroup` (vtable 0x86C80C) for a non-mission
/// group. The flag is unused.
#[hook(addr = 0x5FB590, conv = cdecl, name = "CGroupEventHandler::ComputeKillThreatsBasicResponse", expect = "6A FF 68 86 E4 83 00")]
pub fn geh_compute_kill_threats_basic_response(pg: P, threat: P, orig: P, _flag: u32) -> P {
    if geh_is_kill_task_appropriate(pg, threat) == 0 {
        return geh_flee_ped(pg, threat, orig, 0);
    }
    if rd::<u8>(pg, 0x2D0) == 0 {
        let a = alloc_new(0x14);
        if a.is_null() {
            return GPtr::NULL;
        }
        alloc_kill_basic_ctor(a, threat);
        wr::<u32>(a, 0, 0x86C80C);
        a
    } else {
        let a = alloc_new(0x14);
        if a.is_null() {
            return GPtr::NULL;
        }
        alloc_kill_basic_ctor(a, threat)
    }
}

/// `CGroupEventHandler::ComputeKillPlayerBasicResponse(CPedGroup*, CPed* threat, CPed*, bool)` (0x5FB670): when a kill
/// task is not appropriate, `ComputeFleePedResponse(pg, threat, originator, false)`; else every member gets
/// `CTaskComplexKillPedOnFoot(threat, -1, 0, 0, 0, 1)` and the result is 0.
#[hook(addr = 0x5FB670, conv = cdecl, name = "CGroupEventHandler::ComputeKillPlayerBasicResponse", expect = "55 8B EC 83 E4 F8")]
pub fn geh_compute_kill_player_basic_response(pg: P, threat: P, orig: P, _flag: u32) -> P {
    if threat.is_null() {
        return GPtr::NULL;
    }
    if geh_is_kill_task_appropriate(pg, threat) == 0 {
        return geh_flee_ped(pg, threat, orig, 0);
    }
    let intel = intel_of(pg);
    for k in 0..8 {
        let m = member(pg, k);
        if m.is_null() {
            continue;
        }
        let mut lt = Local::new();
        let t = lt.ptr();
        kill_on_foot_ctor(t, threat, -1, 0, 0, 0, 1);
        set_response(intel, m, t);
        kill_on_foot_dtor(t);
    }
    GPtr::NULL
}

/// `CGroupEventHandler::ComputeResponsePedThreat(const CEventAcquaintancePed&, CPedGroup*, CPed*)` (0x5FBB90): nothing
/// for a member; task 0x5DE kill-threats, 0x5E0 stare (99999999, 0), 0x5E1 flee, 0x5E5 / 0x5ED hassle (flag
/// true / false), 0x5E6 member responses, 0x5EC drive-by (a tail jump in the original), else 0.
#[hook(addr = 0x5FBB90, conv = cdecl, name = "CGroupEventHandler::ComputeResponsePedThreat", expect = "53 8B 5C 24 08")]
pub fn geh_compute_response_ped_threat(e: P, pg: P, orig: P) -> P {
    let threat = p(e, 0x14);
    let task_id = rd::<i16>(e, 0xE) as i32;
    if threat.is_null() {
        return GPtr::NULL;
    }
    if (0..7).any(|k| member(pg, k) == threat) || p(pg, 0x28) == threat {
        return GPtr::NULL;
    }
    match task_id {
        0x5DE => geh_kill_threats_basic(pg, threat, orig, 0),
        0x5E0 => geh_stare(pg, threat, orig, 99_999_999, 0),
        0x5E1 => geh_flee_ped(pg, threat, orig, 0),
        0x5E5 => geh_hassle_threat(pg, threat, orig, 1),
        0x5E6 => geh_member_responses(e, pg, orig),
        0x5EC => geh_driveby(pg, threat, orig),
        0x5ED => geh_hassle_threat(pg, threat, orig, 0),
        _ => GPtr::NULL,
    }
}

/// `CGroupEventHandler::ComputeResponseSeenCop(const CEventSeenCop&, CPedGroup*, CPed*)` (0x5FBCB0): task 0x5DE
/// kill-threats, 0x5E1 flee, 0x5EB hand signal (no ped check), else 0.
#[hook(addr = 0x5FBCB0, conv = cdecl, name = "CGroupEventHandler::ComputeResponseSeenCop", expect = "8B 4C 24 04 8B 51 14")]
pub fn geh_compute_response_seen_cop(e: P, pg: P, orig: P) -> P {
    let cop = p(e, 0x14);
    match rd::<i16>(e, 0xE) as i32 {
        0x5DE => geh_kill_threats_basic(pg, cop, orig, 0),
        0x5E1 => geh_flee_ped(pg, cop, orig, 0),
        0x5EB => geh_hand_signal(pg, cop, orig),
        _ => GPtr::NULL,
    }
}

/// The `MaybeAdjustTaskOfGroupThreatEvent` check inlined in `ComputeResponseGunAimedAt` / `ComputeResponseDamage`:
/// a non-mission group threatened by the player whose leader holds a non-melee weapon and is not friendly with the
/// player switches the event's task id to 0x5DE.
fn maybe_adjust_task(e: P, pg: P, src: P) {
    if rd::<u8>(pg, 0x2D0) != 0 || ped_is_player(src) == 0 {
        return;
    }
    let leader = p(pg, 0x28);
    if leader.is_null() || weapon_is_melee(active_weapon(leader)) != 0 {
        return;
    }
    if intel_is_friendly_with(p(leader, 0x47C), src) == 0 {
        wr::<i16>(e, 0xE, 0x5DE);
    }
}

/// `CGroupEventHandler::ComputeResponseGunAimedAt(const CEventGunAimedAt&, CPedGroup*, CPed*)` (0x5FBD10): for a ped
/// source (`GetSourceEntity`), the task-id adjustment, then 0x5DE kill-threats, 0x5E1 flee, 0x5E6 member
/// responses, else 0.
#[hook(addr = 0x5FBD10, conv = cdecl, name = "CGroupEventHandler::ComputeResponseGunAimedAt", expect = "53 55 8B 6C 24 0C")]
pub fn geh_compute_response_gun_aimed_at(e: P, pg: P, orig: P) -> P {
    let src = v_event_source(e);
    if src.is_null() || !is_ped(src) {
        return GPtr::NULL;
    }
    maybe_adjust_task(e, pg, src);
    match rd::<i16>(e, 0xE) as i32 {
        0x5DE => geh_kill_threats_basic(pg, src, orig, 0),
        0x5E1 => geh_flee_ped(pg, src, orig, 0),
        0x5E6 => geh_member_responses(e, pg, orig),
        _ => GPtr::NULL,
    }
}

/// `CGroupEventHandler::ComputeResponseShotFired(const CEventGunShot&, CPedGroup*, CPed*)` (0x5FBDF0): for a ped
/// shooter (+0x14), 0x5DE kill-threats, 0x5E1 flee, 0x5E6 member responses, else 0.
#[hook(addr = 0x5FBDF0, conv = cdecl, name = "CGroupEventHandler::ComputeResponseShotFired", expect = "56 8B 74 24 08")]
pub fn geh_compute_response_shot_fired(e: P, pg: P, orig: P) -> P {
    let src = p(e, 0x14);
    if src.is_null() || !is_ped(src) {
        return GPtr::NULL;
    }
    match rd::<i16>(e, 0xE) as i32 {
        0x5DE => geh_kill_threats_basic(pg, src, orig, 0),
        0x5E1 => geh_flee_ped(pg, src, orig, 0),
        0x5E6 => geh_member_responses(e, pg, orig),
        _ => GPtr::NULL,
    }
}

/// `CGroupEventHandler::ComputeResponseDraggedOutCar(const CEventDraggedOutCar&, CPedGroup*, CPed*)` (0x5FBE70): for a
/// ped carjacker (+0x14): 0x5DE flees when the carjacker is the player, the originator respects it and the group
/// is not a mission group, else kill-threats; 0x5E1 flee; 0x5E6 member responses and 0x5E7 leader-exited-car (tail
/// jumps in the original); else 0.
#[hook(addr = 0x5FBE70, conv = cdecl, name = "CGroupEventHandler::ComputeResponseDraggedOutCar", expect = "8B 4C 24 04 53")]
pub fn geh_compute_response_dragged_out_car(e: P, pg: P, orig: P) -> P {
    let src = p(e, 0x14);
    if src.is_null() || !is_ped(src) {
        return GPtr::NULL;
    }
    match rd::<i16>(e, 0xE) as i32 {
        0x5DE => {
            if ped_is_player(src) != 0
                && !orig.is_null()
                && intel_respects(p(orig, 0x47C), src) != 0
                && rd::<u8>(pg, 0x2D0) == 0
            {
                geh_flee_ped(pg, src, orig, 0)
            } else {
                geh_kill_threats_basic(pg, src, orig, 0)
            }
        }
        0x5E1 => geh_flee_ped(pg, src, orig, 0),
        0x5E6 => geh_member_responses(e, pg, orig),
        0x5E7 => geh_leader_exited_car(e, pg, orig),
        _ => GPtr::NULL,
    }
}

/// `CGroupEventHandler::ComputeResponseDamage(const CEventDamage&, CPedGroup*, CPed*)` (0x5FBF50): the task id is read
/// *before* the task-id adjustment (the switch uses the old value; the adjustment only changes the event); for a
/// ped attacker (+0x14): 0x5DE kill-threats, 0x5DF kill-player, 0x5E1 flee (flag true), 0x5E6 member responses,
/// else 0.
#[hook(addr = 0x5FBF50, conv = cdecl, name = "CGroupEventHandler::ComputeResponseDamage", expect = "51 8B 44 24 08")]
pub fn geh_compute_response_damage(e: P, pg: P, orig: P) -> P {
    let task_id = rd::<i16>(e, 0xE) as i32;
    let src = p(e, 0x14);
    if src.is_null() || !is_ped(src) {
        return GPtr::NULL;
    }
    maybe_adjust_task(e, pg, src);
    match task_id {
        0x5DE => geh_kill_threats_basic(pg, src, orig, 1),
        0x5DF => geh_kill_player_basic(pg, src, orig, 1),
        0x5E1 => geh_flee_ped(pg, src, orig, 1),
        0x5E6 => geh_member_responses(e, pg, orig),
        _ => GPtr::NULL,
    }
}

/// `CGroupEventHandler::ComputeResponseVehicleDamage(const CEventVehicleDamage&, CPedGroup*, CPed*)` (0x5FC070): for a
/// ped attacker (+0x18): 0x5DE kill-threats, 0x5DF kill-player, 0x5E1 flee (flag true), 0x5E6 member responses,
/// 0x5EC drive-by, else 0.
#[hook(addr = 0x5FC070, conv = cdecl, name = "CGroupEventHandler::ComputeResponseVehicleDamage", expect = "56 8B 74 24 08")]
pub fn geh_compute_response_vehicle_damage(e: P, pg: P, orig: P) -> P {
    let src = p(e, 0x18);
    if src.is_null() || !is_ped(src) {
        return GPtr::NULL;
    }
    match rd::<i16>(e, 0xE) as i32 {
        0x5DE => geh_kill_threats_basic(pg, src, orig, 1),
        0x5DF => geh_kill_player_basic(pg, src, orig, 1),
        0x5E1 => geh_flee_ped(pg, src, orig, 1),
        0x5E6 => geh_member_responses(e, pg, orig),
        0x5EC => geh_driveby(pg, src, orig),
        _ => GPtr::NULL,
    }
}

/// `CGroupEventHandler::ComputeEventResponseTasks(const CEventGroupEvent&, CPedGroup*)` (0x5FC200): dispatches on the
/// base event's type (`ge+0x10`, originator `ge+0xC`) to the response functions (jump table 0x5FC398 / index
/// 0x5FC3E0, types 7..92); other types give 0.
#[hook(addr = 0x5FC200, conv = cdecl, name = "CGroupEventHandler::ComputeEventResponseTasks", expect = "8B 44 24 04 53")]
pub fn geh_compute_event_response_tasks(ge: P, pg: P) -> P {
    let orig = p(ge, 0xC);
    let e = p(ge, 0x10);
    match v_event_type(e) {
        7 => geh_dragged_out_car(e, pg, orig),
        9 => geh_damage(e, pg, orig),
        15 => geh_shot_fired(e, pg, orig),
        20 => geh_sexy_ped(e, pg, orig),
        31 => geh_gun_aimed_at(e, pg, orig),
        36 | 37 => geh_ped_threat(e, pg, orig),
        38 | 39 => geh_ped_friend(e, pg, orig),
        41 | 73 => geh_vehicle_damage(e, pg, orig),
        54 => geh_leader_entered_car(e, pg, orig),
        55 => geh_leader_exited_car(e, pg, orig),
        57 => geh_leader_quit_entering_car(e, pg, orig),
        71 | 76 => geh_player_command(e, pg, orig),
        72 => geh_seen_cop(e, pg, orig),
        75 => geh_danger(e, pg, orig),
        77 => geh_leader_enter_exit(e, pg, orig),
        90 => geh_new_gang_member(e, pg, orig),
        92 => geh_lean_on_vehicle(e, pg, orig),
        _ => GPtr::NULL,
    }
}

/// `CEventHandler::ComputeChatPartnerResponse(CEvent*, CTask*, CTask*)` (0x4B98E0). Without an event nothing is
/// written; without a partner (event +0x10) or when the ped already has a `CTaskComplexPartnerChat` (0x4B4),
/// `m_EventResponseTask` (+0x24) = null; otherwise `GetRandomNumberInRange(3, 7)` is drawn and discarded (return
/// address 0x4B993D) and the response is `new CTaskComplexPartnerChat("...", partner, event+0xC, 0.5f, 4, true,
/// 0, CVector{})` (tag 0x85B9F8). The lead flag byte is passed zero-extended (the original pushes its argument slot
/// with the event pointer's upper bytes; the constructors read the low byte only).
#[hook(addr = 0x4B98E0, conv = thiscall, name = "CEventHandler::ComputeChatPartnerResponse", expect = "64 A1 00 00 00 00")]
pub fn event_handler_compute_chat_partner_response(this: P, e: P, _a: P, _b: P) {
    if e.is_null() {
        return;
    }
    let partner = p(e, 0x10);
    let lead = rd::<u8>(e, 0xC) as u32;
    if partner.is_null() || !intel_find_task(p(p(this, 0), 0x47C), 0x4B4).is_null() {
        wr::<u32>(this, 0x24, 0);
        return;
    }
    random_in_range(3, 7);
    let t = new_task(0x78, |t| partner_chat_ctor(t, 0x85B9F8, partner, lead, 0x3F00_0000, 4, 1, 0, 0, 0, 0));
    wr::<P>(this, 0x24, t);
}

/// `CEventHandler::ComputeSignalAtPedResponse(CEvent*, CTask*, CTask*)` (0x4BB050): `m_EventResponseTask` = `new
/// CTaskComplexSignalAtPed(event+0xC, event+0x10, (bool)event+0x14)` (null when the allocation fails).
#[hook(addr = 0x4BB050, conv = thiscall, name = "CEventHandler::ComputeSignalAtPedResponse", expect = "6A FF 68 0B AD 83 00")]
pub fn event_handler_compute_signal_at_ped_response(this: P, e: P, _a: P, _b: P) {
    let t = new_task(0x18, |t| signal_ctor(t, p(e, 0xC), rd::<i32>(e, 0x10), rd::<u8>(e, 0x14) as u32));
    wr::<P>(this, 0x24, t);
}

/// `CEventHandler::ComputeCreatePartnerTaskResponse(CEvent*, CTask*, CTask*)` (0x4BB130). Without a partner (event
/// +0x10) the response is null; type (+0xC) 0: `new CTaskComplexPartnerDeal(tag, partner, (bool)+0x14, +0x18,
/// CVector{})`; types 1..6: `new CTaskComplexPartnerGreet(tag, partner, (bool)+0x14, +0x18, type - 1, CVector{})`
/// (tag 0x85BA28; null when the allocation fails); any other type leaves `m_EventResponseTask` unchanged.
#[hook(addr = 0x4BB130, conv = thiscall, name = "CEventHandler::ComputeCreatePartnerTaskResponse", expect = "64 A1 00 00 00 00")]
pub fn event_handler_compute_create_partner_task_response(this: P, e: P, _a: P, _b: P) {
    let partner = p(e, 0x10);
    if partner.is_null() {
        wr::<u32>(this, 0x24, 0);
        return;
    }
    let ty = rd::<i32>(e, 0xC);
    if ty == 0 {
        let t = new_task(0x74, |t| {
            partner_deal_ctor(t, 0x85BA28, p(e, 0x10), rd::<u8>(e, 0x14) as u32, rd::<u32>(e, 0x18), 0, 0, 0)
        });
        wr::<P>(this, 0x24, t);
    } else if (1..=6).contains(&ty) {
        let t = new_task(0x78, |t| {
            let kind = rd::<i32>(e, 0xC) - 1;
            partner_greet_ctor(t, 0x85BA28, p(e, 0x10), rd::<u8>(e, 0x14) as u32, rd::<u32>(e, 0x18), kind, 0, 0, 0)
        });
        wr::<P>(this, 0x24, t);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_GEH_COMPUTE_DRIVEBY_RESPONSE,
    HOOK_GEH_IS_KILL_TASK_APPROPRIATE_PORT,
    HOOK_GEH_COMPUTE_RESPONSE_LEADER_ENTERED_CAR,
    HOOK_GEH_COMPUTE_RESPONSE_LEADER_EXITED_CAR,
    HOOK_GEH_COMPUTE_RESPONS_LEADER_QUIT_ENTERING_CAR,
    HOOK_GEH_COMPUTE_RESPONSE_LEADER_ENTER_EXIT,
    HOOK_GEH_COMPUTE_RESPONSE_NEW_GANG_MEMBER,
    HOOK_GEH_COMPUTE_RESPONSE_GATHER,
    HOOK_GEH_COMPUTE_LEAN_ON_VEHICLE_RESPONSE,
    HOOK_GEH_COMPUTE_STARE_RESPONSE,
    HOOK_GEH_COMPUTE_DO_DEAL_RESPONSE,
    HOOK_GEH_COMPUTE_GREET_RESPONSE,
    HOOK_GEH_COMPUTE_RESPONSE_SEXY_PED,
    HOOK_GEH_COMPUTE_HASSLE_THREAT_RESPONSE,
    HOOK_GEH_COMPUTE_HASSLE_SEXY_PED_RESPONSE,
    HOOK_GEH_COMPUTE_FLEE_PED_RESPONSE,
    HOOK_GEH_COMPUTE_HAND_SIGNAL_RESPONSE,
    HOOK_GEH_COMPUTE_WALK_ALONGSIDE_RESPONSE,
    HOOK_GEH_COMPUTE_MEMBER_RESPONSES,
    HOOK_GEH_COMPUTE_RESPONSE_PED_FRIEND,
    HOOK_GEH_COMPUTE_RESPONSE_PLAYER_COMMAND,
    HOOK_GEH_COMPUTE_RESPONSE_DANGER,
    HOOK_GEH_COMPUTE_KILL_THREATS_BASIC_RESPONSE,
    HOOK_GEH_COMPUTE_KILL_PLAYER_BASIC_RESPONSE,
    HOOK_GEH_COMPUTE_RESPONSE_PED_THREAT,
    HOOK_GEH_COMPUTE_RESPONSE_SEEN_COP,
    HOOK_GEH_COMPUTE_RESPONSE_GUN_AIMED_AT,
    HOOK_GEH_COMPUTE_RESPONSE_SHOT_FIRED,
    HOOK_GEH_COMPUTE_RESPONSE_DRAGGED_OUT_CAR,
    HOOK_GEH_COMPUTE_RESPONSE_DAMAGE,
    HOOK_GEH_COMPUTE_RESPONSE_VEHICLE_DAMAGE,
    HOOK_GEH_COMPUTE_EVENT_RESPONSE_TASKS,
    HOOK_EVENT_HANDLER_COMPUTE_CHAT_PARTNER_RESPONSE,
    HOOK_EVENT_HANDLER_COMPUTE_SIGNAL_AT_PED_RESPONSE,
    HOOK_EVENT_HANDLER_COMPUTE_CREATE_PARTNER_TASK_RESPONSE,
];
