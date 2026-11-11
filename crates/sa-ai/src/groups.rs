//! `CPedGroup`, `CPedGroups`, `CPedGroupMembership` and `CPedGroupIntelligence` bodies not ported by earlier passes
//! (PORT-AI-5), plus `CPedGroupDefaultTaskAllocatorRandom::AllocateDefaultTasks`. Ported from the 1.0 US bytes (the
//! exe wins over gta-reversed). Every call to another catalogued function goes through `orig!` at the original
//! address.
//!
//! `CPedGroup` (0x2D4 bytes, 8 of them at `CPedGroups::ms_groups` 0xC09920, used flags at 0xC098E0): +0 ped (unused
//! here), +4 byte, +8 `CPedGroupMembership` (+0 the group, +4 members[8], member 7 = leader at +0x20, +0x24
//! separation range), +0x30 `CPedGroupIntelligence` (+0 the group; pair arrays of 8 x 0x14 `CPedTaskPair` = ped,
//! task, slot, +0xC dword, +0x10 byte at +0xC, +0xAC, +0x14C, +0x1EC; +0x28C default allocator, +0x290 / +0x294
//! allocators, +0x298 / +0x29C), +0x2C8 dword, +0x2D0 bool (mission group).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CPedGroups::ms_groups`
const GROUPS: usize = 0xC09920;
/// `CPedGroups::ms_activeGroups` (bool[8])
const ACTIVE: usize = 0xC098E0;
/// `sizeof(CPedGroup)`
const GROUP_SIZE: usize = 0x2D4;

orig! {
    /// `CPedGroupIntelligence::CPedGroupIntelligence` (0x5F7250)
    fn pgi_ctor(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F7250 => unported;
    /// `CPedGroupIntelligence::Process` (0x5FC4A0)
    fn pgi_process(i: GPtr<u8>) = thiscall 0x5FC4A0 => unported;
    /// `CPedGroupIntelligence::Flush` (0x5F7350)
    fn pgi_flush(i: GPtr<u8>) = thiscall 0x5F7350 => unported;
    /// `CPedGroupIntelligence::SetDefaultTaskAllocator(const CPedGroupDefaultTaskAllocator&)` (0x5FB280, `ret 4`)
    fn pgi_set_default_task_allocator(i: GPtr<u8>, a: GPtr<u8>) = thiscall 0x5FB280 => unported;
    /// `CPedGroupDefaultTaskAllocators::Get(int32)` (0x5F7200, cdecl)
    fn default_task_allocators_get(ty: i32) -> GPtr<u8> = cdecl 0x5F7200 => unported;
    /// `CPedGroupMembership::Process` (0x5FBA60)
    fn membership_process(m: GPtr<u8>) = thiscall 0x5FBA60 => unported;
    /// `CPedGroupMembership::AddMember(CPed*, int32)` (0x5F6AE0, `ret 8`)
    fn membership_add_member(m: GPtr<u8>, ped: GPtr<u8>, i: i32) = thiscall 0x5F6AE0 => unported;
    /// `CPedGroupMembership::RemoveMember(int32)` (0x5F80D0, `ret 4`)
    fn membership_remove_member(m: GPtr<u8>, i: i32) = thiscall 0x5F80D0 => unported;
    /// `CPedGroupMembership::GetObjectForPedToHold` (0x5F6950, cdecl)
    fn membership_get_object_for_ped_to_hold() -> i32 = cdecl 0x5F6950 => unported;
    /// `CPedGroup::Flush` (0x5FB790)
    fn group_flush(g: GPtr<u8>) = thiscall 0x5FB790 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::IsAlive` (0x5E0170)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPed::GiveObjectToPedToHold(int32 modelIndex, uint8)` (0x5E4390, `ret 8`)
    fn ped_give_object_to_ped_to_hold(p: GPtr<u8>, model: i32, b: u32) = thiscall 0x5E4390 => unported;
    /// `CPedIntelligence::SetPedDecisionMakerType(int32)` (0x600B50, `ret 4`)
    fn intel_set_ped_decision_maker_type(i: GPtr<u8>, ty: i32) = thiscall 0x600B50 => unported;
    /// `CTheScripts::IsPlayerOnAMission` (0x464D50)
    fn scripts_is_player_on_a_mission() -> u8 = cdecl 0x464D50 => unported;
    /// `CTaskComplexGangFollower::CTaskComplexGangFollower(CPedGroup*, CPed*, uint8, CVector, float)` (0x65EAA0,
    /// `ret 0x1C`; the vector by value)
    #[allow(clippy::too_many_arguments)]
    fn gang_follower_ctor(t: GPtr<u8>, g: GPtr<u8>, leader: u32, i: u32, x: u32, y: u32, z: u32, r: u32) -> GPtr<u8> = thiscall 0x65EAA0 => unported;
    /// 0x65EE90 (`ret 4`): a `CTaskComplexGangFollower` flag setter
    fn gang_follower_65ee90(t: GPtr<u8>, b: u32) = thiscall 0x65EE90 => unported;
    /// `CTaskComplexGangLeader::CTaskComplexGangLeader(CPedGroup*)` (0x65DED0, `ret 4`)
    fn gang_leader_ctor(t: GPtr<u8>, g: GPtr<u8>) -> GPtr<u8> = thiscall 0x65DED0 => unported;
    /// `CTask::Clone` (vtable +4)
    fn v_clone(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// scalar deleting destructor (vtable +0)
    fn v_delete(t: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CPedGroupDefaultTaskAllocator::AllocateDefaultTasks(CPedGroup*, CPed*)` (vtable +0)
    fn v_allocate_default_tasks(a: GPtr<u8>, g: GPtr<u8>, ped: GPtr<u8>) = thiscall vslot 0x0 => unported;
}

fn group(i: usize) -> GPtr<u8> {
    GPtr::from_addr(GROUPS + i * GROUP_SIZE)
}

fn member(m: GPtr<u8>, i: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(m, 4 + 4 * i as isize)
}

/// A placeable's position (`m_matrix->pos` or the simple transform).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// The tail shared by `SetLeader` and `AddFollower`: an unarmed ped of a non-mission group may get an object to hold.
fn maybe_give_object(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(rd::<GPtr<u8>>(this, 0), 0x2D0) != 0 {
        return;
    }
    let slot = rd::<i8>(ped, 0x718) as isize;
    if rd::<u32>(ped, slot * 0x1C + 0x5A0) != 0 {
        return;
    }
    let obj = membership_get_object_for_ped_to_hold();
    if obj != -1 {
        ped_give_object_to_ped_to_hold(ped, obj, 1);
    }
}

/// `CPedGroup::CPedGroup` (0x5FC150): zeroes the 8 member slots (+0xC), separation range from [0x86C6BC], the group
/// intelligence (its +0 = this), +0x2D0 = 0, the membership's group = this, +0 = 0, +4 = 1. Returns this.
#[hook(addr = 0x5FC150, conv = thiscall, name = "CPedGroup::Constructor", expect = "56 8B F1 57 33 C0")]
pub fn group_ctor(this: GPtr<u8>) -> GPtr<u8> {
    for i in 0..8 {
        wr::<u32>(this, 0xC + 4 * i, 0);
    }
    wr::<u32>(this, 0x2C, g::<u32>(0x86C6BC).read());
    pgi_ctor(this.byte_add(0x30));
    wr(this, 0x30, this);
    wr::<u8>(this, 0x2D0, 0);
    wr(this, 8, this);
    wr::<u32>(this, 0, 0);
    wr::<u8>(this, 4, 1);
    this
}

/// `CPedGroupIntelligence::CPedGroupIntelligence` (0x5F7250): the four pair arrays (ped 0, task 0, slot -1, +0xC 0,
/// +0x10 byte 0; ped and task written twice, as the original's second loop does), +0x298 / +0x29C = -1, default
/// allocator 0xC09910, +0x290 / +0x294 = 0. The group pointer (+0) is left as it is. Returns this.
#[hook(addr = 0x5F7250, conv = thiscall, name = "CPedGroupIntelligence::Constructor", expect = "56 8B C1 57 33 D2")]
pub fn pgi_ctor_port(this: GPtr<u8>) -> GPtr<u8> {
    for base in [0xC, 0xAC, 0x14C, 0x1EC] {
        for i in 0..8 {
            let p = this.byte_add(base + 0x14 * i);
            wr::<u32>(p, 0, 0);
            wr::<u32>(p, 4, 0);
            wr::<u32>(p, 8, 0xFFFF_FFFF);
            wr::<u32>(p, 0xC, 0);
            wr::<u8>(p, 0x10, 0);
        }
    }
    wr::<u32>(this, 0x298, 0xFFFF_FFFF);
    wr::<u32>(this, 0x29C, 0xFFFF_FFFF);
    wr::<u32>(this, 0x28C, 0xC09910);
    wr::<u32>(this, 0x290, 0);
    wr::<u32>(this, 0x294, 0);
    this
}

/// `CPedGroupMembership::AddMember(CPed*, int32 slot)` (0x5F6AE0, `ret 8`): stores and registers the reference; for a
/// group other than 0xC09928 the original scans the members for the ped and ignores the result (the reads are
/// kept); a non-player member gets decision maker type 0 (`SetPedDecisionMakerType(0)`).
#[hook(addr = 0x5F6AE0, conv = thiscall, name = "CPedGroupMembership::AddMember", expect = "8B 44 24 08 53")]
pub fn membership_add_member_port(this: GPtr<u8>, ped: GPtr<u8>, i: i32) {
    let slot = this.byte_add(4 + 4 * i as isize);
    wr(slot, 0, ped);
    entity_register_reference(ped, slot);
    if this.addr() != 0xC09928 && !ped.is_null() {
        let mut a = 0xC0992C;
        while a < 0xC09948 {
            if g::<GPtr<u8>>(a).read() == ped {
                break;
            }
            a += 4;
        }
    }
    if ped_is_player(rd::<GPtr<u8>>(slot, 0)) == 0 {
        intel_set_ped_decision_maker_type(rd::<GPtr<u8>>(rd::<GPtr<u8>>(slot, 0), 0x47C), 0);
    }
}

/// `CPedGroupIntelligence::SetDefaultTaskAllocator(const CPedGroupDefaultTaskAllocator&)` (0x5FB280, `ret 4`; the
/// store continues through the island 0x4018FD): deletes the 8 default-pair tasks, then lets the allocator assign
/// default tasks to every member (`AllocateDefaultTasks(group, nullptr)`).
#[hook(addr = 0x5FB280, conv = thiscall, name = "CPedGroupIntelligence::SetDefaultTaskAllocator", expect = "8B 44 24 04 53")]
pub fn pgi_set_default_task_allocator_port(this: GPtr<u8>, a: GPtr<u8>) {
    wr(this, 0x28C, a);
    for i in 0..8 {
        let p = this.byte_add(0x1F0 + 0x14 * i);
        let t = rd::<GPtr<u8>>(p, 0);
        if !t.is_null() {
            v_delete(t, 1);
        }
        wr::<u32>(p, 0, 0);
    }
    v_allocate_default_tasks(rd::<GPtr<u8>>(this, 0x28C), rd::<GPtr<u8>>(this, 0), GPtr::NULL);
}

/// `CPedGroupIntelligence::SetDefaultTaskAllocatorType(int32)` (0x5FBB70, `ret 4`): `SetDefaultTaskAllocator(
/// CPedGroupDefaultTaskAllocators::Get(type))` (tail jump; the original stores the allocator into its own argument
/// slot).
#[hook(addr = 0x5FBB70, conv = thiscall, name = "CPedGroupIntelligence::SetDefaultTaskAllocatorType", expect = "8B 44 24 04 50")]
pub fn pgi_set_default_task_allocator_type(this: GPtr<u8>, ty: i32) {
    let a = default_task_allocators_get(ty);
    pgi_set_default_task_allocator(this, a);
}

/// `CPedGroups::RemoveGroup(int32)` (0x5FB870, cdecl): clears the used flag and flushes the group (tail jump) when it
/// is in use.
#[hook(addr = 0x5FB870, conv = cdecl, name = "CPedGroups::RemoveGroup", expect = "8B 4C 24 04 8A 81 E0 98 C0 00")]
pub fn groups_remove_group(i: i32) {
    let used: GPtr<u8> = GPtr::from_addr((ACTIVE as u32).wrapping_add(i as u32) as usize);
    if used.read() == 0 {
        return;
    }
    used.write(0);
    group_flush(GPtr::from_addr((i as u32).wrapping_mul(GROUP_SIZE as u32).wrapping_add(GROUPS as u32) as usize));
}

/// `CPedGroupMembership::SetLeader(CPed*)` (0x5FB9C0, `ret 4`): a ped already among members 0..6 is removed from its
/// slot first, the current leader is removed, the ped becomes member 7; an unarmed leader of a non-mission group may
/// get an object to hold.
#[hook(addr = 0x5FB9C0, conv = thiscall, name = "CPedGroupMembership::SetLeader", expect = "56 57 8B 7C 24 0C")]
pub fn membership_set_leader(this: GPtr<u8>, ped: GPtr<u8>) {
    if !ped.is_null()
        && (0..7).any(|i| member(this, i) == ped)
        && let Some(i) = (0..8).find(|&i| member(this, i) == ped)
    {
        membership_remove_member(this, i);
    }
    if rd::<u32>(this, 0x20) != 0 {
        membership_remove_member(this, 7);
    }
    membership_add_member(this, ped, 7);
    maybe_give_object(this, ped);
}

/// `CPedGroupMembership::Process` (0x5FBA60): removes dead members (a dead player leader stays), promotes the first
/// member to leader when there is none, then removes members (not flagged 0x4000 at +0x475) that are farther from the
/// leader than the separation range (+0x24, strictly, ordered) and in the same area (+0x2F).
#[hook(addr = 0x5FBA60, conv = thiscall, name = "CPedGroupMembership::Process", expect = "56 57 8B F1 33 FF")]
pub fn membership_process_port(this: GPtr<u8>) {
    for i in 0..8 {
        let m = member(this, i);
        if m.is_null() || ped_is_alive(m) != 0 {
            continue;
        }
        if i == 7 && ped_is_player(rd::<GPtr<u8>>(this, 0x20)) != 0 {
            continue;
        }
        membership_remove_member(this, i);
    }
    if rd::<u32>(this, 0x20) == 0 {
        if let Some(i) = (0..7).find(|&i| !member(this, i).is_null()) {
            let p = member(this, i);
            membership_remove_member(this, i);
            membership_add_member(this, p, 7);
        }
        if rd::<u32>(this, 0x20) == 0 {
            return;
        }
    }
    for i in 0..7 {
        let m = member(this, i);
        if m.is_null() || rd::<u32>(m, 0x474) & 0x4000 != 0 {
            continue;
        }
        let leader = rd::<GPtr<u8>>(this, 0x20);
        let lp = entity_pos(leader);
        let mp = entity_pos(m);
        let dx = x(rd::<f32>(mp, 0)).fsub_m(rd::<f32>(lp, 0));
        let dy = x(rd::<f32>(mp, 4)).fsub_m(rd::<f32>(lp, 4));
        let dz = x(rd::<f32>(mp, 8)).fsub_m(rd::<f32>(lp, 8));
        let r = x(rd::<f32>(this, 0x24));
        let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        if r.fmul(r).fcom(d2).test_parity(5) {
            continue;
        }
        if rd::<u8>(m, 0x2F) == rd::<u8>(leader, 0x2F) {
            membership_remove_member(this, i);
        }
    }
}

/// `CPedGroup::Process` (0x5FC7E0): the membership, then (tail jump) the group intelligence.
#[hook(addr = 0x5FC7E0, conv = thiscall, name = "CPedGroup::Process", expect = "56 8B F1 8D 4E 08")]
pub fn group_process(this: GPtr<u8>) {
    membership_process(this.byte_add(8));
    pgi_process(this.byte_add(0x30));
}

/// `CPedGroups::Process` (0x5FC800, static): processes every used group and frees the ones left without members;
/// then the mission-state tracking of 0xC098E8 (on a mission last frame) / 0xC098EC: on entering a mission every
/// non-mission used group's +0x2C8 becomes 9, on leaving one -1 (also when 0xC098EC == 8 on a mission);
/// `IsPlayerOnAMission` is called up to five times, exactly as the original's branches do.
#[hook(addr = 0x5FC800, conv = cdecl, name = "CPedGroups::Process", expect = "53 56 57 33 DB")]
pub fn groups_process() {
    for i in 0..8 {
        let used = g::<u8>(ACTIVE + i);
        if used.read() == 0 {
            continue;
        }
        let gp = group(i);
        membership_process(gp.byte_add(8));
        pgi_process(gp.byte_add(0x30));
        let n = (0..8).filter(|&k| rd::<u32>(gp, 0xC + 4 * k) != 0).count();
        if n == 0 && used.read() != 0 {
            used.write(0);
            group_flush(gp);
        }
    }
    let was = g::<u8>(0xC098E8);
    let count = g::<u32>(0xC098EC);
    let v: Option<u32> = 'v: {
        if scripts_is_player_on_a_mission() != 0 && was.read() == 0 {
            count.write(0);
            break 'v Some(9);
        }
        if scripts_is_player_on_a_mission() == 0 && was.read() != 0 {
            count.write(0);
            break 'v Some(0xFFFF_FFFF);
        }
        if scripts_is_player_on_a_mission() != 0 && count.read() == 8 {
            break 'v Some(0xFFFF_FFFF);
        }
        if scripts_is_player_on_a_mission() != 0 {
            break 'v Some(9);
        }
        None
    };
    if let Some(v) = v {
        for i in 0..8 {
            let gp = group(i);
            if g::<u8>(ACTIVE + i).read() != 0 && rd::<u8>(gp, 0x2D0) == 0 {
                wr::<u32>(gp, 0x2C8, v);
            }
        }
    }
    let on = scripts_is_player_on_a_mission() != 0;
    was.write(on as u8);
    if !on {
        count.write(0);
    }
}

/// `CPedGroupDefaultTaskAllocatorRandom::AllocateDefaultTasks(CPedGroup*, CPed*)` (0x5F6E90, `ret 8`): for each
/// default pair 0..6 whose ped is set (and is the given ped, when one is given) a `CTaskComplexGangFollower(group,
/// leader, i, offset i from the table 0xC196E8 (x, y; z = 0), 10.0)` (0x4C bytes), then 0x65EE90 on it (also on a
/// null task) with the group's +4 flag; the leader pair (7) gets a `CTaskComplexGangLeader(group)` (0x38 bytes).
/// The offset vector is built on the stack through `fld`/`fstp`; the follower pair's task is overwritten without
/// deleting the old one (as the original).
#[hook(addr = 0x5F6E90, conv = thiscall, name = "CPedGroupDefaultTaskAllocatorRandom::AllocateDefaultTasks", expect = "64 A1 00 00 00 00")]
pub fn allocator_random_allocate_default_tasks(_this: GPtr<u8>, grp: GPtr<u8>, ped: GPtr<u8>) {
    for i in 0..7u32 {
        let pair = grp.byte_add(0x21C + 0x14 * i as isize);
        let pp = rd::<GPtr<u8>>(pair, 0);
        if pp.is_null() || (!ped.is_null() && pp != ped) {
            continue;
        }
        let t = task_new(0x4C);
        let task = if t.is_null() {
            GPtr::NULL
        } else {
            let y = f32_of(x(g::<f32>(0xC196EC + 8 * i as usize).read()));
            let leader = rd::<u32>(grp, 0x28);
            let xx = f32_of(x(g::<f32>(0xC196E8 + 8 * i as usize).read()));
            gang_follower_ctor(t, grp, leader, i, xx.to_bits(), y.to_bits(), 0, 0x41200000)
        };
        wr(pair, 4, task);
        let flag = (rd::<u8>(grp, 4) != 0) as u32;
        gang_follower_65ee90(task, flag);
    }
    let lp = rd::<GPtr<u8>>(grp, 0x2A8);
    if !lp.is_null() && (ped.is_null() || lp == ped) {
        let t = task_new(0x38);
        let task = if t.is_null() { GPtr::NULL } else { gang_leader_ctor(t, grp) };
        wr(grp, 0x2AC, task);
    }
}

/// `CPedGroupMembership::CountMembers` (0x5F6A50): the number of non-null slots of the 8.
#[hook(addr = 0x5F6A50, conv = thiscall, name = "CPedGroupMembership::CountMembers", expect = "8B 51 04 33 C0")]
pub fn membership_count_members(this: GPtr<u8>) -> u32 {
    (0..8).filter(|&i| !member(this, i).is_null()).count() as u32
}

/// `CPedGroupMembership::AddFollower(CPed*)` (0x5F8020, `ret 4`): clears ped flag 0x2000000 (+0x474), and 0x100000
/// (+0x470) when the leader has player data (+0x480); a ped that is not yet a member or the leader takes the first
/// free slot of 0..6; an unarmed follower of a non-mission group may get an object to hold.
#[hook(addr = 0x5F8020, conv = thiscall, name = "CPedGroupMembership::AddFollower", expect = "56 8B 74 24 08")]
pub fn membership_add_follower(this: GPtr<u8>, ped: GPtr<u8>) {
    wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & 0xFDFF_FFFF);
    let leader = rd::<GPtr<u8>>(this, 0x20);
    if !leader.is_null() && rd::<u32>(leader, 0x480) != 0 {
        wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) & 0xFFEF_FFFF);
    }
    if (0..7).any(|i| member(this, i) == ped) || rd::<GPtr<u8>>(this, 0x20) == ped {
        return;
    }
    let Some(i) = (0..7).find(|&i| member(this, i).is_null()) else {
        return;
    };
    membership_add_member(this, ped, i);
    maybe_give_object(this, ped);
}

/// `CPedGroupIntelligence::SetTask(CPed*, const CTask&, CPedTaskPair* pairs, int32 slot, bool force)` (0x5F7540,
/// `ret 0x14`): in the ped's pair stores a clone of the task and the slot; an existing task is kept when not forced
/// and of the same type, else it is replaced (the old one deleted after the store).
#[hook(addr = 0x5F7540, conv = thiscall, name = "CPedGroupIntelligence::SetTask", expect = "8B 54 24 04 56")]
pub fn pgi_set_task(_this: GPtr<u8>, ped: GPtr<u8>, task: GPtr<u8>, pairs: GPtr<u8>, slot: u32, force: u8) {
    let Some(i) = (0..8).find(|&i| rd::<GPtr<u8>>(pairs, 0x14 * i) == ped) else {
        return;
    };
    let pair = pairs.byte_add(0x14 * i);
    let cur = rd::<GPtr<u8>>(pair, 4);
    if !cur.is_null() && (force != 0 || v_task_type(cur) != v_task_type(task)) {
        let old = rd::<GPtr<u8>>(pair, 4);
        let c = v_clone(task);
        wr(pair, 4, c);
        wr::<u32>(pair, 8, slot);
        if !old.is_null() {
            v_delete(old, 1);
        }
        return;
    }
    if rd::<u32>(pair, 4) == 0 {
        let c = v_clone(task);
        wr(pair, 4, c);
        wr::<u32>(pair, 8, slot);
    }
}

/// `CPedGroupIntelligence::ReportFinishedTask(const CPed*, const CTask*, CPedTaskPair* pairs)` (0x5F76C0, `ret
/// 0xC`): deletes and clears the ped's pair task when its type is the finished task's; true (AL) when one was found.
#[hook(addr = 0x5F76C0, conv = thiscall, name = "CPedGroupIntelligence::ReportFinishedTask-Impl", expect = "53 55 8B 6C 24 10")]
pub fn pgi_report_finished_task(_this: GPtr<u8>, ped: GPtr<u8>, task: GPtr<u8>, pairs: GPtr<u8>) -> u8 {
    for i in 0..8 {
        let pair = pairs.byte_add(0x14 * i);
        if rd::<GPtr<u8>>(pair, 0) != ped {
            continue;
        }
        let t = rd::<GPtr<u8>>(pair, 4);
        if t.is_null() || v_task_type(t) != v_task_type(task) {
            continue;
        }
        let t = rd::<GPtr<u8>>(pair, 4);
        if !t.is_null() {
            v_delete(t, 1);
        }
        wr::<u32>(pair, 4, 0);
        return 1;
    }
    0
}

/// `CPedGroupIntelligence::GetTaskSecondary(const CPed*)` (0x5F8620, `ret 4`): the ped's secondary pair task, null
/// when the ped has no pair.
#[hook(addr = 0x5F8620, conv = thiscall, name = "CPedGroupIntelligence::GetTaskSecondary", expect = "56 8D B1 AC 00 00 00")]
pub fn pgi_get_task_secondary(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    match (0..8).find(|&i| rd::<GPtr<u8>>(this, 0xAC + 0x14 * i) == ped) {
        Some(i) => rd::<u32>(this, 0xB0 + 0x14 * i),
        None => 0,
    }
}

/// `CPedGroupIntelligence::GetTaskSecondarySlot(const CPed*)` (0x5F8650, `ret 4`): the ped's secondary pair slot, -1
/// when the ped has no pair.
#[hook(addr = 0x5F8650, conv = thiscall, name = "CPedGroupIntelligence::GetTaskSecondarySlot", expect = "56 8D B1 AC 00 00 00")]
pub fn pgi_get_task_secondary_slot(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    match (0..8).find(|&i| rd::<GPtr<u8>>(this, 0xAC + 0x14 * i) == ped) {
        Some(i) => rd::<u32>(this, 0xB4 + 0x14 * i),
        None => 0xFFFF_FFFF,
    }
}

/// `CPedGroup::Flush` (0x5FB790): removes every member, flushes the group intelligence, clears +0x2D0.
#[hook(addr = 0x5FB790, conv = thiscall, name = "CPedGroup::Flush", expect = "53 55 56 8B E9")]
pub fn group_flush_port(this: GPtr<u8>) {
    let m = this.byte_add(8);
    for i in 0..8 {
        if member(m, i).is_null() {
            continue;
        }
        membership_remove_member(m, i);
    }
    pgi_flush(this.byte_add(0x30));
    wr::<u8>(this, 0x2D0, 0);
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_GROUP_CTOR,
    HOOK_PGI_CTOR_PORT,
    HOOK_MEMBERSHIP_ADD_MEMBER_PORT,
    HOOK_PGI_SET_DEFAULT_TASK_ALLOCATOR_PORT,
    HOOK_PGI_SET_DEFAULT_TASK_ALLOCATOR_TYPE,
    HOOK_GROUPS_REMOVE_GROUP,
    HOOK_MEMBERSHIP_SET_LEADER,
    HOOK_MEMBERSHIP_PROCESS_PORT,
    HOOK_GROUP_PROCESS,
    HOOK_GROUPS_PROCESS,
    HOOK_ALLOCATOR_RANDOM_ALLOCATE_DEFAULT_TASKS,
    HOOK_MEMBERSHIP_COUNT_MEMBERS,
    HOOK_MEMBERSHIP_ADD_FOLLOWER,
    HOOK_PGI_SET_TASK,
    HOOK_PGI_REPORT_FINISHED_TASK,
    HOOK_PGI_GET_TASK_SECONDARY,
    HOOK_PGI_GET_TASK_SECONDARY_SLOT,
    HOOK_GROUP_FLUSH_PORT,
];
