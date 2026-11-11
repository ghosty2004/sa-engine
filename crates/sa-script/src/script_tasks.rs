//! `CRunningScript::GivePedScriptedTask` (batch `mvpi-script-1`): how a script hands a task to a ped
//! (the open task sequence, an attractor brain, the ped's group, or a `CEventScriptCommand`). Ported
//! from the 1.0 US bytes; unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// `CPools::ms_pPedPool` (0xB74490): peds +0, flag bytes +4; peds are 0x7C4 bytes.
const PED_POOL: usize = 0xB74490;
/// `CTheScripts::LocalVariablesForCurrentMission` (0xA48960)
const MISSION_LOCALS: usize = 0xA48960;
/// `CTaskSequences::ms_iActiveSequence` (0x8D2E98)
const ACTIVE_SEQUENCE: usize = 0x8D2E98;
/// `CTaskSequences::ms_taskSequence` (0xC178F0, 0x40 bytes each)
const TASK_SEQUENCES: usize = 0xC178F0;
/// `CPedScriptedTaskRecord::ms_scriptedTasks` (0xC0B1E8, 0x14 bytes each)
const SCRIPTED_TASKS: usize = 0xC0B1E8;

orig! {
    /// `CPedGroups::GetPedsGroup(CPed*)` (0x5F7E80)
    fn get_peds_group(ped: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CScriptedBrainTaskStore::SetTask(CPed*, CTask*)` (0x635720)
    fn brain_task_store_set_task(ped: GPtr<u8>, task: GPtr<u8>) -> GPtr<u8> = cdecl 0x635720 => unported;
    /// `CPedScriptedTaskRecord::GetVacantSlot()` (0x608540)
    fn get_vacant_slot() -> i32 = cdecl 0x608540 => unported;
    /// `CPedScriptedTaskRecordData::SetAsAttractorScriptTask(CPed*, int32 opcode, CTask*)` (0x608440)
    fn record_set_as_attractor_script_task(rec: GPtr<u8>, ped: GPtr<u8>, opcode: u32, task: GPtr<u8>) = thiscall 0x608440 => unported;
    /// `CPedScriptedTaskRecordData::SetAsGroupTask(CPed*, int32 opcode, CTask*)` (0x6084A0)
    fn record_set_as_group_task(rec: GPtr<u8>, ped: GPtr<u8>, opcode: u32, task: GPtr<u8>) = thiscall 0x6084A0 => unported;
    /// `CPedScriptedTaskRecordData::Set(CPed*, int32 opcode, CEventScriptCommand*)` (0x608390)
    fn record_set_event(rec: GPtr<u8>, ped: GPtr<u8>, opcode: u32, event: GPtr<u8>) = thiscall 0x608390 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0, AL)
    fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPedGroupIntelligence::SetScriptCommandTask(CPed*, const CTask&)` (0x5F8560)
    fn group_intel_set_script_command_task(gi: GPtr<u8>, ped: GPtr<u8>, task: GPtr<u8>) = thiscall 0x5F8560 => unported;
    /// `CPedGroupIntelligence::GetTaskScriptCommand(CPed*)` (0x5F8690)
    fn group_intel_get_task_script_command(gi: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F8690 => unported;
    /// `CEventScriptCommand::CEventScriptCommand(int32 primaryTaskId, CTask*, bool affectsDeadPeds)` (0x4B0A00)
    fn event_script_command_ctor(e: GPtr<u8>, id: u32, task: GPtr<u8>, dead: u32) -> GPtr<u8> = thiscall 0x4B0A00 => unported;
    /// `CEventScriptCommand::~CEventScriptCommand()` (0x4B0A50, catalogue `FUN_004b0a50`)
    fn event_script_command_dtor(e: GPtr<u8>) = thiscall 0x4B0A50 => unported;
    /// `CEventGroup::Add(CEvent*, bool unused)` (0x4AB420): the stored copy or null
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, unused: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10)
    fn sequence_add_task(seq: GPtr<u8>, task: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// The task's virtual slot 0 (scalar deleting destructor), `ecx` = task, flag 1.
    fn task_delete_virtual(task: GPtr<u8>, flag: u32) -> GPtr<u8> = thiscall vslot 0x00 => unported;
}

fn at<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize) -> GPtr<T> {
    p.byte_add(off).cast::<T>()
}

fn ga<T: sa_core::mem::GameData>(addr: usize) -> GPtr<T> {
    GPtr::from_addr(addr)
}

/// The ped of pool handle `h` (`h >> 8` arithmetic, its flag byte equal to `h`'s low byte), else null.
fn ped_at(h: i32) -> GPtr<u8> {
    let pool = ga::<GPtr<u8>>(PED_POOL).read();
    let flags = at::<GPtr<u8>>(pool, 4).read();
    let i = h >> 8;
    if at::<u8>(flags, i as isize).read() == h as u8 {
        at::<GPtr<u8>>(pool, 0).read().byte_add(0x7C4 * i as isize)
    } else {
        GPtr::NULL
    }
}

/// The record `CPedScriptedTaskRecord::GetVacantSlot()` names (`ms_scriptedTasks + slot * 0x14`).
fn vacant_record() -> GPtr<u8> {
    let slot = get_vacant_slot();
    GPtr::from_addr((SCRIPTED_TASKS as u32).wrapping_add((slot as u32).wrapping_mul(0x14)) as usize)
}

/// `CRunningScript::GivePedScriptedTask(int32 pedHandle, CTask* task, int32 opcode)` (0x465C20,
/// thiscall, `ret 0xC`). Handle -1: `CTaskComplexSequence::AddTask(task)` on the open sequence
/// (`ms_taskSequence[ms_iActiveSequence]`). Else the ped of the handle (null when the handle is stale),
/// its group (`CPedGroups::GetPedsGroup`) and, for script type +0xC9 in {0, 2, 3, 5}, the brain ped
/// (local variable 0 of the mission locals 0xA48960 when +0xDC is set, else of the script's +0x3C, as a
/// ped handle; null otherwise). A ped with flag 0x800000 (+0x474) that is not the brain ped: the task
/// is deleted (virtual slot 0, flag 1) when non-null and nothing else happens. A brain ped with type 5:
/// `CScriptedBrainTaskStore::SetTask(ped, task)`; a stored task records
/// `SetAsAttractorScriptTask(ped, opcode, task)` in the vacant record. A group and a non-player ped:
/// `SetScriptCommandTask(group + 0x30, ped, task)`, `SetAsGroupTask(ped, opcode,
/// GetTaskScriptCommand(ped))` in the vacant record, then the task is deleted when non-null. Otherwise
/// a stack `CEventScriptCommand(3, task, false)` goes into the ped's event group (intelligence +0x47C,
/// +0x68); a stored copy records `Set(ped, opcode, copy)` in the vacant record; the stack event is
/// destroyed.
#[hook(addr = 0x465c20, conv = thiscall, name = "CRunningScript::GivePedScriptedTask-OG", expect = "64 A1 00 00 00 00",
       unverified = "mvpi-script-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "deletes the task or stores it in the task / event pools")]
pub fn give_ped_scripted_task(this: GPtr<u8>, ped_handle: i32, task: GPtr<u8>, opcode: u32) {
    if ped_handle == -1 {
        let seq = (ga::<u32>(ACTIVE_SEQUENCE).read() << 6).wrapping_add(TASK_SEQUENCES as u32);
        sequence_add_task(GPtr::from_addr(seq as usize), task);
        return;
    }
    let ped = ped_at(ped_handle);
    let group = get_peds_group(ped);
    let ty = at::<u8>(this, 0xC9).read();
    let brain = if matches!(ty, 5 | 2 | 0 | 3) {
        let locals: GPtr<u8> =
            if at::<u8>(this, 0xDC).read() != 0 { GPtr::from_addr(MISSION_LOCALS) } else { this.byte_add(0x3C) };
        ped_at(at::<i32>(locals, 0).read())
    } else {
        GPtr::NULL
    };
    if at::<u32>(ped, 0x474).read() & 0x80_0000 != 0 && brain != ped {
        if !task.is_null() {
            task_delete_virtual(task, 1);
        }
        return;
    }
    if !brain.is_null() && ty == 5 {
        if brain_task_store_set_task(ped, task).is_null() {
            return;
        }
        record_set_as_attractor_script_task(vacant_record(), ped, opcode, task);
        return;
    }
    if !group.is_null() && ped_is_player(ped) == 0 {
        let gi = group.byte_add(0x30);
        group_intel_set_script_command_task(gi, ped, task);
        let t = group_intel_get_task_script_command(gi, ped);
        record_set_as_group_task(vacant_record(), ped, opcode, t);
        if !task.is_null() {
            task_delete_virtual(task, 1);
        }
        return;
    }
    let mut buf = [0u32; 0x18 / 4];
    let ev = GPtr::from_raw(buf.as_mut_ptr().cast::<u8>());
    event_script_command_ctor(ev, 3, task, 0);
    let intel = at::<GPtr<u8>>(ped, 0x47C).read();
    let copy = event_group_add(intel.byte_add(0x68), ev, 0);
    if !copy.is_null() {
        record_set_event(vacant_record(), ped, opcode, copy);
    }
    event_script_command_dtor(ev);
}

pub const HOOKS: &[HookReg] = &[HOOK_GIVE_PED_SCRIPTED_TASK];
