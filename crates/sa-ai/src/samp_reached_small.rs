//! Small AI functions the original executes under SA:MP 0.3.7 (`catalog/samp-priority.csv`): the `vt_85b4c8` event's
//! `TakesPriorityOver` / scalar deleting destructor / `Clone`, three bool-selected float getters used by
//! `CTaskSimpleUseGun::ProcessPed`, `CTaskSimpleThrowProjectile::ControlThrow` / `FinishAnimThrowProjectileCB`,
//! `CTaskSimpleIKChain::BlendOut`, `CTaskComplexDie::SayDeathSample` and the `CTaskSimpleDead` constructor
//! (PORT-AI-7). Ported from the 1.0 US bytes (the exe wins over gta-reversed). Every call to another catalogued
//! function goes through `orig!` at the original address.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;

orig! {
    /// `CEvent::GetEventType` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CEvent::GetEventPriority` (vtable +8)
    fn v_event_priority(e: GPtr<u8>) -> i32 = thiscall vslot 0x8 => unported;
    /// `CEvent::~CEvent` (`FUN_004b13c0`: `vtbl = 0x85AB38`)
    fn event_base_dtor(e: GPtr<u8>) = thiscall 0x4B13C0 => unported;
    /// `CEventPool::New` (0x4B5570, on the pool in ECX)
    fn event_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B5570 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, `ret 4`)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CPed::Say(uint16 id, uint32 offset, float volume, bool, bool, bool)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, off: u32, vol: u32, a: u32, b: u32, c: u32) = thiscall 0x5EFFE0 => unported;
    /// `CLocalisation::Blood` (0x56D230, AL)
    fn localisation_blood() -> u8 = cdecl 0x56D230 => unported;
    /// `CAEPedAudioEntity::AddAudioEvent(int32 event, float volume, float speed, CPhysical*, uint8, int32, uint32)`
    /// (0x4E2BB0, `ret 0x1C`)
    #[allow(clippy::too_many_arguments)]
    fn ped_audio_add_audio_event(a: GPtr<u8>, ev: u32, vol: u32, speed: u32, ent: GPtr<u8>, b: u32, c: u32, d: u32) = thiscall 0x4E2BB0 => unported;
    /// `CTaskSimple::CTaskSimple` (`FUN_0061a390`)
    fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => unported;
}

// ---- vt_85b4c8 (an event with a float at +0xC) -------------------------------------------------------------------

/// `vt_85b4c8::TakesPriorityOver(const CEvent&)` (0x4B1420, `ret 4`): AL = 1 against events of type 8 or 9, and
/// against type 0x5D when this event's +0xC is above [0x858624] (`test ah, 0x41; je`); otherwise AL = `own priority
/// >= other priority` (signed). One `GetEventType` call per test, in order.
#[hook(addr = 0x4B1420, conv = thiscall, name = "vt_85b4c8::TakesPriorityOver", expect = "56 8B 74 24 08",
       unverified = "ai7-10", route = "L2", hazards = "ax-al-return",
       note = "the original's upper EAX bytes are the own priority's (callers test AL)")]
pub fn vt_85b4c8_takes_priority_over(this: GPtr<u8>, other: GPtr<u8>) -> u8 {
    if v_event_type(other) == 8 || v_event_type(other) == 9 {
        return 1;
    }
    if v_event_type(other) == 0x5D && !x(rd::<f32>(this, 0xC)).fcom(x(g::<f32>(0x858624).read())).test_any(0x41) {
        return 1;
    }
    let theirs = v_event_priority(other);
    let mine = v_event_priority(this);
    u8::from(mine >= theirs)
}

/// `vt_85b4c8` scalar deleting destructor (0x4B6610, `ret 4`): the base destructor, then with flag bit 0 the
/// inlined event-pool delete (slot `(this - objects) / 0x44`, flag byte |= 0x80, first-free = min). Returns `this`.
#[hook(addr = 0x4B6610, conv = thiscall, name = "vt_85b4c8::Destructor", expect = "56 E8 AA AD FF FF",
       unverified = "ai7-10", route = "L2", hazards = "heap-writes")]
pub fn vt_85b4c8_destructor(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    event_base_dtor(this);
    if flags & 1 != 0 {
        let pool = g::<GPtr<u8>>(EVENT_POOL).read();
        let idx = (this.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32 / 0x44;
        let f = at::<u8>(rd::<GPtr<u8>>(pool, 4), idx as isize);
        f.write(f.read() | 0x80);
        if idx < rd::<i32>(pool, 0xC) {
            wr::<i32>(pool, 0xC, idx);
        }
    }
    this
}

/// `vt_85b4c8::Clone` (0x4B7810): a new event-pool slot (null when the pool is full) with time 0 (+4), +8 = 0,
/// +0xC copied through the x87 (`fld; fstp`), `vtbl = 0x85B4C8`.
#[hook(addr = 0x4B7810, conv = thiscall, name = "vt_85b4c8::Clone", expect = "56 8B F1 8B 0D AC 44 B7 00",
       unverified = "ai7-10", route = "L2", hazards = "heap-writes")]
pub fn vt_85b4c8_clone(this: GPtr<u8>) -> GPtr<u8> {
    let e = event_pool_new(g::<GPtr<u8>>(EVENT_POOL).read());
    if e.is_null() {
        return e;
    }
    wr::<u32>(e, 4, 0);
    wr::<f32>(e, 0xC, x(rd::<f32>(this, 0xC)).to_f32());
    wr::<u8>(e, 8, 0);
    wr::<u32>(e, 0, 0x85B4C8);
    e
}

// ---- float getters selected by a bool (callers: CTaskSimpleUseGun::ProcessPed) -----------------------------------

/// `FUN_00608f00(bool)` (0x608F00, `ret 4`): ST0 = +0x4C when the flag byte is set, else +0x40.
#[hook(addr = 0x608F00, conv = thiscall, name = "FUN_00608f00", expect = "8A 44 24 04 84 C0",
       unverified = "ai7-10", route = "L2", hazards = "none")]
pub fn f_608f00(this: GPtr<u8>, b: u32) -> X87 {
    x(rd::<f32>(this, if b as u8 != 0 { 0x4C } else { 0x40 }))
}

/// `FUN_00608f20(bool)` (0x608F20, `ret 4`): ST0 = +0x50 when the flag byte is set, else +0x44.
#[hook(addr = 0x608F20, conv = thiscall, name = "FUN_00608f20", expect = "8A 44 24 04 84 C0",
       unverified = "ai7-10", route = "L2", hazards = "none")]
pub fn f_608f20(this: GPtr<u8>, b: u32) -> X87 {
    x(rd::<f32>(this, if b as u8 != 0 { 0x50 } else { 0x44 }))
}

/// `FUN_0061c150(bool)` (0x61C150, `ret 4`): ST0 = +0x54 when the flag byte is set, else +0x48.
#[hook(addr = 0x61C150, conv = thiscall, name = "FUN_0061c150", expect = "8A 44 24 04 84 C0",
       unverified = "ai7-10", route = "L2", hazards = "none")]
pub fn f_61c150(this: GPtr<u8>, b: u32) -> X87 {
    x(rd::<f32>(this, if b as u8 != 0 { 0x54 } else { 0x48 }))
}

// ---- CTaskSimpleThrowProjectile ----------------------------------------------------------------------------------

/// `CTaskSimpleThrowProjectile::FinishAnimThrowProjectileCB(CAnimBlendAssociation*, void* task)` (0x61F890, cdecl):
/// drops the association (+0xC); the first finish sets +9 (thrown), a later one +8 (finished).
#[hook(addr = 0x61F890, conv = cdecl, name = "CTaskSimpleThrowProjectile::FinishAnimThrowProjectileCB",
       expect = "8B 44 24 08 8A 50 09", unverified = "ai7-10", route = "L2", hazards = "heap-writes")]
pub fn throw_projectile_finish_anim_cb(_assoc: GPtr<u8>, task: GPtr<u8>) {
    let thrown = rd::<u8>(task, 9);
    wr::<u32>(task, 0xC, 0);
    if thrown == 0 {
        wr::<u8>(task, 9, 1);
    } else {
        wr::<u8>(task, 8, 1);
    }
}

/// `CTaskSimpleThrowProjectile::ControlThrow(bool, CEntity*, const CVector*)` (0x61F810, `ret 0xC`): AL = 1 once
/// finished (+8). Otherwise the first `true` call turns the start time (+0x20) into the hold duration (`now - start`)
/// and sets +0xA; a new non-null target entity replaces +0x10 (references updated); a target point is copied to
/// +0x14; AL = 0.
#[hook(addr = 0x61F810, conv = thiscall, name = "CTaskSimpleThrowProjectile::ControlThrow", expect = "57 8B F9 8A 47 08",
       unverified = "ai7-10", route = "L2", hazards = "ax-al-return,heap-writes")]
pub fn throw_projectile_control_throw(this: GPtr<u8>, release: u32, target: GPtr<u8>, point: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 8) != 0 {
        return 1;
    }
    if release as u8 != 0 && rd::<u8>(this, 0xA) == 0 {
        wr::<u32>(this, 0x20, g::<u32>(TIME_MS).read().wrapping_sub(rd::<u32>(this, 0x20)));
        wr::<u8>(this, 0xA, 1);
    }
    if !target.is_null() {
        let slot = this.byte_add(0x10);
        let old = rd::<GPtr<u8>>(slot, 0);
        if target != old {
            if !old.is_null() {
                entity_clean_up_old_reference(old, slot);
            }
            wr::<GPtr<u8>>(slot, 0, target);
            entity_register_reference(target, slot);
        }
    }
    if !point.is_null() {
        for k in 0..3 {
            wr::<u32>(this, 0x14 + k * 4, rd::<u32>(point, k * 4));
        }
    }
    0
}

// ---- CTaskSimpleIKChain ------------------------------------------------------------------------------------------

/// `CTaskSimpleIKChain::BlendOut(int32 time)` (0x633C40, `ret 4`): unless already blending out (+0x54), an
/// unlimited (-1) end time (+8) becomes 0, the blend-out end (+0x48) is `now + time` and +0x54 is set.
#[hook(addr = 0x633C40, conv = thiscall, name = "CTaskSimpleIKChain::BlendOut", expect = "8B 41 54 85 C0",
       unverified = "ai7-10", route = "L2", hazards = "heap-writes")]
pub fn ik_chain_blend_out(this: GPtr<u8>, time: i32) {
    if rd::<u32>(this, 0x54) != 0 {
        return;
    }
    if rd::<i32>(this, 8) == -1 {
        wr::<u32>(this, 8, 0);
    }
    wr::<u32>(this, 0x48, g::<u32>(TIME_MS).read().wrapping_add(time as u32));
    wr::<u32>(this, 0x54, 1);
}

// ---- CTaskComplexDie / CTaskSimpleDead ---------------------------------------------------------------------------

/// `CTaskComplexDie::SayDeathSample(CPed*)` (0x630100, `ret 4`): by the weapon type (+0xC; jump table 0x6301BC
/// over 0x31..0x36): 0x31 / 0x32 say 0x156; 0x33 nothing; 0x35 says 0x155; 0x36 says 0x156 and, with blood
/// enabled, adds ped audio event 0x77 (volume 0, speed 1.0, the ped); 0x34 and everything else say 0x157.
#[hook(addr = 0x630100, conv = thiscall, name = "CTaskComplexDie::SayDeathSample", expect = "8B 49 0C 8D 41 CF",
       unverified = "ai7-10", route = "L2", hazards = "heap-writes")]
pub fn die_say_death_sample(this: GPtr<u8>, ped: GPtr<u8>) {
    let say = |id: u32| ped_say(ped, id, 0, 0x3F80_0000, 0, 0, 0);
    match rd::<u32>(this, 0xC).wrapping_sub(0x31) {
        0 | 1 => say(0x156),
        2 => {}
        4 => say(0x155),
        5 => {
            say(0x156);
            if localisation_blood() != 0 {
                ped_audio_add_audio_event(ped.byte_add(0x138), 0x77, 0, 0x3F80_0000, ped, 0, 0, 0);
            }
        }
        _ => say(0x157),
    }
}

/// `CTaskSimpleDead::CTaskSimpleDead(int32 deathTime, bool)` (0x630590, `ret 8`): `CTaskSimple` base, +8 = the
/// time, flag byte +0xC = (old & 0xF9) | (b & 1) << 1 | 1, `vtbl = 0x86DEA4`.
#[hook(addr = 0x630590, conv = thiscall, name = "CTaskSimpleDead::CTaskSimpleDead", expect = "56 8B F1 E8 F8 9D FE FF",
       unverified = "ai7-10", route = "L2", hazards = "heap-writes")]
pub fn simple_dead_ctor(this: GPtr<u8>, time: u32, b: u32) -> GPtr<u8> {
    task_simple_ctor(this);
    let f = ((b as u8 & 1) << 1) | (rd::<u8>(this, 0xC) & 0xF9) | 1;
    wr::<u32>(this, 8, time);
    wr::<u32>(this, 0, 0x86DEA4);
    wr::<u8>(this, 0xC, f);
    this
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_VT_85B4C8_TAKES_PRIORITY_OVER,
    HOOK_VT_85B4C8_DESTRUCTOR,
    HOOK_VT_85B4C8_CLONE,
    HOOK_F_608F00,
    HOOK_F_608F20,
    HOOK_F_61C150,
    HOOK_THROW_PROJECTILE_FINISH_ANIM_CB,
    HOOK_THROW_PROJECTILE_CONTROL_THROW,
    HOOK_IK_CHAIN_BLEND_OUT,
    HOOK_DIE_SAY_DEATH_SAMPLE,
    HOOK_SIMPLE_DEAD_CTOR,
];
