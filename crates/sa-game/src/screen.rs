//! The status screen (`--screenshot`): one more main-loop iteration after the offscreen run, drawn as a status
//! screen through the D3D9 seam (`sa_platform::gpu::D3d9Device` on a headless `sa_rw_wgpu::d3d9::WgpuDevice`): one bar
//! per frame step, issued as a D3D9 call stream (clear + `DrawPrimitiveUP` of pre-transformed quads). The game itself
//! in a window is [`crate::game_window`].

use sa_platform::gpu::{D3d9Device, PrimitiveType, Viewport, clear, fvf, rs, tss};
use sa_platform::input::InputState;
use sa_rw_wgpu::d3d9::WgpuDevice;

use crate::{bind, boot};

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

