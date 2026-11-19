//! The mod host in the game (feature `mods`, docs/arch/ADR-004-mods.md): the mods of `--mods DIR` (natively) or the
//! page's `?mods=a,b` (web) are loaded before the boot, and called at the game's fixed points:
//!
//! * the settings phase ([`settings`]): once in the boot, after `RsEventHandler(rsINITIALIZE)` has read the
//!   settings file (`CMenuManager::LoadSettings`) and before `RsEventHandler(rsRWINITIALIZE)` selects the video mode
//!   (`psSelectDevice`) and creates the D3D device;
//! * the frame begin ([`frame_boundary`]), at the start of every main-loop iteration: the game events;
//! * the overlay phase ([`overlay`]), at every `IDirect3DDevice9::Present` after the game's own drawing; what the
//!   mods draw goes to the render layer's 2D overlay (`WgpuDevice::draw_overlay`), outside the game's D3D9 state;
//!   then the frame-end event;
//! * the resize event ([`resize`]), from the window / canvas backends;
//! * the hooks: a call of a function a mod hooked is reported by `sa_core::orig::observe` ([`observed`]); its
//!   arguments and result reach the mods as values and handles, and what the hooks decide (changed arguments, a
//!   skipped call, a changed result) goes back to the call.
//!
//! The mods read and change the game through the engine's access layer `sa-access`.
//!
//! **Video modes (ADR-004 §4.1).** A mod's requested video mode is applied through the original's own state, so
//! the game sees what it would see had the player chosen it: the adapter (`IDirect3D9`, [`crate::display`]) lists
//! the real display's modes plus the requested size, and the request becomes the menu's video mode
//! (`FrontEndMenuManager.m_nDisplayVideoMode`, as `gta_sa.set` would hold it) when the original's mode list offers
//! it (`VideoMode::GetVideoModeList`: aspect 4:3, 5:4, 5:3, 16:10). Sizes the original's list leaves out (16:9 and
//! the rest) take the original's other selection path in `psSelectDevice`: `IsVMNotSelected` clear and
//! `gCurrentVideoMode` set (the mode a previous session selected); the menu's own video mode then names the closest
//! listed mode, since the menu's resolution row can only show listed modes ([`after_select`]). No port body changes.
//! Later requests (resize, overlay) are applied at the next main-loop iteration as the menu's resolution change
//! (`m_nPrefsVideoMode`, `VideoMode::SetVideoMode`, `CPostEffects::DoScreenModeDependentInitializations`), when the
//! adapter lists the size.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sa_core::mem::GPtr;
use sa_core::orig::observe::{self, Observed, Phase, Raw};
use sa_core::orig::registry;
use sa_mod_api::hook;
use sa_mod_host::Host;
use sa_mod_host::api::{Display, Game, GameAccess, HookCall, HookDecision, HookRegistry, Registry, VideoRequest};
use sa_platform::input::{InputEvent, InputState, MouseButton};
use sa_platform::win32::key_info;
use sa_rw_wgpu::d3d9::WgpuDevice;
use sa_types::{EntityRef, ObjectRef, PedRef, VehicleRef};

use crate::display;

thread_local! {
    /// The host (game thread only; `None` without mods).
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
    /// The overlay of the current frame (kept to reuse the allocations).
    static OVERLAY: RefCell<sa_platform::gpu::OverlayFrame> = RefCell::new(Default::default());
    /// The request of the settings phase and, for a size the original's list leaves out, the listed mode the menu
    /// shows instead ([`after_select`]).
    static SELECTED: Cell<Option<(VideoRequest, i32, Option<i32>)>> = const { Cell::new(None) };
    /// The game the mods read and change (also turns hooked values into handles and back).
    static GAME: RefCell<Option<Game>> = const { RefCell::new(None) };
}

/// The functions mods can hook: the entries of the address registry, by their names.
struct EngineRegistry;

impl HookRegistry for EngineRegistry {
    fn resolve(&self, name: &str) -> Option<u32> {
        registry::table().iter().find(|e| e.name == name).map(|e| e.addr)
    }

    fn watch(&self, target: u32, on: bool) {
        let t = registry::table();
        if let Ok(i) = t.binary_search_by_key(&target, |e| e.addr) {
            observe::watch(&t[i], on);
        }
    }
}

/// A hooked value as the mods see it: a pooled entity as its handle, any other pointer hidden.
fn mod_value(game: &GameAccess, raw: Raw) -> hook::Value {
    let entity = |e: EntityRef| match e {
        EntityRef::Ped(p) => hook::Value::Ped(p),
        EntityRef::Vehicle(v) => hook::Value::Vehicle(v),
        EntityRef::Object(o) => hook::Value::Object(o),
        _ => hook::Value::Pointer,
    };
    match raw {
        Raw::Float(f) => hook::Value::Float(f as f32),
        // ports declare some object pointers as integers (`this: u32`)
        Raw::Int(v) => u32::try_from(v)
            .ok()
            .and_then(|a| game.entity_at(a))
            .map_or(hook::Value::Int(v as i32), entity),
        Raw::Address(a) => game.entity_at(a).map_or(hook::Value::Pointer, entity),
        Raw::Other => hook::Value::Pointer,
    }
}

/// The game value of a mod's value: a handle as the address of its live entity.
fn game_value(game: &GameAccess, v: hook::Value) -> Option<Raw> {
    let entity = |e| game.entity_addr(e).ok().map(Raw::Address);
    match v {
        hook::Value::Int(i) => Some(Raw::Int(i64::from(i))),
        hook::Value::Float(f) => Some(Raw::Float(f64::from(f))),
        hook::Value::Ped(p) => entity(EntityRef::Ped(p)),
        hook::Value::Vehicle(v) => entity(EntityRef::Vehicle(v)),
        hook::Value::Object(o) => entity(EntityRef::Object(o)),
        hook::Value::Pointer => None,
    }
}

/// The result word of `hook_skip` / `hook_set_result` as the declared result type takes it: an integer, a float's
/// bits, or for a pointer the handle of a live ped, vehicle or object (0: null).
fn result_candidates(game: &GameAccess, word: i32) -> Vec<Raw> {
    let h = word as u32;
    let mut out = vec![Raw::Int(i64::from(word)), Raw::Float(f64::from(f32::from_bits(h)))];
    if h == 0 {
        out.push(Raw::Address(0));
    }
    for e in [EntityRef::Ped(PedRef(h)), EntityRef::Vehicle(VehicleRef(h)), EntityRef::Object(ObjectRef(h))] {
        if let Ok(a) = game.entity_addr(e) {
            out.push(Raw::Address(a));
        }
    }
    out
}

/// Applies what the hooks decided to the observed call.
fn decide(game: &GameAccess, o: &mut Observed<'_>, d: HookDecision) {
    for (i, v) in d.args {
        let Some(raw) = game_value(game, v) else { continue };
        // a port may declare an object pointer as an integer (`this: u32`)
        if !o.set_arg(i, raw)
            && let Raw::Address(a) = raw
        {
            o.set_arg(i, Raw::Int(i64::from(a)));
        }
    }
    if let Some(w) = d.skip
        && !o.skip(None)
    {
        result_candidates(game, w).into_iter().any(|r| o.skip(Some(r)));
    }
    if let Some(w) = d.result {
        result_candidates(game, w).into_iter().any(|r| o.set_result(r));
    }
}

/// `Game::Render2dStuff`: the mods' game-font strings and game sprites are drawn after it (the strings into the
/// buffer the frame's `DrawFonts` draws with the HUD's text).
const RENDER_2D_STUFF: u32 = 0x53E230;

thread_local! {
    /// Whether `Render2dStuff` is watched for the mods' 2D drawing (from the first queued item on).
    static TEXT_POINT: Cell<bool> = const { Cell::new(false) };
}

/// The mods' part of the game's 2D phase ([`Host::draw_2d`]).
fn draw_game_2d() {
    HOST.with(|h| {
        if let Ok(mut h) = h.try_borrow_mut()
            && let Some(h) = h.as_mut()
        {
            h.draw_2d();
        }
    });
}

/// The observer of the hooked functions: hands the call to the mods hooked there and applies their decisions.
fn observed(o: &mut Observed<'_>) {
    if o.entry.addr == RENDER_2D_STUFF && matches!(o.phase, Phase::After) {
        draw_game_2d();
    }
    let Some(game) = GAME.with_borrow(Clone::clone) else { return };
    // a hooked function that runs while the game is being changed for a mod is not reported
    let Ok(g) = game.0.try_borrow() else { return };
    let call = HookCall {
        args: (0..o.arg_count().min(hook::MAX_ARGS)).filter_map(|i| o.arg(i)).map(|r| mod_value(&g, r)).collect(),
        result: o.result().map(|r| mod_value(&g, r)),
    };
    drop(g);
    let phase = match o.phase {
        Phase::Before => hook::BEFORE,
        Phase::After => hook::AFTER,
    };
    // nor one that runs while a mod is being called
    let decision = HOST.with(|h| {
        let mut h = h.try_borrow_mut().ok()?;
        Some(h.as_mut()?.hook(o.entry.addr, phase, &call))
    });
    if let Some(d) = decision.filter(|d| !d.is_empty())
        && let Ok(g) = game.0.try_borrow()
    {
        decide(&g, o, d);
    }
}

/// `FrontEndMenuManager.m_nPrefsVideoMode`, `m_nDisplayVideoMode`.
const MENU_PREFS_VIDEO_MODE: usize = 0xBA681C;
const MENU_DISPLAY_VIDEO_MODE: usize = 0xBA6820;
/// R5G6B5 and X8R8G8B8, in the order RW enumerates the adapter's formats (`rwD3D9` mode list, 0x884788).
const RW_FORMATS: usize = 2;

fn rd(a: usize) -> i32 {
    GPtr::<i32>::from_addr(a).read()
}
fn wr(a: usize, v: i32) {
    GPtr::<i32>::from_addr(a).write(v)
}

/// The display as the mods see it.
fn mod_display(d: &display::Display) -> Display {
    Display { desktop: (d.desktop.0, d.desktop.1), modes: d.modes.iter().map(|m| (m.0, m.1)).collect() }
}

/// Loads the mods of `spec` (a folder natively, a list of mod names on the web). A mod that cannot be loaded is
/// reported and skipped; an unreadable `spec` is an error.
pub fn install(spec: &str) -> Result<(), String> {
    let mut host = Host::new(Box::new(|s| println!("sa-game: {s}")));
    host.set_display(mod_display(&display::reported()));
    let game = Game::new(sa_access::engine());
    host.set_game(game.clone());
    GAME.with_borrow_mut(|g| *g = Some(game));
    host.set_registry(Registry(Rc::new(EngineRegistry)));
    observe::set_handler(observed);
    host.set_network(sa_mod_host::platform_network(crate::env::var_os("SA_MODS_PROXY").as_deref()));
    host.set_http(sa_mod_host::platform_http());
    match sa_mod_host::platform_storage(None) {
        Ok(s) => host.set_storage(s),
        Err(e) => println!("sa-game: mods: {e}; mods that ask for storage are not loaded"),
    }
    let sources = sa_mod_host::sources(spec)?;
    if sources.is_empty() {
        println!("sa-game: mods: none found in {spec}");
    }
    for s in sources {
        if let Err(e) = host.load(s) {
            println!("sa-game: mod not loaded: {e}");
        }
    }
    HOST.with_borrow_mut(|h| *h = Some(host));
    Ok(())
}

fn with_host<R>(f: impl FnOnce(&mut Host) -> R) -> Option<R> {
    HOST.with_borrow_mut(|h| h.as_mut().map(f))
}

/// RW's index of the 32-bit mode `(w, h)` among `modes` (the adapter's list): RW numbers the desktop 0, then the
/// adapter's R5G6B5 modes, then its X8R8G8B8 modes (one entry per size and format, `rwD3D9`'s mode list 0x7F7540).
fn rw_index(modes: &[display::Mode], w: u32, h: u32) -> Option<i32> {
    let i = modes.iter().position(|m| (m.0, m.1) == (w, h))?;
    Some(1 + ((RW_FORMATS - 1) * modes.len() + i) as i32)
}

/// Whether the original's mode list (`VideoMode::GetVideoModeList`) offers the 32-bit exclusive mode `m`.
fn listed(m: display::Mode) -> bool {
    sa_core::video::mode_is_listed(&[m.0 as i32, m.1 as i32, 32, 1, m.2 as i32, 0], sa_core::video::VRAM_AVAIL.get())
}

/// The settings phase (see the module doc): the mods' `sa_settings`, then a requested video mode becomes the
/// adapter's mode list and the original's video-mode state, and the window shows it.
pub fn settings() {
    let Some(req) = with_host(|h| h.settings()).flatten() else { return };
    let reported = display::reported();
    let mut modes = reported.modes.clone();
    if !modes.iter().any(|m| (m.0, m.1) == (req.width, req.height)) {
        modes.push((req.width, req.height, 60));
    }
    let Some(idx) = rw_index(&modes, req.width, req.height) else { return };
    let refresh = modes.iter().find(|m| (m.0, m.1) == (req.width, req.height)).map_or(60, |m| m.2);
    let desktop = reported.desktop;
    display::set_adapter(display::Display { desktop, modes: modes.clone() });
    let placeholder = if listed((req.width, req.height, refresh)) {
        // as `gta_sa.set` holds the menu's choice: psSelectDevice takes it as the preferred and current mode
        wr(MENU_DISPLAY_VIDEO_MODE, idx);
        None
    } else {
        // psSelectDevice's other path: with IsVMNotSelected clear it takes gCurrentVideoMode as it is
        sa_core::video::IS_VM_NOT_SELECTED.set(0);
        sa_core::video::CURRENT_VIDEO_MODE.set(idx);
        wr(MENU_DISPLAY_VIDEO_MODE, 0);
        // the menu's row names the closest listed mode
        modes
            .iter()
            .filter(|m| listed(**m))
            .min_by_key(|m| (m.0 as i64 * m.1 as i64 - req.width as i64 * req.height as i64).abs())
            .and_then(|m| rw_index(&modes, m.0, m.1))
    };
    SELECTED.set(Some((req, idx, placeholder)));
    display::apply_window(req.width, req.height, req.fullscreen);
    println!(
        "sa-game: mods: video mode {}x{} ({}, RW mode {idx}, {}) requested; the adapter lists {} modes",
        req.width,
        req.height,
        if req.fullscreen { "full screen" } else { "window" },
        if placeholder.is_none() { "in the original's mode list: the menu's video mode" } else { "not in the original's mode list: gCurrentVideoMode" },
        modes.len()
    );
}

/// After `psSelectDevice` (the boot's rsRWINITIALIZE step): checks the mode the game selected and, for a size the
/// original's list leaves out, sets the menu's video mode (`m_nPrefsVideoMode` / `m_nDisplayVideoMode`) to the
/// closest listed mode, which its resolution row can show (a size outside the list would name no mode).
pub fn after_select() {
    let Some((req, idx, placeholder)) = SELECTED.get() else { return };
    let cur = sa_core::camerasize::call::rw_engine_get_current_video_mode();
    let mut vmi = [0i32; 6];
    sa_core::bindings::rw_engine_get_video_mode_info(GPtr::from_raw(vmi.as_mut_ptr()).cast(), cur);
    if cur != idx || (vmi[0], vmi[1]) != (req.width as i32, req.height as i32) {
        println!("sa-game: mods: the game selected RW mode {cur} ({}x{}), not the requested {idx}", vmi[0], vmi[1]);
    }
    if let Some(p) = placeholder {
        wr(MENU_PREFS_VIDEO_MODE, p);
        wr(MENU_DISPLAY_VIDEO_MODE, p);
    }
}

/// A video mode requested after the settings phase, applied at the start of a main-loop iteration as the menu's
/// resolution change, when the adapter lists the size (the RW mode list is built once, when RW opens).
pub fn frame_boundary() {
    let Some(req) = with_host(|h| {
        let _perf = crate::perf::scope(crate::perf::MODS);
        h.frame_begin();
        h.take_video_request()
    })
    .flatten() else {
        return;
    };
    if !crate::d3d9::with_device(|_| ()).is_some() {
        return;
    }
    let size = crate::d3d9::with_device(|d| sa_platform::gpu::D3d9Device::back_buffer_size(d));
    if size == Some((req.width, req.height)) {
        return;
    }
    let modes = display::adapter_modes();
    let Some(idx) = rw_index(&modes, req.width, req.height) else {
        println!(
            "sa-game: mods: video mode {}x{} is not in the adapter's mode list (built when the game started): not applied",
            req.width, req.height
        );
        return;
    };
    let m = modes.iter().find(|m| (m.0, m.1) == (req.width, req.height)).copied().unwrap_or((0, 0, 0));
    let menu = if listed(m) {
        idx
    } else {
        SELECTED.get().and_then(|s| s.2).unwrap_or(rd(MENU_PREFS_VIDEO_MODE))
    };
    // CMenuManager's MENU_ACTION_RESOLUTION (accept): the preference, SetVideoMode, the screen-mode dependent setup
    wr(MENU_PREFS_VIDEO_MODE, menu);
    wr(MENU_DISPLAY_VIDEO_MODE, menu);
    sa_hud::menu_mgr::set_video_mode(idx as u32);
    sa_hud::menu_mgr::post_effects_screen_mode_init();
    display::apply_window(req.width, req.height, req.fullscreen);
    println!("sa-game: mods: video mode {}x{} (RW mode {idx}) applied", req.width, req.height);
}

/// The mod API's form of a platform input event (`sa_mod_api::input`), `None` for one mods do not get.
fn input_event(e: &InputEvent) -> Option<(i32, i32, i32)> {
    use sa_mod_api::input as k;
    let button = |b: &MouseButton| match b {
        MouseButton::Left => Some(0),
        MouseButton::Right => Some(1),
        MouseButton::Middle => Some(2),
        MouseButton::X1 | MouseButton::X2 => None,
    };
    Some(match e {
        InputEvent::KeyDown { key, repeat } => (k::KEY_DOWN, i32::from(key_info(*key)?.vk), i32::from(*repeat)),
        InputEvent::KeyUp { key } => (k::KEY_UP, i32::from(key_info(*key)?.vk), 0),
        InputEvent::Char(c) => (k::CHAR, u32::from(*c) as i32, 0),
        InputEvent::MouseDown(b) => (k::MOUSE_BUTTON, button(b)?, 1),
        InputEvent::MouseUp(b) => (k::MOUSE_BUTTON, button(b)?, 0),
        InputEvent::MouseMove { x, y } => (k::MOUSE_MOVE, *x, *y),
        InputEvent::Wheel(lines) => (k::WHEEL, (lines * 120.0).round() as i32, 0),
        InputEvent::Focus(f) => (k::FOCUS, i32::from(*f), 0),
        InputEvent::Resized { .. } | InputEvent::Close => return None,
    })
}

/// Runs this iteration's input through the capturing mods: what one consumes never reaches the game, and while
/// they take the keyboard (or the mouse) the held keys (the mouse state) are hidden from the game too.
pub fn filter_input(input: &mut InputState) {
    let Some((keyboard, mouse)) = with_host(|h| {
        input.events.retain(|e| input_event(e).is_none_or(|(kind, a, b)| !h.input(kind, a, b)));
        let c = h.capture();
        (c.0, c.1)
    }) else {
        return;
    };
    if keyboard {
        input.held.clear();
        input.pressed.clear();
    }
    if keyboard || mouse {
        input.mouse_held.clear();
        input.mouse_delta = (0.0, 0.0);
        input.wheel = 0.0;
    }
}

/// Whether a mod shows the platform's cursor (`input_capture`): the platform then sends absolute pointer positions
/// and clicks, as in the menus.
pub fn cursor_visible() -> bool {
    with_host(|h| h.capture().2).unwrap_or(false)
}

/// Whether a mod takes the mouse (`input_capture`'s mouse) or shows the platform's cursor: the pointer is then
/// released and the mods get its absolute positions, whichever cursor is drawn.
pub fn takes_pointer() -> bool {
    with_host(|h| {
        let c = h.capture();
        c.1 || c.2
    })
    .unwrap_or(false)
}

/// The resize event: the window's client area / the canvas is now `width` x `height` pixels.
pub fn resize(width: u32, height: u32) {
    let screen = crate::d3d9::with_device(|d| sa_platform::gpu::D3d9Device::back_buffer_size(d)).unwrap_or((0, 0));
    with_host(|h| {
        h.set_display(mod_display(&display::reported()));
        h.resize(width, height, screen)
    });
}

/// The overlay phase: runs every mod's overlay callback and draws the result over the back buffer (call before
/// the device's present).
pub fn overlay(d: &mut WgpuDevice) {
    HOST.with_borrow_mut(|h| {
        let Some(h) = h.as_mut() else { return };
        let _perf = crate::perf::scope(crate::perf::MODS);
        let size = d.back_buffer().size();
        OVERLAY.with_borrow_mut(|frame| {
            frame.clear();
            h.overlay((size.width, size.height), frame);
            d.draw_overlay(frame);
        });
        if h.wants_2d() && !TEXT_POINT.get() {
            TEXT_POINT.set(true);
            EngineRegistry.watch(RENDER_2D_STUFF, true);
        }
        h.frame_end();
    });
}

/// The mods' state and cost, for the end of a run.
pub fn summary() -> Option<String> {
    HOST.with_borrow(|h| h.as_ref().map(Host::summary))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rw_numbers_32_bit_modes_after_the_16_bit_ones() {
        let modes = [(1920, 1080, 60), (800, 600, 60), (640, 480, 60)];
        assert_eq!(rw_index(&modes, 1920, 1080), Some(4));
        assert_eq!(rw_index(&modes, 640, 480), Some(6));
        assert_eq!(rw_index(&modes, 1024, 768), None);
    }
}
