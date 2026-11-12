//! Model, collision and animation streaming on the original executable: the original backend's `sa_samp_api::streaming` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not routed through
//! the adapter yet; the client still does that work in its own modules.

use sa_samp_api::streaming::*;
use sa_samp_api::*;

use sa_samp_api::script::ScriptArg;

use super::OriginalGame;
use super::script::backend_op;
use crate::patch::Memory;
use crate::script::op;

/// Client-side shorthands for the streaming calls the client makes through the adapter (main thread).
#[cfg(windows)]
pub mod calls {
    use sa_samp_api::streaming::Streaming as _;
    use sa_samp_api::{ModelId, Vec3};

    use crate::adapter::original::process;

    pub fn model_loaded(model: i32) -> bool {
        process().model_loaded(ModelId(model))
    }

    pub fn request_model(model: i32) {
        let _ = process().request_model(ModelId(model));
    }

    pub fn release_model(model: i32) {
        let _ = process().release_model(ModelId(model));
    }

    pub fn load_all_now() {
        let _ = process().load_all_now();
    }

    pub fn request_collision(x: f32, y: f32) {
        let _ = process().request_collision(Vec3::new(x, y, 0.0));
    }

    pub fn component_loaded(component: i32) -> bool {
        process().component_loaded(ModelId(component))
    }

    pub fn request_component(component: i32) {
        let _ = process().request_component(ModelId(component));
    }
}

impl<M: Memory> Streaming for OriginalGame<M> {
    fn model_kind(&self, _model: ModelId) -> ModelKind {
        ModelKind::None
    }

    /// `REQUEST_MODEL` (0247).
    fn request_model(&mut self, model: ModelId) -> GameResult<()> {
        self.run_op(op::REQUEST_MODEL, &[ScriptArg::Int(model.0)]).map(|_| ())
    }

    /// `HAS_MODEL_LOADED` (0248); a fault reads as not loaded.
    fn model_loaded(&self, model: ModelId) -> bool {
        backend_op(op::HAS_MODEL_LOADED, &[ScriptArg::Int(model.0)]).unwrap_or(false)
    }

    /// `MARK_MODEL_AS_NO_LONGER_NEEDED` (0249).
    fn release_model(&mut self, model: ModelId) -> GameResult<()> {
        self.run_op(op::MARK_MODEL_AS_NO_LONGER_NEEDED, &[ScriptArg::Int(model.0)]).map(|_| ())
    }

    /// `LOAD_ALL_MODELS_NOW` (038B).
    fn load_all_now(&mut self) -> GameResult<()> {
        self.run_op(op::LOAD_ALL_MODELS_NOW, &[]).map(|_| ())
    }

    /// `REQUEST_COLLISION` (04E4) at x, y.
    fn request_collision(&mut self, at: Vec3) -> GameResult<()> {
        self.run_op(op::REQUEST_COLLISION, &[ScriptArg::Float(at.x), ScriptArg::Float(at.y)]).map(|_| ())
    }

    fn collision_loaded(&self, _at: Vec3, _area: u8) -> bool {
        false
    }

    fn request_anim_block(&mut self, _block: &str) -> GameResult<()> {
        Err(GameError::Unsupported("Streaming::request_anim_block"))
    }

    fn anim_block_loaded(&self, _block: &str) -> bool {
        false
    }

    /// `REQUEST_VEHICLE_MOD` (06E9).
    fn request_component(&mut self, model: ModelId) -> GameResult<()> {
        self.run_op(op::REQUEST_VEHICLE_MOD, &[ScriptArg::Int(model.0)]).map(|_| ())
    }

    /// `HAS_VEHICLE_MOD_LOADED` (06EA); a fault reads as not loaded.
    fn component_loaded(&self, model: ModelId) -> bool {
        backend_op(op::HAS_VEHICLE_MOD_LOADED, &[ScriptArg::Int(model.0)]).unwrap_or(false)
    }

    fn streaming_memory(&self) -> (u32, u32) {
        (0, 0)
    }
}
