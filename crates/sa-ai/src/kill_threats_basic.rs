//! FRONTIER-2 (batch `frontier2-1`): the melee fight's group reactions — `CTaskSimpleFight::MakeAbortable`
//! (0x6239F0) and `CTaskAllocatorKillThreatsBasic` (`ComputeClosestPeds` 0x69C850, `AllocateTasks` 0x69D170). Ported
//! from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::bindings::{general_random_int_in_range, ped_is_alive, task_new};
use sa_core::{hook, orig};
use sa_anim::bindings::anim_set_delete_callback;
use sa_math::fpu::{gt, le_or_nan};
use sa_world::bindings::{
    anim_blend_animation, interesting_events_add, kill_ped_on_foot_ctor, look_about_ctor, membership_get_leader,
    membership_get_member, ped_groups_get_peds_group, sequence_add_task, sequence_ctor,
};
use sa_world::world::common::entity_pos;

use crate::bindings::{fight_61c9b0, get_group_id, group_intel_flush_tasks, task_simple_ctor, task_simple_dtor};
use sa_world::bindings::{crt_printf, dtor_6389f0 as sequence_dtor};
use crate::common::*;

orig! {
    /// virtual `CEvent::GetEventPriority()` (vtable +8)
    fn v_event_priority(e: GPtr<u8>) -> i32 = thiscall vslot 0x8 => unported;
    /// `CTaskAllocatorKillThreatsBasic::ComputeClosestPeds(CPedGroup&, CPedGroup&, CPed** closest)` (0x69C850)
    fn compute_closest_peds_o(group: GPtr<u8>, threats: GPtr<u8>, closest: GPtr<u8>) = cdecl 0x69C850 => compute_closest_peds;
    /// `CPedGroupIntelligence::SetEventResponseTask(CPed*, bool, const CTask&, bool, const CTask&, int32)` (0x5F8510)
    fn group_intel_set_event_response_task(gi: GPtr<u8>, ped: GPtr<u8>, has_main: u32, main: GPtr<u8>, has_sec: u32,
        sec: GPtr<u8>, slot: i32) -> u8 = thiscall 0x5F8510 => unported;
    /// `CTaskComplexKillPedGroupOnFoot::Constructor(int32 groupId, CPed*)` (0x622240)
    fn kill_ped_group_on_foot_ctor(t: GPtr<u8>, gid: i32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x622240 => unported;
    /// `CTaskComplexKillPedGroupOnFoot::Destructor` (0x6222B0)
    fn kill_ped_group_on_foot_dtor(t: GPtr<u8>) = thiscall 0x6222B0 => unported;
}

/// `CTaskSimpleFight::MakeAbortable(CPed* ped, int32 priority, const CEvent* event)` (0x6239F0, `ret 0xC`).
///
/// Priorities other than 1 / 2 only request the abort (+0x26 = 1, false). An event of priority (vslot 8, asked twice)
/// up to 0x1F or exactly 0x3C refuses. The strike anim (+0x1C) loses its delete callback (an immediate abort first
/// sets its blend delta to -1000); the idle anim (+0x20) too, and when it was blending in (amount > 0, delta >= 0)
/// the ped's anim group blends anim 3 back at 1000 (immediate) or 16. A player's +0xC / +0x10 player-data fields are
/// cleared and 0x61C9B0 runs. Aborted (+8), true.
#[hook(addr = 0x6239F0, conv = thiscall, name = "CTaskSimpleFight::MakeAbortable", expect = "55 8B 6C 24 0C 83 FD 01",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes")]
pub fn fight_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    if priority != 1 && priority != 2 {
        wr::<u8>(this, 0x26, 1);
        return 0;
    }
    if !event.is_null() && (v_event_priority(event) <= 0x1F || v_event_priority(event) == 0x3C) {
        return 0;
    }
    let a = rd::<GPtr<u8>>(this, 0x1C);
    if !a.is_null() {
        if priority == 2 {
            wr::<u32>(a, 0x1C, 0xC47A_0000);
        }
        anim_set_delete_callback(rd::<GPtr<u8>>(this, 0x1C), 0x4D_1390, 0);
        wr::<u32>(this, 0x1C, 0);
    }
    let b = rd::<GPtr<u8>>(this, 0x20);
    if !b.is_null() {
        anim_set_delete_callback(b, 0x4D_1390, 0);
        let b = rd::<GPtr<u8>>(this, 0x20);
        let zero = x(g::<f32>(0x85_8B50).read());
        if gt(x(rd::<f32>(b, 0x18)), zero) && !x(rd::<f32>(b, 0x1C)).fcom(zero).test_any(0x01) {
            let delta = if priority == 2 { 1000.0 } else { 16.0 };
            anim_blend_animation(rd::<GPtr<u8>>(ped, 0x18), rd::<u32>(ped, 0x4D4), 3, delta);
        }
        wr::<u32>(this, 0x20, 0);
    }
    if !ped.is_null() && ped_is_player(ped) != 0 {
        let pd = rd::<GPtr<u8>>(ped, 0x480).byte_add(0xC);
        wr::<u32>(pd, 0, 0);
        wr::<u32>(pd, 4, 0);
        fight_61c9b0(this, ped);
    }
    wr::<u8>(this, 8, 1);
    1
}

/// `CTaskAllocatorKillThreatsBasic::ComputeClosestPeds(CPedGroup& group, CPedGroup& threats, CPed** closest)`
/// (0x69C850, cdecl).
///
/// `closest[8]` cleared; the 8x8 squared distances (`(dz² + dx²) + dy²`, member minus threat) of every live non-player
/// member to every live threat, FLT_MAX elsewhere. Eight times: the smallest entry (strictly below the running
/// minimum, which starts at `[0x863A3C]`; row-major scan) pairs its member with its threat and clears that row and
/// column. With no live threat leader the first live threat among members 0..6 stands in for it; that ped (or the
/// leader) then fills every unpaired slot of a present member.
#[hook(addr = 0x69C850, conv = cdecl, name = "CTaskAllocatorKillThreatsBasic::ComputeClosestPeds", expect = "81 EC 08 01 00 00 53 55",
       unverified = "frontier2-1", route = "L2", hazards = "none")]
pub fn compute_closest_peds(group: GPtr<u8>, threats: GPtr<u8>, closest: GPtr<u8>) {
    for i in 0..8 {
        wr::<u32>(closest, 4 * i, 0);
    }
    let mut d = [f32::from_bits(0x7F7F_FFFF); 64];
    let gm = group.byte_add(8);
    let tm = threats.byte_add(8);
    for m in 0..8 {
        let ped = membership_get_member(gm, m);
        if ped.is_null() || ped_is_alive(ped) as u8 == 0 || ped_is_player(ped) != 0 {
            continue;
        }
        for t in 0..8 {
            let th = membership_get_member(tm, t);
            if th.is_null() || ped_is_alive(th) as u8 == 0 {
                continue;
            }
            let tp = entity_pos(th);
            let mp = entity_pos(ped);
            let dx = x(rd::<f32>(mp, 0)).fsub_m(rd::<f32>(tp, 0));
            let dy = x(rd::<f32>(mp, 4)).fsub_m(rd::<f32>(tp, 4));
            let dz = x(rd::<f32>(mp, 8)).fsub_m(rd::<f32>(tp, 8));
            d[(m * 8 + t) as usize] = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy)).to_f32();
        }
    }
    for _ in 0..8 {
        let mut min = x(g::<f32>(0x86_3A3C).read());
        let (mut row, mut col) = (-1i32, -1i32);
        for r in 0..8 {
            for c in 0..8 {
                let v = d[r * 8 + c];
                if !le_or_nan(min, x(v)) {
                    min = x(v);
                    row = r as i32;
                    col = c as i32;
                }
            }
        }
        if row >= 0 && col >= 0 {
            for k in 0..8 {
                d[row as usize * 8 + k] = f32::from_bits(0x7F7F_FFFF);
                d[k * 8 + col as usize] = f32::from_bits(0x7F7F_FFFF);
            }
            wr(closest, 4 * row as isize, membership_get_member(tm, col));
        }
    }
    let mut leader = membership_get_leader(tm);
    if leader.is_null() || ped_is_alive(leader) as u8 == 0 {
        let mut found = GPtr::NULL;
        for i in 0..7 {
            let p = membership_get_member(tm, i);
            if !p.is_null() && ped_is_alive(p) as u8 != 0 {
                found = p;
                break;
            }
        }
        if found.is_null() {
            return;
        }
        leader = found;
    }
    for i in 0..8 {
        if !membership_get_member(gm, i).is_null() && rd::<u32>(closest, 4 * i as isize) == 0 {
            wr(closest, 4 * i as isize, leader);
        }
    }
}

/// `CTaskAllocatorKillThreatsBasic::AllocateTasks(CPedGroupIntelligence* intel)` (0x69D170, `ret 4`).
///
/// Flushes the intelligence's main (+0xC) and secondary (+0xAC) task pairs. Nothing more without a threat ped (+4).
/// A threat in a ped group: the same group as ours prints a message (0x870F00) and stops; else every non-player member
/// gets `CTaskComplexKillPedGroupOnFoot(threat group id, its closest threat)` as its event response (with a
/// `CTaskSimpleNone` secondary), and an interesting event 0x18 for our leader. A lone threat: every non-player member
/// gets the sequence `KillPedOnFoot(threat, -1, 0, 0, 0, 1)`, `LookAbout(RandomInt(1000, 2000))`; interesting event
/// 0x17.
#[hook(addr = 0x69D170, conv = thiscall, name = "CTaskAllocatorKillThreatsBasic::AllocateTasks", expect = "55 8B EC 83 E4 F8 6A FF",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the stack tasks are Rust buffers; the SEH frame is not reproduced")]
pub fn kill_threats_basic_allocate_tasks(this: GPtr<u8>, intel: GPtr<u8>) {
    group_intel_flush_tasks(intel, intel.byte_add(0xC), GPtr::NULL);
    group_intel_flush_tasks(intel, intel.byte_add(0xAC), GPtr::NULL);
    let ours = rd::<GPtr<u8>>(intel, 0);
    let members = ours.byte_add(8);
    let threat = rd::<u32>(this, 4);
    if threat == 0 {
        return;
    }
    let tg = ped_groups_get_peds_group(threat);
    let mut none = [0u32; 2];
    let none_p: GPtr<u8> = GPtr::from_raw(none.as_mut_ptr()).cast();
    if tg != 0 {
        let gid = get_group_id(GPtr::from_addr(tg as usize));
        let theirs = GPtr::<u8>::from_addr((0xC0_9920 + gid.wrapping_mul(0x2D4)) as u32 as usize);
        if theirs == ours {
            crt_printf(0x87_0F00);
            return;
        }
        let mut closest = [0u32; 8];
        compute_closest_peds_o(ours, theirs, GPtr::from_raw(closest.as_mut_ptr()).cast());
        for i in 0..8 {
            let m = membership_get_member(members, i);
            if m.is_null() || ped_is_player(m) != 0 {
                continue;
            }
            let mut task = [0u32; 5];
            let tp: GPtr<u8> = GPtr::from_raw(task.as_mut_ptr()).cast();
            kill_ped_group_on_foot_ctor(tp, gid, GPtr::from_addr(closest[i as usize] as usize));
            task_simple_ctor(none_p);
            wr::<u32>(none_p, 0, 0x85_9CFC);
            group_intel_set_event_response_task(intel, m, 1, tp, 0, none_p, -1);
            wr::<u32>(none_p, 0, 0x85_9CFC);
            task_simple_dtor(none_p);
            kill_ped_group_on_foot_dtor(tp);
        }
        interesting_events_add(g(0xC0_B058), 0x18, membership_get_leader(members));
    } else {
        for i in 0..8 {
            let m = membership_get_member(members, i);
            if m.is_null() || ped_is_player(m) != 0 {
                continue;
            }
            let mut seq = [0u32; 0x10];
            let sp: GPtr<u8> = GPtr::from_raw(seq.as_mut_ptr()).cast();
            sequence_ctor(sp);
            let t = task_new(0x38);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                kill_ped_on_foot_ctor(t, rd::<GPtr<u8>>(this, 4), -1, 0, 0, 0, 1)
            };
            sequence_add_task(sp, t);
            let t = task_new(0x34);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                look_about_ctor(t, general_random_int_in_range(1000, 2000) as u32)
            };
            sequence_add_task(sp, t);
            task_simple_ctor(none_p);
            wr::<u32>(none_p, 0, 0x85_9CFC);
            group_intel_set_event_response_task(intel, m, 1, sp, 0, none_p, -1);
            wr::<u32>(none_p, 0, 0x85_9CFC);
            task_simple_dtor(none_p);
            sequence_dtor(sp);
        }
        interesting_events_add(g(0xC0_B058), 0x17, membership_get_leader(members));
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_FIGHT_MAKE_ABORTABLE,
    HOOK_COMPUTE_CLOSEST_PEDS,
    HOOK_KILL_THREATS_BASIC_ALLOCATE_TASKS,
];
