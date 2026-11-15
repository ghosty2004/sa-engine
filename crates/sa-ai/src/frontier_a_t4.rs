//! FRONTIER-2 turn 4, lane A (batch `frontier2-a7`): the group task allocators `CTaskAllocatorKillOnFoot` (0x69BCA0),
//! `CTaskAllocatorPlayerCommandAttack` (0x69C4D0) and `CTaskAllocatorKillThreatsDriveby` (0x69CC30) with the
//! `CPedList` helpers they reach, ported from the 1.0 US bytes (gta-reversed read as an aid; where it differs the
//! bytes win, noted per function).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{
    point_list_empty_b,
    compute_closest_peds_o, group_intel_flush_tasks, kill_ped_group_on_foot_ctor, kill_ped_group_on_foot_dtor,
    pgi_set_task, run_timed_anim_ctor2,
};
use crate::common::*;
use crate::frontier_a_t3::{kill_then_look_about, set_response};
use sa_core::bindings::task_new;
use sa_core::crt::rand_at;
use sa_math::fpu::ftol32;
use sa_world::bindings::{
    crt_printf, dtor_6389f0 as sequence_dtor, gang_drive_by_ctor, interesting_events_add, kill_ped_on_foot_ctor, leave_car_ctor,
    membership_get_leader, membership_get_member, sequence_add_task, sequence_ctor, stand_still_ctor,
    tm_find_task_by_type, weapon_is_type_melee,
};
use sa_world::world::common::{entity_pos, ped_active_weapon};

orig! {
    /// `CPedList::Empty()` (0x699DB0)
    fn ped_list_empty_o(l: GPtr<u8>) -> u32 = thiscall 0x699DB0 => ped_list_empty;
    /// `CPedList::BuildListOfPedsOfPedType(int32)` (0x69A3B0, `ret 4`)
    fn ped_list_build_of_ped_type(l: GPtr<u8>, ty: i32) -> u32 = thiscall 0x69A3B0 => ped_list_build_list_of_peds_of_ped_type;
    /// `CPedList::BuildListFromGroup_NoLeader(CPedGroupMembership&)` (0x699DD0, `ret 4`)
    fn ped_list_build_from_group_no_leader(l: GPtr<u8>, m: GPtr<u8>) -> u32 = thiscall 0x699DD0 => crate::group_list_loops::ped_list_build_list_from_group_no_leader;
    /// `CPedList::RemovePedsThatDontListenToPlayer()` (0x69A420)
    fn ped_list_remove_peds_that_dont_listen(l: GPtr<u8>) -> u32 = thiscall 0x69A420 => crate::group_list_loops::ped_list_remove_peds_that_dont_listen_to_player;
    /// `CPedList::RemovePedsAttackingPedType(int32)` (0x69A450, `ret 4`)
    fn ped_list_remove_peds_attacking(l: GPtr<u8>, ty: u32) -> u32 = thiscall 0x69A450 => crate::group_list_loops::ped_list_remove_peds_attacking_ped_type;
    /// `CPedList::ExtractPedsWithGuns(CPedList&)` (0x69A4C0, `ret 4`)
    fn ped_list_extract_peds_with_guns(l: GPtr<u8>, from: GPtr<u8>) -> u32 = thiscall 0x69A4C0 => crate::group_list_loops::ped_list_extract_peds_with_guns;
    /// `CFormation::FindCoverPoints(CVector pos, float radius)` (0x69B860, cdecl, the vector by value)
    fn formation_find_cover_points(x: u32, y: u32, z: u32, radius: f32) = cdecl 0x69B860 => unported;
    /// `CFormation::DistributeDestinations_CoverPoints(CPedList&, CVector pos)` (0x69B5B0, cdecl, the vector by value)
    fn formation_distribute_cover_points(l: GPtr<u8>, x: u32, y: u32, z: u32) = cdecl 0x69B5B0 => unported;
    /// `CFormation::DistributeDestinations_PedsToAttack(CPedList&)` (0x69B700, cdecl)
    fn formation_distribute_peds_to_attack(l: GPtr<u8>) = cdecl 0x69B700 => unported;
    /// `CFormation::ReturnTargetPedForPed(CPed*, int32* out)` (0x699F50, cdecl)
    fn formation_return_target_ped_for_ped(ped: u32, out: GPtr<u8>) -> u32 = cdecl 0x699F50 => crate::group_list_loops::formation_return_target_ped_for_ped;
    /// `CPedGroupIntelligence::GetTask(CPed*, CPedTaskPair*)` (0x5F7660, `ret 8`)
    fn pgi_get_task(i: GPtr<u8>, ped: GPtr<u8>, pairs: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F7660 => crate::group_task_pairs::pgi_get_task;
    /// `CTaskAllocatorAttack::AllocateTasks(CPedGroupIntelligence*)` (0x69C3F0, `ret 4`)
    fn allocator_attack_allocate_tasks(t: GPtr<u8>, intel: GPtr<u8>) = thiscall 0x69C3F0 => crate::frontier_a::allocator_attack_allocate_tasks;
    /// `CTaskSimpleGangDriveBy::Destructor` (0x6218C0)
    fn gang_drive_by_dtor(t: GPtr<u8>) = thiscall 0x6218C0 => crate::frontier_c_dtors::gang_drive_by_dtor;
    /// task virtual +0x10 (`GetTaskType`)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
}

/// `CFormation::m_DestinationPeds` (0xC1A458, a `CPedList`)
const DESTINATION_PEDS: usize = 0xC1A458;
/// `g_InterestingEvents` (0xC0B058)
const INTERESTING_EVENTS: usize = 0xC0B058;

/// `CPedList::Empty()` (0x699DB0, its first store in the protection stub 0x407E60): count 0 and the 30 entries
/// cleared; EAX = 0 (the `rep stosd` value).
#[hook(addr = 0x699DB0, conv = thiscall, name = "CPedList::Empty", expect = "57 90 E9 A9 E0 D6 FF", this = "CPedList",
       unverified = "frontier2-a7", route = "L2", hazards = "none")]
pub fn ped_list_empty(this: GPtr<u8>) -> u32 {
    wr::<u32>(this, 0, 0);
    for i in 0..30 {
        wr::<u32>(this, 4 + i * 4, 0);
    }
    0
}

/// `CPedList::BuildListOfPedsOfPedType(int32 type)` (0x69A3B0, `ret 4`): count 0; the ped pool (0xB74490) walked
/// from its last slot down, every live ped (flag bit 7 clear, a non-null address) of the type (+0x598) added while
/// the count is below 30; the entries after the count cleared. EAX: 0 after the clearing, else the count (30).
#[hook(addr = 0x69A3B0, conv = thiscall, name = "CPedList::BuildListOfPedsOfPedType", expect = "C7 01 00 00 00 00 57",
       this = "CPedList", unverified = "frontier2-a7", route = "L2", hazards = "none")]
pub fn ped_list_build_list_of_peds_of_ped_type(this: GPtr<u8>, ty: i32) -> u32 {
    wr::<u32>(this, 0, 0);
    let pool = g::<GPtr<u8>>(0xB74490).read();
    let mut i = rd::<u32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<u8>(rd::<GPtr<u8>>(pool, 4), i as isize) & 0x80 != 0 {
            continue;
        }
        let p = rd::<u32>(pool, 0).wrapping_add(i.wrapping_mul(0x7C4));
        if p == 0 || rd::<i32>(GPtr::from_addr(p as usize), 0x598) != ty {
            continue;
        }
        let n = rd::<i32>(this, 0);
        if n < 0x1E {
            wr::<u32>(this, 4 + n as isize * 4, p);
            wr::<i32>(this, 0, n + 1);
        }
    }
    let n = rd::<i32>(this, 0);
    if n >= 0x1E {
        return n as u32;
    }
    for k in n..0x1E {
        wr::<u32>(this, 4 + k as isize * 4, 0);
    }
    0
}

/// A member whose current group task (the +0xC pairs) is one of these keeps it: the gun control, gang drive-by and
/// kill-on-foot variants 0x3FC / 0x3FE / 0x3FF / 0x400 / 0x402, 0x404 / 0x405, the arrest 0x44D and the group kill
/// 0x5DE (the jump tables at 0x69C220 / 0x69C230).
fn keeps_task(intel: GPtr<u8>, ped: GPtr<u8>) -> bool {
    let t = pgi_get_task(intel, ped, intel.byte_add(0xC));
    !t.is_null() && matches!(v_task_type(t), 0x3FC | 0x3FE | 0x3FF | 0x400 | 0x402 | 0x404 | 0x405 | 0x44D | 0x5DE)
}

/// `new CTaskComplexLeaveCar(vehicle, 0, 0, true, false)` added to the sequence when the ped is in a vehicle (+0x46C
/// bit 8, +0x58C).
fn leave_car_first(seq: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 || rd::<u32>(ped, 0x58C) == 0 {
        return;
    }
    let t = task_new(0x34);
    let t = if t.is_null() { GPtr::NULL } else { leave_car_ctor(t, rd::<GPtr<u8>>(ped, 0x58C), 0, 0, 1, 0) };
    sequence_add_task(seq, t);
}

/// `new CTaskComplexKillPedGroupOnFoot(gid, target)` (null on a failed `new`).
fn new_kill_group(gid: i32, target: GPtr<u8>) -> GPtr<u8> {
    let t = task_new(0x14);
    if t.is_null() { GPtr::NULL } else { kill_ped_group_on_foot_ctor(t, gid, target) }
}

/// `new CTaskComplexKillPedOnFoot(target, -1, 0, 0, 0, flag)` (null on a failed `new`).
fn new_kill_on_foot(target: GPtr<u8>, flag: u32) -> GPtr<u8> {
    let t = task_new(0x38);
    if t.is_null() { GPtr::NULL } else { kill_ped_on_foot_ctor(t, target, -1, 0, 0, 0, flag) }
}

/// A stack `CTaskComplexSequence`, built by `fill` and destroyed after `use_it`.
fn with_sequence(fill: impl FnOnce(GPtr<u8>), use_it: impl FnOnce(GPtr<u8>)) {
    let mut seq = [0u32; 0x10];
    let sp: GPtr<u8> = GPtr::from_raw(seq.as_mut_ptr()).cast();
    sequence_ctor(sp);
    fill(sp);
    use_it(sp);
    sequence_dtor(sp);
}

/// `CTaskAllocatorKillOnFoot::AllocateTasks(CPedGroupIntelligence*)` (0x69BCA0, `ret 4`; SEH frame for the stack
/// tasks). The group's members without the leader (those that do not listen to a player leader removed). When the
/// flag byte 0xC1A554 is set, or the target (+4) exists and does not hold a melee weapon: the members with guns get
/// cover points around the target (radius 30) and, unless they keep their task, a sequence (leave car; then the group
/// kill of group +8 or, with group -1, kill-on-foot flag 2 and a `LookAbout` for `1000 - _ftol(frac * -1000)` ms) as
/// their event response. Then every member (the gunmen again included), unless it keeps its task, gets a sequence
/// (leave car; the group kill, or kill-on-foot flag 1) through `SetTask` on the +0xC pairs. gta-reversed leaves out
/// the flag byte and sets the second list's sequence as an event response.
#[hook(addr = 0x69BCA0, conv = thiscall, name = "CTaskAllocatorKillOnFoot::AllocateTasks", expect = "6A FF 68 03 7C 84 00",
       unverified = "frontier2-a7", route = "L2", hazards = "heap-writes", recs = "police-gang",
       note = "the stack lists and tasks are Rust buffers; the SEH frame is not reproduced")]
pub fn kill_on_foot_allocate_tasks(this: GPtr<u8>, intel: GPtr<u8>) {
    let mut guns = [0u32; 0x1F];
    let mut members = [0u32; 0x1F];
    let mut points = [0u32; 0x49];
    let gp: GPtr<u8> = GPtr::from_raw(guns.as_mut_ptr()).cast();
    let mp: GPtr<u8> = GPtr::from_raw(members.as_mut_ptr()).cast();
    ped_list_empty_o(gp);
    ped_list_empty_o(mp);
    point_list_empty_b(GPtr::from_raw(points.as_mut_ptr()).cast());
    let membership = || rd::<GPtr<u8>>(intel, 0).byte_add(8);
    ped_list_build_from_group_no_leader(mp, membership());
    if !membership_get_leader(membership()).is_null() && ped_is_player(membership_get_leader(membership())) != 0 {
        ped_list_remove_peds_that_dont_listen(mp);
    }
    let target = rd::<GPtr<u8>>(this, 4);
    if g::<u8>(0xC1A554).read() != 0 || (!target.is_null() && weapon_is_type_melee(ped_active_weapon(target)) == 0) {
        ped_list_extract_peds_with_guns(gp, mp);
        let p = entity_pos(rd::<GPtr<u8>>(this, 4));
        formation_find_cover_points(rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8), 30.0);
        let p = entity_pos(rd::<GPtr<u8>>(this, 4));
        formation_distribute_cover_points(gp, rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8));
        let mut i = 0;
        while i < rd::<i32>(gp, 0) {
            let ped = rd::<GPtr<u8>>(gp, 4 + i as isize * 4);
            if !keeps_task(intel, ped) {
                with_sequence(|sp| gunman_sequence(this, sp, ped), |sp| set_response(intel, ped, sp));
            }
            i += 1;
        }
    }
    let mut i = 0;
    while i < rd::<i32>(mp, 0) {
        let ped = rd::<GPtr<u8>>(mp, 4 + i as isize * 4);
        if !keeps_task(intel, ped) {
            with_sequence(
                |sp| {
                    leave_car_first(sp, ped);
                    let gid = rd::<i32>(this, 8);
                    let t = if gid != -1 {
                        new_kill_group(gid, rd::<GPtr<u8>>(this, 4))
                    } else {
                        new_kill_on_foot(rd::<GPtr<u8>>(this, 4), 1)
                    };
                    sequence_add_task(sp, t);
                },
                |sp| pgi_set_task(intel, ped, sp, intel.byte_add(0xC), -1, 0),
            );
        }
        i += 1;
    }
}

/// The gunman's sequence of 0x69BCA0 (0x69BE64..0x69BFEB).
fn gunman_sequence(this: GPtr<u8>, sp: GPtr<u8>, ped: GPtr<u8>) {
    leave_car_first(sp, ped);
    let gid = rd::<i32>(this, 8);
    if gid != -1 {
        sequence_add_task(sp, new_kill_group(gid, rd::<GPtr<u8>>(this, 4)));
        return;
    }
    sequence_add_task(sp, new_kill_on_foot(rd::<GPtr<u8>>(this, 4), 2));
    let t = task_new(0x34);
    let t = if t.is_null() {
        GPtr::NULL
    } else {
        let r = rand_at(0x69BF8B) & 0xFFFF;
        let v = ftol32(xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(-1000.0));
        run_timed_anim_ctor2(t, 0, 9, 0x40800000, 0xC0800000, 1000u32.wrapping_sub(v as u32), 0x1A6, 0x85A18C, 0);
        wr::<u32>(t, 0, 0x85A164);
        t
    };
    sequence_add_task(sp, t);
}

/// `CTaskAllocatorPlayerCommandAttack::AllocateTasks(CPedGroupIntelligence*)` (0x69C4D0, `ret 4`). A negative ped type
/// to attack (+0x1C): `CTaskAllocatorAttack::AllocateTasks` when there is a target (+4). Else +0x10 = now, +0x14 =
/// 3000, +0x18 = 1; the members without the leader (those that do not listen to a player leader and those already
/// attacking the type removed), `CFormation::m_DestinationPeds` = the peds of the type, the attackers distributed;
/// each attacker gets, through `SetTask` on the +0xC pairs, a sequence (leave car; kill-on-foot of its formation
/// target, flag 2; stand still 999999 ms). gta-reversed tests `IsPedTypePlayer` and uses `SetEventResponseTask`.
#[hook(addr = 0x69C4D0, conv = thiscall, name = "CTaskAllocatorPlayerCommandAttack::AllocateTasks",
       expect = "6A FF 64 A1 00 00 00 00", unverified = "frontier2-a7", route = "L2", hazards = "heap-writes,uninit-locals",
       note = "ReturnTargetPedForPed leaves the stack target unwritten when the ped has no slot (zero here)")]
pub fn player_command_attack_allocate_tasks(this: GPtr<u8>, intel: GPtr<u8>) {
    let mut list = [0u32; 0x1F];
    let lp: GPtr<u8> = GPtr::from_raw(list.as_mut_ptr()).cast();
    ped_list_empty_o(lp);
    if rd::<i32>(this, 0x1C) < 0 {
        if rd::<u32>(this, 4) != 0 {
            allocator_attack_allocate_tasks(this, intel);
        }
        return;
    }
    wr::<u32>(this, 0x10, g::<u32>(0xB7CB84).read());
    wr::<u32>(this, 0x14, 0xBB8);
    wr::<u8>(this, 0x18, 1);
    let membership = || rd::<GPtr<u8>>(intel, 0).byte_add(8);
    ped_list_build_from_group_no_leader(lp, membership());
    if !membership_get_leader(membership()).is_null() && ped_is_player(membership_get_leader(membership())) != 0 {
        ped_list_remove_peds_that_dont_listen(lp);
    }
    ped_list_remove_peds_attacking(lp, rd::<u32>(this, 0x1C));
    ped_list_build_of_ped_type(g::<u8>(DESTINATION_PEDS), rd::<i32>(this, 0x1C));
    formation_distribute_peds_to_attack(lp);
    let mut i = 0;
    while i < rd::<i32>(lp, 0) {
        let ped = rd::<GPtr<u8>>(lp, 4 + i as isize * 4);
        with_sequence(
            |sp| {
                leave_car_first(sp, ped);
                let mut target = 0u32;
                formation_return_target_ped_for_ped(ped.addr() as u32, GPtr::from_raw(&mut target).cast());
                sequence_add_task(sp, new_kill_on_foot(GPtr::from_addr(target as usize), 2));
                let t = task_new(0x20);
                let t = if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 999_999, 1, 0, 8.0) };
                sequence_add_task(sp, t);
            },
            |sp| pgi_set_task(intel, ped, sp, intel.byte_add(0xC), -1, 0),
        );
        i += 1;
    }
}

/// Whether a member is skipped by the drive-by allocator: none, a player, or the driver of its vehicle.
fn driveby_skips(m: GPtr<u8>) -> bool {
    if m.is_null() || ped_is_player(m) != 0 {
        return true;
    }
    rd::<u32>(m, 0x46C) & 0x100 != 0 && rd::<GPtr<u8>>(rd::<GPtr<u8>>(m, 0x58C), 0x460) == m
}

/// A stack `CTaskSimpleGangDriveBy(target, null, 60, 90, AI_ALL_DIRN, false)` set as the member's event response.
fn driveby_response(intel: GPtr<u8>, m: GPtr<u8>, target: GPtr<u8>) {
    let mut task = [0u32; 0x14];
    let tp: GPtr<u8> = GPtr::from_raw(task.as_mut_ptr()).cast();
    gang_drive_by_ctor(tp, target, GPtr::NULL, 0x42700000, 0x5A, 8, 0);
    set_response(intel, m, tp);
    gang_drive_by_dtor(tp);
}

/// `CTaskAllocatorKillThreatsDriveby::AllocateTasks(CPedGroupIntelligence*)` (0x69CC30, `ret 4`): both task pair sets
/// flushed; nothing without a threat (+4). A threat in a group (its primary task 0xF3, group id +0xC): the same group
/// as ours prints 0x870EC0 and returns; else every member that is not a player or a driver gets, against its closest
/// threat (`ComputeClosestPeds`), a drive-by (in a vehicle) or the group kill, then interesting event 0x18. A lone
/// threat: a drive-by against it (in a vehicle) or the kill-on-foot / look-about sequence, then event 0x17.
#[hook(addr = 0x69CC30, conv = thiscall, name = "CTaskAllocatorKillThreatsDriveby::AllocateTasks",
       expect = "55 8B EC 83 E4 F8 6A FF", unverified = "frontier2-a7", route = "L2", hazards = "heap-writes",
       note = "the stack tasks are Rust buffers; the SEH frame is not reproduced")]
pub fn kill_threats_driveby_allocate_tasks(this: GPtr<u8>, intel: GPtr<u8>) {
    group_intel_flush_tasks(intel, intel.byte_add(0xC), GPtr::NULL);
    group_intel_flush_tasks(intel, intel.byte_add(0xAC), GPtr::NULL);
    let ours = rd::<GPtr<u8>>(intel, 0);
    let members = ours.byte_add(8);
    let threat = rd::<GPtr<u8>>(this, 4);
    if threat.is_null() {
        return;
    }
    let bg = tm_find_task_by_type(rd::<GPtr<u8>>(threat, 0x47C).byte_add(4), 3, 0xF3);
    if bg.is_null() {
        for i in 0..8 {
            let m = membership_get_member(members, i);
            if driveby_skips(m) {
                continue;
            }
            if rd::<u32>(m, 0x46C) & 0x100 != 0 {
                driveby_response(intel, m, rd::<GPtr<u8>>(this, 4));
            } else {
                kill_then_look_about(intel, m, this);
            }
        }
        interesting_events_add(g(INTERESTING_EVENTS), 0x17, membership_get_leader(members));
        return;
    }
    let gid = rd::<i32>(bg, 0xC);
    let theirs = GPtr::<u8>::from_addr((0xC0_9920 + gid.wrapping_mul(0x2D4)) as u32 as usize);
    if theirs == ours {
        crt_printf(0x87_0EC0);
        return;
    }
    let mut closest = [0u32; 8];
    compute_closest_peds_o(ours, theirs, GPtr::from_raw(closest.as_mut_ptr()).cast());
    for i in 0..8 {
        let m = membership_get_member(members, i);
        if driveby_skips(m) {
            continue;
        }
        let target = GPtr::<u8>::from_addr(closest[i as usize] as usize);
        if rd::<u32>(m, 0x46C) & 0x100 != 0 {
            driveby_response(intel, m, target);
        } else {
            let mut task = [0u32; 8];
            let tp: GPtr<u8> = GPtr::from_raw(task.as_mut_ptr()).cast();
            kill_ped_group_on_foot_ctor(tp, gid, target);
            set_response(intel, m, tp);
            kill_ped_group_on_foot_dtor(tp);
        }
    }
    interesting_events_add(g(INTERESTING_EVENTS), 0x18, membership_get_leader(members));
}

orig! {
    /// `FUN_005EB340(CVector* dst, uint32 n, const CVector* val, ...)` (0x5EB340, cdecl): `n` copies of `*val`
    /// (the vector's uninitialised fill; a null `dst` skips the copy, not the count)
    fn vector_fill_n(dst: u32, n: u32, val: u32, vec: u32, val2: u32) = cdecl 0x5EB340 => unported;
    /// `std::vector<CVector>::insert(iterator* ret, iterator where, const CVector& val)` (0x5EDE50, `ret 12`)
    fn vector_insert(v: u32, ret: u32, at: u32, val: u32) -> u32 = thiscall 0x5EDE50 => crate::gen_bodies::g_fun_005ede50_5ede50;
    /// `std::vector<CVector>::push_back(const CVector&)` (0x5EE370, `ret 4`)
    fn vector_push_back(v: u32, val: u32) = thiscall 0x5EE370 => vector3_push_back;
}

/// `std::vector<CVector>` of the shelter displacements (0xC0986C; begin +4 = 0xC09870, end +8 = 0xC09874)
const SHELTER_DISPLACEMENTS: u32 = 0xC0986C;

/// `(end - begin) / 12` of a `std::vector<CVector>` (signed, as the `imul 0x2AAAAAAB` sequence divides).
fn vec3_count(begin: u32, end: u32) -> i32 {
    (end.wrapping_sub(begin) as i32) / 12
}

/// `std::vector<CVector>::push_back(const CVector& val)` (0x5EE370, `ret 4`): with storage and the size below the
/// capacity (unsigned), `*val` is copied to the end (0x5EB340 with one element) and the end moves by 12; else
/// `insert(end, val)` (0x5EDE50, its iterator result written to the argument slot).
#[hook(addr = 0x5EE370, conv = thiscall, name = "std::vector<CVector>::push_back", expect = "53 56 8B F1 57",
       unverified = "frontier2-a7", route = "L2", hazards = "heap-writes")]
pub fn vector3_push_back(this: u32, val: u32) {
    let v = GPtr::<u8>::from_addr(this as usize);
    let begin = rd::<u32>(v, 4);
    let size = if begin == 0 { 0 } else { vec3_count(begin, rd::<u32>(v, 8)) };
    if begin != 0 && (size as u32) < vec3_count(begin, rd::<u32>(v, 0xC)) as u32 {
        let end = rd::<u32>(v, 8);
        vector_fill_n(end, 1, val, this, val);
        wr::<u32>(v, 8, end.wrapping_add(0xC));
        return;
    }
    let mut slot = val;
    vector_insert(this, GPtr::from_raw(&mut slot).addr() as u32, rd::<u32>(v, 8), val);
}

/// `CPedShelterAttractor::GetDisplacement(int32 index)` (0x5EF420, cdecl): the first time (no storage, or empty) five
/// displacements are made: a random angle `rand() * (1 / 32767) * 2pi` (stored as f32) and radius `2 * rand() *
/// (1 / 32767)` give `(cos a * r, sin a * r, 0)` (FCOS / FSIN, each stored as f32); a candidate within 1 of an earlier
/// one (squared `ez^2 + dy^2 + dx^2` below 1, the earlier one's z not offset) is drawn again; each is pushed onto the
/// vector 0xC0986C. Returns `&displacements[index]`.
#[hook(addr = 0x5EF420, conv = cdecl, name = "CPedShelterAttractor::GetDisplacement", expect = "83 EC 20 56 8B 35 70 98 C0 00",
       unverified = "frontier2-a7", route = "L2", hazards = "x87-transcendental,heap-writes")]
pub fn shelter_get_displacement(index: i32) -> u32 {
    let begin = || g::<u32>(0xC09870).read();
    let filled = begin() != 0 && vec3_count(begin(), g::<u32>(0xC09874).read()) != 0;
    if !filled {
        for _ in 0..5 {
            let mut d = loop {
                let r = rand_at(0x5EF467);
                let a = xi(r).fmul_m(f32::from_bits(0x3800_0100)).fmul_m(f32::from_bits(0x40C9_0FDB)).to_f32();
                let r = rand_at(0x5EF484);
                let rad = xi(r).fmul_m(f32::from_bits(0x3800_0100));
                let rad = rad.fadd(rad);
                let dx = x(a).fcos().fmul(rad).to_f32();
                let dy = x(a).fsin().fmul(rad).to_f32();
                if !shelter_too_close(dx, dy) {
                    break [dx, dy, 0.0];
                }
            };
            vector_push_back(SHELTER_DISPLACEMENTS, GPtr::from_raw(d.as_mut_ptr()).addr() as u32);
        }
    }
    begin().wrapping_add((index as u32).wrapping_mul(12))
}

/// Whether `(x, y)` lies within 1 of a displacement already stored (0x5EF4E8..0x5EF531).
fn shelter_too_close(px: f32, py: f32) -> bool {
    let b = g::<u32>(0xC09870).read();
    let n = if b == 0 { 0 } else { vec3_count(b, g::<u32>(0xC09874).read()) };
    for i in 0..n {
        let e = GPtr::<u8>::from_addr(b.wrapping_add(i as u32 * 12) as usize);
        let dx = x(rd::<f32>(e, 0)).fsub_m(px);
        let dy = x(rd::<f32>(e, 4)).fsub_m(py);
        let ez = x(rd::<f32>(e, 8));
        let d = ez.fmul(ez).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        if !sa_math::fpu::ge_or_nan(d, x(1.0)) {
            return true;
        }
    }
    false
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_VECTOR3_PUSH_BACK,
    HOOK_SHELTER_GET_DISPLACEMENT,
    HOOK_PED_LIST_EMPTY,
    HOOK_PED_LIST_BUILD_LIST_OF_PEDS_OF_PED_TYPE,
    HOOK_KILL_ON_FOOT_ALLOCATE_TASKS,
    HOOK_PLAYER_COMMAND_ATTACK_ALLOCATE_TASKS,
    HOOK_KILL_THREATS_DRIVEBY_ALLOCATE_TASKS,
];
