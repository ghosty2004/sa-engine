//! `CPedIntelligence` bodies not ported by PORT-AI-1/2 (PORT-AI-5): constructor, destructor, `operator new`, the
//! per-frame `Process`/`ProcessFirst`/`ProcessAfterProcCol`/`ProcessAfterPreRender`, `ClearTasks`,
//! `FlushImmediately`, `FlushIntelligence`, `ProcessStaticCounter`, `LookAtInterestingEntities`, the seeing-range
//! tests, and `CPlayerRelationshipRecorder::RecordRelationshipWithPlayer`. Ported from the 1.0 US bytes (the exe
//! wins over gta-reversed). Every call to another catalogued function goes through `orig!` at the original
//! address.
//!
//! `CPedIntelligence` (0x294 bytes): +0 the ped, +4 `CTaskManager` (primary tasks +4..+0x14), +0x34
//! `CEventHandler`, +0x68 `CEventGroup`, +0xBC hearing range, +0xC0 seeing range, +0xD4 / +0x124 the vehicle and
//! ped `CEntityScanner`s, +0x18C `CEventScanner`, +0x1A8 its attractor scanner, +0x260 / +0x264 the collision event
//! scanner and the stuck checker, +0x274 / +0x278 static counters, +0x27C the last static position,
//! +0x288..+0x290 the three interesting-entity slots.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::*;

/// `CLoadMonitor` instance the AI process functions time themselves with.
const LOAD_MONITOR: usize = 0xB72978;
/// `0.0f` (.rdata)
const F_ZERO: usize = 0x858B50;

orig! {
    /// `CPedIntelligencePool::New` (0x605EC0, thiscall on the pool)
    fn intel_pool_new(pool: u32) -> GPtr<u8> = thiscall 0x605EC0 => unported;
    /// `CLoadMonitor::StartTimer(int32)` (0x53D050, `ret 4`)
    fn load_monitor_start(m: GPtr<u8>, i: u32) = thiscall 0x53D050 => unported;
    /// `CLoadMonitor::EndTimer(int32)` (0x53D070, `ret 4`)
    fn load_monitor_end(m: GPtr<u8>, i: u32) = thiscall 0x53D070 => unported;
    /// `CTaskManager::CTaskManager(CPed*)` (0x6816A0, `ret 4`)
    fn tm_ctor(tm: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6816A0 => unported;
    /// `CTaskManager::~CTaskManager` (0x6816D0)
    fn tm_dtor(tm: GPtr<u8>) = thiscall 0x6816D0 => unported;
    /// `CTaskManager::Flush` (0x681850)
    fn tm_flush(tm: GPtr<u8>) = thiscall 0x681850 => unported;
    /// `CTaskManager::FlushImmediately` (0x6818A0)
    fn tm_flush_immediately(tm: GPtr<u8>) = thiscall 0x6818A0 => unported;
    /// `CTaskManager::ManageTasks` (0x681C10)
    fn tm_manage_tasks(tm: GPtr<u8>) = thiscall 0x681C10 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn tm_get_task_secondary(tm: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskManager::SetTaskSecondary(CTask*, int32)` (0x681B60, `ret 8`)
    fn tm_set_task_secondary(tm: GPtr<u8>, t: GPtr<u8>, i: i32) = thiscall 0x681B60 => unported;
    /// `CTaskManager::SetTask(CTask*, int32, bool)` (0x681AF0, `ret 0xC`)
    fn tm_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: i32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CTaskManager::GetSimplestActiveTask` (0x6819D0)
    fn tm_get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTaskManager::GetLastTaskOf` (at index; 0x681A00, `ret 4`)
    fn tm_get_simplest_task_at(tm: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x681A00 => unported;
    /// `CTaskManager::GetActiveTask` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CEventHandler::CEventHandler(CPed*)` (0x4C3E80, `ret 4`)
    fn eh_ctor(eh: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x4C3E80 => unported;
    /// `CEventHandler::~CEventHandler` (0x4C3EC0)
    fn eh_dtor(eh: GPtr<u8>) = thiscall 0x4C3EC0 => unported;
    /// `CEventHandler::HandleEvents` (0x4C3F10)
    fn eh_handle_events(eh: GPtr<u8>) = thiscall 0x4C3F10 => unported;
    /// `CEventHandler::FlushImmediately` (0x4C3820)
    fn eh_flush_immediately(eh: GPtr<u8>) = thiscall 0x4C3820 => unported;
    /// `CEventHandlerHistory::ClearAllEvents` (0x4BC550)
    fn ehh_clear_all_events(h: GPtr<u8>) = thiscall 0x4BC550 => unported;
    /// `CEventGroup::CEventGroup(CPed*)` (0x4AB340, `ret 4`)
    fn eg_ctor(eg: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x4AB340 => unported;
    /// `CEventGroup::~CEventGroup` (0x4AB980)
    fn eg_dtor(eg: GPtr<u8>) = thiscall 0x4AB980 => unported;
    /// `CEventGroup::Flush(bool)` (0x4AB370, `ret 4`)
    fn eg_flush(eg: GPtr<u8>, b: u32) = thiscall 0x4AB370 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn eg_add(eg: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEventGroup::HasScriptCommandOfTaskType(int32)` (0x4AB840, `ret 4`)
    fn eg_has_script_command_of_task_type(eg: GPtr<u8>, ty: i32) -> u8 = thiscall 0x4AB840 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32, CTask*, bool)` (0x4B0A00, `ret 0xC`)
    fn script_command_ctor(e: GPtr<u8>, index: u32, task: GPtr<u8>, dead: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand` (0x4B0A50)
    fn script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CEventScanner::CEventScanner` (0x605300)
    fn event_scanner_ctor(s: GPtr<u8>) = thiscall 0x605300 => unported;
    /// `CEventScanner::ScanForEvents(CPed&)` (0x607E30, `ret 4`)
    fn event_scanner_scan(s: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x607E30 => unported;
    /// `CEntityScanner::Clear` (0x5FF9D0)
    fn entity_scanner_clear(s: GPtr<u8>) = thiscall 0x5FF9D0 => unported;
    /// `CEntityScanner::ScanForEntitiesInRange(int32, const CPed&)` (0x5FFA20, `ret 8`)
    fn entity_scanner_scan(s: GPtr<u8>, ty: u32, ped: GPtr<u8>) = thiscall 0x5FFA20 => unported;
    /// `CVehicleScanner::ScanForVehiclesInRange(const CPed&)` (0x5FFE60, `ret 4`)
    fn vehicle_scanner_scan(s: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x5FFE60 => unported;
    /// `CAttractorScanner::Clear` (0x5FFF90)
    fn attractor_scanner_clear(s: GPtr<u8>) = thiscall 0x5FFF90 => unported;
    /// `CPedStuckChecker::TestPedStuck(CPed*, CEventGroup*)` (0x602C00, `ret 8`)
    fn stuck_checker_test(s: GPtr<u8>, ped: GPtr<u8>, eg: GPtr<u8>) -> u8 = thiscall 0x602C00 => unported;
    /// `CCollisionEventScanner::ScanForCollisionEvents(CPed*, CEventGroup*)` (0x604500, `ret 8`)
    fn collision_scanner_scan(s: GPtr<u8>, ped: GPtr<u8>, eg: GPtr<u8>) = thiscall 0x604500 => unported;
    /// `GetPlayerRelationshipRecorder()` (0x61A2E0, cdecl)
    fn get_player_relationship_recorder() -> GPtr<u8> = cdecl 0x61A2E0 => unported;
    /// `CPlayerRelationshipRecorder::ClearRelationshipWithPlayer(const CPed*)` (0x61A150, `ret 4`)
    fn rec_clear(r: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x61A150 => unported;
    /// `CPlayerRelationshipRecorder::RecordRelationshipWithPlayer(const CPed*)` (0x61A1D0, `ret 4`)
    fn rec_record(r: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x61A1D0 => unported;
    /// `CTaskCategories::IsKillPedTask(const CTask*, bool*, bool*)` (0x6985E0, cdecl)
    fn task_is_kill_ped_task(t: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x6985E0 => unported;
    /// `CTaskCategories::IsFollowPedTask(const CTask*, bool*, bool*)` (0x698610, cdecl)
    fn task_is_follow_ped_task(t: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x698610 => unported;
    /// `CPedIntelligence::LookAtInterestingEntities` (0x6020D0)
    fn intel_look_at_interesting_entities(i: GPtr<u8>) = thiscall 0x6020D0 => unported;
    /// `CPedIntelligence::ProcessStaticCounter` (0x605650)
    fn intel_process_static_counter(i: GPtr<u8>) = thiscall 0x605650 => unported;
    /// `CPedIntelligence::IsInterestingEntity(CEntity*)` (0x6020A0, `ret 4`)
    fn intel_is_interesting_entity(i: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x6020A0 => unported;
    /// `CPedScriptedTaskRecord::Process` (0x6085E0, cdecl)
    fn scripted_task_record_process() = cdecl 0x6085E0 => unported;
    /// `CPed::IsAlive` (0x5E0170)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80, cdecl)
    fn ped_groups_get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupIntelligence::ComputeDefaultTasks(CPed*)` (0x5F88D0, `ret 4`)
    fn pgi_compute_default_tasks(i: GPtr<u8>, p: GPtr<u8>) = thiscall 0x5F88D0 => unported;
    /// `CTaskSimpleCarDriveTimed::CTaskSimpleCarDriveTimed(CVehicle*, int32)` (0x5FF940, `ret 8`)
    fn car_drive_timed_ctor(t: GPtr<u8>, veh: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x5FF940 => unported;
    /// `CTaskSimpleCarDrive::CTaskSimpleCarDrive(CVehicle*, CTaskUtilityLineUpPedWithCar*, bool)` (0x63C340, `ret 0xC`)
    fn car_drive_ctor(t: GPtr<u8>, veh: GPtr<u8>, u: u32, b: u32) -> GPtr<u8> = thiscall 0x63C340 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, ms: u32, looped: u32, b: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskSimplePlayerOnFoot::CTaskSimplePlayerOnFoot` (0x685750)
    fn player_on_foot_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x685750 => unported;
    /// `CTaskComplexWander::GetWanderTaskByPedType(CPed*)` (0x673D00, cdecl)
    fn wander_task_by_ped_type(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x673D00 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70, `ret 4`)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40, `ret 4`)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CEntity::UpdateRwFrame` (0x532B00)
    fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CEntity::GetIsOnScreen` (0x534540)
    fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `GetAnimHierarchyFromSkinClump(RpClump*)` (0x734A40, cdecl)
    fn get_anim_hierarchy_from_skin_clump(c: u32) -> u32 = cdecl 0x734A40 => unported;
    /// `RpHAnimIDGetIndex(RpHAnimHierarchy*, int32)` (0x7C51A0, cdecl)
    fn hanim_id_get_index(h: u32, id: u32) -> i32 = cdecl 0x7C51A0 => unported;
    /// `RpHAnimHierarchyGetMatrixArray(RpHAnimHierarchy*)` (0x7C5120, cdecl)
    fn hanim_get_matrix_array(h: u32) -> u32 = cdecl 0x7C5120 => unported;
    /// `RwV3dTransformPoint(RwV3d* out, const RwV3d* in, const RwMatrix*)` (0x7EDD60, cdecl)
    fn rw_v3d_transform_point(out: GPtr<u8>, inp: GPtr<u8>, m: u32) -> GPtr<u8> = cdecl 0x7EDD60 => unported;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0, cdecl)
    fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18A0 => unported;
    /// `FxSystem_c::SetMatrix(RwMatrix*)` (0x4AA630, `ret 4`)
    fn fx_system_set_matrix(fx: GPtr<u8>, m: GPtr<u8>) = thiscall 0x4AA630 => unported;
    /// `CBike::FixHandsToBars(CPed*)` (0x6B7F90, `ret 4`)
    fn bike_fix_hands_to_bars(b: GPtr<u8>, p: GPtr<u8>) = thiscall 0x6B7F90 => unported;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360, cdecl)
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => unported;
    /// `CIKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(m: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `CIKChainManager_c::LookAt(char*, CPed*, CEntity*, int32, int32, RwV3d*, bool, float, int32, int32, bool)`
    /// (0x618970, `ret 0x2C`)
    #[allow(clippy::too_many_arguments)]
    fn ik_look_at(m: GPtr<u8>, purpose: u32, ped: GPtr<u8>, target: u32, time: i32, bone: i32, offset: GPtr<u8>,
        use_offset: u32, speed: u32, blend: u32, prio: u32, b: u32) = thiscall 0x618970 => unported;
    /// `CWorld::FindObjectsInRange(const CVector&, float, bool, int16*, int16, CEntity**, bool, bool, bool, bool,
    /// bool)` (0x564A20, cdecl)
    #[allow(clippy::too_many_arguments)]
    fn world_find_objects_in_range(pos: GPtr<u8>, range: u32, b2d: u32, count: GPtr<u8>, max: u32, out: GPtr<u8>,
        b: u32, v: u32, p: u32, o: u32, d: u32) = cdecl 0x564A20 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180, cdecl; its `rand` returns to 0x407187)
    fn random_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CTask::Clone` (vtable +4)
    fn v_clone(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CTask::IsSimple` (vtable +0xC)
    fn v_is_simple(t: GPtr<u8>) -> u8 = thiscall vslot 0xC => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: u32, e: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskSimple::ProcessPed(CPed*)` (vtable +0x1C)
    fn v_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall vslot 0x1C => unported;
    /// `CTaskSimple::SetPedPosition(CPed*)` (vtable +0x20)
    fn v_set_ped_position(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall vslot 0x20 => unported;
    /// scalar deleting destructor (vtable +0)
    fn v_delete(t: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CEventHandlerHistory::IsRespondingToEvent(int32)` (0x4B8B90, `ret 4`)
    fn ehh_is_responding_to_event(h: GPtr<u8>, ty: i32) -> u8 = thiscall 0x4B8B90 => unported;
    /// `CPedIntelligence::IsFriendlyWith(const CPed&)` (0x601BC0, `ret 4`)
    fn intel_is_friendly_with(i: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x601BC0 => unported;
    /// `CPhysical::GetLightingTotal` (0x544850, ST0)
    fn physical_get_lighting_total(p: GPtr<u8>) -> f64 = thiscall 0x544850 => unported;
    /// `CTaskSimpleDuck::CTaskSimpleDuck(eDuckControlType, uint16, int16)` (0x691FC0, `ret 0xC`)
    fn duck_ctor(t: GPtr<u8>, control: u32, len: u32, shots: u32) -> GPtr<u8> = thiscall 0x691FC0 => unported;
    /// `CTaskSimpleUseGun::ClearAnim(CPed*)` (0x61E190, `ret 4`)
    fn use_gun_clear_anim(t: GPtr<u8>, p: GPtr<u8>) = thiscall 0x61E190 => unported;
    /// `CPedIntelligence::GetTaskUseGun` (0x600F70)
    fn intel_get_task_use_gun(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F70 => unported;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00, `ret 4`)
    fn ped_set_move_state(p: GPtr<u8>, s: u32) = thiscall 0x5DEC00 => unported;
    /// `CEventGroup::HasEvent(CEvent*)` (0x4AB6A0, `ret 4`)
    fn eg_has_event(eg: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x4AB6A0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CScriptedBrainTaskStore::GetTask(const CPed*)` (0x6357C0, cdecl)
    fn brain_task_store_get_task(p: GPtr<u8>) -> u32 = cdecl 0x6357C0 => unported;
    /// `CPedScriptedTaskRecordData::Flush` (0x608350)
    fn task_record_data_flush(r: GPtr<u8>) = thiscall 0x608350 => unported;
    /// `CPedGroupIntelligence::GetTaskScriptCommand(CPed*)` (0x5F8690, `ret 4`)
    fn pgi_get_task_script_command(i: GPtr<u8>, p: GPtr<u8>) -> u32 = thiscall 0x5F8690 => unported;
    /// `CTaskManager::HasTaskSecondary(const CTask*)` (0x681820, `ret 4`)
    fn tm_has_task_secondary(tm: GPtr<u8>, t: u32) -> u8 = thiscall 0x681820 => unported;
}

fn ped(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0)
}

fn tm(this: GPtr<u8>) -> GPtr<u8> {
    this.byte_add(4)
}

fn lm() -> GPtr<u8> {
    GPtr::from_addr(LOAD_MONITOR)
}

/// A placeable's position: `m_matrix->pos` (+0x14 -> +0x30) when it has a matrix, else the simple transform
/// (+4).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `CPedIntelligence::operator new(uint32)` (0x6074A0): `CPedIntelligencePool::New` on `CPools::ms_pPedIntelligencePool`
/// (0xB744C0; tail jump, the size is not used).
// Hooked by sa-ped (ped_life.rs, PED-2); kept here as a plain function for callers in this crate.
pub fn intel_operator_new(_size: u32) -> GPtr<u8> {
    intel_pool_new(g::<u32>(0xB744C0).read())
}

/// `CPedIntelligence::CPedIntelligence(CPed*)` (0x607140, `ret 4`): the task manager, event handler and event
/// group, the ranges from the pedstats defaults (0x86C928 / 0x86C934; 0x86C930 for ped types 7..16, read twice),
/// the two entity scanners (vtables 0x86CD84 / 0x86CD88, 16 zeroed slots each), the event scanner. Returns this.
#[hook(addr = 0x607140, conv = thiscall, name = "CPedIntelligence::Constructor", expect = "6A FF 68 76 EA 83 00")]
pub fn intel_ctor(this: GPtr<u8>, p: GPtr<u8>) -> GPtr<u8> {
    wr(this, 0, p);
    tm_ctor(tm(this), p);
    eh_ctor(this.byte_add(0x34), p);
    eg_ctor(this.byte_add(0x68), p);
    wr::<u32>(this, 0xB4, 0xFFFF_FFFF);
    wr::<u32>(this, 0xB8, 0xFFFF_FFFF);
    wr::<u32>(this, 0xBC, g::<u32>(0x86C928).read());
    wr::<u32>(this, 0xC0, g::<u32>(0x86C928).read());
    wr::<u32>(this, 0xC4, 3);
    wr::<u32>(this, 0xC8, 0x41700000);
    wr::<u32>(this, 0xCC, g::<u32>(0x86C934).read());
    wr::<u8>(this, 0xD0, 0xFF);
    wr::<u8>(this, 0xD1, 0);
    wr::<u8>(this, 0xD2, 0);
    wr::<u32>(this, 0xD8, 0);
    wr::<u32>(this, 0xDC, 0);
    for i in 0..16 {
        wr::<u32>(this, 0xE0 + 4 * i, 0);
    }
    wr::<u32>(this, 0x120, 0);
    wr::<u32>(this, 0xDC, 0x10);
    wr::<u32>(this, 0xD4, 0x86CD84);
    wr::<u32>(this, 0x128, 0);
    wr::<u32>(this, 0x12C, 0);
    for i in 0..16 {
        wr::<u32>(this, 0x130 + 4 * i, 0);
    }
    wr::<u32>(this, 0x170, 0);
    wr::<u32>(this, 0x12C, 0x10);
    wr::<u32>(this, 0x124, 0x86CD88);
    wr::<u8>(this, 0x174, 0);
    wr::<u32>(this, 0x178, 0);
    wr::<u32>(this, 0x17C, 0);
    wr::<u8>(this, 0x180, 0);
    wr::<u8>(this, 0x181, 0);
    wr::<u8>(this, 0x188, 0);
    wr::<u32>(this, 0x178, g::<u32>(0xB7CB84).read());
    wr::<u32>(this, 0x17C, 0);
    wr::<u8>(this, 0x180, 1);
    event_scanner_ctor(this.byte_add(0x18C));
    wr::<u8>(this, 0x260, 0);
    wr::<u16>(this, 0x270, 0);
    wr::<u16>(this, 0x272, 0);
    wr::<u32>(this, 0x274, 0);
    wr::<u32>(this, 0x278, 0);
    let ty = rd::<i32>(p, 0x598);
    if (7..=0x10).contains(&ty) {
        wr::<u32>(this, 0xC0, g::<u32>(0x86C930).read());
        wr::<u32>(this, 0xBC, g::<u32>(0x86C930).read());
    }
    wr::<u32>(this, 0x288, 0);
    wr::<u32>(this, 0x28C, 0);
    wr::<u32>(this, 0x290, 0);
    this
}

/// `CPedIntelligence::~CPedIntelligence` (0x607300): clears the ped from the player relationship recorder, resets
/// both entity scanners (base vtable 0x86CD40, `Clear`), destroys the event group, event handler and task manager.
/// The original leaves the task manager destructor's EAX; no caller reads it.
#[hook(addr = 0x607300, conv = thiscall, name = "CPedIntelligence::Destructor", expect = "6A FF 68 BD EA 83 00")]
pub fn intel_dtor(this: GPtr<u8>) {
    let p = ped(this);
    let r = get_player_relationship_recorder();
    rec_clear(r, p);
    wr::<u32>(this, 0x124, 0x86CD40);
    entity_scanner_clear(this.byte_add(0x124));
    wr::<u32>(this, 0xD4, 0x86CD40);
    entity_scanner_clear(this.byte_add(0xD4));
    eg_dtor(this.byte_add(0x68));
    eh_dtor(this.byte_add(0x34));
    tm_dtor(tm(this));
}

/// `CPedIntelligence::Process` (0x608260): the scanners (entity scan only for a live ped), event scanning, event
/// handling, task management, the player relationship record, then `LookAtInterestingEntities`.
#[hook(addr = 0x608260, conv = thiscall, name = "CPedIntelligence::Process", expect = "56 57 8B F1 6A 00")]
pub fn intel_process(this: GPtr<u8>) {
    load_monitor_start(lm(), 0);
    vehicle_scanner_scan(this.byte_add(0xD4), ped(this));
    let p = ped(this);
    if ped_is_alive(p) != 0 {
        entity_scanner_scan(this.byte_add(0x124), 1, p);
    }
    event_scanner_scan(this.byte_add(0x18C), ped(this));
    eh_handle_events(this.byte_add(0x34));
    tm_manage_tasks(tm(this));
    let p = ped(this);
    let r = get_player_relationship_recorder();
    rec_record(r, p);
    intel_look_at_interesting_entities(this);
    load_monitor_end(lm(), 0);
}

/// `CPedIntelligence::ProcessFirst` (0x6073A0): the static counter, the stuck checker (+0x264), the collision
/// event scanner (+0x260) when not stuck; sets ped flag 0x4000 (+0x478) when the ped is standing on a moving
/// non-object entity (+0xD8 > 0.0, +0xDC set, its type != 3) and the contact normal (+0xE0) dotted with the ped's
/// forward vector is below [0x858F40]; clears a bike's +0x7B4 / +0x7B5 for a ped in a vehicle of type 9; clears
/// ped flag 0x10000 (+0x478).
#[hook(addr = 0x6073A0, conv = thiscall, name = "CPedIntelligence::ProcessFirst", expect = "56 57 8B F1 6A 00")]
pub fn intel_process_first(this: GPtr<u8>) {
    load_monitor_start(lm(), 0);
    intel_process_static_counter(this);
    let eg = this.byte_add(0x68);
    if stuck_checker_test(this.byte_add(0x264), ped(this), eg) == 0 {
        collision_scanner_scan(this.byte_add(0x260), ped(this), eg);
    }
    let p = ped(this);
    if x(rd::<f32>(p, 0xD8)).fcom(x(g::<f32>(F_ZERO).read())) == Cmp::Greater {
        let e = rd::<GPtr<u8>>(p, 0xDC);
        if !e.is_null() && rd::<u8>(e, 0x36) & 7 != 3 {
            let m = rd::<GPtr<u8>>(p, 0x14);
            let d = x(rd::<f32>(p, 0xE8))
                .fmul_m(rd::<f32>(m, 0x18))
                .fadd(x(rd::<f32>(p, 0xE4)).fmul_m(rd::<f32>(m, 0x14)))
                .fadd(x(rd::<f32>(p, 0xE0)).fmul_m(rd::<f32>(m, 0x10)));
            if !d.fcom(x(g::<f32>(0x858F40).read())).test_parity(5) {
                wr::<u32>(p, 0x478, rd::<u32>(p, 0x478) | 0x4000);
            }
        }
    }
    let p = ped(this);
    if rd::<u32>(p, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(p, 0x58C);
        if !v.is_null() && rd::<u32>(v, 0x590) == 9 {
            wr::<u8>(v, 0x7B4, 0);
            let v = rd::<GPtr<u8>>(ped(this), 0x58C);
            wr::<u8>(v, 0x7B5, 0);
        }
    }
    let p = ped(this);
    wr::<u32>(p, 0x478, rd::<u32>(p, 0x478) & 0xFFFE_FFFF);
    load_monitor_end(lm(), 0);
}

/// Adds a `CEventScriptCommand(3, task, false)` built on the stack (0x18 bytes) to the ped's event group, then
/// destroys the stack event.
fn add_script_command(this: GPtr<u8>, task: GPtr<u8>) {
    let mut ev = [0u32; 6];
    let e: GPtr<u8> = GPtr::from_raw(&mut ev as *mut [u32; 6]).cast();
    script_command_ctor(e, 3, task, 0);
    eg_add(this.byte_add(0x68), e, 0);
    script_command_dtor(e);
}

/// `CPedIntelligence::ClearTasks(bool primary, bool secondary)` (0x601420, `ret 8`). Primary: a ped in a vehicle
/// gets a script-command event with a `CTaskSimpleCarDriveTimed(vehicle, 0)` when its default task (+0x14, type
/// read without a null check) is a 0x2C5, else a `CTaskSimpleCarDrive(vehicle, 0, 0)`, unless a 0x2C5 script
/// command is already queued; a ped on foot gets a `CTaskSimpleStandStill(0, 0, 0, 8.0)` unless a 0xCB one is
/// queued; then the events are handled, the tasks managed and `CPedScriptedTaskRecord::Process` runs (also when a
/// command was queued already). Secondary: every secondary slot but 3 is asked to abort urgently and cleared when
/// it agrees, else asked non-urgently.
#[hook(addr = 0x601420, conv = thiscall, name = "CPedIntelligence::ClearTasks", expect = "64 A1 00 00 00 00")]
pub fn intel_clear_tasks(this: GPtr<u8>, primary: u8, secondary: u8) {
    if primary != 0 {
        let p = ped(this);
        let eg = this.byte_add(0x68);
        let queued;
        if rd::<u32>(p, 0x46C) & 0x100 != 0 && !rd::<GPtr<u8>>(p, 0x58C).is_null() {
            queued = eg_has_script_command_of_task_type(eg, 0x2C5) != 0;
            if !queued {
                let task = if v_task_type(rd::<GPtr<u8>>(this, 0x14)) == 0x2C5 {
                    let t = task_new(0x70);
                    if t.is_null() {
                        GPtr::NULL
                    } else {
                        car_drive_timed_ctor(t, rd::<GPtr<u8>>(ped(this), 0x58C), 0)
                    }
                } else {
                    let t = task_new(0x60);
                    if t.is_null() {
                        GPtr::NULL
                    } else {
                        car_drive_ctor(t, rd::<GPtr<u8>>(ped(this), 0x58C), 0, 0)
                    }
                };
                add_script_command(this, task);
            }
        } else {
            queued = eg_has_script_command_of_task_type(eg, 0xCB) != 0;
            if !queued {
                let t = task_new(0x20);
                let task = if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 0, 0, 0, 0x41000000) };
                add_script_command(this, task);
            }
        }
        eh_handle_events(this.byte_add(0x34));
        tm_manage_tasks(tm(this));
        scripted_task_record_process();
    }
    if secondary != 0 {
        for i in 0..6 {
            if i == 3 {
                continue;
            }
            let t = tm_get_task_secondary(tm(this), i);
            if t.is_null() {
                continue;
            }
            if v_make_abortable(t, ped(this), 1, 0) != 0 {
                tm_set_task_secondary(tm(this), GPtr::NULL, i);
            } else {
                v_make_abortable(t, ped(this), 0, 0);
            }
        }
    }
}

/// `CPedIntelligence::FlushImmediately(bool setDefaultTask)` (0x601640, `ret 4`). Keeps clones of the primary
/// slot-3 task when it is a 0xF3, of the secondary slot-4 task when it is a 0x133 (holding an object: its +0x13C
/// byte and flag bit 7 of +0x1C are saved and restored on the clone's object) and of the secondary slot-3 task
/// when it is a 0x131; flushes the event group (`Flush(true)`), the event handler and the task manager, runs
/// `CPedScriptedTaskRecord::Process`, then gives the clones back (the 0xF3 one through the ped's group default
/// tasks when the ped is a non-player group member, else it is deleted; the 0x133 one is set and run with
/// `ProcessPed`). With the argument set the ped gets its default task: player on foot, stand still for a ped with
/// +0x484 == 2, else the wander task of its ped type.
#[hook(addr = 0x601640, conv = thiscall, name = "CPedIntelligence::FlushImmediately", expect = "6A FF 68 26 E8 83 00")]
pub fn intel_flush_immediately(this: GPtr<u8>, set_default: u8) {
    let mut d = GPtr::<u8>::NULL;
    let t = rd::<GPtr<u8>>(this, 0x10);
    if !t.is_null() && v_task_type(t) == 0xF3 {
        d = v_clone(t);
    }
    let tmp = tm(this);
    let hold = if !tm_get_task_secondary(tmp, 4).is_null() && v_task_type(tm_get_task_secondary(tmp, 4)) == 0x133 {
        tm_get_task_secondary(tmp, 4)
    } else {
        GPtr::NULL
    };
    let mut saved_byte: u32 = 0xFFFF_FFFF;
    let mut saved_bit: u8 = set_default;
    let mut hold_clone = GPtr::<u8>::NULL;
    if !hold.is_null() && v_task_type(hold) == 0x133 {
        let e = rd::<GPtr<u8>>(hold, 8);
        if !e.is_null() {
            if rd::<u8>(e, 0x36) & 7 == 4 {
                saved_byte = rd::<u8>(e, 0x13C) as u32;
                saved_bit = ((rd::<u32>(e, 0x1C) >> 7) as u8) & 1;
            }
            hold_clone = v_clone(hold);
        }
    }
    let mut sec3 = GPtr::<u8>::NULL;
    let t = tm_get_task_secondary(tmp, 3);
    if !t.is_null() && v_task_type(t) == 0x131 {
        sec3 = v_clone(t);
    }
    eg_flush(this.byte_add(0x68), 1);
    eh_flush_immediately(this.byte_add(0x34));
    tm_flush_immediately(tmp);
    scripted_task_record_process();
    if !d.is_null() {
        let grp = ped_groups_get_peds_group(ped(this));
        if !grp.is_null() && ped_is_player(ped(this)) == 0 {
            pgi_compute_default_tasks(grp.byte_add(0x30), ped(this));
            tm_set_task(tmp, d, 3, 0);
        } else {
            v_delete(d, 1);
        }
    }
    if !hold_clone.is_null() {
        if saved_byte != 0xFFFF_FFFF {
            wr::<u8>(rd::<GPtr<u8>>(hold_clone, 8), 0x13C, saved_byte as u8);
            let o = rd::<GPtr<u8>>(hold_clone, 8);
            let f = rd::<u32>(o, 0x1C);
            wr::<u32>(o, 0x1C, f ^ (((saved_bit as u32) << 7 ^ f) & 0x80));
        }
        tm_set_task_secondary(tmp, hold_clone, 4);
        v_process_ped(hold_clone, ped(this));
    }
    if !sec3.is_null() {
        tm_set_task_secondary(tmp, sec3, 3);
    }
    if set_default != 0 {
        let p = ped(this);
        let task = if ped_is_player(p) != 0 {
            let t = task_new(0x1C);
            if t.is_null() { GPtr::NULL } else { player_on_foot_ctor(t) }
        } else if rd::<u8>(ped(this), 0x484) == 2 {
            let t = task_new(0x20);
            if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 0, 1, 0, 0x41000000) }
        } else {
            wander_task_by_ped_type(ped(this))
        };
        tm_set_task(tmp, task, 4, 0);
    }
}

/// `CPedIntelligence::ProcessAfterProcCol` (0x6018F0): `SetPedPosition` of the simplest active task when it is
/// simple, else (when that returns false) of the simplest task of primary slot 4; when one returned true the
/// ped's RW matrix and frame are updated. Clears ped flag 0x400 (+0x474).
#[hook(addr = 0x6018F0, conv = thiscall, name = "CPedIntelligence::ProcessAfterProcCol", expect = "55 56 57 8B F9")]
pub fn intel_process_after_proc_col(this: GPtr<u8>) {
    load_monitor_start(lm(), 0);
    let t = tm_get_simplest_active_task(tm(this));
    if !t.is_null() && v_is_simple(t) != 0 {
        let mut set = v_set_ped_position(t, ped(this));
        if set == 0 {
            let t2 = tm_get_simplest_task_at(tm(this), 4);
            if !t2.is_null() && v_is_simple(t2) != 0 {
                set = v_set_ped_position(t2, ped(this));
            }
        }
        if set != 0 {
            let p = ped(this);
            let rw = rd::<GPtr<u8>>(p, 0x18);
            if !rw.is_null() {
                let frame_m = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
                let m = rd::<GPtr<u8>>(p, 0x14);
                if !m.is_null() {
                    matrix_update_rw_matrix(m, frame_m);
                } else {
                    simple_transform_update_rw_matrix(p.byte_add(4), frame_m);
                }
            }
            entity_update_rw_frame(ped(this));
        }
    }
    let p = ped(this);
    wr::<u32>(p, 0x474, rd::<u32>(p, 0x474) & 0xFFFF_FBFF);
    load_monitor_end(lm(), 0);
}

/// `CPedIntelligence::ProcessAfterPreRender` (0x6019B0): `SetPedPosition` of the simple secondary slot-4 task, and
/// when it returns true of a 0x133 secondary slot-0 task; of a 0x3F9 secondary slot-0 task (every test re-reads
/// the slot, as the original does); a molotov (weapon type 0x12) with a particle system gets its matrix from bone
/// 0x18 offset by (0.05, 0.05, 0.14); a ped in a bike (vehicle type 9) gets `FixHandsToBars`. The original's
/// RwMatrix / vectors live in its 0x58-byte frame (a Rust local here).
#[hook(addr = 0x6019B0, conv = thiscall, name = "CPedIntelligence::ProcessAfterPreRender", expect = "83 EC 58 53 56")]
pub fn intel_process_after_pre_render(this: GPtr<u8>) {
    load_monitor_start(lm(), 0);
    let tmp = tm(this);
    if !tm_get_task_secondary(tmp, 4).is_null()
        && v_is_simple(tm_get_task_secondary(tmp, 4)) != 0
        && v_set_ped_position(tm_get_task_secondary(tmp, 4), ped(this)) != 0
        && !tm_get_task_secondary(tmp, 0).is_null()
        && v_task_type(tm_get_task_secondary(tmp, 0)) == 0x133
    {
        v_set_ped_position(tm_get_task_secondary(tmp, 0), ped(this));
    }
    if !tm_get_task_secondary(tmp, 0).is_null()
        && v_task_type(tm_get_task_secondary(tmp, 0)) == 0x3F9
        && !tm_get_task_secondary(tmp, 0).is_null()
    {
        let t = if !tm_get_task_secondary(tmp, 0).is_null() && v_task_type(tm_get_task_secondary(tmp, 0)) == 0x3F9 {
            tm_get_task_secondary(tmp, 0)
        } else {
            GPtr::NULL
        };
        v_set_ped_position(t, ped(this));
    }
    let p = ped(this);
    let slot = rd::<i8>(p, 0x718) as isize;
    let weapon = p.byte_add(slot * 0x1C + 0x5A0);
    if rd::<u32>(weapon, 0) == 0x12 && !rd::<GPtr<u8>>(weapon, 0x18).is_null() {
        // The frame from esp+0x10 (F = esp after the four saved registers): in vector at +0x10, out vector at
        // +0x1C, RwMatrix at +0x28 (position at +0x58).
        let mut f = [0u32; 22];
        let fp: GPtr<u8> = GPtr::from_raw(&mut f as *mut [u32; 22]).cast();
        let at_f = |off: isize| fp.byte_add(off - 0x10);
        let h = get_anim_hierarchy_from_skin_clump(rd::<u32>(p, 0x18));
        let idx = hanim_id_get_index(h, 0x18);
        let mats = hanim_get_matrix_array(h);
        wr::<u32>(at_f(0x10), 0, 0x3D4CCCCD);
        wr::<u32>(at_f(0x14), 0, 0x3D4CCCCD);
        wr::<u32>(at_f(0x18), 0, 0x3E0F5C29);
        rw_v3d_transform_point(at_f(0x1C), at_f(0x10), mats.wrapping_add((idx as u32) << 6));
        let rw = rd::<GPtr<u8>>(ped(this), 0x18);
        let src = if rw.is_null() { GPtr::<u8>::NULL } else { rd::<GPtr<u8>>(rw, 4).byte_add(0x10) };
        for i in 0..16 {
            wr::<u32>(at_f(0x28), 4 * i, rd::<u32>(src, 4 * i));
        }
        let (ox, oy, oz) = (rd::<u32>(at_f(0x1C), 0), rd::<u32>(at_f(0x20), 0), rd::<u32>(at_f(0x24), 0));
        wr::<u32>(at_f(0x58), 0, ox);
        wr::<u32>(at_f(0x5C), 0, oy);
        wr::<u32>(at_f(0x60), 0, oz);
        rw_matrix_update(at_f(0x28));
        fx_system_set_matrix(rd::<GPtr<u8>>(weapon, 0x18), at_f(0x28));
    }
    let p = ped(this);
    if rd::<u32>(p, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(p, 0x58C);
        if !v.is_null() && rd::<u32>(v, 0x590) == 9 {
            bike_fix_hands_to_bars(v, p);
        }
    }
    load_monitor_end(lm(), 0);
}

/// `CPedIntelligence::FlushIntelligence` (0x601DA0): flushes the task manager, zeroes +0x54..+0x64, clears the
/// event history, flushes the event group (`Flush(false)`), clears both entity scanners and (tail call) the
/// attractor scanner.
#[hook(addr = 0x601DA0, conv = thiscall, name = "CPedIntelligence::FlushIntelligence", expect = "56 8B F1 57 8D 4E 04")]
pub fn intel_flush_intelligence(this: GPtr<u8>) {
    tm_flush(tm(this));
    for off in [0x54, 0x58, 0x5C, 0x60, 0x64] {
        wr::<u32>(this, off, 0);
    }
    ehh_clear_all_events(this.byte_add(0x38));
    eg_flush(this.byte_add(0x68), 0);
    entity_scanner_clear(this.byte_add(0xD4));
    entity_scanner_clear(this.byte_add(0x124));
    attractor_scanner_clear(this.byte_add(0x1A8));
}

/// `CPedIntelligence::RecordEventForScript(int32 eventId, int32 priority)` (0x602050, `ret 8`): ignores event 0x20;
/// records id/priority bytes (+0xD1/+0xD2) when the id is 0 or the priority is above the recorded one (signed).
#[hook(addr = 0x602050, conv = thiscall, name = "CPedIntelligence::RecordEventForScript", expect = "8B 44 24 04 83 F8 20")]
pub fn intel_record_event_for_script(this: GPtr<u8>, id: i32, prio: i32) {
    if id == 0x20 {
        return;
    }
    if id == 0 || prio > rd::<u8>(this, 0xD2) as i32 {
        wr::<u8>(this, 0xD1, id as u8);
        wr::<u8>(this, 0xD2, prio as u8);
    }
}

/// `CPedIntelligence::LookAtInterestingEntities` (0x6020D0). Only when ped flag 0x400000 (+0x474) is clear, one of
/// the three interesting-entity slots (+0x288) is set, the IK chain manager (0xC15448) is not making the ped look
/// already and the ped is on screen: `GetRandomNumberInRange(0, 100) == 50` collects the objects within 15.0 of the
/// ped (`CWorld::FindObjectsInRange`, up to 0x400, into the original's 0x101C-byte frame: a Rust local here),
/// keeps the interesting ones (compacted into the same frame three slots lower, as the original does), picks one
/// with `GetRandomNumberInRange(0, n)` and looks at it for `GetRandomNumberInRange(3000, 5000)` ms.
#[hook(addr = 0x6020D0, conv = thiscall, name = "CPedIntelligence::LookAtInterestingEntities", expect = "B8 1C 10 00 00")]
pub fn intel_look_at_interesting_entities_port(this: GPtr<u8>) {
    let p = ped(this);
    if rd::<u32>(p, 0x474) & 0x400000 != 0 {
        return;
    }
    if (0..3).all(|i| rd::<u32>(this, 0x288 + 4 * i) == 0) {
        return;
    }
    let ik: GPtr<u8> = GPtr::from_addr(0xC15448);
    if ik_is_looking(ik, p) != 0 || entity_get_is_on_screen(ped(this)) == 0 || random_in_range(0, 100) != 0x32 {
        return;
    }
    // F = esp after the three saved registers; the frame spans F+0x0..F+0x1028 (word index = offset / 4).
    let mut f = [0u32; 0x1028 / 4];
    let fp: GPtr<u8> = GPtr::from_raw(&mut f as *mut [u32; 0x1028 / 4]).cast();
    wr::<u32>(fp, 0xC, 0xFFFF_FFFF);
    let p = ped(this);
    let pos = entity_pos(p);
    world_find_objects_in_range(pos, 0x41700000, 0, fp.byte_add(0xC), 0x400, fp.byte_add(0x28), 0, 1, 1, 1, 0);
    let mut n: i32 = 0;
    let mut i: i32 = 0;
    while i < rd::<i16>(fp, 0xC) as i32 {
        let e = rd::<GPtr<u8>>(fp, 0x28 + 4 * i as isize);
        if intel_is_interesting_entity(this, e) != 0 {
            wr(fp, 0x1C + 4 * n as isize, e);
            n += 1;
        }
        i += 1;
    }
    if n == 0 {
        return;
    }
    let k = random_in_range(0, n);
    let time = random_in_range(3000, 5000);
    let target = rd::<u32>(fp, (0x1C + 4 * k) as isize);
    wr::<u32>(fp, 0x10, 0);
    wr::<u32>(fp, 0x14, 0);
    wr::<u32>(fp, 0x18, 0);
    ik_look_at(ik, 0x86CD60, ped(this), target, time, -1, fp.byte_add(0x10), 0, 0x3E800000, 500, 3, 0);
}

/// `CPedIntelligence::ProcessStaticCounter` (0x605650): counts frames (+0x274) in which a ped doing a go-to task
/// (while the player's own simplest task is a go-to with move state 4, 6 or 7) stays within sqrt([0x86C940]) of the
/// position saved at +0x27C; +0x278 counts frames since the last save (saturating at 4 on wrap), the position is
/// re-saved every 5th frame of a ped on an entity (+0xDC) and whenever the ped moved.
#[hook(addr = 0x605650, conv = thiscall, name = "CPedIntelligence::ProcessStaticCounter", expect = "53 55 56 8B F1")]
pub fn intel_process_static_counter_port(this: GPtr<u8>) {
    let bump = |this: GPtr<u8>| {
        let c = rd::<u32>(this, 0x278).wrapping_add(1);
        wr::<u32>(this, 0x278, c);
        if c == 0xFFFF_FFFF {
            wr::<u32>(this, 0x278, 4);
        }
    };
    let t = tm_get_simplest_active_task(tm(this));
    if t.is_null() || task_is_go_to_task(t) == 0 {
        bump(this);
        wr::<u32>(this, 0x274, 0);
        return;
    }
    let pt = tm_get_simplest_active_task(rd::<GPtr<u8>>(ped(this), 0x47C).byte_add(4));
    let moving = !pt.is_null() && task_is_go_to_task(pt) != 0 && matches!(rd::<u32>(pt, 8), 4 | 6 | 7);
    if !moving {
        bump(this);
        wr::<u32>(this, 0x274, 0);
        return;
    }
    let p = ped(this);
    if rd::<u32>(p, 0xDC) != 0 {
        if rd::<u32>(this, 0x278) > 4 {
            let pos = entity_pos(p);
            for i in 0..3 {
                wr::<u32>(this, 0x27C + 4 * i, rd::<u32>(pos, 4 * i));
            }
        }
        wr::<u32>(this, 0x278, 0);
    } else {
        bump(this);
    }
    if rd::<u32>(this, 0x278) > 4 {
        wr::<u32>(this, 0x274, 0);
        return;
    }
    let pos = entity_pos(p);
    let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(this, 0x27C));
    let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(this, 0x280));
    let dz = x(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(this, 0x284));
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if !d2.fcom(x(g::<f32>(0x86C940).read())).test_parity(5) {
        wr::<u32>(this, 0x274, rd::<u32>(this, 0x274).wrapping_add(1));
        return;
    }
    wr::<u32>(this, 0x274, 0);
    let pos = entity_pos(p);
    for i in 0..3 {
        wr::<u32>(this, 0x27C + 4 * i, rd::<u32>(pos, 4 * i));
    }
}

/// `CPedIntelligence::GetPedFOVRange` (0x4ABEA0): the larger of the hearing (+0xBC) and seeing (+0xC0) ranges
/// (hearing only when strictly greater and ordered), in ST0.
#[hook(addr = 0x4ABEA0, conv = thiscall, name = "CPedIntelligence::GetPedFOVRange", expect = "D9 81 BC 00 00 00")]
pub fn intel_get_ped_fov_range(this: GPtr<u8>) -> X87 {
    if x(rd::<f32>(this, 0xBC)).fcom(x(rd::<f32>(this, 0xC0))).test_any(0x41) {
        x(rd::<f32>(this, 0xC0))
    } else {
        x(rd::<f32>(this, 0xBC))
    }
}

/// `CPedIntelligence::IsInSeeingRange(const CVector&)` (0x600C60, `ret 4`): 1 when the point is strictly within the
/// seeing range (+0xC0, squared) and strictly in front of the ped (dot with the forward vector > 0.0, the matrix
/// read without a null check), else 0 (EAX).
#[hook(addr = 0x600C60, conv = thiscall, name = "CPedIntelligence::IsInSeeingRange", expect = "8B 11 8B 42 14")]
pub fn intel_is_in_seeing_range(this: GPtr<u8>, v: GPtr<u8>) -> u32 {
    let p = ped(this);
    let pos = entity_pos(p);
    let dx = x(rd::<f32>(v, 0)).fsub_m(rd::<f32>(pos, 0));
    let dy = x(rd::<f32>(v, 4)).fsub_m(rd::<f32>(pos, 4));
    let dz = x(rd::<f32>(v, 8)).fsub_m(rd::<f32>(pos, 8));
    let r = x(rd::<f32>(this, 0xC0));
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if r.fmul(r).fcom(d2).test_any(0x41) {
        return 0;
    }
    let m = rd::<GPtr<u8>>(p, 0x14);
    let dot = dz.fmul_m(rd::<f32>(m, 0x18)).fadd(dy.fmul_m(rd::<f32>(m, 0x14))).fadd(dx.fmul_m(rd::<f32>(m, 0x10)));
    if dot.fcom(x(g::<f32>(F_ZERO).read())).test_any(0x41) {
        0
    } else {
        1
    }
}

/// `CPlayerRelationshipRecorder::RecordRelationshipWithPlayer(const CPed*)` (0x61A1D0, `ret 4`): clears the ped's
/// slot (first match of the 16), then for the ped's active task calls `IsKillPedTask`, `IsFollowPedTask` and
/// `IsKillPedTask` again; the original tests the out-flag each call has just cleared (the second `bool*`), so it
/// never records anything (bug kept). The flags live in the original's frame and in its own argument slot (whose
/// low byte it overwrites); Rust locals here.
#[hook(addr = 0x61A1D0, conv = thiscall, name = "CPlayerRelationshipRecorder::RecordRelationshipWithPlayer", expect = "83 EC 08 53 55")]
pub fn rec_record_relationship_with_player(this: GPtr<u8>, p: GPtr<u8>) {
    for i in 0..16 {
        if rd::<GPtr<u8>>(this, 8 * i) == p {
            wr::<u32>(this, 8 * i, 0);
            wr::<u8>(this, 8 * i + 4, 0);
            break;
        }
    }
    let t = tm_get_active_task(rd::<GPtr<u8>>(p, 0x47C).byte_add(4));
    if t.is_null() {
        return;
    }
    let mut fl = [0u8; 8];
    let f: GPtr<u8> = GPtr::from_raw(&mut fl as *mut [u8; 8]).cast();
    task_is_kill_ped_task(t, f.byte_add(0), f.byte_add(1));
    if rd::<u8>(f, 1) == 0 {
        task_is_follow_ped_task(t, f.byte_add(2), f.byte_add(3));
        if rd::<u8>(f, 3) != 0 {
            record(this, p, 3);
            return;
        }
        task_is_kill_ped_task(t, f.byte_add(4), f.byte_add(5));
        if rd::<u8>(f, 5) != 0 && rd::<u32>(this, 0) != 0 {
            wr::<u8>(this, 4, 7);
            wr(this, 0, p);
        }
        return;
    }
    record(this, p, 3);
}

/// The 0x61A253 tail: slot 0 gets (ped, kind) when slot 0 is in use.
fn record(this: GPtr<u8>, p: GPtr<u8>, kind: u8) {
    if rd::<u32>(this, 0) != 0 {
        wr::<u8>(this, 4, kind);
        wr(this, 0, p);
    }
}


/// `CPedIntelligence::RestorePedDecisionMakerType` (0x600BC0): +0xB4 = +0xB8 when +0xB4 is 0; returns +0xB4 (EAX).
#[hook(addr = 0x600BC0, conv = thiscall, name = "CPedIntelligence::RestorePedDecisionMakerType", expect = "8B 81 B4 00 00 00")]
pub fn intel_restore_ped_decision_maker_type(this: GPtr<u8>) -> u32 {
    let v = rd::<u32>(this, 0xB4);
    if v != 0 {
        return v;
    }
    let v = rd::<u32>(this, 0xB8);
    wr::<u32>(this, 0xB4, v);
    v
}

/// `CPedIntelligence::IsRespondingToEvent(int32)` (0x600DB0, `ret 4`): tail jump to the event history's
/// `IsRespondingToEvent` (+0x38).
#[hook(addr = 0x600DB0, conv = thiscall, name = "CPedIntelligence::IsRespondingToEvent", expect = "83 C1 38 E9 D8 7D EB FF")]
pub fn intel_is_responding_to_event(this: GPtr<u8>, ty: i32) -> u8 {
    ehh_is_responding_to_event(this.byte_add(0x38), ty)
}

/// `CPedIntelligence::AreFriends(const CPed&, const CPed&)` (0x601D10, cdecl): 1 when either ped's intelligence is
/// friendly with the other (first ped's first), else 0 (EAX).
#[hook(addr = 0x601D10, conv = cdecl, name = "CPedIntelligence::AreFriends", expect = "56 8B 74 24 08")]
pub fn intel_are_friends(a: GPtr<u8>, b: GPtr<u8>) -> u32 {
    if intel_is_friendly_with(rd::<GPtr<u8>>(a, 0x47C), b) != 0 {
        return 1;
    }
    (intel_is_friendly_with(rd::<GPtr<u8>>(b, 0x47C), a) != 0) as u32
}

/// `CPedIntelligence::IsPedGoingForCarDoor` (0x602350): the simplest active task, its parent or grandparent (+4) is
/// a 0x320 (AL).
#[hook(addr = 0x602350, conv = thiscall, name = "CPedIntelligence::IsPedGoingForCarDoor", expect = "56 83 C1 04 E8 77 F6 07 00")]
pub fn intel_is_ped_going_for_car_door(this: GPtr<u8>) -> u8 {
    let mut t = tm_get_simplest_active_task(tm(this));
    for k in 0..3 {
        if t.is_null() {
            return 0;
        }
        if v_task_type(t) == 0x320 {
            return 1;
        }
        if k < 2 {
            t = rd::<GPtr<u8>>(t, 4);
        }
    }
    0
}

/// `CPedIntelligence::CanSeeEntityWithLights(const CEntity*, bool)` (0x605550, `ret 8`; the second argument is
/// unused): [0x8D2380] unless the entity is a player ped whose lighting total is at most that; then a light value
/// fading with the distance minus [0x858CB0] (two stages, [0x8D2384] and [0x8D2388], each compared with the double
/// [0x859EF8]); 0.0 when the second stage is not above it. The original stores the distance term as an f32 into its
/// own argument slot. Result in ST0.
#[hook(addr = 0x605550, conv = thiscall, name = "CPedIntelligence::CanSeeEntityWithLights", expect = "56 57 8B 7C 24 0C")]
pub fn intel_can_see_entity_with_lights(this: GPtr<u8>, e: GPtr<u8>, _b: u32) -> X87 {
    let c = || x(g::<f32>(0x8D2380).read());
    if rd::<u8>(e, 0x36) & 7 != 3 || ped_is_player(e) == 0 {
        return c();
    }
    let v = X87::from_f64_exact(physical_get_lighting_total(e));
    if !v.fcom(c()).test_any(0x41) {
        return c();
    }
    let pp = entity_pos(ped(this));
    let ep = entity_pos(e);
    let dx = x(rd::<f32>(ep, 0)).fsub_m(rd::<f32>(pp, 0));
    let dy = x(rd::<f32>(ep, 4)).fsub_m(rd::<f32>(pp, 4));
    let dz = x(rd::<f32>(ep, 8)).fsub_m(rd::<f32>(pp, 8));
    let dist = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy)).fsqrt();
    let m = f32_of(dist.fsub_m(g::<f32>(0x858CB0).read()));
    let k = X87::from_f64_exact(g::<f64>(0x859EF8).read());
    let cm = g::<f32>(0x8D2380).read();
    let r1 = v.fmul(v).fsub(x(m).fdiv_m(g::<f32>(0x8D2384).read()).fmul_m(cm).fmul_m(cm));
    if !r1.fcom(k).test_any(0x41) {
        return r1;
    }
    let r2 = r1.fmul(r1).fsub(x(m).fdiv_m(g::<f32>(0x8D2388).read()).fmul_m(cm).fmul_m(cm));
    if r2.fcom(k).test_any(0x41) {
        return x(g::<f32>(F_ZERO).read());
    }
    r2.fmul_m(g::<f32>(0x858C1C).read())
}

/// The inlined "secondary task `slot` if it has type `ty`" selection with every `GetTaskSecondary` call of the
/// original (null test, type, result).
fn secondary_if(tmp: GPtr<u8>, slot: i32, ty: i32) -> GPtr<u8> {
    if !tm_get_task_secondary(tmp, slot).is_null() && v_task_type(tm_get_task_secondary(tmp, slot)) == ty {
        tm_get_task_secondary(tmp, slot)
    } else {
        GPtr::NULL
    }
}

/// `CPedIntelligence::SetTaskDuckSecondary(uint16 length)` (0x601230, `ret 4`): unless secondary slot 1 holds a
/// duck task (0x19F) whose control type (+0x24) is 4, a new `CTaskSimpleDuck(2, length, -1)` (0x28 bytes) goes to
/// secondary slot 1, a 0x3F9 secondary slot-0 task gets `ClearAnim(ped)`, and the duck task runs `ProcessPed` (no
/// null check).
#[hook(addr = 0x601230, conv = thiscall, name = "CPedIntelligence::SetTaskDuckSecondary", expect = "6A FF 68 AB E7 83 00")]
pub fn intel_set_task_duck_secondary(this: GPtr<u8>, len: u32) {
    let tmp = tm(this);
    if !tm_get_task_secondary(tmp, 1).is_null()
        && v_task_type(tm_get_task_secondary(tmp, 1)) == 0x19F
        && !tm_get_task_secondary(tmp, 1).is_null()
    {
        let t = secondary_if(tmp, 1, 0x19F);
        if rd::<u8>(t, 0x24) == 4 {
            return;
        }
    }
    let t = task_new(0x28);
    let task = if t.is_null() { GPtr::NULL } else { duck_ctor(t, 2, len, 0xFFFF_FFFF) };
    tm_set_task_secondary(tmp, task, 1);
    if !tm_get_task_secondary(tmp, 0).is_null()
        && v_task_type(tm_get_task_secondary(tmp, 0)) == 0x3F9
        && !tm_get_task_secondary(tmp, 0).is_null()
    {
        let gun = secondary_if(tmp, 0, 0x3F9);
        use_gun_clear_anim(gun, ped(this));
    }
    let d = tm_get_task_secondary(tmp, 1);
    v_process_ped(d, ped(this));
}

/// `CPedIntelligence::ClearTaskDuckSecondary` (0x601390): asks the secondary duck task to abort (priority 0); a
/// player's pad duck flag (+0x480 -> +0x14) is cleared, any other ped gets move state 1; a 0x3F9 secondary slot-0
/// task gets `ClearAnim(ped)` through `GetTaskUseGun` (the ped argument pushed before `GetTaskUseGun` is consumed by
/// `ClearAnim`).
#[hook(addr = 0x601390, conv = thiscall, name = "CPedIntelligence::ClearTaskDuckSecondary", expect = "56 57 8B F9 8D 77 04")]
pub fn intel_clear_task_duck_secondary(this: GPtr<u8>) {
    let tmp = tm(this);
    if tm_get_task_secondary(tmp, 1).is_null() {
        return;
    }
    let t = tm_get_task_secondary(tmp, 1);
    v_make_abortable(t, ped(this), 0, 0);
    let p = ped(this);
    let pd = rd::<GPtr<u8>>(p, 0x480);
    if !pd.is_null() {
        wr::<u32>(pd, 0x14, 0);
    } else {
        ped_set_move_state(p, 1);
    }
    if !tm_get_task_secondary(tmp, 0).is_null()
        && v_task_type(tm_get_task_secondary(tmp, 0)) == 0x3F9
        && !tm_get_task_secondary(tmp, 0).is_null()
    {
        let p = ped(this);
        let gun = intel_get_task_use_gun(this);
        use_gun_clear_anim(gun, p);
    }
}

/// `CPedScriptedTaskRecord::Process` (0x6085E0, static): every one of the 128 records (0xC0B1E8, 0x14 bytes: +0
/// opcode, +4 event, +8 task, +0xC ped, +0x10 status) with an event, task or ped: a record without ped is cleared
/// (status 3, opcode -1); with an event, status 0 while the ped's event group still has it, else the ped reference is
/// released and the record cleared; with a task, status 5 / 4 records keep their task while the brain-task store /
/// the ped's group still hold it, others get status 1 (active task), 2 (a primary task) or 6 (a secondary task),
/// otherwise the record is flushed.
#[hook(addr = 0x6085E0, conv = cdecl, name = "CPedScriptedTaskRecord::Process", expect = "53 55 56 57 BE F4 B1 C0 00")]
pub fn scripted_task_record_process_port() {
    let clear = |r: GPtr<u8>| {
        wr::<u32>(r, 0xC, 0);
        wr::<u32>(r, 4, 0);
        wr::<u32>(r, 8, 0);
        wr::<u32>(r, 0x10, 3);
        wr::<u32>(r, 0, 0xFFFF_FFFF);
    };
    for i in 0..128usize {
        let r: GPtr<u8> = GPtr::from_addr(0xC0B1E8 + 0x14 * i);
        let ev = rd::<GPtr<u8>>(r, 4);
        if ev.is_null() && rd::<u32>(r, 8) == 0 && rd::<u32>(r, 0xC) == 0 {
            continue;
        }
        let p = rd::<GPtr<u8>>(r, 0xC);
        if p.is_null() {
            clear(r);
            continue;
        }
        if !ev.is_null() {
            wr::<u32>(r, 0x10, 0);
            if eg_has_event(rd::<GPtr<u8>>(p, 0x47C).byte_add(0x68), ev) != 0 {
                continue;
            }
            let q = rd::<GPtr<u8>>(r, 0xC);
            if !q.is_null() {
                entity_clean_up_old_reference(q, r.byte_add(0xC));
            }
            clear(r);
            continue;
        }
        if rd::<u32>(r, 8) == 0 {
            continue;
        }
        let status = rd::<u32>(r, 0x10);
        if status == 5 || status == 4 {
            let cur = if status == 5 {
                Some(brain_task_store_get_task(p))
            } else {
                let grp = ped_groups_get_peds_group(p);
                if grp.is_null() {
                    None
                } else {
                    Some(pgi_get_task_script_command(grp.byte_add(0x30), rd::<GPtr<u8>>(r, 0xC)))
                }
            };
            if cur != Some(rd::<u32>(r, 8)) {
                task_record_data_flush(r);
            }
            continue;
        }
        let intel = rd::<GPtr<u8>>(p, 0x47C);
        let task = rd::<u32>(r, 8);
        if tm_get_active_task(intel.byte_add(4)).addr() as u32 == task {
            wr::<u32>(r, 0x10, 1);
            continue;
        }
        let mut a = rd::<u32>(intel, 4);
        if a == 0 {
            a = rd::<u32>(intel, 8);
            if a == 0 {
                a = rd::<u32>(intel, 0xC);
            }
        }
        if a == task || rd::<u32>(intel, 0x10) == task || rd::<u32>(intel, 0x14) == task {
            wr::<u32>(r, 0x10, 2);
            continue;
        }
        if tm_has_task_secondary(intel.byte_add(4), task) != 0 {
            wr::<u32>(r, 0x10, 6);
        } else {
            task_record_data_flush(r);
        }
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_INTEL_CTOR,
    HOOK_INTEL_DTOR,
    HOOK_INTEL_PROCESS,
    HOOK_INTEL_PROCESS_FIRST,
    HOOK_INTEL_CLEAR_TASKS,
    HOOK_INTEL_FLUSH_IMMEDIATELY,
    HOOK_INTEL_PROCESS_AFTER_PROC_COL,
    HOOK_INTEL_PROCESS_AFTER_PRE_RENDER,
    HOOK_INTEL_FLUSH_INTELLIGENCE,
    HOOK_INTEL_RECORD_EVENT_FOR_SCRIPT,
    HOOK_INTEL_LOOK_AT_INTERESTING_ENTITIES_PORT,
    HOOK_INTEL_PROCESS_STATIC_COUNTER_PORT,
    HOOK_INTEL_GET_PED_FOV_RANGE,
    HOOK_INTEL_IS_IN_SEEING_RANGE,
    HOOK_REC_RECORD_RELATIONSHIP_WITH_PLAYER,
    HOOK_INTEL_RESTORE_PED_DECISION_MAKER_TYPE,
    HOOK_INTEL_IS_RESPONDING_TO_EVENT,
    HOOK_INTEL_ARE_FRIENDS,
    HOOK_INTEL_IS_PED_GOING_FOR_CAR_DOOR,
    HOOK_INTEL_CAN_SEE_ENTITY_WITH_LIGHTS,
    HOOK_INTEL_SET_TASK_DUCK_SECONDARY,
    HOOK_INTEL_CLEAR_TASK_DUCK_SECONDARY,
    HOOK_SCRIPTED_TASK_RECORD_PROCESS_PORT,
];
