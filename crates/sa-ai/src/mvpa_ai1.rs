//! MVP-A AI ports, batch `mvpa-ai-1`: boot/loading-phase AI functions (decision-maker setters, `CDecision::From`,
//! `CConversations::Clear`, `CCover::Init`, `CTaskSequences::Init`, ped group flushes, the inform queues' `Init`/
//! `Flush`, the task pool's `operator new`/`delete`/`New`, trivial event/task destructors and allocator
//! constructors, and the small `CPedGeometryAnalyser` helpers). Ported from the 1.0 US bytes (the exe wins over
//! gta-reversed). Every call to another catalogued function goes through `orig!` at the original address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPtrListDoubleLink::Flush` (0x552470)
    fn ptr_list_double_link_flush(l: GPtr<u8>) = thiscall 0x552470 => unported;
    /// `CTaskComplexSequence::Flush` (0x632C10)
    fn sequence_flush(s: GPtr<u8>) = thiscall 0x632C10 => crate::small::sequence_flush;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0, AL)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CRadar::ClearBlipForEntity(eBlipType, int32 handle)` (0x587C60, cdecl)
    fn radar_clear_blip_for_entity(ty: u32, handle: i32) = cdecl 0x587C60 => unported;
    /// `CPedIntelligence::RestorePedDecisionMakerType` (0x600BC0)
    fn intel_restore_ped_decision_maker_type(i: GPtr<u8>) -> u32 = thiscall 0x600BC0 => crate::intelrest::intel_restore_ped_decision_maker_type;
    /// `CPedTaskPair::Flush` (0x5E95B0)
    fn task_pair_flush(p: GPtr<u8>) = thiscall 0x5E95B0 => crate::mvpa_ai1::ped_task_pair_flush;
    /// `CTaskPool::New` (0x61A500, thiscall on the pool)
    fn task_pool_new_o(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A500 => crate::mvpa_ai1::task_pool_new;
    /// `CEventAcquaintancePed::~CEventAcquaintancePed` (0x4AF890)
    fn event_acquaintance_ped_dtor(e: GPtr<u8>) = thiscall 0x4AF890 => unported;
    /// `CTaskSimple::~CTaskSimple` (0x61A3A0)
    fn task_simple_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `CTaskComplexCarDrive::~CTaskComplexCarDrive` (0x63CA40)
    fn task_complex_car_drive_dtor(t: GPtr<u8>) = thiscall 0x63CA40 => unported;
    /// `CTaskSimpleIKChain::~CTaskSimpleIKChain` (0x633A90)
    fn task_simple_ik_chain_dtor(t: GPtr<u8>) = thiscall 0x633A90 => unported;
    /// scalar deleting destructor (vtable +0)
    fn v_delete(t: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn task_manager_get_task_secondary(m: GPtr<u8>, slot: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CPedGeometryAnalyser::CanPedTargetPoint(const CPed&, const CVector&, bool)` (0x5F1B70, cdecl; AL, full EAX
    /// passed on)
    fn can_ped_target_point_o(ped: GPtr<u8>, point: GPtr<u8>, check: u32) -> u32 = cdecl 0x5F1B70 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490, cdecl; AL, full EAX)
    #[allow(clippy::too_many_arguments)]
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bld: u32, veh: u32, peds: u32, obj: u32, dum: u32, see: u32, cam: u32) -> u32 = cdecl 0x56A490 => unported;
    /// `CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox(const CPed&, const CVector&, CEntity&,
    /// const CVector&, CPointRoute&, int32)` (0x5F3DD0, cdecl)
    fn route_round_entity_bbox_2(ped: GPtr<u8>, pos: GPtr<u8>, e: GPtr<u8>, target: u32, route: u32, flag: u32) -> u32 = cdecl 0x5F3DD0 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityHitSide(const CVector&, CEntity&)` (0x5F3AC0, cdecl)
    fn entity_hit_side_3(pos: GPtr<u8>, e: GPtr<u8>) -> u32 = cdecl 0x5F3AC0 => crate::mvpa_ai1::compute_entity_hit_side_3;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCornersUncached(float z, CEntity&, CVector* corners)` (0x5F1FA0,
    /// cdecl; `z` passed as its dword)
    fn bbox_corners_uncached(z: u32, e: GPtr<u8>, corners: GPtr<u8>) = cdecl 0x5F1FA0 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCentreUncached(float z, const CVector* corners, CVector& out)`
    /// (0x5F1600, cdecl)
    fn bbox_centre_uncached(z: u32, corners: GPtr<u8>, out: GPtr<u8>) = cdecl 0x5F1600 => crate::mvpa_ai1::compute_entity_bounding_box_centre_uncached;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxSegmentPlanesUncached(const CVector* corners, const CVector&
    /// centre, CVector* normals, float* distances)` (0x5F1750, cdecl)
    fn bbox_segment_planes_uncached(corners: GPtr<u8>, centre: GPtr<u8>, normals: GPtr<u8>, d: GPtr<u8>) = cdecl 0x5F1750 => crate::mvpa_ai1::compute_entity_bounding_box_segment_planes_uncached;
    /// `CPedGeometryAnalyser::ComputeEntityHitSide(const CVector&, const CVector* normals, const float* distances)`
    /// (0x5F1450, cdecl)
    fn entity_hit_side_2(pos: GPtr<u8>, normals: GPtr<u8>, d: GPtr<u8>) -> u32 = cdecl 0x5F1450 => unported;
    /// `CVector::Normalise` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
}

/// `CPools::ms_pPedPool`
const PED_POOL: usize = 0xB74490;

fn cf(addr: usize) -> X87 {
    x(g::<f32>(addr).read())
}

fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// The entity's position: `m_matrix->pos` (+0x14 -> +0x30) when it has a matrix, else the placement (+4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `CPedIntelligence::SetPedDecisionMakerType(int32)` (0x600B50, `ret 4`): with no current type (+0xB4 == 0) the
/// argument goes to the saved type (+0xB8); else a non-zero argument replaces +0xB4 and 0 moves +0xB4 to +0xB8 and
/// clears +0xB4. When +0xB4 is then 7: +0xC8 = 5.0f, +0xC4 = 15.
#[hook(addr = 0x600B50, conv = thiscall, name = "CPedIntelligence::SetPedDecisionMakerType", expect = "8B 81 B4 00 00 00 85 C0",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn intel_set_ped_decision_maker_type(this: GPtr<u8>, ty: i32) {
    let cur = rd::<i32>(this, 0xB4);
    if cur == 0 {
        wr::<i32>(this, 0xB8, ty);
    } else if ty == 0 {
        wr::<i32>(this, 0xB8, cur);
        wr::<i32>(this, 0xB4, 0);
    } else {
        wr::<i32>(this, 0xB4, ty);
    }
    if rd::<i32>(this, 0xB4) == 7 {
        wr::<u32>(this, 0xC8, 0x40A0_0000);
        wr::<u32>(this, 0xC4, 0xF);
    }
}

/// `CPedGroupIntelligence::SetGroupDecisionMakerType(int32)` (0x5F7340, `ret 4`): +0x298 = the argument (the store
/// sits in a protection chunk at 0x4037E0 that jumps back to the `ret 4`).
#[hook(addr = 0x5F7340, conv = thiscall, name = "CPedGroupIntelligence::SetGroupDecisionMakerType", expect = "8B 44 24 04 90 E9 96 C4 E0 FF",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn pgi_set_group_decision_maker_type(this: GPtr<u8>, ty: i32) {
    wr::<i32>(this, 0x298, ty);
}

/// `CDecision::From(const CDecision&)` (0x6006B0, `ret 4`; the loop body is the chunk 0x6006D0, entered through the
/// protection chunk 0x40394C that sets the count 6): copies the 0x3C-byte decision in six steps, each copying
/// dword `+4i` (task types), the four bytes `+0x18+4i` (probabilities) and the two bytes `+0x30+2i` (facial
/// flags), in that order.
#[hook(addr = 0x6006B0, conv = thiscall, name = "CDecision::From", expect = "53 55 56 57 8B 7C 24 14",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn decision_from(this: GPtr<u8>, src: GPtr<u8>) {
    for i in 0..6isize {
        wr::<u32>(this, 4 * i, rd::<u32>(src, 4 * i));
        for k in 0..4 {
            wr::<u8>(this, 0x18 + 4 * i + k, rd::<u8>(src, 0x18 + 4 * i + k));
        }
        wr::<u8>(this, 0x30 + 2 * i, rd::<u8>(src, 0x30 + 2 * i));
        wr::<u8>(this, 0x31 + 2 * i, rd::<u8>(src, 0x31 + 2 * i));
    }
}

/// `CConversations::Clear` (0x43A7B0, a `.HOODLUM` stub; body 0x156DFF0): the 14 conversations (0x9691D8, 0x1C
/// each) get +0/+4 = -1 and +8/+0x10/+0xC = 0; the 50 nodes (0x969570, 0x18 each) get name byte 0 = 0, the two i16
/// links (+0xA, +8) = -1 and +0x10/+0x14/+0xC = 0; then `m_bSettingUpConversation` (0x9691D0) = 0 and
/// `m_SettingUpConversationNumNodes` (0x9691C4) = 0.
#[hook(addr = 0x43A7B0, conv = cdecl, name = "CConversations::Clear", expect = "E9 3B 38 13 01",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn conversations_clear() {
    for i in 0..14usize {
        let c = g::<u8>(0x9691D8 + 0x1C * i);
        wr::<u32>(c, 0, 0xFFFF_FFFF);
        wr::<u32>(c, 4, 0xFFFF_FFFF);
        wr::<u32>(c, 8, 0);
        wr::<u32>(c, 0x10, 0);
        wr::<u32>(c, 0xC, 0);
    }
    for i in 0..50usize {
        let n = g::<u8>(0x969570 + 0x18 * i);
        wr::<u8>(n, 0, 0);
        wr::<u16>(n, 0xA, 0xFFFF);
        wr::<u16>(n, 8, 0xFFFF);
        wr::<u32>(n, 0x10, 0);
        wr::<u32>(n, 0x14, 0);
        wr::<u32>(n, 0xC, 0);
    }
    g::<u8>(0x9691D0).write(0);
    g::<u32>(0x9691C4).write(0);
}

/// `CCover::Init` (0x698710): `m_NumPoints` (0xC197A4) = 0, `m_ListOfProcessedBuildings` (0xC1A2B8) flushed, then
/// the 100 cover points (0xC197C8, 0x1C each) get type byte +0 = 0.
#[hook(addr = 0x698710, conv = cdecl, name = "CCover::Init", expect = "B9 B8 A2 C1 00",
       unverified = "mvpa-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn cover_init() {
    g::<u32>(0xC197A4).write(0);
    ptr_list_double_link_flush(g::<u8>(0xC1A2B8));
    for i in 0..100usize {
        g::<u8>(0xC197C8 + 0x1C * i).write(0);
    }
}

/// `CTaskSequences::Init` (0x632D90): `ms_iActiveSequence` (0x8D2E98) = -1, then for each of the 64 sequences
/// `ms_bIsOpened[i]` (0xC17898) = 0 and `ms_taskSequence[i]` (0xC178F0, 0x40 each) flushed.
#[hook(addr = 0x632D90, conv = cdecl, name = "CTaskSequences::Init", expect = "56 57 C7 05 98 2E 8D 00 FF FF FF FF",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn task_sequences_init() {
    g::<i32>(0x8D2E98).write(-1);
    for i in 0..64usize {
        g::<u8>(0xC17898 + i).write(0);
        sequence_flush(g::<u8>(0xC178F0 + 0x40 * i));
    }
}

/// `CPedGroupMembership::RemoveMember(int32 memberIndex)` (0x5F80D0, `ret 4`): releases the reference of slot
/// `+4 + 4*i` (when set), then, unless that ped is a player (`IsPlayer` is called even on a null slot): a ped with
/// flag 0x2000 at +0x474 loses its blip (`ClearBlipForEntity(2, pool handle)`) and the flag; its intelligence
/// (+0x47C) restores its decision maker type; when the leader slot (+0x20) is set and the leader has player data
/// (+0x480), ped flag 0x100000 at +0x470 is set. The slot becomes 0 in every case.
#[hook(addr = 0x5F80D0, conv = thiscall, name = "CPedGroupMembership::RemoveMember-ByMemIdx", expect = "8B 44 24 04 56 57 8B F9",
       unverified = "mvpa-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn membership_remove_member(this: GPtr<u8>, idx: i32) {
    let slot = this.byte_add(4 + 4 * idx as isize);
    let p = rd::<GPtr<u8>>(slot, 0);
    if !p.is_null() {
        entity_clean_up_old_reference(p, slot);
    }
    if ped_is_player(rd::<GPtr<u8>>(slot, 0)) == 0 {
        let ped = rd::<GPtr<u8>>(slot, 0);
        if rd::<u32>(ped, 0x474) & 0x2000 != 0 {
            let pool = g::<GPtr<u8>>(PED_POOL).read();
            let i = (ped.addr() as i32).wrapping_sub(rd::<i32>(pool, 0)) / 0x7C4;
            let flags = rd::<GPtr<u8>>(pool, 4);
            let handle = (i << 8).wrapping_add(rd::<u8>(flags, i as isize) as i32);
            radar_clear_blip_for_entity(2, handle);
            let ped = rd::<GPtr<u8>>(slot, 0);
            wr::<u32>(ped, 0x474, rd::<u32>(ped, 0x474) & 0xFFFF_DFFF);
        }
        intel_restore_ped_decision_maker_type(rd::<GPtr<u8>>(rd::<GPtr<u8>>(slot, 0), 0x47C));
        let leader = rd::<GPtr<u8>>(this, 0x20);
        if !leader.is_null() && rd::<u32>(leader, 0x480) != 0 {
            let ped = rd::<GPtr<u8>>(slot, 0);
            wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) | 0x10_0000);
        }
    }
    wr::<u32>(slot, 0, 0);
}

/// `CPedGroupIntelligence::Flush` (0x5F7350): flushes the four task-pair arrays (8 x 0x14 at +0xC, +0xAC, +0x1EC,
/// +0x14C, interleaved in that order per index), deletes the old (+4) and new (+8) events (once when they are the
/// same object) and the default task allocator's tasks (+0x290, +0x294), clearing each; +0x298 and +0x29C = -1.
#[hook(addr = 0x5F7350, conv = thiscall, name = "CPedGroupIntelligence::Flush", expect = "53 56 57 8B F9",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn pgi_flush(this: GPtr<u8>) {
    for i in 0..8isize {
        task_pair_flush(this.byte_add(0xC + 0x14 * i));
        task_pair_flush(this.byte_add(0xAC + 0x14 * i));
        task_pair_flush(this.byte_add(0x1EC + 0x14 * i));
        task_pair_flush(this.byte_add(0x14C + 0x14 * i));
    }
    let new = rd::<GPtr<u8>>(this, 8);
    let old = rd::<GPtr<u8>>(this, 4);
    if new == old {
        if !new.is_null() {
            v_delete(new, 1);
        }
        wr::<u32>(this, 4, 0);
    } else {
        if !old.is_null() {
            v_delete(old, 1);
        }
        let new = rd::<GPtr<u8>>(this, 8);
        wr::<u32>(this, 4, 0);
        if !new.is_null() {
            v_delete(new, 1);
        }
    }
    let t = rd::<GPtr<u8>>(this, 0x290);
    wr::<u32>(this, 8, 0);
    if !t.is_null() {
        v_delete(t, 1);
    }
    let t = rd::<GPtr<u8>>(this, 0x294);
    wr::<u32>(this, 0x290, 0);
    if !t.is_null() {
        v_delete(t, 1);
    }
    wr::<u32>(this, 0x294, 0);
    wr::<u32>(this, 0x298, 0xFFFF_FFFF);
    wr::<u32>(this, 0x29C, 0xFFFF_FFFF);
}

/// `CPedTaskPair::Flush` (0x5E95B0; the test continues in the protection chunk 0x403956 and back at 0x5E95BE):
/// the chunk clears the ped (+0 = 0, a plain store, no reference release), then the task (+4) is deleted when
/// set, +4 = 0 and +8 (slot) = -1.
#[hook(addr = 0x5E95B0, conv = thiscall, name = "CPedTaskPair::Flush", expect = "56 8B F1 8B 4E 04",
       unverified = "mvpa-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn ped_task_pair_flush(this: GPtr<u8>) {
    let t = rd::<GPtr<u8>>(this, 4);
    wr::<u32>(this, 0, 0);
    if !t.is_null() {
        v_delete(t, 1);
    }
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0xFFFF_FFFF);
}

/// `CInformFriendsEventQueue::Flush` (0x4AC2A0): each of the 8 entries (0xA9AFB8, 0xC each: +0 ped, +4 event, +8
/// time) releases its ped reference and clears it, deletes its event and clears it, and gets time -1.
#[hook(addr = 0x4AC2A0, conv = cdecl, name = "CInformFriendsEventQueue::Flush", expect = "56 BE B8 AF A9 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn inform_friends_event_queue_flush() {
    for i in 0..8usize {
        let e = g::<u8>(0xA9AFB8 + 0xC * i);
        let p = rd::<GPtr<u8>>(e, 0);
        if !p.is_null() {
            entity_clean_up_old_reference(p, e);
            wr::<u32>(e, 0, 0);
        }
        let ev = rd::<GPtr<u8>>(e, 4);
        if !ev.is_null() {
            v_delete(ev, 1);
            wr::<u32>(e, 4, 0);
        }
        wr::<u32>(e, 8, 0xFFFF_FFFF);
    }
}

/// `CTask::operator new(uint32 size)` (0x61A5A0, cdecl): `CTaskPool::New` on `CPools::ms_pTaskPool` (the size is
/// ignored).
#[hook(addr = 0x61A5A0, conv = cdecl, name = "CTask::operator new", expect = "8B 0D A8 44 B7 00",
       unverified = "mvpa-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_operator_new(_size: u32) -> GPtr<u8> {
    task_pool_new_o(g::<GPtr<u8>>(TASK_POOL).read())
}

/// `CTaskPool::New` (0x61A500, `CPool<CTask>::New`): +0 objects (0x80 each), +4 flags, +8 size, +0xC first free.
/// Advances +0xC to the next slot whose flag has bit 0x80 (free), wrapping once to 0 (a second wrap returns null
/// with +0xC = 0); the flag's free bit is cleared and its 7-bit id incremented (bit 7 kept). Returns the slot.
#[hook(addr = 0x61A500, conv = thiscall, name = "CTaskPool::New", expect = "8B 51 08 56 32 C0",
       unverified = "mvpa-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_pool_new(this: GPtr<u8>) -> GPtr<u8> {
    let size = rd::<i32>(this, 8);
    let mut wrapped = false;
    loop {
        let i = rd::<i32>(this, 0xC).wrapping_add(1);
        wr::<i32>(this, 0xC, i);
        if i == size {
            wr::<i32>(this, 0xC, 0);
            if wrapped {
                return GPtr::NULL;
            }
            wrapped = true;
        }
        let flags = rd::<GPtr<u8>>(this, 4);
        let i = rd::<i32>(this, 0xC) as isize;
        if rd::<u8>(flags, i) & 0x80 != 0 {
            break;
        }
    }
    let i = rd::<i32>(this, 0xC) as isize;
    let flags = rd::<GPtr<u8>>(this, 4);
    wr::<u8>(flags, i, rd::<u8>(flags, i) & 0x7F);
    let i = rd::<i32>(this, 0xC) as isize;
    let flags = rd::<GPtr<u8>>(this, 4);
    let b = rd::<u8>(flags, i);
    wr::<u8>(flags, i, (b & 0x80) | (b.wrapping_add(1) & 0x7F));
    let i = rd::<i32>(this, 0xC);
    rd::<GPtr<u8>>(this, 0).byte_add((i << 7) as isize)
}

/// `CTaskSimple::CTaskSimple` (0x61A390): `m_pParentTask` (+4) = 0, vtable 0x86D4A8. Returns this.
#[hook(addr = 0x61A390, conv = thiscall, name = "FUN_0061a390", expect = "8B C1 C7 40 04 00 00 00 00",
       unverified = "mvpa-ai-1", route = "L2", hazards = "heap-writes,preserved-regs", preserve = "ecx", recs = "grove-walk")]
pub fn task_simple_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 0, 0x86D4A8);
    this
}

/// `CTask::operator delete(void*)` (0x61A5B0, cdecl): slot `(p - objects) >> 7` (arithmetic) of the task pool gets
/// its free bit (0x80) set, and becomes the first-free index when below it (signed). No null test.
#[hook(addr = 0x61A5B0, conv = cdecl, name = "CTask::operator delete", expect = "8B 0D A8 44 B7 00 8B 44 24 04",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_operator_delete(p: GPtr<u8>) {
    let pool = g::<GPtr<u8>>(TASK_POOL).read();
    let i = (p.addr() as i32).wrapping_sub(rd::<i32>(pool, 0)) >> 7;
    let flags = rd::<GPtr<u8>>(pool, 4);
    wr::<u8>(flags, i as isize, rd::<u8>(flags, i as isize) | 0x80);
    if i < rd::<i32>(pool, 0xC) {
        wr::<i32>(pool, 0xC, i);
    }
}

/// `CEventScriptCommand::~CEventScriptCommand` (0x4B0A50): vtable 0x85B378, deletes the task (+0x10) when set,
/// then the `CEvent` vtable 0x85AB38 (the SEH frame has no other effect).
#[hook(addr = 0x4B0A50, conv = thiscall, name = "FUN_004b0a50", expect = "6A FF 68 18 A0 83 00",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_script_command_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85B378);
    let t = rd::<GPtr<u8>>(this, 0x10);
    if !t.is_null() {
        v_delete(t, 1);
    }
    wr::<u32>(this, 0, 0x85AB38);
}

/// `CEventAcquaintancePedRespect::~CEventAcquaintancePedRespect` (0x5FF030): vtable 0x86CA38, then
/// `CEventAcquaintancePed::~CEventAcquaintancePed` (0x4AF890, tail jump).
#[hook(addr = 0x5FF030, conv = thiscall, name = "FUN_005ff030", expect = "C7 01 38 CA 86 00",
       unverified = "mvpa-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_acq_ped_respect_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86CA38);
    event_acquaintance_ped_dtor(this);
}

/// `CEventAcquaintancePedLike::~CEventAcquaintancePedLike` (0x5FF0F0): vtable 0x86CA80, then
/// `CEventAcquaintancePed::~CEventAcquaintancePed` (0x4AF890, tail jump).
#[hook(addr = 0x5FF0F0, conv = thiscall, name = "FUN_005ff0f0", expect = "C7 01 80 CA 86 00",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_acq_ped_like_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86CA80);
    event_acquaintance_ped_dtor(this);
}

/// `CEventSeenCop::~CEventSeenCop` (0x5FF3A0): vtable 0x86CB58, then `CEventAcquaintancePed::~CEventAcquaintancePed`
/// (0x4AF890, tail jump).
#[hook(addr = 0x5FF3A0, conv = thiscall, name = "FUN_005ff3a0", expect = "C7 01 58 CB 86 00",
       unverified = "mvpa-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_seen_cop_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86CB58);
    event_acquaintance_ped_dtor(this);
}

/// `CEventAcquaintancePedDislike::~CEventAcquaintancePedDislike` (0x5FF1B0): vtable 0x86CAC8, then
/// `CEventAcquaintancePed::~CEventAcquaintancePed` (0x4AF890, tail jump).
#[hook(addr = 0x5FF1B0, conv = thiscall, name = "FUN_005ff1b0", expect = "C7 01 C8 CA 86 00",
       unverified = "mvpa-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_acq_ped_dislike_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86CAC8);
    event_acquaintance_ped_dtor(this);
}

/// `CEventInAir::~CEventInAir` (0x4B0D00): only the `CEvent` vtable 0x85AB38.
#[hook(addr = 0x4B0D00, conv = thiscall, name = "FUN_004b0d00", expect = "C7 01 38 AB 85 00 C3",
       unverified = "mvpa-ai-1", route = "L2", hazards = "heap-writes,preserved-regs", preserve = "ecx", recs = "grove-walk")]
pub fn event_in_air_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85AB38);
}

/// `CEvent::~CEvent` (0x4AC010): only the `CEvent` vtable 0x85AB38.
#[hook(addr = 0x4AC010, conv = thiscall, name = "FUN_004ac010", expect = "C7 01 38 AB 85 00 C3",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85AB38);
}

/// `CTaskSimpleStandStill::~CTaskSimpleStandStill` (0x62F360): vtable 0x86DD2C, then `CTaskSimple::~CTaskSimple`
/// (0x61A3A0, tail jump).
#[hook(addr = 0x62F360, conv = thiscall, name = "FUN_0062f360", expect = "C7 01 2C DD 86 00",
       unverified = "mvpa-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_simple_stand_still_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86DD2C);
    task_simple_dtor(this);
}

/// `CTaskSimpleGoTo::~CTaskSimpleGoTo` (0x667D50): vtable 0x86FD1C, then `CTaskSimple::~CTaskSimple` (0x61A3A0, tail
/// jump).
#[hook(addr = 0x667D50, conv = thiscall, name = "FUN_00667d50", expect = "C7 01 1C FD 86 00",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_simple_go_to_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86FD1C);
    task_simple_dtor(this);
}

/// `CTaskComplexCarDriveWander::~CTaskComplexCarDriveWander` (0x63CB50): vtable 0x86E96C, then
/// `CTaskComplexCarDrive::~CTaskComplexCarDrive` (0x63CA40, tail jump).
#[hook(addr = 0x63CB50, conv = thiscall, name = "FUN_0063cb50", expect = "C7 01 6C E9 86 00",
       unverified = "mvpa-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk,driving")]
pub fn task_complex_car_drive_wander_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86E96C);
    task_complex_car_drive_dtor(this);
}

/// `CTaskSimpleIKLookAt::~CTaskSimpleIKLookAt` (0x633EF0): vtable 0x86E37C, then
/// `CTaskSimpleIKChain::~CTaskSimpleIKChain` (0x633A90, tail jump).
#[hook(addr = 0x633EF0, conv = thiscall, name = "CTaskSimpleIKLookAt::Destructor-", expect = "C7 01 7C E3 86 00",
       unverified = "mvpa-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn task_simple_ik_look_at_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86E37C);
    task_simple_ik_chain_dtor(this);
}

/// `CPedGroupDefaultTaskAllocatorFollowAnyMeans::ctor` (0x5F6450): stores vtable 0x86C75C. Returns this.
#[hook(addr = 0x5F6450, conv = thiscall, name = "FUN_005f6450", expect = "8B C1 C7 00 5C C7 86 00",
       route = "split", hazards = "none",
       note = "run only by its CRT static initialiser (before any recorded frame)")]
pub fn allocator_follow_any_means_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0x86C75C);
    this
}

/// `CPedGroupDefaultTaskAllocatorFollowLimited::ctor` (0x5F6480): stores vtable 0x86C764. Returns this.
#[hook(addr = 0x5F6480, conv = thiscall, name = "FUN_005f6480", expect = "8B C1 C7 00 64 C7 86 00",
       route = "split", hazards = "none",
       note = "run only by its CRT static initialiser (before any recorded frame)")]
pub fn allocator_follow_limited_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0x86C764);
    this
}

/// `CPedGroupDefaultTaskAllocatorStandStill::ctor` (0x5F64B0): stores vtable 0x86C76C. Returns this.
#[hook(addr = 0x5F64B0, conv = thiscall, name = "FUN_005f64b0", expect = "8B C1 C7 00 6C C7 86 00",
       route = "split", hazards = "none",
       note = "run only by its CRT static initialiser (before any recorded frame)")]
pub fn allocator_stand_still_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0x86C76C);
    this
}

/// `CPedGroupDefaultTaskAllocatorChat::ctor` (0x5F64E0): stores vtable 0x86C774. Returns this.
#[hook(addr = 0x5F64E0, conv = thiscall, name = "FUN_005f64e0", expect = "8B C1 C7 00 74 C7 86 00",
       route = "split", hazards = "none",
       note = "run only by its CRT static initialiser (before any recorded frame)")]
pub fn allocator_chat_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0x86C774);
    this
}

/// `CPedGroupDefaultTaskAllocatorRandom::ctor` (0x5F6510): stores vtable 0x86C77C. Returns this.
#[hook(addr = 0x5F6510, conv = thiscall, name = "FUN_005f6510", expect = "8B C1 C7 00 7C C7 86 00",
       route = "split", hazards = "none",
       note = "run only by its CRT static initialiser (before any recorded frame)")]
pub fn allocator_random_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0x86C77C);
    this
}

/// `CPedGroupDefaultTaskAllocatorSitInLeaderCar::ctor` (0x5F6540): stores vtable 0x86C784. Returns this.
#[hook(addr = 0x5F6540, conv = thiscall, name = "FUN_005f6540", expect = "8B C1 C7 00 84 C7 86 00",
       route = "split", hazards = "none",
       note = "run only by its CRT static initialiser (before any recorded frame)")]
pub fn allocator_sit_in_leader_car_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0x86C784);
    this
}

/// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCentreUncached(float z, const CVector* corners, CVector& out)`
/// (0x5F1600, cdecl): out = (0, 0, z) then out.xy = corner 0's xy, then `out.x = c[k].x + out.x` /
/// `out.y = c[k].y + out.y` for k = 1..3 (each stored as f32), then both scaled by [0x858C84] (0.25).
#[hook(addr = 0x5F1600, conv = cdecl, name = "CPedGeometryAnalyser::ComputeEntityBoundingBoxCentreUncached", expect = "8B 44 24 0C 8B 4C 24 04",
       unverified = "mvpa-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn compute_entity_bounding_box_centre_uncached(z: u32, corners: GPtr<u8>, out: GPtr<u8>) {
    wr::<u32>(out, 4, 0);
    wr::<u32>(out, 0, 0);
    wr::<u32>(out, 8, z);
    wr::<u32>(out, 0, rd::<u32>(corners, 0));
    wr::<u32>(out, 4, rd::<u32>(corners, 4));
    for k in 1..4isize {
        wr::<f32>(out, 0, f32_of(x(rd::<f32>(corners, 0xC * k)).fadd_m(rd::<f32>(out, 0))));
        wr::<f32>(out, 4, f32_of(x(rd::<f32>(corners, 0xC * k + 4)).fadd_m(rd::<f32>(out, 4))));
    }
    let q = g::<f32>(0x858C84).read();
    wr::<f32>(out, 0, f32_of(x(rd::<f32>(out, 0)).fmul_m(q)));
    wr::<f32>(out, 4, f32_of(x(rd::<f32>(out, 4)).fmul_m(q)));
}

/// `CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanesUncached(float z, const CVector* corners, CVector* normals,
/// float* distances)` (0x5F1670, cdecl; `z` unused): for each edge i (from corner i-1, starting at corner 3, to
/// corner i): d = c[i] - prev (f32 each), `CVector::Normalise(d)`, normal i = (d.y, -d.x, 0), distance i =
/// `-((prev.z*n.z + prev.y*n.y) + prev.x*n.x)` with the normal re-read from memory.
#[hook(addr = 0x5F1670, conv = cdecl, name = "CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanesUncached", expect = "83 EC 24 8B 44 24 2C",
       unverified = "mvpa-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn compute_entity_bounding_box_planes_uncached(_z: u32, corners: GPtr<u8>, normals: GPtr<u8>, dist: GPtr<u8>) {
    let mut prev = [rd::<f32>(corners, 0x24), rd::<f32>(corners, 0x28), rd::<f32>(corners, 0x2C)];
    for i in 0..4isize {
        let c = corners.byte_add(0xC * i);
        let mut d = [0u32; 3];
        let dp = local(&mut d);
        wr::<f32>(dp, 0, f32_of(x(rd::<f32>(c, 0)).fsub_m(prev[0])));
        wr::<f32>(dp, 4, f32_of(x(rd::<f32>(c, 4)).fsub_m(prev[1])));
        wr::<f32>(dp, 8, f32_of(x(rd::<f32>(c, 8)).fsub_m(prev[2])));
        vector_normalise(dp);
        let n = normals.byte_add(0xC * i);
        wr::<u32>(n, 0, rd::<u32>(dp, 4));
        wr::<f32>(n, 4, f32_of(x(rd::<f32>(dp, 0)).fchs()));
        wr::<u32>(n, 8, 0);
        let v = x(prev[2])
            .fmul_m(rd::<f32>(n, 8))
            .fadd(x(prev[1]).fmul_m(rd::<f32>(n, 4)))
            .fadd(x(prev[0]).fmul_m(rd::<f32>(n, 0)))
            .fchs();
        wr::<f32>(dist, 4 * i, f32_of(v));
        prev = [rd::<f32>(c, 0), rd::<f32>(c, 4), rd::<f32>(c, 8)];
    }
}

/// `CPedGeometryAnalyser::ComputeEntityBoundingBoxSegmentPlanesUncached(const CVector* corners, const CVector& centre,
/// CVector* normals, float* distances)` (0x5F1750, cdecl): for each corner i: normal i = (-(c.y - ctr.y),
/// c.x - ctr.x, 0) (not normalised), distance i = `-((c.z*n.z + n.y*c.y) + n.x*c.x)` with the normal re-read.
#[hook(addr = 0x5F1750, conv = cdecl, name = "CPedGeometryAnalyser::ComputeEntityBoundingBoxSegmentPlanesUncached", expect = "83 EC 0C 8B 44 24 10",
       unverified = "mvpa-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn compute_entity_bounding_box_segment_planes_uncached(corners: GPtr<u8>, centre: GPtr<u8>, normals: GPtr<u8>, dist: GPtr<u8>) {
    for i in 0..4isize {
        let c = corners.byte_add(0xC * i);
        let dx = x(rd::<f32>(c, 0)).fsub_m(rd::<f32>(centre, 0));
        let dy = x(rd::<f32>(c, 4)).fsub_m(rd::<f32>(centre, 4));
        let n = normals.byte_add(0xC * i);
        wr::<f32>(n, 0, f32_of(dy.fchs()));
        wr::<f32>(n, 4, f32_of(dx));
        wr::<u32>(n, 8, 0);
        let v = x(rd::<f32>(c, 8))
            .fmul_m(rd::<f32>(n, 8))
            .fadd(x(rd::<f32>(n, 4)).fmul_m(rd::<f32>(c, 4)))
            .fadd(x(rd::<f32>(n, 0)).fmul_m(rd::<f32>(c, 0)))
            .fchs();
        wr::<f32>(dist, 4 * i, f32_of(v));
    }
}

/// `CPedGeometryAnalyser::ComputeEntityHitSide(const CVector& pos, CEntity&)` (0x5F3AC0, cdecl): corners of the
/// entity's box at `pos.z`, their centre, the segment planes, then `ComputeEntityHitSide(pos, normals, distances)`
/// (EAX passed on).
#[hook(addr = 0x5F3AC0, conv = cdecl, name = "CPedGeometryAnalyser::ComputeEntityHitSide-3", expect = "81 EC 80 00 00 00",
       unverified = "mvpa-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn compute_entity_hit_side_3(p: GPtr<u8>, e: GPtr<u8>) -> u32 {
    let z = rd::<u32>(p, 8);
    let mut frame = [0u32; 0x20];
    let f = local(&mut frame);
    // +4 centre, +0x10 distances, +0x20 corners, +0x50 normals (the original frame layout)
    bbox_corners_uncached(z, e, f.byte_add(0x20));
    bbox_centre_uncached(z, f.byte_add(0x20), f.byte_add(4));
    bbox_segment_planes_uncached(f.byte_add(0x20), f.byte_add(4), f.byte_add(0x50), f.byte_add(0x10));
    entity_hit_side_2(p, f.byte_add(0x50), f.byte_add(0x10))
}

/// `CPedGeometryAnalyser::ComputeEntityHitSide(const CPed&, CEntity&)` (0x5F3BC0, cdecl): tail jump to
/// `ComputeEntityHitSide(pedPos, entity)` with the ped's position (matrix, else placement). The original overwrites
/// its first argument slot with that pointer; callers do not read it back.
#[hook(addr = 0x5F3BC0, conv = cdecl, name = "CPedGeometryAnalyser::ComputeEntityHitSide-1", expect = "8B 4C 24 04 8B 41 14",
       unverified = "mvpa-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn compute_entity_hit_side_1(ped: GPtr<u8>, e: GPtr<u8>) -> u32 {
    entity_hit_side_3(pos(ped), e)
}

/// `CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox(const CPed&, CEntity&, const CVector&, CPointRoute&,
/// int32)` (0x5F6110, cdecl): the 6-argument overload with the ped's position inserted second (EAX passed on).
#[hook(addr = 0x5F6110, conv = cdecl, name = "CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox-Entity", expect = "8B 4C 24 04 8B 41 14",
       unverified = "mvpa-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn compute_route_round_entity_bounding_box_entity(ped: GPtr<u8>, e: GPtr<u8>, target: u32, route: u32, flag: u32) -> u32 {
    route_round_entity_bbox_2(ped, pos(ped), e, target, route, flag)
}

/// `CPedGeometryAnalyser::CanPedTargetPed(const CPed& ped, const CPed& target, bool checkDirection)` (0x5F1C40,
/// cdecl): point = target position with z + [0x858F34] (f32); when the target's task manager (+0x47C, +4) has a
/// secondary task in slot 1 (ducking), z -= [0x858624]. Returns `CanPedTargetPoint(ped, point, check)` (full EAX).
#[hook(addr = 0x5F1C40, conv = cdecl, name = "CPedGeometryAnalyser::CanPedTargetPed", expect = "8B 4C 24 08 8B 41 14",
       unverified = "mvpa-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn can_ped_target_ped(ped: GPtr<u8>, target: GPtr<u8>, check: u32) -> u32 {
    let p = pos(target);
    let mut pt = [0u32; 3];
    let pp = local(&mut pt);
    let z = f32_of(x(rd::<f32>(p, 8)).fadd(cf(0x858F34)));
    wr::<u32>(pp, 0, rd::<u32>(p, 0));
    wr::<u32>(pp, 4, rd::<u32>(p, 4));
    wr::<f32>(pp, 8, z);
    let tm = rd::<GPtr<u8>>(target, 0x47C).byte_add(4);
    if !task_manager_get_task_secondary(tm, 1).is_null() {
        wr::<f32>(pp, 8, f32_of(x(rd::<f32>(pp, 8)).fsub(cf(0x858624))));
    }
    can_ped_target_point_o(ped, pp, check)
}

/// `CPedGeometryAnalyser::CanPedTargetPoint(const CPed&, const CVector& point, bool checkDirection)` (0x5F1B70,
/// cdecl, AL): d = point - ped position. With the check, false unless `(dz*fwd.z + dy*fwd.y) + dx*fwd.x` >
/// [0x858B50] or unordered (the matrix is read without a null test). False when `C*C < (dz*dz + dx*dx) + dy*dy`
/// (C = [0x86C69C]). Else `CWorld::GetIsLineOfSightClear(ped position with z + [0x858F34], point, 1, 0, 0, 1, 0, 1,
/// 0)` (full EAX). The false paths leave the FPU status word in AH and the position pointer's upper half in EAX.
#[hook(addr = 0x5F1B70, conv = cdecl, name = "CPedGeometryAnalyser::CanPedTargetPoint", expect = "83 EC 0C 56 8B 74 24 14",
       unverified = "mvpa-ai-1", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn can_ped_target_point(ped: GPtr<u8>, point: GPtr<u8>, check: u32) -> u32 {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let p = pos(ped);
    let dx = x(rd::<f32>(point, 0)).fsub_m(rd::<f32>(p, 0));
    let dy = x(rd::<f32>(point, 4)).fsub_m(rd::<f32>(p, 4));
    let dz = x(rd::<f32>(point, 8)).fsub_m(rd::<f32>(p, 8));
    if check as u8 != 0 {
        let dot = dz
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(dy.fmul_m(rd::<f32>(m, 0x14)))
            .fadd(dx.fmul_m(rd::<f32>(m, 0x10)));
        if !dot.fcom(cf(0x858B50)).test_parity(5) {
            return 0;
        }
    }
    let d2 = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
    let c = cf(0x86C69C);
    if !c.fmul(c).fcom(d2).test_parity(5) {
        return 0;
    }
    let p = pos(ped);
    let mut o = [0u32; 3];
    let op = local(&mut o);
    let z = f32_of(x(rd::<f32>(p, 8)).fadd(cf(0x858F34)));
    wr::<u32>(op, 0, rd::<u32>(p, 0));
    wr::<u32>(op, 4, rd::<u32>(p, 4));
    wr::<f32>(op, 8, z);
    world_get_is_line_of_sight_clear(op, point, 1, 0, 0, 1, 0, 1, 0)
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_INTEL_SET_PED_DECISION_MAKER_TYPE,
    HOOK_PGI_SET_GROUP_DECISION_MAKER_TYPE,
    HOOK_DECISION_FROM,
    HOOK_CONVERSATIONS_CLEAR,
    HOOK_COVER_INIT,
    HOOK_TASK_SEQUENCES_INIT,
    HOOK_MEMBERSHIP_REMOVE_MEMBER,
    HOOK_PGI_FLUSH,
    HOOK_PED_TASK_PAIR_FLUSH,
    HOOK_INFORM_FRIENDS_EVENT_QUEUE_FLUSH,
    HOOK_TASK_OPERATOR_NEW,
    HOOK_TASK_POOL_NEW,
    HOOK_TASK_SIMPLE_CTOR,
    HOOK_TASK_OPERATOR_DELETE,
    HOOK_EVENT_SCRIPT_COMMAND_DTOR,
    HOOK_EVENT_ACQ_PED_RESPECT_DTOR,
    HOOK_EVENT_ACQ_PED_LIKE_DTOR,
    HOOK_EVENT_SEEN_COP_DTOR,
    HOOK_EVENT_ACQ_PED_DISLIKE_DTOR,
    HOOK_EVENT_IN_AIR_DTOR,
    HOOK_EVENT_DTOR,
    HOOK_TASK_SIMPLE_STAND_STILL_DTOR,
    HOOK_TASK_SIMPLE_GO_TO_DTOR,
    HOOK_TASK_COMPLEX_CAR_DRIVE_WANDER_DTOR,
    HOOK_TASK_SIMPLE_IK_LOOK_AT_DTOR,
    HOOK_ALLOCATOR_FOLLOW_ANY_MEANS_CTOR,
    HOOK_ALLOCATOR_FOLLOW_LIMITED_CTOR,
    HOOK_ALLOCATOR_STAND_STILL_CTOR,
    HOOK_ALLOCATOR_CHAT_CTOR,
    HOOK_ALLOCATOR_RANDOM_CTOR,
    HOOK_ALLOCATOR_SIT_IN_LEADER_CAR_CTOR,
    HOOK_COMPUTE_ENTITY_BOUNDING_BOX_CENTRE_UNCACHED,
    HOOK_COMPUTE_ENTITY_BOUNDING_BOX_PLANES_UNCACHED,
    HOOK_COMPUTE_ENTITY_BOUNDING_BOX_SEGMENT_PLANES_UNCACHED,
    HOOK_COMPUTE_ENTITY_HIT_SIDE_3,
    HOOK_COMPUTE_ENTITY_HIT_SIDE_1,
    HOOK_COMPUTE_ROUTE_ROUND_ENTITY_BOUNDING_BOX_ENTITY,
    HOOK_CAN_PED_TARGET_PED,
    HOOK_CAN_PED_TARGET_POINT,
];
