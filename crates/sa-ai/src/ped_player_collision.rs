//! `CEventHandler::ComputePedCollisionWithPlayerResponse` (batch `intro-ai-4`): a ped bumped into by the player.
//! Not reversed in gta-reversed; ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::IsGoToTask(CTask*)` (0x61A360, static)
    fn task_is_go_to_task(t: GPtr<u8>) -> u8 = cdecl 0x61A360 => unported;
    /// `CTaskManager::GetActiveTask` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn tm_find_active_task_by_type(tm: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0, `ret 4`)
    fn intel_find_task_by_type(i: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CPedGroupMembership::CountMembers` (0x5F6A50)
    fn membership_count_members(m: GPtr<u8>) -> i32 = thiscall 0x5F6A50 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::Say(context, offset, volume, b, b, b)` (0x5EFFE0, `ret 0x18`)
    fn ped_say(p: GPtr<u8>, ctx: u32, offset: u32, vol: u32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CGeneral::GetRandomNumberInRange<float>(min, max)` (0x41BD90; draws its own rand, ST0)
    fn general_random_float_in_range(min: f32, max: f32) -> f64 = cdecl 0x41BD90 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::Magnitude` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CVector::Normalise` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `IKChainManager_c::LookAt(...)` (0x618970, `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: u32, torso: u32,
                  speed: u32, blend: i32, prio: i32, b: u32) = thiscall 0x618970 => unported;
    /// `CPedGeometryAnalyser::ComputeRouteRoundEntityBoundingBox(ped, entity, target, route, n)` (0x5F6110)
    fn geometry_compute_route_round_entity_bbox(ped: GPtr<u8>, e: GPtr<u8>, target: GPtr<u8>, route: GPtr<u8>, n: i32) -> u8 = cdecl 0x5F6110 => unported;
    /// `FUN_004bc470` (the seek task's move-state radius setter; `misc.rs`)
    fn seek_set_move_state_radius(t: GPtr<u8>, r: f32) = thiscall 0x4BC470 => crate::misc::fun_004bc470;
    fn say_ctor(t: GPtr<u8>, ctx: i32, time: i32) -> GPtr<u8> = thiscall 0x48E360 => unported;
    fn evasive_step_ctor(t: GPtr<u8>, e: GPtr<u8>, dir: GPtr<u8>) -> GPtr<u8> = thiscall 0x6532D0 => unported;
    fn follow_point_route_ctor(t: GPtr<u8>, ms: i32, route: GPtr<u8>, mode: i32, radius: u32, slow: u32, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x671510 => unported;
}

/// `g_ikChainMan`
const IK_CHAIN_MANAGER: usize = 0xC15448;
/// "CompPedCollPlayerResp"
const S_COMP_PED_COLL_PLAYER_RESP: u32 = 0x85BA70;
/// The player group's membership (`CPedGroups::ms_groups[0]` + 8)
const PLAYER_GROUP_MEMBERSHIP: usize = 0xC09928;
const K_ZERO: usize = 0x858B50;
/// 0.25 / 0.33 (f32) and 0.66 (a double, compared as `fcomp qword`)
const K_LOOK_CHANCE: usize = 0x858C84;
const K_LOOK_ONLY: usize = 0x85AD74;
const K_SAY_F64: usize = 0x85BA68;
const K_RADIUS_PAD: usize = 0x858B1C;
const K_FOLLOW_DIST: usize = 0x86F828;
const K_ROUTE_RADIUS: usize = 0x86FC98;
const K_ROUTE_SLOW: usize = 0x86FC9C;

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn ped(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0)
}

fn intel(p: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(p, 0x47C)
}

fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let t = task_new(size);
    if t.is_null() { t } else { ctor(t) }
}

fn say(this: GPtr<u8>) {
    wr::<GPtr<u8>>(this, 0x2C, new_task(0x1C, |t| say_ctor(t, 0x1C, -1)));
}

fn look_at(this: GPtr<u8>, player: GPtr<u8>) {
    ik_look_at(GPtr::from_addr(IK_CHAIN_MANAGER), S_COMP_PED_COLL_PLAYER_RESP, ped(this), player, 2000, 5, 0, 1, 0x3E80_0000, 500, 3, 0);
}

/// `CTaskComplexEvasiveStep(player, &dir)` with `dir = (-forward.y, forward.x, 0)` of the player's matrix, normalised.
fn evasive_step(player: GPtr<u8>) -> GPtr<u8> {
    let f = rd::<GPtr<u8>>(player, 0x14).byte_add(0x10);
    let mut v = [f32_of(x(rd::<f32>(f, 4)).fchs()).to_bits(), f32_of(x(rd::<f32>(f, 0))).to_bits(), 0u32];
    let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
    vector_normalise(vp);
    new_task(0x1C, |t| evasive_step_ctor(t, player, vp))
}

/// The tasks `ComputePedCollisionWithPlayerResponse` looks up once.
struct Tasks {
    follower: GPtr<u8>,
    avoid: GPtr<u8>,
    t384: GPtr<u8>,
    seek: GPtr<u8>,
    group_no_target: bool,
    player_moving: bool,
}

/// The outcome of the gang follower update (0x4BEBAA / 0x4BEF2E / 0x4BF12E): it ran, it was skipped (a follower
/// without a target in an idle player group: nothing), or the caller's fallback applies.
enum Follow {
    Updated,
    Skip,
    Fallback,
}

fn follow(this: GPtr<u8>, t: &Tasks) -> Follow {
    if t.follower.is_null() {
        return Follow::Fallback;
    }
    let complete = !t.seek.is_null() && (!t.avoid.is_null() || !t.t384.is_null());
    if !complete || !t.group_no_target || t.player_moving {
        if t.group_no_target && !t.player_moving {
            return Follow::Skip;
        }
        return Follow::Fallback;
    }
    let lead = rd::<GPtr<u8>>(t.follower, 0x10);
    let mut v = [0u32; 3];
    let vp: GPtr<u8> = GPtr::from_raw(v.as_mut_ptr()).cast();
    vector_sub(vp, pos_ptr(ped(this)), pos_ptr(lead));
    wr::<u32>(vp, 8, 0);
    let dist = f32_of(X87::from_f64_exact(vector_magnitude(vp)));
    if !t.avoid.is_null() {
        wr::<u8>(t.avoid, 0x5C, rd::<u8>(t.avoid, 0x5C) | 2);
    } else if !t.t384.is_null() {
        wr::<f32>(t.t384, 0x18, f32_of(x(dist).fadd_m(gf(K_RADIUS_PAD))));
    }
    seek_set_move_state_radius(t.seek, f32_of(x(dist).fadd_m(gf(K_RADIUS_PAD))));
    if !x(dist).fcom(x(gf(K_FOLLOW_DIST))).test_parity(0x05) {
        for (i, c) in v.iter().enumerate() {
            wr::<u32>(t.follower, 0x20 + 4 * i as isize, *c);
        }
    }
    Follow::Updated
}

/// `CEventHandler::ComputePedCollisionWithPlayerResponse(CEvent*, CTask* active, CTask* simplest)` (0x4BE7D0,
/// thiscall, `ret 0xC`; not reversed in gta-reversed). Event +0x14 the player (nothing without it), +0x30 the ped's
/// move state (i16). `inFront = d·myForward >= 0`, `facing = d·playerForward <= 0` (d = player − ped, x87); the
/// player moving = its move state (+0x534) 4 / 6 / 7. Tasks: gang follower (0x4B7), active avoid-ped (0x395), 0x384,
/// seek (0x38B). A task 0xF3 without a target (+0xC) marks `groupNoTarget`; then a player group with at most 2
/// members ends the function (no +0x24 write), else `bigGroup`. `Say(0x1C, 0, 1.0, 0, 0, 0)`. "say" = +0x2C =
/// `CTaskSimpleSay(0x1C, -1)`, "look" = `LookAt("CompPedCollPlayerResp", ped, player, 2000, 5, NULL, true, 0.25, 500,
/// 3, false)`, "evade" = `CTaskComplexEvasiveStep(player, (-pf.y, pf.x, 0) normalised)`.
/// - `active` of type 0x391: a player → say; else (the player's active task 0x391) `rand < 0.25` → a second rand:
///   below 0.33 look, else below 0.66 (double) say. Then `facing` and `active +0x10` the player → the follower update
///   (B); else the tail (C).
/// - Else `inFront` (with or without `facing`; not `inFront` and not `facing`: the tail (C)): a move state other than
///   4 / 6 / 7 → say, look, then `bigGroup` → (B), else +0x24 = evade. Move state 4 / 6 / 7: the follower update (A):
///   done or skipped, else with a go-to `simplest` a route round the player (`CTaskComplexFollowPointRoute`, the
///   response; `bigGroup` cleared); then say, look and the tail (C). Not `inFront` but `facing` → say, look, `bigGroup`
///   → (B), else evade.
/// - The follower update: a follower with a seek task and an avoid or 0x384 task, `groupNoTarget` and the player not
///   moving → `dist` = 2D ped − leader; avoid +0x5C |= 2, else 0x384 +0x18 = dist + 0.1; the seek radius = dist +
///   0.1; `dist < 8` → follower +0x20 = the vector. `groupNoTarget` with the player still → nothing.
/// - (C): `bigGroup` → (B); else +0x24 = the response. (B): the follower update, or nothing as above, else evade.
#[hook(addr = 0x4be7d0, conv = thiscall, name = "CEventHandler::ComputePedCollisionWithPlayerResponse", expect = "6A FF 68 56 B4 83 00",
       this = "CEventHandler", unverified = "intro-ai-4", route = "L2", hazards = "none", recs = "grove-walk")]
#[allow(clippy::too_many_lines)]
pub fn handler_compute_ped_collision_with_player_response(this: GPtr<u8>, e: GPtr<u8>, active: GPtr<u8>, simplest: GPtr<u8>) {
    let player = rd::<GPtr<u8>>(e, 0x14);
    if player.is_null() {
        return;
    }
    let p = ped(this);
    let mp = pos_ptr(p);
    let op = pos_ptr(player);
    let dx = x(rd::<f32>(op, 0)).fsub_m(rd::<f32>(mp, 0));
    let dy = x(rd::<f32>(op, 4)).fsub_m(rd::<f32>(mp, 4));
    let dz = x(rd::<f32>(op, 8)).fsub_m(rd::<f32>(mp, 8));
    let dot = |m: GPtr<u8>| {
        let s = dz.fmul_m(rd::<f32>(m, 0x18));
        let s = s.fadd(dy.fmul_m(rd::<f32>(m, 0x14)));
        s.fadd(dx.fmul_m(rd::<f32>(m, 0x10)))
    };
    let in_front = !dot(rd::<GPtr<u8>>(p, 0x14)).fcom(x(gf(K_ZERO))).c0();
    let facing = !dot(rd::<GPtr<u8>>(player, 0x14)).fcom(x(gf(K_ZERO))).test_parity(0x41);
    let pms = rd::<i32>(player, 0x534);
    let my_ms = i32::from(rd::<i16>(e, 0x30));
    let mut tasks = Tasks {
        follower: intel_find_task_by_type(intel(ped(this)), 0x4B7),
        avoid: GPtr::NULL,
        t384: GPtr::NULL,
        seek: GPtr::NULL,
        group_no_target: false,
        player_moving: pms == 4 || pms == 6 || pms == 7,
    };
    tasks.avoid = tm_find_active_task_by_type(intel(ped(this)).byte_add(4), 0x395);
    tasks.t384 = intel_find_task_by_type(intel(ped(this)), 0x384);
    tasks.seek = intel_find_task_by_type(intel(ped(this)), 0x38B);
    let mut big_group = false;
    let f3 = intel_find_task_by_type(intel(ped(this)), 0xF3);
    if !f3.is_null() && rd::<u32>(f3, 0xC) == 0 {
        tasks.group_no_target = true;
        if membership_count_members(GPtr::from_addr(PLAYER_GROUP_MEMBERSHIP)) <= 2 {
            return;
        }
        big_group = true;
    }
    ped_say(ped(this), 0x1C, 0, 0x3F80_0000, 0, 0, 0);
    let mut result = GPtr::<u8>::NULL;
    // `true` = continue with (B), `false` = the tail (C); evade / direct writes return.
    let to_b: bool = 'body: {
        if !active.is_null() && v_task_type(active) == 0x391 {
            if ped_is_player(player) != 0 {
                say(this);
            } else {
                let tm = intel(player).byte_add(4);
                if !tm_get_active_task(tm).is_null()
                    && v_task_type(tm_get_active_task(tm)) == 0x391
                    && !X87::from_f64_exact(general_random_float_in_range(0.0, 1.0)).fcom(x(gf(K_LOOK_CHANCE))).test_parity(0x05)
                {
                    let r = X87::from_f64_exact(general_random_float_in_range(0.0, 1.0));
                    if !r.fcom(x(gf(K_LOOK_ONLY))).test_parity(0x05) {
                        look_at(this, player);
                    } else if !r.fcom(X87::from_f64_exact(g::<f64>(K_SAY_F64).read())).test_parity(0x05) {
                        say(this);
                    }
                }
            }
            break 'body facing && rd::<GPtr<u8>>(active, 0x10) == player;
        }
        if !in_front {
            if !facing {
                break 'body false;
            }
            say(this);
            look_at(this, player);
            if big_group {
                break 'body true;
            }
            wr::<GPtr<u8>>(this, 0x24, evasive_step(player));
            return;
        }
        if !(my_ms == 4 || my_ms == 6 || my_ms == 7) {
            say(this);
            look_at(this, player);
            if big_group {
                break 'body true;
            }
            wr::<GPtr<u8>>(this, 0x24, evasive_step(player));
            return;
        }
        if let Follow::Fallback = follow(this, &tasks)
            && !simplest.is_null()
            && task_is_go_to_task(simplest) != 0
        {
            {
                let mut route = [0u32; 25];
                let rp: GPtr<u8> = GPtr::from_raw(route.as_mut_ptr()).cast();
                geometry_compute_route_round_entity_bbox(ped(this), player, simplest.byte_add(0xC), rp, 0);
                result = new_task(0x3C, |t| {
                    follow_point_route_ctor(t, 4, rp, 0, g::<u32>(K_ROUTE_RADIUS).read(), g::<u32>(K_ROUTE_SLOW).read(), 0, 1, 1)
                });
                big_group = false;
            }
        }
        say(this);
        look_at(this, player);
        false
    };
    if !to_b && !big_group {
        wr::<GPtr<u8>>(this, 0x24, result);
        return;
    }
    match follow(this, &tasks) {
        Follow::Updated | Follow::Skip => wr::<GPtr<u8>>(this, 0x24, result),
        Follow::Fallback => wr::<GPtr<u8>>(this, 0x24, evasive_step(player)),
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_HANDLER_COMPUTE_PED_COLLISION_WITH_PLAYER_RESPONSE];
