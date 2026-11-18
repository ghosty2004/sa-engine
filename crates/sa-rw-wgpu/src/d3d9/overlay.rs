//! The 2D overlay (feature `overlay`): quads of plain colour or of an image, alpha-blended over the back
//! buffer just before the present, outside the game's D3D9 call stream and state (no D3D9 state is read or changed;
//! the back buffer's alpha is kept). The mod host's overlay phase draws through it.

use std::collections::HashMap;

use sa_platform::gpu::{OverlayFrame, OverlayImage};
use wgpu::util::DeviceExt as _;

use super::{COLOR_FORMAT, WgpuDevice};

const OVERLAY_WGSL: &str = r#"
struct V { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) colour: vec4<f32> };
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
@vertex fn vs(@location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) colour: vec4<f32>) -> V {
    return V(vec4<f32>(pos, 0.0, 1.0), uv, colour);
}
@fragment fn fs(v: V) -> @location(0) vec4<f32> { return textureSample(image, image_sampler, v.uv) * v.colour; }
"#;

/// One vertex: clip-space position, texture coordinates, colour (RGBA8 unorm).
const STRIDE: usize = 20;

/// The overlay's pipeline and the images its quads may use.
pub(super) struct Overlay {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    nearest: wgpu::Sampler,
    linear: wgpu::Sampler,
    /// Image 0: one white texel (plain-colour quads).
    white: wgpu::BindGroup,
    images: HashMap<u32, wgpu::BindGroup>,
}

impl Overlay {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Overlay {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("overlay"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = |filter: wgpu::FilterMode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("overlay"),
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: match filter {
                    wgpu::FilterMode::Linear => wgpu::MipmapFilterMode::Linear,
                    _ => wgpu::MipmapFilterMode::Nearest,
                },
                ..Default::default()
            })
        };
        let (nearest, linear) = (
            sampler(wgpu::FilterMode::Nearest),
            sampler(wgpu::FilterMode::Linear),
        );
        let white = OverlayImage {
            id: 0,
            width: 1,
            height: 1,
            smooth: false,
            rgba: [255u8; 4].into(),
        };
        let white = bind_image(device, queue, &layout, &nearest, &white);
        Overlay {
            pipeline: overlay_pipeline(device, &layout),
            white,
            layout,
            nearest,
            linear,
            images: HashMap::new(),
        }
    }

    fn bind(&self, device: &wgpu::Device, queue: &wgpu::Queue, img: &OverlayImage) -> wgpu::BindGroup {
        bind_image(
            device,
            queue,
            &self.layout,
            if img.smooth { &self.linear } else { &self.nearest },
            img,
        )
    }
}

/// The mip chain of a `w` x `h` RGBA8 image down to 1 x 1, each texel the rounded mean of the 2 x 2 texels above it
/// (D3DX's default box filter, channels straight). Level 0 first.
pub fn mip_chain(w: u32, h: u32, rgba: &[u8]) -> Vec<(u32, u32, Vec<u8>)> {
    let mut out = vec![(w, h, rgba[..(w * h * 4) as usize].to_vec())];
    while let Some((pw, ph, p)) = out.last() {
        let (pw, ph) = (*pw, *ph);
        if pw == 1 && ph == 1 {
            break;
        }
        let (nw, nh) = ((pw / 2).max(1), (ph / 2).max(1));
        let mut n = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let at = |xx: u32, yy: u32| {
                        u32::from(p[((yy.min(ph - 1) * pw + xx.min(pw - 1)) * 4 + c) as usize])
                    };
                    let (x0, y0) = (2 * x, 2 * y);
                    let sum = at(x0, y0) + at(x0 + 1, y0) + at(x0, y0 + 1) + at(x0 + 1, y0 + 1);
                    n[((y * nw + x) * 4 + c) as usize] = ((sum + 2) / 4) as u8;
                }
            }
        }
        out.push((nw, nh, n));
    }
    out
}

/// A texture holding `img` (with its mip chain when `smooth`), bound with `sampler` for the overlay's pipeline.
fn bind_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    img: &OverlayImage,
) -> wgpu::BindGroup {
    let (w, h) = (img.width.max(1), img.height.max(1));
    let levels = if img.rgba.len() as u64 != u64::from(w) * u64::from(h) * 4 {
        Vec::new()
    } else if img.smooth {
        mip_chain(w, h, &img.rgba)
    } else {
        vec![(w, h, img.rgba.to_vec())]
    };
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("overlay image"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels.len().max(1) as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, (lw, lh, px)) in levels.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            px,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(lw * 4),
                rows_per_image: Some(*lh),
            },
            wgpu::Extent3d {
                width: *lw,
                height: *lh,
                depth_or_array_layers: 1,
            },
        );
    }
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("overlay image"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

impl WgpuDevice {
    /// Takes `frame`'s image changes and draws its quads over the back buffer (in order, alpha-blended), into the
    /// current submission: call before [`sa_platform::gpu::D3d9Device::present`]. A quad naming an image that does
    /// not exist is drawn in plain colour.
    pub fn draw_overlay(&mut self, frame: &OverlayFrame) {
        if self.overlay.is_none() {
            self.overlay = Some(Overlay::new(&self.device, &self.queue));
        }
        let o = self.overlay.as_mut().expect("created above");
        for id in &frame.removed {
            o.images.remove(id);
        }
        for img in &frame.added {
            let group = o.bind(&self.device, &self.queue, img);
            o.images.insert(img.id, group);
        }
        if frame.quads.is_empty() {
            return;
        }
        let (w, h) = (self.size.0.max(1) as f32, self.size.1.max(1) as f32);
        let mut bytes = Vec::with_capacity(frame.quads.len() * 6 * STRIDE);
        // runs of consecutive quads with the same image: (image, first vertex, vertex count)
        let mut runs: Vec<(u32, u32, u32)> = Vec::new();
        for (i, q) in frame.quads.iter().enumerate() {
            let (x0, y0) = (q.x / w * 2.0 - 1.0, 1.0 - q.y / h * 2.0);
            let (x1, y1) = ((q.x + q.w) / w * 2.0 - 1.0, 1.0 - (q.y + q.h) / h * 2.0);
            let [u0, v0, u1, v1] = q.uv;
            let c = q.rgba.to_be_bytes();
            for (x, y, u, v) in [
                (x0, y0, u0, v0),
                (x1, y0, u1, v0),
                (x0, y1, u0, v1),
                (x0, y1, u0, v1),
                (x1, y0, u1, v0),
                (x1, y1, u1, v1),
            ] {
                for f in [x, y, u, v] {
                    bytes.extend_from_slice(&f.to_le_bytes());
                }
                bytes.extend_from_slice(&c);
            }
            let image = if o.images.contains_key(&q.image) {
                q.image
            } else {
                0
            };
            match runs.last_mut() {
                Some(r) if r.0 == image => r.2 += 6,
                _ => runs.push((image, i as u32 * 6, 6)),
            }
        }
        self.end_pass();
        let o = self.overlay.as_ref().expect("created above");
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
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&o.pipeline);
        pass.set_vertex_buffer(0, vb.slice(..));
        for (image, first, count) in runs {
            pass.set_bind_group(0, o.images.get(&image).unwrap_or(&o.white), &[]);
            pass.draw(first..first + count, 0..1);
        }
    }
}

fn overlay_pipeline(device: &wgpu::Device, group: &wgpu::BindGroupLayout) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("overlay"),
        source: wgpu::ShaderSource::Wgsl(OVERLAY_WGSL.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("overlay"),
        bind_group_layouts: &[Some(group)],
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
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 8,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Unorm8x4,
                        offset: 16,
                        shader_location: 2,
                    },
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
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
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
    use sa_platform::gpu::{D3d9Device as _, OverlayQuad};

    #[test]
    fn mip_chain_box_filters_down_to_one_texel() {
        let mut img = vec![0u8; 4 * 4 * 4];
        for (i, p) in img.chunks_mut(4).enumerate() {
            p.copy_from_slice(&[(i * 10) as u8, 255, 0, if i % 2 == 0 { 255 } else { 0 }]);
        }
        let m = mip_chain(4, 4, &img);
        assert_eq!(
            m.iter().map(|l| (l.0, l.1)).collect::<Vec<_>>(),
            [(4, 4), (2, 2), (1, 1)]
        );
        // texel (0, 0) of level 1 = the mean of texels 0, 1, 4, 5: red (0 + 10 + 40 + 50) / 4 = 25, alpha 127.5 -> 128
        assert_eq!(&m[1].2[..4], &[25, 255, 0, 128]);
        assert_eq!(mip_chain(512, 1024, &vec![0; 512 * 1024 * 4]).len(), 11);
    }

    #[test]
    fn overlay_blends_over_the_back_buffer() {
        let Some(mut d) = WgpuDevice::new_headless(16, 8) else {
            eprintln!("no adapter: skipped");
            return;
        };
        // D3DCLEAR_TARGET, opaque black (D3DCOLOR is ARGB)
        d.clear(1, 0xFF00_0000, 1.0, 0);
        // a 2 x 1 image: blue, then transparent white
        let img = OverlayImage {
            id: 7,
            width: 2,
            height: 1,
            smooth: false,
            rgba: [0, 0, 255, 255, 255, 255, 255, 0].into(),
        };
        d.draw_overlay(&OverlayFrame {
            added: vec![img],
            removed: Vec::new(),
            quads: vec![
                OverlayQuad::rect(0.0, 0.0, 8.0, 4.0, 0xFF00_00FF),
                OverlayQuad::rect(8.0, 0.0, 8.0, 4.0, 0x00FF_0080),
                OverlayQuad {
                    x: 0.0,
                    y: 4.0,
                    w: 8.0,
                    h: 4.0,
                    uv: [0.0, 0.0, 1.0, 1.0],
                    rgba: 0xFFFF_FFFF,
                    image: 7,
                },
            ],
        });
        d.present().unwrap();
        let (w, _, px) = d.read_render_target(None).unwrap();
        let at = |x: u32, y: u32| px[((y * w + x) * 4) as usize..][..3].to_vec();
        assert_eq!(at(2, 2), [0xFF, 0, 0]);
        // 50% green over the cleared (black) back buffer
        let g = at(12, 2)[1];
        assert!((0x7C..=0x84).contains(&g), "{:x?}", at(12, 2));
        // the image's left texel is blue, its right one transparent
        assert_eq!(at(1, 6), [0, 0, 0xFF]);
        assert_eq!(at(6, 6), [0, 0, 0]);
    }
}
