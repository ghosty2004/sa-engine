//! PORT-CAM-3: `CCamera::CamControl` (0x527FA0), the per-frame choice of the camera mode (3912
//! instructions). Ported from the 1.0 US bytes; `docs/re/specs/CAM/CCamera.md` (0x527FA0) as the reading
//! aid (gta-reversed has no body: `plugin::CallMethod`). Stack slots are named as in the spec: `Lxx` =
//! `[esp+0xXX]` of the final frame, i.e. entry ESP - 0xC4 + 0xXX.
//!
//! Every value the original copies with integer moves is carried as raw dwords; float arithmetic goes
//! through `sa_math::x87` in the original's operand order, with the f32 stores where the bytes store.
//! Results of callees returned in ST0 (`GetATanOfXY`, `GetHeading`, `CVector::Magnitude`) come through an
//! f64 (the documented approximation of the earlier CAM passes).

use sa_core::abi::{Fp, HookReg, entry_residue, stack_footprint};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_world::world::common::{find_player_ped, find_player_vehicle};
use sa_world::world::cworld::{PLAYER_IN_FOCUS, PLAYER_INFO_SIZE, PLAYERS};

use crate::camctl::{clean_up_old_reference, register_reference};
use crate::common::{ACTIVE_CAM, CAMS, CCAM_SIZE, g, k, rd, wr, x, xi};

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn cc_get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad` 0x5404A0 (cycle-camera "just down", AL)
    pub fn pad_cycle_camera_down(pad: GPtr<u8>) -> u8 = thiscall 0x5404A0 => unported;
    /// `CPad::sub_5404F0` (AL)
    pub fn pad_5404f0(pad: GPtr<u8>) -> u8 = thiscall 0x5404F0 => unported;
    /// `CPad::sub_540530` (AL)
    pub fn pad_540530(pad: GPtr<u8>) -> u8 = thiscall 0x540530 => unported;
    /// `CPad` 0x540E80 (AX)
    pub fn pad_540e80(pad: GPtr<u8>) -> i16 = thiscall 0x540E80 => unported;
    /// `CPad` 0x540F80 (result unused)
    pub fn pad_540f80(pad: GPtr<u8>) -> u8 = thiscall 0x540F80 => unported;
    /// `CPad::GetPedWalkLeftRight()` (0x53FC90, AX)
    pub fn cc_pad_walk_lr(pad: GPtr<u8>) -> i16 = thiscall 0x53FC90 => unported;
    /// `CPad::GetPedWalkUpDown()` (0x53FD30, AX)
    pub fn cc_pad_walk_ud(pad: GPtr<u8>) -> i16 = thiscall 0x53FD30 => unported;
    /// `CPad::GetTarget()` (0x540670, AL)
    pub fn pad_get_target(pad: GPtr<u8>) -> u8 = thiscall 0x540670 => unported;
    /// `CPad::GetEnterTargeting()` (0x5406B0, AL)
    pub fn cc_pad_enter_targeting(pad: GPtr<u8>) -> u8 = thiscall 0x5406B0 => unported;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390, AL)
    pub fn cc_is_coop() -> u8 = cdecl 0x441390 => unported;
    /// `CCullZones::CamCloseInForPlayer()` (0x72DD20, AL)
    pub fn cam_close_in_for_player() -> u8 = cdecl 0x72DD20 => unported;
    /// `CCullZones::CamStairsForPlayer()` (0x72DD30, AL)
    pub fn cam_stairs_for_player() -> u8 = cdecl 0x72DD30 => unported;
    /// `CCullZones::FindZoneWithStairsAttributeForPlayer()` (0x72DAD0)
    pub fn find_zone_with_stairs_for_player() -> GPtr<u8> = cdecl 0x72DAD0 => unported;
    /// the cull zone's centre (0x72D6D0, thiscall, `ret 4`, returns `out`)
    pub fn zone_centre(zone: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x72D6D0 => crate::camproc::cull_zone_centre;
    /// `CCamera::Restore()` (0x50B930)
    pub fn cc_restore(this: GPtr<u8>) = thiscall 0x50B930 => crate::camctl::restore;
    /// `CCamera::RestoreWithJumpCut()` (0x50BAB0)
    pub fn cc_restore_with_jump_cut(this: GPtr<u8>) = thiscall 0x50BAB0 => crate::camctl::restore_with_jump_cut;
    /// `CCamera::SetCameraDirectlyBehindForFollowPed_CamOnAString()` (0x50BD40)
    pub fn cc_set_behind(this: GPtr<u8>) = thiscall 0x50BD40 => crate::camctl::set_camera_directly_behind_for_follow_ped_cam_on_a_string;
    /// `CCamera::SetCamPositionForFixedMode(const CVector&, const CVector&)` (0x50BEC0)
    pub fn cc_set_fixed(this: GPtr<u8>, src: GPtr<u8>, up: GPtr<u8>) = thiscall 0x50BEC0 => unported;
    /// `CCamera::ClearPlayerWeaponMode()` (0x50AB10)
    pub fn cc_clear_weapon_mode(this: GPtr<u8>) = thiscall 0x50AB10 => crate::camera::clear_player_weapon_mode;
    /// `CCamera::GetArrPosForVehicleType(int32, int32&)` (0x50AF00, AL)
    pub fn cc_get_arr_pos(this: GPtr<u8>, kind: i32, out: GPtr<u8>) -> u8 = thiscall 0x50AF00 => unported;
    /// `CCamera::StartTransition(int16)` (0x515200; the original pushes the full register)
    pub fn cc_start_transition(this: GPtr<u8>, mode: u32) = thiscall 0x515200 => unported;
    /// `CCamera::ProcessObbeCinemaCamera{Car,Train,Heli,Plane,Boat}()`
    pub fn obbe_car(this: GPtr<u8>) = thiscall 0x5267C0 => crate::obbe::obbe_car;
    pub fn obbe_train(this: GPtr<u8>) = thiscall 0x526950 => crate::obbe::obbe_train;
    pub fn obbe_heli(this: GPtr<u8>) = thiscall 0x526AE0 => crate::obbe::obbe_heli;
    pub fn obbe_plane(this: GPtr<u8>) = thiscall 0x526C80 => crate::obbe::obbe_plane;
    pub fn obbe_boat(this: GPtr<u8>) = thiscall 0x526E20 => crate::obbe::obbe_boat;
    /// `CGarages::IsPointInAGarageCameraZone(CVector)` (0x449BA0, the vector by value; AL)
    pub fn is_point_in_garage_camera_zone(px: u32, py: u32, pz: u32) -> u8 = cdecl 0x449BA0 => unported;
    /// `CGarages::CameraShouldBeOutside()` (0x448650, AL)
    pub fn camera_should_be_outside() -> u8 = cdecl 0x448650 => unported;
    /// `CGarage::FindDoorsWithGarage(CObject**, CObject**)` (0x449FF0)
    pub fn find_doors_with_garage(gar: GPtr<u8>, first: GPtr<u8>, second: GPtr<u8>) = thiscall 0x449FF0 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    pub fn cc_get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `CPlaceable::GetHeading()` (0x441DB0, ST0)
    pub fn cc_get_heading(e: GPtr<u8>) -> f64 = thiscall 0x441DB0 => unported;
    /// `CGeneral::GetATanOfXY(float, float)` (0x53CC70, ST0)
    pub fn cc_atan_xy(xb: u32, yb: u32) -> f64 = cdecl 0x53CC70 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` = a - b (0x40FE60)
    pub fn cc_vec_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `out = a + b` (0x40FE30)
    pub fn cc_vec_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `out = s * v` (0x40FE90)
    pub fn cc_vec_s_mul(out: GPtr<u8>, s: u32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `out = v * s` (0x40FEC0)
    pub fn cc_vec_mul_s(out: GPtr<u8>, v: GPtr<u8>, s: u32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    pub fn cc_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    pub fn cc_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CWorld::GetIsLineOfSightClear(...)` (0x56A490, AL)
    pub fn cc_los(a: GPtr<u8>, b: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, ig: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CWorld::ProcessLineOfSight(...)` (0x56BA00, AL)
    pub fn cc_process_los(a: GPtr<u8>, b: GPtr<u8>, cp: GPtr<u8>, ent: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, ic: u32, sh: u32) -> u8 = cdecl 0x56BA00 => unported;
    /// `CWorld::FindGroundZFor3DCoord(float, float, float, bool*, CEntity**)` (0x5696C0, ST0)
    pub fn cc_find_ground_z(px: u32, py: u32, pz: u32, found: GPtr<u8>, ent: u32) -> f64 = cdecl 0x5696C0 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    pub fn cc_vehicle_appearance(veh: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CCam::WellBufferMe(float, float*, float*, float, float, bool)` (0x509AE0)
    pub fn cc_well_buffer_me(target: u32, v: GPtr<u8>, s: GPtr<u8>, top: u32, step: u32, angle: u32) = cdecl 0x509AE0 => unported;
    /// `CCam::GetWeaponFirstPersonOn()` (0x509DC0, AL)
    pub fn cc_weapon_first_person_on(cam: GPtr<u8>) -> u8 = thiscall 0x509DC0 => unported;
    /// `CCam::Using3rdPersonMouseCam()` (0x50A850, AL)
    pub fn cc_using_3rd_person_mouse_cam(cam: GPtr<u8>) -> u8 = thiscall 0x50A850 => unported;
    /// `CTaskManager::GetTaskSecondary(int32)` (0x681810)
    pub fn cc_task_secondary(tm: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x681810 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    pub fn cc_find_active_task_by_type(tm: GPtr<u8>, t: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CPedIntelligence::GetTaskJetPack()` (0x601110)
    pub fn cc_task_jet_pack(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `CPedIntelligence::GetTaskHold(bool)` (0x600FF0)
    pub fn cc_task_hold(intel: GPtr<u8>, ignore: u8) -> GPtr<u8> = thiscall 0x600FF0 => unported;
    /// `CPed::IsPedInControl()` (0x5E3960, AL)
    pub fn cc_is_ped_in_control(ped: GPtr<u8>) -> u8 = thiscall 0x5E3960 => unported;
    /// `CPed::CanWeRunAndFireWithWeapon()` (0x5E88E0, AL)
    pub fn cc_can_run_and_fire(ped: GPtr<u8>) -> u8 = thiscall 0x5E88E0 => unported;
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float, float)` (0x506EA0)
    pub fn cc_report_frontend_audio(ae: GPtr<u8>, id: i32, vol: u32, speed: u32) = thiscall 0x506EA0 => unported;
}

/// The static int16 mode request (`REQ`, persists across frames).
const REQ: usize = 0xB70140;
/// int32 forced car mode (consumed when > 0)
const FORCED_CAR_MODE: usize = 0x8CC824;
const BYTE_B6EC34: usize = 0xB6EC34;
const ARREST_LATCH: usize = 0xB7013C;
const PREV_PED_STATE: usize = 0xB70138;
const ARREST_MODE: usize = 0xB70134;
const SYPHON_LATCH: usize = 0xB7013D;
const USER_PAUSE: usize = 0xB7CB49;
const CODE_PAUSE: usize = 0xB7CB48;
const TIME_STEP: usize = 0xB7CB5C;
const TIME_MS: usize = 0xB7CB84;
const REPLAY_MODE: usize = 0xA43088;
const MODEL_VIEWER: usize = 0xBA6728;
const USE_MOUSE_3RD: usize = 0xB6EC2E;
const DID_CINEMA: usize = 0xB6EC2D;
const SPEED_FX_USER: usize = 0x8D5109;
const PLAYER_PED_VISIBLE: usize = 0x8CC380;
const PED_ZOOM_SMOOTHED_GLOBAL: usize = 0xB6F0FC;
const AUDIO_ENGINE: usize = 0xB6BC90;
const PLAYER1_PED: usize = 0xB7CF28;

/// `.data` / `.rdata` constants (bits).
const K_012: u32 = 0x3DF5_C28F; // 0.12 [0x85A6F0]
const K_ZERO: u32 = 0; // [0x858B50]
const K_HALF: u32 = 0x3F00_0000; // 0.5 [0x858B8C]
const K_ONE: u32 = 0x3F80_0000; // 1.0 [0x858624]
const K_2P32: u32 = 0x4F80_0000; // 2^32 [0x858C54]
const K_PI: u32 = 0x4049_0FDB; // [0x858CB8]
const K_2PI: u32 = 0x40C9_0FDB; // [0x858CBC]
const K_HALF_PI: u32 = 0x3FC9_0FDB; // [0x858FE4]

fn kd(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// `e->m_matrix ? &m_matrix->pos : &placement.pos`
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

fn etype(e: GPtr<u8>) -> u8 {
    rd::<u8>(e, 0x36) & 7
}

/// `m_aCams[m_nActiveCam]` (index re-read at every use).
fn cam(this: GPtr<u8>) -> GPtr<u8> {
    let i = isize::from(rd::<u8>(this, ACTIVE_CAM));
    this.byte_add(CAMS + i * CCAM_SIZE)
}

/// `m_aCams[(m_nActiveCam + 1) signed-mod 2]`
fn other_cam(this: GPtr<u8>) -> GPtr<u8> {
    let i = (i32::from(rd::<u8>(this, ACTIVE_CAM)) + 1) % 2;
    this.byte_add(CAMS + i as isize * CCAM_SIZE)
}

fn cam_mode(this: GPtr<u8>) -> u16 {
    rd::<u16>(cam(this), 0xC)
}

fn req() -> u16 {
    g::<u16>(REQ).read()
}

fn set_req(v: u16) {
    g::<u16>(REQ).write(v)
}

fn pad() -> GPtr<u8> {
    cc_get_pad(0)
}

fn ped0() -> GPtr<u8> {
    find_player_ped(-1)
}

/// `CWorld::Players[CWorld::PlayerInFocus].m_pPed`
fn focus_ped() -> GPtr<u8> {
    let focus = usize::from(PLAYER_IN_FOCUS.get());
    g::<GPtr<u8>>(PLAYERS + focus * PLAYER_INFO_SIZE).read()
}

fn tgt(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0x958)
}

fn vbuf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b.as_mut_ptr()).cast()
}

fn copy3(dst: GPtr<u8>, src: GPtr<u8>) {
    for i in 0..3 {
        wr::<u32>(dst, 4 * i, rd::<u32>(src, 4 * i));
    }
}

/// `test ah, m; jp` after `fcom(p) a, b`: taken.
fn tjp(a: X87, b: X87, m: u8) -> bool {
    a.fcom(b).test_parity(m)
}

/// `test ah, m; jne` after `fcom(p) a, b`: taken.
fn tjne(a: X87, b: X87, m: u8) -> bool {
    a.fcom(b).test_any(m)
}

/// `test ah, m; jnp`: taken.
fn tjnp(a: X87, b: X87, m: u8) -> bool {
    !a.fcom(b).test_parity(m)
}

fn st0(v: f64) -> X87 {
    X87::from_f64_exact(v)
}

/// `(float)(uint32)v`: `fild dword` plus 2^32 when the sign bit is set.
fn fild_u32(v: u32) -> X87 {
    let f = xi(v as i32);
    if (v as i32) < 0 { f.fadd_m(k(K_2P32)) } else { f }
}

/// The zoom smoothing toward `v` (2e / 3c, script and base cases): `s < v`: `n = step + s`, `n > v`
/// gives `v`; else `n = s - step`, `n < v` gives `v` (NaN keeps `n`). `vr` re-reads the target.
fn smooth_toward(s_p: GPtr<u8>, vr: impl Fn() -> f32) {
    let s = x(rd::<f32>(s_p, 0));
    let lt = !tjp(s, x(vr()), 0x05);
    let step = x(kd(TIME_STEP)).fmul_m(k(K_012));
    let r = if lt {
        let n = step.fadd_m(rd::<f32>(s_p, 0));
        if tjne(n, x(vr()), 0x41) { n } else { x(vr()) }
    } else {
        let n = step.fsubr_m(rd::<f32>(s_p, 0));
        if tjp(n, x(vr()), 0x05) { n } else { x(vr()) }
    };
    wr::<f32>(s_p, 0, r.to_f32());
}

/// The smoothing toward a fixed bound `b` (the cull-zone case, the bound loaded before the compare):
/// `s < b`: `n = step + s`, `b >= n` (or NaN) keeps `n`; else `n = s - step`, `b <= n` (or NaN) keeps it.
fn smooth_toward_const(s_p: GPtr<u8>, b: f32) {
    let s = x(rd::<f32>(s_p, 0));
    let lt = !tjp(s, x(b), 0x05);
    let step = x(kd(TIME_STEP)).fmul_m(k(K_012));
    let r = if lt {
        let n = step.fadd_m(rd::<f32>(s_p, 0));
        if tjp(x(b), n, 0x05) { n } else { x(b) }
    } else {
        let n = step.fsubr_m(rd::<f32>(s_p, 0));
        if tjne(x(b), n, 0x41) { n } else { x(b) }
    };
    wr::<f32>(s_p, 0, r.to_f32());
}

/// `test ah, 0x44; jp` not taken: equal (also -0 vs 0), not NaN.
fn eq0(v: f32) -> bool {
    !tjp(x(v), x(k(K_ZERO)), 0x44)
}

/// The stairs zone's extent `n` (2d / 3d): with the zone's int16 words a = +4, b = +6, c = +8,
/// d = +0xA: `|a| + |c| > |b| + |d| ? |a| + |c| : |b| + |d|` (re-reading the words as the bytes).
fn zone_extent(z: GPtr<u8>) -> i32 {
    let w = |o: isize| i32::from(rd::<i16>(z, o));
    let ac = w(4).abs() + w(8).abs();
    let bd = w(6).abs() + w(0xA).abs();
    if ac > bd {
        w(4).abs() + w(8).abs()
    } else {
        w(6).abs() + w(0xA).abs()
    }
}

/// LOS9(a, b) = `GetIsLineOfSightClear(a, b, true, false, false, false, false, false, true)`.
fn los9(a: GPtr<u8>, b: GPtr<u8>) -> bool {
    cc_los(a, b, 1, 0, 0, 0, 0, 0, 1) != 0
}

/// The stairs push-out (2d / 3d): `k = (float)n + (float)n`; `W = pos(T) + k*dir`; LOS9(pos(T), W) ->
/// `out = W`; else `W = pos(T) - k*dir`, LOS9 -> `out = W`. `tmp64` is L64, `tmpw` the W slot.
fn stairs_push_out(
    this: GPtr<u8>,
    zone: GPtr<u8>,
    dir: GPtr<u8>,
    out: GPtr<u8>,
    tmp64: GPtr<u8>,
    w: GPtr<u8>,
) {
    let n = zone_extent(zone);
    let kk = xi(n).fadd(xi(n)).to_f32().to_bits();
    let t = tgt(this);
    cc_vec_s_mul(tmp64, kk, dir);
    cc_vec_add(w, pos(t), tmp64);
    if los9(pos(t), w) {
        copy3(out, w);
        return;
    }
    let t = tgt(this);
    let p = pos(t);
    cc_vec_s_mul(tmp64, kk, dir);
    let mut l88 = [0u32; 3];
    cc_vec_sub(vbuf(&mut l88), p, tmp64);
    copy3(w, vbuf(&mut l88));
    if los9(pos(t), w) {
        copy3(out, w);
    }
}

/// The door-based base point (2d / 3d doors): both doors `a + (b - a)*0.5f` (x: product stored f32
/// then `+ a.x`; y in the register; z: difference stored f32, then `*0.5 + a.z`).
fn door_mid(a: [u32; 3], b: [u32; 3]) -> [u32; 3] {
    let f = f32::from_bits;
    let dx = x(f(b[0])).fsub_m(f(a[0]));
    let dy = x(f(b[1])).fsub_m(f(a[1]));
    let dz = x(f(b[2])).fsub_m(f(a[2])).to_f32();
    let hx = dx.fmul_m(k(K_HALF)).to_f32();
    let hy = dy.fmul_m(k(K_HALF));
    let hz = x(dz).fmul_m(k(K_HALF));
    let rx = x(hx).fadd_m(f(a[0])).to_f32();
    let ry = hy.fadd_m(f(a[1])).to_f32();
    let rz = hz.fadd_m(f(a[2])).to_f32();
    [rx.to_bits(), ry.to_bits(), rz.to_bits()]
}

fn read3(p: GPtr<u8>) -> [u32; 3] {
    [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)]
}

/// The fixed camera from the garage doors (2d 0x528800 / 3d 0x52990C): `first` / `second` door
/// objects; `l88` = pos(T) as captured; `l50` the base used for the no-door direction; `back` the
/// `.data` back-off, `up` the `.data` height. `right` starts as given (L24 / L14).
#[allow(clippy::too_many_arguments)]
fn doors_fixed_cam(
    this: GPtr<u8>,
    first: GPtr<u8>,
    second: GPtr<u8>,
    l88: [u32; 3],
    l50: [u32; 3],
    back: usize,
    up: usize,
) {
    let mut a = [0u32; 3];
    let mut b = [0u32; 3];
    let mut right = [0u32; 3];
    if !first.is_null() {
        let d = rd::<GPtr<u8>>(first, 0x170);
        a = read3(pos(d));
        right = read3(cc_get_matrix(d));
    }
    if !second.is_null() {
        let d = rd::<GPtr<u8>>(second, 0x170);
        b = read3(pos(d));
        right = read3(cc_get_matrix(d));
    }
    let base = if !first.is_null() {
        if !second.is_null() { door_mid(a, b) } else { a }
    } else if !second.is_null() {
        b
    } else {
        let f = f32::from_bits;
        right[0] = x(f(l88[0])).fsub_m(f(l50[0])).to_f32().to_bits();
        right[1] = x(f(l88[1])).fsub_m(f(l50[1])).to_f32().to_bits();
        right[2] = 0;
        cc_normalise(vbuf(&mut right));
        l88
    };
    let mut base = base;
    let mut off = [0u32; 3];
    let mut c = [0u32; 3];
    cc_vec_mul_s(vbuf(&mut off), vbuf(&mut right), g::<u32>(back).read());
    cc_vec_add(vbuf(&mut c), vbuf(&mut base), vbuf(&mut off));
    c[2] = x(f32::from_bits(c[2])).fadd_m(kd(up)).to_f32().to_bits();
    let mut zero = [0u32; 3];
    cc_set_fixed(this, vbuf(&mut c), vbuf(&mut zero));
    wr::<u8>(this, 0x4E, 1);
}

/// The jump-cut field copies shared by 4e, 6j and 7 (fixed-mode vector, target reference, fixed
/// source, up offset, last-above-water position from the other cam).
fn jump_cut_copies(this: GPtr<u8>, looking_at_vector: Option<u8>) {
    copy3(cam(this).byte_add(0x154), this.byte_add(0x860));
    let slot = cam(this).byte_add(0x21C);
    let old = rd::<GPtr<u8>>(slot, 0);
    if !old.is_null() {
        clean_up_old_reference(old, slot);
    }
    wr::<GPtr<u8>>(cam(this), 0x21C, tgt(this));
    let slot = cam(this).byte_add(0x21C);
    register_reference(rd::<GPtr<u8>>(slot, 0), slot);
    copy3(cam(this).byte_add(0x160), this.byte_add(0x86C));
    copy3(cam(this).byte_add(0x16C), this.byte_add(0x878));
    wr::<u8>(cam(this), 2, looking_at_vector.unwrap_or(0));
    let dst = cam(this).byte_add(0x178);
    copy3(dst, other_cam(this).byte_add(0x178));
}

/// The heading test of 6b / 6f: `a` (already `- pi/2`) wrapped against `h = cam.m_fHorizontalAngle`
/// (+0xBC), then `30deg [.data 0x8CC46C] * 0.017453292 < |a - h|`.
fn heading_off(a: X87, h_cam: GPtr<u8>) -> bool {
    let h = || rd::<f32>(h_cam, 0xBC);
    let hp = x(h()).fadd_m(k(K_PI));
    let a = if !tjne(a, hp, 0x41) {
        a.fsub_m(k(K_2PI))
    } else {
        let hm = x(h()).fsub_m(k(K_PI));
        if tjp(a, hm, 0x05) { a } else { a.fadd_m(k(K_2PI)) }
    };
    let d = a.fsub_m(h()).fabs();
    let thr = x(kd(0x8CC46C)).fmul_m(kd(0x8595EC));
    tjnp(thr, d, 0x05)
}

/// `CCamera::CamControl()` (0x527FA0): see the module docs and the spec's blocks 1-8.
///
/// Stack (`footprint`, ADR-001 §5b): on the vehicle stairs path without a garage the door pointers L74 /
/// L78 ([S-0x50] / [S-0x4C], S = entry ESP) are read without being written in this function (spec open
/// question 1): the port reads the caller's residue there (`entry_residue`), as the original does.
/// Footprint: the pushed EBX / EBP / ESI / EDI ([S-0xB8] .. [S-0xC4]) and the initial mode L90 ([S-0x34]).
/// Not reproduced: the other locals and the callees' frames (the window keeps the pre-call bytes).
#[hook(
    addr = 0x527FA0,
    conv = thiscall,
    name = "CCamera::CamControl",
    expect = "81 EC B4 00 00 00 53 55 56",
    this = "CCamera",
    footprint = 0x1000
)]
pub fn cam_control(this: GPtr<u8>) {
    let l90 = i32::from(cam_mode(this) as i16);
    stack_footprint(&[
        (-0xB8, Fp::Ebx),
        (-0xBC, Fp::Ebp),
        (-0xC0, Fp::Esi),
        (-0xC4, Fp::Edi),
        (-0x34, Fp::Val(l90 as u32)),
    ]);
    cam_control_in(this, l90);
}

/// [`cam_control`] without the stack bookkeeping.
pub fn cam_control_in(this: GPtr<u8>, l90: i32) {
    // 1. prologue
    for o in [0x2E, 0x2F, 0x37, 0x36, 0x28, 0x35, 0x22, 0x20] {
        wr::<u8>(this, o, 0);
    }
    let mut l10 = false; // switch now
    let mut l11 = false; // on stairs
    g::<u8>(BYTE_B6EC34).write(0);
    if rd::<GPtr<u8>>(cam(this), 0x21C).is_null() && tgt(this).is_null() {
        let slot = this.byte_add(0x958);
        wr::<GPtr<u8>>(this, 0x958, focus_ped());
        register_reference(rd::<GPtr<u8>>(slot, 0), slot);
    }
    let n = rd::<i32>(this, 0xAC).wrapping_add(1);
    let lim = rd::<i32>(this, 0xA8);
    wr::<i32>(this, 0xAC, n);
    if n > lim {
        wr::<i32>(this, 0xAC, 1);
    }
    wr::<u8>(this, 0x1E, u8::from(rd::<i32>(this, 0xAC) == lim));
    if rd::<u8>(this, 0x1E) == 1 {
        let r = cam_close_in_for_player();
        wr::<u8>(this, 0x54, u8::from(r != 0));
    }
    if rd::<u8>(this, 0x2B) == 1 {
        let p = pad();
        wr::<u8>(p, 0x10E, rd::<u8>(p, 0x10E) & 0xFE);
        let q = ped0();
        wr::<u32>(q, 0x1C, rd::<u32>(q, 0x1C) | 0x80);
    }
    let mut l12 = false;
    let mut di: u16;
    let paused =
        g::<u8>(USER_PAUSE).read() != 0 || g::<u8>(CODE_PAUSE).read() != 0 || rd::<u8>(this, 0x21) != 0;
    'common: {
        if paused {
            di = req();
            break 'common;
        }
        let mut l20: u32 = 0;
        let mut l3c: u32 = 0;
        if rd::<u8>(this, 0x34) == 1 {
            let t = tgt(this);
            if !(etype(t) == 2 && rd::<i32>(t, 0x590) == 6) {
                cc_restore(this);
                let w = rd::<u8>(this, 0x3D) == 1;
                wr::<u8>(this, 0x35, 1);
                wr::<u8>(this, 0x34, 0);
                wr::<u8>(this, 0x3C, u8::from(w));
            }
        }
        let t = tgt(this);
        let ty = etype(t);
        if ty == 2 {
            di = vehicle_target(this, &mut l11, &mut l20);
            break 'common;
        }
        if ty != 3 {
            di = req();
            break 'common;
        }
        match ped_target(this, &mut l11, &mut l3c, &mut l12) {
            Some(d) => di = d,
            None => di = req(),
        }
    }
    // 4b. co-op
    if rd::<u8>(this, 0x43) != 0 {
        let p0 = g::<GPtr<u8>>(PLAYERS).read();
        if !p0.is_null() {
            let p1 = g::<GPtr<u8>>(PLAYER1_PED).read();
            if !p1.is_null() {
                let in0 = rd::<u32>(p0, 0x46C) & 0x100 != 0;
                let v0 = rd::<GPtr<u8>>(p0, 0x58C);
                let ok = in0 && rd::<u32>(p1, 0x46C) & 0x100 != 0 && !v0.is_null() && {
                    let v1 = rd::<GPtr<u8>>(p1, 0x58C);
                    !v1.is_null()
                };
                if ok {
                    let v1 = rd::<GPtr<u8>>(p1, 0x58C);
                    wr::<GPtr<u8>>(this, 0x958, v0);
                    di = if v0 == v1 {
                        if rd::<u8>(this, 0x44) != 0 {
                            rd::<u16>(this, 0x48)
                        } else {
                            rd::<u16>(this, 0x4A)
                        }
                    } else {
                        rd::<u16>(this, 0x46)
                    };
                } else {
                    di = rd::<u16>(this, 0x4C);
                }
                set_req(di);
            }
        }
    }
    // 4c. arrest latch
    let st = rd::<i32>(focus_ped(), 0x530);
    let mut l13 = false;
    if st == 0x3F {
        g::<u8>(ARREST_LATCH).write(1);
    } else if g::<u8>(ARREST_LATCH).read() != 0 {
        l13 = true;
        g::<u8>(ARREST_LATCH).write(0);
    }
    let go = if g::<i32>(PREV_PED_STATE).read() != 0x3F && st == 0x3F {
        !(rd::<i32>(this, 0xB4) == 0 && etype(tgt(this)) == 2)
    } else {
        false
    };
    g::<i32>(PREV_PED_STATE).write(st);
    if go {
        di = 0x20;
        g::<i32>(ARREST_MODE).write(0x20);
        wr::<u8>(cam(this), 0xA, 1);
        set_req(di);
    } else if st == 0x3F {
        di = g::<u16>(ARREST_MODE).read();
        set_req(di);
    }
    // 4d. dead player
    if rd::<i32>(focus_ped(), 0x530) == 0x37 {
        wr::<u8>(this, 0x2F, 0);
        let cm = cam_mode(this);
        if cm == 0x1D {
            di = 0x1D;
        } else if cm == 0x20 {
            di = cm;
        } else {
            di = dead_player_mode(this);
        }
        set_req(di);
    }
    // 4e. restore by jump cut
    if rd::<u8>(this, 0x30) == 1 {
        if ![4, 0x22, 7, 8, 0x33, 0x2E, 0xB, 0x1C, 0x23, 0x12, 3].contains(&di)
            && g::<u8>(USE_MOUSE_3RD).read() == 0
        {
            cc_set_behind(this);
        }
        let m = rd::<u16>(this, 0xC38);
        wr::<u16>(cam(this), 0xC, m);
        set_req(m);
        wr::<u8>(this, 0x2A, 1);
        wr::<u8>(cam(this), 0xA, 1);
        jump_cut_copies(this, None);
        wr::<u8>(this, 0x30, 0);
        wr::<u8>(cam(this), 0xA, 1);
        wr::<u32>(this, 0xC0, rd::<u32>(this, 0xB8));
        wr::<u32>(this, 0xD4, rd::<u32>(this, 0xCC));
        wr::<u8>(this, 0x58, 0);
        wr::<u8>(this, 0x4F, 0);
    }
    // 4f.
    if g::<u8>(MODEL_VIEWER).read() != 0 {
        set_req(9);
    }
    // 5. cinema cameras
    l12 = true;
    let t = tgt(this);
    if !t.is_null() {
        if etype(t) == 2 {
            if rd::<i32>(this, 0xB4) == 5 {
                wr::<u8>(this, 0x2F, 1);
            }
        } else if rd::<i32>(this, 0xC8) == 5 {
            wr::<u8>(this, 0x2E, 1);
        }
    }
    if !find_player_vehicle(-1, 0).is_null() && rd::<i32>(find_player_vehicle(-1, 0), 0x590) == 6 {
        wr::<u8>(this, 0x2F, 1);
    }
    let mut dx: u16 = 'dx: {
        let t = tgt(this);
        if !t.is_null() && etype(t) == 2 {
            let p = ped0();
            if !p.is_null() {
                let s = rd::<i32>(p, 0x530);
                if s == 0x3F || s == 0x37 {
                    wr::<u8>(this, 0x2E, 0);
                    let s = rd::<i32>(p, 0x530);
                    l12 = false;
                    if s == 0x3F {
                        set_req(0x20);
                        break 'dx 0x20;
                    }
                    if s == 0x37 {
                        set_req(0x1D);
                        break 'dx 0x1D;
                    }
                }
            }
        }
        req()
    };
    const NO_CINEMA: [u16; 20] = [
        0x1D, 0x17, 0x1C, 0xB, 7, 0x23, 8, 0x33, 0x20, 0x21, 0x22, 0x24, 0x27, 0x28, 0x34, 0x2A, 0x2B, 0x29,
        0x2D, 0x2E,
    ];
    if rd::<u8>(this, 0x34) == 1
        || NO_CINEMA.contains(&dx)
        || rd::<i32>(this, 0xB0) == 1
        || rd::<u8>(this, 0x28) != 0
        || rd::<u8>(this, 0x26) != 0
        || cam_mode(this) == 0x1D
    {
        l12 = false;
    }
    if rd::<u8>(this, 0xCF0) != 0 {
        wr::<u8>(this, 0x2F, 1);
        l12 = true;
    }
    if rd::<u8>(this, 0x2E) != 0 && l12 {
        // straight to block 6
    } else if rd::<u8>(this, 0x2F) != 0 && l12 {
        g::<u8>(SPEED_FX_USER).write(0);
        let t = tgt(this);
        if etype(t) == 2 {
            let sub = rd::<i32>(t, 0x594);
            if sub == 4 {
                obbe_plane(this);
            } else if cc_vehicle_appearance(t) == 3 {
                obbe_heli(this);
            } else if sub == 5 {
                obbe_boat(this);
            } else if sub == 6 {
                obbe_train(this);
            } else {
                obbe_car(this);
            }
            dx = req();
        }
    } else {
        if rd::<u8>(this, 0x26) != 0 && rd::<u8>(this, 0x2F) != 0 {
            l10 = true;
        }
        l12 = false;
        g::<u8>(DID_CINEMA).write(0);
    }
    // 6 / 7. mode decision
    if rd::<u8>(this, 0x2B) == 1 {
        looking_at_player(this, dx, l10, l11, l13);
    } else {
        not_looking_at_player(this, dx);
    }
    tail(this, l12, l90);
}

/// Block 2 (0x52812F-0x528D0C): vehicle target. Returns DI for the common part.
fn vehicle_target(this: GPtr<u8>, l11: &mut bool, l20: &mut u32) -> u16 {
    let forced = g::<i32>(FORCED_CAR_MODE).read();
    if forced > 0 {
        set_req(forced as u16);
        g::<i32>(FORCED_CAR_MODE).write(-1);
    }
    if rd::<i32>(tgt(this), 0x590) == 6 {
        set_req(3);
        return 3;
    }
    // b. zoom cycling
    'b: {
        if !(pad_cycle_camera_down(pad()) != 0 || pad_5404f0(pad()) != 0) {
            break 'b;
        }
        if g::<u8>(REPLAY_MODE).read() == 1 || rd::<u8>(this, 0x3D) != 0 {
            break 'b;
        }
        if rd::<u8>(this, 0x54) != 0 {
            break 'b;
        }
        if !(rd::<u8>(this, 0x2B) == 1 || rd::<i32>(this, 0xB0) == 2) {
            break 'b;
        }
        if cc_is_coop() != 0 {
            break 'b;
        }
        let down = pad_cycle_camera_down(pad()) != 0;
        let z = rd::<i32>(this, 0xB4) + if down { -1 } else { 1 };
        wr::<i32>(this, 0xB4, z);
        if z > 5 {
            wr::<i32>(this, 0xB4, 0);
        } else if z < 0 {
            wr::<i32>(this, 0xB4, 5);
        }
        let z = rd::<i32>(this, 0xB4);
        if z == 4 {
            let v = if pad_cycle_camera_down(pad()) != 0 { 3 } else { 5 };
            wr::<i32>(this, 0xB4, v);
        } else if z == 0 && rd::<u8>(this, 0x45) != 0 {
            let v = if pad_cycle_camera_down(pad()) != 0 { 5 } else { 1 };
            wr::<i32>(this, 0xB4, v);
        }
    }
    // c.
    if rd::<u8>(this, 0x54) != 0 {
        let z = rd::<i32>(this, 0xB4);
        if z != 4 && z != 0 {
            set_req(18);
        }
    }
    // d. boats, garages and stairs
    'd: {
        let t = tgt(this);
        let mut vt = rd::<i32>(t, 0x590);
        if vt == 5 {
            if rd::<u16>(t, 0x22) != 0x1CC {
                set_req(0x16);
                break 'd;
            }
            vt = 0;
        } else if vt != 0 && vt != 9 {
            break 'd;
        }
        let mut stairs = GPtr::<u8>::from_addr(0);
        if vt == 9 && cam_stairs_for_player() != 0 {
            stairs = find_zone_with_stairs_for_player();
            if !stairs.is_null() {
                *l11 = true;
            }
        }
        let p = read3(pos(tgt(this)));
        if is_point_in_garage_camera_zone(p[0], p[1], p[2]) == 0 && !*l11 {
            if rd::<u8>(this, 0x26) != 0 {
                wr::<u8>(this, 0x28, 1);
                wr::<u8>(this, 0x26, 0);
            }
            wr::<u8>(this, 0x4E, 0);
            set_req(18);
            break 'd;
        }
        'set: {
            if !((rd::<u8>(this, 0x4E) == 0 && rd::<u8>(this, 0x2B) == 1) || rd::<i32>(this, 0xB0) == 2) {
                break 'set;
            }
            let gar = rd::<GPtr<u8>>(this, 0x81C);
            if gar.is_null() && stairs.is_null() {
                break 'set;
            }
            let t = tgt(this);
            let pt = read3(pos(t)); // the pointer taken at 0x528363 (pos(T))
            let mut l30 = [0u32; 3];
            let mut l40 = [0u32; 3];
            let mut l50 = [0u32; 3];
            let mut doors = [0u32; 2]; // L74 / L78
            let sel: u32;
            if !gar.is_null() {
                let d = GPtr::from_raw(doors.as_mut_ptr()).cast::<u8>();
                find_doors_with_garage(gar, d, d.byte_add(4));
                let first = GPtr::<u8>::from_addr(doors[0] as usize);
                if !first.is_null() {
                    sel = 1;
                    let fp = pos(first);
                    l30[0] = rd::<u32>(fp, 0);
                    l30[1] = rd::<u32>(fp, 4);
                    l30[2] = 0;
                } else if doors[1] != 0 {
                    sel = 2;
                } else {
                    sel = 1;
                    let tp = pos(tgt(this));
                    l30[0] = rd::<u32>(tp, 0);
                    l30[1] = rd::<u32>(tp, 4);
                    l40[2] = 0;
                }
            } else {
                // stairs, no garage (0x528445)
                l30 = read3(cam(this).byte_add(0x19C));
                l50 = pt;
                sel = 1;
                let f = f32::from_bits;
                let ddx = x(f(l50[0])).fsub_m(f(l30[0]));
                let ddy = x(f(l50[1])).fsub_m(f(l30[1]));
                let dist = ddy.fmul(ddy).fadd(ddx.fmul(ddx)).fsqrt();
                if !tjne(dist, x(kd(0x858B48)), 0x41) {
                    let t = tgt(this);
                    cc_vec_sub(vbuf(&mut l40), pos(t), vbuf(&mut l50));
                    l40[2] = 0;
                    cc_normalise(vbuf(&mut l40));
                    let mut l64 = [0u32; 3];
                    let mut l14 = [0u32; 3];
                    stairs_push_out(
                        this,
                        stairs,
                        vbuf(&mut l40),
                        vbuf(&mut l30),
                        vbuf(&mut l64),
                        vbuf(&mut l14),
                    );
                }
                // the residue door pointers (never written on this path)
                doors[0] = entry_residue(-0x50).unwrap_or(0);
                doors[1] = entry_residue(-0x4C).unwrap_or(0);
            }
            // stub 0x401B3B: the garage centre or the target's position
            let gar = rd::<GPtr<u8>>(this, 0x81C);
            if !gar.is_null() {
                let gx = x(rd::<f32>(gar, 0x2C))
                    .fadd_m(rd::<f32>(gar, 0x28))
                    .fmul_m(k(K_HALF))
                    .to_f32();
                let gy = x(rd::<f32>(gar, 0x34))
                    .fadd_m(rd::<f32>(gar, 0x30))
                    .fmul_m(k(K_HALF))
                    .to_f32();
                l50 = [gx.to_bits(), gy.to_bits(), 0];
            } else {
                l30[2] = 0;
                if stairs.is_null() {
                    let t = tgt(this);
                    let a = pos(t);
                    let b = pos(t);
                    l50 = [rd::<u32>(b, 0), x(rd::<f32>(a, 4)).to_f32().to_bits(), 0];
                }
            }
            // D = (sel == 1 ? L30 : L40) - L50 is dead (normalised into L7C, never read)
            let mut found = [0u8; 4];
            let t = tgt(this);
            let pt = read3(pos(t));
            let _ = cc_find_ground_z(pt[0], pt[1], pt[2], GPtr::from_raw(found.as_mut_ptr()), 0);
            let l88 = pt;
            let mut d = [0u32; 3];
            let src = if sel == 1 { l30 } else { l40 };
            let f = f32::from_bits;
            d[0] = x(f(src[0])).fsub_m(f(l50[0])).to_f32().to_bits();
            d[1] = x(f(src[1])).fsub_m(f(l50[1])).to_f32().to_bits();
            d[2] = 0;
            cc_normalise(vbuf(&mut d));
            let first = GPtr::<u8>::from_addr(doors[0] as usize);
            let second = GPtr::<u8>::from_addr(doors[1] as usize);
            doors_fixed_cam(this, first, second, l88, l50, 0x8CCF1C, 0x8CCF18);
        }
        // d-tail (0x528A6E)
        let stairs_set = !stairs.is_null();
        if (camera_should_be_outside() != 0 || *l11)
            && rd::<u8>(this, 0x4E) == 1
            && (rd::<u8>(this, 0x2B) == 1 || rd::<i32>(this, 0xB0) == 2)
            && (!rd::<GPtr<u8>>(this, 0x81C).is_null() || stairs_set)
        {
            set_req(15);
            wr::<u8>(this, 0x26, 1);
        } else {
            if rd::<u8>(this, 0x26) != 0 {
                wr::<u8>(this, 0x28, 1);
                wr::<u8>(this, 0x26, 0);
            }
            set_req(18);
        }
    }
    // e. car zoom
    let mut l5c = [0u32; 1];
    let app = cc_vehicle_appearance(tgt(this));
    cc_get_arr_pos(this, app, vbuf(&mut l5c));
    let z = rd::<i32>(this, 0xB4);
    let di: u16;
    if z == 0 && rd::<u8>(this, 0x26) == 0 {
        wr::<i32>(this, 0xB8, 0);
        di = 16;
        set_req(di);
    } else {
        let tbl = match z {
            1 => Some(0x8CC3E0),
            2 => Some(0x8CC3F4),
            3 => Some(0x8CC408),
            _ => None,
        };
        if let Some(t) = tbl {
            wr::<u32>(this, 0xB8, g::<u32>(t + 4 * l5c[0] as usize).read());
        }
        di = req();
    }
    if z == 4 && rd::<u8>(this, 0x26) == 0 {
        wr::<u32>(this, 0xB8, K_ONE);
    }
    if eq0(rd::<f32>(this, 0xBC)) {
        wr::<u32>(this, 0xBC, rd::<u32>(this, 0xB8));
    }
    let s_p = this.byte_add(0xC0);
    if rd::<u8>(this, 0x39) == 1 {
        smooth_toward(s_p, || rd::<f32>(this, 0xC4));
    } else if rd::<u8>(this, 0x54) != 0 {
        *l20 = 0x3F26_6666;
        smooth_toward_const(s_p, kd(0x863274));
    } else {
        smooth_toward(s_p, || rd::<f32>(this, 0xB8));
        if z == 3 && eq0(rd::<f32>(this, 0xB8)) {
            wr::<u32>(this, 0xC0, rd::<u32>(this, 0xB8));
        }
    }
    let c = cam(this);
    cc_well_buffer_me(
        *l20,
        c.byte_add(0x64),
        c.byte_add(0x68),
        0x3DCC_CCCD,
        0x3E80_0000,
        0,
    );
    di
}

/// Block 3 (0x528D19-0x529FF4): ped target. `Some(di)` continues at 0x52A002 with that DI, `None` at
/// 0x529FFB (`di = REQ`).
fn ped_target(this: GPtr<u8>, l11: &mut bool, l3c: &mut u32, l12: &mut bool) -> Option<u16> {
    // a. zoom cycling
    'a: {
        if !(pad_cycle_camera_down(pad()) != 0 || pad_5404f0(pad()) != 0) {
            break 'a;
        }
        if g::<u8>(REPLAY_MODE).read() == 1
            || rd::<u8>(this, 0x3D) != 0
            || rd::<u8>(this, 0x54) != 0
            || rd::<u8>(this, 0x1F) != 0
        {
            break 'a;
        }
        if !(rd::<u8>(this, 0x2B) == 1 || rd::<i32>(this, 0xB0) == 2) {
            break 'a;
        }
        if cc_is_coop() != 0 {
            break 'a;
        }
        let down = pad_cycle_camera_down(pad()) != 0;
        let z = rd::<i32>(this, 0xC8) + if down { -1 } else { 1 };
        wr::<i32>(this, 0xC8, z);
        if z > 3 {
            wr::<i32>(this, 0xC8, 1);
        } else if z < 1 {
            wr::<i32>(this, 0xC8, 3);
        }
    }
    set_req(4);
    // b. first person
    let mut clear = true;
    let mut skip_clear = false;
    'b: {
        if !(rd::<u8>(this, 0x2B) == 1 || rd::<u8>(this, 0x41) != 0) {
            break 'b;
        }
        if etype(tgt(this)) != 3 {
            break 'b;
        }
        if rd::<u8>(this, 0x3D) != 0 && rd::<u8>(this, 0x41) == 0 {
            break 'b;
        }
        if cc_using_3rd_person_mouse_cam(this.byte_add(0x174)) != 0 {
            break 'b;
        }
        let intel = rd::<GPtr<u8>>(ped0(), 0x47C);
        if cc_task_secondary(intel.byte_add(4), 0).is_null() && pad_540e80(pad()) == 0 {
            pad_540f80(pad());
        }
        if rd::<u8>(this, 0x1F) == 0 {
            skip_clear = true;
            clear = false;
            break 'b;
        }
        if cc_pad_walk_lr(pad()) != 0 || cc_pad_walk_ud(pad()) != 0 {
            break 'b;
        }
        if [0x1C, 0x1E, 0x20, 0x22, 0x1A]
            .iter()
            .any(|&o| rd::<i16>(pad(), o) != 0)
        {
            break 'b;
        }
        let dt = g::<u32>(TIME_MS).read().wrapping_sub(rd::<u32>(this, 0x60));
        if !tjne(fild_u32(dt), x(kd(0x863270)), 0x41) {
            break 'b;
        }
        if cc_pad_enter_targeting(pad()) != 0 {
            wr::<u8>(this, 0x20, 1);
            break 'b;
        }
        skip_clear = true;
        clear = false;
    }
    if clear && !skip_clear {
        wr::<u8>(this, 0x1F, 0);
    }
    let p = ped0();
    if cc_is_ped_in_control(p) == 0 || {
        let pd = rd::<GPtr<u8>>(ped0(), 0x480);
        !tjne(x(rd::<f32>(pd, 0x14)), x(k(K_ZERO)), 0x41)
    } {
        wr::<u8>(this, 0x1F, 0);
    }
    if rd::<u8>(this, 0x1F) != 0 {
        set_req(0x10);
        let pp = pad();
        wr::<u8>(pp, 0x10E, rd::<u8>(pp, 0x10E) | 1);
    }
    // c. zoom smoothing
    let zoom = rd::<i32>(this, 0xC8);
    let base_off = match zoom {
        1 => 0xDC,
        3 => 0xE4,
        _ => 0xE0,
    };
    let c = cam(this);
    wr::<u32>(this, 0xCC, rd::<u32>(c, base_off));
    let s_p = this.byte_add(0xD4);
    if rd::<u8>(this, 0x38) == 1 {
        smooth_toward(s_p, || rd::<f32>(this, 0xD8));
    } else if rd::<u8>(this, 0x54) != 0 {
        *l3c = 0x3F33_3333;
        smooth_toward_const(s_p, kd(0x8CCF14));
    } else {
        smooth_toward(s_p, || rd::<f32>(this, 0xCC));
        if zoom == 3 && eq0(rd::<f32>(this, 0xCC)) {
            wr::<u32>(this, 0xD4, rd::<u32>(this, 0xCC));
        }
    }
    cc_well_buffer_me(
        *l3c,
        c.byte_add(0x5C),
        c.byte_add(0x60),
        0x3DCC_CCCD,
        0x3CCC_CCCD,
        0,
    );
    // d. garage / stairs
    let mut stairs = GPtr::<u8>::from_addr(0);
    if cam_stairs_for_player() != 0 {
        stairs = find_zone_with_stairs_for_player();
        if !stairs.is_null() {
            *l11 = true;
        }
    }
    let p = read3(pos(tgt(this)));
    let skip_to_e = if is_point_in_garage_camera_zone(p[0], p[1], p[2]) == 0 && !*l11 {
        if rd::<u8>(this, 0x26) != 0 {
            wr::<u8>(this, 0x28, 1);
            wr::<u8>(this, 0x26, 0);
        }
        wr::<u8>(this, 0x4E, 0);
        true
    } else {
        false
    };
    if !skip_to_e {
        if rd::<u8>(this, 0x4E) == 0 && rd::<u8>(this, 0x2B) == 1 {
            let gar = rd::<GPtr<u8>>(this, 0x81C);
            if !gar.is_null() || *l11 {
                ped_fixed_from_stairs_or_garage(this, stairs, *l11);
            }
            ped_doors(this);
        }
        // d-tail (0x529B9E)
        if (camera_should_be_outside() != 0 || *l11)
            && rd::<u8>(this, 0x2B) == 1
            && rd::<u8>(this, 0x4E) == 1
            && (!rd::<GPtr<u8>>(this, 0x81C).is_null() || *l11)
        {
            set_req(15);
            wr::<u8>(this, 0x26, 1);
        } else {
            if rd::<u8>(this, 0x26) != 0 {
                wr::<u8>(this, 0x28, 1);
                wr::<u8>(this, 0x26, 0);
            }
            set_req(4);
        }
    }
    // e. weapon modes (0x529BF4)
    let wm = rd::<u16>(this, 0x830);
    if pad_get_target(pad()) == 0
        && wm != 0
        && wm != 0x2D
        && wm != 0x37
        && wm != 0x41
        && !(wm == 0x2E && !rd::<GPtr<u8>>(ped0(), 0xFC).is_null())
    {
        cc_clear_weapon_mode(this);
    }
    let mut di = rd::<u16>(this, 0x824);
    if di != 0 {
        set_req(di);
    } else {
        di = req();
    }
    let bp = rd::<u16>(this, 0x830);
    if bp == 0 || *l11 {
        return Some(di);
    }
    if [7, 8, 0x33, 0x22, 0x2D, 0x2E].contains(&bp) || cc_weapon_first_person_on(cam(this)) != 0 {
        // f. first-person weapon (0x52A086)
        let d = if rd::<i32>(focus_ped(), 0x530) == 0x18
            && di != 0x25
            && cc_weapon_first_person_on(cam(this)) == 0
        {
            4
        } else {
            bp
        };
        set_req(d);
        return Some(d);
    }
    if di == 0x25 {
        return Some(di);
    }
    *l12 = false;
    let c = cam(this);
    let mut l24 = read3(pos(tgt(this)));
    let mut l14 = [0u32; 3];
    let mut l40 = [0u32; 3];
    cc_vec_sub(vbuf(&mut l14), this.byte_add(0x854), vbuf(&mut l24));
    cc_vec_sub(vbuf(&mut l40), c.byte_add(0x19C), vbuf(&mut l24));
    let mut l20: u32 = 0;
    if rd::<GPtr<u8>>(ped0(), 0x71C).is_null() && rd::<u8>(rd::<GPtr<u8>>(ped0(), 0x480), 0x34) & 8 == 0 {
        return None;
    }
    if !rd::<GPtr<u8>>(ped0(), 0x71C).is_null() && etype(rd::<GPtr<u8>>(ped0(), 0x71C)) == 3 && {
        rd::<i32>(rd::<GPtr<u8>>(ped0(), 0x71C), 0x530) == 0x37
            || rd::<i32>(rd::<GPtr<u8>>(ped0(), 0x71C), 0x530) == 0x36
    } {
        *l12 = true;
        l20 = g::<u32>(0x8CCF08).read();
    }
    let f = f32::from_bits;
    let l78 = x(f(l14[1]))
        .fmul_m(f(l14[1]))
        .fadd(x(f(l14[0])).fmul_m(f(l14[0])))
        .fsqrt()
        .to_f32();
    let l3c_angle = cc_atan_xy(l40[0], l40[1]) as f32;
    let _ = cc_atan_xy(l14[0], l14[1]);
    let mut di = rd::<u16>(this, 0x830);
    set_req(di);
    let mut l74: u32 = 0;
    if di == 0x35 {
        if !*l12 {
            g::<u8>(SYPHON_LATCH).write(0);
            return Some(di);
        }
        'aim: {
            if rd::<GPtr<u8>>(ped0(), 0x71C).is_null() {
                di = req();
                break 'aim;
            }
            if rd::<u8>(this, 0x58) != 0 && cam_mode(this) != 0x23 {
                di = req();
                break 'aim;
            }
            di = 0x23;
            let thr = if cam_mode(this) == 0x23 && etype(rd::<GPtr<u8>>(ped0(), 0x71C)) == 3 {
                kd(0x8CCF04)
            } else {
                f(l20)
            };
            if tjp(x(l78), x(thr), 0x05) {
                di = req();
                break 'aim;
            }
            l74 = 0x40B3_3333;
            set_req(di);
        }
    }
    if di != 0x23 {
        g::<u8>(SYPHON_LATCH).write(0);
        return Some(di);
    }
    // 0x529EFA
    if g::<u8>(SYPHON_LATCH).read() != 0 {
        return Some(di);
    }
    let t = tgt(this);
    let mut l20e = [0u32; 1];
    let pt = read3(pos(t));
    let a = x(l3c_angle);
    let cx = a.fcos().fmul_m(f(l74)).fadd_m(f(pt[0])).to_f32();
    let cy = a.fsin().fmul_m(f(l74)).fadd_m(f(pt[1])).to_f32();
    let cz = x(f(pt[2])).fadd_m(kd(0x863264)).to_f32();
    let mut cpos = [cx.to_bits(), cy.to_bits(), cz.to_bits()];
    let mut cp = [0u32; 0x2C / 4];
    let hit = cc_process_los(
        pos(t),
        vbuf(&mut cpos),
        vbuf(&mut cp),
        vbuf(&mut l20e),
        1,
        0,
        0,
        1,
        0,
        1,
        1,
        0,
    );
    let mut zero = [0u32; 3];
    if hit != 0 {
        cc_set_fixed(this, vbuf(&mut cp), vbuf(&mut zero));
    } else {
        cc_set_fixed(this, vbuf(&mut cpos), vbuf(&mut zero));
    }
    g::<u8>(SYPHON_LATCH).write(1);
    None
}

/// Block 3d, the fixed position from the garage or the stairs (0x529209-0x529908).
fn ped_fixed_from_stairs_or_garage(this: GPtr<u8>, stairs: GPtr<u8>, l11: bool) {
    let gar = rd::<GPtr<u8>>(this, 0x81C);
    let mut l30 = [0u32; 3];
    let mut l40 = [0u32; 3];
    let mut l50 = [0u32; 3];
    let mut sel: u32 = 2;
    if !gar.is_null() {
        let mut doors = [tgt(this).addr() as u32, 0u32]; // L20 = T, L3C
        let d = GPtr::from_raw(doors.as_mut_ptr()).cast::<u8>();
        find_doors_with_garage(gar, d, d.byte_add(4));
        let first = GPtr::<u8>::from_addr(doors[0] as usize);
        if !first.is_null() {
            sel = 1;
            let fp = pos(first);
            l30 = [rd::<u32>(fp, 0), rd::<u32>(fp, 4), 0];
        } else if doors[1] == 0 {
            sel = 1;
            let tp = pos(tgt(this));
            l30[0] = rd::<u32>(tp, 0);
            l30[1] = rd::<u32>(tp, 4);
            l40[2] = 0;
        }
    } else {
        // stairs, no garage (0x5292D5)
        l30 = read3(cam(this).byte_add(0x19C));
        sel = 1;
        if !stairs.is_null() {
            let mut c = [0u32; 3];
            let r = zone_centre(stairs, vbuf(&mut c));
            l50 = read3(r);
            let t = tgt(this);
            cc_vec_sub(vbuf(&mut l40), pos(t), vbuf(&mut l50));
            l40[2] = 0;
            cc_normalise(vbuf(&mut l40));
            let mut l64 = [0u32; 3];
            let mut l24 = [0u32; 3];
            stairs_push_out(
                this,
                stairs,
                vbuf(&mut l40),
                vbuf(&mut l30),
                vbuf(&mut l64),
                vbuf(&mut l24),
            );
        }
    }
    // 0x529507
    let gar = rd::<GPtr<u8>>(this, 0x81C);
    if !gar.is_null() {
        let gx = x(rd::<f32>(gar, 0x2C))
            .fadd_m(rd::<f32>(gar, 0x28))
            .fmul_m(k(K_HALF))
            .to_f32();
        let gy = x(rd::<f32>(gar, 0x34))
            .fadd_m(rd::<f32>(gar, 0x30))
            .fmul_m(k(K_HALF))
            .to_f32();
        l50 = [gx.to_bits(), gy.to_bits(), 0];
    } else {
        l30[2] = 0;
        if stairs.is_null() {
            let t = tgt(this);
            let a = pos(t);
            let b = pos(t);
            l50 = [rd::<u32>(b, 0), x(rd::<f32>(a, 4)).to_f32().to_bits(), 0];
        }
    }
    let f = f32::from_bits;
    let src = if sel == 1 { l30 } else { l40 };
    let mut dv = [
        x(f(src[0])).fsub_m(f(l50[0])).to_f32().to_bits(),
        x(f(src[1])).fsub_m(f(l50[1])).to_f32().to_bits(),
        x(f(src[2])).fsub_m(f(l50[2])).to_f32().to_bits(),
    ];
    let t = tgt(this);
    let pt = read3(pos(t));
    let mut found = [0u8; 4];
    let gz = cc_find_ground_z(pt[0], pt[1], pt[2], GPtr::from_raw(found.as_mut_ptr()), 0);
    let mut l3c = (gz as f32).to_bits();
    if found[0] == 0 {
        l3c = x(f(pt[2])).fsub_m(kd(0x858CC4)).to_f32().to_bits();
    }
    dv[2] = 0;
    let mut l24 = [0u32; 3];
    let dvp = vbuf(&mut dv);
    if sel == 1 {
        if rd::<GPtr<u8>>(this, 0x81C).is_null() && l11 {
            cc_normalise(dvp);
            if !stairs.is_null() {
                let n = zone_extent(stairs);
                let k2 = xi(n).fmul_m(kd(0x858CB0)).fadd_m(kd(0x86326C)).to_f32().to_bits();
                cc_vec_s_mul(vbuf(&mut l24), k2, dvp);
                let mut r = [0u32; 3];
                cc_vec_add(vbuf(&mut r), vbuf(&mut l50), vbuf(&mut l24));
                l30 = r;
            } else {
                cc_vec_s_mul(vbuf(&mut l24), 0x4070_0000, dvp);
                let mut r = [0u32; 3];
                cc_vec_add(vbuf(&mut r), vbuf(&mut l30), vbuf(&mut l24));
                l30 = r;
            }
        } else {
            cc_normalise(dvp);
            cc_vec_s_mul(vbuf(&mut l24), 0x4150_0000, dvp);
            let mut r = [0u32; 3];
            cc_vec_add(vbuf(&mut r), vbuf(&mut l30), vbuf(&mut l24));
            l30 = r;
        }
    } else {
        cc_normalise(dvp);
        cc_vec_s_mul(vbuf(&mut l24), 0x4150_0000, dvp);
        let mut r = [0u32; 3];
        cc_vec_add(vbuf(&mut r), vbuf(&mut l40), vbuf(&mut l24));
        l30 = r;
    }
    if rd::<i32>(this, 0xC8) == 4 && !l11 {
        l30 = l50;
        let pz = rd::<f32>(pos(ped0()), 8);
        let mut z = x(f(l50[2])).fadd_m(pz).fadd_m(kd(0x859974)).to_f32().to_bits();
        let gar = rd::<GPtr<u8>>(this, 0x81C);
        if !gar.is_null() && !tjne(x(f(z)), x(rd::<f32>(gar, 0x2C)), 0x41) {
            z = rd::<u32>(gar, 0x2C);
        }
        l30[2] = z;
    } else {
        l30[2] = x(f(l3c)).fadd_m(kd(0x863268)).to_f32().to_bits();
    }
    let mut zero = [0u32; 3];
    cc_set_fixed(this, vbuf(&mut l30), vbuf(&mut zero));
    wr::<u8>(this, 0x4E, 1);
}

/// Block 3d doors (0x52990C): the door-based fixed camera when a garage is set.
fn ped_doors(this: GPtr<u8>) {
    let gar = rd::<GPtr<u8>>(this, 0x81C);
    if gar.is_null() {
        return;
    }
    let mut doors = [0u32; 2]; // L3C first, L20 second
    let d = GPtr::from_raw(doors.as_mut_ptr()).cast::<u8>();
    find_doors_with_garage(gar, d, d.byte_add(4));
    let first = GPtr::<u8>::from_addr(doors[0] as usize);
    let second = GPtr::<u8>::from_addr(doors[1] as usize);
    // L88 / L50 as the stairs-or-garage block left them: recomputed the same way (L88 = pos(T) at
    // FindGroundZ time, L50 = the garage centre since a garage is set)
    let t = tgt(this);
    let l88 = read3(pos(t));
    let gx = x(rd::<f32>(gar, 0x2C))
        .fadd_m(rd::<f32>(gar, 0x28))
        .fmul_m(k(K_HALF))
        .to_f32();
    let gy = x(rd::<f32>(gar, 0x34))
        .fadd_m(rd::<f32>(gar, 0x30))
        .fmul_m(k(K_HALF))
        .to_f32();
    let l50 = [gx.to_bits(), gy.to_bits(), 0];
    doors_fixed_cam(this, first, second, l88, l50, 0x8CCF10, 0x8CCF0C);
}

/// Block 4d's loop over the ped's 16 scanned entities (0x52A1F6).
fn dead_player_mode(this: GPtr<u8>) -> u16 {
    let t = tgt(this);
    if etype(t) == 3 {
        let list = rd::<GPtr<u8>>(t, 0x47C).byte_add(0x130);
        for i in 0..16 {
            let e = rd::<GPtr<u8>>(list, 4 * i);
            if e.is_null() {
                continue;
            }
            let task = cc_find_active_task_by_type(rd::<GPtr<u8>>(e, 0x47C).byte_add(4), 0x44C);
            if task.is_null() {
                continue;
            }
            if rd::<GPtr<u8>>(task, 8) != ped0() {
                continue;
            }
            let mut d = [0u32; 3];
            let r = cc_vec_sub(vbuf(&mut d), pos(e), pos(t));
            let m = st0(cc_magnitude(r));
            if tjnp(m, x(kd(0x858B90)), 0x05) {
                wr::<u8>(cam(this), 0xA, 1);
                return 0x20;
            }
        }
    }
    wr::<u8>(cam(this), 0xA, 1);
    0x1D
}

/// Block 6 (0x52A757-0x52B210): the mode decision while looking at the player.
fn looking_at_player(this: GPtr<u8>, dx_in: u16, l10_in: bool, l11: bool, l13: bool) {
    let mut dx = dx_in;
    let mut l10 = l10_in;
    let ax_at = |this: GPtr<u8>| cam_mode(this);
    // a.
    let mut goto_c = false;
    if dx == 1 || dx == 0x10 || dx == 0x25 {
        l10 = true;
    } else if dx == 0x12 || dx == 0x16 {
        let m = cam_mode(this);
        if m == 1 || m == 0x10 || m == 0x25 {
            l10 = true;
        }
    } else if dx == 0xF {
        if cam_mode(this) == 1 {
            l10 = true;
        }
        goto_c = true;
    }
    if !goto_c {
        // b.
        let mut b_done = false;
        if dx == 0x35 || dx == 0x37 || dx == 0x41 {
            let t = tgt(this);
            if !t.is_null() && etype(t) == 3 {
                let jet = !cc_task_jet_pack(rd::<GPtr<u8>>(t, 0x47C)).is_null();
                if req() == 0x35 && cam_mode(this) == 4 && !jet {
                    let c = cam(this);
                    let t = tgt(this);
                    let tgo = rd::<GPtr<u8>>(t, 0x71C);
                    let a = if !tgo.is_null() {
                        let mut d = [0u32; 3];
                        cc_vec_sub(vbuf(&mut d), pos(tgo), pos(t));
                        let f = f32::from_bits;
                        x(f(d[0])).fchs().fpatan(x(f(d[1])))
                    } else {
                        st0(cc_get_heading(t))
                    };
                    let a = a.fsub_m(k(K_HALF_PI));
                    if heading_off(a, c) {
                        l10 = true;
                    } else {
                        let mut d = [0u32; 3];
                        let r = cc_vec_sub(vbuf(&mut d), pos(t), this.byte_add(0x9A4));
                        let m = st0(cc_magnitude(r));
                        let q = x(kd(PED_ZOOM_SMOOTHED_GLOBAL))
                            .fadd_m(kd(0x858CA0))
                            .fmul_m(kd(0x858CE8));
                        if !tjp(q, m, 0x05) {
                            l10 = true;
                        }
                    }
                    if g::<u8>(USE_MOUSE_3RD).read() != 0 {
                        l10 = false;
                    }
                    b_done = true;
                } else {
                    dx = req();
                    l10 = true;
                }
            }
        }
        if !b_done {
            if dx == 0x30 {
                if cam_mode(this) != 0x31 {
                    l10 = true;
                }
            } else if dx == 0x31 && cam_mode(this) != 0x30 {
                l10 = true;
            }
        }
    }
    // c. (0x52A994)
    let ax = ax_at(this);
    dx = req();
    if (ax == 0x30 && dx != 0x31) || (ax == 0x31 && dx != 0x30) {
        l10 = true;
    }
    // d-g.
    let mut goto_h = false;
    let mut goto_g = false;
    if dx == 1 || dx == 0x25 {
        let other = if dx == 1 { 0x25 } else { 1 };
        if ax == other || ax == 0x1D {
            l10 = false;
            goto_g = true;
        } else {
            goto_h = true;
        }
    } else if [
        0x10, 7, 0x22, 8, 0x33, 0x27, 0x28, 0x34, 0x2A, 0x2B, 0x29, 0x2D, 0x20, 0x21, 0x2E,
    ]
    .contains(&dx)
        && etype(tgt(this)) == 3
    {
        l10 = true;
        goto_h = true;
    } else if dx == 0xF {
        if rd::<u8>(this, 0x26) != 1 {
            goto_g = true;
        } else {
            if ([7, 0x2D, 8, 0x33, 0x22, 0x25].contains(&ax)
                || l11
                || [0x10, 0x27, 0x28, 0x34, 0x2A, 0x2B, 0x29, 0x2E].contains(&ax))
                && !tgt(this).is_null()
                && etype(tgt(this)) == 2
            {
                l10 = true;
            }
            goto_h = true;
        }
    } else if dx == 4 {
        let mut bl = false;
        if ax == 0x35 {
            let t = tgt(this);
            if cc_can_run_and_fire(t) != 0 && rd::<u32>(tgt(this), 0x46C) & 0x0400_0000 == 0 {
                let t = tgt(this);
                let a = st0(cc_get_heading(t)).fsub_m(k(K_HALF_PI));
                if heading_off(a, cam(this)) || rd::<u8>(t, 0x46C) & 1 == 0 {
                    bl = true;
                }
                dx = req();
                if g::<u8>(USE_MOUSE_3RD).read() != 0 {
                    bl = false;
                    wr::<u8>(this, 0x28, 1);
                }
            } else {
                dx = req();
            }
        }
        let c = cam(this);
        let m = rd::<u16>(c, 0xC);
        const A: [u16; 19] = [
            0x10, 7, 0x22, 8, 0x33, 0x1D, 0x20, 0x21, 0x1E, 0x27, 0x28, 0x34, 0x2A, 0x2B, 0x29, 0x2D, 1,
            0x25, 0x2E,
        ];
        if (A.contains(&m) || bl || l13) && rd::<u8>(this, 0x28) == 0 {
            if [
                7, 8, 0x33, 0x22, 0x10, 0x27, 0x28, 0x34, 0x2A, 0x2B, 0x29, 0x2D, 0x2E,
            ]
            .contains(&m)
            {
                let b = st0(cc_atan_xy(rd::<u32>(c, 0x190), rd::<u32>(c, 0x194))).fsub_m(k(K_HALF_PI));
                wr::<f32>(tgt(this), 0x558, b.to_f32());
                wr::<f32>(tgt(this), 0x55C, b.to_f32());
            }
            let c = cam(this);
            wr::<u8>(this, 0x37, 1);
            l10 = true;
            let v = if rd::<u16>(c, 0xC) == 0x25 {
                let mut l14 = [0u32; 3];
                cc_vec_sub(vbuf(&mut l14), cam(this).byte_add(0x19C), pos(ped0()));
                l14[2] = 0;
                cc_normalise(vbuf(&mut l14));
                cc_atan_xy(0x3A83_126F, 0x3F80_0000) as f32
            } else {
                st0(cc_atan_xy(rd::<u32>(c, 0x190), rd::<u32>(c, 0x194)))
                    .fadd_m(k(K_PI))
                    .to_f32()
            };
            dx = req();
            wr::<f32>(cam(this), 0x9C, v);
        }
        goto_h = true;
    } else if [0x26, 0x20, 0x21, 0x1D].contains(&dx) {
        l10 = true;
        goto_h = true;
    } else {
        goto_g = true;
    }
    if goto_g && !goto_h && ax == 0x1D {
        l10 = true;
    }
    // h. (0x52ADD6)
    let c = cam(this);
    let bp = rd::<u16>(c, 0xC);
    if dx != bp && rd::<GPtr<u8>>(c, 0x21C).is_null() {
        l10 = true;
    }
    let bl = rd::<u8>(this, 0x26) != 0;
    if bl {
        let gar = rd::<GPtr<u8>>(this, 0x81C);
        if !gar.is_null() {
            let ty = rd::<u8>(gar, 0x4C);
            if (ty == 2 || ty == 3 || ty == 4) && {
                let t = tgt(this);
                etype(t) == 2 && rd::<u16>(t, 0x22) == 0x1A7 && dx != bp
            } {
                l10 = true;
            }
        }
        let e = rd::<GPtr<u8>>(c, 0x21C);
        if !e.is_null() {
            let p = pos(e);
            let ax_ = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(this, 0x86C));
            let ay = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(this, 0x870));
            let az = x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(this, 0x874));
            let p = pos(e);
            let bx = x(rd::<f32>(p, 0)).fsub_m(rd::<f32>(c, 0x19C));
            let by = x(rd::<f32>(p, 4)).fsub_m(rd::<f32>(c, 0x1A0));
            let bz = x(rd::<f32>(p, 8)).fsub_m(rd::<f32>(c, 0x1A4));
            let s = bx.fmul(ax_).fadd(bz.fmul(az)).fadd(by.fmul(ay));
            if !tjp(s, x(k(K_ZERO)), 0x05) {
                l10 = true;
            }
        }
    }
    // i. switch (0x52AEC8)
    let trans = rd::<u8>(this, 0x58) != 0;
    if dx != bp && (l10 || !trans) {
        if l10 {
            jump_cut_6j(this, bl, dx);
        } else if rd::<u8>(this, 0x3A) == 0 {
            cc_start_transition(this, u32::from(dx));
        }
        return;
    }
    if trans && dx != bp {
        let mut dxe = u32::from(dx);
        if !(rd::<u8>(this, 0x3A) != 0 || rd::<u8>(this, 0x2B) == 0) {
            let f = |o: isize, m: isize| {
                x(rd::<f32>(pos(ped0()), o))
                    .fsub_m(rd::<f32>(this, m))
                    .to_f32()
                    .to_bits()
            };
            let mut d = [f(0, 0x9A4), f(4, 0x9A8), f(8, 0x9AC)];
            let t = tgt(this);
            let far = !t.is_null() && etype(t) == 3 && {
                let m = st0(cc_magnitude(vbuf(&mut d)));
                !tjne(m, x(kd(0x863260)), 0x41)
            };
            dxe = u32::from(req());
            if far && (dxe as u16 == 0xB || dxe as u16 == 0x1C) {
                wr::<u8>(this, 0x3A, 1);
            }
        }
        if rd::<u8>(this, 0x3A) != 1 {
            wr::<u8>(this, 0x4F, 1);
            cc_start_transition(this, dxe);
        }
        return;
    }
    if dx == 0xF && tgt(this) != rd::<GPtr<u8>>(cam(this), 0x21C) && bl {
        if trans {
            wr::<u8>(this, 0x4F, 1);
        }
        cc_start_transition(this, 0xF);
    }
}

/// Block 6j, the jump cut (0x52AF2B).
fn jump_cut_6j(this: GPtr<u8>, bl: bool, dx: u16) {
    let dx = jump_cut_dx(this, bl, dx);
    wr::<u16>(cam(this), 0xC, dx);
    wr::<u8>(this, 0x2A, 1);
    let lav = rd::<u8>(this, 0x2C);
    jump_cut_copies(this, Some(lav));
    wr::<u32>(this, 0xC0, rd::<u32>(this, 0xB8));
    wr::<u32>(this, 0xD4, rd::<u32>(this, 0xCC));
    wr::<u8>(this, 0x58, 0);
    wr::<u8>(this, 0x4F, 0);
    wr::<u8>(this, 0x32, 0);
    wr::<u8>(cam(this), 0xA, 1);
}

fn jump_cut_dx(this: GPtr<u8>, bl: bool, dx: u16) -> u16 {
    if !(bl && rd::<u8>(this, 0x28) == 0)
        && ![4, 0x22, 7, 8, 0x33, 0x2E, 0xB, 0x10, 0x1C, 0x23].contains(&dx)
        && g::<u8>(USE_MOUSE_3RD).read() == 0
    {
        cc_set_behind(this);
        return req();
    }
    dx
}

/// Block 7 (0x52B215-0x52B4F5): not looking at the player.
fn not_looking_at_player(this: GPtr<u8>, dx_in: u16) {
    let mut dx = dx_in;
    let mut bl = false;
    let mut l13 = false;
    if rd::<u8>(this, 0x41) == 1 || rd::<u8>(this, 0x42) == 1 {
        if dx == 0x10 {
            if cam_mode(this) != dx {
                bl = true;
            }
        } else {
            let wm = rd::<u16>(this, 0x830);
            if (wm == 7 || wm == 0x10 || wm == 8 || wm == 0x33)
                && pad_get_target(pad()) != 0
                && rd::<u8>(this, 0x42) != 0
            {
                bl = true;
                l13 = true;
            } else if cam_mode(this) != rd::<u16>(this, 0xC38) {
                wr::<u8>(this, 0x32, 1);
                wr::<u16>(this, 0xC3C, 2);
                let p = pad();
                wr::<u8>(p, 0x10E, rd::<u8>(p, 0x10E) & 0xFE);
            }
            dx = req();
        }
    }
    let trans = rd::<u8>(this, 0x58) != 0;
    let scr = rd::<u8>(this, 0x32) == 1;
    if scr && rd::<u16>(this, 0xC3C) == 1 {
        let m = rd::<u16>(this, 0xC38);
        set_req(m);
        if trans {
            wr::<u8>(this, 0x4F, 1);
        }
        cc_start_transition(this, u32::from(m));
        return;
    }
    if !((scr && rd::<u16>(this, 0xC3C) == 2) || bl) {
        return;
    }
    wr::<u8>(this, 0x58, 0);
    wr::<u8>(this, 0x4F, 0);
    let m = if rd::<u8>(this, 0x41) == 1 && dx == 0x10 {
        dx
    } else if l13 {
        rd::<u16>(this, 0x830)
    } else {
        rd::<u16>(this, 0xC38)
    };
    wr::<u16>(cam(this), 0xC, m);
    wr::<u8>(this, 0x2A, 1);
    wr::<u8>(cam(this), 0xA, 1);
    let lav = rd::<u8>(this, 0x2C);
    jump_cut_copies(this, Some(lav));
    wr::<u8>(this, 0x2A, 1);
    wr::<u32>(this, 0xC0, rd::<u32>(this, 0xB8));
    wr::<u32>(this, 0xD4, rd::<u32>(this, 0xCC));
}

/// Block 8 (0x52B4FB-0x52B725).
fn tail(this: GPtr<u8>, l12: bool, l90: i32) {
    wr::<u8>(this, 0x32, 0);
    if rd::<GPtr<u8>>(cam(this), 0x21C).is_null() {
        wr::<GPtr<u8>>(cam(this), 0x21C, tgt(this));
        let slot = cam(this).byte_add(0x21C);
        register_reference(rd::<GPtr<u8>>(slot, 0), slot);
    }
    let m = cam_mode(this);
    let vis_path = m == 0x11 || (etype(tgt(this)) == 3 && [0x10, 7, 0x22, 0x2E, 0x2D, 8, 0x33].contains(&m));
    if vis_path {
        if rd::<u8>(ped0(), 0x1C) & 0x80 != 0 {
            let p = ped0();
            wr::<u32>(p, 0x1C, rd::<u32>(p, 0x1C) & 0xFFFF_FF7F);
            let h = cc_task_hold(rd::<GPtr<u8>>(ped0(), 0x47C), 0);
            if !h.is_null() {
                let o = rd::<GPtr<u8>>(h, 8);
                if !o.is_null() {
                    wr::<u32>(o, 0x1C, rd::<u32>(o, 0x1C) & 0xFFFF_FF7F);
                }
            }
        }
    } else {
        let p = ped0();
        wr::<u32>(p, 0x1C, rd::<u32>(p, 0x1C) | 0x80);
    }
    if cam_mode(this) == 0xF {
        let p = ped0();
        let f = rd::<u32>(p, 0x1C);
        let v = u32::from(g::<u8>(PLAYER_PED_VISIBLE).read()) << 7;
        wr::<u32>(p, 0x1C, f ^ ((v ^ f) & 0x80));
    }
    let mut bl = false;
    if !l12 && rd::<i32>(this, 0xB0) == 2 {
        cc_restore_with_jump_cut(this);
        bl = true;
        wr::<u8>(this, 0x1A, 1);
        let p = ped0();
        if !p.is_null() {
            let m = rd::<GPtr<u8>>(p, 0x14);
            let a = cc_atan_xy(rd::<u32>(m, 0x10), rd::<u32>(m, 0x14));
            wr::<f32>(this, 0x120, a as f32);
        }
    }
    let m = cam_mode(this);
    if (l90 != i32::from(m as i16) || bl || m == 4 || m == 0x12)
        && pad_540530(pad()) != 0
        && g::<u8>(REPLAY_MODE).read() != 1
        && (rd::<u8>(this, 0x2B) == 1 || rd::<i32>(this, 0xB0) == 2)
        && rd::<u8>(this, 0x3D) == 0
        && (rd::<i32>(this, 0xB0) != 2 || (g::<u8>(BYTE_B6EC34).read() == 1 && rd::<u16>(pad(), 0x10E) == 0))
    {
        cc_report_frontend_audio(GPtr::from_addr(AUDIO_ENGINE), 0x20, 0, K_ONE);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_CAM_CONTROL];

/// This module's globals (none: all through `g`).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
