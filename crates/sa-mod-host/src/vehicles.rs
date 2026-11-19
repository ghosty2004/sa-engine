//! The vehicle damage and class, speed, gravity, statistics and entry-exit host functions (API 1.11), vehicle
//! matrices and motion, and a ped's shot (1.15), and the frame limit (1.16).

use sa_mod_api::state::VehicleMotion;
use sa_mod_api::{Call, status, vehicle};
use sa_types::{EntityRef, Matrix, PedRef, Vec3, VehicleRef};

use crate::api::{CallCtx, Mem, guarded, write_record};
use crate::objects::floats;

/// The calls of this module: `None` for any other call.
pub fn calls(ctx: &mut CallCtx, mem: &mut dyn Mem, call: Call) -> Option<i32> {
    let reads =
        matches!(call, Call::vehicle_class { .. } | Call::vehicle_damage { .. } | Call::vehicle_motion { .. });
    let writes = matches!(
        call,
        Call::set_gravity { .. }
            | Call::set_vehicle_damage { .. }
            | Call::set_vehicle_speed { .. }
            | Call::set_ped_speed { .. }
            | Call::set_float_stat { .. }
            | Call::set_entry_exits_enabled { .. }
            | Call::set_vehicle_matrix { .. }
            | Call::fire_ped_weapon { .. }
            | Call::set_frame_limit { .. }
    );
    if !reads && !writes {
        return None;
    }
    if writes && ctx.in_hook() {
        return Some(status::WRONG_PHASE);
    }
    let Some(game) = ctx.game.clone() else { return Some(status::NOT_FOUND) };
    let mut g = game.0.borrow_mut();
    let veh = |h: i32| VehicleRef(h as u32);
    let done = |r: Result<(), i32>| r.map_or_else(|s| s, |()| status::OK);
    Some(match call {
        Call::vehicle_class { handle } => match g.vehicle_class(veh(handle)) {
            Ok(c) => vehicle::class_number(c),
            Err(_) => status::NOT_FOUND,
        },
        Call::vehicle_damage { handle, out_ptr } => match g.vehicle_damage(veh(handle)) {
            Ok(words) => {
                let out: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
                if out_ptr < 0 || !mem.write(out_ptr as u32, &out) {
                    return Some(status::BAD_ARG);
                }
                status::OK
            }
            Err(_) => status::NOT_FOUND,
        },
        Call::set_vehicle_damage { handle, panels, doors, lights, tyres } => {
            let (Ok(lights), Ok(tyres)) = (u8::try_from(lights), u8::try_from(tyres)) else {
                return Some(status::BAD_ARG);
            };
            done(guarded(&mut ctx.logs, "set_vehicle_damage", || {
                g.set_vehicle_parts_damage(veh(handle), panels as u32, doors as u32, lights)?;
                g.set_vehicle_tyres(veh(handle), tyres)
            }))
        }
        Call::set_vehicle_speed { handle, speeds_ptr } => {
            let Some(w) = floats::<6>(mem, speeds_ptr) else { return Some(status::BAD_ARG) };
            let (mv, turn) = (Vec3::new(w[0], w[1], w[2]), Vec3::new(w[3], w[4], w[5]));
            done(guarded(&mut ctx.logs, "set_vehicle_speed", || {
                g.set_physical_speed(EntityRef::Vehicle(veh(handle)), mv, Some(turn))
            }))
        }
        Call::vehicle_motion { handle, buf_ptr, buf_len } => match g.vehicle_snapshot(veh(handle)) {
            Ok(s) => write_record(
                mem,
                buf_ptr,
                buf_len,
                &VehicleMotion {
                    turn_speed: s.turn_speed,
                    moving: s.moving,
                    tractor: s.tractor,
                    trailer: s.trailer,
                    max_passengers: s.max_passengers,
                },
            ),
            Err(_) => status::NOT_FOUND,
        },
        Call::set_vehicle_matrix { handle, matrix_ptr } => {
            let Some(w) = floats::<12>(mem, matrix_ptr) else { return Some(status::BAD_ARG) };
            let v = |i: usize| Vec3::new(w[i], w[i + 1], w[i + 2]);
            let m = Matrix { right: v(0), forward: v(3), up: v(6), pos: v(9) };
            done(guarded(&mut ctx.logs, "set_vehicle_matrix", || g.set_vehicle_matrix(veh(handle), &m)))
        }
        Call::fire_ped_weapon { handle, x, y, z } => {
            if ![x, y, z].iter().all(|v| v.is_finite()) {
                return Some(status::BAD_ARG);
            }
            done(guarded(&mut ctx.logs, "fire_ped_weapon", || g.fire_ped_weapon(PedRef(handle as u32), Vec3::new(x, y, z))))
        }
        Call::set_ped_speed { handle, x, y, z } => {
            if ![x, y, z].iter().all(|v| v.is_finite()) {
                return Some(status::BAD_ARG);
            }
            done(guarded(&mut ctx.logs, "set_ped_speed", || g.set_ped_speed(PedRef(handle as u32), Vec3::new(x, y, z))))
        }
        Call::set_gravity { gravity } => {
            if !gravity.is_finite() {
                return Some(status::BAD_ARG);
            }
            if ctx.changes.gravity.is_none() {
                ctx.changes.gravity = Some(g.gravity());
            }
            done(guarded(&mut ctx.logs, "set_gravity", || g.set_gravity(gravity)))
        }
        Call::set_float_stat { stat, value } => {
            let Some(stat) = u16::try_from(stat).ok().filter(|s| i32::from(*s) < sa_access::FLOAT_STAT_COUNT) else {
                return Some(status::BAD_ARG);
            };
            if !value.is_finite() {
                return Some(status::BAD_ARG);
            }
            done(guarded(&mut ctx.logs, "set_float_stat", || g.set_float_stat(stat, value)))
        }
        Call::set_entry_exits_enabled { on } => {
            match (on != 0, ctx.changes.entry_exit_flags.take()) {
                (true, Some(saved)) => g.set_entry_exit_flags(&saved),
                (true, None) => {}
                (false, saved) => {
                    let now = g.entry_exit_flags();
                    g.set_entry_exit_flags(&now.iter().map(|&(r, _)| (r, 0)).collect::<Vec<_>>());
                    ctx.changes.entry_exit_flags = Some(saved.unwrap_or(now));
                }
            }
            status::OK
        }
        Call::set_frame_limit { fps } => {
            let Some(fps) = u32::try_from(fps).ok().filter(|f| (1..=sa_access::FRAME_LIMIT_MAX).contains(f)) else {
                return Some(status::BAD_ARG);
            };
            if ctx.changes.frame_limit.is_none() {
                ctx.changes.frame_limit = Some(g.frame_limit());
            }
            done(guarded(&mut ctx.logs, "set_frame_limit", || g.set_frame_limit(fps)))
        }
        _ => return None,
    })
}
