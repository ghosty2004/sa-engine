//! The weapon pickup a ped killed by a car drops (PEDHIT): `CPickups::CreatePickupCoorsCloseToCoors` (where it
//! lands; gta-reversed only wraps the original) and `CPickups::GenerateNewOne_WeaponType`. Ported from the 1.0 US
//! bytes.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::mem::GPtr;
use sa_core::orig::at_site;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{f32_of, x, xi};

orig! {
    /// `CWorld::FindGroundZFor3DCoord(float x, float y, float z, bool* found, CEntity** entity)` (0x5696C0, ST0)
    fn find_ground_z_for_3d_coord(px: f32, py: f32, pz: f32, found: GPtr<u8>, entity: GPtr<u8>) -> f64 = cdecl 0x5696C0 => unported;
    /// `FindPlayerCoors(CVector* out, int32 playerId)` (0x56E010); returns `out`
    fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => unported;
    /// `CPickups::TestForPickupsInBubble(CVector, float)` (0x456450)
    fn test_for_pickups_in_bubble(px: f32, py: f32, pz: f32, radius: f32) -> u8 = cdecl 0x456450 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool buildings, bool vehicles, bool peds,
    /// bool objects, bool dummies, bool seeThrough, bool cameraIgnore)` (0x56A490)
    #[allow(clippy::too_many_arguments)]
    fn get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bd: u32, v: u32, p: u32, o: u32, d: u32, s: u32, i: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CWorld::TestSphereAgainstWorld(CVector, float, CEntity* ignore, bool buildings, bool vehicles, bool peds,
    /// bool objects, bool dummies, bool cameraIgnore)` (0x569E20)
    #[allow(clippy::too_many_arguments)]
    fn test_sphere_against_world(cx: f32, cy: f32, cz: f32, r: f32, ignore: u32, b: u32, v: u32, p: u32, o: u32, d: u32, cam: u32) -> GPtr<u8> = cdecl 0x569E20 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn get_weapon_info(wt: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CPickups::GenerateNewOne(CVector, uint32 model, uint8 type, uint32 ammo, uint32 money, bool, const char*)`
    /// (0x456F20)
    #[allow(clippy::too_many_arguments)]
    fn generate_new_one(px: u32, py: u32, pz: u32, model: u32, ty: u32, ammo: u32, money: u32, empty: u32, msg: u32) -> i32 = cdecl 0x456F20 => unported;
}

/// `CPickups::GenerateNewOne_WeaponType(CVector coors, eWeaponType, ePickupType, uint32 ammo, bool isEmpty,
/// const char* message)` (stub 0x457380 → `.HOODLUM` 0x156DF10, cdecl): `GenerateNewOne(coors,
/// GetWeaponInfo(weapon, 1)->m_nModelId1 (+0xC), type, ammo, 0, isEmpty, message)`, its handle returned.
#[hook(addr = 0x457380, conv = cdecl, name = "CPickups::GenerateNewOne_WeaponType", expect = "E9 8B 6B 11 01",
       unverified = "pedhit-1", route = "L2", hazards = "heap-writes", recs = "weapon-combat")]
#[allow(clippy::too_many_arguments)]
pub fn pickups_generate_new_one_weapon_type(px: u32, py: u32, pz: u32, weapon: u32, ty: u32, ammo: u32, empty: u32, msg: u32) -> i32 {
    let model = super::common::rd::<u32>(get_weapon_info(weapon, 1), 0xC);
    generate_new_one(px, py, pz, model, ty, ammo, 0, empty, msg)
}

/// A local `CVector` passed by address.
fn vp(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

/// `CPickups::CreatePickupCoorsCloseToCoors(float inX, float inY, float inZ, float* outX, float* outY, float* outZ)`
/// (0x458A80, cdecl). Up to 32 tries `i`: an angle `a = (rand() & 0xFF) * 0.024543693f`, the candidate
/// `(inX + sin a * 1.5, inY + cos a * 1.5, ground z + 0.5)` (`FindGroundZFor3DCoord` at inZ; no ground: next try).
/// From `p0 = (inX, inY, inZ + 0.3)` the direction to the candidate is stretched to `|d| + 0.4` (`q = p0 + d * k`,
/// `k = (|d| + 0.4) / |d|`). A candidate within 2.0 (2D) of the player (two `FindPlayerCoors` calls per axis, as
/// the bytes) is skipped while `i <= 16`; one farther away also needs no pickup within 1.3 while `i <= 16`. Then the
/// line `q -> p0` must be clear (buildings, and vehicles and objects while `i < 16`), and while `i <= 16` no
/// vehicle within 1.2 of the candidate: the candidate is the result. After 32 tries: `(inX, inY, inZ + 0.4)`.
#[hook(addr = 0x458A80, conv = cdecl, name = "CPickups::CreatePickupCoorsCloseToCoors-",
       expect = "81 EC A8 00 00 00 8B 84 24 AC 00 00 00",
       unverified = "pedhit-1", route = "L2", hazards = "x87-transcendental", recs = "weapon-combat",
       note = "FSIN / FCOS / FSQRT results (sa_math::x87 models)")]
pub fn pickups_create_pickup_coors_close_to_coors(in_x: f32, in_y: f32, in_z: f32, out_x: GPtr<f32>, out_y: GPtr<f32>, out_z: GPtr<f32>) {
    for i in 0..32i32 {
        let r = at_site(0x458ACE, rand) & 0xFF;
        let a = xi(r).fmul_m(0.024_543_693);
        let cx = f32_of(a.fsin().fmul_m(1.5).fadd_m(in_x));
        let cy = f32_of(a.fcos().fmul_m(1.5).fadd_m(in_y));
        let mut found = 0u8;
        let gz = X87::from_f64_exact(find_ground_z_for_3d_coord(cx, cy, in_z, GPtr::from_raw(&mut found as *mut u8), GPtr::NULL));
        let cz = f32_of(gz.fadd_m(0.5));
        if found == 0 {
            continue;
        }
        let mut p0 = [in_x, in_y, f32_of(x(in_z).fadd_m(0.3))];
        let d = [f32_of(x(cx).fsub_m(p0[0])), f32_of(x(cy).fsub_m(p0[1])), f32_of(x(cz).fsub_m(p0[2]))];
        let sq = |d: &[f32; 3]| x(d[2]).fmul_m(d[2]).fadd(x(d[1]).fmul_m(d[1])).fadd(x(d[0]).fmul_m(d[0]));
        let len1 = sq(&d).fsqrt().fadd_m(0.4);
        let k = len1.fdiv(sq(&d).fsqrt());
        let mut q = [
            f32_of(x(d[0]).fmul(k).fadd_m(p0[0])),
            f32_of(x(f32_of(x(d[1]).fmul(k))).fadd_m(p0[1])),
            f32_of(x(f32_of(x(d[2]).fmul(k))).fadd_m(p0[2])),
        ];
        let (mut a1, mut a2, mut a3, mut a4) = ([0f32; 3], [0f32; 3], [0f32; 3], [0f32; 3]);
        let pa = find_player_coors(GPtr::from_raw(a1.as_mut_ptr()), -1);
        let pb = find_player_coors(GPtr::from_raw(a2.as_mut_ptr()), -1);
        let pc = find_player_coors(GPtr::from_raw(a3.as_mut_ptr()), -1);
        let dy1 = f32_of(x(cy).fsub_m(pc.wrapping_index(1).read()));
        let pd = find_player_coors(GPtr::from_raw(a4.as_mut_ptr()), -1);
        let yy = x(cy).fsub_m(pd.wrapping_index(1).read()).fmul_m(dy1);
        let xx = x(cx).fsub_m(pb.read()).fmul(x(cx).fsub_m(pa.read()));
        let near = yy.fadd(xx).fsqrt().fcom(x(2.0)).test_any(0x41);
        if near {
            if i <= 16 {
                continue;
            }
        } else if i <= 16 && test_for_pickups_in_bubble(cx, cy, cz, 1.3) != 0 {
            continue;
        }
        let lo = u32::from(i < 16);
        if get_is_line_of_sight_clear(vp(&mut q), vp(&mut p0), 1, lo, 0, lo, 0, 0, 0) == 0 {
            continue;
        }
        if i <= 16 && !test_sphere_against_world(cx, cy, cz, 1.2, 0, 0, 1, 0, 0, 0, 0).is_null() {
            continue;
        }
        out_x.write(cx);
        out_y.write(cy);
        out_z.write(cz);
        return;
    }
    out_x.write(in_x);
    out_y.write(in_y);
    out_z.write(f32_of(x(in_z).fadd_m(0.4)));
}

/// This module's registrations.
pub const HOOKS: &[HookReg] =
    &[HOOK_PICKUPS_CREATE_PICKUP_COORS_CLOSE_TO_COORS, HOOK_PICKUPS_GENERATE_NEW_ONE_WEAPON_TYPE];
