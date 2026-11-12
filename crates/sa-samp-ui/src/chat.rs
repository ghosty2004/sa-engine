//! The chat window (100 entries, a page of 10 lines at the top left) and the chat input line with its
//! history. Layout constants from samp.dll 0x10067470 / 0x10067940 (via samp-rs's samp-ui spec).

use std::collections::VecDeque;

use sa_samp_api::ui::{ChatKind, ChatMessage};

use crate::draw::{DrawList, OUTLINE_4, OUTLINE_8};
use crate::edit::LineEdit;
use crate::font::{FontId, Measure};
use crate::text::{replace_control_chars, split_line};

/// Entries kept.
pub const CAPACITY: usize = 100;
/// Lines shown.
pub const PAGE_SIZE: usize = 10;
/// Longest name prefix.
pub const MAX_NAME_LEN: usize = 24;
/// Longest text of one entry.
pub const MAX_TEXT_LEN: usize = 144;
/// Player chat wrap width and backtrack (characters).
pub const PLAYER_WRAP: (usize, usize) = (72, 12);
/// Left edge of the lines.
pub const LINE_LEFT: f32 = 45.0;
/// Top of the first line.
pub const FIRST_LINE_TOP: f32 = 10.0;
/// Gap after the `name:` prefix.
pub const NAME_GAP: f32 = 5.0;
/// Input line: max characters, history size, box left, max width.
pub const INPUT_MAX: usize = 128;
/// History entries.
pub const HISTORY_SIZE: usize = 10;
/// Input box left.
pub const INPUT_LEFT: f32 = 40.0;
/// Input box maximum width.
pub const INPUT_MAX_W: f32 = 800.0;
/// Player chat text colour.
pub const PLAYER_TEXT: u32 = 0xFFFF_FFFF;

/// One stored entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Type.
    pub kind: ChatKind,
    /// Name (player chat; empty for continuation lines).
    pub name: String,
    /// `0xAARRGGBB`.
    pub name_colour: u32,
    /// Text (codes kept).
    pub text: String,
    /// `0xAARRGGBB`.
    pub colour: u32,
}

/// Display mode cycled with F7: off, without outline, with outline (samp's default).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Hidden.
    Off,
    /// Shown, no outline.
    Plain,
    /// Shown, outlined.
    Outlined,
}

/// The chat window.
#[derive(Clone, Debug)]
pub struct ChatWindow {
    entries: VecDeque<Entry>,
    /// Lines scrolled up from the newest.
    scroll: usize,
    mode: Mode,
}

impl Default for ChatWindow {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            scroll: 0,
            mode: Mode::Outlined,
        }
    }
}

impl ChatWindow {
    /// Stored entries, oldest first.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    fn push(&mut self, e: Entry) {
        if self.entries.len() == CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(e);
    }

    /// Adds a message: player chat is wrapped at 72 characters into a second, nameless entry; texts are cut
    /// at 144 characters, control characters become spaces. Names over 24 characters drop the entry.
    pub fn add(&mut self, m: &ChatMessage) {
        let text = replace_control_chars(&m.text);
        let colour = m.colour.to_argb_u32();
        match m.kind {
            ChatKind::Player => {
                if m.name.chars().count() > MAX_NAME_LEN {
                    return;
                }
                let (first, rest) = split_line(&text, PLAYER_WRAP.0, PLAYER_WRAP.1);
                let name_colour = m.name_colour.to_argb_u32() | 0xFF00_0000;
                self.push(Entry {
                    kind: ChatKind::Player,
                    name: m.name.clone(),
                    name_colour,
                    text: first,
                    colour: PLAYER_TEXT,
                });
                if let Some(rest) = rest {
                    self.push(Entry {
                        kind: ChatKind::Player,
                        name: String::new(),
                        name_colour,
                        text: rest.chars().take(MAX_TEXT_LEN).collect(),
                        colour: PLAYER_TEXT,
                    });
                }
            }
            kind => self.push(Entry {
                kind,
                name: String::new(),
                name_colour: 0,
                text: text.chars().take(MAX_TEXT_LEN).collect(),
                colour,
            }),
        }
        self.scroll = 0;
    }

    /// Removes every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.scroll = 0;
    }

    /// Display mode.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// F7: next display mode.
    pub fn cycle_mode(&mut self) {
        self.mode = match self.mode {
            Mode::Outlined => Mode::Off,
            Mode::Off => Mode::Plain,
            Mode::Plain => Mode::Outlined,
        };
    }

    /// Page Up / Page Down (one page; clamped).
    pub fn scroll_by(&mut self, lines: isize) {
        let max = self.entries.len().saturating_sub(PAGE_SIZE) as isize;
        self.scroll = (self.scroll as isize + lines).clamp(0, max) as usize;
    }

    /// Lines scrolled up.
    pub fn scroll(&self) -> usize {
        self.scroll
    }

    /// Bottom of the window: `10 + (pageSize + 1)·(h + 1)`.
    pub fn bottom(&self, line_h: f32) -> f32 {
        FIRST_LINE_TOP + (PAGE_SIZE as f32 + 1.0) * (line_h + 1.0)
    }

    /// Draws the visible page. The newest entry is on the last line; with fewer entries the lines fill from
    /// the bottom of the page, as samp's 100-slot ring does.
    pub fn draw(&self, m: &dyn Measure, video_w: u32, d: &mut DrawList) {
        if self.mode == Mode::Off {
            return;
        }
        let h = m.line_height(FontId::Main);
        let outline: &[(i32, i32)] = match (self.mode, video_w > 1280) {
            (Mode::Plain, _) => &[],
            (_, true) => &OUTLINE_8,
            (_, false) => &OUTLINE_4,
        };
        let end = self.entries.len() - self.scroll.min(self.entries.len());
        let start = end.saturating_sub(PAGE_SIZE);
        let skip = PAGE_SIZE - (end - start);
        for (k, e) in self.entries.range(start..end).enumerate() {
            let y = FIRST_LINE_TOP + (k + skip) as f32 * (h + 1.0);
            let mut x = LINE_LEFT;
            if e.kind == ChatKind::Player && !e.name.is_empty() {
                let prefix = format!("{}:", e.name);
                d.coloured(m, x, y, FontId::Main, &prefix, e.name_colour, outline);
                x += m.width(FontId::Main, &prefix) + NAME_GAP;
            }
            d.coloured(m, x, y, FontId::Main, &e.text, e.colour, outline);
        }
    }
}

/// The chat input line (T / F6 opens, Enter sends, Esc closes, Up / Down walk the history).
#[derive(Clone, Debug)]
pub struct ChatInput {
    open: bool,
    edit: LineEdit,
    history: VecDeque<String>,
    /// Position while walking the history (`None` = the line being typed).
    recall: Option<usize>,
}

impl Default for ChatInput {
    fn default() -> Self {
        Self {
            open: false,
            edit: LineEdit::new(INPUT_MAX),
            history: VecDeque::new(),
            recall: None,
        }
    }
}

impl ChatInput {
    /// Whether the line is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The edit.
    pub fn edit(&self) -> &LineEdit {
        &self.edit
    }

    /// The edit, mutable.
    pub fn edit_mut(&mut self) -> &mut LineEdit {
        &mut self.edit
    }

    /// History, newest first.
    pub fn history(&self) -> impl Iterator<Item = &String> {
        self.history.iter()
    }

    /// Opens the line (empty).
    pub fn open(&mut self) {
        self.open = true;
        self.edit.clear();
        self.recall = None;
    }

    /// Closes without sending.
    pub fn close(&mut self) {
        self.open = false;
        self.edit.clear();
        self.recall = None;
    }

    /// Enter: closes; a non-empty line goes to the history and is returned.
    pub fn submit(&mut self) -> Option<String> {
        let t = self.edit.text();
        self.close();
        if t.is_empty() {
            return None;
        }
        self.history.push_front(t.clone());
        self.history.truncate(HISTORY_SIZE);
        Some(t)
    }

    /// Up: the next older history line.
    pub fn history_older(&mut self) {
        let next = self.recall.map_or(0, |i| i + 1);
        if let Some(t) = self.history.get(next) {
            self.recall = Some(next);
            self.edit.set_text(&t.clone());
        }
    }

    /// Down: the next newer history line, then an empty line.
    pub fn history_newer(&mut self) {
        match self.recall {
            Some(0) | None => {
                self.recall = None;
                self.edit.clear();
            }
            Some(i) => {
                self.recall = Some(i - 1);
                let t = self.history[i - 1].clone();
                self.edit.set_text(&t);
            }
        }
    }

    /// Box rectangle: `min(800, 0.6·W)` wide at `(40, chatBottom)`, `14 + 1.5·h0` high.
    pub fn box_rect(video_w: u32, chat_bottom: f32, gui0_h: f32) -> [f32; 4] {
        let w = INPUT_MAX_W.min((0.6 * video_w as f32).round());
        [
            INPUT_LEFT,
            chat_bottom,
            INPUT_LEFT + w,
            chat_bottom + 14.0 + (1.5 * gui0_h).round(),
        ]
    }

    /// Draws the open line: dark box, light frame, text and a blinking caret.
    pub fn draw(&self, m: &dyn Measure, video_w: u32, chat_bottom: f32, time_ms: u32, d: &mut DrawList) {
        if !self.open {
            return;
        }
        let r = Self::box_rect(video_w, chat_bottom, m.line_height(FontId::Gui0));
        d.rect(r[0], r[1], r[2], r[3], 0xC000_0000);
        d.frame(r[0], r[1], r[2], r[3], 0xFFC8_C8C8);
        let pad = 9.0;
        let ty = ((r[1] + r[3] - m.line_height(FontId::Gui0)) / 2.0).round();
        // Scroll the text left so the caret stays inside the box.
        let inner = r[2] - r[0] - 2.0 * pad;
        let before = self.edit.display_before_caret(false);
        let shift = (m.width(FontId::Gui0, &before) - inner + 2.0).max(0.0);
        let text = self.edit.display(false);
        // Characters scrolled out on the left are dropped whole.
        let mut skip = 0;
        let mut acc = 0.0;
        for c in text.chars() {
            if acc >= shift {
                break;
            }
            acc += m.width(FontId::Gui0, &c.to_string());
            skip += 1;
        }
        let shown: String = text.chars().skip(skip).collect();
        d.text(r[0] + pad, ty, FontId::Gui0, 0xFFFF_FFFF, &shown);
        if (time_ms / 500).is_multiple_of(2) {
            let cx = r[0] + pad + m.width(FontId::Gui0, &before) - acc;
            d.rect(cx, ty, cx + 2.0, ty + m.line_height(FontId::Gui0), 0xFFFF_FFFF);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Cmd;
    use crate::font::MonoMeasure;
    use sa_samp_api::Rgba;

    const M: MonoMeasure = MonoMeasure {
        char_w: 7.0,
        line_h: 16.0,
    };

    fn info(t: &str) -> ChatMessage {
        ChatMessage {
            kind: ChatKind::Info,
            name: String::new(),
            name_colour: Rgba::default(),
            text: t.into(),
            colour: Rgba::from_argb_u32(0xFF88_AA62),
        }
    }

    #[test]
    fn capacity_and_truncation() {
        let mut c = ChatWindow::default();
        for i in 0..105 {
            c.add(&info(&format!("m{i}")));
        }
        let t: Vec<&str> = c.entries().map(|e| e.text.as_str()).collect();
        assert_eq!((t.len(), t[0], t[99]), (100, "m5", "m104"));
        c.add(&info(&"a".repeat(200)));
        assert_eq!(c.entries().last().unwrap().text.len(), 144);
        c.add(&info("a\tb"));
        assert_eq!(c.entries().last().unwrap().text, "a b");
    }

    #[test]
    fn player_chat_wraps_into_a_nameless_entry() {
        let mut c = ChatWindow::default();
        let mut text = "w".repeat(90);
        text.replace_range(65..66, " ");
        c.add(&ChatMessage {
            kind: ChatKind::Player,
            name: "Bob".into(),
            name_colour: Rgba::from_argb_u32(0xFF00_FF00),
            text,
            colour: Rgba::from_argb_u32(0xFFFF_FFFF),
        });
        let e: Vec<&Entry> = c.entries().collect();
        assert_eq!(e.len(), 2);
        assert_eq!((e[0].name.as_str(), e[0].text.len()), ("Bob", 65));
        assert_eq!((e[1].name.as_str(), e[1].text.len()), ("", 24));
        // Too long a name drops the message.
        c.add(&ChatMessage {
            kind: ChatKind::Player,
            name: "n".repeat(25),
            ..info("x")
        });
        assert_eq!(c.entries().count(), 2);
    }

    #[test]
    fn layout_fills_from_the_bottom_with_name_prefix() {
        let mut c = ChatWindow::default();
        c.add(&ChatMessage {
            kind: ChatKind::Player,
            name: "Bob".into(),
            name_colour: Rgba::from_argb_u32(0xFF00_FF00),
            text: "hi".into(),
            colour: Rgba::from_argb_u32(0xFFFF_FFFF),
        });
        let mut d = DrawList::default();
        c.draw(&M, 1024, &mut d);
        // Plain colour commands (the last of each outline group): "Bob:" then "hi".
        let coloured: Vec<(f32, f32, u32, &str)> = d
            .cmds
            .iter()
            .filter_map(|c| match c {
                Cmd::Text {
                    x, y, colour, text, ..
                } if *colour != 0xFF00_0000 => Some((*x, *y, *colour, text.as_str())),
                _ => None,
            })
            .collect();
        let y9 = FIRST_LINE_TOP + 9.0 * 17.0;
        assert_eq!(
            coloured,
            vec![
                (45.0, y9, 0xFF00_FF00, "Bob:"),
                (45.0 + 28.0 + 5.0, y9, 0xFFFF_FFFF, "hi")
            ]
        );
        assert_eq!(c.bottom(16.0), 10.0 + 11.0 * 17.0);
        c.cycle_mode();
        let mut d = DrawList::default();
        c.draw(&M, 1024, &mut d);
        assert!(d.cmds.is_empty());
    }

    #[test]
    fn scrolling_clamps() {
        let mut c = ChatWindow::default();
        for i in 0..15 {
            c.add(&info(&i.to_string()));
        }
        c.scroll_by(10);
        assert_eq!(c.scroll(), 5);
        c.scroll_by(-100);
        assert_eq!(c.scroll(), 0);
    }

    #[test]
    fn input_history_and_submit() {
        let mut i = ChatInput::default();
        i.open();
        for ch in "hello".chars() {
            i.edit_mut().insert(ch);
        }
        assert_eq!(i.submit(), Some("hello".into()));
        assert!(!i.is_open());
        i.open();
        assert_eq!(i.submit(), None);
        i.open();
        for ch in "/q".chars() {
            i.edit_mut().insert(ch);
        }
        i.submit();
        i.open();
        i.history_older();
        assert_eq!(i.edit().text(), "/q");
        i.history_older();
        assert_eq!(i.edit().text(), "hello");
        i.history_older();
        assert_eq!(i.edit().text(), "hello");
        i.history_newer();
        assert_eq!(i.edit().text(), "/q");
        i.history_newer();
        assert_eq!(i.edit().text(), "");
        assert_eq!(
            ChatInput::box_rect(1280, 197.0, 20.0),
            [40.0, 197.0, 808.0, 241.0]
        );
    }
}
