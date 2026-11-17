//! FRONTIER-2 turn 10, lane B (batch `frontier2-b23`): `CAutomobile::TankControl` (0x6AE850), the RC Tiger's gun
//! and the Rhino's turret / cannon. Ported from the 1.0 US bytes; gta-reversed is a reading aid only (where it
//! differs, the bytes are followed: the on-a-string camera snaps the turret's heading to its target when within a
//! step, and the shell's offsets keep the bytes' rounding).

use sa_core::abi::HookReg;
use sa_core::bindings::{get_pad, matrix_mul_vec};
use sa_core::field::{gf, gr32, gr8, ptr_of as vp};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::{gt, lt};
use sa_math::vector::CVector;
use sa_math::x87::X87;
use sa_world::world::common::entity_pos;

use crate::common::{f32_of, rd, wr, x, xi};
use crate::mvpd_v1::read_vec;

use crate::bindings::get_car_gun_left_right;
use crate::bindings::weapon_do_tank_doom_aiming as do_tank_doom_aiming;
use sa_pop::bindings::{car_gun_just_down, get_car_gun_up_down};
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::{
    add_explosion, add_gunshell, apply_force, find_player_ped, fire_instant_hit, get_pad_from_player, max_f,
    multiply3x3, multiply3x3_vm, physical_get_speed as get_speed, process_line_of_sight, report_weapon_event,
    vector_add_assign, vector_mul_scalar, vector_scale, vector_sub, weapon_ctor,
};
use sa_rwcore::bindings::frame_get_ltm;

orig! {
    /// `FxManager_c::TriggerTankFire(CVector& pos, CVector& dir)` (0x4A0FA0, thiscall on `g_fx` 0xA9AE00)
    fn fx_trigger_tank_fire(fx: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>) = thiscall 0x4A0FA0 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7_CB84;
/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7_CB5C;
/// pi, 2 pi
const PI: usize = 0x85_8CB8;
const TWO_PI: usize = 0x85_8CBC;
/// The turret's lowest elevation, -7 degrees.
const H_MIN: usize = 0x87_1078;

/// The RC Tiger (model 0x234) under remote control: a just-pressed car gun after `m_nGunFiringTime + [0x8D316C]`
/// fires the minigun (`CWeapon(0x26, 5000)`) from `matrix * TIGER_GUN_POS (0x8D31A4) + timeStep * moveSpeed` (each
/// product stored as an f32 first), a gun shell (direction (0, 0.1), size 0.025) and the weapon audio event.
fn rc_tiger(this: GPtr<u8>) {
    if car_gun_just_down(get_pad(0)) == 0 {
        return;
    }
    if gr32(TIME_MS) <= rd::<u32>(this, 0x4D4).wrapping_add(gr32(0x8D_316C)) {
        return;
    }
    let mut weapon = [0u32; 7];
    weapon_ctor(vp(&mut weapon), 0x26, 0x1388);
    let ts = gf(TIME_STEP);
    let ms = [f32_of(x(ts).fmul_m(rd::<f32>(this, 0x44))), f32_of(x(ts).fmul_m(rd::<f32>(this, 0x48))), f32_of(x(ts).fmul_m(rd::<f32>(this, 0x4C)))];
    let mut gun = read_vec(GPtr::from_addr(0x8D_31A4));
    let mut out = CVector::default();
    let o = read_vec(matrix_mul_vec(vp(&mut out), rd::<GPtr<u8>>(this, 0x14), vp(&mut gun)));
    let mut p = CVector::new(f32_of(x(ms[0]).fadd_m(o.x)), f32_of(x(ms[1]).fadd_m(o.y)), f32_of(x(ms[2]).fadd_m(o.z)));
    fire_instant_hit(vp(&mut weapon), this, vp(&mut p), vp(&mut p), GPtr::NULL, GPtr::NULL, GPtr::NULL, 0, 1);
    let mut dir = [0.0f32, f32::from_bits(0x3DCC_CCCD)];
    add_gunshell(vp(&mut weapon), this, vp(&mut p), GPtr::from_raw(dir.as_mut_ptr()), f32::from_bits(0x3CCC_CCCD));
    report_weapon_event(GPtr::from_addr(0xB6_BC90), 0x91, 0x26, this.addr() as u32);
    wr::<u32>(this, 0x4D4, gr32(TIME_MS));
}

/// The turret follows the on-a-string camera: the camera's front in the tank's space gives the target heading
/// `atan2(-x, y)` (brought within pi of the current one) and elevation `atan2(z, |xy|) + 15 degrees` (stored f32);
/// each moves by at most `timeStep * 0.015` / `timeStep * 0.005` and takes the target when within that step.
fn follow_camera(this: GPtr<u8>, cam: usize) {
    let mut front = read_vec(GPtr::from_addr(cam + 0xB6_F32C));
    let mut out = CVector::default();
    let fd = read_vec(multiply3x3_vm(vp(&mut out), vp(&mut front), rd::<GPtr<u8>>(this, 0x14)));
    let mut v = x(fd.x).fchs().fpatan(x(fd.y));
    let m2d = x(fd.x).fmul_m(fd.x).fadd(x(fd.y).fmul_m(fd.y)).fsqrt();
    let hf = f32_of(x(fd.z).fpatan(m2d).fadd_m(gf(0x87_1068)));
    let f = || rd::<f32>(this, 0x94C);
    // `fcompp; test ah, 0x41; jne`: above f + pi wraps down; else below f - pi (`test ah, 5; jp`) wraps up
    if gt(v, x(f()).fadd_m(gf(PI))) {
        v = v.fsub_m(gf(TWO_PI));
    } else if lt(v, x(f()).fsub_m(gf(PI))) {
        v = v.fadd_m(gf(TWO_PI));
    }
    let d = v.fsub_m(f());
    let t = f32_of(x(gf(TIME_STEP)).fmul_m(gf(0x87_106C)));
    let nv = if gt(d, x(t)) {
        x(t).fadd_m(f())
    } else if lt(d, x(t).fchs()) {
        x(f()).fsub_m(t)
    } else {
        v
    };
    wr::<f32>(this, 0x94C, f32_of(nv));
    let hd = x(hf).fsub_m(rd::<f32>(this, 0x950));
    let t2 = f32_of(x(gf(TIME_STEP)).fmul_m(gf(0x87_1070)));
    if gt(hd, x(t2)) {
        wr::<f32>(this, 0x950, f32_of(x(t2).fadd_m(rd::<f32>(this, 0x950))));
    } else if lt(hd, x(t2).fchs()) {
        wr::<f32>(this, 0x950, f32_of(x(rd::<f32>(this, 0x950)).fsub_m(t2)));
    } else {
        wr::<f32>(this, 0x950, hf);
    }
}

/// `pad input * timeStep * k * (1 / 128)` in the bytes' order (`fild`, three `fmul`).
fn stick(v: i16, k: usize) -> X87 {
    xi(i32::from(v)).fmul_m(gf(TIME_STEP)).fmul_m(gf(k)).fmul_m(gf(0x85_8B88))
}

/// The heading wrap to (-pi, pi] and the elevation limits: at most 35 degrees; with the turret over the back
/// (heading beyond +-90 degrees) at least `-7 - max(-1, cos(heading) * 1.3) * 4` degrees, else at least -7.
fn limit_turret(this: GPtr<u8>) {
    let f = x(rd::<f32>(this, 0x94C));
    if lt(f, x(gf(0x85_8CC0))) {
        wr::<f32>(this, 0x94C, f32_of(f.fadd_m(gf(TWO_PI))));
    } else if gt(f, x(gf(PI))) {
        wr::<f32>(this, 0x94C, f32_of(f.fsub_m(gf(TWO_PI))));
    }
    let g = x(rd::<f32>(this, 0x950));
    if gt(g, x(gf(0x87_1074))) {
        wr::<u32>(this, 0x950, gr32(0x87_1074));
        return;
    }
    let f = x(rd::<f32>(this, 0x94C));
    if gt(f, x(gf(0x85_8FE4))) || lt(f, x(gf(0x85_9998))) {
        let c = f32_of(f.fcos().fmul_m(gf(0x85_9918)));
        let k = x(gf(0x87_107C)).fsub_m(gf(H_MIN));
        let val = max_f(-1.0, c).fmul(k).fsubr_m(gf(H_MIN));
        if gt(val, x(rd::<f32>(this, 0x950))) {
            wr::<f32>(this, 0x950, f32_of(val));
        }
    } else if lt(g, x(gf(H_MIN))) {
        wr::<u32>(this, 0x950, gr32(H_MIN));
    }
}

/// The Rhino's cannon: the barrel direction `matrix (rotation) * (sin(-heading), cos(heading), sin(elevation))`, the
/// muzzle `T` at the turret node's (+0x6A0) position moved by `GetSpeed(T - position) * timeStep` (without the node
/// the original computes `matrix * offset` into a temporary it never reads and uses an uninitialised `T`: zeros
/// here, the Rhino always has the node), the recoil `ApplyForce(dir * (-0.1 * mass), T - position, true)`, the aim
/// correction, the line of sight to `T + dir * 60` (ignoring the tank), the explosion (type 10, by the player) at the
/// end or 4% back from the hit, and the tank fire effect.
fn fire_cannon(this: GPtr<u8>) {
    let f = x(rd::<f32>(this, 0x94C));
    let mut pt = CVector::new(f32_of(f.fchs().fsin()), f32_of(f.fcos()), f32_of(x(rd::<f32>(this, 0x950)).fsin()));
    let mut out = CVector::default();
    let mut dir = read_vec(multiply3x3(vp(&mut out), rd::<GPtr<u8>>(this, 0x14), vp(&mut pt)));
    let mut t = CVector::default();
    let node = rd::<GPtr<u8>>(this, 0x6A0);
    if node.is_null() {
        let fv = x(rd::<f32>(this, 0x94C));
        let (s, c) = (fv.fsin(), fv.fcos());
        let (dx, dy, dz) = (gf(0xC1_C208), gf(0xC1_C20C), gf(0xC1_C210));
        let ox = f32_of(x(dx).fmul(c).fsub(s.fmul_m(dy)));
        let oy = c.fmul_m(dy).fadd(s.fmul_m(dx));
        let oz = x(dz).fsub_m(1.0);
        let mut off = CVector::new(
            f32_of(x(ox).fadd_m(gf(0x8D_318C))),
            f32_of(x(gf(0x8D_3190)).fadd(oy)),
            f32_of(x(gf(0x8D_3194)).fadd(oz)),
        );
        let mut unread = CVector::default();
        matrix_mul_vec(vp(&mut unread), rd::<GPtr<u8>>(this, 0x14), vp(&mut off));
    } else {
        t = read_vec(frame_get_ltm(node).byte_add(0x30));
        let mut rel = CVector::default();
        let r = read_vec(vector_sub(vp(&mut rel), vp(&mut t), entity_pos(this)));
        let mut sp = CVector::default();
        let mut s = read_vec(get_speed(this, GPtr::from_raw(&mut sp), r).cast());
        let mut tmp = CVector::default();
        let st = vector_mul_scalar(vp(&mut tmp), vp(&mut s), gf(TIME_STEP));
        vector_add_assign(vp(&mut t), st);
    }
    let mut rel = CVector::default();
    let dist = read_vec(vector_sub(vp(&mut rel), vp(&mut t), entity_pos(this)));
    let k = f32_of(x(gf(0x87_1080)).fmul_m(rd::<f32>(this, 0x8C)));
    let mut fo = CVector::default();
    let force = read_vec(vector_scale(vp(&mut fo), k, vp(&mut dir)));
    apply_force(this, force, dist, 1);
    let sixty = gf(0x85_8B34);
    let dz60 = f32_of(x(dir.z).fmul_m(sixty));
    let mut end = CVector::new(
        f32_of(x(dir.x).fmul_m(sixty).fadd_m(t.x)),
        f32_of(x(t.y).fadd(x(dir.y).fmul_m(sixty))),
        f32_of(x(t.z).fadd_m(dz60)),
    );
    do_tank_doom_aiming(this, rd::<GPtr<u8>>(this, 0x460), vp(&mut t), vp(&mut end));
    let mut hit = GPtr::<u8>::NULL;
    let mut cp = [0u32; 11];
    GPtr::<GPtr<u8>>::from_addr(0xB7_CD68).write(this);
    process_line_of_sight(vp(&mut t), vp(&mut end), vp(&mut cp), vp(&mut hit), 1, 1, 1, 1, 1, 1, 0, 0);
    GPtr::<u32>::from_addr(0xB7_CD68).write(0);
    if !hit.is_null() {
        let c = [f32::from_bits(cp[0]), f32::from_bits(cp[1]), f32::from_bits(cp[2])];
        let k4 = gf(0x85_8CEC);
        let dx4 = f32_of(x(c[0]).fsub_m(t.x).fmul_m(k4));
        let dy4 = x(c[1]).fsub_m(t.y).fmul_m(k4);
        let dz4 = x(f32_of(x(c[2]).fsub_m(t.z))).fmul_m(k4);
        end = CVector::new(f32_of(x(c[0]).fsub_m(dx4)), f32_of(x(c[1]).fsub(dy4)), f32_of(x(c[2]).fsub(dz4)));
    }
    add_explosion(GPtr::NULL, find_player_ped(-1), 0xA, end.x, end.y, end.z, 0, 1, -1.0, 0);
    let mut d = CVector::new(f32_of(x(end.x).fsub_m(t.x)), f32_of(x(end.y).fsub_m(t.y)), f32_of(x(end.z).fsub_m(t.z)));
    fx_trigger_tank_fire(GPtr::from_addr(0xA9_AE00), vp(&mut t), vp(&mut d));
}

/// `CAutomobile::TankControl()` (0x6AE850, thiscall, plain `ret`).
///
/// The RC Tiger under remote control fires its minigun ([`rc_tiger`]). Otherwise only the player's Rhino (status 0,
/// model 0x1B0) in game state 0 (0x96A8B0) with a player driver and a pad: the turret follows the on-a-string
/// camera (mode 0x12, [`follow_camera`]) or the pad's car gun axes (heading -= left/right * timeStep * 0.015 / 128,
/// elevation += up/down * timeStep * 0.005 / 128), is limited ([`limit_turret`]), and a just-pressed car gun more
/// than 800 ms after the focused player's last big gun shot (CPlayerInfo +0x100) fires the cannon ([`fire_cannon`]).
#[hook(addr = 0x6AE850, conv = thiscall, name = "CAutomobile::TankControl", this = "CAutomobile",
       expect = "81 EC 8C 00 00 00 56", unverified = "frontier2-b23", route = "L2", hazards = "heap-writes",
       note = "explosions, fx and the weapon line of sight; the no-turret-node path's uninitialised muzzle reads 0")]
pub fn tank_control(this: GPtr<u8>) {
    let model = rd::<u16>(this, 0x22);
    if model == 0x234 && rd::<u8>(this, 0x36) & 0xF8 == 0x40 {
        rc_tiger(this);
        return;
    }
    if rd::<u8>(this, 0x36) & 0xF8 != 0 || model != 0x1B0 || gr8(0x96_A8B0) != 0 {
        return;
    }
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if driver.is_null() || ped_is_player(driver) == 0 {
        return;
    }
    let pad = get_pad_from_player(rd::<GPtr<u8>>(this, 0x460));
    if pad.is_null() {
        return;
    }
    let cam = usize::from(gr8(0xB6_F081)) * 0x238;
    if GPtr::<u16>::from_addr(cam + 0xB6_F1A8).read() == 0x12 {
        follow_camera(this, cam);
    } else {
        let lr = get_car_gun_left_right(pad) as i16;
        wr::<f32>(this, 0x94C, f32_of(stick(lr, 0x87_106C).fsubr_m(rd::<f32>(this, 0x94C))));
        let ud = get_car_gun_up_down(pad);
        wr::<f32>(this, 0x950, f32_of(stick(ud, 0x87_1070).fadd_m(rd::<f32>(this, 0x950))));
    }
    limit_turret(this);
    if car_gun_just_down(pad) == 0 {
        return;
    }
    let last = GPtr::<u32>::from_addr(0xB7_CE84 + usize::from(gr8(0xB7_CD74)) * 0x190);
    let now = gr32(TIME_MS);
    if now <= last.read().wrapping_add(0x320) {
        return;
    }
    last.write(now);
    fire_cannon(this);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_TANK_CONTROL];
