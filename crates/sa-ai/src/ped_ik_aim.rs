//! `CPedIK::PointGunAtPosition` (FRONTIER-2 part 4, batch `frontier2-8`): the aim of the armed police in the crossing
//! run. A CPedIK (ANIM) method, but it needs `CPed::GetWeaponSkill` (bound in sa-world, above sa-anim), so it lives
//! here next to its callers (`CTaskSimpleUseGun`). Ported from the 1.0 US bytes.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::common::*;
use sa_anim::bindings::pedik_point_gun_in_direction as point_gun_in_direction_b;
use sa_core::bindings::general_get_radian_angle_between_points as get_radian_angle_between_points;
use sa_core::bindings::get_weapon_info;
use sa_world::bindings::ped_get_weapon_skill;

/// `CPedIK::PointGunAtPosition(const CVector& aimAt, float normalize)` (0x5FDE20, thiscall, `ret 8`, AL from
/// `PointGunInDirection`; the weapon slot load passes through the protection fragment 0x4012EE; FRONTIER-2 part 4,
/// reached by the armed police of the crossing run). The ped's weapon info at its current skill; the aiming offset row
/// `0xC8A8A8 + 24 * info.animGroup(+0x6C, word)`: x = +8 / z = +0xC when ducking (ped +0x46C bit 26), else +0 / +4.
/// `from = pos + x * right + z * up` with the x87 order of the original: `x * right.x` stays extended and is added to
/// pos.x, the y and z products are stored as floats first; `from.x = f32(z * up.x + (x * right.x + pos.x))`, `from.y =
/// f32(f32(z * up.y) + (f32(x * right.y) + pos.y))`, `from.z = f32(f32(z * up.z) + f32(f32(x * right.z) + pos.z))`.
/// Then `PointGunInDirection(GetRadianAngleBetweenPoints(at.x, at.y, from.x, from.y), GetRadianAngleBetweenPoints(at.z,
/// |from - at| (2D, `sqrt(dy² + dx²)`), from.z, 0), false, normalize)`. Both angles are rounded to floats; the original
/// keeps the second in its `aimAt` argument slot.
#[hook(addr = 0x5FDE20, conv = thiscall, name = "CPedIK::PointGunAtPosition", expect = "83 EC 1C 56 57 8B F9 8B 0F 90 90 E9",
       unverified = "frontier2-8", route = "L2", hazards = "x87-transcendental", recs = "weapon-combat",
       note = "crossing 4000 (the armed police aim at CJ)")]
pub fn ped_ik_point_gun_at_position(this: GPtr<u8>, at: GPtr<u8>, normalize: u32) -> u8 {
    let ped = rd::<GPtr<u8>>(this, 0);
    let slot = isize::from(rd::<i8>(ped, 0x718));
    let wt = rd::<i32>(ped, 0x5A0 + slot * 0x1C);
    let wi = get_weapon_info(wt, ped_get_weapon_skill(ped));
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let (px, py, pz) = (rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8));
    let row = 0xC8_A8A8usize.wrapping_add((24 * i32::from(rd::<i16>(wi, 0x6C))) as isize as usize);
    let ducking = (rd::<u32>(ped, 0x46C) >> 26) & 1 != 0;
    let ox = X87::from_f32(sa_core::field::gf(if ducking { row + 8 } else { row }));
    let ry = ox.fmul_m(rd::<f32>(m, 4)).to_f32();
    let rz = ox.fmul_m(rd::<f32>(m, 8)).to_f32();
    let ax = ox.fmul_m(rd::<f32>(m, 0)).fadd_m(px);
    let ay = X87::from_f32(ry).fadd_m(py);
    let az = X87::from_f32(rz).fadd_m(pz).to_f32();
    let oz = X87::from_f32(sa_core::field::gf(if ducking { row + 0xC } else { row + 4 }));
    let uy = oz.fmul_m(rd::<f32>(m, 0x24)).to_f32();
    let uz = oz.fmul_m(rd::<f32>(m, 0x28)).to_f32();
    let fx = oz.fmul_m(rd::<f32>(m, 0x20)).fadd(ax).to_f32();
    let fy = X87::from_f32(uy).fadd(ay).to_f32();
    let fz = X87::from_f32(uz).fadd_m(az).to_f32();
    let (atx, aty, atz) = (rd::<f32>(at, 0), rd::<f32>(at, 4), rd::<f32>(at, 8));
    let z_angle = X87::from_f64_exact(get_radian_angle_between_points(atx, aty, fx, fy).to_f64()).to_f32();
    let dx = X87::from_f32(fx).fsub_m(atx);
    let dy = X87::from_f32(fy).fsub_m(aty);
    let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt().to_f32();
    let pitch = X87::from_f64_exact(get_radian_angle_between_points(atz, d, fz, 0.0).to_f64()).to_f32();
    point_gun_in_direction_b(this, z_angle.to_bits(), pitch.to_bits(), 0, normalize)
}

pub const HOOKS: &[HookReg] = &[HOOK_PED_IK_POINT_GUN_AT_POSITION];
