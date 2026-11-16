//! FRONTIER-2 turn 4, lane B (batch `frontier2-b12`): `CHeli::ProcessControl` and what it reaches. Ported from the
//! 1.0 US bytes (gta-reversed only calls the original); unverified (docs/verify/HOWTO.md).

use sa_core::abi::{HookReg, NativeReg};
use sa_core::field::v3_ptr as vp;
use sa_core::mem::GPtr;
use sa_core::{hook, native, orig};
use sa_math::fpu::{ge_or_nan, gt, le_or_nan, lt};
use sa_math::x87::X87;

use crate::bindings::{automobile_process_control, weapon_fire_one_instant_hit_round};
use crate::common::{f32_of, g, rand_at, rd, wr, x, xi};
use crate::air2::process_weapons as vehicle_process_weapons;
use crate::mvpd_v6::horn_just_down;
use sa_core::bindings::{
    general_random_int_in_range, matrix_mul_vec, random_float_in_range, task_new, vector_magnitude, vector_normalise,
    world_get_is_line_of_sight_clear,
};
use sa_pop::bindings::add_ped;
use sa_input::bindings::get_pad;
use sa_pop::carctrl3::{time_ms as now, time_step};
use sa_world::bindings::{
    cull_zones_no_police, find_player_entity, find_player_ped, find_player_vehicle, find_player_wanted, fx_kill,
    get_at_direction, interesting_events_add, multiply3x3, placeable_get_top_direction, player_no_rain,
    report_weapon_event, ropes_register_rope, anim_blend_animation, find_ground_z_for_3d_coord, get_water_level_no_waves,
    matrix_copy_ctor, matrix_dtor, sequence_add_task, sequence_ctor, task_wander_cop_ctor, tm_set_task, vector_add_assign,
};
use sa_world::world::common::entity_pos;

orig! {
    /// `CAudioEngine::SayPedless(event, ctx, attachTo, delay, probability, overrideSilence, forceAudible, frontEnd)`
    /// (0x5073C0)
    fn say_pedless(engine: GPtr<u8>, event: u32, ctx: u32, attach: GPtr<u8>, delay: u32, prob: f32, ov: u32, force: u32,
                   front: u32) = thiscall 0x5073C0 => unported;
    /// `CHeli::SendDownSwat()` (0x6C69C0)
    fn send_down_swat(h: GPtr<u8>) -> u8 = thiscall 0x6C69C0 => crate::frontier_b4_heli::heli_send_down_swat;
    /// `CTaskComplexUseSwatRope::CTaskComplexUseSwatRope(uint32 ropeId, CHeli* heli)` (0x659470)
    fn task_use_swat_rope_ctor(t: GPtr<u8>, rope: GPtr<u8>, heli: GPtr<u8>) -> GPtr<u8> = thiscall 0x659470 => unported;
    /// `CHeli::FindSwatPositionRelativeToHeli(CVector* out, int32 i)` (0x6C4760; callers keep EDX: not hookable, its
    /// phase B native below)
    fn heli_find_swat_position(h: GPtr<u8>, out: GPtr<u8>, i: u32) -> GPtr<f32> = thiscall 0x6C4760 => crate::frontier_b4_heli::heli_find_swat_position_native;
    /// `CRopes::SetSpeedOfTopNode(uint32 id, CVector speed)` (0x555DF0)
    fn ropes_set_speed_of_top_node(id: GPtr<u8>, sx: f32, sy: f32, sz: f32) = cdecl 0x555DF0 => unported;
    /// `CVehicle::DoBladeCollision(CVector pos, CMatrix& matrix, int16 rotorType, float radius, float damageMult)`
    /// (0x6E2E50)
    fn do_blade_collision(v: GPtr<u8>, px: f32, py: f32, pz: f32, m: GPtr<u8>, rotor: u32, radius: f32, mult: u32) -> u8 = thiscall 0x6E2E50 => crate::vehicle_blade_collision::vehicle_do_blade_collision;
    /// `CWindModifiers::RegisterOne(CVector pos, int32 type, float power)` (0x72C8B0)
    fn wind_register_one(px: f32, py: f32, pz: f32, ty: i32, power: f32) = cdecl 0x72C8B0 => unported;
    /// `CVehicle::UpdateWinch()` (0x6D3B80)
    fn vehicle_update_winch(v: GPtr<u8>) = thiscall 0x6D3B80 => crate::vehicle2::update_winch;
}

/// `CHeli::FindSwatPositionRelativeToHeli(CVector* out, int32 i)` (0x6C4760, leaf, `ret 8`): the rope position `i` of
/// the four (x -1.2 / 1.2, y -1 / 1, z -0.5, in the bytes' order 0..3: (-x, -y), (x, -y), (-x, y), (x, y)); `i` above 3
/// (unsigned) gives the zero vector. Returns `out`. Phase B native only: callers keep EDX across the call. The stores
/// are the bytes' immediate bit patterns.
#[native(addr = 0x6C4760, conv = thiscall, name = "CHeli::FindSwatPositionRelativeToHeli")]
pub fn heli_find_swat_position_native(_h: GPtr<u8>, out: GPtr<u8>, i: u32) -> GPtr<f32> {
    const NEG_X: u32 = 0xBF99_999A; // -1.2
    const POS_X: u32 = 0x3F99_999A; // 1.2
    const NEG_Y: u32 = 0xBF80_0000; // -1.0
    const POS_Y: u32 = 0x3F80_0000; // 1.0
    const Z: u32 = 0xBF00_0000; // -0.5
    let v = match i {
        0 => [NEG_X, NEG_Y, Z],
        1 => [POS_X, NEG_Y, Z],
        2 => [NEG_X, POS_Y, Z],
        3 => [POS_X, POS_Y, Z],
        _ => [0, 0, 0],
    };
    wr::<u32>(out, 0, v[0]);
    wr::<u32>(out, 4, v[1]);
    wr::<u32>(out, 8, v[2]);
    out.cast()
}

/// `CHeli::ProcessControl()` (0x6C7050, vtable slot). `CAutomobile::ProcessControl`, then:
/// - without engine flag +0x428 bit 4 the gun-flash FX +0x580 is killed (+0x580, +0x984 = 0);
/// - the horn of the driver's pad (pad 1 for a driver of ped type 1) toggles the search light +0xA11;
/// - wrecked (+0x40 bit 29) or `CCullZones::PlayerNoRain`: light intensity +0xA00 = 0. Otherwise the target: mission
///   0x28 (police heli) the player entity unless the player sits in a heli / plane (then it may shoot: `shoot`);
///   mission 0x27 the autopilot's target car +0x41C when handling flag +0x988 bit 1; a player-driven police maverick
///   (497) with the light on: no target (a point 30 ahead and 10 below). Flag +0x40 bit 8 clears both. The light flag
///   +0xA11 = target found;
/// - with a target: every elapsed second (+0x9F0) the six-entry history of target position + speed * 100
///   (+0x9C0 x, +0x9D8 y) shifts; the light point (+0x9F4, +0x9F8) is the history blended by the second's fraction,
///   +0x9FC the target z; intensity +0xA00 1.0 under 40 m, falling 0.05 per m to 0 at 60 m; under 0.9 or farther than
///   7 m from the light point the sight timer +0xA04 / +0x504 restarts, else +0xA08 gets a random new time;
/// - shooting (police heli at a player target, wanted flags clear): the delay by wanted level (999999 / 10000 / 5000 /
///   3500 / 2000 ms, halved in a no-police zone, 5000 for a non-player target), a line-of-sight check when the delay
///   ends, then `CWeapon::FireOneInstantHitRound` (damage 20) from 3.5 ahead / 1 below toward the target jittered by
///   +-2.56, and the next shot 150 or 400 ms later;
/// - the swat ropes (mission 0x28: `SendDownSwat`), the four rope timers +0x9BA.. and their top-node speeds, the
///   winch, the weapons and a random interesting event 0x15 (twice the chance while shooting).
#[hook(addr = 0x6C7050, conv = thiscall, name = "CHeli::ProcessControl", this = "CHeli", expect = "83 EC 54 53 55 56 57 8B F1",
       unverified = "frontier2-b12", route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "rand at_site 0x6C748E / 0x6C764D / 0x6C7672 / 0x6C7952; FSQRT of the light distance; reads an unwritten local only on a path that never uses it; fires a round, registers ropes")]
pub fn heli_process_control(this: GPtr<u8>) {
    automobile_process_control(this);
    if rd::<u8>(this, 0x428) & 0x10 == 0 && !rd::<GPtr<u8>>(this, 0x580).is_null() {
        fx_kill(rd::<GPtr<u8>>(this, 0x580));
        wr::<u32>(this, 0x580, 0);
        wr::<u32>(this, 0x984, 0);
    }
    let driver = rd::<GPtr<u8>>(this, 0x460);
    let pad = i32::from(!driver.is_null() && rd::<u32>(driver, 0x598) == 1);
    if horn_just_down(get_pad(pad)) != 0 {
        wr::<u8>(this, 0xA11, u8::from(rd::<u8>(this, 0xA11) == 0));
    }
    // `shoot` = [esp+0x13], `light` = bl, `target` = ebp / [esp+0x14]
    let mut shoot = false;
    'light: {
        if rd::<u32>(this, 0x40) & 0x2000_0000 != 0 || player_no_rain() != 0 {
            wr::<u32>(this, 0xA00, 0);
            break 'light;
        }
        let mission = rd::<u8>(this, 0x3BA);
        let mut light = false;
        let mut target = GPtr::<u8>::NULL;
        let player_in_air = || {
            !find_player_vehicle(-1, 0).is_null()
                && (rd::<u32>(find_player_vehicle(-1, 0), 0x594) == 3 || rd::<u32>(find_player_vehicle(-1, 0), 0x594) == 4)
        };
        if mission == 0x28 && !player_in_air() {
            light = true;
            shoot = true;
            target = find_player_entity(-1);
        } else if mission == 0x27 && !rd::<GPtr<u8>>(this, 0x41C).is_null() && rd::<u8>(this, 0x988) & 2 != 0 {
            target = rd::<GPtr<u8>>(this, 0x41C);
            light = true;
        } else if rd::<u8>(this, 0x36) & 0xF8 == 0 && rd::<u16>(this, 0x22) == 0x1F1 && rd::<u8>(this, 0xA11) != 0 {
            light = true;
        }
        if rd::<u32>(this, 0x40) & 0x100 != 0 {
            light = false;
            shoot = false;
        }
        wr::<u8>(this, 0xA11, u8::from(light));
        if !light {
            break 'light;
        }
        // p = the target point [esp+0x1C], v = its speed [esp+0x40]
        let mut p = [0f32; 3];
        let speed_of;
        if !target.is_null() {
            let tp = entity_pos(target);
            p = [rd::<f32>(tp, 0), rd::<f32>(tp, 4), rd::<f32>(tp, 8)];
            speed_of = target;
        } else {
            let mut at = [0f32; 3];
            let a = get_at_direction(this, GPtr::from_raw(at.as_mut_ptr())).cast::<u8>();
            let ax = f32_of(x(rd::<f32>(a, 0)).fmul_m(-30.0));
            let ay = f32_of(x(rd::<f32>(a, 4)).fmul_m(-30.0));
            let az = f32_of(x(rd::<f32>(a, 8)).fmul_m(-30.0));
            let mut top = [0f32; 3];
            let t = placeable_get_top_direction(this, GPtr::from_raw(top.as_mut_ptr())).cast::<u8>();
            let tx = x(rd::<f32>(t, 0)).fmul_m(10.0);
            let ty = f32_of(x(rd::<f32>(t, 4)).fmul_m(10.0));
            let tz = f32_of(x(rd::<f32>(t, 8)).fmul_m(10.0));
            let pos = entity_pos(this);
            let sx = tx.fadd_m(rd::<f32>(pos, 0));
            let sy = x(ty).fadd_m(rd::<f32>(pos, 4));
            let sz = f32_of(x(tz).fadd_m(rd::<f32>(pos, 8)));
            p[0] = f32_of(sx.fadd_m(ax));
            p[1] = f32_of(sy.fadd_m(ay));
            p[2] = f32_of(x(sz).fadd_m(az));
            speed_of = this;
        }
        let v = [rd::<f32>(speed_of, 0x44), rd::<f32>(speed_of, 0x48), rd::<f32>(speed_of, 0x4C)];
        let mut elapsed = now().wrapping_sub(rd::<u32>(this, 0x9F0)) as i32;
        if elapsed > 1000 {
            let vx = x(v[0]).fmul_m(100.0);
            let vy = x(v[1]).fmul_m(100.0);
            let n = (elapsed.wrapping_sub(1001) as u32) / 1000 + 1;
            elapsed = elapsed.wrapping_add((n as i32).wrapping_mul(-1000));
            for _ in 0..n {
                for k in (1..6isize).rev() {
                    wr::<u32>(this, 0x9C0 + 4 * k, rd::<u32>(this, 0x9C0 + 4 * (k - 1)));
                    wr::<u32>(this, 0x9D8 + 4 * k, rd::<u32>(this, 0x9D8 + 4 * (k - 1)));
                }
                wr::<u32>(this, 0x9F0, rd::<u32>(this, 0x9F0).wrapping_add(1000));
                wr::<f32>(this, 0x9C0, f32_of(vx.fadd_m(p[0])));
                wr::<f32>(this, 0x9D8, f32_of(x(p[1]).fadd(vy)));
            }
        }
        let f = xi(elapsed).fmul_m(f32::from_bits(0x3A83_126F));
        wr::<f32>(this, 0x9FC, p[2]);
        let one_f = x(1.0).fsub(f);
        let sx = one_f.fmul_m(rd::<f32>(this, 0x9C8)).fadd(f.fmul_m(rd::<f32>(this, 0x9C4)));
        let sx_f = f32_of(sx);
        wr::<f32>(this, 0x9F4, sx_f);
        let sy = one_f.fmul_m(rd::<f32>(this, 0x9E0)).fadd(f.fmul_m(rd::<f32>(this, 0x9DC)));
        wr::<f32>(this, 0x9F8, f32_of(sy));
        let pos = entity_pos(this);
        let ddy = sy.fsub_m(rd::<f32>(pos, 4));
        let ddx = x(sx_f).fsub_m(rd::<f32>(pos, 0));
        let dist = ddy.fmul(ddy).fadd(ddx.fmul(ddx)).fsqrt();
        let dist_f = f32_of(dist);
        if !le_or_nan(dist, x(60.0)) {
            wr::<u32>(this, 0xA00, 0);
        } else if !ge_or_nan(x(dist_f), x(40.0)) {
            wr::<f32>(this, 0xA00, 1.0);
        } else {
            wr::<f32>(this, 0xA00, f32_of(x(dist_f).fsub_m(40.0).fmul_m(f32::from_bits(0x3D4C_CCCD)).fsubr_m(1.0)));
        }
        let dx = f32_of(x(p[0]).fsub_m(sx_f));
        let dy = x(p[1]).fsub(sy);
        let restart = if lt(x(rd::<f32>(this, 0xA00)), x(f32::from_bits(0x3F66_6666))) {
            true
        } else {
            gt(dy.fmul(dy).fadd(x(dx).fmul_m(dx)), x(49.0))
        };
        if restart {
            let t = now();
            wr::<u32>(this, 0xA04, t);
            wr::<u32>(this, 0x504, t);
        } else {
            let t = now();
            if t > rd::<u32>(this, 0xA08) {
                let r = rand_at(0x6C_748E);
                wr::<u32>(this, 0xA08, ((r & 0xFFF) as u32).wrapping_add(t).wrapping_add(0x1194));
            }
        }
        if !shoot {
            break 'light;
        }
        let wanted_level = || {
            let data = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
            let w = if data.is_null() { GPtr::<u8>::NULL } else { rd::<GPtr<u8>>(data, 0) };
            rd::<u32>(w, 0x2C)
        };
        let mut delay: i32 = match wanted_level() {
            0..=2 => 999_999,
            3 => 10_000,
            4 => 5000,
            5 => 3500,
            6 => 2000,
            _ => dx.to_bits() as i32,
        };
        if wanted_level() != 0 {
            say_pedless(GPtr::from_addr(0xB6_BC90), 0x34, 0xAE, this, 0, 1.0, 0, 0, 0);
        }
        if cull_zones_no_police() != 0 {
            delay /= 2;
        }
        if target != find_player_ped(-1) {
            delay = 5000;
        }
        let delay = delay as u32;
        if rd::<u8>(find_player_wanted(-1), 0x1E) & 7 != 0 {
            let t = now();
            wr::<u32>(this, 0xA04, t);
            wr::<u32>(this, 0x504, t);
            break 'light;
        }
        let mut off = [0.0, f32::from_bits(0x4060_0000), -1.0];
        let mut gun = [0f32; 3];
        matrix_mul_vec(vp(&mut gun), rd::<GPtr<u8>>(this, 0x14), vp(&mut off));
        let due = rd::<u32>(this, 0xA04).wrapping_add(delay);
        if now() > due && g::<u32>(0xB7_CB78).read() <= due && world_get_is_line_of_sight_clear(vp(&mut gun), vp(&mut p), 1, 0, 0, 0, 0, 0, 0) == 0 {
            let t = now();
            wr::<u32>(this, 0xA04, t);
            wr::<u32>(this, 0x504, t);
        }
        let t = now();
        if t <= rd::<u32>(this, 0xA04).wrapping_add(delay) || t <= rd::<u32>(this, 0x504) {
            break 'light;
        }
        let mut tgt = p;
        let r = (rand_at(0x6C_764D) & 0xFF) - 0x80;
        tgt[0] = f32_of(xi(r).fmul_m(f32::from_bits(0x3CA3_D70A)).fadd_m(tgt[0]));
        let r = (rand_at(0x6C_7672) & 0xFF) - 0x80;
        tgt[1] = f32_of(xi(r).fmul_m(f32::from_bits(0x3CA3_D70A)).fadd_m(tgt[1]));
        let mut dir = [
            f32_of(x(p[0]).fsub_m(gun[0])),
            f32_of(x(p[1]).fsub_m(gun[1])),
            f32_of(x(p[2]).fsub_m(gun[2])),
        ];
        vector_normalise(GPtr::from_raw(&mut dir as *mut [f32; 3]));
        let a = x(dir[0]).fmul_m(3.0);
        let a_f = f32_of(a);
        let b = x(dir[1]).fmul_m(3.0);
        let c = x(dir[2]).fmul_m(3.0);
        let c_f = f32_of(c);
        tgt[0] = f32_of(x(a_f).fadd_m(tgt[0]));
        tgt[1] = f32_of(b.fadd_m(tgt[1]));
        tgt[2] = f32_of(x(tgt[2]).fadd(c));
        let mut start = [
            f32_of(x(a_f).fadd_m(gun[0])),
            f32_of(x(gun[1]).fadd(b)),
            f32_of(x(gun[2]).fadd_m(c_f)),
        ];
        weapon_fire_one_instant_hit_round(GPtr::from_raw(start.as_mut_ptr()).cast(), GPtr::from_raw(tgt.as_mut_ptr()).cast(), 0x14);
        report_weapon_event(GPtr::from_addr(0xB6_BC90), 0x91, 0x1F, this.addr() as u32);
        let roll: X87 = random_float_in_range(0.0, 1.0);
        let next = if ge_or_nan(roll, x(g::<f32>(0x8D_33A4).read())) { 0x96 } else { 0x190 };
        wr::<u32>(this, 0x504, now().wrapping_add(next));
    }
    if rd::<u8>(this, 0x3BA) == 0x28 && rd::<u8>(this, 0x9B9) != 0 {
        send_down_swat(this);
        interesting_events_add(GPtr::from_addr(0xC0_B058), 0x1A, this);
    }
    for i in 0..4u32 {
        let timer = this.byte_add(0x9BA + i as isize);
        let tv = timer.read();
        if tv == 0 {
            continue;
        }
        timer.write(tv - 1);
        let mut rel = [0f32; 3];
        let s = heli_find_swat_position(this, vp(&mut rel), i).cast::<u8>();
        let mut pos = [0f32; 3];
        matrix_mul_vec(vp(&mut pos), rd::<GPtr<u8>>(this, 0x14), s);
        let id = this.byte_add(i as isize);
        ropes_register_rope(id, 8, pos[0], pos[1], pos[2], 0, 0, 0, GPtr::NULL, 20_000);
        if timer.read() == 0 {
            let mut rel = [0f32; 3];
            let s = heli_find_swat_position(this, vp(&mut rel), i).cast::<u8>();
            let mut v = [
                f32_of(x(rd::<f32>(s, 0)).fmul_m(f32::from_bits(0x3D4C_CCCD))),
                f32_of(x(rd::<f32>(s, 4)).fmul_m(f32::from_bits(0x3D4C_CCCD))),
                f32_of(x(rd::<f32>(s, 8)).fmul_m(f32::from_bits(0x3D4C_CCCD))),
            ];
            let mut out = [0f32; 3];
            multiply3x3(vp(&mut out), rd::<GPtr<u8>>(this, 0x14), vp(&mut v));
            ropes_set_speed_of_top_node(id, out[0], out[1], 0.0);
        }
    }
    vehicle_update_winch(this);
    vehicle_process_weapons(this);
    if g::<u8>(0xC0_B184).read() & 1 != 0 {
        let mut chance = f32_of(x(time_step()).fmul_m(f32::from_bits(0x3CA3_D70A)).fmul_m(f32::from_bits(0x3DCC_CCCD)));
        if shoot {
            chance = f32_of(x(chance).fadd(x(chance)));
        }
        let r = rand_at(0x6C_7952);
        if !ge_or_nan(xi(r).fmul_m(f32::from_bits(0x3800_0100)), x(chance)) {
            interesting_events_add(GPtr::from_addr(0xC0_B058), 0x15, this);
        }
    }
}

/// `CHeli::SendDownSwat()` (0x6C69C0, thiscall, AL): one swat member down a rope when swat are left (+0x9B9), flag
/// +0x40 bit 8 is clear, byte 0x8E6314 is 1 and `rand() & 0x7F` is 0 (rand at_site 0x6C6A36), the heli is within 50 of
/// its target car (+0x41C, read unchecked) and moves no faster than 0.1. The rope's top point is the swat position
/// (index count - 1) through a copy of the heli's matrix (`Multiply3x3`) plus the heli position; it needs ground within
/// 2.5 of the target's z and no water at or above that ground. Then rope `this + count - 1` (type 8, 20 s), a cop
/// (`CPopulation::AddPed(6, 3, pos, true)`) with the sequence use-swat-rope + wander-cop (`rand 0..8` direction) as
/// primary task, its +0x1C bit 0 cleared, count - 1, the rope timer +0x9BA[count] = 0xAA and anim 0x98 of group 0
/// blended at 4.0. Returns 1 when one went down.
#[hook(addr = 0x6C69C0, conv = thiscall, name = "CHeli::SendDownSwat", this = "CHeli", expect = "6A FF 64 A1 00 00 00 00 68 F2 84 84 00",
       unverified = "frontier2-b12", route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "rand at_site 0x6C6A36; FSQRT distance; Magnitude / FindGroundZFor3DCoord ST0 read as f64; creates a ped, tasks and a rope")]
pub fn heli_send_down_swat(this: GPtr<u8>) -> u8 {
    let tp = entity_pos(rd::<GPtr<u8>>(this, 0x41C));
    let t = [rd::<f32>(tp, 0), rd::<f32>(tp, 4), rd::<f32>(tp, 8)];
    if rd::<u8>(this, 0x9B9) == 0 || rd::<u32>(this, 0x40) & 0x100 != 0 || g::<u8>(0x8E_6314).read() != 1 {
        return 0;
    }
    if rand_at(0x6C_6A36) & 0x7F != 0 {
        return 0;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let pos = entity_pos(this);
    let dz = x(rd::<f32>(pos, 8)).fsub_m(t[2]);
    let dy = x(rd::<f32>(pos, 4)).fsub_m(t[1]);
    let dx = x(rd::<f32>(pos, 0)).fsub_m(t[0]);
    let d = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt();
    if gt(d, x(50.0)) {
        return 0;
    }
    let mag = X87::from_f64_exact(vector_magnitude(this.byte_add(0x44).cast()));
    if gt(mag, x(0.1)) {
        return 0;
    }
    let mut local = [0u32; 18];
    let lm: GPtr<u8> = GPtr::from_raw(local.as_mut_ptr()).cast();
    matrix_copy_ctor(lm, m);
    let count = rd::<u8>(this, 0x9B9);
    let mut rel = [0f32; 3];
    let s = heli_find_swat_position(this, vp(&mut rel), u32::from(count) - 1);
    let mut r = [0f32; 3];
    multiply3x3(vp(&mut r), lm, s.cast());
    vector_add_assign(vp(&mut r), entity_pos(this));
    let gz = f32_of(X87::from_f64_exact(find_ground_z_for_3d_coord(r[0], r[1], r[2], GPtr::NULL, GPtr::NULL)));
    let mut dzg = x(t[2]).fsub_m(gz);
    if !ge_or_nan(dzg, x(0.0)) {
        dzg = dzg.fchs();
    }
    let mut water = [0f32; 1];
    let ok = !ge_or_nan(dzg, x(2.5))
        && (get_water_level_no_waves(r[0].to_bits(), r[1].to_bits(), r[2].to_bits(), GPtr::from_raw(water.as_mut_ptr()), GPtr::NULL, GPtr::NULL) == 0
            || !ge_or_nan(x(water[0]), x(gz)));
    if !ok {
        matrix_dtor(lm);
        return 0;
    }
    let rope = this.byte_add(isize::from(count) - 1);
    ropes_register_rope(rope, 8, r[0], r[1], r[2], 0, 0, 0, GPtr::NULL, 20_000);
    let ped = add_ped(6, 3, GPtr::from_raw(r.as_mut_ptr()), 1);
    let seq = task_new(0x40);
    let seq = if seq.is_null() { seq } else { sequence_ctor(seq) };
    let t1 = task_new(0x1C);
    let t1 = if t1.is_null() { t1 } else { task_use_swat_rope_ctor(t1, this.byte_add(isize::from(rd::<u8>(this, 0x9B9)) - 1), this) };
    sequence_add_task(seq, t1);
    let t2 = task_new(0x50);
    let t2 = if t2.is_null() {
        t2
    } else {
        let dir = general_random_int_in_range(0, 8);
        task_wander_cop_ctor(t2, 4, dir as u32)
    };
    sequence_add_task(seq, t2);
    tm_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), seq, 3, 0);
    wr::<u32>(ped, 0x1C, rd::<u32>(ped, 0x1C) & 0xFFFF_FFFE);
    let left = rd::<u8>(this, 0x9B9).wrapping_sub(1);
    wr::<u8>(this, 0x9B9, left);
    wr::<u8>(this, 0x9BA + isize::from(left), 0xAA);
    anim_blend_animation(rd::<GPtr<u8>>(ped, 0x18), 0, 0x98, 4.0);
    matrix_dtor(lm);
    1
}

/// `CHeli::ProcessFlyingCarStuff()` (0x6C4E60, vtable slot). Not player / remote / status 8: only with handling flag
/// 0x2000000 (+0x384 +0xCC) the engine flag +0x428 bit 4 is cleared and the rotor speed +0x84C runs down by
/// `0.00055 * timeStep` (to 0; byte +0xB8 cleared while it runs). Driven (status 0 / 3 / 8): the rotor speed rises by
/// 0.001 (0.003 for models 465 / 501) while below 0.22 and +0x40 bit 8 is clear; above 0.15 `FlyingControl` runs
/// (model 2 with +0x42C bit 7; else model 6 with the four control values +0x98C..+0x998, skipped on the ground (+0x960
/// >= 4 wheels, or the parked Leviathan / Skimmer flag) with no input (+0x998 <= 0) and every speed component within
/// 0.02). Above 0.015 with a rotor frame (+0x678): the rotor atomic's bounding sphere (radius > 0.1) cuts with
/// `DoBladeCollision` (rotor type -3, damage by model: 0.9 / 0.8 / 0.5 / 1.0) for status 0 / 8, and the wind
/// modifier registers (power `min(rotor * 6.67, 1)` for status 0 / 3 above 0.0075, 1.0 for status 2). Then (not for
/// 465 / 501, rotor in (0.0044, 0.154)) the camera within 20 of the heli and the rotor turned more than 30 degrees since
/// the last chop (+0x958 vs +0x82C): when the rotor's right vector faces the camera (|dot| > 0.95) the chop sound 0x5E
/// plays and +0x958 = +0x82C.
#[hook(addr = 0x6C4E60, conv = thiscall, name = "CHeli::ProcessFlyingCarStuff", this = "CHeli", expect = "64 A1 00 00 00 00 6A FF 68 6E 84 84 00",
       unverified = "frontier2-b12", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "FSQRT of the camera distance; the stack CMatrix copies live on the Rust stack; the SEH frame is not reproduced")]
pub fn heli_process_flying_car_stuff(this: GPtr<u8>) {
    use crate::mvpb_a1::flying_control;
    use sa_core::bindings::{frame_for_all_objects, matrix_attach, rp_atomic_resync_interpolated_sphere};
    use sa_world::bindings::audio_add_event;
    let status = rd::<u8>(this, 0x36) >> 3;
    let ts = time_step();
    if status != 0 && status != 8 && status != 3 {
        if rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) & 0x200_0000 == 0 {
            return;
        }
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xEF);
        let d = x(ts).fmul_m(f32::from_bits(0x3A10_2DE0));
        if ge_or_nan(d, x(rd::<f32>(this, 0x84C))) {
            wr::<u32>(this, 0x84C, 0);
        } else {
            wr::<u8>(this, 0xB8, 0);
            wr::<f32>(this, 0x84C, f32_of(x(rd::<f32>(this, 0x84C)).fsub(d)));
        }
    } else {
        let model = rd::<u16>(this, 0x22);
        if !ge_or_nan(x(rd::<f32>(this, 0x84C)), x(f32::from_bits(0x3E61_47AE))) && rd::<u32>(this, 0x40) & 0x100 == 0 {
            let inc = if model == 0x1D1 || model == 0x1F5 { f32::from_bits(0x3B44_9BA6) } else { f32::from_bits(0x3A83_126F) };
            wr::<f32>(this, 0x84C, f32_of(x(rd::<f32>(this, 0x84C)).fadd_m(inc)));
        }
        if !le_or_nan(x(rd::<f32>(this, 0x84C)), x(f32::from_bits(0x3E19_999A))) {
            let parked = rd::<u32>(this, 0x40) & 0x800_0000 != 0 && (model == 0x1BF || model == 0x1A1);
            let ctl = |o: isize| rd::<f32>(this, o);
            if rd::<u8>(this, 0x42C) & 0x80 != 0 {
                flying_control(this, 2, ctl(0x98C), ctl(0x990), ctl(0x994), ctl(0x998));
            } else {
                let k = f32::from_bits(0x3CA3_D70A);
                let still = |o: isize| le_or_nan(x(rd::<f32>(this, o)).fabs(), x(k));
                let skip = (rd::<u8>(this, 0x960) >= 4 || parked)
                    && le_or_nan(x(ctl(0x998)), X87::from_f64_exact(0.0))
                    && still(0x44)
                    && still(0x48)
                    && still(0x4C);
                if !skip {
                    flying_control(this, 6, ctl(0x98C), ctl(0x990), ctl(0x994), ctl(0x998));
                }
            }
        }
        let frame = rd::<GPtr<u8>>(this, 0x678);
        if !le_or_nan(x(rd::<f32>(this, 0x84C)), x(f32::from_bits(0x3C75_C290))) && !frame.is_null() {
            let mut a = [0u32; 0x12];
            let am: GPtr<u8> = GPtr::from_raw(a.as_mut_ptr()).cast();
            matrix_attach(am, frame.byte_add(0x10), 0);
            let mut atomic = [0u32; 1];
            frame_for_all_objects(rd::<GPtr<u8>>(this, 0x678), 0x6A_0750, GPtr::from_raw(atomic.as_mut_ptr()).addr() as u32);
            if atomic[0] != 0 {
                let at = GPtr::<u8>::from_addr(atomic[0] as usize);
                if rd::<u8>(at, 0x4C) & 2 != 0 {
                    rp_atomic_resync_interpolated_sphere(at);
                }
                let radius = rd::<f32>(at, 0x28);
                if !le_or_nan(x(radius), x(f32::from_bits(0x3DCC_CCCD))) {
                    let scale = match model {
                        0x1D1 | 0x1F5 => f32::from_bits(0x3F66_6666),
                        0x1D5 | 0x1BF => f32::from_bits(0x3F4C_CCCD),
                        0x1A9 => 0.5,
                        _ => 1.0,
                    };
                    let st = rd::<u8>(this, 0x36) >> 3;
                    if st == 0 || st == 8 {
                        let pos = [a[0xC], a[0xD], a[0xE]].map(f32::from_bits);
                        do_blade_collision(this, pos[0], pos[1], pos[2], rd::<GPtr<u8>>(this, 0x14), 0xFFFF_FFFD, radius, scale.to_bits());
                    }
                }
            }
            let st = rd::<u8>(this, 0x36) >> 3;
            let pos = entity_pos(this);
            let p3 = [rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8)];
            if st == 0 || st == 3 {
                if !le_or_nan(x(rd::<f32>(this, 0x84C)), x(f32::from_bits(0x3BF5_C290))) {
                    let v = x(rd::<f32>(this, 0x84C)).fmul_m(f32::from_bits(0x40D5_5555));
                    let power = if ge_or_nan(x(1.0), v) { f32_of(v) } else { 1.0 };
                    wind_register_one(p3[0], p3[1], p3[2], 1, power);
                }
            } else if st == 2 {
                wind_register_one(p3[0], p3[1], p3[2], 1, 1.0);
            }
            sa_world::bindings::matrix_dtor(am);
        }
    }
    let model = rd::<u16>(this, 0x22);
    if model == 0x1D1 || model == 0x1F5 {
        return;
    }
    let rot = x(rd::<f32>(this, 0x84C));
    if ge_or_nan(rot, x(f32::from_bits(0x3E1D_B22D))) || le_or_nan(rot, x(f32::from_bits(0x3B90_2DE0))) {
        return;
    }
    let frame = rd::<GPtr<u8>>(this, 0x678);
    if frame.is_null() {
        return;
    }
    let hp = entity_pos(this);
    let cp = entity_pos(GPtr::from_addr(0xB6_F028));
    let dz = f32_of(x(rd::<f32>(cp, 8)).fsub_m(rd::<f32>(hp, 8)));
    let dy = f32_of(x(rd::<f32>(cp, 4)).fsub_m(rd::<f32>(hp, 4)));
    let dxe = x(rd::<f32>(cp, 0)).fsub_m(rd::<f32>(hp, 0));
    let dx = f32_of(dxe);
    let d2e = dxe.fmul_m(dx).fadd(x(dy).fmul_m(dy)).fadd(x(dz).fmul_m(dz));
    let d2 = f32_of(d2e);
    if ge_or_nan(d2e, x(400.0)) {
        return;
    }
    let diff = x(rd::<f32>(this, 0x958)).fsub_m(rd::<f32>(this, 0x82C)).fabs();
    if le_or_nan(diff, x(f32::from_bits(0x3F06_0A92))) {
        return;
    }
    let mut b = [0u32; 0x12];
    let bm: GPtr<u8> = GPtr::from_raw(b.as_mut_ptr()).cast();
    let mut c = [0u32; 0x12];
    let cm: GPtr<u8> = GPtr::from_raw(c.as_mut_ptr()).cast();
    matrix_attach(bm, frame.byte_add(0x10), 0);
    let mut v = [b[0], b[1], b[2]].map(f32::from_bits);
    let mut w = [0f32; 3];
    let r = multiply3x3(vp(&mut w), rd::<GPtr<u8>>(this, 0x14), vp(&mut v));
    let w = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
    let mut d = x(d2).fsqrt();
    let k = x(f32::from_bits(0x3C23_D70A));
    if !le_or_nan(k, d) {
        d = k;
    }
    let inv = d.fdivr_m(1.0);
    let nx = x(dx).fmul(inv);
    let ny = inv.fmul_m(dy);
    let nz = inv.fmul_m(dz);
    let dot = x(w[2]).fmul(nz).fadd(x(w[1]).fmul(ny)).fadd(x(w[0]).fmul(nx));
    if !le_or_nan(dot.fabs(), x(f32::from_bits(0x3F73_3333))) {
        audio_add_event(this.byte_add(0x138), 0x5E, 0.0);
        wr::<u32>(this, 0x958, rd::<u32>(this, 0x82C));
    }
    sa_world::bindings::matrix_dtor(cm);
    sa_world::bindings::matrix_dtor(bm);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_HELI_PROCESS_CONTROL, HOOK_HELI_SEND_DOWN_SWAT, HOOK_HELI_PROCESS_FLYING_CAR_STUFF];

/// This module's phase B natives.
pub const NATIVES: &[NativeReg] = &[NATIVE_HELI_FIND_SWAT_POSITION_NATIVE];
