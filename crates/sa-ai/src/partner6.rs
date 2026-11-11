//! Partner, couple, paired-attractor, leave-car-and-wander and wander tasks (PORT-AI-6). Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AI` only as reading aids.
//!
//! `CTaskComplexPartner` (0x70 bytes, vtable 0x870664): +0x14 command name, +0x34 task id, +0x38 partner, +0x3C
//! distance, +0x40 point, +0x4C target point, +0x58 lead, +0x59 state (i8), +0x5A first-to-target flag (i8), +0x5B
//! update-direction count (i8), +0x5C completed, +0x5D always face partner, +0x5E anim block name, +0x6E anims
//! referenced, +0x70 go-to frame counter (i16). Subclasses: Deal (0x870754, 0x74), Greet (0x87078C, 0x78, +0x74 greet
//! type), Chat (0x8707C4, 0x78, +0x74..+0x76), Shove (0x870800, 0x74). Every call into another catalogued function
//! goes through `orig!` at its original address; tasks come from the original pool and keep the original vtables.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `g_ikChainMan`
const IK_CHAIN_MAN: usize = 0xC15448;
/// `0.5f` (`[0x86FC7C]`, the turn-to-face heading change rate the partner tasks pass)
const TURN_RATE: usize = 0x86FC7C;
/// `8.0f` (stand-still blend delta, pushed as an immediate)
const F8: u32 = 0x4100_0000;
/// `4.0f` (run-anim blend delta, pushed as an immediate)
const F4: u32 = 0x4080_0000;

orig! {
    // ---- virtual slots ----
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskComplexWander::GetWanderType` (vtable +0x2C)
    fn v_wander_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x2C => unported;
    /// `CTaskComplexPartner::GetPartnerSequence()` (vtable +0x34)
    fn v_get_partner_sequence(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x34 => unported;

    // ---- this module's own functions (called through the original address) ----
    /// `CTaskComplexPartner::GetPartnerState()` (0x6822B0): the partner's task +0x59 (sign-extended) or 0.
    fn partner_get_partner_state(t: GPtr<u8>) -> i32 = thiscall 0x6822B0 => unported;
    /// `CTaskComplexPartner::ComputeMeetingPositions(CPed*, CVector*, CVector*)` (0x681FE0, `ret 0xC`)
    fn partner_compute_meeting_positions(t: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = thiscall 0x681FE0 => unported;
    /// `CTaskComplexPartnerGreet::Constructor` (0x684210, `ret 0x20`)
    fn partner_greet_ctor_orig(t: GPtr<u8>, name: GPtr<u8>, partner: GPtr<u8>, lead: u32, dist: u32, kind: u32, vx: u32, vy: u32, vz: u32) -> GPtr<u8> = thiscall 0x684210 => unported;
    /// `CTaskComplexPartnerShove::Constructor` (0x6846F0, `ret 0x20`)
    fn partner_shove_ctor_orig(t: GPtr<u8>, name: GPtr<u8>, partner: GPtr<u8>, lead: u32, dist: u32, upd: u32, vx: u32, vy: u32, vz: u32) -> GPtr<u8> = thiscall 0x6846F0 => unported;
    /// `CTaskComplexPartnerChat::Constructor` (0x684290, `ret 0x28`)
    fn partner_chat_ctor_orig(t: GPtr<u8>, name: GPtr<u8>, partner: GPtr<u8>, lead: u32, dist: u32, upd: u32, a5: u32, a6: u32, vx: u32, vy: u32, vz: u32) -> GPtr<u8> = thiscall 0x684290 => unported;
    /// `CTaskComplexPartner::Constructor` (0x681E70, `ret 0x24`)
    fn partner_ctor_orig(t: GPtr<u8>, name: GPtr<u8>, partner: GPtr<u8>, lead: u32, dist: u32, face: u32, upd: u32, vx: u32, vy: u32, vz: u32) -> GPtr<u8> = thiscall 0x681E70 => unported;
    /// `CTaskComplexBeInCouple::AbortArmIK(CPed*)` (0x6837F0, `ret 4`)
    fn be_in_couple_abort_arm_ik(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6837F0 => unported;
    /// `CTaskComplexBeInCouple::GetSideOfPartner(CPed*)` (0x683840, `ret 4`; catalogue `FUN_00683840`): 1 or 2.
    fn be_in_couple_get_side(t: GPtr<u8>, ped: GPtr<u8>) -> i32 = thiscall 0x683840 => unported;
    /// `CTaskComplexUsePairedAttractor::SetPartnership(CPed*, bool, pair*)` (0x6331F0, `ret 0xC`)
    fn paired_set_partnership(t: GPtr<u8>, partner: GPtr<u8>, lead: u32, pair: GPtr<u8>) = thiscall 0x6331F0 => unported;
    /// `CTaskComplexWanderCriminal::LookForCarsToSteal(CPed*)` (0x66B4F0, `ret 4`)
    fn wander_criminal_look_for_cars_to_steal(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x66B4F0 => unported;

    // ---- task bases and constructors ----
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskSimpleStandStill::Constructor(int32 time, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, time: i32, a: u32, b: u32, f: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskSimpleRunAnim::Constructor(AssocGroupId, AnimationId, float, bool)` (0x61A8B0, `ret 0x10`)
    fn run_anim_ctor(t: GPtr<u8>, group: u32, anim: u32, delta: u32, b: u32) -> GPtr<u8> = thiscall 0x61A8B0 => unported;
    /// `CTaskSimpleRunTimedAnim::Constructor-2` (0x61ABE0, 8 stack arguments)
    fn run_timed_anim_ctor2(t: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32) -> GPtr<u8> = thiscall 0x61ABE0 => unported;
    /// `CTaskSimpleChat::CTaskSimpleChat(int32 time)` (0x5F65A0, `ret 4`)
    fn simple_chat_ctor(t: GPtr<u8>, time: u32) -> GPtr<u8> = thiscall 0x5F65A0 => unported;
    /// `CTaskComplexChat::Constructor(bool, CPed*, int32, int16)` (0x682CB0, `ret 0x10`; entry `push -1; jmp island`)
    fn complex_chat_ctor(t: GPtr<u8>, a: u32, partner: GPtr<u8>, b: i32, c: u32) -> GPtr<u8> = thiscall 0x682CB0 => unported;
    /// `CTaskSimpleDoHandSignal::Constructor()` (0x660880)
    fn do_hand_signal_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x660880 => unported;
    /// `CTaskComplexTurnToFaceEntityOrCoord::Constructor(CEntity*, float, float)` (0x66B890, `ret 0xC`)
    fn turn_to_face_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x66B890 => unported;
    /// `CTaskComplexWanderStandard` constructor `(int32 moveState, uint8 dir, bool)` (0x48E4F0, `ret 0xC`)
    fn wander_standard_ctor(t: GPtr<u8>, ms: u32, dir: u32, b: u32) -> GPtr<u8> = thiscall 0x48E4F0 => unported;
    /// `CTaskComplexWander` next-scan-time setter (0x66B150, `ret 4`): `+0x34 = a0`.
    fn wander_set_next_scan_time(t: GPtr<u8>, time: u32) = thiscall 0x66B150 => unported;
    /// `CTaskComplexGoToPointAndStandStill::Constructor(int32, const CVector&, float, float, bool, bool)` (0x668120,
    /// `ret 0x18`)
    fn go_to_point_and_stand_still_ctor(t: GPtr<u8>, ms: u32, p: GPtr<u8>, r: u32, d: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x668120 => unported;
    /// `FUN_00683340` (0x683340, `ret 0x10`): the `CTaskComplexWaitForPed` constructor `(CPed*, float, int32, bool)`
    fn wait_for_ped_ctor(t: GPtr<u8>, ped: GPtr<u8>, f: u32, time: u32, b: u32) -> GPtr<u8> = thiscall 0x683340 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10, `ret 4`)
    fn sequence_add_task(t: GPtr<u8>, task: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskComplexAvoidOtherPedWhileWandering::Constructor(CPed*, const CVector&, int32)` (0x66A100, `ret 0xC`)
    fn avoid_other_ped_while_wandering_ctor(t: GPtr<u8>, ped: GPtr<u8>, target: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x66A100 => unported;

    // ---- events ----
    /// `CEventVehicleToSteal::Constructor(CVehicle*)` (0x4AF670, `ret 4`)
    fn event_vehicle_to_steal_ctor(e: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AF670 => unported;
    /// `CEventVehicleToSteal` destructor (0x4AF700)
    fn event_vehicle_to_steal_dtor(e: GPtr<u8>) = thiscall 0x4AF700 => unported;
    /// `CEventAcquaintancePed::Constructor(CPed*)` (0x4AF820, `ret 4`)
    fn event_acquaintance_ped_ctor(e: GPtr<u8>, p: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AF820 => unported;
    /// `CEventAcquaintancePed` destructor (0x4AF890)
    fn event_acquaintance_ped_dtor(e: GPtr<u8>) = thiscall 0x4AF890 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;

    // ---- other catalogued functions ----
    /// `CAEPedSpeechAudioEntity::RequestPedConversation(CPed*, CPed*)` (0x4E50E0, cdecl)
    fn request_ped_conversation(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x4E50E0 => unported;
    /// `CPed::StopPlayingHandSignal()` (0x5E0480)
    fn ped_stop_playing_hand_signal(p: GPtr<u8>) = thiscall 0x5E0480 => unported;
    /// `CPed::IsAlive()` (0x5E0170)
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTaskManager::SetTask(CTask*, int32, bool)` (0x681AF0, `ret 0xC`)
    fn tm_set_task(tm: GPtr<u8>, t: GPtr<u8>, slot: i32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0, `ret 4`)
    fn intel_find_task_by_type(i: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CPedIntelligence::GetPedEntities()` (0x4893E0)
    fn intel_get_ped_entities(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x4893E0 => unported;
    /// `CPlaceable::GetTopDirection()` (0x41CCB0, `ret 4`, CVector by hidden pointer)
    fn placeable_get_top_direction(p: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x41CCB0 => unported;
    /// `CGeneral::GetNodeHeadingFromVector(float, float)` (0x53CDC0, cdecl)
    fn general_get_node_heading_from_vector(x: u32, y: u32) -> u32 = cdecl 0x53CDC0 => unported;
    /// `CVector2D::Normalise()` (0x44E480)
    fn vector2d_normalise(v: GPtr<u8>) = thiscall 0x44E480 => unported;
    /// `FUN_0040fe30(CVector* out, const CVector* a, const CVector* b)` (0x40FE30, cdecl: `*out = *a + *b`)
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn general_random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `IKChainManager_c::PointArm(name, arm, ped, target, bone, pos, speed, blendTime, cosLimit)` (0x618B60, `ret 0x24`)
    fn ik_point_arm(m: GPtr<u8>, name: u32, arm: i32, ped: GPtr<u8>, target: GPtr<u8>, bone: i32, pos: GPtr<u8>, speed: u32, blend: u32, limit: u32) = thiscall 0x618B60 => unported;
    /// `IKChainManager_c::LookAt(name, CPed*, CEntity*, int32 time, int32 bone, RwV3d*, bool, float, int32, int32, bool)`
    /// (0x618970, `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: u32, b: u32, speed: u32, blend: i32, prio: i32, b2: u32) = thiscall 0x618970 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990)
    fn anim_get_block_index(name: GPtr<u8>) -> i32 = cdecl 0x4D3990 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0)
    fn anim_add_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    fn anim_remove_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CStreaming::RequestModel(int32 id, int32 flags)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CTaskComplexGangLeader::ShouldLoadGangAnims()` (0x65E7F0, static)
    fn should_load_gang_anims() -> u8 = cdecl 0x65E7F0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CScripted2dEffects::GetEffectPairs(const C2dEffect*)` (0x6FA840, cdecl)
    fn scripted_2d_effects_get_effect_pairs(effect: GPtr<u8>) -> GPtr<u8> = cdecl 0x6FA840 => unported;
    /// `CPedAttractorManager::GetPedAttractorManager()` (0x5EE190, cdecl)
    fn get_ped_attractor_manager() -> GPtr<u8> = cdecl 0x5EE190 => unported;
    /// `CPedAttractorManager::GetPedUsingEffect(const C2dEffect*, const CEntity*)` (0x5EBE50, `ret 8`)
    fn attractor_manager_get_ped_using_effect(m: GPtr<u8>, effect: u32, entity: u32) -> GPtr<u8> = thiscall 0x5EBE50 => unported;
    /// `CPathNode::GetPosition()` (0x420A10, `ret 4`, CVector by hidden pointer)
    fn path_node_get_position(n: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x420A10 => unported;
    /// `CCarEnterExit::IsVehicleStealable(const CVehicle*, const CPed*)` (0x6510D0, cdecl)
    fn car_enter_exit_is_vehicle_stealable(v: GPtr<u8>, ped: GPtr<u8>) -> u8 = cdecl 0x6510D0 => unported;
    /// `CTaskComplexWanderStandard::ScanForStuff(CPed*)` (0x672600, `ret 4`)
    fn wander_standard_scan_for_stuff(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x672600 => unported;
    /// `CTaskComplexProstituteSolicit::IsTaskValid(CPed*, CPed*)` (0x661BB0, cdecl)
    fn prostitute_solicit_is_task_valid(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x661BB0 => unported;
}

/// An f32 of the image (`.rdata` constant or `.data` tunable), read when the original reads it.
fn c(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// A dword of the image.
fn d(addr: usize) -> u32 {
    g::<u32>(addr).read()
}

fn now() -> u32 {
    d(TIME_MS)
}

fn ik_man() -> GPtr<u8> {
    GPtr::from_addr(IK_CHAIN_MAN)
}

/// A pointer to a Rust local standing in for a slot of the original's stack frame.
fn slot<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

/// Entity position: matrix +0x30, else the placement at +4.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// The ped's `CTaskManager` (`m_pIntelligence` +4).
fn task_mgr(ped: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(ped, 0x47C).byte_add(4)
}

/// `(float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * k` (fild, fmul, fmul) then `_ftol`.
fn rand_scaled(site: u32, k_addr: usize) -> i32 {
    let r = at_site(site, rand) & 0xFFFF;
    ftol32(xi(r).fmul_m(c(0x858B14)).fmul_m(c(k_addr)))
}

/// Copies the CVector at `src` to `dst` (three dwords).
fn copy_vec(dst: GPtr<u8>, src: GPtr<u8>) {
    for k in 0..3isize {
        wr::<u32>(dst, k * 4, rd::<u32>(src, k * 4));
    }
}

/// The shared tail 0x683C4E (and 0x681F7D): a new `CTaskSimpleStandStill(50, false, false, 8.0f)` or null.
fn stand_still_50() -> GPtr<u8> {
    let p = task_new(0x20);
    if p.is_null() {
        return GPtr::NULL;
    }
    stand_still_ctor(p, 0x32, 0, 0, F8)
}

/// `p ? CTaskSimpleStandStill(p, time, false, false, 8.0f) : null` for an already allocated slot.
fn stand_still_in(p: GPtr<u8>, time: i32) -> GPtr<u8> {
    if p.is_null() {
        GPtr::NULL
    } else {
        stand_still_ctor(p, time, 0, 0, F8)
    }
}

/// A new `CTaskSimpleStandStill(time, false, false, 8.0f)` or null.
fn new_stand_still(time: i32) -> GPtr<u8> {
    stand_still_in(task_new(0x20), time)
}

/// A new `CTaskSimpleRunAnim(group, anim, 4.0f, false)` or null.
fn new_run_anim(group: u32, anim: u32) -> GPtr<u8> {
    let p = task_new(0x20);
    if p.is_null() {
        return GPtr::NULL;
    }
    run_anim_ctor(p, group, anim, F4, 0)
}

/// A new `CTaskComplexTurnToFaceEntityOrCoord(partner (+0x38), [0x86FC7C], k)` or null.
fn new_turn_to_face(this: GPtr<u8>, k: u32) -> GPtr<u8> {
    let p = task_new(0x28);
    if p.is_null() {
        return GPtr::NULL;
    }
    turn_to_face_entity_ctor(p, rd::<GPtr<u8>>(this, 0x38), d(TURN_RATE), k)
}

/// A new `CTaskSimpleDoHandSignal` or null.
fn new_hand_signal() -> GPtr<u8> {
    let p = task_new(0xC);
    if p.is_null() {
        return GPtr::NULL;
    }
    do_hand_signal_ctor(p)
}

/// A new `CTaskSimpleChat(time)` or null.
fn new_simple_chat(time: u32) -> GPtr<u8> {
    let p = task_new(0x34);
    if p.is_null() {
        return GPtr::NULL;
    }
    simple_chat_ctor(p, time)
}

/// A new `CTaskSimpleRunTimedAnim(0, 0xC, 4.0f, -4.0f, time, 0x1A0, 0x86C7B4, false)` with vtable 0x86C78C, or null.
fn new_timed_anim(time: u32) -> GPtr<u8> {
    let p = task_new(0x34);
    if p.is_null() {
        return GPtr::NULL;
    }
    run_timed_anim_ctor2(p, 0, 0xC, F4, 0xC080_0000, time, 0x1A0, 0x86C7B4, 0);
    wr::<u32>(p, 0, 0x86C78C);
    p
}

/// `p = new(0x40); p ? CTaskComplexSequence(p) : null` (the partner sequences add to a null sequence when the pool
/// is full: original behaviour, kept).
fn new_sequence() -> GPtr<u8> {
    let p = task_new(0x40);
    if p.is_null() {
        return GPtr::NULL;
    }
    sequence_ctor(p)
}

// ======================================================================================================
// CTaskComplexPartner
// ======================================================================================================

/// `CTaskComplexPartner::CreateFirstSubTask(CPed*)` (0x681F20, `ret 4`; Deal's slot 0x6825A0 jumps here). A lead
/// chat (task id 0x4B4, +0x74 set) requests the conversation (`RequestPedConversation(ped, partner)`): granted sets
/// +0x76; refused returns null unless +0x75, which clears +0x74. Then `StopPlayingHandSignal` and a new
/// `CTaskSimpleStandStill(50, false, false, 8.0f)`.
#[hook(addr = 0x681F20, conv = thiscall, name = "CTaskComplexPartner::CreateFirstSubTask", expect = "64 A1 00 00 00 00")]
pub fn partner_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(this, 0x58) != 0 && rd::<i32>(this, 0x34) == 0x4B4 && rd::<u8>(this, 0x74) != 0 {
        if request_ped_conversation(ped, rd::<GPtr<u8>>(this, 0x38)) != 0 {
            wr::<u8>(this, 0x76, 1);
        } else if rd::<u8>(this, 0x75) == 0 {
            return GPtr::NULL;
        } else {
            wr::<u8>(this, 0x74, 0);
        }
    }
    ped_stop_playing_hand_signal(ped);
    stand_still_50()
}

/// State 5 with `m_makePedAlwaysFacePartner` and no first-to-target decision (0x683CB0-0x683E34): the 2D offset
/// from the partner to the ped; within (0.99, 1.01) the state becomes 6, else the ped's anim shift (+0x4D8 / +0x4DC)
/// steers it towards distance 1 (at most 0.02 per step, in the ped's right/forward frame; matrix not null-checked).
fn partner_face_partner_step(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let pp = ent_pos(ped);
    let px = rd::<f32>(pp, 0);
    let py = rd::<f32>(pp, 4);
    let qp = ent_pos(rd::<GPtr<u8>>(this, 0x38));
    let qx = rd::<f32>(qp, 0);
    let qy = rd::<f32>(qp, 4);
    let dy = x(py).fsub_m(qy);
    let dx = x(px).fsub_m(qx);
    // the 2D vector at [esp+0x10] (CVector2D)
    let mut v = [f32_of(dx), f32_of(dy)];
    let len = x(v[0]).fmul_m(v[0]).fadd(dy.fmul(dy)).fsqrt();
    if !len.fcom(x(c(0x862CD0))).test_any(0x41) && !len.fcom(x(c(0x870750))).test_parity(0x5) {
        wr::<u8>(this, 0x59, 6);
        return stand_still_50();
    }
    vector2d_normalise(slot(&mut v));
    let my = f32_of(x(v[1]).fadd_m(qy).fsub_m(py));
    let mx = x(v[0]).fadd_m(qx).fsub_m(px);
    v = [f32_of(mx), my];
    let m_len = mx.fmul(mx).fadd(x(my).fmul_m(my)).fsqrt();
    let (sx, sy) = if !m_len.fcom(x(c(0x858B38))).test_any(0x41) {
        vector2d_normalise(slot(&mut v));
        (x(v[0]).fmul_m(c(0x858B38)), x(v[1]).fmul_m(c(0x858B38)))
    } else {
        (x(v[0]), x(v[1]))
    };
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let r0 = x(rd::<f32>(m, 0)).fmul(sx);
    let r1 = x(rd::<f32>(m, 4)).fmul(sy);
    wr::<f32>(ped, 0x4D8, f32_of(r0.fadd(r1)));
    let f0 = x(rd::<f32>(m, 0x10)).fmul(sx);
    let f1 = x(rd::<f32>(m, 0x14)).fmul(sy);
    wr::<f32>(ped, 0x4DC, f32_of(f0.fadd(f1)));
    stand_still_50()
}

/// `CTaskComplexPartner::CreateNextSubTask(CPed*)` (0x683AD0, `ret 4`; the epilogue continues through the straight
/// island 0x401A58). Null unless the partner runs a task of type +0x34 whose partner is `ped`. By state +0x59:
/// 6: with the anims referenced and a direction update left (+0x5B, decremented), `GetPartnerSequence()` (vtable
/// +0x34); 5: without always-face, once the partner's state is >= 5 a non-scared wander task in primary slot 4 is
/// replaced by a `CTaskComplexWanderStandard(4, GetNodeHeadingFromVector(-top.x, -top.y), true)` scanning after
/// now + [0x86FCDC] (the setter 0x66B150 runs on a null task when the pool is full: original behaviour) and the state
/// becomes 6; with always-face see `partner_face_partner_step`, or state 6 when the first-to-target flag is 1 and
/// the partner is in state 6; 4: first-to-target bookkeeping on both tasks (state 5 when the partner is in 4/5, else
/// the partner's point = this target point); 3: a turn-to-face after a 0x387 sub-task, state 4 after a 0x398;
/// 2 (after a 0xCB sub-task): a go-to-point-and-stand-still towards +0x40 (state 3, +0x70 = 0) unless the point's
/// x and y are 0; 1 (after 0xCB): the lead computes the meeting positions and gives the partner its target, state 2.
/// The other paths return a `CTaskSimpleStandStill(50)`.
#[hook(addr = 0x683AD0, conv = thiscall, name = "CTaskComplexPartner::CreateNextSubTask", expect = "6A FF 64 A1 00 00 00 00")]
pub fn partner_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let partner = rd::<GPtr<u8>>(this, 0x38);
    if partner.is_null() {
        return GPtr::NULL;
    }
    let t = tm_find_active_task_by_type(task_mgr(partner), rd::<i32>(this, 0x34));
    if t.is_null() || rd::<GPtr<u8>>(t, 0x38) != ped {
        return GPtr::NULL;
    }
    let find_partner_task =
        || tm_find_active_task_by_type(task_mgr(rd::<GPtr<u8>>(this, 0x38)), rd::<i32>(this, 0x34));
    match rd::<u8>(this, 0x59) {
        6 => {
            if rd::<u8>(this, 0x6E) == 0 {
                return GPtr::NULL;
            }
            let n = rd::<u8>(this, 0x5B);
            if n == 0 {
                return GPtr::NULL;
            }
            wr::<u8>(this, 0x5B, n.wrapping_sub(1));
            v_get_partner_sequence(this)
        }
        5 => {
            if rd::<u8>(this, 0x5D) == 0 {
                if partner_get_partner_state(this) >= 5 {
                    let w = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x47C), 0x14);
                    if !w.is_null() && v_task_type(w) == 0x390 && v_wander_type(w) == 0 {
                        let mut a = [0u32; 3];
                        let mut b = [0u32; 3];
                        let pa = placeable_get_top_direction(ped, slot(&mut a));
                        let pb = placeable_get_top_direction(ped, slot(&mut b));
                        let hy = f32_of(x(rd::<f32>(pb, 4)).fchs());
                        let hx = f32_of(x(rd::<f32>(pa, 0)).fchs());
                        let dir = general_get_node_heading_from_vector(hx.to_bits(), hy.to_bits()) & 0xFFFF;
                        let p = task_new(0x38);
                        let wt = if p.is_null() { GPtr::NULL } else { wander_standard_ctor(p, 4, dir, 1) };
                        wander_set_next_scan_time(wt, now().wrapping_add(d(0x86FCDC)));
                        tm_set_task(task_mgr(ped), wt, 4, 0);
                    }
                    wr::<u8>(this, 0x59, 6);
                }
                stand_still_50()
            } else if rd::<u8>(this, 0x5A) == 1 {
                if partner_get_partner_state(this) == 6 {
                    wr::<u8>(this, 0x59, 6);
                }
                stand_still_50()
            } else {
                partner_face_partner_step(this, ped)
            }
        }
        4 => {
            let s = partner_get_partner_state(this);
            if s == 4 || s == 5 {
                if rd::<u8>(this, 0x5A) == 0xFF {
                    wr::<u8>(this, 0x5A, 0);
                }
                let t = find_partner_task();
                if rd::<u8>(t, 0x5A) == 0xFF {
                    wr::<u8>(t, 0x5A, 1);
                }
                wr::<u8>(this, 0x59, 5);
            } else {
                if rd::<u8>(this, 0x5A) == 0xFF {
                    wr::<u8>(this, 0x5A, 1);
                }
                let t = find_partner_task();
                if rd::<u8>(t, 0x5A) == 0xFF {
                    wr::<u8>(t, 0x5A, 0);
                }
                copy_vec(t.byte_add(0x40), this.byte_add(0x4C));
            }
            stand_still_50()
        }
        3 => {
            if v_task_type(rd::<GPtr<u8>>(this, 8)) == 0x387 {
                let p = task_new(0x28);
                if p.is_null() {
                    return GPtr::NULL;
                }
                return turn_to_face_entity_ctor(p, rd::<GPtr<u8>>(this, 0x38), d(TURN_RATE), d(0x86FC80));
            }
            if v_task_type(rd::<GPtr<u8>>(this, 8)) != 0x398 {
                return GPtr::NULL;
            }
            wr::<u8>(this, 0x59, 4);
            stand_still_50()
        }
        2 => {
            if v_task_type(rd::<GPtr<u8>>(this, 8)) != 0xCB {
                return GPtr::NULL;
            }
            let zero = x(c(0x858B50));
            let x_is_zero = !x(rd::<f32>(this, 0x40)).fcom(zero).test_parity(0x44);
            if x_is_zero && !x(rd::<f32>(this, 0x44)).fcom(zero).test_parity(0x44) {
                return stand_still_50();
            }
            wr::<u8>(this, 0x59, 3);
            wr::<u16>(this, 0x70, 0);
            let p = task_new(0x28);
            if p.is_null() {
                return GPtr::NULL;
            }
            go_to_point_and_stand_still_ctor(p, 4, this.byte_add(0x40), 0x3DCC_CCCD, 0, 0, 1)
        }
        1 => {
            if v_task_type(rd::<GPtr<u8>>(this, 8)) != 0xCB {
                return GPtr::NULL;
            }
            if rd::<u8>(this, 0x58) != 0 {
                partner_compute_meeting_positions(this, ped, this.byte_add(0x40), this.byte_add(0x4C));
                let t = find_partner_task();
                copy_vec(t.byte_add(0x40), this.byte_add(0x4C));
            }
            wr::<u8>(this, 0x59, 2);
            stand_still_50()
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexPartner::RemoveStreamedAnims()` (0x682370): with the anims referenced (+0x6E) and a block name whose
/// first byte differs from the string at 0x858B54 (one-byte `repe cmpsb`), `RemoveAnimBlockRef(GetAnimationBlockIndex
/// (name))`; then +0x6E = 0.
#[hook(addr = 0x682370, conv = thiscall, name = "CTaskComplexPartner::RemoveStreamedAnims", expect = "55 8B E9 8A 45 6E")]
pub fn partner_remove_streamed_anims(this: GPtr<u8>) {
    if rd::<u8>(this, 0x6E) == 0 {
        return;
    }
    if rd::<u8>(this, 0x5E) != g::<u8>(0x858B54).read() {
        let i = anim_get_block_index(this.byte_add(0x5E));
        anim_remove_block_ref(i);
    }
    wr::<u8>(this, 0x6E, 0);
}

// ======================================================================================================
// CTaskComplexBeInCouple
// ======================================================================================================

/// `AbortArmIK(ped)` then null (0x684CE6 / 0x684CEB).
fn couple_fail(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    be_in_couple_abort_arm_ik(this, ped);
    GPtr::NULL
}

/// `CTaskComplexBeInCouple::ControlSubTask(CPed*)` (0x684930, `ret 4`). Fails (`AbortArmIK`, null) unless the
/// partner (+0x10) is alive and runs a couple task (0x4BF) of the opposite role whose partner is `ped`. A follower
/// sets its walk-alongside (0x4B8) side offset (+0x14 = (side == 2 ? -1.05 : 1.05, 0, 0)). Fails when the squared
/// 2D distance exceeds `m_giveUpDist`^2 or either active couple task is missing. A leader further than 2 m
/// (dist^2 > 4.0) aborts the arm IK and, unless it already has a wait-for-ped task (0x4BD), returns a new
/// `CTaskComplexWaitForPed(partner, 0.75f, 20000, false)` (null without aborting when the pool is full). Hand holding
/// (+0x15) points the arm of the partner's side at the midpoint (dist^2 < 2.25, else abort); looking at each other
/// (+0x16) starts a 2000..4000 ms look-at for each of the two peds that is not looking already, with a 20% roll
/// (`GetRandomNumberInRange(0, 100) > 80`, then the time draw). Returns the sub-task.
#[hook(addr = 0x684930, conv = thiscall, name = "CTaskComplexBeInCouple::ControlSubTask", expect = "6A FF 64 A1 00 00 00 00")]
pub fn be_in_couple_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let partner = || rd::<GPtr<u8>>(this, 0x10);
    if partner().is_null() || ped_is_alive(partner()) == 0 {
        return couple_fail(this, ped);
    }
    let t = intel_find_task_by_type(rd::<GPtr<u8>>(partner(), 0x47C), 0x4BF);
    let role_ok = if rd::<u8>(this, 0x14) != 0 {
        !t.is_null() && rd::<u8>(t, 0x14) == 0
    } else {
        !t.is_null() && rd::<u8>(t, 0x14) != 0
    };
    if !role_ok || rd::<GPtr<u8>>(t, 0x10) != ped {
        return couple_fail(this, ped);
    }
    let mine = tm_find_active_task_by_type(task_mgr(ped), 0x4BF);
    let theirs = tm_find_active_task_by_type(task_mgr(partner()), 0x4BF);
    let side = be_in_couple_get_side(this, ped);
    if rd::<u8>(this, 0x14) == 0 {
        let w = intel_find_task_by_type(rd::<GPtr<u8>>(ped, 0x47C), 0x4B8);
        if !w.is_null() {
            wr::<u32>(w, 0x14, if side == 2 { 0xBF86_6666 } else { 0x3F86_6666 });
            wr::<u32>(w, 0x18, 0);
            wr::<u32>(w, 0x1C, 0);
        }
    }
    let q = ent_pos(ped);
    let p = ent_pos(partner());
    let dy = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(q, 4));
    let dx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(q, 0));
    let dx_f = f32_of(dx);
    let dy_f = f32_of(dy);
    let dist_sq = f32_of(dy.fmul_m(dy_f).fadd(x(dx_f).fmul_m(dx_f)));
    let give_up = x(rd::<f32>(this, 0x18));
    if !x(dist_sq).fcom(give_up.fmul(give_up)).test_any(0x41) {
        return couple_fail(this, ped);
    }
    if mine.is_null() || theirs.is_null() {
        return couple_fail(this, ped);
    }
    if !x(dist_sq).fcom(x(c(0x858B90))).test_any(0x41) && rd::<u8>(this, 0x14) != 0 {
        be_in_couple_abort_arm_ik(this, ped);
        if intel_find_task_by_type(rd::<GPtr<u8>>(ped, 0x47C), 0x4BD).is_null() {
            let w = task_new(0x2C);
            if w.is_null() {
                return GPtr::NULL;
            }
            return wait_for_ped_ctor(w, partner(), 0x3F40_0000, 20000, 0);
        }
    }
    if rd::<u8>(this, 0x15) != 0 {
        let mut mid = [
            f32_of(x(dx_f).fmul_m(c(0x858B8C))),
            f32_of(x(dy_f).fmul_m(c(0x858B8C))),
            0.0f32,
        ];
        let mut hand = [0u32; 3];
        vector_add(slot(&mut hand), ent_pos(ped), slot(&mut mid));
        if x(dist_sq).fcom(x(c(0x86E104))).test_parity(0x5) {
            be_in_couple_abort_arm_ik(this, ped);
        } else {
            if side != i32::from(rd::<u8>(this, 0x1C)) {
                be_in_couple_abort_arm_ik(this, ped);
            }
            let arm = match side {
                2 => Some(0),
                1 => Some(1),
                _ => None,
            };
            if let Some(arm) = arm {
                ik_point_arm(
                    ik_man(),
                    0x870848,
                    arm,
                    ped,
                    GPtr::NULL,
                    -1,
                    slot(&mut hand),
                    0x3F00_0000,
                    0xFA,
                    0x41F0_0000,
                );
                wr::<u8>(this, 0x1C, side as u8);
            }
        }
    }
    if rd::<u8>(this, 0x16) != 0 {
        if ik_is_looking(ik_man(), ped) == 0 && general_random_int_in_range(0, 100) > 0x50 {
            let time = general_random_int_in_range(2000, 4000);
            ik_look_at(ik_man(), 0x870838, ped, partner(), time, 5, 0, 0, 0x3E80_0000, 0x1F4, 3, 0);
        }
        if ik_is_looking(ik_man(), partner()) == 0 && general_random_int_in_range(0, 100) > 0x50 {
            let time = general_random_int_in_range(2000, 4000);
            ik_look_at(ik_man(), 0x870838, partner(), ped, time, 5, 0, 0, 0x3E80_0000, 0x1F4, 3, 0);
        }
    }
    rd::<GPtr<u8>>(this, 8)
}

// ======================================================================================================
// CTaskComplexPartnerDeal / Greet / Chat / Shove
// ======================================================================================================

/// The anim block name "gangs" (6 bytes from 0x86F92C: a dword and a word) into +0x5E.
fn copy_gangs_block_name(this: GPtr<u8>) {
    wr::<u32>(this, 0x5E, d(0x86F92C));
    wr::<u16>(this, 0x62, g::<u16>(0x86F930).read());
}

/// `CTaskComplexPartnerDeal::Constructor(const char* name, CPed* partner, bool lead, float distance, CVector point (by
/// value))` (0x684190, `ret 0x1C`): `CTaskComplexPartner(name, partner, lead, distance, true, 1, point)` through
/// 0x681E70, vtable 0x870754, task id 0x4B2, anim block "gangs".
#[hook(addr = 0x684190, conv = thiscall, name = "CTaskComplexPartnerDeal::Constructor", expect = "8B 54 24 18 56")]
#[allow(clippy::too_many_arguments)]
pub fn partner_deal_ctor(
    this: GPtr<u8>,
    name: GPtr<u8>,
    partner: GPtr<u8>,
    lead: u32,
    dist: u32,
    vx: u32,
    vy: u32,
    vz: u32,
) -> GPtr<u8> {
    partner_ctor_orig(this, name, partner, lead, dist, 1, 1, vx, vy, vz);
    wr::<u32>(this, 0, 0x870754);
    wr::<u32>(this, 0x34, 0x4B2);
    copy_gangs_block_name(this);
    this
}

/// `CTaskComplexPartnerGreet::Constructor(const char* name, CPed* partner, bool lead, float distance, int32 greetType,
/// CVector point (by value))` (0x684210, `ret 0x20`): `CTaskComplexPartner(name, partner, lead, distance, true, 1,
/// point)`, +0x74 = greetType, vtable 0x87078C, task id 0x4B3, anim block "gangs".
#[hook(addr = 0x684210, conv = thiscall, name = "CTaskComplexPartnerGreet::Constructor", expect = "8B 54 24 1C 56")]
#[allow(clippy::too_many_arguments)]
pub fn partner_greet_ctor(
    this: GPtr<u8>,
    name: GPtr<u8>,
    partner: GPtr<u8>,
    lead: u32,
    dist: u32,
    kind: u32,
    vx: u32,
    vy: u32,
    vz: u32,
) -> GPtr<u8> {
    partner_ctor_orig(this, name, partner, lead, dist, 1, 1, vx, vy, vz);
    wr::<u32>(this, 0x74, kind);
    wr::<u32>(this, 0, 0x87078C);
    wr::<u32>(this, 0x34, 0x4B3);
    copy_gangs_block_name(this);
    this
}

/// `CTaskComplexPartnerShove::Constructor(const char* name, CPed* partner, bool lead, float distance, int8
/// updateDirection, CVector point (by value))` (0x6846F0, `ret 0x20`): `CTaskComplexPartner(name, partner, lead,
/// distance, false, updateDirection, point)`, +0x5B = updateDirection (byte), vtable 0x870800, task id 0x4B9.
#[hook(addr = 0x6846F0, conv = thiscall, name = "CTaskComplexPartnerShove::Constructor", expect = "8B 54 24 1C 53")]
#[allow(clippy::too_many_arguments)]
pub fn partner_shove_ctor(
    this: GPtr<u8>,
    name: GPtr<u8>,
    partner: GPtr<u8>,
    lead: u32,
    dist: u32,
    upd: u32,
    vx: u32,
    vy: u32,
    vz: u32,
) -> GPtr<u8> {
    partner_ctor_orig(this, name, partner, lead, dist, 0, upd, vx, vy, vz);
    wr::<u8>(this, 0x5B, upd as u8);
    wr::<u32>(this, 0, 0x870800);
    wr::<u32>(this, 0x34, 0x4B9);
    this
}

/// `CTaskComplexPartnerGreet::Clone()` (0x684DA0): `new(0x78)`, `CTaskComplexPartnerGreet(+0x14, partner, lead
/// (zero-extended byte), distance, +0x74, point)` through 0x684210, or null.
#[hook(addr = 0x684DA0, conv = thiscall, name = "CTaskComplexPartnerGreet::Clone", expect = "6A FF 68 8B 6A 84 00")]
pub fn partner_greet_clone(this: GPtr<u8>) -> GPtr<u8> {
    let p = task_new(0x78);
    if p.is_null() {
        return GPtr::NULL;
    }
    partner_greet_ctor_orig(
        p,
        this.byte_add(0x14),
        rd::<GPtr<u8>>(this, 0x38),
        u32::from(rd::<u8>(this, 0x58)),
        rd::<u32>(this, 0x3C),
        rd::<u32>(this, 0x74),
        rd::<u32>(this, 0x40),
        rd::<u32>(this, 0x44),
        rd::<u32>(this, 0x48),
    )
}

/// `CTaskComplexPartnerChat::Clone()` (0x684E30): `new(0x78)`, `CTaskComplexPartnerChat(+0x14, partner, lead
/// (zero-extended), distance, +0x5B (sign-extended), +0x74, +0x75 (zero-extended), point)` through 0x684290, or null.
#[hook(addr = 0x684E30, conv = thiscall, name = "CTaskComplexPartnerChat::Clone", expect = "6A FF 68 AB 6A 84 00")]
pub fn partner_chat_clone(this: GPtr<u8>) -> GPtr<u8> {
    let p = task_new(0x78);
    if p.is_null() {
        return GPtr::NULL;
    }
    partner_chat_ctor_orig(
        p,
        this.byte_add(0x14),
        rd::<GPtr<u8>>(this, 0x38),
        u32::from(rd::<u8>(this, 0x58)),
        rd::<u32>(this, 0x3C),
        i32::from(rd::<i8>(this, 0x5B)) as u32,
        u32::from(rd::<u8>(this, 0x74)),
        u32::from(rd::<u8>(this, 0x75)),
        rd::<u32>(this, 0x40),
        rd::<u32>(this, 0x44),
        rd::<u32>(this, 0x48),
    )
}

/// `CTaskComplexPartnerShove::Clone()` (0x684ED0): `new(0x74)`, `CTaskComplexPartnerShove(+0x14, partner, lead
/// (zero-extended), distance, +0x5B (sign-extended), point)` through 0x6846F0, or null.
#[hook(addr = 0x684ED0, conv = thiscall, name = "CTaskComplexPartnerShove::Clone", expect = "6A FF 68 CB 6A 84 00")]
pub fn partner_shove_clone(this: GPtr<u8>) -> GPtr<u8> {
    let p = task_new(0x74);
    if p.is_null() {
        return GPtr::NULL;
    }
    partner_shove_ctor_orig(
        p,
        this.byte_add(0x14),
        rd::<GPtr<u8>>(this, 0x38),
        u32::from(rd::<u8>(this, 0x58)),
        rd::<u32>(this, 0x3C),
        i32::from(rd::<i8>(this, 0x5B)) as u32,
        rd::<u32>(this, 0x40),
        rd::<u32>(this, 0x44),
        rd::<u32>(this, 0x48),
    )
}

/// `CTaskComplexPartnerGreet::StreamRequiredAnims()` (0x6825B0). Not yet referenced (+0x6E clear): when
/// `ShouldLoadGangAnims()`, a loaded block (`ms_aAnimBlocks[i].bLoaded`, byte 0xB5D4B0 + 0x20 i, == 1) is referenced
/// (+0x6E = 1), else `RequestModel(i + 0x63E7, 8)`. Referenced: when the gang anims are no longer wanted the block
/// is released and +0x6E cleared.
#[hook(addr = 0x6825B0, conv = thiscall, name = "CTaskComplexPartnerGreet::StreamRequiredAnims", expect = "56 8B F1 8A 46 6E")]
pub fn partner_greet_stream_required_anims(this: GPtr<u8>) {
    if rd::<u8>(this, 0x6E) == 0 {
        if should_load_gang_anims() == 0 {
            return;
        }
        let i = anim_get_block_index(this.byte_add(0x5E));
        let loaded = g::<u8>(((i as u32) << 5).wrapping_add(0xB5D4B0) as usize).read();
        if loaded != 1 {
            streaming_request_model(i.wrapping_add(0x63E7), 8);
            return;
        }
        anim_add_block_ref(i);
        wr::<u8>(this, 0x6E, 1);
    } else {
        if should_load_gang_anims() != 0 {
            return;
        }
        let i = anim_get_block_index(this.byte_add(0x5E));
        anim_remove_block_ref(i);
        wr::<u8>(this, 0x6E, 0);
    }
}

/// `CTaskComplexPartnerDeal::GetPartnerSequence()` (0x682440). A new sequence of a turn-to-face (partner,
/// [0x86FC7C], 0.02f) and a `CTaskSimpleRunAnim(0x34, lead ? 0x11F : 0x120, 4.0f, false)`; failed allocations add
/// null tasks.
#[hook(addr = 0x682440, conv = thiscall, name = "CTaskComplexPartnerDeal::GetPartnerSequence", expect = "6A FF 68 47 65 84 00")]
pub fn partner_deal_get_partner_sequence(this: GPtr<u8>) -> GPtr<u8> {
    let seq = new_sequence();
    let anim = if rd::<u8>(this, 0x58) != 0 { 0x11F } else { 0x120 };
    sequence_add_task(seq, new_turn_to_face(this, 0x3CA3_D70A));
    sequence_add_task(seq, new_run_anim(0x34, anim));
    seq
}

/// `CTaskComplexPartnerGreet::GetPartnerSequence()` (0x682630). A new sequence: turn-to-face (partner, [0x86FC7C],
/// 0.001f); the greeting anim of type +0x74 (0..5: `CTaskSimpleRunAnim(0x34, 0x121 / 0x122 / lead ? 0x123 : 0x124 /
/// 0x125 / 0x126 / 0x127)`, other types add nothing). Four times `3000 - _ftol(rand16 / 32768 * [0x87069C])`
/// (r1..r4, rand at 0x682870, 0x68289A, 0x6828C4, 0x6828F2) and `rand() < 0x3FFF` (0x682920). Lead: hand signal,
/// stand-still 1000, stand-still r1, timed anim r2; follower: stand-still 1000, hand signal, timed anim r1,
/// stand-still r2; then, on the last roll, stand-still r3 and `CTaskSimpleChat(r4)`.
#[hook(addr = 0x682630, conv = thiscall, name = "CTaskComplexPartnerGreet::GetPartnerSequence", expect = "6A FF 68 47 66 84 00")]
pub fn partner_greet_get_partner_sequence(this: GPtr<u8>) -> GPtr<u8> {
    let seq = new_sequence();
    sequence_add_task(seq, new_turn_to_face(this, 0x3A83_126F));
    let anim = match rd::<i32>(this, 0x74) {
        0 => Some(0x121),
        1 => Some(0x122),
        2 => Some(if rd::<u8>(this, 0x58) != 0 { 0x123 } else { 0x124 }),
        3 => Some(0x125),
        4 => Some(0x126),
        5 => Some(0x127),
        _ => None,
    };
    if let Some(anim) = anim {
        sequence_add_task(seq, new_run_anim(0x34, anim));
    }
    let r1 = 3000i32.wrapping_sub(rand_scaled(0x682870, 0x87069C));
    let r2 = 3000i32.wrapping_sub(rand_scaled(0x68289A, 0x87069C));
    let r3 = 3000i32.wrapping_sub(rand_scaled(0x6828C4, 0x87069C));
    let r4 = 3000i32.wrapping_sub(rand_scaled(0x6828F2, 0x87069C));
    let chat = at_site(0x682920, rand) < 0x3FFF;
    if rd::<u8>(this, 0x58) != 0 {
        sequence_add_task(seq, new_hand_signal());
        sequence_add_task(seq, new_stand_still(1000));
        sequence_add_task(seq, new_stand_still(r1));
        sequence_add_task(seq, new_timed_anim(r2 as u32));
    } else {
        sequence_add_task(seq, new_stand_still(1000));
        sequence_add_task(seq, new_hand_signal());
        sequence_add_task(seq, new_timed_anim(r1 as u32));
        sequence_add_task(seq, new_stand_still(r2));
    }
    if chat {
        sequence_add_task(seq, new_stand_still(r3));
        sequence_add_task(seq, new_simple_chat(r4 as u32));
    }
    seq
}

/// `CTaskComplexPartnerChat::GetPartnerSequence()` (0x684380). A new sequence: turn-to-face (partner, [0x86FC7C],
/// 0.001f); `h = _ftol(((int)+0x5B << 4) + p.z + p.y + p.x) & 0x7F` (point +0x40), `f = h * [0x858B88]`,
/// `t1 = _ftol(f * [0x859A94] + [0x8707FC])`, `t2 = _ftol((float)([0x858624] - f) * [0x859A94] + [0x8707FC])`. A
/// follower copies the partner chat task's +0x74 (no null check). With +0x74: the low words of t2 (lead) / t1
/// (follower) are replaced by the conversation words at 0xB613F8 / 0xB613FA - 4 * +0x5B, +0x5B is reset when
/// `3 - +0x5B == [0xB613E8]` (i16), and two `CTaskComplexChat(1 / 0, partner, +0x5B, t2)` (lead) or `(0 / 1, ...,
/// t1)` (follower) are added. Without: lead `CTaskSimpleChat(t1)`, stand-still t2; follower stand-still t1,
/// `CTaskSimpleChat(t2)`.
#[hook(addr = 0x684380, conv = thiscall, name = "CTaskComplexPartnerChat::GetPartnerSequence", expect = "6A FF 68 0E 6A 84 00")]
pub fn partner_chat_get_partner_sequence(this: GPtr<u8>) -> GPtr<u8> {
    let seq = new_sequence();
    sequence_add_task(seq, new_turn_to_face(this, 0x3A83_126F));
    let upd = i32::from(rd::<i8>(this, 0x5B)) << 4;
    let h = ftol32(
        xi(upd)
            .fadd_m(rd::<f32>(this, 0x48))
            .fadd_m(rd::<f32>(this, 0x44))
            .fadd_m(rd::<f32>(this, 0x40)),
    ) & 0x7F;
    let f = xi(h).fmul_m(c(0x858B88));
    let a = f32_of(x(c(0x858624)).fsub(f));
    let mut t1 = ftol32(f.fmul_m(c(0x859A94)).fadd_m(c(0x8707FC))) as u32;
    let mut t2 = ftol32(x(a).fmul_m(c(0x859A94)).fadd_m(c(0x8707FC))) as u32;
    if rd::<u8>(this, 0x58) == 0 {
        let pt = intel_find_task_by_type(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x38), 0x47C), 0x4B4);
        wr::<u8>(this, 0x74, rd::<u8>(pt, 0x74));
    }
    if rd::<u8>(this, 0x74) != 0 {
        let idx = i32::from(rd::<i8>(this, 0x5B));
        let off = idx.wrapping_mul(4) as u32;
        let w2 = g::<u16>(0xB613F8u32.wrapping_sub(off) as usize).read();
        let w1 = g::<u16>(0xB613FAu32.wrapping_sub(off) as usize).read();
        t2 = (t2 & 0xFFFF_0000) | u32::from(w2);
        t1 = (t1 & 0xFFFF_0000) | u32::from(w1);
        if 3i32.wrapping_sub(idx) == i32::from(g::<i16>(0xB613E8).read()) {
            wr::<u8>(this, 0x5B, 0);
        }
        let new_chat = |a: u32, time: u32| {
            let p = task_new(0x20);
            if p.is_null() {
                return GPtr::NULL;
            }
            complex_chat_ctor(p, a, rd::<GPtr<u8>>(this, 0x38), i32::from(rd::<i8>(this, 0x5B)), time)
        };
        if rd::<u8>(this, 0x58) != 0 {
            sequence_add_task(seq, new_chat(1, t2));
            sequence_add_task(seq, new_chat(0, t2));
        } else {
            sequence_add_task(seq, new_chat(0, t1));
            sequence_add_task(seq, new_chat(1, t1));
        }
    } else if rd::<u8>(this, 0x58) != 0 {
        sequence_add_task(seq, new_simple_chat(t1));
        sequence_add_task(seq, new_stand_still(t2 as i32));
    } else {
        sequence_add_task(seq, new_stand_still(t1 as i32));
        sequence_add_task(seq, new_simple_chat(t2));
    }
    seq
}

/// `CTaskComplexPartnerShove::GetPartnerSequence()` (0x683120). A new sequence of a turn-to-face (partner,
/// [0x86FC7C], 0.001f) and a `CTaskSimpleRunAnim(0, lead ? 0xA4 : 0x8E, 4.0f, false)`.
#[hook(addr = 0x683120, conv = thiscall, name = "CTaskComplexPartnerShove::GetPartnerSequence", expect = "6A FF 68 3C 67 84 00")]
pub fn partner_shove_get_partner_sequence(this: GPtr<u8>) -> GPtr<u8> {
    let seq = new_sequence();
    sequence_add_task(seq, new_turn_to_face(this, 0x3A83_126F));
    let anim = if rd::<u8>(this, 0x58) != 0 { 0xA4 } else { 0x8E };
    sequence_add_task(seq, new_run_anim(0, anim));
    seq
}

// ======================================================================================================
// CTaskComplexUsePairedAttractor / CTaskComplexAttractorPartnerWait / CTaskComplexUseAttractorPartner
// ======================================================================================================

/// `CTaskComplexUsePairedAttractor::SetPartnership(CPed* partner, bool lead, pair*)` (0x6331F0, `ret 0xC`): +0x10 =
/// partner (registered without a null check), +0x14 = lead (byte), +0x18 = pair.
#[hook(addr = 0x6331F0, conv = thiscall, name = "CTaskComplexUsePairedAttractor::SetPartnership", expect = "56 8B F1 8B 4C 24 08")]
pub fn paired_attractor_set_partnership(this: GPtr<u8>, partner: GPtr<u8>, lead: u32, pair: u32) {
    wr::<GPtr<u8>>(this, 0x10, partner);
    entity_register_reference(partner, this.byte_add(0x10));
    wr::<u8>(this, 0x14, lead as u8);
    wr::<u32>(this, 0x18, pair);
}

/// `CTaskComplexUsePairedAttractor::ClearPartnership()` (0x633220): the partner reference is cleaned up and cleared
/// when set; +0x14 = 0, +0x18 = 0.
#[hook(addr = 0x633220, conv = thiscall, name = "CTaskComplexUsePairedAttractor::ClearPartnership", expect = "53 56 8B F1 8B 4E 10")]
pub fn paired_attractor_clear_partnership(this: GPtr<u8>) {
    let partner = rd::<GPtr<u8>>(this, 0x10);
    if !partner.is_null() {
        entity_clean_up_old_reference(partner, this.byte_add(0x10));
        wr::<u32>(this, 0x10, 0);
    }
    wr::<u8>(this, 0x14, 0);
    wr::<u32>(this, 0x18, 0);
}

/// `CTaskComplexUsePairedAttractor::SeekPartnership(CPed*)` (0x638AE0, `ret 4`). The effect pairs of the attractor's
/// effect (+0xC, its +4); with a positive count, for i = 0..3 the pair `(+0x1C + i + 4) % 4` (signed) whose two
/// effect ids are not -1: the ped using the second effect (`0xC3AB00 + 0x40 * id`) that runs a paired-attractor task
/// (0xF6) without a partner, while this one has none either, becomes the partner (+0x1C = index; this task leads,
/// the other one follows with `ped`).
#[hook(addr = 0x638AE0, conv = thiscall, name = "CTaskComplexUsePairedAttractor::SeekPartnership", expect = "83 EC 0C 56 8B F1")]
pub fn paired_attractor_seek_partnership(this: GPtr<u8>, ped: GPtr<u8>) {
    let pairs = scripted_2d_effects_get_effect_pairs(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 4));
    if rd::<i32>(pairs, 0) <= 0 {
        return;
    }
    for i in 0..4i32 {
        let idx = rd::<i32>(this, 0x1C).wrapping_add(i).wrapping_add(4).wrapping_rem(4);
        let pair = pairs.byte_add(idx as isize * 0x24 + 4);
        if rd::<i32>(pair, 0) == -1 {
            continue;
        }
        let id = rd::<i32>(pair, 0x10);
        if id == -1 {
            continue;
        }
        let effect = ((id as u32) << 6).wrapping_add(0xC3AB00);
        let other = attractor_manager_get_ped_using_effect(get_ped_attractor_manager(), effect, 0);
        if other.is_null() {
            continue;
        }
        let t = tm_find_active_task_by_type(task_mgr(other), 0xF6);
        if t.is_null() || !rd::<GPtr<u8>>(t, 0x10).is_null() || !rd::<GPtr<u8>>(this, 0x10).is_null() {
            continue;
        }
        wr::<i32>(this, 0x1C, idx);
        paired_set_partnership(this, other, 1, pair);
        paired_set_partnership(t, ped, 0, pair);
    }
}

/// The shared `Clone` body of 0x636CF0 / 0x636D60: `new(0x14)`, `CTaskComplex`, +0x10 and +0xC (byte) copied (read
/// before the base constructor), the vtable, or null.
fn attractor_partner_clone(this: GPtr<u8>, vtbl: u32) -> GPtr<u8> {
    let p = task_new(0x14);
    if p.is_null() {
        return GPtr::NULL;
    }
    let lead = rd::<u8>(this, 0xC);
    let pair = rd::<u32>(this, 0x10);
    task_complex_ctor(p);
    wr::<u32>(p, 0x10, pair);
    wr::<u8>(p, 0xC, lead);
    wr::<u32>(p, 0, vtbl);
    p
}

/// `CTaskComplexAttractorPartnerWait::Clone()` (0x636CF0): vtable 0x86E284.
#[hook(addr = 0x636CF0, conv = thiscall, name = "CTaskComplexAttractorPartnerWait::Clone", expect = "6A FF 68 BB 0E 84 00")]
pub fn attractor_partner_wait_clone(this: GPtr<u8>) -> GPtr<u8> {
    attractor_partner_clone(this, 0x86E284)
}

/// `CTaskComplexUseAttractorPartner::Clone()` (0x636D60): vtable 0x86E2B0.
#[hook(addr = 0x636D60, conv = thiscall, name = "CTaskComplexUseAttractorPartner::Clone", expect = "6A FF 68 DB 0E 84 00")]
pub fn use_attractor_partner_clone(this: GPtr<u8>) -> GPtr<u8> {
    attractor_partner_clone(this, 0x86E2B0)
}

/// The shared `CreateFirstSubTask` body of 0x638F20 / 0x638FF0: the sequence index of the pair (+0x10) at `lead_off`
/// (lead, +0xC) or `follow_off`; a new inlined `CTaskComplexUseSequence(index)` (0x1C bytes, vtable 0x86E518, +0x10 =
/// 0, +0x14 = -1, +0x18 = 0) that increments the sequence's reference count (0xC178F0 + 0x40 * index, +0x3C), or
/// null.
fn attractor_partner_use_sequence(this: GPtr<u8>, lead_off: isize, follow_off: isize) -> GPtr<u8> {
    let pair = rd::<GPtr<u8>>(this, 0x10);
    let idx = if rd::<u8>(this, 0xC) != 0 { rd::<u32>(pair, lead_off) } else { rd::<u32>(pair, follow_off) };
    let p = task_new(0x1C);
    if p.is_null() {
        return GPtr::NULL;
    }
    task_complex_ctor(p);
    wr::<u32>(p, 0xC, idx);
    let s = GPtr::<u8>::from_addr((idx << 6).wrapping_add(0xC178F0) as usize);
    wr::<u32>(p, 0, 0x86E518);
    wr::<u32>(p, 0x10, 0);
    wr::<u32>(p, 0x14, 0xFFFF_FFFF);
    wr::<u32>(p, 0x18, 0);
    wr::<u32>(s, 0x3C, rd::<u32>(s, 0x3C).wrapping_add(1));
    p
}

/// `CTaskComplexAttractorPartnerWait::CreateFirstSubTask(CPed*)` (0x638F20, `ret 4`): sequence index pair +4 (lead)
/// or +0x14.
#[hook(addr = 0x638F20, conv = thiscall, name = "CTaskComplexAttractorPartnerWait::CreateFirstSubTask", expect = "6A FF 68 4B 12 84 00")]
pub fn attractor_partner_wait_create_first_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    attractor_partner_use_sequence(this, 4, 0x14)
}

/// `CTaskComplexUseAttractorPartner::CreateFirstSubTask(CPed*)` (0x638FF0, `ret 4`): sequence index pair +8 (lead)
/// or +0x18.
#[hook(addr = 0x638FF0, conv = thiscall, name = "CTaskComplexUseAttractorPartner::CreateFirstSubTask", expect = "6A FF 68 6B 12 84 00")]
pub fn use_attractor_partner_create_first_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    attractor_partner_use_sequence(this, 8, 0x18)
}

// ======================================================================================================
// Wander tasks
// ======================================================================================================

/// `CTaskComplexWanderFlee::Destructor` (0x65B370): `vtbl = 0x86F690`, then the `CTaskComplexWander` destructor
/// 0x669D20 (tail jump).
#[hook(addr = 0x65B370, conv = thiscall, name = "CTaskComplexWanderFlee::Destructor", expect = "C7 01 90 F6 86 00")]
pub fn wander_flee_dtor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86F690);
    crate::dtors::dtor_669d20(this);
}

/// `(dz*dz + dy*dy) + dx*dx` from the position of path node (`area` word at `this+off`, node word at `this+off+2`)
/// to the ped, or None when the area is 0xFFFF or not loaded (`ThePaths.m_pPathNodes`, 0x96F854).
fn node_dist_sq(this: GPtr<u8>, off: isize, ped: GPtr<u8>) -> Option<X87> {
    let area = rd::<u16>(this, off);
    if area == 0xFFFF {
        return None;
    }
    let nodes = g::<u32>(0x96F854 + usize::from(area) * 4).read();
    if nodes == 0 {
        return None;
    }
    let pp = ent_pos(ped);
    let node = u32::from(rd::<u16>(this, off + 2)).wrapping_mul(0x1C).wrapping_add(nodes);
    let mut out = [0u32; 3];
    let np = path_node_get_position(GPtr::from_addr(node as usize), slot(&mut out));
    let dx = x(rd::<f32>(np, 0)).fsub_m(rd::<f32>(pp, 0));
    let dy = x(rd::<f32>(np, 4)).fsub_m(rd::<f32>(pp, 4));
    let dz = x(rd::<f32>(np, 8)).fsub_m(rd::<f32>(pp, 8));
    Some(dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)))
}

/// `CTaskComplexWander::GetDistSqOfClosestPathNodeToPed(CPed*)` (0x669FF0, `ret 4`, ST0). `dNext` (node +0x1C/+0x1E,
/// stored f32, 999999.0f without a node), `dLast` (node +0x18/+0x1A, kept on the x87 stack, [0x86FEC0] without a
/// node); returns dLast only when strictly smaller (`test ah,5; jnp`), else dNext.
#[hook(addr = 0x669FF0, conv = thiscall, name = "CTaskComplexWander::GetDistSqOfClosestPathNodeToPed", expect = "83 EC 0C 53 8B 5C 24 14")]
pub fn wander_get_dist_sq_of_closest_path_node_to_ped(this: GPtr<u8>, ped: GPtr<u8>) -> X87 {
    let next = node_dist_sq(this, 0x1C, ped).map_or(f32::from_bits(0x4974_23F0), f32_of);
    let last = node_dist_sq(this, 0x18, ped).unwrap_or_else(|| x(c(0x86FEC0)));
    if !last.fcom(x(next)).test_parity(0x5) {
        last
    } else {
        x(next)
    }
}

/// `CTaskComplexAvoidOtherPedWhileWandering::Clone()` (0x66D050): `new(0x60)`, `Constructor(+0xC, &+0x1C, +0x58)`
/// through 0x66A100; flag bit 2 of +0x5C is copied by OR (also into a null result when the pool is full: original
/// behaviour, kept).
#[hook(addr = 0x66D050, conv = thiscall, name = "CTaskComplexAvoidOtherPedWhileWandering::Clone", expect = "6A FF 68 0B 54 84 00")]
pub fn avoid_other_ped_while_wandering_clone(this: GPtr<u8>) -> GPtr<u8> {
    let p = task_new(0x60);
    let r = if p.is_null() {
        GPtr::NULL
    } else {
        avoid_other_ped_while_wandering_ctor(p, rd::<GPtr<u8>>(this, 0xC), this.byte_add(0x1C), rd::<u32>(this, 0x58))
    };
    if rd::<u8>(this, 0x5C) & 4 != 0 {
        wr::<u8>(r, 0x5C, rd::<u8>(r, 0x5C) | 4);
    }
    r
}

/// `CTaskComplexWanderCriminal::LookForCarsToSteal(CPed*)` (0x66B4F0, `ret 4`). The closest (strictly, `(dz*dz +
/// dy*dy) + dx*dx` against the stored f32 minimum from FLT_MAX) stealable vehicle (`IsVehicleStealable(v, ped)`) of
/// the ped's vehicle scanner (intel +0xE0, 16 slots). With one: +0x34 = now + 30000; `rand()` (0x66B5C7, unmasked)
/// * [0x858C7C] < [0x858CC4] adds a `CEventVehicleToSteal(vehicle)` (Rust local) to the ped's event group.
#[hook(addr = 0x66B4F0, conv = thiscall, name = "CTaskComplexWanderCriminal::LookForCarsToSteal", expect = "6A FF 68 F8 50 84 00")]
pub fn wander_criminal_look_for_cars_to_steal_port(this: GPtr<u8>, ped: GPtr<u8>) {
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let mut best = GPtr::<u8>::NULL;
    let mut min_d = f32::MAX;
    for i in 0..16isize {
        let v = rd::<GPtr<u8>>(intel, 0xE0 + i * 4);
        if v.is_null() || car_enter_exit_is_vehicle_stealable(v, ped) == 0 {
            continue;
        }
        let vp = ent_pos(v);
        let pp = ent_pos(ped);
        let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(vp, 0));
        let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(vp, 4));
        let dz = x(rd::<f32>(pp, 8)).fsub_m(rd::<f32>(vp, 8));
        let dist = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        if !dist.fcom(x(min_d)).test_parity(0x5) {
            min_d = f32_of(dist);
            best = v;
        }
    }
    if best.is_null() {
        return;
    }
    wr::<u32>(this, 0x34, now().wrapping_add(30000));
    let r = at_site(0x66B5C7, rand);
    if xi(r).fmul_m(c(0x858C7C)).fcom(x(c(0x858CC4))).test_parity(0x5) {
        return;
    }
    let mut ev = [0u32; 8];
    let ep = slot(&mut ev);
    event_vehicle_to_steal_ctor(ep, best);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ep, 0);
    event_vehicle_to_steal_dtor(ep);
}

/// `CTaskComplexWanderCriminal::ScanForStuff(CPed*)` (0x670350, `ret 4`). An unstarted timer (+0x28 start, +0x2C
/// interval, +0x30 started, +0x31 stopped) starts with 50 ms and +0x34 = now - `_ftol(rand16 / 32768 * [0x870110])`
/// (rand at 0x67037A); a stopped one restarts at now. When the interval has passed (unsigned) the timer restarts
/// (50 ms) and, once now >= +0x34 (unsigned), `LookForCarsToSteal(ped)` runs.
#[hook(addr = 0x670350, conv = thiscall, name = "CTaskComplexWanderCriminal::ScanForStuff", expect = "51 56 8B F1 8A 46 30")]
pub fn wander_criminal_scan_for_stuff(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(this, 0x30) == 0 {
        wr::<u32>(this, 0x28, now());
        wr::<u32>(this, 0x2C, 0x32);
        wr::<u8>(this, 0x30, 1);
        let t0 = now();
        let k = rand_scaled(0x67037A, 0x870110);
        wr::<u32>(this, 0x34, t0.wrapping_sub(k as u32));
        if rd::<u8>(this, 0x30) == 0 {
            return;
        }
    }
    if rd::<u8>(this, 0x31) != 0 {
        wr::<u32>(this, 0x28, now());
        wr::<u8>(this, 0x31, 0);
    }
    let end = rd::<u32>(this, 0x2C).wrapping_add(rd::<u32>(this, 0x28));
    let t = now();
    if t < end {
        return;
    }
    wr::<u32>(this, 0x28, t);
    wr::<u32>(this, 0x2C, 0x32);
    wr::<u8>(this, 0x30, 1);
    if now() < rd::<u32>(this, 0x34) {
        return;
    }
    wander_criminal_look_for_cars_to_steal(this, ped);
}

/// `CTaskComplexWanderProstitute::ScanForStuff(CPed*)` (0x672700, `ret 4`). `CTaskComplexWanderStandard::ScanForStuff`,
/// then every 2000 ms (now > +0x38, unsigned): no cop (ped type 6) among the ped scanner's 16 entities; for each
/// player ped (+0x480) in a vehicle (+0x58C) whose last prostitute (player data +0xA8) is not `ped`: `r =
/// _ftol(rand16 / 32768 * [0x870184])` (rand at 0x6727AD), and when the vehicle's monetary value (handling +0xD8)
/// > r (unsigned), its speed^2 * 50^2 (`(sx*sx + sy*sy) + sz*sz`) < [0x86F848] and `IsTaskValid(ped, client)`, a
/// `CEventAcquaintancePed(client)` (Rust local) with vtable 0x86CA38 and task id 0x51D (+0xE) is added to the
/// ped's event group (the loop continues).
#[hook(addr = 0x672700, conv = thiscall, name = "CTaskComplexWanderProstitute::ScanForStuff", expect = "64 A1 00 00 00 00")]
pub fn wander_prostitute_scan_for_stuff(this: GPtr<u8>, ped: GPtr<u8>) {
    wander_standard_scan_for_stuff(this, ped);
    let t = now();
    if t <= rd::<u32>(this, 0x38) {
        return;
    }
    wr::<u32>(this, 0x38, t.wrapping_add(2000));
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let ents = intel_get_ped_entities(intel);
    for i in 0..16isize {
        let e = rd::<GPtr<u8>>(ents, i * 4);
        if !e.is_null() && rd::<i32>(e, 0x598) == 6 {
            return;
        }
    }
    for j in 0..16isize {
        let e = rd::<GPtr<u8>>(ents, j * 4);
        if e.is_null() {
            continue;
        }
        let pd = rd::<GPtr<u8>>(e, 0x480);
        if pd.is_null() || rd::<GPtr<u8>>(e, 0x58C).is_null() || rd::<GPtr<u8>>(pd, 0xA8) == ped {
            continue;
        }
        let r = rand_scaled(0x6727AD, 0x870184) as u32;
        let veh = rd::<GPtr<u8>>(e, 0x58C);
        if rd::<u32>(rd::<GPtr<u8>>(veh, 0x384), 0xD8) <= r {
            continue;
        }
        let k = c(0x858B40);
        let sz = x(rd::<f32>(veh, 0x4C)).fmul_m(k);
        let sy = x(rd::<f32>(veh, 0x48)).fmul_m(k);
        let sx = x(rd::<f32>(veh, 0x44)).fmul_m(k);
        let s = sx.fmul(sx).fadd(sy.fmul(sy)).fadd(sz.fmul(sz));
        if s.fcom(x(c(0x86F848))).test_parity(0x5) {
            continue;
        }
        if prostitute_solicit_is_task_valid(ped, e) == 0 {
            continue;
        }
        let mut ev = [0u32; 8];
        let ep = slot(&mut ev);
        event_acquaintance_ped_ctor(ep, e);
        wr::<u32>(ep, 0, 0x86CA38);
        wr::<u16>(ep, 0xE, 0x51D);
        event_group_add(intel.byte_add(0x68), ep, 0);
        wr::<u32>(ep, 0, 0x86CA38);
        event_acquaintance_ped_dtor(ep);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_PARTNER_CREATE_FIRST_SUB_TASK,
    HOOK_PARTNER_CREATE_NEXT_SUB_TASK,
    HOOK_PARTNER_REMOVE_STREAMED_ANIMS,
    HOOK_BE_IN_COUPLE_CONTROL_SUB_TASK,
    HOOK_PARTNER_DEAL_CTOR,
    HOOK_PARTNER_GREET_CTOR,
    HOOK_PARTNER_SHOVE_CTOR,
    HOOK_PARTNER_GREET_CLONE,
    HOOK_PARTNER_CHAT_CLONE,
    HOOK_PARTNER_SHOVE_CLONE,
    HOOK_PARTNER_GREET_STREAM_REQUIRED_ANIMS,
    HOOK_PARTNER_DEAL_GET_PARTNER_SEQUENCE,
    HOOK_PARTNER_GREET_GET_PARTNER_SEQUENCE,
    HOOK_PARTNER_CHAT_GET_PARTNER_SEQUENCE,
    HOOK_PARTNER_SHOVE_GET_PARTNER_SEQUENCE,
    HOOK_PAIRED_ATTRACTOR_SET_PARTNERSHIP,
    HOOK_PAIRED_ATTRACTOR_CLEAR_PARTNERSHIP,
    HOOK_PAIRED_ATTRACTOR_SEEK_PARTNERSHIP,
    HOOK_ATTRACTOR_PARTNER_WAIT_CLONE,
    HOOK_USE_ATTRACTOR_PARTNER_CLONE,
    HOOK_ATTRACTOR_PARTNER_WAIT_CREATE_FIRST_SUB_TASK,
    HOOK_USE_ATTRACTOR_PARTNER_CREATE_FIRST_SUB_TASK,
    HOOK_WANDER_FLEE_DTOR,
    HOOK_WANDER_GET_DIST_SQ_OF_CLOSEST_PATH_NODE_TO_PED,
    HOOK_AVOID_OTHER_PED_WHILE_WANDERING_CLONE,
    HOOK_WANDER_CRIMINAL_LOOK_FOR_CARS_TO_STEAL_PORT,
    HOOK_WANDER_CRIMINAL_SCAN_FOR_STUFF,
    HOOK_WANDER_PROSTITUTE_SCAN_FOR_STUFF,
];
