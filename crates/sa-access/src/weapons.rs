//! A ped's shot at a point: what a client that replays other players' shots needs.

use sa_core::bindings::{fire_instant_hit, get_weapon_info, ped_get_transformed_bone_position};
use sa_core::mem::GPtr;
use sa_types::{GameError, GameResult, PedRef, Vec3};

use crate::addr::off;
use crate::{Game, View};

/// `CWeaponInfo::m_eWeaponFire` and its instant-hit value.
const INFO_FIRE_TYPE: u32 = 0x0;
const FIRE_INSTANT_HIT: u32 = 1;
/// `CWeaponInfo::m_vecFireOffset`: the muzzle in the frame of the hand bone.
const INFO_FIRE_OFFSET: u32 = 0x24;
/// `BONE_R_HAND`, the bone a ped's gun is held in.
const BONE_RIGHT_HAND: u32 = 24;
/// The standard skill's weapon info (`GetWeaponInfo(type, 1)`), the one without skill variants.
const SKILL_STD: u8 = 1;
/// The weapon types with a weapon info (`aWeaponInfo`).
const WEAPON_TYPES: u32 = crate::addr::WEAPON_INFO_STD_COUNT as u32;

impl<V: View> Game<V> {
    /// Ped `ped` fires one shot of its current weapon, an instant-hit one (pistols, shotguns, sub-machine guns,
    /// rifles, the minigun), at `target`: `CWeapon::FireInstantHit` from the weapon's muzzle at the right hand (the
    /// fire offset transformed by the hand bone, as `CTaskSimpleUseGun` fires) toward the point, aimed whatever
    /// the ped's tasks, with the muzzle flash, the sound and the shell. The game's line test decides what the shot
    /// hits and its damage response what that takes. The weapon's ammo and state stay as they are. Refused for a
    /// ped without its model streamed in, a weapon that does not fire instant hits, or a target that is not finite.
    pub fn fire_ped_weapon(&mut self, ped: PedRef, target: Vec3) -> GameResult<()> {
        let p = self.ped_addr(ped)?;
        if ![target.x, target.y, target.z].iter().all(|c| c.is_finite()) {
            return Err(GameError::Other("target is not finite".into()));
        }
        if self.view.read_u32(p + off::RW_OBJECT) == 0 || self.view.read_u32(p + off::MATRIX) == 0 {
            return Err(GameError::NotLoaded);
        }
        let slot = u32::from(self.view.read_u8(p + off::PED_WEAPON_SLOT));
        if slot >= 13 {
            return Err(GameError::Other(format!("weapon slot {slot}")));
        }
        let weapon = p + off::PED_WEAPONS + slot * off::WEAPON_SIZE;
        let ty = self.view.read_u32(weapon);
        if ty >= WEAPON_TYPES {
            return Err(GameError::Other(format!("weapon type {ty}")));
        }
        let info = get_weapon_info(ty as i32, SKILL_STD);
        let info_addr = info.addr() as u32;
        if info.is_null() || self.view.read_u32(info_addr + INFO_FIRE_TYPE) != FIRE_INSTANT_HIT {
            return Err(GameError::Other(format!("weapon {ty} does not fire instant hits")));
        }
        let offset = self.view.read_vec3(info_addr + INFO_FIRE_OFFSET);
        let mut muzzle = [offset.x, offset.y, offset.z];
        let ped_ptr = GPtr::<u8>::from_addr(p as usize);
        let at = |v: &mut [f32; 3]| GPtr::<u8>::from_raw(v.as_mut_ptr().cast());
        ped_get_transformed_bone_position(ped_ptr, at(&mut muzzle), BONE_RIGHT_HAND, 1);
        let (mut origin, mut target) = (muzzle, [target.x, target.y, target.z]);
        fire_instant_hit(
            GPtr::from_addr(weapon as usize),
            ped_ptr,
            at(&mut origin),
            at(&mut muzzle),
            GPtr::NULL,
            at(&mut target),
            GPtr::NULL,
            1,
            1,
        );
        Ok(())
    }
}
