//! Model, collision and animation streaming on the native engine: the native backend's `sa_samp_api::streaming` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::script::ScriptArg::{Float as F, Int as I};
use sa_samp_api::streaming::*;
use sa_samp_api::*;

use crate::addr;
use crate::script::op;

use crate::{NativeGame, View};

impl<V: View> Streaming for NativeGame<V> {
    fn model_kind(&self, _model: ModelId) -> ModelKind {
        ModelKind::None
    }

    fn request_model(&mut self, model: ModelId) -> GameResult<()> {
        self.run_op(op::REQUEST_MODEL, &[I(model.0)]).map(drop)
    }

    fn model_loaded(&self, model: ModelId) -> bool {
        // `CStreaming::ms_aInfoForModel[model].m_nLoadState == LOADSTATE_LOADED`.
        (0..addr::STREAMING_MODELS as i32).contains(&model.0)
            && self
                .view
                .read_u8(addr::STREAMING_INFO + model.0 as u32 * addr::STREAMING_INFO_SIZE + 0x10)
                == 1
    }

    fn release_model(&mut self, model: ModelId) -> GameResult<()> {
        self.run_op(op::MARK_MODEL_AS_NO_LONGER_NEEDED, &[I(model.0)])
            .map(drop)
    }

    fn load_all_now(&mut self) -> GameResult<()> {
        self.run_op(op::LOAD_ALL_MODELS_NOW, &[]).map(drop)
    }

    fn request_collision(&mut self, at: Vec3) -> GameResult<()> {
        self.run_op(op::REQUEST_COLLISION, &[F(at.x), F(at.y)]).map(drop)
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
