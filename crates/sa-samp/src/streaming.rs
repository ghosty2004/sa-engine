//! Model, collision and animation streaming on the native engine: the native backend's `sa_samp_api::streaming` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::streaming::*;
use sa_samp_api::*;

use crate::{NativeGame, View};

impl<V: View> Streaming for NativeGame<V> {
    fn model_kind(&self, _model: ModelId) -> ModelKind {
        ModelKind::None
    }

    fn request_model(&mut self, _model: ModelId) -> GameResult<()> {
        Err(GameError::Unsupported("Streaming::request_model"))
    }

    fn model_loaded(&self, _model: ModelId) -> bool {
        false
    }

    fn release_model(&mut self, _model: ModelId) -> GameResult<()> {
        Err(GameError::Unsupported("Streaming::release_model"))
    }

    fn load_all_now(&mut self) -> GameResult<()> {
        Err(GameError::Unsupported("Streaming::load_all_now"))
    }

    fn request_collision(&mut self, _at: Vec3) -> GameResult<()> {
        Err(GameError::Unsupported("Streaming::request_collision"))
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

    fn request_component(&mut self, _model: ModelId) -> GameResult<()> {
        Err(GameError::Unsupported("Streaming::request_component"))
    }

    fn component_loaded(&self, _model: ModelId) -> bool {
        false
    }

    fn streaming_memory(&self) -> (u32, u32) {
        (0, 0)
    }
}
