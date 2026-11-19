//! samp-rs's UI on a game that provides the UI backend through `GameAdapter::ui` (sa-engine natively and in the
//! browser): the per-frame steps of the Windows glue expressed with the game adapter instead of the original
//! exe's memory. [`UiHost`] owns the [`UiCore`], the [`UiSession`] and samp's cursor / input-lock state machine
//! ([`InputLock`]: its mode tells the backend what the UI holds), runs samp's frame entry, the textdraw pass after
//! `Render2dStuff` and the Present pass, and routes window events (`sa_samp_api::input::WindowEvent`, mapped to
//! samp-ui's window-procedure events) to the UI. Spec: docs/re/subsystems/ui-rendering.md, "2.3", "2.4", "5", "6".

use std::path::PathBuf;

use sa_samp_api::GameAdapter;
use sa_samp_api::hooks::FramePhase;
use sa_samp_api::input::{EventResult, WindowEvent};
use samp_ui::commands::{GameInfo, PlayerPosition};
use samp_ui::edit::{EditKey, Mods};
use samp_ui::keys::{Event, Vk};

use super::core::{CursorRequest, Outgoing, UiCore, tick_ms};
use super::inputlock::InputLock;
use super::menuwork::{AdapterMenus, MenuWork};
use super::render::{BubbleDraw, CursorReq, LabelDraw, TagDraw};
use super::session::{PassInput, UiSession};
use crate::netclient::NetClient;

/// Mouse-wheel lines per notch (Windows' default `SPI_GETWHEELSCROLLLINES`).
const WHEEL_LINES: u32 = 3;
/// The double-click time (Windows' default `GetDoubleClickTime`).
const DOUBLE_CLICK_MS: u32 = 500;

/// What the UI's local commands may ask of the game on this host.
struct HostInfo {
    connected: bool,
}

impl GameInfo for HostInfo {
    fn connected(&self) -> bool {
        self.connected
    }
    fn memory_counter(&self) -> u32 {
        0
    }
    fn interior(&self) -> u32 {
        0
    }
    fn position(&self) -> Option<PlayerPosition> {
        None
    }
}

/// The world-anchored things the client found this frame (nametags, chat bubbles, 3D labels).
#[derive(Default)]
pub struct WorldDraws {
    pub tags: Vec<TagDraw>,
    pub bubbles: Vec<BubbleDraw>,
    pub labels: Vec<LabelDraw>,
}

/// samp-rs's UI on an adapter-provided backend.
pub struct UiHost {
    pub core: UiCore,
    pub session: UiSession,
    lock: InputLock,
    /// Set by the client before the Present pass.
    pub world: WorldDraws,
    /// The spawn ends class selection: its mode-0 request restores input only after 10 frames.
    pub spawn_release: bool,
    cursor: (i32, i32),
    mods: Mods,
    /// GUI font 1 line height of the last pass (list rows of mouse clicks).
    row_h: i32,
    /// The `WM_CHAR` a key-down of Backspace, Enter or Esc still owes ([`Self::window_event`]).
    pending_char: Option<u8>,
    /// The GTA menu of the current server menu.
    menus: MenuWork,
    /// The text keys the current GTA menu registered.
    menu_keys: Vec<&'static str>,
}

impl UiHost {
    /// The UI with samp's user files in `user_dir` (`None`: in memory only).
    pub fn new(user_dir: Option<PathBuf>) -> Self {
        Self {
            core: UiCore::new(user_dir, false),
            session: UiSession::new(),
            lock: InputLock::new(),
            world: WorldDraws::default(),
            spawn_release: false,
            cursor: (0, 0),
            mods: Mods::default(),
            row_h: 0,
            pending_char: None,
            menus: MenuWork::default(),
            menu_keys: Vec::new(),
        }
    }

    /// The client's frame callback for the UI's phases: samp's frame entry before `Render2dStuff`, the textdraws
    /// after it, the Present pass.
    pub fn frame(&mut self, phase: FramePhase, game: &mut dyn GameAdapter) {
        match phase {
            FramePhase::Before2d => self.frame_entry(game),
            FramePhase::After2d => self.after_2d(game),
            FramePhase::Present => self.present(game),
            _ => {}
        }
    }

    /// samp's per-frame entry: game text, class selection's cursor mode and panel, the scoreboard refresh, chat
    /// bubble expiry. Mirrors samp.dll 0x100C4FB0.
    fn frame_entry(&mut self, game: &mut dyn GameAdapter) {
        let core = &mut self.core;
        if let Some((style, time, text)) = core.requests.game_text.take() {
            // RPC 73: clear prints, then `AddBigMessage` (the adapter keeps the text alive).
            let _ = game.clear_prints();
            let text: String = text.iter().map(|&b| char::from(b)).collect();
            let _ = game.big_message(&text, time, u16::from(style));
        }
        if core.class_selection && !core.scoreboard.is_open() {
            self.lock.request_mode2();
        }
        self.session.class_selection_frame(core);
        let pad = game.pad().0;
        self.menus.frame(core, &mut AdapterMenus { game, keys: &mut self.menu_keys }, (pad.face[2] != 0, pad.face[1] != 0));
        let (rebuild, send) = core.scoreboard.frame(u64::from(tick_ms()), core.remote_ids().count());
        if rebuild {
            core.rebuild_scoreboard();
        }
        if send {
            core.push_outgoing(Outgoing::ScoresRequest);
        }
        core.bubbles.expire(tick_ms());
    }

    /// After `Render2dStuff`: the textdraws, then the input-restore step (ui-rendering.md, "2.4").
    fn after_2d(&mut self, game: &mut dyn GameAdapter) {
        let core = &mut self.core;
        if !game.menu_active()
            && core.netgame
            && !core.scoreboard.is_open()
            && let Some(b) = game.ui()
        {
            super::render::textdraws::draw_all(b, &mut core.textdraws);
        }
        self.lock.frame();
        if std::mem::take(&mut core.requests.input_restore) {
            self.lock.frame();
        }
    }

    /// samp's Present pass (ui-rendering.md, "2.3"): the cursor-mode requests, the layouts, the pass, textdraw
    /// selection, leaving cursor mode 2; then what the UI holds goes to the backend. Nothing with the pause menu
    /// up.
    fn present(&mut self, game: &mut dyn GameAdapter) {
        let menu = game.menu_active();
        if let Some(c) = self.pending_char.take() {
            let info = HostInfo { connected: self.core.connected };
            self.core.handle_event(Event::Char(c), menu, &info, WHEEL_LINES);
        }
        if menu {
            return;
        }
        for req in std::mem::take(&mut self.core.requests.cursor) {
            match req {
                CursorRequest::Mode0 { immediate } => self.lock.request_mode0(immediate),
                CursorRequest::Mode3 => self.lock.request_mode3(),
            };
        }
        let Some(b) = game.ui() else { return };
        self.session.update_layouts(&mut self.core, b);
        self.row_h = b.metrics().gui1_h;
        let input = PassInput {
            overlay: None,
            tags: &self.world.tags,
            bubbles: &self.world.bubbles,
            labels: &self.world.labels,
            vehicle_labels: None,
            audio_title: None,
            net_stats: None,
            netgame_object: self.core.netgame,
            cursor: self.cursor,
        };
        let sb_open = input.sb_open(&self.core);
        let lock = &mut self.lock;
        self.session.draw(&self.core, b, &input, &mut |req| match req {
            CursorReq::Mode2 => {
                lock.request_mode2();
            }
        });
        if self.core.textdraws.selection.active && !sb_open {
            self.lock.request_mode2();
            self.core.textdraws.hover(self.cursor.0, self.cursor.1);
        }
        if !UiSession::wants_mode2(&self.core) && self.lock.mode() == 2 {
            let immediate = !std::mem::take(&mut self.spawn_release);
            self.lock.request_mode0(immediate);
        }
        let (keyboard, mouse, cursor) = match self.lock.mode() {
            1 => (true, false, false),
            2 => (true, true, true),
            3 => (false, true, true),
            4 => (false, true, false),
            _ => (false, false, false),
        };
        b.set_input_state(keyboard, mouse, cursor);
    }

    /// A window event for the UI (samp's window procedure): edit-box keys first, then samp-ui's routing; a left
    /// button release goes to the dialogs, the scoreboard, textdraws and the class-selection panel first.
    pub fn window_event(&mut self, e: WindowEvent, game: &dyn GameAdapter) -> EventResult {
        let menu = game.menu_active();
        let own_char = self.pending_char.is_some_and(|c| matches!(e, WindowEvent::Char { ch, .. } if ch == u32::from(c)));
        self.translate(&e, menu);
        if own_char {
            // The platform's own character for the key-down: routed like Windows' WM_CHAR.
            let c = match e {
                WindowEvent::Char { ch, .. } => ch as u8,
                _ => 0,
            };
            let info = HostInfo { connected: self.core.connected };
            let consumed = self.core.handle_event(Event::Char(c), menu, &info, WHEEL_LINES);
            return if consumed { EventResult::Consumed } else { EventResult::PassToGame };
        }
        let ev = match e {
            WindowEvent::KeyDown { vk, .. } => {
                self.modifier(vk, true);
                if let Some(k) = EditKey::from_vk(u32::from(vk))
                    && self.core.edit_key(k, self.mods)
                {
                    return EventResult::Consumed;
                }
                Event::KeyDown(Vk::from_code(u32::from(vk)))
            }
            WindowEvent::KeyUp { vk } => {
                self.modifier(vk, false);
                Event::KeyUp(Vk::from_code(u32::from(vk)))
            }
            WindowEvent::Char { ch, unicode } => match u8::try_from(ch) {
                Ok(b) => Event::Char(b),
                Err(_) if unicode => Event::Char(b'?'),
                Err(_) => return EventResult::PassToGame,
            },
            WindowEvent::Wheel(d) => Event::Wheel(i32::from(d) / 120),
            WindowEvent::MouseMove { x, y } => {
                self.cursor = (x, y);
                return EventResult::PassToGame;
            }
            WindowEvent::MouseButton { button: 0, down: false } => {
                let (x, y) = self.cursor;
                if !menu
                    && self.session.mouse_up(&mut self.core, self.row_h, x, y, u64::from(tick_ms()), DOUBLE_CLICK_MS)
                {
                    return EventResult::Consumed;
                }
                Event::LeftButtonUp { x, y }
            }
            WindowEvent::MouseButton { .. } => return EventResult::PassToGame,
            WindowEvent::Focus(false) => Event::KillFocus,
            WindowEvent::Focus(true) => return EventResult::PassToGame,
        };
        let info = HostInfo { connected: self.core.connected };
        let consumed = self.core.handle_event(ev, menu, &info, WHEEL_LINES);
        // Windows' TranslateMessage follows these key-downs with their WM_CHAR (Backspace, Enter, Esc), which is
        // where samp's edit boxes and its Esc handling read them. A platform that sends the character itself
        // sends it right after the key; otherwise it is made up before the next event ([`Self::translate`]).
        if let WindowEvent::KeyDown { vk: vk @ (0x08 | 0x0D | 0x1B), .. } = e {
            self.pending_char = Some(vk as u8);
        }
        if consumed { EventResult::Consumed } else { EventResult::PassToGame }
    }

    /// The control character a key-down left pending, unless `e` is that character (the platform's own).
    fn translate(&mut self, e: &WindowEvent, menu: bool) {
        let Some(c) = self.pending_char.take() else { return };
        if matches!(*e, WindowEvent::Char { ch, .. } if ch == u32::from(c)) {
            return;
        }
        let info = HostInfo { connected: self.core.connected };
        self.core.handle_event(Event::Char(c), menu, &info, WHEEL_LINES);
    }

    fn modifier(&mut self, vk: u16, down: bool) {
        match vk {
            0x10 | 0xA0 | 0xA1 => self.mods.shift = down,
            0x11 | 0xA2 | 0xA3 => self.mods.ctrl = down,
            _ => {}
        }
    }

    /// Whether the chat input or a dialog has the keyboard.
    pub fn wants_keyboard(&self) -> bool {
        self.core.input.is_open() || self.core.dialogs.is_open()
    }

    /// Sends what the UI produced (chat lines, commands, dialog answers, clicks).
    pub fn send_outgoing(&mut self, net: &mut NetClient) {
        for o in self.core.take_outgoing() {
            super::core::send_outgoing(net, o);
        }
    }
}
