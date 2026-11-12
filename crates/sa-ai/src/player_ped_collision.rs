//! `CEventHandler::ComputePlayerCollisionWithPedResponse` (batch `intro-ai-4`): the player bumping into a ped.
//! Ported from the 1.0 US bytes (gta-reversed as a reading aid; the bytes win). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CTask::GetTaskType` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTaskManager::GetActiveTask` (0x681720)
    fn tm_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CPlayerPed::AnnoyPlayerPed(bool)` (0x60A040, `ret 4`)
    fn player_ped_annoy_player_ped(p: GPtr<u8>, b: u32) = thiscall 0x60A040 => unported;
    /// `IKChainManager_c::LookAt(...)` (0x618970, `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: u32, torso: u32,
                  speed: u32, blend: i32, prio: i32, b: u32) = thiscall 0x618970 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityHitSide(const CEntity&, const CEntity&)` (0x5F3BC0, cdecl)
    fn geometry_compute_entity_hit_side(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x5F3BC0 => unported;
    fn say_ctor(t: GPtr<u8>, ctx: i32, time: i32) -> GPtr<u8> = thiscall 0x48E360 => unported;
    fn fall_and_get_up_ctor2(t: GPtr<u8>, dir: i32, time: i32) -> GPtr<u8> = thiscall 0x678700 => unported;
    fn hit_response_ctor(t: GPtr<u8>, side: i32) -> GPtr<u8> = thiscall 0x631D70 => unported;
}

/// `g_ikChainMan`
const IK_CHAIN_MANAGER: usize = 0xC15448;
/// The look-at name of this response
const S_PLAYER_COLL_PED: u32 = 0x85B9CC;
/// 0.0
const K_ZERO: usize = 0x858B50;
/// 1.5, the stat (+0x2C of `CPedStat`) above which a ped knocks another over
const K_SHOVE_STAT: usize = 0x85AAE0;

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn ped(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0)
}

fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let t = task_new(size);
    if t.is_null() { t } else { ctor(t) }
}

fn look_at(p: GPtr<u8>, other: GPtr<u8>, time: i32) {
    ik_look_at(GPtr::from_addr(IK_CHAIN_MANAGER), S_PLAYER_COLL_PED, p, other, time, 5, 0, 1, 0x3E80_0000, 500, 3, 0);
}

/// The say task (`CTaskSimpleSay(0x1C, -1)`) stored in +0x2C.
fn say(this: GPtr<u8>) {
    wr::<GPtr<u8>>(this, 0x2C, new_task(0x1C, |t| say_ctor(t, 0x1C, -1)));
}

/// Both stats (+0x59C, `CPedStat` +0x2C) above 1.5 → the higher one knocks over; else the player's above 1.5 (x87,
/// `test ah, 0x41` / `jne`).
fn knocks_over(p: GPtr<u8>, other: GPtr<u8>) -> bool {
    let mine = rd::<f32>(rd::<GPtr<u8>>(p, 0x59C), 0x2C);
    let k = x(gf(K_SHOVE_STAT));
    let gt = |a: f32, b: X87| !x(a).fcom(b).test_any(0x41);
    if gt(mine, k) {
        let theirs = rd::<f32>(rd::<GPtr<u8>>(other, 0x59C), 0x2C);
        if gt(theirs, k) {
            return gt(mine, x(theirs));
        }
    }
    gt(mine, k)
}

/// `CEventHandler::ComputePlayerCollisionWithPedResponse(CEventPlayerCollisionWithPed*, CTask*, CTask*)` (0x4B8CE0,
/// thiscall, `ret 0xC`): the handler's ped is the player; event +0x14 the other ped (nothing without it), +0x30 /
/// +0x32 the two move states (i16). `inFront = d·myForward >= 0`, `facing = d·otherForward <= 0` (d = other − me,
/// x87 as `ComputePedCollisionWithPedResponse`). +0x24 = the response (NULL unless a fall / hit task is made); "say"
/// = +0x2C = `CTaskSimpleSay(0x1C, -1)`; "look" = `LookAt(0x85B9CC, player, other, 2000, 5, NULL, true, 0.25, 500,
/// 3, false)`; "annoy" = `AnnoyPlayerPed(false)` at the end.
/// - The other's active task of type 0x391: say only.
/// - (4, 4): `inFront` → look, say; else `facing` → look, say, annoy.
/// - (6|7, 4|6|7): not `inFront`: `facing` → say, look, annoy. `inFront`, not `facing` → say, look (no annoy). Both:
///   other 4 or not `knocks_over` → say, look, then (mine 7) `CTaskComplexHitResponse(ComputeEntityHitSide(other,
///   player))`, annoy; `knocks_over` → `CTaskComplexFallAndGetUp(side, 0)`, annoy.
/// - (0|1, 4|6|7) with `facing`: side = `ComputeEntityHitSide(other, player)`; other not 4 → the hit response; say,
///   look, annoy.
/// - (7, 0|1) with `inFront`: look (1000 ms), say.
#[hook(addr = 0x4b8ce0, conv = thiscall, name = "CEventHandler::ComputePlayerCollisionWithPedResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-4", route = "L2", hazards = "none", recs = "grove-walk")]
#[allow(clippy::too_many_lines)]
pub fn handler_compute_player_collision_with_ped_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let other = rd::<GPtr<u8>>(e, 0x14);
    if other.is_null() {
        return;
    }
    let p = ped(this);
    let mp = pos_ptr(p);
    let op = pos_ptr(other);
    let dx = x(rd::<f32>(op, 0)).fsub_m(rd::<f32>(mp, 0));
    let dy = x(rd::<f32>(op, 4)).fsub_m(rd::<f32>(mp, 4));
    let dz = x(rd::<f32>(op, 8)).fsub_m(rd::<f32>(mp, 8));
    let dot = |m: GPtr<u8>| {
        let s = dz.fmul_m(rd::<f32>(m, 0x18));
        let s = s.fadd(dy.fmul_m(rd::<f32>(m, 0x14)));
        s.fadd(dx.fmul_m(rd::<f32>(m, 0x10)))
    };
    let in_front = !dot(rd::<GPtr<u8>>(p, 0x14)).fcom(x(gf(K_ZERO))).c0();
    let facing = !dot(rd::<GPtr<u8>>(other, 0x14)).fcom(x(gf(K_ZERO))).test_parity(0x41);
    let mine = i32::from(rd::<i16>(e, 0x30));
    let theirs = i32::from(rd::<i16>(e, 0x32));
    let player = p;
    let mut result = GPtr::<u8>::NULL;
    let tm = rd::<GPtr<u8>>(other, 0x47C).byte_add(4);
    let fast = |ms: i32| ms == 6 || ms == 7;
    let slow = |ms: i32| ms == 0 || ms == 1;
    let annoy = 'body: {
        if !tm_get_active_task(tm).is_null() && v_task_type(tm_get_active_task(tm)) == 0x391 {
            say(this);
            break 'body false;
        }
        if mine == 4 {
            if theirs == 4 {
                if in_front {
                    look_at(ped(this), other, 2000);
                    say(this);
                    break 'body false;
                }
                if facing {
                    look_at(ped(this), other, 2000);
                    say(this);
                    break 'body true;
                }
            }
            break 'body false;
        }
        if fast(mine) && (theirs == 4 || fast(theirs)) {
            if !in_front {
                if !facing {
                    break 'body false;
                }
                say(this);
                look_at(ped(this), other, 2000);
                break 'body true;
            }
            if !facing {
                say(this);
                look_at(ped(this), other, 2000);
                break 'body false;
            }
            if theirs != 4 && knocks_over(ped(this), other) {
                let side = geometry_compute_entity_hit_side(other, ped(this));
                result = new_task(0x18, |t| fall_and_get_up_ctor2(t, side, 0));
                break 'body true;
            }
            say(this);
            look_at(ped(this), other, 2000);
            if mine == 7 {
                let side = geometry_compute_entity_hit_side(other, ped(this));
                result = new_task(0x10, |t| hit_response_ctor(t, side));
            }
            break 'body true;
        }
        if slow(mine) && (theirs == 4 || fast(theirs)) && facing {
            let side = geometry_compute_entity_hit_side(other, ped(this));
            if theirs != 4 {
                result = new_task(0x10, |t| hit_response_ctor(t, side));
            }
            say(this);
            look_at(ped(this), other, 2000);
            break 'body true;
        }
        if fast(mine) && slow(theirs) && in_front && mine == 7 {
            look_at(ped(this), other, 1000);
            say(this);
        }
        false
    };
    if annoy {
        player_ped_annoy_player_ped(player, 0);
    }
    wr::<GPtr<u8>>(this, 0x24, result);
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_HANDLER_COMPUTE_PLAYER_COLLISION_WITH_PED_RESPONSE];
