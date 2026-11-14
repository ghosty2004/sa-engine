//! Window messages for the UI: routed key and mouse events, clicks on dialogs, the scoreboard and the
//! class-selection panel, and the edit-box clipboard.

use super::adapters::{LiveGame, gta_menu_active};
use super::{State, try_with_ui};

/// A window event for the UI; returns whether it is consumed.
pub(in crate::ui) fn on_window_event(ev: samp_ui::keys::Event, wheel_lines: u32) -> bool {
    use samp_ui::keys::Event;
    let menu = gta_menu_active();
    try_with_ui(|st| {
        // Mouse on the dialog / textdraws first (GUI dialog MsgProc, selection handler).
        if let Event::LeftButtonUp { x, y } = ev
            && !menu
            && mouse_up(st, x, y)
        {
            return true;
        }
        let game = LiveGame { connected: st.core.connected };
        let consumed = st.core.handle_event(ev, menu, &game, wheel_lines);
        clipboard_requests(st);
        consumed
    })
    .unwrap_or(false)
}

/// DXUT edit-box clipboard: copy (`CF_UNICODETEXT` from the ANSI selection) and paste
/// (`CF_UNICODETEXT` converted to the ANSI code page).
fn clipboard_requests(st: &mut State) {
    use windows_sys::Win32::Globalization::{CP_ACP, MultiByteToWideChar, WideCharToMultiByte};
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
    const CF_UNICODETEXT: u32 = 13;
    if let Some(sel) = st.core.requests.clipboard_copy.take() {
        // SAFETY: Win32 clipboard sequence; the global block is handed to the clipboard.
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) != 0 {
                EmptyClipboard();
                let n = MultiByteToWideChar(CP_ACP, 0, sel.as_ptr(), sel.len() as i32, std::ptr::null_mut(), 0);
                let h = GlobalAlloc(GMEM_MOVEABLE, (n as usize + 1) * 2);
                if !h.is_null() {
                    let p = GlobalLock(h) as *mut u16;
                    if !p.is_null() {
                        MultiByteToWideChar(CP_ACP, 0, sel.as_ptr(), sel.len() as i32, p, n);
                        *p.add(n as usize) = 0;
                        GlobalUnlock(h);
                        SetClipboardData(CF_UNICODETEXT, h as _);
                    }
                }
                CloseClipboard();
            }
        }
    }
    if std::mem::take(&mut st.core.requests.paste) {
        let mut text = Vec::new();
        // SAFETY: as above; the clipboard keeps ownership of its data.
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) != 0 {
                let h = GetClipboardData(CF_UNICODETEXT);
                if !h.is_null() {
                    let p = GlobalLock(h as _) as *const u16;
                    if !p.is_null() {
                        let mut len = 0usize;
                        while *p.add(len) != 0 {
                            len += 1;
                        }
                        let n = WideCharToMultiByte(
                            CP_ACP,
                            0,
                            p,
                            len as i32,
                            std::ptr::null_mut(),
                            0,
                            std::ptr::null(),
                            std::ptr::null_mut(),
                        );
                        text.resize(n.max(0) as usize, 0);
                        WideCharToMultiByte(
                            CP_ACP,
                            0,
                            p,
                            len as i32,
                            text.as_mut_ptr(),
                            n,
                            std::ptr::null(),
                            std::ptr::null_mut(),
                        );
                        GlobalUnlock(h as _);
                    }
                }
                CloseClipboard();
            }
        }
        st.core.paste(&text);
    }
}

/// Left button released over a hovered textdraw, the scoreboard, the class-selection panel or a dialog control
/// (`crate::ui::session`); a panel click is left for the session tick.
fn mouse_up(st: &mut State, x: i32, y: i32) -> bool {
    use samp_ui::classpanel::ClassButton;
    let Some(r) = st.renderer.as_ref() else { return false };
    // SAFETY: plain Win32 query.
    let dbl = unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime() };
    let now = u64::from(crate::ui::core::tick_ms());
    let consumed = st.session.mouse_up(&mut st.core, r, x, y, now, dbl);
    if let Some(c) = st.session.take_class_click() {
        let i = match c {
            ClassButton::Previous => 1,
            ClassButton::Next => 2,
            ClassButton::Spawn => 3,
        };
        CLASS_CLICK.store(i, std::sync::atomic::Ordering::SeqCst);
    }
    consumed
}

/// A click on the class-selection panel: 1 = `<<`, 2 = `>>`, 3 = `Spawn` (0 = none).
static CLASS_CLICK: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Takes the pending class-selection panel click (session tick).
pub fn take_class_click() -> u8 {
    CLASS_CLICK.swap(0, std::sync::atomic::Ordering::SeqCst)
}

/// Edit-box key-down; true if the input or a dialog edit box has focus (consumed).
pub(in crate::ui) fn on_edit_key(key: samp_ui::edit::EditKey, mods: samp_ui::edit::Mods) -> bool {
    try_with_ui(|st| {
        let r = st.core.edit_key(key, mods);
        clipboard_requests(st);
        r
    })
    .unwrap_or(false)
}
