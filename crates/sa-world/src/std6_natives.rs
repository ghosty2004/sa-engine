//! Phase B registrations (`#[native]`) of existing ports whose entries cannot be hooked in phase A
//! (STD-6: their docs say "not hookable"; collected by address).

/// Every `#[native]` STD-6 added to existing ports of this crate.
pub const NATIVES: &[sa_core::abi::NativeReg] = &[
    crate::world::stats::NATIVE_SET_NEW_RECORD_STAT,
    crate::world::entity::NATIVE_AS_XXX,
    crate::world::entity::NATIVE_PLACEABLE_GET_MATRIX_RAW,
    crate::world::misc2::NATIVE_INTERIOR_GROUP_GET_ENTITY,
    crate::world::misc2::NATIVE_BREAK_OBJECT_DTOR,
    crate::collision::camera_collision_vehicles::NATIVE_CHECK_CAMERA_COLLISION_VEHICLES,
];
