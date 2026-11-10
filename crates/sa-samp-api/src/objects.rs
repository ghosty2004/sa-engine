//! Objects, attached objects, materials and building removal. samp-rs today: `objects.rs`, `objects/*.rs`,
//! `objedit.rs`, `ui/mattext.rs`, `worldhooks/*.rs`. Address mapping: `docs/samp/game-adapter.md` §4.3.

use crate::{
    EntityRef, GameResult, Matrix, ModelId, ObjectRef, PedRef, Rgba, ScriptHandle, TextureRef, Vec3,
    VehicleRef,
};

/// Where an object is attached.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AttachTarget {
    /// `ATTACH_OBJECT_TO_CAR` (0681).
    Vehicle(VehicleRef),
    /// `ATTACH_OBJECT_TO_OBJECT` (069A).
    Object(ObjectRef),
    /// `ATTACH_OBJECT_TO_CHAR` (069B); samp's player-attached objects (RPC 113) use a bone instead.
    Ped(PedRef),
    /// Attached to a ped bone and drawn by samp's ped render hook (RPC 113 SetPlayerAttachedObject).
    PedBone {
        /// Ped.
        ped: PedRef,
        /// Bone id (samp numbering 1..=18).
        bone: u8,
    },
}

/// A material slot override (RPC 44/84 SetObjectMaterial / SetObjectMaterialText).
#[derive(Clone, Debug, PartialEq)]
pub enum MaterialOverride {
    /// Replace the texture of material `index` with `txd:texture` of `model`, tinted by `colour`.
    Texture {
        /// Model whose TXD holds the texture.
        model: ModelId,
        /// TXD name.
        txd: String,
        /// Texture name.
        texture: String,
        /// Tint (ARGB as RGBA).
        colour: Rgba,
    },
    /// Replace material `index` with a texture samp rendered itself (material text, `ui::mattext`).
    Rendered(TextureRef),
    /// Only change the colour.
    Colour(Rgba),
}

/// Result of a ray or sphere test against one object (object picking, `CCollision::TestLineSphere`
/// 0x417470, `CCollision::SphereCastVsEntity` 0x419F00).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjectHit {
    /// Hit point.
    pub point: Vec3,
    /// Distance along the ray (0..1).
    pub fraction: f32,
}

/// Object operations.
pub trait Objects {
    /// Script handle (`CPools::GetObjectRef`, pool 0xB7449C).
    fn object_script_handle(&self, o: ObjectRef) -> GameResult<ScriptHandle>;
    /// Object from a script handle.
    fn object_from_script_handle(&self, h: ScriptHandle) -> Option<ObjectRef>;
    /// Matrix.
    fn object_matrix(&self, o: ObjectRef) -> GameResult<Matrix>;
    /// Whether the object's RW object exists (streamed in; `CEntity::m_pRwObject` +0x18).
    fn object_has_rw(&self, o: ObjectRef) -> GameResult<bool>;
    /// `IS_OBJECT_ATTACHED` (0685).
    fn object_attached(&self, o: ObjectRef) -> GameResult<bool>;
    /// Bounding sphere centre (`CEntity::GetBoundCentre` 0x534250) and radius (col model +0x24/+0x2C).
    fn object_bounds(&self, o: ObjectRef) -> GameResult<(Vec3, f32)>;
    /// Ray test against one object.
    fn object_ray_test(&self, o: ObjectRef, from: Vec3, to: Vec3) -> GameResult<Option<ObjectHit>>;

    /// `CREATE_OBJECT` (0107; negative model ids allowed with
    /// [`crate::config::Behaviour::create_object_negative_models`]).
    fn create_object(&mut self, model: ModelId, pos: Vec3) -> GameResult<ObjectRef>;
    /// `DELETE_OBJECT` (0108) or `REMOVE_OBJECT_ELEGANTLY` (09A2) when `fade`.
    fn destroy_object(&mut self, o: ObjectRef, fade: bool) -> GameResult<()>;
    /// Writes position and rotation (Euler degrees, `SET_OBJECT_ROTATION` 0453 order).
    fn set_object_transform(&mut self, o: ObjectRef, pos: Vec3, rot_deg: Vec3) -> GameResult<()>;
    /// Writes the matrix directly (MoveObject interpolation, attached objects).
    fn set_object_matrix(&mut self, o: ObjectRef, m: &Matrix) -> GameResult<()>;
    /// `SET_OBJECT_SCALE` (08D2).
    fn set_object_scale(&mut self, o: ObjectRef, scale: f32) -> GameResult<()>;
    /// `SET_OBJECT_VISIBLE` (0750).
    fn set_object_visible(&mut self, o: ObjectRef, visible: bool) -> GameResult<()>;
    /// Collision on/off.
    fn set_object_collision(&mut self, o: ObjectRef, on: bool) -> GameResult<()>;
    /// Attaches with an offset and rotation.
    fn attach_object(
        &mut self,
        o: ObjectRef,
        to: AttachTarget,
        offset: Vec3,
        rot_deg: Vec3,
    ) -> GameResult<()>;
    /// `DETACH_OBJECT` (0682).
    fn detach_object(&mut self, o: ObjectRef) -> GameResult<()>;
    /// `PLAY_OBJECT_ANIM` (075A).
    fn play_object_anim(
        &mut self,
        o: ObjectRef,
        anim: &str,
        block: &str,
        delta: f32,
        looped: bool,
    ) -> GameResult<()>;
    /// Sets or clears a material override; applied at render time by the backend (the original backend
    /// does it in samp's `CObject::Render` hook 0x59F180 / vtable 0x866FA8).
    fn set_object_material(&mut self, o: ObjectRef, index: u8, m: Option<MaterialOverride>)
    -> GameResult<()>;
    /// RemoveBuildingForPlayer (RPC 43): hides matching buildings and dummies now and for future IPL loads
    /// (`CFileLoader::LoadObjectInstance` hook 0x538090, occluders 0xC73FA0, dummy pool 0xB744A0).
    /// `model` −1 = any.
    fn remove_buildings(&mut self, model: ModelId, center: Vec3, radius: f32) -> GameResult<u32>;
    /// Makes the camera ignore this entity (`CWorld::CameraToIgnoreThisObject` 0x563F40).
    fn camera_ignore(&mut self, e: EntityRef) -> GameResult<()>;
}
