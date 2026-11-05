//! Model infos: `CBaseModelInfo` and every subclass, `CModelInfo` (the model table and the per-type
//! stores), `ModelIndices`, `CLinkedUpgradeList`.
//!
//! Owners (PORT-STREAM): helper B owns every submodule except [`vehicle`] (helper C).

/// `CBaseModelInfo` (vtable 0x85BBE0 family root)
pub mod base;
/// `CAtomicModelInfo`, `CDamageAtomicModelInfo`, `CLodAtomicModelInfo`
pub mod atomic;
/// `CTimeInfo`, `CTimeModelInfo`, `CLodTimeModelInfo`
pub mod time;
/// `CWeaponModelInfo`
pub mod weapon;
/// `CClumpModelInfo`
pub mod clump;
/// `CPedModelInfo`
pub mod ped;
/// `CModelInfo` (statics, stores, static initialisers/destructors of the stores)
pub mod model_info;
/// `ModelIndices`, `CLinkedUpgradeList`
pub mod model_indices;
/// `CVehicleModelInfo` (owner: helper C)
pub mod vehicle;
