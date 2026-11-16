//! FRONTIER-2 turn 8, lane B (batch `frontier2-b18`): the cinema camera sequencer's two decisions,
//! `CCamera::IsItTimeForNewcam` (0x51D770) and `CCamera::TryToStartNewCamMode` (0x51E560), called by the five
//! `ProcessObbeCinemaCamera*` selectors (obbe.rs). Ported from the 1.0 US bytes with
//! `docs/re/specs/CAM/CCamera.md` as reading aid (gta-reversed has no body); unverified (docs/verify/HOWTO.md).
//! Both entries are `nop; jmp` into a protection fragment that only reserves the stack frame.

use sa_core::abi::HookReg;
use sa_core::field::v3_ptr as vp;
use sa_core::mem::GPtr;
use sa_core::timer::{TIME_MS, fild_u32};
use sa_core::hook;
use sa_math::fpu::{ge_or_nan, gt, le_or_nan, lt};
use sa_math::general::get_atan_of_xy_x87;
use sa_math::x87::X87;

use crate::common::{active_cam, f32_of, g, rd, wr, x, xi};
use crate::obbe::active_cam_under_water;
use sa_core::bindings::{
    get_pad, rw_camera_set_near_clip_plane, vector_magnitude, vector_normalise, world_get_is_line_of_sight_clear,
};
use sa_world::bindings::{
    camera_set_cam_position_for_fixed_mode, camera_take_control, entity_get_is_on_screen, find_player_coors,
    find_player_entity, find_player_ped, find_player_speed, find_player_vehicle, get_water_level_no_waves,
    multiply3x3, player_get_wanted_level, vector_add_assign, vector_sub, vehicle_pool_get_at,
};
use sa_world::world::common::entity_pos;
use sa_world::world::ground_z_for_coord::find_roof_z_for_3d_coord;
use sa_world::world::mvpa2::find_ground_z_for_3d_coord;

use crate::camproc::{
    cam_dw_birdy, cam_dw_cam_man, cam_dw_heli_chase, cam_dw_plane_cam1, cam_dw_plane_cam2, cam_dw_plane_cam3,
    cam_dw_plane_spotter,
};

/// .data: the longest shot (15000.0) and the absolute maximum (20000.0) in ms.
const MAX_SHOT_TIME: usize = 0x8C_CDF0;
const ABS_MAX_SHOT_TIME: usize = 0x8C_CDF8;
/// The right-stick "released" latch (u8) and the sequence step (i8, +1 / -1) the selectors add.
const STICK_LATCH: usize = 0x8C_CDF4;
const STEP: usize = 0x8C_C471;
/// `TheCamera.m_bUseNearClipScript`, written through its absolute address.
const THE_CAMERA_USE_NEAR_CLIP_SCRIPT: usize = 0xB6_F059;
/// `Scene.m_pRwCamera`
const SCENE_CAMERA: usize = 0xC1_703C;
/// `m_vecFixedModeSource` / `m_pTargetEntity` / `m_bUseNearClipScript` / `m_fNearClipScript`
const FIXED_SOURCE: isize = 0x86C;
const TARGET: isize = 0x958;
const USE_NEAR_CLIP: isize = 0x31;
const NEAR_CLIP: isize = 0x138;
/// The skimmer and the Rhino.
const MI_SKIMMER: i16 = 0x1CC;
const MI_RHINO: i16 = 0x1B0;
/// `CVehicle::m_nVehicleType` (+0x590): 5 = boat.
const VEHICLE_TYPE: isize = 0x590;

fn player_vehicle() -> GPtr<u8> {
    find_player_vehicle(-1, 0)
}

/// `GetIsLineOfSightClear(a, b, buildings only)`.
fn los(a: GPtr<u8>, b: GPtr<u8>) -> bool {
    world_get_is_line_of_sight_clear(a, b, 1, 0, 0, 0, 0, 0, 0) != 0
}

/// `LOS(FindPlayerCoors(-1), &m_vecFixedModeSource)`.
fn los_player_fixed(this: GPtr<u8>) -> bool {
    let mut p = [0f32; 3];
    let pp = find_player_coors(vp(&mut p).cast(), -1);
    los(pp.cast(), this.byte_add(FIXED_SOURCE))
}

/// `LOS(target position, active camera source)` (cases 0 and 8).
fn los_target_cam(this: GPtr<u8>) -> bool {
    let t = entity_pos(rd::<GPtr<u8>>(this, TARGET));
    los(t, active_cam(this).byte_add(0x19C))
}

/// `V = VectorSub(FindPlayerCoors(-1), m_vecFixedModeSource)`, with `V.z = 0` unless `keep_z`.
fn v_from_fixed(this: GPtr<u8>, keep_z: bool) -> [f32; 3] {
    let mut p = [0f32; 3];
    let mut out = [0f32; 3];
    let pp = find_player_coors(vp(&mut p).cast(), -1);
    let r = vector_sub(vp(&mut out), pp.cast(), this.byte_add(FIXED_SOURCE));
    let mut v = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
    if !keep_z {
        v[2] = 0.0;
    }
    v
}

/// `CVector::Magnitude` (0x4082C0) of a local, a fresh call at every use as the bytes do.
fn mag(v: &mut [f32; 3]) -> X87 {
    X87::from_f64_exact(vector_magnitude(vp(v).cast()))
}

/// `moving`: `V . FindPlayerSpeed(-1) > 0` with the speed's z term multiplied by 0.0. The cases add the three
/// products in different orders (`(V.y*s.y + s.z*0) + V.x*s.x` and others); with one term a signed zero (or a
/// NaN for an infinite z) every order gives the same register value.
fn moving(v: &[f32; 3]) -> bool {
    let s = find_player_speed(-1);
    let d = x(v[1]).fmul_m(rd::<f32>(s, 4)).fadd(x(rd::<f32>(s, 8)).fmul_m(0.0)).fadd(x(v[0]).fmul_m(rd::<f32>(s, 0)));
    gt(d, x(0.0))
}

/// `boatSkip`: the player is in a boat and the camera target is not the skimmer (FindPlayerVehicle called twice).
fn boat_skip(this: GPtr<u8>) -> bool {
    !player_vehicle().is_null()
        && rd::<i32>(player_vehicle(), VEHICLE_TYPE) == 5
        && rd::<i16>(rd::<GPtr<u8>>(this, TARGET), 0x22) != MI_SKIMMER
}

/// `RwCameraSetNearClipPlane(Scene.m_pRwCamera, 0.15f)`
fn near015() {
    rw_camera_set_near_clip_plane(g::<GPtr<u8>>(SCENE_CAMERA).read(), f32::from_bits(0x3E19_999A));
}

/// `now > start + ms` (unsigned 32-bit).
fn after(start: u32, ms: u32) -> bool {
    TIME_MS.get() > start.wrapping_add(ms)
}

/// `CCamera::IsItTimeForNewcam(int32 seq, uint32 startTimeMs)` (0x51D770, thiscall `ret 8`, AL = 1 to switch).
/// The elapsed time is `fild now (unsigned fix-up) - fisub start`, compared with 20000 in the register and stored
/// f32 for the 15000 tests. A right-stick push past 96 after a release (|x| <= 32 sets the latch) sets the step
/// to +1 / -1 and switches. Then `switch (seq)` (table 0x51E4E8; 9..14 and > 29 keep the camera).
#[hook(addr = 0x51D770, conv = thiscall, name = "CCamera::IsItTimeForNewcam", expect = "90 E9 EB C3 EE FF",
       this = "CCamera", unverified = "frontier2-b18", route = "L2", hazards = "none",
       note = "CVector::Magnitude's ST0 read as f64; the right-stick test of a NaN cannot happen (an int16)")]
pub fn is_it_time_for_newcam(this: GPtr<u8>, seq: i32, start: u32) -> u8 {
    u8::from(time_for_newcam(this, seq, start))
}

fn time_for_newcam(this: GPtr<u8>, seq: i32, start: u32) -> bool {
    if seq < 0 {
        return true;
    }
    let el = fild_u32(TIME_MS.get()).fsub(xi(start as i32));
    let el_f = f32_of(el);
    if gt(el, x(g::<f32>(ABS_MAX_SHOT_TIME).read())) {
        return true;
    }
    let s = f32::from(rd::<i16>(get_pad(0), 4));
    let a = x(s).fabs();
    if le_or_nan(a, x(32.0)) {
        g::<u8>(STICK_LATCH).write(1);
    } else if !le_or_nan(a, x(96.0)) && g::<u8>(STICK_LATCH).read() != 0 {
        g::<i8>(STEP).write(if gt(x(s), x(0.0)) { 1 } else { -1 });
        g::<u8>(STICK_LATCH).write(0);
        return true;
    }
    let long = || gt(x(el_f), x(g::<f32>(MAX_SHOT_TIME).read()));
    match seq {
        0 => {
            let v = player_vehicle();
            if !v.is_null() {
                if rd::<i32>(v, VEHICLE_TYPE) == 5 && rd::<i16>(rd::<GPtr<u8>>(this, TARGET), 0x22) != MI_SKIMMER {
                    return true;
                }
                if rd::<i16>(v, 0x22) == MI_RHINO || !los_target_cam(this) {
                    return true;
                }
            }
            if after(start, 5000) {
                return true;
            }
            near015();
            false
        }
        1 => {
            if long() || boat_skip(this) || !los_player_fixed(this) {
                return true;
            }
            let mut v = v_from_fixed(this, false);
            if gt(mag(&mut v), x(40.0)) && moving(&v) {
                return true;
            }
            lt(mag(&mut v), x(4.5))
        }
        2 => {
            if long() || boat_skip(this) || !los_player_fixed(this) {
                return true;
            }
            let mut v = v_from_fixed(this, false);
            if lt(mag(&mut v), x(2.0)) {
                let n = mag(&mut v).fmul_m(0.5);
                let n = if lt(n, x(0.05)) { x(0.05) } else { n };
                wr::<f32>(this, NEAR_CLIP, f32_of(n));
                wr::<u8>(this, USE_NEAR_CLIP, 1);
            }
            if gt(mag(&mut v), x(29.0)) && moving(&v) {
                return true;
            }
            if lt(mag(&mut v), x(2.0)) {
                return true;
            }
            near015();
            false
        }
        3 => {
            if long() || !los_player_fixed(this) {
                return true;
            }
            let mut v = v_from_fixed(this, false);
            if gt(mag(&mut v), x(48.0)) && moving(&v) {
                return true;
            }
            near015();
            false
        }
        4 | 6 => after(start, 3000),
        5 => {
            if long() || boat_skip(this) || !los_player_fixed(this) {
                return true;
            }
            let mut v = v_from_fixed(this, false);
            gt(mag(&mut v), x(38.0)) && moving(&v)
        }
        7 => after(start, 2000) && entity_get_is_on_screen(player_vehicle()) == 0,
        8 => {
            if long() || boat_skip(this) || !los_target_cam(this) || after(start, 1000) {
                return true;
            }
            wr::<u32>(this, NEAR_CLIP, 0x3F19_999A);
            wr::<u8>(this, USE_NEAR_CLIP, 1);
            false
        }
        15 | 16 | 17 => {
            if long() {
                return true;
            }
            if player_vehicle().is_null() {
                return false;
            }
            if !los_player_fixed(this) {
                return true;
            }
            let mut v = v_from_fixed(this, false);
            let (far, near) = match seq {
                15 => (44.0, 3.0),
                16 => (50.0, 3.0),
                _ => (50.0, 2.0),
            };
            if gt(mag(&mut v), x(far)) && (seq == 16 || moving(&v)) {
                return true;
            }
            lt(mag(&mut v), x(near))
        }
        18 | 19 => {
            if long() || !los_player_fixed(this) {
                return true;
            }
            let mut v = v_from_fixed(this, seq == 18);
            let (far, near) = if seq == 18 { (57.0, 1.0) } else { (36.0, 2.0) };
            if gt(mag(&mut v), x(far)) {
                return true;
            }
            lt(mag(&mut v), x(near))
        }
        20..=23 | 26..=28 => !dw_cam(this, seq),
        24 | 25 => {
            g::<u8>(THE_CAMERA_USE_NEAR_CLIP_SCRIPT).write(0);
            true
        }
        29 => after(start, 5000),
        _ => false,
    }
}

/// `m_aCams[m_nActiveCam].Process_DW_X(true)` for the DW sequence ids 20..23, 26..28.
fn dw_cam(this: GPtr<u8>, seq: i32) -> bool {
    let c = active_cam(this);
    let f: fn(GPtr<u8>, u32) -> u8 = match seq {
        20 => cam_dw_heli_chase,
        21 => cam_dw_cam_man,
        22 => cam_dw_birdy,
        23 => cam_dw_plane_spotter,
        26 => cam_dw_plane_cam1,
        27 => cam_dw_plane_cam2,
        _ => cam_dw_plane_cam3,
    };
    f(c, 1) != 0
}

/// .data water offsets of the angled cameras: 1.0 (0x8CC8C0, copied to a local first) and -2.0 for boats
/// (0x8CC8C4, read in place).
const WATER_OFFSET: usize = 0x8C_C8C0;
const WATER_OFFSET_BOAT: usize = 0x8C_C8C4;
/// `CPools::ms_pVehiclePool`
const VEHICLE_POOL: usize = 0xB7_4494;
/// The wheel camera's offset in the car's frame (-1.4, -2.3, 0.3).
const WHEEL_CAM_OFFSET: [u32; 3] = [0xBFB3_3333, 0xC013_3333, 0x3E99_999A];
/// eCamMode values used here.
const MODE_WHEELCAM: u32 = 14;
const MODE_FIXED: u32 = 15;
const MODE_1STPERSON: u32 = 16;
const MODE_CAM_ON_A_STRING: u32 = 18;
/// 0.8 (0x858C98): the police car's heading test.
const K_0_8: u32 = 0x3F4C_CCCD;

fn rd3(p: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

/// `FindPlayerCoors(-1)` copied.
fn coors() -> [f32; 3] {
    let mut b = [0f32; 3];
    rd3(find_player_coors(vp(&mut b).cast(), -1).cast())
}

/// `P = FindPlayerCoors(-1)`, `S = FindPlayerSpeed(-1)` with `S.z = 0`, normalised.
fn coors_and_dir() -> ([f32; 3], [f32; 3]) {
    let p = coors();
    let mut s = rd3(find_player_speed(-1));
    s[2] = 0.0;
    vector_normalise(vp(&mut s).cast());
    (p, s)
}

fn is_boat(v: GPtr<u8>) -> bool {
    rd::<i32>(v, VEHICLE_TYPE) == 5
}

/// The "x87 position pattern" (cases 1, 2, 3, 5): `C.x = (P.x + S.x*k) + S.y*q` (the first sum stored f32),
/// `C.y = (S.y*k + P.y) + S.x*nq` (unrounded first sum), `C.z = f32(S.z*k) + P.z`; case 3 adds `lift` to the
/// unrounded z sum.
fn side_pos(p: [f32; 3], s: [f32; 3], k: f32, q: f32, nq: f32, lift: Option<f32>) -> [f32; 3] {
    let a = x(s[0]).fmul_m(k);
    let b = x(s[1]).fmul_m(k);
    let c = f32_of(x(s[2]).fmul_m(k));
    let cx1 = f32_of(a.fadd_m(p[0]));
    let t = b.fadd_m(p[1]);
    let z = x(c).fadd_m(p[2]);
    let cz = f32_of(match lift {
        Some(l) => z.fadd_m(l),
        None => z,
    });
    let cx = f32_of(x(s[1]).fmul_m(q).fadd_m(cx1));
    let cy = f32_of(t.fadd(x(s[0]).fmul_m(nq)));
    [cx, cy, cz]
}

/// Ground / roof fix: `g = FindGroundZFor3DCoord(C.x, C.y, C.z + 5, &f, null)`; if found `C.z = g + off`; else
/// `r = FindRoofZFor3DCoord(C.x, C.y, C.z - 5, &f)`; if found `C.z = r + off` (the results stay in ST0).
fn ground_fix(c: &mut [f32; 3], off: f32) {
    let mut found = 0u8;
    let g = find_ground_z_for_3d_coord(c[0], c[1], f32_of(x(c[2]).fadd_m(5.0)), GPtr::from_raw(&mut found), GPtr::NULL);
    if found != 0 {
        c[2] = f32_of(g.fadd_m(off));
        return;
    }
    let r = find_roof_z_for_3d_coord(c[0], c[1], f32_of(x(c[2]).fsub_m(5.0)), GPtr::from_raw(&mut found));
    if found != 0 {
        c[2] = f32_of(r.fadd_m(off));
    }
}

/// The water rule of cases 16..19: `GetWaterLevelNoWaves(C, &lvl, 0, 0)`; then `w = (boat ? -2.0 : 1.0) + lvl`
/// and `C.z = w` when `C.z < w` (`test ah, 5; jp` keeps C.z on >= or NaN).
fn water_fix(c: &mut [f32; 3]) {
    let mut lvl = 0f32;
    let hit = get_water_level_no_waves(
        c[0].to_bits(),
        c[1].to_bits(),
        c[2].to_bits(),
        GPtr::from_raw(&mut lvl),
        GPtr::NULL,
        GPtr::NULL,
    );
    if hit == 0 {
        return;
    }
    let local = g::<f32>(WATER_OFFSET).read();
    let base = if !player_vehicle().is_null() && is_boat(player_vehicle()) {
        g::<f32>(WATER_OFFSET_BOAT).read()
    } else {
        local
    };
    let w = x(base).fadd_m(lvl);
    if lt(x(c[2]), w) {
        c[2] = f32_of(w);
    }
}

/// `V = VectorSub(FindPlayerCoors(-1), C)`, 3-D unless `flat` (V.z = 0).
fn v_from(c: &mut [f32; 3], flat: bool) -> [f32; 3] {
    let mut p = [0f32; 3];
    let mut out = [0f32; 3];
    let pp = find_player_coors(vp(&mut p).cast(), -1);
    let r = vector_sub(vp(&mut out), pp.cast(), vp(c));
    let mut v = rd3(r);
    if flat {
        v[2] = 0.0;
    }
    v
}

/// `LOS(FindPlayerCoors(-1), C)`.
fn los_player(c: &mut [f32; 3]) -> bool {
    let mut p = [0f32; 3];
    let pp = find_player_coors(vp(&mut p).cast(), -1);
    los(pp.cast(), vp(c))
}

/// `SetCamPositionForFixedMode(&C, &(0, 0, 0))` then `TakeControl(FindPlayerEntity(-1), mode, 2, 2)`.
fn fixed(this: GPtr<u8>, c: &mut [f32; 3], mode: u32) {
    let mut up = [0f32; 3];
    camera_set_cam_position_for_fixed_mode(this, vp(c), vp(&mut up));
    take_control_player(this, mode);
}

fn take_control_player(this: GPtr<u8>, mode: u32) {
    camera_take_control(this, find_player_entity(-1), mode, 2, 2);
}

/// The common tail `return !uw()` (0x50B830: the active camera is under water).
fn not_under_water() -> bool {
    active_cam_under_water() == 0
}

/// `a = GetATanOfXY(S.x, S.y) + k` kept in the register; `sin a` stored f32 first, then `S.x = f32(cos a + S.x)`,
/// `S.y = f32(sin + S.y)`.
fn turn(s: &mut [f32; 3], a: X87) {
    let sin = f32_of(a.fsin());
    let cos = a.fcos();
    s[0] = f32_of(cos.fadd_m(s[0]));
    s[1] = f32_of(x(sin).fadd_m(s[1]));
}

/// `C = (S.x*d + P.x, P.y + S.y*d, f32(S.z*d) + P.z)` (cases 15..19; the products formed first).
fn ahead(p: [f32; 3], s: [f32; 3], d: f32) -> [f32; 3] {
    let a = x(s[0]).fmul_m(d);
    let b = x(s[1]).fmul_m(d);
    let cz = f32_of(x(s[2]).fmul_m(d));
    [f32_of(a.fadd_m(p[0])), f32_of(x(p[1]).fadd(b)), f32_of(x(cz).fadd_m(p[2]))]
}

/// The distance tail of cases 15..19: `!LOS -> false`; V 3-D (VectorSub called twice by 15 / 16 / 17, once by
/// 18 / 19); `|V| > far` (and moving when `need_moving`) and `|V| < near` refuse; else the fixed camera.
fn ahead_tail(this: GPtr<u8>, c: &mut [f32; 3], far: f32, near: f32, need_moving: bool, twice: bool) -> bool {
    if !los_player(c) {
        return false;
    }
    let mut v = v_from(c, false);
    let v2 = if twice { v_from(c, false) } else { v };
    if gt(mag(&mut v), x(far)) && (!need_moving || moving(&v2)) {
        return false;
    }
    if lt(mag(&mut v), x(near)) {
        return false;
    }
    fixed(this, c, MODE_FIXED);
    not_under_water()
}

/// Cases 16 / 17: the normalised direction turned by `k` and normalised again, `C = ahead(P, S, d)`, then
/// `C.z = P.z - h` (P re-read, stored) and `r = FindRoofZFor3DCoord(C, &f)` (f starts 0): `f == 1` -> `C.z = f32(r)
/// + 0.5`; else the water rule.
fn angled_low(this: GPtr<u8>, k: u32, d: f32, h: f32, far: f32, near: f32, need_moving: bool) -> bool {
    let (p, mut s) = coors_and_dir();
    let a = get_atan_of_xy_x87(s[0], s[1]).fadd_m(f32::from_bits(k));
    turn(&mut s, a);
    vector_normalise(vp(&mut s).cast());
    let mut c = ahead(p, s, d);
    c[2] = f32_of(x(coors()[2]).fsub_m(h));
    let mut found = 0u8;
    let r = f32_of(find_roof_z_for_3d_coord(c[0], c[1], c[2], GPtr::from_raw(&mut found)));
    if found == 1 {
        c[2] = f32_of(x(r).fadd_m(0.5));
    } else {
        water_fix(&mut c);
    }
    ahead_tail(this, &mut c, far, near, need_moving, true)
}

/// Cases 18 / 19: `P.z = boat ? P.z + up : P.z - down` (stored), the raw speed's angle plus `k`, `S.z = 0`, turned
/// and normalised, `C = ahead(P, S, d)`; the ground height is tested against 1.0 instead of the found flag
/// (original bug, `fcomp 1.0; test ah, 0x44; jp`): equal -> `C.z = f32(g) + 0.5` when `C.z < f32(g)`; else the
/// water rule.
fn angled_high(this: GPtr<u8>, up: f32, down: f32, k: u32, d: f32, far: f32, near: f32) -> bool {
    let mut p = coors();
    p[2] = if !player_vehicle().is_null() && is_boat(player_vehicle()) {
        f32_of(x(p[2]).fadd_m(up))
    } else {
        f32_of(x(p[2]).fsub_m(down))
    };
    let mut s = rd3(find_player_speed(-1));
    let a = get_atan_of_xy_x87(s[0], s[1]).fadd_m(f32::from_bits(k));
    s[2] = 0.0;
    turn(&mut s, a);
    vector_normalise(vp(&mut s).cast());
    let a = x(s[0]).fmul_m(d);
    let b = x(s[1]).fmul_m(d);
    let cz = f32_of(x(s[2]).fmul_m(d));
    let mut c = [f32_of(x(p[0]).fadd(a)), f32_of(x(p[1]).fadd(b)), f32_of(x(cz).fadd_m(p[2]))];
    let mut found = 0u8;
    let gx = find_ground_z_for_3d_coord(c[0], c[1], c[2], GPtr::from_raw(&mut found), GPtr::NULL);
    let g_f = f32_of(gx);
    if !gx.fcom(x(1.0)).test_parity(0x44) {
        if lt(x(c[2]), x(g_f)) {
            c[2] = f32_of(x(g_f).fadd_m(0.5));
        }
    } else {
        water_fix(&mut c);
    }
    ahead_tail(this, &mut c, far, near, false, false)
}

/// The tail of cases 1 and 2: `!LOS -> false`, V with `V.z = 0`, `|V| > far && moving -> false`, `|V| < near ->
/// false`.
fn side_ok(c: &mut [f32; 3], far: f32, near: f32) -> bool {
    if !los_player(c) {
        return false;
    }
    let mut v = v_from(c, true);
    if gt(mag(&mut v), x(far)) && moving(&v) {
        return false;
    }
    !lt(mag(&mut v), x(near))
}

/// A police car candidate of cases 7 / 8: an automobile other than the player's with bit 0 of +0x428 (and, for
/// 7, status 3 in +0x36), `|P - pos| < 30` (`test ah, 5; jp` skips on >= or NaN), behind the player's vehicle
/// (`dy*fwd.y + dx*fwd.x < 0`, dx / dy stored f32 with P fresh per coordinate) and heading the same way
/// (`c.fwd . pv.fwd > 0.8` on x / y). The matrices are read without a null check, as the bytes do.
fn police_candidate(c: GPtr<u8>, status_test: bool) -> bool {
    if c.is_null() || rd::<i32>(c, VEHICLE_TYPE) != 0 || c == player_vehicle() || rd::<u8>(c, 0x428) & 1 == 0 {
        return false;
    }
    if status_test && rd::<u8>(c, 0x36) & 0xF8 != 0x18 {
        return false;
    }
    let dx = f32_of(x(rd::<f32>(entity_pos(c), 0)).fsub_m(coors()[0]));
    let dy = f32_of(x(rd::<f32>(entity_pos(c), 4)).fsub_m(coors()[1]));
    let mut p = [0f32; 3];
    let mut out = [0f32; 3];
    let pp = find_player_coors(vp(&mut p).cast(), -1);
    let r = vector_sub(vp(&mut out), pp.cast(), entity_pos(c));
    if ge_or_nan(X87::from_f64_exact(vector_magnitude(r.cast())), x(30.0)) {
        return false;
    }
    let m1 = rd::<GPtr<u8>>(player_vehicle(), 0x14);
    let m2 = rd::<GPtr<u8>>(player_vehicle(), 0x14);
    let behind = x(dy).fmul_m(rd::<f32>(m2, 0x14)).fadd(x(dx).fmul_m(rd::<f32>(m1, 0x10)));
    if ge_or_nan(behind, x(0.0)) {
        return false;
    }
    let cm = rd::<GPtr<u8>>(c, 0x14);
    let pm1 = rd::<GPtr<u8>>(player_vehicle(), 0x14);
    let pm2 = rd::<GPtr<u8>>(player_vehicle(), 0x14);
    let heading =
        x(rd::<f32>(pm2, 0x14)).fmul_m(rd::<f32>(cm, 0x14)).fadd(x(rd::<f32>(pm1, 0x10)).fmul_m(rd::<f32>(cm, 0x10)));
    !le_or_nan(heading, x(f32::from_bits(K_0_8)))
}

/// The wheel camera point `W = Multiply3x3(v->m_matrix, (-1.4, -2.3, 0.3))`, `W += pos(v)` (operator+=).
fn wheel_point(v: GPtr<u8>) -> [f32; 3] {
    let mut off = WHEEL_CAM_OFFSET.map(f32::from_bits);
    let mut tmp = [0f32; 3];
    let mut w = rd3(multiply3x3(vp(&mut tmp), rd::<GPtr<u8>>(v, 0x14), vp(&mut off)));
    vector_add_assign(vp(&mut w), entity_pos(v));
    w
}

/// `CCamera::TryToStartNewCamMode(int32 seq)` (0x51E560, thiscall `ret 4`, AL = 1 when a camera was started).
/// `switch (seq)` (table 0x520110; 4, 9..14 and > 29 refuse). The fixed-camera cases place the camera from the
/// player's position and direction (with the bytes' x87 orders), check the line of sight and distances, then
/// take control in MODE_FIXED and refuse when the camera is under water; 0 / 8 are wheel cameras, 7 a police
/// car on a string, 20..28 the DW cameras. Case 1 computes its position before the boat test (case 2 after).
#[hook(addr = 0x51E560, conv = thiscall, name = "CCamera::TryToStartNewCamMode", expect = "90 E9 7C 2F EE FF",
       this = "CCamera", unverified = "frontier2-b18", route = "L2", hazards = "x87-transcendental",
       note = "FSIN / FCOS of the angled cameras; CVector::Magnitude's ST0 read as f64; FindGroundZ / FindRoofZ called as ports (ST0 kept); their found flag starts at 0 (uninitialised in the original before the first call, which always writes it)")]
pub fn try_to_start_new_cam_mode(this: GPtr<u8>, seq: i32) -> u8 {
    u8::from(try_start(this, seq))
}

fn try_start(this: GPtr<u8>, seq: i32) -> bool {
    match seq {
        0 => {
            let v = player_vehicle();
            if v.is_null() {
                return false;
            }
            if is_boat(v) && rd::<i16>(rd::<GPtr<u8>>(this, TARGET), 0x22) != MI_SKIMMER {
                return false;
            }
            if rd::<i16>(v, 0x22) == MI_RHINO {
                return false;
            }
            let mut w = wheel_point(player_vehicle());
            let _ = player_vehicle();
            if !los(entity_pos(v), vp(&mut w)) {
                return false;
            }
            camera_take_control(this, v, MODE_WHEELCAM, 2, 2);
            true
        }
        1 => {
            let (p, s) = coors_and_dir();
            let mut c = side_pos(p, s, 20.0, 3.0, -3.0, None);
            if boat_skip(this) {
                return false;
            }
            ground_fix(&mut c, 1.5);
            if !side_ok(&mut c, 40.0, 4.5) {
                return false;
            }
            fixed(this, &mut c, MODE_FIXED);
            not_under_water()
        }
        2 => {
            if boat_skip(this) {
                return false;
            }
            let (p, s) = coors_and_dir();
            let mut c = side_pos(p, s, 16.0, 2.5, -2.5, None);
            ground_fix(&mut c, 0.5);
            if !side_ok(&mut c, 29.0, 2.0) {
                return false;
            }
            fixed(this, &mut c, MODE_FIXED);
            near015();
            not_under_water()
        }
        3 => {
            let (p, s) = coors_and_dir();
            let mut c = side_pos(p, s, 30.0, 8.0, -8.0, Some(16.0));
            if !los_player(&mut c) {
                return false;
            }
            fixed(this, &mut c, MODE_FIXED);
            if !not_under_water() {
                return false;
            }
            near015();
            true
        }
        5 => {
            let (p, s) = coors_and_dir();
            let mut c = side_pos(p, s, 30.0, -6.0, 6.0, None);
            ground_fix(&mut c, 3.5);
            if !los_player(&mut c) {
                return false;
            }
            fixed(this, &mut c, MODE_FIXED);
            not_under_water()
        }
        6 => {
            take_control_player(this, MODE_1STPERSON);
            true
        }
        7 | 8 => police_cam(this, seq == 7),
        15 => {
            if player_vehicle().is_null() {
                return false;
            }
            let (p, s) = coors_and_dir();
            let mut c = ahead(p, s, 34.0);
            c[2] = f32_of(x(coors()[2]).fadd_m(0.5));
            if is_boat(player_vehicle()) {
                c[2] = f32_of(x(c[2]).fadd_m(1.0));
            }
            ahead_tail(this, &mut c, 44.0, 3.0, true, true)
        }
        16 | 17 => {
            if player_vehicle().is_null() {
                return false;
            }
            if seq == 16 {
                angled_low(this, 0x3F86_0A92, 30.0, 5.5, 50.0, 3.0, false)
            } else {
                angled_low(this, 0x4054_3B67, 25.0, 1.0, 50.0, 2.0, true)
            }
        }
        18 => angled_high(this, 23.0, 23.0, 0x4021_F770, 15.0, 57.0, 1.0),
        19 => angled_high(this, 4.0, 1.0, 0x3EFA_35DD, 12.5, 36.0, 2.0),
        20..=23 | 26..=28 => {
            if !dw_cam(this, seq) {
                return false;
            }
            let mode = match seq {
                20 => 56,
                21 => 57,
                22 => 58,
                23 => 59,
                26 => 62,
                27 => 63,
                _ => 64,
            };
            take_control_player(this, mode);
            not_under_water()
        }
        24 | 25 => {
            g::<u8>(THE_CAMERA_USE_NEAR_CLIP_SCRIPT).write(0);
            false
        }
        29 => {
            take_control_player(this, MODE_CAM_ON_A_STRING);
            true
        }
        _ => false,
    }
}

/// Cases 7 (police car on a string, with the status test) and 8 (police wheel camera, aborting the whole scan on
/// the first blocked line of sight): the wanted level must be at least 1 and the player in a vehicle (not a boat
/// unless the target is the skimmer); the vehicle pool is scanned from the top; a started camera that is under
/// water continues the scan.
fn police_cam(this: GPtr<u8>, string: bool) -> bool {
    if player_get_wanted_level(find_player_ped(-1)) < 1 || player_vehicle().is_null() {
        return false;
    }
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let mut n = rd::<i32>(pool, 8);
    if boat_skip(this) {
        return false;
    }
    while n != 0 {
        n -= 1;
        let c = vehicle_pool_get_at(pool, n as u32);
        if !police_candidate(c, string) {
            continue;
        }
        if string {
            camera_take_control(this, c, MODE_CAM_ON_A_STRING, 2, 2);
        } else {
            let mut w = wheel_point(c);
            if !los(entity_pos(c), vp(&mut w)) {
                return false;
            }
            camera_take_control(this, c, MODE_WHEELCAM, 2, 2);
        }
        if not_under_water() {
            return true;
        }
    }
    false
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_IS_IT_TIME_FOR_NEWCAM, HOOK_TRY_TO_START_NEW_CAM_MODE];
