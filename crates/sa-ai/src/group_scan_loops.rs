//! More small AI bodies with loops (the AI-GEN interpreter rejects loops): group/vehicle scans, the ped list
//! compaction, the scripted brain task store and the task sequence clone (PORT-AI-8). Ported from the 1.0 US
//! bytes. Every call to another catalogued function goes through `orig!` at the original address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPedGroupMembership::GetMember(int32)` (0x5F69B0, `ret 4`)
    fn membership_get_member(m: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x5F69B0 => unported;
    /// `CPedGroupMembership::GetLeader` (0x5F69A0)
    fn membership_get_leader(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F69A0 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> u32 = thiscall 0x681740 => unported;
    /// `CPed::IsAlive` (0x5E0170)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) -> u32 = thiscall 0x571B70 => unported;
    /// the scalar deleting destructor (vtable +0, `ret 4`)
    fn v_delete(t: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CTask::Clone` (vtable +4)
    fn v_clone(t: GPtr<u8>) -> u32 = thiscall vslot 0x4 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0, cdecl)
    fn task_new_o(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskSimple::CTaskSimple` (`FUN_0061a390`)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CScriptedBrainTaskStore::Clear(CTask*)` (0x6357F0, cdecl)
    fn brain_store_clear_task(t: u32) = cdecl 0x6357F0 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CEvent::GetSourceEntity` (vtable +0x28)
    fn v_source_entity(e: GPtr<u8>) -> u32 = thiscall vslot 0x28 => unported;
    /// `CEvent::GetSoundLevel(const CEntity*, CVector&)` (0x4B2850, `ret 8`, ST0)
    fn event_get_sound_level(e: GPtr<u8>, ent: u32, pos: u32) -> f64 = thiscall 0x4B2850 => unported;
    /// `CEvent::CalcSoundLevelIncrement(float level, float increment)` (0x4AC050, cdecl, ST0)
    fn event_calc_sound_level_increment(a: u32, b: u32) -> f64 = cdecl 0x4AC050 => unported;
    /// `CTaskSimpleCarCloseDoorFromInside::ComputeAnimID(int32& group, int32& anim)` (0x646410, `ret 8`)
    fn close_door_inside_compute_anim_id(t: GPtr<u8>, g: GPtr<u8>, a: GPtr<u8>) = thiscall 0x646410 => unported;
    /// `CTaskSimpleCarCloseDoorFromOutside::ComputeAnimID(int32& group, int32& anim)` (0x646600, `ret 8`)
    fn close_door_outside_compute_anim_id(t: GPtr<u8>, g: GPtr<u8>, a: GPtr<u8>) = thiscall 0x646600 => unported;
    /// `CTaskSimpleCarSlowDragPedOut::ComputeAnimID_Wrapper(int32& group, int32& anim)` (0x648100, `ret 8`)
    fn slow_drag_ped_out_compute_anim_id(t: GPtr<u8>, g: GPtr<u8>, a: GPtr<u8>) = thiscall 0x648100 => unported;
    /// `CTaskSimpleCarSlowBeDraggedOut::ComputeAnimID(int32& group, int32& anim)` (0x648330, `ret 8`)
    fn slow_be_dragged_out_compute_anim_id(t: GPtr<u8>, g: GPtr<u8>, a: GPtr<u8>) = thiscall 0x648330 => unported;
    /// the vehicle's door-anim virtual (vtable +0x70: `(CPed*, int32 door, int32 group, int32 anim, float)`,
    /// `ret 0x14`)
    fn v_vehicle_door_anim(v: GPtr<u8>, ped: u32, door: u32, group: u32, anim: u32, f: u32) = thiscall vslot 0x70 => unported;
    /// `CTaskSimpleGoTo::CTaskSimpleGoTo(int32 moveState, const CVector&, float radius)` (0x6679C0, `ret 0xC`)
    fn simple_go_to_ctor(t: GPtr<u8>, ms: u32, pos: GPtr<u8>, radius: u32) -> GPtr<u8> = thiscall 0x6679C0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
}

/// `CEventLeaderEnteredCarAsDriver::AffectsPedGroup(CPedGroup*)` (0x4B0EF0, `ret 4`; AL): with a vehicle (+0x14)
/// and the group's +4 byte set, true as soon as a set member of slots 0..6 is not in a vehicle (+0x46C bit 8),
/// is in another vehicle (+0x58C), or runs active task 0x2C0; false otherwise.
#[hook(addr = 0x4B0EF0, conv = thiscall, name = "CEventLeaderEnteredCarAsDriver::AffectsPedGroup", expect = "53 8B D9 8B 43 14",
       unverified = "ai8-5", route = "L2", hazards = "ax-al-return")]
pub fn leader_entered_car_as_driver_affects_ped_group(this: GPtr<u8>, group: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0x14) == 0 || rd::<u8>(group, 4) == 0 {
        return 0;
    }
    let m = group.byte_add(8);
    for i in 0..7 {
        let p = membership_get_member(m, i);
        if p.is_null() {
            continue;
        }
        if rd::<u32>(p, 0x46C) & 0x100 == 0 || rd::<u32>(p, 0x58C) != rd::<u32>(this, 0x14) {
            return 1;
        }
        if tm_find_active_task_by_type(rd::<GPtr<u8>>(p, 0x47C).byte_add(4), 0x2C0) != 0 {
            return 1;
        }
    }
    0
}

/// `CPedList::FillUpHoles` (0x699E20): compacts the 30 entries (+4) in order over the null ones (the count +0
/// is left alone) and zeroes the rest. EAX: 0 after zeroing, else the end of the entries (+0x7C).
#[hook(addr = 0x699E20, conv = thiscall, name = "CPedList::FillUpHoles", expect = "53 56 57 33 D2 8D 41 04",
       unverified = "ai8-5", route = "L2", hazards = "heap-writes")]
pub fn ped_list_fill_up_holes(this: GPtr<u8>) -> u32 {
    let mut n = 0isize;
    for i in 0..30isize {
        let p = rd::<u32>(this, 4 + 4 * i);
        if p != 0 {
            wr::<u32>(this, 4 + 4 * n, p);
            n += 1;
        }
    }
    if n >= 30 {
        return (this.addr() as u32).wrapping_add(4 + 4 * n as u32);
    }
    for k in n..30 {
        wr::<u32>(this, 4 + 4 * k, 0);
    }
    0
}

/// `CScriptedBrainTaskStore::SetTask(CPed*, CTask*)` (0x635720, cdecl): over the 48 entries (0xC188F0, 0x10 bytes:
/// ped, task at +0xC) the ped's entry, else the first free one (the ped stored and referenced), gets the task (the
/// old one deleted); the result is the task. With neither, the task is deleted and the result is null.
#[hook(addr = 0x635720, conv = cdecl, name = "CScriptedBrainTaskStore::SetTask", expect = "8B 4C 24 04 56 83 C8 FF",
       unverified = "ai8-5", route = "L2", hazards = "heap-writes")]
pub fn scripted_brain_task_store_set_task(ped: u32, task: u32) -> u32 {
    let mut free = -1i32;
    for i in 0..0x30usize {
        let e = g::<u32>(0xC188F0 + i * 0x10).read();
        if e == 0 && free == -1 {
            free = i as i32;
        }
        if e == ped {
            return replace_task(0xC188FC + i * 0x10, task);
        }
    }
    if free == -1 {
        if task != 0 {
            v_delete(GPtr::from_addr(task as usize), 1);
        }
        return 0;
    }
    let slot = 0xC188F0 + free as usize * 0x10;
    g::<u32>(slot).write(ped);
    entity_register_reference(GPtr::from_addr(ped as usize), g(slot));
    replace_task(slot + 0xC, task)
}

/// Deletes the task at `slot` (when set) and stores `task` there; returns `task`.
fn replace_task(slot: usize, task: u32) -> u32 {
    let old = g::<u32>(slot).read();
    if old != 0 {
        v_delete(GPtr::from_addr(old as usize), 1);
    }
    g::<u32>(slot).write(task);
    task
}

/// `CTaskComplexSequence::Clone` (0x5F6710): a new sequence (0x40 bytes, the constructor; the original goes on
/// with a null result) gets the clones of the 8 tasks (+0x10; null slots stay null), the repeat flag (+0x30) and
/// the current index (+0xC).
#[hook(addr = 0x5F6710, conv = thiscall, name = "CTaskComplexSequence::Clone", expect = "6A FF 68 DB DE 83 00",
       unverified = "ai8-5", route = "L2", hazards = "heap-writes")]
pub fn sequence_clone(this: GPtr<u8>) -> GPtr<u8> {
    let mem = task_new_o(0x40);
    let c = if mem.is_null() { GPtr::NULL } else { sequence_ctor(mem) };
    for i in 0..8isize {
        let t = rd::<GPtr<u8>>(this, 0x10 + 4 * i);
        let v = if t.is_null() { 0 } else { v_clone(t) };
        wr::<u32>(c, 0x10 + 4 * i, v);
    }
    wr::<u32>(c, 0x30, rd::<u32>(this, 0x30));
    wr::<u32>(c, 0xC, rd::<u32>(this, 0xC));
    c
}

/// `CTaskComplexSeekCoverUntilTargetDead::GetTargetPed` (0x622CC0; part of it in a fragment at 0x401EBA): with a
/// group index (+0xC, >= 0) the group's (0xC09928 + 0x2D4 * index) living leader, else its first living member of
/// slots 0..6, else null; without one the target ped (+0x10) when alive, else null.
#[hook(addr = 0x622CC0, conv = thiscall, name = "CTaskComplexSeekCoverUntilTargetDead::GetTargetPed", expect = "53 55 8B D9 8B 4B 0C",
       unverified = "ai8-5", route = "L2", hazards = "none")]
pub fn seek_cover_until_target_dead_get_target_ped(this: GPtr<u8>) -> GPtr<u8> {
    let idx = rd::<i32>(this, 0xC);
    if idx < 0 {
        let p = rd::<GPtr<u8>>(this, 0x10);
        if !p.is_null() && ped_is_alive(p) != 0 {
            return rd::<GPtr<u8>>(this, 0x10);
        }
        return GPtr::NULL;
    }
    let grp = |t: GPtr<u8>| GPtr::<u8>::from_addr((rd::<i32>(t, 0xC).wrapping_mul(0x2D4) as u32).wrapping_add(0xC09928) as usize);
    let l = membership_get_leader(grp(this));
    if !l.is_null() && ped_is_alive(l) != 0 {
        return l;
    }
    for i in 0..7 {
        let p = membership_get_member(grp(this), i);
        if !p.is_null() && ped_is_alive(p) != 0 {
            return p;
        }
    }
    GPtr::NULL
}

/// `CTaskSimpleWaitForBus::ProcessPed(CPed*)` (0x632AA0, `ret 4`; AL): among the ped's 16 scanned vehicles
/// (intelligence +0xE0) the first bus (+0x429 bit 1) moving slower than 0.005 (`k * k > |v|^2`) with a free seat
/// (+0x484 < +0x488 - 1) becomes the bus (+8) and AL = 1; else AL = 0.
#[hook(addr = 0x632AA0, conv = thiscall, name = "CTaskSimpleWaitForBus::ProcessPed", expect = "8B 44 24 04 55 56 57",
       unverified = "ai8-5", route = "L2", hazards = "ax-al-return,heap-writes")]
pub fn wait_for_bus_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let list = rd::<GPtr<u8>>(ped, 0x47C).byte_add(0xE0);
    for i in 0..16isize {
        let v = rd::<GPtr<u8>>(list, 4 * i);
        if v.is_null() || rd::<u8>(v, 0x429) & 2 == 0 {
            continue;
        }
        let (vx, vy, vz) = (x(rd::<f32>(v, 0x44)), x(rd::<f32>(v, 0x48)), x(rd::<f32>(v, 0x4C)));
        let s = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz));
        let k = g::<f32>(0x86DB2C).read();
        if x(k).fmul_m(k).fcom(s).test_any(0x41) {
            continue;
        }
        if i32::from(rd::<u8>(v, 0x484)) < i32::from(rd::<u8>(v, 0x488)) - 1 {
            wr::<GPtr<u8>>(this, 8, v);
            return 1;
        }
    }
    0
}

/// `CDecision::Add(int32 taskId, float* probs, int32* flags)` (0x600600, `ret 0xC`): the first of the 6 slots that
/// is empty (task id +0 is -1, the 4 probability bytes +0x18 and the 2 flag bytes +0x30 all 0) takes the task, the
/// probabilities through `_ftol` (low bytes) and the flags as `!= 0` bytes.
#[hook(addr = 0x600600, conv = thiscall, name = "CDecision::Add", expect = "53 56 57 8B F9 33 F6",
       unverified = "ai8-5", route = "L2", hazards = "heap-writes")]
pub fn decision_add(this: GPtr<u8>, task: u32, probs: GPtr<u8>, flags: GPtr<u8>) {
    for i in 0..6isize {
        if rd::<i32>(this, 4 * i) != -1 {
            continue;
        }
        if (0..4).any(|k| rd::<u8>(this, 0x18 + 4 * i + k) != 0) || (0..2).any(|k| rd::<u8>(this, 0x30 + 2 * i + k) != 0) {
            continue;
        }
        wr::<u32>(this, 4 * i, task);
        for k in 0..4 {
            wr::<u8>(this, 0x18 + 4 * i + k, ftol32(x(rd::<f32>(probs, 4 * k))) as u8);
        }
        wr::<u8>(this, 0x30 + 2 * i, u8::from(rd::<u32>(flags, 0) != 0));
        wr::<u8>(this, 0x31 + 2 * i, u8::from(rd::<u32>(flags, 4) != 0));
        return;
    }
}

/// `CTaskSimpleIKManager::Clone` (0x639350): a new manager (0x1C bytes: `CTaskSimple`, vtable 0x86E358, the 4
/// slots +8 and the byte +0x18 zeroed; the original goes on with a null result) gets the clones of the 4 IK tasks.
#[hook(addr = 0x639350, conv = thiscall, name = "CTaskSimpleIKManager::Clone", expect = "6A FF 68 AB 12 84 00",
       unverified = "ai8-5", route = "L2", hazards = "heap-writes")]
pub fn ik_manager_clone(this: GPtr<u8>) -> GPtr<u8> {
    let mem = task_new_o(0x1C);
    let c = if mem.is_null() {
        GPtr::NULL
    } else {
        task_simple_ctor(mem);
        wr::<u32>(mem, 0, 0x86E358);
        for o in [8, 0xC, 0x10, 0x14] {
            wr::<u32>(mem, o, 0);
        }
        wr::<u8>(mem, 0x18, 0);
        mem
    };
    for i in 0..4isize {
        let t = rd::<GPtr<u8>>(this, 8 + 4 * i);
        if !t.is_null() {
            let v = v_clone(t);
            wr::<u32>(c, 8 + 4 * i, v);
        }
    }
    c
}

/// The stand-still task the scripted brain falls back to: a `CTaskSimple` (0x18 bytes) with vtable 0x85A278, +8,
/// +0xC, +0x10, +0x11 zeroed and +0x14 = 2000 ms; null when the pool is full.
fn brain_fallback_task() -> u32 {
    let t = task_new_o(0x18);
    if t.is_null() {
        return 0;
    }
    task_simple_ctor(t);
    wr::<u32>(t, 0, 0x85A278);
    wr::<u32>(t, 8, 0);
    wr::<u32>(t, 0xC, 0);
    wr::<u8>(t, 0x10, 0);
    wr::<u8>(t, 0x11, 0);
    wr::<u32>(t, 0x14, 2000);
    t.addr() as u32
}

/// The ped's task in `CScriptedBrainTaskStore` (the inline lookup of `GetTask`), null without an entry.
fn brain_task_of(ped: u32) -> u32 {
    for i in 0..0x30usize {
        if g::<u32>(0xC188F0 + i * 0x10).read() == ped {
            return g::<u32>(0xC188FC + i * 0x10).read();
        }
    }
    0
}

/// `CTaskComplexUseScriptedBrain::CreateFirstSubTask(CPed*)` (0x635A90, `ret 4`): the ped's stored brain task
/// becomes current (+0x18) and the result is its clone; without one +0x18 = null and the result is the
/// stand-still fallback (vtable 0x85A278, 2000 ms).
#[hook(addr = 0x635A90, conv = thiscall, name = "CTaskComplexUseScriptedBrain::CreateFirstSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "ai8-5", route = "L2", hazards = "heap-writes")]
pub fn use_scripted_brain_create_first_sub_task(this: GPtr<u8>, ped: u32) -> u32 {
    let t = brain_task_of(ped);
    if t != 0 {
        wr::<u32>(this, 0x18, t);
        return v_clone(GPtr::from_addr(t as usize));
    }
    wr::<u32>(this, 0x18, 0);
    brain_fallback_task()
}

/// `CTaskComplexUseScriptedBrain::CreateNextSubTask(CPed*)` (0x6359C0, `ret 4`): a current brain task (+0x18) is
/// cleared from the store; after the fallback stand-still (sub-task type 0x709) the result is null; otherwise the
/// ped's stored task becomes current and is cloned, or the fallback is created.
#[hook(addr = 0x6359C0, conv = thiscall, name = "CTaskComplexUseScriptedBrain::CreateNextSubTask", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "ai8-5", route = "L2", hazards = "heap-writes")]
pub fn use_scripted_brain_create_next_sub_task(this: GPtr<u8>, ped: u32) -> u32 {
    let cur = rd::<u32>(this, 0x18);
    if cur != 0 {
        brain_store_clear_task(cur);
        wr::<u32>(this, 0x18, 0);
    }
    if v_task_type(rd::<GPtr<u8>>(this, 8)) == 0x709 {
        return 0;
    }
    let t = brain_task_of(ped);
    wr::<u32>(this, 0x18, t);
    if t != 0 {
        return v_clone(GPtr::from_addr(t as usize));
    }
    brain_fallback_task()
}

/// `CPedGroup::FindDistanceToNearestMember(CPed** outPed)` (0x5FB0A0, `ret 4`; ST0): with a leader (+0x28) the
/// smallest distance from it to the 7 followers (+0xC; `sqrt(dz^2 + dx^2 + dy^2)` through an f32 slot), starting
/// from [0x86C84C]. The original writes every follower it measures to `*outPed` (when given), not only the
/// nearest one. Without a leader the start value.
#[hook(addr = 0x5FB0A0, conv = thiscall, name = "CPedGroup::FindDistanceToNearestMember", expect = "D9 05 4C C8 86 00",
       unverified = "ai8-5", route = "L2", hazards = "x87-transcendental,heap-writes")]
pub fn ped_group_find_distance_to_nearest_member(this: GPtr<u8>, out: GPtr<u8>) -> X87 {
    let mut best = x(g::<f32>(0x86C84C).read());
    let leader = rd::<GPtr<u8>>(this, 0x28);
    if leader.is_null() {
        return best;
    }
    let pos = |e: GPtr<u8>| {
        let m = rd::<GPtr<u8>>(e, 0x14);
        if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
    };
    for i in 0..7isize {
        let m = rd::<GPtr<u8>>(this, 0xC + 4 * i);
        if m.is_null() {
            continue;
        }
        let (a, b) = (pos(m), pos(leader));
        let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(b, 0));
        let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(b, 4));
        let dz = x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(b, 8));
        let d = f32_of(dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy)).fsqrt());
        if best.fcom(x(d)).test_parity(0x05) {
            best = x(d);
        }
        if !out.is_null() {
            wr::<GPtr<u8>>(out, 0, m);
        }
    }
    best
}

/// `CEventGlobalGroup::GetSoundLevel(const CEntity*, CVector& pos)` (0x4AB900, `ret 8`; ST0): over the group's
/// events (count +8, re-read every step; events +0xC) whose source is the entity (or for every event without one),
/// each positive `CEvent::GetSoundLevel` (through an f32 slot) is folded in as `CalcSoundLevelIncrement(sum,
/// level) + level` (sum kept as an f32). Returns the sum.
#[hook(addr = 0x4AB900, conv = thiscall, name = "CEventGlobalGroup::GetSoundLevel", expect = "51 53 55 8B E9",
       unverified = "ai8-5", route = "L2", hazards = "x87-transcendental")]
pub fn event_global_group_get_sound_level(this: GPtr<u8>, ent: u32, pos: u32) -> X87 {
    let mut sum = 0.0f32;
    let mut i = 0i32;
    while i < rd::<i32>(this, 8) {
        let ev = rd::<GPtr<u8>>(this, 0xC + 4 * i as isize);
        if v_source_entity(ev) == ent || ent == 0 {
            let lvl = f32_of(X87::from_f64_exact(event_get_sound_level(rd::<GPtr<u8>>(this, 0xC + 4 * i as isize), ent, pos)));
            if !x(lvl).fcom(x(g::<f32>(0x858B50).read())).test_any(0x41) {
                let inc = X87::from_f64_exact(event_calc_sound_level_increment(sum.to_bits(), lvl.to_bits()));
                sum = f32_of(inc.fadd_m(lvl));
            }
        }
        i += 1;
    }
    x(sum)
}

/// `CPedGeometryAnalyser::GetNearestPed(const CVector&)` (0x5F3590, cdecl): the ped of the ped pool ([0xB74490]:
/// objects +0, flags +4, size +8; 0x7C4 bytes each), walked from the last slot down, with the smallest squared
/// distance (`dz^2 + dx^2 + dy^2`, kept as an f32, strictly below the best, from FLT_MAX); null for an empty pool.
#[hook(addr = 0x5F3590, conv = cdecl, name = "CPedGeometryAnalyser::GetNearestPed", expect = "83 EC 08 57 8B 3D 90 44 B7 00",
       unverified = "ai8-5", route = "L2", hazards = "none")]
pub fn geom_get_nearest_ped(pos: GPtr<u8>) -> u32 {
    let pool = g::<GPtr<u8>>(0xB74490).read();
    let mut best = f32::from_bits(0x7F7F_FFFF);
    let mut res = 0u32;
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
            continue;
        }
        let ped = (rd::<u32>(pool, 0)).wrapping_add((i as u32).wrapping_mul(0x7C4));
        if ped == 0 {
            continue;
        }
        let pp = GPtr::<u8>::from_addr(ped as usize);
        let m = rd::<GPtr<u8>>(pp, 0x14);
        let q = if m.is_null() { pp.byte_add(4) } else { m.byte_add(0x30) };
        let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(q, 0));
        let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(q, 4));
        let dz = x(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(q, 8));
        let d = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
        if !d.fcom(x(best)).test_parity(0x05) {
            best = f32_of(d);
            res = ped;
        }
    }
    res
}

/// `CInterestingEvents::GetInterestingEvent` (0x6028A0): with flag 4 of +0x12C the current event (+0x154, signed,
/// -1 = none; events +0 of 12 bytes: type, time +4, entity +8) is kept while it has an entity and `now <
/// time + duration[type]` (u16 at +0x7E). Otherwise, among the 8 live events, one with a higher priority (byte
/// +0x60 + type) than the best so far is taken, an equal or lower one only when `rand() & 0xFFFF < 0x80`
/// (`rand` is called only then); the choice is stored in +0x154 (byte) and returned (null for none).
#[hook(addr = 0x6028A0, conv = thiscall, name = "CInterestingEvents::GetInterestingEvent", expect = "83 EC 08 53 56 8B F1",
       unverified = "ai8-5", route = "L2", hazards = "heap-writes")]
pub fn interesting_events_get_interesting_event(this: GPtr<u8>) -> u32 {
    let now = g::<u32>(0xB7CB84).read();
    let live = |e: GPtr<u8>| {
        rd::<u32>(e, 8) != 0
            && now < u32::from(rd::<u16>(this, 0x7E + 2 * rd::<i32>(e, 0) as isize)).wrapping_add(rd::<u32>(e, 4))
    };
    let cur = rd::<i8>(this, 0x154);
    if rd::<u8>(this, 0x12C) & 4 != 0 && cur != -1 {
        let e = this.byte_add(i32::from(cur) as isize * 12);
        if live(e) {
            return e.addr() as u32;
        }
    }
    let mut best_pri = 0i32;
    let mut best = -1i32;
    for i in 0..8isize {
        let e = this.byte_add(12 * i);
        if !live(e) {
            continue;
        }
        let pri = i32::from(rd::<u8>(this, 0x60 + rd::<i32>(e, 0) as isize));
        if pri > best_pri || sa_core::orig::at_site(0x60291B, rand) & 0xFFFF < 0x80 {
            best_pri = i32::from(rd::<u8>(this, 0x60 + rd::<i32>(e, 0) as isize));
            best = i as i32;
        }
    }
    wr::<u8>(this, 0x154, best as u8);
    if best == -1 { 0 } else { (this.addr() as u32).wrapping_add(12 * best as u32) }
}

/// The car-door tasks' urgent abort: the anim (+0xC) blends out at -1000.0; with a vehicle (+0x10) the task's
/// `ComputeAnimID(group, anim)` (into a stack slot and, in the original, the priority argument's slot) is passed
/// to the vehicle's door-anim virtual (+0x70) with the ped, the door (+0x14) and 1.0.
fn car_door_urgent_abort(this: GPtr<u8>, ped: u32, compute: fn(GPtr<u8>, GPtr<u8>, GPtr<u8>)) {
    let a = rd::<GPtr<u8>>(this, 0xC);
    if !a.is_null() {
        wr::<u32>(a, 0x1C, 0xC47A_0000);
    }
    if rd::<u32>(this, 0x10) != 0 {
        let mut ga = [0u32; 2];
        let p: GPtr<u8> = GPtr::from_raw(ga.as_mut_ptr()).cast();
        compute(this, p, p.byte_add(4));
        v_vehicle_door_anim(rd::<GPtr<u8>>(this, 0x10), ped, rd::<u32>(this, 0x14), ga[0], ga[1], 0x3F80_0000);
    }
}

/// `CTaskSimpleCarCloseDoorFromInside::MakeAbortable(CPed*, int32 priority, const CEvent*)` (0x64AF50, `ret 0xC`;
/// AL): an urgent abort (priority 2) runs the door-anim abort; AL = 0 in every case.
#[hook(addr = 0x64AF50, conv = thiscall, name = "CTaskSimpleCarCloseDoorFromInside::MakeAbortable", expect = "51 83 7C 24 0C 02",
       unverified = "ai8-5", route = "L2", hazards = "ax-al-return,heap-writes,uninit-locals")]
pub fn close_door_inside_make_abortable(this: GPtr<u8>, ped: u32, prio: i32, _ev: u32) -> u8 {
    if prio == 2 {
        car_door_urgent_abort(this, ped, close_door_inside_compute_anim_id);
    }
    0
}

/// `CTaskSimpleCarCloseDoorFromOutside::MakeAbortable` (0x64B020, `ret 0xC`; AL): the door-anim abort and AL = 1
/// for priority 2, else AL = 0.
#[hook(addr = 0x64B020, conv = thiscall, name = "CTaskSimpleCarCloseDoorFromOutside::MakeAbortable", expect = "51 83 7C 24 0C 02",
       unverified = "ai8-5", route = "L2", hazards = "ax-al-return,heap-writes,uninit-locals")]
pub fn close_door_outside_make_abortable(this: GPtr<u8>, ped: u32, prio: i32, _ev: u32) -> u8 {
    if prio != 2 {
        return 0;
    }
    car_door_urgent_abort(this, ped, close_door_outside_compute_anim_id);
    1
}

/// `CTaskSimpleCarSlowDragPedOut::MakeAbortable` (0x64BFB0, `ret 0xC`; AL): as the close-door-from-outside one
/// with `ComputeAnimID_Wrapper`.
#[hook(addr = 0x64BFB0, conv = thiscall, name = "CTaskSimpleCarSlowDragPedOut::MakeAbortable", expect = "51 83 7C 24 0C 02",
       unverified = "ai8-5", route = "L2", hazards = "ax-al-return,heap-writes,uninit-locals")]
pub fn slow_drag_ped_out_make_abortable(this: GPtr<u8>, ped: u32, prio: i32, _ev: u32) -> u8 {
    if prio != 2 {
        return 0;
    }
    car_door_urgent_abort(this, ped, slow_drag_ped_out_compute_anim_id);
    1
}

/// `CTaskSimpleCarSlowBeDraggedOut::MakeAbortable` (0x64C080, `ret 0xC`; AL): as the close-door-from-outside one
/// with its own `ComputeAnimID`.
#[hook(addr = 0x64C080, conv = thiscall, name = "CTaskSimpleCarSlowBeDraggedOut::MakeAbortable", expect = "51 83 7C 24 0C 02",
       unverified = "ai8-5", route = "L2", hazards = "ax-al-return,heap-writes,uninit-locals")]
pub fn slow_be_dragged_out_make_abortable(this: GPtr<u8>, ped: u32, prio: i32, _ev: u32) -> u8 {
    if prio != 2 {
        return 0;
    }
    car_door_urgent_abort(this, ped, slow_be_dragged_out_compute_anim_id);
    1
}

/// `CTaskAllocator::IsFinished(CPedGroup*)` (0x69C3C0, `ret 4`, ECX unused; AL): true when none of the 8 entries
/// (+0x10, 0x14 bytes each) is set.
#[hook(addr = 0x69C3C0, conv = thiscall, name = "CTaskAllocator::IsFinished", expect = "8B 4C 24 04 33 C0",
       unverified = "ai8-5", route = "L2", hazards = "ax-al-return")]
pub fn task_allocator_is_finished(_this: u32, group: GPtr<u8>) -> u8 {
    for i in 0..8isize {
        if rd::<u32>(group, 0x10 + 0x14 * i) != 0 {
            return 0;
        }
    }
    1
}

/// `CTaskSimpleGoToPointFine::CTaskSimpleGoToPointFine(float moveRatio, CVector target, float radius, CEntity*)`
/// (0x65EEB0, `ret 0x18`): the `CTaskSimpleGoTo` base with move state 4, the target (passed by value: its stack
/// copy, here a Rust local) and the radius; vtable 0x86F964; +0x20 = the move ratio.
#[hook(addr = 0x65EEB0, conv = thiscall, name = "CTaskSimpleGoToPointFine::Constructor", expect = "8B 44 24 14 56 8B F1",
       unverified = "ai8-5", route = "L2", hazards = "heap-writes")]
pub fn go_to_point_fine_ctor(this: GPtr<u8>, ratio: u32, tx: u32, ty: u32, tz: u32, radius: u32, _e: u32) -> GPtr<u8> {
    let mut v = [tx, ty, tz];
    simple_go_to_ctor(this, 4, GPtr::from_raw(v.as_mut_ptr()).cast(), radius);
    wr::<u32>(this, 0, 0x86F964);
    wr::<u32>(this, 0x20, ratio);
    this
}

/// `CTask::IsTaskPtr(const CTask*)` (0x61A4B0, cdecl; AL): whether the pointer is a used slot of the task pool
/// ([0xB744A8]: objects +0, flags +4, size +8; 0x80 bytes each), walked from the last slot down.
#[hook(addr = 0x61A4B0, conv = cdecl, name = "CTask::IsTaskPtr", expect = "53 56 8B 35 A8 44 B7 00",
       unverified = "ai8-5", route = "L2", hazards = "ax-al-return")]
pub fn task_is_task_ptr(task: u32) -> u8 {
    let pool = g::<GPtr<u8>>(0xB744A8).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
            continue;
        }
        let o = rd::<u32>(pool, 0).wrapping_add((i as u32) << 7);
        if o != 0 && o == task {
            return 1;
        }
    }
    0
}

/// Empties a `CScriptedBrainTaskStore` entry: deletes its task (+0xC) and drops the ped reference (+0).
fn brain_store_empty(e: usize) {
    let t = g::<u32>(e + 0xC).read();
    if t != 0 {
        v_delete(GPtr::from_addr(t as usize), 1);
    }
    let ped = g::<GPtr<u8>>(e).read();
    g::<u32>(e + 0xC).write(0);
    if !ped.is_null() {
        entity_clean_up_old_reference(ped, g(e));
        g::<u32>(e).write(0);
    }
}

/// `CScriptedBrainTaskStore::Clear(CTask*)` (0x6357F0, cdecl): the entry holding the task is emptied.
#[hook(addr = 0x6357F0, conv = cdecl, name = "CScriptedBrainTaskStore::Clear-task", expect = "8B 54 24 04 33 C9 B8 FC 88 C1 00",
       unverified = "ai8-5", route = "L2", hazards = "heap-writes")]
pub fn scripted_brain_task_store_clear_task(task: u32) {
    for i in 0..0x30usize {
        if g::<u32>(0xC188FC + i * 0x10).read() == task {
            brain_store_empty(0xC188F0 + i * 0x10);
            return;
        }
    }
}

/// `CScriptedBrainTaskStore::Clear(CPed*)` (0x635850, cdecl): the ped's entry is emptied.
#[hook(addr = 0x635850, conv = cdecl, name = "CScriptedBrainTaskStore::Clear-ped", expect = "8B 54 24 04 33 C9 B8 F0 88 C1 00",
       unverified = "ai8-5", route = "L2", hazards = "heap-writes")]
pub fn scripted_brain_task_store_clear_ped(ped: u32) {
    for i in 0..0x30usize {
        if g::<u32>(0xC188F0 + i * 0x10).read() == ped {
            brain_store_empty(0xC188F0 + i * 0x10);
            return;
        }
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_TASK_IS_TASK_PTR,
    HOOK_SCRIPTED_BRAIN_TASK_STORE_CLEAR_TASK,
    HOOK_SCRIPTED_BRAIN_TASK_STORE_CLEAR_PED,
    HOOK_CLOSE_DOOR_INSIDE_MAKE_ABORTABLE,
    HOOK_CLOSE_DOOR_OUTSIDE_MAKE_ABORTABLE,
    HOOK_SLOW_DRAG_PED_OUT_MAKE_ABORTABLE,
    HOOK_SLOW_BE_DRAGGED_OUT_MAKE_ABORTABLE,
    HOOK_TASK_ALLOCATOR_IS_FINISHED,
    HOOK_GO_TO_POINT_FINE_CTOR,
    HOOK_GEOM_GET_NEAREST_PED,
    HOOK_INTERESTING_EVENTS_GET_INTERESTING_EVENT,
    HOOK_PED_GROUP_FIND_DISTANCE_TO_NEAREST_MEMBER,
    HOOK_EVENT_GLOBAL_GROUP_GET_SOUND_LEVEL,
    HOOK_DECISION_ADD,
    HOOK_IK_MANAGER_CLONE,
    HOOK_USE_SCRIPTED_BRAIN_CREATE_FIRST_SUB_TASK,
    HOOK_USE_SCRIPTED_BRAIN_CREATE_NEXT_SUB_TASK,
    HOOK_LEADER_ENTERED_CAR_AS_DRIVER_AFFECTS_PED_GROUP,
    HOOK_PED_LIST_FILL_UP_HOLES,
    HOOK_SCRIPTED_BRAIN_TASK_STORE_SET_TASK,
    HOOK_SEQUENCE_CLONE,
    HOOK_SEEK_COVER_UNTIL_TARGET_DEAD_GET_TARGET_PED,
    HOOK_WAIT_FOR_BUS_PROCESS_PED,
];
