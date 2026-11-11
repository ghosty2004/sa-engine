//! PORT-AI-4: fleeing — `CTaskComplexSmartFleePoint`, `CTaskComplexSmartFleeEntity`.
//! Ported from the 1.0 US bytes (gta-reversed and `docs/re/specs/AI` as reading aids only).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `g_ikChainMan` (`IKChainManager_c`)
const IK_CHAIN_MAN: usize = 0xC15448;

orig! {
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_get_task_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: u32, ev: u32) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence()` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10, `ret 4`)
    fn sequence_add_task(s: GPtr<u8>, t: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskSimpleRunAnim::CTaskSimpleRunAnim(AssocGroupId, AnimationId, float, bool)` (0x61A8B0, `ret 0x10`)
    fn run_anim_ctor(t: GPtr<u8>, group: u32, id: u32, delta: u32, hold: u32) -> GPtr<u8> = thiscall 0x61A8B0 => unported;
    /// `CTaskSimpleTired::CTaskSimpleTired(int32)` (0x630F20, `ret 4`)
    fn tired_ctor(t: GPtr<u8>, time: u32) -> GPtr<u8> = thiscall 0x630F20 => unported;
    /// `CTaskComplexSmartFleePoint::SetDefaultTaskWanderDir(CPed*)` (0x65BE00, `ret 4`)
    fn flee_point_set_default_task_wander_dir(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x65BE00 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32, bool, bool, float)` (0x62F310, `ret 0x10`)
    fn stand_still_ctor(t: GPtr<u8>, time: u32, looped: u32, keep: u32, delta: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskComplexWander::CTaskComplexWander(int32 moveState, uint8 dir, bool, float)` (0x66F450, `ret 0x10`)
    fn wander_ctor(t: GPtr<u8>, ms: u32, dir: u32, a3: u32, a4: u32) -> GPtr<u8> = thiscall 0x66F450 => unported;
    /// `CTaskComplexSmartFleePoint::CreateSubTask(int32, CPed*)` (0x65BE80, `ret 8`)
    fn flee_point_create_sub_task(t: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x65BE80 => unported;
    /// `CGeneral::GetNodeHeadingFromVector(float, float)` (0x53CDC0, AL)
    fn general_get_node_heading_from_vector(x: f32, y: f32) -> u8 = cdecl 0x53CDC0 => unported;
    /// `CPed::Say(id, offset, volume, a4, a5, a6)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, id: u32, off: u32, vol: u32, a4: u32, a5: u32, a6: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CTaskComplexSmartFleePoint::ComputeFleeDir(CPed*)` (0x65BE40, `ret 4`, AL)
    fn flee_point_compute_flee_dir(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x65BE40 => unported;
    /// `FUN_00669d30`: the wander task's direction setter (thiscall, `ret 4`)
    fn wander_set_dir(t: GPtr<u8>, dir: u32) = thiscall 0x669D30 => unported;
    /// `CTaskTimer::IsOutOfTime()` (0x420E30, AL)
    fn task_timer_is_out_of_time(t: GPtr<u8>) -> u8 = thiscall 0x420E30 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn general_get_random_number_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(this: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::AbortLookAt(CPed*, int32)` (0x618280, `ret 8`)
    fn ik_abort_look_at(this: GPtr<u8>, ped: GPtr<u8>, time: u32) = thiscall 0x618280 => unported;
    /// `IKChainManager_c::LookAt(...)` (0x618970, `ret 0x2C`)
    fn ik_look_at(
        this: GPtr<u8>, purpose: u32, ped: GPtr<u8>, target: u32, time: u32, bone: u32, posn: GPtr<u8>,
        torso: u32, speed: u32, blend: u32, prio: u32, force: u32
    ) -> u32 = thiscall 0x618970 => unported;
    /// `CInterestingEvents::Add(type, CEntity*)` (0x602590, `ret 8`)
    fn interesting_events_add(this: GPtr<u8>, ty: u32, e: GPtr<u8>) = thiscall 0x602590 => unported;
    /// `CTaskComplexSmartFleeEntity::CreateSubTask(int32)` (0x65C530, `ret 4`)
    fn flee_entity_create_sub_task(t: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x65C530 => unported;
    /// `CTaskComplexSmartFleePoint::CTaskComplexSmartFleePoint(const CVector&, bool, float, int32)` (0x65BD20,
    /// `ret 0x10`)
    fn flee_point_ctor(t: GPtr<u8>, pos: GPtr<u8>, scream: u32, radius: u32, time: u32) -> GPtr<u8> = thiscall 0x65BD20 => unported;
    /// `CTaskComplexSmartFleePoint::SetFleePosition(const CVector&, float, bool)` (0x65C3C0, `ret 0xC`)
    fn flee_point_set_flee_position(t: GPtr<u8>, pos: GPtr<u8>, radius: u32, scream: u32) = thiscall 0x65C3C0 => unported;
    /// `CEventSeenPanickedPed::CEventSeenPanickedPed(CPed*)` (0x4B2080, `ret 4`)
    fn seen_panicked_ped_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B2080 => unported;
    /// `FUN_004b2130`: the `CEventSeenPanickedPed` destructor the original calls on its stack event
    fn seen_panicked_ped_dtor(e: GPtr<u8>) = thiscall 0x4B2130 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, flag: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
}

/// Pointer to a Rust stack buffer (stands in for an object the original builds on its own stack).
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// Entity position: matrix +0x30, else the placement at +4.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn ik() -> GPtr<u8> {
    GPtr::from_addr(IK_CHAIN_MAN)
}

fn time_ms() -> u32 {
    g::<u32>(TIME_MS).read()
}

/// `(dz·dz + dy·dy) + dx·dx` of `a - p` as the original's x87 sequence computes it (unrounded differences).
fn dist2(a: GPtr<u8>, p: GPtr<u8>) -> X87 {
    let dx = x(rd::<f32>(a, 0)).fsub_m(rd::<f32>(p, 0));
    let dy = x(rd::<f32>(a, 4)).fsub_m(rd::<f32>(p, 4));
    let dz = x(rd::<f32>(a, 8)).fsub_m(rd::<f32>(p, 8));
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx))
}

// ---- CTaskComplexSmartFleePoint -------------------------------------------------------------------------

/// `CTaskComplexSmartFleePoint::CreateSubTask(int32 type, CPed*)` (0x65BE80, `ret 8`): 0xCB → `SetDefaultTaskWander
/// Dir`, `CTaskSimpleStandStill(0, false, false, 8)`; 0xF4 → for move state 6 (+0x30) a sequence of
/// `CTaskSimpleRunAnim(0, 0xA5, 4, false)` and `CTaskSimpleTired(2000)` (a failed allocation adds null, as the
/// original), else null; 0x2CA → the inlined 0x14-byte task with vtable 0x858EB0 (+0xC = 0, +0x10 = 0, +0x11 = 1);
/// 0x390 → `CTaskComplexWander(move state, dir +0x40, false, [0x86FCB0])` with vtable 0x86F690 (`WanderFlee`);
/// anything else (0x516 included) null.
#[hook(addr = 0x65BE80, conv = thiscall, name = "CTaskComplexSmartFleePoint::CreateSubTask", expect = "64 A1 00 00 00 00")]
pub fn flee_point_create_sub_task_port(this: GPtr<u8>, ty: i32, ped: GPtr<u8>) -> GPtr<u8> {
    match ty {
        0xCB => {
            flee_point_set_default_task_wander_dir(this, ped);
            let t = task_new(0x20);
            if t.is_null() { GPtr::NULL } else { stand_still_ctor(t, 0, 0, 0, 0x4100_0000) }
        }
        0xF4 => {
            if rd::<u32>(this, 0x30) != 6 {
                return GPtr::NULL;
            }
            let s = task_new(0x40);
            let s = if s.is_null() { GPtr::NULL } else { sequence_ctor(s) };
            let a = task_new(0x20);
            let a = if a.is_null() { GPtr::NULL } else { run_anim_ctor(a, 0, 0xA5, 0x4080_0000, 0) };
            sequence_add_task(s, a);
            let t = task_new(0x1C);
            let t = if t.is_null() { GPtr::NULL } else { tired_ctor(t, 0x7D0) };
            sequence_add_task(s, t);
            s
        }
        0x2CA => {
            let t = task_new(0x14);
            if t.is_null() {
                return GPtr::NULL;
            }
            task_complex_ctor(t);
            wr::<u32>(t, 0, 0x858EB0);
            wr::<u32>(t, 0xC, 0);
            wr::<u8>(t, 0x10, 0);
            wr::<u8>(t, 0x11, 1);
            t
        }
        0x390 => {
            let t = task_new(0x28);
            if t.is_null() {
                return GPtr::NULL;
            }
            let k = g::<u32>(0x86FCB0).read();
            let ms = rd::<u32>(this, 0x30);
            let dir = rd::<u8>(this, 0x40) as u32;
            wander_ctor(t, ms, dir, 0, k);
            wr::<u32>(t, 0, 0x86F690);
            t
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexSmartFleePoint::CreateFirstSubTask(CPed*)` (0x65C140, `ret 4`): the start position +0xC = the ped's
/// position; a ped in a vehicle (+0x46C bit 0x100 and +0x58C) leaves it first (0x2CA); otherwise the wander
/// direction +0x40 = `GetNodeHeadingFromVector(f32(x - flee x), f32(y - flee y))` and a flee wander (0x390).
#[hook(addr = 0x65C140, conv = thiscall, name = "CTaskComplexSmartFleePoint::CreateFirstSubTask", expect = "56 8B 74 24 08")]
pub fn flee_point_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let p = ent_pos(ped);
    wr::<u32>(this, 0xC, rd::<u32>(p, 0));
    wr::<u32>(this, 0x10, rd::<u32>(p, 4));
    wr::<u32>(this, 0x14, rd::<u32>(p, 8));
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 && rd::<u32>(ped, 0x58C) != 0 {
        return flee_point_create_sub_task(this, 0x2CA, ped);
    }
    let p = ent_pos(ped);
    let dy = f32_of(x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(this, 0x1C)));
    let dx = f32_of(x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(this, 0x18)));
    let d = general_get_node_heading_from_vector(dx, dy);
    wr::<u8>(this, 0x40, d);
    flee_point_create_sub_task(this, 0x390, ped)
}

/// `CTaskComplexSmartFleePoint::ControlSubTask(CPed*)` (0x65C1E0, `ret 4`). Not a flee wander (0x390): quit any
/// look-at, keep the sub-task. Otherwise: scream (`Say(0x15B, 0, 0.1)`) while +0x24; the wander's move state = +0x30;
/// a new flee position (+0x41) restarts a running timer (+0x34 = now, +0x38 = +0x28) and re-aims the wander when
/// `ComputeFleeDir` changed (+0x40); else a timed-out timer (`CTaskTimer::IsOutOfTime`) or being farther than the
/// radius (+0x2C, squared as f32) from both the flee point (+0x18) and the start (+0xC) switches to stand still
/// (0xCB). Sprinting (state 6) peds not looking at anything look back at the flee point with a 6% chance
/// (`GetRandomNumberInRange(0, 100) <= 5`).
#[hook(addr = 0x65C1E0, conv = thiscall, name = "CTaskComplexSmartFleePoint::ControlSubTask", expect = "56 8B F1 57 8B 7E 08")]
pub fn flee_point_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut res = rd::<GPtr<u8>>(this, 8);
    if v_get_task_type(res) != 0x390 {
        if ik_is_looking(ik(), ped) != 0 {
            ik_abort_look_at(ik(), ped, 0xFA);
        }
        return rd::<GPtr<u8>>(this, 8);
    }
    if rd::<u8>(this, 0x24) != 0 {
        ped_say(ped, 0x15B, 0, 0x3DCC_CCCD, 0, 0, 0);
    }
    let sub = rd::<GPtr<u8>>(this, 8);
    wr::<u32>(sub, 0xC, rd::<u32>(this, 0x30));
    if rd::<u8>(this, 0x41) != 0 {
        let started = rd::<u8>(this, 0x3C) != 0;
        wr::<u8>(this, 0x41, 0);
        if started {
            let t = rd::<u32>(this, 0x28);
            wr::<u32>(this, 0x34, time_ms());
            wr::<u32>(this, 0x38, t);
            wr::<u8>(this, 0x3C, 1);
        }
        let d = flee_point_compute_flee_dir(this, ped);
        if rd::<u8>(this, 0x40) != d {
            wr::<u8>(this, 0x40, d);
            wander_set_dir(sub, d as u32);
        }
        res = rd::<GPtr<u8>>(this, 8);
    } else if rd::<u8>(this, 0x3C) != 0 && task_timer_is_out_of_time(this.byte_add(0x34)) != 0 {
        res = flee_point_create_sub_task(this, 0xCB, ped);
    } else {
        let p = ent_pos(ped);
        let r = x(rd::<f32>(this, 0x2C));
        let r2 = f32_of(r.fmul(r));
        if dist2(this.byte_add(0x18), p).fcom(x(r2)).test_any(0x41) {
            res = rd::<GPtr<u8>>(this, 8);
        } else if !dist2(this.byte_add(0xC), p).fcom(x(r2)).test_any(0x41) {
            res = flee_point_create_sub_task(this, 0xCB, ped);
        }
    }
    if rd::<u32>(this, 0x30) == 6 && ik_is_looking(ik(), ped) == 0 && general_get_random_number_in_range(0, 0x64) <= 5 {
        ik_look_at(ik(), 0x86F770, ped, 0, 0x7D0, 0xFFFF_FFFF, this.byte_add(0x18), 0, 0x3E80_0000, 0x1F4, 3, 0);
    }
    res
}

// ---- CTaskComplexSmartFleeEntity ------------------------------------------------------------------------

/// `CTaskComplexSmartFleeEntity::CreateSubTask(int32 type)` (0x65C530, `ret 4`; the SEH handler push continues
/// through the island 0x401D31): 0xCB → `CTaskSimpleStandStill(_ftol((rand() & 0xFFFF)·[0x858B14]·[0x858B40]),
/// false, false, 8)` (rand at 0x65C637, only when the pool slot was allocated); 0x38E → timer (+0x34 = now, +0x38
/// = +0x28, started), the flee position +0x10 = the entity's position, `CTaskComplexSmartFleePoint(&+0x10, +0x24,
/// +0x20, +0x1C)`; anything else null.
#[hook(addr = 0x65C530, conv = thiscall, name = "CTaskComplexSmartFleeEntity::CreateSubTask", expect = "6A FF E9 FA 57 DA FF")]
pub fn flee_entity_create_sub_task_port(this: GPtr<u8>, ty: u32) -> GPtr<u8> {
    match ty {
        0xCB => {
            let t = task_new(0x20);
            if t.is_null() {
                return GPtr::NULL;
            }
            let r = sa_core::orig::at_site(0x65C637, rand) & 0xFFFF;
            let ms = ftol32(xi(r).fmul_m(g::<f32>(0x858B14).read()).fmul_m(g::<f32>(0x858B40).read()));
            stand_still_ctor(t, ms as u32, 0, 0, 0x4100_0000)
        }
        0x38E => {
            let t = rd::<u32>(this, 0x28);
            wr::<u32>(this, 0x34, time_ms());
            wr::<u32>(this, 0x38, t);
            wr::<u8>(this, 0x3C, 1);
            let p = ent_pos(rd::<GPtr<u8>>(this, 0xC));
            wr::<u32>(this, 0x10, rd::<u32>(p, 0));
            wr::<u32>(this, 0x14, rd::<u32>(p, 4));
            wr::<u32>(this, 0x18, rd::<u32>(p, 8));
            let n = task_new(0x44);
            if n.is_null() {
                return GPtr::NULL;
            }
            let (a, b) = (rd::<u32>(this, 0x1C), rd::<u32>(this, 0x20));
            flee_point_ctor(n, this.byte_add(0x10), rd::<u8>(this, 0x24) as u32, b, a)
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexSmartFleeEntity::CreateFirstSubTask(CPed*)` (0x65C6F0, `ret 4`): null without an entity (+0xC);
/// else an interesting event 0xD for the ped, a 3 s look at the entity (bone 5 for a ped entity, type bits of
/// +0x36 == 3, else -1) unless in a vehicle or already looking, and the flee-point sub-task (0x38E).
#[hook(addr = 0x65C6F0, conv = thiscall, name = "CTaskComplexSmartFleeEntity::CreateFirstSubTask", expect = "57 8B F9 8B 47 0C")]
pub fn flee_entity_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(this, 0xC) == 0 {
        return GPtr::NULL;
    }
    interesting_events_add(GPtr::from_addr(0xC0B058), 0xD, ped);
    if rd::<u32>(ped, 0x46C) & 0x100 == 0 && ik_is_looking(ik(), ped) == 0 {
        let e = rd::<GPtr<u8>>(this, 0xC);
        let bone = if rd::<u8>(e, 0x36) & 7 == 3 { 5 } else { 0xFFFF_FFFF };
        ik_look_at(ik(), 0x86F7B0, ped, e.addr() as u32, 0xBB8, bone, GPtr::NULL, 1, 0x3E80_0000, 0x1F4, 3, 0);
    }
    flee_entity_create_sub_task(this, 0x38E)
}

/// `CTaskComplexSmartFleeEntity::ControlSubTask(CPed*)` (0x65C780, `ret 4`): without an entity the sub-task is asked
/// to abort (`MakeAbortable(ped, 0, null)`); a flee-point sub-task (0x38E) gets the move state (+0x30 → sub +0x30)
/// and, once the started timer (+0x34/+0x38, restart flag +0x3D) expired and the entity moved farther than +0x2C
/// from the stored position ((dz² + dy²) + dx² > r², unrounded), the timer restarts, the position is refreshed and
/// passed on (`SetFleePosition(&+0x10, +0x20, +0x24)`), and a running ped (state > 4) raises a stack
/// `CEventSeenPanickedPed` in the global event group. Always the current sub-task.
#[hook(addr = 0x65C780, conv = thiscall, name = "CTaskComplexSmartFleeEntity::ControlSubTask", expect = "64 A1 00 00 00 00")]
pub fn flee_entity_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    if rd::<u32>(this, 0xC) == 0 {
        v_make_abortable(sub, ped, 0, 0);
        return rd::<GPtr<u8>>(this, 8);
    }
    if v_get_task_type(sub) != 0x38E {
        return rd::<GPtr<u8>>(this, 8);
    }
    wr::<u32>(rd::<GPtr<u8>>(this, 8), 0x30, rd::<u32>(this, 0x30));
    if rd::<u8>(this, 0x3C) == 0 {
        return rd::<GPtr<u8>>(this, 8);
    }
    if rd::<u8>(this, 0x3D) != 0 {
        wr::<u32>(this, 0x34, time_ms());
        wr::<u8>(this, 0x3D, 0);
    }
    let end = rd::<u32>(this, 0x38).wrapping_add(rd::<u32>(this, 0x34));
    let now = time_ms();
    if now < end {
        return rd::<GPtr<u8>>(this, 8);
    }
    let ep = ent_pos(rd::<GPtr<u8>>(this, 0xC));
    let r = x(rd::<f32>(this, 0x2C));
    if r.fmul(r).fcom(dist2(this.byte_add(0x10), ep)).test_parity(5) {
        return rd::<GPtr<u8>>(this, 8);
    }
    let t = rd::<u32>(this, 0x28);
    wr::<u32>(this, 0x34, now);
    wr::<u32>(this, 0x38, t);
    wr::<u8>(this, 0x3C, 1);
    let p = ent_pos(rd::<GPtr<u8>>(this, 0xC));
    wr::<u32>(this, 0x10, rd::<u32>(p, 0));
    wr::<u32>(this, 0x14, rd::<u32>(p, 4));
    wr::<u32>(this, 0x18, rd::<u32>(p, 8));
    let scream = rd::<u8>(this, 0x24) as u32;
    flee_point_set_flee_position(sub, this.byte_add(0x10), rd::<u32>(this, 0x20), scream);
    if rd::<i32>(this, 0x30) > 4 {
        let mut ev = [0u32; 0x11];
        let ep = buf(&mut ev);
        seen_panicked_ped_ctor(ep, ped);
        event_group_add(get_event_global_group(), ep, 0);
        seen_panicked_ped_dtor(ep);
    }
    rd::<GPtr<u8>>(this, 8)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_FLEE_POINT_CREATE_SUB_TASK_PORT,
    HOOK_FLEE_POINT_CREATE_FIRST_SUB_TASK,
    HOOK_FLEE_POINT_CONTROL_SUB_TASK,
    HOOK_FLEE_ENTITY_CREATE_SUB_TASK_PORT,
    HOOK_FLEE_ENTITY_CREATE_FIRST_SUB_TASK,
    HOOK_FLEE_ENTITY_CONTROL_SUB_TASK,
];
