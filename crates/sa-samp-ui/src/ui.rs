//! [`Ui`]: the whole engine-drawn SA:MP UI. Implements `sa_samp_api::ui::SampUi` (samp-rs feeds it and
//! drains events), turns window events into chat / dialog / selection input, and builds one [`DrawList`]
//! per frame.

use std::collections::BTreeMap;

use sa_samp_api::Rgba;
use sa_samp_api::input::{EventResult, WindowEvent};
use sa_samp_api::ui::{
    ChatBubble, ChatMessage, CursorMode, DialogSpec, Nametag, SampUi, ScoreRow, TextDrawSpec, TextLabel3d,
    UiEvent, UiOptions,
};

use crate::chat::{ChatInput, ChatWindow, PAGE_SIZE};
use crate::dialog::Dialog;
use crate::draw::DrawList;
use crate::edit::EditKey;
use crate::font::{FontId, Measure};
use crate::panels::{KillList, KillRow, Scoreboard, draw_cursor};
use crate::textdraw::TextDraws;
use crate::world::{BUBBLE_MAX_LEN, Bubble, NametagRules, bubble_lines, draw_bubbles};
use crate::world::{WorldView, draw_labels, draw_nametags};

/// Windows virtual-key codes the UI reacts to.
pub mod vk {
    /// Backspace.
    pub const BACK: u16 = 0x08;
    /// Tab.
    pub const TAB: u16 = 0x09;
    /// Enter.
    pub const RETURN: u16 = 0x0D;
    /// Escape.
    pub const ESCAPE: u16 = 0x1B;
    /// Page Up.
    pub const PRIOR: u16 = 0x21;
    /// Page Down.
    pub const NEXT: u16 = 0x22;
    /// End.
    pub const END: u16 = 0x23;
    /// Home.
    pub const HOME: u16 = 0x24;
    /// Left.
    pub const LEFT: u16 = 0x25;
    /// Up.
    pub const UP: u16 = 0x26;
    /// Right.
    pub const RIGHT: u16 = 0x27;
    /// Down.
    pub const DOWN: u16 = 0x28;
    /// Delete.
    pub const DELETE: u16 = 0x2E;
    /// T.
    pub const T: u16 = 0x54;
    /// F6.
    pub const F6: u16 = 0x75;
    /// F7.
    pub const F7: u16 = 0x76;
}

/// A big message (`GameTextForPlayer`) for the engine's `CMessages::AddBigMessage`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameText {
    /// Text.
    pub text: String,
    /// Duration.
    pub time_ms: u32,
    /// Style 0..=6.
    pub style: u8,
}

/// The UI state.
#[derive(Debug, Default)]
pub struct Ui {
    /// Chat window.
    pub chat: ChatWindow,
    /// Chat input line.
    pub input: ChatInput,
    dialog: Option<Dialog>,
    nametags: Vec<Nametag>,
    labels: BTreeMap<u16, TextLabel3d>,
    bubbles: BTreeMap<u16, Bubble>,
    /// Textdraws.
    pub textdraws: TextDraws,
    /// Kill list.
    pub kills: KillList,
    scoreboard: Option<Scoreboard>,
    cursor_mode: CursorMode,
    cursor: (f32, f32),
    events: Vec<UiEvent>,
    game_texts: Vec<GameText>,
    swallow_char: bool,
    time_ms: u32,
    /// Screen size of the last frame (for mouse hit tests between frames).
    screen: (f32, f32),
    measure_cache: Option<Box<dyn MeasureSnapshot>>,
    options: UiOptions,
}

/// Metrics kept between frames for mouse hit tests (dialog and scoreboard geometry).
trait MeasureSnapshot: std::fmt::Debug {
    fn width(&self, font: FontId, text: &str) -> f32;
    fn line_height(&self, font: FontId) -> f32;
}

/// Fixed metrics captured from the frame's [`Measure`]: the per-character widths of the charset.
#[derive(Debug)]
struct Snapshot {
    widths: [std::collections::HashMap<char, f32>; 4],
    heights: [f32; 4],
}

impl MeasureSnapshot for Snapshot {
    fn width(&self, font: FontId, text: &str) -> f32 {
        text.chars()
            .map(|c| self.widths[font.index()].get(&c).copied().unwrap_or(0.0))
            .sum()
    }
    fn line_height(&self, font: FontId) -> f32 {
        self.heights[font.index()]
    }
}

impl Measure for Box<dyn MeasureSnapshot> {
    fn width(&self, font: FontId, text: &str) -> f32 {
        self.as_ref().width(font, text)
    }
    fn line_height(&self, font: FontId) -> f32 {
        self.as_ref().line_height(font)
    }
}

fn snapshot(m: &dyn Measure) -> Box<dyn MeasureSnapshot> {
    let chars: Vec<char> = (' '..='~').chain('\u{A1}'..='\u{FF}').collect();
    let widths = FontId::ALL.map(|f| chars.iter().map(|&c| (c, m.width(f, &c.to_string()))).collect());
    Box::new(Snapshot {
        widths,
        heights: FontId::ALL.map(|f| m.line_height(f)),
    })
}

fn char_of(ch: u32, unicode: bool) -> Option<char> {
    if unicode {
        char::from_u32(ch)
    } else {
        u8::try_from(ch).ok().map(char::from)
    }
}

impl Ui {
    /// A UI with nothing shown.
    pub fn new() -> Self {
        Self::default()
    }

    /// The open dialog.
    pub fn dialog(&self) -> Option<&Dialog> {
        self.dialog.as_ref()
    }

    /// The open dialog, mutable.
    pub fn dialog_mut(&mut self) -> Option<&mut Dialog> {
        self.dialog.as_mut()
    }

    /// Cursor position (screen pixels).
    pub fn cursor(&self) -> (f32, f32) {
        self.cursor
    }

    /// Whether the cursor is drawn: a dialog, the scoreboard, textdraw selection or a cursor mode.
    pub fn cursor_visible(&self) -> bool {
        self.dialog.is_some()
            || self.scoreboard.is_some()
            || self.textdraws.selecting.is_some()
            || self.cursor_mode != CursorMode::Off
    }

    /// Whether the mouse belongs to the UI (the camera must not turn): a dialog, the scoreboard, textdraw
    /// selection or the locked cursor mode. The free cursor mode leaves the mouse to the game.
    pub fn locks_mouse(&self) -> bool {
        self.dialog.is_some()
            || self.scoreboard.is_some()
            || self.textdraws.selecting.is_some()
            || self.cursor_mode == CursorMode::Locked
    }

    /// The display options (the renderer sizes the fonts with `font_size`).
    pub fn options(&self) -> &UiOptions {
        &self.options
    }

    /// Big messages queued for the engine since the last call.
    pub fn take_game_texts(&mut self) -> Vec<GameText> {
        std::mem::take(&mut self.game_texts)
    }

    fn dialog_key(&mut self, vk: u16) {
        let Some(d) = self.dialog.as_mut() else { return };
        let page = match self.measure_cache.as_ref() {
            Some(m) => Dialog::page_rows(&d.layout(m, self.screen.0, self.screen.1)),
            None => PAGE_SIZE,
        };
        match vk {
            vk::RETURN => {
                let r = d.response(1);
                self.events.push(r);
                self.dialog = None;
            }
            vk::ESCAPE => {
                let r = d.response(0);
                self.events.push(r);
                self.dialog = None;
            }
            vk::UP => d.list_move(-1, page),
            vk::DOWN => d.list_move(1, page),
            vk::PRIOR => d.list_move(-(page as isize), page),
            vk::NEXT => d.list_move(page as isize, page),
            vk::HOME => {
                d.list_move(isize::MIN, page);
                d.edit_key(EditKey::Home);
            }
            vk::END => {
                d.list_move(isize::MAX, page);
                d.edit_key(EditKey::End);
            }
            vk::LEFT => d.edit_key(EditKey::Left),
            vk::RIGHT => d.edit_key(EditKey::Right),
            vk::BACK => d.edit_key(EditKey::Backspace),
            vk::DELETE => d.edit_key(EditKey::Delete),
            _ => {}
        }
    }

    fn input_key(&mut self, vk: u16) {
        match vk {
            vk::RETURN => {
                if let Some(t) = self.input.submit() {
                    self.events.push(UiEvent::ChatSubmit(t));
                }
            }
            vk::ESCAPE => self.input.close(),
            vk::UP => self.input.history_older(),
            vk::DOWN => self.input.history_newer(),
            vk::PRIOR => self.chat.scroll_by(self.chat.page_size() as isize),
            vk::NEXT => self.chat.scroll_by(-(self.chat.page_size() as isize)),
            vk::LEFT => self.input.edit_mut().key(EditKey::Left),
            vk::RIGHT => self.input.edit_mut().key(EditKey::Right),
            vk::HOME => self.input.edit_mut().key(EditKey::Home),
            vk::END => self.input.edit_mut().key(EditKey::End),
            vk::BACK => self.input.edit_mut().key(EditKey::Backspace),
            vk::DELETE => self.input.edit_mut().key(EditKey::Delete),
            _ => {}
        }
    }

    fn click(&mut self) {
        let (x, y) = self.cursor;
        if let (Some(d), Some(m)) = (self.dialog.as_mut(), self.measure_cache.as_ref()) {
            let l = d.layout(m, self.screen.0, self.screen.1);
            if let Some(r) = d.click(&l, x, y, self.time_ms) {
                self.events.push(r);
                self.dialog = None;
            }
            return;
        }
        if let (Some(sb), Some(m)) = (self.scoreboard.as_ref(), self.measure_cache.as_ref())
            && let Some(id) = sb.hit(m, self.screen.0, self.screen.1, x, y)
        {
            self.events.push(UiEvent::ScoreboardClicked(id));
            return;
        }
        if self.textdraws.selecting.is_some()
            && let Some(id) = self.textdraws.hit(x, y)
        {
            self.events.push(UiEvent::TextDrawClicked(id));
        }
    }

    /// The textdraws, drawn where samp draws them: after `Render2dStuff` (`FramePhase::After2d`), their text
    /// printed into `CFont`'s buffer with the HUD's own text.
    pub fn build_textdraws(&mut self, w: f32, h: f32) -> DrawList {
        let mut d = DrawList::default();
        let cursor = self.cursor_visible().then_some(self.cursor);
        self.textdraws.draw(w, h, cursor, &mut d);
        d
    }

    /// The rest of the frame, in samp's `Present` order (over everything, the game text and the fade
    /// included): nametags, labels, bubbles, kill list, chat, chat input, scoreboard, dialog, cursor.
    pub fn build(
        &mut self,
        m: &dyn Measure,
        world: &dyn WorldView,
        w: f32,
        h: f32,
        time_ms: u32,
    ) -> DrawList {
        self.time_ms = time_ms;
        if self.screen != (w, h) || self.measure_cache.is_none() {
            self.measure_cache = Some(snapshot(m));
        }
        self.screen = (w, h);
        let mut d = DrawList::default();
        let cursor = self.cursor_visible().then_some(self.cursor);
        let rules = NametagRules {
            show: self.options.nametags,
            distance: self.options.nametag_distance,
            los: self.options.nametag_los,
        };
        draw_nametags(&self.nametags, rules, world, m, &mut d);
        draw_labels(self.labels.values(), world, m, &mut d);
        self.bubbles.retain(|_, b| !b.expired(time_ms));
        draw_bubbles(self.bubbles.values(), world, m, &mut d);
        self.kills.draw(m, w, h, &mut d);
        self.chat.draw(m, w as u32, &mut d);
        let bottom = self.chat.bottom(m.line_height(FontId::Main));
        self.chat.draw_scrollbar(m, self.input.is_open(), &mut d);
        self.input.draw(m, w as u32, bottom, time_ms, &mut d);
        if let Some(sb) = &self.scoreboard {
            sb.draw(m, w, h, cursor, &mut d);
        }
        if let Some(dl) = &self.dialog {
            dl.draw(m, w, h, cursor, time_ms, &mut d);
        }
        if let Some((x, y)) = cursor {
            draw_cursor(x, y, &mut d);
        }
        d
    }
}

impl SampUi for Ui {
    fn chat_add(&mut self, msg: &ChatMessage) {
        self.chat.add(msg);
    }

    fn chat_clear(&mut self) {
        self.chat.clear();
    }

    fn show_dialog(&mut self, dialog: Option<&DialogSpec>) {
        self.dialog = dialog.filter(|d| d.id >= 0).map(Dialog::new);
        if self.dialog.is_some() {
            self.input.close();
        }
    }

    fn set_nametags(&mut self, tags: &[Nametag]) {
        self.nametags = tags.to_vec();
    }

    fn set_chat_bubble(&mut self, player_id: u16, bubble: Option<&ChatBubble>) {
        match bubble {
            Some(b) if b.text.chars().count() <= BUBBLE_MAX_LEN => {
                let lines = bubble_lines(&crate::text::replace_control_chars(&b.text));
                self.bubbles.insert(
                    player_id,
                    Bubble {
                        spec: b.clone(),
                        lines,
                        start_ms: self.time_ms,
                    },
                );
            }
            Some(_) => {}
            None => {
                self.bubbles.remove(&player_id);
            }
        }
    }

    fn set_label(&mut self, id: u16, label: Option<&TextLabel3d>) {
        match label {
            Some(l) => {
                self.labels.insert(id, l.clone());
            }
            None => {
                self.labels.remove(&id);
            }
        }
    }

    fn set_textdraw(&mut self, id: u16, td: Option<&TextDrawSpec>) {
        self.textdraws.set(id, td);
    }

    fn game_text(&mut self, text: &str, time_ms: u32, style: u8) {
        // samp ignores empty texts, texts over 200 characters and styles GTA does not have.
        if text.is_empty() || text.chars().count() > 200 || style > 6 {
            return;
        }
        self.game_texts.push(GameText {
            text: text.into(),
            time_ms,
            style,
        });
    }

    fn kill_entry(
        &mut self,
        killer: &str,
        killer_colour: Rgba,
        victim: &str,
        victim_colour: Rgba,
        reason: u8,
    ) {
        self.kills.add(KillRow {
            killer: killer.into(),
            killer_colour: killer_colour.to_argb_u32() | 0xFF00_0000,
            victim: victim.into(),
            victim_colour: victim_colour.to_argb_u32() | 0xFF00_0000,
            reason,
        });
    }

    fn set_scoreboard(&mut self, rows: Option<(&str, &[ScoreRow])>) {
        self.scoreboard = rows.map(|(server, rows)| Scoreboard {
            server: server.into(),
            rows: rows.to_vec(),
        });
    }

    fn set_options(&mut self, options: &UiOptions) {
        self.options = *options;
        self.chat.set_page_size(usize::from(options.chat_page_size));
        self.chat.set_timestamps(options.timestamps);
    }

    fn set_cursor_mode(&mut self, mode: CursorMode) {
        self.cursor_mode = mode;
    }

    fn select_textdraws(&mut self, hover: Option<Rgba>) {
        self.textdraws.selecting = hover;
    }

    fn wants_keyboard(&self) -> bool {
        self.dialog.is_some() || self.input.is_open()
    }

    fn handle_event(&mut self, event: WindowEvent) -> EventResult {
        use EventResult::{Consumed, PassToGame};
        match event {
            WindowEvent::MouseMove { x, y } => {
                self.cursor = (x as f32, y as f32);
                PassToGame
            }
            WindowEvent::MouseButton {
                button: 0,
                down: true,
            } if self.cursor_visible() => {
                self.click();
                Consumed
            }
            WindowEvent::Wheel(delta) => {
                let notches = i32::from(delta) / 120;
                if let (Some(d), Some(m)) = (self.dialog.as_mut(), self.measure_cache.as_ref()) {
                    let page = Dialog::page_rows(&d.layout(m, self.screen.0, self.screen.1));
                    d.wheel(notches, page);
                    return Consumed;
                }
                if self.input.is_open() {
                    self.chat.scroll_by(notches as isize * 3);
                    return Consumed;
                }
                PassToGame
            }
            WindowEvent::KeyDown { vk, repeat } => {
                if self.dialog.is_some() {
                    self.dialog_key(vk);
                    return Consumed;
                }
                if self.input.is_open() {
                    self.input_key(vk);
                    return Consumed;
                }
                match vk {
                    vk::T | vk::F6 if !repeat => {
                        self.input.open();
                        // The character the key types must not land in the new line.
                        self.swallow_char = vk == vk::T;
                        Consumed
                    }
                    vk::F7 if !repeat => {
                        self.chat.cycle_mode();
                        Consumed
                    }
                    vk::PRIOR => {
                        self.chat.scroll_by(self.chat.page_size() as isize);
                        Consumed
                    }
                    vk::NEXT => {
                        self.chat.scroll_by(-(self.chat.page_size() as isize));
                        Consumed
                    }
                    vk::ESCAPE if self.textdraws.selecting.is_some() => {
                        self.textdraws.selecting = None;
                        self.events.push(UiEvent::TextDrawClicked(0xFFFF));
                        Consumed
                    }
                    _ => PassToGame,
                }
            }
            // Key releases always reach the game, so a key held when the chat line opened is not stuck.
            WindowEvent::Char { ch, unicode } => {
                if std::mem::take(&mut self.swallow_char) {
                    return Consumed;
                }
                let Some(c) = char_of(ch, unicode) else {
                    return PassToGame;
                };
                if c.is_control() {
                    return if self.wants_keyboard() {
                        Consumed
                    } else {
                        PassToGame
                    };
                }
                if let Some(d) = self.dialog.as_mut() {
                    d.char(c);
                    Consumed
                } else if self.input.is_open() {
                    self.input.edit_mut().insert(c);
                    Consumed
                } else {
                    PassToGame
                }
            }
            _ => PassToGame,
        }
    }

    fn poll_events(&mut self) -> Vec<UiEvent> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::MonoMeasure;
    use sa_samp_api::ui::DialogStyle;
    use sa_samp_api::{PedRef, Vec3};

    struct NoWorld;
    impl WorldView for NoWorld {
        fn project(&self, _p: Vec3) -> Option<(f32, f32)> {
            None
        }
        fn camera_pos(&self) -> Vec3 {
            Vec3::default()
        }
        fn ped_position(&self, _ped: PedRef) -> Option<Vec3> {
            None
        }
    }

    const M: MonoMeasure = MonoMeasure {
        char_w: 8.0,
        line_h: 18.0,
    };

    fn key(ui: &mut Ui, vk: u16) -> EventResult {
        ui.handle_event(WindowEvent::KeyDown { vk, repeat: false })
    }

    fn typed(ui: &mut Ui, s: &str) {
        for c in s.chars() {
            ui.handle_event(WindowEvent::Char {
                ch: c as u32,
                unicode: true,
            });
        }
    }

    #[test]
    fn t_opens_the_input_swallows_its_char_and_enter_sends() {
        let mut ui = Ui::new();
        assert_eq!(key(&mut ui, vk::T), EventResult::Consumed);
        assert!(ui.wants_keyboard());
        typed(&mut ui, "thello");
        assert_eq!(ui.input.edit().text(), "hello");
        key(&mut ui, vk::LEFT);
        key(&mut ui, vk::BACK);
        assert_eq!(ui.input.edit().text(), "helo");
        key(&mut ui, vk::RETURN);
        assert_eq!(ui.poll_events(), vec![UiEvent::ChatSubmit("helo".into())]);
        assert!(!ui.wants_keyboard());
        // F6 opens without swallowing; Esc closes without sending; Up recalls history.
        key(&mut ui, vk::F6);
        typed(&mut ui, "x");
        assert_eq!(ui.input.edit().text(), "x");
        key(&mut ui, vk::UP);
        assert_eq!(ui.input.edit().text(), "helo");
        key(&mut ui, vk::ESCAPE);
        assert!(ui.poll_events().is_empty() && !ui.input.is_open());
        // Closed: other keys and characters go to the game.
        assert_eq!(key(&mut ui, 0x57), EventResult::PassToGame);
        assert_eq!(
            ui.handle_event(WindowEvent::Char {
                ch: 'w' as u32,
                unicode: true
            }),
            EventResult::PassToGame
        );
    }

    #[test]
    fn dialog_takes_the_keyboard_and_answers() {
        let mut ui = Ui::new();
        ui.show_dialog(Some(&DialogSpec {
            id: 3,
            style: DialogStyle::Input,
            title: "Login".into(),
            body: "Password:".into(),
            button1: "Login".into(),
            button2: "Quit".into(),
        }));
        assert!(ui.cursor_visible());
        assert_eq!(key(&mut ui, vk::T), EventResult::Consumed);
        assert!(!ui.input.is_open());
        typed(&mut ui, "pw");
        key(&mut ui, vk::RETURN);
        assert_eq!(
            ui.poll_events(),
            vec![UiEvent::DialogResponse {
                id: 3,
                button: 1,
                list_item: -1,
                text: "pw".into()
            }]
        );
        assert!(ui.dialog().is_none());
        // Mouse: click the right button of a message box.
        ui.show_dialog(Some(&DialogSpec {
            id: 4,
            style: DialogStyle::MsgBox,
            title: "t".into(),
            body: "b".into(),
            button1: "Yes".into(),
            button2: "No".into(),
        }));
        let _ = ui.build(&M, &NoWorld, 1280.0, 720.0, 0);
        let l = ui.dialog().unwrap().layout(&M, 1280.0, 720.0);
        let b2 = l.button2.unwrap();
        ui.handle_event(WindowEvent::MouseMove {
            x: b2[0] as i32 + 3,
            y: b2[1] as i32 + 3,
        });
        assert_eq!(
            ui.handle_event(WindowEvent::MouseButton {
                button: 0,
                down: true
            }),
            EventResult::Consumed
        );
        assert!(matches!(
            ui.poll_events()[..],
            [UiEvent::DialogResponse { id: 4, button: 0, .. }]
        ));
        // id −1 hides.
        ui.show_dialog(Some(&DialogSpec {
            id: -1,
            ..DialogSpec {
                id: 0,
                style: DialogStyle::MsgBox,
                title: String::new(),
                body: String::new(),
                button1: String::new(),
                button2: String::new(),
            }
        }));
        assert!(ui.dialog().is_none());
    }

    #[test]
    fn textdraw_selection_click_and_escape() {
        let mut ui = Ui::new();
        ui.set_textdraw(
            9,
            Some(&TextDrawSpec {
                text: "Click".into(),
                x: 320.0,
                y: 224.0,
                letter_w: 0.5,
                letter_h: 1.0,
                text_w: 400.0,
                text_h: 20.0,
                colour: Rgba::from_rgba_u32(0xFFFF_FFFF),
                use_box: false,
                box_colour: Rgba::default(),
                shadow: 0,
                outline: 1,
                background_colour: Rgba::from_rgba_u32(0xFF),
                font: 1,
                proportional: true,
                align: sa_samp_api::ui::TdAlign::Left,
                selectable: true,
            }),
        );
        ui.select_textdraws(Some(Rgba::from_rgba_u32(0xFF00_00FF)));
        let _ = ui.build(&M, &NoWorld, 1280.0, 720.0, 0);
        let td = ui.build_textdraws(1280.0, 720.0);
        assert_eq!(td.cmds.len(), 1);
        ui.handle_event(WindowEvent::MouseMove { x: 700, y: 365 });
        ui.handle_event(WindowEvent::MouseButton {
            button: 0,
            down: true,
        });
        assert_eq!(ui.poll_events(), vec![UiEvent::TextDrawClicked(9)]);
        key(&mut ui, vk::ESCAPE);
        assert_eq!(ui.poll_events(), vec![UiEvent::TextDrawClicked(0xFFFF)]);
        assert!(!ui.cursor_visible());
    }

    #[test]
    fn game_text_filters_and_build_orders_layers() {
        let mut ui = Ui::new();
        ui.game_text("", 1000, 3);
        ui.game_text("x", 1000, 7);
        ui.game_text("~r~Welcome", 3000, 3);
        assert_eq!(
            ui.take_game_texts(),
            vec![GameText {
                text: "~r~Welcome".into(),
                time_ms: 3000,
                style: 3
            }]
        );
        ui.chat_add(&ChatMessage {
            kind: sa_samp_api::ui::ChatKind::Info,
            name: String::new(),
            name_colour: Rgba::default(),
            text: "chat".into(),
            colour: Rgba::from_argb_u32(0xFFFF_FFFF),
            time: [12, 0, 0],
        });
        ui.set_cursor_mode(CursorMode::Free);
        let d = ui.build(&M, &NoWorld, 1280.0, 720.0, 0);
        assert!(d.texts().contains(&"chat"));
        assert!(matches!(d.cmds.last(), Some(crate::draw::Cmd::Triangle { .. })));
        // F7 hides the chat.
        key(&mut ui, vk::F7);
        let d = ui.build(&M, &NoWorld, 1280.0, 720.0, 0);
        assert!(!d.texts().contains(&"chat"));
    }
}
