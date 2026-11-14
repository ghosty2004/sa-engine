//! The engine-drawn SA:MP UI on the native backend (docs/samp/overlay.md): [`NativeGame`] owns a
//! `sa_samp_ui::Ui` (served to samp-rs through `GameAdapter::ui`), feeds it window events before the client
//! sees them, and draws it at `FramePhase::After2d` through the ported engine ([`render`]).

pub mod render;

use sa_samp_api::hooks::FramePhase;
use sa_samp_api::input::{EventResult, WindowEvent};
use sa_samp_api::peds::Peds as _;
use sa_samp_api::{GameAdapter as _, PedRef, Vec3};
use sa_samp_ui::font::{FontAtlas, heights_for_width};
use sa_samp_ui::{Ui, WorldView};

use crate::addr::{self, off};
use crate::{NativeGame, View};

/// The UI and its engine resources.
#[derive(Default)]
pub struct Overlay {
    /// The UI state (what `GameAdapter::ui` hands out).
    pub ui: Ui,
    atlas: Option<FontAtlas>,
    /// The atlas raster (`RwRaster*`), 0 before the upload or when it failed.
    raster: u32,
    upload_failed: bool,
    /// samp's skin (`sampgui.png` + `mouse.png`, [`sa_samp_ui::skin`]) as RGBA8, from the user's SA:MP
    /// install; `None` = flat controls.
    skin_image: Option<Vec<u8>>,
    /// The skin raster, 0 before the upload or when it failed.
    skin_raster: u32,
    skin_failed: bool,
    /// The UI font file from the user's system (Arial), `None` = the committed Liberation Sans Bold.
    font_file: Option<Vec<u8>>,
    big: render::BigMessages,
    sprites: render::Sprites,
    /// Frames drawn (for logs).
    pub frames: u64,
}

impl std::fmt::Debug for Overlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Overlay")
            .field("raster", &self.raster)
            .field("frames", &self.frames)
            .finish()
    }
}

/// The world as the overlay sees it on the engine.
struct EngineWorld<'a, V: View> {
    game: &'a NativeGame<V>,
}

impl<V: View> WorldView for EngineWorld<'_, V> {
    fn project(&self, p: Vec3) -> Option<(f32, f32)> {
        render::project(p)
    }
    fn camera_pos(&self) -> Vec3 {
        // `TheCamera.m_mCameraMatrix` position.
        self.game.view.read_vec3(addr::camera::MATRIX + addr::matrix::POS)
    }
    fn ped_position(&self, ped: PedRef) -> Option<Vec3> {
        self.game.ped_snapshot(ped).ok().map(|s| s.matrix.pos)
    }
    fn line_clear(&self, a: Vec3, b: Vec3) -> bool {
        render::line_clear(a, b)
    }
}

impl Overlay {
    /// Gives the overlay samp's skin texture: `rgba` is [`sa_samp_ui::skin::TEXTURE_W`] ×
    /// [`sa_samp_ui::skin::TEXTURE_H`] RGBA8 (`sampgui.png` at the top, `mouse.png` at (0, 512)).
    pub fn set_skin(&mut self, rgba: Vec<u8>) {
        if self.skin_raster != 0 {
            render::destroy_raster(self.skin_raster);
        }
        self.skin_raster = 0;
        self.skin_failed = false;
        self.skin_image = Some(rgba);
    }

    /// Whether the skin texture is on the GPU.
    pub fn skin_uploaded(&self) -> bool {
        self.skin_raster != 0
    }

    /// Uses `ttf` (the user's own Arial Bold, as samp's D3DX "Arial") for the UI fonts instead of the committed
    /// Liberation Sans Bold. Ignored when it does not parse.
    pub fn set_font(&mut self, ttf: Vec<u8>) {
        if sa_samp_ui::font::parses(&ttf) {
            self.font_file = Some(ttf);
            self.atlas = None;
        }
    }

    /// Whether the glyph atlas is on the GPU (otherwise UI text falls back to the game font).
    pub fn atlas_uploaded(&self) -> bool {
        self.raster != 0
    }

    /// Sprite textdraw textures loaded so far.
    pub fn sprites_loaded(&self) -> usize {
        self.sprites.count()
    }
}

impl<V: View> NativeGame<V> {
    /// The overlay (UI state and engine resources).
    pub fn overlay(&mut self) -> &mut Overlay {
        &mut self.overlay
    }

    /// Draws the UI (called by [`NativeGame::dispatch_frame`]) where samp draws it: at `After2d` the queued
    /// big messages go to `CMessages` and the textdraws are printed (their text joins the HUD's in `CFont`'s
    /// buffer, flushed by `Game::Idle`); at `Present` (the end of the frame, over the game text and the fade)
    /// everything else. Nothing while the pause menu is up (samp draws no UI then).
    pub(crate) fn overlay_frame(&mut self, phase: FramePhase) {
        use sa_samp_api::hud::Hud as _;
        if self.menu_active() || !matches!(phase, FramePhase::After2d | FramePhase::Present) {
            return;
        }
        let mut ov = std::mem::take(&mut self.overlay);
        let (w, h) = render::screen_size();
        if phase == FramePhase::After2d {
            for t in ov.ui.take_game_texts() {
                ov.big.show(&t.text, t.time_ms, t.style);
            }
        }
        let heights = heights_for_width(w as u32, ov.ui.options().font_size);
        if ov.atlas.as_ref().is_none_or(|a| a.heights() != heights) {
            ov.atlas = Some(match &ov.font_file {
                Some(ttf) => FontAtlas::build_from(ttf, heights),
                None => FontAtlas::build(heights),
            });
            if ov.raster != 0 {
                render::destroy_raster(ov.raster);
            }
            ov.raster = 0;
            ov.upload_failed = false;
        }
        let atlas = ov.atlas.as_ref().expect("built above");
        if ov.raster == 0 && !ov.upload_failed {
            match render::upload_atlas(atlas) {
                Some(r) => ov.raster = r,
                None => ov.upload_failed = true,
            }
        }
        if ov.skin_raster == 0
            && !ov.skin_failed
            && let Some(img) = &ov.skin_image
        {
            use sa_samp_ui::skin::{TEXTURE_H, TEXTURE_W};
            match render::upload_rgba(TEXTURE_W, TEXTURE_H, img, true) {
                Some(r) => ov.skin_raster = r,
                None => ov.skin_failed = true,
            }
        }
        ov.ui.set_skin(ov.skin_raster != 0);
        if phase == FramePhase::After2d {
            let list = ov.ui.build_textdraws(w, h);
            render::draw(&list, atlas, ov.raster, ov.skin_raster, &mut ov.sprites, false);
        } else {
            let now = self.game_time_ms();
            let list = ov.ui.build(atlas, &EngineWorld { game: self }, w, h, now);
            render::draw(&list, atlas, ov.raster, ov.skin_raster, &mut ov.sprites, true);
            ov.frames += 1;
        }
        self.overlay = ov;
    }

    /// A window event from the platform: the UI first (chat input, dialogs, selection), then the client's
    /// `GameHooks::window_event`. Returns whether the game may also see it.
    pub fn window_event(&mut self, e: WindowEvent) -> EventResult {
        use sa_samp_api::ui::SampUi as _;
        if self.overlay.ui.handle_event(e) == EventResult::Consumed {
            return EventResult::Consumed;
        }
        match self.hooks.take() {
            Some(mut hooks) => {
                let r = hooks.window_event(e, self);
                self.hooks = Some(hooks);
                r
            }
            None => EventResult::PassToGame,
        }
    }

    /// samp's input lock while the UI is up: (keyboard, mouse).
    pub fn ui_input_lock(&self) -> (bool, bool) {
        use sa_samp_api::ui::SampUi as _;
        let keyboard = self.overlay.ui.wants_keyboard();
        (keyboard, keyboard || self.overlay.ui.locks_mouse())
    }

    /// Whether the SA:MP UI shows its cursor (a dialog, the scoreboard, textdraw selection, the class-selection
    /// panel or a cursor mode): the host then gives the pointer back to the UI (absolute moves and clicks).
    pub fn ui_cursor_visible(&self) -> bool {
        self.overlay.ui.cursor_visible()
    }

    /// Live peds within `radius` of `centre` (pool order), for nametag demos and tests.
    pub fn peds_near(&self, centre: Vec3, radius: f32) -> Vec<PedRef> {
        let v = &self.view;
        let pool = v.read_u32(addr::PED_POOL.global);
        if pool == 0 {
            return Vec::new();
        }
        let (objects, map, size) = (v.read_u32(pool), v.read_u32(pool + 4), v.read_i32(pool + 8));
        let mut out = Vec::new();
        for slot in 0..size.max(0) as u32 {
            let b = v.read_u8(map + slot);
            if b & sa_core::pool::FREE != 0 {
                continue;
            }
            let ped = PedRef((slot << 8) | u32::from(b));
            let e = objects + slot * addr::PED_POOL.element;
            if v.read_u32(e + off::VTABLE) == 0 {
                continue;
            }
            if let Ok(s) = self.ped_snapshot(ped) {
                let p = s.matrix.pos;
                let d =
                    ((p.x - centre.x).powi(2) + (p.y - centre.y).powi(2) + (p.z - centre.z).powi(2)).sqrt();
                if d <= radius {
                    out.push(ped);
                }
            }
        }
        out
    }
}
