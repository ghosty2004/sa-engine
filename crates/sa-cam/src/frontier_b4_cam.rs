//! FRONTIER-2 turn 4, lane B (batch `frontier2-b12`): `CCam::ProcessArrestCamOne` (0x518500; gta-reversed has only
//! an unreachable stub). Ported from the 1.0 US bytes with `docs/re/specs/CAM/CCam.md` as reading aid; unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::field::v3_ptr as vp;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::{ge_or_nan, le_or_nan};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::cammodes::avoid_the_geometry;
use crate::common::{f32_of, g, rd, wr, x, xi};
use sa_core::bindings::{add_big_message, cross_product, ped_get_bone_position, text_get, vector_normalise};
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::{
    entity_register_reference, find_ground_z_for_3d_coord, find_player_ped, tm_find_active_task_by_type,
    world_test_sphere_against_world,
};
use sa_world::world::common::entity_pos;

orig! {
    /// `CCam` helper 0x512EF0 (`CPed* cop, bool check`, AL): the cop's head view
    fn head_view(c: GPtr<u8>, cop: GPtr<u8>, check: u32) -> u8 = thiscall 0x512EF0 => crate::arrestcam::arrest_cam_head_view;
    /// `CCam` helper 0x515D80 (`CEntity* target, CPed* cop, const CVector& P, CVector& out`, AL)
    fn source_from(c: GPtr<u8>, t: GPtr<u8>, cop: GPtr<u8>, p: GPtr<u8>, out: GPtr<u8>) -> u8 = thiscall 0x515D80 => crate::arrestcam::arrest_cam_source_from;
    /// `CCam` helper 0x516010 (same arguments, AL)
    fn side_pos(c: GPtr<u8>, t: GPtr<u8>, cop: GPtr<u8>, p: GPtr<u8>, out: GPtr<u8>) -> u8 = thiscall 0x516010 => crate::arrestcam::arrest_cam_side_pos;
    /// `CCam::GetLookFromLampPostPos(CEntity*, CPed*, const CVector&, CVector&)` (0x5161A0, AL)
    fn lamp_post(c: GPtr<u8>, t: GPtr<u8>, cop: GPtr<u8>, p: GPtr<u8>, out: GPtr<u8>) -> u8 = thiscall 0x5161A0 => crate::dwcams::get_look_from_lamp_post_pos;
}

/// `TheCamera` (0xB6F028) and its `m_pTargetEntity` (0xB6F980).
const THE_CAMERA: usize = 0xB6_F028;
const TARGET: usize = 0xB6_F980;
/// `sArrestMode` (i32 0xB6EC58), `s_ExitCam` (CEntity* 0xB6EC5C), the start time (f32 0xB6EC60).
const ARREST_MODE: usize = 0xB6_EC58;
const EXIT_CAM: usize = 0xB6_EC5C;
const START_TIME: usize = 0xB6_EC60;
/// `CTimer::ms_fTimeStep` (0xB7CB5C).
const TIME_STEP: usize = 0xB7_CB5C;

fn normalise(p: GPtr<u8>) {
    vector_normalise(p.cast());
}

/// `GETCOP`: the player's arresting officer (`FindPlayerPed(-1)->+0x480->+8`) when there is one, else `cop`.
fn get_cop(cop: GPtr<u8>) -> GPtr<u8> {
    if find_player_ped(-1).is_null() || rd::<GPtr<u8>>(rd::<GPtr<u8>>(find_player_ped(-1), 0x480), 8).is_null() {
        return cop;
    }
    rd::<GPtr<u8>>(rd::<GPtr<u8>>(find_player_ped(-1), 0x480), 8)
}

/// The look-at point P: a ped's bone 3; a vehicle's player driver's bone 3, else its position. `None` for other
/// entity types. Returns P and the ped (the target ped, the player driver or null).
fn look_at(t: GPtr<u8>, p: &mut [f32; 3]) -> Option<GPtr<u8>> {
    match rd::<u8>(t, 0x36) & 7 {
        3 => {
            ped_get_bone_position(t, vp(p), 3, 1);
            Some(t)
        }
        2 => {
            let d = rd::<GPtr<u8>>(t, 0x460);
            if !d.is_null() && ped_is_player(d) != 0 {
                let d = rd::<GPtr<u8>>(t, 0x460);
                ped_get_bone_position(d, vp(p), 3, 1);
                Some(d)
            } else {
                let q = entity_pos(t);
                *p = [rd::<f32>(q, 0), rd::<f32>(q, 4), rd::<f32>(q, 8)];
                Some(GPtr::NULL)
            }
        }
        _ => None,
    }
}

/// The common tail (0x518737 / 0x51911D): `source = out`, `L = out`, `TheCamera.AvoidTheGeometry(&L, &P, &source,
/// FOV)`, `front = P - source` normalised, `up = (0, 0, 1)`, `R = front x up` normalised, `up = R x front`.
fn aim(this: GPtr<u8>, out: [f32; 3], p: &mut [f32; 3]) {
    let src = this.byte_add(0x19C);
    for (k, v) in out.iter().enumerate() {
        wr::<f32>(src, 4 * k as isize, *v);
    }
    let mut l = out;
    avoid_the_geometry(GPtr::from_addr(THE_CAMERA), vp(&mut l), vp(p), src, rd::<u32>(this, 0xB4));
    let front = this.byte_add(0x190);
    for k in 0..3isize {
        wr::<f32>(front, 4 * k, f32_of(x(p[k as usize]).fsub_m(rd::<f32>(src, 4 * k))));
    }
    normalise(front);
    let up = this.byte_add(0x1B4);
    wr::<u32>(up, 0, 0);
    wr::<u32>(up, 4, 0);
    wr::<f32>(up, 8, 1.0);
    let mut r = [0f32; 3];
    cross_product(vp(&mut r), front, up);
    normalise(vp(&mut r));
    let mut u = [0f32; 3];
    let res = cross_product(vp(&mut u), vp(&mut r), front);
    for k in 0..3isize {
        wr::<u32>(up, 4 * k, rd::<u32>(res, 4 * k));
    }
}

/// KEEP (0x5190D9): `L = source; TheCamera.AvoidTheGeometry(&L, &P, &source, FOV)`.
fn keep(this: GPtr<u8>, p: &mut [f32; 3]) -> u8 {
    let src = this.byte_add(0x19C);
    let mut l = [rd::<f32>(src, 0), rd::<f32>(src, 4), rd::<f32>(src, 8)];
    avoid_the_geometry(GPtr::from_addr(THE_CAMERA), vp(&mut l), vp(p), src, rd::<u32>(this, 0xB4));
    1
}

/// `TestSphereAgainstWorld(v * 0.5 + base, 0.4, target, buildings, vehicles, objects, camera-ignore)` with the
/// bytes' operand order: x and y from the unrounded products, z from the stored f32 product.
fn sphere_blocked(v: [f32; 3], base: [f32; 3], t: GPtr<u8>) -> bool {
    let a = x(v[0]).fmul_m(0.5);
    let b = x(v[1]).fmul_m(0.5);
    let c = f32_of(x(v[2]).fmul_m(0.5));
    let centre = CVector { x: f32_of(a.fadd_m(base[0])), y: f32_of(b.fadd_m(base[1])), z: f32_of(x(c).fadd_m(base[2])) };
    !world_test_sphere_against_world(centre, f32::from_bits(0x3ECC_CCCD), t.addr() as u32, 1, 1, 0, 1, 0, 1).is_null()
}

/// `out += (v * k) * timeStep` with the bytes' rounding: x through a stored f32 product, y unrounded, z from the
/// stored `v.z * k`.
fn step(out: &mut [f32; 3], v: [f32; 3], kk: f32) {
    let ts = g::<f32>(TIME_STEP).read();
    let a = f32_of(x(v[0]).fmul_m(kk).fmul_m(ts));
    let b = x(v[1]).fmul_m(kk).fmul_m(ts);
    let c = x(f32_of(x(v[2]).fmul_m(kk))).fmul_m(ts);
    out[0] = f32_of(x(a).fadd_m(out[0]));
    out[1] = f32_of(b.fadd_m(out[1]));
    out[2] = f32_of(c.fadd_m(out[2]));
}

/// `CCam::ProcessArrestCamOne()` (0x518500, thiscall, AL; the caller ignores the result). FOV 45. Reset
/// (`m_bResetStatics` +0xA): the look-at point and the mode list (ped target: {1, 2, 3, 2, 8} with an arresting cop and
/// `rand / 32767 > 0.5`, else {1, 3, 2, 8}; vehicle: {2, 8, 3, 2} with a cop and `> 0.65`, else {8, 3, 2}; other
/// types return 0), the BUSTED big message unless 0xBAADC0, then the first mode whose helper succeeds (1: the head
/// view, which returns at once; 2 / 3: from the cop, or the ped's group members arresting the player (task 1100);
/// 8: a lamp post) becomes `sArrestMode` (3 is re-drawn among 3..7), `s_ExitCam` gets a reference, and the camera
/// aims; the reset flag stays set while no mode was found. Running: mode 1 the head view; 2 the source from
/// `s_ExitCam` with its z rise limited to `0.1 * timeStep`; 4..7 a side step along `front x up` (6 / 7 mirrored, 5 / 7
/// rising, 4 / 6 held 0.7 above the ground) unless a 0.4 sphere half a unit ahead hits; 8 a circling step; otherwise
/// the source is kept (`AvoidTheGeometry` only).
#[hook(addr = 0x518500, conv = thiscall, name = "CCam::ProcessArrestCamOne", this = "CCam", expect = "83 EC 68 53 55 8B E9 56 57",
       unverified = "frontier2-b12", route = "L2", hazards = "uninit-locals,heap-writes",
       note = "rand at_site 0x5185AB / 0x5186AA / 0x518A96; the reset tail reads `out` even when no mode was found (uninitialised in the original, zeros here); FindGroundZFor3DCoord's ST0 read as f64")]
pub fn process_arrest_cam_one(this: GPtr<u8>) -> u8 {
    let mut modes = [-1i32; 6];
    let mut found = false;
    let mut cop = GPtr::<u8>::NULL;
    wr::<f32>(this, 0xB4, 45.0);
    let mut p = [0f32; 3];
    let mut out = [0f32; 3];
    let mode = g::<i32>(ARREST_MODE);
    let exit_cam = g::<GPtr<u8>>(EXIT_CAM);
    let rnd = |site: u32| xi(sa_core::crt::rand_at(site)).fmul_m(f32::from_bits(0x3800_0100));
    if rd::<u8>(this, 0xA) != 0 {
        let t = g::<GPtr<u8>>(TARGET).read();
        mode.write(0);
        let ped;
        match rd::<u8>(t, 0x36) & 7 {
            3 => {
                ped = t;
                ped_get_bone_position(t, vp(&mut p), 3, 1);
                cop = get_cop(cop);
                if !cop.is_null() && !le_or_nan(rnd(0x51_85AB), x(0.5)) {
                    modes[..5].copy_from_slice(&[1, 2, 3, 2, 8]);
                } else {
                    modes[..4].copy_from_slice(&[1, 3, 2, 8]);
                }
            }
            2 => {
                let d = rd::<GPtr<u8>>(t, 0x460);
                if !d.is_null() && ped_is_player(d) != 0 {
                    ped = rd::<GPtr<u8>>(t, 0x460);
                    ped_get_bone_position(ped, vp(&mut p), 3, 1);
                } else {
                    ped = GPtr::NULL;
                    let q = entity_pos(t);
                    p = [rd::<f32>(q, 0), rd::<f32>(q, 4), rd::<f32>(q, 8)];
                }
                cop = get_cop(cop);
                if !cop.is_null() && !le_or_nan(rnd(0x51_86AA), x(f32::from_bits(0x3F26_6666))) {
                    modes[..4].copy_from_slice(&[2, 8, 3, 2]);
                } else {
                    modes[..3].copy_from_slice(&[8, 3, 2]);
                }
            }
            _ => return 0,
        }
        if g::<u8>(0xBA_ADC0).read() == 0 {
            let s = text_get(GPtr::from_addr(0xC1_B340), GPtr::from_addr(0x85_987C));
            add_big_message(s, 5000, 2);
        }
        let pp = vp(&mut p);
        let op = vp(&mut out);
        let mut i = 0usize;
        while mode.read() == 0 && i < 6 && modes[i] > 0 {
            let m = modes[i];
            exit_cam.write(GPtr::NULL);
            let tt = || g::<GPtr<u8>>(TARGET).read();
            // the ped scan of modes 2 and 3 (both call 0x515D80)
            let scan = |found: &mut bool| {
                let group = rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x130);
                for j in 0..16isize {
                    let q = rd::<GPtr<u8>>(group, 4 * j);
                    if q.is_null() {
                        continue;
                    }
                    let task = tm_find_active_task_by_type(rd::<GPtr<u8>>(q, 0x47C).byte_add(4), 0x44C);
                    if task.is_null() || rd::<GPtr<u8>>(task, 8) != find_player_ped(-1) {
                        continue;
                    }
                    *found = source_from(this, tt(), q, pp, op) != 0;
                    if *found {
                        exit_cam.write(q);
                        return;
                    }
                }
            };
            match m {
                1 => {
                    let now = g::<i32>(0xB7_CB84).read();
                    let mut v = xi(now);
                    if now < 0 {
                        v = v.fadd_m(f32::from_bits(0x4F80_0000));
                    }
                    g::<f32>(START_TIME).write(f32_of(v));
                    if head_view(this, cop, 1) != 0 {
                        let t = g::<GPtr<u8>>(TARGET).read();
                        wr::<u32>(t, 0x1C, rd::<u32>(t, 0x1C) & 0xFFFF_FF7F);
                        mode.write(1);
                        wr::<u8>(this, 0xA, 0);
                        return 1;
                    }
                }
                2 | 3 if !cop.is_null() => {
                    found = if m == 2 { source_from(this, tt(), cop, pp, op) } else { side_pos(this, tt(), cop, pp, op) } != 0;
                    exit_cam.write(cop);
                    cop = GPtr::NULL;
                }
                2 | 3 => {
                    if !ped.is_null() {
                        scan(&mut found);
                    }
                }
                8 => found = lamp_post(this, tt(), cop, pp, op) != 0,
                _ => {}
            }
            if found {
                if !exit_cam.read().is_null() {
                    entity_register_reference(exit_cam.read(), GPtr::from_addr(EXIT_CAM));
                }
                mode.write(modes[i]);
                if modes[i] == 3 {
                    let r = rnd(0x51_8A96).fmul_m(5.0);
                    let rf = x(f32_of(r));
                    let v = if !ge_or_nan(r, x(1.0)) {
                        3
                    } else if !ge_or_nan(rf, x(2.0)) {
                        4
                    } else if !ge_or_nan(rf, x(3.0)) {
                        5
                    } else if !ge_or_nan(rf, x(4.0)) {
                        6
                    } else {
                        7
                    };
                    mode.write(v);
                }
            }
            i += 1;
        }
        aim(this, out, &mut p);
        if mode.read() != 0 {
            wr::<u8>(this, 0xA, 0);
        }
        return 1;
    }
    let m = mode.read();
    if m == 1 {
        let t = g::<GPtr<u8>>(TARGET).read();
        wr::<u32>(t, 0x1C, rd::<u32>(t, 0x1C) & 0xFFFF_FF7F);
        let cop = get_cop(GPtr::NULL);
        return u8::from(head_view(this, cop, 0) != 0);
    }
    let t = g::<GPtr<u8>>(TARGET).read();
    if look_at(t, &mut p).is_none() {
        return 0;
    }
    let src = this.byte_add(0x19C);
    let source = [rd::<f32>(src, 0), rd::<f32>(src, 4), rd::<f32>(src, 8)];
    let ts = g::<f32>(TIME_STEP).read();
    match m {
        2 => {
            let ec = exit_cam.read();
            if ec.is_null() {
                return keep(this, &mut p);
            }
            let ok = source_from(this, t, ec, vp(&mut p), vp(&mut out));
            let zl = x(ts).fmul_m(g::<f32>(0x8C_C7EC).read()).fadd_m(rd::<f32>(this, 0x1A4));
            if !le_or_nan(x(out[2]), zl) {
                out[2] = f32_of(zl);
            }
            if ok == 0 {
                return keep(this, &mut p);
            }
        }
        4..=7 => {
            out = source;
            let front = this.byte_add(0x190);
            for k in 0..3usize {
                wr::<f32>(front, 4 * k as isize, f32_of(x(p[k]).fsub_m(source[k])));
            }
            normalise(front);
            let up = this.byte_add(0x1B4);
            wr::<u32>(up, 0, 0);
            wr::<u32>(up, 4, 0);
            wr::<f32>(up, 8, 1.0);
            let mut r = [0f32; 3];
            cross_product(vp(&mut r), front, up);
            normalise(vp(&mut r));
            if m == 6 || m == 7 {
                for c in r.iter_mut() {
                    *c = f32_of(x(*c).fmul_m(-1.0));
                }
            }
            if sphere_blocked(r, source, t) {
                return keep(this, &mut p);
            }
            step(&mut out, r, g::<f32>(0x8C_C7E4).read());
            if m == 5 || m == 7 {
                out[2] = f32_of(x(ts).fmul_m(g::<f32>(0x8C_C7E8).read()).fadd_m(out[2]));
            } else {
                let mut f = [0u8; 4];
                let gz = find_ground_z_for_3d_coord(out[0], out[1], out[2], GPtr::from_raw(f.as_mut_ptr()), GPtr::NULL);
                if f[0] != 0 {
                    out[2] = f32_of(X87::from_f64_exact(gz).fadd_m(g::<f32>(0x8C_C7F8).read()));
                }
            }
        }
        8 => {
            out = source;
            let front = this.byte_add(0x190);
            for k in 0..3usize {
                wr::<f32>(front, 4 * k as isize, f32_of(x(p[k]).fsub_m(out[k])));
            }
            wr::<u32>(this, 0x198, 0);
            normalise(front);
            let up = this.byte_add(0x1B4);
            wr::<u32>(up, 0, 0);
            wr::<u32>(up, 4, 0);
            wr::<f32>(up, 8, 1.0);
            let mut r = [0f32; 3];
            cross_product(vp(&mut r), front, up);
            normalise(vp(&mut r));
            let k10 = g::<f32>(0x8C_C7FC).read();
            let a = x(r[0]).fmul_m(k10);
            let b = x(r[1]).fmul_m(k10);
            let c = f32_of(x(r[2]).fmul_m(k10));
            let dx = x(p[0]).fsub_m(out[0]);
            let dy = f32_of(x(p[1]).fsub_m(out[1]));
            let dz = f32_of(x(p[2]).fsub_m(out[2]));
            wr::<f32>(front, 0, f32_of(dx.fadd(a)));
            wr::<f32>(front, 4, f32_of(x(dy).fadd(b)));
            wr::<f32>(front, 8, f32_of(x(dz).fadd_m(c)));
            wr::<u32>(this, 0x198, 0);
            normalise(front);
            let fv = [rd::<f32>(front, 0), rd::<f32>(front, 4), rd::<f32>(front, 8)];
            if sphere_blocked(fv, out, t) {
                return keep(this, &mut p);
            }
            step(&mut out, fv, g::<f32>(0x8C_C800).read());
        }
        _ => return keep(this, &mut p),
    }
    aim(this, out, &mut p);
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_ARREST_CAM_ONE];
