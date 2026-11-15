//! FRONTIER-2 (batch `frontier2-1`): `CTaskSimpleFightingControl::ProcessPed` (0x62A0A0) and `CalcMoveCommand`
//! (0x624B50), the AI ped's fight loop the melee scenario reaches once a victim fights back. Ported from the 1.0 US
//! bytes (gta-reversed has not reversed either).

use sa_core::abi::HookReg;
use sa_core::field::gf;
use sa_core::bindings::{
    general_random_int_in_range, random_float_in_range, task_new, tm_get_task_secondary, tm_set_task_secondary,
};
use sa_core::crt::rand_at;
use sa_core::{hook, orig};
use sa_math::fpu::{fabs, gt, le_or_nan, lt};
use sa_core::bindings::{vector_difference, vector_normalise};
use sa_world::bindings::{intel_get_task_fighting, vector_ctor_iterator};
use sa_world::world::common::entity_pos;

use crate::bindings::{bbox_planes, compute_entity_hit_side, fight_ctor, task_fight_control_fight};
use crate::common::*;

/// `CTimer::m_snTimeInMilliseconds`, `CTimer::ms_fTimeStep`.
const TIME_MS: usize = 0xB7_CB84;
const TIME_STEP: usize = 0xB7_CB5C;

orig! {
    /// `CPed::GiveWeaponAtStartOfFight()` (0x5E8AB0)
    fn give_weapon_at_start_of_fight(p: GPtr<u8>) = thiscall 0x5E8AB0 => unported;
    /// `CTaskSimpleFightingControl::CalcMoveCommand(CPed*)` (0x624B50, int16 in AX)
    fn calc_move_command_o(t: GPtr<u8>, ped: GPtr<u8>) -> i16 = thiscall 0x624B50 => calc_move_command;
    /// virtual `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// virtual `CTask::MakeAbortable(CPed*, int32, const CEvent*)` (vtable +0x18)
    fn v_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, prio: i32, ev: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// virtual `CEvent::GetEventType()` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// virtual `CEvent::GetEventPriority()` (vtable +8)
    fn v_event_priority(e: GPtr<u8>) -> i32 = thiscall vslot 0x8 => unported;
}

/// `ftol(timeStep * [0x858B38] * [0x858C4C])`: the frame's milliseconds.
fn step_ms() -> i32 {
    ftol32(x(gf(TIME_STEP)).fmul_m(gf(0x85_8B38)).fmul_m(gf(0x85_8C4C)))
}

/// `CTaskSimpleFightingControl::CalcMoveCommand(CPed* ped)` (0x624B50, `ret 4`, int16 in AX).
///
/// No target (+0xC): -1. The heading error to the target (`atan2(-dx, dy) - heading`, wrapped once into
/// [-pi, pi]) above `[0x8D2E60]` turns the ped (+0x1C += the frame's ms) and gives 0. A ped target: one `rand`
/// draw decides the rare moves (`(r & 15) == 0`); `dist - range (+0x14)` above `[0x858B1C]` → 3, above `[0x858EF4]`
/// → 7 when rare else -1; nearer than `[0x858C98]` → 9 when rare else -1; else one in 64 → 8, then one in 64 → 10.
/// A vehicle target: 3 when the ped is outside the box plane of its hit side by more than `[0x858CB0]`. Anything
/// else: 3 when `dist - range > [0x858C24]`.
#[hook(addr = 0x624B50, conv = thiscall, name = "CTaskSimpleFightingControl::CalcMoveCommand", expect = "83 EC 40 57 8B F9 8B 57",
       unverified = "frontier2-1", route = "L2", hazards = "x87-transcendental,stack-residue",
       note = "rand at_site 0x624C30/0x624CA8/0x624CBD; the distance is kept in the argument slot; AX only (EAX's high half is not reproduced)")]
pub fn calc_move_command(this: GPtr<u8>, ped: GPtr<u8>) -> i16 {
    let target = rd::<GPtr<u8>>(this, 0xC);
    if target.is_null() {
        return -1;
    }
    let pp = entity_pos(ped);
    let tp = entity_pos(target);
    let dx = x(rd::<f32>(tp, 0)).fsub_m(rd::<f32>(pp, 0));
    let dy = x(rd::<f32>(tp, 4)).fsub_m(rd::<f32>(pp, 4));
    let dz = x(rd::<f32>(tp, 8)).fsub_m(rd::<f32>(pp, 8));
    let dist = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy)).fsqrt().to_f32();
    let mut a = dx.fchs().fpatan(dy).fsub_m(rd::<f32>(ped, 0x558));
    if !le_or_nan(a, x(gf(0x85_8CB8))) {
        a = a.fsub_m(gf(0x85_8CBC));
    } else if lt(a, x(gf(0x85_8CC0))) {
        a = a.fadd_m(gf(0x85_8CBC));
    }
    if gt(fabs(a), x(gf(0x8D_2E60))) {
        wr(this, 0x1C, rd::<i32>(this, 0x1C).wrapping_add(step_ms()));
        return 0;
    }
    match rd::<u8>(target, 0x36) & 7 {
        3 => {
            let rare = rand_at(0x62_4C30) & 0xF == 0;
            let d = x(dist).fsub_m(rd::<f32>(this, 0x14));
            if gt(d, x(gf(0x85_8B1C))) {
                return 3;
            }
            if gt(d, x(gf(0x85_8EF4))) {
                return if rare { 7 } else { -1 };
            }
            if lt(x(dist), x(gf(0x85_8C98))) {
                return if rare { 9 } else { -1 };
            }
            if rand_at(0x62_4CA8) & 0x3F == 0 {
                return 8;
            }
            if rand_at(0x62_4CBD) & 0x3F == 0 {
                return 10;
            }
            -1
        }
        2 => {
            let mut normals = [0f32; 12];
            let mut dists = [0f32; 4];
            let np: GPtr<u8> = GPtr::from_raw(normals.as_mut_ptr()).cast();
            vector_ctor_iterator(np, 0xC, 4, 0x40_4350);
            let t = rd::<GPtr<u8>>(this, 0xC);
            let tz = rd::<u32>(entity_pos(t), 8);
            bbox_planes(tz, t, np, GPtr::from_raw(dists.as_mut_ptr()).cast());
            let side = compute_entity_hit_side(entity_pos(ped), rd::<GPtr<u8>>(this, 0xC)) as usize;
            let p = entity_pos(ped);
            let n = &normals[3 * side..3 * side + 3];
            let dot = x(n[2])
                .fmul_m(rd::<f32>(p, 8))
                .fadd(x(n[1]).fmul_m(rd::<f32>(p, 4)))
                .fadd(x(n[0]).fmul_m(rd::<f32>(p, 0)))
                .fadd_m(dists[side]);
            if le_or_nan(dot, x(gf(0x85_8CB0))) { -1 } else { 3 }
        }
        _ => {
            if gt(x(dist).fsub_m(rd::<f32>(this, 0x14)), x(gf(0x85_8C24))) { 3 } else { -1 }
        }
    }
}

/// `CTaskSimpleFightingControl::ProcessPed(CPed* ped)` (0x62A0A0, `ret 4`).
///
/// Done (true) without a target (+0xC) or when aborting (+8). The fight skill is the ped's +0x719 as a float (a
/// non-mission (+0x484 != 2), non-type-6 ped with skill 0x28 takes its stats' +0x30 word). `GiveWeaponAtStartOfFight`.
/// No secondary task: a new `CTaskSimpleFight(target, 0, ftol([0x8D2E50]))` becomes the secondary task, +0x1C = 0.
/// A secondary task other than a fight (0x3F8) is made abortable (urgent), false. Otherwise the command: 0 by
/// default; the attack timer (+0x1C) expired → cleared, 0xB (0xC with an empty current weapon slot unless +0x72D is
/// 4); else a ped target without a block timer (+0x20) may start one (`RandomFloat(0, 100) < [0x8D2E54] * skill *
/// [0x859038]` → `RandomInt([0x8D2E58], [0x8D2E5C])`, command 2), a running block timer counts down by the frame's ms
/// (command 2). The fight's range (+0x14) comes from its combo row; an idle timer (+0x1C == 0) is re-armed to `now +
/// ftol((rand * [0x858C7C] + [0x858C84]) / (skill * +0x18 * [0x859038] * [0x858CB0] + [0x858C24]) * (u32)[0x8D2E4C])`.
/// Combo sets 0 / 1 (signed) take `CalcMoveCommand` when it is not negative. `ControlFight(target, command)`; false.
#[hook(addr = 0x62A0A0, conv = thiscall, name = "CTaskSimpleFightingControl::ProcessPed", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes,stack-residue",
       note = "rand at_site 0x62A2E1; the skill float is kept in the argument slot; the SEH frame is not reproduced")]
#[allow(clippy::too_many_lines)]
pub fn fighting_control_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0xC) == 0 || rd::<u8>(this, 8) != 0 {
        return 1;
    }
    let skill_b = rd::<u8>(ped, 0x719);
    let mut skill = xi(i32::from(skill_b)).to_f32();
    if rd::<u8>(ped, 0x484) != 2 && rd::<u32>(ped, 0x598) != 6 && skill_b == 0x28 {
        skill = xi(i32::from(rd::<u16>(rd::<GPtr<u8>>(ped, 0x59C), 0x30))).to_f32();
    }
    give_weapon_at_start_of_fight(ped);
    let tm = || rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
    let mut cmd: i32 = 0;
    if tm_get_task_secondary(tm(), 0).is_null() {
        let t = task_new(0x28);
        let f = if t.is_null() {
            GPtr::NULL
        } else {
            let idle = ftol32(x(gf(0x8D_2E50)));
            fight_ctor(t, rd::<GPtr<u8>>(this, 0xC), 0, idle as u32)
        };
        tm_set_task_secondary(tm(), f, 0);
        wr::<u32>(this, 0x1C, 0);
    } else {
        if v_task_type(tm_get_task_secondary(tm(), 0)) != 0x3F8 {
            v_make_abortable(tm_get_task_secondary(tm(), 0), ped, 1, GPtr::NULL);
            return 0;
        }
        if rd::<u32>(this, 0x1C) <= g::<u32>(TIME_MS).read() {
            wr::<u32>(this, 0x1C, 0);
            cmd = 0xB;
            if rd::<u8>(ped, 0x72D) != 4 {
                let slot = isize::from(rd::<i8>(ped, 0x718));
                if rd::<u32>(ped, 0x5A0 + slot * 0x1C) == 0 {
                    cmd = 0xC;
                }
            }
        } else {
            let target = rd::<GPtr<u8>>(this, 0xC);
            let mut started = false;
            if rd::<u8>(target, 0x36) & 7 == 3 && rd::<u32>(this, 0x20) == 0 {
                let r = random_float_in_range(0.0, 100.0);
                let k = x(gf(0x8D_2E54)).fmul_m(skill).fmul_m(gf(0x85_9038));
                if !le_or_nan(k, r) {
                    let lo = g::<i32>(0x8D_2E58).read();
                    let hi = g::<i32>(0x8D_2E5C).read();
                    wr(this, 0x20, general_random_int_in_range(lo, hi));
                    started = true;
                    cmd = 2;
                }
            }
            if !started {
                let left = rd::<u32>(this, 0x20);
                if left != 0 {
                    let t = step_ms() as u32;
                    wr::<u32>(this, 0x20, left.saturating_sub(t));
                    cmd = 2;
                }
            }
        }
    }
    let fight = tm_get_task_secondary(tm(), 0);
    let c = i32::from(rd::<i8>(fight, 0x24)) - 4;
    let row = (if c < 0 { 0 } else { c } as usize) * 0x88;
    wr(this, 0x14, x(g::<f32>(0xC1_70D4 + row).read()).to_f32());
    if rd::<u32>(this, 0x1C) == 0 {
        let r = rand_at(0x62_A2E1);
        let base = g::<u32>(0x8D_2E4C).read();
        wr::<u32>(this, 0x1C, base);
        let a = xi(r).fmul_m(gf(0x85_8C7C)).fadd_m(gf(0x85_8C84));
        let b = x(skill)
            .fmul_m(rd::<f32>(this, 0x18))
            .fmul_m(gf(0x85_9038))
            .fmul_m(gf(0x85_8CB0))
            .fadd_m(gf(0x85_8C24));
        let mut m = xi(base as i32);
        if (base as i32) < 0 {
            m = m.fadd_m(gf(0x85_8C54));
        }
        let t = ftol32(a.fdiv(b).fmul(m));
        wr::<u32>(this, 0x1C, (t as u32).wrapping_add(g::<u32>(TIME_MS).read()));
    }
    if rd::<i8>(fight, 0x24) <= 1 {
        let m = calc_move_command_o(this, ped);
        if m > -1 {
            cmd = i32::from(m);
        }
    }
    task_fight_control_fight(fight, rd::<GPtr<u8>>(this, 0xC), cmd as u32);
    0
}

/// `CTaskSimpleFightingControl::MakeAbortable(CPed* ped, int32 priority, const CEvent* event)` (0x61DD00, `ret 0xC`).
///
/// Priority 0: false; 2: true. Urgent (1) with an event: an event of priority (vslot 8) below 0x16 or of type (vslot
/// 4) 2 / 3 / 4 is refused; types 1 / 5 / 6 are refused when the event's direction (+0x18) points along the
/// ped-to-target direction (`-(n.y d.y + n.z d.z + n.x d.x) < [0x863C1C]`), and accepted without a target. Otherwise
/// a ped without a fight task gives true; else the secondary task's `MakeAbortable(ped, priority, event)`.
#[hook(addr = 0x61DD00, conv = thiscall, name = "CTaskSimpleFightingControl::MakeAbortable", expect = "83 EC 0C 55 8B 6C 24 18",
       unverified = "frontier2-1", route = "L2", hazards = "none")]
pub fn fighting_control_make_abortable(this: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 {
    if priority == 0 {
        return 0;
    }
    if priority == 2 {
        return 1;
    }
    if priority == 1 && !event.is_null() {
        if v_event_priority(event) < 0x16 {
            return 0;
        }
        if v_event_type(event) == 2 || v_event_type(event) == 3 || v_event_type(event) == 4 {
            return 0;
        }
        if v_event_type(event) == 1 || v_event_type(event) == 5 || v_event_type(event) == 6 {
            let target = rd::<GPtr<u8>>(this, 0xC);
            if target.is_null() {
                return 1;
            }
            let mut d = [0f32; 3];
            let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
            vector_difference(dp, entity_pos(target), entity_pos(ped));
            vector_normalise(dp.cast());
            let dot = x(d[1])
                .fmul_m(rd::<f32>(event, 0x1C))
                .fadd(x(d[2]).fmul_m(rd::<f32>(event, 0x20)))
                .fadd(x(d[0]).fmul_m(rd::<f32>(event, 0x18)))
                .fchs();
            if lt(dot, x(gf(0x86_3C1C))) {
                return 0;
            }
        }
    }
    if intel_get_task_fighting(rd::<GPtr<u8>>(ped, 0x47C)).is_null() {
        return 1;
    }
    let sec = tm_get_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0);
    v_make_abortable(sec, ped, priority, event)
}

pub const HOOKS: &[HookReg] = &[HOOK_CALC_MOVE_COMMAND, HOOK_FIGHTING_CONTROL_PROCESS_PED, HOOK_FIGHTING_CONTROL_MAKE_ABORTABLE];
