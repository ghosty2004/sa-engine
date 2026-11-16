//! FRONTIER-2 turn 7, lane A (batch `frontier2-a10`): AI frontier entries ported from the 1.0 US bytes (gta-reversed
//! read as an aid; where it differs the bytes win, noted per function).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::ped_replace_weapon_when_exiting_vehicle;
use crate::common::{g, rd, wr, x, xi, X87};
use crate::fall::fild_u32;
use crate::kill_ped_on_foot_armed::current_weapon_info;
use sa_core::bindings::{general_get_radian_angle_between_points, general_limit_radian_angle};
use sa_core::crt::rand_at;
use sa_core::field::gf;
use sa_math::fpu::{fcom_ne, ge_or_nan, gt, le_or_nan, lt, st0};
use sa_math::vector::CVector;
use sa_pop::bindings::{get_look_behind_for_car, pad_get_look_left, pad_get_look_right};
use sa_stream::bindings::{calculate_ground_height, ped_is_player};
use sa_world::bindings::{
    camera_get_look_direction, fx_kill, get_pad_from_player, max_f, min_f, pad_get_accelerate, pad_get_brake,
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

/// Every hook of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_JET_PACK_PROCESS_CONTROL_INPUT,
    HOOK_JET_PACK_PROCESS_THRUST,
    HOOK_JET_PACK_PROCESS_PED,
];
