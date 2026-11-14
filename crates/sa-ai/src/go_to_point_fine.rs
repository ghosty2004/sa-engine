//! `CTaskSimpleGoToPointFine` (batch `frontier-1-1`): `ProcessPed` and `SetBlendedMoveAnim`, ported from the 1.0 US
//! bytes. Fields: +0x8 the move state, +0xC the target, +0x18 the radius, +0x20 the move ratio (0 still, 1 walk,
//! 2 run, 3 sprint; blended in between).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_world::world::common::entity_pos;

use crate::common::*;

/// The blend-in fields of an anim the original sets up after `AddAnimation`: blend amount (+0x18) 0, speed (+0x24) 1.
fn add_anim(ped: GPtr<u8>, id: u32) -> GPtr<u8> {
    let a = anim_add_animation(rd::<GPtr<u8>>(ped, 0x18), rd::<u32>(ped, 0x4D4), id);
    wr::<u32>(a, 0x18, 0);
    wr::<u32>(a, 0x24, 0x3F80_0000);
    a
}

/// `delete a` when non-null (the scalar deleting destructor, flag 1).
fn delete(a: GPtr<u8>) {
    if !a.is_null() {
        virtual_delete(a, 1);
    }
}

/// `CTaskSimpleGoToPointFine::SetBlendedMoveAnim(CPed*)` (0x65EF80, thiscall, `ret 4`). The ped's idle (3), walk (0),
/// run (1), sprint (2) and tired idle (0xA) anims, in that lookup order. Ducking with a duck task: the duck moves at
/// `(0, -min(ratio * 0.5, 1))`, done. `bResetWalkAnims` (+0x470 bit 0) restarts walk / run / sprint and is cleared.
/// Ratio exactly 0: idle blended in (4.0) when missing, walk / run / sprint deleted, ped +0x534 and +0x8 = 1. Else
/// the idle (restarting walk and run first) and the tired idle (blend delta -4) are deleted, then by ratio (compared
/// as doubles): <= 1 walk only (state 4); <= 2 walk `2 - r` and run `r - 1` (state 6); <= 3 run `3 - r` and sprint
/// `r - 2` (state 7); above (or NaN) nothing more. A blended anim is created when missing, flagged playing (+0x2E
/// bit 0) with blend delta 0; the others are deleted. Then `SetMoveState(state)`, ped +0x538 and +0x8 = state.
///
/// Stack: the original keeps the idle anim in its own `ped` argument slot; not reproduced.
#[hook(addr = 0x65EF80, conv = thiscall, name = "CTaskSimpleGoToPointFine::SetBlendedMoveAnim",
       this = "CTaskSimpleGoToPointFine", expect = "83 EC 0C 53 55 56",
       unverified = "frontier-1-1", route = "L2", hazards = "heap-writes",
       note = "the idle anim is kept in the ped argument slot by the original")]
#[allow(clippy::too_many_lines)]
pub fn go_to_point_fine_set_blended_move_anim(this: GPtr<u8>, ped: GPtr<u8>) {
    let clump = || rd::<GPtr<u8>>(ped, 0x18);
    let idle = clump_get_association(clump(), 3);
    let mut walk = clump_get_association(clump(), 0);
    let mut run = clump_get_association(clump(), 1);
    let mut sprint = clump_get_association(clump(), 2);
    let tired = clump_get_association(clump(), 0xA);
    let ratio = || rd::<f32>(this, 0x20);

    if rd::<u32>(ped, 0x46C) & 0x400_0000 != 0 && !intel_get_task_duck(rd::<GPtr<u8>>(ped, 0x47C), 0).is_null() {
        let v = x(ratio()).fmul_m(g::<f32>(0x85_8B8C).read());
        let one = g::<f32>(0x85_8624).read();
        let v = if v.fcom(x(one)).test_any(0x41) { v } else { x(one) };
        let my = v.fchs().to_f32();
        let duck = intel_get_task_duck(rd::<GPtr<u8>>(ped, 0x47C), 0);
        duck_control_duck_move(duck, 0.0, my);
        return;
    }
    if rd::<u8>(ped, 0x470) & 1 != 0 {
        for a in [walk, run, sprint] {
            if !a.is_null() {
                assoc_set_current_time(a, 0.0);
            }
        }
        wr::<u32>(ped, 0x470, rd::<u32>(ped, 0x470) & 0xFFFF_FFFE);
    }
    if !x(ratio()).fcom(x(g::<f32>(0x85_8B50).read())).test_parity(0x44) {
        if idle.is_null() {
            anim_blend_animation(clump(), rd::<u32>(ped, 0x4D4), 3, 4.0);
        }
        delete(walk);
        delete(run);
        delete(sprint);
        wr::<u32>(ped, 0x534, 1);
        wr::<u32>(this, 8, 1);
        return;
    }
    if !idle.is_null() {
        for a in [walk, run] {
            if !a.is_null() {
                assoc_set_current_time(a, 0.0);
            }
        }
        virtual_delete(idle, 1);
    }
    if !tired.is_null() {
        wr::<u32>(tired, 0x1C, 0xC080_0000);
        virtual_delete(tired, 1);
    }
    let le = |k: usize| !x(ratio()).fcom(X87::from_f64_exact(g::<f64>(k).read())).test_parity(0x41);
    let state;
    if le(0x85_A310) {
        if walk.is_null() {
            walk = add_anim(ped, 0);
        }
        wr::<u8>(walk, 0x2E, rd::<u8>(walk, 0x2E) | 1);
        wr::<u32>(walk, 0x18, 0x3F80_0000);
        wr::<u32>(walk, 0x1C, 0);
        delete(run);
        delete(sprint);
        state = 4;
    } else if le(0x86_F990) {
        if walk.is_null() {
            walk = add_anim(ped, 0);
        }
        wr::<u8>(walk, 0x2E, rd::<u8>(walk, 0x2E) | 1);
        let v = x(g::<f32>(0x85_8CA0).read()).fsub_m(ratio()).to_f32();
        wr::<u32>(walk, 0x1C, 0);
        wr::<f32>(walk, 0x18, v);
        if run.is_null() {
            run = add_anim(ped, 1);
        }
        wr::<u8>(run, 0x2E, rd::<u8>(run, 0x2E) | 1);
        let v = x(ratio()).fsub_m(g::<f32>(0x85_8624).read()).to_f32();
        wr::<u32>(run, 0x1C, 0);
        wr::<f32>(run, 0x18, v);
        delete(sprint);
        state = 6;
    } else if le(0x86_F988) {
        delete(walk);
        if run.is_null() {
            run = add_anim(ped, 1);
        }
        wr::<u16>(run, 0x2E, rd::<u16>(run, 0x2E) | 1);
        let v = x(g::<f32>(0x85_8B3C).read()).fsub_m(ratio()).to_f32();
        wr::<u32>(run, 0x1C, 0);
        wr::<f32>(run, 0x18, v);
        if sprint.is_null() {
            sprint = add_anim(ped, 2);
        }
        wr::<u16>(sprint, 0x2E, rd::<u16>(sprint, 0x2E) | 1);
        let v = x(ratio()).fsub_m(g::<f32>(0x85_8CA0).read()).to_f32();
        wr::<u32>(sprint, 0x1C, 0);
        wr::<f32>(sprint, 0x18, v);
        state = 7;
    } else {
        return;
    }
    ped_set_move_state(ped, state);
    wr::<u32>(ped, 0x538, state);
    wr::<u32>(this, 8, state);
}

/// `CTaskSimpleGoToPointFine::ProcessPed(CPed*)` (0x663540, `ret 4`). `d = target - position` (x, y stored as f32);
/// arrived when `radius² >= dy² + dx²`'s complement test fails (`fcompp; test ah, 1`: not below, or NaN) or the ped
/// has circled the target: `QuitIK`, then `SetMoveState(1)` + `SetMoveAnim`, `SetMoveState(+0x8)` + `SetMoveAnim`,
/// true. Otherwise `SetBlendedMoveAnim`, the aiming rotation (+0x55C) =
/// `LimitRadianAngle(GetRadianAngleBetweenPoints(dx, dy, 0, 0))` (the first result stored as f32), `SetUpIK`, false.
///
/// Stack: the original stores the f32 angle into its own `ped` argument slot; not reproduced.
#[hook(addr = 0x663540, conv = thiscall, name = "CTaskSimpleGoToPointFine::ProcessPed",
       this = "CTaskSimpleGoToPointFine", expect = "83 EC 0C 56 8B 74 24 14",
       unverified = "frontier-1-1", route = "L2", hazards = "heap-writes",
       note = "the angle is stored into the ped argument slot by the original")]
pub fn go_to_point_fine_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let p = entity_pos(ped);
    let dx = x(rd::<f32>(this, 0xC)).fsub_m(rd::<f32>(p, 0)).to_f32();
    let dy = x(rd::<f32>(this, 0x10)).fsub_m(rd::<f32>(p, 4)).to_f32();
    let r = x(rd::<f32>(this, 0x18));
    let sq = x(dx).fmul_m(dx).fadd(x(dy).fmul_m(dy));
    let outside = r.fmul(r).fcom(sq).c0();
    if !outside || goto_has_circled_target(this, ped) != 0 {
        goto_quit_ik(this, ped);
        ped_set_move_state(ped, 1);
        ped_set_move_anim(ped);
        ped_set_move_state(ped, rd::<u32>(this, 8));
        ped_set_move_anim(ped);
        return 1;
    }
    go_to_point_fine_set_blended_move_anim(this, ped);
    let a = X87::from_f64_exact(general_get_radian_angle_between_points(dx, dy, 0.0, 0.0)).to_f32();
    let a = X87::from_f64_exact(general_limit_radian_angle(a)).to_f32();
    wr::<f32>(ped, 0x55C, a);
    goto_set_up_ik(this, ped);
    0
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_GO_TO_POINT_FINE_SET_BLENDED_MOVE_ANIM, HOOK_GO_TO_POINT_FINE_PROCESS_PED];
