//! `CTaskComplexWanderStandard::LookForChatPartners` (batch `mvpl-ai-2`): a wandering ped on the pavement picks a
//! nearby wandering ped in front of it and both get a `CEventChatPartner`. Ported from the 1.0 US bytes. Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `SurfaceInfos_c::IsPavement(uint32 surface)` (0x55E7F0, `ret 4`)
    fn surface_is_pavement(si: GPtr<u8>, surface: u32) -> u8 = thiscall 0x55E7F0 => unported;
    /// `CStreaming::IsVeryBusy()` (0x4076A0)
    fn streaming_is_very_busy() -> u8 = cdecl 0x4076A0 => unported;
    /// `CGameLogic::LaRiotsActiveHere()` (0x441C10)
    fn la_riots_active_here() -> u8 = cdecl 0x441C10 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0, `ret 4`)
    fn intel_find_task_by_type(i: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CEventGroup::GetEventOfType(int32)` (0x4AB650, `ret 4`)
    fn event_group_get_event_of_type(g: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x4AB650 => unported;
    /// `CTaskComplexWanderStandard::WillChat(const CPed&, const CPed&)` (0x66AED0, `ret 8`)
    fn wander_will_chat(t: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> u8 = thiscall 0x66AED0 => unported;
    /// `CVector::Difference(const CVector& a, const CVector& b)` (0x40FE00, `ret 8`): `this = a - b`
    fn vector_difference(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = thiscall 0x40FE00 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0, ST0)
    fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CWorld::GetIsLineOfSightClear(start, end, buildings, vehicles, peds, objects, dummies, seeThrough,
    /// ignoreSomeObjects)` (0x56A490)
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bld: u32, veh: u32, ped: u32, obj: u32, dum: u32, see: u32, ign: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CEventChatPartner::CEventChatPartner(bool leader, CPed* partner)` (`FUN_004aecd0`, `ret 8`)
    fn event_chat_partner_ctor(e: GPtr<u8>, leader: u32, partner: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AECD0 => unported;
    /// `CEventChatPartner::~CEventChatPartner()` (`FUN_004aed60`)
    fn event_chat_partner_dtor(e: GPtr<u8>) = thiscall 0x4AED60 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CTaskComplexWanderStandard::SetNextMinScanTime(CPed*)` (0x66AF60, `ret 4`)
    fn wander_set_next_min_scan_time(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x66AF60 => unported;
}

/// `g_surfaceInfos` (0xB79538)
const SURFACE_INFOS: usize = 0xB79538;
/// `CWorld::PlayerInFocus` (byte, 0xB7CD74) and `CWorld::Players[i].m_pPed` (0xB7CD98, stride 0x190)
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
const PLAYERS_PED: u32 = 0xB7CD98;
/// The player vehicle speed² above which nobody starts chatting (0x863244)
const K_PLAYER_SPEED_SQ: usize = 0x863244;
/// The chat range (0x86FCD4; squared on the x87)
const K_CHAT_RANGE: usize = 0x86FCD4;
/// 0.0 (0x858B50)
const K_ZERO: usize = 0x858B50;

fn k(a: usize) -> f32 {
    g::<f32>(a).read()
}
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}
/// `(d.y*F.y + d.z*F.z) + d.x*F.x` with `F` = the matrix forward row (+0x10).
fn forward_dot(d: GPtr<u8>, m: GPtr<u8>) -> X87 {
    x(rd::<f32>(d, 4))
        .fmul_m(rd::<f32>(m, 0x14))
        .fadd(x(rd::<f32>(d, 8)).fmul_m(rd::<f32>(m, 0x18)))
        .fadd(x(rd::<f32>(d, 0)).fmul_m(rd::<f32>(m, 0x10)))
}

/// `CTaskComplexWanderStandard::LookForChatPartners(CPed& ped)` (0x66FDA0, thiscall, `ret 4`), AL.
///
/// False unless the ped stands on pavement (`IsPavement(ped +0xBA)`) and this +0xC is at most 4 (signed); false when
/// the focus player is in a vehicle (flags +0x46C bit 8, vehicle +0x58C) whose `|move speed|²` (`(x² + y²) + z²`,
/// x87) is above the limit, when `CStreaming::IsVeryBusy()` or `CGameLogic::LaRiotsActiveHere()`. Then over the 16
/// nearby peds (intelligence +0x130), the first `other` that: stands on pavement; has an active task whose type
/// equals this task's type; neither ped has a task of type 0x4B4 or 0x4BF (`FindTaskByType`) nor an event of type
/// 0x13 (`GetEventOfType`; checked ped, other, ped, other, ped, other in the original order); `WillChat(ped,
/// other)`; with `d = other - ped` (`Difference`, `MagnitudeSqr`) `|d|²` below `range * range`; `d · ped forward`
/// above 0 and `d · other forward` below 0 (matrices read without a null check); and `GetIsLineOfSightClear(ped,
/// other, buildings + objects)`. For it: a stack `CEventChatPartner(true, other)` into the ped's event group, a
/// `CEventChatPartner(false, ped)` into the other's, `SetNextMinScanTime` for both (ped first), the two destructors
/// (second event first) and AL = 1. The original keeps the ped position pointer in its own argument slot.
#[hook(addr = 0x66fda0, conv = thiscall, name = "CTaskComplexWanderStandard::LookForChatPartners", expect = "6A FF 68 90 55 84 00",
       route = "L2", hazards = "ax-al-return,stack-residue", recs = "grove-walk",
       note = "AL result (upper EAX bytes from the last callee); the events live on the Rust stack; the ped argument slot holds its position pointer")]
pub fn wander_look_for_chat_partners(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let si: GPtr<u8> = GPtr::from_addr(SURFACE_INFOS);
    if surface_is_pavement(si, u32::from(rd::<u8>(ped, 0xBA))) == 0 || rd::<i32>(this, 0xC) > 4 {
        return 0;
    }
    let focus = u32::from(g::<u8>(PLAYER_IN_FOCUS).read());
    let player = g::<GPtr<u8>>((PLAYERS_PED + focus * 0x190) as usize).read();
    let veh = rd::<GPtr<u8>>(player, 0x58C);
    if rd::<u32>(player, 0x46C) & 0x100 != 0 && !veh.is_null() {
        let (vx, vy, vz) = (x(rd::<f32>(veh, 0x44)), x(rd::<f32>(veh, 0x48)), x(rd::<f32>(veh, 0x4C)));
        let s = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz));
        if !s.fcom(x(k(K_PLAYER_SPEED_SQ))).test_any(0x41) {
            return 0;
        }
    }
    if streaming_is_very_busy() != 0 || la_riots_active_here() != 0 {
        return 0;
    }
    let ped_pos = entity_pos(ped);
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let mut d = [0f32; 3];
    let d_p: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
    let mut found = GPtr::<u8>::NULL;
    for i in 0..16 {
        let other = rd::<GPtr<u8>>(intel, 0x130 + 4 * i);
        if other.is_null() || surface_is_pavement(si, u32::from(rd::<u8>(other, 0xBA))) == 0 {
            continue;
        }
        let oi = rd::<GPtr<u8>>(other, 0x47C);
        if tm_get_active_task(oi.byte_add(4)).is_null() {
            continue;
        }
        let t1 = v_task_type(tm_get_active_task(rd::<GPtr<u8>>(other, 0x47C).byte_add(4)));
        if t1 != v_task_type(this) {
            continue;
        }
        let pi = || rd::<GPtr<u8>>(ped, 0x47C);
        let oi = || rd::<GPtr<u8>>(other, 0x47C);
        if !intel_find_task_by_type(pi(), 0x4B4).is_null()
            || !intel_find_task_by_type(oi(), 0x4B4).is_null()
            || !event_group_get_event_of_type(pi().byte_add(0x68), 0x13).is_null()
            || !event_group_get_event_of_type(oi().byte_add(0x68), 0x13).is_null()
            || !intel_find_task_by_type(pi(), 0x4BF).is_null()
            || !intel_find_task_by_type(oi(), 0x4BF).is_null()
        {
            continue;
        }
        if wander_will_chat(this, ped, other) == 0 {
            continue;
        }
        let om = rd::<GPtr<u8>>(other, 0x14);
        let opos = entity_pos(other);
        vector_difference(d_p, opos, ped_pos);
        let m2 = X87::from_f64_exact(vector_magnitude_sqr(d_p));
        let r = k(K_CHAT_RANGE);
        if x(r).fmul_m(r).fcom(m2).test_any(0x41) {
            continue;
        }
        if forward_dot(d_p, rd::<GPtr<u8>>(ped, 0x14)).fcom(x(k(K_ZERO))).test_any(0x41) {
            continue;
        }
        if forward_dot(d_p, om).fcom(x(k(K_ZERO))).test_parity(5) {
            continue;
        }
        if world_get_is_line_of_sight_clear(ped_pos, opos, 1, 0, 0, 1, 0, 0, 0) != 0 {
            found = other;
            break;
        }
    }
    if found.is_null() {
        return 0;
    }
    let other = found;
    let mut ev1 = [0u32; 8];
    let mut ev2 = [0u32; 8];
    let e1: GPtr<u8> = GPtr::from_raw(ev1.as_mut_ptr()).cast();
    let e2: GPtr<u8> = GPtr::from_raw(ev2.as_mut_ptr()).cast();
    event_chat_partner_ctor(e1, 1, other);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e1, 0);
    event_chat_partner_ctor(e2, 0, ped);
    event_group_add(rd::<GPtr<u8>>(other, 0x47C).byte_add(0x68), e2, 0);
    wander_set_next_min_scan_time(this, ped);
    wander_set_next_min_scan_time(this, other);
    event_chat_partner_dtor(e2);
    event_chat_partner_dtor(e1);
    1
}

pub const HOOKS: &[HookReg] = &[HOOK_WANDER_LOOK_FOR_CHAT_PARTNERS];
