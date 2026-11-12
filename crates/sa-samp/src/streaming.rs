//! Model, collision and animation streaming on the native engine: the native backend's `sa_samp_api::streaming` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::script::ScriptArg::{Float as F, Int as I, Text as T};
use sa_samp_api::streaming::*;
use sa_samp_api::*;

use crate::addr;
use crate::script::op;

use crate::{NativeGame, View};

impl<V: View> NativeGame<V> {
    /// The `CAnimBlock` named `block` (case-insensitive, as `CAnimManager::GetAnimationBlock`), if the game has one.
    pub(crate) fn anim_block_addr(&self, block: &str) -> Option<u32> {
        let want = block.as_bytes();
        if want.is_empty() || want.len() > 15 {
            return None;
        }
        let n = self.view.read_u32(addr::ANIM_BLOCK_COUNT).min(180);
        (0..n)
            .map(|i| addr::ANIM_BLOCKS + i * addr::ANIM_BLOCK_SIZE)
            .find(|&b| {
                let mut name = [0u8; 16];
                self.view.read(b, &mut name);
                let len = name.iter().position(|&c| c == 0).unwrap_or(16);
                name[..len].eq_ignore_ascii_case(want)
            })
    }
}

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

    fn request_anim_block(&mut self, block: &str) -> GameResult<()> {
        if self.anim_block_addr(block).is_none() {
            return Err(GameError::Other(format!("unknown animation block {block:?}")));
        }
        self.run_op(op::REQUEST_ANIMATION, &[T(block.as_bytes())])
            .map(drop)
    }

    fn anim_block_loaded(&self, block: &str) -> bool {
        // `CAnimBlock::bLoaded` (+0x10).
        self.anim_block_addr(block)
            .is_some_and(|b| self.view.read_u8(b + 0x10) != 0)
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
