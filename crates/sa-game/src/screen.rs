//! The native window (`--window`) and the offscreen frame (`--screenshot`): the main loop runs the
//! ported frame steps with real input, and the frame is drawn through the D3D9 seam
//! (`sa_platform::gpu::D3d9Device` on `sa_rw_wgpu::d3d9::WgpuDevice`).
//!
//! `Game::RenderScene`/`Render2dStuff` and the RW driver are not ported yet, so what is drawn is a
//! status screen issued as a D3D9 call stream (clear + `DrawPrimitiveUP` of pre-transformed
//! quads), the same kind of calls `_rwD3D9Im2DRenderPrimitive` will make once ported.

use std::sync::Arc;

use sa_platform::gpu::{D3d9Device, PrimitiveType, Viewport, clear, fvf, rs, tss};
use sa_platform::input::InputState;
use sa_platform::native::{Window, WindowApp, WindowConfig, run_windowed};
use sa_rw_wgpu::d3d9::WgpuDevice;

use crate::{bind, boot};

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

/// One XYZRHW|DIFFUSE vertex.
fn vtx(out: &mut Vec<u8>, x: f32, y: f32, color: u32) {
    for f in [x, y, 0.0, 1.0] {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out.extend_from_slice(&color.to_le_bytes());
}

/// Draws the status screen: one bar per frame step (green = ran in Rust, amber = blocked on an
/// unported callee, red = original-only), and a moving marker driven by the bound clock/input.
pub fn draw_status(dev: &mut dyn D3d9Device, outcomes: &[boot::Outcome], t: f32, mouse: (f32, f32)) {
    let (w, h) = dev.back_buffer_size();
    dev.set_viewport(&Viewport {
        x: 0,
        y: 0,
        width: w,
        height: h,
        min_z: 0.0,
        max_z: 1.0,
    });
    let _ = dev.begin_scene();
    dev.clear(clear::TARGET | clear::ZBUFFER, 0xFF10_1820, 1.0, 0);
    dev.set_render_state(rs::ZENABLE, 0);
    dev.set_render_state(rs::CULLMODE, 1);
    dev.set_render_state(rs::LIGHTING, 0);
    dev.set_render_state(rs::ALPHABLENDENABLE, 0);
    dev.set_texture(0, None);
    dev.set_texture_stage_state(0, tss::COLOROP, 3); // SELECTARG2 (diffuse)
    dev.set_texture_stage_state(0, tss::COLORARG2, 0); // D3DTA_DIFFUSE
    dev.set_texture_stage_state(0, tss::ALPHAOP, 3);
    dev.set_texture_stage_state(0, tss::ALPHAARG2, 0);
    dev.set_fvf(fvf::XYZRHW | fvf::DIFFUSE);
    let mut v = Vec::new();
    let n = outcomes.len().max(1) as f32;
    let bar_h = (h as f32 * 0.6 / n).min(40.0);
    for (i, o) in outcomes.iter().enumerate() {
        let c = match o {
            boot::Outcome::Ok(_) => 0xFF30_C060,
            boot::Outcome::Blocked { .. } => 0xFFE0_A020,
            boot::Outcome::OriginalOnly => 0xFFC0_3030,
            boot::Outcome::Panicked(_) | boot::Outcome::Crashed(_) => 0xFFFF_00FF,
        };
        let (x0, y0) = (w as f32 * 0.1, h as f32 * 0.2 + i as f32 * bar_h);
        let (x1, y1) = (w as f32 * 0.9, y0 + bar_h * 0.8);
        for (x, y) in [(x0, y0), (x1, y0), (x0, y1), (x1, y0), (x1, y1), (x0, y1)] {
            vtx(&mut v, x, y, c);
        }
    }
    // marker: moves with time, offset by the DirectInput mouse record
    let mx = (w as f32 * 0.5 + (t * 1.5).sin() * w as f32 * 0.3 + mouse.0).clamp(0.0, w as f32 - 16.0);
    let my = (h as f32 * 0.1 + mouse.1).clamp(0.0, h as f32 - 16.0);
    for (x, y) in [
        (mx, my),
        (mx + 16.0, my),
        (mx, my + 16.0),
        (mx + 16.0, my),
        (mx + 16.0, my + 16.0),
        (mx, my + 16.0),
    ] {
        vtx(&mut v, x, y, 0xFFFF_FFFF);
    }
    let _ = dev.draw_primitive_up(PrimitiveType::TriangleList, (v.len() / 20 / 3) as u32, &v, 20);
    let _ = dev.end_scene();
    let _ = dev.present();
}

/// Runs the frame steps once without forking (they are known not to fault) and returns their
/// outcomes.
pub fn frame(input: &InputState) -> Vec<boot::Outcome> {
    bind::with("input", |p| {
        let t = p.time.time_get_time();
        for e in &input.events {
            p.queue.push_event(e, t);
        }
        p.input = input.clone();
    });
    vec![boot::run_step_direct(&crate::winmain::frame_step())]
}

/// `--screenshot`: one frame offscreen, written as PNG.
pub fn screenshot(path: &std::path::Path) -> Result<(), String> {
    let mut dev = WgpuDevice::new_headless(640, 480).ok_or("no GPU adapter")?;
    let outcomes = frame(&InputState::default());
    draw_status(&mut dev, &outcomes, 0.0, (0.0, 0.0));
    let (w, h, rgba) = dev.read_render_target(None).map_err(|e| format!("{e:?}"))?;
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .and_then(|mut wr| wr.write_image_data(&rgba))
        .map_err(|e| e.to_string())?;
    Ok(())
}

struct Gpu {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    dev: WgpuDevice,
}

struct GameWindow {
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    start: std::time::Instant,
    frames: u64,
    exit_after: Option<f32>,
    error: Option<String>,
}

impl GameWindow {
    fn init_gpu(&mut self, window: Arc<Window>) -> Result<Gpu, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .map_err(|e| format!("no GPU adapter: {e}"))?;
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("sa-game"),
            required_features: adapter.features() & WgpuDevice::optional_features(),
            required_limits: wgpu::Limits::default(),
            experimental_features: Default::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: Default::default(),
        }))
        .map_err(|e| format!("request_device: {e}"))?;
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        let dev = WgpuDevice::new(device, queue, config.width, config.height);
        Ok(Gpu { surface, config, dev })
    }
}

impl WindowApp for GameWindow {
    fn init(&mut self, window: Arc<Window>) {
        match self.init_gpu(window.clone()) {
            Ok(g) => self.gpu = Some(g),
            Err(e) => self.error = Some(e),
        }
        // DirectInput exclusive mode: the game owns the pointer
        let _ = sa_platform::native::set_pointer_capture(&window, true);
        self.window = Some(window);
    }

    fn resized(&mut self, width: u32, height: u32) {
        if let Some(g) = &mut self.gpu {
            g.config.width = width.max(1);
            g.config.height = height.max(1);
            g.surface.configure(g.dev.device(), &g.config);
            let _ = g.dev.reset(g.config.width, g.config.height);
        }
    }

    fn frame(&mut self, input: &InputState) -> bool {
        if self.error.is_some()
            || input
                .events
                .iter()
                .any(|e| matches!(e, sa_platform::input::InputEvent::Close))
        {
            return false;
        }
        let outcomes = frame(input);
        let t = self.start.elapsed().as_secs_f32();
        let Some(g) = &mut self.gpu else { return false };
        let mouse = input.di_mouse_state();
        draw_status(&mut g.dev, &outcomes, t, (mouse.x as f32, mouse.y as f32));
        let st = match g.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            _ => {
                g.surface.configure(g.dev.device(), &g.config);
                return true;
            }
        };
        let view = st.texture.create_view(&Default::default());
        g.dev.blit_to(&view, g.config.format);
        st.present();
        self.frames += 1;
        if self.frames.is_multiple_of(60)
            && let Some(w) = &self.window
        {
            let ok = outcomes
                .iter()
                .filter(|o| matches!(o, boot::Outcome::Ok(_)))
                .count();
            w.set_title(&format!(
                "sa-game (phase B) — {} frames, {:.0} fps — frame steps in Rust: {ok}/{}",
                self.frames,
                self.frames as f32 / t.max(0.001),
                outcomes.len()
            ));
        }
        self.exit_after.is_none_or(|s| t < s)
    }
}

/// `--window`: the main loop in a native window.
pub fn run_window(exit_after: Option<f32>) -> Result<(), String> {
    let mut app = GameWindow {
        window: None,
        gpu: None,
        start: std::time::Instant::now(),
        frames: 0,
        exit_after,
        error: None,
    };
    let mut cfg = WindowConfig::new("sa-game (phase B)", 640, 480);
    cfg.click_to_capture = false;
    run_windowed(cfg, &mut app)?;
    if let Some(e) = app.error {
        return Err(e);
    }
    println!("sa-game: window closed after {} frames", app.frames);
    Ok(())
}
