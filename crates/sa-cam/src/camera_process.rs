//! STD-6: `CCamera::Process` (0x52B730), the per-frame camera update. Ported from the 1.0 US bytes;
//! `docs/re/specs/CAM/CCamera.md` (0x52B730) as the reading aid (its step numbers are used below).
//!
//! The original's stack frame is modelled by `fr` (offsets are the frame's, ESP after the four register
//! pushes, so `B+0x30` is `src`): the callees get pointers into it exactly as the original passes them.
//! Slots the original leaves uninitialised on some paths (the transition with `fS > 1` / NaN, spec open
//! question 2) read 0 here.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{active_cam, cam_of, ftol32, g, k, rd, wr, x};

orig! {
    /// `CCollision::ResetMadeInvisibleObjects()` (0x415540)
    fn reset_made_invisible_objects() = cdecl 0x415540 => unported;
    /// `CCamera::UpdateTargetEntity()` (0x50C360)
    fn update_target_entity(this: GPtr<u8>) = thiscall 0x50C360 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CCamera::CamControl()` (0x527FA0)
    fn cam_control(this: GPtr<u8>) = thiscall 0x527FA0 => unported;
    /// `CCamera::ProcessVectorMoveLinear()` (0x5164A0)
    fn process_vector_move_linear(this: GPtr<u8>) = thiscall 0x5164A0 => unported;
    /// `CCamera::ProcessVectorTrackLinear()` (0x516440)
    fn process_vector_track_linear(this: GPtr<u8>) = thiscall 0x516440 => unported;
    /// `CCamera::ProcessFOVLerp()` (0x516500)
    fn process_fov_lerp(this: GPtr<u8>) = thiscall 0x516500 => unported;
    /// `CCamera::ProcessFade()` (0x50B5D0)
    fn process_fade(this: GPtr<u8>) = thiscall 0x50B5D0 => unported;
    /// `CCamera::ProcessMusicFade()` (0x50B6D0)
    fn process_music_fade(this: GPtr<u8>) = thiscall 0x50B6D0 => unported;
    /// `CCamera::ProcessWideScreenOn()` (0x50B890)
    fn process_wide_screen_on(this: GPtr<u8>) = thiscall 0x50B890 => unported;
    /// `RwCameraSetNearClipPlane(RwCamera*, float)` (0x7EE1D0); the float as its bits
    fn rw_camera_set_near_clip_plane(cam: GPtr<u8>, near: u32) -> GPtr<u8> = cdecl 0x7EE1D0 => unported;
    /// `RwCameraSetFarClipPlane(RwCamera*, float)` (0x7EE2A0); the float as its bits
    fn rw_camera_set_far_clip_plane(cam: GPtr<u8>, far: u32) -> GPtr<u8> = cdecl 0x7EE2A0 => unported;
    /// `CGeneral::GetATanOfXY(float, float)` (0x53CC70, ST0); the floats as their bits
    fn get_atan_of_xy(xb: u32, yb: u32) -> f64 = cdecl 0x53CC70 => unported;
    /// `CCam::Process()` (0x526FC0)
    fn cam_process(cam: GPtr<u8>) = thiscall 0x526FC0 => unported;
    /// `CCamera::ProcessShake(float)` (0x516560)
    fn process_shake(this: GPtr<u8>, ratio: u32) = thiscall 0x516560 => unported;
    /// `CCamera::StoreValuesDuringInterPol(CVector&, CVector&, CVector&, float&)` (0x50C290)
    fn store_values_during_inter_pol(this: GPtr<u8>, s: GPtr<u8>, t: GPtr<u8>, u: GPtr<u8>, f: GPtr<u8>) = thiscall 0x50C290 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730), returns `out`
    fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `VectorSub(CVector* out, const CVector*, const CVector*)` (0x40FE60), returns `out`
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `operator*(CVector*, const CVector&, float)` (0x40FEC0), returns `out`; the float as its bits
    fn vector_scale(out: GPtr<u8>, v: GPtr<u8>, s: u32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `operator+(CVector*, const CVector&, const CVector&)` (0x40FE30), returns `out`
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `CCam::KeepTrackOfTheSpeed(...)` (0x509DF0)
    fn keep_track_of_the_speed(cam: GPtr<u8>, s: GPtr<u8>, t: GPtr<u8>, u: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, f: GPtr<u8>) = thiscall 0x509DF0 => unported;
    /// `CCullZones::CamStairsForPlayer()` (0x72DD30, AL)
    fn cam_stairs_for_player() -> u8 = cdecl 0x72DD30 => unported;
    /// `CWorld::ProcessLineOfSight(...)` (0x56BA00, AL)
    fn process_line_of_sight(a: GPtr<u8>, b: GPtr<u8>, cp: GPtr<u8>, ent: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, ic: u32, sh: u32) -> u8 = cdecl 0x56BA00 => unported;
    /// `CDraw::SetFOV(float)` (0x6FF410); the float as its bits
    fn draw_set_fov(fov: u32) = cdecl 0x6FF410 => unported;
    /// `CCamera::CalculateDerivedValues(bool, bool)` (0x5150E0)
    fn calculate_derived_values(this: GPtr<u8>, a: u32, b: u32) = thiscall 0x5150E0 => unported;
    /// `CCamera::CopyCameraMatrixToRWCam(bool)` (0x50AFA0)
    fn copy_camera_matrix_to_rw_cam(this: GPtr<u8>, update: u32) = thiscall 0x50AFA0 => unported;
    /// `CCamera::UpdateSoundDistances()` (0x515BD0)
    fn update_sound_distances(this: GPtr<u8>) = thiscall 0x515BD0 => unported;
    /// `CWaterLevel::GetWaterLevel(float, float, float, float*, bool, CVector*)` (0x6EB690, AL)
    fn get_water_level(xb: u32, yb: u32, zb: u32, level: GPtr<u8>, waves: u32, normal: GPtr<u8>) -> u8 = cdecl 0x6EB690 => unported;
}

/// 0.0 (0x858B50)
const K_0: u32 = 0;
/// 1.0 (0x858624)
const K_1: u32 = 0x3F80_0000;
/// 0.5 (0x858B8C)
const K_HALF: u32 = 0x3F00_0000;
/// pi (0x858CB8)
const K_PI: u32 = 0x4049_0FDB;
/// 2 pi (0x858CBC)
const K_2PI: u32 = 0x40C9_0FDB;
/// -pi (0x858CC0)
const K_MPI: u32 = 0xC049_0FDB;
/// 0.3 (0x858C24)
const K_SWITCH: u32 = 0x3E99_999A;
/// 2^32 (0x858C54), the unsigned `fild` fix-up
const K_2P32: u32 = 0x4F80_0000;
/// 1.3 (0x8CCF20, `.data`), the minimum source distance from the target while interpolating
const K_MIN_DIST: u32 = 0x3FA6_6666;
/// pi / 180 (0x8595EC)
const K_DEG2RAD: u32 = 0x3C8E_FA35;
/// -0.02 (0x85904C)
const K_DRUNK_SRC: u32 = 0xBCA3_D70A;
/// 0.05 (0x858C28)
const K_DRUNK_UP: u32 = 0x3D4C_CCCD;
/// -0.1 (0x858EF4)
const K_DRUNK_FRONT: u32 = 0xBDCC_CCCD;
/// 5.0 (0x858C80)
const K_DRUNK_STEP: u32 = 0x40A0_0000;
/// 0.00028 (0x863280)
const K_SHAKE_DECAY: u32 = 0x3992_CCF7;
/// 2.0 (0x858CA0)
const K_2: u32 = 0x4000_0000;
/// 0.1 (0x858B1C)
const K_01: u32 = 0x3DCC_CCCD;
/// -255.0 (0x86327C)
const K_M255: u32 = 0xC37F_0000;
/// 0.2 (0x858CC4)
const K_02: u32 = 0x3E4C_CCCD;
/// 70.0 (0x858CE0)
const K_70: u32 = 0x428C_0000;
/// 100.0 (0x858628)
const K_100: u32 = 0x42C8_0000;
/// 0.01 (0x858C58)
const K_001: u32 = 0x3C23_D70A;
/// 0.4 (0x858EE8)
const K_04: u32 = 0x3ECC_CCCD;
/// 0.6 (0x858CC8)
const K_06: u32 = 0x3F19_999A;
/// 0.8333333 (0x863278)
const K_5_6: u32 = 0x3F55_5555;

/// `Scene.m_pRwCamera` (0xC1703C).
const SCENE_CAMERA: usize = 0xC1703C;
/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84).
const TIME_MS: usize = 0xB7CB84;

/// `fild dword` of an unsigned value: + 2^32 when the sign bit is set.
fn fild_u(v: u32) -> X87 {
    let r = X87::from_i32(v as i32);
    if (v as i32) < 0 { r.fadd_m(k(K_2P32)) } else { r }
}

/// `(i + 1) % 2` with the original's signed-modulo idiom (`and 0x80000001; jns; dec; or -2; inc`).
fn other_cam(i: u8) -> i32 {
    (i32::from(i) + 1) % 2
}

/// `GetATanOfXY(front.x, front.y)` unless both are 0 (`test ah,0x44; jp` on each): step 6's beta.
fn front_beta(cam: GPtr<u8>) -> f32 {
    let (fx, fy) = (rd::<f32>(cam, 0x190), rd::<f32>(cam, 0x194));
    let z = x(k(K_0));
    if x(fx).fcom(z).test_parity(0x44) || x(fy).fcom(z).test_parity(0x44) {
        X87::from_f64_exact(get_atan_of_xy(fx.to_bits(), fy.to_bits())).to_f32()
    } else {
        0.0
    }
}

/// `0.5 - cos(u * pi) * 0.5` (`fmul pi; fcos; fmul 0.5; fsubr 0.5`).
fn ease(u: X87) -> X87 {
    u.fmul_m(k(K_PI)).fcos().fmul_m(k(K_HALF)).fsubr_m(k(K_HALF))
}

/// `CCamera::Process()` (0x52B730, thiscall, plain `ret`). See the module docs and the spec's steps.
#[hook(addr = 0x52B730, conv = thiscall, name = "CCamera::Process", expect = "81 EC A0 00 00 00 53 55 56 57 8B F1", this = "CCamera",
       unverified = "std6-cam-1", route = "L2", hazards = "none", recs = "new-game")]
#[allow(clippy::too_many_lines)]
pub fn camera_process(this: GPtr<u8>) {
    let mut fr = [0u32; 0x30];
    let b = GPtr::<u32>::from_raw(fr.as_mut_ptr()).cast::<u8>();
    let f = |o: isize| rd::<f32>(b, o);
    let p = |o: isize| b.byte_add(o);
    let set = |o: isize, v: X87| wr::<f32>(b, o, v.to_f32());
    let setf = |o: isize, v: f32| wr::<f32>(b, o, v);
    let copy3 = |dst: GPtr<u8>, src: GPtr<u8>| {
        for i in 0..3 {
            wr::<u32>(dst, i * 4, rd::<u32>(src, i * 4));
        }
    };
    let pos_of = |e: GPtr<u8>| {
        let m: GPtr<u8> = rd(e, 0x14);
        if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
    };
    let now = || g::<u32>(TIME_MS).read();
    let scene_cam = || g::<GPtr<u8>>(SCENE_CAMERA).read();

    // 1.
    reset_made_invisible_objects();
    setf(0x58, 0.0);
    wr::<u8>(this, 0x2A, 0);
    copy3(this.byte_add(0x848), pos_of(this));
    // 2.
    if rd::<u8>(this, 0x2B) == 1 || rd::<u8>(this, 0x34) == 1 || rd::<i32>(this, 0xB0) == 2 {
        update_target_entity(this);
    }
    // 3.
    let target_slot = this.byte_add(0x958);
    if rd::<u32>(this, 0x958) == 0 {
        let ped = find_player_ped(-1);
        wr(this, 0x958, ped);
        register_reference(ped, target_slot);
    }
    for j in [None, Some(())] {
        let i = i32::from(rd::<u8>(this, 0x59));
        let i = if j.is_some() { other_cam(i as u8) } else { i };
        let slot = cam_of(this, i).byte_add(0x21C);
        if rd::<u32>(slot, 0) == 0 {
            wr::<u32>(slot, 0, rd::<u32>(this, 0x958));
            register_reference(rd(slot, 0), slot);
        }
    }
    // 4.
    cam_control(this);
    let tc = GPtr::<u8>::from_addr(0xB6F028);
    process_vector_move_linear(tc);
    process_vector_track_linear(tc);
    process_fov_lerp(tc);
    if rd::<u8>(this, 0x51) != 0 {
        process_fade(this);
    }
    if rd::<u8>(this, 0x52) != 0 {
        process_music_fade(this);
    }
    if rd::<u8>(this, 0x3D) != 0 {
        process_wide_screen_on(this);
    }
    // 5.
    rw_camera_set_near_clip_plane(scene_cam(), 0x3E99_999A);
    // 6.
    setf(0x1C, front_beta(active_cam(this)));
    cam_process(active_cam(this));
    setf(0x20, front_beta(active_cam(this)));
    // 7.
    if rd::<u8>(this, 0x58) != 0 && now() > rd::<u32>(this, 0x78).wrapping_add(rd::<u32>(this, 0x7C)) {
        wr::<u8>(this, 0x58, 0);
        wr::<u8>(this, 0x4F, 0);
        wr::<u8>(this, 0x3A, 0);
    }
    // 8.
    if rd::<u8>(this, 0x31) != 0 {
        rw_camera_set_near_clip_plane(scene_cam(), rd::<u32>(this, 0x138));
    }
    // 9.
    let mut d = x(f(0x20)).fsub_m(f(0x1C));
    while !d.fcom(x(k(K_PI))).test_any(0x01) {
        d = d.fsub_m(k(K_2PI));
    }
    while !d.fcom(x(k(K_MPI))).test_parity(0x05) {
        d = d.fadd_m(k(K_2PI));
    }
    if !d.fabs().fcom(x(k(K_SWITCH))).test_any(0x41) {
        wr::<u8>(this, 0x2A, 1);
    }
    // 10.
    let mut vehicle_target = false;
    if rd::<u32>(active_cam(this), 0x28) != 3 {
        let t: GPtr<u8> = rd(this, 0x958);
        if rd::<u8>(t, 0x36) & 7 == 2 {
            vehicle_target = true;
        }
    }
    // 11.
    let t = fild_u(now());
    if !t.fcom(x(rd::<f32>(this, 0xC98))).test_parity(0x41) {
        let num = t.fsub_m(rd(this, 0xC94));
        let den = x(rd(this, 0xC98)).fsub_m(rd(this, 0xC94));
        process_shake(this, num.fdiv(den).to_f32().to_bits());
    }
    // 12.
    if rd::<u8>(this, 0x58) != 0 && !vehicle_target {
        transition(this, b);
    } else {
        // 12a.
        let cam = active_cam(this);
        copy3(p(0x30), cam.byte_add(0x19C));
        copy3(p(0x24), cam.byte_add(0x1B4));
        if rd::<u8>(this, 0x2D) != 0 {
            for i in 0..3 {
                set(0x30 + i * 4, x(f(0x30 + i * 4)).fadd_m(rd(this, 0x8FC + i * 4)));
            }
            for i in 0..3 {
                set(0x10 + i * 4, x(rd(cam, 0x148 + i * 4)).fsub_m(f(0x30 + i * 4)));
            }
            copy3(p(0x3C), p(0x10));
            normalise(p(0x3C));
            cross_product(p(0x10), p(0x3C), p(0x24));
            normalise(p(0x10));
            cross_product(p(0x78), p(0x10), p(0x3C));
            copy3(p(0x24), p(0x78));
            normalise(p(0x24));
        } else {
            copy3(p(0x3C), cam.byte_add(0x190));
        }
        wr::<u8>(GPtr::from_addr(0xB70143), 0, 0);
        setf(0x58, rd(active_cam(this), 0xB4));
    }
    // 13.
    if rd::<u8>(this, 0x58) != 0
        && rd::<u8>(this, 0x2C) == 0
        && rd::<u8>(this, 0x2B) == 1
        && cam_stairs_for_player() == 0
        && rd::<u8>(this, 0x26) == 0
    {
        wr::<u32>(b, 0x20, 0);
        let pos = pos_of(rd(this, 0x958));
        if process_line_of_sight(pos, p(0x30), p(0x84), p(0x20), 1, 0, 0, 1, 0, 1, 1, 0) != 0 {
            copy3(p(0x30), p(0x84));
            rw_camera_set_near_clip_plane(scene_cam(), 0x3D4C_CCCD);
        }
    }
    // 14.
    let drunk = g::<f32>(0xC73C58);
    if !x(drunk.read()).fcom(x(k(K_0))).test_any(0x41) {
        let phase = g::<f32>(0xB6EC30);
        let kd = x(drunk.read()).fmul_m(k(K_DRUNK_SRC));
        let a = x(phase.read()).fmul_m(k(K_DEG2RAD));
        let c = a.fcos();
        setf(0x20, c.to_f32());
        set(0x30, c.fmul(kd).fadd_m(f(0x30)));
        let s = a.fsin();
        setf(0x1C, s.to_f32());
        set(0x38, s.fmul(kd).fadd_m(f(0x38)));
        normalise(p(0x24));
        let ku = x(drunk.read()).fmul_m(k(K_DRUNK_UP));
        set(0x24, x(f(0x20)).fmul(ku).fadd_m(f(0x24)));
        set(0x28, x(f(0x1C)).fmul(ku).fadd_m(f(0x28)));
        normalise(p(0x24));
        normalise(p(0x3C));
        let kf = x(drunk.read()).fmul_m(k(K_DRUNK_FRONT));
        set(0x3C, x(f(0x20)).fmul(kf).fadd_m(f(0x3C)));
        set(0x40, x(f(0x1C)).fmul(kf).fadd_m(f(0x40)));
        normalise(p(0x3C));
        cross_product(p(0x78), p(0x3C), p(0x24));
        copy3(p(0x5C), p(0x78));
        normalise(p(0x5C));
        cross_product(p(0x78), p(0x5C), p(0x3C));
        copy3(p(0x24), p(0x78));
        normalise(p(0x24));
        phase.write(x(phase.read()).fadd_m(k(K_DRUNK_STEP)).to_f32());
    }
    // 15.
    cross_product(p(0x78), p(0x24), p(0x3C));
    copy3(this.byte_add(0x974), p(0x78));
    copy3(this.byte_add(0x984), p(0x3C));
    copy3(this.byte_add(0x994), p(0x24));
    copy3(this.byte_add(0x9A4), p(0x30));
    // 16.
    let el = fild_u(now().wrapping_sub(rd::<u32>(this, 0x5C)));
    let force = el.fmul_m(k(K_SHAKE_DECAY)).fsubr_m(rd(this, 0x12C));
    setf(0x1C, force.to_f32());
    if force.fcom(x(k(K_2))).test_parity(0x05) {
        setf(0x1C, 2.0);
    } else if x(f(0x1C)).fcom(x(k(K_0))).test_any(0x41) {
        setf(0x1C, 0.0);
    }
    let force = f(0x1C);
    let r = (sa_core::orig::at_site(0x52C841, sa_core::crt::rand) as u32) & 0xFFFF;
    let kk = x(force).fmul_m(k(K_01));
    for (i, sh) in [0u32, 4, 8].into_iter().enumerate() {
        let n = ((r >> sh) & 0xF) as i32 - 7;
        let o = 0x9A4 + i as isize * 4;
        wr::<f32>(this, o, X87::from_i32(n).fmul(kk).fadd_m(rd(this, o)).to_f32());
    }
    if !kk.fcom(x(k(K_0))).test_any(0x41) && rd::<i32>(this, 0x90) != 1 {
        let v = 25 - ftol32(x(force).fmul_m(k(K_M255)));
        wr::<i32>(this, 0xA4, v.min(150));
    }
    // 17.
    let flag = GPtr::<u8>::from_addr(0xB70142);
    let mut blur = false;
    if rd::<u16>(active_cam(this), 0xC) == 0x10 && !find_player_vehicle(-1, 0).is_null() {
        let m: GPtr<u8> = rd(find_player_vehicle(-1, 0), 0x14);
        if !x(rd(m, 0x28)).fcom(x(k(K_02))).test_parity(0x05) {
            blur = true;
        }
    }
    if blur {
        wr::<u32>(this, 0x8C, 0xFF);
        wr::<u32>(this, 0x88, 0xFF);
        wr::<u32>(this, 0x84, 0xFF);
        wr::<u32>(this, 0x90, 1);
        wr::<u32>(this, 0xA0, 0xF0);
        flag.write(1);
    } else if flag.read() != 0 {
        flag.write(0);
    }
    // 18.
    draw_set_fov(rd::<u32>(b, 0x58));
    calculate_derived_values(this, 0, 1);
    copy_camera_matrix_to_rw_cam(this, 0);
    copy3(this.byte_add(0x908), this.byte_add(0x9A4));
    update_sound_distances(this);
    // 19.
    if g::<u8>(0xB5F851).read() != 0 && g::<u8>(0xB5F853).read() == 0 {
        wr::<f32>(this, 0xF0, 1.0);
    } else {
        wr::<f32>(this, 0xF0, x(k(K_70)).fdiv_m(g::<f32>(0x8D5038).read()).to_f32());
    }
    wr::<u32>(this, 0xF4, rd::<u32>(this, 0xF0));
    wr::<f32>(this, 0xF0, x(g::<f32>(0x8CD800).read()).fmul_m(rd(this, 0xF0)).to_f32());
    // 20.
    let sc = scene_cam();
    let far = X87::from_i32(ftol32(x(rd(sc, 0x84)).fmul_m(k(K_100)))).fmul_m(k(K_001));
    rw_camera_set_far_clip_plane(sc, far.to_f32().to_bits());
    let rw: GPtr<u8> = rd(this, 0x954);
    g::<f32>(0xC3EFA0).write(rd(rw, 0x80));
    let rw: GPtr<u8> = rd(this, 0x954);
    g::<f32>(0xC3EF9C).write(rd(rw, 0x84));
    // 21.
    if rd::<u8>(this, 0x29) == 1 || rd::<u8>(this, 0x2A) == 1 {
        copy3(this.byte_add(0x83C), this.byte_add(0x9A4));
        wr::<u8>(this, 0x29, 0);
    }
    let dx = x(rd(this, 0x9A4)).fsub_m(rd(this, 0x83C));
    let dy = x(rd(this, 0x9A8)).fsub_m(rd(this, 0x840));
    let dz = x(rd(this, 0x9AC)).fsub_m(rd(this, 0x844));
    let frames = rd::<i32>(this, 0x98).wrapping_add(1);
    let n = rd::<i32>(this, 0x94);
    wr::<i32>(this, 0x98, frames);
    let dist = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    let sum = dist.fadd_m(rd(this, 0x128));
    wr::<f32>(this, 0x128, sum.to_f32());
    if frames == n {
        wr::<u32>(this, 0x128, 0);
        wr::<u32>(this, 0x98, 0);
        wr::<f32>(this, 0x124, sum.fdiv(X87::from_i32(n)).to_f32());
    }
    copy3(this.byte_add(0x83C), this.byte_add(0x9A4));
    // 22.
    let o = x(rd(this, 0x150)).fadd_m(k(K_PI)).to_f32();
    let cam = active_cam(this);
    if rd::<u32>(cam, 0x28) != 3 && rd::<u16>(cam, 0xC) != 0x25 {
        copy3(cam.byte_add(0x19C), cam.byte_add(0x1A8));
        wr::<f32>(this, 0x150, o);
    }
    if rd::<u8>(this, 0x58) != 0 {
        let a = rd::<u8>(this, 0x59);
        let cj = cam_of(this, other_cam(a));
        let tj: GPtr<u8> = rd(cj, 0x21C);
        let te: GPtr<u8> = rd(this, 0x958);
        if !tj.is_null()
            && !te.is_null()
            && rd::<u8>(te, 0x36) & 7 == 3
            && rd::<u8>(tj, 0x36) & 7 != 2
            && rd::<u16>(cam_of(this, i32::from(a)), 0xC) != 0x25
            && rd::<u32>(cj, 0x28) != 3
        {
            copy3(cj.byte_add(0x19C), cam_of(this, i32::from(a) % 2).byte_add(0x1A8));
            wr::<f32>(this, 0x150, o);
        }
    }
    // 23.
    wr::<u8>(this, 0x1C, 0);
    wr::<u8>(this, 0x2D, 0);
    // 24.
    let m: GPtr<u8> = rd(this, 0x14);
    let (fx, fy, fz) = if m.is_null() {
        let h = rd::<f32>(this, 0x10);
        setf(0x18, 0.0);
        let fx = x(h).fsin().fchs();
        setf(0x14, x(h).fcos().to_f32());
        (fx, f(0x14), f(0x18))
    } else {
        copy3(p(0x10), m.byte_add(0x10));
        (x(f(0x10)), f(0x14), f(0x18))
    };
    let fx = fx.fmul_m(k(K_04));
    let fy = x(fy).fmul_m(k(K_04));
    setf(0x74, x(fz).fmul_m(k(K_04)).to_f32());
    let pos = pos_of(this);
    set(0x10, fx.fadd_m(rd(pos, 0)));
    set(0x14, fy.fadd_m(rd(pos, 4)));
    set(0x18, x(f(0x74)).fadd_m(rd(pos, 8)));
    let qz = f(0x18);
    let under = g::<f32>(0xC8132C);
    if get_water_level(rd(b, 0x10), rd(b, 0x14), rd(b, 0x18), p(0x68), 1, GPtr::NULL) == 0
        || !x(qz).fsub_m(k(K_06)).fcom(x(f(0x68))).test_any(0x41)
    {
        under.write(0.0);
        return;
    }
    let lvl = f(0x68);
    let depth = x(lvl).fsub_m(qz);
    let wd = g::<f32>(0xC81330);
    if x(k(K_0)).fcom(depth).test_any(0x41) {
        wd.write(depth.to_f32());
    } else {
        wd.write(0.0);
    }
    if !x(qz).fadd_m(k(K_06)).fcom(x(lvl)).test_parity(0x05) {
        under.write(1.0);
    } else {
        let v = x(lvl).fsub_m(k(K_06)).fsubr_m(qz).fmul_m(k(K_5_6)).fsubr_m(k(K_1));
        under.write(v.to_f32());
    }
}

/// Step 12b (0x52BA84-0x52C3D4): the interpolated source / target / up / FOV while
/// `m_bTransitionState` is set, then `CCam::KeepTrackOfTheSpeed`. `b` is the frame (see the module docs).
#[allow(clippy::too_many_lines)]
fn transition(this: GPtr<u8>, b: GPtr<u8>) {
    let f = |o: isize| rd::<f32>(b, o);
    let p = |o: isize| b.byte_add(o);
    let set = |o: isize, v: X87| wr::<f32>(b, o, v.to_f32());
    let setf = |o: isize, v: f32| wr::<f32>(b, o, v);
    let copy3 = |dst: GPtr<u8>, src: GPtr<u8>| {
        for i in 0..3 {
            wr::<u32>(dst, i * 4, rd::<u32>(src, i * 4));
        }
    };
    let tf = |o: isize| rd::<f32>(this, o);
    let zero = x(k(K_0));
    let now = g::<u32>(TIME_MS).read();

    let dur = rd::<u32>(this, 0x7C);
    let mut el = now.wrapping_sub(rd::<u32>(this, 0x78));
    if el >= dur {
        el = dur;
    }
    let elx = fild_u(el);
    setf(0x1C, elx.fdiv(fild_u(dur)).to_f32());
    let mut ft = elx.fdiv(fild_u(rd::<u32>(this, 0x80)));
    if !ft.fcom(zero).test_parity(0x05) {
        ft = zero;
    } else if !ft.fcom(x(k(K_1))).test_any(0x41) {
        ft = x(k(K_1));
    }
    // Target.
    let c18 = tf(0xC18);
    if !ft.fcom(x(c18)).test_parity(0x41) {
        let u = if x(c18).fcom(zero).test_parity(0x44) { ft.fsubr_m(c18).fdiv_m(c18) } else { zero };
        let w = ease(u);
        let wx = w.fmul_m(tf(0x8C0));
        setf(0x50, w.fmul_m(tf(0x8C4)).to_f32());
        setf(0x54, w.fmul_m(tf(0x8C8)).to_f32());
        set(0x10, wx.fadd_m(tf(0x89C)));
        set(0x14, x(f(0x50)).fadd_m(tf(0x8A0)));
        set(0x18, x(f(0x54)).fadd_m(tf(0x8A4)));
        copy3(p(0x4C), p(0x10));
        copy3(this.byte_add(0x8E4), p(0x10));
    } else if !ft.fcom(x(c18)).test_any(0x41) {
        let c1c = tf(0xC1C);
        let u = if x(c1c).fcom(zero).test_parity(0x44) { ft.fsub_m(c18).fdiv_m(c1c) } else { x(k(K_1)) };
        setf(0x20, ease(u).to_f32());
        if !x(tf(0xC18)).fcom(zero).test_parity(0x44) {
            copy3(this.byte_add(0x8E4), this.byte_add(0x89C));
        }
        let cam = active_cam(this);
        let dx = x(rd(cam, 0x148)).fsub_m(tf(0x8E4));
        let dy = x(rd(cam, 0x14C)).fsub_m(tf(0x8E8));
        set(0x18, x(rd(cam, 0x150)).fsub_m(tf(0x8EC)));
        let w = f(0x20);
        setf(0x4C, dx.fmul_m(w).to_f32());
        let py = dy.fmul_m(w);
        let pz = x(f(0x18)).fmul_m(w);
        set(0x10, x(f(0x4C)).fadd_m(tf(0x8E4)));
        set(0x14, py.fadd_m(tf(0x8E8)));
        set(0x18, pz.fadd_m(tf(0x8EC)));
        copy3(p(0x4C), p(0x10));
    }
    // Source / up / FOV.
    let fs = f(0x1C);
    let c10 = tf(0xC10);
    let looking_at_player = || rd::<u8>(this, 0x2B);
    if !x(fs).fcom(x(c10)).test_parity(0x41) {
        // fS <= C10 (0x52BCEE).
        let u = if x(c10).fcom(zero).test_parity(0x44) { x(c10).fsub_m(fs).fdiv_m(c10) } else { zero };
        let w = ease(u);
        setf(0x1C, w.to_f32());
        let wx = w.fmul_m(tf(0x8B4));
        let wy = x(f(0x1C)).fmul_m(tf(0x8B8));
        setf(0x74, x(f(0x1C)).fmul_m(tf(0x8BC)).to_f32());
        wr::<f32>(this, 0x8D8, wx.fadd_m(tf(0x890)).to_f32());
        wr::<f32>(this, 0x8DC, wy.fadd_m(tf(0x894)).to_f32());
        wr::<f32>(this, 0x8E0, x(f(0x74)).fadd_m(tf(0x898)).to_f32());
        if looking_at_player() != 0 {
            push_out(b, this.byte_add(0x8D8), false);
        }
        let w = f(0x1C);
        let ux = x(w).fmul_m(tf(0x8CC));
        let uy = x(w).fmul_m(tf(0x8D0));
        setf(0x74, x(w).fmul_m(tf(0x8D4)).to_f32());
        wr::<f32>(this, 0x8F0, ux.fadd_m(tf(0x8A8)).to_f32());
        wr::<f32>(this, 0x8F4, uy.fadd_m(tf(0x8AC)).to_f32());
        wr::<f32>(this, 0x8F8, x(f(0x74)).fadd_m(tf(0x8B0)).to_f32());
        wr::<f32>(this, 0x110, x(w).fmul_m(tf(0x114)).fadd_m(tf(0x170)).to_f32());
        copy3(p(0x30), this.byte_add(0x8D8));
        for i in 0..3 {
            set(0x3C + i * 4, x(f(0x4C + i * 4)).fsub_m(f(0x30 + i * 4)));
        }
        store_values_during_inter_pol(this, p(0x30), this.byte_add(0x8E4), this.byte_add(0x8F0), this.byte_add(0x110));
        normalise(p(0x3C));
        if looking_at_player() == 1 {
            setf(0x24, 0.0);
            setf(0x28, 0.0);
            setf(0x2C, 1.0);
        } else {
            copy3(p(0x24), this.byte_add(0x8F0));
        }
        normalise(p(0x24));
        orthonormalise(this, b);
        wr::<u32>(b, 0x58, rd::<u32>(this, 0x110));
    } else if !x(fs).fcom(x(c10)).test_any(0x41) && !x(fs).fcom(x(k(K_1))).test_parity(0x41) {
        // C10 < fS <= 1 (0x52C040).
        let c14 = tf(0xC14);
        let u = if x(c14).fcom(zero).test_parity(0x44) { x(fs).fsub_m(c10).fdiv_m(c14) } else { x(k(K_1)) };
        setf(0x1C, ease(u).to_f32());
        let w = rd::<u32>(b, 0x1C);
        let swi = this.byte_add(0x8D8);
        let cam = active_cam(this);
        vector_sub(p(0x6C), cam.byte_add(0x19C), swi);
        vector_scale(p(0x10), p(0x6C), w);
        vector_add(p(0x70), swi, p(0x10));
        copy3(p(0x30), p(0x70));
        if looking_at_player() != 0 {
            push_out(b, p(0x30), true);
        }
        let fovwi = tf(0x110);
        let cam = active_cam(this);
        setf(0x58, x(rd(cam, 0xB4)).fsub_m(fovwi).fmul_m(f(0x1C)).fadd_m(fovwi).to_f32());
        let uwi = this.byte_add(0x8F0);
        vector_sub(p(0x74), cam.byte_add(0x1B4), uwi);
        vector_scale(p(0x6C), p(0x74), w);
        vector_add(p(0x10), uwi, p(0x6C));
        let fxv = x(f(0x4C)).fsub_m(f(0x30));
        copy3(p(0x24), p(0x10));
        set(0x3C, fxv);
        set(0x40, x(f(0x50)).fsub_m(f(0x34)));
        set(0x44, x(f(0x54)).fsub_m(f(0x38)));
        store_values_during_inter_pol(this, p(0x30), p(0x4C), p(0x24), p(0x58));
        normalise(p(0x3C));
        if looking_at_player() == 1 {
            setf(0x24, 0.0);
            setf(0x28, 0.0);
            setf(0x2C, 1.0);
        }
        orthonormalise(this, b);
        wr::<u32>(b, 0x58, rd::<u32>(this, 0x110));
    }
    // 0x52C340.
    for i in 0..3 {
        set(0x10 + i * 4, x(f(0x30 + i * 4)).fsub_m(f(0x4C + i * 4)));
    }
    let h = x(f(0x14)).fmul_m(f(0x14)).fadd(x(f(0x10)).fmul_m(f(0x10))).fsqrt().to_f32();
    let alpha = X87::from_f64_exact(get_atan_of_xy(h.to_bits(), rd(b, 0x18))).to_f32();
    let beta = X87::from_f64_exact(get_atan_of_xy(rd(b, 0x10), rd(b, 0x14))).to_f32();
    setf(0x1C, alpha);
    setf(0x20, beta);
    keep_track_of_the_speed(active_cam(this), p(0x30), p(0x4C), p(0x24), p(0x1C), p(0x20), p(0x58));
}

/// The 1.3 push-out of a source `s` (x, y) from the target (`B+0x4C`) while interpolating: `dx`, `dy` as
/// f32 at `B+0x10` / `B+0x14`; when `sqrt(dx*dx + dy*dy) < 1.3`, `a = GetATanOfXY(dx, dy)` and
/// `s.x = cos(a) * 1.3 + tgt.x`, `s.y = sin(a) * 1.3 + tgt.y`. `dy_first`: the second branch forms
/// `dy*dy` first (the sum is the same).
fn push_out(b: GPtr<u8>, s: GPtr<u8>, dy_first: bool) {
    let f = |o: isize| rd::<f32>(b, o);
    wr::<f32>(b, 0x10, x(rd(s, 0)).fsub_m(f(0x4C)).to_f32());
    wr::<f32>(b, 0x14, x(rd(s, 4)).fsub_m(f(0x50)).to_f32());
    let xx = x(f(0x10)).fmul_m(f(0x10));
    let yy = x(f(0x14)).fmul_m(f(0x14));
    let sum = if dy_first { yy.fadd(xx) } else { xx.fadd(yy) };
    if sum.fsqrt().fcom(x(k(K_MIN_DIST))).test_parity(0x05) {
        return;
    }
    let a = X87::from_f64_exact(get_atan_of_xy(rd(b, 0x10), rd(b, 0x14)));
    wr::<f32>(s, 0, a.fcos().fmul_m(k(K_MIN_DIST)).fadd_m(f(0x4C)).to_f32());
    wr::<f32>(s, 4, a.fsin().fmul_m(k(K_MIN_DIST)).fadd_m(f(0x50)).to_f32());
}

/// Both interpolation branches' end (0x52BF32 / 0x52C258): modes 1 and 37 take
/// `up = Normalise(front x (-1, 0, 0))`, the others `r = Normalise(front x up)`, `up = Normalise(r x front)`.
fn orthonormalise(this: GPtr<u8>, b: GPtr<u8>) {
    let p = |o: isize| b.byte_add(o);
    let mode = rd::<u16>(active_cam(this), 0xC);
    normalise(p(0x3C));
    if mode == 1 || mode == 0x25 {
        wr::<f32>(b, 0x5C, -1.0);
        wr::<f32>(b, 0x60, 0.0);
        wr::<f32>(b, 0x64, 0.0);
        cross_product(p(0x78), p(0x3C), p(0x5C));
    } else {
        normalise(p(0x24));
        cross_product(p(0x6C), p(0x3C), p(0x24));
        for i in 0..3 {
            wr::<u32>(b, 0x5C + i * 4, rd::<u32>(b, 0x6C + i * 4));
        }
        normalise(p(0x5C));
        cross_product(p(0x78), p(0x5C), p(0x3C));
    }
    for i in 0..3 {
        wr::<u32>(b, 0x24 + i * 4, rd::<u32>(b, 0x78 + i * 4));
    }
    normalise(p(0x24));
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CAMERA_PROCESS];
