//! Gang and partner tasks (PORT-AI-5, helper B): `CTaskComplexGangLeader`, `CTaskGangHasslePed`,
//! `CTaskComplexSignalAtPed`, `CTaskComplexGangFollower`, `CTaskComplexPartner`, `CTaskComplexPartnerChat`,
//! `CTaskComplexBeInCouple`. Ported from the 1.0 US bytes (the exe wins over gta-reversed and the specs in
//! `docs/re/specs/AI/`). Every call to another catalogued function goes through `orig!` at the original address
//! (the locomotion tasks these create are constructed through their original constructors); tasks come from the
//! original task pool. Every `rand()` names its original return address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `g_ikChainMan`
const IK_CHAIN_MAN: usize = 0xC15448;
/// `"gangs"` (anim block name)
const STR_GANGS: u32 = 0x86F92C;
/// `CAnimManager::ms_aAnimBlocks` +0x10 (`bLoaded`), stride 0x20.
const ANIM_BLOCK_LOADED: u32 = 0xB5D4B0;

/// An f32 of the image (`.rdata` constant or `.data` tunable), read when the original reads it.
fn c(addr: usize) -> f32 {
    g::<f32>(addr).read()
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

/// `(float)(rand() & 0xFFFF) * 3.0517578e-05f [0x858B14] * k` (fild, fmul, fmul) then `_ftol`.
fn rand_scaled(site: u32, k_addr: usize) -> i32 {
    let r = at_site(site, rand) & 0xFFFF;
    ftol32(xi(r).fmul_m(c(0x858B14)).fmul_m(c(k_addr)))
}

orig! {
    // ---- virtual slots ----
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CreateNextSubTask(CPed*)` (vtable +0x20)
    fn v_create_next_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x20 => unported;
    /// `CTaskComplexPartner::StreamRequiredAnims()` (vtable +0x2C)
    fn v_partner_stream_required_anims(t: GPtr<u8>) = thiscall vslot 0x2C => unported;

    // ---- this module's own functions (called through the original address) ----
    /// `CTaskComplexGangLeader::ShouldLoadGangAnims()` (0x65E7F0, static)
    fn should_load_gang_anims() -> u8 = cdecl 0x65E7F0 => unported;
    /// `CTaskComplexGangLeader::DoGangAbuseSpeech(CPed*, CPed*)` (0x65E860, cdecl)
    fn do_gang_abuse_speech_orig(talker: GPtr<u8>, target: GPtr<u8>) = cdecl 0x65E860 => unported;
    /// `CTaskComplexPartner::Constructor` (0x681E70, `ret 0x24`)
    fn partner_ctor_orig(t: GPtr<u8>, name: GPtr<u8>, partner: GPtr<u8>, lead: u32, dist: u32, face: u32, upd: u32, vx: u32, vy: u32, vz: u32) -> GPtr<u8> = thiscall 0x681E70 => unported;

    // ---- task bases and constructors ----
    /// `CTaskSimple::CTaskSimple` (0x61A390)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskComplexSeekEntityStandard::Constructor(CEntity*, int32, int32, float, float, float, bool, bool)`
    /// (0x46AC10, `ret 0x20`)
    fn seek_entity_standard_ctor(t: GPtr<u8>, e: GPtr<u8>, a: i32, b: i32, r: u32, d1: u32, d2: u32, c1: u32, c2: u32) -> GPtr<u8> = thiscall 0x46AC10 => unported;
    /// `CTaskComplexTurnToFaceEntityOrCoord::Constructor(CEntity*, float, float)` (0x66B890, `ret 0xC`)
    fn turn_to_face_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x66B890 => unported;
    /// `CTaskSimpleStandStill::Constructor(int32 time, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, time: i32, a: u32, b: u32, f: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskSimpleRunAnim::Constructor(AssocGroupId, AnimationId, float, bool)` (0x61A8B0, `ret 0x10`)
    fn run_anim_ctor(t: GPtr<u8>, group: u32, anim: i32, delta: u32, b: u32) -> GPtr<u8> = thiscall 0x61A8B0 => unported;
    /// `CTaskComplexWanderStandard` constructor `(int32 moveState, uint8 dir, bool)` (0x48E4F0, `ret 0xC`)
    fn wander_standard_ctor(t: GPtr<u8>, ms: i32, dir: i32, b: u32) -> GPtr<u8> = thiscall 0x48E4F0 => unported;
    /// `CTaskSimpleCarDrive::Constructor(CVehicle*, CTaskUtilityLineUpPedWithCar*, bool)` (0x63C340, `ret 0xC`)
    fn simple_car_drive_ctor(t: GPtr<u8>, v: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x63C340 => unported;
    /// `CTaskComplexLeaveCar` constructor `(CVehicle*, int32, int32, bool, bool)` (0x63B8C0, `ret 0x14`)
    fn leave_car_ctor(t: GPtr<u8>, v: GPtr<u8>, a: i32, b: i32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
    /// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorXYOffset>::Constructor(CEntity*, int32, int32, float, float,
    /// float, bool, bool)` (0x661DC0, `ret 0x20`)
    fn seek_entity_xy_offset_ctor(t: GPtr<u8>, e: GPtr<u8>, a: i32, b: i32, r: u32, d1: u32, d2: u32, c1: u32, c2: u32) -> GPtr<u8> = thiscall 0x661DC0 => unported;
    /// `CTaskComplexKillPedOnFoot::Constructor(CPed*, int32, uint32, uint32, int32, bool)` (0x620E30, `ret 0x18`)
    fn kill_ped_on_foot_ctor(t: GPtr<u8>, p: GPtr<u8>, a: i32, b: u32, c: u32, d: i32, e: u32) -> GPtr<u8> = thiscall 0x620E30 => unported;
    /// `CTaskComplexPlayHandSignalAnim::Constructor(AnimationId, float)` (0x61B2B0, `ret 8`)
    fn play_hand_signal_anim_ctor(t: GPtr<u8>, anim: i32, delta: u32) -> GPtr<u8> = thiscall 0x61B2B0 => unported;
    /// `CTaskComplexWanderGang` constructor `(int32 moveState, uint8 dir, int32 time, bool, float)` (0x66F5C0, `ret 0x14`)
    fn wander_gang_ctor(t: GPtr<u8>, ms: i32, dir: i32, time: i32, b: u32, f: u32) -> GPtr<u8> = thiscall 0x66F5C0 => unported;
    /// `CTaskComplexWalkAlongsidePed::Constructor(CPed*, float)` (0x683240, `ret 8`)
    fn walk_alongside_ped_ctor(t: GPtr<u8>, p: GPtr<u8>, f: u32) -> GPtr<u8> = thiscall 0x683240 => unported;

    // ---- other catalogued functions ----
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn general_random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `FUN_0065e710(CPed*)` (0x65E710, cdecl; a gang ambient anim id)
    fn gang_anim_65e710(ped: GPtr<u8>) -> i32 = cdecl 0x65E710 => unported;
    /// `CPedGroupMembership::CountMembers()` (0x5F6A50)
    fn membership_count_members(m: GPtr<u8>) -> i32 = thiscall 0x5F6A50 => unported;
    /// `CPedGroupMembership::GetMember(int32)` (0x5F69B0, `ret 4`)
    fn membership_get_member(m: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x5F69B0 => unported;
    /// `CStreaming::IsVeryBusy()` (0x4076A0)
    fn streaming_is_very_busy() -> u8 = cdecl 0x4076A0 => unported;
    /// `CPed::Say(uint16, uint32, float, uint8, uint8, uint8)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, delay: u32, prob: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::StopPlayingHandSignal()` (0x5E0480)
    fn ped_stop_playing_hand_signal(p: GPtr<u8>) = thiscall 0x5E0480 => unported;
    /// `CPed::DropEntityThatThisPedIsHolding(bool)` (0x5E0360, `ret 4`)
    fn ped_drop_held_entity(p: GPtr<u8>, b: u32) = thiscall 0x5E0360 => unported;
    /// `IKChainManager_c::LookAt(name, CPed*, CEntity*, int32 time, int32 bone, RwV3d*, bool, float, int32, int32, bool)`
    /// (0x618970, `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: u32, b: u32, speed: u32, blend: i32, prio: i32, b2: u32) = thiscall 0x618970 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::AbortLookAt(CPed*, int32 blendOutTime)` (0x618280, `ret 8`)
    fn ik_abort_look_at(m: GPtr<u8>, ped: GPtr<u8>, t: i32) = thiscall 0x618280 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810, `ret 4`)
    fn tm_get_task_secondary(tm: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskManager::SetTaskSecondary(CTask*, int32)` (0x681B60, `ret 8`)
    fn tm_set_task_secondary(tm: GPtr<u8>, t: GPtr<u8>, i: i32) = thiscall 0x681B60 => unported;
    /// `CPed::IsPlayingHandSignal()` (0x5E0460)
    fn ped_is_playing_hand_signal(p: GPtr<u8>) -> u8 = thiscall 0x5E0460 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990)
    fn anim_get_block_index(name: u32) -> i32 = cdecl 0x4D3990 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0)
    fn anim_add_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    fn anim_remove_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CStreaming::RequestModel(int32 id, int32 flags)` (0x4087E0)
    fn streaming_request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, float in ST0)
    fn general_radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, float in ST0)
    fn general_limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CVector::FromMultiply3x3(const CMatrix&, const CVector&)` (0x59C6D0, `ret 8`)
    fn vector_from_multiply_3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) = thiscall 0x59C6D0 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn world_find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTaskComplexBeInCouple::AbortArmIK(CPed*)` (0x6837F0, `ret 4`)
    fn be_in_couple_abort_arm_ik(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x6837F0 => unported;
}

/// `fstp dword` of an f64 an original callee returned in ST0.
fn st0_f32(v: f64) -> f32 {
    X87::from_f64_exact(v).to_f32()
}

// ======================================================================================================
// CTaskComplexGangLeader
// ======================================================================================================

/// `CTaskComplexGangLeader::CreateFirstSubTask(CPed*)` (0x65E1F0): `mov eax, [ecx]; jmp [eax+0x20]`, a tail call of
/// the virtual `CreateNextSubTask(ped)`.
#[hook(addr = 0x65E1F0, conv = thiscall, name = "CTaskComplexGangLeader::CreateFirstSubTask", expect = "8B 01 FF 60 20")]
pub fn gang_leader_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    v_create_next_sub_task(this, ped)
}

/// `CTaskComplexGangLeader::CreateNextSubTask(CPed*)` (0x65DFF0, `ret 4`). `n = CountMembers()` of the group
/// (+0xC, membership at +8). After a 0xCB or (second `GetTaskType`) 0x1AA sub-task: member `_ftol(n * ((rand() &
/// 0xFFFF) * 3.0517578e-05f))` (`rand()` 0x65E049); when it exists, a new `CTaskComplexTurnToFaceEntityOrCoord(member,
/// [0x86FC7C], [0x86FC80])`. Otherwise `p = _ftol((rand() & 0xFFFF) * 3.0517578e-05f * 100.0f)` (0x65E0C8): n >= 3 and p
/// <= 95 -> `CTaskSimpleStandStill(5000, false, false, 8.0f)`; else the wander timer (+0x10 = now, +0x14 = 15000 -
/// `_ftol(r * -15000.0f)` with `rand()` 0x65E13C, +0x18 = 1) and `CTaskComplexWanderGang(4, _ftol(r * 8.0f), 5000,
/// true, 0.05f)` (`rand()` 0x65E191 only after a successful allocation).
#[hook(addr = 0x65DFF0, conv = thiscall, name = "CTaskComplexGangLeader::CreateNextSubTask", expect = "6A FF 68 91 43 84 00")]
pub fn gang_leader_create_next_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    let membership = || rd::<GPtr<u8>>(this, 0xC).byte_add(8);
    let n = membership_count_members(membership());
    let sub = || rd::<GPtr<u8>>(this, 8);
    if !sub().is_null() && (v_task_type(sub()) == 0xCB || v_task_type(sub()) == 0x1AA) {
        let r = at_site(0x65E049, rand) & 0xFFFF;
        let rk = xi(r).fmul_m(c(0x858B14));
        let idx = ftol32(xi(n).fmul(rk));
        let m = membership_get_member(membership(), idx);
        if !m.is_null() {
            let mem = task_new(0x28);
            if mem.is_null() {
                return GPtr::NULL;
            }
            let b = g::<u32>(0x86FC80).read();
            let a = g::<u32>(0x86FC7C).read();
            return turn_to_face_entity_ctor(mem, m, a, b);
        }
    }
    let p = rand_scaled(0x65E0C8, 0x858628);
    if n >= 3 && p <= 0x5F {
        let mem = task_new(0x20);
        if mem.is_null() {
            return GPtr::NULL;
        }
        return stand_still_ctor(mem, 0x1388, 0, 0, 0x4100_0000);
    }
    let k = rand_scaled(0x65E13C, 0x86F934);
    let now = g::<u32>(TIME_MS).read();
    wr::<u32>(this, 0x10, now);
    wr::<i32>(this, 0x14, 0x3A98i32.wrapping_sub(k));
    wr::<u8>(this, 0x18, 1);
    let mem = task_new(0x38);
    if mem.is_null() {
        return GPtr::NULL;
    }
    let dir = rand_scaled(0x65E191, 0x859000);
    wander_gang_ctor(mem, 4, dir, 0x1388, 1, 0x3D4C_CCCD)
}

/// `CTaskComplexGangLeader::GetRandomGangAmbientAnim(CPed*, CEntity* held)` (0x65E730, cdecl). Holding the model
/// [0x8CD728]: `rand()` (0x65E750) < 0x3FFF -> 0x12D for ped models 0x67 / 0x69, else 0x12B; otherwise 0x117 +
/// `GetRandomNumberInRange(0, 4)`. Holding the model [0x8CD72C]: `rand()` (0x65E796) < 0x3FFF -> `FUN_0065e710(ped)`,
/// else 0x117 + `GetRandomNumberInRange(0, 8)`. Anything else (or nothing held): 0x117 - `_ftol((rand() & 0xFFFF) *
/// 3.0517578e-05f * -8.0f)` (`rand()` 0x65E7C2; the original stores the masked value in its dead `held` slot).
#[hook(addr = 0x65E730, conv = cdecl, name = "CTaskComplexGangLeader::GetRandomGangAmbientAnim", expect = "8B 44 24 08 85 C0")]
pub fn gang_leader_get_random_gang_ambient_anim(ped: GPtr<u8>, held: GPtr<u8>) -> i32 {
    if !held.is_null() {
        let mi = i32::from(rd::<i16>(held, 0x22));
        if mi == i32::from(g::<u16>(0x8CD728).read()) {
            if at_site(0x65E750, rand) < 0x3FFF {
                let pm = i32::from(rd::<i16>(ped, 0x22));
                return if pm == 0x67 || pm == 0x69 { 0x12D } else { 0x12B };
            }
            return general_random_int_in_range(0, 4).wrapping_add(0x117);
        }
        if mi == i32::from(g::<u16>(0x8CD72C).read()) {
            if at_site(0x65E796, rand) < 0x3FFF {
                return gang_anim_65e710(ped);
            }
            return general_random_int_in_range(0, 8).wrapping_add(0x117);
        }
    }
    0x117i32.wrapping_sub(rand_scaled(0x65E7C2, 0x85B320))
}

/// `CTaskComplexGangLeader::ShouldLoadGangAnims()` (0x65E7F0, static; `nop; nop; jmp 0x40142C`, the island loads
/// `CWorld::PlayerInFocus`). False when the focus player is in a vehicle (+0x46C bit 8, +0x58C) whose squared move
/// speed is above 0.04 [0x863244] (`test ah, 0x41; je`), or when `CStreaming::IsVeryBusy()`; else true. Returns AL
/// (upper EAX bits are left over in the original).
#[hook(addr = 0x65E7F0, conv = cdecl, name = "CTaskComplexGangLeader::ShouldLoadGangAnims", expect = "90 90 E9 35 2C DA FF")]
pub fn gang_leader_should_load_gang_anims() -> u8 {
    let focus = u32::from(g::<u8>(0xB7CD74).read());
    let ped = g::<GPtr<u8>>(focus.wrapping_mul(0x190).wrapping_add(0xB7CD98) as usize).read();
    let flags = rd::<u32>(ped, 0x46C);
    let v = rd::<GPtr<u8>>(ped, 0x58C);
    if flags & 0x100 != 0 && !v.is_null() {
        let vz = x(rd::<f32>(v, 0x4C));
        let vy = x(rd::<f32>(v, 0x48));
        let vx = x(rd::<f32>(v, 0x44));
        let sum = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz));
        if !sum.fcom(x(c(0x863244))).test_any(0x41) {
            return 0;
        }
    }
    if streaming_is_very_busy() != 0 { 0 } else { 1 }
}

/// `CTaskComplexGangLeader::DoGangAbuseSpeech(CPed* talker, CPed* target)` (0x65E860, cdecl). Only for a gang
/// talker (ped type 7..16) and a gang or player target: `talker->Say(id, 0, 1.0f, 0, 0, 0)` with id 1 for target
/// type 7, 4 for type 8 or a player, 7 / 8 / 6 / 5 / 3 for types 0xA..0xE (none otherwise). The target's type is
/// re-read before each compare.
#[hook(addr = 0x65E860, conv = cdecl, name = "CTaskComplexGangLeader::DoGangAbuseSpeech", expect = "56 8B 74 24 08")]
pub fn gang_leader_do_gang_abuse_speech(talker: GPtr<u8>, target: GPtr<u8>) {
    let tt = rd::<i32>(talker, 0x598);
    if !(7..=0x10).contains(&tt) {
        return;
    }
    let t = rd::<i32>(target, 0x598);
    if !(7..=0x10).contains(&t) && ped_is_player(target) == 0 {
        return;
    }
    let say = |id: u32| {
        ped_say(talker, id, 0, 0x3F80_0000, 0, 0, 0);
    };
    let t = rd::<i32>(target, 0x598);
    if t == 7 {
        say(1);
        return;
    }
    if t == 8 || ped_is_player(target) != 0 {
        say(4);
        return;
    }
    let id = match rd::<i32>(target, 0x598) {
        0xA => 7,
        0xB => 8,
        0xC => 6,
        0xD => 5,
        0xE => 3,
        _ => return,
    };
    say(id);
}

// ======================================================================================================
// CTaskGangHasslePed
// ======================================================================================================

/// `CTaskGangHasslePed::CreateFirstSubTask(CPed*)` (0x664380, `ret 4`). Null without a target (+0xC). Two `rand()`
/// calls (return addresses 0x6643AD, 0x6643D2): `r = rand() * 3.0518509e-05f * 2 + 3.0f` and +0x24 = +0x14 +
/// `_ftol((+0x18 - +0x14) * ((rand() & 0xFFFF) * 3.0517578e-05f))`; +0x20 = now, +0x28 = 1. Then
/// `new CTaskComplexSeekEntityStandard(target, 999999, 1000, r * [0x86FC2C], [0x86FC28], [0x86FC30], false, true)`.
#[hook(addr = 0x664380, conv = thiscall, name = "CTaskGangHasslePed::CreateFirstSubTask", expect = "64 A1 00 00 00 00")]
pub fn gang_hassle_ped_create_first_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(this, 0xC).is_null() {
        return GPtr::NULL;
    }
    let r1 = at_site(0x6643AD, rand);
    let hi = rd::<i32>(this, 0x18);
    let lo = rd::<i32>(this, 0x14);
    let k = xi(r1).fmul_m(c(0x858C7C));
    let radius = f32_of(k.fdouble().fadd_m(c(0x858B3C)));
    let r2 = at_site(0x6643D2, rand) & 0xFFFF;
    let now = g::<u32>(TIME_MS).read();
    let f = xi(r2).fmul_m(c(0x858B14));
    wr::<u32>(this, 0x20, now);
    let n = ftol32(xi(hi.wrapping_sub(lo)).fmul(f));
    wr::<i32>(this, 0x24, n.wrapping_add(lo));
    wr::<u8>(this, 0x28, 1);
    let mem = task_new(0x4C);
    if mem.is_null() {
        return GPtr::NULL;
    }
    let c30 = g::<u32>(0x86FC30).read();
    let r = f32_of(x(c(0x86FC2C)).fmul_m(radius));
    let c28 = g::<u32>(0x86FC28).read();
    seek_entity_standard_ctor(
        mem,
        rd::<GPtr<u8>>(this, 0xC),
        0xF423F,
        0x3E8,
        r.to_bits(),
        c28,
        c30,
        0,
        1,
    )
}

/// `CTaskGangHasslePed::ControlSubTask(CPed*)` (0x65FFE0, `ret 4`). Null without a target (+0xC). With the timer
/// running (+0x28; +0x29 restarts it at now): once `now >= +0x20 + +0x24` (unsigned), mode +0x10 == 2 switches to a new
/// `CTaskComplexKillPedOnFoot(target, -1, 0, 0, 0, true)` unless the sub-task already is a 0x3E8 (allocation failure:
/// null), mode 1 continues, any other mode returns null. Then the "gangs" anim block (flag +0x1C); with it loaded,
/// no hand signal playing and no secondary task 4: `r = GetRandomNumberInRange(0, 200)`, 50 < r < 56 -> secondary
/// `CTaskSimpleRunAnim(0x34, 0x117 + _ftol((rand() & 0xFFFF) * 3.0517578e-05f * 8.0f), 4.0f, false)` (`rand()`
/// 0x66019E, after the allocation), r == 100 -> secondary `CTaskComplexPlayHandSignalAnim(-1, 4.0f)` (a failed
/// allocation sets a null secondary task). Mode 0: `ped->Say(0x56, 0, 1.0f, 0, 0, 0)`; modes 1 and 2:
/// `DoGangAbuseSpeech(ped, target)`. Returns the sub-task.
#[hook(addr = 0x65FFE0, conv = thiscall, name = "CTaskGangHasslePed::ControlSubTask", expect = "6A FF 68 B1 45 84 00")]
pub fn gang_hassle_ped_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(this, 0xC).is_null() {
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0x28) != 0 {
        if rd::<u8>(this, 0x29) != 0 {
            wr::<u32>(this, 0x20, g::<u32>(TIME_MS).read());
            wr::<u8>(this, 0x29, 0);
        }
        let end = rd::<u32>(this, 0x24).wrapping_add(rd::<u32>(this, 0x20));
        if g::<u32>(TIME_MS).read() >= end {
            match rd::<i32>(this, 0x10) {
                2 => {
                    if v_task_type(rd::<GPtr<u8>>(this, 8)) != 0x3E8 {
                        let mem = task_new(0x38);
                        if mem.is_null() {
                            return GPtr::NULL;
                        }
                        return kill_ped_on_foot_ctor(mem, rd::<GPtr<u8>>(this, 0xC), -1, 0, 0, 0, 1);
                    }
                }
                1 => {}
                _ => return GPtr::NULL,
            }
        }
    }
    gang_anims_stream(this, 0x1C);
    if rd::<u8>(this, 0x1C) != 0
        && ped_is_playing_hand_signal(ped) == 0
        && tm_get_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 4).is_null()
    {
        let r = general_random_int_in_range(0, 0xC8);
        let task = if r > 0x32 && r < 0x38 {
            let mem = task_new(0x20);
            if mem.is_null() {
                Some(GPtr::NULL)
            } else {
                let i = rand_scaled(0x66019E, 0x859000);
                Some(run_anim_ctor(mem, 0x34, i.wrapping_add(0x117), 0x4080_0000, 0))
            }
        } else if r == 0x64 {
            let mem = task_new(0x18);
            if mem.is_null() {
                Some(GPtr::NULL)
            } else {
                Some(play_hand_signal_anim_ctor(mem, -1, 0x4080_0000))
            }
        } else {
            None
        };
        if let Some(t) = task {
            tm_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 4);
        }
    }
    match rd::<i32>(this, 0x10) {
        0 => {
            ped_say(ped, 0x56, 0, 0x3F80_0000, 0, 0, 0);
        }
        1 | 2 => do_gang_abuse_speech_orig(ped, rd::<GPtr<u8>>(this, 0xC)),
        _ => {}
    }
    rd::<GPtr<u8>>(this, 8)
}

// ======================================================================================================
// CTaskComplexSignalAtPed
// ======================================================================================================

/// `CTaskComplexSignalAtPed::CreateNextSubTask(CPed*)` (0x660B30, `ret 4`). After a 0x398 (stand still): a new
/// `CTaskSimpleStandStill(+0x10, false, false, 8.0f)` (+0x10 == -1: `_ftol((rand() & 0xFFFF) * 3.0517578e-05f *
/// 2000.0f)`, `rand()` 0x660B67, before the allocation). After a 0xCB (`GetTaskType` called again): an inlined
/// `CTaskSimpleDoHandSignal` (0xC bytes: `CTaskSimple`, vtable 0x86FA68, +8 = 0). After a 0x4BE (third call), with
/// +0x15 and +0x14 set: `CTaskSimpleRunAnim(0x34, 0x117 - _ftol((rand() & 0xFFFF) * 3.0517578e-05f * -8.0f), 4.0f,
/// false)` (`rand()` 0x660C5A, after the allocation). Else null.
#[hook(addr = 0x660B30, conv = thiscall, name = "CTaskComplexSignalAtPed::CreateNextSubTask", expect = "64 A1 00 00 00 00")]
pub fn signal_at_ped_create_next_sub_task(this: GPtr<u8>, _ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    if v_task_type(sub()) == 0x398 {
        let mut time = rd::<i32>(this, 0x10);
        if time == -1 {
            time = rand_scaled(0x660B67, 0x8631F0);
        }
        let mem = task_new(0x20);
        if mem.is_null() {
            return GPtr::NULL;
        }
        return stand_still_ctor(mem, time, 0, 0, 0x4100_0000);
    }
    if v_task_type(sub()) == 0xCB {
        let mem = task_new(0xC);
        if mem.is_null() {
            return GPtr::NULL;
        }
        task_simple_ctor(mem);
        wr::<u32>(mem, 0, 0x86FA68);
        wr::<u8>(mem, 8, 0);
        return mem;
    }
    if v_task_type(sub()) == 0x4BE && rd::<u8>(this, 0x15) != 0 && rd::<u8>(this, 0x14) != 0 {
        let mem = task_new(0x20);
        if mem.is_null() {
            return GPtr::NULL;
        }
        let i = rand_scaled(0x660C5A, 0x85B320);
        return run_anim_ctor(mem, 0x34, 0x117i32.wrapping_sub(i), 0x4080_0000, 0);
    }
    GPtr::NULL
}

/// `CTaskComplexSignalAtPed::CreateFirstSubTask(CPed*)` (0x660CC0, `ret 4`). Null without a target (+0xC) or when
/// `ShouldLoadGangAnims()` is false; else `ped->StopPlayingHandSignal()`, `g_ikChainMan.LookAt(0x86FAB8, ped, target,
/// 5000, 5, null, true, 0.3f, 500, 3, false)` and a new `CTaskComplexTurnToFaceEntityOrCoord(target, [0x86FC7C],
/// [0x86FC80])`.
#[hook(addr = 0x660CC0, conv = thiscall, name = "CTaskComplexSignalAtPed::CreateFirstSubTask", expect = "64 A1 00 00 00 00")]
pub fn signal_at_ped_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(this, 0xC).is_null() || should_load_gang_anims() == 0 {
        return GPtr::NULL;
    }
    ped_stop_playing_hand_signal(ped);
    ik_look_at(
        g(IK_CHAIN_MAN),
        0x86FAB8,
        ped,
        rd::<GPtr<u8>>(this, 0xC),
        0x1388,
        5,
        0,
        1,
        0x3E19_999A,
        0x1F4,
        3,
        0,
    );
    let mem = task_new(0x28);
    if mem.is_null() {
        return GPtr::NULL;
    }
    let b = g::<u32>(0x86FC80).read();
    let a = g::<u32>(0x86FC7C).read();
    turn_to_face_entity_ctor(mem, rd::<GPtr<u8>>(this, 0xC), a, b)
}

/// The "gangs" anim block handling shared by the gang tasks (flag byte at `this + off`): without the flag, when
/// `ShouldLoadGangAnims()`: add a reference if the block is loaded (and set the flag), else request it; with the
/// flag, when `!ShouldLoadGangAnims()`: drop the reference and clear the flag.
fn gang_anims_stream(this: GPtr<u8>, off: isize) {
    if rd::<u8>(this, off) == 0 {
        if should_load_gang_anims() != 0 {
            let idx = anim_get_block_index(STR_GANGS);
            if g::<u8>((idx as u32).wrapping_shl(5).wrapping_add(ANIM_BLOCK_LOADED) as usize).read() == 1 {
                anim_add_block_ref(idx);
                wr::<u8>(this, off, 1);
            } else {
                streaming_request_model(idx.wrapping_add(0x63E7), 8);
            }
        }
    } else if should_load_gang_anims() == 0 {
        anim_remove_block_ref(anim_get_block_index(STR_GANGS));
        wr::<u8>(this, off, 0);
    }
}

/// `CTaskComplexSignalAtPed::ControlSubTask(CPed*)` (0x660D80, `ret 4`). The "gangs" anim block (flag +0x15), then,
/// with a target, `ped->m_fAimingRotation (+0x55C) = LimitRadianAngle(GetRadianAngleBetweenPoints(dx, dy, 0, 0))`
/// (d = target - ped, both stored as f32; the original keeps the angle in its dead `ped` argument slot). Returns
/// the sub-task.
#[hook(addr = 0x660D80, conv = thiscall, name = "CTaskComplexSignalAtPed::ControlSubTask", expect = "83 EC 0C 56 8B F1")]
pub fn signal_at_ped_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    gang_anims_stream(this, 0x15);
    let t = rd::<GPtr<u8>>(this, 0xC);
    if !t.is_null() {
        let pp = ent_pos(ped);
        let tp = ent_pos(t);
        let dx = f32_of(x(rd::<f32>(tp, 0)).fsub_m(rd::<f32>(pp, 0)));
        let dy = f32_of(x(rd::<f32>(tp, 4)).fsub_m(rd::<f32>(pp, 4)));
        let a = st0_f32(general_radian_angle_between_points(
            dx.to_bits(),
            dy.to_bits(),
            0,
            0,
        ));
        let l = st0_f32(general_limit_radian_angle(a.to_bits()));
        wr::<f32>(ped, 0x55C, l);
    }
    rd::<GPtr<u8>>(this, 8)
}

// ======================================================================================================
// CTaskComplexGangFollower
// ======================================================================================================

/// `CTaskComplexGangFollower::Constructor(CPedGroup*, CPed* leader, uint8, CVector offset (by value), float)`
/// (0x65EAA0, `ret 0x1C`). `CTaskComplex` base, vtable 0x86F938, +0x40 = +0x44 = 0, bytes +0x48 = +0x49 = 0, +0xC =
/// group, +0x3C = the byte, +0x2C and +0x20 = offset, +0x38 = the float, +0x10 = leader, +0x3D |= 0x14; with a leader
/// it is registered and its position copied to +0x14; +0x3D &= 0xFC; bit 3 of +0x3D = (leader ==
/// `FindPlayerPed(0)`).
#[hook(addr = 0x65EAA0, conv = thiscall, name = "CTaskComplexGangFollower::Constructor", expect = "6A FF 68 E8 43 84 00")]
#[allow(clippy::too_many_arguments)]
pub fn gang_follower_ctor(
    this: GPtr<u8>,
    group: u32,
    leader: GPtr<u8>,
    b: u32,
    vx: u32,
    vy: u32,
    vz: u32,
    f: u32,
) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u32>(this, 0, 0x86F938);
    wr::<u32>(this, 0x40, 0);
    wr::<u32>(this, 0x44, 0);
    wr::<u8>(this, 0x48, 0);
    wr::<u8>(this, 0x49, 0);
    wr::<u32>(this, 0xC, group);
    wr::<u8>(this, 0x3C, b as u8);
    wr::<u32>(this, 0x2C, vx);
    wr::<u32>(this, 0x30, vy);
    wr::<u32>(this, 0x34, vz);
    wr::<u32>(this, 0x20, vx);
    wr::<u32>(this, 0x24, vy);
    wr::<u32>(this, 0x28, vz);
    wr::<u32>(this, 0x38, f);
    let fl = rd::<u8>(this, 0x3D) | 0x14;
    let leader_ref = this.byte_add(0x10);
    wr::<GPtr<u8>>(leader_ref, 0, leader);
    wr::<u8>(this, 0x3D, fl);
    if !leader.is_null() {
        entity_register_reference(leader, leader_ref);
        let p = ent_pos(rd::<GPtr<u8>>(leader_ref, 0));
        wr::<u32>(this, 0x14, rd::<u32>(p, 0));
        wr::<u32>(this, 0x18, rd::<u32>(p, 4));
        wr::<u32>(this, 0x1C, rd::<u32>(p, 8));
    }
    wr::<u8>(this, 0x3D, rd::<u8>(this, 0x3D) & 0xFC);
    let player = world_find_player_ped(0);
    let fl = rd::<u8>(this, 0x3D);
    wr::<u8>(
        this,
        0x3D,
        if rd::<GPtr<u8>>(this, 0x10) == player {
            fl | 8
        } else {
            fl & 0xF7
        },
    );
    this
}

/// `CTaskComplexGangFollower::CalculateOffsetPosition(CVector* out)` (0x65ED40, `ret 4`). Leader running (move
/// state 4, 6 or 7): +0x20 = `FromMultiply3x3(leader matrix, (ms_offsetsRun[+0x3C] at 0xC19728, 0))` and +0x3D bit 4
/// cleared (the leader's last position +0x14 is kept). Otherwise, when the leader moved more than 3 units (squared
/// distance > 9.0 [0x86F838], `test ah, 0x41; je`) or +0x3D bit 4 is clear: +0x14 = leader position, +0x20 =
/// (ms_offsets[+0x3C] at 0xC196E8, 0) and bit 4 set. Copies +0x20 to `out`. The offset vector lives on the
/// original's stack (a Rust local).
#[hook(addr = 0x65ED40, conv = thiscall, name = "CTaskComplexGangFollower::CalculateOffsetPosition", expect = "83 EC 14 56 8B F1")]
pub fn gang_follower_calculate_offset_position(this: GPtr<u8>, out: GPtr<u8>) {
    let leader = rd::<GPtr<u8>>(this, 0x10);
    let ms = rd::<i32>(leader, 0x534);
    let running = ms == 4 || ms == 6 || ms == 7;
    if running {
        let idx = u32::from(rd::<u8>(this, 0x3C));
        let ox = g::<u32>(idx.wrapping_mul(8).wrapping_add(0xC19728) as usize).read();
        let oy = g::<u32>(idx.wrapping_mul(8).wrapping_add(0xC1972C) as usize).read();
        let mut v = [ox, oy, 0u32];
        let vp: GPtr<u8> = GPtr::from_raw(&mut v as *mut [u32; 3]).cast();
        vector_from_multiply_3x3(this.byte_add(0x20), rd::<GPtr<u8>>(leader, 0x14), vp);
        wr::<u8>(this, 0x3D, rd::<u8>(this, 0x3D) & 0xEF);
    } else {
        let p = ent_pos(leader);
        let dx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(this, 0x14));
        let dy = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(this, 0x18));
        let dz = x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(this, 0x1C));
        let sum = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        let far = !sum.fcom(x(c(0x86F838))).test_any(0x41);
        if far || rd::<u8>(this, 0x3D) & 0x10 == 0 {
            let p = ent_pos(leader);
            wr::<u32>(this, 0x14, rd::<u32>(p, 0));
            wr::<u32>(this, 0x18, rd::<u32>(p, 4));
            let z = rd::<u32>(p, 8);
            let idx = u32::from(rd::<u8>(this, 0x3C));
            wr::<u32>(this, 0x1C, z);
            let ox = g::<u32>(idx.wrapping_mul(8).wrapping_add(0xC196E8) as usize).read();
            let oy = g::<u32>(idx.wrapping_mul(8).wrapping_add(0xC196EC) as usize).read();
            wr::<u32>(this, 0x20, ox);
            wr::<u32>(this, 0x24, oy);
            wr::<u32>(this, 0x28, 0);
            wr::<u8>(this, 0x3D, rd::<u8>(this, 0x3D) | 0x10);
        }
    }
    wr::<u32>(out, 0, rd::<u32>(this, 0x20));
    wr::<u32>(out, 4, rd::<u32>(this, 0x24));
    wr::<u32>(out, 8, rd::<u32>(this, 0x28));
}

/// `CTaskComplexGangFollower::CreateFirstSubTask(CPed*)` (0x666160, `ret 4`). No leader: ped +0x478 bit 16 cleared,
/// null. In the leader's vehicle: `CTaskSimpleCarDrive(vehicle, 0, 0)`; in another vehicle: `CTaskComplexLeaveCar(
/// vehicle, 0, 0, true, false)`. On foot, with +0x3D bit 2 and the intelligence value +0x274 not above [0x86C938]
/// (signed): `CTaskComplexSeekEntity<XYOffset>(leader, 50000, 1000, [0x86FC2C] * 0.5f, [0x86F830], [0x86FC30], false,
/// true)` with its offset (+0x44) = +0x20, +0x54 |= 8, +0x50 = 7 (written through the result also when the
/// allocation failed, as the original does); else `CTaskSimpleStandStill(500, false, false, 8.0f)`.
#[hook(addr = 0x666160, conv = thiscall, name = "CTaskComplexGangFollower::CreateFirstSubTask", expect = "64 A1 00 00 00 00")]
pub fn gang_follower_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let leader = rd::<GPtr<u8>>(this, 0x10);
    if leader.is_null() {
        wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) & 0xFFFE_FFFF);
        return GPtr::NULL;
    }
    let in_veh = (rd::<u32>(ped, 0x46C) >> 8) & 1 != 0;
    if in_veh {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        if !v.is_null() && rd::<GPtr<u8>>(leader, 0x58C) == v {
            let mem = task_new(0x60);
            if mem.is_null() {
                return GPtr::NULL;
            }
            return simple_car_drive_ctor(mem, rd::<GPtr<u8>>(ped, 0x58C), 0, 0);
        }
        if !rd::<GPtr<u8>>(ped, 0x58C).is_null() {
            let mem = task_new(0x34);
            if mem.is_null() {
                return GPtr::NULL;
            }
            return leave_car_ctor(mem, rd::<GPtr<u8>>(ped, 0x58C), 0, 0, 1, 0);
        }
    }
    let v274 = rd::<i32>(rd::<GPtr<u8>>(ped, 0x47C), 0x274);
    let limit = g::<i32>(0x86C938).read();
    if rd::<u8>(this, 0x3D) & 4 != 0 && v274 <= limit {
        let mem = task_new(0x58);
        let t = if mem.is_null() {
            GPtr::NULL
        } else {
            let c30 = g::<u32>(0x86FC30).read();
            let r = x(c(0x86FC2C)).fmul_m(c(0x858B8C));
            let c830 = g::<u32>(0x86F830).read();
            seek_entity_xy_offset_ctor(
                mem,
                rd::<GPtr<u8>>(this, 0x10),
                0xC350,
                0x3E8,
                f32_of(r).to_bits(),
                c830,
                c30,
                0,
                1,
            )
        };
        let (ox, oy, oz) = (
            rd::<u32>(this, 0x20),
            rd::<u32>(this, 0x24),
            rd::<u32>(this, 0x28),
        );
        wr::<u32>(t, 0x44, ox);
        wr::<u32>(t, 0x48, oy);
        wr::<u32>(t, 0x4C, oz);
        wr::<u8>(t, 0x54, rd::<u8>(t, 0x54) | 8);
        wr::<u32>(t, 0x50, 7);
        return t;
    }
    let mem = task_new(0x20);
    if mem.is_null() {
        return GPtr::NULL;
    }
    stand_still_ctor(mem, 0x1F4, 0, 0, 0x4100_0000)
}

// ======================================================================================================
// CTaskComplexPartner / CTaskComplexPartnerChat / CTaskComplexBeInCouple
// ======================================================================================================

/// `CTaskComplexPartner::Constructor(const char* name (unused), CPed* partner, bool lead, float distance, bool
/// face, int8 updateDirection, CVector point (by value))` (0x681E70, `ret 0x24`). `CTaskComplex` base, vtable
/// 0x870664, +0x58 lead, +0x5D face, +0x3C distance, +0x5B update count, +0x40..+0x48 point, +0x38 partner
/// (registered when non-null), +0x59 = 1, +0x5C = 0, +0x5A = 0xFF, +0x6E = 0, +0x5E = 0.
#[hook(addr = 0x681E70, conv = thiscall, name = "CTaskComplexPartner::Constructor", expect = "6A FF 68 D8 64 84 00")]
#[allow(clippy::too_many_arguments)]
pub fn partner_ctor(
    this: GPtr<u8>,
    _name: GPtr<u8>,
    partner: GPtr<u8>,
    lead: u32,
    dist: u32,
    face: u32,
    upd: u32,
    vx: u32,
    vy: u32,
    vz: u32,
) -> GPtr<u8> {
    task_complex_ctor(this);
    wr::<u8>(this, 0x58, lead as u8);
    wr::<u8>(this, 0x5D, face as u8);
    wr::<u32>(this, 0x3C, dist);
    wr::<u8>(this, 0x5B, upd as u8);
    wr::<u32>(this, 0x40, vx);
    wr::<u32>(this, 0x44, vy);
    wr::<u32>(this, 0, 0x870664);
    wr::<GPtr<u8>>(this, 0x38, partner);
    wr::<u32>(this, 0x48, vz);
    wr::<u8>(this, 0x59, 1);
    wr::<u8>(this, 0x5C, 0);
    wr::<u8>(this, 0x5A, 0xFF);
    wr::<u8>(this, 0x6E, 0);
    wr::<u8>(this, 0x5E, 0);
    if !partner.is_null() {
        entity_register_reference(partner, this.byte_add(0x38));
    }
    this
}

/// `CTaskComplexPartnerChat::Constructor(const char* name, CPed* partner, bool lead, float distance, int8
/// updateDirection, bool a5, bool a6, CVector point (by value))` (0x684290, `ret 0x28`). `CTaskComplexPartner(name,
/// partner, lead, distance, false, updateDirection, point)` through 0x681E70, vtable 0x8707C4, +0x34 = 0x4B4, +0x76 =
/// 0, +0x74 = a5 (and +0x5B = 4 when set), +0x75 = a6, then the name is copied to +0x14 (unbounded `strcpy`).
#[hook(addr = 0x684290, conv = thiscall, name = "CTaskComplexPartnerChat::Constructor", expect = "8B 54 24 24 56")]
#[allow(clippy::too_many_arguments)]
pub fn partner_chat_ctor(
    this: GPtr<u8>,
    name: GPtr<u8>,
    partner: GPtr<u8>,
    lead: u32,
    dist: u32,
    upd: u32,
    a5: u32,
    a6: u32,
    vx: u32,
    vy: u32,
    vz: u32,
) -> GPtr<u8> {
    partner_ctor_orig(this, name, partner, lead, dist, 0, upd, vx, vy, vz);
    let a5 = a5 as u8;
    wr::<u32>(this, 0, 0x8707C4);
    wr::<u32>(this, 0x34, 0x4B4);
    wr::<u8>(this, 0x76, 0);
    wr::<u8>(this, 0x74, a5);
    if a5 != 0 {
        wr::<u8>(this, 0x5B, 4);
    }
    wr::<u8>(this, 0x75, a6 as u8);
    let mut i = 0isize;
    loop {
        let ch = rd::<u8>(name, i);
        wr::<u8>(this, 0x14 + i, ch);
        i += 1;
        if ch == 0 {
            break;
        }
    }
    this
}

/// `CTaskComplexPartner::ControlSubTask(CPed*)` (0x6840D0, `ret 4`). The sub-task is asked to abort (priority 1)
/// when the state +0x59 > 1 (signed), there is no partner, or +0x5C is set, unless the partner runs a task of type
/// +0x34 (`FindActiveTaskByType`) whose +0x38 is this ped and +0x5C is clear; a 0x387 sub-task also when the
/// counter +0x70 (i16, incremented) exceeds 150. An aborted sub-task returns null; else
/// `DropEntityThatThisPedIsHolding(true)`, `StreamRequiredAnims()` (vtable +0x2C), the IK look-at is aborted (500
/// ms) when looking, and the sub-task is returned.
#[hook(addr = 0x6840D0, conv = thiscall, name = "CTaskComplexPartner::ControlSubTask", expect = "56 8B F1 80 7E 59 01")]
pub fn partner_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = || rd::<GPtr<u8>>(this, 8);
    let skip_abort =
        rd::<i8>(this, 0x59) <= 1 && !rd::<GPtr<u8>>(this, 0x38).is_null() && rd::<u8>(this, 0x5C) == 0;
    let try_abort = if skip_abort {
        false
    } else {
        let partner = rd::<GPtr<u8>>(this, 0x38);
        if partner.is_null() {
            true
        } else {
            let tm = rd::<GPtr<u8>>(partner, 0x47C).byte_add(4);
            let t = tm_find_active_task_by_type(tm, rd::<i32>(this, 0x34));
            t.is_null() || rd::<GPtr<u8>>(t, 0x38) != ped || rd::<u8>(this, 0x5C) != 0
        }
    };
    if try_abort && v_make_abortable(sub(), ped, 1, GPtr::NULL) != 0 {
        return GPtr::NULL;
    }
    if v_task_type(sub()) == 0x387 {
        let n = rd::<i16>(this, 0x70).wrapping_add(1);
        wr::<i16>(this, 0x70, n);
        if rd::<i16>(this, 0x70) > 0x96 && v_make_abortable(sub(), ped, 1, GPtr::NULL) != 0 {
            return GPtr::NULL;
        }
    }
    ped_drop_held_entity(ped, 1);
    v_partner_stream_required_anims(this);
    if ik_is_looking(g(IK_CHAIN_MAN), ped) != 0 {
        ik_abort_look_at(g(IK_CHAIN_MAN), ped, 0x1F4);
    }
    rd::<GPtr<u8>>(this, 8)
}

/// `CTaskComplexBeInCouple::CreateNextSubTask(CPed*)` (0x684840, `ret 4`). `AbortArmIK(ped)` on every path. No
/// partner (+0x10): null. With +0x14 set: a new `CTaskComplexWanderStandard(4, GetRandomNumberInRange(0, 8), true)`
/// (the draw after the allocation); else a new `CTaskComplexWalkAlongsidePed(partner, +0x18)`.
#[hook(addr = 0x684840, conv = thiscall, name = "CTaskComplexBeInCouple::CreateNextSubTask", expect = "64 A1 00 00 00 00")]
pub fn be_in_couple_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<GPtr<u8>>(this, 0x10).is_null() {
        be_in_couple_abort_arm_ik(this, ped);
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0x14) != 0 {
        be_in_couple_abort_arm_ik(this, ped);
        let mem = task_new(0x38);
        if mem.is_null() {
            return GPtr::NULL;
        }
        let dir = general_random_int_in_range(0, 8);
        return wander_standard_ctor(mem, 4, dir, 1);
    }
    be_in_couple_abort_arm_ik(this, ped);
    let mem = task_new(0x20);
    if mem.is_null() {
        return GPtr::NULL;
    }
    walk_alongside_ped_ctor(mem, rd::<GPtr<u8>>(this, 0x10), rd::<u32>(this, 0x18))
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_GANG_LEADER_CREATE_FIRST_SUB_TASK,
    HOOK_GANG_LEADER_CREATE_NEXT_SUB_TASK,
    HOOK_GANG_LEADER_GET_RANDOM_GANG_AMBIENT_ANIM,
    HOOK_GANG_LEADER_SHOULD_LOAD_GANG_ANIMS,
    HOOK_GANG_LEADER_DO_GANG_ABUSE_SPEECH,
    HOOK_GANG_HASSLE_PED_CREATE_FIRST_SUB_TASK,
    HOOK_GANG_HASSLE_PED_CONTROL_SUB_TASK,
    HOOK_SIGNAL_AT_PED_CREATE_NEXT_SUB_TASK,
    HOOK_SIGNAL_AT_PED_CREATE_FIRST_SUB_TASK,
    HOOK_SIGNAL_AT_PED_CONTROL_SUB_TASK,
    HOOK_GANG_FOLLOWER_CTOR,
    HOOK_GANG_FOLLOWER_CALCULATE_OFFSET_POSITION,
    HOOK_GANG_FOLLOWER_CREATE_FIRST_SUB_TASK,
    HOOK_PARTNER_CTOR,
    HOOK_PARTNER_CHAT_CTOR,
    HOOK_PARTNER_CONTROL_SUB_TASK,
    HOOK_BE_IN_COUPLE_CREATE_NEXT_SUB_TASK,
];
