//! Small AI bodies with loops (the AI-GEN interpreter rejects loops): `CPedGroupMembership` member walks,
//! `CPedList` filters, `CFormation` lookups, `CCoverPoint` reservation, the scripted-task status and brain-task
//! lookups and a node-route reversal (PORT-AI-8). Ported from the 1.0 US bytes. Every call to another
//! catalogued function goes through `orig!` at the original address; the EAX a caller may see is returned where
//! the original leaves a defined value.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPedGroupMembership::RemoveMember(int32 memberIndex)` (0x5F80D0, `ret 4`)
    fn membership_remove_member(m: GPtr<u8>, i: i32) -> u32 = thiscall 0x5F80D0 => unported;
    /// `CPedGroupMembership::AddMember(CPed*, int32 memberIndex)` (0x5F6AE0, `ret 8`)
    fn membership_add_member(m: GPtr<u8>, ped: u32, i: i32) -> u32 = thiscall 0x5F6AE0 => unported;
    /// `CPedGroupMembership::GetMember(int32)` (0x5F69B0, `ret 4`)
    fn membership_get_member(m: GPtr<u8>, i: i32) -> u32 = thiscall 0x5F69B0 => unported;
    /// `CPedList::FillUpHoles` (0x699E20)
    fn ped_list_fill_up_holes(l: GPtr<u8>) -> u32 = thiscall 0x699E20 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0, `ret 4`)
    fn intel_find_task_by_type(i: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CWeapon::IsTypeMelee` (0x73B1C0)
    fn weapon_is_type_melee(w: GPtr<u8>) -> u8 = thiscall 0x73B1C0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) -> u32 = thiscall 0x571B70 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CPedIntelligence::IsInACarOrEnteringOne` (0x601CC0)
    fn intel_is_in_a_car_or_entering_one(i: GPtr<u8>) -> u32 = thiscall 0x601CC0 => unported;
    /// `CPedGroupMembership::GetLeader` (0x5F69A0)
    fn membership_get_leader(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F69A0 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18, `ret 0xC`)
    fn v_make_abortable(t: GPtr<u8>, ped: u32, prio: i32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTask::Clone` (vtable +4)
    fn v_clone(t: GPtr<u8>) -> u32 = thiscall vslot 0x4 => unported;
}

/// `CPedScriptedTaskRecord::GetStatus(const CPed*)` (0x608750, cdecl): 1 when any of the 128 records (0xC0B1F4,
/// 0x14 bytes each) has the ped at +0, else 0 (zero-extended to EAX).
#[hook(addr = 0x608750, conv = cdecl, name = "CPedScriptedTaskRecord::GetStatus-", expect = "8B 4C 24 04 32 D2",
       unverified = "ai8-4", route = "L2", hazards = "none")]
pub fn scripted_task_record_get_status(ped: u32) -> u32 {
    let mut a = 0xC0B1F4usize;
    while a < 0xC0BBF4 {
        if g::<u32>(a).read() == ped {
            return 1;
        }
        a += 0x14;
    }
    0
}

/// `CScriptedBrainTaskStore::GetTask(const CPed*)` (0x6357C0, cdecl): the task (+0xC) of the first of the 48
/// entries (0xC188F0, 0x10 bytes each) whose ped (+0) is the ped, else null.
#[hook(addr = 0x6357C0, conv = cdecl, name = "CScriptedBrainTaskStore::GetTask", expect = "56 8B 74 24 08 33 C0",
       unverified = "ai8-4", route = "L2", hazards = "none")]
pub fn scripted_brain_task_store_get_task(ped: u32) -> u32 {
    for i in 0..0x30usize {
        if g::<u32>(0xC188F0 + i * 0x10).read() == ped {
            return g::<u32>(0xC188FC + i * 0x10).read();
        }
    }
    0
}

/// `CPedGroupMembership::Flush`-like walk at 0x5FB990 (catalogued as the class name): removes every member
/// (members at +4, 8 slots) that is set, by index.
#[hook(addr = 0x5FB990, conv = thiscall, name = "CPedGroupMembership", expect = "53 56 8B D9 57 33 F6",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn membership_remove_all_members(this: GPtr<u8>) {
    for i in 0..8 {
        if rd::<u32>(this, 4 + 4 * i as isize) != 0 {
            membership_remove_member(this, i);
        }
    }
}

/// `CPedGroupMembership::From(const CPedGroupMembership&)` (0x5F7FE0, `ret 4`): `AddMember(other.members[i], i)`
/// for the 8 slots, then copies the group pointer (+0) and +0x24. EAX = the last `AddMember` result.
#[hook(addr = 0x5F7FE0, conv = thiscall, name = "CPedGroupMembership::From", expect = "53 55 8B 6C 24 0C",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn membership_from(this: GPtr<u8>, other: GPtr<u8>) -> u32 {
    let mut r = 0;
    for i in 0..8 {
        r = membership_add_member(this, rd::<u32>(other, 4 + 4 * i as isize), i);
    }
    wr::<u32>(this, 0, rd::<u32>(other, 0));
    wr::<u32>(this, 0x24, rd::<u32>(other, 0x24));
    r
}

/// `CPedGroupMembership::RemoveNFollowers(int32 n)` (0x5FB1D0, `ret 4`): walks the 7 follower slots and removes
/// set members whose +0x484 byte is not 2 while `n > 0` (signed), counting `n` down.
#[hook(addr = 0x5FB1D0, conv = thiscall, name = "CPedGroupMembership::RemoveNFollowers", expect = "53 55 8B 6C 24 0C 56",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn membership_remove_n_followers(this: GPtr<u8>, n: i32) {
    let mut n = n;
    for i in 0..7 {
        let m = rd::<GPtr<u8>>(this, 4 + 4 * i as isize);
        if !m.is_null() && n > 0 && rd::<u8>(m, 0x484) != 2 {
            membership_remove_member(this, i);
            n -= 1;
        }
    }
}

/// `CPedGroupMembership::RemoveAllFollowers(bool keepMissionPeds)` (0x5FB190, `ret 4`): removes every set member
/// of slots 0..6 (not the leader slot 7), except, with the flag, those whose +0x484 byte is 2.
#[hook(addr = 0x5FB190, conv = thiscall, name = "CPedGroupMembership::RemoveAllFollowers", expect = "53 8A 5C 24 08 55",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn membership_remove_all_followers(this: GPtr<u8>, keep: u8) {
    for i in 0..8 {
        let m = rd::<GPtr<u8>>(this, 4 + 4 * i as isize);
        if m.is_null() || i == 7 || (keep != 0 && rd::<u8>(m, 0x484) == 2) {
            continue;
        }
        membership_remove_member(this, i);
    }
}

/// `CPedGroupMembership::AppointNewLeader` (0x5FB240): without a leader (+0x20) the first set follower slot
/// (0..6) is removed and added back as the leader (slot 7). EAX: the leader when there is one, 7 without
/// followers, else the `AddMember` result.
#[hook(addr = 0x5FB240, conv = thiscall, name = "CPedGroupMembership::AppointNewLeader", expect = "56 8B F1 8B 46 20",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn membership_appoint_new_leader(this: GPtr<u8>) -> u32 {
    let leader = rd::<u32>(this, 0x20);
    if leader != 0 {
        return leader;
    }
    for i in 0..7 {
        let m = rd::<u32>(this, 4 + 4 * i as isize);
        if m != 0 {
            membership_remove_member(this, i);
            return membership_add_member(this, m, 7);
        }
    }
    7
}

/// `CPedList::RemovePedsThatDontListenToPlayer` (0x69A420): for the list's count (+0) entries (+4), a ped with
/// flag 0x20 of +0x474 is cleared and the count decremented; then `FillUpHoles` (tail call, its EAX).
#[hook(addr = 0x69A420, conv = thiscall, name = "CPedList::RemovePedsThatDontListenToPlayer", expect = "8B 11 85 D2 7E 24",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn ped_list_remove_peds_that_dont_listen_to_player(this: GPtr<u8>) -> u32 {
    let n = rd::<i32>(this, 0);
    for i in 0..n.max(0) as isize {
        let ped = rd::<GPtr<u8>>(this, 4 + 4 * i);
        if rd::<u8>(ped, 0x474) & 0x20 != 0 {
            wr::<u32>(this, 4 + 4 * i, 0);
            wr::<i32>(this, 0, rd::<i32>(this, 0).wrapping_sub(1));
        }
    }
    ped_list_fill_up_holes(this)
}

/// `CPedList::RemovePedsAttackingPedType(int32 pedType)` (0x69A450, `ret 4`): for the initial count entries, a
/// ped without a task 1000 whose target (+0x10) has the ped type (+0x598) is cleared and the count decremented;
/// then `FillUpHoles`.
#[hook(addr = 0x69A450, conv = thiscall, name = "CPedList::RemovePedsAttackingPedType", expect = "57 8B F9 8B 07 85 C0",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn ped_list_remove_peds_attacking_ped_type(this: GPtr<u8>, ty: u32) -> u32 {
    let n = rd::<i32>(this, 0);
    for i in 0..n.max(0) as isize {
        let ped = rd::<GPtr<u8>>(this, 4 + 4 * i);
        let t = intel_find_task_by_type(rd::<GPtr<u8>>(ped, 0x47C), 1000);
        let keep = !t.is_null() && {
            let tgt = rd::<GPtr<u8>>(t, 0x10);
            !tgt.is_null() && rd::<u32>(tgt, 0x598) == ty
        };
        if !keep {
            wr::<u32>(this, 4 + 4 * i, 0);
            wr::<i32>(this, 0, rd::<i32>(this, 0).wrapping_sub(1));
        }
    }
    ped_list_fill_up_holes(this)
}

/// `CPedList::ExtractPedsWithGuns(CPedList& from)` (0x69A4C0, `ret 4`): walks `from` while the index is below its
/// current count (re-read after every removal, as the original does): a ped whose current weapon (slot +0x718,
/// weapons at +0x5A0, 0x1C each) is not melee is appended to this list when it has fewer than 30, and always
/// cleared from `from` with its count decremented; then `from.FillUpHoles()`.
#[hook(addr = 0x69A4C0, conv = thiscall, name = "CPedList::ExtractPedsWithGuns", expect = "53 55 57 8B 7C 24 10",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn ped_list_extract_peds_with_guns(this: GPtr<u8>, from: GPtr<u8>) -> u32 {
    let mut i = 0i32;
    while i < rd::<i32>(from, 0) {
        let slot = from.byte_add(4 + 4 * i as isize);
        let ped = rd::<GPtr<u8>>(slot, 0);
        let w = ped.byte_add(i32::from(rd::<i8>(ped, 0x718)) as isize * 0x1C + 0x5A0);
        if weapon_is_type_melee(w) == 0 {
            let n = rd::<i32>(this, 0);
            if n < 0x1E {
                wr::<u32>(this, 4 + 4 * n as isize, rd::<u32>(slot, 0));
                wr::<i32>(this, 0, n + 1);
            }
            wr::<u32>(slot, 0, 0);
            wr::<i32>(from, 0, rd::<i32>(from, 0).wrapping_sub(1));
        }
        i += 1;
    }
    ped_list_fill_up_holes(from)
}

/// `CPedList::BuildListFromGroup_NoLeader(CPedGroupMembership&)` (0x699DD0, `ret 4`): the list gets the set
/// members of slots 0..6 (`GetMember` called twice per member, as the original does) and the rest of its 30
/// entries is zeroed. EAX: 0 after the fill, the count when the list is full.
#[hook(addr = 0x699DD0, conv = thiscall, name = "CPedList::BuildListFromGroup_NoLeader", expect = "53 8B 5C 24 08 56",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn ped_list_build_list_from_group_no_leader(this: GPtr<u8>, m: GPtr<u8>) -> u32 {
    wr::<u32>(this, 0, 0);
    for i in 0..7 {
        if membership_get_member(m, i) != 0 {
            let p = membership_get_member(m, i);
            let n = rd::<i32>(this, 0);
            wr::<u32>(this, 4 + 4 * n as isize, p);
            wr::<i32>(this, 0, n.wrapping_add(1));
        }
    }
    let n = rd::<i32>(this, 0);
    if n >= 0x1E {
        return n as u32;
    }
    for k in n..0x1E {
        wr::<u32>(this, 4 + 4 * k as isize, 0);
    }
    0
}

/// `CFormation::ReturnTargetPedForPed(CPed*, int32* outTarget)` (0x699F50, cdecl): among the formation's peds
/// (count [0xC1A4D8], peds 0xC1A4DC) the first that is the ped with an assigned slot (0xC1A2C0, >= 0) writes the
/// slot's target (0xC1A45C) to `*out` and returns the slot; else returns the count (0 when it is not positive).
#[hook(addr = 0x699F50, conv = cdecl, name = "CFormation::ReturnTargetPedForPed", expect = "8B 0D D8 A4 C1 00",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn formation_return_target_ped_for_ped(ped: u32, out: GPtr<u8>) -> u32 {
    let n = g::<i32>(0xC1A4D8).read();
    if n <= 0 {
        return 0;
    }
    let mut i = 0i32;
    while i < n {
        let k = i as usize * 4;
        if g::<u32>(0xC1A4DC + k).read() == ped && g::<i32>(0xC1A2C0 + k).read() >= 0 {
            let s = g::<u32>(0xC1A2C0 + k).read();
            wr::<u32>(out, 0, g::<u32>((0xC1A45C_u32.wrapping_add(s.wrapping_mul(4))) as usize).read());
            return s;
        }
        i += 1;
    }
    i as u32
}

/// `CFormation::ReturnDestinationForPed(CPed*, CVector* out)` (0x699FA0, cdecl; AL): among the 24 formation
/// entries the first that is the ped with an assigned slot (>= 0) copies the slot's destination (0xC1A31C, 12
/// bytes each) to `*out` and returns AL = 1 (EAX's upper bytes are those of the copied y); else AL = 0 (EAX = 0).
#[hook(addr = 0x699FA0, conv = cdecl, name = "CFormation::ReturnDestinationForPed", expect = "8B 4C 24 04 33 C0",
       unverified = "ai8-4", route = "L2", hazards = "ax-al-return,heap-writes")]
pub fn formation_return_destination_for_ped(ped: u32, out: GPtr<u8>) -> u8 {
    for i in 0..0x18usize {
        if g::<u32>(0xC1A4DC + i * 4).read() == ped && g::<i32>(0xC1A2C0 + i * 4).read() >= 0 {
            let s = g::<u32>(0xC1A2C0 + i * 4).read();
            let v = GPtr::<u8>::from_addr(s.wrapping_mul(12).wrapping_add(0xC1A31C) as usize);
            for o in [0, 4, 8] {
                wr::<u32>(out, o, rd::<u32>(v, o));
            }
            return 1;
        }
    }
    0
}

/// `CCoverPoint::ReserveCoverPointForPed(CPed*)` (0x698EB0, `ret 4`): over the two user slots (+0x14): a slot
/// already holding the ped returns its index; the first empty slot takes the ped and registers the reference
/// (tail call, its EAX); with both taken by others EAX = 2.
#[hook(addr = 0x698EB0, conv = thiscall, name = "CCoverPoint::ReserveCoverPointForPed", expect = "56 57 8B F9 8B 4C 24 0C",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn cover_point_reserve_cover_point_for_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    for i in 0..2u32 {
        let slot = this.byte_add(0x14 + 4 * i as isize);
        let v = rd::<GPtr<u8>>(slot, 0);
        if v == ped {
            return i;
        }
        if v.is_null() {
            wr::<GPtr<u8>>(slot, 0, ped);
            return entity_register_reference(ped, slot);
        }
    }
    2
}

/// `CCoverPoint::ReleaseCoverPointForPed(CPed*)` (0x698EF0, `ret 4`; the loop counter is set in a fragment at
/// 0x403A1B): each of the two user slots (+0x14) holding the ped drops its reference (when non-null) and is
/// cleared.
#[hook(addr = 0x698EF0, conv = thiscall, name = "CCoverPoint::ReleaseCoverPointForPed", expect = "53 8B 5C 24 08 56",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn cover_point_release_cover_point_for_ped(this: GPtr<u8>, ped: GPtr<u8>) {
    for i in 0..2 {
        let slot = this.byte_add(0x14 + 4 * i);
        let v = rd::<GPtr<u8>>(slot, 0);
        if v == ped {
            if !v.is_null() {
                entity_clean_up_old_reference(v, slot);
            }
            wr::<u32>(slot, 0, 0);
        }
    }
}

/// `FUN_00667900` (0x667900; called by 0x66EFA0): reverses the dword entries (+4) of a count-prefixed (+0) node
/// list in place, swapping from both ends while the front index is below the back one (signed).
#[hook(addr = 0x667900, conv = thiscall, name = "FUN_00667900", expect = "8B 01 56 33 F6 48",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn node_list_reverse(this: GPtr<u8>) {
    let mut back = rd::<i32>(this, 0).wrapping_sub(1);
    let mut front = 0i32;
    if back <= 0 {
        return;
    }
    loop {
        let (pf, pb) = (this.byte_add(4 + 4 * front as isize), this.byte_add(4 + 4 * back as isize));
        let (a, b) = (rd::<u32>(pf, 0), rd::<u32>(pb, 0));
        wr::<u32>(pf, 0, b);
        wr::<u32>(pb, 0, a);
        front += 1;
        back -= 1;
        if front >= back {
            break;
        }
    }
}

/// `CTaskComplexUseScriptedBrain::CTaskComplexUseScriptedBrain(const char* name)` (0x6358B0, `ret 4`): the
/// `CTaskComplex` base, vtable 0x86E568, +0xC = 0, +0x18 = 0, and the name copied byte by byte (with its NUL) to
/// +0xD. Returns `this`.
#[hook(addr = 0x6358B0, conv = thiscall, name = "CTaskComplexUseScriptedBrain::Constructor", expect = "56 8B F1 E8 F8 4A FE FF",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn use_scripted_brain_ctor(this: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0, 0x86E568);
    wr::<u8>(this, 0xC, 0);
    wr::<u32>(this, 0x18, 0);
    let mut i = 0isize;
    loop {
        let c = rd::<u8>(name, i);
        wr::<u8>(this, 0xD + i, c);
        i += 1;
        if c == 0 {
            break;
        }
    }
    this
}

/// `CTaskComplexLeaveCarAsPassengerWait::GetNextPassengerToLeave` (0x63B500): the first set passenger slot
/// (+0x464) below the vehicle's (+0xC) passenger count byte (+0x488), else null.
#[hook(addr = 0x63B500, conv = thiscall, name = "CTaskComplexLeaveCarAsPassengerWait::GetNextPassengerToLeave", expect = "56 57 8B 79 0C",
       unverified = "ai8-4", route = "L2", hazards = "none")]
pub fn leave_car_as_passenger_wait_get_next_passenger(this: GPtr<u8>) -> u32 {
    let v = rd::<GPtr<u8>>(this, 0xC);
    let n = i32::from(rd::<u8>(v, 0x488));
    for i in 0..n.max(0) as isize {
        let p = rd::<u32>(v, 0x464 + 4 * i);
        if p != 0 {
            return p;
        }
    }
    0
}

/// `CEventPlayerCommandToGroup::AffectsPedGroup(CPedGroup*)` (0x4B24D0, `ret 4`; AL): the group's leader (+8
/// membership) must be a player, and every set member of slots 0..6 must share the leader's +0x2F byte.
#[hook(addr = 0x4B24D0, conv = thiscall, name = "CEventPlayerCommandToGroup::AffectsPedGroup", expect = "53 56 57 8B 7C 24 10",
       unverified = "ai8-4", route = "L2", hazards = "ax-al-return")]
pub fn player_command_to_group_affects_ped_group(_this: GPtr<u8>, group: GPtr<u8>) -> u8 {
    let m = group.byte_add(8);
    let leader = membership_get_leader(m);
    if leader.is_null() || ped_is_player(leader) == 0 {
        return 0;
    }
    for i in 0..7 {
        let p = membership_get_member(m, i);
        if p != 0 && rd::<u8>(GPtr::from_addr(p as usize), 0x2F) != rd::<u8>(leader, 0x2F) {
            return 0;
        }
    }
    1
}

/// `CPedList::BuildListFromGroup_NotInCar_NoLeader(CPedGroupMembership&)` (0x69A340, `ret 4`): like
/// `BuildListFromGroup_NoLeader`, skipping members in or entering a car, appending only below 30 entries.
#[hook(addr = 0x69A340, conv = thiscall, name = "CPedList::BuildListFromGroup_NotInCar_NoLeader", expect = "53 8B 5C 24 08 56",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn ped_list_build_list_from_group_not_in_car_no_leader(this: GPtr<u8>, m: GPtr<u8>) -> u32 {
    wr::<u32>(this, 0, 0);
    for i in 0..7 {
        let p = membership_get_member(m, i);
        if p != 0 && intel_is_in_a_car_or_entering_one(rd::<GPtr<u8>>(GPtr::from_addr(p as usize), 0x47C)) == 0 {
            let p = membership_get_member(m, i);
            let n = rd::<i32>(this, 0);
            if n < 0x1E {
                wr::<u32>(this, 4 + 4 * n as isize, p);
                wr::<i32>(this, 0, n + 1);
            }
        }
    }
    let n = rd::<i32>(this, 0);
    if n >= 0x1E {
        return n as u32;
    }
    for k in n..0x1E {
        wr::<u32>(this, 4 + 4 * k as isize, 0);
    }
    0
}

/// `CTaskComplexUseScriptedBrain::ControlSubTask(CPed*)` (0x635B60, `ret 4`): looks the ped's brain task up in
/// `CScriptedBrainTaskStore` (inline, as `GetTask`); when it differs from the current one (+0x18) and the
/// sub-task (+8) agrees to an urgent abort, it becomes current and the result is its `Clone()` (null without
/// one). Otherwise the sub-task.
#[hook(addr = 0x635B60, conv = thiscall, name = "CTaskComplexUseScriptedBrain::ControlSubTask", expect = "8B 54 24 04 56 57",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn use_scripted_brain_control_sub_task(this: GPtr<u8>, ped: u32) -> u32 {
    let mut t = 0u32;
    for i in 0..0x30usize {
        if g::<u32>(0xC188F0 + i * 0x10).read() == ped {
            t = g::<u32>(0xC188FC + i * 0x10).read();
            break;
        }
    }
    if t != rd::<u32>(this, 0x18) && v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, 0) != 0 {
        wr::<u32>(this, 0x18, t);
        if t == 0 {
            return 0;
        }
        return v_clone(GPtr::from_addr(t as usize));
    }
    rd::<u32>(this, 8)
}

/// `CTaskAllocatorPool::New` (0x69D8E0): the pool's next free slot from the cursor (+0xC) on (flag bytes +4, bit
/// 7 = free; size +8), wrapping once to 0 (null when the second wrap comes); the slot's flag loses bit 7 and its
/// 7-bit reference counter is incremented; the result is objects (+0) + 0x20 * slot.
#[hook(addr = 0x69D8E0, conv = thiscall, name = "CTaskAllocatorPool::New", expect = "8B 51 08 56 32 C0",
       unverified = "ai8-4", route = "L2", hazards = "heap-writes")]
pub fn task_allocator_pool_new(this: GPtr<u8>) -> u32 {
    let size = rd::<u32>(this, 8);
    let mut wrapped = false;
    loop {
        let i = rd::<u32>(this, 0xC).wrapping_add(1);
        wr::<u32>(this, 0xC, i);
        if i == size {
            wr::<u32>(this, 0xC, 0);
            if wrapped {
                return 0;
            }
            wrapped = true;
        }
        let j = rd::<u32>(this, 0xC);
        if rd::<u8>(rd::<GPtr<u8>>(this, 4), j as isize) & 0x80 != 0 {
            break;
        }
    }
    let f = rd::<GPtr<u8>>(this, 4).byte_add(rd::<u32>(this, 0xC) as isize);
    wr::<u8>(f, 0, rd::<u8>(f, 0) & 0x7F);
    let f = rd::<GPtr<u8>>(this, 4).byte_add(rd::<u32>(this, 0xC) as isize);
    let b = rd::<u8>(f, 0);
    wr::<u8>(f, 0, (b & 0x80) | (b.wrapping_add(1) & 0x7F));
    (rd::<u32>(this, 0xC) << 5).wrapping_add(rd::<u32>(this, 0))
}

pub const HOOKS: &[HookReg] = &[
    HOOK_USE_SCRIPTED_BRAIN_CONTROL_SUB_TASK,
    HOOK_TASK_ALLOCATOR_POOL_NEW,
    HOOK_USE_SCRIPTED_BRAIN_CTOR,
    HOOK_LEAVE_CAR_AS_PASSENGER_WAIT_GET_NEXT_PASSENGER,
    HOOK_PLAYER_COMMAND_TO_GROUP_AFFECTS_PED_GROUP,
    HOOK_PED_LIST_BUILD_LIST_FROM_GROUP_NOT_IN_CAR_NO_LEADER,
    HOOK_SCRIPTED_TASK_RECORD_GET_STATUS,
    HOOK_SCRIPTED_BRAIN_TASK_STORE_GET_TASK,
    HOOK_MEMBERSHIP_REMOVE_ALL_MEMBERS,
    HOOK_MEMBERSHIP_FROM,
    HOOK_MEMBERSHIP_REMOVE_N_FOLLOWERS,
    HOOK_MEMBERSHIP_REMOVE_ALL_FOLLOWERS,
    HOOK_MEMBERSHIP_APPOINT_NEW_LEADER,
    HOOK_PED_LIST_REMOVE_PEDS_THAT_DONT_LISTEN_TO_PLAYER,
    HOOK_PED_LIST_REMOVE_PEDS_ATTACKING_PED_TYPE,
    HOOK_PED_LIST_EXTRACT_PEDS_WITH_GUNS,
    HOOK_PED_LIST_BUILD_LIST_FROM_GROUP_NO_LEADER,
    HOOK_FORMATION_RETURN_TARGET_PED_FOR_PED,
    HOOK_FORMATION_RETURN_DESTINATION_FOR_PED,
    HOOK_COVER_POINT_RESERVE_COVER_POINT_FOR_PED,
    HOOK_COVER_POINT_RELEASE_COVER_POINT_FOR_PED,
    HOOK_NODE_LIST_REVERSE,
];
