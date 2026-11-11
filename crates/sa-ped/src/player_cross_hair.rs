//! The player's vehicle-gun crosshair (batch `mvpm-ped-1`): `CPlayerCrossHair::Process` (0x56EC80, name proposed in
//! `docs/re/specs/PED/CPlayerInfo.md`). Ported from the 1.0 US bytes (no gta-reversed code exists). Unverified
//! (docs/verify/HOWTO.md).
//!
//! `CPlayerCrossHair` (CPlayerInfo +0x158): +0 u8 active, +4 x, +8 y (both in [-0.9, 0.9]).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPad::GetSteeringLeftRight()` (0x53FB80), AX
    fn pad_get_steering_left_right(pad: GPtr<u8>) -> i16 = thiscall 0x53FB80 => unported;
    /// `CPad::GetSteeringUpDown()` (0x53FBD0), AX
    fn pad_get_steering_up_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FBD0 => unported;
    /// `CPad::GetCarGunFired()` (0x53FF90), AX tested
    fn pad_get_car_gun_fired(pad: GPtr<u8>) -> i16 = thiscall 0x53FF90 => unported;
    /// `CCamera::CamShakeNoPos(CCamera*, float)` (0x50A970, cdecl)
    fn camera_cam_shake_no_pos(cam: GPtr<u8>, strength: f32) = cdecl 0x50A970 => unported;
    /// `CCamera::FindCamFOV()` (0x50AD20, ST0)
    fn camera_find_cam_fov(cam: GPtr<u8>) -> f64 = thiscall 0x50AD20 => unported;
    /// `CWeapon::CWeapon(eWeaponType, int32 ammo)` (0x73B430, `ret 8`)
    fn weapon_ctor(w: GPtr<u8>, ty: i32, ammo: i32) -> GPtr<u8> = thiscall 0x73B430 => unported;
    /// `CWeapon::FireInstantHit(CEntity* firedBy, CVector* origin, CVector* muzzle, CEntity* target, CVector* targetPos,
    /// CVector* originForDriveBy, bool, bool muzzleFlash)` (0x73FB10, `ret 0x20`)
    fn weapon_fire_instant_hit(w: GPtr<u8>, by: GPtr<u8>, origin: GPtr<u8>, muzzle: GPtr<u8>, target: u32, pos: GPtr<u8>, drive_by: u32, a: u32, b: u32) -> u8 = thiscall 0x73FB10 => unported;
}

/// `CTimer::ms_fTimeStep` (0xB7CB5C)
const TIME_STEP: usize = 0xB7CB5C;
/// 1/3000 (0x865030): the crosshair speed
const K_SPEED: usize = 0x865030;
/// The invert-look byte (0xB73402)
const INVERT_LOOK: usize = 0xB73402;
/// 0.9f (0x858C20) and -0.9f (0x858CAC): the crosshair limits
const K_MAX: usize = 0x858C20;
const K_MIN: usize = 0x858CAC;
/// `TheCamera` (0xB6F028); its camera matrix rows (right 0xB6F99C, forward 0xB6F9AC, up 0xB6F9BC, position 0xB6F9CC);
/// its placement matrix pointer (0xB6F03C) and placement position (0xB6F02C)
const THE_CAMERA: usize = 0xB6F028;
const CAM_RIGHT: usize = 0xB6F99C;
const CAM_FWD: usize = 0xB6F9AC;
const CAM_UP: usize = 0xB6F9BC;
const CAM_POS: usize = 0xB6F9CC;
const CAM_MATRIX: usize = 0xB6F03C;
const CAM_PLACEMENT_POS: usize = 0xB6F02C;
/// FOV degrees to the half-angle in radians (0x8631D4, 0.0087266462)
const K_FOV: usize = 0x8631D4;
/// `CDraw::ms_fAspectRatio` (0xC3EFA4)
const ASPECT: usize = 0xC3EFA4;
/// The ray length (0x858A48)
const K_RANGE: usize = 0x858A48;

fn kf(a: usize) -> f32 {
    g::<f32>(a).read()
}
fn tan(v: X87) -> X87 {
    v.fptan().map_or(v, |(t, _)| t)
}

/// `CPlayerCrossHair::Process(int32 playerIndex, CPad* pad)` (0x56EC80, thiscall, `ret 8`).
///
/// Only while active (+0): `x (+4) = f32(((steerLR * ts) * 1/3000) + x)`; `y (+8)` the same with steerUD added when the
/// invert-look byte is set, else subtracted (`fsubr`). Each of x, y clamped: above 0.9 → 0.9, below -0.9 → -0.9. When
/// `GetCarGunFired()`: `CamShakeNoPos(&TheCamera, 0.2)`; `t = tan(FindCamFOV() * k) / aspect * y` (FPTAN), `U = f32(up *
/// t)`; `s = tan(FindCamFOV() * k)`, `Rx = f32(f32(right.x * x) * s)`, `Ry` alike, `Rz = (x * right.z) * s` (unrounded);
/// `D = (f32(f32(fwd.x - Rx) - U.x), f32(f32(fwd.y - Ry) - U.y), f32(f32(fwd.z - Rz) - U.z))` (fwd.x - Rx kept on the
/// FPU); target `T = (f32(D.x * 200 + pos.x), f32(pos.y + D.y * 200), f32(pos.z + f32(D.z * 200)))`. A stack
/// `CWeapon(31, 5000)` fires `FireInstantHit(ped, &O, &O, NULL, &T, NULL, true, true)` from the camera's placement
/// position `O`, with ped flag +0x470 bit 25 cleared during the call and restored afterwards (ped read again).
#[hook(addr = 0x56ec80, conv = thiscall, name = "FUN_0056ec80", expect = "83 EC 5C 56 8B F1",
       unverified = "mvpm-ped-1", route = "L2", hazards = "x87-transcendental,stack-residue", recs = "driving",
       note = "two FPTANs (sa_math::x87 model); the CWeapon lives on the Rust stack (0x1C bytes, no destructor in the original)")]
pub fn player_cross_hair_process(this: GPtr<u8>, player: i32, pad: GPtr<u8>) {
    if rd::<u8>(this, 0) == 0 {
        return;
    }
    let ts = kf(TIME_STEP);
    let k = kf(K_SPEED);
    let lr = pad_get_steering_left_right(pad);
    wr::<f32>(this, 4, xi(i32::from(lr)).fmul_m(ts).fmul_m(k).fadd_m(rd::<f32>(this, 4)).to_f32());
    let ts = kf(TIME_STEP);
    let ud = pad_get_steering_up_down(pad);
    let d = xi(i32::from(ud)).fmul_m(ts).fmul_m(k);
    let ny = if g::<u8>(INVERT_LOOK).read() != 0 { d.fadd_m(rd::<f32>(this, 8)) } else { d.fsubr_m(rd::<f32>(this, 8)) };
    wr::<f32>(this, 8, ny.to_f32());
    for off in [4isize, 8] {
        if !x(rd::<f32>(this, off)).fcom(x(kf(K_MAX))).test_any(0x41) {
            wr::<u32>(this, off, 0x3F66_6666);
        }
        if !x(rd::<f32>(this, off)).fcom(x(kf(K_MIN))).test_parity(5) {
            wr::<u32>(this, off, 0xBF66_6666);
        }
    }
    if pad_get_car_gun_fired(pad) == 0 {
        return;
    }
    let cam: GPtr<u8> = GPtr::from_addr(THE_CAMERA);
    camera_cam_shake_no_pos(cam, f32::from_bits(0x3E4C_CCCD));
    let a = X87::from_f64_exact(camera_find_cam_fov(cam)).fmul_m(kf(K_FOV));
    let t = tan(a).fdiv_m(kf(ASPECT)).fmul_m(rd::<f32>(this, 8));
    let u = [
        x(kf(CAM_UP)).fmul(t).to_f32(),
        x(kf(CAM_UP + 4)).fmul(t).to_f32(),
        x(kf(CAM_UP + 8)).fmul(t).to_f32(),
    ];
    let a2 = X87::from_f64_exact(camera_find_cam_fov(cam)).fmul_m(kf(K_FOV));
    let cx = x(rd::<f32>(this, 4));
    let r0 = x(kf(CAM_RIGHT)).fmul(cx).to_f32();
    let r1 = x(kf(CAM_RIGHT + 4)).fmul(cx).to_f32();
    let rz = cx.fmul_m(kf(CAM_RIGHT + 8));
    let s = tan(a2);
    let rx = x(r0).fmul(s).to_f32();
    let ry = x(r1).fmul(s).to_f32();
    let rz = rz.fmul(s);
    let ex = x(kf(CAM_FWD)).fsub_m(rx);
    let ey = x(kf(CAM_FWD + 4)).fsub_m(ry).to_f32();
    let ez = x(kf(CAM_FWD + 8)).fsub(rz).to_f32();
    let dx = ex.fsub_m(u[0]).to_f32();
    let dy = x(ey).fsub_m(u[1]).to_f32();
    let dz = x(ez).fsub_m(u[2]).to_f32();
    let kr = kf(K_RANGE);
    let tx = x(dx).fmul_m(kr);
    let ty = x(dy).fmul_m(kr);
    let tz = x(dz).fmul_m(kr).to_f32();
    let mut target = [
        tx.fadd_m(kf(CAM_POS)).to_f32(),
        x(kf(CAM_POS + 4)).fadd(ty).to_f32(),
        x(kf(CAM_POS + 8)).fadd_m(tz).to_f32(),
    ];
    let mut weapon = [0u32; 7];
    let w: GPtr<u8> = GPtr::from_raw(weapon.as_mut_ptr()).cast();
    weapon_ctor(w, 0x1F, 0x1388);
    let m = g::<u32>(CAM_MATRIX).read();
    let src = if m != 0 { GPtr::<u8>::from_addr(m as usize + 0x30) } else { GPtr::from_addr(CAM_PLACEMENT_POS) };
    let mut origin = [rd::<u32>(src, 0), rd::<u32>(src, 4), rd::<u32>(src, 8)];
    let slot = PLAYERS + (player as u32).wrapping_mul(0x190) as usize;
    let ped = g::<GPtr<u8>>(slot).read();
    let old = rd::<u32>(ped, 0x470);
    wr::<u32>(ped, 0x470, old & 0xFDFF_FFFF);
    let o: GPtr<u8> = GPtr::from_raw(origin.as_mut_ptr()).cast();
    let tp: GPtr<u8> = GPtr::from_raw(target.as_mut_ptr()).cast();
    weapon_fire_instant_hit(w, ped, o, o, 0, tp, 0, 1, 1);
    let ped = g::<GPtr<u8>>(slot).read();
    let f = rd::<u32>(ped, 0x470);
    let bit = ((old >> 25) & 1) << 25;
    wr::<u32>(ped, 0x470, f ^ ((bit ^ f) & 0x200_0000));
}

pub const HOOKS: &[HookReg] = &[HOOK_PLAYER_CROSS_HAIR_PROCESS];
