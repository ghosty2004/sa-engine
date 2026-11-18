//! FRONTIER-2 turn 14, lane A (batch `frontier2-a26`): `CWeapon::FireAreaEffect`, the flamethrower / spraycan / fire
//! extinguisher shot. Ported from the 1.0 US bytes (`docs/re/specs/WEAPON/CWeapon.deviations.md` and gta-reversed
//! `Weapon.cpp` as reading aids; the bytes win). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::field::rd;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::bindings::try_to_start_fire_at_coors;
use crate::common::{g, get_weapon_info, rand, x, xi, X87};
use sa_core::bindings::{
    cam_using_3rd_person_mouse_cam, find_player_ped, general_angle_between_points, ped_get_bone_position,
    vector_normalise,
};
use sa_world::bindings::{camera_find_3rd_person_cam_target_vector, report_crime, stats_increment_stat, vector_div};
use sa_world::world::common::{entity_pos, placeable_heading};

orig! {
    /// `CShotInfo::AddShot(CEntity* creator, eWeaponType, CVector origin, CVector target)` (0x739C30)
    #[allow(clippy::too_many_arguments)]
    fn add_shot(creator: GPtr<u8>, ty: i32, ox: f32, oy: f32, oz: f32, tx: f32, ty_: f32, tz: f32) -> u8
        = cdecl 0x739C30 => crate::bullet_shot::add_shot;
    /// `CWeapon::DoWeaponEffect(CVector origin, CVector target)` (0x73E690)
    #[allow(clippy::too_many_arguments)]
    fn do_weapon_effect(this: GPtr<u8>, ox: f32, oy: f32, oz: f32, tx: f32, ty: f32, tz: f32)
        = thiscall 0x73E690 => crate::weapon::do_weapon_effect;
}

/// `TheCamera` (0xB6F028), `Cams[0]` (0xB6F19C); 57.295776 (0x859878), 0.017453216 (0x872C60, not pi / 180),
/// 1 / 32767 (0x858C7C), 2.5 (0x858FA0), 3.5 (0x859028), 0.5 (0x858B8C).
const THE_CAMERA: usize = 0xB6_F028;
const CAM0: usize = 0xB6_F19C;
const K_DEG: usize = 0x85_9878;
const K_RAD: usize = 0x87_2C60;
const K_RAND: usize = 0x85_8C7C;
const K_2_5: usize = 0x85_8FA0;
const K_3_5: usize = 0x85_9028;
const K_0_5: usize = 0x85_8B8C;

fn kf(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn v3(p: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [x(a[0]).fsub_m(b[0]).to_f32(), x(a[1]).fsub_m(b[1]).to_f32(), x(a[2]).fsub_m(b[2]).to_f32()]
}

/// `CWeapon::FireAreaEffect(CEntity* firer, CVector* origin, CEntity* targetEntity, CVector* target)` (0x73E800,
/// thiscall `ret 0x10`), always true. `O = *origin`. With a target (the point, else the target entity's position)
/// `GetAngleBetweenPoints(t.x, t.y, O.x, O.y)` is computed and dropped; then `T` = the point, else the entity's bone
/// 3 (`GetBonePosition(3, false)`) for a ped or its position, and `D = Normalise(T - O)`. Without either: the player
/// with the mouse cam (`Cams[0].Using3rdPersonMouseCam()`) aims along `Find3rdPersonCamTargetVector(range (+8), O,
/// &S, &T)`, `D = (T - S) / range`; anyone else along the heading in degrees (`* 57.295776`, stored f32) times
/// 0.017453216: `D = (-sin, cos, 0)`, for a ped with player data `D.z = -tan(pd+0x54)` and `D` normalised (both not
/// in gta-reversed), `T = O + D`. Then `CShotInfo::AddShot(firer, type, O, T)`, `DoWeaponEffect(O, D)`; the
/// flamethrower (37) on `rand() & 3 == 2` tries a creeping fire at `O + D * (rand() / 32767 * 2.5 + 3.5)` raised by
/// 0.5 (`TryToStartFireAtCoors(.., 0, true, false, 2.3)`, stat 0xD8 + 1 when it starts); `ReportCrime(1, null,
/// firer)`.
#[hook(addr = 0x73E800, conv = thiscall, name = "CWeapon::FireAreaEffect", this = "CWeapon",
       expect = "83 EC 40 53 55 56 57 8B E9",
       unverified = "frontier2-a26", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "FSIN/FCOS/FPTAN aim; shot / fire pools; rand sites 0x73EB61, 0x73EB70")]
pub fn fire_area_effect(this: GPtr<u8>, firer: GPtr<u8>, origin: GPtr<u8>, target_ent: GPtr<u8>, target: GPtr<u8>) -> u8 {
    let ty = rd::<i32>(this, 0);
    let wi = get_weapon_info(ty, 1);
    let o = v3(origin);
    let mut d: [f32; 3];
    let t: [f32; 3];
    if !target_ent.is_null() || !target.is_null() {
        let a = if !target_ent.is_null() { v3(entity_pos(target_ent)) } else { v3(target) };
        general_angle_between_points(a[0], a[1], o[0], o[1]);
        t = if !target.is_null() {
            v3(target)
        } else if rd::<u8>(target_ent, 0x36) & 7 == 3 {
            let mut b = [0f32; 3];
            ped_get_bone_position(target_ent, GPtr::from_raw(b.as_mut_ptr()).cast(), 3, 0);
            b
        } else {
            v3(entity_pos(target_ent))
        };
        d = sub3(t, o);
        vector_normalise(GPtr::from_raw(&mut d as *mut [f32; 3]));
    } else {
        let deg = placeable_heading(firer).fmul_m(kf(K_DEG)).to_f32();
        if firer == find_player_ped(-1) && cam_using_3rd_person_mouse_cam(GPtr::from_addr(CAM0)) != 0 {
            let range = rd::<f32>(wi, 8);
            let mut s = [0f32; 3];
            let mut tt = [0f32; 3];
            camera_find_3rd_person_cam_target_vector(
                GPtr::from_addr(THE_CAMERA), range, o[0], o[1], o[2],
                GPtr::from_raw(s.as_mut_ptr()).cast(), GPtr::from_raw(tt.as_mut_ptr()).cast(),
            );
            let mut diff = sub3(tt, s);
            let mut q = [0f32; 3];
            let r = vector_div(
                GPtr::from_raw(q.as_mut_ptr()).cast(), GPtr::from_raw(diff.as_mut_ptr()).cast(), rd::<f32>(wi, 8),
            );
            d = v3(r);
            t = tt;
        } else {
            let r: X87 = x(deg).fmul_m(kf(K_RAD));
            d = [r.fsin().fchs().to_f32(), r.fcos().to_f32(), 0.0];
            if rd::<u8>(firer, 0x36) & 7 == 3 {
                let pd = rd::<GPtr<u8>>(firer, 0x480);
                if !pd.is_null() {
                    let a = x(rd::<f32>(pd, 0x54));
                    let tan = a.fptan().map_or(a, |(v, _one)| v);
                    d[2] = tan.fchs().to_f32();
                    vector_normalise(GPtr::from_raw(&mut d as *mut [f32; 3]));
                }
            }
            t = [x(d[0]).fadd_m(o[0]).to_f32(), x(d[1]).fadd_m(o[1]).to_f32(), x(o[2]).fadd_m(d[2]).to_f32()];
        }
    }
    add_shot(firer, ty, o[0], o[1], o[2], t[0], t[1], t[2]);
    do_weapon_effect(this, o[0], o[1], o[2], d[0], d[1], d[2]);
    if rd::<i32>(this, 0) == 0x25 && sa_core::orig::at_site(0x73_EB61, rand) & 3 == 2 {
        let r = sa_core::orig::at_site(0x73_EB70, rand);
        let f = xi(r).fmul_m(kf(K_RAND)).fmul_m(kf(K_2_5)).fadd_m(kf(K_3_5));
        let p: [f32; 3] = core::array::from_fn(|k| x(f.fmul_m(d[k]).to_f32()).fadd_m(o[k]).to_f32());
        let z = x(p[2]).fadd_m(kf(K_0_5)).to_f32();
        if try_to_start_fire_at_coors(p[0], p[1], z, 0, 1, 0, f32::from_bits(0x4013_3333)) != 0 {
            stats_increment_stat(0xD8, 1.0);
        }
    }
    report_crime(1, GPtr::NULL, firer);
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FIRE_AREA_EFFECT];
