//! Further AI loop bodies: group-command event checks, the interesting-events invalidation, the group's
//! furthest-member distance and two shutdown walks (PORT-AI-8). Ported from the 1.0 US bytes. Every call to
//! another catalogued function goes through `orig!` at the original address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CTaskComplexSequence::Flush` (0x632C10)
    fn sequence_flush(s: GPtr<u8>) = thiscall 0x632C10 => unported;
    /// `CEventPlayerCommandToGroup::AffectsPedGroup(CPedGroup*)` (0x4B24D0, `ret 4`; AL)
    fn player_command_to_group_affects(e: GPtr<u8>, g: GPtr<u8>) -> u8 = thiscall 0x4B24D0 => unported;
    /// `CPedGroupMembership::GetMember(int32)` (0x5F69B0, `ret 4`)
    fn membership_get_member(m: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x5F69B0 => unported;
    /// `CPedGroupMembership::GetLeader` (0x5F69A0)
    fn membership_get_leader(m: GPtr<u8>) -> GPtr<u8> = thiscall 0x5F69A0 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0, `ret 4`)
    fn intel_find_task_by_type(i: GPtr<u8>, ty: i32) -> u32 = thiscall 0x600EE0 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool buildings, bool vehicles, bool peds,
    /// bool objects, bool dummies, bool seeThrough, bool ignoreSomeObjects)` (0x56A490, cdecl)
    #[allow(clippy::too_many_arguments)]
    fn world_los_clear(a: GPtr<u8>, b: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, ig: u32) -> u8 = cdecl 0x56A490 => unported;
}

/// Drops the entity reference held in the slot (when set) and clears the slot.
fn drop_ref(slot: GPtr<u8>) {
    let e = rd::<GPtr<u8>>(slot, 0);
    if !e.is_null() {
        entity_clean_up_old_reference(e, slot);
        wr::<u32>(slot, 0, 0);
    }
}

/// The interesting-events destructor (`CInterestingEvents::Destructor`, 0x856880, cdecl; an `atexit` handler):
/// drops the entity references of the 8 events of the global instance (entity slots 0xC0B060, 12 bytes apart).
#[hook(addr = 0x856880, conv = cdecl, name = "CInterestingEvents::Destructor", expect = "56 57 BE 60 B0 C0 00",
       unverified = "ai8-6", route = "split", hazards = "heap-writes")]
pub fn interesting_events_dtor() {
    for i in 0..8usize {
        drop_ref(g(0xC0B060 + 12 * i));
    }
}

/// `CTaskSequences::CleanUpForShutdown` (0x632DD0, cdecl): for the 64 sequences (0xC178F0, 0x40 bytes each)
/// clears the in-use byte (0xC17898 + i) and flushes the sequence.
#[hook(addr = 0x632DD0, conv = cdecl, name = "CTaskSequences::CleanUpForShutdown", expect = "56 57 33 FF BE F0 78 C1 00",
       unverified = "ai8-6", route = "split", hazards = "heap-writes")]
pub fn task_sequences_clean_up_for_shutdown() {
    for i in 0..0x40usize {
        g::<u8>(0xC17898 + i).write(0);
        sequence_flush(g(0xC178F0 + 0x40 * i));
    }
}

/// `CEventPlayerCommandToGroupAttack::AffectsPedGroup(CPedGroup*)` (0x4B2530, `ret 4`; AL): when the base check
/// (`CEventPlayerCommandToGroup::AffectsPedGroup`) passes, true as soon as a set member of slots 0..6 lacks flag
/// 0x20 of +0x474; false otherwise (the base's AL when it fails).
#[hook(addr = 0x4B2530, conv = thiscall, name = "CEventPlayerCommandToGroupAttack::AffectsPedGroup", expect = "56 8B 74 24 08 56",
       unverified = "ai8-6", route = "L2", hazards = "ax-al-return")]
pub fn player_command_to_group_attack_affects_ped_group(this: GPtr<u8>, group: GPtr<u8>) -> u8 {
    let r = player_command_to_group_affects(this, group);
    if r == 0 {
        return r;
    }
    for i in 0..7 {
        let m = membership_get_member(group.byte_add(8), i);
        if !m.is_null() && rd::<u8>(m, 0x474) & 0x20 == 0 {
            return 1;
        }
    }
    0
}

/// `CEventLeaderExitedCarAsDriver::AffectsPedGroup(CPedGroup*)` (0x4B0F80, `ret 4`; AL): true as soon as a set
/// member of slots 0..6 sits in the leader's vehicle (+0x58C, in-vehicle flag bit 8 of +0x46C), or has task 0x2BC
/// or 0x2CE; false otherwise.
#[hook(addr = 0x4B0F80, conv = thiscall, name = "CEventLeaderExitedCarAsDriver::AffectsPedGroup", expect = "53 8B 5C 24 08 56",
       unverified = "ai8-6", route = "L2", hazards = "ax-al-return")]
pub fn leader_exited_car_as_driver_affects_ped_group(_this: GPtr<u8>, group: GPtr<u8>) -> u8 {
    let m = group.byte_add(8);
    for i in 0..7 {
        let p = membership_get_member(m, i);
        if p.is_null() {
            continue;
        }
        if rd::<u32>(p, 0x58C) != 0 && rd::<u32>(p, 0x46C) & 0x100 != 0 {
            let l = membership_get_leader(m);
            if rd::<u32>(p, 0x58C) == rd::<u32>(l, 0x58C) {
                return 1;
            }
        }
        if intel_find_task_by_type(rd::<GPtr<u8>>(p, 0x47C), 0x2BC) != 0
            || intel_find_task_by_type(rd::<GPtr<u8>>(p, 0x47C), 0x2CE) != 0
        {
            return 1;
        }
    }
    0
}

/// `CInterestingEvents::InvalidateEvent(const CInterestingEvent*)` (0x602960, `ret 4`): the matching one of the 8
/// events (12 bytes each) gets time +4 = 0 and drops its entity (+8); a current event (+0x154) at that index is
/// reset to -1.
#[hook(addr = 0x602960, conv = thiscall, name = "CInterestingEvents::InvalidateEvent", expect = "53 8B D9 8B 4C 24 08",
       unverified = "ai8-6", route = "L2", hazards = "heap-writes")]
pub fn interesting_events_invalidate_event(this: GPtr<u8>, ev: GPtr<u8>) {
    for i in 0..8 {
        let e = this.byte_add(12 * i);
        if e == ev {
            wr::<u32>(e, 4, 0);
            drop_ref(e.byte_add(8));
            if i32::from(rd::<i8>(this, 0x154)) == i as i32 {
                wr::<u8>(this, 0x154, 0xFF);
            }
            return;
        }
    }
}

/// `CInterestingEvents::InvalidateNonVisibleEvents` (0x6029C0): each event whose entity (+8) is not in the line of
/// sight (buildings, see-through) of the active camera's source (0xB6F338 + 0x238 * [0xB6F081], a stack copy) is
/// invalidated as by `InvalidateEvent` (the entity slot is zeroed even when the reference was empty).
#[hook(addr = 0x6029C0, conv = thiscall, name = "CInterestingEvents::InvalidateNonVisibleEvents", expect = "83 EC 0C 0F B6 05 81 F0 B6 00",
       unverified = "ai8-6", route = "L2", hazards = "heap-writes")]
pub fn interesting_events_invalidate_non_visible_events(this: GPtr<u8>) {
    let c = GPtr::<u8>::from_addr(usize::from(g::<u8>(0xB6F081).read()) * 0x238 + 0xB6F338);
    let mut cam = [rd::<u32>(c, 0), rd::<u32>(c, 4), rd::<u32>(c, 8)];
    for i in 0..8 {
        let slot = this.byte_add(8 + 12 * i);
        let e = rd::<GPtr<u8>>(slot, 0);
        if e.is_null() {
            continue;
        }
        let m = rd::<GPtr<u8>>(e, 0x14);
        let p = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
        if world_los_clear(GPtr::from_raw(cam.as_mut_ptr()).cast(), p, 1, 0, 0, 0, 0, 1, 0) != 0 {
            continue;
        }
        wr::<u32>(slot, -4, 0);
        let e = rd::<GPtr<u8>>(slot, 0);
        if !e.is_null() {
            entity_clean_up_old_reference(e, slot);
        }
        wr::<u32>(slot, 0, 0);
        if i32::from(rd::<i8>(this, 0x154)) == i as i32 {
            wr::<u8>(this, 0x154, 0xFF);
        }
    }
}

/// `CPedGroup::FindDistanceToFurthestMember` (0x5FB010; ST0): with a leader (+0x28) the largest distance from it
/// to the 7 followers (+0xC; through an f32 slot; a NaN or equal distance also replaces the best), from 0.0;
/// without a leader 0.0.
#[hook(addr = 0x5FB010, conv = thiscall, name = "CPedGroup::FindDistanceToFurthestMember", expect = "51 D9 05 50 8B 85 00",
       unverified = "ai8-6", route = "L2", hazards = "x87-transcendental")]
pub fn ped_group_find_distance_to_furthest_member(this: GPtr<u8>) -> X87 {
    let mut best = x(g::<f32>(0x858B50).read());
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
        if best.fcom(x(d)).test_any(0x41) {
            best = x(d);
        }
    }
    best
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INTERESTING_EVENTS_DTOR,
    HOOK_TASK_SEQUENCES_CLEAN_UP_FOR_SHUTDOWN,
    HOOK_PLAYER_COMMAND_TO_GROUP_ATTACK_AFFECTS_PED_GROUP,
    HOOK_LEADER_EXITED_CAR_AS_DRIVER_AFFECTS_PED_GROUP,
    HOOK_INTERESTING_EVENTS_INVALIDATE_EVENT,
    HOOK_INTERESTING_EVENTS_INVALIDATE_NON_VISIBLE_EVENTS,
    HOOK_PED_GROUP_FIND_DISTANCE_TO_FURTHEST_MEMBER,
];
