//! PORT-VEH-4 (batch `veh4-3`): `CPlane::VehicleDamage`. Ported from the 1.0 US bytes; gta-reversed as a
//! reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use crate::common::{f32_of, g, jne_mask, rd, wr, x};

orig! {
    /// `CAutomobile::VehicleDamage(float, eVehicleCollisionComponent, CEntity*, CVector*, CVector*, eWeaponType)` (0x6A7650)
    fn automobile_vehicle_damage(v: GPtr<u8>, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32) = thiscall 0x6A7650 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(n: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(n: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CVehicle::CanVehicleBeDamaged(CEntity*, eWeaponType, bool*)` (0x6D1280, AL)
    fn can_vehicle_be_damaged(v: GPtr<u8>, e: u32, weapon: u32, out: GPtr<u8>) -> u8 = thiscall 0x6D1280 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, the result through `out`)
    fn vec_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `Multiply3x3(const CVector&, const CMatrix&)` (0x59C810, the transposed product through `out`)
    fn multiply_3x3_t(out: GPtr<u8>, v: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C810 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0): ST0
    fn magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CDamageManager::ProgressAeroplaneDamage(uint8)` (0x6C2460, AL)
    fn progress_aeroplane_damage(d: GPtr<u8>, node: u32) -> u8 = thiscall 0x6C2460 => unported;
    /// `CDamageManager::GetAeroplaneCompStatus(int32)` (0x6C2300)
    fn get_aeroplane_comp_status(d: GPtr<u8>, node: u32) -> u32 = thiscall 0x6C2300 => unported;
    /// `CBouncingPanel::SetPanel(int16 frame, int16 axis, float limit)` (0x6F4920)
    fn bouncing_panel_set_panel(p: GPtr<u8>, frame: u32, axis: u32, limit: u32) = thiscall 0x6F4920 => unported;
    /// `CVehicle::SetComponentVisibility(RwFrame*, uint32)` (0x6D2700)
    fn set_component_visibility(v: GPtr<u8>, frame: GPtr<u8>, state: u32) = thiscall 0x6D2700 => unported;
    /// `CAutomobile::dmgDrawCarCollidingParticles(const CVector&, float, eWeaponType)` (0x6A6DC0)
    fn dmg_draw_car_colliding_particles(v: GPtr<u8>, coors: u32, intensity: f32, weapon: u32) = thiscall 0x6A6DC0 => unported;
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `CEntity::GetModellingMatrix()` (0x46A2D0)
    fn get_modelling_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x46A2D0 => unported;
    /// `FxManager_c::CreateFxSystem(const char*, RwV3d*, RwMatrix*, bool)` (0x4A9BE0) on `g_fxMan` (0xA9AE80)
    fn create_fx_system(mgr: GPtr<u8>, name: u32, pos: GPtr<u8>, m: GPtr<u8>, ignore_bbox: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    fn fx_play(fx: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `FxSystem_c::SetVelAdd(RwV3d*)` (0x4AA730)
    fn fx_set_vel_add(fx: GPtr<u8>, v: GPtr<u8>) = thiscall 0x4AA730 => unported;
    /// `FxSystem_c::SetLocalParticles(bool)` (0x4AA910)
    fn fx_set_local_particles(fx: GPtr<u8>, b: u32) = thiscall 0x4AA910 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_in_range_i(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
    /// virtual `CVehicle::BlowUpCar(CEntity*, bool)` (vtable +0xA4)
    fn v_blow_up_car(v: GPtr<u8>, damager: GPtr<u8>, hide: u32) = thiscall vslot 0xA4 => unported;
}

/// A stack buffer of `N` dwords as a game pointer.
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// `CPlane::VehicleDamage(float intensity, eVehicleCollisionComponent, CEntity* damager, CVector*
/// coors, CVector* direction, eWeaponType weapon)` (0x6CC4B0).
///
/// The Vortex (539) takes the automobile damage. Otherwise only damageable planes (+0x42A bit 5), and
/// a moving plane flagged +0x40 bit 22 only from the player's ped/vehicle. Collisions (intensity 0):
/// the impulse (+0xD8) is the intensity, at the last collision point (+0xEC); the blow-up thresholds are
/// `max(mass / 10000, 1) * 500` and `mass / 10000 * 5000`; nothing for a non-positive impulse, a
/// flagged (+0x40 bit 20) plane or a ped damager. Weapons: only 19/20/21/51 (intensity 0 then) and only
/// when `CanVehicleBeDamaged`; the scale is 0.333. Health (+0x4C0) loses `intensity * handling +0xC8`
/// times 1/6 (player's plane, +0x429 bit 7) or 0.5, 1/12 (+0x429 bit 7), 2/3 (hit by the player's
/// vehicle) or 0.25, floored at 1 when it was positive. Above the second threshold, or at 1 or less
/// health from positive, it blows up; above the first threshold (or a weapon hit) a non-wrecked plane
/// heavier than 1000 damages the nearest of the nodes 12..24 to the hit point (node 16 may become 17/18
/// at random): a progressed aeroplane component swings as a bouncing panel (nodes 16..22, the first
/// free of four panels, limit -0.02) or is hidden when status 2; collision particles are drawn
/// (`scale * intensity`), and a smoke fx (0x85AA80) is (re)started at the node, moving with `-5 *
/// moveSpeed`, for 2..4 s (+0x9FC).
///
/// Kept as in the original: the zero-intensity path's scale is the 1.0 its blow-up threshold slot was
/// overwritten with.
#[hook(addr = 0x6CC4B0, conv = thiscall, name = "CPlane::VehicleDamage", this = "CPlane",
       expect = "83 EC 48 A1 E4 33 8D 00",
       unverified = "veh4-3", route = "L2", hazards = "heap-writes",
       note = "rand at_site 0x6CC85A/0x6CC86D (node 16); may blow the plane up, damages components, starts a smoke FxSystem; the original keeps several values in its argument slots")]
#[allow(clippy::too_many_arguments)]
pub fn plane_vehicle_damage(this: GPtr<u8>, intensity: f32, comp: u32, damager: u32, coors: u32, dir: u32, weapon: u32) {
    if rd::<i16>(this, 0x22) == 0x21B {
        automobile_vehicle_damage(this, intensity.to_bits(), comp, damager, coors, dir, weapon);
        return;
    }
    if rd::<u8>(this, 0x42A) & 0x20 == 0 {
        return;
    }
    if rd::<u8>(this, 0x36) & 0xF8 != 0
        && rd::<u32>(this, 0x40) & 0x40_0000 != 0
        && rd::<GPtr<u8>>(this, 0xDC) != find_player_ped(-1)
        && rd::<GPtr<u8>>(this, 0xDC) != find_player_vehicle(-1, 0)
    {
        return;
    }
    let mut scale = f32::from_bits(0x3EAA_7EFA);
    let mut t1 = g::<f32>(0x8D_33E4).read();
    let mut t2 = g::<f32>(0x8D_33EC).read();
    let amount: f32;
    let mut coors = coors;
    let mut weapon_hit = false;
    if x(intensity).fcom(x(0.0)) == Cmp::Equal {
        let f = x(rd::<f32>(this, 0x8C)).fdiv_m(g::<f32>(0x8D_33E8).read());
        amount = rd::<f32>(this, 0xD8);
        scale = 1.0;
        let m = if !jne_mask(x(1.0), f, 0x41) { x(1.0) } else { f };
        t1 = f32_of(m.fmul_m(t1));
        t2 = f32_of(f.fmul_m(t2));
        coors = (this.addr() as u32).wrapping_add(0xEC);
        if !x(amount).fcom(x(0.0)).test_parity(0x41) || rd::<u32>(this, 0x40) & 0x10_0000 != 0 {
            return;
        }
        let dmg = rd::<GPtr<u8>>(this, 0xDC);
        if !dmg.is_null() && rd::<u8>(dmg, 0x36) & 7 == 3 {
            return;
        }
    } else {
        let mut flag = [0u32; 1];
        if can_vehicle_be_damaged(this, damager, weapon, buf(&mut flag)) == 0 {
            return;
        }
        if !matches!(weapon, 0x13 | 0x14 | 0x15 | 0x33) {
            return;
        }
        weapon_hit = true;
        amount = 0.0;
    }
    let h = rd::<GPtr<u8>>(this, 0x384);
    let dmg = x(amount).fmul_m(rd::<f32>(h, 0xC8));
    let old = rd::<f32>(this, 0x4C0);
    let player_veh = find_player_vehicle(-1, 0);
    let flags = rd::<u8>(this, 0x429);
    let k = if this == player_veh {
        if flags & 0x80 != 0 { f32::from_bits(0x3E2A_AAAB) } else { 0.5 }
    } else if flags & 0x80 != 0 {
        f32::from_bits(0x3DAA_AAAB)
    } else {
        let d = rd::<GPtr<u8>>(this, 0xDC);
        if !d.is_null() && d == find_player_vehicle(-1, 0) { f32::from_bits(0x3F2A_AAAB) } else { 0.25 }
    };
    let dmgf = f32_of(dmg);
    wr::<f32>(this, 0x4C0, f32_of(x(dmgf).fmul_m(k).fsubr_m(rd::<f32>(this, 0x4C0))));
    if !jp(x(rd::<f32>(this, 0x4C0)), x(0.0), 0x41) && !jne_mask(x(old), x(0.0), 0x41) {
        wr::<u32>(this, 0x4C0, 0x3F80_0000);
    }
    let blow = !x(amount).fcom(x(t2)).test_any(0x41)
        || (!jp(x(rd::<f32>(this, 0x4C0)), x(1.0), 0x41) && !x(old).fcom(x(0.0)).test_any(0x41));
    if blow {
        v_blow_up_car(this, this, 0);
        return;
    }
    if x(amount).fcom(x(t1)).test_any(0x41) && !weapon_hit {
        return;
    }
    if rd::<u8>(this, 0x36) & 0xF8 == 0x28 || jne_mask(x(rd::<f32>(this, 0x8C)), x(1000.0), 0x41) {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let pos = if m.is_null() { this.byte_add(4) } else { m.byte_add(0x30) };
    let mut diff = [0u32; 3];
    vec_sub(buf(&mut diff), GPtr::from_addr(coors as usize), pos);
    let mut local = [0u32; 3];
    let r = multiply_3x3_t(buf(&mut local), buf(&mut diff), m);
    let lp = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
    let mut min = 1000.0f32;
    let mut node = -1i32;
    for n in 12..25i32 {
        let fr = rd::<GPtr<u8>>(this, 0x648 + 4 * n as isize);
        if fr.is_null() {
            continue;
        }
        let mut d = [
            f32_of(x(rd::<f32>(fr, 0x40)).fsub_m(lp[0])).to_bits(),
            f32_of(x(rd::<f32>(fr, 0x44)).fsub_m(lp[1])).to_bits(),
            f32_of(x(rd::<f32>(fr, 0x48)).fsub_m(lp[2])).to_bits(),
        ];
        let d2 = X87::from_f64_exact(magnitude_sqr(buf(&mut d)));
        if d2.fcom(x(min).fmul_m(min)) == Cmp::Less {
            min = f32_of(d2.fsqrt());
            node = n;
        }
    }
    if node <= -1 {
        return;
    }
    if node == 0x10 && sa_core::orig::at_site(0x6C_C85A, sa_core::crt::rand) & 1 != 0 {
        if rd::<u32>(this, 0x68C) != 0 && sa_core::orig::at_site(0x6C_C86D, sa_core::crt::rand) & 1 != 0 {
            node = 0x11;
        } else if rd::<u32>(this, 0x690) != 0 {
            node = 0x12;
        }
    }
    let dm = this.byte_add(0x5A0);
    if progress_aeroplane_damage(dm, node as u32) != 0 {
        if get_aeroplane_comp_status(dm, node as u32) == 1 && (0x10..=0x16).contains(&node) {
            for k in 0..4isize {
                let id = rd::<i16>(this, 0x6AC + 0x20 * k);
                if id == -1 {
                    bouncing_panel_set_panel(this.byte_add(0x6AC + 0x20 * k), node as u32, 0, 0xBCA3_D70A);
                    break;
                }
                if i32::from(id) == node {
                    break;
                }
            }
        } else if get_aeroplane_comp_status(dm, node as u32) == 2 {
            set_component_visibility(this, rd::<GPtr<u8>>(this, 0x648 + 4 * node as isize), 2);
        }
    }
    dmg_draw_car_colliding_particles(this, coors, f32_of(x(scale).fmul_m(amount)), weapon);
    let fr = rd::<GPtr<u8>>(this, 0x648 + 4 * node as isize);
    let mut fpos = [rd::<u32>(fr, 0x40), rd::<u32>(fr, 0x44), rd::<u32>(fr, 0x48)];
    let smoke = rd::<GPtr<u8>>(this, 0x9F8);
    if !smoke.is_null() {
        fx_kill(smoke);
        wr::<u32>(this, 0x9F8, 0);
    }
    let mm = get_modelling_matrix(this);
    if mm.is_null() {
        return;
    }
    let fx = create_fx_system(GPtr::from_addr(0xA9_AE80), 0x85_AA80, buf(&mut fpos), mm, 0);
    wr::<GPtr<u8>>(this, 0x9F8, fx);
    if fx.is_null() {
        return;
    }
    fx_play(fx);
    let v = |o: isize| f32_of(x(rd::<f32>(this, o)).fchs().fmul_m(5.0)).to_bits();
    let (vz, vx, vy) = (x(rd::<f32>(this, 0x4C)).fchs(), v(0x44), v(0x48));
    let mut vel = [vx, vy, f32_of(vz.fmul_m(5.0)).to_bits()];
    fx_set_vel_add(rd::<GPtr<u8>>(this, 0x9F8), buf(&mut vel));
    fx_set_local_particles(rd::<GPtr<u8>>(this, 0x9F8), 1);
    wr::<i32>(this, 0x9FC, random_in_range_i(2000, 4000));
}

/// `fcomp` + `fnstsw ax` + `test ah, mask` + `jp` taken.
fn jp(a: X87, b: X87, mask: u8) -> bool {
    a.fcom(b).test_parity(mask)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PLANE_VEHICLE_DAMAGE];
