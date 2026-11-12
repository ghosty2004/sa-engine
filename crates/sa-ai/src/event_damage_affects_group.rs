//! `CEventDamage::AffectsPedGroup` (INTRO-2: the Ballas drive-by at the end of the intro's first scene, when the
//! damage events reach CJ's group). Ported from the 1.0 US bytes; gta-reversed `EventDamage.cpp` as a reading aid.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn task_manager_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPedGroupMembership::GetMember(int32)` (0x5F69B0, `ret 4`)
    fn membership_get_member(m: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x5F69B0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CPedIntelligence::CanSeeEntityWithLights(const CEntity*, bool)` (0x605550, `ret 8`; ST0 result)
    fn intel_can_see_entity_with_lights(i: GPtr<u8>, e: GPtr<u8>, b: u32) -> f64 = thiscall 0x605550 => unported;
}

/// The position of an entity: the matrix's (+0x14) translation (+0x30) when it has one, else the placement (+4).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `CEventDamage::AffectsPedGroup(CPedGroup*)` (0x4B38D0, thiscall, `ret 4`; vtable slot +0x18).
///
/// False for weapon type (+0x1C) 0x17. True when the source (+0x14) is null, not a ped, has no active task or its
/// active task is not of type 0x403 (a gang drive-by). Else, for each of the group's 8 members (membership at the
/// group +8): `d = Normalise(source position - member position)` (each component stored as f32), and when
/// `member->intelligence.CanSeeEntityWithLights(source, false) > 0.0` and `(d.z * fwd.z + d.y * fwd.y) + d.x *
/// fwd.x` (the member's matrix forward, +0x10, not null-checked) is above [0xC0B034] (both strict), true. Else false.
#[hook(addr = 0x4B38D0, conv = thiscall, name = "CEventDamage::AffectsPedGroup", this = "CEventDamage",
       expect = "83 EC 10 57 8B F9 83 7F 1C 17",
       unverified = "intro3-6", route = "L2", hazards = "none", recs = "intro")]
pub fn event_damage_affects_ped_group(this: GPtr<u8>, group: GPtr<u8>) -> u8 {
    if rd::<i32>(this, 0x1C) == 0x17 {
        return 0;
    }
    let src = rd::<GPtr<u8>>(this, 0x14);
    if src.is_null() || rd::<u8>(src, 0x36) & 7 != 3 {
        return 1;
    }
    if task_manager_get_active_task(rd::<GPtr<u8>>(src, 0x47C).byte_add(4)).is_null() {
        return 1;
    }
    let t = task_manager_get_active_task(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x14), 0x47C).byte_add(4));
    if v_get_task_type(t) != 0x403 {
        return 1;
    }
    let membership = group.byte_add(8);
    for i in 0..8 {
        let m = membership_get_member(membership, i);
        if m.is_null() {
            continue;
        }
        let mp = entity_pos(m);
        let sp = entity_pos(rd::<GPtr<u8>>(this, 0x14));
        let mut d = [0f32; 3];
        for (k, v) in d.iter_mut().enumerate() {
            let o = 4 * k as isize;
            *v = x(rd::<f32>(sp, o)).fsub_m(rd::<f32>(mp, o)).to_f32();
        }
        let dp = GPtr::from_raw(d.as_mut_ptr()).cast::<u8>();
        vector_normalise(dp);
        let seen = intel_can_see_entity_with_lights(rd::<GPtr<u8>>(m, 0x47C), rd::<GPtr<u8>>(this, 0x14), 0);
        if X87::from_f64_exact(seen).fcom(x(g::<f32>(0x858B50).read())).test_any(0x41) {
            continue;
        }
        let f = rd::<GPtr<u8>>(m, 0x14).byte_add(0x10);
        let dot = x(d[2])
            .fmul_m(rd::<f32>(f, 8))
            .fadd(x(d[1]).fmul_m(rd::<f32>(f, 4)))
            .fadd(x(d[0]).fmul_m(rd::<f32>(f, 0)));
        if !dot.fcom(x(g::<f32>(0xC0B034).read())).test_any(0x41) {
            return 1;
        }
    }
    0
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_EVENT_DAMAGE_AFFECTS_PED_GROUP];
