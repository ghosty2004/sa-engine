//! The chat window (direct draw or the backend's render-target cache) and the full-screen chat GUI dialog with
//! the chat scrollbar and the input edit box; the class-selection panel.

use sa_samp_api::ui::{UiBackend, UiFont, UiRect, UiSprite, dt};
use samp_ui::chat::{ChatLayout, ChatWindow};
use samp_ui::dxut::{self, ScrollBar};
use samp_ui::input::{self, ChatInput};
use samp_ui::{classpanel, scoreboard};

use super::gui::{self, EditView};
use super::{PassState, SPRITE_ALPHABLEND, outlined};

/// Chat GUI dialog background: drawn like samp does, but fully transparent.
const CHAT_GUI_BACKGROUND: u32 = 0x0000_0000;

/// Draws the visible chat lines with the chat sprite (ui-rendering.md, "Chat window", "Layout").
pub fn chat_lines(b: &mut dyn UiBackend, layout: &ChatLayout) {
    for line in &layout.lines {
        for item in &line.items {
            let rect = UiRect::new(item.x, item.y, line.rect.right, line.rect.bottom);
            outlined(
                b,
                UiSprite::Chat,
                UiFont::Main,
                UiFont::MainOutline,
                &item.text,
                rect,
                dt::SINGLELINE,
                item.colour,
                layout.outline,
            );
        }
    }
}

/// Present step: the chat window, a blit of the backend's render target or a direct draw (ui-rendering.md,
/// "Draw path").
pub fn chat(b: &mut dyn UiBackend, chat: &ChatWindow) {
    if !chat.mode().chat_visible() || b.chat_blit() {
        return;
    }
    let layout = chat.layout(&super::Measure(b), b.video_size().0);
    b.sprite_begin(UiSprite::Chat, SPRITE_ALPHABLEND);
    chat_lines(b, &layout);
    b.sprite_end(UiSprite::Chat);
}

/// The full-screen chat / input GUI dialog: the chat scrollbar and the input edit box (ui-rendering.md,
/// "Scrollbar and paging", "Chat input").
pub fn chat_gui(b: &mut dyn UiBackend, st: &mut PassState, chat: &ChatWindow, inp: &ChatInput, first_visible: usize) {
    let m = b.metrics();
    let bottom = chat.bottom(m.main_h);
    let (cw, _) = b.client_size();
    let box_r = input::box_rect(cw, bottom, m.gui0_h);
    let (vw, vh) = b.video_size();
    let mut host = st.host_size.unwrap_or((vw, vh));
    if inp.is_open() {
        host = input::host_dialog_size(box_r.width(), bottom);
    }
    st.host_size = Some(host);
    b.dialog_begin(UiRect::new(0, 0, host.0, host.1), CHAT_GUI_BACKGROUND);
    if chat.scrollbar_visible(inp.is_open()) {
        let sb = ScrollBar {
            rect: chat.scrollbar_rect(m.main_h),
            start: samp_ui::chat::SCROLL_MIN as i32,
            end: samp_ui::chat::SCROLL_END as i32,
            pos: chat.scroll_pos() as i32,
            page: chat.page_size() as i32,
        };
        gui::scrollbar(b, &sb);
    }
    if inp.is_open() {
        let e = inp.edit();
        let wide = b.to_wide(e.text());
        let view = EditView {
            text: &wide,
            caret: e.caret(),
            anchor: e.anchor(),
            first_visible,
            focused: true,
            insert_mode: e.insert_mode(),
        };
        gui::edit_box(b, box_r, &view);
    }
    b.dialog_end();
}

/// Class-selection panel (ui-rendering.md, "GUI dialogs created by samp"; samp-ui `classpanel`): the GUI dialog
/// on the scoreboard background with the `<<` `>>` `Spawn` buttons. Returns the three button rects.
pub fn class_panel(b: &mut dyn UiBackend, cursor: (i32, i32)) -> [samp_ui::Rect; 3] {
    let (cw, ch) = b.client_size();
    let rect = classpanel::rect(cw, ch);
    let rects = classpanel::buttons(cw, ch);
    b.dialog_begin(super::ui_rect(rect), scoreboard::BACKGROUND);
    for (r, label) in rects.iter().zip(classpanel::LABELS) {
        gui::button(b, label, *r, r.contains(cursor.0, cursor.1));
    }
    b.dialog_end();
    rects
}

/// The chat input's first visible character after a caret move (DXUT `PlaceCaret`).
pub fn input_first_visible(b: &dyn UiBackend, chat: &ChatWindow, inp: &ChatInput, first: usize) -> usize {
    let (cw, _) = b.client_size();
    let m = b.metrics();
    let r = input::box_rect(cw, chat.bottom(m.main_h), m.gui0_h);
    let wide = b.to_wide(inp.edit().text());
    let width = dxut::edit_text_width(r);
    samp_ui::edit::place_caret(first, inp.edit().caret(), wide.len(), width, &|cp| {
        b.text_width_wide(UiFont::Gui0, &wide[..cp])
    })
}
