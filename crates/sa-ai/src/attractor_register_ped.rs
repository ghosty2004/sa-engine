//! `CPedAttractorManager::RegisterPed<CPedAttractor>` (0x5EF980, batch `writerq-ai-3`): finds or creates the
//! attractor of an effect and registers a ped with it. One template instance serves all ten attractor types (the
//! constructor is chosen by the effect's type). Ported from the 1.0 US bytes (the SEH state store sits in the
//! protection fragment 0x4018DA). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPedAttractorPool::New()` (0x5EA9F0) on `CPools::ms_pPedAttractorPool`
    fn attractor_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x5EA9F0 => unported;
    /// `vector<CPedAttractor*>::push_back(const T&)` (0x5EE400, `ret 4`)
    fn push_back(v: GPtr<u8>, val: GPtr<u8>) = thiscall 0x5EE400 => crate::attractor_ptr_vectors::push_back_a;
    /// `CPedAttractor::RegisterPed(CPed*)` (0x5EEE30, `ret 4`)
    fn attractor_register_ped(a: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5EEE30 => unported;
    /// `CPedATMAttractor` constructor (type 0, 0x5EE470, `ret 0xC`)
    fn ctor0(a: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x5EE470 => unported;
    /// `CPedSeatAttractor` constructor (type 1, 0x5EE540)
    fn ctor1(a: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x5EE540 => unported;
    /// type 2 constructor (0x5EE610)
    fn ctor2(a: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x5EE610 => unported;
    /// type 3 constructor (0x5EE6E0)
    fn ctor3(a: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x5EE6E0 => unported;
    /// type 4 constructor (0x5EE7C0)
    fn ctor4(a: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x5EE7C0 => unported;
    /// type 5 constructor (0x5EE890)
    fn ctor5(a: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x5EE890 => unported;
    /// type 6 constructor (0x5EE960)
    fn ctor6(a: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x5EE960 => unported;
    /// type 7 constructor (0x5EEA30)
    fn ctor7(a: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x5EEA30 => unported;
    /// type 8 constructor (0x5EEB00)
    fn ctor8(a: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x5EEB00 => unported;
    /// type 9 constructor (0x5EEBD0)
    fn ctor9(a: GPtr<u8>, fx: GPtr<u8>, e: GPtr<u8>, ms: u32) -> GPtr<u8> = thiscall 0x5EEBD0 => unported;
}

/// `CPools::ms_pPedAttractorPool`
const ATTRACTOR_POOL: usize = 0xB744C4;

/// `CPedAttractorManager::RegisterPed<CPedAttractor>(CPed*, C2dEffectPedAttractor*, CEntity*, eMoveState,
/// SArray<CPedAttractor*>&)` (0x5EF980, thiscall, `ret 0x14`, SEH frame; `this` unused). The first attractor of the
/// vector with that effect (+4) and entity (+8) is used. Without one: an attractor type (+0x34) above 9 → null (no
/// registration); else a slot of the attractor pool constructed as the type's class (fx, entity, move state; null when
/// the pool is full), pushed to the vector even when null. A non-null attractor gets `CPedAttractor::RegisterPed(ped)`
/// (its result ignored) and is returned.
///
/// Stack: the original stores the new slot in its `entity` argument slot and the attractor in its move-state slot (the
/// `push_back` operand); not reproduced.
#[hook(addr = 0x5EF980, conv = thiscall, name = "CPedAttractorManager::RegisterPed<CPedAttractor>",
       expect = "64 A1 00 00 00 00 6A FF", unverified = "writerq-ai-3", route = "L2",
       hazards = "heap-writes,stack-residue", recs = "grove-walk",
       note = "the SEH state store (-1) is in the protection fragment 0x4018DA; the original rewrites its argument slots")]
pub fn register_ped_t(
    _this: GPtr<u8>,
    ped: GPtr<u8>,
    fx: GPtr<u8>,
    e: GPtr<u8>,
    ms: u32,
    arr: GPtr<u8>,
) -> GPtr<u8> {
    let first = rd::<u32>(arr, 4);
    let n = if first == 0 {
        0
    } else {
        (rd::<u32>(arr, 8).wrapping_sub(first) as i32) >> 2
    };
    let mut a = GPtr::<u8>::NULL;
    for i in 0..n {
        let c = GPtr::<GPtr<u8>>::from_addr(first as usize + 4 * i as usize).read();
        if rd::<GPtr<u8>>(c, 4) == fx && rd::<GPtr<u8>>(c, 8) == e {
            a = c;
            break;
        }
    }
    if a.is_null() {
        let t = rd::<u8>(fx, 0x34);
        if t > 9 {
            return GPtr::NULL;
        }
        let slot = attractor_pool_new(g::<GPtr<u8>>(ATTRACTOR_POOL).read());
        let mut made = if slot.is_null() {
            GPtr::NULL
        } else {
            let ctor = [
                ctor0, ctor1, ctor2, ctor3, ctor4, ctor5, ctor6, ctor7, ctor8, ctor9,
            ][usize::from(t)];
            ctor(slot, fx, e, ms)
        };
        push_back(arr, GPtr::from_raw(&raw mut made).cast());
        a = made;
        if a.is_null() {
            return GPtr::NULL;
        }
    }
    attractor_register_ped(a, ped);
    a
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_REGISTER_PED_T];
