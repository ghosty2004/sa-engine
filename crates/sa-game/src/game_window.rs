//! `--window`: the game in a native window.
//!
//! The window exists before the boot, as in the original (`WinMain` creates the window before RW starts): its GPU
//! device and surface are handed to the D3D9 seam ([`d3d9::set_window`]), so the device `IDirect3D9::CreateDevice`
//! creates draws on the window's GPU device and every `Present` draws the back buffer into the window surface. Then
//! the boot runs, and every redraw of the event loop runs one main-loop iteration (`WinMain`'s loop body: one window
//! message, or one step of the `gGameState` machine) with the window's input: its keyboard and mouse events become
//! window messages (the window procedure feeds `CPad`), its held keys and buttons and relative mouse motion the
//! DirectInput state. Frame pacing is the original's: the in-game frame runs only when the frame limiter's interval
//! has passed (`winmain.rs`, state 9), the iterations between wait out the rest of the interval.
//!
//! The probes that run boot steps in a forked child first are off in this mode (the window's GPU device and display
//! connection exist before the boot; a forked child must not touch them).

use std::sync::Arc;
use web_time::Instant;

use sa_platform::input::{InputEvent, InputState};
use sa_platform::native::{Window, WindowApp, WindowConfig, run_windowed, set_pointer_capture};
use sa_rw_wgpu::d3d9::WgpuDevice;

use crate::app::{Args, Session, merge_input, synthetic_input};
use crate::d3d9;

/// The window's size (the game's default video mode, 800 x 600).
const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    use std::task::{Context, Poll, Wake, Waker};
    struct W(std::thread::Thread);
    impl Wake for W {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(W(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::park();
    }
}

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
        match crate::app::boot(self.args) {
            Ok(s) => self.session = Some(s),
            Err(e) => self.error = Some(e),
        }
        // DirectInput exclusive mode: the game owns the pointer
        let _ = set_pointer_capture(&window, true);
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
        let input = if self.args.auto_input {
            merge_input(input, &synthetic_input(s.n))
        } else {
            input.clone()
        };
        let go_on = s.iterate(input);
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
