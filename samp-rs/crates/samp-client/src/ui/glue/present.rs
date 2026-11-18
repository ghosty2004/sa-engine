//! The Present and Reset wrappers of the device proxy, and samp's per-frame Present pass.

use sa_samp_api::hud::Hud as _;
use std::panic::{AssertUnwindSafe, catch_unwind};

use samp_ui::keys::Overlay;

use super::adapters::{gta_menu_active, key_polled, net_stats_values, set_hud};
use super::editor::editor_pass;
use super::frame::DIRECT_MODE_LINE;
use super::overlays::{bubbles, map_click, nametags};
use super::{State, UI, exec_ops, try_with_ui};
use crate::ui::core::{CursorRequest, Outgoing};
use crate::ui::d3d::renderer as render;
use crate::ui::d3d::{self, HResult, Rect};
use crate::ui::render::{BubbleDraw, CursorReq, LabelDraw, TagDraw};
use crate::ui::session::{PassInput, UiSession};

pub(super) extern "system" fn present_hook(
    _this: *mut std::ffi::c_void,
    a: *const Rect,
    b: *const Rect,
    c: isize,
    d: *const std::ffi::c_void,
) -> HResult {
    let _ = catch_unwind(present_pass);
    d3d::call_present(a, b, c, d)
}

pub(super) extern "system" fn reset_hook(_this: *mut std::ffi::c_void, params: *mut std::ffi::c_void) -> HResult {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let ops = try_with_ui(|st| {
            if let Some(r) = st.renderer.as_mut() {
                r.on_lost();
            }
            crate::plates::on_lost();
            // Lost device: an open chat input is closed (cursor mode 0, immediate restore) and so is an open
            // scoreboard (cursor mode 0, delayed restore).
            let mut ops = Vec::new();
            if st.core.input.is_open() {
                st.core.input.close();
                ops.extend(st.lock.request_mode0(true));
            }
            if st.core.scoreboard.is_open() {
                st.core.scoreboard.close();
                ops.extend(st.lock.request_mode0(false));
            }
            ops
        });
        if let Some(ops) = ops {
            exec_ops(ops);
        }
        // Then the "SetCursor(NULL) once" flag of WM_SETCURSOR is re-armed.
        crate::ui::wndproc::on_lost_device();
        crate::ui::mattext::on_lost();
        // Last in samp's lost-device pass: the model-preview textures.
        crate::ui::preview::on_lost();
    }));
    let hr = d3d::call_reset(params);
    crate::crash::note(crate::crash::crumb::DEVICE_RESET, 0, hr as u32);
    crate::log!("ui: device Reset -> {hr:#x}");
    if hr >= 0 {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            crate::ui::wndproc::on_reset_device();
            let ops = try_with_ui(|st| {
                crate::plates::on_reset();
                if let Some(r) = st.renderer.as_mut() {
                    r.on_reset();
                    // samp re-creates the chat render target through its construction routine, which prints
                    // the direct-mode line again.
                    if st.core.settings.direct_mode {
                        st.core.print_debug(DIRECT_MODE_LINE.as_bytes());
                    }
                }
                // Last step of samp's reset handling: unless the GTA menu is active, cursor mode 2, then mode 0
                // (immediate) with the input restore run at once.
                if crate::adapter::original::process().menu_active_dword() == 0 {
                    let mut ops = st.lock.set_mode(2, false);
                    ops.extend(st.lock.restore_now(true));
                    ops
                } else {
                    Vec::new()
                }
            });
            if let Some(ops) = ops {
                exec_ops(ops);
            }
        }));
    }
    hr
}

/// What the Present pass gathers before it draws.
struct Gathered {
    overlay: Option<Overlay>,
    /// The scoreboard or an F5 / F10 overlay hides the rest of the UI.
    sb_open: bool,
    tags: Vec<TagDraw>,
    bubbles: Vec<BubbleDraw>,
    labels: Vec<LabelDraw>,
    vehicle_labels: Option<Vec<([f32; 3], String)>>,
    audio_title: Option<Vec<u8>>,
    net_stats: Option<Vec<u8>>,
}

/// samp's per-frame Present pass: screenshot, menu check, HUD flags, then the drawing steps in samp's order
/// (docs/re/subsystems/ui-rendering.md, "2.3 Present: per-frame draw order"; steps below use its numbering).
fn present_pass() {
    let Ok(mut g) = UI.try_lock() else { return };
    let Some(st) = g.as_mut() else { return };
    // Step 1: F8 screenshot, before anything of this frame is drawn.
    take_screenshot(st);
    // Step 3: GTA pause menu → no samp UI; only the map right-click.
    if gta_menu_active() {
        pause_menu_map_click(st);
        return;
    }
    run_cursor_requests(st);
    let input = gather(st);
    if st.renderer.is_none() {
        return;
    }
    update_layouts(st);
    let cursor = render::cursor_pos();
    draw_frame(st, &input, cursor);
    after_frame(st, input.sb_open, cursor);
}

/// Step 1: the F8 screenshot.
fn take_screenshot(st: &mut State) {
    if std::mem::take(&mut st.core.requests.screenshot) {
        let (path, index) = st.core.screenshot_target();
        let ok = d3d::screenshot(path.to_string_lossy().as_bytes(), render::hwnd());
        st.core.screenshot_done(ok.then_some(index));
    }
}

/// Step 3: with the GTA pause menu up, only the map right-click (RPC 119).
fn pause_menu_map_click(st: &mut State) {
    if st.core.netgame
        && let Some(p) = map_click()
    {
        // samp sends from this pass and RakNet's own thread transmits it; our session tick does not run
        // while the menu is up, so send and flush here (queued for the next tick if the session is busy).
        if !crate::dll::send_click_map(p) {
            st.core.push_outgoing(Outgoing::ClickMap(p));
        }
    }
}

/// The cursor-mode requests queued by input and RPC handling, in order.
fn run_cursor_requests(st: &mut State) {
    for req in std::mem::take(&mut st.core.requests.cursor) {
        let ops = match req {
            CursorRequest::Mode0 { immediate } => st.lock.request_mode0(immediate),
            CursorRequest::Mode3 => st.lock.request_mode3(),
        };
        exec_ops(ops);
    }
}

/// Steps 4b–9 up to the drawing: overlay keys, HUD flags, nametags, bubbles, labels, the audio title and the F5
/// text.
fn gather(st: &mut State) -> Gathered {
    // Steps 5–8: scoreboard, else F5 (connected) / F10 held → HUD off and
    // only the nametag passes; else F1 held opens the help dialog. Keys are polled with
    // GetAsyncKeyState (any bit set), each only when the earlier ones did not apply.
    let sb_open = st.core.scoreboard.is_open();
    let mut overlay = None;
    if st.core.netgame && !sb_open {
        if key_polled(0x74) && st.core.connected {
            overlay = Some(Overlay::NetStats);
        } else if key_polled(0x79) {
            overlay = Some(Overlay::F10);
        } else if key_polled(0x70) {
            st.core.open_help();
        }
    }
    // Steps 5 / 9: the HUD flags are written on every Present.
    set_hud(UiSession::hud_visible(&st.core, overlay));
    let sb_open = sb_open || overlay.is_some();
    let tags = nametags(st);
    let bubbles = if sb_open { Vec::new() } else { bubbles(st) };
    // Step 4b: the audio stream title line, when the netgame exists.
    let audio_title = if st.core.netgame { crate::audio::title_text() } else { None };
    let net_stats = (overlay == Some(Overlay::NetStats)).then(|| net_stats_text(st));
    let labels = crate::entities::label_draws();
    let vehicle_labels = crate::vehlabels::draws();
    Gathered { overlay, sb_open, tags, bubbles, labels, vehicle_labels, audio_title, net_stats }
}

/// The F5 text with the download / upload rates sampled now.
fn net_stats_text(st: &mut State) -> Vec<u8> {
    let (sent, received) = st.net_bytes;
    let (down, up) = st.rates.sample(samp_proto::time::Instant::now(), sent, received);
    let mut base = st.net_stats.clone();
    base.download_kbs = down;
    base.upload_kbs = up;
    crate::ui::core::net_stats_text(&net_stats_values(&base))
}

/// Font rebuild when asked or when the size changed, then the session's layouts (dialog, chat input, click
/// rects).
fn update_layouts(st: &mut State) {
    let Some(r) = st.renderer.as_mut() else { return };
    st.session.update_layouts(&mut st.core, r);
}

/// Steps 4–12: the shared pass (chat, nametags, bubbles, scoreboard, death window, dialog, overlays, the
/// class-selection panel) on the D3D9 backend.
fn draw_frame(st: &mut State, g: &Gathered, cursor: (i32, i32)) {
    let Some(r) = st.renderer.as_mut() else { return };
    let input = PassInput {
        overlay: g.overlay,
        tags: &g.tags,
        bubbles: &g.bubbles,
        labels: &g.labels,
        vehicle_labels: g.vehicle_labels.as_deref(),
        audio_title: g.audio_title.as_deref(),
        net_stats: g.net_stats.as_deref(),
        netgame_object: st.netgame_object,
        cursor,
    };
    let lock = &mut st.lock;
    st.session.draw(&st.core, r, &input, &mut |req| match req {
        CursorReq::Mode2 => exec_ops(lock.request_mode2()),
    });
}

/// Step 12 after the frame (the class-selection panel was drawn by the pass): the object editor and selection,
/// textdraw selection, then leaving cursor mode 2 when no mode-2 UI is left.
fn after_frame(st: &mut State, sb_open: bool, cursor: (i32, i32)) {
    let Some(r) = st.renderer.as_mut() else { return };
    // Step 12: object editor and object selection.
    if !sb_open {
        editor_pass(&mut st.lock, r);
    }
    // End of step 12: textdraw selection per frame (spec "14.4 Selection mode").
    if st.core.textdraws.selection.active && !sb_open {
        // Cursor mode 2, HUD off, then the hover test.
        exec_ops(st.lock.request_mode2());
        set_hud(false);
        st.core.textdraws.hover(cursor.0, cursor.1);
    }
    // Leaving a mode-2 UI: back to mode 0 with an immediate restore, except after the spawn (delayed restore).
    if !UiSession::wants_mode2(&st.core) && st.lock.mode() == 2 {
        let immediate = !std::mem::take(&mut st.spawn_release);
        exec_ops(st.lock.request_mode0(immediate));
    }
}
