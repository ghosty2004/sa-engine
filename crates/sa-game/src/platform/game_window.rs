//! `--window`: the game in a native window, the native backend of the shared front end ([`crate::frontend`]).
//!
//! The window exists before the boot, as in the original (`WinMain` creates the window before RW starts): its GPU
//! device and surface are handed to the D3D9 seam ([`d3d9::set_window`]), so the device `IDirect3D9::CreateDevice`
//! creates draws on the window's GPU device and every `Present` draws the back buffer into the window surface. Then
//! the boot runs, and every redraw of the event loop runs one main-loop iteration (`WinMain`'s loop body: one window
//! message, or one step of the `gGameState` machine) through [`FrontEnd::iterate`]: the window's keyboard and mouse
//! events go through the front end's input pipeline (window messages for the window procedure, which feeds `CPad`;
//! held keys and buttons and relative mouse motion as the DirectInput state). The pointer is captured in game and
//! free in the menus ([`in_menu`]). Frame pacing is the original's: the in-game frame runs only when the frame limiter's interval
//! has passed (`winmain.rs`, state 9), the iterations between wait out the rest of the interval.
//!
//! The probes that run boot steps in a forked child first are off in this mode (the window's GPU device and display
//! connection exist before the boot; a forked child must not touch them).

use std::sync::Arc;
use web_time::Instant;

use sa_platform::input::{InputEvent, InputState};
use sa_platform::native::{Window, WindowApp, WindowConfig, run_windowed, set_pointer_capture};
use sa_rw_wgpu::d3d9::WgpuDevice;

use crate::app::{Args, Session};
use crate::d3d9;
use crate::frontend::{CLIENT_HEIGHT as HEIGHT, CLIENT_WIDTH as WIDTH, FrontEnd, in_menu, to_client};

use sa_rw_wgpu::d3d9::block_on;

/// The window's surface and a GPU device that can present to it.
fn window_target(window: Arc<Window>) -> Result<d3d9::WindowTarget, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let size = window.inner_size();
    let surface = instance.create_surface(window).map_err(|e| e.to_string())?;
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }))
    .map_err(|e| format!("no GPU adapter for the window: {e}"))?;
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("sa-game window"),
        required_features: adapter.features() & WgpuDevice::optional_features(),
        required_limits: wgpu::Limits::default(),
        experimental_features: Default::default(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: Default::default(),
    }))
    .map_err(|e| format!("request_device: {e}"))?;
    let caps = surface.get_capabilities(&adapter);
    // An 8-bit UNORM back buffer as D3D9's X8R8G8B8. Compositors may list wider formats first (HDR / 10-bit
    // outputs offer Rgba16Unorm, which needs a device feature we do not request), so take the 8-bit ones
    // first, then any non-sRGB format the device can use.
    let device_features = device.features();
    let usable = |f: &wgpu::TextureFormat| device_features.contains(f.required_features());
    let format = [wgpu::TextureFormat::Bgra8Unorm, wgpu::TextureFormat::Rgba8Unorm]
        .into_iter()
        .find(|f| caps.formats.contains(f))
        .or_else(|| caps.formats.iter().copied().find(|f| !f.is_srgb() && usable(f)))
        .or_else(|| caps.formats.iter().copied().find(usable))
        .ok_or("the window surface has no format this device can use")?;
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        // the frame limiter paces the game (no wait for the vertical blank at `Present`)
        present_mode: wgpu::PresentMode::AutoNoVsync,
        desired_maximum_frame_latency: 2,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
    };
    surface.configure(&device, &config);
    Ok(d3d9::WindowTarget {
        surface,
        config,
        device,
        queue,
    })
}

struct GameWindow<'a> {
    args: &'a Args,
    session: Option<Session>,
    error: Option<String>,
    window: Option<Arc<Window>>,
    /// when the main loop started (after the boot)
    start: Instant,
    /// in-game presents shown in the title so far
    titled: usize,
    /// the pointer is captured (relative mode, as DirectInput's exclusive mode in game); released in the menus
    captured: bool,
    /// the shared front end's input pipeline
    front: FrontEnd,
}

impl GameWindow<'_> {
    /// The window's events since the last redraw into the front end (positions in client pixels).
    fn feed(&mut self, input: &InputState, size: (u32, u32)) {
        let f = &mut self.front;
        for e in &input.events {
            match *e {
                InputEvent::KeyDown { key, repeat } => f.key(key, true, repeat),
                InputEvent::KeyUp { key } => f.key(key, false, false),
                InputEvent::Char(c) => f.text(c),
                InputEvent::MouseDown(b) => f.mouse_button(b, true),
                InputEvent::MouseUp(b) => f.mouse_button(b, false),
                InputEvent::MouseMove { x, y } => {
                    let (x, y) = to_client(x, y, size);
                    f.mouse_pos(x, y)
                }
                InputEvent::Wheel(lines) => f.wheel(lines),
                e => f.event(e),
            }
        }
        f.mouse_motion(input.mouse_delta.0, input.mouse_delta.1);
        f.set_pads(input.pads.clone());
    }
}

impl WindowApp for GameWindow<'_> {
    fn init(&mut self, window: Arc<Window>) {
        match window_target(window.clone()) {
            Ok(t) => d3d9::set_window(t),
            Err(e) => {
                self.error = Some(e);
                return;
            }
        }
        crate::boot::NO_PROBE.store(true, std::sync::atomic::Ordering::SeqCst);
        match super::native_main::boot(self.args) {
            Ok(s) => self.session = Some(s),
            Err(e) => self.error = Some(e),
        }
        // DirectInput exclusive mode: the game owns the pointer (released in the menus, see `frame`)
        self.captured = set_pointer_capture(&window, true);
        self.window = Some(window);
        self.start = Instant::now();
    }

    fn resized(&mut self, width: u32, height: u32) {
        d3d9::resize_window(width, height);
    }

    fn frame(&mut self, input: &InputState) -> bool {
        let Some(s) = &mut self.session else { return false };
        if input.events.iter().any(|e| matches!(e, InputEvent::Close))
            || self.args.exit_after.is_some_and(|t| self.start.elapsed().as_secs_f32() >= t)
            || self.args.frames.is_some_and(|n| s.n >= n)
        {
            return false;
        }
        let mut size = (WIDTH, HEIGHT);
        if let Some(w) = &self.window {
            let menu = in_menu();
            if menu && self.captured {
                set_pointer_capture(w, false);
                // the menu draws its own cursor
                w.set_cursor_visible(false);
                self.captured = false;
            } else if !menu && !self.captured {
                self.captured = set_pointer_capture(w, true);
            }
            let s = w.inner_size();
            size = (s.width, s.height);
        }
        self.feed(input, size);
        let Some(s) = &mut self.session else { return false };
        let go_on = self.front.iterate(s);
        let presents = d3d9::frame_times();
        if presents.len() >= self.titled + 60
            && let (Some(w), Some(first)) = (&self.window, presents.first())
        {
            self.titled = presents.len();
            let secs = presents[presents.len() - 1].duration_since(*first).as_secs_f32();
            w.set_title(&format!(
                "sa-game — {} in-game frames, {:.1} fps",
                presents.len(),
                (presents.len() - 1) as f32 / secs.max(0.001)
            ));
        }
        go_on
    }
}

/// Opens the window, boots the game in it and runs the main loop until the window closes, `--exit-after` seconds or
/// `--frames` iterations have passed, or the loop stops.
pub fn run(args: &Args) -> Result<Session, String> {
    let mut app = GameWindow {
        args,
        session: None,
        error: None,
        window: None,
        start: Instant::now(),
        titled: 0,
        captured: false,
        front: FrontEnd::new(args.auto_input),
    };
    let mut cfg = WindowConfig::new("sa-game", WIDTH, HEIGHT);
    cfg.click_to_capture = false;
    run_windowed(cfg, &mut app)?;
    if let Some(e) = app.error {
        return Err(e);
    }
    let s = app.session.ok_or("the window closed before the boot")?;
    println!("sa-game: window closed after {} main-loop iterations", s.n);
    Ok(s)
}
