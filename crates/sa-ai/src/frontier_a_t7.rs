//! FRONTIER-2 turn 7, lane A (batch `frontier2-a10`): AI frontier entries ported from the 1.0 US bytes (gta-reversed
//! read as an aid; where it differs the bytes win, noted per function).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::{
    compute_closest_surface_point_ped, ped_groups_get_peds_group, ped_replace_weapon_when_exiting_vehicle,
    bbox_planes, compute_entity_hit_side_ped, event_group_add, go_to_point_ctor, history_get_current_event,
    scripted_2d_effects_get_index,
    task_timer_is_out_of_time, tm_find_active_task_by_type, tm_get_task_secondary, tm_set_task_secondary,
    throw_projectile_ctor, use_gun_clear_anim, use_gun_control_gun, use_gun_ctor,
};
use sa_core::bindings::{
    get_weapon_info, intel_get_task_use_gun, matrix_set_rotate_z_only, ped_is_alive, tm_get_simplest_active_task,
    vector_difference, vector_magnitude_sqr, vector_normalise, vector_sub,
};
use sa_pop::bindings::get_position;
use crate::common::{g, rd, wr, x, xi, X87};
use crate::fall::fild_u32;
use crate::kill_ped_on_foot_armed::current_weapon_info;
use sa_core::bindings::{general_get_radian_angle_between_points, general_limit_radian_angle};
use sa_anim::bindings::ik_abort_look_at;
use sa_core::crt::rand_at;
use sa_core::field::gf;
use sa_math::fpu::{fcom_any, fcom_ne, fcom_parity, ge_or_nan, gt, le_or_nan, lt, st0};
use sa_math::vector::CVector;
use sa_pop::bindings::{get_look_behind_for_car, pad_get_look_left, pad_get_look_right};
use sa_stream::bindings::{calculate_ground_height, ped_is_player};
use sa_world::bindings::{
    achieve_heading_ctor, camera_get_look_direction, car_enter_exit_compute_passenger_index_from_car_door,
    entity_clean_up_old_reference, follow_node_route_ctor, fx_kill, get_pad_from_player, ik_is_looking, ik_look_at,
    membership_get_leader, ped_say, ped_set_ped_state, event_damage_ctor, event_damage_dtor, fire_extinguish,
    duck_ctor, find_player_wanted, get_task_swim, intel_get_task_duck, kill_ped_on_foot_ctor, stand_still_ctor,
    intel_get_task_throw, vector_ctor_iterator, vector_ne, compute_target_door_to_enter_as_passenger,
    enter_car_as_driver_ctor, enter_car_as_passenger_ctor, sequence_add_task, sequence_ctor, simple_car_drive_ctor,
    task_turn_to_face_ctor, turn_to_face_entity_ctor,
    max_f, min_f, pad_get_accelerate, pad_get_brake,
    pad_get_exit_vehicle, pad_get_ped_walk_left_right, pad_get_ped_walk_up_down, pad_get_target,
    ped_remove_weapon_when_entering_vehicle, physical_apply_move_force_5429f0,
};
use sa_world::physical::shared::ci_pow;
use sa_world::world::common::entity_pos;

orig! {
    /// `CAEPedAudioEntity::TurnOnJetPack()` (0x4E28A0)
    fn ped_audio_turn_on_jet_pack(a: GPtr<u8>) = thiscall 0x4E28A0 => unported;
    /// `CAEPedAudioEntity::UpdateJetPack(float thrustFwd, float thrustAngle)` (0x4E0EE0, `ret 8`)
    fn ped_audio_update_jet_pack(a: GPtr<u8>, fwd: f32, angle: u32) = thiscall 0x4E0EE0 => unported;
}

const TIME_MS: usize = 0xB7CB84;
const TIME_STEP: usize = 0xB7CB5C;
/// `TheCamera`.
const THE_CAMERA: usize = 0xB6F028;
/// The jet pack's force scale (`fmul [0x863984]` = 0.008).
const MAGIC: usize = 0x863984;
/// `THRUST_FULL` (0x8D2F3C, 0.6).
const THRUST_FULL: usize = 0x8D2F3C;

fn ts() -> f32 {
    gf(TIME_STEP)
}

/// `fstp dword [tmp]; fld dword [tmp]`: a value through an f32 stack slot.
fn r(v: X87) -> X87 {
    x(v.to_f32())
}

/// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0) with the three components ST0 holds in turn, each `fmul
/// [CTimer::ms_fTimeStep]` and stored (the vector the original builds on its stack).
fn apply_force_ts(ped: GPtr<u8>, fx: X87, fy: X87, fz: X87) {
    let t = ts();
    let v = CVector { x: fx.fmul_m(t).to_f32(), y: fy.fmul_m(t).to_f32(), z: fz.fmul_m(t).to_f32() };
    physical_apply_move_force_5429f0(ped, v);
}

// ------------------------------------------------------------------------------------------- jet pack

/// The ped's `bIsStanding` (+0x46C bit 0).
fn is_standing(ped: GPtr<u8>) -> bool {
    rd::<u8>(ped, 0x46C) & 1 != 0
}

/// The thrust angle's interpolation shared by both turning paths (0x67E938 / 0x67EAF3): `p = pow(0.9 [0x8D2F54],
/// timeStep)`, `+0x14 = (1 - p) * up_down * 1.3089969 [0x8D2F48] + p * +0x14`.
fn jet_pack_lerp_thrust_angle(this: GPtr<u8>, up_down: f32) {
    let p = ci_pow(x(gf(0x8D2F54)), x(ts()));
    let a = x(gf(0x858624)).fsub(p).fmul_m(up_down).fmul_m(gf(0x8D2F48));
    let b = p.fmul_m(rd::<f32>(this, 0x14));
    wr::<f32>(this, 0x14, a.fadd(b).to_f32());
}

/// `+0x10 -= timeStep * 0.1`, kept at or above -1 (0x67E976 / 0x67EDFE).
fn jet_pack_strafe_left(this: GPtr<u8>) {
    let v = x(ts()).fmul_m(gf(0x858B1C)).fsubr_m(rd::<f32>(this, 0x10));
    let lo = gf(0x858C1C);
    let v = if le_or_nan(x(lo), v) { v } else { x(lo) };
    wr::<f32>(this, 0x10, v.to_f32());
}

/// `+0x10 += timeStep * 0.1`, kept at or below 1 (0x67E9B5 / 0x67EE48).
fn jet_pack_strafe_right(this: GPtr<u8>) {
    let v = x(ts()).fmul_m(gf(0x858B1C)).fadd_m(rd::<f32>(this, 0x10));
    let hi = gf(0x858624);
    let v = if ge_or_nan(x(hi), v) { v } else { x(hi) };
    wr::<f32>(this, 0x10, v.to_f32());
}

/// The strafe's decay towards 0 by `timeStep * 0.05` without passing it (0x67ED69; NaN stays).
fn jet_pack_strafe_decay(this: GPtr<u8>) {
    let s = rd::<f32>(this, 0x10);
    let zero = gf(0x858B50);
    let step = x(ts()).fmul_m(gf(0x858C28));
    if gt(x(s), x(zero)) {
        let v = step.fsubr_m(s);
        let v = if le_or_nan(x(zero), v) { v } else { x(zero) };
        wr::<f32>(this, 0x10, v.to_f32());
    } else if !ge_or_nan(x(s), x(zero)) {
        let v = step.fadd_m(s);
        let v = if ge_or_nan(x(zero), v) { v } else { x(zero) };
        wr::<f32>(this, 0x10, v.to_f32());
    }
}

/// The pull towards the targeted object (0x67EB90): with a target (+0x71C) and not standing, `d` = ped - target
/// position; when the 2D distance is below `|dz| * 0.2 + 3`, the move force `(d.xy / max(0.1, dist), 0) * ((1 -
/// dist / limit) * 3) * 0.008 * 0.6 * mass * timeStep`. gta-reversed divides by `min(0.1, dist)` and scales by the
/// pad's left/right; the bytes use `max` and `dist / limit`.
fn jet_pack_pull_to_target(ped: GPtr<u8>) {
    let t = rd::<GPtr<u8>>(ped, 0x71C);
    if t.is_null() || is_standing(ped) {
        return;
    }
    let (tp, pp) = (entity_pos(t), entity_pos(ped));
    let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(tp, 0)).to_f32();
    let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(tp, 4));
    let lim = x(rd::<f32>(pp, 8)).fsub_m(rd::<f32>(tp, 8)).fabs().fmul_m(gf(0x858CC4)).fadd_m(gf(0x858B3C)).to_f32();
    let dist = x(dx).fmul_m(dx).fadd(dy.fmul(dy)).fsqrt();
    let distf = dist.to_f32();
    if ge_or_nan(dist, x(lim)) {
        return;
    }
    let min = gf(0x858B1C);
    let m = if le_or_nan(x(min), x(distf)) { x(distf) } else { x(min) };
    let inv = m.fdivr_m(gf(0x858624));
    let x0 = x(dx).fmul(inv);
    let ys = dy.fmul(inv).to_f32();
    let zs = inv.fmul_m(gf(0x858B50)).to_f32();
    let s = x(distf).fdiv_m(lim).fsubr_m(gf(0x858624)).fmul_m(gf(0x858B3C));
    let sf = s.to_f32();
    let fx = x0.fmul(s);
    let fy = x(ys).fmul_m(sf);
    let fz = x(sf).fmul_m(zs).to_f32();
    let (mg, full) = (gf(MAGIC), gf(THRUST_FULL));
    let x2 = r(r(fx.fmul_m(mg)).fmul_m(full));
    let y2 = r(fy.fmul_m(mg).fmul_m(full));
    let z2 = x(fz).fmul_m(mg).fmul_m(full);
    let mass = rd::<f32>(ped, 0x8C);
    apply_force_ts(ped, r(x2.fmul_m(mass)), r(y2.fmul_m(mass)), z2.fmul_m(mass));
}

/// `CTaskSimpleJetPack::ProcessControlInput(CPlayerPed*)` (0x67E7B0, `ret 4`; the `mov eax, [edi+0x71C]` at
/// 0x67EB8A runs in the protection fragment 0x4075E5). `ud` / `lr` = the pad's walk up-down / left-right.
/// * Instant-hit arm-aimed weapon (fire type 1, flag bit 1) and a primary task (+0x47C → +0x14) of type 0
///   (player on foot): its `ProcessPlayerWeapon(ped)`.
/// * Standing, move state below 4 and `GetExitVehicle`: `DropJetPack(ped)`, done.
/// * `mag = sqrt(ud² + lr²) / 60`. Mouse 3rd person (0xB6EC2E) and not standing: the heading is the active
///   camera's front as `atan2(-x, y)`, both negated unless the look direction is 3 (gta-reversed subtracts pi);
///   the thrust angle interpolates towards `ud / 128`; the strafe steps by `lr / 128`'s sign (±0.1, clamped to
///   ±1) or decays by 0.05 (max / min with 0). Otherwise, without target, standing or `GetTarget`: `lr / 128`
///   (when non-zero) turns the heading (`* timeStep * -0.05`, wrapped into [-pi, pi]) and the thrust angle
///   interpolates; with one of them, `mag > 0` gives the heading `LimitRadianAngle(angle(0, 0, -lr, ud) -
///   0xB6F178)` and the angle `1.3089969 * mag * -0.3515625`, else the player data's move blend ratio is 0.
/// * The pull towards the targeted object ([`jet_pack_pull_to_target`]).
/// * Mouse 3rd person and not standing: thrust stop off, forward thrust -1 (brake), 1 (accelerate) or
///   `ud / 128 != 0`, strafe decay. Otherwise look behind sets thrust stop, forward 0 and strafe 0; else thrust stop
///   off, forward -1 (brake) / accelerate (gta-reversed also counts the pad's up-down), look left / right without
///   `GetTarget` step the strafe, else it decays.
/// * Move blend ratio (player data +0x14): 0 unless `mag > 0`; then `mag` when `mag - ratio <= timeStep * 0.07`,
///   else `ratio + timeStep * 0.07`.
#[hook(addr = 0x67E7B0, conv = thiscall, name = "CTaskSimpleJetPack::ProcessControlInput", expect = "83 EC 24 53 55 56 57",
       this = "CTaskSimpleJetPack",
       unverified = "frontier2-a10", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "pow through ci_pow (orig::x87_binary 0x822130); FPATAN; one load runs in the protection fragment 0x4075E5")]
pub fn jet_pack_process_control_input(this: GPtr<u8>, ped: GPtr<u8>) {
    let pad = get_pad_from_player(ped);
    let wi = current_weapon_info(ped);
    if rd::<u32>(wi, 0) == 1 && (rd::<u32>(wi, 0x18) >> 1) & 1 != 0 {
        let t = rd::<GPtr<u8>>(rd::<GPtr<u8>>(ped, 0x47C), 0x14);
        if !t.is_null() && crate::common::task_get_type(t) == 0 {
            crate::player_on_foot::on_foot_process_player_weapon_port(t, ped);
        }
    }
    if is_standing(ped) && rd::<i32>(ped, 0x534) < 4 && pad_get_exit_vehicle(pad) != 0 {
        crate::frontier_a::jet_pack_drop_jet_pack(this, ped);
        return;
    }
    let ud = xi(i32::from(pad_get_ped_walk_up_down(pad))).to_f32();
    let lr = xi(i32::from(pad_get_ped_walk_left_right(pad))).to_f32();
    let mag = x(ud).fmul_m(ud).fadd(x(lr).fmul_m(lr)).fsqrt().fmul_m(gf(0x859044)).to_f32();
    let zero = gf(0x858B50);
    let mouse = || g::<u8>(0xB6EC2E).read() != 0;
    let k128 = gf(0x858B88);
    // [esp+0x24]: `ud` until a turning path stores `ud / 128` there.
    let mut ud_slot = ud;
    if mouse() && !is_standing(ped) {
        let lr_s = x(lr).fmul_m(k128).to_f32();
        let ud_s = x(ud).fmul_m(k128).to_f32();
        ud_slot = ud_s;
        let cam = THE_CAMERA + 0x304 + usize::from(g::<u8>(THE_CAMERA + 0x59).read()) * 0x238;
        let (fx, fy) = (gf(cam), gf(cam + 4));
        let (hx, hy) = if camera_get_look_direction(GPtr::from_addr(THE_CAMERA)) == 3 {
            (x(fx), x(fy))
        } else {
            (x(fx).fmul_m(gf(0x858C1C)), x(fy).fmul_m(gf(0x858C1C)))
        };
        wr::<f32>(ped, 0x55C, hx.fchs().fpatan(hy).to_f32());
        jet_pack_lerp_thrust_angle(this, ud_s);
        if lt(x(lr_s), x(zero)) {
            jet_pack_strafe_left(this);
        } else if gt(x(lr_s), x(zero)) {
            jet_pack_strafe_right(this);
        } else {
            let s = rd::<f32>(this, 0x10);
            let step = x(ts()).fmul_m(gf(0x858C28));
            if gt(x(s), x(zero)) {
                wr::<f32>(this, 0x10, max_f(0.0, step.fsubr_m(s).to_f32()).to_f32());
            } else if !ge_or_nan(x(s), x(zero)) {
                wr::<f32>(this, 0x10, min_f(0, step.fadd_m(s).to_f32().to_bits()).to_f32());
            }
        }
    } else if rd::<u32>(ped, 0x71C) == 0 && !is_standing(ped) && pad_get_target(pad) == 0 {
        let lr_s = x(lr).fmul_m(k128);
        let ud_s = x(ud).fmul_m(k128).to_f32();
        ud_slot = ud_s;
        if fcom_ne(lr_s, x(zero)) {
            let v = lr_s.fmul_m(ts()).fmul_m(gf(0x8D2F50)).fadd_m(rd::<f32>(ped, 0x55C));
            wr::<f32>(ped, 0x55C, v.to_f32());
            if gt(v, x(gf(0x858CB8))) {
                wr::<f32>(ped, 0x55C, v.fsub_m(gf(0x858CBC)).to_f32());
            } else if !ge_or_nan(v, x(gf(0x858CC0))) {
                wr::<f32>(ped, 0x55C, v.fadd_m(gf(0x858CBC)).to_f32());
            }
        }
        jet_pack_lerp_thrust_angle(this, ud_s);
    } else if gt(x(mag), x(zero)) {
        let a = general_get_radian_angle_between_points(0.0, 0.0, x(lr).fchs().to_f32(), ud);
        let h = a.fsub_m(gf(0xB6F178)).to_f32();
        wr::<f32>(ped, 0x55C, st0(general_limit_radian_angle(h)).to_f32());
        wr::<f32>(this, 0x14, x(gf(0x8D2F48)).fmul_m(mag).fmul_m(gf(0x870638)).to_f32());
    } else {
        wr::<f32>(rd::<GPtr<u8>>(ped, 0x480), 0x14, 0.0);
    }

    jet_pack_pull_to_target(ped);

    if mouse() && !is_standing(ped) {
        wr::<u8>(this, 0xD, 0);
        let fwd = if pad_get_brake(pad) & 0xFFFF != 0 {
            0xFF
        } else if pad_get_accelerate(pad) & 0xFFFF != 0 || fcom_ne(x(ud_slot), x(zero)) {
            1
        } else {
            0
        };
        wr::<u8>(this, 0xE, fwd);
        jet_pack_strafe_decay(this);
    } else if get_look_behind_for_car(pad) != 0 {
        wr::<u8>(this, 0xD, 1);
        wr::<u8>(this, 0xE, 0);
        wr::<u32>(this, 0x10, 0);
    } else {
        wr::<u8>(this, 0xD, 0);
        let fwd = if pad_get_brake(pad) & 0xFFFF != 0 {
            0xFF
        } else {
            u8::from(pad_get_accelerate(pad) & 0xFFFF != 0)
        };
        wr::<u8>(this, 0xE, fwd);
        if pad_get_look_left(pad) != 0 && pad_get_target(pad) == 0 {
            jet_pack_strafe_left(this);
        } else if pad_get_look_right(pad) != 0 && pad_get_target(pad) == 0 {
            jet_pack_strafe_right(this);
        } else {
            jet_pack_strafe_decay(this);
        }
    }

    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if gt(x(mag), x(zero)) {
        let d = x(ts()).fmul_m(gf(0x858CA8));
        let br = rd::<f32>(pd, 0x14);
        if le_or_nan(x(mag).fsub_m(br), d) {
            wr::<f32>(pd, 0x14, mag);
        } else {
            wr::<f32>(pd, 0x14, d.fadd_m(br).to_f32());
        }
    } else {
        wr::<f32>(pd, 0x14, 0.0);
    }
}

/// One leg swing of [`jet_pack_process_thrust`] (0x67F49F / 0x67F544): along the matrix row at `row` (+0x10 forward,
/// +0 right), speed at `sp`, angle at `an`. `dot = s · row` (0 while standing or just stood, +0x46C bits 0 / 1),
/// `speed = (speed - dot * -0.2 - sin(angle) * timeStep * 0.01) * damp`, `angle += speed * timeStep` clamped to
/// ±pi/4 (0x8D2F58; the upper bound against the unrounded sum, the lower one against the stored value).
fn jet_pack_leg_swing(this: GPtr<u8>, ped: GPtr<u8>, s: [f32; 3], damp: X87, row: isize, sp: isize, an: isize) {
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let mut dot = x(s[2]).fmul_m(rd::<f32>(m, row + 8)).fadd(x(s[1]).fmul_m(rd::<f32>(m, row + 4)));
    dot = dot.fadd(x(s[0]).fmul_m(rd::<f32>(m, row)));
    if rd::<u32>(ped, 0x46C) & 3 != 0 {
        dot = x(gf(0x858B50));
    }
    let v = dot.fmul_m(gf(0x8D2F5C)).fsubr_m(rd::<f32>(this, sp));
    wr::<f32>(this, sp, v.to_f32());
    let a = rd::<f32>(this, an);
    let v = v.fsub(x(a).fsin().fmul_m(ts()).fmul_m(gf(0x8D2F60))).fmul(damp);
    wr::<f32>(this, sp, v.to_f32());
    let na = v.fmul_m(ts()).fadd_m(a);
    let naf = na.to_f32();
    wr::<f32>(this, an, naf);
    let max = gf(0x8D2F58);
    if gt(na, x(max)) {
        wr::<f32>(this, an, max);
    } else {
        let lo = x(max).fchs();
        if !ge_or_nan(x(naf), lo) {
            wr::<f32>(this, an, lo.to_f32());
        }
    }
}

/// `CTaskSimpleJetPack::ProcessThrust(CPed*)` (0x67EF20, `ret 4`). Move speed at ped +0x44, mass +0x8C; every force is
/// `CPhysical::ApplyMoveForce` with the original's f32 slots.
/// * Thrust stop (+0xD): `up * 0.008 * mass * timeStep`; then `sf = timeStep * 0.5 * 0.008`, times 0.75 when `sf²`
///   exceeds the speed², and when `sf²` still exceeds it the speed is zeroed, else the force `-speed / |speed| * sf *
///   mass` (gta-reversed has no mass).
/// * Otherwise `up * 0.008 * 0.8 (x 0.5 when +0xE < 0) * mass * timeStep`; with +0xE > 0 the thrust `-sin(angle) *
///   forward + cos(angle) * up`, its z clamped to 0.75 after the sum (gta-reversed clamps the forward part), times
///   0.008, the height factor (1 below `CalculateGroundHeight(2) + 100`, else `max(0, 1 - (z - h) * 0.05)`), 0.6,
///   mass, timeStep; a non-zero strafe (+0x10, NaN counts) adds `right * strafe * 0.008 * 0.3 * mass * timeStep`.
/// * The leg swings ([`jet_pack_leg_swing`]) from `s = (v - prev frame's v +0x30) + (v - v at entry)` and `damp =
///   pow(0.98, timeStep)`; twist (+0x20) times `damp`.
/// * `v *= pow(1 - |v| * 0.02, timeStep)`, also into +0x30; unless finished (+8) the ped's audio (+0x138)
///   `TurnOnJetPack` and `UpdateJetPack(+0xE, +0x14)`.
#[hook(addr = 0x67EF20, conv = thiscall, name = "CTaskSimpleJetPack::ProcessThrust", expect = "83 EC 30 53 55 56 57",
       this = "CTaskSimpleJetPack",
       unverified = "frontier2-a10", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "pow through ci_pow (orig::x87_binary 0x822130); FSIN / FCOS")]
pub fn jet_pack_process_thrust(this: GPtr<u8>, ped: GPtr<u8>) {
    let v = ped.byte_add(0x44);
    let prev = [rd::<f32>(v, 0), rd::<f32>(v, 4), rd::<f32>(v, 8)];
    let mg = gf(MAGIC);
    let mass = rd::<f32>(ped, 0x8C);
    let m = rd::<GPtr<u8>>(ped, 0x14);
    if rd::<u8>(this, 0xD) != 0 {
        let (ux, uy, uz) = (x(rd::<f32>(m, 0x20)).fmul_m(mg), x(rd::<f32>(m, 0x24)).fmul_m(mg), r(x(rd::<f32>(m, 0x28)).fmul_m(mg)));
        apply_force_ts(ped, r(ux.fmul_m(mass)), uy.fmul_m(mass), uz.fmul_m(mass));
        let (vx, vy, vz) = (rd::<f32>(v, 0), rd::<f32>(v, 4), rd::<f32>(v, 8));
        let sq = x(vx).fmul_m(vx).fadd(x(vy).fmul_m(vy)).fadd(x(vz).fmul_m(vz)).to_f32();
        let mut sf = x(ts()).fmul_m(gf(0x8D2F44)).fmul_m(mg);
        if gt(sf.fmul(sf), x(sq)) {
            sf = sf.fmul_m(gf(0x858F34));
        }
        if le_or_nan(sf.fmul(sf), x(sq)) {
            let neg = gf(0x858C1C);
            let (nx, ny, nz) = (x(vx).fmul_m(neg), x(vy).fmul_m(neg), x(vz).fmul_m(neg).to_f32());
            let inv = x(sq).fsqrt().fdivr_m(gf(0x858624));
            let invf = inv.to_f32();
            let a = inv.fmul(nx).to_f32();
            let b = ny.fmul_m(invf);
            let c = x(nz).fmul_m(invf).to_f32();
            let asf = x(a).fmul(sf).to_f32();
            let bsf = b.fmul(sf).to_f32();
            let csf = sf.fmul_m(c);
            let f = CVector {
                x: x(asf).fmul_m(mass).to_f32(),
                y: x(bsf).fmul_m(mass).to_f32(),
                z: csf.fmul_m(mass).to_f32(),
            };
            physical_apply_move_force_5429f0(ped, f);
        } else {
            for k in 0..3 {
                wr::<u32>(v, k * 4, 0);
            }
        }
    } else {
        let mut k = x(gf(0x8D2F38));
        if rd::<i8>(this, 0xE) < 0 {
            k = k.fmul_m(gf(0x858B8C));
        }
        let (ux, uy, uz) = (x(rd::<f32>(m, 0x20)).fmul_m(mg), x(rd::<f32>(m, 0x24)).fmul_m(mg), r(x(rd::<f32>(m, 0x28)).fmul_m(mg)));
        let (x1, y1, z1) = (r(ux.fmul(k)), uy.fmul(k), r(uz.fmul(k)));
        apply_force_ts(ped, r(x1.fmul_m(mass)), y1.fmul_m(mass), z1.fmul_m(mass));

        if rd::<i8>(this, 0xE) > 0 {
            let m = rd::<GPtr<u8>>(ped, 0x14);
            let ang = rd::<f32>(this, 0x14);
            let c = x(ang).fcos();
            let cx = c.fmul_m(rd::<f32>(m, 0x20));
            let cy = c.fmul_m(rd::<f32>(m, 0x24)).to_f32();
            let cz = c.fmul_m(rd::<f32>(m, 0x28)).to_f32();
            let s = x(ang).fsin().fchs();
            let fx = s.fmul_m(rd::<f32>(m, 0x10)).to_f32();
            let fy = s.fmul_m(rd::<f32>(m, 0x14)).to_f32();
            let fz = s.fmul_m(rd::<f32>(m, 0x18)).to_f32();
            let tx = x(fx).fadd(cx).to_f32();
            let ty = x(fy).fadd_m(cy).to_f32();
            let tz80 = x(fz).fadd_m(cz);
            let tz = if gt(tz80, x(gf(0x8703DC))) { gf(0x8703DC) } else { tz80.to_f32() };
            let cam = GPtr::from_addr(THE_CAMERA);
            let h = calculate_ground_height(cam, 2).fadd_m(gf(0x8703D8));
            let mult = if ge_or_nan(h, x(rd::<f32>(entity_pos(ped), 8))) {
                x(1.0)
            } else {
                let h2 = calculate_ground_height(cam, 2).fadd_m(gf(0x8703D8));
                let v = h2.fsubr_m(rd::<f32>(entity_pos(ped), 8)).fmul_m(gf(0x858C28)).fsubr_m(gf(0x858624));
                if le_or_nan(x(gf(0x858B50)), v) { v } else { x(gf(0x858B50)) }
            };
            let (txm, tym, tzm) = (x(tx).fmul_m(mg), x(ty).fmul_m(mg), r(x(tz).fmul_m(mg)));
            let full = gf(THRUST_FULL);
            let x2 = r(r(txm.fmul(mult)).fmul_m(full));
            let y2 = tym.fmul(mult).fmul_m(full);
            let z3 = r(tzm.fmul(mult)).fmul_m(full);
            apply_force_ts(ped, r(x2.fmul_m(mass)), r(y2.fmul_m(mass)), z3.fmul_m(mass));
        }

        let s = rd::<f32>(this, 0x10);
        if fcom_ne(x(s), x(gf(0x858B50))) {
            let m = rd::<GPtr<u8>>(ped, 0x14);
            let a = x(s).fmul_m(rd::<f32>(m, 0)).fmul_m(mg);
            let sy = x(s).fmul_m(rd::<f32>(m, 4)).to_f32();
            let sz = x(s).fmul_m(rd::<f32>(m, 8)).to_f32();
            let b = x(sy).fmul_m(mg);
            let c = x(sz).fmul_m(mg).to_f32();
            let st = gf(0x8D2F40);
            let a2 = r(a.fmul_m(st));
            let b2 = b.fmul_m(st);
            let c2 = x(c).fmul_m(st);
            apply_force_ts(ped, r(a2.fmul_m(mass)), r(b2.fmul_m(mass)), c2.fmul_m(mass));
        }
    }

    // The leg swings.
    let (vx, vy, vz) = (rd::<f32>(v, 0), rd::<f32>(v, 4), rd::<f32>(v, 8));
    let d1x = x(vx).fsub_m(prev[0]);
    let d1y = x(vy).fsub_m(prev[1]);
    let d1z = x(vz).fsub_m(prev[2]).to_f32();
    let d2x = x(vx).fsub_m(rd::<f32>(this, 0x30));
    let d2y = x(vy).fsub_m(rd::<f32>(this, 0x34)).to_f32();
    let d2z = x(vz).fsub_m(rd::<f32>(this, 0x38)).to_f32();
    let s = [d2x.fadd(d1x).to_f32(), x(d2y).fadd(d1y).to_f32(), x(d2z).fadd_m(d1z).to_f32()];
    let damp = ci_pow(x(gf(0x8D2F64)), x(ts()));
    jet_pack_leg_swing(this, ped, s, damp, 0x10, 0x24, 0x18);
    jet_pack_leg_swing(this, ped, s, damp, 0, 0x28, 0x1C);
    wr::<f32>(this, 0x20, damp.fmul_m(rd::<f32>(this, 0x20)).to_f32());

    let (vx, vy, vz) = (rd::<f32>(v, 0), rd::<f32>(v, 4), rd::<f32>(v, 8));
    let mag = x(vx).fmul_m(vx).fadd(x(vy).fmul_m(vy)).fadd(x(vz).fmul_m(vz)).fsqrt().to_f32();
    let base = x(mag).fmul_m(gf(0x8D2F4C)).fsubr_m(gf(0x858624));
    let p = ci_pow(base, x(ts()));
    let nv = [p.fmul_m(vx).to_f32(), p.fmul_m(vy).to_f32(), p.fmul_m(vz).to_f32()];
    for (k, c) in nv.iter().enumerate() {
        wr::<f32>(v, k as isize * 4, *c);
        wr::<f32>(this, 0x30 + k as isize * 4, *c);
    }
    if rd::<u8>(this, 8) == 0 {
        let audio = ped.byte_add(0x138);
        ped_audio_turn_on_jet_pack(audio);
        ped_audio_update_jet_pack(audio, xi(i32::from(rd::<i8>(this, 0xE))).to_f32(), rd::<u32>(this, 0x14));
    }
}

/// `CTaskSimpleJetPack::ProcessPed(CPed*)` (0x6801F0, `ret 4`, AL).
/// * Finished (+8): with weapons switched (+0xC) and a player, `ReplaceWeaponWhenExitingVehicle`; both thruster
///   effects killed (`StopJetPackEffect` inlined); AL = 1.
/// * Otherwise: weapons not switched → `RemoveWeaponWhenEnteringVehicle(1)` for a player and +0xC = 1 for any ped
///   (gta-reversed sets it for players only); +0x478 |= 0x100; `ProcessAnims(ped)`, and when it succeeds: health
///   (+0x540) at least 1 (or NaN) → a player's `ProcessControlInput`; else thrust stop / forward / angle cleared,
///   the strafe ±1 by `rand() & 1` (+1 when set) clamped to ±3, the heading +0x55C += `sin(time * 0.0015707964) *
///   timeStep * -0.05 * rand() / 32767` and the player data's (+0x480) move blend ratio 0. Then `ProcessThrust`,
///   and a forward thrust > 0 or `|strafe| > 0.1` clears `bIsStanding`. AL = 0.
#[hook(addr = 0x6801F0, conv = thiscall, name = "CTaskSimpleJetPack::ProcessPed", expect = "51 53 56 8B F1 8A 46 08",
       this = "CTaskSimpleJetPack",
       unverified = "frontier2-a10", route = "L2", hazards = "x87-transcendental,ax-al-return,heap-writes",
       note = "rand at 0x6802A9 and 0x680315; FSIN")]
pub fn jet_pack_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 8) != 0 {
        if rd::<u8>(this, 0xC) != 0 && ped_is_player(ped) != 0 {
            ped_replace_weapon_when_exiting_vehicle(ped);
        }
        for o in [0x64, 0x68] {
            let fx = rd::<GPtr<u8>>(this, o);
            if !fx.is_null() {
                fx_kill(fx);
                wr::<u32>(this, o, 0);
            }
        }
        return 1;
    }
    if rd::<u8>(this, 0xC) == 0 {
        if ped_is_player(ped) != 0 {
            ped_remove_weapon_when_entering_vehicle(ped, 1);
        }
        wr::<u8>(this, 0xC, 1);
    }
    wr::<u32>(ped, 0x478, rd::<u32>(ped, 0x478) | 0x100);
    if crate::gen_bodies::g_task_simple_jet_pack_process_anims_67b5c0(this.addr() as u32, ped.addr() as u32) as u8 == 0 {
        return 0;
    }
    if ge_or_nan(x(rd::<f32>(ped, 0x540)), x(gf(0x858624))) {
        if ped_is_player(ped) != 0 {
            jet_pack_process_control_input(this, ped);
        }
    } else {
        wr::<u8>(this, 0xD, 0);
        wr::<u8>(this, 0xE, 0);
        wr::<u32>(this, 0x14, 0);
        let one = gf(0x858624);
        let s = x(rd::<f32>(this, 0x10));
        let s = if rand_at(0x6802A9) & 1 != 0 { s.fadd_m(one) } else { s.fsub_m(one) };
        wr::<f32>(this, 0x10, s.to_f32());
        let s = rd::<f32>(this, 0x10);
        if gt(x(s), x(gf(0x858B3C))) {
            wr::<u32>(this, 0x10, 0x4040_0000);
        } else if !ge_or_nan(x(s), x(gf(0x859024))) {
            wr::<u32>(this, 0x10, 0xC040_0000);
        }
        let t = fild_u32(g::<i32>(TIME_MS).read()).fmul_m(gf(0x870640)).to_f32();
        let rn = xi(rand_at(0x680315)).fmul_m(gf(0x858C7C));
        let pd = rd::<GPtr<u8>>(ped, 0x480);
        let a = x(t).fsin().fmul_m(ts()).fmul_m(gf(0x8D2F50)).fmul(rn).fadd_m(rd::<f32>(ped, 0x55C));
        wr::<f32>(ped, 0x55C, a.to_f32());
        if !pd.is_null() {
            wr::<u32>(pd, 0x14, 0);
        }
    }
    jet_pack_process_thrust(this, ped);
    if rd::<i8>(this, 0xE) > 0 || gt(x(rd::<f32>(this, 0x10)).fabs(), x(gf(0x858B1C))) {
        wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & !1);
    }
    0
}

// ------------------------------------------------------------------------------------------- AI virtuals

/// `CTaskSimpleStealthKill::ProcessPed(CPed*)` (0x62E540, `ret 4`, AL). Finished (+0x14) or no target (+0xC): 1.
/// Without an animation (+0x18): a non-attacker (+8 clear) gets `SetPedState(0x38)`; `ManageAnim(ped)`; still none with
/// +0x15 set: 0. Then +0x528 |= 8 and `d` = target - ped position (dy through an f32 slot). The attacker: `e = |d| -
/// 1` (z², x², then y²); when `|e| > 0.02`, +0x4DC = `e`, or `timeStep * 0.05` when `e` is above it (NaN keeps `e`),
/// and +0x4D8 = 0; heading +0x55C = `atan2(-d.x, d.y)`. The victim: with no animation or animation id 0x15D the heading
/// `atan2(d.x, -d.y)` (both negated, as the bytes). AL = 0.
#[hook(addr = 0x62E540, conv = thiscall, name = "CTaskSimpleStealthKill::ProcessPed", expect = "83 EC 0C 56 8B F1 8A 46 14",
       this = "CTaskSimpleStealthKill",
       unverified = "frontier2-a10", route = "L2", hazards = "x87-transcendental,ax-al-return,heap-writes")]
pub fn stealth_kill_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x14) != 0 || rd::<u32>(this, 0xC) == 0 {
        return 1;
    }
    if rd::<u32>(this, 0x18) == 0 {
        if rd::<u8>(this, 8) == 0 {
            ped_set_ped_state(ped, 0x38);
        }
        crate::gen_bodies::g_task_simple_stealth_kill_manage_anim_6296d0(this.addr() as u32, ped.addr() as u32);
        if rd::<u32>(this, 0x18) == 0 && rd::<u8>(this, 0x15) != 0 {
            return 0;
        }
    }
    wr::<u32>(ped, 0x528, rd::<u32>(ped, 0x528) | 8);
    let pp = entity_pos(ped);
    let tp = entity_pos(rd::<GPtr<u8>>(this, 0xC));
    let dx = x(rd::<f32>(tp, 0)).fsub_m(rd::<f32>(pp, 0));
    let dy = x(rd::<f32>(tp, 4)).fsub_m(rd::<f32>(pp, 4)).to_f32();
    let dz = x(rd::<f32>(tp, 8)).fsub_m(rd::<f32>(pp, 8));
    if rd::<u8>(this, 8) != 0 {
        let len = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(x(dy).fmul_m(dy)).fsqrt();
        let e = len.fsub_m(gf(0x86D630));
        if gt(e.fabs(), x(gf(0x86D634))) {
            let m = x(ts()).fmul_m(gf(0x86D638)).to_f32();
            let v = if le_or_nan(e, x(m)) { e } else { x(m) };
            wr::<f32>(ped, 0x4DC, v.to_f32());
            wr::<u32>(ped, 0x4D8, 0);
        }
        wr::<f32>(ped, 0x55C, dx.fchs().fpatan(x(dy)).to_f32());
        return 0;
    }
    let anim = rd::<GPtr<u8>>(this, 0x18);
    if anim.is_null() || rd::<u16>(anim, 0x2C) == 0x15D {
        let neg = gf(0x858C1C);
        let (nx, ny) = (dx.fmul_m(neg), x(dy).fmul_m(neg));
        wr::<f32>(ped, 0x55C, nx.fchs().fpatan(ny).to_f32());
    }
    0
}

orig! {
    /// `CTaskComplexDestroyCarMelee::CreateSubTask(int32 type, CPed*)` (0x628A70, `ret 8`)
    fn destroy_car_melee_create_sub_task(t: u32, ty: u32, ped: u32) -> u32 = thiscall 0x628A70 => crate::gen_subtasks::g_task_complex_destroy_car_melee_create_sub_task_628a70;
}

/// Calls the generated body of `CTaskComplexDestroyCarMelee::CalculateSearchPositionAndRanges(CPed*)` (0x6289F0).
fn destroy_car_melee_search(this: GPtr<u8>, ped: GPtr<u8>) {
    crate::gen_bodies::g_task_complex_destroy_car_melee_calculate_search_position_and_ranges_6289f0(
        this.addr() as u32, ped.addr() as u32);
}

/// [`destroy_car_melee_create_sub_task`] with pointers.
fn destroy_car_melee_sub(this: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> {
    GPtr::from_addr(destroy_car_melee_create_sub_task(this.addr() as u32, ty, ped.addr() as u32) as usize)
}

/// `CTaskComplexDestroyCarMelee::CreateNextSubTask(CPed*)` (0x62DC20, `ret 4`). Aborting (+0xC): NULL. By the
/// sub-task's type: 0x387 → `CalculateSearchPositionAndRanges`, then `(+0x20)² > |car - ped|²` → 0x3FB, else 0x38B
/// (0xCA in a car, +0x46C bit 21); 0x38B → 0x3FB; 0xCA / 0x3FB → on foot: `CalculateSearchPositionAndRanges` and
/// 0x38B; in a car: 0x516 once more than 3000 ms passed since +0x30 (not -1), else 0xCA; 0x516 and others → NULL.
#[hook(addr = 0x62DC20, conv = thiscall, name = "CTaskComplexDestroyCarMelee::CreateNextSubTask", expect = "83 EC 0C 56 8B F1 8A 46 0C",
       this = "CTaskComplexDestroyCarMelee",
       unverified = "frontier2-a10", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the original writes +0x20 into its own ped argument slot")]
pub fn destroy_car_melee_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(this, 0xC) != 0 {
        return GPtr::NULL;
    }
    let in_car = |p: GPtr<u8>| rd::<u32>(p, 0x46C) & 0x20_0000 != 0;
    match crate::common::task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0x387 => {
            destroy_car_melee_search(this, ped);
            let r = rd::<f32>(this, 0x20);
            let mut d = [0f32; 3];
            let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
            let v = vector_sub(dp, entity_pos(rd::<GPtr<u8>>(this, 0x10)), entity_pos(ped));
            let m = vector_magnitude_sqr(v);
            if gt(x(r).fmul_m(r), m) {
                destroy_car_melee_sub(this, 0x3FB, ped)
            } else if in_car(ped) {
                destroy_car_melee_sub(this, 0xCA, ped)
            } else {
                destroy_car_melee_sub(this, 0x38B, ped)
            }
        }
        0x38B => destroy_car_melee_sub(this, 0x3FB, ped),
        0xCA | 0x3FB => {
            if !in_car(ped) {
                destroy_car_melee_search(this, ped);
                return destroy_car_melee_sub(this, 0x38B, ped);
            }
            let t = rd::<u32>(this, 0x30);
            if t != u32::MAX && g::<u32>(TIME_MS).read().wrapping_sub(t) > 3000 {
                destroy_car_melee_sub(this, 0x516, ped)
            } else {
                destroy_car_melee_sub(this, 0xCA, ped)
            }
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexShuffleSeats::CreateNextSubTask(CPed*)` (0x63D420, `ret 4`). With +0x28 set: after 0x328 / 0x32B /
/// 0x32C → `CreateSubTask(0x330)`, after 0xC8 / 0x330 → 0x516. Otherwise after 0x32B / 0x32C / 0x330 / 0xC8 → 0x516;
/// after 0x328 the target door +0x14 becomes the one beside +0x10 (8 → 10, 9 → 11, 10 → 8, 11 → 9; other values
/// keep it): door 10 → 0x32C without a driver (+0x460), else 0x516; other doors → 0x32B when that passenger seat
/// (+0x464 + `ComputePassengerIndexFromCarDoor` * 4) is empty, else 0x516. Other types: NULL.
#[hook(addr = 0x63D420, conv = thiscall, name = "CTaskComplexShuffleSeats::CreateNextSubTask", expect = "56 8B F1 8A 46 28",
       unverified = "frontier2-a10", route = "L2", hazards = "heap-writes")]
pub fn shuffle_seats_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    use crate::frontier_a_subtasks::shuffle_seats_create_sub_task as sub;
    let ty = crate::common::task_get_type(rd::<GPtr<u8>>(this, 8));
    if rd::<u8>(this, 0x28) != 0 {
        return match ty {
            0x328 | 0x32B | 0x32C => sub(this, 0x330, ped),
            0xC8 | 0x330 => sub(this, 0x516, ped),
            _ => GPtr::NULL,
        };
    }
    match ty {
        0x32B | 0x32C | 0x330 | 0xC8 => sub(this, 0x516, ped),
        0x328 => {
            match rd::<i32>(this, 0x10) {
                8 => wr::<u32>(this, 0x14, 10),
                9 => wr::<u32>(this, 0x14, 11),
                10 => wr::<u32>(this, 0x14, 8),
                11 => wr::<u32>(this, 0x14, 9),
                _ => {}
            }
            let door = rd::<i32>(this, 0x14);
            let veh = rd::<GPtr<u8>>(this, 0xC);
            if door == 10 {
                if rd::<u32>(veh, 0x460) == 0 { sub(this, 0x32C, ped) } else { sub(this, 0x516, ped) }
            } else {
                let i = car_enter_exit_compute_passenger_index_from_car_door(veh, door);
                let seat = rd::<u32>(rd::<GPtr<u8>>(this, 0xC), 0x464 + i as isize * 4);
                if seat != 0 { sub(this, 0x516, ped) } else { sub(this, 0x32B, ped) }
            }
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexEnterCarAsPassengerWait::ControlSubTask(CPed*)` (0x640730, `ret 4`). Once (+0x20): with a ped to wait
/// for (+0x10) that is a player, or else (none) with a group leader that is a player, and the ped running task 0x327:
/// +0x20 = 1 and `Say(0x23 / 0x28, 0, 1.0, 0, 0, 0)`. With a car (+0xC) and a 0x387 sub-task: the closest surface
/// point `p` (`ComputeClosestSurfacePoint(ped, car)`; an uninitialised stack vector when not written); when `|p -
/// goal|² > 0.025` (goal = the sub-task's +0x10; z², y², x²) the sub-task's `GoToPoint(p, 1.5, [0x86FC88], 0)` when
/// the point was found, else `GoToPoint(car position, car col radius + 1.5, [0x86FC88], 0)`. Returns the sub-task.
#[hook(addr = 0x640730, conv = thiscall, name = "CTaskComplexEnterCarAsPassengerWait::ControlSubTask", expect = "83 EC 0C 56 8B F1 8A 46 20",
       unverified = "frontier2-a10", route = "L2", hazards = "heap-writes,uninit-locals",
       note = "the closest point is an uninitialised stack vector when ComputeClosestSurfacePoint does not write it (zeros here)")]
pub fn enter_car_as_passenger_wait_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(this, 0x20) == 0 {
        let wait_for = rd::<GPtr<u8>>(this, 0x10);
        let tm = || rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
        let say = if !wait_for.is_null() && ped_is_player(wait_for) != 0 {
            (!tm_find_active_task_by_type(tm(), 0x327).is_null()).then_some(0x23)
        } else if rd::<u8>(this, 0x20) == 0 && rd::<u32>(this, 0x10) == 0 {
            let group = GPtr::<u8>::from_addr(ped_groups_get_peds_group(ped.addr() as u32) as usize);
            let leader = if group.is_null() { GPtr::NULL } else { membership_get_leader(group.byte_add(8)) };
            (!leader.is_null() && ped_is_player(leader) != 0 && !tm_find_active_task_by_type(tm(), 0x327).is_null())
                .then_some(0x28)
        } else {
            None
        };
        if let Some(id) = say {
            wr::<u8>(this, 0x20, 1);
            ped_say(ped, id, 0, 1.0, 0, 0, 0);
        }
    }
    let car = rd::<GPtr<u8>>(this, 0xC);
    if !car.is_null() && crate::common::task_get_type(rd::<GPtr<u8>>(this, 8)) == 0x387 {
        let mut p = [0f32; 3];
        let pp: GPtr<u8> = GPtr::from_raw(p.as_mut_ptr()).cast();
        let found = compute_closest_surface_point_ped(ped, car, pp) as u8 != 0;
        let sub = rd::<GPtr<u8>>(this, 8);
        let dx = x(p[0]).fsub_m(rd::<f32>(sub, 0x10));
        let dy = x(p[1]).fsub_m(rd::<f32>(sub, 0x14));
        let dz = x(p[2]).fsub_m(rd::<f32>(sub, 0x18));
        let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
        if gt(d2, x(gf(0x859038))) {
            let ms = g::<u32>(0x86FC88).read();
            if found {
                crate::walk::gtpss_go_to_point(sub, pp, 0x3FC0_0000, ms, 0);
            } else {
                let car = rd::<GPtr<u8>>(this, 0xC);
                let mi = g::<GPtr<u8>>(0xA9B0C8 + rd::<i16>(car, 0x22) as isize as usize * 4).read();
                let r = x(rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x24)).fadd_m(gf(0x858CE8)).to_f32();
                crate::walk::gtpss_go_to_point(sub, entity_pos(car), r.to_bits(), ms, 0);
            }
        }
    }
    rd::<GPtr<u8>>(this, 8)
}

/// `CTaskInteriorGoToInfo::CreateNextSubTask(CPed*)` (0x675890, `ret 4`). No info (+0xC) or a sub-task other than
/// 0x38A / 0x384 (the type read twice): NULL. The ped's 2D distance² to the info's position (+4) at most 0.04: a
/// `CTaskSimpleAchieveHeading(LimitRadianAngle(angle(dir.x, dir.y, 0, 0)), 0.5, 0.2)` for the info's direction
/// (+0x10); farther (or NaN) a `CTaskComplexFollowNodeRoute(4, info position, 0.2, 3.0, 2.0, false, -1, true)`. A
/// failed task allocation: NULL.
#[hook(addr = 0x675890, conv = thiscall, name = "CTaskInteriorGoToInfo::CreateNextSubTask", expect = "64 A1 00 00 00 00 6A FF 68 86 5C 84 00",
       unverified = "frontier2-a10", route = "L2", hazards = "heap-writes",
       note = "SEH frame not modelled")]
pub fn interior_go_to_info_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(this, 0xC) == 0 {
        return GPtr::NULL;
    }
    if crate::common::task_get_type(rd::<GPtr<u8>>(this, 8)) != 0x38A
        && crate::common::task_get_type(rd::<GPtr<u8>>(this, 8)) != 0x384
    {
        return GPtr::NULL;
    }
    let info = rd::<GPtr<u8>>(this, 0xC);
    let mut pos = [rd::<u32>(info, 4), rd::<u32>(info, 8), rd::<u32>(info, 0xC)];
    let pp = entity_pos(ped);
    let dy = x(rd::<f32>(pp, 4)).fsub_m(f32::from_bits(pos[1]));
    let dx = x(rd::<f32>(pp, 0)).fsub_m(f32::from_bits(pos[0]));
    let d2 = dx.fmul(dx).fadd(dy.fmul(dy));
    if fcom_parity(d2, x(gf(0x863244)), 0x41) {
        let mem = crate::common::task_new(0x60);
        if mem.is_null() {
            return GPtr::NULL;
        }
        let r = GPtr::from_raw(pos.as_mut_ptr()).cast();
        return follow_node_route_ctor(mem, 4, r, 0x3E4C_CCCD, g::<u32>(0x86FCA4).read(), g::<u32>(0x86FCA8).read(), 0,
                                      u32::MAX, 1);
    }
    let a = general_get_radian_angle_between_points(rd::<f32>(info, 0x10), rd::<f32>(info, 0x14), 0.0, 0.0).to_f32();
    let h = st0(general_limit_radian_angle(a)).to_f32();
    let mem = crate::common::task_new(0x18);
    if mem.is_null() {
        return GPtr::NULL;
    }
    achieve_heading_ctor(mem, h, g::<u32>(0x86FC7C).read(), g::<u32>(0x86FC80).read())
}

/// `CTaskComplexWaitForBackup::CreateNextSubTask(CPed*)` (0x697F10, `ret 4`). By the sub-task's type: 0x2C0 → 0x386;
/// 0xCA → 0x2C0 when the ped has a vehicle (+0x58C) and is in it (+0x46C bit 8), else 0x386; 0x386 → 0x3FC when the
/// current weapon's fire type (skill 1) is 1, else 0x516; 0x3FC → 0x516 unless the backup ped (+0x14) is alive and the
/// timer (+0x18) not out of time, then 0x386 when `(+0xC)²` is at most the 2D distance² from the ped to it (NaN
/// too), else 0x516. Others: NULL.
#[hook(addr = 0x697F10, conv = thiscall, name = "CTaskComplexWaitForBackup::CreateNextSubTask", expect = "83 EC 0C 56 8B F1 8B 4E 08",
       unverified = "frontier2-a10", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the original writes +0xC into its own ped argument slot")]
pub fn wait_for_backup_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    use crate::frontier_a_subtasks::wait_for_backup_create_sub_task as sub;
    match crate::common::task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0x2C0 => sub(this, 0x386, ped),
        0xCA => {
            if rd::<u32>(ped, 0x58C) != 0 && rd::<u32>(ped, 0x46C) & 0x100 != 0 {
                sub(this, 0x2C0, ped)
            } else {
                sub(this, 0x386, ped)
            }
        }
        0x386 => {
            let wt = rd::<i32>(sa_world::world::common::ped_active_weapon(ped), 0);
            if rd::<u32>(get_weapon_info(wt, 1), 0) == 1 { sub(this, 0x3FC, ped) } else { sub(this, 0x516, ped) }
        }
        0x3FC => {
            let other = rd::<GPtr<u8>>(this, 0x14);
            if other.is_null() || ped_is_alive(other) as u8 == 0 || task_timer_is_out_of_time(this.byte_add(0x18)) != 0 {
                return sub(this, 0x516, ped);
            }
            let mut d = [0f32; 3];
            let dp: GPtr<u8> = GPtr::from_raw(d.as_mut_ptr()).cast();
            vector_difference(dp, entity_pos(ped), entity_pos(rd::<GPtr<u8>>(this, 0x14)));
            wr::<u32>(dp, 8, 0);
            let r = rd::<f32>(this, 0xC);
            if le_or_nan(x(r).fmul_m(r), vector_magnitude_sqr(dp)) {
                sub(this, 0x386, ped)
            } else {
                sub(this, 0x516, ped)
            }
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexSeekEntityAiming::ControlSubTask(CPed*)` (0x694C70, `ret 4`). `d²` = the 2D distance² from the target
/// (+0xC) to the ped (y², x²; f32, written over the ped argument slot); `gun` = the ped's secondary attack task (slot
/// 0). After a 0x38B sub-task: within +0x14 (`d² < r²`) a new `CTaskSimpleUseGun(target, 0, 1, 1, false)` as that task
/// when there is none, else a 0x3F9 gun's `ControlGun(ped, target, 1)`; otherwise a 0x3F9 gun's `ControlGun(ped,
/// target, 6)`. After 0xCB: a 0x3F9 gun's `ControlGun(ped, target, 1)`, then unless `d² < (+0x10)²` (or NaN) the
/// sub-task's `MakeAbortable(ped, 0, NULL)`. Returns the sub-task.
#[hook(addr = 0x694C70, conv = thiscall, name = "CTaskComplexSeekEntityAiming::ControlSubTask", expect = "64 A1 00 00 00 00 6A FF 68 DB 75 84 00",
       unverified = "frontier2-a10", route = "L2", hazards = "heap-writes,stack-residue",
       note = "d² is stored over the ped argument slot; SEH frame not modelled")]
pub fn seek_entity_aiming_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let tp = entity_pos(rd::<GPtr<u8>>(this, 0xC));
    let pp = entity_pos(ped);
    let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(tp, 0));
    let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(tp, 4));
    let d2 = dy.fmul(dy).fadd(dx.fmul(dx)).to_f32();
    let gun = tm_get_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0);
    let is_gun = |t: GPtr<u8>| !t.is_null() && crate::common::task_get_type(t) == 0x3F9;
    match crate::common::task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0x38B => {
            let r = rd::<f32>(this, 0x14);
            if !ge_or_nan(x(d2), x(r).fmul_m(r)) {
                if gun.is_null() {
                    let mem = crate::common::task_new(0x3C);
                    let t = if mem.is_null() {
                        GPtr::NULL
                    } else {
                        use_gun_ctor(mem, rd::<GPtr<u8>>(this, 0xC), 0, 0, 0, 1, 1, 0)
                    };
                    tm_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 0);
                } else if is_gun(gun) {
                    use_gun_control_gun(gun, ped, rd::<GPtr<u8>>(this, 0xC), 1);
                }
            } else if is_gun(gun) {
                use_gun_control_gun(gun, ped, rd::<GPtr<u8>>(this, 0xC), 6);
            }
        }
        0xCB => {
            if is_gun(gun) {
                use_gun_control_gun(gun, ped, rd::<GPtr<u8>>(this, 0xC), 1);
            }
            let r = rd::<f32>(this, 0x10);
            if !fcom_any(x(d2), x(r).fmul_m(r), 1) {
                crate::common::task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 0, GPtr::NULL);
            }
        }
        _ => {}
    }
    rd::<GPtr<u8>>(this, 8)
}

orig! {
    /// `CTaskComplexUsePairedAttractor::CreateSubTask(int32 type, CPed*)` (0x638D30, `ret 8`)
    fn use_paired_attractor_create_sub_task(t: u32, ty: u32, ped: u32) -> u32 = thiscall 0x638D30 => crate::gen_subtasks::g_task_complex_use_paired_attractor_create_sub_task_638d30;
    /// `CTaskComplex::CreateFirstSubTask(CPed*)` (vtable +0x24)
    fn v_create_first_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x24 => unported;
}

/// [`use_paired_attractor_create_sub_task`] with pointers.
fn paired_sub(this: GPtr<u8>, ty: u32, ped: GPtr<u8>) -> GPtr<u8> {
    GPtr::from_addr(use_paired_attractor_create_sub_task(this.addr() as u32, ty, ped.addr() as u32) as usize)
}

/// `CTaskComplexUsePairedAttractor::CreateNextSubTask(CPed*)` (0x639E40, `ret 4`). By the sub-task's type: 0xF9 →
/// +0x20 += 1; with a partner (+0x10) `CreateSubTask(0xF8)`; without one 0xF9 while the attractor's scripted effect
/// (+0xC → +4) is still flagged (0xC39FE0 + `CScripted2dEffects::GetIndex`), else 0x516. 0xF8 → 0xF7 with a partner,
/// else 0xF9. 0xF7 → when +0x14 is set +0x1C = `(+0x1C + 5) % 4` (C remainder) and the pair data's (+0x18) +0xC is
/// read, otherwise its +0x1C: 0 → 0x516, 1 → 0xF9, others NULL; then the partner reference is cleaned up and cleared,
/// +0x14 = 0, +0x18 = NULL. Others: NULL.
#[hook(addr = 0x639E40, conv = thiscall, name = "CTaskComplexUsePairedAttractor::CreateNextSubTask", expect = "55 56 8B F1 8B 4E 08",
       unverified = "frontier2-a10", route = "L2", hazards = "heap-writes")]
pub fn use_paired_attractor_create_next_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    match crate::common::task_get_type(rd::<GPtr<u8>>(this, 8)) {
        0xF9 => {
            wr::<u32>(this, 0x20, rd::<u32>(this, 0x20).wrapping_add(1));
            if rd::<u32>(this, 0x10) != 0 {
                return paired_sub(this, 0xF8, ped);
            }
            let i = scripted_2d_effects_get_index(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 4));
            if g::<u8>(0xC39FE0usize.wrapping_add(i as usize)).read() != 0 {
                paired_sub(this, 0xF9, ped)
            } else {
                paired_sub(this, 0x516, ped)
            }
        }
        0xF8 => {
            if rd::<u32>(this, 0x10) == 0 { paired_sub(this, 0xF9, ped) } else { paired_sub(this, 0xF7, ped) }
        }
        0xF7 => {
            let flag = rd::<u8>(this, 0x14) != 0;
            let pair = rd::<GPtr<u8>>(this, 0x18);
            let v = if flag {
                wr::<i32>(this, 0x1C, rd::<i32>(this, 0x1C).wrapping_add(5) % 4);
                rd::<u32>(pair, 0xC)
            } else {
                rd::<u32>(pair, 0x1C)
            };
            let t = match v {
                0 => paired_sub(this, 0x516, ped),
                1 => paired_sub(this, 0xF9, ped),
                _ => GPtr::NULL,
            };
            let partner = rd::<GPtr<u8>>(this, 0x10);
            if !partner.is_null() {
                entity_clean_up_old_reference(partner, this.byte_add(0x10));
                wr::<u32>(this, 0x10, 0);
            }
            wr::<u8>(this, 0x14, 0);
            wr::<u32>(this, 0x18, 0);
            t
        }
        _ => GPtr::NULL,
    }
}

/// `CTaskComplexFleePoint::ControlSubTask(CPed*)` (0x65B660, `ret 4`). A sub-task other than 0x384 / 0x387: an active
/// look-at (`IKChainManager::IsLooking`) is aborted (blend 250); the sub-task is kept. Otherwise: +0x38 → `Say(0x15B,
/// 0, 0.1, 0, 0, 0)`; +0x39 (new flee point) → cleared, the timer (+0x28 start, +0x2C = +0x24 duration, +0x30 on)
/// restarted and `CreateFirstSubTask(ped)`; else a running timer (+0x31 restarts it at now) past its end →
/// `CreateSubTask(0xF4)`. Then, not looking, `(rand() & 0xFFFF) / 32768 * 100` truncated at most 5 →
/// `LookAt("TaskFleePoint", ped, NULL, 2000, -1, &+0xC, false, 0.25, 500, 3, false)`.
#[hook(addr = 0x65B660, conv = thiscall, name = "CTaskComplexFleePoint::ControlSubTask", expect = "56 8B F1 57 8B 7E 08",
       unverified = "frontier2-a10", route = "L2", hazards = "heap-writes",
       note = "rand at 0x65B753")]
pub fn flee_point_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let ik = GPtr::from_addr(0xC15448);
    let mut sub = rd::<GPtr<u8>>(this, 8);
    if crate::common::task_get_type(sub) != 0x384 && crate::common::task_get_type(rd::<GPtr<u8>>(this, 8)) != 0x387 {
        if ik_is_looking(ik, ped) != 0 {
            ik_abort_look_at(ik, ped, 0xFA);
        }
        return rd::<GPtr<u8>>(this, 8);
    }
    if rd::<u8>(this, 0x38) != 0 {
        ped_say(ped, 0x15B, 0, f32::from_bits(0x3DCC_CCCD), 0, 0, 0);
    }
    let now = || g::<u32>(TIME_MS).read();
    if rd::<u8>(this, 0x39) != 0 {
        wr::<u8>(this, 0x39, 0);
        wr::<u32>(this, 0x28, now());
        wr::<u32>(this, 0x2C, rd::<u32>(this, 0x24));
        wr::<u8>(this, 0x30, 1);
        sub = v_create_first_sub_task(this, ped);
    } else if rd::<u8>(this, 0x30) != 0 {
        if rd::<u8>(this, 0x31) != 0 {
            wr::<u32>(this, 0x28, now());
            wr::<u8>(this, 0x31, 0);
        }
        if now() >= rd::<u32>(this, 0x2C).wrapping_add(rd::<u32>(this, 0x28)) {
            sub = GPtr::from_addr(crate::gen_subtasks::g_task_complex_flee_point_create_sub_task_65b460(this.addr() as u32, 0xF4) as usize);
        }
    }
    if ik_is_looking(ik, ped) == 0 {
        let r = xi(rand_at(0x65B753) & 0xFFFF).fmul_m(gf(0x858B14)).fmul_m(gf(0x858628));
        if crate::common::ftol32(r) <= 5 {
            ik_look_at(ik, GPtr::from_addr(0x86F6F8), ped, GPtr::NULL, 2000, -1, this.byte_add(0xC), 0, 0.25, 500, 3, 0);
        }
    }
    sub
}

use sa_pop::bindings::audio_say_pedless;
orig! {
    /// `CLocalisation::PedsOnFire()` (0x56D2D0)
    fn localisation_peds_on_fire() -> u8 = cdecl 0x56D2D0 => unported;
}

/// The ped's target (+0xC) on the boat task's checks: the ped's +0x598 is 6 and the target a player.
fn boat_target_is_player(this: GPtr<u8>, ped: GPtr<u8>) -> bool {
    rd::<u32>(ped, 0x598) == 6 && {
        let t = rd::<GPtr<u8>>(this, 0xC);
        !t.is_null() && ped_is_player(t) != 0
    }
}

/// `CTaskComplexKillPedFromBoat::ControlSubTask(CPed*)` (0x622980, `ret 4`). After 0x3E8 (+0x598 = 6, a player
/// target): the audio engine's `SayPedless(0x34, 0xAD, ped, 0, 1.0, 0, 0, 0)` when the target sits (+0x46C bit 8) in a
/// vehicle (+0x58C) of type 3 (+0x1B8), else 0xAF when its +0x40 has bit 8 (no vehicle or not in one); then, same
/// test, with no wanted level (`FindPlayerWanted(-1)` +0x2C zero) and the sub-task abortable (`MakeAbortable(ped, 1,
/// NULL)`): a `CTaskSimpleStandStill(0, true, false, 8.0)`. After 0xCB (same test) with a wanted level and the
/// sub-task abortable: a `CTaskComplexKillPedOnFoot(target, -1, 0, 0, 0, 1)`. Else the sub-task.
#[hook(addr = 0x622980, conv = thiscall, name = "CTaskComplexKillPedFromBoat::ControlSubTask", expect = "64 A1 00 00 00 00 6A FF 68 86 F9 83 00",
       unverified = "frontier2-a10", route = "L2", hazards = "heap-writes",
       note = "SEH frame not modelled")]
pub fn kill_ped_from_boat_control_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let sub = rd::<GPtr<u8>>(this, 8);
    match crate::common::task_get_type(sub) {
        0x3E8 => {
            if boat_target_is_player(this, ped) {
                let t = rd::<GPtr<u8>>(this, 0xC);
                let veh = rd::<GPtr<u8>>(t, 0x58C);
                let id = if rd::<u32>(t, 0x46C) & 0x100 != 0 && !veh.is_null() {
                    (rd::<u8>(veh, 0x1B8) == 3).then_some(0xAD)
                } else {
                    (rd::<u32>(t, 0x40) & 0x100 != 0).then_some(0xAF)
                };
                if let Some(id) = id {
                    audio_say_pedless(GPtr::from_addr(0xB6BC90), 0x34, id, ped, 0, 1.0, 0, 0, 0);
                }
            }
            if boat_target_is_player(this, ped)
                && rd::<u32>(find_player_wanted(-1), 0x2C) == 0
                && crate::common::task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, GPtr::NULL) != 0
            {
                let mem = crate::common::task_new(0x20);
                return if mem.is_null() { GPtr::NULL } else { stand_still_ctor(mem, 0, 1, 0, 8.0) };
            }
            sub
        }
        0xCB => {
            if boat_target_is_player(this, ped)
                && rd::<u32>(find_player_wanted(-1), 0x2C) != 0
                && crate::common::task_make_abortable(rd::<GPtr<u8>>(this, 8), ped, 1, GPtr::NULL) != 0
            {
                let mem = crate::common::task_new(0x38);
                return if mem.is_null() {
                    GPtr::NULL
                } else {
                    kill_ped_on_foot_ctor(mem, rd::<GPtr<u8>>(this, 0xC), -1, 0, 0, 0, 1)
                };
            }
            sub
        }
        _ => sub,
    }
}

/// `CTaskSimplePlayerOnFire::ProcessPed(CPed*)` (0x6335C0, `ret 4`, AL). First run: timer +8 = now, +0xC = 5000, +0x10
/// on. No fire (+0x730): 1. The timer (restarted at now by +0x11) expired or swimming (`GetTaskSwim`): the fire is
/// extinguished, 1. The local player (0xB7CD98) with the fireproof flag 0xB7CEE6: 0. Otherwise `ComputeFireDamage(ped,
/// response)` (response zeroed): no effect (+8 byte clear) → `Say(0x15A, 0, 0.1, 0, 0, 0)` when peds may burn
/// (`CLocalisation::PedsOnFire`), 0. With an effect: already reported (+0x14) → 1; else the same line, then a
/// stack `CEventDamage(NULL, now, 37, 3, 0, 0, in-vehicle dword)` with the response copied to +0x38 is added to the
/// ped's event group (+0x47C +0x68), +0x14 = 1, the event destroyed; 0.
#[hook(addr = 0x6335C0, conv = thiscall, name = "CTaskSimplePlayerOnFire::ProcessPed", expect = "64 A1 00 00 00 00 6A FF 68 38 07 84 00",
       unverified = "frontier2-a10", route = "L2", hazards = "ax-al-return,heap-writes",
       note = "the event lives on the Rust stack; SEH frame not modelled")]
pub fn player_on_fire_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let now = || g::<u32>(TIME_MS).read();
    if rd::<u8>(this, 0x10) == 0 {
        wr::<u32>(this, 8, now());
        wr::<u32>(this, 0xC, 5000);
        wr::<u8>(this, 0x10, 1);
    }
    let fire = || rd::<GPtr<u8>>(ped, 0x730);
    if fire().is_null() {
        return 1;
    }
    if rd::<u8>(this, 0x10) != 0 {
        if rd::<u8>(this, 0x11) != 0 {
            wr::<u32>(this, 8, now());
            wr::<u8>(this, 0x11, 0);
        }
        if now() >= rd::<u32>(this, 0xC).wrapping_add(rd::<u32>(this, 8)) {
            fire_extinguish(fire());
            return 1;
        }
    }
    if !get_task_swim(rd::<GPtr<u8>>(ped, 0x47C)).is_null() {
        fire_extinguish(fire());
        return 1;
    }
    if g::<u32>(0xB7CD98).read() == ped.addr() as u32 && g::<u8>(0xB7CEE6).read() != 0 {
        return 0;
    }
    let mut resp = [0u32; 3];
    let rp: GPtr<u8> = GPtr::from_raw(resp.as_mut_ptr()).cast();
    crate::frontier_a::on_fire_compute_fire_damage(ped, rp);
    if rd::<u8>(rp, 8) == 0 {
        if localisation_peds_on_fire() != 0 {
            ped_say(ped, 0x15A, 0, f32::from_bits(0x3DCC_CCCD), 0, 0, 0);
        }
        return 0;
    }
    if rd::<u8>(this, 0x14) != 0 {
        return 1;
    }
    if localisation_peds_on_fire() != 0 {
        ped_say(ped, 0x15A, 0, f32::from_bits(0x3DCC_CCCD), 0, 0, 0);
    }
    let mut ev = [0u32; 0x14];
    let ep: GPtr<u8> = GPtr::from_raw(ev.as_mut_ptr()).cast();
    let in_veh = (rd::<u32>(ped, 0x46C) >> 8) & 0xFFFF_FF01;
    event_damage_ctor(ep, GPtr::NULL, now(), 0x25, 3, 0, 0, in_veh);
    for k in 0..3 {
        wr::<u32>(ep, 0x38 + k * 4, rd::<u32>(rp, k * 4));
    }
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), ep, 0);
    wr::<u8>(this, 0x14, 1);
    event_damage_dtor(ep);
    0
}

/// `|n · p + d|` for the bounding-box plane `k` (normals at `normals`, distances at `ds`).
fn plane_distance(normals: GPtr<u8>, ds: GPtr<u8>, k: isize, p: GPtr<u8>) -> X87 {
    let n = normals.byte_add(k * 12);
    x(rd::<f32>(n, 4)).fmul_m(rd::<f32>(p, 4)).fadd(x(rd::<f32>(n, 8)).fmul_m(rd::<f32>(p, 8)))
        .fadd(x(rd::<f32>(n, 0)).fmul_m(rd::<f32>(p, 0))).fadd_m(rd::<f32>(ds, k * 4)).fabs()
}

/// `CTaskComplexDiveFromAttachedEntityAndGetUp::CreateFirstSubTask(CPed*)` (0x657820, `ret 4`). Not attached
/// (+0xFC): the dive direction +0x14 is the ped's forward vector. Attached: the entity's side the ped is on
/// (`ComputeEntityHitSide`) picks one of its four directions (`ComputeEntityDirs`); on a vehicle (+0x36 & 7 = 2), from
/// side 0 or 2 and with the vehicle moving along that direction (`speed · dir > 0`), the bounding-box planes at the
/// ped's height choose between dirs 1 and 3: dir 1 when `|plane 3|` is farther than `|plane 1|` from the ped, else
/// dir 3 (NaN too). Attached peds get +0x77C = 2pi. Then `CreateSubTask(+0x20 ? 0x386 : 0x1F7)`.
#[hook(addr = 0x657820, conv = thiscall, name = "CTaskComplexDiveFromAttachedEntityAndGetUp::CreateFirstSubTask", expect = "83 EC 70 53 55 56",
       unverified = "frontier2-a10", route = "L2", hazards = "heap-writes")]
pub fn dive_from_attached_entity_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let dir = this.byte_add(0x14);
    let ent = rd::<GPtr<u8>>(ped, 0xFC);
    if ent.is_null() {
        let f = rd::<GPtr<u8>>(ped, 0x14).byte_add(0x10);
        for k in 0..3 {
            wr::<u32>(dir, k * 4, rd::<u32>(f, k * 4));
        }
    } else {
        let side = compute_entity_hit_side_ped(ped, ent) as isize;
        let mut dirs = [0u32; 12];
        let dp: GPtr<u8> = GPtr::from_raw(dirs.as_mut_ptr()).cast();
        crate::gen_bodies::g_cped_geometry_analyser_compute_entity_dirs_5f1500(rd::<u32>(ped, 0xFC), dp.addr() as u32);
        let copy = |k: isize| {
            for c in 0..3 {
                wr::<u32>(dir, c * 4, rd::<u32>(dp, k * 12 + c * 4));
            }
        };
        copy(side);
        let ent = rd::<GPtr<u8>>(ped, 0xFC);
        if rd::<u8>(ent, 0x36) & 7 == 2 && (side == 2 || side == 0) {
            let dot = x(rd::<f32>(ent, 0x4C)).fmul_m(rd::<f32>(dir, 8)).fadd(x(rd::<f32>(ent, 0x48)).fmul_m(rd::<f32>(dir, 4)))
                .fadd(x(rd::<f32>(dir, 0)).fmul_m(rd::<f32>(ent, 0x44)));
            if gt(dot, x(gf(0x858B50))) {
                let mut normals = [0u32; 12];
                let mut ds = [0u32; 4];
                let np: GPtr<u8> = GPtr::from_raw(normals.as_mut_ptr()).cast();
                let dsp: GPtr<u8> = GPtr::from_raw(ds.as_mut_ptr()).cast();
                vector_ctor_iterator(np, 0xC, 4, 0x404350);
                bbox_planes(rd::<u32>(entity_pos(ped), 8), rd::<GPtr<u8>>(ped, 0xFC), np, dsp);
                let pp = entity_pos(ped);
                let a = plane_distance(np, dsp, 1, pp);
                let b = plane_distance(np, dsp, 3, entity_pos(ped));
                copy(if le_or_nan(b, a) { 3 } else { 1 });
            }
        }
    }
    if rd::<u32>(ped, 0xFC) != 0 {
        wr::<u32>(ped, 0x77C, 0x40C9_0FDB);
    }
    let ty = if rd::<u8>(this, 0x20) != 0 { 0x386 } else { 0x1F7 };
    GPtr::from_addr(crate::gen_subtasks::g_task_complex_evasive_dive_and_get_up_create_sub_task_653820(this.addr() as u32, ty) as usize)
}

/// `CTaskInteriorGoToInfo::CreateFirstSubTask(CPed*)` (0x676640, `ret 4`). No info (+0xC): NULL. Teleporting (+0x14):
/// the ped is put at the info's position (matrix or placement), its rotations +0x558 / +0x55C = `LimitRadianAngle(
/// angle(dir.x, dir.y, 0, 0))` and that heading set (`SetRotateZOnly` with a matrix, else placement +0x10); NULL.
/// Otherwise, in the task's interior (+0x10, `GetPedsInterior`): a `CTaskSimpleGoToPoint(4, info position, 0.2, false,
/// false)`; elsewhere a `CTaskComplexFollowNodeRoute(4, the task interior's path node (+0x3F0) position, 0.2, 3.0,
/// 2.0, false, -1, true)` whose +0x28 becomes the ped's interior's +0x3F0 (written through a NULL task too, as the
/// bytes).
#[hook(addr = 0x676640, conv = thiscall, name = "CTaskInteriorGoToInfo::CreateFirstSubTask", expect = "64 A1 00 00 00 00 6A FF 68 66 5E 84 00",
       unverified = "frontier2-a10", route = "L2", hazards = "heap-writes,stack-residue",
       note = "the angle is stored over the ped argument slot; SEH frame not modelled")]
pub fn interior_go_to_info_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let info = rd::<GPtr<u8>>(this, 0xC);
    if info.is_null() {
        return GPtr::NULL;
    }
    if rd::<u8>(this, 0x14) != 0 {
        let (dx, dy) = (rd::<f32>(info, 0x10), rd::<f32>(info, 0x14));
        let pos = [rd::<u32>(info, 4), rd::<u32>(info, 8), rd::<u32>(info, 0xC)];
        let dst = entity_pos(ped);
        for (k, v) in pos.iter().enumerate() {
            wr::<u32>(dst, k as isize * 4, *v);
        }
        let a = general_get_radian_angle_between_points(dx, dy, 0.0, 0.0).to_f32();
        let h = st0(general_limit_radian_angle(a)).to_f32();
        wr::<f32>(ped, 0x55C, h);
        wr::<f32>(ped, 0x558, h);
        let m = rd::<GPtr<u8>>(ped, 0x14);
        if m.is_null() {
            wr::<f32>(ped, 0x10, h);
        } else {
            matrix_set_rotate_z_only(m, h);
        }
        return GPtr::NULL;
    }
    let here = sa_world::world::interior_furnish_lookup::get_peds_interior(GPtr::from_addr(0xBAF670), ped);
    if here == rd::<GPtr<u8>>(this, 0x10) {
        let mem = crate::common::task_new(0x30);
        if mem.is_null() {
            return GPtr::NULL;
        }
        let info = rd::<GPtr<u8>>(this, 0xC);
        let mut pos = [rd::<u32>(info, 4), rd::<u32>(info, 8), rd::<u32>(info, 0xC)];
        return go_to_point_ctor(mem, 4, GPtr::from_raw(pos.as_mut_ptr()).cast(), 0x3E4C_CCCD, 0, 0);
    }
    let link = rd::<u32>(rd::<GPtr<u8>>(this, 0x10), 0x3F0);
    let nodes = g::<u32>(0x96F854 + (link & 0xFFFF) as usize * 4).read();
    let node = GPtr::<u8>::from_addr(((link >> 16).wrapping_mul(0x1C)).wrapping_add(nodes) as usize);
    let mut pos = [0f32; 3];
    let pp: GPtr<f32> = GPtr::from_raw(pos.as_mut_ptr());
    get_position(node, pp);
    let mem = crate::common::task_new(0x60);
    let t = if mem.is_null() {
        GPtr::NULL
    } else {
        follow_node_route_ctor(mem, 4, pp.cast(), 0x3E4C_CCCD, g::<u32>(0x86FCA4).read(), g::<u32>(0x86FCA8).read(), 0,
                               u32::MAX, 1)
    };
    wr::<u32>(t, 0x28, rd::<u32>(here, 0x3F0));
    t
}

/// A new `CTaskSimpleDuck(4, 0, -1)` (NULL when the task pool is full) as the ped's secondary task 1 (duck).
fn duck_toggle_start_duck(ped: GPtr<u8>) {
    let mem = crate::common::task_new(0x28);
    let t = if mem.is_null() { GPtr::NULL } else { duck_ctor(mem, 4, 0, u32::MAX) };
    tm_set_task_secondary(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 1);
}

/// `CTaskSimpleDuckToggle::ProcessPed(CPed*)` (0x62F6E0, `ret 4`, AL = 1). Mode +8 0 or -1 with a secondary duck
/// (`GetTaskDuck(0)`): its `MakeAbortable(ped, 0, NULL)`, then the gun task's `ClearAnim(ped)` when there is one
/// (`GetTaskUseGun`). Mode 1 or -1 (-1 only without a duck): `o` = the simplest active task when it is a 0x19F one.
/// Without a ducking secondary task (`GetTaskDuck(1)`): `o.MakeAbortable(ped, 1, NULL)`, a new duck (4, 0, -1) as
/// secondary task 1 and the gun task's `ClearAnim`. With one whose control type (+0x24) is not 4:
/// `o.AbortBecauseOfOtherDuck(ped)`, and unless `SetControlType(4)` succeeds the old duck's
/// `AbortBecauseOfOtherDuck(ped)` and a new duck.
#[hook(addr = 0x62F6E0, conv = thiscall, name = "CTaskSimpleDuckToggle::ProcessPed", expect = "64 A1 00 00 00 00 6A FF 68 26 03 84 00",
       unverified = "frontier2-a10", route = "L2", hazards = "ax-al-return,heap-writes",
       note = "SEH frame not modelled")]
pub fn duck_toggle_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let intel = || rd::<GPtr<u8>>(ped, 0x47C);
    let tm = || intel().byte_add(4);
    let clear_gun_anim = || {
        if !intel_get_task_use_gun(intel()).is_null() {
            use_gun_clear_anim(intel_get_task_use_gun(intel()), ped);
        }
    };
    let mode = rd::<i32>(this, 8);
    if (mode == 0 || mode == -1) && !intel_get_task_duck(intel(), 0).is_null() {
        crate::common::task_make_abortable(intel_get_task_duck(intel(), 0), ped, 0, GPtr::NULL);
        clear_gun_anim();
        return 1;
    }
    let mode = rd::<i32>(this, 8);
    if mode != 1 && mode != -1 {
        return 1;
    }
    let mut other = GPtr::NULL;
    if !tm_get_simplest_active_task(tm()).is_null() && crate::common::task_get_type(tm_get_simplest_active_task(tm())) == 0x19F {
        other = tm_get_simplest_active_task(tm());
    }
    if intel_get_task_duck(intel(), 1).is_null() {
        if !other.is_null() {
            crate::common::task_make_abortable(other, ped, 1, GPtr::NULL);
        }
        duck_toggle_start_duck(ped);
        clear_gun_anim();
        return 1;
    }
    if rd::<u8>(intel_get_task_duck(intel(), 1), 0x24) == 4 {
        return 1;
    }
    if !other.is_null() {
        crate::duck_sequence_groups::duck_abort_because_of_other_duck(other, ped);
    }
    if crate::duck_sequence_groups::duck_set_control_type(intel_get_task_duck(intel(), 1), 4) == 0 {
        crate::duck_sequence_groups::duck_abort_because_of_other_duck(intel_get_task_duck(intel(), 1), ped);
        duck_toggle_start_duck(ped);
    }
    1
}

/// `CTaskSimpleThrowControl::ProcessPed(CPed*)` (0x61F9F0, `ret 4`, AL). Finished (+8): 1. `SetMoveState(1)`. No
/// secondary attack task: once (+9) a `CTaskSimpleThrowProjectile(target +0xC, +0x10 point)` (NULL when the pool is
/// full) becomes it, else 1. A secondary attack task other than 0x3FA: its `MakeAbortable(ped, 1, NULL)`, 0. Then `t`
/// = `GetTaskThrow()`; the point counts when it differs from (0, 0, 0); the heading +0x55C = `atan2(-d.x, d.y)` towards
/// the target entity, else towards the point when it counts. Not started (+9 clear): 0. `t`'s +8 set: 1. With a point
/// it becomes `t`'s +0x14; 0.
#[hook(addr = 0x61F9F0, conv = thiscall, name = "CTaskSimpleThrowControl::ProcessPed", expect = "6A FF 64 A1 00 00 00 00 68 DB F4 83 00",
       unverified = "frontier2-a10", route = "L2", hazards = "x87-transcendental,ax-al-return,heap-writes,stack-residue",
       note = "GetTaskThrow's result is kept over the ped argument slot; SEH frame not modelled")]
pub fn throw_control_process_ped(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 8) != 0 {
        return 1;
    }
    sa_world::bindings::ped_set_move_state(ped, 1);
    let tm = || rd::<GPtr<u8>>(ped, 0x47C).byte_add(4);
    if tm_get_task_secondary(tm(), 0).is_null() {
        if rd::<u8>(this, 9) != 0 {
            return 1;
        }
        let mem = crate::common::task_new(0x24);
        let t = if mem.is_null() {
            GPtr::NULL
        } else {
            throw_projectile_ctor(mem, rd::<GPtr<u8>>(this, 0xC), rd::<u32>(this, 0x10), rd::<u32>(this, 0x14),
                                  rd::<u32>(this, 0x18))
        };
        tm_set_task_secondary(tm(), t, 0);
        wr::<u8>(this, 9, 1);
    } else if crate::common::task_get_type(tm_get_task_secondary(tm(), 0)) != 0x3FA {
        crate::common::task_make_abortable(tm_get_task_secondary(tm(), 0), ped, 1, GPtr::NULL);
        return 0;
    }
    let throw = intel_get_task_throw(rd::<GPtr<u8>>(ped, 0x47C));
    let pos = this.byte_add(0x10);
    let mut zero = [0u32; 3];
    let has_pos = vector_ne(pos, GPtr::from_raw(zero.as_mut_ptr()).cast()) != 0;
    let target = rd::<GPtr<u8>>(this, 0xC);
    let (from, to) = if !target.is_null() {
        (entity_pos(ped), Some(entity_pos(target)))
    } else if has_pos {
        (entity_pos(ped), Some(pos))
    } else {
        (entity_pos(ped), None)
    };
    if let Some(to) = to {
        let dx = x(rd::<f32>(to, 0)).fsub_m(rd::<f32>(from, 0)).to_f32();
        let dy = x(rd::<f32>(to, 4)).fsub_m(rd::<f32>(from, 4)).to_f32();
        wr::<f32>(ped, 0x55C, x(dx).fchs().fpatan(x(dy)).to_f32());
    }
    if rd::<u8>(this, 9) == 0 {
        return 0;
    }
    if rd::<u8>(throw, 8) != 0 {
        return 1;
    }
    if has_pos {
        for k in 0..3 {
            wr::<u32>(throw, 0x14 + k * 4, rd::<u32>(pos, k * 4));
        }
    }
    0
}

orig! {
    /// The scalar deleting destructor (vtable +0).
    fn v_delete(o: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0x0 => unported;
}

/// A new task of `size` bytes built by `ctor`, NULL when the task pool is full.
fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let mem = crate::common::task_new(size);
    if mem.is_null() { GPtr::NULL } else { ctor(mem) }
}

/// A `CTaskComplexSequence` of `first` and a `CTaskSimpleCarDrive(veh, NULL, false)`, stored at the task pair `pair`
/// (+4) when its ped (+0) is set and is `ped` (or `ped` is NULL), else deleted.
fn sit_in_car_sequence(pair: GPtr<u8>, ped: GPtr<u8>, veh: GPtr<u8>, first: impl FnOnce() -> GPtr<u8>) {
    let seq = new_task(0x40, sequence_ctor);
    sequence_add_task(seq, first());
    sequence_add_task(seq, new_task(0x60, |m| simple_car_drive_ctor(m, veh, GPtr::NULL, 0)));
    let m = rd::<GPtr<u8>>(pair, 0);
    if !m.is_null() && (ped.is_null() || m == ped) {
        wr::<GPtr<u8>>(pair, 4, seq);
    } else if !seq.is_null() {
        v_delete(seq, 1);
    }
}

/// `CPedGroupDefaultTaskAllocatorSitInLeaderCar::AllocateDefaultTasks(CPedGroup*, CPed*)` (0x5F6FC0, `ret 8`; `this`
/// unused). With a leader (+0x28) in a vehicle (+0x58C): the leader's pair (+0x2A8) gets the sequence (enter as driver,
/// drive); then each of the seven members (+0xC) up to the vehicle's passenger count (+0x488) gets, in seat order, the
/// sequence (enter as passenger at `ComputeTargetDoorToEnterAsPassenger(veh, seat)`, drive) in its pair (+0x21C, 0x14
/// each). A sequence whose pair is empty or belongs to another ped than the given one is deleted.
#[hook(addr = 0x5F6FC0, conv = thiscall, name = "CPedGroupDefaultTaskAllocatorSitInLeaderCar::AllocateDefaultTasks",
       expect = "64 A1 00 00 00 00 6A FF 68 D2 DF 83 00",
       unverified = "frontier2-a10", route = "L2", hazards = "heap-writes",
       note = "SEH frame not modelled")]
pub fn sit_in_leader_car_allocate_default_tasks(_this: GPtr<u8>, group: GPtr<u8>, ped: GPtr<u8>) {
    let leader = rd::<GPtr<u8>>(group, 0x28);
    if leader.is_null() {
        return;
    }
    let veh = rd::<GPtr<u8>>(leader, 0x58C);
    if veh.is_null() {
        return;
    }
    sit_in_car_sequence(group.byte_add(0x2A8), ped, veh, || new_task(0x50, |m| enter_car_as_driver_ctor(m, veh)));
    let max = i32::from(rd::<u8>(veh, 0x488));
    let mut seat = 0i32;
    for k in 0..7isize {
        if rd::<u32>(group, 0xC + k * 4) == 0 || seat >= max {
            continue;
        }
        let door = compute_target_door_to_enter_as_passenger(veh, seat as u32);
        sit_in_car_sequence(group.byte_add(0x21C + k * 0x14), ped, veh,
                            || new_task(0x50, |m| enter_car_as_passenger_ctor(m, veh, door, 0)));
        seat += 1;
    }
}

orig! {
    /// `CEvent::GetEventType()` (vtable +4)
    fn v_event_type(e: GPtr<u8>) -> i32 = thiscall vslot 0x4 => unported;
}

/// `CTaskComplexInvestigateDisturbance::CreateFirstSubTask(CPed*)` (0x66C820, `ret 4`). The current event (intelligence
/// +0x38 history): type 0x3E → `Say(rand() / 32767 < 0.2 ? 0xCC : 0xCA, 0, 1.0, 0, 0, 0)`, type 0x3F likewise with
/// 0xCB. The direction to the point (+0xC), normalised: facing it (`dir · forward >= 0.707`, NaN too) →
/// `SetTaskDuckSecondary(5000)`; else `LookAt("TaskInvDisturb", ped, NULL, 5000, -1, &+0xC, false, 0.25, 500, 3,
/// false)`. Then a `CTaskComplexTurnToFaceEntityOrCoord` towards the entity (+0x18) or else the point, 3.0 and 0.2
/// (NULL when the pool is full).
#[hook(addr = 0x66C820, conv = thiscall, name = "CTaskComplexInvestigateDisturbance::CreateFirstSubTask",
       expect = "64 A1 00 00 00 00 6A FF 68 D6 52 84 00",
       unverified = "frontier2-a10", route = "L2", hazards = "heap-writes,stack-residue",
       note = "rand at 0x66C86A / 0x66C8B3 (stored over the ped argument slot); SEH frame not modelled")]
pub fn investigate_disturbance_create_first_sub_task(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let ev = history_get_current_event(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x38));
    if !ev.is_null() {
        let say = |site: u32, other: u32| {
            let r = xi(rand_at(site)).fmul_m(gf(0x858C7C));
            let id = if ge_or_nan(r, x(gf(0x858CC4))) { other } else { 0xCC };
            ped_say(ped, id, 0, 1.0, 0, 0, 0);
        };
        if v_event_type(ev) == 0x3E {
            say(0x66C86A, 0xCA);
        } else if v_event_type(ev) == 0x3F {
            say(0x66C8B3, 0xCB);
        }
    }
    let pp = entity_pos(ped);
    let pos = this.byte_add(0xC);
    let dz = x(rd::<f32>(this, 0x14)).fsub_m(rd::<f32>(pp, 8));
    let dy = x(rd::<f32>(pos, 4)).fsub_m(rd::<f32>(pp, 4));
    let dx = x(rd::<f32>(pos, 0)).fsub_m(rd::<f32>(pp, 0));
    let mut d = [dx.to_f32(), dy.to_f32(), dz.to_f32()];
    vector_normalise(GPtr::from_raw(&mut d));
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let dot = x(d[2]).fmul_m(rd::<f32>(m, 0x18)).fadd(x(d[1]).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(x(d[0]).fmul_m(rd::<f32>(m, 0x10)));
    if ge_or_nan(dot, x(gf(0x863C1C))) {
        crate::bindings::intel_set_task_duck_secondary(rd::<GPtr<u8>>(ped, 0x47C), 5000);
    } else {
        ik_look_at(GPtr::from_addr(0xC15448), GPtr::from_addr(0x87003C), ped, GPtr::NULL, 5000, -1, pos, 0, 0.25, 500, 3, 0);
    }
    let ent = rd::<GPtr<u8>>(this, 0x18);
    let tol = g::<u32>(0x86FC80).read();
    if !ent.is_null() {
        new_task(0x28, |m| turn_to_face_entity_ctor(m, rd::<GPtr<u8>>(this, 0x18), 0x4040_0000, tol))
    } else {
        new_task(0x28, |m| task_turn_to_face_ctor(m, pos, 0x4040_0000, tol))
    }
}

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_JET_PACK_PROCESS_CONTROL_INPUT,
    HOOK_JET_PACK_PROCESS_THRUST,
    HOOK_JET_PACK_PROCESS_PED,
    HOOK_STEALTH_KILL_PROCESS_PED,
    HOOK_DESTROY_CAR_MELEE_CREATE_NEXT_SUB_TASK,
    HOOK_SHUFFLE_SEATS_CREATE_NEXT_SUB_TASK,
    HOOK_ENTER_CAR_AS_PASSENGER_WAIT_CONTROL_SUB_TASK,
    HOOK_INTERIOR_GO_TO_INFO_CREATE_NEXT_SUB_TASK,
    HOOK_WAIT_FOR_BACKUP_CREATE_NEXT_SUB_TASK,
    HOOK_SEEK_ENTITY_AIMING_CONTROL_SUB_TASK,
    HOOK_USE_PAIRED_ATTRACTOR_CREATE_NEXT_SUB_TASK,
    HOOK_FLEE_POINT_CONTROL_SUB_TASK,
    HOOK_KILL_PED_FROM_BOAT_CONTROL_SUB_TASK,
    HOOK_PLAYER_ON_FIRE_PROCESS_PED,
    HOOK_DIVE_FROM_ATTACHED_ENTITY_CREATE_FIRST_SUB_TASK,
    HOOK_INTERIOR_GO_TO_INFO_CREATE_FIRST_SUB_TASK,
    HOOK_DUCK_TOGGLE_PROCESS_PED,
    HOOK_THROW_CONTROL_PROCESS_PED,
    HOOK_SIT_IN_LEADER_CAR_ALLOCATE_DEFAULT_TASKS,
    HOOK_INVESTIGATE_DISTURBANCE_CREATE_FIRST_SUB_TASK,
];
