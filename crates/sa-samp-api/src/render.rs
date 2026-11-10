//! samp's own drawing: chat, dialogs, scoreboard, nametags, 3D labels, the cursor, material text and model
//! previews.
//!
//! Today samp-rs draws through Direct3D 9: on the first frame it replaces `_RwD3DDevice` (0xC97C28) with a
//! proxy, intercepts `Present`/`Reset`/`SetTexture`/`EndScene`, and uses D3DX fonts and sprites
//! (`ui/d3d/*.rs`, `ui/render/*.rs`). The native engine has no D3D9 device. [`Overlay`] is the
//! renderer-neutral surface: the original backend keeps the D3D9 path (and exposes the device through
//! [`RenderApi::Direct3D9`] for the parity-critical call stream), the native backend implements the
//! primitives on wgpu. Mapping: `docs/samp/game-adapter.md` §4.10.

use crate::{GameResult, ModelId, Point2, Rect2, Rgba, TextureRef, Vec3};

/// The graphics API behind the overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderApi {
    /// Direct3D 9; `device` is the real `IDirect3DDevice9*` (original and phase A backends).
    Direct3D9 {
        /// Device pointer.
        device: usize,
    },
    /// Engine-owned renderer (wgpu); only the [`Overlay`] primitives are available.
    Engine,
}

/// Pixel format of uploaded texture data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    /// 8-bit RGBA, row-major, no padding.
    Rgba8,
    /// 8-bit alpha only (glyph atlases).
    A8,
}

/// A vertex of an overlay triangle list (screen pixels).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OverlayVertex {
    /// Position.
    pub pos: Point2,
    /// Texture coordinate.
    pub uv: Point2,
    /// Colour (multiplied with the texture).
    pub colour: Rgba,
}

/// A model preview request (textdraw font 5): what to render into a texture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelPreview {
    /// Model (ped, vehicle or object).
    pub model: ModelId,
    /// Rotation in degrees.
    pub rot: Vec3,
    /// Zoom.
    pub zoom: f32,
    /// Vehicle colours (−1 = default).
    pub colours: [i16; 2],
    /// Background colour.
    pub background: Rgba,
    /// Texture size in pixels.
    pub size: u16,
}

/// samp's 2D overlay drawing. Valid only in [`crate::hooks::FramePhase::Present`] (after the game frame,
/// before the swap) unless stated otherwise.
pub trait Overlay {
    /// The graphics API.
    fn render_api(&self) -> RenderApi;
    /// Creates a texture (fonts' glyph atlases, chat render target contents, material text).
    fn create_texture(&mut self, width: u32, height: u32, format: PixelFormat) -> GameResult<TextureRef>;
    /// Uploads pixels into a texture region.
    fn update_texture(
        &mut self,
        tex: TextureRef,
        region: Rect2,
        format: PixelFormat,
        pixels: &[u8],
    ) -> GameResult<()>;
    /// Frees a texture.
    fn destroy_texture(&mut self, tex: TextureRef) -> GameResult<()>;
    /// Draws a textured (or untextured with `None`) triangle list, alpha blended, clipped to `clip`.
    fn draw_triangles(
        &mut self,
        tex: Option<TextureRef>,
        vertices: &[OverlayVertex],
        clip: Option<Rect2>,
    ) -> GameResult<()>;
    /// Draws lines (scoreboard, dialog frames).
    fn draw_lines(&mut self, points: &[Point2], colour: Rgba) -> GameResult<()>;
    /// Renders a model preview into a texture (samp-rs `ui/preview.rs`, rendered after `CGame::Process`).
    /// Valid in [`crate::hooks::FramePhase::AfterGameProcess`].
    fn render_model_preview(&mut self, preview: &ModelPreview) -> GameResult<TextureRef>;
    /// Takes a screenshot of the next presented frame into the user's folder (F8); returns the file name.
    fn screenshot(&mut self) -> GameResult<String>;
}
