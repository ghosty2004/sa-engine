//! CCarCtrl steering, flying, weaving and slow-down (PORT-POP-3, helper A). Ported from the 1.0 US bytes;
//! gta-reversed and `docs/re/specs/POP/*.md` as reading aids. Long x87 sequences follow the original
//! instruction by instruction on [`Fpu`] (the literal register stack of sa-world's COLL ports).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_world::collision::fpu::Fpu;

use crate::common::{ftol32, g, rd, wr};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

pub const HOOKS: &[HookReg] = &[
    HOOK_AUTO_PILOT_MODIFY_SPEED,
    HOOK_TEST_COLLISION_BETWEEN_2_MOVING_RECTS,
    HOOK_TEST_COLLISION_BETWEEN_2_MOVING_RECTS_ONLY_FRONT_BUMPER,
    HOOK_SLOW_CAR_DOWN_FOR_OTHER_CAR,
    HOOK_SLOW_CAR_DOWN_FOR_CARS_SECTOR_LIST,
    HOOK_SLOW_CAR_DOWN_FOR_OBJECTS_SECTOR_LIST,
    HOOK_SLOW_CAR_DOWN_FOR_OBJECT,
    HOOK_FIND_MAXIMUM_SPEED_FOR_THIS_CAR_IN_TRAFFIC,
    HOOK_STEER_AI_CAR_WITH_PHYSICS,
    HOOK_STEER_AI_CAR_WITH_PHYSICS_ONLY_MISSION,
    HOOK_WEAVE_THROUGH_CARS_SECTOR_LIST,
    HOOK_WEAVE_THROUGH_PEDS_SECTOR_LIST,
    HOOK_WEAVE_THROUGH_OBJECTS_SECTOR_LIST,
    HOOK_FIND_ANGLE_TO_WEAVE_THROUGH_TRAFFIC,
    HOOK_WEAVE_FOR_OBJECT,
    HOOK_STEER_AI_CAR_WITH_PHYSICS_HEADING_FOR_TARGET,
    HOOK_STEER_AI_CAR_WITH_PHYSICS_TRYING_TO_BLOCK_TARGET,
    HOOK_FIND_GHOST_ROAD_HEIGHT,
    HOOK_STEER_AI_CAR_WITH_PHYSICS_TRYING_TO_BLOCK_TARGET_STOP,
    HOOK_STEER_AI_CAR_WITH_PHYSICS_FOLLOW_PATH,
    HOOK_GET_AI_HELI_TO_FLY_IN_DIRECTION,
    HOOK_STEER_AI_PLANE_TOWARDS_TARGET_COORS,
    HOOK_STEER_AI_PLANE_TO_FOLLOW_ENTITY,
    HOOK_GET_AI_PLANE_TO_DO_DOG_FIGHT_AGAINST_PLAYER,
    HOOK_STEER_AI_PLANE_TO_CRASH_AND_BURN,
    HOOK_STEER_AI_HELI_TO_CRASH_AND_BURN,
    HOOK_STEER_AI_HELI_FLYING_AWAY_FROM_PLAYER,
    HOOK_STEER_AI_BOAT_WITH_PHYSICS_HEADING_FOR_TARGET,
    HOOK_STEER_AI_BOAT_WITH_PHYSICS_CIRCLING_PLAYER,
    HOOK_STEER_AI_BOAT_WITH_PHYSICS_ATTACKING_PLAYER,
    HOOK_STEER_AI_HELI_TOWARDS_TARGET_COORS,
    HOOK_TEST_WHETHER_TO_FIRE_PLANE_GUNS,
    HOOK_GET_AI_PLANE_TO_ATTACK_PLAYER,
    HOOK_STEER_AI_HELI_AS_POLICE_HELI,
    HOOK_STEER_AI_HELI_TO_LAND,
    HOOK_GET_AI_HELI_TO_ATTACK_PLAYER,
    HOOK_POSSIBLY_FIRE_HS_MISSILE,
    HOOK_TRIGGER_DOG_FIGHT_MOVES,
    HOOK_GET_AI_PLANE_TO_DO_DOG_FIGHT,
    HOOK_FIRE_HELI_ROCKETS_AT_TARGET,
    HOOK_STEER_AI_CAR_BLOCKING_PLAYER_FORWARD_AND_BACK,
    HOOK_SLOW_CAR_DOWN_FOR_PEDS_SECTOR_LIST,
    HOOK_WEAVE_FOR_OTHER_CAR,
    HOOK_STEER_AI_HELI_TO_FOLLOW_ENTITY,
    HOOK_STEER_AI_HELI_TO_KEEP_ENTITY_IN_VIEW,
    HOOK_STEER_AI_CAR_TOWARDS_POINT_IN_ESCORT,
];

/// An f32 constant of the exe by its bits.
#[inline]
fn k(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `fidiv dword [m]`.
fn fidiv(f: &mut Fpu, v: i32) {
    let a = f.pop();
    f.push(a.fdiv(X87::from_i32(v)));
}

/// `fild` of a uint32 as the compiler emits it: `fild` the int32, `fadd 2^32` (0x858C54) when negative.
fn fild_u32(f: &mut Fpu, v: u32) {
    f.fild(v as i32);
    if (v as i32) < 0 {
        f.fadd_m(k(0x4F80_0000));
    }
}

/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84).
fn time_ms() -> u32 {
    g::<u32>(0xB7CB84).read()
}

/// `ThePaths.m_pPathNodes[area]` (0x96F854) is loaded.
fn area_loaded(area: u32) -> bool {
    g::<u32>((0x96F854 + area * 4) as usize).read() != 0
}

/// The navi node (`CCarPathLink`, 14 bytes) of a `CCarPathLinkAddress`: `ThePaths.m_pNaviNodes[a >> 10]`
/// (0x96F974) + `(a & 0x3FF) * 14`.
fn navi_node(a: u16) -> GPtr<u8> {
    let a = u32::from(a);
    g::<GPtr<u8>>((0x96F974 + (a >> 10) * 4) as usize)
        .read()
        .byte_add(((a & 0x3FF) * 14) as isize)
}

orig! {
    /// `CCarPathLink::OneWayLaneOffset()` (0x44DB00, ST0)
    fn one_way_lane_offset(link: GPtr<u8>) -> f64 = thiscall 0x44DB00 => one_way_lane_offset_abi;
    /// `CCurves::CalcSpeedScaleFactor(const CVector&, const CVector&, float, float, float, float)` (0x43C710, ST0)
    fn calc_speed_scale_factor(start: GPtr<f32>, end: GPtr<f32>, sdx: f32, sdy: f32, edx: f32, edy: f32) -> f64 = cdecl 0x43C710 => unported;
}

/// The `orig!` routing target of 0x44DB00 (an ST0 result crosses `orig!` as an f64).
#[cfg_attr(sa_phase_a, allow(dead_code))]
fn one_way_lane_offset_abi(link: GPtr<u8>) -> f64 {
    crate::pathfind::one_way_lane_offset(link).to_f64()
}

// ---------------------------------------------------------------------------------------------------------
// CAutoPilot

/// `CAutoPilot::ModifySpeed(float speed)` (0x41B980, thiscall, `ret 4`): +0x38 = `max(speed, 0.01)` (the
/// `fcom; test ah, 0x41; je` keeps `speed` only when it is greater); with both link areas loaded, the
/// current curve is recomputed: the two lane points (navi node coordinates × 0.125 plus the lane offset
/// `(OneWayLaneOffset + lane) * 5.4` along the perpendicular of the direction × 0.01 × the link
/// direction byte +0x25 / +0x26), `CalcSpeedScaleFactor`, +0x10 = `_ftol(f * (1000 / speed))` and
/// +0xC = `_ftol(time - T * progress)` with the progress `(time - +0xC) / +0x10` taken before
/// (times as uint32).
#[hook(addr = 0x41B980, conv = thiscall, name = "CAutoPilot::ModifySpeed", expect = "D9 44 24 04 83 EC 30", this = "CAutoPilot")]
pub fn auto_pilot_modify_speed(ap: GPtr<u8>, speed: f32) {
    let mut f = Fpu::new();
    f.fld(speed);
    f.fcom_m(k(0x3C23_D70A)); // 0.01
    if f.jne(0x41) {
        f.fstp_st(0);
        f.fld(k(0x3C23_D70A));
    }
    fild_u32(&mut f, time_ms().wrapping_sub(rd::<u32>(ap, 0xC)));
    fidiv(&mut f, rd::<i32>(ap, 0x10));
    let progress = f.fstp();
    wr::<f32>(ap, 0x38, f.fstp());
    let a1 = rd::<u16>(ap, 0x14);
    if !area_loaded(u32::from(a1) >> 10) {
        return;
    }
    let a2 = rd::<u16>(ap, 0x16);
    if !area_loaded(u32::from(a2) >> 10) {
        return;
    }
    let l1 = navi_node(a1);
    let l2 = navi_node(a2);
    let c = k(0x3C23_D70A); // 0.01
    let dir = |l: GPtr<u8>, o: isize, lane: i32| xi_(i32::from(rd::<i8>(l, o))).fmul_m(c).fmul(xi_(lane));
    let lane1 = i32::from(rd::<i8>(ap, 0x25));
    let lane2 = i32::from(rd::<i8>(ap, 0x26));
    let d1x = dir(l1, 8, lane1).to_f32();
    let d1y = dir(l1, 9, lane1).to_f32();
    let d2x = dir(l2, 8, lane2).to_f32();
    let d2y = dir(l2, 9, lane2).to_f32();
    let o1 = X87::from_f64_exact(one_way_lane_offset(l1))
        .fadd(xi_(i32::from(rd::<i8>(ap, 0x27))))
        .fmul_m(k(0x40AC_CCCD)) // 5.4
        .to_f32();
    let o2 = X87::from_f64_exact(one_way_lane_offset(l2))
        .fadd(xi_(i32::from(rd::<i8>(ap, 0x28))))
        .fmul_m(k(0x40AC_CCCD));
    let e = 0.125;
    let mut p2 = [0f32; 3];
    p2[0] = xi_(i32::from(rd::<i16>(l2, 0)))
        .fmul_m(e)
        .fadd(o2.fmul_m(d2y))
        .to_f32();
    p2[1] = xi_(i32::from(rd::<i16>(l2, 2)))
        .fmul_m(e)
        .fsub(o2.fmul_m(d2x))
        .to_f32();
    let mut p1 = [0f32; 3];
    p1[0] = xi_(i32::from(rd::<i16>(l1, 0)))
        .fmul_m(e)
        .fadd(x_(o1).fmul_m(d1y))
        .to_f32();
    p1[1] = xi_(i32::from(rd::<i16>(l1, 2)))
        .fmul_m(e)
        .fsub(x_(o1).fmul_m(d1x))
        .to_f32();
    // the arguments are recomputed from the bytes (same values as d1x.. above)
    let r = calc_speed_scale_factor(
        GPtr::from_raw(p1.as_mut_ptr()),
        GPtr::from_raw(p2.as_mut_ptr()),
        dir(l1, 8, lane1).to_f32(),
        dir(l1, 9, lane1).to_f32(),
        dir(l2, 8, lane2).to_f32(),
        dir(l2, 9, lane2).to_f32(),
    );
    let t = ftol32(
        x_(1000.0)
            .fdiv_m(rd::<f32>(ap, 0x38))
            .fmul(X87::from_f64_exact(r)),
    );
    wr::<i32>(ap, 0x10, t);
    let mut f = Fpu::new();
    fild_u32(&mut f, time_ms());
    let now = f.pop();
    wr::<i32>(ap, 0xC, ftol32(now.fsub(xi_(t).fmul_m(progress))));
}

#[inline]
fn x_(v: f32) -> X87 {
    X87::from_f32(v)
}

#[inline]
fn xi_(v: i32) -> X87 {
    X87::from_i32(v)
}

// ---------------------------------------------------------------------------------------------------------
// Collision tests between two moving boxes

/// `CEntity::GetPosition()` inlined: the matrix position (+0x14 → +0x30) or the placement (+4).
fn entity_pos(e: GPtr<u8>) -> GPtr<f32> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4).cast()
    } else {
        m.byte_add(0x30).cast()
    }
}

/// The collision model of an entity's model: `CModelInfo::ms_modelInfoPtrs[+0x22] → +0x14`.
fn col_model(e: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(crate::common::model_info(i32::from(rd::<i16>(e, 0x22))), 0x14)
}

#[inline]
fn vx(p: GPtr<f32>, i: usize) -> f32 {
    p.wrapping_index(i as i32).read()
}

/// `fcomp; fnstsw; test ah, 0x41; jne` taken: `a <= b` or unordered.
#[inline]
fn le41(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}

/// `fcomp; fnstsw; test ah, 5; jp` taken: `a >= b` (unordered not).
#[inline]
fn ge5(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x05)
}

/// `CCarCtrl::TestCollisionBetween2MovingRects(CVehicle* a, CVehicle* b, float vx, float vy, CVector* fb,
/// CVector* fa)` (0x425B30, ST0): the time in [0, 1] at which `a`'s box (corners from max.x / max.y of its
/// collision bounding box, turned by the 6th argument `fa`) moving by (vx, vy) relative to `b` first
/// overlaps `b`'s box (x from -max.x to max.x across the 5th argument `fb`, y from min.y to max.y along
/// it); 1 when never. For each of `a`'s two front corners the entry/exit times per axis of `b` are
/// computed as the bytes order them (axis 1: entry in a local, exit kept on the x87 stack; axis 2 the
/// reverse), then `E = max(entry1, entry2)`; `E < exit1 && E < exit2 && E <= best` → best = E.
#[hook(addr = 0x425B30, conv = cdecl, name = "CCarCtrl::TestCollisionBetween2MovingRects", expect = "83 EC 3C 53 56")]
pub fn test_collision_between_2_moving_rects(
    a: GPtr<u8>,
    b: GPtr<u8>,
    mvx: f32,
    mvy: f32,
    fb: GPtr<f32>,
    fa: GPtr<f32>,
) -> X87 {
    let one = x_(1.0);
    let zero = k(0x0000_0000);
    let pa = entity_pos(a);
    let pb = entity_pos(b);
    let dx = x_(vx(pa, 0)).fsub_m(vx(pb, 0)).to_f32(); // [esp+0x2c]
    let dy = x_(vx(pa, 1)).fsub_m(vx(pb, 1)).to_f32(); // [esp+0x30]
    let cb = col_model(b);
    let b_max_y = rd::<f32>(cb, 0x10); // [esp+0x1c]
    let b_min_y = x_(rd::<f32>(cb, 4)).fchs().to_f32(); // [esp+0x20], negated
    let b_max_x = rd::<f32>(cb, 0xC); // [esp+0xc]
    let ca = col_model(a);
    let a_max_x = rd::<f32>(ca, 0xC); // [esp+0x28]
    let a_max_y = rd::<f32>(ca, 0x10); // [esp+0x24]
    let (fbx, fby) = (vx(fb, 0), vx(fb, 1));
    let rel_x = x_(mvx).fmul_m(fby).fsub(x_(mvy).fmul_m(fbx)).to_f32(); // [esp+0x34]
    let rel_y = x_(mvx).fmul_m(fbx).fadd(x_(mvy).fmul_m(fby)).to_f32(); // [esp+0x38]
    let (fax, fay) = (vx(fa, 0), vx(fa, 1));
    let mut best = one;
    let mut cx = 0f32; // [esp+0x3c]
    let mut cy = 0f32; // [esp+0x40]
    for corner in 0..2 {
        if corner == 0 {
            cx = x_(a_max_y)
                .fmul_m(fax)
                .fadd(x_(a_max_x).fmul_m(fay))
                .fadd_m(dx)
                .to_f32();
            cy = x_(a_max_y)
                .fmul_m(fay)
                .fadd_m(dy)
                .fsub(x_(a_max_x).fmul_m(fax))
                .to_f32();
        } else {
            cx = x_(a_max_y)
                .fmul_m(fax)
                .fadd_m(dx)
                .fsub(x_(a_max_x).fmul_m(fay))
                .to_f32();
            cy = x_(a_max_x)
                .fmul_m(fax)
                .fadd(x_(a_max_y).fmul_m(fay))
                .fadd_m(dy)
                .to_f32();
        }
        // axis 1 (`fb`'s perpendicular): entry1 in [esp+0x10], exit1 on the stack
        let v = rel_x; // [esp+0x4c]
        let p = x_(cx).fmul_m(fby).fsub(x_(cy).fmul_m(fbx)).to_f32(); // [esp+0x50]
        let mut entry1 = 0f32;
        let exit1: X87;
        let vz = x_(v).fcom(x_(zero)); // compare v with 0.0
        if !le41(x_(p), x_(b_max_x)) {
            // p > max.x
            if vz.test_parity(0x05) {
                entry1 = 1.0;
                exit1 = one;
            } else {
                let q = one.fdiv_m(v);
                let t = x_(p).fsub_m(b_max_x).fmul(q).fchs();
                if ge5(t, one) {
                    entry1 = 1.0;
                    exit1 = one;
                } else {
                    entry1 = t.to_f32();
                    let w = q.fmul(x_(b_max_x).fadd(x_(b_max_x)));
                    let r = t.fsub(w);
                    exit1 = axis1_exit(r);
                }
            }
        } else if !le41(x_(b_max_x).fchs(), x_(p)) {
            // p < -max.x
            if vz.test_any(0x41) {
                entry1 = 1.0;
                exit1 = one;
            } else {
                let q = one.fdiv_m(v);
                let t = x_(p).fadd_m(b_max_x).fmul(q).fchs();
                if ge5(t, one) {
                    entry1 = 1.0;
                    exit1 = one;
                } else {
                    entry1 = t.to_f32();
                    let w = q.fmul(x_(b_max_x).fadd(x_(b_max_x)));
                    let r = w.fadd(t);
                    exit1 = axis1_exit(r);
                }
            }
        } else if !vz.test_any(0x41) {
            exit1 = x_(b_max_x).fsub_m(p).fdiv_m(v);
        } else if vz.test_parity(0x05) {
            exit1 = one;
        } else {
            exit1 = x_(p).fadd_m(b_max_x).fdiv_m(v).fchs();
        }
        // axis 2 (along `fb`): entry2 on the stack, exit2 in [esp+0x14]
        let v = rel_y;
        let p = x_(cy).fmul_m(fby).fadd(x_(cx).fmul_m(fbx)).to_f32();
        let mut exit2 = 1f32;
        let entry2: X87;
        let vz = x_(v).fcom(x_(zero));
        if !le41(x_(p), x_(b_max_y)) {
            // p > max.y
            if vz.test_parity(0x05) {
                entry2 = one;
            } else {
                let q = one.fdiv_m(v).to_f32(); // [esp+0x18]
                let tx = x_(p).fsub_m(b_max_y).fmul_m(q).fchs();
                let t = tx.to_f32();
                if ge5(tx, one) {
                    entry2 = one;
                } else {
                    let r = x_(b_min_y).fadd_m(b_max_y).fmul_m(q).fsubr_m(t);
                    entry2 = x_(t);
                    if !ge5(r, one) {
                        exit2 = r.to_f32();
                    }
                }
            }
        } else if !le41(x_(b_min_y).fchs(), x_(p)) {
            // p < min.y
            if vz.test_any(0x41) {
                entry2 = one;
            } else {
                let q = one.fdiv_m(v).to_f32();
                let tx = x_(p).fadd_m(b_min_y).fmul_m(q).fchs();
                let t = tx.to_f32();
                if ge5(tx, one) {
                    entry2 = one;
                } else {
                    let r = x_(b_min_y).fadd_m(b_max_y).fmul_m(q).fadd_m(t);
                    entry2 = x_(t);
                    if !ge5(r, one) {
                        exit2 = r.to_f32();
                    }
                }
            }
        } else {
            entry2 = x_(zero);
            if !vz.test_any(0x41) {
                exit2 = x_(b_max_y).fsub_m(p).fdiv_m(v).to_f32();
            } else if !vz.test_parity(0x05) {
                exit2 = x_(p).fadd_m(b_min_y).fdiv_m(v).fchs().to_f32();
            }
        }
        // E = max(entry1, entry2): `fld entry1; fcomp st1; test ah, 0x41; jne` keeps entry2
        let e = if le41(x_(entry1), entry2) {
            entry2
        } else {
            x_(entry1)
        };
        let e32 = e.to_f32();
        if ge5(e, exit1) || ge5(x_(e32), x_(exit2)) {
            continue;
        }
        // `fcom E; test ah, 5; jnp` skips when best < E
        if !best.fcom(x_(e32)).test_parity(0x05) {
            continue;
        }
        best = x_(e32);
    }
    let _ = (cx, cy);
    best
}

/// Axis 1's exit value: `fst [esp+0x50]` of `r`; `r >= 1` → 1 (the 1.0 still on the stack), else `r`
/// reloaded from the f32 local.
fn axis1_exit(r: X87) -> X87 {
    let r32 = r.to_f32();
    if ge5(r, x_(1.0)) { x_(1.0) } else { x_(r32) }
}

/// `CCarCtrl::TestCollisionBetween2MovingRects_OnlyFrontBumper(CVehicle* a, CVehicle* b, float vx, float vy,
/// CVector* fb, CVector* fa)` (0x425F70, ST0): for the four corners of `a`'s box (max.x / max.y, and the
/// negated min.y for the rear two; turned by `fa`, jump table 0x426210), the segment from the corner to
/// corner + (vx, vy) is tested against `b`'s front bumper line (from `b`'s position + its max.y along `fb`,
/// ± max.x across it): when the corner is in front of the line and its moved point behind it
/// (`u > 0`, `w < 0`) and the two cross products have opposite signs, `t = u / (u - w)`; the result is the
/// smallest such `t` (1 when none). The fourth x87 slot the bytes load before the loop
/// (`[esp+0x2c]`, never written) is popped unread by the first case.
#[hook(addr = 0x425F70, conv = cdecl, name = "CCarCtrl::TestCollisionBetween2MovingRects_OnlyFrontBumper", expect = "8B 54 24 04 8B 42 14")]
pub fn test_collision_between_2_moving_rects_only_front_bumper(
    a: GPtr<u8>,
    b: GPtr<u8>,
    mvx: f32,
    mvy: f32,
    fb: GPtr<f32>,
    fa: GPtr<f32>,
) -> X87 {
    let pa = entity_pos(a);
    let (pax, pay) = (vx(pa, 0), vx(pa, 1));
    let cb = col_model(b);
    let bmy = rd::<f32>(cb, 0x10);
    let bmx = rd::<f32>(cb, 0xC);
    let ca = col_model(a);
    let amy = rd::<f32>(ca, 0x10);
    let amx = rd::<f32>(ca, 0xC);
    let nmin = x_(rd::<f32>(ca, 4)).fchs().to_f32();
    let pb = entity_pos(b);
    let (pbx, pby) = (vx(pb, 0), vx(pb, 1));
    let (e5x, e5y) = (vx(fb, 0), vx(fb, 1));
    let (e6x, e6y) = (vx(fa, 0), vx(fa, 1));
    let bmx_e5y = x_(bmx).fmul_m(e5y);
    let bmy_e5x = x_(bmy).fmul_m(e5x);
    let l20 = bmy_e5x.fadd(bmx_e5y).fadd_m(pbx).to_f32();
    let l10 = x_(bmy).fmul_m(e5y).to_f32();
    let l18 = x_(bmx).fmul_m(e5x).to_f32();
    let l24 = x_(l10).fadd(x_(pby)).fsub_m(l18).to_f32();
    let l38 = bmy_e5x.fadd_m(pbx).fsub(bmx_e5y).to_f32();
    let q = x_(pby).fadd(x_(l18).fadd_m(l10));
    let e5x_st = x_(e5x);
    let mut best = 1f32;
    for corner in 0..4 {
        let (ca_, cb_) = match corner {
            0 => (
                x_(amx).fmul_m(e6y).fadd(x_(amy).fmul_m(e6x)).fadd_m(pax),
                x_(amy).fmul_m(e6y).fadd_m(pay).fsub(x_(amx).fmul_m(e6x)),
            ),
            1 => (
                x_(amy).fmul_m(e6x).fadd_m(pax).fsub(x_(amx).fmul_m(e6y)),
                x_(amy).fmul_m(e6y).fadd(x_(amx).fmul_m(e6x)).fadd_m(pay),
            ),
            2 => (
                x_(nmin).fmul_m(e6x).fsubr_m(pax).fadd(x_(amx).fmul_m(e6y)),
                x_(nmin).fmul_m(e6y).fsubr_m(pay).fsub(x_(amx).fmul_m(e6x)),
            ),
            _ => (
                x_(nmin).fmul_m(e6x).fsubr_m(pax).fsub(x_(amx).fmul_m(e6y)),
                x_(nmin).fmul_m(e6y).fsubr_m(pay).fadd(x_(amx).fmul_m(e6x)),
            ),
        };
        let a2 = ca_.fadd_m(mvx);
        let b2 = cb_.fadd_m(mvy);
        let u = cb_
            .fsub_m(l24)
            .fmul_m(e5y)
            .fadd(ca_.fsub_m(l20).fmul(e5x_st))
            .to_f32();
        let w = b2
            .fsub_m(l24)
            .fmul_m(e5y)
            .fadd(a2.fsub_m(l20).fmul(e5x_st))
            .to_f32();
        if le41(x_(u), x_(0.0)) || ge5(x_(w), x_(0.0)) {
            continue;
        }
        let c1 = x_(l38).fsub(ca_).fmul_m(mvy).fsub(q.fsub(cb_).fmul_m(mvx));
        let c2 = x_(l20).fsub(ca_).fmul_m(mvy).fsub(x_(l24).fsub(cb_).fmul_m(mvx));
        if ge5(c1.fmul(c2), x_(0.0)) {
            continue;
        }
        let t = x_(u).fsub_m(w).fdivr_m(u);
        // `fld best; fcomp t; test ah, 5; jnp` keeps best when best < t
        if !x_(best).fcom(t).test_parity(0x05) {
            continue;
        }
        best = t.to_f32();
    }
    x_(best)
}

// ---------------------------------------------------------------------------------------------------------
// Slowing down for other cars, peds and objects

orig! {
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, thiscall on the placement)
    fn update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    fn get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CCarCtrl::SwitchVehicleToRealPhysics(CVehicle*)` (0x423FC0)
    fn switch_vehicle_to_real_physics(v: GPtr<u8>) = cdecl 0x423FC0 => crate::carctrl::switch_vehicle_to_real_physics;
    /// `CCarCtrl::TestCollisionBetween2MovingRects_OnlyFrontBumper` (0x425F70, ST0)
    fn test_collision_front_bumper_call(a: GPtr<u8>, b: GPtr<u8>, mvx: f32, mvy: f32, fb: GPtr<f32>, fa: GPtr<f32>) -> f64 = cdecl 0x425F70 => test_collision_front_bumper_abi;
    /// `CCarCtrl::TestCollisionBetween2MovingRects` (0x425B30, ST0)
    fn test_collision_call(a: GPtr<u8>, b: GPtr<u8>, mvx: f32, mvy: f32, fb: GPtr<f32>, fa: GPtr<f32>) -> f64 = cdecl 0x425B30 => test_collision_abi;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn test_collision_front_bumper_abi(
    a: GPtr<u8>,
    b: GPtr<u8>,
    mvx: f32,
    mvy: f32,
    fb: GPtr<f32>,
    fa: GPtr<f32>,
) -> f64 {
    test_collision_between_2_moving_rects_only_front_bumper(a, b, mvx, mvy, fb, fa).to_f64()
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn test_collision_abi(a: GPtr<u8>, b: GPtr<u8>, mvx: f32, mvy: f32, fb: GPtr<f32>, fa: GPtr<f32>) -> f64 {
    test_collision_between_2_moving_rects(a, b, mvx, mvy, fb, fa).to_f64()
}

/// The 2D forward vector of a matrix (+0x10, +0x14) normalised as the inlined `CVector2D::Normalise` of
/// these functions does: `len = sqrt(fy² + fx²)`; `len == 0` (`test ah, 0x44; jnp`) → (1, fy) unchanged,
/// else `(fx / len, fy / len)` as `1/len` products (x87 kept, stored f32).
fn forward_2d_normalised(m: GPtr<u8>) -> (f32, f32) {
    let fx = rd::<f32>(m, 0x10);
    let fy = rd::<f32>(m, 0x14);
    let len = x_(fy).fmul_m(fy).fadd(x_(fx).fmul_m(fx)).fsqrt();
    if !len.fcom(x_(0.0)).test_parity(0x44) {
        return (1.0, fy);
    }
    let inv = len.fdivr_m(1.0);
    (inv.fmul_m(fx).to_f32(), inv.fmul_m(fy).to_f32())
}

/// `CCarCtrl::SlowCarDownForOtherCar(CEntity* other, CVehicle* veh, float* maxSpeed, float speed)`
/// (0x42D0E0): only when `other` is not behind `veh` (2D dot of the offset with `veh`'s normalised
/// forward >= 0). The relative velocity `other.v * 60 - fwd * speed` goes to
/// `TestCollisionBetween2MovingRects_OnlyFrontBumper(other, veh, ..)` and, negated,
/// `TestCollisionBetween2MovingRects(veh, other, ..)`; `t` = the smaller (the first when it is less).
/// `0 <= t < 1.5`: `veh` +0x3DB bit 0, +0x420 = other (referenced); `t < 1/speed` → max 0; `t < 3/speed`
/// → max = min(max, 1); else max = min(max, max(0, (t - 0.2) / 1.3) * speed), `t` replaced by that
/// clamped value. Then `0 <= t < 0.5`, `other` a vehicle (type 2), both cars more than 15 s since their
/// +0x3B0 time, `other` not the player's vehicle, the forwards' dot < -0.5 and `veh < other` (addresses):
/// max = max(max, 0.2 * speed), a ghost car (status 2) switched to real physics, +0x3B9 = 2, +0x3BC =
/// now + 1000.
#[hook(addr = 0x42D0E0, conv = cdecl, name = "CCarCtrl::SlowCarDownForOtherCar", expect = "83 EC 30 53 55")]
pub fn slow_car_down_for_other_car(other: GPtr<u8>, veh: GPtr<u8>, max_speed: GPtr<f32>, speed: f32) {
    let vm = rd::<GPtr<u8>>(veh, 0x14);
    let mut vdir = [0f32; 3]; // [esp+0x1c]
    let (dx, dy) = forward_2d_normalised(vm);
    vdir[0] = dx;
    vdir[1] = dy;
    let po = entity_pos(other);
    let pv = entity_pos(veh);
    let ahead = x_(vx(po, 1))
        .fsub_m(vx(pv, 1))
        .fmul_m(vdir[1])
        .fadd(x_(vx(po, 0)).fsub_m(vx(pv, 0)).fmul_m(vdir[0]));
    // `test ah, 5; jnp`: return when behind (< 0)
    if !ahead.fcom(x_(0.0)).test_parity(0x05) {
        return;
    }
    let ovx = x_(rd::<f32>(other, 0x44)).fmul_m(60.0).to_f32();
    let ovy = x_(rd::<f32>(other, 0x48)).fmul_m(60.0).to_f32();
    let sx = x_(vdir[0]).fmul_m(speed).to_f32();
    let sy = x_(vdir[1]).fmul_m(speed).to_f32();
    if rd::<GPtr<u8>>(other, 0x14).is_null() {
        allocate_matrix(other);
        update_matrix(other.byte_add(4), rd::<GPtr<u8>>(other, 0x14));
    }
    let mut om = rd::<GPtr<u8>>(other, 0x14);
    if om.is_null() {
        allocate_matrix(other);
        update_matrix(other.byte_add(4), rd::<GPtr<u8>>(other, 0x14));
        om = rd::<GPtr<u8>>(other, 0x14);
    }
    let mut odir = [0f32; 3]; // [esp+0x34]
    let (ox, oy) = forward_2d_normalised(om);
    odir[0] = ox;
    odir[1] = oy;
    let rvx = x_(ovx).fsub_m(sx).to_f32();
    let rvy = x_(ovy).fsub_m(sy).to_f32();
    let pv_dir = GPtr::from_raw(vdir.as_mut_ptr());
    let po_dir = GPtr::from_raw(odir.as_mut_ptr());
    let t1 = X87::from_f64_exact(test_collision_front_bumper_call(
        other, veh, rvx, rvy, pv_dir, po_dir,
    ))
    .to_f32();
    let mrvy = x_(rvy).fchs().to_f32();
    let mrvx = x_(rvx).fchs().to_f32();
    let t2 = X87::from_f64_exact(test_collision_call(veh, other, mrvx, mrvy, po_dir, pv_dir));
    // `fld t1; fcomp t2; test ah, 5; jnp` keeps t1 when t1 < t2
    let mut t = if !x_(t1).fcom(t2).test_parity(0x05) {
        t1
    } else {
        t2.to_f32()
    };
    if x_(t).fcom(x_(0.0)).test_any(0x01) {
        return;
    }
    if !ge5(x_(t), x_(1.5)) {
        wr::<u8>(veh, 0x3DB, rd::<u8>(veh, 0x3DB) | 1);
        let field = veh.byte_add(0x420);
        field.cast::<GPtr<u8>>().write(other);
        entity_register_reference(other, field);
        let inv = x_(1.0).fdiv_m(speed);
        if !ge5(x_(t), inv) {
            max_speed.write(0.0);
        } else if !ge5(x_(t), x_(3.0).fmul(inv)) {
            let ms = max_speed.read();
            if !ge5(x_(ms), x_(1.0)) {
                max_speed.write(ms);
            } else {
                max_speed.write(1.0);
            }
        } else {
            let v = x_(t).fsub_m(k(0x3E4C_CCCD)).fmul_m(k(0x3F44_EC4F));
            t = if le41(x_(0.0), v) { v.to_f32() } else { 0.0 };
            let s = x_(t).fmul_m(speed);
            let ms = max_speed.read();
            max_speed.write(if le41(s, x_(ms)) { s.to_f32() } else { ms });
        }
    }
    if x_(t).fcom(x_(0.0)).test_any(0x01) || ge5(x_(t), x_(0.5)) {
        return;
    }
    if rd::<u8>(other, 0x36) & 7 != 2 {
        return;
    }
    let now = time_ms();
    if now.wrapping_sub(rd::<u32>(veh, 0x3B0)) <= 15000 || now.wrapping_sub(rd::<u32>(other, 0x3B0)) <= 15000
    {
        return;
    }
    let vmat = rd::<GPtr<u8>>(veh, 0x14);
    let om1 = get_matrix(other);
    let vmat2 = rd::<GPtr<u8>>(veh, 0x14);
    let om2 = get_matrix(other);
    let dot = x_(rd::<f32>(om2, 0x14))
        .fmul_m(rd::<f32>(vmat2, 0x14))
        .fadd(x_(rd::<f32>(om1, 0x10)).fmul_m(rd::<f32>(vmat, 0x10)))
        .to_f32();
    if other == find_player_vehicle(-1, 0) {
        return;
    }
    if ge5(x_(dot), x_(-0.5)) {
        return;
    }
    if veh.addr() >= other.addr() {
        return;
    }
    let s = x_(speed).fmul_m(k(0x3E4C_CCCD));
    let ms = max_speed.read();
    max_speed.write(if ge5(s, x_(ms)) { s.to_f32() } else { ms });
    if rd::<u8>(veh, 0x36) & 0xF8 == 0x10 {
        switch_vehicle_to_real_physics(veh);
    }
    wr::<u8>(veh, 0x3B9, 2);
    wr::<u32>(veh, 0x3BC, time_ms().wrapping_add(1000));
}

orig! {
    /// `CEntity::GetBoundCentre(CVector& out)` (0x534250, thiscall, `ret 4`)
    fn get_bound_centre(e: GPtr<u8>, out: GPtr<f32>) = thiscall 0x534250 => unported;
    /// `CCollision::DistAlongLine2D(x, y, dx, dy, px, py)` (0x412A80, ST0)
    fn dist_along_line_2d(x: f32, y: f32, dx: f32, dy: f32, px: f32, py: f32) -> f64 = cdecl 0x412A80 => dist_along_line_2d_abi;
    /// `CPlaceable::GetForward(CVector* out)` (0x41CCB0, thiscall, `ret 4`; returns the matrix's +0x10
    /// row, or `out` filled from the heading without a matrix)
    fn get_forward(e: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x41CCB0 => unported;
    /// `CCarCtrl::SlowCarDownForOtherCar` (0x42D0E0)
    fn slow_car_down_for_other_car_call(other: GPtr<u8>, veh: GPtr<u8>, ms: GPtr<f32>, speed: f32) = cdecl 0x42D0E0 => slow_car_down_for_other_car;
    /// `CCarCtrl::SlowCarDownForObject` (0x426220)
    fn slow_car_down_for_object_call(obj: GPtr<u8>, veh: GPtr<u8>, ms: GPtr<f32>, speed: f32) = cdecl 0x426220 => slow_car_down_for_object;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn dist_along_line_2d_abi(x: f32, y: f32, dx: f32, dy: f32, px: f32, py: f32) -> f64 {
    sa_world::collision::prim::dist_along_line_2d(x, y, dx, dy, px, py).to_f64()
}

/// `CWorld::ms_nCurrentScanCode` (0xB7CD78).
fn scan_code() -> u16 {
    g::<u16>(0xB7CD78).read()
}

/// The shared tail of the `SlowCarDownFor{Cars,Objects}SectorList` loops once an entity passed its own
/// filters (the scan code already set): the bound centre inside (minX, maxX) × (minY, maxY) (strict),
/// `|centre.z - veh.z| < 10` (`fabs` in the objects loop, a sign test + `fchs` in the cars loop), and the
/// height difference to the vehicle's forward line at the 2D distance along it below 3.
#[allow(clippy::too_many_arguments)]
fn sector_list_candidate(
    e: GPtr<u8>,
    veh: GPtr<u8>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    fabs_z: bool,
) -> bool {
    let mut c = [0f32; 3]; // [esp+0x10]
    get_bound_centre(e, GPtr::from_raw(c.as_mut_ptr()));
    if le41(x_(c[0]), x_(min_x))
        || ge5(x_(c[0]), x_(max_x))
        || le41(x_(c[1]), x_(min_y))
        || ge5(x_(c[1]), x_(max_y))
    {
        return false;
    }
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let dz = x_(c[2]).fsub_m(vx(entity_pos(veh), 2));
    let dz = if fabs_z {
        dz.fabs()
    } else if ge5(dz, x_(0.0)) {
        dz
    } else {
        dz.fchs()
    };
    if ge5(dz, x_(10.0)) {
        return false;
    }
    let p = entity_pos(veh);
    let d = X87::from_f64_exact(dist_along_line_2d(
        vx(p, 0),
        vx(p, 1),
        rd::<f32>(m, 0x10),
        rd::<f32>(m, 0x14),
        c[0],
        c[1],
    ))
    .to_f32();
    let p = entity_pos(veh);
    let mut fwd = [0f32; 3]; // [esp+0x1c]
    let f = get_forward(veh, GPtr::from_raw(fwd.as_mut_ptr()));
    let h = x_(d).fmul_m(vx(f, 2)).fadd_m(vx(p, 2)).fsubr_m(c[2]);
    let h = if ge5(h, x_(0.0)) { h } else { h.fchs() };
    !ge5(h, x_(3.0))
}

/// `CCarCtrl::SlowCarDownForCarsSectorList(CPtrListDoubleLink&, CVehicle* veh, float minX, float minY, float
/// maxX, float maxY, float* maxSpeed, float speed)` (0x432420): every entity of the list other than `veh`,
/// not yet scanned and using collision (+0x1C bit 0), gets the scan code; candidates
/// ([`sector_list_candidate`]) go to `SlowCarDownForOtherCar`.
#[hook(addr = 0x432420, conv = cdecl, name = "CCarCtrl::SlowCarDownForCarsSectorList<CPtrListDoubleLink<CPhysical*>>", expect = "8B 44 24 04 83 EC 18")]
#[allow(clippy::too_many_arguments)]
pub fn slow_car_down_for_cars_sector_list(
    list: GPtr<u8>,
    veh: GPtr<u8>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    max_speed: GPtr<f32>,
    speed: f32,
) {
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        if e == veh {
            continue;
        }
        let sc = scan_code();
        if rd::<u16>(e, 0x2C) == sc || rd::<u8>(e, 0x1C) & 1 == 0 {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        if sector_list_candidate(e, veh, min_x, min_y, max_x, max_y, false) {
            slow_car_down_for_other_car_call(e, veh, max_speed, speed);
        }
    }
}

/// `CCarCtrl::SlowCarDownForObjectsSectorList(CPtrListDoubleLink&, CVehicle* veh, float minX, float minY,
/// float maxX, float maxY, float* maxSpeed, float speed)` (0x42D4F0): every entity not yet scanned gets the
/// scan code; only the three models at `ModelIndices` 0x8CD57C / 0x8CD580 / 0x8CD584 are candidates
/// ([`sector_list_candidate`]) for `SlowCarDownForObject`.
#[hook(addr = 0x42D4F0, conv = cdecl, name = "CCarCtrl::SlowCarDownForObjectsSectorList<CPtrListDoubleLink<CPhysical*>>", expect = "8B 44 24 04 83 EC 18")]
#[allow(clippy::too_many_arguments)]
pub fn slow_car_down_for_objects_sector_list(
    list: GPtr<u8>,
    veh: GPtr<u8>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    max_speed: GPtr<f32>,
    speed: f32,
) {
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        let sc = scan_code();
        node = rd::<GPtr<u8>>(node, 4);
        if rd::<u16>(e, 0x2C) == sc {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        let mi = i32::from(rd::<i16>(e, 0x22));
        let is = |a: usize| mi == i32::from(g::<u16>(a).read());
        if !(is(0x8CD57C) || is(0x8CD580) || is(0x8CD584)) {
            continue;
        }
        if sector_list_candidate(e, veh, min_x, min_y, max_x, max_y, true) {
            slow_car_down_for_object_call(e, veh, max_speed, speed);
        }
    }
}

/// `CCarCtrl::SlowCarDownForObject(CEntity* obj, CVehicle* veh, float* maxSpeed, float speed)` (0x426220):
/// with `d` = obj - veh (positions), the distance ahead `D = d · fwd` (matrix +0x10 row, summed z, y, x)
/// in (0, 20) and the side distance `|d · right|` below the object's bounding radius (col +0x24) + the
/// car's max.x: `D < 7` → max 0, else max = min(max, (1 - (D - 7) / 13) * speed).
#[hook(addr = 0x426220, conv = cdecl, name = "CCarCtrl::SlowCarDownForObject", expect = "83 EC 0C 56 57")]
pub fn slow_car_down_for_object(obj: GPtr<u8>, veh: GPtr<u8>, max_speed: GPtr<f32>, speed: f32) {
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let pv = entity_pos(veh);
    let po = entity_pos(obj);
    let dx = x_(vx(po, 0)).fsub_m(vx(pv, 0)).to_f32();
    let dy = x_(vx(po, 1)).fsub_m(vx(pv, 1)).to_f32();
    let dzx = x_(vx(po, 2)).fsub_m(vx(pv, 2));
    let dz = dzx.to_f32();
    let dd = dzx
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(x_(dy).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(x_(dx).fmul_m(rd::<f32>(m, 0x10)));
    let d = dd.to_f32();
    if le41(dd, x_(0.0)) || ge5(x_(d), x_(20.0)) {
        return;
    }
    let side = x_(dz)
        .fmul_m(rd::<f32>(m, 8))
        .fadd(x_(dy).fmul_m(rd::<f32>(m, 4)))
        .fadd(x_(dx).fmul_m(rd::<f32>(m, 0)))
        .fabs();
    let lim = x_(rd::<f32>(col_model(obj), 0x24)).fadd_m(rd::<f32>(col_model(veh), 0xC));
    if le41(lim, side) {
        return;
    }
    if !ge5(x_(d), x_(7.0)) {
        max_speed.write(0.0);
        return;
    }
    let v = x_(d)
        .fsub_m(7.0)
        .fmul_m(k(0x3D9D_89D9))
        .fsubr_m(1.0)
        .fmul_m(speed);
    let ms = max_speed.read();
    max_speed.write(if ge5(x_(ms), v) { v.to_f32() } else { ms });
}

orig! {
    /// CRT `floor(double)` (0x8219F0)
    fn crt_floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CCarCtrl::SlowCarDownForPedsSectorList` (0x425440)
    #[allow(clippy::too_many_arguments)]
    fn slow_car_down_for_peds_sector_list_call(list: GPtr<u8>, veh: GPtr<u8>, min_x: f32, min_y: f32, max_x: f32, max_y: f32, ms: GPtr<f32>, speed: f32) = cdecl 0x425440 => slow_car_down_for_peds_sector_list;
    /// `CCarCtrl::SlowCarDownForCarsSectorList` (0x432420)
    #[allow(clippy::too_many_arguments)]
    fn slow_car_down_for_cars_sector_list_call(list: GPtr<u8>, veh: GPtr<u8>, min_x: f32, min_y: f32, max_x: f32, max_y: f32, ms: GPtr<f32>, speed: f32) = cdecl 0x432420 => slow_car_down_for_cars_sector_list;
    /// `CCarCtrl::SlowCarDownForObjectsSectorList` (0x42D4F0)
    #[allow(clippy::too_many_arguments)]
    fn slow_car_down_for_objects_sector_list_call(list: GPtr<u8>, veh: GPtr<u8>, min_x: f32, min_y: f32, max_x: f32, max_y: f32, ms: GPtr<f32>, speed: f32) = cdecl 0x42D4F0 => slow_car_down_for_objects_sector_list;
}

/// The cruise speed as the bytes use it: `fild (u8)+0x3D0; fmul +0x3D4`.
fn cruise_speed(veh: GPtr<u8>) -> X87 {
    xi_(i32::from(rd::<u8>(veh, 0x3D0))).fmul_m(rd::<f32>(veh, 0x3D4))
}

/// The sector index of a world coordinate as these loops compute it: `v * 0.02 + 60` stored f32, then
/// `_ftol(floor(v))` of that f32 (the test `> 0` / `< 119` is made on the same value).
fn sector_of(v: f32) -> (i32, f32) {
    let s = x_(v).fmul_m(k(0x3CA3_D70A)).fadd_m(60.0);
    let s32 = s.to_f32();
    (ftol32(X87::from_f64_exact(crt_floor(s.to_f64()))), s32)
}

/// `CCarCtrl::FindMaximumSpeedForThisCarInTraffic(CVehicle*)` (0x434400, ST0): temp actions 2, 3 and 5 →
/// the cruise speed. Else, over the repeat sectors (0xB992B8, 16 × 16 × 3 lists) of the ±14 box around the
/// car (sector x / y clamped to [0, 119] by the `> 0` / `< 119` tests), with a fresh scan code, each sector
/// slows `maxSpeed` (starting at the cruise speed) through the cars (unless temp action 6), peds and
/// objects lists; then +0x429 bit 5; temp actions 0 and 4 → `maxSpeed`, else `(cruise + maxSpeed) / 2`.
#[hook(addr = 0x434400, conv = cdecl, name = "CCarCtrl::FindMaximumSpeedForThisCarInTraffic", expect = "83 EC 24 56 8B 74 24 2C")]
pub fn find_maximum_speed_for_this_car_in_traffic(veh: GPtr<u8>) -> X87 {
    let ta = rd::<u8>(veh, 0x3B9);
    if ta == 2 || ta == 3 || ta == 5 {
        return cruise_speed(veh);
    }
    let p = entity_pos(veh);
    let min_x = x_(vx(p, 0)).fsub_m(14.0).to_f32();
    let max_x = x_(vx(p, 0)).fadd_m(14.0).to_f32();
    let min_y = x_(vx(p, 1)).fsub_m(14.0).to_f32();
    let max_y = x_(vx(p, 1)).fadd_m(14.0).to_f32();
    let clamp_lo = |v: f32| {
        let (n, s32) = sector_of(v);
        if n > 0 {
            ftol32(X87::from_f64_exact(crt_floor(f64::from(s32))))
        } else {
            0
        }
    };
    let clamp_hi = |v: f32| {
        let (n, s32) = sector_of(v);
        if n < 0x77 {
            ftol32(X87::from_f64_exact(crt_floor(f64::from(s32))))
        } else {
            0x77
        }
    };
    let x0 = clamp_lo(min_x);
    let y0 = clamp_lo(min_y);
    let x1 = clamp_hi(max_x);
    let y1 = clamp_hi(max_y);
    let sc = g::<u16>(0xB7CD78);
    if sc.read() < 0xFFFF {
        sc.write(sc.read() + 1);
    } else {
        world_clear_scan_codes();
        sc.write(1);
    }
    let mut ms = [cruise_speed(veh).to_f32()];
    let msp = GPtr::from_raw(ms.as_mut_ptr());
    let mut y = y0;
    while y <= y1 {
        let mut xx = x0;
        while xx <= x1 {
            let rs = ((xx & 0xF) + ((y & 0xF) << 4)) as isize;
            let lists = g::<u8>(0xB992B8).byte_add(rs * 12);
            if rd::<u8>(veh, 0x3B9) != 6 {
                slow_car_down_for_cars_sector_list_call(
                    lists,
                    veh,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    msp,
                    cruise_speed(veh).to_f32(),
                );
            }
            slow_car_down_for_peds_sector_list_call(
                lists.byte_add(4),
                veh,
                min_x,
                min_y,
                max_x,
                max_y,
                msp,
                cruise_speed(veh).to_f32(),
            );
            slow_car_down_for_objects_sector_list_call(
                lists.byte_add(8),
                veh,
                min_x,
                min_y,
                max_x,
                max_y,
                msp,
                cruise_speed(veh).to_f32(),
            );
            xx += 1;
        }
        y += 1;
    }
    wr::<u8>(veh, 0x429, rd::<u8>(veh, 0x429) | 0x20);
    let ta = rd::<u8>(veh, 0x3B9);
    if ta == 0 || ta == 4 {
        return x_(msp.read());
    }
    cruise_speed(veh).fadd_m(msp.read()).fmul_m(0.5)
}

// ---------------------------------------------------------------------------------------------------------
// SteerAICarWithPhysics

orig! {
    /// `CCarCtrl::SwitchBetweenPhysicsAndGhost(CVehicle*)` (0x4222A0)
    fn switch_between_physics_and_ghost(v: GPtr<u8>) = cdecl 0x4222A0 => crate::carctrl2::switch_between_physics_and_ghost;
    /// `CPathFind::AreNodesLoadedForArea(float x0, float x1, float y0, float y1)` (0x44DD10, thiscall on ThePaths)
    fn are_nodes_loaded_for_area(this: GPtr<u8>, x0: f32, x1: f32, y0: f32, y1: f32) -> u8 = thiscall 0x44DD10 => crate::pathfind::are_nodes_loaded_for_area;
    /// `CCarCtrl::JoinCarWithRoadAccordingToMission(CVehicle*)` (0x432CB0)
    fn join_car_with_road_according_to_mission(v: GPtr<u8>) = cdecl 0x432CB0 => unported;
    /// `CCarCtrl::SteerAICarWithPhysics_OnlyMission(CVehicle*, float* steer, float* gas, float* brake, bool* handbrake)` (0x436A90)
    fn steer_only_mission(v: GPtr<u8>, steer: GPtr<f32>, gas: GPtr<f32>, brake: GPtr<f32>, hb: GPtr<u8>) = cdecl 0x436A90 => steer_ai_car_with_physics_only_mission;
    /// `CCarCtrl::FindMaxSteerAngle(CVehicle*)` (0x427FE0, ST0)
    fn find_max_steer_angle(v: GPtr<u8>) -> f64 = cdecl 0x427FE0 => find_max_steer_angle_abi;
    /// `DotProduct(const CVector&, const CVector&)` (0x40FDB0, ST0)
    fn dot_product(a: GPtr<f32>, b: GPtr<f32>) -> f64 = cdecl 0x40FDB0 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, hidden result pointer first)
    fn vector_mul(out: GPtr<f32>, v: GPtr<f32>, f: f32) -> GPtr<f32> = cdecl 0x40FEC0 => unported;
    /// `operator+(const CVector&, const CVector&)` (0x40FE30, hidden result pointer first)
    fn vector_add(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x40FE30 => unported;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn find_max_steer_angle_abi(v: GPtr<u8>) -> f64 {
    crate::carctrl2::find_max_steer_angle(v).to_f64()
}

/// `CTimer::ms_fTimeStep` (0xB7CB5C).
fn time_step() -> f32 {
    g::<f32>(0xB7CB5C).read()
}

/// `CCarCtrl::SteerAICarWithPhysics(CVehicle*)` (0x437C20; its first block continues at 0x4013E0 and jumps
/// back): vehicles whose +0x424 index (>= 0) has a zero byte in 0x97D6C0 are skipped. Otherwise:
/// `SwitchBetweenPhysicsAndGhost`; +0x420 = 0; missions 0x15..0x1D/0x22..0x23 etc. (byte table 0x4382E4,
/// jump table 0x4382D8) without a target entity (+0x41C) fall back to mission 0 (entry 0) or 0x2A
/// (entry 1, mission 0x27). Autopilot flag +0x3DC bit 0: wait (brake 0.2) until the nodes within ±270 are
/// loaded, then clear the bit and `JoinCarWithRoadAccordingToMission`. Bit 1 (with a mission other than 0
/// and 0xB): temp action 0x16 for 2 s, the bit cleared, the current / previous node addresses swapped.
/// Then the temp action (+0x3BB, jump table 0x438310) sets steer / gas / brake / handbrake (reversing,
/// turning, waiting, ...; `OnlyMission` for the rest) and the outputs go to +0x494 (steer), +0x49C (gas),
/// +0x4A0 (brake), +0x428 bit 5 (handbrake); model 0x21B also copies steer / gas to +0x988 / +0x994.
#[hook(addr = 0x437C20, conv = cdecl, name = "CCarCtrl::SteerAICarWithPhysics", expect = "83 EC 34 56 8B 74 24 3C")]
pub fn steer_ai_car_with_physics(veh: GPtr<u8>) {
    let idx = rd::<i8>(veh, 0x424);
    if idx >= 0 && g::<u8>(0x97D6C0 + idx as usize).read() == 0 {
        return;
    }
    switch_between_physics_and_ghost(veh);
    let m = i32::from(rd::<i8>(veh, 0x3BA)) - 15;
    wr::<u32>(veh, 0x420, 0);
    if (m as u32) <= 0x28 {
        match g::<u8>(0x4382E4 + m as usize).read() {
            0 if rd::<u32>(veh, 0x41C) == 0 => wr::<u8>(veh, 0x3BA, 0),
            1 if rd::<u32>(veh, 0x41C) == 0 => wr::<u8>(veh, 0x3BA, 0x2A),
            _ => {}
        }
    }
    // [esp+0x10] steer, [esp+0x14] gas, [esp+0x18] brake, [esp+0xf] handbrake (zeroed at entry)
    let mut steer = [0f32];
    let mut gas = [0f32];
    let mut brake_l = [0f32];
    let mut hb_l = [0u8];
    let mut hb: u8 = 0; // BL
    // `None`: the brake is the local [esp+0x18] (0x43827E); `Some(v)`: the value on the x87 stack (0x438282)
    let brake: Option<X87>;
    let now = time_ms;
    let flags = rd::<u8>(veh, 0x3DC);
    if flags & 1 != 0 {
        hb = 0;
        gas[0] = 0.0;
        steer[0] = 0.0;
        brake_l[0] = k(0x3E4C_CCCD);
        let p = entity_pos(veh);
        let (px, py) = (vx(p, 0), vx(p, 1));
        let y1 = x_(py).fadd_m(270.0).to_f32();
        let y0 = x_(py).fsub_m(270.0).to_f32();
        let x1 = x_(px).fadd_m(270.0).to_f32();
        let x0 = x_(px).fsub_m(270.0).to_f32();
        if are_nodes_loaded_for_area(g(0x96F050), x0, x1, y0, y1) != 0 {
            wr::<u8>(veh, 0x3DC, rd::<u8>(veh, 0x3DC) & 0xFE);
            join_car_with_road_according_to_mission(veh);
        }
        brake = None;
    } else {
        if flags & 2 != 0 {
            let mission = rd::<u8>(veh, 0x3BA);
            if mission != 0xB && mission != 0 {
                let old394 = rd::<u32>(veh, 0x394);
                wr::<u8>(veh, 0x3BB, 0x16);
                let t = now().wrapping_add(2000);
                wr::<u8>(veh, 0x3DC, flags & 0xFD);
                let old390 = rd::<u32>(veh, 0x390);
                wr::<u32>(veh, 0x3BC, t);
                wr::<u32>(veh, 0x394, old390);
                wr::<u32>(veh, 0x390, old394);
                wr::<u32>(veh, 0x398, old390);
            }
        }
        let ta = rd::<u8>(veh, 0x3BB);
        let expired = || now() > rd::<u32>(veh, 0x3BC);
        let steer_p = GPtr::from_raw(steer.as_mut_ptr());
        let gas_p = GPtr::from_raw(gas.as_mut_ptr());
        let brake_p = GPtr::from_raw(brake_l.as_mut_ptr());
        let hb_p = GPtr::from_raw(hb_l.as_mut_ptr());
        let only_mission = |hb: &mut u8| {
            steer_only_mission(veh, steer_p, gas_p, brake_p, hb_p);
            *hb = hb_p.read();
        };
        match ta {
            1 | 24 => {
                let b = if ta == 1 { x_(k(0x3E4C_CCCD)) } else { x_(1.0) };
                hb = 0;
                gas[0] = 0.0;
                steer[0] = 0.0;
                if expired() {
                    wr::<u8>(veh, 0x3BB, 0);
                    wr::<u32>(veh, 0x3AC, now());
                    wr::<u32>(veh, 0x3B0, now());
                }
                brake = Some(b);
            }
            3 => {
                steer_only_mission(veh, steer_p, gas_p, brake_p, hb_p);
                steer[0] = x_(steer[0]).fchs().to_f32();
                hb = 0;
                let m = rd::<GPtr<u8>>(veh, 0x14).byte_add(0x10);
                let d = X87::from_f64_exact(dot_product(veh.byte_add(0x44).cast(), m.cast()));
                let b = if le41(d, x_(k(0x3D23_D70A))) {
                    gas[0] = -0.5;
                    x_(0.0)
                } else {
                    gas[0] = 0.0;
                    x_(0.5)
                };
                if expired() {
                    wr::<u8>(veh, 0x3BB, 0);
                    wr::<u32>(veh, 0x3AC, now());
                }
                brake = Some(b);
            }
            13 | 14 => {
                gas[0] = -0.75;
                brake_l[0] = 0.0;
                hb = 0;
                let a = X87::from_f64_exact(find_max_steer_angle(veh));
                steer[0] = if ta == 13 { a.to_f32() } else { a.fchs().to_f32() };
                if expired() {
                    wr::<u8>(veh, 0x3BB, 0);
                    wr::<u32>(veh, 0x3AC, now());
                }
                brake = None;
            }
            4..=9 => {
                let (h, s, gv) = match ta {
                    4 => (1, 1.0, 0.0),
                    5 => (1, -1.0, 0.0),
                    6 => (1, 0.0, 0.0),
                    7 => (0, 1.0, 1.0),
                    8 => (0, -1.0, 1.0),
                    _ => (0, 0.0, 0.5),
                };
                hb = h;
                steer[0] = s;
                gas[0] = gv;
                if expired() {
                    wr::<u8>(veh, 0x3BB, 0);
                }
                brake = Some(x_(0.0));
            }
            10 | 11 | 20 | 21 => {
                hb = 0;
                steer[0] = if ta == 11 || ta == 21 { 0.25 } else { -0.25 };
                let until = rd::<u32>(veh, 0x3BC);
                let t = now();
                gas[0] = 0.0;
                if t > until.wrapping_sub(1250) {
                    steer[0] = x_(steer[0]).fchs().to_f32();
                }
                if t > until {
                    if ta == 20 || ta == 21 {
                        wr::<u8>(veh, 0x3BB, 1);
                        wr::<u32>(veh, 0x3BC, now().wrapping_add(4000));
                    } else {
                        wr::<u8>(veh, 0x3BB, 0);
                    }
                }
                brake = Some(x_(k(0x3A83_126F)));
            }
            19 => {
                let until = rd::<u32>(veh, 0x3BC);
                let t = now();
                hb = 0;
                gas[0] = 0.0;
                let b = if t > until { x_(1.0) } else { x_(0.0) };
                let cur = rd::<f32>(veh, 0x494);
                let step = x_(time_step()).fmul(X87::from_f64_exact(0.05));
                let v = if !le41(x_(cur), x_(0.0)) {
                    let v = step.fadd_m(cur);
                    if ge5(X87::from_f64_exact(0.5), v) {
                        v
                    } else {
                        X87::from_f64_exact(0.5)
                    }
                } else {
                    let v = x_(cur).fsub(x_(time_step()).fmul(X87::from_f64_exact(0.05)));
                    if le41(X87::from_f64_exact(-0.5), v) {
                        v
                    } else {
                        X87::from_f64_exact(-0.5)
                    }
                };
                steer[0] = v.to_f32();
                if t > until {
                    wr::<u8>(veh, 0x3BB, 0);
                }
                brake = Some(b);
            }
            22 => {
                hb = 0;
                steer[0] = 0.0;
                let m = rd::<GPtr<u8>>(veh, 0x14).byte_add(0x10);
                let d = X87::from_f64_exact(dot_product(veh.byte_add(0x44).cast(), m.cast()));
                let b = if le41(d, x_(k(0x3DCC_CCCD))) {
                    gas[0] = -0.5;
                    x_(0.0)
                } else {
                    gas[0] = 0.0;
                    x_(0.5)
                };
                if expired() {
                    wr::<u8>(veh, 0x3BB, 0);
                }
                brake = Some(b);
            }
            23 => {
                let f = x_(time_step()).fmul_m(k(0x3C44_9BA6)).to_f32();
                steer[0] = rd::<f32>(veh, 0x494);
                hb = 0;
                gas[0] = 1.0;
                brake_l[0] = 0.0;
                let ms: GPtr<f32> = veh.byte_add(0x44).cast();
                let mut fwd = [0f32; 3]; // [esp+0x1c]
                let mut scaled = [0f32; 3]; // [esp+0x28]
                let mut sum = [0f32; 3]; // [esp+0x30]
                let fp = get_forward(veh, GPtr::from_raw(fwd.as_mut_ptr()));
                let sp = vector_mul(GPtr::from_raw(scaled.as_mut_ptr()), fp, f);
                let r = vector_add(GPtr::from_raw(sum.as_mut_ptr()), ms, sp);
                for i in 0..3 {
                    ms.wrapping_index(i).write(r.wrapping_index(i).read());
                }
                if expired() {
                    wr::<u8>(veh, 0x3BB, 0);
                }
                brake = None;
            }
            15..=18 => {
                if expired() {
                    wr::<u8>(veh, 0x3BB, 0);
                }
                only_mission(&mut hb);
                brake = None;
            }
            _ => {
                only_mission(&mut hb);
                brake = None;
            }
        }
    }
    let b = match brake {
        Some(v) => v,
        None => x_(brake_l[0]),
    };
    wr::<f32>(veh, 0x4A0, b.to_f32());
    wr::<f32>(veh, 0x494, steer[0]);
    let al = rd::<u8>(veh, 0x428);
    let nb = (hb.wrapping_shl(5) ^ al) & 0x20;
    wr::<u8>(veh, 0x428, al ^ nb);
    wr::<f32>(veh, 0x49C, gas[0]);
    if rd::<u16>(veh, 0x22) == 0x21B {
        wr::<f32>(veh, 0x994, gas[0]);
        wr::<f32>(veh, 0x988, steer[0]);
    }
}

orig! {
    /// `CCarCtrl::SteerAICarWithPhysicsFollowPath(CVehicle*, float*, float*, float*, bool*)` (0x434900)
    fn steer_follow_path(v: GPtr<u8>, steer: GPtr<f32>, gas: GPtr<f32>, brake: GPtr<f32>, hb: GPtr<u8>) = cdecl 0x434900 => steer_ai_car_with_physics_follow_path;
    /// `CCarCtrl::SteerAICarParkPerpendicular(CVehicle*, float*, float*, float*, bool*)` (0x433EA0)
    fn steer_park_perpendicular(v: GPtr<u8>, steer: GPtr<f32>, gas: GPtr<f32>, brake: GPtr<f32>, hb: GPtr<u8>) = cdecl 0x433EA0 => unported;
    /// `CCarCtrl::SteerAICarParkParallel(CVehicle*, float*, float*, float*, bool*)` (0x433BA0)
    fn steer_park_parallel(v: GPtr<u8>, steer: GPtr<f32>, gas: GPtr<f32>, brake: GPtr<f32>, hb: GPtr<u8>) = cdecl 0x433BA0 => unported;
    /// `CCarCtrl::SteerAICarWithPhysicsFollowPath_Racing(CVehicle*, float*, float*, float*, bool*)` (0x435830)
    fn steer_follow_path_racing(v: GPtr<u8>, steer: GPtr<f32>, gas: GPtr<f32>, brake: GPtr<f32>, hb: GPtr<u8>) = cdecl 0x435830 => unported;
    /// `CCarCtrl::SteerAICarWithPhysicsFollowPreRecordedPath(CVehicle*, float*, float*, float*, bool*)` (0x432DD0)
    fn steer_follow_pre_recorded_path(v: GPtr<u8>, steer: GPtr<f32>, gas: GPtr<f32>, brake: GPtr<f32>, hb: GPtr<u8>) = cdecl 0x432DD0 => unported;
    /// `CCarCtrl::SteerAICarWithPhysicsHeadingForTarget(CVehicle*, CPhysical*, float x, float y, float*, float*, float*, bool*)` (0x433280)
    #[allow(clippy::too_many_arguments)]
    fn steer_heading_for_target(v: GPtr<u8>, target: GPtr<u8>, x: f32, y: f32, steer: GPtr<f32>, gas: GPtr<f32>, brake: GPtr<f32>, hb: GPtr<u8>) = cdecl 0x433280 => steer_ai_car_with_physics_heading_for_target;
    /// `CCarCtrl::SteerAICarWithPhysicsTryingToBlockTarget_Stop(CVehicle*, float x, float y, float vx, float vy, float*, float*, float*, bool*)` (0x428990)
    #[allow(clippy::too_many_arguments)]
    fn steer_trying_to_block_target_stop(v: GPtr<u8>, x: f32, y: f32, vx: f32, vy: f32, steer: GPtr<f32>, gas: GPtr<f32>, brake: GPtr<f32>, hb: GPtr<u8>) = cdecl 0x428990 => steer_ai_car_with_physics_trying_to_block_target_stop;
    /// `CCarCtrl::SteerAIHeliTowardsTargetCoors(CAutomobile*)` (0x42A630)
    fn steer_ai_heli_towards_target_coors_call(v: GPtr<u8>) = cdecl 0x42A630 => steer_ai_heli_towards_target_coors;
    /// `CCarCtrl::SteerAIPlaneTowardsTargetCoors(CAutomobile*)` (0x423790)
    fn steer_ai_plane_towards_target_coors_call(v: GPtr<u8>) = cdecl 0x423790 => steer_ai_plane_towards_target_coors;
    /// `CCarCtrl::SteerAIBoatWithPhysicsHeadingForTarget(CVehicle*, float x, float y, float*, float*, float*)` (0x428BE0)
    fn steer_ai_boat_heading_for_target(v: GPtr<u8>, x: f32, y: f32, steer: GPtr<f32>, gas: GPtr<f32>, brake: GPtr<f32>) = cdecl 0x428BE0 => steer_ai_boat_with_physics_heading_for_target;
    /// `CCarCtrl::SteerAIBoatWithPhysicsAttackingPlayer(CVehicle*, float*, float*, float*, bool*)` (0x428DE0)
    fn steer_ai_boat_attacking_player(v: GPtr<u8>, steer: GPtr<f32>, gas: GPtr<f32>, brake: GPtr<f32>, hb: GPtr<u8>) = cdecl 0x428DE0 => steer_ai_boat_with_physics_attacking_player;
    /// `CCarCtrl::GetAIPlaneToDoDogFight(CAutomobile*)` (0x429890)
    fn get_ai_plane_to_do_dog_fight_call(v: GPtr<u8>) = cdecl 0x429890 => get_ai_plane_to_do_dog_fight;
    /// `CCarCtrl::GetAIPlaneToDoDogFightAgainstPlayer(CAutomobile*)` (0x42F370)
    fn get_ai_plane_to_do_dog_fight_against_player_call(v: GPtr<u8>) = cdecl 0x42F370 => get_ai_plane_to_do_dog_fight_against_player;
}

/// `CCarCtrl::SteerAICarWithPhysics_OnlyMission(CVehicle*, float* steer, float* gas, float* brake, bool*
/// handbrake)` (0x436A90), partial port (ADR-002 fallback): the mission byte (+0x3BA, byte table 0x437BD4,
/// jump table 0x437B24) selects the steering. PORTED: missions > 0x44 and 7, 0x1B, 0x1C (nothing);
/// 0, 0xA, 0xB (stopped: steer 0, gas 0, handbrake, brake 0.5); the follow-path set (1, 2, 4, 8, 0xC, 0xF,
/// 0x11, 0x2B, 0x34, 0x36, 0x3D, 0x41..0x44) → `SteerAICarWithPhysicsFollowPath`; 0x21 → `_Racing`;
/// 0x22 → `FollowPreRecordedPath`; 9, 0xD → `HeadingForTarget(veh, null, +0x3EC, +0x3F0, ..)`; 0x13 →
/// `TryingToBlockTarget_Stop` with the target entity's position and speed; 0x14 → heli towards target;
/// 0x16 → plane towards target; 0xE → boat heading for +0x3EC/+0x3F0 (handbrake cleared); 0x15 → boat
/// attacking the player; 0x3E / 0x3F → the dog fights. NOT PORTED (fallback to the whole original body,
/// decided at entry from the mission byte): 3, 5, 6, 0x10, 0x12, 0x17..0x1A, 0x1D..0x20, 0x23..0x2A,
/// 0x2C..0x33, 0x35, 0x37..0x3C, 0x40 (ram / block / escort / park / follow-car / heli and plane / boat
/// circling / misc. missions with their own inline steering).
#[hook(addr = 0x436A90, conv = cdecl, name = "CCarCtrl::SteerAICarWithPhysics_OnlyMission", expect = "81 EC E4 00 00 00", fallback)]
pub fn steer_ai_car_with_physics_only_mission(
    veh: GPtr<u8>,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
    hb: GPtr<u8>,
) {
    let mission = i32::from(rd::<i8>(veh, 0x3BA));
    if !(0..=0x44).contains(&mission) {
        return;
    }
    match mission {
        7 | 0x1B | 0x1C => {}
        0 | 0xA | 0xB => {
            steer.write(0.0);
            gas.write(0.0);
            hb.write(1);
            brake.write(0.5);
        }
        1 | 2 | 4 | 8 | 0xC | 0xF | 0x11 | 0x2B | 0x34 | 0x36 | 0x3D | 0x41..=0x44 => {
            steer_follow_path(veh, steer, gas, brake, hb)
        }
        0x21 => steer_follow_path_racing(veh, steer, gas, brake, hb),
        0x22 => steer_follow_pre_recorded_path(veh, steer, gas, brake, hb),
        9 | 0xD => steer_heading_for_target(
            veh,
            GPtr::NULL,
            rd::<f32>(veh, 0x3EC),
            rd::<f32>(veh, 0x3F0),
            steer,
            gas,
            brake,
            hb,
        ),
        0x13 => {
            let t = rd::<GPtr<u8>>(veh, 0x41C);
            let p = entity_pos(t);
            steer_trying_to_block_target_stop(
                veh,
                vx(p, 0),
                vx(p, 1),
                rd::<f32>(t, 0x44),
                rd::<f32>(t, 0x48),
                steer,
                gas,
                brake,
                hb,
            );
        }
        0x14 => steer_ai_heli_towards_target_coors_call(veh),
        0x16 => steer_ai_plane_towards_target_coors_call(veh),
        0xE => {
            steer_ai_boat_heading_for_target(
                veh,
                rd::<f32>(veh, 0x3EC),
                rd::<f32>(veh, 0x3F0),
                steer,
                gas,
                brake,
            );
            hb.write(0);
        }
        0x15 => steer_ai_boat_attacking_player(veh, steer, gas, brake, hb),
        // STD-14: the parking missions (0x437939 / 0x43796D)
        0x2D | 0x31 => steer_park_perpendicular(veh, steer, gas, brake, hb),
        0x2E | 0x32 => steer_park_parallel(veh, steer, gas, brake, hb),
        0x3E => get_ai_plane_to_do_dog_fight_call(veh),
        0x3F => get_ai_plane_to_do_dog_fight_against_player_call(veh),
        _ => steer_ai_car_with_physics_only_mission_original(veh, steer, gas, brake, hb),
    }
}

// ---------------------------------------------------------------------------------------------------------
// Weaving through traffic

orig! {
    /// `CCarCtrl::WeaveForOtherCar(CEntity*, CVehicle*, float*, float*)` (0x426350)
    fn weave_for_other_car_call(e: GPtr<u8>, veh: GPtr<u8>, a: GPtr<f32>, b: GPtr<f32>) = cdecl 0x426350 => weave_for_other_car;
    /// The unnamed ped counterpart of `WeaveForOtherCar` (0x426970, `(CPed*, CVehicle*, float*, float*)`)
    fn weave_for_ped(e: GPtr<u8>, veh: GPtr<u8>, a: GPtr<f32>, b: GPtr<f32>) = cdecl 0x426970 => unported;
    /// `CCarCtrl::WeaveForObject(CEntity*, CVehicle*, float*, float*)` (0x426BC0)
    fn weave_for_object_call(e: GPtr<u8>, veh: GPtr<u8>, a: GPtr<f32>, b: GPtr<f32>) = cdecl 0x426BC0 => weave_for_object;
    /// `CCarCtrl::WeaveThroughCarsSectorList` (0x42D680)
    #[allow(clippy::too_many_arguments)]
    fn weave_through_cars_sector_list_call(list: GPtr<u8>, veh: GPtr<u8>, phys: GPtr<u8>, min_x: f32, min_y: f32, max_x: f32, max_y: f32, a: GPtr<f32>, b: GPtr<f32>) = cdecl 0x42D680 => weave_through_cars_sector_list;
    /// `CCarCtrl::WeaveThroughPedsSectorList` (0x42D7E0)
    #[allow(clippy::too_many_arguments)]
    fn weave_through_peds_sector_list_call(list: GPtr<u8>, veh: GPtr<u8>, phys: GPtr<u8>, min_x: f32, min_y: f32, max_x: f32, max_y: f32, a: GPtr<f32>, b: GPtr<f32>) = cdecl 0x42D7E0 => weave_through_peds_sector_list;
    /// `CCarCtrl::WeaveThroughObjectsSectorList` (0x42D950)
    #[allow(clippy::too_many_arguments)]
    fn weave_through_objects_sector_list_call(list: GPtr<u8>, veh: GPtr<u8>, min_x: f32, min_y: f32, max_x: f32, max_y: f32, a: GPtr<f32>, b: GPtr<f32>) = cdecl 0x42D950 => weave_through_objects_sector_list;
}

/// `|e.z - veh.z|` as the sector loops compute it (`fcomp 0; test ah, 5; jp` keeps the difference, else
/// `fchs`), compared `>= lim` (true = too far).
fn z_too_far(e: GPtr<u8>, veh: GPtr<u8>, lim: f32) -> bool {
    let d = x_(vx(entity_pos(e), 2)).fsub_m(vx(entity_pos(veh), 2));
    let d = if ge5(d, x_(0.0)) { d } else { d.fchs() };
    ge5(d, x_(lim))
}

/// The strict box test of the sector loops on a point: `x <= minX`, `x >= maxX`, `y <= minY`, `y >= maxY` reject.
fn outside_box(px: f32, py: f32, min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> bool {
    le41(x_(px), x_(min_x)) || ge5(x_(px), x_(max_x)) || le41(x_(py), x_(min_y)) || ge5(x_(py), x_(max_y))
}

/// `CCarCtrl::WeaveThroughCarsSectorList(CPtrListDoubleLink&, CVehicle* veh, CPhysical* ignore, float minX,
/// float minY, float maxX, float maxY, float* a, float* b)` (0x42D680): entities not yet scanned, using
/// collision and other than `ignore` get the scan code; with the bound centre strictly inside the box,
/// `|dz| < 8`, not `veh` itself and not both flagged +0x42B bit 3 → `WeaveForOtherCar(e, veh, a, b)`.
#[hook(addr = 0x42D680, conv = cdecl, name = "CCarCtrl::WeaveThroughCarsSectorList<CPtrListDoubleLink<CPhysical*>>", expect = "8B 44 24 04 83 EC 0C")]
#[allow(clippy::too_many_arguments)]
pub fn weave_through_cars_sector_list(
    list: GPtr<u8>,
    veh: GPtr<u8>,
    ignore: GPtr<u8>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    a: GPtr<f32>,
    b: GPtr<f32>,
) {
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        let sc = scan_code();
        node = rd::<GPtr<u8>>(node, 4);
        if rd::<u16>(e, 0x2C) == sc || rd::<u8>(e, 0x1C) & 1 == 0 || e == ignore {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        let mut c = [0f32; 3];
        get_bound_centre(e, GPtr::from_raw(c.as_mut_ptr()));
        if outside_box(c[0], c[1], min_x, min_y, max_x, max_y) || z_too_far(e, veh, 8.0) || e == veh {
            continue;
        }
        if rd::<u8>(veh, 0x42B) & 8 != 0 && rd::<u8>(e, 0x42B) & 8 != 0 {
            continue;
        }
        weave_for_other_car_call(e, veh, a, b);
    }
}

/// `CCarCtrl::WeaveThroughPedsSectorList(CPtrListDoubleLink&, CVehicle* veh, CPhysical* ignore, float minX,
/// float minY, float maxX, float maxY, float* a, float* b)` (0x42D7E0): as the cars loop on the ped
/// positions, `|dz| < 4`, peds not in `veh` (+0x584) nor standing on it (+0xFC) → the ped weave (0x426970).
#[hook(addr = 0x42D7E0, conv = cdecl, name = "CCarCtrl::WeaveThroughPedsSectorList<CPtrListDoubleLink<CPhysical*>>", expect = "8B 44 24 04 53 8B 18")]
#[allow(clippy::too_many_arguments)]
pub fn weave_through_peds_sector_list(
    list: GPtr<u8>,
    veh: GPtr<u8>,
    ignore: GPtr<u8>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    a: GPtr<f32>,
    b: GPtr<f32>,
) {
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        let sc = scan_code();
        node = rd::<GPtr<u8>>(node, 4);
        if rd::<u16>(e, 0x2C) == sc || rd::<u8>(e, 0x1C) & 1 == 0 || e == ignore {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        let p = entity_pos(e);
        if outside_box(vx(p, 0), vx(p, 1), min_x, min_y, max_x, max_y) || z_too_far(e, veh, 4.0) {
            continue;
        }
        if rd::<GPtr<u8>>(e, 0x584) == veh || rd::<GPtr<u8>>(e, 0xFC) == veh {
            continue;
        }
        weave_for_ped(e, veh, a, b);
    }
}

/// `CCarCtrl::WeaveThroughObjectsSectorList(CPtrListDoubleLink&, CVehicle* veh, float minX, float minY, float
/// maxX, float maxY, float* a, float* b)` (0x42D950): entities not yet scanned and using collision get the
/// scan code; positions strictly inside the box, `|dz| < 8` and standing upright (`GetMatrix()` +0x28, up.z,
/// above 0.9) → `WeaveForObject(e, veh, a, b)`.
#[hook(addr = 0x42D950, conv = cdecl, name = "CCarCtrl::WeaveThroughObjectsSectorList<CPtrListDoubleLink<CPhysical*>>", expect = "8B 44 24 04 53 8B 18")]
#[allow(clippy::too_many_arguments)]
pub fn weave_through_objects_sector_list(
    list: GPtr<u8>,
    veh: GPtr<u8>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    a: GPtr<f32>,
    b: GPtr<f32>,
) {
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        let sc = scan_code();
        node = rd::<GPtr<u8>>(node, 4);
        if rd::<u16>(e, 0x2C) == sc || rd::<u8>(e, 0x1C) & 1 == 0 {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        let p = entity_pos(e);
        if outside_box(vx(p, 0), vx(p, 1), min_x, min_y, max_x, max_y) || z_too_far(e, veh, 8.0) {
            continue;
        }
        if le41(x_(rd::<f32>(get_matrix(e), 0x28)), x_(k(0x3F66_6666))) {
            continue;
        }
        weave_for_object_call(e, veh, a, b);
    }
}

/// Wraps an x87 angle into [-π, π] as the inlined loops do: `+2π` while `< -π` (the first test `jp`
/// skips when `>= -π`), then `-2π` while `> π` (`test ah, 0x41; jne` skips when `<= π`).
fn wrap_pi(mut a: X87) -> X87 {
    let npi = x_(k(0xC049_0FDB));
    let pi = x_(k(0x4049_0FDB));
    let tau = k(0x40C9_0FDB);
    if !ge5(a, npi) {
        loop {
            a = a.fadd_m(tau);
            if ge5(a, npi) {
                break;
            }
        }
    }
    if !le41(a, pi) {
        loop {
            a = a.fsub_m(tau);
            if le41(a, pi) {
                break;
            }
        }
    }
    a
}

/// `|a|` as `fcom 0; test ah, 5; jp` (keep) / `fchs`.
fn abs5(a: X87) -> X87 {
    if ge5(a, x_(0.0)) { a } else { a.fchs() }
}

/// `CCarCtrl::FindAngleToWeaveThroughTraffic(CVehicle* veh, CPhysical* ignore, float heading, float target,
/// float range)` (0x4325C0, ST0): box half-size `min(|v|·2.5 + 1, 2) * range * 12` around the car; both
/// limits (`right` = [arg slot 1], `left` = [arg slot 5], starting at `heading`) are narrowed by the
/// cars / peds (unless temp action 6) / objects / cars-again weave of every repeat sector of the box,
/// repeated (same scan code) until neither changes; then with `mid = wrap(heading + wrap(target -
/// heading) / 2)`: both `|wrap(left - mid)|` and `|wrap(right - mid)|` above π/2 → `mid`; else the side
/// closer to `mid` by at least 0.08 (`right` on ties below 0.08) — `right` when `|left| >= |right|`.
#[hook(addr = 0x4325C0, conv = cdecl, name = "CCarCtrl::FindAngleToWeaveThroughTraffic", expect = "83 EC 2C 53 55")]
pub fn find_angle_to_weave_through_traffic(
    veh: GPtr<u8>,
    ignore: GPtr<u8>,
    heading: f32,
    target: f32,
    range: f32,
) -> X87 {
    let (vxv, vyv) = (rd::<f32>(veh, 0x44), rd::<f32>(veh, 0x48));
    let len = x_(vxv).fmul_m(vxv).fadd(x_(vyv).fmul_m(vyv)).fsqrt();
    let s = len.fmul_m(2.5).fadd_m(1.0);
    let s = if !ge5(s, x_(2.0)) { s } else { x_(2.0) };
    let r = s.fmul_m(range).fmul_m(12.0);
    let p = entity_pos(veh);
    let min_x = x_(vx(p, 0)).fsub(r).to_f32();
    let max_x = r.fadd_m(vx(p, 0)).to_f32();
    let min_y = x_(vx(p, 1)).fsub(r).to_f32();
    let max_y = r.fadd_m(vx(p, 1)).to_f32();
    let lo = |v: f32| {
        let (n, s32) = sector_of(v);
        if n > 0 {
            ftol32(X87::from_f64_exact(crt_floor(f64::from(s32))))
        } else {
            0
        }
    };
    let hi = |v: f32| {
        let (n, s32) = sector_of(v);
        if n < 0x77 {
            ftol32(X87::from_f64_exact(crt_floor(f64::from(s32))))
        } else {
            0x77
        }
    };
    let x0 = lo(min_x);
    let y0 = lo(min_y);
    let x1 = hi(max_x);
    let y1 = hi(max_y);
    let mut right = [heading]; // [esp+0x40]
    let mut left = [heading]; // [esp+0x50]
    let sc = g::<u16>(0xB7CD78);
    if sc.read() < 0xFFFF {
        sc.write(sc.read() + 1);
    } else {
        world_clear_scan_codes();
        sc.write(1);
    }
    let rp = GPtr::from_raw(right.as_mut_ptr());
    let lp = GPtr::from_raw(left.as_mut_ptr());
    let mut prev_r = k(0xC61C_3F9A);
    let mut prev_l = k(0xC61C_3F9A);
    loop {
        // `test ah, 0x44; jp`: another pass when a value changed (or compares unordered)
        let same_l = !x_(prev_l).fcom(x_(lp.read())).test_parity(0x44);
        let same_r = !x_(prev_r).fcom(x_(rp.read())).test_parity(0x44);
        if same_l && same_r {
            break;
        }
        prev_r = rp.read();
        prev_l = lp.read();
        let mut y = y0;
        while y <= y1 {
            let mut xx = x0;
            while xx <= x1 {
                let lists = g::<u8>(0xB992B8).byte_add((((xx & 0xF) + ((y & 0xF) << 4)) * 12) as isize);
                weave_through_cars_sector_list_call(lists, veh, ignore, min_x, min_y, max_x, max_y, lp, rp);
                if rd::<u8>(veh, 0x3B9) != 6 {
                    weave_through_peds_sector_list_call(
                        lists.byte_add(4),
                        veh,
                        ignore,
                        min_x,
                        min_y,
                        max_x,
                        max_y,
                        lp,
                        rp,
                    );
                }
                weave_through_objects_sector_list_call(
                    lists.byte_add(8),
                    veh,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    lp,
                    rp,
                );
                weave_through_cars_sector_list_call(lists, veh, ignore, min_x, min_y, max_x, max_y, lp, rp);
                xx += 1;
            }
            y += 1;
        }
    }
    let d = wrap_pi(x_(target).fsub_m(heading));
    let mid = wrap_pi(d.fmul_m(0.5).fadd_m(heading));
    let mid32 = mid.to_f32();
    let a = abs5(wrap_pi(mid.fsubr_m(lp.read())));
    let b = abs5(wrap_pi(x_(rp.read()).fsub_m(mid32)));
    let half_pi = x_(k(0x3FC9_0FDB));
    if !le41(a, half_pi) && !le41(b, half_pi) {
        return x_(mid32);
    }
    if !ge5(abs5(a.fsub(b)), x_(k(0x3DA3_D70A))) {
        return x_(rp.read());
    }
    if ge5(a, b) { x_(rp.read()) } else { x_(lp.read()) }
}

orig! {
    /// `CBaseModelInfo::SwaysInWind()` (stub 0x4212C0; the caller tests the whole EAX)
    fn sways_in_wind(mi: GPtr<u8>) -> u32 = thiscall 0x4212C0 => unported;
}

/// `CPlaceable::GetMatrix` inlined: allocate and update the matrix from the placement when there is none.
fn ensure_matrix(e: GPtr<u8>) {
    if rd::<GPtr<u8>>(e, 0x14).is_null() {
        allocate_matrix(e);
        update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
}

/// Wraps `a` into [-π, π) the way the weave functions store a limit: `fst` to `*p`, `+2π` while `< -π`
/// (reloaded from `*p` once the loop runs), the result stored.
fn store_wrapped_low(p: GPtr<f32>, a: X87) {
    let npi = x_(k(0xC049_0FDB));
    p.write(a.to_f32());
    if ge5(a, npi) {
        return;
    }
    let mut v = x_(p.read());
    loop {
        v = v.fadd_m(k(0x40C9_0FDB));
        if ge5(v, npi) {
            break;
        }
    }
    p.write(v.to_f32());
}

/// The mirror of [`store_wrapped_low`]: `-2π` while `> π`.
fn store_wrapped_high(p: GPtr<f32>, a: X87) {
    let pi = x_(k(0x4049_0FDB));
    p.write(a.to_f32());
    if le41(a, pi) {
        return;
    }
    let mut v = x_(p.read());
    loop {
        v = v.fsub_m(k(0x40C9_0FDB));
        if le41(v, pi) {
            break;
        }
    }
    p.write(v.to_f32());
}

/// `CCarCtrl::WeaveForObject(CEntity* obj, CVehicle* veh, float* a, float* b)` (0x426BC0): models with a
/// known offset (ModelIndices 0x8CD4F4 / 0x8CD518 / 0x8CD51C / 0x8CD520 / 0x8CD524; other models only when
/// `SwaysInWind`, offset 0) are treated at `pos + right * ox + forward * oy`; with `ang = GetATanOfXY` of
/// the offset from the car and `q = (car max.x * 2.4 + 0.3) / dist / 2`, a limit within `q` of `ang` is
/// pushed out to `ang - q` (`a`) / `ang + q` (`b`), wrapped into [-π, π].
#[hook(addr = 0x426BC0, conv = cdecl, name = "CCarCtrl::WeaveForObject", expect = "0F B7 0D F4 D4 8C 00")]
pub fn weave_for_object(obj: GPtr<u8>, veh: GPtr<u8>, a: GPtr<f32>, b: GPtr<f32>) {
    let mi = i32::from(rd::<i16>(obj, 0x22));
    let is = |addr: usize| mi == i32::from(g::<u16>(addr).read());
    let (ox, oy) = if is(0x8CD4F4) {
        (0x403D_3F7D, 0x3E16_872B)
    } else if is(0x8CD518) {
        (0x3F3E_76C9, 0)
    } else if is(0x8CD51C) {
        (0x3D30_20C5, 0)
    } else if is(0x8CD520) {
        (0x3F92_4DD3, 0x3E14_7AE1)
    } else if is(0x8CD524) {
        (0, 0xBD44_9BA6)
    } else {
        if sways_in_wind(crate::common::model_info(mi)) == 0 {
            return;
        }
        (0, 0)
    };
    let (ox, oy) = (k(ox), k(oy));
    let p = entity_pos(obj);
    let (px, py) = (vx(p, 0), vx(p, 1));
    ensure_matrix(obj);
    ensure_matrix(obj);
    let m1 = rd::<GPtr<u8>>(obj, 0x14);
    let nx = x_(oy)
        .fmul_m(rd::<f32>(m1, 0x10))
        .fadd(x_(ox).fmul_m(rd::<f32>(m1, 0)))
        .fadd_m(px)
        .to_f32();
    ensure_matrix(obj);
    ensure_matrix(obj);
    let m2 = rd::<GPtr<u8>>(obj, 0x14);
    let ny = x_(ox)
        .fmul_m(rd::<f32>(m2, 4))
        .fadd(x_(oy).fmul_m(rd::<f32>(m2, 0x14)))
        .fadd_m(py);
    let pv = entity_pos(veh);
    let dx = x_(nx).fsub_m(vx(pv, 0)).to_f32();
    let dy = ny.fsub_m(vx(pv, 1)).to_f32();
    let ang = X87::from_f64_exact(crate::common::get_atan_of_xy(dx, dy));
    let r = x_(rd::<f32>(col_model(veh), 0xC))
        .fmul_m(k(0x4019_999A))
        .fadd_m(k(0x3E99_999A));
    let dist = x_(dy).fmul_m(dy).fadd(x_(dx).fmul_m(dx)).fsqrt();
    let q = r.fdiv(dist).to_f32();
    let q = x_(q).fmul_m(0.5).to_f32();
    if !ge5(abs5(wrap_pi(ang.fsub_m(a.read()))), x_(q)) {
        store_wrapped_low(a, x_(q).fsubr(ang));
    }
    if !ge5(abs5(wrap_pi(ang.fsub_m(b.read()))), x_(q)) {
        store_wrapped_high(b, ang.fadd_m(q));
    }
}

orig! {
    /// `CCarCtrl::FindAngleToWeaveThroughTraffic` (0x4325C0, ST0)
    fn find_angle_to_weave_call(veh: GPtr<u8>, ignore: GPtr<u8>, heading: f32, target: f32, range: f32) -> f64 = cdecl 0x4325C0 => find_angle_to_weave_abi;
    /// `CCarCtrl::FindSpeedMultiplier(float, float, float, float)` (0x4224E0, ST0)
    fn find_speed_multiplier(angle: f32, min: f32, max: f32, coef: f32) -> f64 = cdecl 0x4224E0 => find_speed_multiplier_abi;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn find_angle_to_weave_abi(veh: GPtr<u8>, ignore: GPtr<u8>, heading: f32, target: f32, range: f32) -> f64 {
    find_angle_to_weave_through_traffic(veh, ignore, heading, target, range).to_f64()
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn find_speed_multiplier_abi(angle: f32, min: f32, max: f32, coef: f32) -> f64 {
    crate::carctrl2::find_speed_multiplier(angle, min, max, coef).to_f64()
}

/// `GetATanOfXY` through `orig!` as an x87 value.
fn atan_xy(x: f32, y: f32) -> X87 {
    X87::from_f64_exact(crate::common::get_atan_of_xy(x, y))
}

/// `CCarCtrl::SteerAICarWithPhysicsHeadingForTarget(CVehicle* veh, CPhysical* target, float x, float y,
/// float* steer, float* gas, float* brake, bool* handbrake)` (0x433280): handbrake cleared; the angle to
/// (x, y) (weaved through traffic with range 1 for temp actions 2, 5, 6) minus the heading of the
/// normalised 2D forward, wrapped; handbrake when moving faster than 0.3 with a turn above 0.7; the turn
/// clamped to ±`FindMaxSteerAngle`. The speed multiplier `FindSpeedMultiplier(angle - heading, 0.4, 1.2,
/// 0.4)` times the cruise speed minus the current speed (`|v| * 60`): above 0 → brake 0, gas
/// `min(diff / 10, 1)` (1 at 25 or more); bikes (+0x594 == 10) going on with a diff above 3 and +0x818 at
/// most 0 set +0x818 = 10. Otherwise gas 0, brake `min(-diff / 20, 0.5)`. `*steer` = the clamped turn.
#[hook(addr = 0x433280, conv = cdecl, name = "CCarCtrl::SteerAICarWithPhysicsHeadingForTarget", expect = "83 EC 10 56 8B 74 24 18")]
#[allow(clippy::too_many_arguments)]
pub fn steer_ai_car_with_physics_heading_for_target(
    veh: GPtr<u8>,
    target: GPtr<u8>,
    tx: f32,
    ty: f32,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
    hb: GPtr<u8>,
) {
    hb.write(0);
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let (fx, fy) = forward_2d_normalised(m);
    let p = entity_pos(veh);
    let dy = x_(ty).fsub_m(vx(p, 1)).to_f32();
    let dx = x_(tx).fsub_m(vx(p, 0)).to_f32();
    let ang = atan_xy(dx, dy).to_f32();
    let heading = atan_xy(fx, fy).to_f32();
    let ta = rd::<u8>(veh, 0x3B9);
    let a = if ta == 2 || ta == 5 || ta == 6 {
        X87::from_f64_exact(find_angle_to_weave_call(veh, target, ang, heading, 1.0))
    } else {
        x_(ang)
    };
    let w = wrap_pi(a.fsub_m(heading));
    let mut turn = w.to_f32();
    let (vxx, vyy, vzz) = (rd::<f32>(veh, 0x44), rd::<f32>(veh, 0x48), rd::<f32>(veh, 0x4C));
    let spd = x_(vxx)
        .fmul_m(vxx)
        .fadd(x_(vyy).fmul_m(vyy))
        .fadd(x_(vzz).fmul_m(vzz))
        .fsqrt();
    if !le41(spd, x_(k(0x3E99_999A))) && !le41(abs5(w), x_(k(0x3F33_3333))) {
        hb.write(1);
    }
    let ms = X87::from_f64_exact(find_max_steer_angle(veh));
    let nms = ms.fchs();
    if !ge5(x_(turn), nms) {
        turn = nms.to_f32();
    }
    if !le41(x_(turn), ms) {
        turn = ms.to_f32();
    }
    let p = entity_pos(veh);
    let dy = x_(ty).fsub_m(vx(p, 1)).to_f32();
    let dx = x_(tx).fsub_m(vx(p, 0)).to_f32();
    let rel = atan_xy(dx, dy).fsub_m(heading).to_f32();
    let mult = X87::from_f64_exact(find_speed_multiplier(
        rel,
        k(0x3ECC_CCCD),
        k(0x3F99_999A),
        k(0x3ECC_CCCD),
    ));
    let (vxx, vyy, vzz) = (rd::<f32>(veh, 0x44), rd::<f32>(veh, 0x48), rd::<f32>(veh, 0x4C));
    let cruise = i32::from(rd::<u8>(veh, 0x3D0));
    brake.write(0.0);
    let cur = x_(vzz)
        .fmul_m(vzz)
        .fadd(x_(vyy).fmul_m(vyy))
        .fadd(x_(vxx).fmul_m(vxx))
        .fsqrt()
        .fmul_m(60.0)
        .to_f32();
    let diff = mult.fmul(xi_(cruise)).fsub_m(cur);
    if le41(diff, x_(0.0)) {
        let b = diff.fmul_m(k(0xBD4C_CCCD));
        gas.write(0.0);
        brake.write(if ge5(x_(0.5), b) { b.to_f32() } else { 0.5 });
        steer.write(turn);
        return;
    }
    if ge5(x_(cur), x_(25.0)) {
        gas.write(1.0);
    } else {
        let gv = x_(k(0x3DCC_CCCD)).fmul(diff);
        gas.write(if ge5(x_(1.0), gv) { gv.to_f32() } else { 1.0 });
    }
    if rd::<i32>(veh, 0x594) == 10 && !le41(diff, x_(3.0)) {
        // `fcomp 0; test ah, 0x41; jp`: on when +0x818 > 0 (or unordered)
        if !x_(rd::<f32>(veh, 0x818)).fcom(x_(0.0)).test_parity(0x41) {
            wr::<f32>(veh, 0x818, 10.0);
        }
    }
    steer.write(turn);
}

/// `CCarCtrl::SteerAICarWithPhysicsTryingToBlockTarget(CVehicle* veh, CEntity* unused, float x, float y,
/// float vx, float vy, float* steer, float* gas, float* brake, bool* handbrake)` (0x4335E0): the target
/// velocity, scaled down to length 0.13 when longer, is extrapolated 60 frames ahead; temp action 2;
/// `HeadingForTarget(veh, null, x + vx*60, y + vy*60, ..)`; within 5 of that point the mission becomes
/// 0x13 (from 0x12) or 6.
#[hook(addr = 0x4335E0, conv = cdecl, name = "CCarCtrl::SteerAICarWithPhysicsTryingToBlockTarget", expect = "D9 44 24 18 D9 44 24 14")]
#[allow(clippy::too_many_arguments)]
pub fn steer_ai_car_with_physics_trying_to_block_target(
    veh: GPtr<u8>,
    _unused: GPtr<u8>,
    tx: f32,
    ty: f32,
    tvx: f32,
    tvy: f32,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
    hb: GPtr<u8>,
) {
    let (mut vxx, mut vyy) = (x_(tvx), x_(tvy));
    let len = vxx.fmul(vxx).fadd(vyy.fmul(vyy)).fsqrt();
    let lim = x_(k(0x3E05_1EB8));
    if !le41(len, lim) {
        let s = len.fdivr_m(k(0x3E05_1EB8));
        vxx = vxx.fmul(s);
        vyy = vyy.fmul(s);
    }
    let px = vxx.fmul_m(60.0).fadd_m(tx).to_f32();
    let py = vyy.fmul_m(60.0).fadd_m(ty).to_f32();
    wr::<u8>(veh, 0x3B9, 2);
    steer_heading_for_target(veh, GPtr::NULL, px, py, steer, gas, brake, hb);
    let p = entity_pos(veh);
    let dy = x_(py).fsub_m(vx(p, 1));
    let dx = x_(px).fsub_m(vx(p, 0));
    let d2 = dy.fmul(dy).fadd(dx.fmul(dx));
    if ge5(d2, x_(25.0)) {
        return;
    }
    let m = rd::<u8>(veh, 0x3BA);
    wr::<u8>(veh, 0x3BA, if m == 0x12 { 0x13 } else { 6 });
}

orig! {
    /// `CPathNode::GetPosition(CVector* out)` (0x420A10, thiscall, `ret 4`)
    fn path_node_get_position(node: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x420A10 => crate::pathfind::get_position;
}

/// `ThePaths.m_pPathNodes[area] + node * 0x1C` of a `CNodeAddress` dword (area low word, node high word).
fn path_node(a: u32) -> GPtr<u8> {
    g::<GPtr<u8>>((0x96F854 + (a & 0xFFFF) * 4) as usize)
        .read()
        .byte_add(((a >> 16) * 0x1C) as isize)
}

/// The 2D distance from the vehicle position to a path node as `FindGhostRoadHeight` computes it
/// (`sqrt(dy² + dx²)`).
fn node_dist_2d(node: GPtr<u8>, p: GPtr<f32>) -> X87 {
    let mut v = [0f32; 3];
    let c = path_node_get_position(node, GPtr::from_raw(v.as_mut_ptr()));
    let dx = x_(vx(c, 0)).fsub_m(vx(p, 0));
    let dy = x_(vx(c, 1)).fsub_m(vx(p, 1));
    dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt()
}

/// `CCarCtrl::FindGhostRoadHeight(CVehicle*)` (0x422370, ST0): 0 when either autopilot node (+0x390, +0x394)
/// is unset (word 0xFFFF) or its area is not loaded; else the node heights interpolated by the 2D
/// distances: `(d1 * z2 + d2 * z1) / (d2 + d1)`.
#[hook(addr = 0x422370, conv = cdecl, name = "CCarCtrl::FindGhostRoadHeight", expect = "83 EC 18 57 8B 7C 24 20")]
pub fn find_ghost_road_height(veh: GPtr<u8>) -> X87 {
    if rd::<u16>(veh, 0x390) == 0xFFFF || rd::<u16>(veh, 0x394) == 0xFFFF {
        return x_(0.0);
    }
    let a1 = rd::<u32>(veh, 0x390);
    if !area_loaded(a1 & 0xFFFF) {
        return x_(0.0);
    }
    let a2 = rd::<u32>(veh, 0x394);
    if !area_loaded(a2 & 0xFFFF) {
        return x_(0.0);
    }
    let mut v = [0f32; 3];
    let z1 = vx(
        path_node_get_position(path_node(a1), GPtr::from_raw(v.as_mut_ptr())),
        2,
    );
    let z2 = vx(
        path_node_get_position(path_node(a2), GPtr::from_raw(v.as_mut_ptr())),
        2,
    );
    let d1 = node_dist_2d(path_node(a1), entity_pos(veh)).to_f32();
    let d2 = node_dist_2d(path_node(a2), entity_pos(veh));
    x_(d1).fmul_m(z2).fadd(d2.fmul_m(z1)).fdiv(d2.fadd_m(d1))
}

orig! {
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn task_manager_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask` vtable +0x10 (`GetTaskType`)
    fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CVehicle::IsUpsideDown()` (0x6D1D90)
    fn vehicle_is_upside_down(v: GPtr<u8>) -> u8 = thiscall 0x6D1D90 => unported;
    /// `CCarAI::TellOccupantsToLeaveCar(CVehicle*)` (0x41C760)
    fn tell_occupants_to_leave_car(v: GPtr<u8>) = cdecl 0x41C760 => unported;
}

/// `|v|` of a 3D vector field (+0x44) as `fld z; fld y; fld x` + the squares summed x, y, then z.
fn speed_3d(e: GPtr<u8>) -> X87 {
    let (a, b, c) = (rd::<f32>(e, 0x44), rd::<f32>(e, 0x48), rd::<f32>(e, 0x4C));
    x_(a)
        .fmul_m(a)
        .fadd(x_(b).fmul_m(b))
        .fadd(x_(c).fmul_m(c))
        .fsqrt()
}

/// `CCarCtrl::SteerAICarWithPhysicsTryingToBlockTarget_Stop(CVehicle* veh, float x, float y, float vx, float
/// vy, float* steer, float* gas, float* brake, bool* handbrake)` (0x428990): stopped (steer 0, gas 0,
/// brake 1, handbrake). Farther than 10 from (x, y): mission 0x12 (from 0x13) or 5. Mission 0x13: unless
/// the driver's active task is 0x451, with the car and the target (vx, vy) at rest and +0x428 bit 0 →
/// occupants leave, cruise 0, mission 0. Other missions: the player's vehicle at rest (< 0.05) counts
/// +0x4DC up by `timestep * 16.67` (else 0); a player vehicle that is upside down or at rest for more than
/// 2500, +0x428 bit 0 and within 10 → the same.
#[hook(addr = 0x428990, conv = cdecl, name = "CCarCtrl::SteerAICarWithPhysicsTryingToBlockTarget_Stop", expect = "8B 44 24 18 8B 4C 24 1C")]
#[allow(clippy::too_many_arguments)]
pub fn steer_ai_car_with_physics_trying_to_block_target_stop(
    veh: GPtr<u8>,
    tx: f32,
    ty: f32,
    tvx: f32,
    tvy: f32,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
    hb: GPtr<u8>,
) {
    steer.write(0.0);
    gas.write(0.0);
    brake.write(1.0);
    hb.write(1);
    let p = entity_pos(veh);
    let dy = x_(vx(p, 1)).fsub_m(ty);
    let dx = x_(vx(p, 0)).fsub_m(tx);
    let d2x = dy.fmul(dy).fadd(dx.fmul(dx));
    let d2 = d2x.to_f32();
    if !le41(d2x, x_(100.0)) {
        let m = rd::<u8>(veh, 0x3BA);
        wr::<u8>(veh, 0x3BA, if m == 0x13 { 0x12 } else { 5 });
        return;
    }
    let leave = |veh: GPtr<u8>| {
        tell_occupants_to_leave_car(veh);
        wr::<u8>(veh, 0x3D0, 0);
        wr::<u8>(veh, 0x3BA, 0);
    };
    if rd::<u8>(veh, 0x3BA) == 0x13 {
        let ped = rd::<GPtr<u8>>(veh, 0x460);
        if !ped.is_null() {
            let t = task_manager_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
            if !t.is_null() && task_get_type(t) == 0x451 {
                return;
            }
        }
        let (a, b) = (rd::<f32>(veh, 0x44), rd::<f32>(veh, 0x48));
        if ge5(x_(a).fmul_m(a).fadd(x_(b).fmul_m(b)), x_(k(0x38D1_B717))) {
            return;
        }
        if ge5(x_(tvx).fmul_m(tvx).fadd(x_(tvy).fmul_m(tvy)), x_(k(0x39D1_B717))) {
            return;
        }
        if rd::<u8>(veh, 0x428) & 1 == 0 {
            return;
        }
        leave(veh);
        return;
    }
    let pv = find_player_vehicle(-1, 0);
    if !pv.is_null() && !ge5(speed_3d(find_player_vehicle(-1, 0)), x_(k(0x3D4C_CCCD))) {
        let c = i32::from(rd::<u16>(veh, 0x4DC));
        let v = x_(time_step()).fmul_m(k(0x4185_5555)).fadd(xi_(c));
        wr::<u16>(veh, 0x4DC, ftol32(v) as u16);
    } else {
        wr::<u16>(veh, 0x4DC, 0);
    }
    if !find_player_vehicle(-1, 0).is_null()
        && vehicle_is_upside_down(find_player_vehicle(-1, 0)) == 0
        && (ge5(speed_3d(find_player_vehicle(-1, 0)), x_(k(0x3D4C_CCCD))) || rd::<u16>(veh, 0x4DC) <= 0x9C4)
    {
        return;
    }
    if rd::<u8>(veh, 0x428) & 1 == 0 || ge5(x_(d2), x_(100.0)) {
        return;
    }
    leave(veh);
}

// ---------------------------------------------------------------------------------------------------------
// SteerAICarWithPhysicsFollowPath

orig! {
    /// `CCarCtrl::StopCarIfNodesAreInvalid(CVehicle*)` (0x422590)
    fn stop_car_if_nodes_are_invalid(v: GPtr<u8>) -> u8 = cdecl 0x422590 => crate::carctrl2::stop_car_if_nodes_are_invalid;
    /// `CCarCtrl::PickNextNodeAccordingStrategy(CVehicle*)` (0x432B10)
    fn pick_next_node_according_strategy(v: GPtr<u8>) -> u8 = cdecl 0x432B10 => crate::carctrl4::pick_next_node_according_strategy;
    /// `CWorld::FindPlayerCoors(CVector* out, int32)` (0x56E010)
    fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => unported;
    /// `CCarCtrl::FindMaximumSpeedForThisCarInTraffic(CVehicle*)` (0x434400, ST0)
    fn find_maximum_speed_call(v: GPtr<u8>) -> f64 = cdecl 0x434400 => find_maximum_speed_abi;
    /// `CTrafficLights::ShouldCarStopForLight(CVehicle*, bool)` (0x49D610)
    fn should_car_stop_for_light(v: GPtr<u8>, ignore: u8) -> u8 = cdecl 0x49D610 => crate::misc2::should_car_stop_for_light;
    /// `CTrafficLights::ShouldCarStopForBridge(CVehicle*)` (0x49D420)
    fn should_car_stop_for_bridge(v: GPtr<u8>) -> u8 = cdecl 0x49D420 => crate::misc2::should_car_stop_for_bridge;
    /// `CCarAI::CarHasReasonToStop(CVehicle*)` (0x41C050)
    fn car_has_reason_to_stop(v: GPtr<u8>) = cdecl 0x41C050 => crate::carctrl::car_has_reason_to_stop;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, hidden result pointer first)
    fn vector_sub(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x40FE60 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<f32>) -> f64 = thiscall 0x4082C0 => unported;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn find_maximum_speed_abi(v: GPtr<u8>) -> f64 {
    find_maximum_speed_for_this_car_in_traffic(v).to_f64()
}

/// The two links' scaled directions as `SteerAICarWithPhysicsFollowPath` computes them: `dir * 0.01 * sign`
/// (sign = +0x3B5 for the current link, +0x3B6 for the next one), stored f32:
/// `(cur.x, cur.y, next.x, next.y)`.
fn link_dirs(veh: GPtr<u8>, cur: u16, next: u16) -> (f32, f32, f32, f32) {
    let c = k(0x3C23_D70A);
    let a = xi_(i32::from(rd::<i8>(veh, 0x3B5)));
    let b = xi_(i32::from(rd::<i8>(veh, 0x3B6)));
    let l1 = navi_node(cur);
    let l2 = navi_node(next);
    let d = |l: GPtr<u8>, o: isize, s: X87| xi_(i32::from(rd::<i8>(l, o))).fmul_m(c).fmul(s).to_f32();
    (d(l1, 8, a), d(l1, 9, a), d(l2, 8, b), d(l2, 9, b))
}

/// `(OneWayLaneOffset(link) + lane) * 5.4` of the current link (+0x3A4) and lane byte +0x3B7.
fn lane_offset(veh: GPtr<u8>, link: u16, lane_off: isize) -> X87 {
    X87::from_f64_exact(one_way_lane_offset(navi_node(link)))
        .fadd(xi_(i32::from(rd::<i8>(veh, lane_off))))
        .fmul_m(k(0x40AC_CCCD))
}

/// A navi node's compressed coordinate × 0.125 (`fild word; fmul 0.125`).
fn navi_coord(link: u16, o: isize) -> X87 {
    xi_(i32::from(rd::<i16>(navi_node(link), o))).fmul_m(0.125)
}

/// `CCarCtrl::SteerAICarWithPhysicsFollowPath(CVehicle* veh, float* steer, float* gas, float* brake, bool*
/// handbrake)` (0x434900): follows the lane of the current navi link (+0x3A4) toward the next (+0x3A6).
/// Invalid nodes (`StopCarIfNodesAreInvalid`) → nothing; an unloaded link area → stopped (brake 1).
/// Unless the car is well on its way along the link (distance to the lane point at least 5, then a
/// forward dot product, distance below 8 and a cosine above 0.7 decide, as the bytes order them, and the
/// links differ), `PickNextNodeAccordingStrategy` runs; when it reports a change, the missions 2, 4, 8,
/// 0xC, 0xF, 0x11, 0x2B and 0x41..0x44 switch to their "close" mission and steer with `HeadingForTarget`
/// (player, +0x3EC/+0x3F0 or the target entity). Otherwise the lane point (looked ahead by 0.35 × the
/// distance unless farther than 40) gives the turn (weaved for temp actions 2, 5, 6; clamped to
/// `FindMaxSteerAngle`); the speed factor is the minimum of the bend multipliers and the traffic factor
/// (`FindMaximumSpeedForThisCarInTraffic / cruise` for temp actions 0, 1, 4, 6; 0 when stopping for a
/// light or bridge), and gas / brake follow the difference to the forward speed. Missions 8 / 0xC
/// without a temp action: within 8 of +0x3EC with `|pos · fwd| / dist < 0.2` → temp action 3 for 2 s.
/// The intermediate values keep the bytes' f32 stores and x87-held values.
#[hook(addr = 0x434900, conv = cdecl, name = "CCarCtrl::SteerAICarWithPhysicsFollowPath", expect = "83 EC 60 55 56")]
pub fn steer_ai_car_with_physics_follow_path(
    veh: GPtr<u8>,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
    hb: GPtr<u8>,
) {
    if stop_car_if_nodes_are_invalid(veh) != 0 {
        return;
    }
    let stop = || {
        brake.write(1.0);
        gas.write(0.0);
        steer.write(0.0);
        hb.write(0);
    };
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let (fxn, fyn) = forward_2d_normalised(m);
    let c0 = rd::<u16>(veh, 0x3A4);
    if !area_loaded(u32::from(c0) >> 10) {
        stop();
        return;
    }
    let n0 = rd::<u16>(veh, 0x3A6);
    if !area_loaded(u32::from(n0) >> 10) {
        stop();
        return;
    }
    // [esp+0x18]/[esp+0x14]: the current link's scaled direction x / y ([E-0x54] / [E-0x58]),
    // [E-0x44] / [E-0x48] the next link's
    let (mut d1x, mut d1y, mut d2x, mut d2y) = link_dirs(veh, c0, n0);
    let mut off1 = lane_offset(veh, c0, 0x3B7).to_f32(); // [E-0x60]
    let mut off2 = lane_offset(veh, n0, 0x3B8);
    if rd::<i32>(veh, 0x594) == 10 {
        off1 = x_(off1).fadd_m(k(0x3FBA_9FBF)).to_f32();
        off2 = off2.fadd_m(k(0x3FBA_9FBF));
    }
    let mut cur = rd::<u16>(veh, 0x3A4); // CX
    let next = rd::<u16>(veh, 0x3A6); // DI
    let p1x = navi_coord(cur, 0).fadd(x_(off1).fmul_m(d1y)).to_f32();
    let p1y = navi_coord(cur, 2).fsub(x_(off1).fmul_m(d1x)).to_f32();
    let p2x = navi_coord(next, 0).fadd(off2.fmul_m(d2y)).to_f32();
    let p2y = navi_coord(next, 2).fsub(off2.fmul_m(d2x)).to_f32();
    let p = entity_pos(veh);
    let rx = x_(vx(p, 0)).fsub_m(p1x);
    let ry = x_(vx(p, 1)).fsub_m(p1y);
    let mut dist = ry.fmul(ry).fadd(rx.fmul(rx)).fsqrt().to_f32(); // [E-0x5c]
    let ex = x_(p2x).fsub_m(p1x);
    let ey = x_(p2y).fsub_m(p1y);
    let len = ey.fmul(ey).fadd(ex.fmul(ex)).fsqrt().to_f32();
    let dot = ey.fmul(ry).fadd(ex.fmul(rx)).to_f32();
    // `jnp` on `< 5` / `< 8` skips to the strategy; `test ah, 0x41; je` on the cosine `> 0.7` too
    let mut to_c = false;
    if ge5(x_(dist), x_(5.0)) {
        let check = if !le41(x_(dot), x_(0.0)) {
            ge5(x_(dist), x_(8.0))
        } else {
            true
        };
        if check {
            let cosv = x_(len).fmul_m(dist).fdivr_m(dot);
            if le41(cosv, x_(k(0x3F33_3333))) && next != cur {
                to_c = true;
            }
        }
    }
    if !to_c {
        if pick_next_node_according_strategy(veh) != 0 {
            let mission = rd::<i8>(veh, 0x3BA);
            let target: Option<(u8, u8)> = match mission {
                2 => Some((3, 0)),
                4 => Some((5, 0)),
                0x2B => Some((0x2C, 0)),
                8 => Some((9, 1)),
                0xC => Some((0xD, 1)),
                0xF => Some((0x10, 2)),
                0x11 => Some((0x12, 2)),
                0x41..=0x44 => Some(((mission as u8).wrapping_sub(0x24), 2)),
                _ => None,
            };
            if let Some((nm, src)) = target {
                wr::<u8>(veh, 0x3BA, nm);
                let (tx, ty) = match src {
                    0 => {
                        let mut v = [0f32; 3];
                        let c = find_player_coors(GPtr::from_raw(v.as_mut_ptr()), -1);
                        (vx(c, 0), vx(c, 1))
                    }
                    1 => (rd::<f32>(veh, 0x3EC), rd::<f32>(veh, 0x3F0)),
                    _ => {
                        let t = entity_pos(rd::<GPtr<u8>>(veh, 0x41C));
                        (vx(t, 0), vx(t, 1))
                    }
                };
                steer_heading_for_target(veh, GPtr::NULL, tx, ty, steer, gas, brake, hb);
                return;
            }
        }
        let c1 = rd::<u16>(veh, 0x3A4);
        if !area_loaded(u32::from(c1) >> 10) || !area_loaded(u32::from(rd::<u16>(veh, 0x3A6)) >> 10) {
            stop();
            return;
        }
        off1 = lane_offset(veh, c1, 0x3B7).to_f32();
        cur = rd::<u16>(veh, 0x3A4);
        let p = entity_pos(veh);
        let rx = navi_coord(cur, 0).fadd(x_(off1).fmul_m(d1y)).fsub_m(vx(p, 0));
        let ry = navi_coord(cur, 2).fsub(x_(off1).fmul_m(d1x)).fsub_m(vx(p, 1));
        dist = ry.fmul(ry).fadd(rx.fmul(rx)).fsqrt().to_f32();
        let n1 = rd::<u16>(veh, 0x3A6);
        (d1x, d1y, d2x, d2y) = link_dirs(veh, cur, n1);
    }
    // 0x435055: the lane point of the current link, looked ahead by 0.35 * dist unless farther than 40
    let o_d1y = x_(off1).fmul_m(d1y);
    let lx = navi_coord(cur, 0).fadd(o_d1y).to_f32(); // [E-0x24]
    let o_d1x = x_(off1).fmul_m(d1x).to_f32(); // [E-0x40]
    let ly = navi_coord(cur, 2).fsub_m(o_d1x).to_f32(); // [E-0x20]
    let mut tx = x_(dist).fmul_m(d1x).fmul_m(k(0x3EB3_3333)).fsubr_m(lx).to_f32();
    let mut ty = x_(dist).fmul_m(d1y).fmul_m(k(0x3EB3_3333)).fsubr_m(ly).to_f32();
    if !le41(x_(dist), x_(40.0)) {
        tx = navi_coord(cur, 0).fadd(o_d1y).to_f32();
        ty = navi_coord(cur, 2).fsub_m(o_d1x).to_f32();
    }
    let p = entity_pos(veh);
    let dyt = x_(ty).fsub_m(vx(p, 1)).to_f32();
    let dxt = x_(tx).fsub_m(vx(p, 0)).to_f32();
    let ang = atan_xy(dxt, dyt).to_f32();
    let heading = atan_xy(fxn, fyn).to_f32();
    let ta = rd::<u8>(veh, 0x3B9);
    let a = if ta == 2 || ta == 5 || ta == 6 {
        X87::from_f64_exact(find_angle_to_weave_call(veh, GPtr::NULL, ang, heading, 1.0))
    } else {
        x_(ang)
    };
    let mut turn = wrap_pi(a.fsub_m(heading)).to_f32(); // [E-0x60]
    let ms = X87::from_f64_exact(find_max_steer_angle(veh));
    let nms = ms.fchs();
    if !ge5(x_(turn), nms) {
        turn = nms.to_f32();
    }
    if !le41(x_(turn), ms) {
        turn = ms.to_f32();
    }
    let mm = rd::<GPtr<u8>>(veh, 0x14);
    let fspeed = x_(rd::<f32>(veh, 0x4C))
        .fmul_m(rd::<f32>(mm, 0x18))
        .fadd(x_(rd::<f32>(veh, 0x48)).fmul_m(rd::<f32>(mm, 0x14)))
        .fadd(x_(rd::<f32>(veh, 0x44)).fmul_m(rd::<f32>(mm, 0x10)))
        .fmul_m(60.0)
        .to_f32(); // [E-0x3c]
    let ta = i32::from(rd::<i8>(veh, 0x3B9));
    let mut traffic = if matches!(ta, 0 | 1 | 4 | 6) {
        let s = X87::from_f64_exact(find_maximum_speed_call(veh));
        s.fdiv(xi_(i32::from(rd::<u8>(veh, 0x3D0)))).to_f32()
    } else {
        1.0
    }; // [E-0x4c]
    let ta = i32::from(rd::<i8>(veh, 0x3B9));
    if matches!(ta, 0 | 1 | 5 | 6) && should_car_stop_for_light(veh, 0) != 0 {
        car_has_reason_to_stop(veh);
        traffic = 0.0;
    }
    if should_car_stop_for_bridge(veh) != 0 {
        car_has_reason_to_stop(veh);
        traffic = 0.0;
    }
    // the bend factors
    let cur2 = rd::<u16>(veh, 0x3A4);
    let o = lane_offset(veh, cur2, 0x3B7);
    let p = entity_pos(veh);
    let ry = navi_coord(cur2, 2).fsub(o.fmul_m(d1x)).fsub_m(vx(p, 1)).to_f32();
    let rx = navi_coord(cur2, 0).fadd(o.fmul_m(d1y)).fsub_m(vx(p, 0)).to_f32();
    let rel = atan_xy(rx, ry).fsub_m(heading).to_f32();
    let mult1 = X87::from_f64_exact(find_speed_multiplier(
        rel,
        k(0x3ECC_CCCD),
        k(0x3F99_999A),
        k(0x3ECC_CCCD),
    ))
    .to_f32();
    let a1 = atan_xy(d1x, d1y).to_f32();
    let bend = atan_xy(d2x, d2y).fsubr_m(a1).to_f32();
    let mult2 = X87::from_f64_exact(find_speed_multiplier(
        bend,
        k(0x3DCC_CCCD),
        k(0x3F99_999A),
        k(0x3ECC_CCCD),
    ))
    .to_f32();
    let m2 = if !le41(x_(dist), x_(40.0)) || rd::<u8>(veh, 0x3D0) < 12 {
        x_(1.0)
    } else {
        let q = x_(dist).fmul_m(k(0x3CCC_CCCD)).fsubr_m(1.0);
        let r = x_(1.0).fsub_m(mult2);
        q.fmul(r).fsubr_m(1.0)
    };
    let mmin = if ge5(x_(mult1), m2) { m2 } else { x_(mult1) };
    let factor = if ge5(mmin, x_(traffic)) {
        traffic
    } else if !x_(mult1).fcom(m2).test_parity(0x05) {
        mult1
    } else {
        m2.to_f32()
    };
    brake.write(0.0);
    let s = xi_(i32::from(rd::<u8>(veh, 0x3D0))).fmul_m(factor);
    let diff = s.fsub_m(fspeed).to_f32();
    if !ge5(s, x_(k(0x3D4C_CCCD))) && !ge5(x_(diff), x_(k(0x3CF5_C28F))) {
        brake.write(1.0);
        gas.write(0.0);
    } else if le41(x_(diff), x_(0.0)) {
        let b = x_(diff).fmul_m(k(0xBDAA_AAAB));
        gas.write(0.0);
        brake.write(if ge5(x_(0.5), b) { b.to_f32() } else { 0.5 });
    } else {
        let g = x_(diff).fmul_m(if ge5(x_(fspeed), x_(2.0)) { 0.125 } else { 0.25 });
        gas.write(if ge5(x_(1.0), g) { g.to_f32() } else { 1.0 });
        if rd::<i32>(veh, 0x594) == 10
            && !le41(x_(diff), x_(3.0))
            && !x_(rd::<f32>(veh, 0x818)).fcom(x_(0.0)).test_parity(0x41)
        {
            wr::<f32>(veh, 0x818, 10.0);
        }
    }
    steer.write(turn);
    hb.write(0);
    let mission = rd::<u8>(veh, 0x3BA);
    if (mission != 8 && mission != 0xC) || rd::<u8>(veh, 0x3BB) != 0 {
        return;
    }
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let mut d = [0f32; 3]; // [E-0xc]
    let dv = vector_sub(
        GPtr::from_raw(d.as_mut_ptr()),
        entity_pos(veh),
        veh.byte_add(0x3EC).cast(),
    );
    let dmx = X87::from_f64_exact(vector_magnitude(dv));
    let dm = dmx.to_f32();
    if ge5(dmx, x_(8.0)) {
        return;
    }
    let dp = abs5(X87::from_f64_exact(dot_product(
        entity_pos(veh),
        m.byte_add(0x10).cast(),
    )));
    if ge5(dp.fdiv_m(dm), x_(k(0x3E4C_CCCD))) {
        return;
    }
    wr::<u8>(veh, 0x3BB, 3);
    wr::<u32>(veh, 0x3BC, time_ms().wrapping_add(2000));
}

// ---------------------------------------------------------------------------------------------------------
// Helis and planes: the small steering entries

orig! {
    /// `CCarCtrl::FlyAIPlaneInCertainDirection(CPlane*)` (0x423000)
    fn fly_ai_plane_in_certain_direction(v: GPtr<u8>) = cdecl 0x423000 => unported;
    /// `CCarCtrl::FlyAIHeliInCertainDirection(CHeli*, float angle, float, bool)` (0x429A70)
    fn fly_ai_heli_in_certain_direction(v: GPtr<u8>, angle: f32, dist: f32, b: u8) = cdecl 0x429A70 => unported;
}

/// `CCarCtrl::GetAIHeliToFlyInDirection(CAutomobile*)` (stub 0x42A730 → 0x156DD90):
/// `FlyAIHeliInCertainDirection(veh, +0x9B4, 1000, false)`.
#[hook(addr = 0x42A730, conv = cdecl, name = "CCarCtrl::GetAIHeliToFlyInDirection", expect = "E9 5B 36 14 01")]
pub fn get_ai_heli_to_fly_in_direction(veh: GPtr<u8>) {
    fly_ai_heli_in_certain_direction(veh, rd::<f32>(veh, 0x9B4), 1000.0, 0);
}

/// `CCarCtrl::SteerAIPlaneTowardsTargetCoors(CAutomobile*)` (0x423790): +0x9B4 = the angle to (+0x3EC,
/// +0x3F0), then `FlyAIPlaneInCertainDirection`.
#[hook(addr = 0x423790, conv = cdecl, name = "CCarCtrl::SteerAIPlaneTowardsTargetCoors", expect = "56 8B 74 24 08")]
pub fn steer_ai_plane_towards_target_coors(veh: GPtr<u8>) {
    let p = entity_pos(veh);
    let dy = x_(rd::<f32>(veh, 0x3F0)).fsub_m(vx(p, 1)).to_f32();
    let dx = x_(rd::<f32>(veh, 0x3EC)).fsub_m(vx(p, 0)).to_f32();
    wr::<f32>(veh, 0x9B4, atan_xy(dx, dy).to_f32());
    fly_ai_plane_in_certain_direction(veh);
}

/// `CCarCtrl::SteerAIPlaneToFollowEntity(CAutomobile*)` (0x4237F0): +0x9B4 = the angle to the target entity
/// (+0x41C), +0x9A8 = its z, then `FlyAIPlaneInCertainDirection`.
#[hook(addr = 0x4237F0, conv = cdecl, name = "CCarCtrl::SteerAIPlaneToFollowEntity", expect = "53 56 8B 74 24 0C")]
pub fn steer_ai_plane_to_follow_entity(veh: GPtr<u8>) {
    let t = rd::<GPtr<u8>>(veh, 0x41C);
    let pt = entity_pos(t);
    let p = entity_pos(veh);
    let dy = x_(vx(pt, 1)).fsub_m(vx(p, 1)).to_f32();
    let dx = x_(vx(pt, 0)).fsub_m(vx(p, 0)).to_f32();
    wr::<f32>(veh, 0x9B4, atan_xy(dx, dy).to_f32());
    let t = rd::<GPtr<u8>>(veh, 0x41C);
    wr::<u32>(veh, 0x9A8, entity_pos(t).cast::<u32>().wrapping_index(2).read());
    fly_ai_plane_in_certain_direction(veh);
}

/// `CCarCtrl::GetAIPlaneToDoDogFightAgainstPlayer(CAutomobile*)` (0x42F370): target (+0x41C, not
/// referenced) = the player's vehicle, or the player ped without one; then `GetAIPlaneToDoDogFight` (a
/// tail jump; the bytes store the unchanged argument back first).
#[hook(addr = 0x42F370, conv = cdecl, name = "CCarCtrl::GetAIPlaneToDoDogFightAgainstPlayer", expect = "6A 00 6A FF E8 57 ED 13 00")]
pub fn get_ai_plane_to_do_dog_fight_against_player(veh: GPtr<u8>) {
    let t = if !find_player_vehicle(-1, 0).is_null() {
        find_player_vehicle(-1, 0)
    } else {
        crate::common::find_player_ped(-1)
    };
    wr::<GPtr<u8>>(veh, 0x41C, t);
    get_ai_plane_to_do_dog_fight_call(veh);
}

/// `CCarCtrl::SteerAIPlaneToCrashAndBurn(CAutomobile*)` (stub 0x423880 → 0x156A3A0): with `bit = +0x20 & 1`:
/// +0x98C = -0.3, +0x988 = ±1, +0x994 = 0, +0x990 = ±1 (+ when the bit is set).
#[hook(addr = 0x423880, conv = cdecl, name = "CCarCtrl::SteerAIPlaneToCrashAndBurn", expect = "E9 1B 6B 14 01")]
pub fn steer_ai_plane_to_crash_and_burn(veh: GPtr<u8>) {
    let bit = rd::<u16>(veh, 0x20) & 1 != 0;
    wr::<u32>(veh, 0x98C, 0xBE99_999A);
    wr::<f32>(veh, 0x988, if bit { 1.0 } else { -1.0 });
    wr::<u32>(veh, 0x994, 0);
    wr::<f32>(veh, 0x990, if bit { 1.0 } else { -1.0 });
}

/// `CCarCtrl::SteerAIHeliToCrashAndBurn(CAutomobile*)` (0x4238E0): with `bit = +0x20 & 1`: +0x990 = -0.3,
/// +0x98C = ±(+0xA14) (negated through the x87 when the bit is clear), +0x998 = -0.5, +0x994 = ±1.
#[hook(addr = 0x4238E0, conv = cdecl, name = "CCarCtrl::SteerAIHeliToCrashAndBurn", expect = "8B 44 24 04 66 8B 48 20")]
pub fn steer_ai_heli_to_crash_and_burn(veh: GPtr<u8>) {
    let bit = rd::<u16>(veh, 0x20) & 1 != 0;
    wr::<u32>(veh, 0x990, 0xBE99_999A);
    if bit {
        wr::<u32>(veh, 0x98C, rd::<u32>(veh, 0xA14));
    } else {
        wr::<f32>(veh, 0x98C, x_(rd::<f32>(veh, 0xA14)).fchs().to_f32());
    }
    wr::<u32>(veh, 0x998, 0xBF00_0000);
    wr::<f32>(veh, 0x994, if bit { 1.0 } else { -1.0 });
}

/// `CCarCtrl::SteerAIHeliFlyingAwayFromPlayer(CAutomobile*)` (0x42ACB0): the angle from the player to the
/// heli (`GetATanOfXY` of player - heli, + π), `FlyAIHeliInCertainDirection(veh, angle, 1000, false)`.
#[hook(addr = 0x42ACB0, conv = cdecl, name = "CCarCtrl::SteerAIHeliFlyingAwayFromPlayer", expect = "83 EC 18 53 8B 5C 24 20")]
pub fn steer_ai_heli_flying_away_from_player(veh: GPtr<u8>) {
    let p = entity_pos(veh);
    let mut a = [0f32; 3];
    let mut b = [0f32; 3];
    let pa = find_player_coors(GPtr::from_raw(a.as_mut_ptr()), -1);
    let pb = find_player_coors(GPtr::from_raw(b.as_mut_ptr()), -1);
    let dy = x_(vx(pb, 1)).fsub_m(vx(p, 1)).to_f32();
    let dx = x_(vx(pa, 0)).fsub_m(vx(p, 0)).to_f32();
    let ang = atan_xy(dx, dy).fadd_m(k(0x4049_0FDB)).to_f32();
    fly_ai_heli_in_certain_direction(veh, ang, 1000.0, 0);
}

// ---------------------------------------------------------------------------------------------------------
// Boats

/// `CCarCtrl::SteerAIBoatWithPhysicsHeadingForTarget(CVehicle* veh, float x, float y, float* steer, float*
/// gas, float* brake)` (0x428BE0): turn = wrap(angle to (x, y) - heading of the normalised 2D forward),
/// clamped to ±0.5; `diff = cruise - |v.xy| * 60`. `diff > 0` (or unordered): gas 1, or `1 - (0.25 -
/// diff / cruise) * 4` when `diff / cruise <= 0.25`; brake 0, steer = turn. Otherwise gas -0.1 (-0.2
/// below -5), brake 0, steer = -turn.
#[hook(addr = 0x428BE0, conv = cdecl, name = "CCarCtrl::SteerAIBoatWithPhysicsHeadingForTarget", expect = "83 EC 10 56 8B 74 24 18")]
pub fn steer_ai_boat_with_physics_heading_for_target(
    veh: GPtr<u8>,
    tx: f32,
    ty: f32,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
) {
    let (fxn, fyn) = forward_2d_normalised(rd::<GPtr<u8>>(veh, 0x14));
    let p = entity_pos(veh);
    let dy = x_(ty).fsub_m(vx(p, 1)).to_f32();
    let dx = x_(tx).fsub_m(vx(p, 0)).to_f32();
    let a = atan_xy(dx, dy).to_f32();
    let t = wrap_pi(atan_xy(fxn, fyn).fsubr_m(a));
    let turn = if !ge5(t, x_(-0.5)) {
        x_(-0.5)
    } else if le41(t, x_(0.5)) {
        t
    } else {
        x_(0.5)
    };
    let (mx, my) = (rd::<f32>(veh, 0x44), rd::<f32>(veh, 0x48));
    let cruise = xi_(i32::from(rd::<u8>(veh, 0x3D0))).to_f32();
    let diff = x_(my)
        .fmul_m(my)
        .fadd(x_(mx).fmul_m(mx))
        .fsqrt()
        .fmul_m(60.0)
        .fsubr_m(cruise);
    if diff.fcom(x_(0.0)).test_parity(0x41) {
        let q = diff.fdiv_m(cruise);
        if !le41(q, x_(0.25)) {
            gas.write(1.0);
        } else {
            gas.write(x_(0.25).fsub(q).fmul_m(4.0).fsubr_m(1.0).to_f32());
        }
        brake.write(0.0);
        steer.write(turn.to_f32());
        return;
    }
    gas.write(k(0xBDCC_CCCD));
    if !ge5(diff, x_(-5.0)) {
        gas.write(k(0xBE4C_CCCD));
    }
    brake.write(0.0);
    steer.write(turn.fmul_m(-1.0).to_f32());
}

orig! {
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<f32>) = thiscall 0x59C910 => unported;
}

/// The boats' gas from the cruise speed: `diff = cruise - |v.xy| * 60`; `diff > 0` (or unordered): 1, or
/// `1 - (0.25 - diff / cruise) * 4` when `diff / cruise <= 0.25`; otherwise -0.1 (-0.2 below -5).
fn boat_gas(veh: GPtr<u8>, gas: GPtr<f32>) -> bool {
    let (mx, my) = (rd::<f32>(veh, 0x44), rd::<f32>(veh, 0x48));
    let cruise = xi_(i32::from(rd::<u8>(veh, 0x3D0))).to_f32();
    let diff = x_(my)
        .fmul_m(my)
        .fadd(x_(mx).fmul_m(mx))
        .fsqrt()
        .fmul_m(60.0)
        .fsubr_m(cruise);
    if diff.fcom(x_(0.0)).test_parity(0x41) {
        let q = diff.fdiv_m(cruise);
        if !le41(q, x_(0.25)) {
            gas.write(1.0);
        } else {
            gas.write(x_(0.25).fsub(q).fmul_m(4.0).fsubr_m(1.0).to_f32());
        }
        return true;
    }
    gas.write(k(0xBDCC_CCCD));
    if !ge5(diff, x_(-5.0)) {
        gas.write(k(0xBE4C_CCCD));
    }
    false
}

/// `CCarCtrl::SteerAIBoatWithPhysicsCirclingPlayer(CVehicle* veh, float* steer, float* gas, float* brake,
/// bool* handbrake)` (0x429090): the normalised direction to the player turned by 90° (`(d.y, -d.x)`) times
/// -12 (boats with +0x20 bit 0) or 26 is added to the player's position; the turn toward that point
/// (wrapped, not clamped) is the steer; gas as [`boat_gas`] (no reversed steer); brake 0, no handbrake.
#[hook(addr = 0x429090, conv = cdecl, name = "CCarCtrl::SteerAIBoatWithPhysicsCirclingPlayer", expect = "83 EC 20 56 8B 74 24 28")]
pub fn steer_ai_boat_with_physics_circling_player(
    veh: GPtr<u8>,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
    hb: GPtr<u8>,
) {
    let pv = entity_pos(veh);
    let mut out = [0f32; 3];
    let pp = find_player_coors(GPtr::from_raw(out.as_mut_ptr()), -1);
    let dy = x_(vx(pp, 1)).fsub_m(vx(pv, 1));
    let dx = x_(vx(pp, 0)).fsub_m(vx(pv, 0));
    let mut v = [dx.to_f32(), dy.to_f32(), 0.0];
    let vp = GPtr::from_raw(v.as_mut_ptr());
    vector_normalise(vp);
    let (nx, ny, nz) = (
        vp.read(),
        vp.wrapping_index(1).read(),
        vp.wrapping_index(2).read(),
    );
    let f = if rd::<u8>(veh, 0x20) & 1 != 0 {
        k(0xC140_0000)
    } else {
        26.0
    };
    let ox = x_(ny).fmul_m(f).to_f32();
    let oy = x_(nx).fchs().fmul_m(f).to_f32();
    let _oz = x_(nz).fmul_m(f).to_f32();
    let mut out2 = [0f32; 3];
    let pp = find_player_coors(GPtr::from_raw(out2.as_mut_ptr()), -1);
    let tx = x_(ox).fadd_m(vx(pp, 0));
    let ty = x_(oy).fadd_m(vx(pp, 1));
    let (fxn, fyn) = forward_2d_normalised(rd::<GPtr<u8>>(veh, 0x14));
    let p = entity_pos(veh);
    let ddy = ty.fsub_m(vx(p, 1)).to_f32();
    let ddx = tx.fsub_m(vx(p, 0)).to_f32();
    let a = atan_xy(ddx, ddy).to_f32();
    let turn = wrap_pi(atan_xy(fxn, fyn).fsubr_m(a));
    boat_gas(veh, gas);
    brake.write(0.0);
    steer.write(turn.to_f32());
    hb.write(0);
}

orig! {
    /// `CWorld::FindPlayerSpeed(int32)` (0x56E090, returns `CVector*`)
    fn find_player_speed(id: i32) -> GPtr<f32> = cdecl 0x56E090 => unported;
    /// `CVehicle::FireFixedMachineGuns()` (0x73DF00)
    fn fire_fixed_machine_guns(v: GPtr<u8>) = thiscall 0x73DF00 => unported;
}

/// `CCarCtrl::SteerAIBoatWithPhysicsAttackingPlayer(CVehicle* veh, float* steer, float* gas, float* brake,
/// bool* handbrake)` (0x428DE0): aims at the player's position plus the player's speed × 60 × `min(dist
/// * 0.05, 2)` (3D distance); turn wrapped, not clamped; gas as [`boat_gas`]; brake 0, no handbrake. The
/// Predator (model 0x1AE) within 40 fires its fixed guns while the turn is below 0.15 (signed).
#[hook(addr = 0x428DE0, conv = cdecl, name = "CCarCtrl::SteerAIBoatWithPhysicsAttackingPlayer", expect = "83 EC 1C 56 8B 74 24 24")]
pub fn steer_ai_boat_with_physics_attacking_player(
    veh: GPtr<u8>,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
    hb: GPtr<u8>,
) {
    let pv = entity_pos(veh);
    let mut out = [0f32; 3];
    let pp = find_player_coors(GPtr::from_raw(out.as_mut_ptr()), -1);
    let dx = x_(vx(pp, 0)).fsub_m(vx(pv, 0));
    let dy = x_(vx(pp, 1)).fsub_m(vx(pv, 1));
    let dz = x_(vx(pp, 2)).fsub_m(vx(pv, 2));
    let distx = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    let dist = distx.to_f32();
    let t = distx.fmul_m(k(0x3D4C_CCCD));
    let mut lead = if ge5(t, x_(2.0)) { 2.0 } else { t.to_f32() };
    let (fxn, fyn) = forward_2d_normalised(rd::<GPtr<u8>>(veh, 0x14));
    let sp = find_player_speed(-1);
    let pp = find_player_coors(GPtr::from_raw(out.as_mut_ptr()), -1);
    let tx = x_(lead).fmul_m(vx(sp, 0)).fmul_m(60.0).fadd_m(vx(pp, 0)).to_f32();
    let sp = find_player_speed(-1);
    lead = x_(lead).fmul_m(vx(sp, 1)).fmul_m(60.0).to_f32();
    let pp = find_player_coors(GPtr::from_raw(out.as_mut_ptr()), -1);
    let ty = x_(lead).fadd_m(vx(pp, 1));
    let p = entity_pos(veh);
    let ddy = ty.fsub_m(vx(p, 1)).to_f32();
    let ddx = x_(tx).fsub_m(vx(p, 0)).to_f32();
    let a = atan_xy(ddx, ddy).to_f32();
    let turn = wrap_pi(atan_xy(fxn, fyn).fsubr_m(a));
    boat_gas(veh, gas);
    brake.write(0.0);
    steer.write(turn.to_f32());
    hb.write(0);
    if rd::<u16>(veh, 0x22) == 0x1AE && !ge5(x_(dist), x_(40.0)) && !ge5(turn, x_(k(0x3E19_999A))) {
        fire_fixed_machine_guns(veh);
    }
}

orig! {
    /// `CCarCtrl::FlyAIHeliToTarget_FixedOrientation(CHeli*, float orientation, CVector target, bool)` (0x423940)
    fn fly_ai_heli_to_target_fixed_orientation(v: GPtr<u8>, orient: f32, x: f32, y: f32, z: f32, b: u8) = cdecl 0x423940 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(v: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CCarCtrl::TriggerDogFightMoves(CAutomobile*, CVehicle*)` (0x429300)
    fn trigger_dog_fight_moves_call(v: GPtr<u8>, t: GPtr<u8>) = cdecl 0x429300 => trigger_dog_fight_moves;
    /// `CCarCtrl::TestWhetherToFirePlaneGuns(CVehicle*, CEntity*)` (0x429520)
    fn test_whether_to_fire_plane_guns_call(v: GPtr<u8>, t: GPtr<u8>) = cdecl 0x429520 => test_whether_to_fire_plane_guns;
    /// `CCarCtrl::PossiblyFireHSMissile(CVehicle*, CEntity*)` (0x429600)
    fn possibly_fire_hs_missile_call(v: GPtr<u8>, t: GPtr<u8>) = cdecl 0x429600 => possibly_fire_hs_missile;
}

/// `CCarCtrl::SteerAIHeliTowardsTargetCoors(CAutomobile*)` (0x42A630): a fixed orientation (+0x954 not
/// below 0) → `FlyAIHeliToTarget_FixedOrientation(veh, +0x954, +0x3EC (by value), true)`; else
/// `FlyAIHeliInCertainDirection(veh, angle to +0x3EC/+0x3F0, 2D distance, true)`.
#[hook(addr = 0x42A630, conv = cdecl, name = "CCarCtrl::SteerAIHeliTowardsTargetCoors", expect = "56 8B 74 24 08")]
pub fn steer_ai_heli_towards_target_coors(veh: GPtr<u8>) {
    let o = rd::<f32>(veh, 0x954);
    if !x_(o).fcom(x_(0.0)).test_any(0x01) {
        fly_ai_heli_to_target_fixed_orientation(
            veh,
            o,
            rd::<f32>(veh, 0x3EC),
            rd::<f32>(veh, 0x3F0),
            rd::<f32>(veh, 0x3F4),
            1,
        );
        return;
    }
    let p = entity_pos(veh);
    let dy = x_(rd::<f32>(veh, 0x3F0)).fsub_m(vx(p, 1)).to_f32();
    let dx = x_(rd::<f32>(veh, 0x3EC)).fsub_m(vx(p, 0)).to_f32();
    let a = atan_xy(dx, dy).to_f32();
    let p = entity_pos(veh);
    let ty = x_(rd::<f32>(veh, 0x3F0));
    let tx = x_(rd::<f32>(veh, 0x3EC));
    let yy = ty.fsub_m(vx(p, 1)).fmul(ty.fsub_m(vx(p, 1)));
    let xx = tx.fsub_m(vx(p, 0)).fmul(tx.fsub_m(vx(p, 0)));
    let dist = yy.fadd(xx).fsqrt().to_f32();
    fly_ai_heli_in_certain_direction(veh, a, dist, 1);
}

/// `CCarCtrl::TestWhetherToFirePlaneGuns(CVehicle* veh, CEntity* target)` (0x429520; its first block
/// continues at 0x401F1A): +0x42C bit 4 cleared; for +0x513 of 0 or 1 and a target within 150 (3D) whose
/// normalised direction has a dot product above 0.8 with the forward → bit 4 set (fire).
#[hook(addr = 0x429520, conv = cdecl, name = "CCarCtrl::TestWhetherToFirePlaneGuns", expect = "83 EC 0C 56 8B 74 24 14")]
pub fn test_whether_to_fire_plane_guns(veh: GPtr<u8>, target: GPtr<u8>) {
    let flags = rd::<u8>(veh, 0x42C);
    let kind = rd::<u8>(veh, 0x513);
    wr::<u8>(veh, 0x42C, flags & 0xEF);
    if (kind != 0 && kind != 1) || target.is_null() {
        return;
    }
    let pv = entity_pos(veh);
    let pt = entity_pos(target);
    let dz = x_(vx(pt, 2)).fsub_m(vx(pv, 2));
    let dy = x_(vx(pt, 1)).fsub_m(vx(pv, 1));
    let dx = x_(vx(pt, 0)).fsub_m(vx(pv, 0));
    let mut d = [dx.to_f32(), dy.to_f32(), dz.to_f32()];
    let len = x_(d[0]).fmul_m(d[0]).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt();
    if ge5(len, x_(150.0)) {
        return;
    }
    let dp = GPtr::from_raw(d.as_mut_ptr());
    vector_normalise(dp);
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let dot = x_(vx(dp, 2))
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(x_(vx(dp, 1)).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(x_(vx(dp, 0)).fmul_m(rd::<f32>(m, 0x10)));
    if le41(dot, x_(k(0x3F4C_CCCD))) {
        return;
    }
    wr::<u8>(veh, 0x42C, rd::<u8>(veh, 0x42C) | 0x10);
}

/// `CCarCtrl::GetAIPlaneToAttackPlayer(CAutomobile*)` (0x429780): flies toward the player's position plus
/// the player's speed × 50 (+0x9B4 = the 2D angle, +0x9A8 = the z), `FlyAIPlaneInCertainDirection`;
/// dog-fight moves against a player vehicle of appearance 5; then the gun and heat-seeker tests against
/// the player's vehicle.
#[hook(addr = 0x429780, conv = cdecl, name = "CCarCtrl::GetAIPlaneToAttackPlayer", expect = "83 EC 18 56 6A FF")]
pub fn get_ai_plane_to_attack_player(veh: GPtr<u8>) {
    let s = find_player_speed(-1);
    let ox = x_(vx(s, 0)).fmul_m(50.0).to_f32();
    let oy = x_(vx(s, 1)).fmul_m(50.0).to_f32();
    let oz = x_(vx(s, 2)).fmul_m(50.0).to_f32();
    let mut out = [0f32; 3];
    let pp = find_player_coors(GPtr::from_raw(out.as_mut_ptr()), -1);
    let tx = x_(ox).fadd_m(vx(pp, 0));
    let ty = x_(oy).fadd_m(vx(pp, 1));
    let tz = x_(oz).fadd_m(vx(pp, 2)).to_f32();
    let p = entity_pos(veh);
    let dy = ty.fsub_m(vx(p, 1)).to_f32();
    let dx = tx.fsub_m(vx(p, 0)).to_f32();
    wr::<f32>(veh, 0x9B4, atan_xy(dx, dy).to_f32());
    wr::<f32>(veh, 0x9A8, tz);
    fly_ai_plane_in_certain_direction(veh);
    if !find_player_vehicle(-1, 0).is_null() && get_vehicle_appearance(find_player_vehicle(-1, 0)) == 5 {
        trigger_dog_fight_moves_call(veh, find_player_vehicle(-1, 0));
    }
    test_whether_to_fire_plane_guns_call(veh, find_player_vehicle(-1, 0));
    possibly_fire_hs_missile_call(veh, find_player_vehicle(-1, 0));
}

/// `CCarCtrl::SteerAIHeliAsPoliceHeli(CAutomobile*)` (0x42AAD0): toward the target entity (+0x41C): angle and
/// 2D distance; flying height +0x9A8 = the target's z, at least 6 (at least 25 when farther than 50); the
/// target position is copied to +0x3EC; `FlyAIHeliInCertainDirection(veh, angle, dist, true)`; health
/// (+0x4C0) below 230 → mission 0x2A.
#[hook(addr = 0x42AAD0, conv = cdecl, name = "CCarCtrl::SteerAIHeliAsPoliceHeli", expect = "83 EC 10 53 55 8B 6C 24 1C")]
pub fn steer_ai_heli_as_police_heli(veh: GPtr<u8>) {
    let t = rd::<GPtr<u8>>(veh, 0x41C);
    let pt = entity_pos(t);
    let pv = entity_pos(veh);
    let dy = x_(vx(pt, 1)).fsub_m(vx(pv, 1)).to_f32();
    let dx = x_(vx(pt, 0)).fsub_m(vx(pv, 0)).to_f32();
    let a = atan_xy(dx, dy).to_f32();
    let t = rd::<GPtr<u8>>(veh, 0x41C);
    let pt = entity_pos(t);
    let pv = entity_pos(veh);
    let yy = x_(vx(pt, 1)).fsub_m(vx(pv, 1));
    let xx = x_(vx(pt, 0)).fsub_m(vx(pv, 0));
    let dist = yy.fmul(yy).fadd(xx.fmul(xx)).fsqrt().to_f32();
    let z = x_(vx(pt, 2));
    let h = if !le41(z, x_(6.0)) { z } else { x_(6.0) };
    wr::<f32>(veh, 0x9A8, h.to_f32());
    if !le41(x_(dist), x_(50.0)) {
        let h2 = if !le41(h, x_(25.0)) { h } else { x_(25.0) };
        wr::<f32>(veh, 0x9A8, h2.to_f32());
    }
    let pt = entity_pos(t);
    for i in 0..3 {
        wr::<u32>(
            veh,
            0x3EC + i * 4,
            pt.cast::<u32>().wrapping_index(i as i32).read(),
        );
    }
    fly_ai_heli_in_certain_direction(veh, a, dist, 1);
    if !ge5(x_(rd::<f32>(veh, 0x4C0)), x_(230.0)) {
        wr::<u8>(veh, 0x3BA, 0x2A);
    }
}

orig! {
    /// `CWorld::FindPlayerEntity(int32)` (0x56E120)
    fn find_player_entity(id: i32) -> GPtr<u8> = cdecl 0x56E120 => unported;
    /// `CCarCtrl::FireHeliRocketsAtTarget(CAutomobile*, CEntity*)` (0x42B270)
    fn fire_heli_rockets_at_target_call(v: GPtr<u8>, t: GPtr<u8>) = cdecl 0x42B270 => fire_heli_rockets_at_target;
}

/// `CCarCtrl::SteerAIHeliToLand(CAutomobile*)` (0x42AD30): flies toward (+0x3EC, +0x3F0)
/// (`FlyAIHeliInCertainDirection(veh, angle, 2D dist, true)`); within 10 and nearly at rest (|v.xy| < 0.05):
/// +0x9B0 = +0x9A8 = 0 and, unless all four +0x7D4..+0x7E0 are at least 1, the controls +0x998, +0x98C,
/// +0x990, +0x994 are zeroed.
#[hook(addr = 0x42AD30, conv = cdecl, name = "CCarCtrl::SteerAIHeliToLand", expect = "51 53 56 8B 74 24 10")]
pub fn steer_ai_heli_to_land(veh: GPtr<u8>) {
    let p = entity_pos(veh);
    let dy = x_(rd::<f32>(veh, 0x3F0)).fsub_m(vx(p, 1)).to_f32();
    let dx = x_(rd::<f32>(veh, 0x3EC)).fsub_m(vx(p, 0)).to_f32();
    let a = atan_xy(dx, dy).to_f32();
    let p = entity_pos(veh);
    let ty = x_(rd::<f32>(veh, 0x3F0)).fsub_m(vx(p, 1));
    let tx = x_(rd::<f32>(veh, 0x3EC)).fsub_m(vx(p, 0));
    let dist = ty.fmul(ty).fadd(tx.fmul(tx)).fsqrt().to_f32();
    fly_ai_heli_in_certain_direction(veh, a, dist, 1);
    if ge5(x_(dist), x_(10.0)) {
        return;
    }
    let (mx, my) = (rd::<f32>(veh, 0x44), rd::<f32>(veh, 0x48));
    if ge5(
        x_(mx).fmul_m(mx).fadd(x_(my).fmul_m(my)).fsqrt(),
        x_(k(0x3D4C_CCCD)),
    ) {
        return;
    }
    wr::<u32>(veh, 0x9B0, 0);
    wr::<u32>(veh, 0x9A8, 0);
    let all_full = [0x7D4, 0x7D8, 0x7DC, 0x7E0]
        .iter()
        .all(|&o| ge5(x_(rd::<f32>(veh, o)), x_(1.0)));
    if all_full {
        return;
    }
    wr::<u32>(veh, 0x998, 0);
    wr::<u32>(veh, 0x98C, 0);
    wr::<u32>(veh, 0x990, 0);
    wr::<u32>(veh, 0x994, 0);
}

/// `CCarCtrl::GetAIHeliToAttackPlayer(CAutomobile*)` (0x42F3C0): angle and 2D distance to the player
/// (each `FindPlayerCoors` call as the bytes make them; one y factor goes through an f32 local); +0x9A8 =
/// the player's z, +0x3EC = the player's position. Mission 0x17 closes in (within 15 → mission 0x38) at the
/// distance + 50; mission 0x38 turns away (angle + π; farther than 18 → mission 0x17).
/// `FlyAIHeliInCertainDirection(veh, angle, dist, false)`, then the gun and rocket tests against the
/// player entity.
#[hook(addr = 0x42F3C0, conv = cdecl, name = "CCarCtrl::GetAIHeliToAttackPlayer", expect = "83 EC 40 53 55 56")]
pub fn get_ai_heli_to_attack_player(veh: GPtr<u8>) {
    let mut b30 = [0f32; 3];
    let mut b24 = [0f32; 3];
    let mut b18 = [0f32; 3];
    let mut b0c = [0f32; 3];
    let (p30, p24, p18, p0c) = (
        GPtr::from_raw(b30.as_mut_ptr()),
        GPtr::from_raw(b24.as_mut_ptr()),
        GPtr::from_raw(b18.as_mut_ptr()),
        GPtr::from_raw(b0c.as_mut_ptr()),
    );
    let pos = entity_pos(veh);
    let p1 = find_player_coors(p30, -1);
    let p2 = find_player_coors(p24, -1);
    let dy = x_(vx(p2, 1)).fsub_m(vx(pos, 1)).to_f32();
    let dx = x_(vx(p1, 0)).fsub_m(vx(pos, 0)).to_f32();
    let mut ang = atan_xy(dx, dy).to_f32();
    let pos = entity_pos(veh);
    let p3 = find_player_coors(p24, -1);
    let p4 = find_player_coors(p30, -1);
    let p5 = find_player_coors(p18, -1);
    let t1 = x_(vx(p5, 1)).fsub_m(vx(pos, 1)).to_f32();
    let p6 = find_player_coors(p0c, -1);
    let yy = x_(vx(p6, 1)).fsub_m(vx(pos, 1)).fmul_m(t1);
    let xx = x_(vx(p3, 0))
        .fsub_m(vx(pos, 0))
        .fmul(x_(vx(p4, 0)).fsub_m(vx(pos, 0)));
    let mut dist = yy.fadd(xx).fsqrt().to_f32();
    let p7 = find_player_coors(p0c, -1);
    wr::<u32>(veh, 0x9A8, p7.cast::<u32>().wrapping_index(2).read());
    let p8 = find_player_coors(p0c, -1);
    for i in 0..3 {
        wr::<u32>(
            veh,
            0x3EC + i * 4,
            p8.cast::<u32>().wrapping_index(i as i32).read(),
        );
    }
    match rd::<u8>(veh, 0x3BA) {
        0x17 => {
            if !ge5(x_(dist), x_(15.0)) {
                wr::<u8>(veh, 0x3BA, 0x38);
            }
            dist = x_(dist).fadd_m(50.0).to_f32();
        }
        0x38 => {
            if !le41(x_(dist), x_(18.0)) {
                wr::<u8>(veh, 0x3BA, 0x17);
            }
            ang = x_(ang).fadd_m(k(0x4049_0FDB)).to_f32();
        }
        _ => {}
    }
    fly_ai_heli_in_certain_direction(veh, ang, dist, 0);
    test_whether_to_fire_plane_guns_call(veh, find_player_entity(-1));
    fire_heli_rockets_at_target_call(veh, find_player_entity(-1));
}

orig! {
    /// `CProjectileInfo::AddProjectile(CEntity* creator, eWeaponType, CVector pos, float force, CVector* dir,
    /// CEntity* target)` (0x737C80)
    #[allow(clippy::too_many_arguments)]
    fn add_projectile(creator: GPtr<u8>, wt: i32, x: f32, y: f32, z: f32, force: f32, dir: GPtr<u8>, target: GPtr<u8>) -> u8 = cdecl 0x737C80 => unported;
}

/// `CTimer` milliseconds / 2000 as the bytes compute it (`mul 0x10624DD3`, high dword `>> 7`).
fn ms_div_2000(t: u32) -> u32 {
    ((u64::from(t) * 0x1062_4DD3) >> 32) as u32 >> 7
}

/// `CCarCtrl::PossiblyFireHSMissile(CVehicle* veh, CEntity* target)` (0x429600): once per 2 s period
/// (`previous / 2000 != now / 2000`), a target 30..160 away (3D, both exclusive as the compares order
/// them) within the forward cone (normalised dot above 0.8) gets a heat-seeking rocket (weapon 0x14)
/// from `pos + fwd * 4 - up * 3` with force 1 along the forward.
#[hook(addr = 0x429600, conv = cdecl, name = "CCarCtrl::PossiblyFireHSMissile", expect = "8B 54 24 08 83 EC 3C")]
pub fn possibly_fire_hs_missile(veh: GPtr<u8>, target: GPtr<u8>) {
    if target.is_null() {
        return;
    }
    let pv = entity_pos(veh);
    let pt = entity_pos(target);
    let dz = x_(vx(pt, 2)).fsub_m(vx(pv, 2));
    let dy = x_(vx(pt, 1)).fsub_m(vx(pv, 1));
    let dx = x_(vx(pt, 0)).fsub_m(vx(pv, 0));
    let mut d = [dx.to_f32(), dy.to_f32(), dz.to_f32()];
    let len = x_(d[0]).fmul_m(d[0]).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt();
    if ms_div_2000(g::<u32>(0xB7CB78).read()) == ms_div_2000(time_ms()) {
        return;
    }
    if ge5(len, x_(160.0)) || le41(len, x_(30.0)) {
        return;
    }
    let dp = GPtr::from_raw(d.as_mut_ptr());
    vector_normalise(dp);
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let dot = x_(vx(dp, 2))
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(x_(vx(dp, 1)).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(x_(vx(dp, 0)).fmul_m(rd::<f32>(m, 0x10)));
    if le41(dot, x_(k(0x3F4C_CCCD))) {
        return;
    }
    let pos = entity_pos(veh);
    let mut up3 = [0f32; 3];
    let mut fwd4 = [0f32; 3];
    let mut sum = [0f32; 3];
    let mut res = [0f32; 3];
    let a = vector_mul(GPtr::from_raw(up3.as_mut_ptr()), m.byte_add(0x20).cast(), 3.0);
    let b = vector_mul(GPtr::from_raw(fwd4.as_mut_ptr()), m.byte_add(0x10).cast(), 4.0);
    let c = vector_add(GPtr::from_raw(sum.as_mut_ptr()), pos, b);
    let r = vector_sub(GPtr::from_raw(res.as_mut_ptr()), c, a);
    add_projectile(
        veh,
        0x14,
        vx(r, 0),
        vx(r, 1),
        vx(r, 2),
        1.0,
        m.byte_add(0x10),
        target,
    );
}

/// `CCarCtrl::TriggerDogFightMoves(CAutomobile* veh, CVehicle* target)` (0x429300): without a temp action,
/// a target within 70 (3D) and 15 in height: `rand() & 0xFF` 12..15 picks a move when the normalised
/// direction target → plane has a dot product with the target's forward above 0 (12, 13, 14: temp
/// actions 0x10 / 0x11 / 0x12 for 1500 + (rand & 0x3FF) / 700 + (rand & 0x1FF) ms) or
/// above 0.7 (15: temp action 0xF for 3000 + (rand & 0x7FF) ms).
#[hook(addr = 0x429300, conv = cdecl, name = "CCarCtrl::TriggerDogFightMoves", expect = "83 EC 0C 56 8B 74 24 14")]
pub fn trigger_dog_fight_moves(veh: GPtr<u8>, target: GPtr<u8>) {
    if rd::<u8>(veh, 0x3BB) != 0 {
        return;
    }
    let pv = entity_pos(veh);
    let pt = entity_pos(target);
    let dz = x_(vx(pv, 2)).fsub_m(vx(pt, 2));
    let dy = x_(vx(pv, 1)).fsub_m(vx(pt, 1));
    let dx = x_(vx(pv, 0)).fsub_m(vx(pt, 0));
    let mut d = [dx.to_f32(), dy.to_f32(), dz.to_f32()];
    let len = x_(d[0]).fmul_m(d[0]).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt();
    if ge5(len, x_(70.0)) {
        return;
    }
    let dp = GPtr::from_raw(d.as_mut_ptr());
    vector_normalise(dp);
    let tm = rd::<GPtr<u8>>(target, 0x14);
    let dot = x_(vx(dp, 2))
        .fmul_m(rd::<f32>(tm, 0x18))
        .fadd(x_(vx(dp, 1)).fmul_m(rd::<f32>(tm, 0x14)))
        .fadd(x_(vx(dp, 0)).fmul_m(rd::<f32>(tm, 0x10)))
        .to_f32();
    let h = x_(vx(entity_pos(veh), 2)).fsub_m(vx(entity_pos(target), 2));
    if ge5(abs5(h), x_(15.0)) {
        return;
    }
    let pick = (crate::common::rand_at(0x429427) & 0xFF) - 12;
    let set = |ta: u8, site: u32, mask: i32, base: u32| {
        wr::<u8>(veh, 0x3BB, ta);
        let r = (crate::common::rand_at(site) & mask) as u32;
        wr::<u32>(veh, 0x3BC, r.wrapping_add(time_ms()).wrapping_add(base));
    };
    match pick {
        0 if !le41(x_(dot), x_(0.0)) => set(0x10, 0x429460, 0x3FF, 0x5DC),
        1 if !le41(x_(dot), x_(0.0)) => set(0x11, 0x4294B5, 0x1FF, 0x2BC),
        2 if !le41(x_(dot), x_(0.0)) => set(0x12, 0x4294B5, 0x1FF, 0x2BC),
        3 if !le41(x_(dot), x_(k(0x3F33_3333))) => set(0xF, 0x4294F0, 0x7FF, 0xBB8),
        _ => {}
    }
}

/// `CCarCtrl::GetAIPlaneToDoDogFight(CAutomobile*)` (0x429890): with the target (+0x41C) extrapolated by
/// its speed × 50: mode +0x425 == 1 flies to the stored point (+0x3EC; within 50 in 2D → mode 0);
/// otherwise toward the extrapolated target, and `(rand() & 0x3FF) == 500` switches to mode 1 with a
/// point ±300 around it (`rand * (1/32767) * 600 - 300`, 50 higher). Then `FlyAIPlaneInCertainDirection`
/// and the gun and heat-seeker tests against the target.
#[hook(addr = 0x429890, conv = cdecl, name = "CCarCtrl::GetAIPlaneToDoDogFight", expect = "83 EC 18 56 8B 74 24 20")]
pub fn get_ai_plane_to_do_dog_fight(veh: GPtr<u8>) {
    let t = rd::<GPtr<u8>>(veh, 0x41C);
    let vxx = x_(rd::<f32>(t, 0x44)).fmul_m(50.0);
    let vyy = x_(rd::<f32>(t, 0x48)).fmul_m(50.0);
    let vzz = x_(rd::<f32>(t, 0x4C)).fmul_m(50.0).to_f32();
    let pt = entity_pos(t);
    let tx = vxx.fadd_m(vx(pt, 0)).to_f32();
    let ty = vyy.fadd_m(vx(pt, 1)).to_f32();
    let tz = x_(vzz).fadd_m(vx(pt, 2)).to_f32();
    let p = entity_pos(veh);
    if i32::from(rd::<i8>(veh, 0x425)) - 1 != 0 {
        let dy = x_(ty).fsub_m(vx(p, 1)).to_f32();
        let dx = x_(tx).fsub_m(vx(p, 0)).to_f32();
        wr::<f32>(veh, 0x9B4, atan_xy(dx, dy).to_f32());
        wr::<f32>(veh, 0x9A8, tz);
        if crate::common::rand_at(0x42993D) & 0x3FF == 0x1F4 {
            wr::<u8>(veh, 0x425, 1);
            let off = |site: u32, base: f32| {
                xi_(crate::common::rand_at(site))
                    .fmul_m(k(0x3800_0100))
                    .fmul_m(600.0)
                    .fadd_m(base)
                    .fsub_m(300.0)
                    .to_f32()
            };
            let nx = off(0x429959, tx);
            wr::<f32>(veh, 0x3EC, nx);
            let ny = off(0x429982, ty);
            wr::<f32>(veh, 0x3F0, ny);
            wr::<f32>(veh, 0x3F4, x_(tz).fadd_m(50.0).to_f32());
        }
    } else {
        let dy = x_(rd::<f32>(veh, 0x3F0)).fsub_m(vx(p, 1)).to_f32();
        let dx = x_(rd::<f32>(veh, 0x3EC)).fsub_m(vx(p, 0)).to_f32();
        wr::<f32>(veh, 0x9B4, atan_xy(dx, dy).to_f32());
        wr::<u32>(veh, 0x9A8, rd::<u32>(veh, 0x3F4));
        let p = entity_pos(veh);
        let ex = x_(rd::<f32>(veh, 0x3EC)).fsub_m(vx(p, 0));
        let ey = x_(rd::<f32>(veh, 0x3F0)).fsub_m(vx(p, 1));
        if !ge5(ey.fmul(ey).fadd(ex.fmul(ex)).fsqrt(), x_(50.0)) {
            wr::<u8>(veh, 0x425, 0);
        }
    }
    fly_ai_plane_in_certain_direction(veh);
    test_whether_to_fire_plane_guns_call(veh, t);
    possibly_fire_hs_missile_call(veh, t);
}

orig! {
    /// `CCollision::DistToMathematicalLine(const CVector*, const CVector*, const CVector*)` (0x412970, ST0)
    fn dist_to_mathematical_line(a: GPtr<f32>, b: GPtr<f32>, p: GPtr<f32>) -> f64 = cdecl 0x412970 => dist_to_mathematical_line_abi;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn dist_to_mathematical_line_abi(a: GPtr<f32>, b: GPtr<f32>, p: GPtr<f32>) -> f64 {
    sa_world::collision::prim::dist_to_mathematical_line(a, b, p).to_f64()
}

/// `CCarCtrl::FireHeliRocketsAtTarget(CAutomobile* veh, CEntity* target)` (0x42B270): helis of kind
/// +0x513 0 or 4, once per 250 ms period (`time / 250` changed), with the target within 80 (3D) and
/// within 7 of the forward line: a rocket (weapon 0x13, force 1, no target) from `pos + fwd * 4 ± right
/// * 1.5`, the side alternating with bit 0 of `now / 250` (set → the right vector negated through f32
/// locals).
#[hook(addr = 0x42B270, conv = cdecl, name = "CCarCtrl::FireHeliRocketsAtTarget", expect = "83 EC 30 53 56 57")]
pub fn fire_heli_rockets_at_target(veh: GPtr<u8>, target: GPtr<u8>) {
    let kind = rd::<u8>(veh, 0x513);
    if kind != 4 && kind != 0 {
        return;
    }
    let q = |t: u32| ((u64::from(t) * 0x1062_4DD3) >> 32) as u32 >> 4;
    if q(g::<u32>(0xB7CB78).read()) == q(time_ms()) {
        return;
    }
    let pt = entity_pos(target);
    let pv = entity_pos(veh);
    let dx = x_(vx(pv, 0)).fsub_m(vx(pt, 0));
    let dy = x_(vx(pv, 1)).fsub_m(vx(pt, 1));
    let dz = x_(vx(pv, 2)).fsub_m(vx(pt, 2));
    if ge5(dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt(), x_(80.0)) {
        return;
    }
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let pv = entity_pos(veh);
    let a = [
        x_(vx(pv, 0)).fadd_m(rd::<f32>(m, 0x10)).to_f32(),
        x_(rd::<f32>(m, 0x14)).fadd_m(vx(pv, 1)).to_f32(),
        x_(rd::<f32>(m, 0x18)).fadd_m(vx(pv, 2)).to_f32(),
    ];
    let mut a = a;
    let d = X87::from_f64_exact(dist_to_mathematical_line(
        entity_pos(veh),
        GPtr::from_raw(a.as_mut_ptr()),
        entity_pos(target),
    ));
    if ge5(d, x_(7.0)) {
        return;
    }
    let side = q(time_ms()) & 1 != 0;
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let fwd = m.byte_add(0x10);
    let px = x_(rd::<f32>(fwd, 0)).fmul_m(4.0);
    let fy4 = x_(rd::<f32>(fwd, 4)).fmul_m(4.0).to_f32();
    let fz4 = x_(rd::<f32>(fwd, 8)).fmul_m(4.0).to_f32();
    let pos = entity_pos(veh);
    let px = px.fadd_m(vx(pos, 0));
    let py = x_(fy4).fadd_m(vx(pos, 1));
    let pz = x_(fz4).fadd_m(vx(pos, 2)).to_f32();
    let mut rx = x_(rd::<f32>(m, 0)).fmul_m(1.5).to_f32();
    let mut ry = x_(rd::<f32>(m, 4)).fmul_m(1.5);
    let mut rz = x_(rd::<f32>(m, 8)).fmul_m(1.5).to_f32();
    if side {
        rx = x_(rx).fchs().to_f32();
        ry = x_(ry.fchs().to_f32());
        rz = x_(rz).fchs().to_f32();
    }
    let ox = x_(rx).fadd(px).to_f32();
    let oy = ry.fadd(py).to_f32();
    let oz = x_(rz).fadd_m(pz).to_f32();
    add_projectile(veh, 0x13, ox, oy, oz, 1.0, fwd, GPtr::NULL);
}

/// `CCarCtrl::SteerAICarBlockingPlayerForwardAndBack(CVehicle* veh, float* steer, float* gas, float* brake,
/// bool* handbrake)` (0x422B20): steer 0, no handbrake. With `p` = player speed + 0.1 × the player entity's
/// forward (xy), `r` / `f` the car's normalised 2D right / forward vectors and `d` = player - car:
/// `t = -(r · d) / (r · p)` (0.01 when `r · p` is 0, the f32 stored in the steer argument slot);
/// `t < 0` → gas 0, brake 0. Otherwise `D = (f · p) * t + f · d - (f · v) * t` (forward speed `f · v`):
/// `D > 0` → gas `min(D / 10, 1)`, brake 0; moving forward → gas 0, brake `min(-D / 10, 1)` and handbrake
/// above 0.95; else gas `max(D / 10, -1)`, brake 0.
#[hook(addr = 0x422B20, conv = cdecl, name = "CCarCtrl::SteerAICarBlockingPlayerForwardAndBack", expect = "83 EC 30 8B 44 24 38")]
pub fn steer_ai_car_blocking_player_forward_and_back(
    veh: GPtr<u8>,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
    hb: GPtr<u8>,
) {
    steer.write(0.0);
    hb.write(0);
    let s = find_player_speed(-1);
    let (sx, sy) = (vx(s, 0), vx(s, 1));
    let pe = find_player_entity(-1);
    if rd::<GPtr<u8>>(pe, 0x14).is_null() {
        allocate_matrix(pe);
        update_matrix(pe.byte_add(4), rd::<GPtr<u8>>(pe, 0x14));
    }
    let pm = rd::<GPtr<u8>>(pe, 0x14);
    let fx1 = x_(rd::<f32>(pm, 0x10)).fmul_m(k(0x3DCC_CCCD));
    let fy1 = x_(rd::<f32>(pm, 0x14)).fmul_m(k(0x3DCC_CCCD));
    let vm = rd::<GPtr<u8>>(veh, 0x14);
    let px = fx1.fadd_m(sx).to_f32(); // [E-0x18]
    let py = x_(sy).fadd(fy1).to_f32(); // [E-0x14]
    let mut r = [rd::<f32>(vm, 0), rd::<f32>(vm, 4), 0.0]; // [E-0x24]
    let rp = GPtr::from_raw(r.as_mut_ptr());
    vector_normalise(rp);
    let vm = rd::<GPtr<u8>>(veh, 0x14);
    let mut f = [rd::<f32>(vm, 0x10), rd::<f32>(vm, 0x14), 0.0]; // [E-0x30]
    let fp = GPtr::from_raw(f.as_mut_ptr());
    vector_normalise(fp);
    let (rx, ry, rz) = (vx(rp, 0), vx(rp, 1), vx(rp, 2));
    let (ffx, ffy, ffz) = (vx(fp, 0), vx(fp, 1), vx(fp, 2));
    let pos = entity_pos(veh);
    let mut out = [0f32; 3];
    let pp = find_player_coors(GPtr::from_raw(out.as_mut_ptr()), -1);
    let dx = x_(vx(pp, 0)).fsub_m(vx(pos, 0));
    let dy = x_(vx(pp, 1)).fsub_m(vx(pos, 1));
    let dz = x_(vx(pp, 2)).fsub_m(vx(pos, 2));
    let side = x_(ry).fmul(dy).fadd(dx.fmul_m(rx)).fadd(dz.fmul_m(rz));
    let mut den = x_(ry).fmul_m(py).fadd(x_(rx).fmul_m(px)).fadd(x_(rz).fmul_m(0.0));
    if !den.fcom(x_(0.0)).test_parity(0x44) {
        den = x_(k(0x3C23_D70A));
    }
    let tx = side.fdiv(den).fchs();
    let t = tx.to_f32(); // [E+0x8]
    if !ge5(tx, x_(0.0)) {
        gas.write(0.0);
        brake.write(0.0);
        return;
    }
    let pos = entity_pos(veh);
    let pp = find_player_coors(GPtr::from_raw(out.as_mut_ptr()), -1);
    let dx = x_(vx(pp, 0)).fsub_m(vx(pos, 0));
    let dy = x_(vx(pp, 1)).fsub_m(vx(pos, 1));
    let dz = x_(vx(pp, 2)).fsub_m(vx(pos, 2));
    let fs = x_(ffy)
        .fmul_m(rd::<f32>(veh, 0x48))
        .fadd(x_(ffz).fmul_m(rd::<f32>(veh, 0x4C)))
        .fadd(x_(ffx).fmul_m(rd::<f32>(veh, 0x44)))
        .to_f32(); // [E+0x14]
    let ut = x_(ffy)
        .fmul_m(py)
        .fadd(x_(ffx).fmul_m(px))
        .fadd(x_(ffz).fmul_m(0.0))
        .fmul_m(t);
    let w = x_(ffy).fmul(dy).fadd(dx.fmul_m(ffx)).fadd(dz.fmul_m(ffz));
    let d = ut.fadd(w).fsub(x_(fs).fmul_m(t));
    if !le41(d, x_(0.0)) {
        let g = d.fmul_m(k(0x3DCC_CCCD));
        gas.write(if ge5(x_(1.0), g) { g.to_f32() } else { 1.0 });
        brake.write(0.0);
        return;
    }
    if !le41(x_(fs), x_(0.0)) {
        let b = d.fmul_m(k(0xBDCC_CCCD));
        gas.write(0.0);
        let b = if ge5(x_(1.0), b) { b } else { x_(1.0) };
        brake.write(b.to_f32());
        if !le41(b, x_(k(0x3F73_3333))) {
            hb.write(1);
        }
        return;
    }
    let g = d.fmul_m(k(0x3DCC_CCCD));
    gas.write(if le41(x_(-1.0), g) { g.to_f32() } else { -1.0 });
    brake.write(0.0);
}

orig! {
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0, ST0)
    fn vector_magnitude_sqr(v: GPtr<f32>) -> f64 = thiscall 0x406DA0 => unported;
    /// `fabsf` (0x406D10, ST0)
    fn fabsf(v: f32) -> f64 = cdecl 0x406D10 => unported;
    /// The vehicle event constructor of 0x4AE1A0 (`(CVehicle*)`, `ret 4`; 0x18 bytes, vtable 0x85AE88, a
    /// registered reference to the vehicle at +0x14)
    fn event_vehicle_ctor(ev: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AE1A0 => unported;
    /// Its destructor (0x4AE270)
    fn event_vehicle_dtor(ev: GPtr<u8>) = thiscall 0x4AE270 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(group: GPtr<u8>, ev: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// 0x421050 (`CMentalState::IncrementAnger`-like, thiscall on the ped's +0x47C, `ret 4`)
    fn increment_anger(p: GPtr<u8>, n: i32) = thiscall 0x421050 => unported;
}

/// Constructs the 0x4AE1A0 vehicle event in a local, adds it to the ped's event group (+0x47C → +0x68)
/// and destroys the local again, as the bytes do around their SEH state.
fn add_vehicle_event(ped: GPtr<u8>, veh: GPtr<u8>, then: impl FnOnce()) {
    let mut ev = [0u32; 6];
    let evp: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    event_vehicle_ctor(evp, veh);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), evp, 0);
    then();
    event_vehicle_dtor(evp);
}

/// `CCarCtrl::SlowCarDownForPedsSectorList(CPtrListDoubleLink&, CVehicle* veh, float minX, float minY, float
/// maxX, float maxY, float* maxSpeed, float speed)` (0x425440): `fs` = the forward speed (`f · v`). For
/// every ped using collision, not yet scanned, inside the box, within 6 in height and 3 of the forward
/// line: for temp actions 0, 1, 4, 6 (and not the player ped while the player's remote vehicle is `veh`),
/// a ped ahead beyond the bounding max.y (by less than `fs * 200` and 13) and beside within max.x + 0.5
/// (×1.6 per such ped for bikes, kept in the local) limits `maxSpeed` to `max(1, max(0, d - 1) / 13 *
/// speed)`, sets +0x3DB bit 1 and temp action 1 (below 4) / 0x18 (below 2.5) for 4 s. Peds near the
/// player's honking vehicle (+0x514, within 7) get the vehicle event; for an unwilling-to-stop car (the
/// player's, special types 3/4/6, temp actions other than 0/6, or status 3) a ped in its path (same side as
/// the motion, beyond max.y by less than `|fs| * 50`, `|fs| > 0.05`, beside within max.x + 0.35) gets the
/// event too and, with a player driver, anger 2.
#[hook(addr = 0x425440, conv = cdecl, name = "CCarCtrl::SlowCarDownForPedsSectorList<CPtrListDoubleLink<CPhysical*>>", expect = "6A FF 68 40 74 83 00")]
#[allow(clippy::too_many_arguments)]
pub fn slow_car_down_for_peds_sector_list(
    list: GPtr<u8>,
    veh: GPtr<u8>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    max_speed: GPtr<f32>,
    speed: f32,
) {
    let mut half_w = rd::<f32>(col_model(veh), 0xC); // [E-0x84]
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let fsx = x_(rd::<f32>(m, 0x18))
        .fmul_m(rd::<f32>(veh, 0x4C))
        .fadd(x_(rd::<f32>(m, 0x14)).fmul_m(rd::<f32>(veh, 0x48)))
        .fadd(x_(rd::<f32>(m, 0x10)).fmul_m(rd::<f32>(veh, 0x44)));
    let fs = fsx.to_f32(); // [E-0x8c]
    let ahead_lim = fsx.fmul_m(200.0).to_f32(); // [E-0x68]
    let path_lim = x_(fs).fabs().fmul_m(50.0).to_f32(); // [E-0x64]
    let unwilling = {
        let pv = find_player_vehicle(-1, 0);
        let ty = rd::<i32>(veh, 0x590);
        let ta = rd::<u8>(veh, 0x3B9);
        veh == pv
            || ty == 6
            || ty == 4
            || ty == 3
            || (ta != 0 && ta != 6)
            || rd::<u8>(veh, 0x36) & 0xF8 == 0x18
    };
    let half_l = rd::<f32>(col_model(veh), 0x10); // [E-0x80]
    let mut node = rd::<GPtr<u8>>(list, 0);
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        let sc = scan_code();
        node = rd::<GPtr<u8>>(node, 4);
        if rd::<u16>(e, 0x2C) == sc || rd::<u8>(e, 0x1C) & 1 == 0 {
            continue;
        }
        wr::<u16>(e, 0x2C, sc);
        let pe = entity_pos(e);
        let (px, py, pz) = (vx(pe, 0), vx(pe, 1), vx(pe, 2));
        if outside_box(px, py, min_x, min_y, max_x, max_y) {
            continue;
        }
        let pv = entity_pos(veh);
        let dz = x_(pz).fsub_m(vx(pv, 2));
        let dz = if ge5(dz, x_(0.0)) { dz } else { dz.fchs() };
        if ge5(dz, x_(6.0)) {
            continue;
        }
        let vm = rd::<GPtr<u8>>(veh, 0x14);
        let pv = entity_pos(veh);
        let d = X87::from_f64_exact(dist_along_line_2d(
            vx(pv, 0),
            vx(pv, 1),
            rd::<f32>(vm, 0x10),
            rd::<f32>(vm, 0x14),
            px,
            py,
        ))
        .to_f32();
        let pv = entity_pos(veh);
        let mut fwd = [0f32; 3]; // [E-0x24]
        let fv = get_forward(veh, GPtr::from_raw(fwd.as_mut_ptr()));
        let h = x_(d).fmul_m(vx(fv, 2)).fadd_m(vx(pv, 2)).fsubr_m(pz);
        if ge5(abs5(h), x_(3.0)) {
            continue;
        }
        let vm = rd::<GPtr<u8>>(veh, 0x14);
        let pv = entity_pos(veh);
        let rz = x_(pz).fsub_m(vx(pv, 2)).to_f32(); // [E-0x90]
        let ry = x_(py).fsub_m(vx(pv, 1)).to_f32(); // [E-0x70]
        let rx = x_(px).fsub_m(vx(pv, 0)).to_f32(); // [E-0x60]
        let mut fd = x_(ry)
            .fmul_m(rd::<f32>(vm, 0x14))
            .fadd(x_(rz).fmul_m(rd::<f32>(vm, 0x18)))
            .fadd(x_(rx).fmul_m(rd::<f32>(vm, 0x10)))
            .to_f32(); // [E-0x94]
        let side = |half: f32, add: f32| {
            let vm = rd::<GPtr<u8>>(veh, 0x14);
            let s = x_(ry)
                .fmul_m(rd::<f32>(vm, 4))
                .fadd(x_(rz).fmul_m(rd::<f32>(vm, 8)))
                .fadd(x_(rx).fmul_m(rd::<f32>(vm, 0)))
                .fabs();
            // `fcompp` of side against `half + add`, `test ah, 0x41; jp` skips when greater (or unordered)
            !s.fcom(x_(half).fadd_m(add)).test_parity(0x41)
        };
        let ta = rd::<u8>(veh, 0x3B9);
        let mut slow = ta == 0 || ta == 4 || ta == 1 || ta == 6;
        if slow && e == find_player_ped(-1) {
            let idx = u32::from(g::<u8>(0xB7CD74).read());
            if g::<GPtr<u8>>((0xB7CE70 + idx * 0x190) as usize).read() == veh {
                slow = false;
            }
        }
        if slow && !le41(x_(fd), x_(half_l)) {
            let ax = x_(fd).fsub_m(half_l);
            let ahead = ax.to_f32(); // [E-0x88]
            if !ge5(ax, x_(ahead_lim)) {
                if rd::<i32>(veh, 0x590) == 9 {
                    half_w = x_(half_w).fmul_m(k(0x3FCC_CCCD)).to_f32();
                }
                if side(half_w, 0.5) && !ge5(x_(ahead), x_(13.0)) {
                    let c = x_(ahead).fsub_m(1.0).to_f32(); // [E-0x98]
                    let c0 = || if le41(x_(0.0), x_(c)) { x_(c) } else { x_(0.0) };
                    let k13 = k(0x3D9D_89D9);
                    let v = x_(k13).fmul(c0()).fmul_m(speed);
                    let a = if le41(x_(1.0), v) {
                        c0().fmul_m(k13).fmul_m(speed)
                    } else {
                        x_(1.0)
                    };
                    let ms = max_speed.read();
                    let r = if le41(a, x_(ms)) {
                        let v = x_(k13).fmul(c0()).fmul_m(speed);
                        if le41(x_(1.0), v) {
                            c0().fmul_m(k13).fmul_m(speed)
                        } else {
                            x_(1.0)
                        }
                    } else {
                        x_(ms)
                    };
                    max_speed.write(r.to_f32());
                    wr::<u8>(veh, 0x3DB, rd::<u8>(veh, 0x3DB) | 2);
                    if !ge5(x_(ahead), x_(4.0)) {
                        wr::<u8>(veh, 0x3BB, 1);
                        wr::<u32>(veh, 0x3BC, time_ms().wrapping_add(4000));
                    }
                    if !ge5(x_(ahead), x_(2.5)) {
                        wr::<u8>(veh, 0x3BB, 0x18);
                        wr::<u32>(veh, 0x3BC, time_ms().wrapping_add(4000));
                    }
                }
            }
        }
        if rd::<u8>(e, 0x36) & 7 != 3 {
            continue;
        }
        if veh == find_player_vehicle(-1, 0) && rd::<u32>(veh, 0x514) != 0 {
            let mut dv = [0f32; 3]; // [E-0x18]
            let r = vector_sub(GPtr::from_raw(dv.as_mut_ptr()), entity_pos(e), entity_pos(veh));
            if !ge5(X87::from_f64_exact(vector_magnitude_sqr(r)), x_(49.0)) {
                add_vehicle_event(e, veh, || {});
            }
        }
        if !unwilling {
            continue;
        }
        // `test ah, 0x44; jnp`: nothing when the forward speed is exactly 0
        if !x_(fs).fcom(x_(0.0)).test_parity(0x44) {
            continue;
        }
        let s1 = if ge5(x_(fd), x_(0.0)) { 1 } else { -1 };
        let s2 = if ge5(x_(fs), x_(0.0)) { 1 } else { -1 };
        if s1 != s2 {
            continue;
        }
        let fda = x_(fd).fabs();
        fd = fda.to_f32();
        if le41(fda, x_(half_l)) {
            continue;
        }
        if le41(X87::from_f64_exact(fabsf(fs)), x_(k(0x3D4C_CCCD))) {
            continue;
        }
        if ge5(x_(fd).fsub_m(half_l), x_(path_lim)) {
            continue;
        }
        if !side(half_w, k(0x3EB3_3333)) {
            continue;
        }
        add_vehicle_event(e, veh, || {
            let drv = rd::<GPtr<u8>>(veh, 0x460);
            if !drv.is_null() && ped_is_player(drv) != 0 {
                increment_anger(rd::<GPtr<u8>>(e, 0x47C), 2);
            }
        });
    }
}

orig! {
    /// `CMatrix * CVector` (0x59C890, `(CVector* out, const CMatrix*, const CVector*)`, returns `out`)
    fn matrix_mul_vec(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C890 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix*, const CVector*)` (0x59C790, returns `out`)
    fn multiply_3x3(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C790 => unported;
    /// The unnamed heading test of the weave loops (0x421A50): `(float angle, CVector* p1, CVector* d1,
    /// CVector* p2, CVector* d2, CVector* p3, CVector* d3, CVector* p4, CVector* d4, float vx, float vy, float
    /// speed, bool)`; true when the heading is blocked
    #[allow(clippy::too_many_arguments)]
    fn weave_heading_blocked(
        angle: f32,
        p1: GPtr<f32>,
        d1: GPtr<f32>,
        p2: GPtr<f32>,
        d2: GPtr<f32>,
        p3: GPtr<f32>,
        d3: GPtr<f32>,
        p4: GPtr<f32>,
        d4: GPtr<f32>,
        vx: f32,
        vy: f32,
        speed: f32,
        flag: u32,
    ) -> u8 = cdecl 0x421A50 => unported;
}

/// `CCarCtrl::WeaveForOtherCar(CEntity* other, CVehicle* veh, float* a, float* b)` (0x426350): skips the
/// pairs the missions exclude (3: the player's vehicle; 0x10 / 0x35: the target, 0x35 also flagged
/// +0x42B bit 3 vehicles; 0x37: a ped whose +0x58C is the target; the other car's escort missions
/// 0x1B..0x20 toward `veh`) and cars behind (forward dot < 0). Each car's bounding box grown by 0.2
/// gives a corner point (`CMatrix * v`, the side and end chosen by the sign of the other's offset) and the
/// two edge vectors (`Multiply3x3`); `a` is turned down and `b` up by 6° (wrapped into [0, 2π)) up to 8
/// times while 0x421A50 reports the heading blocked. The thirteenth argument is the side dot product's
/// f32 local with its low byte replaced by `veh max.x < other max.x`, as the bytes pass it.
#[hook(addr = 0x426350, conv = cdecl, name = "CCarCtrl::WeaveForOtherCar", expect = "81 EC 84 00 00 00")]
pub fn weave_for_other_car(other: GPtr<u8>, veh: GPtr<u8>, a: GPtr<f32>, b: GPtr<f32>) {
    let mission = rd::<u8>(veh, 0x3BA);
    if mission == 3 && other == find_player_vehicle(-1, 0) {
        return;
    }
    let mission = rd::<u8>(veh, 0x3BA);
    let target = rd::<GPtr<u8>>(veh, 0x41C);
    if mission == 0x10 && other == target {
        return;
    }
    if mission == 0x35
        && (other == target || (rd::<u8>(other, 0x36) & 7 == 2 && rd::<u8>(other, 0x42B) & 8 != 0))
    {
        return;
    }
    if mission == 0x37
        && rd::<u8>(other, 0x36) & 7 == 3
        && rd::<u32>(other, 0x46C) & 0x100 != 0
        && rd::<GPtr<u8>>(other, 0x58C) == rd::<GPtr<u8>>(veh, 0x41C)
    {
        return;
    }
    if matches!(rd::<u8>(other, 0x3BA), 0x1B..=0x20) && rd::<GPtr<u8>>(other, 0x41C) == veh {
        return;
    }
    let po = entity_pos(other);
    let pv = entity_pos(veh);
    let vm = rd::<GPtr<u8>>(veh, 0x14);
    let dx = x_(vx(po, 0)).fsub_m(vx(pv, 0)).to_f32(); // [E-0x80]
    let dyx = x_(vx(po, 1)).fsub_m(vx(pv, 1));
    let dy = dyx.to_f32(); // [E-0x58]
    let dot = dyx
        .fmul_m(rd::<f32>(vm, 0x14))
        .fadd(x_(dx).fmul_m(rd::<f32>(vm, 0x10)));
    if !dot.fcom(x_(0.0)).test_parity(0x05) {
        return;
    }
    let vside = x_(dy)
        .fmul_m(rd::<f32>(vm, 4))
        .fadd(x_(dx).fmul_m(rd::<f32>(vm, 0)))
        .to_f32();
    let g = k(0x3E4C_CCCD);
    let cv = col_model(veh);
    let mut tmp = [0f32; 3]; // [E-0x54]
    let tp = GPtr::from_raw(tmp.as_mut_ptr());
    let corner = |m: GPtr<u8>, x: f32, y: f32| -> [f32; 3] {
        let mut v = [x, y, 0.0];
        let r = matrix_mul_vec(tp, m, GPtr::from_raw(v.as_mut_ptr()));
        [vx(r, 0), vx(r, 1), vx(r, 2)]
    };
    let edge = |m: GPtr<u8>, x: f32, y: f32| -> [f32; 3] {
        let mut v = [x, y, 0.0];
        let r = multiply_3x3(tp, m, GPtr::from_raw(v.as_mut_ptr()));
        [vx(r, 0), vx(r, 1), vx(r, 2)]
    };
    let ext = |lo: f32, hi: f32| x_(hi).fadd_m(g).fsub_m(lo).fsub_m(g).to_f32();
    let y1 = if le41(dot, x_(0.0)) {
        x_(rd::<f32>(cv, 4)).fsub_m(g)
    } else {
        x_(rd::<f32>(cv, 0x10)).fadd_m(g)
    }
    .to_f32();
    let x1 = x_(rd::<f32>(cv, 0)).fsub_m(g).to_f32();
    let mut w1 = corner(vm, x1, y1); // [E-0x3c]
    let mut r1 = edge(vm, ext(rd::<f32>(cv, 0), rd::<f32>(cv, 0xC)), 0.0); // [E-0x48]
    let y2 = x_(rd::<f32>(cv, 4)).fsub_m(g).to_f32();
    let x2 = if le41(x_(vside), x_(0.0)) {
        x_(rd::<f32>(cv, 0)).fsub_m(g)
    } else {
        x_(rd::<f32>(cv, 0xC)).fadd_m(g)
    }
    .to_f32();
    let mut w2 = corner(vm, x2, y2); // [E-0x24]
    let mut r2 = edge(vm, 0.0, ext(rd::<f32>(cv, 4), rd::<f32>(cv, 0x10))); // [E-0x7c]
    let om = rd::<GPtr<u8>>(other, 0x14);
    let co = col_model(other);
    let oside = x_(dy)
        .fmul_m(rd::<f32>(om, 4))
        .fadd(x_(dx).fmul_m(rd::<f32>(om, 0)))
        .to_f32(); // [E-0x84]
    let odot = x_(dy)
        .fmul_m(rd::<f32>(om, 0x14))
        .fadd(x_(dx).fmul_m(rd::<f32>(om, 0x10)));
    let y3 = if ge5(odot, x_(0.0)) {
        x_(rd::<f32>(co, 4)).fsub_m(g)
    } else {
        x_(rd::<f32>(co, 0x10)).fadd_m(g)
    }
    .to_f32();
    let x3 = x_(rd::<f32>(co, 0)).fsub_m(g).to_f32();
    let mut w3 = corner(om, x3, y3); // [E-0x30]
    let mut r3 = edge(om, ext(rd::<f32>(co, 0), rd::<f32>(co, 0xC)), 0.0); // [E-0x64]
    let y4 = x_(rd::<f32>(co, 4)).fsub_m(g).to_f32();
    let x4 = if ge5(x_(oside), x_(0.0)) {
        x_(rd::<f32>(co, 0)).fsub_m(g)
    } else {
        x_(rd::<f32>(co, 0xC)).fadd_m(g)
    }
    .to_f32();
    let mut w4 = corner(om, x4, y4); // [E-0x18]
    let mut r4 = edge(om, 0.0, ext(rd::<f32>(co, 4), rd::<f32>(co, 0x10))); // [E-0x70]
    let narrower = !ge5(
        x_(rd::<f32>(col_model(veh), 0xC)),
        x_(rd::<f32>(col_model(other), 0xC)),
    );
    let flag = (oside.to_bits() & 0xFFFF_FF00) | u32::from(narrower);
    let (sx, sy) = (rd::<f32>(veh, 0x44), rd::<f32>(veh, 0x48));
    let speed = x_(sx).fmul_m(sx).fadd(x_(sy).fmul_m(sy)).fsqrt().to_f32();
    let mut blocked = |angle: f32| {
        weave_heading_blocked(
            angle,
            GPtr::from_raw(w1.as_mut_ptr()),
            GPtr::from_raw(r1.as_mut_ptr()),
            GPtr::from_raw(w2.as_mut_ptr()),
            GPtr::from_raw(r2.as_mut_ptr()),
            GPtr::from_raw(w3.as_mut_ptr()),
            GPtr::from_raw(r3.as_mut_ptr()),
            GPtr::from_raw(w4.as_mut_ptr()),
            GPtr::from_raw(r4.as_mut_ptr()),
            rd::<f32>(other, 0x44),
            rd::<f32>(other, 0x48),
            speed,
            flag,
        ) != 0
    };
    let step = k(0x3DD6_7750);
    let tau = k(0x40C9_0FDB);
    for _ in 0..8 {
        if !blocked(a.read()) {
            break;
        }
        let v = x_(a.read()).fsub_m(step);
        a.write(v.to_f32());
        if !ge5(v, x_(0.0)) {
            a.write(v.fadd_m(tau).to_f32());
        }
    }
    for _ in 0..8 {
        if !blocked(b.read()) {
            break;
        }
        let v = x_(b.read()).fadd_m(step);
        b.write(v.to_f32());
        if !le41(v, x_(tau)) {
            b.write(v.fsub_m(tau).to_f32());
        }
    }
}

/// `CCarCtrl::SteerAIHeliToFollowEntity(CAutomobile*)` (0x42A750): +0x3EC = the target entity's position
/// (+0x41C), shifted by its forward × the signed byte +0x3DA (z offset `n * 0.0`) when that is set; angle to
/// that point, 2D distance to the target itself, height +0x9A8 as for the police heli (at least 6, at
/// least 25 beyond 50); fixed orientation (+0x954 >= 0) → `FlyAIHeliToTarget_FixedOrientation(veh,
/// +0x954, target position, true)`, else `FlyAIHeliInCertainDirection(veh, angle, dist, true)`. +0x3DB
/// bit 5 with the target within 25: a physics/ghost target vehicle (status 2 or 3) gets mission 1, cruise
/// 100, temp action 2 and status 3; a ped's active task 0x390 gets +0xC = 7; then the bit is cleared.
/// Bit 7 expires 50 s after +0x430 (mission 0x2A); mission 0x29 below 300 health → 0x2A.
#[hook(addr = 0x42A750, conv = cdecl, name = "CCarCtrl::SteerAIHeliToFollowEntity", expect = "83 EC 1C 53 55 56")]
pub fn steer_ai_heli_to_follow_entity(veh: GPtr<u8>) {
    let t = rd::<GPtr<u8>>(veh, 0x41C);
    let pt = entity_pos(t);
    for i in 0..3 {
        wr::<u32>(
            veh,
            0x3EC + i * 4,
            pt.cast::<u32>().wrapping_index(i as i32).read(),
        );
    }
    let n = rd::<i8>(veh, 0x3DA);
    if n != 0 {
        ensure_matrix(t);
        let m = rd::<GPtr<u8>>(t, 0x14);
        let (fx, fy) = (rd::<f32>(m, 0x10), rd::<f32>(m, 0x14));
        let a = xi_(i32::from(n));
        let ox = x_(fx).fmul(a);
        let oy = x_(fy).fmul(a).to_f32();
        let oz = a.fmul_m(0.0).to_f32();
        wr::<f32>(veh, 0x3EC, ox.fadd_m(rd::<f32>(veh, 0x3EC)).to_f32());
        wr::<f32>(veh, 0x3F0, x_(oy).fadd_m(rd::<f32>(veh, 0x3F0)).to_f32());
        wr::<f32>(veh, 0x3F4, x_(oz).fadd_m(rd::<f32>(veh, 0x3F4)).to_f32());
    }
    let p = entity_pos(veh);
    let dy = x_(rd::<f32>(veh, 0x3F0)).fsub_m(vx(p, 1)).to_f32();
    let dx = x_(rd::<f32>(veh, 0x3EC)).fsub_m(vx(p, 0)).to_f32();
    let ang = atan_xy(dx, dy).to_f32();
    let t = rd::<GPtr<u8>>(veh, 0x41C);
    let pt = entity_pos(t);
    let pv = entity_pos(veh);
    let yy = x_(vx(pt, 1)).fsub_m(vx(pv, 1));
    let xx = x_(vx(pt, 0)).fsub_m(vx(pv, 0));
    let dist = yy.fmul(yy).fadd(xx.fmul(xx)).fsqrt().to_f32();
    let z = x_(vx(pt, 2));
    let h = if !le41(z, x_(6.0)) { z } else { x_(6.0) };
    wr::<f32>(veh, 0x9A8, h.to_f32());
    if !le41(x_(dist), x_(50.0)) {
        let h2 = if !le41(h, x_(25.0)) { h } else { x_(25.0) };
        wr::<f32>(veh, 0x9A8, h2.to_f32());
    }
    let o = rd::<f32>(veh, 0x954);
    if !x_(o).fcom(x_(0.0)).test_any(0x01) {
        let pt = entity_pos(t);
        fly_ai_heli_to_target_fixed_orientation(veh, o, vx(pt, 0), vx(pt, 1), vx(pt, 2), 1);
    } else {
        fly_ai_heli_in_certain_direction(veh, ang, dist, 1);
    }
    if rd::<u8>(veh, 0x3DB) & 0x20 != 0 {
        let t = rd::<GPtr<u8>>(veh, 0x41C);
        let pt = entity_pos(t);
        let pv = entity_pos(veh);
        let ex = x_(vx(pv, 0)).fsub_m(vx(pt, 0));
        let ey = x_(vx(pv, 1)).fsub_m(vx(pt, 1));
        if !ge5(ey.fmul(ey).fadd(ex.fmul(ex)).fsqrt(), x_(25.0)) {
            let ty = rd::<u8>(t, 0x36) & 7;
            if ty == 2 {
                let s = rd::<u8>(t, 0x36);
                if s >> 3 == 2 || s >> 3 == 3 {
                    wr::<u8>(t, 0x3BA, 1);
                    wr::<u8>(t, 0x3D0, 0x64);
                    wr::<u8>(t, 0x3B9, 2);
                    wr::<u8>(t, 0x36, (s & 7) | 0x18);
                }
            } else if ty == 3 {
                let task = task_manager_get_active_task(rd::<GPtr<u8>>(t, 0x47C).byte_add(4));
                if !task.is_null() && task_get_type(task) == 0x390 {
                    wr::<u32>(task, 0xC, 7);
                }
            }
            wr::<u8>(veh, 0x3DB, rd::<u8>(veh, 0x3DB) & 0xDF);
        }
    }
    let f = rd::<u8>(veh, 0x3DB);
    if f & 0x80 != 0 && time_ms() > rd::<u32>(veh, 0x430).wrapping_add(50000) {
        wr::<u8>(veh, 0x3DB, f & 0x7F);
        wr::<u8>(veh, 0x3BA, 0x2A);
    }
    if rd::<u8>(veh, 0x3BA) == 0x29 && !ge5(x_(rd::<f32>(veh, 0x4C0)), x_(300.0)) {
        wr::<u8>(veh, 0x3BA, 0x2A);
    }
}

/// `CCarCtrl::SteerAIHeliToKeepEntityInView(CAutomobile*)` (0x42AEB0): farther than 2 × the byte +0x3D8
/// (2D) from the target entity (+0x41C) → `SteerAIHeliToFollowEntity`. Otherwise the heli turns sideways to
/// it: +0x98C = `-wrap(angle + π/2 - heading)` clamped to ±1 (stored once before the clamp; this wrap
/// runs the `> π` loop first), +0x9A8 = +0x9AC = target z + 15, +0x3EC = the target position, +0x998
/// (lift) = `0.3 + (height error) * (0.1 above / 0.2 otherwise) + ((rand() & 15) - 7) * 0.002` clamped to
/// [0, 1], +0x994 = 0.5 inside half the +0x3D8 distance, -0.5 beyond the whole of it, else the right vector's dot
/// product with the speed; +0x990 = the forward speed within 1.5 × +0x3D8 (else 0).
#[hook(addr = 0x42AEB0, conv = cdecl, name = "CCarCtrl::SteerAIHeliToKeepEntityInView", expect = "83 EC 08 53 56")]
pub fn steer_ai_heli_to_keep_entity_in_view(veh: GPtr<u8>) {
    let t = rd::<GPtr<u8>>(veh, 0x41C);
    let pt = entity_pos(t);
    let pv = entity_pos(veh);
    let dy = x_(vx(pt, 1)).fsub_m(vx(pv, 1)).to_f32();
    let dx = x_(vx(pt, 0)).fsub_m(vx(pv, 0)).to_f32();
    let a = atan_xy(dx, dy);
    let t = rd::<GPtr<u8>>(veh, 0x41C);
    let pt = entity_pos(t);
    let pv = entity_pos(veh);
    let yy = x_(vx(pt, 1)).fsub_m(vx(pv, 1));
    let xx = x_(vx(pt, 0)).fsub_m(vx(pv, 0));
    let distx = yy.fmul(yy).fadd(xx.fmul(xx)).fsqrt();
    let dist = distx.to_f32();
    let n = xi_(i32::from(rd::<u8>(veh, 0x3D8)));
    if !le41(distx, n.fadd(n)) {
        steer_ai_heli_to_follow_entity(veh);
        return;
    }
    let vm = rd::<GPtr<u8>>(veh, 0x14);
    let a2 = a.fadd_m(k(0x3FC9_0FDB)).to_f32();
    let mut d = atan_xy(rd::<f32>(vm, 0x10), rd::<f32>(vm, 0x14)).fsubr_m(a2);
    let pi = x_(k(0x4049_0FDB));
    let npi = x_(k(0xC049_0FDB));
    let tau = k(0x40C9_0FDB);
    if !le41(d, pi) {
        loop {
            d = d.fsub_m(tau);
            if le41(d, pi) {
                break;
            }
        }
    }
    if !ge5(d, npi) {
        loop {
            d = d.fadd_m(tau);
            if ge5(d, npi) {
                break;
            }
        }
    }
    let r = d.fmul_m(-1.0);
    wr::<f32>(veh, 0x98C, r.to_f32());
    let c = if !ge5(r, x_(1.0)) && !le41(x_(-1.0), r) {
        x_(-1.0)
    } else if !r.fcom(x_(1.0)).test_parity(0x05) {
        r
    } else {
        x_(1.0)
    };
    wr::<f32>(veh, 0x98C, c.to_f32());
    let t = rd::<GPtr<u8>>(veh, 0x41C);
    wr::<f32>(veh, 0x9A8, x_(vx(entity_pos(t), 2)).fadd_m(15.0).to_f32());
    let pt = entity_pos(t);
    let h = x_(rd::<f32>(veh, 0x9A8));
    for i in 0..3 {
        wr::<u32>(
            veh,
            0x3EC + i * 4,
            pt.cast::<u32>().wrapping_index(i as i32).read(),
        );
    }
    wr::<f32>(veh, 0x9AC, h.to_f32());
    let pv = entity_pos(veh);
    wr::<u32>(veh, 0x998, 0x3E99_999A);
    let e = h.fsub(x_(rd::<f32>(veh, 0x4C)).fmul_m(100.0).fadd_m(vx(pv, 2)));
    let lift = if le41(e, x_(0.0)) {
        e.fmul_m(k(0x3E4C_CCCD))
    } else {
        e.fmul_m(k(0x3DCC_CCCD))
    }
    .fadd_m(k(0x3E99_999A));
    wr::<f32>(veh, 0x998, lift.to_f32());
    let rnd = crate::common::rand_at(0x42B14C) & 0xF;
    let v = xi_(rnd)
        .fsub_m(7.0)
        .fmul_m(k(0x3B03_126F))
        .fadd_m(rd::<f32>(veh, 0x998));
    wr::<f32>(veh, 0x998, v.to_f32());
    let v = if !ge5(v, x_(1.0)) && !le41(x_(0.0), v) {
        x_(0.0)
    } else if !v.fcom(x_(1.0)).test_parity(0x05) {
        v
    } else {
        x_(1.0)
    };
    let n = xi_(i32::from(rd::<u8>(veh, 0x3D8)));
    wr::<f32>(veh, 0x998, v.to_f32());
    if !le41(x_(0.5).fmul(n), x_(dist)) {
        wr::<f32>(veh, 0x994, 0.5);
    } else if !le41(x_(dist), n) {
        wr::<f32>(veh, 0x994, -0.5);
    } else {
        let m = rd::<GPtr<u8>>(veh, 0x14);
        let s = x_(rd::<f32>(veh, 0x4C))
            .fmul_m(rd::<f32>(m, 8))
            .fadd(x_(rd::<f32>(veh, 0x48)).fmul_m(rd::<f32>(m, 4)))
            .fadd(x_(rd::<f32>(m, 0)).fmul_m(rd::<f32>(veh, 0x44)));
        wr::<f32>(veh, 0x994, s.to_f32());
    }
    wr::<u32>(veh, 0x990, 0);
    if !ge5(x_(dist), x_(1.5).fmul(n)) {
        let m = rd::<GPtr<u8>>(veh, 0x14);
        let s = x_(rd::<f32>(veh, 0x4C))
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(x_(rd::<f32>(veh, 0x48)).fmul_m(rd::<f32>(m, 0x14)))
            .fadd(x_(rd::<f32>(veh, 0x44)).fmul_m(rd::<f32>(m, 0x10)));
        wr::<f32>(veh, 0x990, s.to_f32());
    }
}

/// `CCarCtrl::SteerAICarTowardsPointInEscort(CVehicle* veh, CVehicle* leader, float ox, float oy, float*
/// steer, float* gas, float* brake, bool* handbrake)` (0x4336D0): the escort point `P` = leader matrix ×
/// (ox, oy, 0) + the leader's speed (xy); the turn toward `P + 3 × leader forward` (always weaved, range 1;
/// clamped to `FindMaxSteerAngle`). With `along` = the offset to `P` on the normalised forward and `dist`
/// its 2D length: `along > 0.5` → target speed `min(along - 0.6, 4) + speed` capped at `speed + 10` (or
/// `3.5 × dist` when that is not less, 300 when `dist >= 15`); otherwise 8 when `dist >= 15`, else stopped
/// (steer 0, gas 0, brake 1, or 0.1 when `along >= -3`). Gas `min(diff / 10, 1)` (1 at 25 or more) or brake
/// `min(-diff / 20, 0.5)` from the difference to `|v| * 60`. The angle to `P` itself is computed and
/// dropped, as the bytes do.
#[hook(addr = 0x4336D0, conv = cdecl, name = "CCarCtrl::SteerAICarTowardsPointInEscort", expect = "83 EC 24 8B 44 24 30")]
#[allow(clippy::too_many_arguments)]
pub fn steer_ai_car_towards_point_in_escort(
    veh: GPtr<u8>,
    leader: GPtr<u8>,
    ox: f32,
    oy: f32,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
    hb: GPtr<u8>,
) {
    let mut off = [ox, oy, 0.0];
    let mut out = [0f32; 3];
    let w = matrix_mul_vec(
        GPtr::from_raw(out.as_mut_ptr()),
        rd::<GPtr<u8>>(leader, 0x14),
        GPtr::from_raw(off.as_mut_ptr()),
    );
    let (wx, wy) = (vx(w, 0), vx(w, 1));
    let px = x_(rd::<f32>(leader, 0x44)).fadd_m(wx).to_f32();
    hb.write(0);
    let py = x_(rd::<f32>(leader, 0x48)).fadd_m(wy).to_f32();
    let (fxn, fyn) = forward_2d_normalised(rd::<GPtr<u8>>(veh, 0x14));
    let lm = rd::<GPtr<u8>>(leader, 0x14);
    let lx3 = x_(rd::<f32>(lm, 0x10)).fmul_m(3.0);
    let ly3 = x_(rd::<f32>(lm, 0x14)).fmul_m(3.0);
    let ax = lx3.fadd_m(px).to_f32();
    let ay = ly3.fadd_m(py);
    let p = entity_pos(veh);
    let dy = ay.fsub_m(vx(p, 1)).to_f32();
    let dx = x_(ax).fsub_m(vx(p, 0)).to_f32();
    let a = atan_xy(dx, dy).to_f32();
    let h = atan_xy(fxn, fyn).to_f32();
    let wv = X87::from_f64_exact(find_angle_to_weave_call(veh, GPtr::NULL, a, h, 1.0));
    let mut turn = wrap_pi(wv.fsub_m(h)).to_f32();
    let ms = X87::from_f64_exact(find_max_steer_angle(veh));
    let nms = ms.fchs();
    if !ge5(x_(turn), nms) {
        turn = nms.to_f32();
    }
    if !le41(x_(turn), ms) {
        turn = ms.to_f32();
    }
    let p = entity_pos(veh);
    let qy = x_(py).fsub_m(vx(p, 1)).to_f32();
    let qx = x_(px).fsub_m(vx(p, 0)).to_f32();
    let _ = atan_xy(qx, qy);
    let p = entity_pos(veh);
    let along = x_(py)
        .fsub_m(vx(p, 1))
        .fmul_m(fyn)
        .fadd(x_(px).fsub_m(vx(p, 0)).fmul_m(fxn));
    let ry = x_(py).fsub_m(vx(p, 1));
    let rx = x_(px).fsub_m(vx(p, 0));
    let distx = ry.fmul(ry).fadd(rx.fmul(rx)).fsqrt();
    let dist = distx.to_f32();
    let target: X87;
    if !le41(along, x_(0.5)) {
        let spd = speed_3d(veh).fmul_m(60.0).to_f32();
        if ge5(distx, x_(15.0)) {
            target = x_(300.0);
        } else {
            let u = along.fsub_m(0.5).fsub_m(k(0x3DCC_CCCD));
            let m = if ge5(x_(4.0), u) { u } else { x_(4.0) };
            let d35 = x_(dist).fmul_m(3.5).to_f32();
            let t0 = if le41(x_(spd).fadd(m), x_(d35)) {
                x_(d35)
            } else {
                let m2 = if ge5(x_(4.0), u) { u } else { x_(4.0) };
                m2.fadd_m(spd)
            };
            let cap = x_(spd).fadd_m(10.0).to_f32();
            target = if !t0.fcom(x_(cap)).test_parity(0x05) {
                t0
            } else {
                x_(cap)
            };
        }
    } else if ge5(distx, x_(15.0)) {
        target = x_(8.0);
    } else {
        steer.write(0.0);
        gas.write(0.0);
        brake.write(if ge5(along, x_(-3.0)) { k(0x3DCC_CCCD) } else { 1.0 });
        return;
    }
    let (mx, my, mz) = (rd::<f32>(veh, 0x44), rd::<f32>(veh, 0x48), rd::<f32>(veh, 0x4C));
    brake.write(0.0);
    let spd2 = x_(mz)
        .fmul_m(mz)
        .fadd(x_(my).fmul_m(my))
        .fadd(x_(mx).fmul_m(mx))
        .fsqrt()
        .fmul_m(60.0)
        .to_f32();
    let diff = target.fsub_m(spd2);
    if !le41(diff, x_(0.0)) {
        if ge5(x_(spd2), x_(25.0)) {
            gas.write(1.0);
        } else {
            let g = diff.fmul_m(k(0x3DCC_CCCD));
            gas.write(if ge5(x_(1.0), g) { g.to_f32() } else { 1.0 });
        }
    } else {
        let b = diff.fmul_m(k(0xBD4C_CCCD));
        gas.write(0.0);
        brake.write(if ge5(x_(0.5), b) { b.to_f32() } else { 0.5 });
    }
    steer.write(turn);
}
