//! Models, collision, animation blocks and streaming memory. samp-rs today: `client/gta.rs`,
//! `behaviourclient.rs`, `vehicles/*.rs`, `anims.rs`, `ui/glue/adapters.rs`. Mapping:
//! `docs/samp/game-adapter.md` §4.9.

use crate::{GameResult, ModelId, Vec3};

/// Model category, from the model-info vtable (samp-rs compares 0x85BBF0 … 0x85BDC0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelKind {
    /// Atomic / damage atomic / LOD atomic / time / LOD time (0x85BBF0…0x85BCF0).
    Atomic,
    /// Clump (non-ped, non-vehicle).
    Clump,
    /// Ped (0x85BDC0).
    Ped,
    /// Vehicle.
    Vehicle,
    /// Weapon.
    Weapon,
    /// Not defined.
    None,
}

/// Streaming operations.
pub trait Streaming {
    /// Model category.
    fn model_kind(&self, model: ModelId) -> ModelKind;
    /// `REQUEST_MODEL` (0247).
    fn request_model(&mut self, model: ModelId) -> GameResult<()>;
    /// `HAS_MODEL_LOADED` (0248).
    fn model_loaded(&self, model: ModelId) -> bool;
    /// `MARK_MODEL_AS_NO_LONGER_NEEDED` (0249).
    fn release_model(&mut self, model: ModelId) -> GameResult<()>;
    /// `LOAD_ALL_MODELS_NOW` (038B).
    fn load_all_now(&mut self) -> GameResult<()>;
    /// `REQUEST_COLLISION` (04E4).
    fn request_collision(&mut self, at: Vec3) -> GameResult<()>;
    /// Whether collision is loaded around a point (`CColStore::HasCollisionLoaded` 0x410CE0).
    fn collision_loaded(&self, at: Vec3, area: u8) -> bool;
    /// `REQUEST_ANIMATION` (04ED).
    fn request_anim_block(&mut self, block: &str) -> GameResult<()>;
    /// `HAS_ANIMATION_LOADED` (04EE).
    fn anim_block_loaded(&self, block: &str) -> bool;
    /// `REQUEST_VEHICLE_MOD` (06E9).
    fn request_component(&mut self, model: ModelId) -> GameResult<()>;
    /// `HAS_VEHICLE_MOD_LOADED` (06EA).
    fn component_loaded(&self, model: ModelId) -> bool;
    /// Streaming memory: available and used bytes (`CStreaming::ms_memoryAvailable` 0x8A5A80,
    /// `ms_memoryUsed` 0x8E4CB4), shown by `/mem` and F5.
    fn streaming_memory(&self) -> (u32, u32);
}
