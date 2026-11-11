//! WRITER-MVP-B (helperB) AI ports, part 3: `CConversations` queries, `CTaskSimpleDoHandSignal::ProcessPed`, the
//! group-intelligence task lookups used by `CTaskComplexBeInGroup`, `CEvent::GetSoundLevel`,
//! `CTaskComplexWalkAlongsidePed` sub-task creation and `CTaskGoToVehicleAndLean`. Ported from the 1.0 US bytes
//! (the exe wins over gta-reversed). SEH frames are not modelled.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::*;

orig! {
    /// `CConversationNode::ClearRecursively` (0x43A710)
    fn conversation_node_clear_recursively(n: GPtr<u8>) = thiscall 0x43A710 => crate::mvpb_ai::conversation_node_clear_recursively;
    /// `CEntity::GetIsOnScreen` (0x534540)
    fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn task_manager_get_task_secondary(tm: GPtr<u8>, i: u32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskManager::SetTaskSecondary(CTask*, int32)` (0x681B60, `ret 8`)
    fn task_manager_set_task_secondary(tm: GPtr<u8>, t: GPtr<u8>, i: u32) = thiscall 0x681B60 => unported;
    /// `CTaskComplexPlayHandSignalAnim::CTaskComplexPlayHandSignalAnim(AnimationId, float)` (0x61B2B0, `ret 8`)
    fn play_hand_signal_anim_ctor(t: GPtr<u8>, anim: u32, blend: f32) -> GPtr<u8> = thiscall 0x61B2B0 => unported;
    /// `CPedGroupIntelligence::ReportFinishedTask` implementation over one list (0x5F76C0, `ret 0xC`)
    fn pgi_report_finished_task_impl(g: GPtr<u8>, ped: GPtr<u8>, t: GPtr<u8>, list: GPtr<u8>) -> u8 = thiscall 0x5F76C0 => unported;
    /// `CPedGroupIntelligence::ReportFinishedTask` (0x5F86F0, `ret 8`)
    fn pgi_report_finished_task_o(g: GPtr<u8>, ped: GPtr<u8>, t: GPtr<u8>) -> u8 = thiscall 0x5F86F0 => crate::mvpb_ai3::pgi_report_finished_task;
    /// `CPedGroupIntelligence::GetTaskMain(const CPed*)` (0x5F85A0, `ret 4`)
    fn pgi_get_task_main_o(g: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F85A0 => crate::mvpb_ai3::pgi_get_task_main;
    /// `CPedGroupIntelligence::GetTaskSecondary(const CPed*)` (0x5F8620, `ret 4`)
    fn pgi_get_task_secondary(g: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F8620 => unported;
    /// `CPedGroupIntelligence::GetTaskSecondarySlot(const CPed*)` (0x5F8650, `ret 4`)
    fn pgi_get_task_secondary_slot(g: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x5F8650 => unported;
    /// `CTaskComplexLeaveCar::CTaskComplexLeaveCar(CVehicle*, int32, int32, bool, bool)` (0x63B8C0, `ret 0x14`)
    fn leave_car_ctor(t: GPtr<u8>, v: GPtr<u8>, door: u32, delay: u32, sensible: u32, force: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    /// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::CTaskComplexSeekEntity(...)` (0x661DC0, `ret 0x20`)
    fn seek_entity_xy_offset_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, f: u32, g: u32, h: u32) -> GPtr<u8> = thiscall 0x661DC0 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, ms: u32, a: u32, b: u32, f: f32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskComplexWalkAlongsidePed::CreateSubTask(int32, CPed*)` (0x685000, `ret 8`)
    fn walk_alongside_create_sub_task_o(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x685000 => crate::mvpb_ai3::walk_alongside_create_sub_task;
    /// `FUN_00664770(CVector* out, CPed*)` (0x664770, `ret 8`): the lean target position
    fn go_to_vehicle_and_lean_target(t: GPtr<u8>, out: GPtr<f32>, ped: GPtr<u8>) -> GPtr<f32> = thiscall 0x664770 => unported;
    /// `CTaskComplexGoToPointAndStandStill::CTaskComplexGoToPointAndStandStill(int32, const CVector&, float, float, bool, bool)` (0x668120, `ret 0x18`)
    fn go_to_point_and_stand_still_ctor(t: GPtr<u8>, ms: u32, p: GPtr<u8>, r: f32, s: f32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x668120 => unported;
    /// `CPhysical::GetSpeed(CVector point)` (0x542CE0, `ret 0x10`, returns the out pointer)
    fn physical_get_speed(e: GPtr<u8>, out: GPtr<f32>, x: f32, y: f32, z: f32) -> GPtr<f32> = thiscall 0x542CE0 => unported;
    /// `CTaskGoToVehicleAndLean::DoTidyUp(CPed*)` (0x660F60, `ret 4`)
    fn go_to_vehicle_and_lean_tidy_up(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x660F60 => unported;
    /// `CTaskComplexGangLeader::ShouldLoadGangAnims` (0x65E7F0, cdecl)
    fn should_load_gang_anims() -> u8 = cdecl 0x65E7F0 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990, cdecl)
    fn get_animation_block_index(n: u32) -> i32 = cdecl 0x4D3990 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0, cdecl)
    fn add_anim_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0, cdecl)
    fn remove_anim_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0, cdecl)
    fn request_model(id: i32, flags: u32) = cdecl 0x4087E0 => unported;
    /// `CTask::Clone` (vtable +0x4)
    fn v_task_clone(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: u32, e: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CEvent::GetEventType` (vtable +0x4)
    fn v_get_event_type(e: GPtr<u8>) -> u32 = thiscall vslot 0x4 => unported;
    /// `CEvent::GetSourceEntity` (vtable +0x28)
    fn v_get_source_entity(e: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x28 => unported;
    /// `CEventSound...::GetLocalSoundLevel` (vtable +0x30, float in ST0)
    fn v_get_local_sound_level(e: GPtr<u8>) -> f64 = thiscall vslot 0x30 => unported;
}

/// `CConversations::RemoveConversationForPed(CPed*)` (0x43A960 -> .HOODLUM 0x156ADC0, cdecl): every one of the 14
/// conversations (0x9691D8, 0x1C stride) of this ped clears its first node recursively and is reset (node indices
/// -1, ped and the two following fields 0).
#[hook(addr = 0x43A960, conv = cdecl, name = "CConversations::RemoveConversationForPed", expect = "E9 5B 04 13 01",
       unverified = "mvpb-ai-3", route = "L2", hazards = "none")]
pub fn conversations_remove_for_ped(ped: GPtr<u8>) {
    for i in 0..14usize {
        let c: GPtr<u8> = GPtr::from_addr(0x9691E0 + 0x1C * i);
        if rd::<u32>(c, 0) != ped.addr() as u32 {
            continue;
        }
        let n = rd::<i32>(c, -8);
        conversation_node_clear_recursively(GPtr::from_addr(0x969570usize.wrapping_add((n.wrapping_mul(0x18)) as isize as usize)));
        wr::<u32>(c, -8, 0xFFFF_FFFF);
        wr::<u32>(c, -4, 0xFFFF_FFFF);
        wr::<u32>(c, 0, 0);
        wr::<u32>(c, 8, 0);
        wr::<u32>(c, 4, 0);
    }
}

/// `CConversations::IsConversationGoingOn` (0x43AAC0 -> .HOODLUM 0x1570080, cdecl): AL = 1 when any of the 14
/// conversations has a non-zero status (+0x14).
#[hook(addr = 0x43AAC0, conv = cdecl, name = "CConversations::IsConversationGoingOn", expect = "E9 BB 55 13 01",
       unverified = "mvpb-ai-3", route = "L2", hazards = "none")]
pub fn conversations_is_going_on() -> u8 {
    for i in 0..14usize {
        if rd::<u32>(GPtr::from_addr(0x9691EC + 0x1C * i), 0) != 0 {
            return 1;
        }
    }
    0
}

/// `CTaskSimpleDoHandSignal::ProcessPed(CPed*)` (0x660940, `ret 4`): off-screen peds finish (AL = 1). First run
/// (+8 = 0): without a secondary task in slot 4 a `CTaskComplexPlayHandSignalAnim(-1, 4.0)` is set there and +8 = 1
/// (AL = 0); an existing one finishes the task when it is a hand signal (type 0x1AA), else it is asked to abort
/// (prio 1) and AL = 0. Later runs finish when the slot is empty or holds anything but a hand signal.
#[hook(addr = 0x660940, conv = thiscall, name = "CTaskSimpleDoHandSignal::ProcessPed",
       expect = "64 A1 00 00 00 00", unverified = "mvpb-ai-3", route = "L2", hazards = "heap-writes")]
pub fn do_hand_signal_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if entity_get_is_on_screen(ped) == 0 {
        return 1;
    }
    let t = task_manager_get_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 4);
    if rd::<u8>(this, 8) == 0 {
        if t.is_null() {
            let n = task_new(0x18);
            let n = if n.is_null() { GPtr::NULL } else { play_hand_signal_anim_ctor(n, 0xFFFF_FFFF, 4.0) };
            task_manager_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), n, 4);
            wr::<u8>(this, 8, 1);
            return 0;
        }
        if v_get_task_type(t) == 0x1AA {
            return 1;
        }
        v_make_abortable(t, ped, 1, 0);
        return 0;
    }
    if t.is_null() {
        return 1;
    }
    if v_get_task_type(t) == 0x1AA {
        return 0;
    }
    1
}

/// `CPedGroupIntelligence::ReportFinishedTask(const CPed*, const CTask*)` (0x5F86F0, `ret 8`): tries the four task
/// lists in the order +0x14C, +0xC (stops at the first that reports), then +0xAC and +0x1EC unconditionally; AL is
/// the first non-zero report or 0.
#[hook(addr = 0x5F86F0, conv = thiscall, name = "CPedGroupIntelligence::ReportFinishedTask-Wrapper",
       this = "CPedGroupIntelligence", expect = "53 55 8B 6C 24 0C", unverified = "mvpb-ai-3", route = "L2",
       hazards = "heap-writes")]
pub fn pgi_report_finished_task(this: GPtr<u8>, ped: GPtr<u8>, t: GPtr<u8>) -> u8 {
    let r = pgi_report_finished_task_impl(this, ped, t, this.byte_add(0x14C));
    if r != 0 {
        return r;
    }
    let r = pgi_report_finished_task_impl(this, ped, t, this.byte_add(0xC));
    if r != 0 {
        return r;
    }
    pgi_report_finished_task_impl(this, ped, t, this.byte_add(0xAC));
    pgi_report_finished_task_impl(this, ped, t, this.byte_add(0x1EC));
    0
}

/// `CPedGroupIntelligence::GetTaskMain(const CPed*)` (0x5F85A0, `ret 4`): the task (+4) of the ped's entry in the
/// first list (+0x14C, then +0xC, then +0x1EC; 8 entries of 0x14) that has one; 0 when none.
#[hook(addr = 0x5F85A0, conv = thiscall, name = "CPedGroupIntelligence::GetTaskMain", this = "CPedGroupIntelligence",
       expect = "53 8B 5C 24 08", unverified = "mvpb-ai-3", route = "L2", hazards = "none")]
pub fn pgi_get_task_main(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    for base in [0x14Cisize, 0xC, 0x1EC] {
        for i in 0..8isize {
            let e = this.byte_add(base + 0x14 * i);
            if rd::<u32>(e, 0) == ped.addr() as u32 {
                let t = rd::<GPtr<u8>>(e, 4);
                if !t.is_null() {
                    return t;
                }
                break;
            }
        }
    }
    GPtr::NULL
}

fn group_of(this: GPtr<u8>) -> GPtr<u8> {
    GPtr::from_addr((rd::<u32>(this, 0xC).wrapping_mul(0x2D4).wrapping_add(0xC09950)) as usize)
}

/// `CTaskComplexBeInGroup::MonitorSecondaryGroupTask(CPed*)` (0x6330B0, `ret 4`): follows the group's secondary
/// task for this ped: when the stored task (+0x20) is still the group's and the ped's slot is empty, reports it
/// finished and clears +0x20/+0x24; when the group's task changed, aborts the ped's task in the old slot (+0x24,
/// prio 0) and, if the current slot's task aborts (prio 1) or is empty, stores task/slot and gives the ped a clone.
#[hook(addr = 0x6330B0, conv = thiscall, name = "CTaskComplexBeInGroup::MonitorSecondaryGroupTask", this = "CTaskComplexBeInGroup",
       expect = "51 53 55 56 8B D9", unverified = "mvpb-ai-3", route = "L2", hazards = "heap-writes")]
pub fn be_in_group_monitor_secondary(this: GPtr<u8>, ped: GPtr<u8>) {
    let g = group_of(this);
    let task = pgi_get_task_secondary(g, ped);
    let slot = pgi_get_task_secondary_slot(g, ped);
    let tm = |p: GPtr<u8>| rd::<GPtr<u8>>(p, 0x47C).byte_add(4);
    let cur = task_manager_get_task_secondary(tm(ped), slot);
    let stored = rd::<u32>(this, 0x20);
    if stored != 0 && task.addr() as u32 == stored {
        if cur.is_null() {
            pgi_report_finished_task_o(g, ped, GPtr::from_addr(stored as usize));
            wr::<u32>(this, 0x20, 0);
            wr::<u32>(this, 0x24, 0xFFFF_FFFF);
        }
        return;
    }
    if task.addr() as u32 == stored {
        return;
    }
    if !task_manager_get_task_secondary(tm(ped), rd::<u32>(this, 0x24)).is_null() {
        let o = task_manager_get_task_secondary(tm(ped), rd::<u32>(this, 0x24));
        v_make_abortable(o, ped, 0, 0);
    }
    if !cur.is_null() && v_make_abortable(cur, ped, 1, 0) == 0 {
        return;
    }
    wr(this, 0x20, task);
    wr::<u32>(this, 0x24, slot);
    if !task.is_null() {
        let c = v_task_clone(task);
        task_manager_set_task_secondary(tm(ped), c, slot);
    }
}

/// `CTaskComplexBeInGroup::MonitorMainGroupTask(CPed*)` (0x633010, `ret 4`): without a main group task for the ped
/// the sub-task is asked to abort (prio 1) and, when it does, +0x18 = 0, +0x1C = 200 and 0 is returned; a changed
/// group task (pointer +0x18 or type +0x1C) whose predecessor aborts is stored and its clone returned; otherwise the
/// current sub-task (read on entry).
#[hook(addr = 0x633010, conv = thiscall, name = "CTaskComplexBeInGroup::MonitorMainGroupTask", this = "CTaskComplexBeInGroup",
       expect = "53 8B 5C 24 08", unverified = "mvpb-ai-3", route = "L2", hazards = "heap-writes")]
pub fn be_in_group_monitor_main(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let g = group_of(this);
    let sub = rd::<GPtr<u8>>(this, 8);
    let t = pgi_get_task_main_o(g, ped);
    if t.is_null() {
        if v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0 {
            wr::<u32>(this, 0x18, 0);
            wr::<u32>(this, 0x1C, 0xC8);
            return GPtr::NULL;
        }
        return sub;
    }
    if t.addr() as u32 == rd::<u32>(this, 0x18) && v_get_task_type(t) == rd::<u32>(this, 0x1C) {
        return sub;
    }
    if v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) == 0 {
        return sub;
    }
    wr(this, 0x18, t);
    let ty = v_get_task_type(t);
    wr::<u32>(this, 0x1C, ty);
    v_task_clone(t)
}

/// `CEvent::GetSoundLevel(const CEntity*, CVector& pos)` (0x4B2850, `ret 8`, float in ST0): 0 when an entity is
/// given and is not the source, or when the local level (vslot 0x30) is 0; otherwise the distance from the sound's
/// position (event type 0x3E: +0x20; else the source entity's position; 1000 without a source) clamped to >= 1,
/// and `level - 10*log10(d*d) - 0.01*d`. Kept original details: the distance (f32) and the log are stored into the
/// caller's (callee-popped) argument slots.
#[hook(addr = 0x4B2850, conv = thiscall, name = "CEvent::GetSoundLevel", this = "CEvent", expect = "56 57 8B 7C 24 0C",
       unverified = "mvpb-ai-3", route = "L2", hazards = "x87-transcendental",
       note = "FYL2X (log10) result; writes f32 temporaries into its own argument slots (not modelled)")]
pub fn event_get_sound_level(this: GPtr<u8>, ent: GPtr<u8>, pos: GPtr<u8>) -> X87 {
    if !ent.is_null() && v_get_source_entity(this).addr() != ent.addr() {
        return X87::zero();
    }
    if !X87::from_f64_exact(v_get_local_sound_level(this)).fcom(X87::zero()).test_parity(0x44) {
        return X87::zero();
    }
    let mut dist = 1000.0f32;
    let from = if v_get_event_type(this) == 0x3E {
        Some(this.byte_add(0x20))
    } else if !v_get_source_entity(this).is_null() {
        let s = v_get_source_entity(this);
        let m = rd::<GPtr<u8>>(s, 0x14);
        Some(if m.is_null() { s.byte_add(4) } else { m.byte_add(0x30) })
    } else {
        None
    };
    if let Some(p) = from {
        let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(p, 0));
        let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(p, 4));
        let dz = x(rd::<f32>(pos, 8)).fsub_m(rd::<f32>(p, 8));
        let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
        dist = f32_of(d);
        if !d.fcom(x(1.0)).test_parity(5) {
            dist = 1.0;
        }
    }
    let lvl = X87::from_f64_exact(v_get_local_sound_level(this));
    let d2 = x(dist).fmul_m(dist);
    let lg = X87::lg2().fyl2x(d2);
    lvl.fsub(lg.fmul_m(10.0)).fsub(x(dist).fmul_m(0.01))
}

/// `CTaskComplexWalkAlongsidePed::CreateSubTask(int32 type, CPed*)` (0x685000, `ret 8`): 0x2C0 -> leave the ped's
/// vehicle (`CTaskComplexLeaveCar(vehicle, 0, 0, true, false)`); 0x38B -> `CTaskComplexSeekEntity<XYOffset>(target
/// +0xC, 50000, 1000, [0x86FC2C] * 0.5, [0x870660], [0x86FC30], 0, 0)` with the offset +0x14 copied to +0x44, flag 8
/// in +0x54 and +0x50 = 7 (written even when the allocation failed, as the original); 0xCB -> stand still (10 ms,
/// 8.0); else 0.
#[hook(addr = 0x685000, conv = thiscall, name = "CTaskComplexWalkAlongsidePed::CreateSubTask",
       expect = "90 E9 9C C7 D7 FF", unverified = "mvpb-ai-3", route = "L2", hazards = "heap-writes,entry-protection-read")]
pub fn walk_alongside_create_sub_task(this: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> {
    match ty {
        0x2C0 => {
            let t = task_new(0x34);
            if t.is_null() {
                return GPtr::NULL;
            }
            leave_car_ctor(t, rd::<GPtr<u8>>(ped, 0x58C), 0, 0, 1, 0)
        }
        0x38B => {
            let t = task_new(0x58);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                let r = f32_of(x(rd::<f32>(g::<u8>(0x86FC2C), 0)).fmul_m(0.5));
                seek_entity_xy_offset_ctor(
                    t,
                    rd::<GPtr<u8>>(this, 0xC),
                    50000,
                    1000,
                    r.to_bits(),
                    rd::<u32>(g::<u8>(0x870660), 0),
                    rd::<u32>(g::<u8>(0x86FC30), 0),
                    0,
                    0,
                )
            };
            let (a, b, c) = (rd::<u32>(this, 0x14), rd::<u32>(this, 0x18), rd::<u32>(this, 0x1C));
            wr::<u32>(t, 0x44, a);
            wr::<u32>(t, 0x48, b);
            wr::<u32>(t, 0x4C, c);
            wr::<u8>(t, 0x54, rd::<u8>(t, 0x54) | 8);
            wr::<u32>(t, 0x50, 7);
            t
        }
        0xCB => {
            let t = task_new(0x20);
            if t.is_null() {
                return GPtr::NULL;
            }
            stand_still_ctor(t, 10, 0, 0, 8.0)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexWalkAlongsidePed::CreateFirstSubTask(CPed*)` (0x685630, `ret 4`): 0 without a target (+0xC); a ped
/// sitting in a vehicle (+0x46C bit 0x100 and +0x58C) leaves it (0x2C0), else seeks the target (0x38B).
#[hook(addr = 0x685630, conv = thiscall, name = "CTaskComplexWalkAlongsidePed::CreateFirstSubTask",
       expect = "8B 41 0C 85 C0", unverified = "mvpb-ai-3", route = "L2", hazards = "heap-writes")]
pub fn walk_alongside_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(this, 0xC) == 0 {
        return GPtr::NULL;
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 && rd::<u32>(ped, 0x58C) != 0 {
        walk_alongside_create_sub_task_o(this, 0x2C0, ped)
    } else {
        walk_alongside_create_sub_task_o(this, 0x38B, ped)
    }
}

fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CTaskGoToVehicleAndLean::CreateFirstSubTask(CPed*)` (0x664D40, `ret 4`): with a vehicle (+0xC) not yet leaned on
/// (flag 1 of +0x42E, set now; an already set flag is cleared and nothing created), stores the lean target (+0x18)
/// and, when the squared distance to the ped is not below 1 (x87, unordered included), returns
/// `CTaskComplexGoToPointAndStandStill(4, target, 0.05, 2.0, false, true)`. Otherwise the sub-task (+8, if any) is
/// asked to abort (prio 2) and 0 returned.
#[hook(addr = 0x664D40, conv = thiscall, name = "CTaskGoToVehicleAndLean::CreateFirstSubTask", this = "CTaskGoToVehicleAndLean",
       expect = "64 A1 00 00 00 00", unverified = "mvpb-ai-3", route = "L2", hazards = "heap-writes")]
pub fn go_to_vehicle_and_lean_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let veh = rd::<GPtr<u8>>(this, 0xC);
    if !veh.is_null() {
        let b = rd::<u8>(veh, 0x42E);
        if b & 1 != 0 {
            wr::<u8>(veh, 0x42E, rd::<u8>(veh, 0x42E) & 0xFE);
        } else {
            wr::<u8>(veh, 0x42E, b | 1);
            let mut out = [0.0f32; 3];
            let r = go_to_vehicle_and_lean_target(this, GPtr::from_raw(out.as_mut_ptr()), ped);
            for k in 0..3usize {
                wr::<f32>(this, 0x18 + 4 * k as isize, r.wrapping_index(k as i32).read());
            }
            let p = entity_pos(ped);
            let dz = x(rd::<f32>(this, 0x20)).fsub_m(rd::<f32>(p, 8));
            let dy = x(rd::<f32>(this, 0x1C)).fsub_m(rd::<f32>(p, 4));
            let dx = x(rd::<f32>(this, 0x18)).fsub_m(rd::<f32>(p, 0));
            let s = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz));
            if !s.fcom(x(1.0)).test_parity(5) {
                return GPtr::NULL;
            }
            let t = task_new(0x28);
            if t.is_null() {
                return GPtr::NULL;
            }
            return go_to_point_and_stand_still_ctor(t, 4, this.byte_add(0x18), 0.05, 2.0, 0, 1);
        }
    }
    let sub = rd::<GPtr<u8>>(this, 8);
    if !sub.is_null() {
        v_make_abortable(sub, ped, 2, 0);
    }
    GPtr::NULL
}

/// `CTaskGoToVehicleAndLean::ControlSubTask(CPed*)` (0x664E60, `ret 4`): without a vehicle, or when the ped's
/// intelligence value +0x274 exceeds [0x86C938] (the vehicle's lean flag is then cleared), the sub-task is aborted
/// (prio 2) and 0 returned. When the vehicle is attached (+0x460) or moving (speed at the ped's offset > 0.01), or
/// the lean target moved more than sqrt(0.01), and the sub-task aborts (prio 1), `DoTidyUp` and 0. Otherwise loads
/// (`ShouldLoadGangAnims`) or releases the lean anim block (0x86F92C, flag +0x14) and returns the sub-task.
#[hook(addr = 0x664E60, conv = thiscall, name = "CTaskGoToVehicleAndLean::ControlSubTask", this = "CTaskGoToVehicleAndLean",
       expect = "83 EC 18 56 8B F1", unverified = "mvpb-ai-3", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "fsqrt of the vehicle speed compared, not stored")]
pub fn go_to_vehicle_and_lean_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let veh = rd::<GPtr<u8>>(this, 0xC);
    let abort2 = |this: GPtr<u8>| {
        let sub = rd::<GPtr<u8>>(this, 8);
        if !sub.is_null() {
            v_make_abortable(sub, ped, 2, 0);
        }
        GPtr::NULL
    };
    if veh.is_null() {
        return abort2(this);
    }
    if rd::<i32>(rd::<GPtr<u8>>(ped, 0x47C), 0x274) > g::<i32>(0x86C938).read() {
        wr::<u8>(veh, 0x42E, rd::<u8>(veh, 0x42E) & 0xFE);
        return abort2(this);
    }
    let vp = entity_pos(veh);
    let pp = entity_pos(ped);
    let ox = f32_of(x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(vp, 0)));
    let oy = f32_of(x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(vp, 4)));
    let oz = f32_of(x(rd::<f32>(pp, 8)).fsub_m(rd::<f32>(vp, 8)));
    let mut out = [0.0f32; 3];
    let sp = physical_get_speed(veh, GPtr::from_raw(out.as_mut_ptr()), ox, oy, oz);
    let moving = if rd::<u32>(rd::<GPtr<u8>>(this, 0xC), 0x460) != 0 {
        true
    } else {
        let (a, b, c) = (x(sp.read()), x(sp.wrapping_index(1).read()), x(sp.wrapping_index(2).read()));
        let s = a.fmul(a).fadd(b.fmul(b)).fadd(c.fmul(c)).fsqrt();
        !s.fcom(x(0.01)).test_any(0x41)
    };
    if moving && v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0 {
        go_to_vehicle_and_lean_tidy_up(this, ped);
        return GPtr::NULL;
    }
    let mut tgt = [0.0f32; 3];
    go_to_vehicle_and_lean_target(this, GPtr::from_raw(tgt.as_mut_ptr()), ped);
    let dx = x(tgt[0]).fsub_m(rd::<f32>(this, 0x18));
    let dy = x(tgt[1]).fsub_m(rd::<f32>(this, 0x1C));
    let dz = x(tgt[2]).fsub_m(rd::<f32>(this, 0x20));
    let s = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if s.fcom(x(f32::from_bits(0x3C23_D70B))) == Cmp::Greater && v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0 {
        go_to_vehicle_and_lean_tidy_up(this, ped);
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0x14) == 0 {
        if should_load_gang_anims() == 0 {
            return rd::<GPtr<u8>>(this, 8);
        }
        let i = get_animation_block_index(0x86F92C);
        if rd::<u8>(g::<u8>(0xB5D4B0), (i.wrapping_mul(32)) as isize) != 1 {
            request_model(i.wrapping_add(0x63E7), 8);
            return rd::<GPtr<u8>>(this, 8);
        }
        add_anim_block_ref(i);
        wr::<u8>(this, 0x14, 1);
        return rd::<GPtr<u8>>(this, 8);
    }
    if should_load_gang_anims() == 0 {
        let i = get_animation_block_index(0x86F92C);
        remove_anim_block_ref(i);
        wr::<u8>(this, 0x14, 0);
    }
    rd::<GPtr<u8>>(this, 8)
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_CONVERSATIONS_REMOVE_FOR_PED,
    HOOK_CONVERSATIONS_IS_GOING_ON,
    HOOK_DO_HAND_SIGNAL_PROCESS_PED,
    HOOK_PGI_REPORT_FINISHED_TASK,
    HOOK_PGI_GET_TASK_MAIN,
    HOOK_BE_IN_GROUP_MONITOR_SECONDARY,
    HOOK_BE_IN_GROUP_MONITOR_MAIN,
    HOOK_EVENT_GET_SOUND_LEVEL,
    HOOK_WALK_ALONGSIDE_CREATE_SUB_TASK,
    HOOK_WALK_ALONGSIDE_CREATE_FIRST_SUB_TASK,
    HOOK_GO_TO_VEHICLE_AND_LEAN_CREATE_FIRST_SUB_TASK,
    HOOK_GO_TO_VEHICLE_AND_LEAN_CONTROL_SUB_TASK,
];
