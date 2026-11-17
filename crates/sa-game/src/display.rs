//! The display behind the D3D9 adapter (ADR-003 §3.1: shared state, filled by the platform backends).
//!
//! Two lists live here:
//!
//! * the display a backend **reports** ([`set_reported`]): natively the monitor's current mode and video modes (the
//!   window backend), in the browser the canvas's size on the page in device pixels; offscreen nothing, which reads
//!   as the platform's fixed tables (`dx::DESKTOP_MODE`, `dx::ADAPTER_MODES`). Mods see it (`display_mode_*`).
//! * the modes the **adapter** lists to the game (`IDirect3D9::GetAdapterModeCount` / `EnumAdapterModes` /
//!   `GetAdapterDisplayMode`, [`adapter_modes`], [`adapter_desktop`]): the fixed tables, unless a mod requested a
//!   video mode ([`set_adapter`], feature `mods`); then the reported display's modes plus the requested size. A
//!   build or run without such a mod sees exactly the fixed tables (the deterministic runs and the oracle use them).
//!
//! The window backend also installs how a requested video mode is shown ([`set_window_apply`]): natively the window
//! is resized to it (or made full screen).

use std::cell::RefCell;
use std::sync::Mutex;

use crate::dx;

/// A display mode: width, height, refresh rate (Hz).
pub type Mode = (u32, u32, u32);

/// A display: its desktop mode and its modes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Display {
    pub desktop: Mode,
    pub modes: Vec<Mode>,
}

impl Display {
    /// The platform's fixed tables (the offscreen display, and what the adapter lists by default).
    pub fn fixed() -> Display {
        Display { desktop: (dx::DESKTOP_MODE[0], dx::DESKTOP_MODE[1], dx::DESKTOP_MODE[2]), modes: dx::ADAPTER_MODES.to_vec() }
    }
}

static REPORTED: Mutex<Option<Display>> = Mutex::new(None);
static ADAPTER: Mutex<Option<Display>> = Mutex::new(None);

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The backend's display: `modes` are kept once per size (the highest refresh rate), largest first.
pub fn set_reported(desktop: Mode, modes: impl IntoIterator<Item = Mode>) {
    let mut v: Vec<Mode> = Vec::new();
    for m in modes {
        if m.0 == 0 || m.1 == 0 {
            continue;
        }
        match v.iter_mut().find(|e| (e.0, e.1) == (m.0, m.1)) {
            Some(e) => e.2 = e.2.max(m.2),
            None => v.push(m),
        }
    }
    v.sort_by_key(|m| std::cmp::Reverse((m.0 as u64 * m.1 as u64, m.0)));
    *lock(&REPORTED) = Some(Display { desktop, modes: v });
}

/// The display the backend reports (the fixed tables when it reports none).
pub fn reported() -> Display {
    lock(&REPORTED).clone().unwrap_or_else(Display::fixed)
}

/// The adapter lists `display` from now on (before `IDirect3D9` enumerates its modes).
pub fn set_adapter(display: Display) {
    *lock(&ADAPTER) = Some(display);
}

/// The modes the adapter lists (per colour format; `dx` decides which formats have modes).
pub fn adapter_modes() -> Vec<Mode> {
    lock(&ADAPTER).as_ref().map_or_else(|| dx::ADAPTER_MODES.to_vec(), |d| d.modes.clone())
}

/// The adapter's desktop mode (`GetAdapterDisplayMode`).
pub fn adapter_desktop() -> Mode {
    lock(&ADAPTER).as_ref().map_or_else(|| Display::fixed().desktop, |d| d.desktop)
}

/// How the backend shows a video mode: `(width, height, fullscreen)`.
type WindowApply = Box<dyn Fn(u32, u32, bool)>;

thread_local! {
    static WINDOW_APPLY: RefCell<Option<WindowApply>> = const { RefCell::new(None) };
}

/// The window backend's way to show a video mode (resize the window, or full screen).
pub fn set_window_apply(f: WindowApply) {
    WINDOW_APPLY.with_borrow_mut(|w| *w = Some(f));
}

/// Shows the video mode `width` x `height` in the window (no window: nothing to do; the surface follows the back
/// buffer at the next present).
pub fn apply_window(width: u32, height: u32, fullscreen: bool) {
    WINDOW_APPLY.with_borrow(|w| {
        if let Some(f) = w {
            f(width, height, fullscreen)
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reported_modes_are_unique_and_sorted() {
        set_reported((1920, 1080, 60), [(800, 600, 60), (1920, 1080, 60), (800, 600, 75), (0, 0, 0), (1920, 1080, 144)]);
        let d = reported();
        assert_eq!(d.modes, vec![(1920, 1080, 144), (800, 600, 75)]);
        assert_eq!(adapter_modes(), dx::ADAPTER_MODES.to_vec());
    }
}
