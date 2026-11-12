//! `CEventHandler::ComputeVehiclePotentialCollisionResponse` (batch `intro-ai-5`): a ped about to be run over steps,
//! dives, raises its hands or shakes a fist. Ported from the 1.0 US bytes (gta-reversed rewrote the geometry; the
//! bytes win). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CEvent::GetEventType` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlayerPed::AnnoyPlayerPed(bool)` (0x60A040, `ret 4`)
    fn player_ped_annoy_player_ped(p: GPtr<u8>, b: u32) = thiscall 0x60A040 => unported;
    /// `CPlaceable::SetHeading(float)` (0x43E0C0, `ret 4`)
    fn placeable_set_heading(e: GPtr<u8>, h: u32) = thiscall 0x43E0C0 => unported;
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0, `ret 4`)
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::LookAt(...)` (0x618970, `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: u32, torso: u32,
                  speed: u32, blend: i32, prio: i32, b: u32) = thiscall 0x618970 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxCorners(float z, CEntity&, CVector* corners)` (0x5F3650)
    fn geometry_compute_entity_bbox_corners(z: u32, e: GPtr<u8>, corners: GPtr<u8>) = cdecl 0x5F3650 => unported;
    /// `CPedGeometryAnalyser::ComputeEntityBoundingBoxPlanes(float z, CEntity&, CVector* normals, float* ds)` (0x5F3660)
    fn geometry_compute_entity_bbox_planes(z: u32, e: GPtr<u8>, normals: GPtr<u8>, ds: GPtr<u8>) = cdecl 0x5F3660 => unported;
    /// `CPedGeometryAnalyser::ComputeMoveDirToAvoidEntity(const CPed&, CEntity&, CVector& out)` (0x5F3730)
    fn geometry_compute_move_dir_to_avoid_entity(ped: GPtr<u8>, e: GPtr<u8>, out: GPtr<u8>) = cdecl 0x5F3730 => unported;
    /// `CGeneral::GetRandomNumberInRange<float>(min, max)` (0x41BD90; draws its own rand, ST0)
    fn general_random_float_in_range(min: f32, max: f32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CGeneral::GetRandomNumberInRange<int32>(min, max)` (0x407180; draws its own rand)
    fn general_random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(x1, y1, x2, y2)` (0x53CBE0, ST0)
    fn general_get_radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn general_limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence` (0x632BD0)
    fn sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10, `ret 4`)
    fn sequence_add_task(t: GPtr<u8>, task: GPtr<u8>) = thiscall 0x632D10 => unported;
    fn shake_fist_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x690B80 => unported;
    fn hands_up_ctor(t: GPtr<u8>, time: i32) -> GPtr<u8> = thiscall 0x48E970 => unported;
    fn evasive_step_ctor(t: GPtr<u8>, e: GPtr<u8>, dir: GPtr<u8>) -> GPtr<u8> = thiscall 0x6532D0 => unported;
    fn evasive_dive_and_get_up_ctor(t: GPtr<u8>, veh: GPtr<u8>, time: i32, dir: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x6536B0 => unported;
    fn achieve_heading_ctor(t: GPtr<u8>, heading: u32, rate: u32, tol: u32) -> GPtr<u8> = thiscall 0x667E20 => unported;
    fn flee_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, b: u32, d: u32, time: u32, period: u32, f: u32) -> GPtr<u8> = thiscall 0x65B930 => unported;
}

/// `g_ikChainMan`
const IK_CHAIN_MANAGER: usize = 0xC15448;
/// The two look-at names ("CompVhclPotCollResp", "CompVehiclePotCollResp")
const S_PLAYER_LOOK: u32 = 0x85BAB8;
const S_PED_LOOK: u32 = 0x85BA9C;
const K_ZERO: usize = 0x858B50;
const K_HALF: usize = 0x858B8C;
const K_ONE: usize = 0x858624;
/// 3.0, 50.0, 2.0, 0.3, 0.175, 0.05, 0.7
const K_PLAYER_NEAR: usize = 0x858B3C;
const K_SPEED_SCALE: usize = 0x858B40;
const K_PED_SCALE: usize = 0x86C8C8;
const K_FAST: usize = 0x86C8D0;
const K_MARGIN_FAST: usize = 0x85BAB4;
const K_SLOW: usize = 0x86C8CC;
const K_MARGIN_SLOW: usize = 0x858CB0;
const FLEE_ENT_DIST: usize = 0xC18CE4;
const FLEE_ENT_B: usize = 0x86F664;
const FLEE_ENT_C: usize = 0x86F668;
const HEADING_A: usize = 0x86FC7C;
const HEADING_B: usize = 0x86FC80;

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn gu(a: usize) -> u32 {
    g::<u32>(a).read()
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

fn set(this: GPtr<u8>, t: GPtr<u8>) {
    wr::<GPtr<u8>>(this, 0x24, t);
}

/// `LimitRadianAngle(GetRadianAngleBetweenPoints(vehicle.xy, ped.xy))`, stored as f32 (the original's `fstp`).
fn heading_to_vehicle(this: GPtr<u8>, veh: GPtr<u8>) -> f32 {
    let pp = pos_ptr(ped(this));
    let vp = pos_ptr(veh);
    let a = f32_of(X87::from_f64_exact(general_get_radian_angle_between_points(
        rd::<u32>(vp, 0), rd::<u32>(vp, 4), rd::<u32>(pp, 0), rd::<u32>(pp, 4),
    )));
    f32_of(X87::from_f64_exact(general_limit_radian_angle(a.to_bits())))
}

fn x87_abs(v: X87) -> X87 {
    v.fabs()
}

/// `CEventHandler::ComputeVehiclePotentialCollisionResponse(CEventPotentialGetRunOver*, CTask*, CTask*)` (0x4C0BD0,
/// thiscall, `ret 0xC`). Event +0x14 the vehicle; nothing without it or when the ped is in a vehicle.
/// - `vehInFront` = (vehicle − ped)·pedForward > 0 (x87). The vehicle's bounding corners at the ped's height
///   (`ComputeEntityBoundingBoxCorners` into A[4]), its planes (`ComputeEntityBoundingBoxPlanes`, unused here) and the
///   avoid direction (`ComputeMoveDirToAvoidEntity` → `dir`). The original first runs the `eh vector constructor
///   iterator` with the no-op `CVector` constructor over two 4-vector stack arrays (no effect; not reproduced).
/// - The front edge's midpoint: `(A0 + A3) / 2` when `speedAlong = forward·moveSpeed > 0`, else `(A1 + A2) / 2`;
///   `along` / `side` = (ped − midpoint) projected on the vehicle's forward / right axes (f32).
/// - Player: `|along| < 3`, `|along| > |speedAlong · 50|`, player data +0x14 < 1, not looking already →
///   `AnnoyPlayerPed(false)`, `LookAt("CompVhclPotCollResp", ped, vehicle, 1500, -1, NULL, true, 0.25, 500, 3,
///   false)`, and with no weapon or weapon 5 / 0x16 / 0x17 / 0x1C, +0x30 = `CTaskSimpleShakeFist`.
/// - Other peds, `|along| < |2 · speedAlong · 50|`: `r = rand(0, 1)`. `|speedAlong| > 0.3`: margin 0.175 and the
///   thresholds (0.2, 0.1, 0.05) (nothing more when the margin is not above 0); else `|speedAlong| > 0.05`: margin 0.7,
///   thresholds (0.1, 0.05, 0.02); else nothing.
///   - `vehInFront`: `halfWidth(model bbox max.x) − margin < |side|`: `r < t0 / 2` → the target headings (+0x55C) =
///     the heading to the vehicle and `CTaskSimpleShakeFist`, else `CTaskComplexEvasiveStep(vehicle, &dir)`. Not
///     below: `r < t0` → both headings (+0x558 / +0x55C) and `SetHeading` = that heading,
///     `CTaskSimpleHandsUp(3000)`; else `CTaskComplexEvasiveDiveAndGetUp(vehicle, 0, &dir, true)`.
///   - Not in front: `r > t1` and the vehicle's +0x514 == 0 → NULL; else `r > t2` → the evasive step, else the dive.
/// - Other peds farther along: a vehicle of status 0 with +0x514 set, a mission ped (+0x484 == 1) whose current
///   event (intelligence +0x40, else +0x3C) is not of type 0xC: `n = rand(0, 1000)`: below 25 for a non-cop →
///   `CTaskComplexFleeEntity(vehicle, false, <dist>, 750 + rand(0, 500), <defaults>)`; below 100 → a sequence of
///   `CTaskSimpleAchieveHeading(heading to the vehicle)` and a shaken fist; below 200 → the achieve-heading task and
///   `LookAt("CompVehiclePotCollResp", ped, vehicle, 1300, ...)`; below 400 → the look only.
#[hook(addr = 0x4c0bd0, conv = thiscall, name = "CEventHandler::ComputeVehiclePotentialCollisionResponse", expect = "64 A1 00 00 00 00",
       this = "CEventHandler", unverified = "intro-ai-5", route = "L2", hazards = "none", recs = "grove-walk",
       note = "the two no-op eh vector constructor calls over stack arrays are not reproduced")]
#[allow(clippy::too_many_lines)]
pub fn handler_compute_vehicle_potential_collision_response(this: GPtr<u8>, e: GPtr<u8>, _active: GPtr<u8>, _simplest: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(e, 0x14);
    if veh.is_null() || rd::<u32>(ped(this), 0x46C) & 0x100 != 0 {
        return;
    }
    let mi = g::<GPtr<u8>>(0xA9_B0C8 + 4 * (i32::from(rd::<i16>(veh, 0x22)) as usize)).read();
    let half_width = rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0xC);
    let p = ped(this);
    let pm = rd::<GPtr<u8>>(p, 0x14);
    let pp = pos_ptr(p);
    let vp = pos_ptr(veh);
    let dz = x(rd::<f32>(vp, 8)).fsub_m(rd::<f32>(pp, 8));
    let dy = x(rd::<f32>(vp, 4)).fsub_m(rd::<f32>(pp, 4));
    let dx = x(rd::<f32>(vp, 0)).fsub_m(rd::<f32>(pp, 0));
    let s = dy.fmul_m(rd::<f32>(pm, 0x14)).fadd(dz.fmul_m(rd::<f32>(pm, 0x18)));
    let veh_in_front = !s.fadd(dx.fmul_m(rd::<f32>(pm, 0x10))).fcom(x(gf(K_ZERO))).test_any(0x41);
    let mut corners = [0u32; 12];
    let mut normals = [0u32; 12];
    let mut ds = [0u32; 4];
    let mut dir = [0u32; 3];
    let cp: GPtr<u8> = GPtr::from_raw(corners.as_mut_ptr()).cast();
    let dirp: GPtr<u8> = GPtr::from_raw(dir.as_mut_ptr()).cast();
    geometry_compute_entity_bbox_corners(rd::<u32>(pos_ptr(ped(this)), 8), veh, cp);
    geometry_compute_entity_bbox_planes(rd::<u32>(pos_ptr(ped(this)), 8), veh, GPtr::from_raw(normals.as_mut_ptr()).cast(),
                                        GPtr::from_raw(ds.as_mut_ptr()).cast());
    geometry_compute_move_dir_to_avoid_entity(ped(this), veh, dirp);
    let c = |i: isize, k: isize| rd::<f32>(cp, 0xC * i + 4 * k);
    let half = gf(K_HALF);
    let mid = |a: isize, b: isize| {
        let zs = f32_of(x(c(a, 2)).fadd_m(c(b, 2)));
        [f32_of(x(c(a, 0)).fadd_m(c(b, 0)).fmul_m(half)), f32_of(x(c(a, 1)).fadd_m(c(b, 1)).fmul_m(half)), f32_of(x(zs).fmul_m(half))]
    };
    let m03 = mid(3, 0);
    let m12 = mid(2, 1);
    let vm = rd::<GPtr<u8>>(veh, 0x14);
    let sa = x(rd::<f32>(vm, 0x18)).fmul_m(rd::<f32>(veh, 0x4C))
        .fadd(x(rd::<f32>(vm, 0x14)).fmul_m(rd::<f32>(veh, 0x48)))
        .fadd(x(rd::<f32>(vm, 0x10)).fmul_m(rd::<f32>(veh, 0x44)));
    let speed_along = f32_of(sa);
    let front = if sa.fcom(x(gf(K_ZERO))).test_any(0x41) { m12 } else { m03 };
    let pp = pos_ptr(ped(this));
    let px = x(rd::<f32>(pp, 0)).fsub_m(front[0]);
    let py = x(rd::<f32>(pp, 4)).fsub_m(front[1]);
    let pz = x(rd::<f32>(pp, 8)).fsub_m(front[2]);
    let along = f32_of(pz.fmul_m(rd::<f32>(vm, 0x18)).fadd(py.fmul_m(rd::<f32>(vm, 0x14))).fadd(px.fmul_m(rd::<f32>(vm, 0x10))));
    let side = f32_of(pz.fmul_m(rd::<f32>(vm, 8)).fadd(py.fmul_m(rd::<f32>(vm, 4))).fadd(px.fmul_m(rd::<f32>(vm, 0))));

    if ped_is_player(ped(this)) != 0 {
        let p = ped(this);
        let a_along = x87_abs(x(along));
        if a_along.fcom(x(gf(K_PLAYER_NEAR))).test_parity(0x05) {
            return;
        }
        let lim = x87_abs(x(speed_along).fmul_m(gf(K_SPEED_SCALE)));
        if a_along.fcom(lim).test_any(0x41) {
            return;
        }
        if x(rd::<f32>(rd::<GPtr<u8>>(p, 0x480), 0x14)).fcom(x(gf(K_ONE))).test_parity(0x05) {
            return;
        }
        if ik_is_looking(GPtr::from_addr(IK_CHAIN_MANAGER), p) != 0 {
            return;
        }
        player_ped_annoy_player_ped(p, 0);
        ik_look_at(GPtr::from_addr(IK_CHAIN_MANAGER), S_PLAYER_LOOK, ped(this), veh, 1500, -1, 0, 1, 0x3E80_0000, 500, 3, 0);
        let pd = ped(this);
        let w = rd::<u32>(pd, 0x5A0 + 0x1C * isize::from(rd::<i8>(pd, 0x718)));
        if [0, 5, 0x16, 0x17, 0x1C].contains(&w) {
            wr::<GPtr<u8>>(this, 0x30, new_task(0x10, shake_fist_ctor));
        }
        return;
    }
    let t = x87_abs(x(gf(K_PED_SCALE)).fmul_m(speed_along).fmul_m(gf(K_SPEED_SCALE)));
    if !x87_abs(x(along)).fcom(t).test_parity(0x05) {
        let r = X87::from_f64_exact(general_random_float_in_range(0.0, 1.0));
        let a_speed = x87_abs(x(speed_along));
        let (margin, t0, t1, t2);
        if !a_speed.fcom(x(gf(K_FAST))).test_any(0x41) {
            margin = x(gf(K_MARGIN_FAST));
            (t2, t1, t0) = (0.05f32, 0.1f32, 0.2f32);
            if x(gf(K_MARGIN_FAST)).fcom(x(gf(K_ZERO))).test_any(0x41) {
                return;
            }
        } else if a_speed.fcom(x(gf(K_SLOW))).test_any(0x41) {
            return;
        } else {
            margin = x(gf(K_MARGIN_SLOW));
            (t2, t1, t0) = (0.02f32, 0.05f32, 0.1f32);
        }
        if veh_in_front {
            let a_side = x87_abs(x(side));
            if !x(half_width).fsub(margin).fcom(a_side).test_parity(0x05) {
                if !r.fcom(x(t0).fmul_m(half)).test_parity(0x05) {
                    let h = heading_to_vehicle(this, veh);
                    wr::<f32>(ped(this), 0x55C, h);
                    set(this, new_task(0x10, shake_fist_ctor));
                } else {
                    set(this, new_task(0x1C, |t| evasive_step_ctor(t, veh, dirp)));
                }
            } else if !r.fcom(x(t0)).test_parity(0x05) {
                let h = heading_to_vehicle(this, veh);
                wr::<f32>(ped(this), 0x55C, h);
                wr::<f32>(ped(this), 0x558, h);
                placeable_set_heading(ped(this), h.to_bits());
                set(this, new_task(0x34, |t| hands_up_ctor(t, 3000)));
            } else {
                set(this, new_task(0x24, |t| evasive_dive_and_get_up_ctor(t, veh, 0, dirp, 1)));
            }
            return;
        }
        if !r.fcom(x(t1)).test_any(0x41) && rd::<u32>(veh, 0x514) == 0 {
            set(this, GPtr::NULL);
            return;
        }
        if !r.fcom(x(t2)).test_any(0x41) {
            set(this, new_task(0x1C, |t| evasive_step_ctor(t, veh, dirp)));
        } else {
            set(this, new_task(0x24, |t| evasive_dive_and_get_up_ctor(t, veh, 0, dirp, 1)));
        }
        return;
    }
    if rd::<u8>(veh, 0x36) & 0xF8 != 0 || rd::<u8>(ped(this), 0x484) != 1 {
        return;
    }
    let intel = rd::<GPtr<u8>>(ped(this), 0x47C);
    let mut ev = rd::<GPtr<u8>>(intel, 0x40);
    if ev.is_null() {
        ev = rd::<GPtr<u8>>(intel, 0x3C);
    }
    if rd::<u32>(veh, 0x514) == 0 {
        return;
    }
    if !ev.is_null() && v_event_type(ev) == 0xC {
        return;
    }
    let n = general_random_int_in_range(0, 1000);
    if n < 25 && rd::<i32>(ped(this), 0x598) != 6 {
        let time = general_random_int_in_range(0, 500) + 750;
        set(this, new_task(0x3C, |t| flee_entity_ctor(t, veh, 0, gu(FLEE_ENT_DIST), time as u32, gu(FLEE_ENT_B), gu(FLEE_ENT_C))));
        return;
    }
    if n < 100 {
        let seq = new_task(0x40, sequence_ctor);
        set(this, seq);
        let h = heading_to_vehicle(this, veh);
        sequence_add_task(seq, new_task(0x18, |t| achieve_heading_ctor(t, h.to_bits(), gu(HEADING_A), gu(HEADING_B))));
        sequence_add_task(seq, new_task(0x10, shake_fist_ctor));
        return;
    }
    if n < 200 {
        let h = heading_to_vehicle(this, veh);
        set(this, new_task(0x18, |t| achieve_heading_ctor(t, h.to_bits(), gu(HEADING_A), gu(HEADING_B))));
        ik_look_at(GPtr::from_addr(IK_CHAIN_MANAGER), S_PED_LOOK, ped(this), veh, 1300, -1, 0, 1, 0x3E80_0000, 500, 3, 0);
        return;
    }
    if n < 400 {
        ik_look_at(GPtr::from_addr(IK_CHAIN_MANAGER), S_PED_LOOK, ped(this), veh, 1300, -1, 0, 1, 0x3E80_0000, 500, 3, 0);
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_HANDLER_COMPUTE_VEHICLE_POTENTIAL_COLLISION_RESPONSE];
