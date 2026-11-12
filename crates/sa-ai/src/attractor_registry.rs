//! The ped-attractor registry lookups (batch `writerq-ai-2`): `CPedAttractorManager`'s per-type searches
//! (`IsPedRegistered`, `IsPedRegisteredWithEffect`, `GetRelevantAttractor`), `DeRegisterPed`, the type dispatch of
//! `RegisterPedWithAttractor`, `CPedAttractor::IsRegisteredWithPed` / `GetTaskForPed` and
//! `CScripted2dEffects::GetIndex`. Ported from the 1.0 US bytes (gta-reversed as a reading aid; where it differs, the
//! bytes are followed). Unverified (docs/verify/HOWTO.md).
//!
//! The manager holds ten `std::vector<CPedAttractor*>` of 0x10 bytes (first +4, last +8, end +0xC): type 1 at +0,
//! type 0 at +0x10, type k >= 2 at +0x10*k. An attractor's queue and arrived-ped vectors are at +0xC and +0x1C
//! (first +0x10 / +0x20), its task list (0x14-byte entries) at +0x2C (first +0x30).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPedAttractor::IsRegisteredWithPed(CPed*)` (0x5EB4C0, `ret 4`)
    fn attractor_is_registered_with_ped_o(a: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5EB4C0 => attractor_is_registered_with_ped;
    /// `CPedAttractorManager::IsPedRegistered<CPedAttractor>(CPed*, SArray&)` (0x5EB640, `ret 8`)
    fn is_ped_registered_o(m: GPtr<u8>, ped: GPtr<u8>, arr: GPtr<u8>) -> u8 = thiscall 0x5EB640 => is_ped_registered;
    /// `CPedAttractorManager::IsPedRegisteredWithEffect<CPedAttractor>(CPed*, const C2dEffect*, const CEntity*,
    /// SArray&)` (0x5EB690, `ret 0x10`, ECX unused; ported in `misc.rs`)
    fn is_ped_registered_with_effect_t_o(ped: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, arr: GPtr<u8>) -> u8 = stdcall 0x5EB690 => unported;
    /// `CPedAttractorManager::GetRelevantAttractor<CPedAttractor>(...)` (0x5EB7B0, `ret 0x10`)
    fn get_relevant_attractor_t_o(m: GPtr<u8>, ped: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, arr: GPtr<u8>) -> GPtr<u8> = thiscall 0x5EB7B0 => get_relevant_attractor_t;
    /// `CPedAttractorManager::IsPedRegisteredWithEffect(CPed*, const C2dEffect*, const CEntity*)` (0x5EBD70, `ret 0xC`)
    fn is_ped_registered_with_effect_o(m: GPtr<u8>, ped: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x5EBD70 => is_ped_registered_with_effect;
    /// `CPedAttractorManager::IsPedRegisteredWithEffect(CPed*)` (0x5EBCB0, `ret 4`)
    fn is_ped_registered_with_effect_ped_o(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5EBCB0 => is_ped_registered_with_effect_ped;
    /// `CPedAttractorManager::DeRegisterPed<CPedAttractor>(CPed*, CPedAttractor*, SArray&)` (0x5EC740, `ret 0xC`)
    fn deregister_ped_t_o(m: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>, arr: GPtr<u8>) -> u8 = thiscall 0x5EC740 => deregister_ped_t;
    /// `CPedAttractorManager::RegisterPed<CPedAttractor>(CPed*, C2dEffect*, CEntity*, eMoveState, SArray&)` (0x5EF980,
    /// `ret 0x14`)
    fn register_ped_t(m: GPtr<u8>, ped: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, ms: u32, arr: GPtr<u8>) -> GPtr<u8> = thiscall 0x5EF980 => crate::attractor_register_ped::register_ped_t;
    /// `CPedAttractor::DeRegisterPed(CPed*)` (0x5EC5B0, `ret 4`)
    fn attractor_deregister_ped(a: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5EC5B0 => unported;
    /// `CPedAttractor::GetType()` (vtable slot 0)
    fn attractor_get_type(a: GPtr<u8>) -> u32 = thiscall vslot 0x0 => unported;
    /// `CPedAttractor` scalar deleting destructor (vtable slot 4)
    fn attractor_delete(a: GPtr<u8>, flags: u32) = thiscall vslot 0x4 => unported;
    /// `CScripted2dEffects::GetIndex(const C2dEffect*)` (0x6F9F60)
    fn scripted_2d_effects_get_index_o(fx: GPtr<u8>) -> i32 = cdecl 0x6F9F60 => scripted_2d_effects_get_index;
    /// `CEventAttractor::IsEffectActive(CEntity*, const C2dEffectPedAttractor*)` (0x4AF460)
    fn event_attractor_is_effect_active(e: GPtr<u8>, fx: GPtr<u8>) -> u8 = cdecl 0x4AF460 => unported;
}

/// `CScripted2dEffects::ms_effects` (64 entries of 0x40 bytes)
const SCRIPTED_EFFECTS: u32 = 0xC3AB00;
/// The global `std::vector<CTask*>` of attractor tasks handed out (first 0xC09860, last 0xC09864)
const TASKS_FIRST: usize = 0xC09860;
const TASKS_LAST: usize = 0xC09864;

/// Element count of a `std::vector` of pointers whose first / last are at `v + 4` / `v + 8` (0 when first is null).
fn count4(v: GPtr<u8>) -> i32 {
    let first = rd::<u32>(v, 4);
    if first == 0 {
        0
    } else {
        (rd::<u32>(v, 8).wrapping_sub(first) as i32) >> 2
    }
}

/// The manager's vector for attractor type `t` (type 1 at +0, type 0 at +0x10, else +0x10*t).
fn array_of(m: GPtr<u8>, t: u32) -> GPtr<u8> {
    match t {
        0 => m.byte_add(0x10),
        1 => m,
        _ => m.byte_add(0x10 * t as isize),
    }
}

/// `CPedAttractor::IsRegisteredWithPed(CPed*)` (0x5EB4C0, thiscall, `ret 4`): AL = 1 when the ped is in the
/// attractor's vector at +0xC or the one at +0x1C (in that order), else 0.
///
/// EAX's upper bytes: the second vector's count on the miss path (not reproduced).
#[hook(addr = 0x5EB4C0, conv = thiscall, name = "CPedAttractor::IsRegisteredWithPed", expect = "8B 51 10 85 D2",
       unverified = "writerq-ai-2", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn attractor_is_registered_with_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    for v in [this.byte_add(0xC), this.byte_add(0x1C)] {
        let first = rd::<GPtr<u32>>(v, 4);
        for i in 0..count4(v) {
            if first.wrapping_index(i).read() == ped.addr() as u32 {
                return 1;
            }
        }
    }
    0
}

/// `CPedAttractorManager::IsPedRegistered<CPedAttractor>(CPed*, SArray<CPedAttractor*>&)` (0x5EB640, thiscall,
/// `ret 8`; `this` unused): AL = 1 when any attractor of the vector `IsRegisteredWithPed`, else 0.
#[hook(addr = 0x5EB640, conv = thiscall, name = "CPedAttractorManager::IsPedRegistered<CPedAttractor>",
       expect = "8B 44 24 08 53", unverified = "writerq-ai-2", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn is_ped_registered(_this: GPtr<u8>, ped: GPtr<u8>, arr: GPtr<u8>) -> u8 {
    let first = rd::<GPtr<GPtr<u8>>>(arr, 4);
    for i in 0..count4(arr) {
        if attractor_is_registered_with_ped_o(first.wrapping_index(i).read(), ped) != 0 {
            return 1;
        }
    }
    0
}

/// `CPedAttractorManager::GetRelevantAttractor<CPedAttractor>(const CPed*, const C2dEffect*, const CEntity*, SArray&)`
/// (0x5EB7B0, thiscall, `ret 0x10`; `this` unused): the first attractor of the vector with effect (+4) and entity (+8)
/// equal to the arguments that `IsRegisteredWithPed`, else null.
#[hook(addr = 0x5EB7B0, conv = thiscall, name = "CPedAttractorManager::GetRelevantAttractor<CPedAttractor>",
       expect = "8B 44 24 10 53", unverified = "writerq-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn get_relevant_attractor_t(
    _this: GPtr<u8>,
    ped: GPtr<u8>,
    fx: GPtr<u8>,
    e: GPtr<u8>,
    arr: GPtr<u8>,
) -> GPtr<u8> {
    let first = rd::<GPtr<GPtr<u8>>>(arr, 4);
    for i in 0..count4(arr) {
        let a = first.wrapping_index(i).read();
        if rd::<GPtr<u8>>(a, 4) == fx
            && rd::<GPtr<u8>>(a, 8) == e
            && attractor_is_registered_with_ped_o(a, ped) != 0
        {
            return a;
        }
    }
    GPtr::NULL
}

/// `CPedAttractorManager::IsPedRegisteredWithEffect(CPed*)` (0x5EBCB0, thiscall, `ret 4`): `IsPedRegistered` over the
/// vectors at +0, +0x10, ..., +0x90 in that order; AL = 1 at the first hit.
#[hook(addr = 0x5EBCB0, conv = thiscall, name = "CPedAttractorManager::IsPedRegisteredWithEffect-Ped",
       expect = "56 57 8B 7C 24 0C", unverified = "writerq-ai-2", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn is_ped_registered_with_effect_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    u8::from((0..10).any(|k| is_ped_registered_o(this, ped, this.byte_add(0x10 * k)) != 0))
}

/// `CPedAttractorManager::IsPedRegisteredWithEffect(CPed*, const C2dEffect*, const CEntity*)` (0x5EBD70, thiscall,
/// `ret 0xC`): the per-vector test over +0, +0x10, ..., +0x90 in that order; AL = 1 at the first hit.
#[hook(addr = 0x5EBD70, conv = thiscall, name = "CPedAttractorManager::IsPedRegisteredWithEffect-Effect",
       expect = "53 8B 5C 24 0C", unverified = "writerq-ai-2", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn is_ped_registered_with_effect(this: GPtr<u8>, ped: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>) -> u8 {
    u8::from((0..10).any(|k| is_ped_registered_with_effect_t_o(ped, fx, e, this.byte_add(0x10 * k)) != 0))
}

/// `CPedAttractorManager::GetRelevantAttractor(const CPed*, const C2dEffect*, const CEntity*)` (0x5EBF50, thiscall,
/// `ret 0xC`): the per-vector search over +0x10, +0, +0x20, +0x30, ..., +0x90 in that order; the first non-null.
#[hook(addr = 0x5EBF50, conv = thiscall, name = "CPedAttractorManager::GetRelevantAttractor-", expect = "53 8B 5C 24 0C",
       unverified = "writerq-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn get_relevant_attractor(this: GPtr<u8>, ped: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>) -> GPtr<u8> {
    for off in [0x10, 0, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90] {
        let a = get_relevant_attractor_t_o(this, ped, fx, e, this.byte_add(off));
        if !a.is_null() {
            return a;
        }
    }
    GPtr::NULL
}

/// `CPedAttractorManager::DeRegisterPed<CPedAttractor>(CPed*, CPedAttractor*, SArray&)` (0x5EC740, thiscall, `ret
/// 0xC`). A null attractor, or one not in the vector → AL = 0. Else `CPedAttractor::DeRegisterPed(ped)`; when the
/// attractor's two ped vectors (+0x1C, +0xC) are then both empty it is removed from the vector (later entries moved
/// down, last - 4) and deleted (vtable slot 4, flags 1). AL = 1.
#[hook(addr = 0x5EC740, conv = thiscall, name = "CPedAttractorManager::DeRegisterPed<CPedAttractor>",
       expect = "53 56 57 8B 7C 24 14", unverified = "writerq-ai-2", route = "L2", hazards = "heap-writes,ax-al-return",
       recs = "grove-walk", note = "writes the manager's vector (heap) and deletes the attractor")]
pub fn deregister_ped_t(_this: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>, arr: GPtr<u8>) -> u8 {
    if a.is_null() {
        return 0;
    }
    let first = rd::<GPtr<GPtr<u8>>>(arr, 4);
    let n = count4(arr);
    if !(0..n).any(|i| first.wrapping_index(i).read() == a) {
        return 0;
    }
    attractor_deregister_ped(a, ped);
    if count4(a.byte_add(0x1C)).wrapping_add(count4(a.byte_add(0xC))) != 0 {
        return 1;
    }
    let mut p = rd::<u32>(arr, 4);
    let last = rd::<u32>(arr, 8);
    while p != last {
        if GPtr::<GPtr<u8>>::from_addr(p as usize).read() == a {
            let mut q = p + 4;
            while q != last {
                GPtr::<u32>::from_addr(q as usize - 4).write(GPtr::<u32>::from_addr(q as usize).read());
                q += 4;
            }
            wr::<u32>(arr, 8, rd::<u32>(arr, 8).wrapping_sub(4));
            break;
        }
        p += 4;
    }
    attractor_delete(a, 1);
    1
}

/// `CPedAttractorManager::DeRegisterPed(CPed*, CPedAttractor*)` (0x5EC850, thiscall, `ret 8`): AL = 0 for a null
/// attractor or a ped not registered with any effect (`IsPedRegisteredWithEffect(ped)`); else the attractor's type
/// (vtable slot 0) above 9 → 0, otherwise `DeRegisterPed<CPedAttractor>(ped, attractor, vector of that type)`.
#[hook(addr = 0x5EC850, conv = thiscall, name = "CPedAttractorManager::DeRegisterPed-", expect = "56 57 8B 7C 24 10",
       unverified = "writerq-ai-2", route = "L2", hazards = "heap-writes,ax-al-return", recs = "grove-walk")]
pub fn deregister_ped(this: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>) -> u8 {
    if a.is_null() || is_ped_registered_with_effect_ped_o(this, ped) == 0 {
        return 0;
    }
    let t = attractor_get_type(a);
    if t > 9 {
        return 0;
    }
    deregister_ped_t_o(this, ped, a, array_of(this, t))
}

/// `CPedAttractorManager::RegisterPedWithAttractor(CPed*, C2dEffect*, CEntity*, eMoveState)` (0x5EFCA0, thiscall,
/// `ret 0x10`). Not a scripted effect (`CScripted2dEffects::GetIndex` < 0) and not active
/// (`CEventAttractor::IsEffectActive(entity, fx)`) → null. Already registered with the effect, an effect type (+0xC)
/// other than 3 or an attractor type (+0x34) above 9 → null. Else `RegisterPed<CPedAttractor>` with the vector of the
/// type (one template instance serves all ten).
#[hook(addr = 0x5EFCA0, conv = thiscall, name = "CPedAttractorManager::RegisterPedWithAttractor", expect = "53 56 57 8B 7C 24 14",
       unverified = "writerq-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn register_ped_with_attractor(
    this: GPtr<u8>,
    ped: GPtr<u8>,
    fx: GPtr<u8>,
    e: GPtr<u8>,
    ms: u32,
) -> GPtr<u8> {
    if scripted_2d_effects_get_index_o(fx) < 0 && event_attractor_is_effect_active(e, fx) == 0 {
        return GPtr::NULL;
    }
    if is_ped_registered_with_effect_o(this, ped, fx, e) != 0 || rd::<u8>(fx, 0xC) != 3 {
        return GPtr::NULL;
    }
    let t = u32::from(rd::<u8>(fx, 0x34));
    if t > 9 {
        return GPtr::NULL;
    }
    register_ped_t(this, ped, fx, e, ms, array_of(this, t))
}

/// `CPedAttractor::GetTaskForPed(CPed*)` (0x5EC500, thiscall, `ret 4`). The first entry of the attractor's task list
/// (+0x30, 0x14-byte entries: ped, task, ..., +0x10 "handed out") for the ped; none → null. An entry not yet handed out
/// has its task removed from the global task vector (0xC09860, first match, later entries moved down) when it is there;
/// the entry is then marked handed out. Returns the entry's task.
#[hook(addr = 0x5EC500, conv = thiscall, name = "CPedAttractor::GetTaskForPed", expect = "8B 41 30 53 56",
       unverified = "writerq-ai-2", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn attractor_get_task_for_ped(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let first = rd::<u32>(this, 0x30);
    let n = if first == 0 {
        0
    } else {
        (rd::<u32>(this, 0x34).wrapping_sub(first) as i32) / 0x14
    };
    let mut k = 0;
    while k < n {
        let entry: GPtr<u8> = GPtr::from_addr(first as usize + 0x14 * k as usize);
        if rd::<GPtr<u8>>(entry, 0) == ped {
            let task = rd::<u32>(entry, 4);
            if rd::<u8>(entry, 0x10) == 0 {
                let last = g::<u32>(TASKS_LAST).read();
                let mut p = g::<u32>(TASKS_FIRST).read();
                while p != last {
                    if GPtr::<u32>::from_addr(p as usize).read() == task {
                        let mut q = p + 4;
                        while q != last {
                            GPtr::<u32>::from_addr(q as usize - 4)
                                .write(GPtr::<u32>::from_addr(q as usize).read());
                            q += 4;
                        }
                        g::<u32>(TASKS_LAST).write(g::<u32>(TASKS_LAST).read().wrapping_sub(4));
                        break;
                    }
                    p += 4;
                }
            }
            wr::<u8>(entry, 0x10, 1);
            return GPtr::from_addr(task as usize);
        }
        k += 1;
    }
    GPtr::NULL
}

/// `CScripted2dEffects::GetIndex(const C2dEffect*)` (0x6F9F60, cdecl): the index of the effect among the 64 entries
/// of `ms_effects` (0xC3AB00, stride 0x40), else -1.
#[hook(addr = 0x6F9F60, conv = cdecl, name = "CScripted2dEffects::GetIndex", expect = "8B 54 24 04 56",
       unverified = "writerq-ai-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn scripted_2d_effects_get_index(fx: GPtr<u8>) -> i32 {
    (0..64)
        .find(|i| fx.addr() as u32 == SCRIPTED_EFFECTS + 0x40 * *i as u32)
        .unwrap_or(-1)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_ATTRACTOR_IS_REGISTERED_WITH_PED,
    HOOK_IS_PED_REGISTERED,
    HOOK_GET_RELEVANT_ATTRACTOR_T,
    HOOK_IS_PED_REGISTERED_WITH_EFFECT_PED,
    HOOK_IS_PED_REGISTERED_WITH_EFFECT,
    HOOK_GET_RELEVANT_ATTRACTOR,
    HOOK_DEREGISTER_PED_T,
    HOOK_DEREGISTER_PED,
    HOOK_REGISTER_PED_WITH_ATTRACTOR,
    HOOK_ATTRACTOR_GET_TASK_FOR_PED,
    HOOK_SCRIPTED_2D_EFFECTS_GET_INDEX,
];
