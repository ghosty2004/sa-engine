//! MoveObject / StopObject motion: the rules of `worldents` written to the object.

use super::*;

/// MoveObject setup; a move in progress is finished first. Mirrors samp.dll 0x100A9C10.
pub(super) fn start_move(o: &mut Obj, target: [f32; 3], speed: f32, rot: [f32; 3], now_ms: u32) {
    let Some(current) = rows(o.ptr) else { return };
    let prev = o.motion.take();
    let (snap, motion) = world::begin_move(prev.as_ref(), current, target, speed, rot, now_ms);
    if let Some((r, pos)) = snap {
        stop(o);
        write_matrix(o.ptr, &r, pos);
        if let Some(r) = prev.and_then(|m| m.rotation) {
            o.rotation = r;
        }
    }
    o.motion = Some(motion);
}

/// StopObject: move and turn speed 0, moving flag off; the object stays where it is. Mirrors samp.dll 0x100A8070.
pub(super) fn stop(o: &mut Obj) {
    o.motion = None;
    set_speeds(o.ptr, [0.0; 3], Some([0.0; 3]));
}

fn set_speeds(ptr: u32, move_speed: [f32; 3], turn_speed: Option<[f32; 3]>) {
    for (i, v) in move_speed.iter().enumerate() {
        wr(ptr + g::MOVE_SPEED + 4 * i as u32, *v);
    }
    for (i, v) in turn_speed.iter().flatten().enumerate() {
        wr(ptr + g::TURN_SPEED + 4 * i as u32, *v);
    }
}

pub(super) fn move_step(o: &mut Obj, m: &Motion, dt: f32, now_ms: u32) {
    let Some(current) = rows(o.ptr) else { return };
    match world::motion_frame(m, current, dt, now_ms, rd(g::TIME_STEP)) {
        world::MotionFrame::Arrive { at: (r, pos), rotation } => {
            set_speeds(o.ptr, [0.0; 3], Some([0.0; 3]));
            write_matrix(o.ptr, &r, pos);
            if let Some(t) = rotation {
                o.rotation = t;
            }
            stop(o);
        }
        world::MotionFrame::Move { move_speed, turn_speed, at: (r, pos) } => {
            set_speeds(o.ptr, move_speed, turn_speed);
            write_matrix(o.ptr, &r, pos);
        }
        world::MotionFrame::Idle => {}
    }
}
