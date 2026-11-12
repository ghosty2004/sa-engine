//! `CTaskComplexUseEffect::MakeAbortable` (batch `intro2-1`, reached by the intro's ambient peds after the arrival
//! cutscene). Ported from the 1.0 US bytes (gta-reversed `TaskComplexUseEffect.cpp` as a reading aid; the bytes win:
//! the 0x393 test is on the sub-task's type). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::MakeAbortable(CPed*, eAbortPriority, const CEvent*)` (vtable +0x18), AL
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: u32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CEvent::GetEventType()` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `GetPedAttractorManager()` (0x5EE190)
    fn get_ped_attractor_manager() -> GPtr<u8> = cdecl 0x5EE190 => unported;
    /// `CPedAttractorManager::DeRegisterPed(CPed*, CPedAttractor*)` (0x5EC850, `ret 8`), AL
    fn attractor_manager_deregister_ped(m: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>) -> u8 = thiscall 0x5EC850 => unported;
    /// `CVector::operator-` into `out` (0x40FE00, thiscall `out`, `ret 8`)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = thiscall 0x40FE00 => unported;
    /// `CVector::Magnitude()` (0x406DA0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
}

/// 2.25f (0x86E104)
const K_RANGE: usize = 0x86E104;

/// The entity's position: its matrix (+0x14) +0x30 when it has one, else the placement +4.
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CTaskComplexUseEffect::MakeAbortable(CPed* ped, eAbortPriority priority, const CEvent* event)` (0x6322F0,
/// thiscall, `ret 0xC`), AL. The sub-task (+8) `MakeAbortable(ped, priority, event)` false -> 0. Else
/// `GetPedAttractorManager()->DeRegisterPed(ped, +0x14)`, +0x14 = +0x18 = 0. With an event, a sub-task of type 0x393
/// and an event of type 2 or 3: `d = pedPos - subTask+0x10` (x, y as f32); when `dx² + dy² < 2.25` and the event's
/// ped (+0x14) is set: `v = victimPos - subTask+0x10` with z = 0; `|v| < 2.25` and the victim's +0x534 == 1 -> +0x20
/// = 1. Returns 1.
#[hook(addr = 0x6322F0, conv = thiscall, name = "CTaskComplexUseEffect::MakeAbortable", expect = "8B 54 24 08 83 EC 0C",
       unverified = "intro2-1", route = "L2", hazards = "heap-writes", recs = "intro")]
pub fn use_effect_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, prio: u32, event: GPtr<u8>) -> u8 {
    if v_make_abortable(rd::<GPtr<u8>>(this, 8), ped, prio, event) == 0 {
        return 0;
    }
    let a = rd::<GPtr<u8>>(this, 0x14);
    attractor_manager_deregister_ped(get_ped_attractor_manager(), ped, a);
    wr::<u32>(this, 0x14, 0);
    wr::<u32>(this, 0x18, 0);
    if event.is_null() || v_task_type(rd::<GPtr<u8>>(this, 8)) != 0x393 {
        return 1;
    }
    let ty = v_event_type(event);
    if ty != 2 && ty != 3 {
        return 1;
    }
    let target = rd::<GPtr<u8>>(this, 8).byte_add(0x10);
    let pos = entity_pos(ped);
    let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(target, 0)).to_f32();
    let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(target, 4)).to_f32();
    let k = g::<f32>(K_RANGE).read();
    if x(dx).fmul_m(dx).fadd(x(dy).fmul_m(dy)).fcom(x(k)).test_parity(5) {
        return 1;
    }
    let victim = rd::<GPtr<u8>>(event, 0x14);
    if victim.is_null() {
        return 1;
    }
    let mut v = [0f32; 3];
    // through the pointer only: the original stores z = 0 after the subtraction
    let vp = GPtr::from_raw(v.as_mut_ptr()).cast::<u8>();
    vector_sub(vp, entity_pos(victim), target);
    wr::<f32>(vp, 8, 0.0);
    let m = sa_math::x87::X87::from_f64_exact(vector_magnitude(vp));
    if !m.fcom(x(k)).test_parity(5) && rd::<u32>(victim, 0x534) == 1 {
        wr::<u8>(this, 0x20, 1);
    }
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_USE_EFFECT_MAKE_ABORTABLE];
