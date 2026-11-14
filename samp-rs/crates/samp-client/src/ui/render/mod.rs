//! The UI pass, written once and drawn through [`UiBackend`] (Direct3D 9 on the original exe, the ported
//! engine natively): nametags, 3D labels, chat, dialogs, scoreboard, death window, the chat GUI dialog, the
//! class-selection panel and the textdraws, in samp's order and with its draw pattern (so the D3D9 backend's
//! call stream matches the original's). Spec: docs/re/subsystems/ui-rendering.md; measured call order in
//! docs/phase3/m8a-notes.md; the backend split in sa-engine's docs/samp/overlay.md §10.

use sa_samp_api::ui::{UiBackend, UiFont, UiRect, UiSprite, dt};
use samp_ui::chat::ChatWindow;
use samp_ui::colour;
use samp_ui::deathwindow::DeathWindow;
use samp_ui::dialog::{Dialog, DialogLayout};
use samp_ui::fonts::{FontId, Size, TextMeasure};
use samp_ui::input::ChatInput;
use samp_ui::scoreboard::Row;

pub mod chat;
mod gui;
mod panels;
pub mod text;
pub mod textdraws;
mod world;

pub use chat::{chat_lines, class_panel, input_first_visible};

/// `D3DXSPRITE_ALPHABLEND`.
pub const SPRITE_ALPHABLEND: u32 = 0x10;
/// `D3DXSPRITE_DONOTSAVESTATE`.
pub const SPRITE_DONOTSAVESTATE: u32 = 0x1;

/// samp-ui's rectangle as the backend's.
pub fn ui_rect(r: samp_ui::Rect) -> UiRect {
    UiRect::new(r.left, r.top, r.right, r.bottom)
}

/// samp-ui's font ids as the backend's fonts.
pub fn ui_font(f: FontId) -> UiFont {
    match f {
        FontId::Main => UiFont::Main,
        FontId::Small => UiFont::Small,
        FontId::Gui0 => UiFont::Gui0,
        FontId::Gui1 => UiFont::Gui1,
        FontId::DeathNames => UiFont::DeathNames,
        FontId::DeathIcon => UiFont::DeathIcon,
    }
}

/// samp-ui's text metrics from a backend: single lines are `DT_CALCRECT | DT_SINGLELINE` measures, line heights
/// the metrics samp measured when it created the fonts, blocks `DT_CALCRECT | DT_EXPANDTABS`.
pub struct Measure<'a>(pub &'a dyn UiBackend);

impl TextMeasure for Measure<'_> {
    fn measure(&self, font: FontId, text: &[u8]) -> Size {
        if text.is_empty() {
            return Size::default();
        }
        let r = self.0.measure(ui_font(font), text, text.len() as i32, UiRect::default(), dt::SINGLELINE | dt::CALCRECT, 0xFF00_0000);
        Size { w: r.right - r.left, h: r.bottom - r.top }
    }
    fn timestamp_width(&self) -> i32 {
        self.0.metrics().timestamp_w
    }
    fn line_height(&self, font: FontId) -> i32 {
        let m = self.0.metrics();
        match font {
            FontId::Main => m.main_h,
            FontId::Small => m.small_h,
            FontId::Gui0 => m.gui0_h,
            FontId::Gui1 => m.gui1_h,
            f => self.measure(f, b"Y").h,
        }
    }
    fn measure_block(&self, font: FontId, text: &[u8]) -> Size {
        if text.is_empty() {
            return Size::default();
        }
        let r = self.0.measure(ui_font(font), text, -1, UiRect::default(), dt::CALCRECT | dt::EXPANDTABS, 0xFF00_0000);
        Size { w: r.right - r.left, h: r.bottom - r.top }
    }
}

/// The font renderer's outlined draw (outline passes with the outline twin, then the colour-code pass): `text`
/// keeps its `{RRGGBB}` codes.
#[allow(clippy::too_many_arguments)]
pub fn outlined(
    b: &mut dyn UiBackend,
    sprite: UiSprite,
    font: UiFont,
    outline_font: UiFont,
    text: &[u8],
    rect: UiRect,
    flags: u32,
    colour: u32,
    outline: &[(i32, i32)],
) {
    let stripped = colour::strip_codes(text);
    let glyphs: Vec<u32> = colour::glyph_colours(text, colour).iter().map(|g| g.colour).collect();
    b.draw_text_outlined(sprite, font, outline_font, &stripped, rect, flags, colour, &glyphs, outline);
}

/// One nametag to consider this frame.
#[derive(Clone, Debug)]
pub struct TagDraw {
    /// Head bone position (bone 8), world coordinates.
    pub anchor: [f32; 3],
    /// Distance to the camera.
    pub distance: f32,
    pub name: Vec<u8>,
    /// ARGB, alpha forced FF.
    pub colour: u32,
    pub health: f32,
    pub armour: f32,
    pub afk: bool,
}

/// One chat bubble to draw (anchor already raised).
#[derive(Clone, Debug)]
pub struct BubbleDraw {
    pub anchor: [f32; 3],
    pub text: Vec<u8>,
    pub colour: u32,
}

/// One 3D text label to draw this frame.
#[derive(Clone, Debug)]
pub struct LabelDraw {
    /// World position (projected in the Present pass like samp's 3D-text painter).
    pub pos: [f32; 3],
    pub text: Vec<u8>,
    pub argb: u32,
}

/// Scoreboard contents.
pub struct ScoreboardDraw<'a> {
    pub rows: &'a [Row],
    pub server_name: &'a [u8],
    pub players: Vec<u8>,
    pub selected: Option<usize>,
    pub scroll: usize,
}

/// Cursor-mode requests made by the pass (executed by the caller at that point).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorReq {
    Mode2,
}

/// Everything one Present draws.
pub struct Frame<'a> {
    pub chat: &'a ChatWindow,
    pub input: &'a ChatInput,
    pub input_first_visible: usize,
    pub tags: &'a [TagDraw],
    pub nametag_status: bool,
    pub bubbles: &'a [BubbleDraw],
    pub labels: &'a [LabelDraw],
    /// `/dl` vehicle labels (`None`: the flag is off, no batch at all).
    pub vehicle_labels: Option<&'a [([f32; 3], String)]>,
    pub scoreboard: Option<ScoreboardDraw<'a>>,
    pub death: Option<&'a DeathWindow>,
    pub dialog: Option<(&'a Dialog, &'a DialogLayout)>,
    pub cursor: (i32, i32),
    /// F5 / F10 held: the passes after the nametags are skipped; F5 draws its net stats text instead.
    pub overlay: Option<samp_ui::keys::Overlay>,
    /// F5 text (`net_stats_text`) when the net stats overlay is shown.
    pub net_stats: Option<&'a [u8]>,
    /// samp's netgame object exists (created on the game frame after the UI objects): the
    /// bubble and 3D-label passes run.
    pub netgame_object: bool,
    /// Netgame exists and the server's show-nametags setting is on (the guards of samp's bars and names
    /// passes): both nametag passes run, even with nobody to draw.
    pub name_tag_passes: bool,
    /// Audio stream title line, when the netgame exists and a title is known.
    pub audio_title: Option<&'a [u8]>,
}

/// State the pass keeps between frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PassState {
    /// Size of the chat GUI dialog: the video mode, then `(w + 100) × (bottom + 50)` once the input was opened
    /// (ui-rendering.md, "Chat input"); `None` until the first frame.
    pub host_size: Option<(i32, i32)>,
}

/// The Present pass after the HUD decision, in samp's order: nametag bars and names, audio title, `/dl`
/// labels; then either the scoreboard or the F5 / F10 overlay (and stop), or chat bubbles, 3D labels, chat,
/// death window, dialog and the chat GUI dialog (ui-rendering.md, "Present: per-frame draw order").
pub fn frame(b: &mut dyn UiBackend, st: &mut PassState, f: &Frame, cursor_req: &mut dyn FnMut(CursorReq)) {
    if f.name_tag_passes {
        world::tag_bars(b, f.tags);
        world::tag_names(b, f.tags, f.nametag_status);
    }
    if let Some(t) = f.audio_title {
        text::audio_title(b, t);
    }
    if let Some(rows) = f.vehicle_labels {
        world::vehicle_labels(b, rows);
    }
    if let Some(sb) = &f.scoreboard {
        panels::scoreboard(b, sb);
        return;
    }
    if f.overlay.is_some() {
        // F10's draw is empty.
        if let Some(t) = f.net_stats {
            text::net_stats(b, t);
        }
        return;
    }
    if f.netgame_object {
        world::bubbles(b, f.bubbles);
        // 3D text labels: the text3d batch, empty when there are none (like the original).
        world::labels(b, f.labels);
    }
    chat::chat(b, f.chat);
    if f.input.is_open() {
        cursor_req(CursorReq::Mode2);
    }
    if let Some(dw) = f.death {
        panels::death_window(b, dw);
    }
    if let Some((d, l)) = f.dialog {
        cursor_req(CursorReq::Mode2);
        panels::dialog(b, d, l, f.cursor);
    }
    chat::chat_gui(b, st, f.chat, f.input, f.input_first_visible);
}
