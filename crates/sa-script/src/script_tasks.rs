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

use crate::bindings::brain_task_store_set_task;
use crate::bindings::record_set_event;
use sa_ped::bindings::ped_is_player;
use sa_ai::bindings::event_group_add;
use sa_world::bindings::event_script_command_ctor;
use sa_world::bindings::event_script_command_dtor;
use sa_world::bindings::group_intel_set_script_command_task;
use sa_ai::bindings::ped_groups_get_peds_group as get_peds_group;
use sa_world::bindings::sequence_add_task;
use crate::bindings::{affect_secondary_behaviour_ctor, read_label};
use sa_core::bindings::{task_new, tm_set_task_secondary};
use sa_world::bindings::hold_entity_ctor;
use crate::bindings::get_vacant_slot;
use sa_world::bindings::group_intel_get_task_script_command;
orig! {
    /// `CPedScriptedTaskRecordData::SetAsAttractorScriptTask(CPed*, int32 opcode, CTask*)` (0x608440)
    fn record_set_as_attractor_script_task(rec: GPtr<u8>, ped: GPtr<u8>, opcode: u32, task: GPtr<u8>) = thiscall 0x608440 => unported;
    /// `CPedScriptedTaskRecordData::SetAsGroupTask(CPed*, int32 opcode, CTask*)` (0x6084A0)
    fn record_set_as_group_task(rec: GPtr<u8>, ped: GPtr<u8>, opcode: u32, task: GPtr<u8>) = thiscall 0x6084A0 => unported;
    /// The task's virtual slot 0 (scalar deleting destructor), `ecx` = task, flag 1.
    fn task_delete_virtual(task: GPtr<u8>, flag: u32) -> GPtr<u8> = thiscall vslot 0x00 => unported;
}

use sa_core::field::at;

use sa_core::field::g as ga;

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
    let slot = get_vacant_slot() as i32;
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
    let group = GPtr::from_addr(get_peds_group(ped.addr() as u32) as usize);
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
    event_script_command_dtor(ev.cast::<u32>());
}

pub const HOOKS: &[HookReg] = &[HOOK_GIVE_PED_SCRIPTED_TASK];

orig! {
    /// `CTaskSimpleHoldEntity::CTaskSimpleHoldEntity(CEntity*, CVector*, uint8 bone, uint8 boneFlags, const char*
    /// animName, const char* animBlock, eAnimationFlags)` (0x691470)
    #[allow(clippy::too_many_arguments)]
    fn hold_entity_ctor_named(t: GPtr<u8>, e: GPtr<u8>, off: GPtr<u8>, bone: u32, flags: u32, anim: GPtr<u8>, block: GPtr<u8>, anim_flags: u32) -> GPtr<u8> = thiscall 0x691470 => unported;
    /// `CPedScriptedTaskRecordData::Set(CPed*, int32 opcode, CTask*)` (0x6083E0)
    fn record_set_task(r: GPtr<u8>, ped: GPtr<u8>, opcode: u32, task: GPtr<u8>) = thiscall 0x6083E0 => unported;
}

/// `CTheScripts::ScriptParams` (0xA43C78), the collected parameters.
const SCRIPT_PARAMS: usize = 0xA43C78;
/// `CPools::ms_pObjectPool` (0xB7449C): objects +0, flag bytes +4.
const OBJECT_POOL: usize = 0xB7449C;

/// `CRunningScript::ScriptTaskPickUpObject(int32 opcode)` (stub 0x46AF50 → .HOODLUM 0x156B440, thiscall, `ret 4`; SEH
/// frame for the `new` expressions, not modelled). Seven parameters: ped handle, object handle, an offset vector
/// (copied twice on the stack; the second copy is passed), bone and bone flags (bytes); then a 24-byte animation name
/// and a 16-byte block name (`ReadTextLabelFromScript`) and one more parameter (anim flags 0x10 when non-zero, else
/// 0x18). A `CTaskSimpleHoldEntity` (0x3C bytes): when the name or the block is "NULL" the constructor 0x6913A0 with
/// anim 0xBF, group 0 and a flag the protection pushes as `~[0x1560560] ^ [0x4C4572]` = 0; else 0x691470 with the two
/// names and the anim flags. Ped handle -1: only for opcode 0x70A a `CTaskSimpleAffectSecondaryBehaviour(1, 4, task)`
/// (0x14 bytes) goes into the open sequence (`AddTask` reached through a call/ret trampoline: `rol([0x1560568],
/// [0x4D7191])` = 0x632D10); otherwise the hold task is not stored. Else the ped of the handle (null when stale, then
/// dereferenced as the original does) gets `SetTaskSecondary(task, slot)` (slot 4 for opcode 0x70A, `[0x1560564] -
/// [0x6A9663] - 1`; else 0 from the VM 0xCB73E0, descriptor 0x1560028) and the vacant scripted-task record `Set(ped,
/// opcode, task)`.
#[hook(addr = 0x46AF50, conv = thiscall, name = "CRunningScript::ScriptTaskPickUpObject", expect = "E9 EB 04 10 01",
       unverified = "frontier2-c12", route = "L2", hazards = "heap-writes,uninit-locals",
       note = "the bone bytes are pushed as stack dwords with uninitialised upper bytes; allocates tasks; the three protection pushes and the AddTask trampoline evaluated on the exe image")]
pub fn script_task_pick_up_object(this: GPtr<u8>, opcode: u32) {
    crate::vm::collect_parameters_call(this, 7);
    let params = ga::<u8>(SCRIPT_PARAMS);
    let ped_handle = at::<i32>(params, 0).read();
    let pool = ga::<GPtr<u8>>(OBJECT_POOL).read();
    let obj = crate::script_objects::object_at(pool, at::<GPtr<u8>>(pool, 4).read(), at::<i32>(params, 4).read());
    let off = [at::<u32>(params, 8).read(), at::<u32>(params, 12).read(), at::<u32>(params, 16).read()];
    let mut off2 = off;
    let bone = u32::from(at::<u8>(params, 20).read());
    let bone_flags = u32::from(at::<u8>(params, 24).read());
    let mut anim = [0u8; 0x18];
    let mut block = [0u8; 0x10];
    let anim_p = GPtr::from_raw(anim.as_mut_ptr());
    let block_p = GPtr::from_raw(block.as_mut_ptr());
    read_label(this, anim_p, 0x18);
    read_label(this, block_p, 0x10);
    let null = anim[..5] == *b"NULL\0" || block[..5] == *b"NULL\0";
    crate::vm::collect_parameters_call(this, 1);
    let anim_flags = if at::<u32>(params, 0).read() != 0 { 0x10 } else { 0x18 };
    let mem = task_new(0x3C);
    let off_p = GPtr::from_raw(off2.as_mut_ptr().cast::<u8>());
    let task = if mem.is_null() {
        GPtr::NULL
    } else if null {
        hold_entity_ctor(mem, obj, off_p, bone, bone_flags, 0xBF, 0, 0)
    } else {
        hold_entity_ctor_named(mem, obj, off_p, bone, bone_flags, anim_p, block_p, anim_flags)
    };
    if ped_handle == -1 {
        if opcode == 0x70A {
            let m = task_new(0x14);
            let t = if m.is_null() { GPtr::NULL } else { affect_secondary_behaviour_ctor(m, 1, 4, task) };
            let seq = (TASK_SEQUENCES as u32).wrapping_add(ga::<u32>(ACTIVE_SEQUENCE).read() << 6);
            sequence_add_task(GPtr::from_addr(seq as usize), t);
        }
        return;
    }
    let ped = ped_at(ped_handle);
    let tm = at::<GPtr<u8>>(ped, 0x47C).read().byte_add(4);
    tm_set_task_secondary(tm, task, if opcode == 0x70A { 4 } else { 0 });
    record_set_task(vacant_record(), ped, opcode, task);
}

/// Lane C turn 6 hooks (batch `frontier2-c12`).
pub const HOOKS6: &[HookReg] = &[HOOK_SCRIPT_TASK_PICK_UP_OBJECT];
