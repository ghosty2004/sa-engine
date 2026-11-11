//! PORT-CAM-3 (helper A): `CCam::Process_Rocket` (0x511B50), the rocket-launcher first-person camera
//! (modes 8 and 51, heat seeking). Ported from the 1.0 US bytes (gta-reversed reverses it; reading aid
//! only, the bytes win). Same technique as [`crate::followped`]: a frame array at the original's
//! `B`-relative offsets (B = ESP after the four register pushes) and the [`Fpu`] machine.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::platform::SCENE_CAMERA;
use sa_core::{hook, orig};

use crate::camcol::set_near_clip_raw;
use crate::cammodes::ped_get_bone_position;
use crate::camproc::{find_player_ped, get_atan_of_xy_bits, get_pad, get_vectors_ready_for_rw};
use crate::common::{Fpu, g, k, rd, wr};
use crate::followped::{clip_beta, copy3, d, entity_pos, fcos, fsin, push_st0};

orig! {
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70)
    pub fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40)
    pub fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    pub fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CEntity::UpdateRpHAnim()` (0x532B20)
    pub fn entity_update_rp_h_anim(e: GPtr<u8>) = thiscall 0x532B20 => unported;
    /// `CPad::LookAroundLeftRight(CPed*)` / `LookAroundUpDown(CPed*)` (0x540BD0 / 0x540CC0, AX)
    pub fn pad_look_around_left_right(pad: GPtr<u8>, ped: GPtr<u8>) -> i16 = thiscall 0x540BD0 => unported;
    pub fn pad_look_around_up_down(pad: GPtr<u8>, ped: GPtr<u8>) -> i16 = thiscall 0x540CC0 => unported;
    /// `CWeapon::PickTargetForHeatSeekingMissile(CVector origin, CVector dir, float distMul, CEntity* ignore,
    /// bool fromVehicle, CEntity* last)` (0x73F910, cdecl); the vectors and the float as dwords
    pub fn pick_target_for_heat_seeking_missile(ox: u32, oy: u32, oz: u32, dx: u32, dy: u32, dz: u32, mul: u32, ignore: GPtr<u8>, from_veh: u32, last: GPtr<u8>) -> GPtr<u8> = cdecl 0x73F910 => unported;
    /// `CWorld::GetIsLineOfSightClear(...)` (0x56A490, AL)
    pub fn get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bu: u32, ve: u32, pe: u32, ob: u32, du: u32, st: u32, ig: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CWeaponEffects::MarkTarget(int32, CVector pos, uint8 r, uint8 g, uint8 b, uint8 a, float size,
    /// uint8)` (0x742BF0, cdecl); the vector and the float as dwords
    pub fn weapon_effects_mark_target(i: i32, px: u32, py: u32, pz: u32, r: u32, gr: u32, b: u32, a: u32, size: u32, flag: u32) = cdecl 0x742BF0 => unported;
}

/// Statics reset with the camera (u8 0xB70000, f32 0xB6FFFC / 0xB6FFF8).
const S_B70000: usize = 0xB70000;
const S_B6FFFC: usize = 0xB6FFFC;
const S_B6FFF8: usize = 0xB6FFF8;
const USE_MOUSE_3RD_PERSON: usize = 0xB6EC2E;
const MOUSE_ACCEL_VERTICAL: usize = 0xB6EC18;
const MOUSE_ACCEL_HORZNTL: usize = 0xB6EC1C;
const MOUSE_X: usize = 0xB73424;
const MOUSE_Y: usize = 0xB73428;
const TIME_STEP: usize = 0xB7CB5C;
const TIME_MS: usize = 0xB7CB84;
const CAMERA_TARGET: usize = 0xB6F980;
/// `CWeaponEffects` cross-hair 0 fields written here (0xC8A83C, colour bytes 0xC8A84C..4E, 0xC8A85C).
const XHAIR_0: usize = 0xC8A83C;
const XHAIR_R: usize = 0xC8A84C;
const XHAIR_G: usize = 0xC8A84D;
const XHAIR_B: usize = 0xC8A84E;
const XHAIR_SIZE: usize = 0xC8A85C;
/// The near clip of this mode (0x8CCC9C, 0.15).
const ROCKET_NEAR_CLIP: usize = 0x8CCC9C;

const K_1: u32 = 0x3F80_0000; // [0x858624]
const K_0: u32 = 0; // [0x858B50]
const K_M1: u32 = 0xBF80_0000; // [0x858C1C]
const K_0_1: u32 = 0x3DCC_CCCD; // [0x858B1C]
const K_4: u32 = 0x4080_0000; // [0x858B90]
const K_0_0001: u32 = 0x38D1_B717; // [0x858FC4]
const K_HALF_PI: u32 = 0x3FC9_0FDB; // [0x858FE4]
const K_M3: u32 = 0xC040_0000; // [0x859024]
const K_1_14: u32 = 0x3D92_4925; // [0x859B50]
const K_0_0125: u32 = 0x3C4C_CCCD; // [0x85A5C4]
const K_ALPHA_MIN: u32 = 0xBFC7_F1E7; // [0x8630F4]
const K_ALPHA_MAX: u32 = 0x3F86_0A92; // [0x8630F8]
const K_UD_SCALE: u32 = 0x383A_69DC; // [0x8630FC]
const K_LR_SCALE: u32 = 0x3D6A_0EA1; // [0x863100]

/// `CCam::Process_Rocket(const CVector&, float, float, float, bool heatSeeking)` (0x511B50, thiscall,
/// `ret 0x14`). Order of calls, re-reads and x87 operations as in the bytes:
/// - a non-ped target only sets the near clip ([0x8CCC9C]);
/// - FOV 70; reset: beta2 (+0xA8) = beta with the mouse and no aimed-at entity, else alpha 0, beta =
///   beta2 = heading - pi/2; the three statics zeroed, collision checks on;
/// - the ped's RW matrix / frame / animation updated, source = bone 5 + (0, 0, 0.1);
/// - mouse (x or y non-zero): `dB = (mx * -3) * (FOV*0.0125 * accelH)`, `dA = (FOV*0.0125 * accelV) *
///   (my * 4)`; pad: `dB = sign(LR) * (((LR*LR) * 0.0001) * ((FOV*0.0125) * 0.0571)) * ts`,
///   `dA = (((UD*UD) * 4.44e-5) * ((FOV*0.0125) * 1/14)) * ts * sign(UD)` (look-around stick);
///   beta += dB, alpha += dA, `ClipBeta`, alpha clamped to [-1.5620697, 1.0471976];
/// - front from alpha / beta, `GetVectorsReadyForRW`; the target entity's rotations = `atan(front) - pi/2`;
/// - heat seeking: `PickTargetForHeatSeekingMissile(source, front, 1.2, player, false, last)`; the lock
///   bit (CPlayerData+0xA0 bit 31) is refreshed by a line-of-sight test (ped and target collision bits
///   cleared around it) when `now - (low31 == 0) > 1000`, the low 31 bits then become `1 < now`; the lock
///   time +0x94, `MarkTarget`, the cross-hair colour (green after 1.5 s locked) and +0x98 = target.
#[hook(addr = 0x511B50, conv = thiscall, name = "CCam::Process_Rocket", expect = "83 EC 28 53 55 56 8B F1", this = "CCam")]
pub fn process_rocket(this: GPtr<u8>, _target: GPtr<u8>, _o: u32, _sv: u32, _svw: u32, heat_seeking: u32) {
    let mut fr = [0u32; 0x14];
    let b = GPtr::<u32>::from_raw(fr.as_mut_ptr()).cast::<u8>();
    wr::<u32>(b, 0x4C, heat_seeking);
    let mut fpu = Fpu::new();
    let f = &mut fpu;
    let t = rd::<GPtr<u8>>(this, 0x21C);
    if rd::<u8>(t, 0x36) & 7 != 3 {
        set_near_clip_raw(SCENE_CAMERA.get(), g::<u32>(ROCKET_NEAR_CLIP).read());
        return;
    }
    wr::<u32>(this, 0xB4, 0x428C_0000);
    if rd::<u8>(this, 0xA) != 0 {
        if g::<u8>(USE_MOUSE_3RD_PERSON).read() != 0 && rd::<GPtr<u8>>(t, 0x71C).is_null() {
            wr::<u32>(this, 0xA8, rd::<u32>(this, 0xBC));
        } else {
            f.fld(rd(t, 0x558));
            wr::<u32>(this, 0xAC, 0);
            f.fsub_m(k(K_HALF_PI));
            wr::<f32>(this, 0xBC, f.fstp());
            wr::<u32>(this, 0xA8, rd::<u32>(this, 0xBC));
        }
        wr::<u8>(this, 0xA, 0);
        g::<u8>(S_B70000).write(0);
        g::<u32>(S_B6FFFC).write(0);
        g::<u32>(S_B6FFF8).write(0);
        wr::<u8>(this, 0x3, 1);
    }
    let rw = rd::<GPtr<u8>>(t, 0x18);
    if !rw.is_null() {
        let rwm = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let m = rd::<GPtr<u8>>(t, 0x14);
        if !m.is_null() {
            matrix_update_rw_matrix(m, rwm);
        } else {
            simple_transform_update_rw_matrix(t.byte_add(4), rwm);
        }
    }
    entity_update_rw_frame(rd::<GPtr<u8>>(this, 0x21C));
    entity_update_rp_h_anim(rd::<GPtr<u8>>(this, 0x21C));
    ped_get_bone_position(rd::<GPtr<u8>>(this, 0x21C), b.byte_add(0x2C), 5, 1);
    let src = this.byte_add(0x19C);
    wr(b, 0x28, src);
    copy3(src, b.byte_add(0x2C));
    f.fld(rd(this, 0x1A4));
    f.fadd_m(k(K_0_1));
    wr::<f32>(this, 0x1A4, f.fstp());
    let _ = get_pad(0);
    wr::<u32>(b, 0x1C, g::<u32>(MOUSE_X).read());
    f.fld(rd(b, 0x1C));
    f.fcomp_m(k(K_0));
    wr::<u32>(b, 0x20, g::<u32>(MOUSE_Y).read());
    let mouse = f.jp(0x44) || {
        f.fld(rd(b, 0x20));
        f.fcomp_m(k(K_0));
        f.jp(0x44)
    };
    if mouse {
        f.fld(rd(b, 0x1C));
        f.fmul_m(k(K_M3));
        f.fld(rd(b, 0x20));
        f.fmul_m(k(K_4));
        wr::<f32>(b, 0x10, f.fstp());
        f.fld(rd(this, 0xB4));
        f.fmul_m(k(K_0_0125));
        wr::<f32>(b, 0x14, f.fst());
        f.fmul_m(d(MOUSE_ACCEL_HORZNTL));
        f.fmulp(1);
        f.fld(rd(b, 0x14));
        f.fmul_m(d(MOUSE_ACCEL_VERTICAL));
        f.fmul_m(rd(b, 0x10));
    } else {
        let ped = rd::<GPtr<u8>>(this, 0x21C);
        let lr = i32::from(pad_look_around_left_right(get_pad(0), ped)).wrapping_neg();
        wr::<i32>(b, 0x1C, lr);
        let ped = rd::<GPtr<u8>>(this, 0x21C);
        f.fild(rd(b, 0x1C));
        wr::<f32>(b, 0x18, f.fstp());
        let ud = i32::from(pad_look_around_up_down(get_pad(0), ped));
        wr::<i32>(b, 0x1C, ud);
        f.fild(rd(b, 0x1C));
        wr::<u32>(b, 0x1C, K_1);
        wr::<f32>(b, 0x10, f.fstp());
        f.fld(k(K_1));
        f.fld(rd(b, 0x18));
        f.fcomp_m(k(K_0));
        if !f.jp(0x5) {
            f.fstp_st(0);
            f.fld(k(K_M1));
        }
        f.fld(rd(b, 0x10));
        f.fcomp_m(k(K_0));
        if !f.jp(0x5) {
            wr::<u32>(b, 0x1C, K_M1);
        }
        f.fld(rd(this, 0xB4));
        f.fmul_m(k(K_0_0125));
        wr::<f32>(b, 0x14, f.fstp());
        f.fld(rd(b, 0x18));
        f.fmul_m(rd(b, 0x18));
        f.fmul_m(k(K_0_0001));
        f.fld(rd(b, 0x14));
        f.fmul_m(k(K_LR_SCALE));
        f.fmulp(1);
        f.fmul_m(d(TIME_STEP));
        f.fmulp(1);
        f.fld(rd(b, 0x10));
        f.fmul_m(rd(b, 0x10));
        f.fmul_m(k(K_UD_SCALE));
        f.fld(rd(b, 0x14));
        f.fmul_m(k(K_1_14));
        f.fmulp(1);
        f.fmul_m(d(TIME_STEP));
        f.fmul_m(rd(b, 0x1C));
    }
    // 0x511D94: [dA, dB] -> beta += dB, alpha += dA
    f.fxch(1);
    f.fadd_m(rd(this, 0xBC));
    wr::<f32>(this, 0xBC, f.fstp());
    f.fadd_m(rd(this, 0xAC));
    wr::<f32>(this, 0xAC, f.fstp());
    clip_beta(this);
    f.fld(rd(this, 0xAC));
    f.fcomp_m(k(K_ALPHA_MAX));
    if !f.jne(0x41) {
        wr::<u32>(this, 0xAC, K_ALPHA_MAX);
    } else {
        f.fld(rd(this, 0xAC));
        f.fcomp_m(k(K_ALPHA_MIN));
        if !f.jp(0x5) {
            wr::<u32>(this, 0xAC, K_ALPHA_MIN);
        }
    }
    // 0x511E31: front from alpha / beta
    let front = this.byte_add(0x190);
    f.fld(rd(this, 0xAC));
    fcos(f);
    f.fld(rd(this, 0xBC));
    fcos(f);
    f.fmul_st(1);
    f.fchs();
    wr::<f32>(b, 0x1C, f.fstp());
    wr::<u32>(front, 0, rd::<u32>(b, 0x1C));
    f.fld(rd(this, 0xBC));
    fsin(f);
    f.fmul_st(1);
    f.fchs();
    wr::<f32>(b, 0x20, f.fstp());
    wr::<u32>(front, 4, rd::<u32>(b, 0x20));
    f.fstp_st(0);
    f.fld(rd(this, 0xAC));
    fsin(f);
    wr::<f32>(b, 0x24, f.fstp());
    wr::<u32>(front, 8, rd::<u32>(b, 0x24));
    get_vectors_ready_for_rw(this);
    push_st0(
        f,
        get_atan_of_xy_bits(rd::<u32>(front, 0), rd::<u32>(this, 0x194)),
    );
    f.fsub_m(k(K_HALF_PI));
    let te = g::<GPtr<u8>>(CAMERA_TARGET).read();
    let hs = rd::<u8>(b, 0x4C);
    f.fld_st(0);
    wr::<f32>(te, 0x558, f.fstp());
    let te = g::<GPtr<u8>>(CAMERA_TARGET).read();
    wr::<f32>(te, 0x55C, f.fstp());
    if hs != 0 {
        heat_seeking_lock(b, front);
    }
    set_near_clip_raw(SCENE_CAMERA.get(), g::<u32>(ROCKET_NEAR_CLIP).read());
}

/// 0x511EC8: the heat-seeking target, its lock bit and the cross-hair.
fn heat_seeking_lock(b: GPtr<u8>, front: GPtr<u8>) {
    let ped = find_player_ped(-1);
    let pd = rd::<GPtr<u8>>(ped, 0x480);
    if rd::<u32>(pd, 0x94) == 0 {
        wr::<u32>(pd, 0x94, g::<u32>(TIME_MS).read());
    }
    let last = rd::<GPtr<u8>>(pd, 0x98);
    let src = rd::<GPtr<u8>>(b, 0x28);
    let tgt = pick_target_for_heat_seeking_missile(
        rd::<u32>(src, 0),
        rd::<u32>(src, 4),
        rd::<u32>(src, 8),
        rd::<u32>(front, 0),
        rd::<u32>(front, 4),
        rd::<u32>(front, 8),
        0x3F99_999A,
        ped,
        0,
        last,
    );
    let mut now = g::<u32>(TIME_MS).read();
    let locked = if !tgt.is_null() && {
        let a0 = rd::<u32>(pd, 0xA0);
        let low_zero = u32::from(a0 & 0x7FFF_FFFF == 0);
        now.wrapping_sub(low_zero) > 1000
    } {
        let a0 = rd::<u32>(pd, 0xA0);
        let c = u32::from(1 < now);
        wr::<u32>(pd, 0xA0, ((c ^ a0) & 0x7FFF_FFFF) ^ a0);
        let tbit = rd::<u8>(tgt, 0x1C) & 1;
        wr::<u8>(b, 0x4C, tbit);
        let pf = rd::<u32>(ped, 0x1C);
        let pbit = (pf & 0xFF) as u8 & 1;
        wr::<u32>(ped, 0x1C, pf & 0xFFFF_FFFE);
        let tf = rd::<u32>(tgt, 0x1C) & 0xFFFF_FFFE;
        let tpos = entity_pos(tgt);
        wr::<u32>(tgt, 0x1C, tf);
        let ppos = entity_pos(ped);
        let clear = get_is_line_of_sight_clear(ppos, tpos, 1, 1, 0, 1, 0, 1, 0);
        let pf = rd::<u32>(ped, 0x1C);
        wr::<u32>(ped, 0x1C, pf ^ ((u32::from(pbit) ^ pf) & 1));
        let tf = rd::<u32>(tgt, 0x1C);
        let saved = u32::from(rd::<u8>(b, 0x4C));
        wr::<u32>(tgt, 0x1C, tf ^ ((saved ^ tf) & 1));
        let a0 = rd::<u32>(pd, 0xA0);
        wr::<u32>(pd, 0xA0, (u32::from(clear) << 31) | (a0 & 0x7FFF_FFFF));
        now = g::<u32>(TIME_MS).read();
        clear
    } else {
        ((rd::<u32>(pd, 0xA0) >> 31) & 1) as u8
    };
    let tgt = if locked == 0 { GPtr::NULL } else { tgt };
    if tgt.is_null() || tgt != rd::<GPtr<u8>>(pd, 0x98) {
        wr::<u32>(pd, 0x94, now);
        now = g::<u32>(TIME_MS).read();
    }
    if !tgt.is_null() {
        let p = entity_pos(tgt);
        weapon_effects_mark_target(
            0,
            rd::<u32>(p, 0),
            rd::<u32>(p, 4),
            rd::<u32>(p, 8),
            0xFF,
            0xFF,
            0xFF,
            0x64,
            0x3FA6_6666,
            1,
        );
        now = g::<u32>(TIME_MS).read();
    }
    let dt = now.wrapping_sub(rd::<u32>(pd, 0x94));
    g::<u32>(XHAIR_0).write(0);
    g::<u8>(XHAIR_R).write(0xFF);
    if dt > 0x5DC {
        g::<u8>(XHAIR_G).write(0);
        g::<u8>(XHAIR_B).write(0);
        g::<u32>(XHAIR_SIZE).write(0x3F80_0000);
    } else {
        g::<u8>(XHAIR_G).write(0xFF);
        g::<u8>(XHAIR_B).write(0xFF);
        g::<u32>(XHAIR_SIZE).write(0);
    }
    wr(pd, 0x98, tgt);
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_ROCKET];

/// Globals of this module (none: the statics are accessed by address).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
