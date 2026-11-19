//! Objects and models.

use crate::{AreaCode, Matrix, ModelId, Vec3};

/// What a model is, from its model info's class (`CBaseModelInfo` vtable).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelKind {
    /// `CAtomicModelInfo` and its kinds (damage, LOD, time, LOD time).
    Atomic,
    /// `CClumpModelInfo` (a clump that is no ped, vehicle or weapon).
    Clump,
    /// `CPedModelInfo`.
    Ped,
    /// `CVehicleModelInfo`.
    Vehicle,
    /// `CWeaponModelInfo`.
    Weapon,
    /// No model info.
    None,
}

/// An object's state at one moment.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ObjectSnapshot {
    /// Transform.
    pub matrix: Matrix,
    /// Model.
    pub model: ModelId,
    /// Area / interior (+0x2F).
    pub area: AreaCode,
    /// Its RenderWare object exists (streamed in and drawable; `m_pRwObject` +0x18).
    pub has_rw: bool,
    /// Drawn (`bIsVisible`, flags +0x1C bit 7).
    pub visible: bool,
    /// Collides (`bUsesCollision`, flags +0x1C bit 0).
    pub collision: bool,
    /// Attached to another entity (`m_pAttachedTo` +0xFC).
    pub attached: bool,
    /// Scale (`CObject::m_fScale` +0x15C).
    pub scale: f32,
    /// Move speed (+0x44, units per 1/50 s).
    pub move_speed: Vec3,
    /// Turn speed (+0x50).
    pub turn_speed: Vec3,
    /// Centre of the collision model's bounding sphere in the world.
    pub bound_centre: Vec3,
    /// Radius of the collision model's bounding sphere (0 without a collision model).
    pub bound_radius: f32,
}
