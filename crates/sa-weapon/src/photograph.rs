//! FRONTIER-2 turn 14, lane A (batch `frontier2-a26`): `CWeapon::TakePhotograph`, the camera weapon's shot. Ported
//! from the 1.0 US bytes (`docs/re/specs/WEAPON/CWeapon.deviations.md` and gta-reversed `Weapon.cpp` as reading aids;
//! the bytes win). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::field::rd;
use sa_core::field::gf;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{fx_play_and_kill, g, x, xi, X87};
use crate::weapon::{world_process_line_of_sight, PHOTOGRAPH_HAS_BEEN_TAKEN, TAKE_PHOTO};
use sa_core::bindings::{calc_screen_coors, ped_get_bone_position, vector_normalise};
use sa_world::bindings::{fx_create_system, stats_increment_stat};
use sa_world::world::common::{cam_pos, entity_pos};

orig! {
    /// `CPickups::PictureTaken()` (0x456A70)
    fn picture_taken() = cdecl 0x456A70 => sa_world::world::pickups::picture_taken;
}

/// `g_fxMan` (0xA9AE80), "camflash" (0x872C24); `TheCamera.ActiveCam` (0xB6F081, byte), `Cams[n].Mode` (0xB6F1A8,
/// i16, cams 0x238 apart); the doubled camera vector of the line start (0xB6F9AC); `RsGlobal.maximumWidth` /
/// `maximumHeight` (0xC17044 / 0xC17048).
const FX_MAN: usize = 0xA9_AE80;
const CAMFLASH: usize = 0x87_2C24;
const ACTIVE_CAM: usize = 0xB6_F081;
const CAM_MODE: usize = 0xB6_F1A8;
const CAM_VEC: usize = 0xB6_F9AC;
const SCREEN_W: usize = 0xC1_7044;
const SCREEN_H: usize = 0xC1_7048;
/// 125.0 (0x865038), 0.1 (0x858B1C), 0.9 (0x858C20), 1.5 (0x858CE8).
const K_RANGE: usize = 0x86_5038;
const K_LO: usize = 0x85_8B1C;
const K_HI: usize = 0x85_8C20;
const K_BACK: usize = 0x85_8CE8;


/// `sqrt((dz*dz + dy*dy) + dx*dx)` of `p - TheCamera.pos` below 125 (NaN fails).
fn near_camera(p: GPtr<f32>) -> bool {
    let c = cam_pos();
    let d = |o: isize| x(p.byte_add(o).read()).fsub_m(c.byte_add(o).read());
    let (dx, dy, dz) = (d(0), d(4), d(8));
    let r = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
    !r.fcom(x(gf(K_RANGE))).test_parity(0x05)
}

/// `CSprite::CalcScreenCoors(p, &s, &w, &h, false, true)` succeeds and `0.1 * W < s.x < 0.9 * W`,
/// `0.1 * H < s.y < 0.9 * H` (fild'ed screen size times the f32 constants, x87).
fn in_frame(p: GPtr<f32>) -> bool {
    let mut s = [0f32; 3];
    let mut wh = [0f32; 2];
    let sp = GPtr::from_raw(s.as_mut_ptr());
    let whp = GPtr::from_raw(wh.as_mut_ptr());
    if calc_screen_coors(p, sp, whp, whp.byte_add(4), 0, 1) == 0 {
        return false;
    }
    let inside = |size: usize, v: f32| {
        let n = xi(g::<i32>(size).read());
        !n.fmul_m(gf(K_LO)).fcom(x(v)).test_parity(0x05) && !x(v).fcom(n.fmul_m(gf(K_HI))).test_parity(0x05)
    };
    inside(SCREEN_W, s[0]) && inside(SCREEN_H, s[1])
}

/// The line `TheCamera.pos + 2 * v` (0xB6F9AC, `fadd st0, st0`) to `end` (flags 1,1,1,1,1,1,0,0) is clear or
/// first hits `e` itself.
fn camera_sees(end: &mut [f32; 3], e: GPtr<u8>) -> bool {
    let c = cam_pos();
    let v = |o: isize| {
        let t: X87 = x(g::<f32>(CAM_VEC + o as usize).read());
        t.fadd(t).fadd_m(c.byte_add(o).read()).to_f32()
    };
    let mut start = [v(0), v(4), v(8)];
    let mut cp = [0u32; 11];
    let mut hit = GPtr::<u8>::NULL;
    let r = world_process_line_of_sight(
        GPtr::from_raw(start.as_mut_ptr()).cast(),
        GPtr::from_raw(end.as_mut_ptr()).cast(),
        GPtr::from_raw(cp.as_mut_ptr()).cast(),
        GPtr::from_raw(&mut hit as *mut GPtr<u8>).cast(),
        1, 1, 1, 1, 1, 1, 0, 0,
    );
    r == 0 || hit == e
}

/// One pool from its last slot down (the byte map's bit 7 = free): `visit` for every used slot.
fn each_in_pool(pool_cell: usize, stride: u32, mut visit: impl FnMut(GPtr<u8>)) {
    let pool = g::<GPtr<u8>>(pool_cell).read();
    let mut i = rd::<u32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
            continue;
        }
        let e: GPtr<u8> = GPtr::from_addr(rd::<u32>(pool, 0).wrapping_add(i.wrapping_mul(stride)) as usize);
        if !e.is_null() {
            visit(e);
        }
    }
}

/// `CWeapon::TakePhotograph(CEntity* owner, CVector* point)` (0x73C1F0, thiscall `ret 8`). With `point`: the
/// "camflash" effect there (`CreateFxSystem` + `PlayAndKill`). Returns false unless the active cam's mode is 46
/// (`MODE_CAMERA`); then `CPickups::PictureTaken()`, `bPhotographHasBeenTaken` and `ms_bTakePhoto` set, stat 0xA6
/// + 1. Peds (pool 0xB74490, from the last slot down): within 125 of the camera ([`near_camera`]), the head bone
/// (`GetBonePosition(5, false)`) in the screen's inner 80 % ([`in_frame`]), and the line from the camera to
/// `head + Normalise(cam - head) * 1.5` (each step stored f32) clear or ending on the ped ([`camera_sees`]):
/// `+0x470 |= 0x40000` (bHasBeenPhotographed). Objects (pool 0xB7449C): the same with their position (the
/// original also accepts only objects NEARER than 125; gta-reversed inverts that test), `+0x140 |= 0x1000`.
/// Returns true.
#[hook(addr = 0x73C1F0, conv = thiscall, name = "CWeapon::TakePhotograph", this = "CWeapon",
       expect = "8B 44 24 08 81 EC 8C 00 00 00",
       unverified = "frontier2-a26", route = "L2", hazards = "heap-writes",
       note = "camera weapon: flags photographed peds / objects, PictureTaken, stats")]
pub fn weapon_take_photograph(_this: GPtr<u8>, _owner: GPtr<u8>, point: GPtr<u8>) -> u8 {
    if !point.is_null() {
        let fx = fx_create_system(GPtr::from_addr(FX_MAN), GPtr::from_addr(CAMFLASH), point, GPtr::NULL, 0);
        if !fx.is_null() {
            fx_play_and_kill(fx);
        }
    }
    let n = usize::from(g::<u8>(ACTIVE_CAM).read());
    if g::<i16>(CAM_MODE + n * 0x238).read() != 0x2E {
        return 0;
    }
    picture_taken();
    PHOTOGRAPH_HAS_BEEN_TAKEN.set(1);
    TAKE_PHOTO.set(1);
    stats_increment_stat(0xA6, 1.0);
    each_in_pool(0xB7_4490, 0x7C4, |ped| {
        if !near_camera(entity_pos(ped).cast()) {
            return;
        }
        let mut head = [0f32; 3];
        let hp = GPtr::from_raw(head.as_mut_ptr());
        ped_get_bone_position(ped, hp.cast(), 5, 0);
        if !in_frame(hp) {
            return;
        }
        let c = cam_pos();
        let mut dir = [0f32; 3];
        for (k, d) in dir.iter_mut().enumerate() {
            *d = x(c.byte_add(4 * k as isize).read()).fsub_m(head[k]).to_f32();
        }
        vector_normalise(GPtr::from_raw(&mut dir as *mut [f32; 3]));
        for (h, d) in head.iter_mut().zip(dir) {
            let d = x(d).fmul_m(gf(K_BACK)).to_f32();
            *h = x(*h).fadd_m(d).to_f32();
        }
        if camera_sees(&mut head, ped) {
            let f = rd::<u32>(ped, 0x470) | 0x4_0000;
            sa_core::field::wr::<u32>(ped, 0x470, f);
        }
    });
    each_in_pool(0xB7_449C, 0x19C, |obj| {
        let p = entity_pos(obj).cast::<f32>();
        let mut pos = [p.read(), p.byte_add(4).read(), p.byte_add(8).read()];
        let pp = GPtr::from_raw(pos.as_mut_ptr());
        if !near_camera(pp) || !in_frame(pp) {
            return;
        }
        if camera_sees(&mut pos, obj) {
            let f = rd::<u32>(obj, 0x140) | 0x1000;
            sa_core::field::wr::<u32>(obj, 0x140, f);
        }
    });
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_WEAPON_TAKE_PHOTOGRAPH];
