//! IK tasks: `CTaskSimpleIKLookAt::CreateIKChain` (batch `mvpj-ai-4`). Ported from the 1.0 US bytes;
//! unverified (docs/verify/HOWTO.md).
//!
//! `CTaskSimpleIKChain` (base of `CTaskSimpleIKLookAt`): +0x10 the IK chain, +0x16 i16 chain part, +0x18 i16
//! bone tag, +0x1C offset (CVector), +0x28 target entity, +0x2C target bone tag, +0x30 target offset
//! (CVector), +0x3C speed, +0x59 i8 priority.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `g_ikChainMan` (0xC15448)
const IK_CHAIN_MAN: usize = 0xC15448;
/// `""` (0x858B54): the chain name
const STR_EMPTY: usize = 0x858B54;

orig! {
    /// `IKChainManager_c::AddIKChain(name, index, ped, bone, CVector offset, part, entity, bone, CVector offset,
    /// speed, priority)` (0x618750, `ret 0x3C`)
    fn ik_manager_add_ik_chain(m: GPtr<u8>, name: u32, idx: u32, ped: GPtr<u8>, bone: i32, ox: u32, oy: u32, oz: u32, part: u32, ent: u32, tbone: u32, tx: u32, ty: u32, tz: u32, speed: u32, prio: i32) -> GPtr<u8> = thiscall 0x618750 => unported;
    /// `IKChain_c::ClampLimits(tag, x, y, z, current)` (0x618520, `ret 0x14`)
    fn ik_chain_clamp_limits(c: GPtr<u8>, tag: i32, lx: u32, ly: u32, lz: u32, current: u32) = thiscall 0x618520 => unported;
}

/// `CTaskSimpleIKLookAt::CreateIKChain(CPed*)` (0x633FC0, thiscall, `ret 4`). Part (+0x16) = 4; chain (+0x10) =
/// `g_ikChainMan.AddIKChain("", 0, ped, bone +0x18, offset +0x1C, 4, entity +0x28, bone +0x2C, offset +0x30,
/// speed +0x3C, priority +0x59)`; none → EAX = 0; else `ClampLimits(5, false, true, false, true)` on it and
/// AL = 1 with the chain pointer's upper bytes in EAX (reproduced).
#[hook(addr = 0x633fc0, conv = thiscall, name = "CTaskSimpleIKLookAt::CreateIKChain", expect = "56 8B F1 0F BE 46 59",
       unverified = "mvpj-ai-4", route = "L2", hazards = "ax-al-return,heap-writes", recs = "grove-walk",
       note = "the IK chain pool is heap memory")]
pub fn ik_look_at_create_ik_chain(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    wr::<u16>(this, 0x16, 4);
    let c = ik_manager_add_ik_chain(
        g::<u8>(IK_CHAIN_MAN),
        g::<u8>(STR_EMPTY).addr() as u32,
        0,
        ped,
        i32::from(rd::<i16>(this, 0x18)),
        rd::<u32>(this, 0x1C),
        rd::<u32>(this, 0x20),
        rd::<u32>(this, 0x24),
        4,
        rd::<u32>(this, 0x28),
        rd::<u32>(this, 0x2C),
        rd::<u32>(this, 0x30),
        rd::<u32>(this, 0x34),
        rd::<u32>(this, 0x38),
        rd::<u32>(this, 0x3C),
        i32::from(rd::<i8>(this, 0x59)),
    );
    wr::<GPtr<u8>>(this, 0x10, c);
    if c.is_null() {
        return 0;
    }
    ik_chain_clamp_limits(c, 5, 0, 1, 0, 1);
    (c.addr() as u32 & 0xFFFF_FF00) | 1
}

orig! {
    /// `CTask::CreateIKChain(CPed*)`-style virtual of `CTaskSimpleIKChain` (vtable +0x24, `ret 4`)
    fn v_ik_create_chain(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall vslot 0x24 => unported;
    /// `IKChain_c::SetBlend(float)` (0x617C10, `ret 4`)
    fn ik_chain_set_blend(c: GPtr<u8>, b: u32) = thiscall 0x617C10 => unported;
    /// `IKChain_c::UpdateTarget(bool)` (0x617E50, `ret 4`)
    fn ik_chain_update_target(c: GPtr<u8>, b: u32) = thiscall 0x617E50 => unported;
    /// `IKChainManager_c::RemoveIKChain(IKChain_c*)` (0x618170, `ret 4`)
    fn ik_manager_remove_ik_chain(m: GPtr<u8>, c: GPtr<u8>) = thiscall 0x618170 => unported;
    /// `CTaskSimpleIKChain::BlendOut(int32 time)` (0x633C40, `ret 4`)
    fn ik_chain_task_blend_out(t: GPtr<u8>, time: i32) = thiscall 0x633C40 => unported;
}

/// `CTimer::ms_fTimeStep` (0xB7CB5C)
const TIMER_TIME_STEP: usize = 0xB7CB5C;
/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIMER_TIME_MS: usize = 0xB7CB84;

/// `CTaskSimpleIKChain::ProcessPed(CPed*)` (0x633C80, thiscall, `ret 4`). x87. step = `_ftol(ms_fTimeStep * 0.02 *
/// 1000)`, now = the time in ms. Fields: +8 duration (-1 endless), +0xC blend time, +0x10 chain, +0x28 target
/// entity, +0x40 "has entity target" byte, +0x44 current blend, +0x48 end time, +0x4C target blend, +0x50 blend
/// end time. No chain: a lost entity target (+0x40 set, +0x28 NULL) or a failing `CreateIKChain` (vtable +0x24)
/// → done (AL = 1); else +0x48 = -1 or duration + now, +0x4C = 1.0, +0x50 = blend time + now, `SetBlend(+0x44)`,
/// AL = 0. With a chain: past the end (duration set, now > +0x48) → `RemoveIKChain`, +0x10 = NULL, AL = 1. A lost
/// entity target → +0x40 = 0, `UpdateTarget(false)`, `BlendOut(250)`. Duration set and now ≥ +0x48 - blend time →
/// target blend 0, +0x50 = +0x48. now > +0x50 → +0x44 = +0x4C, `SetBlend`, AL = 0; else f = step / (+0x50 - step
/// - now) (fild/fidiv) capped at 1.0, +0x44 = f32((+0x4C - +0x44) * f + +0x44), `SetBlend`, AL = 0.
///
/// The original keeps the divisor in its own `ped` argument slot (popped by `ret 4`); not reproduced. EAX's
/// upper bytes are `SetBlend`'s / the chain manager's leftovers in the original.
#[hook(addr = 0x633c80, conv = thiscall, name = "CTaskSimpleIKChain::ProcessPed", expect = "51 D9 05 5C CB B7 00",
       unverified = "mvpj-ai-6", route = "L2", hazards = "ax-al-return,stack-residue", recs = "grove-walk")]
pub fn ik_chain_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let step = x(g::<f32>(TIMER_TIME_STEP).read()).fmul_m(0.02).fmul_m(1000.0).ftol() as u32 as i32;
    let now = g::<i32>(TIMER_TIME_MS).read();
    let chain = rd::<GPtr<u8>>(this, 0x10);
    if chain.is_null() {
        if rd::<u8>(this, 0x40) != 0 && rd::<u32>(this, 0x28) == 0 {
            return 1;
        }
        if v_ik_create_chain(this, ped) == 0 {
            return 1;
        }
        let t = g::<i32>(TIMER_TIME_MS).read();
        let d = rd::<i32>(this, 8);
        wr::<i32>(this, 0x48, if d == -1 { -1 } else { d.wrapping_add(t) });
        wr::<u32>(this, 0x4C, 0x3F80_0000);
        wr::<i32>(this, 0x50, rd::<i32>(this, 0xC).wrapping_add(t));
        ik_chain_set_blend(rd::<GPtr<u8>>(this, 0x10), rd::<u32>(this, 0x44));
        return 0;
    }
    if rd::<i32>(this, 8) != -1 && now > rd::<i32>(this, 0x48) {
        ik_manager_remove_ik_chain(g::<u8>(IK_CHAIN_MAN), chain);
        wr::<u32>(this, 0x10, 0);
        return 1;
    }
    if rd::<u8>(this, 0x40) != 0 && rd::<u32>(this, 0x28) == 0 {
        wr::<u8>(this, 0x40, 0);
        ik_chain_update_target(chain, 0);
        ik_chain_task_blend_out(this, 0xFA);
    }
    if rd::<i32>(this, 8) != -1 {
        let end = rd::<i32>(this, 0x48);
        if now >= end.wrapping_sub(rd::<i32>(this, 0xC)) {
            wr::<u32>(this, 0x4C, 0);
            wr::<i32>(this, 0x50, end);
        }
    }
    let bend = rd::<i32>(this, 0x50);
    if now > bend {
        let tb = rd::<u32>(this, 0x4C);
        wr::<u32>(this, 0x44, tb);
        ik_chain_set_blend(rd::<GPtr<u8>>(this, 0x10), tb);
        return 0;
    }
    let div = bend.wrapping_sub(step).wrapping_sub(now);
    let mut f = X87::from_i32(step).fdiv(X87::from_i32(div));
    if !f.fcom(X87::one()).test_any(0x41) {
        f = X87::one();
    }
    let cur = rd::<f32>(this, 0x44);
    let nb = x(rd::<f32>(this, 0x4C)).fsub_m(cur).fmul(f).fadd_m(cur).to_f32();
    wr::<f32>(this, 0x44, nb);
    ik_chain_set_blend(rd::<GPtr<u8>>(this, 0x10), nb.to_bits());
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_IK_LOOK_AT_CREATE_IK_CHAIN, HOOK_IK_CHAIN_PROCESS_PED];
