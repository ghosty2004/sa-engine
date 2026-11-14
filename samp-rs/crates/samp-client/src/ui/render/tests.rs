//! The shared pass on a recording backend: what each step asks of a [`UiBackend`], in order.

use sa_samp_api::ui::{ColourQuad, TextdrawFont, UiBackend, UiFont, UiMetrics, UiRect, UiSprite, dt};
use samp_ui::chat::{ChatWindow, LocalTime};
use samp_ui::input::ChatInput;

use super::*;

/// Fixed metrics: every glyph 8 px wide, lines 16 px (main), 14 (small), 20 / 18 (GUI).
#[derive(Default)]
pub struct Recorder {
    pub calls: Vec<String>,
}

fn r(u: UiRect) -> String {
    format!("{},{},{},{}", u.left, u.top, u.right, u.bottom)
}

impl UiBackend for Recorder {
    fn video_size(&self) -> (i32, i32) {
        (800, 600)
    }
    fn client_size(&self) -> (i32, i32) {
        (800, 600)
    }
    fn cursor_pos(&self) -> (i32, i32) {
        (0, 0)
    }
    fn fonts(&mut self, _face: &[u8], _weight: u32, _size: i32, _force: bool) -> bool {
        false
    }
    fn metrics(&self) -> UiMetrics {
        UiMetrics { main_h: 16, small_h: 14, gui0_h: 20, gui1_h: 18, nick_w: 192, icon_w: 31, icon_h: 24, timestamp_w: 80 }
    }
    fn measure(&self, _font: UiFont, text: &[u8], count: i32, rect: UiRect, _flags: u32, _colour: u32) -> UiRect {
        let n = if count < 0 { text.len() } else { count as usize };
        UiRect::new(rect.left, rect.top, rect.left + 8 * n as i32, rect.top + 16)
    }
    fn sprite_begin(&mut self, sprite: UiSprite, _flags: u32) {
        self.calls.push(format!("begin {sprite:?}"));
    }
    fn sprite_end(&mut self, sprite: UiSprite) {
        self.calls.push(format!("end {sprite:?}"));
    }
    fn draw_text(
        &mut self,
        font: UiFont,
        _sprite: Option<UiSprite>,
        text: &[u8],
        _count: i32,
        rect: &mut UiRect,
        flags: u32,
        colour: u32,
    ) -> i32 {
        self.calls.push(format!("text {font:?} {} {:#x} {:#x} {}", r(*rect), flags, colour, String::from_utf8_lossy(text)));
        16
    }
    fn draw_text_coloured(
        &mut self,
        font: UiFont,
        _sprite: UiSprite,
        text: &[u8],
        rect: &mut UiRect,
        flags: u32,
        colour: u32,
        _glyphs: &[u32],
    ) {
        self.calls.push(format!("coloured {font:?} {} {:#x} {:#x} {}", r(*rect), flags, colour, String::from_utf8_lossy(text)));
    }
    fn dialog_begin(&mut self, rect: UiRect, background: u32) {
        self.calls.push(format!("dialog {} {background:#x}", r(rect)));
    }
    fn dialog_end(&mut self) {
        self.calls.push("dialog end".into());
    }
    fn gui_sprite(&mut self, src: UiRect, dst: UiRect, colour: u32) {
        self.calls.push(format!("sprite {} -> {} {colour:#x}", r(src), r(dst)));
    }
    fn gui_text(&mut self, font: UiFont, text: &[u8], rect: UiRect, format: u32, colour: u32) {
        self.calls.push(format!("gui {font:?} {} {format:#x} {colour:#x} {}", r(rect), String::from_utf8_lossy(text)));
    }
    fn gui_text_wide(&mut self, font: UiFont, text: &[u16], _count: i32, rect: UiRect, format: u32, colour: u32) {
        self.calls.push(format!("guiw {font:?} {} {format:#x} {colour:#x} {}", r(rect), String::from_utf16_lossy(text)));
    }
    fn gui_rect(&mut self, rect: UiRect, colour: u32) {
        self.calls.push(format!("rect {} {colour:#x}", r(rect)));
    }
    fn text_width_wide(&self, _font: UiFont, text: &[u16]) -> i32 {
        8 * text.len() as i32
    }
    fn to_wide(&self, text: &[u8]) -> Vec<u16> {
        text.iter().map(|&b| u16::from(b)).collect()
    }
    fn bars_begin(&mut self) {
        self.calls.push("bars".into());
    }
    fn bars_group(&mut self, q: &[ColourQuad; 3]) {
        self.calls.push(format!("bar {} {} {} {}", q[0].x0, q[0].y0, q[2].x1, q[2].colour));
    }
    fn bars_end(&mut self) {
        self.calls.push("bars end".into());
    }
    fn project(&self, p: [f32; 3]) -> [f32; 3] {
        [p[0], p[1], 0.5]
    }
    fn aux_fonts(&mut self, _create: bool) -> bool {
        false
    }
    fn chat_blit(&mut self) -> bool {
        false
    }
    fn textdraw_units(&self) -> (f32, f32) {
        (1.0 / 640.0, 1.0 / 448.0)
    }
    fn textdraw_text(&mut self, f: &TextdrawFont, text: &[u8], _has_keys: bool) {
        self.calls.push(format!("cfont {} {} {}", f.x, f.y, String::from_utf8_lossy(text)));
    }
    fn textdraw_sprite(&mut self, id: u16, _style: u8, _text: &[u8], _serial: u64, _rect: [f32; 4], _colour: u32) -> bool {
        self.calls.push(format!("td sprite {id}"));
        false
    }
    fn textdraw_sync(&mut self, _live: &[(u16, &[u8])]) {}
}

const T: LocalTime = LocalTime { unix: 0, hour: 20, minute: 15, second: 1 };

#[test]
fn chat_lines_are_outlined_text_on_the_chat_sprite() {
    let mut chat = ChatWindow::new();
    chat.add_info(b"hello", T);
    chat.add_player_chat(b"Bob", 0xFF00_FF00, b"hi", T);
    let mut b = Recorder::default();
    chat::chat(&mut b, &chat);
    assert_eq!(b.calls.first().map(String::as_str), Some("begin Chat"));
    assert_eq!(b.calls.last().map(String::as_str), Some("end Chat"));
    // Four outline passes per item with the outline twin, then the coloured pass (default outlined draw).
    let texts: Vec<&String> = b.calls.iter().filter(|c| c.starts_with("coloured")).collect();
    assert_eq!(texts.len(), 3);
    assert!(texts[0].ends_with("hello"));
    assert!(texts[1].contains("Bob:") && texts[2].ends_with("hi"));
    assert_eq!(b.calls.iter().filter(|c| c.starts_with("text MainOutline")).count(), 12);
    // Page size 14 and timestamps keep the newest lines drawn, after the timestamp column.
    chat.set_page_size(14);
    chat.set_timestamps(true);
    let mut b = Recorder::default();
    chat::chat(&mut b, &chat);
    let texts: Vec<&String> = b.calls.iter().filter(|c| c.starts_with("coloured")).collect();
    assert_eq!(texts.len(), 5, "{:?}", b.calls);
    assert!(texts[1].contains(" 130,"), "{}", texts[1]);
}

#[test]
fn class_panel_is_a_gui_dialog_with_three_buttons() {
    let mut b = Recorder::default();
    let rects = chat::class_panel(&mut b, (360, 520));
    assert_eq!(rects[1], samp_ui::Rect::new(355, 515, 445, 545));
    assert_eq!(b.calls[0], "dialog 245,510,555,550 0x960a0a0a");
    // `>>` under the cursor: moved by (−1, −2), red face and fill.
    assert!(b.calls.contains(&"sprite 0,0,136,54 -> 354,513,444,543 0xffb92228".to_string()));
    assert!(b.calls.contains(&"gui Gui0 255,515,345,545 0x5 0xffc8c8c8 <<".to_string()));
    assert_eq!(b.calls.last().map(String::as_str), Some("dialog end"));
}

#[test]
fn chat_gui_draws_the_input_box_with_caret() {
    let chat = ChatWindow::new();
    let mut inp = ChatInput::new();
    inp.open();
    for &c in b"abc" {
        inp.char(c);
    }
    let mut b = Recorder::default();
    let mut st = PassState::default();
    chat::chat_gui(&mut b, &mut st, &chat, &inp, 0);
    // Box at (40, chat bottom) 0.6·W wide (480), 14 + 1.5·20 high.
    let bottom = chat.bottom(16);
    assert_eq!(b.calls[0], "dialog 0,0,580,".to_string() + &(bottom + 50).to_string() + " 0x0");
    assert!(b.calls.iter().any(|c| c == &format!("guiw Gui0 49,{},511,{} 0x4 0xffffffff abc", bottom + 9, bottom + 44 - 9)));
    assert!(b.calls.iter().any(|c| c == &format!("rect 72,{},74,{} 0xff969696", bottom + 9, bottom + 44 - 9)));
}

#[test]
fn bars_and_names_pass() {
    let tags = [TagDraw {
        anchor: [400.0, 300.0, 0.0],
        distance: 0.0,
        name: b"CJ (0)".to_vec(),
        colour: 0xFFFF_8000,
        health: 100.0,
        armour: 50.0,
        afk: false,
    }];
    let mut b = Recorder::default();
    world::tag_bars(&mut b, &tags);
    world::tag_names(&mut b, &tags, true);
    assert_eq!(b.calls[0], "bars");
    assert_eq!(b.calls.len(), 1 + 2 + 1 + 1 + 4 + 1 + 1);
    let x = 400;
    assert!(b.calls[1].starts_with(&format!("bar {} ", x as f32 - 20.0)));
    assert!(b.calls.iter().any(|c| c.starts_with("coloured Small") && c.ends_with("CJ (0)")));
    let _ = dt::CENTER;
}
