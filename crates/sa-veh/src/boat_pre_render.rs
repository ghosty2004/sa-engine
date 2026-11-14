//! PORT-VEH-4 (batch `veh4-2`): `CBoat::PreRender`. Ported from the 1.0 US bytes; gta-reversed as a
//! reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{f32_of, ftol32, g, jne_mask, jp5, rd, wr, x};

use crate::bindings::door_process;
use crate::bindings::vehicle_pre_render;
use sa_rwcore::bindings::frame_get_ltm;
use sa_core::bindings::fx_play;
use sa_core::bindings::fx_stop;
use sa_core::bindings::matrix_attach;
use sa_world::bindings::matrix_dtor;
use sa_core::bindings::matrix_set_rotate;
use sa_core::bindings::matrix_update_rw;
use sa_core::bindings::rw_matrix_update;
use sa_rwcore::bindings::frame_for_all_objects;
use sa_stream::bindings::get_first_object;
use crate::bindings::set_component_atomic_alpha;
use sa_world::bindings::fx_copy_parent_matrix;
use sa_world::bindings::fx_create_system as create_fx_system;
use sa_world::bindings::fx_get_play_status;
use sa_world::bindings::fx_set_const_time;
use sa_world::bindings::fx_set_local_particles;
use sa_world::bindings::fx_system_set_matrix as fx_set_matrix;
use sa_world::bindings::multiply3x3 as multiply_3x3;
use crate::bindings::set_component_rotation;
orig! {
    /// An atomic's render callback (`RpAtomic` +0x48), called with the atomic
    fn call_render_cb(f: u32, atomic: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
    /// `CVehicle::DoBoatSplashes(float)` (0x6DD130)
    fn do_boat_splashes(v: GPtr<u8>, speed: f32) = thiscall 0x6DD130 => unported;
}

use sa_core::field::ptr_of as buf;

/// `|v|` of the three floats at `p` as the original sums them (`(x*x + y*y) + z*z`, FSQRT).
fn length(p: GPtr<u8>) -> f32 {
    let (vx, vy, vz) = (x(rd::<f32>(p, 0)), x(rd::<f32>(p, 4)), x(rd::<f32>(p, 8)));
    f32_of(vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt())
}

/// The atomic of a frame's first atomic object, by `GetCurrentAtomicObjectCB` (0x6A0750).
fn frame_atomic(frame: GPtr<u8>) -> GPtr<u8> {
    let mut a = [0u32; 1];
    frame_for_all_objects(frame, 0x6A_0750, GPtr::<u8>::addr(buf(&mut a)) as u32);
    GPtr::from_addr(a[0] as usize)
}

/// `CBoat::PreRender()` (0x6F1180).
///
/// The vehicle pre-render; brightness 0.5; the three rudder frames (+0x5BC/+0x5C8/+0x5CC) turned about z
/// by `-steer`; the propeller angle (+0x5A8) advanced by `timeStep * propSpeed` (wrapped below 2pi); the
/// two propeller pairs (+0x5D0/+0x5D4, +0x5D8/+0x5DC) set about y to `+-2 * angle` / `-+angle` with the
/// fast/slow blur alphas `ftol((1 - min(speed * 5.09, 1)) * 255)` and `max(150 - that, 0)`; the
/// Marquis' (484) boom door processed and rotated on its axis; the radar dish of 430/453/454 (+0x5B4)
/// turned to +0x5A0 (rendered through its atomic's callback while 0x96914B is set) and advanced by
/// `timeStep * 0.02`; the frame speeds (+0x610 = move + friction move, +0x61C = turn + friction turn);
/// the two propeller splash fx (`boat_prop`, +0x628/+0x62C) created at the propeller frames, stopped
/// while the engine is off, else playing with a constant time `min(2 |moveSpeed|, 1)`; the boat
/// splashes by the length of +0x630.
#[hook(addr = 0x6F1180, conv = thiscall, name = "CBoat::PreRender", this = "CBoat",
       expect = "6A FF 68 18 89 84 00",
       unverified = "veh4-2", route = "L2", hazards = "heap-writes",
       note = "creates/plays the propeller FxSystems and writes RW frames; the CMatrix lives on the Rust stack; the SEH frame is not reproduced")]
pub fn boat_pre_render(this: GPtr<u8>) {
    vehicle_pre_render(this);
    wr::<u32>(this, 0x12C, 0x3F00_0000);
    let mut mat = [0u32; 0x12];
    let m = buf(&mut mat);
    for o in [0x5BCisize, 0x5C8, 0x5CC] {
        let a = f32_of(x(rd::<f32>(this, 0x494)).fchs());
        set_component_rotation(this, rd::<GPtr<u8>>(this, o), 2, a, 1);
    }
    let mut f = x(rd::<f32>(this, 0x5A4)).fmul_m(f32::from_bits(0x40A2_F983));
    if !jp5(x(1.0), f) {
        f = x(1.0);
    }
    let alpha = ftol32(x(1.0).fsub(f).fmul_m(255.0));
    let two_pi = f32::from_bits(0x40C9_0FDB);
    let ts = || g::<f32>(0xB7_CB5C).read();
    let a = x(ts()).fmul_m(rd::<f32>(this, 0x5A4)).fadd_m(rd::<f32>(this, 0x5A8));
    wr::<f32>(this, 0x5A8, f32_of(a));
    if !jne_mask(a, x(two_pi), 0x41) {
        let mut v = x(rd::<f32>(this, 0x5A8));
        loop {
            v = v.fsub_m(two_pi);
            if jne_mask(v, x(two_pi), 0x41) {
                break;
            }
        }
        wr::<f32>(this, 0x5A8, f32_of(v));
    }
    for (fast, slow, sign) in [(0x5D0isize, 0x5D4isize, 1.0f32), (0x5D8, 0x5DC, -1.0)] {
        let fr = rd::<GPtr<u8>>(this, fast);
        if !fr.is_null() {
            let ang = f32_of(x(sign).fmul_m(rd::<f32>(this, 0x5A8)).fdouble());
            set_component_rotation(this, fr, 1, ang, 1);
            let at = frame_atomic(rd::<GPtr<u8>>(this, fast));
            if !at.is_null() {
                set_component_atomic_alpha(at, alpha as u32);
            }
        }
        let fr = rd::<GPtr<u8>>(this, slow);
        if !fr.is_null() {
            let ang = f32_of(x(sign).fmul_m(rd::<f32>(this, 0x5A8)).fchs());
            set_component_rotation(this, fr, 1, ang, 1);
            let at = frame_atomic(rd::<GPtr<u8>>(this, slow));
            if !at.is_null() {
                let a2 = 0x96i32.wrapping_sub(alpha);
                set_component_atomic_alpha(at, (if a2 < 0 { 0 } else { a2 }) as u32);
            }
        }
    }
    if rd::<i16>(this, 0x22) == 0x1E4 {
        let fr = rd::<GPtr<u8>>(this, 0x5C0);
        if !fr.is_null() {
            matrix_attach(m, fr.byte_add(0x10), 0);
            let mut saved = [rd::<u32>(m, 0x30), rd::<u32>(m, 0x34), rd::<u32>(m, 0x38)];
            let mut rot = [0u32; 3];
            let mut off = [0u32; 3];
            multiply_3x3(buf(&mut off), rd::<GPtr<u8>>(this, 0x14), buf(&mut saved));
            door_process(this.byte_add(0x5E0), this, this.byte_add(0x610), this.byte_add(0x61C), buf(&mut off));
            let axis = rd::<u8>(this, 0x5EA) as usize;
            let rp = buf(&mut rot);
            wr::<u32>(rp, 4 * axis as isize, rd::<u32>(this, 0x5EC));
            matrix_set_rotate(m, f32::from_bits(rot[0]), f32::from_bits(rot[1]), f32::from_bits(rot[2]));
            for (k, o) in [(0usize, 0x30isize), (1, 0x34), (2, 0x38)] {
                wr::<f32>(m, o, f32_of(x(rd::<f32>(m, o)).fadd_m(f32::from_bits(saved[k]))));
            }
            matrix_update_rw(m);
        }
    }
    let model = rd::<i16>(this, 0x22);
    if model == 0x1AE || model == 0x1C5 || model == 0x1C6 {
        let fr = rd::<GPtr<u8>>(this, 0x5B4);
        if !fr.is_null() {
            set_component_rotation(this, fr, 2, rd::<f32>(this, 0x5A0), 1);
            if g::<u8>(0x96_914B).read() != 0 {
                let first = get_first_object(rd::<GPtr<u8>>(this, 0x5B4));
                let atomic = get_first_object(rd::<GPtr<u8>>(this, 0x5B4));
                call_render_cb(rd::<u32>(first, 0x48), atomic);
            }
        }
        wr::<f32>(this, 0x5A0, f32_of(x(ts()).fmul_m(f32::from_bits(0x3CA3_D70A)).fadd_m(rd::<f32>(this, 0x5A0))));
    }
    let sum = |a: isize, b: isize| f32_of(x(rd::<f32>(this, a)).fadd_m(rd::<f32>(this, b)));
    let (mz, my, mx) = (sum(0x4C, 0x64), sum(0x48, 0x60), sum(0x5C, 0x44));
    wr::<f32>(this, 0x610, mx);
    wr::<f32>(this, 0x614, my);
    wr::<f32>(this, 0x618, mz);
    let (tz, ty, tx) = (sum(0x58, 0x70), sum(0x54, 0x6C), sum(0x50, 0x68));
    wr::<f32>(this, 0x61C, tx);
    wr::<f32>(this, 0x620, ty);
    wr::<f32>(this, 0x624, tz);
    let speed = length(this.byte_add(0x44));
    for k in 0..2isize {
        let rw = rd::<GPtr<u8>>(this, 0x18);
        let rwm = if rw.is_null() { GPtr::<u8>::NULL } else { rd::<GPtr<u8>>(rw, 4).byte_add(0x10) };
        let slot = 0x628 + 4 * k;
        let frame_o = 0x5D0 + 8 * k;
        if rd::<u32>(this, slot) == 0 {
            if !rwm.is_null() && rd::<u32>(this, frame_o) != 0 {
                let mut off = [0u32; 3];
                let ltm = frame_get_ltm(rd::<GPtr<u8>>(this, frame_o));
                for o in [0x30isize, 0x34, 0x38] {
                    wr::<u32>(rwm, o, rd::<u32>(ltm, o));
                }
                rw_matrix_update(rwm);
                let fx = create_fx_system(GPtr::from_addr(0xA9_AE80), GPtr::from_addr(0x87_22CC), buf(&mut off), rwm, 0);
                wr::<GPtr<u8>>(this, slot, fx);
                if !fx.is_null() {
                    fx_play(fx);
                    fx_set_local_particles(rd::<GPtr<u8>>(this, slot), 1);
                    fx_copy_parent_matrix(rd::<GPtr<u8>>(this, slot));
                }
            }
            if rd::<u32>(this, slot) == 0 {
                continue;
            }
        }
        let fx = rd::<GPtr<u8>>(this, slot);
        if rd::<u32>(this, 0x40) & 0x100 == 0 {
            if fx_get_play_status(fx) == 0 {
                fx_stop(rd::<GPtr<u8>>(this, slot));
            }
            continue;
        }
        if fx_get_play_status(fx) == 1 {
            fx_play(rd::<GPtr<u8>>(this, slot));
        }
        let ltm = frame_get_ltm(rd::<GPtr<u8>>(this, frame_o));
        for o in [0x30isize, 0x34, 0x38] {
            wr::<u32>(rwm, o, rd::<u32>(ltm, o));
        }
        rw_matrix_update(rwm);
        fx_set_matrix(rd::<GPtr<u8>>(this, slot), rwm);
        let mut t = x(speed).fdouble();
        if !jp5(x(1.0), t) {
            t = x(1.0);
        }
        fx_set_const_time(rd::<GPtr<u8>>(this, slot), 1, f32_of(t));
    }
    do_boat_splashes(this, length(this.byte_add(0x630)));
    matrix_dtor(m);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_BOAT_PRE_RENDER];
