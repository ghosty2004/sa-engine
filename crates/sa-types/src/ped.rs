//! Peds.

use crate::{AreaCode, Matrix, ModelId, PedRef, Vec3, VehicleRef, WeaponType};

/// One weapon slot (`CWeapon`, 0x1C bytes at `CPed::m_aWeapons` +0x5A0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WeaponSlot {
    /// Weapon type.
    pub weapon: u8,
    /// `CWeapon::m_nState` (+0x4).
    pub state: u32,
    /// Ammo in clip.
    pub ammo_in_clip: u32,
    /// Total ammo.
    pub ammo_total: u32,
}

/// A ped's state at one moment.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PedSnapshot {
    /// Transform (`CPlaceable::m_matrix` +0x14).
    pub matrix: Matrix,
    /// Move speed (`CPhysical::m_vecMoveSpeed` +0x44).
    pub move_speed: Vec3,
    /// Health (+0x540).
    pub health: f32,
    /// Armour (+0x548).
    pub armour: f32,
    /// Current heading in radians (+0x558).
    pub heading: f32,
    /// Heading goal (+0x55C).
    pub heading_goal: f32,
    /// `ePedState` (+0x530).
    pub ped_state: u32,
    /// Model index (`CEntity::m_nModelIndex` +0x22).
    pub model: ModelId,
    /// Selected weapon slot (+0x718).
    pub weapon_slot: u8,
    /// Current weapon type.
    pub weapon: WeaponType,
    /// Vehicle the ped is in (`CPed::m_pVehicle` +0x58C, valid when the in-vehicle flag is set).
    pub vehicle: Option<VehicleRef>,
    /// Area code the ped is in.
    pub area: AreaCode,
    /// Ped flag words (first two dwords of `CPed` flags; in-vehicle, ducking, ...), raw.
    pub flags: [u32; 2],
    /// Ped is in the world (not a deleted placeable, in the moving list) (the vtable is not `CPlaceable`'s and the ped is in the moving list).
    pub in_world: bool,
}


/// What dealt a ped its last damage (`CPed::m_pLastEntityDamage` +0x764).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DamageSource {
    /// Nothing (no damage since the record was cleared).
    #[default]
    None,
    /// A ped of the ped pool.
    Ped(PedRef),
    /// A vehicle of the vehicle pool.
    Vehicle(VehicleRef),
    /// Anything else (an object, a building).
    Other,
}

/// A ped's last damage: the weapon type (`CPed::m_nLastWeaponDamage` +0x760) and its source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PedDamage {
    /// The weapon type, as the game records it (`eWeaponType`, or the damage kinds above the weapons: 49 rammed,
    /// 50 helicopter blades, 51 explosion, 53 drowned, 54 collision or fall; 255 none).
    pub weapon: WeaponType,
    /// Who or what dealt it.
    pub source: DamageSource,
}
