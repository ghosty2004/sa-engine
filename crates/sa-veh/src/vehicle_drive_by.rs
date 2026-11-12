//! PORT-VEH-4 (batch `veh4-3`): `CVehicle::DoDriveByShootings`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).
//!
//! The body's protection detour (`nop; jmp 0x402348` at 0x741FD7) is the relocated
//! `mov esi, [edi + 0x460]` (the driver) and jumps straight back to 0x741FDD.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, jne_mask, jp5, rd, wr, x};

orig! {
    /// `CPlayerPed::GetPlayerInfoForThisPlayerPed()` (0x609FF0)
    fn get_player_info(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609FF0 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn get_weapon_info(kind: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CPad::GetCarGunFired()` (0x53FF90): int16 in AX
    fn pad_get_car_gun_fired(pad: GPtr<u8>) -> i16 = thiscall 0x53FF90 => unported;
    /// virtual `CVehicle::GetRideAnimData()` (vtable +0xBC)
    fn v_get_ride_anim_data(v: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0xBC => unported;
    /// `CWeapon::Update(CPed* owner)` (0x73DB40)
    fn weapon_update(w: GPtr<u8>, owner: u32) = thiscall 0x73DB40 => unported;
    /// `CPad::GetLookLeft()` (0x53FDD0, AL)
    fn pad_get_look_left(pad: GPtr<u8>) -> u8 = thiscall 0x53FDD0 => unported;
    /// `CPad::GetLookRight()` (0x53FE10, AL)
    fn pad_get_look_right(pad: GPtr<u8>) -> u8 = thiscall 0x53FE10 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, uint32 animId)` (0x4D68B0)
    fn clump_get_association(c: GPtr<u8>, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float)` (0x4D4610)
    fn blend_animation(c: GPtr<u8>, group: u32, id: u32, delta: f32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `CWeapon::FireFromCar(CVehicle*, bool left, bool right)` (0x73FA20); the flags as the dwords
    /// the original pushes
    fn weapon_fire_from_car(w: GPtr<u8>, v: GPtr<u8>, left: u32, right: u32) -> u8 = thiscall 0x73FA20 => unported;
    /// `CPed::DoGunFlash(int32, bool)` (0x5DF340)
    fn ped_do_gun_flash(p: GPtr<u8>, time: i32, right: u32) = thiscall 0x5DF340 => unported;
}

/// `CVehicle::DoDriveByShootings()` (0x741FD0): a player driver allowed to drive-by (player info
/// +0x153) with a pad and an instant-hit weapon (weapon info +0x14 == 4) fires on the car-gun button
/// (state 1 on a BMX-type vehicle, +0x594 == 10, else any non-zero state). The drive-by anims are the
/// ride-anim group's 0xCA/0xCB/0xCC (vehicles with ride-anim data) or 0x4C/0x4D (vehicle flag +0x429
/// bit 3) / 0x4A/0x4B with 0xBF. The weapon is updated; the look direction comes from the pad's look
/// buttons in camera modes 1 and 0x31 or while 0xB6F057 is set, else from the active camera's flags.
/// When aiming (a direction, or fire on a ride-anim vehicle) with ammo the matching anim is kept or
/// blended in at 16 (forward 0xCC for ride-anim vehicles), and it fires once that anim is fully blended
/// (`> 0.99`, not finished) and the shot timer (+0x10) has passed: `FireFromCar(left, right)`, the next
/// shot 70 ms later, a 250 ms gun flash. Otherwise the clip is refilled (`min(total, clip size)`) and
/// the drive-by anims blend out at -8.
#[hook(addr = 0x741FD0, conv = thiscall, name = "CVehicle::DoDriveByShootings", this = "CVehicle",
       expect = "83 EC 24 56 57 8B F9",
       unverified = "veh4-3", route = "L2", hazards = "uninit-locals,heap-writes",
       note = "the left/right flags reach FireFromCar as dwords whose upper three bytes are uninitialised stack in the original (zero here); blends ped anims")]
pub fn vehicle_do_drive_by_shootings(this: GPtr<u8>) {
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if driver.is_null() {
        return;
    }
    let info = get_player_info(driver);
    if info.is_null() || rd::<u8>(info, 0x153) == 0 {
        return;
    }
    let pad = get_pad_from_player(driver);
    if pad.is_null() {
        return;
    }
    let slot = i32::from(rd::<i8>(driver, 0x718));
    let weapon = driver.byte_add(0x5A0 + 0x1C * slot as isize);
    if rd::<u32>(get_weapon_info(rd::<u32>(weapon, 0), 1), 0x14) != 4 {
        return;
    }
    let fire = if rd::<u32>(this, 0x594) == 10 {
        pad_get_car_gun_fired(pad) == 1
    } else {
        pad_get_car_gun_fired(pad) != 0
    };
    let mut group = 0u32;
    let ride = !v_get_ride_anim_data(this).is_null();
    let (a1, a2, a3) = if ride {
        group = rd::<u32>(v_get_ride_anim_data(this), 0);
        (0xCAu32, 0xCBu32, 0xCCu32)
    } else if rd::<u8>(this, 0x429) & 8 != 0 {
        (0x4C, 0x4D, 0xBF)
    } else {
        (0x4A, 0x4B, 0xBF)
    };
    weapon_update(weapon, 0);
    let cam = 0xB6_F1A8 + 0x238 * usize::from(g::<u8>(0xB6_F081).read());
    let mode = g::<u16>(cam).read();
    let (left, right) = if mode == 1 || g::<u8>(0xB6_F057).read() != 0 || mode == 0x31 {
        let l = pad_get_look_left(pad) != 0;
        let r = pad_get_look_right(pad) != 0;
        (l, r)
    } else {
        (g::<u8>(cam - 4).read() != 0, g::<u8>(cam - 3).read() != 0)
    };
    let clump = || rd::<GPtr<u8>>(driver, 0x18);
    let aim = if ride { fire } else { left || right };
    let mut blended = false;
    if aim && rd::<i32>(weapon, 0xC) > 0 {
        let anim = if left {
            Some(a1)
        } else if right {
            Some(a2)
        } else if ride {
            Some(a3)
        } else {
            None
        };
        if let Some(id) = anim {
            let mut assoc = clump_get_association(clump(), id);
            if assoc.is_null() || !jp5(x(rd::<f32>(assoc, 0x1C)), x(0.0)) {
                assoc = blend_animation(clump(), group, id, 16.0);
            }
            if !assoc.is_null()
                && (rd::<u8>(assoc, 0x2E) & 1 != 0 || jne_mask(x(rd::<f32>(assoc, 0x18)), x(f32::from_bits(0x3F7D_70A4)), 0x41))
            {
                return;
            }
        }
        blended = true;
    }
    if blended {
        let now = g::<u32>(0xB7_CB84).read();
        if !fire || now <= rd::<u32>(weapon, 0x10) {
            return;
        }
        weapon_fire_from_car(weapon, this, u32::from(left), u32::from(right));
        wr::<u32>(weapon, 0x10, g::<u32>(0xB7_CB84).read().wrapping_add(0x46));
        ped_do_gun_flash(driver, 0xFA, 0);
        return;
    }
    if rd::<u32>(weapon, 0xC) != 0 {
        let clip = rd::<i16>(get_weapon_info(rd::<u32>(weapon, 0), 1), 0x20) as i32 as u32;
        let total = rd::<u32>(weapon, 0xC);
        wr::<u32>(weapon, 8, if total >= clip { clip } else { total });
    }
    let out = 0xC100_0000u32;
    let mut last = clump_get_association(clump(), a1);
    if !last.is_null() {
        wr::<u32>(last, 0x1C, out);
    }
    last = clump_get_association(clump(), a2);
    if !last.is_null() {
        wr::<u32>(last, 0x1C, out);
    }
    if ride {
        last = clump_get_association(clump(), a3);
    }
    if !last.is_null() {
        wr::<u32>(last, 0x1C, out);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_DO_DRIVE_BY_SHOOTINGS];
