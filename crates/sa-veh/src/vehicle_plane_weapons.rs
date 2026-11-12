//! PORT-VEH-4 (batch `veh4-2`): `CVehicle::GetPlaneWeaponFiringStatus`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{f32_of, g, rd, wr, x};

orig! {
    /// `CWorld::FindPlayerSlotWithVehiclePointer(CEntity*)` (0x564000)
    fn find_player_slot_with_vehicle(v: GPtr<u8>) -> i32 = cdecl 0x564000 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::GetCarGunFired()` (0x53FF90): int16 in AX
    fn pad_get_car_gun_fired(pad: GPtr<u8>) -> i16 = thiscall 0x53FF90 => unported;
    /// `CPad::CarGunJustDown()` (0x53FFE0): int16 in AX
    fn pad_car_gun_just_down(pad: GPtr<u8>) -> i16 = thiscall 0x53FFE0 => unported;
    /// `CPad::GetEnterTargeting()` (0x5406B0, AL)
    fn pad_get_enter_targeting(pad: GPtr<u8>) -> u8 = thiscall 0x5406B0 => unported;
    /// `CPad::GetTarget()` (0x540670, AL)
    fn pad_get_target(pad: GPtr<u8>) -> u8 = thiscall 0x540670 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, the result through `out`)
    fn vec_mul_scalar(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00)
    fn vec_add_assign(v: GPtr<u8>, o: GPtr<u8>) = thiscall 0x411A00 => unported;
    /// `CProjectileInfo::AddProjectile(CEntity*, eWeaponType, CVector, float, CVector*, CEntity*)` (0x737C80)
    fn add_projectile(creator: GPtr<u8>, wt: i32, x: f32, y: f32, z: f32, force: u32, dir: GPtr<u8>, target: GPtr<u8>) -> u8 = cdecl 0x737C80 => unported;
    /// `CWeaponEffects::IsLockedOn(int32)` (0x742BD0, AL)
    fn weapon_effects_is_locked_on(n: i32) -> u8 = cdecl 0x742BD0 => unported;
    /// `CVehicle::ScanAndMarkTargetForHeatSeekingMissile(CEntity*)` (0x6E0400)
    fn scan_and_mark_target(v: GPtr<u8>, e: GPtr<u8>) -> GPtr<u8> = thiscall 0x6E0400 => unported;
}

/// `CTimer::m_snTimeInMilliseconds`.
fn now() -> u32 {
    g::<u32>(0xB7_CB84).read()
}

/// The crosshair colour statics of `CWeaponEffects` 0 (0xC8A84C..4E RGB, 0xC8A85C size).
fn crosshair(r: u8, gb: u8, size: u32) {
    g::<u8>(0xC8_A84C).write(r);
    g::<u8>(0xC8_A84D).write(gb);
    g::<u8>(0xC8_A84E).write(gb);
    g::<u32>(0xC8_A85C).write(size);
}

/// `CVehicle::GetPlaneWeaponFiringStatus(bool& isFiring, eOrdnanceType& ordnance)` (0x6E3440), by model
/// (pad of the player slot driving it):
/// - Hunter (425) / 576: gun fired (2) sets `isFiring` and returns; else a just-pressed gun (1) sets
///   ordnance 1; else both are cleared.
/// - 447/464/487/497/548: gun fired (2) sets `isFiring`; for the RC Baron (464), with 0xC1CC00 and
///   0xB6EC2E set and the gun state 1, `isFiring` = 1 too; ordnance 0.
/// - Rustler (476): gun fired (2) sets `isFiring`; ordnance 0.
/// - Hydra (520): ordnance 0; a just-pressed gun (1) more than 2 s after the last one (+0x4D4) fires an
///   unguided rocket (weapon 58) from `pos + up * -2.5 + at * -1` along the forward axis; a held secondary
///   (just-down 2) with a heat-seeker lock (`IsLockedOn(0)`), a target time (+0x9DC) and the scanned target
///   still the marked one (+0x9E0) for more than 1.5 s returns that target with ordnance 2 (red
///   crosshair), else ordnance 1; otherwise the enter-targeting button restarts the lock timer, the
///   target button scans and keeps the target (re-timing when it changes; red after 1.5 s, else white),
///   and neither clears the lock.
/// - Other models: both outputs cleared.
///
/// Kept as in the original: `isFiring` is not cleared on the paths that do not set it (only the
/// default path clears it), and the Hydra reads its matrix's up/at vectors even without a matrix.
#[hook(addr = 0x6E3440, conv = thiscall, name = "CVehicle::GetPlaneWeaponFiringStatus", this = "CVehicle",
       expect = "83 EC 30 53 55 56 8B F1",
       unverified = "veh4-2", route = "L2", hazards = "heap-writes",
       note = "Hydra: AddProjectile (projectile pool) and the crosshair statics 0xC8A83C/0xC8A84C..4E/0xC8A85C; returns the locked target (EAX)")]
pub fn vehicle_get_plane_weapon_firing_status(this: GPtr<u8>, is_firing: GPtr<u8>, ordnance: GPtr<u8>) -> GPtr<u8> {
    let pad = pad_get_pad(find_player_slot_with_vehicle(this));
    let model = i32::from(rd::<i16>(this, 0x22));
    let case = match model {
        425 | 576 => 0,
        447 | 464 | 487 | 497 | 548 => 1,
        476 => 2,
        520 => 3,
        _ => 4,
    };
    match case {
        0 => {
            if pad_get_car_gun_fired(pad) == 2 {
                wr::<u8>(is_firing, 0, 1);
                return GPtr::NULL;
            }
            if pad_car_gun_just_down(pad) == 1 {
                wr::<u32>(ordnance, 0, 1);
                return GPtr::NULL;
            }
        }
        1 => {
            if pad_get_car_gun_fired(pad) == 2 {
                wr::<u8>(is_firing, 0, 1);
            }
            if model == 464
                && g::<u8>(0xC1_CC00).read() != 0
                && g::<u8>(0xB6_EC2E).read() != 0
                && pad_get_car_gun_fired(pad) == 1
            {
                wr::<u8>(is_firing, 0, 1);
            }
            wr::<u32>(ordnance, 0, 0);
            return GPtr::NULL;
        }
        2 => {
            if pad_get_car_gun_fired(pad) == 2 {
                wr::<u8>(is_firing, 0, 1);
            }
            wr::<u32>(ordnance, 0, 0);
            return GPtr::NULL;
        }
        3 => return hydra(this, pad, ordnance),
        _ => {}
    }
    wr::<u8>(is_firing, 0, 0);
    wr::<u32>(ordnance, 0, 0);
    GPtr::NULL
}

/// The Hydra case (0x6E34ED).
fn hydra(this: GPtr<u8>, pad: GPtr<u8>, ordnance: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(ordnance, 0, 0);
    let t0 = now();
    if pad_car_gun_just_down(pad) == 1 && now() > rd::<u32>(this, 0x4D4).wrapping_add(2000) {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let p = if m.is_null() { this.byte_add(4) } else { m.byte_add(0x30) };
        let pos = [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)];
        let mut a = [0u32; 3];
        let mut b = [0u32; 3];
        let ap: GPtr<u8> = GPtr::from_raw(&mut a as *mut [u32; 3]).cast();
        let bp: GPtr<u8> = GPtr::from_raw(&mut b as *mut [u32; 3]).cast();
        let r = vec_mul_scalar(ap, m.byte_add(0x10), -2.5);
        let mut t1 = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
        let t1p: GPtr<u8> = GPtr::from_raw(&mut t1 as *mut [u32; 3]).cast();
        let r2 = vec_mul_scalar(bp, m.byte_add(0x20), -1.0);
        vec_add_assign(t1p, r2);
        let s = |k: usize| f32_of(x(f32::from_bits(t1[k])).fadd_m(pos[k]));
        let (sx, sy, sz) = (s(0), s(1), s(2));
        add_projectile(this, 0x3A, sx, sy, sz, 0, m.byte_add(0x10), GPtr::NULL);
        wr::<u32>(this, 0x4D4, now());
    }
    if pad_car_gun_just_down(pad) == 2 {
        if weapon_effects_is_locked_on(0) != 0 && rd::<u32>(this, 0x9DC) != 0 {
            let t = scan_and_mark_target(this, rd::<GPtr<u8>>(this, 0x9E0));
            if !t.is_null()
                && t == rd::<GPtr<u8>>(this, 0x9E0)
                && t0.wrapping_sub(rd::<u32>(this, 0x9DC)) > 1500
            {
                wr::<u32>(ordnance, 0, 2);
                crosshair(0xFF, 0, 0x3F80_0000);
                g::<u32>(0xC8_A83C).write(0);
                return t;
            }
        }
        wr::<u32>(ordnance, 0, 1);
        return GPtr::NULL;
    }
    if pad_get_enter_targeting(pad) != 0 {
        wr::<u32>(this, 0x9DC, t0);
        wr::<u32>(this, 0x9E0, 0);
        return GPtr::NULL;
    }
    if pad_get_target(pad) == 0 {
        wr::<u32>(this, 0x9DC, 0);
        wr::<u32>(this, 0x9E0, 0);
        return GPtr::NULL;
    }
    if rd::<u32>(this, 0x9DC) == 0 {
        wr::<u32>(this, 0x9DC, t0);
    }
    let t = scan_and_mark_target(this, rd::<GPtr<u8>>(this, 0x9E0));
    if t.is_null() || t != rd::<GPtr<u8>>(this, 0x9E0) {
        wr::<u32>(this, 0x9DC, t0);
    }
    let d = t0.wrapping_sub(rd::<u32>(this, 0x9DC));
    g::<u32>(0xC8_A83C).write(0);
    if d > 1500 {
        crosshair(0xFF, 0, 0x3F80_0000);
    } else {
        crosshair(0xFF, 0xFF, 0);
    }
    wr::<GPtr<u8>>(this, 0x9E0, t);
    GPtr::NULL
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_GET_PLANE_WEAPON_FIRING_STATUS];
