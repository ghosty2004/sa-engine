//! The per-frame UI pass and the mouse routing around [`UiCore`], independent of the platform: the dialog layout
//! cache, the chat input's first visible character, the click rectangles, the Present draw through a
//! [`UiBackend`], the class-selection panel and the clicks on dialogs, the scoreboard, textdraws and the panel
//! (DXUT's press / release and double click). Both hosts use it: the Windows glue (D3D9 backend) and sa-engine's
//! native client (engine backend). Spec: docs/re/subsystems/ui-rendering.md, "2.3 Present", "4.2", "6.3".

use sa_samp_api::ui::UiBackend;
use samp_ui::chat::DisplayMode;
use samp_ui::classpanel::{self, ClassButton};
use samp_ui::dialog::{Button, DialogLayout};
use samp_ui::keys::Overlay;

use super::core::{Outgoing, UiCore};
use super::render::{self, BubbleDraw, CursorReq, Frame, LabelDraw, Measure, PassState, ScoreboardDraw, TagDraw};

/// What the host gathered for one Present pass.
pub struct PassInput<'a> {
    /// F5 / F10 held.
    pub overlay: Option<Overlay>,
    /// Nametags to consider.
    pub tags: &'a [TagDraw],
    /// Chat bubbles to draw (anchors raised).
    pub bubbles: &'a [BubbleDraw],
    /// 3D text labels in range.
    pub labels: &'a [LabelDraw],
    /// `/dl` vehicle labels (`None`: off).
    pub vehicle_labels: Option<&'a [([f32; 3], String)]>,
    /// Audio stream title line.
    pub audio_title: Option<&'a [u8]>,
    /// The F5 text.
    pub net_stats: Option<&'a [u8]>,
    /// samp's netgame object exists.
    pub netgame_object: bool,
    /// The cursor in client coordinates.
    pub cursor: (i32, i32),
}

impl PassInput<'_> {
    /// The scoreboard or an F5 / F10 overlay hides the rest of the UI.
    pub fn sb_open(&self, core: &UiCore) -> bool {
        core.scoreboard.is_open() || self.overlay.is_some()
    }
}

/// UI state the pass keeps between frames.
#[derive(Default)]
pub struct UiSession {
    /// Cached dialog layout (computed once per open, like the original): dialog serial and layout.
    dialog_layout: Option<(u64, DialogLayout)>,
    input_first_visible: usize,
    input_caret: Option<(usize, usize)>,
    /// Class-selection panel buttons (`<<`, `>>`, `Spawn`) drawn this frame.
    class_buttons: Option<[samp_ui::Rect; 3]>,
    /// The class-selection panel was made visible. samp's class-selection step shows it on the first frame without
    /// an open dialog and keeps it until the spawn.
    pub class_panel_shown: bool,
    class_click: Option<ClassButton>,
    /// Last click on a dialog list row (row, time in ms; double-click detection).
    last_list_click: Option<(usize, u64)>,
    /// Last scoreboard row click.
    last_scoreboard_click: Option<(usize, u64)>,
    pass: PassState,
}

impl UiSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fonts were rebuilt: the dialog layout is measured again.
    pub fn fonts_changed(&mut self) {
        self.dialog_layout = None;
    }

    /// The dialog layout once per open, the list's page size, the chat input's first visible character (DXUT
    /// `PlaceCaret` on caret moves) and the click rects of the input and the scoreboard.
    pub fn update_layouts(&mut self, core: &mut UiCore, b: &dyn UiBackend) {
        let serial = core.dialog_serial;
        if let Some(d) = core.dialogs.current() {
            if self.dialog_layout.as_ref().is_none_or(|(s, _)| *s != serial) {
                let (cw, ch) = b.client_size();
                self.dialog_layout = Some((serial, d.layout(&Measure(b), cw, ch)));
            }
            if let Some((_, l)) = &self.dialog_layout
                && let Some(list) = l.list
            {
                core.dialog_page_rows = d.page_rows(&list, l.list_row_h);
            }
        }
        let m = b.metrics();
        if core.input.is_open() {
            let key = (core.input.edit().caret(), core.input.edit().text().len());
            if self.input_caret != Some(key) {
                self.input_caret = Some(key);
                self.input_first_visible =
                    render::input_first_visible(b, &core.chat, &core.input, self.input_first_visible);
            }
            let (cw, _) = b.client_size();
            core.input_rect = samp_ui::input::box_rect(cw, core.chat.bottom(m.main_h), m.gui0_h);
        } else {
            self.input_caret = None;
        }
        let (w, h) = b.video_size();
        let (sw, shh) = samp_ui::scoreboard::size(w);
        core.scoreboard_rect = samp_ui::Rect::xywh(w / 2 - sw / 2, h / 2 - shh / 2, sw, shh);
    }

    /// The HUD and radar are on unless the scoreboard, an F5 / F10 overlay or the hidden chat mode hides them
    /// (samp writes the flags on every Present).
    pub fn hud_visible(core: &UiCore, overlay: Option<Overlay>) -> bool {
        !core.scoreboard.is_open() && overlay.is_none() && core.chat.mode() != DisplayMode::Hidden
    }

    /// Steps 4–12 of samp's Present pass: the renderer's frame (chat, nametags, bubbles, scoreboard, death
    /// window, dialog, overlays), then the class-selection panel after the chat GUI dialog.
    pub fn draw(
        &mut self,
        core: &UiCore,
        b: &mut dyn UiBackend,
        p: &PassInput,
        cursor_req: &mut dyn FnMut(CursorReq),
    ) {
        let sb_open = p.sb_open(core);
        let sb_draw = core.scoreboard.is_open().then(|| ScoreboardDraw {
            rows: core.scoreboard.rows(),
            server_name: &core.server_name,
            players: samp_ui::scoreboard::players_text(core.remote_ids().count()),
            selected: core.scoreboard.selected(),
            scroll: core.scoreboard.scroll(),
        });
        let dialog = if core.dialogs.drawn(core.input.is_open(), sb_open) {
            core.dialogs.current().zip(self.dialog_layout.as_ref().map(|(_, l)| l))
        } else {
            None
        };
        let frame = Frame {
            chat: &core.chat,
            input: &core.input,
            input_first_visible: self.input_first_visible,
            tags: p.tags,
            nametag_status: core.settings.nametag_status,
            bubbles: p.bubbles,
            labels: p.labels,
            vehicle_labels: p.vehicle_labels,
            scoreboard: sb_draw,
            death: core.deaths.visible().then_some(&core.deaths),
            dialog,
            cursor: p.cursor,
            overlay: p.overlay,
            net_stats: p.net_stats,
            netgame_object: core.netgame && p.netgame_object,
            name_tag_passes: core.netgame && p.netgame_object && core.show_name_tags,
            audio_title: p.audio_title,
        };
        render::frame(b, &mut self.pass, &frame, cursor_req);
        // Step 12: the class-selection GUI dialog after the chat GUI dialog (spec "4.2 GUI dialogs created by
        // samp"); class selection holds cursor mode 2 meanwhile.
        self.class_buttons = (core.class_selection && self.class_panel_shown && !sb_open)
            .then(|| render::class_panel(b, p.cursor));
    }

    /// Whether a UI that holds cursor mode 2 is up (the chat input, class selection, a dialog, textdraw selection).
    pub fn wants_mode2(core: &UiCore) -> bool {
        core.input.is_open() || core.class_selection || core.dialogs.is_open() || core.textdraws.selection.active
    }

    /// Whether the UI shows its cursor and owns the mouse.
    pub fn cursor_shown(&self, core: &UiCore) -> bool {
        core.dialogs.is_open()
            || core.scoreboard.is_open()
            || core.textdraws.selection.active
            || self.class_buttons.is_some()
            || core.input.is_open()
    }

    /// Takes the pending class-selection panel click (the class-selection tick).
    pub fn take_class_click(&mut self) -> Option<ClassButton> {
        self.class_click.take()
    }

    /// Left button released at `(x, y)` over a hovered textdraw, the scoreboard, the class-selection panel or a
    /// dialog control (the GUI dialogs' `MsgProc`, the selection handler). `now_ms` is a millisecond clock,
    /// `double_click_ms` the system double-click time. Returns whether the click was consumed.
    pub fn mouse_up(
        &mut self,
        core: &mut UiCore,
        b: &dyn UiBackend,
        x: i32,
        y: i32,
        now_ms: u64,
        double_click_ms: u32,
    ) -> bool {
        if core.textdraws.selection.active
            && let Some(id) = core.textdraws.selection.hovered
        {
            // RPC 83 OnPlayerSelectTextDraw with the hovered id; selection stays on.
            core.push_outgoing(Outgoing::SelectTextDraw(id));
            return true;
        }
        if core.scoreboard.is_open() {
            return self.scoreboard_click(core, b, x, y, now_ms, double_click_ms);
        }
        if let Some(rects) = &self.class_buttons
            && let Some(c) = classpanel::hit(rects, x, y)
        {
            // `<<` previous class, `>>` next class, `Spawn` (taken by the class-selection tick).
            self.class_click = Some(c);
            return true;
        }
        if !core.dialogs.drawn(core.input.is_open(), core.scoreboard.is_open()) {
            return false;
        }
        let Some((_, l)) = self.dialog_layout.clone() else { return false };
        if l.button1.contains(x, y) {
            core.dialog_respond(Button::Ok);
            return true;
        }
        if l.button2.is_some_and(|r| r.contains(x, y)) {
            core.dialog_respond(Button::Cancel);
            return true;
        }
        if let (Some(list), Some(d)) = (l.list, core.dialogs.current_mut()) {
            if let Some(row) = d.row_at(&list, l.list_row_h, x, y) {
                // A click selects; a double click (two clicks on the same row within the double-click time,
                // DXUT's WM_LBUTTONDBLCLK) activates with response 1.
                let double = self
                    .last_list_click
                    .is_some_and(|(r, t)| r == row && now_ms.saturating_sub(t) <= u64::from(double_click_ms));
                d.select(row);
                if double {
                    self.last_list_click = None;
                    core.dialog_respond(Button::Ok);
                } else {
                    self.last_list_click = Some((row, now_ms));
                }
            }
            return list.contains(x, y);
        }
        l.rect.contains(x, y)
    }

    /// A click on the scoreboard's list box: a click selects the row; a double click activates it, and the
    /// scoreboard's GUI callback sends RPC 23 OnPlayerClickPlayer and closes. Clicks outside the scoreboard fall
    /// through (they close it, see `samp_ui::keys`).
    fn scoreboard_click(
        &mut self,
        core: &mut UiCore,
        b: &dyn UiBackend,
        x: i32,
        y: i32,
        now_ms: u64,
        double_click_ms: u32,
    ) -> bool {
        let r = core.scoreboard_rect;
        if !r.contains(x, y) {
            return false;
        }
        let row_h = b.metrics().gui1_h;
        let list = samp_ui::Rect::new(r.left, r.top + 60, r.right, r.bottom);
        let sel = samp_ui::dialog::list_selection_rect(&list);
        let text = samp_ui::dialog::list_text_rect(&list);
        if row_h <= 0 || x < sel.left || x >= sel.right || y < text.top || y >= sel.bottom {
            return true;
        }
        let row = core.scoreboard.scroll() + ((y - text.top) / row_h) as usize;
        if row >= core.scoreboard.rows().len() {
            return true;
        }
        let double = self
            .last_scoreboard_click
            .is_some_and(|(r, t)| r == row && now_ms.saturating_sub(t) <= u64::from(double_click_ms));
        core.scoreboard.set_selected(Some(row));
        if double {
            self.last_scoreboard_click = None;
            core.scoreboard_activate();
        } else {
            self.last_scoreboard_click = Some((row, now_ms));
        }
        true
    }
}
