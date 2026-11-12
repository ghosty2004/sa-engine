//! The overlay renderer on the native engine: the native backend's `sa_samp_api::render` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::render::*;
use sa_samp_api::*;

use crate::{NativeGame, View};

impl<V: View> Overlay for NativeGame<V> {
    fn render_api(&self) -> RenderApi {
        RenderApi::Direct3D9 { device: 0 }
    }

    fn create_texture(&mut self, _width: u32, _height: u32, _format: PixelFormat) -> GameResult<TextureRef> {
        Err(GameError::Unsupported("Overlay::create_texture"))
    }

    fn update_texture(
        &mut self,
        _tex: TextureRef,
        _region: Rect2,
        _format: PixelFormat,
        _pixels: &[u8],
    ) -> GameResult<()> {
        Err(GameError::Unsupported("Overlay::update_texture"))
    }

    fn destroy_texture(&mut self, _tex: TextureRef) -> GameResult<()> {
        Err(GameError::Unsupported("Overlay::destroy_texture"))
    }

    fn draw_triangles(
        &mut self,
        _tex: Option<TextureRef>,
        _vertices: &[OverlayVertex],
        _clip: Option<Rect2>,
    ) -> GameResult<()> {
        Err(GameError::Unsupported("Overlay::draw_triangles"))
    }

    fn draw_lines(&mut self, _points: &[Point2], _colour: Rgba) -> GameResult<()> {
        Err(GameError::Unsupported("Overlay::draw_lines"))
    }

    fn render_model_preview(&mut self, _preview: &ModelPreview) -> GameResult<TextureRef> {
        Err(GameError::Unsupported("Overlay::render_model_preview"))
    }

    fn screenshot(&mut self) -> GameResult<String> {
        Err(GameError::Unsupported("Overlay::screenshot"))
    }
}
