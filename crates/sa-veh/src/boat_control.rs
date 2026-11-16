//! FRONTIER-2 turn 6, lane B (batch `frontier2-b15`): `CBoat::ProcessControl` (0x6F1770), the `boat` scenario's
//! blocker. Ported from the 1.0 US bytes; gta-reversed's reading was an aid only (where it differs the bytes win: the
//! anchor heading marker is -9999.99, the burning timer adds the frame's whole milliseconds, the fire offset's z is
//! raised by 1.0, statuses 2 / 3 run the AI). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::field::{ptr_of as buf, v3_ptr as vp};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_core::crt::rand_at;
use sa_math::fpu::{ge_or_nan, gt, le_or_nan, lt};
use sa_math::x87::X87;

use crate::automobile_control::process_boat_control;
use crate::bindings::{
    audio_service, do_drive_by_shootings, event_vehicle_on_fire_ctor, event_vehicle_on_fire_dtor, process_car_alarm,
    process_delayed_explosion, update_clump_alpha,
};
use crate::common::{f32_of, ftol32, g, rd, wr, x};
use crate::mvpb_a1::flying_control;
use sa_core::bindings::{find_player_ped, find_player_vehicle, fx_play, matrix_rotate_z};
use sa_core::field::gf;
use sa_pop::bindings::{steer_ai_car_with_physics, update_car_ai};
use sa_pop::carctrl3::{time_ms, time_step as ts};
use sa_world::bindings::{
    event_group_add, find_player_centre_of_world_no_sniper_shift, fx_create_system, fx_kill, get_event_global_group,
    get_modelling_matrix, get_percentage_progress, get_vehicle_appearance, physical_process_control, ped_say,
};
use sa_world::world::common::entity_pos;
/// `GetHeading()`, inlined in the original.
use sa_world::world::entity::get_heading as heading;
use sa_world::world::stats_travel::speed_sq;

orig! {
    /// `CBoat::PruneWakeTrail()` (0x6F0E20, a phase B native)
    fn prune_wake_trail(b: GPtr<u8>) = thiscall 0x6F0E20 => crate::wake_locks_component_rotation::boat_prune_wake_trail;
    /// `CVehicle::DoFixedMachineGuns()` (0x73F400)
    fn do_fixed_machine_guns(v: GPtr<u8>) = thiscall 0x73F400 => crate::vehicle_flight_targets::do_fixed_machine_guns;
    /// virtual (vtable +0x60) `ProcessControlInputs(uint8 padNum)`
    fn v_process_control_inputs(v: GPtr<u8>, pad: u32) = thiscall vslot 0x60 => unported;
    /// virtual (vtable +0xA4) `BlowUpCar(CEntity* damager, bool hideExplosion)`
    fn v_blow_up_car(v: GPtr<u8>, damager: GPtr<u8>, hide: u32) = thiscall vslot 0xA4 => unported;
}

/// The anchor heading's "none" value (-9999.99f).
const NO_HEADING: u32 = 0xC61C_3FF6;

/// `CBoat::ProcessControl()` (0x6F1770, vtable slot). The audio service, the wake trail, the delayed explosion; a
/// scorched hull sinks (buoyancy -= mass * 8e-6 while above mass * 0.0064); a Predator chasing a wanted player's boat
/// picks its next talk time. By status: 0 the driver's inputs, the Predator's guns and the drive-by; 2 the AI and the
/// plain physics, then (in water) done; 3 the AI and its steering; 4 / 5 parked (in water, no steering, brake 0.5)
/// and frozen beyond 150 m of the player. The engine speed follows the throttle (statuses 0 / 3 / 8; the plane's
/// propeller constants, x5 rate with throttle; the Skimmer idles at the standard speed), else decays by 0.95. A
/// collision above 25 damage costs health (the player's boat less; a driver shouts above 60); below 460 health a fire
/// burns within 200 m of the camera (below 250: the `fire_car` system, the on-fire event, and the blow-up after five
/// seconds). Then the physics, `ProcessBoatControl`, the Skimmer's (or the flying-boats cheat's) flight and the anchor.
#[hook(addr = 0x6F1770, conv = thiscall, name = "CBoat::ProcessControl", this = "CBoat", expect = "64 A1 00 00 00 00 6A FF 68 38 89 84 00",
       unverified = "frontier2-b15", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "FSQRT of the player distance, FPATAN of the anchor heading; rand at_site 0x6F1849; the on-fire event lives on the Rust stack; the SEH frame is not reproduced; ProcessBoatControl's bool dwords carry stack garbage above their low byte in the original")]
pub fn boat_process_control(this: GPtr<u8>) {
    audio_service(this.byte_add(0x138));
    prune_wake_trail(this);
    process_delayed_explosion(this);
    if rd::<u32>(this, 0x40) & 0x2000_0000 != 0
        && lt(x(rd::<f32>(this, 0x8C)).fmul_m(f32::from_bits(0x3BD1_B718)), x(rd::<f32>(this, 0xA0)))
    {
        let b = x(rd::<f32>(this, 0x8C)).fmul_m(f32::from_bits(0x3706_37BE)).fsubr_m(rd::<f32>(this, 0xA0));
        wr::<f32>(this, 0xA0, f32_of(b));
    }
    let pd = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
    let wanted = if pd.is_null() { GPtr::<u8>::from_addr(0) } else { rd::<GPtr<u8>>(pd, 0) };
    if rd::<i32>(wanted, 0x2C) > 0 && rd::<u16>(this, 0x22) == 0x1AE {
        let pv = find_player_vehicle(-1, 0);
        if !pv.is_null() && get_vehicle_appearance(pv) == 4 {
            let mission = rd::<i8>(this, 0x3BA);
            if mission >= 2 && (mission <= 5 || mission == 0x15) {
                let now = time_ms();
                if now > rd::<u32>(this, 0x600) {
                    let r = (rand_at(0x6F_1849) as u32) & 0xFFF;
                    wr::<u32>(this, 0x600, r.wrapping_add(now).wrapping_add(0x1194));
                }
            }
        }
    }
    update_clump_alpha(this);
    process_car_alarm(this);
    let unlock = || {
        wr::<u8>(this, 0x5AC, rd::<u8>(this, 0x5AC) & 0xFB);
        wr::<u32>(this, 0x5FC, NO_HEADING);
    };
    let in_water = || {
        wr::<u8>(this, 0x5AC, rd::<u8>(this, 0x5AC) | 3);
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x100);
    };
    match rd::<u8>(this, 0x36) >> 3 {
        0 => {
            unlock();
            let driver = rd::<GPtr<u8>>(this, 0x460);
            if !driver.is_null() {
                v_process_control_inputs(this, rd::<u32>(driver, 0x598));
            }
            if rd::<u16>(this, 0x22) == 0x1AE {
                do_fixed_machine_guns(this);
            }
            do_drive_by_shootings(this);
        }
        2 => {
            unlock();
            update_car_ai(this);
            physical_process_control(this);
            in_water();
            return;
        }
        3 => {
            unlock();
            update_car_ai(this);
            steer_ai_car_with_physics(this);
        }
        4 | 5 => {
            in_water();
            wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xDF);
            wr::<u32>(this, 0x494, 0);
            wr::<f32>(this, 0x4A0, 0.5);
            wr::<u32>(this, 0x49C, 0);
            let p = entity_pos(this);
            let c = find_player_centre_of_world_no_sniper_shift(0);
            let dz = x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(c, 8));
            let dy = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(c, 4));
            let dx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(c, 0));
            let d = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt();
            if !le_or_nan(d, x(150.0)) {
                for o in [0x44isize, 0x48, 0x4C, 0x50, 0x54, 0x58] {
                    wr::<u32>(this, o, 0);
                }
                return;
            }
        }
        _ => {}
    }
    let status = rd::<u8>(this, 0x36) >> 3;
    // the engine (propeller) speed
    if status == 0 || status == 8 || status == 3 {
        let mut roc = gf(0x8D_33D8);
        let mut target = x(0.0);
        let gas = || x(rd::<f32>(this, 0x49C));
        if rd::<u16>(this, 0x22) == 0x1CC {
            target = x(gf(0x8D_33D0));
        } else if gas().fcom(x(0.0)).test_parity(0x44) {
            roc = f32_of(x(gf(0x8D_33D8)).fmul_m(5.0));
        }
        if gt(gas(), x(0.0)) {
            target = x(gf(0x8D_33D4)).fsub_m(gf(0x8D_33D0)).fmul_m(rd::<f32>(this, 0x49C)).fadd_m(gf(0x8D_33D0));
        } else if !ge_or_nan(gas(), x(0.0)) {
            target = x(gf(0x8D_33D0)).fsub_m(gf(0x8D_33CC)).fmul_m(rd::<f32>(this, 0x49C)).fadd_m(gf(0x8D_33D0));
        }
        let e = target.fsub_m(rd::<f32>(this, 0x5A4)).fmul_m(ts()).fmul_m(roc).fadd_m(rd::<f32>(this, 0x5A4));
        wr::<f32>(this, 0x5A4, f32_of(e));
    } else if gt(x(rd::<f32>(this, 0x5A4)), x(0.0)) {
        wr::<f32>(this, 0x5A4, f32_of(x(rd::<f32>(this, 0x5A4)).fmul_m(f32::from_bits(0x3F73_3333))));
    }
    // collision damage
    let mut dmg = f32_of(x(rd::<f32>(this, 0xD8)).fmul_m(rd::<f32>(rd::<GPtr<u8>>(this, 0x384), 0xC8)));
    if gt(x(dmg), x(25.0)) && status != 5 && !x(rd::<f32>(this, 0x4C0)).fcom(x(250.0)).c0() {
        let saved = rd::<f32>(this, 0x4C0);
        if status == 0 && !get_percentage_progress().fcom(x(100.0)).c0() {
            dmg = f32_of(x(dmg).fmul_m(0.5));
        }
        let less = rd::<i8>(this, 0x429) < 0;
        let loss = if this == find_player_vehicle(-1, 0) {
            let d = x(dmg).fsub_m(25.0);
            if less { d.fmul_m(f32::from_bits(0x3E2A_AAAB)) } else { d.fmul_m(0.5) }
        } else {
            let driver = rd::<GPtr<u8>>(this, 0x460);
            if gt(x(dmg), x(60.0)) && !driver.is_null() {
                ped_say(driver, 0x44, 0, 1.0, 0, 0, 0);
            }
            let d = x(dmg).fsub_m(25.0);
            if less { d.fmul_m(f32::from_bits(0x3DAA_AAAB)) } else { d.fmul_m(0.25) }
        };
        wr::<f32>(this, 0x4C0, f32_of(loss.fsubr_m(rd::<f32>(this, 0x4C0))));
        let h = x(rd::<f32>(this, 0x4C0)).fcom(x(0.0));
        if !h.test_parity(0x41) && gt(x(saved), x(0.0)) {
            wr::<f32>(this, 0x4C0, 1.0);
            wr::<u32>(this, 0x60C, rd::<u32>(this, 0xDC));
        }
    }
    // fire
    let health = x(rd::<f32>(this, 0x4C0));
    if health.fcom(x(460.0)).test_parity(0x41) || rd::<u8>(this, 0x36) & 0xF8 == 0x28 {
        wr::<u32>(this, 0x608, 0);
        let f = rd::<GPtr<u8>>(this, 0x57C);
        if !f.is_null() {
            fx_kill(f);
            wr::<u32>(this, 0x57C, 0);
        }
    } else {
        let p = entity_pos(this);
        let cam = entity_pos(GPtr::from_addr(0xB6_F028));
        let near = |i: isize| lt(x(rd::<f32>(p, i)).fsub_m(rd::<f32>(cam, i)).fabs(), x(200.0));
        if near(0) && near(4) {
            let mut off = match rd::<i16>(this, 0x22) {
                0x1C4 => [f32::from_bits(0x3ECC_CCCD), f32::from_bits(0xC019_999A), f32::from_bits(0x3F4C_CCCD)],
                0x1C5 => [2.0, -1.0, 0.5],
                _ => [-1.5, -0.5, f32::from_bits(0x3F99_999A)],
            };
            off[2] = f32_of(x(off[2]).fadd_m(1.0));
            if lt(x(rd::<f32>(this, 0x4C0)), x(250.0)) {
                if rd::<u32>(this, 0x57C) == 0 {
                    let mm = get_modelling_matrix(this);
                    if !mm.is_null() {
                        let fx = fx_create_system(GPtr::from_addr(0xA9_AE80), GPtr::from_addr(0x85_AA68), vp(&mut off), mm, 0);
                        wr::<GPtr<u8>>(this, 0x57C, fx);
                        if !fx.is_null() {
                            fx_play(fx);
                            let mut ev_b = [0u32; 16];
                            let ev = buf(&mut ev_b);
                            event_vehicle_on_fire_ctor(ev, this);
                            event_group_add(get_event_global_group(), ev, 0);
                            event_vehicle_on_fire_dtor(ev);
                        }
                    }
                }
                let n = ftol32(x(ts()).fmul_m(f32::from_bits(0x3CA3_D70A)).fmul_m(1000.0));
                let t = X87::from_f64_exact(f64::from(n as u32)).fadd_m(rd::<f32>(this, 0x608));
                wr::<f32>(this, 0x608, f32_of(t));
                if gt(t, x(5000.0)) {
                    v_blow_up_car(this, rd::<GPtr<u8>>(this, 0x60C), 0);
                }
            }
        }
    }
    let hit_wall = (rd::<u32>(this, 0x1C) >> 12) & 1;
    let post = u32::from(gt(x(rd::<f32>(this, 0xD8)), x(0.0)) && gt(x(rd::<f32>(this, 0xE8)), x(f32::from_bits(0x3DCC_CCCD))));
    physical_process_control(this);
    process_boat_control(this, rd::<GPtr<u8>>(this, 0x5F8), this.byte_add(0x640), hit_wall, post);
    let marker = f32::from_bits(NO_HEADING);
    if rd::<u16>(this, 0x22) == 0x1CC {
        if gt(x(rd::<f32>(this, 0x5A4)), x(gf(0x8D_33CC))) || gt(speed_sq(this), X87::from_f64_exact(0.05)) {
            flying_control(this, 3, marker, marker, marker, marker);
        }
    } else if g::<u8>(0x96_9153).read() != 0 {
        flying_control(this, 5, marker, marker, marker, marker);
    }
    if rd::<u8>(this, 0x5AC) & 4 == 0 {
        return;
    }
    wr::<u32>(this, 0x44, 0);
    wr::<u32>(this, 0x48, 0);
    if rd::<u32>(this, 0x5FC) == NO_HEADING {
        wr::<f32>(this, 0x5FC, f32_of(heading(this)));
        return;
    }
    let angle = f32_of(x(rd::<f32>(this, 0x5FC)).fsub(heading(this)));
    let m = rd::<GPtr<u8>>(this, 0x14);
    let p = entity_pos(this);
    let pos = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
    matrix_rotate_z(m, angle);
    for (i, v) in pos.into_iter().enumerate() {
        wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x30 + i as isize * 4, v);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_BOAT_PROCESS_CONTROL];
