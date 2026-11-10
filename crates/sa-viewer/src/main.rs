//! `sa-viewer`: NATIVE-DEMO, a standalone San Andreas world viewer (no gta_sa.exe, no Wine).
//!
//! ```text
//! sa-viewer --game /path/to/gta-sa [--pos x,y,z] [--dir x,y,z] [--time HH:MM] [--weather N]
//!           [--size WxH] [--draw-dist F] [--budget-mb N] [--area N]
//!           [--screenshot out.png]          headless: load everything visible, render one frame
//!           [--bench SECONDS [--bench-out file.txt]]  headless scripted fly-through, prints FPS/memory
//! ```
//! Window controls: click to capture the mouse (Esc releases, Esc again quits), WASD move,
//! Space/E up, Q/Ctrl down... see `docs/arch/native-demo.md`.

mod loader;
mod timecycle;
mod viewer;
mod world;

use std::sync::Arc;
use std::time::Instant;

use sa_platform::fs::GameFs;
use sa_platform::input::{InputState, Key};
use sa_platform::native::{NativeFs, Window, WindowApp, WindowConfig, run_windowed};
use sa_rw_wgpu::{DEPTH_FORMAT, Renderer};

use viewer::{Options, Viewer};
use world::World;

struct Args {
    game: String,
    pos: Option<[f32; 3]>,
    dir: Option<[f32; 3]>,
    time: Option<f32>,
    weather: usize,
    size: (u32, u32),
    screenshot: Option<String>,
    bench: Option<f32>,
    bench_out: Option<String>,
    exit_after: Option<f32>,
    opts: Options,
}

fn parse_vec3(s: &str) -> Result<[f32; 3], String> {
    let v: Vec<f32> = s.split(',').map(|x| x.trim().parse::<f32>().map_err(|e| format!("{s}: {e}"))).collect::<Result<_, _>>()?;
    if v.len() != 3 {
        return Err(format!("{s}: expected x,y,z"));
    }
    Ok([v[0], v[1], v[2]])
}

fn usage() -> ! {
    eprintln!(
        "usage: sa-viewer --game /path/to/gta-sa [--pos x,y,z] [--dir x,y,z] [--time HH:MM] [--weather N]\n\
         \x20                [--size WxH] [--draw-dist F] [--budget-mb N] [--area N]\n\
         \x20                [--screenshot out.png] [--bench SECONDS [--bench-out file]] [--exit-after SECONDS]"
    );
    std::process::exit(2)
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        game: String::new(),
        pos: None,
        dir: None,
        time: None,
        weather: 0,
        size: (1280, 720),
        screenshot: None,
        bench: None,
        bench_out: None,
        exit_after: None,
        opts: Options::default(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().ok_or_else(|| format!("{k} needs a value"));
        match k.as_str() {
            "--game" => a.game = val()?,
            "--pos" => a.pos = Some(parse_vec3(&val()?)?),
            "--dir" => a.dir = Some(parse_vec3(&val()?)?),
            "--time" => {
                let v = val()?;
                let (h, m) = v.split_once(':').unwrap_or((&v, "0"));
                let h: f32 = h.parse().map_err(|_| format!("bad --time {v}"))?;
                let m: f32 = m.parse().map_err(|_| format!("bad --time {v}"))?;
                a.time = Some(h * 60.0 + m);
            }
            "--weather" => a.weather = val()?.parse().map_err(|_| "bad --weather")?,
            "--size" => {
                let v = val()?;
                let (w, h) = v.split_once('x').ok_or("bad --size (WxH)")?;
                a.size = (w.parse().map_err(|_| "bad --size")?, h.parse().map_err(|_| "bad --size")?);
            }
            "--draw-dist" => a.opts.draw_dist_scale = val()?.parse().map_err(|_| "bad --draw-dist")?,
            "--budget-mb" => a.opts.budget_bytes = val()?.parse::<u64>().map_err(|_| "bad --budget-mb")? << 20,
            "--area" => a.opts.area = val()?.parse().map_err(|_| "bad --area")?,
            "--screenshot" => a.screenshot = Some(val()?),
            "--bench" => a.bench = Some(val()?.parse().map_err(|_| "bad --bench")?),
            "--bench-out" => a.bench_out = Some(val()?),
            "--exit-after" => a.exit_after = Some(val()?.parse().map_err(|_| "bad --exit-after")?),
            "-h" | "--help" => usage(),
            _ => return Err(format!("unknown argument {k}")),
        }
    }
    if a.game.is_empty() {
        return Err("--game is required".into());
    }
    Ok(a)
}

/// Minimal executor for wgpu's futures (they complete without an external reactor natively).
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

fn create_device(instance: &wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), String> {
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: surface,
    }))
    .map_err(|e| format!("no GPU adapter: {e}"))?;
    let features = adapter.features() & Renderer::optional_features();
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("sa-viewer"),
        required_features: features,
        required_limits: wgpu::Limits::default(),
        experimental_features: Default::default(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: Default::default(),
    }))
    .map_err(|e| format!("request_device: {e}"))?;
    Ok((adapter, device, queue))
}

fn load(args: &Args, renderer: Renderer) -> Result<Viewer, String> {
    let fs: Arc<dyn GameFs> = Arc::new(NativeFs::new(&args.game).map_err(|e| e.to_string())?);
    let t = Instant::now();
    let world = World::load(fs.as_ref())?;
    for w in world.warnings.iter().take(10) {
        eprintln!("warning: {w}");
    }
    let lods = world.instances.iter().filter(|i| !i.lod_children.is_empty()).count();
    eprintln!(
        "world: {} archives, {} models, {} txds, {} instances ({} LOD parents), {} water polys in {:.2}s",
        world.archives.len(),
        world.models.len(),
        world.txds.len(),
        world.instances.len(),
        lods,
        world.water.len(),
        t.elapsed().as_secs_f64()
    );
    let weather = args.weather;
    let mut v = Viewer::new(fs, world, renderer, std::mem::take(&mut Options::default()));
    v.opts = Options {
        draw_dist_scale: args.opts.draw_dist_scale,
        budget_bytes: args.opts.budget_bytes,
        area: args.opts.area,
        max_in_flight: args.opts.max_in_flight,
        upload_budget: args.opts.upload_budget,
    };
    v.weather = weather;
    if let Some(p) = args.pos {
        v.cam.pos = p;
    }
    if let Some(d) = args.dir {
        v.cam.look_dir(d);
    }
    if let Some(t) = args.time {
        v.minutes = t;
    }
    Ok(v)
}

fn rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmRSS:")).map(|l| l.to_string()))
        .and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse::<f64>().ok()))
        .map_or(0.0, |kb| kb / 1024.0)
}

// ------------------------------------------------------------------------------- headless ---

struct Offscreen {
    color: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    size: (u32, u32),
}

fn offscreen(device: &wgpu::Device, size: (u32, u32)) -> Offscreen {
    let ext = wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 };
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen"),
        size: ext,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: ext,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    Offscreen {
        color_view: color.create_view(&Default::default()),
        depth_view: depth.create_view(&Default::default()),
        color,
        size,
    }
}

fn read_back(device: &wgpu::Device, queue: &wgpu::Queue, o: &Offscreen) -> Vec<u8> {
    let (w, h) = o.size;
    let bpr = (w * 4).div_ceil(256) * 256;
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (bpr * h) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &o.color, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bpr), rows_per_image: Some(h) },
        },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    queue.submit([enc.finish()]);
    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    let data = slice.get_mapped_range();
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h as usize {
        out.extend_from_slice(&data[y * bpr as usize..y * bpr as usize + (w * 4) as usize]);
    }
    out
}

fn write_png(path: &str, w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
    if let Some(dir) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let f = std::fs::File::create(path).map_err(|e| format!("{path}: {e}"))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(rgba).map_err(|e| e.to_string())?;
    Ok(())
}

fn headless_device() -> Result<(wgpu::Device, wgpu::Queue, String), String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let (adapter, device, queue) = create_device(&instance, None)?;
    let info = adapter.get_info();
    Ok((device, queue, format!("{} ({:?})", info.name, info.backend)))
}

fn run_screenshot(args: &Args, out: &str) -> Result<(), String> {
    let (device, queue, name) = headless_device()?;
    eprintln!("adapter: {name}");
    let renderer = Renderer::new(device.clone(), queue.clone(), wgpu::TextureFormat::Rgba8Unorm);
    let mut v = load(args, renderer)?;
    let t = Instant::now();
    v.update(true);
    let o = offscreen(&device, args.size);
    let mut enc = device.create_command_encoder(&Default::default());
    v.render(&mut enc, &o.color_view, &o.depth_view, args.size);
    queue.submit([enc.finish()]);
    let px = read_back(&device, &queue, &o);
    write_png(out, o.size.0, o.size.1, &px)?;
    let s = v.stats;
    eprintln!(
        "screenshot {out}: streamed in {:.2}s; {} visible entities, {} draw calls, {} triangles, {} models / {} txds on GPU ({:.0} MB), RSS {:.0} MB",
        t.elapsed().as_secs_f64(),
        s.visible,
        s.draw_calls,
        s.triangles,
        s.models_loaded,
        s.txds_loaded,
        s.gpu_mb,
        rss_mb()
    );
    Ok(())
}

/// Scripted fly-through: straight segments between waypoints at 80 m/s, streaming as in the
/// window (time-budgeted uploads), one frame rendered and waited for per step.
fn run_bench(args: &Args, seconds: f32) -> Result<(), String> {
    let (device, queue, name) = headless_device()?;
    let renderer = Renderer::new(device.clone(), queue.clone(), wgpu::TextureFormat::Rgba8Unorm);
    let mut v = load(args, renderer)?;
    let route: [[f32; 3]; 6] = [
        [2495.0, -1670.0, 40.0],
        [1500.0, -1700.0, 80.0],
        [1480.0, -1000.0, 120.0],
        [400.0, -600.0, 150.0],
        [-1500.0, 400.0, 150.0],
        [-2000.0, 800.0, 120.0],
    ];
    let o = offscreen(&device, args.size);
    let speed = 80.0;
    let (mut seg, mut along) = (0usize, 0.0f32);
    let start = Instant::now();
    let mut frame_times = Vec::new();
    let mut peak_rss: f64 = 0.0;
    let mut peak_gpu: f64 = 0.0;
    let mut last = Instant::now();
    v.cam.pos = route[0];
    while start.elapsed().as_secs_f32() < seconds {
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        // advance along the route
        along += speed * dt;
        loop {
            let (a, b) = (route[seg], route[(seg + 1) % route.len()]);
            let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
            if along < len {
                let t = along / len;
                v.cam.pos = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
                v.cam.look_dir([b[0] - a[0], b[1] - a[1], (b[2] - a[2]) - 0.15 * len]);
                break;
            }
            along -= len;
            seg = (seg + 1) % route.len();
        }
        v.update(false);
        let mut enc = device.create_command_encoder(&Default::default());
        v.render(&mut enc, &o.color_view, &o.depth_view, args.size);
        queue.submit([enc.finish()]);
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        frame_times.push(last.elapsed().as_secs_f64());
        peak_rss = peak_rss.max(rss_mb());
        peak_gpu = peak_gpu.max(v.stats.gpu_mb);
    }
    let n = frame_times.len().max(1);
    let total: f64 = frame_times.iter().sum();
    let mut sorted = frame_times.clone();
    sorted.sort_by(|a, b| b.total_cmp(a));
    let p1 = sorted[..(n / 100).max(1)].iter().sum::<f64>() / (n / 100).max(1) as f64;
    let s = v.stats;
    let report = format!(
        "bench: adapter {name}, {}x{}, {:.0}s, {} frames\n\
         avg fps {:.1} (avg frame {:.2} ms), 1% low fps {:.1}, worst frame {:.1} ms\n\
         peak RSS {:.0} MB, peak GPU (models+textures) {:.0} MB, evictions {}\n\
         last frame: {} visible entities, {} draw calls, {} triangles; {} MB read from IMG\n",
        args.size.0,
        args.size.1,
        seconds,
        n,
        n as f64 / total,
        total / n as f64 * 1000.0,
        1.0 / p1,
        sorted[0] * 1000.0,
        peak_rss,
        peak_gpu,
        s.evicted,
        s.visible,
        s.draw_calls,
        s.triangles,
        v.loader_bytes_read() >> 20,
    );
    print!("{report}");
    if let Some(p) = &args.bench_out {
        std::fs::write(p, &report).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// --------------------------------------------------------------------------------- window ---

struct WinState {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    depth: wgpu::TextureView,
    viewer: Viewer,
    window: Arc<Window>,
}

struct App {
    args: Args,
    state: Option<WinState>,
    last: Instant,
    fps_frames: u32,
    fps_time: f64,
    fps: f64,
    speed: f32,
    time_flow: bool,
    error: Option<String>,
    started: Instant,
    total_frames: u64,
}

fn depth_view(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

impl App {
    fn init_gpu(&mut self, window: Arc<Window>) -> Result<WinState, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance.create_surface(window.clone()).map_err(|e| e.to_string())?;
        let (adapter, device, queue) = create_device(&instance, Some(&surface))?;
        let caps = surface.get_capabilities(&adapter);
        // non-sRGB target: the game blends in gamma space
        let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0]);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
                wgpu::PresentMode::Mailbox
            } else {
                wgpu::PresentMode::Fifo
            },
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        let renderer = Renderer::new(device.clone(), queue.clone(), format);
        let viewer = load(&self.args, renderer)?;
        let info = adapter.get_info();
        eprintln!("adapter: {} ({:?}), surface format {format:?}", info.name, info.backend);
        Ok(WinState { depth: depth_view(&device, config.width, config.height), surface, config, device, queue, viewer, window })
    }

    fn step_camera(&mut self, input: &InputState, dt: f32) {
        let Some(s) = &mut self.state else { return };
        let v = &mut s.viewer;
        let sens = 0.0025;
        v.cam.yaw += input.mouse_delta.0 as f32 * sens;
        v.cam.pitch = (v.cam.pitch - input.mouse_delta.1 as f32 * sens).clamp(-1.55, 1.55);
        let turn = 1.5 * dt;
        if input.is_held(Key::Left) {
            v.cam.yaw -= turn;
        }
        if input.is_held(Key::Right) {
            v.cam.yaw += turn;
        }
        if input.is_held(Key::Up) {
            v.cam.pitch = (v.cam.pitch + turn).min(1.55);
        }
        if input.is_held(Key::Down) {
            v.cam.pitch = (v.cam.pitch - turn).max(-1.55);
        }
        if input.wheel != 0.0 {
            self.speed = (self.speed * 1.25f32.powf(input.wheel)).clamp(1.0, 2000.0);
        }
        let mut speed = self.speed;
        if input.is_held(Key::ShiftLeft) {
            speed *= 5.0;
        }
        if input.is_held(Key::AltLeft) {
            speed *= 0.2;
        }
        let (f, r) = (v.cam.dir(), v.cam.right());
        let mut mv = [0.0f32; 3];
        let mut add = |d: [f32; 3], s: f32| (0..3).for_each(|k| mv[k] += d[k] * s);
        if input.is_held(Key::W) {
            add(f, 1.0);
        }
        if input.is_held(Key::S) {
            add(f, -1.0);
        }
        if input.is_held(Key::D) {
            add(r, 1.0);
        }
        if input.is_held(Key::A) {
            add(r, -1.0);
        }
        if input.is_held(Key::Space) || input.is_held(Key::E) {
            add([0.0, 0.0, 1.0], 1.0);
        }
        if input.is_held(Key::Q) || input.is_held(Key::ControlLeft) {
            add([0.0, 0.0, 1.0], -1.0);
        }
        for (p, m) in v.cam.pos.iter_mut().zip(mv) {
            *p += m * speed * dt;
        }
        // time of day / weather
        for k in &input.pressed {
            match k {
                Key::BracketLeft | Key::Minus => v.minutes = (v.minutes - 60.0).rem_euclid(1440.0),
                Key::BracketRight | Key::Equal => v.minutes = (v.minutes + 60.0).rem_euclid(1440.0),
                Key::T => self.time_flow = !self.time_flow,
                Key::Comma => v.weather = (v.weather + v.num_weathers() - 1) % v.num_weathers(),
                Key::Period => v.weather = (v.weather + 1) % v.num_weathers(),
                Key::PageUp => v.opts.draw_dist_scale = (v.opts.draw_dist_scale + 0.1).min(4.0),
                Key::PageDown => v.opts.draw_dist_scale = (v.opts.draw_dist_scale - 0.1).max(0.2),
                Key::Digit(n @ 1..=4) => {
                    let p: [[f32; 3]; 4] =
                        [[2495.0, -1670.0, 30.0], [1480.0, -1700.0, 80.0], [-2000.0, 300.0, 80.0], [2040.0, 1300.0, 60.0]];
                    v.cam.pos = p[*n as usize - 1];
                }
                _ => {}
            }
        }
        if self.time_flow {
            // one game minute per real second (CClock::ms_nMillisecondsPerGameMinute = 1000)
            v.minutes = (v.minutes + dt).rem_euclid(1440.0);
        }
    }
}

impl WindowApp for App {
    fn init(&mut self, window: Arc<Window>) {
        self.started = Instant::now();
        match self.init_gpu(window) {
            Ok(s) => self.state = Some(s),
            Err(e) => self.error = Some(e),
        }
        self.last = Instant::now();
    }

    fn resized(&mut self, width: u32, height: u32) {
        if let Some(s) = &mut self.state
            && width > 0
            && height > 0
        {
            s.config.width = width;
            s.config.height = height;
            s.surface.configure(&s.device, &s.config);
            s.depth = depth_view(&s.device, width, height);
        }
    }

    fn frame(&mut self, input: &InputState) -> bool {
        if self.state.is_none() {
            return false;
        }
        if input.was_pressed(Key::Escape) && !input.pointer_captured {
            return false;
        }
        let dt = self.last.elapsed().as_secs_f32().min(0.25);
        self.last = Instant::now();
        self.step_camera(input, dt);
        let Some(s) = &mut self.state else { return false };
        s.viewer.update(false);
        let frame = match s.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                s.surface.configure(&s.device, &s.config);
                return true;
            }
            _ => return true,
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = s.device.create_command_encoder(&Default::default());
        s.viewer.render(&mut enc, &view, &s.depth, (s.config.width, s.config.height));
        s.queue.submit([enc.finish()]);
        frame.present();

        self.fps_frames += 1;
        self.total_frames += 1;
        if let Some(limit) = self.args.exit_after
            && self.started.elapsed().as_secs_f32() >= limit
        {
            let t = self.started.elapsed().as_secs_f64();
            let st = s.viewer.stats;
            eprintln!(
                "window: {} frames in {t:.1}s ({:.1} fps avg), {} visible entities, {:.0} MB GPU, RSS {:.0} MB",
                self.total_frames,
                self.total_frames as f64 / t,
                st.visible,
                st.gpu_mb,
                rss_mb()
            );
            return false;
        }
        self.fps_time += dt as f64;
        if self.fps_time >= 0.5 {
            self.fps = self.fps_frames as f64 / self.fps_time;
            self.fps_frames = 0;
            self.fps_time = 0.0;
            let v = &s.viewer;
            let st = v.stats;
            let m = v.minutes as u32;
            s.window.set_title(&format!(
                "sa-viewer | {:.0} fps | {:02}:{:02} weather {} | pos {:.0},{:.0},{:.0} | {} ent, {} draws | {} models, {:.0} MB GPU, {} pending | speed {:.0}",
                self.fps,
                m / 60,
                m % 60,
                v.weather,
                v.cam.pos[0],
                v.cam.pos[1],
                v.cam.pos[2],
                st.visible,
                st.draw_calls,
                st.models_loaded,
                st.gpu_mb,
                st.pending,
                self.speed
            ));
        }
        true
    }
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("sa-viewer: {e}");
            usage();
        }
    };
    let r = if let Some(out) = args.screenshot.clone() {
        run_screenshot(&args, &out)
    } else if let Some(secs) = args.bench {
        run_bench(&args, secs)
    } else {
        let (w, h) = args.size;
        let mut app = App {
            args,
            state: None,
            last: Instant::now(),
            fps_frames: 0,
            fps_time: 0.0,
            fps: 0.0,
            speed: 30.0,
            time_flow: false,
            error: None,
            started: Instant::now(),
            total_frames: 0,
        };
        let r = run_windowed(WindowConfig { title: "sa-viewer".into(), width: w, height: h }, &mut app);
        match (r, app.error.take()) {
            (_, Some(e)) => Err(e),
            (Err(e), None) => Err(e),
            (Ok(()), None) => Ok(()),
        }
    };
    if let Err(e) = r {
        eprintln!("sa-viewer: {e}");
        std::process::exit(1);
    }
}
