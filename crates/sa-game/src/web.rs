//! The web front end (wasm32, WEB-1): the same boot and main loop as the native `--window` mode, driven by the
//! page's worker (`web/worker.js`): the game folder is the user's own install ([`crate::webfs`]), the device draws
//! into an `OffscreenCanvas` through wgpu (WebGPU, else WebGL2), and the page's keyboard / mouse events become the
//! platform input (window messages and DirectInput state), exactly as the winit window's do natively.
//!
//! The legacy address arena lives in the low linear memory at the original addresses (the module is linked above
//! [`crate::arena::WASM_LOW_END`]), and `orig!`'s registry dispatches through type-erased entries
//! ([`sa_core::wabi`]). There are no threads: the streaming thread is cooperative (`bind::coop`), the frame limiter
//! does not sleep (the page calls [`frame`] once per animation frame).

use std::cell::RefCell;

use sa_platform::fs::MemFs;
use sa_platform::input::{InputState, Key, MouseButton};
use wasm_bindgen::prelude::*;

use crate::app::{Session, merge_input, synthetic_input};

/// A console line (the crate's `println!` / `eprintln!`).
pub fn log_line(s: String) {
    web_sys::console::log_1(&s.into());
}

thread_local! {
    /// The running session (leaked `Box`). Not a `RefCell`: an iteration left through a JS exception (an unported
    /// callee, [`unported`]) never returns, so a borrow held across it would stay taken.
    static SESSION: std::cell::Cell<*mut Session> = const { std::cell::Cell::new(std::ptr::null_mut()) };
    /// The message of the unported callee that left the current iteration (see [`recover`]).
    static BLOCKED: RefCell<Option<String>> = const { RefCell::new(None) };
    static INPUT: RefCell<InputState> = RefCell::new(InputState::default());
}

#[wasm_bindgen]
extern "C" {
    /// `saGameIo.panic(message)`: the worker posts it to the page at once (the module aborts right after).
    #[wasm_bindgen(js_namespace = saGameIo, js_name = panic)]
    fn io_panic(message: &str);
}

/// A panic's message and location, with the JS stack (wasm function names), to the console and the page.
pub fn report_panic(msg: &str) {
    let stack: String = js_sys::Reflect::get(&js_sys::Error::new("panic"), &"stack".into())
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    let frames: Vec<&str> = stack
        .lines()
        .filter(|l| l.contains("wasm-function"))
        .map(|l| l.trim().trim_start_matches("at ").split(" (").next().unwrap_or(""))
        .filter(|l| !l.contains("panic") && !l.contains("report_panic") && !l.contains("Error>::new"))
        .take(30)
        .collect();
    let text = format!("{msg}\n  {}", frames.join("\n  "));
    web_sys::console::error_1(&text.clone().into());
    io_panic(&text);
}

/// Panic messages to the console and the page (a panic then aborts: wasm32 has no unwinding). The boot replaces
/// this hook with its own, which also reports.
#[wasm_bindgen(start)]
pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        report_panic(&format!("sa-game panicked: {info}"));
    }));
}

/// A development variable (`SA_GAME_STUB`, `SA_GAME_WALK`, `SA_GAME_SCRIPT_SKIP`, ...), from the page's URL query.
#[wasm_bindgen]
pub fn set_env(name: &str, value: &str) {
    crate::env::set(name, value);
}

/// `--start-in-world`: main.scm's intro mission is skipped.
#[wasm_bindgen]
pub fn set_start_in_world() {
    crate::native::set_start_in_world();
}

/// Registers file `id` of the game folder (`path` relative to it, `size` bytes); reads go to `saGameIo.read`.
#[wasm_bindgen]
pub fn add_game_file(path: &str, id: u32, size: f64) {
    crate::webfs::add(path, id, size as u64);
}

/// Creates the GPU device on `canvas` (WebGPU, else WebGL2) and installs it as the window the game's
/// `IDirect3D9::CreateDevice` uses. Returns the backend's name.
///
/// `backend`: `"webgpu"`, `"gl"` (WebGL2) or `""` (WebGPU when the browser has it, else WebGL2).
#[wasm_bindgen]
pub async fn init_gpu(canvas: web_sys::OffscreenCanvas, backend: String) -> Result<String, JsValue> {
    let (w, h) = (canvas.width().max(1), canvas.height().max(1));
    let mut desc = wgpu::InstanceDescriptor::new_with_display_handle(Box::new(WebDisplay));
    desc.backends = match backend.as_str() {
        "webgpu" => wgpu::Backends::BROWSER_WEBGPU,
        "gl" => wgpu::Backends::GL,
        _ => wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
    };
    let instance = wgpu::util::new_instance_with_webgpu_detection(desc).await;
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::OffscreenCanvas(canvas))
        .map_err(|e| JsValue::from_str(&format!("create_surface: {e}")))?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .map_err(|e| JsValue::from_str(&format!("no GPU adapter: {e}")))?;
    let info = adapter.get_info();
    let limits = if info.backend == wgpu::Backend::Gl {
        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
    } else {
        wgpu::Limits::default().using_resolution(adapter.limits())
    };
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("sa-game web"),
            required_features: adapter.features() & sa_rw_wgpu::d3d9::WgpuDevice::optional_features(),
            required_limits: limits,
            experimental_features: Default::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: Default::default(),
        })
        .await
        .map_err(|e| JsValue::from_str(&format!("request_device: {e}")))?;
    device.on_uncaptured_error(std::sync::Arc::new(|e| {
        web_sys::console::error_1(&format!("wgpu: {e}").into());
    }));
    let caps = surface.get_capabilities(&adapter);
    let format = caps
        .formats
        .iter()
        .copied()
        .find(|f| !f.is_srgb())
        .or(caps.formats.first().copied())
        .ok_or_else(|| JsValue::from_str("the canvas has no surface format"))?;
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: w,
        height: h,
        present_mode: wgpu::PresentMode::AutoNoVsync,
        desired_maximum_frame_latency: 2,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
    };
    surface.configure(&device, &config);
    crate::d3d9::set_window(crate::d3d9::WindowTarget {
        surface,
        config,
        device,
        queue,
    });
    Ok(format!("{:?} ({}, {:?})", info.backend, info.name, format))
}

/// The web "display" (WebGL2 surfaces need one; there is nothing behind it).
#[derive(Debug)]
struct WebDisplay;

impl wgpu::rwh::HasDisplayHandle for WebDisplay {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
        Ok(wgpu::rwh::DisplayHandle::web())
    }
}

/// The page's SA:MP options (`?samp=HOST:PORT&proxy=ws://...&name=NAME`, passed as `SA_SAMP`, `SA_SAMP_PROXY`,
/// `SA_SAMP_NAME`; the auth table's text as `SA_SAMP_AUTH`).
fn samp_options() -> crate::app::SampOptions {
    crate::app::SampOptions {
        server: crate::env::var_os("SA_SAMP").filter(|s| !s.is_empty()),
        name: crate::env::var_os("SA_SAMP_NAME").unwrap_or_else(|| "WebCJ".into()),
        auth: None,
        proxy: crate::env::var_os("SA_SAMP_PROXY"),
        ui_demo: crate::env::var_os("SA_SAMP_UI_DEMO").is_some(),
        game: std::path::PathBuf::new(),
    }
}

/// The platform setup and the boot (WinMain up to its main loop), from the registered game folder.
#[wasm_bindgen]
pub fn boot() -> Result<(), JsValue> {
    use sa_platform::fs::GameFs;
    let game = crate::webfs::WebFs;
    let exe = game
        .read("gta_sa.exe")
        .map_err(|e| JsValue::from_str(&format!("gta_sa.exe: {e} (choose your GTA San Andreas folder)")))?;
    crate::boot::NO_PROBE.store(true, std::sync::atomic::Ordering::SeqCst);
    crate::app::QUIET_FRAMES.store(true, std::sync::atomic::Ordering::SeqCst);
    if crate::env::var_os("SA_WEB_SAMPLE").is_some() {
        SAMPLING.store(true, std::sync::atomic::Ordering::Relaxed);
        fn sample(callee: usize, _n: u32) {
            maybe_sample(sa_core::orig::registry::describe(callee));
        }
        let f: fn(usize, u32) = sample;
        sa_core::wabi::SAMPLER.store(f as usize, std::sync::atomic::Ordering::Relaxed);
    }
    let s = crate::app::boot_with(
        Box::new(game),
        Box::new(MemFs::default()),
        exe,
        &format!("your game folder ({} files)", crate::webfs::count()),
        "browser memory (not kept)",
        &samp_options(),
    )
    .map_err(|e| JsValue::from_str(&e))?;
    SESSION.with(|c| c.set(Box::into_raw(Box::new(s))));
    // from now on an unported callee leaves the main-loop iteration instead of aborting the module
    if crate::env::var_os("SA_WEB_NO_RECOVER").is_none() {
        sa_core::orig::set_unported_handler(unported);
    }
    Ok(())
}

/// One main-loop iteration with the input gathered since the last one; false once the loop has stopped. With
/// `SA_WEB_AUTO_INPUT=1` the offscreen mode's synthetic input is added (unattended evidence runs).
#[wasm_bindgen]
pub fn frame() -> bool {
    crate::d3d9::PRESENTED_THIS_TASK.store(0, std::sync::atomic::Ordering::Relaxed);
    if SAMPLING.load(std::sync::atomic::Ordering::Relaxed) {
        thread_local! {
            static LAST_STATS: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
        }
        let now = js_sys::Date::now();
        if now - LAST_STATS.with(|l| l.get()) > 2000.0 {
            LAST_STATS.with(|l| l.set(now));
            if let Some(s) = crate::d3d9::with_device(|d| (d.stats.draws, d.stats.flushes, d.stats.pipelines)) {
                log_line(format!(
                    "gpu: {} draws, {} flushes, {} pipelines, {} presents",
                    s.0,
                    s.1,
                    s.2,
                    crate::d3d9::PRESENTS.load(std::sync::atomic::Ordering::Relaxed)
                ));
            }
        }
    }
    use std::sync::atomic::Ordering;
    let presents = crate::d3d9::PRESENTS.load(Ordering::Relaxed);
    // Most main-loop iterations do not run a game frame: the page runs the loop without the frame limiter's sleep,
    // so between two frames there are many iterations that only peek messages. The game reads its input
    // (DirectInput state, CPad) in the iterations that run a frame, which are the ones that present. So the mouse
    // motion, the wheel and the keys pressed since the last frame stay pending until an iteration presents (natively
    // the limiter sleeps and nearly every iteration is a frame). Window-message events go to the queue once.
    // A key released before any frame saw it held is released after the next frame instead (a quick tap, or one
    // shorter than a slow frame, still reaches CPad).
    let input = INPUT.with(|i| {
        let mut i = i.borrow_mut();
        DEFERRED_UP.with(|d| {
            d.borrow_mut().retain(|&(k, at)| {
                if presents > at {
                    i.key_up(k);
                    false
                } else {
                    true
                }
            })
        });
        let out = i.clone();
        i.events.clear();
        out
    });
    let more = with_session(|s| {
        let input = if crate::env::var_os("SA_WEB_AUTO_INPUT").is_some() {
            merge_input(&input, &synthetic_input(s.n))
        } else {
            input
        };
        s.iterate(input)
    })
    .unwrap_or(false);
    if crate::d3d9::PRESENTS.load(Ordering::Relaxed) != presents {
        INPUT.with(|i| i.borrow_mut().end_frame());
    }
    more
}

thread_local! {
    /// Keys released before a frame saw them held, with the present count at their key-down (see [`frame`]).
    static DEFERRED_UP: RefCell<Vec<(Key, u32)>> = const { RefCell::new(Vec::new()) };
    /// The present count when each held key went down.
    static DOWN_AT: RefCell<Vec<(Key, u32)>> = const { RefCell::new(Vec::new()) };
}

/// `f` with the session, if the boot made one.
#[allow(unsafe_code)]
fn with_session<R>(f: impl FnOnce(&mut Session) -> R) -> Option<R> {
    let p = SESSION.with(|c| c.get());
    // SAFETY: `p` is the leaked box `boot` stored (or null); the module is single-threaded and the page never calls
    // into it re-entrantly, so this is the only reference. An iteration left by a JS exception drops nothing, so the
    // box stays valid.
    (!p.is_null()).then(|| f(unsafe { &mut *p }))
}

/// The prefix of the JS exception [`unported`] throws (the worker recognises it).
pub const BLOCKED_TAG: &str = "sa-game blocked: ";

/// `sa_core::orig::unported`'s handler on wasm32: natively the panic of an unported callee unwinds to the main loop,
/// which reports a blocked iteration and goes on. wasm32 has no unwinding (a panic aborts the module), so this throws
/// a JS exception instead: the engine drops the wasm frames up to the worker's `frame()` call without running Rust
/// destructors, the worker resets the shadow stack pointer and calls [`recover`], which ends the iteration as natively.
/// Game memory is left exactly as the native unwind leaves it (the original code has no destructors to run); what
/// differs is Rust state that an unwind would release: guards held across the call (see `recover`).
fn unported(msg: std::fmt::Arguments<'_>) -> ! {
    let msg = msg.to_string();
    BLOCKED.with(|b| *b.borrow_mut() = Some(msg.clone()));
    wasm_bindgen::throw_str(&format!("{BLOCKED_TAG}{msg}"))
}

/// After the worker caught [`unported`]'s exception (and reset the shadow stack pointer): ends the iteration as a
/// blocked step, as natively. False when there is no pending blocked callee, when the platform lock was left taken
/// (a guard held across the call: the state is not sane, so the session stops) or when the loop stops by its own
/// rules (`CGame::Initialise` incomplete, the same failure three times).
#[wasm_bindgen]
pub fn recover() -> bool {
    let Some(msg) = BLOCKED.with(|b| b.borrow_mut().take()) else {
        return false;
    };
    if !crate::bind::lock_is_free() {
        web_sys::console::error_1(&format!("{BLOCKED_TAG}{msg}: the platform lock is still taken; not resuming").into());
        return false;
    }
    log_line(format!("{BLOCKED_TAG}{msg}"));
    with_session(|s| s.finish_blocked(&msg)).unwrap_or(false)
}

static SAMPLING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// `SA_WEB_SAMPLE=1`: at most every 3 s of real time, the JS stack (wasm function names) of the current call, so a
/// main-loop iteration that does not return shows where it spends its time.
pub fn maybe_sample(what: &str) {
    use std::sync::atomic::Ordering;
    if !SAMPLING.load(Ordering::Relaxed) {
        return;
    }
    thread_local! {
        static LAST: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    }
    let now = js_sys::Date::now();
    if now - LAST.with(|l| l.get()) < 1000.0 {
        return;
    }
    LAST.with(|l| l.set(now));
    let stack: String = js_sys::Reflect::get(&js_sys::Error::new("sample"), &"stack".into())
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default();
    let frames: Vec<&str> = stack
        .lines()
        .filter(|l| l.contains("wasm-function"))
        .map(|l| l.trim().trim_start_matches("at ").split(" (").next().unwrap_or(""))
        .take(40)
        .collect();
    log_line(format!(
        "sample at {what}: game time {} ms, gGameState {}\n  {}",
        sa_core::mem::GPtr::<u32>::from_addr(0xB7CB84).read(),
        crate::winmain::game_state(),
        frames.join("\n  ")
    ));
}

/// Main-loop iterations run so far.
#[wasm_bindgen]
pub fn iterations() -> u32 {
    with_session(|s| s.n).unwrap_or(0)
}

/// The main loop's `gGameState` (9 = in game).
#[wasm_bindgen]
pub fn game_state() -> u32 {
    crate::winmain::game_state()
}

/// In-game frames presented so far.
#[wasm_bindgen]
pub fn in_game_presents() -> u32 {
    crate::d3d9::frame_times().len() as u32
}

/// Frames presented so far (every `Present`).
#[wasm_bindgen]
pub fn presents() -> u32 {
    crate::d3d9::PRESENTS.load(std::sync::atomic::Ordering::Relaxed)
}

/// A DOM `KeyboardEvent.code` going down (`repeat`: auto-repeat) or up.
#[wasm_bindgen]
pub fn key(code: &str, down: bool, repeat: bool) {
    let Some(k) = key_of(code) else { return };
    let now = crate::d3d9::PRESENTS.load(std::sync::atomic::Ordering::Relaxed);
    INPUT.with(|i| {
        let mut i = i.borrow_mut();
        match (down, repeat) {
            (true, false) => {
                DEFERRED_UP.with(|d| d.borrow_mut().retain(|&(x, _)| x != k));
                DOWN_AT.with(|d| {
                    let mut d = d.borrow_mut();
                    d.retain(|&(x, _)| x != k);
                    d.push((k, now));
                });
                i.key_down(k)
            }
            (true, true) => i.key_repeat(k),
            (false, _) => {
                let at = DOWN_AT.with(|d| {
                    let mut d = d.borrow_mut();
                    let at = d.iter().find(|&&(x, _)| x == k).map(|&(_, a)| a);
                    d.retain(|&(x, _)| x != k);
                    at
                });
                // no frame since the key went down: release it after the next one
                match at {
                    Some(at) if at >= now && i.held.contains(&k) => DEFERRED_UP.with(|d| d.borrow_mut().push((k, at))),
                    _ => i.key_up(k),
                }
            }
        }
    });
}

/// Text the layout produced (`WM_CHAR`).
#[wasm_bindgen]
pub fn text(s: &str) {
    INPUT.with(|i| {
        let mut i = i.borrow_mut();
        for c in s.chars() {
            i.events.push(sa_platform::input::InputEvent::Char(c));
        }
    });
}

/// Relative pointer motion (pointer-lock `movementX/Y`).
#[wasm_bindgen]
pub fn mouse_move(dx: f64, dy: f64) {
    INPUT.with(|i| {
        let mut i = i.borrow_mut();
        i.mouse_delta.0 += dx;
        i.mouse_delta.1 += dy;
    });
}

/// Absolute pointer position in the game's client pixels (the page scales the canvas position to the video mode):
/// `WM_MOUSEMOVE`, which the menus read for their cursor (no pointer lock in the menus).
#[wasm_bindgen]
pub fn mouse_pos(x: i32, y: i32) {
    INPUT.with(|i| i.borrow_mut().events.push(sa_platform::input::InputEvent::MouseMove { x, y }));
}

/// A DOM mouse button (0 left, 1 middle, 2 right, 3 back, 4 forward) going down or up.
#[wasm_bindgen]
pub fn mouse_button(button: u32, down: bool) {
    let b = match button {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        3 => MouseButton::X1,
        4 => MouseButton::X2,
        _ => return,
    };
    INPUT.with(|i| {
        let mut i = i.borrow_mut();
        if down { i.mouse_down(b) } else { i.mouse_up(b) }
    });
}

/// Wheel lines (positive = away from the user).
#[wasm_bindgen]
pub fn wheel(lines: f32) {
    INPUT.with(|i| {
        let mut i = i.borrow_mut();
        i.wheel += lines;
        i.events.push(sa_platform::input::InputEvent::Wheel(lines));
    });
}

/// Focus lost: every held key and button is released.
#[wasm_bindgen]
pub fn blur() {
    DEFERRED_UP.with(|d| d.borrow_mut().clear());
    DOWN_AT.with(|d| d.borrow_mut().clear());
    INPUT.with(|i| i.borrow_mut().release_all());
}

/// The platform key of a DOM `KeyboardEvent.code` (layout-independent physical keys, as winit's `KeyCode`).
fn key_of(code: &str) -> Option<Key> {
    if let Some(l) = code.strip_prefix("Key")
        && l.len() == 1
        && l.as_bytes()[0].is_ascii_uppercase()
    {
        const KEYS: [Key; 26] = [
            Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H, Key::I, Key::J, Key::K, Key::L, Key::M,
            Key::N, Key::O, Key::P, Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X, Key::Y, Key::Z,
        ];
        return Some(KEYS[usize::from(l.as_bytes()[0] - b'A')]);
    }
    if let Some(d) = code.strip_prefix("Digit").and_then(|d| d.parse::<u8>().ok()) {
        return Some(Key::Digit(d));
    }
    if let Some(d) = code.strip_prefix("Numpad").and_then(|d| d.parse::<u8>().ok()) {
        return Some(Key::Numpad(d));
    }
    if let Some(n) = code.strip_prefix('F').and_then(|d| d.parse::<u8>().ok()) {
        return Some(Key::Fn(n));
    }
    Some(match code {
        "NumpadDivide" => Key::NumpadDivide,
        "NumpadMultiply" => Key::NumpadMultiply,
        "NumpadSubtract" => Key::NumpadSubtract,
        "NumpadAdd" => Key::NumpadAdd,
        "NumpadDecimal" => Key::NumpadDecimal,
        "NumpadEnter" => Key::NumpadEnter,
        "NumLock" => Key::NumLock,
        "Space" => Key::Space,
        "ShiftLeft" => Key::ShiftLeft,
        "ShiftRight" => Key::ShiftRight,
        "ControlLeft" => Key::ControlLeft,
        "ControlRight" => Key::ControlRight,
        "AltLeft" => Key::AltLeft,
        "AltRight" => Key::AltRight,
        "MetaLeft" => Key::MetaLeft,
        "MetaRight" => Key::MetaRight,
        "ContextMenu" => Key::ContextMenu,
        "CapsLock" => Key::CapsLock,
        "ArrowUp" => Key::Up,
        "ArrowDown" => Key::Down,
        "ArrowLeft" => Key::Left,
        "ArrowRight" => Key::Right,
        "Insert" => Key::Insert,
        "Delete" => Key::Delete,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "PrintScreen" => Key::PrintScreen,
        "ScrollLock" => Key::ScrollLock,
        "Pause" => Key::Pause,
        "BracketLeft" => Key::BracketLeft,
        "BracketRight" => Key::BracketRight,
        "Backslash" => Key::Backslash,
        "Semicolon" => Key::Semicolon,
        "Quote" => Key::Quote,
        "Backquote" => Key::Backquote,
        "Comma" => Key::Comma,
        "Period" => Key::Period,
        "Slash" => Key::Slash,
        "Minus" => Key::Minus,
        "Equal" => Key::Equal,
        "IntlBackslash" => Key::IntlBackslash,
        "Escape" => Key::Escape,
        "Tab" => Key::Tab,
        "Enter" => Key::Enter,
        "Backspace" => Key::Backspace,
        _ => return None,
    })
}
