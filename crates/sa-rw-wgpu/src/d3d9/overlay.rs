//! The 2D overlay (feature `overlay`, MOD-2): filled rectangles alpha-blended over the back buffer just before the
//! present, outside the game's D3D9 call stream and state (no D3D9 state is read or changed; the back buffer's alpha
//! is kept). The mod host's overlay phase draws through it.

use sa_platform::gpu::OverlayRect;
use wgpu::util::DeviceExt as _;

use super::{COLOR_FORMAT, WgpuDevice};

const OVERLAY_WGSL: &str = r#"
struct V { @builtin(position) pos: vec4<f32>, @location(0) colour: vec4<f32> };
@vertex fn vs(@location(0) pos: vec2<f32>, @location(1) colour: vec4<f32>) -> V {
    return V(vec4<f32>(pos, 0.0, 1.0), colour);
}
@fragment fn fs(v: V) -> @location(0) vec4<f32> { return v.colour; }
"#;

/// One vertex: clip-space position, colour (RGBA8 unorm).
const STRIDE: usize = 12;

impl WgpuDevice {
    /// Draws `rects` over the back buffer (in order, alpha-blended), into the current submission: call before
    /// [`sa_platform::gpu::D3d9Device::present`].
    pub fn draw_overlay(&mut self, rects: &[OverlayRect]) {
        if rects.is_empty() {
            return;
        }
        let (w, h) = (self.size.0.max(1) as f32, self.size.1.max(1) as f32);
        let mut bytes = Vec::with_capacity(rects.len() * 6 * STRIDE);
        for r in rects {
            let (x0, y0) = (r.x / w * 2.0 - 1.0, 1.0 - r.y / h * 2.0);
            let (x1, y1) = ((r.x + r.w) / w * 2.0 - 1.0, 1.0 - (r.y + r.h) / h * 2.0);
            let c = r.rgba.to_be_bytes();
            for (x, y) in [(x0, y0), (x1, y0), (x0, y1), (x0, y1), (x1, y0), (x1, y1)] {
                bytes.extend_from_slice(&x.to_le_bytes());
                bytes.extend_from_slice(&y.to_le_bytes());
                bytes.extend_from_slice(&c);
            }
        }
        self.end_pass();
        let pipeline = self.overlay_pipeline.get_or_insert_with(|| overlay_pipeline(&self.device)).clone();
        let vb = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("overlay"),
            contents: &bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
        let enc = Self::encoder(&self.device, &mut self.enc);
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("overlay"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.back_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_vertex_buffer(0, vb.slice(..));
        pass.draw(0..(rects.len() * 6) as u32, 0..1);
    }
}

fn overlay_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("overlay"),
        source: wgpu::ShaderSource::Wgsl(OVERLAY_WGSL.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("overlay"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("overlay"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: STRIDE as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
                    wgpu::VertexAttribute { format: wgpu::VertexFormat::Unorm8x4, offset: 8, shader_location: 1 },
                ],
            }],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendState::ALPHA_BLENDING.color,
                    alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
                }),
                // the back buffer's alpha is the game's (D3D9 reads it in later copies): kept
                write_mask: wgpu::ColorWrites::COLOR,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_platform::gpu::D3d9Device as _;

    #[test]
    fn overlay_blends_over_the_back_buffer() {
        let Some(mut d) = WgpuDevice::new_headless(16, 8) else {
            eprintln!("no adapter: skipped");
            return;
        };
        // D3DCLEAR_TARGET, opaque black (D3DCOLOR is ARGB)
        d.clear(1, 0xFF00_0000, 1.0, 0);
        d.draw_overlay(&[
            OverlayRect { x: 0.0, y: 0.0, w: 8.0, h: 8.0, rgba: 0xFF00_00FF },
            OverlayRect { x: 8.0, y: 0.0, w: 8.0, h: 8.0, rgba: 0x00FF_0080 },
        ]);
        d.present().unwrap();
        let (w, _, px) = d.read_render_target(None).unwrap();
        let at = |x: u32, y: u32| &px[((y * w + x) * 4) as usize..][..3];
        assert_eq!(at(2, 2), [0xFF, 0, 0]);
        // 50% green over the cleared (black) back buffer
        let g = at(12, 4)[1];
        assert!((0x7C..=0x84).contains(&g), "{:x?}", at(12, 4));
    }
}
